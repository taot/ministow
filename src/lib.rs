use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs as unix_fs;

const HELP_TEXT: &str = "ministow 0.1.0

Manage dotfile packages as symlinks with selective directory folding.

Usage:
  ministow [OPTIONS] <PACKAGE>...

Arguments:
  <PACKAGE>...           One or more first-level package directories

Options:
  -T, --target <DIR>     Target directory (default: parent of current working directory)
  -v, --verbose <LEVEL>  Verbosity level: 0, 1, 2 (default: 0)
  -F, --fold <DIR>       Fold a package subdirectory; repeatable
  -C, --config <FILE>    Read options from a config file (default: .ministowrc in cwd)
  -N, --dry-run          Print planned actions without modifying the filesystem
  -I, --ignore-conflicts Ignore install target conflicts during dry-run planning
  -D, --delete           Remove symlinks for the selected package(s)
  -h, --help             Print help and exit
";

#[derive(Clone, Debug, Default)]
struct RawOptions {
    target: Option<String>,
    verbose: Option<u8>,
    folds: Vec<String>,
    config: Option<String>,
    dry_run: bool,
    ignore_conflicts: bool,
    delete: bool,
    help: bool,
    packages: Vec<String>,
    fold_set_by_cli: bool,
}

#[derive(Clone, Debug)]
struct EffectiveOptions {
    target: PathBuf,
    verbose: u8,
    folds: BTreeSet<String>,
    dry_run: bool,
    ignore_conflicts: bool,
    delete: bool,
    packages: Vec<String>,
}

#[derive(Clone, Debug)]
struct AppContext {
    target: PathBuf,
    verbose: u8,
    dry_run: bool,
    ignore_conflicts: bool,
    delete: bool,
    packages: Vec<Package>,
    folds: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct Package {
    name: String,
    root: PathBuf,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Operation {
    Mkdir(PathBuf),
    Link { target: PathBuf, source: PathBuf },
    Unlink(PathBuf),
    Rmdir(PathBuf),
}

#[derive(Clone, Debug, Default)]
struct Plan {
    operations: Vec<Operation>,
    link_count: usize,
    unlink_count: usize,
}

#[derive(Default)]
struct Logger {
    lines: Vec<String>,
}

impl Logger {
    fn push(&mut self, message: impl Into<String>) {
        self.lines.push(message.into());
    }
}

pub fn run_cli(args: &[String], cwd: &Path, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match run_cli_inner(args, cwd, stdout, stderr) {
        Ok(()) => 0,
        Err(err) => {
            let _ = writeln!(stderr, "{err}");
            1
        }
    }
}

fn run_cli_inner(
    args: &[String],
    cwd: &Path,
    stdout: &mut dyn Write,
    _stderr: &mut dyn Write,
) -> Result<(), String> {
    let cli = parse_args(args)?;
    if cli.help {
        write!(stdout, "{HELP_TEXT}").map_err(|err| err.to_string())?;
        return Ok(());
    }

    let config_options = load_config(cwd, cli.config.as_deref())?;
    let options = merge_options(cli, config_options, cwd)?;
    let context = build_context(cwd, options)?;
    let plan = build_plan(&context)?;

    let mut logger = Logger::default();
    if context.verbose >= 1 {
        for package in &context.packages {
            if context.delete {
                logger.push(format!("deleting package '{}'", package.name));
            } else {
                logger.push(format!("stowing package '{}'", package.name));
            }
        }
    }

    for operation in &plan.operations {
        if context.verbose >= 2 {
            logger.push(render_operation(operation));
        }
    }

    if !context.dry_run {
        apply_plan(&plan)?;
    }

    if context.verbose >= 1 {
        if context.delete {
            logger.push(format!("removed {} symlinks", plan.unlink_count));
        } else {
            logger.push(format!("created {} symlinks", plan.link_count));
        }
    }

    for line in logger.lines {
        writeln!(stdout, "{line}").map_err(|err| err.to_string())?;
    }

    Ok(())
}

fn parse_args(args: &[String]) -> Result<RawOptions, String> {
    parse_args_with_mode(args, true)
}

fn parse_args_with_mode(args: &[String], require_packages: bool) -> Result<RawOptions, String> {
    let mut raw = RawOptions::default();
    let mut iter = args.iter().skip(1).peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => raw.help = true,
            "-N" | "--dry-run" => raw.dry_run = true,
            "-I" | "--ignore-conflicts" => raw.ignore_conflicts = true,
            "-D" | "--delete" => raw.delete = true,
            "-T" | "--target" => {
                raw.target = Some(
                    iter.next()
                        .ok_or_else(|| "missing value for --target".to_string())?
                        .clone(),
                );
            }
            "-v" | "--verbose" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "missing value for --verbose".to_string())?;
                raw.verbose = Some(parse_verbose(value)?);
            }
            "-F" | "--fold" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "missing value for --fold".to_string())?;
                raw.folds.push(value.clone());
                raw.fold_set_by_cli = true;
            }
            "-C" | "--config" => {
                raw.config = Some(
                    iter.next()
                        .ok_or_else(|| "missing value for --config".to_string())?
                        .clone(),
                );
            }
            _ if arg.starts_with("--target=") => {
                raw.target = Some(arg[9..].to_string());
            }
            _ if arg.starts_with("--verbose=") => {
                raw.verbose = Some(parse_verbose(&arg[10..])?);
            }
            _ if arg.starts_with("--fold=") => {
                raw.folds.push(arg[7..].to_string());
                raw.fold_set_by_cli = true;
            }
            _ if arg.starts_with("--config=") => {
                raw.config = Some(arg[9..].to_string());
            }
            _ if arg.starts_with('-') => {
                return Err(format!("unknown option '{arg}'"));
            }
            _ => raw.packages.push(arg.clone()),
        }
    }

    if require_packages && !raw.help && raw.packages.is_empty() {
        return Err("at least one <PACKAGE> is required".to_string());
    }

    Ok(raw)
}

fn parse_verbose(value: &str) -> Result<u8, String> {
    match value.parse::<u8>() {
        Ok(level @ 0..=2) => Ok(level),
        _ => Err(format!(
            "invalid verbose level '{value}': expected 0, 1, or 2"
        )),
    }
}

fn load_config(cwd: &Path, explicit_config: Option<&str>) -> Result<RawOptions, String> {
    let config_path = if let Some(path) = explicit_config {
        expand_and_absolutize(path, cwd)?
    } else {
        cwd.join(".ministowrc")
    };

    if !config_path.exists() {
        if explicit_config.is_some() {
            return Err(format!(
                "config file '{}' does not exist",
                config_path.display()
            ));
        }
        return Ok(RawOptions::default());
    }

    let contents = fs::read_to_string(&config_path)
        .map_err(|err| format!("failed to read config '{}': {err}", config_path.display()))?;

    let mut synthetic_args = vec!["ministow".to_string()];
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        synthetic_args.push(trimmed.to_string());
    }

    parse_args_with_mode(&synthetic_args, false)
}

fn merge_options(
    cli: RawOptions,
    config: RawOptions,
    cwd: &Path,
) -> Result<EffectiveOptions, String> {
    let target = if let Some(value) = cli.target.as_deref() {
        expand_and_absolutize(value, cwd)?
    } else if let Some(value) = config.target.as_deref() {
        expand_and_absolutize(value, cwd)?
    } else {
        cwd.parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| "could not determine default target directory".to_string())?
    };

    let verbose = cli.verbose.or(config.verbose).unwrap_or(0);
    let folds = if cli.fold_set_by_cli {
        cli.folds.into_iter().collect()
    } else {
        config.folds.into_iter().collect()
    };
    let dry_run = cli.dry_run || config.dry_run;
    let ignore_conflicts = cli.ignore_conflicts || config.ignore_conflicts;

    if ignore_conflicts && !dry_run {
        return Err("--ignore-conflicts requires --dry-run".to_string());
    }

    let packages = cli
        .packages
        .iter()
        .map(|package| normalize_package_name(package))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(EffectiveOptions {
        target,
        verbose,
        folds,
        dry_run,
        ignore_conflicts,
        delete: cli.delete || config.delete,
        packages,
    })
}

fn normalize_package_name(package: &str) -> Result<String, String> {
    let normalized = package.trim_end_matches(['/', '\\']);
    if normalized.is_empty() {
        return Err(format!("invalid package name '{package}'"));
    }
    Ok(normalized.to_string())
}

fn build_context(cwd: &Path, options: EffectiveOptions) -> Result<AppContext, String> {
    if !options.target.exists() {
        return Err(format!(
            "target directory '{}' does not exist",
            options.target.display()
        ));
    }
    if !options.target.is_dir() {
        return Err(format!(
            "target directory '{}' is not a directory",
            options.target.display()
        ));
    }

    let mut packages = Vec::new();
    let package_names: HashSet<_> = options.packages.iter().cloned().collect();
    for name in &options.packages {
        let root = cwd.join(name);
        if !root.is_dir() {
            return Err(format!("package '{}' does not exist", name));
        }
        packages.push(Package {
            name: name.clone(),
            root,
        });
    }

    let active_folds: BTreeSet<_> = options
        .folds
        .iter()
        .filter(|fold| match fold_package_name(fold) {
            Some(package) => package_names.contains(package),
            None => true,
        })
        .cloned()
        .collect();

    for fold in &active_folds {
        validate_fold_path(cwd, &package_names, fold)?;
    }

    Ok(AppContext {
        target: options.target,
        verbose: options.verbose,
        dry_run: options.dry_run,
        ignore_conflicts: options.ignore_conflicts,
        delete: options.delete,
        packages,
        folds: active_folds,
    })
}

fn fold_package_name(fold: &str) -> Option<&str> {
    let mut components = Path::new(fold).components();
    match components.next() {
        Some(Component::Normal(name)) => name.to_str(),
        _ => None,
    }
}

fn validate_fold_path(
    cwd: &Path,
    package_names: &HashSet<String>,
    fold: &str,
) -> Result<(), String> {
    let fold_path = Path::new(fold);
    let package = fold_package_name(fold)
        .ok_or_else(|| format!("fold path '{}' does not exist in package", fold))?;

    if !package_names.contains(package) {
        return Err(format!("fold path '{}' does not exist in package", fold));
    }

    let remainder = fold_path
        .strip_prefix(&package)
        .map_err(|_| format!("fold path '{}' does not exist in package", fold))?;

    if remainder.as_os_str().is_empty() {
        return Err(format!("fold path '{}' does not exist in package", fold));
    }

    let candidate = cwd.join(&package).join(remainder);
    if !candidate.is_dir() {
        return Err(format!("fold path '{}' does not exist in package", fold));
    }

    Ok(())
}

fn build_plan(context: &AppContext) -> Result<Plan, String> {
    let mut mkdirs = BTreeSet::new();
    let mut links = BTreeMap::new();
    let mut unlinks = BTreeSet::new();
    let mut cleanup_dirs = BTreeSet::new();

    for package in &context.packages {
        if context.delete {
            collect_delete_operations(
                package,
                &context.target,
                &context.folds,
                &mut unlinks,
                &mut cleanup_dirs,
            )?;
        } else {
            collect_install_operations(
                package,
                &context.target,
                &context.folds,
                &mut mkdirs,
                &mut links,
            )?;
        }
    }

    if !(context.dry_run && context.ignore_conflicts && !context.delete) {
        validate_install_plan(&context.target, &mkdirs, &links)?;
    }
    validate_delete_plan(&unlinks, &context.packages)?;

    let mut operations = Vec::new();
    let mut link_count = 0;
    let mut unlink_count = 0;

    if context.delete {
        for path in unlinks {
            operations.push(Operation::Unlink(path));
            unlink_count += 1;
        }

        let removable_dirs = plan_cleanup_dirs(&cleanup_dirs, &operations)?;
        operations.extend(removable_dirs.into_iter().map(Operation::Rmdir));
    } else {
        for path in mkdirs {
            if should_create_dir(&path)? {
                operations.push(Operation::Mkdir(path));
            }
        }
        for (target, source) in links {
            if should_create_link(&target)? {
                operations.push(Operation::Link { target, source });
                link_count += 1;
            }
        }
    }

    Ok(Plan {
        operations,
        link_count,
        unlink_count,
    })
}

fn collect_install_operations(
    package: &Package,
    target_root: &Path,
    folds: &BTreeSet<String>,
    mkdirs: &mut BTreeSet<PathBuf>,
    links: &mut BTreeMap<PathBuf, PathBuf>,
) -> Result<(), String> {
    collect_install_recursive(package, target_root, folds, Path::new(""), mkdirs, links)
}

fn collect_install_recursive(
    package: &Package,
    target_root: &Path,
    folds: &BTreeSet<String>,
    relative: &Path,
    mkdirs: &mut BTreeSet<PathBuf>,
    links: &mut BTreeMap<PathBuf, PathBuf>,
) -> Result<(), String> {
    let source_dir = package.root.join(relative);
    let mut entries = fs::read_dir(&source_dir)
        .map_err(|err| format!("failed to read '{}': {err}", source_dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read '{}': {err}", source_dir.display()))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let name = entry.file_name();
        let child_relative = relative.join(&name);
        let source_path = package.root.join(&child_relative);
        let target_path = target_root.join(&child_relative);
        let fold_key = package_fold_key(&package.name, &child_relative);

        if source_path.is_dir() {
            if folds.contains(&fold_key) {
                ensure_parent_dirs(target_root, &child_relative, mkdirs);
                if let Some(existing) = links.insert(target_path, source_path) {
                    return Err(format!(
                        "conflict at '{}': planned by '{}' and '{}'",
                        child_relative.display(),
                        existing.display(),
                        package.root.join(&child_relative).display()
                    ));
                }
            } else {
                collect_install_recursive(
                    package,
                    target_root,
                    folds,
                    &child_relative,
                    mkdirs,
                    links,
                )?;
            }
        } else if source_path.is_file() {
            ensure_parent_dirs(target_root, &child_relative, mkdirs);
            if let Some(existing) = links.insert(target_path, source_path) {
                return Err(format!(
                    "conflict at '{}': planned by '{}' and '{}'",
                    child_relative.display(),
                    existing.display(),
                    package.root.join(&child_relative).display()
                ));
            }
        }
    }

    Ok(())
}

fn collect_delete_operations(
    package: &Package,
    target_root: &Path,
    folds: &BTreeSet<String>,
    unlinks: &mut BTreeSet<PathBuf>,
    cleanup_dirs: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    collect_delete_recursive(
        package,
        target_root,
        folds,
        Path::new(""),
        unlinks,
        cleanup_dirs,
    )
}

fn collect_delete_recursive(
    package: &Package,
    target_root: &Path,
    folds: &BTreeSet<String>,
    relative: &Path,
    unlinks: &mut BTreeSet<PathBuf>,
    cleanup_dirs: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let source_dir = package.root.join(relative);
    let mut entries = fs::read_dir(&source_dir)
        .map_err(|err| format!("failed to read '{}': {err}", source_dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read '{}': {err}", source_dir.display()))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let name = entry.file_name();
        let child_relative = relative.join(&name);
        let source_path = package.root.join(&child_relative);
        let target_path = target_root.join(&child_relative);
        let fold_key = package_fold_key(&package.name, &child_relative);

        if source_path.is_dir() {
            if folds.contains(&fold_key) {
                maybe_plan_unlink(package, &source_path, &target_path, unlinks, cleanup_dirs)?;
            } else {
                collect_delete_recursive(
                    package,
                    target_root,
                    folds,
                    &child_relative,
                    unlinks,
                    cleanup_dirs,
                )?;
            }
        } else if source_path.is_file() {
            maybe_plan_unlink(package, &source_path, &target_path, unlinks, cleanup_dirs)?;
        }
    }

    Ok(())
}

fn maybe_plan_unlink(
    package: &Package,
    source_path: &Path,
    target_path: &Path,
    unlinks: &mut BTreeSet<PathBuf>,
    cleanup_dirs: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(target_path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(format!(
                "failed to inspect '{}': {err}",
                target_path.display()
            ))
        }
    };

    if !metadata.file_type().is_symlink() {
        return Ok(());
    }

    let resolved = resolve_symlink_target(target_path)?;
    if resolved == source_path {
        unlinks.insert(target_path.to_path_buf());
        add_cleanup_parents(target_path, cleanup_dirs);
        return Ok(());
    }

    let package_root = canonicalize_best_effort(&package.root)?;
    if resolved.starts_with(&package_root) {
        return Err(format!(
            "cannot delete '{}': symlink does not belong to package '{}'",
            target_path.display(),
            package.name
        ));
    }

    Ok(())
}

fn validate_install_plan(
    target_root: &Path,
    mkdirs: &BTreeSet<PathBuf>,
    links: &BTreeMap<PathBuf, PathBuf>,
) -> Result<(), String> {
    for dir in mkdirs {
        if links.contains_key(dir) {
            return Err(format!(
                "conflict at '{}': path is both directory and symlink",
                dir.display()
            ));
        }
        match fs::symlink_metadata(dir) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                return Err(format!(
                    "conflict at '{}': existing file is not a compatible directory",
                    dir.display()
                ))
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(format!("failed to inspect '{}': {err}", dir.display())),
        }
    }

    for (target, source) in links {
        for ancestor in target.ancestors().skip(1) {
            if ancestor == target_root {
                break;
            }
            if links.contains_key(ancestor) {
                return Err(format!(
                    "conflict at '{}': parent path is planned as a symlink",
                    target.display()
                ));
            }
        }

        match fs::symlink_metadata(target) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let resolved = resolve_symlink_target(target)?;
                if resolved != *source {
                    return Err(format!(
                        "conflict at '{}': existing file is not a matching symlink",
                        target.display()
                    ));
                }
            }
            Ok(metadata) if metadata.is_dir() => {
                return Err(format!(
                    "conflict at '{}': existing directory is not a matching symlink",
                    target.display()
                ));
            }
            Ok(_) => {
                return Err(format!(
                    "conflict at '{}': existing file is not a matching symlink",
                    target.display()
                ));
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(format!("failed to inspect '{}': {err}", target.display())),
        }
    }

    Ok(())
}

fn validate_delete_plan(unlinks: &BTreeSet<PathBuf>, packages: &[Package]) -> Result<(), String> {
    let package_roots = packages
        .iter()
        .map(|package| canonicalize_best_effort(&package.root))
        .collect::<Result<Vec<_>, _>>()?;

    for target in unlinks {
        let resolved = resolve_symlink_target(target)?;
        if !package_roots.iter().any(|root| resolved.starts_with(root)) {
            return Err(format!(
                "cannot delete '{}': symlink does not belong to selected packages",
                target.display()
            ));
        }
    }

    Ok(())
}

fn plan_cleanup_dirs(
    candidates: &BTreeSet<PathBuf>,
    operations: &[Operation],
) -> Result<Vec<PathBuf>, String> {
    let unlink_targets = operations
        .iter()
        .filter_map(|operation| match operation {
            Operation::Unlink(path) => Some(path.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    let mut ordered = candidates.iter().cloned().collect::<Vec<_>>();
    ordered.sort_by_key(|path| std::cmp::Reverse(path.components().count()));

    let mut removable = Vec::new();
    let mut removable_set = HashSet::new();

    for dir in ordered {
        if is_dir_removable(&dir, &unlink_targets, &removable_set)? {
            removable_set.insert(dir.clone());
            removable.push(dir);
        }
    }

    Ok(removable)
}

fn should_create_dir(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(!metadata.is_dir()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(err) => Err(format!("failed to inspect '{}': {err}", path.display())),
    }
}

fn should_create_link(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(err) => Err(format!("failed to inspect '{}': {err}", path.display())),
    }
}

fn is_dir_removable(
    dir: &Path,
    unlink_targets: &HashSet<PathBuf>,
    removable_dirs: &HashSet<PathBuf>,
) -> Result<bool, String> {
    let metadata = match fs::symlink_metadata(dir) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to inspect '{}': {err}", dir.display())),
    };

    if !metadata.is_dir() {
        return Ok(false);
    }

    let entries = fs::read_dir(dir)
        .map_err(|err| format!("failed to read '{}': {err}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read '{}': {err}", dir.display()))?;

    for entry in entries {
        let path = entry.path();
        if unlink_targets.contains(&path) || removable_dirs.contains(&path) {
            continue;
        }
        return Ok(false);
    }

    Ok(true)
}

fn apply_plan(plan: &Plan) -> Result<(), String> {
    for operation in &plan.operations {
        match operation {
            Operation::Mkdir(path) => {
                if !path.exists() {
                    fs::create_dir(path)
                        .map_err(|err| format!("failed to create '{}': {err}", path.display()))?;
                }
            }
            Operation::Link { target, source } => {
                if fs::symlink_metadata(target).is_ok() {
                    continue;
                }
                let parent = target.parent().ok_or_else(|| {
                    format!(
                        "cannot determine parent directory for '{}'",
                        target.display()
                    )
                })?;
                let relative = relative_path(source, parent);
                #[cfg(unix)]
                unix_fs::symlink(&relative, target).map_err(|err| {
                    format!(
                        "failed to link '{}' -> '{}': {err}",
                        target.display(),
                        relative.display()
                    )
                })?;
            }
            Operation::Unlink(path) => match fs::remove_file(path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(format!("failed to unlink '{}': {err}", path.display())),
            },
            Operation::Rmdir(path) => match fs::remove_dir(path) {
                Ok(()) => {}
                Err(err)
                    if matches!(
                        err.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                    ) => {}
                Err(err) => return Err(format!("failed to remove '{}': {err}", path.display())),
            },
        }
    }

    Ok(())
}

fn render_operation(operation: &Operation) -> String {
    match operation {
        Operation::Mkdir(path) => format!("mkdir {}", display_path(path)),
        Operation::Link { target, source } => {
            let parent = target.parent().unwrap_or_else(|| Path::new("/"));
            let relative = relative_path(source, parent);
            format!("link {} -> {}", display_path(target), relative.display())
        }
        Operation::Unlink(path) => format!("unlink {}", display_path(path)),
        Operation::Rmdir(path) => format!("rmdir {}", display_path(path)),
    }
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn add_cleanup_parents(path: &Path, cleanup_dirs: &mut BTreeSet<PathBuf>) {
    let mut current = path.parent();
    while let Some(dir) = current {
        cleanup_dirs.insert(dir.to_path_buf());
        current = dir.parent();
    }
}

fn ensure_parent_dirs(target_root: &Path, relative: &Path, mkdirs: &mut BTreeSet<PathBuf>) {
    if let Some(parent) = relative.parent() {
        let mut cursor = PathBuf::new();
        for component in parent.components() {
            cursor.push(component.as_os_str());
            mkdirs.insert(target_root.join(&cursor));
        }
    }
}

fn package_fold_key(package: &str, relative: &Path) -> String {
    let mut path = PathBuf::from(package);
    path.push(relative);
    path.to_string_lossy().to_string()
}

fn resolve_symlink_target(path: &Path) -> Result<PathBuf, String> {
    let raw = fs::read_link(path)
        .map_err(|err| format!("failed to read symlink '{}': {err}", path.display()))?;
    let absolute = if raw.is_absolute() {
        raw
    } else {
        path.parent().unwrap_or_else(|| Path::new(".")).join(raw)
    };
    canonicalize_best_effort(&absolute)
}

fn canonicalize_best_effort(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path).map_err(|err| format!("failed to resolve '{}': {err}", path.display()))
}

fn relative_path(target: &Path, base: &Path) -> PathBuf {
    let target_components = normalized_components(target);
    let base_components = normalized_components(base);

    let common = target_components
        .iter()
        .zip(base_components.iter())
        .take_while(|(left, right)| left == right)
        .count();

    let mut result = PathBuf::new();
    for _ in common..base_components.len() {
        result.push("..");
    }
    for component in target_components.iter().skip(common) {
        result.push(component);
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    result
}

fn normalized_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::RootDir => Some(std::path::MAIN_SEPARATOR.to_string()),
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect()
}

fn expand_and_absolutize(value: &str, cwd: &Path) -> Result<PathBuf, String> {
    let expanded = expand_path_value(value)?;
    let path = PathBuf::from(expanded);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(cwd.join(path))
    }
}

fn expand_path_value(value: &str) -> Result<String, String> {
    let mut output = String::new();
    let chars = value.chars().collect::<Vec<_>>();
    let mut index = 0;

    if value.starts_with('~') {
        let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
        output.push_str(&home);
        index += 1;
    }

    while index < chars.len() {
        if chars[index] != '$' {
            output.push(chars[index]);
            index += 1;
            continue;
        }

        if index + 1 >= chars.len() {
            output.push('$');
            break;
        }

        if chars[index + 1] == '{' {
            let mut end = index + 2;
            while end < chars.len() && chars[end] != '}' {
                end += 1;
            }
            if end >= chars.len() {
                return Err(format!("invalid environment expansion '{value}'"));
            }
            let name = chars[index + 2..end].iter().collect::<String>();
            output.push_str(
                &std::env::var(&name)
                    .map_err(|_| format!("environment variable '{name}' is not set"))?,
            );
            index = end + 1;
            continue;
        }

        let mut end = index + 1;
        while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
            end += 1;
        }
        let name = chars[index + 1..end].iter().collect::<String>();
        if name.is_empty() {
            output.push('$');
            index += 1;
            continue;
        }
        output.push_str(
            &std::env::var(&name)
                .map_err(|_| format!("environment variable '{name}' is not set"))?,
        );
        index = end;
    }

    Ok(output)
}

#[allow(dead_code)]
fn _is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

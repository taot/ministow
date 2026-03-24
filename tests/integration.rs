use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs as unix_fs;

fn setup_repo() -> TempDir {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let target = temp.path().join("target-home");

    fs::create_dir_all(repo.join("base/bin")).unwrap();
    fs::create_dir_all(repo.join("wezterm/.config/wezterm")).unwrap();
    fs::create_dir_all(repo.join("fcitx/.local/share/fcitx5/rime")).unwrap();
    fs::create_dir_all(&target).unwrap();

    fs::write(repo.join("base/.bashrc.init"), "init\n").unwrap();
    fs::write(repo.join("base/.bashrc.env"), "env\n").unwrap();
    fs::write(repo.join("base/.tmux.conf"), "tmux\n").unwrap();
    fs::write(repo.join("base/bin/foreach"), "#!/bin/sh\n").unwrap();
    fs::write(
        repo.join("wezterm/.config/wezterm/wezterm.lua"),
        "return {}\n",
    )
    .unwrap();
    fs::write(
        repo.join("fcitx/.local/share/fcitx5/rime/default.yaml"),
        "patch:\n",
    )
    .unwrap();

    temp
}

fn bin() -> Command {
    Command::cargo_bin("ministow").unwrap()
}

fn read_link_target(path: &Path) -> String {
    fs::read_link(path).unwrap().display().to_string()
}

#[test]
fn installs_basic_package_with_default_target() {
    let temp = setup_repo();
    let repo = temp.path().join("repo");
    let target = temp.path().to_path_buf();

    bin().current_dir(&repo).arg("base").assert().success();

    assert!(target
        .join(".bashrc.init")
        .symlink_metadata()
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(target.join("bin").is_dir());
    assert_eq!(
        read_link_target(&target.join("bin/foreach")),
        "../repo/base/bin/foreach"
    );
}

#[test]
fn respects_explicit_target_and_fold_rules() {
    let temp = setup_repo();
    let repo = temp.path().join("repo");
    let target = temp.path().join("target-home");

    bin()
        .current_dir(&repo)
        .args([
            "--target",
            target.to_str().unwrap(),
            "--fold=wezterm/.config/wezterm",
            "wezterm",
        ])
        .assert()
        .success();

    let folded = target.join(".config/wezterm");
    assert!(folded.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(
        read_link_target(&folded),
        "../../repo/wezterm/.config/wezterm"
    );
}

#[test]
fn deletes_package_and_cleans_empty_directories() {
    let temp = setup_repo();
    let repo = temp.path().join("repo");
    let target = temp.path().join("target-home");

    bin()
        .current_dir(&repo)
        .args(["--target", target.to_str().unwrap(), "base"])
        .assert()
        .success();

    bin()
        .current_dir(&repo)
        .args(["--target", target.to_str().unwrap(), "--delete", "base"])
        .assert()
        .success();

    assert!(!target.join(".bashrc.init").exists());
    assert!(!target.join("bin").exists());
}

#[test]
fn dry_run_reports_actions_without_changes() {
    let temp = setup_repo();
    let repo = temp.path().join("repo");
    let target = temp.path().to_path_buf();

    bin()
        .current_dir(&repo)
        .args(["--dry-run", "--verbose=2", "base"])
        .assert()
        .success()
        .stdout(predicates::str::contains(format!(
            "link {} -> repo/base/.bashrc.init",
            target.join(".bashrc.init").display()
        )));

    assert!(!temp.path().join("target-home/.bashrc.init").exists());
}

#[test]
fn config_file_is_loaded_and_cli_overrides_verbose() {
    let temp = setup_repo();
    let repo = temp.path().join("repo");
    let target = temp.path().join("target-home");

    fs::write(
        repo.join(".ministowrc"),
        format!(
            "--target={}\n--verbose=1\n--fold=wezterm/.config/wezterm\n",
            target.display()
        ),
    )
    .unwrap();

    bin()
        .current_dir(&repo)
        .args(["--verbose=2", "wezterm"])
        .assert()
        .success()
        .stdout(predicates::str::contains(format!(
            "link {} -> ../../repo/wezterm/.config/wezterm",
            target.join(".config/wezterm").display()
        )));
}

#[test]
fn ignores_configured_folds_for_unselected_packages() {
    let temp = setup_repo();
    let repo = temp.path().join("repo");

    fs::create_dir_all(repo.join("base/.bashrc.includes")).unwrap();
    fs::write(
        repo.join("base/.bashrc.includes/base"),
        "source ~/.bashrc.init\n",
    )
    .unwrap();
    fs::write(
        repo.join(".ministowrc"),
        "--fold=base/.bashrc.includes\n--fold=wezterm/.config/wezterm\n",
    )
    .unwrap();

    bin()
        .current_dir(&repo)
        .args(["--dry-run", "--verbose=2", "wezterm"])
        .assert()
        .success()
        .stdout(predicates::str::contains("link "))
        .stderr(predicates::str::is_empty());
}

#[test]
fn detects_conflicts_without_partial_changes() {
    let temp = setup_repo();
    let repo = temp.path().join("repo");
    let target = temp.path().to_path_buf();

    fs::write(target.join(".bashrc.init"), "conflict\n").unwrap();

    bin()
        .current_dir(&repo)
        .arg("base")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "existing file is not a matching symlink",
        ));

    assert!(!target.join(".bashrc.env").exists());
}

#[test]
fn reruns_install_and_delete_idempotently() {
    let temp = setup_repo();
    let repo = temp.path().join("repo");
    let target = temp.path().join("target-home");

    bin().current_dir(&repo).arg("base").assert().success();
    bin().current_dir(&repo).arg("base").assert().success();

    bin()
        .current_dir(&repo)
        .args(["--delete", "base"])
        .assert()
        .success();
    bin()
        .current_dir(&repo)
        .args(["--delete", "base"])
        .assert()
        .success();

    assert!(!target.join(".bashrc.init").exists());
}

#[test]
fn help_output_describes_usage() {
    let temp = setup_repo();
    let repo = temp.path().join("repo");

    bin()
        .current_dir(&repo)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage:"))
        .stdout(predicates::str::contains("--delete"));
}

#[test]
fn multiple_fold_directives_work_together() {
    let temp = setup_repo();
    let repo = temp.path().join("repo");
    let target = temp.path().join("target-home");

    bin()
        .current_dir(&repo)
        .args([
            "--target",
            target.to_str().unwrap(),
            "--fold=wezterm/.config/wezterm",
            "--fold=fcitx/.local/share/fcitx5/rime",
            "wezterm",
            "fcitx",
        ])
        .assert()
        .success();

    assert!(target
        .join(".config/wezterm")
        .symlink_metadata()
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(target
        .join(".local/share/fcitx5/rime")
        .symlink_metadata()
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn delete_folded_directory_removes_symlink() {
    let temp = setup_repo();
    let repo = temp.path().join("repo");
    let target = temp.path().join("target-home");

    bin()
        .current_dir(&repo)
        .args([
            "--target",
            target.to_str().unwrap(),
            "--fold=wezterm/.config/wezterm",
            "wezterm",
        ])
        .assert()
        .success();

    bin()
        .current_dir(&repo)
        .args([
            "--target",
            target.to_str().unwrap(),
            "--fold=wezterm/.config/wezterm",
            "--delete",
            "wezterm",
        ])
        .assert()
        .success();

    assert!(!target.join(".config/wezterm").exists());
}

#[test]
fn refuses_to_delete_symlink_owned_by_another_package() {
    let temp = setup_repo();
    let repo = temp.path().join("repo");
    let target = temp.path().join("target-home");

    fs::create_dir_all(target.join(".config")).unwrap();
    #[cfg(unix)]
    unix_fs::symlink(
        repo.join("fcitx/.local/share/fcitx5/rime"),
        target.join(".config/wezterm"),
    )
    .unwrap();

    bin()
        .current_dir(&repo)
        .args([
            "--target",
            target.to_str().unwrap(),
            "--fold=wezterm/.config/wezterm",
            "--delete",
            "wezterm",
        ])
        .assert()
        .success();
}

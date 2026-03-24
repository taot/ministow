# Product Requirements Document: ministow

## Overview

`ministow` is a Rust command-line tool for managing config files using symlinks, similar to GNU Stow. It preserves Stow's package-based workflow while adding one key capability: configurable tree folding on specific directories.

The primary use case is managing a dotfiles repository where each first-level directory is a package, and package contents are linked into a target directory such as `$HOME`.

## Problem Statement

GNU Stow is effective for managing config files as symlink farms, but its tree-folding behavior is global rather than selectively configurable. Some directories should be linked as whole directories, while others should be unfolded so individual files remain linked separately.

The tool should let users choose exactly which package subdirectories are folded.

## Goals

- Provide a CLI workflow similar to GNU Stow for installing package contents into a target directory.
- Provide a CLI workflow for deleting symlinks previously created for a package.
- Treat each first-level directory under the working directory as a package.
- Preserve relative file layout from package to target.
- Allow users to explicitly choose which package subdirectories should be tree-folded.
- Support configuration through both CLI arguments and a config file.
- Support dry-run planning, including an option to ignore install conflicts while planning.
- Be safe, predictable, and idempotent.

## Non-Goals

- Full GNU Stow feature parity in v1.
- Restow/refold logic in v1.
- Ignore rules, adopt mode, override/defer behavior, or package ownership tracking beyond safe install-time checks.
- Windows-specific symlink handling optimizations.

## Target Users

- Developers managing dotfiles in Git repositories.
- Users already familiar with GNU Stow who want selective tree folding.
- Users with package layouts like:

```text
repo/
  base/
  editors/
  fcitx/
  wezterm/
```

## Example Repository Model

Each first-level directory is a package:

```text
taothomeconfig/
  base/
  editors/
  fcitx/
  wezterm/
```

Examples:

- `ministow base`
- `ministow --delete base`
- `ministow --fold=wezterm/.config/wezterm wezterm`
- `ministow --fold=fcitx/.local/share/fcitx5/rime fcitx`

## Core Behavior

### Package Layout

- The working directory is the stow directory.
- Each first-level subdirectory is a package.
- Package contents define the desired target layout relative to the target directory.

### Default Linking Behavior

When no matching fold rule is present:

- Recreate parent directories in the target as needed.
- Symlink files individually.
- Preserve relative paths from package root.

Example:

```text
base/.bashrc.init -> $HOME/.bashrc.init
base/.bashrc.env -> $HOME/.bashrc.env
base/.tmux.conf -> $HOME/.tmux.conf
base/bin/foreach -> $HOME/bin/foreach
```

### Folding Behavior

If a directory path is specified via `--fold`, that directory is symlinked as a whole instead of descending into it and linking files individually.

Example:

```bash
ministow --fold=wezterm/.config/wezterm wezterm
```

Expected result:

```text
/path/to/target/.config/wezterm -> <repo>/wezterm/.config/wezterm
```

Instead of separate links for files inside `wezterm/.config/wezterm/`.

### Delete Behavior

If `-D` or `--delete` is specified, `ministow` removes symlinks in the target that belong to the given package.

- Only symlinks pointing into the selected package may be removed.
- Empty directories created only to contain removed links should be cleaned up.
- Directories or files not owned by the package must not be removed.
- When a folded directory symlink was created for a package, deleting that package removes the folded symlink.

Example:

```bash
ministow --delete base
```

Expected result:

```text
$HOME/.bashrc.init removed
$HOME/.bashrc.env removed
$HOME/.tmux.conf removed
$HOME/bin/foreach removed
```

### Fold Matching Rules

- Fold paths are package-relative including package name, e.g. `wezterm/.config/wezterm`.
- Matching is exact on directory path.
- `--fold` may be specified multiple times.
- If a directory matches a fold rule, traversal stops at that directory and a directory symlink is created.

## CLI Requirements

### Command Syntax

```bash
ministow [OPTIONS] <PACKAGE>...
```

The default action is install/stow. Use `-D` or `--delete` to unstow packages.

### Positional Arguments

- `<PACKAGE>...`
  - One or more package names.
  - Each package must correspond to an existing first-level directory in the working directory.

### Options

#### `--target=<DIR>`, `-T <DIR>`

- Set the target directory.
- If omitted, default to the parent directory of the current working directory.

#### `--verbose=<LEVEL>`, `-v <LEVEL>`

- Verbosity levels: `0`, `1`, `2`
- Default: `0`

Behavior:

- `0`: errors only
- `1`: high-level operations
- `2`: detailed per-path actions

#### `--fold=<DIR>`, `-F <DIR>`

- Mark a package subdirectory for tree folding.
- Repeatable.
- Example:
  - `--fold=wezterm/.config/wezterm`
  - `--fold=fcitx/.local/share/fcitx5/rime`

#### `--config=<FILE>`, `-C <FILE>`

- Use a specific config file instead of the default.

#### `--dry-run`, `-N`

- Do not perform any operations that modify the file system.
- In combination with verbose output, show what would happen without applying changes.
- Applies to both install and delete actions.

#### `--ignore-conflicts`, `-I`

- Only valid together with `--dry-run`.
- Skips install target conflict validation during planning so users can inspect the operations that would still be performed.
- Does not affect delete-mode ownership checks.

#### `-D`, `--delete`

- Delete symlinks for the given package or packages from the target directory.
- The same package argument rules apply as install mode.
- When omitted, the action is install/stow.

#### `--help`, `-h`

- Print the help message and exit.

## Config File Requirements

### Default Location

- `.ministowrc` in the current working directory.

### Explicit Location

- `--config=<FILE>` overrides the default config file path.

### Config Format

The config file contains command-line style options, one per line.

- Blank lines are ignored.
- Lines starting with `#` are ignored.
- Boolean flags may appear without a value.

Example:

```text
--target=$HOME
--verbose=1
--fold=wezterm/.config/wezterm
--fold=fcitx/.local/share/fcitx5/rime
```

### Precedence Rules

- All CLI options override config file values.
- For single-value options, the CLI value wins.
- For `--fold`, any CLI-provided fold list replaces the config-provided fold list.
- Boolean flags are enabled if either CLI or config enables them.

### Environment Expansion

For config values that are file paths:

- Support `$VAR`
- Support `${VAR}`
- Support `~` where applicable

Examples:

- `--target=$HOME`
- `--target=${HOME}`
- `--target=~/`

## Safety and Conflict Handling

### General Principle

`ministow` must avoid destructive behavior.

### Required Conflict Behavior

The tool must fail with a clear error if the destination path already exists and is not safely compatible.

Cases:

- Existing regular file at target path where symlink is needed -> error
- Existing directory at target path where file symlink is needed -> error
- Existing symlink pointing somewhere else -> error
- Existing file, directory, or symlink at a path that must act as a parent directory -> error

For delete mode:

- Existing non-symlink files matching package paths must not be removed.
- Existing symlinks not pointing into the selected package must not be removed.
- Existing directories containing entries not owned by the selected package must not be removed.
- A mismatched symlink that still resolves inside the selected package root is an error.

### No-Op Cases

The tool should treat these as success without modifying anything:

- Target symlink already exists and points to the intended source
- Target directory already exists and is needed only as a parent directory for nested file links

### Directory Creation

- Parent directories in the target should be created automatically when needed.
- Existing directories may be reused if they are compatible with the planned install.
- The target directory itself must already exist and must be a directory.

## Idempotency

Running the same command multiple times should produce the same filesystem state and should not fail if the desired symlinks already exist correctly.

Running the same delete command multiple times should also be safe and should succeed even if the package's symlinks have already been removed.

## Logging Requirements

### Verbose Level 0

- Show only errors.

### Verbose Level 1

- Show package-level actions and summary.

Examples:

- `stowing package 'base'`
- `deleting package 'base'`
- `created 4 symlinks`
- `removed 4 symlinks`

### Verbose Level 2

- Show every planned/applied filesystem action.
- In dry-run mode, show every planned filesystem action without applying it.
- Omit actions for links and directories that are already in the desired state.

Examples:

- `mkdir $HOME/bin`
- `link $HOME/.bashrc.init -> ../repo/base/.bashrc.init`
- `link $HOME/.config/wezterm -> ../repo/wezterm/.config/wezterm`
- `unlink $HOME/.bashrc.init`
- `rmdir $HOME/bin`

## Technical Requirements

- Implemented in Rust.
- Must run on Linux and MacOS.
- Use relative symlinks where practical.
- Must support UTF-8 file paths supported by Rust standard library behavior.
- Accept package names with trailing `/` or `\` and normalize them.
- Should be structured so restow/refold features can be added later.

## Proposed Internal Design

### Major Components

- CLI parsing
- Config loading and merge
- Filesystem traversal
- Fold rule matching
- Operation planning
- Conflict detection
- Operation execution
- Logging
- Ownership verification for delete operations

### Execution Model

Two-phase approach:

1. Plan
   - Scan packages
   - Resolve fold rules
   - Compute intended mkdir, symlink, unlink, and cleanup operations
   - Detect conflicts

2. Apply
   - Execute operations only if the full plan is conflict-free

## Error Handling Requirements

Errors should be clear and actionable.

The help output should clearly document all supported options, action flags, defaults, and usage syntax.

Examples:

- `package 'base' does not exist`
- `fold path 'wezterm/.config/wezterm' does not exist in package`
- `conflict at '$HOME/.bashrc.init': existing file is not a matching symlink`
- `conflict at '$HOME/.config': existing file is not a compatible directory`
- `target directory '$HOME' does not exist`
- `--ignore-conflicts requires --dry-run`
- `cannot delete '$HOME/.bashrc.init': symlink does not belong to package 'base'`

## Acceptance Criteria

### Basic Package Linking

Given package `base`, when the user runs:

```bash
ministow base
```

Then files inside `base/` are linked into the target preserving relative paths.

### Explicit Target

Given a package and explicit target:

```bash
ministow --target=$HOME base
```

Then links are created under `$HOME`.

### Delete Package

Given package `base` has already been stowed, when the user runs:

```bash
ministow --delete base
```

Then symlinks belonging to `base` are removed from the target.

### Delete Package With Short Flag

Given package `base` has already been stowed, when the user runs:

```bash
ministow -D base
```

Then the result is the same as `ministow --delete base`.

### Delete Folded Directory

Given:

```bash
ministow --target=$HOME --fold=wezterm/.config/wezterm wezterm
```

And the package was previously installed, when the user runs:

```bash
ministow --target=$HOME --fold=wezterm/.config/wezterm --delete wezterm
```

Then the folded symlink at `$HOME/.config/wezterm` is removed.

### Dry Run

Given package `base`, when the user runs:

```bash
ministow --dry-run --verbose=2 base
```

Then no filesystem changes are made, and the tool reports the operations it would perform.

### Dry Run Delete

Given package `base` has already been stowed, when the user runs:

```bash
ministow -N --delete --verbose=2 base
```

Then no filesystem changes are made, and the tool reports the delete operations it would perform.

### Dry Run Ignore Conflicts

Given an existing conflicting file at a target path, when the user runs:

```bash
ministow --dry-run --ignore-conflicts --verbose=2 base
```

Then install target conflict validation is skipped, only still-applicable operations are reported, and no filesystem changes are made.

### Help Output

When the user runs:

```bash
ministow --help
```

Or:

```bash
ministow -h
```

Then the tool prints a help message describing usage, arguments, options, defaults, and exits successfully.

### Directory Folding

Given:

```bash
ministow --target=$HOME --fold=wezterm/.config/wezterm wezterm
```

Then:

- `$HOME/.config/wezterm` is a symlink to the package directory
- files inside that directory are not individually linked

### Multiple Fold Directives

Given:

```bash
ministow --target=$HOME \
  --fold=wezterm/.config/wezterm \
  --fold=fcitx/.local/share/fcitx5/rime \
  wezterm fcitx
```

Then both specified directories are folded.

### Config File Loading

Given `.ministowrc` containing:

```text
--target=$HOME
--verbose=1
--fold=wezterm/.config/wezterm
```

When the user runs:

```bash
ministow wezterm
```

Then config values are applied automatically.

### CLI Override

Given `.ministowrc` containing `--verbose=1`, when the user runs:

```bash
ministow --verbose=2 wezterm
```

Then effective verbosity is `2`.

### CLI Fold List Replaces Config Fold List

Given `.ministowrc` contains fold rules and the user also passes one or more `--fold` options on the CLI, then only the CLI-provided fold rules are used.

### Ignore Configured Folds For Unselected Packages

Given `.ministowrc` contains fold rules for multiple packages, when the user stows only one package, then only fold rules relevant to the selected package set are validated and applied.

### Trailing Slash Package Name

Given a package `fcitx`, when the user runs:

```bash
ministow fcitx/
```

Then the tool treats it the same as `ministow fcitx`.

### Idempotent Re-Run

Given a package has already been stowed successfully, when the same command is run again, then the tool exits successfully and does not duplicate or break links.

### Idempotent Delete Re-Run

Given a package has already been deleted successfully, when the same delete command is run again, then the tool exits successfully and does not fail.

### Conflict Detection

Given an existing non-symlink file at a target path required by a package, when `ministow` is run, then it must fail without making partial changes.

## Milestones

### v1

- Install/stow
- Unstow/delete via `-D` and `--delete`
- Config file support
- Selective fold rules
- Dry-run support via `-N` and `--dry-run`
- Conflict-skipping planning via `-I` and `--ignore-conflicts`
- Built-in help output via `-h` and `--help`
- Safe conflict detection
- Idempotent behavior
- Verbose logging

### Future

- Restow
- Refolding after delete
- Ignore rules
- Dotfile rewriting mode similar to Stow `--dotfiles`

## Current v1 Decisions

- The target directory must already exist; `ministow` does not create it.
- Fold rules for selected packages are hard errors if they do not resolve to an existing directory.
- Fold rules for unselected packages are ignored.
- The default config file location is `.ministowrc` in the current working directory.

## Future Considerations

- Support reading an additional home-directory config file.

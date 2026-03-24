# Product Requirements Document: ministow

## Overview

`ministow` is a Rust command-line tool for managing config files using symlinks, similar to GNU Stow. It preserves Stow's package-based workflow while adding one key capability: configurable tree folding on specific directories.

The primary use case is managing a dotfiles repository where each first-level directory is a package, and package contents are linked into a target directory such as `$HOME`.

## Problem Statement

GNU Stow is effective for managing config files as symlink farms, but its tree-folding behavior is global rather than selectively configurable. Some directories should be linked as whole directories, while others should be unfolded so individual files remain linked separately.

The tool should let users choose exactly which package subdirectories are folded.

## Goals

- Provide a CLI workflow similar to GNU Stow for installing package contents into a target directory.
- Treat each first-level directory under the working directory as a package.
- Preserve relative file layout from package to target.
- Allow users to explicitly choose which package subdirectories should be tree-folded.
- Support configuration through both CLI arguments and a config file.
- Be safe, predictable, and idempotent.

## Non-Goals

- Full GNU Stow feature parity in v1.
- Unstow/delete support in v1.
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
$TARGET/.config/wezterm -> <repo>/wezterm/.config/wezterm
```

Instead of separate links for files inside `wezterm/.config/wezterm/`.

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

### Positional Arguments

- `<PACKAGE>...`
  - One or more package names.
  - Each package must correspond to an existing first-level directory in the working directory.

### Options

#### `--target=<DIR>`

- Set the target directory.
- If omitted, default to the parent directory of the current working directory.

#### `--verbose=<LEVEL>`

- Verbosity levels: `0`, `1`, `2`
- Default: `0`

Behavior:

- `0`: errors only
- `1`: high-level operations
- `2`: detailed per-path actions

#### `--fold=<DIR>`

- Mark a package subdirectory for tree folding.
- Repeatable.
- Example:
  - `--fold=wezterm/.config/wezterm`
  - `--fold=fcitx/.local/share/fcitx5/rime`

#### `--config=<FILE>`

- Use a specific config file instead of the default.

## Config File Requirements

### Default Location

- `.ministowrc` in the current working directory.

### Explicit Location

- `--config=<FILE>` overrides the default config file path.

### Config Format

The config file contains command-line style options, one per line. Example:

```text
--target=$HOME
--verbose=1
--fold=wezterm/.config/wezterm
--fold=fcitx/.local/share/fcitx5/rime
```

### Precedence Rules

- CLI options override config values for single-value options:
  - `--target`
  - `--verbose`
  - `--config`
- Repeatable options are additive:
  - `--fold` entries from config and CLI are combined

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

### No-Op Cases

The tool should treat these as success without modifying anything:

- Target symlink already exists and points to the intended source
- Target directory already exists and is needed only as a parent directory for nested file links

### Directory Creation

- Parent directories in the target should be created automatically when needed.
- Existing directories may be reused if they are compatible with the planned install.

## Idempotency

Running the same command multiple times should produce the same filesystem state and should not fail if the desired symlinks already exist correctly.

## Logging Requirements

### Verbose Level 0

- Show only errors.

### Verbose Level 1

- Show package-level actions and summary.

Examples:

- `stowing package 'base'`
- `created 4 symlinks`

### Verbose Level 2

- Show every planned/applied filesystem action.

Examples:

- `mkdir $HOME/bin`
- `link $HOME/.bashrc.init -> ../repo/base/.bashrc.init`
- `link $HOME/.config/wezterm -> ../repo/wezterm/.config/wezterm`

## Technical Requirements

- Implemented in Rust.
- Must run on Linux.
- Use relative symlinks where practical.
- Must support UTF-8 file paths supported by Rust standard library behavior.
- Should be structured so delete/restow features can be added later.

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

### Execution Model

Two-phase approach:

1. Plan
   - Scan packages
   - Resolve fold rules
   - Compute intended mkdir and symlink operations
   - Detect conflicts

2. Apply
   - Execute operations only if the full plan is conflict-free

## Error Handling Requirements

Errors should be clear and actionable.

Examples:

- `package 'base' does not exist`
- `fold path 'wezterm/.config/wezterm' does not exist in package`
- `conflict at '$HOME/.bashrc.init': existing file is not a matching symlink`
- `target directory '$HOME' does not exist`

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

### Idempotent Re-Run

Given a package has already been stowed successfully, when the same command is run again, then the tool exits successfully and does not duplicate or break links.

### Conflict Detection

Given an existing non-symlink file at a target path required by a package, when `ministow` is run, then it must fail without making partial changes.

## Milestones

### v1

- Install/stow only
- Config file support
- Selective fold rules
- Safe conflict detection
- Idempotent behavior
- Verbose logging

### Future

- Unstow/delete
- Restow
- Refolding after delete
- Ignore rules
- Dry-run mode
- Dotfile rewriting mode similar to Stow `--dotfiles`

## Open Questions

- Whether v1 should require the target directory to already exist or create it if missing.
- Whether fold rules that do not match any package path should be hard errors or warnings.
- Whether to support reading both `.ministowrc` in cwd and a home config file in the future.

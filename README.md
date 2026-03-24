# ministow

`ministow` is a small Rust CLI for managing dotfile-style packages with symlinks.
It works like a minimal GNU Stow clone and adds selective directory folding, so you
can choose when a directory should be linked as a whole instead of linking each file
inside it.

## What it does

- Treats each first-level directory in the current working directory as a package.
- Installs package contents into a target directory by creating relative symlinks.
- Removes symlinks for selected packages with `--delete`.
- Supports repeatable `--fold` rules for linking specific directories as one symlink.
- Supports config loading from `.ministowrc` or `--config <FILE>`.
- Supports dry-run planning and optional conflict-skipping during dry runs.

## Current behavior

- Default target: the parent directory of the current working directory.
- Default config file: `.ministowrc` in the current working directory.
- CLI values override config values.
- If any `--fold` options are provided on the CLI, they replace config fold rules.
- Boolean flags from CLI or config are treated as enabled when either sets them.
- The target directory must already exist.
- Package names with trailing `/` or `\\` are accepted.
- Built for Unix-style symlink support; tested with Linux/macOS-style behavior.

## Example layout

```text
dotfiles/
  base/
    .bashrc.init
    .bashrc.env
    bin/
      foreach
  wezterm/
    .config/
      wezterm/
        wezterm.lua
```

From inside `dotfiles/`, `base` and `wezterm` are package names.

## Usage

```bash
ministow [OPTIONS] <PACKAGE>...
```

## Install

Install the binary to Cargo's bin directory (`~/.cargo/bin` by default):

```bash
cargo install --path .
```

### Common commands

```bash
# Stow into the parent of the current directory
ministow base

# Stow into an explicit target
ministow --target "$HOME" base

# Fold a directory into one symlink
ministow --target "$HOME" --fold=wezterm/.config/wezterm wezterm

# Delete links for a package
ministow --target "$HOME" --delete base

# Preview actions without changing the filesystem
ministow --dry-run --verbose=2 base

# Preview while ignoring install conflicts
ministow --dry-run --ignore-conflicts --verbose=2 base
```

## Options

- `-T, --target <DIR>`: target directory.
- `-v, --verbose <LEVEL>`: verbosity `0`, `1`, or `2`.
- `-F, --fold <DIR>`: fold a package subdirectory; repeatable.
- `-C, --config <FILE>`: read options from a config file.
- `-N, --dry-run`: print planned actions without modifying the filesystem.
- `-I, --ignore-conflicts`: skip install target conflict validation during dry-run planning.
- `-D, --delete`: remove symlinks for selected packages.
- `-h, --help`: print help.

## Folding

Fold paths include the package name and must point to an existing directory.

Example:

```bash
ministow --fold=wezterm/.config/wezterm wezterm
```

That creates a symlink at the target like:

```text
$HOME/.config/wezterm -> <repo>/wezterm/.config/wezterm
```

instead of linking files inside `wezterm/.config/wezterm/` one by one.

## Config file

The config file uses one CLI-style option per line. Blank lines and `#` comments are ignored.

Example `.ministowrc`:

```text
--target=$HOME
--verbose=1
--fold=wezterm/.config/wezterm
```

Path values support `~`, `$VAR`, and `${VAR}` expansion.

## Logging

- `0`: errors only.
- `1`: package-level actions and a summary.
- `2`: per-path operations such as `mkdir`, `link`, `unlink`, and `rmdir`.

When a path is already in the desired state, verbose output stays quiet for that path.

## Safety

- Fails on conflicting existing files, directories, or mismatched symlinks.
- Avoids partial install changes by validating the plan before applying it.
- Deletes only symlinks that resolve into the selected package roots.
- Cleans up empty directories left behind after delete operations.
- Re-running install or delete is intended to be idempotent.

## Build and test

```bash
cargo build
cargo test
```

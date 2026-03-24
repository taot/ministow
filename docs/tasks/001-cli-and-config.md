# Task 001: CLI and Config Surface

Status: `[x]` completed

## Goal

Implement the command-line interface and configuration loading model described in the PRD.

## Scope

- Parse `ministow [OPTIONS] <PACKAGE>...`
- Support install as the default action
- Support delete via `-D` and `--delete`
- Support `--target`, `--verbose`, `--fold`, `--config`, `--dry-run`, and help flags
- Load `.ministowrc` from the working directory by default
- Allow `--config` to override the default config location
- Merge config and CLI values with CLI precedence
- Expand `~`, `$VAR`, and `${VAR}` in config path values

## Requirements

- `--target` defaults to the parent of the current working directory when not set anywhere
- `--verbose` accepts only `0`, `1`, or `2`
- `--fold` is repeatable and preserves all configured entries after merge
- Single-value options from CLI override config values
- Repeatable options from CLI override config-provided values for the same option
- Missing package arguments should produce a clear CLI error
- Help output must exit successfully

## Implementation Notes

- Keep parsed inputs in a single normalized options struct used by later planning stages
- Separate raw parsing from config merge so later tests can cover precedence without invoking the full binary
- Make path expansion explicit and testable instead of scattering it across filesystem code

## Acceptance Criteria

- Running with only package names produces a valid normalized options struct
- Running with `.ministowrc` applies config defaults automatically
- Running with both config and CLI options uses CLI values as the effective configuration
- Running `ministow --help` and `ministow -h` prints usage and exits successfully

## Suggested Tests

- Unit tests for CLI parsing
- Unit tests for config file parsing
- Unit tests for precedence and path expansion
- Snapshot or string-assert tests for help output

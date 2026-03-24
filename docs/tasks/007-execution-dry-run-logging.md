# Task 007: Execution, Dry-Run, and Logging

Status: `[ ]` not started

## Goal

Apply planned operations safely, support dry-run mode, and emit logs that match the configured verbosity.

## Scope

- Execute the full plan only after validation and conflict checks succeed
- Support dry-run for install and delete modes
- Implement verbosity levels `0`, `1`, and `2`
- Report package-level summaries and per-path actions as required

## Requirements

- `--dry-run` must prevent filesystem mutation entirely
- Verbosity `0` shows errors only
- Verbosity `1` shows package-level actions and summary counts
- Verbosity `2` shows each planned or applied `mkdir`, `link`, `unlink`, and `rmdir`
- Dry-run with verbosity `2` shows planned operations without applying them

## Implementation Notes

- Keep execution and logging separate so dry-run can reuse the same plan renderer
- Track summary counts from the operation plan rather than reconstructing them after apply
- Preserve deterministic operation order for predictable output

## Acceptance Criteria

- `ministow --dry-run --verbose=2 base` prints all planned actions and makes no changes
- `ministow -N --delete --verbose=2 base` prints planned delete actions and makes no changes
- Verbosity `1` emits package-level progress and summary counts for install and delete

## Suggested Tests

- Integration tests for dry-run install and dry-run delete
- Output assertions for verbosity levels `1` and `2`
- Tests confirming no filesystem mutation happens in dry-run mode

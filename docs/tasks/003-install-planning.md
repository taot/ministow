# Task 003: Install Planning

Status: `[ ]` not started

## Goal

Plan install-mode filesystem operations for package contents that should be linked into the target.

## Scope

- Traverse package contents for install mode
- Recreate needed parent directories in the target
- Plan individual file symlinks when no fold rule matches
- Preserve relative layout from package root to target
- Prefer relative symlink targets where practical
- Produce a complete plan before any mutation is applied

## Requirements

- Files under a package map to equivalent relative paths under the target
- Parent directories are planned as `mkdir` operations only when needed
- Existing compatible parent directories are treated as reusable no-ops
- Already-correct symlinks are treated as no-ops
- The planner must not apply partial changes while still discovering more work

## Implementation Notes

- Emit structured operations such as `mkdir` and `link`
- Sort or otherwise stabilize traversal so logs and tests stay deterministic
- Compute relative symlink destinations in one place to avoid inconsistent link generation

## Acceptance Criteria

- `ministow base` plans links for each file under `base/`
- Nested files such as `base/bin/foreach` produce parent directory creation and file link operations under the target
- Re-running the same install command produces either an empty plan or no-op-compatible operations

## Suggested Tests

- Unit tests for operation planning from simple package trees
- Integration tests for nested directory creation and relative symlink creation
- Idempotency tests for repeated install runs

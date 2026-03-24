# Task 005: Delete Planning and Cleanup

## Goal

Implement delete-mode planning that removes only symlinks owned by the selected package and cleans up empty directories safely.

## Scope

- Support delete mode through `-D` and `--delete`
- Identify removable symlinks that point into the selected package
- Remove folded directory symlinks created for a package
- Clean up empty directories that were only needed for removed links
- Leave unrelated files, symlinks, and directories untouched

## Requirements

- Only symlinks pointing into the selected package may be removed
- Existing non-symlink files must not be removed in delete mode
- Existing symlinks pointing elsewhere must not be removed
- Directory cleanup must stop when a directory is non-empty or not solely removable
- Re-running delete after prior removal must still succeed

## Implementation Notes

- Plan delete operations from the package tree and fold rules, not by blindly walking the target
- Resolve symlink destinations carefully before deciding ownership
- Emit explicit `unlink` and `rmdir` operations so dry-run and verbose logging stay consistent

## Acceptance Criteria

- `ministow --delete base` removes symlinks that belong to `base`
- `ministow --delete` does not remove unrelated symlinks or regular files at matching paths
- Deleting a previously folded directory removes the folded symlink
- Empty parent directories created only for package links are removed when safe

## Suggested Tests

- Integration tests for deleting standard file links
- Integration tests for deleting folded directory links
- Integration tests for safe cleanup with mixed ownership directories
- Idempotent delete re-run tests

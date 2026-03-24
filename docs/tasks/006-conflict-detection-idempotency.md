# Task 006: Conflict Detection and Idempotency

## Goal

Enforce safety rules so `ministow` avoids destructive behavior and remains idempotent across install and delete operations.

## Scope

- Detect incompatible existing files, directories, and symlinks during planning
- Refuse to apply any partial changes when a conflict exists
- Treat compatible existing state as success without mutation
- Guarantee repeatable results for repeated install and delete commands

## Requirements

- Existing regular file where a symlink is needed is a hard error
- Existing directory where a file symlink is needed is a hard error
- Existing symlink pointing elsewhere is a hard error
- Existing matching symlink is a no-op success
- Existing compatible parent directory is a no-op success
- Conflict detection must happen before apply starts

## Implementation Notes

- Make conflict detection operate on planned operations and current filesystem state
- Return actionable error messages with the target path included
- Keep no-op detection explicit so logs can explain why repeated runs succeed

## Acceptance Criteria

- A conflicting target file causes the command to fail without making partial changes
- An already-correct target symlink does not cause failure or duplication
- Re-running install after success leaves the filesystem unchanged and still exits successfully
- Re-running delete after success also exits successfully

## Suggested Tests

- Integration tests for each conflict case from the PRD
- Integration tests proving no partial changes are applied on conflict
- Repeat-run tests for install and delete idempotency

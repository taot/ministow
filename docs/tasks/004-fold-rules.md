# Task 004: Selective Fold Rules

Status: `[ ]` not started

## Goal

Implement exact-match directory folding so selected package subdirectories are linked as whole directories.

## Scope

- Match fold rules on package-relative directory paths including the package name
- Stop traversal when a directory matches a fold rule
- Plan a directory symlink at the target instead of linking descendants individually
- Support multiple fold directives across one or more packages

## Requirements

- A matched fold rule creates a symlink for the directory itself
- Descendants of a folded directory must not appear as separate link operations
- Fold matching must be exact, not prefix-based beyond the exact directory hit
- Multiple fold rules can be applied in one invocation
- Fold behavior must work in both install planning and delete planning

## Implementation Notes

- Represent fold rules in a way that allows fast exact lookup during traversal
- Ensure logs clearly distinguish a folded directory link from individual file links
- Reuse the same fold matcher for validation, planning, and delete ownership checks

## Acceptance Criteria

- `ministow --fold=wezterm/.config/wezterm wezterm` plans a single symlink for `$TARGET/.config/wezterm`
- No child file links are planned inside the folded directory
- Multiple fold directives across `wezterm` and `fcitx` are honored in the same run

## Suggested Tests

- Unit tests for fold rule matching
- Integration tests confirming traversal stops at the folded directory
- Integration tests for multiple fold directives in one invocation

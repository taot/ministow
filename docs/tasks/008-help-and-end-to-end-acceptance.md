# Task 008: Help Output and End-to-End Acceptance

## Goal

Finish the v1 surface with user-facing help text and integration coverage for the PRD acceptance criteria.

## Scope

- Finalize CLI help text
- Add end-to-end tests covering the primary install, delete, fold, config, dry-run, and conflict scenarios
- Verify behavior on Linux and MacOS-compatible path handling assumptions
- Ensure the code structure still supports future restow/refold work

## Requirements

- Help text must describe usage, arguments, options, defaults, and action flags clearly
- Acceptance coverage should include install, explicit target, delete, short delete flag, fold delete, dry-run, multiple folds, config loading, CLI override, idempotency, and conflict detection
- Test fixtures should cover UTF-8 paths where supported by standard filesystem behavior

## Implementation Notes

- Favor end-to-end tests that exercise the full two-phase plan/apply flow through the binary or top-level entrypoint
- Keep fixture layout close to the examples used in `docs/PRD.md` so failures are easy to reason about
- Note unresolved PRD open questions explicitly if code chooses one interpretation

## Acceptance Criteria

- `ministow --help` and `ministow -h` are documented and tested
- The major acceptance criteria from `docs/PRD.md` have automated coverage
- The final task list supports shipping the v1 milestone without depending on future features

## Suggested Tests

- End-to-end tests for each acceptance criterion in the PRD
- Cross-platform-safe path handling tests where possible in CI
- Regression tests for any bug fixes discovered while implementing earlier tasks

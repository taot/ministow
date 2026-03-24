# Task Breakdown

These tasks translate `docs/PRD.md` into an implementation plan for v1 of `ministow`.

- `001-cli-and-config.md`: define command surface, defaults, and config precedence
- `002-package-discovery-and-validation.md`: package lookup and fold path validation
- `003-install-planning.md`: install-mode traversal and symlink planning
- `004-fold-rules.md`: selective directory folding behavior
- `005-delete-planning-and-cleanup.md`: delete-mode ownership checks and cleanup
- `006-conflict-detection-idempotency.md`: safety rules and repeatable behavior
- `007-execution-dry-run-logging.md`: plan execution, dry-run, and verbosity
- `008-help-and-end-to-end-acceptance.md`: help text and integration coverage

Recommended delivery order:

1. `001-cli-and-config.md`
2. `002-package-discovery-and-validation.md`
3. `003-install-planning.md`
4. `004-fold-rules.md`
5. `005-delete-planning-and-cleanup.md`
6. `006-conflict-detection-idempotency.md`
7. `007-execution-dry-run-logging.md`
8. `008-help-and-end-to-end-acceptance.md`

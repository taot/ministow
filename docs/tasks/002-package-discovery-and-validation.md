# Task 002: Package Discovery and Validation

Status: `[x]` completed

## Goal

Validate requested packages and fold directives before any filesystem changes are planned.

## Scope

- Treat each first-level subdirectory in the working directory as a package
- Resolve requested package names against the working directory
- Validate fold paths as package-relative paths including the package name
- Fail early when packages or fold paths are invalid
- Validate target directory expectations before planning begins

## Requirements

- Each package argument must map to an existing first-level directory
- Fold matches are exact on directory path
- A fold rule must point to an existing directory inside the named package
- Invalid package names must report errors like `package 'base' does not exist`
- Invalid fold rules must report errors like `fold path 'wezterm/.config/wezterm' does not exist in package`
- If the target directory is required to exist in v1, fail clearly when it does not exist

## Implementation Notes

- Build a package resolver that works from the current working directory only
- Normalize fold rules once so install and delete planning can share the same matcher
- Keep validation separate from plan construction to preserve the two-phase execution model

## Acceptance Criteria

- Valid packages and fold rules pass validation without touching the filesystem
- Invalid packages fail before any plan is produced
- Invalid fold rules fail before any plan is produced
- Multiple package arguments are validated together

## Suggested Tests

- Unit tests for package lookup
- Unit tests for fold path normalization and exact matching
- Integration tests for missing package and missing fold directory errors

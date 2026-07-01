---
name: conventional-commits-authoring
description: Use this skill when writing commit messages under Conventional Commits policy, including WHY/HOW commit bodies and semantic commit granularity.
---

# Conventional Commits Authoring

## Goal
Write consistent, review-friendly commit messages that follow repository policy.

## Inputs
- Change summary (what changed)
- Affected files/modules
- Validation results (if relevant to the commit)

## Outputs
- Conventional Commit header: `type(scope?): description`
- Commit body with one-line WHY and per-file HOW bullets
- Correct commit granularity (one semantic change per commit)

## Terminology
- type: `feat` / `fix` / `docs` / `style` / `refactor` / `test` / `chore`
- scope: optional module, package, or directory (for example `cli`, `docs`, `pkg/req`)
- description: concise summary in present tense
- WHY: reason for the change (single sentence)
- HOW: per-file bullet list of concrete modifications

## Workflow
1. Determine semantic unit
   - Split unrelated changes into separate commits.
   - Keep generated files separate when practical.
2. Choose `type` and optional `scope`
   - Select the smallest accurate category.
   - Add scope when it improves clarity.
3. Write commit header
   - Format exactly as `type(scope?): description`.
   - Keep description concise and specific.
4. Write commit body
   - First line: one sentence for WHY.
   - Then HOW bullets per file/path.
5. Final commit-quality check
   - Verify header, WHY, and HOW are internally consistent.
   - Ensure scope and changed files match.

## Safety Rules
- Do not mix unrelated semantic changes in one commit.
- Do not omit WHY in the commit body when policy requires it.
- Do not use vague descriptions like "update" or "fix stuff".
- Do not claim tests passed unless they were actually run.

## Done Criteria
- Header matches `type(scope?): description`.
- Body starts with a one-sentence WHY and includes per-file HOW bullets.
- Commit contains one semantic change.

## Templates

### Commit

```text
type(scope?): description

<WHY: one sentence>

- path/to/file1: <what changed>
- path/to/file2: <what changed>
```

## Quick Checklist
- [ ] Correct type/scope/description
- [ ] WHY sentence written (first body line)
- [ ] HOW bullets written per file
- [ ] One semantic change per commit

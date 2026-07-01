---
name: docs-maintenance-implementation-sync
description: Use this skill to maintain existing documentation by syncing it with implemented and tested behavior, while keeping README lightweight and detailed references in docs/. This skill is implementation-driven (code/tests are the SoT).
---

# Documentation Maintenance & Implementation Sync

## Goal
Keep documentation easy to navigate and accurate against the current implementation.

## Terminology
- SoT (Source of Truth): the primary authority used to judge correctness; in this skill, code plus passing tests.
- drift: mismatch between docs and their SoT.

## Classification Rule
- User guide: explains practical usage (how to operate/use the tool in real workflows).
- Specification reference: defines interface/behavior (contract, schema, flags, processing rules, constraints).
- If a section includes both, split it and keep each part in the appropriate document.

## When Not To Use
- Do not use this skill for specification-first authoring (for example requirement definition, design proposals, ADR drafting, or RFC creation before implementation).
- Do not use this skill when an approved specification is the SoT and code must be updated to match it.
- In those cases, use a spec-driven documentation process (spec as SoT) and treat implementation updates as follow-up work.

## Inputs
- Change scope (feature/fix/refactor)
- Affected code paths/files
- Existing docs (`README.md`, `docs/*.md`)
- Validation commands (`vorbere run check`, `vorbere run test`, `vorbere run build`)

## Outputs
- Concise top-level README focused on onboarding
- Detailed, version-maintainable docs under `docs/`
- Updated links between README and detailed docs
- Verified consistency between implementation and documentation

## Workflow
1. Decide document boundary
   - Keep `README.md` focused on overview, install, and one quick-start command.
   - Move requirement/spec-heavy details to `docs/`.
   - Prefer stable file names in `docs/` (for example `manifest.md` instead of version in filename).
2. Map implementation to docs
   - List changed user-facing behaviors (flags, commands, schema fields, processing order, side effects).
   - Map each behavior to a target doc section.
   - Add missing sections before polishing wording.
3. Update docs with minimal duplication
   - Keep a single source of truth for each topic.
   - In README, keep short pointers/links instead of long repeated explanations.
   - Ensure command examples and option names exactly match current CLI.
4. Drift check (implementation vs docs)
   - Compare docs statements against current SoT (code paths + tests).
   - Verify schema docs against actual struct fields and defaults.
   - Verify behavior docs against real processing order and edge-case handling.
   - Search for stale filenames/old terms and update all references.
5. Validate examples and references
   - Confirm linked files exist and internal links resolve.
   - Ensure sample commands are executable in current CLI shape.
   - Run project validation commands when code behavior changed.

## Safety Rules
- Do not invent behavior not present in implementation.
- Do not leave duplicate conflicting explanations in multiple docs.
- Do not keep version-specific filenames unless historical snapshots are intentionally required.
- Keep edits scoped; avoid broad content rewrites unrelated to the change.
- This skill is implementation-driven: code/tested behavior is treated as SoT.
- Document specs in `docs/`, but only for behavior that exists and is verified by code/tests.

## Done Criteria
- README is short and onboarding-first.
- Detailed behavior/spec lives in `docs/` with clear links.
- No stale links or references to removed filenames.
- Documented fields/flags/flows match implementation and tests.
- `vorbere run check && vorbere run test && vorbere run build` succeeds when behavior changed.

## Quick Checklist
- [ ] README kept minimal
- [ ] Detailed docs updated in `docs/`
- [ ] Links updated and validated
- [ ] Implementation-doc drift check completed
- [ ] Validation commands run when needed

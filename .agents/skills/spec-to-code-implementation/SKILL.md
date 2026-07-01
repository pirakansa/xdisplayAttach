---
name: spec-to-code-implementation
description: Use this skill when implementing code from an approved specification. Translates requirements and acceptance criteria into minimal, validated code changes while preserving compatibility constraints.
---

# Spec-to-Code Implementation

## Goal
Implement approved specifications accurately, with clear traceability from spec requirements to code changes and tests.

## Terminology
- SoT (Source of Truth): the primary authority used to judge correctness; in this skill, the approved specification plus acceptance criteria.
- approved: explicitly reviewed and accepted specification revision that is implementation-ready.
- drift: mismatch between implementation/tests and the approved specification.

## Inputs
- Approved specification document(s)
- Acceptance criteria and constraints
- Target codebase and existing tests
- Validation commands (`vorbere run check`, `vorbere run test`, `vorbere run build`)

## Outputs
- Minimal code changes implementing required behavior
- Tests covering acceptance criteria and critical edge cases
- Updated docs only where implementation details changed
- Clear mapping from requirement -> code -> test

## Workflow
1. Extract implementable requirements
   - Convert spec items into concrete engineering tasks.
   - Identify impacted modules, interfaces, and migration constraints.
   - Confirm the approved revision is the active SoT before coding.
2. Plan change boundaries
   - Define smallest safe change-set per requirement.
   - Keep refactors separate unless required for delivery.
3. Implement incrementally
   - Add/modify code in dependency-aware order.
   - Preserve existing public behavior unless spec requires changes.
4. Validate against acceptance criteria
   - Add or update tests for each implemented requirement.
   - Confirm edge/failure paths defined by the spec.
5. Run full quality gates and report traceability
   - Run check, tests, and build.
   - Summarize requirement-to-code/test mapping for review.
   - Report any detected drift against the approved SoT.

## Safety Rules
- Do not implement beyond approved scope.
- Do not reinterpret ambiguous requirements silently; raise explicit clarifications.
- Do not skip tests for behavior-changing requirements.
- Do not bundle unrelated cleanups with spec delivery.
- If implementation reveals spec gaps/changes, update the spec and re-approve before proceeding to avoid drift.

## When Not To Use
- Do not use this skill before the specification is stable/approved.
- Do not use this skill for exploratory prototyping without acceptance criteria.
- In those cases, use `spec-driven-doc-authoring` first.

## Done Criteria
- All in-scope requirements are implemented.
- Acceptance criteria are covered by tests.
- Compatibility/constraint requirements are respected.
- `vorbere run check && vorbere run test && vorbere run build` succeeds.
- Reviewers can trace each behavior to spec and tests.
- No unresolved drift remains between implementation and the approved specification.

## Quick Checklist
- [ ] Requirements decomposed into tasks
- [ ] Minimal scoped implementation completed
- [ ] Tests aligned with acceptance criteria
- [ ] Quality gates passed
- [ ] Traceability summary prepared

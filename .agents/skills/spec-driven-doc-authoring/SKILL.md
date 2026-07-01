---
name: spec-driven-doc-authoring
description: Use this skill when requirements/design documentation is the SoT and implementation will follow the spec. Suitable for RFCs, ADRs, requirement definitions, and design-first delivery.
---

# Spec-Driven Documentation Authoring

## Goal
Author and evolve specification documents so they can drive implementation with clear scope, constraints, and acceptance criteria.

## Terminology
- SoT (Source of Truth): the primary authority used to judge correctness; in this skill, the specification document.
- approved: explicitly reviewed and accepted baseline that implementation teams can execute.
- drift: mismatch between approved specification and implementation/docs.

## Inputs
- Product/problem statement
- Constraints (timeline, compatibility, security, operations)
- Existing architecture and interfaces
- Stakeholder decisions and open questions

## Outputs
- Clear specification in `docs/` (requirements, design, acceptance criteria)
- Approved specification baseline for implementation handoff
- Explicit assumptions, non-goals, and risks
- Implementation-ready behavior definitions and edge cases
- Traceable decisions for future maintenance

## Workflow
1. Frame the problem and scope
   - Define objective, target users, and success criteria.
   - Separate in-scope and out-of-scope items.
2. Define requirements precisely
   - Capture functional and non-functional requirements.
   - Include constraints (performance, security, compatibility, rollout).
3. Design the behavior and interfaces
   - Specify inputs/outputs, data schema, processing flow, and error handling.
   - Document ordering/priority rules and conflict resolution.
4. Add acceptance criteria and test scenarios
   - Write testable Given/When/Then-style criteria.
   - Include normal paths, edge cases, and failure scenarios.
5. Plan implementation handoff
   - Break work into deliverable steps.
   - Map each requirement to code areas likely to change.
   - Define approval criteria and freeze the approved baseline revision.

## Safety Rules
- Do not describe ambiguous behavior without constraints.
- Do not mix unrelated feature ideas into one spec.
- Do not claim implementation is complete from spec text alone.
- Keep assumptions and unresolved questions explicit.
- Mark draft vs approved status explicitly to avoid SoT confusion.

## When Not To Use
- Do not use this skill for post-implementation doc synchronization.
- Do not use this skill when code/tests are already the SoT.
- In those cases, use `docs-maintenance-implementation-sync`.
- User-facing docs (README/usage) should not be updated as if implemented; update them after implementation or clearly mark as planned.

## Done Criteria
- Scope and non-goals are explicit.
- Requirements are testable and unambiguous.
- Data/flow/error behavior is fully specified.
- Acceptance criteria can directly drive implementation and tests.
- Open questions and decisions are documented.
- Approved baseline is clearly identifiable to prevent drift during implementation.

## Quick Checklist
- [ ] Problem/scope clarified
- [ ] Requirements and constraints defined
- [ ] Behavior/schema/flow specified
- [ ] Acceptance criteria added
- [ ] Handoff steps documented

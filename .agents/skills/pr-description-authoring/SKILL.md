---
name: pr-description-authoring
description: Use this skill when writing pull request descriptions in repository-required format, including Motivation/Design/Tests/Risks sections.
---

# PR Description Authoring

## Goal
Write clear, review-ready PR descriptions that satisfy repository policy and help reviewers validate intent, implementation, and risk.

## Inputs
- Change summary and scope
- Design decisions and trade-offs
- Test commands/results actually executed
- Known risks, limitations, and follow-up items

## Outputs
- PR description
- Required sections: Motivation / Design / Tests / Risks
- Accurate test reporting (only what was run)
- Concise risk statements and mitigation notes
- Optional follow-up list when work is intentionally deferred

## Terminology
- Motivation: why this change is needed
- Design: how the change is implemented
- Tests: what was executed and results
- Risks: possible side effects and rollback/mitigation notes
- Follow-ups: intentionally deferred items to be handled later (optional)

## Workflow
1. Collect scope and intent
   - Summarize user/problem context and expected impact.
   - Exclude unrelated work from the PR narrative.
2. Draft `Motivation`
   - Explain the problem and why this change is the right timing.
3. Draft `Design`
   - Summarize approach, key files, and notable decisions.
   - Mention alternatives only if they affected implementation choices.
4. Draft `Tests`
   - List executed commands and outcomes.
   - If tests were not run, state that explicitly and why.
5. Draft `Risks`
   - Describe potential regressions and operational impact.
   - Add mitigation, monitoring, or rollback notes when relevant.
6. Draft optional `Follow-ups`
   - List deferred items with brief rationale and next steps/owners if known.

## Safety Rules
- Do not claim tests passed unless they were actually run.
- Do not omit known risks for behavior-changing work.
- Do not include unrelated implementation details.
- Do not leave placeholders (e.g. "<fill>") in the final PR text.

## Done Criteria
- All required sections are present.
- Content matches actual code changes and test execution.
- Risks are explicit and actionable.
- Reviewer can understand intent and verify outcomes quickly.
- No placeholders remain in the final PR description.

## Template (Draft only; must be fully filled before submission)

```markdown
### Motivation
<fill>

### Design
<fill>

### Tests
- <fill: executed command + result>

### Risks
<fill>

### Follow-ups (optional)
- <fill: deferred item and next step>
```

## Quick Checklist
- [ ] Motivation written clearly
- [ ] Design explains key decisions
- [ ] Tests reflect actual execution
- [ ] Risks and mitigations documented
- [ ] No placeholders remain

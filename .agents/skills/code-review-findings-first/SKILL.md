---
name: code-review-findings-first
description: Use this skill when asked to review code, PRs, or diffs. Prioritize actionable findings (bugs, regressions, risks, and missing tests) ordered by severity with file/line references before summaries.
---

# Code Review Findings First

## Goal
Deliver review feedback that helps the author fix high-impact issues quickly.

## Inputs
- Diff, changed files, or target commit/PR
- Project policies and test/lint requirements
- Runtime/compatibility constraints when provided

## Outputs
- Findings listed first, ordered by severity
- Each finding includes: impact, evidence, and concrete fix direction
- File references with line numbers where possible
- Clear note when no findings are detected
- Residual risks and testing gaps

## Severity Model
Use these labels consistently:
- Critical: Security/data loss/outage risk or clear functional break
- High: Likely regression or major behavior mismatch
- Medium: Maintainability/performance/correctness risk with limited blast radius
- Low: Minor robustness/readability issue

## Workflow
1. Scope the review
   - Identify exactly what changed and what behavior is affected.
   - Ignore unrelated files unless they create direct risk.
2. Check correctness first
   - Validate logic, edge cases, error handling, and state transitions.
   - Verify compatibility with current contracts and flags.
3. Check operational risk
   - Look for security, data integrity, concurrency, and reliability issues.
   - Call out unsafe defaults and unbounded operations.
4. Check test coverage
   - Confirm tests cover new behavior and failure paths.
   - Flag missing or weak assertions.
5. Report findings first
   - Start with the highest-severity issue.
   - For each issue: what is wrong, why it matters, where it is, how to fix.
6. Add brief wrap-up
   - If no issues: state "No findings" explicitly.
   - List residual risks and what was not validated.

## Safety Rules
- Do not invent behavior; tie every finding to concrete code evidence.
- Do not bury critical issues inside summary text.
- Do not claim tests were run unless they were actually executed.
- Do not treat style nits as primary findings when correctness issues exist.

## Done Criteria
- Findings are first and severity-ordered.
- Each finding is actionable and evidence-based.
- File references are included.
- Testing gaps or unverified assumptions are explicit.
- Summary is short and secondary.

## Output Template
```markdown
## Findings
1. [Critical|High|Medium|Low] <short title>
- Impact: <user/system impact>
- Evidence: <what in code shows this>
- Fix: <recommended change>
- Reference: <path:line>

## Open Questions / Assumptions
- <optional>

## Summary
- <brief change understanding>
- <residual risk / test gap>
```

## Quick Checklist
- [ ] Focused on changed scope and behavior
- [ ] Findings are severity-ordered
- [ ] Every finding includes evidence and fix direction
- [ ] File references are present
- [ ] Missing tests and residual risks are documented

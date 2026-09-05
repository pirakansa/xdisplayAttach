---
name: code-design-review
description: Use this skill when asked to review code responsibilities, module or class boundaries, dependencies, maintainability, readability, testability, or refactoring opportunities.
---

# Code Design Review

## Goal

Review whether the code is structured so that its responsibilities, dependencies, state, and side effects can be understood, tested, and changed safely.

Focus on design and maintainability. Use `code-review-findings-first` separately for exhaustive bug, regression, security, and operational-risk review.

## Review Scope

Evaluate:

* Responsibility allocation between functions, classes, and modules.
* Cohesion and coupling.
* Module and architectural boundaries.
* Dependency direction.
* State and side-effect ownership.
* Placement and duplication of business rules.
* API and abstraction quality.
* Readability that affects safe modification.
* Testability.
* Both under-engineering and over-engineering.

Inspect relevant callers, dependencies, tests, and surrounding code when the reviewed diff alone is insufficient.

## Review Principles

### Evidence over preference

Report a design finding only when there is:

* Concrete code evidence.
* A practical maintenance, correctness, readability, or testing risk.
* An actionable improvement direction.

Do not present personal style preferences or hypothetical future needs as findings.

### Responsibilities

For each important unit, determine:

* Its primary responsibility.
* The data or state it owns.
* The decisions it makes.
* The side effects it performs.
* Its reasons to change.

Consider separation when unrelated responsibilities have different:

* Reasons to change.
* Dependencies.
* Failure modes.
* Lifecycles.
* Testing requirements.

Do not recommend splitting code based only on file size, method count, or line count.

### Boundaries and dependencies

Check whether:

* Domain rules are separated from transport, UI, persistence, and infrastructure details where this provides a meaningful boundary.
* Dependencies follow the intended architectural direction.
* Public interfaces expose only what callers need.
* Callers are not required to know internal sequencing or implementation details.
* Circular, bidirectional, or excessively broad dependencies are avoided.

Do not require interfaces or dependency injection without a concrete substitution, ownership, or testability benefit.

### State and side effects

Check whether:

* Mutable state has one clear owner.
* State transitions are explicit.
* Invalid intermediate states are prevented where practical.
* Side effects occur in predictable locations.
* Methods do not hide unexpected mutation.
* Correct usage does not depend on undocumented call ordering.
* There is a clear source of truth for duplicated state.

### Readability and testability

Flag readability issues only when they make behavior difficult to verify or modify safely.

Look for:

* Mixed abstraction levels.
* Misleading names.
* Deep or indirect control flow.
* Hidden state changes.
* Boolean parameters with unclear meaning.
* Large groups of unrelated parameters.
* Excessive fragmentation across files or methods.
* Tests requiring unrelated infrastructure or extensive setup.

Explain the practical consequence rather than merely naming a principle.

### Abstraction and duplication

Check whether abstractions:

* Hide meaningful complexity.
* Represent policy, ownership, lifecycle, or an external-system boundary.
* Simplify callers.
* Enable meaningful independent testing.

Question abstractions that only forward calls, duplicate another API, or introduce generic machinery for one simple use case.

Flag duplicated logic when it represents the same rule and can realistically diverge. Do not merge code that is only superficially similar but represents different concepts.

## Priority Model

* `P1`: The structure is already likely to cause defects, conflicting state, unsafe changes, or policy bypass.
* `P2`: Ordinary changes or tests are significantly harder or more error-prone because of the structure.
* `P3`: A local design or readability problem with limited impact.
* `Suggestion`: A non-blocking improvement where the current design remains acceptable.

Do not use design priority as a substitute for correctness severity.

## Workflow

1. Identify the reviewed components, entry points, callers, dependencies, and tests.
2. Map the responsibilities, state, decisions, and side effects of important units.
3. Check boundaries, dependency direction, cohesion, coupling, and rule placement.
4. Check readability, testability, duplication, and abstraction quality.
5. Check for both missing structure and unnecessary structure.
6. Report only evidence-based and actionable findings.
7. Recommend the smallest refactoring that establishes a clear responsibility boundary.
8. State assumptions and unreviewed areas.

When recommending a refactor, specify:

* The responsibility currently misplaced or mixed.
* Its proposed owner.
* The interface or behavior that should remain.
* The concrete benefit.
* Relevant tests that should protect the change.

Avoid broad rewrites when an incremental change is sufficient.

## Correctness Issues

Do not perform a full correctness review through this skill.

When an obvious bug, security issue, data-loss risk, or operational failure is discovered:

* Report it briefly in a separate section.
* Do not mix its severity with design priority.
* Recommend validating the change with `code-review-findings-first`.

## Output Template

```markdown
## Design Findings

1. [P1|P2|P3] [<category>] <short title>
   - Current structure: <current ownership or dependency>
   - Concern: <specific design problem>
   - Risk: <practical consequence>
   - Evidence: <concrete code evidence>
   - Refactoring direction: <smallest useful boundary change>
   - Reference: <path:line>

## Improvement Suggestions

- [<category>] <optional improvement>
  - Benefit: <benefit>
  - Trade-off: <cost, when relevant>
  - Reference: <path:line>

## Correctness Issues Noticed

- <brief issue for findings-first validation>

## Open Questions / Assumptions

- <missing requirement or architectural intent>

## Validation

- Tests executed: <commands and results, or “Not executed”>
- Not validated: <relevant limitations>

## Summary

- <main design pressure>
- <highest-priority refactoring direction>
```

Omit empty optional sections.

If there are no actionable design issues, state **“No design findings.”**

## Safety Rules

* Do not invent architectural requirements.
* Do not use SOLID or design-pattern terminology instead of concrete evidence.
* Do not recommend separation based only on code size.
* Do not assume more abstractions improve the design.
* Do not assume fewer abstractions improve the design.
* Do not report the same root cause repeatedly under different categories.
* Do not recommend repository-wide redesign when a local change is sufficient.
* Do not claim tests or commands were run unless they were actually executed.

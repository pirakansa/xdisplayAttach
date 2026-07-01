# AGENTS.md

This document is the README for AI coding agents. It complements the human-facing README.md so that agents can develop safely and efficiently.

---


## Documentation of Process vs Policy

This repository separates **policy** from **how-to guidance**:

- **AGENTS.md = Policy (MUST/MUST NOT)**  
  Contains the mandatory rules agents must follow (e.g., language requirements, required sections, validation expectations, boundaries).
  Keep it short and stable.

- **SKILLS = Procedure / Templates / Checklists**  
  Contains step-by-step workflows, templates, and checklists used to comply with policy.
  Prefer updating skills when improving writing structure or workflow details.

Rule of thumb:
- If it is a non-negotiable rule for reviews/CI: put it in **AGENTS.md**.
- If it is an example, template, or writing process: put it in a **skill**.


---

## Setup Steps

* Recommended: VS Code Dev Container / GitHub Codespaces (use the `.devcontainer/` image).
* Base packages: `sudo apt-get install build-essential`.
* Rust toolchain: `rustup default stable` (rustfmt/clippy components are required).
* Task runner: `vorbere.yaml` via `vorbere run <task>` (CI uses the tasks described later).

---

## Build & Validate

* Build: `vorbere run build`
* Test: `vorbere run test`
* Check: `vorbere run check`
* Cleanup: `vorbere run clean` or `cargo clean`.
* For CLI usage and command examples, see the Usage section in README.md.

---

## Project Structure

We follow the **package layout described in The Cargo Book** for a project consisting of a single binary plus shared library code.

```
.
├── Cargo.toml
├── Cargo.lock
├── vorbere.yaml
├── src/
│   ├── lib.rs
│   ├── main.rs
│   └── bin/
│       ├── named-executable.rs
│       ├── another-executable.rs
│       └── multi-file-executable/
│           ├── main.rs
│           └── some_module.rs
└── tests/
    ├── some-integration-tests.rs
    └── multi-file-test/
        ├── main.rs
        └── test_module.rs
```

### Roles and Guidelines

* Keep business logic in `lib.rs` or `src/<module>.rs`; limit `main.rs` to startup/DI/argument handling.
* Integration tests go under `tests/`, exercising public APIs.
* Place new files under the directories above; avoid introducing new top-level folders without discussion.

### Agent-Specific Rules

* Place new files according to the directory guidelines above; avoid introducing unnecessary top-level directories.
* When modifying existing functions, add or update unit tests and confirm `vorbere run test` passes.
* When writing files or accessing external resources, use temporary directories so existing test data is not overwritten.


---

## Coding Standards

* Always run `vorbere run check` and ensure all included static checks pass with no warnings (CI requirement).
* Prefer `thiserror` for error types; use `anyhow` only in binaries.
* Naming: modules in `snake_case`, types in `UpperCamelCase`.
* Extract magic numbers/URLs into constants with meaningful names.
* Avoid unrelated large refactors; keep changes minimal in scope.

---

## Testing & Verification

* Unit tests: `vorbere run test`
* For additional file or network operations, use temp directories or `httptest` to avoid external dependencies.
* When command behavior changes, keep usage examples in `README.md` and fixtures under `test` consistent.

### Static Analysis / Lint / Vulnerability Scanning

* Run `vorbere run check` as the default entry point for static analysis, linting, vulnerability scanning, and related verification.
* If needed, use underlying component commands only to investigate or isolate specific failures (for example, `vorbere run vulnerability`).

---

## CI Requirements

GitHub Actions (`.github/workflows/ci.yml`) runs the following:

* `vorbere run check`
* `vorbere run test`
* `vorbere run build`

Confirm `vorbere run check` / `vorbere run test` / `vorbere run build` succeed locally before opening a PR. If they fail, format and validate locally, then rerun.

---

## Security & Data Handling

* Do not commit secrets or confidential information.
* Do not log personal or authentication data in logs or error messages.
* Use fictitious URLs and passwords in test data; avoid hitting real services.
* Obtain user approval before accessing external networks.

---

## Agent Notes

* When instructions conflict, prioritize explicit user prompts and clarify any uncertainties.
* Before and after your work, ensure `vorbere run check`, `vorbere run test`, and `vorbere run build` all succeed; report the cause and fix if any of them fail.

---

## Branch Workflow (GitHub Flow)

This project follows **GitHub Flow** based on `main`.

* **main branch**: Always releasable. Direct commits are forbidden; use pull requests.
* **Feature branches (`feature/<topic>`)**: Branch from `main` for new features or enhancements, then open a PR when done.
* **Hotfix branches (`hotfix/<issue>`)**: Branch from `main` for urgent fixes, merge promptly after CI passes.

### Rules

* Always branch from `main`.

---


## Commit Message Policy

Commit messages MUST follow **Conventional Commits** and MUST be written in **English**.

For structured authoring (template, checklist), use the skill: `conventional-commits-authoring`.

---

## Documentation Policy

- **Language**: All documentation (README.md, docs/, inline doc-comments) MUST be written in **English**.
- **README.md (top level)** is onboarding-first: overview, install, and one quick-start. Keep it short and link to details in `docs/`.
- **docs/** holds detailed documentation and is organized as:
  - **User guides** (practical usage / workflows)
  - **Specification references** (contracts: schema, flags, processing rules)
  - If content mixes both, split it into the appropriate documents.
- **Source of truth**
  - For post-implementation updates, treat **code + passing tests** as SoT and use `docs-maintenance-implementation-sync`.
  - For design-first work where the **spec is SoT**, use the spec-driven skills (`spec-driven-doc-authoring` / `spec-to-code-implementation`).
- **PR hygiene**: Update docs with behavior changes. If no doc updates are needed, explicitly note **"No documentation changes"** in the PR description.
---

## Dependency Management Policy

* Add dependencies with `cargo add <crate>`; do not edit Cargo.toml by hand for adds.
* Use SemVer pins; avoid wildcards unless necessary.
* Update dependencies per-PR with `cargo update -p <crate>`, explaining the target and reason.
* Run `cargo audit` for PRs to ensure no known vulnerabilities.
* Limit **dev-dependencies** to tests/tooling; remove when unused. Keep **build-dependencies** minimal and justify large additions.

---

## Release Process

* Follow **SemVer** for versioning.

---

## PR Template

PR descriptions MUST be written in **English** and MUST include:
- Motivation
- Design
- Tests (only what was actually run)
- Risks

For structured authoring (template, checklist), use the skill: `pr-description-authoring`.

---

## Checklist

* [ ] `vorbere run check`
* [ ] `vorbere run test`
* [ ] `vorbere run build`

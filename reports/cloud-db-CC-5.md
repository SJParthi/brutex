STATUS: IN PROGRESS

# CC-5 — Copies register (cloud build report)

Branch `cloud/db-CC-5-20261024`, cut from `origin/feat/db-fix` at `e851da1ca` (Merge DATA-2).
Spec: `origin/cloud/db-base-20261024:reports/db-specs-20261024/CC-5.md`. Decision block D-7040..D-7049.
Prerequisites: none (spec section 1). FILES_ONLY links: none.

## Log

### 2026-10-24 — start

- GO present, HOLD absent, spec present. Branch cut from `e851da1ca`.
- Gate tools: `cargo-deny`, `cargo-llvm-cov`, `cargo-mutants` are not installed in this container;
  installing from crates.io (GitHub release downloads answer 403 through the proxy).
- NEXT: settings key `extra_copy_roots`, then `pull::copies_register` and its tests.

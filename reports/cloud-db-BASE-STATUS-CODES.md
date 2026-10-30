STATUS: IN PROGRESS

# Cloud build report: BASE-STATUS-CODES (30 Oct 2026)

Part: `BASE-STATUS-CODES` — one status rule through one typed classifier: a client's malformed input is
400, a key that names nothing is one not-found answer on every route, 503 only for what exists but cannot
be read (assurance 29 Oct n=31).

- Branch: `cloud/db-BASE-STATUS-CODES-20261030`, cut from `origin/feat/db-fix` at
  `f01d7a0e430e8f9b01e424a299b0bed20738a0ef` (Merge R4-26), the newest head on 30 Oct 12:43 IST.
- Specification: `origin/cloud/db-base-20261030:reports/db-specs-20261030/BASE-STATUS-CODES.md`, read in
  full with `CLAUDE.md` (from `origin/feat/db-fix`), `README.md` and `CONTEXT.md` of that folder.
- Prerequisites: none (the part file, section 1). FILES_ONLY links: none.
- D1: the merge-back waits for `Merge BASE-REDS-1023:` (not on `feat/db-fix` at `f01d7a0e4`: `grep -c` = 0).
- Decision block: D-7550..D-7559 (D-7550 is this part's entry).
- Host: 4 cores, running as root (uid 0), rustc 1.97.1 (the repository's pin), Node 22.22.0, npm 10.9.4.

## Log

### 2026-10-30 12:43 IST

- Waited on the switches from 10:28 IST with one blocking 9-minute poll at a time (README step 1); the
  test printed `OPEN 12:43:15 IST` (HOLD removed by `b9e9ed39`, GO and GO-BASE-STATUS-CODES present).
- Branch cut from `origin/feat/db-fix` `f01d7a0e4`.
- NEXT: read the code the part owns (`sources.rs` `Refusal`, the seven identity routes' refusal paths,
  `server.rs` `route_table`), then write `status_rule.rs` and `status_rule_tests.rs` test first.

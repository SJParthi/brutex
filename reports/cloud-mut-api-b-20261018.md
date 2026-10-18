STATUS: IN PROGRESS

# Cloud mutation and coverage pass B, finished — `crates/api`, 2026-10-18

Branch `cloud/mut-api-b-20261018`, cut from `origin/cloud/mut-api-b-20261017`
at `6a957d3e` (the 17 Oct pass's last push), with `origin/feat/db-fix` at
`67c4e58d` merged in at `19e2c2f9`.

**This report holds no authority** (`CLAUDE.md` §10): it is evidence of what
was measured on this date, on this machine. Every number below was measured
in this session unless it says it was carried.

## Carried

The 17 Oct report (`reports/cloud-mut-api-b-20261017.md`) is kept as it was.
Its scope, its five fixed defects, its carried 16 Oct findings and its
phase-1 results for `import_journal.rs` and `audit.rs` are carried, not
re-measured, except where phase 2 below re-runs them.

## Merge

`origin/feat/db-fix` moved 16 commits since the 17 Oct pass merged
`dd7868a5` (GF-1, BASE-CITED, AH-02, R3-09 repair 1, R4-06 repair 1b, AG-04,
DENY-1, BASE-GATE10, CLOUD-ATTACKO1, R3-14). It merged with **no textual
conflict**. Of the scoped files it changed only `server.rs` (+460 / −77, the
AH-02 head-deadline and the R3-09 census changes) and `emitted.rs`. After the
merge the branch differs from `feat/db-fix` only in the 17 Oct pass's
fourteen files, and `Cargo.toml`, `Cargo.lock` and `deny.toml` are
byte-identical to it.

## Left (in progress)

- Phase 1: `autopilot.rs`, `pullrun.rs`, `server.rs`, every listed mutant on
  the merged tree.
- Phase 2: every survivor and every mutant in code changed since phase 1's
  base.
- The final coverage run; gates.

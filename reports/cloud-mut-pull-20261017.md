STATUS: IN PROGRESS

# Cloud mutation and coverage pass — `crates/pull`, 2026-10-17

Branch `cloud/mut-pull-20261017`, cut from `origin/feat/db-fix` at `f9cdb584`.
Continues `cloud/mut-pull-20261016` (report
`reports/cloud-mut-pull-20261016.md` on that branch), whose test work was
re-applied with `git diff c9e3472e5 origin/cloud/mut-pull-20261016 -- .
':(exclude)reports' | git apply -3`: every hunk applied cleanly, no conflict,
no `feat/db-fix` line touched.

**Scope:** the same ten files — `cash_session_cache.rs`, `fold.rs`,
`gdfl_archive.rs`, `gdfl_cm.rs`, `gdfl_cm_auction.rs`, `gdfl_import_scope.rs`,
`gdfl_nfo.rs`, `gdfl_tickstore.rs`, `request_minutes.rs`, `scrub.rs`. Of
these, `feat/db-fix` changed the production code of two since `c9e3472`:
`request_minutes.rs` (R3-18: `tally`, `stored_line`, `ledger_key`,
`append_named_picking`, the `stored_short` reason) and `gdfl_archive.rs`
(`dos_time`, `raw_name`). The other eight are byte-identical in production.

**No production line changed on this branch.** Tests only, plus one
gate-1d declaration (`.github/gates_tree.rs`, see Gates).

## Done

- Re-applied the 2026-10-16 tests onto the newest `feat/db-fix`.

## Left

## Gates

## NEEDS THE MAC

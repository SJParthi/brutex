STATUS: HELD

# IA-3b (cloud): ingest preflight refusals IA-3 left

Branch `cloud/db-IA-3b-20261024`, cut from `origin/feat/db-fix` e851da1c (Merge DATA-2).
Spec `reports/db-specs-20261024/IA-3b.md` on `origin/cloud/db-base-20261024` (83b4d4e6, GO present,
HOLD absent at start). HOLD appeared at 5dbfdf3f ("shared 5-hour usage window near its limit").

Prerequisites (both merged on `origin/feat/db-fix`): `Merge IA-3-OPT: ...` and `Merge IA-16: ...`.

## Progress log

### 2026-10-24 09:12 IST

- Read README.md, CONTEXT.md and IA-3b.md in full; branch created from e851da1c.
- NEXT: preflight module skeleton, the new `RunRefusal` variants and the one call in `plan`.

### 2026-10-24 09:48 IST (WIP commit, then HELD)

Done (code, compiles: `cargo build -p pull --locked` finished clean, no warning):

- `crates/pull/src/gdfl_run_preflight.rs` (new): `SourceIdentity` and `Watch` (item 4's one
  identity: len, ino, mtime, ctime, never st_dev), the `Host` seam (`Os`: `statfs`/`statvfs`
  and `getrlimit` through rustix), the checks in order (year zips, the tick-store writer lock,
  the legacy cutover, the store's volume and open-file limit, census room with the v4
  compaction model and the disk floor, source span and `outside_source`, the format digest),
  the plan line and the refusal sentences.
- `gdfl_run.rs`: 10 `RunRefusal` variants appended (codes `year_zip_missing`,
  `source_unreadable`, `year_zip_empty`, `source_busy`, `census_room_short`,
  `census_compaction_no_disk`, `legacy_run_may_be_live`, `store_read_only`, `store_not_local`,
  `open_files_short`), the `#[path]` mod line, the one call in `plan`, `Request.legacy_ack`
  (written on the request line only when set, so every earlier line is unchanged),
  `Plan.checks`, the doc line 41 correction.
- `gdfl_cm.rs` + `gdfl_import_causes.rs`: per-day `CmRefusal::DayZipEmpty`, `MemberTooLarge`,
  `SourceChanging`, each `Cause::open` (exhaustive arms, no `_`).
- `gdfl_archive.rs`: `member_bytes` cap check before any read; `stated_entries` (end record's
  count, O(1)); empty nested day zip; the outer archive's `Watch`; `Archive::span`.
- `gdfl_tickstore.rs`: `rebuild` cap check; `open_watched_day` (empty index refused, watch);
  `DayTable` spans; `TickStore::span`.
- `gdfl_nfo.rs`: year zips watched; empty nested day zip; `NfoZips::span`,
  `NfoTickStore::span`.
- `ingest_failkind.rs` + `gdfl_import_causes.rs`: `FailKind::OpenFilesExhausted` from
  `StoreError::Io { code: Some(EMFILE) }` (the store already keeps the errno).
- `gdfl_import_journal.rs`: `format_digest_unpinned=1` on the run line; `Journal::first_open`.
- `gdfl_import_filing.rs`: the store_not_writable fix sentence shared through one constant.
- cli `gdfl_import_command.rs`: `--legacy-run-not-live`, the claim's `legacy_ack` from it,
  the plan line and the unpinned-digest warning on the stderr writer.
- root `Cargo.toml`: rustix `process` feature (getrlimit).

NOT DONE yet (next, after GO):

- `FILES_PER_WORKER` / `FILES_PER_RUN` hold PLACEHOLDER numbers (8, 64): they must be
  replaced by the measurement test before this part is done. Not a measurement.
- No test written yet; the workspace build with tests was started and not read.
- Bench C-GR-05, docs/04, docs/05, docs/06 rows, gates, NEEDS THE MAC list.

Commands: `cargo build -p pull --locked` -> `Finished`.

NEXT: on GO, read the workspace build output, fix what broke, then write
`gdfl_run_preflight_tests.rs` (the 21 named tests and the extreme cases), measure the
descriptors per worker, then the bench and the ledgers.

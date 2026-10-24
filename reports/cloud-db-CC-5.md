STATUS: HELD

# CC-5 — Copies register (cloud build report)

Branch `cloud/db-CC-5-20261024`, cut from `origin/feat/db-fix` at `e851da1ca` (Merge DATA-2).
Spec: `origin/cloud/db-base-20261024:reports/db-specs-20261024/CC-5.md`. Decision block D-7040..D-7049.
Prerequisites: none (spec section 1). FILES_ONLY links: none.

## Done so far

- `crates/store/src/settings.rs`: key 12 `disk.extra_copy_roots` (`Key::ExtraCopyRoots`, slot 11,
  `KEYS` 11 -> 12), `COPY_ROOT_SEPARATOR` `|`, `MAX_COPY_ROOTS_BYTES` (= the settings route's own
  4,096-byte `secret_shape` ceiling, an existing project number; no count of roots is typed).
  `copy_roots()` is the one reader: absolute, no `..`, no empty piece, none twice, none nested,
  checked with an ordered set (O(r log r)). `check` adds that each folder exists; `check_stored`
  holds a stored value to its shape only. Test
  `the_extra_copy_roots_key_is_checked_and_stored_canonically`: passes.
- `crates/api/src/settings_route.rs`: the two required arms (`default_of`: `(None, UNSET_WAIT)`;
  `reader_of`: `Ok(...)` naming `pull::copies_register`, `GET /disk/copies.json`, `cli disk-copies`).
- `crates/pull/src/copies_register.rs` (new) and `copies_register_tests.rs` (new, 24 tests, all
  pass): the read-only `Fs` seam (`lstat`, `list`, `lines` through `store::safe_open`; no writing
  method exists), `config()`, `reader_of()` (D-7044), a walk that enters every directory once by
  `(st_dev, st_ino)` on `available_parallelism` workers (no typed entry, depth, list or thread
  count: the operator's 24 Oct rule), `(st_dev, st_ino)` hazards within one walk, streamed receipts
  (`ALL_VERIFIED.txt`, `VERIFIED.tsv` matched by inode, size, mtime and ctime, never `st_dev`;
  `*.prove.out` with `ALL-PROVED`), twins, the retirement list and its text, the events, the cache
  (`answer`: re-walk only when the roots or a root's own stat change; no request waits on
  another's walk), `total()` for R4-29.

## Left

- The api route `GET /disk/copies.json` and its wiring, the cli verb `disk-copies` and its wiring,
  the R4-06 no-delete list, the /ingest panel and the web bundle, the bench, the docs ledgers
  (02 §42, 04, 05 D-7040.., 06), every gate.

## Log

### 2026-10-24 — start

- GO present, HOLD absent, spec present. Branch cut from `e851da1ca`.
- Gate tools: `cargo-deny` 0.20.2, `cargo-llvm-cov` 0.9.1, `cargo-mutants` 27.1.0 installed from
  crates.io (GitHub release downloads answer 403 through the proxy).

### 2026-10-24 — save point 1 (HELD)

- Files: `crates/store/src/settings.rs`, `crates/store/src/settings_tests.rs`,
  `crates/api/src/settings_route.rs`, `crates/pull/src/lib.rs`, `crates/pull/src/copies_register.rs`,
  `crates/pull/src/copies_register_tests.rs` (header only).
- `cargo clippy -p pull -p store --locked`: no warning, no error.
- Commit `cca93bc4`, pushed.
- `HOLD` found on `origin/cloud/db-base-20261024` at this save point: stopped here, waiting for
  `GO` with no `HOLD`.
- NEXT: write `crates/pull/src/copies_register_tests.rs` (the eight named tests first, failing
  before the module, then the extreme cases), run `cargo test -p pull -p store --locked`.

### 2026-10-24 — resumed, save points 2 and 3, then HELD again

- 12:37 IST the Mac lifted HOLD ("activate all the sessions"); work resumed from NEXT.
- The README's new section (the operator's words of 24 Oct, "no typed number outside the Winner
  Gate") was applied: removed `MAX_COPY_ROOTS`, the walk's entry and depth caps, the list caps, the
  receipt read cap and the statement cap. Commit `ad16891c`.
- Wrote `crates/pull/src/copies_register_tests.rs`: the eight named tests and 16 extreme cases.
- `cargo test -p pull --lib --locked copies_register`: `test result: ok. 24 passed; 0 failed`.
- `cargo test -p store --lib --locked settings`: `22 passed; 3 failed`. The three
  (`a_directory_that_stops_being_readable_is_a_fault_not_nothing_new`,
  `a_disk_that_refuses_the_write_is_refused_with_nothing_visible`,
  `a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason`) are permission-bit tests;
  this container runs as root, which ignores permission bits. To be shown red on the unmodified
  base in the final gate run (not yet shown).
- `cargo clippy -p pull -p store --all-targets --locked`: no warning.
- `HOLD` back on the base at 12:55 IST ("HOLD for parts without GO-<NAME>"); no `GO-CC-5`. Stopped.
- NEXT: the api route `crates/api/src/copies_route.rs` + `copies_route_tests.rs`, its two
  `server.rs` lines, its rows in `operation_audit_tests.rs` and `pool_table_tests.rs`.

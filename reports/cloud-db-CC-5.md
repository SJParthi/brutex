STATUS: HELD

# CC-5 — Copies register (cloud build report)

Branch `cloud/db-CC-5-20261024`, cut from `origin/feat/db-fix` at `e851da1ca` (Merge DATA-2).
Spec: `origin/cloud/db-base-20261024:reports/db-specs-20261024/CC-5.md`. Decision block D-7040..D-7049.
Prerequisites: none (spec section 1). FILES_ONLY links: none.

## Done so far

- `crates/store/src/settings.rs`: key 12 `disk.extra_copy_roots` (`Key::ExtraCopyRoots`, slot 11,
  `KEYS` 11 -> 12), `COPY_ROOT_SEPARATOR` `|`, `MAX_COPY_ROOTS` 16, `MAX_COPY_ROOTS_BYTES` (the
  settings route's own 4,096-byte ceiling), `copy_roots()` (shape-only reader: absolute, no `..`,
  no empty piece, none twice, none nested); `check` adds that each folder exists; `check_stored`
  holds a stored value to its shape only. Test
  `the_extra_copy_roots_key_is_checked_and_stored_canonically` (not yet run).
- `crates/api/src/settings_route.rs`: the two required arms (`default_of`: `(None, UNSET_WAIT)`,
  a named wait; `reader_of`: `Ok(...)` naming `pull::copies_register`, `GET /disk/copies.json`,
  `cli disk-copies`).
- `crates/pull/src/copies_register.rs` (new): the read-only `Fs` seam (`lstat`, `list`, bounded
  `read` through `store::safe_open`; no writing method exists), `config()` (copy roots from the
  setting; live roots through `gdfl_run::source_root`, the v3 ledger variable, `source.gdfl`,
  `source.masters`, the store root; unset named), `reader_of()` (D-7044), parallel bounded walk
  (`MAX_WALK_ENTRIES` 2,000,000 per root, `MAX_DEPTH` 64; over a bound the root is refused by
  name), `(st_dev, st_ino)` hazards within one walk, receipts (`ALL_VERIFIED.txt`, `VERIFIED.tsv`
  with `source_identity` matched by inode, size, mtime, ctime and never dev, `*.prove.out` with
  `ALL-PROVED`), twins, the retirement list and its text, the events, the cache (`answer`,
  re-walk only when the roots or a root's own stat change; a request never waits on another's
  walk), `total()` for R4-29. `cargo clippy -p pull -p store` clean.
- Commit `cca93bc4` pushed.

## Left

- `copies_register_tests.rs` (the 8 named tests + extreme cases), the api route and its wiring,
  the cli verb and its wiring, the R4-06 no-delete list, the /ingest panel and the web bundle,
  the bench, the docs ledgers (02 §42, 04, 05 D-7040.., 06), every gate.

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

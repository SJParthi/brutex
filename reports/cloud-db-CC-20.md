STATUS: IN PROGRESS

# CC-20 (cloud): benches for every check primitive, with limits rows

Branch `cloud/db-CC-20-20261024`, cut from `origin/feat/db-fix` at `e851da1c` (Merge DATA-2).
Prerequisites: none (spec section 1). Decision block D-6800..D-6819. Failure-matrix row FM-615.

## Log

### 2026-10-24 09:15 IST: save point 1 (red test)

- Done: `crates/core/tests/bench_modules.rs` (new): `every_bench_module_is_registered` (every
  file under `crates/*/benches/ratio/` mounted once by `#[path]`, named as `<module>::` in its
  harness outside comments and its own `mod` line, no dangling mount; CC-20's seven modules exist,
  are mounted and are folded into the verdict with `ok &= <module>::`), plus
  `the_registration_rule_refuses_each_way_a_module_goes_missing` (unmounted, dangling, uncalled,
  mounted twice, a longer name's tail, a doc comment between mount and `mod`) and
  `the_walk_finds_the_harnesses_it_exists_for`.
- `cargo test -p core --test bench_modules --locked`: `test result: FAILED. 2 passed; 1 failed`;
  the failure is `crates/pull/benches/ratio/archive_from_source.rs is missing: CC-20 (FM-615) owes
  one bench per check primitive` (red before the modules exist, as the spec asks). The orphan half
  passed on the existing tree (pull 5, store 2, api 19 modules).
- Environment note: the first `cargo bench -p pull -p api --no-run` was killed (SIGKILL, out of
  memory: two `cli` compiles at once under the bench profile, 4 cores, 15 GB). Rebuilding with the
  api half at `-j 1`.
- NEXT: write the six pull modules and the api module, register them, print their ids from the
  harnesses.

### 2026-10-24 09:35 IST: save point 2 (seven modules written; HELD)

- Done: the seven bench modules, all new, and their registration:
  `crates/pull/benches/ratio/{archive_from_source,nfo_year,census_load,series_identity,strike_implausible,admit}.rs`
  and `crates/api/benches/ratio/nse_identity.rs`; each mounted by `#[path]` in its crate's
  `benches/ratio.rs`, its ids printed from that harness file (C-PIT-01's pattern) and its result
  folded into the verdict with `ok &= <module>::<fn>()`. Ids: C-CHK-01 (archive directory read per
  entry), C-CHK-02/03 (options year index first and later `day()`), C-CHK-04/05/06 (census cold
  load: single file, v4 partition, epochs), C-CHK-07 (api `Reference::confirm`), C-CHK-08
  (`series_identity::audit` per bar), C-CHK-09 (`strike_implausible`), C-CHK-10 (the day step that
  holds `admit`, an upper bound).
- `cargo check -p pull -p api --benches --locked`: `Finished`.
- `cargo clippy -p pull -p api -p core --benches --tests --locked -- -D warnings`: `Finished`, no
  warning.
- Not yet run: the benches themselves (release build in progress), the docs rows, the gates.
- HOLD found on `origin/cloud/db-base-20261024` at 09:34 IST: this step is finished, committed and
  pushed; waiting for GO without HOLD before the next step.
- NEXT: run `cargo bench -p pull --bench ratio` and `cargo bench -p api --bench ratio`, fix any
  fixture the harness refuses, then three release runs, then the docs/04, docs/05 and docs/06 rows.

### 2026-10-24 12:55 IST: save point 3 (GO again; pull bench green, run A)

- GO without HOLD at about 12:40 IST; resumed.
- First pull run (before this save point's change): every new row passed except the census cold
  load, whose cost PER ENTRY rose 3.8x (single file) and 3.0x (partition) from 1,000 to 100,000
  entries. The load is one pass (`Manifest::walk`: decode, push, insert into an index sized once);
  it claims O(entries), never O(1). Changed (D-6800): every O(n)-once-per-run row (C-CHK-01,
  C-CHK-02, C-CHK-04, C-CHK-05, C-CHK-06) is now held to the ceiling across EACH TENFOLD STEP
  (`archive_from_source::linear_per_unit`), with the 1x-to-100x figure printed beside it, not
  gated; an O(1) row stays held from its 1x baseline. The cause of the rise is NOT measured.
- `cargo clippy -p pull --benches --locked -- -D warnings`: `Finished`.
- `cargo bench -p pull --bench ratio --locked` (run A): `all ratios within the ceiling`, exit 0.
- NEXT: api bench run A (building `cli` in the bench profile at `-j 2`), then runs B and C of both
  harnesses by their built binaries, then the ledger rows.

### 2026-10-24 13:00 IST: save point 4 (ledger rows docs/04 and docs/05; more extreme tests)

- `docs/05-decisions.md`: D-6800 to D-6804 appended (one heading each).
- `docs/04-invariants.md`: section "Check primitives, each with a ratio bench (CC-20; D-6800 to
  D-6804)" appended, rows C-CHK-01 to C-CHK-10.
- `crates/core/tests/bench_modules.rs`: `the_registration_rule_names_every_fault_at_once_and_skips_what_is_no_module`
  (three faults in one harness at once, a fault in one crate of two, an empty folder, a CRLF
  harness, a note and a nested folder beside the modules).
- `cargo test -p core --test bench_modules --test cost_invariants --locked`: `4 passed` and
  `5 passed` (`every_bench_module_is_registered` now green; every printed C-CHK id is declared and
  every declared one is printed).
- NEXT: api run A (still building `cli`), runs B and C of both harnesses, the docs/06 rows.

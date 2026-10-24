STATUS: HELD

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

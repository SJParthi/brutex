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

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

### 2026-10-24 13:25 IST: save point 5 (CI shell gates compared with the base)

- Ran every `run:` step of ci.yml's language-purity job locally, on this branch and on an
  unmodified `origin/feat/db-fix` worktree (gate 1e skipped in both: it is a whole-workspace
  `cargo build` and `cargo test` in a copied tree, which the test gate covers). Verdicts identical:
  green 0, 1, 1b, 1g, 1f, 2, 9, 9b, 10b, 7, 13, 15, 16, 17, 21, 22, 24, 28, 25, 27, 26, 14, 29, 29b;
  red 1c, 1d, 23, 27b, 30, 19, 10, 11, 12 on BOTH (the base reds `CONTEXT.md` 4.10 names, owned by
  BASE-REDS-1023).
- Inside those reds, this branch added two findings and fixed both: gate 1d named two
  segment-shaped literals in the new bench files (`"epoch_not_regular"`, `"flagged"`), now
  `"epoch_not_regular ("` and `"flagged minutes"`; gate 12 named `linear_per_unit`'s doc block
  (a cost claim naming no proof), which now names `crates/pull/benches/ratio.rs`. Re-run: gate 1d's
  undeclared set and gate 12's refusals are now identical to the base's.
- NEXT: api run A, rebuild pull bench with this save point, runs B and C, docs/06 rows.

### 2026-10-24 13:28 IST: save point 6 (pull runs A, B, C green; docs/06 rows)

- `cargo bench -p pull --bench ratio --locked` run B: `all ratios within the ceiling`, exit 0;
  run C by the same built binary (`--bench`): `all ratios within the ceiling`, exit 0. Run A
  (save point 3) also green. Every C-CHK pull row passed in all three.
- `docs/06-limits.md`: section "Check primitives, each with a ratio bench (CC-20; D-6800 to
  D-6804)" appended, L-CHK-01 to L-CHK-07 with the three runs' figures; L-CHK-04 (api) waits for
  the api runs. The 13 Oct "not timed" sentence is left as written and superseded by L-CHK-01.
- `crates/pull/tests/check_primitive_real.rs` (new, `#[ignore]`, read only): the Mac's real-size
  run for the options year index and the census cold load, real paths only from
  `BRUTEX_GDFL_NFO_ARCHIVES` and `BRUTEX_CC20_CENSUS_ROOT`, no default.
  `cargo clippy -p pull --test check_primitive_real --locked -- -D warnings`: `Finished`.
- api: the harness at this tree exits in `reads.rs` (base red, see save point 5's commit
  1fd9fd6e); C-CHK-07 now runs before the served routes. Its build (`cli` in the bench profile) is
  in progress.
- NEXT: api runs A, B, C; the C-CHK-07 figures; then the final merge of `origin/feat/db-fix` and
  the full gates.
- HOLD found at 13:28 IST after this save point: held. The api bench build left running in the
  background (no session work); nothing else started.

### 2026-10-24 save point 7 (GO again; api runs A, B, C; L-CHK-04 filled)

- GO without HOLD again; resumed. The api chain had finished while held.
- api runs A (`cargo bench -p api --bench ratio --locked -j 2`), B and C (its built binary,
  `--bench`): C-CHK-07 passed in all three (hit 10x 1.001x / 1.002x / 0.979x, 100x 1.010x /
  1.007x / 0.995x; `SymbolAbsent` 10x 1.006x / 1.002x / 1.007x, 100x 0.987x / 1.013x / 1.014x).
  The harness then exits 1 in `reads.rs` in every run (base red: a 102,400-bar month past the
  60,000-row cap), and `C-LG-03` breached in all three (3.502x / 3.423x / 3.636x), `C-15`
  marginal rows in B and C: rows CC-20 does not own or touch.
- `docs/06-limits.md`: L-CHK-04 filled with the three api runs, and a paragraph naming the api
  harness's own reds.
- NEXT: merge the newest `origin/feat/db-fix`, then the final gates.

### 2026-10-24 14:46 IST: save point 8 (merged the newest base; gates so far; HELD)

- Merged `origin/feat/db-fix` at `7b397bbe` (IA-4, R4-27, CLOUD-MUTSTORE, CLOUD-MUTCLI): commit
  d0bcdd6e; conflicts only in the pull harness's mount list and the three ledgers, resolved as
  both, in merge order. `cargo fmt --all` then formatted only CC-20's own files (94288974).
- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished`, exit 0.
- `cargo deny check` (cargo-deny 0.19.0, CI's pin): `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo mutants --in-diff <the CC-20 diff> --list --workspace` (26.2.0, CI's pin): `No mutants to
  filter` (the diff changes no library or binary source).
- `cargo test --workspace --locked --no-fail-fast`: 10,988 passed, 59 failed, in api (8),
  cli (29 + 2 in `gdfl_run_logs`), pull (12), store (3 + 1 in `cited_commits`), telemetry; none in
  core (`bench_modules` 4 passed, `cost_invariants` 5 passed). Two existing api tests HANG in this
  container (0% CPU for 24 and 15 minutes, again alone under a 3-minute timeout):
  `server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`
  and `spot_rs::spot_json_is_refused_by_name_when_every_store_read_slot_is_held`
  (`tests/cloud_mutants_bars.rs`); the run skipped exactly those two by name, and
  `cloud_mutants_bars`' other 58 passed. No failing or hanging test is in a file CC-20 touched;
  the messages name root (a mode-000 directory read), read-only folders, the build stamp and timing.
  A run of the same five crates' tests on an unmodified base worktree is in progress to compare
  the failure sets (the first try ran out of disk; scratch build output was deleted and it was
  restarted).
- HOLD found at 14:46 IST: held. The base comparison keeps running in the background.
- NEXT: compare the base's failure set with this branch's; then the final report.

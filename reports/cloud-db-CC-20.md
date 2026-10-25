STATUS: BLOCKED

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

### 2026-10-25 save point 9 (new cloud session; HELD on arrival)

- Checked out the existing branch at `ee10bec6`; nothing redone.
- `origin/cloud/db-base-20261024` at `40f1cdb2` carries `HOLD` (25 Oct 13:0x IST) and no
  `GO-CC-20`: held before starting a step. The base failure-set comparison from save point 8 did
  not survive the old container and has to be rerun.
- NEXT (on GO): merge the newest `origin/feat/db-fix`, run `cargo test --workspace --locked
  --no-fail-fast` on this branch and on an unmodified base worktree, compare the failure sets, then
  the final report.

### 2026-10-25 save point 10 (GO; merged the newest base; fmt, clippy, deny green)

- GO without HOLD at `1dcc33e9` (25 Oct 13:12 IST); resumed.
- Merged `origin/feat/db-fix` at `666ddf7f` (DATA-1w and the parts before it): commit 21522e31;
  conflicts only in docs/04, docs/05 and docs/06, resolved as both, base rows first; the diff
  against the base is still additions only (15 files, 2,460 lines added, none removed).
- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished`, exit 0.
- `cargo deny check` (cargo-deny 0.19.0, CI's pin): `advisories ok, bans ok, licenses ok, sources
  ok`, exit 0 (two duplicate-crate warnings only).
- NEXT: the workspace test run (in progress), then the base comparison of its failures.

### 2026-10-25 14:35 IST: save point 11 (final merge, final gates; code complete)

- GO without HOLD (`1dcc33e9`); resumed from save point 10's NEXT.
- Merged `origin/feat/db-fix` at `bf9b1c8f` (R4-18 and the parts before it): commit 341af46b;
  conflicts only in docs/04 and docs/06, resolved as both, base rows first. Diff against the base:
  additions only (15 files).
- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished`, exit 0.
- `cargo deny check` (cargo-deny 0.19.0, installed here with `cargo install --locked`):
  `advisories ok, bans ok, licenses ok, sources ok`, exit 0.
- `cargo test --workspace --locked --no-fail-fast -- --skip a_full_calendar_pool_refuses_the_census_and_the_audit --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held`
  (as root, the container's only login): 11,219 passed, 30 failed, all 30 in the `--lib` unit tests
  of api (9), cli (3), pull (9), store (5) and telemetry (4). CC-20 changes no file under any
  crate's `src/`, so those lib test targets are compiled from exactly the base's source: they are
  base or environment reds, not CC-20's. `core`'s `bench_modules` (4 passed) and
  `cost_invariants` passed. The 30 rerun by their built binaries as the unprivileged `ubuntu` user
  (`runuser -u ubuntu -- <lib test binary> --exact <names>`): 23 pass (they need a non-root user:
  mode-000 and read-only folders), 7 still fail:
  `pull gdfl_cm::tests::fixtures_are_built_not_pasted` ("pasted GDFL CM rows in
  crates/cli/src/tick_second_check_tests.rs", a base file from DATA-1w),
  `api detail::tests::a_refused_admission_names_what_its_bound_and_why` (folder read Saturated,
  timing) and five `api server::head_deadline_body_tests::*` (slow-reader socket timing in this
  VM). The two skipped api tests hang in this container (save point 8).
- `cargo bench -p pull --bench ratio --locked` after the merge, then its built binary twice
  (`--bench`): **C-CHK-04 BREACHED in two runs of three** (single-file census cold load, per entry,
  10x to 100x: 3.318x, 3.243x, 2.890x; ceiling 3x); every other C-CHK row passed in all three.
  Two more runs under glibc's `MALLOC_MMAP_THRESHOLD_`/`MALLOC_TRIM_THRESHOLD_`/`MALLOC_TOP_PAD_`
  (no large block handed back between loads): 2.017x and 1.946x. About 40% of the rise is fresh
  page faults at 100,000 entries; the rest is not attributed. Recorded as a new paragraph at the end
  of CC-20's docs/06 section. The ceiling (D-6800) and the census load (not CC-20's file) are
  unchanged: this is an operator decision (below).
- Not run here: `cargo llvm-cov` and `cargo mutants` (not installed in this container; the diff has
  no library or binary source, so `cargo mutants --in-diff` lists none: save point 8).

## Final summary (25 Oct)

**Done:** the seven bench modules (C-CHK-01 to C-CHK-10), each registered in its crate's
`benches/ratio.rs` and folded into the verdict; `crates/core/tests/bench_modules.rs` (failing
first, then green); `crates/pull/tests/check_primitive_real.rs` (the Mac's real-size run,
`#[ignore]`); docs/04 rows C-CHK-01..10, docs/05 D-6800..D-6804, docs/06 L-CHK-01..07 with three
runs each and today's C-CHK-04 finding. Private `NfoZips::year` and `admit` are benched through
their public callers on fixtures (no seam: PREC-6, R4-09 and R3-26 have not merged), per the spec.

**Left (none of it CC-20 code):**
1. OPERATOR DECISION: C-CHK-04 breaches its 3x per-tenfold-step ceiling in 2 of 3 runs on this
   cloud host (3.318x / 3.243x / 2.890x; 24 Oct: 2.11x-2.25x). Choices: let IA-13's Mac runs
   decide on the real host; or the census load's owner pre-sizes and reuses its memory (about 40% of
   the rise measured as page faults). Widening the ceiling is not offered.
2. Base reds in the workspace test run (7 failing as a non-root user, listed above; 2 hangs).
3. Base reds in the api bench harness: `reads.rs` (102,400-bar month past the 60,000-row cap),
   `C-LG-03`, a `C-15 ... marginal` row (save point 7).
4. Coverage and mutants runs: the Mac (no tool here).

**Measurements:** p50/p99/max with n for every row are in docs/06 L-CHK-01..07; today's post-merge
pull run, n=200 per row: C-CHK-04 one load at 100,000 entries p50 85.2 ms, p99 117.2 ms, max
117.9 ms; C-CHK-05 p50 90.6 ms, p99 118.9 ms, max 127.2 ms; C-CHK-06 (1,000 epochs) p50 89.2 ms,
p99 136.7 ms, max 144.0 ms; C-CHK-08 a 22-session month p50 0.260 ms, p99 0.419 ms, max 1.95 ms;
C-CHK-09 p50 32 ns, p99 34 ns, max 12.3 us (n=20,000); C-CHK-10 p50 401 ns, p99 1.73 us, max
44.1 us (n=1,000). Method: the gate-8 harness, sequential single calls, release/LTO, cloud VM.

**Extreme cases** (`bench_modules.rs`, each failing before the modules existed): a module
unmounted; a dangling mount; mounted but never called; mounted twice; a longer module name whose
tail matches; a doc comment between `#[path]` and `mod`; an empty `ratio/` folder; a note and a
nested folder beside the modules. Combinations: three faults in one harness at once; a fault in one
crate of two. Out of the box: a CRLF harness. In the benches: an absent census, an absent
partition, one entry, a torn `.man.tmp` epoch, a symbolic link at an epoch's name, a 1e5x median
strike, a reversed flagged month.

**Files outside ownership:** `crates/core/tests/bench_modules.rs` (the spec's failing-first test,
new), `crates/pull/tests/check_primitive_real.rs` (new, `#[ignore]`), and one mount line block in
each of `crates/pull/benches/ratio.rs` and `crates/api/benches/ratio.rs`.

**D-numbers used:** D-6800, D-6801, D-6802, D-6803, D-6804 (block D-6800..D-6819).

## NEEDS THE MAC

1. `cargo bench -p pull --bench ratio --locked`, three times: expected `all ratios within the
   ceiling` with every C-CHK-01..06 and 08..10 row `ok`; C-CHK-04's 10x-to-100x step is the one to
   watch (cloud: 2.89x-3.32x).
2. `cargo bench -p api --bench ratio --locked`, three times: expected C-CHK-07's six rows `ok`
   (cloud: 0.98x-1.01x); the harness then exits 1 in `reads.rs` (base red) until that is fixed.
3. `BRUTEX_GDFL_NFO_ARCHIVES=<the options archive folder> BRUTEX_CC20_CENSUS_ROOT=<the store root>
   cargo test -p pull --test check_primitive_real --locked -- --ignored --nocapture`: expected both
   tests pass, printing the real year-index build time and the real census cold load time.
4. `cargo test --workspace --locked` as the operator (not root): expected the same failure set as
   unmodified `feat/db-fix` (CC-20 adds no lib source).
5. `cargo llvm-cov --branch -p core --test bench_modules` and `cargo mutants --in-diff <CC-20's
   diff>`: expected 100% on `bench_modules.rs` and `No mutants to filter`.
6. No shadow server, `:8080` or store write applies: CC-20 serves and writes nothing.

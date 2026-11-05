STATUS: BLOCKED OUTSIDE
ASSEMBLED: bbacf363b5d2f3be7a2c19a6423cacfe361fdb67

# BASE-COV-LIBS-LSA: the COV-LIBS assembly (3 Nov 2026 cloud wave, run 5 Nov 2026)

TOKEN `BASE-COV-LIBS-LSA-20261103-1`. Branch `cloud/cov-libs-lsa-20261103`. Code tip (`git rev-list -1 HEAD -- .
':(exclude)reports'`) and the json's `head`: `a944a674d6aa606d670daffd1c63d013bb3933e6`.

## Outcome

- **Assembled**: the four shards stacked by the kit's script with every proof holding (step 1).
- **Settled**: LS4's two `engine/src/lib.rs` patches and four judgment rows; LS2's allocator rows, through a
  `runner` reservation door (D-8569). What could not be settled is left for the operator by name (L-BCL-6).
- **Measured**: Route B on the ten crates whole, every uncovered row classed. Lines 172,204/174,466 and branch sides 12,896/14,228 over 343 files read; the 3,594 uncovered rows are (F) 1,677, (U) 362, ignored-test bodies 1,536, macOS-only 10 and the operator's 9. None is a GAP.
- **The bar is NOT met**, and nothing left can be closed by a test in this package's files:
  - the bodies of `#[ignore]`d tests (L-BCL-3);
  - the 14 `benches/` files no route reads (L-BCL-1);
  - nine rows for a ruling (L-BCL-6);
  - the macOS rows and every figure, which need the Mac's Route B on its pin.

  So D-7250's narrow reading still holds for these crates (D-8568), and `Merge BASE-COV-LIBS:` must wait.
- **Gates** on the pinned stable: fmt, clippy `-D warnings` and the ten crates' tests. All green as `nobody`: 4,075 passed, 0 failed, 17 ignored. As root, only the four proved root-only `telemetry` reds fail.
- **Gate 20**: UNMEASURED. CI's own job command cannot reach a usable profile on this host: `api`'s lib tests hang on the base's 4-core pool red and fail six tests before it, and `cargo test` stops there, so telemetry's tests never run. Nothing is written to ci.yml or §54; it is the Mac's (finding GATE20-CI).

## Step 1: the stack (done)

All four shard branches were fetched; each origin tip equals the tip the brief wrote, so TIP = the written tip for all
four. FROM = the kit commit `eeec369be7dc00c8ca71de19a797641947100905` for all four. Every shard has code outside
`reports/` (`git diff --name-only <FROM> <TIP> -- . ':(exclude)reports'` prints 43, 42, 43 and 23 files), so none was
left out. The kit's `replay-minus-reports.sh` (sha256 `f223185d...`, as MANIFEST.md gives) ran four times, under
`nice -n 15`, in the order LS1, LS2, LS3, LS4:

| call | TIP | ONTO | NEWREF | result line |
|---|---|---|---|---|
| LS1 | `25d6eb50acf7f73f51926fee665e525dae9c47d2` | START | `refs/heads/stack/lsa-1` | `OK refs/heads/stack/lsa-1 = 2f51700d1fe03cdb26a2d1cd2557787abcf78a17 (20 commits replayed onto bd04fb299d23, 11 skipped as reports only; P1-P4 hold; 43 files in X)` |
| LS2 | `0630015e806e1fb83798c0b39acc9af75b87a565` | `stack/lsa-1` | `refs/heads/stack/lsa-2` | `OK refs/heads/stack/lsa-2 = 2c9a3036c1e34ce28d8b48564dd08b04e524f13a (54 commits replayed onto 2f51700d1fe0, 19 skipped as reports only; P1-P4 hold; 42 files in X)` |
| LS3 | `16a9e6c38d4165e334811ea0b841e350be6a1f42` | `stack/lsa-2` | `refs/heads/stack/lsa-3` | `OK refs/heads/stack/lsa-3 = dd62496f51bb1a30f940f1387bd3e92b0b4e6de8 (11 commits replayed onto 2c9a3036c1e3, 7 skipped as reports only; P1-P4 hold; 43 files in X)` |
| LS4 | `1872fde300490b6261299f7ca4d6b75c8fe9c202` | `stack/lsa-3` | `refs/heads/cloud/cov-libs-lsa-20261103` | `OK refs/heads/cloud/cov-libs-lsa-20261103 = bbacf363b5d2f3be7a2c19a6423cacfe361fdb67 (14 commits replayed onto dd62496f51bb, 4 skipped as reports only; P1-P4 hold; 23 files in X)` |

The `stack/lsa-*` refs are local only and were never pushed.

Shard report line 1: LS1 `STATUS: DONE`, LS2 `STATUS: BLOCKED OUTSIDE`, LS3 `STATUS: DONE`, LS4 `STATUS: BLOCKED OUTSIDE`.

## Step 2: the shards' BLOCKED OUTSIDE needs (done)

- **LS4, `crates/engine/src/lib.rs`** (`3069f0c9`): patch 1 (sha256 `c4cbc10e...`, as LS4's report gives) and patch 2
  (`1b62c7b0...`) applied in order, unchanged. LS4's four judgment rows:
  - `the_join_emits_exactly_one_candidate_per_pair`: the frontier is widened to every 2-subset of 0..=3 plus
    `{4,5} {4,6} {5,6}`, so five candidates survive the prune and the distinctness check runs on them (it held over an
    empty level); the block arithmetic is restated for the five blocks (3 + 1 + 0 + 1 + 0 = 5), and a new assertion
    states that all five survive.
  - `caller_input::a_checkpoint_observer_gets_one_terminal_boundary_and_no_later_level`: the observer's `Err` on a call
    after the terminal boundary is an `assert!` (the same failure, now the recipe's (F) shape).
  - Red checks, each then restored with `git checkout` and the tree checked clean: every survivor appended twice (red at
    the new count assertion, left 10, right 5); the third survivor given the second's mask (red at the distinctness
    assertion); the retained sink calling its reporter a second time (red at the new assertion).
  - `cargo test -p engine --locked` (stable 1.97.1, root): 145 passed, 0 failed (the two patch-1 reds are gone).
- **LS2, the 41 rows for a ruling** (`d096f659`, D-8569, D-8563): the method is already decided (D-6017, D-7329, and the
  main session's answer to LS1 recorded as D-8561: a `#[cfg(test)]` seam that runs the real code, never a limit), so
  the assembly applied it rather than leave the rows open:
  - a test-only door, `crates/runner/src/reservation_refusal.rs` (declared in `runner/src/lib.rs`), refuses the n-th
    seamed reservation with a request of `usize::MAX`, so `try_reserve_exact` answers the genuine `TryReserveError`;
    eight sites seamed: `observed_ranges` (stops, targets, trails), `admitted_ratio_pairs`, `ResolvedGridViewV1::ladders`
    and `coordinate_row_offsets`, `grid::replay_universe_v1` and `replay_universe_from_candidates_v1` (evidence, scratch),
    `signal_candle_stop::days_before`;
  - `ratio_bitmap_of` by a direct call with a length of `isize::MAX + 1` (no seam), `canonical_coordinate_ordinal`'s slot
    overflow by a direct call with a stop rung of `usize::MAX`;
  - (U) with a proof: the three `len().checked_add(1)` of `coordinate_row_offsets` and `canonical_coordinate_ordinal`
    (a `Vec<i64>` holds at most `isize::MAX / 8` elements);
  - left for the operator (L-BCL-6): the `(S + 1)(T + 1)` product (two ladders over 2^32 rungs), the `?` after
    `coordinate_row_width` in the same function (a row wider than `usize::MAX` cells, which only a resealed resolution
    over hundreds of MiB of ladders reaches), and the five [HASH] rows (a blake3 collision). Finding F-LS2-1 stays a
    finding (D-8563).
  - Tests: `exit_grid_policy::tests::every_buffer_only_a_refusing_allocator_fails_is_refused_by_name_with_its_size`,
    `grid::cover_tests::a_replay_whose_buffer_only_a_refusing_allocator_fails_names_its_paths`,
    `signal_candle_stop::tests::a_preparation_whose_day_table_the_allocator_refuses_is_an_allocation_error`,
    `reservation_refusal::the_armed_reservation_alone_is_refused_and_the_arming_is_spent`: 4 passed on stable.
  - Red checks, five rounds of hand changes (each restored, tree clean after each): A (the observed buffer's size, the
    scratch's paths, the day table's error) red at the three tests' target assertions; B (the door never spent) red at
    the door test; C (a ladder copy's size, the replay candidates' paths) red at both targets; D (the coordinate slot
    clamped instead of refused) red at the ordinal assertion (left `Ok(None)`); E (the bitmap's size) red at the bitmap
    assertion.
- **LS1, LS3**: nothing outside their files; their docs drafts are step 4.

## Step 3: Route B on the ten crates whole

**Instrument.**
- The kit's `d3-route-b-cloud.sh`, unchanged (sha256 `f032b525...`, as MANIFEST.md gives), with `d3-covset.mjs`
  (`ecef3bdc...`).
- The pinned nightly, installed here as `nightly-2026-08-08`: `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13
  2026-08-07)`, and its `llvm-cov` and `llvm-profdata` print `LLVM version 23.1.0-rust-1.99.0-nightly`.
- Host: x86_64-unknown-linux-gnu, 4 cores, 15 GiB, root. So these are the builder's figures; the Mac re-measures on
  aarch64-apple-darwin.
- Test binaries ran as `nobody` (uid 65534) through
  `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='setpriv --reuid=65534 --regid=65534 --clear-groups'`, as CI and the
  Mac run them (non-root), with `umask 000` so their profiles land. Also: `TMPDIR=/dev/shm/lsa-tmp` (tmpfs, 1777),
  `GIT_CONFIG_GLOBAL` naming the root-owned trees `safe.directory`, `CARGO_PROFILE_DEV_DEBUG=0`.
- Each run used a detached worktree at its tip, a new empty profile folder, and the set file
  `reports/base-cloud-20261103/deep-BASE-COV-LIBS-LSA-d096f65942d4-set.txt.md` (362 coverage files whole, plus 1,334
  mutants lines for the deferred pass).

**Runs** (every code commit after a run's tip touches only crates that run did not measure):

| run | tip | crates | tests (passed / failed / ignored) | wall |
|---|---|---|---|---|
| 1 | `3069f0c9d23c` | `core` `costs` `engine` `greeks` `indicators` `lake` `store` `telemetry` `vocab` | 273/1/0, 385/0/2, 145/0/0, 59/0/0, 528/0/0, 150/0/6, 1104/0/8, 256/0/1, 190/0/0 | 16:02-16:39 IST (build 1 min; `store` 21 min) |
| 2 | `d096f65942d4` | `runner` | 984/0/0 | 16:17-18:26 IST (tests 127 min, beside run 1, the gates and gate 20) |
| 3 | `0b7aa0d6d801` | `core` | 274/0/0 | 16:53-16:54 IST |

Run 1's one failure is `core`'s `every_cited_decision_heads_an_entry_or_is_named_pending`: at `3069f0c9` the D-856x
entries it checks were not yet written. Run 3 measures `core` again once they were. Its figures replace run 1's, and its
seven success-path lines were the only rows that differed. Every script run printed `ROUTE-B DONE ... WITH NOT PROVED
lines above` and exited 3, for the bench and no-code `lib.rs` files alone.

**Figures, as measured** (lines covered/counted; branch sides covered/counted):

| crate | lines | branches | uncovered lines | uncovered sides | files read |
|---|---|---|---|---|---|
| `core` | 7,746/7,774 | 602/640 | 28 | 38 | 21 |
| `costs` | 11,568/11,670 | 480/502 | 102 | 22 | 26 |
| `engine` | 5,484/5,494 | 528/560 | 10 | 32 | 11 |
| `greeks` | 1,918/1,921 | 93/96 | 3 | 3 | 7 |
| `indicators` | 16,909/16,914 | 1,892/2,078 | 5 | 186 | 39 |
| `lake` | 5,678/6,135 | 305/364 | 457 | 59 | 17 |
| `runner` | 52,070/52,264 | 3,671/4,022 | 194 | 351 | 75 |
| `store` | 53,276/54,692 | 3,449/3,990 | 1,416 | 541 | 112 |
| `telemetry` | 9,370/9,397 | 636/678 | 27 | 42 | 16 |
| `vocab` | 8,185/8,205 | 1,240/1,298 | 20 | 58 | 19 |
| **all ten** | **172,204/174,466** | **12,896/14,228** | **2,262** | **1,332** | **343** |

**NOT PROVED by the route (named, never counted covered):**
- the 14 `benches/` files of the ten crates (L-BCL-1): every `[[bench]]` is `test = false`, so `cargo test --no-run`
  builds no object for them;
- the five `src/lib.rs` files with no executable line (L-BCL-2): `core`, `costs`, `greeks`, `lake`, `store`.

Each script run exited 3 for these lines alone.

**Every uncovered row, classed** (`base-cov-libs-gaps-a5.txt.md`; the (F) and (U) rows with their rule or proof in
`base-cov-libs-fu.txt.md`):

| class | rows | where |
|---|---|---|
| (F), first clause: the failing side of an assertion or panic in test code | 1,581 | every crate |
| (F), LS1's step-4 rows (bound 25, skip 28, collect 30, guard 7, fail 6) | 96 | `store` |
| (U) with its proof | 362 | `store` 95, `runner` 261, `lake` 2, `costs` 2, `indicators` 1, `vocab` 1 |
| inside an `#[ignore]`d test (L-BCL-3) | 1,536 | `store` 1,108, `lake` 353, `costs` 70, `telemetry` 5 |
| only macOS takes it (owed to the Mac's run) | 10 | `store` `tests/catalog.rs` |
| for the operator's ruling (L-BCL-6) | 9 | `runner` 7 (`resolved_grid_view.rs` 232 and 261; `validate.rs` 3070-3073, five rows), `telemetry/tests/global_absent.rs` 2 |
| **GAP** | **0** | |

Against the shards' own final lists:
- **`runner`** went from 36 [FAULT] and 5 [HASH] rows to 2 and 5. D-8569's door reached the rest; three became (U), and
  `ratio_bitmap_of` is reached by a direct call.
- **`engine`** went from LS4's 29 `lib.rs` rows for LSA to none: patches 1 and 2, the judgment rows, and one guard side
  that is now (F).
- **Every other crate** has the shard's own figure. `store` has one more branch side covered, a bounding arm the
  `nobody` run takes.

**How the rows were classed.**
- A row keeps the class its shard's list gave it. The lists are on the shard branches; their fenced contents are carried
  verbatim in `base-cov-libs-fu.txt.md`'s appendix.
- Rows are matched by file, line and (for a branch) column and side. In a file the assembly changed, the line is
  followed back to the shard's tip through `git diff -U0 bbacf363 <tip>`.
- A row no list carried is classed by hand, and the lists say so (`[LSA: ...]`).
- Rows classed by hand (9), each with its reason in both lists:
  - `engine/src/lib.rs:3035:48 False`: (F), the distinctness assertion's failing side;
  - `runner/src/exit_grid_policy.rs:8314` (three operand sides): (F), my test's `assert!`;
  - `runner/src/resolved_grid_view.rs` 226, 229 and 284: (U), a `Vec<i64>` holds at most `isize::MAX / 8` elements;
  - `runner/src/resolved_grid_view.rs` 232 and 261: for the operator, inputs of a gibibyte and more.
- LS1's step-4 rows (96 here: bounding, skip, failure-collection, fault-mock guard and failing arms of a test's own
  checks) are (F) under the reading LS1 records as the main session's answer of 5 Nov. The Mac's check confirms that
  reading.
- `store`, measured here as `nobody`, leaves the same rows LS1's root run left, less one bounding arm. So no row turns on
  the user the suite runs as.

## Step 4: docs

`docs/05-decisions.md`:
- D-8560 to D-8567, from the shards' final drafts, unchanged in substance;
- D-8563 also records the assembly's answer to LS2's question;
- D-8568, the assembly's record with the figures above;
- D-8569, `runner`'s reservation door.

No id of D-8560..D-8569 headed an entry at START (`git grep -n -E '^#+ D-856[0-9]' bd04fb29 -- docs` prints nothing).
Code cites D-8561, D-8564, D-8565 and D-8569, and each heads an entry: `core`'s `decision_citations` passes.

`docs/04-invariants.md`:
- BCL-10 (LS3), BCL-11 to BCL-14 (LS4), BCL-15 (D-8569);
- each proof is "`test` in `crates/...rs`", which `.github/invariant_paths.rs` resolves against that exact file.

`docs/06-limits.md`:
- L-BCL-3 now names the ignored tests;
- L-BCL-4 is closed for `runner` too;
- L-BCL-5 carries the assembly's figures;
- new L-BCL-6 holds the operator's rows.

docs/04 and docs/06 cite no commit hash, because `store/tests/cited_commits.rs` reads both whole.

## Step 5: final gates

**At `62535479`** (code identical to `d096f659`): a fresh worktree `/var/tmp/lsa-gate-tree`, target
`/var/tmp/lsa-gate-tgt`, the pinned stable `rustc 1.97.1 (8bab26f4f 2026-07-14)` from `rust-toolchain.toml`, and
`TMPDIR` on `/dev/shm`. The script is `gates.sh` in the session's scratchpad.
- `cargo fmt --all --check`: rc 0 (16:40 IST).
- `cargo clippy -p core -p costs -p engine -p greeks -p indicators -p lake -p runner -p store -p telemetry -p vocab
  --all-targets --locked -- -D warnings`: rc 0 (16:41 IST).
- `cargo test -p <the ten> --locked --no-fail-fast`, test binaries as `nobody` as CI and the Mac run them: rc 0. 135
  result lines, **4,075 passed, 0 failed, 17 ignored** (16:42-17:05 IST).
- The same as root: rc 101; 4,071 passed, 4 failed, 17 ignored (17:05-17:23 IST). The four are root-only host reds
  in `telemetry`'s `sink::tests`:
  - `a_coverage_file_is_read_line_by_line_and_extended_where_it_stands`;
  - `a_failed_delete_is_counted_named_and_left_for_the_next_start`;
  - `a_rename_aside_that_cannot_happen_fails_the_roll_by_name`;
  - `an_audit_stream_that_cannot_be_looked_for_is_an_error`.

  Each `chmod`s a directory and expects the host to refuse, which root bypasses (the base's "telemetry 4",
  CONTEXT.md). **Proof:** the `nobody` run at the same tip passes all four, as LS4's per-test `setpriv` runs did.
  Nothing else fails as root.

**At `604806f8`** (the docs commit of D-8568), as `nobody`: every test target that reads the docs.
- `core`: `decision_citations`, `cost_invariants`, `findings`, `graph`.
- `store`: `docs`, `cited_commits`, `libm_key`.
- `runner`: `d_0694_record`.
- `vocab`: `no_py_language`, `no_runtime_spawn`, `workspace_is_rust`.
- `greeks`: `standalone`.

That is 12 result lines, 0 failed. `a944a674` then rewords one sentence of D-8569, and `decision_citations` passes there too. With the two new lists staged, `cargo test -p vocab --locked --test no_py_language
--test workspace_is_rust` passed 12 and 19. No code changed after `62535479`.

Mutants: deferred (operator 30 Oct). `cargo deny check` was not run: no `Cargo.toml` or `Cargo.lock` changed.

`origin/feat/db-fix` is `f01d7a0e430e8f9b01e424a299b0bed20738a0ef` (`git ls-remote`, checked at the start and before the
final push). It has not moved, so nothing was merged and nothing needs reporting.

## Step 6: gate 20

**UNMEASURED: CI's own job command cannot reach a usable profile on this host.** So nothing is written to
`.github/workflows/ci.yml` or to `docs/06-limits.md` §54, and no gate-20 commit exists.

**How it was run.** `.github/workflows/ci.yml`, job `coverage`, steps Measure and Gate 20, copied into `gate20.sh` in
the session's scratchpad:
- on the final code (`0b7aa0d6`, code identical to `d096f659`), in a fresh worktree `/var/tmp/lsa-ci-tree` with a fresh
  target dir `/var/tmp/lsa-ci-tgt`;
- the pinned stable 1.97.1, and `cargo-llvm-cov 0.8.4` installed as the job installs it (`cargo install cargo-llvm-cov
  --version 0.8.4 --locked`);
- test binaries as `nobody`, as CI's runner is not root;
- `CARGO_PROFILE_DEV_DEBUG=0` for room (CONTEXT.md allows it; no coverage mapping depends on it).

**What happened.**
1. `gates_jobs` built, and its own tests passed (26). `no-crates` found 13 crate manifests, so it measures.
2. `cargo llvm-cov --workspace --locked --fail-under-lines 90 --fail-under-regions 89 --summary-only` built the
   workspace, and `api`'s lib test binary ran first.
   - Six `api` tests failed on this host: `detail::tests::a_refused_admission_names_what_its_bound_and_why` and five
     `server::head_deadline_body_tests` slow-reader timings (CONTEXT.md's "five slow-reader timings").
   - Then `server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`, which
     re-runs itself in a child, left that child asleep at 0% CPU on its pool's barrier for 37 minutes. This is the
     base's "one 4-core pool test" red (CONTEXT.md). The pool holds a fixed 8 slots and the child's runtime has 2
     workers.
   - I stopped the run after 83 minutes (`MEASURE rc=143`, 18:19 IST).
3. `cargo llvm-cov report --locked --text` read the partial profile, and `gates_jobs gate-20` refused it itself:
   `NO USABLE COVERAGE PROFILE — 7186 of 8622 telemetry lines` read as uncovered (`GATE20 rc=1`).

**Why no figure can come from this host.** Without `--no-fail-fast`, `cargo test` stops at the first failing test
binary, and `api`'s lib binary comes first in the workspace and fails here (and hangs). So `telemetry`'s own tests never
run under CI's command on this host, and the profile gate 20 reads would be `api`'s alone. A command with
`--no-fail-fast` or a skipped test would not be CI's own, so no count is taken from one.

Gate 20 stays as START declares it: `level.rs 1`, `lib.rs 6`, `record.rs 1`, `sink.rs 20`, `tail.rs 3`. Finding
`GATE20-CI`: CI's own run on the tip, or the Mac's run of the same command, must measure the counts.

**For context only, not gate 20's instrument.** Route B, crate-scoped, as `nobody`, at `3069f0c9`. Uncovered lines under
`crates/telemetry/src/`:
- `sink.rs` 19;
- `record.rs` 1;
- `level.rs` 1;
- `state_events_tests.rs` 3;
- `lib.rs` 0;
- `tail.rs` 0.

LS4 measured the same. A workspace profile is a different measurement, as §54 says, and its counts may differ.

## Files changed by this session

- **The shards' code**, replayed byte for byte (step 1): 151 files under `crates/`, every one a shard's.
- **The session's own:**
  - `crates/engine/src/lib.rs`: test code; patches 1 and 2 and the judgment rows;
  - `crates/runner/src/reservation_refusal.rs` (new);
  - `crates/runner/src/lib.rs`: its `#[cfg(test)]` declaration;
  - `#[cfg(test)]` seam lines in `crates/runner/src/{exit_grid_policy,grid,resolved_grid_view,signal_candle_stop}.rs`;
  - tests in `crates/runner/src/{exit_grid_policy,grid_cover_tests,signal_candle_stop_tests}.rs`;
  - `docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md`;
  - no change to `.github/workflows/ci.yml` or to docs/06 §54 (gate 20 UNMEASURED).
- **Reports** under `reports/base-cloud-20261103/`:
  - `OWNER-BASE-COV-LIBS-LSA.md`;
  - this report;
  - `fix-BASE-COV-LIBS-LSA.json.md`;
  - `base-cov-libs-gaps-a5.txt.md`;
  - `base-cov-libs-fu.txt.md`;
  - `deep-BASE-COV-LIBS-LSA-d096f65942d4-set.txt.md`.

No production behaviour changed. Every non-docs line this session added is test code, a `#[cfg(test)]` line, or one of
the eight `let asked = <the size the reservation already asked for>;` bindings beside a seamed reservation. A build asks
for the same size through the same call.

## Tests added by this session (4), and the tests changed

- New:
  - `exit_grid_policy::tests::every_buffer_only_a_refusing_allocator_fails_is_refused_by_name_with_its_size`;
  - `grid::cover_tests::a_replay_whose_buffer_only_a_refusing_allocator_fails_names_its_paths`;
  - `signal_candle_stop::tests::a_preparation_whose_day_table_the_allocator_refuses_is_an_allocation_error`;
  - `reservation_refusal::the_armed_reservation_alone_is_refused_and_the_arming_is_spent`.
- Changed in `engine` (`lib.rs`):
  - `tests::the_join_emits_exactly_one_candidate_per_pair`;
  - `caller_input::a_checkpoint_observer_gets_one_terminal_boundary_and_no_later_level`;
  - LS4's patch 2 tests: `the_bench_primitives_agree_with_the_level_they_stand_for`, the exhaustive-join oracle's
    fixture, the cumulative counters, the handed batches;
  - patch 1's two source-shape tests.

Red checks: as in step 2, each test was shown failing under a hand change of the rule it pins, then restored.

**Extreme cases** in the new tests:
- the first and the last reservation of each door refused;
- a refusal armed past every reservation (the answer is unchanged and the arming reported unspent);
- a length of `isize::MAX + 1`;
- a stop rung of `usize::MAX`;
- a single candidate (evidence and scratch both ask for one path);
- three duplicate candidates refused for memory before their duplication is read (combined);
- one- and five-minute signal timeframes;
- the door at `u32::MAX` with a body that reserves nothing;
- a real every-bar walk over eight generated sessions (out of the box).

## Benches

None run, and none owed. The session's only production lines bind each seamed reservation's size to a local
(`let asked = ...`): the same size, the same call, one move, so no step gains work. The shards changed no production
path's cost (each says so). Gate 8 and every p99 are the Mac's.

## Decisions (this session's ids: D-8568, D-8569; D-8560 to D-8567 written from the shards' drafts)

- **D-8568**: the assembly's record. Four shards stacked; LS4's patches and judgment rows; LS2's rows settled by D-8569
  or named. Route B figures of the ten crates whole, every row classed. The bar NOT met, so D-7250's narrow reading
  still holds for these crates.
- **D-8569**: `runner`'s test-only reservation door: eight seamed reservations reached by their real paths, as
  `engine`'s D-7329.

## Findings of the package, as the assembly leaves them

| id | status | what |
|---|---|---|
| LSA-STACK | fixed | four shards stacked; every proof P1-P4 held; ASSEMBLED `bbacf363` |
| LS4-01 | fixed | patch 1: the shipping region of `engine/src/lib.rs`'s source-shape tests (D-8566); the two reds gone |
| LS4-10 | fixed | patch 2: `lib.rs` test-code rows (D-8566) |
| LS4-11 | fixed | the four judgment rows (D-8568): a check over an empty level, and an observer's `Err` |
| LS2-LISTED | partly fixed | 36 [FAULT] rows: 31 now reached (D-8569's door, or a direct call of `ratio_bitmap_of`), 3 (U), 2 for the operator; the 5 [HASH] rows for the operator (D-8563, L-BCL-6) |
| LSA-ROWS-L-BCL-6 | operator | 9 rows: `resolved_grid_view.rs` 232 and 261, `validate.rs`'s five [HASH] rows, `telemetry/tests/global_absent.rs` 106-107 |
| F-LS2-1 | open (operator) | `grid::entry_fills` and `trade::round_trip` price a sub-tick entry low differently; loud, misnamed refusal; its fix needs an id (D-8563) |
| LS4-12 | operator | `global_absent.rs`'s macro argument, never run by design (in L-BCL-6) |
| L-BCL-3 (LS4-13 and the shards') | operator | 1,536 rows inside `#[ignore]`d tests |
| L-BCL-1 | operator | 14 `benches/` files no route reads |
| LS4-09 | host red, proved | 4 `telemetry` sink tests fail as root only; pass as `nobody` at the same tip |
| GATE20-CI | not proved | gate 20 UNMEASURED here; CI's own run (or the Mac's) must measure it |
| LS1-ROOT-REDS, LS1-HOST-RACE, LS1-NEW-ROWS, LS1-TEST-CODE, LS1-TEST-FIXES, LS1-COV-STORE, LS1-SAFE-OPEN-MAC | as LS1 left them | fixed; the `safe_open` arm is (U) on the Mac's target (D-8561) |
| LS2-STEP1, LS2-OPEN, LS2-COVERAGE | as LS2 left them | fixed; the coverage is this assembly's figure now |
| BCL-LS3-lake, BCL-LS3-indicators, BCL-LS3-vocab | as LS3 left them | fixed (D-8564, D-8565) |
| LS4-02 to LS4-08 | as LS4 left them | fixed or (U) with proof (D-8567) |
| LS4-14 | superseded by GATE20-CI | gate 20 was LSA's to take; UNMEASURED on this host |

## NEEDS THE MAC

- **Route B on its pin** (aarch64-apple-darwin) for the ten crates, the bar's own measurement. Owed to it, never claimed
  here:
  - the macOS-only lines (`store`: `file.rs`'s `cfg(target_os = "macos")` items, `src/open_flags.rs`, and the 10
    macOS-only rows of `tests/catalog.rs`);
  - `safe_open.rs:104:19 False` and `:107`, which are (U) there with LS1's proof (D-8561 item 4).
- **The non-root 14-core run** of the ten crates' tests. Here: root, 4 cores, the suites run as `nobody`.
- **L-BCL-3**: the `#[ignore]`d real-data and timing tests (L-BCL-3 names each).
- **Gate 20**, UNMEASURED here (finding `GATE20-CI`; no cloud figure exists, so none is claimed). On this branch's tip,
  run CI's coverage job, or the same command non-root on the Mac: `cargo llvm-cov --workspace --locked ...`, then
  `cargo llvm-cov report --locked --text` and `gates_jobs gate-20`. Write gate 20's `declared` lines in
  `.github/workflows/ci.yml`, and a §54 follow-up, from its "measured:" lines, in one commit. The cloud host could not:
  its `api` lib tests hang and fail first (step 6).
- **The deferred mutants line** for `state/fix/round2/d3-mutants-deferred.txt` (append only):
  `<date> BASE-COV-LIBS-LSA d096f65942d4d3cdb98589e01842dff00c945b39
  reports/base-cloud-20261103/deep-BASE-COV-LIBS-LSA-d096f65942d4-set.txt.md base-reds`. The shards' own lines, where
  their reports name them, are separate.
- **The D1 landing.** Never by this session: no merge, no pull request.

**For the operator:**
- L-BCL-1, the 14 bench files;
- L-BCL-6, nine rows: `runner`'s `coordinate_row_offsets` product and row width (`resolved_grid_view.rs` 232 and
  261), the five [HASH] rows of `validate::authenticate_grid_pair_v4`, and `telemetry/tests/global_absent.rs:106-107`;
- finding F-LS2-1 (D-8563), whose fix needs its own id.

## Not done

- Mutants: deferred (operator 30 Oct). The set file carries the mutants lines.
- **The bar.** The rows of L-BCL-1, L-BCL-3 and L-BCL-6 wait for the operator, and the macOS rows and every figure wait
  for the Mac's Route B on its pin (above). D-7250's narrow reading still holds for these crates (D-8568).
- **Gate 20: UNMEASURED.** CI's own command cannot reach a usable profile on this host (step 6).
- **Finding F-LS2-1 is not fixed.** It is production pricing and needs its own id (D-8563).

## Process notes

- **One slip, the same kind LS2 recorded.** One shell command began with a version probe of the interpreter CLAUDE.md
  section 2 forbids (the language is not named here). It was a stray guard in front of a `perl` substitution; nothing
  ran on it, nothing was written, and no tracked file contains it. Every other command used shell, git, cargo, perl,
  jq and node.
- **Never pushed:** the local refs `stack/lsa-1`, `stack/lsa-2`, `stack/lsa-3` stay in this checkout only. No pull
  request was opened.
- **Created by this session:**
  - worktrees `/var/tmp/lsa-tree`, `/var/tmp/lsa-tree2`, `/var/tmp/lsa-gate-tree`, `/var/tmp/lsa-ci-tree`;
  - target dirs `/var/tmp/lsa-tgt`, `/var/tmp/lsa-deep`, `/var/tmp/lsa-deep2`, `/var/tmp/lsa-gate-tgt`,
    `/var/tmp/lsa-ci-tgt`, `/var/tmp/llvm-cov-install-tgt`;
  - profiles under `/var/tmp/lsa-prof`;
  - `TMPDIR` `/dev/shm/lsa-tmp`;
  - the `nightly-2026-08-08` toolchain and `cargo-llvm-cov` 0.8.4 in this container.

  Nothing was deleted.

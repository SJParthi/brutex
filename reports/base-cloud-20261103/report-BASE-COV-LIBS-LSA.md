STATUS: IN PROGRESS
ASSEMBLED: bbacf363b5d2f3be7a2c19a6423cacfe361fdb67

# BASE-COV-LIBS-LSA: the COV-LIBS assembly (3 Nov 2026 cloud wave)

TOKEN `BASE-COV-LIBS-LSA-20261103-1`. Branch `cloud/cov-libs-lsa-20261103`.

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
  - left for the operator (L-BCL-6): the `(S + 1)(T + 1)` product (two ladders over 2^32 rungs) and the five [HASH] rows
    (a blake3 collision); finding F-LS2-1 stays a finding (D-8563).
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

## Step 4: docs (in progress)

`docs/05-decisions.md` D-8560 to D-8567 and D-8569 (`41ac3ec6`), from the shards' final drafts; `docs/04-invariants.md`
BCL-10 to BCL-15, each proof path-qualified for `.github/invariant_paths.rs`; `docs/06-limits.md` L-BCL-3 (the ignored
tests by name), L-BCL-4 (closed for `runner` too) and L-BCL-6 (the operator's rows). No commit hash is cited in docs/04
or docs/06 (`store/tests/cited_commits.rs` reads both whole). D-8568, the assembly's record, follows the measurement.
`core`'s `decision_citations` and `cost_invariants` pass with the entries in place. No id of D-8560..D-8569 headed an
entry at START (`git grep -n -E '^#+ D-856[0-9]' bd04fb29 -- docs` prints nothing).

## Step 3: Route B (running)

Pinned nightly installed as `nightly-2026-08-08`: `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13
2026-08-07)`, `llvm-cov`/`llvm-profdata` print `LLVM version 23.1.0-rust-1.99.0-nightly`. The kit's
`d3-route-b-cloud.sh` (sha256 `f032b525...`) unchanged, test binaries run as `nobody` through
`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='setpriv --reuid=65534 --regid=65534 --clear-groups'` (as CI and the Mac:
non-root), `umask 000`, `TMPDIR` on `/dev/shm`. Run 1: nine crates at `3069f0c9` (runner is unchanged by every later
commit but its own); run 2: `runner` at `d096f659`.

## Process slip (recorded)

One shell command of mine began with a version probe of the interpreter CLAUDE.md section 2 forbids (the language is not
named here). It was a stray guard in a `perl` substitution command; nothing ran on it, nothing was written to a file
and nothing tracked contains it. Every other command used shell, git, cargo, perl and node only.

## Interim figures (17:10 IST 5 Nov; the final section replaces them)

- Route B run 1 at `3069f0c9` (nine crates; `ROUTE-B DONE 2026-11-05 16:39:55 IST WITH NOT PROVED lines above`, exit 3 for
  the bench and no-code `lib.rs` files only), `core` again at `0b7aa0d6` (its citation gate needs the docs entries):
  `core` lines 7746/7774 branches 602/640; `costs` 11568/11670, 480/502; `engine` 5484/5494, 528/560; `greeks`
  1918/1921, 93/96; `indicators` 16909/16914, 1892/2078; `lake` 5678/6135, 305/364; `store` 53276/54692, 3449/3990;
  `telemetry` 9370/9397, 636/678; `vocab` 8185/8205, 1240/1298. Every uncovered row carries its shard's class but one
  (the engine distinctness guard's failing side, now (F)); no row is a GAP. Run 2 (`runner` at `d096f659`) is running.
- Gates at `62535479` on the pinned stable: `cargo fmt --all --check` rc 0; `cargo clippy -p <ten> --all-targets --locked
  -- -D warnings` rc 0; `cargo test -p <ten> --locked --no-fail-fast` as `nobody`: rc 0, 135 result lines, 4075 passed,
  0 failed, 17 ignored. The root run and gate 20 (CI's command, on this host) are running.

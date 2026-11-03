STATUS: BLOCKED OUTSIDE

# BASE-COV-LIBS-LS4: crates core, costs, engine, greeks, telemetry

TOKEN `BASE-COV-LIBS-LS4-20261103-1`. Branch `cloud/cov-libs-ls4-20261103`, cut from `origin/cloud/base-kit-libs-20261103` at `eeec369b` (START `bd04fb299d23`). Code tip (`git rev-list -1 HEAD -- . ':(exclude)reports'`): `bcbc6524bdb29351f5d211aed11e4f7b76795b7b`.

## Outcome

- The bar is **not met yet**, and nothing left is in my files. By Route B at the code tip: 190 uncovered lines and 145 uncovered branch sides over the five crates, from 294 and 193 at the first measurement (`d76b11b79ae9`, START plus one clippy fix). 229 of the rows that remain are (F)/(U) (`fu-LS4.txt.md`). The other 106 (`gaps-LS4.txt.md`) are all owned outside this shard:
  - 75 lines inside the three `#[ignore]`d tests (L-BCL-3, the operator's): `costs` `fa73061.rs` and `lot_table_tests.rs` timing runs, and `telemetry` `sink.rs`'s 2 GiB run.
  - 25 in `crates/engine/src/lib.rs` (LSA's file) that the two patches below clear. I measured both patches applied: the 25 are gone.
  - 4 in `crates/engine/src/lib.rs` that need LSA's judgment: a distinctness assertion over an always-empty level, and an observer's rejection arm.
  - 2 in `crates/telemetry/tests/global_absent.rs` that cannot be covered by design: the test asserts those lines never run.
- Two reds of START are in `crates/engine/src/lib.rs` and need LSA's patch 1. Every other red is a proved host red (root).
- No production code changed. Every hunk in a production file is inside a `#[cfg(test)]` module or is a `#[cfg(test)]` module declaration. So no route or hot path changed, and no bench was owed.

## What I did

Commits on the branch, all pushed:

| commit | what |
|---|---|
| `a805d0cc` | OWNER file |
| `d76b11b7` | `engine/src/refusal_cover_tests.rs`: `clippy -D warnings` refused attempt 4's file (`explicit_counter_loop`) |
| `abe3cb96` | `core`: `alias_cover_tests.rs`, the four lookups' own-field checks |
| `fb59f603` | `costs`: `stack_cover_tests.rs`, `schedule_cover_tests.rs` (the precise day's STT sum past `i64`) |
| `3000e7a3` | `engine/tests/join_probe_screen.rs`: `JoinProbe`'s refused pair |
| `107274a1` | `costs/tests/dhan_calculator.rs`: a run-time case with no rate card |
| `c2487d5e` | `costs`: the timing helpers behind the two ignored measurements |
| `fcc3a322` | `core/tests/cost_invariants.rs` |
| `60458948` | `core/tests/decision_citations.rs` |
| `9cb09048` | `core/tests/findings.rs` |
| `7bdf142d` | `core/tests/lint.rs`, `core/tests/graph.rs` |
| `e0790fc2` | `engine`: `adversarial.rs` fixture defect, `keep.rs` heap oracle, `column.rs` test reference, `production_callers.rs` |
| `ccb4c678` | `engine/tests/resume_readiness.rs` |
| `5a4eab93` | `telemetry`: `sink.rs` test wait, `sink_recovery_tests.rs` (two clippy items, guard to assert) |
| `bcbc6524` | `core` `findings.rs` no-git child, `engine` `production_callers.rs` use-line case |
| `6d0a05b3` | lists: `gaps-LS4.txt.md`, `fu-LS4.txt.md`, `deep-BASE-COV-LIBS-LS4-bcbc6524bdb2-set.txt.md` |

Files changed under `crates/` (23, `git diff --name-only eeec369b bcbc6524 -- crates/`): `core/src/alias.rs` (declaration only), `core/src/alias_cover_tests.rs` (new), `core/tests/{cost_invariants,decision_citations,findings,graph,lint}.rs`, `costs/src/lot_table_tests.rs`, `costs/src/schedule.rs` (declaration only), `costs/src/schedule_cover_tests.rs` (new), `costs/src/stack.rs` (declaration only), `costs/src/stack_cover_tests.rs` (new), `costs/tests/{dhan_calculator,fa73061}.rs`, `engine/src/column.rs` and `engine/src/keep.rs` (test modules only), `engine/src/refusal_cover_tests.rs`, `engine/tests/{adversarial,production_callers,resume_readiness}.rs`, `engine/tests/join_probe_screen.rs` (new), `telemetry/src/sink.rs` (test module only), `telemetry/src/sink_recovery_tests.rs`. No file outside the five crates and this folder; `lib.rs` untouched.

### Defects found and fixed (all in test code of my files)

1. **A fixture that held no bit** (`engine/tests/adversarial.rs`). `one(nth)` set its bit through `vocab::table::set_exact(..).unwrap_or(m)`. The seventh live position (7) is a `near_*` bit, which `set_exact` refuses without a tolerance, so the shape "eight bars, one bit each, all different" has always held one bar with no bit. The silent fallback hid it (§4). `one()` now sets the bit by position, as the sweep sees it. `every_one_bit_fixture_holds_one_live_bit` checks every live position and is red under the old fallback.
2. **Two checks that ran on nothing.** `core/tests/findings.rs` `every_refuted_finding_carries_its_reason`: no ledger row is REFUTED today, so its loop never ran. `core/tests/lint.rs`: no file in `OTHERS` names a float today, so the per-line float check never ran. Each is now a function of its input (`refuted_without_reason`, `floats_off_the_boundary`), checked on input that breaks the rule as well as on the real data.
3. **`clippy -D warnings` refused both of attempt 4's never-compiled files**: `refusal_cover_tests.rs` (`explicit_counter_loop`), and `sink_recovery_tests.rs` (`duration_suboptimal_units`, `manual_assert_eq`).
4. **Attempt 4's seam broke two engine tests**, inside `lib.rs`: BLOCKED OUTSIDE below.

### Test-code restructures (each the same verdict, or a stricter one)

- **Silent skips that would hide a failure now refuse by name** (§4): a crate directory named in no UTF-8 (`cost_invariants.rs`); a git that cannot start after `history_is_at` has just run it (`findings.rs`, three tests).
- **`else` arms that cannot happen are gone, without changing meaning:**
  - `split(" |").next()` (always `Some`) became `split_once` (`cost_invariants.rs`);
  - a filter operand no line could fail (`"| "` versus `"|---"`, `findings.rs`);
  - a `get_mut` over the same chunks (`column.rs`'s test-only reference);
  - three waits whose only way out was a panic below them became assertions (`sink.rs` `reaped`, and the `sink_recovery_tests.rs` FIFO guard).
- **Helpers that only walked the real tree now take a root**, so a scratch tree drives their skips: `cost_invariants.rs` and `production_callers.rs`.
- **Each history test is a function of `(repo, fixed)` answering `None` when skipped**, so a bare tree drives the skip (`findings.rs`).

## Tests added (27) and red checks

Every new or changed test was shown red under a hand change of the behaviour it pins, then restored. The commit messages name each hand change.

- `core`:
  - `alias::cover_tests::{an_index_that_points_at_a_row_naming_another_symbol_answers_nothing, a_lineage_whose_index_points_at_another_change_answers_nothing}`: red under each of the four own-field checks removed.
  - `cost_invariants::{the_bench_walks_pass_over_what_is_not_a_crates_bench, the_id_row_and_reach_readers_hold_at_their_edges}`.
  - `decision_citations::the_walk_skips_build_output_and_hidden_folders`, plus `named(..)` checked in `the_reader_resolves_entries_and_pending_names_and_nothing_else`.
  - `findings::{the_empty_root_is_asked_in_the_working_directory_unfenced, the_history_checks_skip_a_bare_tree_and_the_readers_hold_at_their_edges, where_git_cannot_run_the_history_is_skipped_by_name}`, plus a file-root case in `only_a_tree_with_no_git_dir_skips_the_history`.
  - `lint::a_float_off_the_boundary_is_named_and_one_on_its_way_is_not`.
  - `graph::the_key_and_inline_table_readers_hold_at_shapes_no_manifest_writes`.
- `costs`:
  - `stack::cover_tests::{a_calculator_day_the_page_cannot_price_is_refused_by_the_page, a_day_refused_twice_over_names_the_earlier_refusal}`.
  - `schedule::cover_tests::{a_days_stt_sum_past_i64_is_refused_by_name, a_whole_stt_day_inside_the_sum_prices_or_names_a_later_step}`: red under a saturating STT sum.
  - `dhan_calculator::a_case_with_no_rate_card_is_refused_by_every_precise_helper`: red under BSE futures falling back to the NSE card.
  - `fa73061::the_timing_helpers_pick_the_named_samples_and_call_every_operation`.
  - `lot::table_tests::the_lookup_timer_picks_the_named_samples_and_times_a_table`.
- `engine`:
  - `join_probe_screen::{a_pair_whose_union_lacks_a_frequent_subset_is_counted_and_refused, refused_and_kept_pairs_in_one_frontier_are_each_counted_once}`: red under the screen's verdict inverted.
  - `keep::tests::the_heap_oracle_refuses_a_broken_heap`.
  - `adversarial::every_one_bit_fixture_holds_one_live_bit`.
  - `production_callers::{an_item_after_the_first_gate_without_its_own_is_named, the_source_walk_reads_rust_at_every_depth_and_nothing_else}`.
  - `resume_readiness::{the_oracle_combines_only_positions_a_ladder_can_keep, the_failing_wrapper_forwards_a_flush}`, plus the three refused resumes now hand a callback that panics if reached.

The operator's extreme-case rule is applied per file tested: each new test file or test lists its cases, marks two or more COMBINED, and one OUT OF THE BOX.

The new production-path coverage:

- `core/src/alias.rs:692,701,811,820` (False sides);
- `costs/src/stack.rs:589` (STT sum overflow);
- `engine/src/lib.rs:191` (`screen_blocks`' refused pair);
- `costs/tests/dhan_calculator.rs:64-82` (a `const fn` that only ran at compile time).

## Coverage by Route B

**Pins.** `rustup toolchain install nightly-2026-08-08 --profile minimal --component llvm-tools-preview`. `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, and `llvm-cov` and `llvm-profdata` print `LLVM version 23.1.0-rust-1.99.0-nightly`. Host: x86_64-unknown-linux-gnu, so these are the builder's figures, re-measured on the Mac's pin.

**Command.** The kit's `d3-route-b-cloud.sh`, unchanged. Its MANIFEST sha256 `f032b525...` was checked.

```bash
umask 000; D3_W=<scratch> D3_DIR=<kit extract> D3_NIGHTLY=~/.rustup/toolchains/nightly-2026-08-08-x86_64-unknown-linux-gnu D3_LLVM_BIN=$D3_NIGHTLY/lib/rustlib/x86_64-unknown-linux-gnu/bin D3_CARGO=$D3_NIGHTLY/bin/cargo TMPDIR=/dev/shm/ls4-tmp GIT_CONFIG_GLOBAL=/dev/shm/ls4-gitconfig CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='setpriv --reuid=65534 --regid=65534 --clear-groups' d3-route-b-cloud.sh /var/tmp/ls4-tree /var/tmp/ls4-tgt /var/tmp/ls4-prof/bcbc6524bdb2-1-B <logs> <set> core costs engine greeks telemetry
```

**Non-root, and why.** The host runs as root, which bypasses permission bits. So the test binaries run as `nobody` through cargo's runner setting, the way CI and the Mac run them; the build and the merge stay as the script does them. The `umask` lets the `nobody` test processes, including the children `where_permission_binds` spawns, write their profiles. The git config names this root-owned checkout `safe.directory`, so git does not refuse it as "dubious ownership" for another user.

**Wall time.** 52 s (`real 0m51.847s`): build 11 s, tests about 40 s, nothing queued. One shared `CARGO_TARGET_DIR` (`/var/tmp/ls4-tgt`, coverage under `cov/`).

| crate | first measure `d76b11b79ae9` (lines / sides uncovered) | code tip `bcbc6524bdb2` | covered / counted at the tip |
|---|---|---|---|
| core | 56 / 73 | 28 / 38 | lines 7746/7774, branches 602/640, 21 files |
| costs | 171 / 22 | 102 / 22 | lines 11568/11670, branches 480/502, 26 files |
| engine | 35 / 50 | 30 / 40 | lines 5448/5478, branches 530/570, 11 files |
| greeks | 3 / 3 | 3 / 3 | lines 1918/1921, branches 93/96, 7 files |
| telemetry | 29 / 45 | 27 / 42 | lines 9370/9397, branches 636/678, 16 files |
| all five | 294 / 193 | 190 / 145 | lines 36050/36240, branches 2341/2486, 81 files |

- **Engine with LSA's patches.** With both `lib.rs` patches below applied to the measurement tree (never committed), engine's 145 tests all pass. `lib.rs` reads lines 2240/2243 and branches 126/148. What remains there is (F) apart from the four judgment rows.
- **NOT PROVED (9 files)**: the 6 `benches/` files (L-BCL-1: `test = false`, so no bench object is built), and `lib.rs` of `core`, `costs` and `greeks` (L-BCL-2: no executable line).
- **Telemetry per file, for gate 20 (LSA).** Route B, crate-scoped, non-root. Uncovered lines: `sink.rs` 19 (8 assert-message (F), 6 panic (F), 5 inside the `#[ignore]`d 2 GiB test), `lib.rs` 0, `tail.rs` 0, `record.rs` 1 (F), `level.rs` 1 (F). Uncovered sides: `sink.rs` 28, `lib.rs` 2, `tail.rs` 2, `record.rs` 0.
  - These are **not** gate 20's instrument. Gate 20 (`declared` at START: level.rs 1, lib.rs 6, record.rs 1, sink.rs 20, tail.rs 3) reads CI's `cargo llvm-cov` 0.8.4 workspace profile on stable. LSA must take its counts from that run on the assembled tree.
  - My only change to a counted telemetry line is in `sink.rs`'s test module (`reaped`: one panic line folded into an assertion's message).
- **Lists.** `gaps-LS4.txt.md`, `fu-LS4.txt.md`, and the set file `deep-BASE-COV-LIBS-LS4-bcbc6524bdb2-set.txt.md`. The set file has the coverage set (90 files whole; 81 read, 9 NOT PROVED) and 171 mutants lines, the D2 set of `f01d7a0e4...bcbc6524bdb2` in these crates, for the deferred background pass. Each list holds one fenced block whose sha256 is in its header.
- **(U) rows (2), each with its proof in `fu-LS4.txt.md`.**
  - `costs/src/stack.rs:720`: the calculator day's `Exact::sum` cannot overflow, because `page_lines` panics on any brokerage past `i128::MAX/18` before it.
  - `costs/src/stack.rs:907`: `to_rupee`'s narrowing cannot fail for any `i64`.

## Gates at the code tip (this host)

- `cargo fmt --all --check`: RC 0.
- `cargo clippy -p core -p costs -p engine -p greeks -p telemetry --all-targets --locked -- -D warnings`: RC 0.
- `cargo test -p core -p costs -p engine -p greeks -p telemetry --locked --no-fail-fast`:
  - as root: passed 1113, failed 6, ignored 3 (31 result lines);
  - as `nobody` (runner as above): passed 1117, failed 2, ignored 3.
- The 2 failing everywhere are `engine` `tests::the_sweep_counts_support_against_the_fixed_width_column` and `tests::the_walk_has_exactly_three_early_exits_and_each_is_a_budget`, fixed by LSA patch 1.
- **With both patches applied (temporarily):** clippy RC 0 and the `nobody` run passes 1119, fails 0, ignores 3.
- The package is `core`; `brutex_core` is its lib target, so the brief's `-p brutex-core` names no package and `-p core` was used.

## Host reds (proved, for the Mac)

As root, four telemetry tests `chmod` a directory and expect the host to refuse, which root bypasses. They are the base's "telemetry 4" root reds in `CONTEXT.md`:

- `sink::tests::a_coverage_file_is_read_line_by_line_and_extended_where_it_stands`
- `sink::tests::a_failed_delete_is_counted_named_and_left_for_the_next_start`
- `sink::tests::a_rename_aside_that_cannot_happen_fails_the_roll_by_name`
- `sink::tests::an_audit_stream_that_cannot_be_looked_for_is_an_error`

Proof: the same test binary run with `setpriv --reuid=65534 --regid=65534 --clear-groups <binary> --exact <test>` passes each (`1 passed`), and as root each fails. The whole `nobody` run above passes them too.

Two environment artefacts of this host, fixed in the environment and not in code:

- `where_permission_binds` children could not start while the target sat under a mode-700 directory, so the target moved to `/var/tmp`.
- `nobody` could not write a root-owned `TMPDIR` or `CARGO_TARGET_TMPDIR`. Both are now sticky and world-writable.

## BLOCKED OUTSIDE (for LSA): `crates/engine/src/lib.rs`

Both patches are proved on this host: applied in order, `rustfmt --check` is clean, `clippy -p engine --all-targets -D warnings` is clean, and `cargo test -p engine --locked` is green. `lib.rs` was restored after each run and never committed.

**Patch 1 (required; draft D-8566). It fixes the two reds.** Attempt 4 (D-7329) put an indented `#[cfg(test)]` arm inside `reserved` (line 208). Two source-shape tests cut the file at its first `#[cfg(test)]`, so their shipping region ended at line 208 and they failed (`left: 0, right: 2`). The file says (lines 2517-2518) that its tests read up to the first `#[cfg(test)]`, and a sibling test already splits on `"\n#[cfg(test)]\n"`. Patch 1 makes the two do the same, so they read exactly the region they read before attempt 4. It also corrects the stale `too_many_lines` figure: clippy counted 114 before the patch and 116 after, where the reason says 101. It cites only D-7329. sha256 of the patch text: `c4cbc10e6c8dee30d8efd6952282239aa86ad27fcd9097964e9110e463d1e45c`.

```diff
diff --git a/crates/engine/src/lib.rs b/crates/engine/src/lib.rs
index 756789fc..71baf1c1 100644
--- a/crates/engine/src/lib.rs
+++ b/crates/engine/src/lib.rs
@@ -2515,7 +2515,8 @@ const WORST_K3: usize = LIVE_POSITIONS * (LIVE_POSITIONS - 1) * (LIVE_POSITIONS
 const WORST_K4: usize = WORST_K3 * (LIVE_POSITIONS - 3) / 4;
 
 /// The manifest reader behind V-06b. Declared here, past the shipping region, because
-/// several tests below read this file only up to its first `#[cfg(test)]`.
+/// several tests below read this file only up to its first `#[cfg(test)]` line at the
+/// margin (the test-only arm inside `reserved` is indented, D-7329).
 #[cfg(test)]
 mod manifest;
 
@@ -4387,7 +4388,7 @@ mod tests {
     #[test]
     #[expect(
         clippy::too_many_lines,
-        reason = "at 101 lines, and every one of them is the accounting this \
+        reason = "at 116 lines, and every one of them is the accounting this \
                   test exists to publish: it reads its own shipping region and \
                   names each early exit together with the budget that licenses \
                   it. The length IS the enumeration -- splitting it would leave \
@@ -4395,7 +4396,7 @@ mod tests {
                   which is precisely the drift the assertion refuses. Placed on \
                   this function rather than the module so it claims no more than \
                   it needs. Safe against the scan itself: the region examined is \
-                  `src.split(\"#[cfg(test)]\").next()`, which ends far above here"
+                  `src.split_once(\"\\n#[cfg(test)]\\n\")`, which ends far above here"
     )]
     fn the_walk_has_exactly_three_early_exits_and_each_is_a_budget() {
         let src = include_str!("lib.rs");
@@ -4410,8 +4411,12 @@ mod tests {
         //
         // `return` is counted only in the SHIPPING region: the test module below
         // is full of ordinary returns and closures, and scanning it would pin a
-        // number that moves whenever a test is added.
-        let shipping = src.split("#[cfg(test)]").next().unwrap_or(src);
+        // number that moves whenever a test is added. The region ends at the
+        // first `#[cfg(test)]` line at the margin: the test-only arm inside
+        // `reserved` is indented and stands in it (D-7329).
+        let shipping = src
+            .split_once("\n#[cfg(test)]\n")
+            .map_or(src, |(code, _)| code);
         let count_exits = |text: &str| -> usize {
             text.lines()
                 .map(|l| l.split_once("//").map_or(l, |(code, _)| code))
@@ -4712,10 +4717,14 @@ mod tests {
         // `drain` iterates it, so its call reads `support(mask)` on an already
         // borrowed item and the old needle stopped seeing it.
         //
-        // Splitting on the first `#[cfg(test)]` is the same discipline the
-        // `column.rs` check below already uses, and it separates the two by what
-        // they ARE rather than by how they happen to be written.
-        let shipped = src.split("#[cfg(test)]").next().unwrap_or(src);
+        // Splitting on the first `#[cfg(test)]` line at the margin is the same
+        // discipline the `column.rs` check below already uses, and it separates
+        // the two by what they ARE rather than by how they happen to be written.
+        // At the margin, because the test-only arm inside `reserved` is indented
+        // and stands in the shipping region (D-7329).
+        let shipped = src
+            .split_once("\n#[cfg(test)]\n")
+            .map_or(src, |(code, _)| code);
         assert_eq!(
             shipped.matches(concat!("column.", "support(")).count(),
             2,
```

**Patch 2 (for the bar; apply on top of patch 1). It covers `lib.rs` test code.**

- The level pairs are walked with `zip(skip(1))`, and the probe is built with `expect`. The old `let`-`else` arms could not run, because `windows(2)` always yields two and an `assert!(is_ok)` came first.
- The exhaustive-join oracle's fixture gains `{9,10}` and `{9,11}` without `{10,11}`, so its subset check meets a union it must refuse. Oracle and ladder still agree.
- Every handed batch is pushed, then asserted non-empty. The filter that used to drop an empty batch never had one to drop.

With it, the 6 patch-2 rows in `gaps-LS4.txt.md` are measured gone. sha256 of the patch text: `1b62c7b048c4f8dfe565ea199a3c2339c8b08b475ce617132cad512b6106f925`.

```diff
diff --git a/crates/engine/src/lib.rs b/crates/engine/src/lib.rs
index 6e8512a0..d572182f 100644
--- a/crates/engine/src/lib.rs
+++ b/crates/engine/src/lib.rs
@@ -4154,15 +4154,19 @@ mod tests {
     /// survive both prunes, and `offer` / `append` behave as the set and
     /// vector they front. D-0924.
     #[test]
+    #[allow(
+        clippy::expect_used,
+        reason = "test-only: a fixture-sized frontier always reserves"
+    )]
     fn the_bench_primitives_agree_with_the_level_they_stand_for() {
         let wide: &[u32] = &[0, 1, 2, 3, 4, 5, 6, 7, 8];
         let b = bars(&[wide, wide, &[0, 1, 2], &[]]);
         let s = Ladder::with_min_hits(1).walk(&b, wide);
-        for pair in s.levels.windows(2) {
-            let [prev, next] = pair else { continue };
-            let built = primitives::JoinProbe::try_new(&prev.frequent);
-            assert!(built.is_ok(), "fixture-sized reservation");
-            let Ok(probe) = built else { continue };
+        // Each level beside the next: a pair always has both, so no arm is
+        // left for one that does not (BASE-COV-LIBS-LS4).
+        for (prev, next) in s.levels.iter().zip(s.levels.iter().skip(1)) {
+            let probe =
+                primitives::JoinProbe::try_new(&prev.frequent).expect("fixture-sized reservation");
             let (pairs, survivors) = probe.screen_every_pair();
             assert_eq!(pairs, next.generated, "k={} pairs", next.k);
             assert_eq!(
@@ -5153,8 +5157,23 @@ mod tests {
         // has somewhere to lose candidates rather than being masked by a frontier
         // small enough that every pair happens to be in one block.
         let wide: &[u32] = &[0, 1, 2, 3, 4, 5, 6, 7];
-        let b = bars(&[wide, wide, wide, &[0, 1, 2], &[3, 4, 5], &[8], &[8]]);
-        let live: Vec<u32> = (0..=8).collect();
+        // {9,10} and {9,11} are frequent and {10,11} is not, so the oracle
+        // meets a union with an infrequent subset and must refuse it, as the
+        // subset prune does (BASE-COV-LIBS-LS4).
+        let b = bars(&[
+            wide,
+            wide,
+            wide,
+            &[0, 1, 2],
+            &[3, 4, 5],
+            &[8],
+            &[8],
+            &[9, 10],
+            &[9, 10],
+            &[9, 11],
+            &[9, 11],
+        ]);
+        let live: Vec<u32> = (0..=11).collect();
         let min_hits = 2;
         let s = Ladder::with_min_hits(min_hits).walk(&b, &live);
         assert!(s.completed(), "the fixture must not breach a budget");
@@ -5294,10 +5313,7 @@ mod tests {
 
         // The cumulative counters only ever grow, which is what makes them
         // readable as progress rather than as a per-level figure.
-        for pair in reported.windows(2) {
-            let (Some(&(_, a0, p0)), Some(&(_, a1, p1))) = (pair.first(), pair.get(1)) else {
-                continue;
-            };
+        for (&(_, a0, p0), &(_, a1, p1)) in reported.iter().zip(reported.iter().skip(1)) {
             assert!(a1 >= a0, "admitted is cumulative and cannot shrink");
             assert!(p1 >= p0, "pairs walked is cumulative and cannot shrink");
         }
@@ -5615,10 +5631,12 @@ mod tests {
             let mut handed = Vec::new();
             let (next, halt, _, _) =
                 ladder.try_next_level_observing(&column, &first, 2, 0, 0, &mut |pending| {
-                    if pending > 0 {
-                        handed.push(pending);
-                    }
+                    handed.push(pending);
                 })?;
+            // Every handoff is a batch that holds something: none is empty
+            // (BASE-COV-LIBS-LS4; the filter that used to drop an empty one
+            // never had one to drop).
+            assert!(handed.iter().all(|&pending| pending > 0));
             let (tail, full_batches) = handed.split_last().ok_or("no actual support batch")?;
             assert!(
                 full_batches
```

**For LSA's judgment: 4 rows in `lib.rs` test code** (line numbers at the code tip):

1. **`lib.rs:3014`, True and False.** The fixture of the block-arithmetic test prunes every candidate: each union lacks a 2-subset. So `level.frequent` is empty, and "no pair repeats a candidate" is checked on nothing. Two possible fixes: assert the level is empty and leave distinctness of real survivors to `engine/tests/sweep_readiness_oracle.rs:159`; or widen the column so some candidates survive.
2. **`lib.rs:5906` True, and the line at 5907.** In `a_checkpoint_observer_gets_one_terminal_boundary_and_no_later_level`, the observer's rejection runs only if the walk calls it after its terminal boundary, which would be a production defect. It is the test's failure expressed as an `Err`. Two possible fixes: (F) by extension, or `assert!(!terminal_seen, ..)`.

**Gate 20 (`.github/workflows/ci.yml`) and docs/06 §54** are LSA's, from CI's workspace instrument (see Coverage). **docs entries:** the drafts below.

## Decision drafts (ids D-8566, D-8567; for LSA to write)

- **D-8566.** `engine`: the shipping region the source-shape tests of `lib.rs` read ends at the first `#[cfg(test)]` line at the margin (patch 1). Attempt 4's test-only arm in `reserved` (D-7329) is indented and stands in that region. Cutting at its first `#[cfg(test)]` text ended the region at line 208, and `the_sweep_counts_support_against_the_fixed_width_column` and `the_walk_has_exactly_three_early_exits_and_each_is_a_budget` failed. Splitting on `"\n#[cfg(test)]\n"` (as `every_level_appends_its_survivors_through_the_one_primitive` already did) reads the same region as before attempt 4. The `too_many_lines` reason's stale "101" becomes clippy's measured 116. If patch 2 is taken, the same entry records it: the `lib.rs` test code walks level pairs with `zip`, the exhaustive-join oracle meets a union with an infrequent subset, and every handed batch is asserted non-empty.
- **D-8567.** BASE-COV-LIBS shard LS4 (`core`, `costs`, `engine`, `greeks`, `telemetry`):
  - **Figures.** Route B on the pinned nightly (rustc 1.99.0-nightly 1a98b1e13, LLVM 23.1.0), crate-scoped, test binaries as a non-root user, on an x86_64 Linux builder: from 294 uncovered lines and 193 uncovered sides (`d76b11b79ae9`) to 190 and 145 (`bcbc6524bdb2`). The (F)/(U) list is `reports/base-cloud-20261103/fu-LS4.txt.md` (227 (F), 2 (U)).
  - **The bar is not met for these crates:** 75 lines inside `#[ignore]`d tests (L-BCL-3), 29 rows in `engine/src/lib.rs` test code (25 cleared by D-8566's patches, 4 for judgment), and 2 by design in `telemetry/tests/global_absent.rs`.
  - **D-7250's narrow reading does NOT end for these crates at this shard.** It ends at `Merge BASE-COV-LIBS:` only when a Route B measurement of the assembled tip shows the bar.
  - **Recorded as made:** test-code restructures in which silent skips refuse by name and impossible `else` arms are replaced by forms with none; two vacuous checks made functions of their input (`findings.rs` REFUTED, `lint.rs` floats); and the `engine/tests/adversarial.rs` fixture defect (`one(nth)` silently empty for a `near_*` position).
- **docs/04 rows (drafts).**
  - "A lookup in `core::alias` answers only a row whose own field names what was asked, whatever its index says": `alias::cover_tests::an_index_that_points_at_a_row_naming_another_symbol_answers_nothing`.
  - "A precise day's trade STT plus exercise STT past `i64` is refused by name": `schedule::cover_tests::a_days_stt_sum_past_i64_is_refused_by_name`.
  - "`JoinProbe` counts a pair the subset screen refuses as a pair and never as a survivor": `join_probe_screen::a_pair_whose_union_lacks_a_frequent_subset_is_counted_and_refused`.
  - "Every one-bit fixture of the adversarial sweep holds exactly one live bit": `adversarial::every_one_bit_fixture_holds_one_live_bit`.
- **docs/06 (drafts).**
  - L-BCL-5 gains LS4's figures above.
  - L-BCL-3 stands for the three ignored tests in these crates (`costs` `fa73061.rs` `the_dated_lookups_cost_the_same_on_every_row_measured`, `costs` `lot_table_tests.rs` `a_lookup_is_flat_in_the_table`, `telemetry` `sink.rs` `an_audit_line_survives_2_gib_of_refusals_at_full_size`). Their helpers are now checked where the gate runs.
  - `telemetry/tests/global_absent.rs:106-107` are named as uncoverable by design: the test asserts they never run.

## NEEDS THE MAC

- Route B on its pin (aarch64-apple-darwin).
- The macOS rows. None of this shard's lines is macOS-only; the (U) proofs hold on every target.
- The non-root 14-core test run.
- The ignored real-data and timing tests (L-BCL-3).
- The D1 landing.
- The line for `state/fix/round2/d3-mutants-deferred.txt` (append only), from the recipe's deferred-mutants rule: `<date> BASE-COV-LIBS-LS4 bcbc6524bdb29351f5d211aed11e4f7b76795b7b reports/base-cloud-20261103/deep-BASE-COV-LIBS-LS4-bcbc6524bdb2-set.txt.md base-reds`.

## Not done

- Mutants: deferred (operator 30 Oct). The set file carries the mutants lines.
- `global_absent.rs:106-107`: by design, for the operator or the Mac.
- The four judgment rows and both patches are LSA's.
- Gate 20 counts come from CI's instrument (LSA).
- No `cargo llvm-cov` workspace run on this host: it is gate 20's instrument, not this shard's bar, and it builds the whole workspace.
- Benches: none. No production code changed. The only engine source edits are in `column.rs` and `keep.rs` test modules.

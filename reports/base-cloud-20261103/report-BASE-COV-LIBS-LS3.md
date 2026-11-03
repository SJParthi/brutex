STATUS: IN PROGRESS

# report-BASE-COV-LIBS-LS3 (session BASE-COV-LIBS-LS3, 3 Nov 2026 cloud wave)

TOKEN `BASE-COV-LIBS-LS3-20261103-1`; branch `cloud/cov-libs-ls3-20261103`, cut from
`origin/cloud/base-kit-libs-20261103` (`eeec369b`, code START `bd04fb299d23`). Shard: crates
`lake`, `indicators`, `vocab` (SHARDS.md, COV-LIBS attempt 5). Decision ids: D-8564, D-8565.

## Kit check

Every kit file this session reads matched MANIFEST.md's sha256 (CLOUD, CONTEXT, DECISIONS,
SHARDS, the progress file whole; RULES-MAC-COV-LIBS, fix-BASE-COV-LIBS.json, the a1-after gaps,
the (F)/(U) list, the ee65c9f6fcbd set, the D3 recipe, both Route B scripts, d3-covset.mjs and
replay-minus-reports.sh by their fenced content).

## Host

Linux x86_64, 4 cores, 15 GiB memory, 29 GiB free disk, runs as root. Gates on the repository's
pinned stable `rustc 1.97.1 (8bab26f4f 2026-07-14)`; one `CARGO_TARGET_DIR` per toolchain;
`TMPDIR` on the `/dev/shm` tmpfs.

## Step 1: the shard's tests at START

`cargo test -p lake -p indicators -p vocab --lib --locked --no-fail-fast`: EXIT 0
(indicators 396 passed, lake 101 passed, vocab 70 passed; 0 failed).
`cargo test -p lake -p indicators -p vocab --locked --no-fail-fast` (every target): EXIT 0,
0 failed; 6 ignored, all real-lake tests (`real_lake.rs` 4, `real_lake_regression.rs` 1,
`refusals.rs` 1: they need `~/.brutex/lake`, L-BCL-3, the Mac's).

## Route B baseline at START (code `bd04fb299d23`, branch tip `6ba720789967`)

Pinned nightly installed here as `nightly-2026-08-08`: `rustc -V` prints exactly
`rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`; its llvm-cov and llvm-profdata print
`LLVM version 23.1.0-rust-1.99.0-nightly`. Run by the kit's `d3-route-b-cloud.sh` (D3_NIGHTLY,
D3_LLVM_BIN, D3_CARGO pointed at that toolchain; no RUSTC_BOOTSTRAP) in a detached worktree at the
tip, set file = every `.rs` of the three crates whole (71 files, the same coverage lines as the kit's
`deep-BASE-COV-LIBS-ee65c9f6fcbd-set.txt` for these crates). 15:11:17-15:18:42 IST: build 0 min,
lake tests 0 min (135 passed, 6 ignored), indicators 4 min (508 passed), vocab 1 min (174 passed).

| crate | lines | uncovered lines | branch sides | uncovered sides |
|---|---|---|---|---|
| indicators | 16498/16540 | 42 | 1915/2186 | 271 |
| lake | 4987/5623 | 636 | 274/366 | 92 |
| vocab | 7636/7741 | 105 | 1126/1286 | 160 |
| shard | 29121/29904 | 783 | 3315/3838 | 523 |

NOT READ by the route (exit 3 lines): `crates/{lake,indicators,vocab}/benches/ratio.rs` (their
`[[bench]]` is `test = false`, so `cargo test --no-run` builds none: L-BCL-1) and
`crates/lake/src/lib.rs` (module declarations only, no executable line).

First classification of the 1,306 rows (recipe v4 step 4, by a brace-matching reader of each
`assert!`/`panic!` invocation in test code, then by hand): 410 (F) candidates, 84 production GAP
rows, 812 test-code GAP rows (of them 271 in the `#[ignore]`d real-lake tests and their helpers).

## Work log

(appended as each step lands)

### Step 2: lake (commit `9f6b33a5`)

Production gaps closed by tests that drive their input: `reader.rs` a negative row-group count
(-1, -4, `i64::MIN`, and with the file count lying too), a column chunk past the end of the file
(1, 2 and 4,096 bytes past, `i64::MAX` long, and the boundary: a chunk ending exactly at the last
byte is refused as unread chunk bytes, never as past the end), a null `spot_at_bar` (first row, last
row, both, neither), the three wrong-type reads (all six wrong pairings, an empty group, and the
right read after the wrong one through the same `Columns`); `page.rs` a position past the chunk
(at the end, 1 and 2 past, `usize::MAX`, and past with nothing owed); `schema.rs` `spec` at run
time and non-timestamp logical types on the timestamp, through `detect` and directly.

Two production changes, both drafted as D-8564 below: `Columns::mismatch` now names the type the
read asked for (it named the spec's type, which for every call that reaches it is the type the file
holds: "should be DOUBLE but the file holds DOUBLE"); `page::read_header` measures the header by how
far the slice advanced, removing a `u64`-to-`usize` refusal no 64-bit target reaches.

Test code: the RFC 8878 frame walk (`zstd_v082_frames::block_types`) is proved over every
descriptor shape from `page.rs` (the frozen frames held one content-size width and always a
checksum), and `page.rs`'s inline copy of that walk now calls it; `refusals.rs`'s real-lake scan,
walk and month census are driven on synthetic files and trees (each detector shown firing on a file
made to carry its defect, so the `#[ignore]`d real-lake scan cannot pass vacuously); arms that
collected accepted cuts into a list now fail where they happen; the nested-column test's `Ok` arm,
an outcome the reader can no longer produce (it refuses the shape at open), now fails; a 32-bit
count under a signed annotation is refused by type; `synthetic.rs`'s footer rewrites use
`Option::map` for fields the writer always sets.

`cargo test -p lake --locked --no-fail-fast`: EXIT 0 (lib 112 passed, was 101; refusals 17 passed
and 1 ignored, was 13; synthetic 18; events 1; real-lake 5 ignored). `cargo clippy -p lake
--all-targets --locked -- -D warnings`: EXIT 0. `cargo fmt --all --check`: EXIT 0.

Red checks (`scratchpad red.sh`: hand change to a committed file, run the one test, restore with
`git checkout`): 18 of 18 RED, e.g. mismatch arm `Int64`->`Double`; negative-count `value` -> 0;
the chunk slice clamped to the file; a null spot refused; header length +1; the past-chunk message;
the non-timestamp arm -> `None`; `spec` dropping its name; the frame walk's 4-byte content size ->
3 and its checksum test negated; the scan's `!=` -> `>` (twice), its nesting test without the
repetition half, its version-2 count removed; the stride floor 1 -> 2; a year that is a file pushed;
the census count +2 and its existence filter removed.

Route B on lake at `9f6b33a5f5d3`: lines 5678/6135 (457 uncovered), branch sides 305/364 (59).
What is left: (F) failing arms, `schema.rs:125-126` as (U), and the `#[ignore]`d real-lake tests
(`real_lake.rs` 132 lines, `real_lake_regression.rs` 126, `refusals.rs`
`no_real_lake_file_triggers_either_defect`), which need `~/.brutex/lake`: NEEDS THE MAC.

### Step 3: indicators (commit `ee901685`)

Production changes, drafted as D-8565: the exact-minute bridge carries the previous minute's
stamp (a lookup of `index - 1` inside `index > 0` had an arm no slice reaches) and fetches the
closing minute and its masks in one lookup (`minute_masks` holds one entry per minute, so the
second lookup's arm was unreachable); `Evaluator::crossings_of` walks `CROSSINGS`, `last_side`
and `crossings_seen` together by `zip` (both arrays are `[_; CROSSINGS.len()]`) and chooses a
crossing's edge with its side (the later `Side::Unknown => continue` arm was unreachable);
the evaluator maps a VWAP refusal through `vwap::Refused::into_corrupt` (its `Corrupt` arm, as a
match in the evaluator, was reachable by no bar the evaluator accepts; the conversion is proved
for both refusals directly). Behaviour for every reachable input is unchanged.

New tests, each driving its input (white-box where the state is unreachable through the public
API, which every module's own comments call out): `anchored::tests::a_closing_minute_on_the_next_ist_day_is_refused_even_when_it_exists`,
`anchored::tests::the_resampler_gives_a_fully_punched_day_no_bar`,
`column::column_cover_tests::an_overlay_onto_a_column_whose_sources_and_rows_disagree_is_refused_whole`,
`evaluator::evaluator_cover_tests::{the_base_tags_are_distinct_and_no_base_is_zero,
an_unseeded_state_on_the_bars_own_day_claims_no_session_open,
warm_needs_the_previous_session_even_with_the_ladder_full}`,
`gap::gap_cover_tests::{a_rung_handed_the_pivot_band_is_refused_by_the_vocabulary_and_stays_unset,
a_leg_whose_length_leaves_i64_is_unavailable_and_sets_nothing, a_leg_of_no_length_is_unavailable_at_every_rung}`,
`orb::orb_cover_tests::{a_state_on_its_bars_own_day_that_is_not_live_starts_the_session_again,
a_closed_window_whose_span_leaves_i64_publishes_no_availability}`,
`session::session_cover_tests::a_state_on_its_bars_own_day_that_is_not_live_starts_the_session_again`,
`trend::trend_cover_tests::{a_period_of_zero_is_warm_before_it_has_a_value_and_speaks_for_nothing_yet,
a_ring_with_an_empty_slot_publishes_no_swing}`,
`vwap::vwap_cover_tests::{each_refusal_keeps_its_own_name_through_the_evaluator,
a_state_on_its_bars_own_day_that_is_not_live_starts_the_session_again}`,
`vwap::tests::an_impossible_state_is_refused_by_the_oracle_and_clamped_by_sigma`,
`tests/exact_minute_orb_gap_readiness.rs::a_flat_opening_range_answers_its_exact_relations_and_leaves_near_unknown`,
`tests/trend_known_readiness.rs::a_close_between_a_lower_swing_high_and_a_higher_swing_low_breaks_the_newer`
(the evaluator agrees with the independent oracle on a close that breaks both swings),
`tests/module_doc_counts.rs::the_header_parsers_read_what_they_should_and_refuse_the_rest`.

Test code: an always-in-range `get_mut` under `if let` became `.expect` (the crate's own
convention: `expect` panics inside core and leaves no region); dead oracle clauses in the
readiness tests (a range past `i64::MAX` from positive prices, a band whose base `Widths::new`
refuses, a determined leg of no length, a gap leg of no length) were asserted or removed with the
reason beside them; latched flags that only a regression could set became assertions on the spot
(the pattern this crate already uses); a short stop is now asserted to tighten on every candle
of a steady fall (it counted at least one); or-chains over break positions are read as a set with
`.any()` (the fixtures break up only as 56 and down only as 59, noted for the Mac: no fixture of
those two tests produces 57 or 58 on a bar the earlier positions miss).

`cargo test -p indicators --locked --no-fail-fast`: EXIT 0, lib 413 passed (was 396), every
integration target ok. `cargo clippy -p indicators --all-targets --locked -- -D warnings`: EXIT 0.

Red checks: 24 of 24 RED (`scratchpad red-ind.txt`): the next-day clause removed; the carried
stamp never updated; the masks looked up one minute late; the resampler's end-of-day flush
removed; the column length check made `>`; the no-base tag 0 -> 3; the `seeded` conjunct removed;
the `previous` conjunct removed; the crossing edge up -> down; `into_corrupt` collapsing record
faults; not-live resets removed in vwap, orb and session; the sigma clamp `max(0)` -> `abs()`;
the gap ladder handed the Fibonacci band regardless; an overflowing leg given a length; a flat
leg given a span; an overflowing ORB window given `i64::MAX`; a flat window given a span of one;
a zero-period average reporting zero; an incomplete ring stepping past an empty slot; the newer
swing losing to the older; a reversed range parsed; `ist_day` truncating instead of flooring.

### Step 4: vocab production (commit `783793a3`)

Production changes, drafted as D-8565: an `And`/`Or` step takes both operands in one slice
pattern (`Some([.., left, right]) = stack.get_mut(..used)`: its only refusal is the reachable
underflow, and the left operand's second lookup had an unreachable arm); `Expression::encode`
splits its `[u8; 4 + 3 * MAX_INSTRUCTIONS]` buffer once (`get_mut(4..)` could not miss);
`render_program` seeds its work stack with the root (the first push onto an empty stack could
not overflow).

New tests: `expression::cover_tests::a_child_link_back_to_its_own_node_ends_the_render_in_an_error_not_a_loop`
(the renderer's documented bound, by a direct call with corrupt links),
`implication::cover_tests::{compatibility_is_symmetric_and_refuses_exactly_the_impossible_bands,
a_pair_is_informative_exactly_when_neither_restates_the_other_and_both_can_hold}` (all 484 ordered
pairs of the pivot chain), `expression_search::invariant_tests::the_references_refuse_an_overlong_prefix_and_an_uncarried_length`;
`table::tests` now probes every missing token and puts a colliding name in the shared probe of the
two-rows test. The vocab test modules are inline in the files they test, because
`tests/no_float.rs` holds the float scan to exactly the `pub mod` files of `lib.rs`.

`cargo test -p vocab --locked --no-fail-fast`: EXIT 0 (lib 74 passed, was 70).
`cargo clippy -p vocab --all-targets --locked -- -D warnings`: EXIT 0.

Red checks: 8 of 8 RED (`scratchpad red-vocab.txt`): the operand count never dropped; the body split one byte late; the work stack seeded empty; overflow ignored with the loop ending `Ok`; a (below, above) band compatible at equal rank; the second restatement test removed; the name index probing by two; an uncarried length clamped to `u16::MAX`.

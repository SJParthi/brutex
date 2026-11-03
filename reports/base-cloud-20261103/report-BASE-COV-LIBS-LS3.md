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

### Step 5: vocab test code (commits `8275882e`, `f9355fdc`, `7ae41f8a`, `70d241e2`, `7f422402`, `4a853e80`, `6d34cf19`)

What was left in vocab after step 4 was test code: the workspace gates `tests/no_runtime_spawn.rs`
(141 rows), `tests/no_py_language.rs` (20) and `tests/workspace_is_rust.rs` (11), and one row
each in `src/expression.rs`, `tests/expression_search.rs` and `tests/no_float.rs`, three in
`tests/mask.rs`. No production line changed in this step, and no gate's verdict changed.

- `expression::cover_tests::a_child_link_back_to_its_own_node_ends_the_render_in_an_error_not_a_loop`
  also hands the renderer a negation and a join linked to each other: the stack fills within the
  3N bound and the negation's push is the one that meets its end.
- `tests/expression_search.rs` runs the enumeration at a budget of 3 as well as 50,000, so it
  pauses every few nodes, and must find exactly the same programs (at 50,000 alone it never paused).
- `tests/mask.rs`: half the anti-monotonicity trials use a bar that holds its base (no random bar
  did, so no trial was a hit), and every trial now states exactly when a wider candidate hits:
  where the base hits and the bar holds the added bit.
- `tests/no_float.rs`: the directory listing is a helper, tested on a scratch directory holding an
  upper-case extension, a file with none, a backup of a source and a document.
- `tests/workspace_is_rust.rs`: the lock's reachability walk files each package beside its own
  `Cell<bool>` seen flag (its two lookups by index could not miss, and the walk stays O(V + E) as
  its doc says); the semver order checks zip neighbours instead of matching `windows(2)`; the
  comparator probe uses `expect`; `the_lock_parser_keeps_what_it_understands_and_skips_the_rest`
  hands the parser a key before any package, a table that is not a package, a bracketed and an
  unquoted line inside a dependency list, a source line and a key after the last package.
- `tests/no_py_language.rs` (D-5250's gate): the program it runs as `git` is a parameter, `GIT` at
  every real call, so `a_host_that_cannot_run_git_walks_the_tree_and_the_git_proofs_skip_by_name`
  hands the file lister and both git-backed proofs a program path that cannot exist: the fallback
  walk still steps over `.git`, and each proof names why it skipped. The gate prints its
  absent-file line whatever the count, through `absent_line`, which
  `the_absent_file_line_counts_every_one_shows_ten_and_masks_them` feeds none and twelve, the name
  in the first. `a_listed_path_that_cannot_be_inspected_is_a_failure_not_an_absence` shows a listed
  path whose metadata fails for any reason but absence (a path through a regular file) stops the
  walk instead of shrinking it.
- `tests/no_runtime_spawn.rs` (D-5240's gate): seven tests hand the gate's own readers the input
  that takes each one's other side, with the answer stated exactly:
  `the_lexer_reads_the_rarer_spellings_as_rustc_does` (nine shebang shapes, `r##x`, `r#1`, three
  escapes `rustc` refuses, a continuation over a blank line),
  `the_cfg_and_attribute_readers_answer_at_their_edges`,
  `items_and_macro_inputs_end_where_the_model_says`,
  `the_needles_readers_answer_at_their_edges`, `a_pass_records_what_the_rarer_placements_hold`,
  `manifests_are_read_as_toml_reads_them` and
  `the_verdict_is_silent_where_the_model_has_nothing_to_refuse`.

`cargo test -p vocab --locked --no-fail-fast`: EXIT 0 (lib 74; expression 11, expression_search
8, readiness 3, implication_pointers 4, mask 8, no_float 4, no_py_language 12, no_runtime_spawn
17, stale_claims 10, table 20, workspace_is_rust 19; 0 failed). `cargo clippy -p vocab
--all-targets --locked -- -D warnings`: EXIT 0.

Red checks (`scratchpad red-vocab.log`), 40 hand changes: 36 RED, 4 green and each equivalent on
every input the code can be handed. Two of the 36 were green at first, so the test's case was
sharpened until the rule changed its answer (a literal TOML string ending in a backslash; a string
after a `members` that is no array), and the inner doc-comment shebang case now ends at `[` for the
same reason. RED: `hits` ignoring its sixth word; a pause that skips a
choice; the listing matching `rs` in one case only; the lock parser's source line and its bracket
rule; the walk's seen test inverted; the absent line showing eleven, and showing the name; the
metadata guard taking every error as absence; the fallback walk returning nothing, and not stepping
over `.git`; each skip line reworded; and 23 changes across the spawn gate's readers (a doc
comment after `#!`, `r#` with no name after it, a refused `\x` escape, the blank line of a
continuation, an empty `any()`, a trailing comma in `cfg_attr`, `extern` without an ABI, a
visibility's path, `const` with block values, a gated `mod name` with neither `;` nor body, the
type as the first token, a plain name before `::`, an import group through a variable, a later
attribute dropping the first `#[path]`, an inner attribute's naming taken as test code,
`in_module_body` ignoring a pending signature, the `\e` escape, a literal string's backslash,
`members` that is no array, a `Cargo.toml` outside `crates/`, a manifest naming a manifest, a
pasted file that is test code whole, a test file that is never scanned). Green, equivalent: the
walk's `!flag.get()` for `!flag.replace(true)` (the lock graph is acyclic, so the flag bounds the
work, not the answer); `macro_input_end` without its `extern crate` clause (such an item holds no
brace); the bare-`#[test]` exemption without `statement` (on a parameter, the pending signature
already refuses it); the `_tests.rs` rule without `!under_tests_dir` (every file under `tests/` is
already in the test set). The crossed-links render case extends a test already red-checked; no
change to the negation's overflow alone turns its answer, since the walk's bound answers `Err`
too.

## Lists

`gaps-LS3.txt.md` (every uncovered row of the three crates with its class, the per-file table and
the totals) and `fu-LS3.txt.md` (the (F) rows with their rule, the (U) rows with their proofs, the
Mac's rows with their reasons), regenerated from each Route B run by the scratchpad classifier
(a brace-matching reader of every `assert!`/`panic!` invocation in test code, the GAP rows then
read by hand, and a hand-checked override list for the (U) and Mac rows).

## Decision drafts (for LSA; ids from this session's block)

**D-8564 (lake): two refusal paths restructured so that what reaches them is what they name.**
(1) `reader::Columns::{int64, int32, double}` pass the type they ask for to `mismatch`, which now
reports `ColumnTypeMismatch { want: <asked>, got: <held> }`; it reported the layout spec's type as
`want`, which for every call that can reach the arm (a read asking for a column as the wrong type;
no file reaches it, `schema::detect` refuses a wrongly typed column at open) is the type the file
holds, so the refusal read "should be DOUBLE but the file holds DOUBLE". Proved by
`lake::reader::tests::a_column_read_as_the_wrong_type_names_the_type_asked_for_and_the_type_held`
(red under `Int64` -> `Double`). (2) `page::read_header` takes the header length as how far the
`&[u8]` advanced (`rest.len() - unread.len()`), replacing a `Cursor` position turned from `u64`
into `usize` through a refusal no 64-bit target reaches. Proved by
`lake::page::tests::a_header_length_is_exactly_the_bytes_the_parse_consumed` (red under `+ 1`).
Behaviour for every file is unchanged. Suggested docs/04 row: "a wrong-type column read names the
type asked for and the type held" beside the first test.

**D-8565 (indicators, vocab): guard arms no input can reach, restructured away; each equivalent
for every reachable input and proved by the crates' suites and a red check.** indicators: the
exact-minute bridge carries the previous minute's stamp and fetches the closing minute with its
masks in one lookup; `Evaluator::crossings_of` zips `CROSSINGS`, `last_side` and
`crossings_seen` (both `[_; CROSSINGS.len()]`) and chooses the crossing edge with its side; the
evaluator maps a VWAP refusal through `vwap::Refused::into_corrupt` (proved for both refusals by
`vwap::vwap_cover_tests::each_refusal_keeps_its_own_name_through_the_evaluator`). vocab: an
`And`/`Or` step takes both operands in one slice pattern; `Expression::encode` splits its
fixed-size buffer once; `render_program` seeds its work stack with the root. The kept-on-purpose
documenting conjuncts (`Evaluator::warmed_up`'s `previous`, `known_after`'s `seeded`, the
not-live clauses of `Orb`, `SessionState` and `Vwap`) are NOT removed: each is shown deciding on
its own by a white-box state test, so a later edit cannot delete it unseen. In vocab's own gate
tests, three test-only restructures with no verdict changed: `tests/no_py_language.rs` (D-5250)
runs `git` through a named program, `GIT`, so a test can hand it one that cannot run, and prints
its absent-file line whatever the count; `tests/workspace_is_rust.rs` files each lock package
beside its own seen flag (O(V + E), as its doc states).

## (U) items (proofs in fu-LS3.txt.md)

`crates/lake/src/schema.rs` the `i >= n` refusal of `detect` (two rows);
`crates/vocab/src/table.rs` the `None` after `index_of`'s probe loop;
`crates/indicators/src/vwap.rs` the exhaustion of `isqrt_i128_counted`'s Newton loop.
Each proof is from the code's own arithmetic and tables, with no word size, endianness or
platform in it, so each holds for aarch64-apple-darwin as CLOUD.md item 8 asks.

## NEEDS THE MAC

Route B on its pin (this host's figures are the builder's); the `#[ignore]`d real-lake tests
(`crates/lake/tests/real_lake.rs`, `real_lake_regression.rs`, and
`refusals.rs::no_real_lake_file_triggers_either_defect`: they need `~/.brutex/lake`, L-BCL-3;
the scan's helpers are covered here on synthetic files, so only those bodies wait); the three
`benches/ratio.rs` (`test = false`, L-BCL-1; gate 8 and every p99 are the Mac's); macOS rows
(none of these crates has a `cfg(target_os)` line); non-root 14-core runs; the D1 landing.

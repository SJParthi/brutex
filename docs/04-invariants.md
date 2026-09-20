# 04 — Invariants

Every row names the test that proves it. **An invariant with no test named
beside it is deleted from this file, not shipped.** A consistency check in CI
fails if a row's test does not exist.

Status: `✓` proven · `◐` proven where it has been run, and where that is is
named · `○` test written, awaiting the crate · `—` not yet reachable (the crate
does not exist) · `✗` **the named test does not exist in any file, and the crate
it names is tracked today** — the row is a gap, not a proof.

**`✗` was added by D-0045 and it is the point of that entry.** Seven rows wore
`—` while naming a test that exists in zero files, in three crates that are
tracked and compiled today. `—` means "the crate does not exist", so those rows
read as *not yet reachable* when the truth was *nobody wrote it*. A row pointing
at a phantom test is worse than an empty row, because it looks proven. Every
such row now carries `✗`, keeps the test name it would need, and says in its own
cell that the name is a plan rather than a proof. The names are deliberately
**left in the backticks** so CI gate 10 goes on reporting them by name every
run; removing the token would make the gate green by blinding it.

**Six of the ten rows gate 10 reported now name a test that runs. Four still did
not, and a fifth joined them when A-20's absolute went stale.** What changed and
what did not is in *The ten phantom rows, one at a time* near the end of this
file, and *The last five, and the gate that stopped being red* after it.

**Of those five, two were WRITABLE and were written; three were not, and are
allowlisted by name.** A-20 (D-0060) and X-02 (D-0061) have tests. P-03, X-01
and X-13 (D-0062) sit in CI gate 10's `allow_pending` with the reason beside
each, because in all three cases the code the row describes does not exist —
there is no trading calendar, no run-identity function, no cross-vendor bar
comparison — so a test would assert an absence and be deleted the day the thing
arrives. **That allowlist is the third mechanism the gate offers and it is
honest only while the subject is genuinely absent.** It is not a way to go
green: a row is still never deleted, still never weakened, and the entry states
what is missing and what closes it. Gate 10 exits 0 now, and X-10 is `◐` rather
than `✓` for exactly that reason — a tick bought by an allowlist is not a tick
earned.

**X-07 and X-08 were narrowed by D-0036, not weakened.** X-07 sat at `—`, which
this legend defines as "the crate does not exist" — untrue once `crates/pull`
existed, and `CLAUDE.md` §9's mutation bullet had simply not been run. It is now
`◐` and names where it *has* been run: `crates/pull` only. `crates/core`,
`crates/store` and `crates/api` have never been measured, and no row here claims
otherwise. X-08 claimed CI gate 1c proved "no literal credential path"; the gate
matches only a slash-joined path with a well-known environment segment, which
was demonstrated by running its exact pattern over a file hardcoding two real
segments as bare constants. The row now says what the gate does, gate 1d covers
the crate where the gap matters, and `docs/06-limits.md` §18 records the rest.

---

## Store

| # | Must hold | Proven by | |
|---|---|---|---|
| S-01 | `size_of::<Bar>() == 56` and `align_of::<Bar>() == 8` | `const _: () = assert!(…)` — a compile error, not a test | — |
| S-02 | `read(i)` returns the bytes `write(i, b)` wrote, for every *i* | `store::roundtrip::every_committed_index_returns_the_record_that_was_written` · `store::roundtrip::the_open_interest_sentinel_survives_the_disk_and_is_not_a_zero` — **every** index of a 160-record file, across three appends, two block boundaries and a close-and-reopen, asserted twice at each index: the record `read_record` returns, and the 56 raw bytes the file holds at `Layout::offset_of(i)`. The second is the half a codec test cannot make. This row named a `proptest` module for which this workspace has no dependency; S-18's `decode(image(r)) == r` over 4,276 records is still the codec's proof and was never the file's | ✓ |
| S-03 | A reader never observes a record beyond `n_valid` | `store::fault::commit_counter_publishes_last` — the module was written `store::loom::`, and there is **no `loom` module and no `loom` dependency**; the test is a plain `#[test]` walking all 65 commit prefixes | ✓ |
| S-04 | A crash between data write and counter publish loses the tail and corrupts nothing | `store::fault::kill_between_write_and_commit` | — |
| S-05 | A full disk during append returns `Err`, never a signal | **PROVEN AT THE CLASSIFIER, NOT AT THE KERNEL.** `store::file::a_full_disk_mid_write_is_returned_and_never_signalled` · `store::file::every_classified_kind_gets_its_own_name` — a scripted host returns `ErrorKind::StorageFull` part way through a write and the loop hands back `StoreError::DiskFull` as a **value**, which is the whole argument for banning a writable mapping. That loop is the only write path `append` has. **UNVERIFIED: that a real full disk produces that kind on this store's write.** No test in this repository fills a filesystem, and `crates/store/tests/write.rs` names this and the read-only mount as the two conditions it will not fake. The row named a `fault` module test that exists in no file | ◐ |
| S-06 | A flipped bit in any block is detected on the next read of that block — **of a block that carries a checksum, and this build writes none** | **PROVEN AS AN ALGORITHM, NOT AS A PROPERTY OF ANY FILE ON DISK.** `store::fault::bitflip_detected` walks every bit of every byte of a block and `block::verify` names each one — but it builds its header with `Header::genesis(7, 60, FLAG_CHECKSUMS)`, the flag **set in memory**. `store::file::initialise` passes `Header::genesis(symbol_id, timeframe_secs, 0)` — flag **clear** — so `FLAG_CHECKSUMS` is set nowhere in production, the `FileKind::Checksums` sidecar is never created, and `block::seal`/`block::verify` have no production caller. Every file this build has written is therefore **uncovered**: a lost write to the record extent is undetectable, because an all-zero record is a legal flat bar whose open interest is a real zero. `crates/store/src/file.rs` §"This build writes files with no block checksums, on purpose" discloses exactly this and says what closing it needs — a sidecar entry naming the record count it covers, which is a `docs/02-store-format.md` change with a decision entry behind it. What **is** true of a real file today is S-06b | ◐ |
| S-06b | A verification request against a file this build wrote is **refused, not answered** — the flag is clear, so `block::verify` returns `ChecksumsAbsent` rather than reporting a healthy file | `store::format::FormatError::ChecksumsAbsent`, reached through `block::verify`; `docs/02-store-format.md` §6 provides for the state | ✓ |
| S-07 | A file whose length does not divide by the stride truncates to the last whole record and logs | `store::fault::ragged_tail_truncates_loudly` | — |
| S-08 | `i64::MIN` in `open_interest` is never confused with `0`, and round-trips as null through the record image | `store::unit::oi_sentinel_distinct` — the module was written `store::proptest::`, and **no `proptest` dependency exists**; it is a plain `#[test]`, and it proves the *distinctness* half only. The *round-trip* half is `store::unit::decoding_the_image_returns_the_record_byte_for_byte` (S-18), which walks `i64::MIN` in every field | ✓ |
| S-09 | Opening a file with an unknown `format_version` refuses; it never guesses | `store::unit::unknown_version_refuses` | — |
| S-10 | Two concurrent writers on one file are refused by the advisory lock | `store::write::a_second_writer_is_refused_while_the_month_is_held` — the test **exists and passes**, and this row named an `integration` module that does not exist while claiming no lock was exercised anywhere. A second `BarFile` on a held month is `StoreError::Locked` naming the lock file, the month opens again once the first is dropped, and the lock file is not deleted. Two open descriptions in **one process**; a second *process* is exercised nowhere here, and `crates/store/src/file.rs` lists what an advisory lock does not protect against | ✓ |
| S-11 | The checksum reproduces a hardcoded value at every length a wide kernel can break on — 0, 1, 8, 15, 16, 56, 60, 64, 4087, 4088, 4089 bytes, and the all-zero and all-ones block | `store::unit::the_crc_reproduces_a_hardcoded_value_at_every_length_that_can_break` | ✓ |
| S-12 | The shipped checksum kernel agrees with an independent bit-by-bit reference at every length across a stride boundary, on every target | `store::unit::the_fast_kernel_agrees_with_a_bit_by_bit_reference_on_every_length` | ✓ |
| S-13 | The header slot's covered domain is exactly bytes `0..56 ‖ 60..64`; filling the four-byte hole is a different number and is refused as such | `store::unit::the_covered_domain_is_the_slot_minus_its_checksum` | ✓ |
| S-14 | Splitting the checksum input at any point gives the answer for the whole | `store::unit::splitting_the_input_anywhere_gives_the_same_checksum` | ✓ |
| S-15 | The lookup table is the polynomial: lane 0 is one folded byte, and lane *k* is lane *k−1* advanced by one zero byte | `store::crc::the_table_is_the_polynomial_lane_by_lane` | ✓ |
| S-16 | The 56 bytes one known record encodes to are pinned as a literal array. A body replaced with zeros, a moved offset or a flipped byte order each fail | `store::unit::the_record_image_is_the_pinned_bytes_of_a_known_bar` | ✓ |
| S-17 | The image is little-endian and each of the seven fields owns its own eight bytes; all 56 (field, byte) placements are asserted against all 56 image bytes | `store::unit::the_image_is_little_endian_and_each_field_owns_its_own_offset` | ✓ |
| S-18 | `decode(image(r)) == r` exactly, over every 64-bit boundary in every field — `i64::MIN`, `i64::MAX`, zero, negatives — and the encoder is injective across the sampled 4,276 records | `store::unit::decoding_the_image_returns_the_record_byte_for_byte` | ✓ |
| S-19 | A buffer shorter than 56 bytes is refused by length and never completed with invented zeros | `store::unit::a_short_record_is_refused_and_never_completed_with_zeros` | ✓ |
| S-20 | `BLOCK_LEN == RECORD_STRIDE × RECORDS_PER_BLOCK == 56 × 73 == 4088`, and the last record of a block ends exactly on the block boundary — so no record straddles | `store::geometry::no_record_straddles_a_block` (5,000 indices), plus **three** compile-time pins in `store::format` that can actually fail: `BLOCK_LEN == 4088`, `RECORDS_PER_BLOCK == 73`, `BLOCK_LEN == 56 * 73`. **The closed-form assertion the source comment points at cannot fail and proves nothing** — see below | ✓ |
| S-21 | Decoding a record reads exactly 56 bytes: a 1 MiB buffer of `0xFF` past the record gives the same answer as the bare 56 bytes. Behavioural, not a timing measurement | `store::unit::decoding_reads_exactly_fifty_six_bytes_however_long_the_buffer_is` | ✓ |
| S-22 | The two doors into a month agree by construction: `open_or_create` and `open_existing` both reach the four checks through `BarFile::validated`, so a month refused by one is refused by the other with the **same** `StoreError` — asserted for a symbol mismatch and for a timeframe mismatch, as values and not by shape | `store::write::the_reader_door_opens_what_the_writer_wrote_and_refuses_exactly_what_it_refuses` | ✓ |
| S-23 | **All six** `telemetry::emit` sites in this crate reach a file, each driven through the production call that owns it — `Header::read_region` for the unreadable region and the walk-back, `Header::commit` for the refused commit, `block::verify` for both block events, `BarFile::append` for the committed batch — and **no other line is written**: the six drives produce exactly six records, so an emit added to a path documented as silent (the ordinary header read, the commit that succeeds, the block that verifies) fails the same test that a deleted emit does. The target, the sentence and the **level** of each are asserted, the last because each site's doc comment argues for the level it chose and `store.append` at `Debug` is what keeps a backfill from writing a line per bar. Not one of these was proven before: each could have been deleted outright and every gate stayed green. The events are **never constructed by the test** — the mistake that made a set of `pull.member` tests worthless was fabricating an `Event` with a production target on a locally-opened `Sink`, which proves the sink works and nothing about the emit | `store::emits::every_emit_in_this_crate_reaches_the_log_through_its_production_call` | ✓ |
| S-24 | **The store can list what it holds.** `store::catalog::walk` returns every spot instrument-month under a root, read off the tree rather than named by a caller. Before it, `crates/store` held **zero** `read_dir` calls and nothing could ask the store what months existed — which is why `cli sweep-stored` takes an explicit vendor, underlying, rung, year and month, and why no caller could sweep more than one instrument-month per invocation. Proved against the operator's real backup: 18 of 18 instrument-months found, both feeds, all nine rungs | `store::catalog::one_spot_month_is_found_with_its_segments_intact` | ✓ |
| S-25 | **Every file the walk sees is accounted for, and the census proves it.** Seven buckets — spot, contract, other-kind, unknown-vendor, unknown-rung, malformed-month, wrong-depth — and `Census::reconciles` asserts the parts sum to `seen`. A walk that silently dropped a file would be the shortfall §4 bans; measured on a fixture holding **every** refusal at once, because a census that reconciles on a clean tree proves nothing about the arithmetic | `store::catalog::the_census_reconciles_over_a_store_holding_every_refusal` | ✓ |
| S-27 | **A greeks record's packed moneyness survives a strike below the money.** The provenance word carries the signed step count in its upper 32 bits; reading it back without the sign extension yields `4,294,967,294` where `-2` was meant — large enough to look like corruption, small enough to look like a far strike, and present on **every in-the-money row**. Asserted across nine step values including both `i16` bounds, plus a `-1` that sets every upper bit against the three byte-wide fields beneath it | `store::geometry::a_below_the_money_strike_survives_the_provenance_packing` | ✓ |
| S-28 | **A non-finite derivative never reaches the disk.** `Overlay::is_sane` answers `true` because a spot and a volatility constrain nothing about one another and an all-zero row is a legal reading. A greeks row is different: a `NaN` delta is not a reading at all, and it looks exactly like a real one until it is multiplied by something. Refused at the write boundary, across all seven float fields | `store::geometry::a_non_finite_derivative_is_refused_at_the_write_boundary` | ✓ |
| S-29 | **Three geometries are told apart by magic, stride AND version, pairwise.** `.bin` is 56 bytes at version 2, `.ovl` is 24 at version 9, `.grk` is 80 at version 8. A shared version makes resolution pick whichever row is first; a shared stride makes two geometries indistinguishable to a reader that resolved correctly. Asserted as a property over the whole of `Layout::KNOWN` rather than as a pair, so a fourth row is checked against all three. **All three are IN `KNOWN`** — excluding a sidecar was tried and moves the problem: `Header::decode_parts` resolves a version while decoding, before any caller names a table, so a sidecar outside the list is `UnknownVersion` at its own first byte. What separates them is `file::table_of`, chosen by file kind | `store::unit::the_constants_are_the_current_versions_layout` · `store::geometry::three_geometries_are_told_apart_by_magic_stride_and_version` | ✓ |
| S-26 | **A month the renderer writes always parses back.** `YearMonth`'s `Display` is `{:04}-{:02}` and `catalog::parse_month` is its inverse, checked over a decade — 120 months, both halves — so a change to either fails the build rather than leaving a store that cannot be listed. Width is checked before value: `2026-8` is refused, because accepting it would let a hand-made directory pass as one the writer produced | `store::catalog::every_month_the_renderer_writes_parses_back` | ✓ |

## The batch sweep — `cli sweep-all`, every stored month in one run

| # | Must hold | Proven by | |
|---|---|---|---|
| BA-01 | **Every stored instrument-month at a feed and rung is swept by one command.** `cli sweep-all` enumerates through `store::catalog` and sweeps each, where `sweep-stored` took exactly one month and had no alternative — nothing could list the store. At `docs/07-plan.md` §4's ~54,000 instrument-months that is one invocation rather than 54,000. Proved on the operator's real backup: 2,624 bars swept, ladder to k=4, 29 combinations kept | `cli::batch::only_the_named_feed_and_rung_are_swept` | ✓ |
| BA-02 | **A month that cannot be loaded is named and the run continues.** One unreadable file must not abandon the other 53,999 — the same reasoning `store::catalog` applies to an unreadable subdirectory. The refusal names the instrument, is counted, and the tally still reconciles | `cli::batch::a_month_that_cannot_be_loaded_is_named_and_the_run_continues` | ✓ |
| BA-03 | **Offered equals swept plus refused, and a shortfall is announced in the report itself.** A run reporting "12 swept" of 54,000 offered has lost 53,988 months and would otherwise read as a success | `cli::batch::the_tally_reconciles_only_when_every_month_is_accounted_for` | ✓ |
| BA-04 | **A ceiling breach is reported as a floor on depth, never as a depth.** §6 says depth is decided by extinction; a month that stopped on the candidate ceiling did not get there, and printing its `k` as an answer would state a depth the search never reached | `cli::batch::a_ceiling_breach_is_reported_as_a_floor_and_not_as_a_depth` | ✓ |
| BA-05 | **Nothing is logged from inside the sweep, and progress is reported between months.** Gate 17 forbids any `telemetry::` reference in the crates holding the mask, the vocabulary and the sweep, because the ladder evaluates billions of times. Per-instrument-month granularity is outside that loop and therefore free | gate 17, and `cli` declares no `telemetry` dependency | ✓ |

## Vocabulary and indicators

| # | Must hold | Proven by | |
|---|---|---|---|
| V-01 | Condition bit indices are stable across releases | `vocab::table::the_table_is_a_contiguous_run_of_indices`, `vocab::table::a_tombstone_keeps_its_index_and_always_evaluates_false` | Previously named a golden `bit_table_frozen` test that existed in no file. Gate 10 skipped the row while `crates/vocab` was not a workspace member and went red the moment it became one — D-0080. The old name is deliberately not written in `<crate>::<module>::<name>` form here, because gate 10 reads that shape as a claim and cannot tell a citation from a history note. **And the sentence used to demonstrate the trap by falling into it**, spelling the placeholder in exactly the shape it warns about, so gate 10 counted this row as naming a crate called `crate`. The angle brackets are what make it a placeholder rather than a claim: gate 10 matches `` `[a-z_]+::[a-z_]+::[a-z_0-9]+` `` and `<` is not in that class. |
| V-02 | At bar *i* the evaluator reads no bar `> i` | `indicators::barrier::no_lookahead` (index-guarded accessor) | — |
| V-03 | Bits `0..=(i)` are identical whether bars `i+1..` are absent, mutated, or extreme | `indicators::proptest::suffix_independence` | — |
| V-04 | Time-of-day and VWAP bits are cleared on a daily timeframe | `indicators::unit::daily_mask_clears` | — |
| V-05 | The fast evaluator agrees with a naive reference on random input | `indicators::proptest::differential_vs_naive` | — |
| V-06 | Bits are computed exactly once per slice, never re-derived per candidate — proved **structurally** rather than by counting calls: `Ladder::walk` takes `&[ConditionMask]`, already computed, and `crates/engine` does not depend on `crates/indicators` at all, so no expression in the sweep can compute a condition bit. A counting spy would prove the current code does not recompute; this proves it CANNOT | `engine::tests::the_sweep_cannot_compute_a_condition_bit` | ✓ |

## Sweep

| # | Must hold | Proven by | |
|---|---|---|---|
| E-01 | `(bits & mask) == mask` is anti-monotone over mask supersets | `engine::tests::support_never_increases_as_bits_are_added` | ✓ |
| E-02 | The Apriori kept-set equals the brute-force kept-set, **exactly** — 12,288 sweeps over every assignment of six bars drawn from four bit patterns at three thresholds, compared as sets so a shortfall is a missed combination and a surplus is an invented one | `engine::tests::the_apriori_kept_set_equals_the_brute_force_kept_set` | ✓ |
| E-03 | Emission order does not depend on hash iteration order, so the ranked list is bit-identical and not merely set-equal. **Narrowed:** this proves the order is DETERMINISTIC, not that it matches an external reference enumeration — there is no reference implementation to compare against, and inventing one would compare this code to itself | `engine::tests::the_order_of_the_output_does_not_depend_on_hash_iteration_order` | ✓ |
| E-04 | **No depth parameter exists on any public sweep entry point.** `Ladder` carries four private fields — `min_hits`, `ceiling`, `pair_budget`, `support_lanes` — and no depth, public or private; `k` exists only as a loop counter `walk` computes for itself. **The stated proof used to be `size_of::<Ladder>() == size_of::<u64>()`, and that arithmetic was false**. The named test now derives 32 bytes from the four actual field types and pairs that size guard with behavioural proof that one and four support lanes return the exact same sweep. Size is only a change detector; the property is that no field can choose depth or answer. D-0138 is why this matters rather than being pedantry: the ceiling was in fact capping depth, halting at k=9 with the pair budget 99.95% unused | `engine::tests::the_type_carries_no_depth_field`; `engine::tests::a_support_lane_bound_changes_only_scheduling` | ✓ |
| E-05 | The ladder terminates when a level produces no frequent candidate, and the empty level is recorded rather than hidden | `engine::tests::depth_is_reached_by_extinction_and_not_by_a_caller` | ✓ |
| E-06 | The prefix join reaches every k≥2 candidate through exactly one admissible pair, so no duplicate candidate is evaluated and no dedup set is required; k=1 still counts and rejects repeated offered positions explicitly | `engine::tests::one_k_set_is_evaluated_once_however_many_pairs_produce_it`; `engine::tests::the_join_emits_exactly_one_candidate_per_pair`; `engine::tests::a_duplicated_position_is_offered_once` | ✓ |
| E-07 | A rerun with identical inputs produces byte-identical output — two independent walks rendered to text and compared as text, which is what §3 rule 5's "byte for byte" means for a result a file will hold | `engine::tests::a_rerun_with_identical_inputs_produces_an_identical_sweep` | ✓ |
| E-08 | Peak memory stays under the declared budget at the declared candidate count | **UNMEASURED** — see `docs/06-limits.md`. No budget has been declared and no process-memory measurement exists, so there is nothing to prove yet; the row is kept rather than deleted because the invariant is real | — |

## Complexity — gate 8

| # | Must hold | Proven by | |
|---|---|---|---|
| C-01 | Reading the header costs the same at 1×, 10× and 100× the region offered | `store::bench::header_read_is_flat` | ✓ |
| C-02 | Per-candidate evaluation cost on the **live** `Column::support` path does not grow with what the candidate requires. The fixed-six-word row fold measured k=1 → k=4 at 0.971× and k=1 → k=8 at 0.996×; C-E-09 extends the same live method to every one of the 384 representable positions at 0.942×. The former vertical path was Θ(k) and measured 7.307× at k=8; C-E-02b retains that failed state and the corrective proof rather than describing it as current | `engine::bench::support_costs_the_same_per_bar_at_every_depth`; `engine::bench::live_support_is_flat_across_the_entire_mask_width` | ✓ |
| C-03 | Duplicate-rejection cost does not grow with the number of already offered k=1 positions | **SOURCE CORRECTED; CURRENT MEASUREMENT PENDING.** `C-E-10` now drives the exact production shape: a pre-sized `HashSet<u32>` and repeated `insert` of one existing position at 1,000 / 10,000 / 100,000 accepted positions. The former hit/miss numbers came from a standalone mask `contains` benchmark and are not evidence for this corrected row. Any eventual ratios are empirical expected-cost observations, not a worst-case hash-table guarantee. The injective prefix join proves no duplicate can be emitted at k≥2, so its former candidate `seen` set remains absent | `engine::bench::duplicate_rejection_costs_the_same_however_much_is_seen` after it is rerun · `engine::tests::one_k_set_is_evaluated_once_however_many_pairs_produce_it` for k≥2 injectivity | — |
| C-04 | A result append that fits the level's existing reservation is flat from 1× to 100× results held | **SOURCE CORRECTED; CURRENT MEASUREMENT PENDING.** `C-E-11` now uses production's exact `Itemset` and pre-sizes the vector before timing. The former 0.553×–0.607× numbers measured an unreserved `Vec<ConditionMask>` even though current `next_level` reserves a capped previous-frontier heuristic; they are historical and do not prove this corrected row. Outgrowing that heuristic still has only `Vec`'s amortised O(1) bound and an individual growth push may move every held item | `engine::bench::result_append_costs_the_same_however_many_are_held` after it is rerun | — |
| C-07 | Sealing one block costs the same at 1×, 10× and 100× the file's record count | `store::bench::block_seal_is_flat` | ✓ |
| C-08 | The block checksum beats the bit-by-bit kernel it replaced by at least 3×, measured in the same process | `store::bench::checksum_beats_the_bit_loop` | ✓ |
| C-28 | **Bar lookup — the FIRST operation `CLAUDE.md` §3 rule 4 names — costs the same whatever the file holds.** `BarFile::read_record` was documented "in O(1)" and, until this row, **nothing in the workspace measured it**; an independent O(1) audit of all thirteen crates found the gap. Both ends are read at 1×, 10× and 100× the record count — index 0 and the LAST committed index — because a scan that walked to the record would be flat in the first and linear in the second. Measured: 0.573×/0.567× at index 0, 0.992×/0.989× at the last, and **0.915× reading the last record of a 100,000-record file against the first of the same file**, which is the row a scan breaks first | `store::bench::record_read_is_flat_in_the_file` | ✓ |
| C-29 | One record read costs a bounded multiple of the **address arithmetic** it is built on — `Layout::offset_of`, one multiply and one add, no syscall. **This is the row C-28 cannot replace**: C-28 divides one read by another, so a UNIFORM slowdown cancels, and an audit measured a mask operation 174× slower passing its crate's ratio rows at 0.98×–1.00×. Measured 199.721 / 191.483 / 205.049 floors, a 1.07× spread — tighter than expected for a filesystem path, because the page is resident by the second trial. Budget **800**, sized on the worst observed with 4× left over; a first draft read 4,000 on the assumption a syscall would be noisy, and measuring showed a budget with 19× headroom is not a bound | `store::bench::record_read_stays_within_its_budget` | ✓ |
| C-27 | One dashboard render costs a bounded multiple of the per-render **floor** — one integer written into a `String`, the smallest unit of work any page here is built from. **This is the row C-14 and C-15 cannot replace**: they bound the per-row and marginal figures by dividing one render by another, so a UNIFORM slowdown cancels — and an audit measured a mask operation 174× slower passing its crate's ratio rows at 0.98×–1.00×. Measured 529.464 / 493.599 / 528.448 floors, a 1.07× spread. Budget **2,100**. A page at 50,000 rows costs about five hundred single-number writes; this row exists so that all three api cost figures rising together is still visible | `api::bench::the_dashboard_stays_within_its_budget` | ✓ |
| C-09 | Decoding one vendor row costs the same whether a field is 28 bytes or 4 MiB | `core::bench::decode_is_flat_in_field_width` | ✓ |
| C-10 | An over-wide vendor field is **refused**, not merely decoded quickly | `core::bench::an_over_wide_row_is_refused` | ✓ |
| C-09b | One vendor-row decode costs a bounded multiple of the per-row **floor** — the cheapest possible touch of the same eleven fields, eleven `len` reads and a sum. **This is the row a ratio cannot replace**: C-09 divides one decode cost by another, so a UNIFORM slowdown cancels in the quotient, and an audit measured exactly that elsewhere — a mask operation 174x slower passed its crate's ratio rows at 0.98x–1.00x. Measured over four consecutive runs: 33.823, 35.329, 35.972, 35.899 floors, budget **110**, sized on the worst observed with 3.06x left for a different microarchitecture | `core::bench::decode_stays_within_its_budget` | ✓ |
| C-11 | Reading the census beats re-deriving it from the entries by at least 100×, measured in one process | `pull::bench::census_beats_the_scan_it_replaces` | ✓ |
| C-12 | One entry lookup — hit or miss — costs the same at 1×, 10× and 100× the census, measured on the map a **loaded** manifest holds | `pull::bench::entry_lookup_is_flat` | ✓ |
| C-13 | Appending a month to a loaded census costs the same at 1×, 8× and 32× the census — measured at the counts `7·2^k` where a table reserved to exactly the census has **no free slot at all**, not only at round numbers | `pull::bench::append_after_load_is_flat` | ✓ |
| C-26 | One manifest entry lookup costs a bounded multiple of the per-lookup **floor** — one entry-count read and an add on the same manifest, no hash and no probe. **This is the row C-12 cannot replace**: it divides one lookup by another, so a UNIFORM slowdown cancels, and an audit measured a mask operation 174× slower passing its crate's ratio rows at 0.98×–1.00×. Measured 57.167 / 57.161 / 58.440 floors — a **1.02× spread, the tightest in the workspace**, because both legs read the same resident struct and neither allocates. Budget **240**. A breach here is a rehash at an exact load factor — the defect recorded as a 2.4 ms stall at 50,000 entries, which no quotient can see | `pull::bench::entry_lookup_stays_within_its_budget` | ✓ |
| C-14 | Rendering one instruments page costs the same at 2,787 and at 50,000 instruments — for **every** one of the six sort columns, both directions, all four universe pills, the escape hatch and a clamped deep page | `api::bench::every_order_and_pill_is_flat`, `api::bench::the_hatch_and_the_last_page_are_flat` | ✓ |
| C-15 | One more instrument in the universe costs a request **at most 1 ns**, measured as the slope between two sizes that draw the same number of rows | `api::bench::gate`, the `C-15 … marginal` lines | ✓ |
| C-16 | The dashboard costs the same at 2 and at 50,000 instruments — it draws no rows, so its raw ratio is the whole statement | `api::bench::the_dashboard_is_flat` | ✓ |
| C-17 | The page still **draws** its rows: cost per rendered row does not grow with the universe, so flatness cannot be bought by rendering less | `api::bench::gate`, the `C-17 … per row` and `… rows` lines | ✓ |

C-14 through C-17 exist because `docs/07-o1-architecture.md` layer 12 was marked
BUILT with the words *"a fixed row cap with paging. NEVER O(universe)"* and only
the **rendered rows** were capped. An audit measured 3.569 ms at 2,787
instruments and 124.916 ms at 50,000 **rendering exactly 200 rows both times** —
62.5 ns per instrument per request, on a page whose output never changed shape.
The row cap is what hid it. After D-0042: 147,987 ns → 151,618 ns, ratio 1.02×,
marginal 0–259 ps per instrument.

**Re-measured independently for D-0045**, by running `cargo bench -p api` on the
same machine on 2026-08-07 — exit **0**, "all ratios within the ceiling". Across
all six sort columns × four pills, plus the escape hatch and the clamped deep
page: **C-14** ratio 2,787 → 50,000 spans **0.929× – 1.088×**; **C-15** marginal
**0 – 259 ps** per instrument per request against the asserted 1,000 ps ceiling;
**C-16** dashboard **1.052×** at 2 → 50,000; **C-17** cost per rendered row
**0.202× – 0.404×**, with the row counts printed and checked (200 drawn at
n = 50,000). The absolute page figure is ~137–166 µs at both sizes. The audit's
"before" pair (3.569 ms and 124.916 ms) was taken on a **debug** server and this
is a release bench, so only the *shape* is comparable between them — the ratio
is, and the ratio is the claim.

C-15 is the row that matters most, because it is the audit's own number and it
needs no baseline: a slope measured between two sizes that draw the same 200
rows is universe size and nothing else. It went from 85,400 ps to 0–259 ps.

**It came back, and C-15 is what caught it.** On 2026-08-12 `cargo bench -p api`
exited **1** with **thirty C-15 breaches out of thirty lines, 1,433 – 2,100 ps**.
The rows, the orders, the pills and the counts were all still flat — the slope
was in the NOTES the page draws beside them, whose text grows with the universe
and which the renderer re-read in full on every request. Fixed by D-0130 and
re-measured on the same machine, exit **0**: **C-15 0 – 280 ps**, **C-14
0.974× – 1.084×**, **C-16 dashboard 0.974×**, absolute page ~157 µs at 2,787.

**Read the C-16 row of that regression before trusting a ratio ceiling.** While
C-15 was breaching on all thirty lines, C-16 stayed GREEN at 1.954× against its
3.0× ceiling — over a dashboard that had gone from 87.1 µs to 170.2 µs and is
now 10.4 µs. A ratio wide enough to tolerate honest machine variance is wide
enough to hide a doubling. The slope caught what the ratio could not, which is
why this table carries three shapes and not one number.

C-17 is the guard on the other three. A page that got flat by rendering nothing
would pass C-14, C-15 and C-16 and be worse than what it replaced, which is the
same class of mistake as capping the rows and calling the page constant.

**Search is deliberately not in this table.** It is measured by the same bench
and asserted by none of it, because a substring that matches most of the universe
must look at most of the universe. `docs/06-limits.md` §24 states the two cases
that stay linear and prints their cost.

C-13 is the row that had no bench at all until D-0040, and `CLAUDE.md` §3 rule 4
names **result append** among the operations that must be O(1). Measured at a
57,344-entry census: **5,100,585 ps per append before, 95,214 ps after**, and
the ratio across a 32× census went from 22.304× to 1.020×. The 1× baseline in
the "before" column was itself rehashing, so 53.6× is the honest figure for what
was removed. The round counts 1,000 / 10,000 / 50,000 passed the ceiling in both
columns — `HashMap::with_capacity` happened to round them up and leave spare
slots — which is why the harness measures both sets and why a bench that had
only visited round numbers would have reported the defect as absent.

C-12 was measured on a manifest built by `Manifest::genesis` + `record` until
D-0036 — a map that grew by rehashing — while three documents attributed the
flatness to a reservation taken on the load path that the harness never
called. The bench now builds its census through `Manifest::load`, so the map
measured is the map the claim is about, and
`pull::unit::the_loaded_index_is_reserved_from_the_census` (M-17) asserts where
the reservation comes from as a number.

C-01 was previously stated as "bar read cost is flat from 1× to 100× file size"
and proven by `store::bench::read_ratio`, which did not exist — there was no bar
reader in `crates/store` then. D-0034 restated it as the flatness that **is**
measurable today and named a bench that runs.

**The reader has since shipped and this paragraph's last sentence has not.**
`BarFile::read_record` is one multiply, one add and one 56-byte positional read,
and S-02 walks it at every index of a 160-record file. C-01 still names the
header read rather than the bar read, and deliberately: **no bench in this
repository times a syscall.** `crates/store/benches/ratio.rs` measures the
arithmetic and the checksum, which is what `crates/store/src/file.rs` says in as
many words beside `read_record` — the operation is constant and the device
latency underneath it is UNVERIFIED. A bar-read ratio row that timed a `pread`
would be measuring the operator's disk, so the row returns when there is a bench
that separates the two, not merely when the reader exists.

**Ceiling.** A ratio is a failure above **1.4×** on dedicated hardware,
**3.0×** on shared CI. The gap is measurement noise on shared vCPU, not a
different standard — a ratio above 3.0× on shared hardware is a real
regression, and a number between 1.4 and 3.0 on shared hardware is re-run on
dedicated hardware before it is believed. The harnesses assert the CI number
and print the ratio, so a local run can be read against the tighter one.

**These rows are enforced by a gate that ran for the first time on
2026-08-01.** Before D-0034 gate 8 probed for a repository-root `benches/`
directory, found none, and exited zero — see `docs/06-limits.md` §7b and §7c.

## Ingest

| # | Must hold | Proven by | |
|---|---|---|---|
| P-01 | A rate governor never issues above the configured ceiling, under any concurrency | `pull::concurrency::the_ceiling_holds_however_many_threads_share_the_governor` · `pull::concurrency::a_throttle_recorded_by_one_thread_binds_every_other` — 512 requests from eight real threads at one fixed instant are issued exactly the allowance between them, three times over, and a throttle one caller records binds the rest. `Governor::admit` takes `&mut self`, the type holds no interior mutability and the crate is `#![forbid(unsafe_code)]`, so exclusive access is the **only** sharing safe Rust admits and no two `admit` calls can interleave — which is why the `loom` module this row named is not merely absent but would have nothing to enumerate. **What is not bounded: two separate `Governor` values.** The type is `Copy`; nothing here or in the crate holds the sum of two of them to one ceiling | ◐ |
| P-02 | A bar outside the requested window is never stored | `pull::integration::a_bar_outside_the_requested_window_is_never_stored` · `pull::integration::a_narrower_window_stores_strictly_fewer_bars_and_says_why` · `pull::broker::a_bar_outside_the_session_is_dropped_and_counted_rather_than_stored` — **the first citation named no test in any file.** It read `a_bar_outside_the_window_or_the_session_is_never_stored`, which is the two real tests welded into one name: the window half is `integration.rs`, the session half is `broker.rs`, and the welded name is neither. Gate 10 discards the module segment, so `integration` vs `broker` was never the problem — the FUNCTION did not exist. Both are cited now, separately, because they prove different halves of the row. **"never stored" is checkable**, because `pull::ingest::from_dir` takes a vendor's folder all the way to an append. The window tests reopen the month afterwards and read **every** committed record back, asserting each one is inside the operator's window and inside the exchange's session, from a fixture carrying a row on each side of every boundary — 15:29:59 in, 15:30:00 out. The narrower window stores strictly fewer bars off the same bytes, so the filter is keyed on the request rather than on the file. The window *arithmetic* remains `pull::unit::a_window_is_inclusive_at_both_ends_and_refuses_to_run_backwards`, `pull::unit::every_second_of_a_day_falls_on_exactly_one_side_of_the_session` and `pull::unit::an_inclusive_window_survives_the_vendors_exclusive_to_date`. One member, one month, one instrument | ✓ |
| P-03 | A bar on a non-trading date is dropped and counted | **NOT PROVEN, and the code says so first.** `pull::unit::calendar_filter` exists in no file, and `crates/pull/src/session.rs` states plainly that there is **no trading calendar and no holiday list** here — so there is nothing yet to prove. A weekend rule without a holiday list would be wrong, which is why `pull::unit::a_saturday_is_a_full_session_because_there_is_no_weekend_rule` asserts the *absence* as the current behaviour. **The name stays in the backticks. What changed is that gate 10 now reports it as PENDING rather than as red**: D-0062 put `P-03` in the gate's `allow_pending` allowlist with the reason beside it, and the reason is the one above — the subject does not exist, so a test here would assert an absence and be deleted the day a calendar arrives. What closes it, in this order: a holiday list **sourced** into `docs/00-charter.md` §3 first, because golden rule 1 forbids inventing one; then the filter in `session.rs`; then `pull::unit::calendar_filter` driving it. The exemption keys on the ROW, so gate 10 also stops checking the second name above — that test still runs under `cargo test`, and the gate prints both silenced names every run | ✗ |
| P-04 | Re-running an ingest stores nothing new and reports zero net-new | **THE DISK HALF HOLDS. THE REPORTING HALF DOES NOT, AND THE TEST PINS THAT RATHER THAN HIDING IT.** `pull::integration::idempotent_repull_leaves_the_file_byte_identical` · `pull::integration::a_second_window_over_the_same_month_appends_rather_than_rewrites` — a second run over the same folder and window leaves the bar file **byte for byte** what it was, and a run that brings bars the file does not hold still appends them, so idempotence is not bought by refusing every second run. What is **false today** is "reports zero net-new": `Ingested::bars_stored` counts the bars a member *offered*, `BarFile::append`'s already-present answer never reaches it, and the re-run therefore reports 3 stored where it wrote 0. The test asserts that 3. Fixing it is a `crates/pull/src/ingest.rs` change this row does not own | ◐ |
| P-05 | A credential is read, never written; no token is ever minted | `pull::unit::readonly_credentials` (a write attempt must panic the test double) | ✓ |
| P-06 | An auth failure halts the pull loudly rather than degrading | `pull::unit::auth_halt` | ✓ |
| P-07 | A missing, unreadable, or incomplete credential configuration halts the pull and names the absent segment; it never defaults | `pull::unit::credential_config_absent_halts` · `pull::unit::a_missing_table_or_key_is_a_halt` | ✓ |
| P-08 | The credential configuration supplies path segments only; a secret value found in it is refused | `pull::unit::credential_config_rejects_secret_value` | ✓ |

Added by D-0035. **That paragraph read "`P-01` through `P-04` keep their `—`:
there is no rate governor, no window walk and no calendar filter yet." Two
thirds of it went stale and nothing moved the rows.** D-0037 shipped
`pull::rate::Governor`; `pull::session` shipped `Window`, `Day`, `DropReason`
and `DropCensus`. So a rate governor and a window walk both exist today, and all
four rows still named tests that exist in no file while wearing a glyph that
means *the crate does not exist*. `crates/pull` is tracked and compiled.
Corrected to `✗` by D-0045, each row saying which half is proven and which half
is not. Only the calendar filter is still genuinely absent from the code, and
`crates/pull/src/session.rs` is where that is decided and said.

**Three of those four rows have since been given tests, and the fourth has not.**
`crates/pull/src/ingest.rs` shipped the join — a vendor's folder to
`BarFile::append` — which is what made "stored" a checkable word for the first
time, so P-02 and P-04 are now driven end to end by
`crates/pull/tests/integration.rs`, and P-01 by
`crates/pull/tests/concurrency.rs`. **P-03 is unchanged and stays `✗`**: there
is still no trading calendar, and `crates/pull/src/session.rs` and
`crates/pull/src/lib.rs` both still say P-03 keeps its `—` where the table has
said `✗` since D-0045. Those two headers are stale on the glyph and right on the
substance; they are not this change's files to edit.

| # | Must hold | Proven by | |
|---|---|---|---|
| P-09 | A vendor that rejects a credential gets **one** re-read, and an unchanged value halts naming the vendor and the field; it is never re-minted | `pull::unit::a_dead_token_is_re_read_once_and_then_halts` | ✓ |
| P-10 | The parameter is always asked for decrypted, and the assembled path is what is asked for | `pull::unit::the_adapter_always_asks_for_a_decrypted_value` | ✓ |
| P-11 | A credential value never reaches a formatter: `Debug` is a redaction and there is no `Display` | `pull::unit::a_secret_never_prints_its_value` | ✓ |
| P-12 | A parameter path exists only for segments the configuration carries; a field nobody configured is refused, never assembled | `pull::unit::the_only_path_is_the_one_the_configuration_assembles` | ✓ |
| P-13 | The pasted-secret backstop fires at the first byte that is too many and not one before, and the length and byte-set bounds catch every credential shape measured | `pull::unit::the_secret_backstop_is_the_first_byte_that_is_too_many` | ✓ |
| P-14 | The declared path bound is reached exactly by a maximal configuration; it is tight, not merely sufficient | `pull::unit::a_maximal_path_is_exactly_the_declared_bound` | ✓ |
| P-15 | Every line the configuration reader does not understand is a halt naming the line; none is skipped | `pull::unit::every_line_this_reader_does_not_know_is_a_halt` · `pull::unit::a_vendor_tables_own_keys_are_checked_too` | ✓ |
| P-16 | The configured region is checked against the one `CLAUDE.md` §8 fixes, not merely read | `pull::unit::the_region_is_checked_rather_than_merely_read` | ✓ |
| P-17 | The configuration file is bounded **at the read** — at most `MAX_FILE_BYTES` + 1 bytes are ever pulled in, whatever `stat` claimed — and by line length before a line is parsed | `pull::unit::the_configuration_file_is_bounded_before_it_is_read` | ✓ |

Added by D-0036. P-17 previously read "by size before it is read", and the
check was `metadata().len()`, which is `0` for a FIFO, a character device and
any `/proc` entry — so the bound was not a bound and the read that followed was
unbounded. It is now taken on what was actually read.

| # | Must hold | Proven by | |
|---|---|---|---|
| P-18 | No formatter renders a path segment: `Debug` on the assembled path, on the whole configuration and on a vendor's table is a redaction, and `Display` on the path is the one audited exit | `pull::unit::no_formatter_renders_a_path_segment` | ✓ |
| P-19 | A path that is not a regular file is refused by name before it is opened, so a FIFO cannot hang the read and a device cannot make it unbounded | `pull::unit::a_path_that_is_not_a_regular_file_is_refused_by_name` | ✓ |

### The adaptive rate governor

Added by D-0037. Every row is proven by a test that runs today, and **not one of
those tests sleeps or reads a clock** — `pull::rate::Governor::admit` takes the
instant as an argument, so every boundary below is asserted at the exact
microsecond rather than near it.

**That paragraph read "`P-01` keeps its `—`, narrowed rather than satisfied",
and the glyph was wrong twice over** — D-0045 moved it to `✗`, and it is now
`◐`. The reasoning it gave is still the right reasoning and is now the
*argument* rather than the excuse: `Governor` takes `&mut self` and has no
interior mutability, so it cannot be shared across tasks without a lock this
crate does not supply — and that is exactly why an interleaving checker has
nothing to enumerate here. `crates/pull/tests/concurrency.rs` supplies the lock
in the test, races eight threads through it, and asserts the pool is held to the
allowance one caller would have had. The single-caller arithmetic below is
unchanged and is still where every boundary is asserted at the microsecond.

| # | Must hold | Proven by | |
|---|---|---|---|
| P-20 | The first request of a pull is admitted and costs exactly one permit in every bounded span; a span the vendor does not bound is `None`, never a large number | `pull::unit::the_first_request_of_a_pull_is_admitted` | ✓ |
| P-21 | A window admits exactly its allowance and refuses the next one, and a refusal charges nothing | `pull::unit::a_window_saturates_exactly_at_its_allowance_and_not_one_over` | ✓ |
| P-22 | A throttle **steps down** the allowance of **every** span and drains every bucket, and leaves every published ceiling untouched | `pull::unit::a_refusal_steps_down_every_allowance_and_drains_every_bucket` — **this row said "halves" and named a test that no longer exists.** D-0326 replaced halving with `ceiling / BACKOFF_STEPS` floored at one permit, because *"halving on that evidence discards capacity known to exist"* — one refusal took a 500-per-minute allowance to 250 on the strength of a single 429. The test was renamed in the same commit and this citation was not, so the row cited a corpse while describing behaviour the code had stopped having. Measured today: 8→7 per second, 500→485 per minute, 100,000→96,875 per day, every bucket drained to `Some(0)`, and every `ceiling` still reading its published figure — a back-off is not evidence about a ceiling, so only the allowance moves | ✓ |
| P-23 | Sustained success raises the allowance by exactly one permit at a time and stops **at** the published ceiling, never past it | `pull::unit::sustained_success_walks_up_one_permit_at_a_time_and_stops_at_the_ceiling` | ✓ |
| P-24 | The allowance never reaches zero, however many refusals arrive; the floor of one is a rate, so a governor can always climb back out | `pull::unit::the_allowance_never_reaches_zero_however_many_refusals` | ✓ |
| P-25 | A drained window earns back at exactly the permitted rate — asserted at the microsecond on both sides — and idle time never accumulates past a full bucket | `pull::unit::a_drained_window_earns_back_at_the_permitted_rate_and_is_whole_after_one_span` | ✓ |
| P-26 | When spans disagree the one with the longest wait denies and is named, no span is charged, and a tie goes deterministically to the shorter span | `pull::unit::the_span_that_waits_longest_denies_and_nothing_is_charged` · `pull::unit::a_tie_between_two_spans_is_broken_by_the_shorter_one` | ✓ |
| P-27 | The wait a denial reports is the **smallest** that clears it: one microsecond earlier is still a refusal | `pull::unit::a_denial_names_the_exact_wait_that_clears_it` | ✓ |
| P-28 | A clock that goes backwards grants no capacity and cannot panic; recovery is measured from the highest instant ever seen, not from the bottom of the dip | `pull::unit::a_clock_that_goes_backwards_grants_nothing` | ✓ |
| P-29 | A clock reading at the far end of `u64` does not wrap and grants at most a full bucket, including at the widest ceiling × span product this build accepts | `pull::unit::a_clock_at_the_far_end_of_u64_does_not_wrap` | ✓ |
| P-30 | Budgets pool per `(vendor, request kind)`: one kind's pool exhausting leaves the other untouched, and a throttle on one moves only that one | `pull::unit::one_request_kinds_pool_is_exhausted_while_the_other_is_untouched` | ✓ |
| P-31 | The same script gives the same verdicts and the same final state, every time — and the script exercises both arms | `pull::unit::the_same_script_gives_the_same_verdicts_and_the_same_state` | ✓ |
| P-32 | Every declared rate bound is exact at the limit — `MAX_CEILING` accepted and the first value past it refused, naming the span — and a ceiling of zero is refused rather than read as "no window" | `pull::unit::every_declared_rate_bound_is_exact_at_the_limit` | ✓ |
| P-33 | The vendor figures in `pull::rate` are the ones `docs/00-charter.md` §4 records, frozen against hardcoded numbers rather than re-derived | `pull::unit::the_published_vendor_figures_are_the_ones_the_charter_records` | ✓ |
| P-34 | A governor's whole state is a fixed-size `Copy` struct: ten thousand admitted requests leave its `size_of` unchanged, and it owns no allocation | `pull::unit::a_governor_holds_no_allocation_and_no_history` | ✓ |
| P-35 | Against a vendor honouring less than it publishes, the allowance converges into `1..=honoured` and never passes the published ceiling; when the refusals stop it walks back up to that ceiling | `pull::unit::the_allowance_converges_onto_the_rate_the_vendor_actually_honours` | ✓ |
| P-36 | A TOTP secret past `MAX_SECRET_LEN` is refused **before** a single character is decoded, so the only loop in `pull::totp` is bounded by a constant — proven with an over-long secret whose tail is not base32, so a bound moved after the loop would refuse it under a different name — and the bound is `>` and not `>=` | `pull::totp::the_length_bound_is_checked_before_a_single_character_is_decoded` | ✓ |

| P-37 | The fold grid's origin is the **IST day**, not the UTC day: a bar stamped 00:00 IST keeps its own calendar date through `fold`, and 00:00 IST and 23:59 IST of the same session land in one bucket | `pull::unit::a_daily_bucket_is_an_ist_day_so_a_midnight_ist_bar_keeps_its_own_date` | ✓ |
| P-38 | Anchoring that grid to IST leaves the minute rung byte-for-byte unchanged, because 60 divides `IST_OFFSET_SECS` — pinned across negative, zero and boundary instants | `pull::unit::anchoring_the_grid_to_ist_leaves_the_minute_rung_byte_for_byte_unchanged` | ✓ |

| P-39 | **Twenty of this crate's twenty-one `telemetry::emit` sites reach a file**, each driven through the shipped call that owns it — `Governor::record_throttled`, `CredentialConfig::load`, `split_window`, `base32_decode`, `Selection::of`, `Manifest::{open, image}`, `csv::decode`, `archive::read_dir`, `fetch::land`, `ingest::from_dir`, `CredentialReader::{read, reread_after_rejection}` and `HttpSource::window_async` against a loopback socket — and each record is found by target, sentence and one field, with a sequence number no earlier than the one the sink would have stamped before the call. The events are **never constructed by the test**: fabricating an `Event` with a production target on a locally-opened `Sink` is exactly what made the `pull.member` tests in `crates/api/src/logs.rs` worthless. Two of the sites are also proven **silent** where their own doc comments promise silence — a selection that dropped nothing and a re-run that moved nothing each write no line at all | `pull::emit_sites::every_emit_site_in_this_crate_reaches_a_file` | ✓ |

| P-40 | **The whole refusal cross product holds ten properties at every point.** 14 statuses × 13 `error_type` values × 2 contract states = 364 classifications, each asserted total, idempotent, and consistent between `decided_by`, `disposition`, `contested` and `unrecognised`. Includes the property that makes the module safe to add: a feed declaring **no** contract classifies to exactly the status, at every one of those points | `pull::refusal::the_whole_cross_product_holds_every_property` | ✓ |
| P-41 | **`TokenException` and `PermissionException` are two different events under one HTTP 403** — the first says a later run could succeed and the second says it could not, and the status axis alone answers `SessionDead` for both | `pull::refusal::the_two_403s_are_two_different_events` · `api::server::a_vendor_that_names_its_refusal_decides_the_ladder` | ✓ |
| P-42 | An `error_type` the vendor's own reader does not know survives **verbatim** to the operator and is never folded into `GeneralException`; a near miss in case, whitespace, length or encoding is not a hit, and a megabyte of it is refused like any other wrong name | `pull::refusal::a_near_miss_is_not_a_hit` · `pull::refusal::a_megabyte_of_error_type_is_refused_like_any_other_wrong_name` | ✓ |
| P-43 | A malformed error body — not JSON, not an object, no such field, a non-string field, a truncated object — yields *no name* rather than a second failure of this build's own, on a path that is already failing | `pull::refusal::a_malformed_error_body_yields_no_name_rather_than_a_new_failure` | ✓ |
| P-44 | **Every request through the governed source is charged, not just the first.** `pull::chain::month` is a 1 + N walk — one call for a month's expiries, one per expiry for its contracts, in a loop with nothing between them — and `fno_walk` charged the governor ONCE for all of it. Measured 2026-08-20, journal seq 790–823: Groww answered 429, the governor halved its spans *after* the refusal, and `POST /pull/fno` returned 502 twice while the spot pass of the same run stored 58,572 bars with `failed: 0`. The guard is on the TRANSPORT, so a walk added later inherits it. **The elapsed-time half of the assertion is the load-bearing one** — a test that only counted requests passes straight over the bug | `api::server::every_request_through_the_governed_source_is_charged` — twelve calls against Dhan's published 5/s must exceed one second of wall clock | ✓ |
| P-45 | **The expiry moment is read from the dated session table, never inlined.** A contract stops trading at its venue's close, and that close MOVED — 15:30 to 15:40 for NSE derivatives on 2026-08-03. A tenor computed against a fixed close is wrong by up to ten minutes on the one day where a wrong tenor costs most, and wrong silently. The unit asserts 30 minutes left at 15:00 before that date and 40 after it, from the same code | `pull::tenor::the_expiry_moment_follows_the_dated_close_and_is_not_hardcoded` | ✓ |
| P-46 | **A weekly's whole life sits below the band `greeks` validates itself in.** `crates/greeks/src/bsm.rs` skips `years_to_expiry < 0.02` in its analytic-versus-numerical test — 7.3 days — and a seven-day NIFTY weekly is born at **0.019910** years, half a percent under the line, falling from there. So the crate's own proof that its greeks match a numerical derivative has never run at the maturity this market trades at. Carried per priced row as `below_validated_band` rather than derived on a report, because deriving it at read time needs the dated session table and a reader that cannot reach one would print the greeks without the caveat | `pull::tenor::every_bar_of_a_seven_day_weekly_sits_below_the_validated_band` | ✓ |
| P-47 | **A resume position, never a flag.** `EntryKey` is `(contract, exchange, segment, symbol, timeframe, month)` — the window is not in it — and `Entry::check` refuses only `rows == 0`, so ONE BAR makes a contract-month "held". A boolean gate built on that reported a five-day month as complete for all thirty-one, and §8's append-only rule meant a re-run could not repair it: the file already held the prefix, so the wider batch was refused. `owed` asks the same single probe for `last_ts_micros` and answers with the day to resume from. The unit asserts the OLD gate's wrong answer beside the new one, so the contrast is a fact rather than a claim | `pull::fnowork::a_month_pulled_to_day_five_is_short_where_the_boolean_gate_called_it_held` | ✓ |
| P-48 | **A rate cannot exist without its provenance.** `docs/00-charter.md` records no risk-free rate, so §3 rule 1 forbids this build claiming one. `pricing::Rate` takes a closed `RateSource` enum, there is no constant of the type anywhere in the workspace and no `Default`, so a rate with no stated source is not a rate that compiles — the rule stops being something a reviewer has to notice. A blank `Charter` citation is refused too: it looks like the rule was followed | `pull::pricing::a_rate_without_a_citation_is_refused` | ✓ |
| P-50 | **A percentage typed where a decimal was meant is refused by name.** `9.46` for `0.0946` is the single likeliest wrong number on this path and the worst-behaved: the model accepts it, every greek comes out finite, and nothing downstream says a word — so the run is wrong in a way no later reader can detect. The ±1.0 screen is the only place that catches it. The band is symmetric rather than a floor at zero because **negative rates are real** | `pull::pricing::a_rate_without_a_citation_is_refused` — the `RateImplausible` and negative-rate assertions | ✓ |
| P-49 | **Paisa and rupees imply the same volatility.** Black-Scholes is homogeneous of degree one in `(spot, strike, premium)`, which is what makes working in paisa the same equation rather than an approximation and avoids a division that would round. Two consequences are stated rather than left to be found: implied volatility and delta are scale-free, and **gamma, vega, theta and rho are not** — they come out per paisa. The claim is checked, not cited | `pull::pricing::the_scale_does_not_change_the_implied_volatility` | ✓ |
| P-51 | **A shared rate governor is charged by exactly one side, and that side is the caller.** `Governor::admit` is a withdrawal and not a query — the type offers no peek — so two layers gating one governor spend two permits per request and halve the ceiling without saying so. `sharing` is the contract that settles it: handing a governor over and spending from it are one act, so a source that was handed one observes rather than withdraws. A source that was NOT handed one keeps the governor `new` built for it and gates itself, so there is no third state and no way to add an ungoverned request path by forgetting something. **What this does not bound:** the feedback path is deliberately unconditional — `record_success` and `record_throttled` run on every answer either way, because a shared governor must learn from every path whether or not that path is the one that pays | `pull::http::a_shared_governor_is_charged_by_the_caller_and_not_again_here` — `Governor::admit` is the only thing that moves `cursor_micros`, and it moves it forward only, so the cursor moves if and only if the governor was asked for a permit. A shared source must leave it exactly where it found it; an owning source must move it, and that second half is what stops the first from holding for the wrong reason. **An earlier draft asserted timing instead and its mutant survived** — with the guard removed the unguarded path slept out the remainder of the second and still returned inside the timeout, which is a test that asserts nothing in the sense `CLAUDE.md` §4 bans | ✓ |
| P-52 | **An ordered-subsequence match between a vendor's symbol and an exchange's name is unsound, and the digits are what make it sound.** A vendor abbreviates: Zerodha writes `NIFTY PVT BANK` where NSE publishes `Nifty Private Bank`, and neither is derivable from the other by rule, so the two are joined by asking which published names accept the symbol as an ordered abbreviation. Measured against NSE's published list that rule hands `NIFTYGS813YR` — the 8-13 year G-Sec benchmark — to `Nifty LargeMidcap 250 plus 8-13 yr G-Sec 70:30`, a hybrid equity-and-debt index that is not the same instrument in any sense. The letters simply fall in order inside a longer name: subsequence has no notion of a word boundary, so its false-positive rate rises with the length of the candidate. **The numbers are the discriminator** — the abbreviation names one, the expansion names three. `digits_agree` requires the count to match and each abbreviation run to be a *suffix* of its expansion run, a suffix rather than an equality because `BHARATBONDAPR30` legitimately shortens `APRIL2030`. **What this does not claim:** the constraint makes the rule conservative, not correct. It converts false-unique into refused, and a symbol whose words the vendor *reordered* — `GS` ahead of the tenor — is refused rather than resolved. Refusing is what §4 requires of a join that cannot see; naming the hybrid would be the invention §3 rule 1 forbids | `pull::nseindex::the_hybrid_index_never_answers_for_the_gsec_benchmark` — asserts first that the unsound rule really does accept the hybrid, so the test fails if the false positive is ever quietly designed out rather than guarded, and then that the digit constraint refuses it and that `resolve` answers `Absent` for both candidates | ✓ |
| P-53 | **A vendor's third decimal is its float's error, not the exchange's price, so it is snapped half-up rather than refused.** Dhan types every OHLC field as `float` in its own documentation, and a binary float cannot hold most two-decimal decimals: BANKNIFTY's `35922.60` is held as the nearest `f32`, `35922.6015625`, and printed to four places as `35922.6016`. That the value is an exact multiple of 2⁻⁸ — `35922.6015625 × 256 = 9_196_186`, a whole number — is what proves the provenance rather than merely suggesting it. At that magnitude one `f32` step is 1/256 of a rupee, so only `.00`, `.25`, `.50` and `.75` survive the round trip and 96% of bars carry the error before the vendor sends them. Refusing such a value discarded the whole window and with it the whole instrument: 42 Dhan minute runs, none reaching the store, one recording `bars_stored: 439240, bars_committed: 0`. **Snapping loses nothing** — it removes the vendor's rounding error and recovers the exchange's own two-decimal price, and it is what `CLAUDE.md` §7 and D-0010 always required. **What this does not extend to:** a count is not a price and `one_number` still refuses `250.5`, and the local-archive CSV path still refuses a third decimal because text a vendor wrote as text has not been through a float | `pull::http::the_four_dhan_index_opens_that_refused_now_land_on_the_exchanges_price` — feeds the four literal values from the `pull.http` refusal events and asserts each lands on the exchange's price · `pull::http::the_snap_is_half_up_at_the_boundary_and_on_both_signs` · `pull::http::a_fractional_count_is_still_refused_rather_than_snapped` pins the half that did not change | ✓ |
| P-54 | **A vendor refusal moves the governor exactly once, and the allowance moves incrementally in both directions.** Three defects in one loop. First, `window_async` records the feedback where the status is read and `api::with_retry` recorded it again on the same shared `Arc`, so one 429 QUARTERED the allowance — on a premise its own doc stated and contradicted inside one sentence, *"both entry points already call … which the bars path's `window_async` does not"*. The transport owns the feedback now, with one exception it cannot see: a throttle the vendor NAMES in its body under a non-429 status, which the wrapper still records. Second, `relax` HALVED a figure the vendor published — measured, the day allowance walked 100,000 → 50,000 → 25,000 → 12,705 → 6,352 and a 213-instrument pull needing ~12,996 requests could no longer finish at all. The step is `ceiling / BACKOFF_STEPS` in both directions now, so a refusal costs 3.1% of a daily quota rather than half of it. **Third, and it is what a symmetric rule gets wrong:** one step down per refusal against one step up per success settles ABOVE the honoured rate rather than below — a vendor publishing 8/s and honouring 3/s held a stable 6, because three successes a second exactly cancelled three refusals a second. In integer permits that asymmetry cannot come from the step SIZE, since `ceiling / N` floors to one in both directions at a small ceiling; it comes from FREQUENCY, one step up per `SUCCESSES_PER_STEP` clean answers. **What this does not claim:** the daily ceiling is untouched at 100,000, and converging from far above the honoured rate now costs ~32 refusals where halving cost ~10 | `pull::unit::the_allowance_converges_onto_the_rate_the_vendor_actually_honours` — failed on the symmetric version, and is why `SUCCESSES_PER_STEP` exists · `pull::unit::a_refused_day_allowance_stays_usable_and_recovers_in_bounded_requests` asserts BOUNDS, never exact counts, so it cannot become a copy of the implementation · `pull::unit::three_refusals_take_a_rate_ceiling_where_one_halving_used_to` states the trade as arithmetic · `pull::concurrency::a_throttle_recorded_by_one_thread_binds_every_other` | ✓ |
| P-55 | **A volume counts shares traded, so on a TRADED listing it is refused at the vendor boundary when it is negative — through all THREE decode doors.** (An index is not traded and its volume column carries nothing; P-60 owns that carve-out and was written when this row alone would have been false.) `Bar::counts_are_sane` is `volume >= 0` and it runs in `store::file::survey`, one crate and ~1,500 lines from the vendor, by which point the body is gone: Dhan's `volume: -95342` on ADANIENT decoded cleanly, folded, and died as `ImpossibleCount` at "batch record 2333" — an index into a per-month slice the operator cannot open, printed beside the open-interest null as a second suspect, and costing the instrument-month, its seven derived rungs and every later month in the batch. The floor is on `one_volume` and NOT on `one_number`, because `timestamp` has a legal negative range — `IstMoment::from_epoch_secs` adds the 19,800-second IST offset before testing the sign, so `-19_800..0` is 1970-01-01 and is admitted. **Three doors, and the first patch guarded two:** the parallel-array arm that Dhan actually answers in was missed while `decode_objects` and `decode_positional` were fixed, and the test caught it. **What this does not do:** it does not drop-and-count — a negative volume is evidence the decoder may be reading the wrong column, and `csv.rs` already records the ruling that storing a sent value as absent asserts something false. `csv::decode`'s own volume field has the same hole and is not closed here | `pull::http::a_negative_volume_is_refused_where_the_vendor_still_owns_the_value` — drives a real parallel-array body and pins `one_volume` directly · `pull::http::a_negative_timestamp_is_not_a_negative_count` pins the half that must not change | ✓ |
| P-56 | **A price the vendor did not write as zero never becomes one.** Half-up sends everything under half a paisa to zero, and the reader it replaced refused those values outright — so D-0321's snap silently gained a band, `[0.00001, 0.005)` and its mirror, where a real value decodes as a clean `0`. Zero is a LEGAL price (`Bar::ohlc_is_sane` admits it, `every_price_on_the_paisa_grid_survives_intact` pins `"0"` as real), so nothing downstream could tell an invented zero from a sent one — the same lie the null-column refusal names in its own words three hundred lines up. **The negative half is the sharp one:** `-0.001` snaps to `0`, `0 < 0` is false, and it walked past the below-zero guard entirely, making that diagnostic unreachable across `(-0.005, 0)`. The test is on the TEXT and not the number, so `0`, `0.00` and `-0.0` still decode as the zero they are. **Found by adversarial review of the fix, not by the suite** | `pull::http::a_price_that_is_not_zero_never_snaps_to_zero` — six values across both signs refused by name, four real zeros still accepted | ✓ |
| P-57 | **A vendor's own sentence outranks a code it misfiled, and it can only ever PROMOTE to a dead session.** Measured: Dhan answered HTTP 400 with `{"errorType":"Order_Error","errorCode":"DH-906","errorMessage":"Invalid Token"}`. `dhan::read` maps `DH-906` to `RequestWrong` and is faithful to the annexure in doing so — *"Incorrect request for order"* — so the run was told its own REQUEST was wrong, never re-read the credential, and re-asked the whole instrument from chunk one against a dead token, up to `MAX_PASSES` = 400. `errorType` corroborated the wrong code, so only the message carried the truth. **The guard is what makes this safe:** the sentence is consulted only where the code's answer is not already an auth verdict, so `DH-902`/`806` (`NotEntitled`, the one disposition no later run can fix) return before it is read — promoting either would tell an unsubscribed operator to wait for a refresh that fixes nothing, which is the exact failure `dhan.rs` exists to close. Five published phrases, never the bare word `token`: `811` "Invalid Expiry Date" and `813` "Invalid `SecurityId`" both contain "invalid" and must keep their own dispositions. **Recorded structural fact:** `refusal::classify` — the function this rule belongs in — has ZERO production call sites, so the rule went where the code actually runs, `http::refusal_words` | `pull::dhan::the_body_that_carried_a_dead_token_under_an_order_code_is_read_as_a_dead_session` feeds the operator's literal body · `pull::dhan::an_unsubscribed_account_is_never_promoted_to_a_dead_session` pins the guard on all six auth codes · `pull::dhan::a_sentence_that_merely_contains_invalid_is_not_a_dead_session` pins the four near misses · `pull::dhan::a_contract_without_a_message_reader_is_unchanged` pins Kite and Groww | ✓ |
| P-58 | **A refused chunk discards the suffix, never the contiguous prefix — and a short window is reported as a member failure.** `fetch_chunks` returned `Err` on any chunk failure and dropped every chunk already fetched, on the stated premise that *"a partial answer to a whole request is a gap this store cannot correct later"*. The premise is false: `split_window` sets `start = end + 1` every iteration, so chunks ASCEND, TILE and never overlap, and chunks `1..k-1` are therefore strictly OLDER than the one that failed. `Header::advance` refuses only a batch beginning at or before what is committed, so the suffix stays a legal append; and both resume paths — `autopilot::next_window` and `fnowork::owed` — probe `Entry::last_ts_micros`, a TIMESTAMP rather than a flag, so the next run asks for exactly what this one discarded. Measured cost of the old rule: one run recorded `bars_stored: 439240, bars_committed: 0`. **There were TWO discard sites** and the second — the budget wait — was never named by the message. **The safety rests on the shortfall being loud:** `Chunks::unfetched` becomes a member failure, so `Ingested::balances` answers NO and a run that fetched 65 months of 66 cannot read as clean; an EMPTY prefix stays an `Err`, because that boundary carries 502-versus-400. **Known limit, recorded not closed:** `ladder::probing` is a boolean with no window in its key, so a partly-filled boundary month could let the day gate admit a minute pull for a 23/31-done month — bounded at one instrument-month per failed instrument per run, and self-healing on the next day pass | `api::server::a_refused_chunk_keeps_the_contiguous_prefix_and_names_what_is_missing` — a real socket answering then refusing, asserting the prefix survives, the reason travels, AND that every kept chunk is older than the refused one, which is the line that catches `split_window` if it ever stops ascending · `api::server::an_empty_prefix_is_still_a_refusal_and_not_an_empty_success` · `pull::session::a_chunk_fills_the_cap_and_the_chunks_tile_the_window` is the ordering proof this rests on | ✓ |
| P-59 | **A transport error carries every cause underneath it, and a hang is told apart from a refusal.** `reqwest::Error`'s `Display` prints *"error sending request for url (…)"* and stops; DNS, TLS, reset and timeout all live in `source()` and were dropped. Measured: all four `niftyindices.com` constituent pages failed every thirty seconds for days and the page named no cause — while the intervals between failures were 29.998 s, 30.002 s and 30.001 s, **exactly `DOCUMENT_TIMEOUT_SECS`**, which is the signature of a host that accepts the request and goes silent. A DNS failure or a refused connection returns in milliseconds; only a hang costs the full timeout, so that one measurement rules out the three causes an operator would otherwise chase first. `cause_chain` renders the chain, bounded at eight links because a cyclic `source()` is a hang rather than a diagnostic, and a `connect_timeout` now splits DNS and TLS from the answer — which `masters::PublicFetch` already did, in its own words. **What this does not claim:** the host is still silent and nothing here changes that; this makes the next failure diagnosable rather than fixing it | `pull::resolve::a_socket_that_never_answers_refuses_rather_than_hanging` covers the hang path; the chain rendering is exercised by every transport failure and is **UNVERIFIED as a unit** — no test in this workspace constructs a nested `reqwest::Error` | ~ |
| P-60 | **An index is not traded, so its volume column has no referent — a negative there is recorded as the zero it always is, counted, and never refused.** Measured, not asserted: across 6,493 stored BANKNIFTY 1-minute bars read back through the shipped decoder, the ONLY distinct value in that column is `0`, and the operator confirms index volume is normally zero. So Dhan's `-2` and `-125` are noise in a structurally empty column, and P-55's refusal cost the whole backfill: BANKNIFTY died at chunk 1 of 21 with an EMPTY prefix (so D-0327 correctly yielded nothing at all) and NIFTY at chunk 3, leaving every month after 2022-02 absent while the OHLC in those chunks was good. **The rule turns on whether the column can carry a quantity, never on the sign alone** — an equity or a derivative still refuses, because there shares DID trade and zero would be the lie §4 bans. **Counted, not silent:** one `pull.decode` event per WINDOW (not per row — a 90-day minute chunk is ~34,000 bars and an emit in that loop is the cost §3 rule 4 refuses), carrying the count AND a `bars` denominator, because 34,000-of-34,000 means the decoder is reading the wrong column and 34,000-of-100,000 means a vendor patch, and those want opposite responses. **What this gives up, stated:** an operator sees how many were corrected in a window and cannot see WHICH bars; on disk a corrected zero is indistinguishable from a sent one. **What it does not cover:** `csv::decode`'s volume field is not listing-aware, so the identical bar still kills the batch on the archive path — recorded, not closed | `pull::http::an_index_negative_volume_is_zero_and_an_equitys_is_still_refused` — drives the operator's own `-125` on all three listings, asserting the index keeps its OHLC and the equity and derivative still refuse, and that a real zero decodes untouched on all three · `pull::emit_sites::every_emit_site_in_this_crate_reaches_a_file` drives the correction through `decode_body` and proves the event reaches a FILE, which is what kills the surviving mutant `cargo mutants` found on `note_volumes_corrected` | ✓ |
| P-61 | **A null price skips its row in ALL THREE decode shapes, and the columns are length-checked before the mask exists.** A vendor answers `open: null` for a minute that did not trade; `decode_objects` and `decode_positional` `continue` past it and the parallel-array arm refused the whole window — and that arm is Dhan's, whose own field table marks every response field *Required: No*. **Third time a rule reached two of three doors and the missed door was the same one:** D-0323 on negative volumes, D-0332 again, this on nulls. The columnar shape is the one that gets missed because it meets a bar as a COLUMN — there is no `continue` to reach for, and skipping index 3 of `open` without skipping index 3 of the other six assembles a bar from rows that never lined up. So `kept_rows` decides once, across the price quartet, and every reader filters through one mask. **Skipped, never zero-filled** — a zero price is a lie about a minute with no trade — and counted, with the same event and `eprintln!` the object shape always wrote. **The ordering is load-bearing:** every column is verified BEFORE the mask, because `zip` stops at the shorter side and a longer column would be trimmed to fit, erasing the disagreement `RawWindow::decode` exists to catch. The refusal stays `LengthDisagreement`, which names all seven lengths; a check inside the filter would have had two numbers and could not say which column was odd. `open_interest` reports the quartet's length when the descriptor declares no name, because an absent column and an empty one are different facts | `pull::http::a_null_price_skips_its_row_rather_than_refusing_the_window` — asserts WHICH two rows survive, field by field and stamp by stamp, because a filter applied to some columns and not others keeps the right count and the wrong rows; then walks all four prices one null at a time · `pull::http::all_three_decode_shapes_skip_a_null_price_alike` fails when the shapes diverge, which is worth more than asking the next person to remember · `pull::http::arrays_that_disagree_in_length_refuse_the_whole_window` still pins the `LengthDisagreement` variant | ✓ |
| P-62 | **A calendar is read from the segment the census recorded, never from a default — and the cache key is the whole path, not the name.** `calendar_json` grouped held entries into a map keyed on `series.symbol` alone and then passed the literals `"NSE"` and `"INDEX"` to `calendar_of::cached` on BOTH branches, for every instrument. `census::held_entries` yields `(Series, YearMonth)` and `Series` carries `exchange`, `segment` AND `contract`, all three populated from the same `EntryKey` `pull::ingest` wrote the file under; the grouping discarded them, so nothing was left to pass and a constant stood in for a fact already read. **Measured:** `ADANIENT` is an `EQ` row filed at `bars/dhan/NSE/CASH/ADANIENT/`, and this route asked `bars/dhan/NSE/INDEX/ADANIENT/` **366 times** — 61 held months × 2 rungs × 3 cache-missing derivations — while **1,240 daily bars** sat on disk, decoded and verified. `CLAUDE.md` §4's banned row exactly, and the WRITE path had already been fixed for the mirror defect, its comment predicting this one: *"a later reader asking for `NSE/CASH/360ONE` finds nothing while the data sits one directory over."* **The cache key widens with it, and that is not incidental:** `Cache` was `(Vendor, String)`, safe only while every caller passed the same literals; making one symbol addressable under two segments would otherwise serve `NSE/CASH/X`'s calendar to a request for `NSE/INDEX/X` under a hit — the same wrong answer arriving through the map instead of the path. **Contract-bearing series are dropped and said so:** `derive` hands `bars::open` a `contract: None`, so those bars — at `symbol/contract/` — cannot be addressed through it; they were previously probed at `symbol/`, found empty and silently counted as voting for nothing. **Order is pinned** because `HashMap` iteration varies per process and `agree` ships the names it derived from. **What this does not claim:** the F&O calendar is not now derivable — it is deliberately out of scope, and threading `contract` through `derive` is recorded, not closed | `api::server::an_equity_keeps_cash_and_a_name_under_two_segments_does_not_merge` — asserts the equity is keyed under CASH, is NOT reachable under INDEX, and that one name under two segments stays two buckets, which is the half that makes the first hold on a store that has both · `api::server::a_contract_series_is_left_out_rather_than_probed_at_the_spot_path` · `api::calendar_of::a_cash_instrument_is_read_from_cash_and_is_absent_from_index` asserts BOTH that the right segment reads and that the wrong one returns empty AND reports unreadable, because the first half alone would pass against a `derive` that ignored its argument | ✓ |
| P-63 | **A refusal under HTTP 200 is a refusal, and only a THROTTLE moves the rate allowance.** `window_async` recorded rate feedback from the status alone — 429 narrowed, everything else earned the additive increase — and read the body for a refusal only afterwards, by which point the governor had been told a failed call went well. This is not hypothetical for Dhan: its own SDK example tests `response["status"] == "failure"` and never reads the HTTP status, so a `{"errorCode":"DH-901"}` under a 200 produced NO disposition at all — no session-dead, no credential re-read, no ladder — plus an allowance one step wider than before. `weigh_answered_body` moves the decision to where the evidence is: the body goes through `refusal::disposition_of` before any success is recorded, a named refusal returns `VendorRefused` carrying its disposition, and only a clean body earns the increase. **`Disposition::Throttled` and only that one records the throttle**, because a throttle is a statement about PACE and nothing else is — `cargo mutants` proved this clause load-bearing by flipping `==` to `!=` and surviving the whole suite, a mutation that would make the governor back off for a dead token and accelerate into a rate limit, both wrong at once | `pull::http::a_refusal_under_a_200_is_a_refusal_and_never_a_success` covers DH-901, the misfiled DH-906 read by sentence, and a clean body · `pull::http::only_a_throttle_named_in_the_body_narrows_the_allowance` reads the allowance through the shared governor because the narrowing IS the side effect: DH-904 must lower it, DH-901 on a second client must not move it. Mutants on this function: 4 tested, 2 caught, 2 unviable, 0 missed | ✓ |
| P-64 | **An impossible ROW is dropped; the window, the chunk and the suffix all survive — and the chunk-level discard stays exactly as it is.** Two measured faults took one path out: Dhan's `volume: -125` on ADANIENT, and an impossible OHLC on NIFTY record 5997 / BANKNIFTY record 7075. The columnar decoder meets a bar as a COLUMN and collects into a `Result`, so the first `Err` short-circuited the array; the window's failure ended the chunk loop at **request 1 of 21**; and because Dhan's descriptor holds exactly one intraday spelling (`Minute1 → "1"`) with 2/3/5/10/15/30/60 rolled up locally from it, losing the 1-minute rung lost **all eight**. ADANIENT: 61 months of daily bars, **zero** intraday. The OHLC pair recurred identically across **six consecutive runs ~4½ minutes apart** — deterministic vendor bytes, so the retry could never converge. **The refusals were right about the values and wrong about their reach:** `kept_rows` now takes the listing and drops a negative-volume row on a TRADED listing only (an index is untouched — P-60 — its column has no referent), and `drop_impossible_bars` runs at all three `RawWindow::decode` sites with the same predicate as `Bar::ohlc_is_sane`, applied where the vendor's own row is still in hand rather than ~1,500 lines downstream where the batch index names nothing an operator can open. Both count; both emit one `pull.decode` warning PER WINDOW with a denominator, never per row — a 90-day minute chunk is ~34,000 bars and an emit in that loop is the cost §3 rule 4 refuses. **What must NOT change, and it is the load-bearing half:** making `fetch_chunks` continue past a failed chunk would be destructive — `Header::advance` refuses any batch beginning at or before what is committed, so writing chunk N+1 over a skipped chunk N makes N **permanently unwritable**. A skipped ROW is a legal gap because bars need only be strictly increasing; a skipped CHUNK is not. **What this does not claim:** the deterministic retry still re-asks for a month it will never complete — quarantine is recorded, not closed | `pull::http::an_impossible_bar_is_dropped_and_the_rest_of_the_window_survives` asserts WHICH row survived — volume, price and stamp — because a filter applied to some columns and not others keeps the right count and the wrong rows · `pull::http::each_clause_of_the_bar_relationship_drops_the_row_on_its_own` drives all nine clauses one at a time plus five legal shapes, including a flat bar and a zero-priced one · `pull::http::a_dropped_row_leaves_every_column_and_an_absent_one_stays_absent` · `pull::http::a_clean_window_is_not_filtered_at_all` · `pull::http::a_negative_volume_is_caught_whether_it_arrives_as_an_int_or_a_float` — `as_i64()` answers `None` for `-125.0` · `pull::http::each_column_on_its_own_can_be_the_odd_length_one` covers the OVER-length case, the only one that can tell this check apart from `RawWindow::decode`'s · `pull::emit_sites::every_emit_site_in_this_crate_reaches_a_file` drives both new events to a FILE. Mutants: 58 tested, 58 caught, 0 missed | ✓ |
| P-65 | **Every decoder checks the sign of every count, and `i64::MIN` in the open-interest column is still SHOUTED about rather than skipped.** D-0337 moved the JSON path's refusals from the window to the row and named three gaps it did not reach; this closes them. **The archive decoder had no sign check at all:** `csv::paisa` parses a leading minus by design, and a snapshot row carries ONE price into all four OHLC fields — so a negative price built a bar `Bar::ohlc_is_sane` refuses, travelling ~1,500 lines to die against a batch index naming no line of the file it came from. The price is now refused AT THE LINE, carrying the line number and the text, matching this decoder's own file-level design; the volume SKIPS its row and is counted, because its neighbour's zero-substitution reasoning does not carry — an unreadable field cannot be expressed as absence in `RawRow::volume`, but a field that parsed and said `-125` is the vendor stating something impossible, and a zero beside it would assert no trade in a minute this build has no reading for. **`open_interest` had no negative guard one column over from the volume that does:** `one_number` refuses only the sentinel, so `-5` decoded, landed and died at `survey` as `ImpossibleCount` — the exact omission D-0323 fixed for volume. `kept_rows` now reads that column's VALUES, not only its `.len()`. **`i64::MIN` is exempt and the first draft got it exactly backwards:** written as `as_i64().is_some_and(..).unwrap_or_else(|| as_f64()..)`, the sentinel passed the integer predicate, fell through to the float fallback, came back `-9.22e18` and was silently skipped — swallowing the one case §7 needs loud. An existing test caught it, and that is the argument for the `match`: when there IS an integer spelling it is the whole answer | `pull::http::a_negative_open_interest_skips_its_row_and_the_sentinel_still_shouts` — an ordinary `-5` skips its row while a good neighbour survives, `i64::MIN` beside a good row still REFUSES (the neighbour is there so a skip would leave something behind and read as success), and both float spellings are driven, including `0.0`: `cargo mutants` turned `< 0.0` into `<= 0.0` and survived, a mutation that would delete the bar of every contract with zero open interest — most of an option chain, every day · `pull::csv::the_sentinel_guard_reads_each_layouts_own_open_interest_column` now asserts BOTH halves — the file decodes, so the sentinel guard is still scoped to its one column, and the row is gone · `pull::pipeline::a_price_is_put_on_the_paisa_grid_exactly_or_refused` · `pull::emit_sites::every_emit_site_in_this_crate_reaches_a_file` drives the third new event to a FILE. Mutants: 55 tested, 54 caught, 1 unviable, 0 missed | ✓ |
| P-66 | **The last `"INDEX"` default on a read path is gone: `/bars` resolves its segment from the census, honours an explicit one, and REFUSES a name it cannot locate.** `bars_html` read `param_or(query, "segment", "INDEX")`, justified in `param_or`'s own doc as *"the only values the engine surface has (`CLAUDE.md` §1)"* — which conflates two sets §1 keeps apart in consecutive sentences: the engine SWEEPS two instruments, and *"futures, options and single stocks may be stored. They are never swept."* This route reads the store. **Latent rather than live** — `render` always writes `&segment=` from the census, so only a hand-typed URL reached it — which is why it is separate from P-62, where the identical literal WAS live and cost 366 probes against 1,240 bars on disk. Resolved rather than refused, because a URL needing three parameters to answer a one-parameter question is the worse answer; an EXPLICIT parameter always wins, since the census answers a question the caller did not ask and must never override one they did. **The two outcomes are now different statuses and that is the point:** `400` is "I could not work out where to look", `404` is "I looked and the month is absent" — the defect collapsed both into the second, and the two send an operator opposite ways. `param_or` is deleted, which repaired a sentence: it had been inserted between the halves of `bars_html`'s doc paragraph, splitting *"between absent"* from *"and unreadable"* | `api::server::the_bars_route_refuses_a_name_it_cannot_locate_rather_than_guessing_index` — asserts the refusal NAMES why it could not look, that an explicitly addressed series is still looked for, and that the two arms answer 400 and 404 rather than one status for both | ✓ |
| P-67 | **A body this build could not read is kept verbatim, with the reason beside it, under a ceiling that cannot grow.** Dhan sent `volume: -125` for `ADANIENT` and nothing could afterwards say what `-125` WAS — a genuine value, a wrapped `int32`, or a column at the wrong offset, three faults wanting three responses. The evidence was gone because nothing keeps it: `pull.journal` is a fixed-stride record of URL and message, measured at **7.8 KB / 2 records** for a run that committed ~2M bars, and `capture::record`'s budget keeps the first few answers a feed GIVES — an answer that fails to decode is exactly the one it has no reason to keep. `record_unreadable` is called from the `map_err` that builds `BodyNotUnderstood`, before the sentence replaces the evidence: body verbatim and LAST in the file, the refusal's own words on a `why:` line, the URL, and the byte count STATED rather than delimited because a separator could occur inside JSON. **Its own budget, deliberately:** a feed that spent its unreadable budget must still keep a good answer, and one vendor misbehaving must not blind the build to a second starting to — `PER_SLOT` per feed, `FEED_COUNT * PER_SLOT` files for the life of the process, because a run that fails a million times is the one that must not also fill the disk. **It can never fail a pull and here that matters MORE:** the caller is already returning a failure, and turning a diagnostic's failed write into a second different failure would replace the operator's reason with one about the disk — §4 pointing the wrong way. **No header, ever** (§8 puts the credential in one) | `pull::capture::an_unreadable_body_is_kept_with_its_reason_and_the_budget_stops` — asserts the body is byte-for-byte and last, that the verdict travels with it, that the length is stated, that no credential header appears, that the ceiling is a ceiling and not a rate, that the fixture budget is untouched, and that a second feed's budget is independent. Mutants: 9 tested, 9 caught | ✓ |
| P-68 | **A chunk that answers with NO ROWS is reported as a member failure, and the chunks after it are still kept.** `fetch_chunks` pushed every answered body with no emptiness check; the only record was a `Trace` line, below the default floor, and `note_short_window` never fired because `unfetched` is set only on a REFUSAL. Downstream the silence compounds: `from_window` records no entry and no failure for an empty batch, `Ingested::balances()` is `0 == 0 + 0` and answers YES, the run reports `Stored` — and if a LATER chunk of the same month lands, `last_ts_micros` reaches the month's end, `autopilot::next_window` answers `None` (a pure last-timestamp test, no interior-hole check), and the month is marked complete with those days gone. §8 forbids the rewrite and `Header::advance` refuses a batch beginning before what is committed, so nothing can go back for them. **Emptiness alone is not proof of a hole** — a holiday-only chunk legitimately holds none — which is why this counts rather than refuses; but `split_window` sizes chunks from the vendor's published cap (~90 days for Dhan's minute route) and no stretch of the Indian calendar that long lacks a session, so it is reported. **The suffix is KEPT**, which is what separates this from a refusal: the vendor answered, it answered with nothing, and discarding the rest would be D-0327 from the other direction. **What this does NOT close:** `pull::gaps` was built to classify absent minutes and has ZERO callers, so nothing can yet tell a complete month from a holed one — this makes the moment of loss loud, not the state afterwards detectable | `api::server::a_chunk_that_answers_with_no_rows_is_named_rather_than_taken_as_success` — drives a well-formed all-empty-columns body (not a malformed one, which would take the refusal path and prove nothing) followed by a landing chunk, and asserts the suffix survives, the reason names NO ROWS AT ALL, and it explains why it cannot be corrected later | ✓ |
| P-69 | **The one event per run NAMES the run, and the name comes from the plan rather than from a field that is `None` wherever it emits.** Measured on the operator's store, 2026-08-28: **884 `pull.run "ingested"` events in one rolling log**, each carrying `members / rows / bars / committed / folded / dropped / failures` and **not one naming its instrument, month or rung** — so the log said 884 times that something had been ingested and could not answer *which* month came up short, the only question those counters exist to serve. **The first correction was wrong and this is why the invariant is stated about the SOURCE:** `Ingested` already carries `pending: Option<Held>` whose `Entry::key` is exactly the `(exchange, segment, symbol, timeframe, month)` tuple wanted, so the draft read it from there — free, nothing threaded — and named nothing, because `pending` is assigned in `from_rows` alone (the rolling-contract path) while **every one of the 884 comes through `from_members`**, where it is `None` from first line to last. It compiled and all 714 tests stayed green. `note_run` now takes `members` and the `Plan`, which carries the run's constants unconditionally. **`month` is ONE key**: `2022-10` inside a month, `2022-10..2022-11` across one, because the log's field match is substring and a separate `to_month` would hide a spanning run from exactly the query an operator types. **`instrument` is named only when the run has exactly one member** — a broker pull is always `from_members(slice::from_ref(&member), ..)`, a folder walk can carry many, and picking the first would read like a fact and not be one, the §4 shape. Cost is five `with` calls and one `YearMonth` render per RUN, never per bar | `pull::emit_sites::every_emit_site_in_this_crate_reaches_a_file`, row `ingest.rs — note_run`, asserting the `instrument` field of a record read back **off the disk** — the row that refused the draft, via its drive's failed `pending.is_some()`. Falsified deliberately before landing: asserting the wrong symbol fails with the file's own contents, so the row is not vacuous | ✓ |
| P-70 | **A stored month can be asked whether it is WHOLE, and the denominator comes from the calendar rather than from the bars being audited.** `pull::gaps` classifies every absent minute into `closed` / `outside-window` / `vendor-hole` / `unmeasured`, a taxonomy built so a complete series stops reading as a short one — and it had **ZERO callers**: `grep -rn "gaps::"` outside its own file returned nothing workspace-wide, so every fact it could state was unreachable. P-68 named this as its own unclosed half — an empty chunk is loud at the moment it happens and the STATE afterwards was undetectable, while §8 and `Header::advance` together mean nothing can go back for the minutes, so knowing is the only remedy. **A count of bars can never answer it:** 375-a-day arithmetic is wrong in both directions, because Muhurat trades one hour in the afternoon, a disaster-recovery Saturday has a two-hour hole by design, and five pre-2025 Diwali sessions have a length this build does not know — counting calls all three a loss and a real loss indistinguishable from them. **The range is `Day::new(y, m, 1)`..`end_of_month`, NOT the first and last stored bar**, which is the difference between an audit and a tautology: a month whose first four trading days never landed would otherwise report a clean interior and a perfect score, the missing days falling outside a range they themselves defined. Unreadable records are counted BESIDE the verdict, because a bar `bars::page` could not read would otherwise be scored as a hole — "the file is damaged" and "the vendor is missing minutes" are opposite faults wanting opposite fixes. **NOT O(1) and not claimed to be**: one forward pass over the month's minutes (~44,640) and one over its bars (≤~11,625), which is none of the five operations §3 rule 4 names, is bounded by a constant month, and is asked once per instrument-month by an operator rather than inside a loop; `MAX_GAPS` bounds the answer's size and `truncated` says so out loud | `api::server::the_gaps_route_finds_a_missing_minute_and_calls_the_weekend_no_loss` — builds the month minute-by-minute from `calendar::kind_of` rather than from a date the test believes something about, so an amended holiday table cannot fail it for being right; asserts a complete month scores `lost_minutes:0` with the weekend still REPORTED as `closed`, then removes one middle minute and asserts the run names which minute on which day, with `expected` unchanged across both stores — the fixed point a hole is measured against. Its own first draft filled one day and asserted zero losses against a month owing 22: the route answered `expected:8250, held:375, lost:7875`, correctly | ✓ |

P-39 is D-0075's other half. The 21 emit sites landed with the telemetry
dependency and not one had a test that called the production helper and then
looked in the file; each could have been deleted outright and every gate stayed
green, which was demonstrated by suppressing `ingest::note_landed`, renaming
`pull.http` to `pull.https` and dropping `work::note_narrowed`'s quiet guard —
the first two fail this test naming the file and line, the third fails its
absence half. One test and not twenty, because `telemetry::install` writes a
per-process `OnceLock` and refuses a second call, so a test binary gets exactly
one sink. The floor is `Trace` and that is load-bearing: seven of these targets
speak below `Info`, so at the default floor the test would assert nothing while
appearing to pass.

**The twenty-first is `crates/pull/src/ssm.rs:678`, `pull.ssm`, and it is not
covered.** That `emit` is written inline in the body of the `async` function
that POSTs to Parameter Store, after `send().await` and after the status check —
there is no `note_*` helper to drive and no host to substitute, so reaching it
needs a live signed HTTPS call to AWS. `CLAUDE.md` §3 rule 6: named here rather
than left for a coverage report to find. Extracting it into a helper, the shape
every other module in this crate already uses, is what would make it provable.

P-37 and P-38 are D-0070. The grid was anchored at the Unix epoch, so every
day-wide bucket edge fell at UTC midnight — 05:30 **inside** the IST session it
was meant to contain. A daily candle stamped 00:00 IST of D is 18:30 UTC of
D−1, and flooring it to that grid re-stamped it at 00:00 UTC of D−1: every
daily bar moved back one calendar day. `crate::ingest`'s month guard saw it only
where the shift crossed a month boundary and stored a wrong answer silently
everywhere else. P-38 exists because the fix must be provably inert at the rung
that holds the engine's real data.

P-36 added by the gate 12 sweep. The counter half of that module's cost claim
is not a new row: `pull::totp::the_rfc_6238_sha1_vectors_reproduce_exactly`
already pins it, because RFC 6238's `T = 20,000,000,000` vector is a counter
with four leading zero bytes and reproducing it requires the message to be eight
big-endian bytes for every counter.

## The manifest — layer 13

Added by D-0035. `docs/07-o1-architecture.md` layer 13: counters, never scans.
Every row here is proven by a test that runs today.

| # | Must hold | Proven by | |
|---|---|---|---|
| M-01 | An entry survives its own 64-byte image, field for field, on every exchange and segment code | `pull::unit::an_entry_round_trips_through_its_image` | ✓ |
| M-02 | The checksum's domain is exactly bytes `0..60`; covering the checksum itself is a different number and is pinned against a hardcoded value | `pull::unit::the_covered_domain_is_the_image_minus_its_checksum` | ✓ |
| M-03 | A flipped bit in any of an entry's 512 bits is detected | `pull::unit::a_flipped_bit_in_any_entry_byte_is_detected` | ✓ |
| M-04 | A torn header commit never reports a count that was not committed — every one of the 65 prefixes gives the previous generation or the new one | `pull::unit::a_torn_header_commit_never_reports_an_uncommitted_count` | ✓ |
| M-05 | A header that became durable before the entries it counts falls back one generation rather than condemning the file — whether the entry region is short, **or its committed bytes are zeroed, garbage or half-written** | `pull::unit::a_header_published_before_its_entries_falls_back_a_generation` | ✓ |
| M-06 | A header counter that disagrees with the entries it counts is refused; the counter is checked against the thing it counts, once, on load | `pull::unit::a_counter_that_disagrees_with_its_entries_is_refused` | ✓ |
| M-07 | A key whose row count or last timestamp goes backwards is refused, on write and on load — and one that repeats its last timestamp exactly is accepted, on both | `pull::unit::a_row_count_that_went_backwards_is_refused` · `pull::unit::a_key_whose_history_goes_backwards_on_disk_is_refused` · `pull::unit::a_key_that_repeats_its_last_timestamp_is_accepted` | ✓ |
| M-08 | An entry's address is arithmetic, and the ordinal is bounded rather than the product checked | `pull::unit::the_offset_of_an_entry_is_arithmetic` | ✓ |
| M-09 | The exchange and segment codes on disk are frozen against hardcoded numbers, never re-derived from the enum | `pull::unit::the_exchange_and_segment_codes_are_frozen` | ✓ |
| M-10 | A manifest whose header names another vendor is refused by name; the file name and the header must agree | `pull::unit::a_manifest_for_another_vendor_is_refused` | ✓ |
| M-11 | The three counters are maintained on every write, and a refused record leaves them untouched | `pull::unit::the_counters_are_maintained_on_write` | ✓ |
| M-12 | Every counter refuses to wrap — generation, entry count, key count and row total, on write and on load | `pull::unit::the_header_counters_refuse_to_wrap` · `pull::unit::the_row_total_refuses_to_wrap` · `pull::unit::the_load_time_row_total_refuses_to_wrap` | ✓ |
| M-13 | A slot that is not this format's header is refused by name, and a commit found in the wrong slot is not a candidate | `pull::unit::a_slot_that_is_not_a_header_is_named` · `pull::unit::a_short_or_misplaced_header_region_is_refused` | ✓ |
| M-14 | A genesis census exists only for a file with nothing in it; a writer cannot start a new census over one that already holds months | `pull::unit::the_only_genesis_is_an_empty_file` | ✓ |
| M-15 | Every declared bound is exact — the value at the limit is accepted and the first one past it is refused, on `advance`, `validate`, `commit`, the line bound and the field bound | `pull::unit::every_declared_bound_is_exact_at_the_limit` | ✓ |
| M-16 | A generation recovered by stepping over a damaged one is never silent: the census names what it stepped over | `pull::unit::a_corrupt_header_slot_is_named_by_the_census_that_survives_it` | ✓ |
| M-17 | The loaded index is reserved from the committed entry count, not from the region's byte length, so the reservation is proportional to the census and never to the file | `pull::unit::the_loaded_index_is_reserved_from_the_census` | ✓ |
| M-18 | The reservation is the census doubled and never past `MAX_ENTRIES`; from half the ceiling upward it covers every append `advance` will ever accept, so no append can rehash at all | `pull::unit::the_reservation_is_capped_at_the_design_ceiling` | ✓ |
| M-19 | A loaded index carries free room for at least `n_valid` further appends, and none of them rebuilds the table — checked at a census of `7·2^8`, where a table reserved to exactly the census has zero free slots | `pull::unit::a_loaded_index_carries_headroom_for_the_appends_after_it` | ✓ |
| M-20 | A whole manifest survives its own image, through the reader unchanged — zero entries, one, several, and a key recorded twice | `pull::unit::a_whole_manifest_survives_its_own_image` | ✓ |
| M-21 | The image puts every byte at the address the reader computes for it: the slot at `generation % 2`, entry *i* at `offset_of(i)`, and the length at the commit's own `durable_through` | `pull::unit::the_image_puts_every_byte_where_the_reader_looks_for_it` | ✓ |
| M-22 | A half-installed image is refused by name, never believed | `pull::unit::a_half_installed_image_is_refused_by_name` | ✓ |
| M-23 | The image carries the entry LOG, in the order it was recorded, not the index over it — an older entry for a key is never compacted away | `pull::unit::the_log_is_the_entry_region_in_order` | ✓ |
| M-24 | A census that loaded degraded images as its **repair**, and the repaired file reloads clean | `pull::unit::a_degraded_census_images_as_its_repair` | ✓ |
| M-25 | A version-1 census still reads after version 2 exists — every counter, every entry, at version 1's own 64-byte stride — and every one of its closes reads back as *not recorded*, never as zero | `pull::unit::an_old_version_manifest_still_reads_after_version_2_exists` | ✓ |
| M-26 | A version-2 entry round-trips both closes exactly, and a pair that is half-recorded or not a price is refused rather than half-believed | `pull::unit::a_version_2_entry_round_trips_both_closes_exactly` | ✓ |
| M-27 | An absent close is distinguishable from a close of zero — in the type, in the sixteen bytes, and across a whole census; and "not held at all" is a third answer, not the same one | `pull::unit::an_absent_close_is_not_a_zero_close` | ✓ |
| M-28 | Rebuilding a census from the same rows is byte-identical, and a version-1 census converges: the upgrade is paid once and imaging the result again changes not one byte | `pull::unit::rebuilding_a_census_from_the_same_rows_is_byte_identical` | ✓ |
| M-29 | The stride, the header fields and the entry's two halves are the geometry `docs/02-store-format.md` §11 states, pinned against hardcoded numbers rather than against the constants themselves | `pull::unit::the_manifest_geometry_is_what_the_format_document_says` | ✓ |
| M-30 | A version-2 entry opens with a version-1 entry, byte for byte, which is why one decoder serves both versions' base fields | `pull::unit::a_version_2_entry_opens_with_a_version_1_entry` | ✓ |
| M-31 | A flipped bit in any of a version-2 entry's 1,024 bits is detected — every byte is covered by exactly one of its two checksums | `pull::unit::a_flipped_bit_in_any_version_2_entry_byte_is_detected` | ✓ |
| M-32 | A known version keeps its stride after a new one exists, proven through the production resolver against a table already holding a third version | `pull::unit::a_known_manifest_version_keeps_its_stride_after_a_new_one_exists` | ✓ |
| M-33 | Every declared layout states one stride in both widths, a degenerate row is refused **by field**, and the stride bound is exact at the limit | `pull::unit::every_known_manifest_layout_states_one_stride_in_both_widths` | ✓ |
| M-34 | A month whose file **is** the batch just written takes its closes from that batch and issues no read; every other shape falls to the two positional reads rather than recording a suffix's first close as the month's | `pull::ingest::tests::a_virgin_month_takes_its_closes_from_the_batch_it_just_wrote` | ✓ |
| M-35 | A version-1 census on disk is left byte for byte alone by a run that records nothing, and is rewritten **whole** at version 2 by the first run that records anything — never appended to at a stride the file does not have | `pull::census::a_version_1_census_upgrades_on_the_first_run_that_records_anything` | ✓ |

M-25 through M-35 added by D-0067. **M-34 is a claim about a branch, not about a
stopwatch**, and that is deliberate: `closes_in_hand` returning `Some` is the
only way the two positional reads are not reached, so proving the arm is not
taken is what proves the reads are not issued. A timing test could not prove it
and a mock would only prove the mock.

**M-25 is the migration requirement.** 43,422 entries across two vendors are on
disk in version 1's geometry, `CLAUDE.md` §3 rule 8 does not admit mutating them
in place, and reading them at version 2's stride would decode every second entry
as the tail of the one before it. Two further rows guard the same seam from the
other side: M-32 proves a new version cannot alter what an old one resolves to,
and `pull::unit::two_slots_naming_two_versions_are_each_walked_at_their_own_stride`
covers the crash in the middle of an upgrade, where the two header slots name
two versions and each must be validated against its own capacity.

M-18 and M-19 added by D-0040. **M-19 stops one short of an unconditional
claim, deliberately.** Its last assertion is that the append *after* the
headroom does grow the table, so the row cannot be read as "append never
rehashes": past `n_valid` new keys the cost is amortised O(1) with an
`O(n_keys)` worst case, and `docs/06-limits.md` §23 says what removing that
last arm would cost. The same test also refuted a claim written into
`Manifest::record` while D-0040 was being written — that an update to a key
already held can never grow the table. `HashMap::insert` asks for a slot before
it looks the key up, so on a full table it grows anyway; the test asserts the
update-inside-the-headroom case that does hold and the one past it that does
not.

M-14 through M-17 added by D-0036, each closing a defect that the tests above
could not see. M-05 and M-07 were restated there rather than replaced: both
claimed more than the code did.

## Instrument identity and the vendor merge

Added by D-0024. Every row here is proven by a test that runs today.

| # | Must hold | Proven by | |
|---|---|---|---|
| I-01 | A row on the equity segment that is not a share is declined, and the share of the same name is kept — whichever order the file lists them in | `core::vendor::the_cholafin_bond_is_declined_and_the_cholafin_share_is_kept` | ✓ |
| I-02 | Every measured debt and fund class of either vendor is declined by name, not by falling through a default | `core::vendor::every_measured_debt_and_fund_class_is_declined_by_name` | ✓ |
| I-03 | An index row is never gated on a listing class it does not have, from either vendor | `core::vendor::an_index_row_survives_the_gate_from_both_vendors` | ✓ |
| I-04 | The listing class is trimmed before it is read, so padding cannot decline every share in a file | `core::vendor::dhans_class_column_is_trimmed_before_it_is_read` | ✓ |
| I-05 | An SME listing is declined under its **own** reason and never lumped in with debt | `core::vendor::the_equity_board_is_kept_and_the_sme_board_is_declined_separately` | ✓ |
| I-06 | An ISIN is refused unless its ISO 6166 check digit verifies | `core::isin::the_one_real_row_with_a_bad_check_digit_is_refused` | ✓ |
| I-07 | The one real ISIN that fails its check digit never reaches the parse, because the equity gate declines it first | `core::vendor::the_sdl_with_the_bad_check_digit_never_reaches_the_isin_parse` | ✓ |
| I-08 | A kept equity carries a parseable ISIN or the row is an error; it is never a quiet `None` | `core::vendor::a_kept_equity_must_carry_a_parseable_isin` | ✓ |
| I-09 | An index carries no ISIN, and no sentinel is invented for one | `api::merge::an_index_carries_no_isin_and_still_merges_on_identity` | ✓ |
| I-10 | Two vendors giving one key two different ISINs is **one** key and one loud line naming both; neither is dropped and neither wins | `api::merge::a_cross_vendor_isin_conflict_is_reported_and_neither_side_is_dropped` | ✓ |
| I-11 | A series suffix is stripped only when a second vendor confirms the identity by ISIN | `api::merge::a_suffixed_symbol_merges_only_when_the_isin_confirms_it` | ✓ |
| I-12 | An unconfirmed suffix is left exactly as the vendor wrote it | `api::merge::an_unconfirmed_suffix_is_left_exactly_as_the_vendor_wrote_it` | ✓ |
| I-13 | A trailing dash that is not the row's own series is never stripped, so `BAJAJ-AUTO` survives | `core::vendor::a_dash_that_is_not_the_rows_own_series_is_never_stripped` | ✓ |
| I-14 | Every column a vendor's reader needs is required by name, and a missing one is refused and named | `api::master::every_column_a_vendor_needs_is_required_and_named_when_absent` | ✓ |
| I-15 | The binary's entry point is measured by running it, not exempted from the coverage gate | `api::binary::the_binary_reports_what_it_read_and_exits_zero` | ✓ |
**The seven rows below were appended as I-16 … I-22, and those seven ids were
already taken** by the equity-gate section that follows. **Renumbered to I-31 …
I-37 by D-0045**, taking the next free ids after I-30. No file outside this one
cited either block — checked before the renumber — so nothing else moves.

| # | Must hold | Proven by | |
|---|---|---|---|
| I-31 | A vendor field wider than `MAX_FIELD_BYTES` is refused **before** anything reads it, whichever of the ten fields it is | `core::vendor::an_over_wide_field_is_refused_whichever_field_it_is` | ✓ |
| I-32 | The width bound is the first byte that is too many and not one before, and it never substitutes for the parsers below it | `core::vendor::the_bound_is_the_first_byte_that_is_too_many_and_not_one_before` | ✓ |
| I-33 | The widest value measured in either real master still passes the width gate untouched | `core::vendor::a_row_of_ordinary_width_passes_the_gate_untouched` | ✓ |
| I-34 | The width gate runs before the test-marker scan and does not shadow it | `core::vendor::the_test_marker_scan_still_declines_a_real_test_listing` | ✓ |
| I-35 | A master larger than the reader holds is refused from its size, before it is read into memory | `api::master::a_master_larger_than_this_reader_holds_is_refused_before_it_is_read` | ✓ |
| I-36 | A row longer than the reader splits is named at its line number and never split | `api::master::a_row_longer_than_this_reader_splits_is_named_and_never_split` | ✓ |
| I-37 | An over-wide field makes the row an error that names the field; it is never a silent keep | `api::master::a_field_wider_than_core_will_read_is_an_error_and_not_a_silent_keep` | ✓ |
| I-39 | Hashing a `Symbol` — and hashing a whole `InstrumentKey` — feeds a hasher the **same number of bytes** at one character as at `SYMBOL_CAPACITY`, so the dedup probe's cost does not depend on what a vendor sent. Counted with a hasher that records only how much it was fed, so it is exact and machine-independent; `core::symbol::padding_never_affects_identity` beside it proves the hash's *identity* and says nothing about its cost | `core::symbol::hashing_feeds_the_same_number_of_bytes_however_long_the_input_was` | ✓ |

I-38 is taken, by the NSE series tables section near the end of this file.
I-39 added by the gate 12 sweep: `crates/core/src/symbol.rs` and
`crates/core/src/instrument.rs` had both rested an O(1) dedup claim on the field
being fixed-width, and nothing measured it in either direction.

## The equity gate after D-0025, and the refusal after D-0026

Added by D-0025, D-0026 and D-0027. Every row here is proven by a test that
runs today.

| # | Must hold | Proven by | |
|---|---|---|---|
| I-16 | The three measured series tables are sorted, mutually disjoint and non-empty, so `binary_search` cannot return garbage and no code can get two verdicts | `core::vendor::the_measured_series_tables_are_sorted_disjoint_and_complete` | ✓ |
| I-17 | A cash-equity row whose series this engine has never seen gets its **own** reason, never the one a debenture gets | `core::vendor::an_unrecognised_series_is_its_own_loud_reason_never_a_bond` | ✓ |
| I-18 | The unrecognised **code itself** reaches the operator, not only a count of it | `api::master::an_unrecognised_series_is_recorded_under_the_code_itself` | ✓ |
| I-19 | A fund plan is declined from **both** vendors, and a genuine ETF on the equity board is still kept | `core::vendor::a_mutual_fund_plan_is_declined_from_both_vendors_on_one_series_alphabet` | ✓ |
| I-20 | The surveillance and partly-paid equity series are kept as equity, from both vendors, and never labelled debt | `core::vendor::the_surveillance_and_partly_paid_equity_series_are_kept_not_called_debt` | ✓ |
| I-21 | A declined row carries its ISIN as evidence, and an unparseable one is neither an error nor a silent substitute | `core::vendor::a_declined_row_carries_its_isin_as_evidence_for_the_cross_check` | ✓ |
| I-22 | One vendor keeping an ISIN another declined is a named disagreement, and the instrument is not dropped | `api::merge::one_vendor_keeping_what_another_declined_is_a_named_disagreement` | ✓ |
| I-23 | A decline about the venue is never mistaken for a disagreement about the paper | `api::merge::a_decline_about_the_venue_is_not_a_disagreement_about_the_paper` | ✓ |
| I-24 | A row with fewer fields than its columns need is unreadable and names the shortfall; it is never a routine decline | `api::master::a_row_with_too_few_fields_is_unreadable_and_names_the_shortfall` | ✓ |
| I-25 | Every distinct parse failure reaches the operator with its count and the first line that hit it | `api::master::unreadable_rows_are_grouped_by_reason_with_the_first_line_that_hit_it` | ✓ |
| I-26 | A searched page still says that a vendor was never read | `api::server::a_searched_page_still_says_a_vendor_was_never_read` | ✓ |
| I-27 | A vendor that was never read makes the status, the exit code and `/health` all say so | `api::binary::the_binary_exits_non_zero_when_a_vendor_was_never_read` · `api::server::health_answers_503_when_a_vendor_was_never_read` | ✓ |
| I-28 | An ISIN conflict refuses the universe rather than logging it and continuing | `api::server::an_isin_conflict_reaches_the_report_and_the_page` | ✓ |
| I-29 | An unrecognised listing class degrades the run, while a routine bond does not | `api::server::an_unrecognised_listing_class_names_the_code_and_degrades_the_run` | ✓ |
| I-30 | A universe member only one vendor named is counted separately from one two vendors confirmed, and named | `api::merge::the_census_separates_what_two_vendors_confirmed_from_what_one_asserted` | ✓ |

## The instrument universes

Added by D-0029. `crates/core/src/universe.rs` shipped with none of these rows;
CI gate 10 walks rows→tests and never tests→rows, so the build stayed green
while `CLAUDE.md` §9 was violated.

| # | Must hold | Proven by | |
|---|---|---|---|
| U-01 | Both constituent lists are sorted and unique, so `binary_search` is valid | `core::universe::both_lists_are_sorted_and_unique_so_binary_search_is_valid` | ✓ |
| U-02 | No two universes share a bit, so an append-only bitset stays safe (`CLAUDE.md` §3.8) | `core::universe::bits_are_distinct_powers_of_two` | ✓ |
| U-03 | The list lengths are the measured ones, and no exchange test instrument is ever a member | `core::universe::the_counts_are_the_measured_ones` | ✓ |
| U-04 | No SME ticker is in either universe — the claim `Skip::SmeBoard` declines 1,117 real shares on | `core::universe::no_measured_sme_ticker_belongs_to_either_universe` | ✓ |
| U-05 | An index is its own universe and a live derivative is in none | `core::universe::an_index_is_its_own_universe_and_a_live_derivative_is_in_none` | ✓ |
| U-06 | The four published NIFTY tiers nest — every 50 name is in the 100, every 100 in the 200, every 200 in the 500, every 500 in the Total Market — and the converse does not hold, so the bits carry information | `core::universe::the_published_tiers_nest_one_inside_the_next` | ✓ |
| U-07 | `INDEX`, `FNO` and `TOTAL_MARKET` are still bits 0, 1 and 2, and the four tiers appended at 3–6 are pinned as numbers (`CLAUDE.md` §3.8) | `core::universe::the_three_original_bits_never_moved` | ✓ |
| U-08 | **Six ISIN arrays sit beside the six name arrays, the same length, aligned index for index.** They were transcribed from FIVE different published files, so every symbol appearing in more than one carries the same ISIN in all of them, and no ISIN appears twice within one array — a row that slipped by one anywhere disagrees at the first shared symbol. The overlap is pinned as a number, 1,807 positions naming 749 distinct symbols, so the cross-check cannot go vacuous | `core::universe::the_isin_arrays_are_positionally_aligned_with_the_names` | ✓ |
| U-09 | **Every transcribed ISIN is `INE` + nine, twelve characters, and passes the ISO 6166 check digit** — computed by `Isin::new`, so no second copy of that arithmetic exists to disagree. A mistyped character is a build failure rather than an identifier that looks right and points at nothing | `core::universe::every_transcribed_isin_is_well_formed` | ✓ |
| U-10 | **The six positions with no ISIN are NAMED, not counted.** Five indices — `NIFTY`, `BANKNIFTY`, `FINNIFTY`, `MIDCPNIFTY`, `NIFTYNXT50` — which no numbering agency issues one for, and `AGL`, which the exchange's own file has no row for and which stays UNVERIFIED (§11). "Six are absent" would still pass if a different six went absent | `core::universe::the_absent_isins_are_exactly_these_six_names` | ✓ |
| U-11 | **The two placeholder scrips are malformed by construction and reached nothing.** `DUM510W01014` and `DUM256C01024` are shown to fail the ISO 6166 check digit by running the parser on them, and neither `DUMMYINXGN` nor `DUMMYTRVN` nor either pseudo-ISIN appears in any array or ever resolves | `core::universe::the_placeholder_scrips_are_malformed_and_are_not_here` | ✓ |
| U-12 | **A membership probe returns the index the table was built from, for all 1,813 members of all six tables.** `contains` is `position(..).is_some()`, so there is one probe loop and not two; a table that found a member but pointed at another slot would hand a caller a different company's ISIN, and the ordinal is asserted symbol by symbol rather than sampled | `core::universe::a_position_probe_finds_the_index_the_table_was_built_from` · `core::universe::nse_isin_answers_from_the_exchanges_own_column` | ✓ |

**U-06 is what lets `of_equity` set five bits for one symbol.** Added by
D-0089. A NIFTY 50 name is also a NIFTY 100, 200, 500 and Total Market name, and
the function sets every one of those bits — but it does not *derive* them. It
probes all six tables and reads each bit from the file that publishes it, so
that no bit is ever set on another file's authority. Nesting is then a property
of the DATA, and this row is where the data is checked: symbol by symbol, in
the direction that can fail, across all 750 members of the four tiers.

The check runs both ways on purpose. If only containment were asserted, four
copies of the same list would pass it. So the row also asserts that a Total
Market name outside the NIFTY 500 exists and does **not** claim the NIFTY 500
bit — without that, `of_equity` could return every bit for every member and
still be green.

The direction matters more than it looks. NSE rebalances these indices
semi-annually and the constants are snapshots (`docs/00-charter.md` §4c). A
rebalance that moved one name out of the 500 while leaving it in the 200 would
break nesting for exactly one symbol, and this row fails naming that symbol
rather than letting `of_equity` answer confidently and wrongly. That is the
`CLAUDE.md` §4 "degrade loudly and name the reason" row, applied to data that
arrives from outside.

**U-08 through U-12 are D-0122, and they are the reason `of_equity`'s answer
can now be joined on.** D-0089 transcribed the exchange's constituent files and
kept only the `Symbol` column, so no NSE-issued ISIN existed in this build and a
constituent's identity resolved through the merged vendor universe by symbol
first — the hop `docs/06-limits.md` §62 recorded. D-0122 carries the `ISIN Code`
column those same files have always had, positionally aligned beside the names,
and `core::universe::nse_isin` answers from it in constant time. **D-0125 wired
the consumer**: `api::constituents` calls it, the symbol step is deleted rather
than demoted, and CJ-02 and CJ-05 now state what the join keys on rather than
what it had to hop through to get there.

**U-08 is the row that carries the weight, and it checks what can actually be
checked.** "Index *i* names the same instrument in both arrays" is a claim about
a file that is not in this repository — `CLAUDE.md` §2 allows no tracked `.csv`
— so it cannot be re-derived here. What can be, and is stronger than a length
equality: the six arrays came from five separately published files, they overlap
heavily because the tiers nest (U-06), and every shared symbol must carry the
same ISIN in all of them. 1,058 of the 1,807 filled positions are a second,
third, fourth or fifth opinion on a symbol another file already named. Measured:
zero disagreements.

**U-10 is a measurement written as a set rather than as a total.** Five of the
six absences are correct answers — an index is not a security and is issued no
ISIN. The sixth is `AGL`, which is a real gap, and the row names it rather than
letting it hide inside a count. Filling that position from a broker master would
have been the `CLAUDE.md` §3 rule 1 invention this whole transcription exists to
remove, and filling it with `GRINDWELL`'s ISIN — the name the exchange's file
holds where this array holds `AGL` — would have made the wrong row *join*.

**U-07 is `CLAUDE.md` §3 rule 8 written as an assertion rather than a promise.**
`Universe::bits()` is stamped onto merged rows and rendered on the page. Had
D-0089 inserted a tier at bit 2 instead of appending at bit 3, every value
already written would silently have meant something else, and nothing in the
type system would have objected. All seven positions are pinned as literal
numbers so the next insertion cannot renumber them either.

## The ingest and store pages

Added by D-0038. Every row here is proven by a test that runs today.

| # | Must hold | Proven by | |
|---|---|---|---|
| A-01 | Starting a pull is a `POST`; a `GET` on either submit route answers 405 without reaching a parser | `api::server::the_server_answers_every_route_and_then_shuts_down_gracefully` | ✓ |
| A-02 | An expiry that has not passed is refused, and `today` itself counts as live | `api::ingest::an_expired_contract_is_accepted_and_a_live_one_can_never_be` · `api::server::the_fno_form_cannot_request_a_live_contract_over_http_either` | ✓ |
| A-03 | The expiry input's `max` is the day before today, so a live contract is not even offerable — and the parser refuses one anyway | `api::render::the_ingest_page_carries_two_forms_and_never_a_get_that_starts_a_pull` | ✓ |
| A-04 | A date field is `YYYY-MM-DD` or it is refused naming the field and the value; no ambiguous format is ever guessed at | `api::ingest::a_date_field_is_refused_by_name_whichever_way_it_is_wrong` | ✓ |
| A-05 | A window is inclusive at both ends, a backwards one is refused rather than swapped, and a one-day window is legal | `api::ingest::a_window_is_inclusive_both_ends_and_a_backwards_one_is_refused_not_swapped` | ✓ |
| A-06 | The window length bound is the first day that is too many and not one before | `api::ingest::the_window_is_bounded_at_the_boundary_and_the_bound_is_tight` | ✓ |
| A-07 | The vendor's non-inclusive `toDate` is shown on the receipt as the day after the operator's last day, and said to be so | `api::server::a_valid_window_is_echoed_with_the_wire_date_and_still_starts_nothing` | ✓ |
| A-08 | No capture counter renders `0` while nothing is running: an unmeasured value is `—`, under a block naming what is absent | `api::render::a_capture_that_is_not_running_shows_dashes_and_a_named_reason` | ✓ |
| A-09 | Every reason the session filter counts is named on the page by `DropReason::label`, measured or not, and its bar width is integer arithmetic | `api::render::a_recorded_run_reports_real_counts_with_integer_bars` | ✓ |
| A-10 | An absent manifest renders as a named absence at a named path; it is never a 500 and never zeros | `api::census::an_absent_manifest_names_the_path_and_is_never_zeros` · `api::server::the_store_page_renders_an_absent_manifest_rather_than_failing` | ✓ |
| A-11 | A manifest whose counters are zero reads as zero and is not loud — an empty store is a real answer, not an absence | `api::census::a_manifest_whose_counters_are_zero_reads_as_zero_and_is_not_loud` | ✓ |
| A-12 | A manifest that will not load is loud and carries the manifest's own refusal, never a generic failure | `api::census::a_manifest_that_will_not_load_is_loud_and_names_the_refusal` | ✓ |
| A-13 | The coverage grid is addressed by ordinal arithmetic, so a page builds only the rows it shows | `api::census::the_grid_is_addressed_by_arithmetic_and_pages_without_building_the_rest` | ✓ |
| A-14 | A page past the end of the grid clamps to the last page rather than erroring or emptying | `api::server::the_store_page_reads_zero_as_zero_and_pages_past_the_end_by_clamping` · `api::server::the_store_grid_pages_when_it_is_larger_than_one_page` | ✓ |
| A-15 | A nav entry for a page that does not exist is shown disabled, never hidden and never linked | `api::server::the_server_answers_every_route_and_then_shuts_down_gracefully` | ✓ |
| A-16 | The coverage grid's axis is the store's own vocabulary, so a manifest holding only F&O futures still reaches the grid — rows held and cells held cannot disagree about an empty store | `api::census::a_store_of_nothing_but_futures_still_reaches_the_grid` | ✓ |
| A-17 | A census that is absent or unreadable contributes no series to the axis and invents none; two vendors holding one series contribute one row | `api::census::an_unloadable_census_adds_nothing_to_the_axis` | ✓ |
| A-18 | The two swept series are always on the axis, held or not, so a fresh install names what it is missing | `api::census::a_store_of_nothing_but_futures_still_reaches_the_grid` · `api::server::a_site_with_no_universe_still_shows_the_two_instruments_that_matter` | ✓ |
| A-19 | A series renders as the store names it, and `Series::of` and `Series::at` are inverses, so the axis and the probe are the same values | `api::census::a_series_reads_back_as_the_store_names_it` | ✓ |
| A-20 | **The absolute this row used to claim was abandoned by D-0060; this is what replaced it, and it is checked over more than the old wording ever was.** Exactly one script reaches a browser from this server — the external, deferred `/typeahead.js`, on `/instruments` alone. Every other page's budget is zero, the date picker's included. On **every** page, that one included, there is no inline event handler, no `javascript:` URL, and no `<script` that operator text opened | `api::render::every_page_carries_the_one_script_this_repository_chose_and_no_other` · `api::render::the_injection_reaches_every_page_and_arrives_escaped` · `api::calendar::nothing_the_picker_emits_is_a_script` — **the row read "no page this server emits contains a script" and wore `✓` while `render.rs` line 1055 emitted one.** It was true the day it was written and false the next: D-0052 and D-0053 opened `web/`, `f046b36` landed the type-ahead, and no row was revisited. The name this row used to carry, `the_page_contains_no_script_at_all`, could not be written honestly at any point after that day, so the CLAIM changed and the tick did not move on its own. **It is written here without its crate and module segments on purpose:** spelled in full it is a token gate 10 scrapes out of this row and reports as missing every run, which is the right behaviour for a pending row and the wrong one for an abandoned name. D-0060 is where the abandonment is signed. What is checked now is all seven page functions — `dashboard_page`, `instruments_page`, `pull_page`, `receipt_page`, `store_page`, `bars_page`, `audit_page` — rather than the four that had piecewise assertions, and each is rendered twice: once with ordinary text and once with `"'&><script>` in every field a route, an operator or a vendor can fill, so the count is proof of escaping and not only of absence. Handlers are matched by SHAPE, a word beginning `on` in attribute position followed by `=`, because naming `onclick`, `onload` and `onerror` bans three of about ninety | ✓ |
| A-21 | At most one date panel per form can be open, so two panels cannot overlap at any viewport | `api::calendar::the_latch_is_a_radio_so_two_panels_cannot_be_open_at_once` | ✓ |
| A-22 | Exactly one pane of the picker is shown at a time, chosen by which radios are checked and by no extra control | `api::calendar::three_panes_are_emitted_and_the_year_pane_is_the_one_with_no_prerequisite` | ✓ |
| A-23 | A month arrow steps exactly one month and never crosses a year boundary; the two that would are inert spans, not labels | `api::calendar::the_arrows_step_one_month_and_do_not_cross_a_year_boundary` | ✓ |
| A-24 | No control in the picker is `required`, because an unfocusable `required` control blocks submission in silence | `api::calendar::no_control_in_the_picker_is_required` · `api::server::the_pickers_do_not_block_submission_and_do_not_close_on_their_own_chrome` | ✓ |
| A-25 | Nothing later than a field's ceiling can be clicked — not a day, and not a month in the ceiling's own year | `api::calendar::the_computed_rules_cover_alignment_the_month_end_and_the_ceiling` | ✓ |
| A-26 | The picker never offers a year no `Day` can hold | `api::calendar::the_offered_span_ends_at_the_cap_and_is_twelve_years_long` | ✓ |
| A-27 | The coverage grid's axis comes out strictly ascending in one stated order whatever order the censuses were read in, and a series two vendors both hold is one row — so a page ordinal names the same instrument after a restart | `api::census::the_axis_is_sorted_and_deduplicated_whatever_order_the_censuses_arrive_in` | ✓ |
| A-28 | `StoreFilter::keeps` searches a haystack bounded by the **type**: a symbol one byte past `SYMBOL_CAPACITY` is refused at construction, so no longer haystack can ever be handed to it | `api::census::the_symbol_arm_searches_a_haystack_the_symbol_type_bounds` | ✓ |
| A-29 | The unfiltered `/store` page **borrows** the held table — the same allocation, not an equal copy — and only a filter that narrows owns its selection | `api::census::the_unfiltered_page_borrows_the_table_and_copies_nothing` | ✓ |
| A-30 | A month's percentage change is integer basis points — 1.25% is `125` — computed with no float at any step, and a month that closed where it opened is a real `0` rather than an unknown | `api::server::a_month_that_moved_1_25_percent_is_125_basis_points_and_a_flat_month_is_a_real_zero` | ✓ |
| A-31 | Basis points round **half away from zero in both directions**, so a gain and its mirror-image loss print the same magnitude; the fixture is an exact 312.5 bp tie, not a rounding accident | `api::server::basis_points_round_half_away_from_zero_in_both_directions` | ✓ |
| A-32 | Basis points are carried as `i64` because `i32` overflows on an ordinary price: a base of ₹0.01 and a close of ₹2,147.50 is 2,147,490,000 bp | `api::server::basis_points_do_not_fit_i32_and_an_ordinary_price_proves_it` | ✓ |
| A-33 | A base of zero paisa is refused rather than divided by, and a move whose ×10,000 leaves `i64` is refused rather than wrapped — both bounds asserted at the boundary, not near it | `api::server::a_base_of_zero_paisa_is_refused_rather_than_divided_by` · `api::server::a_move_that_leaves_i64_is_refused_rather_than_wrapped` | ✓ |
| A-34 | **No instrument that a corporate action can re-base renders a percentage while no threshold is sourced.** Cash and F&O are refused in *both* columns whatever their closes say and whether or not the neighbouring month is held; only an index, which never splits, renders — and the permanent reason outranks every temporary one | `api::server::no_equity_month_renders_a_number_while_no_threshold_is_sourced` | ✓ |
| A-35 | An unknown percentage is `null` and one of five named reason codes on the wire — never `0`, never a missing key, and never a number and a reason together | `api::server::every_row_carries_a_number_or_a_named_reason_and_never_both` · `api::server::a_month_with_no_close_recorded_is_unknown_and_never_zero` | ✓ |
| A-36 | An instrument's first held month renders its own change and `no_earlier_month` for the previous one; a census gap is **not** searched past — the row one month back is probed and that is all | `api::server::an_instruments_first_month_has_a_change_and_no_previous_change` · `api::server::a_row_and_its_neighbour_cost_two_probes_and_no_file` | ✓ |
| A-37 | `/store.json` prices a row and its neighbour from the census alone: the fixture's manifest path does not exist on disk, so no bar file can have been opened | `api::server::a_row_and_its_neighbour_cost_two_probes_and_no_file` | ✓ |
| A-41 | `/instruments.json` carries **every** universe a row is in as the `universes` array — the four NIFTY tiers included — while the shipped `universe` string stays frozen at `index`, `fno`, `ntm`, those joined by `+`, or `other`. A bit alone in the new range renders `other` in the frozen field and its own token in the array, and both fields are generated from one spelling table so they cannot disagree about a bit's name | `api::server::the_wire_carries_every_membership_and_the_frozen_field_never_widens` | ✓ |
| A-42 | The catalogue's pill filter, its pill counts, its dashboard figures and its tracked scope are **inert** under the appended bits: a row carrying all seven selects, counts and pages exactly as the same row carrying three. The universe *column* is the deliberate exception — it orders by `universe.bits()`, so an appended membership visibly reorders it | `api::catalog::the_pill_filter_is_pinned_to_three_universes` · `api::catalog::the_universe_column_orders_by_every_bit_including_the_appended_ones` | ✓ |
| A-43 | **A vendor that names its refusal decides the ladder, and `NotEntitled` is an arm of its own.** The name outranks the status — including when they disagree, and including the untyped `Invalid_Authentication` body marker — and an unentitled key is never reported as a dead session. A feed with no declared contract takes the status path unchanged | `api::server::a_vendor_that_names_its_refusal_decides_the_ladder` | ✓ |
| A-44 | The vendor-named path and the status path call **one** rate ladder and **one** backend ladder, asserted by equality against the status path rather than by repeating the waits — so a named rate refusal under a 500 still takes the rate ladder | `api::server::the_named_path_and_the_status_path_share_one_ladder` | ✓ |

**A-41 and A-42 are one decision seen from both ends,** added by D-0090. D-0089
landed NIFTY 50 / 100 / 200 / 500 as `Universe` bits 3–6 and wired none of it,
so four rows on `/` and `/db` shipped disabled with "no endpoint carries this
membership" written on them. A-41 is the wire finally saying what the data
knows; A-42 is the promise that saying it moved nothing that was already
working.

The pair exists because "add a field" and "do not disturb the old one" are
different claims and only one of them is obvious. A single widened `universe`
string would have satisfied every browser that splits on `+` and broken every
reader that compares or bookmarks the whole value — the failure would have been
in somebody else's page, days later, with nothing in this repository red. So
the frozen field is asserted at the one input that can tell pinning apart from
coincidence: a bare `NIFTY_50`, a bit with no `ntm` beside it, which no real
constituent ever is.

A-42's exception is stated rather than repaired. D-0089 recorded that the four
new bits "change no byte of any response"; that is true of every rendered cell
and false of `?sort=universe`, because `Lead::Bits` sorts on the bitset itself.
Masking it back to three bits would order the column by a membership the row no
longer has — quiet and wrong, against visible and right.

A-27 through A-29 added by the gate 12 sweep. All three are properties
`crates/api/src/census.rs` already argued for at length in prose and none of
them had a test: the axis's sort and dedup are one line each, and the `Cow`
borrow could have been deleted with every assertion in that file still passing,
because the two arms are equal by value and differ only in what they cost.

## The pull journal — the codec, both halves

`~/.brutex/store/audit/pull.journal` is append-only, so a record whose byte the
reader cannot name is not a rendering bug — it is a row lost for good. Every row
here is proven by a test that runs today.

| # | Must hold | Proven by | |
|---|---|---|---|
| J-01 | Every scope and every outcome survives the trip to its stored byte and back as **itself** — the test walks `Scope::ALL` and `Outcome::ALL` rather than naming variants, so a variant the reader does not know cannot hide behind a list that agrees with it | `api::audit::scopes_and_outcomes_round_trip_through_their_stored_byte` | ✓ |
| J-02 | The code space is **dense from zero and exactly `COUNT` wide**: each byte below `COUNT` names a variant, `COUNT` itself names none, and the codes ascend by one in `ALL` order — so a corrupt or future byte is refused by name instead of read as some other outcome | `api::audit::scopes_and_outcomes_round_trip_through_their_stored_byte` · the `const _: ()` blocks beside `Outcome::of_code` | ✓ |
| J-03 | A clean run that stored nothing reads back off disk as `Empty`, not as `UnknownOutcome { code: 4 }` — the variant that exists to say STORED NOTHING can actually be read | `api::audit::a_run_that_stored_nothing_says_so_instead_of_reporting_success` | ✓ |
| J-04 | **A torn tail is terminal for later appends, not the new origin of every record after it.** The writer measures the journal while holding its append lock and refuses any length not divisible by 256 before writing one byte. It neither truncates nor repairs the interrupted bytes, so the whole records before them stay at their original addresses and append-only history is untouched | `api::audit::a_torn_tail_is_named_and_the_whole_records_before_it_still_read` asserts the existing row remains readable, the new append is refused naming the exact 100-byte tail, and the file is byte-identical before and after the refusal | ✓ |
| J-05 | **Two cooperating writers cannot interleave one journal append.** The journal inode carries an OS advisory exclusive lock across measure, one fixed-stride write and `sync_all`; a second writer is refused loudly and leaves the length unchanged. Dropping the first handle releases the lock, so no stale PID or sentinel can wedge future history | `api::audit::a_held_journal_lock_refuses_a_second_writer_without_appending` | ✓ |

**J-02 is a build gate, not only a test.** `Outcome::code` is a `match self` and
is therefore exhaustive — the compiler forces a new variant to be given a byte.
`Outcome::of_code` is a `match code` ending in `_ => None`, which is total by
construction and which the compiler has nothing to say about. That asymmetry is
how `Empty` came to be written as byte 4 and read back as a fault: the writer
half was compulsory and the reader half was not. `COUNT` closes it by being
load-bearing twice over — it is the length of the `ALL` array and the input to
the assertion that `of_code(COUNT)` is `None` — so teaching `of_code` a new byte
fails the assertion, raising `COUNT` to repair that makes the `ALL` literal the
wrong length, and listing the variant in `ALL` is what puts it into J-01's walk.

**It stops one short of an unconditional claim, deliberately.** A variant given
an arm in `code` and put in *neither* `of_code` nor `ALL` is still not caught.
Closing that needs the compiler to enumerate an enum's variants, which stable
Rust will not do without a macro — and §2 does not trade a language rule for
this. `ALL` is the shortest list left to keep honest.

**The hand-written list was the defect, not a symptom of it.** The round trip
previously named four outcomes and then asserted `of_code(4) == None`, which is
the reader's bug restated as the test's expectation. A list written beside the
code it checks agrees with that code by construction; only a list the code
itself must consume can disagree with it.

## The vendor socket

Added by D-0049 and D-0050. `crates/pull/src/http.rs` is the only code in this
repository that reaches a broker, so every row here is about a bar that must not
be invented and a credential that must not travel.

| # | Must hold | Proven by | |
|---|---|---|---|
| H-01 | Every one of a bar's seven fields is read from the **one** object the descriptor's `envelope` names, so a bar can never be assembled from two different JSON objects | `pull::http::one_bar_can_never_be_assembled_from_two_different_objects` | ✓ |
| H-02 | With no envelope declared, the top level is the only place looked; a wrapped body is refused rather than rummaged through | `pull::http::no_envelope_means_the_top_level_and_nowhere_else` | ✓ |
| H-03 | A declared envelope the answer does not carry is refused naming the key expected **and** the keys present, so a wrong descriptor is a one-row diff | `pull::http::a_missing_envelope_names_the_key_expected_and_the_keys_present` | ✓ |
| H-04 | Arrays under a declared envelope are found, and only there | `pull::http::arrays_under_the_declared_envelope_are_found` | ✓ |
| H-05 | A redirect is never followed, so the credential never reaches a host the descriptor did not name — proven over two real sockets, and confirmed to fail without the policy | `pull::http::a_redirect_is_refused_and_the_token_never_reaches_its_target` | ✓ |
| H-06 | A redirect is reported as `VendorRefused` carrying its status, the `Location`, and the reason — never silently, and never carrying the credential | `pull::http::a_redirect_is_refused_and_the_token_never_reaches_its_target` | ✓ |
| H-07 | An ordinary refusal still carries the vendor's own words and its own status, so a 429 stays distinguishable from a 500 | `pull::http::a_non_redirect_refusal_carries_the_body_and_its_status` | ✓ |
| H-08 | The credential never appears in a `Debug` rendering, and the blocking seam refuses by name rather than silently blocking | `pull::http::the_sync_seam_refuses_and_the_token_is_never_printed` | ✓ |
| H-09 | A refusal from a feed that declares a body-level error contract is classified where the **whole** body is in hand, and the verdict travels on `VendorRefused::named` rather than being re-read from the 500-character `detail` | `pull::refusal::only_the_feed_whose_error_page_was_read_declares_a_contract` | ✓ |

## The rung — which bar length, and where it lands

Added by D-0055. `Timeframe::DAY_1` existed in the store from D-0054 and nothing
could reach it. These rows are about the one field that decides three things at
once — the store directory, the fold bucket, and whether the session filter
applies — and about the two vendor facts nobody has written down.

| # | Must hold | Proven by | |
|---|---|---|---|
| RG-01 | `Granularity::Day1` carries `Timeframe::DAY_1`, a daily pull lands under `1day/`, and **nothing** is written under `1min/` for it | `pull::broker::a_daily_pull_lands_under_the_day_directory_and_folds_the_session_into_one_bar` — the whole broker path over a real socket, with the negative half asserted as well as the positive: the bar length is the PATH in this store, so a daily bar filed under `1min/` produces a well-formed file with a valid checksum and an accurate counter that no later reader can tell from real minute data. The same test asserts the FOLD, because the same field decides it — four one-minute bars in one session become one bar whose open is the first, high the maximum, low the minimum, close the last and volume the sum | ✓ |
| RG-02 | **Every rung `store::path::Timeframe::KNOWN` ships carries a store timeframe, and no rung it does not** — in both directions, and each agreeing rung agrees on its directory name **and** on its kind | `pull::vendor::every_rung_the_store_ships_carries_a_timeframe_and_the_rest_refuse` — plus a `const` assertion per shared rung beside `store_timeframe`, generated by one macro, and `Timeframe::KNOWN.len() == 7` pinned beside them. Was "exactly two rungs", which was the DEFECT stated as the rule: `store_timeframe` carried a `_ => None` catch-all and five rungs the store has shipped since D-0054 answered that they were unfileable. The two directions are different failures — a rung with no timeframe cannot be pulled, a timeframe no rung names is a directory nothing can write into — so both are asserted. The day rung's seconds cannot be tied the way the intraday ones are: `Grid::Daily` carries no interval, so what is pinned is the name, that the ladder calls the rung an aggregate, and that `DAY_1.secs()` is a whole number of `MINUTE_1` bars. D-0132 | ✓ |
| RG-03 | A rung the store cannot carry is **refused by name at the write boundary**, never substituted, and nothing reaches the disk | `pull::broker::a_rung_the_store_cannot_carry_is_refused_and_the_refusal_names_it` — `Granularity::Second1` is the reachable case: both archive feeds serve it and `crates/store` has no directory for it. The refusal names the rung it refused and the rungs this build does store, one failure for the run rather than one per member, and `bars/` is never created | ✓ |
| RG-04 | The window cap is per (feed, rung): **each rung is split by the cap its own vendor published, and the month bounds only a rung with no cap at all** | `api::server::each_rung_is_split_by_the_cap_its_own_vendor_published` — **this row asserted the OPPOSITE and every number in it was wrong.** It read *"a daily window is split by the month and never by the one-minute cap"*, which is what the code did until D-0320 removed the month clamp: `ingest::one` refused a batch spanning two months, so `split_window` cut at month ends to prevent it. That coupling cost forty requests out of every forty-one, and `ingest::months_in` now splits a landed batch downstream instead. Measured today, 2020-01-01..2026-08-07 per instrument: **Groww 81 chunks at its 30-day minute cap and 14 at its 180-day day cap; Dhan 27 at its 90-day minute cap and 80 at the day rung, where it published no cap and the month is the only bound left.** The `if daily_cap.is_some()` branch now asserts `daily.len() < WINDOW_MONTHS`, and *"the day rung is still being cut at month ends"* is its FAILURE message | ✓ |
| RG-05 | ~~No chunk ever spans two months, **at every cap including none**~~ | **RETIRED BY D-0320, AND THE TEST WAS DELETED WITH IT.** The month clamp cost *"forty requests out of every forty-one"*: a chunk was cut at the month end even where the vendor's published cap had room, so a 37-day daily window became 80 requests where 2 would do. The coupling it existed for is gone — `ingest::months_in` splits a landed batch by month downstream, so a chunk spanning two months is now filed correctly rather than refused. The successor, `pull::session::a_chunk_fills_the_cap_and_the_chunks_tile_the_window`, keeps the two clauses that survive — no chunk wider than the cap, and the chunks tile the window with no gap and no overlap — and asserts the month property **only** where the vendor published no cap, which is the honest remainder. **Cited by MR-38, which is the row D-0320 was written for; this one is not repointed at it, because a second row on one test that states a property the test deliberately stopped asserting is the drift this file exists to refuse.** Found by an audit that replicated gate 10 and measured five rows citing tests that exist in no file — this was one of them, and the only one that was a genuine retirement rather than a rename | ✗ |
| RG-06 | The rung goes on the wire in the feed's **own** word, and a rung with no recorded word refuses before the socket | `pull::http::the_rung_is_named_on_the_wire_and_an_unrecorded_one_refuses_before_the_socket` — `candle_interval=1minute` reaches the vendor, which is not `1min`, the store's spelling for the same rung. The daily word is refused by name with UNVERIFIED in the message and nothing is sent, asserted by the listener never receiving a second request | ✓ |
| RG-07 | No descriptor carries a day-level window cap or a daily interval word, and a rung field and a rung table exist together or not at all | `pull::vendor::a_rung_on_the_wire_needs_a_word_and_the_unrecorded_ones_stay_absent` — **the absence is the assertion.** `docs/00-charter.md` §4 records a one-minute cap for both brokers and a day-level one for neither, and records no daily interval spelling; the failure mode this row guards is somebody filling one in from memory | ✓ |
| RG-08 | The form offers exactly the rungs some feed declares **and** the store can file — no more, no fewer | `api::render::the_spot_form_offers_every_storable_rung_its_feeds_declare_and_no_other` — walked over the whole ladder rather than spot-checked, so `1s` (served by both archive feeds, unfilable by `crates/store`) is asserted absent. A control that can only ever refuse is what the descriptor table's own empty-set `const` assertion already forbids | ✓ |
| RG-09 | An unstated rung is the minute, and a rung this ladder does not have is refused naming what arrived | `api::ingest::a_spot_request_names_its_bar_length_defaults_to_the_minute_or_is_refused` — the default is the load-bearing half: `/pull/spot` is a replayable POST, and a body written before the field existed that silently changed rung would file one window into two directories the append-only store cannot then reconcile | ✓ |

**What these rows do NOT claim.** That a daily bar's timestamp is the session
open. `pull::fold` buckets on `(t + A).div_euclid(width) * width - A`, where `A`
is `IST_ANCHOR_MICROS` — the grid is anchored at **IST midnight**, not at the
UTC epoch. An NSE session therefore falls inside one IST day and yields one
bucket, and the resulting bar is stamped **00:00:00 IST**, not 09:15.

**This paragraph previously said the grid was UTC-epoch aligned and the stamp
was 05:30 IST.** Both were true before the anchor moved and neither is now. The
arithmetic note in `pull::fold` records why the two coincide for every width
that divides 19,800 and diverge for every width that does not — which is the
same fact `Timeframe::aligns_with_the_open` turns on, and the reason the 30- and
60-minute rungs are refused rather than filed.

Anchoring at IST midnight is what makes this correct for a venue whose session
does not cross an IST midnight, and this surface has no other. The Muhurat
sessions `docs/00-charter.md` §3 records are inside one IST day as well; what
this build cannot yet express is a venue with TWO sessions on one day, which
`docs/06-limits.md` records.
`pull::broker::a_daily_pull_lands_under_the_day_directory_and_folds_the_session_into_one_bar`
is what pins the one bucket; nothing here pins the stamp to a session boundary,
because it is not one.

## The pull order — cheap pass first, and it is enforced

The rule is the operator's, 15 Aug 2026, and `crates/api/src/ladder.rs` carries
it verbatim: day before minute, spot before derivatives, and the two archive
feeds exempt. D-0054 wrote the sequencing down and nothing enforced it until
D-0153 wired it into `broker_run` — **before a socket, after the target list**.

| # | Invariant | Proof | State |
|---|---|---|---|
| PO-01 | The minute rung is refused until every instrument-month in the window is held at the day rung, and the refusal carries `missing of total` rather than a sentence | `api::ladder::the_minute_rung_waits_for_the_day_pass_and_names_what_is_missing` | ✓ |
| PO-02 | The day rung has nothing in front of it and is never gated, so an empty store cannot block the cheap pass | `api::ladder::the_day_rung_is_never_gated`, `api::ladder::an_ungated_rung_runs_against_an_empty_store` | ✓ |
| PO-03 | A derivative with no spot behind it is refused for the SEGMENT, and that refusal is reported ahead of the rung one — a window failing both would otherwise send the operator to pull the day rung of a segment they should not be on | `api::ladder::derivatives_wait_for_spot_and_that_refusal_is_reported_first`, `api::ladder::with_spot_held_the_derivative_still_owes_its_own_day_pass` | ✓ |
| PO-04 | Each instrument is probed under its **own** segment and its **own** venue, never one taken for the batch. `SpotTarget::Fno` and `Everything` span `Index` and `Cash`, and `catalog::tracked` filters on universe flags and never on exchange, so neither is a batch property | `api::ladder::a_mixed_target_is_probed_under_each_instruments_own_segment`, `api::ladder::a_month_held_at_one_venue_is_not_held_at_another` | ✓ |
| PO-05 | The gate reads the **store**, not the audit journal. A day pass that reported `Stored` and wrote nothing — `audit::Outcome::Empty`, which balances trivially at `0 = 0 + 0 + 0` — passes a journal check and fails this one. `Manifest::record_held` refuses a zero-row entry as `EmptyEntry`, so an `Empty` run leaves no entry and absence is the whole test | `api::ladder::a_month_with_no_bars_cannot_even_be_recorded` | ✓ |
| PO-06 | An archive feed is exempt in the rule and in the wiring: a folder feed issues no request, so there is no expensive call for a cheap one to protect | `api::ladder::a_folder_feed_is_never_gated`, `api::server::a_folder_feed_reaches_the_loop_with_an_empty_store` | ✓ |
| PO-07 | `broker_run` consults the order before `note_run_started` and before any socket, and a refused run reports `attempted: 0` with `blocked` set — distinct from an empty `reached`, about which nothing may be concluded regarding the vendor | `api::server::a_minute_run_is_refused_until_the_day_pass_has_landed` | ✓ |
| PO-08 | A census that will not load **refuses**, quoting its own words. Reading it as "nothing held" would refuse a day pull the operator could have run; reading it as "everything held" would open the gate on a file this build just refused — `CLAUDE.md` §4's fallback that hides a failure | `api::server::an_unreadable_census_refuses_and_names_what_would_not_load` | ✓ |
| PO-09 | A window naming no instrument-month is not refused by the order — it has no prerequisite that could be missing, and the form already refuses an empty request for its own reason | `api::ladder::a_request_naming_nothing_is_not_refused_by_the_ladder` | ✓ |
| PO-11 | A blocked run's reason and its HTTP status are the RUN's own, never restated by the receipt. An out-of-sequence request answers `409` and names the pass to run; an unreachable broker answers `503`. The receipt carried both as literals, which was true while `Broker::Refused` was the only producer and became false the moment the pull order was the second | `api::server::a_run_the_order_refuses_says_so_on_its_receipt_and_answers_409` — which asserts the broker literal is ABSENT, not merely that the order's words are present | ✓ |
| PO-12 | A refusal's **headline** is the reason the request was refused, not a description of the transport that would have served it. `accepted_html` fills that line from `halt_for`, which on a serving process is a paragraph about Parameter Store and socket plumbing — true, and not an answer to *why did my pull not run* | `refused_html` is the renderer; `api::server::a_run_the_order_refuses_says_so_on_its_receipt_and_answers_409` reads the headline back off the page | ✓ |
| PO-13 | **No refusal sentence this build writes is read by `autopilot::classify` as a credential or disk fault.** `classify` matches by SUBSTRING and `observe` turns a feed-wide `Credential` into a permanent halt — so refusal text is classifier input, not prose. A draft of `unreachable_broker` said "no credential was read", and that one word turned a transport-shaped refusal that should back off into a halt telling the operator their token was dead | `api::server::no_refusal_this_module_writes_is_read_as_a_credential_or_disk_fault`, and `api::autopilot::a_whole_round_runs_and_a_refused_broker_is_named_on_the_page` is what caught it | ✓ |
| PO-10 | A window is exactly the set of month files it touches — one inside a month, two across a boundary, and the year boundary where a naive `+1` breaks. This is the multiplier in the gate's cost, and it is the measured half | `api::ladder::a_window_names_every_month_file_it_touches` | ✓ |

**What these rows do NOT claim.** That futures precede options. The operator's
order is spot → expired futures → expired options; `Segment` has three variants
and **both** derivative legs are `Fno`. The store keys a month on that enum, so
it cannot tell a futures month from an options month, and `segment_precedes`
does not pretend otherwise — inventing a distinction the census cannot answer
would be a gate reporting on a fact nobody records.

Nor do they claim the per-probe `O(1)`. `Manifest::entry` is one `HashMap::get`
and no test here isolates it; `docs/04-invariants.md` C-03 says outright that
the nearest measurement does not isolate a probe's own cost either. What IS
proven is the multiplier — P-10 — and that the census is never walked, which is
structural and visible in the loop. Recorded in `docs/06-limits.md`.

## Cross-cutting

| # | Must hold | Proven by | |
|---|---|---|---|
| X-01 | Run identity changes if any loaded bar differs by one field | **NOT PROVEN, AND THERE IS NOTHING TO PROVE IT AGAINST.** `core::proptest::identity_sensitivity` exists in no file; no `proptest` dependency exists. There is no run-identity function either: `blake3` sits in the workspace dependency table and **no member takes it**, and no crate holds a function that hashes `CLAUDE.md` §3 rule 3's nine inputs. So this row has neither the test nor the code. **The subject is further away than the hash:** five of those nine inputs have no source at all, because `crates/vocab`, `crates/indicators` and `crates/engine` are not workspace members — there is no mask, no `vocab_version` and no sweep to identify. The name stays in the backticks, and D-0062 put `X-01` in gate 10's `allow_pending` with that reason, so the row reads PENDING rather than red. What closes it: those crates, then a `data_digest` over the loaded bars, then the identity function, then the test — and **not** by adding `proptest`, which this workspace has twice declined in favour of exhaustive ordinary `#[test]`s (S-02 walks every index; P-01 uses eight real threads) | ✗ |
| X-02 | Prices never touch a float on any path from wire to store to result | **THE ROW WAS RIGHT THAT THE LINT WAS A `warn`, AND THAT IS NOW FIXED RATHER THAN DESCRIBED.** D-0061 tightened `float_arithmetic` from `"warn"` to `"deny"` in `Cargo.toml`; nothing broke, because CI already passed `-D warnings` and every float in this workspace was already either the one boundary conversion or a statistical value. What the `warn` cost was honesty: six comments across `crates/api`, `crates/costs`, `crates/pull` and `crates/store` told a reader the lint was denied, and one flag stood between that and false. `core::lint::no_float_in_price` now exists and reads three facts off the source — that the workspace lint table **denies** the lint, that exactly one *item-level* `#[allow]` overrides it, and that it sits on `Paisa::from_rupees_half_up` — then walks every other module of `crates/core` and requires each float it finds to be handed to that conversion within two lines. `crates/core/src/vendor.rs`'s `parse_strike` is the one that is, and it does no arithmetic. `core::lint::the_module_list_is_the_whole_crate` checks the scanned list against `lib.rs`'s own `pub mod` lines, so a new module is a failing test rather than a silently unscanned file. **It is a source check and says so in its own header: it covers `crates/core` and no other crate**, because a Rust test cannot walk a tree without depending on the directory it ran from. `crates/greeks`'s four module-wide allows are not a second price path — a delta of `0.00017142680429549402` is a statistical value and `CLAUDE.md` §7 keeps those at full precision, so the line is between a price and a statistic rather than between an integer and a float. The type half is still X-11's | ◐ |
| X-03 | Every tracked file has an allowed extension | CI gate 1 | ✓ |
| X-04 | No build script invokes an external process | CI gate 2 | ✓ |
| X-05 | ~~`web` depends on `core` alone~~ — **THE GATE SKIPS PERMANENTLY AND THIS TICK WAS FALSE.** There is no `crates/web`: D-0052 and D-0053 moved the front end to `web/` as an unrestricted directory, and `CLAUDE.md` §5 now states the consequence outright. Gate 7's body opens `[ -f "$f" ] \|\| { echo "skip — crates/web does not exist"; exit 0; }`, and a second instance at `ci.yml` does the same on `[ -d crates/web ]`. A skip that exits 0 is a pass to every reader of the check, which is the shape `docs/06-limits.md` §7b names as the cause of ten separate findings. The rule has nothing to bind; §2's toolchain boundary governs the front end now | CI gate 7 — **skips, proves nothing** | ✗ |
| X-06 | **Line and region** coverage is 100% on every crate, with no omit list | **THE GATE EXITS 1 ON THIS TREE.** Measured by running the CI command itself — `cargo llvm-cov --workspace --locked --fail-under-lines 100 --fail-under-regions 100 --summary-only`, cargo-llvm-cov 0.8.4, 2026-08-07 at commit `79c5e80`: **exit 1**, TOTAL **96.75% regions** (851 of 26,169 missed), **96.24% lines** (592 of 15,734), 93.88% functions (94 of 1,537). Eight files are short, and the table below names every one | ✗ |
| X-06b | ~~Branch coverage is 100% on every crate~~ | **NOT MEASURED.** `llvm-cov` instruments zero branches on the pinned stable toolchain and `--branch` cannot run there at all. Narrowed by D-0030; recorded in `docs/06-limits.md` §7. | — |
| X-07 | No mutant survives on a touched module | `cargo-mutants`, run per change. `crates/pull`, D-0036: **263 mutants, 227 caught, 36 unviable, 0 survivors**. `crates/costs`, D-0044: **163 mutants over `trip.rs`, `money.rs`, `fill.rs`, `scope.rs`, `error.rs` — 106 caught, 57 unviable, 0 survivors**, and one mutant that *did* survive a first run is written up in `docs/06-limits.md` §27. **Never measured at all: `crates/core`, `crates/store`, `crates/api`**, and the nine `crates/costs` files outside that list. `crates/store` planted five mutants by hand, which §22 records is not a survey. There is no `cargo-mutants` step in CI, so nothing enforces this row | ◐ |
| X-08 | No tracked file contains a **slash-joined** credential path whose environment segment is a well-known one | CI gate 1c | ✓ |
| X-08b | No literal under `crates/pull` that could be a path segment is undeclared | CI gate 1d | ✓ |
| X-09 | `core` declares no dependency at all | CI gate 9 | ✓ |
| X-10 | Every reachable row in this file names a test that exists | **THE GATE EXITS 0 ON THIS TREE, AND THIS ROW IS `◐` RATHER THAN `✓` BECAUSE THREE ROWS ARE EXEMPTED RATHER THAN PROVEN.** Measured by running gate 10's own script at this commit: **611 rows read, 568 named tests checked against a tracked crate, 1 skipped for a crate that is not a workspace member, 7 exempted by the allowlist, 0 missing.** The first two figures move whenever any row is added and are a reading, not a bound; the last three are the ones this row is about — and one of *those* had gone stale, which is worth recording. It read **17 skipped** and now reads **1**, because that 17 was `crates/vocab`, `crates/indicators` and `crates/engine` being treated as non-members. All three are listed in the root `Cargo.toml`, so their rows are now CHECKED rather than waved past. The remaining 1 was V-01's literal placeholder, which the gate read as a crate called `crate` — and quoting it HERE made a second one, so this row named a nonexistent test by describing one. Both are now written `<crate>::<module>::<name>`, which the gate's pattern cannot match. The same false premise was written into gate 10's own X-01 exemption and is corrected there. It read **10 missing**, then **4**, then **5** when A-20 joined them, across three earlier passes. Two of the five were written — A-20 by D-0060 and X-02 by D-0061 — and three were allowlisted by D-0062: P-03, X-01, X-13, each because the code the row describes does not exist and a test would therefore assert an absence. The 7 exempted tokens come from those 3 rows; a row naming a test twice is counted twice, and P-03's second name is a test that exists and passes. **A tick bought by an allowlist is not a tick earned**, which is the same argument X-06 makes in the other direction, so the glyph names where this has been proven — every row but three — rather than claiming all of them | ◐ |
| X-12 | Each vendor writes only under its own path prefix; no vendor can overwrite another | `store::unit::vendor_prefix_isolated` — the test **exists and passes**, and this row wore `—` anyway. It proves the claim *lexically*: the first segment is a `Vendor` rather than a string, and no segment can hold a separator. It does not touch a filesystem | ✓ |
| X-13 | A bar-for-bar mismatch between two vendors refuses the window and names the timestamp | **NOT PROVEN, AND THE REASON HAS CHANGED.** `store::unit::vendor_disagreement_refuses` exists in no file. `crates/store` **does** have a bar reader now — `BarFile::read_record`, walked at every index by S-02 — so the missing piece is no longer the reader. What is missing is the comparison: nothing in this repository opens two vendors' months and matches them bar for bar, so there is no code for a test to drive. **This row was checked against the product direction before it was left standing, and it survives that check.** `docs/07-plan.md` R-6 — "no vendor comparison anywhere" — is about DISPLAY, and says so in its own enforcement column: a feed picker on `/store` and one row column. X-13 is an ingest-time refusal, and the same repository already ships a cross-vendor validation that refuses one level up — `api::merge::a_cross_vendor_isin_conflict_is_reported_and_neither_side_is_dropped`, whose own assertion reads "a named disagreement REFUSES the universe". Comparing two vendors to validate is established practice here; showing two vendors to an operator is what R-6 forbids, and abandoning this row on R-6 would conflate them and discard a real safeguard. The name stays in the backticks, and D-0062 put `X-13` in gate 10's `allow_pending` so it reads PENDING rather than red. What closes it: a two-`BarFile` comparison in `crates/store` for one (exchange, segment, symbol, timeframe, month) that refuses and names the first divergent timestamp, then `store::unit::vendor_disagreement_refuses` | ✗ |
| X-11 | A price is constructible from a float only through the one checked conversion | `core::price::refuses_an_out_of_range_price_instead_of_saturating` (private field; no other path exists) | ✓ |
| X-14 | **Two feeds delivering byte-identical bars for one instrument-month do not share a run identity.** The store is keyed by vendor, so one instrument-month exists once per feed and sweeping two of them is two runs — but until D-0225 the identity's eight terms named *what* was computed and none named *whose data*. Two vendors redistributing one NSE feed publish the same OHLCV at the same timestamps for a clean month, at which point `data_digest` is equal and so is every other term. Separation was incidental, not guaranteed, while `cli::STORED_PROVENANCE` told the reader the identity "names the exact column they came from" | `runner::identity::two_feeds_with_byte_identical_bars_do_not_collide` — **holds `data_digest` EQUAL on purpose**, so it fails on any implementation that leans on the bytes differing, and asserts one feed still reproduces its own identity so the term discriminates rather than adding noise · `runner::identity::every_term_changes_the_identity` gains a ninth arm, for the reason `pair_budget` needed one: a term written and never read is a term two runs can share | ✓ |
| X-15 | **A ranked run agrees with a plain one, and no ranked row is mispaired.** `Sweeper::run_ranked` builds the bar-bit column and the `Forward` from ONE slice, which is what makes `outcome::Edge::mismatched` unreachable by construction rather than merely detected afterwards — a non-zero count means that row's mean and `t` were computed from returns belonging to other bars. **Corrected for D-0394:** the two runner methods do NOT share `fold_and_walk`; the plain path retains a `Sweep` and the ranked path streams retired frontiers. What they share is the engine's single `Ladder::walk_into`, reached through different sinks, so retention cannot fork the Apriori walk | `runner::tests::a_ranked_run_agrees_with_a_plain_one_and_never_mispairs` — asserts census, warm-up boundary, every level tally, exclusions, halt, threshold and ladder depth match `run`; `considered == streamed`; raw and effective trials equal the retained formulas; the global heap respects `keep`; retained and streamed detectability ranks agree; the fixture is non-empty; and `mismatched == 0` on every row. Engine-level order and five-exit equivalence are pinned separately by `engine::tests::a_streamed_walk_and_a_retaining_walk_are_the_same_walk` | ✓ |
| X-16 | **Every set bit is rendered as a name, including one the table cannot name.** `ConditionMask` is 384 wide against a 280-row table, so a mask can carry an unnameable bit; rendering it as `?N` rather than dropping it keeps the printed arity equal to `popcount`, so a reader counting names against the `k` the ladder reports cannot be misled by a silent omission. The empty mask renders as a named empty set, because a blank where a combination should be is indistinguishable from a rendering bug | `runner::every_set_bit_is_named_and_an_unnameable_one_is_shown_rather_than_dropped` — asserts the COUNT first (the property) and the strings second. The `?N` arm is unreachable from production, so without this test it is an uncovered region under §9's 100% floor | ✓ |
| X-17 | **A sample too thin for the statistics it is running says so, and a sufficient one is silent.** One stored instrument-month is ~20 trading days, on which a five-fold walk-forward tests each fold on a handful of days and a block-10 bootstrap draw is one or two blocks. The figures were never wrong; they rendered in exactly the same format as figures over 3,650 sessions with nothing to tell a reader which they held (§3 rule 6). The warning names which figures are affected and which are not — the trades, the grid and the excursions measure what happened | `cli::a_thin_sample_is_named_and_a_sufficient_one_says_nothing` — asserts silence AT the boundary and above it, because a warning that always fires is noise a reader learns to skip; asserts the thin case names both `p-values` and `PBO` and also says `unaffected`, so the honest half is not discarded with the dishonest half; and asserts zero sessions does not divide by zero | ✓ |
| X-18 | **A stored run's data term binds every bar series that can change its answer.** A native one-series run retains the historical `data_digest` byte for byte, so an exact rerun does not append a duplicate identity. A coarse signal run whose entries, exits, stops, targets and trails execute on a separately loaded one-minute slice domain-separates and hashes both complete ordered slices; changing one hidden interior minute re-keys the run even when every coarse signal bar remains byte-identical. Every stored operator path that loads a separate execution series calls that composition before trading | `runner::identity::a_coarse_run_identity_binds_the_interior_execution_path`; `runner::identity::execution_data_binding_is_deterministic_and_presence_is_explicit`; `cli::derived_floor_tests::every_stored_two_series_operator_path_binds_execution_into_identity` | ✓ |

### X-06, measured rather than asserted — D-0045

The row above carried a tick. The gate it names exits 1, and had been exiting 1
for some time. These are the eight files short of 100%, from the run described
in the row:

| File | Regions | Missed | | Lines | Missed | |
|---|---|---|---|---|---|---|
| `pull/src/vendor.rs` | 445 | 445 | **0.00%** | 366 | 366 | **0.00%** |
| `pull/src/ingest.rs` | 168 | 168 | **0.00%** | 100 | 100 | **0.00%** |
| `pull/src/archive.rs` | 142 | 50 | 64.79% | 93 | 34 | 63.44% |
| `pull/src/fetch.rs` | 210 | 60 | 71.43% | 148 | 47 | 68.24% |
| `pull/src/csv.rs` | 373 | 99 | 73.46% | 160 | 30 | 81.25% |
| `pull/src/fold.rs` | 94 | 11 | 88.30% | 77 | 14 | 81.82% |
| `store/src/file.rs` | 813 | 17 | 97.91% | 510 | 0 | 100.00% |
| `api/src/ingest.rs` | 730 | 1 | 99.86% | 546 | 1 | 99.82% |

Six of the eight are in `crates/pull`, and **two of them are at 0.00%** — 613
regions and 466 lines between them, in tracked files with no `#[test]` in them
at all. `archive.rs`, `fetch.rs`, `csv.rs` and `fold.rs` are the same pattern
less severely. That is modules committed ahead of their tests, and it is the
same shape as the defect D-0029 recorded for `core/src/universe.rs`: CI gate 10
walks rows→tests and never tests→rows, so a module that claims nothing is
invisible to it.

**This figure is a snapshot and it moved twice while D-0045 was being written.**
The first run of the session measured 97.41% regions / 96.93% lines over six
short files; commits `1f98bb5`, `eb95996` and `79c5e80` then landed and it fell
to the figure above over eight. An audit before that measured 95.31% lines /
95.91% regions. **The gate has been red across all three**, and the direction is
not monotone. That volatility is the argument for recording a command, a commit
and a date here rather than a tick: the tick was wrong at every one of those
points and looked equally right at each.

### S-20 names three assertions that cannot fail — D-0045

`crates/store/src/format.rs` defines `BLOCK_LEN` as
`RECORD_STRIDE * RECORDS_PER_BLOCK`. Three of the `const` assertions beside it
are therefore **tautologies** — they restate that definition and hold for every
possible value of the two factors:

- `assert!(BLOCK_LEN.is_multiple_of(RECORD_STRIDE))`
- `assert!(BLOCK_LEN / RECORD_STRIDE == RECORDS_PER_BLOCK)`
- `assert!((RECORDS_PER_BLOCK - 1) * RECORD_STRIDE + RECORD_STRIDE == BLOCK_LEN)`
  — which reduces to `n·s == n·s`

**Measured, not reasoned:** a scratch crate carrying the identical definitions
and exactly these three assertions compiles at **exit 0** with
`RECORDS_PER_BLOCK = 72`, and again at **exit 0** with `RECORDS_PER_BLOCK = 1`.
The mutant D-0039 reports killing — `BLOCK_LEN` 4088 → 4096 — is also
unwritable, because `BLOCK_LEN` is not a literal to mutate.

The third of these is the one the source comment introduces with *"a compile
error rather than a walk"*, and `CLAUDE.md` §4 bans a test that asserts nothing.
**The no-straddle property is still genuinely secured** — by `BLOCK_LEN == 4088`,
`RECORDS_PER_BLOCK == 73` and `BLOCK_LEN == 56 * 73`, which are falsifiable and
which D-0039 also added, plus the runtime walk. So this is a wrong *citation*,
not a missing guarantee, and S-20 now names the assertions that carry it. The
source comment is in `crates/store/src/format.rs` and is reported rather than
edited — that file is not this change's to touch.

### The nine rows that name a tool this repository does not have — D-0045

`loom` and `proptest` appear in **no `Cargo.toml` in this workspace**. Verified
by `grep -rn 'loom\|proptest' --include=Cargo.toml .`, which returns nothing.
Nine rows name a module of one of those two names:

*Written as a list, not a table: a `| id |` row here would be parsed by CI gate
10 as a real invariant row and counted twice.*

- **S-02** named `proptest::roundtrip` — ✗ no such function anywhere.
- **S-03** named `loom::commit_counter_publishes_last` — ✓ it **exists**, as
  `store::fault::…`, an ordinary `#[test]`. Path corrected above.
- **S-08** named `proptest::oi_sentinel_distinct` — ✓ it **exists**, as
  `store::unit::…`, an ordinary `#[test]`. Path corrected above.
- **V-03** named `indicators::proptest::suffix_independence` — `—`,
  `crates/indicators` does not exist.
- **V-05** named `indicators::proptest::differential_vs_naive` — `—`, same.
- **E-01** named `engine::proptest::antimonotone` — `—`, `crates/engine` does
  not exist.
- **E-02** named `engine::proptest::apriori_equals_bruteforce` — `—`, same.
- **P-01** named `pull::loom::governor_ceiling` — ✗ no such function, and
  `crates/pull` is tracked and compiled today.
- **X-01** named `core::proptest::identity_sensitivity` — ✗ no such function,
  and `crates/core` is tracked and compiled today.

The four `—` entries are legitimately unreachable: their crates do not exist, which
is what `—` means. But **the module name is a promise about a dependency**, and
adding either tool is a workspace-manifest change nobody has made or decided on.
Those four rows are naming an implementation that would have to be chosen first.
No row here now claims a property-based or a concurrency proof that has ever run.

**Two of those nine have since moved, and the tool count did not change.**
`loom` and `proptest` are still in **no** `Cargo.toml` in this workspace, and
neither was added to satisfy a row — re-checked with the same command. S-02 is
now proven by `store::roundtrip::…`, ordinary `#[test]`s that walk *every* index
rather than sampling random ones, and P-01 by `pull::concurrency::…`, ordinary
`#[test]`s with real threads. X-01 is unchanged and still `✗`. A property-based
proof and an interleaving proof remain things this repository has never run, and
no row claims either.

### Why gate 10 could not see any of this — D-0045

Gate 10's own comment says it: *"The module segment. `store::unit::x` and
`store::fault::x` are the same question to this gate."* It matches on
`(crate, fn)`. So S-03 and S-08 passed it for their whole lives while pointing
at modules that do not exist, and the ten genuinely-absent tests were caught
only when the gate stopped honouring the `—` glyph as a skip. A row can still
name the wrong module and stay green. That gap is now recorded rather than
rediscovered.

### The ten phantom rows, one at a time

*Written as bullets, not a table, for the reason the section above gives: a
`| id |` line here would be read by CI gate 10 as a real invariant row.*

**No decision id is claimed for this pass.** `docs/05-decisions.md` ends at
D-0045 and is not this change's file; the ledger entry it owes is named at the
foot of this section.

Two of the ten were **already proven and pointing at the wrong name** — the
worst of the three cases, because such a row reads as a gap when the property
holds, and the next person writes the test twice:

- **S-10** named `store::integration::second_writer_refused` and said "no
  advisory lock is exercised anywhere in this repository".
  `store::write::a_second_writer_is_refused_while_the_month_is_held` had been
  exercising one, and passing. Row corrected; nothing was written.
- **S-05** named `store::fault::enospc_returns_error`.
  `store::file::a_full_disk_mid_write_is_returned_and_never_signalled` proves
  the classifier and the write loop against a scripted host. Row corrected to
  `◐`, because the kernel half is not proven and cannot be here.

Four had **no test and all four turned out to be writable** — three test files,
eight tests, and not one new dependency. Two of them were writable only because
code landed after the row was last read: `crates/store/src/file.rs` gave the
crate a bar file at all, and `crates/pull/src/ingest.rs` gave this repository
its first path from a vendor's folder to a stored bar:

- **S-02** — `crates/store/tests/roundtrip.rs`. Every index of a 160-record
  file, the decoded record and the raw bytes at its computed offset, across
  three appends and a reopen.
- **P-01** — `crates/pull/tests/concurrency.rs`. Eight threads, one fixed
  instant, exactly the allowance issued between them. `◐`: two separate
  governors are two budgets and nothing bounds their sum.
- **P-02**, **P-04** — `crates/pull/tests/integration.rs`. A vendor's folder
  through `pull::ingest::from_dir` to a file, then the file reopened and every
  record read back. P-04 is `◐` and the reason is a finding rather than a
  caveat: the re-run **writes** nothing and **reports** three.

Four could not be proven and were **not** faked. Each keeps its name, its `✗`
and its own account of what is missing:

- **P-03** — there is no trading calendar and `docs/00-charter.md` records no
  holiday list. A weekend rule would be wrong, which is a stronger reason than
  "not yet".
- **X-01** — no run-identity function exists in any crate; `blake3` is in the
  workspace table and no member takes it.
- **X-02** — the source check named is CI gate 11's job, not a test's. Writing
  a Rust test that greps the tree would assert a spelling.
- **X-13** — the bar reader arrived; the two-vendor comparison did not.

**CI gate 10 therefore still exits 1, on four lines, by choice.** The one lever
that would silence them is the `allow_pending` allowlist in
`.github/workflows/ci.yml`, which is deliberately empty and is not this change's
file. Using it would be a decision to stop reporting four known gaps, and that
is a `docs/05-decisions.md` entry somebody has to sign — which is the ledger
entry this pass owes, alongside the three new test files.

### The last five, and the gate that stopped being red

*The paragraph above is kept as written. This section is what happened next.*

**The four became five**, and the fifth was not a new gap so much as an old
claim that had gone stale without anyone touching it. A-20 read "no page this
server emits contains a script" and wore `✓`. It was true on 2026-08-07, the day
its row landed in `08a4258`. D-0052 and D-0053 opened `web/` on 2026-08-08, and
`f046b36` put a deferred `<script src="/typeahead.js">` on `/instruments` the
same day. **A row can go false while nobody edits it**, which is the failure
mode this whole file is built against, and no gate can see it: gate 10 checks
that a name exists, never that a sentence is still true.

The five were then sorted by ONE question — *does the thing this row describes
exist today?* — and the answer decided the mechanism:

- **A-20 — it exists, and more of it than the row covered.** Written: D-0060.
  Seven page functions, each rendered twice, script counted as a budget rather
  than banned as an absence. The absolute was abandoned because it was false;
  what replaced it is checked over three more pages than the four that had
  piecewise assertions.
- **X-02 — the lint existed and was weaker than the row claimed.** Written:
  D-0061. `float_arithmetic` went from `"warn"` to `"deny"`, which broke
  nothing, and `core::lint::no_float_in_price` now reads that off the table so
  it cannot drift back. Scoped to `crates/core` and says so.
- **P-03, X-01, X-13 — the subject does not exist.** Allowlisted: D-0062. No
  trading calendar, no run-identity function, no two-vendor bar comparison. A
  test written today would assert an absence, pass, and have to be deleted the
  day the thing arrives — which is a test that asserts nothing, and `CLAUDE.md`
  §4 bans those outright.

**X-13 was checked against the product before it was left standing.**
`docs/07-plan.md` R-6 forbids vendor comparison, and on the face of it that
reads like an argument to abandon this row. It is not: R-6's own enforcement
column is "feed picker on `/store`; the counter cards and the row column both
follow it", which is DISPLAY. X-13 is an ingest-time refusal, and `crates/api`
already ships a cross-vendor validation that refuses — `merge`'s ISIN conflict,
whose assertion reads "a named disagreement REFUSES the universe". Comparing two
vendors to validate is established here; showing two feeds to an operator is
what R-6 forbids. Abandoning X-13 on R-6 would have conflated the two and thrown
away a safeguard on a misreading.

**What the allowlist costs, so it is not free.** It keys on the row, not on the
token, so P-03's second name — a test that exists and passes — stops being
checked too. Gate 10 prints every silenced token by name each run, so the cost
is in the log rather than hidden, and the entry in `ci.yml` says it out loud.

---

## Greeks — closed form, and the one solve that is not

`crates/greeks` is `f64` throughout and never sees a paisa. `CLAUDE.md` §7
reserves `i64` for prices and keeps statistical values at full precision; a
delta is the second kind. D-0046.

**`G-01` … `G-09` below are the SECOND family with those ids.** The rung
section above carries a `G-01` … `G-09` of its own, and the two are unrelated.
Nothing outside this file cites either block by id today, and nothing should
start: CI gate 12 resolves a row id with the **first** match in this file, so a
doc comment citing `G-09` from `crates/greeks` would be validated against the
rung row instead — which is the hazard D-0045 renumbered `K-*` to `C-K-*` to
remove, still open here. Cite these rows by their test name until one family is
renumbered under its own decision entry. Found by the gate 12 sweep; not fixed
by it, because renumbering a row is a change to this file's contract and belongs
with the decision that signs it.

| # | Must hold | Proven by | |
|---|---|---|---|
| G-01 | Gamma and vega are **bit-identical** between a call and a put at the same strike — not close, identical, because both are computed above the branch | `greeks::bsm::gamma_and_vega_are_bit_identical_between_the_call_and_the_put` | ✓ |
| G-02 | `delta_call − delta_put == e^-qT` to a measured bound, and the bound is 2 ulps rather than a hoped-for zero | `greeks::bsm::the_delta_difference_is_the_carry_discount_to_a_measured_bound` | ✓ |
| G-03 | Put-call parity `C − P == S·e^-qT − K·e^-rT` holds across the grid to 2.1e-16 of spot | `greeks::bsm::put_call_parity_holds_across_the_grid` | ✓ |
| G-04 | Every one of the five greeks reproduces a central difference **wherever the stencil can resolve it**, and the number of comparisons actually made is asserted | `greeks::bsm::every_greek_reproduces_a_central_difference_wherever_one_can_be_taken` | ✓ |
| G-05 | The shipped normal CDF agrees with an independently implemented reference to 2.22e-16 absolute and 8.9e-9 relative — the check that caught a transposed digit in a Hart coefficient | `greeks::normal::hart_agrees_with_an_independent_reference` | ✓ |
| G-06 | The normal CDF matches published values at twelve points, on a **relative** tolerance so the tail is actually checked | `greeks::normal::the_cdf_matches_known_values` | ✓ |
| G-07 | Both Hart branches and both saturating tails are exercised, and the two branches meet at the split inside Hart's own tail accuracy | `greeks::normal::both_hart_branches_and_both_saturating_tails_are_exercised` | ✓ |
| G-08 | `volatility -> price -> IV` recovers the **volatility** to 1e-4 relative, and every point that does not is refused as one of exactly two named kinds whose counts account for the whole refusal count. Measured worst 2.33e-6; the superseded `price -> IV -> price` form is asserted too and is the weaker of the two — it measures `0e0` at the point where the volatility is 5.14% wrong. D-0046 | `greeks::solver::a_volatility_round_trips_through_the_solver_and_back` | ✓ |
| G-09 | One solve never costs more than `BRACKET_EVALUATIONS + NEWTON_STEPS + BISECTION_STEPS + FINAL_EVALUATION = 2 + 8 + 64 + 1 = 75` model evaluations — **every** evaluation, not only the ones inside the search — and both methods are actually exercised | `greeks::solver::the_iteration_count_never_exceeds_the_arithmetic_bound` | ✓ |
| G-10 | The same inputs give the same volatility **bit for bit**, and the same iteration count and method, **within one process and one target**. Bit-for-bit reproducibility does NOT hold across targets — a different libm moves 140 of 1,344 solved volatilities by up to 4.22e-14 relative — and `docs/06-limits.md` §18 carries the measurement (`CLAUDE.md` §3 rules 5 and 6) | `greeks::solver::the_solver_is_idempotent_to_the_bit` | ✓ |
| G-11 | A price does not determine a volatility when one unit in the last place *the price actually has* — set by the two legs it is a difference of, not by its own magnitude — moves the answer by more than `1e-3` of itself; that is **refused**, never returned | `greeks::solver::a_price_that_does_not_determine_a_volatility_is_refused_not_returned` | ✓ |
| G-12 | A quote at or below the discounted intrinsic value is refused on both sides, and a negative quote lands in the same refusal with `intrinsic` printed as zero | `greeks::solver::a_price_at_or_below_the_discounted_intrinsic_is_refused_on_both_sides` | ✓ |
| G-13 | A quote at or above the model's supremum is refused on both sides | `greeks::solver::a_price_at_or_above_the_model_supremum_is_refused_on_both_sides` | ✓ |
| G-14 | A quote inside the arbitrage bounds but outside the searched volatility band is refused, never clamped into it | `greeks::solver::a_price_outside_the_searched_band_is_refused_rather_than_clamped_into_it` | ✓ |
| G-15 | A NaN or an infinity in any input is refused **by the name of the field it arrived in** | `greeks::bsm::a_non_finite_input_is_refused_field_by_field` · `greeks::solver::a_non_finite_market_price_is_refused_before_anything_is_computed` | ✓ |
| G-16 | A non-positive spot, strike or volatility is refused by name, and `T <= 0` gets its own refusal rather than being called malformed | `greeks::bsm::a_non_positive_spot_strike_or_volatility_is_refused_by_name` · `greeks::bsm::an_expired_contract_is_its_own_refusal_and_not_a_malformed_input` | ✓ |
| G-17 | Every accepted range refuses the value just past it and names the field and the bound | `greeks::bsm::every_bound_refuses_the_value_just_past_it_and_names_it` | ✓ |
| G-18 | Inputs inside every bound that the model still cannot represent are refused, never returned as a `NaN` greek | `greeks::bsm::an_in_range_input_that_the_model_cannot_represent_is_refused_not_returned` | ✓ |
| G-19 | A strike between two rungs is refused, never rounded onto one; and a call and a put read the same rung from opposite sides | `greeks::moneyness::a_strike_between_two_rungs_is_refused_and_never_rounded_onto_one` · `greeks::moneyness::a_call_and_a_put_read_the_same_rung_from_opposite_sides` | ✓ |
| G-20 | A rung past `MAX_STEPS` is refused rather than truncated into range by the one `f64 -> i32` cast in the crate | `greeks::moneyness::a_rung_beyond_the_bound_is_refused_rather_than_truncated_into_range` | ✓ |
| G-29 | Every bound is INCLUSIVE: the value exactly on it is accepted, which is what stops the refusal above from being satisfied by refusing everything | `greeks::bsm::every_bound_accepts_the_value_exactly_on_it` | ✓ |
| G-30 | The Brenner-Subrahmanyam seed lands within 1% of the answer at the money, and the search then finishes in Newton in at most four steps | `greeks::solver::the_seed_lands_next_to_the_answer_at_the_money` | ✓ |
| G-31 | The reported iteration count is the work actually done, floor as well as ceiling: the three fixed evaluations plus at least one Newton step, and the bisection's 64 halvings on top of the Newton steps that preceded them | `greeks::solver::the_iteration_count_is_the_work_actually_done` | ✓ |
| G-32 | The reported cost is **every** model evaluation, checked against a count the solver did not compute — a thread-local counter inside `Checked::greeks`, the one function all of them pass through — on a Newton solve, on the worst solve and on a refusal | `greeks::solver::the_reported_cost_is_every_model_evaluation` | ✓ |
| G-33 | The scale the `Indeterminate` guard screens on is the sum of the two legs the price is a difference of, and it is measurably coarser than one ulp of the price itself — 100.8× on a one-day in-the-money NIFTY strike | `greeks::bsm::the_price_scale_is_the_two_legs_and_it_dwarfs_the_price_they_leave` | ✓ |

### Complexity rows for the greeks

Enforced by gate 8, `crates/greeks/benches/ratio.rs`, against the same **3.0×
shared-CI ceiling**. **These ids are `C-G-*` and not `G-*` on purpose.** This
file already carries two unrelated families spelled `G-01` … `G-09` — the rung
rows above and the greeks rows here — and gate 12 resolves a row id by the
first match, so a third family reusing that prefix would make the collision
worse. `C-G-*` is unambiguous against both, which is the lesson D-0045 recorded
when it renumbered `K-*` to `C-K-*`.

**The solver row is a different shape from the other two, and the difference is
the point.** `greeks::solver` opens by saying it is not O(1) and is not claimed
to be; what it has is an ARITHMETIC ceiling of `MAX_ITERATIONS` = 75 model
evaluations, held by `G-09` and `G-32`. Timing an easy solve against a hard one
and calling the ratio a bound would be measuring the evaluation COUNT, which is
allowed to differ. So `C-G-03` measures cost **per model evaluation** — the part
that has to be constant for a bounded count to bound anything at all. A solver
that did more work per step on a difficult quote would pass `G-09` and still be
unbounded in time.

| # | Must hold | Proven by | |
|---|---|---|---|
| C-G-01 | A closed-form price costs the same whatever it is pricing: at the money against a strike a tenth of spot and one ten times it, and one day to expiry against five years | `greeks::bench::a_price_costs_the_same_whatever_the_contract_is` | ✓ |
| C-G-05 | One Black-Scholes price costs a bounded multiple of the per-price **floor** — one fused multiply-add on the same contract's own fields, no `exp`, no CDF, no branch on moneyness. **This is the row a ratio cannot replace**: every other row here divides one price cost by another, so a UNIFORM slowdown cancels, and an audit measured a mask operation 174× slower passing its crate's ratio rows at 0.98×–1.00×. Measured over four runs: 29.966, 30.025, 29.798, 31.418 floors — a 1.05× spread, the tightest of the workspace's budgets because both legs are register float arithmetic. Budget **100**, sized on the worst observed with 3.18× left for a different microarchitecture | `greeks::bench::a_price_stays_within_its_budget` | ✓ |
| C-G-02 | The full greek set costs the same across those same contracts — every greek falls out of the same two normal-CDF evaluations, so none of them adds a data-dependent path | `greeks::bench::the_full_greek_set_costs_the_same_whatever_the_contract_is` | ✓ |
| C-G-03 | One **model evaluation** costs the same however hard the quote is, so `G-09`'s ceiling of 75 evaluations is a bound on work and not merely on step count — and the evaluation count each solve reports is checked against `MAX_ITERATIONS` in the bench as well as in the unit test | `greeks::bench::one_model_evaluation_costs_the_same_however_hard_the_quote_is` | ✓ |

Measured on the operator's machine, 2026-08-09, `cargo bench -p greeks`, exit 0.
C-G-01 spanned **0.943× – 0.998×** at ~23–25 ns a price; C-G-02 **0.996×** and
**1.011×**. C-G-03 is the one worth reading twice: the at-the-money solve
finished in **7** evaluations and the 20%-out-of-the-money one in **68**, both
under the ceiling of 75 — a **9.7× spread in count** — and the cost per
evaluation across that spread was **1.195×**. A bench that had timed the two
solves against each other would have reported ~9.7× and breached, on a crate
whose bound was never violated. That is the measurement this row exists to
avoid mis-stating.

A strike ten times spot is used in C-G-01 and C-G-02 but NOT in C-G-03: its
model price is close enough to zero that the solve is refused before it starts,
which is the no-arbitrage guard working rather than a cost to measure. No figure
from a CI runner is claimed, because none was taken.

## Greeks against the vendors — the external anchor

Every row here is measured against one live Dhan option-chain response. D-0046.

| # | Must hold | Proven by | |
|---|---|---|---|
| G-21 | The shipped closed form reproduces all eight of Dhan's published numbers from a fitted state, so a sign error, a missing discount factor or a wrong unit anywhere in `greeks::bsm` fails here. **None of the eight is an independent prediction** — six are inputs to the fit and the two gammas are the transposition criterion restated, which G-35 pins. The claim that the gammas were a prediction is withdrawn by D-0046 | `greeks::vendor_anchor::our_greeks_reproduce_the_captured_dhan_chain` | ✓ |
| G-22 | Vega is published per one percentage point: the raw scaling implies an index level of 258, the per-percent scaling 25,851 | `greeks::vendor_anchor::vega_is_published_per_percentage_point_and_the_index_level_proves_it` | ✓ |
| G-23 | Theta is published per a **calendar** day and not a trading day: the two sides of one strike agree on `r` 23 times better under 365 than under 252. **365 itself is not selected** — 375 beats it, and G-34 locates what the criterion actually picks | `greeks::vendor_anchor::the_trading_day_divisor_is_excluded_and_the_calendar_one_is_not_selected` | ✓ |
| G-24 | The two published volatilities are transposed, by a scale-free identity that contains no spot, strike, maturity or rate | `greeks::vendor_anchor::the_two_published_volatilities_are_transposed_and_the_identity_says_so` | ✓ |
| G-25 | `delta_call − delta_put = 1.00603` is reproduced by `N(d1c) + N(−d1p)` at `q = 0`, and a single volatility would need `q = −42.54%`. This says **nothing** about the carry — it inverts the two deltas, so it cannot fail for any deltas, volatilities or carry — and D-0046 withdraws the claim that it did | `greeks::vendor_anchor::the_delta_difference_is_reproduced_by_the_two_volatilities_and_says_nothing_about_the_carry` | ✓ |
| G-26 | `T` is recovered in closed form from the deltas and the volatilities alone and lands inside its 95% rounding-box interval — **conditional on `q = 0` and on the transposition.** The interval is a rounding-box width, not an identification result | `greeks::vendor_anchor::the_maturity_is_recovered_in_closed_form_from_the_deltas_and_the_volatilities` | ✓ |
| G-27 | The fit does **not** close, and the residuals are pinned so no later change can claim it does — with the spot residual, which is invariant to the day divisor from 252 to 500, separated from the rate residual, which is an artifact of choosing 365 | `greeks::vendor_anchor::the_two_sides_disagree_on_the_rate_and_the_disagreement_is_reported_not_hidden` | ✓ |
| G-28 | `crates/greeks` declares no dependency and no dev-dependency, and names no type from this workspace, so it can be taken by git URL on its own | **CI gate 9b.** `greeks::standalone::the_whole_public_surface_is_reachable_with_nothing_else_in_scope` is the companion and proves only the narrower thing its name says: the listed public items are reachable with nothing else in scope. An integration test cannot see a leak it does not name, and a `pub use` adds no region for coverage to see. D-0046 | ✓ |
| G-34 | The criterion that excludes a 252-day divisor has its root at `D* = 370.0757`, where the two sides agree on `r` to 2.8e-15 points, and the spread is monotone in the divisor across 250–450 so no second root hides at 365 | `greeks::vendor_anchor::the_rate_criterion_has_its_root_at_370_and_365_is_a_convention_near_it` | ✓ |
| G-35 | The fitted gammas are `n(d1)^2/(100·vega·sigma)` and are therefore **invariant to `r`, `T`, `S` and `K`** — identical to fifteen digits with `r` forced from −50% to +200% and `T` scaled 0.25× to 10× — so reproducing them confirms nothing about any of those | `greeks::vendor_anchor::the_two_gammas_are_invariant_to_the_rate_and_the_maturity` | ✓ |
| G-36 | **No single contract reproduces the chain.** The two deltas force the model's vega ratio to 0.998641 against the vendor's 1.001360, a quantity containing no `S`, `K`, `T` or `r`; the best possible single contract is off by 884× the vendor's own display half-ulp | `greeks::vendor_anchor::no_single_contract_reproduces_the_chain_and_the_shortfall_is_a_number` | ✓ |
| G-37 | `q = 0` is **consistent** with the sample and not implied by it: `q = 1%` and `q = 2%` reproduce all eight published fields, gammas included, at maturities a third shorter | `greeks::vendor_anchor::the_carry_is_consistent_with_zero_and_the_sample_cannot_pin_it` | ✓ |

## How this file stays honest

1. A new guarantee anywhere in the codebase adds a row **here first**.
2. The row names a test. Not a plan for a test.
3. CI checks that each named test exists. A row pointing at nothing fails the
   build, which is what stops this file from decaying into a wish list.
4. Rows are never deleted to make a build pass. Either the invariant holds or
   the decision to abandon it is recorded in `docs/05-decisions.md`.

---

## Transaction costs

Added by D-0041, and appended at the end of the file rather than beside the
other sections because two other changes were editing this file at the same
time. Every row is proven by a test that runs today.

| # | Must hold | Proven by | |
|---|---|---|---|
| K-01 | A date in an unverified window produces a refusal and never a number, on both venues and on every representable day before the boundary | `costs::regime::the_exchange_charge_refuses_exactly_the_pre_boundary_days_and_no_others` · `costs::regime::the_exchange_charge_prices_the_boundary_and_refuses_the_day_before_it` | ✓ |
| K-02 | An unverified row carries no numeric field at all, so there is nothing to unwrap, default or configure — the escape hatch does not exist rather than being closed | `costs::regime::an_unverified_row_carries_no_number_for_anything_to_reach` | ✓ |
| K-03 | The refusal names the circulars that were identified, says none was retrieved, and carries the one action that would close the window | `costs::regime::the_refusal_names_the_circulars_that_were_identified_and_never_retrieved` · `costs::error::a_refusal_names_the_charge_the_venue_the_window_and_the_remedy` | ✓ |
| K-04 | A refusal window is derived from the table, never written down a second time, and a table with no refusal row reports none | `costs::regime::a_refusal_window_is_derived_from_the_table_and_never_written_twice` | ✓ |
| K-05 | A regime boundary is inclusive on its own day: the day before and the day of resolve to different rates, on every shipped boundary | `costs::regime::stt_on_options_premium_holds_on_both_sides_of_both_boundaries` · `costs::regime::stt_on_exercise_holds_on_both_sides_of_its_boundary` | ✓ |
| K-06 | The lookup agrees with a naive last-row-that-started scan on **every** representable day against **every** shipped table, and is idempotent | `costs::regime::the_lookup_agrees_with_a_naive_scan_on_every_representable_day` | ✓ |
| K-07 | Each rate covers exactly the days it should — asserted as a histogram over the whole 40,542-day domain, so an unshipped rate anywhere is a failed comparison rather than a value folded into a neighbouring bucket | `costs::regime::a_boundary_is_inclusive_on_its_own_day_across_the_whole_domain` | ✓ |
| K-08 | Every shipped table anchors at the first representable day, ascends strictly, has no hole, carries no negative rate and carries no blank citation — and the compiler, not a test, is what enforces it | `costs::regime::every_shipped_table_passes_the_validation_the_compiler_already_ran` · `costs::regime::the_validation_rejects_every_way_a_table_can_be_wrong` | ✓ |
| K-09 | Every flat rate is the figure its citation gives, and the `bps_x100` scale reads as rupees per crore — the reading the circulars themselves quote | `costs::rate::every_flat_rate_is_the_cited_figure` · `costs::rate::the_scale_reads_as_rupees_per_crore` | ✓ |
| K-10 | Stamp duty is charged on the buy leg and is zero on the sell leg, so a round trip pays it once | `costs::rate::stamp_duty_is_charged_on_the_buy_leg_and_never_on_the_sell_leg` | ✓ |
| K-11 | Brokerage is per executed **order** and flat for both brokers — a round trip is two orders, and lots do not enter it | `costs::rate::brokerage_is_flat_per_order_and_both_brokers_are_priced` | ✓ |
| K-12 | An underlying outside `CLAUDE.md` §1's engine surface is refused and never defaulted onto a venue, and the costable set is that surface read from `core` rather than a copy of it | `costs::venue::an_underlying_outside_the_engine_surface_is_refused_and_never_defaulted` · `costs::venue::the_costable_set_is_the_engine_surface_itself_and_cannot_drift_from_it` | ✓ |
| K-13 | A date outside the representable window is refused **by name**, distinct from a date that is not real; an impossible date is refused rather than normalised | `costs::day::refuses_a_year_outside_the_window_by_name` · `costs::day::refuses_an_impossible_date_rather_than_normalising_it` | ✓ |
| K-14 | The day ordinal is a bijection onto a contiguous integer range across all 111 years, so one date compare is one integer compare | `costs::day::every_representable_day_is_one_ordinal_after_the_one_before_it` · `costs::day::the_ordinals_are_the_hand_computed_ones` | ✓ |
| K-15 | This crate's year window is `core`'s, and a widening of `core` fails here rather than silently widening the rate tables | `costs::day::the_year_window_is_cores_and_a_drift_would_be_caught` | ✓ |
| K-16 | A refusal's `Display` propagates a writer error from **every** part it writes, rather than reporting success over a truncated refusal | `costs::error::display_propagates_a_writer_error_from_every_part_it_writes` | ✓ |

### Complexity rows for the same crate

These were appended as **C-14 through C-17** and those four ids were already
taken, by the `crates/api` rendering rows above. **Renumbered to `C-K-01`
through `C-K-04` by D-0045** — the ids `crates/costs/benches/ratio.rs` has been
printing all along. Enforced by gate 8, which runs that bench, and held to the
same **3.0× shared-CI ceiling**.

| # | Must hold | Proven by | |
|---|---|---|---|
| C-K-01 | A regime lookup costs the same whichever row it selects — the anchor row and the last row of the same table | `costs::bench::the_selected_row_does_not_change_the_cost` | ✓ |
| C-K-02 | A regime lookup costs the same on a two-row table as on a three-row one, so the trip count is not being paid for at runtime | `costs::bench::the_row_count_does_not_change_the_cost` | ✓ |
| C-K-03 | Refusing costs what pricing costs — a refusal is not a slow path callers learn to avoid asking for | `costs::bench::a_refusal_costs_what_a_rate_costs` | ✓ |
| C-K-04 | The pre-boundary window is **refused**, not merely refused quickly — speed is half the claim and a lookup that got fast by returning the current rate would pass every ratio above | `costs::bench::the_pre_boundary_window_still_refuses` | ✓ |

Measured on the operator's machine, 2026-08-07, over **three** runs: 1.7–4.3 ns
per lookup, worst ratio **1.552×** — and that worst figure came from the run
taken while other work was compiling on the same machine, where the baseline
itself moved from 2,127 ps to 3,833 ps. The two quiet runs both topped out at
**1.189×**. No figure from a CI runner is claimed, because none was taken.

## Transaction costs — the option arithmetic

Appended for the same concurrency reason as the section above: three agents held
this file open. K-17 through K-34 and C-18 through C-22 belong to
`crates/costs` stage 2 (`docs/05-decisions.md` D-0043). Every row names a test
that runs today.

| # | Must hold | Proven by | |
|---|---|---|---|
| K-17 | The at-the-money rung is exact integer half-up rounding onto the grid, with a tie going to the **higher** strike — and the source's own float pins land on the same answers | `costs::strike::a_spot_exactly_halfway_between_two_rungs_rounds_up` · `costs::strike::the_source_pins_land_on_the_source_answers` | ✓ |
| K-18 | The rung is the **nearest** one, checked against an independently written nearest-multiple search over every paisa of two whole steps, and it never moves backwards as the spot rises | `costs::strike::the_rung_is_the_nearest_one_for_every_spot_across_two_whole_steps` · `costs::strike::the_rung_never_moves_backwards_as_the_spot_rises` | ✓ |
| K-19 | The rounding is exact for a step that does not divide the spot evenly, including an **odd** step in paisa — the case the predecessor's float chain could only approximate | `costs::strike::a_step_that_does_not_divide_the_spot_evenly_is_still_exact` | ✓ |
| K-20 | The snap happens **once**: a rung re-rounded is itself, and a resolved strike is already on the grid | `costs::strike::the_snap_happens_once_and_a_second_pass_changes_nothing` | ✓ |
| K-21 | A spot below half a step has no rung and is **refused**, not struck at zero; a zero or negative spot is refused **by name**; a rung past `i64` is refused rather than wrapped | `costs::strike::a_spot_below_the_lowest_rung_is_refused_rather_than_struck_at_nothing` · `costs::strike::a_zero_or_negative_spot_is_refused_by_name` · `costs::strike::a_spot_above_the_highest_representable_rung_is_refused_rather_than_wrapped` | ✓ |
| K-22 | "Plus" is further **out** of the money in the trade direction: a call moves up and a put moves down, and the two sides are exact mirrors about the rung | `costs::strike::plus_is_further_out_of_the_money_in_the_trade_direction` · `costs::strike::the_two_sides_are_exact_mirrors_of_each_other_about_the_rung` | ✓ |
| K-23 | A moneyness that walks off the grid is refused by name — including `i32::MIN`, whose negation has no `i32` | `costs::strike::a_moneyness_that_walks_off_the_grid_is_refused_rather_than_wrapped` | ✓ |
| K-24 | The two swept underlyings carry **different** published steps (50 and 100 rupees), and the same strike therefore reads as a different moneyness on each | `costs::strike::the_two_swept_underlyings_carry_the_two_published_steps` · `costs::moneyness::the_two_grids_disagree_about_the_same_strike_and_that_is_the_point` | ✓ |
| K-25 | The offset rounds **half toward zero**, so `moneyness == 0` is exactly the at-the-money band — proven against an independent re-derivation of the source's own classify rule over every paisa of two whole steps, on both grids and both sides | `costs::moneyness::the_bucket_is_the_sign_of_the_moneyness_and_the_band_edge_agrees` · `costs::moneyness::a_half_step_tie_goes_down_to_the_rung_at_the_money_names` | ✓ |
| K-26 | The offset matches an independently written nearest-multiple search across four whole steps, and the source's own decimal-oracle pins | `costs::moneyness::the_offset_matches_a_nearest_multiple_search_across_four_whole_steps` · `costs::moneyness::the_source_offset_pins_land_on_the_source_answers` | ✓ |
| K-27 | Resolving a strike from a moneyness and reading the moneyness back off it is the **identity**, on both sides and both grids | `costs::moneyness::the_moneyness_and_the_strike_are_exact_inverses_of_each_other` | ✓ |
| K-28 | Moneyness is defined at zero, at the documented chain edges (±10) and one past each — and one past is **outside the chain yet still resolvable**, because the chain is a query and never a refusal | `costs::moneyness::moneyness_at_zero_at_the_chain_edges_and_one_past_each` · `costs::moneyness::the_seven_locked_offset_rules_read_as_the_source_spells_them` | ✓ |
| K-29 | The lot size in force is the source's figure on **every** transition date and on the day before it, and the two underlyings differ on the same day | `costs::lot::every_transition_and_the_day_before_it_are_the_source_figures` · `costs::lot::the_two_underlyings_carry_different_lots_on_the_same_day` | ✓ |
| K-30 | The quantity is `lots × lot size` and nothing else; a lot count that is not a trade is refused by name; an overflow is refused rather than wrapped or saturated | `costs::lot::the_quantity_is_the_product_and_nothing_else` · `costs::lot::a_lot_count_that_is_not_a_trade_is_refused_by_name` · `costs::lot::a_quantity_past_i64_is_refused_rather_than_wrapped` | ✓ |
| K-31 | The next weekly expiry is the **first** day on or after the day asked with the regime's weekday — checked by brute-force day scan across the whole verified window on both underlyings | `costs::expiry::the_next_weekly_is_the_first_day_on_or_after_with_the_regimes_weekday` | ✓ |
| K-32 | The next monthly expiry is the last regime weekday of its own month or the next, on every day of the verified window — and the regime is **re-read** for a rolled month rather than carried across it | `costs::expiry::the_next_monthly_is_the_last_regime_weekday_of_its_own_or_the_next_month` · `costs::expiry::the_regime_is_re_read_for_the_rolled_month_and_not_carried_across` | ✓ |
| K-33 | A withdrawn weekly contract is a **cited value**, never a refusal, and the two never read the same | `costs::expiry::a_withdrawn_weekly_and_a_refusal_never_read_the_same` | ✓ |
| K-34 | Every day before the source's own recorded history **refuses**, on every stage-2 table, and the refusal is carried out of the date functions word for word rather than replaced by a vaguer one | `costs::lot::the_pre_history_window_refuses_on_every_day_of_it_and_never_after` · `costs::expiry::the_pre_history_window_refuses_on_both_tables_and_every_day_of_it` · `costs::strike::the_step_is_refused_before_the_window_the_source_verified` | ✓ |
| K-35 | The day ordinal round-trips exactly over all 40,542 representable days, the weekday advances one day at a time across all of them, and an ordinal off either end is refused by name rather than saturated | `costs::day::from_ordinal_is_the_exact_inverse_of_ordinal_over_the_whole_window` · `costs::day::the_weekday_walks_forward_one_day_at_a_time_across_the_whole_window` · `costs::day::an_ordinal_off_either_end_is_refused_by_name_and_never_saturated` | ✓ |
| K-36 | The per-underlying tables are keyed on `core`'s own `SWEPT` order, asserted at **compile time**, so a reorder or a widening is a build failure rather than a silently swapped lot size | `costs::venue::a_slot_is_cores_own_index_and_round_trips_to_cores_own_row` · `costs::strike::the_step_tables_are_keyed_on_cores_order_and_every_row_is_positive` · `costs::lot::every_shipped_lot_row_is_positive_and_keyed_on_cores_order` · `costs::expiry::every_shipped_expiry_row_is_a_trading_weekday_and_keyed_on_cores_order` | ✓ |
| K-37 | The generic dated lookup agrees with a naive last-row-that-started scan on **every** representable day, and every way a table can be shaped wrong is caught by the compiler | `costs::dated::the_lookup_agrees_with_a_naive_scan_on_every_representable_day` · `costs::dated::every_shape_violation_is_caught_and_a_sound_table_is_not` | ✓ |

### Complexity rows for the option arithmetic

Enforced by gate 8, `crates/costs/benches/ratio.rs`, against the same **3.0×
shared-CI ceiling**. Renumbered from **C-18 … C-22** by D-0045, onto the ids the
bench prints.

| # | Must hold | Proven by | |
|---|---|---|---|
| C-K-05 | The at-the-money rung costs the same wherever the spot is — a 50-rupee spot and a spot near `i64::MAX` | `costs::bench::the_spot_magnitude_does_not_change_the_rung_cost` | ✓ |
| C-K-06 | The resolved strike costs the same however deep the moneyness — `ATM`, the chain edge, and a million steps out — and reading a moneyness off a far strike costs what reading it off a near one does | `costs::bench::the_moneyness_depth_does_not_change_the_strike_cost` | ✓ |
| C-K-07 | The quantity costs the same for one lot and for a million, so the multiplication has not become an accumulation | `costs::bench::the_lot_count_does_not_change_the_quantity_cost` | ✓ |
| C-K-08 | The expiry costs the same wherever in the calendar it is asked: the in-month arm against the rollover arm, January against a December that rolls the year, and zero days ahead against six | `costs::bench::the_calendar_position_does_not_change_the_expiry_cost` | ✓ |
| C-K-09 | The stage-2 pre-history windows are **refused**, not merely refused quickly — a lot-size lookup that got fast by handing back the first recorded lot for a 2019 trade would pass every ratio above | `costs::bench::the_stage_two_pre_history_windows_still_refuse` | ✓ |

Measured on the operator's machine, 2026-08-07, over four runs. Per-call cost
0.62–15.7 ns. Every ratio held under 3.0×; the largest was **2.300×**, the
monthly rollover arm against the in-month arm, and that figure is the bound
working as stated rather than a scan appearing: the rollover arm does the month
resolution and the table lookup **twice**, which is exactly the "at most two"
the claim is. Every other stage-2 ratio, across all four runs, stayed inside
0.81×–1.14×. No figure from a CI runner is claimed, because none was taken.

## Transaction costs — the round trip

`crates/costs` stage 3. Every row is proven by a test in `crates/costs`; the
worked-example rows are the predecessor repository's own enforced oracles from
`COSTS_VERIFIED` §5, reproduced to the paisa. See `docs/05-decisions.md` D-0044
and `docs/06-limits.md` §27.

| # | Must hold | Proven by | |
|---|---|---|---|
| K-38 | The four worked examples price to the **paisa** — one NIFTY lot, five NIFTY lots, one BANKNIFTY lot and the BSE rate set — with every rate and every lot size read out of this crate's own dated tables rather than restated in the test | `costs::trip::the_first_worked_example_prices_one_nifty_lot_to_the_paisa` · `costs::trip::the_second_worked_example_amortises_the_flat_brokerage_over_five_lots` · `costs::trip::the_third_worked_example_prices_one_banknifty_lot` · `costs::trip::the_fourth_worked_example_prices_the_bse_rate_set_through_the_pure_core` | ✓ |
| K-39 | The transaction tax reads the **sell** premium and nothing else: not the buy leg, not both legs, and never `strike × quantity` — with each wrong answer priced out beside the right one | `costs::trip::the_transaction_tax_reads_the_sell_premium_and_nothing_else` | ✓ |
| K-40 | Stamp duty is charged on the **buy** leg exactly once, and the sell-side rate is zero so the sell leg cannot be charged even deliberately | `costs::trip::the_stamp_duty_is_charged_on_the_buy_leg_and_exactly_once` | ✓ |
| K-41 | GST is 18% on the services base — brokerage, exchange charge, SEBI fee and IPFT, each already rounded, with the tax and the stamp duty excluded — rounded **once**; rounding two 9% halves separately overcharges by exactly ₹1, computed rather than asserted | `costs::trip::the_gst_is_rounded_once_and_rounding_each_half_overcharges_by_a_rupee` · `costs::trip::the_gst_base_is_the_sum_of_all_four_service_components_with_a_plus_sign` | ✓ |
| K-42 | Brokerage is per executed **order**, flat: a thousand lots pay what one lot pays, on both brokers, while every other charge scales | `costs::trip::the_brokerage_is_flat_per_order_and_a_thousand_lots_pay_what_one_pays` | ✓ |
| K-43 | A round trip whose entry day lands in an unverified window **refuses entirely** — no charge is priced at zero and no current rate is applied backwards — and the refusal carries the window, the citation gap and the remedy out whole | `costs::trip::a_round_trip_in_the_unverified_window_refuses_the_whole_trip_and_names_it` · `costs::trip::the_dated_pair_carries_whichever_of_the_two_lookups_refused` | ✓ |
| K-44 | Every regime is keyed on the **entry** day: a round trip that straddles a tax boundary is priced at the regime it opened under, and the lot size is the entry day's too | `costs::trip::the_regime_is_the_entry_days_and_the_exit_day_never_moves_it` · `costs::trip::the_lot_size_is_the_entry_days_and_a_pre_history_entry_refuses` | ✓ |
| K-45 | Each leg fills on the adverse extreme of its **own** bar plus one further tick, the two directions read different anchors off the same two bars, the sell floor binds at one tick and shortens the realized slippage with it, and the buy leg needs no floor because no legal bar can reach one | `costs::fill::a_long_fills_the_entry_high_and_the_exit_low_each_one_tick_adverse` · `costs::fill::a_short_fills_the_exit_high_and_the_entry_low_and_is_adverse_on_both_legs` · `costs::fill::the_sell_floor_binds_at_one_tick_and_the_realized_slippage_follows_it` · `costs::fill::the_buy_leg_needs_no_floor_because_no_legal_bar_can_reach_it` | ✓ |
| K-46 | The worst-case fill never flatters an open-anchored one, at either bracket end of any bar, on either direction | `costs::fill::the_worst_case_fill_never_flatters_an_open_anchored_one` | ✓ |
| K-47 | The two rounding laws are different functions: a levy ceils to the paisa per leg and is summed after, a statutory levy floors to the paisa and then ceils to the whole rupee — and each is the **least** integer at or above its quotient, checked densely and at every remainder that can flip it | `costs::money::the_statutory_raw_stage_floors_where_the_levy_stage_ceils` · `costs::money::the_ceiling_is_the_least_integer_at_or_above_the_quotient` · `costs::money::the_rupee_ceiling_is_the_least_whole_rupee_at_or_above_the_amount` | ✓ |
| K-48 | An index spot or index future is priced **signal-only** — every charge zero, net equal to gross, the fills unchanged — and it consults no rate, so it prices inside a window where every rate refuses | `costs::scope::only_the_option_segment_bears_the_charge_stack` · `costs::trip::a_signal_only_segment_pays_nothing_and_its_net_is_its_gross` · `costs::trip::a_signal_only_segment_prices_inside_a_window_where_every_rate_refuses` | ✓ |
| K-49 | An expiry outcome is **refused**, never priced with premium arithmetic; an exit before its entry, a lot count on a segment with no options lot table, a zero or negative quantity, an inverted bar and a sub-tick bar high are each refused **by name** | `costs::trip::an_expiry_outcome_is_refused_rather_than_priced_as_a_normal_close` · `costs::trip::an_exit_day_before_its_entry_day_is_refused` · `costs::trip::a_lot_count_is_refused_for_a_segment_the_options_lot_table_does_not_cover` · `costs::trip::a_round_trip_of_no_contracts_is_refused_by_name_on_every_path` · `costs::fill::a_bar_whose_low_is_above_its_high_is_refused_rather_than_swapped` · `costs::fill::a_bar_whose_high_is_below_one_tick_is_refused_by_name` | ✓ |
| K-50 | Every arithmetic site that can leave `i64` is refused **by name** and never wrapped or saturated — both notionals, the slippage line, the net, each per-leg levy, each two-leg sum, the GST base, the total, and each statutory stage | `costs::trip::every_position_overflow_site_is_refused_by_name_and_never_wrapped` · `costs::trip::every_levy_overflow_site_is_refused_by_name_and_never_wrapped` · `costs::trip::every_flat_levy_overflow_site_is_refused_by_name_and_never_wrapped` · `costs::trip::the_sell_leg_of_a_per_leg_levy_is_guarded_independently_of_the_buy_leg` · `costs::trip::the_statutory_rupee_ceiling_is_reachable_from_the_stack_and_refuses` · `costs::trip::a_signal_only_round_trip_guards_its_arithmetic_too` · `costs::fill::a_fill_or_a_slippage_past_i64_is_refused_by_name_and_never_wrapped` · `costs::money::a_result_past_i64_is_refused_by_name_and_never_saturated` | ✓ |
| K-51 | A breakdown's own two laws hold on every swept combination — the total is the sum of its seven itemised charges and the net is the gross less the total — and a breakdown that broke either is **refused rather than reported** | `costs::trip::the_internal_laws_hold_across_a_deterministic_sweep_of_the_envelope` · `costs::trip::a_breakdown_that_broke_its_own_law_is_refused_rather_than_reported` · `costs::trip::the_breakdown_itemises_seven_charges_that_sum_to_its_total` | ✓ |
| K-52 | A losing round trip still pays every charge, and a round trip whose charges exceed its gross reports a negative net — both computed, not asserted | `costs::trip::a_losing_round_trip_reports_a_negative_net_and_still_pays_every_charge` · `costs::trip::a_round_trip_whose_charges_exceed_its_gross_reports_a_negative_net` | ✓ |
| K-53 | The entry point is the pure core plus the resolution and nothing else, on every swept combination of underlying, date, direction and segment | `costs::trip::the_entry_point_and_the_pure_core_agree_on_every_swept_combination` | ✓ |

### Complexity rows for the round trip

Enforced by gate 8, `crates/costs/benches/ratio.rs`, against the same **3.0×
shared-CI ceiling**.

Renumbered from **C-23 … C-25** by D-0045, onto the ids the bench prints.

| # | Must hold | Proven by | |
|---|---|---|---|
| C-K-10 | The charge stack costs the same however big the trade is: one lot against a million, a five-paisa premium against a lakh-rupee one, and a winning trip against a losing one | `costs::bench::the_trade_size_does_not_change_the_charge_stack_cost` | ✓ |
| C-K-11 | The whole entry point costs the same whatever it is asked: the trade's size, its date and its underlying do not move it, and a signal-only trip never costs **more** than a cost-bearing one | `costs::bench::the_entry_point_costs_the_same_whatever_it_is_asked` | ✓ |
| C-K-12 | The unverified window **refuses the whole round trip** — not merely refuses it quickly — while the day after prices and a signal-only trip in the same window prices at zero | `costs::bench::the_stage_three_refusals_are_still_refusals` | ✓ |
| C-K-13 | One charge stack costs a bounded multiple of the per-stack **floor** — one multiply and one add on the same two fill prices, no rate table, no rounding, no branch on side. **This is the row a ratio cannot replace**: every other row here divides one charge-stack cost by another, so a UNIFORM slowdown cancels, and an audit measured a mask operation 174× slower passing its crate's ratio rows at 0.98×–1.00×. Measured over four runs: 36.140, 36.942, 37.388, 39.627 floors, budget **120**, sized on the worst observed with 3.03× left for a different microarchitecture | `costs::bench::the_charge_stack_stays_within_its_budget` | ✓ |

Measured on the operator's machine, 2026-08-07, over four runs. Per-call cost
32.9–37.6 ns for `charge_stack` and 37.3–41.2 ns for `price`. Every ratio held:
C-23 spanned 0.895×–1.030× and C-24's three size/date/underlying ratios spanned
0.954×–1.039×. The two figures far **below** 1.0× are the two arms that do less
work by design and are reported rather than hidden: a signal-only trip resolves
no rate (0.128×–0.137×) and a refused trip stops at the first refusal
(0.164×–0.179×). No figure from a CI runner is claimed, because none was taken.

## The lake reader — a footer that parses and lies

`crates/lake` reads the 40 GB Parquet lake at `~/.brutex/lake`. For every
expired contract in there the lake is the only copy, so a file that is damaged
must be *named*, never guessed at and never fatal — a reader that dies on one
file takes the sweep that was part-way through the other 115 with it.

**This section was one row and is now eight, and it is still not the crate's
full set.** The rest of `lake`'s invariants are not in this file yet. That is a
gap and it is written down rather than papered over; see `docs/06-limits.md`.

| # | Must hold | Proven by | |
|---|---|---|---|
| L-01 | A column chunk whose declared offset or length is **negative** is refused as `ImpossibleLength`, naming the offending number, and the process survives — from either field the chunk start can come from | `lake::synthetic::a_negative_chunk_length_in_the_footer_is_refused_by_name_and_never_aborts`, `lake::synthetic::a_negative_chunk_offset_is_refused_by_name_from_either_field_it_can_come_from` | ✓ |
| L-02 | A column chunk that delivers fewer rows than its row group declares is refused as `ShortColumnChunk` **naming the column, the row it diverged at, and both counts** — never accepted with the tail filled in. A `num_values` short of `num_rows` and a chunk cut on an exact page boundary are both this refusal | `lake::reader::fewer_levels_than_the_row_count_is_refused_as_a_short_chunk`, `lake::refusals::a_column_chunk_short_of_its_row_count_is_refused_and_never_fills_the_tail_with_nulls`, `lake::refusals::a_column_chunk_whose_bytes_were_cut_is_refused_rather_than_padded_with_nulls` | ✓ |
| L-03 | A shortfall on a **required** column is reported as the shortfall, not as `UnexpectedNull`. Byte loss and a vendor gap are different faults and the reader says which | `lake::refusals::a_short_chunk_on_a_required_column_names_the_shortfall_not_a_phantom_null`, `lake::reader::a_short_chunk_says_which_column_ran_out_where_and_by_how_much` | ✓ |
| L-04 | Fewer values than the definition levels claim is refused, never resolved into a `None` on a row whose level says PRESENT | `lake::reader::fewer_values_than_the_levels_claim_is_refused_and_never_invents_a_null`, `lake::refusals::parquet_itself_refuses_a_page_whose_values_are_short_of_its_definition_levels` | ✓ |
| L-05 | A leaf that is not a flat `OPTIONAL` primitive — nested, `REQUIRED` or `REPEATED` — is refused as `UnsupportedColumnShape` **at the schema gate**, before a page is decompressed, naming the column and both levels | `lake::refusals::a_nested_column_is_refused_rather_than_silently_decoding_to_all_null`, `lake::refusals::a_required_or_repeated_leaf_is_refused_by_name_rather_than_read_as_flat` | ✓ |
| L-06 | Two distinct lake directory names never parse to one `InstrumentKey`. The month's case tolerance is injective over the twelve tokens; the strike carries no tolerance at all, and a leading zero is refused | `lake::contract::the_month_case_tolerance_is_injective_and_can_never_alias_two_contracts`, `lake::contract::a_leading_zero_strike_is_refused_rather_than_aliased_onto_another_contract`, `lake::refusals::a_leading_zero_strike_is_refused_rather_than_aliased_onto_another_contract` | ✓ |
| L-07 | Every refusal above fires on **damage only**: 1,737 real lake files across both layouts and seven years decode to the identical digest they did before the refusals existed | `lake::real_lake_regression::a_wide_sample_of_the_real_lake_decodes_with_no_refusal_and_a_stable_digest` | ◐ |
| L-08 | Each of this crate's three `emit` sites **reaches a file**, driven through its production entry point and read back off disk. One line per file and no more; a schema refusal names the column and precedes the `lake.file` line for the same file; the level and the row count move with the outcome, so an empty lake and an unreadable one are not one line | `lake::page::a_refused_page_writes_its_reason_to_the_log`, `lake::events::every_file_this_reader_opens_or_refuses_writes_its_line_to_the_log` | ✓ |

L-01 is not a hypothetical. `parquet`'s own `ColumnChunkMetaData::byte_range`
ends in `assert!(col_start >= 0 && col_len >= 0)`, and the reader called it. A
thrift footer that parses perfectly well can still carry a negative
`total_compressed_size` or `dictionary_page_offset` — that is what a bad sector,
an interrupted write or a truncated object body leaves behind — so the two
fields are now read and checked in `Columns::pages` before any arithmetic, and
`byte_range` is not called at all. There was nothing to catch: `[profile.release]`
sets `panic = "abort"`.

**L-02 to L-05 replaced a fabrication, and one of them replaced a comment that
said the opposite of its code.** `Columns::expand` walked `0..num_rows` and
pushed `None` for every row the decoded definition levels did not reach, so a
chunk delivering 400 of 2,480 rows was *accepted* with 2,080 nulls this crate
invented — the first at row 400, whose true value was 1,400. On `open_interest`
each invented null is an invented `i64::MIN`, which `CLAUDE.md` §7 defines as
*the vendor reported none*, and nothing downstream can tell it from one the
vendor really sent. Beside it, a unit test asserted `expand(&[], &[1], 1) ==
vec![None]` under the comment *"Fewer values than levels claim: refuses to
invent one"* — a null invented on a row whose level says PRESENT, described as a
refusal. Both are D-0063.

**L-05 is the one that was invisible.** A `ColumnDescriptor`'s `name()` is the
*leaf* name, so an `open_interest` one optional group deep presents as
`open_interest` with the right physical type and passed every check `detect`
made. Its definition levels then run 0..=2 while `expand` read "present" as the
single level 1, so a file genuinely holding 7, 8, 9 decoded to three nulls and
was accepted. The shape is now refused rather than decoded, because a level-2
leaf distinguishes a null group from a null leaf inside a present group and
`Bar` has one `None` for both; `docs/06-limits.md` §37 records what that costs.

**L-07 is `◐` and not `✓` for the reason `tests/real_lake.rs` gives.** The
fixture is 40 GB of operator data that CI gate 1 forbids committing, so on every
CI runner the test prints that it is skipping and proves nothing. Where it has
been run — the operator's machine, 2026-08-09 — it read 1,737 files, 11,526,017
rows, 189 F&O and 1,548 cash/index, and every per-file digest and the whole-run
digest `5bb7cad9d96bb347` were byte-identical before and after the refusals were
added. That is the measurement that says these refusals cost nothing on sound
data; it is not a claim about CI.

**L-08 is about the events, and it exists because every refusal above was
provable while the record of it was not.** CI gate 18 mutated one `emit` to
`()` — deleting it outright — and the whole suite stayed green, which was a true
statement about all three of this crate's emit sites and about 54 of the
workspace's 56. Every test in `synthetic.rs` and `refusals.rs` asserts on the
`Result` and none asserts that anything was *recorded*, so the lines an operator
reads at hour eleven of a backfill could have been deleted with no gate
noticing. The three sites are covered from two test binaries and not one,
because `telemetry::install` writes a per-process `OnceLock` and refuses a
second call: the page site is proven from the lib target, and `tests/events.rs`
— its own binary, its own process, its own `OnceLock` — writes six Parquet
fixtures to a temporary directory and drives `LakeFile::open` over each. It must
be `open` and not `from_bytes`: the `lake.file` line lives on the path-taking
entry point, and the entry point every other test in the crate uses emits
nothing at all. The floor is `Trace`, because a successful open speaks at
`Debug` and the default `Info` floor would filter the line the test is there to
find.

### Complexity rows for the lake reader

Enforced by gate 8, `crates/lake/benches/ratio.rs`, against the same **3.0×
shared-CI ceiling**. The bench was tracked and wired with `harness = false`
before these rows existed, and gate 14 refused `crates/lake` for exactly that
reason: a crate that claims a bound and names no measurement. The measurement
was already there; what was missing was the row that points at it.

`crates/lake` makes **one** O(1) claim and the other two rows are the honest
super-constant ones beside it, stated so that neither is mistaken for the other.
Opening a file is O(file bytes) and decoding a row group is O(bytes in the
group) — benching those would measure zstd, not this crate — so they are not
here, and `docs/06-limits.md` is where that gap is recorded.

| # | Must hold | Proven by | |
|---|---|---|---|
| C-L-01 | `Batch::row` costs the same at 2,480, 24,800 and 248,000 rows, and the **last** row of a 248,000-row batch costs what its **first** row costs — the form a scan would fail by 248,000× | `lake::bench::row_lookup_is_constant_in_batch_size` | ✓ |
| C-L-02 | A full walk stays linear: cost **per row** does not grow from 2,480 rows to 248,000, so iteration is not quadratic in the batch | `lake::bench::iteration_is_linear_per_row` | ✓ |
| C-L-03 | Parsing a contract name does not scan it — a 4 KiB name that is **refused** never costs more than a 26-byte name that is accepted | `lake::bench::contract_parse_does_not_scan_the_name` | ✓ |
| C-L-04 | One row lookup costs a bounded multiple of the per-row **floor** — one length read and an add on the same batch, no row decode. **This is the row a ratio cannot replace**: C-L-01 divides one lookup by another, so a UNIFORM slowdown cancels, and an audit measured a mask operation 174× slower passing its crate's ratio rows at 0.98×–1.00×. Measured 10.071 / 10.011 / 10.011 floors — **the ratio held at ten while the floor itself moved 2.1× between runs**, which is the whole argument for measuring against a floor rather than a wall-clock number. Budget **40** | `lake::bench::row_lookup_stays_within_its_budget` | ✓ |

Measured on the operator's machine, 2026-08-09, `cargo bench -p lake`, exit 0.
C-L-01 spanned **0.986× – 1.073×** across its four comparisons, at 4.5–4.9 ns
per lookup. C-L-02 measured **1.007×** at 252 ps and 254 ps per row. C-L-03
measured **0.082×** — 41.1 ns to accept the real 26-byte name against 3.4 ns to
refuse the 4 KiB one — and that figure is far below 1.0× rather than near it
**because the refusal is the cheaper path, not because the parser is fast**:
the length is rejected before the name is walked. It is reported rather than
hidden, for the same reason C-K-11's two sub-1.0× arms are. No figure from a CI
runner is claimed, because none was taken.

## The audit console's one read — `/audit.json`

`crates/api/src/audit_json.rs` serves the journal and one feed's month roll-up
to the browser console. It is the route an operator refreshes for twelve hours,
so its cost is a property that has to hold, not a hope. D-0059.

| # | Must hold | Proven by | |
|---|---|---|---|
| AJ-01 | A read is bounded by the PAGE, never by the journal: at most `audit::MAX_PAGE_RECORDS` records leave disk however long the file is | `api::audit_json::a_page_reads_at_most_the_records_it_shows`, `api::audit::the_tail_reads_only_the_records_it_shows` | ✓ |
| AJ-02 | A `page` past the end **clamps** to the last page and answers; it does not refuse, and it does not answer page 0 of an empty list | `api::audit_json::a_page_past_the_end_clamps_to_the_last_page_rather_than_refusing` | ✓ |
| AJ-03 | A missing or unknown `feed` is a **400 naming the parameter**, never a defaulted vendor. `ingest::parse_vendor("")` answers `Some(Dhan)`, so the empty case is checked before it, not by it | `api::audit_json::the_route_refuses_a_missing_feed_with_400_and_names_the_parameter` | ✓ |
| AJ-04 | No answer ever carries two feeds' numbers | `api::audit_json::the_answer_names_one_feed_and_never_a_second` | ✓ |
| AJ-05 | An absent journal and an absent manifest answer with `false`/`null` and a named path — never a `0` that a reader could mistake for a measurement | `api::audit_json::an_empty_store_answers_every_key_with_a_reason_and_never_a_zero_that_means_unknown` | ✓ |
| AJ-06 | Every field of a decoded record survives the round trip, and a note holding `&` or `"` arrives as text — escaped, never mangled | `api::audit_json::a_recorded_run_survives_the_round_trip_field_for_field` | ✓ |
| AJ-07 | A clock that cannot name today is **said** (`"today":null` with the refusal in `"clock"`) and the journal is still answered | `api::audit_json::a_refused_clock_says_so_and_still_answers_the_journal` | ✓ |

AJ-03 is not defensive coding. The default lives inside the parser, so every
caller that only checks for `None` has already silently chosen a vendor — which
is how `/store.json` answers `[]` with HTTP 200 for a feed nobody named, over a
store holding 139 million bars. The test was written first and it is what found
it.

**What is NOT invariant here, and is a limit rather than a bug.** The
`generation` and `committed_at` this route reports are the only evidence a run
is in flight, because nothing writes a record for a run that has not finished.
When a pull is between commits, "the store has not moved" and "nothing is
running" are indistinguishable from outside, and the console says the former
rather than the latter. See `docs/06-limits.md`.

## The front end on disk — `crates/api/src/assets.rs`

The binary serves the built front end itself, read from a directory at request
time. D-0064. Serving files off disk is the one thing in this crate that can
hand out a file nobody meant to publish, so every rule below is a property that
has to hold rather than a behaviour that happens to be right today.

Every row was verified by **deleting the line it names and re-running the
suite**: the tests listed went red, the line was restored, and the suite went
green again. That is weaker than a mutant survey and is not described as one.

| # | Must hold | Proven by | |
|---|---|---|---|
| W-01 | The path is percent-decoded **once**. `%252e%252e` is the literal name `%2e%2e`, never `..` | `api::assets::a_double_encoded_traversal_is_a_name_and_not_a_traversal` | ✓ |
| W-02 | `..` is refused in every spelling — bare, `%2e%2e`, `..%2f`, `..%5c`, mid-path, and under `_app/` — with the rule named in the body | `api::assets::a_parent_directory_is_refused_in_every_spelling` | ✓ |
| W-03 | A null byte in the decoded path is a refusal, not a truncation | `api::assets::a_null_byte_is_refused` | ✓ |
| W-04 | A malformed `%` escape is a refusal, never a byte taken literally | `api::assets::a_malformed_escape_is_refused_rather_than_taken_literally` | ✓ |
| W-05 | A decoded path that is not UTF-8 is a refusal | `api::assets::a_path_that_is_not_utf8_is_refused` | ✓ |
| W-06 | Every segment is exactly one `Component::Normal`; `\` is a separator here, not a file-name character | `api::assets::a_leading_separator_is_not_an_ordinary_name_and_a_backslash_is_a_separator` | ✓ |
| W-07 | A resolved path outside the canonical root is `403` — the symlink case — and a symlink that stays inside is still served | `api::assets::a_symlink_pointing_out_of_the_root_is_refused`, `api::assets::a_symlink_staying_inside_the_root_is_served` | ✓ |
| W-08 | An absent path with an extension, or under `_app/`, is a **404 and never HTML**; an absent path without one is the client-routed shell | `api::assets::a_real_asset_is_served_and_a_missing_one_is_a_404_not_the_shell`, `api::assets::a_client_routed_path_gets_the_shell_and_so_does_the_root` | ✓ |
| W-09 | `Content-Type` comes from the extension alone; an unknown extension is `application/octet-stream` and never a guess | `api::assets::every_extension_the_front_end_emits_has_a_type_and_the_rest_do_not_guess` | ✓ |
| W-10 | A missing asset directory answers **503 naming the directory, the override and the command**, and every JSON route still answers | `api::assets::a_missing_root_is_named_loudly_and_the_server_still_runs`, `api::assets::a_root_that_is_a_file_is_not_a_root` | ✓ |
| W-11 | An asset directory with no `index.html` says so rather than answering blank | `api::assets::a_build_with_no_shell_says_so_rather_than_answering_blank` | ✓ |
| W-12 | Only `GET` and `HEAD` reach the front end; anything else is `405` naming the method | `api::assets::only_get_and_head_reach_the_front_end` | ✓ |
| W-13 | Every registered route wins over a file of the same name on disk, and `POST /pull/spot` is still a `POST` route | `api::server::the_static_handler_is_last_and_never_shadows_a_route` | ✓ |
| W-14 | `/typeahead.js` is read from disk at request time, and its absence is a named 404 rather than a build failure | `api::assets::the_typeahead_is_read_from_disk_and_says_so_when_it_is_not_there` | ✓ |

**W-13 is the row that needed a decoy.** A routing-order assertion with nothing
at the shadowed path cannot fail, so `api::server::tests::front` writes a
`store.json` into the build directory that answers `"DECOY"`. Deleting the
`/store.json` route from the router makes the test serve it and go red, which is
what makes the row an assertion rather than a description.

**What is NOT claimed.** That `crates/api` has been mutation-tested. It has not;
`docs/06-limits.md` records that gap. The reversion check above is what was
actually done and is the weaker thing.

## The NSE series tables — layer 4, with the probe asserted as a number

Added by D-0065. `core::vendor::board_of` classified an NSE series code with
three `binary_search` calls, which `docs/07-o1-architecture.md` layer 4 forbids
without qualification — "no search of any kind ... **never `binary_search`**".
It probes three `MemberIndex` tables now, the same open-addressed structure
`crates/core/src/universe.rs` already built to retire a `binary_search` over
750 entries.

| # | Must hold | Proven by | |
|---|---|---|---|
| I-38 | Every series table answers in at most 8 probes, measured by walking it the way `contains` does, and every measured code is found while an absent one is found in none of the three | `core::vendor::the_series_tables_probe_in_bounded_time` | ✓ |

**The number in that row is the row.** Layer 4's "How a layer is proven" records
that the first open-addressed table written in this workspace measured **14**
probes — worse than the `binary_search` it replaced, and still O(1) by
definition — and that only its own test caught it. It happened again here: 120
codes in 256 slots is under half full, `MemberIndex::build` accepted it, and the
worst probe measured **10**. The table is 512 slots and the worst probe is
**6** because the assertion refused the first attempt. A row saying "the probe
is bounded" without a number would have shipped the 10.

**I-16 is not superseded and its test is unchanged.** Sortedness no longer makes
a search valid, because there is no search; it is still how a duplicate in a
hand-maintained list is caught, and that is what the test asserts. The row's
sentence still says "so `binary_search` cannot return garbage", and so does
`U-01`'s, whose test is *named* `..._so_binary_search_is_valid`. Both describe a
reason that has expired rather than a property that has. They are left alone
here on purpose — renaming a test moves a token CI gate 10 scrapes out of this
file, and doing that in the same change as the code it describes is how a green
gate stops meaning anything. Named here so the next reader finds it stated
rather than discovers it.

## The event sink — what one `emit` does not depend on

Added by the gate 12 sweep. `crates/telemetry/src/sink.rs` lists what one
`emit` costs and closes with "nothing in that list is a function of how many
events came before, how large the file is, or how many files there are". Of
those three, exactly one could have been a lie in the code rather than in the
comment, and it is the one that had no test.

| # | Must hold | Proven by | |
|---|---|---|---|
| T-01 | The rotation check reads the sink's **own** running byte count and never the file's size: the current file grown behind the sink's back to sixty-four times the bound does not cause a roll, and the running count still causes one afterwards | `telemetry::sink::the_roll_decision_reads_the_running_count_and_never_the_files_size` | ✓ |

**Why the second half of that row is there.** The first half alone is satisfied
by a sink that has stopped rotating at all, which is why the test goes on to
emit until the count rolls. Every other rotation test in that file drives the
count and the file size together, so a `metadata` call on the write path would
have passed all of them — `FileTarget::len`'s own comment says "at open time
only — never on the write path", and nothing held it.

**T-01 is still a count and not a timing**, and that is the right shape for it:
it asserts which NUMBER the roll decision reads, which no stopwatch can show.
What has changed since it was written is the sentence that used to follow it —
"`crates/telemetry` carries no bench … CI gate 14 refuses this crate for exactly
that reason". The bench exists now and the three rows below are it.

### The control surface and the writer's own losses

Two invariants added after a sweep of `crates/telemetry`'s exports for items
with **no caller outside the crate** — the shape `telemetry::tail` wore before
`/logs` existed. The sweep found nineteen; seventeen are limits and seams that
are correctly internal-facing. Two were features that had been built, tested,
mutation-checked, and never wired to anything.

| # | Must hold | Proven by | |
|---|---|---|---|
| T-02 | `BRUTEX_LOG_LEVEL` carries the per-subsystem syntax through to the sink: a bare word is the global floor, `target=level` raises one subtree, the longest matching prefix wins, and an unreadable clause is **named in the banner** rather than silently dropped | `api::server::the_log_level_env_parses_a_global_floor_and_per_subsystem_overrides` | ✓ |
| T-03 | What the WRITER lost reaches both `/logs` and `/logs.json`: a non-zero `dropped` or `rotation_failures` renders as a fault naming the count, a healthy sink states its zero explicitly, and no sink at all is `null` rather than a zeroed object | `api::logs::a_sink_that_lost_events_says_so_on_the_page_and_in_the_json` | ✓ |
| T-04 | An override the operator asked for and did not get is named: the ceiling note fires **only** when a clause was actually dropped, and it names the clause rather than counting it — exactly `MAX_TARGET_LEVELS` applied announces nothing | `api::server::the_override_ceiling_is_named_only_when_something_was_actually_dropped` | ✓ |
| T-05 | A per-target floor can be raised **above** the global floor to silence a subsystem, not only lowered to amplify one: events that pass `fast_floor` are still filtered by `level_for`, and the subsystem still speaks at or above its own floor | `telemetry::sink::a_target_floor_can_raise_one_subsystem_above_the_global_floor_and_silence_it` | ✓ |
| T-06 | The roll decision's two boundaries are `>` and not `>=`: a first event larger than the whole bound does **not** roll an empty file (which would spend a rotation and, at `keep_files = 2`, discard half the history to make room in a file that was already empty) | `telemetry::sink::a_first_event_larger_than_the_whole_bound_does_not_roll_an_empty_file` | ✓ |
| T-07 | An event that fills the file **exactly** to its bound does not roll — the bound is derived from a measured line, and the next event still rolls, so the test cannot be passed by a sink that has stopped rotating | `telemetry::sink::an_event_that_fills_the_file_exactly_to_its_bound_does_not_roll` | ✓ |
| T-08 | A roll that cannot complete is attempted **once** and never re-attempted: `roll` unlinks the oldest file and renames the rest up *before* the step that fails, so a re-attempt shifts the whole set again and `keep_files` further events empty the retained history one file per event | `telemetry::sink::a_roll_that_cannot_complete_does_not_empty_the_history_one_file_per_event` | ✓ |
| T-09 | A torn write is terminated with a newline, so a partial line cannot fuse with the NEXT event — the fragment stays visible as one `malformed` line and exactly one event is counted lost | `telemetry::sink::a_torn_write_is_terminated_so_the_next_event_is_not_fused_onto_it` | ✓ |
| T-10 | Only `NotFound` is success in a rotation: an unlink or a rename that refuses for any other reason is a **failed** roll, counted and loud — never swallowed into a successful one | `telemetry::sink::a_rotation_error_that_is_not_absence_is_reported_rather_than_swallowed` | ✓ |
| T-11 | Either half of `Health::is_loud` makes a sink loud on its own — a failed roll with nothing dropped is still loud, which every other test in the file masked | `telemetry::sink::each_half_of_is_loud_is_load_bearing_on_its_own` | ✓ |
| T-12 | A reopened sink resumes the byte count of the file it found, so the footprint bound survives a restart rather than allowing prior size **plus** the bound | `telemetry::sink::a_reopened_sink_resumes_the_byte_count_of_the_file_it_found` | ✓ |
| T-13 | `Query::since` is inclusive: the record sitting exactly on the boundary is returned, not treated as the end of the walk | `telemetry::tail::since_returns_the_record_that_sits_exactly_on_the_boundary` | ✓ |
| T-14 | The reader's two constants are pinned as **relations** — `READ_BLOCK` holds many events and is a fraction of a file; `DEFAULT_MAX_SCAN_BYTES` equals one file's bound and is less than the whole window — so a value change that breaks the design fails rather than moving the expectation with it | `telemetry::tail::the_readers_constants_hold_the_relations_they_were_chosen_for` | ✓ |
| T-15 | The pre-year-0 era yields a real DATE, not merely a negative year: `civil_from_days(-800_000)` is `(-221, 9, 4)`, and every conversion returns a month in 1..=12 and a day in 1..=31 | `telemetry::clock::the_negative_era_yields_a_real_date_and_not_merely_a_negative_year` | ✓ |
| T-16 | Year zero renders `0000`, not `-0000`; the year before it is signed | `telemetry::clock::year_zero_carries_no_sign` | ✓ |
| T-17 | A target past `MAX_TARGET_BYTES` is truncated **and the line says `cut`**, so a reader never shows a shortened subsystem name as though it were whole; a target that fits raises no flag | `telemetry::encode::a_target_past_its_ceiling_is_truncated_and_the_line_admits_it` | ✓ |
| T-18 | `Emitted::is_written` is true only for a write — false for filtered, dropped, and not-installed | `telemetry::sink::is_written_is_true_only_for_a_write` | ✓ |
| T-19 | A restart resumes the sequence after a line of the **widest legal shape**, so a sequence cannot restart at zero and repeat every number | `telemetry::sink::a_restart_resumes_the_sequence_after_a_line_of_the_widest_legal_shape` | ✓ |
| T-20 | A file that exists and cannot be stat-ed is a **named line in `Tail::errors`**, while an absent one stays silent — the sparse set is the ordinary state and must raise nothing | `telemetry::tail::a_file_that_exists_and_refuses_to_be_read_is_named_in_errors` | ✓ |
| T-21 | A log can say whether it is WHOLE: an unfiltered walk reports the exact number of events the sink numbered and the file does not hold, and a filtered walk reports `None` rather than mistaking a skipped record for a lost one | `telemetry::tail::a_hole_in_the_sequence_is_counted_and_a_filter_is_not_mistaken_for_one` | ✓ |
| T-22 | Every event carries the run it belongs to, so one file holding several interleaved backfills splits back into them; an event outside any run **omits** the key rather than writing a zero a reader must interpret, and a run filter reports `missing: None` rather than mistaking the other run's records for loss | `telemetry::tail::a_log_holding_several_runs_splits_back_into_them` | ✓ |
| T-23 | The reader is bounded in SPACE as well as time: a run of bytes wider than any line this crate can write is refused and COUNTED as malformed rather than accumulated, so a truncated, concatenated or foreign file cannot exhaust memory — and a whole line after the garbage is still returned | `telemetry::tail::a_run_of_bytes_wider_than_any_line_is_refused_rather_than_accumulated` | ✓ |
| T-24 | The run filter is reachable from a URL and from the form, and survives the 'as JSON' link — a filter nobody can ask for is not a feature | `api::logs::a_run_can_be_asked_for_by_url_and_survives_the_json_link` | ✓ |
| T-25 | **No event is returned twice.** The walk keys on the file the operating system knows, not on the path it answered to, so a roll that renames a file onto the next path while the walk is between two of them is met and refused — before its bytes are read, not after its records are decoded | `telemetry::tail::a_roll_landing_mid_walk_never_returns_the_same_event_twice` | ✓ |
| T-26 | And the refusal is not over-broad: a sequence that restarted at zero — which `resume_seq` documents and a wiped current file causes — puts the same NUMBER on distinct events, and every one of them is still returned. A repeat is a repeated FILE, never a repeated number | `telemetry::tail::a_restarted_sequence_is_not_mistaken_for_a_repeat` | ✓ |
| T-27 | The global floor and the fast floor are **one atomic word**, so two callers racing `set_min_level` cannot leave them describing different floors: the pair is published in a single store, and every word that store can publish is `(global, min(global, every override))` — checked at construction and at all five levels, with an override below the global floor and one above it | `telemetry::sink::the_two_floors_live_in_one_word_and_are_never_observed_crossed` | ✓ |

**Why T-02 is an invariant and not a feature note.** `Config::with_target_level`,
`Sink::level_for` and `MAX_TARGET_LEVELS` existed with tests and no surviving
mutants. Nothing parsed them out of the environment, so
`BRUTEX_LOG_LEVEL=info,pull=debug` did not select anything — it failed to parse
as a level word and fell back to `Info`. Every gate in §9 was green over a
feature no operator could reach. The invariant is the **wiring**, which is the
part no unit test in `crates/telemetry` can see.

**Why T-03 needs a hand-built `Health`.** `dropped` rises when a write fails,
`rotation_failures` when a rename does. Neither is reachable from a unit test,
so the loud branch would otherwise be a branch that only ever renders in
production — where it is least useful to discover it is wrong. `Health`'s fields
are `pub`, so the test states the sink's condition directly instead of trying to
cause it. The quiet branch asserts the zero is **printed**: an absent banner and
a banner this page forgot to render are indistinguishable to an operator.

**What T-03 closes.** `telemetry::tail` reads what reached the file and is
structurally incapable of reporting what never got there. A page built from the
tail alone shows `0 events` for two opposite worlds — nothing happened, and
everything was thrown away. `Health::dropped`'s own doc comment says *"this is
the number a page must show"*; no page showed it, and `Health::is_loud` had no
caller anywhere in the workspace. `CLAUDE.md` §4 bans a fallback that hides a
failure, and a sink that drops events behind a clean page is that fallback.

**Why T-27 is deterministic, and what it therefore does not prove.** The defect
was a race: `Sink::set_min_level` stored the global floor into one atomic and
then computed and stored the fast floor into another, so two callers interleaved
four stores and left the pair crossed — `min_level()` reporting `error` while a
`trace` event was still admitted, and the reverse. A barrier-synchronised probe
(two setters, one observer, reads taken only at the instant nothing was writing)
saw it at rounds 2023, 2597, 5275 and 6269 of four million on this machine.

That probe is **not** the committed test, because a test that fails at round
2,023 one run and 17,715 the next reports the scheduler rather than the code. The
committed test asserts the property that makes the race impossible instead of
sampling for the race: the pair has exactly one mutator, one store of one word,
so the only states any reader can observe are words `set_min_level` wrote, and
every one of those words is consistent. It reads the packed field as a single
load, so **splitting the pair back into two atomics does not make the test flaky
— it stops the test compiling.**

What it does not prove, stated: no committed test exercises a real concurrent
interleaving of the level-setting path, and there is no `loom` in this workspace
to enumerate one. `docs/06-limits.md` carries that limit.

### Complexity rows for the event stream

Enforced by gate 8, `crates/telemetry/benches/ratio.rs`, against the same **3.0×
shared-CI ceiling**. Two claims, two different sizes: the sink's is per-`emit`
against **events already written**, the tail's is the same twenty events against
a file a hundred times bigger.

| # | Must hold | Proven by | |
|---|---|---|---|
| C-T-01 | One `emit` costs the same with 1,000, 10,000 and 100,000 events already in the file — the level check, the clock, the lock, the render and the rotation check are none of them a function of what came before | `telemetry::bench::emit_cost_does_not_grow_with_the_file` | ✓ |
| C-T-01b | The same claim **at p99 rather than at the mean**: the 99th percentile of the per-call distribution is flat across the same three file sizes, under the same 3.0× ceiling. A mean cannot see a tail — an emit that occasionally took a thousand times longer would move it by a fraction of a percent and C-T-01 would stay green | `telemetry::bench::the_tail_is_flat_in_the_size_of_the_file_too` | ✓ |
| C-T-02 | A filtered event returns at the level check having touched nothing else: its cost is flat in the file too, and it is **measurably** cheaper than a written one on the same sink rather than merely documented as such | `telemetry::bench::a_filtered_event_touches_nothing_and_stays_flat` | ✓ |
| C-T-03 | Reading the last twenty events costs the same at 1,000, 10,000 and 100,000 events in the file, and reads the **same number of bytes** at every size — O(1) in the size of the file, which is the bound that matters because the file grows all day and N does not | `telemetry::bench::the_tail_is_flat_in_the_size_of_the_file` | ✓ |
| C-T-04 | One written `emit` costs a bounded multiple of the per-emit **floor** — a filtered event on the same sink, which enters `Sink::emit` and returns at the first level comparison having touched no clock, no lock and no buffer. **This is the row a ratio cannot replace**: every other row here divides one emit cost by another, so a UNIFORM slowdown cancels, and an audit measured a mask operation 174x slower passing its crate's ratio rows at 0.98x–1.00x. Measured over four runs: 173.5, 298.2, 219.3, 329.5 floors, budget **1,000**. **The 1.9x spread is the disk's** — the numerator reaches the filesystem and the denominator does not — so this row cannot resolve a regression under about 3x and does not claim to; a 174x uniform one would read ~57,000 floors and be refused by a factor of 57 | `telemetry::bench::emit_stays_within_its_budget` | ✓ |

**These three rows are ratios of MEANS, and prove flatness across file size —
not a worst-case bound.** Gate 8 takes `min` over 20 trials on sinks built with
rotation disabled, so a roll is neither timed nor timeable here. The worst case
is measured separately and reported in `docs/06-limits.md` §46: the emit that
rolls costs up to **4,859×** the median ordinary one, and p99 rises 161× from one
thread to eight. The bound in §3 rule 4 still holds — a roll is at most
`2 × keep_files` syscalls and no term grows with events logged — but the number
quoted for it was, until §46, the wrong number.

Measured on the operator's machine, 2026-08-09, `cargo bench -p telemetry`,
exit 0. C-T-01 measured **0.990×** and **0.968×**. C-T-02 measured **0.974×**
flat, and the filtered-against-written comparison came out at **0.002×** — a
filtered event is ~390× cheaper than a written one, which is the early return
doing exactly what `sink.rs` says it does. C-T-03 measured **1.011×** and
**1.035×**, and the byte count is the sharper half: **8,192 bytes read at 1,000
events and 8,192 at 100,000** — one `READ_BLOCK` either way, unchanged by a file
a hundred times larger. That equality is asserted in the bench, not just
printed. No figure from a CI runner is claimed, because none was taken.

## Every emit site in `crates/api` — proven, or named

`crates/api` holds **twenty-six** `telemetry::emit` calls: twenty-five in the
LIB target and one in `main.rs`, which is its own test binary. Before this
sweep, exactly the one in `main.rs` was proven — the other twenty-five were
*executed* by tests and asserted nothing, because `telemetry::emit` answers
`Emitted::NotInstalled` when no sink is installed and every site discards its
return under the name `_dropped_when_filtered`. A site could be reached by a
hundred tests, write nothing, and no assertion anywhere would move. That is
`CLAUDE.md` §4's "a test that asserts nothing" wearing a coverage report as a
disguise, and it is the same shape S-23 and P-39 record for `crates/store` and
`crates/pull`.

**Never a fabricated event.** The tests in `crates/api/src/logs.rs` that look
like they cover `pull.member` build their own `Event` with a production target
on a locally-opened `Sink`. That proves the sink works and says nothing about
the emit — every row below drives the shipped call instead.

| # | Must hold | Proven by | |
|---|---|---|---|
| A-38 | **All twenty-five of this crate's LIB emit sites reach a file**, each driven through the shipped call that owns it — `census::read_vendor`, `master::load`, `merge::merge`, `Catalog::build`, `Journal::{append, look, page}`, `bars::{open, page}`, `ingest::parse_spot`, `Assets::{new, respond}`, `Control::{pause, resume}`, the `/audit.json` handler, `broker_run` over an empty universe and over a universe of one, `note_member_failure`, `fetch_chunks` against a loopback vendor that refuses and one that answers, a served HTTP request through the `note_request` layer, and `run` itself — and each record is found by target, sentence and one field it could only have got from that drive, at a sequence number no earlier than the one the sink would have stamped before the call. The **level** is asserted on every row, because four sites choose theirs from a condition and a flipped ternary is invisible to every other kind of test | `api::emitted::every_reachable_emit_site_puts_a_record_in_the_file` · `api::server::the_run_events_and_the_request_event_reach_the_installed_sink` · `api::server::the_two_sites_past_the_socket_are_driven_over_a_real_one` · `api::server::the_refused_instrument_site_is_driven_over_a_real_universe` · `api::server::run_serves_until_the_signal_and_exits_zero` | ✓ |
| A-39 | **The three sites whose doc comments promise silence are silent.** `Assets::note_missing` fires on a doubling, so the 1st and 2nd miss write a line and the 3rd writes nothing; `bars::note_unreadable_records` writes nothing for a page where every record read; `audit::note_looked` writes nothing for an absent journal and nothing for a whole one. Without this, every one of those guards could be deleted and A-38 would still pass — and a scanner walking a wordlist would roll the run's own beginning out of the 64 MiB window, which is the shape D-0072 forbids | `api::emitted::the_silent_arms_stay_silent` | ✓ |
| A-40 | The list of sites that are **not** proven is asserted rather than described, and it is now **empty**: the sites proven in `emitted`, plus the sites proven in `server::tests`, plus the sites named as unreachable, are exactly the twenty-five in the LIB target, counted from the source. A site added with no row fails the count, and an unreachable list that grows again has to name what it is waiting for | `api::emitted::the_three_sites_this_binary_cannot_reach_are_named_rather_than_forgotten` | ✓ |
| A-45 | **A run that was never started reads as NOT running.** `/pull/run.json` renders a default document for an empty slot, and the page asks it on load — so if "running" were `finished.is_none()` alone, every fresh tab would believe a backfill was in flight, watch a run that did not exist, and never offer the Pull button again. The flag is therefore two facts, not one: STARTED, and not yet finished | `api::pullrun::a_run_that_never_started_still_answers_a_whole_document` · `api::pullrun::a_run_is_in_flight_exactly_while_it_has_no_summary` — **this row exists because its own test caught the bug**: the first version of `running()` returned `true` for `Progress::default()` and the assertion failed on the never-pressed document | ✓ |
| A-46 | **A leg that cannot be read refuses the WHOLE run, and names it.** Not "drop that one and run the rest": a run that silently omitted the leg it could not parse would be reported to the operator as a completed window over an instrument it never asked for. Both failure modes are covered — a leg short of its five parts, and a leg naming a route this build does not serve, which is refused rather than mapped onto spot | `api::pullrun::a_leg_that_cannot_be_read_refuses_the_whole_run` · `api::pullrun::an_unknown_route_is_refused_rather_than_sent_to_spot` — one bad leg beside three good ones refuses all four | ✓ |
| A-47 | **A form body reaches its route byte-for-byte through both encodings.** The wire carries one `leg` field per leg as `route\|vendor\|dir\|label\|body`, and every form body contains `&` while some contain `\|`, so the body is encoded a second time inside the field. The two passes must be exactly symmetric or `pull_spot` is handed a request the operator did not make — and the failure would be silent, because a corrupted window is still a legal window | `api::pullrun::a_body_carrying_every_separator_survives_both_encodings` — a body carrying `&`, `=`, `\|`, `+` and a literal `%` escape comes back identical. **What this does not bound:** the page's encoder is `encodeURIComponent` and the test's is a hand-written stand-in for it; they agree on these five characters and are not proven to agree on all | ◐ |
| A-48 | **Feeds run in parallel; legs inside one feed run in order.** The operator's rule, and it is what the rate budget dictates rather than a preference: a budget is per VENDOR, so two legs fired at one broker together spend one ceiling twice, while two brokers share no ceiling, no census lock and no manifest. The order within a chain is `pull::fold`'s ladder — spot before the derivatives that reference it, the day pass before the minute pass — and a rung nobody ranked sorts LAST, never first, which is the one position that breaks the ladder | `api::pullrun::legs_group_by_feed_in_tick_order_and_sort_onto_the_ladder` · `api::pullrun::an_unranked_rung_sorts_last_and_never_first` · `api::pullrun::two_legs_on_one_rung_keep_the_order_they_were_ticked_in` — the grouping keeps tick order, the sort is stable, and an unknown rung goes last. **What this does not bound:** that the spawned chains actually overlap in time. The fan-out is one `tokio::spawn` per group with every handle awaited after all are started, which is the shape; no test observes two vendors on the wire at once | ◐ |
| A-49 | **A dead credential is decided from what the VENDOR said, never from the page's own prose.** `accepted_html` fills its headline from `halt_for`, and both sentences it can return contain the word *credential* — so `classify(&html)` answered `Credential` for every page the route ever rendered, and every failing leg reported `HALTED · CREDENTIAL DEAD` whatever had gone wrong. Measured 2026-08-25: Dhan's HTTP 400 `DH-905 Input_Exception`, with the credential read successfully fourteen times in the same run, was reported as a dead token and the feed skipped for the rest of the run under §8 | `api::pullrun::the_headline_on_every_page_is_not_a_dead_token` — asserts BOTH that `classify` still answers `Credential` for `HTTP_LIVE`, so a later reword cannot be mistaken for this fix, and that `credential_fault_in_page` does not; drives `halt_for` itself so a third sentence is covered by the test rather than by having been thought about; replays Dhan's real DH-905 body inside the real headline; and checks the cure does not overshoot, because a genuine 403 `TokenException` must still halt | ✓ |
| A-50 | **A vendor outage stops the RUN, so the per-instrument 5xx ladder can be long enough to outlast a blip.** The ladder is spent per instrument (~785 in the equity universe), so a long one alone is a three-hour sleep and a short one is a 1.25 s budget that tests nothing — measured 2026-08-25, Kite's edge 503 ended a run in 4.5 s on a valid token and a legal window. Three CONSECUTIVE instruments on 5xx halts the run, so an outage costs three ladders not 785, and the ladder is `1000·n²` over 5 attempts (~30 s). The two constants are one decision | `api::server::three_consecutive_vendor_failures_stop_the_run_and_a_success_clears_it` — counts up only on the vendor's own side, resets on any success, and proves two-down-success-two-down is not an outage; `api::server::the_5xx_ladder_costs_thirty_seconds_because_the_breaker_bounds_it` asserts the sum beside the breaker that licenses it, so one cannot move without the other; `SERVER_ERROR_ATTEMPTS < THROTTLE_ATTEMPTS` refuses at compile time a budget the loop could never spend | ✓ |
| A-51 | **The breaker reads a control-character MARKER, never the refusal's prose.** `broker_window` answers `Result<_, String>`, so a run-level decision made by searching that string is the A-49 defect one commit later. `VENDOR_DOWN` is `\u{2}`, which no vendor sentence, instrument name or paragraph can forge, and it is stripped before the reason reaches an operator or the journal. The strip ORDER matters: the wire prefix is applied outside `laddered`, so they arrive `\u{1}\u{2}…` and testing for the second first finds nothing — a breaker that never trips is indistinguishable from a vendor that is never down | `api::server::three_consecutive_vendor_failures_stop_the_run_and_a_success_clears_it` asserts both markers are control characters, that they differ, and that a doubly-marked refusal strips to the vendor's own words in that order | ✓ |
| A-52 | **A stored mask becomes NAMES, and three different silences are never one.** The ledger stores the winning combination and the page used to say it could not be shown. Now: `has_mask === undefined` says the SERVER is older than the page (a missing field, not an absent mask — the page comes off disk and the route does not); `has_mask === false` says the LEDGER predates the field; an empty mask on a v3 ledger says the run genuinely found no combination; and bits set are named from `/vocab.json`. The middle two are byte-identical, so only `has_mask` separates them | `api::server::the_vocabulary_is_served_whole_so_a_mask_can_be_read_as_names` — every position present including tombstones, each carrying its own index, `live` proven discriminating in both directions; and verified on the operator's real ledger (version 2, `has_mask: false`) rendering "This ledger predates the condition mask… not a run that found no conditions" | ✓ |

### The three that were not proven, and what was actually true

For a long time three sites were listed as out of reach, all inside
`broker_run`'s per-instrument loop or past the socket it opens:
`pull.spot instrument refused`, `pull.http vendor refused a window`, and
`pull.chunk answered`. Each entry said reaching it needed a live vendor, and
each was wrong in its own way. **None of them needed one, and the recorded
transport for `pull::fetch` they were all said to be waiting on was never
written.**

* `pull.http vendor refused a window` and `pull.chunk answered` needed a
  **socket**, not a vendor. A `TcpListener` on `127.0.0.1:0` supplies one, which
  is how `crates/pull`'s own socket tests already worked. The refusal is driven
  against a 503 and the success against a 200 whose body is the shape the
  shipped Dhan descriptor declares — parallel arrays, no envelope, rupee prices,
  no `open_interest` array — so the decode under test is the production decode.
* `pull.spot instrument refused` needed a **non-empty universe** and nothing
  else. The loop emits for whatever `broker_window` refuses, and that function
  refuses a rung the feed does not declare on its fourth statement, above the
  credential file and above `AwsIdentity::discover`. Dhan declares `Day1` alone,
  a form with no `granularity` field means `Minute1`, and `served` refuses by
  name.

No test here authenticates against a broker — the failure `Site::broker`'s own
doc comment records, and which a previous version of this suite committed. The
standing lesson is the one D-0072's neighbours keep re-learning: **an
"unreachable" list is worth re-reading rather than trusting.**

### One sink, because `install` is a process singleton

`telemetry::install` refuses a second call, so a test binary has exactly one
installed sink and every assertion about a production emit is an assertion about
*that* sink. `crates/api/src/emitted.rs` owns it — one directory, one `Trace`
floor — and `logs.rs`'s handler test and `server.rs`'s `run` test both take it
from there rather than installing their own. The floor is load-bearing:
`api.request served` is `Debug` for a request that is neither a 4xx nor a 5xx,
so at the default `Info` floor that site writes nothing on the ordinary path and
a test asserting against it would be asserting the floor. `emitted::sink`
asserts that the sink it hands back is the one it configured, so a future third
installer fails by name instead of silently changing the directory and the floor
for every other test — and so that nothing this suite writes can land in the
operator's own `~/.brutex/store/logs`.

The reads are keyed on the sink's **sequence number**, not on
`Query::since`. `Sink::emit` reads the clock before it takes the lock, so under
a parallel test binary two events can be ordered one way by their timestamps and
the other way in the file — and `since` does not merely filter, it *ends* the
walk at the first record older than it. One older-stamped line written by a
concurrent test was enough to stop the walk before the record under assertion;
measured, it failed about one run in three. `seq` is assigned inside the lock
and is unbroken across a rotation, so it is the order the file is actually in.


---

## The condition mask — the sweep's inner loop, measured

`ConditionMask::hits` is called once per candidate per bar. At 1.2 million bars
and a frontier of any interesting size it runs more often than every other
operation in this workspace combined, which is why `CLAUDE.md` §3 rule 4 names
mask evaluation among the five that must be constant.

Two things prove that jointly, and neither is sufficient alone. The **source**
guarantee is `vocab::mask::hits_does_the_same_work_for_every_input`: it reads the
function body, refuses `for`, `while`, `loop`, `return` and `if`, and counts the
operators against `WORDS` — so a word added to the mask and not to the body fails
the build rather than being silently ignored. The **cost** guarantee is the bench
below: a source with no branch to take still has to be shown not to take one.

The third row is the one §6 rests on. The Apriori ladder has no depth parameter
and walks upward until the frontier empties, which is affordable only because
evaluating a wide combination costs what a narrow one costs. If `hits` were
per-bit, the ladder's total work would grow quadratically in depth and the
missing parameter would be a performance defect rather than a design decision.

| # | Must hold | Proven by | |
|---|---|---|---|
| C-V-01 | A hit and a miss cost the same, so the sweep's per-bar cost does not depend on how selective the frontier is | `vocab::bench::a_hit_and_a_miss_cost_the_same` | ✓ |
| C-V-02 | A miss costs the same **in every word**, checked for all six — this is the measurement that catches an early-exit loop, which returns sooner on a candidate failing in word 0 and would make cost depend on which conditions a combination happens to require | `vocab::bench::a_miss_costs_the_same_in_every_word` | ✓ |
| C-V-03 | The cost does not grow with what the candidate requires: a 1-bit candidate against all 234 live bits. §6's absent depth parameter depends on this row | `vocab::bench::the_cost_does_not_grow_with_what_the_candidate_requires` | ✓ |
| C-V-04 | `popcount`, `union` and `intersect` do not grow with the bits set — `popcount` especially, because a shift-and-count implementation costs more when more are set and the frontier's reconciliation calls it on every mask | `vocab::bench::the_set_operations_do_not_grow_with_the_bits_set` | ✓ |
| C-V-05 | **Every mask operation costs a bounded multiple of the floor — the measurement C-V-01…C-V-04 structurally CANNOT take.** Those four compare an operation to ITSELF on a different input, so a uniform slowdown divides out and a 174× regression across the board would pass every one of them. This compares each operation to something that cannot slow down with it. Budget 12 floors, chosen with headroom for a different microarchitecture rather than picked to pass. Measured 2026-08-26: `get` 1.481, `hits` 1.596, `popcount` 1.878, `intersect` 2.188, `union` 2.208, `with_bit` 3.373 — widest is 3.5× inside budget. **Ran and passed for as long as the bench has existed and was declared nowhere until D-0298** | `vocab::bench::no_operation_costs_more_than_its_budget` | ✓ |
| C-V-06 | **Condition lookup — the SECOND operation `CLAUDE.md` §3 rule 4 names — costs the same wherever in the table it lands.** `table::definition` is `TABLE.get(index)` and `table::is_live` wraps it; `engine::Ladder::walk` calls `is_live` once per offered position, and **no bench measured either** until an O(1) audit of all thirteen crates found the gap. The rows above measure the MASK, six words of register arithmetic — a different operation on different data. The index must VARY because a direct index costs the same everywhere and **a scan does not**: position 0, position 279, the midpoint, and one past the table, since a linear search would be flat at 0, linear at 279, and would walk the whole table before answering the miss. Measured 1.000×–1.062× on `is_live`, up to 1.111× on `definition` | `vocab::bench::a_condition_lookup_costs_the_same_wherever_it_lands` | ✓ |

Measured on the operator's machine, 2026-08-11, `cargo bench -p vocab`, exit 0.
**Seven measurement sites producing eleven points** — the per-word loop is one
site and emits one point per interior word, so it covers a seventh word
automatically if `WORDS` ever grows rather than needing a line added beside it.
Gate 14's floor is stated as 7 because that is what its own counter sees.
`hits` costs **~0.82 ns** and every ratio landed between
**0.961× and 1.044×** against a ceiling of 3.0×:

| Point | Ratio |
|---|---|
| C-V-01 hit → miss | 0.998× |
| C-V-02 miss in word 0 → word 5 | 0.996× |
| C-V-02 miss in word 0 → words 1, 2, 3, 4 | 0.961×, 0.961×, 0.996×, 0.997× |
| C-V-03 1-bit candidate → 234-bit candidate | 1.037× |
| C-V-04 popcount, 1 bit → 234 bits | 0.965× |
| C-V-04 union / intersect, empty → full | 1.044× / 0.973× |

**What the numbers say and do not say.** 0.996× between a word-0 miss and a
word-5 miss is the early-exit measurement, and it is flat — six words are read on
every call whatever the answer. 1.037× between a 1-bit and a 234-bit candidate is
the depth measurement, and it is flat for the same reason: the work is per WORD,
not per BIT, so requiring 234 conditions costs what requiring one costs.

What they do not say is that the compiler cannot introduce a branch. A ratio near
1.0 is evidence and the source check is the guarantee; the honest claim is the
conjunction, which is why both are listed and neither is described as sufficient.

A zero baseline is reported as `UNMEASURABLE` and **fails**, rather than dividing
by zero or passing. An operation the optimiser deleted has not been shown to be
constant — it has been shown to be absent, and calling that a pass is the
fallback §4 bans.


---

## The indicator fold — one candle in, and what it does not depend on

Every module in `crates/indicators` is a streaming fold, `step(&mut self, bar:
&Candle)`. `const _: () = assert!(size_of::<Evaluator>() <= 1792)` fails the build
if the state grows, and that bounds the **space**. It says nothing about time, and
the two come apart in one way that matters: a fold whose state is fixed-size can
still do more work as it goes if it ever rescans that state.

| # | Must hold | Proven by | |
|---|---|---|---|
| C-I-01 | One candle costs the same however many came before it — measured at 1,000, 50,000 and 200,000 candles folded, so drift appearing only at an intermediate depth is visible rather than hidden between two endpoints | `indicators::bench::a_candle_costs_the_same_however_many_came_before` | ✓ |
| C-I-02 | One candle costs the same whatever it CONTAINS: a motionless bar, a wandering one and a decisive trend bar. Checked **in both directions** — a bar that costs half as much is as much a data-dependent cost as one that costs twice as much, and the one-sided check every other bench uses would report it green | `indicators::bench::a_candle_costs_the_same_whatever_it_contains` | ✓ |
| C-I-03 | `isqrt_i128`'s iteration count stays under `ITERATION_CEILING` = 130 at 1, `i64::MAX`, `10^30` and `i128::MAX`, and the root satisfies `root² ≤ v < (root+1)²` at every one of them — a function that gave up early would be fast and useless | `indicators::bench::the_integer_square_root_is_bounded_and_flat_per_iteration` | ✓ |
| C-I-04 | A session rollover costs what an ordinary mid-session candle costs. Rollover is the one place per-candle work could legitimately spike: it copies the running extremes into the previous-session slots and reseeds. That is a fixed number of field moves, and this row is what says it is not a rebuild from history | `indicators::bench::a_session_rollover_costs_what_an_ordinary_candle_costs` | ✓ |
| C-I-05 | **One bar's whole evaluation costs a bounded multiple of the floor.** Budget 2,000 floors. Measured **twice on 2026-08-26, an hour apart, at 397.533 and 963.263** — a 2.4× spread on the same binary and the same machine, because the FLOOR is re-measured per run and moves with machine state. Both are inside budget; a single figure quoted from one run would read as precision this row does not have. That spread is the reason the budget is 2,000 and not 500. The bench's own doc named this row's absence as a defect — *"JUDGED HERE, PINNED NOWHERE ELSE"* — because its verdict feeds gate 8's exit status while `docs/04-invariants.md` carried `C-I-01`…`C-I-04` and no `C-I-05`, and it reports outside the helper gate 14 counts. **Deleting the assertion would have deleted the measurement with every static check still green.** Declared by D-0298 | `indicators::bench::a_candle_stays_within_its_budget` | ✓ |
| C-I-06 | **Building the whole column costs the same per bar however long it is** — 20,000 bars against 200,000, measured 2026-08-26 at **1.058×**. `C-I-01` measures one `step` against a deeper evaluator, which is the fold's own bound; this measures the vector backing the column, the per-bar warm-up test and the census. A `Vec` reallocating per push, a census scanning its own buckets, or a warm-up test walking history would all be invisible to `C-I-01` and are visible here. Both legs sit past the warm-up boundary on purpose. Declared by D-0298 | `indicators::bench::the_column_costs_the_same_per_bar_however_long_it_is` | ✓ |

Measured on the operator's machine, 2026-08-11, `cargo bench -p indicators`,
exit 0. The evaluator is **1,664 bytes** and emits **234 positions**.

| Point | Ratio |
|---|---|
| C-I-01 1,000 → 50,000 candles in | 1.015× |
| C-I-01 1,000 → 200,000 candles in | 1.009× |
| C-I-02 wandering → motionless | 0.535× (1.87× inverted) |
| C-I-02 wandering → decisive trend | 0.947× |
| C-I-04 mid-session → session rollover | 0.772× |

**C-I-01 is the row this crate exists to earn.** 1.009× at 200× the depth is the
fold not accumulating — the claim the `size_of` assertion cannot make.

**C-I-02 does not read as flat and is not reported as flat.** A motionless candle
costs 0.535× a wandering one, because the range-relative predicates refuse on a
zero range and never run their arithmetic. That is under the ceiling in both
directions and it is a **1.87× content dependence**, recorded here as measured
rather than described as constant. `docs/06-limits.md` carries it.

**C-I-03 asserts a bound, not a flat cost, and the difference is the whole row.**
An earlier version of this bench timed `isqrt_i128(1)` against
`isqrt_i128(i128::MAX)` and breached at **217×**. The breach was correct and the
measurement was wrong: the Newton loop exits on convergence, so the function never
promised a flat cost. Iterations measured **1 → 37 → 55 → 69** against a ceiling
of 130. The cost spread is printed as context and deliberately **not** compared to
a ceiling — see `docs/06-limits.md` for why, and for the numbers.


---

## The sweep — what a per-bar cost is allowed to depend on

Three costs in this crate are **not** constant and none of them is a defect.
Support counting is O(bars) because it *is* the measurement. The Apriori level
join is O(|frontier|²), which is that algorithm's shape. Building a candidate
walks the full 384-bit width because `ConditionMask` exposes no bit iterator, and
384 is a compile-time constant.

What must not happen is a per-bar **constant that drifts upward**. That is how a
linear pass becomes superlinear with nothing in the source resembling a nested
loop, and it is what these rows measure. Isolated primitive rows compare the
quotient in **both directions** — a variant that is cheaper is as much a data
dependence as one that is dearer. C-E-04 is the deliberate exception: a complete
ladder is `a·bars + b`, where its fixed join, sort and reconciliation work is
amortised over the larger column. It therefore rejects only a greater per-bar
cost, after proving both legs completed the same non-empty topology and produced
the same masks and accounting. D-0429.

| # | Must hold | Proven by | |
|---|---|---|---|
| C-E-01 | The per-bar cost of support counting does not grow with the column: 10,000 against 100,000 against 1,000,000 bars | `engine::bench::support_costs_the_same_per_bar_at_every_column_length` | ✓ |
| C-E-02 | The per-bar cost of the **live** `Column::support` does not grow with what the combination requires: k=1 against k=4 against k=8. The owned column folds one branchless fixed-six-word `hits` per row whatever `k` is. Measured k=1 → k=4 at 0.971× and k=1 → k=8 at 0.996× | `engine::bench::support_costs_the_same_per_bar_at_every_depth` | ✓ |
| C-E-02b | **Historical defect retained, corrective state proved.** From 7461f57 until this repair the live sweep used a vertical position-bitmap layout whose per-bar work was Θ(k), measured **7.307×** from k=1 to k=8. That contradicted C-02 while a flat row-major helper with no live caller kept passing. `Column` now owns one fixed-six-word mask per bar; the live body performs exactly one `row.hits(candidate)` and contains no position iterator or popcount-dependent branch. The independent differential test checks more than 1,000 live/reference results across 1/63/64/65/127/128/300 bars and k=0..4, so restoring the vertical algorithm cannot hide behind result equivalence | `engine::column::tests::the_live_support_body_is_one_fixed_width_hit_test`; `engine::column::tests::the_fixed_width_column_agrees_with_the_vertical_reference`; C-E-02 and C-E-09 for measured flatness | ✓ |
| C-E-03 | The per-bar cost does not depend on the ANSWER: a live `Column` where every bar matches against one where none does, verified to be those two extremes before being timed. `Column::support` folds a branchless fixed-six-word `hits` with no early exit; measured all → none at 0.710× | `engine::bench::support_costs_the_same_whether_bars_match_or_not` | ✓ |
| C-E-04 | A whole ladder walk — generate, subset-prune, reject repeated k=1 positions, count support — does not get dearer per bar at 100,000 bars than at 10,000, with the live set held fixed so only the bar count varies and the O(&#124;frontier&#124;²) join is not what is being measured. The injective prefix join has no k≥2 duplicate-rejection pass. Before timing, both legs must complete, walk a non-empty identical level/k topology, reconcile every level and return the same ordered masks and accounting. The one-sided ceiling is intentional because the fixed join/sort/reconciliation term is amortised over more bars; isolated primitives retain symmetric checks. D-0429 | `engine::bench::a_ladder_walk_does_not_get_dearer_per_bar` | ✓ |
| C-E-05 | One supported bar costs a bounded multiple of the per-bar **floor** — the cheapest possible walk over the same slice, one `wrapping_add` per bar. A ratio divides two costs of the same operation and so cancels a uniform slowdown: an audit found a mask operation 174× slower passing every ratio row at 0.98×–1.00×. This row is the one that would not have | `engine::bench::support_stays_within_its_budget` | ✓ |
| C-E-06 | The owned live column stays in the same cost class as the fixed-width slice reference. Both perform one six-word hit test per bar and are required to return the same support before timing. Fixed-width reference → live measured 0.739× at k=1 and 0.671× at k=8; the wrapper exists for reuse and adds no candidate-dependent pass | `engine::bench::live_column_costs_like_the_fixed_width_reference` | ✓ |
| C-E-07 | The per-bar cost of the **fingerprinted** support path does not depend on the ANSWER: every bar matching against none matching, both extremes verified before timing. `support_fingerprinted` performs the same fixed-width hit test while packing consecutive 64-bar hit words and folding their identity; it measured 1.009× | `engine::bench::fingerprinted_support_costs_the_same_whether_bars_match_or_not` | ✓ |
| C-E-08 | One pair of the level join costs the same whatever the frontier holds: a 100-wide frontier against a 1,000-wide. This is the row `DEFAULT_PAIR_BUDGET` was cited against before it existed — the rows ran 01–07 and 09, and the budget's own justification named a measurement nobody had taken | `engine::bench::one_join_pair_costs_the_same_at_every_frontier_width` | ✓ |
| C-E-09 | The live `Column::support` path is flat across the mask's **entire representation**, k=1 against k=384. Both endpoints still perform one fixed-six-word `hits` per bar; measured 0.942×. This closes the possibility that an implementation keyed above the ordinary k=8 fixture passes C-E-02 | `engine::bench::live_support_is_flat_across_the_entire_mask_width` | ✓ |
| C-E-10 | **Duplicate rejection — the fourth operation rule 4 names — is benchmarked only where production performs it.** k=1 pre-sizes an offered-position `HashSet<u32>` and rejects a repeat when `insert` returns false; the corrected row varies 1,000 / 10,000 / 100,000 already accepted positions. k≥2 has no candidate-dedup table because the prefix join is injective. Corrected-row ratios have not yet been taken; the superseded mask hit/miss numbers are not carried forward. Even a green rerun is expected/amortised evidence, not an adversarial worst-case hash-table bound | `engine::bench::duplicate_rejection_costs_the_same_however_much_is_seen`; `engine::tests::one_k_set_is_evaluated_once_however_many_pairs_produce_it` | — |
| C-E-11 | **Result append — the fifth operation rule 4 names — is isolated on production's already-reserved `Vec<Itemset>` path.** k=1 reserves the offered width and later levels reserve the previous-frontier heuristic capped by the candidate ceiling. The vector is allocated before timing and cleared between trials, so the row measures pushes that fit that reservation. Corrected-row ratios have not yet been taken. A push after an expanding level exhausts the heuristic may reallocate and move all held items; only the amortised O(1) `Vec` bound applies there | `engine::bench::result_append_costs_the_same_however_many_are_held` | — |

### `crates/runner` — cost rows

The crate shipped three cost claims and no bench. Its `lib.rs` header admitted
so in words, which satisfies gate 12 and does not satisfy gate 14: admitting a
number is missing is not the same as taking it. An adversarial audit ran gate
14"s own script and found the crate refused. These are the measurements.

Two of the three rows below were REWRITTEN by their own first result. C-R-01
began as `a run costs the same per bar` and read 0.002x -- the claim being
false, not a regression, because a run contains an O(candidates) ladder that has
no business being divided by a bar count. C-R-02 began as a per-level cost and
read 0.197x, which is the fixed prologue amortising, not anything about bars.
Neither ceiling was widened.

| # | Must hold | Proven by | |
|---|---|---|---|
| C-R-01 | The COLUMN BUILD costs the same per offered bar at every column length: 3,000 against 12,000 bars, with the ladder neutered to `min_hits = u64::MAX` so only the per-bar pass is timed. Divided by OFFERED bars because `Column::build` steps every one of them -- a swept-bar divisor charges the fixed 1,876-bar warm-up to 1,124 bars in one column and 10,124 in the other, and read 0.348x for exactly that reason | `runner::bench::the_column_build_costs_the_same_per_bar_at_every_column_length` | ✓ |
| C-R-02 | The audit costs NOTHING per bar. Both ladders are neutered to a single level so the level count is held equal and only the column varies -- 1,124 against 10,124 swept bars. `report.rs` is allowed to exist outside gate 17's silence precisely because it costs one string per run rather than anything per bar, and a render whose cost tracked the column would make that sentence false | `runner::bench::the_report_costs_nothing_per_bar` | ✓ |
| C-R-03 | Assembling a run identity does not depend on how many bars the digest covered: by the time `identity` runs, the data digest is thirty-two bytes whatever produced it. `data_digest` itself is O(bars) and honestly so; it is computed outside the timed region. Measured at 20,000 repetitions because at five the row read 0.369x, which was the scheduler | `runner::bench::the_identity_does_not_depend_on_how_many_bars_the_digest_covered` | ✓ |
| C-R-04 | One bar's column build costs a bounded multiple of the per-bar **floor** — the same slice walked with one `wrapping_add` per bar, same loop, same bounds checks, same memory traffic, none of the indicator work. **This is the row C-R-01 cannot replace**: it divides one per-bar cost by another, so a UNIFORM slowdown cancels, and an audit measured a mask operation 174× slower passing its crate's ratio rows at 0.98×–1.00×. Measured 1270.202 / 990.494 / 968.863 floors; the 1.31× spread is the workspace's widest and is expected, since the numerator runs ten indicator families against a denominator of one add. **The magnitude is the point**: a thousand of the cheapest per-bar operations is what turning one bar into 280 condition bits costs, and knowing that is the only way to notice it becoming two thousand. Budget **5,000** | `runner::bench::the_column_build_stays_within_its_budget` | ✓ |

Measured on the operator's machine, 2026-08-30,
`cargo bench -p engine --bench ratio`, exit 0. These are regression observations
from one machine and run, not service-level guarantees. `size_of::<Ladder>()` is
**32 bytes** — its four controls are `min_hits`, `ceiling`, `pair_budget`, and
the scheduling-only `support_lanes`; there is no depth field. The complete benchmark prints every
declared C-E row, including the isolated live-layout, join, duplicate, and append
measurements that the earlier six-point table omitted.

| Point | Ratio |
|---|---|
| Per-bar floor | 349 ps |
| C-E-05 live support / per-bar floor, k=1 | 1.498× |
| C-E-05 live support / per-bar floor, k=8 | 1.819× |
| C-E-06 fixed-width reference → live column, k=1 / k=8 | 0.739× / 0.671× |
| C-E-01 10,000 → 100,000 bars | 1.271× |
| C-E-01 10,000 → 1,000,000 bars | 0.663× (cheaper) |
| C-E-02 live k=1 → k=4 | 0.971× (cheaper) |
| C-E-02 live k=1 → k=8 | 0.996× (cheaper) |
| C-E-03 live all match → none match | 0.710× (cheaper) |
| C-E-07 fingerprinted all match → none match | 1.009× |
| C-E-08 one join pair, 100 → 1,000 frontier | 1.074× |
| C-E-10 k=1 duplicate insert, 1,000 → 10,000 / 100,000 offered | **pending rerun after production-shape correction** |
| C-E-11 reserved `Itemset` append, 1,000 → 10,000 / 100,000 held | **pending rerun after production-shape correction** |
| C-E-09 live `Column::support`, k=1 → k=384 | 0.942× (cheaper) |
| C-E-04 walk, 10,000 → 100,000 bars | 0.337× (fixed work amortised) |

**C-E-02 and C-E-09 both measure the production `Column::support` method.** The
owned row-major implementation performs one fixed-six-word hit test per bar from
k=1 through the complete k=384 representation. The ladder can therefore walk to
extinction without turning candidate depth into a caller-controlled cost cap; this
is not a claim that total sweep work is constant.

**C-E-03 at 0.710× is the live no-early-exit measurement**, and the two columns are
verified to be the extremes — 100,000 of 100,000 matches against 0 of 100,000 —
before either is timed. A column that quietly failed to be an extreme would print
a reassuring number and mean nothing.

**C-E-01's 1.271× at 100,000 bars remained inside the symmetric ceiling, and the
million-bar leg measured 0.663×.** The
ratio is empirical noise around a per-bar constant, not a proof of worst-case
latency on other hardware.

**C-E-04 walked to depth 8 and found 255 frequent sets** at `min_hits = 1` over
eight live positions — every non-empty subset of the eight, which is the complete
lattice. That is the extinction condition being reached rather than a cap: the
ladder stopped because k=9 has no candidates, not because anything told it to.

The 1,000,000-bar column is deliberately absent from C-E-04: a full walk over a
million bars at eight live positions takes minutes and gate 8 runs on every push.
The 10× step is enough to see a drifting constant. Stated rather than implied, per
§3 rule 6.


---

## The indicator fold's four corrected predicates — 2026-08-11

Four positions reported measurements they were not making. Each row below names the test
that catches it, and every one of those tests was run against the pre-fix code and seen to
fail there — a guard nobody has watched fail is not known to be a guard.

| # | Must hold | Proven by | |
|---|---|---|---|
| IF-01 | A flat bar (`close == open`) is **neither** direction, so it sets neither 37 `prior_n_bullish` nor 38 `prior_n_bearish` and it breaks 39's alternation. The ring stored `close > open`, which filed a flat bar as `false` = bearish, so 38 asserted three bearish bars over a run in which 31 `bar_bearish` was never once set | `indicators::session::a_run_of_flat_bars_is_neither_a_bullish_nor_a_bearish_run`, `indicators::session::a_flat_bar_between_two_up_bars_is_not_an_alternation` | ✓ |
| IF-02 | 39 `prior_alternating` stays **clear** on a monotone run. Nothing pinned this: replacing the `differing` test with `true` passed all 22 session tests, so 39 could fire on three consecutive up bars — the same defect as I-01 in the opposite direction | `indicators::session::a_run_of_three_up_bars_sets_prior_n_bullish_on_the_fourth`, `indicators::session::a_run_of_three_down_bars_sets_prior_n_bearish_on_the_fourth` | ✓ |
| IF-03 | A period speaks only on the candle its own period completes. Bit 2 `close_above_ema200` fired on the **second** candle of a run, where the "200-period average" was one candle old — `docs/03-vocabulary.md` §4 says a bit that cannot be evaluated evaluates false, never "probably". The EMISSION is gated on a folded counter, not the accessors, because `Atr::value()` returning `None` stops `SuperTrend::fold` seeding at all | `indicators::trend::the_second_candle_of_a_run_names_no_period_at_all`, `indicators::trend::each_position_speaks_on_the_candle_its_own_period_completes` | ✓ |
| IF-04 | A pivot ladder whose rung leaves `i64` **refuses**, and never clamps. Clamping collapsed R4 onto R5 — two vocabulary positions becoming one predicate — and set six positions measured against levels that do not exist | `indicators::daily::a_rung_past_the_type_refuses_the_whole_ladder` | ✓ |
| IF-05 | A session whose intermediate `high + low + close` needs `i128` still yields **ten distinct rungs**. The i128 widening had no guard at all after I-04's fix deleted the test that pinned the near-edge arithmetic; dropping the widening now makes the pivot come back as −3,074,457,345,618,258,605 | `indicators::daily::a_session_whose_sum_needs_the_widening_still_yields_distinct_rungs` | ✓ |
| IF-06 | A band base that leaves `i64` **abstains by name** rather than saturating, at all **four** sites. A saturated span made the band up to 2× too narrow, and a close inside the missing half set **no bit while nothing refused** | `indicators::orb::a_window_whose_span_leaves_the_type_sets_nothing`, `indicators::fib::a_five_session_span_that_leaves_the_type_sets_nothing`, `indicators::gap::a_leg_whose_length_leaves_the_type_sets_nothing`, `indicators::trend::a_swing_window_whose_span_leaves_the_type_publishes_nothing` | ✓ |
| IF-07 | A refused bar changes **nothing**. `Evaluator::step` used to raise VWAP's accumulator refusal after the session rollover and seven module folds, so the next bar's bits were computed from a bar the caller was told was refused — four invented bits, and no agreement with a clean evaluator over 400 following bars | `indicators::evaluator::an_overflowing_accumulator_reaches_the_caller_as_a_refusal`, `indicators::evaluator::a_refused_bar_does_not_complete_a_session` | ✓ |
| IF-08 | `TrendState::bits` is a function of the bar. It took `&mut self` and advanced the market-structure latch from inside the emit, so the same close answered `choch_bearish` then `bos_bearish`. `bits(&self)` makes the old shape a **compile error** | `indicators::trend::bits_is_idempotent_across_a_break_and_a_change_of_character`, `indicators::trend::the_structure_latch_advances_exactly_once_per_candle` | ✓ |
| IF-09 | A change of character **reaches the mask**. I-08's two tests both pass if `step` stops advancing the latch altogether, which makes positions 58 and 59 unreachable and reports every reversal as a continuation. This is the only test in `trend.rs` that fails when the advance is removed | `indicators::trend::a_change_of_character_reaches_the_mask` | ✓ |

| IF-10 | A non-regular session's OHLC **never** becomes the previous-day anchor. `docs/00-charter.md` §3 forbids it by name and claimed an "anchor walk restricted to trading days" that did not exist — a grep for holiday/muhurat/trading_calendar over the three crates returned zero hits. Measured before the fix: all 375 bars of the session after a 60-bar day emitted different pivot positions | `indicators::evaluator::a_non_regular_session_never_becomes_the_previous_day_anchor` | ✓ |
| IF-11 | A non-regular session does not advance the five-session rolling window. The second of the charter's three prohibitions: `Prev5` is what positions 110–120 are measured against, and a one-hour OHLC entering it contaminates them for five more sessions | `indicators::evaluator::a_non_regular_session_does_not_advance_the_rolling_window` | ✓ |
| IF-12 | A non-regular session **still emits its own bars**. The charter forbids that hour becoming an anchor, not that it be silenced — it is real trading, and every intraday position on it is a genuine measurement | `indicators::evaluator::a_non_regular_session_still_emits_its_own_bars` | ✓ |

| IF-13 | `Evaluator::warmed_up` is **monotone** and `every_family_can_answer` is not, because the opening ranges re-open every session. Collapsing the two — which the first version did — produces a signal that goes false at every session boundary and cannot be used to choose where a sweep starts | `indicators::evaluator::only_the_run_level_signal_is_monotone` | ✓ |
| IF-14 | `warmed_up` is a **conjunction**, so the slowest family decides. Five 30-bar sessions fills `Prev5` while the 200-period EMA has seen 150 candles, and the signal must be false. Reducing it to the session count alone left every test green until this row asserted the run-level signal directly instead of the per-bar one | `indicators::evaluator::the_slowest_family_decides_and_it_is_not_always_prev5` | ✓ |
| IF-15 | The signals report the boundary the five-session ladder imposes: false at four completed sessions, true at five with the longest window closed | `indicators::evaluator::the_evaluator_reports_when_every_family_can_finally_answer` | ✓ |

| IF-16 | 276/277 compare the close to the **session's** open, and a flat close sets **neither** — nor does the session's first bar, which has no session open to compare against yet | `indicators::evaluator::the_close_against_the_session_open_sets_at_most_one_bit` | ✓ |
| IF-17 | 278/279 report the structure **in force between** breaks, not only on the bars where 56–59 fire. Without that distinction they are a duplicate of the break events and every other test still passes | `indicators::evaluator::the_structure_in_force_is_reported_between_breaks_and_not_before_the_first` | ✓ |

| IF-18 | The six non-regular days ARE the charter's six, derived by an arithmetic sharing no code with the constant. A sweep shifted 2023-11-12 by one day in **both** spellings — so the const assertion still passed — and nothing caught it. An off-by-one is worse than the defect the calendar fixed: a regular session's anchor is discarded while the real Muhurat goes on poisoning the next day's | `indicators::evaluator::the_nine_non_regular_days_are_the_charter_dates` | ✓ |
| IF-19 | 278 means UP and 279 means DOWN. I-17 passes with the mapping **swapped**, so the regime could have been reported permanently inverted — a sweep asking for "structure up" would get every bar where it was down, which is a false combination rather than a missing one | `indicators::evaluator::the_structure_direction_bits_are_not_swapped` | ✓ |
| IF-20 | `warmed_up` requires the pivot ladder, not only the session count. The four conditions look coupled and are not: `close_the_books` fills `prev5` unconditionally and installs `yesterday` only `if let Ok(levels)`, so five unusable sessions give a full window with no ladder | `indicators::evaluator::warmed_up_is_false_when_the_pivot_ladder_is_absent_despite_five_sessions` | ✓ |

| IF-21 | `Calendar::default()` is the charter's six, **not** an empty calendar. `impl Default` was the only uncovered FUNCTION in these three crates, and its body was mutated to `all_regular()` uncaught — the same hole twice: a `Default` nobody exercises is one whose value nobody has checked, and an empty one silently restores the defect D-0110 fixed for every consumer that writes `Calendar::default()` | `indicators::evaluator::the_default_calendar_is_the_charters_six_and_not_an_empty_one` | ✓ |

| IF-22 | The market-structure latch advances from the **same** classification the mask was built from. `TrendState::emit` returns both and `step` shares one value, so there is exactly one `classify` on the library path — folding before the advance, or advancing on a different price, were each uncaught mutations and the second is now unwritable | `indicators::trend::bits_is_idempotent_across_a_break_and_a_change_of_character`, `indicators::trend::a_change_of_character_reaches_the_mask` | ✓ |

**I-18, I-19 and I-20 exist because a verification sweep applied mutations that nothing
caught.** All three fixes were green, tested, and documented; three specific breaks passed
undetected — a swapped direction mapping, a shifted calendar date, and a deleted conjunct.
Each is now guarded and each was confirmed by re-applying the mutation.

**I-20 was written twice.** The first version fed five unusable sessions of two bars each and
asserted `!warmed_up()`. It passed with the `yesterday` conjunct deleted, because ten candles
leaves the 200-period EMA unwarmed — so the `false` came from the trend and the assertion said
nothing about the pivot ladder. Forty ordinary bars per session were added so everything else
is warm and a false answer can only come from `yesterday`. **This is the third time in one day
that a test of mine passed for the wrong reason**, and all three were found by mutating rather
than by reading.

**I-10 and I-11 both fail on the obvious slip**, which is why the verdict is taken on
`self.day` and not `today`: asking about the session that is *starting* rather than the one
that just *ended* inverts the fix, so the Muhurat session would poison the anchor and the
regular day after it would be discarded. Measured, by making that exact change.

**I-12 is the row that stops the fix going too far.** An earlier version of I-10's test
compared the whole mask and failed — the EMA, ATR and swing ring had legitimately seen the
60 bars, and the charter says nothing about them. The test was wrong, not the fix.

**I-09 exists because I-08 was not enough**, and that is the general lesson rather than a
detail of this row. Two tests were written, both passed, and then a plausible refactor slip
— deleting the latch advance — was applied and **all 216 tests stayed green**. A test suite
that cannot fail on a plausible break is a suite that will not catch the next one. The
discipline that produced I-02 and I-09 is the same: after a test passes, break the code a
second way and check something notices.

`docs/06-limits.md` §57 records a false derivation in one of the fixes' own reports, and
§58 the three things this phase did **not** close.

## The three routes that answer without running — D-0092, D-0093, D-0094

`GET /ingest/status.json`, `POST /ingest/queue` and `POST /autopilot/control`.
Each of them answers a question an operator had no machine-readable answer to,
and none of them contacts a vendor. The rows are prefixed `IC-` rather than
continuing `A-`: two agents were appending to this file at once and a collided
identifier is worse than a new prefix.

| # | Must hold | Proven by | |
|---|---|---|---|
| IC-01 | `/ingest/status.json` reads **no** file and probes **no** manifest: it projects `autopilot::Status`, which the backfill publishes once per pass. Cost is one uncontended lock and one pass over the feed reports | `api::ingest::route_tests::the_status_reports_the_month_the_window_and_what_is_behind` · read of `ingest::status_json`, which names no `census` call | ✓ |
| IC-02 | Every state names what the next sweep is waiting on **in words** — the operator's pause, every feed halted (with the restart requirement), a task that stopped before its first round, and no round finished yet — and an empty `waiting_on` is never the whole answer | `api::ingest::route_tests::what_the_sweep_waits_on_is_named_in_every_state` | ✓ |
| IC-03 | An unreadable status refuses with **503 and the reason**, never an empty list. A poisoned lock cannot say what is outstanding, and "nothing" is a different fact | `api::ingest::route_tests::the_status_route_answers_and_refuses_an_unreadable_one` | ✓ |
| IC-04 | `POST /ingest/queue` **never** answers that anything was queued, and a legal selection is refused naming what does exist instead | `api::ingest::route_tests::a_legal_selection_is_refused_by_name_rather_than_accepted_and_dropped` | ✓ |
| IC-05 | It writes nothing to the store root, journals nothing, and leaves the pull seat free — asserted against the directory and the seat, not against a comment | `api::ingest::route_tests::queueing_touches_neither_the_store_nor_a_vendor` | ✓ |
| IC-06 | Every refusal names the control it is about, and the one refusal that is about the machine's clock names **`null`** rather than a guessed field | `api::ingest::route_tests::a_refused_selection_names_the_field_and_the_clock_names_none` | ✓ |
| IC-07 | A resume that would change nothing is **refused with 409**, and the halt's own reason survives it — nothing is published over a halt | `api::autopilot::tests::a_resume_against_a_wholly_halted_backfill_is_refused_and_keeps_the_reason` · `api::autopilot::tests::a_resume_before_the_first_round_of_a_halted_task_is_refused` | ✓ |
| IC-08 | A resume that WOULD change something is honoured, and when any feed stays terminal both the answer and the published detail name it | `api::autopilot::tests::a_partly_halted_backfill_resumes_and_names_what_stayed_dead` · `api::autopilot::tests::a_resume_before_the_first_round_of_a_live_task_is_admitted` | ✓ |
| IC-09 | An unreadable status refuses to START and still permits a STOP, saying the reason could not be published rather than pretending it was | `api::autopilot::tests::a_control_over_a_poisoned_status_refuses_to_start_and_still_stops` | ✓ |
| IC-10 | The control takes exactly `start`, `stop`, `resume`; every other word — including `pause`, the sibling route's own name — is refused **by name** and moves no flag | `api::autopilot::tests::the_control_takes_three_words_and_refuses_the_rest` | ✓ |
| IC-11 | All three routes are registered, `GET /ingest/queue` is 405, and **`GET /autopilot` still reaches the front end** — the control is at `/autopilot/control` precisely because a `post`-only route at the bare path would 405 the operator's page | `api::ingest::route_tests::the_three_routes_answer_and_none_of_them_shadows_the_front_end` | ✓ |

**What is NOT invariant here, and is a limit rather than a bug.** IC-01's answer
is as old as the last finished round, and before the first one there is no
survey at all. That is why `surveyed` is a field and IC-02 is a row: the
staleness is reported, not removed. See `docs/06-limits.md`.

A halt is cleared by no **control** — no route can reach `FeedState::halted`,
because the feed table is a local of `autopilot::fly`. IC-07 is that fact made
checkable rather than a workaround for it, and D-0093 says why a revive control
was rejected rather than half-built. **Since D-0108 two of the three halt classes
are additionally cleared by EVIDENCE**, which is not a control and cannot be
pressed: a census that loads again and a disk that accepts a write probe. The
third — a dead broker credential — is cleared by neither, and is never re-checked
at all, because `CLAUDE.md` §8 forbids minting a token and a retry against an
unchanged dead value is §4's banned shape. See the `AU-` rows below.

## The autopilot flies on Run, and a halt is probation only where it can be measured — D-0108

The rows are prefixed `AU-` rather than continuing `IC-`: two sessions share this
tree and a collided identifier is worse than a new prefix.

Every row below is proved with no socket, no credential, no vendor and no bar.
The two rows that touch a disk touch a scratch directory this suite owns.

| # | Must hold | Proven by | |
|---|---|---|---|
| AU-01 | **The boot default FLIES and exactly one byte string holds it on the ground.** An absent `BRUTEX_AUTOPILOT` flies — that is the tracked Run configuration's own state, and the whole of the reported "press Run and nothing happens" defect. Only `pause` grounds it: `PAUSE`, `Pause`, `paused`, ` pause`, `pause `, `pause\n`, `stop`, `false`, `0`, `no`, `off` and the empty string all fly, and `run` is still accepted as a flying value so an existing alias does not silently change meaning. The environment reader and the pure decision are asserted to agree | `api::autopilot::tests::the_boot_default_flies_and_only_the_exact_word_pause_holds_it` | ✓ |
| AU-01b | **The grace window is a real window on a site that is NOT paused.** Under the old default a served `Control` came up paused, so "nothing is contacted before somebody could say no" was guaranteed by the pause; flying by default moves that whole guarantee onto the countdown, so it is asserted rather than assumed — `grace` on an unpaused site does not return inside 300 ms, publishes `starting` with the Pause instruction while it waits, and journals nothing. `GRACE_SECS >= 5` is additionally a **compile-time** floor beside the constant, so shrinking the consent gate fails the build rather than a test. This is also the margin that keeps `cargo test` off the vendor on the one test that drives `run` end to end: `run_in` spawns `fly` and aborts it the moment `serve` returns, which for an already-resolved shutdown future is microseconds inside a twenty-second window | `api::autopilot::tests::an_unpaused_grace_window_really_waits_before_anything_is_contacted` · `const _: () = assert!(GRACE_SECS >= 5)` in `api::autopilot` | ✓ |
| AU-02 | **A credential halt requires that NOT ONE instrument was reached.** A sweep that reached 772 of 773 and saw one `status 401` backs off and then stalls — bounded and visible — instead of making the feed terminal for the life of the process; the feed-wide case still takes its one §8 re-read and then halts naming §8. A run blocked before it attempted anything counts as feed-wide, because halting is the direction that costs nothing outside this machine | `api::autopilot::tests::a_credential_reason_from_a_sweep_that_reached_somebody_backs_off_instead_of_halting` | ✓ |
| AU-03 | **A dead credential arms NO re-check of any kind** — no probe, no schedule, no timer. `CLAUDE.md` §8 forbids minting a token and §4 forbids a retry that hides a permanent fault, so this class is named and left named. Only `Halt::Store` arms anything, and each halt class carries its own distinct word | `api::autopilot::tests::every_halt_class_names_itself_and_only_the_store_class_arms_a_probe` · `api::autopilot::tests::a_credential_reason_from_a_sweep_that_reached_somebody_backs_off_instead_of_halting` | ✓ |
| AU-04 | **A census halt clears on `Census::Held` and on nothing else.** A manifest an operator repaired puts the feed back on the ladder on the next pass, with no restart, and the page says why; `Census::Unreadable` keeps it terminal and — the narrow rule — `Census::Absent` does **not** revive it, because a census reporting nothing held is not evidence the fault is gone, and a feed revived on it would re-offer months whose bar files `BarFile::append` refuses wholesale | `api::autopilot::tests::a_repaired_manifest_clears_its_own_halt_and_an_absent_one_does_not` | ✓ |
| AU-05 | **The store-halt probe is bounded at `STORE_PROBES` = 8 and gives up out loud.** The schedule doubles from 60 s and caps at 3600 s — seven gaps, 7,380 s, read back from `probe_secs` rather than trusted from a comment — and the ninth request answers `Due::Spent` however long is waited, saying the allowance is spent and that nothing further is written or read. An unarmed feed answers `Spent`, never `Now` | `api::autopilot::tests::the_store_probe_is_bounded_measures_the_disk_and_says_so_when_it_is_spent` | ✓ |
| AU-06 | **The probe is a MEASUREMENT of a real disk and leaves nothing behind.** Three probes against a writable root leave the directory entry count where they found it (§3 rule 5), and a root that cannot be created reports the host's own refusal naming the path — never "the disk is fine" | `api::autopilot::tests::the_write_probe_measures_the_disk_and_leaves_nothing_behind` | ✓ |
| AU-07 | **A store halt clears inside a real `round`, on evidence, with nothing asked of any vendor.** The store refusal halts the feed and arms a probe due immediately; one round probes the scratch store root, the write succeeds, the halt is cleared, and the published detail says both that it cleared and that no vendor was contacted to establish it | `api::autopilot::tests::a_store_halted_feed_probes_the_disk_inside_a_round_and_comes_back` | ✓ |
| AU-08 | **A stalled month is reconsidered at most `STALL_RETRIES` = 2 times per process and then never again.** The first idle pass that sees a stall stamps it and does not retry it; one second short of `STALL_RECHECK_SECS` is still short of it; the allowance is exhausted one reconsideration at a time and afterwards `reconsider` answers `None` however long is waited — time does not renew it. The worst case `MAX_MONTH_ATTEMPTS × (1 + STALL_RETRIES)` = **9** attempts per stalled month per process is asserted as arithmetic, and `stall_note` says out loud which stalls are SPENT | `api::autopilot::tests::a_stalled_month_is_reconsidered_twice_at_the_earliest_and_then_never_again` | ✓ |
| AU-09 | **Reconsideration takes the oldest month across feeds, one per pass, and never on a terminal feed.** The same rule the ladder itself climbs by, so a reconsideration cannot jump the queue; a halted feed owes none, because there is nothing to drive; and an empty stall list produces an empty note rather than a reassuring sentence about a fact nobody asserted | `api::autopilot::tests::reconsideration_takes_the_oldest_month_and_skips_a_terminal_feed` | ✓ |
| AU-10 | **A reconsideration happens only from the idle branch of a real round, moves the frontier BACK to that month, and the page and the detail agree about which attempt it is.** The round returns `0` rather than `IDLE_POLL_SECS` because a reconsidered month is work; the JSON carries `retried` and `retries_max` beside the original reason verbatim; and a second round in the same interval does not ask again | `api::autopilot::tests::a_round_with_nothing_missing_reconsiders_a_stalled_month_and_says_which_attempt` | ✓ |
| AU-11 | **An unusable clock is waited on a bounded number of times and then named.** Every wait says which check of `CLOCK_WAITS` = 20 it is and that nothing is being contacted meanwhile; past the bound `clock_wait` answers `None` and `fly` stops saying the allowance is spent. It used to `return` on the first reading, which made an NTP-shaped fault terminal for the life of the process and made every later resume refusable for ever | `api::autopilot::tests::the_clock_is_waited_on_a_bounded_number_of_times_and_then_named` | ✓ |

**What is NOT invariant here, and is a limit rather than a bug.** `/health` still
answers 200 while every feed is terminal — it reads the masters and nothing else
— so a monitor cannot yet learn from a status code that the backfill is dead. A
halt also still writes nothing durable to the event log; the only two emit sites
in `autopilot.rs` are pause and resume. Both are changes to files another session
holds (`server.rs`, `emitted.rs`), and A-40's source-derived emit-site count is
the reason the second cannot be done unilaterally. D-0108 records both, and until
the first exists the owner is still required to *look* in order to learn that a
credential died.

## The four NIFTY tiers are requestable and none of them is swept — D-0105

`api::ingest::SpotTarget` went from three variants to seven so that a pull can
name the NIFTY 50, 100, 200 and 500. The rows are prefixed `ST-` rather than
continuing `IC-`: two sessions share this tree and a collided identifier is worse
than a new prefix.

Every row below is proved without a socket, a credential or a bar. The
membership facts come from compile-time tables in `brutex_core::universe`.

| # | Must hold | Proven by | |
|---|---|---|---|
| ST-01 | Each tier target resolves to its **published constituent list borrowed from `core`** — the same names in the same order, 500 / 200 / 100 / 50 of them — and every one of those names carries that tier's `Universe` bit, so the roster and the membership predicate are one set and not two lists that agree today | `api::ingest::tests::the_n500_target_resolves_to_the_five_hundred_published_constituents` · `api::ingest::tests::the_n200_target_resolves_to_the_two_hundred_published_constituents` · `api::ingest::tests::the_n100_target_resolves_to_the_one_hundred_published_constituents` · `api::ingest::tests::the_n50_target_resolves_to_the_fifty_published_constituents` | ✓ |
| ST-02 | **A tier is STORED and never SWEPT** (`CLAUDE.md` §1). No constituent of any tier is `is_sweepable`, none is named by `SpotTarget::Swept`, neither swept index is named by any tier, and `InstrumentKey::SWEPT` still has exactly **two** entries — widening the engine surface breaks a test whose name says why | `api::ingest::tests::a_nifty_tier_target_stores_and_never_sweeps` | ✓ |
| ST-03 | The slug a pull is requested WITH is the token `/instruments.json` counts a row BY — `n50`, `n100`, `n200`, `n500`, `server::UNIVERSE_TOKENS` verbatim — and it round-trips through `from_slug` to exactly one variant | `api::ingest::tests::the_n50_target_resolves_to_the_fifty_published_constituents` and its three siblings · `api::ingest::tests::a_spot_request_names_its_target_or_is_refused` | ✓ |
| ST-04 | **An unknown slug is still refused BY NAME.** The new arms make no default: `nifty50`, `nifty-50`, `N50`, `n 50`, `n25`, `ntm`, `fno`, `*`, `mcx` and `bse` each refuse as `UnknownTarget`, the refusal quotes what arrived and lists every legal slug, and an empty field stays its own separate refusal | `api::ingest::tests::a_slug_that_is_not_a_target_is_refused_by_name_after_the_tiers_were_added` | ✓ |
| ST-05 | `members()` answers **`None` rather than an invented list** for the two targets no published file defines — the engine surface and the vendor master's index series — and `Some` for every other target | `api::ingest::tests::the_two_targets_with_no_published_list_return_none_rather_than_an_invented_one` | ✓ |
| ST-06 | Every target's counter is its **own** population: the count the form shows and the count the receipt echoes come from the slot that target occupies, for all seven, end to end over a real POST | `api::server::tests::each_spot_target_reports_its_own_population_and_not_a_neighbours` | ✓ |

**What is NOT invariant here, and is a limit rather than a bug.** A tier being
*requestable* is not a tier being *fetchable*. The broker path refuses every
target that names a set — tiers included, exactly as `equities` always has been —
because `pull::vendor::HttpSpec` carries no request-parameter map;
`api::server::broker_target_tests::only_the_swept_target_names_a_single_instrument_this_path_can_reach`
pins that to one target and is the reminder to widen the guard when the map
lands. The archive path does not read the target at all. D-0105 records both.

## The history floor is a fact about a RUNG, and the wire end is a fact about a VENDOR — D-0113

The rows are prefixed `HF-` rather than continuing an existing block: two sessions
share this tree and a collided identifier is worse than a new prefix.

Every row below is proved with no socket, no credential, no vendor and no bar. The
one row that resolves a rolling floor does it against a **fixed** day written into
the test, because a test that reads the clock asserts something different every day
it runs.

| # | Must hold | Proven by | |
|---|---|---|---|
| HF-01 | **A feed's history CLAIM is keyed on the RUNG, and one vendor's two rungs carry two different claims.** Groww's own interval table gives its `1 day` row "Full history" and its `1 min` row "Last 3 months", so a single per-vendor claim is necessarily wrong for one of them: `contested` answers `RollingMonths { months: 3 }` at `1min` and `Unbounded` at `1day`, and the two are asserted to differ. What BINDS is a separate field and is the operator's figure — the same `Fixed 2020-01-01` at both rungs — so the vendor's per-rung claim and the binding floor are two fields rather than one. Every recorded row names a rung the store can actually file | `pull::vendor::one_vendors_two_rungs_carry_two_different_claims` | ✓ |
| HF-02 | **A rung nothing states a floor for claims NOTHING** — not zero, not the epoch, not "no limit". Both archive feeds record an empty table and every rung of both answers `HistoryFloor::Unstated`; a broker rung no source speaks about (`60min`, spelled `1hr` until D-0132 gave the rung the store's own name) answers the same rather than inheriting the rung beside it | `pull::vendor::a_rung_with_no_recorded_floor_claims_nothing` | ✓ |
| HF-03 | **`none` and `unknown` are two different facts and never collapse.** A vendor stating it serves back to inception is `HistoryFloor::Unbounded`; nobody having stated anything is `Unstated`. Both resolve to no day — that is asserted — and `/feeds.json` still reports them as two different words, so a reader can tell a claim from the absence of one | `api::server::the_two_floors_that_name_no_day_resolve_to_none` · `api::server::feeds_json_carries_a_floor_per_rung_with_the_source_that_made_it` | ✓ |
| HF-04 | **Where the operator and the vendor documentation disagree, BOTH are carried and the stricter binds.** Three of the four recorded rows are contested; every contested row names the displaced claim, its source and why it lost, every uncontested row carries an empty reason, and no claim that agrees is recorded as a contest. The binding floor is asserted to be the LATER day of the two — the one that refuses first | `pull::vendor::a_contested_floor_keeps_the_claim_it_displaced` · `pull::vendor::the_binding_floor_is_never_the_looser_of_the_two` | ✓ |
| HF-05 | **The per-vendor floor the clamp reads is never LATER than a rung's own.** Two fields answer "how far back", and only one direction of drift is survivable: earlier spends requests a vendor answers empty, later refuses days it holds — and a refused day cannot be prepended into an append-only store | `pull::vendor::the_vendor_wide_floor_is_never_later_than_a_rungs_own` | ✓ |
| HF-06 | **A months-shaped rolling floor is a calendar walk, and the day of the month is clamped DOWN.** 31 May less three months is the last day February has, in a leap year and a common one alike, and 31 July less one month is 30 June — never a spill forward into the next month, which would move a floor later than the vendor's. An ordinary day is untouched, a walk below 1970 is refused by name, and three months is asserted to be a calendar span rather than 90 days | `pull::session::a_short_month_clamps_the_day_rather_than_spilling_into_the_next` · `pull::session::a_month_walk_keeps_the_day_and_borrows_across_january` · `pull::session::a_walk_below_the_first_year_this_build_can_name_is_refused` | ✓ |
| HF-07 | **A rolling floor is COMPUTED and never stored**, and the day the API shows is the day the pull will be clamped to: `/feeds.json` resolves it through `clamp_to_floor` itself, so there is one arithmetic site rather than two answers. The emitted day is asserted to be a rendered date rather than a literal anybody wrote down | `api::server::feeds_json_carries_a_floor_per_rung_with_the_source_that_made_it` | ✓ |
| HF-08 | **The wire end covers the last session the operator named, and not the next one** — for BOTH feeds, whose conventions are opposite. Dhan's `toDate` is documented non-inclusive so the wire value is the day after; Groww's `end_time` is an instant taken inclusively, so the day is unchanged and the clock is pushed to the end of it. The property is asserted in terms of neither: the wire end falls strictly after the session's last print (09:15–15:29, `docs/00-charter.md` §3) and strictly before the next session's first | `pull::http::every_feeds_wire_end_covers_the_last_session_and_not_the_next` | ✓ |
| HF-09 | **The range end is a PER-VENDOR fact and the wrong one is a silent loss.** With Dhan's row read as inclusive the request would end before the session it names — one session lost per request, indistinguishable from a holiday. With a blanket `+1` applied to Groww the request names a day outside the window the operator asked for. Both wrong directions are asserted, so neither row can be "simplified" into the other | `pull::http::the_range_end_is_a_per_vendor_fact_and_the_wrong_one_loses_a_session` | ✓ |

**What is NOT invariant here, and is a limit rather than a bug.** `fetch_chunks`
still clamps with the **per-vendor** `HttpSpec::history_floor`, so Groww's
one-minute rung is bounded at 2020 by the pull while the API correctly reports its
three-month floor. HF-05 is what keeps that safe rather than wrong — the pull
over-asks and the vendor answers empty; it never under-asks. Closing it is a
one-line change at the call site and it changes what a live pull does, so D-0113
records it rather than taking it in passing. Separately, `pull::fetch::wire_end` and
`Window::wire_to` are two further "wire end" implementations with no caller outside
tests, and the second carries a blanket `+1` with no vendor in its signature; HF-08
and HF-09 hold for the path that actually reaches a socket and say nothing about
those two.

## The granularity floor is a fact about a VENDOR, and a tick is a rung nobody serves — D-0118

The rows are prefixed `GF-` for the reason the `HF-` block gives: two sessions
share this tree and a collided identifier is worse than a new prefix.

Every row below is proved with no socket, no credential, no vendor and no bar.
`HF-*` above answers **how far back** a rung reaches; these answer **how fine**
a vendor goes, and GF-04 is the row that keeps the two from being read as one
refusal.

| # | Must hold | Proven by | |
|---|---|---|---|
| GF-01 | **No feed in this build serves a tick stream, and the refusal is permanent.** It is not a floor some feed clears — the finest rung of the ladder is refused by all four feeds, and no row's floor claims to be a print stream. `FinestKind::Tick` exists so the negative can be stated and is constructed by no descriptor; a `const` block under `DESCRIPTORS` makes a row that claims one a build failure rather than a review comment. The variant's own behaviour is exercised directly, because a variant no test constructs is one nobody has checked | `pull::vendor::no_feed_in_this_build_serves_a_tick_and_the_refusal_is_permanent` | ✓ |
| GF-02 | **A conflated one-second snapshot is never labelled a tick.** Both archives bottom out at one second and both were MEASURED to be snapshots — whole-second timestamps, no sub-second field, several rows per second with no tiebreaker (`docs/08-vendor-samples.md`). Both brokers bottom out at one minute and both are candles. The kind travels with the floor rather than being inferred from the rung, because one second of an archive and one second of a broker would be two different objects; `is_conflated` and `is_tick_stream` are asserted on both shapes, and the three labels are asserted to differ so two of them cannot read the same on a page | `pull::vendor::a_conflated_second_is_never_labelled_a_tick` | ✓ |
| GF-03 | **Every feed answers every rung, with one of two verdicts, in O(1).** The whole 4 × 11 matrix: a rung is refused exactly when it is finer than the vendor's floor, exactly one rung per feed answers `Finest`, the three arms sum to the ladder's length, and every refusal names the rung asked for, the rung served instead, what a record at it is, a non-blank reason and a non-blank source. A rung ABOVE the floor answers `None` for its record shape rather than guessing one — nothing here measured it | `pull::vendor::every_feed_answers_every_rung_with_one_of_two_verdicts` | ✓ |
| GF-04 | **The granularity floor and the history floor refuse different things, and only one of them could ever move.** Groww at one second is the vendor having no such rung at any date; Groww at one minute in 2019 is the vendor having the rung and not that far back. The first is asserted refused with `HistoryFloor::Unstated` beside it — no source states a depth for a rung the vendor does not have — and the second is asserted NOT refused with a `RollingMonths { months: 3 }` depth. The archives are the mirror image: their granularity floor is measured and their history floor is stated by nobody | `pull::vendor::the_granularity_floor_and_the_history_floor_refuse_different_things` | ✓ |
| GF-05 | **A build never fetches a rung its vendor cannot serve.** `granularities` may be NARROWER than the floor allows — Dhan's minute rung is exactly that, and the reason is this build's single `bars_path` rather than anything Dhan does — and it may never be wider. Checked as one mask per row by the compiler and driven from outside here, including the ladder's edge where nothing is finer than the first rung. The two refusals are asserted to differ: Dhan does not fetch the minute rung and Dhan's vendor does not refuse it, which is a refusal a code change fixes | `pull::vendor::a_build_never_fetches_a_rung_its_vendor_cannot_serve` | ✓ |
| GF-06 | **Every granularity floor names a reason and a source, in the vendor's own terms.** `CLAUDE.md` §3 rule 1 — a refusal an operator cannot go and check is one they have to take on faith. All four reasons are asserted non-blank and pairwise distinct, and so are all four citations, so no row can wear another vendor's words | `pull::vendor::every_granularity_floor_names_a_reason_and_a_source` | ✓ |
| GF-07 | **The ladder ascends, which is what makes the single comparison sound.** `Granularity::is_finer_than` compares two `u8` discriminants; that is an answer only if the discriminants are ordered the way the grids are. The `const` block beside the function pins the ten adjacent pairs for the compiler; the test walks all 121 ordered pairs, because a ladder can ascend between neighbours and still not be a total order three steps away. Irreflexivity is asserted too | `pull::vendor::the_ladder_ascends_so_one_comparison_decides_which_rung_is_finer` | ✓ |

**What is NOT invariant here, and is a limit rather than a bug.** The front end
is untouched by this change. `web/src/routes/ingest/+page.svelte` still renders
a rung the vendor can never serve with the same *no directory* annotation it
gives a rung nobody has pulled yet — the store-shaped refusal and the
vendor-shaped one still read identically on the page. Nothing above claims
otherwise: these rows are about `crates/pull`, and the fact now exists in one
place where before it existed in none. Carrying it over the HTTP surface is the
next phase and is recorded in D-0118 rather than implied here. Separately,
`docs/00-charter.md` §4 has **no vendor rows at all** for TrueData and GDFL, so
GF-02's sources are `docs/08-vendor-samples.md`'s measurements and the owner's
rule of 2026-08-12 rather than a charter row; that gap is named in D-0118.

## An NSE tier resolves to vendor ids on `(exchange, ISIN)`, and the four buckets sum — D-0117

`brutex_core::universe` held six published constituent lists and `api::master`
parsed both vendor masters, and **nothing joined them**: there was no code
anywhere that turned "the NIFTY 200" into the instrument ids a pull must name.
`api::constituents` is that join. The rows are prefixed `CJ-` rather than
continuing an existing block: two sessions share this tree and a collided
identifier is worse than a new prefix.

Every row below is proved with no socket, no credential, no master file and no
bar — the fixtures are `merge::merge` over hand-written listings, and the ISINs
in them are real ones read off the two masters on 2026-08-12, because
`Isin::new` verifies the check digit and a fixture that could not exist proves
nothing about a join that will meet the real file.

| # | Must hold | Proven by | |
|---|---|---|---|
| CJ-01 | **The partition sums, for every vendor and every tier.** `matched + lacks + ambiguous + malformed + no_nse_isin == Tier::published()` — five buckets since D-0125 — and not only as a total: they are asserted DISJOINT and their union asserted equal to the published list *name for name*, so a join that dropped one constituent and double-counted another cannot pass. The five addends and the total are PRINTED for all twenty-four `(vendor, tier)` rows, so the arithmetic is readable and not merely asserted. Held for all four vendors — including the two archives, which publish no master and legitimately lack everything | `api::constituents::every_constituent_lands_in_exactly_one_bucket` · `api::constituents::a_vendor_with_no_master_lacks_every_name_and_the_sum_still_holds` | ✓ |
| CJ-02 | **The join key is NSE's OWN ISIN, at both ends, and never a symbol.** Since D-0125 a constituent's identity is `core::universe::nse_isin(symbol)` — the ISIN the exchange prints beside that name in its own file (U-08) — matched against the ISIN column of the vendor's master. Every matched, lacked and ambiguous row of every tier is asserted to carry exactly that value. One ISIN resolves to a different id per vendor — `2885` at Dhan, `NSE-RELIANCE` at Groww — and the tier's answer carries that vendor's own id, never the other's. A vendor that lists no row for the ISIN yields `None` rather than a substitute, because filing one broker's bars under another's id is what D-0019 exists to prevent | `api::constituents::every_matched_row_joined_on_the_isin_the_exchange_itself_prints` · `api::constituents::a_constituent_both_masters_list_resolves_to_that_vendors_own_id` · `api::constituents::a_constituent_one_master_lacks_is_reported_and_never_dropped` | ✓ |
| CJ-03 | **Two rows of one master claiming one NSE ISIN is `ambiguous`, and no id is chosen.** Every claimant is listed; the tier's id list gains none of them; `Join::id` answers `None`. The same ISIN stays unambiguous for the other vendor, so ambiguity is a fact about ONE master and never about the ISIN | `api::constituents::two_rows_of_one_master_claiming_one_isin_are_ambiguous_and_no_id_is_chosen` | ✓ |
| CJ-04 | **Every "no ISIN to join on" is a named reason on the row, in the bucket that names whose gap it is.** D-0125 split them. `malformed` is a published name this build cannot make a key of — `NotASymbol`, `PlaceholderScrip` — and is empty on all six lists. `no_nse_isin` is **the exchange's own file naming no ISIN**: `NoRowInTheExchangesFile` (the five F&O index underlyings, which are not shares) and `TheExchangesRowNamesNoIsin` (`AGL`, one Total Market row with an empty cell, §11). The second bucket is asserted IDENTICAL for all four feeds, because it is a fact about NSE's file and about no master. Every reason prints a sentence and is asserted to answer the right side of the split | `api::constituents::the_exchanges_own_gap_is_its_own_bucket_and_the_same_for_every_feed` · `api::constituents::a_name_that_cannot_be_a_key_is_malformed_and_never_probed` · `api::constituents::every_reason_prints_a_sentence_and_says_which_bucket_it_belongs_to` | ✓ |
| CJ-05 | **There is no symbol step left to say out loud, and a vendor row spelled correctly still lacks.** D-0125 deleted it rather than demoting it: a constituent whose NSE ISIN no master carries is `lacks`, never retried by name. The proof is a master row whose SYMBOL is a constituent's and whose ISIN is another company's — the old join matched it by name; the new one lacks it, and that vendor's id is reachable only under the ISIN its own master gave. What survives is a different measurement: `Matched::corroboration` names which mastered vendors' files carry NSE's ISIN, `TierJoin::uncorroborated` lists the matched rows exactly one file agrees with, and the note carries that count — on the line it affected and on no other, asserted against a universe where nothing is uncorroborated | `api::constituents::the_symbol_step_is_gone_and_a_vendor_row_spelled_right_still_lacks` · `api::constituents::the_notes_name_the_five_buckets_their_sum_and_every_unresolved_row` | ✓ |
| CJ-06 | **A tier's count is checked against the list it borrows, and no list holds a placeholder scrip.** `Tier::published()` is a literal — 50 / 100 / 200 / 500 / 750 / 213 — asserted equal to `members().len()`, so a truncated transcription fails rather than answering for fewer names than the index has; and `DUMMYINXGN` / `DUMMYTRVN` appear in no list this build carries, which asserts D-0089's drop instead of trusting it. `Tier::ALL` is pinned in position order, because `Join::tier` indexes a flat array by it | `api::constituents::every_tiers_published_count_matches_the_list_it_borrows` · `api::constituents::no_tier_this_build_carries_holds_a_placeholder_scrip` | ✓ |
| CJ-07 | **The join and `core::universe::of_equity` cannot drift.** Every matched row of every tier carries that tier's own `Universe` bit — the two are built from the same six lists by different routes, and if they disagree one of them is reading a list the other is not | `api::constituents::every_matched_row_also_carries_the_tiers_universe_bit` | ✓ |
| CJ-08 | **Both lookups are O(1) and neither notices the universe behind it.** `(vendor, tier) -> ids` is one index into a flat array and `(vendor, exchange, ISIN) -> id` is one hash probe on a `Copy` key; measured against universes 40× apart in indexed ISINs (100 against 4,000), both are flat within a 4.0× ceiling. Re-measured after D-0125 re-keyed the join on NSE's own ISIN, release profile, best of 9 × 200,000: **302 → 302 ps** and **1,515 → 1,212 ps** — the array index is unchanged and the hash probe is FASTER on the larger universe, which is the noise floor rather than a speed-up, and is the shape a constant-time operation makes. D-0125 added two constant probes per constituent at BUILD time (`NTM_INDEX::position` and `nse_isin`) and removed one merged-universe probe; nothing moved onto a request path. The whole-universe pass happens once, in `Read::new`, beside `Catalog::build` — D-0039 and D-0042 | `api::constituents::the_two_lookups_do_not_grow_with_the_universe` | ✓ |

**Measured against the real masters, 2026-08-12** — 2,795 merged instruments,
2,760 distinct `(exchange, ISIN)` pairs indexed, `Join::build` **441 µs**. Both
vendors resolve **50/50, 100/100, 200/200, 500/500 and 750/750** with zero
lacking, zero ambiguous, zero malformed and **zero rows resting on a single
master's spelling**. The F&O list resolves 208 of 213, and the five that do not
are `BANKNIFTY`, `FINNIFTY`, `MIDCPNIFTY`, `NIFTY` and `NIFTYNXT50` — index
underlyings, which have no ISIN because no numbering agency issues one for a
computed level.

## Per-feed coverage — `api::coverage` (D-0120)

What a *universe* holds and what a *feed* can be asked for are two numbers, and
the form showed only the first. These rows hold the second.

| # | Invariant | Proof | |
|---|---|---|---|
| CV-01 | **A target has a tier exactly when it has a published list, and it is that list.** `SpotTarget::tier()` and `SpotTarget::members()` are pinned against each other for all seven targets, and the tier's `members()` and `universe()` are asserted equal to the target's — so a target cannot join through a neighbour's list, which is precisely what a positional mapping gets wrong | `api::ingest::every_target_with_a_published_list_joins_through_that_lists_tier` | ✓ |
| CV-02 | **A target resolves to ONE feed's ids, and `None` is not an empty list.** `Join::ids_for(vendor, target)` hands back exactly `Join::tier(vendor, target.tier()?).ids()`; `Swept` and `Indices` answer `None` because no published list defines them, which is a different claim from "this feed reaches nothing in it". One feed's ids never appear in another's answer | `api::constituents::a_spot_target_resolves_to_one_feeds_ids_and_the_two_that_cannot_say_so` | ✓ |
| CV-03 | **The count on the control is the length of the id array a request is built from.** `Covered::matched` is `ids_for(..).len()`, not a second tally kept beside the matched rows — two counts of one fact drift the first time one of them is edited, and the visible half is the one nobody edits | `api::coverage::the_matched_count_is_the_length_of_the_ids_a_pull_would_name` | ✓ |
| CV-04 | **Every unreachable name is named with its bucket and its reason.** `lacks` is "some master knows this ISIN and THIS feed has no row for it"; `ambiguous` is two of the feed's own rows claiming one ISIN; `malformed` is a published name with no usable ISIN at all — including `no vendor master names it`, which is not the same fact as `lacks` and must not be collapsed into it. The list is emitted whole and never truncated | `api::coverage::the_wire_carries_the_slug_the_count_and_every_reason` · `api::server::universes_json_says_what_each_target_resolves_to_for_the_named_feed` | ✓ |
| CV-05 | **A feed with no instrument master answers `None`, never zero.** The two archive vendors publish no master — a folder of CSVs is its own listing — so every count is null and the wire says `"counted_from":"no master"`. `0 matched, 35 lacks` would be a measurement of a file that does not exist, and it reads as "this feed has nothing" | `api::coverage::a_feed_that_publishes_no_master_is_not_reported_as_empty` | ✓ |
| CV-06 | **The two targets no published list defines carry no denominator.** `Swept` is the engine surface and `Indices` is whatever a master calls an index series; `published` is `None` for both and `counted_from` says `master`. Inventing a denominator for them would be `CLAUDE.md` §3 rule 1. For the other five, `matched + lacks + ambiguous + malformed == Tier::published()` | `api::coverage::the_five_list_defined_targets_read_the_join_and_the_two_others_do_not` | ✓ |
| CV-07 | **A per-feed answer differs per feed, and the fixture proves it rather than the prose.** Three index series, Groww listing two and Dhan two, only one shared: the universe count is 3 and neither feed reaches 3. The swept pair is counted the same way, because "the engine surface is two instruments" does not mean a given broker lists both | `api::coverage::the_two_feeds_reach_different_index_sets_and_the_counts_say_so` · `api::coverage::the_swept_surface_is_counted_per_feed_like_everything_else` | ✓ |
| CV-08 | **A shortfall produces a startup note; a whole build produces silence — and the note does not claim a filter the pull path lacks.** Only `(feed, target)` pairs that are short are named, with the names on the line; and the sentence says the run STILL ATTEMPTS the universe's number and refuses the difference by name, which is `docs/06-limits.md` §63 written where it is true | `api::coverage::the_notes_name_only_the_targets_a_feed_is_short_of` | ✓ |
| CV-09 | **The receipt prints both numbers with different labels, and neither is a bare count.** *Instruments covered* says `N in the merged universe`; *This feed can name* is the reach, as a whole set, as a fraction with the first five missing names and a remainder, or as "publishes no instrument master" — and it points at `/universes.json?feed=…` for the rest. All seven targets are driven end to end through a real server | `api::server::each_spot_target_reports_its_own_population_and_not_a_neighbours` · `api::server::the_receipt_reports_reach_for_the_feed_and_never_a_number_it_did_not_measure` | ✓ |
| CV-10 | **An unknown feed is refused by name on `/universes.json`, and a compound bitset gets no single token.** 400 naming what was asked and what is known, rather than `/instruments.json`'s default-to-Dhan — which there would enable four controls against the other broker's reach. `universe_token_of` answers `""` for the empty set and for two bits at once, because a set's name is the array field | `api::server::universes_json_says_what_each_target_resolves_to_for_the_named_feed` · `api::server::a_compound_bitset_has_no_single_token` | ✓ |
| CV-11 | **The lookup is one array index and does not grow with the universe.** `Coverage::of(vendor, target)` measured against universes 40× apart in merged rows, flat within a 4.0× ceiling. `Coverage::build` is one fold over the merged universe per `(mastered vendor, master-counted target)` — four folds — and it runs in `Read::new` beside `Catalog::build`, once per process. D-0039, D-0042 | `api::coverage::the_lookup_does_not_grow_with_the_universe` · `api::coverage::every_target_indexes_its_own_slot` | ✓ |

**Measured against the real masters, 2026-08-12.** Every NIFTY tier is whole for
both feeds — `n50` 50/50, `n100` 100/100, `n200` 200/200, `n500` 500/500,
`equities` 750/750, zero lacking, zero ambiguous, zero malformed. `swept` is
2/2 for both. The one target the universe over-promises is `indices`: **35**
merged index keys, of which Groww lists **24** and Dhan **15**, with only four
carrying both — a spelling disagreement no ISIN can reconcile, because an index
has none. `docs/06-limits.md` §64.

**What is NOT invariant here, and is a limit rather than a bug.** The ISIN a
constituent joins on is **not** the exchange's own: this repository holds the
symbol column of the NSE constituent files and nothing else (`docs/00-charter.md`
§4c), so the ISIN comes from the merged vendor universe and is corroborated by
both masters rather than published. That is recorded in `docs/06-limits.md` §62
and in D-0117. Separately, **the four NIFTY tiers are now requestable from the
API and still disabled in the browser**: D-0120 wired `SpotTarget::tier` →
`Join::ids_for` → `Coverage` → `GET /universes.json?feed=<vendor>`, so the slug,
the per-feed count and every unreachable name with its reason are on the wire —
but `web/src/routes/ingest/+page.svelte` carries `target: null` on those four
rows of its own `UNIVERSES` table, and that is a change in `web/` and not in any
crate. And a run of a short target still ATTEMPTS the universe's number rather
than the feed's, which is `docs/06-limits.md` §63.

## A source is REST or a folder, and a folder's reach is the files — D-0123

The operator's rule of 12 Aug 2026 — TrueData and GDFL are read entirely from
bought CSV files in a folder, every other feed is always REST — is a two-valued
property of the vendor, and it decides five separate behaviours that were
previously decided one at a time by matching on a transport's payload. These
rows hold the two kinds apart at the places where treating them alike produced a
precise and wrong answer.

The one that matters most is the third: **a folder that is missing and a folder
that is there and empty are not the same event.** Both used to produce no bars
and no reason, so both got read as "no data yet" — and an operator spent the
next hour on tokens and entitlements when the answer was a path.

| # | Invariant | Proof | |
|---|---|---|---|
| SK-01 | **Every feed is REST or a folder, and exactly the two named vendors are folders.** The membership is checked by a `const` block under `DESCRIPTORS` rather than asserted in a doc comment, so a fifth row that contradicts the operator's rule is a build failure. `store_vendor` is `TrueData`/`Gdfl` on exactly the two rows whose transport is a folder, which is the same statement seen from the store-prefix side | `pull::folder::every_feed_is_rest_or_folder_and_the_two_named_vendors_are_the_folders` · `pull::vendor` `const` block | ✓ |
| SK-02 | **The kind decides the credential, the quota, the floor and the finest rung — as behaviour, not as a label.** `needs_credential` is false for a folder and `crate::config` asks that rather than naming vendors, so `CLAUDE.md` §8's machinery does not run and a missing token cannot block a source with nothing to authenticate against. `has_history_floor` is false, and `Descriptor::history` is EMPTY for both folder rows so there is nothing to fall back to. `finest_possible` is one second for a folder and one minute for REST, cross-checked against every row's own measured floor by a `const` block | `pull::folder::the_kind_decides_credential_quota_floor_and_finest_rung` · `pull::folder::a_folder_feed_needs_no_credential_and_a_rest_feed_does` | ✓ |
| SK-03 | **The verb for a folder is `read`, everywhere it appears.** Nothing is asked of anybody, no quota moves, nothing can rate-limit it and there is no remote party to be unavailable. `verb_past` is spelled out rather than suffixed, because `read` does not take an `-ed` and a helper that appended one would have written `readed` on the arm the type exists for. `/feeds.json` carries the word so the browser cannot coin its own | `pull::folder::the_verb_for_a_folder_is_read_and_never_pull` · `api::server::feeds_json_carries_a_floor_per_rung_with_the_source_that_made_it` | ✓ |
| SK-04 | **A folder feed's reach is READ off the disk and never declared.** There is no vendor constant, no history floor and no fallback: `Reach` is computed by walking the folder, and the three answers stay three — `Empty` (there and holding nothing), `Blank` (files delivered, no rows) and `Days` (the real span). There is deliberately no fourth arm meaning *unknown*, because a caller would have to treat it as unbounded | `pull::folder::a_folder_feeds_reach_comes_from_the_folder_and_never_from_a_floor` · `pull::folder::an_empty_folder_has_an_empty_reach_and_says_so` · `pull::folder::a_folder_of_blank_files_is_not_the_same_as_an_empty_folder` | ✓ |
| SK-05 | **A missing or unreadable folder HALTS LOUDLY and names the path — never an empty reach.** `CLAUDE.md` §4 bans the fallback that hides a failure, and this is the one it was hiding. Over the wire the distinction is the status code: an empty folder is **200** with `"state":"empty"`, and a folder that cannot be walked is **409** carrying `path`. A member that will not decode, and a row whose timestamp is not a moment on this calendar, refuse the WHOLE reach rather than skipping — a reach computed from the rows that happened to parse is a narrower answer wearing a complete one's words | `pull::folder::a_missing_folder_halts_loudly_and_names_the_path` · `pull::folder::an_unreadable_folder_halts_loudly_and_names_the_path` · `pull::folder::a_member_that_will_not_decode_refuses_the_reach_by_name` · `pull::folder::a_row_whose_timestamp_is_not_a_moment_refuses_the_whole_reach` · `api::folder::a_missing_folder_halts_loudly_and_names_the_path` · `api::folder::an_empty_folder_answers_empty_with_the_path_and_never_a_halt` | ✓ |
| SK-06 | **There is no default folder, and the root resolves exactly as the store and masters roots do.** `BRUTEX_ARCHIVES`, else `$HOME/.brutex/vendor-data`, read in ONE place, with no literal path in any tracked file. With neither set it REFUSES where the two siblings fall back to `.` — and the deviation is deliberate: a missing master renders `UNAVAILABLE` and a missing store is created on write, but a folder feed's whole reach IS its folder, so the working directory would report an honest, precise and completely wrong reach, most often `Empty`, which is indistinguishable from "not bought yet" | `pull::folder::the_root_is_the_variable_then_the_home_directory` · `pull::folder::with_no_variable_and_no_home_there_is_no_default_folder` · `pull::folder::the_environment_reader_and_the_pure_resolver_agree` | ✓ |
| SK-07 | **A REST feed has no folder and is refused BY NAME, never given one.** A path invented for a broker would come back as "that folder is missing" — the same words as a real missing folder, for a completely different reason. The refusal names the kind and the verb the feed's bars actually arrive under | `pull::folder::a_rest_feed_cannot_be_asked_to_read_a_folder` · `api::folder::a_rest_feed_is_refused_by_name_and_never_given_a_path` | ✓ |
| SK-08 | **Timestamps are keyed to the SECOND, so two rows sharing a second is expected input — and LAST WINS for the price.** Measured in `docs/08-vendor-samples.md`: three rows in one second at TrueData, four at GDFL, no sub-second field and no tiebreaker at either. Refusing them would refuse every file the operator bought. Folded at `Bucket::SECOND` the second's rows become one record — first in file order is the open, the extremes are the high and low, the volumes sum, and the **last in file order is the close**. Nothing is dropped and nothing is sorted: rows sharing a second carry no tiebreaker, so a sort would invent one and quietly change which price became the open. This is what makes the read idempotent under §3 rule 5 — the same folder yields the same bars, byte for byte | `pull::folder::a_shared_second_folds_last_price_wins` | ✓ |
| SK-09 | **The column shape a decoder is handed comes from the descriptor, and its two spellings cannot drift.** `ColumnLayout::shape` is the `Columns` variant for that `(feed, segment)`; a `const` block holds it against the layout's own column list on all three things that silently mis-read a row — the count, the header row and the date format. A layout whose two spellings disagree fails the build. An unmeasured segment refuses by name rather than borrowing the other vendor's shape | `api::folder::the_shape_is_the_descriptors_and_a_single_layout_needs_no_segment` · `api::folder::an_unmeasured_segment_refuses_by_name_and_lists_what_was_measured` · `pull::vendor` `shape_agrees` `const` block | ✓ |

**What is NOT held here, and is named rather than implied.** `api::server::run_local`
still hardcodes `Columns::Gdfl` and `Segment::Fno` for every archive feed, so a
TrueData folder driven through the ingest form is decoded against GDFL's ten
columns. The measured shape now exists to read (SK-09) and `/folder.json` reads
it, but that call site cannot take it without also deciding the segment — and
the segment decides where bars are FILED, which append-only history makes
unrenameable. Recorded in `docs/06-limits.md`; it is a `crates/api` change these
rows do not own.

## Five silent answers, and what now separates them — D-0124

Each of these was a degradation that reached the browser wearing a success's
clothes: an empty array, a zero, an idle phase, a green badge. In every case the
information that would have separated the failure from the ordinary state was
present in the process and discarded at the last step before the wire —
`Census::Unreadable`'s reason, `Read::notes`, `Read::status()`, the work list's
own length. `CLAUDE.md` §4: degrade loudly and name the reason, or refuse.

**The body shape did not move.** `/store.json` and `/instruments.json` answer a
JSON array in every state, before and after, because a consumer expecting an
array must not start receiving an object — that would be the very failure being
fixed, re-introduced by the fix. The reason travels beside the payload, in
headers, and the two states where every number in the body is absent rather than
measured also move the status code.

| # | Invariant | Proof | |
|---|---|---|---|
| SF-01 | **A damaged counter and an empty store are not the same response.** They used to be equal byte for byte — `200`, `content-type: application/json`, body `[]` — so `/db` called a store that may hold millions of bars empty and Markets asserted "holds no bars". `/store.json` now answers `503` when `census::read_vendor` refused the manifest, and carries `x-brutex-census-state` (`held`·`absent`·`unreadable`, `Census::name`'s own words, which are `/audit.json`'s), `x-brutex-census-note` (the refusal verbatim, naming the file) and `x-brutex-census-degraded`. **The body is still `[]`, and a fresh install with no manifest is still `200`** — `absent` and `unreadable` are two states, not one | `api::server::a_corrupt_census_is_not_answered_as_an_empty_store` · `api::server::a_missing_census_row_is_absent_and_says_so` | ✓ |
| SF-02 | **`/instruments.json` says when its zeroes are not measurements.** An unreadable census makes `rows_for` answer `None` for every key, so every row read `"bars":0` — the same em dash an un-pulled instrument shows — at `200` with no status. It now answers `503` with the same three census headers, and the body is unchanged | `api::server::instruments_json_says_when_its_zeroes_are_not_measurements` | ✓ |
| SF-03 | **A feed's master is answered for PER FEED, in three states, because a whole-read boolean cannot.** `Read::unread` records which vendor was never read and why; `unavailable` is derived from it rather than set beside it, so the boolean and the list cannot disagree. `Read::master` answers `read`, `UNAVAILABLE` (this build expects the file and could not read it — the list is absent, not empty) or `not-mastered` (an archive feed publishes no scrip file, so `[]` is the **true** answer and the status stays `200`). The failing feed answers `503` with `x-brutex-master-state`, `x-brutex-master-note` and `x-brutex-universe-status`; **the other feed on the same site answers `200` and complete** | `api::server::instruments_json_says_when_its_zeroes_are_not_measurements` · `api::server::an_archive_feed_names_itself_unmastered_rather_than_empty` | ✓ |
| SF-04 | **A routine disagreement does not refuse the route.** `Read::status()` is `DEGRADED` for any ISIN or eligibility conflict, and the list and the counts are both real in that state. Only an unreadable census or an unread master moves the status — refusing the type-ahead for a conflict would take the console down for a fact the header already carries | `api::server::instruments_json_says_when_its_zeroes_are_not_measurements` (the `?feed=dhan` half) | ✓ |
| SF-05 | **A note can never be lost to the alphabet a header admits, and can never open a second header.** The note carries a filesystem path — arbitrary bytes on this platform — and a vendor's own refusal text, already observed to hold `·` and `—`. `note_alphabet` maps everything outside `0x20..=0x7E` to `?` and bounds the value, so the conversion has no failure arm to be a coverage hole; a `unwrap_or(<empty>)` would have answered a corrupt census with a BLANK reason, which is this whole change arriving one layer down | `api::server::a_hostile_note_is_still_a_header` | ✓ |
| SF-06 | **The completeness claim cannot be built without the universe that justifies it.** `Settled::Complete` holds a `NonZeroUsize` and `Settled::over` is the only constructor, so there is **no code path** from an empty work list to the words "the store is complete" — which is what was published, at phase `idle`, once a minute, over a store holding nothing, whenever the masters failed to load. `NoUniverse` publishes `Phase::Halted` (the masters are read once at startup, so idling is a countdown to an event that cannot occur) and carries `Read::status()` and the read's own `UNAVAILABLE` notes, which name the file. A guard beside the `format!` was rejected: it fixes today's path and leaves the shape, and the next arm added gets the defect back | `api::autopilot::the_completeness_claim_cannot_be_built_without_the_universe_behind_it` · `api::autopilot::an_empty_universe_is_published_as_halted_and_never_as_complete` | ✓ |
| SF-07 | **A stalled month is still reconsidered whatever the universe looks like now.** A stall is a month that WAS asked for and did not land, so it is real work; gating it on `Settled` would trade one silent state for another, and `carry_on` outranks the halt because a task about to ask for a month is not halted | `api::autopilot::a_round_with_nothing_missing_reconsiders_a_stalled_month_and_says_which_attempt` | ✓ |
| SF-08 | **A relative fallback for the store root is a REFUSAL, not a default.** `store_dir_from(None, None)` returned `PathBuf::from(".")` beside a test that asserted it under the words *"no HOME is a broken environment, not a supported one"* — the sentence and the value said opposite things. `.` is the working directory, which for this launcher is the repository checkout, so the first append built `bars/`, `manifest/` and `audit/` inside the git tree, invisible to CI gate 1 because it walks `git ls-files`. Both resolvers now refuse, naming both variables, the directory the old fallback would have used and what it would have created; the serve path refuses **before the listener is served** and exits `FAILED`. An **explicit** `BRUTEX_STORE` is still honoured as given, relative included — that is a stated choice, and refusing a choice is not the same act as inventing one | `api::server::the_store_root_comes_from_the_environment_or_defaults_under_home` · `api::server::the_masters_directory_comes_from_the_environment_or_defaults_under_home` · `api::server::a_serve_with_no_store_root_refuses_instead_of_serving_the_checkout` | ✓ |
| SF-09 | **Every `/pull/spot` answer names itself a receipt, and no other route does.** The page parses any `200` HTML for a `.badge` and falls open to `verdict: 'OK', good: true` when there is none, so a request that never reached this process rendered a green OK with a blank reason. A content type cannot separate them — a receipt legitimately is `text/html` — so `x-brutex-receipt: pull-spot` is stamped on **all three arms**: the seat-conflict `409`, the malformed-form refusal and the completed run. `/dashboard`, another `200` carrying HTML, does not carry it, which is what makes requiring it a discriminator | `api::server::every_spot_answer_names_itself_a_receipt_and_no_other_route_does` | ✓ |

**What is NOT held here, and is a `web/` change rather than a crate one.** The
receipt marker is on the wire and nothing reads it yet:
`web/src/routes/ingest/+page.svelte` still calls `readReceipt(await r.text(),
r.ok, r.status)` with no header test, so the fail-open parse stands until that
page requires `x-brutex-receipt === 'pull-spot'` before parsing. The same is
true of the four census and master headers — `/db` and Markets can now see why
an array is empty and do not look. `web/src` was held by another session while
these landed. Three of the five also reach those pages as a status code they
already branch on, which is the floor rather than the fix.

## The granularity floor and the source kind cross the wire once — D-0126

`pull::vendor::GranularityFloor` existed on every `Descriptor` (D-0118) and
`/feeds.json` did not emit it, so `web/src/routes/ingest/+page.svelte` carried a
**transcription of all four consts** — `finest`, `kind`, `because` and `source`,
copied word for word — and every sentence the rung control printed was built
from the copy. Two copies of one vendor fact can disagree, and the browser's is
the one that would be wrong: it is versioned with that file, and nothing
rebuilds it when a const is reworded.

The same endpoint carried the SOURCE KIND twice: a `transport` word (`broker` /
`archive`) minted by a `match` in the handler, and a `kind` word (`rest` /
`folder`) read from `SourceKind`. The page then wrote
`active?.kind ?? (active?.transport === 'archive' ? 'folder' : 'rest')` — a
reconstruction of the fact standing behind the fact, looking exactly as
authoritative as it.

The rows are prefixed `GW-` for the reason `GF-` and `SK-` give: two sessions
share this tree and a collided identifier is worse than a new prefix. `GF-*`
holds the fact inside `crates/pull`; these hold what survives the wire.

| # | Must hold | Proven by | |
|---|---|---|---|
| GW-01 | **The whole granularity floor is emitted, not the number.** Every feed row carries `finest` with the rung, the kind's word, the kind's own sentence, `tick_stream`, `conflated`, the vendor's reason and its citation. The reason and the source are asserted to be the descriptor's own strings rather than a shape, so a reworded const fails here instead of leaving a browser showing yesterday's reason | `api::server::feeds_json_carries_the_granularity_floor_and_never_calls_a_snapshot_a_tick` | ✓ |
| GW-02 | **A conflated snapshot never crosses the wire as a tick.** The tick-versus-conflated distinction travels as `tick_stream` and `conflated` — `FinestKind::is_tick_stream` and `is_conflated` answered on the server — beside the word, so no reader has to match on a string to decide what may be printed next to a number. No row carries `tick_stream: true`, and that is asserted as a **field**, not as an omission: a page must be able to see the negative stated. Both one-second rows carry `conflated: true`, both one-minute rows `false` | `api::server::feeds_json_carries_the_granularity_floor_and_never_calls_a_snapshot_a_tick` | ✓ |
| GW-03 | **`FinestKind::Tick` has a wire word although no descriptor constructs one.** The word lives in its own `const fn` rather than as an arm inside the emitter: an arm nothing reaches is a region that can never run, which is the coverage hole `CLAUDE.md` §9 cannot forgive, and the negative is the sentence that has to survive. All three words are asserted, and the snapshot word is asserted **not** to be the tick word | `api::server::finest_kind_words_are_three_and_the_tick_word_is_one_of_them` | ✓ |
| GW-04 | **The source kind is on the wire ONCE.** `transport` is gone — asserted absent from the body rather than merely unused — and `kind`, `kind_label` and `verb` are all `SourceKind`'s, reached through one `Feed::source_kind` per row. The readiness rule that used to be a third `match` on the transport now asks `needs_credential`, which is its actual reason: a credential is what proves a broker's entitlement, so an empty store means "nothing pulled yet" for REST and "not bought" for a folder | `api::server::feeds_json_carries_a_floor_per_rung_with_the_source_that_made_it` | ✓ |
| GW-05 | **The no-JS page labels a feed with `SourceKind`'s word too.** `render::feed_select` minted a fourth vocabulary for the same split off its own `match` on `Transport`. It now prints `SourceKind::label`, and the words it used to coin are asserted absent — so a page, a receipt and a refusal cannot call one feed three things. Only the clause about which of the form's other fields mean anything stays local, because that is a property of the page | `api::render::the_pull_page_offers_every_feed_in_the_table` | ✓ |

**What is NOT held here, and is a `web/` fact rather than a crate one.** The
browser now reads both fields and holds no copy of either: the four-row `FINEST`
table, the `KIND_LABEL` map and the `transport`-derived guess are deleted from
`web/src/routes/ingest/+page.svelte`, and a row that arrives without `finest` or
without `kind` is reported as a **missing field on a server that predates it**
rather than answered from a fallback. Nothing above proves that — these rows are
about `crates/api`, and there is no test harness in this repository that renders
that page.

**And one second copy is still standing.** The HISTORY floor — `FLOOR_OPERATOR`
and `FLOOR_DOC` in the same file — is the same defect one layer over, and it is
NOT fixed by this change. `/feeds.json` has emitted `history` per rung since
D-0113 and `feedFloor` does not read it. The drift is already visible in the
copy: `FLOOR_OPERATOR` carries a `zerodha` row for a feed no descriptor in this
build names, and neither table has a row for `truedata` or `gdfl`, which the
wire answers for.

## What a silent success looked like on the startup path and on two pages — D-0129

Nine findings from two adversarial sweeps, every one of them REPRODUCED before
it was fixed. They share one shape and it is the shape `CLAUDE.md` §4 names: a
value that was measured, discarded at the last step, and replaced by a default
that reads as success — `0` for an exit code, `true` for a build, `full` for a
month, `100.0%` for a meter, `OK` for a receipt, `_ignored` for a write.

The runs quoted in the `Proof` column are in `docs/05-decisions.md` D-0129.

| # | Invariant | Proof | |
|---|---|---|---|
| SS-01 | **A serve that ran its whole life over a `DEGRADED` universe exits `3`, and the banner says so on the way in.** The masters are read one line above the banner and the verdict was discarded: eight `println!`s and not one named the universe, the `api.serve listening` event carried five fields and not that one, and `Ok(())` from the graceful shutdown mapped to `OK` — which `api::main`'s `exit_note` prints as *"everything went as asked"* over a session `/health` had been answering `503` for throughout. D-0026 decided this for `report` and `tests/binary.rs` asserts it; this is the same rule on the path that actually runs. A server that FELL OVER is still `FAILED`: a universe verdict does not outrank a stopped process | `api::server::a_serve_over_a_degraded_universe_exits_non_zero_rather_than_claiming_success` · `api::server::run_serves_until_the_signal_and_exits_zero` (the exit code is computed from `report`'s own verdict, so the test asserts the code and not the machine it runs on) | ✓ |
| SS-02 | **`built()` is not a verdict, and the banner no longer prints one from it.** `Assets::new` was `canonicalize().ok().filter(is_dir)` and `built()` was `root.is_some()`, so an empty `build/`, a half-written one and a bundle older than the sources beside it all printed `serving` — while every page answered `503`. This repository's own green test asserts `built()` for a **completely empty** directory one screen from a `503` assertion. `Build` has four answers — `Missing`, `NoShell`, `Stale { newer, by_secs }`, `Serving` — each naming the fix, and `Build::Stale` is a comparison of two measured mtimes, never a guess: with no `web/src` to compare against, nothing is claimed in either direction | `api::assets::a_build_directory_that_exists_is_not_the_same_as_a_build` | ✓ |
| SS-03 | **Two servers over one store are refused, and the refusal names the other one.** The only guard was the bind, which is honest for a byte-identical address and nothing else — measured on this machine, `127.0.0.1:8080` and `0.0.0.0:8080` both bind, in either order, and any other port is not even a collision. Both processes then spawn `autopilot::fly` against one `BRUTEX_STORE` and spend one shared vendor token's quota twice; the file locks that do exist are taken AFTER the vendor has been paid. The lock's subject is the STORE, and it is held by the OS on an open file description, so a killed process never wedges the next start. A second serve inside ONE process is allowed and documented: that is this suite, and an advisory lock would refuse itself | `api::server::a_second_server_over_one_store_is_refused_and_the_reason_reaches_the_operator` | ✓ |
| SS-04 | **A refusal that could not be journalled says so on its own receipt.** Nine accepted paths named the journal's answer through `recorded_fact`, under a doc comment citing §4; the three refusal paths were `let _ignored = journal.append(&record);`. On a read-only store root, a full disk, or an `audit` path that is a file, the refusal answered a page saying nothing about the journal and `/audit` showed no such refusal ever happening — while `/autopilot` tells the operator that a failure missing from `/audit` "was never written down". The append happens inside `refused_and_recorded` now, so recording a refusal and answering with a page that omits the result is not expressible | `api::server::a_refusal_that_never_reached_the_journal_says_so_on_the_receipt` | ✓ |
| SS-05 | **A tick whose record cannot be journalled carries the reason to `/autopilot.json`.** `tick` builds exactly one `Record` per pass and appended it as `let _ignored`. The payload carried `journal` — the PATH — and a path is not a proof that anything reached it. `TickOutcome::journal_error` travels to `Status::journal_error` and onto the wire, and it is CLEARED by a tick whose record lands, so the page shows the file's current state rather than the worst this process ever saw | `api::autopilot::a_tick_that_cannot_be_journalled_carries_the_reason_to_the_page` | ✓ |
| SS-06 | **The bar count per instrument on `/instruments.json` is ONE pass over the census.** The fold was a closure scanning the whole entry vector per symbol, and it was both the `sort_unstable_by_key` key and the emitted field — ~22.6 scans per row at n=785 plus one per emitted row, so the request cost the PRODUCT of the universe and the census for a response whose size never moves (measured, optimised, no disk I/O: 750 entries 15.5 ms, 9,000 165.9 ms, 43,422 819.2 ms, body flat at ~25 KB, on a route every page in the front end loads). The test asserts the PASS COUNT, not a duration: the iterator counts what it yields | `api::server::instrument_bar_counts_are_one_pass_over_the_census` | ✓ |

### The three that are `web/`, and the test harness that now exists for them

`docs/04-invariants.md` has said twice that "there is no test harness in this
repository that renders that page". There still is not — and there is now one
for the **arithmetic**, which is where all three of these lived.
`web/tests/*.test.js` runs under `node --test` (node's own runner: no
dependency, no config, no toolchain) against the same modules the pages import.
`npm test` in `web/`. CI does not run it, deliberately — gate 2 shadows `node`,
`npm` and five bundlers to prove `cargo build` needs none of them, and adding a
Node job to run these would be a larger change than this batch earns. They are
run by hand and they fail loudly when the arithmetic moves.

| # | Invariant | Proof | |
|---|---|---|---|
| SS-07 | **A month's completeness reads the same whatever order the rows arrive in.** `/db`'s month card took `fullest` from whichever row of the month `/store.json` sent first — out of a map correctly keyed on (month, RUNG), under a comment forbidding exactly that — and multiplied it by the row count. Measured on 2021-08 holding NIFTY@1day=23, NIFTY@1min=8,625 and BANKNIFTY@1min=8,625, a month complete at both rungs: `25,033.33%` with the title *"every one of the 3 instruments holds all 23 bars"* if the daily row came first, `66.76%` and tagged SHORT if the minute row did. `owed` is now the sum of each row's OWN (month, rung) denominator, and `fullest` survives only where one rung makes it a single number | `web/tests/completeness.test.js` · *a month held at two rungs reads the same whichever row arrives first* | ✓ |
| SS-08 | **A row that is its own denominator is `unverified`, never `full`.** The denominator is the fullest row at the same (month, rung); where that group holds ONE row, "short by zero" is a tautology. A store holding `2026-08 1min = 1 bar` reported `Complete 1 of 1 · Bars missing 0 · Coverage 100.00%` with a green chip and a filled meter — and a whole store of one bar reported the same. Such rows are counted in neither direction: `pct` is `null`, no meter is drawn, `Complete` excludes them and says how many, and Coverage's numerator and denominator both drop them. The corroborated cases are untouched — two instruments at one rung, 8,250 against 1, is still a `gap` at 0.01% | `web/tests/completeness.test.js` · *a month whose only row is short is not full* · *a store holding one bar does not report itself complete* · *two instruments in one month at one rung are still judged against each other* | ✓ |
| SS-09 | **`/ingest`'s progress meter has one population under both halves of its ratio.** The fold counted every row in the window months — every instrument, every rung — while `expectedUnits` counted only the request's reach, so with anything else already held in those months the numerator was in the hundreds against a denominator of 2, `Math.min(1, …)` pinned the bar at `100.0%` and `unitsLeft` clamped to `0`, which silently removed the ETA line, with half the request still on the wire. Two pre-existing rows were enough. Both readings go through `askScope` now, and what falls outside it is COUNTED (`outside`) and named beside the meter rather than credited or hidden | `web/tests/fold.test.js` · *the ask sees only what the ask reaches* · *the rung is part of the ask* | ✓ |
| SS-10 | **A `200` with no `x-brutex-receipt: pull-spot` is not a receipt.** D-0124 put that marker on all three arms of `pull_spot` and recorded that the browser half was not done, because `web/src` was held by another session. It is done: `notAReceipt` refuses on the marker first, then on the content type, then on the absence of the verdict element — and `readReceipt` can no longer fall open to `verdict: 'OK', good: true` over a request no server ever saw | `web/tests/receipt.test.js` · *a 200 with no receipt marker is refused, whatever it carries* | ✓ |

## The notes a page draws are prepared once, not re-read per request — D-0130

`NP-*` because `C-18` … `C-25` are retired and never reused (D-0045), and
because this is a **correctness** row, not a complexity one: the complexity
statement already exists as C-15 and is asserted by
`crates/api/benches/ratio.rs`. What is new is the thing preparing the notes
could have broken silently, and did not.

| # | Invariant | Proof | |
|---|---|---|---|
| NP-01 | **A note is loud on a page iff its FULL text carries a loud word, however far past the 160-byte display clamp that word sits.** `render::Notes` decides each line's display string and its loudness once, where the notes are built, because the renderer used to ask both questions on every request over text that grows with the universe — five substring searches per line, twice over, plus `clamp` counting the separators in the tail, which is the whole of gate 8's thirty C-15 breaches. Deriving loudness from the CLAMPED head instead would be cheaper still and wrong: a loud word past the cut would stop being loud, a red line would render grey and the summary would count it as routine, which is the silent downgrade `CLAUDE.md` §4 forbids. The test builds a note whose `UNCHECKED` sits past the cut, **asserts the fixture really is past the cut** — without that the test would pass against the broken derivation — and then asserts the line is still counted and still classed | `api::render::a_prepared_note_keeps_the_loudness_of_its_whole_text_and_draws_only_its_head` | ✓ |
| NP-02 | **Two prepared sets join without either being re-read, and the loud tally is the sum.** `/store` draws its own lines beside the universe's and got the second set by cloning the raw strings — a per-request copy of a note whose length is the universe. `Notes::extend_from` appends the PREPARED lines, which are bounded at 160 bytes each. Dropping the other set's loud count would leave the summary saying "0 needing attention" above a red line, which is the same defect as never marking it, so the sum is asserted rather than the append alone | `api::render::a_prepared_note_keeps_the_loudness_of_its_whole_text_and_draws_only_its_head` (the `extend_from` half) | ✓ |

**What this pair does NOT claim.** That a note's length is bounded. It is not —
`docs/06-limits.md` §67 records that `coverage::Coverage::notes` grows with the
universe, that `/health` still writes every byte of it on every poll, and that
nothing in CI would fail if a third note started growing the same way.

## A stored price is never negative — D-0143

At every boundary a price crosses, it is `>= 0`.

`Bar::ohlc_is_sane` (`crates/store/src/format.rs`) is the binding one, because
`BarFile::append` calls `survey` before it writes a byte and every ingest path
— vendor JSON and CSV archive alike — reaches `append`. `http::one_price`
(`crates/pull/src/http.rs`) refuses earlier, where the vendor's original value
is still in hand and can be named in the refusal.

**Why the ordering check was not already this.** The three clauses of
`ohlc_is_sane` compare the four prices to EACH OTHER. `-100/-100/-100/-100`
satisfies all three, so a fully negative bar was appended, checksummed, counted
and recorded as good. §3 rule 8 makes that month permanent.

**Proved by** `negative_prices_are_not_sane_however_well_ordered` in
`crates/store/tests/unit.rs` and
`a_negative_price_is_refused_where_the_vendor_value_can_still_be_named` in
`crates/pull/src/http.rs`.

## A stored count is never negative, except the OI sentinel — D-0148

`volume >= 0` always, and `open_interest` is either `OI_NULL` or `>= 0`.

Enforced by `Bar::counts_are_sane` (`crates/store/src/format.rs`), called by
`survey` in `crates/store/src/file.rs` — the same all-or-nothing gate that
carries the price rule, so a bad batch leaves the month exactly as it was.

**Why the ordering check and the price rule did not already cover it.**
`ohlc_is_sane` is named for, and only examines, the four prices. A bar with a
well-ordered non-negative OHLC and `volume: -1` satisfied every question the
store asked before this.

**Proved by** `a_negative_count_is_refused_and_the_oi_sentinel_is_not` in
`crates/store/tests/unit.rs`, which also pins `OI_NULL == i64::MIN` and refuses
`i64::MIN + 1`.


## The walk-forward's own guarantees — D-0156, D-0157

Five properties added while fixing the selection defects. Each is here because
`CLAUDE.md` §9 asks for the invariant beside the test that proves it, and
because every one of them was, at some point this session, *believed* rather
than checked.

| Id | Invariant | Proven by | ✓ |
|---|---|---|---|
| R-01 | **Every candidate the sweep produced is priced.** `FoldResult::priced` counts what the ranking loop visited and must equal `considered`. A prefix cap of any size breaks the equality the moment it truncates — measured with `.take(N)` restored on a 9,299-candidate fold: N=512 dropped 8,787, N=5,000 dropped 4,299, N=9,000 dropped 299. The value-comparison test it replaced fired only below ~1,683 and passed at the shipped 20,000, so it tested a constant rather than a property | `runner::validate::every_candidate_the_sweep_produced_is_priced_and_none_is_skipped` | ✓ |
| R-02 | **A fold's out-of-sample walk sees no training bar.** `restricted` blanks every row whose source is before the cut, and the check is against `ConditionMask::ZERO` rather than against a mask — an empty mask hits everything, `(bits & 0) == 0`, so asking whether a blanked row "hits" one is a question whose answer is always yes. Mutating `clear_before(from)` to `clear_before(0)` leaves 1,312 live rows before bar 3,188 | `runner::validate::the_out_of_sample_walk_cannot_see_a_single_training_bar` | ✓ |
| R-03 | **The short direction is exercised, and differs from the long one.** No test ran `walk_forward` short before this, so a `panic!` planted in `side_of`'s `Short` arm survived the whole suite — half the execution model was never entered. The test also requires the two directions to disagree somewhere, so a direction accepted and then ignored fails it | `runner::validate::a_short_walk_forward_runs_and_is_not_the_long_one` | ✓ |
| R-04 | **The stepdown threshold never rises as the surviving set shrinks.** Each Romano–Wolf round takes the bootstrap maximum over fewer strategies, so the bar must be non-increasing; that monotonicity is why a strategy masked by a stronger one can clear later. Drawing fresh indices per round broke it — measured 32.536 → 33.906 at seed 97, rising in 19 of 400 configurations, 0 of 400 with one resample matrix held across the stepdown | `runner::bootstrap::the_stepdown_threshold_never_rises_as_the_surviving_set_shrinks` | ✓ |
| R-05 | **A grid holds exactly `(stops+1)(targets+1)(1 + trails·(targets+1))` cells.** The count lived in four places and two were wrong: `evaluate`'s doc said `(rungs+1)^2`, the module header said `400 variants`, `validate::DEFAULT_RUNGS` said 125, and the reservation asked for 25 before pushing 125. One `grid::variants` function is now the only place it is computed. **The formula changed shape** when the arming axis landed — see R-06 for why it is not a fourth factor | `runner::grid::the_cell_count_is_the_product_of_all_three_ladders_and_nothing_reserves_less` and `runner::grid::arming_tests::the_grid_never_pairs_an_arm_with_no_trail` | ✓ |
| R-06 | **No cell pairs an arm with no trail, so the grid is 525 wide and not 625.** `Cell::arm` indexes the target ladder, so a naive fourth factor reads `(S+1)(T+1)(R+1)(T+1)`. The 100 cells arming nothing cannot differ from the trail-less cell they duplicate, and a grid reporting one answer a hundred times has learned nothing a hundred times. The test also pins that no two cells carry the same setting, which is the defect class that let the trails factor go missing from the reservation before D-0208, and that `Grid::baseline` still matches exactly one row | `runner::grid::arming_tests::the_grid_never_pairs_an_arm_with_no_trail` and `..::exactly_one_cell_is_the_baseline` | ✓ |
| R-07 | **A give-back made before arming cannot fire an armed trail.** `Crossings::trailing` is the largest retreat since ENTRY; an armed trail needs the largest retreat since ARMING, and one is not a filtered view of the other. Measured on the fixture path — up 300 ppm, back 200, up to 800, back 100 — the since-entry retreat is 200 and the since-arming retreat is 100, so a 150 ppm rung fires the un-armed trail and never fires the armed one. Conflating them would fire a trailing take profit for a give-back the position never made | `runner::excursion::armed_tests::a_give_back_before_arming_cannot_fire_the_armed_trail` | ✓ |
| R-08 | **The arming bar never fires the trail it armed.** Arming is recorded after that bar's target cursor advances, so the first bar that can fire an armed trail is the next one. One minute both reaching the target and giving back the trail distance is the intra-bar ordering D-0212 refuses to assume for the plain trail, and assuming it here would be no better. Pinned with a bar that reaches the target AND falls five times the trail rung | `runner::excursion::armed_tests::the_arming_bar_itself_never_fires_the_trail_it_armed` | ✓ |
| R-09 | **An armed rung pair that does not exist reads as NEVER, not as a neighbour's.** The armed tables are row-major and flat, so an out-of-range trailing rung would wrap into the next arming row and answer for a different cell — silently, and with a plausible number. Both bounds are guarded and both are tested | `runner::excursion::armed_tests::an_out_of_range_rung_pair_answers_never_and_not_a_neighbour` | ✓ |

**What is NOT claimed here — and one sentence that stopped being true.**

This paragraph read: *"`FoldResult::out_of_sample` is a level-less walk … so the
exit chosen in sample cannot be applied to the test window. Selection became
joint under D-0157; the out-of-sample validation did not."* The first half still
holds and the second no longer does. `FoldResult::out_of_sample_exit` exists and
is populated by `grid::with_levels` using the **training** ladders, so the chosen
variant IS scored on the test window with rung values that no test bar decided.
`out_of_sample` beside it remains the level-less walk on purpose — the comparison
"with these levels versus without them" is the whole question the grid answers,
and losing the without-them number would lose it.

What is still not claimed: the per-variant exit decision is constant, but
`excursion::crossings` is not. It was `O(bars + rungs)` and the arming axis makes
it `O(bars·arm_rungs + arm_rungs·trail_rungs)`, four times the per-bar trailing
work at the shipped four rungs. That is **stated and not measured** — no bench
row covers this crate's grid, which `docs/06-limits.md` already records.

## A refused bar prices nothing — D-0243

The exact `Evaluator::step` acceptance bitmap is the only gate a raw execution
record may pass before supplying a price or path fact in `crates/runner`.
D-0243 originally applied `Candle::check` at the consumers, which closed the
four record-local variants but could not see the sequence-state variants
`TimestampNotIncreasing` and `AccumulatorTooLarge`. D-0421 supersedes that
narrow mechanism: the column records the evaluator's own verdict once, and
forward, trade and grid consumers share it without restating a predicate.

**What it cost while it did not hold.** One mis-assembled record among 7,500
synthetic bars: `forward(H=2).at(4688)` went `Some(-119)` → `Some(7_494_560)`,
thirty-two live positions shifted, `close_above_ema20`'s mean went −15.17 → +2,120.07
paisa and its `t` −10.464 → +0.993, and `trade::walk`'s best total went
**−14,700 → +79,930 — a change of sign**.

**Why nothing caught it.** `n` was unchanged, so `Edge::mismatched` stayed 0;
`Census::reconciles()` and `Trades::reconciles()` both stayed true. The only trace
was `refused: 1` against `swept: 5,623`. `map_or(0, ..)` was the defect and **zero
is a price**: a missing index and a corrupt record both read as "the close was 0".

**Proved by** `runner::outcome::refused_bar_tests::a_bar_the_run_refused_can_never_price_an_outcome`,
whose second half kills the mutant that refuses everything, and
`..::a_refused_bar_shrinks_the_sample_rather_than_moving_the_mean`, which holds
the property that makes this a loud degradation: the refusal costs sample size
rather than moving the mean. EB-01 through EB-04 below extend the same invariant
to both stateful variants and every downstream price/path consumer.

## The crossing family is derived and cannot outlive its bar — D-0244

**These rows were `V-01`…`V-05` and are `CX-01`…`CX-05`.** `V-` was already the
`crates/vocab` / evaluator prefix at the top of this file, so twelve ids in this
file each named two unrelated invariants and gate 27 was red — the exact
regression D-0215 recorded fixing. The prefix moved on THIS block rather than the
other one because the other one is cited: `crates/indicators/tests/invariants.rs`
implements `V-02`…`V-05` by name, and `docs/11-findings.md` cites `V-02` and
`V-03` as the crate's two order-safety proofs. These five were cited nowhere
outside this file, so renumbering them breaks no citation — which is the whole
test for which side of a collision may move. `CX` is crossings.

| Id | Invariant | Proven by | ✓ |
|---|---|---|---|
| CX-01 | **A crossing is set only when the state was clear on the previous bar of the same session and is set on this one.** `Evaluator::crossings_of` reads `previous_mask` and `vocab::table::CROSSINGS` and nothing else — no level, no formula, no module | `indicators::evaluator` emit tests; `only_live_positions_are_ever_emitted` caught the family's absence from `positions()` on its first run | ✓ |
| CX-02 | **No crossing survives a session boundary.** `previous_mask` is cleared in the rollover beside `seeded`, so no crossing bit fires on a session's first bar. An overnight change of side is a GAP, which the 132–142 family already describes, and this engine is intraday-only by §1 | the rollover sets `previous_mask = None`; `suffix_independence` and `two_runs_of_one_slice_agree_exactly` hold across it | ✓ |
| CX-03 | **`previous_mask` is `None` and never `ZERO`.** An all-clear mask is indistinguishable from a bar on which every level happened to be un-crossed, and against that every `close_above_X` set on the first bar would read as a fresh crossing. `docs/03-vocabulary.md` §4: an unknowable condition is unset, not guessed | the field's type; the early return in `crossings_of` | ✓ |
| CX-04 | **`CROSSINGS` names only live, two-sided levels.** Sixteen `close_above_` names are `void` by D-0080 and two more are the VWAP pair, dead on every runnable path because the only production `Evaluator::new` passes `Availability::Absent`. Seventeen remain | `vocab::table::CROSSINGS` has 17 entries; every index in it is `BitStatus::Live` | ✓ |
| CX-05 | **The append did not widen the mask, so `VOCAB_VERSION` holds at 3.** 280 → 314 against 384 bits. The assertion pins version, count AND the 70 remaining positions together, because the version alone cannot tell a reader how close the next family is to forcing a widen — and a widen re-keys every run ever recorded | `vocab::tests::the_version_is_the_widened_table` | ✓ |

**What is NOT claimed.** The relation space over the 88 levels the engine computes
is twelve deep — open, high, low and close each above and below, plus near,
touched, and crossed up and down: **1,056 positions**. With this family the engine
covers **298**. `touched` is not merely absent but unrepresentable — `set_near`
takes one scalar and `set_exact` takes none — so a two-sided test against a level
needs a new public function in `vocab::table`, not a new row.

## The exit report names what it counted, and names the row it chose — D-0253

**These rows were `W-01`…`W-07` and are `ER-01`…`ER-07`**, for the reason the
`CX` block above states: `W-` was already the `crates/api` asset-path prefix, so
these seven collided with it. They are cited nowhere outside this file. `ER` is
exit report.

| Id | Invariant | Proven by | ✓ |
|---|---|---|---|
| ER-01 | **Every round trip is charged to exactly one of five counters, and the counter is the order that closed it.** `stopped` is a fixed stop, `trailed_stop` a trailing STOP LOSS, `trailed_profit` a trailing TAKE PROFIT, plus `targeted` and `timed_out`. They sum to `trades` | `grid::every_trade_ends_by_exactly_one_of_the_five_exits`; `runner/tests/end_to_end.rs`'s five-way sum | ✓ |
| ER-02 | **A counter can only be moved by the order that owns it.** No fixed stop ⟹ `stopped == 0`; no TSL ⟹ `trailed_stop == 0`; no TTP ⟹ `trailed_profit == 0`; no target ⟹ `targeted == 0` — each with a witness cell that DID fire it, so no implication is vacuous | `grid::each_exit_counter_can_only_be_moved_by_the_order_that_owns_it`; mutant `Armed → trailed_stop` killed | ✓ |
| ER-03 | **A truncated walk-forward is countable.** `halted_folds()` is a filter over `FoldResult::halted`, and the audit prints the row whether it is zero or not — a run that degraded names the reason | `validate::a_truncated_walk_is_countable_and_a_complete_one_counts_zero`, which asserts BOTH a non-zero and a zero | ✓ |
| ER-04 | **The table's top variant row IS `Grid::best()`.** The sort key is `Reverse((pessimistic, merit(c), index))`, and descending on the index reproduces `max_by_key`'s last-maximum rule exactly | `audit::the_top_variant_row_is_the_cell_best_actually_returns` (ties on money and merit); `audit::the_top_row_prefers_the_simpler_of_two_variants_tied_on_money` (ties on money alone) | ✓ |
| ER-05 | **A selector's own row is never dropped by the display cut.** `keep` bounds the table; `best()` and `sharpest()` rank the whole grid, so their rows are printed below the cut when they fall past it | `audit::a_chosen_row_below_the_cut_is_printed_anyway`, at `keep = 1` | ✓ |
| ER-06 | **Every rung index in the `exit` column resolves to a ppm distance on the same page.** Three `ladder, … (ppm)` rows print `index=ppm`, and their labels do not share the count row's prefix | `audit::the_rung_ladders_are_printed_as_index_equals_ppm` | ✓ |
| ER-07 | **Every column heading ends flush at its field, and every value on every row has a space before it.** The widest value a column can hold still leaves a separator, so two numbers can never weld into one | `audit::every_exit_column_heading_ends_where_its_number_ends` — the assertion that caught `unknown` at ten digits in ten columns, and `ret/DD` at `i64::MAX` | ✓ |

**What is NOT claimed.** `return_over_drawdown` is measured and rendered and
**nothing ranks on it** — whether it or `edge_ratio` should decide the selection
is still the open question in `docs/06-limits.md`, and answering it by quietly
switching the key would be the defect this entry exists to remove, recreated one
layer down. The nineteen-field width spec lives in three places because
`writeln!` requires a literal format string; W-07 is what makes the third copy a
specification rather than a duplicate.

## The backtest console reads the ledger and never outranks it — D-0272

The results ledger is written by exactly one crate, `cli`, and read by two:
`cli` itself and `crates/api/src/backtest.rs`. A second reader is safe; a
second writer is how a format diverges. Everything below is about keeping the
reader honest about a file it does not own.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| BT-01 | **The reader's record layout is the writer's, checked at compile time.** `FIELD_SUM` adds twenty-four summands covering twenty-six fields in `Record::to_bytes`' own order, and a `const` assertion fails the BUILD if the total is not `PAYLOAD_BYTES`. A wrong stride still PARSES — every byte pattern is a legal value of its type — and yields a page of confident nonsense with no error anywhere | `backtest::the_stride_is_the_sum_of_the_fields_the_writer_writes`, and the `const _: () = assert!(FIELD_SUM == PAYLOAD_BYTES)` that precedes it | ✓ |
| BT-01a | **The reader's stride is PINNED to the writer's constant, and self-agreement is not agreement.** `const _: () = assert!(STRIDE_BYTES == cli::results::STRIDE_BYTES)` fails the build when `cli` changes the format. BT-01 alone cannot catch that: it proves the reader agrees with ITSELF, which it did through both drifts — 205 and 205 stayed equal while the writer moved to 213, then 213 and 213 stayed equal while it moved to 261 | `backtest::the_stride_is_the_sum_of_the_fields_the_writer_writes`' final line, and the `const` assertion itself | ✓ |
| BT-13 | **Both known versions are READ; only an unknown one is refused.** Refusing to read a released version is a different act from refusing to mutate it (§3 rule 8), and the only ledger on disk is a version-2 file `cli results` reads. A reader that refused it would disagree with the writer about a file they both open | `backtest::a_version_2_ledger_is_read_rather_than_refused`; `backtest::an_unknown_version_is_refused_rather_than_guessed_at` asserts the refusal names BOTH known versions | ✓ |
| BT-14 | **A version-2 record's seal is checked at 205 bytes, BEFORE it is widened.** Widening first would hash 253 bytes of which 48 are zeroes the reader invented, and every healthy version-2 record in existence would render as damaged | `backtest::a_version_2_record_keeps_its_seal_because_it_is_checked_at_its_own_length`; `backtest::a_damaged_version_2_record_is_still_caught` proves the check still rejects | ✓ |
| BT-15 | **Every address comes from the FILE's own version, never from the newest.** `stride_of(version)` feeds the count, the ragged-tail test and every seek. Reading a version-2 file at 261 is the same mid-record landing the unknown-version refusal exists to prevent | `backtest::a_version_2_ledger_is_read_rather_than_refused` (`total == 2` at 213); `backtest::a_ragged_version_2_tail_is_measured_against_213` | ✓ |
| BT-16 | **An absent mask and an empty mask are different answers.** A widened version-2 run and a version-3 run that recorded no combination both carry six zero words and the bytes cannot separate them, so `Ledger::version` travels with the rows and `has_mask` says which case the page is looking at. Rendering both as "no combination" is the fallback §4 bans | `backtest::a_version_2_run_reads_an_empty_mask_and_the_ledger_says_it_is_absent` | ✓ |
| BT-17 | **The mask reaches JSON as strings, because a JSON number cannot carry 64 bits.** A mask word is decoded into condition names, and a value above 2^53 parsed as an IEEE-754 double names DIFFERENT conditions without throwing. The fixture sets bit 63 on purpose so every round trip exercises it | `backtest::the_mask_survives_json_as_a_string_because_a_number_would_round` — asserts `"9223372036854775808"` exact | ✓ |
| BT-18 | **Version 3 APPENDED the mask; nothing before it moved.** That is what makes widening a copy rather than a re-layout, and it is asserted arithmetically rather than trusted | `backtest::the_mask_is_exactly_what_version_3_appended_to_version_2`; `backtest::a_version_2_record_decodes_every_other_field_exactly_as_version_3_does` compares every non-mask field across both versions; and `const _: () = assert!(PAYLOAD_BYTES_V2 + 8 * 6 == PAYLOAD_BYTES)` | ✓ |
| BT-02 | **A halted run is excluded from ranking, never merely ranked lower.** `Ledger::best_complete` filters `halted` before it compares, so a halted row with an enormous total does not win; `None` is the shape of `cli`'s `NO COMPLETE RUN` | `backtest::a_halted_run_is_never_crowned_however_large_its_total` (a halted row at 9,000,000 paisa loses to a complete one at 100); `backtest::no_complete_run_is_none_rather_than_a_fabricated_winner` | ✓ |
| BT-03 | **Ranking is on `pessimistic`, the worst-case fills.** Selection ranks on that figure everywhere in this workspace, and a page ranking on `optimistic` would name a different winner from the CLI reading the same file | `backtest::the_best_complete_run_is_the_highest_worst_case_total` | ✓ |
| BT-04 | **Runs are newest first by SEEKING, not by sorting.** The file is append-only so index order is recording order; the newest `limit` records are the last `limit` addresses, one seek each. O(limit), never O(total) | `backtest::runs_arrive_newest_first`; `backtest::a_limit_takes_the_newest_and_says_it_stopped` | ✓ |
| BT-05 | **A window is never presented as the whole ledger.** `total` is reported beside `scanned` and `hit_scan_cap` is set when they differ, so a capped read says so | `backtest::a_limit_takes_the_newest_and_says_it_stopped` — `total: 3`, `scanned: 2`, `hit_scan_cap: true` | ✓ |
| BT-06 | **Every way the file can be wrong produces a SENTENCE, and none produces a blank.** Missing file, short file, foreign magic, unknown version, unmeasurable length, unreadable header, unreadable record — seven refusals, each naming the path and what was not done | `backtest::a_missing_file_says_so_and_says_it_is_not_an_error`, `a_file_shorter_than_its_header_parses_nothing`, `a_foreign_file_is_refused_before_it_is_parsed`, `an_unknown_version_is_refused_rather_than_guessed_at`, `a_length_that_cannot_be_taken_refuses_with_the_path`, `an_unreadable_header_refuses_with_the_path`, `a_file_that_cannot_be_opened_at_all_names_the_reason` | ✓ |
| BT-07 | **"No runs yet" and "I cannot read the runs" are different sentences.** A `NotFound` open says the ledger is created by the first run `cli` records and that this is NOT an error; every other open failure says the opposite | `backtest::a_file_that_cannot_be_opened_at_all_names_the_reason` asserts the empty-ledger sentence is ABSENT from a real failure | ✓ |
| BT-08 | **A partial read keeps what it read.** A record that cannot be read returns the newer records above it plus the sentence naming where the read stopped — never all-or-nothing | `backtest::a_record_that_cannot_be_read_keeps_the_records_above_it` — one row kept, `total` still 3 | ✓ |
| BT-09 | **A ragged tail is reported and is not fatal.** Bytes past the last whole record mean an interrupted append; the whole records are served and `partial_tail` says the remainder was left alone. This crate never writes to that file | `backtest::a_ragged_tail_is_named_and_the_whole_records_are_still_served` | ✓ |
| BT-10 | **A `limit` is clamped, never refused.** A bookmarked `?limit=99999` is an operator who wants everything; the honest answer is everything up to the ceiling plus the flag saying the ceiling was reached | `backtest::a_limit_is_clamped_at_both_ends_and_never_refused` | ✓ |
| BT-11 | **An unresolvable store root answers 503 in the shape the page parses.** A configuration failure must not also be a parse failure on top of it | `backtest::an_unresolvable_store_root_answers_503_with_the_shape_the_page_parses` | ✓ |
| BT-12 | **`/backtest` is NOT a registered route, and that is what keeps one path one application.** A registered route beats `Router::fallback` unconditionally, so a Rust page there would make a click render Svelte and a reload render Rust — the live `/audit` defect `web/vite.config.js` documents | the router block in `server.rs` registers `/backtest.json` alone; `web/svelte.config.js`'s `SERVER_RENDERED` set does not name `/backtest` | ✓ |

---

## C-CLI — `crates/cli` re-measures the bounds it states

`crates/cli` was the **only** one of the thirteen workspace crates with no
`benches/` directory. Gate 14 refused it in those terms — *"REFUSED crates/cli —
20 cost claim(s), no row in the table"* — while every other crate shipped
`benches/ratio.rs`.

That hole was the worst one available. `CLAUDE.md` §5 makes `cli` the only entry
point from which the sweep is reachable at all: `api` depends on `core`, `pull`,
`store` and `telemetry`, not on `runner`, `engine` or `indicators`. So the binary
an operator actually runs to produce a trading result carried **no measured bound
of any kind**, and a regression to O(n) in the result store, in rung resolution
or in the report render would have breached no bench and tripped no gate.

The four rows are taken from the cost table `crates/cli/src/results.rs`'s own
header prints, because a bound a module states in writing is exactly the bound
gate 14 asks it to re-measure. Every row divides one per-unit cost by another
per-unit cost of the **same** operation; the ceiling is 2.500x, the figure
`crates/runner`'s bench uses and for the same reason.

| # | Must hold | Proven by | |
|---|---|---|---|
| C-CLI-01 | **Reading record *i* does not depend on *i*.** The ledger header says the address is `HEADER + i·STRIDE`, "an add and a multiply". Measured as record 0 against record 1,023 of the same 1,024-record file — if the seek had become a walk, the last record would cost proportionally more than the first | `cli::bench::the_read_does_not_depend_on_which_record` | ✓ |
| C-CLI-02 | **Counting does not depend on how many there are.** The header says `(file_len - HEADER) / STRIDE`, "no walk". Measured at 64 records against 1,024 — a sixteen-fold difference a count that walked could not hide inside the ceiling | `cli::bench::the_count_does_not_depend_on_how_many_there_are` | ✓ |
| C-CLI-03 | **The duplicate check is a hash probe, not a walk.** Timed on an ALREADY OPEN ledger, so the one pass that builds the set is excluded by construction — the header is explicit that the build is the O(runs) part and the query is the part that must not scan. A hit against a MISS, because a set that had silently become a linear search shows its worst case on the miss, which must reach the end before it can answer | `cli::bench::the_duplicate_check_does_not_scan_the_ledger` | ✓ |
| C-CLI-04 | **Encoding a record does not depend on the ledger it will join.** `to_bytes` writes one fixed-stride array from one struct; a `to_bytes` that consulted the store would stop being a pure function of the record it was called on | `cli::bench::the_encode_does_not_depend_on_the_ledger_it_joins` | ✓ |
| C-CLI-05 | **Encoding one record costs a bounded multiple of the cheapest instruction there is.** C-CLI-04 proves `to_bytes` does not depend on WHICH record; only an absolute row can say the constant is small. A quotient cannot see a uniform slowdown — a mask operation 174x slower once passed its crate's ratio rows at 0.98x–1.00x, and until this row `cli` was the only one of thirteen crates with no absolute floor at all. Measured 493.4–500.2 floors over thirteen runs against a floor of 520–522 ps; budget 2,000, 4.0x headroom on the worst observed. Every measurement was taken at load average 40–104 on 14 cores and is therefore an UPPER bound | `cli::bench::the_encode_stays_within_its_budget` | ✓ |
| C-CLI-06 | **The duplicate probe costs a bounded multiple of the same floor.** `CLAUDE.md` §3 rule 4 names duplicate rejection as one of five operations that must be O(1); `Results::holds` is this crate's spelling of it, and `results.rs`'s own doc says that bound is argued rather than timed. One SipHash of 32 bytes plus one `hashbrown` group probe. Measured 14.86–16.27 floors; budget 65, 4.0x headroom on the worst observed. Same load caveat as C-CLI-05 | `cli::bench::the_duplicate_check_stays_within_its_budget` | ✓ |

Measured on the operator's machine, 2026-08-25, `cargo bench -p cli`, exit 0:
0.989x, 1.000x, 0.880x and 1.006x. **Not measured here:** whether any of those
costs is *small*. These rows refuse a cost that GROWS; `docs/06-limits.md` is
where absolute figures and the things nobody has timed are recorded.

## A sweep that refused is not a sweep that finished — D-0295

`/backtest`'s Run control is the one place this console causes work rather than
reporting on it. Everything below is about the answer it gives when that work
does not happen. The shape is D-0129's — a value that was measured, discarded at
the last step, and replaced by a default that reads as success — and these are
the tenth and eleventh of that family, on a route that did not exist when D-0129
was written.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SW-01 | **Exactly one of `report` and `refusal` is set on any ended run.** `conduct` assigned `cli`'s answer to `report` for EVERY outcome and left `refusal` at `None` always, so a `range_all` that refused all nine rungs — an unknown feed word, a span the store holds no month of — reached the wire as `"refusal":null` with `"in_flight":false`. `settle` files the text by whether it opens with `refused`; the WORD and not the word with its colon, because `descend` opens one *"refused at the ceiling"* and a report opens with `STORED_PROVENANCE` and so cannot collide | `api::sweeprun::exactly_one_outcome_field_is_set_whatever_cli_returned` (five shapes including the empty string and both refusal spellings); `a_refused_sweep_is_a_refusal_and_never_a_report`; `a_real_report_is_a_report_and_never_a_refusal` | ✓ |
| SW-02 | **A refusal reaches the page as a refusal and a null report.** The field is what the browser reads to decide it failed, so the JSON is asserted directly rather than through the struct | `api::sweeprun::a_refusal_reaches_the_page_as_a_refusal_and_a_null_report` | ✓ |
| SW-03 | **A settled run is out of flight whichever way it ended.** `in_flight` is the page's only test of doneness, so a settle that left the stamp unset would poll for ever against a finished sweep — the same silent wedge in the opposite direction | `api::sweeprun::a_settled_run_is_no_longer_in_flight_whichever_way_it_ended` | ✓ |
| SW-04 | **A run that refused every rung is `failed`, not `done`.** `pollSweep` was `if (run.in_flight) … else done` — two readings of a payload carrying three — so a run that read no bar printed **"Sweep finished"** in green and re-read a ledger that had gained no row. `sweepOutcome` is total over the payload and the caller has no fall-through branch left to guess in | `web/tests/sweep.test.js` · *a run that refused every rung is failed, not finished* · *every payload the route can send lands in exactly one phase* | ✓ |
| SW-05 | **`in_flight` is read BEFORE `refusal`, and the order is load-bearing.** The slot keeps a finished run until the next replaces it, so a payload can carry a fresh `in_flight: true` beside a stale refusal. Reading the refusal first would report a new run as failed before it had done anything | `web/tests/sweep.test.js` · *a run still going is running, whatever else the slot holds* | ✓ |
| SW-06 | **An empty refusal string is not a refusal.** `json_string("")` is a legal payload, and painting a red block with no reason in it is worse than the green one it replaced | `web/tests/sweep.test.js` · *an empty refusal string is not a refusal* | ✓ |

**`report` was on the wire from the first commit of this route and nothing read
it.** Rendering it is not cosmetic: `range_all` prints `REFUSED: why` on the row
of any rung that refused while OTHERS succeeded — a partial run, with rows in
the ledger and no whole-command refusal to catch it — and which rungs never ran
was unknowable from this page.

**Not proved here, and not fixed here.** The run sweeps at `SUPPORT_PPM =
200_000`. `cli::elite_descend`'s own doc records that a run launched at 20,000
ppm *"cannot report a once-a-week setup no matter how long it runs"*; this route
launches at ten times that, and neither descent is reachable from any HTTP
route. See `docs/05-decisions.md` D-0295's closing paragraph.

## A run that cannot be recorded is refused before it is started — D-0296

`Results::append` refuses on a ledger version it does not write, and that
refusal is correct. These rows are about WHEN the operator learns it. Measured
on the operator's own store: a version-2 `runs.bin` against a build writing
version 3 cost nine rungs over 121 months of one-minute bars and recorded
nothing, for an answer that sat in sixteen header bytes the whole time.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SW-07 | **`/backtest.json` says whether a sweep could record, and names both versions.** `appendable` is the SERVER's answer — `self.version == VERSION && self.refusal.is_none()` — not a comparison the browser makes against a `3` of its own, which is the hand-kept second copy §5 refuses for the vocabulary and would go silently wrong the first time the format moved. `writes_version` travels so the banner can name both numbers rather than only that they disagree | `api::backtest::an_older_ledger_says_a_sweep_cannot_record_before_one_is_started` | ✓ |
| SW-08 | **A ledger the reader refused is never reported appendable.** `refusal` is set for a damaged header, an unknown version and a file that would not open. Answering `appendable: true` over any of those would send the operator to start a sweep against a file the reader has already given up on | `api::backtest::a_refused_ledger_is_never_appendable_however_its_version_reads` | ✓ |
| SW-09 | **Only an explicit `false` blocks the control, and a blocked ledger renders whatever it has.** An absent field is not evidence of a fault — a red banner over a payload that never made the claim is an alarm nobody can act on and nobody can clear. A blocked ledger missing its detail fields still renders: `null` is a state the page words around, `undefined` is a blank screen | `web/tests/sweep.test.js` · *an absent appendable field makes no claim in either direction* · *a blocked ledger still reports what it can when fields are missing* | ✓ |
| SW-10 | **One fault draws one banner.** `api::backtest` reports `appendable: false` for an unreadable ledger too, which is right, but the page already renders `refusal` on its own. `ledgerBlock` returns `null` in that case | `web/tests/sweep.test.js` · *a ledger that was already refused gets one banner, not two* | ✓ |

**Not pinned, and worth knowing.** `api::backtest::VERSION` is a hand-kept copy
of `cli::results::VERSION`. `STRIDE_BYTES` has a `const` assertion tying it to
`cli`'s (BT-01a, added after that constant drifted twice); this one has none,
because `cli::results::VERSION` is not `pub`. If `cli` bumps to version 4,
`appendable` reports against a stale 3 until someone notices.

## A run that cannot be stamped is refused before the slot — D-0297

`cli` already refuses to record a run from an unstamped build, correctly: §3
rule 3 puts `commit` in every run's identity and `option_env!` resolves at
compile time, so it cannot be filled in later. These rows are about the gate
sitting one layer too deep — inside `audit_range`, past the slot, past nine span
loads.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SW-11 | **An unstamped build refuses before a single bar is read, and the sentence names the variable.** Measured by running the command this route calls: `cli range-all zerodha NIFTY 2026 8 2026 8 200000` refused all eight rungs on exactly this cause, and the answer never depended on one bar it read. The refusal says outright that nothing was swept, which is the difference between it and the nine-rung refusal it replaces | `api::sweeprun::an_unstamped_build_refuses_before_a_single_bar_is_read` | ✓ |
| SW-12 | **It answers 503, never 400 and never 409.** The body is perfect and nothing is in flight; the fix is a rebuild and a restart on the server's side, which is what 503 says. A 400 would send the operator to correct a request that is already correct | `api::sweeprun::an_unstamped_build_is_503_and_never_a_bad_request` | ✓ |
| SW-13 | **Only one full lowercase SHA-1 spelling is a stamp.** Empty, abbreviated, non-hexadecimal, uppercase, whitespace-padded and newline-terminated values all refuse as 503 before engine work; a canonical value is the only admitted representation. `api` delegates to `cli`'s validator rather than owning a second spelling | `api::sweeprun::tests::only_a_canonical_stamped_build_does_not_refuse`; `cli::build_provenance::tests::only_full_lowercase_sha1_is_canonical` | ✓ |
| SW-14 | **The check runs BEFORE the slot is claimed.** An unstamped build refuses every run it could ever start, so claiming first would answer 409 `Busy` to a second press while the first was busy failing for a reason no wait can fix | `api::emitted` drives the busy row through `run_with(.., Some(stamp))` and the unstamped row through `run_with(.., None)`; both are in the census | ✓ |
| SW-15 | **`commit_stamped` and `appendable` are two facts, not one flag.** A ledger can be perfectly appendable and every sweep still refuse. The fixes differ — a `mv` against a rebuild and a restart — so an operator told only "you cannot record" would not know which to apply | `api::backtest::the_payload_says_whether_this_build_may_record_at_all` | ✓ |
| SW-16 | **Every refusal variant carries a sentence.** `why` matches all four; a variant added without an arm does not compile, and one added without a sentence returns an empty string here | `api::sweeprun::every_refusal_variant_carries_its_sentence` | ✓ |

**The state a default server runs in is now proved, not inferred from an
environment variable.** `.claude/launch.json` starts `cargo run --release -p
api -- serve` with no explicit stamp. A clean supported Git checkout receives
its verified HEAD automatically; a dirty, untracked, unsupported or otherwise
unprovable source tree receives the empty refusal sentinel and every browser
sweep stops here. D-0432 and RP-01 through RP-03 supersede the older
presence-only premise.

## The descent is reachable from the console, and shares the sweep's guards — D-0299

`cli` exposes fourteen commands and the browser could cause exactly one of them.
These rows are about the second, and about the conversion that made it safe to
add.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SW-17 | **A descent walks ONE rung and says which eight it accepts when told none.** A descent's whole shape is one rung's threshold walked, so the field is required rather than defaulted — a default would pick the timeframe the operator is hunting on, silently | `api::sweeprun::a_descent_without_a_rung_is_refused_and_names_the_eight`; `a_rung_this_engine_does_not_sweep_is_refused_by_name` | ✓ |
| SW-18 | **The rung list this route offers IS `cli`'s, not a copy that agrees with it.** `EVERY_RUNG` is re-exported from `cli`, so `assert_eq!(EVERY_RUNG, cli::EVERY_RUNG)` is an identity and there is nothing left to drift | `api::sweeprun::the_rung_list_here_is_the_one_cli_sweeps_by_and_not_a_copy_of_it` — **this row described the method that was replaced, and that method was silently vacuous.** It read *"it is a copy… the test asks `cli` about a rung it cannot sweep and compares"*, which sounds like a cross-check and was not one: `elite_descend_in_points` does not validate against `EVERY_RUNG` at all — it hands the name to `stored::load_span`, whose refusal lists the STORE's nine timeframes — so the comparison ran in one direction only, api ⊆ refusal. **The drift it was meant to catch actually happened**: api's copy held `1day` and was missing `30min` while the suite stayed green. Both halves of the old prose are now false — there is no copy, and no refusal is driven — so the row states the identity instead. Also asserted: no `1day`, a present `30min`, and every entry ending `min` | ✓ |
| SW-19 | **The stop ceiling crosses the wire in POINTS and is converted against the span's own bars.** `api` never computes or sees a ppm. `cli::points_to_ppm` converts against `NIFTY_REFERENCE`, and `reference_price`'s doc records that 800 ppm is twenty points on NIFTY and **forty-one on BANKNIFTY** — the same family of slip that once shipped a stop ladder at 2..10 ppm against a documented 200..1000, putting every priced stop inside the entry bar's own range. `elite_descend_in_points` reads the midpoint of the loaded span and converts there | `api::sweeprun::a_stop_ceiling_of_zero_or_less_is_refused_rather_than_swept_with`; `cli::elite_descend_in_points`'s own refusal when the conversion yields nothing | ✓ |
| SW-20 | **A descent and a sweep are told apart on the wire.** `support_ppm` is a fixed threshold on one and the ceiling a walk BEGAN from on the other, so a page reading it without knowing which would mislabel a hunt for a rare setup as a nine-rung comparison. `Kind` carries it; `Kind::Sweep` is the default so seven existing call sites are untouched | `api::sweeprun::a_descent_and_a_sweep_are_told_apart_on_the_wire` | ✓ |
| SW-21 | **The descent reuses the sweep's span rules rather than restating them.** Two parsers for one span is two places for a bound to drift — and the bound that matters here is the one that called `abort()` in a release build on `{"from_year":18446744073709551615}`. `descent_from` delegates to `asked_from`, so it inherits that refusal for free | `api::sweeprun::the_descent_reuses_the_sweeps_span_rules_rather_than_restating_them` | ✓ |
| SW-22 | **One slot, one ledger, one busy refusal.** A descent appends to the same append-only file as a sweep, so two finishing together can interleave two records. Both take the same slot, the same commit gate and the same 409 | `api::emitted` drives the descent's refusal; the busy path is shared code with SW-14's row | ✓ |

**Nine of fourteen are still terminal-only**, named here rather than left to be
found: `audit`, `audit-range`, `auto`, `auto-stored`, `screen`, `sweep`,
`sweep-all`, `sweep-stored`, `top`. `results` and `verify` are READABLE through
`/backtest.json` and `/verify.json` and cannot be caused.

## Every engine command the browser may cause, and the three it may not — D-0300

Fourteen command names. Seven causable, three readable, three refused by name
with the reason. These rows are about the closed dispatch and the one rule that
keeps a generated figure off a console that reads the store.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SW-23 | **The dispatch is CLOSED and an unknown word is refused listing what is accepted.** A fixed enum parsed from a fixed word list, not a generic run-anything surface | `api::sweeprun::an_unknown_command_is_refused_and_lists_what_is_accepted`; `every_stored_command_parses_into_its_own_shape` | ✓ |
| SW-24 | **`sweep`, `audit` and `auto` are refused with the PROVENANCE reason, never as typos.** They run over generated bars; §5 makes the banner the only thing separating a real sweep from an invented one, and a synthetic-data command on a store-reading console invites a generated figure being read as a measured one. The refusal names the rule and the stored equivalents, because an operator who typed `sweep` typed it correctly | `api::sweeprun::a_generated_bar_command_is_refused_with_the_reason_not_as_a_typo`; driven end to end in `api::emitted` | ✓ |
| SW-25 | **`sweep-stored` refuses a span longer than the one month it walks.** `cli::sweep_stored` takes a year and a month, not a range. Sweeping the opening month of a long request and recording it under that request's identity is a shorter answer wearing the request's name — so it refuses and points at `audit-range` | `api::sweeprun::sweep_stored_refuses_a_span_longer_than_the_one_month_it_walks` | ✓ |
| SW-26 | **A batch has no instrument and no span, and says so in values a page cannot mistake.** `sweep-all` reports `ALL` rather than an empty string — a blank reads as a field that failed to load — and a window of `(0,1)..(0,1)`, a year no real month can take | `api::sweeprun::every_stored_command_parses_into_its_own_shape` | ✓ |
| SW-27 | **A screen without a support, or with a zero ceiling or listing bound, is refused.** Support zero makes every combination frequent so the frontier never empties; a ceiling of zero admits no trade; zero rows is no answer | `api::sweeprun::a_screen_without_a_support_or_a_ceiling_is_refused` | ✓ |
| SW-28 | **Every command that needs a rung or a hit floor is refused without one, and `auto-stored` needs no floor** — searching for the threshold is the whole reason it exists, so demanding one would be asking the operator to answer the question they came to ask | `api::sweeprun::a_command_needing_a_rung_is_refused_without_one`; `a_command_needing_a_hit_floor_is_refused_without_one` | ✓ |
| SW-29 | **`/engine/top.json` takes no slot and no commit gate**, because it records nothing — §3 rule 3's identity requirement does not bind on a read, so an unstamped build can serve it honestly. `feed` and `underlying` filter together, matching `cli top`; a one-sided case invented here would make the page disagree with the terminal about one file | `api::sweeprun::top_json`'s refusal arm; the shape is `cli::top_list`'s own signature | ✓ |
| SW-30 | **All three kinds share one slot, one ledger and one busy refusal.** A sweep, a descent and a command all append to the same append-only file, so two finishing together can interleave two records | `api::sweeprun::a_command_run_is_marked_as_one_on_the_wire` for the wire; the busy path is shared code with SW-14 and SW-22 | ✓ |

## Browser engine admission refuses unobservable or silently changed work — D-0434

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SW-31 | **A browser engine task occupies no slot and starts no blocking work unless its exact attempt-start marker was written.** There is no process-local attempt fallback. Missing sink, filtered marker, dropped write and exhausted exact-id space answer 503 with the cause; all three task entry points cross the same gate before publishing acceptance or spawning | `api::sweeprun::an_engine_task_starts_only_after_its_required_attempt_marker_was_written`; `all_three_browser_engine_tasks_arm_and_disarm_the_same_finisher` pins all three marker gates; `api::emitted::every_reachable_emit_site_puts_a_record_in_the_file` drives the production marker | ✓ |
| SW-32 | **Every HTTP command carrying `min_hits` refuses zero before taking the slot.** `audit-range`, `sweep-stored`, and `sweep-all` share one positive parser. The engine's defensive zero-to-one floor remains, but the HTTP boundary never accepts a value the computation will silently change | `api::sweeprun::every_http_hit_floor_refuses_zero_instead_of_running_at_one` | ✓ |

## The dashboard shows both halves of the log — D-0301

`cli` and `api` own separate log directories so two live writers cannot
interleave lines in one file. These rows are about the reader, which walked one
of them.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| LG-01 | **`/logs` shows the CLI half as well as the server's.** Every event a terminal-run sweep wrote was on disk and invisible on the page that exists to show it — and `cli`'s banner printed that fact above every command an operator ran | `api::logs::the_page_shows_the_cli_half_and_not_only_the_servers` | ✓ |
| LG-02 | **The union is ordered on the CLOCK, not on sequence.** Each sink numbers from its own run, so `seq` is meaningless across two directories; ordering by it would interleave a terminal event from this morning with a server event from last week wherever the counters collided | `api::logs::the_merged_answer_is_newest_first_across_both_halves` | ✓ |
| LG-03 | **The caller's `limit` bounds the union.** Each half honours it already, so an untruncated merge returns up to twice what was asked for | `api::logs::the_limit_bounds_the_union_and_not_each_half` | ✓ |
| LG-04 | **A store where nobody ran `cli` is an empty half, never an error, and never claims older events exist.** `tail` renders a missing directory as no records and `walked` initialises `reached_oldest` true — checked rather than assumed, because that field was once `false` on ordinary full pages | `api::logs::a_store_where_nobody_ran_cli_is_an_empty_half_and_not_an_error` | ✓ |
| LG-05 | **Completeness flags merge in the direction that cannot over-promise.** `hit_scan_cap` and `partial_tail` OR; `reached_oldest` ANDs; `missing` sums, and a half answering `None` contributes nothing rather than a zero that would read as "none lost" | `api::logs::both_halves`, and LG-04 for the absent-half direction | ✓ |
| LG-06 | **A ledger run's event count is bounded by constants, not by the operator's span.** Every loop in `ledger_all` and `ledger_v6` walks a compile-time array — `LEDGER_RUNGS` (8), `ROUTE_FAMILIES` (2), `ALL_GATES` (39) — and neither module holds a loop over a bar, a candidate or a grid cell. So one run emits 17 events (`ledger-all`) or 43 (`ledger-v6`) whether the span is one month or eighty, which is the per-run/per-rung granularity gate 17 prescribes as affordable. Before this both verbs emitted ZERO events of any kind, so a multi-hour run was terminal text only | `cli::ledger_all::tests::every_ledger_boundary_is_targeted_typed_and_inside_the_field_ceiling`, `cli::ledger_v6::tests::every_v6_boundary_names_its_rung_and_stays_inside_the_field_ceiling` | ✓ |
| LG-07 | **Every refusal is a Warn that names its reason.** `CLAUDE.md` §4 bans a failure that is invisible, and a ledger chain that emitted only on success would report a halted run as silence. `run_refused` fires on every exit path | `cli::ledger_all::tests::every_refused_boundary_is_a_warning_that_names_its_reason`, `cli::ledger_v6::tests::a_refused_family_is_a_warning_that_names_the_family` | ✓ |
| LG-08 | **The boundaries reach the log FILE, not merely the builder.** Proven by driving the real verb and reading the real sink, in the shape `api::emitted` uses. **Honest limit: 5 of 27 call sites are proven to land** — both verbs' start and refusal, plus the gate refusal, all reachable because `parse_vendor` refuses before any store root is touched. The other 22 fire only after a span has loaded; their FIELDS are proven by LG-06, their REACH is not, and no store fixture exists on that path today | `cli::ledger_all::tests::the_ledger_all_run_boundaries_reach_the_log_file`, `cli::ledger_all::tests::the_unset_gate_refusal_reaches_the_log_file`, `cli::ledger_v6::tests::the_ledger_v6_run_boundaries_reach_the_log_file` | ◐ |

## A cost claim and its measurement cannot drift apart — D-0302

Seven defects in one session shared one shape: a written claim that no longer
matched the code. These rows close that class for the cost invariants, in both
directions, with everything discovered at runtime.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| RC-01 | **Every id a bench prints is declared as a row.** A measurement nobody wrote down is a number with no claim attached — `C-I-05`'s own doc read *"JUDGED HERE, PINNED NOWHERE ELSE"* and observed that deleting its assertion would delete the measurement with every static check still green | `core::cost_invariants::every_measured_cost_invariant_is_declared` | ✓ |
| RC-02 | **Every declared row reaches a bench**, directly or by naming another cost id that does, taken to a fixed point. A row claiming a bound with no measurement is a promise nothing keeps, and gate 8 cannot see it because gate 8 runs the benches that EXIST. `C-02` now names the live C-E-02 and C-E-09 benches directly; C-E-02b remains the append-only record of the Θ(k) implementation those rows replaced | `core::cost_invariants::every_declared_cost_invariant_reaches_a_bench` | ✓ |
| RC-03 | **No exception is listed.** The four rows without their own bench pass by naming the id that measures them, which each already does because *"superseded by"* has to say by what. A hand-kept allowlist is the same rotting claim this section exists to refuse | the absence of an allowlist in that file, and RC-02 passing without one | ✓ |
| RC-04 | **The crate list and the bench list are walked, never written down.** `read_dir` over `crates/` finds every `benches/ratio.rs` at runtime, so a crate added tomorrow is covered without editing the test — and a crate carrying a manifest and no bench fails | `core::cost_invariants::every_crate_carries_a_ratio_bench` | ✓ |
| RC-05 | **A walk that finds nothing fails rather than passing vacuously.** Both checks above are satisfied by an empty document and an empty bench set — the shape §4 bans, and one this repository has been bitten by: gate 8 once *"tested for a benches directory at the repository root, found none, and exited zero"* | `core::cost_invariants::the_reconciliation_is_reading_something` | ✓ |
| RC-06 | **The five operations `CLAUDE.md` §3 rule 4 names are all still mentioned.** A document that stopped naming one would leave that rule with nothing behind it and no ratio would notice | `core::cost_invariants::the_document_declares_the_five_operations_rule_4_names` | ✓ |

**Measured to bite, not merely to pass.** Three mutations were run against the
real tree and reverted: a fabricated row with no bench (failed, naming it), a
bench id renamed so its measurement lost its row (failed, naming it), and a
crate's bench file removed — which **cargo itself refuses**, because the
manifest declares the target.

**Not checked here:** the figures. A row claiming 0.996× against a bench
measuring 1.687× passes, because a number lives in a run and this is a static
read. Gate 8 is what fails on a breach.

## The support floor is derived from the data, never typed and never baked — D-0303

`SUPPORT_PPM = 200_000` decided what the engine was allowed to find, and the
engine's own doc records that a tenth of it already prunes a once-a-week setup.
These rows are about its replacement.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SF-10 | **The derived floor admits what the constant pruned.** On this store's own bar counts the constant demanded 4,324 hits on the 1min rung, 2,328 on 15min and 334 on 1day; the derived floor is under a hundred on every one of them. A cadence in the thousands is not one an operator calls rare | `api::sweeprun::the_threshold_is_derived_and_admits_what_the_constant_pruned` | ✓ |
| SF-11 | **It is still a RATIO per rung, so the rungs stay comparable.** Held as one absolute count, a 1min and a 1day rung would clear the same hits from twenty-times-different bar counts and the daily rung would find nothing for a reason with nothing to do with the market. Nine rungs share a *statistical standard* now rather than an arbitrary percentage | same test's final assertion — the floor differs across a 13× bar spread | ✓ |
| SF-12 | **The floor is never zero, at any bar count including zero.** The property is unchanged and its subject moved: it used to be that the constant could never be zero, and it is now that the derivation never is. A span the store could not fill must not become a threshold of zero | `api::sweeprun::a_support_threshold_of_zero_cannot_be_asked_for_at_all` | ✓ |
| SF-13 | **`None` is the absence of a threshold, not a magic zero.** `support_ppm` reaches the wire as `null` on a derived run and the banner prints `DERIVED per rung from its own bars`. There is no single number to report because nine rungs derive nine floors, and a figure printed without its provenance invites comparing two runs that measured different things | `Progress::to_json`'s `None` arm; `cli::range_all`'s banner | ✓ |
| SF-14 | **The stop ceiling and the listing bound cannot move the floor.** `statistical_floor_ppm` reads the win rate and the assurance and nothing else, so a caller made to supply a risk ceiling in order to learn a statistical floor would set it carefully for no effect | `cli::statistical_support_floor` passes `1` for both, with the reason | ✓ |
| SF-15 | **A floor that no sample size can support is refused, never reported as one.** `assurance_floor_bp` returns the caller's figure unchanged at or below chance, so `Rules::derived` — which takes `base_bp.max(Rules::operator's 5_000)` — yields the pair `(5_000, 5_000)` on any series whose own base rate is below chance. A 95% lower bound cannot reach the rate it bounds, so `grid::trades_needed_for` exhausts and returns `TRADES_SEARCH_CEILING`, which `statistical_floor_ppm` then prices as a floor. **Measured, zerodha NIFTY 60min 2020-01..2026-08: `floor 434140 ppm ... about 4999 round trip(s)`, ONE descent rung, nothing passed — the cap wearing a statistic's clothes, and every rare setup pruned before it was priced.** The same span with a satisfiable pair descends four rungs to 30563 ppm (352 round trips) and weighs 38,503,239 combinations to extinction at depth 19. §4 bans a fallback that hides a failure — D-0591, **corrected by D-0592**. **This row shipped stating two falsehoods and an adversarial audit found both.** (a) The guard tested the proxy `bound >= rate`, which above chance is arithmetically just `rate <= 5_000`: measured on the shipped bound, rates 5001, 5100 and 5272 ALL still return the 5,000-trade cap and the identical 434140 ppm, unrefused — and the refusal's own text said "set BRUTEX_MIN_WIN_RATE_BP above 5000", which lands inside that band. (b) The proxy REFUSED `BRUTEX_MIN_WIN_RATE_BP=0`, which `Rules::stated` admits deliberately and `trades_needed_for` satisfies in ONE round trip via its `assurance_bp <= 0` arm. The claim that `knobs::sizing_rate` "already refused this pair on the typed path" was also false — its filter `*value > 5_000` ADMITS 5001, and it guards a different knob on `statistical_support_floor`, which has no check at all. The guard now asks the search itself — `trades_needed_for(..) >= TRADES_SEARCH_CEILING` — which closes both directions and stays correct if the bound, the rounding or the ceiling changes. **Still open: `statistical_support_floor`, reached by `crates/api/src/sweeprun.rs` and the browser Run button, remains unguarded because `statistical_floor_ppm` returns a bare `u64` no caller can distinguish from a measurement.** | `cli::tests::an_unsatisfiable_confidence_pair_refuses_rather_than_pricing_the_cap` — asserts the guard agrees with `trades_needed_for` across 18 rates; pins the measured band as values (5000/5001/5100/5272 refuse, 5273/6000 descend); pins that 0 descends; and pins that the refusal does NOT contain "above 5000" | ✓ |

**No minimum was invented to replace it.** The floor is not a trade count
somebody chose; it is the point below which a number stops meaning anything —
four round trips on the shipped Wilson bound at an 80% rate, against the 526 the
retired cadence floor demanded.

**Still static and named rather than left to be found:** `NIFTY_REFERENCE`
(25,000, wrong for BANKNIFTY — mitigated on the browser routes by the
`*_in_points` entry points, live on the argv `elite` arm), the exit grid's
point ladder, `WALK_FORWARD_SPLITS`, `BOOTSTRAP_DRAWS`, `BOOTSTRAP_ALPHA_PPM`,
`MIN_AUDIT_SESSIONS`, and `engine::DEFAULT_CEILING` / `DEFAULT_PAIR_BUDGET`,
which halt a ladder and so bound what a run explores.

## MR — refreshing the instrument masters

Before D-0308 nothing in this workspace had ever fetched a master: the four
files arrived on the operator's disk by hand, and a stale one was invisible
because the parse succeeded and every symbol renamed since resolved to the old
row. D-0310 added the exchange's own index list; D-0311 turned a single ask into
a downloader.

| # | Invariant | Proof | Status |
|---|---|---|---|
| MR-01 | **A body that is not a master never overwrites one.** Four independent guards, and each catches a shape the others let through: below `MIN_BODY_BYTES` (a truncated download, an error page), a first byte of `{` `[` or `<` (JSON or HTML where CSV was expected), a first line with no comma (a plain-text splash), and — for the JSON source — a document `nse_index_csv` cannot convert. The write is atomic through a per-source `.partial` and a rename, so a refusal leaves the previous master exactly as it was | `pull::masters::a_body_whose_first_line_has_no_comma_is_refused_with_that_line_quoted` · `a_json_body_that_will_not_convert_refuses_at_the_landing_and_names_the_url` · `no_partial_file_survives_a_landing` | ✓ |
| MR-02 | **The guards run on the CONVERTED body, never on what the host answered.** NSE answers JSON and the engine reads CSV; run in the other order the opener check would refuse every correct response the endpoint will ever give — a guard rejecting the thing it exists to admit | `pull::masters::the_index_source_lands_through_the_conversion` | ✓ |
| MR-03 | **A malformed index document refuses rather than emits, in four named ways** — not JSON, not an object, no rows at all, or a comma or newline inside a name or category. `Published::read` splits on commas and does not unquote, so quoting a field would deliver its quotes as part of the index's name: refusing is the only alternative to corrupting | `pull::masters::an_index_json_that_is_not_the_expected_shape_refuses_rather_than_emits` · `a_comma_in_an_index_name_refuses_rather_than_breaking_the_columns` | ✓ |
| MR-04 | **One malformed element does not cost the whole document.** A list entry that is not a string, and a category whose value is not a list, are skipped; every well-formed name beside them still converts. Refusing the document over one bad element would cost the catalogue every index NSE publishes | `pull::masters::an_index_document_with_a_non_string_name_skips_it_rather_than_refusing` | ✓ |
| MR-05 | **Every HTTP status maps to exactly one of three verdicts, and the boundaries are asserted rather than sampled.** `501` and `505` sit inside the otherwise-retryable `5xx` range and are named ahead of it; `401` and `403` are `Reprime` rather than `Never` because a session-gated host answers them to a request that is not the same request once a session exists; a refusal carrying no status at all is `Again`, because a dropped socket is the road and not a decision the host made | `pull::masters::every_status_lands_in_the_verdict_its_row_names` | ✓ |
| MR-06 | **The backoff is bounded, deterministic and never zero.** 500 ms doubling to a 32 s cap. Written first with `checked_shl`, which bounds the shift AMOUNT and not the result — `500u64.checked_shl(63)` is `Some(0)` — so the ladder would have made five attempts with no wait between them while claiming to back off. `checked_pow`/`checked_mul` bound the value | `pull::masters::the_backoff_schedule_is_the_one_documented`, whose `backoff_ms(63)` row is the one that fails on the old implementation | ✓ |
| MR-07 | **A settled refusal is asked once and a transient one is asked to a fixed ceiling.** Five attempts at a `404` is four requests spent learning what the first one said; one attempt at a `503` is a coin toss. Both bounds are asserted, so the loop cannot run away inside an HTTP handler | `pull::masters::a_settled_refusal_is_not_re_asked_even_once` · `a_transient_refusal_gives_up_after_the_documented_number_of_asks` | ✓ |
| MR-08 | **Mirrors are walked after a primary is exhausted, never alternately** — and a settled refusal at the primary moves to the mirror rather than ending the fetch, because the file may well be at the other address | `pull::masters::a_mirror_is_tried_only_after_the_primary_is_exhausted` · `a_settled_refusal_moves_straight_to_the_mirror` | ✓ |
| MR-09 | **A source that declares a prime is primed before its first ask, re-primed at most once, and a prime that FAILS is a row rather than a silence.** The real request goes out regardless: the prime satisfies a requirement this repository cannot verify (`nseindia.com` is unreachable from the environment this was built in), so the host's own answer is more informative than this module's guess about why it mattered. §4 bans a fallback that HIDES a failure, not one that reports it | `pull::masters::a_session_gated_host_is_primed_before_the_first_ask` · `a_forbidden_answer_reprimes_exactly_once_and_then_stops` · `a_prime_that_refuses_is_recorded_and_the_real_ask_still_goes_out` | ✓ |
| MR-10 | **A cookie is presented only to the host that set it.** The jar is keyed by host, so a session established at the exchange cannot ride along to a vendor CDN. It orders its fields through a `BTreeMap`, so the same session produces a byte-identical request line on every run — §3 rule 5, applied to a header nobody would otherwise think of as an output | `pull::masters::a_cookie_never_leaves_the_host_that_set_it` · `the_cookie_header_is_byte_identical_for_the_same_set` | ✓ |
| MR-11 | **The credentialed source declares no prime, whatever else changes in the table.** A prime is an extra address the shared token would travel to, which walks around what `may_fetch` exists to prevent. Asserted over `SOURCES` rather than about one row, so a future source that needs a token inherits it | `pull::masters::the_credentialed_source_declares_no_prime` | ✓ |
| MR-12 | **A public refresh contacts only hosts a public source names.** Checked as membership over every URL, every mirror and every prime — an earlier version counted requests instead, and broke the moment a source declared a prime: four requests for three sources looked like a leak and was a session being established. The count was never the property; whose host was contacted is | `api::mastersrun::a_public_refresh_never_asks_for_the_credentialed_url` | ✓ |
| MR-13 | **A source that was not asked is a row carrying an EMPTY ledger, never an absence.** "Skipped" and "asked and got nothing" are different facts, and the page distinguishes them by the array rather than by parsing a sentence. The route shipped once answering `200` with `zerodha_instruments.csv` absent because a skipped source fell out of the vector entirely | `api::mastersrun::a_skipped_source_is_a_row_and_never_an_absence` · `a_skipped_source_carries_an_empty_ledger_rather_than_a_missing_one` | ✓ |
| MR-14 | **Every step reaches the page: which URL, which attempt, what was waited, what came back, and what was decided about asking again.** A refresh that answers one sentence has discarded the three questions an operator actually has, and `refused_settled` and `refused_retrying` call for opposite responses while rendering identically without it | `api::mastersrun::the_attempt_ledger_reaches_the_page_with_every_step_named` · `a_settled_refusal_reaches_the_page_labelled_as_settled` | ✓ |
| MR-15 | **The credentialed leg runs the same ladder as the public one.** It was the leg with no retry at all, and it is the master for the feed holding every bar in the store — nothing about spending a credential makes a transient failure less transient. A credential that could not be READ is separate: nothing was asked, so the ledger stays empty | `crates/api/src/mastersrun.rs`'s `refresh`, whose credentialed arm calls the same `obtain` | ✓ |
| MR-16 | **A response body cannot grow without bound.** `Response::text` reads to the end with no ceiling; the declared length is refused first where there is one, and the body is then read chunk by chunk against `MAX_BODY_BYTES`, because a declaration is optional and optionally true. Decoded with `from_utf8` and never `from_utf8_lossy`: a replacement character where a symbol used to be parses cleanly and resolves the wrong instrument | `pull::masters::the_body_ceiling_is_far_above_any_real_master_and_far_below_this_machine`; the loop itself is inside the socket call — `docs/06-limits.md` §95 | ~ |
| MR-17 | **The refresh is reachable without reading the route table.** `GET /masters` carries one row per source — asserted against `SOURCES` rather than against `4`, so a fifth master cannot be added and silently left off — and `("/masters", "Masters", true)` puts it in the nav beside every other page. D-0308 shipped the two JSON routes with neither | `api::mastersrun::the_page_carries_a_row_for_every_source_and_names_what_each_costs` · `the_page_is_reachable_from_the_navigation` | ✓ |
| MR-18 | **Opening the page fetches nothing.** The load path stats four files; the refresh is bound to a press and to nothing else. Both halves are asserted — that the listener exists, and that `refresh();` appears nowhere in the script — because a page that quietly spends a vendor request on load is the operator's one standing prohibition | `api::mastersrun::the_page_carries_one_external_script_and_no_inline_handler` (the HTML half), and CI's `web` job over `web/masters.js` (the binding half). **The single test this row used to name has never existed and could not**: gate 1e runs the whole workspace with `web/` DETACHED to prove no crate depends on the front end, so a Rust test that opened `masters.js` would go red in the one job whose green is that guarantee. The proof is split because the subject is | ✓ |
| MR-19 | **The credentialed source is visibly distinguished from the free ones, and a primed source says so.** Pressing a control that spends a shared token must not look like pressing one that fetches a public CDN file; and an unexplained extra request in the ledger reads as a bug, so the row that causes it names the priming URL | `api::mastersrun::the_page_separates_the_free_sources_from_the_one_that_spends_a_token` · `the_page_says_a_prime_happens_where_one_does` | ✓ |
| MR-20 | **New bytes on disk do not silently change what the server answers, and the page says so.** `Site::load` parses the masters once at startup with no reload path. The row reports `newer_than_parse` and the post-refresh line asks for a restart; `status.json` carries `modified_unix_millis` so "present" can be told from "present and eleven months old", `null` rather than a zero that renders as 1970 | `api::mastersrun::the_page_says_a_restart_is_required_rather_than_pretending_otherwise` · `the_status_answer_carries_when_each_master_was_written` | ✓ |
| MR-21 | **Every step of a refresh is durable, not only the response body.** One event per network round trip and one per source outcome, so an operator asking "why did this fail an hour ago" can search it. Before this, the whole route wrote three counts and the attempt ledger died with the browser tab | `api::emitted::every_reachable_emit_site_puts_a_record_in_the_file`, rows `api.masters.attempt refused settled` and `api.masters.source could not be refreshed` | ✓ |
| MR-22 | **The log level follows the outcome, at both granularities.** A refresh where all four sources refused used to emit `Info: the masters were refreshed` with `landed=0` — a success-shaped line over a total failure. A settled refusal is `Error`, a retryable one is `Warn`, a landing is `Info`; a skipped source is `Warn` because the guard working is neither | the two census rows above pin the `Error` arms; `crates/api/src/mastersrun.rs`'s `record` and the summary emit carry the rest | ✓ |
| MR-23 | **The join every master exists for is reachable in one press, and names the symbols it could not resolve.** `/indexmap.json` reported them and had no page and no nav entry. A symbol the exchange does not confirm is one whose bars are filed under a name nothing corroborates, so it is shown rather than tallied | `api::mastersrun::the_page_cross_verifies_every_feed_against_the_exchange` · `an_unconfirmed_symbol_is_never_filtered_out_of_the_view` · `the_cross_verification_is_also_bound_to_a_press` | ✓ |
| MR-24 | **The page renders the nav it appears in.** Being *in* the nav and *rendering* one are different things, and this page had the first without the second — an operator following the link landed with no way back and no indication of where they were. It renders `render::nav` rather than a hand-written link bar, so one list of pages exists rather than two that must agree forever | `api::mastersrun::the_page_renders_the_nav_it_appears_in`, which asserts every other built page is reachable from here | ✓ |
| MR-25 | **A master the reader cannot parse never reaches disk.** `land`'s other guards describe the SHAPE of a master — long enough, not JSON or HTML, first line carries a comma — and a vendor's *other*, equally well-formed CSV at a neighbouring URL satisfies every one of them. That is not hypothetical: Dhan's compact master landed at 26 MB and `/health` answered `dhan: UNAVAILABLE — no column "SECURITY_ID"`. The column check reads `core::vendor::master_columns`, the same declaration the reader fails on, so there is no second list to drift | `pull::masters::the_compact_dhan_master_is_refused_because_the_reader_cannot_read_it` · `a_wrong_but_well_formed_master_leaves_the_good_one_on_disk` | ✓ |
| MR-26 | **An absent column is not a required one.** Zerodha publishes no `ISIN` and no listing class, declared as `""`. A guard that looked for a column named `""` would refuse that vendor's entire real master for lacking a field it has never published — stricter than the reader it guards, which is worse than no guard | `pull::masters::a_master_carrying_every_declared_column_is_not_refused` · `the_guard_reads_the_vendors_own_declaration_and_never_a_second_list`, whose final rows assert against Zerodha's real published header | ✓ |
| MR-27 | **A refresh takes effect without a restart.** The masters were parsed once at startup with no reload path, so pressing Refresh gave new bytes on disk behind an old universe and `restart_required` was hardcoded `true`. `Site::reparse` swaps the three masters-derived fields under one `RwLock`; `run`, `sweep`, `budgets` and `calendars` stay outside it, so reloading cannot cancel a pull in progress | `api::emitted`'s `api.masters.reload` row; `crates/api/src/server.rs`'s `Site::reparse` | ✓ |
| MR-28 | **A reload never replaces a working universe with an empty one.** A failed download must not cost an operator the parse they already had — the refresh destroying what it was pressed to improve. `reparse` compares the fresh instrument count against the held one and returns the refusal with the old universe untouched | `Site::reparse`'s guard clause; the census row drives the arm where nothing is lost | ~ |
| MR-29 | **No lock is held across a network crawl.** `universe_resolve` awaits ~150 constituent fetches, and a `RwLockReadGuard` spanning an `.await` makes the future `!Send` — axum refuses the handler at compile time. A reader held for the length of a crawl would block the operator's next refresh for minutes | the compiler: the handler does not build otherwise. `server::universe_route_tests::the_resolve_route_reads_its_master_from_memory_and_never_from_a_vendor` pins that it still reads from memory | ✓ |
| MR-30 | **An index with no basket is not a failed crawl.** Measured on the first live pass: 16 of 17 refusals were derived indices — leverage, inverse, futures, USD, arbitrage, debt-hybrid — which publish no constituent file because they have none. At `Error` beside a real failure they buried the one that mattered, a constituent CSV served as a bot-check. Now a separate list, at `Warn`, on its own wire key and its own cell | `pull::resolve::an_index_page_with_no_link_fails_rather_than_composing_a_filename` · `api::server::an_empty_pass_renders_every_bucket_and_reports_why_it_cannot_publish`' snapshot row asserting `"failed":1` and `"unlinked":1` separately | ✓ |
| MR-31 | **And it is never dropped.** A basket index whose link has MOVED produces the identical observation, so discarding the class would take that with it. The message states both readings and says the pass cannot distinguish them; every page is named on the wire | the same test asserts the derived index is named, not merely counted | ✓ |
| MR-32 | **Completeness ignores the unlinked list.** A pass is complete when nothing it could read failed to read. Folding them in would make every pass against the real exchange unpublishable for ever, which is the same as never publishing, arrived at by accident | `pull::resolve`'s test asserts `is_complete()` holds with an unlinked page present | ✓ |
| MR-33 | **The crawl transport presents the same browser shape as the masters one.** It sent no user agent at all — the request shape a filter exists to catch — while `masters::PublicFetch` had already been given one for the same host family. Sufficiency against `nseindia.com` is UNVERIFIED; the next live crawl is the measurement | `crates/pull/src/resolve.rs`'s `HttpDocuments::new`; see D-0317 | ~ |
| MR-34 | **The calendar is read from disk per request, never from the boot snapshot.** `Site::entries` is filled once in `Site::load` and never again, so a server started before its first pull answered `{"sessions":0,"days":[]}` for life — and `web/`'s `isSession` then fell back to counting weekdays, expecting 1,758 daily bars where 1,671 exist and printing SHORT against 204 complete instruments. The shortfall was the 92 public holidays, to the bar. Both the names and the months come from `census::read_all` | `crates/api/src/server.rs`'s `calendar_json`; the arithmetic is reproduced in D-0318 | ✓ |
| MR-35 | **No request-serving code answers from the boot snapshot.** `Site::entries` is filled once in `Site::load` and never again, and reaching for it has now cost three separate defects: the ladder gate refusing a minute pass whose day pass had landed, `/calendar.json` answering zero sessions and making the page print SHORT against 204 complete instruments, and `/store` rendering empty beside a `/store.json` answering 15,857 rows. A comment cannot hold a `pub` field that is the obvious thing to reach for, so the rule is mechanical — the scan is proven to bite by reintroducing the bug and watching it fail | `api::server::an_empty_pass_renders_every_bucket_and_reports_why_it_cannot_publish::no_handler_answers_a_request_from_the_boot_entries_snapshot` | ✓ |
| MR-36 | **A month before an instrument listed does not hold the minute rung shut.** The gate required a day entry for every month in the window, and a month before listing returns zero bars, which by design writes no manifest entry — so `held` was false for ever. Measured: 28 of 210 F&O underlyings are post-2019 listings accounting for **751 unfetchable instrument-months**, and the minute pass would have 409'd permanently. An instrument's owed span now starts at its earliest held month | `api::ladder::a_month_before_the_instrument_listed_does_not_hold_the_minute_rung_shut` | ✓ |
| MR-37 | **And a hole INSIDE the span still refuses.** Narrowing the question must not weaken it: excusing an interior gap would let the minute pass run over a day pass that genuinely failed, which is what the gate exists to prevent. An instrument holding nothing at all keeps every month owed, because that is a day pass that has not run | the same test asserts both arms | ✓ |
| MR-38 | **The fetch is chunked to the vendor's cap, not to the store's file boundary.** `split_window` clamped every chunk to a month end because `ingest::one` refused a cross-month batch, which turned a 2,460-day window into 81 requests where Zerodha's documented 2,000-day `day` cap needs 2 — measured at 0.35 s per instrument-month against a 0.33 s ceiling, 94 minutes against 2.3. `months_in` splits a decoded batch into one run per month and `one` writes each to its own file | `pull::session::a_chunk_fills_the_cap_and_the_chunks_tile_the_window` · `the_documented_caps_turn_a_seven_year_window_into_a_handful_of_requests` | ✓ |
| MR-39 | **And the bars reach the right month.** A splitter that filed July's bars under August would be worse than the refusal it replaced — the requests saved and the store silently wrong until a backtest read the wrong window. The split is over DECODED bars, so the month comes from the same `TimestampEncoding` dispatch that `land` uses, never a second implementation of it | `pull::broker::a_batch_spanning_two_months_lands_in_both_and_the_bars_go_to_the_right_one` | ✓ |

## Every rule floor is measured, and the one that was blind set itself to zero — D-0378, D-0379

`Rules::operator()` shipped four constants that decided which combinations an
operator is shown. These rows are about their replacements, and about three
adjacent defects that each switched a rule off without saying so.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| RF-01 | **A series and its mirror derive the SAME floor.** The mirror is the same market seen from the other side, every forward delta negated, so a null that gates both directions cannot differ between them. Under the long-only reading they were complements and could not both be right — on a falling span it returned near zero, and `min_win_rate_bp`, `min_assurance_bp` and `min_weakest_bp` all became floors every combination clears | `cli::derived_floor_tests::the_base_rate_is_the_same_for_a_series_and_its_mirror` — fails on the old code with left 10000, right 0 | ✓ |
| RF-02 | **A flat forward window is a win for NEITHER side.** A window that closed exactly where it opened pays for no position in either direction, and awarding it to one would make the two rates sum past 10,000 and stop them being complementary. Measured: on `ADANIENT` one-minute bars at a one-bar horizon, 6.98% of decided windows are flat and reading them as "not up" moves the rate 698 bp — across the coin flip, 4592 to 5290 | the `else if delta < 0` arm; `indicators/src/session.rs` records the identical defect it had to fix | ✓ |
| RF-03 | **A reward-to-risk floor never rounds to zero.** `admits` reads `min_rr_bp == 0 \|\| …`, so zero DROPS the rule. The guard refused rates at or above 100% because they "would divide by a break-even of zero", but the quotient hit zero ninety-nine basis points earlier and every rate in `9901..=9999` disabled the rule silently | `cli::derived_floor_tests::a_break_even_floor_never_switches_its_own_rule_off`, asserted across all 9,999 inputs rather than at three edges — the defect was a band, not a boundary | ✓ |
| RF-04 | **At a coin flip the arithmetic reproduces the constant it replaced, exactly.** 125, and a lower win rate must demand a LARGER payoff. A direction assertion rather than a value one, because the first draft was a hundred times too large and every derived floor exceeded the guard and fell back to the constant it was written to replace | same test's final two assertions | ✓ |
| RF-05 | **An unusable knob is not a stated floor.** `operator` parsed and fell back; `derived` asked whether the variable was PRESENT, and `knobs::var` returns `Some("")` for one exported and empty. A bare `BRUTEX_MIN_WIN_RATE_BP=` in a shell profile handed back the coin flip with no message and no refusal — §4's banned fallback. Both callers read one `Rules::stated` | `cli::derived_floor_tests::an_unusable_knob_is_not_a_stated_floor`, driving five unusable spellings and one usable one | ✓ |
| RF-06 | **A knob is APPENDED to `policy_of`, never inserted, and the position is asserted by index.** The length check cannot see an insert — sixteen is sixteen wherever the term sits — and an insert shifts every term below it, silently re-keying every run identity ever recorded | `every_knob_that_moves_the_answer_moves_the_identity` pins `start[15]`; `every_operator_rule_moves_the_identity` pins `start[5]`. The second caught the real one, failing with `left: 1` — `u64::from(validate)` standing where the ceiling belongs | ✓ |

**The four floors, and what each replaced:** `min_win_rate_bp` was 5,000 (a coin
flip), `min_rr_bp` 125 chosen separately, `min_weakest_bp` **0 — no per-period
floor at all**, `min_ret_over_dd_bp` 500. Each still falls back to its stated
default when the benchmark is undefined, and each reaches the run identity, so a
derived-floor run and a stated-floor run are different runs and are recorded as
such.

**Not claimed:** that the derived floors are *harder*. `breakeven_rr_bp` is
looser than 125 for every rate above a coin flip — that is what the arithmetic
says, and it is applied to `min_win / -worst_trade`, which is smaller than the
average ratio the formula assumes, so the gate errs strict in a way the number
does not show.

## The side reaches the operator — D-0380, D-0382

The engine's pricing layer is symmetric about direction everywhere — `fills_at`,
`BarMoves::of`, `level_price` and `peak` each resolve both cases with a comment
explaining why. Its selection and reporting layers were not. These rows are about
closing that.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SD-01 | **A setup and its mirror are worth the same payoff.** `payoff_bp` took the positive moves as the wins always, so for a short it returned the exact reciprocal and `ByPayoff` ordered shorts by inverse merit. `Lens::Payoff` is what `audit_range_inner` selects | `runner::outcome::tests::negating_every_move_leaves_the_payoff_where_it_was` | ✓ |
| SD-02 | **The ideal short is worth the same as the ideal long.** A combination whose forward move is ALWAYS down has `wins == 0` and scored ZERO — the minimum — so `keep`, a hard cut, discarded the best short in the run before it met an exit grid | `runner::outcome::tests::the_perfect_short_and_the_perfect_long_are_worth_the_same` | ✓ |
| SD-03 | **The side travels with the row and is never re-derived from the mean's sign.** Two of the three surfaces an operator selects from carry no mean at all, so there it is not a copy — it is unrecoverable. The test row is written SHORT with a POSITIVE mean, so anything re-deriving it answers long | `api::frontierjson::tests::a_ranked_row_carries_its_verdict_and_the_rules_it_was_judged_against` | ✓ |
| SD-04 | **The column shows the side the cell was PRICED with, not a second reading of the evidence.** `Screened::side` takes the same value handed to `grid::evaluate`, so the column cannot describe a different trade from the one measured | `Screened { side: direction_of(side), .. }`, four lines below the `evaluate` call | ✓ |

**Where the side now appears:** `frontier::Row::direction` (one byte, taken from
the reserve), `/frontier.json`'s `"direction"` key, the SCREENED table's second
column, and `traded_line`. `costs::fill::Direction::as_str` — documented *"for a
refusal or a breakdown line"* and until now without a production caller — is what
names it.

**Still open, and stated because §3 rule 6 asks:** `walk_forward` takes ONE
direction, from the top row of a rank over the whole span including the test
folds, and applies it to every candidate in every fold. That is look-ahead on the
side, and it means PBO is computed on a ranking in which every short-edged
candidate was priced as a long. Not fixed here.

## A verdict is computed from the rules the run applied — D-0382

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| RJ-01 | **The rules on the wire are the run's own.** `api::frontierjson` read `Rules::operator()` at REQUEST time while the run swept at `Rules::derived` or `Rules::elite`. A row the run REFUSED rendered PASS and a row it ADMITTED rendered FAIL, with the wrong threshold printed beside it | reverting the handler to `Rules::operator()` fails the test with `"min_win_rate_bp":5000,"min_rr_bp":125` for a row written under elite's 8000/300 | ✓ |
| RJ-02 | **A version this build does not write is REFUSED, never widened.** A v3 row read with zeroes in the new fields carries all-zero floors, and `admits` treats zero as *rule off* for `min_rr_bp` and `max_mae_ppm` — so every priced row would pass. The refusal names the remedy: the file is regenerable | `frontier::check_header`'s version arm, and its own message | ✓ |
| RJ-03 | **Reading creates nothing.** `/engine/top.json` called the creating openers, so asking it a question MADE `results/`, `runs.bin` and `frontier.bin` — and it then read the files it had just made and reported no run was recorded | `results::tests::opening_to_read_creates_neither_the_file_nor_its_directory`, asserted on the filesystem after the call rather than on the refusal text | ✓ |
| RJ-04 | **An absent frontier is not a refusal at `top_at`.** A run with no frontier file recorded no frontier, which is a different fact from "this run found nothing" and is the one the page already states. Every other refusal — wrong magic, wrong version, a ragged tail — still refuses, because those are about a file that EXISTS and is not this one | `cli::tests::a_run_with_no_frontier_says_so_rather_than_looking_empty` | ✓ |

## Both runs are measured, and the grains reach one hour — D-0384

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SG-01 | **Both streaks are measured and each one ends the other.** `max_losing_streak` shipped alone, so a reader learned what to sit through and never what to sit through it for. The test sequence puts the four-win run AFTER the three-loss run, so a counter the losses did not clear would read seven | `runner::grid::tests::both_streaks_are_measured_and_each_one_ends_the_other` | ✓ |
| SG-02 | **A flat round trip breaks the winning run and extends the losing one.** `pess > 0` is the only win `tally_trade` knows. That is the rule `max_losing_streak` already carried; it is stated now because two counters side by side look like they partition and these do not | same test's second half | ✓ |
| SG-03 | **The hour grain SEPARATES what every coarser grain joins.** A combination whose whole edge sits between 09:15 and 10:15 is positive in all six of year, half, quarter, month, week and day, and `weakest_bp` — their minimum — reports it steady. That is the one-regime-carrying-four shape at a resolution the ladder could not resolve | `cli::stability::tests::the_hour_grain_separates_two_times_of_one_trading_day`, on two stamps five hours apart in one IST session | ✓ |

## The fold decides its own side, on training bars alone — D-0387

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| WF-01 | **What the caller passes cannot move the answer.** The direction handed in came from a rank over the whole span, test folds included — the side fitted to the window it is meant to be tested on, which is the look-ahead this function's own doc rejects for the stop. A Long caller and a Short caller must now produce identical folds | `runner::validate::tests::the_fold_decides_the_side_and_the_caller_cannot`, asserted fold for fold on `(in_sample, chosen, chosen_side)` | ✓ |
| WF-02 | **Each candidate is priced on the side ITS OWN training evidence points to.** One direction applied to all of them meant a short-edged candidate carried roughly the negation of its true total, and `best` selects on `pessimistic >`, so it could never be chosen — and `pbo` ranks that vector | `crate::outcome::edge(&train_column, &forward, &item.mask)` per candidate, on `train` alone | ✓ |
| WF-03 | **The winner's side travels into the test window unchanged**, exactly as its exit rungs do. Deriving it again from the test bars would fit the side to the data being tested | `chosen_side` carried through `Assessed` → `best` → the out-of-sample pass | ✓ |
| WF-04 | **A fold that decides a side says which.** `chosen_side` is `Some` exactly when `chosen` is, and the walk-forward table carries the column. The same argument `chosen_exit` was added under: a decision nobody can read is a decision nobody can check | the pairing assertion in the same test; `audit::walk_forward`'s `side` column | ✓ |

## A running sweep can be looked at — D-0388, D-0389

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| LV-01 | **Nothing in flight is an empty list, not a refusal.** `live::current` skips a file whose magic, version, header or count does not read, because the directory is transient by design and a half-written file is an ordinary state. A stale file from a previous format is skipped by the same rule | `api::livejson::tests::a_store_with_nothing_in_flight_answers_empty` | ✓ |
| LV-02 | **Asking what is running creates nothing.** The same rule `/engine/top.json` had to be taught | the same test asserts on the filesystem after the call | ✓ |
| LV-03 | **A live row is NOT given a verdict.** `publish_ranked` runs before the exit grid, so `trades`, `wins`, `pessimistic`, `worst_trade`, `max_drawdown`, `min_win`, `gross_win` and `gross_loss` are structural zeros — a `meets` block computed from them reports 0% win rate and FAIL, which is three figures nobody measured wearing the shape of three that somebody did | `api::livejson::tests::a_run_in_flight_carries_its_bar_and_is_not_given_a_verdict` asserts the ABSENCE of `meets` and `win_rate_bp` | ✓ |
| LV-04 | **The bar travels with the rows, and the comparison is on the wire.** A `\|t\|` is a verdict against the trial count, and that count grows while the run is in flight. Both integers are served and `clears_bar` states the answer, so two figures on different scales are never eyeballed against each other | same test: `t_milli` 1802 against `bar_milli` 5673 gives `clears_bar: false` | ✓ |
| LV-05 | **The grid phase brackets itself with one event in and one out.** Entry and exit rather than a single line, so a run that died inside the phase leaves the entry unmatched — which is the reading an operator needs. One per rung, no loop over bars, no loop over candidates | `note` at both ends of `trade_and_screen`; gate 17's four swept crates verified still silent after the change | ✓ |

## An unpriceable path still holds the position — D-0390

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| OV-01 | **A candidate whose path cannot be priced still BLOCKS the next signal.** It was filtered out of the sequence, so it stopped blocking and the next signal became a trade the one-position rule forbids — `trade::walk` takes such a trade and blocks; the grid took the NEXT one and omitted this one | `runner::grid::tests::a_refused_path_blocks_the_next_signal_instead_of_vanishing` measures both readings: the old filter gives 1 trade, the fix gives 0 | ✓ |
| OV-02 | **It is never tallied.** `crossings` cannot say whether a stop was hit on a bar it could not read, so pricing it would invent a path. The block and the tally are separate decisions and only the tally is refused | `blocks_without_pricing` returns before any cell arithmetic | ✓ |
| OV-03 | **The block runs to `time_exit`, the LATEST the position could have closed.** A stop might have released it sooner and the unreadable bar is exactly what would say. Blocking to the time exit can only refuse a later signal, never invent one — the direction an unmeasurable case must err in | the same helper; stated in its doc | ✓ |
| OV-04 | **"Every path refused" still refuses rather than reporting a clean zero.** The emptiness guard meant that only while the filter ran; `all()` returns `true` on an empty vector, so the old case is preserved exactly and the new one covered | `with_levels`' guard, and `one_variant` returning a zero-trade cell is unreachable through it | ✓ |

## A grain that is measured is a grain that is stored — D-0391

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| GR-01 | **The share array is sized from the grain ladder, never typed.** `[i64; 6]` against seven grains made `get_mut(6)` return `None`, and the `if let` discarded the finest grain silently — after paying a full pass over every trade row to compute it | `shares_bp: [i64; crate::stability::GRAINS.len()]`; the past-the-end test indexes `GRAINS.len()` rather than a typed 6 | ✓ |
| GR-02 | **Every grain has a column under its own name.** Five labels over seven values printed Half under "quarterly" and Week under "daily", and Day never appeared at all | a module-level `const` assert ties the hand-written column count to `GRAINS.len()` | ✓ |
| GR-03 | **A derived floor never rounds to zero**, because `Rules::admits` treats zero as *rule off* and `return_over_drawdown` returns 0 for a losing variant — so a zero floor admits every loss on that leg. Measured: three real NIFTY spans truncate to zero | the `ratio <= 0` clamp in `hold_return_over_drawdown_bp`, matching the guard `breakeven_rr_bp` already carried | ✓ |
| GR-04 | **An instrumented phase reaches the surface it was instrumented for.** The events were emitted on a target the console filters out, and without the field it then filters on — dropped twice over, and invisible | `cli.audit` with the rung; the reducer's five states and the table's five renders | ✓ |
| GR-05 | **A file this build cannot read costs only its own half of the report.** A v3 frontier aborted the whole of `cli top`, including the ledger row already in hand, and `/engine/top.json` answered HTTP 400 until the file was deleted by hand | `no_frontier` names which absence it is and the ledger half renders regardless | ✓ |
| GR-06 | **An absent ledger is an answer, not a refusal.** `open_read`'s refusal is right for a caller opening a ledger; `top_at` asks which run is best, and on a store that never recorded one the answer is a sentence — not a line labelled `refused:` whose own text says it is not an error | `top_at`'s fold on the phrase both absence refusals end with | ✓ |

## A horizon is a duration, not a subscript — D-0393

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| HZ-01 | **A hold ends at the last bar inside the horizon's own DURATION.** `entry + h` is a duration only where the bars are contiguous; across the 91-minute halt of 2024-03-02 a fifteen-bar hold ran 104 minutes | `horizon_bar` walks the timestamps against a deadline, not the index | ✓ |
| HZ-02 | **The step is the MEDIAN, so one gap cannot redefine the timeframe.** A mean over 105 bars containing a single 91-minute hole reads as a 1.85-minute bar and the deadline stretches with it | `median_step_micros`; asserted at 60,000,000 µs on the real day's fixture | ✓ |
| HZ-03 | **Contiguous bars are untouched, byte for byte.** A duration bound that also moved the ordinary case would be a behaviour change wearing a bug fix's clothes | the same test asserts `horizon_bar(bars, 0, 15, step) == 15` on the unbroken block | ✓ |
| HZ-04 | **It shortens within the trading day and never across one.** An overnight gap already has an owner in `forced_exits`; shortening there would turn "the data ran out" into "it exited on the last bar", which is a fabricated square-off | the `ist_day(bar) == entry_day` guard, and `a_slice_that_stops_mid_session_fabricates_no_square_off` still green | ✓ |

## The ranking can state the operator's own rule — D-0593

The standing rule is `min(win) >= 3x max(loss)`, and until D-0593 no ranking
stage could express it: `Edge` carried eleven fields and every one was a count, a
sum or a `t`, from which an extremum is not recoverable. These rows are about
closing that, and about the tie-break that would have reopened it.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| AS-01 | **The operator's rule is computable BEFORE the exit grid, not only after it.** `grid::Cell::reward_to_risk_bp` always computed the true min/max ratio, but a `Cell` exists only once an exit grid is built, and the `|t|` cut that decides which combinations reach a grid runs first — so the rule was computable only downstream of the gate it had to pass, and a rare asymmetric winner was cut before anything could notice it satisfied the rule. `Edge` now carries `min_win_paisa`, `max_win_paisa` and `max_loss_paisa`, maintained in `Sides::observe` beside the sums, and `Edge::worst_reward_risk_bp` states the rule at the cut | `runner::rank::tests::the_asymmetry_lens_inverts_the_order_that_t_puts_a_rare_winner_in` asserts 60 paisa over a 20 paisa largest loss is exactly `300` | ✓ |
| AS-02 | **The mean ratio always flatters a dispersed sample, and the test asserts the gap rather than assuming it.** Since `min_win <= mean_win` and `max_loss >= mean_loss`, `worst_reward_risk_bp` is ALWAYS at or below `payoff_bp`; on the pinned grinder the two read 0.12x and 1.11x for the same trades. That difference is the whole reason `payoff_bp` could not stand in for the rule | the same test asserts `grinder.payoff_bp() > grinder.worst_reward_risk_bp()` | ✓ |
| AS-03 | **The lens inverts the order `\|t\|` puts a rare winner in, and BOTH halves are pinned.** A test that only checked the new order would pass just as well if the old one had never been a problem, so the defect is asserted first: under `\|t\|` the grinder outranks the rare asymmetric winner, and under `Asymmetry` the winner comes first | `the_asymmetry_lens_inverts_the_order_that_t_puts_a_rare_winner_in` asserts `g > w` and then `ByAsymmetry(w) > ByAsymmetry(g)` | ✓ |
| AS-04 | **The tie-break is `max_win`, NOT `\|t\|`, and that difference is load-bearing.** Every other lens falls through to `\|t\|`, which is right for them. This key saturates at `i64::MAX` on any sample that never lost, so ties are the common case — and `\|t\|` is `mean / (sd / sqrt(n))`, whose `sd` is inflated by exactly the winners that make a rare setup valuable. Falling to `\|t\|` would separate two never-lost combinations by preferring the one whose wins are smaller and more uniform, undoing the lens. `\|t\|` remains the LAST term so the order is still total under §3 rule 5 | `ByAsymmetry::cmp` orders on `worst_reward_risk_bp`, then `max_win_paisa.total_cmp`, then `Scored::cmp` | ✓ |
| AS-05 | **Both ends of the ratio are reported, never divided.** Nothing lost is `i64::MAX` — a real answer about a real sample, left for the caller to decide about rather than folded into a sentinel here; nothing won is zero, the floor, because a setup that never won is not asymmetric, it is absent, and ranking it above anything would be the fallback §4 bans | `runner::rank::tests::the_asymmetry_ratio_reports_both_ends_rather_than_dividing` | ✓ |
| AS-06 | **The lens is appended as identity term 3 and renumbers nothing.** A lens decides which combinations reach the exit grid, so it decides the answer — §3 rule 3 wants a different identity, and reusing 0, 1 or 2 would collide two runs that swept one span under opposite questions. §3 rule 8 forbids renumbering the three that exist | `cli::lib.rs`'s lens arm, appended below `Lens::Path => 2`; `every_knob_that_moves_the_answer_moves_the_identity` still green | ✓ |
| AS-07 | **Nothing that ran before it changes.** `rank` still passes `Lens::Detectability`, and the three historical orderings are literally the same code — each lens is a newtype so the old ones cannot drift while a new one is edited | `cargo test -p runner --lib` 583 passed / 0 failed; workspace `clippy -D warnings` clean | ✓ |
| AS-08 | **The same question is asked at the cell, not only at the cut — D-0594.** `ExitGridSelectorV1::GuaranteedFloor` prices every win at `min_win` and every loss at `worst_loss`, answering "what if each win had been my worst"; that is honest and conservative and is NOT corrected. For a fat right tail it is the wrong question — the one enormous win is priced as the smallest, so the tail cannot influence the choice and the selector prefers the exit that makes wins UNIFORM. `OperatorRule` ranks on `Cell::reward_to_risk_bp` instead, the cell-level twin of `Lens::Asymmetry`. Neither is the default and no call site changed | the selector arm in `ExitGridPolicyV1::select`; `cli::execution_capability::tests::every_exit_grid_selector_round_trips_and_keeps_its_appended_tag` | ✓ |
| AS-09 | **Every selector codec moved together, and the tags are pinned as literals.** Six codecs carry a selector — `exit_grid_policy::selector_byte`, `execution_v3`, `execution_v4`, `execution_capability` (the only one with a DECODE side), `boolean_candidate_grid` (zero-based where the others are one-based, a pre-existing difference; the numbers are never compared across codecs) and `api`'s JSON label. All were found by the compiler's exhaustiveness check, not by grep. The tags are asserted as literals rather than merely round-tripped, because a round-trip alone still passes if every number shifts by one and §3 rule 8 is about them staying put | `every_exit_grid_selector_round_trips_and_keeps_its_appended_tag` asserts 1/2/3/4 by value and that an unknown tag is refused rather than defaulted | ✓ |
| AS-10 | **A cadence rarer than one trade a week is expressible — D-0594.** `descend`'s `PER_WEEK` was a whole number floored at one, and the refusal guarding that floor was right about ZERO and silent about the UNIT: one a week is fifty-two a year, so the rarest cadence the command could state was already eight times more frequent than the objective. `Cadence` is `PerWeek` or `PerYear`; a bare number keeps its historical meaning bit-identically and `/y` is the new spelling. Zero is still refused on either arm for the reason it always was, and the banner reports the unit GIVEN rather than a normalised one — six a year and zero a week are the same integer and only one was asked for | `cli::tests::a_cadence_rarer_than_one_a_week_is_expressible_and_a_bare_number_is_unchanged` asserts the per-week arm equals the function it replaced at 1/2/7/40, that `PerYear(6)` reaches below `PerWeek(1)`, that zero has no floor on either arm, and every accepted and refused spelling | ✓ |
| AS-12 | **The floor of the wins is the smallest win under the worst reading — D-0602, REVERSING D-0595.** `min_win` is `min(pess)` over trades with `pess > 0`, and nothing else. D-0595 excluded any win no larger than `opt - pess`, arguing such a win "is a win under one admissible ordering and a loss under another". That is false: `pess` and `opt` are the two ENDS of one trade's interval, `pess <= opt` always, so `pess > 0` means the trade won under the worst ordering and therefore under every ordering. The bracket measures uncertainty in the win's SIZE, never its sign. The exclusion raised `min_win` — the numerator of `min_worst_reward_risk_ppm`, the operator's own 3:1 rule — while `accrue_risk` applied no test to losses, so both halves of the ratio moved it UP. This row previously asserted the opposite and cited a test that pinned the defect | `runner::grid::tests::the_floor_is_the_smallest_win_under_the_worst_reading_whatever_its_bracket` — the 20-paisa win IS the floor, and the same pessimistic values with a zero bracket give the identical floor | ✓ |
| AS-13 | **All four folds dropped the bracket test — D-0602.** `min_win` is folded in four places: `grid::tally_trade`, `population_base_evidence_v2::fold_trade_rows` and `institutional_evidence` (these three reconcile and refuse on disagreement), and `index_stop_qualification_metrics` (the single-stop route). **The fourth was missed on the first pass** because it names the variable `win`, not `pess`, so a search for `pess > bracket` could not find it — and it is the copy on the path live sweeps run. It is also the only one NOT reconciled against the others, which is exactly why its copy of the defect survived the reconciliation that caught the rest. This row previously said the four held "the identical bracket test"; they now hold none | all four folds compute `min(worst)` over wins; `the_floor_is_the_smallest_win_under_the_worst_reading_whatever_its_bracket` | ✓ |
| AS-14 | **The counter that would disambiguate `min_win == 0` is deliberately absent.** Zero now means either "nothing won" or "nothing won by more than its own uncertainty". A `scratch_wins` field would separate them and was written, then removed: `TradeAggregatesV2` has `encode_aggregates`/`decode_aggregates` at a FIXED width, so a new field is a new file version at its own stride under §4 and §3 rule 8 forbids mutating a format in place. Documented on the field rather than paid for with a format version nothing else needs yet | the field's own doc comment; no encoder or decoder changed | ✓ |
| AS-11 | **All four baked findings are now closed, and none of them cost an invented number.** D-0594 declined the last two on the ground that each needed a declared magnitude. D-0595 removed that objection — the threshold is the trade's own `best - worst` bracket — and D-0596 applied the same principle to a day. What each repair actually cost was structural, not numeric: three reconciled folds moving together for the trade, and a record version plus a policy version for the day | D-0593 (the lens) · D-0594 (the selector and the cadence) · D-0595 (the trade) · D-0596 (the day) | ✓ |
| AS-15 | **A day whose sign depends on the reading is evidence for neither side — D-0596.** `index_consistency` classified a day by the SIGN of its pessimistic sum alone, so a day netting one paisa counted as much as one netting fifty thousand rupees, and a day that lost a paisa under the worst reading while winning handsomely under the best was filed as a loss. Every `Policy` threshold counts DAYS, so that classification was the entire input, and it VETOES through `combined_qualifies`. `Policy::V2` counts a day a WIN when its pessimistic sum exceeds its own bracket, a LOSS when even its optimistic sum is negative, and a SCRATCH otherwise — falling to the existing zero-day arm, which already preserves the streak as a no-trade day does. No constant is introduced and none of the five thresholds moves | `cli::index_consistency_tests::a_day_inside_its_own_bracket_is_a_win_under_v1_and_a_scratch_under_v2` — a span of one-paisa days against a 200-paisa bracket is every day a winner under V1 and not one under V2, plus the three outcomes separated on one day each | ✓ |
| AS-16 | **V1 is frozen to the byte, and a `BRICDY01` row keeps its meaning exactly.** The literal version word became `self.version()`, which returns 1 for V1, so V1's bytes and digest are unchanged and all 44 `Policy::V1` call sites compile untouched — an enum variant reads the same as the associated const it replaced. A three-word session row decodes with `optimistic_paisa` taken as EQUAL to the pessimistic one (never invented), giving a zero bracket, under which V2's test reduces to V1's sign test exactly. V2 keeps all five thresholds and both rule codes and the record does not grow; only which days reach the counters differs | `the_two_policies_share_every_threshold_and_never_share_an_identity` asserts every threshold equal, the widths equal and the digests different; `a_version_one_session_row_keeps_its_meaning_under_both_policies` decodes a real `BRICDY01` row and runs it under both | ✓ |
| AS-17 | **Decode gained two refusals rather than two coercions, and one immediately caught a fixture.** An optimistic reading below the pessimistic one is refused, not sorted — the two are ends of a measurement interval and the one called pessimistic must be the low end. A no-trade day may not carry a return on EITHER side; the old check tested only one, and the stricter version failed `loss_streak_crosses_training_boundary_and_flat_days_do_not_reset_it` on the spot, where the fixture had zeroed a no-trade day's pessimistic sum and left its optimistic one at 100 | `a_version_one_session_row_keeps_its_meaning_under_both_policies` asserts both refusals on hand-built records | ✓ |
| AS-18 | **The boolean route is unchanged by V2 rather than silently degraded by it.** `BooleanSessionV1` carries `return_paisa` alone and has no second reading; inventing one would be worse than not having it. Its sessions are built with the two equal, giving a zero bracket, under which V2 reduces exactly to V1. Making that route magnitude-aware is its own record's version to bump, and the call site says so | the comment at the `boolean_qualification_v1` construction site; V2 selects nothing yet, so no route changes today | ✓ |
| AS-19 | **V4 counts a day by its sign under BOTH readings — D-0605.** Positive under the worst reading is a win under every reading, negative under the best is a loss under every reading, anything else is neither. V2/V3's bracket test filed small certain wins as neither; V1-V3 keep their exact rules, chosen by the version word | `cli::index_consistency_tests::v4_counts_a_day_by_its_sign_under_both_readings` — seven (pessimistic, optimistic) pairs under V3 and V4, each round-tripped | ✓ |
| AS-20 | **V4 divides only by decided days, and no week or streak can fail it — D-0605.** One win in four decided days is exactly the floor; a fourth loss, or a span with no decided day at all, fails the ratio and nothing else. The same days under V3 fail its ratio and its weekly minimum — the rule that rejected all 8,000 settings of the first live batch | `v4_ratio_counts_decided_days_and_no_week_or_streak_rule_can_fail` | ✓ |
| AS-21 | **Every approved version keeps its exact bytes, rule and identity, and `decode` IS the approved list.** Words pinned as literals for V1-V4, four distinct digests, `from_digest` and `decode` recover each, `INDEX_STOP` is V4. `decode` was a hand-written chain that would have refused a fourth version everywhere it was read back | `every_approved_policy_keeps_its_exact_bytes_rule_and_identity` | ✓ |
| AS-22 | **V4 refuses a row whose readings are inverted, or that trades nothing yet carries a reading; V3 answers exactly as before.** V4's win and loss tests exclude each other only on an ordered pair | `v4_refuses_inverted_or_idle_readings_that_v3_never_checked` | ✓ |
| AS-23 | **Each served day carries its class under its record's own rule, and the policy description carries the rule and basis.** The browser labelled days by V1's sign test because nothing else was sent; the description gains `day_rule` and `ratio_basis` (14 keys, pinned) and each assessment carries its own description | `api::index_consistency_projection::tests::each_day_is_classed_under_its_own_record_s_rule`; `policy_descriptor_reads_the_canonical_rust_policy_and_exact_scope` | ✓ |
| AS-24 | **The browser checks structure, never a version's numbers.** `index-consistency.js` re-applied V1's 60%, 2-day streak and 3-win/2-loss week to every record, which would have broken the page on the first V3 pass; rule text now comes from the served description through one function | `web/tests/index-consistency.test.js`: "the rule text is read from the served policy…", "a saved assessment carries the exact rule its digest names" | ✓ |

## Ranked retention changes memory, not the search — D-0394

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SR-01 | **The retained and streamed doors are one Apriori walk, level for level and byte for byte.** Each retained `Frontier` becomes the identical `Tally`; exclusions, bars, threshold, halt, completion and extinction depth agree; every survivor reaches the streamed callback once in retained order; and the level reporter fires at the same boundary, including k=1. The shared body is engine `Ladder::walk_into`, not runner `fold_and_walk` | `engine::tests::a_streamed_walk_and_a_retaining_walk_are_the_same_walk` compares every level's five-exit reconciliation, the full ordered survivor byte stream and every result field | ✓ |
| SR-02 | **A halt stays the same halt, and its partial level is not lost.** Streaming cannot turn a budget breach into extinction merely because the loop breaks before an ordinary next level | `engine::tests::a_streamed_walk_that_halts_reports_the_breach_and_the_partial_level` compares the retained and streamed `Halt`, requires both level records and observes halted k=2 at the callback; `engine::keep::a_tally_reconciles_exactly_when_its_frontier_does` pins the five-exit identity after survivor vectors are gone | ✓ |
| SR-03 | **A retention cap cannot choose a depth.** Cap zero and a cap wider than the answer reach the same levels, counters, depth, bars, threshold, halt and `streamed` total; the difference is only held versus loudly discarded rows | `engine::keep::a_cap_of_zero_and_a_generous_cap_walk_the_same_ladder` uses a climbing fixture deeper than two and asserts both sides, including `discarded == streamed` at zero | ✓ |
| SR-04 | **The current `Streamed` value contains one fixed record per level and no survivor vector.** The search still holds its adjacent frontiers while joining; what is removed is every already-retired level from the returned result. **PARTIAL PROOF:** the returned type has the claimed shape, but the test does not measure allocator traffic or peak RSS and cannot by itself stop a future field from retaining survivors | `engine::keep::a_streamed_walk_retains_no_survivor_at_all` checks the present result shape and reconciliation. A structural field/size gate plus allocation or peak-RSS scaling at 1×/10×/100× survivors is still required for a memory-bound assurance | ◐ |
| SR-05 | **Incremental ranking is the retained rank under both lenses, not one cut per level.** At `keep = 0` and a binding cap, retirement produces the same global `top`, `closed_top`, redundant count, closure verdict and considered count as ranking the retained `Sweep` afterwards under both Detectability and Payoff | `runner::rank::an_incremental_ranker_matches_the_retained_ranker_under_both_lenses` | ✓ |
| SR-06 | **Every frequent survivor is accounted exactly once before discard; with a positive retention cap, every survivor is scored before the cut.** `keep == 0` deliberately counts without computing an edge because no score can be retained; claiming that arm scores would be false. `Ranked::considered == Streamed::streamed`; the top is bounded by `keep`; and the same-series streamed rank equals the retained rank while every retained row remains paired to its own `Forward` | `runner::tests::a_ranked_run_agrees_with_a_plain_one_and_never_mispairs` proves accounting, bounds and pairing on a non-empty positive-cap fixture; `runner::rank::keeping_zero_weighs_everything_and_retains_nothing` proves the count-only zero-cap contract | ✓ |
| SR-07 | **Adjacent-level closure is exact, and historical selection remains rank-all → top-N → closure-filter with no backfill.** An equal-support immediate superset marks the same rows redundant as the retained whole-sweep pass, and `closed_top` is the same ordered subset of `top` | `runner::rank::an_incremental_ranker_matches_the_retained_ranker_under_both_lenses` compares `closed_top`, `redundant` and `closure_complete` against retained `rank_by` under both lenses and both caps | ✓ |
| SR-08 | **Raw and effective trial counts equal the retained formulas:** `Σ(survivors + infrequent)` and that raw count minus exact adjacent-level redundancy. `Streamed::streamed` is only the frequent half and can never stand in for raw hypotheses | `runner::tests::a_ranked_run_agrees_with_a_plain_one_and_never_mispairs` compares streamed `RankedOutcome::{trials,effective_trials}` directly with `significance::{trials,effective_trials}` on the retained `Sweep` built from the same bars | ✓ |
| SR-09 | **A projected run scores every survivor on the execution column before discard, under both lenses.** The hard cut and its closure-filtered subset must equal ranking the retained signal sweep directly against that execution column; a signal-ranked prefix is not an admissible substitute | `runner::tests::prepared_projected_ranking_matches_a_retained_execution_ranking` calls `run_prepared_ranked_by_reporting` and compares `top`, `closed_top` and `considered` with the retained execution-series rank under Detectability and Payoff | ✓ |
| SR-10 | **A halted ranked walk is reported but never traded or recorded.** A partial successor certifies no closure, so `closure_complete` and `RankedOutcome::is_complete` are false and the operator stops before every downstream stage | `runner::tests::a_halted_ranked_sweep_cannot_certify_closure` pins the result state and `considered == streamed`; `cli::tests::a_halted_streamed_audit_stops_before_every_downstream_stage` requires REFUSED / NOT TRADED and the absence of trades, exit grid and result record | ✓ |

## Session, execution and runtime policy — D-0395 through D-0399

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SB-01 | **Every swept intraday execution uses the fixed 15:10 IST liquidation deadline, and only the exact unique accepted one-minute record stamped 15:09 can price it.** That left-labelled record closes at the deadline; 15:10 is post-deadline. The rule is a product policy rather than a session-close inference and therefore applies unchanged to regular and extended evidence | `runner::outcome::tests::every_session_uses_fixed_1510_and_only_exact_1509_prices_it`; `runner::trade::tests::the_forced_exit_is_fixed_at_1510_and_requires_the_exact_1509_bar`; `runner::trade::tests::the_1509_row_prices_the_forced_fill_and_the_1510_row_is_unreachable` | ✓ |
| SB-02 | **Neither a neighbouring minute, slice end nor a later day proves the fixed forced-fill record.** Missing, refused, duplicated or otherwise ambiguous 15:09 data fabricates no price; a non-regular session without that row drops a hold needing forced liquidation, while an earlier exact horizon may remain measurable | `runner::outcome::tests::missing_corrupt_or_ambiguous_1509_never_fabricates_a_forced_exit`; `runner::trade::tests::completeness_not_a_later_day_proves_a_session_ended`; `runner::outcome::tests::a_coarse_rung_prices_exact_horizons_but_not_an_unproved_square_off` | ✓ |
| XP-01 | **Per-slice timing and session facts are derived once outside candidate/grid loops.** Candidate evaluation consumes `SliceFacts` and performs no timestamp sort or session-bound derivation | `runner::trade::tests::the_per_candidate_walk_derives_nothing_and_sorts_nothing`; `runner::grid::tests::a_grid_over_hoisted_slice_facts_is_the_same_grid` | ✓ |
| XP-02 | **Derived trade floors and ranking are always measured on the actual one-minute execution slice, independent of the signal rung.** A native `1min` command self-aligns the one stored slice; every coarser shipping audit must load the matching execution slice or refuse before pricing. The no-second-slice helper remains valid only for the native one-series case, not as permission to trade coarse OHLCV | `cli::derived_floor_tests::the_derived_floors_do_not_move_with_the_signal_rung`; `cli::derived_floor_tests::with_no_execution_series_the_floors_stay_on_the_swept_bars`; `cli::derived_floor_tests::a_native_minute_path_self_projects_and_a_coarse_path_without_execution_refuses`; `cli::tests::the_stored_audit_banner_names_the_rung_its_trades_filled_on` | ✓ |
| KF-01 | **An unusable runtime knob is named before any fallback is applied.** Empty text clears a request; malformed, zero or out-of-range counts are distinct and visible refusals | `cli::knobs::tests::an_unusable_count_is_named_rather_than_silently_defaulted`; `cli::tests::a_knob_this_run_could_not_use_is_named_beneath_the_banner` | ✓ |
| KF-02 | **The resolved exit-grid rung count is one value shared by execution, walk-forward and run identity.** A request-local value cannot change the grid without changing the identity or be re-read differently inside a fold | `cli::tests::request_local_grid_rungs_reach_the_fold_and_identity_as_one_value`; `runner::validate::tests::an_explicit_rung_count_changes_no_other_walk_forward_semantics` | ✓ |
| TI-01 | **Temporary credential fixtures cannot collide across calls or simultaneous test binaries.** Their path contains the process id and an atomic per-process serial | `pull::unit::temporary_configuration_paths_are_process_local_and_unique`; four simultaneous complete `pull --test unit` processes were also measured green during D-0399 | ✓ |

## The comparison keeps every record and crowns none — D-0400

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| BT-19 | **A run that cannot be ranked remains available and receives no rank.** Integrity-seal failures, halted searches, non-signal timeframes, partial spans, zero-trade runs, contradictory fill totals and missing or invalid numeric results each receive one explicit status; the rankable rows move ahead of them, but no row is dropped from the ordered result and the excluded rows keep their ledger order | `web/tests/comparison.test.js` · *every exclusion has one explicit status and damaged data wins precedence* · *invalid counts and absent risk results cannot slip into ranking* · *ineligible rows remain visible in their original order and keep honest null gaps* | ✓ |
| BT-20 | **Rank is the descending pessimistic total, and `#1` is a reference among the displayed records rather than a promise.** The optimistic total cannot move the order; ledger index breaks an equal pessimistic result deterministically; only the first rankable row receives `reference: true`, while excluded rows receive neither a rank, a reference claim nor deltas. The page calls these recorded totals rather than like-for-like strategy scores because feed, instrument, rung, span and settings may differ | `web/tests/comparison.test.js` · *rankable rows lead by pessimistic result with stable ledger-index tie breaking* · *deltas are only against the best rankable reference*; `/backtest` labels it “Reference” and states that it is not a promise | ✓ |

## The immediate minute is exact and an exit minute stays closed — D-0401

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| XM-01 | **A signal aligns only to the one-minute bar whose open timestamp exactly equals the signal bar's close timestamp.** The first later available bar is not a delayed fill and cannot substitute for a missing immediate minute | `runner::align::tests::a_signal_lands_only_on_the_execution_bar_at_its_exact_close` | ✓ |
| XM-02 | **A hole at that timestamp makes the signal unreachable, visibly.** Two signal closes at minutes 15 and 30 against execution bars 0–9 and 40 produce two `None` mappings and `unreachable == 2`; neither is silently moved to minute 40 | `runner::align::tests::an_execution_gap_makes_the_immediate_fill_unreachable` | ✓ |
| XM-03 | **One execution minute cannot both close the existing position and open its successor.** Exclusivity compares actual fill indices, not signal indices, and requires `next_entry > previous_exit` for a reprojected fill series | `runner::trade::tests::a_reprojected_column_enters_on_the_bar_it_was_told_to` | ✓ |
| XM-04 | **Every exit-grid variant applies the same strict boundary.** A candidate whose fill equals the previous exit is blocked; the immediately following fill remains eligible | `runner::grid::tests::an_exit_bar_cannot_also_be_the_next_candidates_entry_bar` | ✓ |

## Browser engine task lifecycle — D-0403

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| BE-01 | **A browser-started sweep, descent or stored-data command cannot leave the shared engine slot in flight after its background task unwinds or is dropped before starting.** One armed `TaskFinisher` is created before each `spawn_blocking`; abnormal drop stamps the existing progress finished and files an explicit refusal, while normal completion writes its answer and disarms | `api::sweeprun::tests::a_panicking_engine_task_becomes_a_visible_refusal_and_releases_the_slot`; `api::sweeprun::tests::all_three_browser_engine_tasks_arm_and_disarm_the_same_finisher` | ✓ |
| BE-02 | **A finisher belonging to an old task cannot paint over a later task.** Normal completion disarms its guard before destruction, and an armed guard declines to rewrite a slot that is already finished | `api::sweeprun::tests::a_normal_finisher_preserves_its_answer_and_cannot_abort_the_next_run`; `api::sweeprun::tests::an_armed_guard_does_not_rewrite_a_slot_that_is_already_finished` | ✓ |

## An unpriceable exit still owns the position — D-0406

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| OX-01 | **A priceable entry followed by an unpriceable exit remains occupied through its conservative time exit, for both long and short.** It contributes no `Trade` or money, but no successor may enter on or before that exit bar | `runner::trade::tests::an_unpriceable_exit_still_occupies_both_long_and_short_walks` injects a corrupt exit after a valid entry, checks the ordered block-only interval, checks both directions, and requires every later taken entry to be strictly after it | ✓ |
| OX-02 | **Every exit-grid variant consumes the same ordered occupancy stream.** A block-only path is never tallied and still suppresses an otherwise clean overlapping successor; removing that path makes the successor trade, so the proof is not vacuous | `runner::grid::tests::an_exit_unpriceable_path_blocks_the_next_signal_in_every_grid_walk` compares the complete sequence with the same sequence after its block-only predecessor is removed | ✓ |

## Master replacement is serialized and durably published — D-0405

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| ML-01 | **Two cooperating refreshes of the same master cannot share its partial replacement.** A persistent per-target inode is locked before the old target is compared and held until replacement settlement; another caller waits, while different source filenames map to different locks | `pull::masters::tests::a_second_same_source_landing_waits_for_the_first_sources_lock`; `pull::masters::tests::every_source_has_its_own_persistent_lock_inode` | ✓ |
| ML-02 | **`Written` means the new file bytes were synced before rename and the containing directory was synced after rename.** A post-rename directory-sync refusal is represented separately as `Landed::Uncertain`, carrying byte count, change flag and cause; it is not mislabeled as either a durable write or an untouched old file | `pull::masters::tests::an_unchanged_master_lands_and_says_it_did_not_change`; `api::mastersrun::tests::a_refusal_reaches_the_page_as_a_sentence_and_a_written_false` checks that a normal write renders `durable:true` and an uncertain replacement renders `written:false`, `durable:false`, its byte count and its cause | ✓ |
| ML-03 | **An ordinary completed landing leaves no partial file, and a body refused by validation leaves the target as the old whole file.** Cleanup after a failed local write is best-effort and its uninjectable failure branch is named in §99 rather than claimed by this proof | `pull::masters::tests::no_partial_file_survives_a_landing`; `pull::masters::tests::a_short_body_is_refused_rather_than_written_over_a_good_master` | ✓ |

## Raw vendor evidence is exclusive and its loss is observable — D-0407

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| VC-01 | **A process restart can never open an earlier capture for writing.** PID and a once-per-process timestamp only spread names; `create_new` is the authority, so a collision advances within a fixed candidate set and the old bytes remain byte-identical | `pull::capture::tests::a_restart_collision_creates_a_new_file_and_never_overwrites_the_old_one` | ✓ |
| VC-02 | **Concurrent writers of the same capture prefix either receive distinct exclusive files containing their own bytes or refuse; they never overwrite or exchange bodies.** Collision handling has exactly sixteen attempts and therefore cannot turn a hostile directory into an unbounded scan | `pull::capture::tests::concurrent_writers_get_distinct_exclusive_names_and_keep_their_own_bytes`; `pull::capture::tests::exhausting_every_bounded_name_refuses_without_touching_any_capture` | ✓ |
| VC-03 | **Every capture failure is counted even though it never changes the pull outcome, and it emits one searchable `pull.capture` error without copying the URL, body or any header into telemetry.** This includes failure to resolve the capture root before a file is opened | `pull::capture::tests::a_root_that_cannot_be_written_is_counted_rather_than_hidden`; `pull::emit_sites::every_emit_site_in_this_crate_reaches_a_file` drives the shipped recorder and finds `vendor capture could not be written` with feed `truedata` | ✓ |

## Every recorded calendar grain and streak reaches the dashboard — D-0408

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| BT-21 | **None of the eight Rust period grains is discarded by the browser.** Hour, weekday, day, week, month, quarter, half-year and year each retain a chronological chart series; integer keys decode to the exact UTC/IST range, civil date, epoch-aligned seven-day block or civil period they name | `web/tests/trade-analytics.test.js` · *all eight Rust period grains remain available to the page* · *integer period keys receive exact, honest labels* | ✓ |
| BT-22 | **Every consecutive result run survives with its count and paisa sum in resolution order.** A flat trade breaks a win and joins the losing run, matching `runner::grid::tally_trade`; the distribution may still classify that same zero separately as breakeven | `web/tests/trade-analytics.test.js` · *streaks are ordered by seq and preserve every run for count and amount charts* · *flat trades break wins and extend losing streaks exactly like the Rust grid* | ✓ |
| BT-23 | **Dense analytics cannot put an unbounded series into the DOM.** Page indices clamp at both ends; a time chart admits at most 24 buckets and a streak chart at most 96 per-trade running steps, while the complete durable series and its total remain represented. Every page of one chart retains the complete series' vertical scale | `web/tests/trade-analytics.test.js` · *dense histories admit only a fixed window to the DOM and clamp every page edge*; `web/tests/backtest-reference.test.js` · fixed chart-window and full-series-scale source contracts | ✓ |

## Comparison authority and exact browser integers — D-0409

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| BT-24 | **Signal-rung membership comes from the engine, not a browser spelling rule.** Every `/backtest.json` response publishes the exact `cli::EVERY_RUNG` values; `15min` is rankable when present, while `0min`, `garbagemin`, `1day` and absent or malformed authority cannot enter the comparison | `api::backtest::tests::the_json_publishes_the_exact_engine_signal_rungs`; `api::backtest::tests::an_unresolvable_store_root_answers_503_with_the_shape_the_page_parses`; `web/tests/comparison.test.js` · *signal membership comes from the exact wire set, never a min suffix guess* | ✓ |
| BT-25 | **No browser comparison formats or ranks a rounded integer.** Every ranked count and money scalar, the optimistic-minus-pessimistic gap and every reference delta must be a JavaScript safe integer. Unsafe raw values or derived differences remain visible with no rank and no fabricated delta | `web/tests/comparison.test.js` · *unsafe money and fill gaps are excluded instead of rounded* · *a delta wider than the exact integer range cannot enter ranking* | ✓ |

## The Backtest report distinguishes unavailable data from access control — D-0410

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| BT-26 | **No unavailable Backtest value is rendered as a padlock.** A padlock communicates permission or a paywall, while this report owns missing evidence, undefined arithmetic and inapplicable account metrics. Compact cells use an em dash, larger values say “Unavailable”, and both preserve the exact reason for pointer and assistive readers. Known facts remain facts: statutory commission is `0.00%` for the spot-index scope; margin is not applicable; unrecorded per-trade price and excursion columns are omitted; immutable testing periods are disabled and say a new run is required | `web/tests/backtest-ui.test.js` · *unavailable data is neutral evidence, never a padlock or permission claim* · *known commission is shown as zero and its unmodeled spread stays visible* · *the trade ledger contains only durable per-trade fields* · *recorded-period controls cannot pretend to mutate an immutable run* | ✓ |
| BT-27 | **Selection and focus are separate states in the reference-matched drill-down.** The expanded analysis owns a local dark ramp, loss red and profit teal without changing the surrounding console theme; an active report tab is a light filled pill, while blue remains the keyboard focus ring. At phone width the four-column statistic row becomes one column and wide tab/table content scrolls inside its own surface rather than widening the page | `web/tests/backtest-ui.test.js` · *the drill-down owns the reference dark ramp and selection is not confused with focus*; live internal-browser checks at desktop and phone widths | ✓ |
| BT-28 | **Every backtest input has a stable unique control name, and the drill-down heading tree does not skip a level.** Dynamic names derive from the engine-knob or ranking-weight key; the open-run `h2` owns `h3` panels whose own subsections are `h4`, with the existing visual classes unchanged | `web/tests/backtest-accessibility.test.js` · *every backtest input has a stable submitted-control name* · *the drill-down heading tree advances one level at a time* | ✓ |
| BT-32 | **An enabled drill-down control must change visible evidence, and its exclusive state must be machine-readable.** Benchmark scale controls that changed no data are absent; chart ranges remain disabled until the chart handle exists; empty streak modes are disabled; report, plot, split, period, time-grain and streak selectors expose `aria-pressed`. Time and streak bars expose accessible image names, the fixed trade order declares `aria-sort="descending"`, opening focuses the report, and closing restores the opening control. The details table contains only `Metric | This run` in source—not hidden Long/Short placeholders—and every remaining unavailable value owns a specific reason | `web/tests/backtest-ui.test.js` · *the details table owns one recorded run and no hidden directional wall* · *every enabled drill-down selector changes evidence or exposes its state* · *dense chart and fixed-order table evidence is assistive-technology-readable*; live internal-browser exercise of every tab, scale, pager, view and disclosure | ✓ |

## A result becomes public only as one complete set — D-0404

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| RS-01 | **Frontier and trade blocks, then one sealed fixed-stride receipt with both exact row counts, are prepared and synced before the ledger commit marker; their directory entries are confirmed before that marker is attempted.** The process-wide mutex and persistent writer inode span the complete sequence, not one file at a time | `cli::tests::the_result_set_commit_marker_is_called_after_both_detail_preparations`; `cli::tests::the_result_set_writer_lock_spans_competing_handles`; `cli::result_set::tests::two_stale_handles_converge_on_one_exact_receipt` | ✓ |
| RS-02 | **A prepared child or receipt with no ledger parent is invisible to read-only callers.** An exact rerun reuses each only after every child row and both receipt counts compare equal, then appends one ledger row; repeating that recovery adds no child, receipt, or marker | `cli::tests::a_pre_ledger_crash_is_hidden_then_resumed_byte_for_byte`; `cli::result_set::tests::zero_counts_are_first_class_and_exact_reuse_adds_no_byte`; `api::frontierjson::tests::a_ranked_row_carries_its_verdict_and_the_rules_it_was_judged_against` includes receipt then last-written parent | ✓ |
| RS-03 | **Different child bytes or receipt counts under one identity, a torn child/receipt tail, and a corrupted receipt seal all refuse before the ledger.** Neither state is overwritten, padded, truncated or presented as recovered success | `cli::tests::a_mismatched_prepared_block_permanently_refuses_the_commit`; `cli::tests::a_torn_prepared_tail_blocks_every_later_commit`; `cli::result_set::tests::a_count_mismatch_is_never_rewritten`; `cli::result_set::tests::a_torn_tail_and_a_corrupt_seal_are_both_refused` | ✓ |
| RS-04 | **Zero detail rows are explicit receipt values, never inferred from unrelated ledger aggregates or a missing pathname.** A committed parent may have non-zero streamed combinations and chosen-grid trades while both detail counts are zero; either count may also be zero while the other is non-zero. A missing child is empty only when its sealed count is zero | `cli::tests::legitimate_empty_details_are_proved_by_the_receipt_not_inferred_from_totals`; `cli::tests::a_receipt_distinguishes_each_zero_from_the_other_nonzero_child`; `api::frontierjson::tests::a_deleted_committed_frontier_refuses_unless_its_receipted_count_is_zero`; `api::trades::tests::a_deleted_committed_trade_file_refuses_unless_its_receipted_count_is_zero` | ✓ |
| RS-05 | **A committed receipt must match the indexed child count, and every receipted row must retain its seal and identity.** Any mismatch, foreign row, seal damage, content shortfall or whole-file loss with a positive count refuses the entire public child; a valid prefix or fabricated empty list is never exposed as the complete answer | `cli::tests::a_committed_receipt_count_mismatch_is_a_loud_incomplete_result`; `cli::tests::committed_corruption_exposes_no_partial_frontier_or_trade_list`; the two deleted-child API tests in RS-04 | ✓ |
| RS-06 | **A committed legacy row without a receipt is unverifiable, whether its detail block is present, absent, or an aligned prefix.** It is refused rather than guessed complete or empty; history stays unchanged and only an exact rerun may append the missing receipt | `cli::tests::a_legacy_parent_with_missing_children_is_never_rendered_as_empty` | ✓ |
| RS-07 | **One result-set persistence attempt emits exactly one bounded correlation event after its ledger-last outcome settles.** Written and byte-verified-reused commits carry identity, commit state, selected direction, `chosen-grid-v1`, exact frontier/trade row counts and the stable zero-based ledger index; every refusal carries the same identity and its bounded reason. No success event can precede the parent marker and no bar/candidate loop reaches either event | `cli::tests::result_set_audit_events_are_bounded_complete_and_typed`; `cli::tests::a_verified_rerun_retains_its_nonzero_ledger_index`; `cli::tests::the_result_set_commit_marker_is_called_after_both_detail_preparations` | ✓ |

## Drill-down decoding is exact or visibly refused — D-0411

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| BT-29 | **One unsafe trade fact, period fact or derived integer refuses the complete analytics payload before any chart or table can use it.** Every required field and all eight period arrays are safe integers; row count, bar duration, fill ordering, bucket counts and per-grain totals agree; equity, streak, duration, gross-result, drawdown, swing, period and calendar-slot arithmetic remains exact. A refusal exposes no prefix, while an explicitly absent or existing-empty file remains an honest empty state | `web/tests/trade-analytics.test.js` · *a complete safe trade payload admits every row and every Rust period grain* · *legitimate empty trade states stay distinct from malformed partial analytics* · *one unsafe or malformed trade integer refuses the whole payload* · *safe endpoints whose fold would leave the exact range are refused before analytics* · *all eight period arrays and all integer bucket facts are mandatory and checked* · *period and slot sums cannot cross the safe boundary through individually safe buckets* · *the largest safe integer remains admissible when every fold stays safe*; `web/tests/backtest-ui.test.js` · *trade analytics is admitted before publication and a refusal is visible across the report* | ✓ |
| BT-30 | **A condition mask decodes all six canonical `u64` words or none of them.** Invalid length, syntax, type or range returns no positions and a visible reason; six zero words are a valid empty combination, not an error; `u64::MAX` reaches bit 383 without conversion through `Number` | `web/tests/mask.test.js` · *six canonical u64 words decode little-endian without Number rounding* · *an empty canonical mask is valid and different from an undecodable mask* · *mask decoding is all or nothing across length, syntax, type and u64 bounds*; `web/tests/backtest-ui.test.js` · *every mask display uses the all-or-nothing decoder and names refusal* | ✓ |
| BT-31 | **Every integer setting printed in the human comparison is exact and internally bounded.** Unsafe months, span parts, bars, hits, enumerations, depth, trades or index make the row unrankable before an ordinary stopped/complete label can disguise the defect; the cells say “not exact” or “span not exact” rather than formatting a rounded double | `web/tests/comparison.test.js` · *every count printed by the comparison must be exact before any other status*; `web/tests/backtest-ui.test.js` · *unsafe comparison counts render refusal words rather than finite-number formatting* | ✓ |

## A damaged month can never become a missing month — D-0412

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| LS-01 | **Only `StoreError::Missing` may become a named hole in a multi-month span.** A live writer, torn or malformed file, checksum/record refusal, permission fault and every other store error abort the whole load with its original reason; none may be swept as a shorter sample under `Span::missing` | `cli::stored::tests::a_corrupt_middle_month_refuses_the_whole_span_instead_of_becoming_missing` tears February between readable January and March and requires a whole-span refusal; `a_month_the_store_lacks_is_named_and_the_span_continues` pins the sole recoverable case | ✓ |

## A stored file does not widen the engine surface — D-0413

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| LS-02 | **Every one-month and multi-month stored sweep admits exactly `InstrumentKey::SWEPT` and refuses every other valid stored instrument before opening its bytes.** Storage eligibility is not sweep eligibility: `NSE-INDIAVIX` remains reference-only even when a complete bar file exists | `cli::stored::tests::an_existing_reference_index_is_refused_before_its_store_file_is_read` seeds readable INDIAVIX bars and requires the core `NotSweepable` refusal; `exactly_the_two_core_sweep_keys_cross_the_stored_loader_guard` proves NIFTY and BANKNIFTY reach the store while no duplicated CLI allow-list decides it | ✓ |

## A drill-down response belongs only to the run that requested it — D-0415

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| BT-33 | **Frontier and trade responses publish only while both their request ticket and open run identity still match.** Opening B invalidates pending A independently in each stream; closing invalidates both. A late success or failure can therefore change neither B nor the closed idle state | `web/tests/request-gate.test.js` resolves B before A, resolves A after close, and rejects stale A after B succeeds; `web/tests/backtest-ui.test.js` · *drill-down detail responses are generation and identity guarded* proves both fetch paths and the close path use the gate | ✓ |

## Browser engine bodies are one strict JSON object — D-0416

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| BE-03 | **A sweep, descent or stored-command request is one complete typed JSON object before any default or engine state is reachable.** Non-object text, trailing bytes, truncated or mixed-type rung arrays, explicit nulls and duplicate known fields are refused as one body; no valid prefix survives and a bad/absent rung list cannot widen to every timeframe. Escaped strings and valid unknown fields retain compatibility | `api::sweeprun::tests::malformed_non_object_and_truncated_json_never_reaches_semantic_defaults`; `duplicate_known_fields_are_refused_before_any_engine_work`; `null_or_wrong_typed_known_fields_are_refused_not_treated_as_absent`; `escaped_strings_and_unknown_fields_keep_valid_request_compatibility` | ✓ |

## Engine completion logging never claims an unproved write — D-0417

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| AL-01 | **Sweep, descent and stored-command completion share one outcome classifier and no completion message says a ledger row exists.** A report logs `Info/report`; a refusal logs `Warn/refused` with its reason; both or neither logs `Error/invalid`. Operation, feed, underlying and elapsed time remain searchable | `api::sweeprun::tests::completion_audit_never_invents_a_ledger_commit`; `contradictory_or_missing_completion_state_is_an_error`; `api::emitted::every_reachable_emit_site_puts_a_record_in_the_file` drives the shipped refusal event and checks operation, outcome and reason | ✓ |

## The unauthenticated HTTP surface is loopback-only — D-0418

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| HS-01 | **No command-line spelling can make this unauthenticated server listen on a non-loopback address.** IPv4 addresses in `127.0.0.0/8` and IPv6 `::1` are admitted; IPv4/IPv6 unspecified, public and IPv4-mapped addresses refuse before a socket is opened, and no bypass mode exists | `api::server::tests::the_command_line_is_parsed_or_refused_and_never_guessed` | ✓ |
| HS-02 | **Origin refusal and body-size refusal both precede every mutating parser, without a false universal ordering claim.** An oversized cross-origin write answers `403` in the origin middleware; an otherwise admitted body over 8 KiB answers `413` during body extraction. Neither reaches the form/JSON parser or engine state | `api::server::tests::a_cross_origin_write_is_refused_and_this_page_s_own_is_not`; `a_body_larger_than_this_server_reads_is_refused_and_never_parsed` | ✓ |

## Frontier schema bytes decode strictly or not at all — D-0419

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| FD-01 | **A valid row seal cannot bless an unknown frontier schema.** Direction is exactly `0=long` or `1=short`, every reserved row byte is zero, and any other sealed value refuses. The sealed identity remains indexed so recovery finds the damaged block, exposes no decoded prefix, and cannot append a duplicate block around it | `cli::frontier::tests::sealed_unknown_direction_and_reserved_row_bytes_are_refused`; `a_sealed_invalid_row_blocks_read_and_duplicate_recovery` | ✓ |
| FD-02 | **Frontier header bytes `12..16` are zero for version 4.** A non-zero reserved byte refuses both writer/recovery and read-only opens before any row is indexed or written | `cli::frontier::tests::a_nonzero_reserved_header_byte_refuses_the_file` | ✓ |

## An exchange timetable never invents a spot-index publication timetable — D-0420

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| P-71 | **2021-02-24 has a verified 220-minute NSE normal-market session, but a generic NIFTY/BANKNIFTY/VIX audit makes no claim about how many spot-index bars were published that day.** SEBI records both the two trading windows and NIFTY computation unavailable during part of the still-trading morning; it does not supply one common publication window for all three indices. The exchange calendar therefore cannot shrink to the store's shared 54-bar prefix—or become closed when the damaged daily rung omits the date—while the index audit and ingest dashboard mark the complete index month unmeasured instead of reporting either zero loss or 166 invented vendor holes | `pull::calendar::tests::every_irregular_session_holds_the_bars_its_evidence_proves`; `pull::calendar::tests::the_derivation_uses_the_sourced_outage_and_agrees_on_other_irregular_days`; `pull::calendar::tests::a_missing_daily_outage_row_cannot_reclassify_the_verified_session_as_closed`; `pull::gaps::tests::the_outage_day_separates_market_minutes_from_unverified_index_bars`; `api::calendar_of::wire::the_outage_day_publishes_market_minutes_and_refuses_index_minutes`; `api::server::an_index_audit_keeps_the_outage_day_unmeasured`; `web/tests/calendar-owed.test.js` | ✓ |

## One selected candidate, cell and durable trade stream — D-0414

These rows supersede BT-26's narrower statement that per-trade excursions are
unrecorded. Entry/exit prices and an exit-cause tag remain absent; direction,
MAE and MFE are now durable chosen-grid facts.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| CG-01 | **Only the first final admitted screen row can become the ledger/detail winner.** Evidence rank one cannot displace a lower-ranked passing candidate after consistency demotion; if no row is admitted, no selected result is recorded. Each cascade tier replaces the prior priced map, so an unvisited earlier-tier mask cannot be relabelled with later rules | `cli::tests::a_failed_evidence_rank_one_cannot_displace_the_first_final_admitted_row`; `a_later_tier_with_a_smaller_cap_drops_unvisited_earlier_cells` | ✓ |
| CG-02 | **Chosen rows replay the exact selected stop/target/live-trail/armed-trail variant and its own one-position exclusivity.** The replayed complete `Cell` must equal selection, while an independent row fold must equal count, both P&L sums, worst trade and maximum drawdown; no level-less fallback exists | `runner::grid::rewalk_tests::materialized_rows_replay_the_selected_variants_own_exclusivity`; `a_detail_row_that_no_longer_sums_to_the_selected_cell_is_refused`; `runner::audit::tests::an_explicit_selected_cell_overrides_the_grids_unconstrained_best` | ✓ |
| CG-03 | **Legacy level-less bytes never acquire chosen-grid meaning.** The current child is `chosen-trades.bin` magic `BRUTEXCT`, version 1, stride 136; receipt version 2 is stride 64 with sealed direction/policy. Legacy `trades.bin` v2 and receipt v1 receive explicit rerun/version refusals, and all assigned header/row reserve bytes must remain zero | `cli::trades::tests::a_legacy_level_less_file_is_named_and_never_reinterpreted`; `unknown_direction_and_reserved_bytes_are_never_given_a_meaning`; `cli::result_set::tests::the_header_stride_version_and_reserve_are_exact`; `an_intact_legacy_receipt_names_its_version_before_this_versions_stride` | ✓ |
| CG-04 | **Selected direction is sealed result metadata even for zero rows, never reverse-decoded from the frequency identity's `Undirected` term.** Every chosen row must equal the receipt direction; any disagreement or unknown enum refuses the complete block and API response | `api::trades::tests::a_committed_chosen_trade_exposes_policy_direction_and_both_excursions`; `a_receipt_direction_that_disagrees_with_its_rows_exposes_nothing`; `a_deleted_committed_trade_file_refuses_unless_its_receipted_count_is_zero` | ✓ |
| CG-05 | **The browser admits chosen trades all-or-nothing.** Policy is exactly `chosen-grid-v1`, direction is `long` or `short`, every row agrees, and adverse/favourable ppm+paisa are safe non-negative integers. Successful empty responses retain all eight period arrays; a malformed metadata or excursion field exposes no prefix | `web/tests/trade-analytics.test.js` · *chosen-grid policy and direction are canonical and every row agrees with the receipt* · *one unsafe or malformed trade integer refuses the whole payload* · *legitimate empty trade states stay distinct from malformed partial analytics*; `api::trades::tests::an_absent_file_answers_empty_and_names_why` | ✓ |
| CG-06 | **The dashboard shows only the durable chosen-grid facts.** It names selected direction and policy, renders per-trade MAE/MFE in ppm and paisa, and states that price, exit cause and cross-rung chart alignment are absent rather than fabricating markers or unavailable excursion cells | `web/tests/backtest-ui.test.js` · *the chosen-grid trade ledger exposes every durable row fact without inventing prices or causes* · *the details table owns one recorded run and names its sealed selected direction* | ✓ |
| CG-07 | **One `/trades.json` request uses one canonical committed receipt snapshot.** The receipt is read once before a fresh child open; that same identity/count/direction proof reconciles the complete row block and is retained on missing-child paths. No receipt is an explicitly reasoned uncommitted/absent answer, never clean committed-empty; only a captured zero count proves committed emptiness | `api::trades::tests::the_receipt_is_read_once_before_the_fresh_child_and_never_inside_missing_handling`; `a_present_child_without_the_canonical_receipt_is_never_clean_empty`; `a_committed_corrupt_trade_block_exposes_no_valid_prefix`; `cli::trades::tests::an_external_receipt_snapshot_is_read_only_and_cannot_cross_identities` | ✓ |

## Stateful execution membership and exact time exits — D-0421

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| EB-01 | **Every native or checked execution slice owns exactly one acceptance verdict per offered bar, produced by the same configured `Evaluator::step` that defines corruption.** The runner reads the shared bitmap in O(1); it does not repeat a local candle predicate. A length-only projection that never saw the target bars refuses every lookup | `indicators::column::tests::acceptance_bitmap_records_a_duplicate_timestamp_in_constant_time_shape`; `indicators::column::tests::checked_reprojection_records_execution_accumulator_refusal` | ✓ |
| EB-02 | **Neither stateful corruption can supply a forward-return endpoint or hide inside its inclusive path.** A rejected signal, exact exit or interior record yields no return and increments the explicit refusal path rather than becoming an ordinary tail | `runner::outcome::refusal_coverage::duplicate_timestamp_never_prices_a_forward_entry_exit_or_interior`; `runner::outcome::refusal_coverage::oversized_accumulator_never_prices_a_forward_entry_exit_or_interior` | ✓ |
| EB-03 | **Long and short trade walks reject both stateful variants in every execution role without creating overlap room.** A rejected entry opens nothing; a rejected exit or interior contributes no fill but retains conservative block-only occupancy through the time exit, and every signal still reconciles exactly once | `runner::trade::tests::stateful_refusals_gate_long_and_short_entries_exits_and_interiors` drives 2 refusal reasons × 3 path roles × 2 directions | ✓ |
| EB-04 | **A rejected interior contributes no high, low, MAE/MFE peak, stop, target or trailing crossing, grid tally, or chosen materialized row.** Its position interval still blocks the next signal, and `Grid::refused_paths` makes the loss visible | `runner::excursion::tests::rejected_membership_blocks_every_interior_crossing_and_peak`; `runner::grid::arming_tests::chosen_materialization_never_contains_a_statefully_refused_path` covers both refusal reasons and both directions and reconciles the surviving selected rows | ✓ |
| EB-05 | **A time exit exists only at its exact accepted one-minute wall-clock endpoint or at the fixed 15:10 policy boundary proved by the unique accepted 15:09 record.** No prior or next bar substitutes for a missing minute; a timestamp gap, rejected interior or absent/ambiguous 15:09 drops the affected path rather than shortening, delaying or fabricating it | `runner::trade::tests::a_hold_does_not_run_across_an_intraday_halt`; `runner::outcome::tests::missing_corrupt_or_ambiguous_1509_never_fabricates_a_forced_exit`; `runner::trade::tests::the_1509_row_prices_the_forced_fill_and_the_1510_row_is_unreachable`; `runner::outcome::tests::a_coarse_rung_prices_exact_horizons_but_not_an_unproved_square_off` | ✓ |
| EB-06 | **The indexed excursion equals the scan it replaced, for both directions and any entry base.** `peak_adverse` and `peak_favourable` were `for i in from..=to` inside `one_variant`, so the grid carried a `variants × trades × span` term. `Crossings` now records the running extreme PRICE per offset and the reads are two loads and a division. Exact because `entry - low` is monotone non-increasing in the low, so `max(entry - lowᵢ) = entry - min(lowᵢ)`, through the same `i128` widen, truncating divide and `i64::MAX` saturation the scan spelled out. Holds on any path where `Crossings::refused() == 0`, which `blocks_without_pricing` guarantees before a variant is priced | `runner::grid::tests::the_indexed_excursion_equals_the_scan_it_replaced`, and four edge tests beside it for an empty range, the forced 15:10 square-off, a non-positive entry fill, and an unbounded `to` | ✓ |
| EB-07 | **The stored extremes carry NO entry base, and that is what keeps risk pessimistic.** `one_variant` measures its adverse excursion from `entry_pess`, the worst fill, and its favourable one from `entry_opt`, the open — deliberately different prices. `crossings_with`'s running `mae`/`mfe` are ppm of `entry_opt` alone, so recording THOSE per offset (which is what the source comment proposing this reduction actually prescribed) would have silently re-based `worst_mae` onto the optimistic open and understated the excursion every stop rule is judged against. Recording the extreme price instead lets each caller apply its own entry | `runner::grid::tests::the_indexed_excursion_equals_the_scan_it_replaced` covers six bases including `i64::MAX` and negatives; `runner::grid::tests::the_grid_over_the_wide_bar_fixture_is_byte_stable` pins the whole grid's fingerprint, which was identical before and after the change | ✓ |

## Browser frontier numbers are exact or no frontier is published — D-0422

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| BT-34 | **Both frontier consumers admit one complete exact payload before any row reaches ranking.** Every bare envelope, rule and row integer is a safe integer; nullable ratios, direction and six canonical mask words retain their wire meaning; counts, admissions, evidence, trades, gross results, priced state and verdicts reconcile. One unsafe or contradictory value and every partial read expose zero rows, never a plausible prefix; committed-empty and explicitly absent children remain honest empty states | `web/tests/frontier-analytics.test.js` · *one complete exact frontier admits every row without copying or reranking it* · *every bare Rust integer field refuses the whole frontier once JavaScript cannot represent it exactly* · *canonical masks and directions are required before any ranking row is published* · *envelope and row arithmetic contradictions refuse the entire answer* · *partial server reads are refusals, while both explicit empty states stay honest*; `web/tests/backtest-ui.test.js` · *both frontier consumers admit exact complete payloads before publishing any ranking row* | ✓ |

## The Strategy Tester drill-down never substitutes density for evidence — D-0423

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| BT-35 | **The expanded Backtest report remains exact, bounded and operable at every depth.** Performance overlays one worst-fill bar per durable trade with the cumulative strategy curve; bars, axis, line and fill share the complete-series scale, and strategy run-ups/drawdowns remain selectable. The newest-first ledger renders at most 20 trades per page inside a focusable, sticky, two-axis evidence viewport without truncating the admitted analytics, and it precedes frontier ranking in both DOM and visual order. Known empty totals render zero, undefined ratios say “Undefined”, inapplicable account metrics say “Not applicable”, and missing evidence says “Unavailable”, with an exact disclosure reason rather than a padlock and a 12-pixel text floor. Row activation, the testing-period dialog, disclosures and pagers remain keyboard reachable, restore focus, and do not turn recorded facts into fake controls | `web/tests/backtest-reference.test.js` · all reference-contract tests; `web/tests/backtest-truth.test.js` · all evidence-semantics tests; `web/tests/backtest-ui.test.js` · keyboard activation, dialog ownership, native table semantics, recorded toolbar facts, disabled selectors and narrow-layout tests; live internal-browser exercise of every report and analytics tab at desktop and phone widths | ✓ |

## Ledger rows are exact computation objects or metadata only — D-0425

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| BT-36 | **No parsed `/backtest.json` row reaches an answer, rung leader, rank, derived metric, decoded mask or full report unless the complete row is exact and internally consistent.** Every integer and narrower Rust wire domain, dense exit/mask array, identity, label, boolean, span relation and engine-published signal-rung membership crosses one all-or-nothing door; exact endpoints whose difference or scaled ratio is unsafe still refuse that derived result. A rejected row remains visible only as non-drillable metadata with its reason, and integrity-failure precedence never restores computation | `web/tests/comparison.test.js` · *one all-or-nothing admission door covers every full-report integer and mask word* · *narrow Rust wire domains and contradictory span metadata are refused* · *an unsafe row derives no secondary metrics even when their own endpoints are safe* · *malformed array members remain refused metadata instead of throwing* · *signal membership comes from the exact wire set, never a min suffix guess* · *derived integer arithmetic refuses results outside the exact range*; `web/tests/backtest-ui.test.js` · *ledger rows cross one exact-integer door before answer, rung, sort, or drill computation* · *malformed ledger envelopes and unsafe derived arithmetic fail closed* | ✓ |

## A stale detail handle cannot write around newer or damaged history — D-0426

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| RS-08 | **Every frontier, chosen-trade and receipt write remeasures the append-only file while holding its writer lock before it rejects a duplicate, reuses an identity or extends the file.** A stale handle refuses shrinkage; stale handles and fresh reopens refuse a ragged tail, a bad sealed row, a seal-valid but schema-invalid row and a second non-contiguous block for one identity, so none appends beyond those facts or blesses a cached prefix as durable. An `A,B,A` file keeps A's first exact range rather than spanning B, but poisons every write and durability promotion; healthy contiguous blocks still reopen and reuse. An injected partial frontier append is truncated only to that call's locked pre-write length, leaving every older whole row byte-identical; a failed rollback is itself a named refusal. A byte-equal prepared frontier or chosen-trade block repeats `sync_all` before reuse, an exact receipt repeats its own barrier, and the parent ledger remains last | `cli::frontier::tests::two_stale_handles_cannot_append_the_same_frontier_identity`; `a_stale_handle_refuses_to_extend_past_a_bad_sealed_frontier_row`; `a_reopened_writer_refuses_to_extend_past_a_bad_sealed_frontier_row`; `a_sealed_invalid_row_on_fresh_reopen_keeps_the_healthy_prefix_but_poisons_writes`; `an_a_b_a_frontier_on_fresh_reopen_never_spans_b_and_poisons_writes`; `a_partial_frontier_write_rolls_back_to_the_last_whole_row`; `cli::trades::tests::two_stale_handles_cannot_append_the_same_chosen_trade_identity`; `a_stale_handle_refuses_to_extend_a_ragged_chosen_trade_tail`; `a_stale_handle_refuses_to_extend_past_a_bad_sealed_trade_row`; `a_reopened_writer_refuses_to_extend_past_a_bad_sealed_trade_row`; `a_sealed_invalid_trade_on_fresh_reopen_keeps_the_healthy_prefix_but_poisons_writes`; `an_a_b_a_trade_file_on_fresh_reopen_never_spans_b_and_poisons_writes`; `cli::result_set::tests::two_stale_handles_converge_on_one_exact_receipt`; `a_stale_handle_refuses_a_new_ragged_tail_before_appending`; `a_stale_handle_refuses_to_reuse_a_receipt_after_the_file_shrinks`; `a_torn_tail_and_a_corrupt_seal_are_both_refused`; `cli::tests::the_result_set_commit_marker_is_called_after_both_detail_preparations` | ✓ |

## A native one-minute path cannot learn a slower clock from missing rows — D-0427

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| XM-05 | **Every shipping audit path reprojects its signal column onto exact one-minute execution timestamps.** Native `1min` uses the same checked stored slice; each coarse path must supply its separately loaded matching slice and cannot fall back to signal OHLCV. A missing immediate minute is counted and dropped rather than replaced by the next stored row, and an interior cadence break invalidates the held path | `cli::derived_floor_tests::a_native_minute_path_self_projects_and_a_coarse_path_without_execution_refuses`; `cli::derived_floor_tests::execution_validation_allows_gaps_and_refuses_corrupt_or_ambiguous_minutes`; `runner::trade::tests::a_native_one_minute_gap_never_delays_entry_to_the_next_stored_row`; `runner::trade::tests::a_sparse_native_slice_cannot_redefine_one_minute_as_its_median_gap`; `runner::align::tests::a_signal_lands_only_on_the_execution_bar_at_its_exact_close`; `runner::align::tests::an_execution_gap_makes_the_immediate_fill_unreachable` | ✓ |

## Browser evidence is bound, recomputed and published as one answer — D-0428

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| BT-37 | **A frontier or chosen-trade body is computational evidence only when its canonical top-level identity equals the requested admitted run.** Frontier raw counts and money recompute every stored integer ratio and average with Rust-compatible saturating `i64` arithmetic, recompute the Wilson assurance predicate, then reconcile every rule verdict and the admitted count. Chosen rows remain in canonical wire sequence, use only the fill-sourced or exact-next-bar source convention, never overlap in bar or time order, and fold exactly to the immutable ledger's count, optimistic/pessimistic totals, worst trade and drawdown. Every one of the eight period arrays is rebuilt from each trade's entry timestamp and must match every bucket key and aggregate, so equal grand totals cannot hide a row filed in the wrong period | `api::frontierjson::tests::a_ranked_row_carries_its_verdict_and_the_rules_it_was_judged_against`; `api::trades::tests::a_committed_chosen_trade_exposes_policy_direction_and_both_excursions`; `web/tests/frontier-analytics.test.js` · *raw cell totals, every derived figure, and every rule verdict must agree exactly* · *the body identity is canonical and bound to the requested run*; `web/tests/trade-analytics.test.js` · *wire order, exact signal source, and one-position-at-a-time occupancy are mandatory* · *chosen rows reconcile to the immutable committed ledger aggregate* · *period totals cannot hide a trade moved into the wrong calendar bucket* | ✓ |
| BT-38 | **Vocabulary, price bars and asynchronous drill-down state cross complete admission boundaries before publication.** A vocabulary is one dense, unique, append-positioned table within the 384-bit mask capacity or publishes no prefix. A bar window reconciles total/month counts, order, OHLC domains, nullable reasons and every exact integer before either the chart series or buy-and-hold endpoint can use it. Every series, rung and benchmark request owns a generation ticket on success, refusal and exception, and closing the report invalidates all three. Frontier board slots are keyed by the complete feed/instrument/span/support comparison question plus rung, so one scope cannot overwrite another. The benchmark always reads the run's recorded rung through the same bar door and is visibly identified as a current-store reference, not as bytes bound to the run's historical `data_digest` | `web/tests/backtest-admission.test.js` · *vocabulary admission is atomic, dense, bounded, and type exact* · *one complete bar-window door rejects truncation, corruption, and wrong order* · *month-span arithmetic is inclusive and refuses inverted or malformed ranges* · *series and benchmark both cross the same bar admission door* · *every drill loader drops stale success, failure, catch, and close-panel writes* · *frontier board slots are comparison questions and expose their context* | ✓ |
| BT-39 | **No exact integer becomes an approximate browser fact merely to derive or display it.** Fixed-width mask words and saturating frontier intermediates use `BigInt`; bare JSON money/count values must already be safe integers, and any additive or scaled result outside that range refuses the complete computation. Rupee formatting splits the integer quotient and paisa remainder, so the safe-integer boundary retains its final paisa instead of crossing floating-point division; unavailable values remain an em dash | `web/tests/frontier-analytics.test.js` · *every bare Rust integer field refuses the whole frontier once JavaScript cannot represent it exactly* · *raw cell totals, every derived figure, and every rule verdict must agree exactly*; `web/tests/trade-analytics.test.js` · *safe endpoints whose fold would leave the exact range are refused before analytics* · *the largest safe integer remains admissible when every fold stays safe*; `web/tests/comparison.test.js` · *derived integer arithmetic refuses results outside the exact range*; `web/tests/money.test.js` · *integer quotient and remainder never invent or drop a paisa* · *the complete safe-integer boundary retains its exact final paisa* | ✓ |

## A persisted run names only a proved clean source commit — D-0432

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| RP-01 | **The run-identity commit has one canonical wire spelling: exactly forty lowercase hexadecimal characters.** Empty, short, non-hexadecimal, uppercase and padded values become no stamp; API admission calls the same `cli` validator, so presence alone cannot start a persisted computation | `cli::build_provenance::tests::only_full_lowercase_sha1_is_canonical`; `api::sweeprun::tests::only_a_canonical_stamped_build_does_not_refuse` | ✓ |
| RP-02 | **Neither automatic nor explicit build input can stamp a changed source tree.** The resolved HEAD tree must equal the checksummed Git index, and every indexed non-front-end file's mode and Git blob digest must equal the working tree. An explicit canonical value must additionally equal that resolved HEAD. Unstaged changes, staged changes whose worktree agrees with the index, a mismatched explicit commit and an untracked compilation input each produce the empty refusal sentinel | `cli::build_provenance::tests::a_clean_head_is_automatically_and_explicitly_proved`; `invalid_and_mismatched_explicit_stamps_refuse`; `an_unstaged_tracked_change_refuses`; `a_staged_change_that_matches_the_worktree_still_refuses`; `an_untracked_compilation_input_refuses` | ✓ |
| RP-03 | **A normal clean clone remains provable without starting a process.** The verifier authenticates Git index and object bytes, reads loose or pack-indexed objects, resolves OFS/REF deltas and checks reconstructed object ids before accepting the commit/tree. Synthetic pack-only history proves plain, OFS-delta and REF-delta objects deterministically; the checkout's packed tree adds a real repository fixture. Excluded regenerated `target/` output cannot make a clean source falsely dirty | `cli::build_provenance::tests::a_clean_pack_only_clone_is_automatically_proved`; `clean_ofs_and_ref_delta_packs_are_proved`; `the_checkout_head_tree_object_is_decodable`; `excluded_build_output_does_not_create_a_false_dirty_result` | ✓ |

## Every HTTP write is bound to the exact local origin — D-0433

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| HS-03 | **No DNS-rebound or proxy-asserted authority can reach a mutating handler.** Every non-GET/HEAD request carries exactly one `Host` naming `localhost` or the exact bound loopback IP at the bound port and exactly one matching plain-HTTP `Origin`; optional fetch metadata may only say `same-origin`, and every forwarded-authority header refuses. Missing, malformed, repeated, foreign, wrong-port, HTTPS and contradictory shapes fail before parsing the body. The live-router proof drives the truthful rebinding combination—foreign Host and Origin plus `Sec-Fetch-Site: same-origin`—against all three engine writers, while a matching bound-loopback browser request still crosses the same middleware | `api::server::tests::only_a_same_origin_write_passes_and_a_read_always_does`; `api::server::tests::a_cross_origin_write_is_refused_and_this_page_s_own_is_not`; `api::server::tests::a_rebound_foreign_host_is_refused_on_every_engine_post_route` | ✓ |

## Detail reads are bounded, identity-exact and explicit about completeness — D-0435

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| HD-01 | **Neither `/frontier.json` nor `/trades.json` performs blocking file/index work on a Tokio worker or exposes an unchecked prefix as a complete run.** One shared four-slot gate refuses saturation before queueing; opened-handle readers cap each ledger, receipt and child at 64 MiB; queries, page coordinates, selected rows and response bytes have fixed ceilings. One bounded receipt snapshot is reconciled with one exact child identity. Every bounded block is fully seal/schema/count validated before a page is cut: a one-page answer alone says `complete: true`; a partial page says 206, `page_complete: true`, `complete: false`, exact total/continuation metadata and a refusal; an over-cap result, corrupt block, foreign identity or out-of-range page exposes no plausible prefix | `api::detail::tests::admitted_work_runs_off_the_tokio_worker`; `pagination_refuses_malformed_zero_and_over_limit_values`; `preflight_refuses_an_oversized_file_from_metadata_without_reading_it`; `windows_name_partial_complete_and_out_of_range_pages`; `api::trades::tests::concurrent_detail_saturation_returns_429_without_queuing_work`; `a_large_valid_result_is_explicitly_paged_and_never_presented_as_complete`; `a_valid_result_beyond_the_hard_row_ceiling_refuses_without_a_prefix`; `an_unknown_identity_beside_a_valid_run_exposes_no_foreign_rows`; `a_committed_corrupt_trade_block_exposes_no_valid_prefix`; `api::frontierjson::tests::one_bounded_receipt_snapshot_is_reconciled_without_a_parent_reopen`; `a_large_valid_frontier_is_explicitly_paged_and_never_called_complete`; `a_valid_frontier_beyond_the_hard_row_ceiling_exposes_no_prefix`; `an_unknown_identity_beside_a_valid_frontier_exposes_no_foreign_rows`; `a_committed_corrupt_frontier_exposes_no_valid_prefix`; `cli::results::tests::bounded_read_refuses_before_indexing_an_over_limit_ledger`; `cli::result_set::tests::bounded_read_refuses_before_indexing_an_over_limit_manifest`; `cli::frontier::tests::a_bounded_reader_refuses_before_indexing_an_oversized_file`; `cli::trades::tests::a_bounded_reader_refuses_before_indexing_an_oversized_file` | ✓ |

## Every stored signal executes on exact one-minute evidence — D-0436

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| UE-01 | **Every stored non-`1min` audit loads the same feed, underlying, month/span and `1min` rung before it can price anything; native `1min` self-aligns the already loaded slice.** Empty, corrupt, duplicated, backward, sub-minute or off-grid execution evidence refuses the run. Whole missing minutes remain explicit gaps so exact projection can drop only paths that need them; no coarse OHLCV fallback exists | `cli::derived_floor_tests::a_native_minute_path_self_projects_and_a_coarse_path_without_execution_refuses`; `cli::derived_floor_tests::execution_validation_allows_gaps_and_refuses_corrupt_or_ambiguous_minutes`; `cli::tests::the_stored_audit_banner_names_the_rung_its_trades_filled_on` | ✓ |
| UE-02 | **A signal on any rung may open only on the stored one-minute bar whose timestamp exactly equals that signal bar's close, on the same IST day.** A gap, a signal on the session's last bar or an irregular path with no exact timestamp drops the entry; neither the next available row nor an interpolated/tick price may substitute. Every horizon, stop, target, trail, grid walk and chosen replay after entry consumes only that projected one-minute column | `runner::align::tests::a_signal_lands_only_on_the_execution_bar_at_its_exact_close`; `runner::align::tests::a_bucket_whose_close_lands_in_the_next_session_is_unreachable`; `runner::align::tests::an_execution_gap_makes_the_immediate_fill_unreachable`; `runner::trade::tests::a_native_one_minute_gap_never_delays_entry_to_the_next_stored_row`; `cli::derived_floor_tests::a_native_minute_path_self_projects_and_a_coarse_path_without_execution_refuses` | ✓ |
| UE-03 | **Walk-forward selection remains in signal-index space and each fold sees only its permitted signal prefix; pricing is reprojected afterwards.** Training execution ends strictly before the first test signal timestamp, the purge remains between signal windows, test conditions are blanked before the test range, and its execution label path ends at the last test signal close plus the configured one-minute horizon. Anchored and rolling folds share this two-series door, so no coarse fold can silently re-enter same-series validation | `runner::validate::tests::a_coarse_fold_projects_exactly_and_cannot_read_its_test_execution_bar`; `runner::validate::tests::a_projected_walk_accepts_minute_gaps_but_refuses_malformed_execution_time` | ✓ |
| UE-04 | **Forced liquidation is fixed at 15:10 IST and can use only the unique accepted one-minute 15:09 interval.** The 15:10 row is unreachable; missing, refused, duplicated or ambiguous 15:09 evidence creates no forced price. Regular, extended and non-regular data all face the same clock, so a session lacking 15:09 drops only holds that need the forced boundary | `runner::outcome::tests::every_session_uses_fixed_1510_and_only_exact_1509_prices_it`; `runner::outcome::tests::missing_corrupt_or_ambiguous_1509_never_fabricates_a_forced_exit`; `runner::trade::tests::the_1509_row_prices_the_forced_fill_and_the_1510_row_is_unreachable` | ✓ |
| UE-05 | **Run identity and operator reporting name both series that determine a coarse answer.** The data term domain-separates complete ordered signal and execution digests; changing a hidden one-minute row changes identity, and changing the required 15:09 row changes the modeled result while changing post-deadline 15:10 does not. Native one-series identity remains byte-compatible | `runner::identity::a_coarse_run_identity_binds_the_interior_execution_path`; `runner::identity::execution_data_binding_is_deterministic_and_presence_is_explicit`; `cli::derived_floor_tests::every_stored_two_series_operator_path_binds_execution_into_identity`; `cli::tests::the_stored_audit_banner_names_the_rung_its_trades_filled_on`; `runner::trade::tests::the_1509_row_prices_the_forced_fill_and_the_1510_row_is_unreachable` | ✓ |
| UE-06 | **The active stored sweep prices the optimistic leg at stored one-minute Open and the pessimistic leg at the direction-aware stored PrintedExtreme, with no added adverse tick outside OHLCV.** The existing walk/grid occupancy state still permits at most one position at a time within that directional candidate walk; a refused path remains conservatively blocking. This does not claim a live-brokerage fill or a workspace-wide/cross-strategy long-plus-short portfolio mutex | `runner::trade::tests::the_active_round_trip_cannot_reach_the_adverse_tick_fill_policy`; `runner::trade::tests::the_best_case_is_the_open_and_the_worst_is_the_printed_extreme`; `runner::trade::tests::no_two_trades_overlap_and_every_signal_is_accounted_for`; `runner::trade::tests::stateful_refusals_gate_long_and_short_entries_exits_and_interiors` | ✓ |

## Final Top-N selection is admitted, bounded and deterministic — D-0437

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| GN-01 | **The V1 final-ranking kernel can retain no refused row and no more than 25 admitted rows; a refused extreme cannot move an admitted score.** It scores eleven integer/fixed-point criteria in two passes, refuses zero/over-25/all-zero-policy/mismatched-population shapes, counts every terminal admission class and returns strongest first under one total tie-break. Top 10 is exactly the prefix of Top 25, independent of arrival order. This proves the bounded kernel only: a caller still needs a complete-population count/digest and extinction receipt before the result is global | `runner::topn::tests::population_edges_are_exact_and_top_ten_is_one_prefix`; `top_ten_is_exactly_the_prefix_of_the_same_top_twenty_five`; `arrival_order_and_fixed_chunks_cannot_move_a_tie`; `equal_extrema_and_undefined_ratios_are_neutral_and_counted`; `institutional_refusal_excludes_even_a_better_raw_score`; `refused_extremes_cannot_distort_admitted_scores`; `runtime_weights_change_the_order_without_changing_the_population`; `extreme_integer_domains_do_not_wrap_or_escape_the_score_scale`; `a_mismatched_second_pass_is_refused_instead_of_misnormalised`; `bounded_keeper_matches_a_full_sort_for_every_small_arrival_permutation` | ✓ |

## One portfolio position excludes every other strategy — D-0438

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| GP-01 | **One `GlobalSinglePositionV1` lock spans long and short, NIFTY and BANKNIFTY, and every supported intraday rung; the inclusive exit minute remains occupied and only the following timestamp can enter.** Each minute is atomically validated, canonically ordered by priority then digest and fully reconciled into mutually exclusive outcomes. Oversize, non-increasing time, duplicate key, invalid priority/instrument/rung and impossible occupancy refuse without mutation; unreachable or pre-entry-refused evidence never releases or fabricates a position | `runner::portfolio::tests::every_shuffle_has_the_same_priority_then_digest_answer`; `one_global_lock_crosses_direction_instrument_and_rung`; `the_exit_minute_blocks_and_the_following_minute_enters`; `unreachable_refused_and_invalid_intents_are_never_silent`; `duplicate_key_and_nonincreasing_time_refuse_without_mutation`; `invalid_surface_refuses_atomically_in_fixed_precedence`; `four_strategies_across_six_minutes_exhaust_every_terminal_shape`; `scheduler_state_and_results_are_fixed_width_values` | ✓ |

## Nested sweep parallelism shares the machine — D-0439

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| PA-01 | **An outer parallel sweep never gives every inner ladder the whole machine's support-worker count.** CLI ladders receive `ceil(available cores / declared concurrent sweeps)` lanes, with a floor of one; fourteen cores shared by eight rungs therefore means two rather than fourteen per rung. A one-lane and four-lane engine walk return exactly equal frontiers, counters, ordering and halt state, so scheduling cannot move the answer | `engine::tests::a_support_lane_bound_changes_only_scheduling`; `cli::tests::support_workers_are_bounded_across_concurrent_sweeps` | ✓ |

## Both final-ranking passes are the exact same population — D-0440

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| GN-02 | **Top-N cannot seal when pass two omits, duplicates, replaces or reorders a candidate, even when its row count and extrema remain plausible.** One ordered BLAKE3 proof binds every fixed candidate field and exact reconciliation counts across both passes. A complete 32-byte strategy digest breaks ties between semantic exit cells sharing the same mask, direction and metrics | `runner::topn::tests::a_second_pass_omission_duplicate_replacement_or_reorder_never_seals`; `complete_strategy_digest_breaks_equal_mask_direction_exit_cell_ties` | ✓ |

## The complete closed-mask population has an uncapped stream — D-0441

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| CP-01 | **Every frequent mask reaches the population sink once in canonical order with an exact closure verdict, independent of evidence rank or a retained prefix.** Normal extinction produces the same closed masks as the retaining reference and reconciles considered into closed plus redundant. A sink refusal reaches exactly one row and returns no partial population result | `runner::tests::the_population_door_streams_every_closed_mask_without_an_evidence_cap`; `one_population_sink_refusal_exposes_no_partial_run` | ✓ |

## All eight per-rung Top-25 lists reach one global minute — D-0442

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| GP-02 | **One global arbitration accepts exactly the full `8 × 25 = 200` legal constituent surface and never mistakes the per-rung rank ceiling for a whole-portfolio cap.** Priority remains 1..=25 within each rung; a 201-row batch refuses atomically, while a shuffled 200-row batch produces one admission, 199 simultaneous blocks and exact reconciliation | `runner::portfolio::tests::all_eight_per_rung_top_twenty_fives_reach_one_global_arbitration`; `wider_than_all_eight_top_twenty_fives_refuses_whole_and_changes_no_state`; `every_supported_rung_and_both_priority_boundaries_are_admitted` | ✓ |

## Dynamic index-specific exit grids never learn from OOS — D-0443

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| EG-01 | **Only NIFTY and BANKNIFTY resolve; each instrument and long/short side uses its own exact TRAINING one-minute adverse/favourable distribution, and OOS replay cannot derive or alter a rung.** Long and short swap `open-low`/`high-open` stop and target axes while the trail uses the full printed range. Every policy/resolution field changes its digest; evaluation returns a private capability bound to the resolution and side; the complete canonical ratio-filtered coordinate population is enumerated and checked; missing, duplicate, replacement, opposite-side, foreign-capability, malformed-minute, unsupported-cost and quality-limit shapes refuse before selection can masquerade as complete | `runner::exit_grid_policy::tests::only_the_two_nse_spot_indices_resolve`; `each_legal_index_resolves_from_its_own_training_distribution`; `long_and_short_resolve_separate_adverse_and_favourable_axes`; `every_policy_field_is_digest_load_bearing`; `every_resolved_content_field_is_digest_load_bearing`; `checked_cell_count_matches_every_small_ratio_filtered_enumeration`; `the_complete_training_grid_has_one_reachable_exact_enumerator`; `oos_replay_refuses_every_non_one_minute_or_corrupt_shape_before_pricing`; `exact_ladders_and_quality_limits_drive_selection_without_oos_derivation` | ✓ |

## Stored daily anchors and GapFib use their own causal evidence — D-0444

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| DR-01 | **Every stored intraday signal column uses complete same-feed, same-instrument `1day` evidence strictly before the signal IST day and replaces all GapFib positions 132..=142 only from the exact stored `1min` bar ending at that signal close.** Same/future daily rows, a missing exact close, a later minute, malformed cadence, insufficient prior-session context or an aggregate/minute close mismatch refuses without a coarse/daily fallback. Stored replay uses the same prepared builder, and the run data term binds the complete signal, exact-minute and daily streams plus eligibility, schema, calendar, overlay-policy and honest integrity-state bytes | `indicators::anchored::tests::only_a_strictly_earlier_day_can_become_the_anchor`; `appending_valid_future_daily_prices_cannot_change_a_current_mask`; `exact_minute_overlay_never_substitutes_a_later_minute`; `signal_aggregate_close_must_equal_its_exact_final_minute`; `future_exact_minutes_cannot_change_an_already_mapped_signal_column`; `indicators::column::tests::exact_gap_replacement_clears_every_local_gap_bit_and_no_other_family`; `cli::stored::tests::exact_minute_context_requires_a_complete_prior_three_bar_session`; `exact_minute_context_refuses_holes_and_malformed_cadence`; `runner::identity::tests::the_three_stream_identity_binds_every_reference_choice`; `runner::validate::tests::a_prepared_fold_builder_refusal_never_falls_back_to_the_ordinary_evaluator`; `cli::derived_floor_tests::every_stored_three_series_operator_path_binds_all_inputs_into_identity`; `cli::tests::every_intraday_rung_has_an_explicit_exact_minute_overlay_duration` | ✓ |

## Exact exit-grid authority survives gaps, deadline and stored anchors — D-0445

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| EG-02 | **A selected exit coordinate can be replayed only from the complete matching resolution/evaluation capability and the complete matching run identity.** Bars at or after 15:10 IST stay source-bound but cannot choose a tradable rung; missing whole minutes stay absent; accepted non-regular sessions are not rewritten to the regular clock; opening gaps are applied before same-bar retrace; and a stored signal + one-minute + previous-day run refuses the weaker two-stream identity constructor. Only NIFTY/BANKNIFTY and their separately resolved long/short sides can cross the door | `runner::exit_grid_policy::tests::post_deadline_prices_rekey_the_source_but_cannot_choose_a_tradable_rung`; `calendar_attested_nonregular_minutes_are_not_rewritten_as_regular_session_data`; `oos_replay_refuses_every_non_one_minute_or_corrupt_shape_before_pricing`; `exact_ladders_and_quality_limits_drive_selection_without_oos_derivation`; `stored_execution_run_preserves_the_complete_daily_reference_identity`; `runner::grid::tests::every_fixed_order_prices_the_open_before_a_same_bar_retrace`; `both_trailing_sides_use_stop_like_opening_gap_semantics`; `target_and_trail_ties_bracket_long_and_short_gap_and_non_gap_paths`; `three_way_ties_keep_only_causally_reachable_selected_orders_on_both_sides` | ✓ |

## Replay evidence and durable discovery stay exact — D-0447

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| EG-03 | **Chosen-coordinate replay publishes every reachable pre-exclusivity OOS entry, not only the entries a strategy-local fold happened to take.** A candidate on an earlier candidate's inclusive exit minute remains present and blocked, while the immediately following minute may enter. If a known entry's crossing path cannot be priced, it carries no invented money row and retains occupancy through its conservative time exit. The opaque universe reconciles its refusal count, execution coordinates and timestamps and rejects a torn candidate; the compatibility replay delegates to that universe and remains stricter by refusing any unpriceable path | `runner::grid::tests::replay_universe_retains_locally_blocked_entries_and_inclusive_boundaries`; `runner::grid::tests::crossing_refusal_remains_reachable_occupancy_without_a_fabricated_price`; `runner::exit_grid_policy::tests::candidate_universe_digest_binds_every_path_and_legacy_replay_stays_strict` | ✓ |
| DR-02 | **The minute context used by the daily-reference join and the exact minute slice priced by the exit grid are separate bound facts.** The complete context, including warm-up, contributes to the three-stream run identity; only an exact contiguous byte-for-byte subslice may become the execution capability. A warm-up-only mutation therefore re-keys the run, detached execution bytes refuse, and a changed requested slice authorizes only its matching execution series | `runner::exit_grid_policy::tests::daily_reference_context_and_exact_evaluated_slice_are_bound_separately` | ✓ |
| RM-01 | **The V1 durable-mask adapter never clears, redirects or blesses a non-live stored bit.** Empty and canonical-live words round-trip exactly; tombstoned bits name their recorded replacement, void and unallocated positions remain distinct, and the lowest invalid position has stable precedence across the fixed six-word representation. This proves the adapter itself, not a durable reader: no production caller invokes it yet | `runner::replay_mask::tests::zero_and_every_live_position_round_trip_exactly`; `runner::replay_mask::tests::every_word_boundary_is_classified_by_its_exact_position`; `runner::replay_mask::tests::all_three_tombstones_refuse_with_their_recorded_replacements`; `runner::replay_mask::tests::the_lowest_invalid_bit_has_stable_precedence` | ✓ |
| GN-03 | **“Latest selection” means the last appended receipt for one exact `(shared cohort digest, signal rung)` pair.** A receipt from another cohort or rung cannot be substituted, an absent pair returns no selection, and reopening reconstructs the same answer from file order. This proves bounded ledger discovery, not operator integration: no non-test caller asks for the latest selection yet | `cli::selection::tests::latest_lookup_is_exactly_isolated_by_cohort_and_rung` | ✓ |

## Calendar and admission authority are appended, never inferred — D-0448

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| PV-01 | **A Population V4 receipt authorizes only its exact V2/V3 population plus complete signal-rung and independent one-minute calendar receipts covering the full requested civil-month span.** Swapped rungs, shrunken endpoints, missing V3, corrupt/ragged records, changed coverage and stale same-length files refuse; legacy V2/V3 files remain audit-only and are never promoted by open | `cli::population::tests::v4_requires_typed_full_month_signal_and_one_minute_coverage`; `v4_coverage_codec_fails_closed_on_seal_version_rungs_bounds_digests_and_reserve`; `pre_v4_ledger_remains_audit_readable_but_never_gains_v4_authority`; `v4_reopen_refuses_missing_v3_bad_header_ragged_tail_and_bad_seal`; `v4_page_refuses_post_open_same_length_authority_mutation` | ✓ |
| AD-01 | **Canonical admission bytes decode to the exact policy, evidence and verdict they encode, and a sealed decision recomputes the verdict rather than trusting it.** Wrong domain/version/length/reserve/tag, invalid policy/evidence, unknown reason bits, inconsistent partitions and a forged but individually valid verdict all refuse, including when the outer digest is recomputed | `runner::admission::tests::canonical_decoders_round_trip_every_authoritative_record`; `canonical_headers_refuse_length_magic_domain_reserve_version_and_payload_length`; `policy_decoder_refuses_invalid_boolean_tags_and_invalid_payload_values`; `evidence_decoder_refuses_unknown_tags_and_nonzero_payloadless_reserves`; `verdict_decoder_refuses_unknown_reasons_tags_and_redundancy_forgery`; `recomputed_outer_seal_cannot_authorize_a_forged_valid_verdict` | ✓ |
| AD-02 | **Admission decisions become visible only through a receipt appended after the complete contiguous decision block.** The receipt binds the Population V4 completion, exact canonical policy, four terminal counts and ordered decision bytes. A prepared orphan is hidden and only an exact retry may complete it; conflicting reruns, forged verdicts, bad headers/reserves/seals, ragged tails and stale length or generation changes refuse without replacing bytes | `cli::admission_store::tests::receipt_last_commit_reopens_pages_and_reuses_only_exact_bytes`; `a_prepared_orphan_is_hidden_and_only_an_exact_retry_can_complete_it`; `exact_codecs_bind_every_status_and_refuse_reserves_and_seals`; `reopen_refuses_forged_verdicts_ragged_tails_bad_headers_and_reserves`; `stale_handles_refuse_length_and_same_length_generation_changes` | ✓ |

## Final selection reads recomputed admission authority — D-0449

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| GN-04 | **Selection V3 ranks only rows returned by the immutable Population V4/admission join, binds both completion digests for NIFTY and BANKNIFTY, and resolves every retained candidate back to its exact joined source row.** Three ordered passes must reproduce one proof; a legacy admission summary cannot admit a row, a duplicate strategy alias cannot satisfy two winners, Top 10 is the exact prefix of Top 25, and V1/V2 files never fall back into the V3 decoder. This proves admitted Top-N persistence only: V3 alone carries no reconstructible horizon/grid authority and cannot authorize replay | `cli::selection_v3::tests::only_recomputed_sidecar_admitted_status_enters_topn`; `two_pass_top_twenty_five_resolves_exact_rows_and_duplicate_alias_refuses`; `authority_digest_changes_identity_and_top_ten_is_exact_prefix`; `empty_and_sub_twenty_five_populations_have_exact_cardinality`; `exact_layout_roundtrips_and_reserved_or_seal_mutations_refuse`; `ledger_reopens_indexes_and_isolates_latest_by_exact_cohort_and_rung`; `header_is_exact_and_v1_or_v2_magic_never_falls_back`; `same_length_external_mutation_makes_writable_handle_stale` | ✓ |

## Institutional evidence is explicit before it is authoritative — D-0452

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| IE-01 | **The public institutional-evidence kernel never converts an absent measurement into numeric zero, clamps an invalid probability, mixes foreign population/strategy/grid/evaluation authority, or lets post-selection evidence authorize the admission required to create that selection.** A typed legacy `RomanoWolfReceipt` still binds in-memory White/SPA/decision denominators. D-0461's receipt-last exact family projection remains post-selection, so the pre-selection builder keeps adjusted FWER/Romano--Wolf fields unmeasured and prevents an admission→selection→admission cycle. Anchored walk-forward decided/profitable/OOS values remain measurable, but its supplied legacy placement diagnostic can only be reconciled: `pbo_ppm`, contributing folds and unrankable folds all remain `Unmeasured` because they have no genuine CSCV authority. The 44-field source matrix and seven blocker rows are complete and unique | `cli::institutional_evidence::tests::public_builder_keeps_absent_admission_authorities_fail_closed`; `public_builder_refuses_foreign_population_strategy_cell_and_evaluation_authorities`; `missing_family_parts_are_unmeasured_and_never_zero`; `family_receipt_reconciles_rejected_and_non_rejected_candidates`; `durable_family_authority_is_exact_post_selection_evidence_not_admission_input`; `family_receipt_refuses_mismatched_denominators_and_alpha`; `supplied_anchored_fold_legacy_diagnostic_must_match_the_walk_forward_fold_set`; `aligned_legacy_diagnostic_never_authorizes_genuine_pbo_fields`; `empty_walk_forward_does_not_invent_pbo_or_outcome_evidence`; `misaligned_legacy_candidate_arrays_still_refuse_validation_evidence`; `singleton_legacy_fold_never_becomes_measured_pbo_admission_evidence`; `invalid_probabilities_are_refused_instead_of_clamped`; `weakest_period_refuses_bucket_overflow_instead_of_saturating`; `source_and_blocker_matrices_are_complete_unique_and_renderable` | ✓ |
| PW-01 | **The uncapped population/admission producer expands every closed mask in canonical long-then-short coordinate order, withholds all output on incomplete closure or mandatory-evidence refusal, and commits/reopens Population V4 plus admission receipt-last authority before an exact retry can reuse both.** The proof uses synthetic bars and supplied evidence; it is not a production stored-run caller and does not prove that all institutional evidence was measured from durable real artifacts | `cli::population_admission_writer::tests::direct_population_evidence_is_exact_or_refused`; `explicit_unmeasured_metric_is_not_replaced_by_numeric_row_value`; `complete_v4_claim_cannot_hide_incomplete_candidate_coverage`; `undefined_ranking_ratio_cannot_hide_measured_admission_ratio`; `commit_coordinator_reopens_joined_authority_and_exact_rerun_reuses_both_receipts`; `population_identity_binds_every_supplied_authority_term`; `strategy_identity_binds_mask_side_and_exact_exit_coordinate`; `uncapped_producer_expands_closed_masks_long_then_short_at_exact_count`; `halted_unknown_population_refuses_before_evidence_finalization`; `foreign_grid_and_wrong_side_execution_run_are_refused`; `mandatory_statistical_evidence_refusal_exposes_no_partial_population` | ✓ |

## Execution capability is row-for-row Population V4 authority — D-0450

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| EC-01 | **A V1 execution completion becomes visible only after exact long/short dynamic parameters, all percentile atoms and one capability per Population V4 row are durably appended and reconciled.** Reorder, copied/foreign identity, noncanonical tags/reserves/ordinals, torn seals, corrupt completion and stale same-length mutation refuse; an exact rerun reuses semantic authority even when a prior valid orphan changes physical offsets. The real V4 fixture proves preparation and persistence, but its selected-exit identities are test fixtures: no test yet reconstructs the sealed `SelectedExitV1` from actual resolved/training/stored inputs and no production caller invokes preparation | `cli::execution_capability::tests::runtime_sized_parameter_codec_reconstructs_thirty_one_levels_per_axis`; `codecs_refuse_resealed_unknown_tags_and_nonzero_reserves`; `recomputed_outer_seal_cannot_forge_selected_exit_identity`; `torn_seal_and_noncanonical_percentile_ordinal_are_refused`; `crash_orphan_offsets_do_not_rekey_semantic_completion`; `real_population_v4_prepares_every_row_and_refuses_reordered_capabilities`; `execution_ledger_reopens_reuses_and_retains_valid_orphan_evidence`; `execution_ledger_refuses_corruption_and_a_stale_same_length_handle` | ✓ |

## Execution Disposition V2 supersedes the V1 cardinality mismatch — D-0463

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| ED-01 | **A V2 completion persists exactly one terminal disposition for every Population V4 row in canonical sequence.** `Authorized` alone carries one sparse capability and zero refusal bits; `PolicyRefused` carries no capability and retains exact nonempty known refusal bits. Structural runner failures remain errors and cannot become either terminal. The receipt binds the exact Population V4 and admission completions, dynamic parameter blocks, row order, terminal counts, admission marginals and full 4×2 admission/execution matrix. Missing or foreign rows, forged tags/bits/reserves, append/reopen/reuse disagreement, valid orphans, torn/corrupt records, a semantically resealed matrix redistribution and same-length stale mutation are all exercised fail closed. This supersedes the V1 cardinality blocker at the versioned ledger boundary only: controlled fixtures and an absent non-test stored caller do not prove end-to-end Step 3 | `cli::execution_disposition_v2::v2_tests::public_ledgers_cover_the_complete_matrix_and_reclassify_after_reopen`; `parameterized_fixture_proves_banknifty_hourly_top25_capacity`; `durable_reclassification_refuses_every_foreign_authority_axis`; `fixed_row_codec_preserves_dynamic_provenance_and_both_terminals`; `row_decoder_refuses_seal_unknown_tags_bits_reserve_and_half_ttp`; `completion_codec_binds_all_three_blocks_and_excludes_physical_offsets`; `ledger_appends_reopens_pages_reuses_and_reconstructs_parameters`; `valid_orphans_survive_but_torn_files_and_same_length_corruption_refuse`; `runner::exit_grid_policy::tests::structural_errors_precede_and_never_become_policy_refusals` | ✓ |

## Global replay persistence is exact but public preparation is still open — D-0451

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| VX-01 | **India VIX lookup is exact-minute, same-feed and same-month reference evidence only.** Exact stamps preserve all seven stored fields; a hole is typed absent, while off-grid/wrong-month/reordered/duplicate/corrupt storage or a missing/foreign feed refuses instead of borrowing or interpolating. This proves the reference door, not a production global replay caller | `cli::vix_reference::tests::exact_lookup_carries_all_seven_fields_and_a_hole_stays_typed_absent`; `the_reference_door_does_not_weaken_the_stored_sweep_guard`; `an_off_grid_stored_bar_and_an_off_grid_lookup_both_refuse`; `a_wrong_month_stored_bar_and_a_wrong_month_lookup_both_refuse`; `duplicate_exact_timestamps_are_refused_even_after_the_store_was_corrupted`; `reordered_unique_timestamps_are_refused`; `post_write_ohlc_and_count_corruption_are_refused`; `feed_and_whole_month_absence_refuse_instead_of_borrowing_another_source`; `store_open_failures_remain_named_and_are_not_downgraded_to_absence` | ✓ |
| GPR-01 | **A V1 global replay receipt reconciles exactly 200 canonical streams, every reachable decision, one shared long/short/index/rung position lock, priceable admitted money rows and exact-or-absent VIX publication stamps.** The public preparation test joins eight Selection V3 receipts to sixteen execution authorities, reconstructs 200 witnessed selections into scheduler-visible streams, and refuses missing or foreign authority. Receipt-last append/reopen/reuse, valid orphan retention, reordered-decision refusal, same-length stale refusal, inclusive occupancy and replay/VIX identity separation are also proved. All inputs are controlled fixtures and no non-test caller supplies real stored selected reconstruction, so this row does not prove end-to-end Step 3 | `cli::global_replay::tests::public_prepare_reaches_scheduler_with_all_authorities_and_refuses_missing_or_foreign_ones`; `fixed_record_codecs_roundtrip_and_seal_or_reserve_mutations_refuse`; `inclusive_global_occupancy_replays_and_pricing_refusal_never_gets_money`; `vix_changes_publication_but_not_selection_pnl_or_replay_identity`; `receipt_last_ledger_reopens_replays_and_reuses_exact_publication`; `valid_orphan_is_not_committed_and_reordered_decision_refuses_reopen`; `same_length_external_mutation_makes_open_writer_stale` | ✓ |

## Stored-data completeness is exact Population V4 authority — D-0458

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| DC-01 | **Institutional `data_complete` can become `Complete` only from a reopened sealed V1 authority for the same semantic Population V4.** Preparation recomputes the exact signal, complete one-minute context, evaluated execution and daily-reference identities; requires the execution bytes to be one contiguous subslice of that context; rebuilds both complete civil-span calendar receipts; and reconciles data/feed/commit/calendar/daily-policy identities with Population V4. The append-only fixed-stride ledger seals every record, appends before indexing, reuses only exact bytes and refuses foreign identity, noncanonical reserve, ragged/corrupt history and stale same-length handles. This is not a vendor-checksum claim and has no non-test stored-run caller yet | `cli::stored_data_completeness::tests::institutional_complete_requires_reopened_matching_authority`; `exact_three_stream_bytes_and_population_v4_are_required`; `sealed_append_reopen_reuse_corruption_and_stale_handles_fail_closed`; `ragged_tail_and_foreign_population_are_never_hidden`; `codec_reserves_and_seals_are_not_ignored` | ✓ |

## Romano--Wolf adjusted probabilities retain exact evidence — D-0460

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| RW-01 | **Every candidate adjusted p-value is derived from one shared deterministic stationary-bootstrap matrix, retains its exact `(strict exceedances + 1)/(draws + 1)` fraction, and is nondecreasing in canonical stepdown rank.** Descending observed-statistic order uses caller position only as the explicit exact-tie break; the receipt binds that ordered family, seed, block and raw return bytes; O(1) positional lookup cannot move a probability to another candidate. The first row is named only as the full-family maximum/intersection p-value. Zero/misaligned/one-period/zero-variance inputs refuse instead of producing numeric evidence. This is an in-memory authority, not a durable production family or Step-3 completion | `runner::bootstrap::adjusted_p_value_tests::exact_denominators_seed_and_full_precision_survive_the_receipt`; `canonical_stepdown_probabilities_are_monotone_and_familywise_is_named`; `monotonicity_repair_is_load_bearing_and_not_a_sorted_output_decoration`; `no_rejection_and_all_rejection_are_both_complete_results`; `ties_and_input_permutations_cannot_silently_rename_probabilities`; `repeated_seed_is_byte_identical_and_any_input_change_rekeys_the_family`; `zero_or_malformed_or_degenerate_input_is_refused_not_numeric` | ✓ |

## Exact family statistics become authority only after a receipt — D-0461

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| IS-01 | **A selected candidate's exact family statistics become durable authority only after the sealed value row is synced and its matching completion is synced last.** The row binds Population V4, receipt-last selection, ranking policy, strategy, ordered family, bootstrap inputs, canonical candidate/rank, exact initial/adjusted/full-family counts and the original White/SPA `f64` bits. Exact retry reuses bytes; only an exact tail orphan may receive a completion. Foreign/changed retry, zero/malformed source, torn/corrupt/reordered/duplicate history and stale length or same-length content refuse. The post-selection adapter preserves those exact values but the pre-selection admission builder refuses it to prevent an authority cycle; global full-precision completeness remains unmeasured | `cli::institutional_statistics::tests::exact_authority_appends_reopens_and_reuses_without_losing_precision`; `changed_same_subject_and_malformed_sources_refuse`; `only_exact_retry_can_complete_a_valid_trailing_orphan`; `torn_corrupt_duplicate_and_reordered_files_refuse`; `stale_handles_refuse_length_and_same_length_changes`; `cli::institutional_evidence::tests::durable_family_authority_is_exact_post_selection_evidence_not_admission_input` | ✓ |

## Global Replay V2 is receipt-last execution authority only — D-0459

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| GPR-02 | **A V2 completion can become visible only after exactly eight reproduced Selection V4 receipts, sixteen distinct Population V4/admission/Execution V2 completions and all 200 freshly reclassified selected witnesses produce one exact global schedule.** Manifest, candidate, stream and decision records are sealed and synced before the receipt; reopen validates every record, skips only valid unreferenced orphan ranges, reconstructs all streams and re-runs the shared long/short/index/rung scheduler before indexing. Exact retry reuses byte-equivalent reconstructed evidence; a missing/duplicate/foreign/stale witness or authority, corrupt seal, reordered sealed decision or same-length stale writer refuses. The receipt explicitly binds execution-only/no-money/no-VIX scope, so it cannot claim absent admitted-money rows or VIX stamps, and controlled fixtures do not prove a production stored caller | `cli::global_replay_v2::tests::public_prepare_and_receipt_last_reopen_use_all_eight_sixteen_and_two_hundred`; `manifest_codec_is_new_sealed_and_round_trips_every_authority`; `manifest_identity_refuses_copied_selection_population_and_execution_authority`; `global_scheduler_blocks_direction_instrument_and_timeframe_until_exit`; `same_minute_order_is_priority_then_digest_not_caller_order`; `candidate_identity_binds_universe_ordinal_time_and_refusal_path` | ✓ |

## Step-3 comparison never promotes absence or a foreign authority — D-0462

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SC-01 | **One comparison request names exactly sixteen canonical Population V4 identities, eight Selection V4 identities and one Global Replay V2 completion; every row reports full IDs, exact counts and exactly one of `READY`, `BLOCKED`, `UNMEASURED` or `REFUSED`.** Only current typed ledgers are opened. An absent current file/receipt stays unmeasured, a valid stage with an unavailable prerequisite or incomplete 8×25 topology is blocked, and every typed stale/corrupt error or foreign digest/ID join is refused. A structurally valid Selection V4 receipt is still `BLOCKED` until `verify_against_authorities` replays it with the exact `RankingPolicyV1`; Global Replay is blocked transitively. No persisted receipt or caller-authored default substitutes for that live capability. Rendering is deterministic and performs no second join | `cli::step3_comparison::tests::absent_current_format_stages_are_unmeasured_without_fallback`; `corrupt_existing_population_stage_is_refused`; `consistent_population_and_admission_facts_reconcile`; `foreign_admission_identity_is_refused`; `stale_or_corrupt_probe_is_never_ready`; `exact_top10_top25_and_8x25_counts_are_required`; `structurally_consistent_selection_stays_blocked_without_full_authority_replay`; `renderer_is_deterministic_and_keeps_all_four_states_distinct`; `request_and_bounds_refuse_zero_duplicate_and_undersized_inputs` | ✓ |

## White and SPA exact receipts preserve their existing procedure — D-0465

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| WS-01 | **An exact White or SPA receipt retains the original `bootstrap maximum >= observed` count, its finite-resample `+1/+1` fraction, full statistic/p-value bits, complete ordered-family identity and every resampling input without changing the legacy `Verdict`.** Procedure domains cannot alias; draw, seed, block, candidate count, period count, order or one return mutation changes identity. Empty, misaligned, zero-draw, zero-block, one-period and overflowing-denominator inputs cannot mint exact evidence | `runner::bootstrap::exact_family_test_receipt_tests::exact_counts_bits_and_legacy_verdicts_are_identical`; `repeated_inputs_are_identical_and_every_identity_term_rekeys`; `malformed_or_unresampleable_inputs_never_gain_exact_authority`; `handled_zero_variance_keeps_named_procedure_compatibility` | ✓ |

## The anchored-fold bottom-half diagnostic is exact and never promoted to PBO — D-0467

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| AW-01 | **One V1 anchored-fold placement chooses the first in-sample maximum under the same canonical strict-`>` scan as validation and retains an exact doubled out-of-sample midrank.** An even tied block cannot be rounded across the midpoint before classification. The aggregate is explicitly an anchored walk-forward bottom-half rate: it is not CSCV/PBO, cannot authorize Population Statistics V2 and is rendered with that limit beside the number | `runner::pbo::tests::exact_v1_uses_the_same_first_strict_maximum_as_validation`; `exact_v1_preserves_an_even_tied_block_across_the_midpoint`; `runner::audit::tests::a_half_legacy_rate_is_bounded_to_the_supplied_anchored_folds`; `a_measured_legacy_rate_prints_its_denominator_median_and_limit` | ✓ |
| AW-02 | **Admission Evidence V1 cannot promote that anchored diagnostic through its historical PBO-named slots.** Public construction refuses every measured PBO value, contributing-fold count or unrankable-fold count before policy evaluation. Reopen refuses pre-D-0470 measured bytes at the nested evidence boundary, while historical `Unmeasured` and `Refused` states remain readable. Therefore no publicly constructible or reopened V1 evidence can authorize PBO or become admitted; a genuine CSCV authority requires a successor evidence version | `runner::admission::tests::caller_crafted_measured_pbo_named_v1_evidence_is_refused_before_policy_evaluation`; `reopened_legacy_measured_pbo_named_v1_bytes_refuse_loudly`; `unmeasured_and_refused_pbo_named_v1_bytes_remain_readable`; `publicly_constructible_and_reopened_v1_evidence_cannot_authorize_pbo`; complete focused admission suite **32/32** and strict Runner library/test Clippy green on 2026-08-31 | ✓ |

## Candidate Universe V1 is sealed evidence; stored authoring has one production door — D-0468, D-0475

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| CU-01 | **A completed Candidate Universe V1 block encodes and binds every claimed pre-admission source term and exactly one canonical row for every reconciled closed-mask × direction × exit-coordinate cell; standalone bytes do not prove production derivation.** Data/feed/source-commit/vocabulary/evaluation, separate long/short grid policy and resolution, canonical calendar policy, prior-day policy, full requested span, complete signal and one-minute calendars, exact signal/execution streams and both columns all rekey the structural record. Row semantics bind the six-word mask, support, direction, grid/run/coordinate and every raw direct `Cell` field; no admission, rank or selected result is stored. D-0475/SO-01 is the sole public stored-origin authoring path; the ledger itself still exposes no caller-authored source/digest constructor | `cli::candidate_universe::tests::exact_stream_and_column_facts_are_identity_terms`; `row_and_receipt_codecs_cover_all_raw_facts_and_full_seals`; `canonical_coordinate_order_is_constant_state_and_refuses_duplicates`; `cli::step3_orchestrator::tests::exact_reopened_join_is_the_only_success_shape` | ✓ |
| CU-02 | **Candidate rows become visible only through a synced receipt-last completion; production authoring is reachable only through the owning stored orchestrator.** Exact committed retry reuses bytes; one exact orphan prefix may receive only its missing suffix and matching receipt. Foreign orphan, torn/ragged/corrupt/reordered history, over-bound open, cached same-length mutation or replacement of the lock/row/receipt path refuses. One sequence seek is fixed-stride; open, construction, validation and pages remain input-dependent | `cli::candidate_universe::tests::ledger_is_receipt_last_reopenable_paged_and_idempotent`; `exact_orphan_prefix_resumes_and_foreign_orphan_refuses`; `ragged_corrupt_and_stale_files_fail_closed`; `replaced_lock_row_and_receipt_paths_refuse_cached_audits`; `cli::step3_orchestrator::tests::every_foreign_reopened_candidate_term_refuses_by_name` | ✓ |

## Stored pre-admission spans require an explicit nonzero ceiling — D-0469

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| LS-03 | **The bounded stored-span door refuses from each validated month header before allocating or reading records that would cross the caller's explicit cumulative ceiling.** The bound has no zero or `Default` value, and it is a resource refusal only: it never truncates a month, samples bars, changes the requested span or creates a sweep-depth parameter. Missing months retain the existing named-hole semantics; every corrupt/unsafe month still refuses the whole span | `cli::stored::tests::bounded_span_refuses_from_header_before_crossing_its_record_ceiling`; `stored_span_bound_has_no_zero_or_implicit_default`; `a_span_joins_its_months_into_one_strictly_increasing_series`; `a_corrupt_middle_month_refuses_the_whole_span_instead_of_becoming_missing` | ✓ |

## Pre-Admission Data V1 binds measured streams; stored authoring is owned by D-0475 — D-0471, D-0475

**This family is `PAD-`, and it was `PA-` for one commit.** `PA-01` was
assigned twice in the same commit — once to the outer-parallel-sweep
support-worker bound above, and once to the row below — so gates 10b and 27
were both red and neither id resolved to anything. The evidence rule D-0045
set and D-0215 reapplied gives the letter to whichever side is cited from
OUTSIDE this document: the parallel-sweep row is cited by
`docs/05-decisions.md` D-0439 and by `docs/06-limits.md` §129, so it keeps
`PA-01`, and this family moved to the free prefix `PAD-`. `PAD` was checked
against every prefix in this document and against every tracked file before it
was chosen; it collided with nothing.

`PA-02` and `PA-03` ARE RETIRED and must never be reassigned — CLAUDE.md §3
rule 8 forbids reusing an identifier, and both were live for the length of one
commit. The `PA-` family now has exactly one member.

**The one citation outside this document was carried with the rename.**
`docs/02-store-format.md:1320` (the bounded-file paragraph closing the
Pre-Admission Data V1 section) read "D-0471, PA-01/PA-02 and limits §153 define
the boundary", and both ids there mean THIS family; it now reads
`PAD-01/PAD-02`. The two surviving `PA-01` citations —
`docs/05-decisions.md:29638` and `docs/06-limits.md:6759` — mean the
parallel-sweep row and are correct as written.

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| PAD-01 | **One Pre-Admission Data V1 record binds the exact Candidate completion to measured signal, complete one-minute context, an exact contiguous execution subslice, prior-day daily records/eligibility, both full-span calendars, feed/commit/policies and explicit nonzero load ceilings.** No public constructor accepts caller-authored stream or policy digests; the private measurement path recomputes the composite data identity and both calendar receipts | `cli::pre_admission_data::tests::exact_740_byte_data_and_completion_codecs_bind_every_semantic_field`; `exact_contiguous_execution_and_daily_eligibility_are_measured_not_claimed` | ✓ |
| PAD-02 | **A Pre-Admission entry is visible only when adjacent sealed Data and Completion records repeat every semantic field exactly.** Exact retry reuses bytes; one exact tail orphan may receive its receipt. Ragged, corrupt, foreign, stale, over-bound or path-replaced history refuses, and allocation is fallible before persistence. The writable ledger also requires an already-existing directory and cannot manufacture a disappeared external-volume root. The ledger still exposes no public source/digest constructor; D-0475's owning stored orchestrator is the only public production authoring path | `cli::pre_admission_data::tests::ledger_is_receipt_last_reopenable_paged_bounded_and_idempotent`; `exact_trailing_data_orphan_resumes_and_foreign_retry_refuses`; `ragged_corrupt_and_foreign_completion_pairs_fail_closed`; `stale_same_length_mutation_and_nonzero_bounds_fail_closed`; `replaced_lock_and_data_paths_refuse_cached_audits`; `writer_refuses_missing_or_non_directory_root_without_creating_it`; `cli::step3_orchestrator::tests::exact_reopened_join_is_the_only_success_shape`; focused suite **8/8** green on 2026-08-31 | ✓ |

## Population Statistics V2 recomputes evidence; its writer cannot mint it — D-0472, D-0474

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| PS-01 | **One completed Statistics V2 audit binds one exact NIFTY/BANKNIFTY Pre-Admission pair and retains the complete candidate, aligned-period and complementary-split families needed to recompute its stored CSCV/PBO, White, SPA, Wilson and Romano--Wolf evidence.** Family order, candidate order, shared period identity, canonical split masks, counts, digests, procedure inputs and full-precision values are verified rather than trusted | `cli::population_statistics_v2::tests::complete_pair_recomputes_reopens_pages_and_exactly_reuses`; `mismatched_pre_admission_identity_candidate_order_and_cscv_family_refuse`; `calendar_feed_commit_daily_and_span_are_audit_identity_terms` | ✓ |
| PS-02 | **Statistics V2 becomes visible only after its sealed Data/raw-row block and matching receipt-last Completion are durable, and its public surface cannot mint a production audit.** An exact trailing prefix accepts only the byte-identical suffix; foreign retry, corrupt/ragged/resealed input, explicit-bound breach and stale same-length mutation refuse. The public durability seam accepts only an opaque `PreparedPopulationStatisticsV2`, syncs Data/raw rows before Completion, drops the writer and freshly reopens the exact audit. Its only constructor and every raw candidate/period/split input remain test-private until a typed production source derives the observations, so no caller can supply a semantic digest, return, split score or p-value determinant | `cli::population_statistics_v2::tests::exact_trailing_prefix_retry_completes_and_foreign_retry_refuses`; `explicit_bounds_and_post_open_same_length_mutation_refuse`; `ragged_corrupt_and_semantically_resealed_sources_refuse`; `opaque_writer_freshly_reopens_and_preserves_written_or_reused`; `writer_refuses_missing_or_non_directory_root_without_creating_it`; focused suite **8/8** green on 2026-08-31 | ✓ |

## Stored Candidate-to-Pre-Admission construction has one bounded authority door — D-0475

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SO-01 | **A public stored Candidate-to-Pre-Admission transaction cannot accept raw bars, source/calendar/grid digests or a caller-authored commit.** A canonical clean build stamp is required before any store read; bounded same-feed signal, daily and exact-minute streams derive complete canonical-session calendars, the exact requested one-minute execution subspan and distinct attested long/short grids before the private Candidate source can exist. Every stored daily row and every unique signal day is classified by the canonical NSE calendar; prior GapFib warm-up walks backward to the latest accepted session, validates every minute against its measured window and requires its exact final three minutes. The transaction exposes only identities recovered after both receipt-last ledgers reopen | `cli::step3_orchestrator::tests::exact_reopened_join_is_the_only_success_shape`; `cli::stored::tests::a_stray_daily_row_on_a_measured_closed_day_refuses`; `a_single_signal_day_measured_closed_refuses_inside_the_daily_door`; `exact_minute_context_refuses_sunday_and_weekday_closed_rows`; `exact_minute_context_refuses_three_early_prior_session_bars`; focused stored suite **51/51** green on 2026-08-31 | ✓ |
| SO-02 | **The daily and exact-minute typed context doors preserve the stored month-header ceiling before allocation and remain fallible while transforming admitted rows.** They reuse one canonical private conversion/calendar authority and never truncate a month, sample bars, substitute another stream or create a sweep-depth control. All three transformed daily vectors reserve through named `try_reserve_exact` refusals | `cli::stored::tests::typed_daily_and_minute_context_doors_keep_the_header_bound_protective`; `bounded_span_refuses_from_header_before_crossing_its_record_ceiling`; focused stored suite **51/51**, `cargo check -p cli --lib --locked`, file formatting and diff checks green on 2026-08-31 | ✓ |

## A disappeared configured store root is refused before API/CLI side effects — D-0473

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| RV-01 | **The API server lock, API census, autopilot write probe and CLI process boundary treat the configured store root as pre-existing authority, not a directory they may manufacture.** Missing and non-directory roots refuse and remain unchanged. A root-level census refusal is `Unreadable` for every fixed vendor rather than a false all-`Absent` clean store. API startup and CLI preflight have no canonicalization fallback; the API opens its lock and builds its serving state from the same canonical target even if the configured symlink is retargeted, and the CLI performs its preflight before installing telemetry or dispatching a command. This invariant is deliberately about initial existence, type, root-level census meaning, one API symlink race and process ordering only: it does not assert that an existing path names the expected removable volume or remains attached after the check | `api::autopilot::tests::the_write_probe_measures_the_disk_and_leaves_nothing_behind`; `api::server::tests::a_missing_store_root_is_refused_without_manufacturing_a_replacement`; `api::server::tests::a_retargeted_store_symlink_cannot_move_the_admitted_server_lock`; `api::census::tests::a_missing_or_non_directory_root_is_loud_and_never_manufactured`; `cli::tests::the_store_root_prefers_the_override_and_refuses_when_it_has_neither`; `cli::tests::a_missing_store_root_is_not_created_or_replaced`; `cli::tests::a_regular_file_cannot_be_used_as_the_store_root`; `cli::tests::an_existing_store_directory_is_returned_canonically`; `cli/tests/binary.rs::a_missing_store_refuses_before_logging_or_dispatch` | ✓ |
| RV-02 | **Selection V4 and Execution Disposition V2 writable ledgers independently require an already-existing directory authority root and may create only its direct `results` child.** An absent root or a regular file at the configured spelling refuses and remains unchanged; neither writer may recursively recreate a vanished removable-volume hierarchy. This proves only initial pathname existence/type at these two boundaries, not device identity, continuous attachment or freedom from a later path replacement | `cli::selection_v4::tests::writable_ledger_refuses_missing_or_nondirectory_root_without_recreation`; `cli::execution_disposition_v2::tests::writable_ledger_refuses_missing_or_nondirectory_root_without_recreation`; focused Selection **9/9**, Execution **10/10**, CLI library check and owned-file strict Clippy green on 2026-08-31 | ✓ |

All eleven named focused tests are green at this checkpoint: the API probe,
startup-lock and root-census refusals; the CLI resolver and three exact-path
admission outcomes; the API configured-symlink retarget regression; and the
binary ordering assertion; plus the Selection V4 and Execution Disposition V2
direct-writer refusals. They prove only the
initial boundaries they drive. A same-name replacement, TOCTOU or physical
unplug after admission, a cloned/wrong device, and other direct library writers
that bypass the CLI binary remain limits, not hidden parts of RV-01/RV-02.

## Exact Candidate observations are sealed before Statistics V2 — D-0476

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| CO-01 | **Every Candidate row has one aligned observation for every exact accepted IST session, including explicit zero-trade sessions.** Only exact materialized `TradeRow` values can enter: entry/exit indices and timestamps must match the one-minute execution bars, cross-day trades refuse, the exit day owns the row, return is a checked sum of `worst`, wins count only `worst > 0`, and totals must exactly equal the evaluated `Cell`. NIFTY then BANKNIFTY pair only with an identical ordered session sequence and source policies | `cli::population_observations_v1::tests::exact_worst_rows_attribute_to_exit_sessions_and_preserve_zero_sessions`; `cross_day_foreign_timestamp_duplicate_semantic_and_overflow_refuse`; `cli::candidate_universe::tests::typed_producer_seals_both_full_grids_and_internal_commit_is_exactly_idempotent`; Candidate focused suite **10/10** green on 2026-08-31 | ✓ |
| CO-02 | **CSCV layout and scores cannot be caller-authored or resource-adjusted.** Version one selects the largest even divisor of the complete period count in `2..=16`, keeps every period in equal contiguous blocks, enumerates the exact canonical complementary-half family and derives both scores by checked summation. No divisor, overflow, padding, truncation, fallback or foreign/duplicate identity refuses | `cli::population_observations_v1::tests::deterministic_layout_uses_all_periods_equal_blocks_and_never_defaults`; `split_scores_are_checked_complete_and_identity_bound` | ✓ |
| CO-03 | **The companion observation authority becomes visible only after fixed-stride Data and Completion are synced and a fresh read-only reopen binds the complete opaque pair.** Exact retry reuses; one exact tail Data may receive only its Completion. Missing/non-directory root, foreign orphan, corrupt/stale bytes, explicit-bound breach or lock/data path replacement refuses. The file stores identities/counts/digests rather than raw period/split rows, so a crash rerun must derive the same observations again | `cli::population_observations_v1::tests::authority_is_receipt_last_freshly_reopened_and_exactly_reused`; `authority_refuses_absent_root_foreign_orphan_corruption_and_stale_bytes`; `replaced_lock_and_data_paths_refuse_cached_authority`; observation focused suite **7/7** green on 2026-08-31 | ✓ |

These invariants prove the Candidate-to-observation boundary and its companion
audit. They do not prove that Statistics V2, Finalization V2, successor
admission or an all-rung real stored caller consumes it. The physical-volume
and hot-unplug limits in D-0473/limits §155 also remain unchanged.

## Paired Observation-to-Statistics production keeps both stored causes — D-0480

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| OS-01 | **The crate-private Observation-to-Statistics V2 transaction can consume only the exact opaque stored NIFTY then BANKNIFTY Candidate/Pre-Admission capabilities and retains both beside the paired observations, fresh commits and detached Statistics projection.** Both source capabilities must name the same held canonical directory device/inode before and after every persistence boundary. The output roots must already exist. No bars, candidate rows, digests, split scores, network input or fallback can enter the transaction | `cli::step3_orchestrator::tests::paired_observation_statistics_seam_is_crate_private_and_source_retaining`; `observation_statistics_family_order_refuses_swapped_and_same_family`; `paired_source_roots_require_the_same_held_directory_identity`; `observation_statistics_roots_must_both_preexist_as_directories` | ✓ |
| OS-02 | **A successful pair freshly reopens Observation and Statistics and reconciles authority, pair, candidate, period, split and both family-source identities; exact retry preserves the same audits/projection and byte-identical output directories.** Pre-Admission comparison ignores only its physical append sequence and compares every semantic accessor, so NIFTY sequence zero and BANKNIFTY sequence one can share the ledger without weakening identity | `cli::step3_orchestrator::tests::stored_pair_commits_observation_and_statistics_then_reuses_exact_bytes`; `every_observation_statistics_crosswire_term_refuses`; `every_foreign_reopened_candidate_term_refuses_by_name`; orchestrator **14/14**, Observation **7/7**, Statistics **17/17**, strict CLI Clippy and formatting green on 2026-08-31 | ✓ |

The on-disk success fixture is deterministic and synthetic. These invariants
prove the bounded production seam and idempotence, not real-market correctness,
Admission V2, Finalization, the all-rung caller or Step-3 closure. Output-root
handles are checked through the final return but are not retained afterward;
the documented pathname replacement and physical-unplug limits remain.

## Anchored Admission V2 cannot hide incomplete chosen evidence — D-0481

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| AV-01 | **Every fold counted as decided by the opaque Admission V2 projection has one exact pessimistic OOS exit for its chosen ordinal.** A chosen fold with an absent OOS result refuses as `IncompleteChosenOos`; it cannot be dropped from the decided denominator or converted into an invented zero for profitability or aggregate return | `runner::validate::tests::anchored_admission_v2_refuses_a_decided_fold_without_an_exact_oos_exit`; focused Admission V2 **6/6**, validate **25/25**, complete Runner library **472/472** green on 2026-08-31 | ✓ |
| AV-02 | **The opaque captured score family contains exactly one retained row for every candidate the fold reports as priced.** Matching retained vectors are insufficient when their length differs from `priced`; the complete family refuses before it can contribute a candidate-family or walk digest | `runner::validate::tests::anchored_admission_v2_refuses_when_a_priced_candidate_was_dropped_before_capture`; strict Runner library/test Clippy and formatting green on 2026-08-31 | ✓ |
| AV-03 | **Zero resolved rungs cannot enter the anchored Admission V2 search or be silently replaced with the legacy default.** The public V2 door refuses before invoking its column builder, and capture plus semantic reconciliation repeat the invariant; legacy non-V2 callers retain their existing compatibility behaviour | `runner::validate::tests::anchored_admission_v2_refuses_zero_rungs_before_running_any_search`; complete Runner library **474/474** green on 2026-08-31 | ✓ |
| AV-04 | **A cross-crate consumer can inspect anchored search facts only through a non-constructible, non-`Debug` projection produced by full private seal reconciliation.** Typed policy/family/walk identities and four aggregate accessors cross the boundary; raw folds and chosen ordinals do not. Tampered seals, resealed zero policy and a wrong ordinal refuse, while compile-fail proofs reject construction and generic Debug logging | `runner::validate::tests::anchored_search_projection_is_typed_detached_and_seal_checked`; `anchored_admission_v2_refuses_oos_from_the_wrong_ordinal`; Runner doctests **3/3** and strict Runner library/test Clippy green on 2026-08-31 | ✓ |

These Runner invariants are arithmetic/provenance prerequisites only. They do
not turn the detached Runner result into a durable Candidate, Statistics,
Admission or Finalization authority. D-0482 defines the public projection
boundary; CLI must still bind it to freshly reopened durable lineage.

## Population Admission V2 is bounded structural durability, not production authority — D-0483

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| AV-05 | **One Admission V2 block is a fixed 2,048-byte-decision/4,096-byte-Completion NIFTY-first then BANKNIFTY family under five explicit nonzero ceilings.** The block binds every source-policy, family, Observation, Statistics, anchored-search, policy and cardinality term; Runner widths are aliases of Runner's canonical constants. Global/family ordering, duplicate semantics or decisions, crosswired family authorities, instrument-bearing aliases, unknown/reserved bytes and a changed terminal status refuse or rekey. Aggregate status counts cannot hide a status swap because status is part of each decision identity and the ordered Completion digest | `cli::population_admission_v2::tests::bounds_are_explicit_and_cover_all_five_axes`; `fixed_codecs_round_trip_and_reserves_are_zero`; `canonical_order_crosswire_duplicate_and_every_identity_rekey_refuse`; `instrument_bearing_family_aliases_refuse_but_content_hashes_may_match`; `status_swap_with_preserved_counts_rekeys_and_outer_reseal_refuses` | ✓ |
| AV-06 | **Admission V2 decisions become visible only through a receipt synced after the complete ordered decision block, and public reopen returns structural evidence rather than production authority.** Promotion is crate-private and succeeds only after dropping the writer, freshly reopening, rereading and exactly comparing the indexed bytes with an opaque private preparation. Exact reuse reissues both file durability barriers and the directory barrier. A complete orphan accepts only its exact retry and Completion in decision→Completion→directory order; a proper-prefix orphan, foreign retry, ragged/torn/corrupt/resealed/reordered/duplicate block, explicit-bound breach, same-length mutation or root/path replacement refuses without truncation or fallback | `cli::population_admission_v2::tests::persist_fresh_reopen_and_exact_reuse_promote_only_opaque_match`; `exact_reuse_executes_both_file_and_directory_durability_barriers`; `exact_orphan_retry_reissues_ordered_durability_barriers`; `orphan_decisions_accept_only_exact_retry_then_completion`; `ragged_torn_and_outer_seal_corruption_refuse`; `semantic_reseal_reordered_and_duplicate_disk_blocks_refuse`; `same_length_mutation_and_root_replacement_invalidate_open_view`; `existing_root_and_no_follow_are_fail_closed`; `over_bound_preparation_refuses_before_duplicate_set_allocation`; focused component **14/14** green on 2026-09-01 | ✓ |

AV-05/AV-06 prove a versioned bounded codec and ledger component. They do not
prove a production admission: no production constructor can currently create
the private preparation from reopened Step-3 authorities, and no Finalization
V2 or all-rung stored caller consumes the private authenticated result.

## Anchored Search V3 has no family-level direction — D-0484

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| AS-25 | **Each V3 candidate selects its own side from its own TRAINING mean edge: strictly negative is Short; positive, positive zero and negative zero are Long.** The public V3 constructor has no direction argument, its exhaustively destructured policy has no requested-direction field, and V2 is not convertible into V3 | `runner::validate::tests::anchored_search_v3_side_rule_covers_long_short_and_exact_zero`; `anchored_search_v3_projection_is_typed_sealed_and_rekeys`; the `AnchoredAdmissionValidationV2`→`AnchoredSearchValidationV3` compile-fail proof | ✓ |
| AS-26 | **A V3 fold with no winner has no side.** The shared search core represents the selected side as `Option<Direction>` and starts the no-winner arm at `None`; there is no caller fallback for the fold to inherit. Capture and reconciliation require every no-choice visible side/exit/value to remain absent | `runner::validate::tests::anchored_search_v3_no_winner_has_no_side_or_caller_fallback` | ✓ |
| AS-27 | **The V3 side rule and every selected candidate side are identity-bearing, while the public projection remains a detached equality surface only.** Separate V3 policy/family/candidate/walk domains rekey the successor; an unknown rule refuses even after hash recomputation, an altered selected side breaks the walk seal, and callers receive only typed policy/family/walk identities plus four aggregate facts | `runner::validate::tests::anchored_search_v3_projection_is_typed_sealed_and_rekeys`; construction/Debug/From-V2 compile-fail proofs | ✓ |

These are in-memory Runner provenance invariants. They do not claim a durable
search receipt, cross-feed equality, Admission/Finalization authority, real
stored-market execution, whole-run O(1), or constant retained space.

## Population Admission V3 and Finalization V3 retain exact successor evidence — D-0485

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| AV-07 | **One Admission V3 block is the exact canonical NIFTY-first then BANKNIFTY join of retained Candidate, Base Evidence, Observation, Statistics and Search V4 authorities.** Preparation authenticates the complete Statistics family once, fixed-offset joins every Base row by global/family ordinal and semantic identity, recomputes the canonical Runner V3 decision, and persists every terminal status without converting genuine V3 evidence into Admission V1. Receipt-last append, fresh reopen, exact reuse, valid-zero family, corruption, stale generation, path replacement and every source-identity crosswire fail closed | `cli::population_admission_v3::tests`; `cli::step3_orchestrator::tests::stored_pair_commits_observation_and_statistics_then_reuses_exact_bytes`; focused Admission V3 **10/10**, orchestrator **14/14**, strict CLI library/test Clippy green on 2026-09-01 | ✓ |
| AV-08 | **Finalization V3 can be produced only from the nonconstructible freshly reopened Admission V3 authority and preserves one ordered projection per Admission decision.** Its Finalization identity, ordered-row digest, family counts, terminal counts and Completion are recomputed from the authenticated bulk projection; data rows precede Completion and exact retry is byte-identical. A detached receipt or caller-authored Candidate/Statistics/Search/Base/Admission digest cannot mint the authority | `cli::population_finalization_v3::tests`; `cli::step3_orchestrator::tests::stored_pair_commits_observation_and_statistics_then_reuses_exact_bytes`; focused Finalization V3 **9/9**, orchestrator **14/14**, strict CLI library/test Clippy green on 2026-09-01 | ✓ |

AV-07/AV-08 prove the successor Admission/Finalization boundary only. A
Finalization row carries the Candidate-row digest rather than the authenticated
Candidate record bytes needed to construct final Population rows. Population,
Execution, Selection, Global Replay, operator surfaces and the real stored
Zerodha sweep therefore remain open and may not be inferred from these green
component suites.

## Browser bindings remain outside the host Rust graph — D-0486

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| HG-01 | **`wasm-bindgen`, `js-sys` and `web-sys` may remain target-gated entries in `Cargo.lock`, but none may be reachable from the current host workspace graph.** The check is graph-based rather than a lockfile-count allowlist, so an unchanged target-gated lock entry cannot hide a newly reachable host dependency. It changes no crate dependency and makes no claim about packages it does not name | `.github/workflows/ci.yml` Gate 13a runs `cargo tree --workspace --locked -i` for each named binding after the locked fetch and before the host build, and fails on any nonempty inverse tree; the same three commands produced no inverse tree on 2026-09-01 | ✓ |

This gate proves only host dependency reachability for the three named browser
binding families. The unrestricted `web/` tree remains outside the Rust crate
graph, and the tracked-file language/path gate remains the authority for every
other repository-boundary rule.

## Pre-Admission Data V2 authenticates natural zero-family extinction — D-0488

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| PAD-03 | **A zero-row Pre-Admission Data V2 value can be produced only from the exact authenticated Candidate Universe V1 production result whose sealed Completion proves natural extinction, complete closure, nonzero extinction depth, no unknown closure and exact zero-row reconciliation.** V2 is a separate 812-byte receipt-last format and retains every V1 source, family, stream, calendar, policy and load-bound identity; V1 remains byte-for-byte separate and continues to reject zero rows. Bare zero, removal of one proof term, semantic resealing, foreign/torn/ragged/corrupt history, stale same-length mutation and lock/data path replacement refuse; exact reuse and one exact receipt-less tail retry preserve bytes | `cli::pre_admission_data::tests::v2_zero_family_codec_requires_every_extinction_proof_term`; `v2_zero_family_ledger_is_receipt_last_reopenable_and_exactly_idempotent`; `v2_corrupt_resealed_ragged_and_stale_files_fail_closed`; `v2_replaced_lock_and_data_paths_refuse_cached_zero_family_audits`; complete focused V1+V2 module suite **13/13** green on 2026-09-01 | ✓ |

PAD-03 proves only this versioned Candidate-to-Pre-Admission component. No
Statistics, Admission, Population, Execution, Selection or Replay zero-family
successor consumes it. Observation V2 now authenticates the exact zero-family
production result under OZ-01; that narrow receipt is not a statistic, a real
stored sweep or Step-3 completion evidence.

## Observation V2 records zero-family extinction without invented rows — D-0489

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| OZ-01 | **Observation V2 can be produced only from one opaque Pre-Admission Data V2 production plus that exact production's receipt-last commit, and its sole disposition is a family-specific `NaturallyExtinct` authority with exactly zero observation rows.** Each independent 1,024-byte Data/Completion record embeds and revalidates the complete sealed 812-byte Pre-Admission V2 Data record, including Candidate/source identity and every extinction/closure term. V1 bytes and semantics are untouched. A foreign commit, nonzero family, caller-invented row, unsealed mutation, outer-resealed corrupt embedded source, ragged history, foreign tail retry, stale same-length edit and lock/data path replacement refuse; exact retry and the one exact tail retry are receipt-last and idempotent | `cli::population_observations_v1::tests::v2_zero_family_consumes_only_authenticated_extinction_and_keeps_family_identity`; `v2_authority_is_receipt_last_freshly_reopened_idempotent_and_orphan_safe`; `v2_ragged_corrupt_resealed_and_stale_authorities_fail_closed`; `v2_replaced_lock_and_data_paths_refuse_cached_zero_family_authority`; complete focused Observation V1+V2 suite **11/11** green on 2026-09-01 | ✓ |

OZ-01 proves only the zero-row Observation authority. It deliberately records
no period rows, split scores, statistics, assurance or admission decision. A
typed successor must join the nonempty sibling and explicitly represent
insufficient statistical evidence; this receipt alone is not Population,
Execution, Selection, Replay or a real stored-market sweep.

## Population V5 reaches Execution V3 only through retained source replay — D-0487

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| EX-01 | **The instrument term in Execution V3 is the domain-separated digest of the complete canonical `InstrumentKey`, not a symbol-only or CLI-local encoding.** Runner owns the sole structural encoder used by resolved exit grids; exchange, segment, kind, expiry, strike and option side therefore cannot be forgotten or independently reinterpreted. The retained Long and Short resolutions must reproduce the same full instrument digest | `runner::exit_grid_policy::tests::canonical_instrument_digest_binds_every_structural_field`; `cli::candidate_universe::tests::typed_producer_seals_both_full_grids_and_internal_commit_is_exactly_idempotent` | ✓ |
| EX-02 | **One-family Execution replay can be minted only by rebuilding the exact Candidate descriptor, execution column, run and complete resolved Long/Short grids from the retained stored context, then reproducing every authenticated Candidate row and Runner terminal disposition in canonical order.** Raw bars, grids, masks, coordinates, IDs and detached digests never cross the adapter. Exact replay is deterministic; a foreign Completion or one corrupt authenticated record refuses | `cli::candidate_universe::tests::typed_producer_seals_both_full_grids_and_internal_commit_is_exactly_idempotent`; `cli::step3_orchestrator::tests::every_foreign_stored_execution_term_refuses_by_name` | ✓ |
| EX-03 | **The sole production Execution V3 commit door consumes `CommittedStoredPopulationV5`, derives all four family/direction parameter blocks and every disposition from the retained authority, syncs fixed layout 5 data before Completion, freshly reopens exact bytes, and reauthenticates the upstream source before and after.** The resulting nonconstructible capability retains Population V5, so a Selection successor can recover authenticated strategy/mask/ranking facts without silently copying them into or widening the Execution codec. Exact retry reuses bytes; foreign/orphan Execution bytes, corrupt/stale held files and a changed retained Population/Finalization source refuse | `cli::step3_orchestrator::tests::stored_pair_commits_observation_and_statistics_then_reuses_exact_bytes`; `cli::execution_v3::tests::receipt_last_append_fresh_reopen_and_reuse_are_exact`; `every_receipt_last_crash_prefix_resumes_only_exact_bytes`; `foreign_or_out_of_order_orphans_are_refused_without_overwrite`; `retained_generation_symlink_hardlink_and_ragged_files_fail_closed` | ✓ |

These invariants close one stored NIFTY-plus-BANKNIFTY Population V5 block to
one Execution V3 block. They do not close the canonical eight-rung coordinator,
Selection V5, global replay, operator surfaces or the real Zerodha sweep.
Candidate reconstruction and Runner replay are input-dependent; preparation is
linear in retained rows plus percentile atoms, and generation authentication
hashes bounded files. Only admitted fixed-offset record addressing is O(1) in
record count. No whole-run, persistence, filesystem-latency or space claim is
made constant-time.

## Mixed-terminal Finalization V4 retains Admission and cannot invent Population rows — D-0496

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| FV4-01 | **Finalization V4 can be prepared only from the retained fresh-reopen Admission V4 authority and reproduces its exact receipt, common policy/cohort, NIFTY-first family lineages and evaluated decisions.** Every persisted Runner record is reverified; Decision order and Statistics Candidate order remain distinct. No detached receipt, row, digest, family, terminal, policy or status can mint semantic authority | `cli::population_finalization_v4::tests::write_fresh_reopen_exact_reuse_and_population_projection_are_exact`; complete focused suite **5/5** green on 2026-09-01 | ✓ |
| FV4-02 | **All nine ordered pairs of `Evaluated`, `InsufficientForCscv` and `NaturallyExtinct` survive unchanged.** Evaluated families have one real decision per Candidate; an insufficient family has one real Candidate and no decision/PBO invention; an extinct family has zero Candidate/statistic/decision rows and retains its exact Observation V2 proof | `cli::population_finalization_v4::tests::all_nine_terminal_pairs_preserve_shape_without_invented_rows`; `write_fresh_reopen_exact_reuse_and_population_projection_are_exact`; complete focused suite **5/5** green | ✓ |
| FV4-03 | **Finalization V4 becomes authoritative only after fixed Data/Family/Decision evidence is synced, Completion is synced last, the writer is dropped and a fresh read-only reopen equals the retained preparation.** Exact reuse and only one exact receipt-less prefix are accepted. Corrupt, ragged, foreign, reordered, duplicate, policy-crosswired, semantically resealed, stale same-length or pathname-replaced evidence refuses. Population projection reauthenticates the retained Admission source before and after the complete Finalization read | `cli::population_finalization_v4::tests::every_exact_trailing_prefix_recovers_but_foreign_and_ragged_refuse`; `policy_family_duplicate_corruption_and_resealed_crosswires_refuse`; `stale_same_length_and_path_replacement_refuse_retained_authority`; complete focused suite **5/5** green | ✓ |

These invariants close only the Admission V4-to-Finalization V4 library seam.
Existing Population V5 is V3-specific and is deliberately unchanged; a new
Population byte version must consume the neutral typed successor without
inventing Candidate rows for insufficient or extinct families. The production
all-rung caller, operator publication, real stored sweep and complete gate
matrix remain open.

## Canonical all-rung Execution retains named live authorities — D-0491

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| EX-04 | **All eight Execution V3 destinations and bounds are caller-explicit and retained by canonical rung name before the first append.** A destination must already be a nonsymlink directory whose final component is exactly `1min`, `2min`, `3min`, `5min`, `10min`, `15min`, `30min` or `60min`; every destination is physically/canonically disjoint from every other destination and the complete retained Population topology. Swapped, missing, duplicate, nested, Population-overlapping, symlinked or pathname-replaced roots refuse. Eight distinct bounds survive the named projection without a default or shared value | `cli::all_rung_population_v5::tests::execution_roots_and_bounds_are_explicit_canonical_and_rechecked`; `reordered_missing_duplicate_nested_and_population_execution_roots_refuse`; `retained_execution_root_symlink_and_path_replacement_refuse`; complete focused module suite **9/9** green on 2026-09-01 | ✓ |
| EX-05 | **A retained all-rung Execution authority can remain live only when every literal 60→3,600-second rung still joins its exact Population receipt, ordered-row digest, row count, global sequence, Population-row identity and NIFTY-then-BANKNIFTY family topology under the exact committed bound.** The coordinator consumes the opaque Population wrapper by value, destructures its private array once and calls only D-0487's production door in literal canonical order; its result keeps eight named authorities and paired receipts private rather than releasing a reorderable array | `cli::all_rung_population_v5::tests::canonical_topology_is_exact_nifty_first_banknifty_second_and_eight_rungs`; `row_topology_authenticates_receipt_counts_and_canonical_order`; `foreign_bounds_population_and_disposition_topology_refuse`; D-0487 `cli::execution_v3::tests::receipt_last_append_fresh_reopen_and_reuse_are_exact`, `every_receipt_last_crash_prefix_resumes_only_exact_bytes`, `foreign_or_out_of_order_orphans_are_refused_without_overwrite`, and `retained_generation_symlink_hardlink_and_ragged_files_fail_closed`; focused all-rung module **9/9** green on 2026-09-01 | ✓ |

The focused all-rung suite exercises its topology, root and identity checkers;
the lower D-0487 suite exercises one-rung persistence, fresh reopen, exact
reuse, prefix recovery and stale/corrupt refusal. It does **not** construct and
persist eight complete Population-to-Execution chains in one test, physically
inject ENOSPC/hot-unplug, wire Selection/Replay/operator surfaces, or run real
stored market data. The coordinator currently has no non-test caller, so its
focused build emits dead-code warnings and is not strict-Clippy closure.

## Selection V5 consumes retained Execution V3 without caller-authored ranking facts — D-0490

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SV-01 | **The sole one-rung Selection V5 production door consumes and retains `CommittedStoredExecutionV3`, exact-joins every freshly authenticated Population V5 row to its reopened durable Execution V3 disposition in canonical NIFTY-first/BANKNIFTY order, and derives mask, direction, strategy and ranking metrics only from authenticated Candidate and Admission bytes.** Missing, duplicate, foreign or reordered rows, stale source identities, a Population/Execution semantic mismatch and an admitted row with absent or contradictory required metric evidence refuse. Both source families must be nonempty; the winner set itself may contain only one family | `cli::selection_v5::tests::eligibility_family_completeness_and_strategy_aliases_fail_closed`; `complete_two_family_source_may_legitimately_select_one_family_only` | ✓ |
| SV-02 | **Selection V5 runs Runner's canonical deterministic two-pass Top-25 over the complete eligible population, binds the exact ranking policy and ordered-population proof, persists receipt-last fixed layout-5 bytes, freshly reopens exact preparation and returns Top-10 only as the first ten Top-25 rows.** Deterministic ties, changed policy identity, exact write/reopen/reuse, every valid orphan prefix, foreign/reordered prefix, corruption and stale/path replacement all fail closed or recover only by exact bytes | `cli::selection_v5::tests::one_combined_top_twenty_five_has_exact_ties_and_one_top_ten_prefix`; `ranking_policy_identity_is_bound_and_exact_policy_rerun_reuses`; `receipt_last_commit_fresh_reopen_and_exact_reuse_are_byte_identical`; `every_exact_winner_prefix_recovers_but_foreign_or_reordered_prefix_refuses`; `orphan_prefix_source_identity_mutations_refuse`; `corruption_ragged_files_replaced_children_and_replaced_root_fail_closed` | ✓ |

SV-01 and SV-02 prove the nonempty one-rung authority seam and its bounded
ledger. They do not prove a zero-family successor, the canonical eight-rung
Selection join, Global Replay V3, publication surfaces or the final real
stored-market sweep. Complete ranking, source authentication and persistence
are linear or filesystem-dependent work; only fixed-stride addressing is O(1)
in record count after admission, and hash lookup is average O(1).

## Population Statistics V3 preserves extinction as typed absence — D-0492

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| PS3-01 | **Every Statistics V3 family has exactly one terminal whose shape is enforced rather than caller-labelled.** `Evaluated` has at least two real Candidates plus complete White, SPA, Romano--Wolf, Wilson and CSCV/PBO evidence; `InsufficientForCscv` has exactly one real Candidate and retains every statistic that is measurable while carrying no PBO; `NaturallyExtinct` has zero Candidate/period/split rows and no numeric statistic, bootstrap procedure or zero placeholder. One nonempty and one extinct family work in either orientation, and two exact extinct families form a truthful all-extinct authority | `cli::population_statistics_v3::tests::terminals_refuse_missing_or_fabricated_evidence_and_cover_both_orientations`; `all_extinct_requires_two_exact_authenticated_observation_v2_commits`; complete focused Statistics V3 suite **6/6** green on 2026-09-01 | ✓ |
| PS3-02 | **Statistics V3 derives ordered NIFTY-first/BANKNIFTY lineage only from authenticated Observation sources and binds every real statistic and count into the sealed authority.** The evaluated side consumes one exact Observation V1 family plus its matching Pre-Admission audit; the extinct side consumes the opaque Observation V2 production plus its exact receipt-last commit. Crosswired family/terminal/source status, a foreign commit, missing evaluated evidence, fabricated extinct evidence and semantically resealed candidate bytes refuse. No public constructor accepts rows, terminal labels, statistics or identity digests | `cli::population_statistics_v3::tests::terminals_refuse_missing_or_fabricated_evidence_and_cover_both_orientations`; `all_extinct_requires_two_exact_authenticated_observation_v2_commits`; `receipt_last_exact_reuse_candidate_lookup_and_foreign_commit_refusal`; `corrupt_and_resealed_candidate_bytes_refuse` | ✓ |
| PS3-03 | **Statistics V3 persists evidence before its adjacent Completion, admits only bounded fixed-stride history and freshly reopens the exact authority.** Exact committed retry is byte-identical; one exact receipt-less tail prefix may resume, while ragged/torn mismatch, corrupt or outer-resealed records, stale same-length mutation and lock/data pathname replacement fail closed. Candidate lookup is by authenticated fixed offset only after the complete block has passed validation | `cli::population_statistics_v3::tests::receipt_last_exact_reuse_candidate_lookup_and_foreign_commit_refusal`; `torn_prefix_retries_exactly_and_ragged_tail_refuses`; `corrupt_and_resealed_candidate_bytes_refuse`; `stale_handle_and_named_path_replacement_fail_closed` | ✓ |

These invariants close only the Statistics successor seam. Statistics V2 bytes
and paired-nonempty semantics remain unchanged. The opaque V3 Admission source
has no non-test consumer yet, and no Admission/Finalization/Population
extinction successor, all-rung Selection, Global Replay, operator surface,
full-gate matrix or real stored-market sweep is proved by the focused suite.

## Canonical all-rung Selection V5 is a named retained authority — D-0494

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| SV-03 | **All eight Selection V5 roots and bounds are named and admitted before the first append, and every Selection root remains physically/canonically disjoint from every other Selection root and every retained Population/Execution root.** The opaque topology token retains all three live root generations and reauthenticates them after all eight commits and on subsequent audits; swapped, duplicate, nested, upstream-overlapping, symlinked or pathname-replaced roots refuse without exposing paths or detachable root receipts | `cli::all_rung_population_v5::tests::all_rung_selection_roots_are_named_disjoint_from_every_upstream_root_and_pre_admitted`; `all_rung_selection_retained_root_symlink_and_path_replacement_refuse`; focused all-rung Selection suite **5/5** green on 2026-09-01 | ✓ |
| SV-04 | **The coordinator consumes the opaque all-rung Execution V3 authority once and invokes the sole one-rung Selection door in literal 60→3,600-second order.** Each rung must reproduce its exact Execution receipt, bound, ranking policy, Top-25, Top-10 prefix and durable selected-exit join; Selection/Completion/Population/Execution/winner identities cannot cross rungs. The successor surface consumes eight named live authorities, preflights every Top-25 and the retained topology before the first callback, then yields the fixed 8×25 cohort in rung-major/rank-major order without exposing an array, receipt, path or caller-writable winner fact | `cli::all_rung_selection_v5::tests::canonical_rungs_and_cross_rung_receipts_are_exact`; `foreign_counts_rungs_and_cross_rung_identities_refuse`; `exact_top_ten_prefix_refuses_missing_reordered_or_foreign_rows`; D-0490 focused Selection suite **11/11** green on 2026-09-01 | ✓ |

These invariants close only the current paired-nonempty all-rung Selection
authority. The focused test does not construct all eight complete persisted
upstream chains in one fixture, and no Global Replay V3, operator surface,
full-gate matrix or real stored-market sweep follows from it. Selection V5
cannot truthfully represent natural zero-family extinction; that remains a
separately versioned successor. Authentication/ranking/persistence are linear
or filesystem-dependent; the fixed dispatch and bounded 200-winner handoff do
not make the complete coordinator O(1).

## Global Replay V3 consumes authority instead of caller facts — D-0493

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| GR3-01 | **The V3 production join consumes one preflighted all-rung Selection V5 successor and exactly 200 opaque Runner OOS replay capabilities in canonical rung/rank order.** Runner mints each capability only at the authenticated selected-exit replay boundary and binds the exact full swept instrument, canonical stored feed, direction, first-OOS boundary, run, selected exit and replay candidates. Global Replay exact-joins selected exit, instrument family, direction and OOS causality before creating a private witness; unknown feed, missing/extra/reordered capability, foreign exit/instrument/direction, pre-OOS signal, duplicate row or mixed/reused Selection authority refuses | The typed join compiles against D-0494's 5/5-green all-rung successor; `runner::exit_grid_policy::tests::global_replay_witness_mints_every_identity_at_the_authenticated_replay_door`; `cli::global_replay_v3::tests::malformed_or_noncanonical_authority_sets_refuse_before_publication`; Runner mint **1/1** and focused V3 **5/5** green on 2026-09-01. No one focused fixture constructs both upstream opaque capabilities together | ✓ |
| GR3-02 | **All 200 streams compete under one global inclusive single-position lock across instrument, direction and rung.** Reachable price-refused attempts still occupy; same-minute ties are deterministic; a following entry is eligible only strictly after the admitted occupied-through minute. Money exists only for globally admitted priceable decisions, and same-feed India VIX is stamped afterward as exact-minute or typed-absent reference evidence without changing replay identity, execution or P&L | `cli::global_replay_v3::tests::fixed_topology_and_one_inclusive_global_lock_are_enforced`; `malformed_or_noncanonical_authority_sets_refuse_before_publication` | ✓ |
| GR3-03 | **V3 uses independent fixed-stride Witness/Candidate/Decision/Money/Completion codecs and persists Completion last under explicit bounds.** Exact retry is byte-identical; fresh reopen decodes and validates every record, reconstructs the global scheduler and VIX publication from persisted evidence and requires exact semantic equality. The sole crate-level commit door returns an opaque committed capability only after that fresh reopen reproduces the prepared identity, counts and counters. Corrupt bytes, torn/ragged tails, foreign money and a valid receipt attempting to hide incompatible data refuse; V1/V2 bytes are never relabelled | `cli::global_replay_v3::tests::every_v3_codec_round_trips_and_refuses_one_corrupt_byte`; `receipt_last_commit_reuses_exact_bytes_and_reopen_replays_semantics`; `torn_tail_and_foreign_money_are_never_hidden_by_a_valid_receipt` | ✓ |

These invariants prove the controlled V3 authority, scheduler and ledger seam,
not the complete Step-3 production workflow. A crate-private production commit
door exists, but no upstream Step-3 orchestrator or
CLI/API/database/audit/monitoring/dashboard caller invokes it yet; no real stored
market sweep ran, and no complete workspace/coverage/mutation gate is claimed.
Complete replay work and storage are input proportional or system dependent;
only admitted fixed-stride address arithmetic is worst-case O(1).

## Statistics V3 reaches a typed mixed-terminal Admission V4 authority — D-0495

| ID | Invariant | Proof | ✓ |
|---|---|---|---|
| PA4-01 | **Admission V4 preserves literal NIFTY-first/BANKNIFTY terminal shape.** An evaluated family has one freshly verified Runner V3 exact-grid decision per real Candidate; an insufficient family retains its one real Candidate lineage but has no invented PBO/draft/decision; a naturally extinct family retains its authenticated Observation V2 proof with no Candidate, statistic or decision row. The block and every decision bind one canonical Runner policy, and changing only the outer policy or a family/Search identity refuses | `cli::population_admission_v4::tests::terminals_preserve_both_mixed_orientations_insufficient_and_all_extinct`; `reordered_family_and_corrupt_or_resealed_decision_refuse`; focused Admission V4 **5/5** and Statistics V3 **7/7** green on 2026-09-01 | ✓ |
| PA4-02 | **The version-separated ledger is bounded, fixed-stride, append-only and receipt-last.** Data, exact NIFTY Family, exact BANKNIFTY Family and every Decision are synchronized before Completion. Exact retry reuses identical bytes and one exact receipt-less suffix can resume; ragged/foreign/reordered/noncanonical/corrupt/resealed history, stale same-length mutation and lock/data path replacement fail closed | `cli::population_admission_v4::tests::receipt_last_write_reopen_reuse_and_finalization_projection_are_exact`; `exact_trailing_prefix_recovers_but_foreign_or_ragged_prefix_refuses`; `reordered_family_and_corrupt_or_resealed_decision_refuse`; `stale_same_length_and_named_path_replacement_refuse` | ✓ |
| PA4-03 | **A Finalization-facing projection can be minted only by the retained freshly reopened Admission V4 authority and reproduces the exact common source, two family lineages and ordered evaluated-Candidate decisions.** A structural receipt alone is insufficient; the authority compares every decoded record with the opaque prepared value and re-verifies the detached Runner decision arithmetic before projection | `cli::population_admission_v4::tests::receipt_last_write_reopen_reuse_and_finalization_projection_are_exact`; `stale_same_length_and_named_path_replacement_refuse` | ✓ |

These invariants prove the mixed-terminal Admission V4 library authority and
its typed Finalization handoff, not a persistent Finalization V4 authority or
the complete production chain. The production preparation door statically
consumes Statistics, Candidate, Base Evidence and retained/durable Search
authorities and contains no caller-authored digest/status parameter, but the
five focused tests use controlled private prepared values; no one focused
fixture constructs every upstream opaque authority together. A
version-separated Finalization V4 codec/receipt-last ledger, all-rung caller,
operator surfaces, full gate matrix and real stored sweep remain open. Whole
authentication, policy evaluation, hashing, persistence and reopen are linear
or system dependent; fixed-record address arithmetic alone is worst-case O(1).

## Stored post-training OOS evidence is derived, opaque and version-neutral — D-0497

| ID | Invariant | Proof | State |
|---|---|---|---|
| OOS-01 | **A stored post-training OOS cohort can name only the feed, full swept index, rung, commit, evaluator, horizon, ladder, calendars and dynamic Long/Short training grids retained by the authenticated stored Candidate transaction.** Its caller supplies only a civil month span and explicit signal/minute/daily record ceilings. The common bounded loader must return complete stored signal, previous-day and exact-minute evidence, and the first requested execution minute must be strictly later than both matching training grids' last minute | `cli::step3_orchestrator::tests::stored_post_training_oos_cohort_is_exact_and_mints_only_opaque_witnesses`; `stored_post_training_oos_refuses_absent_overlap_and_preallocation_overflow` | ✓ — controlled OOS suite **3/3** green on 2026-09-01 |
| OOS-02 | **The cohort seal binds the held root generation, canonical source and complete OOS bytes/boundaries before Runner may mint a witness.** Long and Short training authorities must agree; missing/incomplete/over-bound/overlapping evidence, cross-family dispositions and deterministic root substitution refuse. Exact retained inputs derive the same cohort and witness IDs. The witness remains opaque and moves together with its cohort identity; callers cannot write feed, instrument, direction, mask, exit, timestamp, price or digest fields | `cli::step3_orchestrator::tests::stored_post_training_oos_cohort_is_exact_and_mints_only_opaque_witnesses`; `stored_post_training_oos_refuses_crosswired_and_replaced_sources` | ✓ — controlled OOS suite **3/3** green on 2026-09-01 |
| GR4-01 | **Terminal-aware Global Replay must consume Selection V6's actual zero-through-twenty-five winner prefix for each of eight named rungs and preflight every matching opaque OOS witness before the first durable append.** The complete actual set then shares one inclusive global occupancy lock; V3's fixed 8x25 topology cannot be silently reused for extinct/insufficient families | D-0497 contract map in `docs/07-plan.md`; implementation waits for the frozen Selection V6 move type | Open by design |

OOS-01/OOS-02 have a measured **3/3** controlled-fixture receipt and a green
focused CLI library check. The strict shared-tree Clippy attempt remains red
(115 library and 126 library-test diagnostics); the four diagnostics it found
in this new cohort/orchestrator boundary were corrected statically afterward,
and a post-correction strict rerun remains open. GR4-01 is a locked successor
contract, not implemented code. None of these rows proves CLI/API/database/audit/dashboard
publication, physical file replacement after an already-owned immutable
snapshot, hot unplug, profitability, a complete workspace gate or a real
stored Zerodha sweep. Store loading, causal column derivation, replay, hashing
and authentication are input proportional; filesystem synchronization and
device latency are system dependent. No whole-cohort or whole-replay O(1)
time, space or latency claim is made.

## Candidate reachability, terminal-aware Population V6 and Execution V4 — D-0498

| ID | Invariant | Proof | State |
|---|---|---|---|
| CU-03 | **A nonempty Candidate V1 authority can never be a singleton.** Its exact row count is `closed_itemsets * (Long cells per mask + Short cells per mask)`; both production directions are nonempty, so every nonzero receipt has at least two rows and its canonical sequence contains both Long and Short | `cli::candidate_universe::tests::nonempty_candidate_receipt_is_exact_two_sided_mask_expansion`; the receipt validator independently recomputes the same product and rejects nonzero `< 2` | Static implementation; focused Cargo proof pending |
| PS3-04 | **The two-evaluated Statistics V3 production door consumes two genuine evaluated Observation/Pre-Admission authorities, derives canonical NIFTY-first/BANKNIFTY order and exact common source identity, and changes no V3 bytes or caller-visible terminal meaning.** No family/status/count/digest/statistic enters; swapped, crosswired and stale audits refuse | `cli::population_statistics_v3::tests::two_evaluated_production_preserves_real_lineage_and_refuses_order_crosswire_and_stale_audit` | Static implementation; focused Cargo proof pending |
| PV6-01 | **Population V6 admits exactly the four Candidate-V1-reachable ordered terminal pairs E/E, E/X, X/E and X/X and refuses all five pairs containing `InsufficientForCscv`.** Evaluated families retain their exact Candidate authorities; extinct families retain only their exact Finalization V4 terminal envelopes and invent no Candidate/statistic/decision row. NIFTY remains first even when it has zero rows | `cli::population_v6::tests::terminal_reachability_matrix_accepts_four_pairs_and_refuses_five_singleton_pairs`; `candidate_duplicates_gaps_crosswires_and_nested_corruption_refuse` | Static implementation; focused Cargo proof pending |
| PV6-02 | **The Population V6 fixed codec exact-joins every real Candidate V1 canonical record to Finalization/Admission/Statistics/Base/Search/Runner lineage in one global NIFTY-then-BANKNIFTY sequence.** Candidate, evaluated and decision counts are distinct persisted fields and reconcile exactly, including truthful zero. Duplicate/gapped/crosswired ordinals, corrupt embedded Candidate or Runner bytes, foreign prefix, ragged tail, same-length mutation, stale handle, symlink root and exact-byte pathname replacement refuse | `cli::population_v6::tests::candidate_duplicates_gaps_crosswires_and_nested_corruption_refuse`; `every_exact_prefix_recovers_but_foreign_and_ragged_prefixes_refuse`; `same_length_corruption_stale_handle_and_named_path_replacement_refuse` | Static implementation; focused Cargo proof pending |
| PV6-03 | **Population V6 writes Data/Families/Candidates before Completion, freshly reopens the complete bounded block, reuses only byte-identical history and retains the live upstream authority.** Its only Execution V4 source carries the exact receipt, both terminal envelopes, common rung/horizon, 0/2/4 real Long/Short parameter capabilities and the complete globally ordered Candidate/Runner-disposition join; no detached identity or caller row can construct it | `cli::population_v6::tests::genuine_evaluated_pair_commits_reopens_reuses_and_mints_exact_execution_source`; `every_exact_prefix_recovers_but_foreign_and_ragged_prefixes_refuse` | Static implementation; focused Cargo proof pending |
| EV4-01 | **Execution V4 consumes and retains Population V6 by value and persists exactly two parameters for each evaluated family and none for an extinct family.** Completion retains both terminal envelopes and four fixed family/direction identity slots; zero slots are legal only for their extinct family. X/X has authenticated nonzero empty-set digests and zero parameter/percentile/disposition rows; mixed families keep canonical active-family ordering | `cli::execution_v4::tests::all_extinct_block_retains_terminals_and_nonzero_empty_authorities`; `mixed_evaluated_extinct_topologies_persist_in_both_family_orders`; `terminal_envelopes_reject_singletons_missing_rows_and_extinct_rows` | Static implementation; focused Cargo proof pending |
| EV4-02 | **Execution V4 persistence is fixed, bounded and receipt-last across parameter, percentile, disposition and Completion files.** Every exact crash prefix resumes only matching bytes; corrupt/resealed/reserved/foreign/out-of-order/ragged/stale/symlink/hardlink state refuses. The fresh authority reauthenticates retained Population before and after durable reads and exposes only an opaque Selection V6 source | `cli::execution_v4::tests::fixed_codecs_are_canonical_and_fail_closed`; `receipt_last_append_fresh_reopen_and_reuse_are_exact`; `every_receipt_last_crash_prefix_resumes_only_exact_bytes`; `mixed_topology_crash_prefixes_resume_for_nifty_or_banknifty_first_parameter`; `foreign_or_out_of_order_orphans_are_refused_without_overwrite`; `retained_generation_symlink_hardlink_and_ragged_files_fail_closed` | Static implementation; focused Cargo proof pending |

These rows correct production reachability; they do not erase the older codec
shape or mutate V3/V4/V5 history. They also do not prove the full all-rung
Population/Execution chain, Selection V6, Global Replay V4, operator surfaces,
physical storage-fault behavior, full workspace gates or a real stored sweep.
All complete-source authentication, joins, hashing, persistence and reopen are
bounded but input/file/system proportional. Only fixed-record addressing after
admission is worst-case O(1), and identity-map lookup is average O(1).

## Charter non-regular sessions are withheld from every swept series — D-0502

**This group opens at `SC-06` rather than `SC-01`, and the gap is deliberate.**
It was written as `SC-01` and so was the Step-3 comparison group above it, which
was issued first — `36f2350b`, 2026-09-01, against `11feb080`, 2026-09-02. Two
rows shared one id, and CI gates 10b and 27 both fail on a duplicate. Neither
had ever run: the `push:` trigger is `branches: [main]` and this branch is more
than 1,300 commits ahead, so both gates were red and silent.

The older row keeps the id because ids are cited and citations must not move.
`docs/05-decisions.md` and `docs/06-limits.md` each name `SC-01`, and both mean
the D-0462 row. This one had no citation anywhere in the tree, so it is the one
that moved — the same rule `docs/05-decisions.md` D-0104 applies to its own
duplicate decision numbers, which it declines to renumber precisely because
forty citations point at them.

`SC-02` through `SC-05` below are untouched: renumbering a row that is already
correct to make a table read tidily is the kind of churn append-only history
exists to refuse. The group is out of numeric order and correct.

| ID | Invariant | Proof | State |
|---|---|---|---|
| SC-06 | **Every bar whose IST day appears in `CHARTER_NON_REGULAR_IST_DAYS` is withheld from every swept series, on every rung except `1day`, by one act at one place.** `load_classified_with_ceiling` is the sole decode path for `load`, `load_span`, `load_daily_context`, `load_exact_minute_context` and both bounded twins, so the signal series, the exact one-minute `GapFib` context and the one-minute execution path cannot disagree about which sessions a run saw | `cli::stored` — the filter is inside the single shared decoder; `1day` exemption pinned by `Timeframe::DAY_1` type comparison | Static implementation; the consistency property is structural, not asserted by a focused test |
| SC-02 | **A withheld day's bar is CHECKED before it is dropped, and a bar the measured calendar cannot place refuses.** The test is `expected_buckets_v2` bucket containment, the same geometry the receipt uses, so the loader and the receipt agree by construction. MEASURED: dhan `NIFTY/1min/2024-03.bin` holds 6 857 records against zerodha's 6 855, the difference being two bars on IST day 19 784 at minutes 600 and 750, one past each measured window edge | `cli::stored::refuse_uncalendared_withheld_bar`; the two out-of-window bars are refused rather than silently dropped, preserving the existing `require_canonical_minute` and `hash_offered_calendar_v2` refusals | Static implementation; focused Cargo proof pending |
| SC-03 | **Calendar receipt policy 3 expects zero buckets on a withheld day, hashes it under a tag of its own, and refuses any timestamp offered on it.** A withheld day is not hashed as `DayKind::Closed`: a holiday and a withheld drill are different facts and must not share a digest. The day's real geometry is still measured into `withheld_buckets`, so the receipt states the size of what it removed | `cli::stored::tests::calendar_receipt_v2_withholds_split_and_short_exceptions_on_all_eight_rungs` — asserts the refusal on offered bars AND `expected == 0`, `withheld_buckets == {105,54,35,21,12,7,5,3}` / `{60,30,20,12,6,4,2,2}` across all eight rungs | Proven by focused Cargo test |
| SC-04 | **The swept-series calendar policy reaches the run identity appended, never inserted, and reaches the daily-reference policy identity too.** Tag 11 of `data_digest_with_daily_reference` sits after all ten existing tags; `daily_reference_policy_digest_v1` hashes it after the excluded-day list. Binding the VERSION rather than only its consequence re-keys even a span holding none of the four days | `runner::identity::data_digest_with_daily_reference`; `cli::stored_data_completeness::daily_reference_policy_digest_v1` | Static implementation; focused Cargo proof pending |
| SC-05 | **The withholding is stated wherever the sample size is stated.** `span_banner` names the withheld days as dates and counts their bars beside `MONTHS MISSING FROM THIS SPAN`; `daily_reference_note` carries the exact 1min stream's withheld days; a span emptied entirely by the calendar refuses with its own sentence rather than the absent-months one | `cli::span_banner`; `cli::daily_reference_note`; `cli::stored::empty_span_refusal` | Static implementation; focused Cargo proof pending |
| SC-07 | **The VWAP verdict of a stored run is the instrument's KIND, decided before the first bar, and no production stored path pins it.** `vwap_availability` takes a key and no bars, so there is nothing to look ahead into; an equity is `Present`, an index — including both swept indices and India VIX — is `Absent` exactly as every NIFTY row was ever computed, and a contract is `Absent` rather than a panic. Exactly two `Availability::Absent` literals remain in `cli/src/lib.rs` production code, both named (the synthetic evaluator; `audit_bars`' `Err` fallback), and every `stored_anchored_column(` call passes a verdict. The per-fold evaluators are built to the caller's verdict through `Evaluator::availability()`, so a fold never sweeps a different vocabulary from the span it validates. D-0507 | `cli::stored::tests::vwap_availability_is_decided_by_the_kind_and_reads_no_bar`; `cli::tests::the_stored_paths_take_the_vwap_verdict_from_the_key_and_never_pin_it`; `indicators::evaluator::tests::availability_is_the_verdict_the_evaluator_was_built_with` | Cargo test |
| SC-08 | **`cli pool` pools what adds and bounds what does not, prices every candidate on exactly the column the screen would build, and fabricates no row.** The surface it screens is `swept_index` over the catalog — a stored equity off the F&O list, a reference index and any other rung are skipped without a second list. Trades, wins, net, gross win and gross loss are sums across instruments; the worst trade and the smallest win are minima; the drawdown column is the LARGEST single-instrument drawdown, a lower bound, and is labelled `dd>=`. A refused instrument contributes to no candidate and is named; a candidate that fired nowhere is dropped, never ranked as a zero-trade winner. `price_all` names the same loaders and checks as `screen_range_inner` in the same order, so a pooled cell is the cell `range-rung` would show. Ratios are in hundredths, as the grid's are, so the tail rule compares directly with `Rules::min_rr_bp`. A pooled rank whose candidate index is missing is refused in the report, not indexed. The pooled table is not written to the store, and the report says so. D-0509 | `cli::pool::tests::the_surface_is_the_swept_index_and_not_everything_stored`; `cli::pool::tests::the_fold_adds_totals_and_takes_the_extremes`; `cli::pool::tests::refusals_and_unfired_candidates_are_never_folded_in`; `cli::pool::tests::the_ranking_leads_with_the_rule_and_the_drawdown`; `cli::pool::tests::ratios_render_as_multiples_and_the_sentinel_as_words`; `cli::pool::tests::the_pool_prepares_a_span_exactly_as_the_screen_does`; `cli::pool::tests::a_missing_pooled_candidate_is_reported_not_indexed`; `cli::tests::every_command_is_listed_in_both_places` | Cargo test |

These rows do not prove the full eight-rung sweep over the real store, and they
do not re-measure the bench gate. All four `pull::calendar::IRREGULAR` entries
and all five `LENGTH_UNMEASURED` entries are charter days, so after policy 3 no
split or short session is reachable from production input at all — the split and
short bucket geometry in `expected_buckets_v2` remains correct and tested but is
exercised only by tests. `crates/api`'s five independent store readers are
deliberately unfiltered: they serve what is ON DISK, which is a different
question from what a run SWEPT, and the store itself is never modified.

## The exit grid, the validation boundary and the evaluable candle — D-0503, D-0504, D-0505

| ID | Invariant | Proof | State |
|---|---|---|---|
| XG-01 | **The exit grid's width does not depend on how many rungs were asked for.** `rungs_within_cell_budget` sizes from `whole_machine_ceiling`, not `derived_ceiling`: the `checked_div(threads)` below it already bounds how many transient grids exist at once, and `SWEEPS_SHARING_THIS_MACHINE` bounds RETAINED state, so applying both counted one concurrency twice. MEASURED on fourteen cores: one rung asked solved to 7 rungs and 6,784 cells, eight rungs asked solved to 4 and **625** | `cli::tests::the_exit_grid_width_does_not_move_when_sweeps_share_the_machine` — asserts equality across `SharedBy::these(8)` and asserts the solved width is at or above five, so a divided budget leaking back in cannot pass | Proven by focused Cargo test |
| XG-02 | **The grid width reaches the answer, so XG-01 is a §3 rule 5 property and not a performance one.** The width selects the winning `Cell`, so `trades`, `pessimistic`, `optimistic`, `worst_trade`, `max_drawdown` and all five `exit_rungs` move with it — while the run identity folds `ceiling_asked`, the undivided figure, so three different searches keyed one way | `cli::policy_of` folds the grid rungs as identity term 3; `cli::ceiling_asked` is the undivided term 6 pinned by D-0501's test | Static implementation; the consequence is argued in D-0503, not asserted by a separate test |
| XG-03 | **The ladder's memory bound is untouched by XG-01.** `ladder_within` still takes `ceiling_from_env`, which is `shared_out(ceiling_asked())` — the division that exists because eight rungs each claiming the whole machine asked 157 GB of 48 | `cli::ceiling_from_env`; `an_explicit_ceiling_is_divided_among_sweeps_like_a_derived_one` and `the_ceiling_is_derived_from_this_machine_and_not_from_an_assumed_one` both still pass | Proven by existing Cargo tests, re-run with XG-01 |
| VS-01 | **Every validation stage emits a matched entered/finished pair, or neither.** `timed_validation_stage` takes the closure rather than being two calls, so an early return or a panic inside `both_shapes`, `overfitting_of` or `bootstrap_family` cannot leave an `entered` with no `finished` — the shape `SharedBy` takes for the same reason | `cli::timed_validation_stage` — the second `note_validation_stage` is unreachable except through the first | Static implementation; the RAII shape is structural |
| VS-02 | **A run with validation off logs no validation stage.** The emitters sit INSIDE `validated_if`'s closure and inside `validate.then(..)`, not around them, so `BRUTEX_VALIDATE=0` produces no walk-forward, PBO or bootstrap boundary | `cli::audit_bars` — all three call sites | Static implementation; focused Cargo proof pending |
| VS-03 | **The boundary is emitted from `cli` and can never move deeper.** Gate 17 silences `vocab engine indicators runner`, and all three stages execute inside `runner`; `cli` is the innermost crate that may speak | `.github/workflows/ci.yml` gate 17, `swept='vocab engine indicators runner'` | Proven by CI gate |
| CE-01 | **Every module refuses exactly what the aggregate refuses.** `CurDayFib`, `Patterns`, `Orb`, `SessionState`, `GapFib`, `TrendState` and `Vwap` call `Candle::check_evaluable`, which is `check` plus the zero-price refusal `Evaluator::stepped` applies. A module accepting a bar the aggregate refuses is a partially-evaluated mask — some families emitted, others refused | `indicators::tests::every_module_refuses_exactly_what_the_evaluator_refuses` — walks the whole corruption matrix against all seven | Proven by focused Cargo test |
| CE-02 | **`Candle::check`'s own contract is unchanged, and an all-zero candle is still structurally sane to it.** The zero refusal is a SECOND predicate rather than a fifth clause, because fourteen callers read `check` as the structural question and three fixtures build extreme bars on that reading | `indicators::candle::a_defaulted_candle_is_all_zero_and_its_open_interest_is_a_real_zero` — still asserts `Ok(())`, unmodified | Proven by an existing Cargo test that was NOT changed |
| CE-03 | **`check_evaluable` is strictly additive.** The zero clause runs after all of `check`, so every record refused before is still refused for the reason it always was and this catches only what passed all four | `indicators::Candle::check_evaluable`; `extreme_prices_neither_panic_nor_overflow` still receives `RangeOverflows` and not `PriceNotPositive` for an `i64::MIN` low | Proven by an existing Cargo test |

These rows do not prove the eight-rung sweep over the real store. XG-01 is
measured on the reference machine's fourteen cores and its arithmetic is
machine-dependent by design — a narrower machine solves to fewer rungs, which is
the intent and not a regression. VS-02 and XG-02 are static implementations whose
consequences are argued in `docs/05-decisions.md` rather than asserted by tests of
their own.

## Cash-stock research preparation (2026-09-05)

| Invariant | Focused proof | Boundary |
|---|---|---|
| Requested history starts 2020-01-01 and ends on the previous IST calendar date; UTC midnight does not choose the day | `research::tests::yesterday_uses_ist_midnight_not_the_utc_date`; `year_and_leap_month_rollovers_are_exact` | Planner API; legacy range commands unchanged |
| Today and pre-start timestamps are excluded with microsecond precision; clipping preserves order and is idempotent | `microsecond_boundaries_exclude_today_and_pre_2020_data`; `clipping_preserves_order_and_is_idempotent` | Signal/execution input helper, not automatic warm-up handling |
| Requested final date changes identity fields even when a holiday adds no bars | `a_new_day_changes_identity_words_even_without_new_bars` | Fields supplied for future run identity, not a persisted run receipt |
| Empty/partial inventory is never presented as complete or as a sweep | `empty_inventory_is_not_reported_as_ready_or_as_a_search`; `inventory_deduplicates_paths_and_separates_feed_segment_and_month` | File-path census only, not verified record coverage |
| An F&O index underlying is never relabelled as a research cash stock | `cash_inventory_never_relabels_an_index_as_a_stock` | Existing F&O/total-market snapshot intersection, not historical membership |
| SL+TP / SL+TTP omit unrequested exit shapes before pricing and match the corresponding legacy cells | `grid::exit_family_tests` (enumeration, ratio admission, saturation, synthetic long/short equivalence, empty input) | Opt-in runner APIs; CLI selection and causal calibration not integrated |

Focused tests do not establish the section 9 full-workspace, coverage, mutation,
real-data completeness or 24-hour throughput requirements.

## Instrument and ingest repair (2026-09-05, D-0510)

| Invariant | Regression proof | Boundary |
|---|---|---|
| Conflicting duplicate identities never choose a vendor ID | `duplicate_conflicting_assertions_survive_loading_and_refuse_lookup` | Master parsing and merged lookup |
| Crawl ISIN evidence belongs to the selected vendor | `crawl_rows_never_borrow_another_vendors_isin` | No vendor network calls in the test |
| Damaged or missing feed cannot replace a previous usable snapshot | `master_reload_preserves_snapshot_on_a_damaged_or_missing_feed` | Reload transaction, not external file restoration |
| Application refreshes serialize before fetching | `refresh_requests_wait_before_starting_the_next_fetch` | Process-local transaction |
| A resumed incomplete bucket equals one-shot complete input | `resumed_partial_bucket_matches_one_shot_bytes` | Appendable source suffix, not insertion into sealed history |
| Repeated broker candles never double volume | `broker_duplicates_do_not_double_volume_and_conflicts_are_refused` | Archive snapshot aggregation remains distinct |

See `docs/12-ingest-audit.md` for pending full-gate verification and limitations.

## Pull readiness (2026-09-05, D-0511)

| Invariant | Regression proof | Boundary |
|---|---|---|
| A requested published basket never silently shrinks | `mapping_preflight_never_silently_shrinks_the_requested_basket` | Static membership snapshot, not historical membership |
| Unresolved selected identities refuse before network work | `mapping_preflight_refuses_before_any_instrument_is_attempted` | Zero attempted instruments; no vendor access in test |
| A ticker alone is insufficient cash identity evidence | `mapping_preflight_accepts_only_the_selected_verified_cash_identity` | Vendor ID must match exchange ISIN join |
| The current ID cannot borrow an older snapshot's join | `mapping_preflight_rechecks_the_current_snapshot_and_vendor` | ID and listing resolved together; stored prior data unchanged |
| Index IDs do not require a cash-share ISIN | `mapping_preflight_indices_need_ids_but_never_a_stock_isin` | Still requires selected vendor's ID |
| Missing closing buckets on an earlier observed day are reported | `an_absent_closing_bucket_is_named_before_the_next_observed_day` | Eight intraday widths; absent intervening days not inferred |
| A final unknown endpoint is not invented | `a_final_days_absent_tail_has_no_proven_request_endpoint` | Full request-span attestation remains separate |
# Dated derived-session bounds (D-0512)

| Invariant | Proof |
|---|---|
| Regular derivatives minutes follow the dated exclusive close; cash/index are not extended | `pull/tests/anchor.rs::regular_venue_minutes_follow_the_dated_close_exclusively` |
| Venue hours never override exceptional calendar classification | `pull/tests/anchor.rs::venue_hours_do_not_override_an_exceptional_calendar_session` |
| Futures ingestion preserves the complete 375/385-minute volume across all eight stored intraday widths | `pull/tests/derive.rs::future_derived_files_preserve_the_dated_regular_session` |

## Explicit archive product and missing identity evidence (D-0513)

| Invariant | Proof |
|---|---|
| Plain FNO preserves price, volume, OI and zeros without inventing bid/ask | `csv::tests::plain_fno_reads_price_volume_and_open_interest_without_bidask_fields` |
| Five-field and nine-field products reject each other's widths, including mixed files | `csv::tests::plain_fno_and_bidask_require_their_exact_declared_field_counts` |
| A malformed plain FNO row names the failing line | `csv::tests::a_malformed_plain_fno_row_refuses_the_whole_file_at_its_line` |
| Zerodha's cash token can exist while required identity evidence is absent | `server::tests::mapping_preflight_zerodha_cash_names_missing_evidence_not_missing_id` |

## Explicit Zerodha mapping and cash-session gate (D-0514)

| Invariant | Proof |
|---|---|
| Strict default; reject invalid, repeated and wrong-vendor alternatives | `ingest::tests::cash_identity_requires_explicit_valid_vendor_scoped_choice` |
| Native duplicate rows are idempotent; vendors remain independent | `merge::tests::native_ids_accept_duplicates_and_keep_vendors_separate` |
| Conflicting forward/reverse identities refuse regardless of row order | `merge::tests::native_conflicts_in_either_direction_stay_refused_under_permutation` |
| No substitution of a confirmed unsuffixed alias | `merge::tests::native_lookup_never_uses_the_confirmed_unsuffixed_alias` |
| New mode can resolve the real-shaped Zerodha fixture without claiming ISIN | `server::tests::mapping_preflight_zerodha_cash_names_missing_evidence_not_missing_id` |
| Cash session gate includes the effective day and excludes index-only requests | `server::tests::cash_session_preflight_preserves_the_effective_date_and_instrument_scope` |
| Native mapping assurance survives the audit source-length bound | `server::tests::native_policy_assurance_survives_the_bounded_audit_source` |

## One-minute derivation boundary (D-0515)

| Invariant | Proof |
|---|---|
| Coarse source remains stored but cannot publish unverified derivatives | `coarse_source_is_stored_but_cannot_publish_unverified_derivatives` |
| Shifted broker minutes refuse before any write without changing input | `broker_candles_shifted_thirty_seconds_refuse_before_any_write` |
| Broker within-minute duplicates cannot be rounded into volume; archive snapshots remain distinct inputs | `an_extra_broker_timestamp_inside_a_minute_refuses_but_archive_seconds_survive` |
| Requested width and subsecond precision are checked at the broker boundary | `broker_grid_uses_requested_width_and_preserves_millisecond_precision` |
| Daily source timestamps do not inherit the intraday grid | `daily_broker_sources_do_not_require_the_intraday_opening_grid` |

## Requested spot-minute coverage (D-0516)

| Invariant | Proof |
|---|---|
| Missing month-end tails and entire trading days are named without weekend gaps | `request_minutes_report_month_end_absent_days_and_final_tail_without_weekend_gaps` |
| Opening/interior gaps are grouped; empty regular-day requests are not complete | `request_minutes_aggregate_opening_and_interior_gaps_and_report_empty_requests` |
| Closed days are skipped; exceptional/unmeasured sessions remain unverified | `request_minutes_skip_closed_days_but_name_unverified_sessions` |
| Unordered rows and derivative requests are not mis-certified | `request_minutes_do_not_attest_unordered_rows_or_derivative_requests` |
| Cash eligibility is not inferred from a generic venue schedule | `request_minutes_cash_requires_instrument_eligibility_after_session_change` |
| Daily cash requests are exempt from the intraday CAS guard | `server::tests::cash_session_preflight_preserves_the_effective_date_and_instrument_scope` |
| Coverage warnings reach a telemetry file | `emit_sites::every_emit_site_in_this_crate_reaches_a_file` |
| Catalogue coverage is not described as pull readiness or permission to silently shrink a basket | `coverage::tests::the_notes_name_only_the_targets_a_feed_is_short_of` |

## Separate native-token/ISIN evidence and basket completion (D-0517)

| Invariant | Proof |
|---|---|
| Cross-check is explicit and Zerodha-only; invalid and repeated choices refuse | `ingest::tests::cash_identity_requires_explicit_valid_vendor_scoped_choice` |
| Successful cross-check does not fabricate a Zerodha ISIN | `server::tests::cross_checked_identity_retains_both_sources_without_inventing_a_vendor_isin` |
| Missing, wrong, ambiguous or changed independent evidence refuses | `server::tests::cross_checked_identity_refuses_absent_wrong_or_ambiguous_independent_evidence` |
| ISIN evidence cannot substitute for the exact native token | `server::tests::cross_checked_identity_still_requires_the_exact_zerodha_native_key` |
| Policy assurance survives the fixed audit source bound | `server::tests::native_policy_assurance_survives_the_bounded_audit_source` |
| A partial basket retains successful writes but never reports STORED | `server::tests::a_partial_broker_basket_cannot_render_or_record_as_stored` |
| An interrupted basket cannot certify completion | `server::tests::an_interrupted_broker_basket_keeps_writes_but_fails_completion` |
| Browser never infers a relaxed choice from a truthy value or another vendor | `web/tests/cash-identity.test.js` |
| Spot receipts name the selected identity policy and actual timeframe; wire-date captions follow that feed's timeframe-specific range semantics | `server::tests::spot_receipt_names_the_selected_policy_and_rung_not_dhan_wire_dates` |

## Pull coordinator completion (D-0518)

| Invariant | Proof (`pullrun::tests`) |
|---|---|
| Healthy siblings cannot erase a dead feed | `a_dead_feed_remains_incomplete_after_healthy_feeds_finish` |
| A permanent preflight refusal does not loop through identical retries | `permanent_422_stops_after_one_response_without_retrying` |
| Idle/empty is not full coverage | `empty_receipts_end_idle_retries_without_claiming_coverage` |
| Interrupted and skipped legs are not completed attempts | `operator_stop_counts_the_returned_leg_and_leaves_the_rest_unattempted` |
| Retry counters describe actual rescheduling | `retries_count_rescheduled_feeds_and_reset_pass_counters` |
| A terminal cause replaces an earlier transient error | `a_terminal_refusal_replaces_an_earlier_transient_cause_and_stays_halted` |
| A panicking chain is attributed to the correct feed after a sibling halts | `a_panicking_chain_keeps_its_feed_index_after_a_sibling_halts` |

## Dated cash continuous-session boundary (D-0519)

| Invariant | Proof |
|---|---|
| Eligible/ineligible cash minute landing, gap auditing and all seven derived rungs share one dated close; replay adds no duplicate source candles | `dated_cash_close_filters_auction_and_derives_the_same_continuous_session` |
| Missing dated evidence is never interpreted as an ordinary cash session | `request_minutes_cash_requires_instrument_eligibility_after_session_change`, `dated_cash_close_filters_auction_and_derives_the_same_continuous_session` |
| Eligibility alone cannot certify an unmeasured calendar: filtered source persists, derived output is withheld, replay commits no duplicates | `eligibility_does_not_certify_an_unmeasured_calendar_but_source_is_preserved` |
| Only observed in-window CAS dates are requested, once each, under the descriptor's timestamp encoding | `server::tests::observed_eligibility_dates_are_windowed_deduplicated_and_encoding_aware` |
| Dated eligibility verification and resolved counts reach the telemetry sink | `server::tests::dated_cash_session_evidence_is_logged_with_resolved_counts` |
| All 208 current cash identities match every supplied dated NSE master, without alias substitution | `cash_auction::tests::actual_dated_masters_match_every_current_fno_cash_identity` (explicit local-fixture test; 25 dates) |
| A later metadata refusal cannot erase earlier fetched chunk writes; all refused rows remain counted and replay commits no duplicate source candles | `server::tests::missing_later_cash_evidence_preserves_earlier_chunks_and_counts_refused_rows` |

## Optional archive discovery (D-0520)

`render::disabled_archive_suggestions_never_enter_the_filesystem_discovery`
proves the disabled branch does not invoke discovery and the enabled branch
still returns the discovered list. The broker-only process can therefore
avoid optional Downloads access without changing broker/data permissions.

## Zerodha form defaults and unknown denominators — 2026-09-06

Frontend tests in `web/tests/cash-identity.test.js` pin the cross-checked form
default, preserve strict handling for unknown choices and other vendors, and
check that the form wires the default and renders an unknown expected count
as `unverified`. These tests do not certify vendor history or browser runtime
coverage. Backend request identity rules remain unchanged.

`web/tests/row-pull.test.js` verifies that a row retry contains exactly one
member, preserves all other request fields, encodes special symbol characters,
and rejects an empty symbol. This narrows requests, not historical gap repair.

## Explicit revision and runtime-calendar proofs — 2026-09-06

The ten tests in `crates/store/tests/repair.rs` cover original-byte preservation,
existing readers, exact retries, rejected corrections at an existing ordinal,
source-generation mismatch, missing original timestamps, invalid rows, bounded
inputs, source locks, corrupt evidence, incomplete publication, damaged receipts
and competing publishers. See `crates/store/REPAIR.md` for the exact test-to-rule
table and remaining production-promotion and physical-fault limits. These tests
do not establish power-loss guarantees, 100% coverage or a live-store migration.

`calendar::runtime_tests::observations_preserve_every_static_answer_and_both_unknown_bounds`
proves observed dates cannot override static schedules or turn unknown bounds
into certified sessions, including extreme day values.
`calendar::runtime_tests::unknown_observed_lengths_and_empty_calendars_carry_no_authority`
proves missing observation coverage and unmeasured lengths stay unverified.

### Bounded calendar extension through 2026-09-04

`calendar::tests::bounded_extension_has_exactly_ten_open_and_four_closed_dates`,
`extension_preserves_every_original_bit_and_exception`, and
`extension_matches_both_measured_index_minute_grids` pin the finite extension
without changing historical bits. The source is recorded in the charter.
`server::tests::production_ingestion_attests_only_the_bounded_calendar_extension`
checks the production boundary. The cash-session cache test
`observed_unknown_sep11_loads_flag_without_attesting_a_session` checks that
acquiring eligibility metadata still cannot attest an unknown trading date.

### Partial-month cash replay context

`server::tests::cash_partial_month_replay_loads_committed_dates_outside_request`
checks that earlier stored source dates receive their own verified eligibility
when a later suffix is replayed, without adding unobserved dates or changing
original bar bytes. `cash_partial_month_replay_refuses_missing_or_corrupt_earlier_receipt`
checks that warm memory cannot hide damaged or missing saved evidence and that
no bar or derived file is written on that refusal.

`server::tests::several_diagnostics_for_one_instrument_are_not_failed_member_counts`
pins the receipt distinction between members read and failure diagnostics:
multiple coverage reports for one stock cannot be labelled multiple failed
members. It also keeps the request's unclean verdict and named reasons.

Pull derivation tests `incomplete_historical_source_requires_evidence_before_a_store_remedy`,
`unverified_historical_schedule_requires_evidence_and_preserves_bytes`, and
`absent_buckets_and_observed_day_tails_require_source_evidence_first` distinguish
incomplete evidence from confirmed stored-byte conflicts. A source gap is not
presented as something a storage revision alone can fix. Complete historical
candidate gaps still require explicit versioned gapfill; all original bytes
and refusal behavior remain protected by the derive regressions.

Autopilot tests `paused_idle_status_scopes_the_pause_to_the_automatic_scheduler`,
`paused_active_status_keeps_direct_pull_progress_visible`, and
`pull_activity_transitions_preserve_the_scheduler_phase` pin independent pull
activity without resuming or altering the automatic scheduler.

## Stored-minute recovery audit hardening — 2026-09-06

| Invariant | Regression |
|---|---|
| Cash gap audits use the exact date's verified continuous-session close; a previous day's flag is never carried forward | `pull::gaps::tests::cash_auction_flags_are_dated_and_missing_metadata_is_not_a_clean_day` |
| An explicit non-CAS flag retains the 15:15–15:29 obligation, including an entirely absent source day | `pull::gaps::tests::cash_ineligible_tail_and_entire_absent_day_remain_expected_gaps` |
| Old exceptional sessions retain their windows; a dated master cannot extend the calendar, and extreme day values terminate safely | `pull::gaps::tests::cash_audit_preserves_pre_change_sessions_and_never_promotes_unknown_days` |
| Off-grid, duplicate and backward timestamps cannot certify minute coverage or create spurious later holes | `pull::gaps::tests::stored_minute_audit_rejects_off_grid_duplicate_and_backward_timestamps` |
| The gap endpoint validates local master receipts and exact identity, reports missing/corrupt evidence, never fetches it on GET, and preserves original bars | `api::server::tests::cash_gap_page_uses_receipted_local_flags_and_refuses_missing_or_corrupt_evidence` |
| A daily/coarse candle is not minute-source evidence | `api::server::tests::minute_gap_route_refuses_daily_or_coarse_rows_as_minute_evidence` |
| A missing file and an empty file retain the same independently known calendar obligation | `api::server::tests::absent_minute_file_retains_calendar_obligation_like_an_empty_file` |
| Committed off-grid records reach an explicit invalid-input field, not a green completeness result | `api::server::tests::committed_off_grid_minutes_do_not_certify_the_gap_page` |

These are source-grid checks, not historical listing-membership proofs or
evidence that a provider can supply every expected minute. The complete test,
coverage and mutation status must be reported separately.

### Retry-cycle receipt checkpoints

The `api::pullrun::tests` regressions below drive the production coordinator
through synthetic responses, without a vendor connection or credentials.

| Invariant | Regression |
|---|---|
| Only owed legs repeat while clean receipts remain checkpointed | `retry_passes_only_request_owed_legs_including_successes_after_a_failure` |
| Failed prerequisite rungs defer dependent minutes/seconds and FNO legs | `failed_daily_and_minute_rungs_defer_dependents_until_their_retry_succeeds` |
| Healthy feeds do not refresh while another feed is repairing | `healthy_feeds_wait_until_all_retrying_feeds_recover_before_a_fresh_pass` |
| Credential and permanent refusals stay halted after earlier progress | `terminal_refusals_after_checkpointed_progress_never_resurrect_a_feed` |
| Operator stops preserve completed receipts and count only attempts actually entered | `a_stop_during_retry_counts_only_the_returned_request_and_preserves_checkpoints`, `stop_before_a_queued_retry_starts_does_not_count_it_as_attempted` |
| An empty label cannot hide a request that panicked after entry | `a_panicking_request_with_an_empty_label_is_attempted_not_skipped` |
| A partial repair pass is not a fresh full no-growth observation | `a_growing_failed_pass_recovers_only_owed_legs_then_requires_full_idle_passes` |
| Empty responses checkpoint the request, not historical coverage | `empty_receipts_are_checkpointed_during_repairs_and_reasked_on_full_passes` |
| A non-clean partial receipt remains unresolved, not success | `identical_partial_receipts_keep_the_full_failed_leg_owed_without_growth` |

These checkpoints are per live retry cycle, not durable restart checkpoints
or per-instrument/chunk retry planning. The last regression records a limit:
an unchanged partial leg still retries under the existing pass ceiling.

### Gap-page evidence verdicts

`web/tests/gap-verdict.test.js` proves that absent/unreadable files, invalid
timestamps, evidence errors, truncated reads and unmeasured calendar dates
cannot render as complete. Unknown required totals are not zero. A zero
denominator does not establish whole-span coverage. The route accepts only
one-minute sources, and changing its selected feed/source/span clears stale
results. These are UI regressions, not a certificate of market completeness.

### Focused sweep-readiness audit — 2026-09-06 (D-0521)

| Invariant | Executable proof |
|---|---|
| A regular-column bar must be warm before and after its fold; an unusable rollover daily ladder cannot admit a now-cold mask | `indicators/tests/audit_readiness.rs::a_session_rollover_cannot_admit_a_mask_after_its_daily_anchor_becomes_unusable` |
| Finite complete ladders equal independently enumerated subsets across all six words, including threshold edges and shuffled/repeated/invalid offers | `engine/tests/sweep_readiness_oracle.rs::complete_ladders_equal_independent_exhaustive_subsets_across_all_six_words` (378 fixture cells; this is not universal input coverage) |
| A writer opened before another writer's interrupted tail refuses every tested partial stride and changes no existing byte | `cli::results::tests::an_open_writer_refuses_a_later_torn_tail_without_changing_any_byte` |
| Unsealed catch-up rows do not supply deduplication identities; later sealed rows do; damaged strides remain readable as explicit damage | `cli::results::tests::an_open_writer_does_not_index_corrupted_catchup_identities` |
| History shrinkage cannot reuse an open writer's stale index to append | `cli::results::tests::an_open_writer_refuses_a_shrunken_ledger_without_reusing_its_stale_index` |
| Both combination displays assemble the whole bounded frontier, reconcile totals, and refuse incomplete or inconsistent pages | `web/tests/frontier-pages.test.js` (257/4,096 rows, malformed/cyclic cursors, changing identity/rules/counts/ranks, HTTP/JSON failures and explicit refusals) |

The fixes do not assert whole-crate coverage, every-allocation recovery,
collision-free hashes, permanent telemetry retention, universal expression
support or constant total sweep work. Their remaining verification obligations
and the measured limitations belong to the dated audit report.

### Durable evidence-scoped pull recovery — 2026-09-06

| Invariant | Executable proof |
|---|---|
| Exact feed, membership, exclusions, rungs and inclusive month bounds; normalized duplicate submissions | `api::recovery::tests::exact_scope_order_dedup_and_partial_month_bounds`, `bad_feed_empty_basket_excluded_index_and_wrong_envelope_refuse` |
| Interrupted requests consume the same three-attempt budget after reopen | `budgets_survive_restart_and_interrupted_attempts_are_charged` |
| A changed basket cannot refund an exhausted exact-day reservation | `overlapping_plans_cannot_purchase_more_attempts_for_the_same_gap` |
| Clean HTTP alone and uncertain source coverage cannot verify a unit | `failed_receipts_and_unknown_coverage_never_become_verified` |
| A repaired gap stops retries without hiding lifecycle uncertainty; a newly owed verified child keeps its remaining budget | `repaired_gaps_stop_without_laundering_lifecycle_uncertainty` |
| Actual candles on closed/outside-session minutes remain unresolved | `closed_or_outside_session_source_minutes_never_verify` |
| Exact-day recovery requires only its own dated CAS evidence, never a different day's file | `api::server::tests::recovery_cash_clock_requires_only_its_exact_inclusive_dates` |
| Both recovery boundary events actually reach the installed log sink | `recovery_boundary_events_are_read_back_from_the_installed_sink` and `api::emitted` accounting |
| Missing stock history is neither a retry warrant nor an automatic listing exemption | `absent_stock_history_is_unknown_not_a_retry_or_exemption` |
| Exact sourced withdrawal boundaries preserve conflicting observations | `sourced_withdrawal_is_exact_and_conflicting_source_is_preserved`, `pull::cash_auction::tests::independent_forcemot_inactivity_has_exact_identity_and_inclusive_boundaries` |
| Closed/unknown dates, duplicate timestamps and unsafe source access are explicit | `session_boundaries_closed_unknown_and_duplicates_are_not_silent`, `source_reader_uses_the_real_store_path_and_never_creates_missing_files` |
| Repeated seeding preserves state; missing roots, invalid requests and duplicate ownership do not launch work | `seeding_reuses_exact_budgets_and_syncs_activation_without_vendor_access`, `invalid_start_missing_pointer_and_duplicate_claim_do_not_start_work` |
| Start acknowledgement follows durable plan activation; a read-only fixture finishes without vendor or source writes | `start_acknowledges_a_saved_plan_and_read_only_fixture_finishes` |
| STOP during seeding survives both seeding and process restart | `stop_during_seed_cannot_be_cleared_by_seed_or_boot_resume` |
| STOP/explicit clear are append-only and scoped to one plan; uncertainty never clears intent | `api::recovery_control::tests::stop_and_explicit_clear_survive_new_site_and_preserve_history`, `corrupt_or_empty_stop_history_refuses_and_is_never_truncated`, `valid_crc_does_not_let_foreign_identity_or_status_clear_a_stop` |
| STOP success requires durable state; failed persistence returns 503 with memory stop retained, while non-recovery stop stays filesystem-free | `handler_acknowledges_only_durable_recovery_stop`, `handler_io_refusal_returns_503_and_retains_memory_stop`, `normal_pull_stop_and_idle_handler_remain_filesystem_free` |
| Escaped page content and empty/error states cannot imply complete coverage | `status_page_escapes_payload_and_never_calls_an_empty_state_complete` |
| Fixed encoding, reserved bytes, CRC, immutable bodies, writer locks, replay and direct tail bounds | `api::recovery_journal::tests::every_reserved_and_padding_byte_is_checked_even_with_a_valid_crc` |
| Partial/zero-byte writes, sync uncertainty and foreign extent changes never publish a successful append | `partial_and_zero_byte_write_failures_do_not_publish_and_poison_the_handle`, `sync_failure_preserves_old_index_and_reopen_recovers_the_uncertain_record`, `metadata_failure_or_foreign_length_change_poison_without_a_write` |
| Lifecycle evidence binds exact symbol/ISIN, date, URL, size and digest, without claiming full history | `pull::cash_session_cache::tests::local_lifecycle_binds_exact_identity_date_source_length_and_digest`, `actual_receipted_lifecycle_snapshot_never_claims_complete_history` |

These checks do not establish 100% branch coverage, mutation closure, historical
point-in-time identity, provider completeness or constant total run cost.

### Historical sweep readiness follow-through (D-0523)

These are executable invariants. Their presence in this table does not assert
that a later source snapshot passed every required gate.

| Invariant | Executable proof |
|---|---|
| Every live condition has an emitter; all 62 candle patterns have named positive and dark-counterpart fixtures | `indicators::evaluator::tests::the_position_set_is_the_union_of_the_modules`; `pattern::exemplars` inventory and translation/reset tests; `docs/15-indicator-readiness.md` |
| Cold rollover cannot admit a newly unready truth row | `indicators/tests/audit_readiness.rs::a_session_rollover_cannot_admit_a_mask_after_its_daily_anchor_becomes_unusable` |
| Nonpositive valid-shaped candles refuse transactionally; missing indicator evidence remains unknown | `indicators/tests/sweep_predicate_readiness.rs` sign, availability, VWAP, projection and anchored-column regressions |
| Fixed six-word AND search agrees with independent finite enumeration; scheduling requests cannot change answers | `engine/tests/sweep_readiness_oracle.rs`; maximum-lane scheduling and explicit-memory metadata tests |
| AND/OR/NOT preserves unknown, every live position is accepted, nonlive positions and explicit resource breaches refuse | `vocab/tests/expression.rs` truth-table, full signed vocabulary and capacity tests |
| Canonical saved programs validate opcodes, operands, padding, stack shape and structural sibling order | `persisted_programs_reject_bad_versions_stack_shapes_operands_and_padding`; `every_short_encoded_program_agrees_with_independent_infix_reconstruction` |
| Complete expression bytes and every variable run term bind the identity | `runner::expression::tests::expression_identity_binds_the_program_and_every_runtime_term` |
| Bad source alignment/order and failed row delivery cannot return a completed expression summary | Runner expression streaming tests |
| Stored expression integrity, exact source rows, unknowns, empty history, no-overwrite and same-read mutation detection remain explicit | `cli::expression::tests::durable_rows_round_trip_exact_sources_unknowns_and_empty_history`, including every-byte/every-truncation, interleaved write and generated-column integration tests |
| Lost/replaced pending expression paths and changed acknowledged bytes cannot be successfully published | `cli::expression::tests::replaced_or_mutated_pending_evidence_cannot_be_acknowledged_as_published` |
| Cold opens and concurrent appends preserve row boundaries and file-generation evidence | `cli::results::tests::shared_writer_refreshes_external_appends_before_duplicate_rejection` and `cli::result_set::tests::a_same_length_mutation_invalidates_the_stale_generation_before_append`; eight-writer stress and non-retained-device header diagnostic |
| Start, exact token/identity, child counts, row seals, completion and requested validation cannot be reconstructed from absent data | `crates/cli/tests/sweep_evidence.rs` lifecycle, concurrent attempt, corruption, deletion, truncation and foreign-file matrices |
| Acknowledged evidence lost/replaced before terminal publication cannot become a completed empty attempt | Public sweep-evidence before-terminal loss/replacement regression matrices |
| A group of starts hands back only attempts whose whole start is durable, and a crash at any grouped barrier -- the pending write present, missing or torn, un-flushed directory entries lost -- leaves no attempt Completed without its rows and no token reusable (D-0607) | `every_grouped_barrier_crash_leaves_only_harmless_states`; `a_refusal_at_any_grouped_barrier_is_loud_and_keeps_only_a_durable_prefix`; `a_superseded_start_inside_a_group_hands_back_only_the_durable_prefix` in `crates/cli/src/sweep_evidence_tests.rs` |
| The ancestor-chain memo holds only chains this process flushed itself, and any evidence I/O refusal forgets it (D-0607) | `the_directory_memo_skips_a_flushed_chain_and_forgets_it_after_an_io_refusal`; `a_refused_chain_barrier_refuses_the_start_and_is_not_remembered` |
| Grouped and single starts leave the same evidence; a group finish refuses in sequential order and never shares a journal barrier across roots (D-0607) | `eight_single_starts_and_one_group_of_eight_leave_the_same_evidence`; `a_refused_attempt_stops_a_group_finish_in_sequential_order`; `attempts_from_two_roots_cannot_share_one_journal_barrier` |
| Grouping cannot change a catalog: groups of 16, 1 and 4 reproduce the ungrouped identity, completion digest and body bytes, and a refused start inside a group returns the refusal sequential starts meet first (D-0607) | `grouped_catalogs_keep_the_ungrouped_bytes_and_terminals`; `a_refused_start_inside_a_group_keeps_the_sequential_first_refusal`; `refused_later_starts_and_finishes_leave_no_completion` in `crates/cli/src/index_stop_tests.rs` |
| Empty, cold, refused or unreconciled search samples cannot be completed; only a recorded resource breach is halted | `cli::sweep_wiring_tests` completion classification matrices; shared `runner::complete` requires nonzero swept bars |
| A durable admission refusal precedes evaluator preparation; child/receipt failure never publishes a parent result | `cli::sweep_wiring_tests` reservation, evaluator refusal, identity and actual unadmitted-publication fault tests |
| Actual separately loaded execution bytes affect stored identity | `cli::sweep_wiring_tests` interior execution digest sensitivity |
| Every automatic probe is announced before its walk and reports levels/terminal; a callback error prevents subsequent successful probes | `runner/tests/auto_reporting.rs` |
| Every assembled frontier/depth page belongs to one exact identity/attempt with reconciling counts | `web/tests/frontier-pages.test.js`; `web/tests/sweep-evidence.test.js`; `api::sweepevidence::tests::durable_depths_are_paged_with_exact_attempt_and_validation_state` |
| A failed refreshed handle is surfaced before a later reopen; committed top rendering shares the CLI rule | `api::detail::tests::verified_cache_exposes_refresh_refusal_before_any_later_reopen`; `api::topjson::tests::cached_http_report_uses_the_exact_cli_renderer_after_parent_commit` |
| Attempt events and inspection commands cannot masquerade as whole sweep completion; active CLI observation outranks a completed browser slot | `api::sweeprun::tests::attempt_lifecycle_cannot_hide_or_complete_a_command_and_the_marker_window_is_bounded` command/attempt interleaving, clipped/missing fields and overlapping lifecycle fixtures |
| Only five represented historical rules are described by the visible five-rule verdict | `web/tests/sweep-evidence.test.js` human comparison and frontend contract tests |
| Human table stays synchronized with its reviewed source; automated checks retain separate actual logs | `web/sweep-readiness/build-review.mjs --check`; Rust `web/sweep-readiness/verify.rs` |

Complete touched-crate branch/line coverage, no surviving mutation,
real-data/deployment verification and full institutional admission remain
separate required evidence. No row above grants universal O(1) latency or space.

### Resumable historical search and candidate authority (D-0524)

| Invariant | Executable proof |
|---|---|
| AND continuation preserves uninterrupted survivors, counters and extinction across the same fixed policy | `engine/tests/resume_readiness.rs`; independent exhaustive configurations and each reached restart boundary |
| Stored AND recovery validates the column/policy before restoring audit rows, persists each completed level and refuses final replacement | `cli::and_checkpoint::tests::persisted_interrupt_restart_and_terminal_replay_preserve_every_depth_row` persisted restart, sink-failure, ranked-helper and final-corruption tests |
| Marked corrupt latest checkpoints never fall back, unmarked reservations are never reused, failed writers cannot acknowledge later work | `cli::search_checkpoint::tests::latest_corruption_never_falls_back_to_an_older_valid_checkpoint` restart, corruption, admission and poisoned-writer tests |
| An owner symlink cannot create an unrelated missing target | `dangling_owner_symlink_refuses_without_creating_a_foreign_file` |
| Every saved expression step is a reachable grammar successor; structural validity alone cannot claim exhaustion | `structurally_valid_sealed_early_exhaustion_and_wrong_successor_are_refused` |
| Canonical expression order and work survive bounded pause/reopen; bad widths, alphabets and maximum-capacity programs obey the same grammar | `vocab/tests/expression_search.rs` and `vocab/tests/expression_search_readiness.rs` |
| Maximum encoded expressions render without recursive stack growth; formatter errors propagate | `canonical_display_round_trips_normal_programs_and_renders_maximum_wire_without_recursion` |
| Search identity binds initial language/alphabet and nine run terms, independently of later cursor progress | `runner::expression::tests::search_identity_binds_language_alphabet_and_market_inputs` search identity tests |
| Expression conjunction pricing exactly matches all legacy grid fields and each materialized cell in both directions | `runner/tests/expression_pricing.rs::conjunctions_match_all_legacy_trade_and_grid_fields_for_both_sides` |
| OR/NOT pricing uses definite truth, shares exact execution, and rejects a modified selected cell | Remaining `runner/tests/expression_pricing.rs` tests |
| Priced expression resume requires both exact trade children in addition to signal evidence | `priced_search_restarts_with_both_sides_and_refuses_missing_trade_children` |
| Captured nonwinning candidates keep their own selected-cell trades, actual caps, tiers and no-cell outcomes | `cli::candidate_trades::tests::real_screen_keeps_nonwinning_candidate_traces_and_actual_cap_across_tiers` real-screen, exact two-side and multi-tier tests |
| Capture failure or lost acknowledged children prevent sealing/parent publication, including after validation | `actual_audit_refuses_a_failed_capture_before_publishing_its_parent`, callback-failure and final-confirmation tests |
| Expression capture cannot substitute an AND catalog with identical referenced bits | `expression_capture_replays_or_not_without_relabelling_the_same_referenced_and_bits`; API/browser model-pinning tests |
| Malformed price grids/overrides refuse before pricing, every variable policy term changes identity, and dropped execution signals remain visible | `cli::expression_search::tests::pricing_refuses_each_malformed_override_and_identity_binds_every_policy_term` pricing admission, identity and projection-report tests |
| Selection V6 requires genuine Execution V4 authority, preserves terminal cardinalities and authenticates every fixed byte | `cli::selection_v6::tests`, including actual Population/Execution handoff, every-byte mutation and exact append/reopen |
| V4 chronological replay consumes exact retained OOS witnesses and preserves inclusive occupancy and explicit unpriced decisions | `cli::global_replay_v4` regression tests, including the genuine stored-witness fixture |

These are finite executable invariants. The per-module measured coverage and
mutation results are separate; none of these rows asserts complete market-input
coverage or approved institutional configuration.

### Strict input admission and safe stored publication (D-0525)

| Invariant | Executable proof |
|---|---|
| Cold audit authenticates exact header, data and CRC sidecar, including partial format blocks and physical extent | `store::checksum_audit` tests: record/CRC byte faults, block boundaries, replacement and missing/extra/truncated input matrices |
| Receipts are append-only, source-bound and cannot grant rows after any required byte or path changes | `exact_receipts_reopen_with_same_identity_and_keep_every_real_row`, `every_torn_prefix_recovers_only_by_appending_the_exact_suffix`, `warm_receipt_checks_refuse_every_fault_and_never_release_a_row` |
| Visible complete receipt bytes are not authority while the publisher owns the exclusive lock; typed readers retain shared ownership | `visible_complete_bytes_are_not_durable_authority_while_a_publisher_holds_the_lock` |
| Missing receipts, prior context, physical resource ceilings and FIFO paths refuse without replacement data or waiting for a pipe peer | `strict_input_caps_and_missing_prior_context_refuse_without_fallback`, `receipt_fifo_cannot_block_either_read_or_publication`, store bounded subprocess path tests |
| Native and coarse strict inputs use the same calendar conversion and exact real-row handles, with all six roles required through publication | `native_and_coarse_use_exact_audited_rows_and_the_same_calendar_converters`, `every_context_source_and_the_saved_role_binding_remain_required_after_loading` |
| Lost or corrupt acknowledged depth/ranked children and a failed final source guard never reach the real parent result writer | `stored_month_publication_tests::lost_or_corrupt_acknowledged_children_never_reach_the_real_parent_writer`, `strict_guard_refusal_precedes_terminal_and_parent_publication` |
| Successful computation evidence is sealed before parent publication; a failed parent append returns its actual failure | `genuine_terminal_and_all_acknowledged_rows_exist_before_parent_publication`, `parent_write_refusal_keeps_completed_computation_and_returns_the_exact_failure` |
| Every priced-audit publication branch seals acknowledged depth/rank evidence first; late loss cannot append a parent, and final capture confirmation remains required | `audit_publication_tests::actual_audit_late_depth_or_ranking_loss_cannot_publish_a_parent`, `actual_audit_publishes_only_after_the_real_empty_ranking_is_sealed`, `audit_finalizer_rechecks_exact_priced_catalog_and_children_before_terminal` |
| Test-only one-shot publication fault injection cannot contaminate a later audit | `audit_publication_tests::test_fault_guard_cannot_leak_an_unconsumed_callback` |
| Permanent result-publication refusal cannot return a successful stored-command exit; prose or prefix lookalikes do not become failures | `tests::a_failed_stored_parent_append_cannot_return_command_success`, `a_refusal_is_recognised_in_every_spelling_a_renderer_emits` |
| An internal missing execution minute is described as a missing exact same-day bar, without incorrectly calling it a session end | `a_coarse_execution_report_names_an_internal_missing_minute` |
| Checksum completion cannot masquerade as sweep or financial completion | `web/tests/sweep-evidence.test.js` checksum-audit comparison regression |
| Missing institutional policy exposes all 37 required choices before loading market spans for sizing, for both final selection and later replay | `ledger_v6::tests::missing_policy_refuses_before_ledger_v6_market_sizing`, `missing_policy_refuses_before_ledger_v6_replay_market_sizing` |
| Checksum test fixtures cannot contaminate the process-wide production telemetry census | All seven `store::checksum_audit::tests::full_audit_matches_exact_file_images_at_all_partial_block_boundaries` retain the existing `emits::hold_the_sink` guard; `emits::every_emit_in_this_crate_reaches_the_log_through_its_production_call` still requires exactly seven real production emit sites |

This list identifies executable tests, not an assertion that every verification
gate or every possible source/OS failure has passed. The dated verification
report records actual runs separately.

### VWAP and strict multi-month completion (D-0526)

| Invariant | Executable proof |
|---|---|
| All 20 VWAP predicates have their own reachable true/false outcomes and correct tolerance-dependent availability | `vwap::tests::all_twenty_known_positions_follow_their_actual_reference_and_tolerance` |
| Every actual evaluator VWAP bit and its negation agrees with independent integer levels; first contribution and new session remain Unknown | `sweep_predicate_readiness::all_vwap_predicates_and_negations_match_independent_integer_levels` |
| Zero dispersion certifies 13 exact predicates but never treats the seven undefined near predicates as known false | `vwap::tests::zero_dispersion_knows_exact_comparisons_but_cannot_certify_near_false` |
| Unrepresentable offset/upper/lower levels cannot produce truth or known-false band evidence | `vwap::tests::unrepresentable_band_levels_cannot_be_truth_or_known_false` |
| Moving decoded range bars does not release source/receipt authority; every required month and predecessor remains necessary | `audited_range_tests::each_unique_source_is_required_even_after_decoded_bars_are_moved`, `every_required_month_missing_or_corrupt_refuses_without_a_shorter_span`, `linked_role_nodes_are_exact_reusable_and_each_predecessor_stays_required` |
| Strict source failure is durable before any audit computation, and actual terminal rechecking precedes parent publication | `audit_publication_tests::strict_source_refusal_is_saved_before_any_audit_computation`, `strict_source_must_remain_current_until_the_real_audit_terminal` |
| Legacy institutional output directories and sizing reads cannot precede complete policy resolution | `ledger_all::tests::missing_policy_refuses_before_legacy_ledger_market_sizing_or_output_creation` |
| The actual strict multi-month kernel publishes and reuses valid native/coarse evidence, but terminal source/receipt/rank loss prevents a parent | `audited_stored::range::tests::actual_strict_range_kernel_publishes_and_reuses_native_and_coarse_evidence`, `actual_strict_range_terminal_refuses_changed_source_binding_or_acknowledged_rank` |

New finite tests are evidence of these invariants, not a declaration that the
separate line/branch coverage, mutation, source-wide or deployment gates passed.

### Available false predicates and explicit launch evidence (D-0527)

| Invariant | Executable proof |
|---|---|
| All 20 opening-range true/false/NOT predicates use their frozen window, including the exact first closing bar and session reset | `orb_known_readiness::all_twenty_relations_and_not_match_independent_frozen_range_comparisons`, `first_closing_bar_holes_preopen_and_missing_windows_follow_actual_session_evidence` |
| Opening-range exact comparisons and near availability distinguish zero span, invalid widths and positive integer extremes; refused bars preserve state | Remaining `orb_known_readiness` tests and anchored Column handoff |
| All 27 previous-day/five-session Fibonacci true/false/NOT predicates agree with independent integer levels and tolerance boundaries | `fib_known_readiness::all_twenty_seven_predicates_and_not_match_integer_oracle_at_both_band_edges` |
| Previous-session availability requires completed usable references; cold, unusable, nonregular and overflowing references cannot silently become false | Remaining `fib_known_readiness` tests, including `public_column_retains_known_false_fibonacci_answers_for_negated_search` |
| Current-day false answers use the emitted pre-fold leg through new extremes, leg erasure/flip, equal touches and session reset | `current_day_fib_known_readiness::availability_uses_the_old_leg_through_new_extremes_touches_and_session_reset` and independent every-rung/band/overflow tests |
| All eleven gap predicates and their negations use pre-fold levels; absent/engulfing/overflowing gaps and nonregular anchors keep their explicit semantics | `gap_known_readiness::all_eleven_gap_predicates_and_negations_match_independent_pre_fold_levels`, `absent_ambiguous_and_unrepresentable_gap_references_never_satisfy_not`, `non_regular_session_cannot_replace_the_prior_regular_gap_anchor` |
| Exact-minute gap replacement copies known false, clears stale local availability, preserves other families and refuses without partial mutation | `gap_known_readiness::exact_minute_overlay_preserves_known_false_and_is_transactional_on_missing_evidence`, `column::tests::exact_gap_replacement_clears_every_local_gap_bit_and_no_other_family` |
| Future exact-minute evidence cannot change already mapped truth or availability | `anchored::tests::future_exact_minutes_cannot_change_an_already_mapped_signal_column` |
| Coarse and straddling candles cannot define short opening ranges; stored ORB and GapFib truth/known pairs come from exact one-minute closes | `exact_minute_orb_gap_readiness` independent opening-range oracle across declared intraday rungs |
| A new session without an opening window clears stale truth and availability, including absent gap references | `exact_minute_orb_gap_readiness::new_session_without_opening_window_clears_stale_truth_and_known_including_absent_gap` |
| The combined minute bridge rejects missing, mismatched, corrupt or unaligned evidence before changing any row | `exact_minute_orb_gap_readiness::combined_bridge_rejects_late_missing_mismatched_corrupt_and_unaligned_minutes_transactionally` |
| A summary, missing read, failed read or nonpricing operation cannot certify trade capture; the rendered candidate action and wording share eligibility | `web/tests/sweep-evidence.test.js` missing/failed, noncandidate-mode and audit/expression comparison tests |
| A priced no-cell outcome preserves recorded signals/refusals without fabricated totals; exit indices map to their own saved ladders | `web/tests/candidate-trades.test.js` no-cell, selected-exit and missing/empty/zero distinction tests |
| Launch and detailed comparison tables agree with the dated reviewed report, retaining only supported local evidence links | `web/sweep-readiness/build-review.mjs --check` |
| The strict API preserves exact feed/rung/span/attempt and accepts only server-owned checksum paths/limits | `strict_tests::strict_command_parser_and_typed_adapter_preserve_exact_request_and_attempt`, `request_paths_and_limits_cannot_replace_server_strict_configuration` |
| Missing/invalid strict configuration and out-of-domain request/environment settings refuse before a slot or attempt, with explicit field names | `strict_tests::missing_strict_configuration_is_structured_503_before_attempt_or_slot`, `strict_out_of_domain_request_settings_refuse_before_configuration_slot_or_start`, `strict_invalid_server_environment_refuses_before_configuration_slot_or_start` |
| Strict CLI invalid runtime settings cannot open real source admission or begin preparation | `audited_range_tests::strict_invalid_runtime_settings_refuse_before_real_source_admission_or_preparation` |
| Native/coarse strict API adapter retries reuse genuine parent evidence; late rank loss returns error and cannot publish a parent | `audited_range_tests::strict_api_adapter_reuses_real_parent_and_returns_late_rank_loss_as_error` |
| A source change after folding explicitly seals preparation Refused before returning the actual error | `audited_range_tests::strict_preparation_source_change_seals_refusal_before_returning_error` |
| Strict settings share actual runtime scalar bounds and reject fallback/clamping before source admission | `strict_range_knobs::tests::strict_scalar_bounds_match_shared_readers_at_exact_limits`, `strict_environment_validation_refuses_without_defaulting_or_exposing_values` |
| An unwritten strict task-completion event becomes a visible refusal while preserving the original outcome and already-written result evidence | `strict_tests::strict_terminal_audit_requires_written_and_preserves_original_evidence_text` |
| Strict and ordinary commands share the same pre-spawn cleanup guard, durable attempt boundary and terminal finish; strict terminal audit settles before normal disarm | `strict_tests::strict_and_ordinary_commands_share_one_guarded_task_and_terminal_finish`, `sweeprun::tests::all_three_browser_engine_tasks_arm_and_disarm_the_same_finisher` |

Every proof above is finite and source-scoped. Saved test results are not an
attestation of subsequent source edits, runtime configuration or deployment.

### Time-window false availability (D-0528)

| Invariant | Executable proof |
|---|---|
| All four existing half-open clock predicates are known on every accepted timestamp; NOT preserves both true and false answers | `time_known_readiness::all_1440_minutes_on_two_days_preserve_half_open_truth_and_make_every_false_clock_known` |
| No positive clock window is invented outside the existing intervals; rollover requires no price history | `time_known_readiness::outside_the_declared_windows_no_positive_clock_is_substituted_and_rollover_needs_no_history` |
| Clock negation composes correctly with AND/OR while missing price references remain Unknown | `time_known_readiness::clock_negation_composes_with_and_or_while_an_unseeded_price_predicate_stays_unknown` |
| A warmed search column retains all315 post-early-morning bullish signals in the declared375-minute fixture | `time_known_readiness::actual_search_column_keeps_the_315_not_early_morning_signals` |
| A corrupt or repeated candle cannot publish clock availability or advance the state used by the next accepted candle | `time_known_readiness::a_refused_boundary_candle_cannot_advance_or_publish_clock_state` |

### Session, trend and crossing false availability (D-0529)

| Invariant | Executable proof |
|---|---|
| All25 session answers match independent integer descriptions; the19 new known positions belong only to that module | `session_known_readiness` independent all-position oracle and `session` private exact-owner test |
| Prior-three history excludes the current bar and resets within an already warm Column; unavailable prior history cannot satisfy NOT | `prior_three_excludes_current_and_resets_even_when_the_search_column_is_warm`, `newly_available_false_descriptions_compose_without_promoting_absent_history` |
| Missing/flat gaps, odd rounding, near boundaries, zero ranges, integer extremes and nonregular references retain their declared semantics | `session_known_readiness` gap, extremes and calendar tests |
| A refused bar cannot publish session availability or advance its references | `session_known_readiness::refusals_do_not_publish_or_warm_any_session_reference` |
| Swing availability waits for both observed right-hand confirmation bars; each near band needs its own valid tolerance | `trend_known_readiness::confirmed_swings_become_known_only_after_both_right_hand_bars_have_folded`, `each_swing_band_checks_its_exact_tolerance_and_inclusive_edges` |
| First BoS, continuation, both reversal directions, equality and non-events agree with an independent structure oracle; a missing opposite swing remains Unknown | `first_break_continuation_reversal_quiet_and_equality_follow_the_independent_oracle`, `a_missing_opposite_swing_stays_unknown_and_cannot_satisfy_negation` |
| Refused/extreme candles conserve trend references; earlier average/stop contribution gates and existing cross-day history remain unchanged | Remaining `trend_known_readiness` public tests |
| Every crossing/ordinal truth, known answer and negation agrees with an independent last-definite-side oracle | `crossing_known_readiness::every_crossing_and_ordinal_matches_a_last_definite_side_oracle_including_known_non_events` |
| A usable touch preserves memory/count, a new session clears it, and a refused candle cannot change the next valid answer | `a_touch_is_known_false_but_cannot_erase_the_side_or_count_across_it`, `available_crossing_negation_survives_column_handoff_and_refusal_is_transactional` |
| Both current base references and an earlier same-session definite side are required to certify all five crossing/ordinal non-events; other bits and emitted truth stay intact | `evaluator::tests::crossing_known_requires_both_current_sides_and_a_prior_side_in_the_same_session` |
| A truth-only fold commits the same state and refusal behavior; switching later to Boolean availability cannot lose any reference | `evaluator::tests::truth_only_fold_can_handoff_to_known_without_state_or_refusal_drift` and existing public truth/known equivalence oracles |

### Strict V6, fixed intraday clock and browser correlation (D-0530–D-0532)

| Invariant | Executable proof |
|---|---|
| Every bit of both full checksum bindings changes computation identity; old no-receipt identities remain distinct and unchanged | `identity::tests::checksum_receipt_identity_binds_every_bit_and_keeps_unknown_distinct` and legacy identity regressions |
| Independent signal/minute/daily limits refuse before Candidate computation; identical strict inputs reuse, while changed receipt policy rekeys | `strict_v6_independent_role_caps_refuse_before_candidate_computation`, `strict_v6_same_real_source_reuses_and_receipt_policy_rekeys_ordinary_identity` |
| Required source/receipt replacement and mutation during the actual ladder cannot publish a successful Candidate | `strict_v6_every_consumed_source_and_receipt_replacement_invalidates_retained_authority`, `strict_v6_source_change_during_actual_ladder_refuses_before_candidate_publication` |
| Both evaluated, both mixed and genuinely all-empty stored families reach their correct Selection authority without a fabricated zero-row V1 | `strict_v6_both_evaluated_and_both_mixed_shapes_reach_real_selection`, `strict_v6_extinct_selection_keeps_both_family_sources_through_final_reauthentication` |
| Empty and evaluated family capabilities freshly reopen all retained supporting ledgers; later OOS sources survive witness projection | `strict_v6_retained_empty_and_evaluated_families_reopen_every_saved_support_record`, `strict_v6_oos_witness_keeps_actual_later_source_guard_after_cohort_is_dropped` |
| One-microsecond/half-minute offsets cannot certify15:09 or move the15:10 boundary; missing required data does not erase earlier valid ordinary horizons | `fixed_deadline_readiness::microsecond_and_half_minute_offsets_never_certify_a_1509_record_or_a_fill`, `a_lone_off_grid_required_record_is_missing_while_earlier_exact_rows_stay_usable`, `a_shifted_last_row_cannot_move_the_fixed_deadline_or_replace_the_exact_boundary` |
| Negative civil days and timestamp extremes use checked exact deadlines; native/projected mask/expression paths reject off-grid execution and interiors | Remaining `fixed_deadline_readiness` public tests and canonical trade-boundary regressions |
| A supplied compatibility exit table cannot override canonical session facts, claim a later close or substitute a foreign session | `fixed_deadline_readiness::public_compatibility_walk_refuses_a_later_boundary_from_a_foreign_slice` and `trade` exact-table equivalence regression |
| Command acceptance and terminal status preserve adjacent u64 attempt tokens beyond browser integer precision | `strict_tests::command_acceptance_and_status_preserve_exact_attempts_beyond_browser_integer_precision` |
| The browser queues every selected admitted pair, preserves exact requests, stops on uncertainty/refusal and never retries an ambiguous POST | `web/tests/receipt-batch.test.js`12 mocked transport/state-machine regressions |

These are named finite proofs. Complete touched-crate coverage, all mutations,
production activation and approved institutional policy remain separate gates.

### Actual clocks and exact-attempt recovery (D-0533)

| Invariant | Executable proof |
|---|---|
| Foreign cached geometry cannot move an actual15:09 forced-close record to15:20 in either direction, column mode, predicate mode or public grid | `foreign_slice_deadline_readiness::every_public_cached_walk_refuses_foreign_facts_that_move_1509_to_1520` |
| Actual clock checks preserve earlier valid ordinary horizons and refuse a cached lookup that names a different actual instant | `actual_clock_guard_preserves_earlier_horizons_on_both_public_predicate_paths`, `matching_forced_boundary_cannot_hide_a_foreign_earlier_horizon_lookup` |
| Valid endpoints cannot hide a late or foreign-day interior timestamp from stop/target pricing | `foreign_slice_deadline_readiness::checked_grids_refuse_an_actual_late_or_foreign_day_inside_cached_endpoints` |
| Only a canonical positive u64 token selects status; aliases, duplicate keys and overflow refuse | `strict_tests::exact_status_selector_requires_one_canonical_positive_u64_without_aliases` |
| Unrelated active or unknown CLI activity cannot hide a matching retained browser terminal | `strict_tests::exact_status_retains_browser_terminal_despite_active_or_unknown_external_status` |
| Running/refused outcomes retain their exact identity; absent/replaced slots never substitute another attempt | `strict_tests::exact_status_keeps_running_and_refused_states_and_never_substitutes_another_attempt` |
| Every batch poll and manual recheck addresses its accepted decimal token, including values above JavaScript integer precision | `web/tests/receipt-batch.test.js` exact-selector polling and recheck regressions |

### Parallel strict-test isolation (D-0534)

| Invariant | Executable proof |
|---|---|
| Strict range evidence tests cannot observe another test's temporary process-wide knob values | The shared `knobs::serially()` guard spans `actual_strict_range_kernel_publishes_and_reuses_native_and_coarse_evidence`, `actual_strict_range_terminal_refuses_changed_source_binding_or_acknowledged_rank`, `strict_api_adapter_reuses_real_parent_and_returns_late_rank_loss_as_error` and `strict_preparation_source_change_seals_refusal_before_returning_error`; verified together by `cargo test --workspace --locked` |

### Explicit research policy (D-0535)

| Invariant | Executable proof |
|---|---|
| One supplied profile resolves all37 delegated fields; risk multiples preserve exact units and values | `research_policy::tests::shipped_profile_resolves_all_37_without_requiring_manual_values` |
| Every missing, duplicate, unknown, reserved stated or invalid version field refuses | `every_missing_duplicate_unknown_and_stated_field_refuses` |
| Invalid integers, booleans, non-price risk expressions, overflow, zero risk, bad UTF-8 and excessive input refuse | `exact_domains_overflow_and_malformed_values_refuse` |
| Exact file bytes and risk provenance are deterministic, including comment-only file edits | `file_bytes_and_risk_are_audited_even_if_numeric_values_match` |
| Missing, directory, final-symlink and oversized file inputs refuse | `disk_reader_refuses_missing_directory_symlink_and_oversized_input` |
| The executable explanation validates the complete runner policy without market input or hidden override values | `executable_policy_check_validates_without_reading_markets_or_overrides` |
| Human percentages, ratios and signed price units retain exact precision | `human_limits_preserve_exact_rates_ratios_prices_and_special_rules` |
| Selected policy reaches V6 physical preflight without manually supplying gates; explicit overrides are visible and malformed values refuse | `selected_runtime_profile_reaches_v6_preflight_and_preserves_override_refusals` |
| The guide contains every configured field once and its human limit agrees with the file | `node web/policy-guide/build-guide.mjs --check` (frontend only) |
| Exactly16KiB can hold a complete valid profile; one extra byte refuses without accepting a prefix | `research_policy::adversarial_tests::exact_file_bound_accepts_complete_text_and_one_extra_byte_refuses` |
| A loaded profile owns its snapshot; rereading a changed or removed file authenticates the new actual state | `loaded_profile_is_an_owned_snapshot_and_rereads_require_current_real_bytes` |
| Signed and unsigned risk extremes cannot wrap or cross price domains | `signed_and_unsigned_risk_extremes_never_wrap_or_cross_domains` |
| Disabling an optional rejection rule does not claim the underlying evidence was measured; alternate count/rate limits render exactly | `edited_optional_checks_and_count_limits_explain_applied_values_without_claiming_evidence` |
| Owner-selected file paths, per-field override precedence and malformed-policy refusal remain isolated from request knobs | `isolated_process_precedence_preserves_owner_selected_path_and_refusals` and five named isolated child cases |
| Appending, replacing or removing the actual policy file between its bounded read and final generation check refuses; a subsequent clean read remains usable | `real_policy_file_mutations_inside_the_read_window_refuse_without_returning_a_snapshot`, using a one-shot thread-local test-only mutation hook |

The7 September final policy run passed13 unique tests plus6 isolated child
executions. Exact-binary coverage measured100% lines and branches for parsing,
risk resolution, human formatting and fixed-point formatting. The reader is
94.12% lines and50% branches: during-read generation failure was not injected.
A defensive parser index-guard region is unreachable through the current closed
field enum and remains uncovered. This is not whole-CLI coverage closure.
Evidence: `target/sweep-audit-20260906/research-policy-coverage/final-tests.log`
and `final-selected.txt`; merged-profile diagnostics were empty.

A later14-test run plus the same6 isolated child executions passed after adding
the actual read-window mutation regression. This closes the named behavioral
gap; the earlier13-test coverage percentages remain a dated measurement until
a new exact-binary instrumented run is recorded. Log:
`target/sweep-audit-20260906/research-policy-read-window-tests.log`.

The subsequent exact-binary instrumented rerun also passed14 tests plus6 child
executions, with empty profile/coverage diagnostics. `ResearchPolicy::read`
now has18/18 measured lines and2/2 branches;33/36 regions are covered. Initial
generation and low-level read I/O error closures remain untested. Parsing and
the three value-formatting functions retain their previously measured complete
main-function line/branch coverage, with the same closed-schema index-region
limit. The coverage README records both checkpoints without merging profiles.

### Actual-clock assertion closure (D-0533 follow-up, 2026-09-07)

| Invariant | Executable proof |
|---|---|
| Signed/extreme timestamp day identity uses the exact civil interval; absent, reversed, subminute, foreign-day and unrepresentable forced-exit endpoints refuse | `actual_day_matches_civil_interval_membership_at_signed_extremes_and_midnight`, `square_off_requires_available_ordered_actual_endpoints_and_a_representable_deadline`, `square_off_exact_intervals_reject_foreign_days_subminutes_and_late_entry` |
| Earlier ordinary horizons remain valid without forging exact15:09 forced fills; cached horizon indices must match actual timestamps and positive cadence | `truncated_boundaries_allow_earlier_holds_without_forging_forced_one_minute_fills`, `horizon_lookup_requires_positive_cadence_exact_actual_timestamp_and_present_index` |
| Actual path bounds admit inclusive whole-minute endpoints and refuse absent, reversed, duplicated or backward timestamps | `clock_accepts_inclusive_whole_minute_endpoints_and_never_duplicate_or_reversed_time`, `clock_missing_or_reversed_endpoints_cannot_admit_an_actual_record` |
| Refused clock/candle rows cannot change extrema; counted refusals preserve offset alignment | `admission_keeps_clock_membership_and_candle_refusals_out_of_extremes`, `admission_counts_duplicate_clock_once_and_preserves_extremes_offset_alignment` |

Fresh exact-binary profiles cover the selected clock functions and closures at
100% measured lines/regions/available branches, with58 test executions and no
profile mismatch diagnostics. A63-mutant replay caught all63; the earlier public
suite alone missed8. This is selected-function evidence, not whole-runner or
whole-module100% coverage. The exact evidence manifest is
`target/sweep-audit-20260906/clock-boundary-final-coverage/README.md`.

### Full-program research families, statistics and comparison (D-0536–D-0537)

| Invariant | Executable proof |
|---|---|
| Research scope matches the existing snapshot intersection and cannot alias cash, derivatives, references or nonmembers to a legacy index | `research_family_readiness::runtime_scope_matches_the_independent_snapshot_intersection`, `scope_refuses_impostors_duplicates_and_limits_without_silent_truncation` |
| Named capacity errors, every family-codec byte, complete membership identity and required observed stops remain explicit | `scope_refusals_report_the_named_problem_and_actual_physical_limit`, `fixed_family_codec_rejects_every_changed_byte_and_wrong_length`, `membership_identity_binds_the_complete_ordered_snapshot_tables`, `exact_observed_required_stop_retains_its_public_coordinate` |
| Shared cash resolution preserves old two-index bytes and prices/replays every Boolean coordinate without an AND or index alias | `legacy_resolution_identity_matches_the_recorded_pre_extraction_library`, `eligible_cash_prices_and_replays_every_boolean_coordinate_without_index_alias` |
| Every supplied program, direction and exit cell retains exact trade/session conservation, zero coordinates, full grid settings and horizon | `strict_boolean_catalog_keeps_every_coordinate_exact_rows_and_unknowns_and_reuses` |
| Body, receipt or actual source replacement invalidates retained candidate authority; acknowledged body loss cannot publish completion | `committed_boolean_family_refuses_body_receipt_and_actual_source_replacement`, `acknowledged_boolean_candidate_body_loss_refuses_before_completion_publication` |
| Duplicate catalogs and physical-cap failures cannot publish a complete candidate family | `malformed_catalog_and_physical_caps_refuse_without_a_completed_candidate_parent` |
| Observers cannot accept a publisher's uncompleted durability window; owner replacement refuses and an existing reader cannot indefinitely block identical reruns | `observer_holds_publication_barrier_and_refuses_changed_owner` and the retained-reader rerun assertions in the complete catalog test |
| Complete zero-inclusive program populations use the existing exact statistics kernels; constant-return candidates cannot gain invented Romano–Wolf evidence | `complete_program_population_matches_existing_exact_bootstrap_receipts`, `zero_trade_coordinate_remains_in_family_and_cannot_gain_rw_evidence` |
| Work, memory arithmetic and odd/misaligned periods refuse without a shortened or padded population | `explicit_physical_limits_and_overflow_refuse_before_shared_numeric_work`, `cscv_keeps_all_periods_and_refuses_misalignment_without_padding`, `actual_source_odd_calendar_and_foreign_catalog_refuse_without_statistics_completion` |
| Real producer capabilities from complete generated stored cash/index fixtures reach durable statistics, exact reuse and subsequent corruption refusal | `actual_opaque_cash_and_index_sources_publish_complete_idempotent_statistics`, `committed_statistics_refuse_tampered_body_receipt_and_linked_candidate` |
| A complete explicit policy comparison retains every candidate and every missing later-period reason; policy changes rekey and corruption refuses | `complete_cash_boolean_admission_retains_every_reason_and_missing_oos_cannot_pass`, `admission_policy_identity_resource_refusal_and_corruption_are_explicit` |
| A missing or refused worker result cannot be mistaken for a complete requested cohort | `boolean_catalog_command::tests::incomplete_or_refused_worker_results_cannot_complete_a_cohort` |

The initial stored-fixture success assumptions were corrected from measured
calendar and predicate evidence: complete July–August has42 accepted sessions,
and a cash VWAP tautology is not a zero-signal expression. No actual session was
dropped or padded, and no production policy or cash availability was relaxed to
make those tests pass. The16 candidate/statistics/admission checks passed across
`boolean-final-combined-v2-tests.log` (14 passes,2 then-invalid admission fixture
policies) and `boolean-admission-final-v2-tests.log` (both repaired admission
fixtures pass). This is generated-fixture integration evidence, not a production
historical run or a later all-source release clearance. The final common-reader
extraction and added observers require their own subsequent combined checks.

The focused research-family/resolution/comparison mutation campaign reconciled
116 distinct descriptions:100 caught,15 unviable and one surviving redundant
early family-codec guard. Replacing its width/magic `||` with `&&` still cannot
admit an invalid encoding because the final canonical re-encoding rejects it.
The defensive guard is retained and the survivor is reported, so this is not a
zero-survivor result. The corresponding13-test instrumented scope measured
research-family169/169 lines and18/18 branches, research-resolution147/151 lines
and7/8 branches, and detached comparison56/56 lines and2/2 branches. Later cold
observer codec changes are outside that source checkpoint. Evidence is under
`target/sweep-audit-20260906/research-family-coverage*` and the reconciled mutation
artifacts; none of this closes whole-runner coverage or all touched modules.

### Saved Boolean observation (D-0538)

| Invariant | Executable proof |
|---|---|
| All three namespaces refuse an active publisher; an idle cached reader releases its lease so identical publication can finish | `every_namespace_observer_waits_for_publication_and_idle_cache_allows_rerun` |
| Unknown namespaces, over-budget bodies, corrupt content and replaced generations refuse; failed decoding/projection releases its lease | `observation_rejects_caps_unknown_namespaces_corruption_and_foreign_generation`, `refused_decoder_and_projection_release_the_publication_lease` |
| Cold statistics retain every candidate, split and source with exact aggregate byte admission; resealed padding, count, kind and source substitutions refuse | `cold_statistics_pages_conserve_all_candidates_splits_sources_and_exact_read_budget`, `cold_statistics_refuses_resealed_padding_counts_kinds_and_cross_linked_sources` |
| Cold admission preserves the complete saved policy, values and reason partitions and refuses changed ancestors or resealed policy/verdict substitutions | `cold_admission_preserves_complete_policy_all_values_reasons_and_refuses_changed_ancestors`, `cold_admission_refuses_resealed_policy_values_verdict_padding_and_foreign_stage` |
| HTTP selectors, completion pins and cardinality cannot substitute a later or partial page; unknown storage roots are not searched or created | `exact_model_selectors_require_completion_for_every_later_or_detail_page`, `page_cardinality_and_completion_never_accept_a_partial_or_replacement_page`, `missing_saved_receipt_names_configured_root_without_creating_or_searching_it` |
| All44 evidence fields and reason positions and all39 policy values preserve exact integers, unavailable states and false requirements | `all_44_evidence_fields_preserve_exact_extremes_and_each_unavailable_tag`, `all_44_reason_positions_render_their_exact_saved_partition_and_status`, `all_39_policy_values_preserve_complete_numeric_values_and_false_requirements` |
| Browser pages reconcile exact parent/source identities, cardinalities, floating bits and reason masks before display | `web/tests/boolean-catalog.test.js`, `web/tests/boolean-evidence.test.js` |

The final linked generated-fixture run passed23 tests and a subsequent final
cold-reader rerun passed4 tests. Logs are `boolean-linked-observation-final-tests.log`
and `boolean-linked-observation-frozen-tests.log` under the ignored sweep audit
directory. These do not establish original raw-source freshness on a cold saved
reader or claim a production historical campaign.

The later decoder revision uses fallible fixed-array conversion for record width,
preserving exact canonical validation while removing the redundant boolean
guards. The14-test focused rerun passed. The current decoder replay tested9
mutants:7 caught,2 unviable, no survivors. This is a targeted delta, not a fresh
whole116-mutant campaign. New exact-binary whole-file coverage is
research-family167/167 lines and14/14 branches, observer codec30/30 lines and6/6
branches, and detached comparison56/56 lines and2/2 branches. Resolution remains
147/151 lines and7/8 branches; whole-crate closure is not claimed. See
`research-fixed-width-decoders-mutation.log` and the updated ignored evidence
manifest for exact binary/source hashes and profile diagnostics.

| Invariant | Executable proof |
|---|---|
| A statistics work ceiling that overflows refuses before catalog, market or output access | `boolean_catalog_command::tests::impossible_statistics_budget_refuses_before_catalog_market_or_output_access` (isolated executable child with a valid profile and missing market/catalog paths) |
| Exhausted candidate capture reports the actual program, side, coordinate and required versus remaining trade/byte capacity without a completed parent | `malformed_catalog_and_physical_caps_refuse_without_a_completed_candidate_parent`, including a generated stored tautology with a one-trade ceiling |

### Incremental grammar campaign recovery (D-0541)

| Invariant | Executable proof |
|---|---|
| Binary batches preserve the existing cursor's exact ordering and resume without skipping programs | `bounded_batches_resume_without_skipping_reordering_or_false_exhaustion` |
| Cold batch decoding replays admitted work and rejects altered programs, counts, cursor bytes and padding | `binary_batch_replay_detects_changed_counters_programs_and_padding` |
| Invalid batch framing and physical count/length limits refuse before parsing a separately malformed inner cursor | `batch_format_and_byte_admission_precede_cursor_parsing`, `batch_count_and_exact_length_admission_precede_cursor_parsing` |
| Node-only progress stays paused and all cumulative counter/capacity arithmetic is checked | `node_only_progress_remains_paused_and_all_arithmetic_is_checked` |
| Only a terminal fixed grammar cursor reports exhaustion | `only_the_cursor_terminal_state_can_report_fixed_grammar_exhaustion` |
| Pending batches survive restart; a failed child completion check cannot advance the grammar cursor | `restart_keeps_unfinished_batch_and_requires_exact_campaign_before_advancing` |
| Resealed predecessor substitutions, skipped/repeated work, malformed completions and changed batch boundaries refuse | `sealed_but_skipped_reordered_foreign_and_incomplete_history_refuses`, `checkpoint_counter_reseeding_and_changed_batch_boundaries_refuse` |
| Empty bounded node work advances without inventing a historical campaign | `empty_node_work_checkpoint_can_resume_without_inventing_a_campaign` |
| Invalid alphabets, dates and zero work allowances refuse before output or market access | `command_shape_refuses_invalid_alphabet_spans_and_work_allowances` |
| Before pricing, reserve the full next checkpoint transition so unchanged bounds can recover it; charge interrupted ordinals, history bytes, the shared directory limit and arithmetic overflow | `checkpoint_admission_reserves_plan_and_done_before_work_and_restarts_at_exact_cap`, `checkpoint_admission_charges_crash_holes_memory_discovery_and_overflow` |
| Keep source guards through grammar completion and refuse same-byte inode replacement before or after acknowledgment without erasing saved history | `grammar_done_holds_and_rechecks_source_generation_before_and_after_acknowledgment` |
| Generated strict sources reach actual eight-timeframe execution, cold completion readers and exact retry; a genuine foreign campaign refuses before its ancestry reader is called | `generated_grammar_executes_and_recovers_real_eight_rung_receipts` |

These are finite regression obligations. They do not establish whole touched-
crate coverage, mutation closure, unbounded throughput or whole-grammar
statistical acceptance. Fresh combined test and real-OHLCV evidence must be
reported against the actual source checkpoint, separately from earlier runs.

### Eight-timeframe saved campaign (D-0539)

| Invariant | Executable proof |
|---|---|
| Fixed widths, padding, family scope and aggregate completion reconcile exactly | `fixed_campaign_codec_refuses_padding_width_status_scope_and_completion_forgery` |
| Ordered full-wire program digest preserves AND/OR/NOT and program order | `exact_program_digest_binds_order_operators_and_full_fixed_wire` |
| Saved pause remains observable under a writer lease and cannot become completed by age or liveness | `checkpoint_pause_resume_pin_history_and_owner_are_independent_of_liveness` |
| Overview checks exact child receipts, their byte budget and publication barriers without claiming to read bodies | `overview_checks_exact_receipts_and_budget_without_pretending_to_check_bodies` |
| Acknowledged links cannot change/disappear across progress; predecessor substitutions refuse | `campaign_history_refuses_crosswired_predecessors_and_changed_acknowledged_stages` |
| Reservation-only or exhausted checkpoint history cannot become completed evidence | `unacknowledged_snapshot_and_exhausted_reservation_never_become_complete` |
| All eight generated-fixture timeframes reach the real candidate/statistics/admission path, resume and preserve completed pins; changed raw sources, child bodies or program/horizon links refuse | `generated_all_eight_campaign_pauses_resumes_exactly_and_refuses_changed_source_or_child` |

### Frozen-exit later comparison (D-0540)

| Invariant | Executable proof |
|---|---|
| Every original coordinate is replayed, including zero/unknown programs | `later_replays_every_frozen_coordinate_and_preserves_zero_unknown_programs` |
| Original anchor/program/series cannot be substituted and overlapping data refuses | `later_refuses_overlap_program_substitution_foreign_series_and_changed_anchor` |
| Later price changes cannot re-resolve training levels; a later timestamp on the same IST session still refuses | `later_prices_cannot_reresolve_training_levels`, `a_later_timestamp_in_the_same_ist_session_is_not_an_oos_period` |
| Foreign resolution, evaluator substitution and entirely cold data refuse | `later_refuses_foreign_frozen_resolution_changed_evaluator_and_entirely_cold_data` |
| Durable later capture, exact original links, every zero session, cold pages, malformed bytes and 15:09/15:10 bar-open boundary are checked | `stored_later_comparison_preserves_population_zero_sessions_and_pinned_cold_pages` |
| Overlap, changed build, insufficient capacity and lost original receipt refuse | `stored_later_refuses_overlap_wrong_build_resource_and_original_receipt_loss` |
| Invalid or absent later command arguments refuse before I/O | `explicit_later_command_rejects_overlap_daily_zero_and_missing_arguments_before_io` |
| The later lifecycle round-trips as its distinct operation14 | `validation_request_and_completion_kind_remain_explicit` in `crates/cli/tests/sweep_evidence.rs` |

The automated overview/later projections and refresh races are also exercised
by `booleancampaignjson_tests.rs`, `booleanoosjson_tests.rs`, and the frontend
`boolean-campaign.test.js`, `boolean-oos.test.js`, `campaign-monitor.test.js`
suites. A browser projection test is distinct from a real raw-price replay.

### Zero-conservative family and fixed-training windows (D-0542–D-0544)

| Invariant | Executable proof |
|---|---|
| Full coordinate order retains exact zero and nonpositive hypotheses conservatively | `zero_and_nonpositive_rows_remain_mapped_with_conservative_one`, `negative_variable_rows_still_compete_in_positive_resample_maxima` |
| All-zero families produce no rejection; nonzero constants refuse | `all_zero_is_complete_non_rejection_but_nonzero_constants_still_refuse` |
| Full-family identity binds ordering, values, procedure and physical bounds | `full_identity_binds_zeros_order_every_value_procedure_and_physical_admission` |
| The mapped procedure agrees with an independent full-family zero-floor reference over1,152 small families | `complete_small_integer_families_match_explicit_zero_floored_full_suffix_reference` |
| Exact allocation does not renumber retries or round across thresholds | `every_predeclared_cell_shares_one_budget_and_retries_do_not_renumber`, `exact_boundary_scaling_and_draw_resolution_never_round_through_ppm`, `complete_small_probability_lattice_matches_unreduced_integer_comparison` |
| Whole declared civil partition precedes observed outcomes; zero/unreachable alpha stays explicit | `qualification_partition_covers_every_civil_day_before_later_observations`, `qualification_zero_and_unreachable_alpha_keep_all_eight_allocated_units` |
| Full plan identities, exact bounds, overlap and detached malformed partitions refuse | `qualification_identity_binds_both_full_sources_and_every_declared_term`, `qualification_plan_refuses_overlap_invalid_dates_and_exact_physical_caps`, `qualification_detached_codec_is_exact_bounded_and_cannot_hide_foreign_partition` |
| Actual selected-coordinate folds conserve every trade and zero window | `complete_fixed_original_fold_projection_matches_independent_trade_partition` |
| Both directions reject foreign programs, anchors, selected coordinates and ordinals | `both_sides_reject_foreign_full_program_anchors_selections_and_ordinals` |
| Timestamp extremes, missing actual windows, exact map capacity and numeric overflow refuse correctly | `exact_ist_day_preserves_both_timestamp_extremes_and_negative_epoch_boundaries`, `fixed_plan_rejects_training_overlap_missing_dates_empty_actual_folds_and_mapping_ceiling`, `defensive_numeric_overflow_and_zero_outcomes_cannot_be_encoded_as_success` |
| Optional fold integration preserves every existing OOS V1 byte and original authority | `optional_fixed_training_folds_preserve_every_v1_byte_and_exact_original_authority` |
| Foreign months, missing partition coverage and additional proof-memory limits refuse | `optional_fold_proof_refuses_foreign_month_partitions_and_additional_memory_bounds` |
| Eight journal slots resume exact pending reservations and cannot be reassigned | `all_eight_slots_restart_without_reassignment_and_reuse_exact_pending_reservations` |
| Failed or uncertain publication retains old acknowledged state and poisons the writer | `failed_begin_finish_and_refusal_keep_old_slots_and_poison_the_uncertain_writer`, `uncertain_readback_never_exposes_unacknowledged_completion_and_reopen_checks_the_child` |
| Crash holes count toward limits; resealed history cannot omit acknowledged transitions | `exact_checkpoint_cap_counts_crash_holes_but_not_as_acknowledged_records`, `validly_resealed_latest_cannot_omit_acknowledged_intermediate_history`, `resealed_state_cannot_skip_begin_retract_completion_or_crosswire_units` |
| Malformed scope/completion and bounded UTF-8 refusal text remain explicit | `scope_initial_record_foreign_identity_and_partial_completion_refuse`, `utf8_refusal_is_bounded_retained_and_retry_clears_only_the_pending_reason` |
| Complete original/later qualification reopens every coordinate, preserves zero families and refuses missing fold authority, foreign plans or changed source evidence | `complete_original_later_qualification_reopens_all_coordinates_and_keeps_zero_families`, `qualification_refuses_missing_original_fold_authority_wrong_plan_and_changed_source` |
| Exact finite allocation cannot cross the policy threshold through rounded ppm observations | `qualification_probability_projection_preserves_exact_finite_allocation_boundary` |
| Saved numeric mapping retains unreduced conservative one and rejects altered ranks, probabilities, plans, padding or recurrence | `qualification_codec_preserves_unreduced_zero_one_and_complete_active_mapping`, `qualification_codec_refuses_resealed_rank_probability_plan_and_padding_changes`, `qualification_codec_reconciles_exact_stepdown_recurrence_and_section_overflow` |
| Cold source audit reproduces full matrix identity and original statistics without claiming to rerun resampling | `cold_source_audit_reproduces_full_identity_and_statistics_without_resampling` |
| Every ordered original and later identity belongs to its predeclared rung, and changing any of the sixteen bindings changes the scope | `qualification_ordered_source_bindings_preserve_every_identity_and_family_position`, `qualification_identity_binds_both_full_sources_and_every_declared_term` |
| Read-only eight-slot observation permits a concurrent owner, preserves refusal and rejects changed predecessor history without opening a writer or authenticating child bodies | `read_only_overview_observes_owner_and_all_eight_slots_without_opening_child_bodies`, `overview_keeps_recorded_refusal_and_refuses_changed_pinned_history` |
| Missing or malformed command arguments and overlapping training/later spans refuse before source or output access | `qualified_command_rejects_invalid_or_overlapping_scope_before_io` |
| Fresh overall completion rechecks every earlier saved child, and missing evidence before or after final acknowledgment cannot produce a success result | `final_publication_rechecks_all_saved_children_and_reopens_the_same_eight_links`, `deleted_earlier_child_blocks_final_publication_and_preserves_a_durable_refusal`, `failure_after_final_acknowledgment_is_explicit_and_never_rewrites_saved_history` |
| Runtime observation admission retains exact positive limits and rejects malformed or unaddressable configuration without fallback | `missing_defaults_and_exact_positive_runtime_budgets_are_retained`, `malformed_or_unaddressable_values_refuse_without_default`, `non_utf8_configuration_and_authentication_failure_are_explicit` |
| A smaller observation allowance cannot reuse a previously admitted catalog or later cache; every evidence model reports the exact configured bound | `a_smaller_server_budget_cannot_reuse_a_previously_admitted_catalog`, `lowered_budget_refuses_cached_later_ancestry_and_original_pin_recovers_after_readmission`, `every_evidence_model_names_the_exact_server_owned_budget_on_authentication_refusal` |

Test names identify obligations, not an assertion that all verification gates
have passed on any later source revision. Record the actual combined run and
remaining coverage/mutation obligations separately.

### Search-wide allowance and durable grammar continuation (D-0548–D-0549)

| Invariant | Executable proof |
|---|---|
| History and Plan admission bound actual retained capacity; oversized sealed records refuse before replay | `one_slot_history_admission_bounds_actual_retained_capacity`, `plan_buffer_capacity_stays_within_exact_serialized_admission`, `restore_rejects_sealed_oversize_before_body_decode_or_child_replay` |
| Descriptor/program, seal/payload and Plan sequence/seal/width checks remain independently required | `campaign_projection_requires_each_exact_descriptor_and_program_field`, `acknowledged_record_requires_both_expected_seal_and_exact_payload`, `done_link_requires_exact_plan_sequence_seal_and_record_width` |
| Countable batch/timeframe shares telescope under one alpha and retain exact identity on retry | `independently_accumulated_finite_prefixes_telescope_without_exceeding_alpha`, `fixed_slots_retries_and_digest_bind_the_entire_numeric_contract` |
| Exact fractions, draw floors, arithmetic ceilings and every small probability-lattice boundary remain conservative | `exact_threshold_draw_floor_and_scaled_probability_share_one_boundary`, `probability_lattice_matches_direct_unreduced_integer_comparison`, `nonintegral_draw_threshold_is_rounded_up_before_subtracting_one`, `machine_and_draw_ceilings_refuse_without_wrapping_or_reusing_a_slot` |
| Reservation, refusal and completion preserve the same pre-work binding; only completed work advances | `generated_reservation_completion_and_refusal_round_trip_without_changing_binding`, `ordered_transitions_require_prework_reservation_and_preserve_failed_slot_on_retry` |
| Complete declared program allocation is admitted independently before corrupted-cursor replay | `physical_grammar_reservation_precedes_corrupt_cursor_replay` |
| Truncations, altered padding, partial predecessors, changed source/policy/programs and forged rung counts cannot become accepted records | `every_record_truncation_and_exact_header_length_refuses`, `every_byte_mutation_is_refused_or_remains_an_exact_canonical_observation`, `refusal_utf8_padding_and_predecessor_parts_are_exact`, `frozen_source_policy_program_limits_and_allocation_cannot_be_substituted`, `missing_overflowed_or_reassigned_rung_results_never_count_as_completion` |
| Node-only grammar progress carries no invented qualification or results | `node_only_progress_carries_no_invented_qualification_or_results` |
| Wrong search identity refuses before grammar replay; aggregate byte and node limits do not silently become per-record limits | `foreign_requested_identity_is_rejected_before_a_malformed_grammar_is_replayed`, `exact_complete_history_bytes_and_replay_allowances_are_not_per_record_fallbacks` |
| Lost or skipped acknowledged history cannot fall back to older success | `skipping_an_acknowledged_record_or_replacing_its_pin_refuses_the_entire_history`, `lost_or_corrupt_latest_acknowledgement_never_falls_back_to_an_older_snapshot` |
| Journal completion is not child authority; malformed/empty/missing observation remains explicit | `journal_reopens_pending_refused_and_complete_observations_without_inventing_child_authority`, `unknown_empty_and_zero_replay_requests_have_distinct_explicit_refusals` |
| Search correction changes only probability evidence; it preserves the complete original observations and all absent authority | `search_projection_changes_only_exact_probability_fields_and_retains_full_source`, `missing_fold_and_execution_authority_is_never_replaced_by_numeric_success`, `inconsistent_detached_source_verdict_cannot_be_promoted` |
| Equality may pass, a fractional ppm excess cannot round down to pass, and insufficient draw resolution never increases draws or admits a result | `equality_passes_but_fractional_ppm_excess_never_rounds_into_admission`, `unreachable_draw_resolution_keeps_evidence_and_draws_without_positive_admission` |
| Genuine generated all-eight qualification resumes the same refused batch, refuses a lost child before new work, preserves restored old pins and rejects paging after parent loss | `generated_search_recovers_same_ordinal_and_refuses_missing_ancestry` |
| Failed terminal publication retains the original work error, while failure after acknowledged completion never rewrites that history | `search_settlement_preserves_original_failure_when_terminal_audit_cannot_be_written`, `search_settlement_retains_acknowledged_completion_and_names_late_failure` |
| Malformed command scope and resource values refuse before source access; changing an invocation pause preserves fixed batch work and grammar | `search_command_refuses_missing_scope_invalid_dates_and_malformed_allowances_before_io`, `search_invocation_pause_allowance_does_not_change_initial_grammar_or_fixed_batch_work` |

Test fixtures labelled generated are fault/contract checks, not historical market
results. File-byte mutation enumeration is a codec property check, distinct from
mutation-testing production code. These names do not certify later revisions or
full-crate coverage; preserve actual result logs beside the source checkpoint.

### Exact runtime exit-grid wiring (D-0551)

| Invariant | Executable proof |
|---|---|
| Missing resolution and explicit five retain the original directional policies and digests | `absent_and_explicit_five_preserve_original_policy_and_digest` |
| Every admitted scalar resolution constructs its exact three axes, preserving risk, execution and cell limits; two-level policies match an independent source-only census | `every_admitted_runtime_resolution_binds_exact_axes_without_changing_risk` |
| Invalid, zero, fractional, overflowing and over-ceiling settings refuse without default or clamping | `invalid_runtime_resolution_refuses_without_default_or_clamp` |
| Changing either directional exit policy changes the actual source-only candidate descriptor, while a catalog-only change preserves that descriptor | `generated_source_descriptor_binds_both_runtime_exit_policies_independent_of_catalog` |
| Node allowances exceeding record admission, missing completion pins and orphan summaries cannot become accepted search evidence | `declaration_rejects_progressed_cursors_zero_authority_and_invalid_physical_caps`, `missing_overflowed_or_reassigned_rung_results_never_count_as_completion`, `node_only_progress_carries_no_invented_qualification_or_results` |
| An actual linked search detail reports its complete parent/campaign/child byte admission and selected-batch replay work; wrong child pins and undeclared timeframe ordinals refuse | `generated_search_recovers_same_ordinal_and_refuses_missing_ancestry` |

### Explicit settings and nested failure settlement (D-0552)

| Invariant | Executable proof |
|---|---|
| Each supported ordinary-range scalar retains its ordinary meaning, but every unused legacy control refuses on the explicit Boolean path | `boolean_rejects_each_unused_legacy_setting_without_changing_ordinary_validation` |
| The Boolean grid setting retains the shared exact scalar boundary without accepting another legacy control | `boolean_grid_resolution_keeps_the_shared_exact_boundary` |
| Request overrides and malformed/non-UTF-8 environment values are checked; actual Boolean preparation rejects unsupported controls before absent/inaccessible policy and source paths | `boolean_request_preflight_checks_override_non_utf8_and_actual_prepared_boundary` |
| An actual refusal-publication collision preserves the original cause and publication error, states unconfirmed persistence, poisons the failed writer and reopens as pending | `failed_refusal_publication_preserves_original_reason_and_reopens_pending_state` |
| Successful refusal publication remains observable, while subsequent verification failure after completion retains the acknowledged history | `deleted_earlier_child_blocks_final_publication_and_preserves_a_durable_refusal`, `failure_after_final_acknowledgment_is_explicit_and_never_rewrites_saved_history` |

### Versioned shared probability ceilings and release admission (D-0553, D-0554)

| Invariant | Executable proof |
|---|---|
| A writer-valid unequal FWER/Romano policy cannot let V2 White/SPA comparisons exceed the shared ceiling; the exact original V1 outcome remains observable | `stricter_shared_search_allowance_cannot_be_relaxed_by_individual_policy_ceilings` |
| Effective policy only tightens four probability fields, retains every other value, preserves original bytes and is idempotent | `shared_probability_cap_preserves_every_other_policy_value_and_never_loosens_a_ceiling`, `search_policy_exposes_effective_limits_without_replacing_the_original` |
| Equality, one-unit-above, zero and unrepresentable exact fractions have explicit outcomes without optimistic rounding | `unequal_family_ceilings_keep_exact_boundary_failures_as_saved_rejections`, `zero_shared_allowance_rejects_positive_probability_without_aborting_the_search`, `exact_probability_projection_refuses_unrepresentable_integer_boundaries` |
| V1 bytes/hash remain independently pinned; V2 has a distinct declared identity and neither version can enter the other's history | `legacy_projection_spec_has_pinned_original_bytes_and_digest`, `legacy_records_round_trip_exactly_and_new_rule_changes_only_versioned_identity`, `mixed_or_unknown_projection_headers_refuse_before_inner_replay`, `projection_rule_cannot_change_inside_an_existing_search_history` |
| Historical defensive arithmetic is retained without claiming inconsistent detached alpha is an actual writer configuration | `legacy_probability_guard_remains_exact_for_inconsistent_detached_allocations` |
| Self/forward ancestry, acknowledged retries beyond admission, corrupted derived caches and a skipped reservation cannot become accepted progress | `self_or_forward_predecessor_refuses_before_opening_a_foreign_checkpoint`, `declared_record_allowance_counts_every_acknowledged_retry`, `replay_cache_corruption_cannot_replace_authenticated_history_or_completed_positions`, `a_completed_result_cannot_skip_the_next_prework_reservation` |
| UI checks the exact recorded rule, original/effective policy and row-policy binding, and cannot silently relabel a historical result | `historical version remains explicit and cannot silently become a current shared-limit result`, `missing, unrelated, loosened and incorrectly bound effective policies refuse`, `an admitted label cannot override the exact shared probability bound` |
| Missing release measurements refuse before live observation, including completely offline execution | `offline_and_missing_mandatory_admission_never_invoke_live_observation`, `absent_stale_incomplete_foreign_or_missing_mandatory_measurements_refuse` |
| Coverage requires all scoped source files and exact raw counts; a partial mutation census, survivor or timeout cannot clear release | `coverage_requires_exact_line_branch_counts_and_all_source_files`, `survivors_timeouts_unviable_and_incomplete_mutant_census_never_clear_release` |
| Compiler-invalid mutations need exact completed compiler evidence and cannot hide infrastructure failure | `compiler_invalid_is_accounted_separately_with_exact_completed_build_evidence`, `timeout_infrastructure_foreign_or_incomplete_build_is_not_compiler_invalid` |
| A missing active recovery plan returns503 and a visible error without creating or mutating history | `missing_active_plan_is_503_and_visible_without_creating_or_mutating_history` |

Names identify proof obligations, not automatic gate passes. Preserve actual
execution, source and measurement receipts; no finite test set proves every
possible market input, filesystem race or whole-crate coverage.

### Release evidence remains bound through observation (D-0556)

| Invariant | Executable proof |
|---|---|
| Observation cannot replace or mutate already verified artifact, source or measurement bytes and retain admission | `terminal_observation_rechecks_artifact_source_and_measurement_bytes` |
| Newly added source/dashboard build files are checked again with the original inventory bounds | `terminal_observation_rechecks_complete_source_and_dashboard_censuses` |
| A genuine caught outcome retains completed Build then Test phases in order | `genuine_cargo_test_phase_shape_is_retained_by_complete_census`, `caught_summary_requires_both_completed_phases_in_order` |
| A caught summary cannot override successful tests, timeouts, signals or infrastructure failures | `caught_summary_never_overrides_success_timeout_or_infrastructure_status` |

These checks bind the bytes observed at the final verification point; they do
not claim an exclusive lease against future writers.

### Unambiguous release reports and incomplete child refusal (D-0557)

| Invariant | Executable proof |
|---|---|
| Raw report keys are unique at every nesting level after escape decoding and integer values retain exact precision | `raw_report_json_preserves_exact_numbers_and_rejects_recursive_duplicate_keys` |
| Physical input and recursion limits remain finite; trailing JSON refuses | `strict_report_json_keeps_byte_recursion_and_complete_input_bounds` |
| Contradictory checks and status cannot overwrite a failure or active observation | `duplicate_checks_and_status_fields_are_refused_without_observation` |
| Duplicate coverage or mutation fields cannot become passing measurements; noninteger/overflow counts refuse | `duplicate_coverage_census_and_outcome_fields_cannot_replace_failures`, `coverage_requires_exact_line_branch_counts_and_all_source_files` |
| A different real campaign checkpoint cannot replace a pinned completed-search claim | `a_new_campaign_checkpoint_cannot_replace_the_exact_pin_in_a_completed_search_claim` |
| An exact campaign pin with waiting, started or refused slots cannot establish completed children | `matching_campaign_pin_cannot_promote_waiting_started_or_refused_slots_to_complete` |

The D-0558 frontend preparation is checked by running the production check
command with the generated framework configuration absent; it is not a new
backend requirement or a test of visual layout.

### Browser startup, saved-result visibility and request ownership (D-0559–D-0560)

| Invariant | Executable proof |
|---|---|
| A validated saved feed avoids inventory discovery; first visits still prefer actual held data using HEAD only | `web/tests/feed-startup.test.js`: saved preference and first-visit held-data cases |
| Missing, malformed, degraded or foreign census headers never become fabricated zero rows | `web/tests/feed-summary.test.js` |
| An explicit choice or clear cannot be overwritten by late discovery or census replies | `web/tests/feed-startup.test.js`, compiled store-module cases in `web/tests/store-startup.test.js` |
| Concurrent selected-feed consumers share one fetch and parse; refresh and failures retain their identity | `web/tests/store-census.test.js` |
| A disposed status read cannot publish or resurrect a poll; hidden health checks stop | `web/tests/page-requests.test.js` |
| Comparison work is opt-in, bounded to two groups, and cannot publish a cancelled partial population | `web/tests/bounded-comparison.test.js` |
| Empty ordinary ledger wording does not assert that saved Boolean research is absent | `web/tests/page-requests.test.js`: ordinary-ledger scope case |
| Browser result selection cannot name a store root, repeat keys or invent an out-of-page setting | `web/tests/saved-backtest.test.js` |
| An older or disposed result request cannot replace the current selection, and refusal never becomes empty success | `web/tests/saved-backtest.test.js` |
| The local viewer refuses nonloopback launches, control methods, foreign pins and unsafe assets without creating store history | native tests in `web/saved-backtest/viewer.rs` |

Actual browser evidence for the saved market-data run and the isolated full
frontend is recorded under `target/sweep-audit-20260906/page-load-20260907/`.
Those finite observations are not an exhaustive browser/platform or full-sweep
certification. The inspector does not relax the existing release gates.

## DB preparation and saved-result explanations — D-0561 to D-0563

| Invariant | Executable proof |
|---|---|
| DB preparation publishes one complete current snapshot; cancellation, failed decoration and stale work cannot publish partial totals | `web/tests/database-preparation.test.js` |
| Chunked prefixes preserve complete buckets and initial pickers wait for actual held segments | `web/tests/database-preparation.test.js`: mixed-rung, full-store, generated 148,222-row and actual-page picker cases |
| Visible setting joins are limited to 32 rows, split gaps, read sequentially and preserve the exact original/later ancestry | `web/tests/saved-backtest-setting-pages.test.js` |
| Duplicate coordinates, missing numeric facts, changed pins, caller edits during reads and later-page failure cannot become a partial successful comparison | `web/tests/saved-backtest-setting-pages.test.js` |
| Integer money, ppm distances, policy ratios, missing values and measured zero retain distinct exact meanings | `web/tests/saved-backtest-explanations.test.js` |
| Rule descriptions consume the supplied Rust vocabulary; NOT never turns unknown VWAP into a known entry | `web/tests/saved-backtest-explanations.test.js`; native vocabulary test in `web/saved-backtest/viewer.rs` |
| Frontend publication preserves every old immutable chunk and verifies its sealed source, destination and rollback bytes | native `staging_apply_retry_and_rollback_preserve_both_chunk_generations` in `web/sweep-readiness/frontend-publish.rs` |
| Changed files, duplicate manifests, symlinks, lock contention and an interrupted apply remain explicit refusal or resumable states | remaining native tests in `web/sweep-readiness/frontend-publish.rs` |

These frontend and inspection proofs do not close the outstanding complete
touched-crate coverage, full-module mutation or production recovery requirements.

## Explicit main-app inspection — D-0564

| Invariant | Executable proof |
|---|---|
| Only the complete declared inspection mode disables execution with that label; failures stay closed and a legacy 404 is named separately | `web/tests/runtime-inspection.test.js` |
| A saved-results link must use canonical loopback HTTP and the exact saved-selection contract | `web/tests/runtime-inspection.test.js`; native saved-link test in `web/sweep-readiness/main-inspector-tests.rs` |
| Run and Descend refuse before touching command state or transport when execution is disabled | actual-function cases in `web/tests/runtime-inspection.test.js` |
| All control methods, unknown GET routes and foreign request authorities refuse before the inner handler | `web/sweep-readiness/main-inspector-tests.rs` |
| Real read-handler fixtures retain every file and directory; construction starts no acquisition or recovery | `actual_read_handlers_preserve_every_fixture_byte_and_directory` in `web/sweep-readiness/main-inspector-tests.rs` |
| A route-shaped frontend asset cannot grant a backend read, and empty directories also count toward traversal bounds | `asset_inventory_cannot_open_extra_backend_routes_and_bounds_empty_directories` in `web/sweep-readiness/main-inspector-tests.rs` |
| Missing exact-setting links are explicitly refused even inside a final short page | `an exact result in the final short page must exist before it is shown as opened` in `web/tests/saved-backtest.test.js` |
| Both original and later execution refusals remain visible ahead of secondary measured failures | `web/tests/saved-backtest-explanations.test.js`: period-refusal and missing-period cases |

## Inspection navigation, frozen labels and final read ownership — D-0565 to D-0567

| Invariant | Executable proof |
|---|---|
| Cross-port user navigation admits only already permitted HTML documents; JSON, frames, background requests, origins, bodies and duplicate headers still refuse | native `user_document_navigation_crosses_ports_but_evidence_and_frames_do_not` and `navigation_does_not_admit_foreign_origins_duplicate_headers_or_bodies` in `web/saved-backtest/viewer.rs`; corresponding document tests in `web/sweep-readiness/main-inspector-tests.rs` |
| Busy retries have at most three attempts, preserve Retry-After bounds and final failures, and cannot restart after cancellation | `web/tests/saved-backtest-requests.test.js` |
| Trade and zero-inclusive session pages match the displayed setting's exact original/later identities, exits, totals and refusals | final seven join cases in `web/tests/saved-backtest-setting-pages.test.js` |
| Failed comparison loading disables all row inspection; a third busy response cannot publish a selected trade | live browser fault-injection evidence under `target/sweep-audit-20260906/db-results-20260908/`; error responses alone were injected, never market bars or results |
| Historical vocabulary has an explicit snapshot-byte pin and complete native-table validation; changed fields refuse even with a recomputed matching file pin | native `recorded_vocabulary_refuses_changed_pins_tables_versions_and_commits` in `web/saved-backtest/viewer.rs` |
| Captured historical vocabulary and saved-grid bindings do not invent a current reader source commit | actual captured vocabulary integration in `vocabulary_comes_from_linked_rust_table_and_foreign_grid_refuses`; final `/viewer.json` evidence |
| Changing only the imported saved-selection contract changes the main version; its absence refuses versioning | both actual-function cases in `web/tests/source-digest.test.js` |

These are finite inspection and frontend proofs. They do not replace the
outstanding complete crate coverage, module mutation, dependency-policy,
production recovery and full-sweep activation requirements.

## Durable invocation and integrated research inspection — 2026-09-08

| Invariant | Executable proof |
|---|---|
| Outer invocation IDs survive reopen; a saved start is not process liveness | `cli::operation_audit::tests::exact_status_survives_reopen_and_never_equates_a_start_with_liveness` |
| The reserved namespace base is never an invocation, before or after index creation; exact maximum IDs remain lossless | `cli::operation_audit::tests::durable_id_boundaries_are_exact_before_and_after_index_creation` |
| The oldest saved invocation ends newest-first pagination with no invented continuation | `api::operation_audit::tests::oldest_invocation_ends_the_exact_newest_first_cursor` |
| Duplicate terminals, busy writers, torn files and failed sync/partial writes cannot acknowledge success | `cli::operation_audit::tests::{a_second_terminal_or_later_progress_never_appends,busy_index_refuses_without_reserving_or_dispatching_an_invocation,torn_index_and_torn_invocation_are_preserved_and_refused,disk_full_partial_write_and_sync_failure_are_never_acknowledged}` |
| Panic/drop have explicit terminal outcomes when storage permits; scoped progress never borrows another thread's identity | `cli::operation_audit::tests::{dropped_and_panicking_owners_record_distinct_terminal_facts,thread_context_is_scoped_and_counts_only_explicit_boundaries}` |
| Production HTTP records its bounded public request separately, refuses before dispatch when admission fails, and leaves its audit reader read-only | `api::operation_audit::tests::{production_router_wires_durable_request_audit_and_its_read_only_reader,failed_audit_start_never_polls_the_handler,failed_terminal_says_that_the_handler_already_ran}` |
| Missing activated recovery history cannot be recreated by boot or a same-scope start | `api::recovery::tests::missing_activated_plan_cannot_be_recreated_by_boot_or_same_scope_start` |
| Recovery inspection creates no missing journal and preserves original history | `api::recovery::tests::missing_active_plan_is_503_and_visible_without_creating_or_mutating_history` |
| Successor preparation is idempotent, separate from activation, preserves budgets and keeps predecessor disclosure beyond the recent event window | `api::recovery::tests::successor_preparation_is_idempotent_separate_from_activation_and_keeps_old_budgets` |
| Busy/malformed/changed scope and incomplete seals refuse without silently replacing work | `api::recovery::tests::{concurrent_successor_preparation_or_start_refuses_without_claiming_or_clearing_stop,successor_identity_rejects_implicit_duplicate_malformed_or_changed_scope_fields,lost_budget_stale_pointer_and_incomplete_seal_never_become_prepared_work}` |
| Observed removal/replacement cannot keep publishing journal appends through an unnamed or different inode | `api::recovery_journal::tests::deleted_or_replaced_filename_refuses_append_and_preserves_the_old_index` |
| The integrated tester preserves exact selection, cancellation, bounded paging and recorded zero versus read failure | `web/tests/research-tester.test.js`; actual source/coordinate joins in `web/tests/saved-backtest-setting-pages.test.js` |
| Audit display keeps exact upper-half IDs, newest-first cursors, explicit unknown endings and bounded failure reasons | `web/tests/invocation-audit.test.js` |
| Retry preserves the failed page's exact source, period, kind and decimal offset, including offsets above JavaScript's integer precision | `retry after a page failure retains the exact nonzero offset, period, kind and source pins` in `web/tests/research-tester.test.js` |
| Historical naming requires two explicit matching grids; a missing or sparse grid cannot pass by vacuous iteration | `missing, foreign or incomplete grid provenance suppresses names instead of relabelling history` in `web/tests/research-tester.test.js` |
| A typed audit read refusal preserves its bounded reason without claiming an engine task or required write was attempted | `typed read failures preserve the storage reason without claiming a write was attempted` in `web/tests/invocation-audit.test.js` |
| HTTP routing assertions read the status line, not digits embedded in diagnostic content or a temporary path | `api::ingest::route_tests::the_three_routes_answer_and_none_of_them_shadows_the_front_end`, using a deliberate `405` fixture path |

These tests establish their named finite properties. Complete touched-crate
line/branch coverage, mutation results, dependency-rule compliance and full
production sweep admission remain separate required evidence.

## Declared Boolean launch and observation — D-0572

| Invariant | Executable proof |
|---|---|
| The launch preserves all selected families, both periods, all eight native timeframes and exact work allowances | `api::booleanlaunch::tests::declared_search_dispatch_preserves_all_families_periods_and_exact_work_allowances` |
| Malformed, extra, duplicate, null, path-bearing or noncanonical declarations never become work | `api::booleanlaunch::tests::{launch_refuses_absence_null_duplicates_unknown_paths_and_noncanonical_or_unsafe_numbers,launch_scope_never_turns_references_derivatives_alias_duplicates_or_wrong_spans_into_work}` |
| Search identity, acknowledged counts and exhaustion cannot regress or turn a work allowance into complete grammar exhaustion | `api::booleanlaunch::tests::exact_status_separates_durable_identity_paused_work_and_exhaustion`; `web/tests/boolean-launch.test.js` |
| A busy shared worker slot refuses without reserving another invocation | `api::booleanlaunch::tests::repeated_launch_cannot_bypass_the_shared_active_worker_gate` |
| Configuration reads use the production route and audit only their HTTP outcome without starting a search | `api::booleanlaunch::tests::production_metadata_route_audits_only_its_http_outcome_without_launching_work` |
| Changed displayed policy refuses before acceptance; an accepted empty private store ends with a durable refusal instead of invented results | `api::booleanlaunch::tests::explicit_configuration_accepts_the_declared_launch_and_preserves_refused_terminal_status` |
| Missing, filtered or dropped terminal telemetry refuses success while preserving the original computation evidence | `api::booleanlaunch::tests::boolean_terminal_audit_failure_refuses_success_without_erasing_saved_evidence` |
| A published search callback can read its acknowledged reservation, and final exhaustion/counts come from the saved journal | `cli::boolean_search_command::integration_tests::generated_search_recovers_same_ordinal_and_refuses_missing_ancestry` |
| Browser admission preserves native signed/boolean policy values, exact IDs and single-submission behavior; framework proxies cannot silently replace validated identities | `web/tests/boolean-launch.test.js` |
| Actual page startup and polling route Boolean jobs before ordinary outcome handling, ignore stale replies and refresh a resumed same-ID search | `web/tests/boolean-launch-page.test.js` |
| The shipped frontend accepts the native configuration serializer's signed paisa, boolean requirements and maximum u64 values without coercion | `web/sweep-readiness/check-native-launch.mjs`, against the retained native test's actual metadata output |

These are finite contract tests. A generated failure fixture is not a historical
market outcome, and a successful command is not full research or release
clearance. The dated comparison records which source revision actually ran the
tests, coverage and mutation checks.

## Backtest read recovery — D-0573

| Invariant | Executable proof |
|---|---|
| A temporary ledger overload retries the same bounded GET visibly; a successful retry still passes the ordinary envelope validator | `the actual ledger loader recovers from server backpressure with a visible, exact GET retry` in `web/tests/backtest-ledger-retry.test.js` |
| Persistent overload stops after the retry bound; a generic evidence refusal is validated once and malformed success supplies no results | `persistent busy replies stop; generic 503 evidence is validated once and malformed success is refused` in `web/tests/backtest-ledger-retry.test.js` |
| Replacing or disposing a ledger request aborts its wait and revokes late publication | `replacing a ledger read aborts its pending wait and an old completion cannot replace the new result`; `component cancellation aborts the waiting read and revokes any late reply` in `web/tests/backtest-ledger-retry.test.js` |
| A current validated status clears an obsolete startup warning; malformed and stale responses cannot establish recovery | `a recovered valid status clears the old startup warning; malformed or stale status cannot clear it` in `web/tests/boolean-launch-page.test.js` |
| HTTP success with an absent or invalid job field remains unknown and continues observation; only explicit null establishes no job | `the real status seam requires explicit no-job evidence and keeps malformed HTTP successes unknown` in `web/tests/boolean-launch-page.test.js` |

## Census transfer, page ownership and current sweep admission — D-0574 / D-0575

| Invariant | Executable proof |
|---|---|
| Warm census responses share bytes and validators; simultaneous cold encoding occurs once for an exact snapshot | `server::store_wire::tests::unchanged_sources_share_bytes_and_never_rebuild_or_rehash`; `concurrent_cold_requests_encode_once` |
| Failed or poisoned encoding is an explicit refusal; vendor and format identities remain separate | `server::store_wire::tests::failed_encoding_is_not_cached_as_an_empty_success`; `poisoned_cache_refuses_instead_of_serving_stale_bytes`; `vendor_and_format_do_not_share_validators_or_bodies` |
| Compact transfer preserves every original field and unknown reason | `server::universe_route_tests::compact_census_preserves_every_expanded_row_and_unknown_reason`; `web/tests/store-census-wire.test.js` |
| HEAD does not populate the wire cache or invent a zero representation length; exact conditional GET returns 304; corrupt census and invalid input do not become successful or retryable input | `server::tests::routed_census_head_omits_representation_length_without_encoding`; `server::universe_route_tests::census_head_does_not_encode_and_matching_get_reuses_bytes`; `compact_census_refusals_never_turn_into_not_modified_or_retryable_input` |
| Simultaneous cold census reads reuse the already-published snapshot for identical manifest stamps | `server::universe_route_tests::concurrent_cold_census_readers_publish_one_shared_snapshot` |
| Only validated 304 evidence reuses immutable prepared rows, and visible time pages use addressed native reads | `web/tests/store-census.test.js`; `database-preparation.test.js`; `database-pages.test.js`; `store-startup.test.js` |
| Feed surveys avoid full inventories; cancelled page or catalogue work cannot publish into newer selections | `web/tests/feed-summary.test.js`; `catalogue-loader.test.js`; `page-lifecycle.test.js`; `pooled.test.js` |
| Native application capabilities identify runtime availability without certifying release readiness | `server::universe_route_tests::actual_application_capability_is_explicit_and_not_release_clearance`; `web/tests/runtime-inspection.test.js` |
| A cooperating active sweep owns one validated store lease; ordinary dispatch cannot lock one store and compute into another | `cli::execution_lease::tests::a_probe_is_read_only_and_a_lease_excludes_until_drop`; `api::sweeprun::admission_tests::ordinary_dispatch_cannot_claim_one_store_and_compute_into_another` |
| Historical outcome uncertainty is retained separately from current admission, and adopted-job completion refreshes admission without another POST | `web/tests/sweep-admission.test.js`; `boolean-launch-page.test.js`; `api::sweeprun::admission_tests::historical_inspection_does_not_own_the_execution_lease_or_become_completed` |

## Native day targets and honest request deadlines — D-0576

| Invariant | Executable proof |
|---|---|
| Native civil-day targets draw inclusive monthly coverage without mutating the reported endpoints | `web/tests/autopilot-target.test.js` |
| Leap days, malformed inputs, reversed days within one month and the 600-month limit retain exact refusal behavior | `web/tests/autopilot-target.test.js` |
| A request deadline reports the observed absence of a timely answer and does not invent a locked store or accepted connection | `web/tests/ask.test.js` |

The dated page comparison records actual narrow-screen navigation inspection;
that finite browser check is not a claim of universal layout or latency coverage.

## Target-scoped Autopilot coverage — D-0577

| Invariant | Executable proof |
|---|---|
| Coverage selects the exact feed, timeframe and date window before counting distinct instrument-months | `actual coverage filters the declared feed, timeframe and window before counting distinct records` in `web/tests/autopilot-coverage.test.js` |
| Empty months remain visible even when unrelated aggregate records exceed the reported count; an instrument count cannot establish membership | `the live-shaped 141-month span keeps all 59 empty months even when global counts exceed a reported denominator`; `an excess of different stored instruments in one month cannot fill an empty month or prove target membership` in `web/tests/autopilot-coverage.test.js` |
| Identical records count once and conflicting duplicates withhold measured totals | `identical census records are counted once and conflicting duplicates withhold the actual headline totals` in `web/tests/autopilot-coverage.test.js` |
| Partial-month presence and bar counts remain separate from unknown overlap | `partial months exclude outside bars and distinguish confirmed presence from unknown overlap` in `web/tests/autopilot-coverage.test.js` |
| Missing scope, failed reads, contradictory timestamps and unsafe counts cannot publish measured coverage | `missing scope, malformed ranges, failed reads and unsafe counts cannot publish a measured target total` in `web/tests/autopilot-coverage.test.js` |

## Selected qualified-search timeframes — D-0578

| Invariant | Executable proof |
|---|---|
| Every nonempty supported subset retains its canonical physical slots and never widens to all eight | `cli::boolean_search_command::tests::selected_launch_validation_accepts_every_canonical_subset_and_rejects_invalid_scope_before_io`; `api::booleanlaunch::tests::every_nonempty_canonical_timeframe_subset_is_preserved_without_widening` |
| Excluded signal resolutions are not read and repeated execution resumes the exact selection | `generated_selected_search_reads_no_unselected_signal_rungs_and_resumes_exact_scope` in `crates/cli/src/boolean_search_integration_tests.rs` |
| Subset identity, historical all-eight bytes and omitted-result refusal remain distinct | `scoped_search_records_bind_selection_keep_legacy_bytes_and_refuse_omitted_results` in `crates/cli/src/boolean_search_record_tests.rs` |
| Nonadjacent saved slots resume without completing excluded units; summary rows preserve their original indices | `selected_nonadjacent_slots_resume_without_starting_or_completing_excluded_units` in `crates/cli/src/boolean_qualified_journal_tests.rs`; `api::booleansearchjson::tests::every_selected_summary_keeps_its_original_physical_rung_and_count` |
| Browser launch, saved research and strategy testing retain native selected scope | `web/tests/boolean-launch.test.js`; `web/tests/boolean-qualified-search.test.js`; `web/tests/qualified-campaign.test.js`; `web/tests/research-tester.test.js` |

## Boolean population bounds — D-0580

| Invariant | Executable proof |
|---|---|
| The exact conjunction lower bound cannot overflow or saturate into a small population | `cli::boolean_search_launch::sizing::tests::count_matches_integer_subsets_and_retains_large_exact_decimals` |
| The reported alphabet is the production evaluator's alphabet and its complete conjunction fits the existing expression representation | `cli::boolean_search_launch::sizing::tests::live_model_counts_the_runtime_alphabet_without_claiming_a_total_or_eta` |
| Sizing metadata cannot present a lower bound as a measured total or an elapsed-time forecast | `api::booleanlaunch::work::tests::sizing_is_a_lower_bound_and_never_a_measured_population_or_eta` |

## Index daily/weekly consistency — D-0579

| Invariant | Executable proof in `crates/cli/src/index_consistency_tests.rs` |
|---|---|
| All 1,024 five-session combinations of winning, losing, flat and no-trade days obey the declared daily ratio, weekly tests and streak limit | `every_five_day_win_loss_flat_no_trade_sequence_obeys_the_declared_policy` |
| Every nonempty pattern of absent expected sessions is missing evidence, not an invented zero day | `every_missing_position_refuses_instead_of_manufacturing_a_zero_day` |
| A three-loss streak across two otherwise passing weeks fails; flat/no-trade sessions cannot clear it | `losses_cross_week_boundaries_even_when_both_individual_weeks_pass`; `flat_and_no_trade_days_preserve_a_loss_streak_and_only_a_win_resets_it` |
| Calendar closures, exclusions and genuine weekend sessions keep their separate meanings | `long_loss_runs_are_counted_exactly_across_holidays_and_year_boundaries`; `existing_excluded_sessions_and_real_weekend_sessions_remain_distinct` |
| Short/partial weeks do not supply vacuous weekly approval, and unknown calendar facts stay unmeasured | `short_and_partial_weeks_never_pass_the_weekly_test_vacuously`; `missing_calendar_authority_stays_unmeasured_even_when_rows_are_offered` |
| Malformed order, counts, P&L, spans and overflow refuse before presenting partial work as complete | `malformed_order_counts_returns_spans_and_unexpected_sessions_refuse`; `count_and_both_paisa_overflow_directions_refuse_before_partial_day_updates` |
| Only the two canonical index families receive the policy; retaining details cannot alter its result identity | `policy_is_index_only_and_week_retention_does_not_change_identity` |
| Truncation, changed policy fields and forged passing-week claims cannot reopen as valid evidence | `exact_versioned_codecs_refuse_truncation_changed_rules_and_forged_week_claims` |

## Native signal-candle stop and complete ancestry — D-0581/D-0582

These name executable regression cases. Their existence is not a claim that
every release gate or all possible faults have been verified.

| Invariant | Executable proof |
|---|---|
| All eight signal resolutions preserve the original candle low/high and immediate next-minute entry | `runner::signal_candle_stop::tests::all_eight_rungs_keep_original_signal_low_high_and_exact_next_minute_entry` |
| Both sides permit 15:09 entry; a triggered stop takes precedence over that minute's forced close | `entry_at_1509_can_stop_or_close_in_its_own_minute_on_both_sides` |
| A gap at/beyond the stop never creates a free entry; subsequent stop gaps retain both conservative and optimistic readings | `exact_open_at_or_beyond_stop_skips_the_entry_without_a_free_gap_trade`; `shared_stop_fill_uses_open_gap_before_retrace_and_printed_extreme_for_both_sides` |
| Missing, refused or duplicated entry/path/closing bars cannot supply a nearby fabricated execution | `missing_or_refused_path_before_a_stop_preserves_occupancy_and_never_prices_later_touch`; `missing_duplicate_or_corrupt_closing_record_is_not_a_nearby_close`; `missing_immediate_entry_never_moves_to_a_later_open_and_duplicate_entry_blocks_day` |
| A signal whose would-be entry opens after 15:09 is too late whether or not that minute exists, at all eight rungs; only an absent minute at or before 15:09 is unreachable (D-0603) | `a_signal_closing_after_the_last_entry_minute_is_too_late_even_with_no_minute_there` |
| A stored event may lack an entry only as unreachable, or as too late when its close is after its own day's 15:09, so a mid-session hole cannot be filed as late (D-0603) | `only_a_data_hole_or_a_provably_late_signal_may_lack_an_entry` in `crates/cli/src/index_stop_store_tests.rs` |
| There is at most one open position per setting; later windows retain their original causal prefix | `one_position_allows_sequential_reentry_but_never_the_same_exit_minute`; `fixed_later_window_preserves_causal_column_and_rekeys_without_counting_earlier_days` |
| Unavailable index VWAP remains unknown under OR/NOT, with no invented trade | `cli::index_stop::tests::index_spot_vwap_unknown_is_not_made_true_by_or_not` |
| Candidate observations include complete long/short pairs and reconcile every event/trade/day even after malicious resealing | all six tests in `crates/cli/src/index_stop_store_tests.rs` |
| Original, later and full-span consistency are mandatory; numerical claims must reproduce from exact native parents | `crates/cli/src/index_stop_qualification_tests.rs` |
| CSCV built from per-segment totals is byte-identical to the per-period route; a row whose absolute total exceeds `i64::MAX` keeps the per-period loop (D-0604) | `cscv_segment_totals_reproduce_every_per_period_split_exactly` in `crates/cli/src/index_stop_qualification_tests.rs` |
| White, SPA and Romano-Wolf computed in one walk over the draws are the separate procedures bit for bit -- receipts, statistics, p-value bits, digests; a qualification saved by the three-pass build re-verifies (D-0606) | `runner::bootstrap::family_pass::tests::ties_zero_variance_zero_rows_and_hopeless_rows_match_the_separate_procedures`; `runner::bootstrap_zero_v2::tests::one_walk_reproduces_all_three_separate_calls_byte_for_byte`; `saved_statistics_are_the_three_separate_procedures_bit_for_bit` in `crates/cli/src/index_stop_qualification_tests.rs` |
| Neither the thread count, the chunk size nor the task split can move a count, and a generation step holds at most its index budget (D-0606) | `one_thread_and_many_threads_agree_bit_for_bit`; `neither_the_chunk_size_nor_the_task_split_moves_a_count`; `a_generation_step_holds_at_most_its_index_budget` |
| The shared walk refuses exactly the inputs, in exactly the order and with exactly the messages, of the separate calls it replaces (D-0606) | `every_refusal_is_the_one_the_separate_calls_reach_first`; `refusals_name_the_first_separate_procedure_that_refuses`; `a_shared_walk_refusal_keeps_the_message_its_separate_procedure_produced` |
| At most `lanes` timeframes run at once while nested work reaches every other pool thread; each timeframe runs exactly once and returns in declared order (D-0606) | `cli::index_stop_search::live::tests::at_most_lanes_timeframes_run_at_once_while_nested_work_reaches_the_other_threads`; `lanes_run_every_timeframe_once_and_return_them_in_declared_order`; `every_slot_is_filled_by_exactly_one_lane_or_the_batch_is_refused` |
| A saved receipt cannot enlarge the current cold reader's numerical work or estimated memory admission | `saved_limits_cannot_enlarge_current_cold_replay_work_or_memory_admission` in `crates/cli/src/index_stop_qualification_tests.rs` |
| Tighter sufficient current replay limits preserve the original procedure bounds, complete statistics, rows and completion pin after independent resource admission | `tighter_sufficient_replay_limits_preserve_original_statistics_and_completion` in `crates/cli/src/index_stop_qualification_tests.rs` |
| Checkpoint recovery compares the exact source pair, measurement windows, effective policy, resampling procedure, allocation and answer-changing limits before numerical replay | `checkpoint_expected_facts_reject_recomputed_foreign_children_before_replay`; `checkpoint_expected_sources_and_windows_reject_valid_foreign_ancestry_before_replay` in `crates/cli/src/index_stop_qualification_tests.rs` |
| Recovering an acknowledged checkpoint cannot borrow a larger replay allowance from its saved producer configuration | `checkpoint_replay_caps_cannot_be_enlarged_by_declared_producer_bounds`; `complete_native_search_reopens_all_ancestors_and_resumes_without_duplicating_the_batch` |
| Reader admission cannot be exceeded by an acknowledged checkpoint; exact retry retains the original cursor and selected slots | all six tests in `crates/cli/src/index_stop_search_checkpoint_tests.rs` |
| Generated complete stored history reaches native execution, daily/statistical evidence, durable qualification and exact batch resume | `complete_native_search_reopens_all_ancestors_and_resumes_without_duplicating_the_batch` in `crates/cli/src/index_stop_search_tests.rs` |
| Missing source or observer failure cannot create completed research | `source_and_observer_refusals_never_create_a_completed_search` |
| Both indices and all 255 nonempty timeframe subsets have exact launch scope, with no grid fields accepted | `api::indexstoplaunch::tests::every_selected_subset_and_both_indices_preserve_exact_native_scope`; `missing_ambiguous_and_grid_fields_never_become_single_stop_execution` |

Generated fixtures exercise mechanics only. They do not establish a real-market
winning strategy, complete historical source coverage or production readiness.

## Backtest single-stop controls and comparison — D-0583

| Invariant | Executable proof |
|---|---|
| One explicit Run sweep action submits one exact request; refresh, restore, hidden-page handling and uncertain responses do not resubmit | `web/tests/index-stop-launch.test.js` |
| The page preserves all 255 nonempty timeframe subsets and refuses malformed or mixed index scope | `web/tests/index-stop-launch.test.js` |
| One saved timeframe comparison opens automatically from exact acknowledged pins; switching timeframes opens no execution request | `web/tests/index-stop-launch.test.js` |
| The displayed prior context month crosses year boundaries while the selected training period remains unchanged; the date is not source-availability certification | `the required earlier context month crosses years without changing the selected training span or certifying source availability` in `web/tests/index-stop-launch.test.js` |
| The inherited reward/loss caption reads the exact effective policy, preserves overrides and unknown values, and cannot change a plan or submit a sweep | `the inherited reward/risk caption uses the exact effective threshold including overrides and u64 precision`; `an unresolved or malformed effective ratio stays unavailable instead of assuming zero or three`; `the visible caption follows resolved policy changes without mutating a plan or submitting a request` in `web/tests/index-stop-launch.test.js` |
| Native results retain exact index-point arithmetic, both fill readings and bounded pinned trade pages | `web/tests/index-stop-results.test.js` |
| A comparison keeps institutional, day/week and combined verdicts distinct and refuses malformed evidence or changed pins | `web/tests/index-stop-qualification.test.js` |

## Original source inspection and cumulative comparison — D-0585/D-0586

These references identify tests, not a claim that the release has passed its
coverage, mutation, real-source or browser gates.

| Invariant | Executable proof |
|---|---|
| Original trades and names are readable without writes; changing a selected setting reuses authenticated source state | `original_context_reads_exact_archived_trades_and_zero_trade_names_without_writes` in `crates/cli/src/index_stop_source_context_tests.rs` |
| Source plus the actual complete catalog must fit before publication; runtime bounds never fall back after invalid input | `original_context_costs_cover_actual_catalog_and_refuse_before_publication`; `original_context_budget_resolver_is_exact_and_context_binding_rejects_foreign_pins` |
| One guarded source projection holds all three distinct publication locks; success, callback failure, partial acquisition failure and attempted nested access cannot release a still-needed lock | `original_context_projection_holds_all_three_owners_until_success_and_error_callbacks_finish`; `original_context_busy_owner_prevents_projection_and_releases_earlier_leases`; `original_context_nested_reader_access_cannot_unlock_a_guarded_view` |
| Current monthly files and current vocabulary names cannot replace saved source evidence | `original_context_is_independent_of_changed_current_market_files`; `original_context_names_are_pinned_not_substituted_by_current_vocabulary` |
| Legacy absence, foreign resealed source/build, torn bytes, malformed lengths and page overflow refuse without a substitute or prefix | `original_context_legacy_missing_refuses_without_changing_existing_catalog`; `original_context_foreign_relation_and_changed_build_are_rejected_after_valid_sealing`; `original_context_partial_torn_and_changed_archives_never_supply_candles`; `original_context_independent_bounds_and_exact_window_extents_refuse_without_prefixes`; `original_context_codec_rejects_tails_lengths_versions_and_foreign_loader_before_publication` |
| Separate signal/execution roles and later measurement retain the original causal input prefix | `original_context_separate_execution_and_later_measurement_keep_original_causal_prefix` |
| V1 and V2 declarations retain exact scope and policy; malformed or nested envelopes refuse | `declaration_versions_keep_exact_scope_sources_and_preallocation_policy`; `truncation_trailing_unknown_policy_unpinned_context_and_nested_envelopes_refuse` in `crates/cli/src/index_stop_search_reader_tests.rs` |
| Old pinned prefixes survive newer pending work; acknowledged orphan checkpoints cannot silently disappear | `pinned_prefix_survives_new_pending_checkpoint_without_relabelling_or_future_reads`; `orphan_acknowledged_checkpoint_is_not_silently_removed_from_cumulative_scope` |
| Two complete families use exact shared numerical work; one-unit-short and overflow bounds refuse the complete aggregate | `two_complete_generated_families_are_parent_bound_before_cumulative_admission`; `cumulative_charge_refuses_overflow_and_never_returns_a_prefix` |
| One cumulative projection protects every contributing artifact, including families outside the displayed page; conflicting aliases, partial lock acquisition and nested access cannot release an outer lease | `compound_scope_deduplicates_aliases_without_unlocking_other_namespaces`; `compound_scope_refuses_conflicting_aliases_and_rolls_back_partial_acquisition`; `compound_projection_rejects_same_descriptor_reentry_without_releasing_outer_lock`; `compound_scope_revalidates_after_callback_and_releases_every_owner` in `crates/cli/src/boolean_observation_file_tests.rs`; `two_complete_generated_families_are_parent_bound_before_cumulative_admission` |
| Source and cumulative API requests require exact scope and bounded pins before file reads; ranking precedes filtering and pagination | `api::indexstopcandlesjson::tests::original_source_requests_are_pinned_exact_and_bounded_before_file_reads`; `api::indexstoprankingjson::tests::cross_batch_ranking_precedes_pages_and_qualified_filter_retains_total_order` |
| Source metadata, trade coordinates, original condition tokens and candle prices must match; aborted reads cannot publish stale charts or launch work | `web/tests/index-stop-source.test.js`; `web/tests/index-stop-chart.test.js` |
| Cumulative refresh failures preserve the old scope and exact details beyond the first page target the saved canonical setting | `web/tests/index-stop-ranking.test.js`; `web/tests/index-stop-qualification.test.js` |

## Original single-stop VIX annotations — D-0587

These test references do not certify real-market data or replace the native
coverage and mutation gates.

| Invariant | Executable proof |
|---|---|
| Original VIX candles retain all seven fields, both directions and the true forced-close interval | `index_stop_vix_exact_original_fields_both_directions_and_forced_interval_are_visible` in `crates/cli/src/index_stop_vix_tests.rs` |
| A hole in a validated month differs from an unavailable, corrupt or locked original month | `index_stop_vix_valid_month_holes_and_unavailable_months_are_never_conflated`; `index_stop_vix_corrupt_and_locked_reference_months_publish_explicit_unavailability` |
| Retry and cold inspection preserve the first saved snapshot; different VIX data changes only the independent reference publication | `index_stop_vix_retries_and_cold_reopen_retain_snapshot_after_current_data_changes`; `index_stop_vix_reference_changes_only_its_independent_publication` |
| Zero-trade extents, malformed pages and insufficient complete capture/read budgets cannot produce partial annotations or inspection writes | `index_stop_vix_bounds_pages_and_zero_trade_extents_refuse_without_inspection_writes`; `index_stop_vix_full_capture_budget_refuses_before_any_companion_publication` |
| Missing, torn, corrupt, foreign resealed or hidden-unavailable companions do not borrow current VIX or complete a refused native publication | `index_stop_vix_missing_torn_or_corrupt_companion_never_backfills_on_read`; `index_stop_vix_resealed_foreign_trade_and_hidden_unavailable_state_are_rejected`; `index_stop_vix_corrupt_publication_prevents_completed_native_attempt_without_rewriting_catalog` |
| One VIX page retains both publication owners throughout the projection and releases them on callback error | `index_stop_vix_compound_projection_excludes_each_writer_and_releases_on_error` |
| Repeated references to one original minute agree on the complete stamp, within a trade and across settings; cold-validation scratch is admitted first | `index_stop_vix_resealed_repeated_minutes_cannot_disagree_across_settings_or_within_a_trade`; `index_stop_vix_repeated_minute_scratch_is_admitted_before_cold_validation` |
| The exact saved reference publication remains required through the outer native catalog completion | `single_stop_retained_vix_publication_is_required_through_outer_terminal` in `crates/cli/src/index_stop_tests.rs` |
| API requests preserve exact catalog/trade pins and distinguish absence, unavailability and numeric zero; invalid pages preserve current cache admission while changed owners evict it | `api::indexstopvixjson::tests::invalid_vix_pages_keep_current_admission_but_changed_or_busy_owners_evict` |
| The browser validates original trade/month/stamp agreement, both directions and exit intervals; cancelled or stale replies cannot replace another trade or start work | `web/tests/index-stop-vix.test.js` |

## Single-stop CSCV payload admission — D-0589

| Invariant | Executable proof |
|---|---|
| A 16-period, zero-trade family still charges all 6,435 canonical masks, saved and reproduced native split payloads and both candidate score vectors before numerical replay | `sixteen_training_periods_refuse_unadmitted_cscv_payload_before_replay` in `crates/cli/src/index_stop_qualification_tests.rs` |
| The exact checked payload estimate admits production and current replay; one byte less refuses, while sufficient current limits leave the saved identity, completion, original bounds, statistics and rows unchanged | `complete_cscv_payload_exact_and_minus_one_admission_preserves_saved_evidence` |
| A resealed saved split extent or layout cannot exceed or replace the independently derived training geometry before replay; genuine no-layout evidence retains its absent state | `resealed_cscv_layout_and_extent_are_rejected_before_numeric_replay` |

These are payload-admission tests, not a process-RSS measurement. Allocator
metadata, over-allocation, thread stacks and unrelated process allocations are
outside this estimate. Stored observation-body byte admission remains separate.

## Native policy schema and launch ownership — D-0590

| Invariant | Executable proof |
|---|---|
| Browser validation requires the exact native V1 names and wire types, derived without copying threshold values; unsupported native types or mismatched field tables stop frontend generation | `the shipped browser schema exactly matches all native policy names and wire types without threshold defaults`; `unsupported native types, duplicate names and a changed or missing V1 field cannot generate a permissive browser schema` in `web/tests/native-policy-schema.test.js` |
| Missing, unknown, duplicate or wrongly typed policy fields cannot create a ready index launch plan or dispatch a request; valid complete effective overrides remain visible | `each missing, replaced, duplicate or wrongly typed policy field blocks index launch before request dispatch`; `the inherited reward/risk caption uses the exact effective threshold including overrides and u64 precision` in `web/tests/index-stop-launch.test.js`; the complete policy cases in `web/tests/boolean-launch.test.js` |
| A stale A-to-B-to-A configuration response cannot restore readiness for a newer request or outdated form values | `actual configuration A to B to A reads cannot restore stale readiness, and current form choices rebuild or refuse the exact plan` in `web/tests/index-stop-launch.test.js` |
| Repeated clicks while acknowledgement is pending cannot submit a second plan | `rapid repeated clicks cannot submit a second plan while the first acknowledgement is still pending` in `web/tests/index-stop-launch.test.js` |

Configuration validation does not attest historical OHLCV. The preview labels
that boundary explicitly; native source admission and the operator's launch
hold remain separate requirements.

## One policy, one owner — D-0600, D-0601

| Invariant | Executable proof |
|---|---|
| A complete week is classified by the policy being EVALUATED, not by a constant. A one-win four-loss week passes under V3, records no weekly reason, is counted as passing rather than failing, and its row survives the round trip; the same week fails under V1 with both weekly reasons set, and a V1 row does not decode as a valid V3 row | `a_one_win_four_loss_week_passes_under_v3_and_its_row_survives_the_round_trip` in `crates/cli/src/index_consistency_tests.rs` |
| V1 and V2 keep identical thresholds and distinct digests, so a freeze test over those two alone cannot observe a policy-dependent classifier | `the shipped policies agree on every threshold and differ only in their version word` in `crates/cli/src/index_consistency_tests.rs` |
| The consistency store addresses an artifact by the policy its evidence declares, and refuses a mixed set after opening an attempt so the refusal is audited. An EMPTY set declares no policy, therefore has no content address, therefore refuses without a row — structural, not an oversight | `a_mixed_policy_set_is_refused_and_an_empty_one_cannot_even_be_addressed` in `crates/cli/src/index_consistency_store_tests.rs` |

**Why these exist.** `Week::finish` read `Policy::V1`'s two weekly thresholds
regardless of the policy in force. It was invisible for as long as it existed
because V1 and V2 share all five numbers, so the freeze test compared two
policies that agreed; `Policy::V3` moved them and made it live. The break band
was exactly the shape V3 exists to admit, and the failure was not a wrong
verdict but a refused artifact: the classifier said Failed, the reason-finder
using V3's numbers found nothing to record, and the self-check then rejected
bytes the same call had just produced. Before this row, `Policy::V3` appeared in
zero test files and five production call sites.

## Integration refusal observability (D-0611)

| ID | Invariant | Named proof |
|---|---|---|
| CIRO-01 | A metadata configuration refusal preserves its response reason and writes the same reason through the installed production logger | `every_reachable_emit_site_puts_a_record_in_the_file` in `crates/api/src/emitted.rs` |
| CIRO-02 | Broker partial failures, interrupted baskets, recovery refusals and missing cash receipts emit their production refusal events | `a_partial_broker_basket_cannot_render_or_record_as_stored`, `an_interrupted_broker_basket_keeps_writes_but_fails_completion`, `recovery_resume_refusal_reaches_the_installed_log`, `cash_partial_month_replay_refuses_missing_or_corrupt_earlier_receipt` in `crates/api/src/server.rs` |
| CIRO-03 | Off-grid and conflicting broker candles refuse before committing bars and their refusal reaches the durable log; missing minute coverage records its stage | `every_emit_site_in_this_crate_reaches_a_file` in `crates/pull/src/emit_sites.rs` |
| CIRO-04 | Receipt drop releases its own shared lease despite a surviving duplicate descriptor, while a separate live reader continues excluding publication | `dropping_receipt_releases_lock_even_when_a_duplicate_descriptor_survives` in `crates/cli/src/checksum_receipts_tests.rs` |
| CIRO-05 | The consistency integration fixture has multiple observable days after warm-up and checks exact shares for every current grain, including hour, plus yearly count and worst daily period | `consistency_is_measured_from_real_bars_and_not_merely_wired` in `crates/cli/src/lib.rs` |
| CIRO-06 | Compression distinguishes chunk counters across the 32-bit boundary, including both nonzero words, without requiring an enormous input allocation | `compression_preserves_both_halves_of_large_chunk_counters` in `crates/core/src/blake3.rs` |
| CIRO-07 | All sixteen compression output words match the official empty-input extended hash vector, including the eight words outside the public digest | `complete_compression_output_matches_the_official_empty_vector` in `crates/core/src/blake3.rs` |
| CIRO-08 | Root output sets ROOT idempotently when an internal output is already marked, retaining the official empty digest | `root_output_keeps_an_already_present_root_flag` in `crates/core/src/blake3.rs` |
| CIRO-09 | Archive vendors retain stable namespaces but refuse invented master metadata; every vendor has an independent set bit | `archive_vendors_have_names_but_cannot_supply_master_metadata`, `a_vendor_set_is_a_set_and_every_vendor_has_its_own_bit` in `crates/core/src/vendor.rs` |
| CIRO-10 | Empty or oversized vendor identifiers cannot become requestable listings; bounded UTF-8 identifiers preserve their bytes after surrounding whitespace is removed | `vendor_ids_refuse_missing_and_oversized_values_before_a_listing_is_kept` in `crates/core/src/vendor.rs` |
| CIRO-11 | A private generated V2 replay round-trips all durable evidence, reuses without changing bytes, and refuses stale or read-only writers | `generated_replay_ledger_round_trips_reuses_and_refuses_stale_or_read_only_writers` in `crates/cli/src/global_replay_v2.rs` |
| CIRO-12 | Every V2 replay companion rejects torn headers/records and corrupt seals; exact restoration recovers the same generated replay | `generated_replay_ledger_refuses_torn_or_corrupt_records_in_every_companion` in `crates/cli/src/global_replay_v2.rs` |
| CIRO-13 | Unacknowledged V2 replay records remain invisible and append-only during recovery; completion capacity refuses before publication and expands only on explicit reopen | `generated_replay_ledger_preserves_unacknowledged_orphans_and_enforces_completion_budget` in `crates/cli/src/global_replay_v2.rs` |
| CIRO-14 | Negative decimal prices round toward positive infinity at an exact half-paisa and choose the correct neighbor on either side, including near zero | `negative_decimal_text_rounds_correctly_on_both_sides_of_the_half_paisa` in `crates/core/src/price.rs` |
| CIRO-15 | A declared master format without ISIN/series retains cash identity without inventing fields and still recognizes its index segment/name mapping | `cash_without_published_isin_or_series_keeps_identity_without_inventing_either` in `crates/core/src/vendor.rs` |
| CIRO-16 | Completed BLAKE3 chunks consume exactly the completed subtree suffix and preserve all other stack entries through the 63-level counter boundary | `chunk_folding_consumes_exactly_the_completed_subtree_suffix` in `crates/core/src/blake3.rs` |
| CIRO-17 | A corrupted stage remains refused without hiding the other five stage states; incomplete, foreign or overflowing admission/storage evidence cannot become ready | `each_corrupt_stage_is_refused_without_hiding_the_other_five_states`, `file_preflight_refuses_missing_nonfiles_and_oversize_without_reading_them`, `admission_reconciliation_refuses_foreign_incomplete_and_overflowed_evidence`, `aggregate_admission_and_stored_counts_never_wrap_into_ready_rows`, `stored_reconciliation_binds_both_identities_and_retains_exact_counts` in `crates/cli/src/step3_comparison.rs` |
| CIRO-18 | Option contract rendering admits the exact 24-byte capacity and refuses a further strike digit without truncating identity, for both sides | `contract_rendering_accepts_the_exact_capacity_and_refuses_the_next_digit` in `crates/core/src/instrument.rs` |
| CIRO-19 | The on-disk comparison reopens all sixteen generated population/admission/execution authorities and eight selections without promoting incomplete selection evidence to ready; noncanonical identity order and file bounds refuse | `complete_generated_authorities_reconcile_but_cannot_pretend_selection_is_ready` in `crates/cli/src/step3_comparison.rs` |
| CIRO-20 | A member at the full 24-byte symbol capacity retains its exact ordinal; the length guard refuses only longer input | `a_position_probe_keeps_a_member_at_the_exact_symbol_capacity` in `crates/core/src/universe.rs` |
| CIRO-21 | A valid, durable V3 selection cannot replace an absent requested V4 selection; restoring the current bytes recovers their prior blocked state | `a_valid_older_selection_cannot_replace_the_requested_current_format` in `crates/cli/src/step3_comparison.rs` |
| CIRO-22 | Mutation matrices partition the complete enumerated diff without sampling; every assigned case has exactly one caught or unviable outcome, and survivors, timeouts, missing, duplicate or foreign outcomes refuse | `large_plan_partitions_every_case_once_without_sampling`, `wrong_missing_reordered_and_foreign_assignments_refuse`, `only_complete_disjoint_caught_and_unviable_results_pass`, `capacity_and_empty_matrix_are_explicit`, `malformed_or_duplicate_lists_are_not_silently_counted` in `.github/mutation_gate.rs` |
| CIRO-23 | Every single-byte bit flip in a committed V5 winner or completion is rejected even after its outer checksum is recomputed; exact byte restoration recovers the same receipt | `every_record_byte_is_checked_even_when_the_outer_seal_is_recomputed` in `crates/cli/src/selection_v5.rs` |
| CIRO-24 | Test scratch cleanup removes only owned entries strictly older than one hour, preserves foreign, recent, future and non-UTF-8 names, and creates nothing for an absent root | `scratch_cleanup_uses_explicit_age_and_preserves_foreign_or_recent_entries`, `scratch_cleanup_does_not_interpret_non_utf8_names_as_owned_entries` in `crates/telemetry/src/lib.rs` |
| CIRO-25 | Operator argument failures precede store access, support retains an extinction floor, calibrated caps keep their measured prefix, and reports retain cost, validation and refusal qualifications | `malformed_stored_command_numbers_refuse_before_any_store_access`, `support_is_scaled_in_ppm_and_cannot_disable_extinction`, `calibrated_caps_keep_measured_work_and_never_exceed_available_candidates`, `validation_disclosures_keep_complete_sections_and_their_qualifications`, `retention_disclosure_preserves_the_reason_no_candidate_traded`, `command_reports_preserve_the_failure_and_the_requested_result` in `crates/cli/src/operator_boundary_tests.rs` |
| CIRO-26 | Selection V3 and V4 reject every single-byte bit flip even with a recomputed outer seal, for empty, partial and full winner sets; Execution V4 binds every byte in each companion through the whole ledger, and exact restoration recovers its receipt | `every_selection_v3_byte_is_bound_even_after_the_outer_seal_is_recomputed` in `crates/cli/src/selection_v3.rs`; `every_selection_v4_byte_is_bound_even_after_the_outer_seal_is_recomputed` in `crates/cli/src/selection_v4.rs`; `every_execution_v4_companion_byte_is_bound_after_outer_resealing` in `crates/cli/src/execution_v4.rs` |
| CIRO-27 | Saved single-stop pages retain exact paired directions, completion pins and child links, disclose excluded costs and unassessed admission, refuse stale/corrupt evidence or out-of-range pages, and malformed HTTP requests return a structured refusal before store access | `saved_stop_pages_keep_exact_links_pins_and_unassessed_policy`, `malformed_stop_requests_return_a_refusal_without_touching_the_store` in `crates/api/src/indexstop_projection_tests.rs` |
| CIRO-28 | Global Replay V2 rejects every one-bit byte change in each companion's header and first record even after a payload's outer seal is recomputed; restoring the exact files recovers the same completion IDs and reconstructed replay | `every_global_replay_v2_companion_byte_is_bound_after_outer_resealing` in `crates/cli/src/global_replay_v2.rs` |
| CIRO-29 | Fold ladder construction rescales support while preserving the parent's candidate, pair and support-worker limits, including zero and oversized requested worker bounds | `folds_preserve_parent_worker_candidate_and_pair_limits` in `crates/runner/src/validate.rs` |
| CIRO-30 | Global run reservations are absent without a sink and distinct when installed, explicitly correlated events do not change the ambient run, and a renamed log descriptor can still sync after reopening fails | `installing_twice_is_refused_by_name_and_the_first_one_keeps_working` in `crates/telemetry/src/lib.rs`; `emitting_with_no_sink_installed_is_reported_and_is_not_a_panic` in `crates/telemetry/tests/global_absent.rs`; `a_roll_that_renames_and_cannot_reopen_writes_into_the_file_it_rolled` in `crates/telemetry/src/sink.rs` |
| CIRO-31 | Native and coarse audited stored-month publication records exactly one unpriced empty result at maximal support, exact reruns preserve ledger bytes, stale sources cannot publish, and exact restoration plus fresh authentication reuses the parent | `audited_month_publication_is_idempotent_and_stale_inputs_cannot_publish` in `crates/cli/src/audited_stored_tests.rs` |
| CIRO-32 | Calendar HTTP reads newly published census identities after startup, separates feeds and cash/index paths, and refuses an ambiguous symbol instead of choosing one stored identity | `calendar_reads_current_feed_and_cash_identity_after_startup`, `calendar_refuses_a_symbol_held_under_multiple_identities` in `crates/api/src/calendar_route_tests.rs` |
| CIRO-33 | The complete generated stored Candidate/Observation/Statistics/Admission/Population/Execution chain reaches the real Selection V5 commit, retains exact source identities and eligible counts, rejects a changed completion, and reuses identical bytes on retry; measured ranking values must equal direct metrics at every admission verdict and an admitted row cannot lack them | `stored_pair_commits_observation_and_statistics_then_reuses_exact_bytes` in `crates/cli/src/step3_orchestrator.rs`; `measured_ranking_evidence_cannot_disagree_with_direct_metrics_at_any_verdict` in `crates/cli/src/selection_v5.rs` |
| CIRO-34 | Verification never deletes a pre-existing scratch directory, simultaneous ledger self-checks own distinct paths, sixteen name collisions refuse without deleting any owner and a later free name works; all six malformed operator requests refuse without a provenance banner for each broker feed | `ledger_self_checks_do_not_delete_an_existing_directory_and_can_run_concurrently`, `operator_self_check_rejects_all_malformed_requests_before_claiming_provenance` in `crates/cli/src/operator_boundary_tests.rs` |
| CIRO-35 | A failed fold cannot agree with an empty derived file; stored audits name every field and overflow count, read all seven derived rungs without changing bytes, keep later verdicts after a coarse-file refusal, and require readable minute authority | `malformed_minutes_cannot_agree_with_an_empty_derived_file`, `every_stored_field_disagreement_retains_its_exact_position_and_values`, `disagreements_past_the_cap_are_counted_and_still_refuse` in `crates/cli/src/fold_audit.rs`; `stored_audit_reads_all_seven_derived_rungs_without_changing_any_source`, `unreadable_derived_rungs_are_named_and_do_not_hide_later_verdicts`, `missing_or_corrupt_minute_authority_refuses_the_entire_audit` in `crates/cli/src/fold_audit_io_tests.rs` |
| CIRO-36 | Stored batch retries retain the exact parent identity and bytes while recording distinct attempts, other feeds remain excluded, missing context refuses before publication, and a failed result write leaves durable Refused evidence before a repaired retry can complete | `stored_batch_publication_reconciles_exact_retries_and_keeps_other_feeds_out`, `batch_missing_required_context_refuses_before_creating_a_parent_or_attempt`, `batch_publication_failure_is_durable_refusal_and_a_repaired_retry_can_complete` in `crates/cli/src/batch_stored_tests.rs` |
| CIRO-37 | All-rung retained handles occupy at most 64 KiB each; the actual generated eight-rung stored transaction publishes Population, Execution and Selection, authenticates topology, reuses identical publication bytes on retry, and refuses a changed Selection completion | `all_rung_retained_handles_stay_below_one_sixty_four_kib_stack_budget`, `all_eight_stored_rungs_publish_exact_selection_chains_and_reuse_every_byte` in `crates/cli/src/step3_all_rung_tests.rs` |
| CIRO-38 | Native and coarse monthly audits preserve a completed empty search, exact saved identity and parent bytes across distinct retry attempts; changed minute authority cannot change that parent, and missing execution or prior context refuses before any parent or attempt | `monthly_audits_publish_empty_extinction_and_exact_retry_identity_at_both_resolutions`, `monthly_audit_missing_execution_or_prior_context_refuses_before_an_attempt` in `crates/cli/src/audited_stored_tests.rs` |
| CIRO-39 | Stored native and coarse range audits retain requested and found month counts, name an absent month and the shorter sample, publish a completed empty result, and reuse exact parent bytes across distinct completed attempts | `range_audits_record_exact_requested_and_found_months_without_inventing_missing_bars` in `crates/cli/src/audited_stored_tests.rs` |
| CIRO-40 | The fold-audit command returns failure for any unreadable requested minute month or derived rung as well as disagreement, keeps all later verdicts and bounded disagreement counts, and succeeds only after exact source restoration | `fold_command_fails_on_unreadable_authority_and_retains_every_later_verdict` in `crates/cli/src/fold_audit_io_tests.rs` |
| CIRO-41 | Saved VIX projections authenticate exact, absent and unavailable companions against their original native catalog, retain full integer/reference provenance and bounded pages, refuse changed evidence or foreign pins, and recover by cold authentication of restored bytes | `saved_vix_pages_authenticate_exact_absent_and_unavailable_companions_and_recover_cold`, `malformed_reference_request_has_a_json_refusal_before_any_store_access` in `crates/api/src/indexstopvix_projection_tests.rs` |
| CIRO-42 | Operator verification fails partial requested history, incomplete repeatability runs and empty causal-prefix comparisons; it requires a later suffix, refuses missing/unknown feeds, never prints complete success without evidence and never changes source bytes; observable completed sweeps and real suffix agreement can pass | `stored_self_check_reports_partial_history_and_missing_feed_as_failures`, `series_self_checks_require_observations_and_a_real_suffix` in `crates/cli/src/operator_boundary_tests.rs` |

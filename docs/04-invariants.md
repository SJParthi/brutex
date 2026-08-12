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

## Vocabulary and indicators

| # | Must hold | Proven by | |
|---|---|---|---|
| V-01 | Condition bit indices are stable across releases | `vocab::table::the_table_is_a_contiguous_run_of_indices`, `vocab::table::a_tombstone_keeps_its_index_and_always_evaluates_false` | Previously named a golden `bit_table_frozen` test that existed in no file. Gate 10 skipped the row while `crates/vocab` was not a workspace member and went red the moment it became one — D-0080. The old name is deliberately not written in `crate::module::name` form here, because gate 10 reads that shape as a claim and cannot tell a citation from a history note. |
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
| E-04 | **No depth parameter exists on any public sweep entry point** — `size_of::<Ladder>() == size_of::<u64>()`, so there is nowhere for a depth to hide | `engine::tests::the_type_carries_no_depth_field` | ✓ |
| E-05 | The ladder terminates when a level produces no frequent candidate, and the empty level is recorded rather than hidden | `engine::tests::depth_is_reached_by_extinction_and_not_by_a_caller` | ✓ |
| E-06 | A duplicate candidate is evaluated once however many pairs produce it, and the rejection is counted rather than silent | `engine::tests::one_k_set_is_evaluated_once_however_many_pairs_produce_it` | ✓ |
| E-07 | A rerun with identical inputs produces byte-identical output — two independent walks rendered to text and compared as text, which is what §3 rule 5's "byte for byte" means for a result a file will hold | `engine::tests::a_rerun_with_identical_inputs_produces_an_identical_sweep` | ✓ |
| E-08 | Peak memory stays under the declared budget at the declared candidate count | **UNMEASURED** — see `docs/06-limits.md`. No budget has been declared and no process-memory measurement exists, so there is nothing to prove yet; the row is kept rather than deleted because the invariant is real | — |

## Complexity — gate 8

| # | Must hold | Proven by | |
|---|---|---|---|
| C-01 | Reading the header costs the same at 1×, 10× and 100× the region offered | `store::bench::header_read_is_flat` | ✓ |
| C-02 | Per-candidate evaluation cost does not grow with what the candidate requires — k=1 to k=8, measured at 1.117× | `engine::bench::support_costs_the_same_per_bar_at_every_depth` | ✓ |
| C-03 | Duplicate-rejection cost does not drive the per-bar cost up as the column grows — folded into the whole-ladder measurement rather than isolated. **Narrowed:** the seen-set size is not varied independently, so this row proves dedup does not make a walk superlinear, and does NOT isolate a probe's own cost | `engine::bench::a_ladder_walk_costs_the_same_per_bar_at_every_column_length` | ✓ |
| C-04 | Result append cost is flat from 1× to 100× results held | **UNMEASURED** — see `docs/06-limits.md`. The engine appends to a `Vec`, whose amortised push is O(1) by construction rather than by measurement here, and no bench varies the results-held count independently | — |
| C-07 | Sealing one block costs the same at 1×, 10× and 100× the file's record count | `store::bench::block_seal_is_flat` | ✓ |
| C-08 | The block checksum beats the bit-by-bit kernel it replaced by at least 3×, measured in the same process | `store::bench::checksum_beats_the_bit_loop` | ✓ |
| C-09 | Decoding one vendor row costs the same whether a field is 28 bytes or 4 MiB | `core::bench::decode_is_flat_in_field_width` | ✓ |
| C-10 | An over-wide vendor field is **refused**, not merely decoded quickly | `core::bench::an_over_wide_row_is_refused` | ✓ |
| C-11 | Reading the census beats re-deriving it from the entries by at least 100×, measured in one process | `pull::bench::census_beats_the_scan_it_replaces` | ✓ |
| C-12 | One entry lookup — hit or miss — costs the same at 1×, 10× and 100× the census, measured on the map a **loaded** manifest holds | `pull::bench::entry_lookup_is_flat` | ✓ |
| C-13 | Appending a month to a loaded census costs the same at 1×, 8× and 32× the census — measured at the counts `7·2^k` where a table reserved to exactly the census has **no free slot at all**, not only at round numbers | `pull::bench::append_after_load_is_flat` | ✓ |
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
| P-02 | A bar outside the requested window is never stored | `pull::integration::a_bar_outside_the_window_or_the_session_is_never_stored` · `pull::integration::a_narrower_window_stores_strictly_fewer_bars_and_says_why` — **"never stored" is now checkable**, because `pull::ingest::from_dir` takes a vendor's folder all the way to an append. Both tests reopen the month afterwards and read **every** committed record back, asserting each one is inside the operator's window and inside the exchange's session, from a fixture carrying a row on each side of every boundary — 15:29:59 in, 15:30:00 out. The narrower window stores strictly fewer bars off the same bytes, so the filter is keyed on the request rather than on the file. The window *arithmetic* remains `pull::unit::a_window_is_inclusive_at_both_ends_and_refuses_to_run_backwards`, `pull::unit::every_second_of_a_day_falls_on_exactly_one_side_of_the_session` and `pull::unit::an_inclusive_window_survives_the_vendors_exclusive_to_date`. One member, one month, one instrument | ✓ |
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
| P-22 | A throttle halves the allowance of **every** span and drains every bucket, and leaves every published ceiling untouched | `pull::unit::a_refusal_halves_every_allowance_and_drains_every_bucket` | ✓ |
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

## The rung — which bar length, and where it lands

Added by D-0055. `Timeframe::DAY_1` existed in the store from D-0054 and nothing
could reach it. These rows are about the one field that decides three things at
once — the store directory, the fold bucket, and whether the session filter
applies — and about the two vendor facts nobody has written down.

| # | Must hold | Proven by | |
|---|---|---|---|
| G-01 | `Granularity::Day1` carries `Timeframe::DAY_1`, a daily pull lands under `1day/`, and **nothing** is written under `1min/` for it | `pull::broker::a_daily_pull_lands_under_the_day_directory_and_folds_the_session_into_one_bar` — the whole broker path over a real socket, with the negative half asserted as well as the positive: the bar length is the PATH in this store, so a daily bar filed under `1min/` produces a well-formed file with a valid checksum and an accurate counter that no later reader can tell from real minute data. The same test asserts the FOLD, because the same field decides it — four one-minute bars in one session become one bar whose open is the first, high the maximum, low the minimum, close the last and volume the sum | ✓ |
| G-02 | Exactly two rungs carry a store timeframe, and each agrees with the store on its directory name **and** on its kind | `pull::vendor::the_minute_and_day_rungs_carry_a_store_timeframe_and_the_rest_refuse` — plus three `const` assertions beside `store_timeframe` that make a rename a compile error rather than a misfiling. The day rung's seconds cannot be tied the way the minute's are: `Grid::Daily` carries no interval, so what is pinned is the name, that the ladder calls the rung an aggregate, and that `DAY_1.secs()` is a whole number of `MINUTE_1` bars | ✓ |
| G-03 | A rung the store cannot carry is **refused by name at the write boundary**, never substituted, and nothing reaches the disk | `pull::broker::a_rung_the_store_cannot_carry_is_refused_and_the_refusal_names_it` — `Granularity::Second1` is the reachable case: both archive feeds serve it and `crates/store` has no directory for it. The refusal names the rung it refused and the rungs this build does store, one failure for the run rather than one per member, and `bars/` is never created | ✓ |
| G-04 | The window cap is per (feed, rung): a daily window is split by the **month** and never by the one-minute cap | `api::server::a_daily_window_is_split_by_the_month_and_not_by_the_one_minute_cap` — 2020-01-01..2026-08-07 per instrument is 126 requests at Groww's one-minute cap and 80 at the day rung; Dhan is 80 at both, because its 90-day cap is already wider than any month. The equality is asserted for Dhan rather than elided, so a `<=` that let the minute cap back in on Groww cannot pass | ✓ |
| G-05 | No chunk ever spans two months, **at every cap including none** | `pull::session::no_chunk_ever_spans_two_months` — the `None` row is the one the month clamp exists for on its own: a rung no vendor published a cap for has nothing else holding its chunks inside one file, and `fetch::land` refuses a batch spanning two. 699 members failed that way on a real 37-day pull | ✓ |
| G-06 | The rung goes on the wire in the feed's **own** word, and a rung with no recorded word refuses before the socket | `pull::http::the_rung_is_named_on_the_wire_and_an_unrecorded_one_refuses_before_the_socket` — `candle_interval=1minute` reaches the vendor, which is not `1min`, the store's spelling for the same rung. The daily word is refused by name with UNVERIFIED in the message and nothing is sent, asserted by the listener never receiving a second request | ✓ |
| G-07 | No descriptor carries a day-level window cap or a daily interval word, and a rung field and a rung table exist together or not at all | `pull::vendor::a_rung_on_the_wire_needs_a_word_and_the_unrecorded_ones_stay_absent` — **the absence is the assertion.** `docs/00-charter.md` §4 records a one-minute cap for both brokers and a day-level one for neither, and records no daily interval spelling; the failure mode this row guards is somebody filling one in from memory | ✓ |
| G-08 | The form offers exactly the rungs some feed declares **and** the store can file — no more, no fewer | `api::render::the_spot_form_offers_every_storable_rung_its_feeds_declare_and_no_other` — walked over the whole ladder rather than spot-checked, so `1s` (served by both archive feeds, unfilable by `crates/store`) is asserted absent. A control that can only ever refuse is what the descriptor table's own empty-set `const` assertion already forbids | ✓ |
| G-09 | An unstated rung is the minute, and a rung this ladder does not have is refused naming what arrived | `api::ingest::a_spot_request_names_its_bar_length_defaults_to_the_minute_or_is_refused` — the default is the load-bearing half: `/pull/spot` is a replayable POST, and a body written before the field existed that silently changed rung would file one window into two directories the append-only store cannot then reconcile | ✓ |

**What these rows do NOT claim.** That a daily bar's timestamp is the session
open. `pull::fold` buckets on `ts_micros.div_euclid(width) * width`, which is
UTC-epoch aligned, so an NSE session — 03:45 to 10:00 UTC, and 12:45 to 13:45
UTC for the Muhurat sessions `docs/00-charter.md` §3 records — falls inside one
UTC day and yields one bucket. The resulting bar is stamped **00:00:00 UTC,
which is 05:30 IST**, not 09:15. That is correct-by-construction only for venues
whose session does not cross a UTC midnight, and this surface has no other.
`pull::broker::a_daily_pull_lands_under_the_day_directory_and_folds_the_session_into_one_bar`
is what pins the one bucket; nothing here pins the stamp to a session boundary,
because it is not one.

## Cross-cutting

| # | Must hold | Proven by | |
|---|---|---|---|
| X-01 | Run identity changes if any loaded bar differs by one field | **NOT PROVEN, AND THERE IS NOTHING TO PROVE IT AGAINST.** `core::proptest::identity_sensitivity` exists in no file; no `proptest` dependency exists. There is no run-identity function either: `blake3` sits in the workspace dependency table and **no member takes it**, and no crate holds a function that hashes `CLAUDE.md` §3 rule 3's nine inputs. So this row has neither the test nor the code. **The subject is further away than the hash:** five of those nine inputs have no source at all, because `crates/vocab`, `crates/indicators` and `crates/engine` are not workspace members — there is no mask, no `vocab_version` and no sweep to identify. The name stays in the backticks, and D-0062 put `X-01` in gate 10's `allow_pending` with that reason, so the row reads PENDING rather than red. What closes it: those crates, then a `data_digest` over the loaded bars, then the identity function, then the test — and **not** by adding `proptest`, which this workspace has twice declined in favour of exhaustive ordinary `#[test]`s (S-02 walks every index; P-01 uses eight real threads) | ✗ |
| X-02 | Prices never touch a float on any path from wire to store to result | **THE ROW WAS RIGHT THAT THE LINT WAS A `warn`, AND THAT IS NOW FIXED RATHER THAN DESCRIBED.** D-0061 tightened `float_arithmetic` from `"warn"` to `"deny"` in `Cargo.toml`; nothing broke, because CI already passed `-D warnings` and every float in this workspace was already either the one boundary conversion or a statistical value. What the `warn` cost was honesty: six comments across `crates/api`, `crates/costs`, `crates/pull` and `crates/store` told a reader the lint was denied, and one flag stood between that and false. `core::lint::no_float_in_price` now exists and reads three facts off the source — that the workspace lint table **denies** the lint, that exactly one *item-level* `#[allow]` overrides it, and that it sits on `Paisa::from_rupees_half_up` — then walks every other module of `crates/core` and requires each float it finds to be handed to that conversion within two lines. `crates/core/src/vendor.rs`'s `parse_strike` is the one that is, and it does no arithmetic. `core::lint::the_module_list_is_the_whole_crate` checks the scanned list against `lib.rs`'s own `pub mod` lines, so a new module is a failing test rather than a silently unscanned file. **It is a source check and says so in its own header: it covers `crates/core` and no other crate**, because a Rust test cannot walk a tree without depending on the directory it ran from. `crates/greeks`'s four module-wide allows are not a second price path — a delta of `0.00017142680429549402` is a statistical value and `CLAUDE.md` §7 keeps those at full precision, so the line is between a price and a statistic rather than between an integer and a float. The type half is still X-11's | ◐ |
| X-03 | Every tracked file has an allowed extension | CI gate 1 | ✓ |
| X-04 | No build script invokes an external process | CI gate 2 | ✓ |
| X-05 | `web` depends on `core` alone | CI gate 7 | ✓ |
| X-06 | **Line and region** coverage is 100% on every crate, with no omit list | **THE GATE EXITS 1 ON THIS TREE.** Measured by running the CI command itself — `cargo llvm-cov --workspace --locked --fail-under-lines 100 --fail-under-regions 100 --summary-only`, cargo-llvm-cov 0.8.4, 2026-08-07 at commit `79c5e80`: **exit 1**, TOTAL **96.75% regions** (851 of 26,169 missed), **96.24% lines** (592 of 15,734), 93.88% functions (94 of 1,537). Eight files are short, and the table below names every one | ✗ |
| X-06b | ~~Branch coverage is 100% on every crate~~ | **NOT MEASURED.** `llvm-cov` instruments zero branches on the pinned stable toolchain and `--branch` cannot run there at all. Narrowed by D-0030; recorded in `docs/06-limits.md` §7. | — |
| X-07 | No mutant survives on a touched module | `cargo-mutants`, run per change. `crates/pull`, D-0036: **263 mutants, 227 caught, 36 unviable, 0 survivors**. `crates/costs`, D-0044: **163 mutants over `trip.rs`, `money.rs`, `fill.rs`, `scope.rs`, `error.rs` — 106 caught, 57 unviable, 0 survivors**, and one mutant that *did* survive a first run is written up in `docs/06-limits.md` §27. **Never measured at all: `crates/core`, `crates/store`, `crates/api`**, and the nine `crates/costs` files outside that list. `crates/store` planted five mutants by hand, which §22 records is not a survey. There is no `cargo-mutants` step in CI, so nothing enforces this row | ◐ |
| X-08 | No tracked file contains a **slash-joined** credential path whose environment segment is a well-known one | CI gate 1c | ✓ |
| X-08b | No literal under `crates/pull` that could be a path segment is undeclared | CI gate 1d | ✓ |
| X-09 | `core` declares no dependency at all | CI gate 9 | ✓ |
| X-10 | Every reachable row in this file names a test that exists | **THE GATE EXITS 0 ON THIS TREE, AND THIS ROW IS `◐` RATHER THAN `✓` BECAUSE THREE ROWS ARE EXEMPTED RATHER THAN PROVEN.** Measured by running gate 10's own script at this commit: **611 rows read, 568 named tests checked against a tracked crate, 1 skipped for a crate that is not a workspace member, 7 exempted by the allowlist, 0 missing.** The first two figures move whenever any row is added and are a reading, not a bound; the last three are the ones this row is about — and one of *those* had gone stale, which is worth recording. It read **17 skipped** and now reads **1**, because that 17 was `crates/vocab`, `crates/indicators` and `crates/engine` being treated as non-members. All three are listed in the root `Cargo.toml`, so their rows are now CHECKED rather than waved past. The remaining 1 is V-01's literal placeholder `crate::module::name`, which the gate reads as a crate called `crate`. The same false premise was written into gate 10's own X-01 exemption and is corrected there. It read **10 missing**, then **4**, then **5** when A-20 joined them, across three earlier passes. Two of the five were written — A-20 by D-0060 and X-02 by D-0061 — and three were allowlisted by D-0062: P-03, X-01, X-13, each because the code the row describes does not exist and a test would therefore assert an absence. The 7 exempted tokens come from those 3 rows; a row naming a test twice is counted twice, and P-03's second name is a test that exists and passes. **A tick bought by an allowlist is not a tick earned**, which is the same argument X-06 makes in the other direction, so the glyph names where this has been proven — every row but three — rather than claiming all of them | ◐ |
| X-12 | Each vendor writes only under its own path prefix; no vendor can overwrite another | `store::unit::vendor_prefix_isolated` — the test **exists and passes**, and this row wore `—` anyway. It proves the claim *lexically*: the first segment is a `Vendor` rather than a string, and no segment can hold a separator. It does not touch a filesystem | ✓ |
| X-13 | A bar-for-bar mismatch between two vendors refuses the window and names the timestamp | **NOT PROVEN, AND THE REASON HAS CHANGED.** `store::unit::vendor_disagreement_refuses` exists in no file. `crates/store` **does** have a bar reader now — `BarFile::read_record`, walked at every index by S-02 — so the missing piece is no longer the reader. What is missing is the comparison: nothing in this repository opens two vendors' months and matches them bar for bar, so there is no code for a test to drive. **This row was checked against the product direction before it was left standing, and it survives that check.** `docs/07-plan.md` R-6 — "no vendor comparison anywhere" — is about DISPLAY, and says so in its own enforcement column: a feed picker on `/store` and one row column. X-13 is an ingest-time refusal, and the same repository already ships a cross-vendor validation that refuses one level up — `api::merge::a_cross_vendor_isin_conflict_is_reported_and_neither_side_is_dropped`, whose own assertion reads "a named disagreement REFUSES the universe". Comparing two vendors to validate is established practice here; showing two vendors to an operator is what R-6 forbids, and abandoning this row on R-6 would conflate them and discard a real safeguard. The name stays in the backticks, and D-0062 put `X-13` in gate 10's `allow_pending` so it reads PENDING rather than red. What closes it: a two-`BarFile` comparison in `crates/store` for one (exchange, segment, symbol, timeframe, month) that refuses and names the first divergent timestamp, then `store::unit::vendor_disagreement_refuses` | ✗ |
| X-11 | A price is constructible from a float only through the one checked conversion | `core::price::refuses_an_out_of_range_price_instead_of_saturating` (private field; no other path exists) | ✓ |

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
| K-25 | The offset rounds **half toward zero**, so `moneyness == 0` is exactly the at-the-money band — proven against an independent re-derivation of the source's own classify rule over every paisa of two whole steps, on both grids and both sides | `costs::moneyness::the_bucket_is_the_sign_of_the_moneyness_and_the_band_edge_agrees` · `costs::moneyness::a_tie_lands_at_the_money_and_one_paisa_past_it_does_not_on_either_side` | ✓ |
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

What must not happen is a per-bar **constant that drifts**. That is how a linear
pass becomes superlinear with nothing in the source resembling a nested loop, and
it is what these rows measure. Every one divides by the bar count and compares the
quotient, in **both directions** — a variant that is cheaper is as much a data
dependence as one that is dearer.

| # | Must hold | Proven by | |
|---|---|---|---|
| C-E-01 | The per-bar cost of support counting does not grow with the column: 10,000 against 100,000 against 1,000,000 bars | `engine::bench::support_costs_the_same_per_bar_at_every_column_length` | ✓ |
| C-E-02 | The per-bar cost does not grow with what the combination REQUIRES: k=1 against k=4 against k=8. **§6's absent depth parameter depends on this row** — if a deep combination cost more per bar, the ladder's total work would be quadratic in depth and the missing parameter would be a performance defect rather than a design decision | `engine::bench::support_costs_the_same_per_bar_at_every_depth` | ✓ |
| C-E-03 | The per-bar cost does not depend on the ANSWER: a column where every bar matches against one where none does, verified to be those two extremes before being timed. `support` folds a branchless `hits` with no early exit, so a short-circuiting version would be fast on selective candidates and the sweep's runtime would track the market rather than the bar count | `engine::bench::support_costs_the_same_whether_bars_match_or_not` | ✓ |
| C-E-04 | A whole ladder walk — generate, subset-prune, reject duplicates, count support — costs the same per bar at 10,000 and 100,000 bars, with the live set held fixed so only the bar count varies and the O(&#124;frontier&#124;²) join is not what is being measured. Every frontier is checked to reconcile before its cost is reported: a walk that is not sound is not worth timing | `engine::bench::a_ladder_walk_costs_the_same_per_bar_at_every_column_length` | ✓ |

Measured on the operator's machine, 2026-08-11, `cargo bench -p engine`, exit 0.
`size_of::<Ladder>()` is **8 bytes** — one `u64`, with no room for a depth field.
**Four measurement sites producing six points**: C-E-01 and C-E-02 each loop over
their variants, so adding a column length or a depth adds a point without adding a
line. Gate 14's floor is stated as 4 because that is what its own counter sees.

| Point | Ratio |
|---|---|
| C-E-01 10,000 → 100,000 bars | 1.002× |
| C-E-01 10,000 → 1,000,000 bars | 0.574× (cheaper) |
| C-E-02 k=1 → k=4 | 1.118× |
| C-E-02 k=1 → k=8 | 1.117× |
| C-E-03 all match → none match | 0.998× |
| C-E-04 walk, 10,000 → 100,000 bars | 0.842× |

**C-E-02 is the row §6 rests on.** 1.117× from k=1 to k=8 is `hits` being `WORDS`
word operations whatever the popcount: requiring eight conditions costs what
requiring one costs. The ladder can therefore walk to extinction without a depth
parameter being a cost decision.

**C-E-03 at 0.998× is the no-early-exit measurement**, and the two columns are
verified to be the extremes — 100,000 of 100,000 matches against 0 of 100,000 —
before either is timed. A column that quietly failed to be an extreme would print
a reassuring number and mean nothing.

**C-E-01's 0.574× at a million bars is the sweep getting CHEAPER per bar, not
dearer**, which is the branch predictor and the prefetcher warming over a longer
run. It is reported as `(cheaper)` rather than silently passing a one-sided
check. The honest reading is that the per-bar constant does not drift upward; the
downward movement is the machine, not the algorithm.

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
| I-01 | A flat bar (`close == open`) is **neither** direction, so it sets neither 37 `prior_n_bullish` nor 38 `prior_n_bearish` and it breaks 39's alternation. The ring stored `close > open`, which filed a flat bar as `false` = bearish, so 38 asserted three bearish bars over a run in which 31 `bar_bearish` was never once set | `indicators::session::a_run_of_flat_bars_is_neither_a_bullish_nor_a_bearish_run`, `indicators::session::a_flat_bar_between_two_up_bars_is_not_an_alternation` | ✓ |
| I-02 | 39 `prior_alternating` stays **clear** on a monotone run. Nothing pinned this: replacing the `differing` test with `true` passed all 22 session tests, so 39 could fire on three consecutive up bars — the same defect as I-01 in the opposite direction | `indicators::session::a_run_of_three_up_bars_sets_prior_n_bullish_on_the_fourth`, `indicators::session::a_run_of_three_down_bars_sets_prior_n_bearish_on_the_fourth` | ✓ |
| I-03 | A period speaks only on the candle its own period completes. Bit 2 `close_above_ema200` fired on the **second** candle of a run, where the "200-period average" was one candle old — `docs/03-vocabulary.md` §4 says a bit that cannot be evaluated evaluates false, never "probably". The EMISSION is gated on a folded counter, not the accessors, because `Atr::value()` returning `None` stops `SuperTrend::fold` seeding at all | `indicators::trend::the_second_candle_of_a_run_names_no_period_at_all`, `indicators::trend::each_position_speaks_on_the_candle_its_own_period_completes` | ✓ |
| I-04 | A pivot ladder whose rung leaves `i64` **refuses**, and never clamps. Clamping collapsed R4 onto R5 — two vocabulary positions becoming one predicate — and set six positions measured against levels that do not exist | `indicators::daily::a_rung_past_the_type_refuses_the_whole_ladder` | ✓ |
| I-05 | A session whose intermediate `high + low + close` needs `i128` still yields **ten distinct rungs**. The i128 widening had no guard at all after I-04's fix deleted the test that pinned the near-edge arithmetic; dropping the widening now makes the pivot come back as −3,074,457,345,618,258,605 | `indicators::daily::a_session_whose_sum_needs_the_widening_still_yields_distinct_rungs` | ✓ |
| I-06 | A band base that leaves `i64` **abstains by name** rather than saturating, at all **four** sites. A saturated span made the band up to 2× too narrow, and a close inside the missing half set **no bit while nothing refused** | `indicators::orb::a_window_whose_span_leaves_the_type_sets_nothing`, `indicators::fib::a_five_session_span_that_leaves_the_type_sets_nothing`, `indicators::gap::a_leg_whose_length_leaves_the_type_sets_nothing`, `indicators::trend::a_swing_window_whose_span_leaves_the_type_publishes_nothing` | ✓ |
| I-07 | A refused bar changes **nothing**. `Evaluator::step` used to raise VWAP's accumulator refusal after the session rollover and seven module folds, so the next bar's bits were computed from a bar the caller was told was refused — four invented bits, and no agreement with a clean evaluator over 400 following bars | `indicators::evaluator::an_overflowing_accumulator_reaches_the_caller_as_a_refusal`, `indicators::evaluator::a_refused_bar_does_not_complete_a_session` | ✓ |
| I-08 | `TrendState::bits` is a function of the bar. It took `&mut self` and advanced the market-structure latch from inside the emit, so the same close answered `choch_bearish` then `bos_bearish`. `bits(&self)` makes the old shape a **compile error** | `indicators::trend::bits_is_idempotent_across_a_break_and_a_change_of_character`, `indicators::trend::the_structure_latch_advances_exactly_once_per_candle` | ✓ |
| I-09 | A change of character **reaches the mask**. I-08's two tests both pass if `step` stops advancing the latch altogether, which makes positions 58 and 59 unreachable and reports every reversal as a continuation. This is the only test in `trend.rs` that fails when the advance is removed | `indicators::trend::a_change_of_character_reaches_the_mask` | ✓ |

| I-10 | A non-regular session's OHLC **never** becomes the previous-day anchor. `docs/00-charter.md` §3 forbids it by name and claimed an "anchor walk restricted to trading days" that did not exist — a grep for holiday/muhurat/trading_calendar over the three crates returned zero hits. Measured before the fix: all 375 bars of the session after a 60-bar day emitted different pivot positions | `indicators::evaluator::a_non_regular_session_never_becomes_the_previous_day_anchor` | ✓ |
| I-11 | A non-regular session does not advance the five-session rolling window. The second of the charter's three prohibitions: `Prev5` is what positions 110–120 are measured against, and a one-hour OHLC entering it contaminates them for five more sessions | `indicators::evaluator::a_non_regular_session_does_not_advance_the_rolling_window` | ✓ |
| I-12 | A non-regular session **still emits its own bars**. The charter forbids that hour becoming an anchor, not that it be silenced — it is real trading, and every intraday position on it is a genuine measurement | `indicators::evaluator::a_non_regular_session_still_emits_its_own_bars` | ✓ |

| I-13 | `Evaluator::warmed_up` is **monotone** and `every_family_can_answer` is not, because the opening ranges re-open every session. Collapsing the two — which the first version did — produces a signal that goes false at every session boundary and cannot be used to choose where a sweep starts | `indicators::evaluator::only_the_run_level_signal_is_monotone` | ✓ |
| I-14 | `warmed_up` is a **conjunction**, so the slowest family decides. Five 30-bar sessions fills `Prev5` while the 200-period EMA has seen 150 candles, and the signal must be false. Reducing it to the session count alone left every test green until this row asserted the run-level signal directly instead of the per-bar one | `indicators::evaluator::the_slowest_family_decides_and_it_is_not_always_prev5` | ✓ |
| I-15 | The signals report the boundary the five-session ladder imposes: false at four completed sessions, true at five with the longest window closed | `indicators::evaluator::the_evaluator_reports_when_every_family_can_finally_answer` | ✓ |

| I-16 | 276/277 compare the close to the **session's** open, and a flat close sets **neither** — nor does the session's first bar, which has no session open to compare against yet | `indicators::evaluator::the_close_against_the_session_open_sets_at_most_one_bit` | ✓ |
| I-17 | 278/279 report the structure **in force between** breaks, not only on the bars where 56–59 fire. Without that distinction they are a duplicate of the break events and every other test still passes | `indicators::evaluator::the_structure_in_force_is_reported_between_breaks_and_not_before_the_first` | ✓ |

| I-18 | The six non-regular days ARE the charter's six, derived by an arithmetic sharing no code with the constant. A sweep shifted 2023-11-12 by one day in **both** spellings — so the const assertion still passed — and nothing caught it. An off-by-one is worse than the defect the calendar fixed: a regular session's anchor is discarded while the real Muhurat goes on poisoning the next day's | `indicators::evaluator::the_six_non_regular_days_are_the_charter_dates` | ✓ |
| I-19 | 278 means UP and 279 means DOWN. I-17 passes with the mapping **swapped**, so the regime could have been reported permanently inverted — a sweep asking for "structure up" would get every bar where it was down, which is a false combination rather than a missing one | `indicators::evaluator::the_structure_direction_bits_are_not_swapped` | ✓ |
| I-20 | `warmed_up` requires the pivot ladder, not only the session count. The four conditions look coupled and are not: `close_the_books` fills `prev5` unconditionally and installs `yesterday` only `if let Ok(levels)`, so five unusable sessions give a full window with no ladder | `indicators::evaluator::warmed_up_is_false_when_the_pivot_ladder_is_absent_despite_five_sessions` | ✓ |

| I-21 | `Calendar::default()` is the charter's six, **not** an empty calendar. `impl Default` was the only uncovered FUNCTION in these three crates, and its body was mutated to `all_regular()` uncaught — the same hole twice: a `Default` nobody exercises is one whose value nobody has checked, and an empty one silently restores the defect D-0110 fixed for every consumer that writes `Calendar::default()` | `indicators::evaluator::the_default_calendar_is_the_charters_six_and_not_an_empty_one` | ✓ |

| I-22 | The market-structure latch advances from the **same** classification the mask was built from. `TrendState::emit` returns both and `step` shares one value, so there is exactly one `classify` on the library path — folding before the advance, or advancing on a different price, were each uncaught mutations and the second is now unwritable | `indicators::trend::bits_is_idempotent_across_a_break_and_a_change_of_character`, `indicators::trend::a_change_of_character_reaches_the_mask` | ✓ |

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
| HF-01 | **A feed's history floor is keyed on the RUNG, and one vendor's two rungs carry two different floors.** Groww's own interval table gives its `1 day` row "Full history" and its `1 min` row "Last 3 months", so a per-vendor floor is necessarily wrong for one of them; the descriptor answers `RollingMonths { months: 3 }` at `1min` and `Fixed 2020-01-01` at `1day`, and the two are asserted to differ. Every recorded row names a rung the store can actually file | `pull::vendor::one_vendors_two_rungs_carry_two_different_floors` | ✓ |
| HF-02 | **A rung nothing states a floor for claims NOTHING** — not zero, not the epoch, not "no limit". Both archive feeds record an empty table and every rung of both answers `HistoryFloor::Unstated`; a broker rung no source speaks about (`1hr`) answers the same rather than inheriting the rung beside it | `pull::vendor::a_rung_with_no_recorded_floor_claims_nothing` | ✓ |
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
| CV-07 | **A per-feed answer differs per feed, and the fixture proves it rather than the prose.** Three index series, Groww listing two and Dhan two, only one shared: the universe count is 3 and neither feed reaches 3. The swept pair is counted the same way, because "the engine surface is two instruments" does not mean a given broker lists both | `api::coverage::the_two_feeds_reach_different_index_sets_and_the_counts_say_so` · `api::coverage::the_swept_pair_is_counted_per_feed_like_everything_else` | ✓ |
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

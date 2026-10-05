# Round 2 data-path attack — report (HEAD 8d95552, uncommitted)

## Findings table

| attack | cases tried | failures on HEAD | fixed? | evidence |
|---|---|---|---|---|
| R2-1 D-3122 skips reach the operator's receipt (api `landed_answer` Balances line, `audit::Record::of_run` note) | 64 seeded receipts + 2 note cases | 2 tests failed on HEAD: printed `yes — 480 read = 466 stored + 0 folded + 0 dropped` (false equation); journal note claimed "stored, folded or dropped" over a skipped candle | yes (D-3180) | `a_balanced_receipt_prints_an_equation_that_adds_up_including_decoder_skips`, `the_journal_note_of_a_run_with_decoder_skips_names_them` in crates/api/src/attack_r2_receipt_tests.rs:76,127; fix crates/api/src/server.rs:10503-10540 + `decoder_skips_said` :10573, crates/api/src/audit.rs:704 |
| R2-2 D-3140 last-stamp read vs a rotted last record | 5 tail shapes (1, 10, 73, 74, 146 records) x 64 stamp bits = 320; + 48 non-stamp-byte rots of a full tail block | 1 test failed on HEAD: a single flipped stamp bit answered `LastStampDisagrees{header: right, record: rotted}` instead of `BlockChecksum` — even in the partially covered tail block D-0910 already verifies, because D-3140's check ran first | yes (D-3181) | `a_rotted_last_stamp_is_refused_as_a_block_checksum_never_as_a_header_disagreement` crates/store/tests/attack_r2_store.rs:140; D-0910 preserved: `rot_outside_the_stamp_of_a_full_tail_block_still_lets_the_next_append_commit` :179; fix crates/store/src/file.rs:2158 (`last_stamp_is_the_headers`) |
| E2E decode -> ingest -> store -> derive -> SpotBook on seeded random July-2025 months (1-4 sessions, null/impossible candles, pre-open/after-close, exact twins, random day-aligned chunking, rerun, second chunking) | 48 months (~37k candles) | 0 | n/a | `random_months_land_the_modelled_bytes_through_every_stage` crates/pull/tests/attack_r2_pipeline.rs:393 |
| D-3117 vendor vs solved premium screen agree (same arm, same bits) | 200,000 random quotes x 3 vendor vols + solved-vol round trip | 0 | n/a | `the_vendor_and_solved_paths_refuse_the_same_premiums_under_the_same_arm` crates/pull/tests/attack_r2_pricing.rs:103 |
| Hostile vendor volatility (NaN, ±inf, 0, -0, -0.2, 10.5, ±MAX) | 45,000 | 0 | n/a | `a_hostile_vendor_volatility_is_refused_never_priced` attack_r2_pricing.rs:153 |
| D-3130 `Bucket::of_secs` exhaustive + every store rung still bucketable (callers: ingest fold_in_place/derive, cli fold_audit, cli tests) | 200,000 widths + 3 extremes + 10 rungs | 0 | n/a | `of_secs_accepts_exactly_the_divisors_of_a_day_and_every_store_rung` attack_r2_fold.rs:41 |
| D-3131/3132 `fold_from_bars` composition over every rung pair on holed sessions | 20 x 9 x 9 pairs (>400 compositions, 80 GridMisaligned) | 0 | n/a | `folding_through_any_legal_intermediate_rung_is_folding_directly` attack_r2_fold.rs:99 |
| D-3131 every single off-grid shift / repeat refused at its position | >300 | 0 | n/a | `every_single_off_grid_or_repeated_bar_is_refused_at_its_position` attack_r2_fold.rs:164 |

Also reviewed by reading (no failing test written): every `RawWindow {` construction (only `decode`/`FakeSource` build windows; `finish` is the single attach site for all three shapes), `Ingested::absorb` (carries `decoder_skips`), cli has no ingest caller, `complete_minutes_with_calendar` still withholds (not refuses) duplicate/off-grid minutes, SpotBook ambiguity reaches api only through `PricingError::SpotAmbiguous`'s Display (via `PricedAll::why`), `screen_premium` uses the same check/finite/bounds order the solver had.

## Open items (not fixed, named)

1. **api fno chain receipt drops skips.** `FnoLanded::rows_read` (server.rs ~11891, `fetch_chain_chunks`) is documented as "successfully decoded rows"; decoder skips on a contract window appear nowhere on `/api fno` receipts or `FnoCounts`. Same D-3122 class as R2-1, on a path that needs the async fixture (`fno_boundary_tests::Fixture::serve`) to test. Not fixed: no failing test written.
2. **api cash-schedule-refused branch** (server.rs ~7629) builds `rows_read` from `body.rows.len()` only and leaves `decoder_skips` zero. `balances()` is already false there (a failure is pushed), so no receipt claims balance; it only under-counts the vendor's rows. Not fixed: untested branch.
3. **Round-1 fixture bug (test only):** `crates/pull/tests/attack_ingest.rs::plan` passes `PriceScale::Rupees` over decoder output that is already paisa, so every bar those tests store is x100 (measured: 254128400 stored for a 25412.84 candle). Production uses `pull::http::DECODED_PRICE_SCALE` (server.rs:6452). Their assertions are relative so they still pass; the new pipeline test uses the constant. Ingest silently accepts the double scale; that is the trap `DECODED_PRICE_SCALE` names, not a new defect.
4. Different chunkings of one answer land identical RECORDS but not identical FILE BYTES (header generation counts commits). Expected, stated in the test.

## Draft decision entries

**D-3180 — The receipt and the journal name the candles the decoder skipped.**
D-3122 counted decoder-skipped candles into `Ingested::rows_read` and made `balances` account for them, but the spot receipt (`api::server::landed_answer`) still printed `yes — R read = S stored + F folded + D dropped` with no skip term, so a balanced run with skips printed a false equation (measured: `480 read = 466 stored + 0 folded + 0 dropped`). The journal note from `audit::Record::of_run` said every row was "stored, folded into an open bar, or dropped" over a candle that was none of those. The receipt now has a "Candles the decoder skipped" row (total and each reason that fired) and both Balances lines carry `+ N skipped by the decoder`. The 256-byte record has no spare field, so a balanced run with skips gets the note `every row accounted for; N skipped by the decoder`, which fits `NOTE_CAPACITY` (68) for any count. One existing assertion (`several_diagnostics_for_one_instrument_are_not_failed_member_counts`) was updated to the new wording. The fno chain receipt is still open (see open items).

**D-3181 — A rotted last stamp is refused as rot, not as a lying header.**
D-3140 compared the header's `last_ts_micros` with record `n_valid-1`, read without the block verify, and refused a mismatch as `LastStampDisagrees`. When the record rotted and the header was right, that named the wrong culprit. It also ran before D-0910's verify of a partially covered tail block, so even that case answered "header" whenever the flipped bits were in the stamp (failed for all 320 stamp-bit cases). On a mismatch only, the record's block is now verified against the sidecar first: a failure is returned as `BlockChecksum`, and `LastStampDisagrees` is returned only when the block verifies. The extra read is on the refusal path only. For every append that would have committed, D-0910's "a full tail block is not read" still holds; `rot_outside_the_stamp_of_a_full_tail_block_still_lets_the_next_append_commit` pins this.

## Draft invariant rows

| DPR-01 | A balanced spot receipt prints `read = stored + folded + dropped + skipped`, the sum is exact, and any decoder skip is named on the page | `a_balanced_receipt_prints_an_equation_that_adds_up_including_decoder_skips` in `crates/api/src/attack_r2_receipt_tests.rs` | ✓ |
| DPR-02 | The journal note of a balanced run with decoder skips names them and is not cut | `the_journal_note_of_a_run_with_decoder_skips_names_them` in `crates/api/src/attack_r2_receipt_tests.rs` | ✓ |
| DPR-03 | A rotted stamp in the last record is refused as `BlockChecksum` of its block, never `LastStampDisagrees`, and the month is byte-identical | `a_rotted_last_stamp_is_refused_as_a_block_checksum_never_as_a_header_disagreement` in `crates/store/tests/attack_r2_store.rs` | ✓ |
| DPR-04 | Rot outside the stamp of a full tail block does not stop the next append (D-0910 unchanged by D-3181) | `rot_outside_the_stamp_of_a_full_tail_block_still_lets_the_next_append_commit` in `crates/store/tests/attack_r2_store.rs` | ✓ |
| DPR-05 | On seeded random months, decode->ingest->store->derive lands exactly the modelled minute file and complete derived buckets; the receipt arithmetic holds with skips; a skip is named as a coverage gap; a rerun changes no byte; any chunking lands identical records; SpotBook answers every stored stamp and no neighbour | `random_months_land_the_modelled_bytes_through_every_stage` in `crates/pull/tests/attack_r2_pipeline.rs` | ✓ |
| DPR-06 | The vendor-vol and solved paths refuse a premium under the same no-arbitrage arm; a solved vol re-priced as a vendor vol gives bit-identical greeks | `the_vendor_and_solved_paths_refuse_the_same_premiums_under_the_same_arm` in `crates/pull/tests/attack_r2_pricing.rs` | ✓ |
| DPR-07 | A non-finite, non-positive or out-of-band vendor volatility never prices | `a_hostile_vendor_volatility_is_refused_never_priced` in `crates/pull/tests/attack_r2_pricing.rs` | ✓ |
| DPR-08 | `Bucket::of_secs` accepts exactly the 96 divisors of a day, and every store rung | `of_secs_accepts_exactly_the_divisors_of_a_day_and_every_store_rung` in `crates/pull/tests/attack_r2_fold.rs` | ✓ |
| DPR-09 | Folding through any legal intermediate rung equals folding directly; otherwise only `NarrowerThanSource`/`GridMisaligned` | `folding_through_any_legal_intermediate_rung_is_folding_directly` in `crates/pull/tests/attack_r2_fold.rs` | ✓ |
| DPR-10 | Every off-grid or repeated source bar is refused at its own position | `every_single_off_grid_or_repeated_bar_is_refused_at_its_position` in `crates/pull/tests/attack_r2_fold.rs` | ✓ |

## Commands and counts

- `CARGO_BUILD_JOBS=3 cargo test -p api --locked --lib attack_r2` on HEAD + tests: 0 passed, 2 failed (R2-1 shown). After the fix it passes inside the workspace run.
- `cargo test -p store --locked --test attack_r2_store` on HEAD: 1 passed, 1 failed (R2-2). After the fix, `cargo test -p store --locked`: 382 passed, 0 failed (run as root, no permission-test failures seen).
- `cargo test -p pull --locked --test attack_r2_pipeline --test attack_r2_pricing --test attack_r2_fold`: 6 passed, 0 failed (pipeline 48 cases in about 4.1 s; pricing about 0.6 s).
- `cargo fmt --all --check`: clean.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean (after renaming single-char bindings in my pipeline test).
- `cargo test --workspace --locked --no-fail-fast`: exit 0. 7040 passed, 0 failed, 14 ignored, across 213 test binaries.

## O(1)

D-3181 adds no work to a committing append. The block verify runs only when the stamps already disagree, which is a refusal. I took no new timings.

## Needs real vendor data

None of the findings do. Open item 1 needs a real contract window that has null-price candles to show what the fno receipt says to an operator.

## Files modified

- crates/api/src/server.rs (receipt rows and balance lines, `decoder_skips_said`, one existing assertion's wording, `#[path]` include of the new test module)
- crates/api/src/audit.rs (journal note)
- crates/store/src/file.rs (`last_stamp_is_the_headers`)
- new: crates/api/src/attack_r2_receipt_tests.rs, crates/store/tests/attack_r2_store.rs, crates/pull/tests/attack_r2_pipeline.rs, crates/pull/tests/attack_r2_pricing.rs, crates/pull/tests/attack_r2_fold.rs

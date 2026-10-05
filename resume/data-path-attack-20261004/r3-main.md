# Round 3 data-path attack — report (base 55b92db, branch claude/attack-data-pipeline-hgxmw9)

## Commits (local, not pushed)

- d214ca9 api, pull: chain and refused-schedule receipts count decoder-skipped candles; ingest attack fixture scales once (D-3182..D-3184)
- d807a8e gates: the language-purity static gates pass on the data-path attack tree (D-3185..D-3189)
- defa05d api, pull: round 3 data-path attack fixes (D-3123..D-3126)
- 0368dbb docs: decisions, invariants and findings for rounds 2 and 3 (D-3180..D-3189, D-3123..D-3126)

## STEP 1 — round 2 open items

| item | failing test on 55b92db | measured on HEAD | fix | decision / invariant |
|---|---|---|---|---|
| (a) F&O chain receipt drops decoder skips | `a_named_chain_receipt_counts_and_names_the_candles_the_decoder_skipped`, `a_refused_contract_keeps_the_skipped_candles_of_its_answered_chunks` (crates/api/src/fno_boundary_tests.rs, end of file; async `Fixture::serve` harness, 3 null after-close candles) | `rows_read` 385, expected 388; no skip row on the page | `FetchedBatch`/`FnoLanded` carry vendor count + `decoder_skips`; chain receipt gets the "Candles the decoder skipped" row (server.rs `fetch_chain_chunks`, `fno_land`, chain receipt facts) | D-3182, DPR-11, DPR-12 |
| (b) refused cash schedule under-counts | `a_refused_cash_schedule_still_counts_the_candles_the_decoder_skipped` (server.rs `mod tests`) | `rows_read` 360, expected 365; skips 0 | new `vendor_count` helper used on the refused branch of `land_spot` | D-3183, DPR-13 |
| (c) attack_ingest fixture x100 | `the_session_edges_are_kept_or_dropped_by_name_and_the_tally_balances` now asserts exact paisa | stored (240000500, 240047500, 239971000, 240014000), expected (2400005, 2400475, 2399710, 2400140) | `plan` uses `pull::http::DECODED_PRICE_SCALE` | D-3184, DPR-14 (test-only defect) |

## STEP 2 — static gates (rungates.sh, Gate 1e and Gate 7 skipped)

Before (55b92db): FAIL 1d, 19, 10, 11, 12; all others PASS.
After (0368dbb): 28 PASS, 0 FAIL, 2 SKIP.

| gate | cause | fix | decision |
|---|---|---|---|
| 1d | 55 undeclared literals from the attack tests and a chain.rs test module (+ `clean` from round 3) | group `ATTACK_LITERAL` in .github/gates_tree.rs with reasons by kind | D-3185 |
| 19 | ingest.rs D-3120 month refusal pushed a failure with no event | `note_not_filed(.., "month append", ..)` before the push; the server.rs refused-schedule branch kept its emit within the window via `vendor_count` | D-3186 |
| 10 | MR-04 named the pre-D-3158 test name | row kept, old statement struck, reversal stated, proof repointed | D-3188 |
| 11 | rule 2: bsm 37/36, moneyness 9/6, solver 15/14, pricing 24/22; rule 3: chain.rs, pricing.rs; rule 7: pricing 4/2; loose fno.rs rule 6 | premium widening hoisted (pricing back to 22); greeks counts raised; chain.rs 2 (filed + D-3126 `listed`, both reserve per expiry), pricing.rs 1 (`ambiguous`, empty on well-formed months); pricing member 4 (two hash probes); fno.rs rule-6 entry removed; reasons in docs/06-limits.md | D-3189 |
| 12 | attack_fold.rs:790 cost claim named no proof | doc names `crates/pull/tests/attack_fold.rs` | D-3187 |

Also fixed while there: the `pull::pricing::price` doc block had its UNVERIFIED note spliced into the middle of a sentence; reordered.

## STEP 4 — round 3 attack

| attack | cases | failures on commit before | fixed? | evidence |
|---|---|---|---|---|
| D-3110 ambiguity reaching the chain receipt (`chain_quotes` uses `SpotBook::at`) | 3 (contradicted stamp, two contradicted stamps, missing stamp) | 1: contradicted stamp reported as "no index bar is stored" | yes, D-3123 | `a_chain_row_at_a_contradicted_index_stamp_is_refused_as_a_disagreement` crates/api/src/attack_r3_tests.rs; fix server.rs `chain_quotes` (`lookup` match) |
| D-3111 class dedupe across contract-months (`PricedCount::absorb`/`absorb_count`) | 7 months via real `chain_quotes` + `price_all`, both folds | 1: five below-intrinsic sentences filled every slot, `supremum` refusal never named | yes, D-3124 | `a_receipt_names_a_new_kind_of_refusal_after_many_months_of_one_kind` attack_r3_tests.rs; fix server.rs `keep_reason`/`reason_shape` |
| D-3122 on the local-archive CSV door | clean 375-row member + 375 rows with 3 negative-volume rows | 1: rows_read 375 of 378 offered, decoder_skips 0, balances() true | yes, D-3125 | `a_csv_row_the_decoder_skips_is_still_on_the_receipt` crates/pull/tests/attack_r3_ingest.rs; fix csv.rs `decode_counted`, archive.rs `Member::skipped`, ingest.rs `from_members_inner`/`refused_whole` |
| D-3116 dedupe across expiries for monthly names | 2 walks (monthly FUT+CE under 2 expiries; dated FUT under 2 expiries) | 1: monthly names filed as 4 contracts | yes, D-3126 | `one_vendor_name_under_two_expiries_is_filed_once` crates/pull/tests/attack_r3_chain.rs; fix chain.rs `listed` + `relisted` |

Tried by reading, no failing test written (no defect found):
- HTTP decoders: all four prices checked for null before any push; skips carried through `finish` on all three shapes.
- Rolling (Dhan) decoder refuses null volume/stamps rather than skipping, so it is not the D-3122 class.
- D-3181 `last_stamp_is_the_headers`: empty file returns early; verify only on mismatch.
- `audit` / `audit_json` / web audit page: no arithmetic on `rows_read`, so counting skips into the fno journal changes no derived figure.
- `from_window` early returns (off-grid, conflict) already count `raw.skipped`; the new `Member::skipped` is default there so nothing double counts.
- `ingest::from_rows` (rolling door) has no decoder skips to carry.

## Final checks (on 0368dbb)

- `cargo fmt --all --check`: clean
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean
- `cargo test --workspace --locked --no-fail-fast`: SEE BELOW
- static gates: 28 PASS, 0 FAIL, Gate 1e and Gate 7 skipped

## Notes

- Decision numbers used: D-3182..D-3189 (STEP 1-2) and D-3123..D-3126 (round 3), per coordinator.
- Needs real vendor data: none of the findings. D-3126's monthly-name relisting is a vendor misbehaviour shape; whether Groww ever lists one monthly name under two dates is not known from inside the repo.
- Open (not fixed, pre-existing, outside round 1-2 code): `Rolled::absorb` pushes run-failure reasons with no dedupe at all (server.rs, `Rolled::absorb`), so one failure repeated across 252 rolling runs fills all slots. Not tested.

# gaps — missing functionality and unwired features at HEAD 1087e54

## Verdict

Every CLAUDE.md law I was asked to check holds in code at 1087e54. That covers the nine-term identity with feed, VIX as reference only, equities kept out of Selection V6, no `k` parameter, paisa with half-up snapping, the i64::MIN OI null, credentials that halt with no default and are never minted, the opposite provenance banners, and append-only bits. The main finding is about reachability.

I ran a compiler experiment on a scratch copy of HEAD. In it, every `pub mod` that neither binary references was made `pub(crate)`, the module-level `expect/allow(dead_code)` attributes were lifted, and the crate was checked with `cargo check`. It found **1,334 dead-code items**. About 16 `pub` modules make up the whole superseded Step-3 V1–V4 chain: Selection V1/V3/V4, Global Replay V1/V2, the Step-3 comparison read model, institutional evidence/statistics, admission store/writer and Execution disposition V2. These modules compile and are tested, but **no `cli` verb and no `api` route reaches them**. Because they are declared `pub`, the dead-code lint never flags them.

Several runner validation primitives also have only test callers: Benjamini–Hochberg, the anchored walk-forward bottom-half rate and the V3 projected walk-forward. These look like answers to the user's objective, but no entry point uses them.

The user's objective is rare, massive single-stock winners across 210 instruments. Five things it needs are missing. All five are documented as limits, but they are still missing:

- split/bonus (corporate-action) detection
- point-in-time F&O membership
- any equity charge stack or slippage
- multi-day holds
- out-of-sample validation and multiple-testing control for the **pooled cross-sectional** hypothesis that `pool` searches for

The Boolean `*-qualified-*` verbs do provide later-period OOS and a search-wide alpha spend over up to 215 symbols. However, nothing feeds `pool` or `range-rung` discoveries into them; the catalog is hand-written. `docs/07-plan.md` has drifted on several status rows.

Method: code reading at HEAD, the compiler experiment above (scratch copy only, no tracked file touched), grep reference counts, and one probe test (deleted afterwards). `git status --porcelain` shows no file of mine.

## Probe run

`crates/cli/tests/zz_audit_gaps_1.rs` called `cli::run(["sweep","3","40"])`. It was run with `cargo test -p cli --test zz_audit_gaps_1 -- --nocapture`, and the file was then deleted. Output, verbatim (trimmed):

```
exit=1
has 'identity': true
has 64-hex: false
=== THESE BARS ARE GENERATED, NOT MARKET DATA ===
...
SWEEP
  identity                            NOT RECORDED
BARS
  offered                                     1125
  swept                                          0  0.0%
  warming (bits correct, not swept)           1125  100.0%
test synthetic_sweep_has_no_run_identity ... ok
```

## Compiler reachability experiment (cli)

Scratch copy: `git archive HEAD`. In `crates/cli/src/lib.rs` (scratch only):

- Every `pub mod` that `api` or `main` does not name became `pub(crate) mod`.
- Root-level `pub` items not named by `api`/`main` became `pub(crate)`.
- `expect/allow(dead_code)` became `warn(dead_code)`.

The command was `cargo check -p cli --lib`, which exited 0 with 1,334 dead-code warnings. The kept-public modules are the 25 that `api` imports; their items count as reachable (over-approximation), so this list is a lower bound on what is unreachable.

| module (cli/src) | dead items | approx. pub items | reading |
|---|---:|---:|---|
| global_replay.rs (V1) | 126 | 47 | whole module unreachable |
| global_replay_v2.rs | 115 | 72 | whole module unreachable |
| population.rs (V4) | 107 | 97 | whole module unreachable |
| execution_capability.rs | 99 | 68 | unreachable |
| execution_disposition_v2.rs | 79 | 84 | unreachable |
| selection_v4.rs | 78 | 52 | unreachable |
| selection.rs | 77 | 87 | unreachable |
| step3_comparison.rs | 69 | 23 | unreachable (no file references it at all) |
| institutional_statistics.rs | 63 | 45 | unreachable |
| admission_store.rs | 62 | 42 | unreachable |
| selection_v3.rs | 52 | 35 | unreachable |
| population_admission_writer.rs | 51 | 26 | unreachable |
| institutional_evidence.rs | 51 | 29 | mostly unreachable (`build_institutional_evidence_v1` :948, `measure_independent_support_sessions_v1` :1243, `validation_values` :1835 ...) |
| stored_data_completeness.rs | 48 | 26 | unreachable |
| selection_v4_authority.rs, admission_join.rs | 18, 13 | — | unreachable |
| anchored_search_lineage_v2/v3, all_rung_population_v5, all_rung_selection_v5, population_admission_v3/v4 | — | — | already under `expect/allow(dead_code)` with stated reasons (lib.rs:90-148) |

## Findings

| id | severity | crate | file:line | what is wrong (plain language) | evidence | status |
|---|---|---|---|---|---|---|
| gaps-1 | medium | cli | cli/src/lib.rs:160-276 (`pub mod admission_store`, `selection`, `selection_v3`, `selection_v4`, `selection_v4_authority`, `global_replay`, `global_replay_v2`, `population`, `execution_capability`, `execution_disposition_v2`, `institutional_evidence`, `institutional_statistics`, `stored_data_completeness`, `population_admission_writer`, `admission_join`, `step3_comparison`) | The whole earlier Step-3 authority chain is built, tested and kept compiling, but no `cli` verb and no `api` route calls it. That is roughly 1,200 items. Because the modules are `pub`, the `dead_code` lint and `clippy -D warnings` cannot flag them, so this unreachable code costs build time, tests and coverage obligations with no operator effect. `step3_comparison` is referenced by no other file at all. | Compiler experiment above. `grep -rlE "\bstep3_comparison::" crates/cli/src crates/api/src` lists only its own file. Dispatch table cli/src/lib.rs:2174-2300 names no verb that reaches these modules. | NEW. The plan describes these as superseded checkpoints (docs/07-plan.md §11 rows ~106-116) but nowhere says they are unreachable. |
| gaps-2 | info | cli | cli/src/lib.rs:90-148 | Six crate-private successor modules are still dead outside tests: lineage V2/V3, Population V5, Selection V5, Admission V3/V4. | `#[cfg_attr(not(test), expect(dead_code, reason = "the canonical all-rung Population V5 authority awaits its typed Execution V3 consumer"))]` and five similar attributes | DOCUMENTED: lib.rs reasons, docs/07-plan.md §11 |
| gaps-3 | medium | runner | runner/src/significance.rs:360 `benjamini_hochberg`; runner/src/pbo.rs:160 `anchored_walk_forward_bottom_half_rate_v1`; runner/src/validate.rs:2166 `walk_forward_projected_prepared_anchored_search_v3`; runner/src/admission.rs:4754/4814 `evaluate_v2_projection`/`evaluate_v3_projection` | Validation primitives the objective needs (FDR control, the walk-forward overfit rate, the V3 walk-forward door, admission projections) exist and are tested, but no production path calls them. A reader of the runner API would believe FDR control is available, and it is reachable from nowhere. | `grep -rnw benjamini_hochberg crates` gives significance.rs:360 (definition) and :564/:851/:863/:876/:882, all after `#[cfg(test)]` at :557. pbo.rs refs: :417/:451, after test module :410. validate.rs refs: :5146/:5315/:7206, after test module :5135. admission.rs refs: :6135/:6365, after test module :6092. evaluate_v2/v3_projection refs: :8570+/:8859+, inside test region. | NEW (prior/probeengine.md noted only `p_value` as unwired) |
| gaps-4 | low | cli | cli/src/research.rs:81-104 (`bounds_micros`, `contains`, `retain`), :134 `midnight_micros` | The frozen 2020-01-01→yesterday research window is never applied to any search, sweep or run identity. Only the read-only `research-plan` inventory uses the type. | Compiler: `research.rs:81:12: methods bounds_micros, contains, and retain are never used`; `research.rs:134:4: function midnight_micros is never used` | DOCUMENTED: docs/07-plan.md:882 item 4 ("Integrate the frozen date ... into the actual search") — still open |
| gaps-5 | medium | cli | cli/src/pool.rs:55-72, :817-881 | The cross-sectional "which stock before it moves" pooled table is exactly the user's objective, and it has: no out-of-sample split, no multiple-testing correction (it is the largest of instruments × candidates comparisons), no persistence or identity, shortlist-only discovery (union of per-instrument top rows found under the support floor, so truly rare setups are pruned before pooling), and coarse signal-bar fills. | pool.rs:59 "Every figure is in sample and the pooled table is the largest of `instruments × candidates` comparisons"; :67 "The POOLED table is rendered and is NOT written to the store"; :875 `grid::evaluate_over(bars, &column, ...)` with `bars = span.bars` (signal rung) | DOCUMENTED: pool.rs header, D-0509, docs/07-plan.md:884-889 items 5-6. Coarse fills KNOWN: fix-queue_lane1-b GAP13-15 (NOT-FIXED in resume v1) |
| gaps-6 | high (objective) | cli/runner | runner/src/audit.rs:158-170; api/src/server.rs:4129-4140 | Corporate actions (split, bonus, demerger) are not detected or adjusted in any equity sweep. A 1:2 split is a −50% overnight gap: it is exactly the "rare, tiny-loss, massive-win" signature the objective hunts, so equity rankings can be topped by data artefacts. Reports say "CORPORATE ACTIONS ARE UNCHECKED"; nothing detects or excludes such days. | audit.rs:158 "D-0018 requires a suspected split or bonus -- an unexplained overnight ..."; :170 "...demerger detection has run over these bars, so an overnight jump in..."; api/src/trades.rs:845 asserts the label only | DOCUMENTED: D-0018, D-0694 (labelled, not implemented) |
| gaps-7 | medium (objective) | core/cli | core/src/universe.rs (`FNO_UNDERLYINGS` constant); runner/src/research_family.rs:4; cli/src/research.rs:208 | The swept equity set is today's F&O list applied back to 2020. There is no point-in-time membership, so stocks that left F&O (often the large losers) are absent: survivorship bias in every pooled or ranked equity result. | research_family.rs:4 "It is not historical point-in-time membership"; research.rs:208 "...listing/corporate-action history and point-in-time F&O membership." | DOCUMENTED: docs/07-plan.md cash-stock item 3 |
| gaps-8 | medium (objective) | costs/runner | costs/src/trip.rs (charge stack); runner/src/trade.rs:64-80; runner/src/audit.rs:279 | `costs::trip::price` (the F&O charge computation) is called by no crate outside `costs`. No equity charge path exists, and fills carry zero slippage/spread. "Loses tiny" tight-stop setups are therefore flattered most. | `grep -rnw trip crates/{cli,runner,api,pull}/src \| grep costs` finds only doc comments; audit.rs:279 "...until `costs::trip::price` is wired"; trade.rs:75 "`costs::scope::Segment` has no equity variant" | DOCUMENTED: CLAUDE.md §1, D-0509/D-0525/D-0681 |
| gaps-9 | low (objective) | runner | runner/src/outcome.rs:58-70 `FORCED_EXIT_MINUTE = 15*60+10` | Every trade is intraday with a forced 15:10 IST exit. A multi-day trend (the typical "massive winner" in a single stock) cannot be represented. | outcome.rs:70 `pub const FORCED_EXIT_MINUTE: i64 = 15 * 60 + 10;` | DOCUMENTED: product policy, D-0531 and docs/05-decisions.md:29471-29501 |
| gaps-10 | low | api/cli | cli/src/ledger_v6.rs:75; api/src (no match) | The authoritative V6 chain (`ledger-all`, `ledger-v6`, `ledger-v6-replay` → Selection V6 / Global Replay V4) is CLI-only text. No API route or browser page reads or launches it, while the operator procedure (plan §0) is "press Run on api, open the browser". Positive check: equities are structurally excluded from Selection V6 (`ROUTE_FAMILIES = ["NIFTY","BANKNIFTY"]`). | `grep -rln -i "selection_v6\|ledger_v6\|global_replay_v4\|ledger-v6" crates/api/src web/src` gives no output | DOCUMENTED: docs/07-plan.md §11 order 5 "current successor callers open" (API half) |
| gaps-11 | low | cli | cli/src/lib.rs:353-384 (usage), pool.rs, results.rs | No automated handoff from discovery (`pool`, `range-rung`, `sweep-*` AND masks) to later-period qualification. The `boolean-*-stored` verbs take a hand-written `CATALOG_FILE`, so validating a discovered winner out of sample is a manual transcription step, with the D-0751 risk that displayed text cannot always be re-entered. | `grep -rn -i "catalog_file\|write.*catalog\|export.*catalog" cli/src/{pool,results,frontier,lib}.rs` finds only test fixture writes | NEW (scenario gap) |
| gaps-12 | info | runner/cli | runner/src/search_allocation_v1.rs:1-17; docs/30-search-wide-qualification.md "Opening many independent research projects is not covered" | Multiple-testing control is per declared search. Re-running the qualified search on a new symbol list, grammar, window or policy starts a fresh alpha budget, so the 210× multiplicity across separate invocations is uncontrolled. | search_allocation_v1.rs:10 "Changing directories, seeds or batch sizes must not restart the spending sequence"; docs/30 states the cross-project limit | DOCUMENTED: docs/30 |
| gaps-13 | info | docs | docs/07-plan.md:96 (R-4), :431 (§9.3), :481 (§10 #11); §11 "No CLI ... caller" rows | Status rows contradict the code. R-4 says F&O is "Not yet — OPEN", but `/pull/fno` is routed (api/src/server.rs:16135). §9.3 says "no trading calendar in this build", but `pull::calendar::kind_of`/`expected_bars` exist (calendar.rs:432) and CLAUDE.md §5 names it the canonical authority. §10 #11 says a token expiring mid-run has "no re-read", but `credential_law::Watch::reread` runs on the production path (server.rs:7932, :7963; D-0948). §11 says no CLI caller exists for Selection V6/Replay V4, but `ledger-v6`/`ledger-v6-replay` are dispatched (lib.rs:2275-2282). | quoted lines | NEW (doc drift) |
| gaps-14 | info | pull | pull/src/gaps.rs:548 `provable_holes`; pull/src/calendar.rs:432 `expected_bars`, :451 `sessions_between`; pull/src/fetch.rs:1017 `fetch_and_land` | Completeness-loop helpers with no production caller. The api `recovery`/`autopilot` code implements its own path (`classify_*`, dry-round retirement autopilot.rs:102), so these are a second, unused implementation. | `grep -rnw provable_holes crates` gives gaps.rs only (tests after :582). `expected_bars`/`sessions_between` refs only in calendar.rs test module (:466+). `fetch_and_land` refs only in pull/tests/pipeline.rs:824/:841 | NEW |
| gaps-15 | low | cli | cli/src/lib.rs:5967 `audit_run_within`, :10900 `breakeven_rr_bp`, :13718 `cadence_floor_ppm`; stored.rs:598 `calendar_receipt_v1`, :1508 `calendar_receipt_v2` (+ structs :230-387); minute_gaps.rs:277 `withhold_holed_days`; checksum_receipts.rs:111 `admit_month`; candidate_universe.rs:2974 `CandidateUniversePageV1`; pre_admission_data.rs:937 `PreAdmissionDataPageV1` | `pub` items in live modules that only tests call: calendar-receipt V1/V2 hashing, paged readers, helper functions. | Compiler warnings, e.g. `stored.rs:1508:8: function calendar_receipt_v2 is never used`; `lib.rs:5961:15: function audit_run_within is never used` | NEW |
| gaps-16 | info | cli | cli/src/lib.rs:2760 `render(&outcome, None)`, :3848 `render_auto(&found, None)` | Generated-bar runs (`sweep`, `audit`, `auto`) compute with no run identity, despite §3 rule 3 "No computation without that identity recorded". They say so loudly ("identity NOT RECORDED") and carry the GENERATED banner. | Probe output above | DOCUMENTED: CLAUDE.md §5 ("a synthetic bar has no instrument to name") |

## CLAUDE.md claims verified in code

| claim | verdict | evidence |
|---|---|---|
| Run identity = 9 terms incl. `feed` | HOLDS | runner/src/identity.rs:690-816: tags MASK, DIRECTION, INSTRUMENT (incl. kind :741-765), TIMEFRAME, PARAMS, DATA_DIGEST, VOCAB_VERSION, COMMIT, and :816 `term(&mut hasher, tag::FEED, run.feed.as_bytes())`. Test `every_term_changes_the_identity` varies feed (:1412-1418); `two_feeds_with_byte_identical_bars_do_not_collide` (:888). Stored callers pass `span.vendor.as_str()` (cli lib.rs:1096, :3538, :4090, :6162 ...). |
| VIX reference only: not in vocab, ranking or identity | HOLDS | `grep -rni vix crates/{vocab,engine,indicators}/src` returns nothing. stored.rs:1917-1948 `swept_index` refuses non-sweepable keys; test stored.rs:3555-3567 asserts INDIAVIX is "storable but not sweepable". Every stored loader goes through `swept_index` (lib.rs:721/1082, audited_stored.rs:49, audited_range.rs:156, boolean_catalog_command.rs:104, checksum_receipts.rs:549, pool.rs:564). index_stop_vix.rs:1-3 "cannot enter strategy/source/search identities, ranking or prices". |
| Equities: gross label, never in Selection V6/execution authority | HOLDS (structurally) | ledger_v6.rs:75 `const ROUTE_FAMILIES: [&str; 2] = ["NIFTY", "BANKNIFTY"];`. Labels: boolean_oos_command.rs:91, boolean_qualified_command.rs:144, boolean_search_command.rs:256, expression_search.rs:347, pool.rs:636, boolean_catalog_command.rs:443/480 (`CostScope::CashEquity.report_note()`). UNVERIFIED: whether `boolean-campaign-stored` / `boolean-grammar-campaign-stored` text carries the cost sentence; their own banner (boolean_campaign.rs:431-435) has none, and the per-rung delegation was not traced. |
| No `k` parameter | HOLDS | engine/src/lib.rs:1009-1016 `Ladder { min_hits, ceiling, pair_budget, support_lanes }`, no depth field. No `BRUTEX_*DEPTH`/`_K` env var (grep). The `ceiling`/`pair_budget` halts are documented at engine lib.rs:873-900. |
| Paisa i64, half-up at the write boundary | HOLDS | core/src/price.rs:108 `from_rupees_half_up`, :217 `from_rupee_text_half_up` (negative tie toward +inf, test :386-400). Used at pull/src/http.rs:1415, rolling.rs:733, lake/src/bar.rs:162. Archive CSV refuses a third decimal rather than snapping (pull/src/csv.rs:366-372, deliberate). |
| i64::MIN = OI null | HOLDS | store/src/format.rs:207 `pub const OI_NULL: i64 = i64::MIN;`; pull/src/fetch.rs:961 `row.open_interest.unwrap_or(i64::MIN)`; resample folds last-known OI, never a sum (runner/src/resample.rs:361). |
| Credentials: path from untracked config, halt loudly, no default | HOLDS | pull/src/config.rs:67-88 "It halts. It never defaults"; `load` :753. |
| Never mints; stale token re-read, halt on same value | HOLDS | No `session/token` / access-token endpoint anywhere (grep empty). api/src/credential_law.rs:1-30; production call server.rs:7932/7963 `watch.reread(...)`. `pull::totp` is used only by its own tests and emit_sites (KNOWN: lane3-b). |
| Provenance banners make opposite claims | HOLDS | cli lib.rs:337 `PROVENANCE` ("GENERATED, NOT MARKET DATA"), `STORED_PROVENANCE` ("REAL MARKET DATA"); test lib.rs:22233; probe output above. |
| Append-only condition bits | HOLDS (doc-coupled) | vocab/tests/table.rs:229 tombstones keep their index, :310 no live row in a retired position, :468 retired duplicates not re-added, :119 table equals docs/03-vocabulary.md name for name. A coordinated renumber of table and document together would pass, because nothing pins against prior history (VOCAB_VERSION=3, vocab/src/lib.rs:92). Info only. |

## Objective scenarios with no implementation (summary)

1. Split/bonus/demerger detection or adjustment for the 208 equities (gaps-6).
2. Point-in-time F&O membership, i.e. survivorship control (gaps-7).
3. Any equity charge stack, slippage or spread model (gaps-8). A charter-sourced equity charge stack is also the precondition CLAUDE.md §1 sets before any equity result can enter Selection V6 or execution authority.
4. Out-of-sample, walk-forward and multiple-testing control for the pooled cross-sectional hypothesis (gaps-5), plus a discovery → qualification handoff (gaps-11).
5. Multiplicity control across separate research declarations (gaps-12).
6. Multi-day holding (gaps-9).
7. Browser and API visibility of the authoritative V6 results (gaps-10).

## Hot-path table

Not applicable. This lane checks reachability and gaps, not O(1). No hot path was re-measured.

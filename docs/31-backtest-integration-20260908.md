# Backtest integration verification — 8 September 2026

**The complete historical sweep is not cleared yet.** The existing Backtest
page now connects a saved search setting to its saved strategy tester.
The launch, audit and recovery-safety connections are implemented. The current
functional gates passed; the unmet release conditions below still decide
whether the complete campaign can start.

The current Rust verification candidate D is the isolated, clean local commit
`81ff30959a1acd9a0823179d4964b62b9c61e62a`. It contains an exact copy of the
shared source at capture time and its own Git objects; no commit environment
override was used. Frontend follow-up repairs have separate current-source
logs. The shared checkout includes substantial pre-existing work, so its total
Git diff is not a count of changes made by this review.

The final click-path review then found that the main button launched the AND
range path, while the declared AND/OR/NOT campaign had only saved-result
inspection in the page. The new launch bridge and frontend mode are now
implemented and passed D's combined native functional checks. The final
frontend build is `5de2914c51716737`, including the later bounded-read recovery
fix. Candidate C (`bbf60cda5c0f26b43ca635007dadfced9132d1b1`) remains an earlier
baseline; its partial coverage cannot certify D's later launch changes.

## Current comparison

| Area | Status | Working evidence | Extreme cases checked | Limit before full clearance |
|---|---|---|---|---|
| Existing Backtest strategy tester | Verified | A selected saved search setting opens training/later totals, entry and exit rules, trades, trading days and every saved policy check in the existing page | Wrong setting, source, checkpoint, child completion, direction, period, refreshed parent and stale responses are refused or cancelled | The current inspection services do not enable a production sweep. The later period is part of selection, not an untouched final test. |
| Launch AND, OR and NOT from Backtest | Limited | The declared-search mode and audited Rust bridge passed the fresh native gates. All 603 frontend tests, type checking and the production build passed; the matching browser displays the eight-timeframe launch controls and named missing configuration | Exact IDs, all selected symbols, wholly later dates, changed policy, repeated submission, reload, edited controls and a resumed search with the same ID | The private viewer deliberately disables work and has no campaign receipt configuration. A private empty-input dispatch recorded refusal; that is not a historical sweep. No full sweep is cleared. |
| Malformed launch requests | Verified | The frozen D public parser passed 10,340 cases: nine exact accepted declarations and 10,331 refusals, with no unexpected outcomes or panics | All 153 pairs of the 18 fields, each with 64 malformed-value combinations; duplicate and escaped keys, bad spans, aliases, unknown controls, numeric boundaries and requests up to 65,536 bytes | Pairwise cases do not exhaust higher-order combinations. An early refusal can mask another invalid field; parser success does not establish data admission or worker completion. |
| Busy-page loading and status recovery | Verified | The actual page recovered from an injected first 429 on each results/status read and then displayed unmodified native responses; the busy notice was visible and cleared after recovery | Bounded retries, server minimum delays, repeated overload, generic evidence refusals, cancellation, late replies and 13 malformed status shapes on each of the two readers | Only GET reads retry. Unknown submissions are never reposted. HTTP success alone cannot establish idle state or complete work. These are finite transport checks, not a production latency benchmark. |
| Actual saved RELIANCE observations | Verified | Browser inspection of setting 480 shows 1,395 training trades and 1,947 later trades; pessimistic totals are minus INR 1,593.40 and minus INR 1,865.40 per unit | Real saved observations remain visible even when required acceptance evidence is refused | These are observations across a particular setting, before costs. They are not unique market executions across every setting, account returns or an approved strategy. |
| Exact page retries | Verified | A failed nonzero page retains its complete original request when retried | Both periods, trades and sessions, offset 32 and offsets above JavaScript's exact-number limit | Each request loads at most 32 rows. An unopened page is not certified by a summary. |
| Missing data and recorded zero | Verified | A recorded zero remains a valid zero; a failed read has no replacement result body | Missing facts, malformed pages, invalid joins, aborted loads, zero trades and unknown VWAP inputs | Absence of evidence never becomes a zero or an admitted outcome. |
| Protection against incorrect indicator names | Verified | Names appear only when both saved exit grids match the loaded vocabulary build | Missing, null, sparse and foreign grids; NOT of an unavailable VWAP condition | On the verification page, older settings retain numeric conditions because their vocabulary does not match. A current vocabulary cannot rename an older result. |
| Durable command and request history | Verified | Separate append-only records for CLI commands, browser tasks and selected HTTP reads/controls; the exact saved HTTP outcome survived an actual private-server restart | Busy index, corrupt bytes, torn writes, repeated terminals, missing terminal, panic, cancellation and reopen | Fixed-size records still require storage barriers. Completion of an HTTP response or command does not mean a full search completed. |
| Exact invocation IDs | Verified | IDs are preserved as decimal strings in live progress, polling, links and history pages | Adjacent IDs above 2 to the power 53, legacy low IDs, overflow, invalid cursors, duplicate or unordered records | The bounded reader verifies the index and first/last record; it does not claim to audit all middle progress records. |
| Safeguards when recovery history is missing | Verified | A missing activated journal cannot be silently recreated; separate prepared successors preserve the original scope and shared retry budget | Missing/replaced path, torn journal, repeated preparation, wrong predecessor or generation, restarted process | The original missing file has not been recovered. A successor does not reconstruct its completion. Selected sweep data validity is a separate check. |
| Safe recovery preparation | Verified | 61 recovery tests plus the authentic 34,440-window identity check passed; production recovery files stayed unchanged | Preparation cannot activate a worker, clear STOP, reset attempts, read bars or contact a vendor | No live successor was prepared or activated during this verification. |
| Intraday execution | Limited | Shared execution uses real stored OHLCV, integer prices and the fixed 15:10 IST exit rule; the current workspace rerun includes its clock and foreign-slice tests | Missing deadline minute, coarse signals, off-grid timestamps, next-day and foreign-cache data | OHLCV gives the recorded final one-minute interval, not a tick-exact fill. No synthetic historical result is substituted. |
| AND, OR, NOT and candle conditions | Limited | The retained engine and expression tests cover truth/availability, exhaustive small-space oracles, bounded continuation and candle exemplars | Unknown negation, empty frontier, invalid bit positions, resumed searches and worker limits | Boolean conditions on a bar are not an ordered-event language. Finite test oracles do not enumerate every possible market history. |
| Institutional comparison | Limited | Saved settings retain all 39 common policy values and 44 individual check outcomes, with shared search-wide probability limits | Empty families, small samples, concentration, missing statistics, refused evidence and later-period exit retuning | Boolean/cash research does not have the separate Selection V6/global execution authority. No global cross-study testing budget or untouched final holdout is established. |
| Strongest 0.1 percent of setups | Not cleared | The requested target is retained as a selection objective; failed candidates are not promoted to fill a ranking | A best-ranked loser, a tiny lucky sample or an empty admitted set cannot establish a profitable edge | Complete-search percentile ranking and consistently profitable performance are not demonstrated. Costs remain excluded as requested. |
| Automatic comparison and repeatable checks | Verified | This table is generated from this dated report. The Rust verifier now runs the entire frontend test glob and production build as well as workspace gates | New frontend regression files cannot be silently left out by a hand-picked test list; missing report fields refuse generation | Regeneration projects recorded evidence. It does not turn a dated review into continuous deployment monitoring or full coverage. |
| Rust boundary | Limited | The D extension audit passed 542 tracked/nonignored paths outside web, including 478 Rust files; all selected API and CLI source files match the shared workspace | Wrong extensions and paths outside the frontend exception are refused | Selected transitive build scripts still launch compiler probes. The literal no-external-process build-script requirement remains unmet; no conflicting dependency patch was installed. |
| Current functional verification | Verified | Candidate D passed formatting, strict workspace Clippy, dependency-policy checks and 5,212 tests across 99 targets; 11 tests were ignored. The final frontend passed 603 tests, type checking and its production build | Combined checks include launch admission, status, durable boundaries and the existing engine, execution and store suites; nested child test counts are excluded | Ignored tests are not passes. Functional success does not clear the full coverage, mutation, dependency-rule or activation conditions. |
| Full coverage and mutation closure | Not cleared | The superseded C instrumented run was stopped after its ladder test exceeded an hour; available diagnostic profiles are retained. All 10 selected audit mutations were caught after two boundary-test repairs | Reserved IDs, final-page cursors, duplicate terminals, page bounds, ordering and dispatch | C coverage is incomplete and cannot certify D. Only 10 of 249 audit-module mutations were exercised; the 46,386-workspace census is enumeration only. Full touched-crate line/branch and mutation closure remain unmet. |
| Speed, memory and customer scale | Limited | Fixed masks, finite pages and allocation limits reduce repeated work; jobs share the current 14-core, 48-GiB MacBook | Resource ceilings, overload, interrupted work and storage failures remain visible; instrumented ranking activity was sampled directly | Two test threads do not cap internal pools to two CPUs. Total search, retained history, disk latency and millions of customers have no O(1) guarantee. The disconnected 8-TB disk adds no current capacity. |

## Evidence and scope

The current source-built verification uses **8092** with a private copy of saved
research. It serves the existing Backtest design and actual D production
readers, with sweep/pull controls disabled. Its 15 private-viewer guards passed.
The listener was stopped during each frontend rebuild and restarted afterwards.
The **8080** main inspector was restarted with the same binary and settings
after its old static-file list refused the rebuilt frontend's script with 403;
the same script returned 200 afterwards. Its cached native source remains
unverified, distinct from D. The **8081** saved-results viewer was not changed.
The earlier C browser evidence records frontend version
`792880fe811a6ab7`; the current D reader serves frontend `5de2914c51716737`.
This comparison does not activate a production sweep. At the final browser
check the private viewer had no strict input receipt configuration or selected
campaign scope; it exposed those missing prerequisites and disabled launch.

The following are local retained evidence directories, not links to a broker
or trading service:

- [Combined source and frontend logs](../target/sweep-audit-20260908/source-gates/).
- [Durable invocation checks](../target/sweep-audit-20260908/audit-closure/implementation-notes.md).
- [Recovery checks and unchanged-file evidence](../target/sweep-audit-20260908/recovery-closure/README.md).
- [Research acceptance audit](../target/sweep-audit-20260908/research-acceptance/assessment.md).
- [Dependency boundary assessment](../target/sweep-audit-20260908/dependency-closure/assessment.md).
- [Actual browser pages and durable restart proof](../target/sweep-audit-20260908/browser-closure/browser-verification.md).
- [Independent final production-source and archive comparison](../target/sweep-audit-20260908/browser-closure/final-source-provenance.md).
- [Actual native-to-frontend launch contract](../target/sweep-audit-20260908/launch-closure/native-frontend-contract.json).
- [Fresh D test totals and exact source comparison](../target/sweep-audit-20260908/source-gates/workspace-d-summary.md).
- [Finite parser attack matrix](../target/sweep-audit-20260908/launch-closure/parser-adversarial/summary.md).
- [Current browser and bounded-read recovery](../target/sweep-audit-20260908/browser-launch-d/verification.md).

Tests use adversarial fixtures to exercise failures. The displayed market
observations above come from the retained real-OHLCV research archive. Those
two forms of evidence must remain distinct. No new full historical sweep,
vendor pull, live order or production service activation is part of these
verification results.

The incomplete C coverage diagnostic also exposed an exporter compatibility
problem: automatic discovery omitted nightly test executables in their newer
output directories. Re-exporting the retained profiles with 89 byte-distinct
native objects included all 62 started targets and all 13 crates. LLVM still
reported 2,916 functions with mismatched data; the interrupted and later unrun
tests remain missing. The resulting counts are diagnostic, not a coverage
approval for either C or D. See the [coverage measurement and limits](../target/sweep-audit-20260908/coverage-c/measurement.md)
and [final incomplete diagnostic receipt](../target/sweep-audit-20260908/coverage-c/README.md),
including the [explicit-object diagnostic table](../target/sweep-audit-20260908/coverage-c/exact-unique-objects/coverage-summary.md).

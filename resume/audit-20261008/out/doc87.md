# doc87 triage (PARTIAL: paused by the coordinator)

Tree: /home/claude/integ at a7a27dc3, read only. 36 of 87 ids classified; 51 NOT-YET-TRIAGED (listed at the end).

## Counts per verdict (classified so far)

- FIX: 6
- KEEP-MEASURED: 0
- KEEP-MEASURE: 6
- ALREADY-FIXED: 6
- OVERLAP: 18
- UNVERIFIED: 0
- NOT-YET-TRIAGED: 51

Note: KEEP-MEASURED is 0 so far because the D-2290 kept-cost figures come from an uncommitted scratch harness (docs/06-limits.md:16700); those rows are OVERLAP with defp rnew-3.

## WP-api

ALREADY-FIXED 5; FIX 6; KEEP-MEASURE 4; OVERLAP 5;

1. **BD-5** (FIX, S). Plan: Say a closed window's pending suppressed count on the next api.request of any status and once at shutdown (before the exit sync intu adds); keep the ration. Test: Flood 60 cross-site failures then one 200: exactly one Warn summary with suppressed=10; a pending count is written at shutdown. Files: crates/api/src/logs.rs.
2. **BD-6** (FIX, S). Plan: Refusing handlers attach their refusal sentence as a response extension; note_request copies at most 256 bytes of it into the api.request event as why. Test: A 503 from a refusing route logs api.request with why equal to the body's sentence; a 10 KB reason is cut to 256 bytes. Files: crates/api/src/logs.rs; crates/api/src/server.rs.
3. **BD-11** (FIX, S). Plan: Retry telemetry::install at most once per named interval (e.g. 60 s) from the request path and write one Error naming the gap once it opens; refusing to start instead is an owner choice: UNVERIFIED which the owner wants. Test: Unwritable log dir at start gives No sink; make it writable; after the interval the next request installs and /logs shows a sink and a gap event. Files: crates/api/src/server.rs; crates/api/src/logs.rs.
4. **api-assets** (FIX, M). Plan: Hold content-addressed _app/immutable files in memory after the first read (bounded by the build's file count); memoise the shell and version.json under a stat stamp; only the response copy stays O(file). Test: Count reads over 1,000 GETs of one immutable asset = 1; a rewritten shell is re-read; then api::latency p50/p99/max/n/load. Files: crates/api/src/assets.rs; docs/06-limits.md.
5. **api-boolean-evidence-cold** (FIX, M). Plan: Keep K readers per route (named constant, LRU, exact key), as D-4434 did for trade readers; the first cold hash per key stays and gets measured. Test: Alternate two identities for 100 unpinned first pages: 2 cold opens, not 100; api::latency cold open at 3 body sizes. Files: crates/api/src/booleanjson.rs; crates/api/src/booleanevidencejson.rs; crates/api/src/detail.rs.
6. **api-index-stop-cold** (FIX, S). Plan: Route indexstopjson (and booleanoosjson) through detail::must_admit so an unpinned first page reuses a current held reader. Test: 10 unpinned first pages on unchanged evidence: 1 cold open; a moved generation forces a cold open. Files: crates/api/src/indexstopjson.rs; crates/api/src/booleanoosjson.rs.
7. **api-candidate-trades** (KEEP-MEASURE, S). Plan: Extend the fixture with trading candidates and larger catalogs. Test: api::latency: cold TradeReader::open at 10^2/10^4/10^5 trades and cold summary_for at 10^3/10^5 candidates, n>=200, p50/p99/max/load. Files: crates/api/src/candidatejson.rs; docs/06-limits.md.
8. **api-boolean-campaign** (KEEP-MEASURE, S). Plan: Time render at checkpoint counts up to MAX_CHECKPOINTS=1,024 (fixture may need cli's builder, as D-1444 notes for evidence trees). Test: api::latency latency_boolean_campaign_by_checkpoints at 1/100/1,023 checkpoints, n>=100. Files: crates/api/src/booleancampaignjson_tests.rs; docs/06-limits.md.
9. **api-logs** (KEEP-MEASURE, S). Plan: Time one /logs.json request with both halves at the 4 MiB cap and a no-match filter. Test: api::latency latency_logs_json_at_the_scan_cap, n=200. Files: crates/api/src/logs.rs; docs/06-limits.md.
10. **api-json-memos** (KEEP-MEASURE, S). Plan: Time each cold build. Test: api::latency cold instruments_json/calendar_json/store filter/indexmap_json at U about 2,780 and E 15,857 and 93,776, n>=50. Files: crates/api/src/server.rs; docs/06-limits.md.
11. **crate-spawns-production** (ALREADY-FIXED). D-4430 (docs/05-decisions.md:67571, commit 40669ba3): opening is opt-in with BRUTEX_OPEN=1; default spawns nothing and prints the address
12. **api-bars-json-past-end** (ALREADY-FIXED). D-4432 (docs/05-decisions.md:67602, 2d38c60a): a from past the header's last stamp costs one record read in either month kind
13. **api-bars-window-sorted** (ALREADY-FIXED). D-4439 (f2324b9c): ts+extremes use kept month folds; non-ts sort refused past MAX_SCAN_WINDOW_RECORDS=1,048,576 (named constant). Its test fix still needs a rerun (fxb1-pause; api validation is intu's)
14. **api-verify-json** (ALREADY-FIXED). D-4435 (7deb5b6f): one page of at most MAX_VERIFY_PAGE=1,024 opens; newest-entry list memoised per snapshot. Test fixes still need a rerun (fxb1-pause)
15. **api-engine-top-refusal** (ALREADY-FIXED). D-4433 (c41d1ad5): a persistent refusal costs one record read, not a cold walk; a repair is seen at once
16. **W1-api5-1** (OVERLAP). defp rnew-3 (bench the D-2290 kept costs: Manifest::load). D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700), so not KEEP-MEASURED
17. **W1-api5-2** (OVERLAP). defp rnew-3 (census miss = Manifest::load per manifest). D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700), so not KEEP-MEASURED
18. **api-census-now** (OVERLAP). defp rnew-3: a miss is Manifest::load, timed only by the scratch harness. D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700), so not KEEP-MEASURED
19. **W1-api3-1** (OVERLAP). defp rnew-3: the kept manifest re-read cites D-2290 Manifest::load figures. D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700), so not KEEP-MEASURED
20. **W1-api2-1** (OVERLAP). defp rnew-3: calendar derivation cites 'same month walk' (docs/06-limits.md:16710). D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700), so not KEEP-MEASURED; rnew-3 must time calendar_of::derive itself, not reuse the committed_cash_days walk

## WP-pull-store

OVERLAP 9;

1. **W1-pull2-0** (OVERLAP). defp rnew-3 (Manifest::load per window). D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700), so not KEEP-MEASURED
2. **W1-pull2-6** (OVERLAP). defp rnew-3 (Manifest::load per rolling answer). D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700), so not KEEP-MEASURED
3. **pull-read-census-per-body** (OVERLAP). defp rnew-3 (Manifest::load per body). D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700), so not KEEP-MEASURED
4. **W1-pull2-5** (OVERLAP). defp rnew-3 (per-record committed_cash_days). D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700), so not KEEP-MEASURED
5. **W1-pull2-3** (OVERLAP). defp rnew-3; row says 'same month walk as above' (docs/06-limits.md:16709-16710), a proxy: derive_all's rerun re-folds too and needs its own timing. D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700), so not KEEP-MEASURED
6. **pull-json-decode** (OVERLAP). defp o1api-33 (typed/streaming decode or NOT-FIXABLE with reason); memory multiple counted by a_json_decodes_peak_memory_is_measured_against_its_body (docs/06-limits.md after :16717)
7. **R9-csr-o1-0** (OVERLAP). defp rnew-3 (tail-block-with-fstat cold read). D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700), so not KEEP-MEASURED
8. **store-committed-cash-days** (OVERLAP). defp rnew-3 (committed_cash_days walk). D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700), so not KEEP-MEASURED
9. **store-time-lookup-bisection** (OVERLAP). defp rnew-3 (legacy bisection p99) and intl rnew-2 (the D-2290 row still says 'an index file would be a new store format version', docs/06-limits.md:16706). D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700), so not KEEP-MEASURED

## WP-lower

ALREADY-FIXED 1; KEEP-MEASURE 2; OVERLAP 4;

1. **BD-1** (KEEP-MEASURE, S). Plan: Commit a bench row: emit vs emit+sync_data per event. Test: C-T bench row, n=10^4 events, p50/p99/max with load average. Files: crates/telemetry/benches/ratio.rs; docs/06-limits.md.
2. **BD-2** (KEEP-MEASURE, S). Plan: Test that writes 3x the bound and asserts the set never exceeds DEFAULT_MAX_FILE_BYTES x DEFAULT_KEEP_FILES plus one event; time a rotation. Test: telemetry test the_retained_set_never_exceeds_its_bound; rotation p50/p99/max, n>=100. Files: crates/telemetry/src/sink.rs; crates/telemetry/benches/ratio.rs.
3. **W3-engine1-2** (ALREADY-FIXED). D-4480 (368a103d): keep::Best removed; D-2290 row struck
4. **engine-level-sort** (OVERLAP). intl (fxd WIP 2cfa70ae: W3-engine1-1, D-4483, FXD-08 measures the level sort)
5. **rule4-mask-eval-width** (OVERLAP). intl (fxd WIP 2cfa70ae: so1-1, D-4481, O1P row for Column::support per bar)
6. **vocab-expression-evaluate** (OVERLAP). intl (fxd WIP 2cfa70ae: o1engine-22, D-4484)
7. **vocab-cursor-advance** (OVERLAP). intl (fxd WIP 2cfa70ae: o1engine-23, D-4485)

## NOT-YET-TRIAGED

- WP-api (12): BD-7, api-audit-page-fsync, api-autopilot-json, api-bounded-routes, api-dashboard, api-health, api-instruments-page, api-instruments-search, api-pull-posts, api-run-json-poll, api-trades-json, api-universe-resolve
- WP-pull-store (16): ET-bars-candles-store-1, ET-bars-candles-store-1/-8, ET-bars-candles-store-8, rederive, pull-held-series, pull-manifest-append, pull-manifest-entry, pull-vendor-pull, BD-12, store-append, store-checksum-block, store-tix-rebuild, lake-open-decode, BD-10, core-universe-membership, core-vendor-decode
- WP-lower (23): ET-o1-proof-coverage-2, W3-engine1-0, engine-join-pair, engine-subset-prune, engine-sweep-total, rule4-condition-lookup, vocab-name-index, BD-4, BD-3, BD-8, telemetry-emit, telemetry-filtered-tail, BD-9, cli-research-replays, cli-results-ledger, cli-results-top, cli-screen-select, cli-span-reloads, p99-absent-benches, deps-build-scripts-spawn-rustc, deps-inline-asm, deps-native-code-other-targets, workflow-interpreters-web-job

# Audit helpers (workstream 5.6) — resume state, 2026-10-04

Read-only helpers for the zero-findings loop. Never edit code; findings go to the zero-findings thread.
Audited head: final/all-fixes-zero **1f4de71** (merges zero/api-routes, zero/cli-edges-2, zero/numeric, #74 c97ff00).

| Angle | Pass | Prior items re-verified | New findings | Report |
|---|---|---|---|---|
| Crash/edge | 5 | CE-1..41: 36 fixed, 2 partial (CE-19, CE-41), 2 not fixed (CE-9, CE-20), 1 wrong fix (CE-14) | CE-42 med, CE-43 HIGH, CE-44..46 low | crash-edge-pass5.md |
| Tests/docs/security | 4 | 76 open: 54 fixed, 22 not fixed | P4-01 med (gate 10 red: 3 docs/04 rows cite renamed tests), P4-02 low | tests-docs-security-pass4.md |
| Concurrency | 4 | 25 cited: 20 fixed, 5 partial; pass-3 6/6 addressed | conc4-1 med, conc4-2 low | conc-pass4-ledgers-locks.md |
| Numeric/O(1) | 4 | 41: 14 fixed, 1 partial, 26 not fixed (pst-3, clib-1, clib-2 held for user) | p4num-1 low, p4num-2 low | numeric-pass4.md |

All new findings were sent to the zero-findings thread (session_01GS9PBjP9nShhuEHrx1N2ia) on 2026-10-04.
Crash/edge pass 6 done: CE-47..49 medium (Rust JSON vs web validator drift, ran in Node), CE-50, CE-51 low; sent. Correction to pass 6: CE-42 is still NOT fixed (unpriced arm frontier.rs:3348-3352 uses Verdict::default()).

## Next
Every angle found something new, so each runs another pass on the next zero staging head: re-verify open
rows (FIXED / NOT FIXED / PARTIAL / WRONG FIX) and review the fix diff. Stop an angle when a pass finds nothing new.
Note: non-root cargo via setpriv fails in the cloud box (/root, which holds the toolchain, is mode 700).

## Round 2 (same head 1f4de71, new themes) — done 2026-10-04 ~08:20 UTC
| Angle | Pass | Theme | New findings | Report |
|---|---|---|---|---|
| Crash/edge | 7 | calendar and session edges | CE-52 med (outage-day reopening dropped by ingest), CE-53 med (Muhurat-only holiday accepted as expiry), CE-54, CE-55 low | crash-edge-pass7.md |
| Concurrency | 5 | side files, failed barriers | conc5-1, conc5-2 low | conc-pass5.md |
| Numeric | 5 | denominators, refused stats shown as numbers | p5num-1 med (calendar gate only on first `top` rows), p5num-2..5 low | numeric-pass5.md |
| Tests/docs/security | 5 | invariant rows vs code, weak tests, new inputs | P5-01 med (gate 10 fails earlier on RS-08, ZR-44), P5-02..07 low | tests-docs-security-pass5.md |
All sent to the zero-findings thread. Round 1 + 2 total: 34 new findings. Next: round 3 when the zero head moves (re-verify all open rows), plus new themes if it does not.

## Round 3 (same head 1f4de71, new areas) — done 2026-10-04 ~08:50 UTC
| Angle | Pass | Area | New | Report |
|---|---|---|---|---|
| Crash/edge | 8 | vendor parsers, web under bad replies | CE-56 med (duplicate JSON key in live Dhan rolling read), CE-57..60 low | crash-edge-pass8.md |
| Concurrency | 6 | api state machines | conc6-1..4 low | conc-pass6.md |
| Numeric | 6 | costs, greeks, indicators | p6num-1, p6num-2 low (greeks, indicators clean) | numeric-pass6.md |
| Tests/docs | 6 | Rust-only, gate replay, graph, vocab | P6-01..03 med (gates 1d, 11, 12 red at 1f4de71; green at #74 73441e5), P6-04 low | tests-docs-security-pass6.md |
Rust-only: clean across 982 files. Owner question left open by D-1602: CI shell/awk vs CLAUDE.md §2 "no interpreted runtime as a tool".
Total new across rounds 1-3: 49.

## Round 4 (same head 1f4de71) — done 2026-10-04 ~09:30 UTC
| Angle | Pass | Area | New | Report |
|---|---|---|---|---|
| Crash/edge | 9 | store/lake on-disk bytes | CE-61 med (forged header count aborts api server, ran), CE-62, CE-63 low | crash-edge-pass9.md |
| Determinism | 7 | run identity, idempotence | conc7-1, conc7-2 low (identity hash itself sound) | conc-pass7.md |
| Engine | 7 | Apriori + O(1) ops | p7num-1..3 low (join proven complete and injective) | numeric-pass7.md |
| House rules | 7 | empty tests, fallbacks, float money | P7-01, P7-02 low | tests-docs-security-pass7.md |
Total new across rounds 1-4: 59. Severity trend: round 4 = 1 medium, 9 low.

## Round 5 (same head 1f4de71) — done 2026-10-04 ~10:05 UTC
| Angle | Pass | Area | New | Report |
|---|---|---|---|---|
| Crash/edge | 10 | counts sizing memory/loops | CE-64 med (FIFO/device census read hangs or OOMs the api, ran), CE-65, CE-66 low | crash-edge-pass10.md |
| Network | 8 | pull clients, retries, credentials | conc8-1 med (5xx breaker can never trip, ran), conc8-2..4 low | conc-pass8.md |
| Statistics | 8 | RC, SPA, Romano-Wolf, bootstrap vs literature | p8num-1 med (n>=30 normal bar overspends Bonferroni alpha up to 1,322x) | numeric-pass8.md |
| Docs vs behaviour | 8 | 35 cli commands, 50 routes | P8-01..05 low | tests-docs-security-pass8.md |
Total new across rounds 1-5: 72. Each round still finds 1-3 mediums, so rounds continue.

## Round 6 (same head 1f4de71) — done 2026-10-04 ~13:00 UTC
| Angle | Pass | Area | New | Report |
|---|---|---|---|---|
| Crash/edge | 11 | text in/out, unicode, str slicing | CE-67..69 low (no injection or panic route) | crash-edge-pass11.md |
| Concurrency | 9 | telemetry/logging pipeline | conc9-1 med (out-of-order progress marks healthy runs failed, ran), conc9-2, conc9-3 low | conc-pass9.md |
| Trading | 9 | entry/hold/exit model | p9num-1..3 low (pessimistic figure correct) | numeric-pass9.md |
| Law vs code | 9 | CLAUDE.md claims, charter sources | P9-01..04 low (21/23 claims hold) | tests-docs-security-pass9.md |
Total new across rounds 1-6: 85. Owner items added: P9-02 (no source for lot sizes/strike steps/expiry weekdays), P9-03 (Groww cap).
Zero thread at 09:30 had local fix branches (zero/edges-3, ci-web, calendar, data-edges, conc-api, conc-data) not yet on staging; re-verify when it pushes.

## Round 7 (same head 1f4de71) — done 2026-10-04 ~12:40 UTC
| Angle | Pass | Area | New | Report |
|---|---|---|---|---|
| Crash/edge | 12 | web front end's own logic | CE-70 med (holding chart wrong interval, daily key has no year), CE-71..73 low | crash-edge-pass12.md |
| Concurrency | 10 | multi-stage ledger chain | conc10-1 med (store-root ledgers have one lifetime 2^24-row cap, ran), conc10-2 low | conc-pass10.md |
| Data shaping | 10 | folding, paisa, OI | p10num-1 med (GDFL LTQ=0 quote rows folded into bar prices), p10num-2 low | numeric-pass10.md |
| Test strength | 10 | 50 hand/CM mutations | P10-01 med (GST-rate mutant survives, ran), P10-02 med (lane-order mutant), P10-03..07 low | tests-docs-security-pass10.md |
Total new across rounds 1-7: 100. Round 7 = 5 medium, 10 low. Re-checked still open: ledgerall-1, pop2-7, ledgerv6-1.

## Round 8 (same head 1f4de71) — done 2026-10-04 ~13:30 UTC
| Angle | Pass | Area | New | Report |
|---|---|---|---|---|
| Crash/edge | 13 | arithmetic and panic routes | CE-74 low (cross-vendor row sum overflow aborts api) | crash-edge-pass13.md |
| Concurrency | 11 | filesystem failure (ENOSPC, EIO, EMFILE, dir fsync) | conc11-1..3 low | conc-pass11.md |
| Indicators | 11 | condition bit semantics vs standard formulas | p11num-1 med (gapped day dropped before evaluator shifts later trend bits, ran), p11num-2..4 low | numeric-pass11.md |
| Security | 11 | api, credentials, supply chain, history | P11-01 med (cross-site bar reads make pull writes fail Locked, ran), P11-02..04 low | tests-docs-security-pass11.md |
Total new across rounds 1-8: 112. Round 8 = 2 medium, 10 low. P10-02 confirmed by run. Owner item added: P11-04 (personal paths/emails in docs and commit history; history rewrite is the owner's call). Newly fixed: hunt-ci-12, locks-1.

## Round 9 (same head 1f4de71) — done 2026-10-04 ~14:15 UTC
| Angle | Pass | Area | New | Report |
|---|---|---|---|---|
| Crash/edge | 14 | operator inputs: cli args, config, env, POST bodies | CE-76 med (instrument case breaks rerun idempotence, ran), CE-75 low (grid rungs clamped silently, ran) | crash-edge-pass14.md |
| Concurrency | 12 | kill and resume | conc12-1 med (weekend/holiday month ends never complete; restart re-fetches, ran) | conc-pass12.md |
| O(1) | 12 | cost of each op outside the engine | p12num-1 low (frontier/detail ledgers re-indexed every run) | numeric-pass12.md |
| Invariants | 12 | docs/04 rows vs tests | P12-01..08 low | tests-docs-security-pass12.md |
Total new across rounds 1-9: 124. Round 9 = 2 medium, 10 low. Newly fixed: CE-3, CE-23, W2-cli16-2 (as docs).

## Round 10 (same head 1f4de71) — done 2026-10-04 ~14:45 UTC
| Angle | Pass | Area | New | Report |
|---|---|---|---|---|
| Crash/edge | 15 | api to web JSON contract | CE-77 med (census-degraded header ignored), CE-78 med (autopilot journal_error never shown), CE-79..83 low | crash-edge-pass15.md |
| Concurrency | 13 | logging/monitoring coverage of failures | conc13-4 med (autopilot/pull halts never logged), conc13-5 med (store trace events name only a hash), conc13-1..3, 6..8 low | conc-pass13.md |
| Costs | 13 | cost rates vs charter | p13num-1..5 low (all rates UNVERIFIED = hunt-costs-5) | numeric-pass13.md |
| Rust-only | 13 | §2 every corner | P13-01 med (committed web/build stale; W1 red), P13-02..05 low | tests-docs-security-pass13.md |
Total new across rounds 1-10: 149. Round 10 = 5 medium, 20 low. CI tools list for owner question D-1602 is in tests-docs-security-pass13.md.

## Round 11 (same head 1f4de71) — done 2026-10-04 ~15:20 UTC
| Angle | Pass | Area | New | Report |
|---|---|---|---|---|
| Crash/edge | 16 | time and clock | CE-84..87 low (wall clock used for intervals/ages) | crash-edge-pass16.md |
| Concurrency | 14 | operations that clash (pair matrix) | conc14-1, conc14-2 low | conc-pass14.md |
| Metrics | 14 | ranking and descriptive metrics | p14num-1 med (live "won %" inverted for sell rows, ran), p14num-2, p14num-3 low | numeric-pass14.md |
| Docs | 14 | docs/02 store format + docs/01 vs code | P14-01 med (census header vendor bytes wrong, ran), P14-02 med (CRC sidecar layout undocumented), P14-03..06 low | tests-docs-security-pass14.md |
Total new across rounds 1-11: 164. Round 11 = 3 medium, 12 low. Newly fixed: xcut-1, run1-3, P1-16-01/02/04..08, P1-14-02.

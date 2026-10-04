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

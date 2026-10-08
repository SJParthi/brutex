# Lane 1-b verify and finish — resume state (thread started 2026-10-08)

Owner ask (2026-10-08): "Redo the 55 missing lane 1 fixes and push them onto final/all-fixes (PR #74)."

## Fact found first
The 55 were NOT missing: the redo on `final/all-fixes-00bbns` (ae479d2) and follow-ups D-2100..D-2106
(last ac0802f) are ancestors of PR #74 head fbdabaec. RESUME-20261003.md predates that merge.
So this thread re-verified every id on fbdabaec and is fixing what is not fully fixed.

## Verification on fbdabaec (static, 5 checkers; reports G1..G5.md here)
56 ids (55 NOT-FIXED + W2-cli8-6 PARTIAL from v1.md): 33 FIXED, 6 FIXED-BOUNDED, 13 PARTIAL,
4 DOCUMENTED-ONLY, 0 NOT-FIXED, 0 REGRESSED; ~25 new small defects (G1-1.., G2-1.., G3-1.., G4-1.., G5-1..).

## Fix lanes (local worktrees, branches lane1b/fx-a..e from fbdabaec; decision range D-4500..D-4599)
| Fixer | Items | D-numbers | Invariant ids |
|---|---|---|---|
| A | G1-1 pool-oos pass 1 order + sharing, G1-2, G2-4, W2-cli8-9/GAP13-13/GAP13-15 test gaps, G1-3, stretch R9-cli-o1-1 | D-4500..4515 | L1FA- |
| B | G2-5 recorded audit capture cost, W2-cli8-10, W2-cli8-11, W2-cli8-6+G2-3 census, G2-1, G2-2, W2-cli8-4/5/7 test gaps | D-4516..4531 | L1FB- |
| C | G5-2/W2-cli13-4 FIFO opens, G5-1, G5-3, G5-4, G1-4/R9-cli-law-1, G3-3, G3-5, G3-8, web zero ceiling | D-4532..4547 | L1FC- |
| D | G3-1/GAP12-6 Boolean CAS close, G3-4 identity reason codes, AC-whp-law-2, GAP15-21/G3-2 banners, G3-6, G3-7, G3-9, F-CEC7A0 | D-4548..4563 | L1FD- |
| E | W2-cli12-1, G4-1, W2-cli11-3, W2-cli11-2, W2-cli12-2 (population ledgers O(1)) | D-4564..4579 | L1FE- |
| F (queued) | W2-cli3-4 one writer per run, G4-4, W2-cli3-3, G4-2, G4-3, W2-cli7-2 decision | D-4580..4595 | L1FF- |

W2-cli7-2 default chosen by this thread: replay stays a full re-proof (a replay that trusts its own sealed
output proves nothing new); cost stated as the price of the proof. G4.md §A has the cheaper sealed-manifest
design if the owner prefers it.

## Landing
Do not push to final/all-fixes directly (every push cancels the long Gate 18 run). Merge the fixer branches
into `final/all-fixes-xp04wq`, validate (fmt, clippy, full tests as non-root, static gates, cargo-mutants on
the diff), push that branch, and send branch + sha to the PR 74 CI thread to land in its one combined push.
If #74 has merged, open exactly one new PR instead.

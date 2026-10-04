# Zero-findings thread (5.5) — save at 2026-10-04 13:45 UTC

Saved at the coordinator's usage warning (82% of the 5-hour limit). The tracker of record is
`/mnt/project-files/fix-board/status/zero-findings.tsv` (states: found, fixing, partial, branch,
doc, pushed, green, operator).

## Branches on origin (none has a PR; PR #74 stays the only PR)

| Branch | Head | What |
|---|---|---|
| `zero-work` | d6995f4d | Staging. Earlier merges (edges-3 part 1, ci-web, fixboard zero-web12, zero-num56), web harness fixes, and fixboard/zero-p10. Not yet re-gated since the last three merges. Becomes `final/all-fixes-zero` once gated. |
| `final/all-fixes-zero` | 1f4de71 | The last pushed, gated staging head. |
| `zero/edges-3` | 4817f9b9 | D-1771..D-1780, main thread. Tests run as non-root: pull, indicators, engine OK; api OK on the earlier run; runner had one failure fixed in this commit but not re-run; cli still unrun. |
| `wip/zero/calendar` | 71ea7588 | P-03 calendar drop filter (helper, may be mid-work) |
| `wip/zero/data-edges` | 59f6092e | p10num-1/2 (helper) |
| `wip/zero/network` | c5a21fb6 | P11-01, P11-03, conc11-2, conc12-1 (helper) |
| `wip/zero/numeric-edges` | aad8d88a | rep-1, p6num-1/2 (helper) |
| `wip/zero/conc-api` | c6d5a5df | conc11-3 (helper) |
| `wip/zero/conc-data` | 477ae866 | conc10-1/2, conc11-1 (helper) |
| `fixboard/zero-p10` | 7b2ffdd9 | P10 mutant kills, merged into zero-work |

Not yet on origin (helpers started 13:0x UTC, worktrees under /home/claude/wt/): `zero/docs04`
(P12-01..08, D-1790..1799), `zero/withheld-splice` (p11num-1, D-1781..1785),
`zero/web-contract` (CE-77-web..CE-83, D-1786..1789). If lost, re-dispatch from the finding files.

## Next steps, in order

1. Run the cli tests and `runner audit::` on `zero/edges-3`, then cargo clippy on cli, runner and pull, and `cargo test -p pull --test unit --test folder`.
2. Merge `zero/edges-3` into `zero-work`. Then `npm --prefix web run build` and commit web/build (P13-01: the bundle is stale). Then W1, W2, W3 and the `Gate 1e` run (rebuild the scanner first).
3. Merge helper branches as they report. Push `zero-work:final/all-fixes-zero` as a merge, never forced. Merge into `final/all-fixes` (PR #74).
4. Dispatch the open rows: P13-02..05, conc13-1..8, CE-84..90, p14num-1/2, conc14-1/2, P14-01..07, p15num-1/2, conc15-1..6, and the older found rows.
5. Owner items stay named: p13num-4 (Finance Act 2023 STT text), p14num-3 (same as run1-1/run1-2), P11-04, pst-3, clib-1/2, gaps-7.

Main-thread decision numbers used: D-1771..D-1780. Free: D-1800 onward per the D-number blocks in project memory.

# GDFL one-second engine build: resume state (Mac)

Saved 2026-10-04 13:42 UTC by the GDFL build thread (new account, Mac, Remote Control) as the 5-hour limit neared 93%. Weekly 45%+. Supersedes the 3 Oct 20:10 save. No GDFL data rows are in this file or on any branch pushed. All four part branches stay LOCAL on the Mac (never pushed).

## Heads at save time (worktrees under work-20260925/wt/G-<part>)

| Part | Branch | Head | Uncommitted files |
|---|---|---|---|
| census | feat/gdfl-census | 3593ca0d | 0 |
| core-second | feat/gdfl-core-second | 0272901e | 0 |
| store-grid | feat/gdfl-store-grid | b8ee88f9 | 0 |
| gdfl-cm | feat/gdfl-cm-reader | 786e1122 | 5 |

## Where it stands

- Round-3 reviews done, 9 of 9, with background Agent-tool workers (no Workflow run). Results: core-second clean except nits; store-grid 2 blocking (GS-01b shape test missed walks/defaults) + nits; gdfl-cm 2 gate failures (1d, 12), 1 should-fix, 1 surviving mutant (gdfl_cm.rs:906 && -> ||), nits; census repair r2 in progress. Repairs were committed on top (heads above); their full Definition-of-Done runs were still finishing.
- Coverage scope decided by the thread (4 Oct): each part is held to 100% line+branch on its OWN diff (nightly llvm-cov --branch); pre-existing main debt belongs to PR #74.
- New decision numbers for this thread: D-2800..D-2899. Taken: D-2800 (gdfl-cm reads the vendor zips; CmSource trait), D-2801 (tick-store CmSource, pure-Rust ruzstd), D-2802..D-2807 reserved for api/web wiring, D-2808 (per-second volume in SecondCell + grid record).

## Plan changes on 4 Oct (operator)

1. The extracted GDFL CSV folders are being deleted (proven byte-identical to the zips). The reader reads the vendor zip /Volumes/WD_BLACK/NSE (Stock+Indices)_01.09.2018 to 24.09.2026_Tick.zip directly (outer zip stores 4,209 day zips uncompressed; members are deflate), behind a CmSource trait. The census verb moves to the same source.
2. Tick store (built by the WD Black thread): /Volumes/WD_BLACK/brutex/tickstore-data, spec FORMAT.md v1 (BRTXTS01). It is CmSource implementation 2 (prioritised), so the zips can later move to the cloud.
3. GDFL must be viewable in the EXISTING Brutex app pages (no new pages). Wiring plan: work-20260925/state/design/gdfl-tick-view.md (no page shows ticks today; /db ATM/ITM/OTM lacks a spot source).
4. Per-second OHLCV with volume (no-LTQ ticks dropped), traceable to source ticks on /db; fills worst-case from seconds, only with enough volume; signals on minutes; exits via exit grids. Fit note: work-20260925/state/design/gdfl-1s-operator-20261004.md (volume-less indices cannot satisfy a volume gate: refuse by name).

## Next steps

1. Finish each part's repair and its full Definition of Done; then round-4 review (3 lenses) of each new head; repeat until clean.
2. gdfl-cm: zip source + CmSource, then tick-store source. census: read via CmSource, rerun and compare with run-b5ec8d7c. core-second + store-grid: add volume (D-2808).
3. Squash-integrate the clean parts into feat/gdfl-1s from origin/main; prove with git log -p that no GDFL row is in any commit. Trial merges show only doc-tail conflicts on main; against origin/final/all-fixes real conflicts in store (file.rs, layout.rs, unit.rs vs store v3), core/pull vendor.rs, pull csv.rs, ci.yml, 07-plan.md: merge final/all-fixes into feat/gdfl-1s locally and resolve.
4. Import into the existing pages per gdfl-tick-view.md; then code-only landing on PR #74 through its merge gate.
5. Usage: checked every 15 min; pause Claude work at 5-hour 93%, save + stop at 98% or weekly 93%.

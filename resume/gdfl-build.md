# GDFL build: resume here (saved 2026-10-05, weekly usage 95%)

Local file on the Mac, not tracked. The same state is on GitHub: branch `fix-queue`, file `resume/gdfl-build.md`.

## Paste this as the first message of a new Claude Code session (normal session, on this Mac)

```
Continue the Brutex GDFL build on my Mac. Read /Volumes/WD_BLACK/brutex/fresh-20260919/project/RESUME-GDFL-20261005.md
in full, then CLAUDE.md on origin/final/all-fixes. Work in /Volumes/WD_BLACK/brutex/fresh-20260919/project and the
worktrees under /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/. Start with "Next steps" step 1.
Rules: Rust only except web/; O(1) with measured p50/p99/max; fix every finding; real evidence only; never commit or push
a real GDFL data row; PR #74 is the only PR; never ask me to approve anything; save state to fix-queue before 93% weekly
usage and stop at 98%.
```

## Where everything is (all committed, worktrees clean)

| Part | Branch (local) | Head | Worktree | State |
|---|---|---|---|---|
| census (data-quality report) | feat/gdfl-census | fe171a39 | wt/G-census | Round-4 review fixed; all gates OK, 0 surviving mutants, 100% branch coverage. Zip read waits for integration. |
| one-second clock | feat/gdfl-core-second | 47c794cf | wt/G-core-second | Round-3 fixes + per-second volume (D-2808) committed; needs its final check run and a round-4 review. |
| one-second storage | feat/gdfl-store-grid | d3d0e964 | wt/G-store-grid | Round-3 blocking fixes (O(1) shape test) + 48-byte cell; needs final check run and round-4 review. |
| GDFL reader | feat/gdfl-cm-reader | e9c6388e | wt/G-gdfl-cm | Zip + tick-store sources (D-2800, D-2801); needs final check run and round-4 review. |
| **Import (priority)** | feat/gdfl-import | 3cc6b8e0 | wt/G-import | `cli gdfl-import`, 1-second only, D-2802..D-2806, attack-thread tests ported. Last run: only 3 machine-environment test failures (socket path length, /dev/full lock, non-UTF-8 filename on APFS); mutants run was in progress. Pushed copy for review: origin/feat/gdfl-import @ 8e6b5219 (older, code only). |
| /db spot for ATM/ITM/OTM | feat/db-spot | 5be14255 | wt/G-dbspot | D-2807, route /spot.json; needs final check run. |
| RBI 91-day T-bill rate | feat/rate-tbill | 0990a6a4 | wt/G-rate | D-2810, dated rate lookup for IV/greeks; series at work-20260925/state/rates/; needs final check run. |

Decision numbers for this work: D-2800..D-2899 (used: 2800-2808, 2810).

## Proof already in hand (work-20260925/state/)

- `import-scratch/PROOF.md`: 3 Aug 2026 imported into a scratch store. 11,524 option files, 0 refused, 27.6M ticks -> 3,430,829 one-second bars. Zip and tick store give byte-identical stores (35,202 files). 16/16 raw-tick spot checks equal. Re-run writes nothing. A 2018-12-03 day also imports with 0 refused.
- `name-census/`: every option name 2018-09-03..2026-09-24 (1,218,362 contracts) decodes one way, 0 refused, 0 trade after expiry. Monthly expiry table and the 2018-2019 trading-day list (NSE circulars 36475, 39612, 40536, MSD 42319) are here too.
- `design/store-lookup-p99-20261004.log`: store lookup by time was p99 104 us (1min month) / 562 us (1s month) before PR #74's new `.tix` time index.
- Screenshots of /db showing GDFL 1-second data: `import-scratch/db-gdfl-nifty-*.jpg`.

## Viewing the data now

A second copy of the app runs over the scratch store at http://127.0.0.1:8081/db (feed Global Datafeeds, timeframe 1s; NIFTY spot, and NIFTY options under segment "Expired options" with an expiry chosen). Your normal app at :8080 reads your real store, which has no GDFL yet. Known gaps on /db: the Time column shows only HH:MM for 1-second bars (needs a seconds display); moneyness/IV/greeks need feat/db-spot and feat/rate-tbill.

## Agreed design (operator, 4-5 Oct)

- GDFL supplies 1-second data only; minutes come from Zerodha 1-minute pulls and the app derives higher timeframes.
- The underlying drives entries; fills, slippage and costs use the option's own 1-second bars: worst price of the next traded second, only if its volume covers the order (index spot has no volume check).
- Stocks and options: bars from rows with traded quantity only; volume = sum of LTQ; OI = last OpenInterest of the second.
- IV and greeks use the RBI 91-day T-bill yield, latest auction on or before the trade day.

## Next steps

1. Import (feat/gdfl-import): finish the mutants run, then the full Definition of Done once; merge origin/final/all-fixes into it (merge, not rebase) to pick up the `.tix` time index, then re-measure lookup p99.
2. Final check runs for core-second, store-grid, gdfl-cm, db-spot, rate-tbill; then one round-4 review each (three lenses); repair until a round finds nothing.
3. Attack round over every year (decoder fuzz, 1-second build properties, holiday/expiry edges) until a round finds zero.
4. Squash-integrate the clean parts into a new local branch feat/gdfl-1s; prove with `git log -p` that no GDFL row is in any commit; merge final/all-fixes and resolve the store v3 conflicts.
5. /db: show seconds in the Time column for 1-second bars; wire /spot.json and the rate so moneyness, IV and greeks fill in.
6. Land code only through PR #74's merge gate once the user says to import into the real store.

## Running when this session stopped (2026-10-05 02:15 UTC)

`work-20260925/state/import-scratch/run-all-days.sh` is importing EVERY GDFL day (2018-09-03..2026-09-24) into the scratch store the :8081 viewer reads: indices, then stocks, then options. It is resumable: re-run the script and finished days are skipped. Progress: `import-scratch/logs/all-status.txt` and `logs/all-<kind>.txt`. Options for all days may need roughly 1 TB and many hours; check `df -h /Volumes/WD_BLACK` (4.3 TB free at start).

## Update 2026-10-05 03:33 UTC
The user asked to load GDFL into the REAL app store, then said to leave it for the new account. The import into `/Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store` was STOPPED partway through the indices (Sep 2018 onward partly loaded; stocks and options not started). Resume it with `work-20260925/state/import-scratch/run-real-store.sh` (finished days are skipped). The scratch-store import (run-all-days.sh) is also stopped; :8081 still serves what it holds.

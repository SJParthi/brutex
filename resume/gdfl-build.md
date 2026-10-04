# GDFL one-second engine build: resume state (Mac)

Saved 2026-10-04 18:47 UTC by the GDFL build thread before the 5-hour limit pause (resume 22:03 UTC). No GDFL data rows in this file or in any pushed branch. Only origin/feat/gdfl-import (8e6b5219, code only, user said go ahead 18:12 UTC) is pushed; part branches stay local.

## Heads (worktrees under work-20260925/wt/G-<part>)

| Part | Branch | Head | Uncommitted files |
|---|---|---|---|
| census | feat/gdfl-census | 3593ca0d | 4 |
| core-second | feat/gdfl-core-second | 47c794cf | 0 |
| store-grid | feat/gdfl-store-grid | 94e5a960 | 0 |
| gdfl-cm | feat/gdfl-cm-reader | e9c6388e | 0 |
| import | feat/gdfl-import | 794bd68c | 0 |
| dbspot | feat/db-spot | 97fa4c70 | 0 |
| rate | feat/rate-tbill | 28a49141 | 0 |

## Design as of 4-5 Oct (operator)

- GDFL is 1-SECOND ONLY. No GDFL minutes; minutes come from Zerodha 1min, app derives higher rungs. Underlying drives entries; real fills/slippage/costs on OPTIONS' own 1s data (worst case next traded second, volume-gated; index spot no volume check).
- Source: vendor zips or verified tick store (FORMAT.md v1) behind CmSource. CSV folders deleted.
- Per option second: .bin 56 B (OHLC from LTQ>0 rows, volume = sum LTQ, OI) + .grk 80 B (spot same second, IV, greeks, rate, moneyness steps). Intrinsic/extrinsic on read.
- Rate: RBI 91-day T-bill implicit cut-off yield, as-of latest auction <= trade day (user tapped "RBI T-bill"); branch feat/rate-tbill, D-2810.
- Decisions: D-2800 zip source, D-2801 tick-store source, D-2802..2807 import/api wiring, D-2808 per-second volume, D-2809 fill rule (to write), D-2810 rate.
- Store lookup by time is a bisection today: measured p99 104 us (1min month) / 562 us (1s month); by row p99 ~200 ns. Another thread is making time lookup direct.

## Name decoding (proven, state/name-census/)

Rule: from 2019-02-01 every name DD MON YY STRIKE; before, only NIFTY/BANKNIFTY weeklies dated, everything else YY MON STRIKE expiring last Thursday (previous trading day on a holiday). Over all 1,218,362 tickers: 0 refused, 0 two-way, 0 trade after decoded expiry, 887,302 last trade on expiry. Long-dated monthly contracts were renamed at the cutover (6,253 twins): key by contract, not ticker.

## Open work

1. Import (feat/gdfl-import): era-aware decoder diffed against appearances.tsv; 10^6 fuzz; 1s-build property tests; one-day real proof to state/import-scratch/PROOF.md; .grk pass after the rate lands; take attack-thread commits D-3160 tests, D-3161, D-3162 (origin/claude/attack-gdfl).
2. census: round-4 repair (gate 11 count 5->4, signed-price cut-tail test, invented quote values, plan-row condition).
3. store-grid, gdfl-cm: finish checks; round-4 reviews of core-second, store-grid, gdfl-cm.
4. /db spot read (D-2807); RBI rate series (D-2810).
5. Attack round over every year until a round finds zero; then squash-integrate into feat/gdfl-1s, prove no GDFL rows.
6. Board: https://claude.ai/artifact/UsmiHfzRdNWgQeyQZnLULq (source in the session scratchpad; republish each milestone).

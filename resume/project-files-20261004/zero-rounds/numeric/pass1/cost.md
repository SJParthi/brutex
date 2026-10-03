# SLICE=cost — numeric pass 1 (crates/costs and its call sites), commit 331b05c

## Verdict

**NO NEW FINDINGS.** The `costs` arithmetic is integer-only (paisa `i64`, products formed in `i128`, one checked narrowing per result, refused rather than saturated). The rounding laws are consistent with the crate's stated over-charge-only intent. One KNOWN item from the prior audit is still open, and it still contradicts `docs/06-limits.md` §27 (re-verified below; not counted as new).

Production reach today: outside `crates/costs`, only `costs::fill` (`Bar::new`, `fills_at` with `Anchor::Open`/`PrintedExtreme`, `Direction`) is on a live path, in `runner::{trade,grid}`. `costs::trip::price` / `charge_stack` has no production caller (already DOCUMENTED as gaps-8). `pull::pricing` uses `strike::strike_step_on` and `at_the_money`, and `pull::rolling` uses the `expiry` functions.

## Re-verified known item (NOT new)

- **hunt-costs-1 (prior audit, medium): still open at 331b05c.** `crates/costs/src/trip.rs:1222-1226`: `price` calls `Rates::resolve(.., trip.entry().day())`, so STT on a long's exit-day sell is charged at the entry day's rate. `docs/06-limits.md:1550-1590` §27 still says "always in the same direction" (over-charge only) and does not list this case. Probe output, verbatim (1000 units, flat 100.00 to 120.00, entry 2026-03-31, exit 2026-04-01):
  ```
  long: sell_notional=11995000 stt=12000 (sell leg day = exit 2026-04-01 (0.15%))
  short: sell_notional=9995000 stt=10000 (sell leg day = entry 2026-03-31 (0.10%))
  ```
  The long pays ₹120. At the sell day's 0.15% it would be ₹180. The short is correct because its sell leg is the entry leg.

## Probe

`$SCRATCH/probes/cost` is a standalone crate with path dependencies on `crates/costs` and `crates/core`, built with the release profile. Besides the check above, it priced 1,000,000 random same-day round trips (premiums 5 paisa to ₹50,000, quantity 1 to 20,000, long and short, NSE and BSE rates on 2026-05-15). Each trip was compared with an exact half-up reference: non-statutory levies half-up per leg to the paisa, and STT, stamp duty and GST half-up to the rupee, with GST taken on the reference's own rounded base:
```
fuzz trips=1000000 under-charged=0 worst over-charge paisa=306 (doc ceiling 306)
```
So the §27 bound (at most ₹3.06 over, never under) holds exactly for same-day trips.

## Checked and clean

- `money.rs`:
  - `scaled` is total in `i128`.
  - `ceil_div` is a true ceiling for either sign, using `div_euclid`/`rem_euclid` with constant positive divisors, so it cannot divide by zero.
  - `floor_to_paisa`, `ceil_to_paisa` and `ceil_to_rupee` each narrow with a refusal.
  - `statutory_levy` (floor to the paisa, then ceil to the rupee) is ≥ the half-up-to-rupee figure and also ≥ the half-up-to-paisa figure for every raw amount. The worst gap is 100 paisa, as §27 states.
- Per-leg ceilings: `both_legs` ceils each leg and then sums, which is ≥ one rounding of the sum. That is the stated direction.
- GST:
  - The base is the ALREADY-ROUNDED brokerage, exchange charge, SEBI fee and IPFT. STT and stamp duty are excluded.
  - It is rounded once, with no CGST/SGST split.
  - The ceiled components raise the base by at most 6 paisa, which is ≤1.08 paisa of raw GST, so this cannot push the GST over-charge past 100 paisa.
- Sell-side and buy-side legs for shorts: `worst_case_fills` maps the short's BUY to the exit cover and its SELL to the entry open. STT reads `sell_notional` and stamp duty reads `buy_notional`. `position` computes the gross as one `sell - buy` with no direction branch, and that subtraction cannot overflow because both notionals are in `[0, i64::MAX]`.
- Overflow:
  - Notionals use `checked_mul`; the slippage line uses `checked_mul`; the net uses `checked_sub`.
  - `contract_quantity` and `notional` use `checked_mul`.
  - The brokerage `* 2` is guarded by compile-time asserts at `trip.rs:113-114`.
  - The `validated()` sums are formed in `i128`.
- Rate representation: `BpsX100` is the rate × 10^7 as an integer, never a float. It cannot be minted outside the crate (`pub(crate) new`), and `is_well_shaped` asserts rows are ≥ 0. Shipped figures checked against their own stated scale:
  - STT 6,250 / 10,000 / 15,000 = 0.0625% / 0.10% / 0.15%
  - NSE exchange charge 3,503 = 0.03503%; BSE 3,250 = 0.0325%
  - SEBI fee 10 = ₹10 per crore
  - NSE IPFT 50; BSE IPFT 0, a DOCUMENTED UNVERIFIED zero
  - stamp duty 300 = 0.003%, buy side only
  - GST 1,800,000 = 18%
  - TICK = 5 paisa
- No `f32`/`f64` anywhere in `crates/costs/src` outside tests (grep).
- `rate_on` and `DatedTable` lookups walk fixed arrays, so they are O(1) (also covered by prior o1eng2).
- `strike::at_the_money` rounds half-up in `i128` (`(2s+step) div (2*step)`). `moneyness::atm_offset` sends a tie to the lower offset. The two are mutually consistent: the ATM strike always has offset 0 because `atm - spot ∈ (-step/2, step/2]`. `strike_at` and `into_strike` are checked.
- `day.rs` civil-date arithmetic is bounded by the 1990..=2100 window. The `u8` subtraction in `earlier_in_same_month` has a documented and tested precondition (`back <= 6` from a day ≥ 28).
- Runner call sites:
  - `grid.rs:1707-1770` (`entry_fills`, `exit_fill`) and `trade.rs:1344-1396`: `PrintedExtreme` takes the fill-bar extremes of the trade's own entry and exit bars, which is adverse and not a flattering look-ahead.
  - `pnl` uses `saturating_sub` on two values ≥ TICK, so it cannot saturate.
  - Every call is O(1) per trade.
- `pull::pricing` passes the integer ATM into `greeks::Moneyness::from_ladder`. The values are on the grid, so the float division is exact up to its off-grid tolerance.
- Displays `rate.rs:93` and `strike.rs:97` would drop the sign of a value in (-1, 0). No negative rate or strike can be constructed, so this is not reported.

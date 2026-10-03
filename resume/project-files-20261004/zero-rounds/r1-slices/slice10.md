# slice10 (crates/costs) — 2 findings

## F1 [medium] The public `Rates::new` + `charge_stack` path skips the dated refusal, so the "refusal is enforced by the type system" claim is false
- where: crates/costs/src/trip.rs:300-318 (`pub const fn Rates::new`), crates/costs/src/trip.rs:1007 (`pub fn charge_stack`), crates/costs/src/rate.rs (`pub const ZERO`), crates/costs/src/regime.rs:409,415 (`pub const NSE_EXCHANGE_CHARGE` / `BSE_EXCHANGE_CHARGE`). The doc claims it contradicts are at crates/costs/src/lib.rs:17-25 and crates/costs/src/trip.rs:258-260.
- what: lib.rs says the refusal for the pre-2024-10-01 exchange-charge window "is enforced by the type system rather than by a runtime check ... The only way a caller ever holds a BpsX100 is to have been handed one out of a Rate::Verified row ... there is no flag, no keyword and no fall-back that could reach one". trip.rs says "a Rates can only be assembled out of figures that came from a citation-grounded table. That is what carries stage one's refusal contract into stage three intact." Neither holds. `BpsX100::ZERO` and the post-2024 exchange-charge constants are `pub`. `Rates::new` takes the STT, exchange-charge and IPFT rates with no date, and `charge_stack` is `pub` and reads no table. So any caller can price a 2023 NSE option trip at the 2024 rate (the "current rate applied backwards" that lib.rs trap 5 says cannot happen), or with STT, exchange charge and IPFT all at zero. No Refusal is raised. Today this is latent: nothing outside `costs` calls `Rates::new` or `charge_stack` (grep over crates/). It becomes live as soon as the equity/option charge stack gets wired, which CLAUDE.md §1 blocks Selection V6 on.
- evidence: probe crates/costs/tests/zz_probe_slice10.rs (public API only; deleted afterwards):
  ```
  resolve on 2023-06-15: true                      <- Rates::resolve refuses
  exchange_charge_rate refuses: true
  2023 trip priced at the 2024 exchange charge: exchange=387 total=5695
  zero STT/exchange/ipft accepted: stt=0 exchange=0 total=4902
  ```
  (`Rates::new(Broker::Groww, stt_options_rate(2023-06-15)?, NSE_EXCHANGE_CHARGE, ipft(Nse))` and `Rates::new(Groww, BpsX100::ZERO, BpsX100::ZERO, BpsX100::ZERO)`, each passed to `charge_stack(fills, 50, &rates)`, return Ok.)
- fix: make `Rates::new` `pub(crate)` (or `#[cfg(test)]`) so `Rates::resolve(broker, exchange, day)` is the only public constructor, and have `charge_stack` take the resolved `Rates` only through `price`. Otherwise, give `Rates` a private "resolved-on" day field and drop the two sentences. At minimum, correct lib.rs:17-25 and trip.rs:258-260 so they no longer claim the refusal cannot be bypassed.

## F2 [low] `realized_slip_per_unit` counts a sell fill that the floor pushed above the printed low as adverse slippage
- where: crates/costs/src/fill.rs:545 (`let realized = tick + (i128::from(sell_anchor) - i128::from(sell_fill)).abs();`). The contradicting docs are at crates/costs/src/fill.rs:34-38 and :279-283, and the test that pins the wrong sign is crates/costs/src/fill.rs:752-755.
- what: when the sell anchor (a bar low) is below the one-tick floor, the floor raises the sell fill. That fill is better than the printed low, so it is favourable to the seller. `.abs()` turns this favourable move into an adverse one. The module header says the line "reports the movement it actually made, so the informational slippage line never overstates what the notionals carry". For a low of 0 the notionals carry +5 (buy) and -5 (sell, favourable), which nets to 0. The line reports 10. For a low of -95 it reports 105, where the true net is 5 - 100 = -95. `Charges::slippage` (trip.rs:942-948) multiplies this figure by the quantity and shows it as the trip's slippage. It is informational only and is not subtracted from the net (trip.rs:737-740). It is reachable only from bars with a low below one tick, which `Bar::new` admits on purpose.
- evidence: probe output `sell anchor 0 -> sell fill 5 (5 paisa ABOVE the printed low), realized slip 10`. The existing test `the_sell_floor_binds_at_one_tick_and_the_realized_slippage_follows_it` asserts `(10_005, 5, 105)` for a low of -95, which pins the overstatement as the expected output.
- fix: use the signed difference `tick + (sell_anchor - sell_fill)`, which is negative when the floor lifted the fill, and clamp it at 0 if the field must stay non-negative. Then correct the test expectation and the doc at fill.rs:279-283 ("and more when the sell anchor sat below the floor").

## Checked, not reported
- money.rs rounding laws (ceil_div, floor/ceil, narrowing), day.rs ordinal/civil round trip, weekday, last_of_its_month: correct.
- dated.rs `value_on` (with its verified_from fix), regime.rs `rate_on`: regime.rs still names the next row's start whatever that row holds, the pre-fix behaviour dated.rs describes. On the shipped tables (unverified anchor, then verified rows) it is unreachable, so it is latent and not reported.
- expiry.rs weekly row-crossing loop and monthly reference-day rule: traced across 2024-11-13/14, 2025-08-29 to 09-02 and Dec 2024 to Jan 2025, all correct. Trading holidays are a documented limit.
- strike.rs ATM rounding (ties go up) and moneyness.rs atm_offset (ties go down) agree: the ATM strike always has offset 0.
- lot.rs dated tables and the counts in prose ("seven times", "five times") match the tables.
- trip.rs: position(), the GST base, the per-leg ceilings and the op-count table are correct. STT at the entry day's rate is known (hunt-costs-1), not re-reported. The source-citation gap is known (hunt-costs-5).

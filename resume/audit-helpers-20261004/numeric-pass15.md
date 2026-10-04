# Numeric pass 15: option pricing and greeks (tag num15, commit 1f4de71)

**Verdict.** 2 new findings: 1 medium (p15num-1, proven by a throwaway test that ran) and 1 low (p15num-2, not run). The closed form, the CDF and the solver are correct. They reproduce three textbook references to the printed precision (see the table below). The defects are in the **inputs** the model is fed and in how its float output is stored, not in the formulas. Pass 1 (grk.md) and pass 6 cover the domain edges. Their IDs are not reported again here.

Scope read: crates/greeks/src/{bsm,solver,normal,moneyness,lib}.rs, crates/pull/src/{pricing,tenor}.rs, the api callers (server.rs:12082-12280, 13420-13600), the store overlap check (store/src/file.rs:2233-2330, 2819-2857), and charter §4b and 06-limits §29.

## Checklist

| item | where | verdict | source |
|---|---|---|---|
| model | bsm.rs `Checked::greeks`: BSM with continuous yield `q`. d1 = [ln(S/K)+(r-q+σ²/2)T]/(σ√T). Price = S e^-qT N(d1) - K e^-rT N(d2) | formula correct. **Input wrong: q = 0 on a spot index, see p15num-1** | Hull, *Options, Futures and Other Derivatives* (OFOD) 8e, eq. 15.20/15.21 and 17.4 (Merton) |
| greeks | delta e^-qT N(d1); gamma e^-qT n(d1)/(Sσ√T); vega S e^-qT n(d1)√T; theta per **year**; rho per 1.00 | correct. **Ran**: Hull Ch.19 example (S 49, K 50, r .05, σ .2, T 20/52) gives Δ .52160, Γ .06554, ν 12.105, Θ -4.3053/yr, ρ 8.907. Hull prints .522, .066, 12.1, -4.31, 8.91 | Hull 8e §19.1-19.7, Table 19.x |
| price refs | Hull Ex 15.6 (42/40/.1/.2/.5): c 4.759422, p 0.808599. Hull Ex 17.1 (930/900/.08/q .03/.2/2/12): c 51.832957 | match (ran) | Hull 8e Ex 15.6 and 17.1 |
| put-call parity | bsm.rs test `put_call_parity_holds_across_the_grid`, ≤1e-14 of spot | clean | C - P = S e^-qT - K e^-rT |
| rate | `Rate::measured`, operator-supplied, continuous, ±1.0 typo screen. Unsourced in charter (§4b: UNVERIFIED) and carried per row | clean, honest | — |
| dividend / carry | pricing.rs:578-583 hard-codes `carry: 0.0` | **p15num-1** | charter §4b row "Dhan's carry is zero: UNVERIFIED" |
| day count | tenor.rs: wall-clock seconds to the venue close on the expiry day, divided by 365×86,400 (`YearBasis::Calendar365`, the only variant) | consistent with theta per calendar day (charter §4b, divisor 365 UNVERIFIED). Expiry moment = `Venue::NseDerivatives.hours_on(expiry).close_minute()`, dated, not hardcoded | charter §4b |
| T→0, after the close | tenor ≤ 0 gives `AlreadyExpired` (tenor.rs). greeks `years <= 0` gives `Expired` | clean (the bar-open bias is grk-2) | — |
| IV solver | bracket [1e-6, 5]. Floor and cap are checked first. Newton ≤ 8 steps (|step| ≤ 1e-12), then 75 fixed bisections. Max 86 evaluations. Below intrinsic, above the maximum, outside the band and Indeterminate are each refused by name. No clamp reaches the output (the Brenner-Subrahmanyam seed clamp is only a starting point) | clean, no silent fallback. Round trip on Hull 15.6 gives 0.20000000000000015 by Newton in 8 evaluations | Brenner & Subrahmanyam (1988) seed |
| very low or high vol | greeks accepts (0, 10]. The solver band is [1e-6, 5]. A vendor vol in (5, 10] is accepted by `greeks_at` but no solve could produce it | consistent (the vendor unit is grk-1) | — |
| zero or negative price | pricing.rs:558-566 `NotPositive` for spot, strike and premium. The solver refuses a price ≤ intrinsic | clean | — |
| normal CDF | Hart 5666 rational below 7.07, continued fraction above, saturates at |x| > 37. Abs error 2.2e-16, rel error 8.9e-9 at -7.78 (pass 1 measured this against glibc erfc) | clean | Hart (1968) #5666, as in West (2005) "Better approximations to cumulative normal functions" |
| f64 / paisa | paisa i64 → f64 is exact below 2^53. The model runs in paisa (degree-1 homogeneous). Γ/ν/Θ/ρ are stored per paisa and documented (format.rs:522-527). No greek converts back to paisa | clean | — |
| cross-platform determinism | lib.rs:96-104 says greeks/IV "must never enter a content hash or be compared byte for byte across machines". No run identity or data_digest reads .grk (grep). **But the store's overlap check does compare .grk rows byte for byte** | **p15num-2** | — |

## New findings

### p15num-1 (medium): the carry is hard-coded to 0, called "NOT AN ASSUMPTION", while the charter marks q = 0 UNVERIFIED. Every solved IV and greek in `.grk` is biased with no flag.
- `crates/pull/src/pricing.rs:578-583` (also :57-58 and `crates/greeks/src/bsm.rs:128-129`):
  ```rust
  // ZERO, AND NOT AN ASSUMPTION. `greeks::bsm::Contract::carry` says it
  // itself: "Zero for a spot index with no dividend adjustment." ...
  carry: 0.0,
  ```
- **Why it is wrong.**
  - The model is spot-BSM, so its forward is S·e^{rT}. NIFTY and BANKNIFTY are price-return indices whose constituents pay dividends, so the traded forward is S·e^{(r-q)T}.
  - `docs/00-charter.md:491` (§4b) records "Dhan's carry is zero" as **UNVERIFIED** (D-0046), noting that q = 1% and q = 2% fit the captured chain equally well.
  - The code cites only its own doc comment, which is circular, and calls the value "not an assumption". That is the invention that CLAUDE.md §3 rule 1 forbids ("write UNVERIFIED and stop").
  - The error is one-signed by side: call IVs are pushed down and put IVs up. Delta is shifted in the same way.
  - The rows are filed with `VOL_FROM_SOLVED` (server.rs:13426-13450). `Greek` has no carry field and no flag, so nothing downstream can tell the bias is there or undo it.
  - Separately, a deep-ITM put or call whose market time value is smaller than S(1-e^{-qT}) is refused as `PriceBelowIntrinsic`. That refusal is loud, but it is a model artifact, not a bad quote.
- **Repro (ran).** Throwaway test `crates/greeks/tests/num15_probe.rs` in a scratch worktree (deleted afterwards). Premiums were generated at true σ = 0.12, q = 1% (inside the charter's own fitting range), r = 0.065, S = 25,000. They were then solved with q = 0, exactly as `pull::pricing::contract_of` does:
  ```
  T 7d  K 25000 Call solved_iv 0.118151 err -0.185 vol pts; delta filed 0.53362 true 0.52851
  T 7d  K 25000 Put  solved_iv 0.121634 err +0.163 vol pts; delta filed -0.46715 true -0.47130
  T 30d K 25000 Call solved_iv 0.115888 err -0.411 ; Put 0.123180 err +0.318
  T 30d K 24000 Call solved_iv 0.100491 err -1.951 vol pts; delta filed 0.94704 true 0.90835
  T 30d K 26000 Put  solved_iv 0.129264 err +0.926 vol pts
  T 90d K 25000 Call err -0.794 ; Put err +0.507
  ```
  At q = 1.3% the 30d ITM call is 2.76 vol points low. On the 7d ATM strike, the call and the put disagree by 0.35 vol points (q = 1%) to 0.45 vol points (q = 1.3%) from a single fact.
- **Fix (minimal).** Replace the "NOT AN ASSUMPTION" comment with UNVERIFIED (charter §4b). Either refuse to file solved greeks until a sourced carry exists, or add the carry (or an `UNVERIFIED_CARRY` bit) to the `Greek` provenance so each row says it was priced at q = 0. The proper fix is to price on the forward: Black-76 with F taken from the same-stamp future, or the put-call-parity-implied forward from the same strike pair.

### p15num-2 (low, latent): `.grk` overlap re-checks float greeks byte for byte, which the greeks crate says must never be done across machines
- `crates/store/src/file.rs:2287-2329` (`already_stored`, `suffix_that_follows`, and `diagnose_overlap` :2853 `!stored.same_bytes(row)` → `Conflict::Restated`). The writer is `crates/pull/src/ingest.rs:2828` `file.append(rows)`, reached from `api/src/server.rs` `price_chain_month` and `file_the_greeks`. Against that, `crates/greeks/src/lib.rs:96-104`:
  ```
  //! A greek or an implied volatility from this crate must therefore never
  //! enter a content hash or be compared byte for byte across machines.
  ```
- **Why it is wrong.** A resumed or re-pulled window re-prices days already held, and the append verifies those rows by `same_bytes`. Two ordinary events make that comparison fail:
  - The store is read on another target or libm, or glibc's `exp`/`log` change after an OS upgrade. lib.rs measured 140 of 1,344 IVs differing.
  - The operator re-runs with a different rate. `FnoRequest.rate` is per request (api/src/ingest.rs:1169, 1736).

  In either case the overlap reads as `Restated`, and **the whole batch is refused, including the new days after the overlap**. Those days then never get greeks. The refusal is counted (server.rs "COUNTED AS REFUSED"), so it is loud. But it sends the operator to a "restated history" diagnosis for a value the crate itself says is not bit-stable. Idempotence (§3 rule 5) holds only on one machine and one libm.
- **Repro.** Not run. Reasoned from file.rs:2287-2329 and 2853, plus the lib.rs measurement.
- **Fix.** For `Greek` rows, verify the overlap on the integer inputs only (ts, spot, and the rate bits as the request identity) and append the true suffix. Alternatively, refuse with a named "greeks are not bit-stable across libm; overlap re-priced" error that still files the suffix. Either way, document the rule in 06-limits §29.

## Verification of earlier greeks findings at 1f4de71

| id | status | evidence |
|---|---|---|
| grk-1 (vendor IV unit) | NOT FIXED | api/src/server.rs:13575-13576 `sent.insert(ts, millionths_to_decimal(iv))`. pull/src/pricing.rs:760-764 takes it as is. No percent screen. |
| grk-2 / apis-1 (tenor from the bar OPEN) | NOT FIXED | server.rs:12117 and :13552 `Tenor::between(bar.ts_micros / ts, ..)`. tenor.rs:253-255 `bar_secs` from the open stamp, unchanged. |
| p6num greeks rows (domain edges) | still clean | bsm.rs `check` and solver.rs guards are unchanged. |

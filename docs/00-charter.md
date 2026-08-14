# 00 — Charter

Scope, verified external facts, and the prohibitions. Written as prohibitions
because a prohibition survives a rewrite better than a goal does.

---

## 1. Scope lock

**The engine sweeps exactly two instruments, both on NSE.** Narrowed from
three by D-0017: `BSE-SENSEX` is no longer swept and BSE is no longer pulled.

| Symbol | Exchange | Segment |
|---|---|---|
| `NSE-NIFTY` | NSE | INDEX |
| `NSE-BANKNIFTY` | NSE | INDEX |

`NSE-INDIAVIX` is **reference only** — stored, stamped onto observable trades
as `vix_at_entry` / `vix_at_exit`, and never in the condition vocabulary, the
ranking inputs, or run identity.

The store may hold futures, options and single-stock series. Nothing outside
the three symbols above is ever swept.

Widening this list requires an entry in `docs/05-decisions.md`. It does not
happen because a task seemed to need it.

---

## 2. Prohibitions

1. No language other than Rust, in any form, at any layer. See `CLAUDE.md` §2.
2. No writable memory mapping of a store file. Writes go through positional
   syscalls. A writable mapping raises SIGBUS on a full disk and a signal
   cannot be caught in any language.
3. No depth parameter on the sweep. Depth ends at extinction.
4. No ORM, no query planner, no dynamic schema. The path is the index.
5. No float in a price, a cost, or a P&L. Paisa integers only.
6. No fallback that hides the reason it fired. Degrade loudly and name it, or
   refuse. Never silently.
7. No condition bit renumbered or reused. Append only.
8. No token minted by this repository. Credentials are read-only.
9. No claim of a measurement that was not taken.
10. No literal credential path in a tracked file. This repository is public.
    Documents carry the shape `/<org>/<env>/<vendor>/<field>`; the real
    segments are resolved at runtime from an untracked local configuration.
    Enforced by CI gate 1c, not by review. See D-0013.

---

## 3. Verified market facts

Sources are Indian exchange publications and the vendor documentation cited in
§4. Every row here is load-bearing; changing one changes results.

| Fact | Value |
|---|---|
| Regular session | 09:15 – 15:30 IST, NSE and BSE equity |
| Pre-open auction | 09:00 – 09:15 IST — excluded from bars |
| First index tick | 09:15:00 IST |
| Timezone | IST, fixed +05:30, no daylight saving |
| Bars per regular session | 375 at 1-minute granularity |
| Muhurat (Diwali) session | ~1 hour, an evening session on a date that is
  otherwise a holiday. Verified dates: 2020-11-14, 2021-11-04, 2022-10-24,
  2023-11-12, 2024-11-01, 2025-10-21 |
| Special weekend sessions | Union Budget sessions falling on a weekend, run at
  full regular hours. **2020-02-01 (Sat), 2025-02-01 (Sat), 2026-02-01 (Sun)**
  — 375 bars each, confirmed in the lake. This is the complete set: 1 February
  fell on a weekend in no other year from 2020 to date. Supersedes an earlier
  claim of 2021-01-30 and 2021-02-01; see D-0014. |
| Bar timestamp | the **OPEN** (left edge) of its minute. A bar covers the
  half-open window `[t, t + tf)`, left-closed and left-labelled. VERIFIED |
| Last regular 1-minute bar | **15:29:00 IST**, not 15:30. 09:15 through 15:29
  inclusive is exactly 375 bars, which is the arithmetic that closes it. |
| Forced exit | 15:20 IST, **inclusive of the 15:20 bar**: a bar is a
  forced-exit bar iff `bar_open + tf > 15:20`. The 15:19 bar closes exactly at
  15:20 and is not forced. Generalises to `session_close − 10 min`, which
  yields 14:35 for the 2025 Muhurat session and 16:50 for 2021-02-24. |
| Tick grid | 2 decimal places |
| Price storage | paisa integers, `i64` |
| Track-1 brokerage | none. Spot indices are not tradable; the sweep is
  signal-only. Cost modelling belongs to the options translation layer. |

**Muhurat is not a normal session.** A previous-day anchor must never roll
across it — the one-hour evening OHLC is not the prior regular day. This was a
real defect that silently poisoned six years of daily anchors.

Stated exactly, because "never roll across it" is not implementable:

> For a bar on any day after a Muhurat session, the previous-day anchor is the
> OHLC of the **last regular trading session strictly before the Muhurat
> date**. The Muhurat day's own OHLC never enters the previous-day anchor, the
> multi-day rolling history, or the previous-session edge.

Worked case: the anchor for 2024-11-04 (Mon) is **2024-10-31 (Thu)** — not
2024-11-01, which was the Muhurat Friday. The mechanism is that every Muhurat
date is also a non-trading date, so an anchor walk restricted to trading days
skips it structurally rather than by a special case that can be forgotten.

Muhurat sessions, confirmed bar-for-bar against the lake:

| Date | Session (IST) | Bars |
|---|---|---|
| 2020-11-14 (Sat) | 18:15 – 19:15 | 60 |
| 2021-11-04 (Thu) | 18:15 – 19:15 | 60 |
| 2022-10-24 (Mon) | 18:15 – 19:15 | 60 |
| 2023-11-12 (Sun) | 18:00 – 19:00 | **46** — a 14-bar deficit; the open is UNVERIFIED, first bar is 18:07 |
| 2024-11-01 (Fri) | 18:00 – 19:00 | 60 |
| 2025-10-21 (Tue) | 13:45 – 14:45 | 60 — an afternoon session, not an evening one |

2026 has an announced Muhurat date (2026-11-08) with **no notified timings**.
It is therefore absent rather than guessed.

---

## 4. Vendor facts

Evidence lane is recorded per row and is never promoted while copying.

### Groww — primary

| Fact | Value | Lane |
|---|---|---|
| Transport | official SDK, injected client | verified |
| History endpoint | one method for spot and derivatives | verified |
| Granularity fetched | `1minute` only. Every other timeframe is derived — **at the write boundary**, by `pull::fold`, into the rung the bars are filed under. See D-0055. | decided |
| History depth, daily | **2020-01-01, and the vendor claims more.** This row read "from 2020 / documented" and the lane was wrong: no vendor page says 2020. The operator stated it on **11 Aug 2026** and again on **12 Aug 2026**, and the vendor's own interval table gives its `1 day` row **"Full history"** (`Groww Docs / 08-historical-data.md`). Two sources, disagreeing. The operator's is both the LATER day and the higher-standing claim, so it binds on either rule and the vendor's is carried beside it — `pull::vendor::Descriptor::history`, D-0113, D-0131. | operator-stated 11 Aug 2026, restated 12 Aug 2026, contested by vendor docs |
| History depth, 1-minute | **2020-01-01 — the same day as the daily rung, and this row was REVERSED on 12 Aug 2026.** It read "a rolling 3 months — NOT 2020", binding the vendor's published `1 min` row **"Last 3 months"** (`Groww Docs / 08-historical-data.md`) on the rule that the stricter claim wins. The operator restated the floor on **12 Aug 2026**, having watched this build refuse January 2020 through May 2026 on his own account: *"GROWW — data is available from JANUARY 2020. A fixed floor, not a rolling one."* That is a report of what his entitlement answers, not a competing reading of what Groww published, so it outranks rather than merely out-stricts — see `pull::vendor::ClaimStanding`. The vendor's quarter is carried beside it as the contested claim. **UNVERIFIED:** he stated the figure against the vendor, not against a rung, so it is applied to both rungs unchanged; whether this rung truly reaches 2020 is unmeasured, and no request was made to find out. D-0113, D-0131. | operator-stated 12 Aug 2026, contested by vendor docs; the per-rung split UNVERIFIED |
| Window cap, 1-minute | 30 days per request at 1-minute granularity | documented |
| Window cap, 1-minute — the vendor's own table says 7 | **UNRESOLVED, and both figures are written down.** The 30 above is what this repository has carried and is what `pull::vendor::HttpSpec::window_caps` encodes. `Groww Docs / 08-historical-data.md`'s interval table gives the `1 min` row a **"Max Duration per Request" of 7 days**, and its `1 day` row 1,080. Nothing was changed on the strength of this reading: the 30 is operator-facing history and a narrower cap only costs requests, while a wrong one loses bars. Named here so it is not discovered a third time. | conflicting sources |
| Window cap, daily | **UNVERIFIED.** No day-level figure is published in any source this repository has read; the 30 above carries its own "at 1-minute granularity" qualifier and is not promoted. Encoded as **absent** in `pull::vendor::HttpSpec::window_caps`, which means "the vendor bounds nothing here" — the store's one-month-per-file boundary still splits every request. | unverified |
| Daily interval word | **`1day`.** The vendor's own annexure, *Candle Interval*, gives `GrowwAPI.CANDLE_INTERVAL_DAY` the value **`1day`** — the same table that gives `CANDLE_INTERVAL_MIN_1` the value `1minute` this repository already used. The full table also carries `2minute`…`4hour`, `1week` and `1month`; none is recorded here, because `store::path::Timeframe` has a directory for two rungs and a token for a rung the store cannot file is a request whose answer has nowhere to go. Was UNVERIFIED until the docs were read; D-0076. | verified from vendor annexure |
| Index segment word | **`CASH`** — the same word an equity takes. The vendor's live-data page states it: *"Use the segment value FNO for derivatives and CASH for stocks and index."* Before this was read, `Listing::Index` was absent from the descriptor and a live index pull refused by name with `FetchError::ListingNotSpellable`. D-0076. | verified from vendor docs |
| Instrument-type word | **Not applicable to this request.** The annexure carries an instrument-type alphabet (`EQ`, `IDX`, `FUT`, `CE`, `PE`), but the historical-candles request schema is `exchange`, `segment`, `trading_symbol`, `start_time`, `end_time`, `interval_in_minutes` and nothing else — there is no field for a kind, so no word is written into one. Contrast Dhan, whose request carries `instrument`. | verified from vendor docs |
| Response shape | row arrays: `[ts, o, h, l, c, v, oi]`, `oi` null off-derivatives | verified |
| Timestamp | native IST string, or epoch seconds defensively | verified |
| Price unit | rupees as float on the wire; converted to paisa at the boundary | verified |
| Rate limit | 500 requests per minute. **No daily quota.** | operator-confirmed, not published |
| Per-second cap | **UNVERIFIED.** The published 10/s applies to a different endpoint group. Production ceiling is 8/s, chosen not measured. | unverified |
| Auth | TOTP-derived daily token, reset 06:00 IST | verified |
| Credentials | `/<org>/<env>/<vendor>/<field>` — SecureStrings, region `ap-south-1`, read-only. Fields: `api-key`, `totp-secret`, `access-token`. Real segments resolved at runtime; see D-0013. | verified |

### Dhan — secondary, spot indices only

| Fact | Value | Lane |
|---|---|---|
| Endpoint | intraday charts, 1/5/15/25/60 min — we fetch 1 only | verified from SDK |
| Response shape | **parallel column arrays**, not rows. Unequal lengths reject the chunk. | documented |
| Timestamp | epoch seconds, UTC | verified from SDK |
| Window cap, 1-minute | 90 days per request. Documented against the **intraday charts** endpoint named one row above, so it is recorded at the one-minute rung and not promoted past it. | documented |
| Window cap, daily | **UNVERIFIED.** Whether the 90 above applies to a day-level request is not stated anywhere read; nor is whether `/v2/charts/historical` serves daily at all — the endpoint row calls it "intraday charts" and the descriptor's path string does not say. Encoded as **absent**, which the store's month boundary already bounds: every chunk is ≤ 31 days and therefore inside the 90 regardless. | unverified |
| Daily interval word | **Not applicable — and that is itself the fact.** This vendor's request carries no interval parameter at all (five fields: `securityId`, `exchangeSegment`, `instrument`, `fromDate`, `toDate`), so this build cannot vary the rung on its wire. Bars are folded into whatever rung they are filed under, which is correct for any vendor cadence no coarser than the target. | verified from SDK |
| History depth, daily | **A rolling ~5 years, and the vendor claims more.** Operator, **11 Aug 2026**: a rolling last 5 years — **not a fixed floor**, it moves every day. `Dhan Docs / 12-historical-data.md`, *Get Daily Historical Data*: *"The data for any scrip is available back upto the date of its inception."* Two sources, disagreeing; the operator's is the later day, so it binds and the vendor's is carried beside it. A stated absence of a floor cannot widen a stated one. **Restated unchanged on 12 Aug 2026** — *"DHAN — a ROLLING 5 YEARS"* — in the same breath as the Groww correction, so this row is confirmed rather than merely unrevisited. D-0113, D-0131. | operator-stated 11 Aug 2026, restated 12 Aug 2026, contested by vendor docs |
| History depth, 1-minute | **A rolling 5 years, and here the two sources AGREE.** Same page, *Get Intraday Historical Data*: *"…for last 5 years."* Recorded although this build does not serve the rung — the row is a vendor fact, and `/feeds.json` emits it marked `served: false`. D-0113. | documented |
| `toDate` inclusivity | **NON-INCLUSIVE on the daily endpoint, and NOT STATED on the intraday one.** `Dhan Docs / 12-historical-data.md`: the daily request table describes `toDate` as *"End date (YYYY-MM-DD, non-inclusive)"*; the intraday table one section below describes the same field as *"End date (YYYY-MM-DD)"*, with no qualifier. The expired-options endpoint (`14-expired-options-data.md`) repeats *non-inclusive*. `pull::vendor::HttpSpec::range_end` is per **vendor**, and the served path is the daily one, so the encoded `Exclusive` is right for what is sent today — and it is **UNVERIFIED** for the intraday path, which cannot be enabled without settling it. D-0113. | documented (daily) · unverified (intraday) |
| Rate limit | 5/s, 100,000/day, no per-minute governor | documented |
| Subscription | paid data plan; enforcement surfaces as a specific error code | documented |
| Credentials | `/<org>/<env>/<vendor>/<field>` — read-only. Fields: `client-id`, `access-token`. Real segments resolved at runtime; see D-0013. | verified |
| Security ids | NIFTY = 13 (verified from the SDK's own example). BANKNIFTY 25, SENSEX 51, INDIA VIX 21 — **community sources only, unverified.** | mixed |
| India VIX candle availability | **UNVERIFIED.** No documentation states it. Treat as a hard gate before relying on it. | unverified |

### 4z. Zerodha — recorded, and carried nowhere

The operator stated on **11 Aug 2026** that Zerodha serves **a rolling 10 years**
of history. It is written here because §3 rule 1 wants a stated fact traceable,
and it is carried in **no** descriptor: `pull::vendor::Feed` has four rows and
none of them is this vendor. There is no transport, no credential field and no
wire name for it in this repository, so there is nothing for a floor to hang
off. If a row is ever added, this line is the source it starts from — and it is
one source, operator-stated, with no vendor page read against it.

| Fact | Value | Lane |
|---|---|---|
| History depth | rolling 10 years | operator-stated 11 Aug 2026, restated 14 Aug 2026, no vendor page states it |
| Descriptor | **none exists.** Not a feed this build can name. | verified from source |

**The vendor page has now been read.** `https://kite.trade/docs/connect/v3/historical/`,
read 14 Aug 2026. Every row below is quoted from it, so the "no vendor page read"
lane above applies only to the history depth, which that page still does not state.

| Fact | Value | Lane |
|---|---|---|
| Base URL | `https://api.kite.trade` | documented |
| Endpoint | `GET /instruments/historical/:instrument_token/:interval` | documented |
| Auth | header `Authorization: token api_key:access_token`, plus `X-Kite-Version: 3` | documented |
| Instrument identity | numeric `instrument_token`, from the instruments API — **not a symbol** | documented |
| Intervals published | `minute` `3minute` `5minute` `10minute` `15minute` `30minute` `60minute` `day` | documented |
| Intervals THIS BUILD WILL ASK FOR | **`minute` and `day` only** — the same two rungs Groww and Dhan serve. The operator narrowed it on 14 Aug 2026: "one and only one day pull and one min pull". The other six are real and are not wired, so `Descriptor::granularities` carries two and the remaining six refuse by name like every other unfetched rung. | operator-stated 14 Aug 2026 |
| Window params | `from` / `to`, `yyyy-mm-dd hh:mm:ss` | documented |
| Extra params | `continuous` (0/1), `oi` (0/1) | documented |
| Response | `{status, data:{candles:[[ts,o,h,l,c,volume(,oi)]]}}` — an array of ARRAYS, positional | documented |
| Timestamp | `2017-12-15T09:15:00+0530` — ISO **carrying an offset** | documented |
| Prices | decimal rupees (`1704.5`) | documented |
| Expired F&O | `continuous=1` returns **day** candles for expired contracts of a live token's underlying, NFO and MCX futures | documented |
| Window cap | **UNVERIFIED.** The historical page states none. | unverified |
| Rate limit | **3 requests/second** on the historical candle endpoint | documented |
| Rate-limit refusal | HTTP **429** | documented |
| Session expiry | **`TokenException`, HTTP 403.** Caused by logout, natural expiry, **or the user logging into another Kite instance.** Clear the session and re-login. | documented |
| Other exceptions | `InputException` (bad params) · `NetworkException` (API↔OMS) · `DataException` (OMS response unparseable) · `GeneralException` | documented |
| Other HTTP codes | 400 bad params · 404 not found · 410 gone permanently · 500 · 502 OMS down · 503 · 504 | documented |
| History DEPTH per interval | **UNVERIFIED.** No page read states how far back any interval reaches. The operator's rolling 10 years is the only figure, and no vendor page confirms it. | unverified |
| Gap-filling / completeness | **UNVERIFIED, and no page claims it.** Nothing read states that a candle exists for every session minute, nor what a missing session returns. | unverified |

**`TokenException` is the row to read twice.** It fires when the user logs into
ANOTHER Kite instance — so a human opening kite.zerodha.com while a backfill runs
kills that backfill's session. `CLAUDE.md` §8's rule that this repository never
mints a token is not a limitation here, it is the correct posture: a mint would
invalidate whatever else holds the session.

At 3 req/s the arithmetic for a 2020→yesterday backfill is 80 month-windows ×
~800 instruments ÷ 3 = **~5.9 hours of wall clock at the cap, per rung**, before
any retry. That is arithmetic from a published limit, not a measurement.

#### Why this vendor cannot be described by the current descriptor, and it is all three

`docs/07-plan.md` §5 measured three fields that "cannot express an arbitrary
broker at all" and predicted the vendor that would prove it. Zerodha is that
vendor, and it trips **every one**:

1. **`HttpSpec::bars_path` is a fixed string concatenated with `base_url`.**
   Kite carries the instrument AND the interval as PATH SEGMENTS
   (`/instruments/historical/5633/minute`). §5's exact words: "A vendor whose
   instrument and granularity are *path segments* cannot be described."
2. **`AuthScheme` is `Raw | Bearer`.** Kite's is `token api_key:access_token` —
   a prefix AND a second secret. §5: "A scheme carrying a prefix and a **second**
   secret cannot be described."
3. **`TimestampEncoding::IsoDateTimeText` documents itself as carrying no zone.**
   Kite returns `+0530`. §5: "A vendor returning `+0530` cannot be described."

A fourth, not in §5 and found here: the instrument is a NUMERIC TOKEN from a
separate instruments call, not a tradingsymbol, so the (exchange, symbol) key
this repository joins on does not address a Kite request at all.

**So adding Zerodha is not a descriptor row.** It is three `pull::vendor` type
changes plus an instrument-token map, and each needs its own decision entry. The
prediction in §5 was right and this is the evidence that closed it.

### 4a. Instrument facts transcribed into source

Golden rule 1: every claim about an instrument names the route that produced
it. These are the instrument-level facts compiled into this repository as Rust
data rather than fetched, so the route has to be recorded here.

| Fact | Where it lives | Route | Lane |
|---|---|---|---|
| **NIFTY Total Market — 750 constituents** | `core::universe::NIFTY_TOTAL_MARKET` | niftyindices.com states the index holds 750 stocks: "all stocks that are part of Nifty 500 and Nifty Microcap 250". The names were transcribed from the local lake's NSE/CASH directories and matched the primary broker's equity list 750/750 with zero misses. Re-checked against both real masters 2026-08-01: 750/750 resolve as a kept equity in **both** vendors, ISINs agreeing. | derived + cross-checked; **UNVERIFIED against an NSE constituent circular** |
| **F&O underlyings — 213 names** | `core::universe::FNO_UNDERLYINGS` | Derived from the derivative rows of both vendor masters, excluding `*NSETEST`. The two brokers agree exactly: 213 each, zero on either side only. | derived + cross-checked; **UNVERIFIED against an NSE circular** |
| **NSE board series — 128 codes** | `core::vendor::EQUITY_BOARD_SERIES`, `SME_BOARD_SERIES`, `NON_EQUITY_SERIES` | The measured union of every distinct `series` on a Groww `NSE/CASH/EQ` row and every distinct `SERIES` on a Dhan `NSE/E/EQUITY` row, 2026-08-01. 6 equity-board, 2 SME, 120 debt and fund. Board verdicts agree on 4,080 of 4,080 shared ISINs. | measured from both masters; **UNVERIFIED against an NSE series circular** |

Both index lists are **snapshots** of rebalanced indices, and none of the three
has been checked against an exchange publication. `docs/06-limits.md` §11
carries what that costs. D-0025 and D-0029.

**The Total Market row above is now partly contradicted, and the contradiction
is recorded rather than repaired.** On 2026-08-11 the exchange's own file was
fetched for the first time (§4c). Excluding two placeholder scrips it holds 750
real names, the same count — but not the same names. The exchange lists
`GRINDWELL`, which `NIFTY_TOTAL_MARKET` does not hold; `NIFTY_TOTAL_MARKET`
holds `AGL`, which the exchange does not list. One name in 750, and the
constant's 750 is therefore **not a subset** of the exchange's 752. The array
is deliberately left as it stands: it was measured 750/750 against two real
vendor masters under the D-0025 gate, and that measurement cannot be redone
from a downloaded CSV. `AGL` is **UNVERIFIED** — this repository does not know
what instrument it is, and will not guess. D-0089 states the resolution and
`docs/06-limits.md` §11 carries the cost.

### 4b. Option-greek facts, measured from a live chain

Golden rule 1 again. `crates/greeks` makes claims about what a vendor's option
chain *means*, and a unit convention nobody states is exactly the kind of claim
that has to name its route. Every row below was measured from **one live Dhan
option-chain response, one strike, both sides**, captured 2026-08-01:
`IV 11.939337251984934 / 9.789193798280868` (percent), `delta 0.53871 /
−0.46732`, `gamma 0.00132 / 0.00109`, `theta −15.1539 / −10.61131`,
`vega 12.2025 / 12.18593`. The response carried **no rho, no spot, no strike,
no timestamp and no expiry.**

| Fact | Where it lives | Route | Lane |
|---|---|---|---|
| **Dhan publishes vega per one percentage point** | `greeks::bsm` module docs; `greeks::vendor_anchor::vega_is_published_per_percentage_point_and_the_index_level_proves_it` | The raw scaling implies an index level of **258.51**; the per-percent scaling implies **25,851.19**, which is where NIFTY trades. | measured; the vendor documents no unit |
| **Dhan publishes theta per a CALENDAR day, not a trading day** | same, and `greeks::vendor_anchor::the_trading_day_divisor_is_excluded_and_the_calendar_one_is_not_selected` | The two sides of one strike must agree on `r`. They agree to **0.9999** points under 365 and **23.2598** under 252 — a factor of 23.3. | measured — but only as an **exclusion of 252** |
| **The divisor is exactly 365** | `greeks::vendor_anchor::the_rate_criterion_has_its_root_at_370_and_365_is_a_convention_near_it` | The same criterion is affine in the divisor and has its root at **`D* = 370.0757`**, where the two sides agree to 2.8e-15 points; it also prefers **375** (spread 0.9700) to 365 (0.9999). `365` is the nearest ordinary calendar convention to that root, not a measurement of it. | **UNVERIFIED.** Withdrawn from "measured" by D-0046 |
| **Dhan's two published IVs are transposed relative to its delta/gamma/vega block** | `greeks::vendor_anchor::the_two_published_volatilities_are_transposed_and_the_identity_says_so` | The scale-free identity `vega·gamma·sigma == n(d1)^2` — no spot, strike, maturity or rate in it — gives 1.21979 / 0.82249 as published and 1.00012 / 1.00315 swapped, against gamma's own printed slack of 0.38% / 0.46%. | measured, **from one strike only.** Whether every strike is transposed the same way is **UNVERIFIED** |
| **Dhan uses standard spot BSM** | `greeks::vendor_anchor::our_greeks_reproduce_the_captured_dhan_chain` | Gamma and vega are near-identical between the two sides at one strike, which is BSM. | measured; **forward Black-76 is not excluded — UNVERIFIED** |
| **Dhan's carry is zero** | `greeks::vendor_anchor::the_carry_is_consistent_with_zero_and_the_sample_cannot_pin_it` | `q = 0` reproduces the chain, and a single volatility would need `q = −42.54%`, which is not a rate. But `q = 1%` and `q = 2%` reproduce **all eight** published fields too, gammas included, at `T = 4.085` and `3.428` calendar days. | **UNVERIFIED.** `q = 0` is *consistent*, not measured. Withdrawn from "measured" by D-0046 |
| **Dhan publishes no rho; Groww publishes all five** | `crates/greeks` ships rho regardless | The captured response has no `rho` field. Groww documents a dedicated Greeks section at `groww.in/trade-api/docs/curl/live-data`. | read from the response and from the vendor documentation |
| **Dhan's risk-free rate** | nowhere — nothing in this repository hardcodes one | Solved at **9.4619%** (call side) and **10.4618%** (put side), each ±0.55 at 95% from the printed precision of delta alone, with a 1.00-point side-to-side residual outside both intervals. | **UNVERIFIED.** A hardcoded 10.0% fits the sample and so does a market rate near 7%; this sample cannot separate them |
| **NIFTY and BANKNIFTY strike intervals** | nowhere — `Moneyness::from_ladder` takes the interval as an argument | No source states them. | **UNVERIFIED.** Assumed nowhere in code |

The maturity is the best-conditioned parameter the sample carries:
`T = 0.0141324716` years = **5.15835 calendar days**, ±0.079 at 95% over the
rounding box of the printed fields. That interval is a **rounding-box width,
not an identification result**, and it is conditional on `q = 0` and on the
transposition — at `q = 2%` the same eight fields give `T = 3.428` days. The
**day count that generates it** is UNVERIFIED without the capture timestamp.

**And the sample is internally inconsistent with spot BSM at one instant.**
Matching both published deltas forces the model's vega ratio to `0.998641`
against the vendor's `1.001360`, in a quantity containing no spot, strike,
maturity or rate. The best possible *single* contract is off by **884× the
vendor's own display precision** — measured. Everything above is fitted with
**two** mutually inconsistent contracts, one per side, which is a diagnostic
and not an agreement. D-0046, and `docs/06-limits.md` §18.

### 4c. NSE index constituents, read from the exchange's own files

Golden rule 1 again, and this time the rule caught something that had already
shipped. The web pages offered NIFTY 50 / 100 / 200 / 500 and produced them by
slicing `core::universe::NIFTY_TOTAL_MARKET`, which is stored alphabetically —
so `slice(0, 50)` yielded `360ONE, 3MINDIA, AADHARHFC, …` and the page called
that the NIFTY 50. No source said any of it. The four tiers did not exist in
this repository at all. D-0089 replaces the slice with the exchange's own
published constituent files, which are the route recorded here.

**Fetched 2026-08-11**, over HTTPS, from `nsearchives.nseindia.com` — the
archive host of the exchange that computes the indices. Each file is a CSV with
the header `Company Name, Industry, Symbol, Series, ISIN Code`; **every row
carries an ISIN**, which is what makes a row checkable against a vendor master
rather than merely readable.

| Index | URL | Rows | Where it lives |
|---|---|---|---|
| NIFTY 50 | `https://nsearchives.nseindia.com/content/indices/ind_nifty50list.csv` | 50 | `core::universe::NIFTY_50` |
| NIFTY 100 | `https://nsearchives.nseindia.com/content/indices/ind_nifty100list.csv` | 100 | `core::universe::NIFTY_100` |
| NIFTY 200 | `https://nsearchives.nseindia.com/content/indices/ind_nifty200list.csv` | 200 | `core::universe::NIFTY_200` |
| NIFTY 500 | `https://nsearchives.nseindia.com/content/indices/ind_nifty500list.csv` | 500 | `core::universe::NIFTY_500` |
| NIFTY Total Market | `https://nsearchives.nseindia.com/content/indices/ind_niftytotalmarket_list.csv` | **752** | `core::universe::NIFTY_TOTAL_MARKET` holds **750** — see below |

**Symbols only are transcribed.** `CLAUDE.md` §2 allows no `.csv` in the
tracked tree, so the files are input and the data becomes Rust. The company
name, industry, series and ISIN are read, used for the checks in this section,
and not carried: nothing in `crates/core` reads them, and a field carried
without a reader is a field that goes stale unnoticed.

**Verified on the fetched files, not asserted:**

- The tiers **nest**: all 50 NIFTY 50 symbols are in the 100, all 100 in the
  200, all 200 in the 500, and all 500 in `NIFTY_TOTAL_MARKET` — zero outside
  in every direction. Proved for every symbol by
  `core::universe::the_published_tiers_nest_one_inside_the_next`, which is why
  `of_equity` may set several bits for one name.
- The Total Market file is exactly the union of the NIFTY 500 file and the
  NIFTY Microcap 250 file — 752 rows, row for row, nothing on either side
  alone. That matches niftyindices.com's own definition, "all stocks that are
  part of Nifty 500 and Nifty Microcap 250".

**The 752 that is 750.** Two of the 752 rows are NSE placeholder scrips, not
constituents: `DUMMYINXGN` ("Dummy Inox Green Ltd.") and `DUMMYTRVN` ("Dummy
Triveni Ltd."). Their identifiers are not ISINs — they read `DUM510W01014` and
`DUM256C01024`, the real ISINs of `INOXGREEN` and `TRIVENI` with `INE` replaced
by `DUM` — and both of those real constituents are separately present in the
same file. Dropping the two placeholders leaves **750 real names**, which is
the count niftyindices.com states and the count `NIFTY_TOTAL_MARKET` declares.
The agreement of the counts is a coincidence of arithmetic, not of membership:
see §4a's row, `docs/06-limits.md` §11, and D-0089, which records that the
repository's 750 and the exchange's 750 differ by one name.

**What NSE publishes and this repository does NOT carry.** The exchange's own
index directory (`https://www.nseindia.com/api/allIndices`, same fetch) lists
**139 indices**. This repository carries **five** of them — the four tiers above
plus NIFTY Total Market — alongside the derived F&O underlying list of §4a. The
other 134, sectoral and thematic and strategy indices among them — NIFTY BANK,
NIFTY IT, NIFTY MIDCAP 150, NIFTY MICROCAP 250 and the rest — are **not
carried**, are not a universe, and no part of this repository may claim
membership in one. Two of them appear in this section only as evidence:
Microcap 250 was read to check the Total Market's composition, and it was not
transcribed.

These are **snapshots.** NSE rebalances these indices semi-annually and none of
the five has been checked against a constituent circular. `docs/06-limits.md`
§11 carries what that costs.

---

## 5. Run identity

```
blake3(
  strategy_mask ‖ direction ‖ instrument ‖ timeframe ‖ mode ‖
  canonical_params ‖ data_digest ‖ vocab_version ‖ commit_sha
)
```

`data_digest` is a streamed digest over **every field of every loaded bar** —
not a sample, not a count-and-endpoints fingerprint. One differing bar re-keys
the identity. That is the point.

No computation runs without that identity recorded first.

---

## 6. The sweep, in one paragraph

Load a contiguous slice of 1-minute bars for one instrument. Compute the 74
condition bits per bar, once. Walk the combination ladder from k=1: at each
level, join the previous frequent frontier with itself, prune any candidate
whose (k−1)-subsets are not all frequent, evaluate the survivors against the
bar-bit column, keep those with at least `min_hits` hits, and recurse. Stop
when a level produces nothing. Rank the survivors, persist a bounded top set,
and record the identity.

Nothing about that paragraph has a tunable depth.

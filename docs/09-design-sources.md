# 09 — Design sources

The three indicators the condition vocabulary is derived from, recorded verbatim.

**Why this file exists.** `docs/03-vocabulary.md` names the bits and
`crates/vocab/src/table.rs` implements them, but neither says where a formula
came from. Golden rule 1 in `CLAUDE.md` §3 forbids inventing one:

> Every claim about a vendor, an exchange, an instrument or a cost is traceable
> to a source recorded in `docs/00-charter.md`. If you are unsure, write
> `UNVERIFIED` and stop.

That rule worked exactly as designed and produced a wrong answer. An audit
refused pivot levels **R4, R5, S4 and S5** on the grounds that no tracked
document recorded their formula — true of this repository, false of the world.
The formulas were in the operator's own indicator, which lived only in a chat
transcript. Two shipped bits, `near_pivot_r5` (54) and `near_pivot_s5` (55),
had names and no arithmetic behind them for the same reason.

**114 of the 188 bit positions trace to the scripts below.** Until this file
existed, none of that was auditable from a clone. It is now.

**Scope note.** This is a `.md` document quoting other languages inside fenced
blocks, which `CLAUDE.md` §2 permits — `.md` is an allowed extension. Nothing
here is compiled, executed, or read at run time by any crate. It is a record.

---

## 1. CPR + S/R zones

**Authorship.** Mozilla Public License 2.0, https://mozilla.org/MPL/2.0/ ·
© `vedhaviyash4` · extended by the operator with R5/S5 and per-level zones, and
ported to Pine v6.

**What the vocabulary takes from it.** Thirteen daily levels — pivot, BC, TC,
R1–R5, S1–S5 — plus the previous day's high and low as their own lines, and a
**band** around every R and S whose half-width is derived from the CPR width
rather than from a fixed point count.

The recurrence, which is the part that was missing:

```
daily_pivot = (H + L + C) / 3          // H, L, C from the PREVIOUS completed day
daily_bc    = (H + L) / 2
daily_tc    = 2 * daily_pivot - daily_bc

daily_r1 = 2 * daily_pivot - L
daily_r2 = daily_pivot + H - L
daily_r3 = daily_r1 + H - L
daily_r4 = daily_r3 + daily_r2 - daily_r1      //  = P + 2(H - L)
daily_r5 = daily_r4 + daily_r3 - daily_r2      //  = 2P + 2H - 3L

daily_s1 = 2 * daily_pivot - H
daily_s2 = daily_pivot - H + L
daily_s3 = daily_s1 - H + L
daily_s4 = daily_s3 + daily_s2 - daily_s1      //  = P - 2(H - L)
daily_s5 = daily_s4 + daily_s3 - daily_s2      //  = 2P - 3H + 2L
```

And the band, which is why the `near_*` tolerance in
`crates/vocab/src/tolerance.rs` is a fraction of a **range** and not of a
**level** (D-0076):

```
cpr_half = abs(daily_pivot - daily_bc) * zone_mult      // zone_mult default 1.0
zone_bottom(level) = level - cpr_half
zone_top(level)    = level + cpr_half
```

Two consequences the implementation must honour, both recorded as findings
elsewhere:

* At `zone_mult = 1.0` the **pivot's own band is exactly `[bc, tc]`**, the CPR
  body. That makes `near_pivot_p` and `inside_cpr` the same predicate, which is
  why bit 6 is a tombstone pointing at bit 62.
* When the previous day closed at the midpoint of its own range, `pivot == bc`
  and `cpr_half` is **zero** — every band collapses to a line and every
  "inside the band" test becomes exact equality.

### Full source

```pine
// This source code is subject to the terms of the Mozilla Public License 2.0 at https://mozilla.org/MPL/2.0/
// © vedhaviyash4  — extended: R5/S5 + per-level zones, ported to Pine v6

//@version=6
indicator(title="CPR + S/R Zones", shorttitle="CPR ZONES", overlay=true)

// ─────────── inputs ───────────
trackprice_bool = input.bool(true,  "Track price")
use_prev_day    = input.bool(true,  "Use PREVIOUS day HLC (standard CPR)")
show_zones      = input.bool(true,  "Show R/S zones (bottom + top band)")
show_r5s5       = input.bool(true,  "Show R5 / S5")
zone_mult       = input.float(1.0,  "Zone width  (x CPR width)", minval=0.1, step=0.1)
fill_zones      = input.bool(true,  "Shade zones")

// ─────────── source ───────────
// [1] + lookahead_on = the standard NON-repainting previous-completed-day idiom.
h_prev = request.security(syminfo.tickerid, "D", high[1],  lookahead=barmerge.lookahead_on)
l_prev = request.security(syminfo.tickerid, "D", low[1],   lookahead=barmerge.lookahead_on)
c_prev = request.security(syminfo.tickerid, "D", close[1], lookahead=barmerge.lookahead_on)

// Current forming day (your original behaviour — REPAINTS intraday).
h_cur  = request.security(syminfo.tickerid, "D", high)
l_cur  = request.security(syminfo.tickerid, "D", low)
c_cur  = request.security(syminfo.tickerid, "D", close)

dh = use_prev_day ? h_prev : h_cur
dl = use_prev_day ? l_prev : l_cur
dc = use_prev_day ? c_prev : c_cur

// ─────────── CPR ───────────
daily_pivot = (dh + dl + dc) / 3
daily_bc    = (dh + dl) / 2
daily_tc    = 2 * daily_pivot - daily_bc

// ─────────── R / S (r5,s5 continue the original recurrence) ───────────
daily_r1 = 2 * daily_pivot - dl
daily_r2 = daily_pivot + dh - dl
daily_r3 = daily_r1 + dh - dl
daily_r4 = daily_r3 + daily_r2 - daily_r1
daily_r5 = daily_r4 + daily_r3 - daily_r2

daily_s1 = 2 * daily_pivot - dh
daily_s2 = daily_pivot - dh + dl
daily_s3 = daily_s1 - dh + dl
daily_s4 = daily_s3 + daily_s2 - daily_s1
daily_s5 = daily_s4 + daily_s3 - daily_s2

// ─────────── zone half-width = CPR half-width ───────────
cpr_half = math.abs(daily_pivot - daily_bc) * zone_mult

zb(float level, bool visible) => visible ? level - cpr_half : na   // zone bottom
zt(float level, bool visible) => visible ? level + cpr_half : na   // zone top

r5v = show_r5s5 ? daily_r5 : na
s5v = show_r5s5 ? daily_s5 : na
z5  = show_zones and show_r5s5

// ─────────── CPR plots ───────────
plot(daily_pivot, title="Pivot", color=color.purple, linewidth=2, style=plot.style_circles, trackprice=trackprice_bool)
pBC = plot(daily_bc, title="BC", color=color.purple, linewidth=2, style=plot.style_circles, trackprice=trackprice_bool)
pTC = plot(daily_tc, title="TC", color=color.purple, linewidth=2, style=plot.style_circles, trackprice=trackprice_bool)
fill(pBC, pTC, color=fill_zones ? color.new(color.purple, 85) : na, title="CPR band")

plot(dh, title="Prev day high", color=color.black, linewidth=2, style=plot.style_line, trackprice=trackprice_bool)
plot(dl, title="Prev day low",  color=color.black, linewidth=2, style=plot.style_line, trackprice=trackprice_bool)

// ─────────── resistances + zones ───────────
plot(daily_r1, title="R1", color=color.green, linewidth=2, style=plot.style_line, trackprice=trackprice_bool)
plot(daily_r2, title="R2", color=color.green, linewidth=2, style=plot.style_line, trackprice=trackprice_bool)
plot(daily_r3, title="R3", color=color.green, linewidth=2, style=plot.style_line, trackprice=trackprice_bool)
plot(daily_r4, title="R4", color=color.green, linewidth=2, style=plot.style_line, trackprice=trackprice_bool)
plot(r5v,      title="R5", color=color.green, linewidth=2, style=plot.style_line, trackprice=trackprice_bool)

pR1b = plot(zb(daily_r1, show_zones), title="R1 bottom", color=color.new(color.green, 45), linewidth=1)
pR1t = plot(zt(daily_r1, show_zones), title="R1 top",    color=color.new(color.green, 45), linewidth=1)
fill(pR1b, pR1t, color=fill_zones ? color.new(color.green, 88) : na, title="R1 zone")

pR2b = plot(zb(daily_r2, show_zones), title="R2 bottom", color=color.new(color.green, 45), linewidth=1)
pR2t = plot(zt(daily_r2, show_zones), title="R2 top",    color=color.new(color.green, 45), linewidth=1)
fill(pR2b, pR2t, color=fill_zones ? color.new(color.green, 88) : na, title="R2 zone")

pR3b = plot(zb(daily_r3, show_zones), title="R3 bottom", color=color.new(color.green, 45), linewidth=1)
pR3t = plot(zt(daily_r3, show_zones), title="R3 top",    color=color.new(color.green, 45), linewidth=1)
fill(pR3b, pR3t, color=fill_zones ? color.new(color.green, 88) : na, title="R3 zone")

pR4b = plot(zb(daily_r4, show_zones), title="R4 bottom", color=color.new(color.green, 45), linewidth=1)
pR4t = plot(zt(daily_r4, show_zones), title="R4 top",    color=color.new(color.green, 45), linewidth=1)
fill(pR4b, pR4t, color=fill_zones ? color.new(color.green, 88) : na, title="R4 zone")

pR5b = plot(zb(daily_r5, z5), title="R5 bottom", color=color.new(color.green, 45), linewidth=1)
pR5t = plot(zt(daily_r5, z5), title="R5 top",    color=color.new(color.green, 45), linewidth=1)
fill(pR5b, pR5t, color=fill_zones ? color.new(color.green, 88) : na, title="R5 zone")

// ─────────── supports + zones ───────────
plot(daily_s1, title="S1", color=color.red, linewidth=2, style=plot.style_line, trackprice=trackprice_bool)
plot(daily_s2, title="S2", color=color.red, linewidth=2, style=plot.style_line, trackprice=trackprice_bool)
plot(daily_s3, title="S3", color=color.red, linewidth=2, style=plot.style_line, trackprice=trackprice_bool)
plot(daily_s4, title="S4", color=color.red, linewidth=2, style=plot.style_line, trackprice=trackprice_bool)
plot(s5v,      title="S5", color=color.red, linewidth=2, style=plot.style_line, trackprice=trackprice_bool)

pS1b = plot(zb(daily_s1, show_zones), title="S1 bottom", color=color.new(color.red, 45), linewidth=1)
pS1t = plot(zt(daily_s1, show_zones), title="S1 top",    color=color.new(color.red, 45), linewidth=1)
fill(pS1b, pS1t, color=fill_zones ? color.new(color.red, 88) : na, title="S1 zone")

pS2b = plot(zb(daily_s2, show_zones), title="S2 bottom", color=color.new(color.red, 45), linewidth=1)
pS2t = plot(zt(daily_s2, show_zones), title="S2 top",    color=color.new(color.red, 45), linewidth=1)
fill(pS2b, pS2t, color=fill_zones ? color.new(color.red, 88) : na, title="S2 zone")

pS3b = plot(zb(daily_s3, show_zones), title="S3 bottom", color=color.new(color.red, 45), linewidth=1)
pS3t = plot(zt(daily_s3, show_zones), title="S3 top",    color=color.new(color.red, 45), linewidth=1)
fill(pS3b, pS3t, color=fill_zones ? color.new(color.red, 88) : na, title="S3 zone")

pS4b = plot(zb(daily_s4, show_zones), title="S4 bottom", color=color.new(color.red, 45), linewidth=1)
pS4t = plot(zt(daily_s4, show_zones), title="S4 top",    color=color.new(color.red, 45), linewidth=1)
fill(pS4b, pS4t, color=fill_zones ? color.new(color.red, 88) : na, title="S4 zone")

pS5b = plot(zb(daily_s5, z5), title="S5 bottom", color=color.new(color.red, 45), linewidth=1)
pS5t = plot(zt(daily_s5, z5), title="S5 top",    color=color.new(color.red, 45), linewidth=1)
fill(pS5b, pS5t, color=fill_zones ? color.new(color.red, 88) : na, title="S5 zone")
```

---

## 2. Volume Weighted Average Price

**Authorship.** TradingView's built-in VWAP indicator, Pine v6.

**What the vocabulary takes from it.** A session-anchored VWAP on `hlc3` with
**three** band multipliers (1.0, 2.0, 3.0), and two band calculation modes —
standard deviation and percentage.

**The blocker, measured.** `docs/06-limits.md` records that across all
**1,706,290** stored bars the `volume` column holds exactly one value: literal
zero, and `open_interest` is 100% null. A spot index has no traded volume by
construction — it is a computed number, not an instrument that trades. So on
the two swept instruments this indicator is a division by zero, and the script
says so itself:

```pine
cumVolume = ta.cum(volume)
if barstate.islast and cumVolume == 0
    runtime.error("No volume is provided by the data vendor.")
```

The vocabulary allocates the bits anyway and lets them **abstain**, so that if
the swept surface ever widens to futures, options or single stocks — which
`CLAUDE.md` §1 stores but never sweeps — the positions already exist and no
renumbering is needed. `docs/03-vocabulary.md` §4 is the rule: *a bit that
cannot be evaluated evaluates false; it never evaluates to "probably"*.

**Also measured:** VWAP is **not** timeframe-invariant. Unlike a running high or
low, a coarse bar's `hlc3` is not the volume-weighted average of its constituent
`hlc3`s, so 100% of aligned bar boundaries disagree — worst gap 14.97 NIFTY
points at the 60-minute rung on synthetic bars.

### Full source

```pine
//@version=6
indicator(title="Volume Weighted Average Price", shorttitle="VWAP", overlay=true, timeframe="", timeframe_gaps=true)

hideonDWM = input(false, title="Hide VWAP on 1D or Above", group="VWAP Settings", display = display.none)
var anchor = input.string(defval = "Session", title="Anchor Period",
 options=["Session", "Week", "Month", "Quarter", "Year", "Decade", "Century", "Earnings", "Dividends", "Splits"], group="VWAP Settings")
src = input(title = "Source", defval = hlc3, group="VWAP Settings", display = display.none)
offset = input.int(0, title="Offset", group="VWAP Settings", display = display.none)

BANDS_GROUP = "Bands Settings"
CALC_MODE_TOOLTIP = "Determines the units used to calculate the distance of the bands. When 'Percentage' is selected, a multiplier of 1 means 1%."
calcModeInput = input.string("Standard Deviation", "Bands Calculation Mode", options = ["Standard Deviation", "Percentage"], group = BANDS_GROUP, tooltip = CALC_MODE_TOOLTIP, display = display.none)
showBand_1 = input(true, title = "", group = BANDS_GROUP, inline = "band_1", display = display.none)
bandMult_1 = input.float(1.0, title = "Bands Multiplier #1", group = BANDS_GROUP, inline = "band_1", step = 0.5, minval=0, display = display.none, active = showBand_1)
showBand_2 = input(false, title = "", group = BANDS_GROUP, inline = "band_2", display = display.none)
bandMult_2 = input.float(2.0, title = "Bands Multiplier #2", group = BANDS_GROUP, inline = "band_2", step = 0.5, minval=0, display = display.none, active = showBand_2)
showBand_3 = input(false, title = "", group = BANDS_GROUP, inline = "band_3", display = display.none)
bandMult_3 = input.float(3.0, title = "Bands Multiplier #3", group = BANDS_GROUP, inline = "band_3", step = 0.5, minval=0, display = display.none, active = showBand_3)

cumVolume = ta.cum(volume)
if barstate.islast and cumVolume == 0
    runtime.error("No volume is provided by the data vendor.")

isNewPeriod = switch anchor
	"Earnings" =>
		new_earnings_actual = request.earnings(syminfo.tickerid, earnings.actual, barmerge.gaps_on, barmerge.lookahead_on, ignore_invalid_symbol=true)
		new_earnings_standardized = request.earnings(syminfo.tickerid, earnings.standardized, barmerge.gaps_on, barmerge.lookahead_on, ignore_invalid_symbol=true)
		not na(new_earnings_actual) or not na(new_earnings_standardized)
	"Dividends" =>
		new_dividends = request.dividends(syminfo.tickerid, dividends.gross, barmerge.gaps_on, barmerge.lookahead_on, ignore_invalid_symbol=true)
		not na(new_dividends)
	"Splits"    =>
		new_split = request.splits(syminfo.tickerid, splits.denominator, barmerge.gaps_on, barmerge.lookahead_on, ignore_invalid_symbol=true)
		not na(new_split)
	"Session"   => timeframe.change("D")
	"Week"      => timeframe.change("W")
	"Month"     => timeframe.change("M")
	"Quarter"   => timeframe.change("3M")
	"Year"      => timeframe.change("12M")
	"Decade"    => timeframe.change("12M") and year % 10 == 0
	"Century"   => timeframe.change("12M") and year % 100 == 0
	=> false

isEsdAnchor = anchor == "Earnings" or anchor == "Dividends" or anchor == "Splits"
if na(src[1]) and not isEsdAnchor
	isNewPeriod := true

float vwapValue = na
float upperBandValue1 = na
float lowerBandValue1 = na
float upperBandValue2 = na
float lowerBandValue2 = na
float upperBandValue3 = na
float lowerBandValue3 = na

if not (hideonDWM and timeframe.isdwm)
    [_vwap, _stdevUpper, _] = ta.vwap(src, isNewPeriod, 1)
	vwapValue := _vwap
    stdevAbs = _stdevUpper - _vwap
	bandBasis = calcModeInput == "Standard Deviation" ? stdevAbs : _vwap * 0.01
	upperBandValue1 := _vwap + bandBasis * bandMult_1
	lowerBandValue1 := _vwap - bandBasis * bandMult_1
	upperBandValue2 := _vwap + bandBasis * bandMult_2
	lowerBandValue2 := _vwap - bandBasis * bandMult_2
	upperBandValue3 := _vwap + bandBasis * bandMult_3
	lowerBandValue3 := _vwap - bandBasis * bandMult_3

plot(vwapValue, title = "VWAP", color = #2962FF, offset = offset)

displayBand1 = showBand_1 ? display.all : display.none
upperBand_1 = plot(upperBandValue1, title="Upper Band #1", color = color.green, offset = offset, display = displayBand1, editable = showBand_1)
lowerBand_1 = plot(lowerBandValue1, title="Lower Band #1", color = color.green, offset = offset, display = displayBand1, editable = showBand_1)
fill(upperBand_1, lowerBand_1,      title="Bands Fill #1", color = color.new(color.green, 95),   display = displayBand1, editable = showBand_1)

displayBand2 = showBand_2 ? display.all : display.none
upperBand_2 = plot(upperBandValue2, title="Upper Band #2", color = color.olive, offset = offset, display = displayBand2, editable = showBand_2)
lowerBand_2 = plot(lowerBandValue2, title="Lower Band #2", color = color.olive, offset = offset, display = displayBand2, editable = showBand_2)
fill(upperBand_2, lowerBand_2,      title="Bands Fill #2", color = color.new(color.olive, 95),   display = displayBand2, editable = showBand_2)

displayBand3 = showBand_3 ? display.all : display.none
upperBand_3 = plot(upperBandValue3, title="Upper Band #3", color = color.teal, offset = offset, display = displayBand3, editable = showBand_3)
lowerBand_3 = plot(lowerBandValue3, title="Lower Band #3", color = color.teal, offset = offset, display = displayBand3, editable = showBand_3)
fill(upperBand_3, lowerBand_3,      title="Bands Fill #3", color = color.new(color.teal, 95),   display = displayBand3, editable = showBand_3)
```

---

## 3. Opening Range Breakout

**Authorship.** Pine v4, `study()`.

**What the vocabulary takes from it.** The highest and lowest price of a window
that starts at the session open, tracked while the window is open and then
**frozen** for the rest of the day.

Three details the implementation must honour:

* The extremes are **wick** high and low, not body.
* `hide = timeframe.isintraday and timeframe.multiplier <= inputMax` — the
  indicator refuses to draw when one chart bar is longer than the whole window.
  A 5-minute opening range cannot be resolved on a 15-minute chart. This is why
  the ORB bits must abstain per run on coarse rungs rather than emit a value.
* The plot colour goes transparent while the level is still moving
  (`orb_high[1] != orb_high ? na : color.green`). Cosmetic in Pine, but it
  encodes a real rule: **the level is not final until the window closes**, and a
  bit read during formation is a repaint.

The window is a **session string**, so any window is expressible — 5/15/30/60
minutes is the operator's chosen subset, not a limit of the method.

### Full source

```pine
//@version=4
study("ORB", overlay = true)

inputMax = input(5, title= "ORB total time (minutes)")
sess = input("0915-0920", type=input.session, title="Session Time")
t = time(timeframe.period, sess + ":1234567")
hide = timeframe.isintraday and timeframe.multiplier <= inputMax

is_newbar(res) => change(time(res)) != 0
in_session = not na(t)
is_first = in_session and not in_session[1]

orb_high = float(na)
orb_low = float(na)

if is_first
    orb_high := high
    orb_low := low
else
    orb_high := orb_high[1]
    orb_low := orb_low[1]
if high > orb_high and in_session
    orb_high := high
if low < orb_low and in_session
    orb_low := low

plot(hide ? orb_high : na , style=plot.style_line, color=orb_high[1] != orb_high ? na : color.green, title="ORB High", linewidth=2)
plot(hide ? orb_low : na , style=plot.style_line, color=orb_low[1] != orb_low ? na : color.red, title="ORB Low", linewidth=2)
```

---

## 4. The gap-leg rule

Not a Pine script. Supplied as a spreadsheet — `ES Trading Calculation.xlsx`,
sheets `GAPUP` and `GAPDOWN` — and read out of the cell formulas rather than the
row labels, because the two disagree.

| | Gap up | Gap down |
|---|---|---|
| X1 | previous day's **last 3-minute candle HIGH** | previous day's **last 3-minute candle LOW** |
| X2 | today's **first 3-minute candle HIGH** | today's **first 3-minute candle LOW** |
| X3 | `X2 - X1` | `X1 - X2` |
| X4 | `X3 / 2` | `X3 / 2` |
| **X5** | **`X2 - X4`** | **`X2 + X4`** |

`X5` is the **midpoint of the gap** in both directions — verified against all six
worked examples in the file, where `X5 == (X1 + X2) / 2` held exactly. In
Fibonacci terms it is **rung 0.5 of the gap leg**, which the eleven-rung ladder
at positions 132–142 contains as a strict superset.

**Two defects in the source file, recorded because the arithmetic is right and
the labels are not:**

1. The GAPUP row label reads `X5 = X1 - X4`, but cell `B7` computes
   `=SUM(B4-B6)`, which is `X2 - X4`. For the NIFTY example that is 24681.5
   against 24612.5 — **69 points apart**. The formula is correct; the label is
   wrong.
2. In GAPDOWN the NIFTY and BANKNIFTY rows hold each other's values —
   57305/57060 under "NIFTY", 24301/24240 under "BANK NIFTY". GAPUP has them the
   right way round.

**Why three minutes.** The fold grid is anchored at IST midnight and the NSE
open is **555 minutes** past it, so a rung's bars begin exactly at the open only
when its length divides 555. One, three, five and fifteen do. Thirty and sixty
do **not** — 555/30 is 18.5 — which leaves their first bar of the day a 15- and
45-minute stub. A rule that reads "the first candle of the day" reads a partial
bar on those rungs. See D-0077 and `Timeframe::aligns_with_the_open`.

---

## 5. What is still UNVERIFIED

* The **India VIX expected-move sheet** in the same workbook scales daily
  movement by `sqrt(365)` rather than `sqrt(252)`, which understates a daily move
  by a factor of 1.204 against a trading-day convention. `docs/00-charter.md`
  already carries the 365 divisor as an unverified convention. Separately,
  `CLAUDE.md` §1 makes `NSE-INDIAVIX` reference-only: it never enters the
  condition vocabulary, ranking, or run identity, so nothing from that sheet
  becomes a bit.
* The workbook's **Pivot sheet** computes `R3 = P + R2 - S1`, where the CPR
  script computes `R3 = R1 + (H - L)`. On the sheet's own BANKNIFTY numbers those
  differ by **8.67 points**. `S3` agrees on both. One of the two resistance
  ladders is wrong and this file does not decide which.
* **The CPR's narrow / wide cut points, `80` and `250` permille, have no source.**
  Positions 63, 274 and 275 — `narrow_cpr_day`, `wide_cpr_day`, `neutral_cpr_day` —
  need two thresholds and **no document this repository has read publishes one**.
  Every source in §1 gives the CPR's *construction* and none gives a width at which
  it becomes worth naming narrow. So they are UNVERIFIED, they are marked as such at
  their definition in `crates/indicators/src/daily.rs`, and they are a
  caller-supplied `CprWidth` rather than a hardcoded constant — `bits_with` takes
  the widths, so a result is never stamped with a threshold nobody can name.
  D-0096.

  **What IS verified is the ceiling they sit under, and it is derived rather than
  read.** With `pivot = (h+l+c)/3`, `bc = (h+l)/2` and `tc = 2·pivot − bc` — the
  construction §1 records — the width follows:

  ```text
  width = |tc − bc| = 2·|pivot − bc| = |2c − h − l| / 3
  ```

  `|2c − h − l|` is largest when the close sits exactly on the high or exactly on
  the low, where it equals `h − l`. So **a CPR can never exceed one third of the
  previous session's range**: 333 permille is the ceiling, not 1000.

  That changes what the two numbers mean. They are fractions of the *reachable* 333,
  not of 1000 — `80` is the bottom quarter and `250` the top quarter — and a "wide"
  cut of, say, 500 would be **unreachable**, making position 274 a constant false.
  That is the defect class D-0080 found in the 39 void forming-day positions, and it
  is excluded here by a const assertion (`CprWidth::CLASSICAL.wide < 333`) that fails
  the build rather than by care.

  **What would settle it** is a source that states a width threshold, or a measured
  distribution of `width / range` over real NIFTY and BANKNIFTY sessions with the
  quartiles read off it. The second is the honest route and it needs bar data; there
  is none on this machine, every pulled bar having been deleted deliberately. Until
  one or the other exists the two numbers stay UNVERIFIED and stay caller-supplied.
* **No `zone_mult` other than 1.0 has been considered.** The input allows
  0.1 upward in 0.1 steps. `CLAUDE.md` §6 is hostile to a tunable parameter for
  exactly the reason it gives about `k`, and nothing yet decides whether the
  zone width is fixed at the CPR half-width or is a swept parameter.

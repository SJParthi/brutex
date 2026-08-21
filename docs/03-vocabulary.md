# 03 — Condition vocabulary

74 conditions. Bit index is the identity: **never renumbered, never reused,
never reordered.** New conditions append at the next free bit.

The mask is `u128`. Bits 0–73 are live; 74–127 are free headroom — 54 more
conditions can be added without touching the mask type, the store, or any
existing result.

---

## 1. Why the index is frozen

A stored result set is a set of masks. A mask is a set of bit positions. If
bit 22 means `near_fib_50` today and `near_pivot_r4` after a helpful reorder,
every historical result silently means something different and no test can
detect it — the bytes are identical.

So: append only. A retired condition keeps its position forever as a tombstone
that always evaluates false. The position is never recycled.

---

## 2. Evaluation contract

For bar *i*, the evaluator produces one `u128` where bit *b* is set iff
condition *b* holds at that bar.

A candidate mask *M* **hits** bar *i* iff:

```rust
(bar_bits[i] & M) == M
```

This is the superset relation, and it is **anti-monotone**: adding a required
bit can only remove hits. Every pruning guarantee in the sweep rests on that
one property, so it is stated here rather than buried in the engine.

Bits are computed **once per slice** and shared read-only. They are never
recomputed per candidate, per worker, or per level.

---

## 3. Look-ahead

At bar *i* the evaluator may read bars `0..=i` and nothing else. State that
carries forward — moving averages, pivots, swing detection — updates **after**
the bar is emitted, never before.

Swing-based conditions (`bos_*`, `choch_*`, `near_swing_*`) confirm a swing
only *k* bars after it occurred. That latency is correct and must not be
"fixed": removing it would be look-ahead.

---

## 4. Conditions that do not apply to daily bars

Time-of-day bits (44–47) and VWAP bits (52–53) are meaningless on a 1-day bar.
On a daily timeframe they are cleared, not left as noise. A bit that cannot be
evaluated evaluates false — it never evaluates to "probably".

VWAP additionally requires traded volume. Spot indices carry none, so bits
52–53 permanently abstain on the two engine instruments. That is honest and
documented rather than quietly producing zeros that look like signal.

---

## 5. The table

### Moving averages — bits 0–5

| Bit | Name |
|---:|---|
| 0 | `close_above_ema20` |
| 1 | `close_below_ema20` |
| 2 | `close_above_ema200` |
| 3 | `close_below_ema200` |
| 4 | `ema20_above_ema200` |
| 5 | `ema20_below_ema200` |

### Classic pivots — bits 6–12

| Bit | Name |
|---:|---|
| 6 | `near_pivot_p` |
| 7 | `near_pivot_r1` |
| 8 | `near_pivot_r2` |
| 9 | `near_pivot_r3` |
| 10 | `near_pivot_s1` |
| 11 | `near_pivot_s2` |
| 12 | `near_pivot_s3` |

### Previous-day high / low — bits 13–18

| Bit | Name |
|---:|---|
| 13 | `close_above_pdh` |
| 14 | `close_below_pdh` |
| 15 | `close_above_pdl` |
| 16 | `close_below_pdl` |
| 17 | `near_pdh` |
| 18 | `near_pdl` |

### Fibonacci — bearish anchor (PDH) — bits 19–29

| Bit | Name |
|---:|---|
| 19 | `near_fib_0` |
| 20 | `near_fib_236` |
| 21 | `near_fib_382` |
| 22 | `near_fib_50` |
| 23 | `near_fib_618` |
| 24 | `near_fib_786` |
| 25 | `near_fib_100` |
| 26 | `near_fib_1272` |
| 27 | `near_fib_1618` |
| 28 | `near_fib_200` |
| 29 | `near_fib_2618` |

### Bar shape — bits 30–36

| Bit | Name |
|---:|---|
| 30 | `bar_bullish` |
| 31 | `bar_bearish` |
| 32 | `bar_doji` |
| 33 | `bar_large_body` |
| 34 | `bar_small_body` |
| 35 | `long_upper_wick` |
| 36 | `long_lower_wick` |

### Prior-bar sequence — bits 37–39

| Bit | Name |
|---:|---|
| 37 | `prior_n_bullish` |
| 38 | `prior_n_bearish` |
| 39 | `prior_alternating` |

### Position within the day — bits 40–43

| Bit | Name |
|---:|---|
| 40 | `close_at_day_high` |
| 41 | `close_at_day_low` |
| 42 | `close_in_upper_third` |
| 43 | `close_in_lower_third` |

### Time of day — bits 44–47

| Bit | Name |
|---:|---|
| 44 | `early_morning` |
| 45 | `mid_morning` |
| 46 | `midday` |
| 47 | `afternoon` |

### Day type — bits 48–51

| Bit | Name |
|---:|---|
| 48 | `gap_up_day` |
| 49 | `gap_down_day` |
| 50 | `inside_day` |
| 51 | `outside_day` |

### VWAP — bits 52–53

| Bit | Name |
|---:|---|
| 52 | `close_above_vwap` |
| 53 | `close_below_vwap` |

### Extended pivots — bits 54–55

| Bit | Name |
|---:|---|
| 54 | `near_pivot_r5` |
| 55 | `near_pivot_s5` |

### Market structure — bits 56–59

| Bit | Name |
|---:|---|
| 56 | `bos_bullish` |
| 57 | `bos_bearish` |
| 58 | `choch_bullish` |
| 59 | `choch_bearish` |

### Central pivot range — bits 60–63

| Bit | Name |
|---:|---|
| 60 | `above_cpr_tc` |
| 61 | `below_cpr_bc` |
| 62 | `inside_cpr` |
| 63 | `narrow_cpr_day` |

### SuperTrend — bits 64–65

| Bit | Name |
|---:|---|
| 64 | `close_above_supertrend` |
| 65 | `close_below_supertrend` |

### Gap midpoint — bits 66–68

| Bit | Name |
|---:|---|
| 66 | `close_above_gap_mid` |
| 67 | `close_below_gap_mid` |
| 68 | `near_gap_mid` |

### Fibonacci — bullish anchor (PDL) — bits 69–70

| Bit | Name |
|---:|---|
| 69 | `near_fib_bull_236` |
| 70 | `near_fib_bull_786` |

### Fibonacci — extension — bit 71

| Bit | Name |
|---:|---|
| 71 | `near_fib_424` |

### Swing levels — bits 72–73

| Bit | Name |
|---:|---|
| 72 | `near_swing_high` |
| 73 | `near_swing_low` |

---

## 6. Headroom

| | |
|---|---|
| live bits | 74 (0–73) |
| free bits | 54 (74–127) |
| mask type | `u128` |

Adding a condition: append at bit 74, implement the evaluator, add a row here,
add an entry to `docs/05-decisions.md`. No other file changes. No migration.
No existing result becomes invalid.

---

## 7. The appended positions — 74 to 273

Positions 0–73 above are the shipped vocabulary and this document is their
source: `vocab::table::the_shipped_74_are_the_document_character_for_character`
reads these rows and compares them to the code, so a rename here or there is a
failing build.

**For 74–273 the direction is reversed.** They were named in
`crates/vocab/src/table.rs` first, and these rows are the record rather than the
source. The test that guards them asserts *completeness* — every live position
appears here — not transcription, because a document generated from the code
cannot check the code.

Their formulas trace to `docs/09-design-sources.md`, which records the three
indicators and the gap-leg spreadsheet the design comes from.


### Pivot bands, outer relations — bits 74–85

| Bit | Name | Needs a tolerance | Status |
|---:|---|---|---|
| 74 | `close_above_pivot_r1_band` | — | live |
| 75 | `close_below_pivot_r1_band` | — | live |
| 76 | `close_above_pivot_r2_band` | — | live |
| 77 | `close_below_pivot_r2_band` | — | live |
| 78 | `close_above_pivot_r3_band` | — | live |
| 79 | `close_below_pivot_r3_band` | — | live |
| 80 | `close_above_pivot_s1_band` | — | live |
| 81 | `close_below_pivot_s1_band` | — | live |
| 82 | `close_above_pivot_s2_band` | — | live |
| 83 | `close_below_pivot_s2_band` | — | live |
| 84 | `close_above_pivot_s3_band` | — | live |
| 85 | `close_below_pivot_s3_band` | — | live |


### Opening range, four windows — bits 86–105

| Bit | Name | Needs a tolerance | Status |
|---:|---|---|---|
| 86 | `orb5_close_above_high` | — | live |
| 87 | `orb5_close_below_low` | — | live |
| 88 | `orb5_close_inside` | — | live |
| 89 | `orb5_near_high` | yes | live |
| 90 | `orb5_near_low` | yes | live |
| 91 | `orb15_close_above_high` | — | live |
| 92 | `orb15_close_below_low` | — | live |
| 93 | `orb15_close_inside` | — | live |
| 94 | `orb15_near_high` | yes | live |
| 95 | `orb15_near_low` | yes | live |
| 96 | `orb30_close_above_high` | — | live |
| 97 | `orb30_close_below_low` | — | live |
| 98 | `orb30_close_inside` | — | live |
| 99 | `orb30_near_high` | yes | live |
| 100 | `orb30_near_low` | yes | live |
| 101 | `orb60_close_above_high` | — | live |
| 102 | `orb60_close_below_low` | — | live |
| 103 | `orb60_close_inside` | — | live |
| 104 | `orb60_near_high` | yes | live |
| 105 | `orb60_near_low` | yes | live |


### Fibonacci, bullish anchor, extensions only — bits 106–109

| Bit | Name | Needs a tolerance | Status |
|---:|---|---|---|
| 106 | `near_fib_bull_1272` | yes | live |
| 107 | `near_fib_bull_1618` | yes | live |
| 108 | `near_fib_bull_200` | yes | live |
| 109 | `near_fib_bull_2618` | yes | live |


### Fibonacci over the last five sessions, static — bits 110–120

| Bit | Name | Needs a tolerance | Status |
|---:|---|---|---|
| 110 | `near_fib_prev5_0` | yes | live |
| 111 | `near_fib_prev5_236` | yes | live |
| 112 | `near_fib_prev5_382` | yes | live |
| 113 | `near_fib_prev5_50` | yes | live |
| 114 | `near_fib_prev5_618` | yes | live |
| 115 | `near_fib_prev5_786` | yes | live |
| 116 | `near_fib_prev5_100` | yes | live |
| 117 | `near_fib_prev5_1272` | yes | live |
| 118 | `near_fib_prev5_1618` | yes | live |
| 119 | `near_fib_prev5_200` | yes | live |
| 120 | `near_fib_prev5_2618` | yes | live |


### Fibonacci over the current session, running — bits 121–131

| Bit | Name | Needs a tolerance | Status |
|---:|---|---|---|
| 121 | `near_fib_curday_0` | yes | live |
| 122 | `near_fib_curday_236` | yes | live |
| 123 | `near_fib_curday_382` | yes | live |
| 124 | `near_fib_curday_50` | yes | live |
| 125 | `near_fib_curday_618` | yes | live |
| 126 | `near_fib_curday_786` | yes | live |
| 127 | `near_fib_curday_100` | yes | live |
| 128 | `near_fib_curday_1272` | yes | live |
| 129 | `near_fib_curday_1618` | yes | live |
| 130 | `near_fib_curday_200` | yes | live |
| 131 | `near_fib_curday_2618` | yes | live |


### Fibonacci over the opening gap leg — bits 132–142

| Bit | Name | Needs a tolerance | Status |
|---:|---|---|---|
| 132 | `near_fib_gap_0` | yes | live |
| 133 | `near_fib_gap_236` | yes | live |
| 134 | `near_fib_gap_382` | yes | live |
| 135 | `near_fib_gap_50` | yes | live |
| 136 | `near_fib_gap_618` | yes | live |
| 137 | `near_fib_gap_786` | yes | live |
| 138 | `near_fib_gap_100` | yes | live |
| 139 | `near_fib_gap_1272` | yes | live |
| 140 | `near_fib_gap_1618` | yes | live |
| 141 | `near_fib_gap_200` | yes | live |
| 142 | `near_fib_gap_2618` | yes | live |


### Session-anchored VWAP — bits 143–152

| Bit | Name | Needs a tolerance | Status |
|---:|---|---|---|
| 143 | `close_above_vwap_session` | — | live |
| 144 | `close_below_vwap_session` | — | live |
| 145 | `near_vwap_session` | yes | live |
| 146 | `close_above_vwap_band1_upper` | — | live |
| 147 | `close_below_vwap_band1_lower` | — | live |
| 148 | `close_above_vwap_band2_upper` | — | live |
| 149 | `close_below_vwap_band2_lower` | — | live |
| 150 | `near_vwap_band1_upper` | yes | live |
| 151 | `near_vwap_band1_lower` | yes | live |
| 152 | `inside_vwap_band1` | — | live |


### Candlestick patterns — bits 153–177

| Bit | Name | Needs a tolerance | Status |
|---:|---|---|---|
| 153 | `pat_hammer` | — | live |
| 154 | `pat_inverted_hammer` | — | live |
| 155 | `pat_hanging_man` | — | live |
| 156 | `pat_shooting_star` | — | live |
| 157 | `pat_bullish_engulfing` | — | live |
| 158 | `pat_bearish_engulfing` | — | live |
| 159 | `pat_bullish_harami` | — | live |
| 160 | `pat_bearish_harami` | — | live |
| 161 | `pat_piercing_line` | — | live |
| 162 | `pat_dark_cloud_cover` | — | live |
| 163 | `pat_morning_star` | — | live |
| 164 | `pat_evening_star` | — | live |
| 165 | `pat_three_white_soldiers` | — | live |
| 166 | `pat_three_black_crows` | — | live |
| 167 | `pat_three_inside_up` | — | live |
| 168 | `pat_three_inside_down` | — | live |
| 169 | `pat_tweezer_top` | — | live |
| 170 | `pat_tweezer_bottom` | — | live |
| 171 | `pat_spinning_top` | — | live |
| 172 | `pat_marubozu_bullish` | — | live |
| 173 | `pat_marubozu_bearish` | — | live |
| 174 | `pat_dragonfly_doji` | — | live |
| 175 | `pat_gravestone_doji` | — | live |
| 176 | `pat_rising_three_methods` | — | live |
| 177 | `pat_falling_three_methods` | — | live |


### The fourth and fifth pivot rungs, completed — bits 178–187

| Bit | Name | Needs a tolerance | Status |
|---:|---|---|---|
| 178 | `near_pivot_r4` | yes | live |
| 179 | `near_pivot_s4` | yes | live |
| 180 | `close_above_pivot_r4_band` | — | live |
| 181 | `close_below_pivot_r4_band` | — | live |
| 182 | `close_above_pivot_s4_band` | — | live |
| 183 | `close_below_pivot_s4_band` | — | live |
| 184 | `close_above_pivot_r5_band` | — | live |
| 185 | `close_below_pivot_r5_band` | — | live |
| 186 | `close_above_pivot_s5_band` | — | live |
| 187 | `close_below_pivot_s5_band` | — | live |


### BC and TC as levels in their own right — bits 188–189

| Bit | Name | Needs a tolerance | Status |
|---:|---|---|---|
| 188 | `near_cpr_tc` | yes | live |
| 189 | `near_cpr_bc` | yes | live |


### VWAP bands 2 and 3, completed — bits 190–197

| Bit | Name | Needs a tolerance | Status |
|---:|---|---|---|
| 190 | `near_vwap_band2_upper` | yes | live |
| 191 | `near_vwap_band2_lower` | yes | live |
| 192 | `inside_vwap_band2` | — | live |
| 193 | `close_above_vwap_band3_upper` | — | live |
| 194 | `close_below_vwap_band3_lower` | — | live |
| 195 | `near_vwap_band3_upper` | yes | live |
| 196 | `near_vwap_band3_lower` | yes | live |
| 197 | `inside_vwap_band3` | — | live |


### The rest of the classical candlestick set — bits 198–234

| Bit | Name | Needs a tolerance | Status |
|---:|---|---|---|
| 198 | `pat_abandoned_baby_bull` | — | live |
| 199 | `pat_abandoned_baby_bear` | — | live |
| 200 | `pat_three_line_strike_bull` | — | live |
| 201 | `pat_three_line_strike_bear` | — | live |
| 202 | `pat_kicker_bull` | — | live |
| 203 | `pat_kicker_bear` | — | live |
| 204 | `pat_belt_hold_bull` | — | live |
| 205 | `pat_belt_hold_bear` | — | live |
| 206 | `pat_counterattack_bull` | — | live |
| 207 | `pat_counterattack_bear` | — | live |
| 208 | `pat_separating_lines_bull` | — | live |
| 209 | `pat_separating_lines_bear` | — | live |
| 210 | `pat_on_neck` | — | live |
| 211 | `pat_in_neck` | — | live |
| 212 | `pat_thrusting` | — | live |
| 213 | `pat_tasuki_gap_up` | — | live |
| 214 | `pat_tasuki_gap_down` | — | live |
| 215 | `pat_side_by_side_white` | — | live |
| 216 | `pat_mat_hold` | — | live |
| 217 | `pat_stick_sandwich` | — | live |
| 218 | `pat_ladder_bottom` | — | live |
| 219 | `pat_ladder_top` | — | live |
| 220 | `pat_concealing_baby_swallow` | — | live |
| 221 | `pat_unique_three_river` | — | live |
| 222 | `pat_breakaway_bull` | — | live |
| 223 | `pat_breakaway_bear` | — | live |
| 224 | `pat_long_legged_doji` | — | live |
| 225 | `pat_four_price_doji` | — | live |
| 226 | `pat_rickshaw_man` | — | live |
| 227 | `pat_high_wave` | — | live |
| 228 | `pat_homing_pigeon` | — | live |
| 229 | `pat_matching_low` | — | live |
| 230 | `pat_identical_three_crows` | — | live |
| 231 | `pat_advance_block` | — | live |
| 232 | `pat_deliberation` | — | live |
| 233 | `pat_tri_star_bull` | — | live |
| 234 | `pat_tri_star_bear` | — | live |


### The current-FORMING-day pivot family — bits 235–273

| Bit | Name | Needs a tolerance | Status |
|---:|---|---|---|
| 235 | `near_forming_pivot_pivot` | — | **void** |
| 236 | `close_above_forming_pivot_pivot_band` | — | **void** |
| 237 | `close_below_forming_pivot_pivot_band` | — | **void** |
| 238 | `near_forming_pivot_cpr_bc` | — | **void** |
| 239 | `close_above_forming_pivot_cpr_bc_band` | — | **void** |
| 240 | `close_below_forming_pivot_cpr_bc_band` | — | **void** |
| 241 | `near_forming_pivot_cpr_tc` | — | **void** |
| 242 | `close_above_forming_pivot_cpr_tc_band` | — | **void** |
| 243 | `close_below_forming_pivot_cpr_tc_band` | — | **void** |
| 244 | `near_forming_pivot_r1` | — | **void** |
| 245 | `close_above_forming_pivot_r1_band` | — | **void** |
| 246 | `close_below_forming_pivot_r1_band` | — | **void** |
| 247 | `near_forming_pivot_r2` | — | **void** |
| 248 | `close_above_forming_pivot_r2_band` | — | **void** |
| 249 | `close_below_forming_pivot_r2_band` | — | **void** |
| 250 | `near_forming_pivot_r3` | — | **void** |
| 251 | `close_above_forming_pivot_r3_band` | — | **void** |
| 252 | `close_below_forming_pivot_r3_band` | — | **void** |
| 253 | `near_forming_pivot_r4` | — | **void** |
| 254 | `close_above_forming_pivot_r4_band` | — | **void** |
| 255 | `close_below_forming_pivot_r4_band` | — | **void** |
| 256 | `near_forming_pivot_r5` | — | **void** |
| 257 | `close_above_forming_pivot_r5_band` | — | **void** |
| 258 | `close_below_forming_pivot_r5_band` | — | **void** |
| 259 | `near_forming_pivot_s1` | — | **void** |
| 260 | `close_above_forming_pivot_s1_band` | — | **void** |
| 261 | `close_below_forming_pivot_s1_band` | — | **void** |
| 262 | `near_forming_pivot_s2` | — | **void** |
| 263 | `close_above_forming_pivot_s2_band` | — | **void** |
| 264 | `close_below_forming_pivot_s2_band` | — | **void** |
| 265 | `near_forming_pivot_s3` | — | **void** |
| 266 | `close_above_forming_pivot_s3_band` | — | **void** |
| 267 | `close_below_forming_pivot_s3_band` | — | **void** |
| 268 | `near_forming_pivot_s4` | — | **void** |
| 269 | `close_above_forming_pivot_s4_band` | — | **void** |
| 270 | `close_below_forming_pivot_s4_band` | — | **void** |
| 271 | `near_forming_pivot_s5` | — | **void** |
| 272 | `close_above_forming_pivot_s5_band` | — | **void** |
| 273 | `close_below_forming_pivot_s5_band` | — | **void** |
| 274 | `wide_cpr_day` | — | live |
| 275 | `neutral_cpr_day` | — | live |



### The close against the day's own open — bits 276–277

| Bit | Name | Fires when |
|---|---|---|
| 276 | `close_above_day_open` | `close > ` this session's first bar's open |
| 277 | `close_below_day_open` | `close < ` this session's first bar's open |

**A flat close sets neither.** Equality is a third state, exactly as D-0109 locked for 37
and 38 and as 30/31 already did for the bar's own open.

`Evaluator.running_open` was written on the first bar of every session and **read nowhere in
the repository** — a field maintained for a question the vocabulary could not ask. It is the
level every percent-change quote is measured against, and "up on the day" was inexpressible:
bits 40–43 give position within the day's *range*, which is a different question, and 30–31
compare the close to the *bar's* own open.

No threshold, so nothing here is UNVERIFIED.

### The market structure in force — bits 278–279

| Bit | Name | Fires when |
|---|---|---|
| 278 | `structure_up_in_force` | the last break was upward |
| 279 | `structure_down_in_force` | the last break was downward |

Bits 56–59 are **break events** — `bos_up`, `bos_down`, `choch_up`, `choch_down` — each true
only on the handful of bars where a level was taken out. `Structure::last` holds which
direction is in force *between* those events, which is the regime, and it was unpublished: a
sweep could ask "did structure break up on this bar" and could not ask "is the structure
up".

**Neither is set before the first break.** There is no structure yet, and §4 forbids a bit
evaluating to "probably". Same source and same latch as 56–59, so no new formula and no
threshold.


### The moment a level changed side — bits 280–313, D-0244

Every level relation above this point is a **state**: `close_above_ema20` is true on bar
500, on bar 501, and on every bar of a two-hour trend. Nothing distinguished *just crossed*
from *has been above since this morning*, and a grep for `cross` across the whole table
returned zero. Even the break family is a state — `bos_up` fires on every bar above the
swing high, not only the first.

A crossing is two states one bar apart, and both states were already here, so these are
**derived** rather than measured: `crossed_up_X` is `close_above_X` clear on the previous
bar of this session and set on this one. No indicator module computes anything new.

**Seventeen levels, not thirty-five.** Sixteen `close_above_` names are `void` by D-0080,
| 280 | `crossed_up_ema20` | `close_above_ema20` was clear on the previous bar and is set now |
| 281 | `crossed_down_ema20` | `close_below_ema20` was clear on the previous bar and is set now |
| 282 | `crossed_up_ema200` | `close_above_ema200` was clear on the previous bar and is set now |
| 283 | `crossed_down_ema200` | `close_below_ema200` was clear on the previous bar and is set now |
| 284 | `crossed_up_pdh` | `close_above_pdh` was clear on the previous bar and is set now |
| 285 | `crossed_down_pdh` | `close_below_pdh` was clear on the previous bar and is set now |
| 286 | `crossed_up_pdl` | `close_above_pdl` was clear on the previous bar and is set now |
| 287 | `crossed_down_pdl` | `close_below_pdl` was clear on the previous bar and is set now |
| 288 | `crossed_up_supertrend` | `close_above_supertrend` was clear on the previous bar and is set now |
| 289 | `crossed_down_supertrend` | `close_below_supertrend` was clear on the previous bar and is set now |
| 290 | `crossed_up_gap_mid` | `close_above_gap_mid` was clear on the previous bar and is set now |
| 291 | `crossed_down_gap_mid` | `close_below_gap_mid` was clear on the previous bar and is set now |
| 292 | `crossed_up_pivot_r1_band` | `close_above_pivot_r1_band` was clear on the previous bar and is set now |
| 293 | `crossed_down_pivot_r1_band` | `close_below_pivot_r1_band` was clear on the previous bar and is set now |
| 294 | `crossed_up_pivot_r2_band` | `close_above_pivot_r2_band` was clear on the previous bar and is set now |
| 295 | `crossed_down_pivot_r2_band` | `close_below_pivot_r2_band` was clear on the previous bar and is set now |
| 296 | `crossed_up_pivot_r3_band` | `close_above_pivot_r3_band` was clear on the previous bar and is set now |
| 297 | `crossed_down_pivot_r3_band` | `close_below_pivot_r3_band` was clear on the previous bar and is set now |
| 298 | `crossed_up_pivot_s1_band` | `close_above_pivot_s1_band` was clear on the previous bar and is set now |
| 299 | `crossed_down_pivot_s1_band` | `close_below_pivot_s1_band` was clear on the previous bar and is set now |
| 300 | `crossed_up_pivot_s2_band` | `close_above_pivot_s2_band` was clear on the previous bar and is set now |
| 301 | `crossed_down_pivot_s2_band` | `close_below_pivot_s2_band` was clear on the previous bar and is set now |
| 302 | `crossed_up_pivot_s3_band` | `close_above_pivot_s3_band` was clear on the previous bar and is set now |
| 303 | `crossed_down_pivot_s3_band` | `close_below_pivot_s3_band` was clear on the previous bar and is set now |
| 304 | `crossed_up_pivot_r4_band` | `close_above_pivot_r4_band` was clear on the previous bar and is set now |
| 305 | `crossed_down_pivot_r4_band` | `close_below_pivot_r4_band` was clear on the previous bar and is set now |
| 306 | `crossed_up_pivot_s4_band` | `close_above_pivot_s4_band` was clear on the previous bar and is set now |
| 307 | `crossed_down_pivot_s4_band` | `close_below_pivot_s4_band` was clear on the previous bar and is set now |
| 308 | `crossed_up_pivot_r5_band` | `close_above_pivot_r5_band` was clear on the previous bar and is set now |
| 309 | `crossed_down_pivot_r5_band` | `close_below_pivot_r5_band` was clear on the previous bar and is set now |
| 310 | `crossed_up_pivot_s5_band` | `close_above_pivot_s5_band` was clear on the previous bar and is set now |
| 311 | `crossed_down_pivot_s5_band` | `close_below_pivot_s5_band` was clear on the previous bar and is set now |
| 312 | `crossed_up_day_open` | `close_above_day_open` was clear on the previous bar and is set now |
| 313 | `crossed_down_day_open` | `close_below_day_open` was clear on the previous bar and is set now |

### Which test of this level this is — bits 314–364, D-0246

Bits 280–313 gave the vocabulary an edge; these give that edge an **ordinal**, and the
difference is one a trader acts on. `crossed_up_pdh` fires byte-identically on the clean
first break of the day and on the ninth probe of a level that has already absorbed eight
attempts.

**No combination of existing bits can express a count.** `ConditionMask::hits` is pure
conjunction, so *not the first test* cannot be spelled by leaving a bit clear, and a tally
cannot be assembled out of level states however many are ANDed. It has to be positions.

**Counting crossings, not touches, and that choice is what made it fit.** No document
defines a touch — `near_X`, a close on the level, and a wick through it are three different
predicates — so picking one would be a stated assumption needing its own entry. A crossing
is already defined, already derived and already tested, so the ordinal inherits its whole
definition from 280–313 and introduces none.

**Three buckets, exactly one of which fires.** First, second, third-or-later. A bit per
exact count would be unbounded; the third is open-ended because the distinction a trader
draws is fresh / retest / being hammered, not fourth from fifth. They partition the crossing
bars — one on a bar that crosses, none on a bar that does not — which is what makes them
safe to AND with anything else.

**Per session.** The count is cleared in the rollover beside the crossing state: a level
crossed twice yesterday and once today is on its first test today, not its third.

| Bit | Name | Fires when |
|---|---|---|
| 314 | `first_cross_ema20` | this bar is the session's first crossing of `ema20`, in either direction |
| 315 | `second_cross_ema20` | this bar is the session's second crossing of `ema20`, in either direction |
| 316 | `third_plus_cross_ema20` | this bar is the session's third or any later crossing of `ema20`, in either direction |
| 317 | `first_cross_ema200` | this bar is the session's first crossing of `ema200`, in either direction |
| 318 | `second_cross_ema200` | this bar is the session's second crossing of `ema200`, in either direction |
| 319 | `third_plus_cross_ema200` | this bar is the session's third or any later crossing of `ema200`, in either direction |
| 320 | `first_cross_pdh` | this bar is the session's first crossing of `pdh`, in either direction |
| 321 | `second_cross_pdh` | this bar is the session's second crossing of `pdh`, in either direction |
| 322 | `third_plus_cross_pdh` | this bar is the session's third or any later crossing of `pdh`, in either direction |
| 323 | `first_cross_pdl` | this bar is the session's first crossing of `pdl`, in either direction |
| 324 | `second_cross_pdl` | this bar is the session's second crossing of `pdl`, in either direction |
| 325 | `third_plus_cross_pdl` | this bar is the session's third or any later crossing of `pdl`, in either direction |
| 326 | `first_cross_supertrend` | this bar is the session's first crossing of `supertrend`, in either direction |
| 327 | `second_cross_supertrend` | this bar is the session's second crossing of `supertrend`, in either direction |
| 328 | `third_plus_cross_supertrend` | this bar is the session's third or any later crossing of `supertrend`, in either direction |
| 329 | `first_cross_gap_mid` | this bar is the session's first crossing of `gap_mid`, in either direction |
| 330 | `second_cross_gap_mid` | this bar is the session's second crossing of `gap_mid`, in either direction |
| 331 | `third_plus_cross_gap_mid` | this bar is the session's third or any later crossing of `gap_mid`, in either direction |
| 332 | `first_cross_pivot_r1_band` | this bar is the session's first crossing of `pivot_r1_band`, in either direction |
| 333 | `second_cross_pivot_r1_band` | this bar is the session's second crossing of `pivot_r1_band`, in either direction |
| 334 | `third_plus_cross_pivot_r1_band` | this bar is the session's third or any later crossing of `pivot_r1_band`, in either direction |
| 335 | `first_cross_pivot_r2_band` | this bar is the session's first crossing of `pivot_r2_band`, in either direction |
| 336 | `second_cross_pivot_r2_band` | this bar is the session's second crossing of `pivot_r2_band`, in either direction |
| 337 | `third_plus_cross_pivot_r2_band` | this bar is the session's third or any later crossing of `pivot_r2_band`, in either direction |
| 338 | `first_cross_pivot_r3_band` | this bar is the session's first crossing of `pivot_r3_band`, in either direction |
| 339 | `second_cross_pivot_r3_band` | this bar is the session's second crossing of `pivot_r3_band`, in either direction |
| 340 | `third_plus_cross_pivot_r3_band` | this bar is the session's third or any later crossing of `pivot_r3_band`, in either direction |
| 341 | `first_cross_pivot_s1_band` | this bar is the session's first crossing of `pivot_s1_band`, in either direction |
| 342 | `second_cross_pivot_s1_band` | this bar is the session's second crossing of `pivot_s1_band`, in either direction |
| 343 | `third_plus_cross_pivot_s1_band` | this bar is the session's third or any later crossing of `pivot_s1_band`, in either direction |
| 344 | `first_cross_pivot_s2_band` | this bar is the session's first crossing of `pivot_s2_band`, in either direction |
| 345 | `second_cross_pivot_s2_band` | this bar is the session's second crossing of `pivot_s2_band`, in either direction |
| 346 | `third_plus_cross_pivot_s2_band` | this bar is the session's third or any later crossing of `pivot_s2_band`, in either direction |
| 347 | `first_cross_pivot_s3_band` | this bar is the session's first crossing of `pivot_s3_band`, in either direction |
| 348 | `second_cross_pivot_s3_band` | this bar is the session's second crossing of `pivot_s3_band`, in either direction |
| 349 | `third_plus_cross_pivot_s3_band` | this bar is the session's third or any later crossing of `pivot_s3_band`, in either direction |
| 350 | `first_cross_pivot_r4_band` | this bar is the session's first crossing of `pivot_r4_band`, in either direction |
| 351 | `second_cross_pivot_r4_band` | this bar is the session's second crossing of `pivot_r4_band`, in either direction |
| 352 | `third_plus_cross_pivot_r4_band` | this bar is the session's third or any later crossing of `pivot_r4_band`, in either direction |
| 353 | `first_cross_pivot_s4_band` | this bar is the session's first crossing of `pivot_s4_band`, in either direction |
| 354 | `second_cross_pivot_s4_band` | this bar is the session's second crossing of `pivot_s4_band`, in either direction |
| 355 | `third_plus_cross_pivot_s4_band` | this bar is the session's third or any later crossing of `pivot_s4_band`, in either direction |
| 356 | `first_cross_pivot_r5_band` | this bar is the session's first crossing of `pivot_r5_band`, in either direction |
| 357 | `second_cross_pivot_r5_band` | this bar is the session's second crossing of `pivot_r5_band`, in either direction |
| 358 | `third_plus_cross_pivot_r5_band` | this bar is the session's third or any later crossing of `pivot_r5_band`, in either direction |
| 359 | `first_cross_pivot_s5_band` | this bar is the session's first crossing of `pivot_s5_band`, in either direction |
| 360 | `second_cross_pivot_s5_band` | this bar is the session's second crossing of `pivot_s5_band`, in either direction |
| 361 | `third_plus_cross_pivot_s5_band` | this bar is the session's third or any later crossing of `pivot_s5_band`, in either direction |
| 362 | `first_cross_day_open` | this bar is the session's first crossing of `day_open`, in either direction |
| 363 | `second_cross_day_open` | this bar is the session's second crossing of `day_open`, in either direction |
| 364 | `third_plus_cross_day_open` | this bar is the session's third or any later crossing of `day_open`, in either direction |

---

## 8. Headroom, restated

| | |
|---|---|
| positions allocated | 365 (0–364) |
| live | 323 |
| retired (duplicated a live position) | 3 — bits 6, 19, 25 |
| **void** (definitionally constant) | **39** — bits 235–273, D-0080 |
| declaring `Kind::Near`, needing a tolerance | 81 live, of 97 allocated |
| mask type | `ConditionMask`, `[u64; 6]` |
| mask width | 384 bits |
| free positions | 19 |

The first five rows account for every allocated position exactly once:
323 + 3 + 39 = 365. If a future append breaks that sum, this table is the stale
copy and the table in `crates/vocab/src/table.rs` is the truth.

**Nineteen free, and that number is now the constraint.** The crossing family
(D-0244) took the table 280 to 314 and its ordinals (D-0246) took it to 365, both
against the same 384-bit mask — so neither widened it and `VOCAB_VERSION` holds
at 3. The next family needing more than nineteen positions cannot be an append.
It widens `ConditionMask::WORDS`, which IS a version bump, and re-keys every run
ever recorded: the same sweep over the same bars produces a different identity,
and nothing stored can be found by the identity a rerun computes.

**§6 above is superseded and kept for the record.** It said 74 live bits in a
`u128` with 54 free, and that was true until this table widened. The `u128` claim
in §6 and the sentence *"No other file changes. No migration."* were both true up
to bit 128 and stop being true at 129 — the mask width is a term in run identity
(`CLAUDE.md` §3 rule 3), so widening re-keys every historical run rather than
reinterpreting it. `VOCAB_VERSION` carries that.


### What a flat bar does to 37, 38 and 39 — D-0109

A bar with `close == open` is **neither direction**. It sets neither 37 `prior_n_bullish`
nor 38 `prior_n_bearish`, and it **breaks** 39 `prior_alternating`, because a bar that is
neither direction cannot be the opposite of its neighbour.

This matches what 30 `bar_bullish` and 31 `bar_up`/31 `bar_bearish` already do for the
current bar — a flat bar sets neither, and 32 `bar_doji` is what fires instead. The
prior-direction ring used to store a `bool` and file a flat bar as bearish, so 38 asserted
three bearish bars over a run in which 31 never fired once. It now stores an
`Option<Ordering>`.

The rule is recorded here because it was a semantic decision living only in a code
comment, and masks are built on it.

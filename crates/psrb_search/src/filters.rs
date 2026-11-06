//! The filter library: SEARCH-SPEC 3's 74 fixed filters (ids 0..=73, table order) and
//! the operator's ADDENDUM 3 / 4 library (ids 74..=170). Each filter has a LONG test and
//! a SHORT test (its mirror, or the same test when the table says "both"); the tests
//! themselves are computed in `phase_a::filters_at`, on data up to the signal candle's
//! close only. Not ready means false (SEARCH-SPEC 3.1).

/// One filter: its id, its group (a combination takes at most one filter of a group)
/// and its words for the reports.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Def {
    pub(crate) id: &'static str,
    pub(crate) group: &'static str,
    pub(crate) words: &'static str,
}

/// SEARCH-SPEC 3's library.
pub(crate) const N_BASE: usize = 74;
/// ADDENDUM 3 / 4's library (candle anatomy, named patterns, chart structures,
/// Fibonacci, minimum expected move).
pub(crate) const N_L2: usize = 97;
/// Every fixed filter.
pub(crate) const N_FILT: usize = N_BASE + N_L2;
/// Words of filter bits per record.
pub(crate) const FW: usize = 3;

const fn d(id: &'static str, group: &'static str, words: &'static str) -> Def {
    Def { id, group, words }
}

/// The library in id order.
pub(crate) const DEFS: [Def; N_FILT] = [
    // (a) indicators: 30 filters, 18 groups
    d("A01", "RSI", "RSI 14 above 60 (short: below 40)"),
    d("A02", "RSI", "RSI 14 above 70 (short: below 30)"),
    d("A03", "RSICAP", "RSI 14 not above 80 (short: not below 20)"),
    d("A04", "WR", "Williams %R 14 above -85 (short: below -15)"),
    d("A05", "WR", "Williams %R 14 above -50 (short: below -50)"),
    d("A06", "WR", "Williams %R 14 above -15 (short: below -85)"),
    d("A07", "CCI", "CCI 20 above +100 (short: below -100)"),
    d("A08", "CCI", "CCI 20 above +200 (short: below -200)"),
    d("A09", "BBW", "close beyond Bollinger 20 at 2.5 sigma"),
    d("A10", "BBW", "close beyond Bollinger 20 at 3.0 sigma"),
    d("A11", "EMA50", "close above EMA 50 (short: below)"),
    d("A12", "EMA50S", "EMA 50 rising over 5 candles (short: falling)"),
    d("A13", "EMA200", "close above EMA 200 (short: below)"),
    d("A14", "EMA200S", "EMA 200 rising over 10 candles (short: falling)"),
    d("A15", "EMASTK", "EMA 50 above EMA 200 (short: below)"),
    d("A16", "ADX", "ADX 14 at least 20 with +DI above -DI (short: -DI above +DI)"),
    d("A17", "ADX", "ADX 14 at least 30 with +DI above -DI (short: -DI above +DI)"),
    d("A18", "ATRD", "daily ATR 14 at least 2.5% of the previous close"),
    d("A19", "ATRD", "daily ATR 14 at most 1.5% of the previous close"),
    d("A20", "MACD", "MACD (12, 26, 9) histogram above 0 (short: below 0)"),
    d("A21", "MACD", "MACD line above 0 and histogram rising (short: below 0 and falling)"),
    d("A22", "STOCH", "Stochastic (14, 3) %K at least 80 and above %D (short: at most 20 and below %D)"),
    d("A23", "STOCH", "Stochastic (14, 3) %K at least 50 and above %D (short: at most 50 and below %D)"),
    d("A24", "VWAP", "close above the session VWAP (short: below)"),
    d("A25", "VWAP", "close above the session VWAP by at most 0.5% (short: below by at most 0.5%)"),
    d("A26", "VWAP", "close at least 1.0% beyond the session VWAP"),
    d("A27", "VOL", "candle volume at least 1.5x its 20-candle average"),
    d("A28", "VOL", "candle volume at least 2x its 20-candle average"),
    d("A29", "RVOL", "day volume so far at least 1.5x the 10-session mean at the same minute"),
    d("A30", "OBV", "OBV rising over 10 candles (short: falling)"),
    // (b) price context: 16 filters, 6 groups
    d("B01", "PDHL", "close beyond the previous day's high (short: low)"),
    d("B02", "PDHL", "first close of the session beyond the previous day's high (short: low)"),
    d("B03", "ORB", "close beyond the 15-minute opening range, from 09:30"),
    d("B04", "ORB", "close beyond the 30-minute opening range, from 09:45"),
    d("B05", "GAP", "gap of at least 0.5% in the trade's direction"),
    d("B06", "GAP", "gap of at least 0.5% against the trade"),
    d("B07", "GAP", "gap within 0.25% (flat open)"),
    d("B08", "DAYEXT", "close within 0.25% of the day's high (short: low)"),
    d("B09", "DAYEXT", "close at most 1.0% above the day's low (short: below the day's high): a young move"),
    d("B10", "CPR", "CPR width within 10 bp"),
    d("B11", "CPR", "CPR width within 20 bp"),
    d("B12", "CPR", "CPR width at least 50 bp (wide day)"),
    d("B13", "PIV", "close beyond R1 (short: S1)"),
    d("B14", "PIV", "close beyond R2 (short: S2)"),
    d("B15", "PIV", "close between R1 and R2 (short: between S1 and S2)"),
    d("B16", "PIV", "close above the pivot P (short: below)"),
    // (c) bars and candles: 12 filters, 6 groups
    d("C01", "PAT", "bullish engulfing (short: bearish engulfing)"),
    d("C02", "PAT", "hammer (short: shooting star)"),
    d("C03", "PAT", "bullish marubozu (short: bearish marubozu)"),
    d("C04", "PAT", "three white soldiers (short: three black crows)"),
    d("C05", "PRIOR", "a doji just before the signal candle"),
    d("C06", "PRIOR", "an inside bar just before, broken by the signal candle's close"),
    d("C07", "PRIOR", "an NR4 candle just before the signal candle"),
    d("C08", "PRIOR", "the signal candle is an outside bar closing in the trade's direction"),
    d("C09", "CLOC", "close in the top 20% of the candle (short: bottom 20%)"),
    d("C10", "BODY", "a big body (at least 60% of the range) in the trade's direction"),
    d("C11", "RUN3", "three rising closes, all up candles (short: three falling, all down)"),
    d("C12", "WRB", "a wide-range candle (at least 2x the 10-candle average range)"),
    // (d) chart structure: 5 filters, 3 groups
    d("D01", "BRK", "close beyond the last 20 candles' high (short: low)"),
    d("D02", "BRK", "close beyond the last 50 candles' high (short: low)"),
    d("D03", "HHHL", "three candles of higher highs and higher lows (short: lower)"),
    d("D04", "HHHL", "the last 10 candles above the 10 before in highs and lows (short: below)"),
    d("D05", "SQZ", "a Bollinger squeeze (width in its lowest 20% of 120 candles) then a 2-sigma break"),
    // (e) time and market: 11 filters, 6 groups
    d("E01", "TIME", "signal at or before 11:00"),
    d("E02", "TIME", "signal after 11:00 and at or before 13:30"),
    d("E03", "TIME", "signal after 13:30"),
    d("E04", "TOPEN", "signal at or after 09:30 (skip the first 15 minutes)"),
    d("E05", "DOW", "not a Monday"),
    d("E06", "DOW", "not a Friday"),
    d("E07", "DOW", "Tuesday, Wednesday or Thursday"),
    d("E08", "NIFTY", "NIFTY 50 up on the day at that minute (short: down)"),
    d("E09", "NIFTY", "NIFTY 50 above its EMA 20 on the signal timeframe (short: below)"),
    d("E10", "RS", "the stock's day move at least 0.5 points above NIFTY's (short: below)"),
    d("E11", "DAYTR", "close above the day's open and the previous close (short: below both)"),
    // ADDENDUM 4 (1): candle anatomy of the signal candle (0) and the two before it (1, 2), in the session
    d("F01", "BODY0", "signal candle body under 20% of its range"),
    d("F02", "BODY0", "signal candle body 20-50% of its range"),
    d("F03", "BODY0", "signal candle body 50-80% of its range"),
    d("F04", "BODY0", "signal candle body at least 80% of its range"),
    d("F05", "BODY1", "candle t-1 body under 20% of its range"),
    d("F06", "BODY1", "candle t-1 body 20-50% of its range"),
    d("F07", "BODY1", "candle t-1 body 50-80% of its range"),
    d("F08", "BODY1", "candle t-1 body at least 80% of its range"),
    d("F09", "BODY2", "candle t-2 body under 20% of its range"),
    d("F10", "BODY2", "candle t-2 body 20-50% of its range"),
    d("F11", "BODY2", "candle t-2 body 50-80% of its range"),
    d("F12", "BODY2", "candle t-2 body at least 80% of its range"),
    d("F13", "UW0", "signal candle upper wick under 20% of range (short: lower wick)"),
    d("F14", "UW0", "signal candle upper wick 20-50% of range (short: lower wick)"),
    d("F15", "UW0", "signal candle upper wick 50-80% of range (short: lower wick)"),
    d("F16", "UW0", "signal candle upper wick at least 80% of range (short: lower wick)"),
    d("F17", "UW1", "candle t-1 upper wick under 20% of range (short: lower wick)"),
    d("F18", "UW1", "candle t-1 upper wick 20-50% of range (short: lower wick)"),
    d("F19", "UW1", "candle t-1 upper wick 50-80% of range (short: lower wick)"),
    d("F20", "UW1", "candle t-1 upper wick at least 80% of range (short: lower wick)"),
    d("F21", "UW2", "candle t-2 upper wick under 20% of range (short: lower wick)"),
    d("F22", "UW2", "candle t-2 upper wick 20-50% of range (short: lower wick)"),
    d("F23", "UW2", "candle t-2 upper wick 50-80% of range (short: lower wick)"),
    d("F24", "UW2", "candle t-2 upper wick at least 80% of range (short: lower wick)"),
    d("F25", "LW0", "signal candle lower wick under 20% of range (short: upper wick)"),
    d("F26", "LW0", "signal candle lower wick 20-50% of range (short: upper wick)"),
    d("F27", "LW0", "signal candle lower wick 50-80% of range (short: upper wick)"),
    d("F28", "LW0", "signal candle lower wick at least 80% of range (short: upper wick)"),
    d("F29", "LW1", "candle t-1 lower wick under 20% of range (short: upper wick)"),
    d("F30", "LW1", "candle t-1 lower wick 20-50% of range (short: upper wick)"),
    d("F31", "LW1", "candle t-1 lower wick 50-80% of range (short: upper wick)"),
    d("F32", "LW1", "candle t-1 lower wick at least 80% of range (short: upper wick)"),
    d("F33", "LW2", "candle t-2 lower wick under 20% of range (short: upper wick)"),
    d("F34", "LW2", "candle t-2 lower wick 20-50% of range (short: upper wick)"),
    d("F35", "LW2", "candle t-2 lower wick 50-80% of range (short: upper wick)"),
    d("F36", "LW2", "candle t-2 lower wick at least 80% of range (short: upper wick)"),
    d("F37", "CP0", "signal candle closes in the top third of its range (short: bottom third)"),
    d("F38", "CP0", "signal candle closes in the middle third of its range"),
    d("F39", "CP0", "signal candle closes in the bottom third of its range (short: top third)"),
    d("F40", "CP1", "candle t-1 closes in the top third (short: bottom third)"),
    d("F41", "CP1", "candle t-1 closes in the middle third"),
    d("F42", "CP1", "candle t-1 closes in the bottom third (short: top third)"),
    d("F43", "CP2", "candle t-2 closes in the top third (short: bottom third)"),
    d("F44", "CP2", "candle t-2 closes in the middle third"),
    d("F45", "CP2", "candle t-2 closes in the bottom third (short: top third)"),
    d("F46", "RA0", "signal candle range below 0.7x the 14-candle ATR"),
    d("F47", "RA0", "signal candle range 0.7x-1.3x the 14-candle ATR"),
    d("F48", "RA0", "signal candle range above 1.3x the 14-candle ATR"),
    d("F49", "RA1", "candle t-1 range below 0.7x the 14-candle ATR"),
    d("F50", "RA1", "candle t-1 range 0.7x-1.3x the 14-candle ATR"),
    d("F51", "RA1", "candle t-1 range above 1.3x the 14-candle ATR"),
    d("F52", "RA2", "candle t-2 range below 0.7x the 14-candle ATR"),
    d("F53", "RA2", "candle t-2 range 0.7x-1.3x the 14-candle ATR"),
    d("F54", "RA2", "candle t-2 range above 1.3x the 14-candle ATR"),
    // ADDENDUM 4 (3): named candlestick patterns (pattern.rs, classical thresholds, UNVERIFIED)
    d("N01", "NPAT", "inverted hammer (short: hanging man)"),
    d("N02", "NPAT", "bullish harami (short: bearish harami)"),
    d("N03", "NPAT", "piercing line (short: dark cloud cover)"),
    d("N04", "NPAT", "morning star (short: evening star)"),
    d("N05", "NPAT", "three inside up (short: three inside down)"),
    d("N06", "NPAT", "tweezer bottom (short: tweezer top)"),
    d("N07", "NPAT", "spinning top"),
    d("N08", "NPAT", "dragonfly doji (short: gravestone doji)"),
    d("N09", "NPAT", "rising three methods (short: falling three methods)"),
    d("N10", "NPAT", "abandoned baby bull (short: bear)"),
    d("N11", "NPAT", "three line strike bull (short: bear)"),
    d("N12", "NPAT", "kicker bull (short: bear)"),
    d("N13", "NPAT", "belt hold bull (short: bear)"),
    d("N14", "NPAT", "counterattack bull (short: bear)"),
    d("N15", "NPAT", "separating lines bull (short: bear)"),
    d("N16", "NPAT", "long-legged doji"),
    d("N17", "NPAT", "the signal candle is a doji (body at most 10% of the range)"),
    // ADDENDUM 4 (4): named chart structures on the signal timeframe
    d("G01", "CHART", "a fresh close beyond the last fractal-2 swing high (short: swing low)"),
    d("G02", "CHART", "a fresh close beyond the last fractal-3 swing high (short: swing low)"),
    d("G03", "CHART", "the last two fractal-2 swings make a higher high and a higher low (short: lower)"),
    d("G04", "CHART", "a double bottom (two fractal-3 lows within 0.3%) broken above its neckline (short: double top)"),
    d("G05", "CHART", "a flag: a 1% impulse over 10 candles, a 5-candle pause within half of it, then a break"),
    d("G06", "CHART", "a triangle squeeze (lower highs and higher lows over 10 candles) then a break"),
    d("G07", "CHART", "a range breakout in the last 10 candles, retested by this candle and held"),
    // ADDENDUM 4 (5): Fibonacci retracements with a reversal close
    d("H01", "FIBSW", "first touch of the 38.2% retracement of the last fractal-3 swing, closing back"),
    d("H02", "FIBSW", "second touch of the 38.2% retracement of the last swing, closing back"),
    d("H03", "FIBSW", "first touch of the 50% retracement of the last swing, closing back"),
    d("H04", "FIBSW", "second touch of the 50% retracement of the last swing, closing back"),
    d("H05", "FIBSW", "first touch of the 61.8% retracement of the last swing, closing back"),
    d("H06", "FIBSW", "second touch of the 61.8% retracement of the last swing, closing back"),
    d("H07", "FIBSW", "first touch of the 78.6% retracement of the last swing, closing back"),
    d("H08", "FIBSW", "second touch of the 78.6% retracement of the last swing, closing back"),
    d("H09", "FIBPD", "first touch today of the 38.2% retracement of the previous day's range, closing back"),
    d("H10", "FIBPD", "second touch today of the 38.2% retracement of the previous day's range"),
    d("H11", "FIBPD", "first touch today of the 50% retracement of the previous day's range"),
    d("H12", "FIBPD", "second touch today of the 50% retracement of the previous day's range"),
    d("H13", "FIBPD", "first touch today of the 61.8% retracement of the previous day's range"),
    d("H14", "FIBPD", "second touch today of the 61.8% retracement of the previous day's range"),
    d("H15", "FIBPD", "first touch today of the 78.6% retracement of the previous day's range"),
    d("H16", "FIBPD", "second touch today of the 78.6% retracement of the previous day's range"),
    // ADDENDUM 3 (3): a minimum expected move
    d("M01", "MOVE", "the signal timeframe's ATR 14 at least 0.25% of the close"),
    d("M02", "MOVE", "the signal timeframe's ATR 14 at least 0.50% of the close"),
    d("M03", "MOVE", "the signal timeframe's ATR 14 at least 1.00% of the close"),
];

/// The group number of every filter (groups numbered in order of first appearance).
pub(crate) fn groups() -> ([u8; N_FILT], usize) {
    let mut names: Vec<&str> = Vec::new();
    let mut out = [0_u8; N_FILT];
    for (i, def) in DEFS.iter().enumerate() {
        let g = match names.iter().position(|n| *n == def.group) {
            Some(p) => p,
            None => {
                names.push(def.group);
                names.len() - 1
            }
        };
        if let Some(slot) = out.get_mut(i) {
            *slot = u8::try_from(g).unwrap_or(u8::MAX);
        }
    }
    (out, names.len())
}

/// Set bit `i` of a filter word array.
pub(crate) fn set(bits: &mut [u64; FW], i: usize, on: bool) {
    if on && let Some(w) = bits.get_mut(i / 64) {
        *w |= 1_u64 << (i % 64);
    }
}

/// Bit `i` of a filter word array.
pub(crate) fn get(bits: &[u64; FW], i: usize) -> bool {
    bits.get(i / 64).is_some_and(|w| (w >> (i % 64)) & 1 == 1)
}

/// The candle code of ADDENDUM 4 (2): direction x body bucket x larger-wick side, 24
/// codes, on the LONG meaning; a short's code is the code of the price-mirrored candle.
/// `None` for a zero-range candle.
pub(crate) fn candle_code(o: i128, h: i128, l: i128, c: i128) -> Option<u8> {
    let r = h - l;
    if r <= 0 {
        return None;
    }
    let up = c > o || (c == o && 2 * c >= h + l);
    let body = (c - o).abs();
    let bucket = body_bucket(body, r);
    let uw = h - c.max(o);
    let lw = c.min(o) - l;
    let wick: u8 = if uw > lw {
        0
    } else if uw < lw {
        1
    } else {
        2
    };
    Some(u8::from(!up) * 12 + bucket * 3 + wick)
}

/// Body (or wick) as a share of the range: 0 = under 20%, 1 = 20-50%, 2 = 50-80%,
/// 3 = at least 80% (exact cross-multiplication; r > 0).
pub(crate) fn body_bucket(x: i128, r: i128) -> u8 {
    if 100 * x < 20 * r {
        0
    } else if 100 * x < 50 * r {
        1
    } else if 100 * x < 80 * r {
        2
    } else {
        3
    }
}

/// The words of a candle code.
pub(crate) fn code_words(code: u8) -> String {
    let dir = if code >= 12 { "down" } else { "up" };
    let b = (code % 12) / 3;
    let w = code % 3;
    let body = ["body<20%", "body 20-50%", "body 50-80%", "body>=80%"].get(usize::from(b)).copied().unwrap_or("?");
    let wick = ["upper wick larger", "lower wick larger", "wicks equal"].get(usize::from(w)).copied().unwrap_or("?");
    format!("{dir} {body} {wick}")
}

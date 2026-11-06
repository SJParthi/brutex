//! Shared constants, time helpers, money formatting and the candidate record.
//!
//! Every instant is UTC microseconds (the store's own stamp); every price and money
//! figure is integer paisa; every percentage is integer basis points. No float.

use pull::session::Day;

/// Microseconds in a second, a minute and a day.
pub(crate) const US_S: i64 = 1_000_000;
pub(crate) const US_MIN: i64 = 60 * US_S;
pub(crate) const US_DAY: i64 = 86_400 * US_S;
/// IST is UTC + 05:30.
pub(crate) const IST: i64 = 19_800 * US_S;
/// 09:15 IST, the session open, as minutes of the IST day.
pub(crate) const OPEN_MIN: i64 = 555;
/// 15:30 IST, the continuous close of a regular session.
pub(crate) const CLOSE_FULL: i64 = 930;
/// 15:15 IST, the continuous close of a closing-auction-eligible stock-day (from 2026-08-03).
pub(crate) const CLOSE_CAS: i64 = 915;
/// 15:08:00 IST: no entry order from here; no stop move or flip decided from here.
pub(crate) const CUT_MIN: i64 = 908;
/// 15:10:00 IST: the broker's square-off.
pub(crate) const SQ_MIN: i64 = 910;
/// Rs 20,00,000.
pub(crate) const ACCOUNT_PAISA: i64 = 200_000_000;
/// Rs 1,00,000 margin x 5 = Rs 5,00,000 exposure per trade.
pub(crate) const EXPOSURE_PAISA: i64 = 50_000_000;
/// Positions open at once.
pub(crate) const SLOTS: usize = 20;
/// The six timeframes, in minutes.
pub(crate) const TFS: [i64; 6] = [1, 3, 5, 15, 30, 60];
pub(crate) const N_TF: usize = 6;
pub(crate) const N_RS: usize = 3;
pub(crate) const RS_NAMES: [&str; 3] = ["VIDEO-ORIGINAL", "AUTHOR-V2", "VIDEO-SCREEN"];
pub(crate) const SIDE_NAMES: [&str; 2] = ["LONG", "SHORT"];
pub(crate) const ACCOUNT_NAMES: [&str; 3] = ["LONG", "SHORT", "BOTH"];
pub(crate) const READING_NAMES: [&str; 6] = ["ST-ATR10", "BB20", "RSI14", "WR14", "CCI20", "EMA200"];
/// The clean candles each reading needs after a hole before it may decide again.
pub(crate) const READING_PERIODS: [u32; 6] = [10, 20, 14, 14, 20, 200];
pub(crate) const EXIT_NAMES: [&str; 5] = ["stop", "flip", "time", "square-off", "unclosable"];
/// Month index base: 2018-01 is index 0.
pub(crate) const BASE_YEAR: u16 = 2018;
/// 2018-09, the window's first month, as a month index.
pub(crate) const WIN_FIRST_MI: usize = 8;
/// 2026-10, the window's last month, as a month index.
pub(crate) const WIN_LAST_MI: usize = 105;
/// 2018-09 ..= 2026-10.
pub(crate) const N_MONTHS: usize = 98;
/// The line every output starts with (the spec's line 1).
pub(crate) const PREVIEW_LINE: &str = "PREVIEW: not the PSRB result; one fixed recipe outside the main build; approximate as listed below; nothing here feeds any other run";

/// UTC microseconds of IST midnight starting IST day `d` (days since 1970-01-01).
pub(crate) fn day_start(d: i64) -> i64 {
    d * US_DAY - IST
}

/// UTC microseconds of IST minute `minute` of IST day `d`.
pub(crate) fn at_min(d: i64, minute: i64) -> i64 {
    day_start(d) + minute * US_MIN
}

/// The IST day number of an instant.
pub(crate) fn ist_day_of(ts: i64) -> i64 {
    (ts + IST).div_euclid(US_DAY)
}

/// The IST minute of the day of an instant.
pub(crate) fn ist_min_of(ts: i64) -> i64 {
    (ts + IST).rem_euclid(US_DAY).div_euclid(US_MIN)
}

/// The civil date of IST day `d`.
pub(crate) fn civil(d: i64) -> Result<(u16, u8, u8), String> {
    let days = u32::try_from(d).map_err(|_| format!("day {d} is before 1970"))?;
    let day = Day::from_days(days).map_err(|e| format!("day {d}: {e}"))?;
    Ok((day.year(), day.month(), day.day()))
}

/// `pull::session::Day` of IST day `d`.
pub(crate) fn session_day(d: i64) -> Result<Day, String> {
    let days = u32::try_from(d).map_err(|_| format!("day {d} is before 1970"))?;
    Day::from_days(days).map_err(|e| format!("day {d}: {e}"))
}

/// The IST day number of a civil date.
pub(crate) fn day_number(y: u16, m: u8, dd: u8) -> Result<i64, String> {
    let day = Day::new(y, m, dd).map_err(|e| format!("{y}-{m:02}-{dd:02}: {e}"))?;
    Ok(i64::from(day.days_from_epoch()))
}

/// The (year, month) of month index `mi` (2018-01 = 0).
pub(crate) fn month_of_index(mi: usize) -> Result<(u16, u8), String> {
    let y = u16::try_from(mi / 12).map_err(|_| format!("month index {mi}"))?;
    let m = u8::try_from(mi % 12 + 1).map_err(|_| format!("month index {mi}"))?;
    Ok((y + BASE_YEAR, m))
}

/// The month index of (year, month), `None` before 2018-01.
pub(crate) fn month_index(y: u16, m: u8) -> Option<usize> {
    if y < BASE_YEAR || m == 0 || m > 12 {
        return None;
    }
    Some(usize::from(y - BASE_YEAR) * 12 + usize::from(m - 1))
}

/// The window index (2018-09 = 0 ..= 2026-10 = 97) of the IST month holding `ts`.
pub(crate) fn window_index(ts: i64) -> Option<usize> {
    let (y, m, _) = civil(ist_day_of(ts)).ok()?;
    let mi = month_index(y, m)?;
    if (WIN_FIRST_MI..=WIN_LAST_MI).contains(&mi) {
        Some(mi - WIN_FIRST_MI)
    } else {
        None
    }
}

/// "YYYY-MM" of window index `wi`.
pub(crate) fn window_label(wi: usize) -> String {
    month_of_index(wi + WIN_FIRST_MI).map_or_else(|e| e, |(y, m)| format!("{y}-{m:02}"))
}

/// "YYYY-MM-DD HH:MM:SS" IST of an instant.
pub(crate) fn fmt_ist(ts: i64) -> String {
    let d = ist_day_of(ts);
    let within = (ts + IST).rem_euclid(US_DAY);
    let secs = within.div_euclid(US_S);
    let (h, mi, s) = (secs / 3600, (secs / 60) % 60, secs % 60);
    match civil(d) {
        Ok((y, m, dd)) => format!("{y}-{m:02}-{dd:02} {h:02}:{mi:02}:{s:02}"),
        Err(e) => e,
    }
}

/// "YYYY-MM-DD" IST of an instant.
pub(crate) fn fmt_date(ts: i64) -> String {
    match civil(ist_day_of(ts)) {
        Ok((y, m, dd)) => format!("{y}-{m:02}-{dd:02}"),
        Err(e) => e,
    }
}

/// Indian digit grouping of a whole number: 12,34,56,789.
fn group(n: u128) -> String {
    let s = n.to_string();
    let len = s.len();
    if len <= 3 {
        return s;
    }
    let (head, tail) = s.split_at(len - 3);
    let hl = head.len();
    let mut out = String::with_capacity(len + len / 2);
    for (i, ch) in head.chars().enumerate() {
        if i > 0 && (hl - i) % 2 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.push(',');
    out.push_str(tail);
    out
}

/// Paisa as rupees with two decimals by integer division: "Rs -1,23,456.78".
pub(crate) fn rupees(p: i128) -> String {
    let sign = if p < 0 { "-" } else { "" };
    let a = p.unsigned_abs();
    format!("Rs {sign}{}.{:02}", group(a / 100), a % 100)
}

/// Basis points as a percentage with two decimals: "-12.34%".
pub(crate) fn pct(bp: i128) -> String {
    let sign = if bp < 0 { "-" } else { "" };
    let a = bp.unsigned_abs();
    format!("{sign}{}.{:02}%", a / 100, a % 100)
}

/// Permille as a percentage with one decimal: "53.4%".
pub(crate) fn permille(pm: i128) -> String {
    let sign = if pm < 0 { "-" } else { "" };
    let a = pm.unsigned_abs();
    format!("{sign}{}.{}%", a / 10, a % 10)
}

/// Thousandths as a ratio with three decimals: "1.234".
pub(crate) fn thousandths(t: i128) -> String {
    let sign = if t < 0 { "-" } else { "" };
    let a = t.unsigned_abs();
    format!("{sign}{}.{:03}", a / 1000, a % 1000)
}

/// What became of a signal candle's would-be trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum Status {
    /// Entered and exited.
    Filled,
    /// No traded second (or minute) left that day.
    #[default]
    EntryNotFilled,
    /// Rs 5,00,000 buys less than one share.
    ZeroQty,
    /// The Zerodha level could not be mapped to GDFL.
    NoRatio,
}

/// How a trade ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum ExitKind {
    /// The trailed resting stop.
    #[default]
    Stop,
    /// A close across the trade's own Supertrend side band.
    Flip,
    /// The 15:07:59 exit order.
    Time,
    /// The broker's square-off from 15:10:00.
    SquareOff,
    /// Nothing traded from 15:10:00 to the session end.
    Unclosable,
}

impl ExitKind {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Stop => 0,
            Self::Flip => 1,
            Self::Time => 2,
            Self::SquareOff => 3,
            Self::Unclosable => 4,
        }
    }
}

/// One signal candle's would-be trade, priced. Its path depends only on its own
/// stock's data; the portfolio decides later whether it is taken.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cand {
    /// The signal candle's close instant T.
    pub(crate) t_signal: i64,
    /// The stock's rank in the 218 spellings' byte order.
    pub(crate) stock: u16,
    pub(crate) rs: u8,
    pub(crate) tf: u8,
    /// 0 long, 1 short.
    pub(crate) side: u8,
    pub(crate) status: Status,
    pub(crate) exit_kind: ExitKind,
    /// Filled from Zerodha minutes (3.5's fallback).
    pub(crate) fallback: bool,
    /// Entry filled more than 60 s after T.
    pub(crate) late: bool,
    /// The entry fill's second (fallback: minute start + 59 s).
    pub(crate) entry_us: i64,
    pub(crate) entry_px: i64,
    pub(crate) qty: i64,
    /// The exit fill's second (fallback: minute start + 59 s).
    pub(crate) exit_us: i64,
    pub(crate) exit_px: i64,
    pub(crate) gross: i64,
    pub(crate) charges: i64,
    pub(crate) net: i64,
    /// The initial resting stop, on the fill basis.
    pub(crate) stop0: i64,
    /// The signal candle's open, high, low, close (Zerodha, adjusted).
    pub(crate) sig: [i64; 4],
}

// ---------------------------------------------------------------------------
// The preview search's own constants (SEARCH-SPEC 2, 4, 5, 7).
// ---------------------------------------------------------------------------

/// Line 1 of every output of the search (SEARCH-SPEC 10).
pub(crate) const SEARCH_LINE: &str = "PREVIEW SEARCH: not the PSRB result; a smaller filter library than the full PSRB search (PARTS-SEARCH.md), outside the main build; approximate as listed below; nothing here feeds any other run";
/// The six exit lanes (SEARCH-SPEC 4.1).
pub(crate) const N_LANE: usize = 6;
pub(crate) const LANE_NAMES: [&str; 6] = ["X1", "X2", "X3", "X4", "X5", "X6"];
pub(crate) const LANE_WORDS: [&str; 6] = [
    "Supertrend (10, 3) on the signal timeframe",
    "Supertrend (10, 2) on the signal timeframe",
    "Supertrend (7, 1.5) on the signal timeframe",
    "Supertrend (10, 3) on the faster timeframe",
    "Supertrend (10, 2) on the faster timeframe",
    "Supertrend (7, 1.5) on the faster timeframe",
];
/// The Supertrend each lane reads: 0 = (10, 2), 1 = (10, 3), 2 = (7, 1.5).
pub(crate) const LANE_ST: [usize; 6] = [1, 0, 2, 1, 0, 2];
/// Lanes X4-X6 read the faster timeframe.
pub(crate) const LANE_FAST: [bool; 6] = [false, false, false, true, true, true];
/// The clean candles each Supertrend needs after a hole (its ATR period).
pub(crate) const ST_PERIOD: [u32; 3] = [10, 10, 7];
/// The faster timeframe (index into TFS) of each timeframe: 3 -> 1, 5 -> 1, 15 -> 5, 30 -> 5, 60 -> 15.
pub(crate) const FAST_OF: [Option<usize>; 6] = [None, Some(0), Some(0), Some(2), Some(2), Some(3)];
/// The five cores (SEARCH-SPEC 2.2).
pub(crate) const N_CORE: usize = 5;
pub(crate) const CORE_NAMES: [&str; 5] = ["K1 ST+BB", "K2 ST+R1X", "K3 VIDEO-ORIGINAL", "K4 AUTHOR-V2", "K5 VIDEO-SCREEN"];
pub(crate) const CORE_WORDS: [&str; 5] = [
    "the exit lane's Supertrend on the trade's side and a close beyond Bollinger 20 at 2.0 sigma",
    "the exit lane's Supertrend on the trade's side and a fresh close beyond the previous day's R1 (S1 for a short)",
    "VIDEO-ORIGINAL: R1/S1 + the lane's Supertrend side + Bollinger 20 at 2.0 sigma + RSI 14 at 70/30",
    "AUTHOR-V2: CPR within 20 bp + R1/S1 + the lane's Supertrend side + Bollinger 20 at 3.0 sigma + RSI 70/30 + Williams %R 14 (long > -85, short < -15) + CCI 20 at +/-200 + EMA 200 side",
    "VIDEO-SCREEN: AUTHOR-V2 with Williams %R long > -15, short < -85",
];
/// The six accounts (SEARCH-SPEC 5.2).
pub(crate) const N_SIDE: usize = 6;
pub(crate) const SIDE6: [&str; 6] = ["LONG", "SHORT", "BOTH", "FADE-LONG", "FADE-SHORT", "FADE-BOTH"];
/// Rs 2,50,000: half exposure (SIZE AFTER A FALL).
pub(crate) const HALF_EXPOSURE_PAISA: i64 = 25_000_000;
/// The window index of 2020-01, the first unseen month.
pub(crate) const UNSEEN_FIRST: usize = 16;
/// The 82 unseen months 2020-01 ..= 2026-10.
pub(crate) const N_UNSEEN: usize = 82;

/// The lanes a timeframe has: three, or six with a faster timeframe.
pub(crate) const fn lanes_of(ti: usize) -> usize {
    if ti == 0 { 3 } else { 6 }
}

/// splitmix64 (the engine's constants).
pub(crate) const fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A draw keyed by up to four integers.
pub(crate) const fn draw(a: u64, b: u64, c: u64, d: u64) -> u64 {
    splitmix(splitmix(splitmix(splitmix(a) ^ b) ^ c) ^ d)
}

/// Rupees of paisa as an i128 helper for i64 values.
pub(crate) fn rs64(p: i64) -> String {
    rupees(i128::from(p))
}

/// Floored basis points of `num / den` (den > 0), sign-normalised (check1 note 3).
pub(crate) fn bp_of(num: i128, den: i128) -> Option<i128> {
    if den <= 0 {
        return None;
    }
    Some((10_000 * num).div_euclid(den))
}

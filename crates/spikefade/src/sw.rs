//! v3.1 SWITCHES (MAIN-RULINGS-ENGINE-v22 items 1-8 and addenda 1-3; the operator's decisions (a)-(d) of 15 Nov 08:5x). Each switch's
//! default is engine v3's reading, so with every switch at its default v3.1 replays v3 byte for byte (proof P0). `run3 --v31` turns every
//! v2.2 delta and decision on at once (the headline engine); each can also be set alone (the comparisons and the named tests).
//!
//! The switches are process-wide in the binary (set once from the command line before any work; one relaxed atomic load in the hot loops).
//! Under `cfg(test)` each test thread has its own copy, so a named test that turns a switch on never changes another test's reading.

use std::sync::atomic::{AtomicI64, Ordering};

/// A switch (an index into the switch table).
pub type Sw = usize;
/// Item 1: size every event with the EXACT NSE margin (SPAN + premium term + ELM, the per-contract-day table) instead of the v2.1 proxy.
pub const MARGIN_EXACT: Sw = 0;
/// Item 2: the fill entitlement carries across blocks (crate v2.3 K1).
pub const CARRY: Sw = 1;
/// Item 3: orders end at 15:30:00 (no fill after 15:30; never-closed units known at 15:30).
pub const END_1530: Sw = 2;
/// Item 3: never-closed units valued at the highest HIGH between the EXIT DECISION and 15:30 (v3: from the entry E0 to the session end).
pub const VALUE_FROM_EXIT: Sw = 3;
/// Item 4: the margin check takes the unrealised loss of the open positions off the available money (F-C).
pub const MTM: Sw = 4;
/// Item 5: max drawdown on the equity at each close instant, same-second closes grouped (the S ranking reads it).
pub const DD_GROUPED: Sw = 5;
/// Item 6 (printed beside, walk-forward choice only): one Top Volume list per UNDERLYING and expiry (v3: per segment and expiry date).
pub const LISTS_PER_UND: Sw = 6;
/// check-build-v3 S1: the design 5.2 tie rule (families on, SL kind rank, smaller level, setting file order, list, share, N).
pub const TIE52: Sw = 7;
/// Decisions (a)-(c): the HEADLINE group (SEARCH small stops only, share at most 25%, positive at +2% adverse on both sides) and every
/// pick's jump-stress table and adverse-move measurements.
pub const HEADLINE: Sw = 8;
/// EXTREME-MATRIX-SELLING X05 / X06 (checklist item 5): an exit order is RE-SENT every block until it fills (crate v2.3 K2).
pub const RESEND: Sw = 9;
/// Addendum 9: MARGIN RULE v2 (index exposure base = the near-month future's price in the file; + Rs 20 per order; lots = floor((slot -
/// Rs 20) / margin per lot)); off = rule v1 (margin-truth's, the 782-trade proof's).
pub const MARGIN_V2: Sw = 10;
/// Addendum 7 / checklist item 9: the 15:10 square-off is the strategy's OWN order (Rs 20 brokerage, no broker square-off fee); Dhan's auto
/// square-off fee only for units still open (never closed). Off = v2.1 (the square-off charged as Dhan's square-off with its fee).
pub const OWN_SQUAREOFF: Sw = 11;
// ---- v3.2 (MAIN-RULINGS-ENGINE-v22 addenda 10-16, the v3.1 check's M1-M4 / S1-S8 and the audit's fixes). Each default = engine v3.1's
// reading, so with every v3.2 switch off v3.2 replays v3.1 byte for byte (proof P0). `run3 --v32` = `--v31` + every v3.2 switch.
/// Addendum 10 (THE NET DEFINITION; check M1 / M4): every filling block x every NSE freeze slice is an executed order (Rs 20 + GST each) in
/// the charges AND in the +2% gate, with NSE's DATED freeze quantities (`--freeze`). Off = v3.1 (1 sale order, at most 2 buy orders).
pub const ORDERS_A10: Sw = 12;
/// Addendum 11: MARGIN RULE v3 for STOCK options (the AEL OTH row for every short stock option; crate v2.4 M3). Off = rule v2's stock rate.
pub const MARGIN_V3: Sw = 13;
/// Addendum 14 (LIQUIDITY FIRST): top-K by volume 5 / 10 / 20, the entry gate (prints / silence / recent volume covering the WHOLE order),
/// the exit-capacity check, refuse never half-enter, the per-trade times-to-fill, unfilled exits 0 for the headline. Off = v3.1.
pub const LIQ14: Sw = 14;
/// Checklist item 10: NSE F&O ban days (dated ban lists, `--ban-dir`): a stock option of a banned underlying is refused that day.
pub const BAN: Sw = 15;
/// Check M2 / S3: the X view and the 10% share twin of every A / X / S pick are detailed trade by trade (wanted picks).
pub const PICKS32: Sw = 16;
/// Addenda 12 / 12b / 13 / 13b: every trade's entry why, exit why, the three gains (previous official close from the NSE F&O bhavcopy,
/// today's open, the signal window), its liquidity and order counts, in the trade rows.
pub const FIELDS32: Sw = 17;
/// Addendum 16: the index expiry captured: the which-expiry and days-to-expiry dials, every trade's expiry facts and the result split by
/// days-to-expiry bucket and weekly / monthly, the dated expiry calendar.
pub const EXPIRY16: Sw = 18;
/// Addendum 15: the month (>= Rs 2,00,000) and week verdicts per account, the verdict view, the closest misses with rupee gaps, the s26
/// portfolio layer under the Rs 20 L cap, the exact combo count, the measured p99 per account step.
pub const SEARCH15: Sw = 19;
/// Addenda 8 / 8b: IV and the five Greeks at the entry and at the exit of every trade through the app's OWN crates/greeks code.
pub const GREEKS: Sw = 20;
// ---- v3.2 REPAIR A (check-engine-1 M1-M3 / S1-S9 and check-engine-2 M1-M4 / S1-S10 of 15 Nov). Each is a v3.2 switch (default off: P0
// against v3.1 holds); `--v32` turns each on unless `--repair-a off` (then `--v32` replays the frozen h6 binary: proof P0-h6).
/// check-engine-2 M1 / S7 / S10: the s26 portfolio refuses a sale when margin in use + the running MTM loss of its open positions would leave
/// less free money than the sale's margin (the kernel's rule), counts `margin_mtm_short`; every pick and portfolio prints its per-minute
/// (margin + MTM loss) peak and the minutes above Rs 20 L; portfolio rows labelled "members walked alone; chosen on the same days".
pub const PORT_MTM: Sw = 21;
/// check-engine-2 M2: the which-expiry dial takes its target expiry from the PREVIOUS session's F&O bhavcopy list; a target the cube did not
/// stage is REFUSED and counted (`expiry_not_staged`, per day and underlying, with its label), never replaced by the nearest.
pub const EXP_NOSUB: Sw = 22;
/// check-engine-2 M3: days to expiry and the theta target's minutes read ONE dated special-session calendar (`--special-sessions`), never
/// the days a run happens to load.
pub const SESS_DATED: Sw = 23;
/// check-engine-2 M4 / S2: Table W chooses each row's pick by the addendum 15 / 19 order on its learning months (months >= Rs 2 L, positive
/// weeks, positive days, then the smallest max DD, then net) and says PASSED only when every test month >= Rs 2 L, every test week and every
/// test day are positive, walked from the row's own start.
pub const WF19: Sw = 24;
/// check-engine-1 M1: charges at the DATED rate set only (the dearer-of reading printed beside); a refused dated set refuses the trade.
pub const CHG_DATED: Sw = 25;
/// check-engine-1 M2: the HEADLINE group admits no account with the liquidity gate OFF (addendum 14 (b) is a hard gate); every headline
/// prints its P / G / F.
pub const HL_STRICT: Sw = 26;
/// check-engine-1 M3 / S3 / S4 / S8: EXIT_WHY with the rule, the level, the trigger second and HOW the level was set in rupees on every row
/// (HELD rows too); the flags column words; percentages rounded half up; named time-to-fill headers.
pub const WHY12: Sw = 27;
/// check-engine-2 S1: weeks counted against the run's weeks (a week with no trade is net 0, not positive); week gaps in the closest misses.
pub const WK_RUN: Sw = 28;
/// check-engine-2 S3: a per-segment slot-count grid inside 5..100 (`--ngrid-idx`, `--ngrid-stk`; default index 5, 10, 15, 20 and stock 5,
/// 10, 20, 40); per segment and N the accounts that booked a trade are printed.
pub const NGRID: Sw = 29;
/// check-engine-2 S9 (addendum 19): positive days / weeks / months against the run's, the worst day / week / month WITH dates, and the
/// addendum 19 order (months >= Rs 2 L, weeks, days, max DD, net) in the verdict view and the closest misses.
pub const A19: Sw = 30;
/// SPEED-UP 4 wired into stage B (exact setting dedup with a full compare; results byte-identical): NOT part of `--v32` (`--dedup on`).
pub const DEDUP: Sw = 31;
/// check-engine-2 S5: the O(1) evidence per account-event step from SAMPLED step blocks (wall clock, step-weighted p50 / p99, each work
/// item's set-up excluded), printed with its true name.
pub const STEPT: Sw = 32;
// ---- v3.2 round B (MAIN-RULINGS-ENGINE-v22 addenda 17 + 18; ported from v3.2-B, job v32-par, onto repair A). Off by default and NOT part
// of `--v32` (so every repair A run replays byte for byte: proof P0-RA); `--v32b` = `--v32` + SPLITS18 + SIDE17, or each alone by its flag.
/// Addendum 18: the SPLIT PERMUTATIONS from the one walk (train/test cuts 2019-2024, walk-forward anchored + rolling, 6 / 12 / 24-month
/// learning, 1 / 3 / 6-month steps; each split chooses on its learning months only; per kept combo the pass share, worst and median split).
pub const SPLITS18: Sw = 33;
/// Addendum 17: the SIDE-BY-SIDE account (index AND stock options under ONE Rs 20 L account, the running MTM loss included; utilization).
pub const SIDE17: Sw = 34;
/// Addendum 18, way A (re-chosen at every fold, a LABELLED COMPARISON): needs every month of every account (whole mode only); never part
/// of `--v32b`.
pub const WAYA18: Sw = 35;
const N: usize = 36;
/// The end (exclusive) of the repair-A switches.
const END_RA: usize = 33;
/// The number of switches (tests reset every one).
pub const COUNT: usize = N;
/// The first repair-A switch.
const FIRST_RA: usize = 21;
/// The first v3.2 switch.
const FIRST32: usize = 12;
/// The headline gate's stress (bp per side; addendum 3: +2%).
pub static GATE_BP: AtomicI64 = AtomicI64::new(200);

#[cfg(not(test))]
static FLAGS: [std::sync::atomic::AtomicBool; N] = [const { std::sync::atomic::AtomicBool::new(false) }; N];
#[cfg(test)]
thread_local! {
    static FLAGS: [std::cell::Cell<bool>; N] = const { [const { std::cell::Cell::new(false) }; N] };
}

/// One switch's value.
#[inline]
#[must_use]
pub fn on(s: Sw) -> bool {
    #[cfg(not(test))]
    {
        FLAGS.get(s).is_some_and(|f| f.load(Ordering::Relaxed))
    }
    #[cfg(test)]
    {
        FLAGS.with(|f| f.get(s).is_some_and(std::cell::Cell::get))
    }
}

/// Sets one switch.
pub fn set(s: Sw, v: bool) {
    #[cfg(not(test))]
    {
        if let Some(f) = FLAGS.get(s) {
            f.store(v, Ordering::Relaxed);
        }
    }
    #[cfg(test)]
    {
        FLAGS.with(|f| {
            if let Some(c) = f.get(s) {
                c.set(v);
            }
        });
    }
}

/// The switches as printed in every RUN log and RESULTS header.
#[must_use]
pub fn words() -> String {
    let w = |s: Sw, a: &str, b: &str| if on(s) { a.to_owned() } else { b.to_owned() };
    let g = GATE_BP.load(Ordering::Relaxed);
    [
        w(MARGIN_EXACT, "margin EXACT (NSE SPAN + premium term + ELM, per contract-day table)", "margin PROXY (v2.1: C_VAR1 stock / assumed index formula)"),
        w(CARRY, "fill entitlement CARRIED across blocks (crate v2.3 K1)", "fill per-block whole-lot floor (v2.1)"),
        w(END_1530, "orders end 15:30", "square-off walks to 15:40 (v2.1)"),
        w(VALUE_FROM_EXIT, "never-closed units valued from the exit decision to 15:30", "never-closed units valued from the entry to the session end (v2.1)"),
        w(MTM, "margin check includes the running MTM loss", "margin check without the MTM loss (v2.1)"),
        w(DD_GROUPED, "max DD with same-second closes grouped", "max DD in booking order (v2.1)"),
        w(LISTS_PER_UND, "lists one per underlying and expiry (walk-forward-choice variant)", "lists one per segment and expiry date"),
        w(TIE52, "ties by design 5.2", "ties by (list, setting, share, N) (v3)"),
        w(HEADLINE, &format!("HEADLINE gate: SEARCH small stops, share <= 25%, positive at +{}.{:02}% adverse both sides", g / 100, g % 100), "no headline gate (v3)"),
        w(RESEND, "exit orders RE-SENT every block until filled (crate v2.3 K2)", "one exit order at the trigger's protection limit (v2.1)"),
        w(MARGIN_V2, "margin rule v2 (index base = near-month future in the file; + Rs 20 per order)", "margin rule v1 (index base = live spot; no order add-on)"),
        w(OWN_SQUAREOFF, "the 15:10 square-off is our own order (Rs 20, no broker fee; Dhan's fee only for never-closed units)", "the 15:10 square-off charged as Dhan's square-off with its fee (v2.1)"),
    ]
    .into_iter()
    // v3.2-B: the two words only when one of its switches is on (every --v32 run prints the v3.2 words byte for byte)
    .chain(if on(SPLITS18) || on(SIDE17) { vec![w(SPLITS18, "split permutations (addendum 18: TT 2019-2024, walk-forward anchored + rolling 6/12/24 m x 1/3/6 m, learning months only, per-combo pass share / worst / median)", "no split permutations"), w(SIDE17, "side-by-side account (addendum 17: index AND stock options under one Rs 20 L incl. the running MTM loss)", "no side-by-side account")] } else { Vec::new() })
    .chain(if on(WAYA18) { vec!["way A of the splits computed (re-chosen at every fold: LABELLED COMPARISON; month series, whole mode)".to_owned()] } else { Vec::new() })
    .collect::<Vec<_>>()
    .join("; ")
}

/// v3.2: the v3.2 switches as printed in every v3.2 RUN log and RESULTS header (empty when none is on: the v3.1 words stay byte for byte).
#[must_use]
pub fn words32() -> String {
    if !any32() {
        return String::new();
    }
    let w = |s: Sw, a: &str, b: &str| if on(s) { a.to_owned() } else { b.to_owned() };
    [
        w(ORDERS_A10, "EVERY filling block x every NSE freeze slice = one executed order (Rs 20 + GST) in the charges and the +2% gate (addendum 10)", "v3.1 order count (1 sale order, at most 2 buy orders)"),
        w(MARGIN_V3, "stock margin RULE v3 (AEL OTH row for every short stock option; 46/46 Dhan screens)", "stock margin rule v2 (OTM row when out of the money)"),
        w(LIQ14, "LIQUIDITY FIRST (addendum 14: top-K by volume 5/10/20, entry gate + exit capacity cover the whole order, unfilled exits 0 for the headline)", "liquidity as v3.1"),
        w(BAN, "F&O ban days refused (dated NSE ban lists)", "F&O ban days not applied"),
        w(PICKS32, "X view and every pick's 10% twin detailed", "picks as v3.1"),
        w(FIELDS32, "per-trade entry why / exit why / three gains / liquidity / orders", "trade rows as v3.1"),
        w(EXPIRY16, "index expiry captured (which-expiry and days-to-expiry dials, expiry facts, split by bucket)", "expiry as v3.1"),
        w(SEARCH15, "month >= Rs 2,00,000 and week verdicts, verdict view, closest misses, s26 portfolio layer, combo count", "search as v3.1"),
        w(GREEKS, "Greeks at entry and exit through the app's own crates/greeks", "no Greeks in the engine"),
    ]
    .join("; ")
        + &words_ra()
}

/// v3.2 REPAIR A: the repair switches' words (empty when none is on: the h6 words stay byte for byte, proof P0-h6).
#[must_use]
pub fn words_ra() -> String {
    if !(FIRST_RA..END_RA).any(on) {
        return String::new();
    }
    let w = |s: Sw, a: &str, b: &str| if on(s) { a.to_owned() } else { b.to_owned() };
    let mut v = vec![
        w(PORT_MTM, "portfolio cap with the running MTM loss + per-minute (margin + MTM) peak (check-2 M1/S7)", "portfolio cap without MTM (h6)"),
        w(EXP_NOSUB, "which-expiry target from the previous bhavcopy, not staged = refused and counted (check-2 M2)", "which-expiry dial as h6"),
        w(SESS_DATED, "dated special-session calendar for days to expiry and theta (check-2 M3)", "session calendar from the loaded days (h6)"),
        w(WF19, "Table W: choice and PASSED by addenda 15/19 on months, weeks, days (check-2 M4/S2)", "Table W as v3.1 (h6)"),
        w(CHG_DATED, "charges at the DATED rates only (dearer-of beside) (check-1 M1)", "charges at the dearer of dated and 2026-10-30 (h6)"),
        w(HL_STRICT, "headline excludes the liquidity gate OFF (check-1 M2)", "headline admits liquidity OFF (h6)"),
        w(WHY12, "exit why with rule, level, trigger and how the level was set; rounded percentages (check-1 M3/S3/S4/S8)", "exit why as h6"),
        w(WK_RUN, "weeks counted against the run's weeks (check-2 S1)", "weeks counted when seen (h6)"),
        w(NGRID, &format!("per-segment N grid (index {}, stock {}) (check-2 S3)", ngrid_words(0), ngrid_words(1)), "N grid 5/10/20/40 (h6)"),
        w(A19, "addendum 19 complete: days/weeks/months with worst dates, order in the verdict view (check-2 S9)", "verdict view as h6"),
        w(STEPT, "step timing from sampled step blocks (wall clock, step-weighted) (check-2 S5)", "step timing per work item (h6)"),
    ];
    if on(DEDUP) {
        v.push("SPEED-UP 4 exact setting dedup in stage B (results byte-identical)".to_owned());
    }
    format!("; REPAIR A: {}", v.join("; "))
}

/// v3.2 REPAIR A (check-engine-2 S3): the slot-count grid per segment (index 0, stock 1), 4 values each inside 5..100.
pub static NGRID_SEG: [[std::sync::atomic::AtomicI64; 4]; 2] = [
    [AtomicI64::new(5), AtomicI64::new(10), AtomicI64::new(15), AtomicI64::new(20)],
    [AtomicI64::new(5), AtomicI64::new(10), AtomicI64::new(20), AtomicI64::new(40)],
];

#[cfg(test)]
thread_local! {
    /// REPAIR A (named tests): a test thread's own slot-count grid (the process-wide grid is never changed by a test).
    pub static NGRID_T: std::cell::Cell<Option<[[i64; 4]; 2]>> = const { std::cell::Cell::new(None) };
}

/// The slot count N of slot index `ni` for segment `seg` (v3.1's 5 / 10 / 20 / 40 unless the NGRID switch is on).
#[inline]
#[must_use]
pub fn n_slots(seg: u8, ni: usize) -> i64 {
    const V31: [i64; 4] = [5, 10, 20, 40];
    if on(NGRID) {
        // MUTANT m_v32r_ngrid_v31: the per-segment grid read but v3.1's grid used
        if cfg!(feature = "m_v32r_ngrid_v31") {
            return V31.get(ni).copied().unwrap_or(5);
        }
        #[cfg(test)]
        if let Some(g) = NGRID_T.with(std::cell::Cell::get) {
            return g.get(usize::from(seg.min(1))).and_then(|r| r.get(ni)).copied().unwrap_or(5);
        }
        NGRID_SEG.get(usize::from(seg.min(1))).and_then(|g| g.get(ni)).map_or(5, |x| x.load(Ordering::Relaxed))
    } else {
        V31.get(ni).copied().unwrap_or(5)
    }
}

/// The grid of a segment in words ("5/10/15/20").
#[must_use]
pub fn ngrid_words(seg: u8) -> String {
    NGRID_SEG.get(usize::from(seg.min(1))).map_or_else(String::new, |g| g.iter().map(|x| x.load(Ordering::Relaxed).to_string()).collect::<Vec<_>>().join("/"))
}

/// Whether any v3.2 switch is on (a v3.2 run).
#[must_use]
pub fn any32() -> bool {
    (FIRST32..N).any(on)
}

/// Whether any v3.1 switch is on (a v3.1 run, not a P0 replay).
#[must_use]
pub fn any() -> bool {
    (0..N).any(on)
}

/// Reads the switches from the command line (`--v31` = every v2.2 delta and decision; then each flag may override).
///
/// # Errors
/// A switch value that is not one of its words.
pub fn from_args(a: &[String]) -> Result<(), String> {
    let all32b = a.iter().any(|x| x == "--v32b");
    let all32 = all32b || a.iter().any(|x| x == "--v32");
    let all = all32 || a.iter().any(|x| x == "--v31");
    let flag = |k: &str, yes: &str, no: &str| -> Result<Option<bool>, String> {
        match a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str) {
            None => Ok(None),
            Some(v) if v == yes => Ok(Some(true)),
            Some(v) if v == no => Ok(Some(false)),
            Some(v) => Err(format!("{k} {yes}|{no}, not {v}")),
        }
    };
    for (s, k, yes, no) in [(MARGIN_EXACT, "--margin", "exact", "proxy"), (CARRY, "--fill-carry", "on", "off"), (END_1530, "--close", "1530", "1540"), (VALUE_FROM_EXIT, "--never-closed", "from-exit", "from-entry"), (MTM, "--mtm", "on", "off"), (DD_GROUPED, "--dd", "grouped", "booking"), (TIE52, "--ties", "design52", "v3"), (HEADLINE, "--headline", "on", "off"), (RESEND, "--resend", "on", "off"), (MARGIN_V2, "--margin-rule", "v2", "v1"), (OWN_SQUAREOFF, "--square-off", "own", "broker")] {
        set(s, flag(k, yes, no)?.unwrap_or(all));
    }
    // v3.2: each v3.2 switch on with --v32 (or alone by its flag); off by default (P0 against v3.1)
    for (s, k, yes, no) in [(ORDERS_A10, "--orders", "block-slice", "v31"), (MARGIN_V3, "--stock-margin", "v3", "v2"), (LIQ14, "--liquidity", "a14", "v31"), (BAN, "--ban", "on", "off"), (PICKS32, "--picks32", "on", "off"), (FIELDS32, "--fields32", "on", "off"), (EXPIRY16, "--expiry16", "on", "off"), (SEARCH15, "--search15", "on", "off"), (GREEKS, "--greeks", "on", "off")] {
        set(s, flag(k, yes, no)?.unwrap_or(all32));
    }
    // v3.2 REPAIR A: each repair switch on with --v32 unless --repair-a off (then --v32 = the frozen h6 binary's reading, proof P0-h6)
    let ra = all32 && !matches!(a.iter().position(|x| x == "--repair-a").and_then(|i| a.get(i + 1)).map(String::as_str), Some("off"));
    if let Some(v) = a.iter().position(|x| x == "--repair-a").and_then(|i| a.get(i + 1))
        && v != "on"
        && v != "off"
    {
        return Err(format!("--repair-a on|off, not {v}"));
    }
    for (s, k, yes, no) in [(PORT_MTM, "--portfolio-mtm", "on", "off"), (EXP_NOSUB, "--expiry-dial", "strict", "v32"), (SESS_DATED, "--sess-cal", "dated", "loaded"), (WF19, "--wf-verdict", "a19", "v31"), (CHG_DATED, "--charges", "dated", "dearer"), (HL_STRICT, "--headline-liq", "strict", "any"), (WHY12, "--why12", "on", "off"), (WK_RUN, "--weeks", "run", "seen"), (NGRID, "--ngrid", "seg", "v31"), (A19, "--a19", "on", "off"), (STEPT, "--step-timing", "sampled", "item")] {
        set(s, flag(k, yes, no)?.unwrap_or(ra));
    }
    // SPEED-UP 4 is exact (results byte-identical) and opt-in: never implied by --v32
    set(DEDUP, flag("--dedup", "on", "off")?.unwrap_or(false));
    for (seg, k) in [(0_usize, "--ngrid-idx"), (1, "--ngrid-stk")] {
        if let Some(v) = a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)) {
            let g: Vec<i64> = v.split(',').map(|x| x.trim().parse::<i64>().map_err(|_| format!("{k} {v}"))).collect::<Result<Vec<_>, _>>()?;
            if g.len() != 4 || g.iter().any(|&n| !(5..=100).contains(&n)) || g.windows(2).any(|w| w.first() >= w.get(1)) {
                return Err(format!("{k} {v}: four rising slot counts inside 5..100 (addendum 15 / REQ 66)"));
            }
            for (i, n) in g.into_iter().enumerate() {
                if let Some(x) = NGRID_SEG.get(seg).and_then(|r| r.get(i)) {
                    x.store(n, Ordering::Relaxed);
                }
            }
        }
    }
    for (s, k) in [(WF19, "--wf-verdict a19"), (A19, "--a19 on"), (WK_RUN, "--weeks run"), (PORT_MTM, "--portfolio-mtm on")] {
        if on(s) && !on(SEARCH15) {
            return Err(format!("{k} needs the SEARCH15 switch (--search15 on or --v32): its month / week / session verdicts"));
        }
    }
    if on(EXP_NOSUB) && !on(EXPIRY16) {
        return Err("--expiry-dial strict needs the EXPIRY16 switch (--expiry16 on or --v32)".to_owned());
    }
    if on(SESS_DATED) && !a.iter().any(|x| x == "--special-sessions") {
        return Err("--sess-cal dated (or --v32) needs --special-sessions <SPECIAL-SESSIONS-NSE.tsv> (the dated NSE special-session calendar)".to_owned());
    }
    // v3.2-B: addenda 18 and 17 (never with --v32 alone)
    for (s, k, yes, no) in [(SPLITS18, "--splits18", "on", "off"), (SIDE17, "--side17", "on", "off")] {
        set(s, flag(k, yes, no)?.unwrap_or(all32b));
    }
    // round B: way A of the splits only on request, never in the day-major (history) mode: it keeps every month of every account
    set(WAYA18, flag("--splits18-way-a", "on", "off")?.unwrap_or(false));
    if on(WAYA18) && !on(SPLITS18) {
        return Err("--splits18-way-a on needs the split permutations (--splits18 on or --v32b)".to_owned());
    }
    if on(WAYA18) && a.iter().any(|x| x == "--day-major") {
        return Err("--splits18-way-a on is whole mode only: way A keeps every month of every account (80 bytes a month; a day-major batch holds every account of the batch for the whole run)".to_owned());
    }
    if on(ORDERS_A10) && !a.iter().any(|x| x == "--freeze") {
        return Err("--orders block-slice (or --v32) needs --freeze <FREEZE-DATED.tsv> (NSE's dated freeze quantities)".to_owned());
    }
    if on(BAN) && !a.iter().any(|x| x == "--ban-dir") {
        return Err("--ban on (or --v32) needs --ban-dir <dir of NSE fo_secban_DDMMYYYY.csv> (repeatable)".to_owned());
    }
    if on(GREEKS) && !a.iter().any(|x| x == "--tbill") {
        return Err("--greeks on (or --v32) needs --tbill <rbi-91d-tbill.tsv> (the app's risk-free source)".to_owned());
    }
    if on(MARGIN_V3) && !on(MARGIN_EXACT) {
        return Err("--stock-margin v3 needs the exact margin (--margin exact or --v31)".to_owned());
    }
    // the per-underlying lists are a labelled VARIANT (never part of --v31)
    set(LISTS_PER_UND, flag("--lists", "underlying", "expiry")?.unwrap_or(false));
    if let Some(v) = a.iter().position(|x| x == "--gate-bp").and_then(|i| a.get(i + 1)) {
        GATE_BP.store(v.parse::<i64>().map_err(|_| format!("--gate-bp {v}"))?, Ordering::Relaxed);
    }
    if on(MARGIN_EXACT) && !a.iter().any(|x| x == "--margin-manifest") {
        return Err("--margin exact (or --v31) needs --margin-manifest <SNAPSHOTS tsv> (v31/margin/build-snapshots.mjs)".to_owned());
    }
    Ok(())
}

/// Whether any REPAIR A switch is on (else a v3.2 run prints exactly as the frozen h6 binary: proof P0-h6).
#[must_use]
pub fn any_ra() -> bool {
    (FIRST_RA..N).any(on)
}

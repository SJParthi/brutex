//! v3.4 integrate:D (round D, 16 Nov 2026; MAIN-RULINGS-ENGINE-v22 Addendum 30.2: the two BLOCKING items COVERED, not labelled): the
//! switches that wire data:F3's dated stock freeze table (`v34/tables/STOCK-FREEZE-DATED.tsv`, `v34/F3-STOCK-FREEZE.md` section 7) and
//! data:F7's band / protection model (`v34/F7-BANDS-PROTECTION.md` section 5, tables `v34/tables/F7-*.tsv`) into v3.4.
//!
//! Like `fix34` these CHANGE results. Each defaults to OFF, so with every one off the binary replays build:D-high's v3.4 (and, with every
//! v3.4 switch off, v3.3 final 8d9a5b67...) byte for byte (proof P0). `run3 --int34` turns every switch on (each needs its table); each has its
//! own flag. Their words go to RUN.log, to the RESULTS header and to the day-major checkpoint fingerprint (empty when none is on).
//! Process-wide in the binary (set once from the command line before any work); under `cfg(test)` each test thread has its own copy.

use std::sync::atomic::Ordering;

/// An integrate:D switch (an index into the switch table).
pub type Ix = usize;
/// F3 (Addendum 30.2, data:F3 section 7): the dated stock freeze table read EXACTLY: a stock contract-day in the table's range reads ITS
/// session's row (never an earlier one), the row's kind (NSE SOURCE / RULE-BOUND / GAP-BOUND) and alias counted per distinct contract-day; a
/// contract-day with no row in range is REFUSED (`freeze_none_in_range`), never "no limit"; checklist row 12 COVERED only when that count is
/// 0, no stock lookup read the undated file and the pinned sha256 (`--freeze-stock-sha`) holds.
pub const F3D: Ix = 0;
/// F7 (data:F7 5.2 / 5.9): every own buy-back and square-off walked under the dated READINGS of `F7-PROTECTION-READINGS.tsv` (R0 = the v3.4
/// path, R5 / R10 = Dhan's protection at 5% / 10% re-sent every block, RM = NSE's market-order cap LTP + max(20%, Rs 10)), the DEAREST booked
/// (buy-back rupees + every charge of its orders + the never-closed value + Dhan's fee); the sale by the one strict rule R5 (5% limit), a sale
/// R0 fills but R5 refuses is refused (`sale_refused_by_f7`).
pub const PROT: Ix = 1;
/// F7 (5.3): NSE's limit price protection in the strict readings, dated per contract class (index weekly + monthly from 2022-10-31, stock
/// from 2023-02-06, every index option from 2023-03-31): a re-send whose limit is above High LPP (Rs 20 / 40% over the last 30 s average,
/// bounded from the tape's own prints) is rejected (0 capacity that block); a sale below Low LPP is refused at T.
pub const LPP: Ix = 2;
/// F7 (5.6): in the strict readings no own order or re-send is sent at or after 15:23:00 (Dhan's 15:25 RMS square-off rejects user orders in
/// the last 2 minutes); the units still open are valued per ruling item 3 and Dhan's auto square-off fee is charged.
pub const CUT: Ix = 3;
const N: usize = 4;
/// The number of switches (tests reset every one).
pub const COUNT: usize = N;
/// The switches `--int34` turns on.
pub const INT34_ALL: [Ix; N] = [F3D, PROT, LPP, CUT];

#[cfg(not(test))]
static FLAGS: [std::sync::atomic::AtomicBool; N] = [const { std::sync::atomic::AtomicBool::new(false) }; N];
#[cfg(test)]
thread_local! {
    static FLAGS: [std::cell::Cell<bool>; N] = const { [const { std::cell::Cell::new(false) }; N] };
}

/// One switch's value.
#[inline]
#[must_use]
pub fn on(s: Ix) -> bool {
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
pub fn set(s: Ix, v: bool) {
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

/// Whether any switch is on.
#[must_use]
pub fn any() -> bool {
    (0..N).any(on)
}

fn word(s: Ix) -> String {
    match s {
        F3D => format!("dated stock freeze limits read EXACTLY from {} (exact session, SOURCE / RULE-BOUND / GAP-BOUND / ALIAS counted, a contract-day without its row refused) (standing F3, data:F3)", crate::freeze::stock_src()),
        PROT => format!("protection READINGS R0 / R5 / R10 / RM per {} (the dearest buy-back booked; the sale by R5) (standing F7 = exits F-B1, F29, data:F7)", crate::f7::readings_src()),
        LPP => "NSE limit price protection dated per contract class in the strict readings (re-send above High LPP rejected: 0 capacity; sale below Low LPP refused) (standing F7, data:F7 5.3)".to_owned(),
        CUT => "no own order or re-send at or after 15:23:00 in the strict readings (Dhan RMS 15:25) (standing F7, data:F7 5.6)".to_owned(),
        _ => String::new(),
    }
}

/// The words of every switch on (RUN.log, RESULTS).
#[must_use]
pub fn words() -> String {
    (0..N).filter(|&s| on(s)).map(word).collect::<Vec<_>>().join("; ")
}

/// The fingerprint part (empty when none is on: every earlier fingerprint unchanged).
#[must_use]
pub fn fp_part() -> String {
    let force = if crate::f7::FORCE_BOUND.load(Ordering::Relaxed) { "; f7 FORCE-BOUND (a measurement mode)" } else { "" };
    if !any() {
        return force.to_owned();
    }
    // the tables by content (sha256), so a checkpoint, a resume or a shard under another table is refused (each NSE contract file is
    // checked against the manifest's sha256 when its day loads)
    // sha256 only, never a path (a cloud shard reads the same tables under other paths: the kit's argument digest leaves path values out)
    let sha = |k: &str| crate::f7::SHAS.lock().ok().and_then(|g| g.get(k).cloned()).unwrap_or_else(|| "-".to_owned());
    let tables = format!(
        "; f3 sha256 {}; f7 readings sha256 {}; bands manifest sha256 {}; ca sha256 {}",
        if on(F3D) { crate::freeze::FREEZE.get().and_then(|t| t.stock.as_ref()).map_or_else(|| "-".to_owned(), |s| s.sha.clone()) } else { "-".to_owned() },
        sha("readings"),
        sha("manifest"),
        sha("ca")
    );
    format!("; int34 {}{tables}{force}", (0..N).filter(|&s| on(s)).map(|s| ["F3D", "PROT", "LPP", "CUT"].get(s).copied().unwrap_or("?")).collect::<Vec<_>>().join(","))
}

/// Reads the switches (`--int34` = all; each flag may override) and their tables.
///
/// # Errors
/// A switch value that is not one of its words; a switch on without its table.
pub fn from_args(a: &[String]) -> Result<(), String> {
    let all = a.iter().any(|x| x == "--int34");
    let get = |k: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str);
    for (s, k) in [(F3D, "--freeze-stock-dated"), (PROT, "--prot-readings"), (LPP, "--lpp-dated"), (CUT, "--cut-1523")] {
        let v = match get(k) {
            None => all,
            Some("on") => true,
            Some("off") => false,
            Some(v) => return Err(format!("{k} on|off, not {v}")),
        };
        set(s, v);
    }
    if (on(PROT) || on(LPP) || on(CUT)) && crate::f7::READINGS_T.get().is_none() {
        crate::f7::load_readings(get("--f7-readings").ok_or("--prot-readings / --lpp-dated / --cut-1523 (or --int34) need --f7-readings <data:F7's F7-PROTECTION-READINGS.tsv>")?)?;
    }
    if get("--f7-force-bound") == Some("on") {
        crate::f7::FORCE_BOUND.store(true, Ordering::Relaxed);
    }
    if (on(LPP) || on(CUT)) && !on(PROT) {
        return Err("--lpp-dated / --cut-1523 act in the strict readings: they need --prot-readings on".to_owned());
    }
    Ok(())
}

/// The checks that need the tables read after the switches (`freeze::from_args` runs after [`from_args`]).
///
/// # Errors
/// F3D on without data:F3's table among the `--freeze` sources.
pub fn check() -> Result<(), String> {
    if on(F3D) && !crate::freeze::stock_loaded() {
        return Err("--freeze-stock-dated on (or --int34) needs --freeze <data:F3's STOCK-FREEZE-DATED.tsv> (its 16-column header)".to_owned());
    }
    Ok(())
}

//! Real bars, out of the store, in the shape the sweep takes.
//!
//! # The gap this closes
//!
//! `crates/api` declared `store` and not `runner`; `crates/cli` declared
//! `runner` and not `store`. **No crate in this workspace could do both**, so
//! nothing connected a pulled bar to a ranked result — `cli sweep` has only ever
//! swept `runner::synthetic`, and said so in its own banner. `CLAUDE.md` §5 calls
//! that the live gap. This module is the join.
//!
//! # Why the conversion is a struct literal and not a trait
//!
//! `store::format::Bar` and `indicators::Candle` carry the same seven fields
//! with the same names and the same types — `ts_micros`, `open`, `high`, `low`,
//! `close`, `volume`, `open_interest`. They are two names for one record because
//! the crates that own them may not name each other: gate 22 pins `indicators`
//! to `vocab` alone, so it cannot depend on `store`, and a shared type would
//! have to live in a crate both could see. Copying seven fields here is the
//! price of that boundary, and it is the boundary that keeps a bar from reaching
//! the sweep by accident.
//!
//! # What arrives with the bars, and why it matters
//!
//! The store path IS `bars/<vendor>/<exchange>/<segment>/<symbol>/<rung>/<month>`,
//! so a load knows its **vendor, instrument, timeframe and month** before it
//! reads a byte. Those are four of the eight terms `runner::identity` needs, and
//! three of them were the reason a run identity could not be recorded: with
//! synthetic bars there is no instrument to name and naming one would be an
//! invention `CLAUDE.md` §3 rule 1 forbids. A stored load has them all.

use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::vendor::Vendor;
use indicators::Candle;
use std::path::Path;
use store::file::BarFile;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

/// One instrument-month, and everything the store knew about it.
///
/// The fields are the provenance a report needs. They are returned rather than
/// logged because a caller that renders a figure has to be able to say where the
/// figure came from, and a log line cannot be put in a table.
#[derive(Debug, Clone)]
pub struct Loaded {
    /// The bars, oldest first, exactly as the file holds them.
    pub bars: Vec<Candle>,
    /// Which feed wrote them. The first path segment, never inferred.
    pub vendor: Vendor,
    /// What they are bars of.
    pub key: InstrumentKey,
    /// The rung, as its canonical directory word — `1min`, `1day`.
    pub timeframe: &'static str,
}

/// Why a load could not happen, in the operator's words.
///
/// A `String` and not an error enum, deliberately: every arm here is a sentence
/// a person reads once and acts on, and none of them is matched on. An enum
/// would invite a caller to branch on a distinction that does not exist.
pub type Refusal = String;

/// The rung whose directory word is `name`, or the words that do exist.
///
/// `Timeframe::KNOWN` is the store's own declared list, so a rung added there is
/// selectable here with no edit. The refusal names every legal word rather than
/// saying "unknown": a caller who typed `1hour` needs to be told it is `60min`.
fn rung(name: &str) -> Result<Timeframe, Refusal> {
    Timeframe::KNOWN
        .iter()
        .copied()
        .find(|t| t.as_str() == name)
        .ok_or_else(|| {
            let known: Vec<&str> = Timeframe::KNOWN.iter().map(|t| t.as_str()).collect();
            format!(
                "`{name}` is not a rung this store carries. The rungs are: {}.",
                known.join(", ")
            )
        })
}

/// One instrument-month of real bars, or the reason there are none.
///
/// # Errors
///
/// Every arm is a [`Refusal`] naming what was asked for and what the store said.
/// A missing file is the ordinary case — it means that month was never pulled —
/// and it is reported as that rather than as an I/O error, because the operator's
/// next action is a pull and not a filesystem check.
pub fn load(
    root: &Path,
    vendor: Vendor,
    underlying: &str,
    rung_name: &str,
    year: u16,
    month: u8,
) -> Result<Loaded, Refusal> {
    let timeframe = rung(rung_name)?;
    let key = InstrumentKey::index(Exchange::Nse, underlying)
        .map_err(|why| format!("`{underlying}` is not an index this engine sweeps: {why}"))?;
    let ym = YearMonth::new(year, month)
        .map_err(|why| format!("{year}-{month:02} is not a month: {why}"))?;
    let path = StorePath::for_key(vendor, &key, timeframe, ym, FileKind::Bars)
        .map_err(|why| format!("no store path for that selection: {why}"))?;

    // THE SYMBOL ID IS THE STORE'S OWN, NOT A NEW ONE. `pull::ingest` writes
    // `fnv1a(symbol) as u32` into the header, and `open_existing` compares what
    // it is handed against what it finds. Computing it any other way here would
    // make every file refuse to open with a mismatch that named nothing real.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the id IS the low 32 bits of the FNV-1a hash — `pull::ingest` \
                  writes it that way at its own `as u32`, and the header compares \
                  what it was handed against what it finds. Widening or checking \
                  here would compute a different number and every file would \
                  refuse to open with a mismatch that named nothing real."
    )]
    let symbol_id = brutex_core::universe::fnv1a(underlying) as u32;

    let file = BarFile::open_existing(root, path, symbol_id).map_err(|why| {
        format!(
            "{underlying} {rung_name} {year}-{month:02} is not in the store for \
             {}: {why}. Nothing was read. Pull that instrument-month first.",
            vendor.as_str()
        )
    })?;

    // RESERVED ONCE, FROM THE HEADER'S OWN COUNT. `records()` is a field read,
    // not a walk, so this is one allocation for a known length rather than a
    // doubling per bar.
    let n = file.records();
    let mut bars = Vec::with_capacity(usize::try_from(n).unwrap_or(0));
    for i in 0..n {
        let bar = file
            .read_record(i)
            .map_err(|why| format!("record {i} of {n} could not be read: {why}"))?;
        bars.push(Candle {
            ts_micros: bar.ts_micros,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
            open_interest: bar.open_interest,
        });
    }

    Ok(Loaded {
        bars,
        vendor,
        key,
        timeframe: timeframe.as_str(),
    })
}

/// How long one bar of `rung_name` lasts, in microseconds.
///
/// # Taken from the rung, never inferred from the data
///
/// `runner::align` needs this to know when a signal bar CLOSES, which is the
/// instant its mask becomes knowable. The alternative — subtracting one bar's
/// stamp from the next — looks equivalent and is not: a gap between two stamps
/// is a halt, a holiday or a session boundary, not a longer bar. Deriving the
/// length that way would make the deadline move with the data and would place
/// the first entry of every session hours late.
///
/// # Errors
///
/// Every refusal [`rung`] makes, unchanged, so an unknown rung is named the same
/// way here as everywhere else.
pub fn rung_length_micros(rung_name: &str) -> Result<i64, Refusal> {
    let timeframe = rung(rung_name)?;
    Ok(i64::from(timeframe.secs()).saturating_mul(1_000_000))
}
/// A CONTIGUOUS SPAN of real bars, joined across every month it covers.
///
/// # Why this type exists, and why one month was never the unit
///
/// The store keeps one file per `(vendor, instrument, timeframe, MONTH)`, so
/// seven years of one-minute bars is **eighty-four files**. That is a STORAGE
/// layout — it says nothing about what a sweep should cover — and until this
/// type the loader could open exactly one of them, so the largest question the
/// engine could be asked was "what worked in March".
///
/// That is not a smaller version of the real question, it is a DIFFERENT one. A
/// combination that fires on two percent of bars in every single month is not
/// the same as one that fires on two percent of seven years; a trade cannot open
/// in one month and close in the next; and a walk-forward split inside one month
/// tests against days, not against regimes. Sweeping eighty-four months
/// separately and reading the eighty-four answers is not the seven-year answer.
///
/// # What is NOT hidden
///
/// A month the store does not hold is NAMED in [`Self::missing`] and rendered,
/// never skipped quietly. A span with a hole is a shorter sample, not a
/// corrected one, and `CLAUDE.md` §4 bans the fallback that would let it read
/// like a whole one.
#[derive(Clone, Debug)]
pub struct Span {
    /// Every bar in the span, oldest first, monotonic in time across the join.
    pub bars: Vec<Candle>,
    /// Which feed wrote them.
    pub vendor: Vendor,
    /// What they are bars of.
    pub key: InstrumentKey,
    /// The rung, as its canonical directory word.
    pub timeframe: &'static str,
    /// Months the range covers, whether or not the store holds them.
    pub asked: u32,
    /// Months that were actually opened and read.
    pub found: u32,
    /// Months the range covers that the store does not hold, in order.
    ///
    /// Rendered by the caller. A hole moves every figure computed over the span
    /// and the operator has to see it to know that.
    pub missing: Vec<(u16, u8)>,
}

impl Span {
    /// Whether the store held every month the range asked for.
    #[must_use]
    pub const fn complete(&self) -> bool {
        self.missing.is_empty()
    }
}

/// The month after this one, or `None` past December 9999.
///
/// A free function rather than a method on `YearMonth`, because that type lives
/// in `crates/store` and this crate does not own it.
const fn next_month(year: u16, month: u8) -> Option<(u16, u8)> {
    if month < 12 {
        return Some((year, month + 1));
    }
    if year >= 9999 {
        return None;
    }
    Some((year + 1, 1))
}

/// The longest span this command will assemble, in months.
///
/// # A BOUND, BECAUSE THE ARGUMENTS ARE `u16` AND NOTHING ELSE STOPS THEM
///
/// `YEAR` parses as `u16`, so `audit-range ... 1970 1 65535 1` asks for a range
/// of **763,000 months**. Nothing downstream refuses it: `YearMonth::new` is not
/// reached until a path is built, one per month, and every one of those is a
/// failed file open. The walk would grind for minutes and then produce a refusal
/// saying nothing was found — a hang wearing a result's clothes, which is the
/// §4 fallback in its slowest form.
///
/// A hundred years is far past any span this engine will be asked for — seven
/// years is 84 — and it is a NUMBER, so the refusal can name it rather than
/// saying "too long".
const MAX_SPAN_MONTHS: usize = 1_200;

/// Every month from `from` up to and including `to`, oldest first.
///
/// Bounded twice over: by [`MAX_SPAN_MONTHS`] before the walk starts, and by
/// `next_month` refusing past 9999-12 inside it.
fn months_between(from: (u16, u8), to: (u16, u8)) -> Result<Vec<(u16, u8)>, Refusal> {
    // THE ENDPOINTS ARE MONTHS, AND THAT IS CHECKED HERE RATHER THAN DISCOVERED.
    //
    // `MONTH` parses as `u8`, so 13 through 255 arrive intact. Without this
    // guard they walk straight into the loop, `load` builds no path for them,
    // and `load_span` files them under `missing` -- so `audit-range ... 2019 13
    // 2026 8` would report "2019-13 is missing from the store". It is not
    // missing. It is not a month. Reporting a malformed argument as absent data
    // sends the operator to a pull that can never fix it, which is the §4
    // fallback that hides a failure in its most expensive form.
    //
    // Only the ENDPOINTS need checking: every month between them comes from
    // `next_month`, which yields 1..=12 by construction.
    for (label, (y, m)) in [("FROM", from), ("TO", to)] {
        if m == 0 || m > 12 {
            return Err(format!(
                "{label} month {m} is not a month: {y}-{m:02} does not exist. \
                 MONTH is 1..=12. Nothing was read."
            ));
        }
    }
    if (from.0, from.1) > (to.0, to.1) {
        return Err(format!(
            "the range runs backwards: {}-{:02} is after {}-{:02}. Give FROM \
             first and TO second.",
            from.0, from.1, to.0, to.1
        ));
    }
    // COUNTED BEFORE IT IS WALKED. `to.0 - from.0` is at most 65,535 years, so
    // the product cannot overflow a `usize` on any target this builds for, and
    // the check happens before a single month is pushed.
    let months = usize::from(to.0.saturating_sub(from.0))
        .saturating_mul(12)
        .saturating_add(usize::from(to.1))
        .saturating_sub(usize::from(from.1))
        .saturating_add(1);
    if months > MAX_SPAN_MONTHS {
        return Err(format!(
            "{}-{:02}..{}-{:02} is {months} months. The longest span this \
             command assembles is {MAX_SPAN_MONTHS}. Nothing was read.",
            from.0, from.1, to.0, to.1
        ));
    }
    let mut out = Vec::with_capacity(months);
    let mut at = from;
    loop {
        out.push(at);
        if at == to {
            break;
        }
        let Some(next) = next_month(at.0, at.1) else {
            return Err(format!(
                "the range ran past 9999-12 before reaching {}-{:02}",
                to.0, to.1
            ));
        };
        at = next;
    }
    Ok(out)
}

/// Loads every month in `from ..= to` as ONE series.
///
/// # The join is checked, not assumed
///
/// Two files opened in order are two files, and nothing about the filesystem
/// guarantees the last bar of one precedes the first bar of the next. The whole
/// engine assumes a strictly increasing series: `Column::build` folds bar by bar
/// and `CLAUDE.md` §3 rule 7's no-look-ahead property is held by that shape, so
/// a series that steps backwards at a join would fold a later bar into an
/// earlier state and no test downstream would catch it.
///
/// So the boundary is COMPARED and a non-increasing step REFUSES. That is a
/// corrupt or mis-keyed store and the operator's next action is to look at the
/// files, not to read a number computed over them.
///
/// # Errors
///
/// A backwards range, a bad instrument or rung, a non-increasing join, or an
/// unreadable record. A month the store simply does not hold is NOT an error —
/// it is recorded in [`Span::missing`] and the span continues, because a
/// seven-year request with one month un-pulled should return six years and
/// eleven months and say so, not refuse everything.
///
/// # Cost
///
/// One `open_existing` per month and one `read_record` per bar, each O(1), with
/// the destination reserved once from the sum of the headers' own record counts.
/// Total work is O(total bars), which is the size of the answer and not a
/// per-operation cost: `CLAUDE.md` §3 rule 4 governs bar lookup, condition
/// lookup, mask evaluation, duplicate rejection and result append, and this is
/// none of them. Nothing here scans, sorts or searches.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
pub fn load_span(
    root: &Path,
    vendor: Vendor,
    underlying: &str,
    rung_name: &str,
    from: (u16, u8),
    to: (u16, u8),
) -> Result<Span, Refusal> {
    let timeframe = rung(rung_name)?;
    let key = InstrumentKey::index(Exchange::Nse, underlying)
        .map_err(|why| format!("`{underlying}` is not an index this engine sweeps: {why}"))?;
    let wanted = months_between(from, to)?;

    let mut bars: Vec<Candle> = Vec::new();
    let mut missing: Vec<(u16, u8)> = Vec::new();
    let mut found: u32 = 0;

    for &(year, month) in &wanted {
        match load(root, vendor, underlying, rung_name, year, month) {
            Err(_) => missing.push((year, month)),
            Ok(one) => {
                // THE JOIN, CHECKED. Compared against the last bar already held
                // rather than against the previous month's own last bar, so a
                // hole in the middle does not let a backwards step through.
                if let (Some(prev), Some(first)) = (bars.last(), one.bars.first())
                    && first.ts_micros <= prev.ts_micros
                {
                    return Err(format!(
                        "the span steps backwards at {year}-{month:02}: that \
                         month's first bar is stamped {} and the bar before it \
                         is stamped {}. A series that is not strictly \
                         increasing folds a later bar into an earlier state, so \
                         nothing was swept. Check the store for that month.",
                        first.ts_micros, prev.ts_micros
                    ));
                }
                found = found.saturating_add(1);
                bars.extend(one.bars);
            }
        }
    }

    if bars.is_empty() {
        return Err(format!(
            "{underlying} {rung_name} {}-{:02}..{}-{:02} holds no bars for {}: \
             all {} month(s) are absent from the store. Nothing was read.",
            from.0,
            from.1,
            to.0,
            to.1,
            vendor.as_str(),
            wanted.len()
        ));
    }

    Ok(Span {
        bars,
        vendor,
        key,
        timeframe: timeframe.as_str(),
        asked: u32::try_from(wanted.len()).unwrap_or(u32::MAX),
        found,
        missing,
    })
}
#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    // The seeding helper computes the store's own symbol id the store's own
    // way. Same reason as the production site above.
    clippy::cast_possible_truncation
)]
mod tests {
    use super::*;
    use store::format::Bar;

    /// A store root of this test's own, so `cargo test`'s threads cannot collide.
    ///
    /// The same idiom `crates/store`'s own tests use, tagged per test and
    /// suffixed with the process id.
    fn root(tag: &str) -> std::path::PathBuf {
        let p =
            std::env::temp_dir().join(format!("brutex-cli-stored-{tag}-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&p);
        p
    }

    /// `n` bars on a one-minute grid from the 2026-08-03 open, in paisa.
    fn bars(n: i64) -> Vec<Bar> {
        // 2026-08-03 09:15 IST as epoch micros, so the timestamps land inside
        // the month the path names. A bar outside its own month is a refusal the
        // store makes for itself and is not what these tests are about.
        let open = 1_785_727_500_000_000_i64;
        (0..n)
            .map(|i| Bar {
                ts_micros: open + i * 60_000_000,
                open: 2_500_000 + i,
                high: 2_500_100 + i,
                low: 2_499_900 + i,
                close: 2_500_050 + i,
                volume: 0,
                open_interest: i64::MIN,
            })
            .collect()
    }

    /// Writes `n` bars for NIFTY 1min 2026-08 under `vendor`, and returns the root.
    fn seeded(tag: &str, vendor: Vendor, n: i64) -> std::path::PathBuf {
        let r = root(tag);
        let key = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("NIFTY is swept");
        let ym = YearMonth::new(2026, 8).expect("a real month");
        let path = StorePath::for_key(vendor, &key, Timeframe::MINUTE_1, ym, FileKind::Bars)
            .expect("a path for a swept index");
        let id = brutex_core::universe::fnv1a("NIFTY") as u32;
        let mut file = BarFile::open_or_create(&r, path, id).expect("a fresh month opens");
        // An empty batch is refused by the store as `EmptyBatch`, correctly — so
        // thezero -bar case is a file that was created and never appended to, which
        // is exactly the state a reached-but-empty month leaves behind.
        if n > 0 {
            file.append(&bars(n)).expect("and takes its bars");
        }
        r
    }

    #[test]
    fn a_stored_month_loads_with_its_provenance_attached() {
        let r = seeded("happy", Vendor::Dhan, 5);
        let got = load(&r, Vendor::Dhan, "NIFTY", "1min", 2026, 8).expect("it is in the store");

        assert_eq!(got.bars.len(), 5, "every record is read");
        assert_eq!(
            got.vendor,
            Vendor::Dhan,
            "the feed comes back with the bars"
        );
        assert_eq!(got.timeframe, "1min", "and so does the rung");
        assert_eq!(
            got.key.underlying.as_str(),
            "NIFTY",
            "and the instrument it is bars of"
        );
    }

    #[test]
    fn every_field_survives_the_crossing() {
        // `store::format::Bar` and `indicators::Candle` are two names for one
        // record. Seven fields, copied by hand because the crates that own them
        // may not name each other, so seven fields are asserted by hand.
        let r = seeded("fields", Vendor::Groww, 3);
        let got = load(&r, Vendor::Groww, "NIFTY", "1min", 2026, 8).expect("stored");
        let want = bars(3);
        for (i, (c, b)) in got.bars.iter().zip(want.iter()).enumerate() {
            assert_eq!(c.ts_micros, b.ts_micros, "bar {i} ts");
            assert_eq!(c.open, b.open, "bar {i} open");
            assert_eq!(c.high, b.high, "bar {i} high");
            assert_eq!(c.low, b.low, "bar {i} low");
            assert_eq!(c.close, b.close, "bar {i} close");
            assert_eq!(c.volume, b.volume, "bar {i} volume");
            assert_eq!(
                c.open_interest, b.open_interest,
                "bar {i} open interest — the i64::MIN sentinel must cross intact"
            );
        }
    }

    #[test]
    fn the_bars_come_back_oldest_first() {
        let r = seeded("order", Vendor::Dhan, 8);
        let got = load(&r, Vendor::Dhan, "NIFTY", "1min", 2026, 8).expect("stored");
        assert!(
            got.bars.windows(2).all(|w| w[0].ts_micros < w[1].ts_micros),
            "the sweep reads bars 0..N and a reversed column would silently invert every condition"
        );
    }

    #[test]
    fn a_month_that_was_never_pulled_says_so_and_names_the_pull() {
        let r = root("absent");
        let why = load(&r, Vendor::Dhan, "NIFTY", "1min", 2026, 8).expect_err("nothing is there");
        assert!(
            why.contains("not in the store"),
            "it names the absence: {why}"
        );
        assert!(why.contains("dhan"), "and which feed's store: {why}");
        assert!(
            why.contains("Pull that instrument-month first"),
            "and the action that fixes it, because a filesystem error names the wrong next step: {why}"
        );
    }

    #[test]
    fn a_rung_the_store_does_not_carry_lists_the_ones_it_does() {
        let why = load(&root("rung"), Vendor::Dhan, "NIFTY", "1hour", 2026, 8)
            .expect_err("1hour is not a rung");
        assert!(why.contains("`1hour` is not a rung"), "{why}");
        assert!(
            why.contains("60min"),
            "an operator who typed 1hour needs to be told the word is 60min: {why}"
        );
        assert!(
            why.contains("1day"),
            "the list is every rung, not a sample: {why}"
        );
    }

    #[test]
    fn a_name_the_store_has_no_file_for_is_refused_as_absent() {
        // AND `InstrumentKey::index` DOES NOT VALIDATE THE NAME. It accepts
        // `RELIANCE` and renders `NSE/INDEX/RELIANCE/…`, even though CLAUDE.md
        // §1 puts exactly two instruments on the engine surface. The refusal
        // therefore comes from the STORE — no such file — and not from the key.
        // That is worth knowing: nothing between a typed symbol and a path
        // checks it is swept, so the absence of the file is the only guard.
        let why = load(&root("ins"), Vendor::Dhan, "RELIANCE", "1min", 2026, 8)
            .expect_err("nothing was ever pulled for it");
        assert!(
            why.contains("RELIANCE"),
            "the refusal quotes what was asked: {why}"
        );
        assert!(why.contains("not in the store"), "{why}");
    }

    #[test]
    fn a_month_that_is_not_a_month_is_refused_before_any_path_is_built() {
        let why =
            load(&root("month"), Vendor::Dhan, "NIFTY", "1min", 2026, 13).expect_err("no month 13");
        assert!(why.contains("2026-13 is not a month"), "{why}");
    }

    #[test]
    fn an_empty_month_loads_as_empty_and_not_as_an_error() {
        // A file that exists and holds nothing is a real answer: the month was
        // reached and had no session. Reporting it as a refusal would send the
        // operator to pull something that is already there.
        let r = seeded("empty", Vendor::Dhan, 0);
        let got = load(&r, Vendor::Dhan, "NIFTY", "1min", 2026, 8).expect("the file exists");
        assert!(got.bars.is_empty());
        assert_eq!(
            got.vendor,
            Vendor::Dhan,
            "provenance survives an empty month"
        );
    }

    #[test]
    fn two_feeds_are_two_stores_and_one_does_not_answer_for_the_other() {
        // The whole reason the vendor is the first path segment (D-0019). If
        // this ever passes for the wrong feed, a run could be attributed to a
        // broker that never sent the bars.
        let r = seeded("split", Vendor::Dhan, 4);
        assert_eq!(
            load(&r, Vendor::Dhan, "NIFTY", "1min", 2026, 8)
                .expect("dhan is there")
                .bars
                .len(),
            4
        );
        assert!(
            load(&r, Vendor::Groww, "NIFTY", "1min", 2026, 8).is_err(),
            "groww's month was never written and must not resolve to dhan's"
        );
    }

    /// Days since 1970-01-01 for a civil date, by Howard Hinnant's algorithm.
    ///
    /// Written out rather than pulled in: `CLAUDE.md` §2 allows no new
    /// dependency for a test helper, and the store refuses a bar stamped outside
    /// the month its path names — so a multi-month fixture MUST compute a real
    /// timestamp per month rather than reusing one.
    const fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
        let y = if m <= 2 { y - 1 } else { y };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The 3rd of that month at the same time of day the single-month fixture
    /// uses, as epoch micros.
    ///
    /// # 03:25 UTC, and the twenty minutes are worth a sentence
    ///
    /// `bars` above hard-codes `1_785_727_500_000_000` and its comment calls it
    /// "2026-08-03 09:15 IST". That constant is 03:25 UTC, which is 08:55 IST --
    /// twenty minutes before the open the comment names. The first draft of this
    /// helper computed 09:15 honestly and disagreed with the constant by exactly
    /// 1,200,000,000 micros, which is how the gap was found.
    ///
    /// This matches the CONSTANT rather than the comment, deliberately: both
    /// fixtures must stamp bars the same way or a span test and a month test
    /// would be measuring different grids. Whether the comment or the constant
    /// is the thing to change is a question about a fixture that predates this
    /// module and is not one a span test should answer by quietly diverging.
    /// Neither value affects any assertion here -- every bar lands inside the
    /// month its path names either way, which is all the store checks.
    const fn opening_micros(year: i64, month: i64) -> i64 {
        (days_from_civil(year, month, 3) * 86_400 + 3 * 3_600 + 25 * 60) * 1_000_000
    }

    /// `n` bars on a one-minute grid from that month's 3rd, on the same
    /// time-of-day grid as `bars`.
    fn bars_in(year: i64, month: i64, n: i64) -> Vec<Bar> {
        let open = opening_micros(year, month);
        (0..n)
            .map(|i| Bar {
                ts_micros: open + i * 60_000_000,
                open: 2_500_000 + i,
                high: 2_500_100 + i,
                low: 2_499_900 + i,
                close: 2_500_050 + i,
                volume: 0,
                open_interest: i64::MIN,
            })
            .collect()
    }

    /// Writes `n` bars into each of `months` for NIFTY 1min, and returns the root.
    fn seeded_months(
        tag: &str,
        vendor: Vendor,
        months: &[(u16, u8)],
        n: i64,
    ) -> std::path::PathBuf {
        let r = root(tag);
        let key = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("NIFTY is swept");
        let id = brutex_core::universe::fnv1a("NIFTY") as u32;
        for &(y, m) in months {
            let ym = YearMonth::new(y, m).expect("a real month");
            let path = StorePath::for_key(vendor, &key, Timeframe::MINUTE_1, ym, FileKind::Bars)
                .expect("a path for a swept index");
            let mut file = BarFile::open_or_create(&r, path, id).expect("a fresh month opens");
            file.append(&bars_in(i64::from(y), i64::from(m), n))
                .expect("and takes its bars");
        }
        r
    }

    /// The date helper agrees with the constant the single-month fixture uses.
    ///
    /// Without this the multi-month fixtures could all be stamped consistently
    /// WRONG and every span test would still pass, because they only ever
    /// compare against each other.
    #[test]
    fn the_date_helper_reproduces_the_fixture_constant() {
        assert_eq!(
            opening_micros(2026, 8),
            1_785_727_500_000_000,
            "2026-08-03 09:15 IST is the timestamp `bars` already uses"
        );
    }

    #[test]
    fn a_range_walks_forward_and_includes_both_ends() {
        assert_eq!(
            months_between((2026, 1), (2026, 3)).expect("forward"),
            vec![(2026, 1), (2026, 2), (2026, 3)]
        );
    }

    #[test]
    fn a_range_of_one_month_is_that_month() {
        assert_eq!(
            months_between((2026, 5), (2026, 5)).expect("one"),
            vec![(2026, 5)]
        );
    }

    #[test]
    fn a_range_rolls_the_year_at_december() {
        assert_eq!(
            months_between((2025, 11), (2026, 2)).expect("rolls"),
            vec![(2025, 11), (2025, 12), (2026, 1), (2026, 2)]
        );
    }

    #[test]
    fn a_backwards_range_is_refused_by_name() {
        let why = months_between((2026, 8), (2026, 1)).expect_err("backwards");
        assert!(
            why.contains("runs backwards"),
            "the refusal must say which way round to give them: {why}"
        );
    }

    #[test]
    fn a_span_longer_than_the_cap_refuses_before_it_walks() {
        // `YEAR` parses as u16, so this is the range an operator can actually
        // type. 763,000 months of failed file opens is not a refusal, it is a
        // hang -- so the count is checked before the first month is pushed.
        let why = months_between((1970, 1), (65535, 1)).expect_err("too long");
        assert!(
            why.contains("1200"),
            "the refusal must name the bound rather than saying `too long`: {why}"
        );
        // And the boundary itself is admitted, so the cap is a cap and not an
        // off-by-one that rejects the longest legal span.
        assert_eq!(
            months_between((2000, 1), (2099, 12))
                .expect("exactly the cap")
                .len(),
            MAX_SPAN_MONTHS
        );
    }

    #[test]
    fn a_span_joins_its_months_into_one_strictly_increasing_series() {
        let r = seeded_months(
            "span-join",
            Vendor::Zerodha,
            &[(2026, 1), (2026, 2), (2026, 3)],
            4,
        );
        let got = load_span(&r, Vendor::Zerodha, "NIFTY", "1min", (2026, 1), (2026, 3))
            .expect("three months are in the store");

        assert_eq!(got.bars.len(), 12, "every bar of every month is present");
        assert_eq!(got.found, 3);
        assert_eq!(got.asked, 3);
        assert!(got.complete(), "nothing is missing");
        assert!(got.missing.is_empty());

        // THE PROPERTY THE WHOLE ENGINE RESTS ON. `Column::build` folds bar by
        // bar and §3 rule 7's no-look-ahead holds by that shape, so a join that
        // stepped backwards would fold a later bar into an earlier state and
        // nothing downstream would notice.
        for pair in got.bars.windows(2) {
            let (a, b) = (
                pair.first().expect("a pair has a first"),
                pair.last().expect("and a last"),
            );
            assert!(
                b.ts_micros > a.ts_micros,
                "the joined series must be strictly increasing at every step, \
                 including across a month boundary: {} then {}",
                a.ts_micros,
                b.ts_micros
            );
        }
    }

    #[test]
    fn a_month_the_store_lacks_is_named_and_the_span_continues() {
        // February is absent. A seven-year request with one month un-pulled must
        // return six years and eleven months AND SAY SO -- refusing everything
        // would be worse, and skipping it silently is the §4 fallback.
        let r = seeded_months("span-hole", Vendor::Zerodha, &[(2026, 1), (2026, 3)], 5);
        let got = load_span(&r, Vendor::Zerodha, "NIFTY", "1min", (2026, 1), (2026, 3))
            .expect("two of the three months are there");

        assert_eq!(got.bars.len(), 10, "only the months that exist contribute");
        assert_eq!(got.asked, 3);
        assert_eq!(got.found, 2);
        assert_eq!(got.missing, vec![(2026, 2)], "the hole is named, in order");
        assert!(!got.complete(), "and the span knows it is not whole");
    }

    #[test]
    fn a_span_the_store_holds_nothing_for_refuses_rather_than_returning_empty() {
        let r = root("span-empty");
        let why = load_span(&r, Vendor::Zerodha, "NIFTY", "1min", (2026, 1), (2026, 3))
            .expect_err("nothing is stored");
        assert!(
            why.contains("all 3 month(s) are absent"),
            "the refusal must say how many months it looked for: {why}"
        );
    }

    #[test]
    fn a_span_refuses_a_rung_and_an_instrument_the_store_cannot_carry() {
        let r = seeded_months("span-bad", Vendor::Zerodha, &[(2026, 1)], 2);
        assert!(
            load_span(&r, Vendor::Zerodha, "NIFTY", "7min", (2026, 1), (2026, 1))
                .expect_err("no such rung")
                .contains("7min"),
            "an unknown rung is refused before any month is opened"
        );
        assert!(
            load_span(
                &r,
                Vendor::Zerodha,
                "../../etc",
                "1min",
                (2026, 1),
                (2026, 1)
            )
            .expect_err("not an index")
            .contains("is not an index this engine sweeps"),
            "a path-shaped name is refused as an instrument, not walked"
        );
    }

    #[test]
    fn a_month_outside_one_to_twelve_is_refused_and_never_reported_as_missing() {
        // FOUND BY A TEST, NOT BY REVIEW. `MONTH` parses as `u8`, so 13..=255
        // arrive intact, walk into the loop, build no path, and land in
        // `Span::missing`. The operator would read "2019-13 is missing from the
        // store" and go pull a month that cannot exist.
        for bad in [0_u8, 13, 99, 255] {
            let why = months_between((2019, bad), (2026, 8))
                .expect_err("a month outside 1..=12 is not a month");
            assert!(
                why.contains("is not a month"),
                "FROM month {bad} must be refused as malformed, not walked: {why}"
            );
            assert!(
                why.contains("FROM"),
                "and the refusal must say WHICH end was wrong: {why}"
            );
            let why =
                months_between((2019, 1), (2026, bad)).expect_err("the far end is checked too");
            assert!(
                why.contains("TO"),
                "the TO end must be named just as clearly: {why}"
            );
        }
        // And the legal ends are still admitted, so the guard is a guard and not
        // an off-by-one that rejects January or December.
        assert_eq!(
            months_between((2026, 1), (2026, 12)).expect("legal").len(),
            12
        );
    }
}

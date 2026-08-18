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
}

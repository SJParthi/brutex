#![cfg(test)]
//! Round-5 attack tests of the GDFL data path. Round 4 recorded the bar
//! definition in every `done` line (D-3191) and stated that a day built
//! under an old definition "never passes as a clean result", leaning on the
//! append-only store to refuse different bars. The store compares only the
//! bars it is OFFERED; a definition that builds FEWER bars of a day than the
//! store holds offered a run the store matched record for record, and the
//! day was closed `done` with the old bars still standing. Every value is
//! invented at run time; no vendor row is quoted. Every property draws from
//! a fixed-seed splitmix64, so a rerun is the same run (`CLAUDE.md` §3
//! rule 5).
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail; the reference indexes on purpose"
)]

use std::path::Path;

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe};

use super::*;
use crate::gdfl_fixtures::{put, scratch};

// ── shared helpers ─────────────────────────────────────────────────────────

/// splitmix64.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn d1() -> Day {
    day(2024, 4, 1)
}

const T: u32 = 9 * 3_600 + 15 * 60;

fn tick(sod: u32, ltp: i64, ltq: u64) -> Tick {
    Tick {
        sod,
        ltp,
        ltq,
        oi: None,
    }
}

fn stock(symbol: &str, ticks: Vec<Tick>) -> TickFile {
    TickFile {
        symbol: symbol.to_owned(),
        contract: None,
        name: format!("{symbol}.invented"),
        ticks,
    }
}

/// One stocks run over `from..=to`, every held day handed `files`.
fn run_stocks(store: &Path, from: Day, to: Day, files: &[TickFile]) -> Report {
    let run = Run {
        kind: ImportKind::Stocks,
        from,
        to,
        only: &[],
        store_root: store,
    };
    drive(&run, |_, sink| {
        for file in files {
            sink(file.clone());
        }
        DayRead {
            held: true,
            ..DayRead::default()
        }
    })
    .unwrap()
}

fn journal_text(store: &Path) -> String {
    std::fs::read_to_string(journal_path(store)).unwrap_or_default()
}

/// The journal with every `definition=<current>` replaced by `with`.
fn restate_journal(store: &Path, with: &str) {
    let current = format!(" definition={BAR_DEFINITION}");
    let text = journal_text(store).replace(&current, with);
    put(store, "imports/gdfl.journal", text.as_bytes());
}

/// Every one-second bar of `symbol`'s April 2024 cash month, or empty.
fn held(store: &Path, symbol: &str) -> Vec<Bar> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "CASH",
        symbol,
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: d1().year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    #[expect(clippy::cast_possible_truncation, reason = "the store's own id fold")]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    BarFile::open_existing(store, path, id).map_or_else(
        |_| Vec::new(),
        |file| {
            (0..file.header().n_valid)
                .map(|i| file.read_record(i).unwrap())
                .collect()
        },
    )
}

/// The file with every untraded row taken out. Definition 1 (D-2802 as
/// first written) dropped those rows BEFORE it placed the rest, so what it
/// built from `file` is exactly what the current definition builds from
/// this: with every row traded the two definitions are one rule.
fn traded_only(file: &TickFile) -> TickFile {
    TickFile {
        ticks: file.ticks.iter().copied().filter(|t| t.ltq > 0).collect(),
        ..file.clone()
    }
}

// ── 1. a restated day whose bars are fewer than the store holds ────────────

/// D-3170's own shape, with nothing after the deferred trade: the untraded
/// row at T+10 makes the trade at T+3 late, and no row after it is in order,
/// so the current definition drops it (`late_unresolved`) where definition 1
/// filed it at T+3. The rebuilt day is [T]; the store holds [T, T+3]. The
/// store was offered [T], matched it record for record and answered
/// `AlreadyPresent`; the day was closed `done` under this definition with
/// the T+3 bar, a print the file had not yet shown by T+3, still standing.
/// The same when the rebuilt day has no bar at all: no write was attempted,
/// so nothing was compared. And when both old trades are deferred to one
/// new second after them, the store was offered [T+20], found it after
/// everything it held, and APPENDED it beside the two old bars: a day that
/// neither definition builds. Each must be refused by name on every run,
/// the bars never touched, the day never closed `done`: `incomplete` when a
/// write was begun, no line when none was (D-3141).
#[test]
fn r5_01_a_restated_day_with_fewer_bars_than_the_store_holds_is_never_clean() {
    let cases = [
        (
            "a trailing bar dropped",
            vec![
                tick(T, 10_000, 2),
                tick(T + 10, 10_020, 0),
                tick(T + 3, 10_050, 4),
            ],
        ),
        (
            "every bar dropped",
            vec![
                tick(T + 10, 10_020, 0),
                tick(T, 10_000, 2),
                tick(T + 3, 10_050, 4),
            ],
        ),
        (
            "one new bar after every old one",
            vec![
                tick(T + 10, 10_020, 0),
                tick(T, 10_000, 2),
                tick(T + 3, 10_050, 4),
                tick(T + 20, 10_070, 0),
            ],
        ),
    ];
    for (what, ticks) in cases {
        let root = scratch("r5-fewer");
        let file = stock("RELIANCE", ticks);
        let old = run_stocks(&root, d1(), d1(), &[traded_only(&file)]);
        assert!(old.failures.is_empty(), "{what}: {:?}", old.failures);
        restate_journal(&root, " definition=1");
        let before = held(&root, "RELIANCE");
        assert_eq!(before.len(), 2, "{what}: definition 1 built two bars");
        for attempt in 0..3 {
            let got = run_stocks(&root, d1(), d1(), std::slice::from_ref(&file));
            assert_eq!(got.days_skipped, 0, "{what} run {attempt}: not skipped");
            assert_eq!(got.resumed, vec![d1()], "{what} run {attempt}");
            assert_eq!(got.seconds_committed, 0, "{what} run {attempt}");
            assert!(
                got.failures
                    .iter()
                    .any(|f| f.instrument.contains("RELIANCE.invented")),
                "{what} run {attempt}: refused by name: {:?}",
                got.failures
            );
            assert_eq!(held(&root, "RELIANCE"), before, "{what}: never rewritten");
            // Never closed `done` under this definition: `incomplete` when a
            // write was begun, and no line at all when none was (a day that
            // wrote nothing and failed records nothing, D-3190), so the day
            // stays restated and is named on every run.
            let text = journal_text(&root);
            let current = format!("done stocks 2024-04-01 * definition={BAR_DEFINITION} ");
            assert!(
                !text.lines().any(|line| line.starts_with(&current)),
                "{what} run {attempt}: {text}"
            );
            assert_eq!(got.restated, vec![d1()], "{what} run {attempt}");
            let last = text.lines().last().unwrap();
            let begun = what == "a trailing bar dropped";
            assert_eq!(
                last.starts_with(&format!(
                    "incomplete stocks 2024-04-01 * definition={BAR_DEFINITION} "
                )),
                begun,
                "{what} run {attempt}: {text}"
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// Over random files: a day rebuilt under the current definition after
/// definition 1 is clean EXACTLY when the store then holds, bar for bar,
/// what the current definition builds from the file into an empty store.
/// Fewer bars than the store holds, or other bars, are a named refusal; the
/// same bars, or the old ones and a following suffix, are clean (D-3141).
#[test]
fn r5_02_a_restated_day_is_clean_exactly_when_the_store_holds_what_it_builds() {
    let mut rng = Rng(0x5EED_0005_0000_0001);
    let (mut clean, mut refused, mut fewer) = (0, 0, 0);
    for case in 0..240 {
        let rows = 1 + rng.below(10);
        let ticks: Vec<Tick> = (0..rows)
            .map(|_| {
                let sod = T + u32::try_from(rng.below(30)).unwrap();
                let ltq = if rng.below(3) == 0 {
                    0
                } else {
                    1 + rng.below(5)
                };
                let ltp = 10_000 + i64::try_from(rng.below(40)).unwrap() * 5;
                tick(sod, ltp, ltq)
            })
            .collect();
        let file = stock("SBIN", ticks);
        if convert(ImportKind::Stocks, d1(), &traded_only(&file).ticks)
            .unwrap()
            .seconds
            .is_empty()
        {
            continue;
        }
        let fresh = scratch("r5-fresh");
        let want = run_stocks(&fresh, d1(), d1(), std::slice::from_ref(&file));
        assert!(want.failures.is_empty(), "{case}: {:?}", want.failures);
        let built = held(&fresh, "SBIN");
        std::fs::remove_dir_all(fresh).unwrap();
        let root = scratch("r5-restate");
        let old_bars = {
            let old = run_stocks(&root, d1(), d1(), &[traded_only(&file)]);
            assert!(old.failures.is_empty(), "{case}: {:?}", old.failures);
            held(&root, "SBIN")
        };
        restate_journal(&root, " definition=1");
        let got = run_stocks(&root, d1(), d1(), std::slice::from_ref(&file));
        let now = held(&root, "SBIN");
        let same = now == built;
        assert_eq!(
            got.failures.is_empty(),
            same,
            "{case}: clean exactly when the store holds what this definition builds; \
             old {old_bars:?} built {built:?} now {now:?} failures {:?}",
            got.failures
        );
        let current = format!("done stocks 2024-04-01 * definition={BAR_DEFINITION} ");
        let last = journal_text(&root).lines().last().unwrap().to_owned();
        assert_eq!(
            last.starts_with(&current),
            same,
            "{case}: closed done under this definition exactly when clean: {last}"
        );
        if same {
            clean += 1;
        } else {
            refused += 1;
            assert_eq!(now, old_bars, "{case}: refused, never rewritten; built {built:?} failures {:?}", got.failures);
            if built.len() < old_bars.len() {
                fewer += 1;
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    assert!(
        clean > 20 && refused > 20 && fewer > 5,
        "every branch drawn: {clean} clean, {refused} refused, {fewer} fewer"
    );
}

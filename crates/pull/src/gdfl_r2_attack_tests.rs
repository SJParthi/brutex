#![cfg(test)]
//! Round-2 attack tests of the GDFL data path: the import runtime, the
//! options and capital-market readers and both sources, run end to end over
//! random invented day trees and compared with a reference written here,
//! slowly and independently. Every value is invented at run time; no vendor
//! row is quoted (`gdfl_cm::tests::fixtures_are_built_not_pasted`). Every
//! property draws from a fixed-seed splitmix64, so a rerun is the same run
//! (`CLAUDE.md` §3 rule 5).
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail; the reference indexes on purpose"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe};

use super::*;
use crate::gdfl_archive::{Archive, MONTHS};
use crate::gdfl_fixtures::{Method, bts, csv, hms, put, scratch, zip};
use crate::gdfl_nfo::{NfoTickStore, NfoZips};
use crate::gdfl_tickstore::TickStore;

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

    fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[usize::try_from(self.below(from.len() as u64)).unwrap()]
    }
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn plus(on: Day, days: u32) -> Day {
    Day::from_days(on.days_from_epoch() + days).unwrap()
}

/// 1970-01-01 was a Thursday.
fn weekday(on: Day) -> bool {
    (on.days_from_epoch() + 3) % 7 < 5
}

fn ddmmyyyy(on: Day) -> String {
    format!("{:02}{:02}{:04}", on.day(), on.month(), on.year())
}

fn month_name(on: Day) -> &'static str {
    MONTHS[usize::from(on.month()) - 1]
}

const OPEN: u32 = 9 * 3_600 + 15 * 60;
const CLOSE: u32 = 15 * 3_600 + 30 * 60;

/// The days of April 2024 a world may use: each is asserted to be a regular
/// full session by the calendar before it is used.
const APRIL: [u8; 8] = [1, 2, 3, 4, 5, 8, 9, 10];

/// The UTC microsecond of second `sod` of IST day `on`, written here.
fn utc_micros(on: Day, sod: u32) -> i64 {
    (i64::from(on.days_from_epoch()) * 86_400 + i64::from(sod) - 19_800) * 1_000_000
}

/// The shortest decimal of a strike in paisa.
fn strike_text(paisa: i64) -> String {
    let (whole, frac) = (paisa / 100, paisa % 100);
    if frac == 0 {
        format!("{whole}")
    } else if frac % 10 == 0 {
        format!("{whole}.{}", frac / 10)
    } else {
        format!("{whole}.{frac:02}")
    }
}

/// A price in paisa as the vendor writes it, two places.
fn price_text(paisa: i64) -> String {
    format!("{}.{:02}", paisa / 100, paisa % 100)
}

/// One row of a GDFL file, every field written here.
fn line(stem: &str, on: Day, r: &Row) -> String {
    format!(
        "{stem},{:02}/{:02}/{:04},{},{},0,0,0,0,{},{}",
        on.day(),
        on.month(),
        on.year(),
        hms(r.sod),
        r.ltp_text,
        r.ltq,
        r.oi
    )
}

/// One invented row and what it should decode to.
#[derive(Clone, Debug)]
struct Row {
    sod: u32,
    ltp: i64,
    ltp_text: String,
    ltq: u64,
    oi: u64,
}

/// A file of random rows: forward steps, repeats, back-steps, forward
/// spikes, untraded rows, stamps before the open and after the close, and,
/// when `overflow`, one quantity past `i64::MAX`.
fn random_rows(rng: &mut Rng, overflow: bool) -> Vec<Row> {
    let n = rng.below(48);
    let mut sod = OPEN - 120 + u32::try_from(rng.below(400)).unwrap();
    let mut out = Vec::new();
    for _ in 0..n {
        match rng.below(100) {
            0..=34 => {}
            35..=64 => sod += u32::try_from(1 + rng.below(3)).unwrap(),
            65..=82 => sod = sod.saturating_sub(u32::try_from(1 + rng.below(6)).unwrap()),
            83..=90 => sod += u32::try_from(60 + rng.below(9_000)).unwrap(),
            _ => sod = sod.saturating_sub(u32::try_from(rng.below(90)).unwrap()),
        }
        sod = sod.min(86_399);
        let ltq = if rng.below(3) == 0 {
            0
        } else {
            1 + rng.below(5_000)
        };
        let ltp = 5 * (1 + i64::try_from(rng.below(200_000)).unwrap());
        out.push(Row {
            sod,
            ltp,
            ltp_text: price_text(ltp),
            ltq,
            oi: rng.below(10_000_000),
        });
    }
    if overflow {
        // Two traded rows in one in-session second whose sum, or one row
        // alone, is past `i64::MAX`: either way the file refuses.
        let at = OPEN + 600 + u32::try_from(rng.below(600)).unwrap();
        let big = u64::try_from(i64::MAX).unwrap() - rng.below(2);
        for ltq in [big, 2] {
            out.push(Row {
                sod: at,
                ltp: 500,
                ltp_text: price_text(500),
                ltq,
                oi: 1,
            });
        }
        // An in-order row after them, so neither is left unplaced.
        out.push(Row {
            sod: 86_399,
            ltp: 500,
            ltp_text: price_text(500),
            ltq: 1,
            oi: 1,
        });
    }
    out
}

/// What the reference expects one file to give.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Expect {
    rows: usize,
    zero: usize,
    late: usize,
    unresolved: usize,
    back: u32,
    /// Every one-second bar the fold makes, session or not.
    bars: Vec<Bar>,
}

/// The agreed bar, written the slow way: the running maximum over every row
/// before row `i` (traded or not), a late kept row placed at the stamp of the
/// next in-order row of any kind, the rest dropped and counted, then one bar
/// per placed second in file order. `None` when a placed quantity or a
/// second's sum does not fit an `i64`.
fn reference(on: Day, rows: &[Row], every_row: bool, oi: bool) -> Option<Expect> {
    let n = rows.len();
    let in_order = |i: usize| (0..i).all(|j| rows[j].sod <= rows[i].sod);
    let mut out = Expect {
        rows: n,
        ..Expect::default()
    };
    let mut placed: Vec<(u32, &Row)> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        if !every_row && r.ltq == 0 {
            out.zero += 1;
            continue;
        }
        if in_order(i) {
            placed.push((r.sod, r));
            continue;
        }
        let high = (0..i).map(|j| rows[j].sod).max().unwrap();
        out.late += 1;
        out.back = out.back.max(high - r.sod);
        match (i + 1..n).find(|&j| in_order(j)) {
            Some(j) => placed.push((rows[j].sod, r)),
            None => out.unresolved += 1,
        }
    }
    let mut sums: Vec<u128> = Vec::new();
    for (second, r) in placed {
        if !every_row && i64::try_from(r.ltq).is_err() {
            return None;
        }
        let volume = if every_row { 0 } else { u128::from(r.ltq) };
        let oi = if oi {
            i64::try_from(r.oi).unwrap()
        } else {
            OI_NULL
        };
        let ts = utc_micros(on, second);
        match out.bars.last_mut() {
            Some(bar) if bar.ts_micros == ts => {
                bar.high = bar.high.max(r.ltp);
                bar.low = bar.low.min(r.ltp);
                bar.close = r.ltp;
                bar.open_interest = oi;
                *sums.last_mut().unwrap() += volume;
            }
            _ => {
                out.bars.push(Bar {
                    ts_micros: ts,
                    open: r.ltp,
                    high: r.ltp,
                    low: r.ltp,
                    close: r.ltp,
                    volume: 0,
                    open_interest: oi,
                });
                sums.push(volume);
            }
        }
    }
    for (bar, sum) in out.bars.iter_mut().zip(sums) {
        bar.volume = i64::try_from(sum).ok()?;
    }
    Some(out)
}

/// Whether a bar's second is inside the venue's session, `[09:15, 15:30)`.
fn in_session(on: Day, bar: &Bar) -> bool {
    bar.ts_micros >= utc_micros(on, OPEN) && bar.ts_micros < utc_micros(on, CLOSE)
}

/// Every file under `dir`, relative path to bytes.
fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                out.insert(path.strip_prefix(dir).unwrap().to_path_buf(), bytes);
            }
        }
    }
    out
}

/// Every one-second bar of one instrument in April 2024, or `None` when the
/// store holds no such file.
fn held(store: &Path, segment: &str, symbol: &str, contract: Option<Contract>) -> Option<Vec<Bar>> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract,
        timeframe: Timeframe::SECOND_1,
        month: day(2024, 4, 1).year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    #[expect(clippy::cast_possible_truncation, reason = "the store's own id fold")]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    let file = BarFile::open_existing(store, path, id).ok()?;
    Some(
        (0..file.header().n_valid)
            .map(|i| file.read_record(i).unwrap())
            .collect(),
    )
}

/// The totals a report should carry, accumulated by the reference.
#[derive(Debug, Default, PartialEq, Eq)]
struct Totals {
    days_imported: usize,
    files: usize,
    files_skipped: usize,
    files_refused: usize,
    rows: usize,
    ltq_zero_dropped: usize,
    late_rows: usize,
    late_unresolved: usize,
    max_back_s: u32,
    seconds: usize,
    outside_session: usize,
}

fn totals_of(report: &Report) -> Totals {
    Totals {
        days_imported: report.days_imported,
        files: report.files,
        files_skipped: report.files_skipped,
        files_refused: report.files_refused,
        rows: report.rows,
        ltq_zero_dropped: report.ltq_zero_dropped,
        late_rows: report.late_rows,
        late_unresolved: report.late_unresolved,
        max_back_s: report.max_back_s,
        seconds: report.seconds,
        outside_session: report.outside_session,
    }
}

impl Totals {
    fn file(&mut self, want: &Expect, on: Day) {
        self.files += 1;
        self.rows += want.rows;
        self.ltq_zero_dropped += want.zero;
        self.late_rows += want.late;
        self.late_unresolved += want.unresolved;
        self.max_back_s = self.max_back_s.max(want.back);
        let kept = want.bars.iter().filter(|b| in_session(on, b)).count();
        self.seconds += kept;
        self.outside_session += want.bars.len() - kept;
    }
}

// ── the options world ──────────────────────────────────────────────────────

/// Underlyings a world draws from: two indices, an F&O share whose name
/// begins with another's (`NIFTYNXT50`), a symbol with `&`, one with a
/// leading digit, and a share outside today's F&O list.
const UNDERS: [&str; 6] = [
    "NIFTY",
    "BANKNIFTY",
    "NIFTYNXT50",
    "M&M",
    "360ONE",
    "TV18BRDCST",
];

/// One option contract of a world.
#[derive(Clone, Copy, Debug)]
struct Opt {
    under: &'static str,
    expiry: Day,
    paisa: i64,
    call: bool,
}

impl Opt {
    fn name(&self) -> String {
        format!(
            "{}{:02}{}{:02}{}{}",
            self.under,
            self.expiry.day(),
            month_name(self.expiry),
            self.expiry.year() % 100,
            strike_text(self.paisa),
            if self.call { "CE" } else { "PE" }
        )
    }

    fn segment(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}-{}-{}",
            self.expiry.year(),
            self.expiry.month(),
            self.expiry.day(),
            self.paisa,
            if self.call { "CE" } else { "PE" }
        )
    }
}

/// What one entry of an options day is.
#[derive(Clone, Debug)]
enum Entry {
    /// An option file of the day: its contract, its rows, and whether one
    /// row is bent (a three-place price) so the reader refuses it.
    Option {
        opt: Opt,
        rows: Vec<Row>,
        bent: bool,
        upper: bool,
    },
    /// An option file whose name does not decode (a weekend expiry), its
    /// underlying, and whether the name is spelled `.CSV`.
    Undecodable {
        under: &'static str,
        name: String,
        upper: bool,
    },
    /// Not an option file of the day.
    Other,
}

fn nfo_entry_name(on: Day, ticker: &str, upper: bool) -> String {
    format!(
        "GFDLNFO_TICK_{}\\Options\\{ticker}.NFO.{}",
        ddmmyyyy(on),
        if upper { "CSV" } else { "csv" }
    )
}

/// The bytes of one entry.
fn nfo_bytes(on: Day, entry: &Entry) -> (String, Vec<u8>) {
    match entry {
        Entry::Option {
            opt,
            rows,
            bent,
            upper,
        } => {
            let stem = format!("{}.NFO", opt.name());
            let mut lines: Vec<String> = rows.iter().map(|r| line(&stem, on, r)).collect();
            if *bent {
                let mut r = rows.first().cloned().unwrap_or(Row {
                    sod: OPEN,
                    ltp: 5,
                    ltp_text: String::new(),
                    ltq: 1,
                    oi: 0,
                });
                r.ltp_text = format!("{}1", price_text(r.ltp));
                lines.push(line(&stem, on, &r));
            }
            (nfo_entry_name(on, &opt.name(), *upper), csv(&lines))
        }
        Entry::Undecodable { name, upper, .. } => {
            let stem = format!("{name}.NFO");
            let r = Row {
                sod: OPEN,
                ltp: 5,
                ltp_text: price_text(5),
                ltq: 1,
                oi: 0,
            };
            (
                nfo_entry_name(on, name, *upper),
                csv(&[line(&stem, on, &r)]),
            )
        }
        Entry::Other => (
            format!("GFDLNFO_TICK_{}\\Futures\\NIFTY-I.NFO.csv", ddmmyyyy(on)),
            csv(&[]),
        ),
    }
}

/// A random world: one to three days of April 2024, each a random list of
/// entries over a small pool of contracts (so series recur across days),
/// with duplicates, undecodable names, bent files, fold refusals and
/// entries that are not option files.
fn random_world(rng: &mut Rng) -> Vec<(Day, Vec<Entry>)> {
    let first = usize::try_from(rng.below(6)).unwrap();
    let span = 1 + usize::try_from(rng.below(3)).unwrap();
    let days: Vec<Day> = APRIL[first..(first + span).min(APRIL.len())]
        .iter()
        .map(|&d| day(2024, 4, d))
        .collect();
    let mut pool: Vec<Opt> = Vec::new();
    for _ in 0..=rng.below(6) {
        let under = rng.pick(&UNDERS);
        let mut expiry = plus(days[days.len() - 1], u32::try_from(rng.below(90)).unwrap());
        while !weekday(expiry) {
            expiry = expiry.succ().unwrap();
        }
        pool.push(Opt {
            under,
            expiry,
            paisa: rng.pick(&[5_i64, 250, 7_750, 101_250, 2_200_000, 107_525]),
            call: rng.below(2) == 0,
        });
    }
    let mut world = Vec::new();
    for on in days {
        let mut entries: Vec<Entry> = Vec::new();
        for _ in 0..rng.below(9) {
            let entry = match rng.below(20) {
                0..=11 => Entry::Option {
                    opt: rng.pick(&pool),
                    rows: random_rows(rng, false),
                    bent: false,
                    upper: rng.below(4) == 0,
                },
                12 => Entry::Option {
                    opt: rng.pick(&pool),
                    rows: random_rows(rng, false),
                    bent: true,
                    upper: false,
                },
                13 => Entry::Option {
                    opt: rng.pick(&pool),
                    rows: random_rows(rng, true),
                    bent: false,
                    upper: false,
                },
                14..=16 => {
                    // 2024-04-06 is a Saturday: the dated shape, no contract.
                    let under = rng.pick(&UNDERS);
                    Entry::Undecodable {
                        under,
                        name: format!("{under}06APR24100CE"),
                        upper: rng.below(2) == 0,
                    }
                }
                17 => Entry::Undecodable {
                    under: "",
                    name: "QQ.Q".to_owned(),
                    upper: false,
                },
                _ => Entry::Other,
            };
            entries.push(entry);
        }
        world.push((on, entries));
    }
    world
}

/// Writes a world as a tick store under `root/ts` and as yearly zips under
/// `root/zips`.
fn write_world(root: &Path, world: &[(Day, Vec<Entry>)]) {
    let mut outer: Vec<(String, Vec<u8>)> = Vec::new();
    for (on, entries) in world {
        let files: Vec<(String, Vec<u8>)> = entries.iter().map(|e| nfo_bytes(*on, e)).collect();
        let refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        let rel = format!(
            "{:04}/{}_{:04}/GFDLNFO_TICK_{}",
            on.year(),
            month_name(*on),
            on.year(),
            ddmmyyyy(*on)
        );
        put(root, &format!("ts/options/{rel}.bts"), &bts(&refs));
        let members: Vec<(&str, &[u8], Method)> = files
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Deflated))
            .collect();
        outer.push((
            format!(
                "{}_{:04}/GFDLNFO_TICK_{}.zip",
                month_name(*on),
                on.year(),
                ddmmyyyy(*on)
            ),
            zip(&members),
        ));
    }
    let members: Vec<(&str, &[u8], Method)> = outer
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
        .collect();
    put(root, "zips/2024.zip", &zip(&members));
}

/// The longest name of `names` that `ticker` starts with.
fn longest<'a>(ticker: &str, names: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let mut best: Option<&str> = None;
    for name in names {
        if ticker.starts_with(name) && best.is_none_or(|b| name.len() > b.len()) {
            best = Some(name);
        }
    }
    best
}

/// What the reference expects of a whole options world.
struct Want {
    totals: Totals,
    /// Every failure, `day name` to the words its reason must hold.
    failures: Vec<(String, &'static str)>,
    /// Each contract's bars in the store, in day order.
    series: BTreeMap<(String, String), Vec<Bar>>,
    /// Days journalled `done` (no failure).
    clean_days: usize,
}

/// The reference for one world under the filter `only`: which entries are
/// wanted, refused or filed, independently of `nfo_day`.
fn want_nfo(world: &[(Day, Vec<Entry>)], only: &[&str]) -> Want {
    let wants = |under: &str| only.is_empty() || only.contains(&under);
    let mut want = Want {
        totals: Totals::default(),
        failures: Vec::new(),
        series: BTreeMap::new(),
        clean_days: 0,
    };
    for (on, entries) in world {
        want.totals.days_imported += 1;
        let before = want.failures.len();
        let mut per_contract: BTreeMap<String, usize> = BTreeMap::new();
        let mut per_name: BTreeMap<String, usize> = BTreeMap::new();
        for e in entries {
            match e {
                Entry::Option { opt, .. } => {
                    *per_contract
                        .entry(format!("{} {}", opt.under, opt.segment()))
                        .or_default() += 1;
                }
                Entry::Undecodable { name, .. } => *per_name.entry(name.clone()).or_default() += 1,
                Entry::Other => {}
            }
        }
        let mut files: Vec<(Opt, Expect)> = Vec::new();
        let mut late_failures: Vec<(String, &'static str)> = Vec::new();
        for e in entries {
            match e {
                Entry::Other => want.totals.files_skipped += 1,
                Entry::Undecodable { under, name, .. } => {
                    // Wanted by the longest F&O underlying it starts with
                    // (D-3175), or by a filter name no F&O underlying
                    // outspells (D-3167).
                    let fno = longest(name, brutex_core::universe::FNO_UNDERLYINGS);
                    let by_filter = longest(name, only.iter().copied());
                    let wanted = only.is_empty()
                        || match (by_filter, fno) {
                            (Some(f), Some(u)) => f.len() >= u.len(),
                            (Some(_), None) => true,
                            (None, _) => false,
                        };
                    let _ = under;
                    if wanted {
                        want.totals.files_refused += 1;
                        let why = if per_name[name] > 1 {
                            "two entries of one day name this ticker"
                        } else if name.contains("06APR24") {
                            "the stated expiry is not a real weekday"
                        } else {
                            "the ticker reads as neither GDFL option format"
                        };
                        late_failures.push((format!("{on} {name}"), why));
                    } else {
                        want.totals.files_skipped += 1;
                    }
                }
                Entry::Option {
                    opt, rows, bent, ..
                } => {
                    if !wants(opt.under) {
                        want.totals.files_skipped += 1;
                        continue;
                    }
                    let name = opt.name();
                    if per_contract[&format!("{} {}", opt.under, opt.segment())] > 1 {
                        want.totals.files_refused += 1;
                        late_failures.push((
                            format!("{on} {name}"),
                            "another ticker of the same day names the same contract",
                        ));
                        continue;
                    }
                    if *bent {
                        want.totals.files_refused += 1;
                        late_failures.push((format!("{on} {name}"), "the LTP is not a two-place"));
                        continue;
                    }
                    if let Some(expect) = reference(*on, rows, false, true) {
                        files.push((*opt, expect));
                    } else {
                        want.totals.files_refused += 1;
                        want.failures
                            .push((format!("{on} {name}"), "does not fit an i64"));
                    }
                }
            }
        }
        for (opt, expect) in files {
            want.totals.file(&expect, *on);
            let kept: Vec<Bar> = expect
                .bars
                .into_iter()
                .filter(|b| in_session(*on, b))
                .collect();
            want.series
                .entry((opt.under.to_owned(), opt.segment()))
                .or_default()
                .extend(kept);
        }
        want.failures.extend(late_failures);
        if want.failures.len() == before {
            want.clean_days += 1;
        }
    }
    want
}

fn run_options(root: &Path, zips: bool, store: &Path, only: &[String]) -> Report {
    let run = Run {
        kind: ImportKind::Options,
        from: day(2024, 4, 1),
        to: day(2024, 4, 10),
        only,
        store_root: store,
    };
    if zips {
        run_nfo(&NfoZips::new(&root.join("zips")), &run).unwrap()
    } else {
        run_nfo(&NfoTickStore::new(&root.join("ts")), &run).unwrap()
    }
}

/// Every listed entry of an options day is exactly one of a file, a skip or
/// a refusal, also when the fold refuses a file the reader read (D-3168).
/// Before the fix such a file was counted as read AND as refused.
#[test]
fn r2_01_every_options_entry_is_one_file_skip_or_refusal_even_when_the_fold_refuses() {
    let on = day(2024, 4, 1);
    let mut rng = Rng(7);
    let good = Opt {
        under: "NIFTY",
        expiry: day(2024, 4, 4),
        paisa: 2_200_000,
        call: true,
    };
    let big = Opt {
        call: false,
        ..good
    };
    let world = vec![(
        on,
        vec![
            Entry::Option {
                opt: good,
                rows: random_rows(&mut rng, false),
                bent: false,
                upper: false,
            },
            Entry::Option {
                opt: big,
                rows: random_rows(&mut rng, true),
                bent: false,
                upper: false,
            },
            Entry::Other,
        ],
    )];
    let root = scratch("r2-fold-count");
    write_world(&root, &world);
    let report = run_options(&root, false, &root.join("S"), &[]);
    assert_eq!(report.files_refused, 1, "{report:?}");
    assert_eq!(
        report.files + report.files_skipped + report.files_refused,
        3,
        "three entries, each counted once: {report:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A filter that names an underlying outside today's F&O list
/// (`TV18BRDCST`, an F&O share in the 2018-2019 archive) claims that
/// underlying's undecodable files as refusals, exactly as it claims its
/// decodable ones; before the fix they were counted as skips, so a filtered
/// run reported a broken file of the very underlying it asked for as nothing
/// at all (D-3167).
#[test]
fn r2_02_a_filter_claims_the_undecodable_files_of_its_own_underlying() {
    let on = day(2024, 4, 1);
    let mut rng = Rng(11);
    let opt = Opt {
        under: "TV18BRDCST",
        expiry: day(2024, 4, 25),
        paisa: 5_000,
        call: true,
    };
    let world = vec![(
        on,
        vec![
            Entry::Option {
                opt,
                rows: random_rows(&mut rng, false),
                bent: false,
                upper: false,
            },
            Entry::Undecodable {
                under: "TV18BRDCST",
                name: "TV18BRDCST06APR2450CE".to_owned(),
                upper: false,
            },
            // Not this filter's: `NIFTYNXT50` outspells `NIFTY` (D-3175).
            Entry::Undecodable {
                under: "NIFTYNXT50",
                name: "NIFTYNXT5006APR24100CE".to_owned(),
                upper: false,
            },
        ],
    )];
    let root = scratch("r2-filter-claim");
    write_world(&root, &world);
    for (only, refused, skipped) in [
        (vec!["TV18BRDCST".to_owned()], 1, 1),
        (vec!["NIFTY".to_owned()], 0, 3),
        (vec!["NIFTYNXT50".to_owned()], 1, 2),
        (Vec::new(), 2, 0),
    ] {
        let store = scratch("r2-filter-claim-store");
        let report = run_options(&root, false, &store, &only);
        assert_eq!(
            (report.files_refused, report.files_skipped),
            (refused, skipped),
            "{only:?}: {report:?}"
        );
        std::fs::remove_dir_all(store).unwrap();
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// 300 random options worlds, each imported from the tick store and from
/// the zips into two fresh stores, with and without a filter: the two runs
/// report the same and leave byte-identical stores; every report total,
/// every named failure and every stored bar of every contract equals the
/// reference; every entry is one file, skip or refusal; and a rerun writes
/// nothing and skips exactly the clean days.
#[test]
fn r2_03_random_options_worlds_match_the_reference_through_both_sources() {
    for &d in &APRIL {
        assert!(
            crate::gdfl_cm::session_gate(day(2024, 4, d)).is_ok(),
            "2024-04-{d:02} is a regular session"
        );
    }
    let mut rng = Rng(0x7232_4744_464C_574F);
    let (mut bars_checked, mut refusals, mut filtered) = (0_usize, 0_usize, 0_usize);
    for case in 0..300 {
        let world = random_world(&mut rng);
        let only: Vec<&str> = if rng.below(3) == 0 {
            filtered += 1;
            let mut only = vec![rng.pick(&UNDERS)];
            if rng.below(2) == 0 {
                only.push(rng.pick(&UNDERS));
            }
            only
        } else {
            Vec::new()
        };
        let only_owned: Vec<String> = only.iter().map(|s| (*s).to_owned()).collect();
        let root = scratch("r2-world");
        write_world(&root, &world);
        let (a, b) = (root.join("A"), root.join("B"));
        let ra = run_options(&root, false, &a, &only_owned);
        let rb = run_options(&root, true, &b, &only_owned);
        let ctx = format!("case {case} only {only:?} world {world:?}");
        assert_eq!(ra, rb, "{ctx}: the two sources report one run");
        assert_eq!(tree(&a), tree(&b), "{ctx}: the two stores are one store");
        let want = want_nfo(&world, &only);
        assert_eq!(totals_of(&ra), want.totals, "{ctx}: {ra:?}");
        assert_eq!(ra.seconds_committed, ra.seconds, "{ctx}");
        let entries: usize = world.iter().map(|(_, e)| e.len()).sum();
        assert_eq!(
            ra.files + ra.files_skipped + ra.files_refused,
            entries,
            "{ctx}: {ra:?}"
        );
        let mut got: Vec<(String, String)> = ra
            .failures
            .iter()
            .map(|f| (f.instrument.clone(), f.why.clone()))
            .collect();
        got.sort();
        let mut expected = want.failures.clone();
        expected.sort();
        assert_eq!(got.len(), expected.len(), "{ctx}: {got:?} vs {expected:?}");
        for ((name, why), (want_name, want_why)) in got.iter().zip(&expected) {
            assert_eq!(name, want_name, "{ctx}");
            assert!(why.contains(want_why), "{ctx}: {why} lacks {want_why:?}");
        }
        refusals += got.len();
        for ((under, segment), want_bars) in &want.series {
            let contract = Contract::parse(segment).unwrap();
            let got = held(&a, "FNO", under, Some(contract));
            if want_bars.is_empty() {
                assert!(got.is_none(), "{ctx}: {under} {segment} has no session bar");
            } else {
                assert_eq!(got.as_ref(), Some(want_bars), "{ctx}: {under} {segment}");
                bars_checked += want_bars.len();
            }
        }
        // The rerun: every clean day skipped unread, nothing written.
        let before = tree(&a);
        let again = run_options(&root, false, &a, &only_owned);
        assert_eq!(again.days_skipped, want.clean_days, "{ctx}: {again:?}");
        assert_eq!(again.seconds_committed, 0, "{ctx}");
        let after = tree(&a);
        let bars_only = |t: &BTreeMap<PathBuf, Vec<u8>>| -> BTreeMap<PathBuf, Vec<u8>> {
            t.iter()
                .filter(|(p, _)| p.starts_with("bars"))
                .map(|(p, b)| (p.clone(), b.clone()))
                .collect()
        };
        assert_eq!(bars_only(&before), bars_only(&after), "{ctx}");
        std::fs::remove_dir_all(root).unwrap();
    }
    assert!(
        bars_checked > 1_000 && refusals > 500 && filtered > 50,
        "the worlds reach every path: {bars_checked} bars, {refusals} refusals, {filtered} filtered"
    );
}

// ── the stocks world ───────────────────────────────────────────────────────

/// One entry of a capital-market stocks day.
#[derive(Clone, Debug)]
enum Stock {
    /// A swept share's file: its symbol, rows, `.CSV` spelling, whether a
    /// row is bent and whether a quantity overflows.
    File {
        sym: &'static str,
        rows: Vec<Row>,
        upper: bool,
        bent: bool,
    },
    /// A file the stocks tree holds that is not swept (`.BE.NSE`).
    Unswept { sym: &'static str },
}

const SHARES: [&str; 5] = ["RELIANCE", "SBIN", "M&M", "BAJAJ-AUTO", "360ONE"];

fn stock_bytes(on: Day, folder: &str, entry: &Stock) -> (String, Vec<u8>) {
    match entry {
        Stock::File {
            sym,
            rows,
            upper,
            bent,
        } => {
            let stem = format!("{sym}.NSE");
            let mut lines: Vec<String> = rows.iter().map(|r| line(&stem, on, r)).collect();
            if *bent {
                lines.insert(
                    0,
                    line(
                        &stem,
                        on,
                        &Row {
                            sod: OPEN,
                            ltp: 5,
                            ltp_text: "0.051".to_owned(),
                            ltq: 1,
                            oi: 0,
                        },
                    ),
                );
            }
            (
                format!("{folder}/{stem}.{}", if *upper { "CSV" } else { "csv" }),
                csv(&lines),
            )
        }
        Stock::Unswept { sym } => (
            format!("{folder}/{sym}.BE.NSE.csv"),
            csv(&[line(
                &format!("{sym}.BE.NSE"),
                on,
                &Row {
                    sod: OPEN,
                    ltp: 5,
                    ltp_text: price_text(5),
                    ltq: 1,
                    oi: 0,
                },
            )]),
        ),
    }
}

/// The capital-market rows the stock reader hands on keep no open interest:
/// the reference's rows are written with OI 0, as the files carry.
fn stock_rows(rng: &mut Rng, overflow: bool) -> Vec<Row> {
    let mut rows = random_rows(rng, overflow);
    for r in &mut rows {
        r.oi = 0;
    }
    rows
}

/// 150 random stocks worlds through both sources: reports equal, stores
/// byte-identical, totals, failures and every stored bar equal to the
/// reference, and every entry one file, skip or refusal.
#[test]
fn r2_04_random_stocks_worlds_match_the_reference_through_both_sources() {
    let mut rng = Rng(0x7232_5354_4f43_4b53);
    let mut bars_checked = 0_usize;
    for case in 0..150 {
        let span = 1 + usize::try_from(rng.below(2)).unwrap();
        let first = usize::try_from(rng.below(6)).unwrap();
        let days: Vec<Day> = APRIL[first..first + span]
            .iter()
            .map(|&d| day(2024, 4, d))
            .collect();
        let mut world: Vec<(Day, Vec<Stock>)> = Vec::new();
        for &on in &days {
            let mut entries = Vec::new();
            for _ in 0..rng.below(7) {
                entries.push(match rng.below(10) {
                    0..=6 => Stock::File {
                        sym: rng.pick(&SHARES),
                        rows: stock_rows(&mut rng, false),
                        upper: rng.below(5) == 0,
                        bent: false,
                    },
                    7 => Stock::File {
                        sym: rng.pick(&SHARES),
                        rows: stock_rows(&mut rng, true),
                        upper: false,
                        bent: false,
                    },
                    8 => Stock::File {
                        sym: rng.pick(&SHARES),
                        rows: stock_rows(&mut rng, false),
                        upper: false,
                        bent: true,
                    },
                    _ => Stock::Unswept {
                        sym: rng.pick(&SHARES),
                    },
                });
            }
            world.push((on, entries));
        }
        let root = scratch("r2-stocks");
        let mut outer: Vec<(String, Vec<u8>)> = Vec::new();
        for (on, entries) in &world {
            let folder = crate::gdfl_cm::day_folder_name(CmKind::Stocks, *on);
            let files: Vec<(String, Vec<u8>)> = entries
                .iter()
                .map(|e| stock_bytes(*on, &folder, e))
                .collect();
            let refs: Vec<(&str, &[u8])> = files
                .iter()
                .map(|(n, b)| (n.as_str(), b.as_slice()))
                .collect();
            let rel = format!(
                "STOCKS/{:04}/{}_{:04}/{folder}",
                on.year(),
                month_name(*on),
                on.year()
            );
            put(&root, &format!("ts/cm/{rel}.bts"), &bts(&refs));
            let members: Vec<(&str, &[u8], Method)> = files
                .iter()
                .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Deflated))
                .collect();
            outer.push((format!("{rel}.zip"), zip(&members)));
        }
        let members: Vec<(&str, &[u8], Method)> = outer
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
            .collect();
        put(&root, "zips/cm.zip", &zip(&members));
        let run = |store: &Path, zips: bool| {
            let run = Run {
                kind: ImportKind::Stocks,
                from: days[0],
                to: days[days.len() - 1],
                only: &[],
                store_root: store,
            };
            if zips {
                run_cm(&Archive::open(&root.join("zips/cm.zip")).unwrap(), &run).unwrap()
            } else {
                run_cm(&TickStore::new(&root.join("ts")), &run).unwrap()
            }
        };
        let (a, b) = (root.join("A"), root.join("B"));
        let (ra, rb) = (run(&a, false), run(&b, true));
        let ctx = format!("case {case} world {world:?}");
        assert_eq!(ra, rb, "{ctx}");
        assert_eq!(tree(&a), tree(&b), "{ctx}");
        // The reference: per day, each symbol's FIRST entry is the one the
        // day offers; a second entry of a symbol makes both names twins (one
        // refusal for the first, a skip for the rest, D-3174).
        let mut totals = Totals::default();
        let mut failures: Vec<(String, &str)> = Vec::new();
        let mut series: BTreeMap<&str, Vec<Bar>> = BTreeMap::new();
        for (on, entries) in &world {
            totals.days_imported += 1;
            let mut count: BTreeMap<&str, usize> = BTreeMap::new();
            for e in entries {
                if let Stock::File { sym, .. } = e {
                    *count.entry(sym).or_default() += 1;
                }
            }
            let mut seen: Vec<&str> = Vec::new();
            let mut files: Vec<(&str, Expect)> = Vec::new();
            let mut late: Vec<(String, &str)> = Vec::new();
            for e in entries {
                match e {
                    Stock::Unswept { .. } => totals.files_skipped += 1,
                    Stock::File {
                        sym, rows, bent, ..
                    } => {
                        if seen.contains(sym) {
                            totals.files_skipped += 1;
                            continue;
                        }
                        seen.push(sym);
                        let name = format!("{sym}.NSE");
                        if count[sym] > 1 {
                            totals.files_refused += 1;
                            late.push((format!("{on} {name}"), "two names for one ticker"));
                        } else if *bent {
                            totals.files_refused += 1;
                            late.push((format!("{on} {name}"), "line 2"));
                        } else if !rows
                            .iter()
                            .any(|r| r.ltq > 0 && (OPEN..CLOSE).contains(&r.sod))
                        {
                            // The stock reader's own rule (CM-05): no traded
                            // row inside the session, nothing a fill can use.
                            totals.files_refused += 1;
                            late.push((format!("{on} {name}"), "no usable row"));
                        } else if let Some(expect) = reference(*on, rows, false, false) {
                            files.push((sym, expect));
                        } else {
                            totals.files_refused += 1;
                            failures.push((format!("{on} {name}"), "does not fit an i64"));
                        }
                    }
                }
            }
            for (sym, expect) in files {
                totals.file(&expect, *on);
                series
                    .entry(sym)
                    .or_default()
                    .extend(expect.bars.into_iter().filter(|b| in_session(*on, b)));
            }
            failures.extend(late);
        }
        assert_eq!(totals_of(&ra), totals, "{ctx}: {ra:?}");
        let entries: usize = world.iter().map(|(_, e)| e.len()).sum();
        assert_eq!(
            ra.files + ra.files_skipped + ra.files_refused,
            entries,
            "{ctx}: {ra:?}"
        );
        let mut got: Vec<(String, String)> = ra
            .failures
            .iter()
            .map(|f| (f.instrument.clone(), f.why.clone()))
            .collect();
        got.sort();
        failures.sort();
        assert_eq!(got.len(), failures.len(), "{ctx}: {got:?} vs {failures:?}");
        for ((name, why), (want_name, want_why)) in got.iter().zip(&failures) {
            assert!(
                name.starts_with(want_name.as_str()),
                "{ctx}: {name} vs {want_name}"
            );
            assert!(why.contains(want_why), "{ctx}: {why} lacks {want_why:?}");
        }
        for (sym, want_bars) in &series {
            let got = held(&a, "CASH", sym, None);
            if want_bars.is_empty() {
                assert!(got.is_none(), "{ctx}: {sym}");
            } else {
                assert_eq!(got.as_ref(), Some(want_bars), "{ctx}: {sym}");
                bars_checked += want_bars.len();
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    assert!(bars_checked > 500, "{bars_checked}");
}

// ── the journal's torn-line rule (D-3173) against a foreign file ───────────

fn run_stocks_on(store: &Path) -> Result<Report, ImportRefusal> {
    let run = Run {
        kind: ImportKind::Stocks,
        from: day(2024, 4, 1),
        to: day(2024, 4, 1),
        only: &[],
        store_root: store,
    };
    drive(&run, |_, _| DayRead {
        held: true,
        ..DayRead::default()
    })
}

/// A journal that is not one is refused on EVERY run and left byte for byte
/// as it was. Before the fix the torn-line rule closed a foreign last line
/// with ` (torn)` before reading the rest, so a one-line foreign file was
/// refused once and then, its only line now "torn", read as an empty
/// journal by every later run (D-3169). A real torn tail, a prefix of a
/// journal line, is still closed once and ignored.
#[test]
fn r2_05_a_foreign_journal_is_refused_every_time_and_never_written() {
    for foreign in [
        &b"hello world"[..],
        b"done options 2024-04-01 * files=1\nhello",
        b"hello\ndone options 2024-04-01 * files=1\n",
        b"done options 2024-04-01 * files=1\nhello\nbegin st",
        b"\x00\x01",
        b"BEGIN stocks 2024-04-01 *",
        b" begin",
    ] {
        let root = scratch("r2-journal");
        put(&root, "imports/gdfl.journal", foreign);
        for attempt in 0..3 {
            let got = run_stocks_on(&root);
            assert!(
                matches!(got, Err(ImportRefusal::Journal { .. })),
                "{foreign:?}, run {attempt}: {got:?}"
            );
            assert_eq!(
                std::fs::read(journal_path(&root)).unwrap(),
                foreign,
                "{foreign:?}: a refused journal is not written"
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    for torn in [
        &b"b"[..],
        b"begin",
        b"begin ",
        b"begin stocks 2024-04-0",
        b"d",
        b"done stocks 2024-04-01 * fil",
        b"inc",
        b"incomplete stocks",
        b"done st (to",
    ] {
        let root = scratch("r2-journal");
        // `definition=` names the bar definition that built the day (D-3191).
        let mut text = b"done stocks 2024-04-01 * definition=2 files=0\n".to_vec();
        text.extend_from_slice(torn);
        put(&root, "imports/gdfl.journal", &text);
        for attempt in 0..3 {
            let got = run_stocks_on(&root);
            assert_eq!(
                got.as_ref().map(|r| r.days_skipped),
                Ok(1),
                "{torn:?}, run {attempt}: {got:?}"
            );
        }
        let after = std::fs::read(journal_path(&root)).unwrap();
        let mut want = text.clone();
        want.extend_from_slice(b" (torn)\n");
        assert_eq!(after, want, "{torn:?}: closed once");
        std::fs::remove_dir_all(root).unwrap();
    }
}

// ── the era rule and contract keying, through a real listing ───────────────

/// Pre-cutover days read through `nfo_day` from a tick store (the calendar
/// gate is `drive`'s, so the listing path runs on days the calendar has not
/// measured): monthly and dated spellings of index and share contracts,
/// both spellings of one contract on one day, names both forms read, and
/// table months on, before and after their expiry. Every entry is one file,
/// skip or refusal; a file handed on carries exactly the contract its name
/// decodes to; and two names decoding to one contract are both refused,
/// never merged and never filed as two.
#[test]
fn r2_06_pre_cutover_listings_key_every_file_by_its_decoded_contract() {
    let mut rng = Rng(0x7232_4552_4153_3031);
    let days = [
        day(2018, 9, 3),
        day(2018, 10, 1),
        day(2018, 12, 3),
        day(2018, 12, 27),
        day(2018, 12, 28),
        day(2019, 1, 15),
        day(2019, 1, 31),
        day(2019, 2, 1),
        day(2019, 2, 28),
    ];
    let unders = ["NIFTY", "BANKNIFTY", "ACC", "TV18BRDCST", "M&M"];
    let (mut handed, mut ambiguous, mut tried) = (0_usize, 0_usize, 0_usize);
    for case in 0..2_000 {
        let on = rng.pick(&days);
        let mut names: Vec<String> = Vec::new();
        for _ in 0..=rng.below(6) {
            let under = rng.pick(&unders);
            let ((year, month), (ey, em, ed)) =
                crate::gdfl_nfo::MONTHLY_EXPIRIES[usize::try_from(rng.below(15)).unwrap()];
            let expiry = day(ey, em, ed);
            let strike = rng.pick(&["195", "1260", "10500", "27000", "1927000", "50"]);
            let side = if rng.below(2) == 0 { "CE" } else { "PE" };
            let mon = MONTHS[usize::from(month) - 1];
            names.push(match rng.below(3) {
                0 => format!("{under}{:02}{mon}{strike}{side}", year % 100),
                1 => format!(
                    "{under}{:02}{mon}{:02}{strike}{side}",
                    expiry.day(),
                    year % 100
                ),
                _ => {
                    let dated = plus(on, u32::try_from(rng.below(40)).unwrap());
                    format!(
                        "{under}{:02}{}{:02}{strike}{side}",
                        dated.day(),
                        month_name(dated),
                        dated.year() % 100
                    )
                }
            });
        }
        // Sometimes the same name twice, `.csv` and `.CSV`.
        if rng.below(5) == 0 {
            let again = names[0].clone();
            names.push(again);
        }
        tried += names.len();
        let root = scratch("r2-era");
        let files: Vec<(String, Vec<u8>)> = names
            .iter()
            .enumerate()
            .map(|(k, name)| {
                let stem = format!("{name}.NFO");
                let r = Row {
                    sod: OPEN,
                    ltp: 500,
                    ltp_text: price_text(500),
                    ltq: 1,
                    oi: 0,
                };
                (
                    nfo_entry_name(on, name, k > 0 && names[..k].contains(name)),
                    csv(&[line(&stem, on, &r)]),
                )
            })
            .collect();
        let refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        put(
            &root,
            &format!(
                "ts/options/{:04}/{}_{:04}/GFDLNFO_TICK_{}.bts",
                on.year(),
                month_name(on),
                on.year(),
                ddmmyyyy(on)
            ),
            &bts(&refs),
        );
        let run = Run {
            kind: ImportKind::Options,
            from: on,
            to: on,
            only: &[],
            store_root: &root,
        };
        let mut got: Vec<TickFile> = Vec::new();
        let read = nfo_day(&NfoTickStore::new(&root.join("ts")), &run, on, &mut |f| {
            got.push(f);
        });
        let ctx = format!("case {case} on {on}: {names:?}");
        assert!(read.held, "{ctx}");
        assert_eq!(
            got.len() + read.skipped + read.refused.len(),
            names.len(),
            "{ctx}"
        );
        // The reference: each name's decoding, grouped by contract.
        let decoded: Vec<Option<(String, String)>> = names
            .iter()
            .map(|n| {
                crate::gdfl_nfo::decode_ticker(n, on).ok().map(|t| {
                    (
                        t.underlying.as_str().to_owned(),
                        t.contract.as_str().to_owned(),
                    )
                })
            })
            .collect();
        for (name, key) in names.iter().zip(&decoded) {
            let Some(key) = key else {
                continue;
            };
            let twins = decoded.iter().filter(|k| k.as_ref() == Some(key)).count();
            let sunk: Vec<&TickFile> = got.iter().filter(|f| &f.name == name).collect();
            if twins > 1 {
                ambiguous += 1;
                assert!(sunk.is_empty(), "{ctx}: {name} was filed beside its twin");
                assert!(
                    read.refused
                        .iter()
                        .any(|r| &r.instrument == name && r.why.contains("same contract")),
                    "{ctx}: {name} is not refused as ambiguous: {:?}",
                    read.refused
                );
            } else {
                assert_eq!(sunk.len(), 1, "{ctx}: {name}");
                let f = sunk[0];
                assert_eq!(
                    (
                        f.symbol.clone(),
                        f.contract.as_ref().map(|c| c.as_str().to_owned())
                    ),
                    (key.0.clone(), Some(key.1.clone())),
                    "{ctx}: {name}"
                );
                handed += 1;
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    assert!(
        handed > 500 && ambiguous > 100 && tried > 6_000,
        "{handed} handed, {ambiguous} ambiguous, {tried} names"
    );
}

/// Every table month, read as a share's monthly name on every weekday from
/// three months before its expiry to a week after it: before the cutover it
/// is the table's day exactly while the trade day is on or before it and is
/// refused by name after; from the cutover the same text is only ever read
/// as the dated form, never as the table's day unless that dated reading
/// states it.
#[test]
fn r2_07_every_table_month_on_every_weekday_around_its_expiry() {
    let cutover = {
        let (y, m, d) = crate::gdfl_nfo::DATED_FORM_FROM;
        day(y, m, d)
    };
    let mut tried = 0_usize;
    for ((year, month), (ey, em, ed)) in crate::gdfl_nfo::MONTHLY_EXPIRIES {
        let expiry = day(ey, em, ed);
        let name = format!(
            "ACC{:02}{}1280PE",
            year % 100,
            MONTHS[usize::from(month) - 1]
        );
        let want = format!("{expiry}-128000-PE");
        let mut on = Day::from_days(expiry.days_from_epoch() - 92).unwrap();
        while on <= plus(expiry, 7) {
            if weekday(on) {
                tried += 1;
                let got = crate::gdfl_nfo::decode_ticker(&name, on);
                if on < cutover {
                    if on <= expiry {
                        assert_eq!(
                            got.as_ref().map(|t| t.contract.as_str().to_owned()),
                            Ok(want.clone()),
                            "{name} on {on}"
                        );
                    } else {
                        assert_eq!(
                            got,
                            Err(NfoRefusal::ExpiryRefused {
                                ticker: name.clone()
                            }),
                            "{name} on {on}"
                        );
                    }
                } else if let Ok(t) = got {
                    // Dated: `{yy}{MON}12` is the day, month and year 2012.
                    assert!(
                        t.contract.as_str().starts_with("2012-"),
                        "{name} on {on}: {}",
                        t.contract.as_str()
                    );
                }
            }
            on = on.succ().unwrap();
        }
    }
    assert!(tried > 15 * 60, "{tried}");
}

/// The scratch directories of D-3177 are distinct under contention: 16
/// threads each make 64 with one tag, every one exists, is empty, and no two
/// share a path.
#[test]
fn r2_08_scratch_dirs_never_collide_under_contention() {
    let made: Vec<PathBuf> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..16)
            .map(|_| s.spawn(|| (0..64).map(|_| scratch("r2-race")).collect::<Vec<_>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    let unique: std::collections::BTreeSet<&PathBuf> = made.iter().collect();
    assert_eq!(unique.len(), 16 * 64);
    for dir in &made {
        assert_eq!(std::fs::read_dir(dir).unwrap().count(), 0, "{dir:?}");
        std::fs::remove_dir(dir).unwrap();
    }
}

/// The era constants and the table agree with each other: both cutover days
/// are weekdays in order, the index weeklies start before the share
/// cutover, and the table holds every month a pre-cutover share monthly can
/// name (the trade month and the two after it, from the index weeklies'
/// first month to the cutover), with nothing listed twice.
#[test]
fn r2_09_the_era_constants_and_the_table_are_one_story() {
    let (y, m, d) = crate::gdfl_nfo::DATED_FORM_FROM;
    let cutover = day(y, m, d);
    let (y, m, d) = crate::gdfl_nfo::INDEX_WEEKLY_DATED_FROM;
    let weeklies = day(y, m, d);
    assert!(weekday(cutover) && weekday(weeklies));
    assert!(weeklies < cutover);
    let listed: Vec<(u16, u8)> = crate::gdfl_nfo::MONTHLY_EXPIRIES
        .iter()
        .map(|&(month, _)| month)
        .collect();
    let unique: std::collections::BTreeSet<&(u16, u8)> = listed.iter().collect();
    assert_eq!(unique.len(), listed.len(), "a month listed twice");
    let mut on = weeklies;
    let mut needed = 0;
    while on < cutover {
        for ahead in 0_u8..3 {
            let month0 = on.month() - 1 + ahead;
            let month = (on.year() + u16::from(month0 / 12), month0 % 12 + 1);
            assert!(
                listed.contains(&month),
                "{month:?}, traded from {on}, has no day"
            );
            needed += 1;
        }
        on = on.end_of_month().succ().unwrap();
    }
    assert_eq!(
        needed, 15,
        "five trade months (2018-09 to 2019-01), three contract months each"
    );
}

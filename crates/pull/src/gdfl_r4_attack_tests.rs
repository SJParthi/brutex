#![cfg(test)]
//! Round-4 attack tests of the GDFL data path. Round 3 left two journal
//! questions open: a day an earlier run closed `incomplete` was imported
//! again and named nowhere, and a `done` line did not say which bar
//! definition built the day, so a day done before D-2812 was never rebuilt.
//! Both are settled here first; the rest of the file attacks what rounds 1
//! to 3 changed. Every value is invented at run time; no vendor row is
//! quoted. Every property draws from a fixed-seed splitmix64, so a rerun is
//! the same run (`CLAUDE.md` §3 rule 5).
//!
//! Ported from `claude/attack-gdfl` commit `bda0464f` (D-2827). The
//! attack's D-3190 to D-3195 are this branch's D-2821 to D-2826, and the
//! earlier attack numbers its comments cited are D-2812, D-2814 and D-2815.
//! An adaptation is marked `ADAPTED` with its reason; `r4_11` to `r4_14`
//! are this branch's own (marked `LOCAL`).
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail; the reference indexes on purpose"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::*;
use crate::gdfl_fixtures::{put, scratch};

// ── shared helpers ─────────────────────────────────────────────────────────

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

/// Every file under `dir` but the journal and the census, by relative path.
fn bars_tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
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
                let rel = path.strip_prefix(dir).unwrap().to_path_buf();
                if !rel.starts_with("imports") && !rel.starts_with("manifest") {
                    out.insert(rel, std::fs::read(&path).unwrap());
                }
            }
        }
    }
    out
}

fn journal_text(store: &Path) -> String {
    std::fs::read_to_string(journal_path(store)).unwrap_or_default()
}

/// The journal with every `definition=<current>` of a `done` line replaced
/// by `with` (an empty `with` drops the field, as a line written before the
/// field existed).
fn restate_journal(store: &Path, with: &str) {
    let current = format!(" definition={BAR_DEFINITION}");
    let text = journal_text(store).replace(&current, with);
    put(store, "imports/gdfl.journal", text.as_bytes());
}

// ── 1. every day the journal sends back is named ──────────────────────────

/// A day an earlier run closed `incomplete` is imported again; before the
/// fix the report named only days begun and never closed, so a retried
/// `incomplete` day was in no list (round 3's first open item, D-2821).
#[test]
fn r4_01_a_day_closed_incomplete_is_named_when_it_is_imported_again() {
    let root = scratch("r4-retried");
    let good = stock("RELIANCE", vec![tick(T, 10_000, 3), tick(T + 2, 10_100, 1)]);
    // Round one: a file the fold refuses (a volume past `i64`) beside a good
    // one, so the day is begun and closed `incomplete`.
    let bad = stock("SBIN", vec![tick(T, 500, u64::MAX), tick(T, 501, u64::MAX)]);
    let first = run_stocks(&root, d1(), d1(), &[good.clone(), bad]);
    assert_eq!(first.failures.len(), 1, "{:?}", first.failures);
    assert!(first.resumed.is_empty(), "nothing earlier to retry");
    assert!(
        journal_text(&root).contains("incomplete stocks 2024-04-01 * "),
        "{}",
        journal_text(&root)
    );
    // Round two: the day is imported again and named.
    let second = run_stocks(&root, d1(), d1(), std::slice::from_ref(&good));
    assert_eq!(second.days_imported, 1);
    assert_eq!(second.resumed, vec![d1()], "the retried day is named");
    assert!(second.restated.is_empty(), "{second:?}");
    assert!(second.failures.is_empty(), "{:?}", second.failures);
    // Round three: done now, skipped unread, named nowhere.
    let third = run_stocks(&root, d1(), d1(), &[good]);
    assert_eq!((third.days_skipped, third.days_imported), (1, 0));
    assert!(third.resumed.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

/// A crashed day (begun, never closed), an `incomplete` day and a day done
/// under another bar definition are all named in `resumed`, in day order;
/// a day done under this definition is not.
#[test]
fn r4_02_every_kind_of_retried_day_is_named_in_day_order() {
    let root = scratch("r4-retried");
    let key = |d: u8| format!("stocks 2024-04-0{d} *");
    let lines = [
        format!("begin {}", key(1)),
        format!("begin {}", key(2)),
        format!("incomplete {} files=1 seconds=0 failures=1", key(2)),
        format!("done {} files=1 seconds=0 failures=0", key(3)),
        format!(
            "done {} definition={BAR_DEFINITION} files=1 seconds=0 failures=0",
            key(4)
        ),
        format!("done {} definition=1 files=1 seconds=0 failures=0", key(5)),
    ];
    put(
        &root,
        "imports/gdfl.journal",
        format!("{}\n", lines.join("\n")).as_bytes(),
    );
    let got = run_stocks(&root, d1(), day(2024, 4, 5), &[]);
    assert_eq!(got.days_skipped, 1, "only the current definition is done");
    assert_eq!(
        got.resumed,
        vec![d1(), day(2024, 4, 2), day(2024, 4, 3), day(2024, 4, 5)]
    );
    assert_eq!(got.restated, vec![day(2024, 4, 3), day(2024, 4, 5)]);
    std::fs::remove_dir_all(root).unwrap();
}

// ── 2. a day done under another bar definition is rebuilt ─────────────────

/// A day whose `done` line names an older bar definition is imported again.
/// Its bars are the store's to judge: the same bars are `AlreadyPresent`,
/// nothing is written, and the day is closed `done` under the current
/// definition, so the next run skips it. Before the fix every such day was
/// skipped for ever (round 3's second open item, D-2822).
///
/// ADAPTED (D-2822, D-2827): the attack also dropped the field (`""`) and
/// expected a rebuild. Here a `done` line with no `definition=` whose stats
/// are D-2811's six fields is what this branch's own code wrote under
/// definition 2 before the field existed, and is read as done:
/// `r4_11_a_journal_this_branch_wrote_before_definition_existed_is_resumed_as_its_own`
/// holds that case. A line of any other shape with no field is still
/// rebuilt (`r4_02`, `r4_11`).
#[test]
fn r4_03_a_day_done_under_another_definition_is_imported_again() {
    for older in [" definition=1", " definition=0", " definition=4294967295"] {
        let root = scratch("r4-restated");
        let file = stock("RELIANCE", vec![tick(T, 10_000, 3), tick(T + 2, 10_100, 1)]);
        let first = run_stocks(&root, d1(), d1(), std::slice::from_ref(&file));
        assert!(first.failures.is_empty(), "{:?}", first.failures);
        assert!(
            journal_text(&root).contains(&format!(
                "done stocks 2024-04-01 * definition={BAR_DEFINITION} files=1 "
            )),
            "{}",
            journal_text(&root)
        );
        let held = bars_tree(&root);
        restate_journal(&root, older);
        let again = run_stocks(&root, d1(), d1(), std::slice::from_ref(&file));
        assert_eq!(
            (again.days_skipped, again.days_imported),
            (0, 1),
            "{older:?}: rebuilt"
        );
        assert_eq!(again.resumed, vec![d1()], "{older:?}");
        assert_eq!(again.restated, vec![d1()], "{older:?}");
        assert_eq!(again.seconds_committed, 0, "{older:?}: the same bars");
        assert!(again.failures.is_empty(), "{older:?}: {:?}", again.failures);
        assert_eq!(bars_tree(&root), held, "{older:?}: byte for byte");
        let third = run_stocks(&root, d1(), d1(), &[file]);
        assert_eq!(third.days_skipped, 1, "{older:?}: done under this one");
        assert!(third.restated.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// The D-2812 shape: a trade after an untraded row stamped later. A store
/// built under the old definition holds that trade at its own earlier
/// second; the current definition defers it. Rebuilding cannot write over
/// what the month holds (append-only), so every run refuses the file by
/// name, closes the day `incomplete`, names it, and leaves the bars byte for
/// byte as they were. Before the fix the day was skipped unread and the
/// look-ahead bar stood as a clean result.
#[test]
fn r4_04_a_day_whose_old_bars_differ_is_refused_by_name_never_rewritten() {
    let root = scratch("r4-restated");
    // What definition 1 built from the file below: the trade at T+3 kept at
    // T+3, because the untraded row at T+10 did not move the running maximum.
    let old = stock(
        "RELIANCE",
        vec![
            tick(T, 10_000, 2),
            tick(T + 3, 10_050, 4),
            tick(T + 20, 10_100, 1),
        ],
    );
    let first = run_stocks(&root, d1(), d1(), &[old]);
    assert!(first.failures.is_empty(), "{:?}", first.failures);
    restate_journal(&root, " definition=1");
    let held = bars_tree(&root);
    let real = stock(
        "RELIANCE",
        vec![
            tick(T, 10_000, 2),
            tick(T + 10, 10_020, 0),
            tick(T + 3, 10_050, 4),
            tick(T + 20, 10_100, 1),
        ],
    );
    for attempt in 0..3 {
        let got = run_stocks(&root, d1(), d1(), std::slice::from_ref(&real));
        assert_eq!(got.days_skipped, 0, "run {attempt}: not skipped");
        assert_eq!(got.resumed, vec![d1()], "run {attempt}");
        assert_eq!(got.seconds_committed, 0, "run {attempt}");
        assert!(
            got.failures
                .iter()
                .any(|f| f.instrument.contains("RELIANCE.invented")),
            "run {attempt}: refused by name: {:?}",
            got.failures
        );
        assert_eq!(bars_tree(&root), held, "run {attempt}: never rewritten");
        let text = journal_text(&root);
        let last = text.lines().last().unwrap();
        // ADAPTED (D-3317): the D-2822 stats are followed by the run's fields
        // and the stage that failed, so the line no longer ends at them.
        assert!(
            last.starts_with(&format!(
                "incomplete stocks 2024-04-01 * definition={BAR_DEFINITION} files=1 "
            )) && last.contains(" late=1 late_unresolved=0 max_back_s=7 ")
                && last.contains(" stage=file reason="),
            "{text}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// A `definition=` that is not a number is not a line the journal writes:
/// the run is refused and the journal left as it was.
#[test]
fn r4_05_a_done_line_whose_definition_is_not_a_number_is_foreign() {
    for bad in [
        "definition=",
        "definition=x",
        "definition=-1",
        "definition=+2",
        "definition=02",
        "definition= 2",
        "definition=4294967296",
        "definition=2 definition=2",
    ] {
        let root = scratch("r4-restated");
        let text = format!("done stocks 2024-04-01 * {bad} files=0\n");
        put(&root, "imports/gdfl.journal", text.as_bytes());
        let run = Run {
            kind: ImportKind::Stocks,
            from: d1(),
            to: d1(),
            only: &[],
            store_root: &root,
        };
        let got = drive(&run, |_, _| DayRead::default());
        assert!(
            matches!(got, Err(ImportRefusal::Journal { .. })),
            "{bad:?}: {got:?}"
        );
        assert_eq!(journal_text(&root), text, "{bad:?}: not written");
        std::fs::remove_dir_all(root).unwrap();
    }
}

// ── 3. an undecodable name's shape, with a strike that is no strike ────────

fn filtered<'a>(only: &'a [String], store: &'a Path) -> Run<'a> {
    Run {
        kind: ImportKind::Options,
        from: d1(),
        to: d1(),
        only,
        store_root: store,
    }
}

/// Which of `filters` claim `ticker` as its own undecodable file.
fn claimants(ticker: &str, filters: &[&str]) -> Vec<String> {
    let store = PathBuf::from("unused");
    let mut out = Vec::new();
    for &filter in filters {
        let only = [filter.to_owned()];
        if claims_undecodable(&filtered(&only, &store), ticker) {
            out.push(filter.to_owned());
        }
    }
    out
}

/// D-2818 made a dated or monthly shape name its underlying, but only when
/// the strike after it parsed. A strike that is no strike (three decimals, a
/// trailing zero, past `i64`, a sign) or a name past the decoder's length cap
/// left the name "shapeless", and the longest-prefix rule handed `LTI…` to
/// `LT` and `NIFTYIT…` to `NIFTY`: both filters claimed one file, the
/// round-3 defect again by another door (D-2823).
#[test]
fn r4_06_a_shape_whose_strike_is_no_strike_still_names_its_underlying() {
    let huge = "9".repeat(24);
    let long = "1".repeat(70);
    let cases: [(String, &str); 13] = [
        ("LTI06APR24100.125CE".to_owned(), "LTI"),
        ("LTI06APR24100.50CE".to_owned(), "LTI"),
        (format!("LTI06APR24{huge}PE"), "LTI"),
        ("LTI06APR24-100CE".to_owned(), "LTI"),
        ("LTI06APR24100..5CE".to_owned(), "LTI"),
        ("LTI24APR100.505CE".to_owned(), "LTI"),
        ("LTI06APR24CE".to_owned(), "LTI"),
        ("NIFTYIT06APR24100.125CE".to_owned(), "NIFTYIT"),
        (format!("NIFTYIT25APR{huge}PE"), "NIFTYIT"),
        (format!("NIFTYIT25APR24{long}CE"), "NIFTYIT"),
        (format!("LTI25APR24{long}PE"), "LTI"),
        ("LT06APR24100.125CE".to_owned(), "LT"),
        ("NIFTY06APR24100.125CE".to_owned(), "NIFTY"),
    ];
    for (name, owner) in &cases {
        assert!(decode_ticker(name, d1()).is_err(), "{name} must not decode");
        let got = claimants(name, &["LT", "LTI", "NIFTY", "NIFTYIT", "NIFTYNXT50"]);
        assert_eq!(got, vec![(*owner).to_owned()], "{name}");
    }
}

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

    fn upto(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % (n as u64)).unwrap()
    }

    fn pick<'a>(&mut self, from: &[&'a str]) -> &'a str {
        from[self.upto(from.len())]
    }
}

/// 300,000 random names over every F&O underlying and two retired ones,
/// dated and monthly, with strikes that parse and strikes that do not, on
/// trade days either side of the dated-form cutover. Every name that decodes
/// is shaped to exactly its decoded underlying, and every name built as
/// `<underlying><two digits><month><no letter>` is shaped to the underlying
/// it was built from, so exactly one of any filters naming the underlying
/// and a prefix of it claims it.
#[test]
fn r4_07_every_name_is_shaped_to_the_underlying_it_was_built_from() {
    let mut rng = Rng(0x7234_5348_4150_4531);
    let mut unders: Vec<&str> = brutex_core::universe::FNO_UNDERLYINGS.to_vec();
    unders.extend(["LTI", "NIFTYIT", "TV18BRDCST"]);
    let months = crate::gdfl_archive::MONTHS;
    let tails = ["", ".", "..5", "-1", ".125", ".50", "0100", "+5", " 5", "_"];
    let trade = [day(2018, 10, 1), day(2019, 3, 1), d1(), day(2025, 1, 6)];
    let (mut decoded, mut built) = (0_usize, 0_usize);
    for case in 0..300_000 {
        let under = rng.pick(&unders);
        let on = trade[rng.upto(trade.len())];
        // Half the time near a live contract: a day of the month, the trade
        // year, the trade month or the next; otherwise any two digits.
        let live = rng.upto(2) == 0;
        let (dd, yy) = if live {
            (1 + rng.upto(28), usize::from(on.year() % 100))
        } else {
            (rng.upto(100), rng.upto(100))
        };
        let month = if live {
            months[(usize::from(on.month()) - 1 + rng.upto(2)) % 12]
        } else {
            months[rng.upto(12)]
        };
        let mut strike = (1 + rng.upto(60_000)).to_string();
        if rng.upto(3) == 0 {
            strike.push_str(rng.pick(&tails));
        }
        if rng.upto(20) == 0 {
            strike.push_str(&"7".repeat(rng.upto(80)));
        }
        let name = if rng.upto(2) == 0 {
            format!("{under}{dd:02}{month}{yy:02}{strike}")
        } else {
            format!("{under}{yy:02}{month}{strike}")
        };
        let name = format!("{name}{}", if rng.upto(2) == 0 { "CE" } else { "PE" });
        let shaped = crate::gdfl_nfo::shaped_underlying(&name);
        if let Ok(ticker) = decode_ticker(&name, on) {
            decoded += 1;
            assert_eq!(
                shaped,
                Some(ticker.underlying.as_str()),
                "case {case}: {name}"
            );
        }
        built += 1;
        assert_eq!(shaped, Some(under), "case {case}: {name}");
    }
    assert!(decoded > 20_000 && built == 300_000, "{decoded} decoded");
}

// ── 4. a columnar block's columns, together, against its size ──────────────

fn zstd(bytes: &[u8]) -> Vec<u8> {
    ruzstd::encoding::compress_to_vec(bytes, ruzstd::encoding::CompressionLevel::Fastest)
}

/// A columnar (kind 1) block: `\n` rows, a trailing terminator, `header`,
/// `cols` as `(tag, payload, text_len)`, `rows` rows, then `tail` bytes.
fn columnar_block(header: &str, rows: u64, cols: &[(u8, Vec<u8>, u64)], tail: &[u8]) -> Vec<u8> {
    let mut out = vec![1, 0, 1];
    out.extend(u32::try_from(header.len()).unwrap().to_le_bytes());
    out.extend(header.as_bytes());
    out.extend(u32::try_from(cols.len()).unwrap().to_le_bytes());
    out.extend(rows.to_le_bytes());
    for (tag, payload, text_len) in cols {
        let frame = zstd(payload);
        out.push(*tag);
        out.extend((payload.len() as u64).to_le_bytes());
        out.extend(text_len.to_le_bytes());
        out.extend((frame.len() as u64).to_le_bytes());
        out.extend(frame);
    }
    out.extend_from_slice(tail);
    out
}

/// `block` at offset 8 of a day file, and its entry stating `size`, `rows`.
fn day_of(block: &[u8], size: u64, rows: u64) -> (Vec<u8>, crate::gdfl_tickstore::IndexEntry) {
    let mut day = b"BRTXTS01".to_vec();
    day.extend_from_slice(block);
    let entry = crate::gdfl_tickstore::IndexEntry {
        name: "GFDLCM_STOCK_TICK_01042024/RELIANCE.NSE.csv".to_owned(),
        kind: 1,
        off: 8,
        len: block.len() as u64,
        size,
        crc: 0,
        rows,
        dos_time: 0,
    };
    (day, entry)
}

/// D-2819 held each column's text to the entry's size, one at a time, and
/// the rebuilt file to it only once every column was decoded. Every row
/// holds a field of every column, so the columns' texts TOGETHER are at most
/// the size; a block of a thousand columns each one size long decoded a
/// thousand sizes before any check summed them. The block below ends in one
/// stray byte: decoded whole it is refused for that byte (`COLUMNAR`), after
/// 60 MB of columns from a 64 KB size; refused as soon as the sum passes the
/// size, it is `PAST_SIZE` at the second column (D-2824).
#[test]
fn r4_08_a_columnar_blocks_columns_together_never_pass_its_size() {
    let size = 65_536_u64;
    for (tag, cols) in [(0_u8, 1_000_usize), (1, 1_000)] {
        let header = vec!["c"; cols].join(",");
        let (rows, payload, text_len) = if tag == 0 {
            (1_u64, vec![b'x'; 60_000], 60_000)
        } else {
            // 3,000 numeric rows of seventeen bytes each
            // (`0.000000000000000`), from one shape byte apiece.
            let mut payload = vec![0x0F_u8; 3_000];
            payload.extend([0, 0]);
            (3_000, payload, 18 * 3_000 - 1)
        };
        let columns: Vec<(u8, Vec<u8>, u64)> = (0..cols)
            .map(|_| (tag, payload.clone(), text_len))
            .collect();
        let block = columnar_block(&header, rows, &columns, b"!");
        let (day, entry) = day_of(&block, size, rows);
        let got = crate::gdfl_tickstore::rebuild(&day, &entry);
        assert_eq!(
            got.as_ref().map(Vec::len),
            Err(&CmRefusal::TickStoreMalformed {
                what: "a columnar block rebuilds past its entry's stated size"
            }),
            "tag {tag}"
        );
    }
    // An honest block of many columns, whose texts sum to exactly what the
    // size leaves after the header, commas and terminators, still rebuilds.
    let cols = 64_usize;
    let header = vec!["c"; cols].join(",");
    let field = vec![b'y'; 1_000];
    let columns: Vec<(u8, Vec<u8>, u64)> = (0..cols).map(|_| (0, field.clone(), 1_000)).collect();
    let block = columnar_block(&header, 1, &columns, b"");
    let want = header.len() + 1 + cols * 1_000 + (cols - 1) + 1;
    let (day, entry) = day_of(&block, want as u64, 1);
    let got = crate::gdfl_tickstore::rebuild(&day, &entry).unwrap();
    assert_eq!(got.len(), want);
}

// ── 5. the one runtime and a second that is no second of the day ──────────

/// `convert` and `drive` are public and take a `Tick` whose `sod` is "the
/// second of the IST day": both readers hold it to `0..86_400`, the runtime
/// did not. A stamp past the day was filed at that many seconds after the
/// day's midnight, which is ANOTHER day: a row of 1 April stamped 34:00:00
/// became a bar of 2 April at 10:00, inside that day's session, and a run
/// over both days filed it there with no failure (D-2825).
#[test]
fn r4_09_a_stamp_past_the_day_is_refused_never_filed_on_the_next() {
    for sod in [86_400_u32, 86_400 + 36_000, u32::MAX] {
        for kind in ImportKind::ALL {
            let got = convert(kind, d1(), &[tick(T, 10_000, 1), tick(sod, 10_100, 1)]);
            assert!(got.is_err(), "{kind:?} {sod}: {got:?}");
            let why = got.unwrap_err().to_string();
            assert!(why.contains(&format!("second {sod} ")), "{why}");
            assert!(why.contains("not a second of the day"), "{why}");
        }
        let root = scratch("r4-stamp");
        let file = stock("RELIANCE", vec![tick(T, 10_000, 1), tick(sod, 10_100, 1)]);
        let got = run_stocks(&root, d1(), day(2024, 4, 2), &[file]);
        assert_eq!((got.seconds_committed, got.files_refused), (0, 2), "{sod}");
        assert!(bars_tree(&root).is_empty(), "{sod}: nothing filed");
        std::fs::remove_dir_all(root).unwrap();
    }
    // The last second of the day is still a second of the day.
    assert!(convert(ImportKind::Stocks, d1(), &[tick(86_399, 1, 1)]).is_ok());
}

// ── 6. a journal write that fails part way, inside one run ─────────────────

/// A journal append that fails part way (a full disk, an I/O error) leaves a
/// torn line and is a named failure, and the run goes on to its next day.
/// The next append was written straight after the torn bytes, gluing two
/// lines into one: `do` + `begin stocks 2024-04-02 *` is a foreign line the
/// next load refuses on every later run (D-2814's refusal, D-2814's harm),
/// and `done stocks 2024-04-01 * definition=2 fi` + `done stocks 2024-04-02
/// …` loads as day 1 done and loses day 2's line. Only a LOAD closed a torn
/// tail; an append inside the run did not (D-2826). Here the second day's
/// read cuts the first day's `done` line to every strict prefix, as a failed
/// write leaves it.
#[test]
fn r4_10_a_journal_line_torn_inside_a_run_is_closed_before_the_next_one() {
    let good = stock("RELIANCE", vec![tick(T, 10_000, 3), tick(T + 2, 10_100, 1)]);
    let line_len = {
        let root = scratch("r4-torn-append");
        run_stocks(&root, d1(), d1(), std::slice::from_ref(&good));
        let text = journal_text(&root);
        std::fs::remove_dir_all(root).unwrap();
        text.lines().last().unwrap().len() + 1
    };
    let mut refused = 0_usize;
    for keep in 0..line_len {
        let root = scratch("r4-torn-append");
        let path = journal_path(&root);
        let run = Run {
            kind: ImportKind::Stocks,
            from: d1(),
            to: day(2024, 4, 2),
            only: &[],
            store_root: &root,
        };
        let first = drive(&run, |on, sink| {
            if on != d1() {
                // Day 1's `done` line, as a failed write left it.
                let len = std::fs::metadata(&path).unwrap().len();
                let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
                file.set_len(len - (line_len - keep) as u64).unwrap();
            }
            sink(good.clone());
            DayRead {
                held: true,
                ..DayRead::default()
            }
        })
        .unwrap();
        assert!(first.failures.is_empty(), "{keep}: {:?}", first.failures);
        let second = drive(&run, |_, sink| {
            sink(good.clone());
            DayRead {
                held: true,
                ..DayRead::default()
            }
        });
        let Ok(second) = second else {
            refused += 1;
            continue;
        };
        assert_eq!(
            (second.days_skipped, second.days_imported, &second.resumed),
            (1, 1, &vec![d1()]),
            "keep {keep}: day 2 done, day 1 begun and never closed: {}",
            journal_text(&root)
        );
        assert_eq!(second.seconds_committed, 0, "keep {keep}");
        std::fs::remove_dir_all(root).unwrap();
    }
    assert_eq!(
        refused, 0,
        "{refused} of {line_len} torn writes stopped every later run"
    );
}

// ── 7. LOCAL: a journal this branch wrote before `definition=` existed ─────

/// The `done`/`incomplete` stats exactly as the code of D-2811 wrote them,
/// before the journal recorded `definition=` (commits 9e094a14 to 3cc6b8e0;
/// `close_day`'s format string, copied here as text).
fn d2811_stats(files: usize, seconds: usize, failures: usize, late: usize, back: u32) -> String {
    format!(
        "files={files} seconds={seconds} failures={failures} late={late} late_unresolved=0 \
         max_back_s={back}"
    )
}

/// LOCAL, not in the attack (D-2822, D-2827). An import built from 3cc6b8e0
/// writes `begin <key>`, then `done <key> <stats>` or `incomplete <key>
/// <stats>` with D-2811's six stats and no `definition=`, and closes a torn
/// tail with ` (torn)`. That journal is THIS store's own, written under
/// definition 2 (D-2811 and D-2812 landed in one commit), so a resume over it
/// skips every day it closed `done`, unread and unnamed; takes up again,
/// named in `resumed` and never in `restated`, the days it began and never
/// closed or closed `incomplete`; refuses nothing as foreign; and never
/// rewrites a byte already in the journal, only appending after it. A line
/// in the shape written before D-2811 (three stats, definition 1) is rebuilt
/// and named `restated`. Every key, count and day is invented.
#[test]
fn r4_11_a_journal_this_branch_wrote_before_definition_existed_is_resumed_as_its_own() {
    let root = scratch("r4-legacy");
    let key = |kind: &str, d: u8, only: &str| format!("{kind} 2024-04-{d:02} {only}");
    let mut lines = Vec::new();
    // Days 1 to 4 and 8: stocks done, as 3cc6b8e0 wrote them.
    for (d, files, seconds, late, back) in [
        (1, 3, 40, 0, 0),
        (2, 1, 7, 2, 5),
        (3, 2, 0, 0, 0),
        (4, 5, 1_234, 1, 1),
        (8, 1, 1, 0, 0),
    ] {
        lines.push(format!("begin {}", key("stocks", d, "*")));
        lines.push(format!(
            "done {} {}",
            key("stocks", d, "*"),
            d2811_stats(files, seconds, 0, late, back)
        ));
    }
    // A clean day with no write is closed `done` without a `begin`.
    lines.push(format!(
        "done {} {}",
        key("stocks", 9, "*"),
        d2811_stats(0, 0, 0, 0, 0)
    ));
    // Day 5 closed `incomplete`; day 10 begun and never closed (a crash).
    lines.push(format!("begin {}", key("stocks", 5, "*")));
    lines.push(format!(
        "incomplete {} {}",
        key("stocks", 5, "*"),
        d2811_stats(2, 9, 1, 0, 0)
    ));
    lines.push(format!("begin {}", key("stocks", 10, "*")));
    // Other kinds and filters under the same journal stay their own keys.
    lines.push(format!("begin {}", key("options", 1, "BANKNIFTY,NIFTY")));
    lines.push(format!(
        "done {} {}",
        key("options", 1, "BANKNIFTY,NIFTY"),
        d2811_stats(4, 12, 0, 0, 0)
    ));
    lines.push(format!(
        "done {} {}",
        key("indices", 1, "*"),
        d2811_stats(2, 9, 0, 1, 3)
    ));
    // A torn line as a crash left it and 3cc6b8e0's next load closed it.
    lines.push("done stocks 2024-04-0 (torn)".to_owned());
    let written = format!("{}\n", lines.join("\n"));
    put(&root, "imports/gdfl.journal", written.as_bytes());

    let run = Run {
        kind: ImportKind::Stocks,
        from: d1(),
        to: day(2024, 4, 10),
        only: &[],
        store_root: &root,
    };
    let good = stock("RELIANCE", vec![tick(T, 10_000, 3), tick(T + 2, 10_100, 1)]);
    let mut read = Vec::new();
    let got = drive(&run, |on, sink| {
        read.push(on);
        sink(good.clone());
        DayRead {
            held: true,
            ..DayRead::default()
        }
    })
    .unwrap();
    // Days 1, 2, 3, 4, 8 and 9 were done: skipped, never read, never named.
    assert_eq!(got.days_skipped, 6, "{got:?}");
    assert_eq!(
        read,
        vec![day(2024, 4, 5), day(2024, 4, 10)],
        "only the unfinished days are read"
    );
    assert_eq!(got.resumed, vec![day(2024, 4, 5), day(2024, 4, 10)]);
    assert!(got.restated.is_empty(), "{got:?}");
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    // Days 6 and 7 are a weekend.
    assert_eq!(got.days_closed, 2, "{got:?}");
    // Nothing already written is rewritten: the old journal is a prefix.
    let after = journal_text(&root);
    assert!(after.starts_with(&written), "{after}");
    // ADAPTED (D-3317 item 4): the run's `run` line comes first, once.
    assert!(after[written.len()..].starts_with("run "), "{after}");
    for (at, appended) in after[written.len()..].lines().enumerate() {
        assert!(
            (at == 0 && appended.starts_with("run "))
                || appended.starts_with("begin stocks 2024-04-")
                || appended.starts_with("done stocks 2024-04-")
                    && appended.contains(&format!(" definition={BAR_DEFINITION} ")),
            "{appended}"
        );
    }
    // A rerun skips every stocks day of the range and writes nothing.
    let journal = journal_text(&root);
    let bars = bars_tree(&root);
    let mut reread = Vec::new();
    let again = drive(&run, |on, _| {
        reread.push(on);
        DayRead::default()
    });
    assert!(reread.is_empty(), "{reread:?} read again");
    assert_eq!(again.unwrap().days_skipped, 8);
    assert_eq!(journal_text(&root), journal);
    assert_eq!(bars_tree(&root), bars);
    // The other keys of the old journal are done too, by their own kind and
    // filter.
    let loaded = Journal::load(journal_path(&root)).unwrap();
    // ADAPTED (D-3342, WP-11b): an options day built under definition 2 is
    // restated, its open interest built by the rule definition 3 replaced;
    // an index day is current under 2, its bars the same under both.
    let options = key("options", 1, "BANKNIFTY,NIFTY");
    assert!(loaded.restated.iter().eq([&options]) && loaded.open.is_empty());
    assert!(loaded.done.contains(&key("indices", 1, "*")) && loaded.incomplete.is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}

/// LOCAL, not in the attack (D-2822, D-2827). The shape written before
/// D-2811 (three stats, definition 1), and a D-2811 shape bent by one field,
/// name no definition: the day is rebuilt and named `restated`, and the
/// journal's earlier bytes are never rewritten.
#[test]
fn r4_12_a_done_line_of_no_known_definition_is_rebuilt_and_restated() {
    let good = stock("RELIANCE", vec![tick(T, 10_000, 3), tick(T + 2, 10_100, 1)]);
    for stats in [
        "files=1 seconds=0 failures=0",
        "files=1 seconds=0 failures=0 late=0 late_unresolved=0",
        "files=1 seconds=0 failures=0 late=0 late_unresolved=0 max_back_s=0 extra=1",
        "seconds=0 files=1 failures=0 late=0 late_unresolved=0 max_back_s=0",
        "files=1 seconds=0 failures=0 late=0 late_unresolved=0 max_back_s=",
        "files=1 seconds=0 failures=0 late=0 late_unresolved=0 max_back_s=-1",
    ] {
        let root = scratch("r4-legacy");
        let text = format!("done stocks 2024-04-01 * {stats}\n");
        put(&root, "imports/gdfl.journal", text.as_bytes());
        let got = run_stocks(&root, d1(), d1(), std::slice::from_ref(&good));
        assert_eq!((got.days_skipped, got.days_imported), (0, 1), "{stats}");
        assert_eq!(got.restated, vec![d1()], "{stats}");
        assert!(journal_text(&root).starts_with(&text), "{stats}");
        std::fs::remove_dir_all(root).unwrap();
    }
}

// ── 8. LOCAL: the edges mutation testing found unheld ─────────────────────

/// LOCAL (D-2819, D-2827). The row bound of a columnar block is its own
/// check, not only the rebuild loop's: a block stating more rows than its
/// size, with a header inside it, is refused `PAST_SIZE` before a column is
/// decoded, so a malformed numeric column behind it is never reached
/// (without the bound it refused as the column's own fault). A block of
/// exactly as many rows as its size, every field empty and no trailing
/// terminator, rebuilds to exactly its size.
#[test]
fn r4_13_a_columnar_block_is_held_to_its_size_by_its_row_count_alone() {
    let past = CmRefusal::TickStoreMalformed {
        what: "a columnar block rebuilds past its entry's stated size",
    };
    // `\n` rows, no trailing terminator.
    let block = |header: &str, rows: u64, tag: u8, payload: &[u8], text_len: u64| {
        let frame = zstd(payload);
        let mut out = vec![1, 0, 0];
        out.extend(u32::try_from(header.len()).unwrap().to_le_bytes());
        out.extend(header.as_bytes());
        out.extend(1_u32.to_le_bytes());
        out.extend(rows.to_le_bytes());
        out.push(tag);
        out.extend((payload.len() as u64).to_le_bytes());
        out.extend(text_len.to_le_bytes());
        out.extend((frame.len() as u64).to_le_bytes());
        out.extend(frame);
        out
    };
    // 100 rows stated against 50 bytes, a one-byte header, and a numeric
    // column whose two-byte payload holds no shape for any row.
    let bad = block("h", 100, 1, &[0, 0], 0);
    let (day, entry) = day_of(&bad, 50, 100);
    assert_eq!(
        crate::gdfl_tickstore::rebuild(&day, &entry),
        Err(past.clone())
    );
    // Inside the size the same column is refused as its own fault.
    let (day, entry) = day_of(&bad, 1_000, 100);
    assert_ne!(
        crate::gdfl_tickstore::rebuild(&day, &entry),
        Err(past.clone())
    );
    assert!(crate::gdfl_tickstore::rebuild(&day, &entry).is_err());
    // Exactly as many rows as bytes: 64 empty fields, one terminator each.
    let rows = 64_u64;
    let newlines = vec![b'\n'; usize::try_from(rows - 1).unwrap()];
    let exact = block("", rows, 0, &newlines, rows - 1);
    let (day, entry) = day_of(&exact, rows, rows);
    let got = crate::gdfl_tickstore::rebuild(&day, &entry).unwrap();
    assert_eq!(got, vec![b'\n'; usize::try_from(rows).unwrap()]);
}

/// LOCAL (D-2823, D-2827). A shape needs text before its two digits: a name
/// that IS the shape names no underlying, and the prefix rule decides it.
/// No side, no month in its last letters, or no two digits before them is
/// no shape either.
#[test]
fn r4_14_a_shape_with_nothing_before_it_names_no_underlying() {
    use crate::gdfl_nfo::shaped_underlying;
    assert_eq!(shaped_underlying("06APR24100CE"), None);
    assert_eq!(shaped_underlying("24APR100PE"), None);
    assert_eq!(shaped_underlying("X06APR24100CE"), Some("X"));
    assert_eq!(shaped_underlying("X24APR100PE"), Some("X"));
    assert_eq!(shaped_underlying("LTI06APR24100"), None);
    assert_eq!(shaped_underlying("LTI06XYZ24100CE"), None);
    assert_eq!(shaped_underlying("LTIA6APR24100CE"), None);
    assert_eq!(shaped_underlying("APR24CE"), None);
    assert_eq!(shaped_underlying("CE"), None);
}

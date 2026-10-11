//! `cli scrub-blocks` on owned stores of invented bars (DB-B74, DB-B65; WP-81).
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use pull::manifest::{Closes, EntryKey, Held};
use store::format::{Bar, OI_NULL};
use store::path::{Timeframe, YearMonth};

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// A run with no month bound, no rate and no commit.
const WHOLE: Bounds<'static> = Bounds {
    months: None,
    bytes_per_s: 0,
    commit: None,
};

/// At most `months` months, unthrottled.
fn at_most(months: u64) -> Bounds<'static> {
    Bounds {
        months: Some(months),
        ..WHOLE
    }
}

/// An owned, empty store root.
fn root_of(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-cli-scrub-blocks-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("manifest")).unwrap();
    root
}

#[expect(clippy::cast_possible_truncation, reason = "the writer's own fold")]
fn symbol_id() -> u32 {
    brutex_core::universe::fnv1a("NIFTY") as u32
}

/// The NIFTY index minute month `m` of 2025, as `vendor` files it.
fn month_path(vendor: Vendor, m: u8) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2025, m).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// `n` invented minute bars from day 2 of 2025-`m`.
fn rows(m: u8, n: i64) -> Vec<Bar> {
    let start = i64::from(
        pull::session::Day::new(2025, m, 2)
            .unwrap()
            .days_from_epoch(),
    ) * 86_400_000_000;
    (0..n)
        .map(|i| Bar {
            ts_micros: start + i * 60_000_000,
            open: 333_333 + i,
            high: 333_433 + i,
            low: 333_233 + i,
            close: 333_333 + i,
            volume: 1,
            open_interest: OI_NULL,
        })
        .collect()
}

/// Writes months `1..=months` of `bars` invented bars each for `vendor` and
/// counts every one in its census. With `recorded`, each landing writes its
/// provenance sidecar, as an importer's does.
fn fill(root: &Path, vendor: Vendor, months: u8, bars: i64, recorded: bool) {
    let mut held = Vec::new();
    for m in 1..=months {
        let path = month_path(vendor, m);
        let batch = rows(m, bars);
        let mut writer = BarFile::open_or_create(root, path, symbol_id()).unwrap();
        let appended = writer.append(&batch).unwrap();
        if recorded {
            let landing = store::provenance::Landing {
                source: store::provenance::Source {
                    kind: store::provenance::SourceKind::VendorEndpoint,
                    name: "/invented/endpoint",
                    bytes: 1,
                    crc32: 0,
                },
                day: store::provenance::Day::new(2025, m, 2).unwrap(),
                bar_definition: 0,
                journal_ordinal: u64::from(m),
                commit: None,
                landed_at_micros: 1_791_547_200_000_000,
            };
            store::provenance::record(root, &writer, path, appended, &landing).unwrap();
        }
        held.push(Held::new(
            Entry {
                key: EntryKey {
                    contract: None,
                    exchange: Exchange::Nse,
                    segment: Segment::Index,
                    symbol: Symbol::new("NIFTY").unwrap(),
                    timeframe: Timeframe::MINUTE_1,
                    month: YearMonth::new(2025, m).unwrap(),
                },
                rows: u64::try_from(bars).unwrap(),
                first_ts_micros: batch[0].ts_micros,
                last_ts_micros: batch[batch.len() - 1].ts_micros,
            },
            Closes::UNKNOWN,
        ));
    }
    census_store::record(root, vendor, &held).expect("the census counts them");
}

/// An owned store with `months` Zerodha NIFTY minute months of 150 invented
/// bars each (three checksum blocks), every one in the census, none with a
/// provenance sidecar — a store written before D-3334.
fn store(tag: &str, months: u8) -> PathBuf {
    let root = root_of(tag);
    fill(&root, Vendor::Zerodha, months, 150, false);
    root
}

fn bin(vendor: Vendor, m: u8) -> String {
    format!(
        "bars/{}/NSE/INDEX/NIFTY/1min/2025-{m:02}.bin",
        vendor.as_str()
    )
}

/// Flips one byte inside record `index` of Zerodha month `m`.
fn rot(root: &Path, m: u8, index: u64) {
    let bin = root.join(bin(Vendor::Zerodha, m));
    let at =
        usize::try_from(store::format::HEADER_LEN + index * store::format::RECORD_STRIDE).unwrap();
    let mut bytes = std::fs::read(&bin).unwrap();
    bytes[at + 8] ^= 0x33;
    std::fs::write(&bin, bytes).unwrap();
}

fn read(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative)).unwrap_or_default()
}

fn totals(months: u64, blocks: u64) -> Totals {
    Totals {
        months,
        blocks,
        unbound: months,
        ..Totals::default()
    }
}

/// **A PASS FINDS THE ROTTED BLOCK AND KEEPS IT (DB-B74).** Every block of
/// every census month is read; the one damaged block is a `bad` line naming
/// the month, the block and its records; the cursor ends with `done` and the
/// pass's totals; the next run starts the next pass.
#[test]
fn a_pass_reads_every_block_and_keeps_the_bad_one_in_the_ledger() {
    let root = store("pass", 3);
    rot(&root, 2, 100);
    let ran = run(
        &root,
        Vendor::Zerodha,
        Bounds {
            commit: Some(COMMIT),
            ..WHOLE
        },
    )
    .expect("ran");
    assert!(ran.finished_pass);
    assert_eq!(ran.pass, 1);
    let found = Totals {
        bad: 1,
        ..totals(3, 9)
    };
    assert_eq!(ran.this_run, found);
    assert_eq!(ran.pass_totals, found);
    let ledger = read(&root, &ledger_name(Vendor::Zerodha));
    assert_eq!(
        ledger
            .lines()
            .next()
            .unwrap()
            .split(' ')
            .skip(2)
            .collect::<Vec<_>>(),
        [
            "vendor=zerodha",
            &format!("commit={COMMIT}"),
            "pass=1",
            "resume=-"
        ],
        "{ledger}"
    );
    let bad: Vec<&str> = ledger.lines().filter(|l| l.starts_with("bad ")).collect();
    assert_eq!(bad.len(), 1, "{ledger}");
    assert!(
        bad[0].starts_with(&format!(
            "bad {} block=1 records=73-145 run=",
            bin(Vendor::Zerodha, 2)
        )) && bad[0].contains(" why="),
        "{ledger}"
    );
    let cursor = read(&root, &cursor_name(Vendor::Zerodha));
    let last = cursor.lines().last().unwrap();
    assert!(
        last.starts_with("done run=")
            && last.ends_with(
                "pass=1 months=3 blocks=9 bad=1 unverifiable=0 misfiled=0 unbound=3 busy=0 missing=0"
            ),
        "{cursor}"
    );
    assert!(cursor.ends_with('\n'));
    let again = run(&root, Vendor::Zerodha, WHOLE).expect("ran again");
    assert_eq!(again.pass, 2, "a finished pass starts the next one");
    assert_eq!(again.pass_totals, found, "and counts from nothing");
    let _ = std::fs::remove_dir_all(&root);
}

/// **A BOUNDED RUN RESUMES WHERE IT STOPPED, AND `--months 0` READS
/// NOTHING.** One month per run: the first leaves a `progress` line with its
/// cursor, the next reads only the months after it and carries the pass's
/// totals, and the last finishes the pass.
#[test]
fn a_bounded_run_resumes_after_its_cursor_with_the_pass_totals() {
    let root = store("resume", 3);
    rot(&root, 3, 10);
    let none = run(&root, Vendor::Zerodha, at_most(0)).expect("zero months");
    assert!(!none.finished_pass);
    assert_eq!(none.this_run, Totals::default());
    assert!(
        read(&root, &cursor_name(Vendor::Zerodha)).ends_with(" cursor=-\n"),
        "no month read, no cursor"
    );
    let first = run(&root, Vendor::Zerodha, at_most(1)).expect("first");
    assert!(!first.finished_pass);
    assert_eq!(first.this_run, totals(1, 3), "it began at the first month");
    let cursor = read(&root, &cursor_name(Vendor::Zerodha));
    let resume = resume_from(&cursor, false).expect("read");
    assert_eq!(resume.pass, 1);
    assert_eq!(
        resume.cursor.as_deref(),
        Some(bin(Vendor::Zerodha, 1).as_str())
    );
    assert_eq!(resume.totals, totals(1, 3));
    let second = run(&root, Vendor::Zerodha, at_most(1)).expect("second");
    assert_eq!(second.this_run, totals(1, 3));
    assert_eq!(second.pass_totals, totals(2, 6));
    let third = run(&root, Vendor::Zerodha, at_most(5)).expect("third");
    assert!(third.finished_pass);
    assert_eq!(
        third.pass_totals,
        Totals {
            bad: 1,
            ..totals(3, 9)
        }
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **ONE VENDOR'S PASS NEVER MOVES ANOTHER'S.** One shared ledger resumed a
/// Zerodha run from GDFL's cursor and its totals, and a Zerodha `done` threw
/// GDFL's unfinished pass away. Each vendor has its own two files.
#[test]
fn each_vendor_keeps_its_own_cursor_and_ledger() {
    let root = root_of("vendors");
    fill(&root, Vendor::Zerodha, 3, 150, false);
    fill(&root, Vendor::Gdfl, 2, 80, false);
    let half = run(&root, Vendor::Zerodha, at_most(2)).expect("zerodha, two of three");
    assert!(!half.finished_pass);
    let gdfl = run(&root, Vendor::Gdfl, WHOLE).expect("gdfl, whole");
    assert_eq!((gdfl.pass, gdfl.finished_pass), (1, true));
    assert_eq!(
        gdfl.pass_totals,
        totals(2, 4),
        "from nothing, its own months"
    );
    let rest = run(&root, Vendor::Zerodha, WHOLE).expect("zerodha, the rest");
    assert_eq!((rest.pass, rest.finished_pass), (1, true));
    assert_eq!(rest.this_run, totals(1, 3), "only its own third month");
    assert_eq!(rest.pass_totals, totals(3, 9));
    assert!(
        read(&root, &ledger_name(Vendor::Gdfl))
            .lines()
            .all(|line| line.contains("vendor=gdfl") || line.contains("bars/gdfl/")),
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **THE CURSOR SURVIVES ANY NUMBER OF FINDINGS.** Forty months whose `.crc`
/// is gone are forty `unverifiable` lines, more than the tail a resume reads;
/// with one file for both, the cursor was behind them and the pass restarted
/// in silence. The second run reads only the last twenty.
#[test]
fn a_resume_is_not_lost_behind_the_findings() {
    let root = root_of("findings");
    let mut held = Vec::new();
    // Forty months need forty distinct (year, month) pairs.
    for n in 0..40_u16 {
        let (year, m) = (2022 + n / 12, u8::try_from(n % 12 + 1).unwrap());
        let month = YearMonth::new(year, m).unwrap();
        let path = StorePath::new(PathParts {
            vendor: Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month,
            file: FileKind::Bars,
        })
        .unwrap();
        let start = i64::from(
            pull::session::Day::new(year, m, 2)
                .unwrap()
                .days_from_epoch(),
        ) * 86_400_000_000;
        let bar = Bar {
            ts_micros: start,
            open: 444_444,
            high: 444_544,
            low: 444_344,
            close: 444_444,
            volume: 1,
            open_interest: OI_NULL,
        };
        let mut writer = BarFile::open_or_create(&root, path, symbol_id()).unwrap();
        writer.append(&[bar]).unwrap();
        drop(writer);
        std::fs::remove_file(path.with_file(FileKind::Checksums).to_path_buf(&root)).unwrap();
        held.push(Held::new(
            Entry {
                key: EntryKey {
                    contract: None,
                    exchange: Exchange::Nse,
                    segment: Segment::Index,
                    symbol: Symbol::new("NIFTY").unwrap(),
                    timeframe: Timeframe::MINUTE_1,
                    month,
                },
                rows: 1,
                first_ts_micros: start,
                last_ts_micros: start,
            },
            Closes::UNKNOWN,
        ));
    }
    census_store::record(&root, Vendor::Zerodha, &held).unwrap();

    let first = run(&root, Vendor::Zerodha, at_most(20)).expect("first twenty");
    assert_eq!(first.this_run.unverifiable, 20);
    let ledger = read(&root, &ledger_name(Vendor::Zerodha));
    assert!(
        u64::try_from(ledger.len()).unwrap() > CURSOR_TAIL_BYTES / 2,
        "the findings are long: {} bytes",
        ledger.len()
    );
    let second = run(&root, Vendor::Zerodha, WHOLE).expect("the rest");
    assert_eq!(second.pass, 1, "the same pass, resumed");
    assert_eq!(
        second.this_run.months, 20,
        "only the months after the cursor"
    );
    assert_eq!(second.pass_totals.unverifiable, 40);
    assert!(second.finished_pass);
    let _ = std::fs::remove_dir_all(&root);
}

/// **A RUN KILLED MID-LINE IS RESUMED FROM ITS LAST WHOLE LINE; A CURSOR NO
/// LINE OF WHICH CAN BE READ IS REFUSED.** The torn tail claims months that
/// were never finished, and is not read. Garbage is not "start again": the
/// run refuses by name and reads nothing.
#[test]
fn a_torn_cursor_line_is_not_read_and_an_unreadable_cursor_is_refused() {
    let root = store("torn", 3);
    let first = run(&root, Vendor::Zerodha, at_most(1)).expect("first");
    assert_eq!(first.this_run.months, 1);
    let cursor_path = root.join(cursor_name(Vendor::Zerodha));
    let mut torn = std::fs::read_to_string(&cursor_path).unwrap();
    torn.push_str("progress run=9 pass=1 months=3 blocks=9 bad=0 unverifiable=0 misfi");
    std::fs::write(&cursor_path, &torn).unwrap();
    let second = run(&root, Vendor::Zerodha, WHOLE).expect("resumed from the whole line");
    assert_eq!(
        second.this_run.months, 2,
        "the torn line's three months are not believed"
    );
    assert!(second.finished_pass);
    assert_eq!(second.pass_totals, totals(3, 9));
    // THE RUN AFTER THAT IS THE NEXT PASS: this test stopped one run early,
    // while the `done` line above was joined onto the fragment and every
    // later run was refused "its newest line names no cursor".
    let third = run(&root, Vendor::Zerodha, WHOLE).expect("pass 2, not a refusal");
    assert_eq!((third.pass, third.finished_pass), (2, true));
    assert_eq!(third.pass_totals, totals(3, 9));

    let ledger_before = read(&root, &ledger_name(Vendor::Zerodha));
    for garbage in [
        "\u{0}\u{0}\u{0}",
        "nonsense\n",
        "progress run=1 pass=1\n",
        "done run=1 pass=0 months=1 blocks=1 bad=0 unverifiable=0 misfiled=0 unbound=0 busy=0 missing=0\n",
    ] {
        std::fs::write(&cursor_path, garbage).unwrap();
        let why = run(&root, Vendor::Zerodha, WHOLE).expect_err("refused");
        assert!(
            why.contains("where to resume cannot be told")
                && why.contains("scrub-blocks.zerodha.cursor"),
            "{garbage:?}: {why}"
        );
        assert_eq!(
            read(&root, &ledger_name(Vendor::Zerodha)),
            ledger_before,
            "nothing was read or written"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// **ONE RUN AT A TIME PER VENDOR.** A second run while one holds the cursor
/// is refused at once, by name, and reads nothing; the other vendor is free.
#[test]
fn a_second_run_of_the_same_vendor_is_refused_while_one_holds_the_cursor() {
    let root = store("two", 2);
    std::fs::create_dir_all(root.join(DIR)).unwrap();
    let held = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join(cursor_name(Vendor::Zerodha)))
        .unwrap();
    let lock = Flock::try_lock(&held, root.join(cursor_name(Vendor::Zerodha))).expect("held");
    let why = run(&root, Vendor::Zerodha, WHOLE).expect_err("refused");
    assert!(why.contains("is held by another scrub of zerodha"), "{why}");
    assert!(
        !root.join(ledger_name(Vendor::Zerodha)).exists(),
        "nothing was read"
    );
    assert!(
        run(&root, Vendor::Gdfl, WHOLE).is_ok(),
        "another vendor is free"
    );
    drop(lock);
    assert!(
        run(&root, Vendor::Zerodha, WHOLE)
            .expect("free again")
            .finished_pass
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A MONTH FILED UNDER ANOTHER NAME, A MONTH THE DISK LOST AND A MONTH WITH
/// NO SIDECAR TO BIND IT ARE EACH COUNTED AND NAMED (DB-B65).** Four months
/// with provenance sidecars: one is then copied whole over a sibling series'
/// name the census also holds, one is deleted, one loses its `.prv`.
#[test]
fn a_misfiled_month_a_missing_month_and_an_unbound_month_are_named() {
    let root = root_of("misfiled");
    fill(&root, Vendor::Zerodha, 4, 80, true);
    // Month 2's files, copied whole over month 3's name.
    for kind in [
        FileKind::Bars,
        FileKind::Checksums,
        FileKind::TimeIndex,
        FileKind::Lock,
        FileKind::Provenance,
    ] {
        let from = month_path(Vendor::Zerodha, 2)
            .with_file(kind)
            .to_path_buf(&root);
        let to = month_path(Vendor::Zerodha, 3)
            .with_file(kind)
            .to_path_buf(&root);
        if from.exists() {
            std::fs::copy(&from, &to).unwrap();
        }
    }
    std::fs::remove_file(root.join(bin(Vendor::Zerodha, 4))).unwrap();
    std::fs::remove_file(
        month_path(Vendor::Zerodha, 1)
            .with_file(FileKind::Provenance)
            .to_path_buf(&root),
    )
    .unwrap();

    let mut out = String::new();
    let at = root.clone();
    assert_eq!(
        command_with(
            &["zerodha", "--mib-per-s", "0"],
            &|| Ok(at.clone()),
            None,
            &mut out
        ),
        FAILED,
        "{out}"
    );
    assert!(
        out.contains(
            "months=4 blocks=4 bad=0 unverifiable=0 misfiled=1 unbound=1 busy=0 missing=1"
        ),
        "{out}"
    );
    let ledger = read(&root, &ledger_name(Vendor::Zerodha));
    let misfiled: Vec<&str> = ledger
        .lines()
        .filter(|l| l.starts_with("misfiled "))
        .collect();
    assert_eq!(misfiled.len(), 1, "{ledger}");
    assert!(
        misfiled[0].starts_with(&format!("misfiled {} run=", bin(Vendor::Zerodha, 3)))
            && misfiled[0].contains("its month is not this month's"),
        "{ledger}"
    );
    assert!(
        ledger
            .lines()
            .any(|l| l.starts_with(&format!("missing {} run=", bin(Vendor::Zerodha, 4)))),
        "{ledger}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The extremes: a vendor with no census is an empty, complete pass; a month
/// of one record is one block; a ledger that cannot be opened stops the run
/// before the cursor moves.
#[test]
fn an_empty_census_a_single_record_and_an_unwritable_ledger() {
    let root = root_of("extremes");
    let empty = run(&root, Vendor::Gdfl, WHOLE).expect("no census, no months");
    assert!(empty.finished_pass);
    assert_eq!(empty.pass_totals, Totals::default());

    fill(&root, Vendor::Zerodha, 1, 1, false);
    let one = run(&root, Vendor::Zerodha, WHOLE).expect("one record");
    assert_eq!(one.pass_totals, totals(1, 1));

    let blocked = root_of("extremes-blocked");
    fill(&blocked, Vendor::Zerodha, 2, 10, false);
    std::fs::create_dir_all(blocked.join(ledger_name(Vendor::Zerodha))).unwrap();
    let why = run(&blocked, Vendor::Zerodha, WHOLE).expect_err("no ledger, no run");
    assert!(why.contains("scrub-blocks.zerodha.log"), "{why}");
    assert_eq!(
        read(&blocked, &cursor_name(Vendor::Zerodha)),
        "",
        "the cursor did not move past findings that were never kept"
    );
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&blocked);
}

/// **A TAIL A KILLED RUN LEFT IS ENDED BEFORE THE NEXT LINE IS APPENDED, IN
/// THE CURSOR AND THE LEDGER, AND IS NEVER READ.**
///
/// The fragment here is the worst kind: a whole-looking `progress` line, cut
/// only of its newline, claiming 999 months, 5 bad blocks and a cursor two
/// months ahead. Before the fix the next run's line was joined onto it, so
/// every reader took the fragment's `months=`/`bad=`/`cursor=` for the pass's
/// and the run after that resumed from `…2025-04progress`: months 3 and 4
/// were never read in a pass that still ended `done`.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one killed run followed through four more"
)]
fn a_torn_tail_is_ended_before_the_next_line_and_its_words_are_never_read() {
    use std::io::Write as _;
    let root = store("torn-joined", 5);
    let cursor_at = root.join(cursor_name(Vendor::Zerodha));
    let ledger_at = root.join(ledger_name(Vendor::Zerodha));
    let none = run(&root, Vendor::Zerodha, at_most(0)).expect("zero months");
    assert!(none.torn.is_empty(), "nothing was torn yet");
    let tear = |at: &Path, fragment: &str| {
        let mut file = std::fs::OpenOptions::new().append(true).open(at).unwrap();
        file.write_all(fragment.as_bytes()).unwrap();
    };
    let claimed = format!(
        "progress run=5 pass=1 months=999 blocks=77 bad=5 unverifiable=0 misfiled=0 unbound=0 \
         busy=0 missing=0 cursor={}",
        bin(Vendor::Zerodha, 4).trim_end_matches("bin")
    );
    tear(&cursor_at, &claimed);
    let half_finding = format!("bad {} block=1 records=73-1", bin(Vendor::Zerodha, 2));
    tear(&ledger_at, &half_finding);

    // THE RUN AFTER THE KILL: it starts where the last WHOLE line says.
    let first = run(&root, Vendor::Zerodha, at_most(2)).expect("resumed");
    assert_eq!(first.this_run, totals(2, 6));
    assert_eq!(
        first.pass_totals,
        totals(2, 6),
        "999 months were never read"
    );
    assert_eq!(
        first.torn,
        vec![
            (
                cursor_name(Vendor::Zerodha),
                u64::try_from(claimed.len()).unwrap()
            ),
            (
                ledger_name(Vendor::Zerodha),
                u64::try_from(half_finding.len()).unwrap()
            ),
        ]
    );
    let cursor = read(&root, &cursor_name(Vendor::Zerodha));
    assert!(
        cursor.contains(&format!("{claimed}{VOID}\n")),
        "the fragment is ended as a void line: {cursor}"
    );
    let resume = resume_from(&cursor, false).expect("read");
    assert_eq!(
        (resume.pass, resume.cursor.as_deref(), resume.totals),
        (1, Some(bin(Vendor::Zerodha, 2).as_str()), totals(2, 6))
    );
    let ledger = read(&root, &ledger_name(Vendor::Zerodha));
    assert!(
        ledger.contains(&format!("{half_finding}{VOID}\n")),
        "{ledger}"
    );
    for (file, bytes) in &first.torn {
        assert!(
            ledger.contains(&format!("\ntorn {file} bytes={bytes} run={}\n", first.id)),
            "each ended tail is a ledger line: {ledger}"
        );
    }
    assert!(
        ledger.contains(&format!(
            "\nrun {} vendor=zerodha commit=unstamped pass=1 resume=-\n",
            first.id
        )),
        "the run line starts at a line's start: {ledger}"
    );

    // THE RUN AFTER THAT: the one that resumed from a path no run reached.
    let second = run(&root, Vendor::Zerodha, at_most(2)).expect("second");
    assert_eq!(second.this_run, totals(2, 6), "months 3 and 4 are read");
    assert_eq!(second.pass_totals, totals(4, 12));
    assert!(second.torn.is_empty());
    let ledger = read(&root, &ledger_name(Vendor::Zerodha));
    assert!(
        ledger.contains(&format!(
            "\nrun {} vendor=zerodha commit=unstamped pass=1 resume={}\n",
            second.id,
            bin(Vendor::Zerodha, 2)
        )),
        "{ledger}"
    );
    assert!(!ledger.contains("2025-04progress") && !ledger.contains("2025-04.progress"));

    // AND THE PASS ENDS WITH EVERY MONTH READ ONCE, then the next begins.
    let third = run(&root, Vendor::Zerodha, WHOLE).expect("third");
    assert!(third.finished_pass);
    assert_eq!(third.this_run, totals(1, 3));
    assert_eq!(third.pass_totals, totals(5, 15));
    // A `done` line torn of its newline claims a pass nobody finished.
    tear(
        &cursor_at,
        "done run=9 pass=7 months=1 blocks=1 bad=0 unverifiable=0 misfiled=0 unbound=0 busy=0 \
         missing=0",
    );
    let next = run(&root, Vendor::Zerodha, WHOLE).expect("the next pass, not a refusal");
    assert_eq!((next.pass, next.finished_pass), (2, true));
    assert_eq!(next.pass_totals, totals(5, 15));
    assert_eq!(next.torn.len(), 1);

    // EVERY LINE OF THE CURSOR IS ONE THIS BUILD WROTE WHOLE, OR IS VOID.
    let cursor = read(&root, &cursor_name(Vendor::Zerodha));
    assert!(cursor.ends_with('\n'));
    let (mut whole, mut void) = (0, 0);
    for line in cursor.lines() {
        if is_void(line) {
            void += 1;
            assert_eq!(cursor_line(line), None);
        } else {
            whole += 1;
            assert!(cursor_line(line).is_some(), "{line:?}");
        }
    }
    assert_eq!((whole, void), (5, 2));
    let _ = std::fs::remove_dir_all(&root);
}

/// **NO TWO RUNS OF A VENDOR SHARE AN ID.** Ids were the clock's second: two
/// runs inside one second wrote the same id, and the API's run→pass map then
/// dated the first run's findings by the second run's pass.
#[test]
fn no_two_runs_of_a_vendor_share_an_id_inside_one_second() {
    let root = store("ids", 3);
    let mut ids = Vec::new();
    for _ in 0..3 {
        ids.push(run(&root, Vendor::Zerodha, at_most(1)).expect("ran").id);
    }
    // A pass that ends and the next pass's first run: the pair that mattered.
    ids.push(run(&root, Vendor::Zerodha, WHOLE).expect("pass 2").id);
    assert!(
        ids.windows(2).all(|pair| pair.first() < pair.last()),
        "strictly later, whatever the clock said: {ids:?}"
    );
    let ledger = read(&root, &ledger_name(Vendor::Zerodha));
    let written: Vec<u64> = ledger.lines().filter_map(run_of).collect();
    assert_eq!(written, ids, "one `run` line each, in order: {ledger}");
    // A run killed before its first cursor line left its id in the ledger
    // only; the next run still takes a later one.
    std::fs::write(root.join(cursor_name(Vendor::Zerodha)), "").unwrap();
    let far = u64::MAX / 2;
    std::fs::write(
        root.join(ledger_name(Vendor::Zerodha)),
        format!(
            "run {far} vendor=zerodha commit=unstamped pass=1 resume=-\nbusy x/y.bin run={far}\n"
        ),
    )
    .unwrap();
    assert_eq!(
        run(&root, Vendor::Zerodha, at_most(0)).expect("ran").id,
        far + 1
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A CURSOR OR LEDGER THAT IS NOT A PLAIN FILE IS REFUSED BY NAME, AT
/// ONCE.** A symlink would send the lines to a file the store does not own, a
/// FIFO would take them and keep none (and an open without `O_NONBLOCK` waits
/// for a reader that never comes), a directory holds no lines.
#[test]
fn a_cursor_or_ledger_that_is_no_plain_file_is_refused_at_once() {
    let root = store("odd", 1);
    let audit = root.join(DIR);
    std::fs::create_dir_all(&audit).unwrap();
    let elsewhere = root.join("elsewhere.txt");
    std::fs::write(&elsewhere, "").unwrap();
    for name in [cursor_name(Vendor::Zerodha), ledger_name(Vendor::Zerodha)] {
        let at = root.join(&name);
        for kind in ["symlink", "fifo", "directory"] {
            match kind {
                "symlink" => std::os::unix::fs::symlink(&elsewhere, &at).unwrap(),
                "fifo" => {
                    // `mkfifo` is the external program, run from a test only.
                    let made = std::process::Command::new("mkfifo")
                        .arg(&at)
                        .status()
                        .expect("mkfifo ran");
                    assert!(made.success());
                }
                _ => std::fs::create_dir(&at).unwrap(),
            }
            let asked = Instant::now();
            let why = run(&root, Vendor::Zerodha, WHOLE).expect_err("refused");
            assert!(
                asked.elapsed() < Duration::from_secs(5),
                "{kind} at {name}: answered without waiting"
            );
            assert!(why.contains(&name), "{kind}: it names the file: {why}");
            assert_eq!(
                std::fs::read_to_string(&elsewhere).unwrap(),
                "",
                "nothing was written through the link"
            );
            if kind == "directory" {
                std::fs::remove_dir(&at).unwrap();
            } else {
                std::fs::remove_file(&at).unwrap();
            }
            // The other file of the pair is put back to nothing, so each
            // refusal is the named file's own.
            let _ = std::fs::remove_file(root.join(cursor_name(Vendor::Zerodha)));
            let _ = std::fs::remove_file(root.join(ledger_name(Vendor::Zerodha)));
        }
    }
    let ran = run(&root, Vendor::Zerodha, WHOLE).expect("plain files again");
    assert_eq!(ran.pass_totals, totals(1, 3));
    let _ = std::fs::remove_dir_all(&root);
}

/// **A CURSOR LINE IS READ WHOLE OR NOT AT ALL.** Exactly the words the
/// scrub writes, in its order. The reader this replaces took the first
/// `months=` word anywhere in the line.
#[test]
fn a_cursor_line_is_exactly_the_words_the_scrub_writes() {
    const T: &str = "months=3 blocks=6 bad=1 unverifiable=0 misfiled=0 unbound=3 busy=0 missing=2";
    let progress = cursor_line(&format!("progress run=7 pass=2 {T} cursor=a/b.bin")).unwrap();
    assert_eq!(
        progress,
        CursorLine {
            done: false,
            run: 7,
            pass: 2,
            totals: [3, 6, 1, 0, 0, 3, 0, 2],
            cursor: Some("a/b.bin".to_owned()),
        }
    );
    assert_eq!(Totals::of(progress.totals).array(), progress.totals);
    assert_eq!(
        Totals::of(progress.totals).words(),
        T,
        "the writer and the reader spell one order"
    );
    assert_eq!(
        cursor_line(&format!("progress run=7 pass=2 {T} cursor=-"))
            .unwrap()
            .cursor,
        None
    );
    let done = cursor_line(&format!("done run=8 pass=2 {T}")).unwrap();
    assert!(done.done && done.cursor.is_none() && done.run == 8);
    for refused in [
        // Two lines' words: a fragment the next line was joined onto.
        format!(
            "progress run=5 pass=1 {T} cursor=a/2019-11-28-11progress run=6 pass=1 {T} cursor=b"
        ),
        format!("progress run=5 pass=1 {T} cursor=a done run=6 pass=1 {T}"),
        format!("done run=8 pass=2 {T} cursor=a"),
        format!("done run=8 pass=2 {T} "),
        format!("progress run=7 pass=2 {T}"),
        format!("progress run=7 pass=2 {T} cursor="),
        format!("progress pass=2 run=7 {T} cursor=a"),
        format!("progress run=7 pass=0 {T} cursor=a"),
        format!("progress run=+7 pass=2 {T} cursor=a"),
        format!(
            "progress run=7 pass=2 {} cursor=a",
            T.replace("bad=1", "bad=")
        ),
        format!(
            "progress run=7 pass=2 {} cursor=a",
            T.replace(" busy=0", "")
        ),
        format!(
            "progress run=7 pass=2 {} cursor=a",
            T.replace("months=3", "months=3 months=4")
        ),
        format!("progress run=7 pass=2 {T} cursor=a{VOID}"),
        format!("run 7 vendor=gdfl {T}"),
        String::new(),
    ] {
        assert_eq!(cursor_line(&refused), None, "{refused:?}");
    }
    // The ledger's run ids, by line kind; a void line names none a reader uses.
    assert_eq!(
        run_of("run 41 vendor=gdfl commit=x pass=1 resume=-"),
        Some(41)
    );
    assert_eq!(
        run_of("bad a/b.bin block=1 records=1-2 run=42 why=said run=9"),
        Some(42)
    );
    assert_eq!(run_of("busy a/b.bin run=43"), Some(43));
    assert_eq!(run_of("nonsense"), None);
    let tail = format!("cut\nbusy a run=1\n\nbad x{VOID}\nbusy b run=2\ntorn");
    let lines: Vec<&str> = whole_lines(&tail, true).collect();
    assert_eq!(lines, ["busy a run=1", "busy b run=2"]);
}

/// A reason that happens to end in the void words is still a line a reader
/// reads: the writer never writes a line that ends in them.
#[test]
fn no_written_line_reads_as_a_void_one() {
    let root = root_of("void-line");
    let mut lines = Lines::open(&root, "audit/x.log").expect("opened");
    lines
        .line(&format!("unverifiable a/b.bin run=1 why=it said{VOID}"))
        .unwrap();
    lines.line("busy a/b.bin run=1").unwrap();
    let text = read(&root, "audit/x.log");
    assert_eq!(
        text,
        format!("unverifiable a/b.bin run=1 why=it said{VOID}.\nbusy a/b.bin run=1\n")
    );
    assert_eq!(whole_lines(&text, false).count(), 2);
    assert_eq!(
        lines.end_torn_tail().unwrap(),
        0,
        "a whole file is left alone"
    );
    assert_eq!(read(&root, "audit/x.log"), text);
    let _ = std::fs::remove_dir_all(&root);
}

/// The resume reader, line by line.
#[test]
fn the_resume_reader_reads_whole_lines_only() {
    const T: &str = "months=3 blocks=6 bad=1 unverifiable=0 misfiled=0 unbound=3 busy=0 missing=0";
    assert_eq!(resume_from("", false).unwrap().pass, 1);
    let after_done = resume_from(
        &format!("progress run=1 pass=4 {T} cursor=a\ndone run=1 pass=4 {T}\n"),
        false,
    )
    .unwrap();
    assert_eq!(
        (after_done.pass, after_done.cursor, after_done.totals),
        (5, None, Totals::default())
    );
    let mid = resume_from(
        &format!("done run=1 pass=1 {T}\nprogress run=2 pass=2 {T} cursor=x/y.bin\n"),
        false,
    )
    .unwrap();
    assert_eq!(mid.pass, 2);
    assert_eq!(mid.cursor.as_deref(), Some("x/y.bin"));
    assert_eq!(
        (mid.totals.months, mid.totals.bad, mid.totals.unbound),
        (3, 1, 3)
    );
    // A tail cut from a longer file: its first fragment is not a line.
    let cut = resume_from(
        &format!("=9 cursor=zzz\nprogress run=2 pass=2 {T} cursor=k.bin\n"),
        true,
    )
    .unwrap();
    assert_eq!(cut.cursor.as_deref(), Some("k.bin"));
    assert!(
        resume_from(&format!("progress run=2 pass=2 {T} cursor=k.bin\n"), true).is_err(),
        "a cut tail holding only its first fragment has no whole line"
    );
    // Blank lines after the newest line do not hide it.
    assert_eq!(
        resume_from(&format!("done run=1 pass=7 {T}\n\n\n"), false)
            .unwrap()
            .pass,
        8
    );
    for refused in [
        "x",
        "\n",
        "progress run=1 pass=1 months=1 cursor=a\n",
        &format!("progress run=1 pass=1 {T}\n"),
        &format!("progress run=1 pass=x {T} cursor=a\n"),
        &format!("progress run=1 pass=0 {T} cursor=a\n"),
        &format!("run 1 vendor=gdfl {T}\n"),
    ] {
        assert!(resume_from(refused, false).is_err(), "{refused:?}");
    }
}

/// The verb's refusals and exit codes.
#[test]
fn the_verb_says_what_it_did() {
    let root = store("verb", 1);
    let at = root.clone();
    let mut out = String::new();
    assert_eq!(
        command_with(&[], &|| Ok(at.clone()), None, &mut out),
        MISUSED
    );
    assert_eq!(
        command_with(&["nobody"], &|| Ok(at.clone()), None, &mut out),
        MISUSED
    );
    for bad in [
        &["zerodha", "--months"][..],
        &["zerodha", "--months", "-1"],
        &["zerodha", "--months", "many"],
        &["zerodha", "--mib-per-s", "1.5"],
        &["zerodha", "--fast", "1"],
    ] {
        assert_eq!(
            command_with(bad, &|| Ok(at.clone()), None, &mut out),
            MISUSED,
            "{bad:?}"
        );
    }
    assert!(!root.join(DIR).exists(), "a misused verb opened nothing");
    out.clear();
    assert_eq!(
        command_with(
            &["zerodha", "--mib-per-s", "0"],
            &|| Ok(at.clone()),
            None,
            &mut out
        ),
        OK,
        "{out}"
    );
    assert!(
        out.contains("pass 1")
            && out.contains("pass complete")
            && out.contains("scrub-blocks.zerodha.log")
            && out.contains("scrub-blocks.zerodha.cursor"),
        "{out}"
    );
    rot(&root, 1, 0);
    out.clear();
    assert_eq!(
        command_with(
            &["zerodha", "--mib-per-s", "0"],
            &|| Ok(at.clone()),
            None,
            &mut out
        ),
        FAILED,
        "{out}"
    );
    assert!(out.contains("pass 2") && out.contains("bad=1"), "{out}");
    out.clear();
    assert_eq!(
        command_with(
            &["zerodha"],
            &|| Err("the store root is not a directory".to_owned()),
            None,
            &mut out
        ),
        FAILED
    );
    assert!(
        out.contains("SCRUB BLOCKS REFUSED: the store root"),
        "{out}"
    );
    // A throttled run still finishes: 3 blocks at 1 MiB/s owe microseconds.
    out.clear();
    assert_eq!(
        command_with(
            &["zerodha", "--mib-per-s", "1", "--months", "1"],
            &|| Ok(at.clone()),
            None,
            &mut out
        ),
        FAILED,
        "the rot is still there: {out}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Rules audit RA-128. The rate limit sleeps with NO month open: a writer of
/// the month just read is admitted while the run is still paying its rate.
/// Before, the sleep sat inside the month's read, under its shared lock, so
/// at a low rate one month refused its writer for bytes / rate seconds.
#[test]
fn the_rate_is_slept_with_no_month_open_so_a_writer_is_not_held_for_it() {
    let root = root_of("rate-hold");
    fill(&root, Vendor::Zerodha, 1, 73, false);
    // One block at a third of a block a second: the run owes three seconds.
    let slow = Bounds {
        bytes_per_s: BLOCK_BYTES / 3,
        ..WHOLE
    };
    let scrub_root = root.clone();
    let scrub = std::thread::spawn(move || run(&scrub_root, Vendor::Zerodha, slow));
    // The run line is written just before the first month is opened; the one
    // block is read within milliseconds of it. Only then does a writer ask:
    // one that got in first would make the month `busy` to the scrub.
    let ledger = root.join(ledger_name(Vendor::Zerodha));
    while !std::fs::read_to_string(&ledger)
        .is_ok_and(|text| text.contains("\nrun ") || text.starts_with("run "))
    {
        assert!(
            !scrub.is_finished(),
            "the run ended before it wrote a run line"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let started = Instant::now();
    std::thread::sleep(Duration::from_millis(300));
    // A writer asks for the month until it is admitted or the run is over.
    let mut admitted_while_running = None;
    let mut refused_otherwise = None;
    while !scrub.is_finished() {
        match BarFile::open_or_create(&root, month_path(Vendor::Zerodha, 1), symbol_id()) {
            Ok(writer) => {
                // Admitted: the month is not held. Was the run still going?
                let still_running = !scrub.is_finished();
                drop(writer);
                if still_running {
                    admitted_while_running = Some(started.elapsed());
                }
                break;
            }
            Err(StoreError::Locked { .. }) => std::thread::sleep(Duration::from_millis(5)),
            Err(why) => {
                refused_otherwise = Some(why.to_string());
                break;
            }
        }
    }
    assert_eq!(
        refused_otherwise, None,
        "only a held month may refuse the writer"
    );
    let ran = scrub.join().unwrap().unwrap();
    let took = started.elapsed();
    // The run did pay its rate, and read the month it was asked to.
    assert!(took >= Duration::from_secs(2), "the run slept {took:?}");
    assert_eq!(
        (ran.this_run.months, ran.this_run.blocks, ran.this_run.busy),
        (1, 1, 0)
    );
    assert!(ran.finished_pass);
    let at = admitted_while_running
        .expect("a writer of the month was admitted only after the run was over");
    assert!(
        at < Duration::from_millis(1_500),
        "the month was held {at:?} of a {took:?} run"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

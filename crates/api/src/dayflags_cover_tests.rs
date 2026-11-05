//! The day band's paths no other test of `dayflags` takes (BASE-COV-API-AS3),
//! each driven by the input that reaches it and asserted on what the route
//! answers there: a request, a ledger line, a pull-journal record, a file or a
//! folder where the other should be, or a direct call with what a caller could
//! hand it.
//!
//! EVERY FIXTURE IS INVENTED. Prices are a few rupees, every symbol a pull
//! refuses is a `ZZ` one, and no row, record or line is a vendor's.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]

use std::fmt::Write as _;

use super::tests::{ask, bar, day, file_bars, flags_line, index_flags, record, root_of, write};
use super::*;
use pull::calendar::{Halt, HaltEvidence};
use store::format::Bar;

/// The instant every invented pull record is written at (2026-10-06 IST).
fn written_at() -> std::time::SystemTime {
    std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_791_266_622)
}

/// A run record whose source names its vendor by policy, as the operator's
/// runs do.
fn run_record(source: &str) -> crate::audit::Record {
    crate::audit::Record::refused(
        crate::audit::Scope::Spot,
        crate::audit::Outcome::Failed,
        written_at(),
        source,
        "fixture run",
    )
}

/// A member failure of `symbol` over an invented 2018-2026 window.
fn member_record(symbol: &str, note: &str) -> crate::audit::Record {
    crate::audit::Record::member_failure(
        crate::audit::Scope::Spot,
        written_at(),
        symbol,
        pull::session::Window::new(day("2018-01-01"), day("2026-09-30")).expect("a window"),
        note,
    )
}

/// A Zerodha run and its member failures, appended by the journal's own
/// writer, which keeps its index.
fn zerodha_run(root: &Path, members: &[(&str, &str)]) {
    let journal = crate::audit::Journal::at(root);
    journal
        .append(&run_record("zerodha_nse_checked fixture"))
        .expect("a run record");
    for (symbol, note) in members {
        journal
            .append(&member_record(symbol, note))
            .expect("a member failure");
    }
}

/// A whole-window refusal in the operator's shape, its stamp invented.
const WHOLE: &str = "conflicting vendor candles at 1578853800; refused before append";

/// A refusal that says `refused` and names no day.
const DAYLESS: &str = "rows refused for a reason naming no day";

/// `inst`'s month holding `on`, on `feed` at `rung`.
fn month_of<'a>(inst: &'a Instrument, feed: Vendor, rung: Timeframe, on: &str) -> StorePath<'a> {
    StorePath::new(PathParts {
        vendor: feed,
        exchange: inst.exchange.as_str(),
        segment: inst.segment.as_str(),
        symbol: &inst.symbol,
        contract: inst.contract,
        timeframe: rung,
        month: day(on).year_month().expect("a month"),
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

/// The walk-at-start mark put on, or taken off, `path`'s tail.
fn set_mark(path: &Path, on: bool) {
    let mut held = pull_tails().lock().unwrap_or_else(PoisonError::into_inner);
    let tail = held.get_mut(path).expect("the journal's tail");
    tail.warming = on.then(std::time::Instant::now);
}

/// SPLITCO's invented measured split, ex 2025-03-06, as the only row of the
/// store's corporate-action table.
fn install_split(store: &Path) {
    let split = pull::corporate_actions::Action {
        symbol: "SPLITCO".to_owned(),
        isin: None,
        ex_date: day("2025-03-06"),
        kind: pull::corporate_actions::Kind::Unclassed,
        ratio: pull::corporate_actions::Ratio::parse("2").expect("a ratio"),
        tag: pull::corporate_actions::Tag::Measured,
        after: Some(day("2025-03-05")),
        evidence: "an invented measured split".to_owned(),
    };
    pull::reference_data::write(
        store,
        &pull::reference_data::Draft {
            table: pull::reference_data::Table::CorporateActions,
            source: "invented ledger".to_owned(),
            source_sha256: None,
            fetched: day("2026-10-22"),
            standing: pull::reference_data::Standing::Measured,
            decision: "D-6892".to_owned(),
            rows: vec![split.fields()],
        },
    )
    .expect("the table lands");
}

/// The phrases of a day with nothing but its calendar of record: every other
/// source empty, so a test sets only what it is about.
fn says_on<'a>(record: &'a DayRecord, calendar: &'a Value) -> Says<'a> {
    Says {
        record,
        calendar,
        import: None,
        quality: &[],
        degraded: &[],
        gap: None,
        junk: None,
        pulled: &[],
        journal_present: None,
        journal_front: None,
        start: None,
        day: Day::from_days(u32::try_from(record.day).expect("a day count")).expect("a day"),
        segment: Segment::Index,
        own: Held::Bars(375),
        rung: Timeframe::MINUTE_1,
        finest: None,
        fills: None,
        halted: None,
        peer: Held::Bars(0),
        feed: Vendor::Zerodha,
        expired: None,
        sidecar: None,
        definition: None,
        index: Vec::new(),
        peer_as: None,
    }
}

/// The calendar of record's entry for `on`.
fn record_on(on: &str) -> DayRecord {
    pull::calendar::record_of(i64::from(day(on).days_from_epoch()))
}

/// 2020-03-13's halt, as a primary record would state it.
fn sourced_halt() -> DayRecord {
    let mut halted = record_on("2020-03-13");
    halted.halt = halted.halt.map(|halt| Halt {
        evidence: HaltEvidence::Sourced("an invented primary record"),
        ..halt
    });
    halted
}

/// **A CONTRACT THE STORE CANNOT NAME IS REFUSED BY NAME, AND A TAIL THAT ONLY
/// LOOKS LIKE ONE STAYS PART OF THE SYMBOL.** `contract=` that is no contract
/// segment is refused with its own text; a key whose tail has a contract's
/// shape and a byte (or a length) no contract holds is not split there, and
/// its whole rest, too long for a symbol, is refused as one; each of the
/// shape's fixed places refuses on its own.
///
/// Extremes: singles — `contract=` of letters, `contract=` holding a slash, a
/// shaped tail with a lowercase byte, a shaped tail one byte past the contract
/// capacity, five leading digits, letters for the month, no dash after the
/// month, letters for the day, no dash after the day, a bare date and its
/// dash; combinations — a shaped tail that will not parse inside a symbol
/// holding its own dash (`BAJAJ-AUTO`) at once, a refused `contract=` beside a
/// key that embeds a good contract at once; out of the box — a shaped tail of
/// 48 digits and dashes, twice the contract capacity, through the whole key.
#[test]
fn a_contract_the_store_cannot_name_is_refused_and_a_shaped_tail_must_parse() {
    // `contract=` THAT NAMES NO CONTRACT SEGMENT.
    for text in ["garbage", "2019-03-28/1150000-CE"] {
        assert_eq!(
            instrument_of("NSE-FNO-NIFTY", text).expect_err("refused"),
            format!("contract={text:?} is not a contract segment this store can name")
        );
    }
    assert_eq!(
        instrument_of("NSE-FNO-NIFTY-2019-03-28-1150000-CE", "lower-case").expect_err("refused"),
        "contract=\"lower-case\" is not a contract segment this store can name",
        "refused by its own text, whatever the key embeds"
    );
    // A SHAPED TAIL THAT WILL NOT PARSE IS NOT A CONTRACT.
    assert!(
        contract_shaped("2019-03-28-11500x0-CE"),
        "the premise: shaped"
    );
    assert_eq!(
        Contract::parse("2019-03-28-11500x0-CE"),
        None,
        "the premise"
    );
    let past = format!("2019-03-28-{}-CE", "1".repeat(34));
    assert_eq!(past.len(), 48);
    assert!(
        contract_shaped(&past),
        "the premise: shaped, past the capacity"
    );
    let wide = format!("NIFTY-{past}");
    for rest in [
        "NIFTY-2019-03-28-11500x0-CE",
        "NIFTY-2019-03-28-11500000000-CE",
        "BAJAJ-AUTO-2019-03-28-11500x0-PE",
        wide.as_str(),
    ] {
        assert_eq!(split_contract(rest), (rest, None), "{rest}");
    }
    assert_eq!(
        split_contract("BAJAJ-AUTO-2019-03-28-1150000-CE"),
        ("BAJAJ-AUTO", Contract::parse("2019-03-28-1150000-CE")),
        "the control: a tail that parses is the contract"
    );
    for text in [
        "NSE-FNO-NIFTY-2019-03-28-11500x0-CE".to_owned(),
        format!("NSE-FNO-{wide}"),
    ] {
        let symbol = text.trim_start_matches("NSE-FNO-");
        assert_eq!(
            instrument_of(&text, "").expect_err("refused"),
            format!(
                "instrument={text:?}: {symbol:?} is not a store symbol as the census spells it"
            )
        );
    }
    // EACH FIXED PLACE OF THE SHAPE.
    for (tail, place) in [
        ("12345-03-28-1150000-CE", "a dash after the year"),
        ("2019-AB-28-1150000-CE", "the month's digits"),
        ("2019-03X28-1150000-CE", "a dash after the month"),
        ("2019-03-XY-1150000-CE", "the day's digits"),
        ("2019-03-28X1150000-CE", "a dash after the day"),
        ("2019-03-28-", "a side or FUT"),
    ] {
        assert!(!contract_shaped(tail), "{place}: {tail}");
    }
    assert!(contract_shaped("2019-03-28-FUT"));
}

/// **A RECORD THAT IS NO OBJECT, OR WHOSE NOTES ARE NOT LISTS, KEEPS WHAT IT
/// HOLDS** (R4-26). A share's action on its ex-date goes into the day's
/// `notes` list, or beside a `notes` that is not one; a record that is no
/// object, or whose `notes` and `corporate_action_notes` are both not lists,
/// is left exactly as it was, and the table is answered either way.
///
/// Extremes: singles — a `null` record, a record that is an array, `notes`
/// text beside a `corporate_action_notes` number, `notes` text alone, `notes`
/// a list holding another note, a day with no action, an absent table, a page
/// of no days; combinations — a `null` record on an absent table at once,
/// `notes` text beside a `corporate_action_notes` list already holding a note
/// at once; out of the box — one day more than there are records (the extra
/// day has nothing to write into).
#[test]
fn a_record_that_cannot_hold_a_note_keeps_what_it_holds() {
    let store = root_of("cover-actions");
    let on = day("2025-03-06");
    // NO TABLE: refused by name, and nothing written.
    let mut untouched = vec![Value::Null];
    let said = corporate_action_notes(&store, "SPLITCO", &[on], &mut untouched);
    assert_eq!(said["served"], false, "{said:#}");
    assert_eq!(said["refusal"]["kind"], "absent");
    assert_eq!(untouched, [Value::Null]);
    install_split(&store);
    let mut records = vec![
        Value::Null,
        json!(["an array, not a record"]),
        json!({ "notes": "kept as text", "corporate_action_notes": 7 }),
        json!({ "notes": "kept as text" }),
        json!({ "notes": [{ "note": "other" }] }),
        json!({ "notes": "kept as text", "corporate_action_notes": [{ "note": "earlier" }] }),
        json!({ "day": "2025-03-07" }),
    ];
    let before = records.clone();
    let days = [on, on, on, on, on, on, day("2025-03-07"), on];
    let said = corporate_action_notes(&store, "SPLITCO", &days, &mut records);
    assert_eq!(said["served"], true, "{said:#}");
    // NO OBJECT, OR NO LIST TO WRITE INTO: as it was.
    assert_eq!(records[..3], before[..3]);
    // `notes` TEXT ALONE: the note goes beside it, the text kept.
    assert_eq!(records[3]["notes"], "kept as text");
    let beside = records[3]["corporate_action_notes"]
        .as_array()
        .expect("a list beside");
    assert_eq!(beside.len(), 1);
    assert_eq!(beside[0]["note"], "corporate_action");
    assert_eq!(beside[0]["ratio"], "2/1");
    // `notes` A LIST: the action after the note it held.
    let notes = records[4]["notes"].as_array().expect("the list");
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0]["note"], "other");
    assert_eq!(notes[1]["note"], "corporate_action");
    // BESIDE A LIST ALREADY THERE: appended to it.
    let beside = records[5]["corporate_action_notes"]
        .as_array()
        .expect("the list beside");
    assert_eq!(beside.len(), 2);
    assert_eq!(beside[0]["note"], "earlier");
    assert_eq!(beside[1]["note"], "corporate_action");
    // A DAY WITH NO ACTION: untouched.
    assert_eq!(records[6], before[6]);
    // NO DAYS: nothing to write, the table still answered.
    assert_eq!(
        corporate_action_notes(&store, "SPLITCO", &[], &mut [])["served"],
        true
    );
}

/// **THE LINEAGE LEDGER IS READ AGAIN WHEN IT CHANGES, AND ONE THAT WILL NOT
/// READ IS SAID, NEVER AN EMPTY LIST** (D-3331). An `alias` line is the
/// decoder's and never counted unreadable; a ledger rewritten under its path
/// is indexed again; a folder, or bytes that are not text, at the ledger's
/// path are named in `why`.
///
/// Extremes: singles — no ledger, a ledger of `alias` lines only, a blank
/// line, a line no record reads, a ledger that grows, a ledger cut back, a
/// folder at the path, a Zerodha page, an index (no lineage at all);
/// combinations — `alias` lines and an unreadable line in one ledger at
/// once, a folder at the path asked twice (no failure is kept as an index) at
/// once; out of the box — bytes of another format (not UTF-8) at the path.
#[test]
fn the_lineage_ledger_is_read_again_when_it_changes_and_a_refused_one_is_said() {
    let contract = instrument_of("NSE-FNO-NIFTY-2019-03-28-1150000-CE", "").expect("a key");
    let root = root_of("cover-lineage");
    let path = pull::gdfl_lineage::lineage_path(&root);
    let read = || lineage_json(&root, Vendor::Gdfl, &contract);
    assert_eq!(read()["present"], false);
    // `alias` LINES ARE THE DECODER'S: read, never unreadable.
    let aliases = "alias NIFTY invented one\n\nalias NIFTY invented two\n";
    write(&path, aliases);
    let index = index_lineage(aliases);
    assert_eq!(index.unreadable, 0);
    assert!(index.by_key.is_empty());
    assert!(index.events.is_empty());
    let got = read();
    assert_eq!(got["present"], true, "{got:#}");
    assert_eq!(got["unreadable_lines"], 0);
    assert_eq!(got["records"], json!([]));
    // REWRITTEN UNDER ITS PATH: read again, by its new stamp.
    write(
        &path,
        "alias NIFTY invented one\nnot a lineage line\nalias NIFTY invented two\nnor this\n",
    );
    assert_eq!(read()["unreadable_lines"], 2);
    write(&path, "alias NIFTY x\n");
    assert_eq!(read()["unreadable_lines"], 0, "cut back, and read again");
    // A FOLDER AT THE LEDGER'S PATH: named, never an empty list, each time.
    let folder = root_of("cover-lineage-folder");
    let at = pull::gdfl_lineage::lineage_path(&folder);
    std::fs::create_dir_all(at.join("inside")).expect("a folder where the ledger goes");
    for _ in 0..2 {
        let got = lineage_json(&folder, Vendor::Gdfl, &contract);
        assert_eq!(got["present"], true, "{got:#}");
        assert_eq!(got["records"], json!([]));
        let why = got["why"].as_str().expect("why");
        assert!(
            why.starts_with(&format!("cannot read {}", at.display())),
            "{why}"
        );
    }
    // BYTES OF ANOTHER FORMAT.
    let bytes = root_of("cover-lineage-bytes");
    let at = pull::gdfl_lineage::lineage_path(&bytes);
    std::fs::create_dir_all(at.parent().expect("a parent")).expect("mkdir");
    std::fs::write(&at, [0xFF, 0xFE, b'\n', 0x80]).expect("bytes");
    let got = lineage_json(&bytes, Vendor::Gdfl, &contract);
    let why = got["why"].as_str().expect("why");
    assert!(why.contains("valid UTF-8"), "{got:#}");
    // NO LINEAGE OFF A GDFL CONTRACT.
    assert_eq!(lineage_json(&root, Vendor::Zerodha, &contract), Value::Null);
    let index = instrument_of("NSE-INDEX-NIFTY", "").expect("a key");
    assert_eq!(lineage_json(&root, Vendor::Gdfl, &index), Value::Null);
}

/// **A JOURNAL OF MORE ROWS THAN ONE `/imports.json` PAGE IS READ TO ITS LAST
/// PAGE, AND ONE RUN THAT CLOSED A DAY UNDER FOUR FILTERS IS ONE CLOSER**
/// (AF-02). The 65 weekdays of 2024's first quarter, each closed by one run
/// under `*`, another underlying's list, a list naming NIFTY and NIFTY alone:
/// 260 rows, two pages of 200. Every day reads the newest close that names
/// NIFTY, on either page; a weekend reads no line, reached.
///
/// Extremes: singles — 260 rows (past one page), a day on the second page, a
/// day on the first page, a day closed four times by one run, a filter
/// naming another underlying only, a `*` close, a list naming NIFTY, a
/// weekend with no line, the page's last day; combinations — a day on the
/// second page closed four times by its one run at once, a weekend between
/// two pages' days at once; out of the box — a torn last line (no newline:
/// a line being written) that would have changed the last day.
#[test]
fn a_journal_past_one_page_is_read_to_its_last_page() {
    let root = root_of("cover-journal-pages");
    let mut text = String::from(
        "run 1704067200 source=zip archive=/tmp/a.zip archive_bytes=7 commit=abc \
         started=1704067200 archive_cd_blake3=00\n",
    );
    let weekdays: Vec<Day> = days_between(day("2024-01-01"), day("2024-03-31"))
        .into_iter()
        .filter(|on| !pull::calendar::weekday_of(i64::from(on.days_from_epoch())).is_weekend())
        .collect();
    assert_eq!(weekdays.len(), 65, "the premise");
    for on in &weekdays {
        for (filter, files) in [
            ("*", 1),
            ("NIFTYNXT50,BANKNIFTY", 4),
            ("BANKNIFTY,NIFTY", 2),
            ("NIFTY", 3),
        ] {
            let _ = writeln!(text, "begin indices {on} {filter}");
            let _ = writeln!(
                text,
                "done indices {on} {filter} files={files} seconds=3 failures=0 late=0 \
                 late_unresolved=0 max_back_s=0 run=1704067200"
            );
        }
    }
    text.push_str("done indices 2024-03-29 NIFTY files=9");
    write(&pull::gdfl_import::journal_path(&root), &text);
    let got = answer(
        &root,
        &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-01-01&to=2024-03-31"),
    );
    assert_eq!(got["days"].as_array().map(Vec::len), Some(91));
    for on in ["2024-01-01", "2024-02-15", "2024-03-11", "2024-03-29"] {
        let import = &record(&got, on)["import"];
        assert_eq!(import["filter"], "NIFTY", "{on}: {import:#}");
        assert_eq!(import["files"], 3, "{on}: {import:#}");
        assert_eq!(import["run"], 1_704_067_200_u64);
        assert_eq!(import["outcome"], "done");
    }
    let weekend = &record(&got, "2024-03-09")["import"];
    assert_eq!(weekend["outcome"], "none", "{weekend:#}");
    assert_eq!(weekend["reached"], true);
}

/// **EACH LOGGED REFUSAL IS PLACED BY ITS TIME, COUNTED ONCE, AND NAMED UP TO
/// THE BOUND** (DB-B45, AF-02 repair 2). A refusal logged after the next
/// closing run started is that run's; one logged before the close's run is
/// history, under the run then running; the rest are the close's own, this
/// instrument's or another file's. A refusal naming no file is about every
/// instrument, and a share is matched in the stocks tree.
///
/// Extremes: singles — a refusal with an empty `about` (day-level), a share's
/// own file, another share's file, a share's series file, an index file on a
/// share's page, the same later refusal twice, two later refusals about this
/// instrument, a later one about another file, five distinct earlier
/// refusals (past the names kept), one logged before any run line, five
/// current ones about this instrument; combinations — a later refusal that is
/// also a repeat at once, an earlier refusal before every run line and past
/// the bound at once; out of the box — a logged entry with no time at all
/// (nothing places it outside the close's own window).
#[test]
fn each_refusal_is_placed_by_its_time_counted_once_and_named_to_the_bound() {
    let on = day("2024-03-04");
    let nifty = instrument_of("NSE-INDEX-NIFTY", "").expect("a key");
    let share = instrument_of("NSE-CASH-RELIANCE", "").expect("a key");
    // A DAY-LEVEL REFUSAL NAMES NO FILE: it is about every instrument.
    assert!(about_this("", &nifty, on));
    assert!(about_this("", &share, on));
    // A SHARE READS THE STOCKS TREE: its own file, never another's.
    assert!(about_this("RELIANCE.NSE.csv", &share, on));
    assert!(!about_this("TCS.NSE.csv", &share, on));
    assert!(
        !about_this("RELIANCE.BE.NSE.csv", &share, on),
        "a series file"
    );
    assert!(
        !about_this("NIFTY 50.NSE_IDX.csv", &share, on),
        "an index file"
    );
    let logged =
        |about: &str, why: &str, at_ms: u64| json!({ "about": about, "why": why, "at_ms": at_ms });
    let own = "NIFTY 50.NSE_IDX.csv";
    let bank = "NIFTY BANK.NSE_IDX.csv";
    let mut entries = vec![
        logged(own, "later one", 3_000_000),
        logged(own, "later one", 3_100_000),
        logged(own, "later two", 3_200_000),
        logged(bank, "later three", 3_300_000),
        logged("calendar", "before any run", 500_000),
    ];
    for n in 1..=5_u64 {
        entries.push(logged("calendar", &format!("earlier {n}"), 1_500_000 + n));
    }
    for n in 1..=5_u64 {
        entries.push(logged(own, &format!("current {n}"), 2_500_000 + n));
    }
    entries.push(logged(bank, "another file", 2_600_000));
    entries.push(json!({ "about": "", "why": "no time at all" }));
    let row = json!({ "run": 2_000, "logged": entries });
    let runs = [
        RunLine {
            id: 1_000,
            started: 1_000,
            commit: Some("aaa".to_owned()),
        },
        RunLine {
            id: 2_000,
            started: 2_000,
            commit: None,
        },
    ];
    let got = refusals_of(&row, &nifty, on, &runs, Some(3_000));
    assert_eq!(got.later_count, 3, "{got:?}");
    assert_eq!(got.later_this_count, 2);
    assert_eq!(
        got.later_this_first,
        Some((own.to_owned(), "later one".to_owned()))
    );
    assert_eq!(got.superseded_count, 6);
    assert_eq!(got.superseded.len(), MAX_REFUSALS_NAMED);
    assert_eq!(got.superseded[0].run, None, "logged before any run line");
    assert_eq!(got.superseded[1].run, Some(1_000));
    assert_eq!(got.superseded[1].commit.as_deref(), Some("aaa"));
    assert_eq!(got.this_count, 6, "five current, and the one with no time");
    assert_eq!(got.this.len(), MAX_REFUSALS_NAMED);
    assert_eq!(got.this[0], (own.to_owned(), "current 1".to_owned()));
    assert_eq!(got.other_count, 1);
    assert_eq!(
        got.other_first,
        Some((bank.to_owned(), "another file".to_owned()))
    );
}

/// **A LEDGER IS READ THROUGH ITS TAIL WHATEVER IS DONE TO IT**: a fragment
/// with no newline is no line until its newline lands; a ledger cut back in
/// place is read again from its start; blank lines are no lines and no
/// faults; a ledger under a path whose folder is a file is refused by name.
///
/// Extremes: singles — an absent ledger, a ledger of one fragment, the
/// fragment's newline landing, a read with nothing new, a ledger that grows
/// by a line this reader refuses, a ledger cut back in place, a blank line in
/// the gap ledger, a blank line in the day ledger; combinations — a ledger cut
/// back to blank lines around one line at once, then cut back to a fragment
/// (nothing whole left) at once; out of the box — a file standing where the
/// ledgers' folder goes.
#[test]
fn a_ledger_is_read_through_its_tail_whatever_is_done_to_it() {
    let root = root_of("cover-ledger-tail");
    let gaps = gaps_path(&root);
    let line = |on: &str| {
        format!(
            "gap NSE-INDEX-ZZOIL {on} v=1 rung=1min owed=375 held=370 missing=5 \
             first_missing=10:00 runs=1 longest=5 longest_at=10:00 why=vendor_empty run=7"
        )
    };
    let read = || read_ledger(gap_tails(), &gaps).expect("a ledger that reads");
    assert!(!read().present);
    // ONE FRAGMENT: no line yet.
    write(&gaps, &line("2024-01-02"));
    let torn = read();
    assert_eq!((torn.lines, torn.unterminated), (0, torn.bytes));
    assert!(torn.parsed.gaps.is_empty());
    // ITS NEWLINE LANDS: one line.
    super::tests::append(&gaps, "\n");
    let whole = read();
    assert_eq!((whole.lines, whole.unterminated), (1, 0));
    assert!(
        whole
            .parsed
            .get("NSE-INDEX-ZZOIL", day("2024-01-02"))
            .is_some()
    );
    assert_eq!(read().read_now, 0, "nothing new");
    // IT GROWS, by two lines and one this reader refuses.
    let grown = format!(
        "{}\n{}\n{}\n",
        line("2024-01-03"),
        line("2024-01-04"),
        line("2024-01-05").replace("v=1", "v=2")
    );
    super::tests::append(&gaps, &grown);
    let more = read();
    assert_eq!(more.lines, 4);
    assert_eq!(more.parsed.gaps.len(), 3);
    assert_eq!(more.parsed.faults.count, 1);
    // CUT BACK IN PLACE: read again from its start; blank lines are nothing.
    write(&gaps, &format!("\n{}\n\n", line("2024-01-08")));
    let cut = read();
    assert_eq!(cut.lines, 3);
    assert_eq!(cut.parsed.gaps.len(), 1);
    assert_eq!(cut.parsed.faults.count, 0);
    assert!(
        cut.parsed
            .get("NSE-INDEX-ZZOIL", day("2024-01-08"))
            .is_some()
    );
    write(&gaps, "gap NSE");
    let fragment = read();
    assert_eq!((fragment.lines, fragment.unterminated), (0, 7));
    assert!(fragment.parsed.gaps.is_empty());
    // THE DAY LEDGER'S BLANK LINE.
    write(
        &quality_path(&root),
        &format!(
            "\n{}",
            index_flags("2024-01-02", "NIFTY", &[("rows", "90")])
        ),
    );
    let quality = read_ledger(quality_tails(), &quality_path(&root)).expect("a day ledger");
    assert_eq!(quality.lines, 2);
    assert_eq!(quality.parsed.rows.len(), 1);
    assert_eq!(quality.parsed.faults.count, 0);
    // A FILE WHERE THE LEDGERS' FOLDER GOES.
    let file_root = root_of("cover-ledger-under-a-file");
    write(
        &file_root.join("audit"),
        "a file where the audit folder goes",
    );
    let under = gaps_path(&file_root);
    let why = read_ledger(gap_tails(), &under)
        .err()
        .expect("refused, not absent");
    assert!(why.starts_with(&format!("{}: ", under.display())), "{why}");
    assert!(why.contains("Not a directory"), "{why}");
}

/// **A LEDGER LINE THIS READER CANNOT NAME IS REFUSED WITH ITS REASON**, and
/// its faults are named to a bound and counted past it.
///
/// Extremes: singles — a one-digit hour, a minute with one digit, hour 24, a
/// seventeenth fault (past the names kept), an index line with no scope, a gap
/// line naming a field twice, a gap line with no key, a gap line of version
/// 2, a gap line with an empty `why`, a derive line with no key, a derive
/// line of version 3; combinations — a gap line of version 2 naming `v` twice
/// at once (the repeat is refused first), a fault text past 80 characters and
/// past the names kept at once; out of the box — an empty line handed to each
/// ledger's fold (no line, no fault).
#[test]
fn a_ledger_line_this_reader_cannot_name_is_refused_with_its_reason() {
    assert_eq!(clock("1:00:00", true), Err("not HH:MM:SS or -"));
    assert_eq!(clock("10:0", false), Err("not HH:MM or -"));
    assert_eq!(clock("24:00:00", true), Err("not HH:MM:SS or -"));
    assert_eq!(clock("09:15:30", true), Ok(Some(33_330)));
    assert_eq!(clock("-", false), Ok(None));
    // SIXTEEN NAMED, EVERY ONE COUNTED.
    let mut faults = Faults::default();
    let long = "z".repeat(100);
    for number in 1..=17 {
        faults.note(number, &long, "invented");
    }
    assert_eq!(faults.count, 17);
    assert_eq!(faults.named.len(), MAX_NAMED_FAULTS);
    assert_eq!(faults.named[15].1.len(), 80);
    assert_eq!(faults.json()["lines"].as_array().map(Vec::len), Some(16));
    // AN INDEX LINE'S SCOPE IS AN INDEX SYMBOL.
    assert_eq!(
        parse_quality(1, "flags indices 2024-01-02  v=1").expect_err("refused"),
        "scope \"\" is not an index symbol for indices or `*` for stocks and options"
    );
    // THE GAP LINE.
    let rest = "rung=1min owed=375 held=370 missing=5 first_missing=10:00 runs=1 longest=5 \
                longest_at=10:00 run=7";
    let gap = |fields: &str| format!("gap NSE-INDEX-ZZOIL 2024-01-02 {fields} {rest}");
    for (line, why) in [
        (gap("v=1 v=1 why=vendor_empty"), "v appears twice"),
        (gap("v=2 v=2 why=vendor_empty"), "v appears twice"),
        (
            format!("gap  2024-01-02 v=1 why=vendor_empty {rest}"),
            "no instrument key",
        ),
        (
            gap("v=2 why=vendor_empty"),
            "v=2 is not 1, the version this reader reads",
        ),
        (gap("v=1 why="), "why=\"\" is not a snake-case code"),
    ] {
        assert_eq!(parse_gap(1, &line).expect_err("refused"), why, "{line}");
    }
    // THE DERIVE LINE.
    let derived = "days=20 buckets=20 withheld=0 partial=0 run=7";
    assert_eq!(
        parse_derive(1, &format!("derive  1min 2024-01 v=1 {derived}")).expect_err("refused"),
        "no instrument key"
    );
    assert_eq!(
        parse_derive(
            1,
            &format!("derive NSE-INDEX-ZZOIL 1min 2024-01 v=3 {derived}")
        )
        .expect_err("refused"),
        "v=3 is not 1, the version this reader reads"
    );
    assert!(
        parse_derive(
            1,
            &format!("derive NSE-INDEX-ZZOIL 1min 2024-01 v=1 {derived}")
        )
        .is_ok()
    );
    // AN EMPTY LINE IS NO LINE AND NO FAULT, IN EITHER LEDGER.
    let mut quality = QualityLedger::default();
    quality.line(1, "");
    assert!(quality.rows.is_empty());
    assert_eq!(quality.faults.count, 0);
    let mut gaps = GapLedger::default();
    gaps.line(1, "");
    assert!(gaps.gaps.is_empty());
    assert_eq!(gaps.faults.count, 0);
    // A REFUSED LINE THROUGH THE FOLD IS A FAULT WITH ITS REASON.
    gaps.line(2, &gap("v=2 why=vendor_empty"));
    assert_eq!(gaps.faults.named[0].0, 2);
    assert_eq!(
        gaps.faults.named[0].2,
        "v=2 is not 1, the version this reader reads"
    );
}

/// **A PULL RECORD IS KEPT ONLY WHERE IT NAMES A REFUSAL, AND A RECORD THIS
/// READER CANNOT READ IS A FAULT, NEVER A REFUSAL** (D-3315, D-3344).
///
/// Extremes: singles — a note naming no refusal, a note that refuses and names
/// no day, a whole-window refusal, an image shorter than the note bytes, a
/// kind byte no version writes, a run whose bytes changed, a member whose
/// bytes changed, a Zerodha member naming no day, a member of a run that
/// names no vendor, a window's day before its first; combinations — a damaged
/// run followed by its member (counted apart, never Zerodha's) at once, a cut
/// note shown with its mark and judged at a window's two ends at once; out of
/// the box — 255 bytes handed in as a record, one short.
#[test]
fn a_pull_record_is_kept_only_where_it_names_a_refusal() {
    // THE NOTE DECIDES.
    let quiet = member_record("ZZOIL", "request minute coverage UNVERIFIED on 2018-11-07");
    assert_eq!(pull_refusal(1, &quiet), None);
    assert_eq!(pull_refusal(2, &member_record("ZZOIL", DAYLESS)), None);
    let whole = pull_refusal(3, &member_record("ZZOIL", WHOLE)).expect("a whole-window refusal");
    assert!(whole.whole_window);
    // SHOWN AS JOURNALLED, AND MARKED WHERE CUT; ABOUT ITS WINDOW'S DAYS ONLY.
    let (from, to) = (
        day("2020-01-10").days_from_epoch(),
        day("2020-01-20").days_from_epoch(),
    );
    let window = PullRefusal {
        record: 9,
        at_unix_secs: 1_791_266_622,
        from_days: from,
        to_days: to,
        note: "an invented note".to_owned(),
        cut: false,
        names_day: None,
        whole_window: true,
    };
    assert_eq!(window.shown(), "an invented note");
    assert!(!window.about(from - 1));
    assert!(window.about(from));
    assert!(window.about(to));
    assert!(!window.about(to + 1));
    let marked = PullRefusal {
        cut: true,
        ..window.clone()
    };
    assert_eq!(marked.shown(), "an invented note...");
    assert!(!may_refuse(&[0_u8; 100]), "shorter than the note bytes");
    assert!(may_refuse(&member_record("ZZOIL", WHOLE).image()));
    let mut parsed = PullRefusals::default();
    let mut vendor = Some(Vendor::Zerodha);
    // ONE BYTE SHORT OF A RECORD: not a record, nothing noted.
    pull_record(
        &mut parsed,
        &mut vendor,
        1,
        &[0_u8; crate::audit::RECORD_LEN - 1],
    );
    assert_eq!(parsed.faults.count, 0);
    assert_eq!(vendor, Some(Vendor::Zerodha));
    // A KIND NO VERSION WRITES.
    let mut odd = run_record("zerodha_nse_checked fixture").image();
    odd[PULL_KIND_AT] = 0xEE;
    pull_record(&mut parsed, &mut vendor, 2, &odd);
    assert_eq!(parsed.faults.count, 1);
    assert_eq!(
        parsed.faults.named[0].2,
        "a record kind this reader does not know"
    );
    // A MEMBER WHOSE NOTE NAMES NO REFUSAL: not decoded, not a fault.
    pull_record(&mut parsed, &mut vendor, 3, &quiet.image());
    // A ZERODHA MEMBER THAT REFUSES AND NAMES NO DAY: decoded, not kept.
    pull_record(
        &mut parsed,
        &mut vendor,
        4,
        &member_record("ZZOIL", DAYLESS).image(),
    );
    assert!(parsed.by_symbol.is_empty());
    assert_eq!(parsed.faults.count, 1);
    // A WHOLE-WINDOW REFUSAL OF ZERODHA'S: kept.
    pull_record(
        &mut parsed,
        &mut vendor,
        5,
        &member_record("ZZOIL", WHOLE).image(),
    );
    assert_eq!(parsed.by_symbol["ZZOIL"].len(), 1);
    // A MEMBER WHOSE BYTES CHANGED: a fault, and the run's vendor stands.
    let mut damaged = member_record("ZZOIL", WHOLE).image();
    damaged[16] ^= 0xFF;
    pull_record(&mut parsed, &mut vendor, 6, &damaged);
    assert_eq!(parsed.faults.count, 2);
    assert_eq!(vendor, Some(Vendor::Zerodha));
    // A RUN WHOSE BYTES CHANGED NAMES NO VENDOR: its members are counted apart.
    let mut run = run_record("zerodha_nse_checked fixture").image();
    run[16] ^= 0xFF;
    pull_record(&mut parsed, &mut vendor, 7, &run);
    assert_eq!(parsed.faults.count, 3);
    assert_eq!(vendor, None);
    pull_record(
        &mut parsed,
        &mut vendor,
        8,
        &member_record("ZZOIL", WHOLE).image(),
    );
    assert_eq!(parsed.other_vendor, 1);
    assert_eq!(parsed.by_symbol["ZZOIL"].len(), 1);
    // A JOURNAL THAT WOULD NOT READ, IN THE ANSWER'S WORDS.
    let failed = PullUnread::Failed("an invented failure".to_owned());
    assert_eq!(
        failed.note(),
        "The Zerodha pull journal could not be read, so no record names a pull refusal and \
         refused_front is judged from the gap ledger alone: an invented failure"
    );
    assert_eq!(
        failed.json(),
        json!({ "present": null, "error": "an invented failure" })
    );
}

/// **A WALK STOPS WHERE THE FILE STOPS**: told the journal holds more than it
/// does (cut back, or a record half written, after its length was read), the
/// walk keeps the whole records it found and goes no further.
///
/// Extremes: singles — a torn tail and no whole record, a whole record and a
/// torn one, nothing past the reader's place, a length already read, a length
/// of zero, a length of one record exactly read whole, a folder at the
/// journal's path, a run record's vendor kept; combinations — a whole record
/// and a torn tail read against a length three records long at once, the
/// torn tail read again alone against the same length at once; out of the box
/// — a length of `u64::MAX`, past every bound.
#[test]
fn a_walk_stops_where_the_file_stops() {
    let root = root_of("cover-advance");
    let path = root.join("pull.journal");
    let record = run_record("zerodha_nse_checked fixture").image();
    let stride = crate::audit::RECORD_LEN_U64;
    // A TORN TAIL ONLY: no whole record.
    std::fs::write(&path, b"torn tail").expect("the journal");
    let mut tail = PullTail::fresh(None);
    assert_eq!(tail.advance(&path, 3 * stride), Ok(0));
    assert_eq!(tail.upto, 0);
    // A WHOLE RECORD AND A TORN ONE, AGAINST A LENGTH OF THREE.
    let mut bytes = record.to_vec();
    bytes.extend_from_slice(b"torn tail");
    std::fs::write(&path, &bytes).expect("the journal");
    assert_eq!(tail.advance(&path, 3 * stride), Ok(1));
    assert_eq!(tail.upto, stride);
    assert_eq!(
        tail.vendor,
        Some(Vendor::Zerodha),
        "the run's vendor stands"
    );
    // AGAIN: only the torn bytes past the reader's place.
    assert_eq!(tail.advance(&path, 3 * stride), Ok(0));
    assert_eq!(tail.upto, stride);
    // CUT BACK TO THE RECORD: nothing past the reader's place.
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("open")
        .set_len(stride)
        .expect("cut back");
    assert_eq!(tail.advance(&path, 3 * stride), Ok(0));
    // A LENGTH ALREADY READ, AND ONE OF ZERO: nothing opened.
    assert_eq!(tail.advance(&path, stride), Ok(0));
    assert_eq!(tail.advance(&path, 0), Ok(0));
    // ONE RECORD EXACTLY, AND A LENGTH PAST EVERY BOUND: the file's end.
    assert_eq!(PullTail::fresh(None).advance(&path, stride), Ok(1));
    let mut unbounded = PullTail::fresh(None);
    assert_eq!(unbounded.advance(&path, u64::MAX), Ok(1));
    assert_eq!(unbounded.upto, stride);
    // A FOLDER AT THE JOURNAL'S PATH: refused by name.
    let folder = root.join("folder.journal");
    std::fs::create_dir_all(&folder).expect("a folder");
    let why = PullTail::fresh(None)
        .advance(&folder, stride)
        .expect_err("a folder is not read");
    assert!(why.starts_with(&format!("{}: ", folder.display())), "{why}");
}

/// **A JOURNAL REPLACED OR CUT BACK UNDER ITS TAIL IS WALKED AGAIN FROM ITS
/// START, BY A REQUEST AND AT START ALIKE; A WALK AT START WITH NOTHING NEW
/// HOLDS NO REQUEST BACK** (DB-A34, DB-N19).
///
/// Extremes: singles — a journal with no index read by `/ingest`'s reader, a
/// journal replaced by another file (a new inode), a journal cut back in
/// place, the walk-at-start mark on a tail with nothing new, a journal walked
/// at start after its replacement, one walked at start after it was cut back,
/// a request after the walk at start (nothing new), a mark taken off a path no
/// tail holds; combinations — a replacement holding the very same bytes (only
/// the inode differs) at once, a cut back to the run record alone (its
/// refusal gone with it) at once; out of the box — the mark taken off a path
/// that was never a journal.
#[test]
fn a_journal_replaced_or_cut_back_under_its_tail_is_walked_again() {
    let root = root_of("cover-pull-restart");
    zerodha_run(&root, &[("ZZOIL", WHOLE)]);
    let journal = crate::audit::Journal::at(&root);
    std::fs::remove_dir_all(journal.index_dir()).expect("the appends built an index");
    let path = journal.path.clone();
    let stride = crate::audit::RECORD_LEN_U64;
    let (refusals, read) = zerodha_refusals_of(&root, "ZZOIL").expect("/ingest's reader");
    assert_eq!(refusals.len(), 1);
    assert_eq!(read["via"]["kind"], "walk", "{read:#}");
    assert_eq!(read["read_now"], 2);
    // REPLACED: the very same bytes under a new inode.
    let bytes = std::fs::read(&path).expect("the journal");
    let next = path.with_extension("next");
    std::fs::write(&next, &bytes).expect("a copy");
    std::fs::rename(&next, &path).expect("replaced");
    let again = read_pull_journal(&root, "ZZOIL").expect("walked again");
    assert_eq!(again.read_now, 2, "every record of the new file");
    assert_eq!(again.parsed.by_symbol["ZZOIL"].len(), 1, "never twice");
    // CUT BACK IN PLACE to the run record: read again, the refusal gone.
    let cut_back = || {
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("open")
            .set_len(stride)
            .expect("cut back");
    };
    cut_back();
    let cut = read_pull_journal(&root, "ZZOIL").expect("walked again");
    assert_eq!((cut.records, cut.read_now), (1, 1));
    assert!(!cut.parsed.by_symbol.contains_key("ZZOIL"));
    // THE WALK-AT-START MARK ON A TAIL WITH NOTHING NEW: the request answers.
    set_mark(&path, true);
    let marked = read_pull_journal(&root, "ZZOIL").expect("nothing to wait for");
    assert_eq!(marked.read_now, 0);
    set_mark(&path, false);
    // AT START, AFTER A REPLACEMENT AND AFTER A CUT BACK: walked again.
    std::fs::write(&next, &bytes).expect("a copy");
    std::fs::rename(&next, &path).expect("replaced");
    assert_eq!(warm_pull_journal(&root).map(|walked| walked.read), Ok(2));
    cut_back();
    assert_eq!(warm_pull_journal(&root).map(|walked| walked.read), Ok(1));
    let after = read_pull_journal(&root, "ZZOIL").expect("a tail read");
    assert_eq!(after.read_now, 0, "the walk at start was installed");
    // A MARK TAKEN OFF A PATH NO TAIL HOLDS: nothing made.
    let never = root.join("never-a-journal");
    drop(Unmark(&never));
    let held = pull_tails().lock().unwrap_or_else(PoisonError::into_inner);
    assert!(!held.contains_key(&never));
}

/// **THROUGH THE INDEX, A RECORD THE INDEX NAMES WRONGLY IS A FAULT, AND THE
/// RECORDS PAST ITS COVER ARE READ IN ORDER** (WP-33c, D-5400). The index
/// files a refusal by its ordinal; the record at that ordinal replaced in
/// place by another symbol's, by a run or by bytes that will not decode is a
/// fault named by its source, never the symbol's refusal.
///
/// Extremes: singles — an indexed refusal that names no day, the indexed
/// record replaced by another symbol's, by a run, by bytes that will not
/// decode, a run past the cover, damaged bytes past the cover, a refusal of
/// the symbol past the cover, a journal of one run; combinations — damage at
/// the indexed record and past the cover at once, a run and a refusal past
/// the cover at once; out of the box — 256 bytes of one repeated byte
/// standing as a record.
#[test]
fn through_the_index_a_wrongly_named_record_is_a_fault() {
    let root = root_of("cover-pull-index");
    zerodha_run(&root, &[("ZZOIL", DAYLESS)]);
    let journal = crate::audit::Journal::at(&root);
    let read = || read_pull_journal(&root, "ZZOIL").expect("through the index");
    let first = read();
    assert_eq!(
        first.via,
        Via::Index {
            covered: 2,
            runs: 1
        }
    );
    assert_eq!(first.read_now, 1, "the one indexed refusal");
    assert!(
        first.parsed.by_symbol.is_empty(),
        "no day named, nothing kept"
    );
    assert_eq!(first.parsed.faults.count, 0);
    // THE INDEXED RECORD REPLACED IN PLACE: the same inode, the index as it was.
    let put = |image: &[u8]| {
        use std::os::unix::fs::FileExt as _;
        std::fs::OpenOptions::new()
            .write(true)
            .open(&journal.path)
            .expect("open")
            .write_all_at(image, crate::audit::RECORD_LEN_U64)
            .expect("in place");
    };
    let named_wrongly = "the journal's index names this record under another source or kind";
    put(&member_record("ZZOTHER", DAYLESS).image());
    let other = read();
    assert_eq!(
        other.via,
        Via::Index {
            covered: 2,
            runs: 1
        }
    );
    assert_eq!(other.parsed.faults.count, 1);
    assert_eq!(
        other.parsed.faults.named[0],
        (2, "ZZOTHER".to_owned(), named_wrongly.to_owned())
    );
    put(&run_record("zerodha_nse_checked fixture").image());
    let run = read();
    assert_eq!(
        run.parsed.faults.named[0],
        (
            2,
            "zerodha_nse_checked fixture".to_owned(),
            named_wrongly.to_owned()
        )
    );
    put(&[0x5A; crate::audit::RECORD_LEN]);
    let garbage = read();
    assert_eq!(garbage.parsed.faults.count, 1);
    assert_eq!(garbage.parsed.faults.named[0].1, "", "no source to name");
    // PAST THE COVER: a run, which is no fault and no refusal.
    let mut bytes = std::fs::read(&journal.path).expect("the journal");
    bytes.extend_from_slice(&run_record("zerodha_nse_checked fixture").image());
    std::fs::write(&journal.path, &bytes).expect("past the cover");
    let past = read();
    assert_eq!((past.records, past.read_now), (3, 2));
    assert_eq!(past.parsed.faults.count, 1);
    assert!(past.parsed.by_symbol.is_empty());
    // THEN BYTES THAT WILL NOT DECODE, THEN A REFUSAL OF THE SYMBOL.
    bytes.extend_from_slice(&[0xA5; crate::audit::RECORD_LEN]);
    bytes.extend_from_slice(&member_record("ZZOIL", WHOLE).image());
    std::fs::write(&journal.path, &bytes).expect("past the cover");
    let both = read();
    assert_eq!(
        both.via,
        Via::Index {
            covered: 2,
            runs: 1
        }
    );
    assert_eq!((both.records, both.read_now), (5, 4));
    assert_eq!(
        both.parsed.faults.count, 2,
        "the indexed record and one past the cover"
    );
    assert_eq!(both.parsed.faults.named[1].0, 4);
    assert_eq!(
        both.parsed.by_symbol["ZZOIL"].len(),
        1,
        "under the run before it"
    );
}

/// **A FOLDER WHERE A JOURNAL OR A LEDGER GOES, OR A FILE WHERE THEIR FOLDER
/// GOES, IS NAMED IN THE ANSWER AND NEVER READ AS EMPTY** (R3-08 repair 2).
/// On a GDFL page the import journal and the day ledger are folders; on a
/// Zerodha page `audit` is a file, so the gap ledger and the pull journal under
/// it are refused, by a request, at start and by `/ingest`'s reader alike; and
/// a folder at the pull journal's path is refused, never walked as records.
///
/// Extremes: singles — the import journal a folder, the day ledger a folder,
/// `audit` a file under the gap ledger, `audit` a file under the pull journal
/// read by a request, at start, and by `/ingest`, the pull journal a folder at
/// start, the same folder on a request; combinations — the import journal and
/// the day ledger both folders on one GDFL page at once, the gap ledger and
/// the pull journal both under a file on one Zerodha page at once; out of the
/// box — a folder of 64 entries at the pull journal's path, sized like a
/// journal of whole records.
#[test]
fn a_folder_or_a_file_where_a_source_goes_is_named_never_read_as_empty() {
    // THE GDFL PAGE: the import journal and the day ledger are folders.
    let root = root_of("cover-folders-gdfl");
    for path in [pull::gdfl_import::journal_path(&root), quality_path(&root)] {
        write(&path.join("inside"), "an entry");
        let size = std::fs::metadata(&path).expect("the folder").len();
        assert!(size > 0, "the premise: a folder of some size");
    }
    let got = answer(
        &root,
        &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-01-02&to=2024-01-02"),
    );
    let journal_why = got["sources"]["journal"]["error"]
        .as_str()
        .expect("the journal's refusal");
    assert!(journal_why.contains("Is a directory"), "{got:#}");
    assert_eq!(got["sources"]["journal"]["journal"], Value::Null);
    let quality_why = got["sources"]["quality"]["error"]
        .as_str()
        .expect("the ledger's refusal");
    assert!(quality_why.contains("Is a directory"), "{got:#}");
    let notes: Vec<&str> = got["notes"]
        .as_array()
        .expect("notes")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let journal_note = format!(
        "The import journal could not be read, so every record's import is null: {journal_why}"
    );
    let quality_note = format!(
        "The GDFL day ledger could not be read, so every record's quality is null: {quality_why}"
    );
    assert!(notes.contains(&journal_note.as_str()), "{notes:#?}");
    assert!(notes.contains(&quality_note.as_str()), "{notes:#?}");
    assert_eq!(got["source_faults"].as_array().map(Vec::len), Some(2));
    let only = record(&got, "2024-01-02");
    assert_eq!(only["import"], Value::Null);
    assert_eq!(only["quality"], Value::Null);
    // THE ZERODHA PAGE: `audit` is a file.
    let file_root = root_of("cover-folders-zerodha");
    write(
        &file_root.join("audit"),
        "a file where the audit folder goes",
    );
    let journal = crate::audit::journal_path(&file_root);
    let unread = read_pull_journal(&file_root, "ZZOIL").err();
    let Some(PullUnread::Failed(failed)) = unread else {
        panic!("a journal under a file is refused: {unread:?}");
    };
    assert!(
        failed.starts_with(&format!("{}: ", journal.display())),
        "{failed}"
    );
    assert!(failed.contains("Not a directory"), "{failed}");
    let warm = warm_pull_journal(&file_root).expect_err("refused at start too");
    assert_eq!(warm, failed);
    let note = PullUnread::Failed(failed.clone()).note();
    assert_eq!(
        zerodha_refusals_of(&file_root, "ZZOIL").expect_err("refused"),
        note
    );
    let got = answer(
        &file_root,
        &ask("feed=zerodha&instrument=NSE-INDEX-ZZOIL&from=2024-01-02&to=2024-01-02"),
    );
    assert_eq!(
        got["sources"]["pull_journal"],
        json!({ "present": null, "error": failed })
    );
    let gaps_why = got["sources"]["gaps"]["error"].as_str().expect("refused");
    assert!(gaps_why.contains("Not a directory"), "{got:#}");
    let faults: Vec<&str> = got["source_faults"]
        .as_array()
        .expect("faults")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(faults, [gaps_why, note.as_str()]);
    let notes: Vec<&str> = got["notes"]
        .as_array()
        .expect("notes")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(notes.contains(&note.as_str()), "{notes:#?}");
    // A FOLDER AT THE PULL JOURNAL'S PATH, sized like whole records.
    let folder_root = root_of("cover-folders-journal");
    let folder = crate::audit::journal_path(&folder_root);
    for n in 0..64 {
        write(&folder.join(format!("entry-{n:02}")), "x");
    }
    let size = std::fs::metadata(&folder).expect("the folder").len();
    assert!(
        size >= crate::audit::RECORD_LEN_U64,
        "the premise: {size} bytes"
    );
    let warm = warm_pull_journal(&folder_root).expect_err("a folder is not walked");
    assert!(
        warm.starts_with(&format!("{}: ", folder.display())),
        "{warm}"
    );
    assert!(warm.contains("Is a directory"), "{warm}");
    let unread = read_pull_journal(&folder_root, "ZZOIL").err();
    let Some(PullUnread::Failed(request)) = unread else {
        panic!("a folder is refused on a request: {unread:?}");
    };
    assert_eq!(request, warm);
}

/// **A ZERODHA `1day` MONTH THAT WILL NOT READ IS SAID ON EACH OF ITS DAYS,
/// AS DAMAGE OR AS A WRITER'S HOLD** (R3-08, RA-117). January holds 65
/// records, one past what a month of days can; February is text; a writer
/// holds March. Damage is answered as damage beside the writer.
///
/// Extremes: singles — a month of 65 records, a month whose bytes are no bar
/// file, a month a writer holds, every day of the damaged months, every day
/// of the held month, a page of no days, the unreadable day bar's word, the
/// held day bar's word; combinations — damaged and held months on one page at
/// once (no "ask again" over damage), 65 records all on one day at once; out
/// of the box — a text file where the month's bars go.
#[test]
fn a_day_bar_month_that_will_not_read_is_said_on_each_of_its_days() {
    let root = root_of("cover-day-bars");
    let key = "NSE-INDEX-ZZDAY";
    let inst = instrument_of(key, "").expect("a key");
    let odd: Vec<Bar> = (0..65)
        .map(|second| bar("2024-01-02", second, [500, 510, 490, 505], 1))
        .collect();
    file_bars(&root, Vendor::Zerodha, key, Timeframe::DAY_1, odd);
    write(
        &month_of(&inst, Vendor::Zerodha, Timeframe::DAY_1, "2024-02-01").to_path_buf(&root),
        "not a bar file, invented for this test",
    );
    let held = month_of(&inst, Vendor::Zerodha, Timeframe::DAY_1, "2024-03-01");
    let writer = BarFile::open_or_create(&root, held, symbol_id(&inst.symbol)).expect("a writer");
    let days = days_between(day("2024-01-01"), day("2024-03-31"));
    let bars = day_bars(&root, &inst, &days);
    let (damaged, march) = bars.per_day.split_at(31 + 29);
    assert_eq!(damaged, [DayBarAt::Unreadable; 60]);
    assert_eq!(march, [DayBarAt::BeingWritten; 31]);
    assert_eq!(
        bars.notes[0],
        "zerodha NSE-INDEX-ZZDAY 1day 2024-01-01: the month could not be read: it holds 65 \
         records, more than the 64 a month of days can"
    );
    assert!(
        bars.notes[1]
            .starts_with("zerodha NSE-INDEX-ZZDAY 1day 2024-02-01: the month could not be read: "),
        "{:?}",
        bars.notes
    );
    assert!(
        bars.notes[2]
            .starts_with("zerodha NSE-INDEX-ZZDAY 1day 2024-03-01: being written, ask again"),
        "{:?}",
        bars.notes
    );
    assert_eq!(bars.faults, bars.notes);
    let got = answer(
        &root,
        &ask(&format!(
            "feed=zerodha&instrument={key}&from=2024-01&to=2024-03"
        )),
    );
    assert_eq!(
        record(&got, "2024-01-02")["day_bar"],
        json!({ "held": null, "why": "unreadable" })
    );
    assert_eq!(
        record(&got, "2024-03-04")["day_bar"],
        json!({ "held": null, "why": "being_written" })
    );
    assert!(
        being_written(&got).is_none(),
        "damage beside the writer is damage"
    );
    drop(writer);
    // A PAGE OF NO DAYS: nothing read, and nothing before it looked for.
    let none = day_bars(&root, &inst, &[]);
    assert!(none.per_day.is_empty());
    assert!(!none.none_before);
    assert!(none.notes.is_empty());
}

/// **A MONTH THE STORE CANNOT NAME, OR WHOSE SERVED GENERATION IS GONE, IS
/// REFUSED; ONE IT CAN NAME AND DOES NOT HOLD IS ABSENT; A SERIES WHOSE FIRST
/// HELD MONTH WILL NOT OPEN STARTS NOWHERE** (D-3315, D-3350).
///
/// Extremes: singles — a symbol not in the store's case, an empty symbol, an
/// option kind with no contract on GDFL (no retired-key probe), a repaired
/// month whose revision's bars are gone, a first held month that is text, the
/// same with the months back known empty, a first held month that reads (its
/// first bar), a page whose months back hold one; combinations — a revision
/// gone under a month whose original is still there at once, a text month
/// ahead of the page with nothing held behind it at once; out of the box — a
/// lowercase symbol, which the census key reader never makes, handed straight
/// to the month probe.
#[test]
fn a_month_the_store_cannot_name_or_open_is_refused_and_starts_nothing() {
    let root = root_of("cover-months");
    let month = YearMonth::new(2024, 3).expect("a month");
    // A SYMBOL THE STORE CANNOT NAME: no month held, and the open refused.
    for symbol in ["lower", ""] {
        let odd = Instrument {
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol: symbol.to_owned(),
            contract: None,
        };
        assert!(!day_month_held(&root, &odd, month), "{symbol:?}");
        let opened = open_month(&root, Vendor::Zerodha, &odd, Timeframe::DAY_1, month);
        let Opened::Refused(why) = opened else {
            panic!("{symbol:?}: a path the store cannot name is refused");
        };
        assert!(why.starts_with(&format!("NSE-INDEX-{symbol} ")), "{why}");
    }
    // AN OPTION KIND NAMES NO CONTRACT: no retired key to ask after; absent.
    let kind = instrument_of("NSE-FNO-NIFTY", "").expect("a kind");
    let opened = open_month(&root, Vendor::Gdfl, &kind, Timeframe::SECOND_1, month);
    assert!(matches!(opened, Opened::Absent));
    // A REPAIRED MONTH WHOSE REVISION'S BARS ARE GONE: refused, never absent.
    let nifty = instrument_of("NSE-INDEX-NIFTY", "").expect("a key");
    let path = month_of(&nifty, Vendor::Zerodha, Timeframe::MINUTE_1, "2024-03-01");
    let id = symbol_id(&nifty.symbol);
    let first = bar("2024-03-01", 9 * 3_600 + 15 * 60, [500; 4], 1);
    let later = bar("2024-03-01", 9 * 3_600 + 16 * 60, [501; 4], 1);
    let mut file = BarFile::open_or_create(&root, path, id).expect("a bar file");
    file.append(&[first]).expect("one record");
    let header = file.header();
    drop(file);
    let revision = store::repair::Revision::new(1).expect("revision 1");
    store::repair::publish(&root, path, id, revision, header, &[first, later]).expect("a revision");
    store::generation::promote(&root, path, id, revision).expect("promoted");
    let served = open_month(&root, Vendor::Zerodha, &nifty, Timeframe::MINUTE_1, month);
    assert!(
        matches!(served, Opened::File(_)),
        "the premise: the revision is served"
    );
    std::fs::remove_file(path.to_path_buf(&store::repair::revision_root(&root, revision)))
        .expect("the revision's bars go");
    let gone = open_month(&root, Vendor::Zerodha, &nifty, Timeframe::MINUTE_1, month);
    let Opened::Refused(why) = gone else {
        panic!("a served revision that is gone is refused");
    };
    assert!(why.contains("bar-revisions-v1"), "{why}");
    assert!(why.contains("does not exist"), "{why}");
    // A FIRST HELD MONTH THAT IS TEXT: nothing to be in front of.
    let zz = instrument_of("NSE-INDEX-ZZSTART", "").expect("a key");
    write(
        &month_of(&zz, Vendor::Zerodha, Timeframe::DAY_1, "2024-05-01").to_path_buf(&root),
        "not a bar file, invented for this test",
    );
    for none_before in [false, true] {
        assert_eq!(
            series_start(&root, &zz, day("2024-03-04"), none_before),
            SeriesStart::NoneHeld,
            "{none_before}"
        );
    }
    // A FIRST HELD MONTH THAT READS: its first bar (the control).
    file_bars(
        &root,
        Vendor::Zerodha,
        "NSE-INDEX-ZZSTART",
        Timeframe::DAY_1,
        vec![bar("2024-04-02", 0, [500, 510, 490, 505], 1)],
    );
    assert_eq!(
        series_start(&root, &zz, day("2024-03-04"), false),
        SeriesStart::FirstBar(day("2024-04-02"))
    );
    // A MONTH BACK IS HELD: the page is past the front.
    assert_eq!(
        series_start(&root, &zz, day("2024-06-03"), false),
        SeriesStart::HeldBefore
    );
}

/// **A SOURCED HALT, A SAMPLED DAY WITH NO COUNTS, AN OPTIONS DAY OF LTQ=1
/// LEADS AND A DAY BAR PRICED AT OR BELOW ZERO ARE EACH SAID FOR WHAT THEY
/// ARE** (D-3320, D-3335 §3, DB-B20).
///
/// Extremes: singles — a sourced halt (the calendar holds none yet), a systems
/// outage's word, a sampled day with no counts, one with a held count and no
/// session, an options day whose LTQ=1 leads are exactly half, one whose are
/// one short of half, a day bar with a zero high, one with a zero low, one
/// with a zero close after a held close, a close after a zero close, a close
/// after a negative close, a phrase of 141 characters, one of 140, the
/// unreadable and the held day bar's words; combinations — a zero close after
/// a zero close at once, zero quotes under half with leads at half at once;
/// out of the box — 141 two-byte rupee signs (cut by characters, never by
/// bytes), and `is` given as the text `"true"`.
#[test]
fn each_odd_day_is_said_for_what_it_is() {
    let (_, events) = calendar_json(&sourced_halt(), Venue::NseIndex, (Vendor::Gdfl, Some(0)));
    assert_eq!(events["halt"]["standing"], "sourced", "{events:#}");
    assert_eq!(events["halt"]["evidence"], "an invented primary record");
    assert_eq!(special_word(SpecialKind::SystemsOutage), "systems_outage");
    assert_eq!(special_word(SpecialKind::Muhurat), "muhurat");
    // A SAMPLED DAY WITH NO COUNTS SAYS NO SHARE OF A SESSION.
    let most = pull::gdfl_import::quality::SAMPLED_SS_DISTINCT;
    let suspected = format!(
        "index: minute-sampled or mostly missing (suspected), at most {most} a minute; no \
         day-ledger line measures its rows yet"
    );
    assert_eq!(
        sampled_says(&json!({ "is": true, "of": "own" })),
        Some(suspected.clone())
    );
    assert_eq!(
        sampled_says(&json!({ "is": true, "of": "own", "held_s": 3 })),
        Some(suspected)
    );
    assert_eq!(
        sampled_says(&json!({ "is": "true", "of": "own" })),
        None,
        "text is no yes"
    );
    // AN OPTIONS DAY: degraded at half its traded files, not under.
    let options = |set: &[(&str, &str)]| {
        let text = flags_line("options", "2024-01-02", "*", &OPTIONS_FIELDS, set);
        parse_quality(1, text.trim_end()).expect("an options line")
    };
    let traded = [("files_traded", "10"), ("zero_quote_files", "4")];
    let half = options(&[traded[0], traded[1], ("ltq1_lead_files", "5")]);
    let under = options(&[traded[0], traded[1], ("ltq1_lead_files", "4")]);
    let degraded = degraded_json(&[(ImportKind::Options, Some(&half))], &Value::Null);
    assert_eq!(degraded.len(), 1);
    assert_eq!(
        degraded[0]["says"],
        "options: 4 of 10 traded files carry zero bid and ask, 5 open with an LTQ=1 row"
    );
    assert!(degraded_json(&[(ImportKind::Options, Some(&under))], &Value::Null).is_empty());
    // A DAY BAR AT OR BELOW ZERO.
    let session = record_on("2024-01-02");
    let bar_of = |open: i64, high: i64, low: i64, close: i64| {
        DayBarAt::Bar(DayBar {
            open,
            high,
            low,
            close,
            volume: 1,
        })
    };
    let junk = |at: DayBarAt, prev: Option<i64>| {
        junk_codes(
            &session,
            at,
            prev,
            Held::Bars(375),
            Held::Bars(22_500),
            Segment::Index,
        )
    };
    assert_eq!(junk(bar_of(500, 0, 490, 505), Some(500)), ["zero_price"]);
    assert_eq!(junk(bar_of(500, 510, 0, 505), Some(500)), ["zero_price"]);
    assert_eq!(
        junk(bar_of(500, 510, 490, 0), Some(500)),
        ["zero_price"],
        "no jump from zero"
    );
    assert_eq!(junk(bar_of(500, 510, 490, 0), Some(0)), ["zero_price"]);
    assert!(
        junk(bar_of(500, 510, 490, 505), Some(0)).is_empty(),
        "no jump after zero"
    );
    assert!(
        junk(bar_of(500, 510, 490, 505), Some(-5)).is_empty(),
        "nor after below zero"
    );
    assert_eq!(
        junk(bar_of(500, 1_300, 490, 1_300), Some(500)),
        ["impossible_jump"]
    );
    assert_eq!(
        day_bar_json(DayBarAt::Unreadable),
        json!({ "held": null, "why": "unreadable" })
    );
    assert_eq!(
        day_bar_json(DayBarAt::BeingWritten),
        json!({ "held": null, "why": "being_written" })
    );
    // A PHRASE IS CUT BY CHARACTERS, AND MARKED.
    let long = "z".repeat(MAX_PHRASE_CHARS + 1);
    assert_eq!(cut(&long), format!("{}...", "z".repeat(MAX_PHRASE_CHARS)));
    assert_eq!(cut(&long[1..]), long[1..]);
    let rupees = "\u{20b9}".repeat(MAX_PHRASE_CHARS + 1);
    assert_eq!(
        cut(&rupees),
        format!("{}...", "\u{20b9}".repeat(MAX_PHRASE_CHARS))
    );
}

/// **EACH PHRASE A DAY'S SOURCES OWE IS SAID, AND NONE IS MADE FROM A FIELD
/// THAT IS NOT THERE** (DB-B45, D-3315, D-3335, AF-02 repair 2).
///
/// Extremes: singles — a degraded entry with no `says`, a sourced halt,
/// earlier refusals with no closing run, index holes with a stamp, index holes
/// with none, rows refused at import, LTQ=0 rows on a stocks day, a withheld
/// session in the gap ledger, a ledger code with no phrase of its own, three
/// dated pull refusals, the other feed's month unreadable, a close with no
/// refusal at all, two refusals the logs read do not hold; combinations — a
/// whole-window refusal before the first bar that the journal does not call a
/// front, with the journal read, at once; another file's refusal beside
/// refusals no log read holds at once; out of the box — a degraded entry whose
/// `says` is a number.
#[test]
fn each_phrase_a_days_sources_owe_is_said() {
    let session = record_on("2024-01-02");
    let calendar = json!({});
    let has = |said: &[String], phrase: &str| {
        assert!(
            said.iter().any(|s| s == phrase),
            "{phrase:?} not in {said:#?}"
        );
    };
    // A DEGRADED ENTRY WITH NO WORDS, OR WORDS THAT ARE NO TEXT: no phrase.
    let odd = [json!({ "kind": "options" }), json!({ "says": 7 })];
    let said = Says {
        degraded: &odd,
        ..says_on(&session, &calendar)
    }
    .phrases();
    assert!(
        !said.iter().any(|s| s.starts_with("degraded vendor day")),
        "{said:#?}"
    );
    has(
        &says_on(&sourced_halt(), &calendar).phrases(),
        "halt (sourced) 09:22-10:20",
    );
    // EARLIER REFUSALS WITH NO CLOSING RUN TO NAME.
    let mut history = Vec::new();
    Says::history_and_late_phrases(
        &json!({ "refusals": { "superseded_count": 2 } }),
        &mut history,
    );
    assert_eq!(
        history,
        ["2 refusal(s) logged by an earlier run are history"]
    );
    // THE DAY LEDGER'S HOLES, REFUSED ROWS AND LTQ=0 ROWS.
    let index_line = |set: &[(&str, &str)]| {
        parse_quality(1, index_flags("2024-01-02", "NIFTY", set).trim_end()).expect("a line")
    };
    let holes = index_line(&[
        ("holes_5s", "3"),
        ("hole_max_s", "12"),
        ("hole_max_at", "10:00:00"),
        ("rows_refused", "4"),
    ]);
    let unstamped = index_line(&[("holes_5s", "1"), ("hole_max_s", "6")]);
    let stocks_text = flags_line(
        "stocks",
        "2024-01-02",
        "*",
        &STOCKS_FIELDS,
        &[("ltq0_new_ltp_rows", "6")],
    );
    let stocks = parse_quality(1, stocks_text.trim_end()).expect("a stocks line");
    let lines = [
        (ImportKind::Indices, Some(&holes)),
        (ImportKind::Stocks, Some(&stocks)),
    ];
    let said = Says {
        quality: &lines,
        ..says_on(&session, &calendar)
    }
    .phrases();
    has(
        &said,
        "3 in-session index hole(s) over 5 s, the longest 12 s at 10:00:00",
    );
    has(&said, "indices: 4 row(s) refused at import");
    has(
        &said,
        "stocks: 6 LTQ=0 row(s) carry a new price (flagged; bars unchanged)",
    );
    let bare = [(ImportKind::Indices, Some(&unstamped))];
    let said = Says {
        quality: &bare,
        ..says_on(&session, &calendar)
    }
    .phrases();
    has(
        &said,
        "1 in-session index hole(s) over 5 s, the longest 6 s at -",
    );
    // THE GAP LEDGER'S CODES.
    let gap_line = |why: &str| GapLine {
        rung: "1min".to_owned(),
        owed: 375,
        held: 0,
        missing: 375,
        first_missing: None,
        runs: 0,
        longest: 0,
        longest_at: None,
        why: why.to_owned(),
        run: 7,
        line: 1,
        may_be_no_trade: false,
    };
    for (why, phrase) in [
        (
            "withheld_unmeasured_session",
            "Zerodha withheld the day: a session of unmeasured length",
        ),
        ("an_invented_code", "Zerodha ledger: an_invented_code"),
    ] {
        let line = gap_line(why);
        let said = Says {
            gap: Some(&line),
            ..says_on(&session, &calendar)
        }
        .phrases();
        has(&said, phrase);
    }
    // THE PULL JOURNAL'S REFUSALS.
    let refusal = |names: Option<&str>, whole_window: bool| PullRefusal {
        record: 2,
        at_unix_secs: 1_791_266_622,
        from_days: day("2018-01-01").days_from_epoch(),
        to_days: day("2026-09-30").days_from_epoch(),
        note: "an invented refusal".to_owned(),
        cut: false,
        names_day: names.map(|on| day(on).days_from_epoch()),
        whole_window,
    };
    let whole = refusal(None, true);
    let front = [&whole];
    let before = record_on("2020-01-13");
    let said = Says {
        pulled: &front,
        journal_present: Some(true),
        journal_front: Some(false),
        start: Some(SeriesStart::FirstBar(day("2023-06-26"))),
        ..says_on(&before, &calendar)
    }
    .phrases();
    has(
        &said,
        "before Zerodha's first held 1day bar, 2023-06-26; no pull refusal is recorded for this day",
    );
    assert!(
        !said.iter().any(|s| s.starts_with("refused front")),
        "{said:#?}"
    );
    let dated: Vec<PullRefusal> = (0..3).map(|_| refusal(Some("2020-11-14"), false)).collect();
    let named: Vec<&PullRefusal> = dated.iter().collect();
    let muhurat = record_on("2020-11-14");
    let said = Says {
        pulled: &named,
        ..says_on(&muhurat, &calendar)
    }
    .phrases();
    has(
        &said,
        "a Zerodha pull refused rows of this day, run of 2026-10-06: an invented refusal (and 2 \
         more record(s))",
    );
    // THE OTHER FEED'S MONTH WILL NOT READ.
    let said = Says {
        peer: Held::Unreadable,
        ..says_on(&session, &calendar)
    }
    .phrases();
    has(&said, "GDFL's month could not be read");
    // A CLOSE'S REFUSALS AGAINST THE LOGS READ.
    let logs = logs_read();
    assert_eq!(
        Tally {
            failures: 3,
            this: 0,
            other: 1
        }
        .unseen(),
        format!("2 of them are in none of the logs this route reads ({logs})")
    );
    assert_eq!(
        closed_phrase(&json!({ "failures": 0 }), "done_with_refusals", None),
        Some("imported with 0 vendor refusal(s)".to_owned())
    );
    let other = json!({
        "failures": 5,
        "refusals": {
            "this_count": 0,
            "other_count": 2,
            "other_first": { "about": "ZZ.NSE.csv", "why": "an invented refusal" },
        },
    });
    assert_eq!(
        closed_phrase(&other, "done_with_refusals", None),
        Some(format!(
            "imported with 5 vendor refusal(s); 2 of them in the logs this route reads, none of \
             those about this instrument; the first is ZZ.NSE.csv: an invented refusal; 3 of them \
             are in none of the logs this route reads ({logs})"
        ))
    );
    assert_eq!(closed_phrase(&other, "done", None), None);
}

/// **THE ENVELOPE NAMES EVERY SOURCE THAT WOULD NOT READ, AND HOW THE PULL
/// JOURNAL WAS READ** (R3-08 repair 2, WP-33c, DB-N19).
///
/// Extremes: singles — the import journal refused, the day ledger refused,
/// the pull journal refused, a walk of the pull journal, a walk of 100,001
/// records, 100,001 records through a partial index, exactly 100,000 records
/// (not said), no pull journal read at all; combinations — the journal, the
/// day ledger and the pull journal all refused on one page at once, a walk
/// past the threshold through a partial index at once; out of the box — a
/// walk whose reason is an empty sentence.
#[test]
fn the_envelope_names_every_source_that_would_not_read() {
    let root = root_of("cover-envelope");
    let gaps = read_ledger(gap_tails(), &gaps_path(&root));
    let walked = |read_now: u64, via: Via| PullRead {
        path: root.join("pull.journal"),
        present: true,
        bytes: read_now * crate::audit::RECORD_LEN_U64,
        records: read_now,
        torn: 0,
        read_now,
        read_micros: 2_500_000,
        parsed: Arc::new(PullRefusals::default()),
        via,
    };
    let journal: Result<JournalRows, String> = Err("an invented journal refusal".to_owned());
    let quality: Result<LedgerRead<QualityLedger>, String> =
        Err("an invented ledger refusal".to_owned());
    let sources = sources_json(Some(&journal), Some(&quality), &gaps, None);
    assert_eq!(
        sources["journal"],
        json!({
            "route": "/imports.json?feed=gdfl",
            "journal": null,
            "error": "an invented journal refusal",
        })
    );
    assert_eq!(sources["quality"]["error"], "an invented ledger refusal");
    assert_eq!(sources["pull_journal"], Value::Null);
    let asked = ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-01-02");
    let notes = source_notes(&asked, Some(&journal), Some(&quality), &gaps);
    assert_eq!(
        notes[..2],
        [
            "The import journal could not be read, so every record's import is null: an \
             invented journal refusal",
            "The GDFL day ledger could not be read, so every record's quality is null: an \
             invented ledger refusal",
        ]
    );
    let walk: Result<PullRead, PullUnread> = Ok(walked(
        3,
        Via::Walk {
            why: "no index yet".to_owned(),
        },
    ));
    assert_eq!(
        sources_json(None, None, &gaps, Some(&walk))["pull_journal"]["via"],
        json!({ "kind": "walk", "why": "no index yet", "refusals_of": "every instrument" })
    );
    let failed: Result<PullRead, PullUnread> =
        Err(PullUnread::Failed("an invented journal failure".to_owned()));
    let said = PullUnread::Failed("an invented journal failure".to_owned()).note();
    assert_eq!(
        source_faults(None, None, &gaps, Some(&failed)),
        [said.clone()]
    );
    assert_eq!(
        source_faults(Some(&journal), Some(&quality), &gaps, Some(&failed)),
        [
            "an invented journal refusal".to_owned(),
            "an invented ledger refusal".to_owned(),
            said,
        ]
    );
    // A WALK PAST THE THRESHOLD IS SAID, WITH ITS TIME AND WHY.
    let over: Result<PullRead, PullUnread> = Ok(walked(
        100_001,
        Via::Walk {
            why: "no index yet".to_owned(),
        },
    ));
    assert_eq!(
        pull_notes(Some(&over)),
        [
            "This answer read 100001 pull-journal record(s) in 2500 ms by a walk of the journal, \
             because its index could not be used (no index yet); a process walks it once, and \
             later reads cost the records appended since."
        ]
    );
    let partial: Result<PullRead, PullUnread> = Ok(walked(
        100_001,
        Via::Index {
            covered: 7,
            runs: 1,
        },
    ));
    let said = pull_notes(Some(&partial));
    assert!(
        said[0].contains("could not be used (the index covers only part of it)"),
        "{said:?}"
    );
    let at: Result<PullRead, PullUnread> = Ok(walked(100_000, Via::Walk { why: String::new() }));
    assert!(
        pull_notes(Some(&at)).is_empty(),
        "the threshold itself is not said"
    );
    let unsaid: Result<PullRead, PullUnread> =
        Ok(walked(100_001, Via::Walk { why: String::new() }));
    let said = pull_notes(Some(&unsaid));
    assert!(said[0].contains("could not be used (); "), "{said:?}");
    assert!(pull_notes(None).is_empty());
}

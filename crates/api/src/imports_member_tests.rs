//! IA-10a (F84, SF-9; D-4896): every api reader of the GDFL import journal
//! names a member key (`<filter>+<member>`, D-4878) and a store hole (IA-16's
//! five reasons), and the day band finds a refusal's run from the event
//! itself, never from the clock and never by decoding every `run` line.
//!
//! EVERY FIXTURE IS INVENTED: the members name real indices because the rule
//! is about names, but every price is a few rupees, every share is a `ZZ`
//! one no exchange lists, and every count is small and round.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]

use super::*;
use crate::server::import_journal::{member_key, member_symbol, parse_day};

/// The run that closed the whole tree.
const RUN_BASE: u64 = 1_792_000_000;
/// The run that reopened two members of the same days, ten minutes later.
const RUN_MEMBER: u64 = 1_792_000_600;

/// A `run` line started at its id, with an invented commit.
fn run_line(id: u64, commit: &str) -> String {
    format!(
        "run {id} source=zip archive=/zz/invented archive_bytes=1 commit={commit} started={id}\n"
    )
}

/// A `begin` and its close: `close` is the verb, `tail` the fields after the
/// counts.
fn key(close: &str, day: &str, filter: &str, tail: &str) -> String {
    let tail = if tail.is_empty() {
        String::new()
    } else {
        format!(" {tail}")
    };
    format!(
        "begin indices {day} {filter}\n{close} indices {day} {filter} definition=2 files=1 \
         seconds=2 failures=0 late=0 late_unresolved=0 max_back_s=0{tail}\n"
    )
}

/// One importer `refused` event as a ring holds it, with the sink's run when
/// it carries one.
fn refused(seq: u32, ms: u64, run: Option<u64>, day: &str, about: &str, why: &str) -> String {
    let run = run.map_or_else(String::new, |id| format!("\"run\":{id},"));
    format!(
        "{{\"seq\":{seq},\"ts\":\"2026-10-17T00:00:00.000Z\",\"ms\":{ms},{run}\"level\":\"error\",\
         \"target\":\"gdfl.import\",\"msg\":\"refused\",\"fields\":{{\"kind\":\"indices\",\
         \"day\":\"{day}\",\"about\":\"{about}\",\"why\":\"{why}\",\"stage\":\"file\"}}}}\n"
    )
}

/// A store root of its own with `journal` as its journal and `ring` as the
/// `cli` ring's one file (none when empty).
fn store(name: &str, journal: &str, ring: &str) -> std::path::PathBuf {
    let root = crate::server::import_journal::tests::journal(&format!("ia10a-{name}"), journal);
    if !ring.is_empty() {
        let dir = root.join("logs").join("cli");
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(dir.join("events.ndjson"), ring).expect("write the cli ring");
    }
    root
}

/// `n` one-second bars of the index `symbol` from 09:15 IST on `on`, priced a
/// few rupees, appended to its GDFL 1s month.
fn bars(root: &std::path::Path, symbol: &str, on: &str, n: i64) {
    use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
    let day = parse_day(on).expect("a day");
    let open = i64::from(day.days_from_epoch()) * 86_400 - pull::session::IST_OFFSET_SECS
        + 9 * 3_600
        + 15 * 60;
    let batch: Vec<store::format::Bar> = (0..n)
        .map(|i| store::format::Bar {
            ts_micros: (open + i) * 1_000_000,
            open: 7_77 + i,
            high: 7_80 + i,
            low: 7_74 + i,
            close: 7_77 + i,
            volume: 0,
            open_interest: store::format::OI_NULL,
        })
        .collect();
    let (year, month) = (day.year(), day.month());
    let path = StorePath::new(PathParts {
        vendor: brutex_core::vendor::Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol,
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(year, month).expect("a month"),
        file: FileKind::Bars,
    })
    .expect("a legal path");
    let id = u32::try_from(brutex_core::universe::fnv1a(symbol) & 0xFFFF_FFFF).expect("32 bits");
    let mut file = store::file::BarFile::open_or_create(root, path, id).expect("a bar file");
    let _appended = file.append(&batch).expect("the bars append");
}

fn ask(query: &str) -> Asked {
    asked(query).unwrap_or_else(|why| panic!("{query}: {why}"))
}

fn rows(got: &Value) -> &Vec<Value> {
    got["rows"].as_array().expect("rows")
}

/// The day band's record of `on`.
fn band_day<'a>(band: &'a Value, on: &str) -> &'a Value {
    band["days"]
        .as_array()
        .expect("days")
        .iter()
        .find(|record| record["day"] == on)
        .unwrap_or_else(|| panic!("{on} in {band:#}"))
}

/// `/dayflags.json`'s answer for `query`.
fn day_band(root: &std::path::Path, query: &str) -> Value {
    let asked = crate::server::dayflags::asked(query).unwrap_or_else(|why| panic!("{why}"));
    crate::server::dayflags::answer(root, &asked)
}

/// The holes object an answer carries, by reason.
fn holes(got: &Value) -> Vec<(String, u64)> {
    got["holes"]
        .as_object()
        .unwrap_or_else(|| panic!("a holes object in {got:#}"))
        .iter()
        .map(|(reason, n)| (reason.clone(), n.as_u64().expect("a count")))
        .collect()
}

/// The journal of IA-JR-01: on 2019-03-01 the whole tree is done under one
/// run, and a later run reopened two members of the same day, each left a
/// hole (one a repair reads, one none does); on 2019-03-04 a member key is
/// the day's only key, done.
fn member_day_journal() -> String {
    [
        run_line(RUN_BASE, "aaaa"),
        key("done", "2019-03-01", "*", &format!("run={RUN_BASE}")),
        run_line(RUN_MEMBER, "bbbb"),
        key(
            "incomplete",
            "2019-03-01",
            "*+NSE-INDEX-NIFTYIT",
            &format!("stage=file reason=needs_month_generation run={RUN_MEMBER}"),
        ),
        key(
            "incomplete",
            "2019-03-01",
            "*+NSE-INDEX-NIFTYNEXT50",
            &format!("stage=file reason=bars_differ_held run={RUN_MEMBER}"),
        ),
        key(
            "done",
            "2019-03-04",
            "*+NSE-INDEX-NIFTYIT",
            &format!("run={RUN_MEMBER}"),
        ),
    ]
    .concat()
}

/// **IA-JR-01: THE API'S JOURNAL READER NAMES MEMBER KEYS AND HOLES**, on
/// every route that reads the journal, over one fixture where `*+M` is a hole
/// and `*` is done for the same day:
///
/// * `/imports.json`: each member row names its base filter and its member;
///   a hole carries `hole: {reason, repairable}` and is counted under
///   `holes` by its reason, `holes_unrepairable` apart, never under
///   `counts.incomplete`; the day whose only key is a member key is
///   journalled, and the whole-day refusal its own run logged gains no
///   made-up log-only `*` row;
/// * `/audit.json?feed=gdfl` through [`summary`]: the same counts;
/// * `/dayflags.json`: NIFTYIT's day stands on its member key and shows the
///   hole; NIFTY's stands on the `*` close, done, and is not touched;
/// * `/provenance.json`: NIFTYIT's month rebuilds 2019-03-01 from its member
///   row, a hole, never from the `*` close's run; NIFTY's from the `*` row.
///
/// Before IA-10a `/imports.json` counted the two holes as plain
/// `incomplete`, the day band and provenance served NIFTYIT the `*` close's
/// `done`, and the member-only day gained a refused `*` row from the log.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one fixture read by the four readers IA-JR-01 names, each asserted against the same journal"
)]
fn the_api_journal_reader_names_member_keys_and_holes() {
    let ring = [
        // The member run's own whole-day refusal of the member-only day:
        // its member key, whose base is `*`, owns it.
        refused(
            1,
            RUN_MEMBER * 1_000 + 9_000,
            Some(RUN_MEMBER),
            "2019-03-04",
            "census",
            "zz invented census words",
        ),
        // The member's own hole, logged by the member run.
        refused(
            2,
            RUN_MEMBER * 1_000 + 5_000,
            Some(RUN_MEMBER),
            "2019-03-01",
            "NIFTY IT.NSE_IDX.csv",
            "zz the month already passed",
        ),
        // The other member's, logged by the same run in the same window:
        // the window alone cannot say whose it is; `pull`'s member of the
        // file does.
        refused(
            3,
            RUN_MEMBER * 1_000 + 6_000,
            Some(RUN_MEMBER),
            "2019-03-01",
            "NIFTY NEXT 50.NSE_IDX.csv",
            "zz a held bar differs",
        ),
    ]
    .concat();
    let root = store("jr01", &member_day_journal(), &ring);
    bars(&root, "NIFTY", "2019-03-01", 3);
    bars(&root, "NIFTYIT", "2019-03-01", 2);
    bars(&root, "NIFTYIT", "2019-03-04", 2);

    // /imports.json
    let got = answer(
        &root,
        &ask("feed=gdfl&kind=indices&from=2019-03-01&to=2019-03-31&order=asc"),
    )
    .expect("reads");
    let shown: Vec<(&str, &str, &str, Value, Value)> = rows(&got)
        .iter()
        .map(|row| {
            (
                row["day"].as_str().unwrap(),
                row["filter"].as_str().unwrap(),
                row["base_filter"].as_str().unwrap(),
                row.get("member").cloned().expect("member, null or named"),
                row.get("hole").cloned().expect("hole, null or named"),
            )
        })
        .collect();
    assert_eq!(
        shown,
        [
            ("2019-03-01", "*", "*", Value::Null, Value::Null),
            (
                "2019-03-01",
                "*+NSE-INDEX-NIFTYIT",
                "*",
                json!("NSE-INDEX-NIFTYIT"),
                json!({ "reason": "needs_month_generation", "repairable": true }),
            ),
            (
                "2019-03-01",
                "*+NSE-INDEX-NIFTYNEXT50",
                "*",
                json!("NSE-INDEX-NIFTYNEXT50"),
                json!({ "reason": "bars_differ_held", "repairable": false }),
            ),
            (
                "2019-03-04",
                "*+NSE-INDEX-NIFTYIT",
                "*",
                json!("NSE-INDEX-NIFTYIT"),
                Value::Null
            ),
        ],
        "no log-only `*` row is made up for the member-only day: {got:#}"
    );
    assert_eq!(rows(&got)[1]["member_symbol"], "NIFTYIT");
    assert_eq!(got["counts"]["done"], 2, "{}", got["counts"]);
    assert_eq!(
        got["counts"]["incomplete"], 0,
        "a hole is not a plain incomplete"
    );
    assert_eq!(got["counts"]["refused"], 0, "{}", got["counts"]);
    assert_eq!(
        holes(&got),
        [
            ("bars_differ_held".to_owned(), 1),
            ("month_superseded".to_owned(), 0),
            ("needs_month_generation".to_owned(), 1),
            ("repair_impossible".to_owned(), 0),
            ("repair_refused".to_owned(), 0),
        ]
    );
    assert_eq!(got["holes_unrepairable"], 1);
    assert!(
        got["notes"].as_array().unwrap().iter().any(|note| note
            .as_str()
            .unwrap()
            .starts_with("2 row(s) closed `incomplete` are store holes")),
        "{:#}",
        got["notes"]
    );
    // `outcome=incomplete` lists the plain ones; `reason=` finds a hole.
    let plain = answer(&root, &ask("feed=gdfl&kind=indices&outcome=incomplete")).expect("reads");
    assert!(rows(&plain).is_empty(), "{plain:#}");
    let by_reason = answer(&root, &ask("feed=gdfl&reason=bars_differ_held")).expect("reads");
    assert_eq!(rows(&by_reason).len(), 1);
    assert_eq!(rows(&by_reason)[0]["member"], "NSE-INDEX-NIFTYNEXT50");

    // /audit.json?feed=gdfl, through the summary
    let brief = summary(&root);
    assert_eq!(brief["counts"], got["counts"], "{brief:#}");
    assert_eq!(brief["holes"], got["holes"]);
    assert_eq!(brief["holes_unrepairable"], 1);

    // /dayflags.json
    let band = day_band(
        &root,
        "feed=gdfl&instrument=NSE-INDEX-NIFTYIT&from=2019-03-01&to=2019-03-04",
    );
    let member = &band_day(&band, "2019-03-01")["import"];
    assert_eq!(member["outcome"], "incomplete", "{member:#}");
    assert_eq!(member["member"], "NSE-INDEX-NIFTYIT");
    assert_eq!(
        member["hole"],
        json!({ "reason": "needs_month_generation", "repairable": true })
    );
    assert_eq!(member["run"], RUN_MEMBER);
    assert_eq!(
        member["refusals"]["this_count"], 1,
        "the member's own file's refusal is its own: {member:#}"
    );
    assert_eq!(
        member["refusals"]["this"][0]["why"],
        "zz the month already passed"
    );
    assert_eq!(
        member["refusals"]["other_count"], 1,
        "the other member's, in the same run's window, is not: {member:#}"
    );
    assert_eq!(band_day(&band, "2019-03-04")["import"]["outcome"], "done");
    let next50 = day_band(
        &root,
        "feed=gdfl&instrument=NSE-INDEX-NIFTYNEXT50&from=2019-03-01&to=2019-03-01",
    );
    let other = &band_day(&next50, "2019-03-01")["import"];
    assert_eq!(
        other["hole"],
        json!({ "reason": "bars_differ_held", "repairable": false }),
        "{other:#}"
    );
    assert_eq!(other["refusals"]["this"][0]["why"], "zz a held bar differs");
    assert_eq!(other["refusals"]["other_count"], 1);
    let base = day_band(
        &root,
        "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2019-03-01&to=2019-03-04",
    );
    let nifty = &band_day(&base, "2019-03-01")["import"];
    assert_eq!(nifty["outcome"], "done", "the base close's members keep it");
    assert_eq!(nifty["filter"], "*");
    assert_eq!(nifty.get("hole"), Some(&Value::Null));
    assert_eq!(
        band_day(&base, "2019-03-04")["import"]["outcome"],
        "none",
        "a member key covers its member and nothing else"
    );

    // /provenance.json
    let provenance = |query: &str| {
        let asked = crate::server::provenance::asked(query).unwrap_or_else(|why| panic!("{why}"));
        crate::server::provenance::answer(&root, &asked)
    };
    let rebuilt = provenance("feed=gdfl&instrument=NSE-INDEX-NIFTYIT&timeframe=1s&month=2019-03");
    assert_eq!(
        rebuilt.status,
        axum::http::StatusCode::OK,
        "{:#}",
        rebuilt.body
    );
    let gdfl = &rebuilt.body["reconstruction"];
    let days: Vec<(&str, &str, Value)> = gdfl["days"]
        .as_array()
        .unwrap_or_else(|| panic!("days in {:#}", rebuilt.body))
        .iter()
        .map(|row| {
            (
                row["day"].as_str().unwrap(),
                row["filter"].as_str().unwrap(),
                row.get("hole").cloned().expect("hole"),
            )
        })
        .collect();
    assert_eq!(
        days,
        [
            (
                "2019-03-01",
                "*+NSE-INDEX-NIFTYIT",
                json!({ "reason": "needs_month_generation", "repairable": true })
            ),
            ("2019-03-04", "*+NSE-INDEX-NIFTYIT", Value::Null),
        ],
        "the member's day rows, never the `*` close's: {gdfl:#}"
    );
    assert_eq!(gdfl["journal_days_holes"], 1);
    let runs: Vec<u64> = gdfl["archive"]["runs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|run| run["id"].as_u64().unwrap_or_else(|| panic!("{run:#}")))
        .collect();
    assert_eq!(
        runs,
        [RUN_MEMBER],
        "no bar is credited to the `*` close's run"
    );
    let nifty = provenance("feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=1s&month=2019-03");
    let nifty_days = nifty.body["reconstruction"]["days"].as_array().unwrap();
    assert_eq!(nifty_days.len(), 1, "{:#}", nifty.body);
    assert_eq!(nifty_days[0]["filter"], "*");
    let _ = std::fs::remove_dir_all(&root);
}

/// **A DAY BAND'S REFUSAL NAMES ITS RUN FROM THE EVENT, NOT THE CLOCK**
/// (D-4896, DB-A34). Three runs close or attempt 2019-03-05: A, then B,
/// whose `*` close is the day's, then C, which closes the day under a list
/// that does not name NIFTY. Each refusal about a NIFTY file is placed by the
/// run its record carries:
///
/// * A's, logged inside B's window by the clock: an earlier run's, history,
///   attributed to A (`run_from: event`);
/// * B's, logged before A started by a clock that stepped back: B's own;
/// * C's, logged inside B's window: never B's close's, so no close of the
///   day owns it and `/imports.json` keeps its log-only `*` row;
/// * one with no run, and one whose run no `run` line names (a `cli`
///   attempt's key): placed by time as before, and the answer says so.
///
/// Before D-4896 the clock placed every one of them, so A's and C's were
/// B's own and B's was history of no run.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "three runs and their refusals on one journal, each placement asserted in the order the doc states them"
)]
fn a_day_bands_refusal_names_its_run_from_the_event_not_the_clock() {
    const A: u64 = 1_792_100_000;
    const B: u64 = 1_792_100_600;
    const C: u64 = 1_792_101_200;
    let journal = [
        run_line(A, "aaaa"),
        run_line(B, "bbbb"),
        key(
            "incomplete",
            "2019-03-05",
            "*",
            &format!("stage=file reason=filing run={B}"),
        ),
        run_line(C, "cccc"),
        key("done", "2019-03-05", "BANKNIFTY", &format!("run={C}")),
    ]
    .concat();
    let in_b = B * 1_000 + 30_000;
    let file = "NIFTY 50.NSE_IDX.csv";
    let ring = [
        refused(1, in_b, Some(A), "2019-03-05", file, "zz said by A"),
        refused(
            2,
            A * 1_000 - 60_000,
            Some(B),
            "2019-03-05",
            file,
            "zz said by B",
        ),
        refused(3, in_b, Some(C), "2019-03-05", file, "zz said by C"),
        refused(4, in_b, None, "2019-03-05", file, "zz said by no run"),
        refused(
            5,
            A * 1_000 + 1_000,
            Some(777),
            "2019-03-05",
            file,
            "zz said by an attempt",
        ),
    ]
    .concat();
    let root = store("band-run", &journal, &ring);
    let band = day_band(
        &root,
        "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2019-03-05&to=2019-03-05",
    );
    let import = &band_day(&band, "2019-03-05")["import"];
    assert_eq!(import["run"], B, "{import:#}");
    let refusals = &import["refusals"];
    let this: Vec<&str> = refusals["this"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| pair["why"].as_str().unwrap())
        .collect();
    assert_eq!(this, ["zz said by B", "zz said by no run"], "{refusals:#}");
    let history: Vec<(&str, Value, &str, Value)> = refusals["superseded"]
        .as_array()
        .unwrap()
        .iter()
        .map(|old| {
            (
                old["why"].as_str().unwrap(),
                old["run"].clone(),
                old["run_from"].as_str().unwrap(),
                old["commit"].clone(),
            )
        })
        .collect();
    assert_eq!(
        history,
        [
            ("zz said by A", json!(A), "event", json!("aaaa")),
            ("zz said by an attempt", json!(A), "time", json!("aaaa")),
        ],
        "{refusals:#}"
    );
    assert_eq!(refusals["later_count"], 1, "C's is never B's: {refusals:#}");
    assert_eq!(refusals["later_this_count"], 1);
    assert_eq!(refusals["later_this_first"]["why"], "zz said by C");
    assert_eq!(
        refusals["placed"],
        json!({ "by_event_run": 3, "by_time": 2 }),
        "{refusals:#}"
    );
    // THE TWO ROUTES AGREE: `/imports.json` places by the same function, so
    // C's refusal, which no close owns, keeps its log-only row.
    let got = answer(
        &root,
        &ask("feed=gdfl&kind=indices&from=2019-03-05&to=2019-03-05&order=asc"),
    )
    .expect("reads");
    let logged: Vec<Value> = rows(&got)
        .iter()
        .filter(|row| row["outcome_from"] == "log")
        .map(|row| row["failures"].clone())
        .collect();
    assert_eq!(logged, [json!(1)], "{got:#}");
    let star = rows(&got)
        .iter()
        .find(|row| row["filter"] == "*" && row["outcome_from"] == "journal")
        .expect("the `*` close");
    let runs: Vec<Value> = star["logged"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["run"].clone())
        .collect();
    assert_eq!(
        runs,
        [json!(A), json!(B), json!(C), Value::Null, json!(777)],
        "each logged line carries its record's own run"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **[`placed_of`]: BY THE EVENT'S RUN WHEN THE JOURNAL NAMES IT, ELSE BY
/// TIME.** Two runs that started in the same second share one window by the
/// clock, and are told apart by their ids; an event run of zero, one no
/// `run` line names, and a close that names no run all fall back to the
/// clock; a run started later than the close's is never the close's.
#[test]
fn a_refusal_is_placed_by_its_own_run_and_by_time_only_without_one() {
    const SAME: u64 = 1_792_200_000;
    let root = store(
        "placed-of",
        &[
            run_line(SAME, "aaaa"),
            format!("run {} source=zip started={SAME}\n", SAME + 1),
            run_line(SAME + 600, "cccc"),
        ]
        .concat(),
        "",
    );
    let parsed = crate::server::import_journal::snapshot(&root)
        .expect("reads")
        .parsed;
    let at = Some(SAME * 1_000 + 5);
    let by = |event: Option<u64>, close: Option<u64>| placed_of(&parsed, event, at, close, None);
    assert_eq!(by(Some(SAME), Some(SAME)), (Placed::Within, PlacedBy::Run));
    assert_eq!(
        by(Some(SAME), Some(SAME + 1)),
        (Placed::Before, PlacedBy::Run),
        "the same second, told apart by the id"
    );
    assert_eq!(
        by(Some(SAME + 1), Some(SAME)),
        (Placed::After, PlacedBy::Run)
    );
    assert_eq!(
        by(Some(SAME + 600), Some(SAME)),
        (Placed::After, PlacedBy::Run)
    );
    assert_eq!(
        by(Some(0), Some(SAME + 1)),
        (Placed::Within, PlacedBy::Time)
    );
    assert_eq!(by(None, Some(SAME + 1)), (Placed::Within, PlacedBy::Time));
    assert_eq!(
        by(Some(42), Some(SAME + 600)),
        (Placed::Before, PlacedBy::Time),
        "a run no line names is the clock's"
    );
    assert_eq!(by(Some(SAME), None), (Placed::Within, PlacedBy::Time));
    // The start index: the latest start at or before a moment, ties to the
    // larger id; nothing before the first run.
    assert_eq!(
        parsed.run_running_at(SAME * 1_000).map(|run| run.id),
        Some(SAME + 1)
    );
    assert_eq!(
        parsed
            .run_running_at((SAME + 600) * 1_000 + 999)
            .map(|run| run.id),
        Some(SAME + 600)
    );
    assert!(parsed.run_running_at(SAME * 1_000 - 1).is_none());
    assert_eq!(parsed.started_of(SAME + 1), SAME, "its own started=");
    assert_eq!(
        parsed.started_of(5),
        5,
        "an id no line names is its own second"
    );
    assert!(!parsed.holds_run(5) && parsed.holds_run(SAME));
    assert_eq!(PlacedBy::Run.as_str(), "event");
    assert_eq!(PlacedBy::Time.as_str(), "time");
    let _ = std::fs::remove_dir_all(&root);
}

/// **EVERY HOLE REASON IS A HOLE, AND NOTHING ELSE IS** (the five at once,
/// the maximum): each of IA-16's five reasons on an `incomplete` close is a
/// hole under its own reason, two of them unrepairable; an `incomplete`
/// with another reason or none, a `refused` with a hole's word and a `done`
/// with refusals that names one are not holes and keep their outcomes.
#[test]
fn every_hole_reason_is_a_hole_and_nothing_else_is() {
    let mut journal = String::new();
    for (n, reason) in pull::gdfl_import::hole_reasons().iter().enumerate() {
        journal.push_str(&key(
            "incomplete",
            &format!("2019-04-0{}", n + 1),
            "*",
            &format!("stage=file reason={reason}"),
        ));
    }
    journal.push_str(&key(
        "incomplete",
        "2019-04-08",
        "*",
        "stage=file reason=filing",
    ));
    journal.push_str(&key("incomplete", "2019-04-09", "*", ""));
    journal.push_str(&key(
        "refused",
        "2019-04-10",
        "*",
        "stage=file reason=needs_month_generation",
    ));
    journal.push_str(&key(
        "done",
        "2019-04-11",
        "*",
        "vendor_refused=1 reason=bars_differ_held",
    ));
    let root = store("five", &journal, "");
    let got = answer(&root, &ask("feed=gdfl&order=asc")).expect("reads");
    assert!(
        holes(&got).iter().all(|(_, n)| *n == 1),
        "{:#}",
        got["holes"]
    );
    assert_eq!(holes(&got).len(), 5);
    assert_eq!(got["holes_unrepairable"], 2);
    assert_eq!(got["counts"]["incomplete"], 2, "{}", got["counts"]);
    assert_eq!(got["counts"]["refused"], 1);
    assert_eq!(got["counts"]["done_with_refusals"], 1);
    let repairable: Vec<(Value, Value)> = rows(&got)
        .iter()
        .map(|row| {
            (
                row["hole"]["reason"].clone(),
                row["hole"]["repairable"].clone(),
            )
        })
        .filter(|(reason, _)| !reason.is_null())
        .collect();
    assert_eq!(
        repairable,
        [
            (json!("needs_month_generation"), json!(true)),
            (json!("month_superseded"), json!(true)),
            (json!("repair_refused"), json!(true)),
            (json!("bars_differ_held"), json!(false)),
            (json!("repair_impossible"), json!(false)),
        ]
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A HOLE CLOSED AGAIN IS NO LONGER A HOLE, AND A TORN ONE NEVER WAS**
/// (duplicate, out-of-order and torn closes, and a crash resumed, together):
/// the last close of a key wins, so a hole a repair run closed `done` is
/// done, a done day closed again as a hole is a hole, a hole begun again by
/// a resumed run is open, and a hole line a crash cut is counted as torn and
/// leaves the key as its line before it said.
#[test]
fn a_hole_closed_again_is_no_longer_a_hole_and_a_torn_one_never_was() {
    let hole = "stage=file reason=repair_refused";
    let journal = [
        key("incomplete", "2019-05-02", "*+NSE-INDEX-NIFTYIT", hole),
        key("done", "2019-05-02", "*+NSE-INDEX-NIFTYIT", ""),
        key("done", "2019-05-03", "*+NSE-INDEX-NIFTYIT", ""),
        key("incomplete", "2019-05-03", "*+NSE-INDEX-NIFTYIT", hole),
        key("incomplete", "2019-05-06", "*+NSE-INDEX-NIFTYIT", hole),
        "begin indices 2019-05-06 *+NSE-INDEX-NIFTYIT\n".to_owned(),
        key("done", "2019-05-07", "*", ""),
        format!("incomplete indices 2019-05-07 * definition=2 {hole} (torn)\n"),
    ]
    .concat();
    let root = store("closed-again", &journal, "");
    let got = answer(&root, &ask("feed=gdfl&order=asc")).expect("reads");
    let standing: Vec<(&str, &str, bool)> = rows(&got)
        .iter()
        .map(|row| {
            (
                row["day"].as_str().unwrap(),
                row["outcome"].as_str().unwrap(),
                !row.get("hole").expect("hole").is_null(),
            )
        })
        .collect();
    assert_eq!(
        standing,
        [
            ("2019-05-02", "done", false),
            ("2019-05-03", "incomplete", true),
            ("2019-05-06", "open", false),
            ("2019-05-07", "done", false),
        ]
    );
    assert_eq!(got["holes"]["repair_refused"], 1);
    assert_eq!(got["journal"]["torn"]["count"], 1);
    let _ = std::fs::remove_dir_all(&root);
}

/// **A `+` WITH NOTHING ON ONE SIDE NAMES NO MEMBER** (garbage keys, the
/// case no list names): `*+` and `+NSE-INDEX-NIFTYIT` are base keys of their
/// own text, which name no symbol, so the day band and the counts treat them
/// as filters of nobody; a second `+` stays inside the member, which then
/// names no symbol either.
#[test]
fn a_plus_with_nothing_on_one_side_names_no_member() {
    assert_eq!(member_key("*+"), None);
    assert_eq!(member_key("+NSE-INDEX-NIFTYIT"), None);
    assert_eq!(member_key("*"), None);
    assert_eq!(member_key("BANKNIFTY,NIFTY"), None);
    assert_eq!(
        member_key("*+NSE-INDEX-NIFTYIT+X"),
        Some(("*", "NSE-INDEX-NIFTYIT+X"))
    );
    let journal = [
        key(
            "incomplete",
            "2019-06-03",
            "*+",
            "stage=file reason=needs_month_generation",
        ),
        key("done", "2019-06-03", "+NSE-INDEX-NIFTYIT", ""),
        key("done", "2019-06-03", "*+NSE-INDEX-NIFTYIT+X", ""),
    ]
    .concat();
    let root = store("garbage-keys", &journal, "");
    let got = answer(&root, &ask("feed=gdfl&order=asc")).expect("reads");
    let members: Vec<(Value, Value)> = rows(&got)
        .iter()
        .map(|row| (row["base_filter"].clone(), row["member"].clone()))
        .collect();
    assert_eq!(
        members,
        [
            (json!("*+"), Value::Null),
            (json!("*"), json!("NSE-INDEX-NIFTYIT+X")),
            (json!("+NSE-INDEX-NIFTYIT"), Value::Null),
        ]
    );
    assert_eq!(
        got["holes"]["needs_month_generation"], 1,
        "a hole of a key of nobody"
    );
    let band = day_band(
        &root,
        "feed=gdfl&instrument=NSE-INDEX-NIFTYIT&from=2019-06-03&to=2019-06-03",
    );
    assert_eq!(
        band_day(&band, "2019-06-03")["import"]["outcome"],
        "none",
        "no garbage key is NIFTYIT's: {band:#}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A MEMBER NAMES ITS SYMBOL BY `pull`'S ONE RULE** (renamed series): each
/// member is made by `pull`'s own `CmScope::member_of`, and maps to the
/// store symbol it is filed under: an index member to its store key, a share
/// member to its share, a `.BE` or `.BZ` series to the share whose day it
/// is. A name `pull` gives no series stays whole.
#[test]
fn a_member_names_its_symbol_by_pulls_one_rule() {
    let scope = pull::gdfl_cm::CmScope::stored(["RUCHI", "KPIT"]).expect("a scope");
    for (kind, stem, symbol) in [
        (
            pull::gdfl_cm::CmKind::Indices,
            "NIFTY IT.NSE_IDX",
            "NIFTYIT",
        ),
        (pull::gdfl_cm::CmKind::Stocks, "KPIT.NSE", "KPIT"),
        (pull::gdfl_cm::CmKind::Stocks, "RUCHI.BE.NSE", "RUCHI"),
    ] {
        let member = scope.member_of(kind, stem).expect("a member");
        assert_eq!(member_symbol(&member), symbol, "{member}");
    }
    assert_eq!(member_symbol("NSE-CASH-ZZSHARE.BZ"), "ZZSHARE");
    assert_eq!(member_symbol("NSE-CASH-ZZSHARE.SM"), "ZZSHARE.SM");
    assert_eq!(member_symbol("ZZBARE"), "ZZBARE");
}

/// **MEMBER HOLES ACROSS A YEAR'S END COUNT IN THEIR OWN RANGE** (a year
/// boundary, a member key and a hole at once): a December hole and a
/// January hole of one member are each counted only in the range that holds
/// their day, and both in the whole journal.
#[test]
fn member_holes_across_a_year_end_count_in_their_own_range() {
    let hole = "stage=file reason=repair_impossible";
    let journal = [
        key("incomplete", "2019-12-31", "*+NSE-INDEX-NIFTYIT", hole),
        key("incomplete", "2020-01-01", "*+NSE-INDEX-NIFTYIT", hole),
    ]
    .concat();
    let root = store("year-end", &journal, "");
    for (query, n) in [
        ("feed=gdfl&from=2019-12-01&to=2019-12-31", 1),
        ("feed=gdfl&from=2020-01-01&to=2020-01-31", 1),
        ("feed=gdfl", 2),
    ] {
        let got = answer(&root, &ask(query)).expect("reads");
        assert_eq!(got["holes"]["repair_impossible"], n, "{query}");
        assert_eq!(got["holes_unrepairable"], n, "{query}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// **AN EMPTY OR ABSENT JOURNAL SAYS ZERO HOLES, BY NAME**: every reason is
/// listed with 0 and `holes_unrepairable` is 0, so an answer with no hole
/// says so rather than saying nothing; the summary the same.
#[test]
fn an_empty_or_absent_journal_says_zero_holes_by_name() {
    let empty = store("empty", "", "");
    let absent = crate::scratch::path("ia10a-absent");
    let _ = std::fs::remove_dir_all(&absent);
    std::fs::create_dir_all(&absent).expect("a scratch root");
    for root in [&empty, &absent] {
        for got in [
            answer(root, &ask("feed=gdfl")).expect("reads"),
            summary(root),
        ] {
            assert_eq!(holes(&got).len(), 5, "{got:#}");
            assert!(holes(&got).iter().all(|(_, n)| *n == 0));
            assert_eq!(got["holes_unrepairable"], 0);
        }
    }
    let _ = std::fs::remove_dir_all(&empty);
    let _ = std::fs::remove_dir_all(&absent);
}

/// **AN OPEN MEMBER KEY IS ONE MEMBER'S DAY, NOT THE SEGMENT'S** (item 4,
/// D-TBD-IA-10a-1): the open-day scan keeps its answer for base keys, an
/// open `*` day is listed and an open member key is not, and the reach's
/// `from` counts the open member keys and names the first.
#[test]
fn an_open_member_key_is_named_and_is_not_the_segments_open_day() {
    let journal = [
        key("done", "2019-07-10", "*", ""),
        "begin indices 2019-07-02 *\n".to_owned(),
        "begin indices 2019-07-03 *+NSE-INDEX-NIFTYIT\n".to_owned(),
        "begin indices 2019-07-04 *+NSE-INDEX-NIFTYNEXT50\n".to_owned(),
    ]
    .concat();
    let root = store("open-member", &journal, "");
    let epoch = |text: &str| i64::from(parse_day(text).expect("a day").days_from_epoch());
    let reach = crate::server::feeds_route::imported_to(
        &root,
        brutex_core::vendor::Vendor::Gdfl,
        "INDEX",
        (epoch("2019-07-01"), epoch("2019-07-31")),
    );
    assert_eq!(reach.to, Some(epoch("2019-07-10")));
    assert_eq!(reach.open, [epoch("2019-07-02")], "{}", reach.from);
    assert!(
        reach.from.contains(
            "2 open member key(s) in these days (the first: `indices 2019-07-03 \
             *+NSE-INDEX-NIFTYIT`)"
        ),
        "{}",
        reach.from
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **THE DAY BAND DECODES NO `run` LINE, WHATEVER THE JOURNAL HOLDS**
/// (D-4896; L-IA-39): over 10,000 `run` lines, a day band whose refusals
/// carry their runs decodes none, where it decoded every one before.
#[test]
fn the_day_band_decodes_no_run_line_whatever_the_journal_holds() {
    let mut journal = String::with_capacity(10_100 * 64);
    for id in 1..=10_000_u64 {
        journal.push_str(&run_line(1_700_000_000 + id, "zz"));
    }
    journal.push_str(&key(
        "incomplete",
        "2019-08-05",
        "*",
        "stage=file reason=filing run=1700010000",
    ));
    let ring = refused(
        1,
        1_700_010_000_000 + 5,
        Some(1_700_009_999),
        "2019-08-05",
        "NIFTY 50.NSE_IDX.csv",
        "zz an earlier run's",
    );
    let root = store("no-decode", &journal, &ring);
    let _before = RUN_LINES_DECODED.with(|count| count.replace(0));
    let band = day_band(
        &root,
        "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2019-08-05&to=2019-08-05",
    );
    assert_eq!(RUN_LINES_DECODED.with(std::cell::Cell::get), 0);
    let old = &band_day(&band, "2019-08-05")["import"]["refusals"]["superseded"][0];
    assert_eq!(old["run"], 1_700_009_999_u64, "{old:#}");
    assert_eq!(old["run_from"], "event");
    let _ = std::fs::remove_dir_all(&root);
}

/// `key`, for any import kind.
fn kind_key(kind: &str, close: &str, day: &str, filter: &str, tail: &str) -> String {
    key(close, day, filter, tail).replace(" indices ", &format!(" {kind} "))
}

/// **A SHARE MEMBER'S HOLE IS ITS SHARE'S, A SERIES' TOO, AND AN OPTIONS
/// CLOSE UNDER A MEMBER KEY IS SHOWN, NEVER READ AS A MEMBER TREE** (a share
/// member, a `.BE` series and a hole at once; and the out-of-the-box case no
/// writer makes: `pull` writes member keys for the capital-market kinds only,
/// so an `options` close under one is a hand-edited or foreign line):
///
/// * the share's day band shows `*+NSE-CASH-ZZSHARE`'s hole on the day the
///   `*` close is done, and the `.BE` series' hole on its own day;
/// * the option's day band shows the options close under its own key with
///   its member named and no hole, and reads no member tree for it (an option
///   is never a member);
/// * `/imports.json`: a refusal about a file that names no swept symbol, on a
///   stocks day whose only close is a member key, is that close's (its base
///   is `*`), so no log-only `*` row is made up for the day.
#[test]
fn a_share_members_hole_is_its_shares_and_an_options_member_key_is_shown_not_read() {
    let hole = "stage=file reason=repair_impossible";
    let journal = [
        kind_key("stocks", "done", "2019-03-01", "*", ""),
        kind_key(
            "stocks",
            "incomplete",
            "2019-03-01",
            "*+NSE-CASH-ZZSHARE",
            hole,
        ),
        kind_key(
            "stocks",
            "incomplete",
            "2019-03-05",
            "*+NSE-CASH-ZZSHARE.BE",
            hole,
        ),
        kind_key("options", "done", "2019-03-01", "*+NSE-INDEX-NIFTY", ""),
    ]
    .concat();
    let ring = refused(
        1,
        1_000,
        None,
        "2019-03-05",
        "ZZNOWHERE.NSE.csv",
        "zz unknown",
    )
    .replace("\"kind\":\"indices\"", "\"kind\":\"stocks\"");
    let root = store("share-option", &journal, &ring);
    let unrepairable = json!({ "reason": "repair_impossible", "repairable": false });

    let share = day_band(
        &root,
        "feed=gdfl&instrument=NSE-CASH-ZZSHARE&from=2019-03-01&to=2019-03-05",
    );
    for (on, member) in [
        ("2019-03-01", "NSE-CASH-ZZSHARE"),
        ("2019-03-05", "NSE-CASH-ZZSHARE.BE"),
    ] {
        let import = &band_day(&share, on)["import"];
        assert_eq!(import["member"], member, "{on}: {import:#}");
        assert_eq!(import["outcome"], "incomplete", "{on}");
        assert_eq!(import["hole"], unrepairable, "{on}");
    }
    assert_eq!(band_day(&share, "2019-03-04")["import"]["outcome"], "none");

    let option = day_band(
        &root,
        "feed=gdfl&instrument=NSE-FNO-NIFTY-2019-03-28-1150000-CE&from=2019-03-01&to=2019-03-01",
    );
    let import = &band_day(&option, "2019-03-01")["import"];
    assert_eq!(import["kind"], "options", "{import:#}");
    assert_eq!(import["filter"], "*+NSE-INDEX-NIFTY");
    assert_eq!(import["member"], "NSE-INDEX-NIFTY");
    assert_eq!(import["outcome"], "done");
    assert_eq!(import.get("hole"), Some(&Value::Null));

    let got = answer(
        &root,
        &ask("feed=gdfl&kind=stocks&from=2019-03-05&to=2019-03-05"),
    )
    .expect("reads");
    let shown: Vec<(&str, &str)> = rows(&got)
        .iter()
        .map(|row| {
            (
                row["filter"].as_str().unwrap(),
                row["outcome"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        shown,
        [("*+NSE-CASH-ZZSHARE.BE", "incomplete")],
        "no log-only `*` row is made up for a refusal the member key owns: {got:#}"
    );
    assert_eq!(got["holes"]["repair_impossible"], 1);
    assert_eq!(got["holes_unrepairable"], 1);
    let _ = std::fs::remove_dir_all(&root);
}

/// **A MEMBER-ONLY DAY OWNS ITS OWN MEMBER'S FILE AND NO OTHER'S** (a member
/// key, a swept symbol and a log-only row at once): on a day whose only close
/// is `*+NSE-INDEX-NIFTY`, a refusal of NIFTY's own file is that close's,
/// while a refusal of BANKNIFTY's file is no close's and keeps the day's
/// log-only `*` row, so a refusal another member's run never read is never
/// hidden under the member close beside it.
#[test]
fn a_member_only_day_owns_its_own_members_file_and_no_other() {
    let journal = key("done", "2019-03-04", "*+NSE-INDEX-NIFTY", "");
    let refusals = |about: &str| refused(1, 1_000, None, "2019-03-04", about, "zz refused");
    let shown = |name: &str, about: &str| {
        let root = store(name, &journal, &refusals(about));
        let got = answer(
            &root,
            &ask("feed=gdfl&kind=indices&from=2019-03-04&to=2019-03-04&order=asc"),
        )
        .expect("reads");
        let filters: Vec<(String, String)> = rows(&got)
            .iter()
            .map(|row| {
                (
                    row["filter"].as_str().unwrap().to_owned(),
                    row["outcome"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let _ = std::fs::remove_dir_all(&root);
        filters
    };
    let own = ("*+NSE-INDEX-NIFTY".to_owned(), "done".to_owned());
    assert_eq!(
        shown("own-file", "NIFTY 50.NSE_IDX.csv"),
        std::slice::from_ref(&own)
    );
    let other = shown("other-file", "NIFTY BANK.NSE_IDX.csv");
    assert_eq!(
        other,
        [("*".to_owned(), "refused".to_owned()), own],
        "another swept member's refusal is no member close's"
    );
}

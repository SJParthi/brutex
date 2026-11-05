//! What no other test of `logs.rs` reached, each row driven by its own input
//! and asserted on what the code answers there (BASE-COV-API-AS3). Every log
//! line here is invented: no vendor's row, no market figure.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::r3_24_edge_tests::{ask_ts, hand_stream, reads};
use super::{
    About, DAY_IMPORTED, DirEvidence, DirRefusals, DirState, Half, ImportRefusals, Indexed, Keys,
    MAX_KEYED_READS, NO_LOG_JSON, SCAN_BYTES, Scope, Slot, Stream, admitted, all_halves_live,
    asked, audited_import_refusals, bears, contradicted, gdfl_about, index_rows, instant_of,
    instrument_of, keyed_json, keyed_json_across, merge_streams, page_shell, probes, ring_scan,
    span_page, unknown_span_words,
};

/// A fresh scratch directory of this module's own.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = crate::scratch::path(&format!("logs-cover-{name}"));
    let _ignored = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// A sink on a fresh scratch directory, writing every level.
fn sink_in(name: &str) -> (std::path::PathBuf, telemetry::Sink) {
    let dir = scratch(name);
    let sink = telemetry::Sink::open(
        &telemetry::Config::new(&dir).with_min_level(telemetry::Level::Trace),
    )
    .expect("opens");
    (dir, sink)
}

/// One ring line as a sink writes it: `run` when given, at `ms`, with the
/// `fields` object's members written as given.
fn ring_line(
    seq: u64,
    run: Option<u64>,
    ms: i64,
    (level, target, message): (&str, &str, &str),
    fields: &str,
) -> String {
    let run = run.map_or_else(String::new, |run| format!("\"run\":{run},"));
    format!(
        "{{\"seq\":{seq},{run}\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":{ms},\
         \"level\":\"{level}\",\"target\":\"{target}\",\"msg\":\"{message}\",\
         \"fields\":{{{fields}}}}}\n"
    )
}

/// A record this module owns, at `error`, every field a string.
fn record(target: &str, message: &str, fields: &[(&str, &str)]) -> telemetry::Record {
    telemetry::Record {
        seq: 1,
        run: 0,
        at_unix_millis: 150,
        at_utc: String::new(),
        level: telemetry::Level::Error,
        target: target.to_owned(),
        message: message.to_owned(),
        fields: fields
            .iter()
            .map(|(key, value)| {
                (
                    (*key).to_owned(),
                    telemetry::OwnedValue::Str((*value).to_owned()),
                )
            })
            .collect(),
        cut: false,
        dropped_fields: 0,
    }
}

/// The `/logs.json` body of a keyed request over `dir` alone.
fn keyed(dir: &std::path::Path, query: &str) -> serde_json::Value {
    serde_json::from_str(&keyed_json(dir, None, &asked(query), None)).expect("JSON")
}

/// **THE PAGE'S FORM KEEPS EACH END OF A SPAN, AND ITS JSON LINK THE RING
/// QUESTION** (BASE-COV-API-AS3). An end left open is an empty field, never
/// the sentinel the request holds it as; a closed span shows both ends; and
/// `ring=1` rides the link to the JSON, which a request without it does not.
///
/// Extremes: singles — a span open at its end (`from` alone); a span open at
/// its start (`to` alone); `ring=1`; combinations — `ring=1` beside a span
/// open at its end, both carried at once; a closed span whose ends are one
/// day's first and last millisecond; out of the box — a `from` written as a
/// bare count of milliseconds, zero, the first instant there is.
#[test]
fn the_page_form_keeps_each_span_end_and_the_ring_question() {
    let first = instant_of("2024-01-02", false).expect("a day");
    let last = instant_of("2024-01-02", true).expect("a day");
    let field = |name: &str, value: &str| format!("name=\"{name}\" value=\"{value}\"");
    let open_end = page_shell(&asked("from=2024-01-02&ring=1"), "");
    assert!(
        open_end.contains(&field("from", &first.to_string())),
        "{open_end}"
    );
    assert!(open_end.contains(&field("to", "")), "{open_end}");
    assert!(
        open_end.contains(&format!("/logs.json?limit=50&from={first}&ring=1\"")),
        "{open_end}"
    );
    let open_start = page_shell(&asked("to=2024-01-02"), "");
    assert!(open_start.contains(&field("from", "")), "{open_start}");
    assert!(
        open_start.contains(&field("to", &last.to_string())),
        "{open_start}"
    );
    assert!(!open_start.contains("ring=1"), "{open_start}");
    let closed = page_shell(&asked("from=2024-01-02&to=2024-01-02"), "");
    assert!(
        closed.contains(&field("from", &first.to_string())),
        "{closed}"
    );
    assert!(closed.contains(&field("to", &last.to_string())), "{closed}");
    let epoch = page_shell(&asked("from=0"), "");
    assert!(epoch.contains(&field("from", "0")), "{epoch}");
    assert!(epoch.contains(&field("to", "")), "{epoch}");
}

/// **A LIVE COPY READ UNDER A TARGET OR A RUN COUNTS NO GAP**
/// (BASE-COV-API-AS3). A gap in `seq` among the lines a filter kept means
/// nothing, so `missing` is `None` under a target alone and under a run
/// alone, as it is under a level; the copy still answers from memory with no
/// byte read, its file deleted.
///
/// Extremes: singles — a target alone; a run alone; combinations — a target
/// beneath the lines' own and a run at once; a limit of one under a run;
/// out of the box — a run no line carries, which the copy cannot fill, so the
/// files answer and the ring's file is gone: an empty answer, never the
/// unfiltered copy.
#[test]
fn a_live_copy_read_under_a_target_or_a_run_counts_no_gap() {
    let (dir, sink) = sink_in("live-filters");
    for n in 0..12_u32 {
        let event = telemetry::Event::info("a.b", "line").with("n", n);
        assert_eq!(sink.emit_for_run(7, &event), telemetry::Emitted::Written);
    }
    std::fs::remove_file(telemetry::current_path(&dir)).expect("the ring file");
    let halves = [Half {
        name: "served",
        path: dir.clone(),
    }];
    for query in [
        "limit=5&target=a",
        "limit=5&run=7",
        "limit=3&target=a.b&run=7",
        "limit=1&run=7",
    ] {
        let question = asked(query);
        let tail = all_halves_live(&halves, &question.query, Some(&sink));
        assert_eq!(tail.records.len(), question.limit, "{query}");
        assert_eq!((tail.bytes_read, tail.files_read), (0, 0), "{query}");
        assert_eq!(tail.missing, None, "{query}: a gap means nothing here");
    }
    let none = all_halves_live(&halves, &asked("limit=5&run=8").query, Some(&sink));
    assert!(none.records.is_empty(), "{:?}", none.records);
    drop(sink);
    let _ignored = std::fs::remove_dir_all(&dir);
}

/// **A LINE THE KEYS DO NOT MEET BEARS ON NOTHING** (BASE-COV-API-AS3). A
/// line with no day and another month's is no answer to a day; another run's
/// line no answer to a run; a line stamped either side of a span no answer to
/// it. An option file the importer's reader cannot decode is a file of no
/// instrument; a span whose last time is unknown says so once the file is
/// closed; a level floor keeps a quieter line out; and a "day imported" line
/// beside no store is marked nothing.
///
/// Extremes: singles — another month's line asked by day; another run's line;
/// a line one millisecond before a span and one after; a level floor above
/// the line; combinations — an option ticker that does not decode ending
/// `CE`, and one ending `PE`, each with a valid day and kind; a closed file
/// whose first and last times are both unknown; out of the box — the words
/// of a scope no request names (`day`), and a "day imported" line checked
/// against no store.
#[test]
fn a_line_the_keys_do_not_meet_bears_on_nothing() {
    use telemetry::run_dirs::AuditSpan;
    assert_eq!(
        [Scope::Event, Scope::Day, Scope::Month].map(Scope::as_str),
        ["event", "day", "month"]
    );
    let option = |about: &str| {
        record(
            pull::gdfl_import::TARGET,
            "refused",
            &[("kind", "options"), ("day", "2024-01-02"), ("about", about)],
        )
    };
    assert_eq!(gdfl_about(&option("NOTATICKERCE")), Some(About::Unfiled));
    assert_eq!(gdfl_about(&option("NOTATICKERPE")), Some(About::Unfiled));
    assert!(matches!(
        gdfl_about(&option("census")),
        Some(About::WholeDay { .. })
    ));
    let day = crate::server::import_journal::parse_day("2024-01-02").expect("a day");
    let by_day = Keys {
        day: Some(day),
        ..Keys::default()
    };
    let month = |held: &str| record("pull.spot", "member did not land", &[("month", held)]);
    assert_eq!(bears(&month("2024-02"), &by_day), None);
    assert_eq!(bears(&month("2024-01"), &by_day), Some(Scope::Month));
    let by_run = Keys {
        run: Some(7),
        ..Keys::default()
    };
    let mut ran = record("cli.sweep", "rung entered", &[]);
    ran.run = 3;
    assert_eq!(bears(&ran, &by_run), None);
    ran.run = 7;
    assert_eq!(bears(&ran, &by_run), Some(Scope::Event));
    let by_span = Keys {
        span: Some((100, 200)),
        ..Keys::default()
    };
    for (at, answer) in [
        (99, None),
        (100, Some(Scope::Event)),
        (200, Some(Scope::Event)),
        (201, None),
    ] {
        ran.at_unix_millis = at;
        assert_eq!(bears(&ran, &by_span), answer, "at {at}");
    }
    let span = |first_ms, last_ms| AuditSpan { first_ms, last_ms };
    assert_eq!(
        unknown_span_words(&span(Some(1), None), false).as_deref(),
        Some("its last line's time cannot be read")
    );
    assert_eq!(
        unknown_span_words(&span(None, None), false).as_deref(),
        Some("its first line's time cannot be read and its last line's time cannot be read")
    );
    assert_eq!(unknown_span_words(&span(Some(1), None), true), None);
    let mut quiet = record("a.b", "said", &[]);
    quiet.level = telemetry::Level::Info;
    assert!(!admitted(&quiet, &asked("level=error")));
    assert!(admitted(&quiet, &asked("level=info")));
    let imported = record(
        pull::gdfl_import::TARGET,
        DAY_IMPORTED,
        &[("kind", "indices"), ("day", "2024-01-02")],
    );
    let marks = contradicted(&[imported, month("2024-02")], None);
    assert_eq!(marks.len(), 2);
    assert!(marks.iter().all(Option::is_none), "{marks:?}");
}

/// **EACH KEY PROBES ITS OWN POSTING** (BASE-COV-API-AS3). A contract or an
/// instrument beside a feed that is not GDFL's probes no importer target; a
/// feed alone probes its own posting, and GDFL's the importer's target too;
/// no key probes nothing.
///
/// Extremes: singles — a contract beside a non-GDFL feed; an instrument
/// beside one; a feed alone; no key at all; combinations — a contract beside
/// GDFL's feed (its target probed too); an instrument asked in its long form
/// beside a non-GDFL feed (both spellings probed, no target); out of the box —
/// a feed with no other key, which names a posting of its own.
#[test]
fn each_key_probes_its_own_posting() {
    let gdfl = brutex_core::vendor::Vendor::Gdfl.as_str();
    let zerodha = brutex_core::vendor::Vendor::Zerodha.as_str();
    let target = pull::gdfl_import::TARGET.to_owned();
    let contract = |feed: &str| Keys {
        contract: Some("FIXTURE-CONTRACT".to_owned()),
        feed: Some(feed.to_owned()),
        ..Keys::default()
    };
    assert_eq!(
        probes(&contract(zerodha)),
        [("contract", "FIXTURE-CONTRACT".to_owned())]
    );
    assert_eq!(
        probes(&contract(gdfl)),
        [
            ("contract", "FIXTURE-CONTRACT".to_owned()),
            ("target", target.clone())
        ]
    );
    let instrument = Keys {
        instrument: instrument_of("NSE-INDEX-NIFTY"),
        feed: Some(zerodha.to_owned()),
        ..Keys::default()
    };
    assert_eq!(
        probes(&instrument),
        [
            ("instrument", "NIFTY".to_owned()),
            ("instrument", "NSE-INDEX-NIFTY".to_owned())
        ]
    );
    for (feed, posted) in [
        (gdfl, vec![("feed", gdfl.to_owned()), ("target", target)]),
        (zerodha, vec![("feed", zerodha.to_owned())]),
    ] {
        let alone = Keys {
            feed: Some(feed.to_owned()),
            ..Keys::default()
        };
        assert_eq!(probes(&alone), posted, "{feed}");
    }
    assert!(probes(&Keys::default()).is_empty(), "no key, no posting");
}

/// **A RING SCAN NAMES WHAT IT CANNOT READ, PASSES AN EMPTY FILE AND STOPS AT
/// ITS BUDGET** (BASE-COV-API-AS3).
///
/// Extremes: singles — a file where the ring's directory should be (every
/// ring path under it refused by name); an empty ring file (passed over,
/// unread); a budget of zero (capped, nothing read); a line that is no record
/// (skipped); combinations — a budget exactly the newest file's length with an
/// older file left (capped at the older file); a folder where the newest
/// ring file should be beside an older file that holds a line (named, and the
/// older file still read); out of the box — the folder in the file's place.
#[test]
fn a_ring_scan_names_what_it_cannot_read_and_stops_at_its_budget() {
    let root = scratch("ring-scan");
    let keep = |record: &telemetry::Record| (record.message == "kept").then_some(Scope::Event);
    let kept = ring_line(1, None, 1_000, ("error", "ring.test", "kept"), "");
    let passed = ring_line(2, None, 1_001, ("error", "ring.test", "passed over"), "");
    let file = root.join("a-file");
    std::fs::write(&file, &kept).expect("a file");
    let under_a_file = ring_scan(&file, SCAN_BYTES, keep);
    let named = file.display().to_string();
    assert!(
        !under_a_file.errors.is_empty(),
        "every ring path is refused"
    );
    assert!(
        under_a_file.errors.iter().all(|why| why.contains(&named)),
        "{:?}",
        under_a_file.errors
    );
    assert_eq!((under_a_file.files_read, under_a_file.found.len()), (0, 0));
    let empty = root.join("empty");
    std::fs::create_dir_all(&empty).expect("a ring directory");
    std::fs::write(telemetry::current_path(&empty), b"").expect("an empty ring file");
    let scanned = ring_scan(&empty, SCAN_BYTES, keep);
    assert_eq!(
        (scanned.files_read, scanned.bytes_read, scanned.hit_cap),
        (0, 0, false)
    );
    assert!(scanned.errors.is_empty(), "{:?}", scanned.errors);
    let full = root.join("full");
    std::fs::create_dir_all(&full).expect("a ring directory");
    std::fs::write(
        telemetry::current_path(&full),
        format!("{kept}not a record\n{passed}"),
    )
    .expect("a ring file");
    let starved = ring_scan(&full, 0, keep);
    assert!(starved.hit_cap, "no budget is a capped scan");
    assert_eq!((starved.files_read, starved.found.len()), (0, 0));
    let whole = ring_scan(&full, SCAN_BYTES, keep);
    assert_eq!((whole.files_read, whole.found.len()), (1, 1));
    assert!(!whole.hit_cap);
    let two = root.join("two");
    std::fs::create_dir_all(&two).expect("a ring directory");
    std::fs::write(telemetry::current_path(&two), &kept).expect("the newest file");
    std::fs::write(telemetry::rotated_path(&two, 1), &kept).expect("an older file");
    let exact = u64::try_from(kept.len()).expect("a length");
    let edge = ring_scan(&two, exact, keep);
    assert!(edge.hit_cap, "the older file is past the budget");
    assert_eq!((edge.files_read, edge.found.len()), (1, 1));
    let odd = root.join("odd");
    std::fs::create_dir_all(telemetry::current_path(&odd)).expect("a folder in its place");
    std::fs::write(telemetry::current_path(&odd).join("inside"), b"x").expect("not empty");
    std::fs::write(telemetry::rotated_path(&odd, 1), &kept).expect("an older file");
    let past = ring_scan(&odd, SCAN_BYTES, keep);
    assert_eq!(past.errors.len(), 1, "{:?}", past.errors);
    assert!(
        past.errors[0].contains(&telemetry::current_path(&odd).display().to_string()),
        "{:?}",
        past.errors
    );
    assert_eq!(past.found.len(), 1, "the older file is still read");
    let _ignored = std::fs::remove_dir_all(&root);
}

/// **A LINE IN THE INDEX AND THE RING IS SHOWN ONCE, AND RING ROWS ARE NEWEST
/// FIRST** (BASE-COV-API-AS3). With `ring=1` a line the audit stream also
/// holds is answered from the index alone; lines only the ring holds are
/// answered newest first; a keyed question over no directory says so.
///
/// Extremes: singles — one line in both the stream and the ring; three lines
/// in the ring alone; combinations — the ring read on a first page beside an
/// index that holds the same line; out of the box — a keyed question with no
/// log directory at all.
#[test]
fn a_line_in_the_index_and_the_ring_is_shown_once_and_ring_rows_newest_first() {
    let refusal = |about: &'static str| {
        telemetry::Event::error("gdfl.import", "refused")
            .with("kind", "indices")
            .with("day", "2022-03-07")
            .with("about", about)
            .with("why", "an invented fault")
    };
    let (both, sink) = sink_in("ring-and-index");
    let _ = sink.emit(&refusal("FIXTURE ONE.NSE_IDX.csv"));
    drop(sink);
    let got = keyed(&both, "feed=gdfl&day=2022-03-07&ring=1");
    assert_eq!(
        got["records"].as_array().expect("records").len(),
        1,
        "{got:#}"
    );
    assert_eq!(got["keyed"]["rows"][0]["source"], "audit", "{got:#}");
    assert_eq!(got["keyed"]["ring"]["scanned"], true, "{got:#}");
    let ring_only = scratch("ring-only");
    {
        let sink = telemetry::Sink::open(&telemetry::Config::new(&ring_only).with_audit(false))
            .expect("opens");
        for about in ["FIXTURE A.csv", "FIXTURE B.csv", "FIXTURE C.csv"] {
            let _ = sink.emit(&refusal(about));
        }
    }
    let got = keyed(&ring_only, "feed=gdfl&day=2022-03-07&ring=1");
    let seqs: Vec<u64> = got["records"]
        .as_array()
        .expect("records")
        .iter()
        .map(|record| record["seq"].as_u64().expect("a seq"))
        .collect();
    assert_eq!(seqs.len(), 3, "{got:#}");
    assert!(
        seqs.iter()
            .zip(seqs.iter().skip(1))
            .all(|(newer, older)| newer > older),
        "{seqs:?}"
    );
    assert_eq!(
        keyed_json_across(&[], &asked("feed=gdfl&day=2022-03-07"), None),
        NO_LOG_JSON
    );
    let _ignored = std::fs::remove_dir_all(&both);
    let _ignored = std::fs::remove_dir_all(&ring_only);
}

/// **A SPAN READ OVER A FOLDER WHERE ITS AUDIT FILE SHOULD BE IS REFUSED BY
/// NAME** (BASE-COV-API-AS3). The walk opens the folder and cannot read it;
/// the search's first probe cannot either; through the keyed answer the
/// stream is named in `errors`, never read as empty, and its cursor offers
/// the stream again from its newest.
///
/// Extremes: singles — a span open at its end; a span closed at its end;
/// combinations — the folder reached through a whole keyed answer, its rows
/// empty and its cursor kept; out of the box — a folder, with a file in it,
/// where the audit stream's current file is named.
#[test]
fn a_span_over_a_folder_where_its_audit_file_should_be_is_refused_by_name() {
    let dir = scratch("span-over-a-folder");
    let current = telemetry::audit_current_path(&dir);
    std::fs::create_dir(&current).expect("a folder in the file's place");
    std::fs::write(current.join("inside"), b"x").expect("so the folder holds something");
    let named = current.display().to_string();
    let walked = span_page(&dir, (0, i64::MAX), 10, None).expect_err("no audit file");
    assert!(walked.contains(&named), "{walked}");
    let searched = span_page(&dir, (0, 1_000), 10, None).expect_err("no audit file");
    assert!(searched.contains(&named), "{searched}");
    let indexed = index_rows(&[("served", "audit", dir.as_path())], &asked("from=0"));
    assert!(indexed.rows.is_empty());
    assert_eq!(indexed.errors.len(), 1, "{:?}", indexed.errors);
    assert!(indexed.errors[0].contains(&named), "{:?}", indexed.errors);
    assert_eq!(indexed.next.as_deref(), Some("*"), "offered again");
    let _ignored = std::fs::remove_dir_all(&dir);
}

/// **A SEARCH STOPPED BY A LINE THAT WILL NOT READ, OR BY NO BUDGET LEFT,
/// STILL ANSWERS WHAT IT CAN AND SAYS SO** (BASE-COV-API-AS3).
///
/// Forty lines a millisecond apart, the span 5..=9 beside the day key: the
/// search passes the newer lines by a gallop and a bisection. With the
/// bisection's first probe torn, it stops there and the walk reads on from
/// the bracket, meeting and counting the torn line: the same five lines are
/// answered, fewer passed by search. Then a first half whose 2,000 pointers
/// all point into an emptied file spends the request's reads before the
/// second half's search begins: that search reads nothing, and the answer is
/// capped with a cursor.
///
/// Extremes: singles — the bisection's first probe torn; the read budget
/// spent before a stream's first probe; combinations — a torn line inside the
/// bracket the gallop made while the walk still finds every line of the span;
/// two halves, the first spending every read on lines that rolled off;
/// out of the box — an audit file emptied under its 2,000 pointers.
#[test]
fn a_search_stopped_by_a_line_that_will_not_read_or_by_no_budget_still_answers() {
    let times: Vec<i64> = (1..=40).collect();
    let intact = scratch("bisect-intact");
    hand_stream(&intact, &[&times]);
    let torn = scratch("bisect-torn");
    hand_stream(&torn, &[&times]);
    let file = telemetry::audit_path_for(&torn, 0, 0).expect("the file");
    let mut text = std::fs::read(&file).expect("the file");
    let seventeenth: usize = text
        .split_inclusive(|byte| *byte == b'\n')
        .take(16)
        .map(<[u8]>::len)
        .sum();
    *text.get_mut(seventeenth).expect("line 17") = b'#';
    std::fs::write(&file, text).expect("line 17 torn");
    let query = "day=2020-01-02&from=5&to=9&limit=50";
    let (whole, whole_ts) = ask_ts(&intact, query);
    let (stopped, stopped_ts) = ask_ts(&torn, query);
    assert_eq!(whole_ts, [9, 8, 7, 6, 5], "{:#}", whole["keyed"]);
    assert_eq!(stopped_ts, whole_ts, "{:#}", stopped["keyed"]);
    assert_eq!(
        whole["keyed"]["index"]["passed"], 34,
        "{:#}",
        whole["keyed"]
    );
    assert_eq!(whole["keyed"]["index"]["rolled_off"], 0);
    assert_eq!(
        stopped["keyed"]["index"]["passed"], 19,
        "the search stopped at the torn line: {:#}",
        stopped["keyed"]
    );
    assert_eq!(
        stopped["keyed"]["index"]["rolled_off"], 1,
        "met and counted"
    );
    let spent = scratch("budget-spent-served");
    let late = scratch("budget-spent-cli");
    let many: Vec<i64> = (1..=2_000).collect();
    hand_stream(&spent, &[&many]);
    std::fs::write(
        telemetry::audit_path_for(&spent, 0, 0).expect("a file"),
        b"",
    )
    .expect("emptied");
    hand_stream(&late, &[&[5, 6, 30]]);
    let question = asked("day=2020-01-02&from=1&to=20&limit=5");
    let got: serde_json::Value =
        serde_json::from_str(&keyed_json(&spent, Some(&late), &question, None)).expect("JSON");
    let index = &got["keyed"]["index"];
    assert!(
        got["records"].as_array().expect("records").is_empty(),
        "{index:#}"
    );
    assert_eq!(reads(&got), 2_000, "{index:#}");
    assert_eq!(index["capped"], true, "{index:#}");
    assert_eq!(index["rolled_off"], 1_999, "{index:#}");
    let second = index["postings"]
        .as_array()
        .expect("postings")
        .iter()
        .filter(|posting| posting["half"] == "cli")
        .find(|posting| posting["source"] == "audit")
        .expect("the second half's stream");
    assert_eq!(second["read"], 0, "{index:#}");
    assert!(got["keyed"]["next"].is_string(), "{:#}", got["keyed"]);
    for dir in [intact, torn, spent, late] {
        let _ignored = std::fs::remove_dir_all(dir);
    }
}

/// One pointer of a hand-built stream, its place's index its `seq`.
fn pointer(serial: u64, seq: u64) -> (telemetry::PostingAt, telemetry::AuditPointer) {
    (
        telemetry::PostingAt { serial, index: seq },
        telemetry::AuditPointer {
            serial,
            offset: 0,
            seq,
            len: 1,
            level: telemetry::Level::Error,
        },
    )
}

/// A record of `seq` stamped `ms`, as a stream's head holds it.
fn line_at(seq: u64, ms: i64) -> telemetry::Record {
    let mut line = record("hand.test", "held", &[]);
    line.seq = seq;
    line.at_unix_millis = ms;
    line
}

/// A stream of the `served` half built by hand: its fetched page and head
/// as given, nothing read yet.
fn hand<'d>(
    source: &'static str,
    dir: &'d std::path::Path,
    pointers: Vec<(telemetry::PostingAt, telemetry::AuditPointer)>,
    head: Option<(telemetry::AuditPointer, telemetry::Record)>,
) -> Stream<'d> {
    Stream {
        half: "served",
        source,
        dir,
        prune: None,
        spans: std::collections::HashMap::new(),
        placed: false,
        serial: 1,
        dimension: "day",
        value: "2020-01-02".to_owned(),
        page: telemetry::PostingsPage {
            pointers,
            ..telemetry::PostingsPage::default()
        },
        next: 0,
        last: None,
        from: Slot::Newest,
        more: false,
        head,
        read: 0,
        passed: 0,
        resume: None,
        damage: Vec::new(),
        unreadable: 0,
        unread_at: None,
    }
}

/// **A MERGE SHOWS A LINE ONCE, KEEPS ANOTHER FILE'S LINE, AND STOPS AT A
/// HEAD WITH NO PLACE** (BASE-COV-API-AS3). Streams built by hand, through
/// the merge itself: the audit stream's line is shown; the same line at the
/// head of the half's state stream, which holds no place for it, is not
/// consumed there and not shown again; an older line of another audit file
/// is not taken for it; and when the next head chosen has no place, the
/// merge stops rather than show it. A pass past the fetched page moves
/// nothing.
///
/// Extremes: singles — a head with no place in its page; a pass to an index
/// past the page; combinations — the same line at two streams' heads, one of
/// them placeless; one source's two streams whose heads are in different
/// files; out of the box — streams no posting produced, merged directly.
#[test]
fn a_merge_shows_a_line_once_and_stops_at_a_head_with_no_place() {
    let dir = scratch("hand-merge");
    let (a_at, a) = pointer(1, 10);
    let (_, b) = pointer(0, 10);
    let (c_at, c) = pointer(0, 4);
    let mut streams = [
        hand(
            "audit",
            &dir,
            vec![(a_at, a)],
            Some((a, line_at(10, 1_000))),
        ),
        hand("state", &dir, Vec::new(), Some((b, line_at(10, 1_000)))),
        hand("audit", &dir, vec![(c_at, c)], Some((c, line_at(4, 500)))),
    ];
    let mut out = Indexed::default();
    let budget = merge_streams(&mut streams, &asked("limit=10"), &mut out);
    assert_eq!(budget, MAX_KEYED_READS, "no line was read");
    assert_eq!(out.rows.len(), 1, "{:?}", out.rows);
    assert_eq!(out.rows[0].3.seq, 10);
    assert!(!out.capped);
    let [first, second, third] = &streams;
    assert_eq!((first.next, first.read), (1, 1));
    assert!(second.head.is_none(), "taken, and with no place, not shown");
    assert_eq!(second.read, 0);
    assert!(
        third.head.is_some(),
        "another file's line is not the same line"
    );
    assert_eq!(third.read, 0);
    let mut past = hand("audit", &dir, vec![pointer(0, 1), pointer(0, 2)], None);
    past.pass_to(5);
    assert_eq!((past.next, past.read, past.passed), (0, 0, 0));
    past.pass_to(2);
    assert_eq!((past.next, past.read, past.passed), (2, 2, 2));
    let _ignored = std::fs::remove_dir_all(&dir);
}

/// **THE IMPORTER'S RING IS FOLDED LINE BY LINE, EACH FILED WHERE ITS ATTEMPT
/// AND ITS FIELDS PUT IT** (BASE-COV-API-AS3). A start with no run opens no
/// attempt; a finish of none open closes nothing; a run named twice is filed
/// once and one that is no number never; a day's line is filed under its
/// attempt once; an error with no day and a "day imported" with no kind are
/// read and kept nowhere; a line that will not decode is counted. A ring whose
/// fold stands inside an attempt it never saw opened files the day's line
/// under that attempt's id, keeps the refusal, and its finish closes it.
///
/// Extremes: singles — a start with no run; a finish of no open attempt; a
/// run field that is text; the same run twice; an error with no day; a "day
/// imported" with no kind; combinations — three importer lines of one day
/// under one attempt (filed once); a refusal and a finish under an attempt
/// the ring never opened; out of the box — an importer line cut mid-key.
#[test]
fn the_importers_ring_is_folded_and_each_line_filed_where_it_belongs() {
    let lifecycle = |seq: u64, run: Option<u64>, message: &str| {
        ring_line(
            seq,
            run,
            1_000,
            ("info", "cli.lifecycle", message),
            r#""command":"gdfl-import","phase":"completed""#,
        )
    };
    let importer = |seq: u64, level: &str, message: &str, fields: &str| {
        ring_line(seq, None, 1_000, (level, "gdfl.import", message), fields)
    };
    let day = r#""kind":"options","day":"2020-01-02""#;
    let mut held = DirRefusals::default();
    held.fold(lifecycle(1, None, "command started").as_bytes());
    assert!(held.current.is_none(), "a start with no run opens nothing");
    held.fold(lifecycle(2, Some(41), "command finished").as_bytes());
    assert!(
        held.attempt(41).is_none(),
        "a finish of none open files nothing"
    );
    held.fold(lifecycle(3, Some(42), "command started").as_bytes());
    held.fold(importer(4, "info", "run started", &format!(r#""run":"seven",{day}"#)).as_bytes());
    held.fold(importer(5, "info", "run started", &format!(r#""run":7,{day}"#)).as_bytes());
    held.fold(importer(6, "info", "run started", &format!(r#""run":7,{day}"#)).as_bytes());
    held.fold(importer(7, "error", "refused", r#""kind":"options""#).as_bytes());
    held.fold(importer(8, "info", DAY_IMPORTED, r#""day":"2020-01-02""#).as_bytes());
    held.fold(br#"{"seq":9,"target":"gdfl.import","ms"#);
    let attempt = held.attempt(42).expect("the open attempt");
    assert_eq!(attempt.importer_runs, [7]);
    assert_eq!(attempt.lines, 5, "every importer line that decoded");
    assert_eq!(held.attempts_of("options", "2020-01-02"), [42]);
    assert!(held.of("options", "2020-01-02").is_empty());
    assert_eq!(held.pace("options"), None);
    assert_eq!(held.malformed, 1);
    let mut orphan = DirRefusals {
        current: Some(9),
        ..DirRefusals::default()
    };
    orphan.fold(importer(1, "error", "refused", day).as_bytes());
    assert!(orphan.attempt(9).is_none());
    assert_eq!(orphan.attempts_of("options", "2020-01-02"), [9]);
    assert_eq!(orphan.of("options", "2020-01-02").len(), 1);
    orphan.fold(lifecycle(2, Some(9), "command finished").as_bytes());
    assert!(orphan.current.is_none(), "its finish closes it");
    assert!(orphan.attempt(9).is_none(), "and files no attempt");
}

/// **EVIDENCE HELD IN TWO DIRECTORIES MERGES OLDEST FIRST, AND ONE ATTEMPT
/// HELD IN BOTH IS NAMED ONCE** (BASE-COV-API-AS3). Two directories a run of
/// a kind can write, each holding a refusal of the same day under the same
/// attempt: the day's refusals come oldest first from both, each directory
/// named; the attempt once. The same reads in another order, or fewer of
/// them, are not the same reads.
///
/// Extremes: singles — the same refusal day in two directories; one attempt
/// id in two directories; combinations — the newer refusal held by the first
/// directory and the older by the second, merged by time; the two reads
/// swapped; out of the box — the reads compared against a list of one.
#[test]
fn evidence_held_in_two_directories_merges_oldest_first_and_names_an_attempt_once() {
    let halves: Vec<&'static str> = telemetry::run_dirs::gdfl_evidence("options").collect();
    let &[one, two, ..] = halves.as_slice() else {
        panic!("a run of options can write two directories: {halves:?}")
    };
    let ring = |ms: i64| {
        let mut held = DirRefusals::default();
        held.fold(
            ring_line(
                1,
                Some(42),
                ms,
                ("info", "cli.lifecycle", "command started"),
                r#""command":"gdfl-import""#,
            )
            .as_bytes(),
        );
        held.fold(
            ring_line(
                2,
                None,
                ms,
                ("error", "gdfl.import", "refused"),
                r#""kind":"options","day":"2020-01-02""#,
            )
            .as_bytes(),
        );
        std::sync::Arc::new(held)
    };
    let evidence = |half: &'static str, held| DirEvidence {
        half,
        path: std::path::Path::new("/nowhere").join(half),
        state: DirState::Read,
        held,
    };
    let both = ImportRefusals {
        dirs: vec![evidence(one, ring(2_000)), evidence(two, ring(1_000))],
    };
    let logged = both.logged("options", "2020-01-02");
    let times: Vec<i64> = logged.records.iter().map(|r| r.at_unix_millis).collect();
    assert_eq!(times, [1_000, 2_000], "oldest first across both");
    let names: Vec<String> = both.dirs().iter().map(DirEvidence::name).collect();
    assert_eq!(logged.from, names);
    assert_eq!(both.attempts_of("options", "2020-01-02"), [42]);
    assert!(!both.same_reads(&both.dirs()[..1]), "fewer reads");
    let swapped: Vec<DirEvidence> = both.dirs().iter().rev().cloned().collect();
    assert!(
        !both.same_reads(&swapped),
        "the same reads in another order"
    );
    assert!(both.same_reads(both.dirs()), "the same reads");
}

/// **AN AUDITED REFUSAL OF A DAY IS THE IMPORTER'S ERROR AND NOTHING ELSE
/// THE DAY'S POSTING HOLDS** (BASE-COV-API-AS3). The day's posting of the
/// `cli` directory's audit stream holds the importer's refusal, the
/// importer's own `run started` (audited for its word, at `info`) and another
/// target's error on that day; only the refusal is answered.
///
/// Extremes: singles — another target's error on the day; the importer's own
/// line at `info`; combinations — all three in one posting; out of the box —
/// a lifecycle word in an importer line making it audited at `info`.
#[test]
fn an_audited_refusal_of_a_day_is_the_importers_error_and_nothing_else() {
    let root = scratch("audited-keep");
    let cli = root.join("logs").join("cli");
    std::fs::create_dir_all(&cli).expect("a run directory");
    {
        let sink = telemetry::Sink::open(
            &telemetry::Config::new(&cli).with_min_level(telemetry::Level::Trace),
        )
        .expect("opens");
        for event in [
            telemetry::Event::error("gdfl.import", "refused")
                .with("kind", "options")
                .with("day", "2020-01-02")
                .with("why", "an invented fault"),
            telemetry::Event::info("gdfl.import", "run started")
                .with("kind", "options")
                .with("day", "2020-01-02"),
            telemetry::Event::error("pull.member", "member did not land")
                .with("kind", "options")
                .with("day", "2020-01-02"),
        ] {
            assert_eq!(sink.emit(&event), telemetry::Emitted::Written);
        }
    }
    let got = audited_import_refusals(&root, "options", "2020-01-02");
    assert_eq!(got.records.len(), 1, "{:?}", got.records);
    assert_eq!(got.records[0].message, "refused");
    assert_eq!(got.from, ["logs/cli"]);
    assert!(got.unread.is_empty(), "{:?}", got.unread);
    let _ignored = std::fs::remove_dir_all(&root);
}

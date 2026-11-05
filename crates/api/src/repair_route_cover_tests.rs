//! Rows of `repair_route.rs` no test reached before BASE-COV-API repair 3
//! (shard AS1, D-8661): a plan query refused by each of its own rules, a
//! known rung other than the minute, a derived rung's refold, a series'
//! first held month whose leading sessions are never a repair, and a repair
//! journal that will not seek or will not read. Every bar, symbol and
//! journal line is invented.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a whole-file test module; fixtures are bounded and a failed assertion is the test failing"
)]

use super::*;
use pull::session::IST_OFFSET_SECS;
use store::format::{Bar, OI_NULL};
use store::path::{StorePath, YearMonth};

/// An invented bar at 09:15 IST of `day`, on the grid of every rung.
fn bar(day: Day) -> Bar {
    let secs = i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + 555 * 60;
    Bar {
        ts_micros: secs * 1_000_000,
        open: 700_000,
        high: 700_009,
        low: 699_991,
        close: 700_003,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// A fresh scratch store holding one bar of `symbol` at `rung` on each
/// `(month, day)` of 2020.
fn store_of(name: &str, symbol: &str, rung: Timeframe, held: &[(u8, u8)]) -> std::path::PathBuf {
    let root = crate::scratch::path(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let id = repair_plan::symbol_id(symbol);
    for (month, day) in held {
        let path = StorePath::new(PathParts {
            vendor: brutex_core::vendor::Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol,
            contract: None,
            timeframe: rung,
            month: YearMonth::new(2020, *month).expect("a month"),
            file: FileKind::Bars,
        })
        .expect("a store path");
        let mut file = store::file::BarFile::open_or_create(&root, path, id).expect("a bar file");
        file.append(&[bar(Day::new(2020, *month, *day).expect("a day"))])
            .expect("an append");
    }
    root
}

/// The plan's answer to `query` over `root`, parsed.
fn planned(root: &Path, query: &str) -> serde_json::Value {
    let (status, _, body) = plan_answer(root, query);
    assert_eq!(status, axum::http::StatusCode::OK, "{query}: {body}");
    serde_json::from_str(&body).expect("one object")
}

/// The refusal `query` earns over `root`: a 400 and its one reason.
fn refusal(root: &Path, query: &str) -> String {
    let (status, _, body) = plan_answer(root, query);
    assert_eq!(
        status,
        axum::http::StatusCode::BAD_REQUEST,
        "{query}: {body}"
    );
    let json: serde_json::Value = serde_json::from_str(&body).expect("one object");
    json["error"].as_str().expect("a reason").to_owned()
}

/// **EVERY PLAN QUERY IS REFUSED BY ITS OWN RULE** (BASE-COV-API repair 3,
/// D-8661): `plan_answer`. Extremes: a feed this build does not store; an
/// empty instrument; an instrument of a byte no symbol holds; one of 25
/// bytes, past a symbol's 24; a segment that is neither INDEX nor CASH, in
/// lower case; a window that runs backwards; and, at once, a feed and an
/// instrument both bad (the feed is refused first), and a segment and a rung
/// both bad (the segment first); out of the box, a symbol of exactly 24
/// bytes at a known rung other than the minute, which is answered.
#[test]
fn every_plan_query_is_refused_by_its_own_rule() {
    let root = crate::scratch::path("repair-route-cover-refusals");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let window = "from=2020-12-01&to=2020-12-31";
    let long = "A".repeat(25);
    for (query, why) in [
        (
            format!("feed=nonsense&instrument=X&{window}"),
            "\"nonsense\" is not a feed this build stores",
        ),
        (
            format!("feed=zerodha&instrument=&{window}"),
            "\"\" is not a store symbol",
        ),
        (
            format!("feed=zerodha&instrument=a.b&{window}"),
            "\"A.B\" is not a store symbol",
        ),
        (
            format!("feed=zerodha&instrument={long}&{window}"),
            "\"AAAAAAAAAAAAAAAAAAAAAAAAA\" is not a store symbol",
        ),
        (
            format!("feed=zerodha&instrument=X&segment=fo&{window}"),
            "segment \"FO\": a repair plan reads INDEX or CASH",
        ),
        (
            "feed=zerodha&instrument=X&from=2020-12-31&to=2020-12-01".to_owned(),
            "window 2020-12-31..=2020-12-01 runs backwards",
        ),
        (
            format!("feed=nonsense&instrument=a.b&{window}"),
            "\"nonsense\" is not a feed this build stores",
        ),
        (
            format!("feed=zerodha&instrument=X&segment=fo&rung=7min&{window}"),
            "segment \"FO\": a repair plan reads INDEX or CASH",
        ),
    ] {
        assert_eq!(refusal(&root, &query), why, "{query}");
    }
    let full = "A".repeat(24);
    let json = planned(
        &root,
        &format!("feed=zerodha&instrument={full}&rung=5min&{window}"),
    );
    assert_eq!(json["rung"], "5min");
    assert_eq!(json["instrument"], format!("NSE-INDEX-{full}"));
    let _ = std::fs::remove_dir_all(root);
}

/// **A DERIVED RUNG IS REFOLDED, NEVER PULLED, AND A SERIES' FIRST HELD
/// MONTH NEVER PLANS ITS LEADING SESSIONS AS A REPAIR** (BASE-COV-API repair
/// 3, D-8661): `repair_json`, `derived_rung` and `filled_by`. Extremes: a
/// five-minute rung with a hole at the front of December (one minute body,
/// one refold line); the day rung (pulled, never derived); the second rung
/// (finer than a minute: pulled); a minute series held from December alone
/// (its leading sessions are before the first held bar); and, at once, a
/// derived rung whose December needs a repair beside a cash segment (the
/// body's target is the equities, the refold names the segment); out of the
/// box, a derived rung with nothing held at all (no range, so no refold).
#[test]
fn a_derived_rung_is_refolded_and_a_first_held_month_is_never_a_repair() {
    let window = "from=2020-12-01&to=2020-12-31";
    let derived = store_of(
        "repair-route-cover-derived",
        "ROUTEIDX",
        Timeframe::MINUTE_5,
        &[(11, 27), (12, 16)],
    );
    let json = planned(
        &derived,
        &format!("feed=zerodha&instrument=routeidx&rung=5min&{window}"),
    );
    let repair = &json["repair"];
    assert!(repair["derived"].is_string(), "{json}");
    assert_eq!(
        repair["refold"],
        serde_json::json!([
            "cli zerodha-repair ROUTEIDX 2020-12-01 2020-12-31 --segment INDEX --from-store <this store> --reason <why>"
        ]),
        "{json}"
    );
    let body = repair["bodies"][0].as_str().expect("one body");
    assert!(
        body.starts_with("target=indices&vendor=zerodha&granularity=1min&"),
        "a derived rung asks for the minutes: {body}"
    );
    for rung in ["1day", "1s"] {
        let json = planned(
            &derived,
            &format!("feed=zerodha&instrument=routeidx&rung={rung}&{window}"),
        );
        assert!(json["repair"]["derived"].is_null(), "{rung}: {json}");
        assert_eq!(json["repair"]["refold"], serde_json::json!([]), "{rung}");
    }
    let empty = planned(
        &derived,
        &format!("feed=zerodha&instrument=NOTHELD&rung=5min&{window}"),
    );
    assert!(empty["repair"]["derived"].is_string(), "{empty}");
    assert_eq!(empty["repair"]["refold"], serde_json::json!([]), "{empty}");

    let cash = planned(
        &derived,
        &format!("feed=zerodha&instrument=routeidx&segment=cash&rung=5min&{window}"),
    );
    assert_eq!(cash["instrument"], "NSE-CASH-ROUTEIDX");
    assert_eq!(cash["repair"]["refold"], serde_json::json!([]), "{cash}");
    assert!(
        cash["absence_may_be_no_trade"].as_bool().expect("a bool"),
        "a share may simply not have traded: {cash}"
    );

    let first = store_of(
        "repair-route-cover-first",
        "ROUTEIDX",
        Timeframe::MINUTE_1,
        &[(12, 16)],
    );
    let json = planned(
        &first,
        &format!("feed=zerodha&instrument=routeidx&{window}"),
    );
    let december = &json["months"][0];
    let before: Vec<&serde_json::Value> = december["missing"]
        .as_array()
        .expect("missing sessions")
        .iter()
        .filter(|missing| missing["fill"] == "before_first")
        .collect();
    assert!(!before.is_empty(), "{json}");
    for missing in &before {
        assert!(
            missing["by"]
                .as_str()
                .expect("a sentence")
                .starts_with("before the series' first held bar"),
            "{json}"
        );
    }
    assert_eq!(
        json["totals"]["before_first"],
        serde_json::json!(before.len())
    );
    assert_eq!(
        json["totals"]["repair_months"], 0,
        "never planned as a repair"
    );
    assert_eq!(json["repair"]["ranges"], serde_json::json!([]));
    for root in [derived, first] {
        let _ = std::fs::remove_dir_all(root);
    }
}

/// **A REPAIR JOURNAL THAT WILL NOT SEEK OR WILL NOT READ IS NAMED, NEVER
/// SKIPPED** (BASE-COV-API repair 3, D-8661): `journal_lines`. Extremes: a
/// folder where the journal goes (it opens, and will not read as text); a
/// FIFO there, held open for writing (it opens, and will not seek: `mkfifo`,
/// the external program, run from a test only, as `census.rs`' tests do); a
/// journal that is not UTF-8; a journal of only other series' lines (an
/// empty list); and, at once, a journal five bytes past the 64 KiB read whose
/// first line, cut by the read, still names the series (skipped, never
/// half-read) beside a whole line that does; out of the box, a line naming the series as a prefix of another
/// key (`NSE-INDEX-ROUTEIDXB` is not `NSE-INDEX-ROUTEIDX`).
#[test]
fn a_repair_journal_that_will_not_seek_or_read_is_named() {
    let query = "feed=zerodha&instrument=routeidx&from=2020-12-01&to=2020-12-31";
    let root = crate::scratch::path("repair-route-cover-journal");
    let _ = std::fs::remove_dir_all(&root);
    let audit = root.join("audit");
    std::fs::create_dir_all(&audit).expect("an audit folder");
    let path = audit.join("zerodha.repairs");

    std::fs::create_dir_all(&path).expect("a folder where the journal goes");
    let json = planned(&root, query);
    assert!(
        json["journal"]["error"]
            .as_str()
            .expect("an error")
            .ends_with("zerodha.repairs is not text"),
        "{json}"
    );
    std::fs::remove_dir(&path).expect("the folder gone");

    let made = std::process::Command::new("mkfifo")
        .arg(&path)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "the FIFO is made");
    let held = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .expect("the FIFO held open, so the route's open does not wait");
    let json = planned(&root, query);
    drop(held);
    let said = json["journal"]["error"].as_str().expect("an error");
    assert!(
        said.starts_with("cannot read ") && said.ends_with("zerodha.repairs"),
        "{json}"
    );
    std::fs::remove_file(&path).expect("the FIFO gone");

    std::fs::write(&path, [0xff, 0xfe, b'\n']).expect("not UTF-8");
    let json = planned(&root, query);
    assert!(
        json["journal"]["error"]
            .as_str()
            .expect("an error")
            .ends_with("is not text"),
        "{json}"
    );

    std::fs::write(&path, "2020-12-20 repair NSE-INDEX-OTHER from 1 to 2\n").expect("a journal");
    let json = planned(&root, query);
    assert_eq!(json["journal"]["lines"], serde_json::json!([]), "{json}");
    assert_eq!(json["journal"]["truncated"], false);

    // FIVE BYTES PAST THE READ: it starts five bytes into the first line,
    // whose rest still names the series, so only the skip keeps it out.
    let cut = "0123456789 NSE-INDEX-ROUTEIDX cut\n";
    let last = "2020-12-21 repair NSE-INDEX-ROUTEIDXB prefix\n2020-12-22 repair NSE-INDEX-ROUTEIDX whole\n";
    let max = usize::try_from(MAX_JOURNAL_BYTES).expect("small");
    let filler = max + 5 - cut.len() - last.len();
    let text = format!("{cut}{}\n{last}", "z".repeat(filler - 1));
    assert_eq!(text.len(), max + 5);
    std::fs::write(&path, &text).expect("a long journal");
    let json = planned(&root, query);
    assert_eq!(json["journal"]["truncated"], true, "{json}");
    assert_eq!(
        json["journal"]["lines"],
        serde_json::json!(["2020-12-22 repair NSE-INDEX-ROUTEIDX whole"]),
        "the cut line and the longer key are not the series'"
    );
    let _ = std::fs::remove_dir_all(root);
}

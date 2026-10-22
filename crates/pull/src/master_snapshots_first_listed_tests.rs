//! R4-28 repair 1: the futures first-listed index.
//!
//! Every row here is invented: underlyings named `ZQ...`, tokens in the eight
//! millions. The header is Kite's published column list, a schema and not a
//! row.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use super::*;
use crate::master_snapshots::{file_name, receipt_name, snapshot};

const URL: &str = "https://api.kite.trade/instruments";

fn route() -> ContinuousSpec {
    crate::vendor::ZERODHA_CONTINUOUS
}

fn scratch(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir().join(format!(
        "brutex-r428-fl-{name}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).expect("a real day")
}

fn stamp(on: Day) -> Stamp {
    Stamp { day: on, ordinal: 1 }
}

/// One invented NFO future of `name`.
fn fut(token: u64, name: &str, expiry: &str) -> String {
    format!("{token},{},ZQF{token},\"{name}\",0,{expiry},0,0.05,75,FUT,NFO-FUT,NFO", token / 256)
}

/// One invented NFO option, which the index never takes in.
fn opt(token: u64, name: &str, expiry: &str) -> String {
    format!("{token},{},ZQO{token},\"{name}\",0,{expiry},100,0.05,75,CE,NFO-OPT,NFO", token / 256)
}

fn body(rows: &[String]) -> String {
    let mut body = String::from(crate::vendor::KITE_MASTER_HEADER);
    for one in rows {
        body.push('\n');
        body.push_str(one);
    }
    body.push('\n');
    body
}

fn write(root: &Path, on: Day, rows: &[String]) {
    snapshot(root, Kind::Kite, on, URL, body(rows).as_bytes()).expect("a snapshot");
}

fn index_text(root: &Path, file: &str) -> String {
    fs::read_to_string(root.join(DIR).join(file)).unwrap_or_default()
}

fn stamps(root: &Path) -> Vec<Stamp> {
    Catalog::read(root).unwrap().receipted(Kind::Kite).to_vec()
}

#[test]
fn a_name_a_file_cannot_carry_is_encoded_or_refused_never_trusted() {
    assert_eq!(encoded("ZQIDX").as_deref(), Some("ZQIDX"));
    assert_eq!(encoded("ZQ&M").as_deref(), Some("ZQ%26M"), "an ampersand");
    assert_eq!(encoded("..").as_deref(), Some("%2E%2E"), "never a parent");
    assert_eq!(encoded("a/b").as_deref(), Some("a%2Fb"), "never a path");
    assert_eq!(encoded(""), None, "an empty name has no file");
    assert_eq!(encoded(&"Z".repeat(MAX_ENCODED)).map(|w| w.len()), Some(MAX_ENCODED));
    assert_eq!(encoded(&"Z".repeat(MAX_ENCODED + 1)), None, "past a file name");
    assert!(!carried("A|B") && !carried("A\nB") && !carried("A\rB") && !carried(""));
    assert!(carried("ZQ&M"));
}

#[test]
fn a_line_cut_or_edited_never_reads_as_a_record() {
    let line = sealed("fl1|future|NFO|FUT|ZQIDX|2026-01-27|8000001|2026-01-02|20260102");
    assert_eq!(
        unsealed(&line),
        Some("fl1|future|NFO|FUT|ZQIDX|2026-01-27|8000001|2026-01-02|20260102")
    );
    assert_eq!(unsealed(line.trim_end_matches('\n')), None, "cut before its end");
    let edited = line.replace("2026-01-02", "2025-01-02");
    assert_eq!(unsealed(&edited), None, "one date changed");
    assert_eq!(unsealed("\n"), None);
}

#[test]
fn a_stamp_base_round_trips_and_a_name_this_module_did_not_make_is_refused() {
    assert_eq!(parse_base("20260102"), Some(stamp(day(2026, 1, 2))));
    assert_eq!(
        parse_base("20260102_2"),
        Some(Stamp {
            day: day(2026, 1, 2),
            ordinal: 2
        })
    );
    for refused in ["20260102_1", "20260102_02", "20260102_", "2026010", "20261302", "x"] {
        assert_eq!(parse_base(refused), None, "{refused}");
    }
}

#[test]
fn a_kill_mid_append_starts_the_next_line_fresh_and_the_cut_line_is_counted() {
    let dir = scratch("append");
    let path = dir.join("NFO").join("ZQIDX.idx");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "fl1|future|NFO|FUT|ZQ").unwrap();
    let line = sealed("fl1|absorbed|20260102|NFO|FUT|indexed|0|0|");
    append(&dir, &path, &line).unwrap();
    append(&dir, &path, &line).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert_eq!(text, format!("fl1|future|NFO|FUT|ZQ\n{line}{line}"));
    let (lines, torn) = read_lines(&path).unwrap();
    assert_eq!((lines.len(), torn), (2, 1));
    assert_eq!(read_lines(&dir.join("absent.idx")).unwrap(), (Vec::new(), 0));
}

#[test]
fn each_contract_is_written_once_and_the_earliest_listing_day_wins() {
    let root = scratch("earliest");
    let jan = fut(8_000_001, "ZQIDX", "2026-01-27");
    let feb = fut(8_000_002, "ZQIDX", "2026-02-24");
    write(&root, day(2026, 1, 9), &[jan.clone(), opt(8_100_001, "ZQIDX", "2026-01-27")]);
    write(&root, day(2026, 1, 16), &[jan.clone(), feb.clone()]);
    write(&root, day(2026, 1, 23), &[jan.clone(), feb.clone()]);
    let file = index_text(&root, "NFO/ZQIDX.idx");
    assert_eq!(file.lines().count(), 2, "one line per contract, never per snapshot: {file}");
    assert!(!file.contains("8100001"), "an option is never taken in");
    // OUT OF ORDER: a snapshot dated before the others, written after them.
    write(&root, day(2026, 1, 2), &[feb, jan]);
    let listed = listing(&root, &stamps(&root), route(), "ZQIDX", day(2026, 1, 31)).unwrap();
    assert_eq!(
        listed.first,
        BTreeMap::from([
            ((day(2026, 1, 27), 8_000_001), day(2026, 1, 2)),
            ((day(2026, 2, 24), 8_000_002), day(2026, 1, 2)),
        ])
    );
    assert!(listed.unreadable.is_empty() && listed.torn == 0, "{listed:?}");
    let early = listing(&root, &stamps(&root), route(), "ZQIDX", day(2026, 1, 1)).unwrap();
    assert!(early.first.is_empty(), "nothing first listed after the day asked");
    let absorbed = index_text(&root, ABSORBED);
    assert_eq!(absorbed.lines().count(), 4, "one commit line per snapshot: {absorbed}");
}

#[test]
fn another_underlying_or_a_name_sharing_its_file_never_answers() {
    let root = scratch("names");
    write(
        &root,
        day(2026, 1, 9),
        &[
            fut(8_000_001, "ZQIDX", "2026-01-27"),
            fut(8_000_002, "zqidx", "2026-01-27"),
            fut(8_000_003, "ZQOTHER", "2026-01-27"),
        ],
    );
    let listed = listing(&root, &stamps(&root), route(), "ZQIDX", day(2026, 1, 31)).unwrap();
    assert_eq!(
        listed.first.keys().copied().collect::<Vec<_>>(),
        vec![(day(2026, 1, 27), 8_000_001)],
        "only the exact name, whichever file a case-insensitive volume shares"
    );
    let none = listing(&root, &stamps(&root), route(), "ZQNONE", day(2026, 1, 31)).unwrap();
    assert!(none.first.is_empty() && none.unreadable.is_empty());
}

#[test]
fn rows_a_line_cannot_carry_are_counted_on_the_commit_line_never_dropped_silently() {
    let root = scratch("skipped");
    write(
        &root,
        day(2026, 1, 9),
        &[
            fut(8_000_001, "ZQIDX", "2026-01-27"),
            fut(8_000_002, "", "2026-01-27"),
            fut(8_000_003, "ZQ|BAR", "2026-01-27"),
            fut(8_000_004, "ZQIDX", ""),
            fut(8_000_005, &"Z".repeat(MAX_ENCODED + 1), "2026-01-27"),
        ],
    );
    let absorbed = index_text(&root, ABSORBED);
    assert!(absorbed.starts_with("fl1|absorbed|20260109|NFO|FUT|indexed|1|4|"), "{absorbed}");
}

#[test]
fn an_unparseable_snapshot_is_recorded_once_and_named_by_every_listing() {
    let root = scratch("unparseable");
    write(&root, day(2026, 1, 2), &[fut(8_000_001, "ZQIDX", "2026-01-27")]);
    snapshot(&root, Kind::Kite, day(2026, 1, 9), URL, b"not,a,kite,header\n1,2,3,4\n").unwrap();
    for _ in 0..2 {
        let listed = listing(&root, &stamps(&root), route(), "ZQIDX", day(2026, 1, 31)).unwrap();
        assert_eq!(listed.first.len(), 1, "the readable snapshot still answers");
        assert_eq!(listed.unreadable.len(), 1, "{listed:?}");
        assert!(
            listed.unreadable[0].starts_with("snapshot_unreadable: 2026-01-09: the dump has no column")
                && listed.unreadable[0].contains("recorded when the futures index took it in"),
            "{listed:?}"
        );
    }
    let absorbed = index_text(&root, ABSORBED);
    assert_eq!(absorbed.matches("|20260109|").count(), 1, "recorded once: {absorbed}");
    assert!(absorbed.contains("|unparseable|0|0|the dump has no column"), "{absorbed}");
    let before = listing(&root, &stamps(&root), route(), "ZQIDX", day(2026, 1, 8)).unwrap();
    assert!(before.unreadable.is_empty(), "a snapshot after the day asked is not needed");
}

#[test]
fn a_snapshot_that_does_not_check_is_named_and_retried_never_recorded() {
    let source = scratch("rot-source");
    write(&source, day(2026, 1, 2), &[fut(8_000_001, "ZQIDX", "2026-01-27")]);
    let root = scratch("rot");
    let held = stamp(day(2026, 1, 2));
    for name in [file_name(Kind::Kite, held), receipt_name(Kind::Kite, held)] {
        fs::copy(source.join(&name), root.join(&name)).unwrap();
    }
    // ROT ON DISK: the body no longer matches its receipt.
    fs::write(root.join(file_name(Kind::Kite, held)), b"garbled").unwrap();
    for _ in 0..2 {
        let listed = listing(&root, &stamps(&root), route(), "ZQIDX", day(2026, 1, 31)).unwrap();
        assert!(listed.first.is_empty());
        assert_eq!(listed.unreadable.len(), 1, "{listed:?}");
        assert!(listed.unreadable[0].contains("does not match its receipt"), "{listed:?}");
    }
    assert_eq!(index_text(&root, ABSORBED), "", "never recorded, so retried");
}

#[test]
fn index_lines_that_landed_without_their_commit_line_are_taken_in_again_and_fold() {
    let source = scratch("kill-source");
    write(&source, day(2026, 1, 2), &[fut(8_000_001, "ZQIDX", "2026-01-27")]);
    write(&source, day(2026, 1, 9), &[fut(8_000_002, "ZQIDX", "2026-02-24")]);
    let root = scratch("kill");
    for &held in &stamps(&source) {
        for name in [file_name(Kind::Kite, held), receipt_name(Kind::Kite, held)] {
            fs::copy(source.join(&name), root.join(&name)).unwrap();
        }
    }
    // A KILL AFTER THE UNDERLYING'S LINES AND BEFORE THE COMMIT LINE: the
    // lines are there, one cut short, and the absorbed log is not.
    let lines = index_text(&source, "NFO/ZQIDX.idx");
    fs::create_dir_all(root.join(DIR).join("NFO")).unwrap();
    fs::write(root.join(DIR).join("NFO/ZQIDX.idx"), &lines[..lines.len() - 5]).unwrap();
    let listed = listing(&root, &stamps(&root), route(), "ZQIDX", day(2026, 1, 31)).unwrap();
    assert_eq!(
        listed.first,
        BTreeMap::from([
            ((day(2026, 1, 27), 8_000_001), day(2026, 1, 2)),
            ((day(2026, 2, 24), 8_000_002), day(2026, 1, 9)),
        ])
    );
    assert!(listed.unreadable.is_empty(), "{listed:?}");
    assert_eq!(listed.torn, 1, "the cut line is counted");
    assert_eq!(index_text(&root, ABSORBED).lines().count(), 2, "both taken in");
}

#[test]
fn absorbing_with_nothing_pending_writes_nothing_and_a_route_no_line_can_carry_is_refused() {
    let root = scratch("idle");
    write(&root, day(2026, 1, 2), &[fut(8_000_001, "ZQIDX", "2026-01-27")]);
    let before = index_text(&root, ABSORBED);
    let again = absorb_pending(&root, route()).unwrap();
    assert_eq!(again, Absorbed::default());
    assert_eq!(index_text(&root, ABSORBED), before);
    let bad = ContinuousSpec {
        exchange: "N|FO",
        ..route()
    };
    assert!(absorb_pending(&root, bad).unwrap_err().contains("cannot carry"));
    // ANOTHER TYPE IS ITS OWN INDEX: nothing of FUT answers for it.
    let other = ContinuousSpec {
        future_type: "FUTX",
        ..route()
    };
    let taken = absorb_pending(&root, other).unwrap();
    assert_eq!(taken.indexed, vec![stamp(day(2026, 1, 2))]);
    let listed = listing(&root, &stamps(&root), other, "ZQIDX", day(2026, 1, 31)).unwrap();
    assert!(listed.first.is_empty());
}

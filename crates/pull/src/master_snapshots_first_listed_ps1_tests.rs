//! BASE-COV-PULL-PS1 (D-7335): the futures first-listed index's refusals no
//! test had reached (Route B, recipe v4): a generation or an index folder that
//! is a file, a commit line or an index line that checks and does not parse, a
//! line of another exchange or name in an underlying's file, a record and a
//! commit line that cannot be appended to, a sync the host refuses, a walk to
//! the root past a folder that is not the file's, an I/O failure after a
//! rebuild, a listing whose new generation is damaged too, and the one event a
//! write whose index failed leaves. Every row is invented: underlyings `ZQ...`,
//! tokens in the eight millions.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::master_snapshots::{Wrote, file_name, receipt_name, snapshot};

const URL: &str = "https://invented.example/instruments";

fn route() -> ContinuousSpec {
    crate::vendor::ZERODHA_CONTINUOUS
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).expect("a real day")
}

fn stamp(on: Day) -> Stamp {
    Stamp {
        day: on,
        ordinal: 1,
    }
}

/// One invented NFO future of `name`.
fn fut(token: u64, name: &str, expiry: &str) -> String {
    format!(
        "{token},{},ZQF{token},\"{name}\",0,{expiry},0,0.05,75,FUT,NFO-FUT,NFO",
        token / 256
    )
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

/// A Kite snapshot of `rows` dated `on`, taken into the index as it is written.
fn write(root: &Path, on: Day, rows: &[String]) {
    snapshot(root, Kind::Kite, on, URL, body(rows).as_bytes()).expect("a snapshot");
}

fn book(root: &Path) -> Book {
    Book::open(root).unwrap()
}

fn index_text(root: &Path, file: &str) -> String {
    fs::read_to_string(generation_dir(root).unwrap().join(file)).unwrap_or_default()
}

/// Copies `source`'s receipted snapshots, and only them, into a fresh root: a
/// root whose snapshots no index has taken in.
fn snapshots_only(source: &Path, name: &str) -> PathBuf {
    let root = crate::gdfl_fixtures::scratch(name);
    for &held in Catalog::read(source).unwrap().receipted(Kind::Kite) {
        for file in [file_name(Kind::Kite, held), receipt_name(Kind::Kite, held)] {
            fs::copy(source.join(&file), root.join(&file)).unwrap();
        }
    }
    root
}

/// `ZQIDX`'s January future first listed 2026-01-02 and its February one
/// 2026-01-09, each by its own snapshot, both taken in as written.
fn two_days(name: &str) -> PathBuf {
    let root = crate::gdfl_fixtures::scratch(name);
    write(
        &root,
        day(2026, 1, 2),
        &[fut(8_000_001, "ZQIDX", "2026-01-27")],
    );
    write(
        &root,
        day(2026, 1, 9),
        &[fut(8_000_002, "ZQIDX", "2026-02-24")],
    );
    root
}

fn both_contracts() -> BTreeMap<(Day, u64), Day> {
    BTreeMap::from([
        ((day(2026, 1, 27), 8_000_001), day(2026, 1, 2)),
        ((day(2026, 2, 24), 8_000_002), day(2026, 1, 9)),
    ])
}

/// A link at `root`'s first generation's commit log that leads into a folder
/// that does not exist: it reads as no log, and no line can be appended
/// through it. Returns the link's path.
fn dangling_log(root: &Path) -> PathBuf {
    let first = generation_dir(root).unwrap();
    fs::create_dir_all(&first).unwrap();
    let link = first.join(ABSORBED);
    std::os::unix::fs::symlink(root.join("no-such-folder").join(ABSORBED), &link).unwrap();
    link
}

/// The text of field `key` of `record`, or `None`.
fn text(record: &telemetry::Record, key: &str) -> Option<String> {
    record
        .field(key)
        .and_then(telemetry::OwnedValue::as_str)
        .map(str::to_owned)
}

/// **A DASH AND AN UNDERSCORE ARE KEPT IN A FILE NAME**, as letters and digits
/// are, and every other byte is escaped: a name holding both is a file of its
/// own name.
#[test]
fn a_dash_and_an_underscore_are_kept_in_a_file_name() {
    assert_eq!(encoded("ZQ-IDX_1").as_deref(), Some("ZQ-IDX_1"));
    assert_eq!(encoded("ZQ.IDX+1").as_deref(), Some("ZQ%2EIDX%2B1"));
    assert_eq!(
        path_of(Path::new("/zq"), "NFO", "ZQ-IDX_1"),
        Some(PathBuf::from("/zq/NFO/ZQ-IDX_1.idx"))
    );
}

/// **A GENERATION THAT IS A FILE IS NAMED, NEVER READ AS AN EMPTY INDEX**: the
/// commit log under it cannot be inspected (the host says a part of its path
/// is not a folder, which is not "missing"), so the call fails with that
/// reason rather than taking every snapshot in again as if nothing were held.
#[test]
fn a_generation_that_is_a_file_is_named_never_read_as_an_empty_index() {
    let root = crate::gdfl_fixtures::scratch("ps1-fl-generation-file");
    let first = root.join(DIR).join(generation_name(1));
    fs::create_dir_all(root.join(DIR)).unwrap();
    fs::write(&first, b"a file where a generation goes").unwrap();
    assert_eq!(generation_dir(&root).unwrap(), first, "still the first");
    let log = first.join(ABSORBED);
    let why = absorb_pending(&root, route()).unwrap_err();
    assert!(
        why.starts_with(&format!("{} could not be inspected: ", log.display())),
        "{why}"
    );
    assert!(first.is_file(), "left as it was");
    fs::remove_dir_all(root).unwrap();
}

/// **AN APPEND WHOSE SYNC THE HOST REFUSES IS NAMED**, its earlier lines said
/// to be untouched: a character device takes the bytes, and this host refuses
/// to sync it (checked first, by syncing the device directly), which is the
/// refusal a file system that cannot sync gives. The words are the host's.
#[test]
fn an_append_whose_sync_the_host_refuses_is_named() {
    let device = Path::new("/dev/null");
    let direct = OpenOptions::new()
        .read(true)
        .append(true)
        .open(device)
        .and_then(|file| file.sync_all());
    let why = direct.expect_err("a character device takes no sync on this host");
    let dir = crate::gdfl_fixtures::scratch("ps1-fl-device");
    let said =
        format!("/dev/null could not be appended to; its earlier lines are untouched: {why}");
    assert_eq!(append(&dir, device, "an invented line\n"), Err(said));
    fs::remove_dir_all(dir).unwrap();
}

/// **A NEW FILE OUTSIDE THE FOLDER IT IS SYNCED UP TO SYNCS EVERY FOLDER ABOVE
/// IT, TO THE ROOT, AND STOPS THERE**: no caller passes such a pair (each
/// passes a folder the file is under), and the walk is bounded by the path
/// itself, never by finding the folder. The answer is exactly what syncing the
/// file system's root alone answers on this host, and the line is whole.
#[test]
fn a_new_file_outside_its_folder_syncs_every_folder_up_to_the_root() {
    let dir = crate::gdfl_fixtures::scratch("ps1-fl-walk");
    let path = dir.join("fresh").join("ZQIDX.idx");
    let line = sealed("fl2|an invented line");
    assert_eq!(
        append(&dir.join("elsewhere"), &path, &line),
        sync_dir(Path::new("/"))
    );
    assert_eq!(fs::read_to_string(path).unwrap(), line);
    fs::remove_dir_all(dir).unwrap();
}

/// **AN INDEX FOLDER THAT IS A FILE IS NAMED, NEVER TAKEN FOR NO INDEX**: the
/// generations cannot be listed, so none is chosen and nothing is taken in. An
/// absent folder is the first generation; a file at its name is a refusal,
/// the same one every absorb gives.
#[test]
fn an_index_folder_that_is_a_file_is_named_never_taken_for_no_index() {
    let root = crate::gdfl_fixtures::scratch("ps1-fl-top-file");
    let top = root.join(DIR);
    fs::write(&top, b"a file where the index goes").unwrap();
    let why = generation_dir(&root).unwrap_err();
    assert!(
        why.starts_with(&format!("{} could not be listed: ", top.display())),
        "{why}"
    );
    assert_eq!(absorb_pending(&root, route()), Err(why));
    assert!(top.is_file(), "left as it was");
    fs::remove_dir_all(root).unwrap();
}

/// **A GENERATIONS RECORD THAT CANNOT BE APPENDED TO STARTS NO GENERATION**:
/// why a generation is started is written before its folder is made, so a
/// record that cannot take the line (a folder at its name) fails the listing
/// by name, and the damaged generation stays the index, exactly as found.
#[test]
fn a_generations_record_that_cannot_be_appended_to_starts_no_generation() {
    let root = two_days("ps1-fl-record");
    let first = generation_dir(&root).unwrap();
    let log = fs::read_to_string(first.join(ABSORBED)).unwrap();
    let garbled = log.replacen("|indexed|", "|indexes|", 1);
    fs::write(first.join(ABSORBED), &garbled).unwrap();
    let record = root.join(DIR).join(GENERATIONS);
    fs::create_dir(&record).unwrap();
    let why = listing(&root, &book(&root), route(), "ZQIDX", day(2026, 1, 31)).unwrap_err();
    assert!(
        why.starts_with(&format!(
            "{} could not be opened to append: ",
            record.display()
        )),
        "{why}"
    );
    assert_eq!(generation_dir(&root).unwrap(), first, "none was started");
    assert!(!root.join(DIR).join(generation_name(2)).exists());
    assert_eq!(fs::read_to_string(first.join(ABSORBED)).unwrap(), garbled);
    fs::remove_dir_all(root).unwrap();
}

/// **AN I/O FAILURE IN THE NEW GENERATION IS NAMED AFTER THE REBUILD**: the
/// call says the generation it found did not match its log and the next one
/// was started, then the failure, and it is an error, never a listing; one
/// generation was started and none after it.
#[test]
fn an_io_failure_in_the_new_generation_is_named_after_the_rebuild() {
    let root = crate::gdfl_fixtures::scratch("ps1-fl-io-after");
    let mut seen = Vec::new();
    let failed = with_rebuild(&root, route(), |generation| -> Result<(), Fault> {
        seen.push(generation.number);
        Err(if generation.number == 1 {
            Fault::Damaged("g1 invented rot".to_owned())
        } else {
            Fault::Io("an invented disk fault".to_owned())
        })
    });
    assert_eq!(seen, [1, 2]);
    let said = "generation g1 of the futures index did not match its commit log (g1 invented \
                rot), so it was left as it is and generation g2 was started, and every \
                receipted snapshot this call needed was taken into it again; then an invented \
                disk fault";
    assert_eq!(failed, Err(said.to_owned()));
    assert!(generation_dir(&root).unwrap().ends_with(generation_name(2)));
    fs::remove_dir_all(root).unwrap();
}

/// **A COMMIT LINE THAT CHECKS AND DOES NOT PARSE IS DAMAGE, EACH COUNTED**: a
/// line whose check holds and whose fields are no commit line damages the
/// generation, one line each. A commit line of another exchange parses, and is
/// neither taken nor counted for this route.
#[test]
fn a_commit_line_that_checks_and_does_not_parse_is_damage_each_counted() {
    let dir = crate::gdfl_fixtures::scratch("ps1-fl-log");
    let path = dir.join(ABSORBED);
    for line in [
        // Nine fields.
        "fl2|absorbed|20260102|NFO|FUT|indexed|0|0|",
        // Another format.
        "fl1|absorbed|20260102|NFO|FUT|indexed|0|0||",
        // No stamp.
        "fl2|absorbed|2026010|NFO|FUT|indexed|0|0||",
        // A count with no name.
        "fl2|absorbed|20260102|NFO|FUT|indexed|0|0|ZQIDX=1,=1|",
        // Parse rules that are no number.
        "fl2|absorbed|20260102|NFO|FUT|unparseable@x|0|0||invented",
        // A status of no kind.
        "fl2|absorbed|20260102|NFO|FUT|indexes|0|0||",
    ] {
        fs::write(&path, sealed(line)).unwrap();
        assert_eq!(
            read_log(&dir, route()).unwrap_err(),
            Fault::Damaged(format!(
                "{} has 1 line(s) that do not check",
                path.display()
            )),
            "{line}"
        );
    }
    fs::write(
        path,
        sealed("fl2|absorbed|20260102|BFO|FUT|indexed|1|0|ZQIDX=5|"),
    )
    .unwrap();
    let log = read_log(&dir, route()).unwrap();
    assert_eq!((log.taken.len(), log.committed.len()), (0, 0), "{log:?}");
    fs::remove_dir_all(dir).unwrap();
}

/// **AN INDEX LINE THAT CHECKS AND DOES NOT PARSE IS DAMAGE, EACH COUNTED**: a
/// line of an underlying's file whose check holds and whose fields are no
/// future line damages the generation, one line each, whatever the log
/// records.
#[test]
fn an_index_line_that_checks_and_does_not_parse_is_damage_each_counted() {
    let dir = crate::gdfl_fixtures::scratch("ps1-fl-file");
    let path = dir.join("ZQIDX.idx");
    for line in [
        // Eight fields.
        "fl2|future|NFO|FUT|ZQIDX|2026-01-27|8000001|2026-01-02",
        // Another format.
        "fl1|future|NFO|FUT|ZQIDX|2026-01-27|8000001|2026-01-02|20260102",
        // No expiry.
        "fl2|future|NFO|FUT|ZQIDX|2026-1-27|8000001|2026-01-02|20260102",
        // No token.
        "fl2|future|NFO|FUT|ZQIDX|2026-01-27|80000x1|2026-01-02|20260102",
        // No first day.
        "fl2|future|NFO|FUT|ZQIDX|2026-01-27|8000001|2026-13-02|20260102",
        // No stamp.
        "fl2|future|NFO|FUT|ZQIDX|2026-01-27|8000001|2026-01-02|2026010",
    ] {
        fs::write(&path, sealed(line)).unwrap();
        assert_eq!(
            read_underlying(&path, route(), "ZQIDX", &Log::default()).unwrap_err(),
            Fault::Damaged(format!(
                "{} has 1 line(s) that do not check",
                path.display()
            )),
            "{line}"
        );
    }
    fs::remove_dir_all(dir).unwrap();
}

/// **A LINE OF ANOTHER EXCHANGE OR ANOTHER NAME IN AN UNDERLYING'S FILE NEVER
/// ANSWERS FOR IT**: two words that differ only in case share a folder or a
/// file on a case-insensitive volume, so a file can hold checked lines of
/// `nfo` or of `zqidx`. The listing of `ZQIDX` on `NFO` is its own two
/// contracts, with nothing rebuilt and nothing refused.
#[test]
fn a_line_of_another_exchange_or_name_in_the_file_never_answers() {
    let root = two_days("ps1-fl-foreign");
    let file = generation_dir(&root).unwrap().join("NFO").join("ZQIDX.idx");
    let mut held = fs::read_to_string(&file).unwrap();
    held.push_str(&sealed(
        "fl2|future|nfo|FUT|ZQIDX|2026-03-31|8000003|2026-01-02|20260102",
    ));
    held.push_str(&sealed(
        "fl2|future|NFO|FUT|zqidx|2026-03-31|8000004|2026-01-02|20260102",
    ));
    fs::write(file, held).unwrap();
    let listed = listing(&root, &book(&root), route(), "ZQIDX", day(2026, 1, 31)).unwrap();
    assert_eq!(listed.first, both_contracts());
    assert_eq!(
        (listed.refused.len(), listed.rebuilt.as_deref()),
        (0, None),
        "{listed:?}"
    );
    fs::remove_dir_all(root).unwrap();
}

/// **A TYPE NO LINE CAN CARRY IS REFUSED BEFORE ANYTHING IS WRITTEN**, as an
/// exchange no line can carry is: the words name both, and no index folder is
/// made.
#[test]
fn a_type_no_line_can_carry_is_refused_before_anything_is_written() {
    let root = crate::gdfl_fixtures::scratch("ps1-fl-type");
    let bad = ContinuousSpec {
        future_type: "F|UT",
        ..route()
    };
    let said = "the futures index cannot carry the exchange \"NFO\" or type \"F|UT\"";
    assert_eq!(absorb_pending(&root, bad), Err(said.to_owned()));
    assert!(!root.join(DIR).exists(), "nothing written");
    fs::remove_dir_all(root).unwrap();
}

/// **A COMMIT LINE THAT CANNOT BE WRITTEN FAILS THE CALL BY NAME, AND THE NEXT
/// CALL COMMITS WHAT IT ADDED**: a link at the commit log leading into a folder
/// that does not exist reads as no log and refuses the append. The snapshot is
/// not recorded and the link is left as it was; once it is gone, the next call
/// takes the snapshot in, commits the whole line the failed call added to the
/// underlying's file, and adds no duplicate.
#[test]
fn a_commit_line_that_cannot_be_written_fails_the_call_and_the_next_commits() {
    let source = crate::gdfl_fixtures::scratch("ps1-fl-commit-source");
    write(
        &source,
        day(2026, 1, 2),
        &[fut(8_000_001, "ZQIDX", "2026-01-27")],
    );
    let root = snapshots_only(&source, "ps1-fl-commit");
    let link = dangling_log(&root);
    let why = absorb_pending(&root, route()).unwrap_err();
    assert!(
        why.starts_with(&format!(
            "{} could not be opened to append: ",
            link.display()
        )),
        "{why}"
    );
    assert_eq!(
        fs::read_link(&link).unwrap(),
        root.join("no-such-folder").join(ABSORBED),
        "left as it was"
    );
    let file = generation_dir(&root).unwrap().join("NFO").join("ZQIDX.idx");
    let added = fs::read_to_string(&file).unwrap();
    assert_eq!(added.lines().count(), 1, "{added}");
    fs::remove_file(link).unwrap();
    assert_eq!(
        absorb_pending(&root, route()),
        Ok(Absorbed {
            indexed: vec![stamp(day(2026, 1, 2))],
            ..Absorbed::default()
        })
    );
    assert_eq!(fs::read_to_string(file).unwrap(), added, "no duplicate");
    let log = index_text(&root, ABSORBED);
    assert!(
        log.starts_with("fl2|absorbed|20260102|NFO|FUT|indexed|0|0|ZQIDX=1|"),
        "{log}"
    );
    fs::remove_dir_all(source).unwrap();
    fs::remove_dir_all(root).unwrap();
}

/// **A SNAPSHOT WRITE WHOSE INDEX CANNOT COMMIT SAYS SO IN ONE ERROR EVENT**,
/// and the write itself stands: the event names the index and carries why, so
/// the operator hears of the snapshot every futures plan will take in again.
/// Run in a process of its own, the one install there: the event is built
/// only when a sink would write it (`emit_if!`).
#[test]
fn a_failed_index_write_is_one_error_event() {
    crate::support::where_permission_binds(
        "master_snapshots::first_listed::ps1_tests::a_failed_index_write_is_one_error_event",
        a_failed_index_write_is_one_error_event_body,
    );
}

fn a_failed_index_write_is_one_error_event_body() {
    let home = crate::gdfl_fixtures::scratch("ps1-fl-sink");
    let sink =
        telemetry::install(&telemetry::Config::new(&home).with_min_level(telemetry::Level::Trace))
            .expect("the only install in this process");
    let root = crate::gdfl_fixtures::scratch("ps1-fl-sink-root");
    let link = dangling_log(&root);
    let on = day(2026, 1, 2);
    let rows = [fut(8_000_001, "ZQIDX", "2026-01-27")];
    assert_eq!(
        snapshot(&root, Kind::Kite, on, URL, body(&rows).as_bytes()),
        Ok(Wrote::New(stamp(on))),
        "the write stands"
    );
    let records = telemetry::tail(
        &home,
        sink.keep_files(),
        &telemetry::Query::last(telemetry::MAX_LIMIT).from_target(EVENT_TARGET),
    )
    .records;
    let failed: Vec<&telemetry::Record> = records
        .iter()
        .filter(|record| {
            record.message
                == "the futures first-listed index could not take the snapshots in; each futures \
                    plan retries them and names any it cannot read"
        })
        .collect();
    assert_eq!(failed.len(), 1, "{records:?}");
    assert_eq!(failed[0].level, telemetry::Level::Error);
    assert_eq!(text(failed[0], "index"), Some(DIR.to_owned()));
    let detail = text(failed[0], "detail").unwrap_or_default();
    assert!(
        detail.starts_with(&format!(
            "{} could not be opened to append: ",
            link.display()
        )),
        "{detail}"
    );
}

/// **A LISTING WHOSE NEW GENERATION DOES NOT MATCH ITS LOG EITHER IS REFUSED BY
/// NAME, WITH NOTHING LISTED**: a route whose exchange no line can carry (the
/// listing door takes any route; only the absorb door refuses one) writes a
/// commit line no reader parses, so the generation it found and the one it
/// starts are both damaged; the refusal names both and plans nothing.
#[test]
fn a_listing_whose_new_generation_is_damaged_too_is_refused_by_name() {
    let root = crate::gdfl_fixtures::scratch("ps1-fl-still");
    write(
        &root,
        day(2026, 1, 9),
        &[fut(8_000_001, "ZQIDX", "2026-01-27")],
    );
    let bar = ContinuousSpec {
        exchange: "N|FO",
        ..route()
    };
    let listed = listing(&root, &book(&root), bar, "ZQIDX", day(2026, 1, 31)).unwrap();
    let log = |number: u32| root.join(DIR).join(generation_name(number)).join(ABSORBED);
    let refused = format!(
        "futures_index_damaged: generation g1 of the futures index did not match its commit log \
         ({} has 1 line(s) that do not check), so it was left as it is and generation g2 was \
         started, and every receipted snapshot this call needed was taken into it again; it does \
         not match its own commit log either: {} has 1 line(s) that do not check; no contract of \
         ZQIDX was planned from it",
        log(1).display(),
        log(2).display()
    );
    assert_eq!(
        listed,
        Listing {
            refused: vec![refused],
            ..Listing::default()
        }
    );
    assert!(generation_dir(&root).unwrap().ends_with(generation_name(2)));
    fs::remove_dir_all(root).unwrap();
}

#![cfg(test)]
//! Tests of `cli alerts`. Every store root is a scratch directory and every
//! alert is invented; the queue's own rules are tested in `pull::alerts`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use std::os::unix::fs::FileExt as _;

use pull::alerts::{NewAlert, Put, put_at};

use super::*;

const NOW: u64 = 1_791_234_567;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-cli-alerts-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs the verb over `store` and returns (exit, output).
fn run(store: &Path, arguments: &[&str]) -> (u8, String) {
    let mut out = String::new();
    let root = store.to_path_buf();
    let code = command_with(arguments, &move || Ok(root.clone()), &mut out);
    (code, out)
}

fn put(store: &Path, kind: u8, key: &str, text: &str) -> u32 {
    let alert = NewAlert {
        kind,
        key,
        day: 18_682,
        run: 77,
        text,
    };
    match put_at(store, &alert, NOW).unwrap() {
        Put::Written { ordinal, .. } => ordinal,
        Put::Duplicate { ordinal } => panic!("duplicate of {ordinal}"),
    }
}

/// Flips a byte of ordinal `k` in the store's queue.
fn spoil(store: &Path, ordinal: u64) {
    let queue = alerts::Names::under(store).queue;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(queue)
        .unwrap();
    let at = alerts::offset_of(ordinal) + 50;
    let mut one = [0_u8; 1];
    file.read_exact_at(&mut one, at).unwrap();
    file.write_all_at(&[one[0] ^ 0xFF], at).unwrap();
}

#[test]
fn the_listing_shows_open_alerts_newest_first_and_an_acknowledgement_hides_one() {
    let store = scratch("list");
    let (code, out) = run(&store, &[]);
    assert_eq!(code, OK);
    assert!(out.contains("no queue yet, no alert"), "{out}");
    assert!(!store.join("audit").exists(), "a listing never writes");

    put(&store, 6, "hole:77:options:no_zip", "first day 2021-02-24");
    put(&store, 5, "disk_full:77", "no space left");
    let (code, out) = run(&store, &[]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("2 unacknowledged in the newest 1024 records"),
        "{out}"
    );
    assert!(
        out.contains("0 older unacknowledged (cli alerts --older)"),
        "{out}"
    );
    let disk = out
        .find("#2  disk_full  run 77  day 2021-02-24")
        .expect(&out);
    let hole = out.find("#1  hole  run 77  day 2021-02-24").expect(&out);
    assert!(disk < hole, "newest first");
    assert!(
        out.contains("put 2026-10-05T"),
        "the put time is printed in UTC: {out}"
    );
    assert!(out.contains("first day 2021-02-24"));

    // IA-AL-03, the cli's half: the acknowledgement is recorded and hides it.
    let (code, out) = run(&store, &["--ack", "1"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains(
            "ALERT ACKNOWLEDGED: #1 (hole, run 77); recorded as #3; unacknowledged left: 1"
        ),
        "{out}"
    );
    let (_, out) = run(&store, &[]);
    assert!(
        out.contains("1 unacknowledged") && !out.contains("#1  hole"),
        "{out}"
    );

    // Refusals are by name, exit FAILED, and write nothing.
    let len = std::fs::metadata(alerts::Names::under(&store).queue)
        .unwrap()
        .len();
    for (ordinal, code_word) in [
        ("1", "already_acknowledged"),
        ("0", "no_such_alert"),
        ("3", "no_such_alert"),
        ("99", "no_such_alert"),
    ] {
        let (code, out) = run(&store, &["--ack", ordinal]);
        assert_eq!(code, FAILED, "{out}");
        assert!(
            out.contains(&format!("ALERTS REFUSED: {code_word}")),
            "{out}"
        );
    }
    assert_eq!(
        std::fs::metadata(alerts::Names::under(&store).queue)
            .unwrap()
            .len(),
        len
    );
    std::fs::remove_dir_all(&store).unwrap();
}

#[test]
fn garbage_arguments_are_a_misuse_and_a_missing_store_is_refused() {
    let store = scratch("misuse");
    for arguments in [
        &["--ack"][..],
        &["--ack", "x"],
        &["--ack", "-1"],
        &["--ack", "4294967296"],
        &["--before", "1.5"],
        &["--before", "+5"],
        &["--ack", "+4"],
        &["--ack", ""],
        &["--before", " 5"],
        &["--file"],
        &["--file", ""],
        &["--older", "extra"],
        &["--nope"],
        &["--ack", "1", "--older"],
    ] {
        let (code, out) = run(&store, arguments);
        assert_eq!(code, MISUSED, "{arguments:?}: {out}");
        assert!(
            out.contains("ALERTS MISUSED") && out.contains(USAGE),
            "{out}"
        );
    }
    assert!(!store.join("audit").exists(), "a misuse touches nothing");

    let mut out = String::new();
    let code = command_with(
        &[],
        &|| Err("BRUTEX_STORE names no directory".to_owned()),
        &mut out,
    );
    assert_eq!(code, FAILED);
    assert!(
        out.contains("ALERTS REFUSED: BRUTEX_STORE names no directory"),
        "{out}"
    );

    // A root that vanished after it was proved: refused by the queue's name.
    let (code, out) = run(&store.join("gone"), &[]);
    assert_eq!(code, FAILED);
    assert!(out.contains("ALERTS REFUSED: alerts_unavailable"), "{out}");
    std::fs::remove_dir_all(&store).unwrap();
}

#[test]
fn older_alerts_are_counted_on_the_listing_and_listed_by_older_and_before() {
    // IA-AL-04, the cli's half.
    let store = scratch("older");
    put(
        &store,
        8,
        "runs_ledger:4096",
        "the runs ledger fails at 4096",
    );
    for i in 0..1_030_u32 {
        put(&store, 9, &format!("mismatch:{i}"), "x");
    }
    let (code, out) = run(&store, &[]);
    assert_eq!(code, OK);
    assert!(
        out.contains("1024 unacknowledged in the newest 1024 records"),
        "{out}"
    );
    assert!(
        out.contains("7 older unacknowledged (cli alerts --older)"),
        "{out}"
    );
    assert!(
        out.contains("the page before: cli alerts --before 8"),
        "{out}"
    );
    assert!(
        !out.contains("runs_ledger_damaged"),
        "ordinal 1 is outside the one read"
    );

    let (code, out) = run(&store, &["--older"]);
    assert_eq!(code, OK);
    assert!(
        out.contains("ALERTS OLDER -- 7 unacknowledged older than the newest 1024"),
        "{out}"
    );
    assert!(out.contains("#1  runs_ledger_damaged  run 77"), "{out}");

    let (code, out) = run(&store, &["--before", "8"]);
    assert_eq!(code, OK);
    assert!(
        out.contains("7 unacknowledged in the 1024 records before #8"),
        "{out}"
    );
    assert!(
        out.contains("#1  runs_ledger_damaged") && !out.contains("the page before"),
        "{out}"
    );

    // Outside the window the cli acknowledges what a route refuses.
    let (code, out) = run(&store, &["--ack", "1"]);
    assert_eq!(code, OK, "{out}");
    let (_, out) = run(&store, &[]);
    // The acknowledgement is one more record, so the window moved by one:
    // #8 left it and #1 is no longer open.
    assert!(out.contains("1023 unacknowledged in the newest"), "{out}");
    assert!(out.contains("7 older unacknowledged"), "{out}");
    let (_, out) = run(&store, &["--older"]);
    assert!(!out.contains("runs_ledger_damaged"), "{out}");
    // The page that holds #1 agrees: its acknowledgement is record #1032,
    // far after that page, and the alert is no longer listed there.
    let (code, out) = run(&store, &["--before", "9"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("7 unacknowledged in the 1024 records before #9"),
        "{out}"
    );
    assert!(!out.contains("runs_ledger_damaged"), "{out}");
    std::fs::remove_dir_all(&store).unwrap();
}

#[test]
fn every_form_of_the_listing_agrees_about_an_acknowledged_alert() {
    // IA-AL-18, the cli's half: 1,090 open alerts, then acknowledgements of
    // #1..#10 (records #1091..#1100), which sit outside the pages they name.
    let store = scratch("agree");
    for i in 1..=1_090_u32 {
        put(&store, 9, &format!("mismatch:{i}"), "x");
    }
    for k in 1..=10_u32 {
        alerts::ack_at(&store, k, "cli", Scope::Whole, NOW).unwrap();
    }
    let (code, out) = run(&store, &[]);
    assert_eq!(code, OK);
    assert!(
        out.contains("1014 unacknowledged in the newest 1024 records; 66 older unacknowledged"),
        "{out}"
    );
    assert!(
        out.contains("the page before: cli alerts --before 77"),
        "{out}"
    );
    let (_, out) = run(&store, &["--older"]);
    assert!(out.contains("ALERTS OLDER -- 66 unacknowledged"), "{out}");

    // The page the listing points at: the same 66, and none of #1..#10.
    let (code, out) = run(&store, &["--before", "77"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains(
            "66 unacknowledged in the 1024 records before #77; 0 unacknowledged older than \
             this page"
        ),
        "{out}"
    );
    for k in 1..=10 {
        assert!(!out.contains(&format!("  #{k}  ")), "#{k} is listed: {out}");
    }
    assert!(out.contains("  #11  mismatch") && out.contains("  #76  mismatch"));
    assert!(!out.contains("the page before"), "{out}");

    // A page that starts after them: its older count leaves them out too.
    let (_, out) = run(&store, &["--before", "1091"]);
    assert!(
        out.contains(
            "1024 unacknowledged in the 1024 records before #1091; 56 unacknowledged older \
             than this page"
        ),
        "{out}"
    );
    assert!(
        out.contains("the page before: cli alerts --before 67"),
        "{out}"
    );
    // And the acknowledgement itself says what the listing now shows.
    let (code, out) = run(&store, &["--ack", "5"]);
    assert_eq!(code, FAILED);
    assert!(out.contains("already_acknowledged: 5"), "{out}");

    // An acknowledgement nobody can read any more is not applied in silence:
    // the page says so, lists the alert again and does not exit clean.
    spoil(&store, 1_095);
    let (code, out) = run(&store, &["--before", "77"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("1 records after this page fail their CRC"),
        "{out}"
    );
    assert!(
        out.contains("67 unacknowledged in the 1024 records before #77") && out.contains("  #5  "),
        "{out}"
    );
    std::fs::remove_dir_all(&store).unwrap();
}

/// An open alert as a raw queue holds it.
fn raw_alert(i: u32, open_after: u32) -> alerts::Record {
    alerts::Record {
        kind_of_record: alerts::TYPE_ALERT,
        kind: 6,
        flags: 0,
        day: 18_682,
        run: 77,
        put_at: NOW,
        key: alerts::key_hash(&format!("invented:{i}")),
        acked: 0,
        open_after,
        text: format!("invented {i}"),
    }
}

/// Writes a queue whole from records, as earlier puts would have left it.
fn raw_queue(store: &Path, records: &[alerts::Record]) {
    let names = alerts::Names::under(store);
    std::fs::create_dir_all(&names.dir).unwrap();
    let mut bytes = alerts::header(NOW);
    for record in records {
        bytes.extend_from_slice(&record.encode());
    }
    std::fs::write(&names.queue, bytes).unwrap();
    std::fs::write(&names.lock, b"").unwrap();
}

#[test]
fn the_older_listing_names_what_it_cannot_read_and_exits_non_zero() {
    // IA-AL-21, the cli's half. No queue: no alert, and nothing is written.
    let store = scratch("older-named");
    let (code, out) = run(&store, &["--older"]);
    assert_eq!(code, OK);
    assert!(
        out.contains("ALERTS OLDER -- ") && out.contains("no queue yet, no alert"),
        "{out}"
    );
    assert!(!store.join("audit").exists(), "a listing never writes");

    // 1,200 open alerts, #50 damaged in place: older than the newest window.
    let open: Vec<alerts::Record> = (1..=1_200).map(|i| raw_alert(i, i)).collect();
    raw_queue(&store, &open);
    spoil(&store, 50);
    let (code, out) = run(&store, &[]);
    assert_eq!(code, OK, "the window itself is whole: {out}");
    assert!(
        out.contains("1024 unacknowledged in the newest 1024 records; 176 older unacknowledged"),
        "{out}"
    );
    let (code, out) = run(&store, &["--older"]);
    assert_eq!(code, FAILED, "a record nobody can read is not a clean exit");
    assert!(
        out.contains(
            "ALERTS OLDER -- 175 unacknowledged older than the newest 1024 records; 1 older \
             records fail their CRC; 1200 records"
        ),
        "{out}"
    );
    assert!(
        out.contains("  #50  DAMAGED: the record fails its CRC"),
        "{out}"
    );
    assert_eq!(out.matches("  #").count(), 176, "175 alerts and #50");
    assert!(out.contains("  #176  hole") && out.contains("  #1  hole"));
    // The page that holds it says the same thing the same way.
    let (code, page) = run(&store, &["--before", "177"]);
    assert_eq!(code, FAILED);
    assert!(
        page.contains("175 unacknowledged in the 1024 records before #177")
            && page.contains("  #50  DAMAGED: the record fails its CRC"),
        "{page}"
    );

    // A damaged ACKNOWLEDGEMENT older than the window (#62, of #5) beside a
    // later build's record (#61).
    let mut records: Vec<alerts::Record> = (1..=60).map(|i| raw_alert(i, i)).collect();
    let mut later = raw_alert(61, 60);
    later.kind_of_record = 3;
    records.push(later);
    records.push(alerts::Record {
        kind_of_record: alerts::TYPE_ACK,
        kind: 0,
        flags: 0,
        day: 0,
        run: 0,
        put_at: NOW,
        key: [0; 16],
        acked: 5,
        open_after: 59,
        text: "cli".to_owned(),
    });
    records.extend((63..=1_300).map(|i| raw_alert(i, i - 3)));
    raw_queue(&store, &records);
    let (code, out) = run(&store, &["--older"]);
    assert_eq!(code, OK, "a later build's record is named, not a failure");
    assert!(
        out.contains("ALERTS OLDER -- 273 unacknowledged")
            && out.contains("; 0 older records fail their CRC; 1300 records"),
        "{out}"
    );
    assert!(
        out.contains("  #61  UNKNOWN record type 3 (a later build's)"),
        "{out}"
    );
    assert!(!out.contains("  #5  "), "#5 is acknowledged: {out}");
    spoil(&store, 62);
    let (code, out) = run(&store, &["--older"]);
    assert_eq!(code, FAILED);
    assert!(
        out.contains("ALERTS OLDER -- 274 unacknowledged")
            && out.contains("; 1 older records fail their CRC"),
        "{out}"
    );
    assert!(out.contains("  #5  hole"), "listed again: {out}");
    assert!(
        out.contains("  #61  UNKNOWN record type 3 (a later build's)")
            && out.contains("  #62  DAMAGED: the record fails its CRC"),
        "{out}"
    );
    assert!(out.contains("an acknowledgement here is not applied"));

    // Damage inside the newest window: named by the default listing, counted
    // by `--older`, whose own alerts may depend on it.
    spoil(&store, 1_290);
    let (code, out) = run(&store, &["--older"]);
    assert_eq!(code, FAILED);
    assert!(
        out.contains("  1 of the newest 1024 records fail their CRC"),
        "{out}"
    );
    let (code, out) = run(&store, &[]);
    assert_eq!(code, FAILED);
    assert!(out.contains("  #1290  DAMAGED"), "{out}");

    // `--before` past the end is the newest page, and says so; at the end
    // plus one it is the records before that ordinal.
    let (_, out) = run(&store, &["--before", "5000"]);
    assert!(
        out.contains("in the newest 1024 records (#5000 is past the last record)"),
        "{out}"
    );
    let (_, out) = run(&store, &["--before", "1301"]);
    assert!(out.contains("in the 1024 records before #1301;"), "{out}");
    std::fs::remove_dir_all(&store).unwrap();
}

#[test]
fn a_damaged_queue_is_shown_replaced_and_its_kept_file_listed() {
    // IA-AL-07, the cli's half.
    let store = scratch("replace");
    for key in ["a", "b", "c", "d"] {
        put(&store, 6, key, key);
    }
    let (code, out) = run(&store, &["--replace-damaged"]);
    assert_eq!(code, FAILED);
    assert!(
        out.contains("ALERTS REFUSED: alerts_not_damaged"),
        "a healthy queue stays: {out}"
    );

    // One damaged record inside the window: shown, and the exit says so.
    spoil(&store, 2);
    let (code, out) = run(&store, &[]);
    assert_eq!(code, FAILED, "damage is never a clean exit");
    assert!(
        out.contains("#2  DAMAGED: the record fails its CRC"),
        "{out}"
    );
    assert!(out.contains("3 unacknowledged"), "{out}");

    // Two at the tail: every put refuses, and the listing still shows it.
    spoil(&store, 3);
    spoil(&store, 4);
    let (_, out) = run(&store, &[]);
    assert!(
        out.contains("#4  DAMAGED") && out.contains("#3  DAMAGED"),
        "{out}"
    );

    let (code, out) = run(&store, &["--replace-damaged"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("ALERT QUEUE REPLACED: kept "), "{out}");
    assert!(out.contains("unacknowledged alerts it held: 1;"), "{out}");
    let kept: Vec<PathBuf> = std::fs::read_dir(store.join("audit"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("alerts.damaged-")
        })
        .collect();
    assert_eq!(kept.len(), 1);
    assert!(
        out.contains(&format!("cli alerts --file {}", kept[0].display())),
        "{out}"
    );

    let (code, out) = run(&store, &[]);
    assert_eq!(code, OK);
    assert!(out.contains("#1  queue_replaced  run 0  day -"), "{out}");
    assert!(out.contains("open alerts it held: 1"), "{out}");

    // `--file` needs no store root: it lists the kept file's good records and
    // names its damaged ones.
    let mut out = String::new();
    let path = kept[0].to_string_lossy().into_owned();
    let code = command_with(&["--file", &path], &|| Err("no store".to_owned()), &mut out);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("4 records, 3 damaged, header ok, 0 torn bytes"),
        "{out}"
    );
    assert!(
        out.contains("#1  hole  run 77") && out.contains("#4  DAMAGED"),
        "{out}"
    );

    let (code, out) = run(&store, &["--file", "/no/such/kept/file"]);
    assert_eq!(code, FAILED);
    assert!(out.contains("ALERTS REFUSED: alerts_io"), "{out}");
    std::fs::remove_dir_all(&store).unwrap();
}

#[test]
fn a_later_builds_record_and_a_cut_text_are_printed_not_dropped() {
    let store = scratch("later");
    let long = "y".repeat(90);
    put(&store, 6, "k", &long);
    let (_, out) = run(&store, &[]);
    assert!(
        out.contains(&format!("{} [text cut]", "y".repeat(75))),
        "{out}"
    );

    // A record type and a kind this build cannot name, as a later build
    // would append them.
    let names = alerts::Names::under(&store);
    let mut later = alerts::Record {
        kind_of_record: 9,
        kind: 0,
        flags: 0,
        day: 0,
        run: 0,
        put_at: NOW,
        key: [0; 16],
        acked: 0,
        open_after: 1,
        text: String::new(),
    };
    let mut bytes = std::fs::read(&names.queue).unwrap();
    bytes.extend_from_slice(&later.encode());
    later.kind_of_record = alerts::TYPE_ALERT;
    later.kind = 77;
    later.open_after = 2;
    bytes.extend_from_slice(&later.encode());
    std::fs::write(&names.queue, &bytes).unwrap();
    let (code, out) = run(&store, &[]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("#3  kind_77  run 0  day -"), "{out}");
    assert!(
        out.contains("#2  UNKNOWN record type 9 (a later build's)"),
        "{out}"
    );
    let (_, out) = run(&store, &["--file", &names.queue.to_string_lossy()]);
    assert!(
        out.contains("#2  UNKNOWN record type 9 (a later build's), put 2026-10-05T"),
        "{out}"
    );
    std::fs::remove_dir_all(&store).unwrap();
}

#[test]
fn days_and_moments_are_printed_as_calendar_text() {
    assert_eq!(day_text(0), "1970-01-01");
    assert_eq!(day_text(18_682), "2021-02-24");
    assert_eq!(moment_text(0), "1970-01-01T00:00:00Z");
    assert_eq!(moment_text(86_399), "1970-01-01T23:59:59Z");
    assert_eq!(moment_text(1_614_168_000 + 3_661), "2021-02-24T13:01:01Z");
}

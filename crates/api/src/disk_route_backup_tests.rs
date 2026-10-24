//! `/disk.json`'s `backup` object over the receipt and drill lines `cli
//! backup` and `cli backup-drill` write (R4-09b, AUT-06; D-7137, D-7177..D-7179).
//! Every line here is built by the cli's own types, so the parser reads what
//! the writer writes, not a copy of its format.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

/// 2026-10-24T00:00:00Z.
const NOW: u64 = 1_792_800_000;

fn root(tag: &str) -> PathBuf {
    let root = crate::scratch::path(&format!("disk-backup-{tag}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("ops")).unwrap();
    std::fs::create_dir_all(root.join("target")).unwrap();
    root
}

fn receipt(dest: PathBuf, copied_at: u64) -> cli::BackupReceipt {
    cli::BackupReceipt {
        copied_at,
        dest,
        raw_bytes: 600,
        store_bytes: 913,
        run: 1_792_799_000,
        complete: true,
        files: 12,
        bytes: 1_513,
        copied_files: 12,
        copied_bytes: 1_513,
        unchanged: 0,
        busy: 0,
        missing: 0,
        unreadable: 0,
        commit: None,
    }
}

fn drill(from: PathBuf, result: &str) -> cli::BackupDrillLine {
    cli::BackupDrillLine {
        drilled_at: NOW - 3_600,
        from,
        drill: NOW - 3_600,
        result: result.to_owned(),
        months: 3,
        asked: 3,
        held: 9,
        files: 18,
        bytes: 4_096,
        moved: 1,
        failed: 0,
        busy: 0,
        scratch: PathBuf::from("/tmp/brutex-backup-drill-1"),
        commit: None,
        units: vec!["bars/gdfl/NSE/INDEX/NIFTY/1min/2025-01.bin".to_owned()],
    }
}

fn json(root: &Path, now: u64) -> serde_json::Value {
    serde_json::from_str(&backup_json(root, now)).expect("JSON")
}

/// **THE RECEIPT LINE IS READ BY THE EXISTING BACKUPS-LOG PARSER (D-7177).**
/// The line `cli backup` appends goes through `last_backup` — the one parser,
/// unchanged — as a recorded copy, and its words come out as numbers.
#[test]
fn the_receipt_line_is_read_by_the_existing_backups_log_parser() {
    let root = root("parser");
    let line = receipt(root.join("target"), NOW - 60).line();
    std::fs::write(root.join(BACKUP_RECEIPTS), format!("{line}\n")).unwrap();
    assert_eq!(
        BACKUP_RECEIPTS,
        cli::BACKUP_RECEIPTS,
        "one receipt log for both crates"
    );
    assert_eq!(last_backup(&root).expect("a recorded copy"), line);
    assert!(line.len() <= MAX_RECEIPT_LINE);
    let body = json(&root, NOW);
    assert_eq!(body["recorded"], true);
    assert_eq!(body["receipt"]["copied"], "2026-10-23T23:59:00Z");
    assert_eq!(body["receipt"]["by"], "cli-backup");
    assert_eq!(body["receipt"]["state"], "complete");
    assert_eq!(body["receipt"]["files"], 12);
    assert_eq!(body["receipt"]["bytes"], 1_513);
    assert_eq!(body["receipt"]["raw_bytes"], 600);
    assert_eq!(body["receipt"]["store_bytes"], 913);
    assert_eq!(
        body["receipt"]["dest"],
        root.join("target").display().to_string()
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **/disk.json SHOWS THE MANIFEST AND DRILL DATES (AUT-06).** The receipt's
/// date with the manifest's file count and bytes, the target present, not
/// stale, and the last drill's date and result — in the whole answer.
#[test]
fn disk_json_shows_the_manifest_and_drill_dates() {
    let root = root("dates");
    let target = root.join("target");
    std::fs::write(
        root.join(BACKUP_RECEIPTS),
        format!("{}\n", receipt(target.clone(), NOW - 86_400).line()),
    )
    .unwrap();
    std::fs::write(
        root.join(cli::BACKUP_DRILLS),
        format!("{}\n", drill(target.clone(), "pass").line()),
    )
    .unwrap();
    let body = json(&root, NOW);
    assert_eq!(body["receipt"]["copied"], "2026-10-23T00:00:00Z");
    assert_eq!(body["receipt"]["files"], 12);
    assert_eq!(body["target"], "present");
    assert_eq!(body["age_days"], 1);
    assert_eq!(body["stale"], false);
    assert!(body["stale_why"].is_null());
    assert_eq!(body["stale_after_days"], STALE_COPY_DAYS);
    assert_eq!(body["drill"]["recorded"], true);
    assert_eq!(body["drill"]["drilled"], "2026-10-23T23:00:00Z");
    assert_eq!(body["drill"]["result"], "pass");
    assert_eq!(body["drill"]["months"], 3);
    assert_eq!(body["drill"]["moved"], 1);
    assert_eq!(body["drill"]["from"], target.display().to_string());
    // AND IN THE WHOLE ANSWER the page reads.
    let whole: serde_json::Value = serde_json::from_str(&measured(&root, None)).expect("JSON");
    assert_eq!(whole["backup"]["receipt"]["files"], 12);
    assert_eq!(whole["backup"]["drill"]["result"], "pass");
    let _ = std::fs::remove_dir_all(&root);
}

/// **A MISSING TARGET READS AS BACKUP STALE (AUT-06).** The second drive
/// unplugged: the receipt still stands, and the page says by name that the
/// backup is stale and why.
#[test]
fn a_missing_target_reads_as_backup_stale() {
    let root = root("missing");
    let gone = root.join("unplugged");
    std::fs::write(
        root.join(BACKUP_RECEIPTS),
        format!("{}\n", receipt(gone, NOW - 60).line()),
    )
    .unwrap();
    let body = json(&root, NOW);
    assert_eq!(body["recorded"], true);
    assert_eq!(body["target"], "missing");
    assert_eq!(body["stale"], true);
    assert_eq!(
        body["stale_why"],
        "backup stale: last copy 2026-10-23T23:59:00Z; target missing"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Every other way a copy goes stale is named too, each computed when read:
/// too old, dated after now, partial, the target a file, more than one at once;
/// no receipt at all; a receipt written by hand carries no `cli backup` words
/// and its path is read as written.
#[test]
fn every_way_a_copy_goes_stale_is_named() {
    let root = root("stale");
    let target = root.join("target");
    let write =
        |line: String| std::fs::write(root.join(BACKUP_RECEIPTS), format!("{line}\n")).unwrap();
    write(receipt(target.clone(), NOW - 9 * 86_400).line());
    assert_eq!(
        json(&root, NOW)["stale_why"],
        "backup stale: last copy 2026-10-15T00:00:00Z; 9 days old (more than 7)"
    );
    write(receipt(target.clone(), NOW + 60).line());
    assert_eq!(
        json(&root, NOW)["stale_why"],
        "backup stale: last copy 2026-10-24T00:01:00Z; the copy is dated after now"
    );
    assert!(json(&root, NOW)["age_days"].is_null());
    let mut partial = receipt(target.clone(), NOW);
    partial.complete = false;
    partial.busy = 2;
    write(partial.line());
    let body = json(&root, NOW);
    assert_eq!(body["receipt"]["busy"], 2);
    assert_eq!(
        body["stale_why"],
        "backup stale: last copy 2026-10-24T00:00:00Z; the last run was partial"
    );
    // COMBINED: old, unplugged and partial at once, every reason said.
    let mut all = receipt(root.join("gone"), NOW - 30 * 86_400);
    all.complete = false;
    write(all.line());
    assert_eq!(
        json(&root, NOW)["stale_why"],
        "backup stale: last copy 2026-09-24T00:00:00Z; 30 days old (more than 7); target \
         missing; the last run was partial"
    );
    std::fs::write(root.join("afile"), "x").unwrap();
    write(receipt(root.join("afile"), NOW).line());
    assert_eq!(json(&root, NOW)["target"], "not_a_directory");
    // A RECEIPT BY HAND, as the runbook wrote them before the verb.
    write(format!(
        "copied=2026-10-23T00:00Z dest={} raw_bytes=1 store_bytes=2 by=operator",
        target.display()
    ));
    let body = json(&root, NOW);
    assert_eq!(body["recorded"], true);
    assert!(body["receipt"]["files"].is_null() && body["receipt"]["run"].is_null());
    assert_eq!(body["receipt"]["raw_bytes"], 1);
    assert_eq!(body["receipt"]["by"], "operator");
    assert_eq!(body["target"], "present");
    assert_eq!(body["stale"], false);
    // NO RECEIPT: stale, said.
    std::fs::remove_file(root.join(BACKUP_RECEIPTS)).unwrap();
    let body = json(&root, NOW);
    assert_eq!(body["recorded"], false);
    assert_eq!(body["stale"], true);
    assert_eq!(
        body["stale_why"],
        "backup stale: no copy of this store is recorded"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The drill log: never run, said; a newest line that is no drill line is no
/// drill, said; a torn last line leaves the whole one before it standing; a
/// failed drill is served as `fail`.
#[test]
fn only_a_whole_drill_line_is_a_recorded_drill() {
    let root = root("drill");
    let none = json(&root, NOW);
    assert_eq!(none["drill"]["recorded"], false);
    assert!(
        none["drill"]["why"]
            .as_str()
            .unwrap()
            .contains("no restore drill is recorded"),
        "{none}"
    );
    let good = drill(root.join("target"), "fail").line();
    for (log, recorded) in [
        (format!("{good}\nrsync: oops\n"), false),
        (format!("{good}\ndrilled=banana result=pass\n"), false),
        (format!("{good}\ndrilled=2026-10-23T00:00:00Z\n"), false),
        (format!("{good}\ndrilled=2026-10-23T00:00:00Z res"), true),
        (String::new(), false),
    ] {
        std::fs::write(root.join(cli::BACKUP_DRILLS), &log).unwrap();
        let body = json(&root, NOW);
        assert_eq!(body["drill"]["recorded"], recorded, "{log:?}: {body}");
        if recorded {
            assert_eq!(body["drill"]["result"], "fail");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// A UTC instant in Unix seconds, at the edges a receipt can carry.
#[test]
fn an_instant_is_read_in_unix_seconds() {
    assert_eq!(instant_secs("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(instant_secs("2026-10-24T00:00Z"), Some(NOW));
    assert_eq!(instant_secs("2024-02-29T23:59:59Z"), Some(1_709_251_199));
    assert_eq!(
        instant_secs("1969-12-31T23:59:59Z"),
        None,
        "before the epoch"
    );
    assert_eq!(instant_secs("2026-02-30T00:00Z"), None);
    assert_eq!(instant_secs("banana"), None);
}

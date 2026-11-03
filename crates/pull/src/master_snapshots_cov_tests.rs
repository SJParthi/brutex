//! BASE-COV-PULL: the refusals of the dated master snapshots no test had
//! reached (Route B, recipe v4): names, receipts, bounds, the diff's own
//! records and the token walk's unreadable neighbour. Every row is invented
//! (`ZQIDX`, tokens in the nine millions).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use std::os::unix::fs::PermissionsExt as _;

use super::tests::{HEAD as HEAD_TEXT, day, dump, fut, scratch, share, stamp};
use super::*;

const URL: &str = "https://invented.example/instruments";

#[test]
fn a_date_of_any_other_shape_is_no_date() {
    assert_eq!(parse_compact("2026010"), None, "seven digits");
    assert_eq!(parse_compact("2026a102"), None, "a letter");
    assert_eq!(parse_compact("20260230"), None, "no such day");
    assert_eq!(parse_compact("20260102"), Some(day(2026, 1, 2)));
    assert_eq!(parse_dashed("2026-01-02-03"), None, "a fourth part");
    assert_eq!(parse_dashed("26-01-02"), None);
    assert_eq!(parse_dashed("2026-1-02"), None);
    assert_eq!(parse_dashed("2026-01-2"), None);
    assert_eq!(parse_dashed("2026-01-02"), Some(day(2026, 1, 2)));
}

#[test]
fn an_ordinal_is_two_or_more_with_no_leading_zero() {
    assert_eq!(parse_ordinal("02"), None);
    assert_eq!(parse_ordinal("x2"), None);
    assert_eq!(parse_ordinal("1"), None);
    assert_eq!(parse_ordinal("2"), Some(2));
    assert_eq!(parse_ordinal("99999999999"), None, "past a u32");
}

/// A root that is a file is named, not called empty; a missing root is empty.
#[test]
fn a_listing_of_a_file_is_refused_and_of_nothing_is_empty() {
    let root = scratch("cov-list");
    let file = root.join("a-file");
    fs::write(&file, b"x").unwrap();
    let why = Catalog::read(&file).unwrap_err();
    assert!(why.contains("could not be listed"), "{why}");
    let below = file.join("below");
    let why = read_quiesced(&below).unwrap_err();
    assert!(why.contains("could not be read"), "{why}");
    assert!(
        Catalog::read(&root.join("missing"))
            .unwrap()
            .incomplete_of(Kind::Kite)
            .is_empty()
    );
}

/// A receipt with no body is incomplete, and named by the receipt's own file.
#[test]
fn a_lone_receipt_is_an_incomplete_entry_named_by_its_own_file() {
    let root = scratch("cov-lone");
    let at = stamp(day(2026, 3, 2), 1);
    fs::write(root.join(receipt_name(Kind::Kite, at)), b"no body").unwrap();
    let held = Catalog::read(&root).unwrap().incomplete_of(Kind::Kite);
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].file, receipt_name(Kind::Kite, at));
    assert_eq!(held[0].why, LONE_RECEIPT);
}

/// Other kinds' and other days' entries are not counted against a day.
#[test]
fn a_days_count_of_entries_counts_that_kind_and_day_alone() {
    let root = scratch("cov-held-on");
    let (on, other) = (day(2026, 3, 2), day(2026, 3, 3));
    for (kind, at) in [
        (Kind::Kite, stamp(on, 1)),
        (Kind::Kite, stamp(other, 1)),
        (Kind::NseTotalMarket, stamp(on, 1)),
    ] {
        fs::write(root.join(file_name(kind, at)), b"cut").unwrap();
    }
    let catalog = Catalog::read(&root).unwrap();
    assert_eq!(catalog.held_on(Kind::Kite, on), (0, 1));
    assert_eq!(catalog.held_on(Kind::Kite, day(2026, 3, 4)), (0, 0));
}

#[test]
fn a_directory_or_an_oversized_file_is_not_read_as_a_receipt() {
    let root = scratch("cov-limited");
    let why = read_limited(&root, 10).unwrap_err();
    assert!(why.contains("not a regular file within 10 bytes"), "{why}");
    let big = root.join("big");
    fs::write(&big, [0_u8; 11]).unwrap();
    assert!(read_limited(&big, 10).is_err());
    assert_eq!(read_limited(&big, 11).unwrap().len(), 11, "at the limit");
}

#[test]
fn a_receipt_that_is_not_utf8_is_refused_never_read() {
    let root = scratch("cov-receipt-bytes");
    let at = stamp(day(2026, 3, 2), 1);
    fs::write(root.join(receipt_name(Kind::Kite, at)), [0xFF, 0xFE]).unwrap();
    let why = read_checked(&root, Kind::Kite, at).unwrap_err();
    assert!(why.contains("is not UTF-8"), "{why}");
}

fn receipt_of(body: &[u8]) -> Receipt {
    Receipt {
        url: URL.to_owned(),
        compressed_bytes: 0,
        sha256: String::new(),
        body_bytes: body.len() as u64,
        body_sha256: hex(&Sha256::digest(body)),
    }
}

/// Bytes past the one gzip member, a length and a digest that are not the
/// receipt's: each is refused by name, whatever the rest says.
#[test]
fn a_body_is_held_to_its_receipt_on_every_count() {
    let body = b"a body of no vendor";
    let good = compress(body).unwrap();
    assert_eq!(expand(&good, &receipt_of(body)).unwrap().as_bytes(), body);
    let mut trailing = good.clone();
    trailing.push(0);
    let why = expand(&trailing, &receipt_of(body)).unwrap_err();
    assert!(why.contains("bytes after its gzip member"), "{why}");
    let mut long = receipt_of(body);
    long.body_bytes += 1;
    assert!(expand(&good, &long).unwrap_err().contains("does not match"));
    let mut other = receipt_of(body);
    other.body_sha256 = "0".repeat(64);
    assert!(
        expand(&good, &other)
            .unwrap_err()
            .contains("does not match")
    );
    let binary = compress(&[0xFF]).unwrap();
    let why = expand(&binary, &receipt_of(&[0xFF])).unwrap_err();
    assert!(why.contains("not UTF-8"), "{why}");
}

#[test]
fn a_sync_of_a_directory_that_is_gone_is_named() {
    let root = scratch("cov-sync").join("gone");
    let why = sync_dir(&root).unwrap_err();
    assert!(why.contains("crash durability UNVERIFIED"), "{why}");
}

#[test]
fn a_write_refuses_each_body_url_and_root_it_cannot_receipt() {
    let root = scratch("cov-write");
    let on = day(2026, 3, 2);
    let body = dump(&[fut(9_000_001, "ZQIDX26MARFUT", "2026-03-31", 75)]);
    let over = vec![0_u8; MAX_BODY_BYTES + 1];
    let why = snapshot(&root, Kind::Kite, on, URL, &over).unwrap_err();
    assert!(why.contains("byte bound; no snapshot was written"), "{why}");
    let why = snapshot(&root, Kind::Kite, on, "https://a\nb", body.as_bytes()).unwrap_err();
    assert!(why.contains("line break"), "{why}");
    let file = root.join("a-file");
    fs::write(&file, b"x").unwrap();
    let why = snapshot(&file, Kind::Kite, on, URL, body.as_bytes()).unwrap_err();
    assert!(why.contains("could not be created"), "{why}");
    let locked = scratch("cov-write-lock");
    fs::create_dir(locked.join(LOCK_FILE)).unwrap();
    let why = snapshot(&locked, Kind::Kite, on, URL, body.as_bytes()).unwrap_err();
    assert!(why.contains("could not be opened for locking"), "{why}");
}

/// A root the writer can enter and not list: the lock opens, the listing
/// under it does not, and both answers carry the same reason.
#[test]
fn a_root_that_will_not_list_under_the_lock_is_refused_on_both_counts() {
    let root = scratch("cov-write-unlisted");
    fs::write(root.join(LOCK_FILE), b"").unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o300)).unwrap();
    let on = day(2026, 3, 2);
    let written = write_snapshot(&root, Kind::Kite, on, URL, b"a body");
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let why = written.wrote.unwrap_err();
    assert!(why.contains("could not be listed"), "{why}");
    assert_eq!(written.incomplete.unwrap_err(), why);
}

/// A receipt that no longer checks, or states another length of one digest,
/// is not "already held": the same bytes take the next ordinal.
#[test]
fn only_a_receipt_that_checks_and_agrees_is_already_held() {
    let root = scratch("cov-held");
    let on = day(2026, 3, 2);
    let body = dump(&[fut(9_000_001, "ZQIDX26MARFUT", "2026-03-31", 75)]);
    let first = snapshot(&root, Kind::Kite, on, URL, body.as_bytes()).unwrap();
    let at = first.stamp();
    let path = root.join(receipt_name(Kind::Kite, at));
    let text = fs::read_to_string(&path).unwrap();
    let stated = format!("body_bytes={}\n", body.len());
    let lying = text.replace(&stated, &format!("body_bytes={}\n", body.len() + 1));
    assert_ne!(lying, text);
    fs::remove_file(&path).unwrap();
    fs::write(&path, lying).unwrap();
    let next = snapshot(&root, Kind::Kite, on, URL, body.as_bytes()).unwrap();
    assert_eq!(next, Wrote::New(stamp(on, 2)));
    let path = root.join(receipt_name(Kind::Kite, next.stamp()));
    fs::remove_file(&path).unwrap();
    fs::write(&path, "not a receipt").unwrap();
    let third = snapshot(&root, Kind::Kite, on, URL, body.as_bytes()).unwrap();
    assert_eq!(third, Wrote::New(stamp(on, 3)));
}

/// A stale listing: the ordinal it gives already has a receipt. The body lands
/// as an incomplete entry and the receipt is refused, never overwritten.
#[test]
fn a_receipt_that_appeared_after_the_listing_is_left_as_it_is() {
    let root = scratch("cov-stale-catalog");
    let on = day(2026, 3, 2);
    let at = stamp(on, 1);
    fs::write(root.join(receipt_name(Kind::Kite, at)), b"someone else's").unwrap();
    let body = b"a body";
    let compressed = compress(body).unwrap();
    let why = write_held(
        &root,
        Kind::Kite,
        on,
        URL,
        body,
        &compressed,
        &Catalog::default(),
    )
    .unwrap_err();
    assert!(why.contains("already exists and is left as it is"), "{why}");
    assert_eq!(
        fs::read(root.join(receipt_name(Kind::Kite, at))).unwrap(),
        b"someone else's"
    );
}

#[test]
fn today_is_a_day_the_clock_can_name() {
    assert!(today_ist().unwrap() >= day(2026, 1, 1));
}

#[test]
fn a_csv_line_keeps_quotes_doubled_and_inner_and_refuses_one_left_open() {
    assert_eq!(split_line(r#""a""b",c"#).unwrap(), [r#"a"b"#, "c"]);
    assert_eq!(split_line(r#"ab"c,d"#).unwrap(), [r#"ab"c"#, "d"]);
    assert_eq!(split_line(",").unwrap(), ["", ""]);
    assert_eq!(split_line("").unwrap(), [""]);
    assert!(split_line(r#""open"#).is_err());
}

#[test]
fn a_decimal_is_unsigned_digits_or_nothing() {
    assert_eq!(decimal(""), None);
    assert_eq!(decimal(".5"), None);
    assert_eq!(decimal("1a"), None);
    assert_eq!(decimal("1.5a"), None);
    assert_eq!(decimal("-1"), None);
    assert_eq!(decimal(" 007.500 ").as_deref(), Some("7.5"));
    assert_eq!(decimal("0.0").as_deref(), Some("0"));
}

/// A blank line in a dump is passed over wherever it falls, a dump of the
/// header alone is empty, and each row answers its own columns.
#[test]
fn a_snapshot_reads_past_blank_lines_and_answers_for_each_row() {
    let at = stamp(day(2026, 3, 2), 1);
    let row = fut(9_000_001, "ZQIDX26MARFUT", "2026-03-31", 75);
    let body = format!("{HEAD_TEXT}\n\n{row}\r\n\n");
    let held = Snapshot::parse(at, &body).unwrap();
    assert!(!held.is_empty());
    let listed = held.rows().next().unwrap();
    assert_eq!(listed.tick_size(), "0.05");
    assert_eq!(listed.stamp(), at);
    let bare = Snapshot::parse(at, &format!("{HEAD_TEXT}\n")).unwrap();
    assert!(bare.is_empty());
}

/// Each column of the contract's key tells two rows apart on its own, and a
/// share is a cash row of NSE and nothing else that looks like one.
#[test]
fn a_contract_differs_in_any_column_of_its_key() {
    let at = stamp(day(2026, 3, 2), 1);
    let row =
        |exchange: &str, symbol: &str, expiry: &str, strike: &str, kind: &str, segment: &str| {
            super::tests::row(
                9_000_001, symbol, "ZQIDX", expiry, strike, 75, kind, segment, exchange,
            )
        };
    let one = |text: String| Snapshot::parse(at, &dump(&[text])).unwrap();
    let base = one(row("NFO", "ZQA", "2026-03-31", "100", "CE", "NFO-OPT"));
    let listed = base.rows().next().unwrap();
    for other in [
        one(row("BFO", "ZQA", "2026-03-31", "100", "CE", "NFO-OPT")),
        one(row("NFO", "ZQB", "2026-03-31", "100", "CE", "NFO-OPT")),
        one(row("NFO", "ZQA", "2026-04-30", "100", "CE", "NFO-OPT")),
        one(row("NFO", "ZQA", "2026-03-31", "200", "CE", "NFO-OPT")),
        one(row("NFO", "ZQA", "2026-03-31", "100", "PE", "NFO-OPT")),
    ] {
        assert!(!listed.same_contract(&other.rows().next().unwrap()));
    }
    assert!(listed.same_contract(&listed));
    let share_of = |exchange: &str, kind: &str, segment: &str| {
        let held = one(row(exchange, "ZQS", "", "0", kind, segment));
        held.rows().next().unwrap().is_nse_share()
    };
    assert!(share_of("NSE", "EQ", "NSE"));
    assert!(!share_of("NSE", "EQ", "INDICES"));
    assert!(!share_of("NSE", "FUT", "NSE"));
    assert!(!share_of("BSE", "EQ", "NSE"));
}

#[test]
fn a_book_refuses_a_span_past_its_bound_and_an_index_past_its_end() {
    let empty = |on: Day| Snapshot::parse(stamp(on, 1), &format!("{HEAD_TEXT}\n")).unwrap();
    let why = Book::of(vec![empty(day(1970, 1, 1)), empty(day(2026, 1, 1))]).unwrap_err();
    assert!(why.contains("a misnamed file is refused"), "{why}");
    let book = Book::of(vec![empty(day(2026, 1, 1))]).unwrap();
    assert!(book.snapshot(0).is_ok());
    assert_eq!(book.snapshot(1).unwrap_err(), "no snapshot 1 in this book");
    assert_eq!(
        book.with_snapshot(usize::MAX, |_| ()).unwrap_err(),
        format!("no snapshot {} in this book", usize::MAX)
    );
}

/// The token walk needs the next later snapshot; damaged, it answers
/// `Unreadable` naming that snapshot, never the older row.
#[test]
fn a_later_snapshot_that_will_not_read_leaves_the_token_unread() {
    let root = scratch("cov-later");
    let (first, later) = (day(2026, 3, 2), day(2026, 3, 9));
    let row = fut(9_000_001, "ZQIDX26MARFUT", "2026-03-31", 75);
    for on in [first, later] {
        snapshot(
            &root,
            Kind::Kite,
            on,
            URL,
            dump(std::slice::from_ref(&row)).as_bytes(),
        )
        .unwrap();
    }
    let body = root.join(file_name(Kind::Kite, stamp(later, 1)));
    fs::remove_file(&body).unwrap();
    fs::write(&body, b"damaged").unwrap();
    let book = Book::open(&root).unwrap();
    let on = book.token_on(day(2026, 3, 4), "NFO", 9_000_001);
    assert_eq!(on.word(), "snapshot_unreadable");
    let on_first = book.token_on(first, "NFO", 9_000_001);
    assert_eq!(on_first.word(), "resolved");
}

/// A re-dated contract that also changed its lot says both; an ISIN the same
/// on both dates says nothing; a token renamed away with its old key still
/// listed is a new listing, not a rename.
#[test]
fn a_diff_tells_each_change_of_a_token_apart() {
    let (a, b) = (stamp(day(2026, 3, 2), 1), stamp(day(2026, 3, 3), 1));
    let parse = |at: Stamp, rows: &[String]| Snapshot::parse(at, &dump(rows)).unwrap();
    let old = parse(a, &[fut(9_000_001, "ZQIDX26MARFUT", "2026-03-31", 75)]);
    let new = parse(b, &[fut(9_000_001, "ZQIDX26MARFUT", "2026-03-30", 50)]);
    let words: Vec<&str> = diff(&old, &new, None, None)
        .iter()
        .map(|item| item.change.word())
        .collect();
    assert!(
        words.contains(&"expiry") && words.contains(&"lot_size"),
        "{words:?}"
    );
    let cash = |at: Stamp| parse(at, &[share(9_000_002, "ZQSHARE", "ZQ SHARE")]);
    let same = Isins::parse(&super::tests::nse_file(&[("ZQSHARE", "INE000INVENT")])).unwrap();
    assert_eq!(diff(&cash(a), &cash(b), Some(&same), Some(&same)), []);
    let kept = parse(
        b,
        &[
            fut(9_000_001, "ZQIDX26MARFUT", "2026-03-31", 75),
            fut(9_000_001, "ZQIDX26MAYFUT", "2026-05-31", 75),
        ],
    );
    let words: Vec<&str> = diff(&old, &kept, None, None)
        .iter()
        .map(|item| item.change.word())
        .collect();
    assert_eq!(words, ["listing"]);
}

#[test]
fn a_diff_text_of_any_other_shape_is_refused_by_name() {
    let pad = |line: &str| {
        let mut text = line.to_owned();
        text.extend(core::iter::repeat_n(' ', RECORD_LEN - 1 - text.len()));
        text.push('\n');
        text
    };
    assert!(
        parse_diff(&pad("not-a-diff|"))
            .unwrap_err()
            .contains("header")
    );
    let head = format!("{DIFF_FORMAT}|previous=a|current=b|records=1|isin_unknown_total=0|");
    assert_eq!(
        parse_diff(&pad(&head)).unwrap_err(),
        "the header counts 1 records and the file holds 0"
    );
    let short = format!("{}{}", pad(&head), pad("listing|NFO"));
    assert_eq!(
        parse_diff(&short).unwrap_err(),
        "a diff record has fewer than six fields"
    );
    let wide = Stamp {
        day: day(2026, 3, 2),
        ordinal: u32::MAX,
    };
    let huge = DiffHead {
        previous: "p".repeat(RECORD_LEN),
        current: wide.to_string(),
        records: 0,
        isin_unknown_total: 0,
    };
    assert_eq!(
        header_line(&huge).unwrap_err(),
        "the diff header is wider than the record length"
    );
}

/// The two writers' race: a diff that appears between the check and the lock
/// is read, not written over; a name with nothing readable behind it is
/// refused. Both are the path a second refresh takes.
#[test]
fn a_diff_is_never_written_over_and_a_directory_at_its_name_is_refused() {
    let root = scratch("cov-diff-name");
    let (a, b) = (day(2026, 3, 2), day(2026, 3, 3));
    let row = fut(9_000_001, "ZQIDX26MARFUT", "2026-03-31", 75);
    for on in [a, b] {
        snapshot(
            &root,
            Kind::Kite,
            on,
            URL,
            dump(std::slice::from_ref(&row)).as_bytes(),
        )
        .unwrap();
    }
    let path = root.join(diff_name(stamp(b, 1)));
    fs::create_dir(&path).unwrap();
    let why = diff_for(&root, stamp(b, 1)).unwrap_err();
    assert!(why.contains("could not be read"), "{why}");
}

#[test]
fn a_partial_name_held_by_each_ordinal_stops_the_diff_by_name() {
    let root = scratch("cov-diff-partials");
    let at = stamp(day(2026, 3, 3), 1);
    for attempt in 1..=MAX_ORDINAL {
        fs::write(root.join(diff_partial_name(at, attempt)), b"cut").unwrap();
    }
    let path = root.join(diff_name(at));
    let why = commit_diff(&root, at, &path, b"a diff").unwrap_err();
    assert!(why.contains("is held by a cut write"), "{why}");
    let missing = root.join("not-a-directory");
    fs::write(&missing, b"x").unwrap();
    let why = commit_diff(&missing, at, &path, b"a diff").unwrap_err();
    assert!(why.contains("no existing file was touched"), "{why}");
}

#[test]
fn a_hook_answer_carries_each_refusal_as_json() {
    let refused = Snapshotted {
        diff: Some(Err("an invented refusal".to_owned())),
        incomplete: Err("an unlisted folder".to_owned()),
        ..sample()
    };
    let json = refused.json();
    assert!(
        json.contains(r#"{"refusal":"an invented refusal"}"#),
        "{json}"
    );
    assert!(
        json.contains(r#""held":null,"named":[],"unnamed":null,"refusal":"an unlisted folder""#),
        "{json}"
    );
}

fn sample() -> Snapshotted {
    Snapshotted {
        kind: Kind::Kite,
        day: Some(day(2026, 3, 2)),
        wrote: Ok(Wrote::AlreadyHeld(stamp(day(2026, 3, 2), 1))),
        diff: None,
        incomplete: Ok(Vec::new()),
    }
}

/// Every shape of a hook's outcome reaches its events without a sink as well
/// as with one, and none of them panics (a combination: a refused write, a
/// listing that failed and a diff that failed, at once).
#[test]
fn every_outcome_of_a_hook_can_be_said() {
    note(&sample());
    note(&Snapshotted {
        wrote: Err("an invented refusal".to_owned()),
        incomplete: Err("an unlisted folder".to_owned()),
        diff: Some(Err("no diff".to_owned())),
        ..sample()
    });
    note(&Snapshotted {
        diff: Some(Err("no diff".to_owned())),
        ..sample()
    });
    note(&Snapshotted {
        diff: Some(Ok(None)),
        ..sample()
    });
}

// ───── BASE-COV-PULL repair 2 (D-7334) ─────

/// The text of field `key` of `record`, or `None`.
fn text(record: &telemetry::Record, key: &str) -> Option<String> {
    record
        .field(key)
        .and_then(telemetry::OwnedValue::as_str)
        .map(str::to_owned)
}

/// The count of field `key` of `record`, or `None`.
fn count(record: &telemetry::Record, key: &str) -> Option<u64> {
    record.field(key).and_then(telemetry::OwnedValue::as_u64)
}

/// **EVERY EVENT OF A HOOK CALL REACHES A SINK WITH ITS FIELDS** (repair 2):
/// a listing that failed is one `Error` naming why; incomplete entries past
/// the bound are named newest first, at most [`INCOMPLETE_NAMED`], and one
/// more event counts the rest; the outcome names its kind, day, stamp,
/// outcome word, previous snapshot, counts, diff file, detail and how many
/// entries are incomplete; and a diff naming more changes than one refresh
/// names says how many it did not, then names each of the first
/// [`DIFF_NAMED`]. A refused write says `refused` with no day. Run in a
/// process of its own, the one install there: these events are built only
/// when a sink would write them (`emit_if!`). Extremes at once: one past each
/// bound, a stamp past every incomplete entry's ordinal, and a refusal with
/// no day.
#[test]
fn every_event_of_a_hook_call_reaches_a_sink_with_its_fields() {
    crate::support::where_permission_binds(
        "master_snapshots::cov_tests::every_event_of_a_hook_call_reaches_a_sink_with_its_fields",
        every_event_of_a_hook_call_reaches_a_sink_with_its_fields_body,
    );
}

fn every_event_of_a_hook_call_reaches_a_sink_with_its_fields_body() {
    let home = scratch("cov-hook-sink");
    let sink =
        telemetry::install(&telemetry::Config::new(&home).with_min_level(telemetry::Level::Trace))
            .expect("the only install in this process");
    let on = day(2026, 3, 2);
    let held = u32::try_from(INCOMPLETE_NAMED).unwrap() + 1;
    let incomplete: Vec<Incomplete> = (1..=held)
        .map(|ordinal| Incomplete {
            kind: Kind::Kite,
            stamp: stamp(on, ordinal),
            file: format!("invented-{ordinal}.partial"),
            why: "an invented cut",
        })
        .collect();
    let changes = u64::try_from(DIFF_NAMED).unwrap() + 1;
    let summary = DiffSummary {
        previous: stamp(day(2026, 2, 27), 1),
        current: stamp(on, held + 1),
        file: "invented.diff".to_owned(),
        counts: [changes, 0, 0, 0, 0, 0, 0, 0],
        isin_unknown_total: 0,
        named: (0..changes)
            .map(|token| Item {
                change: Change::Listing,
                exchange: "NFO".to_owned(),
                token: 9_100_000 + token,
                key: format!("ZQIDX{token}"),
                old: String::new(),
                new: format!("ZQIDX{token}"),
            })
            .collect(),
    };
    let counts_line = summary.counts_line();
    note_incomplete(Kind::Kite, &Err("an invented listing fault".to_owned()));
    note(&Snapshotted {
        kind: Kind::Kite,
        day: Some(on),
        wrote: Ok(Wrote::New(stamp(on, held + 1))),
        diff: Some(Ok(Some(summary))),
        incomplete: Ok(incomplete),
    });
    note(&Snapshotted {
        kind: Kind::NseTotalMarket,
        day: None,
        wrote: Err("an invented refusal".to_owned()),
        diff: None,
        incomplete: Ok(Vec::new()),
    });
    let records = telemetry::tail(
        &home,
        sink.keep_files(),
        &telemetry::Query::last(telemetry::MAX_LIMIT).from_target(EVENT_TARGET),
    )
    .records;
    let of = |message: &str| -> Vec<&telemetry::Record> {
        records
            .iter()
            .filter(|record| record.message == message)
            .collect()
    };
    let unlisted =
        of("the snapshots directory could not be listed, so its incomplete entries are not named");
    assert_eq!(unlisted.len(), 1);
    assert_eq!(
        (text(unlisted[0], "kind"), text(unlisted[0], "why")),
        (
            Some("instruments".to_owned()),
            Some("an invented listing fault".to_owned())
        )
    );
    // A tail answers newest first: the first event a call wrote is last.
    let named = of(INCOMPLETE_MESSAGE);
    assert_eq!(named.len(), INCOMPLETE_NAMED);
    assert_eq!(
        text(named[0], "entry"),
        Some("invented-2.partial".to_owned()),
        "the 64th named is the second oldest entry"
    );
    let first = named[INCOMPLETE_NAMED - 1];
    assert_eq!(
        (
            text(first, "entry"),
            text(first, "stamp"),
            text(first, "day"),
            text(first, "why"),
            count(first, "held"),
        ),
        (
            Some(format!("invented-{held}.partial")),
            Some(stamp(on, held).to_string()),
            Some(on.to_string()),
            Some("an invented cut".to_owned()),
            Some(u64::from(held)),
        ),
        "the newest entry is named first"
    );
    let more = of("more incomplete master snapshot entries are held than one refresh names");
    assert_eq!(more.len(), 1);
    assert_eq!(
        (count(more[0], "unnamed"), count(more[0], "held")),
        (Some(1), Some(u64::from(held)))
    );
    let wrote = of("a dated master snapshot was written");
    assert_eq!(wrote.len(), 1);
    let fields: Vec<Option<String>> = [
        "kind", "day", "stamp", "outcome", "previous", "counts", "diff",
    ]
    .iter()
    .map(|key| text(wrote[0], key))
    .collect();
    assert_eq!(
        fields,
        [
            Some("instruments".to_owned()),
            Some(on.to_string()),
            Some(stamp(on, held + 1).to_string()),
            Some("new".to_owned()),
            Some(stamp(day(2026, 2, 27), 1).to_string()),
            Some(counts_line),
            Some("invented.diff".to_owned()),
        ]
    );
    assert!(
        text(wrote[0], "detail")
            .unwrap_or_default()
            .contains(&format!(
                "{} entries of the day's earlier ordinals",
                INCOMPLETE_NAMED + 1
            )),
        "{:?}",
        wrote[0]
    );
    assert_eq!(count(wrote[0], "incomplete"), Some(u64::from(held)));
    let bounded = of(
        "more master changes than one refresh names; the rest are counted here and every one is in the .diff file",
    );
    assert_eq!(bounded.len(), 1);
    assert_eq!(
        (
            count(bounded[0], "named"),
            count(bounded[0], "unnamed"),
            text(bounded[0], "diff"),
            text(bounded[0], "day"),
        ),
        (
            Some(u64::try_from(DIFF_NAMED).unwrap()),
            Some(1),
            Some("invented.diff".to_owned()),
            Some(on.to_string()),
        )
    );
    let each = of("a master change between two snapshots");
    assert_eq!(each.len(), DIFF_NAMED);
    assert_eq!(
        ["change", "exchange", "token", "symbol", "new", "day"]
            .iter()
            .map(|key| text(each[DIFF_NAMED - 1], key))
            .collect::<Vec<_>>(),
        [
            Some("listing".to_owned()),
            Some("NFO".to_owned()),
            Some("9100000".to_owned()),
            Some("ZQIDX0".to_owned()),
            Some("ZQIDX0".to_owned()),
            Some(on.to_string()),
        ],
        "the first change is named first"
    );
    let refused = of("no master snapshot was written");
    assert_eq!(refused.len(), 1);
    assert_eq!(
        (
            text(refused[0], "kind"),
            text(refused[0], "outcome"),
            text(refused[0], "detail"),
        ),
        (
            Some("nse_total_market".to_owned()),
            Some("refused".to_owned()),
            Some("an invented refusal".to_owned()),
        )
    );
}

/// A diff record of values in four-byte letters: each fits its field by
/// bytes, and padded by letters the record runs past its length, so it is
/// refused by name and never cut (an input the Kite dump can carry: a name
/// or symbol that is not ASCII). One such letter in every field is exactly
/// the record length with no room for its line break, refused too; one
/// ASCII byte in every field is a whole record.
#[test]
fn a_record_of_wide_letters_past_the_record_length_is_refused() {
    let wide = |letters: usize| "\u{1D538}".repeat(letters);
    let fields = [wide(3), wide(2), wide(5), wide(12), wide(10), wide(10)];
    let refused = Err("a record is wider than the record length".to_owned());
    assert_eq!(fixed(fields.each_ref().map(String::as_str)), refused);
    let one = wide(1);
    assert_eq!(fixed([one.as_str(); 6]), refused);
    let line = fixed(["a"; 6]).unwrap();
    assert_eq!(line.len(), RECORD_LEN);
    assert!(line.ends_with('\n'), "{line:?}");
}

/// A link at the diff's name that leads nowhere is read as no diff before the
/// lock and found as an entry under it: the diff is refused by name and the
/// link is left exactly as it was, never written over.
#[test]
fn a_dangling_link_at_the_diff_name_is_never_written_over() {
    let root = scratch("cov-diff-link");
    let (first, second) = (day(2026, 3, 2), day(2026, 3, 9));
    let row = fut(9_000_011, "ZQIDX26MARFUT", "2026-03-31", 75);
    snapshot(
        &root,
        Kind::Kite,
        first,
        URL,
        dump(std::slice::from_ref(&row)).as_bytes(),
    )
    .unwrap();
    let link = root.join(diff_name(stamp(second, 1)));
    std::os::unix::fs::symlink(root.join("nowhere"), &link).unwrap();
    snapshot(
        &root,
        Kind::Kite,
        second,
        URL,
        dump(std::slice::from_ref(&row)).as_bytes(),
    )
    .unwrap();
    let why = diff_for(&root, stamp(second, 1)).unwrap_err();
    assert!(
        why.contains("is a name with nothing readable behind it"),
        "{why}"
    );
    assert_eq!(
        std::fs::read_link(link).unwrap(),
        root.join("nowhere"),
        "left as it was"
    );
}

/// An index the day table holds past the book's stamps answers that no
/// snapshot governs the day, never a panic (a book whose table and stamps
/// disagree, made by hand: no listing makes one).
#[test]
fn a_table_index_past_the_stamps_answers_no_snapshot() {
    let empty = |on: Day| Snapshot::parse(stamp(on, 1), &format!("{HEAD_TEXT}\n")).unwrap();
    let mut book = Book::of(vec![empty(day(2026, 1, 1))]).unwrap();
    for at in &mut book.table {
        *at = 7;
    }
    assert!(matches!(
        book.token_on(day(2026, 1, 1), "NFO", 9_000_001),
        TokenOn::NoSnapshot
    ));
}

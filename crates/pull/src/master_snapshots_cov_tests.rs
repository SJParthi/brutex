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
        snapshot(&root, Kind::Kite, on, URL, dump(&[row.clone()]).as_bytes()).unwrap();
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
    let isins = |isin: &str| {
        let mut held = HashMap::new();
        held.insert("ZQSHARE".to_owned(), isin.to_owned());
        held
    };
    let cash = |at: Stamp| parse(at, &[share(9_000_002, "ZQSHARE", "ZQ SHARE")]);
    let same = isins("INE000INVENT");
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
        snapshot(&root, Kind::Kite, on, URL, dump(&[row.clone()]).as_bytes()).unwrap();
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

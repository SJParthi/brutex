//! R4-28 (DLY-09): dated master snapshots, the day's diff and `token_on`.
//!
//! Every row here is invented: `ZQ`-prefixed names, tokens in the nine
//! millions and ISINs no exchange issued. The header is Kite's published
//! column list, which is a schema and not a data row.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;

use super::*;

const URL: &str = "https://api.kite.trade/instruments";
const HEAD: &str = "instrument_token,exchange_token,tradingsymbol,name,last_price,expiry,strike,tick_size,lot_size,instrument_type,segment,exchange";
const NSE_HEAD: &str = "Company Name,Industry,Symbol,Series,ISIN Code";

/// The `today` a staleness check is asked under: a fixed day, so no test here
/// reads the wall clock.
const TODAY: Day = match Day::new(2026, 10, 23) {
    Ok(today) => today,
    Err(_) => panic!("a real day"),
};

/// A new, empty folder of this test's own. Left for the main session's
/// clean-up: nothing in this suite deletes a file.
fn scratch(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir =
        std::env::temp_dir().join(format!("brutex-r428-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).expect("a real day")
}

fn stamp(day: Day, ordinal: u32) -> Stamp {
    Stamp { day, ordinal }
}

/// One invented dump row.
#[allow(clippy::too_many_arguments, reason = "one argument per Kite column")]
fn row(
    token: u64,
    symbol: &str,
    name: &str,
    expiry: &str,
    strike: &str,
    lot: u32,
    kind: &str,
    segment: &str,
    exchange: &str,
) -> String {
    format!(
        "{token},{},{symbol},\"{name}\",0,{expiry},{strike},0.05,{lot},{kind},{segment},{exchange}",
        token / 256
    )
}

fn fut(token: u64, symbol: &str, expiry: &str, lot: u32) -> String {
    row(
        token, symbol, "ZQIDX", expiry, "0", lot, "FUT", "NFO-FUT", "NFO",
    )
}

fn share(token: u64, symbol: &str, name: &str) -> String {
    row(token, symbol, name, "", "0", 1, "EQ", "NSE", "NSE")
}

fn dump(rows: &[String]) -> String {
    let mut out = String::from(HEAD);
    for one in rows {
        out.push('\n');
        out.push_str(one);
    }
    out.push('\n');
    out
}

fn nse_file(rows: &[(&str, &str)]) -> String {
    let mut out = String::from(NSE_HEAD);
    for (symbol, isin) in rows {
        let _infallible = write!(out, "\nZq {symbol} Ltd.,Testing,{symbol},EQ,{isin}");
    }
    out.push('\n');
    out
}

fn kite_source() -> &'static Source {
    crate::masters::SOURCES
        .iter()
        .find(|source| source.vendor == Some(Vendor::Zerodha))
        .expect("the Zerodha master is a source")
}

fn nse_source() -> &'static Source {
    crate::masters::SOURCES
        .iter()
        .find(|source| source.file == NSE_TOTAL_MARKET_FILE)
        .expect("the NSE total market file is a source")
}

fn written() -> Landed {
    Landed::Written {
        bytes: 1,
        changed: true,
    }
}

/// Two days on which token 9,000,001 names two different futures.
fn two_days(root: &Path) -> (Day, Day) {
    let (d1, d2) = (day(2026, 1, 5), day(2026, 2, 2));
    let first = dump(&[
        fut(9_000_001, "ZQIDX26JANFUT", "2026-01-29", 75),
        fut(9_000_002, "ZQIDX26MARFUT", "2026-03-26", 75),
    ]);
    let second = dump(&[
        fut(9_000_001, "ZQIDX26APRFUT", "2026-04-28", 75),
        fut(9_000_002, "ZQIDX26MARFUT", "2026-03-26", 75),
    ]);
    assert_eq!(
        snapshot(root, Kind::Kite, d1, URL, first.as_bytes()),
        Ok(Wrote::New(stamp(d1, 1)))
    );
    assert_eq!(
        snapshot(root, Kind::Kite, d2, URL, second.as_bytes()),
        Ok(Wrote::New(stamp(d2, 1)))
    );
    (d1, d2)
}

#[test]
fn one_token_two_dates_resolves_to_two_contracts() {
    let root = scratch("two-dates");
    let (d1, d2) = two_days(&root);
    let book = Book::open(&root).expect("a book");
    let TokenOn::Resolved {
        snapshot: s1,
        listed: on_first,
    } = book.token_on(d1, "NFO", 9_000_001)
    else {
        panic!("the first date resolves");
    };
    let TokenOn::Resolved {
        snapshot: s2,
        listed: on_second,
    } = book.token_on(d2, "NFO", 9_000_001)
    else {
        panic!("the second date resolves");
    };
    assert_eq!((s1, s2), (stamp(d1, 1), stamp(d2, 1)));
    assert_eq!(on_first.tradingsymbol(), "ZQIDX26JANFUT");
    assert_eq!(on_first.expiry(), Some(day(2026, 1, 29)));
    assert_eq!(on_second.tradingsymbol(), "ZQIDX26APRFUT");
    assert_eq!(on_second.expiry(), Some(day(2026, 4, 28)));
    assert!(
        !on_first.same_contract(&on_second),
        "one token, two contracts"
    );
}

#[test]
fn a_token_reused_between_two_sparse_snapshots_is_ambiguous_not_the_older_contract() {
    let root = scratch("sparse");
    let (d1, d2) = two_days(&root);
    let book = Book::open(&root).expect("a book");
    let between = day(2026, 1, 20);
    match book.token_on(between, "NFO", 9_000_001) {
        TokenOn::Ambiguous { older, newer } => {
            assert_eq!((older, newer), (stamp(d1, 1), stamp(d2, 1)));
        }
        other => panic!("a reused token between snapshots is ambiguous, got {other:?}"),
    }
    assert_eq!(
        book.token_on(between, "NFO", 9_000_001).word(),
        "token_ambiguous"
    );
    // A token both snapshots map to one contract is believed between them.
    let TokenOn::Resolved { snapshot, listed } = book.token_on(between, "NFO", 9_000_002) else {
        panic!("an agreeing token resolves");
    };
    assert_eq!(snapshot, stamp(d1, 1));
    assert_eq!(listed.tradingsymbol(), "ZQIDX26MARFUT");
    // After the newest snapshot there is no later one to disagree.
    let TokenOn::Resolved { listed, .. } = book.token_on(day(2026, 2, 20), "NFO", 9_000_001) else {
        panic!("after the newest snapshot the newest answers");
    };
    assert_eq!(listed.tradingsymbol(), "ZQIDX26APRFUT");
}

#[test]
fn a_date_before_the_first_snapshot_resolves_nothing() {
    let root = scratch("before-first");
    let (d1, _) = two_days(&root);
    let book = Book::open(&root).expect("a book");
    for before in [day(2026, 1, 4), day(2018, 1, 1), day(1970, 1, 1)] {
        match book.token_on(before, "NFO", 9_000_001) {
            TokenOn::BeforeFirst { first } => assert_eq!(first, stamp(d1, 1)),
            other => panic!("before the first snapshot nothing resolves, got {other:?}"),
        }
    }
    let empty = Book::open(&scratch("no-snapshot")).expect("an empty book");
    assert!(matches!(
        empty.token_on(d1, "NFO", 9_000_001),
        TokenOn::NoSnapshot
    ));
    assert_eq!(empty.newest(), None);
}

#[test]
fn a_snapshot_is_never_rewritten() {
    let root = scratch("never-rewritten");
    let d1 = day(2026, 3, 2);
    let first = dump(&[fut(9_000_011, "ZQIDX26MARFUT", "2026-03-26", 75)]);
    let other = dump(&[fut(9_000_011, "ZQIDX26MARFUT", "2026-03-26", 65)]);
    assert_eq!(
        snapshot(&root, Kind::Kite, d1, URL, first.as_bytes()),
        Ok(Wrote::New(stamp(d1, 1)))
    );
    let body = root.join(file_name(Kind::Kite, stamp(d1, 1)));
    let receipt = root.join(receipt_name(Kind::Kite, stamp(d1, 1)));
    let (body_bytes, receipt_bytes) = (fs::read(&body).unwrap(), fs::read(&receipt).unwrap());
    let modified = fs::metadata(&body).unwrap().modified().unwrap();

    // THE SAME BYTES AGAIN: already held, nothing touched.
    assert_eq!(
        snapshot(&root, Kind::Kite, d1, URL, first.as_bytes()),
        Ok(Wrote::AlreadyHeld(stamp(d1, 1)))
    );
    // DIFFERENT BYTES THE SAME DAY: a second file, the first untouched.
    assert_eq!(
        snapshot(&root, Kind::Kite, d1, URL, other.as_bytes()),
        Ok(Wrote::New(stamp(d1, 2)))
    );
    assert_eq!(fs::read(&body).unwrap(), body_bytes);
    assert_eq!(fs::read(&receipt).unwrap(), receipt_bytes);
    assert_eq!(fs::metadata(&body).unwrap().modified().unwrap(), modified);
    // The first body is never the day's answer again: `_2` is the newest.
    assert_eq!(
        snapshot(&root, Kind::Kite, d1, URL, first.as_bytes()),
        Ok(Wrote::New(stamp(d1, 3))),
        "the old bytes back after a change are a new state, not the old file"
    );
    // AND THE WRITER ITSELF REFUSES AN EXISTING NAME.
    let refused = write_new(&body, b"over").expect_err("O_EXCL refuses an existing file");
    assert!(
        refused.contains("no existing file was touched"),
        "{refused}"
    );
    assert_eq!(fs::read(&body).unwrap(), body_bytes);
    assert_eq!(read_body(&root, Kind::Kite, stamp(d1, 1)).unwrap(), first);
}

#[test]
fn a_snapshot_cut_before_its_receipt_is_never_read_and_the_next_refresh_is_used() {
    let root = scratch("cut");
    let d1 = day(2026, 3, 4);
    let cut = dump(&[fut(9_000_021, "ZQIDX26MARFUT", "2026-03-26", 75)]);
    let next = dump(&[fut(9_000_021, "ZQIDX26APRFUT", "2026-04-28", 75)]);
    // A KILL BETWEEN BODY AND RECEIPT: the body alone is on disk.
    fs::write(
        root.join(file_name(Kind::Kite, stamp(d1, 1))),
        compress(cut.as_bytes()).unwrap(),
    )
    .unwrap();
    let catalog = Catalog::read(&root).unwrap();
    assert!(catalog.receipted(Kind::Kite).is_empty());
    assert_eq!(catalog.incomplete().len(), 1);
    assert!(
        catalog.incomplete()[0].to_string().contains("never read"),
        "{:?}",
        catalog.incomplete()
    );
    assert_eq!(
        catalog.incomplete()[0].file,
        file_name(Kind::Kite, stamp(d1, 1))
    );
    assert_eq!(catalog.incomplete()[0].why, CUT_BODY);
    assert!(matches!(
        Book::open(&root).unwrap().token_on(d1, "NFO", 9_000_021),
        TokenOn::NoSnapshot
    ));
    assert_eq!(newest_snapshot_day(&root, TODAY), Ok(None));

    // THE NEXT REFRESH TAKES THE NEXT ORDINAL AND IS THE DAY'S SNAPSHOT.
    assert_eq!(
        snapshot(&root, Kind::Kite, d1, URL, next.as_bytes()),
        Ok(Wrote::New(stamp(d1, 2)))
    );
    let book = Book::open(&root).unwrap();
    assert_eq!(book.stamps(), &[stamp(d1, 2)]);
    let TokenOn::Resolved { listed, .. } = book.token_on(d1, "NFO", 9_000_021) else {
        panic!("the receipted refresh answers");
    };
    assert_eq!(listed.tradingsymbol(), "ZQIDX26APRFUT");
    // The diff never compares against the incomplete body.
    assert_eq!(diff_for(&root, stamp(d1, 2)), Ok(None));
    assert_eq!(newest_snapshot_day(&root, TODAY), Ok(Some(d1)));
}

/// R4-28 repair 3 (verify r3 blocking): the residue a kill leaves is named by
/// the refresh that steps past it. The verifier's case: the refreshed day
/// holds a body with no receipt, and the day before holds a body with a
/// partial receipt. The `/masters/refresh` outcome names all three entries.
#[test]
fn a_refresh_after_a_cut_body_names_every_incomplete_entry() {
    let dir = scratch("cut-named");
    let root = dir.join(SNAPSHOT_DIR);
    fs::create_dir_all(&root).unwrap();
    let (d0, d1) = (day(2026, 3, 3), day(2026, 3, 4));
    let cut = dump(&[fut(9_000_031, "ZQIDX26MARFUT", "2026-03-26", 75)]);
    let cut_today = file_name(Kind::Kite, stamp(d1, 1));
    let cut_before = file_name(Kind::Kite, stamp(d0, 1));
    let partial = format!("{}{PARTIAL}", receipt_name(Kind::Kite, stamp(d0, 1)));
    for name in [&cut_today, &cut_before] {
        fs::write(root.join(name), compress(cut.as_bytes()).unwrap()).unwrap();
    }
    fs::write(root.join(&partial), b"brutex-master-snapshot-v1\nkind=instr").unwrap();
    // AN NSE ENTRY BELONGS TO THE NSE HOOK, never to the Kite one.
    let nse_cut = file_name(Kind::NseTotalMarket, stamp(d1, 1));
    fs::write(root.join(&nse_cut), b"cut").unwrap();
    let next = dump(&[fut(9_000_031, "ZQIDX26APRFUT", "2026-04-28", 75)]);
    let outcome = after_land(&dir, kite_source(), &next, &written(), Ok(d1)).unwrap();
    assert_eq!(outcome.wrote, Ok(Wrote::New(stamp(d1, 2))));
    let json = outcome.json();
    for name in [&cut_today, &cut_before, &partial] {
        assert!(
            json.contains(&format!("\"{name}\"")),
            "the refresh outcome names {name}: {json}"
        );
    }
    // STRUCTURED, OLDEST FIRST, THIS KIND'S ONLY; THE JSON NEWEST FIRST.
    let named: Vec<&str> = outcome
        .incomplete
        .as_ref()
        .unwrap()
        .iter()
        .map(|one| one.file.as_str())
        .collect();
    assert_eq!(
        named,
        [cut_before.as_str(), partial.as_str(), cut_today.as_str()]
    );
    assert!(!json.contains(&nse_cut), "{json}");
    assert!(
        json.contains(r#""incomplete":{"held":3,"named":[{"file":"instruments_20260304.csv.gz","stamp":"2026-03-04","why":"a body with no receipt (a write cut before its commit marker)"}"#)
            && json.contains(r#""unnamed":0,"refusal":null}"#),
        "{json}"
    );
    // WHY THE DAY'S SNAPSHOT IS `_2`, in the event's own words.
    let why = stepped_past(&outcome);
    assert!(
        why.starts_with("ordinal 2: 1 entries") && why.contains(&cut_today),
        "{why}"
    );
    assert!(!why.contains(&cut_before), "only the day's own: {why}");
    // A REFRESH THAT WRITES NOTHING STILL NAMES THEM, from one listing.
    let refused = after_land(
        &dir,
        kite_source(),
        &next,
        &Landed::Refused("an invented refusal".to_owned()),
        Ok(d1),
    )
    .unwrap();
    assert!(refused.wrote.is_err());
    assert_eq!(refused.incomplete.as_ref().map(Vec::len), Ok(3));
    assert!(refused.json().contains(&format!("\"{cut_today}\"")));
    assert_eq!(stepped_past(&refused), "");
    // THE NSE HOOK NAMES ITS OWN.
    let nse = after_land(
        &dir,
        nse_source(),
        &nse_file(&[("ZQAAA", "INE9ZQ000001")]),
        &written(),
        Ok(d1),
    )
    .unwrap();
    assert_eq!(nse.wrote, Ok(Wrote::New(stamp(d1, 2))));
    assert!(nse.json().contains(&format!("\"{nse_cut}\"")));
    assert!(!nse.json().contains(&cut_today));
}

/// R4-28 repair 3: past [`INCOMPLETE_NAMED`] entries the newest are named and
/// the rest counted, never dropped unseen.
#[test]
fn incomplete_entries_past_the_bound_are_counted_not_dropped() {
    let dir = scratch("cut-many");
    let root = dir.join(SNAPSHOT_DIR);
    fs::create_dir_all(&root).unwrap();
    let first = day(2026, 1, 1).days_from_epoch();
    let held = INCOMPLETE_NAMED + 2;
    let names: Vec<String> = (0..held)
        .map(|n| {
            let on = Day::from_days(first + u32::try_from(n).unwrap()).unwrap();
            let name = file_name(Kind::Kite, stamp(on, 1));
            fs::write(root.join(&name), b"cut").unwrap();
            name
        })
        .collect();
    let on = day(2026, 6, 1);
    let body = dump(&[fut(9_000_041, "ZQIDX26JUNFUT", "2026-06-30", 75)]);
    let outcome = after_land(&dir, kite_source(), &body, &written(), Ok(on)).unwrap();
    assert_eq!(outcome.wrote, Ok(Wrote::New(stamp(on, 1))));
    assert_eq!(outcome.incomplete.as_ref().map(Vec::len), Ok(held));
    let json = outcome.json();
    assert!(
        json.contains(&format!(r#""held":{held},"#)) && json.contains(r#""unnamed":2,"#),
        "{json}"
    );
    for (n, name) in names.iter().enumerate() {
        assert_eq!(
            json.contains(&format!("\"{name}\"")),
            n >= 2,
            "the newest {INCOMPLETE_NAMED} are named, the oldest two counted: {name}"
        );
    }
}

/// The two days of the diff test, with every injected change.
fn diff_days(root: &Path) -> (Stamp, Stamp) {
    let (a, b) = (day(2026, 3, 2), day(2026, 3, 3));
    let first = dump(&[
        fut(9_100_001, "ZQIDX26MARFUT", "2026-03-26", 75),
        fut(9_100_002, "ZQIDX26APRFUT", "2026-04-30", 75),
        row(
            9_100_003,
            "ZQIDX26MAR1000CE",
            "ZQIDX",
            "2026-03-26",
            "1000",
            75,
            "CE",
            "NFO-OPT",
            "NFO",
        ),
        fut(9_100_005, "ZQIDX26MAYFUT", "2026-05-28", 75),
        share(9_200_001, "ZQALPHA", "ZQ ALPHA"),
        share(9_200_002, "ZQBETA", "ZQ BETA"),
        share(9_200_003, "ZQGAMMA", "ZQ GAMMA"),
        share(9_200_009, "ZQOMEGA", "ZQ OMEGA"),
        share(9_200_010, "ZQKAPPA", "ZQ KAPPA"),
    ]);
    let second = dump(&[
        fut(9_100_001, "ZQIDX26MARFUT", "2026-03-26", 65),
        fut(9_100_002, "ZQIDX26APRFUT", "2026-04-29", 75),
        fut(9_100_006, "ZQIDX26MAYFUT", "2026-05-28", 75),
        row(
            9_100_007,
            "ZQIDX26MAR900PE",
            "ZQIDX",
            "2026-03-26",
            "900.0",
            75,
            "PE",
            "NFO-OPT",
            "NFO",
        ),
        share(9_200_001, "ZQALPHA", "ZQ ALPHA"),
        share(9_200_002, "ZQBETANEW", "ZQ BETA"),
        share(9_200_004, "ZQDELTA", "ZQ DELTA"),
        share(9_200_009, "ZQOMEGA", "ZQ OMEGA"),
        share(9_200_010, "ZQKAPPA", "ZQ KAPPA"),
        share(9_200_011, "ZQNEW", "ZQ NEW"),
    ]);
    let nse_a = nse_file(&[
        ("ZQALPHA", "INE0ZQ000011"),
        ("ZQBETA", "INE0ZQ000022"),
        ("ZQGAMMA", "INE0ZQ000033"),
        ("ZQKAPPA", "INE0ZQ000044"),
    ]);
    let nse_b = nse_file(&[
        ("ZQALPHA", "INE0ZQ000012"),
        ("ZQBETANEW", "INE0ZQ000022"),
        ("ZQDELTA", "INE0ZQ000033"),
    ]);
    let nse_url = nse_source().url;
    snapshot(root, Kind::NseTotalMarket, a, nse_url, nse_a.as_bytes()).unwrap();
    snapshot(root, Kind::Kite, a, URL, first.as_bytes()).unwrap();
    snapshot(root, Kind::NseTotalMarket, b, nse_url, nse_b.as_bytes()).unwrap();
    snapshot(root, Kind::Kite, b, URL, second.as_bytes()).unwrap();
    (stamp(a, 1), stamp(b, 1))
}

fn expected_items() -> Vec<Item> {
    let one = |change, exchange: &str, token, key: &str, old: &str, new: &str| Item {
        change,
        exchange: exchange.to_owned(),
        token,
        key: key.to_owned(),
        old: old.to_owned(),
        new: new.to_owned(),
    };
    let mut items = vec![
        one(
            Change::Listing,
            "NFO",
            9_100_007,
            "ZQIDX26MAR900PE",
            "",
            "ZQIDX26MAR900PE",
        ),
        one(
            Change::Delisting,
            "NFO",
            9_100_003,
            "ZQIDX26MAR1000CE",
            "ZQIDX26MAR1000CE",
            "",
        ),
        one(
            Change::Symbol,
            "NSE",
            9_200_002,
            "ZQBETANEW",
            "ZQBETA",
            "ZQBETANEW",
        ),
        one(
            Change::Symbol,
            "NSE",
            9_200_004,
            "ZQDELTA",
            "ZQGAMMA",
            "ZQDELTA",
        ),
        one(
            Change::Isin,
            "NSE",
            9_200_001,
            "ZQALPHA",
            "INE0ZQ000011",
            "INE0ZQ000012",
        ),
        one(
            Change::LotSize,
            "NFO",
            9_100_001,
            "ZQIDX26MARFUT",
            "75",
            "65",
        ),
        one(
            Change::Expiry,
            "NFO",
            9_100_002,
            "ZQIDX26APRFUT",
            "2026-04-30",
            "2026-04-29",
        ),
        one(
            Change::Token,
            "NFO",
            9_100_006,
            "ZQIDX26MAYFUT",
            "9100005",
            "9100006",
        ),
        one(Change::Listing, "NSE", 9_200_011, "ZQNEW", "", "ZQNEW"),
        // NAMED THE DAY EACH BECOMES UNKNOWN: KAPPA left the NSE file, NEW was
        // never in it. OMEGA is unknown on both days: counted, not named again.
        one(Change::IsinUnknown, "NSE", 9_200_010, "ZQKAPPA", "", ""),
        one(Change::IsinUnknown, "NSE", 9_200_011, "ZQNEW", "", ""),
    ];
    items.sort_unstable();
    items
}

#[test]
fn the_diff_of_two_days_lists_exactly_the_injected_changes() {
    let root = scratch("diff");
    let (a, b) = diff_days(&root);
    let summary = diff_for(&root, b)
        .unwrap()
        .expect("a previous snapshot exists");
    assert_eq!(summary.previous, a);
    assert_eq!(summary.current, b);
    assert_eq!(summary.counts, [2, 1, 2, 1, 1, 1, 1, 2]);
    assert_eq!(
        summary.isin_unknown_total, 3,
        "OMEGA, KAPPA and NEW have no ISIN on day two"
    );
    assert_eq!(
        summary.counts_line(),
        "listing=2 delisting=1 symbol=2 isin=1 lot_size=1 expiry=1 token=1 isin_unknown=2 isin_unknown_total=3"
    );
    let expected = expected_items();
    let named: Vec<Item> = expected
        .iter()
        .filter(|one| one.change.named())
        .cloned()
        .collect();
    assert_eq!(
        summary.named, named,
        "every symbol, ISIN, lot, expiry and token change is named"
    );
    // THE FILE HOLDS EXACTLY THE INJECTED CHANGES, in fixed records.
    let text = fs::read_to_string(root.join(diff_name(b))).unwrap();
    assert_eq!(text.len(), RECORD_LEN * (expected.len() + 1));
    assert!(text.lines().all(|line| line.len() == RECORD_LEN - 1));
    let (head, items) = parse_diff(&text).unwrap();
    assert_eq!(
        (head.previous.as_str(), head.current.as_str()),
        ("2026-03-02", "2026-03-03")
    );
    assert_eq!((head.records, head.isin_unknown_total), (expected.len(), 3));
    assert_eq!(items, expected);
    // READ BACK, NOT RECOMPUTED, on the next call; and never rewritten.
    let modified = fs::metadata(root.join(diff_name(b)))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(diff_for(&root, b).unwrap().unwrap(), summary);
    assert_eq!(
        fs::metadata(root.join(diff_name(b)))
            .unwrap()
            .modified()
            .unwrap(),
        modified
    );
    // The first day has nothing to compare against.
    assert_eq!(diff_for(&root, a), Ok(None));
}

#[test]
fn an_isin_change_between_masters_is_named() {
    let root = scratch("isin");
    let (_, b) = diff_days(&root);
    let summary = diff_for(&root, b).unwrap().unwrap();
    let isin: Vec<&Item> = summary
        .named
        .iter()
        .filter(|one| one.change == Change::Isin)
        .collect();
    assert_eq!(isin.len(), 1);
    assert_eq!(
        (
            isin[0].key.as_str(),
            isin[0].old.as_str(),
            isin[0].new.as_str()
        ),
        ("ZQALPHA", "INE0ZQ000011", "INE0ZQ000012")
    );
}

#[test]
fn a_day_without_its_own_nse_file_counts_every_share_isin_unknown_and_infers_nothing() {
    let (a, b) = (day(2026, 3, 9), day(2026, 3, 10));
    let root = scratch("no-nse");
    let rows = [
        share(9_300_001, "ZQONE", "ZQ ONE"),
        share(9_300_002, "ZQTWO", "ZQ TWO"),
    ];
    snapshot(&root, Kind::Kite, a, URL, dump(&rows).as_bytes()).unwrap();
    // An NSE file of ANOTHER day must not be borrowed.
    snapshot(
        &root,
        Kind::NseTotalMarket,
        a,
        nse_source().url,
        nse_file(&[("ZQONE", "INE0ZQ000099")]).as_bytes(),
    )
    .unwrap();
    snapshot(&root, Kind::Kite, b, URL, dump(&rows).as_bytes()).unwrap();
    let summary = diff_for(&root, stamp(b, 1)).unwrap().unwrap();
    // ONE NAMED (known on day one, unknown on day two), BOTH COUNTED.
    assert_eq!(summary.counts, [0, 0, 0, 0, 0, 0, 0, 1]);
    assert_eq!(summary.isin_unknown_total, 2);
    assert!(summary.named.is_empty());
}

#[test]
fn the_landing_hook_snapshots_both_masters_and_reports_the_diff() {
    let dir = scratch("hook");
    let root = dir.join(SNAPSHOT_DIR);
    let (a, b) = (day(2026, 4, 6), day(2026, 4, 7));
    let nse = nse_file(&[("ZQALPHA", "INE0ZQ000011")]);
    let first = dump(&[
        share(9_400_001, "ZQALPHA", "ZQ ALPHA"),
        fut(9_400_002, "ZQIDX26APRFUT", "2026-04-28", 75),
    ]);
    let second = dump(&[
        share(9_400_001, "ZQALPHA", "ZQ ALPHA"),
        fut(9_400_002, "ZQIDX26APRFUT", "2026-04-28", 30),
    ]);

    let n = after_land(&dir, nse_source(), &nse, &written(), Ok(a))
        .expect("the NSE file is snapshotted");
    assert_eq!(n.kind, Kind::NseTotalMarket);
    assert_eq!(n.wrote, Ok(Wrote::New(stamp(a, 1))));
    assert_eq!(n.diff, None);
    let k1 = after_land(&dir, kite_source(), &first, &written(), Ok(a))
        .expect("the Kite dump is snapshotted");
    assert_eq!(
        k1.diff,
        Some(Ok(None)),
        "the first Kite snapshot has nothing to compare"
    );
    assert!(
        k1.json()
            .contains(r#""note":"the first snapshot: nothing to compare""#),
        "{}",
        k1.json()
    );
    let k2 = after_land(&dir, kite_source(), &second, &written(), Ok(b)).unwrap();
    let json = k2.json();
    assert!(json.contains(r#""wrote":"new""#), "{json}");
    assert!(json.contains(r#""lot_size":1"#), "{json}");
    assert!(json.contains(r#""change":"lot_size","exchange":"NFO","token":9400002,"symbol":"ZQIDX26APRFUT","old":"75","new":"30""#), "{json}");
    // The isin of the second day is unknown: no NSE file was landed that day.
    assert!(json.contains(r#""isin_unknown":1"#), "{json}");
    // A source that is not snapshotted is left alone.
    let groww = crate::masters::SOURCES
        .iter()
        .find(|source| source.vendor == Some(Vendor::Groww))
        .unwrap();
    assert_eq!(after_land(&dir, groww, "x", &written(), Ok(a)), None);
    assert_eq!(
        Catalog::read(&root).unwrap().receipted(Kind::Kite),
        &[stamp(a, 1), stamp(b, 1)]
    );
}

#[test]
fn a_refused_landing_writes_no_snapshot_and_the_newest_stands() {
    let dir = scratch("refused");
    let root = dir.join(SNAPSHOT_DIR);
    let a = day(2026, 4, 6);
    let body = dump(&[fut(9_500_001, "ZQIDX26APRFUT", "2026-04-28", 75)]);
    after_land(&dir, kite_source(), &body, &written(), Ok(a)).unwrap();
    // A DOWNLOAD THAT FAILED MIDWAY: the landing refused, so no snapshot.
    let cut = &body[..body.len() / 2];
    let refused = after_land(
        &dir,
        kite_source(),
        cut,
        &Landed::Refused("a truncated body".to_owned()),
        Ok(day(2026, 4, 7)),
    )
    .unwrap();
    let why = refused
        .wrote
        .clone()
        .expect_err("no snapshot for a refused landing");
    assert!(why.contains("the newest one stands"), "{why}");
    assert!(refused.json().contains(r#""wrote":"refused""#));
    assert_eq!(
        Catalog::read(&root).unwrap().receipted(Kind::Kite),
        &[stamp(a, 1)]
    );
    assert_eq!(newest_snapshot_day(&root, TODAY), Ok(Some(a)));
}

/// R4-28 repair 3 (verify r3 A13): a clock that jumped ahead dates a snapshot
/// in the future, and that snapshot stays the newest. `master_stale` must not
/// read it as fresh, so a newest snapshot after the caller's today is refused
/// by name. A snapshot dated today is not in the future.
#[test]
fn a_future_dated_newest_snapshot_is_refused_never_believed_fresh() {
    let root = scratch("future-dated");
    let (_, d2) = two_days(&root);
    assert_eq!(newest_snapshot_day(&root, d2), Ok(Some(d2)));
    let why = newest_snapshot_day(&root, day(2026, 2, 1)).unwrap_err();
    assert!(
        why.contains("after today 2026-02-01")
            && why.contains(&file_name(Kind::Kite, stamp(d2, 1)))
            && why.contains("refused rather than believed fresh"),
        "{why}"
    );
}

#[test]
fn newest_snapshot_day_is_read_from_the_receipt_and_a_torn_one_is_refused_not_replaced() {
    let root = scratch("newest");
    let (d1, d2) = two_days(&root);
    assert_eq!(newest_snapshot_day(&root, TODAY), Ok(Some(d2)));
    // TEAR THE NEWEST RECEIPT: refused by name, never answered with d1.
    let receipt = root.join(receipt_name(Kind::Kite, stamp(d2, 1)));
    let text = fs::read_to_string(&receipt).unwrap();
    fs::write(&receipt, text.replace("ordinal=1", "ordinal=7")).unwrap();
    let why = newest_snapshot_day(&root, TODAY).expect_err("a torn receipt is refused");
    assert!(why.contains("refused, never read"), "{why}");
    let book = Book::open(&root).unwrap();
    match book.token_on(d2, "NFO", 9_000_001) {
        TokenOn::Unreadable { snapshot, why } => {
            assert_eq!(snapshot, stamp(d2, 1));
            assert!(why.contains("refused"), "{why}");
        }
        other => panic!("a torn snapshot is unreadable, got {other:?}"),
    }
    // The older snapshot is still read for its own day.
    assert!(matches!(
        book.token_on(d1, "NFO", 9_000_001),
        TokenOn::Resolved { .. }
    ));
}

#[test]
fn a_body_that_does_not_match_its_receipt_is_refused() {
    let root = scratch("mismatch");
    let d1 = day(2026, 5, 4);
    let body = dump(&[fut(9_600_001, "ZQIDX26MAYFUT", "2026-05-26", 75)]);
    snapshot(&root, Kind::Kite, d1, URL, body.as_bytes()).unwrap();
    let path = root.join(file_name(Kind::Kite, stamp(d1, 1)));
    let mut bytes = fs::read(&path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    fs::write(&path, &bytes).unwrap();
    let why = read_body(&root, Kind::Kite, stamp(d1, 1)).expect_err("a changed body is refused");
    assert!(why.contains("does not match its receipt"), "{why}");
}

#[test]
fn every_refusal_of_a_dump_is_named() {
    let s = stamp(day(2026, 5, 4), 1);
    assert_eq!(
        snapshot(&scratch("empty"), Kind::Kite, s.day, URL, b"").unwrap_err(),
        "an empty body is not a master; no snapshot was written"
    );
    let missing = Snapshot::parse(s, "instrument_token,tradingsymbol\n1,ZQ\n").unwrap_err();
    assert!(missing.contains("no column \"name\""), "{missing}");
    let garbage = Snapshot::parse(
        s,
        &dump(&["abc,1,ZQ,\"ZQ\",0,,0,0.05,1,EQ,NSE,NSE".to_owned()]),
    )
    .unwrap_err();
    assert!(
        garbage.contains("line 2") && garbage.contains("instrument_token"),
        "{garbage}"
    );
    let bad_expiry = Snapshot::parse(
        s,
        &dump(&[fut(9_700_001, "ZQIDX26MAYFUT", "26-05-2026", 75)]),
    )
    .unwrap_err();
    assert!(bad_expiry.contains("expiry"), "{bad_expiry}");
    let open_quote = Snapshot::parse(
        s,
        &format!("{HEAD}\n9700002,1,ZQ,\"ZQ,0,,0,0.05,1,EQ,NSE,NSE\n"),
    )
    .unwrap_err();
    assert!(open_quote.contains("not closed"), "{open_quote}");
    assert!(Snapshot::parse(s, "").unwrap_err().contains("empty"));
}

#[test]
fn one_row_a_quoted_comma_and_a_respelled_strike_all_read() {
    let s = stamp(day(2026, 5, 4), 1);
    let one = Snapshot::parse(
        s,
        &dump(&[row(
            9_800_001,
            "ZQIDX26MAY1000CE",
            "ZQ, INDEX",
            "2026-05-26",
            "1000.0",
            75,
            "CE",
            "NFO-OPT",
            "NFO",
        )]),
    )
    .unwrap();
    assert_eq!(one.len(), 1);
    let Lookup::Listed(listed) = one.lookup("NFO", 9_800_001) else {
        panic!("one row is found");
    };
    assert_eq!(listed.name(), "ZQ, INDEX");
    assert_eq!(listed.strike(), "1000");
    let other = Snapshot::parse(
        s,
        &dump(&[row(
            9_800_001,
            "ZQIDX26MAY1000CE",
            "ZQ, INDEX",
            "2026-05-26",
            "1000",
            75,
            "CE",
            "NFO-OPT",
            "NFO",
        )]),
    )
    .unwrap();
    let Lookup::Listed(again) = other.lookup("NFO", 9_800_001) else {
        panic!("found");
    };
    assert!(
        listed.same_contract(&again),
        "1000.0 and 1000 are one strike"
    );
    assert!(
        matches!(one.lookup("", 0), Lookup::Absent),
        "garbage parameters find nothing"
    );
    assert!(
        matches!(one.lookup("NSE", 9_800_001), Lookup::Absent),
        "the exchange is part of the key"
    );
}

#[test]
fn a_token_twice_in_one_snapshot_is_believed_for_neither_row() {
    let root = scratch("duplicated");
    let d1 = day(2026, 5, 4);
    let body = dump(&[
        fut(9_900_001, "ZQIDX26MAYFUT", "2026-05-26", 75),
        fut(9_900_001, "ZQIDX26JUNFUT", "2026-06-30", 75),
    ]);
    snapshot(&root, Kind::Kite, d1, URL, body.as_bytes()).unwrap();
    let book = Book::open(&root).unwrap();
    assert!(matches!(
        book.token_on(d1, "NFO", 9_900_001),
        TokenOn::Duplicated { .. }
    ));
    assert_eq!(
        book.token_on(d1, "NFO", 9_900_001).word(),
        "token_duplicated"
    );
    assert!(matches!(
        book.token_on(d1, "NFO", 1),
        TokenOn::NotListed { .. }
    ));
}

#[test]
fn the_day_table_crosses_a_year_boundary_and_two_snapshots_of_one_day() {
    let (end, start) = (day(2025, 12, 31), day(2026, 1, 1));
    let body = |symbol: &str| dump(&[fut(9_910_001, symbol, "2026-01-27", 75)]);
    let snapshots = vec![
        Snapshot::parse(stamp(end, 1), &body("ZQIDX25DECFUT")).unwrap(),
        Snapshot::parse(stamp(start, 1), &body("ZQIDX26JANFUT")).unwrap(),
        Snapshot::parse(stamp(start, 2), &body("ZQIDX26JANFUTB")).unwrap(),
    ];
    let book = Book::of(snapshots).unwrap();
    let symbol = |d| match book.token_on(d, "NFO", 9_910_001) {
        TokenOn::Resolved { listed, snapshot } => (listed.tradingsymbol().to_owned(), snapshot),
        other => panic!("resolves, got {other:?}"),
    };
    assert_eq!(symbol(end), ("ZQIDX25DECFUT".to_owned(), stamp(end, 1)));
    assert_eq!(
        symbol(start),
        ("ZQIDX26JANFUTB".to_owned(), stamp(start, 2)),
        "the day's newest ordinal"
    );
    // A DAY PAST THE NEWEST SNAPSHOT is governed by its newest ordinal, and a
    // row whose contract expired before the day is `token_expired`, never
    // resolved (R4-28 repair 3, PREC-10): the governing stamp is asserted on
    // the refusal itself.
    match book.token_on(day(2026, 3, 1), "NFO", 9_910_001) {
        TokenOn::Expired { snapshot, listed } => {
            assert_eq!(snapshot, stamp(start, 2));
            assert_eq!(listed.tradingsymbol(), "ZQIDX26JANFUTB");
        }
        other => panic!("expired, got {other:?}"),
    }
    assert_eq!(
        symbol(day(2026, 1, 27)),
        ("ZQIDX26JANFUTB".to_owned(), stamp(start, 2)),
        "the expiry day itself still resolves"
    );
    assert!(
        Book::of(vec![
            Snapshot::parse(stamp(end, 1), &body("A")).unwrap(),
            Snapshot::parse(stamp(end, 1), &body("B")).unwrap(),
        ])
        .is_err()
    );
}

/// R4-28 repair 3 (verify r3 followup PREC-10, reproduced on real data: a
/// NIFTY October future resolved on 2026-11-02, after its 2026-10-27 expiry).
/// A row past its own expiry is `token_expired` on every path: between two
/// snapshots that both list it, on a snapshot day that still lists it, and
/// after the newest snapshot. Invented rows.
#[test]
fn a_token_past_its_contract_expiry_is_expired_never_resolved() {
    let (s1, s2) = (day(2026, 10, 20), day(2026, 10, 30));
    let row = fut(9_940_001, "ZQIDX26OCTFUT", "2026-10-27", 75);
    let next = fut(9_940_002, "ZQIDX26NOVFUT", "2026-11-24", 75);
    let book = Book::of(vec![
        Snapshot::parse(stamp(s1, 1), &dump(&[row.clone(), next.clone()])).unwrap(),
        // A LATE FLUSH: the later snapshot still lists the expired contract.
        Snapshot::parse(stamp(s2, 1), &dump(&[row, next])).unwrap(),
    ])
    .unwrap();
    for (on, governing) in [(day(2026, 10, 28), s1), (s2, s2), (day(2026, 11, 2), s2)] {
        match book.token_on(on, "NFO", 9_940_001) {
            TokenOn::Expired { snapshot, listed } => {
                assert_eq!(snapshot, stamp(governing, 1), "{on}");
                assert_eq!(listed.expiry(), Some(day(2026, 10, 27)), "{on}");
            }
            other => panic!("{on}: expired, got {other:?}"),
        }
        assert_eq!(book.token_on(on, "NFO", 9_940_001).word(), "token_expired");
    }
    for on in [s1, day(2026, 10, 27)] {
        assert!(
            matches!(
                book.token_on(on, "NFO", 9_940_001),
                TokenOn::Resolved { .. }
            ),
            "{on}: on or before its expiry it resolves"
        );
    }
    assert!(
        matches!(
            book.token_on(day(2026, 11, 2), "NFO", 9_940_002),
            TokenOn::Resolved { .. }
        ),
        "a live contract is untouched by the rule"
    );
}

#[test]
fn concurrent_refreshes_of_one_day_write_one_snapshot_or_one_each() {
    let root = Arc::new(scratch("concurrent"));
    let d1 = day(2026, 6, 1);
    let same = Arc::new(dump(&[fut(9_920_001, "ZQIDX26JUNFUT", "2026-06-30", 75)]));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let (root, same) = (Arc::clone(&root), Arc::clone(&same));
            std::thread::spawn(move || snapshot(&root, Kind::Kite, d1, URL, same.as_bytes()))
        })
        .collect();
    let outcomes: Vec<Wrote> = handles
        .into_iter()
        .map(|h| h.join().unwrap().unwrap())
        .collect();
    assert_eq!(
        outcomes
            .iter()
            .filter(|w| matches!(w, Wrote::New(_)))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|w| matches!(w, Wrote::AlreadyHeld(_)))
            .count(),
        3
    );
    let d2 = day(2026, 6, 2);
    let handles: Vec<_> = (0..4_u32)
        .map(|n| {
            let root = Arc::clone(&root);
            std::thread::spawn(move || {
                let body = dump(&[fut(9_920_001, "ZQIDX26JUNFUT", "2026-06-30", 75 + n)]);
                snapshot(&root, Kind::Kite, d2, URL, body.as_bytes())
            })
        })
        .collect();
    let mut ordinals: Vec<u32> = handles
        .into_iter()
        .map(|h| h.join().unwrap().unwrap().stamp().ordinal)
        .collect();
    ordinals.sort_unstable();
    assert_eq!(ordinals, [1, 2, 3, 4]);
}

#[test]
fn a_day_past_its_last_ordinal_is_refused_by_name() {
    let root = scratch("max-ordinal");
    let d1 = day(2026, 6, 3);
    fs::write(
        root.join(file_name(Kind::Kite, stamp(d1, MAX_ORDINAL))),
        b"cut",
    )
    .unwrap();
    let why = snapshot(
        &root,
        Kind::Kite,
        d1,
        URL,
        dump(&[fut(9_930_001, "ZQIDX26JUNFUT", "2026-06-30", 75)]).as_bytes(),
    )
    .unwrap_err();
    assert!(
        why.contains("refused rather than numbered without end"),
        "{why}"
    );
    // R4-28 repair 3: the cut entry is counted as what it is, never as a
    // snapshot.
    assert!(
        why.contains("(0 receipted snapshots, 1 incomplete entries never read)"),
        "{why}"
    );
}

#[test]
fn a_torn_diff_is_refused_by_name() {
    let root = scratch("torn-diff");
    let (_, b) = diff_days(&root);
    fs::write(root.join(diff_name(b)), b"brutex-master-diff-v1|cut").unwrap();
    let why = diff_for(&root, b).unwrap_err();
    assert!(why.contains("torn"), "{why}");
    assert!(parse_diff("").unwrap_err().contains("empty"));
}

#[test]
fn a_value_wider_than_its_field_writes_no_diff() {
    let s = (stamp(day(2026, 6, 4), 1), stamp(day(2026, 6, 5), 1));
    let wide = Item {
        change: Change::Listing,
        exchange: "NFO".to_owned(),
        token: 1,
        key: "Z".repeat(49),
        old: String::new(),
        new: String::new(),
    };
    let why = render_diff(s.0, s.1, &[wide], 0).unwrap_err();
    assert!(why.contains("does not fit"), "{why}");
    let separator = Item {
        key: "ZQ|X".to_owned(),
        ..expected_items()[0].clone()
    };
    assert!(render_diff(s.0, s.1, &[separator], 0).is_err());
}

#[test]
fn file_names_round_trip_and_strangers_are_ignored() {
    let s = stamp(day(2026, 10, 22), 2);
    for kind in Kind::ALL {
        assert_eq!(parse_name(&file_name(kind, s)), Some((kind, s, false)));
        assert_eq!(parse_name(&receipt_name(kind, s)), Some((kind, s, true)));
    }
    assert_eq!(diff_name(s), "20261022_2.diff");
    assert_eq!(
        file_name(Kind::Kite, stamp(s.day, 1)),
        "instruments_20261022.csv.gz"
    );
    for stranger in [
        "instruments_20261022_1.csv.gz",
        "instruments_20261022_02.csv.gz",
        "instruments_20261322.csv.gz",
        "instruments_2026102.csv.gz",
        "instruments_20261022.csv",
        "zerodha_instruments.csv",
        ".master-snapshots.lock",
    ] {
        assert_eq!(parse_name(stranger), None, "{stranger}");
    }
}

#[test]
fn a_clock_that_names_no_day_writes_no_snapshot_and_says_so() {
    let dir = scratch("clock");
    let body = dump(&[fut(9_940_001, "ZQIDX26JUNFUT", "2026-06-30", 75)]);
    let outcome = after_land(
        &dir,
        kite_source(),
        &body,
        &written(),
        Err("the clock is before 1970".to_owned()),
    )
    .unwrap();
    assert_eq!(outcome.day, None);
    let why = outcome.wrote.clone().unwrap_err();
    assert!(
        why.contains("no snapshot was written") && why.contains("1970"),
        "{why}"
    );
    assert!(
        outcome.json().contains(r#""day":null"#),
        "{}",
        outcome.json()
    );
    assert!(
        Catalog::read(&dir.join(SNAPSHOT_DIR))
            .unwrap()
            .receipted(Kind::Kite)
            .is_empty()
    );
}

/// The measured cost of a real-size dump (docs/06-limits.md L-R428-01..03):
/// one snapshot write, one parse, one diff, and `token_on` p50/p99/max over
/// n=200 tokens between two snapshots. Read only: `BRUTEX_KITE_DUMP` names a
/// COPY of a landed dump; every file this writes goes to a new scratch folder.
#[test]
#[ignore = "measures a real-size dump; run by hand with BRUTEX_KITE_DUMP set"]
fn a_real_size_dump_is_measured() {
    use std::hint::black_box;
    use std::time::Instant;
    let Some(path) = std::env::var_os("BRUTEX_KITE_DUMP") else {
        panic!("set BRUTEX_KITE_DUMP to a copy of a landed Kite dump");
    };
    let body = fs::read_to_string(&path).unwrap();
    let root = scratch("real-size");
    let (d1, d2, between) = (day(2026, 10, 15), day(2026, 10, 22), day(2026, 10, 18));
    let started = Instant::now();
    snapshot(&root, Kind::Kite, d1, URL, body.as_bytes()).unwrap();
    let write_ms = started.elapsed().as_millis();
    let changed = body.replacen(",75,FUT,", ",65,FUT,", 3);
    snapshot(&root, Kind::Kite, d2, URL, changed.as_bytes()).unwrap();
    let started = Instant::now();
    let parsed = Snapshot::parse(stamp(d1, 1), &body).unwrap();
    let parse_ms = started.elapsed().as_millis();
    let started = Instant::now();
    let summary = diff_for(&root, stamp(d2, 1)).unwrap().unwrap();
    let diff_ms = started.elapsed().as_millis();
    let book = Book::open(&root).unwrap();
    let step = (parsed.len() / 200).max(1);
    let tokens: Vec<(String, u64)> = parsed
        .rows()
        .step_by(step)
        .take(200)
        .map(|listed| (listed.exchange().to_owned(), listed.token()))
        .collect();
    let started = Instant::now();
    let first = book.token_on(between, &tokens[0].0, tokens[0].1).word();
    let cold_ms = started.elapsed().as_millis();
    let mut ns: Vec<u128> = tokens
        .iter()
        .map(|(exchange, token)| {
            let started = Instant::now();
            black_box(book.token_on(black_box(between), exchange, *token));
            started.elapsed().as_nanos()
        })
        .collect();
    ns.sort_unstable();
    let n = ns.len();
    let compressed = fs::metadata(root.join(file_name(Kind::Kite, stamp(d1, 1))))
        .unwrap()
        .len();
    println!(
        "real-size dump: rows={} body_bytes={} compressed_bytes={compressed} write_ms={write_ms} \
         parse_ms={parse_ms} diff_ms={diff_ms} diff={} first_token_on={first} cold_ms={cold_ms} \
         token_on n={n} p50_ns={} p99_ns={} max_ns={}",
        parsed.len(),
        body.len(),
        summary.counts_line(),
        ns[n / 2],
        ns[n * 99 / 100],
        ns[n - 1],
    );
    assert_eq!(summary.counts[4], 3, "the three injected lot changes");
}

#[test]
fn the_columns_read_are_the_vendors_own_and_none_is_an_isin() {
    let header: Vec<&str> = crate::vendor::KITE_MASTER_HEADER.split(',').collect();
    assert_eq!(header.len(), 12, "the vendor's twelve columns");
    for column in KITE_COLUMNS {
        assert!(header.contains(&column), "{column} is a Kite column");
    }
    // ONE HEADER, NOT TWO: every column the universe reader requires of this
    // vendor is in the same header the snapshots read.
    for column in crate::masters::required_columns(Vendor::Zerodha) {
        assert!(header.contains(&column), "{column} is a Kite column");
    }
    assert!(
        header
            .iter()
            .all(|column| !column.to_ascii_lowercase().contains("isin")),
        "the dump carries no ISIN: an ISIN comes from a same-date NSE file"
    );
    assert!(crate::vendor::KITE_TOKENS_ARE_REUSED.contains("flushed"));
}

/// R4-28 repair 1: between two snapshot days the next later snapshot is that
/// day's NEWEST ordinal (the day's snapshot), never its first. Before the
/// repair a later refresh the same day that corrected the first was ignored,
/// and a token both days' snapshots agree on read as ambiguous.
#[test]
fn the_next_later_snapshot_is_that_days_newest_ordinal_not_its_first() {
    let root = scratch("next-ordinal");
    let (d1, d2) = (day(2026, 1, 5), day(2026, 1, 12));
    let jan = dump(&[fut(9_000_001, "ZQIDX26JANFUT", "2026-01-29", 75)]);
    let apr = dump(&[fut(9_000_001, "ZQIDX26APRFUT", "2026-04-28", 75)]);
    for (on, body) in [(d1, &jan), (d2, &apr), (d2, &jan)] {
        snapshot(&root, Kind::Kite, on, URL, body.as_bytes()).unwrap();
    }
    let book = Book::open(&root).unwrap();
    assert_eq!(book.stamps(), &[stamp(d1, 1), stamp(d2, 1), stamp(d2, 2)]);
    let between = day(2026, 1, 8);
    let TokenOn::Resolved {
        snapshot: held,
        listed,
    } = book.token_on(between, "NFO", 9_000_001)
    else {
        panic!("the day's newest snapshot agrees, so the older one is believed");
    };
    assert_eq!(
        (held, listed.tradingsymbol()),
        (stamp(d1, 1), "ZQIDX26JANFUT")
    );
    // AND THE OTHER WAY: the day's first agrees, its newest does not.
    let other = scratch("next-ordinal-other");
    for (on, body) in [(d1, &jan), (d2, &jan), (d2, &apr)] {
        snapshot(&other, Kind::Kite, on, URL, body.as_bytes()).unwrap();
    }
    match Book::open(&other)
        .unwrap()
        .token_on(between, "NFO", 9_000_001)
    {
        TokenOn::Ambiguous { older, newer } => {
            assert_eq!((older, newer), (stamp(d1, 1), stamp(d2, 2)));
        }
        answer => panic!("the day's newest disagrees, so ambiguous; got {answer:?}"),
    }
}

/// R4-28 repair 1: a receipt is renamed into place whole, so a kill
/// mid-receipt leaves a `.partial` file the catalog names, and the body
/// stays incomplete and unread; the next refresh is the day's snapshot.
#[test]
fn a_receipt_cut_before_its_rename_is_named_and_its_body_never_read() {
    let source = scratch("partial-source");
    let on = day(2026, 1, 5);
    let body = dump(&[fut(9_000_001, "ZQIDX26JANFUT", "2026-01-29", 75)]);
    snapshot(&source, Kind::Kite, on, URL, body.as_bytes()).unwrap();
    let root = scratch("partial");
    let first = stamp(on, 1);
    fs::copy(
        source.join(file_name(Kind::Kite, first)),
        root.join(file_name(Kind::Kite, first)),
    )
    .unwrap();
    let receipt = fs::read(source.join(receipt_name(Kind::Kite, first))).unwrap();
    let partial = format!("{}{PARTIAL}", receipt_name(Kind::Kite, first));
    fs::write(root.join(&partial), &receipt[..receipt.len() / 2]).unwrap();
    let catalog = Catalog::read(&root).unwrap();
    assert!(catalog.receipted(Kind::Kite).is_empty());
    assert_eq!(catalog.incomplete().len(), 2, "{:?}", catalog.incomplete());
    assert!(
        catalog
            .incomplete()
            .iter()
            .any(|one| one.file == partial
                && one.to_string().starts_with(&partial)
                && one.why.contains("before its rename")),
        "{:?}",
        catalog.incomplete()
    );
    assert!(Book::open(&root).unwrap().stamps().is_empty());
    assert_eq!(
        snapshot(&root, Kind::Kite, on, URL, body.as_bytes()),
        Ok(Wrote::New(stamp(on, 2)))
    );
    assert!(
        !root
            .join(format!(
                "{}{PARTIAL}",
                receipt_name(Kind::Kite, stamp(on, 2))
            ))
            .exists(),
        "a committed receipt leaves no partial name"
    );
    assert_eq!(newest_snapshot_day(&root, TODAY), Ok(Some(on)));
}

/// A receipt already at the name is refused, never overwritten.
#[test]
fn a_receipt_already_at_its_name_is_refused_and_left_as_it_is() {
    let root = scratch("receipt-held");
    let held = stamp(day(2026, 1, 5), 1);
    fs::write(root.join(receipt_name(Kind::Kite, held)), b"held").unwrap();
    let why = commit_receipt(&root, Kind::Kite, held, b"new").unwrap_err();
    assert!(why.contains("already exists and is left as it is"), "{why}");
    assert_eq!(
        fs::read(root.join(receipt_name(Kind::Kite, held))).unwrap(),
        b"held"
    );
}

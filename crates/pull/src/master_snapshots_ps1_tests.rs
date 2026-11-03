//! BASE-COV-PULL-PS1 (D-7335): the diff's matching arms and its delisting
//! pass on inputs that fail one condition at a time, and a diff and a receipt
//! whose rename is refused. Every row is invented: `ZQ` names, tokens in the
//! nine millions, and ISINs that end in a letter where every issued ISIN
//! carries its check digit.

#![allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]

use super::tests::{day, dump, fut, nse_file, row, scratch, share, stamp};
use super::*;

/// The two stamps every diff here compares, a day apart.
fn pair() -> (Stamp, Stamp) {
    (stamp(day(2026, 3, 2), 1), stamp(day(2026, 3, 3), 1))
}

/// The diff of two invented dumps, with no NSE file on either date.
fn changes(old: &[String], new: &[String]) -> Vec<Item> {
    let (a, b) = pair();
    diff(
        &Snapshot::parse(a, &dump(old)).unwrap(),
        &Snapshot::parse(b, &dump(new)).unwrap(),
        None,
        None,
    )
}

/// One diff record, spelled out field by field.
fn record(change: Change, exchange: &str, token: u64, key: &str, old: &str, new: &str) -> Item {
    Item {
        change,
        exchange: exchange.to_owned(),
        token,
        key: key.to_owned(),
        old: old.to_owned(),
        new: new.to_owned(),
    }
}

/// **A TOKEN TAKEN BY A NEW KEY IS A RENAME ONLY WHEN THE OLD KEY IS GONE AND
/// THE CONTRACT AGREES.** Each of the four checks on the old row (its key
/// gone, its type, its strike, its underlying) fails once, alone. The old key
/// still listed, under a new token and after the row that took the old token:
/// the new key is a listing and the old key a token change. Another
/// instrument type, another strike, or another underlying name under the
/// same token: the new row is a listing and the old one a delisting, never a
/// symbol or an expiry change.
#[test]
fn a_reused_token_is_a_rename_only_when_its_old_key_is_gone_and_its_contract_agrees() {
    let (was, now) = (9_000_201_u64, 9_000_202_u64);
    assert_eq!(
        changes(
            &[fut(was, "ZQIDX26MARFUT", "2026-03-26", 75)],
            &[
                fut(was, "ZQIDX26APRFUT", "2026-04-28", 75),
                fut(now, "ZQIDX26MARFUT", "2026-03-26", 75),
            ],
        ),
        [
            record(
                Change::Listing,
                "NFO",
                was,
                "ZQIDX26APRFUT",
                "",
                "ZQIDX26APRFUT"
            ),
            record(
                Change::Token,
                "NFO",
                now,
                "ZQIDX26MARFUT",
                &was.to_string(),
                &now.to_string(),
            ),
        ]
    );
    let option = |token: u64, symbol: &str, strike: &str, kind: &str| {
        row(
            token,
            symbol,
            "ZQIDX",
            "2026-03-26",
            strike,
            75,
            kind,
            "NFO-OPT",
            "NFO",
        )
    };
    let underlying = |token: u64, symbol: &str, name: &str| {
        row(
            token,
            symbol,
            name,
            "2026-03-26",
            "0",
            75,
            "FUT",
            "NFO-FUT",
            "NFO",
        )
    };
    let listed_and_delisted = |token: u64, before: &str, after: &str| {
        [
            record(Change::Listing, "NFO", token, after, "", after),
            record(Change::Delisting, "NFO", token, before, before, ""),
        ]
    };
    assert_eq!(
        changes(
            &[option(9_000_301, "ZQIDX26MAR100CE", "100", "CE")],
            &[option(9_000_301, "ZQIDX26MAR100PE", "100", "PE")],
        ),
        listed_and_delisted(9_000_301, "ZQIDX26MAR100CE", "ZQIDX26MAR100PE"),
        "another instrument type"
    );
    assert_eq!(
        changes(
            &[option(9_000_302, "ZQIDX26MAR100CE", "100", "CE")],
            &[option(9_000_302, "ZQIDX26MAR200CE", "200", "CE")],
        ),
        listed_and_delisted(9_000_302, "ZQIDX26MAR100CE", "ZQIDX26MAR200CE"),
        "another strike"
    );
    assert_eq!(
        changes(
            &[underlying(9_000_303, "ZQIDX26MARFUT", "ZQIDX")],
            &[underlying(9_000_303, "ZQOTH26MARFUT", "ZQOTH")],
        ),
        listed_and_delisted(9_000_303, "ZQIDX26MARFUT", "ZQOTH26MARFUT"),
        "another underlying"
    );
}

/// **AN ISIN WHOSE GONE SHARE IS ALREADY CLAIMED, OR THAT NO GONE SHARE
/// CARRIES, IS A LISTING.** A share renamed under its own token claims its
/// old row first. A new share whose same-date ISIN is that old row's own
/// ISIN is then a listing, not a second rename of one share; a new share
/// whose ISIN no gone share carried is a listing too. The renamed share has
/// no ISIN on the new date, so it is named unknown.
#[test]
fn an_isin_whose_gone_share_is_claimed_or_absent_names_a_listing_not_a_rename() {
    let (a, b) = pair();
    let old = Snapshot::parse(a, &dump(&[share(9_000_501, "ZQALPHA", "ZQ ALPHA")])).unwrap();
    let new = Snapshot::parse(
        b,
        &dump(&[
            share(9_000_501, "ZQALPHANEW", "ZQ ALPHA"),
            share(9_000_502, "ZQBETA", "ZQ BETA"),
            share(9_000_503, "ZQGAMMA", "ZQ GAMMA"),
        ]),
    )
    .unwrap();
    let then = Isins::parse(&nse_file(&[("ZQALPHA", "INE000ZQALFA")])).unwrap();
    let today = Isins::parse(&nse_file(&[
        ("ZQBETA", "INE000ZQALFA"),
        ("ZQGAMMA", "INE000ZQGAMA"),
    ]))
    .unwrap();
    assert_eq!(
        diff(&old, &new, Some(&then), Some(&today)),
        [
            record(Change::Listing, "NSE", 9_000_502, "ZQBETA", "", "ZQBETA"),
            record(Change::Listing, "NSE", 9_000_503, "ZQGAMMA", "", "ZQGAMMA"),
            record(
                Change::Symbol,
                "NSE",
                9_000_501,
                "ZQALPHANEW",
                "ZQALPHA",
                "ZQALPHANEW"
            ),
            record(Change::IsinUnknown, "NSE", 9_000_501, "ZQALPHANEW", "", ""),
        ]
    );
}

/// **A CONTRACT LISTED TWICE BEFORE AND ONCE AFTER IS STILL LISTED.** The key
/// index holds one row of a key, so the other row of a contract the old dump
/// lists twice is never matched; its key is in the new dump, a delisting is
/// by key, and so nothing is delisted and nothing changed.
#[test]
fn a_contract_listed_twice_before_and_once_after_is_not_delisted() {
    let june = fut(9_000_401, "ZQIDX26JUNFUT", "2026-06-25", 75);
    assert_eq!(changes(&[june.clone(), june.clone()], &[june]), []);
}

/// **A DIFF WHOSE RENAME IS REFUSED STAYS PARTIAL, WHOLE, AND NAMED.** The
/// partial is written and synced before the rename, so a rename the
/// filesystem refuses (here into a folder that does not exist, which refuses
/// root as it refuses anyone) leaves every byte under the partial's name,
/// nothing under the diff's own name, and a refusal naming the diff it could
/// not commit.
#[test]
fn a_diff_whose_rename_is_refused_stays_partial_whole_and_named() {
    let root = scratch("PS1-DIFF-RENAME");
    let at = stamp(day(2026, 3, 3), 1);
    let unplaced = root.join("ZQ-ABSENT").join(diff_name(at));
    let why = commit_diff(&root, at, &unplaced, b"ZQ INVENTED DIFF").unwrap_err();
    let named = format!("{} could not be committed: ", unplaced.display());
    assert!(why.starts_with(&named), "{why}");
    assert!(
        why.ends_with("; the diff stays partial and is never read"),
        "{why}"
    );
    assert_eq!(
        fs::read(root.join(diff_partial_name(at, 1))).unwrap(),
        b"ZQ INVENTED DIFF"
    );
    assert!(fs::symlink_metadata(root.join(diff_name(at))).is_err());
}

/// A receipt whose rename the host refuses stays partial, whole and named:
/// nothing is at the receipt's own name, so the body it would receipt stays
/// incomplete and is never read, and the refusal names the receipt in the
/// host's words. The rename `commit_receipt` hands `commit_receipt_renaming`,
/// refused by a test (a folder made at the name after the check is a race).
#[test]
fn a_receipt_whose_rename_is_refused_stays_partial_whole_and_named() {
    let root = scratch("PS1-RECEIPT-RENAME");
    let at = stamp(day(2026, 3, 3), 1);
    let refused =
        commit_receipt_renaming(&root, (Kind::Kite, at), b"ZQ INVENTED RECEIPT", &|_, _| {
            Err(std::io::Error::other("ZQ invented: not renamed"))
        });
    assert_eq!(
        refused,
        Err(format!(
            "{} could not be committed: ZQ invented: not renamed; the body stays incomplete \
             and is never read",
            root.join(receipt_name(Kind::Kite, at)).display()
        ))
    );
    let partial = root.join(format!("{}{PARTIAL}", receipt_name(Kind::Kite, at)));
    assert_eq!(fs::read(partial).unwrap(), b"ZQ INVENTED RECEIPT");
    assert!(fs::symlink_metadata(root.join(receipt_name(Kind::Kite, at))).is_err());
}

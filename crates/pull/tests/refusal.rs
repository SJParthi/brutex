//! The two-axis refusal classifier, over its WHOLE cross product.
//!
//! # Why this file enumerates instead of sampling
//!
//! `pull::refusal::classify` takes three inputs — a feed's error-name contract,
//! an HTTP status and a body-level name — and every one of them is a closed
//! set at the point a test runs. Twelve statuses the vendor publishes plus one
//! it does not, ten names its reader knows plus one it does not plus the
//! absent case, and two contract states. **312 combinations**, and the whole
//! space is walked rather than three interesting rows being picked out of it.
//!
//! That matters because the defect this module was written for was invisible
//! at exactly one point in the space: two different names under one status.
//! A sampled test that happened to pick `TokenException` would have passed
//! while `PermissionException` was misreported, which is how the defect
//! survived in the first place.
//!
//! # What is asserted over the whole space, rather than per row
//!
//! Ten properties, each of which must hold at all 312 points:
//!
//! | # | Property |
//! |---|---|
//! | 1 | Total — every point classifies, nothing panics |
//! | 2 | Idempotent — the same inputs answer the same verdict (`CLAUDE.md` §3 rule 5) |
//! | 3 | A feed with **no** declared contract is exactly the status, at every point |
//! | 4 | `decided_by` and `disposition` agree: the named axis is the one that produced the answer |
//! | 5 | `contested` is set **iff** the two axes genuinely disagree |
//! | 6 | `contested`, when set, is never equal to the verdict it lost to |
//! | 7 | `unrecognised` is set **iff** a contract was declared, a name arrived, and its reader did not know it |
//! | 8 | `unrecognised` and `contested` are never both set |
//! | 9 | A recognised name binds at **every** status, not only the one the vendor documents it under |
//! | 10 | Only `Throttled` and `RetryBounded` permit another request; only `NotEntitled` says a later run cannot succeed |

// The same exceptions every test module in this workspace takes: a test that
// cannot panic cannot fail, and the lints that forbid panicking exist to keep
// them out of the crate, not out of its tests.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use pull::kite::{EXCEPTION_COUNT, KITE, KiteException};
use pull::refusal::{Axis, Disposition, ErrorNames, classify, named_error_of, status_disposition};

/// Every status the vendor's own table publishes, plus `200`, plus one it does
/// not publish at all.
///
/// `418` is here because an unstated status is a real answer — a proxy, a WAF,
/// a load balancer in front of the vendor — and the module has a rule for it
/// that is only checkable if something asks.
const STATUSES: [u16; 14] = [
    200, 400, 401, 403, 404, 405, 410, 418, 429, 500, 502, 503, 504, 599,
];

/// A name no vendor sends, used as the stand-in for every name this build's
/// reader has never seen. The vendor's exception list is explicitly open, so
/// this case is not hypothetical.
const UNKNOWN: &str = "QuantumFluxException";

/// Every value the name axis can take: absent, the ten known, and one unknown.
fn names() -> Vec<Option<&'static str>> {
    let mut v: Vec<Option<&'static str>> = vec![None, Some(UNKNOWN), Some("")];
    v.extend(KiteException::ALL.iter().map(|e| Some(e.wire())));
    v
}

/// **The whole cross product, and the ten properties that hold at every point.**
///
/// 14 statuses × 13 name values × 2 contract states = 364 classifications, each
/// checked against every invariant the module claims.
#[test]
fn the_whole_cross_product_holds_every_property() {
    let contracts: [Option<&ErrorNames>; 2] = [None, Some(&KITE)];
    let mut points = 0_u32;
    let mut disagreements = 0_u32;
    let mut unrecognised_seen = 0_u32;

    for contract in contracts {
        for status in STATUSES {
            for name in names() {
                points += 1;
                let v = classify(contract, status, name);

                // 2 — idempotent.
                assert_eq!(
                    v,
                    classify(contract, status, name),
                    "same inputs, different verdict: {contract:?} {status} {name:?}"
                );

                let from_status = status_disposition(status);
                let known = name.and_then(KiteException::from_wire);

                // 3 — no contract declared means the status, unchanged. This is
                // the property that proves adding the module regressed no feed:
                // Dhan and Groww both carry `error_names: None`.
                if contract.is_none() {
                    assert_eq!(
                        v.disposition, from_status,
                        "an undeclared contract must leave the status deciding"
                    );
                    assert_eq!(v.decided_by, Axis::Status);
                    assert_eq!(v.contested, None);
                    assert_eq!(
                        v.unrecognised, None,
                        "a feed with no contract has no name to fail to recognise"
                    );
                    continue;
                }

                // 4 — the axis named is the axis that answered.
                match v.decided_by {
                    Axis::Status => assert_eq!(v.disposition, from_status),
                    Axis::ErrorName => {
                        let e = known.expect("only a recognised name can decide");
                        assert_eq!(v.disposition, e.disposition());
                        // 9 — and it decided REGARDLESS of the status. The
                        // vendor documents `TokenException` under 403; nothing
                        // stops it arriving under a 500, and the name still
                        // binds.
                        assert_eq!(
                            classify(contract, 500, name).disposition,
                            e.disposition(),
                            "{e} must bind at every status, not only its documented one"
                        );
                    }
                }

                // 5 and 6 — contested is exactly the disagreement, and is never
                // the winner.
                match known {
                    Some(e) if e.disposition() != from_status => {
                        disagreements += 1;
                        assert_eq!(
                            v.contested,
                            Some(from_status),
                            "the losing axis must be kept, not resolved away"
                        );
                        assert_ne!(v.contested, Some(v.disposition), "6");
                    }
                    _ => assert_eq!(
                        v.contested, None,
                        "nothing disagreed, so nothing is contested"
                    ),
                }

                // 7 and 8 — unrecognised is exactly the unknown-name case, it
                // carries the string verbatim, and it never coexists with a
                // disagreement (because the status decided, so nothing lost).
                let expect_unrecognised = name.is_some() && known.is_none();
                assert_eq!(
                    v.unrecognised.is_some(),
                    expect_unrecognised,
                    "unrecognised must mark exactly the names the reader did not know"
                );
                if let Some(raw) = v.unrecognised {
                    unrecognised_seen += 1;
                    assert_eq!(Some(raw), name, "the name must survive verbatim");
                    assert_eq!(v.decided_by, Axis::Status);
                    assert_eq!(v.contested, None, "8");
                }

                // 10 — the two operator-facing predicates.
                assert_eq!(
                    v.disposition.permits_more_requests(),
                    matches!(
                        v.disposition,
                        Disposition::Throttled | Disposition::RetryBounded
                    )
                );
                assert_eq!(
                    v.disposition.a_later_run_could_succeed(),
                    v.disposition != Disposition::NotEntitled
                );
                assert_eq!(
                    v.axes_disagreed(),
                    v.contested.is_some(),
                    "the predicate must read the field it names"
                );
            }
        }
    }

    // 1 — every point was reached. The count is asserted so a `names()` that
    // silently shrinks cannot make this suite pass by testing less.
    assert_eq!(points, 14 * 13 * 2, "the whole space, not part of it");
    assert!(
        disagreements > 0,
        "if the axes never disagree the `contested` field is decoration"
    );
    assert!(unrecognised_seen > 0, "the open-name case must be reached");
}

/// **THE PAIR THIS MODULE EXISTS FOR.** Two names, one status, two futures.
///
/// `TokenException` and `PermissionException` are both answered under HTTP 403.
/// Before `pull::refusal` the retry ladder read the status alone, so an API key
/// with no historical-data subscription was reported as an expired session and
/// the operator was told the next pull would read a refreshed credential — a
/// sentence that is false in every clause for that key, and a failure wearing
/// another failure's clothes, which `CLAUDE.md` §4 bans.
#[test]
fn the_two_403s_are_two_different_events() {
    let expired = classify(Some(&KITE), 403, Some("TokenException"));
    let unentitled = classify(Some(&KITE), 403, Some("PermissionException"));

    assert_eq!(expired.disposition, Disposition::SessionDead);
    assert_eq!(unentitled.disposition, Disposition::NotEntitled);
    assert_ne!(
        expired.disposition, unentitled.disposition,
        "the whole point: one status, two answers"
    );

    // The operator-facing difference, which is the one that costs money.
    assert!(
        expired.disposition.a_later_run_could_succeed(),
        "tomorrow's credential fixes an expired session"
    );
    assert!(
        !unentitled.disposition.a_later_run_could_succeed(),
        "no credential this repository can read fixes an unbought subscription"
    );

    // The status axis said `SessionDead` for both. For the unentitled key that
    // is kept as the claim that did NOT bind rather than dropped.
    assert_eq!(unentitled.contested, Some(Disposition::SessionDead));
    assert!(unentitled.axes_disagreed());
    assert_eq!(
        expired.contested, None,
        "the axes agreed here, so there is nothing to contest"
    );

    // And the status axis alone still cannot tell them apart, which is the
    // measurement that makes the sentence above a fact rather than a claim.
    assert_eq!(status_disposition(403), Disposition::SessionDead);
}

/// A name is refused unless it is spelled exactly as the vendor spells it.
///
/// Every one of these is a real way a body arrives wrong — a proxy lowercasing
/// a field, a log line pasted with its whitespace, a truncated read, a name
/// that is a prefix of a real one — and every one of them must reach the
/// operator as itself rather than being rounded to the nearest known name.
#[test]
fn a_near_miss_is_not_a_hit() {
    let near = [
        "",
        " ",
        "tokenexception",
        "TOKENEXCEPTION",
        "TokenException ",
        " TokenException",
        "TokenExceptio",
        "TokenExceptionn",
        "Token",
        "Exception",
        "TokenExceptioX",
        "Token\0Exception",
        "TokenException\n",
        "\u{feff}TokenException",
        "टोकन",
    ];
    for raw in near {
        assert_eq!(
            KiteException::from_wire(raw),
            None,
            "{raw:?} is not how the vendor spells any of its names"
        );
        let v = classify(Some(&KITE), 403, Some(raw));
        assert_eq!(
            v.unrecognised,
            Some(raw),
            "{raw:?} must survive verbatim for the operator to read"
        );
        assert_eq!(
            v.disposition,
            status_disposition(403),
            "and the status decides, because no name was understood"
        );
    }
}

/// A hostile `error_type` costs one length test, not a scan.
///
/// A `match` over string literals switches on the length first, so a megabyte
/// of body-supplied name fails every bucket without a byte being compared.
/// This asserts the ANSWER rather than the timing — a timing assertion here
/// would be the flaky measurement `CLAUDE.md` §3 rule 6 forbids claiming — and
/// `benches/ratio.rs` is where cost is measured.
#[test]
fn a_megabyte_of_error_type_is_refused_like_any_other_wrong_name() {
    let huge = "A".repeat(1_000_000);
    assert_eq!(KiteException::from_wire(&huge), None);

    let v = classify(Some(&KITE), 500, Some(huge.as_str()));
    assert_eq!(
        v.disposition,
        Disposition::RetryBounded,
        "the status decides"
    );
    assert_eq!(v.unrecognised.map(str::len), Some(1_000_000));

    // A name of exactly a REAL name's length, which is the one case a
    // length-only test would let through.
    let same_len = "X".repeat("PermissionException".len());
    assert_eq!(KiteException::from_wire(&same_len), None);
}

/// Every name round-trips, and no two names share a spelling.
#[test]
fn the_ten_names_are_ten_distinct_spellings() {
    let mut seen = std::collections::HashSet::new();
    for e in KiteException::ALL {
        assert_eq!(
            KiteException::from_wire(e.wire()),
            Some(e),
            "{e} must read back as itself"
        );
        assert!(seen.insert(e.wire()), "{e} shares a spelling with another");
        assert!(
            e.wire().ends_with("Exception"),
            "{e}: the vendor's names all end that way, and one that does not is \
             a typo this catches"
        );
        assert!(
            !e.label().is_empty(),
            "{e} must be explainable to an operator"
        );
    }
    assert_eq!(seen.len(), EXCEPTION_COUNT);
    assert_eq!(KiteException::ALL.len(), EXCEPTION_COUNT);

    // Nine are on the page and exactly one is not — the fact that makes the
    // 403 pair separable at all.
    let off_page = KiteException::ALL
        .iter()
        .filter(|e| !e.on_documentation_page())
        .count();
    assert_eq!(off_page, 1, "PermissionException, and nothing else");
    assert!(!KiteException::Permission.on_documentation_page());
}

/// The vendor's three published handling groups are the three this build uses.
///
/// Quoted from <https://kite.trade/docs/connect/v3/exceptions/>; a row that
/// drifts from the page is what this pins.
#[test]
fn the_handling_groups_are_the_vendors_own() {
    // "they are candidates for a bounded retry"
    for e in [KiteException::Network, KiteException::Data] {
        assert_eq!(e.disposition(), Disposition::RetryBounded);
        assert!(e.disposition().permits_more_requests());
    }
    // "deterministic rejections of the request as sent; retrying without
    //  changing the request will fail again"
    for e in [
        KiteException::Input,
        KiteException::Margin,
        KiteException::Holding,
    ] {
        assert_eq!(e.disposition(), Disposition::RequestWrong);
        assert!(!e.disposition().permits_more_requests());
    }
    // "clear the user's session and re-initiate a login"
    assert_eq!(KiteException::Token.disposition(), Disposition::SessionDead);
    // The one the page does not rule on, and the reason this module exists.
    assert_eq!(
        KiteException::Permission.disposition(),
        Disposition::NotEntitled
    );
    // "an unclassified error" is not a retry policy.
    for e in [
        KiteException::User,
        KiteException::Order,
        KiteException::General,
    ] {
        assert_eq!(e.disposition(), Disposition::ReasonGiven);
        assert!(!e.disposition().permits_more_requests());
    }
}

/// Only a 429 narrows the governor's rate, and every disposition is spellable.
#[test]
fn only_a_rate_refusal_narrows_the_rate() {
    for d in Disposition::ALL {
        assert_eq!(d.narrows_the_rate(), d == Disposition::Throttled);
        assert!(!d.word().is_empty());
        assert!(!d.label().is_empty());
        assert_eq!(d.to_string(), d.label());
    }
    assert_eq!(Disposition::ALL.len(), 6);
    // A 5xx names no budget, so it must not narrow one.
    assert!(!status_disposition(502).narrows_the_rate());
    assert!(status_disposition(429).narrows_the_rate());
}

/// Every status the vendor publishes has the verdict its own table describes.
#[test]
fn the_status_table_is_the_vendors_table() {
    let rows: [(u16, Disposition); 12] = [
        (400, Disposition::RequestWrong),
        (401, Disposition::SessionDead),
        (403, Disposition::SessionDead),
        (404, Disposition::RequestWrong),
        (405, Disposition::RequestWrong),
        (410, Disposition::RequestWrong),
        (429, Disposition::Throttled),
        (500, Disposition::RetryBounded),
        (502, Disposition::RetryBounded),
        (503, Disposition::RetryBounded),
        (504, Disposition::RetryBounded),
        // Success carrying an error envelope is the answer contradicting
        // itself, and is not a retry.
        (200, Disposition::ReasonGiven),
    ];
    for (status, expected) in rows {
        assert_eq!(status_disposition(status), expected, "status {status}");
    }
    // An unstated status is `ReasonGiven` — answered, not retried. The cheap
    // error, per the function's own header.
    for unstated in [0_u16, 100, 204, 301, 418, 451, 599, u16::MAX] {
        assert_eq!(
            status_disposition(unstated),
            Disposition::ReasonGiven,
            "status {unstated} is stated by no source"
        );
    }
}

/// Pulling the name out of a body: every malformed shape is "no name", never a
/// second error of this build's own.
#[test]
fn a_malformed_error_body_yields_no_name_rather_than_a_new_failure() {
    // The vendor's own documented envelope.
    assert_eq!(
        named_error_of(
            r#"{"status":"error","message":"Error message","error_type":"GeneralException"}"#,
            KITE.field,
            KITE.envelope
        )
        .as_deref(),
        Some("GeneralException")
    );

    for body in [
        "",
        " ",
        "not json at all",
        "<html><body>502 Bad Gateway</body></html>",
        "null",
        "[1,2,3]",
        "{}",
        r#"{"status":"error"}"#,
        r#"{"error_type":7}"#,
        r#"{"error_type":null}"#,
        r#"{"error_type":["TokenException"]}"#,
        r#"{"error_type":{"name":"TokenException"}}"#,
        r#"{"data":{"error_type":"TokenException"}}"#,
        r#"{"error_type":true}"#,
        "{\"error_type\":\"TokenException\"",
    ] {
        assert_eq!(
            named_error_of(body, KITE.field, KITE.envelope),
            None,
            "{body:?} carries no readable name at the top level"
        );
    }

    // A duplicate key is the last one, which is serde_json's rule and not this
    // build's. Pinned so a swap of parser is visible.
    assert_eq!(
        named_error_of(
            r#"{"error_type":"TokenException","error_type":"InputException"}"#,
            KITE.field,
            KITE.envelope
        )
        .as_deref(),
        Some("InputException")
    );

    // An empty string IS a name that arrived — and it is not a known one, so
    // it must reach the operator rather than being read as absent.
    assert_eq!(
        named_error_of(r#"{"error_type":""}"#, KITE.field, KITE.envelope).as_deref(),
        Some("")
    );
    let v = classify(Some(&KITE), 403, Some(""));
    assert_eq!(v.unrecognised, Some(""));
    assert_eq!(v.disposition, Disposition::SessionDead);
}

/// The contract travels with its citation, and the field it names is the one
/// the vendor writes.
#[test]
fn the_kite_contract_carries_the_field_and_the_source() {
    assert_eq!(KITE.field, "error_type");
    assert!(
        KITE.source.contains("kite.trade"),
        "CLAUDE.md §3 rule 1: the claim is traceable"
    );
    assert!(
        KITE.source.contains("pykiteconnect"),
        "and the tenth name's own source is named, because the page does not \
         carry it"
    );
    // Debug prints the data, never the function pointer's address — a derived
    // Debug here would make any snapshot of a descriptor unstable between runs.
    let shown = format!("{KITE:?}");
    assert!(shown.contains("error_type"), "{shown}");
    assert!(
        !shown.contains("0x"),
        "no address may leak into a rendering: {shown}"
    );
}

/// The shipped descriptors carry the contract exactly where a source exists,
/// and `None` everywhere one does not.
///
/// `None` is a recorded absence rather than a hole — see
/// `HttpSpec::error_names` — and this pins which feeds are in which state, so
/// a row added from memory is visible.
#[test]
fn only_the_feed_whose_error_page_was_read_declares_a_contract() {
    // THREE FEEDS NOW, AND THE RULE IS UNCHANGED: a contract appears where a
    // page has been read, and nowhere else. This asserted ONE — Zerodha — back
    // when the other two brokers' error pages had not been read into the
    // repository. Both are now in the operator's own doc pack, and leaving the
    // row at `None` was not caution: it meant `classify` decided on HTTP status
    // alone, so Dhan's DH-902 "not subscribed" and Groww's GA005 "not
    // authorised" were both reported as dead sessions, sending an operator to
    // refresh a credential that was alive.
    //
    // Each feed is checked against its OWN field, its OWN nesting and its OWN
    // reader, because the three vendors agree on none of them: `error_type` at
    // the root, `errorCode` at the root, `code` inside an `error` object.
    for feed in pull::vendor::Feed::ALL {
        let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
            continue;
        };
        let expected = matches!(
            feed,
            pull::vendor::Feed::Zerodha | pull::vendor::Feed::Dhan | pull::vendor::Feed::Groww
        );
        assert_eq!(
            spec.error_names.is_some(),
            expected,
            "{}: a contract appears here only when a page has been read for it",
            feed.display()
        );
        let Some(contract) = spec.error_names else {
            continue;
        };
        // AND IT ACTUALLY READS THE VENDOR'S NAMES, rather than being a row
        // pointing at an empty reader. The probe is each vendor's own
        // entitlement code — the one the status axis gets wrong — so a row
        // wired to the wrong reader fails here rather than in production.
        let (field, envelope, entitlement) = match feed {
            pull::vendor::Feed::Zerodha => (KITE.field, None, "PermissionException"),
            pull::vendor::Feed::Dhan => ("errorCode", None, "DH-902"),
            // GROWW'S ENTITLEMENT-SHAPED CODE IS NOT AN ENTITLEMENT. Its
            // published table has no token-expiry code at all, so GA005 is
            // where one must arrive and `NotEntitled` would decline the retry
            // it needs. `GA004` is the one that genuinely cannot be retried
            // into existence. See `pull::groww`'s arm comments.
            _ => ("code", Some("error"), "GA004"),
        };
        assert_eq!(contract.field, field, "{}", feed.display());
        assert_eq!(contract.envelope, envelope, "{}", feed.display());
        // EACH VENDOR'S OWN PROBE, and each answers what its own page says.
        // Kite and Dhan publish a split between "your token died" and "you are
        // not subscribed", so theirs are `NotEntitled`. Groww publishes no such
        // split, so the probe here is the code that is unambiguously not
        // retryable rather than the one that is ambiguous.
        let expected = match feed {
            pull::vendor::Feed::Groww => Disposition::ReasonGiven,
            _ => Disposition::NotEntitled,
        };
        assert_eq!(
            (contract.read)(entitlement),
            Some(expected),
            "{}: {entitlement} must read as {expected:?}",
            feed.display()
        );
        assert_eq!((contract.read)(UNKNOWN), None, "{}", feed.display());
        // AND THE CITATION IS NOT EMPTY. A row whose source nobody can go and
        // check is the shape this repository treats as worse than an absent
        // row: it reads as though someone verified it.
        assert!(!contract.source.is_empty(), "{}", feed.display());
    }
}

// ---------------------------------------------------------------------------
// the OTHER thing a feed must be asked rather than assumed: its join key
// ---------------------------------------------------------------------------

/// **THE JOIN KEY IS THE VENDOR'S, NOT A LITERAL.**
///
/// `crates/api/src/server.rs` passed `JoinKey::Isin` as a constant under a
/// comment promising that asking the descriptor "is what keeps that true when
/// it lands". The feed that publishes no ISIN landed on 14 Aug 2026, and the
/// literal became wrong for exactly one vendor.
///
/// The failure it would have produced is the reason this test asserts the
/// derivation rather than the call site: `universe::resolve` **skips an empty
/// key on purpose**, so joining Zerodha on a column it does not have raises no
/// error at all. Every instrument lands in `Verdict::Lacks`, and a total join
/// failure is indistinguishable from a vendor that lists nothing.
///
/// NO CATCH-ALL. Every vendor is named, so a sixth cannot inherit a key by
/// default — which is the exact mechanism that produced the bug.
#[test]
fn the_join_key_is_derived_from_the_vendors_own_master_columns() {
    use brutex_core::vendor::Vendor;
    use pull::universe::JoinKey;

    for vendor in Vendor::ALL {
        let expected = match vendor {
            // Both publish an ISIN column, spelled differently, which is
            // exactly why the column NAME is the thing consulted.
            Vendor::Groww | Vendor::Dhan => JoinKey::Isin,
            // Twelve columns, none an ISIN. docs/00-charter.md §4z.
            Vendor::Zerodha => JoinKey::TradingSymbol,
            // Archives: no master of their own to publish one in.
            Vendor::TrueData | Vendor::Gdfl => JoinKey::TradingSymbol,
            // `Vendor` is `#[non_exhaustive]`, so this arm is required from
            // outside the crate and CANNOT be a default. It panics, because
            // inheriting a key by omission is precisely the bug this test was
            // written for: a vendor added without a row here fails loudly, and
            // whoever adds it has to state which key its master supports.
            other => panic!(
                "{other:?} is a vendor with no join-key row in this test. Its \
                 master_columns().isin is {:?} — name it above rather than \
                 letting it inherit a key by omission, which is the defect \
                 `JoinKey::for_vendor` exists to close.",
                other.master_columns().isin
            ),
        };
        assert_eq!(
            JoinKey::for_vendor(vendor),
            expected,
            "{vendor:?}: the key must follow master_columns().isin, which is \
             {:?}",
            vendor.master_columns().isin
        );
        // AND THE DERIVATION AGREES WITH THE COLUMN, not merely with this
        // table — so editing `master_columns` moves both together.
        assert_eq!(
            JoinKey::for_vendor(vendor) == JoinKey::Isin,
            !vendor.master_columns().isin.is_empty(),
            "{vendor:?}"
        );
    }

    // A SYMBOL JOIN IS THE WEAKER ONE AND MUST SAY SO. A page that renders
    // "500 of 500 matched" identically for both keys tells the reader
    // something it does not know.
    assert!(JoinKey::Isin.is_identity());
    assert!(!JoinKey::TradingSymbol.is_identity());
    assert!(!JoinKey::for_vendor(Vendor::Zerodha).is_identity());
}

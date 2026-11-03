#![cfg(test)]
//! BASE-COV-PULL repair 2 (D-7334): the arms of `http` no test had reached
//! (Route B on `c9d653d5f`, `state/fix/round2/deep-BASE-COV-PULL-c9d653d5f914-gaps.txt`):
//! a credential's `Debug`, a rate budget the governor refuses, a source a
//! caller left ungoverned, and a discovery field in a bars request. The
//! descriptor is the tests' invented one (`https://vendor.invalid`); no
//! request leaves the process.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::tests::spec_top;
use super::*;

/// A credential's `Debug` names each secret it holds and shows neither: one
/// secret, and a pair whose second is shown held, never its value.
#[test]
fn a_credential_says_what_it_holds_and_never_the_value() {
    let one = format!("{:?}", Credential::token("zq-token-invented".to_owned()));
    assert_eq!(one, r#"Credential { token: "<redacted>", key: None }"#);
    let pair = format!(
        "{:?}",
        Credential::pair("zq-key-invented".to_owned(), "zq-token-invented".to_owned())
    );
    assert_eq!(
        pair,
        r#"Credential { token: "<redacted>", key: Some("<redacted>") }"#
    );
}

/// A descriptor whose budget the governor will not take (a ceiling of zero,
/// and one past its range at once) is refused when the source is built,
/// naming the descriptor, and nothing is sent: an unenforced budget never
/// reads as an enforced one.
#[test]
fn a_budget_no_governor_takes_is_refused_when_the_source_is_built() {
    for budget in [
        crate::vendor::Budget {
            per_second: Some(0),
            per_minute: None,
            per_day: None,
        },
        crate::vendor::Budget {
            per_second: Some(u32::MAX),
            per_minute: Some(0),
            per_day: Some(u32::MAX),
        },
    ] {
        let spec = HttpSpec {
            budget,
            ..spec_top(PriceScale::Paisa)
        };
        let refused = HttpSource::new(spec, Credential::token("zq".to_owned())).unwrap_err();
        let FetchError::TransportFailed { detail } = &refused else {
            panic!("{refused:?}");
        };
        assert!(
            detail.contains(
                "https://vendor.invalid declares a rate budget this build cannot enforce"
            ),
            "{detail}"
        );
    }
}

/// A source a caller shared with no governor is ungoverned from then on: a
/// governor handed to it later is not taken (it would enforce a ceiling the
/// caller never shared), and a permit is waited for no time at all.
#[test]
fn a_source_shared_with_no_governor_stays_ungoverned() {
    let source = HttpSource::new(
        spec_top(PriceScale::Paisa),
        Credential::token("zq".to_owned()),
    )
    .unwrap();
    assert!(
        source.governor().is_some(),
        "a declared budget, even one of no span, is governed"
    );
    let shared = source.sharing(None);
    assert!(shared.governor().is_none());
    let offered = crate::rate::Governor::new(Some(1), None, None).unwrap();
    let again = shared.sharing(Some(std::sync::Arc::new(std::sync::Mutex::new(offered))));
    assert!(again.governor().is_none(), "never governed after the fact");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");
    assert_eq!(runtime.block_on(again.wait_for_permit()), Ok(()));
}

/// Each discovery field (underlying, year, month, expiry date) in a bars
/// request is refused by its name, never sent with some other value.
#[test]
fn a_discovery_field_in_a_bars_request_is_refused_by_name() {
    let source = HttpSource::new(
        spec_top(PriceScale::Paisa),
        Credential::token("zq".to_owned()),
    )
    .unwrap();
    let day = crate::session::Day::new(2024, 4, 1).unwrap();
    let request = BarRequest {
        instrument_id: "13".to_owned(),
        listing: crate::vendor::Listing::Index,
        window: crate::session::Window::new(day, day).unwrap(),
        granularity: crate::vendor::Granularity::Minute1,
    };
    for (name, value) in [
        ("underlying", crate::vendor::ParamValue::Underlying),
        ("year", crate::vendor::ParamValue::Year),
        ("month", crate::vendor::ParamValue::Month),
        ("expiry", crate::vendor::ParamValue::ExpiryDate),
    ] {
        let param = crate::vendor::Param { name, value };
        let refused = source
            .resolve_param(&param, &request, "2024-04-01", "2024-04-02")
            .unwrap_err();
        let FetchError::NotABarsParam { field } = &refused else {
            panic!("{refused:?}");
        };
        assert_eq!(*field, name);
    }
}

// ── BASE-COV-PULL-PS3 (Route B on `2e7a0b6d`): the arms still unreached ────
//
// Every server below is a loopback socket in this process, and every token
// is invented. No request leaves the host.

/// A loopback server that answers one request with `answer` (a whole HTTP
/// response) and hands back the request it read. It is never joined: a
/// caller that never connects fails at its own timed wait.
fn serve_once(answer: String) -> (String, std::sync::mpsc::Receiver<String>) {
    use std::io::{Read as _, Write as _};
    let socket = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let addr = socket.local_addr().expect("an address");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (mut stream, _) = socket.accept().expect("a client connects");
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).unwrap_or(0);
        let _sent = tx.send(String::from_utf8_lossy(&buf[..n]).into_owned());
        let _written = stream.write_all(answer.as_bytes());
        let _flushed = stream.flush();
    });
    (format!("http://{addr}"), rx)
}

/// The request a [`serve_once`] server read, within five seconds.
fn seen(report: &std::sync::mpsc::Receiver<String>) -> String {
    report
        .recv_timeout(core::time::Duration::from_secs(5))
        .expect("the request has to reach the fixture, or this proves nothing")
}

/// An HTTP/1.1 answer of `status` carrying `body`, closing the connection.
fn answer(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// Dhan's shipped row, whose error contract these tests borrow. Its base URL
/// is left behind wherever a request is made, so no source here names a feed
/// and no capture is written.
fn dhan() -> HttpSpec {
    let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
    else {
        panic!("Dhan is an HTTP broker")
    };
    spec
}

/// `url` as a descriptor's base URL, which is `'static`.
fn base(url: String) -> &'static str {
    Box::leak(url.into_boxed_str())
}

/// One blocking call on a current-thread runtime.
fn block_on<F: core::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(future)
}

/// A one-day bars request for an index, ending on `last`.
fn ending_on(last: crate::session::Day) -> BarRequest {
    BarRequest {
        instrument_id: "13".to_owned(),
        listing: crate::vendor::Listing::Index,
        window: crate::session::Window::new(last, last).unwrap(),
        granularity: crate::vendor::Granularity::Minute1,
    }
}

/// An invented day in 2019 that a request can name.
fn a_day() -> crate::session::Day {
    crate::session::Day::new(2019, 6, 27).unwrap()
}

/// The per-second allowance a source's governor stands at.
fn allowance(source: &HttpSource) -> Option<u32> {
    source.governor.as_ref().and_then(|lock| {
        lock.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .permitted(crate::rate::WindowSpan::Second)
    })
}

/// Dhan's throttle, named in a body: the refusal contract reads it.
const THROTTLED: &str = r#"{"errorCode":"DH-904","errorMessage":"Too many requests"}"#;

/// **A CLIENT THE HOST CANNOT BUILD IS REFUSED NAMING THE CLIENT, AND A
/// CREDENTIAL THAT DOES NOT FIT IS FOUND BEFORE ANY CLIENT IS BUILT.** The
/// TLS backend this build links never fails in a test process, so the
/// builder is handed in: one that fails is `transport_failed` carrying its
/// words, and with a mismatched credential the builder is never called.
#[test]
fn a_client_the_host_cannot_build_is_refused_after_the_credential_is_checked() {
    let refused = HttpSource::built_with(
        &spec_top(PriceScale::Paisa),
        Credential::token("zq-invented".to_owned()),
        || Err("no TLS backend in this invented host".to_owned()),
    )
    .unwrap_err();
    assert_eq!(
        refused,
        FetchError::TransportFailed {
            detail: "the HTTPS client could not be built: no TLS backend in this invented host"
                .to_owned()
        }
    );
    let mismatch = HttpSource::built_with(
        &spec_top(PriceScale::Paisa),
        Credential::pair("zq-key".to_owned(), "zq-token".to_owned()),
        || panic!("the client was built before the credential was checked"),
    )
    .unwrap_err();
    assert!(
        matches!(mismatch, FetchError::CredentialMismatch { .. }),
        "{mismatch:?}"
    );
}

/// **A KEY COUNTS ONLY INSIDE AN OBJECT, AND ONLY WHEN IT DECODES.** Bodies
/// `serde_json` never passes on, driven directly: an unterminated string and
/// a trailing escape end the walk, a key-shaped string outside any object or
/// inside an array is no key, and a key whose escape does not decode is not
/// counted however often it appears; an escaped spelling of a held key is
/// still the same key.
#[test]
fn a_key_counts_only_inside_an_object_and_only_when_it_decodes() {
    for body in [
        r#"{"open":1,"op"#,
        r#""x\"#,
        r#""a":1,"a":2"#,
        r#"["a":1,"a":2]"#,
        r#"{"\q":1,"\q":2}"#,
        "",
    ] {
        assert_eq!(repeated_key(body), None, "{body}");
    }
    assert_eq!(repeated_key(r#"{"a":1,"a":2}"#), Some("a".to_owned()));
}

/// **A PRICE COLUMN SHORTER THAN `open` KEEPS ITS ROW FOR THE LENGTH CHECK.**
/// The relationship is judged only where all four prices are there: row 0
/// (a high under its open) is dropped from every column, row 1 (no high at
/// all) is kept, and the short column is left short, so `RawWindow::decode`
/// still refuses the disagreement instead of a silent drop erasing it.
#[test]
fn a_price_column_shorter_than_open_keeps_its_row_for_the_length_check() {
    let mut arrays = ParallelArrays {
        open: vec![100, 200],
        high: vec![50],
        low: vec![40, 190],
        close: vec![45, 195],
        volume: vec![1, 2],
        timestamp: vec![11, 12],
        open_interest: Vec::new(),
    };
    let refused = drop_impossible_bars(&mut arrays, crate::vendor::Listing::Equity);
    assert_eq!(refused.count, 1, "row 0 alone");
    assert_eq!(arrays.open, vec![200]);
    assert_eq!(arrays.high, Vec::<i64>::new(), "still the short one");
    assert_eq!(arrays.timestamp, vec![12]);
    assert!(
        matches!(
            RawWindow::decode(&arrays),
            Err(FetchError::LengthDisagreement { .. })
        ),
        "the disagreement is still refused"
    );
}

/// **A FEED'S EXTRA HEADERS GO OUT ON THE ROLLING POST, THE DISCOVERY GET AND
/// THE BARS REQUEST ALIKE.** A row naming a header beyond the credential (as
/// Groww's names its API version) has it sent on all three paths, each driven
/// over its own loopback socket, and each answer is taken as a success.
#[test]
fn a_feeds_extra_headers_go_out_on_every_request_path() {
    const EXTRA: &[(&str, &str)] = &[("x-zq-extra", "zq-invented-value")];
    let sent = "x-zq-extra: zq-invented-value";
    for post in [true, false] {
        let spec = HttpSpec {
            extra_headers: EXTRA,
            ..spec_top(PriceScale::Rupees)
        };
        let source = HttpSource::new(spec, Credential::token("zq-secret".to_owned())).unwrap();
        let (url, report) = serve_once(answer("200 OK", "{}"));
        let got = block_on(async {
            if post {
                source.post_json(&url, "{}".to_owned()).await
            } else {
                crate::chain::Discovery::get(&source, &url).await
            }
        });
        assert_eq!(got, Ok("{}".to_owned()), "post {post}");
        assert!(seen(&report).contains(sent), "post {post}");
    }
    let body = r#"{"open":[1.5],"high":[1.5],"low":[1.5],"close":[1.5],"volume":[3],"timestamp":[1561607100]}"#;
    let (url, report) = serve_once(answer("200 OK", body));
    let spec = HttpSpec {
        base_url: base(url),
        extra_headers: EXTRA,
        ..spec_top(PriceScale::Rupees)
    };
    let source = HttpSource::new(spec, Credential::token("zq-secret".to_owned())).unwrap();
    let window = block_on(source.window_async(&ending_on(a_day()))).unwrap();
    assert_eq!(window.rows.len(), 1);
    assert_eq!(window.rows[0].open, 150);
    assert!(seen(&report).contains(sent));
}

/// **A SOURCE WITH NO GOVERNOR TAKES A THROTTLE AND MOVES NOTHING.** Shared
/// with no governor, a source answered 429 on the rolling POST and on the
/// bars request, a throttle named under a 400 on a discovery GET, and 2xx
/// bodies naming a throttle or nothing, still refuses each by its status and
/// names what the vendor named, with no governor to tell.
#[test]
fn a_source_with_no_governor_is_refused_alike_and_records_nothing() {
    let spec = HttpSpec {
        error_names: dhan().error_names,
        ..spec_top(PriceScale::Rupees)
    };
    let ungoverned = |spec: HttpSpec| {
        HttpSource::new(spec, Credential::token("zq-secret".to_owned()))
            .unwrap()
            .sharing(None)
    };
    let source = ungoverned(spec);
    assert!(source.governor().is_none());
    let (url, report) = serve_once(answer("429 Too Many Requests", "{}"));
    let refusal = block_on(source.post_json(&url, "{}".to_owned())).unwrap_err();
    seen(&report);
    assert_eq!(refusal.status, Some(429));
    let (url, report) = serve_once(answer("400 Bad Request", THROTTLED));
    let refusal = block_on(crate::chain::Discovery::get(&source, &url)).unwrap_err();
    seen(&report);
    assert_eq!(
        (refusal.status, refusal.named),
        (Some(400), Some(crate::refusal::Disposition::Throttled))
    );
    let (url, report) = serve_once(answer("429 Too Many Requests", "{}"));
    let bars = ungoverned(HttpSpec {
        base_url: base(url),
        ..spec
    });
    let refused = block_on(bars.window_async(&ending_on(a_day()))).unwrap_err();
    seen(&report);
    assert!(
        matches!(refused, FetchError::VendorRefused { status: 429, .. }),
        "{refused:?}"
    );
    let named = serde_json::from_str::<serde_json::Value>(THROTTLED).unwrap();
    assert_eq!(
        source.weigh_body_parsed(Some(&named)),
        Some(crate::refusal::Disposition::Throttled)
    );
    assert_eq!(
        source.weigh_body_parsed(Some(&serde_json::json!({"data": {}}))),
        None
    );
    assert!(source.governor().is_none(), "still ungoverned");
}

/// **A 429 WHOSE BODY ALSO NAMES A THROTTLE NARROWS THE ALLOWANCE ONCE.** The
/// status already records the throttle; the body naming it too is the same
/// throttle, so on the rolling POST and the discovery GET the allowance stands
/// exactly one step down, where a governor throttled once stands, and not
/// where one throttled twice does.
#[test]
fn a_429_whose_body_also_names_a_throttle_narrows_the_allowance_once() {
    let budget = crate::vendor::Budget {
        per_second: Some(40),
        per_minute: None,
        per_day: None,
    };
    let spec = HttpSpec {
        error_names: dhan().error_names,
        budget,
        ..spec_top(PriceScale::Rupees)
    };
    let mut reference = crate::rate::Governor::new(Some(40), None, None).unwrap();
    reference.record_throttled();
    let once = reference.permitted(crate::rate::WindowSpan::Second);
    reference.record_throttled();
    let twice = reference.permitted(crate::rate::WindowSpan::Second);
    assert_ne!(once, twice, "one throttle and two are told apart");
    for post in [true, false] {
        let source = HttpSource::new(spec, Credential::token("zq-secret".to_owned())).unwrap();
        let (url, report) = serve_once(answer("429 Too Many Requests", THROTTLED));
        let refusal = block_on(async {
            if post {
                source.post_json(&url, "{}".to_owned()).await
            } else {
                crate::chain::Discovery::get(&source, &url).await
            }
        })
        .unwrap_err();
        seen(&report);
        assert_eq!(
            (refusal.status, refusal.named),
            (Some(429), Some(crate::refusal::Disposition::Throttled)),
            "post {post}"
        );
        assert_eq!(allowance(&source), once, "post {post}");
    }
}

/// **THE BARS REQUEST NAMES A WINDOW END IT CANNOT SEND, A REFUSAL UNDER A
/// 200, AND A BODY IT CANNOT READ, EACH FOR WHAT IT IS.** The last nameable
/// day has no exclusive successor, so the request is refused before any
/// socket; a 200 whose body names a dead session is the vendor's refusal and
/// carries its head; a 200 whose columns disagree in length is a body not
/// understood, worded by the disagreement itself.
#[test]
fn the_bars_request_names_an_unsendable_end_a_200_refusal_and_an_unreadable_body() {
    let token = || Credential::token("zq-secret".to_owned());
    let source = HttpSource::new(spec_top(PriceScale::Rupees), token()).unwrap();
    let last = crate::session::Day::new(9999, 12, 31).unwrap();
    let refused = block_on(source.window_async(&ending_on(last))).unwrap_err();
    let FetchError::TransportFailed { detail } = &refused else {
        panic!("{refused:?}")
    };
    assert!(detail.contains("9999-12-31 has no successor"), "{detail}");

    let dead = r#"{"errorType":"Invalid_Authentication","errorCode":"DH-901","errorMessage":"Client ID or access token is invalid"}"#;
    let (url, report) = serve_once(answer("200 OK", dead));
    let spec = HttpSpec {
        base_url: base(url),
        error_names: dhan().error_names,
        ..spec_top(PriceScale::Rupees)
    };
    let source = HttpSource::new(spec, token()).unwrap();
    let unanswered = block_on(source.window_answered_async(&ending_on(a_day()))).unwrap_err();
    seen(&report);
    assert!(
        matches!(
            unanswered.error,
            FetchError::VendorRefused {
                status: 200,
                named: Some(crate::refusal::Disposition::SessionDead),
                ..
            }
        ),
        "{:?}",
        unanswered.error
    );
    let head = unanswered.refused.expect("the head travels with it");
    assert_eq!((head.status, &*head.head), (200, dead));

    let short =
        r#"{"open":[1,1],"high":[1],"low":[1,1],"close":[1,1],"volume":[1,1],"timestamp":[1,2]}"#;
    let (url, report) = serve_once(answer("200 OK", short));
    let spec = HttpSpec {
        base_url: base(url),
        ..spec_top(PriceScale::Rupees)
    };
    let source = HttpSource::new(spec, token()).unwrap();
    let refused = block_on(source.window_async(&ending_on(a_day()))).unwrap_err();
    seen(&report);
    let why = decode_body(
        short,
        &spec_top(PriceScale::Rupees),
        crate::vendor::Listing::Index,
    )
    .unwrap_err();
    assert!(
        matches!(why, FetchError::LengthDisagreement { .. }),
        "{why:?}"
    );
    assert_eq!(
        refused,
        FetchError::BodyNotUnderstood {
            detail: why.to_string()
        }
    );
}

/// The positional shape: one array per bar under `candles`.
fn positional() -> HttpSpec {
    HttpSpec {
        response: ResponseShape::PositionalRows {
            envelope: None,
            array: "candles",
        },
        ..spec_top(PriceScale::Rupees)
    }
}

/// **THE POSITIONAL SHAPE REFUSES A BAR THAT IS NOT SIX OR SEVEN CELLS, AND
/// SKIPS A BAR WITH A NULL PRICE.** A bar that is an object, or of zero, five
/// or eight cells, is refused naming the bar and what it was; a null price
/// skips its bar (the third decode shape now asserted alike with the other
/// two), a null volume on an index is zero, and a window of only null bars is
/// a window of none, not a refusal.
#[test]
fn the_positional_shape_refuses_a_bar_of_another_width_and_skips_a_null_price() {
    let index = crate::vendor::Listing::Index;
    for (rows, said) in [
        (
            r#"[{"t":1}]"#,
            r#"bar 0 is {"t":1}, and this vendor sends one ARRAY per bar"#,
        ),
        (r"[[]]", "bar 0 carries 0 cell(s); "),
        (
            r"[[1751337900,1,1,1,1,1],[1,2,3,4,5]]",
            "bar 1 carries 5 cell(s); ",
        ),
        (r"[[1,2,3,4,5,6,7,8]]", "bar 0 carries 8 cell(s); "),
    ] {
        let body = format!(r#"{{"candles":{rows}}}"#);
        let refused = decode_body(&body, &positional(), index).unwrap_err();
        let FetchError::TransportFailed { detail } = &refused else {
            panic!("{refused:?}")
        };
        assert!(detail.starts_with(said), "{detail}");
    }
    let body = r#"{"candles":[
        [1751337900,100.00,100.50,99.50,100.25,10],
        [1751337960,null,100.50,99.50,100.25,0],
        [1751338020,101.00,101.50,100.50,101.25,null]]}"#;
    let window = decode_body(body, &positional(), index).unwrap();
    assert_eq!(window.rows.len(), 2, "the untraded bar is skipped");
    let first = &window.rows[0];
    assert_eq!(
        (first.timestamp, first.open, first.volume),
        (1_751_337_900, 10_000, 10)
    );
    let second = &window.rows[1];
    assert_eq!(
        (second.timestamp, second.close, second.volume),
        (1_751_338_020, 10_125, 0)
    );
    let all_null = r#"{"candles":[[1751337900,null,null,null,null,null]]}"#;
    assert!(
        decode_body(all_null, &positional(), index)
            .unwrap()
            .rows
            .is_empty()
    );
}

/// **A FEED THAT SPELLS ITS STAMPS AS TEXT REFUSES A NUMBER, AND ONE WHOSE
/// STAMPS CARRY AN OFFSET REFUSES A STAMP WITHOUT ONE.** Each text encoding
/// names the bar and the number it was sent; a stamp with no offset on an
/// offset feed is refused rather than read as IST; and the widest offsets on
/// the first and the last nameable instants still land, whole seconds apart
/// from their local times.
#[test]
fn a_text_stamp_feed_refuses_a_number_and_an_offset_feed_refuses_no_offset() {
    use crate::vendor::TimestampEncoding as T;
    for encoding in [T::IstDateTimeText, T::IsoDateTimeText, T::IsoDateTimeOffset] {
        let refused = one_stamp(&serde_json::json!(1_751_337_900), encoding, 4).unwrap_err();
        assert_eq!(
            refused,
            FetchError::TransportFailed {
                detail: "bar 4 stamps 1751337900, and this feed spells its timestamps as text"
                    .to_owned()
            },
            "{encoding:?}"
        );
    }
    let refused = one_stamp(
        &serde_json::json!("2025-07-01T09:15:00"),
        T::IsoDateTimeOffset,
        2,
    )
    .unwrap_err();
    let FetchError::TransportFailed { detail } = &refused else {
        panic!("{refused:?}")
    };
    assert!(
        detail.starts_with(
            "bar 2 stamps \"2025-07-01T09:15:00\", and this feed's timestamps CARRY their own \
             UTC offset"
        ),
        "{detail}"
    );
    let widest = 14 * 3_600 + 59 * 60;
    let last = crate::session::Day::new(9999, 12, 31).unwrap();
    let last_local = i64::from(last.days_from_epoch()) * 86_400 + 86_399;
    for (stamp, utc) in [
        ("1970-01-01T00:00:00+14:59", -widest),
        ("9999-12-31T23:59:59-14:59", last_local + widest),
    ] {
        assert_eq!(
            one_stamp(&serde_json::json!(stamp), T::IsoDateTimeOffset, 0),
            Ok(utc),
            "{stamp}"
        );
    }
}

/// **A PATH SEGMENT IS REFUSED UNLESS EVERY BYTE MEANS ITSELF AND IT IS NOT A
/// DOT-SEGMENT.** `.` and `..` (which a resolver elides or climbs), an empty
/// segment, a space, a slash, a percent sign and a non-ASCII letter are each
/// refused naming the placeholder and the value; a run of digits, a dotted
/// symbol, every other unreserved mark and three dots pass unchanged.
#[test]
fn a_path_segment_is_refused_unless_every_byte_means_itself() {
    for value in [".", "..", "", "NIFTY 50", "a/b", "a%20b", "\u{e9}"] {
        assert_eq!(
            path_safe(value, "instrument_token"),
            Err(FetchError::PathSegmentUnusable {
                placeholder: "instrument_token",
                value: value.to_owned()
            }),
            "{value:?}"
        );
    }
    for value in ["256265", "BAJAJ.NS", "a-b_c~d", "...", ".x"] {
        assert_eq!(path_safe(value, "instrument_token"), Ok(value));
    }
}

/// Runs `body` alone in a child of this test binary whose environment also
/// carries `env`, and asserts the child passed exactly one test. The capture
/// budgets are counters for the whole process, so a test that spends them
/// runs where it spends nobody else's; and a test cannot set a variable in
/// its own process (`set_var` is `unsafe` under edition 2024, and this crate
/// forbids `unsafe`). The same shape as `crate::support::where_permission_binds`,
/// which carries no environment.
fn in_a_child_with(test: &str, env: &[(&str, &std::ffi::OsStr)], body: impl FnOnce()) {
    const CHILD: &str = "BRUTEX_HTTP_COV_CHILD";
    if std::env::var_os(CHILD).is_some() {
        body();
        return;
    }
    let mut command = std::process::Command::new(std::env::current_exe().expect("this binary"));
    command
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CHILD, "1");
    for (key, value) in env {
        command.env(key, value);
    }
    let output = command.output().expect("the child test process starts");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let said = format!(
        "child {test} failed or ran no test:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "{said}"
    );
}

/// The files under `root/captures` whose names begin `prefix`.
fn captures(root: &std::path::Path, prefix: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root.join("captures"))
        .map(|listing| {
            listing
                .flatten()
                .map(|item| item.file_name().to_string_lossy().into_owned())
                .filter(|name| name.starts_with(prefix))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// A source built from Dhan's shipped row, so it names its feed; it is only
/// ever handed bodies here, never sent.
fn dhan_source() -> HttpSource {
    HttpSource::new(dhan(), Credential::token("zq-secret".to_owned())).unwrap()
}

/// **THE FIRST ANSWERS AND UNREADABLE BODIES ARE KEPT UP TO THE BUDGET AND
/// NEVER PAST IT.** In a child whose folder root is a scratch folder, a feed's
/// answers are kept four to a slot and the fifth and sixth are not, and the
/// same for the bodies this build could not read, each written with the
/// sentence saying why; nothing is written anywhere else.
#[test]
fn the_first_answers_and_unreadable_bodies_are_kept_up_to_the_budget() {
    let root = crate::gdfl_fixtures::scratch("http-keep");
    in_a_child_with(
        "http::cov_tests::the_first_answers_and_unreadable_bodies_are_kept_up_to_the_budget",
        &[(crate::folder::ROOT_ENV, root.as_os_str())],
        || {
            let root = std::path::PathBuf::from(
                std::env::var_os(crate::folder::ROOT_ENV).expect("the parent named the root"),
            );
            let source = dhan_source();
            let feed = crate::vendor::Feed::Dhan;
            let slot = crate::capture::PER_SLOT;
            for _ in 0..slot + 2 {
                source.keep_first(crate::capture::Method::Post, "http://127.0.0.1/zq", "{}");
                source.keep_unreadable("http://127.0.0.1/zq", "{\"zq\":-1}", "an invented reason");
            }
            assert_eq!(
                crate::capture::kept(feed, crate::capture::Method::Post),
                slot
            );
            assert_eq!(crate::capture::unread_kept(feed), slot);
            let wire = feed.wire();
            let post = crate::capture::Method::Post.word();
            let kept = captures(&root, &format!("{wire}-{post}-"));
            let unread = captures(&root, &format!("{wire}-unreadable-"));
            assert_eq!(kept.len(), usize::try_from(slot).unwrap(), "{kept:?}");
            assert_eq!(unread.len(), usize::try_from(slot).unwrap(), "{unread:?}");
            for name in &unread {
                let text = std::fs::read_to_string(root.join("captures").join(name)).unwrap();
                assert!(text.contains("why: an invented reason"), "{text}");
                assert!(text.ends_with("{\"zq\":-1}"), "{text}");
            }
        },
    );
}

/// **A FOLDER ROOT THAT IS SET BUT EMPTY KEEPS NOTHING AND FAILS NOTHING.**
/// In a child whose root variable is set to nothing, a feed's answer and an
/// unreadable body are each offered once: neither is kept, neither spends
/// its budget, each refusal is counted, and the call returns as it would
/// have had the write succeeded.
#[test]
fn an_unusable_folder_root_keeps_nothing_and_fails_nothing() {
    in_a_child_with(
        "http::cov_tests::an_unusable_folder_root_keeps_nothing_and_fails_nothing",
        &[(crate::folder::ROOT_ENV, std::ffi::OsStr::new(""))],
        || {
            assert!(
                crate::folder::root().is_err(),
                "the premise: no usable root"
            );
            let source = dhan_source();
            let feed = crate::vendor::Feed::Dhan;
            source.keep_first(crate::capture::Method::Get, "http://127.0.0.1/zq", "{}");
            source.keep_unreadable("http://127.0.0.1/zq", "{}", "an invented reason");
            assert_eq!(crate::capture::kept(feed, crate::capture::Method::Get), 0);
            assert_eq!(crate::capture::unread_kept(feed), 0);
            assert_eq!(crate::capture::refused(), 2);
        },
    );
}

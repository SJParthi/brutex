#![cfg(test)]
//! Finite selector/projection tests. These do not price or attest market bars.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "bounded projection fixtures and assertions"
)]
use super::*;

const ID: &str = "1212121212121212121212121212121212121212121212121212121212121212";

#[test]
fn exact_model_selectors_require_completion_for_every_later_or_detail_page() {
    for model in [Model::Statistics, Model::Admission, Model::Qualification] {
        assert!(Asked::parse(&format!("identity={ID}"), model).is_ok());
        assert!(
            Asked::parse(
                &format!("identity={ID}&completion={ID}&offset=1&limit=256"),
                model
            )
            .is_ok()
        );
        for suffix in [
            "&offset=1",
            "&offset=01",
            "&limit=0",
            "&limit=257",
            "&path=/tmp",
            "&identity=x",
            "&completion=",
            "&offset=18446744073709551616",
            "&kind=trades",
            "&limit=2&limit=2",
            "&max_bytes=1610612736",
        ] {
            assert!(
                Asked::parse(&format!("identity={ID}{suffix}"), model).is_err(),
                "{model:?}: {suffix}"
            );
        }
    }
    for kind in ["sources", "splits"] {
        assert!(Asked::parse(&format!("identity={ID}&kind={kind}"), Model::Statistics).is_err());
        let query = format!("identity={ID}&completion={ID}&kind={kind}");
        assert!(Asked::parse(&query, Model::Statistics).is_ok());
        assert!(Asked::parse(&query, Model::Admission).is_err());
        assert!(Asked::parse(&query, Model::Qualification).is_err());
    }
}

#[test]
fn page_cardinality_and_completion_never_accept_a_partial_or_replacement_page() {
    let asked = Asked::parse(
        &format!("identity={ID}&completion={ID}&offset=1&limit=2"),
        Model::Statistics,
    )
    .unwrap();
    let body = base(
        &asked,
        [0x12; 32],
        4,
        vec![json!({"index":"1"}), json!({"index":"2"})],
    )
    .unwrap();
    assert_eq!(body["next"], "3");
    assert_eq!(body["total"], "4");
    assert!(base(&asked, [0x13; 32], 4, vec![json!({}), json!({})]).is_err());
    assert!(base(&asked, [0x12; 32], 4, vec![json!({})]).is_err());
    assert!(base(&asked, [0x12; 32], 0, vec![]).is_err());
    assert_eq!(
        base(&asked, [0x12; 32], 1, vec![]).unwrap()["next"],
        Value::Null
    );
}

#[test]
fn exact_float_bits_and_unavailable_codes_are_not_rounded_into_success() {
    for value in [0.0_f64, -0.0, f64::MIN_POSITIVE, f64::MAX] {
        let projected = float(value.to_bits());
        assert_eq!(projected["bits"], value.to_bits().to_string());
        let restored = projected["decimal"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap();
        assert_eq!(restored.to_bits(), value.to_bits());
    }
    for bits in [
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        0x7ff8_0000_0000_0042,
    ] {
        let projected = float(bits);
        assert_eq!(projected["bits"], bits.to_string());
        assert!(projected["decimal"].is_null());
    }
    assert_eq!(
        availability(2).unwrap(),
        "unavailable-constant-return-candidate"
    );
    assert_eq!(availability(3).unwrap(), "unavailable-numerical-refusal");
    assert!(availability(0).is_err());
    assert!(availability(4).is_err());
    let test = family_test([0, 0, 9_007_199_254_740_993, u64::MAX, 1, 2, 3, 4]);
    assert_eq!(test["exact_probability"]["numerator"], "9007199254740993");
    assert_eq!(
        test["exact_probability"]["denominator"],
        u64::MAX.to_string()
    );
}

#[tokio::test]
async fn public_evidence_routes_reject_invalid_selectors_before_accessing_a_store() {
    for uri in [
        "/boolean-statistics.json?identity=bad",
        "/boolean-statistics.json?kind=sources",
        "/boolean-statistics.json?identity=12&path=/tmp",
    ] {
        let reply = statistics_json(uri.parse().unwrap()).await;
        assert_eq!(reply.0, StatusCode::BAD_REQUEST);
        let body: Value = serde_json::from_str(&reply.2).unwrap();
        assert_eq!(body["status"], "refused");
        assert_eq!(body["rows"], json!([]));
    }
    let reply = admission_json(
        format!("/boolean-admission.json?identity={ID}&completion={ID}&kind=splits")
            .parse()
            .unwrap(),
    )
    .await;
    assert_eq!(reply.0, StatusCode::BAD_REQUEST);
    let reply = qualification_json(
        format!("/boolean-qualification.json?identity={ID}&offset=1")
            .parse()
            .unwrap(),
    )
    .await;
    assert_eq!(reply.0, StatusCode::BAD_REQUEST);
}

#[test]
fn missing_saved_receipt_names_configured_root_without_creating_or_searching_it() {
    let root = crate::scratch::path("boolean-observer-no-evidence");
    for model in [Model::Statistics, Model::Admission, Model::Qualification] {
        let asked = Asked::parse(&format!("identity={ID}"), model).unwrap();
        let why = render(&root, &asked).unwrap_err();
        assert!(why.contains(&root.display().to_string()));
        assert!(why.contains("OUTPUT_ROOT"));
        assert!(why.contains("no other folder searched"));
        assert!(!root.exists());
    }
}

#[test]
fn every_evidence_model_names_the_exact_server_owned_budget_on_authentication_refusal() {
    let root = crate::scratch::path("boolean-observer-exact-budget");
    let budget = crate::detail::BooleanObservationBudget::from_value(Some(std::ffi::OsStr::new(
        "1610612736",
    )))
    .unwrap();
    for model in [Model::Statistics, Model::Admission, Model::Qualification] {
        let asked = Asked::parse(&format!("identity={ID}"), model).unwrap();
        let why = render_with_budget(&root, &asked, budget).unwrap_err();
        assert!(
            why.contains("1610612736 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES")
        );
        assert!(why.contains(model.name()) && why.contains("no body prefix returned"));
    }
    assert!(!root.exists());
}

#[test]
fn daily_and_weekly_pages_require_exact_setting_parent_pin_and_bounded_period_selector() {
    for kind in ["index-days", "index-weeks"] {
        for period in ["full", "training", "later"] {
            let query = format!(
                "identity={ID}&completion={ID}&kind={kind}&setting=7&period={period}&offset=2&limit=256"
            );
            let asked = Asked::parse(&query, Model::Qualification).unwrap();
            assert_eq!(asked.setting, Some(7));
            assert_eq!(asked.period, period);
            assert_eq!(asked.offset, 2);
            assert_eq!(asked.limit, 256);
            assert!(Asked::parse(&query, Model::Admission).is_err());
        }
        for invalid in [
            format!("identity={ID}&kind={kind}&setting=0"),
            format!("identity={ID}&completion={ID}&kind={kind}"),
            format!("identity={ID}&completion={ID}&kind={kind}&setting=0&period=unknown"),
            format!("identity={ID}&completion={ID}&kind={kind}&setting=0&setting=1"),
            format!("identity={ID}&completion={ID}&kind={kind}&setting=0&limit=257"),
        ] {
            assert!(
                Asked::parse(&invalid, Model::Qualification).is_err(),
                "{invalid}"
            );
        }
    }
    assert!(Asked::parse(&format!("identity={ID}&setting=0"), Model::Qualification).is_err());
    assert!(Asked::parse(&format!("identity={ID}&period=full"), Model::Qualification).is_err());
}

/// Whether `call` is a statement at the top level of the function `body`, as
/// rustfmt indents one, and every `return` written before it returns exactly
/// an `Err`: a `return Err(` whose expression ends where that call's
/// parenthesis closes.
///
/// Before the call, a `?` can only return an error, so a page can leave the
/// function before the call only by a `return`, and every such `return` is
/// refused, however it is spelled. A call under a condition, a loop or a
/// closure is indented deeper and is not found. A `return` inside a closure
/// before the call is refused too, though it leaves only the closure. A word
/// inside a `"` string or after `//` is not code and is not read. The text
/// after the call is not read, so a page built after it is not held to the
/// call.
pub(crate) fn on_every_page(body: &str, call: &str) -> bool {
    body.split_once(&format!("\n    {call}"))
        .is_some_and(|(before, _)| only_errors_return(&code_only(before)))
}

/// `code` with the contents of every `"` string and every `//` comment
/// removed. A `'"'` character literal would be read as opening a string; the
/// bodies this is applied to hold none.
fn code_only(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let mut chars = code.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            out.push('"');
            while let Some(inner) = chars.next() {
                match inner {
                    '\\' => {
                        chars.next();
                    }
                    '"' => break,
                    _ => {}
                }
            }
            out.push('"');
        } else if c == '/' && chars.peek() == Some(&'/') {
            for rest in chars.by_ref() {
                if rest == '\n' {
                    out.push('\n');
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Whether every `return` keyword in `code` returns exactly an `Err`: it is
/// written `return Err(`, and its expression ends where that call's
/// parenthesis closes, at a `;`, a `,` or a `}` after any whitespace. So
/// `return Err(why).or_else(|_| page)`, which returns the page, is refused. A
/// word such as `returned` is no keyword. The parentheses are counted in the
/// text [`code_only`] leaves, so one inside a string is not counted, and one
/// in a character literal would be; the bodies this is applied to hold none.
fn only_errors_return(code: &str) -> bool {
    let part_of_a_word =
        |byte: Option<&u8>| byte.is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_');
    code.match_indices("return").all(|(at, _)| {
        let bytes = code.as_bytes();
        let keyword = !part_of_a_word(at.checked_sub(1).and_then(|before| bytes.get(before)))
            && !part_of_a_word(bytes.get(at + "return".len()));
        !keyword
            || code[at..]
                .strip_prefix("return Err(")
                .is_some_and(ends_where_it_closes)
    })
}

/// Whether `rest`, the text after an opening `(`, closes that parenthesis and
/// then, past any whitespace, ends the expression with a `;`, a `,` or a `}`.
fn ends_where_it_closes(rest: &str) -> bool {
    let mut open = 1_usize;
    for (at, c) in rest.char_indices() {
        match c {
            '(' => open += 1,
            ')' if open == 1 => {
                return rest[at + 1..].trim_start().starts_with([';', ',', '}']);
            }
            ')' => open -= 1,
            _ => {}
        }
    }
    false
}

/// **THE SOURCE RULE REFUSES A PAGE RETURNED BEFORE THE CALL, HOWEVER ITS
/// `return` IS SPELLED, AND ADMITS AN EARLY ERROR.** D-0694, AF-19.
///
/// The rule once refused only the letters `return Ok`, and a
/// `render_with_budget` that returned `statistics(reader, asked)` before its
/// call on a read with a completion pin passed the test below, which applies
/// the rule to that function, while every such page went out without the
/// note.
#[test]
fn the_source_rule_refuses_every_early_return_but_an_error() {
    const CALL: &str = "crate::detail::put_equity_note(&mut body, note)?;";
    let body =
        |before: &str| format!("\n    let mut body = page()?;{before}\n    {CALL}\n    Ok(body)");
    for early in [
        "\n    if pinned {\n        return statistics(reader, asked);\n    }",
        "\n    if pinned {\n        return Ok(body);\n    }",
        "\n    let body = match cached {\n        Some(page) => return page,\n        None => body,\n    };",
        "\n    let said = \"a \\\"quoted\\\" word\";\n    if pinned {\n        return page;\n    }",
        "\n    let link = \"http://host\";\n    if pinned {\n        return page;\n    }",
        "\n    if pinned {\n        return Err(String::new()).or_else(|_| statistics(reader, asked));\n    }",
        "\n    if pinned {\n        return Err(why)\n            .or_else(|_| statistics(reader, asked));\n    }",
    ] {
        assert!(
            !on_every_page(&body(early), CALL),
            "a page returned before the call: {early}"
        );
    }
    for admitted in [
        "",
        "\n    if stale {\n        return Err(\"changed\".into());\n    }",
        "\n    if stale {\n        return Err(format!(\"changed ({})\", body.len()));\n    }",
        "\n    let body = match cached {\n        Err(why) => return Err(why),\n        Ok(page) => page,\n    };",
        "\n    let body = match cached {\n        Err(why) => { return Err(why) }\n        Ok(page) => page,\n    };",
        "\n    let returned = body.len();",
        "\n    let scope = \"no future-return guarantee\";",
        "\n    // return the page only once it carries the note",
    ] {
        assert!(
            on_every_page(&body(admitted), CALL),
            "nothing returns a page before the call: {admitted}"
        );
    }
    assert!(
        !on_every_page(&format!("\n    if pinned {{\n        {CALL}\n    }}"), CALL),
        "a call under a condition is not a statement of the body"
    );
}

/// **A statistics, admission or qualification page over a stock family
/// states the equity note; a page over indices does not.** D-0694, AF-19.
///
/// Each of the three pages serves a candidate's trades, wins and returns
/// from saved statistics whose linked sources name their families. The page
/// carries `cli::research_equity_note` over those sources, the Boolean
/// research heading's own decision, so one cash source is enough and a page
/// of index sources gains no key. The three readers are saved evidence this
/// crate has no fixture for, so no page of theirs is rendered here with a
/// stock family: `render_with_budget` is held to putting the note in, over
/// each reader's own sources, by its source. The call must be a statement at
/// the top level of its body, and every `return` before it must return an
/// `Err`, so no page can be returned before the call is made. A call made
/// only when a completion was asked for is indented under its condition and
/// fails here; the same call passed when this checked only that the line was
/// present.
#[test]
fn a_page_over_a_stock_source_states_the_equity_note_and_an_index_page_does_not() {
    let family = |key: Result<brutex_core::instrument::InstrumentKey, _>| {
        cli::boolean_observation::ResearchFamilyV1::new(key.unwrap()).unwrap()
    };
    let nse = brutex_core::instrument::Exchange::Nse;
    let source = |family| StatisticsSource {
        family,
        identity: [1; 32],
        completion: [2; 32],
        coordinates: 2,
    };
    let nifty = source(family(brutex_core::instrument::InstrumentKey::index(
        nse, "NIFTY",
    )));
    let bank = source(family(brutex_core::instrument::InstrumentKey::index(
        nse,
        "BANKNIFTY",
    )));
    let reliance = source(family(brutex_core::instrument::InstrumentKey::cash(
        nse, "RELIANCE",
    )));
    let note = cli::research_equity_note([reliance.family]);
    assert!(
        note.contains("CORPORATE ACTIONS ARE UNCHECKED"),
        "premise: {note}"
    );
    assert_eq!(sources_note(&[nifty, bank]), "");
    assert_eq!(sources_note(&[]), "");
    assert_eq!(sources_note(&[nifty, reliance]), note);

    let text = include_str!("booleanevidencejson.rs");
    let body = |name: &str| {
        text.split_once(&format!("\nfn {name}("))
            .and_then(|(_, tail)| tail.split_once("\n}\n"))
            .map(|(body, _)| body)
            .unwrap()
    };
    assert!(
        on_every_page(
            body("render_with_budget"),
            "crate::detail::put_equity_note(&mut body, equity_note(reader))?;"
        ),
        "every page of the three models carries the note"
    );
    let note_of = body("equity_note");
    for arm in [
        "Reader::Statistics(reader) => reader.sources(),",
        "Reader::Admission(reader) => reader.statistics().sources(),",
        "Reader::Qualification(reader) => reader.original().statistics().sources(),",
    ] {
        assert!(note_of.contains(arm), "{arm}: each model's own sources");
    }
}

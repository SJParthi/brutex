#![cfg(test)]
//! Finite wire fixtures test observation only; these bytes are not market proof.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "bounded independent wire fixtures and assertions"
)]
use super::*;
use brutex_core::blake3::hash;
use std::fs;

const ID: &str = "4242424242424242424242424242424242424242424242424242424242424242";
static CACHE_TEST: Mutex<()> = Mutex::new(());
fn words(out: &mut Vec<u8>, values: &[u64]) {
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
}
pub(crate) fn fixture(name: &str) -> PathBuf {
    fixture_for(
        name,
        brutex_core::instrument::InstrumentKey::index(
            brutex_core::instrument::Exchange::Nse,
            "NIFTY",
        )
        .unwrap(),
    )
}
/// [`fixture`] over `key`'s family, every other byte the same.
pub(crate) fn fixture_for(name: &str, key: brutex_core::instrument::InstrumentKey) -> PathBuf {
    let root = crate::scratch::path(name);
    fs::create_dir_all(&root).unwrap();
    let directory = root.join("boolean-candidates-v1").join(ID);
    fs::create_dir_all(&directory).unwrap();
    let family = cli::boolean_observation::ResearchFamilyV1::new(key).unwrap();
    let program = vocab::expression::Expression::parse("0 | !1").unwrap();
    let mut body = b"BRBOOL01".to_vec();
    body.extend_from_slice(&[0x42; 32]);
    body.extend_from_slice(&[0x19; 32]);
    body.extend_from_slice(&family.encode());
    words(&mut body, &[1, 1, 2]);
    body.extend_from_slice(&program.encode());
    words(&mut body, &[0]);
    for side in 0..2 {
        body.extend_from_slice(&[1; 160]);
        words(&mut body, &[10, 1, 60_000_001, 1, 5, 60, 0, side]);
        for _ in 0..3 {
            words(&mut body, &[1, 1, 2]);
        }
        words(&mut body, &[1, 100, 10000, 100, 100, 0]);
        body.extend_from_slice(&[7; 32]);
        words(&mut body, &[0, 0, 100, 100]);
        for _ in 0..3 {
            words(&mut body, &[1, 1000]);
        }
    }
    for side in 0..2 {
        body.extend_from_slice(&[u8::try_from(side + 1).unwrap(); 32]);
        words(&mut body, &[0]);
        body.extend_from_slice(&[3; 32]);
        words(&mut body, &[side, 0]);
        // One 30-word measured cell: same exact trade on both sides for this
        // wire-contract fixture; it does not claim a priced market producer.
        let mut cell = vec![0_u64; 30];
        cell[..5].fill(u64::MAX);
        cell[5] = 1;
        cell[6] = 1;
        cell[7] = 10;
        cell[8] = 12;
        cell[14] = 1;
        words(&mut body, &cell);
        words(&mut body, &[3, 1, 1, 1, 10, 1, 4, 5]); // refusal, periods, trades, support, four truth counts
        words(&mut body, &[0, 10, 1, 1]);
        words(&mut body, &[0, 1, 2, 12, 10, 1, 60_000_001, 0, 0, 2, 2]);
    }
    let mut receipt = b"BRBLCM01".to_vec();
    receipt.extend_from_slice(&[0x42; 32]);
    receipt.extend_from_slice(&hash(&body));
    words(&mut receipt, &[body.len() as u64]);
    receipt.extend_from_slice(&hash(&receipt));
    fs::write(directory.join("owner.lock"), []).unwrap();
    fs::write(directory.join("body.bin"), body).unwrap();
    fs::write(directory.join("complete.bin"), receipt).unwrap();
    root
}

#[test]
fn canonical_selectors_and_pins_refuse_cross_page_fallback() {
    assert!(Asked::parse(&format!("identity={ID}&limit=256")).is_ok());
    for suffix in [
        "&offset=1",
        "&offset=01",
        "&limit=0",
        "&limit=257",
        "&candidate=0",
        "&kind=trades",
        "&kind=grid&side=long&axis=stop",
        "&side=long",
        "&unknown=1",
        "&identity=bad",
        "&completion=bad",
        "&offset=18446744073709551616",
    ] {
        assert!(
            Asked::parse(&format!("identity={ID}{suffix}")).is_err(),
            "{suffix}"
        );
    }
    assert!(
        Asked::parse(&format!(
            "identity={ID}&completion={ID}&kind=sessions&candidate=9007199254740993"
        ))
        .is_ok()
    );
    assert!(Asked::parse(&"x".repeat(513)).is_err());
}

/// expr-3, D-2576: the catalog's cold open runs with the cache's mutex FREE,
/// and a warm page after it is still served from the installed reader (no
/// second cold admission). On the old `render_with_budget` the guard was held
/// across `Reader::open`, so the probe at the open point recorded `false`.
#[test]
fn a_cold_catalog_open_does_not_hold_the_cache_lock() {
    let _cache = CACHE_TEST.lock().unwrap();
    let root = fixture("boolean-api-cold-unlocked");
    let asked = Asked::parse(&format!("identity={ID}&limit=1")).unwrap();
    crate::detail::SLOT_FREE_AT_OPEN.with(|cell| cell.set(None));
    // A different root forces a cold admission whatever an earlier test cached.
    let missing = crate::scratch::path("boolean-api-cold-unlocked-absent");
    assert!(render(&missing, &asked).is_err());
    assert_eq!(
        crate::detail::SLOT_FREE_AT_OPEN.with(std::cell::Cell::get),
        Some(true),
        "the catalog was opened with the cache locked"
    );
    let cold = COLD_ADMISSIONS.with(std::cell::Cell::get);
    let first = render(&root, &asked).unwrap();
    assert_eq!(COLD_ADMISSIONS.with(std::cell::Cell::get), cold + 1);
    let pin = first["completion"].as_str().unwrap().to_owned();
    let warm = render(
        &root,
        &Asked::parse(&format!("identity={ID}&completion={pin}&limit=1")).unwrap(),
    )
    .unwrap();
    assert_eq!(COLD_ADMISSIONS.with(std::cell::Cell::get), cold + 1, "the warm page reopened");
    assert_eq!(warm["rows"], first["rows"]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn authenticated_catalog_projects_exact_program_coordinate_trade_session_and_grid_pages() {
    let _cache = CACHE_TEST.lock().unwrap();
    let root = fixture("boolean-api-projections");
    let first = render(
        &root,
        &Asked::parse(&format!("identity={ID}&limit=1")).unwrap(),
    )
    .unwrap();
    assert_eq!(first["rows"][0]["expression"], "(0 | !(1))");
    assert_eq!(first["next"], "1");
    assert_eq!(first["coordinate_count"], "2");
    assert_eq!(
        first["rows"][0]["execution_refusals"],
        json!(["Missing stop", "Missing target"])
    );
    let pin = first["completion"].as_str().unwrap();
    for (suffix, total) in [
        ("kind=programs", 1),
        ("kind=trades&candidate=0", 1),
        ("kind=sessions&candidate=0", 1),
        ("kind=grid&side=short&axis=stop", 1),
        ("kind=grid&side=long&axis=requested-target", 1),
        ("offset=1&limit=1", 2),
    ] {
        let page = render(
            &root,
            &Asked::parse(&format!("identity={ID}&completion={pin}&{suffix}")).unwrap(),
        )
        .unwrap();
        assert_eq!(page["total"], total.to_string());
        assert_eq!(page["rows"].as_array().unwrap().len(), 1);
        assert_eq!(page["next"], Value::Null);
        if suffix.starts_with("kind=trades") {
            assert_eq!(page["rows"][0]["worst"], "10");
            assert_eq!(page["selected"]["index"], "0");
        }
        if suffix.starts_with("kind=sessions") {
            assert_eq!(page["rows"][0]["return_paisa"], "10");
        }
        if suffix.contains("axis=requested") {
            assert_eq!(page["rows"][0]["denominator"], "2");
        }
    }
    assert!(
        render(
            &root,
            &Asked::parse(&format!("identity={ID}&completion={ID}&offset=1")).unwrap()
        )
        .is_err()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_smaller_server_budget_cannot_reuse_a_previously_admitted_catalog() {
    let _cache = CACHE_TEST.lock().unwrap();
    let root = fixture("boolean-api-budget-cache");
    let admitted = crate::detail::BooleanObservationBudget::from_value(None).unwrap();
    let tiny = crate::detail::BooleanObservationBudget::from_value(Some(std::ffi::OsStr::new("1")))
        .unwrap();
    let first = render_with_budget(
        &root,
        &Asked::parse(&format!("identity={ID}")).unwrap(),
        admitted,
    )
    .unwrap();
    assert_eq!(first["observation_byte_limit"], "67108864");
    let asked = Asked::parse(&format!(
        "identity={ID}&completion={}",
        first["completion"].as_str().unwrap()
    ))
    .unwrap();
    let why = render_with_budget(&root, &asked, tiny).unwrap_err();
    assert!(why.contains("1 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES"));
    let reopened = render_with_budget(&root, &asked, admitted).unwrap();
    assert_eq!(reopened["completion"], first["completion"]);
    assert_eq!(reopened["rows"], first["rows"]);
    fs::write(
        root.join("boolean-candidates-v1")
            .join(ID)
            .join("complete.bin"),
        [],
    )
    .unwrap();
    assert!(
        render_with_budget(&root, &asked, admitted).is_err(),
        "a sufficient budget does not forgive receipt corruption"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_busy_replaced_or_truncated_evidence_never_returns_saved_empty() {
    let _cache = CACHE_TEST.lock().unwrap();
    let missing = crate::scratch::path("boolean-api-missing");
    assert!(render(&missing, &Asked::parse(&format!("identity={ID}")).unwrap()).is_err());
    let root = fixture("boolean-api-refusals");
    let reader = Reader::open(&root, [0x42; 32], crate::detail::MAX_SCAN_BYTES).unwrap();
    let directory = root.join("boolean-candidates-v1").join(ID);
    let owner = std::fs::File::open(directory.join("owner.lock")).unwrap();
    owner.try_lock().unwrap();
    assert!(project(&reader, &Asked::parse(&format!("identity={ID}")).unwrap()).is_err());
    owner.unlock().unwrap();
    assert!(project(&reader, &Asked::parse(&format!("identity={ID}")).unwrap()).is_ok());
    fs::write(directory.join("body.bin"), b"changed").unwrap();
    assert!(project(&reader, &Asked::parse(&format!("identity={ID}")).unwrap()).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn exact_integer_projection_and_page_extents_are_not_rounded_or_truncated() {
    let row = TradeRow {
        signal_bar: 9_007_199_254_740_993,
        entry_bar: 1,
        exit_bar: 2,
        best: i64::MAX,
        worst: i64::MIN,
        entry_micros: 1,
        exit_micros: 2,
        adverse: 0,
        adverse_paisa: 0,
        favourable: 1,
        favourable_paisa: 1,
    };
    let value = trade(0, row);
    assert_eq!(value["signal_bar"], "9007199254740993");
    assert_eq!(value["worst"], i64::MIN.to_string());
    assert_eq!(next_offset(0, 0, 32, 0).unwrap(), None);
    assert_eq!(next_offset(257, 0, 256, 256).unwrap(), Some(256));
    assert!(next_offset(257, 0, 256, 255).is_err());
    assert!(next_offset(257, 258, 1, 0).is_err());
    assert!(refusal_names(64).is_err());
}

#[test]
fn pinned_cache_refuses_replacement_while_an_explicit_new_read_authenticates_again() {
    let _cache = CACHE_TEST.lock().unwrap();
    let root = fixture("boolean-api-cache-refresh");
    let first = Asked::parse(&format!("identity={ID}")).unwrap();
    let body = render(&root, &first).unwrap();
    let pin = body["completion"].as_str().unwrap();
    let pinned = Asked::parse(&format!("identity={ID}&completion={pin}")).unwrap();
    let path = root.join("boolean-candidates-v1").join(ID).join("body.bin");
    let replacement = path.with_extension("replacement");
    fs::copy(&path, &replacement).unwrap();
    fs::rename(replacement, &path).unwrap();
    assert!(render(&root, &pinned).is_err());
    assert_eq!(render(&root, &first).unwrap()["completion"], pin);
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn public_handler_refuses_bad_selectors_before_reading_any_root() {
    let (status, _, body) =
        boolean_json(Uri::from_static("/boolean-candidates.json?identity=bad")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["status"],
        "refused"
    );
}

/// The page the catalog under `root` serves for `suffix`, pinned to that
/// catalog's own completion after the first page.
fn page_of(root: &Path, suffix: &str) -> Value {
    let first = render(root, &Asked::parse(&format!("identity={ID}")).unwrap()).unwrap();
    if suffix.is_empty() {
        return first;
    }
    let pin = first["completion"].as_str().unwrap();
    render(
        root,
        &Asked::parse(&format!("identity={ID}&completion={pin}&{suffix}")).unwrap(),
    )
    .unwrap()
}

/// **A stock family's catalog page says what its figures are made of; an
/// index family's is the page it was.** D-0694, AF-19.
///
/// A catalog page serves per-coordinate trades, wins and pessimistic and
/// optimistic totals, per-trade best and worst, and per-session returns. The
/// text heading over the same research states `cli::research_equity_note`
/// for a scope holding a cash stock, and the JSON carried only `"cash":true`.
/// Two catalogs identical but for the family are compared on every page
/// kind: the stock's carries the note, the index's has no such key, and
/// with the note and the family's own fields set aside the two pages are
/// equal, so the note is the only thing the stock's page adds.
#[test]
fn a_stock_catalog_carries_the_equity_note_and_an_index_catalog_does_not() {
    let _cache = CACHE_TEST.lock().unwrap();
    let index = fixture("boolean-api-equity-index");
    let stock = fixture_for(
        "boolean-api-equity-stock",
        brutex_core::instrument::InstrumentKey::cash(
            brutex_core::instrument::Exchange::Nse,
            "RELIANCE",
        )
        .unwrap(),
    );
    let family = cli::boolean_observation::ResearchFamilyV1::new(
        brutex_core::instrument::InstrumentKey::cash(
            brutex_core::instrument::Exchange::Nse,
            "RELIANCE",
        )
        .unwrap(),
    )
    .unwrap();
    let note = cli::research_equity_note([family]);
    assert_eq!(
        note,
        cli::equity_note_for("RELIANCE"),
        "one wording for a stock, whichever payload carries it"
    );
    for part in ["GROSS OF EVERY CHARGE", "CORPORATE ACTIONS ARE UNCHECKED"] {
        assert!(note.contains(part), "premise, {part}: {note}");
    }
    for suffix in [
        "",
        "kind=programs",
        "kind=trades&candidate=0",
        "kind=sessions&candidate=0",
        "kind=grid&side=short&axis=stop",
    ] {
        let mut index_page = page_of(&index, suffix);
        let mut stock_page = page_of(&stock, suffix);
        assert_eq!(stock_page["cash"], true, "premise: {suffix}");
        assert_eq!(index_page["cash"], false, "premise: {suffix}");
        assert_eq!(
            stock_page.get("equity_note"),
            Some(&json!(note)),
            "{suffix:?}: a stock family's page"
        );
        assert_eq!(
            index_page.get("equity_note"),
            None,
            "{suffix:?}: an index family's page"
        );
        for page in [&mut index_page, &mut stock_page] {
            let object = page.as_object_mut().unwrap();
            for own in [
                "equity_note",
                "instrument",
                "cash",
                "membership_digest",
                "completion",
            ] {
                object.remove(own);
            }
        }
        assert_eq!(
            stock_page, index_page,
            "{suffix:?}: the note is the only thing a stock's page adds"
        );
    }
    fs::remove_dir_all(index).unwrap();
    fs::remove_dir_all(stock).unwrap();
}

/// Cold authentications this thread has run so far.
fn cold() -> usize {
    COLD_ADMISSIONS.with(std::cell::Cell::get)
}

/// **A first page with no completion pin reuses a held catalog that is still
/// current, and authenticates afresh only when the key differs or the held
/// files changed.** W1-api1-5, D-1444.
///
/// Until D-1444 every request without `completion` -- every first page --
/// dropped the cached reader and hashed and decoded the whole body again,
/// under the one process-wide mutex. Here 64 first pages in a row cost one
/// cold authentication and are byte-identical. Then each way the held reader
/// can stop being the right one is tried, and each must reopen or refuse
/// rather than serve the held copy: the body renamed over with the same
/// bytes (a new generation, reopened and served), the receipt emptied
/// (refused, pinned or not), a busy publisher (refused, not served from the
/// cache), a smaller budget, and a second root alternating with the first,
/// which is the single slot's stated cost: every switch is cold.
#[test]
fn an_unpinned_first_page_reuses_a_current_catalog_and_reopens_only_on_change() {
    let _cache = CACHE_TEST.lock().unwrap();
    let root = fixture("boolean-api-unpinned-warm");
    let other = fixture("boolean-api-unpinned-warm-other");
    let first = Asked::parse(&format!("identity={ID}")).unwrap();
    // A different root is a different key, so the slot starts cold whatever
    // an earlier test left in it.
    let start = cold();
    let opening = render(&root, &first).unwrap();
    assert_eq!(cold(), start + 1, "the first page of a new key is cold");
    for n in 0..64 {
        let again = render(&root, &first).unwrap();
        assert_eq!(again, opening, "page {n} is the same page, byte for byte");
    }
    assert_eq!(
        cold(),
        start + 1,
        "64 unpinned first pages over an unchanged catalog cost no second authentication"
    );
    let pin = opening["completion"].as_str().unwrap();
    let pinned = Asked::parse(&format!("identity={ID}&completion={pin}&kind=programs")).unwrap();
    assert_eq!(render(&root, &pinned).unwrap()["total"], "1");
    assert_eq!(cold(), start + 1, "a pinned later page stays warm too");

    // A busy publisher: the held reader cannot prove it is current, so the
    // page is refused rather than served from memory.
    let directory = root.join("boolean-candidates-v1").join(ID);
    let owner = fs::File::open(directory.join("owner.lock")).unwrap();
    owner.try_lock().unwrap();
    assert!(render(&root, &first).is_err(), "busy owner, unpinned");
    owner.unlock().unwrap();
    let after_busy = cold();
    assert_eq!(render(&root, &first).unwrap(), opening);

    // The body renamed over with identical bytes: a new generation. The
    // unpinned page reopens and serves the same completion.
    let path = directory.join("body.bin");
    let replacement = path.with_extension("replacement");
    fs::copy(&path, &replacement).unwrap();
    fs::rename(replacement, &path).unwrap();
    let before = cold();
    assert_eq!(render(&root, &first).unwrap()["completion"], pin);
    assert_eq!(
        cold(),
        before + 1,
        "a changed generation authenticates again"
    );
    assert!(after_busy <= before);

    // Alternating roots: the single slot makes every switch cold. That is the
    // bound docs/06-limits.md states, not a defect this change removes.
    let before = cold();
    for _ in 0..3 {
        render(&other, &first).unwrap();
        render(&root, &first).unwrap();
    }
    assert_eq!(
        cold(),
        before + 6,
        "each alternation is one cold authentication"
    );

    // A smaller budget is a different key and is refused, never served warm.
    let tiny = crate::detail::BooleanObservationBudget::from_value(Some(std::ffi::OsStr::new("1")))
        .unwrap();
    assert!(render_with_budget(&root, &first, tiny).is_err());

    // A corrupt receipt: neither an unpinned nor a pinned page is served
    // from the held reader.
    render(&root, &first).unwrap();
    fs::write(directory.join("complete.bin"), []).unwrap();
    assert!(
        render(&root, &first).is_err(),
        "unpinned over an empty receipt"
    );
    assert!(
        render(&root, &pinned).is_err(),
        "pinned over an empty receipt"
    );
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(other).unwrap();
}

/// `docs/06-limits.md`'s D-1444 section, which states the JSON renderers'
/// cold and per-request costs, with its line wrapping collapsed to single
/// spaces so a phrase is found wherever the paragraph happens to break.
pub(crate) fn d0951_limits() -> String {
    let limits = include_str!("../../../docs/06-limits.md");
    let start = limits.find("\n## JSON renderers: cold admissions and per-request walks — D-1444");
    assert!(
        start.is_some(),
        "docs/06-limits.md carries the D-1444 section"
    );
    let rest = &limits[start.unwrap_or(limits.len()) + 1..];
    let section = rest.find("\n## ").map_or(rest, |end| &rest[..end]);
    section.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The bullet of the D-1444 section that names `id`, collapsed as
/// [`d0951_limits`] is.
pub(crate) fn d0951_bullet(id: &str) -> String {
    let section = d0951_limits();
    let bullet = section
        .split(" - **")
        .skip(1)
        .find(|bullet| bullet.contains(&format!("({id})")));
    assert!(
        bullet.is_some(),
        "the D-1444 section has a bullet for {id}: {section}"
    );
    bullet.unwrap_or_default().to_owned()
}

/// **What an unpinned first page still costs is written where the limits
/// live, and the code it describes is the code that runs.** W1-api1-5, D-1444.
#[test]
fn the_cold_admission_bound_is_stated_and_both_renderers_take_the_shared_decision() {
    let bullet = d0951_bullet("W1-api1-5");
    for word in [
        "booleanjson::render_with_budget",
        "booleanevidencejson::render_with_budget",
        "detail::must_admit",
        "require_current",
        "O(saved body bytes)",
        "ONE identity",
        "booleanoosjson",
        "indexstopjson",
    ] {
        assert!(bullet.contains(word), "the bullet names {word}: {bullet}");
    }
    assert!(d0951_limits().contains("UNVERIFIED as a measurement"));
    for (file, source) in [
        ("booleanjson.rs", include_str!("booleanjson.rs")),
        (
            "booleanevidencejson.rs",
            include_str!("booleanevidencejson.rs"),
        ),
    ] {
        let found = source.split_once("\nfn render_with_budget(");
        assert!(found.is_some(), "{file} has render_with_budget");
        let body = found.unwrap_or_default().1;
        let body = &body[..body.find("\n}\n").unwrap()];
        assert!(
            body.contains("crate::detail::must_admit(held.is_some(), asked.completion.is_some(),"),
            "{file} decides through must_admit: {body}"
        );
        assert!(
            !body.contains("asked.completion.is_none()"),
            "{file} no longer reopens on every unpinned page: {body}"
        );
    }
}

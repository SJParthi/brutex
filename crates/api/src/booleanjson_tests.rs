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

/// **EVERY SELECTOR A CATALOG PAGE CANNOT TAKE IS REFUSED BY ITS OWN WORDS**
/// (BASE-COV-API-AS2). Extremes: (1) a page kind nobody names; (2) a side
/// that is neither long nor short; (3) a grid axis nobody names; (4) a grid
/// page with no side; (5) a grid page with a side and no axis; (6) a
/// coordinates page that names an axis; (7) the exact selectors a grid page
/// takes, with its pin, are read; (8) every refusal names the selector, never
/// a fallback page. Combined: (9) a grid page with neither side nor axis:
/// the side is asked for first; (10) an unknown kind AND an unknown side: the
/// kind is read first. Out of the box: (11) a side in another case, `LONG`,
/// is no side.
#[test]
fn every_catalog_selector_it_cannot_take_is_refused_by_its_own_words() {
    let selectors = "catalog page requires exact completion, selectors and limit 1..=256";
    for (suffix, said) in [
        ("&kind=everything", "unknown catalog page kind"),
        ("&side=up", "side must be long or short"),
        ("&side=LONG", "side must be long or short"),
        ("&axis=sideways", "unknown saved grid axis"),
        ("&kind=grid&axis=stop", selectors),
        ("&kind=grid&side=long", selectors),
        ("&kind=grid", selectors),
        ("&axis=target", selectors),
        ("&kind=everything&side=up", "unknown catalog page kind"),
    ] {
        assert_eq!(
            Asked::parse(&format!("identity={ID}&completion={ID}{suffix}"))
                .err()
                .as_deref(),
            Some(said),
            "{suffix}"
        );
    }
    assert!(
        Asked::parse(&format!(
            "identity={ID}&completion={ID}&kind=grid&side=short&axis=requested-trail"
        ))
        .is_ok()
    );
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
/// An [`Asked`] the parser never builds: a page selector handed straight to
/// the projection, for the projection's own refusals.
fn unparsed(kind: &str, side: Option<Side>, axis: Option<&str>, offset: usize) -> Asked {
    Asked {
        identity: [0x42; 32],
        completion: None,
        kind: kind.to_owned(),
        candidate: None,
        side,
        axis: axis.map(str::to_owned),
        offset,
        limit: 1,
    }
}

/// **EVERY SAVED GRID AXIS IS PAGED, AND A KIND, AXIS OR RUNG NO CATALOG HOLDS
/// IS REFUSED BY ITS OWN WORDS** (BASE-COV-API-AS2): the requested trail ladder
/// and the resolved target and trail ladders are served as pages; a page kind,
/// a requested axis or a resolved axis the parser never hands the projection
/// is refused by name when the projection is called with it; a coordinate's
/// selected rung is read off its side's ladder and a rung past it is refused;
/// and a second identity asked under the root whose catalog is held is a cold
/// admission of its own, never the held one.
/// Extremes: 12 (8 single, 2 combined, 2 out of the box): (1) the long grid's
/// requested trail ladder pages its one rung, 1/2; (2) the short grid's
/// resolved target ladder pages 1000 ppm; (3) the long grid's resolved trail
/// ladder likewise; (4) a page kind no parser admits is "unsupported catalog
/// page"; (5) a requested axis outside stop, target and trail is "saved
/// requested axis missing"; (6) a resolved one is "saved resolved axis
/// missing"; (7) a selected stop rung 0 reads the ladder's 1000 ppm and every
/// unselected axis reads null; (8) a target rung past the one-rung ladder is
/// "coordinate ladder index missing". Combined: (9) a stop rung that resolves
/// AND a target rung past its ladder in one cell: the cell is refused whole,
/// never half read; (10) a second identity under the held root: a cold
/// admission, refused naming that identity and the root, and the held catalog
/// asked again is opened again and is the same page. Out of the box:
/// (11) `usize::MAX` as a rung is the same refusal, never a wrap; (12) an
/// offset past a one-rung resolved ladder is "catalog page outside saved
/// extent".
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one test walks every case its Extremes: line names, in order"
)]
fn every_saved_axis_is_paged_and_a_kind_axis_or_rung_no_catalog_holds_is_refused_by_name() {
    let _cache = CACHE_TEST.lock().unwrap();
    let root = fixture("boolean-api-axes");
    // (1), (2), (3): the three axes no earlier test pages.
    for (suffix, field, value) in [
        (
            "kind=grid&side=long&axis=requested-trail",
            "denominator",
            "2",
        ),
        ("kind=grid&side=short&axis=target", "ppm", "1000"),
        ("kind=grid&side=long&axis=trail", "ppm", "1000"),
    ] {
        let served = page_of(&root, suffix);
        assert_eq!(served["total"], "1", "{suffix}");
        assert_eq!(served["rows"][0]["index"], "0", "{suffix}");
        assert_eq!(served["rows"][0][field], value, "{suffix}");
    }
    let reader = Reader::open(&root, [0x42; 32], crate::detail::MAX_SCAN_BYTES).unwrap();
    let pin = reader.completion_digest();
    // (4): a page kind no parser admits.
    assert_eq!(
        page(&reader, &unparsed("everything", None, None, 0), pin)
            .err()
            .as_deref(),
        Some("unsupported catalog page")
    );
    // (5), (6), (12): axes and an extent no saved grid holds.
    for (side, axis, offset, said) in [
        (
            Side::Long,
            "requested-sideways",
            0,
            "saved requested axis missing",
        ),
        (Side::Short, "sideways", 0, "saved resolved axis missing"),
        (Side::Short, "stop", 2, "catalog page outside saved extent"),
    ] {
        assert_eq!(
            grid_page(&reader, &unparsed("grid", Some(side), Some(axis), offset))
                .err()
                .as_deref(),
            Some(said),
            "{axis} at {offset}"
        );
    }
    // (7), (8), (9), (11): a coordinate's selected rungs.
    let long = &reader.grids()[0];
    let levels = selected_levels(
        &Cell {
            stop: Some(0),
            ..Cell::default()
        },
        long,
    )
    .unwrap();
    assert_eq!(levels["stop_ppm"], "1000");
    for unselected in ["target_ppm", "tsl_ppm", "ttp_arm_ppm", "ttp_trail_ppm"] {
        assert_eq!(levels[unselected], Value::Null, "{unselected}");
    }
    for cell in [
        Cell {
            target: Some(1),
            ..Cell::default()
        },
        Cell {
            stop: Some(0),
            target: Some(1),
            ..Cell::default()
        },
        Cell {
            tsl: Some(usize::MAX),
            ..Cell::default()
        },
    ] {
        assert_eq!(
            selected_levels(&cell, long).err().as_deref(),
            Some("coordinate ladder index missing"),
            "{cell:?}"
        );
    }
    // (10): a second identity under the root whose catalog is held.
    let first = Asked::parse(&format!("identity={ID}")).unwrap();
    let held = render(&root, &first).unwrap();
    let other = [0x24; 32];
    let before = cold();
    let why = render(
        &root,
        &Asked {
            identity: other,
            ..unparsed("coordinates", None, None, 0)
        },
    )
    .unwrap_err();
    assert_eq!(cold(), before + 1, "a second identity is a cold admission");
    assert!(why.contains(&crate::server::hex32(other)), "{why}");
    assert!(why.contains(&root.display().to_string()), "{why}");
    assert_eq!(
        render(&root, &first).unwrap(),
        held,
        "the held catalog again"
    );
    assert_eq!(cold(), before + 2, "opened again, never lost");
    fs::remove_dir_all(root).unwrap();
}

/// [`fixture`] with each grid's saved forced stop replaced, long grid then
/// short, by a `(kind, level)` pair, and the receipt sealed again over the
/// changed body. Every other byte is the fixture's.
fn fixture_forced(name: &str, forced: [(u64, i64); 2]) -> PathBuf {
    let root = fixture(name);
    let directory = root.join("boolean-candidates-v1").join(ID);
    let mut body = fs::read(directory.join("body.bin")).unwrap();
    // Each grid writes its cost model, `[7; 32]`, and then a disabled forced
    // stop, kind 0 and level 0: sixteen zero bytes. Nothing else in the body
    // is a run of thirty-two sevens.
    let mut disabled = vec![7_u8; 32];
    disabled.extend_from_slice(&[0; 16]);
    let kinds: Vec<usize> = body
        .windows(disabled.len())
        .enumerate()
        .filter(|(_, window)| *window == disabled.as_slice())
        .map(|(start, _)| start + 32)
        .collect();
    assert_eq!(kinds.len(), 2, "premise: one forced stop per grid");
    for (at, (kind, level)) in kinds.into_iter().zip(forced) {
        body[at..at + 8].copy_from_slice(&kind.to_le_bytes());
        body[at + 8..at + 16].copy_from_slice(&level.to_le_bytes());
    }
    let mut receipt = b"BRBLCM01".to_vec();
    receipt.extend_from_slice(&[0x42; 32]);
    receipt.extend_from_slice(&hash(&body));
    words(&mut receipt, &[body.len() as u64]);
    receipt.extend_from_slice(&hash(&receipt));
    fs::write(directory.join("body.bin"), body).unwrap();
    fs::write(directory.join("complete.bin"), receipt).unwrap();
    root
}

/// **A GRID'S FORCED STOP IS SERVED BY ITS KIND AND ITS EXACT LEVEL, AND ONE
/// NO POLICY HOLDS REFUSES THE CATALOG** (BASE-COV-API-AS2): the saved policy
/// of each grid is written with an include-exact or a require-exact forced
/// stop, and every page's grid summaries say which, and at what level.
/// Extremes: 11 (7 single, 2 combined, 2 out of the box): (1) an include-exact
/// forced stop is served `include-exact-observed`; (2) a require-exact one is
/// served `require-exact-observed`; (3) a level of 1 ppm, the least a policy
/// takes, is served "1"; (4) a level of `i64::MAX` is served exact, as text;
/// (5) a pinned grid page carries the same summaries as the first page;
/// (6) the summary read straight off the reader is the served one; (7) the
/// fixture's own disabled forced stop is still `disabled` with a null level.
/// Combined: (8) include on the long grid AND require on the short: each grid
/// says its own; (9) a require-exact kind AND a level of 0: the catalog is
/// refused in the policy's own words and no page is served. Out of the box:
/// (10) a disabled kind that carries a level is no encoding and is refused;
/// (11) a kind no build writes, 3, is refused the same way.
#[test]
fn a_forced_stop_is_served_by_its_kind_and_its_exact_level() {
    let _cache = CACHE_TEST.lock().unwrap();
    let first = Asked::parse(&format!("identity={ID}")).unwrap();
    // (1)-(6), (8).
    let root = fixture_forced("boolean-api-forced", [(1, 1), (2, i64::MAX)]);
    let served = render(&root, &first).unwrap();
    assert_eq!(served["grids"][0]["side"], "long");
    assert_eq!(
        served["grids"][0]["forced_stop"],
        json!({"kind":"include-exact-observed","ppm":"1"})
    );
    assert_eq!(
        served["grids"][1]["forced_stop"],
        json!({"kind":"require-exact-observed","ppm":i64::MAX.to_string()})
    );
    let pin = served["completion"].as_str().unwrap();
    let grid = render(
        &root,
        &Asked::parse(&format!(
            "identity={ID}&completion={pin}&kind=grid&side=short&axis=stop"
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(grid["grids"], served["grids"]);
    let reader = Reader::open(&root, [0x42; 32], crate::detail::MAX_SCAN_BYTES).unwrap();
    assert_eq!(grid_summary(&reader.grids()[1]), served["grids"][1]);
    // (7).
    let plain = fixture("boolean-api-forced-disabled");
    assert_eq!(
        render(&plain, &first).unwrap()["grids"][0]["forced_stop"],
        json!({"kind":"disabled","ppm":null})
    );
    // (9), (10), (11).
    for (name, long, said) in [
        (
            "boolean-api-forced-zero",
            (2, 0),
            "exit-grid V1 refusal: InvalidForcedStop(0)",
        ),
        (
            "boolean-api-forced-level",
            (0, 5),
            "Boolean forced-stop encoding unknown",
        ),
        (
            "boolean-api-forced-three",
            (3, 1),
            "Boolean forced-stop encoding unknown",
        ),
    ] {
        let refused = fixture_forced(name, [long, (0, 0)]);
        let why = render(&refused, &first).unwrap_err();
        assert!(why.contains(said), "{name}: {why}");
        assert!(
            why.contains("no other folder was searched"),
            "{name}: {why}"
        );
        fs::remove_dir_all(refused).unwrap();
    }
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(plain).unwrap();
}

/// Names the handler child's catalog root.
const HANDLER_CHILD: &str = "BRUTEX_TEST_BOOLEAN_HANDLER_CHILD";

/// **THE PUBLIC HANDLER SERVES A SAVED PAGE WHOLE FROM THE ROOT `BRUTEX_STORE`
/// NAMES, AND REFUSES BY NAME WHAT THAT ROOT CANNOT ANSWER**
/// (BASE-COV-API-AS2). Run in a child, because the handler finds its root only
/// through the process environment; the child proves it ran by the file it
/// leaves.
/// Extremes: 11 (7 single, 2 combined, 2 out of the box): (1) the first page,
/// with no pin, is a 200 the catalog saved; (2) it is served as
/// `application/json`; (3) whole, under the response ceiling; (4) it is the
/// page `render` makes for the same question; (5) a pinned programs page is a
/// 200; (6) a pin the catalog does not hold is a 503 that says the completion
/// changed, with no rows; (7) an identity the root does not hold is a 503
/// naming it, the root and `BRUTEX_STORE`. Combined: (8) a pinned page AND an
/// offset past both coordinates: a 503 in the reader's words, no rows; (9) a
/// pinned trades page AND the short side's coordinate: a 200 with that
/// coordinate selected. Out of the box: (10) a limit of 256 over a catalog of
/// two coordinates: both rows and no next page; (11) a selector the parser
/// refuses is a 400 before the root is read at all.
#[test]
fn the_public_handler_serves_a_saved_page_whole_from_the_configured_root() {
    if let Some(root) = std::env::var_os(HANDLER_CHILD) {
        let root = PathBuf::from(root);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ask = |query: &str| {
            let (status, headers, body) = runtime.block_on(boolean_json(
                format!("/boolean-candidates.json?{query}").parse().unwrap(),
            ));
            let value: Value = serde_json::from_str(&body).unwrap();
            (status, headers[0].1, body.len(), value)
        };
        // (1)-(4): the first page, whole.
        let (status, kind, bytes, first) = ask(&format!("identity={ID}"));
        assert_eq!(status, StatusCode::OK, "{first}");
        assert_eq!(kind, "application/json");
        assert!(bytes <= crate::detail::MAX_RESPONSE_BYTES, "{bytes}");
        assert_eq!(first["status"], "saved");
        assert_eq!(first["total"], "2");
        assert_eq!(
            first,
            render(&root, &Asked::parse(&format!("identity={ID}")).unwrap()).unwrap()
        );
        let pin = first["completion"].as_str().unwrap().to_owned();
        // (5): a pinned later page.
        let (status, _, _, programs) =
            ask(&format!("identity={ID}&completion={pin}&kind=programs"));
        assert_eq!(status, StatusCode::OK, "{programs}");
        assert_eq!(programs["rows"][0]["expression"], "(0 | !(1))");
        // (9): the short side's coordinate.
        let (status, _, _, trades) = ask(&format!(
            "identity={ID}&completion={pin}&kind=trades&candidate=1"
        ));
        assert_eq!(status, StatusCode::OK, "{trades}");
        assert_eq!(trades["selected"]["index"], "1");
        assert_eq!(trades["selected"]["side"], "short");
        // (10): a limit past the catalog.
        let (status, _, _, all) = ask(&format!("identity={ID}&limit=256"));
        assert_eq!(status, StatusCode::OK, "{all}");
        assert_eq!(all["rows"].as_array().unwrap().len(), 2);
        assert_eq!(all["next"], Value::Null);
        // (6), (8): refusals in their own words.
        for (query, said) in [
            (
                format!("identity={ID}&completion={}&kind=programs", "7".repeat(64)),
                "catalog completion changed; no replacement page returned",
            ),
            (
                format!("identity={ID}&completion={pin}&offset=5"),
                "Boolean page bounds refused",
            ),
        ] {
            let (status, _, _, refused) = ask(&query);
            assert_eq!(
                status,
                StatusCode::SERVICE_UNAVAILABLE,
                "{query}: {refused}"
            );
            assert_eq!(refused["refusal"], said, "{query}");
            assert_eq!(refused["rows"], json!([]), "{query}");
        }
        // (7): an identity the root does not hold.
        let absent = "24".repeat(32);
        let (status, _, _, missing) = ask(&format!("identity={absent}"));
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{missing}");
        let why = missing["refusal"].as_str().unwrap();
        let shown = root.display().to_string();
        for part in [absent.as_str(), shown.as_str(), "BRUTEX_STORE"] {
            assert!(why.contains(part), "{part}: {why}");
        }
        // (11): a selector the parser refuses.
        let (status, _, _, bad) = ask("identity=bad");
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
        fs::write(root.join("child-ran"), b"served").unwrap();
        return;
    }
    let root = fixture("boolean-api-handler");
    let _ = crate::isolated::rerun(
        "booleanjson::tests::the_public_handler_serves_a_saved_page_whole_from_the_configured_root",
        &[
            (HANDLER_CHILD, root.as_os_str()),
            ("BRUTEX_STORE", root.as_os_str()),
        ],
    );
    assert_eq!(
        fs::read(root.join("child-ran")).ok().as_deref(),
        Some(&b"served"[..]),
        "the child asked the handler"
    );
    fs::remove_dir_all(root).unwrap();
}

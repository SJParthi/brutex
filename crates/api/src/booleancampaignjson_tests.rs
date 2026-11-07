#![cfg(test)]
//! Finite selector and projection tests; these are not generated market runs.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "bounded fixtures and assertions"
)]
use super::*;
use cli::boolean_campaign::{Catalog, Status};

const ID: &str = "1212121212121212121212121212121212121212121212121212121212121212";

#[test]
fn exact_campaign_selectors_refuse_paths_history_and_repeated_or_malformed_pins() {
    assert!(Asked::parse(&format!("identity={ID}")).is_ok());
    assert!(Asked::parse(&format!("identity={ID}&pin={ID}")).is_ok());
    for suffix in [
        "&pin=",
        "&pin=short",
        "&pin=x&pin=y",
        "&path=/tmp",
        "&sequence=0",
        "&limit=8",
        "&identity=x",
        "&completion=x",
    ] {
        assert!(
            Asked::parse(&format!("identity={ID}{suffix}")).is_err(),
            "{suffix}"
        );
    }
    assert!(Asked::parse("").is_err());
    assert!(Asked::parse(&"x".repeat(513)).is_err());
}

#[test]
fn missing_or_replaced_pin_never_falls_back_to_a_new_campaign_snapshot() {
    assert!(require_pin(None, [3; 32]).is_ok());
    assert!(require_pin(Some([3; 32]), [3; 32]).is_ok());
    assert!(
        require_pin(Some([2; 32]), [3; 32])
            .unwrap_err()
            .contains("explicitly refresh")
    );
}

#[test]
fn all_recorded_states_preserve_expected_uncompleted_families_and_exact_links() {
    let key = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .unwrap();
    let family = cli::boolean_observation::ResearchFamilyV1::new(key).unwrap();
    let states = [
        Status::Waiting,
        Status::Running,
        Status::Paused,
        Status::Refused,
        Status::Completed,
    ];
    let mut input = Vec::new();
    for status in states {
        input.push(Rung {
            rung: "1min",
            status,
            reason: if status == Status::Refused {
                "exact source changed".into()
            } else {
                String::new()
            },
            catalogs: vec![Catalog {
                family,
                expected: [3; 32],
                completion: (status == Status::Completed).then_some([4; 32]),
            }],
            statistics: (status == Status::Completed).then_some(Link {
                identity: [5; 32],
                completion: [6; 32],
            }),
            admission: (status == Status::Completed).then_some(Link {
                identity: [7; 32],
                completion: [8; 32],
            }),
        });
    }
    let projected = rows(&input);
    for (index, status) in states.iter().enumerate() {
        assert_eq!(projected[index]["state"], status.as_str());
        assert_eq!(
            projected[index]["expected_catalogs"][0]["identity"],
            crate::server::hex32([3; 32])
        );
        assert_eq!(
            projected[index]["catalogs"].as_array().unwrap().len(),
            usize::from(*status == Status::Completed)
        );
    }
    assert!(projected[0]["expected_catalogs"][0]["completion"].is_null());
    assert!(projected[0]["admission"].is_null());
    assert_eq!(projected[3]["reason"], "exact source changed");
    assert_eq!(
        projected[4]["catalogs"][0]["completion"],
        crate::server::hex32([4; 32])
    );
    assert_eq!(
        projected[4]["statistics"]["completion"],
        crate::server::hex32([6; 32])
    );
    assert_eq!(
        projected[4]["admission"]["completion"],
        crate::server::hex32([8; 32])
    );
}

#[tokio::test]
async fn public_campaign_route_refuses_invalid_selectors_before_reading_a_root() {
    for uri in [
        "/boolean-campaign.json",
        "/boolean-campaign.json?identity=short",
        "/boolean-campaign.json?path=/tmp",
    ] {
        let reply = campaign_json(uri.parse().unwrap()).await;
        assert_eq!(reply.0, StatusCode::BAD_REQUEST);
        let body: Value = serde_json::from_str(&reply.2).unwrap();
        assert_eq!(body["status"], "refused");
        assert_eq!(body["rows"], json!([]));
        let reply = qualified_campaign_json(uri.parse().unwrap()).await;
        assert_eq!(reply.0, StatusCode::BAD_REQUEST);
    }
}

#[test]
fn missing_campaign_reports_configured_root_without_creating_or_searching_outputs() {
    let root = crate::scratch::path("missing-boolean-campaign");
    let why = render(
        &root,
        &Asked {
            identity: [0x12; 32],
            pin: None,
        },
    )
    .unwrap_err();
    assert!(why.contains(root.to_str().unwrap()));
    assert!(why.contains("BRUTEX_STORE must match command OUTPUT_ROOT"));
    assert!(!root.exists());
    let why = render_qualified(
        &root,
        &Asked {
            identity: [0x12; 32],
            pin: None,
        },
    )
    .unwrap_err();
    assert!(why.contains(root.to_str().unwrap()));
    assert!(!root.exists());
}

#[test]
fn qualified_overview_authenticates_all_eight_initial_units_and_refuses_replacement() {
    use std::fs;
    let root = crate::scratch::path("qualified-eight-projection");
    let descriptor = [91; 32];
    let units: [[u8; 32]; 8] = std::array::from_fn(|index| [u8::try_from(index + 1).unwrap(); 32]);
    let mut h = brutex_core::blake3::Hasher::new();
    h.update(b"brutex.qualified-eight-slot-campaign.v1\0");
    h.update(&descriptor);
    for unit in units {
        h.update(&unit);
    }
    let identity = h.finalize();
    let directory = root
        .join("boolean-qualified-campaign-v1")
        .join(crate::server::hex32(identity));
    fs::create_dir_all(directory.join("0000000000000001")).unwrap();
    fs::write(directory.join("owner.lock"), []).unwrap();
    let mut body = b"BRQCAM01".to_vec();
    body.extend_from_slice(&identity);
    body.extend_from_slice(&descriptor);
    body.extend_from_slice(&[0; 40]);
    for unit in units {
        body.extend_from_slice(&unit);
        body.extend_from_slice(&[0; 1104]);
    }
    assert_eq!(body.len(), 9200);
    let mut bytes = b"BTXCHK01".to_vec();
    bytes.extend_from_slice(&identity);
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&9200_u64.to_le_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(&body);
    let mut h = brutex_core::blake3::Hasher::new();
    h.update(&bytes);
    let pin = h.finalize();
    bytes.extend_from_slice(&pin);
    let path = directory.join("0000000000000001/payload");
    fs::write(&path, bytes).unwrap();
    fs::write(directory.join("0000000000000001/complete"), pin).unwrap();
    let value = render_qualified(
        &root,
        &Asked {
            identity,
            pin: Some(pin),
        },
    )
    .unwrap();
    assert_eq!(value["state"], "waiting");
    assert_eq!(value["history_records"], "1");
    assert_eq!(value["rows"].as_array().unwrap().len(), 8);
    assert_eq!(value["rows"][7]["rung"], "60min");
    assert_eq!(value["rows"][7]["unit"], crate::server::hex32([8; 32]));
    assert_eq!(value["child_completion_receipts_checked"], false);
    assert_eq!(value["child_bodies_checked"], false);
    assert!(value["rows"][0]["qualification"].is_null());
    assert!(
        render_qualified(
            &root,
            &Asked {
                identity,
                pin: Some([37; 32])
            }
        )
        .is_err()
    );
    fs::write(path, b"changed").unwrap();
    assert!(
        render_qualified(
            &root,
            &Asked {
                identity,
                pin: Some(pin)
            }
        )
        .is_err()
    );
    fs::remove_dir_all(root).unwrap();
}

/// A waiting campaign over `family` alone, saved as its one acknowledged
/// snapshot under `root`, in the bytes `cli::boolean_campaign`'s codec and
/// checkpoint reader admit: every rung expects one catalog of that family,
/// and no child is complete, so no child receipt is read. Returns the
/// campaign's identity and the snapshot's pin.
fn save_waiting_campaign(
    root: &Path,
    family: cli::boolean_observation::ResearchFamilyV1,
) -> ([u8; 32], [u8; 32]) {
    use std::fs;
    const REASON_BYTES: usize = 1024;
    let (descriptor, programs, expected) = ([21; 32], [22; 32], [23; 32]);
    let (program_count, from, to, horizon) = (1_u64, 202_401_u64, 202_402_u64, 60_u64);
    let mut h = brutex_core::blake3::Hasher::new();
    h.update(b"brutex-boolean-campaign-identity-v1\0");
    h.update(&descriptor);
    h.update(&programs);
    for word in [program_count, from, to, horizon] {
        h.update(&word.to_le_bytes());
    }
    for rung in cli::EVERY_RUNG {
        h.update(&1_u64.to_le_bytes());
        h.update(&(rung.len() as u64).to_le_bytes());
        h.update(rung.as_bytes());
        h.update(&family.encode());
        h.update(&expected);
    }
    let identity = h.finalize();

    let mut body = b"BRBCAM01".to_vec();
    body.extend_from_slice(&identity);
    body.extend_from_slice(&descriptor);
    body.extend_from_slice(&programs);
    // program count, family count, from, to, horizon, waiting, no predecessor
    for word in [program_count, 1, from, to, horizon, 0, u64::MAX] {
        body.extend_from_slice(&word.to_le_bytes());
    }
    body.resize(256, 0);
    for _ in cli::EVERY_RUNG {
        // waiting, two absent stage links, an empty reason
        body.extend_from_slice(&0_u64.to_le_bytes());
        body.resize(body.len() + 128, 0);
        body.extend_from_slice(&0_u64.to_le_bytes());
        body.resize(body.len() + REASON_BYTES, 0);
        body.extend_from_slice(&family.encode());
        body.extend_from_slice(&expected);
        body.extend_from_slice(&[0; 32]);
    }
    assert_eq!(
        body.len(),
        256 + 8 * (1168 + 192),
        "the codec's fixed width"
    );

    let directory = root
        .join("boolean-campaign-v1")
        .join(crate::server::hex32(identity));
    fs::create_dir_all(directory.join("0000000000000001")).unwrap();
    fs::write(directory.join("owner.lock"), []).unwrap();
    let mut bytes = b"BTXCHK01".to_vec();
    bytes.extend_from_slice(&identity);
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&(body.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(&body);
    let pin = brutex_core::blake3::hash(&bytes);
    bytes.extend_from_slice(&pin);
    fs::write(directory.join("0000000000000001/payload"), bytes).unwrap();
    fs::write(directory.join("0000000000000001/complete"), pin).unwrap();
    (identity, pin)
}

/// **A campaign that expects a stock family states the equity note; a
/// campaign of indices does not.** D-0694, AF-19.
///
/// A campaign snapshot names each rung's expected catalogs and their family,
/// and each catalog's figures are a stock's when its family is cash. The
/// snapshot carries `cli::research_equity_note` over every expected family,
/// the Boolean research heading's own decision, so one cash family anywhere
/// is enough and a campaign of indices gains no key.
///
/// The helper is checked on hand-built rungs, and then `render` serves a
/// saved campaign over RELIANCE and one over NIFTY, each read with no pin and
/// with its own. The stock's page carries the note on both reads, and the
/// index's carries no key. Until this test rendered a page it held `render`
/// to the note by its source text, which a call made only on a pinned read
/// passed.
#[test]
fn a_campaign_expecting_a_stock_family_states_the_equity_note_and_an_index_campaign_does_not() {
    let family = |key: Result<brutex_core::instrument::InstrumentKey, _>| {
        cli::boolean_observation::ResearchFamilyV1::new(key.unwrap()).unwrap()
    };
    let nse = brutex_core::instrument::Exchange::Nse;
    let nifty = family(brutex_core::instrument::InstrumentKey::index(nse, "NIFTY"));
    let reliance = family(brutex_core::instrument::InstrumentKey::cash(
        nse, "RELIANCE",
    ));
    let rung = |families: &[cli::boolean_observation::ResearchFamilyV1]| Rung {
        rung: "5min",
        status: Status::Waiting,
        reason: String::new(),
        catalogs: families
            .iter()
            .map(|family| Catalog {
                family: *family,
                expected: [3; 32],
                completion: None,
            })
            .collect(),
        statistics: None,
        admission: None,
    };
    let note = cli::research_equity_note([reliance]);
    assert!(
        note.contains("CORPORATE ACTIONS ARE UNCHECKED"),
        "premise: {note}"
    );
    assert_eq!(equity_note(&[rung(&[nifty]), rung(&[nifty])]), "");
    assert_eq!(equity_note(&[]), "");
    assert_eq!(equity_note(&[rung(&[nifty, reliance])]), note);
    assert_eq!(
        equity_note(&[rung(&[nifty]), rung(&[reliance])]),
        note,
        "a stock family on any rung"
    );

    let root = crate::scratch::path("boolean-campaign-equity-note");
    let _stale = std::fs::remove_dir_all(&root);
    for (family, cash) in [(nifty, false), (reliance, true)] {
        let (identity, saved_pin) = save_waiting_campaign(&root, family);
        for pin in [None, Some(saved_pin)] {
            let page = render(&root, &Asked { identity, pin }).unwrap();
            let named = format!("{} (pin: {})", family.instrument(), pin.is_some());
            assert_eq!(page["status"], "saved", "{named}: premise");
            assert_eq!(page["rows"].as_array().unwrap().len(), 8, "{named}");
            for row in page["rows"].as_array().unwrap() {
                assert_eq!(
                    row["expected_catalogs"][0]["cash"], cash,
                    "{named}: premise, the family the rung expects"
                );
            }
            assert_eq!(
                page.get("equity_note"),
                cash.then(|| json!(note)).as_ref(),
                "{named}: a stock campaign's page states the note, an index one's has no key"
            );
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// **The qualified campaign route walks its whole history on every GET, and
/// that is stated.** W1-api1-4, D-1444.
///
/// No cache is added: the route serves a mutable latest snapshot. The bullet
/// must name the walk, its bound and the absence of a cache, and the source
/// must still be the uncached open the bullet describes.
#[test]
fn the_qualified_campaign_history_walk_per_request_is_stated() {
    let bullet = crate::booleanjson::tests::d0951_bullet("W1-api1-4");
    for word in [
        "booleancampaignjson::render_qualified",
        "QualifiedCampaign",
        "O(H)",
        "2H decodes",
        "DIRECTORY_LIMIT",
        "1,000,000",
        "detail::MAX_SCAN_BYTES",
        "no cache",
    ] {
        assert!(bullet.contains(word), "the bullet names {word}: {bullet}");
    }
    let source = include_str!("booleancampaignjson.rs");
    let body = source.split_once("\nfn render_qualified(").unwrap().1;
    let body = &body[..body.find("\n}\n").unwrap()];
    assert!(body.contains("cli::boolean_evidence::QualifiedCampaign::open(root,asked.identity,crate::detail::MAX_SCAN_BYTES)"));
    assert!(!source.contains("static CACHE"), "the route holds no cache");
}
/// One step of a qualified campaign's saved history: the slot it changes, and how.
#[derive(Clone, Copy)]
enum Step {
    /// An acknowledged start.
    Start(usize),
    /// A recorded completion link.
    Finish(usize),
    /// A recorded refusal reason.
    Refuse(usize, &'static str),
}

/// A qualified campaign of eight fixed units under `descriptor`, saved under
/// `root` as the acknowledged history the observer authenticates: the opening
/// all-waiting scope at sequence 1, then one record per step, each naming the
/// one before it. `mask` selects the rungs, `u8::MAX` being the original
/// all-eight encoding. A finished slot's link is `[0xC0; 32]` with the pin
/// `[0xD0 + slot; 32]`. Returns the identity and every record's pin, oldest
/// first.
fn save_qualified(
    root: &Path,
    descriptor: [u8; 32],
    mask: u8,
    steps: &[Step],
) -> ([u8; 32], Vec<[u8; 32]>) {
    const REASON: usize = 1024;
    let units: [[u8; 32]; 8] = std::array::from_fn(|index| [u8::try_from(index + 1).unwrap(); 32]);
    let mut h = brutex_core::blake3::Hasher::new();
    h.update(b"brutex.qualified-eight-slot-campaign.v1\0");
    if mask != u8::MAX {
        h.update(b"selected-intraday-rungs-v2\0");
        h.update(&[mask]);
    }
    h.update(&descriptor);
    for unit in units {
        h.update(&unit);
    }
    let identity = h.finalize();
    let directory = root
        .join("boolean-qualified-campaign-v1")
        .join(crate::server::hex32(identity));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("owner.lock"), []).unwrap();
    // Each slot: started, its completion pin, its refusal reason.
    let mut slots: [(bool, Option<[u8; 32]>, &str); 8] = [(false, None, ""); 8];
    let mut pins: Vec<[u8; 32]> = Vec::new();
    let opening = std::iter::once(None);
    for (sequence, step) in (1_u64..).zip(opening.chain(steps.iter().copied().map(Some))) {
        match step {
            Some(Step::Start(at)) => slots[at] = (true, None, ""),
            Some(Step::Finish(at)) => {
                slots[at] = (true, Some([u8::try_from(0xD0 + at).unwrap(); 32]), "");
            }
            Some(Step::Refuse(at, why)) => slots[at].2 = why,
            None => {}
        }
        let mut body = if mask == u8::MAX {
            b"BRQCAM01".to_vec()
        } else {
            b"BRQCAM02".to_vec()
        };
        body.extend_from_slice(&identity);
        body.extend_from_slice(&descriptor);
        let (prior, prior_pin) = pins.last().map_or((0, [0; 32]), |pin| (sequence - 1, *pin));
        body.extend_from_slice(&prior.to_le_bytes());
        body.extend_from_slice(&prior_pin);
        if mask != u8::MAX {
            body.extend_from_slice(&u64::from(mask).to_le_bytes());
        }
        for (unit, (started, pin, reason)) in units.iter().zip(slots) {
            body.extend_from_slice(unit);
            body.extend_from_slice(&u64::from(started).to_le_bytes());
            let (link, pin) = pin.map_or(([0; 32], [0; 32]), |pin| ([0xC0; 32], pin));
            body.extend_from_slice(&link);
            body.extend_from_slice(&pin);
            body.extend_from_slice(&(reason.len() as u64).to_le_bytes());
            body.extend_from_slice(reason.as_bytes());
            body.resize(body.len() + REASON - reason.len(), 0);
        }
        let mut bytes = b"BTXCHK01".to_vec();
        bytes.extend_from_slice(&identity);
        bytes.extend_from_slice(&sequence.to_le_bytes());
        bytes.extend_from_slice(&(body.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(&body);
        let pin = brutex_core::blake3::hash(&bytes);
        bytes.extend_from_slice(&pin);
        let record = directory.join(format!("{sequence:016x}"));
        std::fs::create_dir_all(&record).unwrap();
        std::fs::write(record.join("payload"), bytes).unwrap();
        std::fs::write(record.join("complete"), pin).unwrap();
        pins.push(pin);
    }
    (identity, pins)
}

/// **EVERY SLOT STATE A QUALIFIED HISTORY RECORDS IS SERVED BY NAME, AND THE
/// CAMPAIGN'S STATE IS READ FROM ITS SELECTED SLOTS** (BASE-COV-API-AS2): a
/// completed slot carries its recorded link, a refused one its reason, a
/// started one neither; a campaign is completed only when every selected slot
/// is, refused when any slot recorded a refusal, running when any started.
/// Extremes: 12 (8 single, 2 combined, 2 out of the box): (1) one selected
/// slot started and finished is a completed row carrying the link and pin
/// recorded for it; (2) a campaign of that one slot is completed; (3) the
/// seven slots the selection left out are no rows; (4) a started slot that
/// recorded a refusal is a refused row carrying its reason; (5) a campaign
/// holding it is refused; (6) a started slot with no outcome is a running row
/// with no reason and no link; (7) a campaign holding only that is running;
/// (8) the history count is the opening scope plus one record per step.
/// Combined: (9) a completed slot AND a running one AND six waiting: running,
/// never completed nor waiting; (10) two selected slots both finished:
/// completed, each row its own pin. Out of the box: (11) a refused slot
/// started again is running and carries no reason: the retry cleared it;
/// (12) an earlier record's own pin is refused: only the latest snapshot is
/// served.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one test walks every slot state its Extremes: line names, in order"
)]
fn every_slot_state_a_qualified_history_records_is_served_by_name() {
    let root = crate::scratch::path("qualified-slot-states");
    let _stale = std::fs::remove_dir_all(&root);
    let serve = |identity: [u8; 32], pins: &[[u8; 32]]| {
        let pin = pins.last().copied();
        render_qualified(&root, &Asked { identity, pin }).unwrap()
    };
    // (1), (2), (3), (8): one selected slot, finished.
    let (identity, pins) = save_qualified(&root, [91; 32], 0b1, &[Step::Start(0), Step::Finish(0)]);
    let one = serve(identity, &pins);
    assert_eq!(one["state"], "completed");
    assert_eq!(one["timeframes"], json!(["1min"]));
    assert_eq!(one["history_records"], "3");
    assert_eq!(one["rows"].as_array().unwrap().len(), 1, "(3)");
    assert_eq!(one["rows"][0]["state"], "completed");
    assert_eq!(
        one["rows"][0]["qualification"]["identity"],
        crate::server::hex32([0xC0; 32])
    );
    assert_eq!(
        one["rows"][0]["qualification"]["completion"],
        crate::server::hex32([0xD0; 32])
    );
    assert!(one["rows"][0]["reason"].is_null());
    // (4), (5): a refusal beside seven waiting slots.
    let (identity, pins) = save_qualified(
        &root,
        [92; 32],
        u8::MAX,
        &[Step::Start(0), Step::Refuse(0, "exact source changed")],
    );
    let refused = serve(identity, &pins);
    assert_eq!(refused["state"], "refused");
    assert_eq!(refused["rows"][0]["state"], "refused");
    assert_eq!(refused["rows"][0]["reason"], "exact source changed");
    assert!(refused["rows"][0]["qualification"].is_null());
    assert_eq!(refused["rows"][1]["state"], "waiting");
    // (6), (7), (12): one started slot, then the opening record's own pin.
    let (identity, pins) = save_qualified(&root, [93; 32], u8::MAX, &[Step::Start(0)]);
    let running = serve(identity, &pins);
    assert_eq!(running["state"], "running");
    assert_eq!(running["rows"][0]["state"], "running");
    assert!(running["rows"][0]["reason"].is_null());
    assert!(running["rows"][0]["qualification"].is_null());
    let stale = render_qualified(
        &root,
        &Asked {
            identity,
            pin: pins.first().copied(),
        },
    )
    .unwrap_err();
    assert!(stale.contains("explicitly refresh"), "{stale}");
    // (9), (11): one completed, then a refusal retried.
    let (identity, pins) = save_qualified(
        &root,
        [94; 32],
        u8::MAX,
        &[
            Step::Start(0),
            Step::Finish(0),
            Step::Start(1),
            Step::Refuse(1, "a source changed"),
            Step::Start(1),
        ],
    );
    let mixed = serve(identity, &pins);
    assert_eq!(mixed["state"], "running");
    assert_eq!(mixed["history_records"], "6");
    let states: Vec<&str> = mixed["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["state"].as_str().unwrap())
        .collect();
    assert_eq!(
        states,
        [
            "completed",
            "running",
            "waiting",
            "waiting",
            "waiting",
            "waiting",
            "waiting",
            "waiting"
        ]
    );
    assert!(mixed["rows"][1]["reason"].is_null(), "the retry cleared it");
    // (10): two selected slots, both finished.
    let (identity, pins) = save_qualified(
        &root,
        [95; 32],
        0b11,
        &[
            Step::Start(0),
            Step::Finish(0),
            Step::Start(1),
            Step::Finish(1),
        ],
    );
    let both = serve(identity, &pins);
    assert_eq!(both["state"], "completed");
    assert_eq!(both["timeframes"], json!(["1min", "2min"]));
    assert_eq!(
        both["rows"][1]["qualification"]["completion"],
        crate::server::hex32([0xD1; 32])
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Names the handler child's store root.
const HANDLER_CHILD: &str = "BRUTEX_TEST_BOOLEAN_CAMPAIGN_HANDLER_CHILD";
/// Hands the child the two identities it asks, campaign first.
const HANDLER_IDS: &str = "BRUTEX_TEST_BOOLEAN_CAMPAIGN_HANDLER_IDS";

/// **BOTH CAMPAIGN DOORS SERVE A SAVED SNAPSHOT WHOLE FROM THE ROOT
/// `BRUTEX_STORE` NAMES** (BASE-COV-API-AS2). Run in a child, because the
/// doors find their root only through the process environment; the child
/// proves it ran by the file it leaves.
/// Extremes: 11 (7 single, 2 combined, 2 out of the box): (1) the campaign
/// door answers a saved waiting snapshot 200 with no pin; (2) served as
/// `application/json`, whole, under the response ceiling; (3) with all eight
/// rungs; (4) asked with its own pin it is the same page; (5) asked with a
/// pin that is not its own it is a 503 that says to refresh, no rows; (6) the
/// qualified door answers a saved history 200; (7) its state is the
/// history's, running. Combined: (8) the campaign door asked for the
/// qualified campaign's identity: a 503, the two namespaces are apart; (9) the
/// qualified door asked with a pin that is not its own: a 503 to refresh.
/// Out of the box: (10) an identity of sixty-four zeros is read as a
/// selector and answered as a campaign the root does not hold, naming the
/// root; (11) the qualified door asked for the plain campaign's identity is
/// refused alike.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one test walks every door its Extremes: line names, in order"
)]
fn both_campaign_doors_serve_a_saved_snapshot_whole_from_the_configured_root() {
    if let Some(root) = std::env::var_os(HANDLER_CHILD) {
        let root = std::path::PathBuf::from(root);
        let ids = std::env::var(HANDLER_IDS).unwrap();
        let (campaign, qualified) = ids.split_once(',').unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let plain = |query: String| {
            runtime.block_on(campaign_json(
                format!("/boolean-campaign.json?{query}").parse().unwrap(),
            ))
        };
        let qualified_door = |query: String| {
            runtime.block_on(qualified_campaign_json(
                format!("/boolean-qualified-campaign.json?{query}")
                    .parse()
                    .unwrap(),
            ))
        };
        let json_of = |body: &str| serde_json::from_str::<Value>(body).unwrap();
        // (1), (2), (3).
        let (status, headers, body) = plain(format!("identity={campaign}"));
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(headers[0].1, "application/json");
        assert!(body.len() <= crate::detail::MAX_RESPONSE_BYTES);
        let page = json_of(&body);
        assert_eq!(page["status"], "saved");
        assert_eq!(page["authority"], "acknowledged-campaign-snapshot");
        assert_eq!(page["rows"].as_array().unwrap().len(), 8);
        // (4), (5).
        let pin = page["pin"].as_str().unwrap().to_owned();
        let (status, _, body) = plain(format!("identity={campaign}&pin={pin}"));
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(json_of(&body), page);
        let foreign = "37".repeat(32);
        let (status, _, body) = plain(format!("identity={campaign}&pin={foreign}"));
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
        let refused = json_of(&body);
        assert!(
            refused["refusal"]
                .as_str()
                .unwrap()
                .contains("explicitly refresh"),
            "{refused}"
        );
        assert_eq!(refused["rows"], json!([]));
        // (6), (7), (9).
        let (status, _, body) = qualified_door(format!("identity={qualified}"));
        assert_eq!(status, StatusCode::OK, "{body}");
        let history = json_of(&body);
        assert_eq!(history["state"], "running");
        assert_eq!(
            history["authority"],
            "acknowledged-qualified-campaign-history"
        );
        let (status, _, body) = qualified_door(format!("identity={qualified}&pin={foreign}"));
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
        // (8), (10), (11): identities a door does not hold.
        let shown = root.display().to_string();
        for (status, _, body) in [
            plain(format!("identity={qualified}")),
            plain(format!("identity={}", "0".repeat(64))),
            qualified_door(format!("identity={campaign}")),
        ] {
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
            let said = json_of(&body);
            assert!(said["refusal"].as_str().unwrap().contains(&shown), "{said}");
        }
        std::fs::write(root.join("child-ran"), b"served").unwrap();
        return;
    }
    let root = crate::scratch::path("boolean-campaign-handler");
    let _stale = std::fs::remove_dir_all(&root);
    let nifty = cli::boolean_observation::ResearchFamilyV1::new(
        brutex_core::instrument::InstrumentKey::index(
            brutex_core::instrument::Exchange::Nse,
            "NIFTY",
        )
        .unwrap(),
    )
    .unwrap();
    let (campaign, _) = save_waiting_campaign(&root, nifty);
    let (qualified, _) = save_qualified(&root, [96; 32], u8::MAX, &[Step::Start(0)]);
    let ids = format!(
        "{},{}",
        crate::server::hex32(campaign),
        crate::server::hex32(qualified)
    );
    let _ = crate::isolated::rerun(
        "booleancampaignjson::tests::both_campaign_doors_serve_a_saved_snapshot_whole_from_the_configured_root",
        &[
            (HANDLER_CHILD, root.as_os_str()),
            (HANDLER_IDS, std::ffi::OsStr::new(&ids)),
            ("BRUTEX_STORE", root.as_os_str()),
        ],
    );
    assert_eq!(
        std::fs::read(root.join("child-ran")).ok().as_deref(),
        Some(&b"served"[..]),
        "the child asked both doors"
    );
    std::fs::remove_dir_all(root).unwrap();
}

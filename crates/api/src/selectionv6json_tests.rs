#![cfg(test)]
//! `/selection-v6.json`: the family selector, the per-rung read and the
//! projection. D-1578. The decode itself is proved against a genuine
//! committed block in
//! `cli::selection_v6::tests::the_display_reader_decodes_the_authoritys_winners_and_refuses_any_other_family`.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "bounded fixtures and assertions"
)]
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-api-selection-v6-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// **An equity asked for by name is refused with `CLAUDE.md` §1's sentence.**
/// audit-20261003 gaps-10.
///
/// The route takes one selector, `family`, and only NIFTY or BANKNIFTY. A
/// cash equity on the engine surface is told it is one and why it cannot be
/// in Selection V6; anything else, an unknown key, a repeat or an empty value
/// is refused too. Through the handler, every refusal is a 400 whose body
/// carries the sentence and no rung.
#[tokio::test]
async fn an_equity_family_is_refused_loudly_and_only_the_two_indices_select() {
    assert_eq!(asked_family(""), Ok(None));
    assert_eq!(asked_family("family=NIFTY"), Ok(Some("NIFTY")));
    assert_eq!(asked_family("family=BANKNIFTY"), Ok(Some("BANKNIFTY")));
    for equity in ["RELIANCE", "TCS"] {
        let why = asked_family(&format!("family={equity}")).unwrap_err();
        assert!(
            why.starts_with(&format!("{equity} is a cash equity.")),
            "{why}"
        );
        assert!(why.contains(cli::SELECTION_V6_EQUITY_REFUSAL), "{why}");
    }
    for query in [
        "family=",
        "family=NIFTY&family=NIFTY",
        "rung=1min",
        "family",
        "family=INDIAVIX",
    ] {
        assert!(asked_family(query).is_err(), "{query}");
    }
    let (status, _, body) =
        selection_v6_json("/selection-v6.json?family=RELIANCE".parse().unwrap()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["status"], "refused");
    assert_eq!(body["rungs"], json!([]));
    assert!(
        body["refusal"]
            .as_str()
            .unwrap()
            .contains("No equity result may enter Selection V6"),
        "{body}"
    );
}

/// **The page selectors are bounded and strict.** Rust and O(1) sweep OS-5,
/// D-2303.
///
/// `from` and `limit` are decimal block numbers, each at most once; `limit`
/// is 1 to `PAGE_BLOCKS`. Absent rungs report zero blocks at the page asked
/// for. A rung with more blocks than a page is paged through `from`, never
/// refused (proved against genuine blocks in
/// `cli::selection_v6::tests::the_display_reader_decodes_the_authoritys_winners_and_refuses_any_other_family`).
#[test]
fn the_page_selectors_are_bounded_and_strict() {
    assert_eq!(
        asked("").unwrap(),
        Asked {
            family: None,
            from: 0,
            limit: PAGE_BLOCKS
        }
    );
    assert_eq!(
        asked("from=64&limit=3&family=NIFTY").unwrap(),
        Asked {
            family: Some("NIFTY"),
            from: 64,
            limit: 3
        }
    );
    for query in [
        "limit=0",
        "limit=9",
        "from=-1",
        "from=1e3",
        "from=",
        "from=1&from=2",
        "limit=2&limit=2",
        "page=1",
        "from=99999999999999999999999",
    ] {
        assert!(asked(query).is_err(), "{query}");
    }
    let scratch = Scratch::new();
    let body = render_page(
        &scratch.0,
        Asked {
            family: None,
            from: 7,
            limit: 2,
        },
    );
    for rung in body["rungs"].as_array().unwrap() {
        assert_eq!(rung["total_blocks"], "0");
        assert_eq!(rung["from"], "7");
    }
}

/// Every one of the eight rungs is reported, in ledger order: absent ones by
/// path, a damaged file as refused with the seal named, never as an empty
/// winner list.
#[test]
fn every_rung_is_absent_saved_or_refused_by_name() {
    let scratch = Scratch::new();
    let body = render(&scratch.0, None);
    assert_eq!(body["status"], "absent");
    assert_eq!(body["authority"], "sealed-stored-selection-v6-record");
    assert_eq!(body["equities"], cli::SELECTION_V6_EQUITY_REFUSAL);
    let rungs = body["rungs"].as_array().unwrap();
    let names: Vec<&str> = rungs.iter().map(|r| r["rung"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min"
        ]
    );
    for rung in rungs {
        assert_eq!(rung["status"], "absent");
        assert!(
            rung["path"]
                .as_str()
                .unwrap()
                .ends_with("global-selection-v6.bin")
        );
    }
    // A FORGED BLOCK: right size, wrong seal.
    let directory = scratch.0.join("selection").join("5min");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("global-selection-v6.bin"), [7_u8; 16_384]).unwrap();
    // A TORN TAIL: a commit that never finished.
    let torn = scratch.0.join("selection").join("60min");
    std::fs::create_dir_all(&torn).unwrap();
    std::fs::write(torn.join("global-selection-v6.bin"), [0_u8; 100]).unwrap();
    let body = render(&scratch.0, None);
    assert_eq!(body["status"], "absent", "no rung holds a readable record");
    let rung = |name: &str| {
        body["rungs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["rung"] == name)
            .unwrap()
            .clone()
    };
    assert_eq!(rung("5min")["status"], "refused");
    assert!(
        rung("5min")["refusal"]
            .as_str()
            .unwrap()
            .contains("completion seal")
    );
    assert_eq!(rung("60min")["status"], "refused");
    assert!(
        rung("60min")["refusal"]
            .as_str()
            .unwrap()
            .contains("incomplete trailing block")
    );
    assert_eq!(rung("1min")["status"], "absent");
}

fn winner(rank: u32, family: &'static str) -> cli::StoredSelectionV6Winner {
    cli::StoredSelectionV6Winner {
        rank,
        family,
        strategy_digest: [1; 32],
        disposition_id: [2; 32],
        selected_exit_digest: [3; 32],
        global_sequence: u64::from(rank),
        family_sequence: 0,
        score: u64::MAX,
        direction: "short",
        mask_words: [u64::MAX, 0, 0, 0, 0, 1 << 45],
        drawdown: 5,
        worst_loss: 4,
        losing_rate_ppm: 1,
        losing_trades: 1,
        winning_trades: 9,
        win_rate_ppm: 900_000,
        average_win: 10,
        average_loss: 4,
        assurance_ppm: 7,
        pessimistic_profit: i64::MIN,
        loss_ratio_ppm: None,
        reward_to_risk_ppm: Some(0),
    }
}

/// The projection carries every stored figure as an exact decimal string,
/// keeps an undefined ratio distinct from a measured zero, marks the Top-10
/// prefix, and a family selector narrows the winners and nothing else.
#[test]
fn a_record_projects_exactly_and_the_family_selector_narrows_winners() {
    let family = |family, terminal| cli::StoredSelectionV6Family {
        family,
        terminal,
        candidate_count: 3,
        evaluated_count: 3,
        decision_count: 3,
    };
    let record = cli::StoredSelectionV6Record {
        identity: [9; 32],
        rung_seconds: 300,
        horizon_bars: 15,
        population_id: [4; 32],
        execution_completion_id: [5; 32],
        policy_digest: [6; 32],
        row_count: 12,
        authorized_count: 11,
        policy_refused_count: 1,
        families: [
            family("NIFTY", "evaluated"),
            family("BANKNIFTY", "naturally-extinct"),
        ],
        considered: 12,
        admitted: 11,
        refused: 1,
        unmeasured: 0,
        winners: (0..12)
            .map(|rank| winner(rank, if rank % 3 == 0 { "BANKNIFTY" } else { "NIFTY" }))
            .collect(),
    };
    let all = project(&record, None);
    assert_eq!(all["winner_count"], 12);
    let winners = all["winners"].as_array().unwrap();
    assert_eq!(winners.len(), 12);
    assert_eq!(winners[0]["score"], u64::MAX.to_string());
    assert_eq!(winners[0]["pessimistic_profit"], i64::MIN.to_string());
    assert_eq!(winners[0]["loss_ratio_ppm"], Value::Null);
    assert_eq!(winners[0]["reward_to_risk_ppm"], "0");
    assert_eq!(winners[0]["mask_words"][5], (1_u64 << 45).to_string());
    assert_eq!(winners[9]["top_ten"], true);
    assert_eq!(winners[10]["top_ten"], false);
    assert_eq!(all["identity"], crate::server::hex32([9; 32]));
    assert_eq!(all["families"][1]["terminal"], "naturally-extinct");
    let banknifty = project(&record, Some("BANKNIFTY"));
    let narrowed = banknifty["winners"].as_array().unwrap();
    assert_eq!(narrowed.len(), 4);
    assert!(narrowed.iter().all(|w| w["family"] == "BANKNIFTY"));
    assert_eq!(
        banknifty["winner_count"], 12,
        "the record's own count is kept"
    );
}

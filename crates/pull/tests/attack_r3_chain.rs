//! ATTACK ROUND 3, CHAIN: the D-3116 dedupe across expiries.
//!
//! `chain::month` deduplicates a repeated name WITHIN one expiry's answer and
//! a second spelling of one decoded contract ACROSS the walk (D-3116). Its
//! own comment says the same NAME under a second expiry "still reaches
//! `read_contract`, whose token check refuses it by name". That holds for a
//! dated name, whose token carries its day. A monthly name carries no day
//! (`Mar25`), so the token check accepts it under any expiry of its month.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use std::cell::RefCell;

struct Canned(RefCell<Vec<String>>);

impl pull::chain::Discovery for Canned {
    async fn get(&self, _url: &str) -> Result<String, pull::chain::Refusal> {
        let mut left = self.0.borrow_mut();
        if left.is_empty() {
            return Err(pull::chain::Refusal::transport("no answer left".to_owned()));
        }
        Ok(left.remove(0))
    }
}

fn walk(expiries: &[&str], names: &[&str]) -> pull::chain::Chain {
    let quoted = |items: &[&str]| {
        items
            .iter()
            .map(|n| format!("\"{n}\""))
            .collect::<Vec<_>>()
            .join(",")
    };
    let mut answers = vec![format!(r#"{{"expiries":[{}]}}"#, quoted(expiries))];
    for _ in expiries {
        answers.push(format!(r#"{{"contracts":[{}]}}"#, quoted(names)));
    }
    let canned = Canned(RefCell::new(answers));
    let ask = pull::fno::Ask {
        underlying: "NIFTY".to_owned(),
        year: 2025,
        month: 3,
        expiry: String::new(),
    };
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime")
        .block_on(pull::chain::month(pull::vendor::Feed::Groww, &ask, &canned))
        .expect("the walk completes")
}

/// ROUND 3, D-3126: one vendor NAME answered under two expiries is one series
/// the vendor will serve once, whatever date it was listed beside. Filing it
/// as two contracts fetches the same bars twice and stores one copy under an
/// expiry that is not theirs — undetectable afterwards, and §8 forbids the
/// repair. The second listing is refused by name; a DATED name under the wrong
/// expiry is still refused by `read_contract`, as before.
#[test]
fn one_vendor_name_under_two_expiries_is_filed_once() {
    let chain = walk(
        &["2025-03-26", "2025-03-27"],
        &["NSE-NIFTY-Mar25-FUT", "NSE-NIFTY-Mar25-24000-CE"],
    );
    let filed: Vec<&str> = chain
        .contracts
        .iter()
        .map(|found| found.vendor_symbol.as_str())
        .collect();
    assert_eq!(
        filed,
        ["NSE-NIFTY-Mar25-FUT", "NSE-NIFTY-Mar25-24000-CE"],
        "each name is filed once, under the first expiry that listed it: {chain:#?}"
    );
    assert_eq!(chain.unreadable.len(), 2, "{chain:#?}");
    for said in &chain.unreadable {
        assert!(
            said.contains("2025-03-26"),
            "the first listing is named: {said}"
        );
    }
    assert!(!chain.whole());

    // A dated name under the wrong expiry is refused by its own token.
    let dated = walk(&["2025-03-26", "2025-03-27"], &["NSE-NIFTY-27Mar25-FUT"]);
    assert_eq!(dated.contracts.len(), 1, "{dated:#?}");
    assert_eq!(dated.unreadable.len(), 1, "{dated:#?}");
}

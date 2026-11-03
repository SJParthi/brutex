#![cfg(test)]
//! BASE-COV-PULL-PS2 (3 Nov 2026): the chain walk's arms Route B found
//! unreached on START (2e7a0b6d3b56). Every expiry and contract is invented.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

/// A transport that answers from memory in order, recording each URL, and
/// refuses once its answers run out (the second test needs that refusal).
struct Answers {
    asked: std::cell::RefCell<Vec<String>>,
    left: std::cell::RefCell<Vec<String>>,
}

impl Answers {
    fn of(bodies: &[&str]) -> Self {
        Self {
            asked: std::cell::RefCell::new(Vec::new()),
            left: std::cell::RefCell::new(bodies.iter().map(|b| (*b).to_owned()).collect()),
        }
    }
}

impl Discovery for Answers {
    async fn get(&self, url: &str) -> Result<String, Refusal> {
        self.asked.borrow_mut().push(url.to_owned());
        let mut left = self.left.borrow_mut();
        if left.is_empty() {
            return Err(Refusal::transport("no answer left".to_owned()));
        }
        Ok(left.remove(0))
    }
}

fn ask() -> Ask {
    Ask {
        underlying: "NIFTY".to_owned(),
        year: 2024,
        month: 1,
        expiry: String::new(),
    }
}

/// PS2-CH-01: an expiry the vendor spells any way but `YYYY-MM-DD` is named
/// unreadable ("its contracts were not asked for") and never sent: the walk
/// asks contracts only for the one it can read.
#[tokio::test]
async fn an_expiry_the_vendor_spells_otherwise_is_named_and_never_asked() {
    let from = Answers::of(&[
        r#"{"payload":{"expiries":["2024-01-25","25-Jan-2024","2024-1-25"]}}"#,
        r#"{"payload":{"contracts":["NSE-NIFTY-25Jan24-21000-CE"]}}"#,
    ]);
    let chain = month(Feed::Groww, &ask(), &from).await.unwrap();
    assert_eq!(
        chain.unreadable,
        [
            "expiry \"25-Jan-2024\" (its contracts were not asked for)",
            "expiry \"2024-1-25\" (its contracts were not asked for)",
        ]
    );
    let asked = from.asked.borrow();
    assert_eq!(
        asked.len(),
        2,
        "one expiries call, one contracts call: {asked:?}"
    );
    assert!(asked[1].contains("2024-01-25"), "{}", asked[1]);
    assert_eq!(chain.contracts.len(), 1);
    assert!(!chain.whole());
}

/// PS2-CH-02: a contracts call that gets no answer stops the walk with a
/// transport refusal naming that call's URL and the host's words; nothing of
/// the month is returned as if whole.
#[tokio::test]
async fn a_contracts_call_that_gets_no_answer_stops_the_walk_by_its_url() {
    let from = Answers::of(&[r#"{"payload":{"expiries":["2024-01-25"]}}"#]);
    let refused = month(Feed::Groww, &ask(), &from).await;
    let Err(ChainError::Transport { url, why }) = refused else {
        panic!("{refused:?}");
    };
    assert!(
        url.contains("2024-01-25"),
        "the contracts call's URL: {url}"
    );
    assert_eq!(from.asked.borrow().last(), Some(&url));
    assert!(why.contains("no answer left"), "{why}");
}

/// PS2-CH-03: only a ten-byte `YYYY-MM-DD` naming a real date is an ISO
/// expiry: a short text, a slash for either dash, letters for digits and a
/// month or day no calendar has are none.
#[test]
fn only_a_ten_byte_dashed_real_date_is_an_iso_expiry() {
    assert_eq!(
        iso_expiry("2024-01-25"),
        Some(brutex_core::instrument::Expiry::new(2024, 1, 25).unwrap())
    );
    for text in [
        "2024-01-2",
        "2024-01-255",
        "2024/01-25",
        "2024-01/25",
        "abcd-01-25",
        "2024-0a-25",
        "2024-01-2b",
        "2024-13-25",
        "2024-02-30",
        "",
    ] {
        assert_eq!(iso_expiry(text), None, "{text:?}");
    }
}

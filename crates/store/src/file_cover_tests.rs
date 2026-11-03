#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
//! BASE-COV-LIBS-LS1: the write door's words and sentences that no other test
//! of this module reached, each driven by the value or the input that reaches
//! it. Every stamp and price is invented.

use std::fmt::{self, Write as _};
use std::path::PathBuf;

use super::*;

/// A formatter sink that refuses its `refuse`-th write (counted from 0) and
/// takes every other one, so a sentence that went on writing after a refusal
/// shows it in what was taken.
struct Refuses {
    refuse: usize,
    calls: usize,
    took: String,
}

impl Refuses {
    const fn at(refuse: usize) -> Self {
        Self {
            refuse,
            calls: 0,
            took: String::new(),
        }
    }
}

impl fmt::Write for Refuses {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let call = self.calls;
        self.calls += 1;
        if call == self.refuse {
            return Err(fmt::Error);
        }
        self.took.push_str(s);
        Ok(())
    }
}

fn sealed_refusal(offer: SealedOffer) -> StoreError {
    StoreError::Sealed {
        path: PathBuf::from("/brutex-cover/bars/gdfl/NSE/INDEX/QCOVA/1s/2031-02.sbar"),
        n_valid: 3,
        last_ts_micros: 1_927_000_000_000_000,
        at: 1,
        ts_micros: 1_927_000_001_000_000,
        offer,
    }
}

/// **EVERY SEALED OFFER HAS ITS OWN WORD, AND THE SENTENCE NAMES THE
/// CONFLICT.** A batch a sealed month refuses carries what the raw month
/// would have met; the three disagreements are three words, never `follows`,
/// and the sentence says which held stamp the batch met.
#[test]
fn every_sealed_offer_has_its_own_word_and_the_sentence_names_the_conflict() {
    let words = [
        (SealedOffer::Follows, "follows"),
        (SealedOffer::Disagrees(Conflict::Restated), "restated"),
        (SealedOffer::Disagrees(Conflict::NotHeld), "not_held"),
        (SealedOffer::Disagrees(Conflict::Skipped), "skipped"),
    ];
    let mut seen = std::collections::BTreeSet::new();
    for (offer, word) in words {
        assert_eq!(offer.as_str(), word, "{offer:?}");
        assert!(seen.insert(word), "{word} said twice");
        let said = sealed_refusal(offer).to_string();
        assert!(said.contains(&format!("(sealed_offer={word})")), "{said}");
    }
    let restated = sealed_refusal(SealedOffer::Disagrees(Conflict::Restated)).to_string();
    assert!(
        restated.contains("batch record 1 restates the record held at 1927000001000000"),
        "{restated}"
    );
    let skipped = sealed_refusal(SealedOffer::Disagrees(Conflict::Skipped)).to_string();
    assert!(
        skipped.contains("batch record 1 skips the record held at 1927000001000000"),
        "{skipped}"
    );
    let follows = sealed_refusal(SealedOffer::Follows).to_string();
    assert!(
        follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),
        "{follows}"
    );
}

/// **A SINK THAT REFUSES THE SENTENCE ENDS IT AT THE REFUSED WRITE.** The
/// sentence is written in pieces; a refusal of the first piece is returned
/// and nothing after it is written, though the sink would take the rest. A
/// sink that refuses nothing gets the whole sentence the `Display` gives.
#[test]
fn a_sink_that_refuses_the_sealed_sentence_ends_it_at_the_refused_write() {
    let refused = sealed_refusal(SealedOffer::Disagrees(Conflict::NotHeld));
    let mut first = Refuses::at(0);
    assert_eq!(write!(first, "{refused}"), Err(fmt::Error));
    assert_eq!(
        (first.calls, first.took.as_str()),
        (1, ""),
        "nothing is written after the refused piece"
    );
    let whole = refused.to_string();
    let mut never = Refuses::at(usize::MAX);
    assert_eq!(write!(never, "{refused}"), Ok(()));
    assert_eq!(never.took, whole);
    // The last piece refused: everything before it was taken, in order.
    let mut last = Refuses::at(never.calls - 1);
    assert_eq!(write!(last, "{refused}"), Err(fmt::Error));
    assert!(
        whole.starts_with(&last.took) && last.took.len() < whole.len(),
        "{:?}",
        last.took
    );
}

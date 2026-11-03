#![cfg(test)]
//! BASE-COV-PULL repair 2 (D-7334): each census step of a re-key plan (FM-273
//! step 4) said in the receipt's words, and what it adds to the plan's
//! pending count: a step not known done is pending, never read as done.

use super::*;

/// The four steps, each its own line and its own pending count (extremes:
/// no key listed, and a listed step of several keys).
#[test]
fn every_census_step_says_itself_and_counts_what_is_pending() {
    let listed = CensusStep::Listed(vec!["ZQOLD 2024-03".to_owned(), "ZQOLD 2024-04".to_owned()]);
    let steps = [
        CensusStep::Clear,
        listed,
        CensusStep::NotRead,
        CensusStep::Unreadable("an invented fault".to_owned()),
    ];
    let said: Vec<(String, usize)> = steps
        .iter()
        .map(|step| (step.describe(), step.pending()))
        .collect();
    assert_eq!(
        said,
        [
            (
                "census: lists no key the ledger withdraws (step 4 done)".to_owned(),
                0
            ),
            (
                "census: 2 withdrawn key(s) still listed; --apply installs the FNO epoch \
                 without them (FM-273 step 4)"
                    .to_owned(),
                2
            ),
            (
                "census: not read (a plan of a survey with no store root); step 4 is not \
                 known done"
                    .to_owned(),
                1
            ),
            (
                "census: could not be read (an invented fault); step 4 is not known done"
                    .to_owned(),
                1
            ),
        ]
    );
    assert_eq!(CensusStep::default(), CensusStep::NotRead);
}

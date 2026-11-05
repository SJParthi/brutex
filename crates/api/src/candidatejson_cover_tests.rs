//! Rows of `candidatejson.rs` no test reached before BASE-COV-API repair 3
//! (shard AS1, D-8661): each refusal of a candidate query that the existing
//! bounds test reached only as a repeated field.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a whole-file test module; fixtures are bounded and a failed assertion is the test failing"
)]

use super::*;

const ID: &str = "4242424242424242424242424242424242424242424242424242424242424242";

/// The refusal of `identity=ID&attempt=1` with `rest` appended, or `ok`.
fn said(rest: &str) -> String {
    Asked::parse(&format!("identity={ID}&attempt=1{rest}"))
        .map_or_else(|why| why, |_| "ok".to_owned())
}

/// **EVERY CANDIDATE QUERY IS READ OR REFUSED BY ITS OWN RULE**
/// (BASE-COV-API repair 3, D-8661): `Asked::parse`. Extremes: a limit of
/// one and of 256, read; a limit of zero; of 257; a direction that is neither
/// long nor short; a rank with no direction; a direction with no rank; a rank
/// of zero; and, at once, a rank of zero with a direction that is neither
/// (the direction is refused first), and a limit of zero with a rank and no
/// direction (the limit is refused first); out of the box, a limit of
/// `u64::MAX`.
#[test]
fn every_candidate_query_is_read_or_refused_by_its_own_rule() {
    let limit = "candidate page limit must be 1..=256";
    let direction = "candidate direction must be long or short";
    let pair = "candidate trade pages require both positive rank and direction";
    for (rest, expected) in [
        ("&limit=1", "ok"),
        ("&limit=256", "ok"),
        ("&limit=0", limit),
        ("&limit=257", limit),
        ("&direction=up", direction),
        ("&rank=1", pair),
        ("&direction=long", pair),
        ("&rank=0&direction=long", pair),
        ("&rank=0&direction=up", direction),
        ("&limit=0&rank=1", limit),
        ("&limit=18446744073709551615", limit),
    ] {
        assert_eq!(said(rest), expected, "{rest}");
    }
}

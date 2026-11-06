#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use std::collections::BTreeSet;

use super::*;
use crate::rng::SplitMix64;

fn names(list: &[Name]) -> Vec<&str> {
    list.iter().map(|n| n.name.as_str()).collect()
}

#[test]
fn t_f01_loads_in_byte_order_with_ordinals_and_ids() {
    let list = load(b"RELIANCE\nM&M\nICICILIFE\n").unwrap();
    assert_eq!(names(&list), ["ICICILIFE", "M&M", "RELIANCE"]);
    assert_eq!(list.iter().map(|n| n.ordinal).collect::<Vec<_>>(), [0, 1, 2]);
    assert_eq!(list[0].series, "ICICIPRULI");
    assert_eq!(list[0].id, fnv1a32(b"ICICIPRULI"));
    assert_eq!(list[2].series, "RELIANCE");
    let shuffled = load(b"ICICILIFE\nRELIANCE\nM&M").unwrap();
    assert_eq!(shuffled, list);
    assert_eq!(fnv1a32(b""), 0x811c_9dc5);
}

/// X-F01: refusals and byte-order edges.
#[test]
fn x_f01_universe_extremes() {
    assert_eq!(load(b"A\nA\n"), Err(vec![Refusal::Duplicate(1, 2)]));
    assert_eq!(load(b"A\n\nB\n"), Err(vec![Refusal::EmptyLine(2)]));
    assert_eq!(load(b"reliance\n"), Err(vec![Refusal::BadName(1)]));
    assert_eq!(load(b" RELIANCE\n"), Err(vec![Refusal::BadName(1)]));
    assert_eq!(load(b"RELIANCE \n"), Err(vec![Refusal::BadName(1)]));
    let order = load(b"MANAPPURAM\nM&MFIN\nM&M\nBAJAJFINSV\nBAJAJ-AUTO\n").unwrap();
    assert_eq!(names(&order), ["BAJAJ-AUTO", "BAJAJFINSV", "M&M", "M&MFIN", "MANAPPURAM"]);
    assert_eq!(load(b""), Err(vec![Refusal::Empty]));
    assert_eq!(load(b"\n"), Err(vec![Refusal::Empty]));
    // combo: the alias and its target listed together.
    assert_eq!(load(b"ICICILIFE\nICICIPRULI\n"), Err(vec![Refusal::AliasAndTarget]));
    // combo: a forced id collision and a duplicate spelling, the collision first.
    let collide = load_with(b"A\nB\nC\nC\n", |_| 7);
    assert_eq!(
        collide,
        Err(vec![Refusal::IdCollision("A".to_owned(), "B".to_owned()), Refusal::IdCollision("B".to_owned(), "C".to_owned()), Refusal::Duplicate(3, 4)])
    );
    // oob: a byte-order mark, a carriage return and a byte that is not UTF-8.
    assert_eq!(load(b"\xEF\xBB\xBFA\n"), Err(vec![Refusal::Encoding]));
    assert_eq!(load(b"A\r\nB\r\n"), Err(vec![Refusal::Encoding]));
    assert_eq!(load(b"A\n\xFF\n"), Err(vec![Refusal::Encoding]));
}

/// P-F01-1: generated lists against a `BTreeSet` model.
#[test]
fn p_f01_load_vs_btreeset_model() {
    let alphabet = b"ABCZ09&-";
    for seed in crate::generator::SEEDS {
        let mut r = SplitMix64::new(seed);
        for _ in 0..2_500 {
            let n = r.below(6) + 1;
            let mut lines: Vec<String> = Vec::new();
            for _ in 0..n {
                let len = r.below(3) + 1;
                let s: String = (0..len).map(|_| char::from(alphabet[usize::try_from(r.below(8)).unwrap()])).collect();
                lines.push(s);
            }
            let text = lines.join("\n");
            let model: BTreeSet<&str> = lines.iter().map(String::as_str).collect();
            match load(text.as_bytes()) {
                Ok(list) => {
                    assert_eq!(model.len(), lines.len());
                    assert_eq!(names(&list), model.into_iter().collect::<Vec<_>>());
                }
                Err(e) => assert!(model.len() < lines.len(), "{text:?}: {e:?}"),
            }
        }
    }
}

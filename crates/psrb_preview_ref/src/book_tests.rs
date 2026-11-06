#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::generator::day;
use crate::rng::SplitMix64;
use crate::trade::{ExitKind, Fill, Simulated};

fn filled(exit: u32) -> Outcome {
    Outcome::Filled(Simulated {
        entry: Fill { second: 0, price: 1 },
        qty: 1,
        exit: Fill { second: exit, price: 1 },
        kind: ExitKind::Time,
        fallback: false,
        late: false,
        first_stop: 1,
        last_stop: 1,
        no_ratio_trail: 0,
    })
}

fn cand(t: u32, symbol: &str, side: Side, outcome: Outcome) -> Candidate {
    Candidate { day: day(2018, 9, 4), t, symbol: symbol.to_owned(), side, outcome }
}

#[test]
fn t_f53_merge_orders_by_instant_then_bytes_and_views_filter_sides() {
    let a = vec![cand(20, "B", Side::Long, filled(30)), cand(10, "B", Side::Short, filled(30))];
    let b = vec![cand(20, "A", Side::Short, filled(30))];
    let both = merge(&[a.clone(), b.clone()], View::Both).unwrap();
    let keys: Vec<(u32, &str)> = both.iter().map(|c| (c.t, c.symbol.as_str())).collect();
    assert_eq!(keys, [(10, "B"), (20, "A"), (20, "B")]);
    assert_eq!(merge(&[a.clone(), b.clone()], View::Long).unwrap().len(), 1);
    assert_eq!(merge(&[a, b], View::Short).unwrap().len(), 2);
    assert_eq!(View::ALL.map(View::label), ["LONG", "SHORT", "BOTH"]);
    assert!(merge(&[], View::Both).unwrap().is_empty());
}

/// X-F53 and X-F54: the book at its edges.
#[test]
fn x_f53_f54_book_extremes() {
    // a duplicate key refuses by name
    let dup = merge(&[vec![cand(5, "A", Side::Long, filled(9))], vec![cand(5, "A", Side::Short, filled(9))]], View::Both);
    assert_eq!(dup, Err(DuplicateKey { day: day(2018, 9, 4), t: 5, symbol: "A".to_owned() }));
    // prefix symbols at one instant: M&M before M&MFIN
    let pre = merge(&[vec![cand(1, "M&MFIN", Side::Long, filled(9))], vec![cand(1, "M&M", Side::Long, filled(9))]], View::Long).unwrap();
    assert_eq!(pre[0].symbol, "M&M");
    // 20 admitted, the 21st skipped
    let mut slots = Slots::default();
    for i in 0..20 {
        assert_eq!(slots.offer(&cand(100, &format!("S{i:02}"), Side::Long, filled(200))), Admission::Admitted);
    }
    assert_eq!(slots.offer(&cand(100, "S20", Side::Long, filled(200))), Admission::SlotsFull);
    assert_eq!(slots.len(), 20);
    // the exit second equal to T is still held (inclusive); one before it is freed
    assert_eq!(slots.offer(&cand(200, "S21", Side::Long, filled(300))), Admission::SlotsFull);
    assert_eq!(slots.offer(&cand(201, "S21", Side::Long, filled(300))), Admission::Admitted);
    // the stock's own exit at its next T: while open
    assert_eq!(slots.offer(&cand(300, "S21", Side::Long, filled(400))), Admission::WhileOpen);
    // combo: a while-open candidate is counted before the full book
    let mut full = Slots::default();
    for i in 0..20 {
        full.offer(&cand(1, &format!("S{i:02}"), Side::Long, filled(50)));
    }
    assert_eq!(full.offer(&cand(2, "S03", Side::Short, filled(60))), Admission::WhileOpen);
    // oob: an unfillable candidate never holds a slot, even on an empty book (Q6)
    let mut empty = Slots::default();
    assert!(empty.is_empty());
    for o in [Outcome::NotFilled, Outcome::NoRatio, Outcome::ZeroQuantity] {
        assert_eq!(empty.offer(&cand(1, "X", Side::Long, o)), Admission::Unfillable);
    }
    assert!(empty.is_empty());
    // a new day frees every earlier position
    let mut next = full.clone();
    let tomorrow = Candidate { day: day(2018, 9, 5), ..cand(1, "S99", Side::Long, filled(5)) };
    assert_eq!(next.offer(&tomorrow), Admission::Admitted);
    assert_eq!(next.len(), 1);
    assert_eq!(cand(1, "Q", Side::Long, Outcome::NotFilled).exit_at(), None);
}

/// P-F54-1: generated books against a model counting the open intervals at each T.
#[test]
fn p_f54_slots_vs_interval_model() {
    for seed in crate::generator::SEEDS {
        let mut r = SplitMix64::new(seed);
        for _ in 0..250 {
            let mut list: Vec<Candidate> = Vec::new();
            for i in 0..60u32 {
                let t = i * 10 + u32::try_from(r.below(10)).unwrap();
                let symbol = format!("S{:02}", r.below(30));
                let o = if r.chance(1, 10) { Outcome::NotFilled } else { filled(t + u32::try_from(r.below(400)).unwrap()) };
                list.push(cand(t, &symbol, Side::Long, o));
            }
            list.sort_by(|a, b| a.key().cmp(&b.key()));
            list.dedup_by(|a, b| a.key() == b.key());
            let mut slots = Slots::default();
            let mut model: Vec<(String, u32)> = Vec::new();
            for c in &list {
                let got = slots.offer(c);
                model.retain(|(_, exit)| *exit >= c.t);
                let want = if model.iter().any(|(s, _)| *s == c.symbol) {
                    Admission::WhileOpen
                } else if let Some((_, exit)) = c.exit_at() {
                    if model.len() >= SLOTS {
                        Admission::SlotsFull
                    } else {
                        model.push((c.symbol.clone(), exit));
                        Admission::Admitted
                    }
                } else {
                    Admission::Unfillable
                };
                assert_eq!(got, want);
            }
        }
    }
}

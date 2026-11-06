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

const ANSWERS: [Answer; 3] = [Answer::Yes, Answer::No, Answer::Unknown];

fn folder() -> Membership {
    let mut m = Membership::all_members(&["RELIANCE", "SYRMA"]);
    m.answers.insert(("SYRMA".to_owned(), day(2026, 9, 30)), Answer::No);
    m.carry_from = Some(day(2026, 9, 30));
    m
}

#[test]
fn t_f02_the_probe_is_dated_only_on_the_expected_triple() {
    let mut dated = 0;
    for opened in [true, false] {
        for a in ANSWERS {
            for b in ANSWERS {
                for c in ANSWERS {
                    let (mode, reasons) = probe(&Probe { opened, reliance_first: a, reliance_last: b, syrma_last: c });
                    let ok = opened && a == Answer::Yes && b == Answer::Yes && c != Answer::Yes;
                    assert_eq!(mode == Mode::Dated, ok);
                    assert_eq!(reasons.is_empty(), ok);
                    dated += usize::from(ok);
                }
            }
        }
    }
    assert_eq!(dated, 2);
    let (_, all_wrong) =
        probe(&Probe { opened: false, reliance_first: Answer::No, reliance_last: Answer::Unknown, syrma_last: Answer::Yes });
    assert_eq!(all_wrong.len(), 4);
    assert!(SURVIVORSHIP.contains("218"));
}

/// X-F03: the membership answers at their edges.
#[test]
fn x_f03_may_trade_extremes() {
    let m = folder();
    let (first, last) = (day(2018, 9, 3), day(2026, 10, 30));
    assert_eq!(m.may_trade("RELIANCE", first, first, last), Ok(Answer::Yes));
    assert_eq!(m.may_trade("SYRMA", day(2026, 9, 30), first, last), Ok(Answer::No));
    // the carry: every October day asks 2026-09-30 (combo with the dated 'no').
    assert_eq!(m.may_trade("SYRMA", day(2026, 10, 1), first, last), Ok(Answer::No));
    assert_eq!(m.may_trade("SYRMA", last, first, last), Ok(Answer::No));
    assert_eq!(m.asked_day(day(2026, 10, 14)), day(2026, 9, 30));
    assert_eq!(m.asked_day(day(2026, 9, 29)), day(2026, 9, 29));
    // oob: outside the window, never asked.
    assert_eq!(m.may_trade("RELIANCE", day(2026, 11, 2), first, last), Err(Refusal::OutsideWindow));
    assert_eq!(m.may_trade("RELIANCE", day(2018, 8, 31), first, last), Err(Refusal::OutsideWindow));
    assert_eq!(m.may_trade("ABSENT", first, first, last), Ok(Answer::No));
    let mut unknown = folder();
    unknown.answers.insert(("RELIANCE".to_owned(), day(2026, 9, 30)), Answer::Unknown);
    // combo: a refusal on the carry day makes every October day unknown.
    assert_eq!(unknown.may_trade("RELIANCE", day(2026, 10, 20), first, last), Ok(Answer::Unknown));
    let mut today = folder();
    today.probe.syrma_last = Answer::Yes;
    assert_eq!(today.may_trade("SYRMA", day(2026, 9, 30), first, last), Ok(Answer::Yes));
    assert_eq!(today.may_trade("ABSENT", first, first, last), Ok(Answer::No));
}

/// P-F03-1: generated (name, day) questions against a map model with the carry rule.
#[test]
fn p_f03_may_trade_vs_map_model() {
    let first = day(2026, 8, 3);
    let last = day(2026, 10, 30);
    for seed in crate::generator::SEEDS {
        let mut r = SplitMix64::new(seed);
        let mut m = Membership::all_members(&["A", "B"]);
        m.carry_from = Some(day(2026, 9, 30));
        for _ in 0..40 {
            let d = Day::from_index(first.index() + i32::try_from(r.below(60)).unwrap());
            let a = ANSWERS[usize::try_from(r.below(3)).unwrap()];
            m.answers.insert((if r.chance(1, 2) { "A" } else { "B" }.to_owned(), d), a);
        }
        for _ in 0..2_500 {
            let d = Day::from_index(first.index() + i32::try_from(r.below(100)).unwrap() - 5);
            let name = ["A", "B", "C"][usize::try_from(r.below(3)).unwrap()];
            let got = m.may_trade(name, d, first, last);
            let model = if d < first || d > last {
                Err(Refusal::OutsideWindow)
            } else if name == "C" {
                Ok(Answer::No)
            } else {
                let asked = if d > day(2026, 9, 30) { day(2026, 9, 30) } else { d };
                Ok(*m.answers.get(&(name.to_owned(), asked)).unwrap_or(&Answer::Yes))
            };
            assert_eq!(got, model, "{name} {d:?}");
        }
    }
}

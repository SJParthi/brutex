#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::market::Bar;
use crate::ruleset::{AUTHOR_V2, VIDEO_ORIGINAL, VIDEO_SCREEN};

/// A bank warmed on `n` flat closes at `p`, then one jump candle to `p + j`.
fn bank_after_jump(n: usize, p: i64, j: i64) -> Bank {
    let mut bank = Bank::new();
    for _ in 0..n {
        bank.step(Some(&Bar::flat(p, 1)), false);
    }
    bank.step(Some(&Bar::flat(p + j, 1)), false);
    bank
}

fn ctx(bank: &Bank, close: i64, prev: Option<AcceptedBar>) -> Context<'_> {
    Context {
        bank,
        close,
        prev,
        in_span: true,
        hole: false,
    }
}

#[test]
fn t_f29_a_jump_after_a_quiet_spell_signals_every_rule_set_on_its_side() {
    let flat = AcceptedBar::from_hlc(100_000, 100_000, 100_000);
    let up = bank_after_jump(300, 100_000, 1_000);
    for set in [VIDEO_ORIGINAL, AUTHOR_V2, VIDEO_SCREEN] {
        assert_eq!(
            evaluate(&set, &ctx(&up, 101_000, flat)),
            Some(Side::Long),
            "{}",
            set.name
        );
    }
    let down = bank_after_jump(300, 100_000, -1_000);
    for set in [VIDEO_ORIGINAL, AUTHOR_V2, VIDEO_SCREEN] {
        assert_eq!(
            evaluate(&set, &ctx(&down, 99_000, flat)),
            Some(Side::Short),
            "{}",
            set.name
        );
    }
}

/// T-F29-6: each condition false alone (here: the previous session missing) blocks.
#[test]
fn t_f29_one_condition_off_blocks_the_signal() {
    let up = bank_after_jump(300, 100_000, 1_000);
    for set in [VIDEO_ORIGINAL, AUTHOR_V2, VIDEO_SCREEN] {
        assert_eq!(
            evaluate(&set, &ctx(&up, 101_000, None)),
            None,
            "{}",
            set.name
        );
    }
    // a wide previous day (CPR 36.8 bp) whose R1 (100,133) is under the close: C1 false
    // for AV2 and VS, VO (no C1) still signals
    let wide = AcceptedBar::from_hlc(100_500, 99_000, 99_200);
    assert_eq!(
        evaluate(&VIDEO_ORIGINAL, &ctx(&up, 101_000, wide)),
        Some(Side::Long)
    );
    assert_eq!(evaluate(&AUTHOR_V2, &ctx(&up, 101_000, wide)), None);
}

/// X-F28: the gate at its edges.
#[test]
fn x_f28_gate_extremes() {
    let flat = AcceptedBar::from_hlc(100_000, 100_000, 100_000);
    let cold = bank_after_jump(30, 100_000, 1_000);
    // only the EMA blocks: AV2 and VS blocked, VO ready, same candle
    assert_eq!(
        ready_gate(&VIDEO_ORIGINAL, &ctx(&cold, 101_000, flat)),
        vec![]
    );
    assert_eq!(
        ready_gate(&AUTHOR_V2, &ctx(&cold, 101_000, flat)),
        vec![Blocker::Cold(Kind::Ema)]
    );
    assert_eq!(
        evaluate(&VIDEO_ORIGINAL, &ctx(&cold, 101_000, flat)),
        Some(Side::Long)
    );
    // outside the span and a hole: both listed, in order
    let mut c = ctx(&cold, 101_000, flat);
    c.in_span = false;
    c.hole = true;
    assert_eq!(
        ready_gate(&VIDEO_ORIGINAL, &c),
        vec![Blocker::OutsideSpan, Blocker::Hole]
    );
    assert_eq!(evaluate(&VIDEO_ORIGINAL, &c), None);
    // combo: all cold on the first candle
    let mut first = Bank::new();
    first.step(Some(&Bar::flat(100, 1)), false);
    let blockers = ready_gate(&AUTHOR_V2, &ctx(&first, 100, flat));
    assert_eq!(blockers.len(), 6);
    // combo: warm but tainted
    let mut tainted = bank_after_jump(300, 100_000, 1_000);
    tainted.step(None, true);
    tainted.step(Some(&Bar::flat(101_000, 1)), false);
    let b = ready_gate(&AUTHOR_V2, &ctx(&tainted, 101_000, flat));
    assert!(
        b.contains(&Blocker::Tainted(Kind::Supertrend)) && b.contains(&Blocker::Tainted(Kind::Ema))
    );
    assert_eq!(
        readings_used(&VIDEO_ORIGINAL),
        vec![Kind::Supertrend, Kind::Bollinger, Kind::Rsi]
    );
    assert_eq!(readings_used(&AUTHOR_V2).len(), 6);
}

/// T-F29-1..3: every condition mask on both sides, through `condition` directly: a
/// signal iff every condition holds on that side.
#[test]
fn t_f29_every_condition_reads_its_own_state() {
    let flat = AcceptedBar::from_hlc(100_000, 100_000, 100_000);
    let up = bank_after_jump(300, 100_000, 1_000);
    let c = ctx(&up, 101_000, flat);
    for set in [VIDEO_ORIGINAL, AUTHOR_V2, VIDEO_SCREEN] {
        for cond in set.conds {
            assert!(
                condition(cond, &c, Side::Long),
                "{} {}",
                set.name,
                cond.id()
            );
            let short = condition(cond, &c, Side::Short);
            // C1 (narrow CPR) and C8 is false short; the levels overlap only for C6 on AV2
            assert_eq!(
                short,
                matches!(cond, Cond::NarrowCpr { .. }),
                "{} {} short",
                set.name,
                cond.id()
            );
        }
    }
}

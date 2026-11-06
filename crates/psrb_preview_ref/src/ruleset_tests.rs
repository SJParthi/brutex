#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;

#[test]
fn t_the_three_tables_are_the_spec() {
    for set in RULE_SETS {
        assert_eq!(check(&set), Ok(()), "{}", set.name);
    }
    assert_eq!(supertrend_factor(&VIDEO_ORIGINAL), Some(3_000));
    assert_eq!(supertrend_factor(&AUTHOR_V2), Some(2_000));
    assert_eq!(supertrend_factor(&VIDEO_SCREEN), Some(2_000));
    assert_eq!(VIDEO_ORIGINAL.conds.len(), 4);
    assert_eq!(AUTHOR_V2.conds.len(), 8);
    // VIDEO-SCREEN differs from AUTHOR-V2 in C6's two levels only
    let differ: Vec<&str> = AUTHOR_V2
        .conds
        .iter()
        .zip(VIDEO_SCREEN.conds)
        .filter(|(a, b)| a != b)
        .map(|(a, _)| a.id())
        .collect();
    assert_eq!(differ, ["C6"]);
    let ids: Vec<&str> = AUTHOR_V2.conds.iter().map(|c| c.id()).collect();
    assert_eq!(ids, ["C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8"]);
    let readings: Vec<Option<Kind>> = AUTHOR_V2.conds.iter().map(|c| c.reading()).collect();
    assert_eq!(
        readings,
        [
            None,
            None,
            Some(Kind::Supertrend),
            Some(Kind::Bollinger),
            Some(Kind::Rsi),
            Some(Kind::WilliamsR),
            Some(Kind::Cci),
            Some(Kind::Ema)
        ]
    );
}

/// X-F22-8, X-F29-8: tables refused at load.
#[test]
fn x_tables_refused_at_load() {
    assert_eq!(
        check(&RuleSet {
            name: "EMPTY",
            short: "E",
            conds: &[]
        }),
        Err(Refusal::Empty)
    );
    assert_eq!(
        check(&RuleSet {
            name: "TWICE",
            short: "T",
            conds: &[Cond::Ema, Cond::Ema]
        }),
        Err(Refusal::Repeated("C8"))
    );
    let swapped = RuleSet {
        name: "AUTHOR-V2",
        short: "AV2",
        conds: &[
            Cond::NarrowCpr { bp: 20 },
            Cond::BeyondR1S1,
            Cond::Supertrend {
                factor_milli: 3_000,
            },
            Cond::Bollinger { width_milli: 3_000 },
            Cond::Rsi {
                long_ppm: 700_000,
                short_ppm: 300_000,
            },
            Cond::WilliamsR {
                long_ppm: -850_000,
                short_ppm: -150_000,
            },
            Cond::Cci {
                long_milli: 200_000,
                short_milli: -200_000,
            },
            Cond::Ema,
        ],
    };
    assert_eq!(check(&swapped), Err(Refusal::NotTheSpec("C3")));
    let short = RuleSet {
        name: "VIDEO-ORIGINAL",
        short: "VO",
        conds: &[Cond::BeyondR1S1],
    };
    assert_eq!(check(&short), Err(Refusal::NotTheSpec("length")));
    assert_eq!(
        check(&RuleSet {
            name: "OTHER",
            short: "O",
            conds: &[Cond::Ema]
        }),
        Ok(())
    );
    assert_eq!(
        supertrend_factor(&RuleSet {
            name: "NO-ST",
            short: "N",
            conds: &[Cond::Ema]
        }),
        None
    );
}

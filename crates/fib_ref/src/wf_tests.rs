use super::*;
use crate::model::Status;

fn t(month: i32, net: i128, hold: u32) -> SeriesTrade {
    SeriesTrade { month, net, hold }
}

const BAR_PLUS: i128 = 2_600_000_000 / 100; // Rs 2,60,000

#[test]
fn the_gate_matches_the_kits_boundaries() {
    assert!(clears(3, 0, 2_706));
    assert!(!clears(2, 0, 2_706));
    assert!(clears(6, 0, 5_412));
    assert!(!clears(5, 0, 5_412));
    assert!(clears(10, 0, 9_548));
    assert!(!clears(9, 0, 9_548));
    assert!(clears(677_853, 675_147, 5_412));
    assert!(!clears(677_852, 675_148, 5_412));
    assert!(!clears(0, 0, 0));
    assert!(!clears(5, 5, 0));
    assert!(!clears(4, 5, 0));
    assert!(clears(u32::MAX, u32::MAX - 1, 0));
    assert!(!clears(u32::MAX, u32::MAX - 1, 1));
    assert_eq!(ZSQ, [2_706, 5_412, 9_548]);
}

#[test]
fn the_luck_rule_clears_at_49_or_fewer_at_or_above() {
    let best: Vec<u32> = (0..1000).map(|b| if b < 49 { 80 } else { 10 }).collect();
    assert!(luck_clears(70, &best)); // 49 at or above: 50 x 20 = 1000
    let best50: Vec<u32> = (0..1000).map(|b| if b < 50 { 80 } else { 10 }).collect();
    assert!(!luck_clears(70, &best50)); // 51 x 20 = 1020
    assert!(luck_clears(81, &best50));
    assert!(luck_clears(0, &[]));
    assert!(!luck_clears(0, &[0; 1000]));
}

#[test]
fn passers_rank_by_drawdown_then_net() {
    let pass = |id, nets: &[i128]| Series {
        id,
        trades: nets
            .iter()
            .enumerate()
            .map(|(m, &n)| t(i32::try_from(m).unwrap_or(0), n, 60))
            .collect(),
    };
    let a = pass(1, &[BAR_PLUS + 58_882, BAR_PLUS + 58_882]);
    let b = pass(2, &[BAR_PLUS + 10_058_882, BAR_PLUS + 58_882]);
    let r = Readings::HEADLINE;
    let ka = key(&a, 0, 1, &r);
    let kb = key(&b, 0, 1, &r);
    assert!(ka.passer && kb.passer);
    assert_eq!(ka.at_bar, 2);
    // both have no drawdown beyond the data charge: equal tenths, more net wins
    assert_eq!(cmp_ranking(&kb, &ka), Ordering::Less);
    let miss = pass(3, &[BAR_PLUS * 10]);
    let km = key(&miss, 0, 1, &r); // month 1 has no trade
    assert!(!km.passer);
    assert_eq!(cmp_ranking(&ka, &km), Ordering::Less);
    assert_eq!(cmp_ranking(&km, &ka), Ordering::Greater);
}

#[test]
fn misses_rank_by_shortfall_then_worst_month() {
    let r = Readings::HEADLINE;
    let s = |id, trades: Vec<SeriesTrade>| Series { id, trades };
    let a = key(&s(1, vec![t(0, 100_000, 10), t(1, 100_000, 10)]), 0, 1, &r);
    let b = key(&s(2, vec![t(0, 200_000, 10)]), 0, 1, &r);
    // shortfall: a owes (bar - 41,118) x 2, b owes (bar - 141,118) + bar
    assert_eq!(a.shortfall, 2 * (i128::from(BAR) - 100_000 + 58_882));
    assert_eq!(
        b.shortfall,
        i128::from(BAR) - 200_000 + 58_882 + i128::from(BAR)
    );
    // b netted more in all (one data charge, not two): its shortfall is less
    assert_eq!(cmp_ranking(&b, &a), Ordering::Less);
    // equal shortfall: the higher worst month wins
    let c = key(&s(3, vec![t(0, 150_000, 10), t(1, 50_000, 10)]), 0, 1, &r);
    assert_eq!(c.shortfall, a.shortfall);
    assert_eq!(cmp_ranking(&a, &c), Ordering::Less);
    // an empty range is no passer
    let e = key(&s(4, vec![]), 5, 4, &r);
    assert!(!e.passer);
    assert_eq!(e.months, 0);
}

#[test]
fn keys_four_to_eight_break_ties_in_order() {
    let base = Key {
        passer: true,
        dd_tenths: 0,
        dd: 0,
        net: 100,
        wins: 1,
        trades: 2,
        hold: 100,
        shortfall: 0,
        worst: 0,
        at_bar: 1,
        months: 1,
        id: 1,
    };
    let rate = Key {
        wins: 2,
        id: 2,
        ..base
    };
    assert_eq!(cmp_ranking(&rate, &base), Ordering::Less);
    let none = Key {
        wins: 0,
        trades: 0,
        hold: 0,
        id: 3,
        ..base
    };
    assert_eq!(cmp_ranking(&base, &none), Ordering::Less);
    assert_eq!(cmp_ranking(&none, &base), Ordering::Greater);
    let none2 = Key { id: 4, ..none };
    assert_eq!(cmp_ranking(&none, &none2), Ordering::Less); // by id
    let fewer = Key {
        wins: 1,
        trades: 2,
        id: 5,
        ..base
    };
    let more = Key {
        wins: 2,
        trades: 4,
        id: 6,
        ..base
    };
    assert_eq!(cmp_ranking(&fewer, &more), Ordering::Less); // same rate, fewer trades
    let short = Key {
        hold: 50,
        id: 7,
        ..base
    };
    assert_eq!(cmp_ranking(&short, &base), Ordering::Less);
    let per = Key {
        net: 100,
        trades: 2,
        hold: 100,
        id: 8,
        ..base
    };
    assert_eq!(cmp_ranking(&base, &per), Ordering::Less); // all equal but id
    let dd = Key {
        dd_tenths: 1,
        id: 0,
        ..base
    };
    assert_eq!(cmp_ranking(&base, &dd), Ordering::Less);
    let rich = Key {
        net: 200,
        id: 9,
        ..base
    };
    assert_eq!(cmp_ranking(&rich, &base), Ordering::Less);
    // net per trade decides between equal-net... it cannot differ with equal net and trades
    let _ = per;
}

#[test]
fn windows_and_the_chain() {
    let w = windows();
    assert_eq!(w.len(), 7);
    assert_eq!(crate::clock::month_label(w[0].learn_first), "2018-09");
    assert_eq!(crate::clock::month_label(w[0].learn_last), "2019-12");
    assert_eq!(crate::clock::month_label(w[6].unseen_last), "2026-10");
    let unseen: i32 = w.iter().map(|x| x.unseen_last - x.unseen_first + 1).sum();
    assert_eq!(unseen, 82);
    // two candidates: A good before 2022, B after
    let mut a = Series {
        id: 1,
        trades: vec![],
    };
    let mut b = Series {
        id: 2,
        trades: vec![],
    };
    for m in w[0].learn_first..=w[6].unseen_last {
        let early = m < 2022 * 12;
        a.trades
            .push(t(m, if early { 40_000_000 } else { -100_000 }, 30));
        b.trades
            .push(t(m, if early { 1_000 } else { 40_000_000 }, 30));
    }
    let r = Readings::HEADLINE;
    let (picks, months, joined) = chain(&[a, b], &r);
    assert_eq!(picks[0], (2020, Some(1)));
    assert_eq!(picks.len(), 7);
    assert_eq!(months.len(), 82);
    assert_eq!(joined.trades.len(), 82);
    assert!(pick(&[], &w[0], &r).is_none());
    let (p, m, _) = chain(&[], &r);
    assert!(p.iter().all(|x| x.1.is_none()));
    assert!(m.iter().all(|x| x.trades == 0 && x.net == 0));
}

fn good_key() -> Key {
    Key {
        passer: true,
        dd_tenths: 0,
        dd: 0,
        net: 1,
        wins: 1,
        trades: 1,
        hold: 1,
        shortfall: 0,
        worst: 0,
        at_bar: 82,
        months: 82,
        id: 1,
    }
}

#[test]
fn the_verdict_rules_in_order() {
    let r = Readings::HEADLINE;
    let months: Vec<ChainMonth> = (0..82)
        .map(|m| ChainMonth {
            month: m,
            trades: 1,
            net: i128::from(BAR),
        })
        .collect();
    let below = Twin {
        key: Key {
            passer: false,
            id: 9,
            ..good_key()
        },
        whole: true,
    };
    let above = Twin {
        key: Key {
            id: 0,
            ..good_key()
        },
        whole: true,
    };
    let mut v = VerdictIn {
        months: None,
        key: good_key(),
        luck: true,
        twins: [None; 3],
        interim: false,
    };
    assert_eq!(verdict(&v, &r), Verdict::No("no headline"));
    let mut low = months.clone();
    low[40].net -= 1;
    v.months = Some(low);
    assert_eq!(verdict(&v, &r), Verdict::No("a month below the bar"));
    let mut empty = months.clone();
    empty[3].trades = 0;
    v.months = Some(empty);
    assert_eq!(verdict(&v, &r), Verdict::No("a month below the bar"));
    v.months = Some(months);
    v.luck = false;
    assert_eq!(verdict(&v, &r), Verdict::No("the sign test does not clear"));
    v.luck = true;
    assert_eq!(
        verdict(&v, &r),
        Verdict::Pending("fewer than three twin seeds have run")
    );
    v.twins[0] = Some(above);
    assert_eq!(
        verdict(&v, &r),
        Verdict::No("twin seed 1 at or above the headline")
    );
    v.twins[0] = Some(Twin {
        whole: false,
        ..above
    });
    assert_eq!(
        verdict(&v, &r),
        Verdict::No("twin seed 1 at or above the headline")
    );
    v.twins[0] = Some(Twin {
        whole: false,
        ..below
    });
    assert_eq!(
        verdict(&v, &r),
        Verdict::Pending("fewer than three twin seeds have run")
    );
    v.twins = [Some(below), Some(below), Some(below)];
    assert_eq!(verdict(&v, &r), Verdict::Yes);
    v.twins = [Some(below), Some(above), Some(below)];
    assert_eq!(
        verdict(&v, &r),
        Verdict::No("twin seed 2 or 3 at or above the headline")
    );
    v.twins = [Some(below), Some(above), None];
    assert_eq!(
        verdict(&v, &r),
        Verdict::Pending("fewer than three twin seeds have run")
    );
    // interim: always pending, or a NO may print
    v.twins = [Some(below), Some(below), Some(below)];
    v.interim = true;
    assert_eq!(verdict(&v, &r), Verdict::Pending("interim"));
    let mut r2 = r;
    r2.interim = InterimLine::NoCanPrint;
    assert_eq!(verdict(&v, &r2), Verdict::Pending("interim"));
    v.luck = false;
    assert_eq!(
        verdict(&v, &r2),
        Verdict::No("the sign test does not clear")
    );
    assert_eq!(verdict(&v, &r), Verdict::Pending("interim"));
    assert_eq!(Verdict::Yes.line(), "YES");
    assert_eq!(Verdict::No("x").line(), "NO (x)");
    assert_eq!(Verdict::Pending("y").line(), "PENDING (y)");
}

#[test]
fn whole_lots_round_down() {
    // Rs 2 lakh premium, lot 650 at Rs 30.00: 19,500 a lot: 10 lots (Rs 1,95,000)
    assert_eq!(option_lots(20_000_000, 650, 3_000), Ok(10));
    assert_eq!(option_lots(19_499_999, 650, 3_000), Ok(9));
    assert_eq!(option_lots(1_950_000, 650, 3_000), Ok(1));
    assert_eq!(
        option_lots(1_949_999, 650, 3_000),
        Err(Status::LotAbovePremium)
    );
    assert_eq!(option_lots(3_000_000, 650, 3_000), Ok(1)); // Rs 30,000 seed: one lot
    assert_eq!(option_lots(0, 1, 1), Err(Status::LotAbovePremium));
    assert_eq!(option_lots(5, 0, 0), Ok(5));
}

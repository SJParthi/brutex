use super::*;
use crate::clock::OPEN;

pub(crate) fn outcome(net: i128, end: u32) -> Outcome {
    Outcome {
        status: Status::Ok,
        filled: 100,
        first_fill: Some(end.saturating_sub(60)),
        end,
        net,
        charges: 40_000,
    }
}

pub(crate) fn signal(id: u32, sym: u32, date: Date, start: u32, net: i128, end: u32) -> Signal {
    Signal {
        id,
        sym,
        date,
        decided_at: start - 14,
        start,
        lev: Leverage::FIVE,
        outcomes: [outcome(net, end); 5],
    }
}

fn d(day: u32) -> Date {
    Date {
        y: 2026,
        m: 10,
        d: day,
    }
}

fn statuses(out: &AccountOut) -> Vec<Status> {
    out.admissions.iter().map(|a| a.status).collect()
}

#[test]
fn the_sixth_trade_of_a_day_is_refused_by_name() {
    let sigs: Vec<Signal> = (0..7)
        .map(|k| {
            signal(
                k,
                k,
                d(5),
                OPEN + 600 + 60 * k,
                100_000,
                OPEN + 620 + 60 * k,
            )
        })
        .collect();
    let out = run(&sigs, 5, &MAIN, &Readings::HEADLINE, i128::from(CAPITAL));
    assert_eq!(
        statuses(&out),
        [
            Status::Ok,
            Status::Ok,
            Status::Ok,
            Status::Ok,
            Status::Ok,
            Status::DayCap,
            Status::DayCap
        ]
    );
    assert_eq!(out.trades.len(), 5);
    let out_b = run(
        &sigs,
        10,
        &CEILING_B,
        &Readings::HEADLINE,
        i128::from(CAPITAL),
    );
    assert!(out_b.admissions.iter().all(|a| a.status == Status::Ok));
    assert_eq!(out_b.admissions[0].amount, Some(10 * LAKH));
}

#[test]
fn the_101st_of_a_month_is_refused_by_name() {
    let mut sigs = Vec::new();
    let mut id = 0;
    for day in 1..=25u32 {
        for k in 0..5u32 {
            sigs.push(signal(
                id,
                id,
                Date {
                    y: 2026,
                    m: 7,
                    d: day,
                },
                OPEN + 600 + 60 * k,
                10_000,
                OPEN + 620 + 60 * k,
            ));
            id += 1;
        }
    }
    let out = run(&sigs, 5, &MAIN, &Readings::HEADLINE, i128::from(CAPITAL));
    let ok = out
        .admissions
        .iter()
        .filter(|a| a.status == Status::Ok)
        .count();
    let capped = out
        .admissions
        .iter()
        .filter(|a| a.status == Status::MonthCap)
        .count();
    assert_eq!((ok, capped), (100, 25));
    assert_eq!(out.admissions[100].status, Status::MonthCap);
    assert_eq!(out.months.len(), 1);
    assert_eq!(out.months[0].trades, 100);
    assert_eq!(out.months[0].net, 100 * 10_000 - i128::from(DATA_CHARGE));
}

#[test]
fn one_position_per_stock_slots_and_the_lock_end() {
    let sigs = [
        signal(1, 7, d(5), OPEN + 600, 1, OPEN + 900),
        signal(2, 7, d(5), OPEN + 700, 1, OPEN + 800),
        signal(3, 8, d(5), OPEN + 900, 1, OPEN + 950),
    ];
    let out = run(&sigs, 1, &MAIN, &Readings::HEADLINE, i128::from(CAPITAL));
    assert_eq!(
        statuses(&out),
        [Status::Ok, Status::WhileOpen, Status::SlotsFull]
    );
    let mut r = Readings::HEADLINE;
    r.lock_end = LockEnd::Exclusive;
    let out = run(&sigs, 1, &MAIN, &r, i128::from(CAPITAL));
    assert_eq!(statuses(&out), [Status::Ok, Status::WhileOpen, Status::Ok]);
}

#[test]
fn leverage_quantises_the_amount_and_the_half_step_readings() {
    let three = Leverage { num: 1, den: 3 };
    assert_eq!(
        quantise(full_step(1, &MAIN), three, &MAIN, BelowHalf::Allowed),
        Ok(75 * LAKH / 10)
    );
    assert_eq!(
        quantise(
            full_step(1, &CEILING_B),
            three,
            &CEILING_B,
            BelowHalf::Allowed
        ),
        Ok(5 * LAKH)
    );
    assert_eq!(
        quantise(
            full_step(10, &MAIN),
            Leverage::FIVE,
            &MAIN,
            BelowHalf::Allowed
        ),
        Ok(10 * LAKH)
    );
    assert_eq!(
        quantise(
            full_step(20, &MAIN),
            Leverage::FIVE,
            &MAIN,
            BelowHalf::Allowed
        ),
        Ok(5 * LAKH)
    );
    let two = Leverage { num: 1, den: 2 };
    assert_eq!(
        quantise(full_step(1, &MAIN), two, &MAIN, BelowHalf::Allowed),
        Ok(5 * LAKH)
    );
    assert_eq!(
        quantise(full_step(1, &MAIN), two, &MAIN, BelowHalf::Refused),
        Err(Status::SizeRefused)
    );
    let one = Leverage { num: 1, den: 1 };
    assert_eq!(
        quantise(full_step(1, &MAIN), one, &MAIN, BelowHalf::Allowed),
        Ok(25 * LAKH / 10)
    );
    let none = Leverage { num: 2, den: 1 };
    assert_eq!(
        quantise(full_step(1, &MAIN), none, &MAIN, BelowHalf::Allowed),
        Err(Status::MarginCap)
    );
    assert_eq!(full_step(7, &MAIN), 142_857_142);
    assert_eq!(full_step(0, &MAIN), 15 * LAKH);
    assert_eq!(
        Leverage::from_var(2_345),
        Leverage {
            num: 2_345,
            den: 10_000
        }
    );
    assert_eq!(
        Leverage::from_var(1_000),
        Leverage {
            num: 2_000,
            den: 10_000
        }
    );
    assert_eq!(Leverage::FIVE.margin(15 * LAKH), i128::from(3 * LAKH));
    assert_eq!(Leverage { num: 1, den: 3 }.margin(10), 4);
}

#[test]
fn the_margin_check_steps_down_once_then_refuses() {
    let sigs: Vec<Signal> = (0..5)
        .map(|k| signal(k, k, d(6), OPEN + 600 + 60 * k, 0, OPEN + 10_000))
        .collect();
    // four open use Rs 12 lakh; Rs 13 lakh equity: 15 lakh needs 3, steps to 10 lakh needing 2: 14 > 13
    let out = run(&sigs, 5, &MAIN, &Readings::HEADLINE, 130_000_000);
    assert_eq!(out.admissions[3].amount, Some(15 * LAKH));
    assert_eq!(out.admissions[4].status, Status::MarginCap);
    let out = run(&sigs, 5, &MAIN, &Readings::HEADLINE, 140_000_000);
    assert_eq!(
        (out.admissions[4].status, out.admissions[4].amount),
        (Status::Ok, Some(10 * LAKH))
    );
    let one = [signal(9, 9, d(6), OPEN + 600, 0, OPEN + 700)];
    let out = run(&one, 1, &MAIN, &Readings::HEADLINE, 20_000_000);
    assert_eq!(
        (out.admissions[0].status, out.admissions[0].amount),
        (Status::Ok, Some(10 * LAKH))
    );
    let out = run(&one, 1, &MAIN, &Readings::HEADLINE, 10_000_000);
    assert_eq!(out.admissions[0].status, Status::MarginCap);
    // N = 20's half step is Rs 2.5 lakh: below it nothing is walked
    let tiny = [Signal {
        lev: Leverage { num: 1, den: 1 },
        ..one[0]
    }];
    let out = run(&tiny, 20, &MAIN, &Readings::HEADLINE, 100_000);
    assert_eq!(out.admissions[0].status, Status::SizeRefused);
    // N = 1 at 15 lakh steps to 10 lakh, then 7.5 lakh is not tried: one step only
    let out = run(&one, 1, &MAIN, &Readings::HEADLINE, 16_000_000);
    assert_eq!(out.admissions[0].status, Status::MarginCap);
}

#[test]
fn a_walk_refusal_admits_nothing_and_counts_nothing() {
    let refused = Signal {
        outcomes: [Outcome {
            status: Status::PriceAbovePosition,
            filled: 0,
            first_fill: None,
            end: OPEN + 700,
            net: 0,
            charges: 0,
        }; 5],
        ..signal(50, 50, d(8), OPEN + 600, 0, OPEN + 700)
    };
    let after = signal(51, 51, d(8), OPEN + 650, 10, OPEN + 900);
    let out = run(
        &[refused, after],
        1,
        &MAIN,
        &Readings::HEADLINE,
        i128::from(CAPITAL),
    );
    assert_eq!(statuses(&out), [Status::PriceAbovePosition, Status::Ok]);
    assert_eq!(out.trades.len(), 1);
}

#[test]
fn exposure_binds_only_beside_a_larger_position() {
    // N slots of the full step never pass Rs 1 crore (floor(1 crore / N) x N),
    // so one recipe's account cannot reach the exposure check's refusal;
    // a merged account can, as this state shows.
    let r = Readings::HEADLINE;
    let st = State {
        r: &r,
        equity: i128::from(10 * CRORE),
        peak: 0,
        max_dd: 0,
        open: vec![Open {
            id: 1,
            sym: 1,
            margin: 0,
            amount: 90 * LAKH,
            filled: true,
            day: 0,
            month: 0,
            net: 0,
        }],
        day_count: HashMap::new(),
        month_count: HashMap::new(),
        debit_at: HashMap::new(),
        debited: HashSet::new(),
        heap: BinaryHeap::new(),
    };
    let s = signal(2, 2, d(16), OPEN + 600, 0, OPEN + 700);
    // Rs 15 lakh passes Rs 1 crore; one step down, Rs 10 lakh, fits exactly
    assert_eq!(st.check(&s, 5, full_step(5, &MAIN), &MAIN), Ok(10 * LAKH));
    let st2 = State {
        open: vec![Open {
            amount: 95 * LAKH,
            ..st.open[0]
        }],
        ..st
    };
    assert_eq!(
        st2.check(&s, 5, full_step(5, &MAIN), &MAIN),
        Err(Status::ExposureCap)
    );
    // N = 12 at Rs 8.33 lakh quantises to Rs 7.5 lakh: twelve fit Rs 90 lakh
    let wide = Ceilings {
        day_cap: 30,
        month_cap: 100,
        amount: 15 * LAKH,
        margin: 15 * LAKH,
    };
    let sigs: Vec<Signal> = (0..13)
        .map(|k| signal(k, k, d(9), OPEN + 600 + 10 * k, 0, OPEN + 20_000))
        .collect();
    let out = run(&sigs, 12, &wide, &r, i128::from(30 * CRORE));
    assert!(
        out.admissions
            .iter()
            .take(12)
            .all(|a| a.amount == Some(75 * LAKH / 10))
    );
    assert_eq!(out.admissions[12].status, Status::SlotsFull);
}

#[test]
fn zero_fill_entries_release_their_count_under_the_headline() {
    let dead = Outcome {
        status: Status::Ok,
        filled: 0,
        first_fill: None,
        end: OPEN + 650,
        net: 0,
        charges: 0,
    };
    let mut sigs: Vec<Signal> = (0..5)
        .map(|k| Signal {
            outcomes: [dead; 5],
            ..signal(k, k, d(12), OPEN + 600 + k, 0, OPEN + 650)
        })
        .collect();
    sigs.push(signal(9, 9, d(12), OPEN + 700, 5_000, OPEN + 760));
    let head = run(&sigs, 5, &MAIN, &Readings::HEADLINE, i128::from(CAPITAL));
    assert_eq!(head.admissions[5].status, Status::Ok);
    assert_eq!(head.trades.len(), 1);
    let mut r = Readings::HEADLINE;
    r.zero_fill_count = ZeroFillCount::Kept;
    let kept = run(&sigs, 5, &MAIN, &r, i128::from(CAPITAL));
    assert_eq!(kept.admissions[5].status, Status::DayCap);
}

#[test]
fn months_drawdown_and_the_data_charge_readings() {
    let sigs = [
        signal(
            1,
            1,
            Date {
                y: 2026,
                m: 1,
                d: 5,
            },
            OPEN + 600,
            300_000,
            OPEN + 700,
        ),
        signal(
            2,
            2,
            Date {
                y: 2026,
                m: 1,
                d: 6,
            },
            OPEN + 600,
            -500_000,
            OPEN + 700,
        ),
        signal(
            3,
            3,
            Date {
                y: 2026,
                m: 2,
                d: 2,
            },
            OPEN + 600,
            100_000,
            OPEN + 700,
        ),
    ];
    let out = run(&sigs, 5, &MAIN, &Readings::HEADLINE, i128::from(CAPITAL));
    assert_eq!(out.months.len(), 2);
    assert_eq!(
        out.months[0].net,
        300_000 - 500_000 - i128::from(DATA_CHARGE)
    );
    assert_eq!(out.months[0].wins, 1);
    assert_eq!(out.months[0].worst, -500_000);
    // the curve: -58,882, +300,000, -500,000, -58,882, +100,000
    assert_eq!(out.max_dd, 500_000 + i128::from(DATA_CHARGE));
    assert_eq!(out.max_dd_tenths, 2);
    let mut r = Readings::HEADLINE;
    r.dd_round = DdRound::Ceil;
    assert_eq!(
        run(&sigs, 5, &MAIN, &r, i128::from(CAPITAL)).max_dd_tenths,
        3
    );
    r = Readings::HEADLINE;
    r.data_charge_at = DataChargeAt::OutOfCurve;
    let oc = run(&sigs, 5, &MAIN, &r, i128::from(CAPITAL));
    assert_eq!(oc.max_dd, 500_000);
    assert_eq!(oc.months[0].net, out.months[0].net);
    assert_eq!(
        out.end_equity,
        i128::from(CAPITAL) - 100_000 - 2 * i128::from(DATA_CHARGE)
    );
    assert_eq!(dd_tenths(0, DdRound::Ceil), 0);
    assert_eq!(dd_tenths(1, DdRound::Ceil), 1);
    assert_eq!(dd_tenths(199_999, DdRound::Floor), 0);
    assert_eq!(dd_tenths(200_000, DdRound::Floor), 1);
}

#[test]
fn an_earlier_fill_moves_the_months_debit_forward() {
    // the second admitted trade fills before the first one does
    let mut a = signal(1, 1, d(20), OPEN + 600, 1_000, OPEN + 3_000);
    a.outcomes = [Outcome {
        first_fill: Some(OPEN + 2_000),
        ..a.outcomes[0]
    }; 5];
    let mut b = signal(2, 2, d(20), OPEN + 700, 2_000, OPEN + 800);
    b.outcomes = [Outcome {
        first_fill: Some(OPEN + 720),
        ..b.outcomes[0]
    }; 5];
    let out = run(&[a, b], 5, &MAIN, &Readings::HEADLINE, i128::from(CAPITAL));
    assert_eq!(
        out.end_equity,
        i128::from(CAPITAL) + 3_000 - i128::from(DATA_CHARGE)
    );
    assert_eq!(out.max_dd, i128::from(DATA_CHARGE));
}

#[test]
fn the_half_step_readings_at_and_below_the_half() {
    // at 5x the full Rs 15 lakh is far above the half: allowed under both
    assert_eq!(
        quantise(
            full_step(1, &MAIN),
            Leverage::FIVE,
            &MAIN,
            BelowHalf::Refused
        ),
        Ok(15 * LAKH)
    );
    // at 3x Rs 7.5 lakh is exactly the half: allowed under both
    let three = Leverage { num: 1, den: 3 };
    assert_eq!(
        quantise(full_step(1, &MAIN), three, &MAIN, BelowHalf::Refused),
        Ok(75 * LAKH / 10)
    );
    // the margin check's step down from Rs 7.5 lakh lands on Rs 5 lakh, below
    // N = 1's half step: size_refused
    let s = [Signal {
        lev: three,
        ..signal(1, 1, d(17), OPEN + 600, 0, OPEN + 700)
    }];
    let out = run(&s, 1, &MAIN, &Readings::HEADLINE, 20_000_000);
    assert_eq!(out.admissions[0].status, Status::SizeRefused);
}

use super::*;

fn date(s: &str) -> Date {
    Date::parse(s).unwrap_or(Date {
        y: 2026,
        m: 1,
        d: 1,
    })
}

fn rt(qty: i64, buy: i64, sell: i64) -> Vec<Order> {
    vec![
        Order {
            side: OrderSide::Buy,
            fills: vec![(qty, buy)],
            square_off: false,
        },
        Order {
            side: OrderSide::Sell,
            fills: vec![(qty, sell)],
            square_off: false,
        },
    ]
}

#[test]
fn all_24_contract_notes_match_on_both_clocks_and_both_readings() {
    let r = Readings::HEADLINE;
    for (k, (d, orders, want)) in vectors::contract_notes().into_iter().enumerate() {
        let p = price_equity(date(d), &orders, &r);
        let got = [
            p.dated_p.total(),
            p.today_p.total(),
            p.dated_w.total(),
            p.today_w.total(),
        ];
        assert_eq!(got, want, "CN{:02} {d}", k + 1);
    }
}

#[test]
fn cn01_component_by_component() {
    let p = price_equity(
        date("2018-09-03"),
        &rt(400, 125_000, 126_250),
        &Readings::HEADLINE,
    );
    let d = p.dated_p;
    assert_eq!(
        (d.brokerage, d.exchange, d.sebi, d.stt, d.stamp, d.gst),
        (4_000, 3_266, 151, 12_600, 10_000, 1_308)
    );
    let t = p.today_p;
    assert_eq!(
        (t.exchange, t.sebi, t.stamp, t.gst),
        (3_085, 101, 1_500, 1_293)
    );
    assert_eq!(p.today_w.gst, 1_294);
    assert_eq!((p.dated_w.dp, p.dated_w.dp_gst), (1_250, 225));
    assert_eq!(p.headline(), 31_325);
    assert_eq!(p.stress(), 32_800);
}

#[test]
fn the_adani_screen_and_the_official_reading() {
    // 547 x Rs 2,742.00 bought and sold on 2026-11-07
    let orders = rt(547, 274_200, 274_200);
    let p = price_equity(date("2026-11-07"), &orders, &Readings::HEADLINE);
    assert_eq!(p.headline(), 57_941); // Rs 579.41: 25.1's passed-calculator figure
    assert_eq!(p.dated_p.total(), p.today_p.total());
    assert_eq!(p.today_w.total(), 59_417); // the addendum says 594.18 (on its 579.43)
    let (buy, sell) = estimator(149_987_400, 149_987_400, false);
    assert_eq!(buy, (2_000, 4_605, 1_189, 150, 4_500, 12_444)); // Rs 124.44
    assert_eq!(sell, (2_000, 4_605, 1_189, 150, 37_497, 45_441)); // Rs 454.41
    assert_eq!(buy.5 + sell.5, 57_885); // Rs 578.85 on Dhan's screen
}

#[test]
fn the_hdfcbank_770_ce_screen_and_the_option_reading() {
    let orders = rt(11_700, 2_695, 2_695);
    let (buy, sell) = estimator(31_531_500, 31_531_500, true);
    assert_eq!(buy, (2_000, 11_203, 2_377, 32, 946, 16_558)); // Rs 165.58, stamp 9.46
    assert_eq!(sell, (2_000, 11_203, 2_377, 32, 47_297, 62_909)); // Rs 629.09
    assert_eq!(buy.5 + sell.5, 79_467); // Rs 794.67
    let p = price_option(&orders, &Readings::HEADLINE);
    assert_eq!(p.total(), 79_433); // Rs 794.33: within Rs 1 of the screen
    assert_eq!((p.stt, p.stamp), (47_300, 900));
    let mut r = Readings::HEADLINE;
    r.option_brokerage = OptionBrokerage::Min20;
    assert_eq!(price_option(&orders, &r).total(), 79_433);
    // a Rs 30,000 premium differs between the two brokerage readings
    let small = rt(1_000, 3_000, 3_000);
    assert_eq!(price_option(&small, &Readings::HEADLINE).brokerage, 4_000);
    assert_eq!(price_option(&small, &r).brokerage, 1_800);
}

#[test]
fn the_state_stamp_readings_part_below_five_lakh() {
    assert_eq!(state_stamp(100_000_000, StateStamp::Stated), 10_000); // Rs 10 lakh: TS cap
    assert_eq!(state_stamp(100_000_000, StateStamp::Literal), 10_000);
    assert_eq!(state_stamp(40_000_000, StateStamp::Stated), 4_000); // Rs 4 lakh: TS Rs 40
    assert_eq!(state_stamp(40_000_000, StateStamp::Literal), 5_000); // HP Rs 50
    assert_eq!(state_stamp(200_000_000, StateStamp::Stated), 12_000); // Rs 20 lakh: TN
    assert_eq!(state_stamp(0, StateStamp::Literal), 5_000);
    // a Rs 2.5 lakh round trip on 2019-06-03: the dated clock's stamp
    let orders = rt(250, 99_960, 99_800);
    let head = price_equity(date("2019-06-03"), &orders, &Readings::HEADLINE);
    let mut r = Readings::HEADLINE;
    r.state_stamp = StateStamp::Literal;
    let lit = price_equity(date("2019-06-03"), &orders, &r);
    assert_eq!(head.dated_p.stamp, 4_994);
    assert_eq!(lit.dated_p.stamp, 5_000);
    assert_eq!(lit.dated_p.total() - head.dated_p.total(), 6);
}

#[test]
fn rates_by_date_cover_every_boundary() {
    assert_eq!(dated(date("2018-01-01")).nse, 32_500);
    assert_eq!(dated(date("2020-12-31")).nse, 32_500);
    assert_eq!(dated(date("2021-01-01")).nse, 34_500);
    assert_eq!(
        dated(date("2023-04-01")).nse + dated(date("2023-04-01")).ipft,
        33_500
    );
    assert_eq!(dated(date("2024-04-01")).nse, 32_200);
    assert_eq!(
        dated(date("2024-10-01")).nse + dated(date("2024-10-01")).ipft,
        30_700
    );
    assert_eq!(
        dated(date("2026-03-01")).nse + dated(date("2026-03-01")).ipft,
        30_700
    );
    assert_eq!(dated(date("2019-03-31")).sebi, 1_500);
    assert_eq!(dated(date("2019-04-01")).sebi, 1_000);
    assert_eq!(dated(date("2020-06-01")).sebi, 500);
    assert_eq!(dated(date("2021-04-01")).sebi, 1_000);
    assert!(!dated(date("2022-07-17")).gst_on_sebi);
    assert!(dated(date("2022-07-18")).gst_on_sebi);
    assert!(dated(date("2020-06-30")).state_stamp);
    assert!(!dated(date("2020-07-01")).state_stamp);
    assert_eq!(TODAY.nse + TODAY.ipft, 30_700);
}

#[test]
fn edge_orders_price_without_dividing_by_zero() {
    let empty: Vec<Order> = Vec::new();
    let p = price_equity(date("2026-10-30"), &empty, &Readings::HEADLINE);
    assert_eq!(p.headline(), 0);
    assert_eq!(p.stress(), 0); // no sell: no DP
    let buy_only = vec![Order {
        side: OrderSide::Buy,
        fills: vec![(10, 10_000)],
        square_off: false,
    }];
    let b = price_equity(date("2026-10-30"), &buy_only, &Readings::HEADLINE);
    assert_eq!(b.today_p.stt, 0);
    assert!(b.today_p.total() > 0);
    let zero_fill = vec![Order {
        side: OrderSide::Sell,
        fills: vec![(0, 10_000)],
        square_off: true,
    }];
    let z = price_equity(date("2026-10-30"), &zero_fill, &Readings::HEADLINE);
    assert_eq!(z.headline(), 0);
    assert_eq!(brokerage_of(66_666_666), 2_000);
    assert_eq!(brokerage_of(5_000_000), 1_500);
    assert_eq!(Breakdown::default().total(), 0);
}

#[test]
fn stt_is_on_the_smaller_quantity_at_the_days_vwap() {
    // COSTS.md section 5 step 4: 0.025% x min(BQTY, SQTY) x VWAP, to the
    // paisa, then the day's total to the rupee. Bought 100 at Rs 1,000, sold
    // 60 at Rs 1,010: VWAP Rs 1,003.75; 60 x 1,003.75 x 0.025% = Rs 15.05625,
    // Rs 15.06 to the paisa, Rs 15 to the rupee (100 shares would be Rs 25).
    let orders = vec![
        Order {
            side: OrderSide::Buy,
            fills: vec![(100, 100_000)],
            square_off: false,
        },
        Order {
            side: OrderSide::Sell,
            fills: vec![(60, 101_000)],
            square_off: false,
        },
    ];
    let r = Readings::HEADLINE;
    let d = date("2026-11-06");
    assert_eq!(official(d, &orders, &dated(d), &r).stt, 1_500);
    // the same with the sides swapped: still the smaller quantity
    let swapped = vec![
        Order {
            side: OrderSide::Buy,
            fills: vec![(60, 101_000)],
            square_off: false,
        },
        Order {
            side: OrderSide::Sell,
            fills: vec![(100, 100_000)],
            square_off: false,
        },
    ];
    assert_eq!(official(d, &swapped, &dated(d), &r).stt, 1_500);
}

#![cfg(test)]
//! Tests of the corporate-action lineage detector (D-3331, D-3343). Every
//! contract, lot and open interest here is synthetic; the shapes follow the
//! audit's measured events (NTPC x1.2, BRITANNIA x2, BAJFINANCE x10, CONCOR's
//! rounded lot) without copying a vendor row.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

fn d(y: u16, m: u8, dd: u8) -> Day {
    Day::new(y, m, dd).unwrap()
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

/// A contract-day with an inferred lot and the two OIs.
fn cd(expiry: Day, strike: i64, side: OptionSide, lot: u64, first: i64, close: i64) -> ContractDay {
    ContractDay {
        expiry,
        strike,
        side,
        lot: LotEvidence::Inferred(lot),
        first_oi: Some(first),
        close_oi: Some(close),
    }
}

fn seg(expiry: Day, strike: i64, side: OptionSide) -> String {
    cd(expiry, strike, side, 1, 0, 0)
        .contract()
        .unwrap()
        .to_string()
}

/// D-3343: the lot of a contract-day is the greatest common divisor of its
/// traded quantities, the LTQ = 1 marker rows that lead a file left out.
/// Before D-3343 it was the smallest quantity, and a day that traded 2 and 3
/// lots (200 and 300 at a lot of 100) read no lot at all.
#[test]
fn the_lot_is_the_common_divisor_of_the_traded_quantities_less_the_leading_markers() {
    assert_eq!(
        LotEvidence::of_quantities(&[0, 4_000, 8_000, 4_000, 12_000]),
        LotEvidence::Inferred(4_000)
    );
    assert_eq!(
        LotEvidence::of_quantities(&[200, 300]),
        LotEvidence::Inferred(100),
        "two and three lots: the minimum would have read 200, or nothing"
    );
    assert_eq!(
        LotEvidence::of_quantities(&[150, 450]),
        LotEvidence::Inferred(150),
        "an upper bound: one lot traded alone is the lot"
    );
    // The 30 May 2019 shape: LTQ = 1 marker rows lead the file. They are not
    // a lot of one share.
    assert_eq!(
        LotEvidence::of_quantities(&[1, 0, 1, 75, 150]),
        LotEvidence::Inferred(75)
    );
    // An LTQ of 1 after a real quantity is a quantity, not a marker.
    assert_eq!(
        LotEvidence::of_quantities(&[75, 1]),
        LotEvidence::Inferred(1)
    );
    assert_eq!(
        LotEvidence::of_quantities(&[1, 1, 0]),
        LotEvidence::MarkersOnly { rows: 2 }
    );
    assert_eq!(LotEvidence::of_quantities(&[0, 0]), LotEvidence::NoTrade);
    assert_eq!(LotEvidence::of_quantities(&[]), LotEvidence::NoTrade);
    let lot = LotEvidence::Inferred(4_800);
    assert_eq!(lot.lot(), Some(4_800));
    assert_eq!(lot.lots(9_600), Some(2), "whole lots only");
    assert_eq!(lot.lots(4_000), None, "never a fraction of a lot");
    assert_eq!(LotEvidence::NoTrade.lot(), None);
    assert_eq!(LotEvidence::MarkersOnly { rows: 3 }.lot(), None);
    assert_eq!(LotEvidence::MarkersOnly { rows: 3 }.lots(9), None);
    assert_eq!(LotEvidence::NoTrade.lots(9), None);
    assert_eq!(
        (LotEvidence::LABEL, LotEvidence::BASIS),
        ("inferred", "ltq_gcd")
    );
    assert_eq!((gcd(0, 7), gcd(7, 0), gcd(12, 18), gcd(0, 0)), (7, 7, 6, 0));
}

#[test]
fn a_contract_day_reads_its_lot_and_its_first_and_closing_oi_from_its_rows() {
    let on = d(2019, 3, 19);
    let ticker = crate::gdfl_nfo::decode_ticker("NTPC28MAR19125PE", on).unwrap();
    // File order, with a late row: two rows share the earliest second 33,325
    // (the first of them opens), and two the latest second 55,000 (the last
    // of them closes).
    let rows = [
        (33_400, 0, 1_992_100),
        (33_325, 4_800, 1_992_000),
        (33_325, 0, 1_992_050),
        (55_000, 9_600, 2_000_000),
        (55_000, 0, 2_000_400),
        (40_000, 4_800, 1_995_000),
    ];
    let day = ContractDay::from_rows(&ticker, rows);
    assert_eq!(day.lot, LotEvidence::Inferred(4_800));
    assert_eq!(day.first_oi, Some(1_992_000));
    assert_eq!(day.close_oi, Some(2_000_400));
    assert_eq!(
        day.contract().unwrap().as_str(),
        "2019-03-28-12500-PE",
        "keyed on the settled expiry and the stated strike"
    );
    // Quantities of 4,800 and 7,200 (one lot and one and a half of it): the
    // common divisor, 2,400, an upper bound the true lot divides.
    let odd = ContractDay::from_rows(&ticker, [(1, 4_800, 5), (2, 7_200, 5)]);
    assert_eq!(odd.lot, LotEvidence::Inferred(2_400));
    // A leading marker row is left out of the lot but not of the OI.
    let marked = ContractDay::from_rows(&ticker, [(7, 1, 11), (9, 4_800, 12)]);
    assert_eq!(
        (marked.lot, marked.first_oi, marked.close_oi),
        (LotEvidence::Inferred(4_800), Some(11), Some(12))
    );
    let none = ContractDay::from_rows(&ticker, std::iter::empty::<(u32, u64, i64)>());
    assert_eq!(
        (none.lot, none.first_oi, none.close_oi),
        (LotEvidence::NoTrade, None, None)
    );
}

#[test]
fn the_day_lot_is_the_most_common_contract_lot_and_a_tie_is_none() {
    let e = d(2019, 3, 28);
    let put = OptionSide::Put;
    let day = [
        cd(e, 10_000, put, 4_000, 1, 1),
        cd(e, 10_500, put, 4_000, 1, 1),
        cd(e, 11_000, put, 8_000, 1, 1),
    ];
    assert_eq!(day_lot(&day), Some(4_000));
    assert_eq!(day_lot(&day[1..]), None, "one each: a tie is no lot");
    assert_eq!(day_lot(&[]), None);
    let mut silent = day[0];
    silent.lot = LotEvidence::NoTrade;
    assert_eq!(day_lot(&[silent]), None);
}

/// The NTPC shape of DB-G71: a factor of 1.2 (lot 4,000 to 4,800). Every
/// pre-ex strike is renamed to strike / 1.2 on the 0.05 grid, and where that
/// text equals another pre-ex strike (the old `150PE` becomes `125PE`, a text
/// the old `125PE` also had) the record says the series is stitched.
#[test]
fn a_bonus_renames_every_strike_by_the_lot_step_and_names_the_reused_texts() {
    let (e, put) = (d(2019, 3, 28), OptionSide::Put);
    let strikes: Vec<i64> = (0..=16).map(|i| 10_000 + i * 500).collect(); // 100..180
    let before: Vec<ContractDay> = strikes
        .iter()
        .map(|&k| cd(e, k, put, 4_000, 10_000, 1_692_000))
        .collect();
    let after: Vec<ContractDay> = strikes
        .iter()
        .map(|&k| {
            let new = predicted(k, 4_000, 4_800).unwrap();
            cd(e, new, put, 4_800, 1_992_000, 1_992_000)
        })
        .collect();
    let event = detect(sym("NTPC"), d(2019, 3, 19), &before, &after).expect("an adjustment");
    assert_eq!((event.lot_before, event.lot_after), (4_000, 4_800));
    assert_eq!(event.lineage.len(), strikes.len(), "every contract paired");
    assert!(event.unpaired.is_empty());
    let of = |old: i64| {
        event
            .lineage
            .iter()
            .find(|l| l.old.as_str() == seg(e, old, put))
            .unwrap()
    };
    let stitched = of(15_000);
    assert_eq!(stitched.new.as_str(), "2019-03-28-12500-PE");
    assert!(stitched.reused, "the old 125PE text named another contract");
    assert_eq!(stitched.oi, OiCarry::Scaled, "1,692,000 x 1.2 is 2,030,400");
    let fresh = of(12_500);
    assert_eq!(
        fresh.new.as_str(),
        "2019-03-28-10415-PE",
        "104.1666 to 104.15"
    );
    assert!(!fresh.reused);
    assert_eq!(fresh.underlying.as_str(), "NTPC");
    assert_eq!(fresh.ex_date, d(2019, 3, 19));
    // Every post-ex key a pre-ex contract also carried is a stitched series.
    // Every pre-ex key is retired: renamed (all seventeen here) or stopped.
    let retired = event.retired();
    assert_eq!(retired.len(), strikes.len());
    assert!(retired.contains(&Contract::parse("2019-03-28-12500-PE").unwrap()));
    assert!(!retired.contains(&Contract::parse("2019-03-28-10415-PE").unwrap()));
    let stitched: Vec<&str> = event.stitched.iter().map(Contract::as_str).collect();
    assert_eq!(
        stitched,
        [
            "2019-03-28-10000-PE",
            "2019-03-28-12500-PE",
            "2019-03-28-15000-PE"
        ]
    );
}

/// The BRITANNIA shape of DB-G72: a factor of 2. The pre-ex `6000CE` series
/// stops the day before the ex-date and continues as `3000CE`; the record
/// links them.
#[test]
fn a_split_links_the_pre_ex_series_to_the_one_it_continues_as() {
    let (e, call) = (d(2018, 12, 27), OptionSide::Call);
    let before = [
        cd(e, 600_000, call, 100, 500, 500),
        cd(e, 630_000, call, 100, 1_500, 1_500),
    ];
    let after = [
        cd(e, 300_000, call, 200, 1_000, 1_200),
        cd(e, 315_000, call, 200, 3_200, 3_000),
    ];
    let event = detect(sym("BRITANNIA"), d(2018, 11, 29), &before, &after).unwrap();
    let pairs: Vec<(String, String)> = event
        .lineage
        .iter()
        .map(|l| (l.old.to_string(), l.new.to_string()))
        .collect();
    assert_eq!(
        pairs,
        [
            (
                "2018-12-27-600000-CE".to_owned(),
                "2018-12-27-300000-CE".to_owned()
            ),
            (
                "2018-12-27-630000-CE".to_owned(),
                "2018-12-27-315000-CE".to_owned()
            ),
        ]
    );
    assert!(event.lineage.iter().all(|l| !l.reused));
    assert!(
        event.stitched.is_empty(),
        "no post-ex key reuses a pre-ex text"
    );
    assert!(
        event.lineage.iter().all(|l| l.oi == OiCarry::Scaled),
        "500 to 1,000 and 1,500 to 3,200 scale by 2"
    );
}

/// DB-G77: the BAJFINANCE x10 shape. The first post-ex OI equals the pre-ex
/// close OI exactly, so it is the vendor's unadjusted value; the ex-date rows
/// that carry it are found, and the record names the second the adjustment
/// lands, up to which a bar's OI change is the vendor's fault.
#[test]
fn ex_date_oi_the_vendor_left_unscaled_is_named_and_its_rows_counted() {
    let (e, put) = (d(2025, 6, 26), OptionSide::Put);
    let ex = d(2025, 6, 16);
    let before = [
        cd(e, 950_000, put, 125, 150_000, 162_125),
        cd(e, 900_000, put, 125, 100_000, 124_250),
    ];
    let after = [
        cd(e, 95_000, put, 1_250, 162_125, 1_751_250),
        cd(e, 90_000, put, 1_250, 1_238_750, 1_240_000),
    ];
    let event = detect(sym("BAJFINANCE"), ex, &before, &after).unwrap();
    let (stale, scaled) = (&event.lineage[0], &event.lineage[1]);
    assert_eq!(stale.oi, OiCarry::Unadjusted);
    assert_eq!(stale.oi.as_str(), "vendor_oi_unadjusted");
    assert_eq!(stale.pre_close_oi, Some(162_125));
    assert_eq!(scaled.oi, OiCarry::Scaled);
    assert_eq!(stale.factor(), (10, 1));
    // 09:15:31 to 13:53:39 at the stale value (one row drifted 1% by a trade
    // the vendor did not scale either), the adjusted value from 13:53:40.
    let rows = [
        (33_331, 162_125),
        (40_000, 163_750),
        (50_019, 162_125),
        (50_020, 1_751_250),
        (50_100, 1_752_500),
    ];
    assert_eq!(
        stale.stale_oi(&rows),
        StaleOi {
            rows: 3,
            until_sod: Some(50_020)
        }
    );
    // A stale row served AFTER an adjusted one (the vendor flipping back)
    // moves the bound to the first adjusted row after it.
    let flip = [
        (33_331, 162_125),
        (50_020, 1_751_250),
        (50_100, 162_125),
        (50_200, 1_751_300),
    ];
    assert_eq!(
        stale.stale_oi(&flip),
        StaleOi {
            rows: 2,
            until_sod: Some(50_200)
        }
    );
    // Every row stale: the bound is the last of them.
    assert_eq!(
        stale.stale_oi(&[(33_331, 162_125), (36_000, 162_125)]),
        StaleOi {
            rows: 2,
            until_sod: Some(36_000)
        }
    );
    let none = StaleOi {
        rows: 0,
        until_sod: None,
    };
    assert_eq!(stale.stale_oi(&[]), none);
    assert_eq!(
        stale.stale_oi(&[(33_331, 1_751_250)]),
        none,
        "adjusted at once"
    );
    // A pair whose OI carried scaled reads no row.
    assert_eq!(scaled.stale_oi(&rows), none);
    // The record carries the bound as a unix second: 2025-06-16 13:53:40 IST
    // is 08:23:40 UTC.
    let record = stale.record(&rows);
    assert_eq!(record.oi_unadjusted_until, 1_750_062_220);
    assert_eq!(
        record.line(),
        "lineage NSE-FNO-BAJFINANCE-2025-06-26-950000-PE NSE-FNO-BAJFINANCE-2025-06-26-95000-PE ex=2025-06-16 factor=10/1 lot_old=125 lot_new=1250 oi_unadjusted_until=1750062220 v=1"
    );
    assert_eq!(scaled.record(&rows).oi_unadjusted_until, 0);
    assert_eq!(unix_second(d(1970, 1, 1), 19_800), 0);
}

/// A lot REVISION (no corporate action) keeps the strikes: most pre-ex keys
/// continue, so nothing is declared even if a strike happens to line up.
#[test]
fn a_lot_revision_that_keeps_the_strikes_is_not_an_adjustment() {
    let (e, call) = (d(2024, 7, 25), OptionSide::Call);
    // 1,200 to 1,100 (12/11): 110 x 12/11 = 120 lines up with a real strike.
    let strikes = [10_000, 10_500, 11_000, 11_500, 12_000, 12_500];
    let before: Vec<ContractDay> = strikes
        .iter()
        .map(|&k| cd(e, k, call, 1_200, 9, 9))
        .collect();
    let after: Vec<ContractDay> = strikes
        .iter()
        .map(|&k| cd(e, k, call, 1_100, 9, 9))
        .collect();
    assert_eq!(predicted(11_000, 1_200, 1_100), Some(12_000));
    assert_eq!(detect(sym("ACC"), d(2024, 7, 1), &before, &after), None);
    // The same lots on both days: not even looked at.
    assert_eq!(detect(sym("ACC"), d(2024, 7, 1), &before, &before), None);
    // A day whose lot cannot be read: none.
    assert_eq!(detect(sym("ACC"), d(2024, 7, 1), &before, &[]), None);
}

/// A day whose quantities read a wrong lot (sub-lot marker rows of one share)
/// predicts strikes thousands of times off the ladder: nothing pairs, so
/// nothing is declared.
#[test]
fn a_wrong_lot_from_marker_rows_declares_nothing() {
    let (e, call) = (d(2019, 6, 27), OptionSide::Call);
    let before = [
        cd(e, 1_200_000, call, 75, 1, 1),
        cd(e, 1_210_000, call, 75, 1, 1),
    ];
    let after = [
        cd(e, 1_200_000, call, 1, 1, 1),
        cd(e, 1_210_000, call, 1, 1, 1),
    ];
    assert_eq!(detect(sym("NIFTY"), d(2019, 5, 30), &before, &after), None);
}

/// A lot rounded to a whole share moves the prediction off the grid
/// (CONCOR's 1,250 to 1,563 for a factor of 1.25: 600 / 1.2504 = 479.85);
/// the vendor's 480 still pairs, and two candidates equally close pair
/// neither.
#[test]
fn a_rounded_lot_still_pairs_and_an_ambiguous_partner_is_never_guessed() {
    let (e, call) = (d(2019, 6, 27), OptionSide::Call);
    assert_eq!(predicted(60_000, 1_250, 1_563), Some(47_985));
    let before = [
        cd(e, 60_000, call, 1_250, 7, 7),
        cd(e, 62_000, call, 1_250, 7, 7),
    ];
    let after = [
        cd(e, 48_000, call, 1_563, 7, 7),
        // 620 / 1.2504 = 495.84: 495.80 and 495.90 are both within tolerance.
        cd(e, 49_580, call, 1_563, 7, 7),
        cd(e, 49_590, call, 1_563, 7, 7),
    ];
    let event = detect(sym("CONCOR"), d(2019, 6, 20), &before, &after).unwrap();
    assert_eq!(event.lineage.len(), 1);
    assert_eq!(event.lineage[0].new.as_str(), "2019-06-27-48000-CE");
    assert_eq!(event.unpaired.len(), 1);
    assert_eq!(event.unpaired[0].as_str(), "2019-06-27-62000-CE");
    assert_eq!(event.retired().len(), 2, "the paired and the unpaired");
    // A pre-ex contract whose expiry has no post-ex ladder is unpaired too.
    let other = [cd(d(2019, 7, 25), 60_000, call, 1_250, 7, 7)];
    let none = detect(
        sym("CONCOR"),
        d(2019, 6, 20),
        &[before[0], other[0]],
        &after,
    )
    .unwrap();
    assert_eq!(none.unpaired.len(), 1);
}

#[test]
fn the_oi_carry_names_each_case() {
    let carry = |pre, post| OiCarry::of(pre, post, 4_000, 4_800);
    assert_eq!(carry(Some(1_000_000), Some(1_190_000)), OiCarry::Scaled);
    assert_eq!(carry(Some(1_000_000), Some(1_005_000)), OiCarry::Unadjusted);
    // At x1.2 the gap is 20%, so an eighth of it (2.5%) bounds "unscaled".
    assert_eq!(carry(Some(1_000_000), Some(1_025_000)), OiCarry::Unadjusted);
    assert_eq!(
        carry(Some(1_000_000), Some(1_028_000)),
        OiCarry::Unexplained,
        "2.8% over the close at x1.2 is not called stale"
    );
    // At x10 the 5% bound applies.
    let ten = |pre, post| OiCarry::of(pre, post, 125, 1_250);
    assert_eq!(ten(Some(1_000_000), Some(973_000)), OiCarry::Unadjusted);
    assert_eq!(ten(Some(1_000_000), Some(1_050_000)), OiCarry::Unadjusted);
    assert_eq!(
        ten(Some(1_000_000), Some(1_060_000)),
        OiCarry::Unexplained,
        "6% off the unscaled value and nearer it than the scaled one"
    );
    assert_eq!(ten(Some(1_000_000), Some(9_800_000)), OiCarry::Scaled);
    assert_eq!(
        carry(Some(1_000_000), Some(3_000_000)),
        OiCarry::Unexplained
    );
    assert_eq!(carry(None, Some(1)), OiCarry::Missing);
    assert_eq!(carry(Some(1), None), OiCarry::Missing);
    assert_eq!(carry(Some(0), Some(1)), OiCarry::Missing);
    assert_eq!(OiCarry::of(Some(5), Some(5), 0, 5), OiCarry::Missing);
    // A step under 5% cannot be told from no step.
    assert_eq!(
        OiCarry::of(Some(1_000), Some(1_000), 1_000, 1_020),
        OiCarry::Indistinct
    );
    for carry in [
        OiCarry::Scaled,
        OiCarry::Unadjusted,
        OiCarry::Indistinct,
        OiCarry::Unexplained,
        OiCarry::Missing,
    ] {
        assert_eq!(OiCarry::parse(carry.as_str()), Some(carry));
    }
    assert_eq!(OiCarry::parse("scaled "), None);
}

/// D-3331 decision 3: the `lineage` record, in D-3317's line rules, reads
/// back exactly, and anything else is refused, never repaired.
#[test]
fn a_lineage_record_is_one_line_that_reads_back_exactly() {
    let pair = Lineage {
        underlying: sym("NTPC"),
        ex_date: d(2019, 3, 19),
        old: Contract::parse("2019-03-28-15000-PE").unwrap(),
        new: Contract::parse("2019-03-28-12500-PE").unwrap(),
        lot_before: 4_000,
        lot_after: 4_800,
        pre_close_oi: Some(1_692_000),
        oi: OiCarry::Scaled,
        reused: true,
    };
    assert_eq!(pair.factor(), (6, 5));
    let record = pair.record(&[]);
    let line = record.line();
    assert_eq!(
        line,
        "lineage NSE-FNO-NTPC-2019-03-28-15000-PE NSE-FNO-NTPC-2019-03-28-12500-PE ex=2019-03-19 factor=6/5 lot_old=4000 lot_new=4800 oi_unadjusted_until=0 v=1"
    );
    assert_eq!(LineageRecord::parse(&line), Some(record));
    // A dash in the underlying reads back whole.
    let dashed = LineageRecord {
        underlying: sym("BAJAJ-AUTO"),
        ..record
    };
    assert_eq!(LineageRecord::parse(&dashed.line()), Some(dashed));
    for bad in [
        line.replacen("lineage", "Lineage", 1),
        line.replace(" v=1", " v=2"),
        line.replace(" v=1", ""),
        line.replace("factor=6/5", "factor=12/10"),
        line.replace("factor=6/5", "factor=6"),
        line.replace("lot_old=4000", "lot_old=+4000"),
        line.replace("lot_old=4000", "lot_old="),
        line.replace("lot_old=4000 lot_new=4800", "lot_new=4800 lot_old=4000"),
        line.replace("ex=2019-03-19", "ex=2019-3-19"),
        line.replace("ex=2019-03-19", "ex=2019-02-30"),
        line.replace("ex=", "exdate="),
        line.replace("oi_unadjusted_until=0", "oi_unadjusted_until=-1"),
        line.replace(
            "NSE-FNO-NTPC-2019-03-28-12500-PE",
            "NSE-FNO-GAIL-2019-03-28-12500-PE",
        ),
        line.replace("NSE-FNO-NTPC-2019-03-28-15000-PE", "NSE-CASH-NTPC"),
        line.replace("NTPC", "ntpc"),
        line.replace(' ', "  "),
        line.replace(' ', "\t"),
        format!("{line} extra=1"),
        String::new(),
    ] {
        assert_eq!(LineageRecord::parse(&bad), None, "{bad:?}");
    }
}

/// The two ledgers sit beside the import journal, under the names D-3331
/// and D-3343 give them.
#[test]
fn the_ledgers_are_the_files_the_decisions_name() {
    let root = std::path::Path::new("/store");
    assert_eq!(
        lineage_path(root),
        std::path::Path::new("/store/imports/gdfl.lineage")
    );
    assert_eq!(
        lots_path(root),
        std::path::Path::new("/store/imports/gdfl.lots")
    );
    assert_eq!(
        lineage_path(root).parent(),
        crate::gdfl_import::journal_path(root).parent()
    );
}

/// The census's instrument text, read back; anything else is refused.
#[test]
fn an_option_key_is_the_census_text_and_reads_back_exactly() {
    let c = Contract::parse("2019-03-28-12500-PE").unwrap();
    assert_eq!(
        option_key(sym("NTPC"), c),
        "NSE-FNO-NTPC-2019-03-28-12500-PE"
    );
    assert_eq!(
        parse_option_key("NSE-FNO-NTPC-2019-03-28-12500-PE"),
        Some((sym("NTPC"), c))
    );
    assert_eq!(
        parse_option_key("NSE-FNO-NAM-INDIA-2019-03-28-12500-CE")
            .map(|(u, k)| (u.as_str().to_owned(), k.as_str().to_owned())),
        Some(("NAM-INDIA".to_owned(), "2019-03-28-12500-CE".to_owned()))
    );
    for bad in [
        "NSE-FNO-NTPC-2019-03-28-FUT",
        "NSE-FNO-NTPC-2019-03-28-12500-XE",
        "NSE-FNO-NTPC-2019-03-28-012500-PE",
        "NSE-FNO-NTPC-2019-02-30-12500-PE",
        "NSE-FNO-NTPC-19-03-28-12500-PE",
        "NSE-FNO-ntpc-2019-03-28-12500-PE",
        "NSE-FNO--2019-03-28-12500-PE",
        "NSE-FNO-2019-03-28-12500-PE",
        "NSE-INDEX-NTPC-2019-03-28-12500-PE",
        "BSE-FNO-NTPC-2019-03-28-12500-PE",
        "",
    ] {
        assert_eq!(parse_option_key(bad), None, "{bad:?}");
    }
    assert_eq!(parse_day("2019-03-28"), Some(d(2019, 3, 28)));
    for bad in [
        "2019-3-28",
        "2019-03",
        "x019-03-28",
        "2019-13-01",
        "201903-28",
    ] {
        assert_eq!(parse_day(bad), None, "{bad}");
    }
}

/// D-3331 decision 3: a reused key's `stitched` record names where its
/// pre-ex contract went and which pre-ex contract was renamed onto it.
#[test]
fn a_stitched_record_links_both_contracts_of_a_reused_key_and_reads_back() {
    let (e, put) = (d(2019, 3, 28), OptionSide::Put);
    // 150 -> 125 (reusing 125's text), 125 -> 104.15, 140 -> 116.65; 100
    // stops trading, and a newly listed 100 trades under its retired text.
    let before = [
        cd(e, 15_000, put, 4_000, 1, 1_000),
        cd(e, 12_500, put, 4_000, 1, 1_000),
        cd(e, 14_000, put, 4_000, 1, 1_000),
        cd(e, 10_000, put, 4_000, 1, 1_000),
    ];
    let after = [
        cd(e, 12_500, put, 4_800, 1_200, 1_200),
        cd(e, 10_415, put, 4_800, 1_200, 1_200),
        cd(e, 11_665, put, 4_800, 1_200, 1_200),
        cd(e, 10_000, put, 4_800, 1_200, 1_200),
    ];
    let event = detect(sym("NTPC"), d(2019, 3, 19), &before, &after).unwrap();
    let k = |strike| Contract::parse(&seg(e, strike, put)).unwrap();
    let records = event.stitched_records();
    assert_eq!(records.len(), 2, "{records:?}");
    let reused = event.stitched_record(k(12_500)).unwrap();
    assert_eq!(
        (reused.before, reused.after),
        (Some(k(10_415)), Some(k(15_000))),
        "125's own contract continues as 104.15; the old 150 now trades as 125"
    );
    assert!(records.contains(&reused));
    // 100 predicts 83.35, where no post-ex strike lies, so it is unpaired;
    // the 100 under its retired text is a newly listed contract.
    let listed = event.stitched_record(k(10_000)).unwrap();
    assert_eq!((listed.before, listed.after), (None, None));
    // 150 was retired too, though not listed again on the ex-date.
    assert_eq!(
        event.stitched_record(k(15_000)).unwrap().before,
        Some(k(12_500))
    );
    assert_eq!(event.stitched_record(k(10_415)), None, "not a retired key");
    let line = reused.line();
    assert_eq!(
        line,
        "stitched NSE-FNO-NTPC-2019-03-28-12500-PE ex=2019-03-19 before=2019-03-28-10415-PE after=2019-03-28-15000-PE v=1"
    );
    assert_eq!(StitchedRecord::parse(&line), Some(reused));
    let dashes = listed.line();
    assert_eq!(
        dashes,
        "stitched NSE-FNO-NTPC-2019-03-28-10000-PE ex=2019-03-19 before=- after=- v=1"
    );
    assert_eq!(StitchedRecord::parse(&dashes), Some(listed));
    for bad in [
        line.replace("stitched", "stitch"),
        line.replace("before=2019-03-28-10415-PE", "before=2019-03-28-10415-FUT"),
        line.replace("after=2019-03-28-15000-PE", "after="),
        line.replace("after=2019-03-28-15000-PE", "after=--"),
        line.replace(" v=1", " v=0"),
        line.replace("ex=2019-03-19 ", ""),
        format!("{line} "),
    ] {
        assert_eq!(StitchedRecord::parse(&bad), None, "{bad:?}");
    }
}

/// D-3343: a lot is recorded when first seen and when a later day lowers it,
/// to the divisor both days share; an unchanged or higher day writes nothing,
/// and a key reused at an ex-date starts again.
#[test]
fn the_lot_ledger_records_a_lot_when_first_seen_and_when_it_falls() {
    let (u, c) = (sym("NTPC"), Contract::parse("2019-03-28-12500-PE").unwrap());
    let mut ledger = LotLedger::new();
    let first = ledger
        .observe(u, c, d(2019, 3, 1), LotEvidence::Inferred(8_000))
        .unwrap();
    assert_eq!(
        first.line(),
        "lot NSE-FNO-NTPC-2019-03-28-12500-PE from=2019-03-01 lot=8000 basis=ltq_gcd v=1"
    );
    assert_eq!(LotRecord::parse(&first.line()), Some(first));
    assert_eq!(
        ledger.observe(u, c, d(2019, 3, 4), LotEvidence::Inferred(8_000)),
        None,
        "unchanged"
    );
    assert_eq!(
        ledger.observe(u, c, d(2019, 3, 5), LotEvidence::Inferred(16_000)),
        None,
        "a day of two-lot trades cannot raise it"
    );
    assert_eq!(
        ledger.observe(u, c, d(2019, 3, 6), LotEvidence::NoTrade),
        None
    );
    let fell = ledger
        .observe(u, c, d(2019, 3, 7), LotEvidence::Inferred(12_000))
        .unwrap();
    assert_eq!(
        (fell.from, fell.lot),
        (d(2019, 3, 7), 4_000),
        "gcd(8000, 12000)"
    );
    assert_eq!(ledger.lot_of(u, c), Some(4_000));
    // The key is reused at the ex-date: the next day is a new lineage.
    ledger.restart(u, c);
    assert_eq!(ledger.lot_of(u, c), None);
    let again = ledger
        .observe(u, c, d(2019, 3, 19), LotEvidence::Inferred(4_800))
        .unwrap();
    assert_eq!(again.lot, 4_800);
    // Seeded from the ledger file, a re-run writes nothing new.
    let mut rerun = LotLedger::new();
    rerun.seed(&again);
    assert_eq!(
        rerun.observe(u, c, d(2019, 3, 20), LotEvidence::Inferred(9_600)),
        None
    );
    assert_eq!(rerun, {
        let mut want = LotLedger::new();
        want.seed(&again);
        want
    });
    let line = again.line();
    for bad in [
        line.replace("basis=ltq_gcd", "basis=ltq_min"),
        line.replace("lot=4800", "lot=0"),
        line.replace("lot=4800", "lot=04800"),
        line.replace("from=2019-03-19", "from=2019-03-32"),
        line.replace("lot NSE", "lots NSE"),
        line.replace(" v=1", ""),
        line.replace("2019-03-28-12500-PE", "2019-03-28-12500-CE "),
    ] {
        assert_eq!(LotRecord::parse(&bad), None, "{bad:?}");
    }
}

#[test]
fn the_prediction_is_half_up_on_the_grid_and_refuses_what_it_cannot_hold() {
    assert_eq!(predicted(15_000, 4_000, 4_800), Some(12_500));
    assert_eq!(predicted(10_000, 1, 3), Some(3_335), "33.333 to 33.35");
    assert_eq!(predicted(1, 1, 1), Some(0));
    assert_eq!(predicted(10_000, 1, 0), None);
    assert_eq!(predicted(-5, 1, 1), None);
    assert_eq!(single_near(&[100, 200], 150), None, "none within 5");
    assert_eq!(single_near(&[148, 152], 150), None, "two within 5");
    assert_eq!(single_near(&[148, 300], 150), Some(148));
    assert_eq!(single_near(&[], 150), None);
}

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

// ---------------------------------------------------------------------------
// WP-12e: events that keep the lot (DB-B02, DB-B03, DB-B13) and one contract
// across a symbol change (DB-B05, DB-B11, DB-B62). Every strike, lot and open
// interest is invented; the shapes follow the audit's measured events (a
// 9-rupee shift off a 5-rupee grid, a 20-rupee shift on a 10-rupee grid, a
// re-listing) without copying a vendor row.
// ---------------------------------------------------------------------------

/// A distinct, positive open interest per (strike, side): no two keys of a
/// ladder share it, so each can name a partner.
fn oi_of(strike: i64, side: OptionSide) -> i64 {
    7_000_000 + strike * 3 + i64::from(side == OptionSide::Put)
}

/// Both sides of every strike, each closing on its own OI.
fn ladder(expiry: Day, strikes: &[i64], lot: u64) -> Vec<ContractDay> {
    strikes
        .iter()
        .flat_map(|&k| {
            [OptionSide::Call, OptionSide::Put].map(|side| {
                let oi = oi_of(k, side);
                cd(expiry, k, side, lot, oi, oi)
            })
        })
        .collect()
}

/// DB-B02, DB-B13. A strike adjustment that keeps the lot (the shape of a
/// dividend: every strike less one constant, off the grid) is an event: each
/// pre-ex contract is linked to the key it continues as. Before WP-12e
/// `detect` returned `None` whenever the lot was unchanged, so every such
/// series ended weeks before its expiry and restarted unlinked.
#[test]
fn an_offset_that_keeps_the_lot_links_every_contract_to_its_shifted_key() {
    let (e1, e2) = (d(2020, 2, 27), d(2020, 3, 26));
    let strikes: Vec<i64> = (0..7).map(|i| 12_000 + i * 500).collect();
    let mut before = ladder(e1, &strikes, 3_400);
    before.extend(ladder(e2, &strikes[..3], 3_400));
    // Every key moves down 9.00. The first OI of most is some other number;
    // on four of them it is exactly the pre-ex close of the key 9.00 above.
    let mut after: Vec<ContractDay> = before
        .iter()
        .enumerate()
        .map(|(i, old)| {
            let carried = old.close_oi.unwrap();
            let first = if i % 5 == 0 { carried } else { carried + 3_400 };
            cd(
                old.expiry,
                old.strike - 900,
                old.side,
                3_400,
                first,
                first + 1,
            )
        })
        .collect();
    // One pre-ex contract did not trade on the ex-date: unpaired, not guessed.
    let absent = after.pop().unwrap();
    let action = detect(sym("OIL"), d(2020, 2, 20), &before, &after).expect("an event");
    assert_eq!(action.kind, EventKind::StrikeOffset { shift: 900 });
    assert_eq!(action.kind.as_str(), "strike_offset");
    assert_eq!((action.lot_before, action.lot_after), (3_400, 3_400));
    assert_eq!(action.lineage.len(), before.len() - 1);
    assert_eq!(action.unpaired.len(), 1);
    assert_eq!(
        action.unpaired[0].to_string(),
        seg(absent.expiry, absent.strike + 900, absent.side)
    );
    assert!(action.stitched.is_empty(), "no pre-ex text trades again");
    for pair in &action.lineage {
        let (expiry, old, side) = contract_parts(pair.old).unwrap();
        assert_eq!(
            contract_parts(pair.new),
            Some((expiry, old - 900, side)),
            "{pair:?}"
        );
        assert!(!pair.reused);
        // The record is D-3331's line, factor 1/1: the keys carry the shift.
        let record = pair.record(&[]);
        assert_eq!(
            (record.factor, record.lot_old, record.lot_new),
            ((1, 1), 3_400, 3_400)
        );
        assert_eq!(record.oi_unadjusted_until, 0);
        assert_eq!(LineageRecord::parse(&record.line()), Some(record));
    }
    // Every pre-ex key is retired, so a later file under one is stitched.
    assert_eq!(action.retired().len(), before.len());
    // The day after, the shifted keys carry on: not an event.
    assert_eq!(detect(sym("OIL"), d(2020, 2, 24), &after, &after), None);
}

/// DB-B02. When the shift is a multiple of the strike step the pre-ex TEXTS
/// trade on the ex-date as other contracts. The names continue, so only the
/// OI tells: it carries on the shifted key and on no same text (the audit's
/// VEDL 2024-09-10: 0 of 37 same-text, 13 of 43 shifted). Every reused key
/// is named stitched with the contract it held before and the one renamed
/// onto it. Before WP-12e: `None`, and one store series silently held two
/// exchange contracts.
#[test]
fn an_offset_on_the_strike_grid_is_proved_by_the_oi_and_names_the_stitched_keys() {
    let e = d(2024, 9, 26);
    let strikes: Vec<i64> = (0..9).map(|i| 40_000 + i * 1_000).collect();
    let before = ladder(e, &strikes, 2_300);
    // Minus 20.00 on a 10.00 grid: 38,000..46,000; seven texts are reused.
    let moved = |carry_every: usize| -> Vec<ContractDay> {
        before
            .iter()
            .enumerate()
            .map(|(i, old)| {
                let carried = old.close_oi.unwrap();
                let first = if i % carry_every == 0 {
                    carried
                } else {
                    carried + 2_300
                };
                cd(e, old.strike - 2_000, old.side, 2_300, first, first)
            })
            .collect()
    };
    let after = moved(3);
    let action = detect(sym("VEDL"), d(2024, 9, 10), &before, &after).expect("an event");
    assert_eq!(action.kind, EventKind::StrikeOffset { shift: 2_000 });
    assert_eq!(action.lineage.len(), 18);
    assert_eq!(
        action.stitched.len(),
        14,
        "seven reused strikes, both sides"
    );
    assert_eq!(action.lineage.iter().filter(|pair| pair.reused).count(), 14);
    let key = action.stitched[0];
    let (_, strike, side) = contract_parts(key).unwrap();
    let record = action.stitched_record(key).expect("a retired key");
    assert_eq!(
        record.before.unwrap().to_string(),
        seg(e, strike - 2_000, side)
    );
    assert_eq!(
        record.after.unwrap().to_string(),
        seg(e, strike + 2_000, side)
    );
    assert_eq!(StitchedRecord::parse(&record.line()), Some(record));
    assert_eq!(action.stitched_records().len(), 14);

    // Fewer carried pairs than the bar proves nothing while texts continue.
    assert_eq!(MIN_OI_PAIRS, 5);
    let four = moved(5);
    assert_eq!(
        four.iter()
            .zip(&before)
            .filter(|(new, old)| new.first_oi == old.close_oi)
            .count(),
        4
    );
    assert_eq!(detect(sym("VEDL"), d(2024, 9, 10), &before, &four), None);
}

/// The ordinary day. The same texts trade; some open on their own previous
/// close. Nothing is declared, whatever else the day's numbers look like.
#[test]
fn an_ordinary_day_with_an_unchanged_lot_declares_nothing() {
    let e = d(2024, 9, 26);
    let strikes: Vec<i64> = (0..9).map(|i| 40_000 + i * 1_000).collect();
    let before = ladder(e, &strikes, 2_300);
    let day = d(2024, 9, 6);
    // Every text carries its own OI.
    assert_eq!(detect(sym("VEDL"), day, &before, &before), None);
    // No text carries, and no shift is voted for: an absent carry alone is
    // what a thin underlying's ordinary day looks like.
    let moved_oi: Vec<ContractDay> = before
        .iter()
        .map(|c| cd(e, c.strike, c.side, 2_300, c.close_oi.unwrap() + 2_300, 1))
        .collect();
    assert_eq!(detect(sym("VEDL"), day, &before, &moved_oi), None);
    // Six keys open on the close of the key 20.00 above, but two same texts
    // carry too: six is not more than four times two.
    let mixed: Vec<ContractDay> = before
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let first = match i {
                0 | 1 => c.close_oi.unwrap(),
                2..=7 => oi_of(c.strike + 2_000, c.side),
                _ => 11,
            };
            cd(e, c.strike, c.side, 2_300, first, first)
        })
        .collect();
    assert_eq!(detect(sym("VEDL"), day, &before, &mixed), None);
    // An OI two pre-ex keys of a ladder share names no partner: every pre-ex
    // key closes on one lot, every key opens on it, and no shift is voted.
    let flat: Vec<ContractDay> = strikes
        .iter()
        .map(|&k| cd(e, k, OptionSide::Call, 2_300, 2_300, 2_300))
        .collect();
    let flat_moved: Vec<ContractDay> = strikes
        .iter()
        .map(|&k| cd(e, k - 2_000, OptionSide::Call, 2_300, 2_300, 2_300))
        .collect();
    assert_eq!(detect(sym("VEDL"), day, &flat, &flat_moved), None);
    // Two shifts with equal votes: neither is chosen.
    let tie: Vec<ContractDay> = before
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let first = if i < 6 {
                oi_of(c.strike + 2_000, c.side)
            } else if i < 12 {
                oi_of(c.strike - 3_000, c.side)
            } else {
                13
            };
            cd(e, c.strike, c.side, 2_300, first, first)
        })
        .collect();
    assert_eq!(detect(sym("VEDL"), day, &before, &tie), None);
    // A non-positive or missing OI never votes and never carries.
    let zero: Vec<ContractDay> = strikes
        .iter()
        .map(|&k| cd(e, k, OptionSide::Call, 2_300, 0, 0))
        .collect();
    assert_eq!(detect(sym("VEDL"), day, &zero, &zero), None);
}

/// DB-B03, DB-B13. Every pre-ex contract stops and the same expiries are
/// listed again under strikes the OI does not relate: a re-listing (the
/// shape of an early expiry on a scheme of arrangement). Nothing is paired;
/// every pre-ex key is retired, so a text of the old generation listed again
/// later is named stitched rather than read as one series. Before WP-12e:
/// `None`.
#[test]
fn a_name_break_with_no_carried_oi_is_a_relisting_that_pairs_nothing() {
    let (e1, e2) = (d(2020, 3, 26), d(2020, 4, 30));
    let old: Vec<i64> = (0..4).map(|i| 70_000 + i * 2_000).collect();
    let mut before = ladder(e1, &old, 900);
    before.extend(ladder(e2, &old[..1], 900));
    // The same expiries, new strikes around a new price, OI starting afresh.
    let fresh = |expiry, n: i64| -> Vec<ContractDay> {
        (0..n)
            .flat_map(|i| {
                [OptionSide::Call, OptionSide::Put]
                    .map(|side| cd(expiry, 27_000 + i * 1_000, side, 900, 900 * (i + 1), 1_800))
            })
            .collect()
    };
    let mut after = fresh(e1, 3);
    after.extend(fresh(e2, 1));
    let ex = d(2020, 3, 4);
    let action = detect(sym("TATACHEM"), ex, &before, &after).expect("a name break");
    assert_eq!(action.kind, EventKind::Relisted);
    assert_eq!(action.kind.as_str(), "relisted");
    assert!(
        action.lineage.is_empty(),
        "no arithmetic relates the generations"
    );
    assert_eq!(action.unpaired.len(), before.len());
    assert!(action.stitched.is_empty());
    let retired = action.retired();
    assert_eq!(retired.len(), 10);
    // An old-generation text listed again after the ex-date is stitched, with
    // no partner on either side.
    let again = before[0].contract().unwrap();
    let record = action.stitched_record(again).expect("retired");
    assert_eq!((record.before, record.after), (None, None));
    assert_eq!(record.ex_date, ex);
    assert_eq!(StitchedRecord::parse(&record.line()), Some(record));

    // Not a break: fewer than the bar on the day's side...
    assert_eq!(MIN_EVENT_KEYS, 8);
    assert_eq!(detect(sym("TATACHEM"), ex, &before, &after[..7]), None);
    // ...or on the pre-ex side...
    assert_eq!(detect(sym("TATACHEM"), ex, &before[..7], &after), None);
    // ...a NEW expiry the underlying did not hold (a listing, not a break)...
    let listing = fresh(d(2020, 5, 28), 5);
    assert_eq!(detect(sym("TATACHEM"), ex, &before, &listing), None);
    // ...no pre-ex state at all (the first day of a run)...
    assert_eq!(detect(sym("TATACHEM"), ex, &[], &after), None);
    // ...or one pre-ex text still trading.
    let mut one_continues = after.clone();
    one_continues.push(cd(e1, old[0], OptionSide::Call, 900, 5, 5));
    assert_eq!(detect(sym("TATACHEM"), ex, &before, &one_continues), None);

    // The same break with three keys opening on the close of the key 430.00
    // above is an offset: the OI relates the generations.
    assert_eq!(MIN_OI_FRESH, 3);
    let mut carried = after.clone();
    for (slot, k) in carried.iter_mut().zip([70_000, 72_000, 74_000]) {
        *slot = cd(
            e1,
            k - 43_000,
            OptionSide::Call,
            900,
            oi_of(k, OptionSide::Call),
            1,
        );
    }
    let offset = detect(sym("TATACHEM"), ex, &before, &carried).expect("an offset");
    assert_eq!(offset.kind, EventKind::StrikeOffset { shift: 43_000 });
    // Two are not enough: it stays a re-listing.
    carried[2] = after[2];
    let two = detect(sym("TATACHEM"), ex, &before, &carried).expect("a name break");
    assert_eq!(two.kind, EventKind::Relisted);
}

/// DB-B03. A re-listing whose new strikes REUSE the old texts shows no name
/// break, and is declared only on a SOURCED ex-date (NSE/FAOP/73857, VEDL
/// 2026-04-30). The reused keys are stitched. The same rows on any other day
/// or underlying declare nothing: NOT DETECTED without a source.
#[test]
fn a_relisting_that_reuses_the_texts_is_declared_only_from_a_sourced_ex_date() {
    let e = d(2026, 5, 26);
    let strikes: Vec<i64> = (0..6).map(|i| 40_000 + i * 1_000).collect();
    let before = ladder(e, &strikes, 1_150);
    let after: Vec<ContractDay> = before
        .iter()
        .map(|c| cd(e, c.strike, c.side, 1_150, 1_150, 2_300))
        .collect();
    let action = detect(sym("VEDL"), d(2026, 4, 30), &before, &after).expect("sourced");
    assert_eq!(action.kind, EventKind::Relisted);
    assert!(action.lineage.is_empty());
    assert_eq!(action.stitched.len(), 12);
    assert_eq!(action.stitched_records().len(), 12);
    assert_eq!(detect(sym("VEDL"), d(2026, 4, 29), &before, &after), None);
    assert_eq!(
        detect(sym("HINDUNILVR"), d(2026, 4, 30), &before, &after),
        None
    );
    // A lot step is still a lot step, and says so.
    let split: Vec<ContractDay> = before
        .iter()
        .map(|c| cd(e, c.strike / 2, c.side, 2_300, c.close_oi.unwrap() * 2, 1))
        .collect();
    let step = detect(sym("ACC"), d(2026, 4, 30), &before, &split).expect("a lot step");
    assert_eq!(step.kind, EventKind::LotStep);
    assert_eq!(step.kind.as_str(), "lot_step");
}

/// DB-B05, DB-B11, DB-B62. An option file is filed under the newest symbol
/// its name's dated changes reach, on the days the exchange used the name,
/// and refused on the days it did not. A symbol with no row keeps its key.
#[test]
fn an_option_file_is_filed_under_the_newest_symbol_its_name_reaches() {
    let filed =
        |stem: &str, on: Day| filed_underlying(sym(stem), on).map(|s| s.as_str().to_owned());
    // The day before the change, the old name; the day of it, the new one.
    assert_eq!(filed("NIITTECH", d(2020, 8, 19)).as_deref(), Ok("COFORGE"));
    assert_eq!(filed("COFORGE", d(2020, 8, 20)).as_deref(), Ok("COFORGE"));
    assert_eq!(
        filed("IDFCBANK", d(2019, 1, 15)).as_deref(),
        Ok("IDFCFIRSTB")
    );
    // A chain files under its end from the first name on.
    assert_eq!(filed("LTI", d(2022, 12, 2)).as_deref(), Ok("LTM"));
    assert_eq!(filed("LTIM", d(2022, 12, 5)).as_deref(), Ok("LTM"));
    assert_eq!(filed("LTM", d(2026, 2, 27)).as_deref(), Ok("LTM"));
    // The wrong side of a change is refused with the change it is on.
    let Err(Stale::AfterChange(change)) = filed("NIITTECH", d(2020, 8, 20)) else {
        panic!("an old name on its change day");
    };
    assert_eq!((change.old, change.new), ("NIITTECH", "COFORGE"));
    assert!(matches!(
        filed("COFORGE", d(2020, 8, 19)),
        Err(Stale::BeforeChange(_))
    ));
    // No row, no successor guessed: indices, ordinary shares, and the two
    // renames NSE's file does not carry under these names.
    for own in [
        "NIFTY",
        "BANKNIFTY",
        "RELIANCE",
        "TATAGLOBAL",
        "KPIT",
        "M&M",
    ] {
        assert_eq!(filed(own, d(2020, 2, 26)).as_deref(), Ok(own));
    }

    // Through the decoder: one exchange contract traded on both sides of the
    // rename is ONE store key. Before WP-12e these were two.
    let old = crate::gdfl_nfo::decode_ticker("NIITTECH27AUG201900CE", d(2020, 8, 19)).unwrap();
    let new = crate::gdfl_nfo::decode_ticker("COFORGE27AUG201900CE", d(2020, 8, 20)).unwrap();
    assert_eq!(old.underlying.as_str(), "COFORGE");
    assert_eq!(
        option_key(old.underlying, old.contract),
        option_key(new.underlying, new.contract)
    );
    assert_eq!(
        option_key(old.underlying, old.contract),
        "NSE-FNO-COFORGE-2020-08-27-190000-CE"
    );
    // The old name on the change day is refused by name, never filed.
    let stale =
        crate::gdfl_nfo::decode_ticker("NIITTECH27AUG201900CE", d(2020, 8, 20)).unwrap_err();
    assert!(matches!(
        stale,
        crate::gdfl_nfo::NfoRefusal::UnderlyingStale { .. }
    ));
    let said = stale.to_string();
    assert!(
        said.contains("NIITTECH27AUG201900CE")
            && said.contains("NIITTECH became COFORGE on 2020-08-20"),
        "{said}"
    );
    // An index and an unrenamed share decode exactly as before.
    let nifty = crate::gdfl_nfo::decode_ticker("NIFTY27AUG2011500CE", d(2020, 8, 19)).unwrap();
    assert_eq!(nifty.underlying.as_str(), "NIFTY");
}

/// Cost of the unchanged-lot arm on an ordinary day: one pass, measured.
/// `cargo test -p pull --release -- --ignored the_unchanged_lot_arm --nocapture`
#[test]
#[ignore = "a measurement: prints p50, p99 and max"]
fn the_unchanged_lot_arm_costs_one_pass_on_an_ordinary_day() {
    let strikes: Vec<i64> = (0..50).map(|i| 1_000_000 + i * 5_000).collect();
    let mut day = Vec::new();
    for expiry in [
        d(2020, 9, 17),
        d(2020, 9, 24),
        d(2020, 10, 1),
        d(2020, 10, 29),
    ] {
        day.extend(ladder(expiry, &strikes, 75));
    }
    assert_eq!(day.len(), 400);
    let mut nanos: Vec<u128> = (0..2_000)
        .map(|_| {
            let at = std::time::Instant::now();
            let got = detect(sym("NIFTY"), d(2020, 9, 16), &day, &day);
            let took = at.elapsed().as_nanos();
            assert_eq!(got, None);
            took
        })
        .collect();
    nanos.sort_unstable();
    println!(
        "detect, 400 contracts, unchanged lot, n=2000: p50 {} ns, p99 {} ns, max {} ns",
        nanos[1_000], nanos[1_980], nanos[1_999]
    );
}

/// The keys two vendor symbols name, counted by the symbols in order of
/// their lives; panics when two lives of one key overlap.
fn joined_across_renames(
    keys: &HashMap<String, std::collections::BTreeMap<String, (Day, Day)>>,
) -> std::collections::BTreeMap<String, u32> {
    use std::collections::BTreeMap;
    let mut joined: BTreeMap<String, u32> = BTreeMap::new();
    for (key, stems) in keys {
        if stems.len() < 2 {
            continue;
        }
        let mut lives: Vec<(&String, &(Day, Day))> = stems.iter().collect();
        lives.sort_by_key(|(_, life)| life.0.days_from_epoch());
        for pair in lives.windows(2) {
            assert!(
                pair[0].1.1.days_from_epoch() < pair[1].1.0.days_from_epoch(),
                "{key}: {pair:?} overlap"
            );
        }
        let label = lives
            .iter()
            .map(|(stem, _)| stem.as_str())
            .collect::<Vec<_>>()
            .join("->");
        *joined.entry(label).or_default() += 1;
    }
    joined
}

/// WP-12e against the operator's REAL option file NAMES, read-only: the name
/// census `BRUTEX_NAME_CENSUS` (one line per name: name, first vendor day,
/// last vendor day, days; tab-separated; no vendor row). Every name is
/// decoded on its first and its last day. It proves, on the whole archive:
/// no name is on the wrong side of a symbol change; every contract two
/// symbols name is one key whose old-name life ends before its new-name life
/// begins (DB-B05, DB-B11, DB-B62); and the alias table re-keys exactly its
/// own count of names, each onto a key its successor text also names or
/// alone (DB-B01). Prints the per-rename counts.
#[test]
#[ignore = "requires BRUTEX_NAME_CENSUS (names only); read-only"]
fn the_real_names_are_one_key_per_contract_across_renames_and_redatings() {
    use std::collections::BTreeMap;
    let path = std::env::var_os("BRUTEX_NAME_CENSUS").expect("BRUTEX_NAME_CENSUS");
    let text = std::fs::read_to_string(path).unwrap();
    // key -> stem -> (first, last)
    let mut keys: HashMap<String, BTreeMap<String, (Day, Day)>> = HashMap::new();
    let (mut names, mut decoded, mut stale, mut aliased) = (0_u32, 0_u32, Vec::new(), 0_u32);
    let mut refused: BTreeMap<String, u32> = BTreeMap::new();
    for line in text.lines() {
        let mut fields = line.split('\t');
        let (Some(name), Some(first), Some(last)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let (Some(first), Some(last)) = (parse_day(first), parse_day(last)) else {
            continue;
        };
        names += 1;
        let mut both = Vec::new();
        for on in [first, last] {
            match crate::gdfl_nfo::decode_ticker(name, on) {
                Ok(ticker) => both.push(ticker),
                Err(crate::gdfl_nfo::NfoRefusal::UnderlyingStale { .. }) => {
                    stale.push(format!("{name} on {on}"));
                }
                Err(why) => {
                    let word = format!("{why:?}");
                    let word = word.split([' ', '{']).next().unwrap_or_default().to_owned();
                    *refused.entry(word).or_default() += 1;
                }
            }
        }
        let [a, b] = both.as_slice() else {
            continue;
        };
        decoded += 1;
        assert_eq!(
            (a.underlying, a.contract),
            (b.underlying, b.contract),
            "{name}: one name, one key, on its first and last day"
        );
        aliased += u32::from(matches!(
            a.settlement,
            crate::expiry_alias::Settlement::Aliased(_)
        ));
        let stem_len = name.len() - 2;
        let stem: String = name[..stem_len]
            .chars()
            .take_while(|c| !c.is_ascii_digit())
            .collect();
        let slot = keys
            .entry(option_key(a.underlying, a.contract))
            .or_default()
            .entry(stem)
            .or_insert((first, last));
        slot.0 = if first.days_from_epoch() < slot.0.days_from_epoch() {
            first
        } else {
            slot.0
        };
        slot.1 = if last.days_from_epoch() > slot.1.days_from_epoch() {
            last
        } else {
            slot.1
        };
    }
    let joined = joined_across_renames(&keys);
    println!("names {names}, decoded on both days {decoded}, aliased names {aliased}");
    println!("refused (first or last day), by kind: {refused:?}");
    println!(
        "contracts named by two symbols, now one key: {} in {} renames",
        joined.values().sum::<u32>(),
        joined.len()
    );
    for (label, n) in &joined {
        println!("  {label}: {n}");
    }
    assert!(
        stale.is_empty(),
        "{} stale names, first {:?}",
        stale.len(),
        stale.first()
    );
    assert!(
        joined.values().sum::<u32>() > 0,
        "no contract crossed a rename"
    );
    assert_eq!(
        aliased,
        crate::expiry_alias::ALIASES
            .iter()
            .map(|row| row.contracts)
            .sum::<u32>(),
        "the table re-keys exactly its own count of names"
    );
}

/// D-3355: an `event` record is one line that reads back exactly, and any
/// other text is no record: a kind its lots or counts contradict, an event
/// that retired nothing, a value not in its canonical spelling, a field out
/// of place.
#[test]
fn an_event_record_reads_back_exactly_and_refuses_any_other_text() {
    let ex = d(2020, 3, 4);
    let events = [
        (
            EventKind::LotStep,
            (6_000, 5_000),
            (31, 2),
            "event NSE-FNO-NTPC ex=2020-03-04 kind=lot_step shift=0 lot_old=6000 lot_new=5000 \
             paired=31 stopped=2 v=1",
        ),
        (
            EventKind::StrikeOffset { shift: 900 },
            (3_400, 3_400),
            (19, 1),
            "event NSE-FNO-NTPC ex=2020-03-04 kind=strike_offset shift=900 lot_old=3400 \
             lot_new=3400 paired=19 stopped=1 v=1",
        ),
        (
            EventKind::StrikeOffset { shift: -250 },
            (3_400, 3_400),
            (4, 0),
            "event NSE-FNO-NTPC ex=2020-03-04 kind=strike_offset shift=-250 lot_old=3400 \
             lot_new=3400 paired=4 stopped=0 v=1",
        ),
        (
            EventKind::Relisted,
            (900, 900),
            (0, 32),
            "event NSE-FNO-NTPC ex=2020-03-04 kind=relisted shift=0 lot_old=900 lot_new=900 \
             paired=0 stopped=32 v=1",
        ),
    ];
    for (kind, (lot_old, lot_new), (paired, stopped), line) in events {
        let record = EventRecord {
            underlying: sym("NTPC"),
            ex_date: ex,
            kind,
            lot_old,
            lot_new,
            paired,
            stopped,
        };
        assert_eq!(record.line(), line);
        assert_eq!(EventRecord::parse(line), Some(record), "{line}");
        assert_eq!(EventKind::parse(kind.as_str(), kind.shift()), Some(kind));
    }
    // An underlying holding a dash reads back whole.
    let dashed = EventRecord {
        underlying: sym("BAJAJ-AUTO"),
        ex_date: ex,
        kind: EventKind::Relisted,
        lot_old: 250,
        lot_new: 250,
        paired: 0,
        stopped: 8,
    };
    assert_eq!(EventRecord::parse(&dashed.line()), Some(dashed));
    for bad in [
        // A lot step with equal lots; an offset and a re-listing with unequal.
        "event NSE-FNO-NTPC ex=2020-03-04 kind=lot_step shift=0 lot_old=900 lot_new=900 paired=3 stopped=0 v=1",
        "event NSE-FNO-NTPC ex=2020-03-04 kind=strike_offset shift=900 lot_old=900 lot_new=800 paired=3 stopped=0 v=1",
        "event NSE-FNO-NTPC ex=2020-03-04 kind=relisted shift=0 lot_old=900 lot_new=800 paired=0 stopped=3 v=1",
        // A re-listing that pairs; an offset of nothing; a shift on a kind without one.
        "event NSE-FNO-NTPC ex=2020-03-04 kind=relisted shift=0 lot_old=900 lot_new=900 paired=1 stopped=3 v=1",
        "event NSE-FNO-NTPC ex=2020-03-04 kind=strike_offset shift=0 lot_old=900 lot_new=900 paired=3 stopped=0 v=1",
        "event NSE-FNO-NTPC ex=2020-03-04 kind=relisted shift=900 lot_old=900 lot_new=900 paired=0 stopped=3 v=1",
        // An event that retired nothing.
        "event NSE-FNO-NTPC ex=2020-03-04 kind=relisted shift=0 lot_old=900 lot_new=900 paired=0 stopped=0 v=1",
        // Not canonical, out of place, unknown, torn, another version.
        "event NSE-FNO-NTPC ex=2020-03-04 kind=strike_offset shift=+900 lot_old=900 lot_new=900 paired=3 stopped=0 v=1",
        "event NSE-FNO-NTPC ex=2020-03-04 kind=strike_offset shift=-0 lot_old=900 lot_new=900 paired=3 stopped=0 v=1",
        "event NSE-FNO-NTPC ex=2020-03-04 kind=strike_offset shift=0900 lot_old=900 lot_new=900 paired=3 stopped=0 v=1",
        "event NSE-FNO-NTPC ex=2020-3-4 kind=relisted shift=0 lot_old=900 lot_new=900 paired=0 stopped=3 v=1",
        "event NSE-FNO-NTPC kind=relisted ex=2020-03-04 shift=0 lot_old=900 lot_new=900 paired=0 stopped=3 v=1",
        "event NSE-FNO-NTPC ex=2020-03-04 kind=dividend shift=0 lot_old=900 lot_new=900 paired=0 stopped=3 v=1",
        "event NSE-FNO-NTPC ex=2020-03-04 kind=relisted shift=0 lot_old=900 lot_new=900 paired=0 stopped=3",
        "event NSE-FNO-NTPC ex=2020-03-04 kind=relisted shift=0 lot_old=900 lot_new=900 paired=0 stopped=3 v=2",
        "event NSE-FNO-ntpc ex=2020-03-04 kind=relisted shift=0 lot_old=900 lot_new=900 paired=0 stopped=3 v=1",
        "event NTPC ex=2020-03-04 kind=relisted shift=0 lot_old=900 lot_new=900 paired=0 stopped=3 v=1",
        "event  NSE-FNO-NTPC ex=2020-03-04 kind=relisted shift=0 lot_old=900 lot_new=900 paired=0 stopped=3 v=1",
        "events NSE-FNO-NTPC ex=2020-03-04 kind=relisted shift=0 lot_old=900 lot_new=900 paired=0 stopped=3 v=1",
        "event NSE-FNO-NTPC ex=2020-03-04 kind=relisted shift=0 lot_old=900 lot_new=900 paired=0 stopped=99999999999999999999 v=1",
        "",
    ] {
        assert_eq!(EventRecord::parse(bad), None, "{bad:?}");
    }
}

/// D-3355: a `stopped` record is one line that reads back exactly, and it is
/// no other record kind, nor any other kind it.
#[test]
fn a_stopped_record_reads_back_exactly_and_refuses_any_other_text() {
    let ex = d(2020, 3, 4);
    let key = cd(d(2020, 3, 26), 70_000, OptionSide::Put, 1, 0, 0)
        .contract()
        .unwrap();
    let stopped = StoppedRecord {
        underlying: sym("BAJAJ-AUTO"),
        key,
        ex_date: ex,
    };
    let line = "stopped NSE-FNO-BAJAJ-AUTO-2020-03-26-70000-PE ex=2020-03-04 v=1";
    assert_eq!(stopped.line(), line);
    assert_eq!(StoppedRecord::parse(line), Some(stopped));
    for bad in [
        "stopped NSE-FNO-BAJAJ-AUTO-2020-03-26-70000-PE ex=2020-03-04",
        "stopped NSE-FNO-BAJAJ-AUTO-2020-03-26-70000-PE ex=2020-03-04 v=2",
        "stopped NSE-FNO-BAJAJ-AUTO-2020-03-26-070000-PE ex=2020-03-04 v=1",
        "stopped NSE-FNO-BAJAJ-AUTO-2020-03-26-70000-XX ex=2020-03-04 v=1",
        "stopped NSE-FNO-BAJAJ-AUTO ex=2020-03-04 v=1",
        "stopped NSE-FNO-BAJAJ-AUTO-2020-03-26-70000-PE v=1 ex=2020-03-04",
        "stitched NSE-FNO-BAJAJ-AUTO-2020-03-26-70000-PE ex=2020-03-04 v=1",
        "stopped NSE-FNO-BAJAJ-AUTO-2020-03-26-70000-PE ex=2020-03-04 v=1 ",
    ] {
        assert_eq!(StoppedRecord::parse(bad), None, "{bad:?}");
    }
    // Neither reads as any other record kind, nor any other kind as these.
    let event = "event NSE-FNO-NTPC ex=2020-03-04 kind=relisted shift=0 lot_old=900 lot_new=900 \
                 paired=0 stopped=32 v=1";
    assert!(EventRecord::parse(event).is_some());
    assert_eq!(LineageRecord::parse(line), None);
    assert_eq!(StitchedRecord::parse(line), None);
    assert_eq!(RetiredRecord::parse(line), None);
    assert_eq!(EventRecord::parse(line), None);
    assert_eq!(StoppedRecord::parse(event), None);
    assert_eq!(StitchedRecord::parse(event), None);
    assert_eq!(LineageRecord::parse(event), None);
}

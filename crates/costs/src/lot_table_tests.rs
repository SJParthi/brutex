//! The lot table as store data (R3-30): rows handed in by `pull`'s one
//! loader, looked up with a named absence wherever no sourced row applies.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

use brutex_core::symbol::Symbol;

use crate::venue::swept_slot;

fn day(year: u16, month: u8, d: u8) -> TradeDay {
    TradeDay::new(year, month, d).expect("a real date")
}

fn sourced(underlying: &str, effective: TradeDay, units: u32) -> LotRow {
    LotRow {
        underlying: underlying.to_owned(),
        effective,
        units,
        standing: LotStanding::Sourced,
        source: "an invented circular, for this test only".to_owned(),
        source_sha256: Some("ab".repeat(32)),
        read: Some(day(2026, 10, 20)),
    }
}

fn unverified(underlying: &str, effective: TradeDay, units: u32) -> LotRow {
    LotRow {
        standing: LotStanding::Unverified,
        source_sha256: None,
        read: None,
        source: "no circular read".to_owned(),
        ..sourced(underlying, effective, units)
    }
}

#[test]
fn the_compiled_rows_move_into_the_table_unchanged() {
    // Every NIFTY and BANKNIFTY row of the compiled table, same day, same
    // units, its own citation kept, in the same order.
    let rows = compiled_rows();
    let mut expected = Vec::new();
    for (slot, table) in LOT_SIZES.iter().enumerate() {
        for row in table.rows() {
            if let Dated::Verified(lot) = row.value {
                expected.push((
                    InstrumentKey::SWEPT[slot].1,
                    row.start,
                    lot.units(),
                    row.source,
                ));
            }
        }
    }
    assert_eq!(rows.len(), expected.len());
    assert_eq!(rows.len(), 10, "five dated rows per swept underlying");
    for (row, (underlying, start, units, source)) in rows.iter().zip(&expected) {
        assert_eq!(row.underlying, *underlying);
        assert_eq!(row.effective, *start);
        assert_eq!(row.units, *units);
        let normal = source.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            row.source.starts_with(&normal),
            "the row keeps its own citation first: {}",
            row.source
        );
    }
    // And the compiled lookup agrees with the table on every row's own day
    // and the day before it, for every SOURCED row.
    let table = LotTable::from_rows(rows.clone()).expect("the compiled rows build");
    for row in rows
        .iter()
        .filter(|row| row.standing == LotStanding::Sourced)
    {
        let slot = swept_slot(Symbol::new(&row.underlying).unwrap()).unwrap();
        let answered = table
            .lot_on(&row.underlying, row.effective)
            .expect("sourced");
        assert_eq!(answered.lot, lot_size_on(slot, row.effective).unwrap());
    }
}

#[test]
fn a_row_is_sourced_exactly_where_a_charter_circular_turns_the_lot() {
    let rows = compiled_rows();
    let sourced: Vec<(&str, TradeDay, u32)> = rows
        .iter()
        .filter(|row| row.standing == LotStanding::Sourced)
        .map(|row| (row.underlying.as_str(), row.effective, row.units))
        .collect();
    assert_eq!(
        sourced,
        vec![
            ("NIFTY", day(2024, 11, 20), 75),
            ("NIFTY", day(2026, 1, 1), 65),
            ("BANKNIFTY", day(2024, 11, 20), 30),
            ("BANKNIFTY", day(2025, 12, 30), 30),
        ],
        "NSE/FAOP/64625 and NSE/FAOP/70616 are the circulars docs/00-charter.md records"
    );
    for row in &rows {
        match row.standing {
            LotStanding::Sourced => {
                let sha = row.source_sha256.as_deref().unwrap();
                assert!(sha == FAOP64625_SHA256 || sha == FAOP70616_SHA256);
                assert_eq!(row.read, Some(day(2026, 10, 10)));
                assert!(row.source.contains("NSE/FAOP/"), "{}", row.source);
            }
            LotStanding::Unverified => {
                assert!(row.source_sha256.is_none() && row.read.is_none());
                assert!(row.source.contains("UNVERIFIED"), "{}", row.source);
            }
        }
    }
}

#[test]
fn an_underlying_without_a_sourced_lot_is_named_absent() {
    let table = LotTable::from_rows(compiled_rows()).unwrap();
    // A share with no row at all: named, never an inferred lot.
    let none = table.lot_on("RELIANCE", day(2025, 6, 2)).unwrap_err();
    assert_eq!(
        none,
        LotAbsence::NoRow {
            underlying: "RELIANCE".to_owned()
        }
    );
    assert_eq!(none.word(), "no_row");
    assert!(none.to_string().contains("no lot is inferred"));
    // A day governed by an unverified row: the row is shown, not applied.
    let listed = table.lot_on("NIFTY", day(2022, 3, 1)).unwrap_err();
    match &listed {
        LotAbsence::Unverified {
            effective, units, ..
        } => {
            assert_eq!(*effective, day(2021, 7, 29));
            assert_eq!(*units, 50);
        }
        other => panic!("expected the unverified row, got {other:?}"),
    }
    assert_eq!(listed.word(), "unverified");
    // Before the first row: the compiled LOT_GAP refusal, unchanged.
    let before = table.lot_on("NIFTY", day(2020, 12, 31)).unwrap_err();
    match &before {
        LotAbsence::BeforeFirstRow { first, gap, .. } => {
            assert_eq!(*first, LOT_VERIFIED_FROM);
            assert_eq!(*gap, LOT_GAP);
        }
        other => panic!("expected before-first-row, got {other:?}"),
    }
    assert!(
        lot_size_on(
            swept_slot(Symbol::new("NIFTY").unwrap()).unwrap(),
            day(2020, 12, 31)
        )
        .is_err()
    );
    // A sourced row answers, with the row that gives it.
    let answer = table.lot_on("NIFTY", day(2025, 3, 3)).unwrap();
    assert_eq!(answer.lot.units(), 75);
    assert_eq!(answer.row.effective, day(2024, 11, 20));
}

#[test]
fn a_lot_row_added_to_the_table_changes_the_next_lookup_without_a_code_change() {
    let mut rows = compiled_rows();
    let before = LotTable::from_rows(rows.clone()).unwrap();
    assert!(matches!(
        before.lot_on("SBIN", day(2025, 1, 6)),
        Err(LotAbsence::NoRow { .. })
    ));
    rows.push(sourced("SBIN", day(2024, 12, 27), 750));
    let after = LotTable::from_rows(rows).unwrap();
    assert_eq!(
        after.lot_on("SBIN", day(2025, 1, 6)).unwrap().lot.units(),
        750
    );
    assert!(matches!(
        after.lot_on("SBIN", day(2024, 12, 26)),
        Err(LotAbsence::BeforeFirstRow { gap, .. }) if gap != LOT_GAP
    ));
}

#[test]
fn every_malformed_table_is_refused_by_name() {
    // Empty is a table with no underlying: every lookup is NoRow.
    let empty = LotTable::from_rows(Vec::new()).unwrap();
    assert_eq!(empty.underlyings(), 0);
    assert!(matches!(
        empty.lot_on("NIFTY", day(2025, 1, 1)),
        Err(LotAbsence::NoRow { .. })
    ));
    // Zero units.
    assert_eq!(
        LotTable::from_rows(vec![sourced("SBIN", day(2025, 1, 1), 0)]),
        Err(LotTableError::ZeroLot { row: 1 })
    );
    // A sourced row with no digest.
    let mut bare = sourced("SBIN", day(2025, 1, 1), 1);
    bare.source_sha256 = None;
    assert_eq!(
        LotTable::from_rows(vec![bare]),
        Err(LotTableError::SourcedWithoutReceipt { row: 1 })
    );
    // A digest that is not hex.
    let mut wrong = sourced("SBIN", day(2025, 1, 1), 1);
    wrong.source_sha256 = Some("Z".repeat(64));
    assert_eq!(
        LotTable::from_rows(vec![wrong]),
        Err(LotTableError::SourcedWithoutReceipt { row: 1 })
    );
    // Duplicate day, and out of order, for one underlying -- even with
    // another underlying's row between them.
    for second in [day(2025, 1, 1), day(2024, 12, 31)] {
        let rows = vec![
            sourced("SBIN", day(2025, 1, 1), 1),
            sourced("TCS", day(2025, 1, 1), 1),
            sourced("SBIN", second, 2),
        ];
        assert_eq!(
            LotTable::from_rows(rows),
            Err(LotTableError::NotAscending { row: 3 })
        );
    }
    // Padded, empty and over-long text.
    for (text, column) in [(" SBIN", "underlying"), ("", "underlying")] {
        let mut row = sourced("SBIN", day(2025, 1, 1), 1);
        row.underlying = text.to_owned();
        assert_eq!(
            LotTable::from_rows(vec![row]),
            Err(LotTableError::Text { row: 1, column })
        );
    }
    let mut long = sourced("SBIN", day(2025, 1, 1), 1);
    long.source = "x".repeat(MAX_LOT_TEXT_BYTES + 1);
    assert_eq!(
        LotTable::from_rows(vec![long]),
        Err(LotTableError::Text {
            row: 1,
            column: "source"
        })
    );
    // One more row than an underlying may hold.
    let start = day(2020, 1, 1).ordinal();
    let many: Vec<LotRow> = (0..=MAX_LOT_ROWS_PER_UNDERLYING)
        .map(|at| {
            let ordinal = start + i32::try_from(at).unwrap();
            sourced("SBIN", TradeDay::from_ordinal(ordinal).unwrap(), 1)
        })
        .collect();
    assert_eq!(
        LotTable::from_rows(many),
        Err(LotTableError::TooManyForUnderlying {
            row: MAX_LOT_ROWS_PER_UNDERLYING + 1
        })
    );
    // One more row than the table may hold.
    let too_many = vec![unverified("SBIN", day(2025, 1, 1), 1); MAX_LOT_ROWS + 1];
    assert_eq!(
        LotTable::from_rows(too_many),
        Err(LotTableError::TooManyRows {
            rows: MAX_LOT_ROWS + 1
        })
    );
    // Every error says what it refused.
    for why in [
        LotTableError::TooManyRows { rows: 9 },
        LotTableError::Text {
            row: 1,
            column: "source",
        },
        LotTableError::ZeroLot { row: 2 },
        LotTableError::SourcedWithoutReceipt { row: 3 },
        LotTableError::NotAscending { row: 4 },
        LotTableError::TooManyForUnderlying { row: 5 },
    ] {
        assert!(why.to_string().contains("lot"), "{why}");
    }
}

#[test]
fn the_boundaries_of_a_row_are_exact() {
    let table = LotTable::from_rows(vec![
        sourced("SBIN", day(2025, 1, 1), 1_500),
        sourced("SBIN", day(2025, 7, 1), 750),
    ])
    .unwrap();
    assert!(matches!(
        table.lot_on("SBIN", day(2024, 12, 31)),
        Err(LotAbsence::BeforeFirstRow { .. })
    ));
    assert_eq!(
        table.lot_on("SBIN", day(2025, 1, 1)).unwrap().lot.units(),
        1_500
    );
    assert_eq!(
        table.lot_on("SBIN", day(2025, 6, 30)).unwrap().lot.units(),
        1_500
    );
    assert_eq!(
        table.lot_on("SBIN", day(2025, 7, 1)).unwrap().lot.units(),
        750
    );
    assert_eq!(
        table.lot_on("SBIN", TradeDay::MAX).unwrap().lot.units(),
        750
    );
    // A lookup is case-exact: no fuzzy name matching invents a row.
    assert!(matches!(
        table.lot_on("sbin", day(2025, 7, 1)),
        Err(LotAbsence::NoRow { .. })
    ));
}

/// p50, p99 and max of `n` lookups of `table`, in nanoseconds.
fn time_lookups(table: &LotTable, names: &[String], n: usize) -> (u128, u128, u128) {
    let days = [
        day(2019, 3, 4),
        day(2022, 8, 1),
        day(2025, 5, 5),
        day(2026, 9, 30),
    ];
    let mut samples = Vec::with_capacity(n);
    for at in 0..n {
        let name = &names[at % names.len()];
        let when = days[at % days.len()];
        let started = std::time::Instant::now();
        let answer = table.lot_on(name, when);
        samples.push(started.elapsed().as_nanos());
        std::hint::black_box(answer.is_ok());
    }
    samples.sort_unstable();
    let pick = |q: usize| samples[(samples.len() * q / 100).min(samples.len() - 1)];
    (pick(50), pick(99), *samples.last().unwrap())
}

#[test]
#[ignore = "a timing measurement: run with --release -- --ignored a_lookup_is_flat_in_the_table"]
fn a_lookup_is_flat_in_the_table() {
    fn table_of(underlyings: usize) -> (LotTable, Vec<String>) {
        let names: Vec<String> = (0..underlyings).map(|at| format!("U{at:05}")).collect();
        let mut rows = Vec::new();
        for name in &names {
            for step in 0..MAX_LOT_ROWS_PER_UNDERLYING {
                let ordinal = day(2018, 1, 1).ordinal() + i32::try_from(step * 45).unwrap();
                rows.push(sourced(name, TradeDay::from_ordinal(ordinal).unwrap(), 25));
            }
        }
        (LotTable::from_rows(rows).unwrap(), names)
    }
    let (small, small_names) = table_of(2);
    let (large, large_names) = table_of(1_500);
    let n = 100_000;
    let (s50, s99, smax) = time_lookups(&small, &small_names, n);
    let (l50, l99, lmax) = time_lookups(&large, &large_names, n);
    println!(
        "lot lookup n={n}: 2 underlyings x {MAX_LOT_ROWS_PER_UNDERLYING} rows p50 {s50} ns p99 {s99} ns max {smax} ns; \
         1500 underlyings x {MAX_LOT_ROWS_PER_UNDERLYING} rows p50 {l50} ns p99 {l99} ns max {lmax} ns"
    );
    assert!(
        l99 <= s99.saturating_mul(10).max(2_000),
        "a lookup in a table 750 times larger stays within ten times the p99"
    );
}

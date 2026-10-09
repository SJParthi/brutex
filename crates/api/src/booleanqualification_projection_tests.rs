#![cfg(test)]
//! Serialization only; actual evidence admission is owned by the CLI reader.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "finite exact projection assertions"
)]
use super::*;

#[test]
fn conservative_zero_keeps_unreduced_draw_denominator_without_inventing_shared_facts() {
    let projected = romano([2, u64::MAX, u64::MAX, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
    assert_eq!(projected["classification"], "conservative-zero");
    assert_eq!(
        projected["probability"]["denominator"],
        u64::MAX.to_string()
    );
    assert_eq!(projected["probability"]["numerator"], u64::MAX.to_string());
    assert!(projected["shared"].is_null());
}

#[test]
fn measured_and_nonpositive_classes_keep_exact_subfamily_indices_and_numeric_bits() {
    for (class, number) in [(1, 2.5_f64), (3, -0.0)] {
        let projected = romano([
            class,
            100,
            100,
            1,
            9_007_199_254_740_993,
            7,
            number.to_bits(),
            2,
            3,
            100,
            99,
            100,
        ])
        .unwrap();
        assert_eq!(projected["shared"]["strategy"], "9007199254740993");
        assert_eq!(
            projected["shared"]["statistic"]["bits"],
            number.to_bits().to_string()
        );
        assert_eq!(projected["shared"]["initial"]["numerator"], "3");
        assert_eq!(projected["shared"]["adjusted"]["numerator"], "99");
        assert_eq!(projected["shared"]["stepdown_rank"], "7");
    }
    assert!(romano([0; 12]).is_err());
    assert!(romano([2, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0]).is_err());
}

/// **A QUALIFICATION ROW MUST NAME THE ORIGINAL ROW BESIDE IT.** Each of the
/// three coordinates alone refuses; all three equal is the only acceptance
/// (GAP14-58, D-0731).
#[test]
fn a_qualification_row_is_refused_unless_identity_family_and_coordinate_all_match() {
    let original = cli::boolean_evidence::StatisticsRow {
        source: 3,
        coordinate: 7,
        identity: [9; 32],
        period_digest: [0; 32],
        run: [0; 32],
        program_index: 0,
        side: "long",
        ordinal: 0,
        execution_refusal_bits: 0,
        trades: 0,
        wins: 0,
        return_paisa: 0,
        wilson_lower_bits: 0,
        romano_availability: 1,
        romano: None,
    };
    assert_eq!(same_coordinate([9; 32], 3, 7, &original), Ok(()));
    for (identity, family, coordinate) in [([8; 32], 3, 7), ([9; 32], 4, 7), ([9; 32], 3, 6)] {
        assert_eq!(
            same_coordinate(identity, family, coordinate, &original),
            Err("qualification row differs from original coordinate".to_owned()),
            "{family} {coordinate}"
        );
    }
}

/// **`project` RUNS THE CROSS-CHECK ON EVERY ROW, BEFORE IT RENDERS IT.** No
/// api test can build a saved qualification (the fixture is `cli`'s own
/// `#[cfg(test)]` code), so the call is pinned in the source: inside `project`'s
/// row loop, `same_coordinate` is called with the row's three coordinates and
/// `?`, ahead of the first rendered field (GAP14-58, D-0731).
#[test]
fn project_cross_checks_each_row_before_rendering_it() {
    let source = include_str!("booleanqualification_projection.rs");
    let from = source.find("pub(super) fn project(").unwrap();
    let body = &source[from..from + source[from..].find("\n}\n").unwrap()];
    let row_loop = &body[body.find("for (n, (row, original)) in").unwrap()..];
    let check = row_loop
        .find("same_coordinate(row.original, row.family, row.coordinate, original)?;")
        .unwrap();
    let render = row_loop.find("admission_projection::row(").unwrap();
    assert!(check < render, "the check runs before the row is rendered");
}

/// **A ROW'S INDEX IS ITS PAGE'S OFFSET PLUS ITS PLACE ON THE PAGE.** Zero,
/// one, an offset with the first row, a later row, and the extremes that
/// still fit. G18-api-26.
#[test]
fn a_rows_index_is_the_offset_plus_its_place_on_the_page() {
    for (offset, n, index) in [
        (0, 0, 0),
        (0, 1, 1),
        (1, 1, 2),
        (2, 2, 4),
        (7, 0, 7),
        (7, 3, 10),
        (usize::MAX - 3, 3, usize::MAX),
    ] {
        assert_eq!(super::row_index(offset, n), index, "{offset} + {n}");
    }
}

/// **A qualification page renders at most `MAX_PAGE_FOLD_ROWS` folds, and past
/// it is refused by name before a row is read.** W1-api2-8, D-4443.
#[test]
fn a_page_past_the_fold_row_ceiling_is_refused_with_the_limit_that_fits() {
    assert_eq!(MAX_PAGE_FOLD_ROWS, 16_384);
    for (rows, folds) in [
        (0, 0),
        (0, 1 << 40),
        (256, 64),
        (1, 16_384),
        (16_384, 1),
        (128, 128),
    ] {
        assert!(
            fold_rows_admitted(rows, folds).is_ok(),
            "{rows} x {folds} fits"
        );
    }
    let why = fold_rows_admitted(256, 65).unwrap_err();
    assert!(why.contains("16640 fold rows"), "{why}");
    assert!(why.contains("ask limit=252 or less"), "{why}");
    assert!(why.contains("D-4443"), "{why}");
    assert!(
        fold_rows_admitted(252, 65).is_ok(),
        "the limit it names fits"
    );
    assert!(fold_rows_admitted(253, 65).is_err(), "and is the largest");
    assert!(fold_rows_admitted(2, 8_193).is_err());
    let one = fold_rows_admitted(1, 16_385).unwrap_err();
    assert!(one.contains("cannot be paged here"), "{one}");
    assert!(
        fold_rows_admitted(usize::MAX, 2).is_err(),
        "an overflow is not a fit"
    );
    let source = include_str!("booleanqualification_projection.rs");
    let project = &source[source.find("pub(super) fn project(").unwrap()..];
    let check = project.find("fold_rows_admitted(").unwrap();
    assert!(
        check < project.find("reader.rows(pin").unwrap(),
        "the ceiling is checked before any row is read"
    );
    assert!(project.contains(
        "asked\n            .limit\n            .min(reader.row_count().saturating_sub(asked.offset)),\n        reader.fold_count(),"
    ));
}

/// The fold row a page serves keeps every saved figure exactly, as text.
#[test]
fn a_fold_row_serves_each_saved_figure_as_exact_text() {
    let row = fold_row(3, [19_000, 19_030], [u64::MAX, 7, 2], i64::MIN);
    assert_eq!(row["index"], "3");
    assert_eq!(row["first_day"], "19000");
    assert_eq!(row["last_day"], "19030");
    assert_eq!(row["sessions"], u64::MAX.to_string());
    assert_eq!(row["trades"], "7");
    assert_eq!(row["wins"], "2");
    assert_eq!(row["return_paisa"], i64::MIN.to_string());
}

/// What a page at the ceiling spends rendering its folds: `MAX_PAGE_FOLD_ROWS`
/// calls of the page's own `fold_row`, serialized. A saved qualification can
/// be built only by `cli`'s private fixtures, so this times the fold half of
/// the page, not the route. W1-api2-8, D-4443.
#[test]
#[ignore = "a latency measurement, run on purpose: see crate::latency"]
fn latency_fold_rows_at_the_page_ceiling() -> Result<(), String> {
    let timed = crate::latency::Timed::run(100, || {
        let rows = (0..MAX_PAGE_FOLD_ROWS)
            .map(|n| {
                let day = i64::try_from(n).unwrap_or(0);
                fold_row(n % 64, [day, day + 30], [21, 40, 19], -1_234_567)
            })
            .collect::<Vec<_>>();
        serde_json::to_string(&rows)
            .map(drop)
            .map_err(|why| why.to_string())
    })?;
    println!(
        "{}",
        timed.line("16,384 fold rows rendered and serialized (one page at the ceiling)")
    );
    Ok(())
}

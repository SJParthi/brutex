//! Extreme cases of symbols, renames and expiries in `core`, for the cloud
//! extremes round of 10 Nov 2026 (`EXTREMES-REPORT.md` at the repository
//! root).
//!
//! * EVERY applied row of the symbol-change table, at its two edges: the day
//!   before the change and the day of it, for the old stem and the new (the
//!   existing tests walk the `LTI -> LTIM -> LTM` chain alone);
//! * the table's own consistency (no self-rename, no symbol leaving or
//!   arriving twice, every effective day a real date) and its agreement with
//!   the swept F&O list (no retired symbol is swept);
//! * a symbol's case folding against look-alike characters (full-width,
//!   Cyrillic, zero-width and non-breaking spaces, control bytes);
//! * the expiry date's leap rule at the century years.
//!
//! Every value is read from the crate's own tables or invented.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use brutex_core::alias::{LINEAGE, SYMBOL_CHANGES, Stale, SymbolChange, Ymd};
use brutex_core::instrument::Expiry;
use brutex_core::symbol::{SYMBOL_CAPACITY, Symbol};
use brutex_core::universe::FNO_UNDERLYINGS;

/// Days since 1970-01-01 of a proleptic Gregorian date.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// The proleptic Gregorian date of `days` since 1970-01-01.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Whether `ymd` names a real date.
fn real(ymd: Ymd) -> bool {
    let (y, m, d) = (
        i64::from(ymd.year),
        i64::from(ymd.month),
        i64::from(ymd.day),
    );
    civil_from_days(days_from_civil(y, m, d)) == (y, m, d)
}

/// The day before `ymd`.
fn before(ymd: Ymd) -> Ymd {
    let days = days_from_civil(
        i64::from(ymd.year),
        i64::from(ymd.month),
        i64::from(ymd.day),
    );
    let (y, m, d) = civil_from_days(days - 1);
    Ymd::new(
        u16::try_from(y).unwrap(),
        u8::try_from(m).unwrap(),
        u8::try_from(d).unwrap(),
    )
}

fn applied() -> Vec<SymbolChange> {
    SYMBOL_CHANGES
        .iter()
        .copied()
        .filter(|row| row.status.applies())
        .collect()
}

// ===========================================================================
// Every rename at its edges
// ===========================================================================

/// For every applied change, on the day before it the OLD stem is filed under
/// the chain's newest symbol and the NEW stem is refused as not yet in use;
/// on the day of it the old stem is refused as no longer in use and the new
/// stem is filed. Both stems reach one newest symbol, and walking back from
/// it passes through the row.
#[test]
fn every_applied_rename_files_and_refuses_on_exactly_its_own_edges() {
    let rows = applied();
    assert!(
        rows.len() >= 20,
        "the table holds its applied rows: {}",
        rows.len()
    );
    for row in rows {
        let eve = before(row.effective);
        let newest = LINEAGE.current(row.new);
        assert_eq!(
            LINEAGE.current(row.old),
            newest,
            "{} -> {}",
            row.old,
            row.new
        );
        assert_eq!(
            LINEAGE.filed_under(row.old, eve),
            Ok(newest),
            "{} on {eve}, the eve of its change",
            row.old
        );
        assert_eq!(
            LINEAGE.filed_under(row.old, row.effective),
            Err(Stale::AfterChange(row)),
            "{} on {}, its change day",
            row.old,
            row.effective
        );
        assert_eq!(
            LINEAGE.filed_under(row.new, eve),
            Err(Stale::BeforeChange(row)),
            "{} on {eve}, before it existed",
            row.new
        );
        assert_eq!(
            LINEAGE.filed_under(row.new, row.effective),
            Ok(newest),
            "{} on {}, its first day",
            row.new,
            row.effective
        );
        assert!(
            LINEAGE.backward(newest).any(|hop| *hop == row),
            "walking back from {newest} passes {} -> {}",
            row.old,
            row.new
        );
    }
}

/// The table is a function of the exchange's history: no share renamed to
/// itself, no symbol leaving twice or arriving twice among the applied rows,
/// every effective day a real date, and a chain's links in date order.
#[test]
fn the_rename_table_is_consistent_with_itself() {
    let rows = applied();
    for row in &rows {
        assert_ne!(row.old, row.new, "a self-rename");
        assert!(
            real(row.effective),
            "{}: {} is not a date",
            row.old,
            row.effective
        );
        assert!(Symbol::new(row.old).is_ok() && Symbol::new(row.new).is_ok());
    }
    for (i, a) in rows.iter().enumerate() {
        for b in &rows[i + 1..] {
            assert_ne!(a.old, b.old, "{} leaves twice", a.old);
            assert_ne!(a.new, b.new, "{} arrives twice", a.new);
            if a.new == b.old {
                assert!(
                    a.effective < b.effective,
                    "{} -> {} -> {}",
                    a.old,
                    a.new,
                    b.new
                );
            }
            if b.new == a.old {
                assert!(
                    b.effective < a.effective,
                    "{} -> {} -> {}",
                    b.old,
                    b.new,
                    a.new
                );
            }
        }
    }
}

/// The swept F&O list (a snapshot re-measured 2026-08-01) names no symbol the
/// rename table retired before that day: every swept underlying is the
/// newest symbol of its own chain.
#[test]
fn the_swept_list_names_no_retired_symbol() {
    for symbol in FNO_UNDERLYINGS {
        assert_eq!(
            LINEAGE.current(symbol),
            symbol,
            "{symbol} is swept under a name the rename table retired"
        );
    }
}

// ===========================================================================
// Symbols: case folds, look-alikes never do
// ===========================================================================

/// Case folds (`m&m` is `M&M`), and nothing else does: a full-width letter, a
/// Cyrillic look-alike, a zero-width or non-breaking space, a tab, a carriage
/// return, a line feed and a trailing space are each refused, so no two
/// spellings of one name become two instruments and no look-alike becomes
/// the real one.
#[test]
fn a_symbol_folds_case_and_refuses_every_look_alike() {
    assert_eq!(Symbol::new("m&m"), Symbol::new("M&M"));
    assert_eq!(Symbol::new("bajaj-auto"), Symbol::new("BAJAJ-AUTO"));
    assert_eq!(Symbol::new("gvt&d").unwrap().as_str(), "GVT&D");
    for look_alike in [
        "\u{ff2e}\u{ff29}\u{ff26}\u{ff34}\u{ff39}",
        "NIFT\u{0423}",
        "\u{0410}DANIPORTS",
        "\u{200b}NIFTY",
        "NIF\u{00a0}TY",
        "NIFTY\t",
        "NIFTY\r",
        "NIFTY\n",
        "M&M ",
        " M&M",
        "M\u{ff06}M",
    ] {
        assert!(
            Symbol::new(look_alike).is_err(),
            "{look_alike:?} must be refused, never read as an ASCII symbol"
        );
    }
    let widest = format!("{}&-_", "A".repeat(SYMBOL_CAPACITY - 3));
    assert_eq!(Symbol::new(&widest).unwrap().len(), SYMBOL_CAPACITY);
    assert!(Symbol::new(&format!("{widest}A")).is_err());
}

// ===========================================================================
// Expiry dates at the century years
// ===========================================================================

/// An expiry on 29 February exists in 2000 (divisible by 400) and 2096, and
/// not in 2100 (a century year, not divisible by 400) or 2023; the range
/// ends are 1990 and 2100.
#[test]
fn an_expiry_on_twenty_nine_february_follows_the_century_rule() {
    for (y, ok) in [
        (2000, true),
        (2096, true),
        (2024, true),
        (2100, false),
        (2023, false),
    ] {
        assert_eq!(Expiry::new(y, 2, 29).is_ok(), ok, "29 February {y}");
    }
    assert!(Expiry::new(1990, 1, 1).is_ok());
    assert!(Expiry::new(1989, 12, 31).is_err());
    assert!(Expiry::new(2100, 12, 31).is_ok());
    assert!(Expiry::new(2101, 1, 1).is_err());
}

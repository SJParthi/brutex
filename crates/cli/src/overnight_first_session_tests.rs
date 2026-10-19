#![cfg(test)]
//! The overnight line measures the move into the slice's first session from
//! the session stored before it, and says so when it cannot (corp-8).
//!
//! Generated stores only: invented constant-price sessions, written through
//! the store's own writer.
#![allow(clippy::expect_used)]

use super::*;
use store::format::Bar;

fn session(month: u8, date: u8, price: i64, volume: i64) -> Vec<Bar> {
    let civil = pull::session::Day::new(2025, month, date).expect("date");
    let day = i64::from(civil.days_from_epoch());
    let DayKind::Open(session) = pull::calendar::kind_of(day) else {
        return Vec::new();
    };
    session
        .windows
        .iter()
        .take(usize::from(session.count))
        .flat_map(|window| window.from..=window.to)
        .step_by(5)
        .map(|minute| Bar {
            ts_micros: day * 86_400_000_000 + i64::from(minute) * 60_000_000
                - indicators::IST_OFFSET_MICROS,
            open: price,
            high: price,
            low: price,
            close: price,
            volume,
            open_interest: i64::MIN,
        })
        .collect()
}

fn write(root: &Path, symbol: &str, month: u8, rows: &[Bar]) {
    let key = swept_index(symbol).expect("key");
    let path = StorePath::for_key(
        Vendor::Zerodha,
        &key,
        Timeframe::MINUTE_5,
        YearMonth::new(2025, month).expect("month"),
        FileKind::Bars,
    )
    .expect("path");
    let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
    let id = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]);
    BarFile::open_or_create(root, path, id)
        .expect("writer")
        .append(rows)
        .expect("generated rows");
}

fn candles(rows: &[Bar]) -> Vec<Candle> {
    rows.iter()
        .map(|bar| Candle {
            ts_micros: bar.ts_micros,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
            open_interest: bar.open_interest,
        })
        .collect()
}

fn scratch(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-overnight-first-session-{name}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("scratch");
    root
}

/// May 2025 for an invented share: its first session (2 May) opens 20% below
/// the last April close, and a later session moves 5%. The month alone named
/// the 5% move; with April stored, the line names the -20.00% move into
/// 2 May, from the close on 30 April.
#[test]
fn the_overnight_line_measures_the_move_into_the_first_session() {
    let root = scratch("measured");
    let april = session(4, 30, 100_000, 10);
    write(&root, "RELIANCE", 4, &april);
    let mut may = session(5, 2, 80_000, 10);
    may.extend(session(5, 5, 76_000, 10));
    write(&root, "RELIANCE", 5, &may);
    let key = swept_index("RELIANCE").expect("key");
    let note = overnight_note(&root, Vendor::Zerodha, &key, "5min", &candles(&may));
    let flat = note.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains(
            "-20.00% into the 2025-05-02 session (close 1000.00 on 2025-04-30 to open 800.00)"
        ),
        "{note}"
    );
    assert!(!note.contains("NOT MEASURED"), "{note}");
    assert!(!note.contains("traded no volume"), "{note}");

    // An index gets no line and reads nothing.
    let index = swept_index("NIFTY").expect("key");
    assert_eq!(
        overnight_note(&root, Vendor::Zerodha, &index, "5min", &candles(&may)),
        ""
    );
}

/// The first stored month has no session before it: the line still names the
/// largest move it could measure and SAYS the move into the first session was
/// not measured, and why.
#[test]
fn the_prior_month_absent_is_said() {
    let root = scratch("absent");
    let mut may = session(5, 2, 80_000, 10);
    may.extend(session(5, 5, 76_000, 10));
    write(&root, "RELIANCE", 5, &may);
    let key = swept_index("RELIANCE").expect("key");
    let note = overnight_note(&root, Vendor::Zerodha, &key, "5min", &candles(&may));
    let flat = note.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("-5.00% into the 2025-05-05"), "{note}");
    assert!(
        flat.contains(
            "THE MOVE INTO THE FIRST SESSION HERE (2025-05-02) WAS NOT MEASURED: no 5min month 2025-04 is stored before it."
        ),
        "{note}"
    );
    // One session and no prior: none measured, and the first session named.
    let one = candles(&session(5, 2, 80_000, 10));
    let note = overnight_note(&root, Vendor::Zerodha, &key, "5min", &one);
    assert!(
        note.starts_with(runner::audit::NO_OVERNIGHT_MEASURED),
        "{note}"
    );
    assert!(note.contains("WAS NOT MEASURED"), "{note}");
    // No bars: nothing to measure into, nothing read.
    assert_eq!(
        overnight_note(&root, Vendor::Zerodha, &key, "5min", &[]),
        runner::audit::NO_OVERNIGHT_MEASURED
    );
}

/// A move whose open bar traded no volume is marked as possibly never traded.
#[test]
fn a_zero_volume_open_is_marked_never_traded() {
    let root = scratch("untraded");
    write(&root, "RELIANCE", 4, &session(4, 30, 100_000, 10));
    let mut may = session(5, 2, 122_850, 10);
    if let Some(first) = may.first_mut() {
        first.volume = 0;
    }
    write(&root, "RELIANCE", 5, &may);
    let key = swept_index("RELIANCE").expect("key");
    let note = overnight_note(&root, Vendor::Zerodha, &key, "5min", &candles(&may));
    let flat = note.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("+22.85% into the 2025-05-02"), "{note}");
    assert!(
        flat.contains("Its open bar traded no volume: the price may never have traded."),
        "{note}"
    );
}

/// THE LARGEST MOVE TO A TRADED OPEN IS NAMED BESIDE A MARKED ONE (AH-01
/// repair round 1). TMPV 2025-10-14 on Zerodha: the line named +22.85% off an
/// untraded open, and the session's first trade, 12.72% below the prior close,
/// was named nowhere. An invented share in that shape -- 2 May's open bar
/// untraded at +22.85%, its first trade at an invented -11.84%, 5 May 5% higher -- names
/// both, the marked one first; with nothing traded after the prior close the
/// line says none; a traded largest move carries no second line.
#[test]
fn the_largest_traded_move_is_named_beside_an_untraded_one() {
    let root = scratch("traded-beside");
    write(&root, "RELIANCE", 4, &session(4, 30, 100_000, 10));
    let mut may = session(5, 2, 88_160, 10);
    if let Some(first) = may.first_mut() {
        *first = Bar {
            open: 122_850,
            high: 122_850,
            low: 122_850,
            close: 122_850,
            volume: 0,
            ..*first
        };
    }
    let later = session(5, 5, 92_568, 10);
    assert!(!later.is_empty(), "premise: 2025-05-05 is a session");
    may.extend(later);
    write(&root, "RELIANCE", 5, &may);
    let key = swept_index("RELIANCE").expect("key");
    let note = overnight_note(&root, Vendor::Zerodha, &key, "5min", &candles(&may));
    let flat = note.split_whitespace().collect::<Vec<_>>().join(" ");
    let marked = flat
        .find("+22.85% into the 2025-05-02")
        .expect("the marked move");
    let named = "THE LARGEST MOVE TO A TRADED OPEN: -11.84% into the 2025-05-02 session \
                 (close 1000.00 on 2025-04-30 to the session's first traded open, 881.60).";
    assert!(flat.contains(named), "the traded move is named: {note}");
    let traded = flat.find(named).expect("the traded move");
    assert!(marked < traded, "the marked move first: {note}");

    // Nothing traded after the prior month's close: the line says none.
    let root = scratch("traded-none");
    write(&root, "RELIANCE", 4, &session(4, 30, 100_000, 10));
    let lone = session(5, 2, 122_850, 0);
    write(&root, "RELIANCE", 5, &lone);
    let note = overnight_note(&root, Vendor::Zerodha, &key, "5min", &candles(&lone));
    assert!(note.contains("traded no volume"), "premise: {note}");
    assert!(note.contains(runner::audit::NO_TRADED_OVERNIGHT), "{note}");

    // A traded largest move carries no second line.
    let root = scratch("traded-only");
    write(&root, "RELIANCE", 4, &session(4, 30, 100_000, 10));
    let open = session(5, 2, 122_850, 10);
    write(&root, "RELIANCE", 5, &open);
    let note = overnight_note(&root, Vendor::Zerodha, &key, "5min", &candles(&open));
    assert!(
        !note.contains("THE LARGEST MOVE TO A TRADED OPEN"),
        "{note}"
    );
}

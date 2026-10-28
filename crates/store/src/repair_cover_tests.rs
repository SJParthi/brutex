#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
//! BASE-COV-LIBS: the repair door's refusal sentences no other test reads,
//! and the one bar comparison every door asks, field by field. Every price
//! is invented.

use super::*;
use crate::format::OI_NULL;

fn bar(open_interest: i64) -> Bar {
    Bar {
        ts_micros: 1_900_000_000_000_000,
        open: 101,
        high: 109,
        low: 97,
        close: 103,
        volume: 11,
        open_interest,
    }
}

#[test]
fn every_drop_and_restate_refusal_reads_as_its_own_sentence() {
    let cases = [
        (RepairError::NoBasisWitness, "no offered bar shares a stamp"),
        (
            RepairError::IndexedSibling(PathBuf::from("/brutex-cover/x.tix")),
            "repair refused: /brutex-cover/x.tix is indexed by record",
        ),
        (
            RepairError::UnheldStamp(-5),
            "drop refused: the month does not hold a bar at -5",
        ),
        (
            RepairError::WholeMonth { bars: 4 },
            "drop names every one of the month's 4 bars",
        ),
        (RepairError::NothingNamed, "it names no bar"),
        (
            RepairError::Reason("no reason".to_owned()),
            "refused: no reason; nothing was written",
        ),
        (
            RepairError::ReadBack { index: 7 },
            "record 7 of the published revision does not read back",
        ),
        (
            RepairError::QuarantineUnknown(crate::quarantine::QuarantineError::Reason(
                "bad".to_owned(),
            )),
            "whether the month is quarantined is not known (",
        ),
    ];
    for (refusal, said) in cases {
        let text = refusal.to_string();
        assert!(text.contains(said), "{text}");
        assert!(
            text.contains("refused") || text.contains("drop names"),
            "{text}"
        );
    }
}

#[test]
fn a_bar_agrees_only_when_every_field_does() {
    let held = bar(5);
    assert!(same_bar(&held, &held, true));
    let fields: [fn(&mut Bar); 7] = [
        |b| b.ts_micros += 1,
        |b| b.open += 1,
        |b| b.high += 1,
        |b| b.low -= 1,
        |b| b.close += 1,
        |b| b.volume += 1,
        |b| b.open_interest += 1,
    ];
    for (at, change) in fields.iter().enumerate() {
        let mut offered = held;
        change(&mut offered);
        assert!(!same_bar(&held, &offered, true), "field {at}");
        assert!(!same_bar(&held, &offered, false), "field {at}");
    }
    // Where open interest is no measurement, a held 0 and an offered null are
    // one absence, both ways; a real figure against either is not.
    assert!(same_bar(&bar(0), &bar(OI_NULL), false));
    assert!(same_bar(&bar(OI_NULL), &bar(0), false));
    assert!(!same_bar(&bar(0), &bar(OI_NULL), true));
    assert!(!same_bar(&bar(5), &bar(OI_NULL), false));
    assert!(!same_bar(&bar(OI_NULL), &bar(5), false));
    assert!(!same_bar(&bar(0), &bar(5), false));
    // The absence agrees, but another field does not: still no agreement.
    let mut moved = bar(OI_NULL);
    moved.close += 1;
    assert!(!same_bar(&bar(0), &moved, false));
}

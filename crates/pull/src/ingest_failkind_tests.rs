//! IA-16's tests of the failure kind: every store and repair refusal by its
//! row of design §5.8's table (IA-GR-39), the one repair-reason rule
//! (IA-GR-42), and what each kind does to a day (a hole, held back,
//! repairable). Every value is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail; a table of every variant is as long as \
              the enum"
)]

use std::io::ErrorKind;
use std::path::PathBuf;

use store::file::{Action, Conflict, SealedOffer, StoreError};
use store::format::{Bar, FormatError};
use store::path::{FileKind, YearMonth};
use store::repair::RepairError;

use super::{DoorRefusal, FailKind, MAX_REPAIR_REASON, repair_reason_ok};

fn p() -> PathBuf {
    PathBuf::from("/invented/store/bars/x.bars")
}

/// A sealed month's refusal of a batch whose raw form would meet `offer`.
fn sealed(offer: SealedOffer) -> StoreError {
    StoreError::Sealed {
        path: PathBuf::from("/invented/store/bars/x.sbar"),
        n_valid: 375,
        last_ts_micros: 9,
        at: 0,
        ts_micros: 11,
        offer,
    }
}

/// Every variant of [`StoreError`] this build knows, each with the kind its
/// row of §5.8's table gives it. Adding a variant to the store without a row
/// here leaves [`FailKind::of_store`]'s wildcard to answer it `Other`, which
/// the test below refuses.
fn every_store_refusal() -> Vec<(StoreError, FailKind)> {
    let io = |kind| FailKind::Io {
        kind,
        publication_may_be_visible: false,
    };
    let month = YearMonth::new(2019, 4).unwrap();
    vec![
        (StoreError::Locked { path: p() }, FailKind::MonthBusy),
        (
            StoreError::DiskFull {
                path: p(),
                action: Action::Write,
            },
            FailKind::DiskFull,
        ),
        (
            StoreError::OverlapDisagrees {
                path: p(),
                at: 3,
                ts_micros: 7,
                conflict: Conflict::NotHeld,
            },
            FailKind::NotHeld,
        ),
        (
            StoreError::OverlapDisagrees {
                path: p(),
                at: 3,
                ts_micros: 7,
                conflict: Conflict::Restated,
            },
            FailKind::Differs,
        ),
        (
            StoreError::OverlapDisagrees {
                path: p(),
                at: 3,
                ts_micros: 7,
                conflict: Conflict::Skipped,
            },
            FailKind::Differs,
        ),
        (
            StoreError::Superseded {
                path: p(),
                ordinal: 2,
            },
            FailKind::Superseded,
        ),
        // SC-4 (D-5904), IA-16 repair 2 (D-7131): a sealed month is answered
        // as its raw form would be. `Follows` is its successor generation;
        // each `Disagrees` arm is the raw form's `OverlapDisagrees` row.
        (sealed(SealedOffer::Follows), FailKind::Sealed),
        (
            sealed(SealedOffer::Disagrees(Conflict::NotHeld)),
            FailKind::NotHeld,
        ),
        (
            sealed(SealedOffer::Disagrees(Conflict::Restated)),
            FailKind::Differs,
        ),
        (
            sealed(SealedOffer::Disagrees(Conflict::Skipped)),
            FailKind::Differs,
        ),
        (
            StoreError::Io {
                path: p(),
                action: Action::Read,
                kind: ErrorKind::Other,
                code: Some(5),
            },
            io(ErrorKind::Other),
        ),
        (
            StoreError::Io {
                path: p(),
                action: Action::Write,
                kind: ErrorKind::StorageFull,
                code: Some(28),
            },
            FailKind::DiskFull,
        ),
        (
            StoreError::BarrierFailed { path: p() },
            io(ErrorKind::Other),
        ),
        (
            StoreError::ShortWrite {
                path: p(),
                offset: 0,
                asked: 64,
                wrote: 3,
            },
            io(ErrorKind::WriteZero),
        ),
        (
            StoreError::ShortRead {
                path: p(),
                offset: 0,
                asked: 64,
                read: 3,
            },
            io(ErrorKind::UnexpectedEof),
        ),
        (
            StoreError::Missing {
                path: p(),
                action: Action::Open,
            },
            io(ErrorKind::NotFound),
        ),
        (
            StoreError::Denied {
                path: p(),
                action: Action::Open,
            },
            io(ErrorKind::PermissionDenied),
        ),
        (
            StoreError::ReadOnly {
                path: p(),
                action: Action::Write,
            },
            io(ErrorKind::ReadOnlyFilesystem),
        ),
        (
            StoreError::RaggedTail {
                path: p(),
                len: 70,
                extra: 6,
            },
            FailKind::StoreDamaged,
        ),
        (
            StoreError::Format {
                path: p(),
                source: FormatError::OffsetOverflow,
            },
            FailKind::StoreDamaged,
        ),
        (
            StoreError::SymbolMismatch {
                path: p(),
                stored: 1,
                asked: 2,
            },
            FailKind::StoreDamaged,
        ),
        (
            StoreError::TimeframeMismatch {
                path: p(),
                stored: 1,
                asked: 60,
            },
            FailKind::StoreDamaged,
        ),
        (
            StoreError::BlockChecksum {
                path: p(),
                block: 0,
                stored: 1,
                computed: 2,
            },
            FailKind::StoreDamaged,
        ),
        (
            StoreError::ChecksumsMissing {
                path: p(),
                sidecar: p(),
                block: 0,
            },
            FailKind::StoreDamaged,
        ),
        (
            StoreError::NotCommitted {
                index: 9,
                n_valid: 3,
            },
            FailKind::StoreDamaged,
        ),
        (
            StoreError::CommittedRecordsLost {
                path: p(),
                sidecar: p(),
                sidecar_len: 0,
            },
            FailKind::StoreDamaged,
        ),
        (
            StoreError::TimeIndex {
                path: p(),
                reason: "invented".to_owned(),
            },
            FailKind::StoreDamaged,
        ),
        // Round 5, SF-2: WP-39's sort index.
        (
            StoreError::SortIndex {
                path: p(),
                reason: "invented".to_owned(),
            },
            FailKind::StoreDamaged,
        ),
        (
            StoreError::Generation {
                path: p(),
                reason: "invented".to_owned(),
            },
            FailKind::StoreDamaged,
        ),
        (
            StoreError::IsADirectory {
                path: p(),
                action: Action::Open,
            },
            FailKind::StoreDamaged,
        ),
        (
            StoreError::NotADirectory {
                path: p(),
                action: Action::Open,
            },
            FailKind::StoreDamaged,
        ),
        (
            StoreError::NotARegularFile {
                path: p(),
                action: Action::Open,
            },
            FailKind::StoreDamaged,
        ),
        (StoreError::Symlinked { path: p() }, FailKind::StoreDamaged),
        (
            StoreError::NotABarPath {
                found: FileKind::Checksums,
            },
            FailKind::StoreDamaged,
        ),
        (
            StoreError::Misfiled {
                path: p(),
                sidecar: p(),
                field: "symbol_id",
            },
            FailKind::StoreDamaged,
        ),
        // THE SEALED KIND (SC-3/SC-4, compressed store): a sealed month that
        // fails its own check is the month's own fault: `store_damaged`.
        (
            StoreError::SealedRefused {
                path: p(),
                reason: "sealed_crc",
                detail: "invented".to_owned(),
            },
            FailKind::StoreDamaged,
        ),
        (StoreError::EmptyBatch, FailKind::BatchRefused),
        (
            StoreError::BatchNotOrdered {
                at: 1,
                previous: 9,
                next: 3,
            },
            FailKind::BatchRefused,
        ),
        (
            StoreError::ImpossibleCount {
                at: 1,
                volume: -1,
                open_interest: 0,
            },
            FailKind::BatchRefused,
        ),
        (StoreError::ImpossibleBar { at: 1 }, FailKind::BatchRefused),
        (
            StoreError::OutsideMonth {
                at: 1,
                ts_micros: 0,
                month,
            },
            FailKind::BatchRefused,
        ),
        (
            StoreError::OffGrid {
                at: 1,
                ts_micros: 7,
                timeframe_secs: 1,
            },
            FailKind::BatchRefused,
        ),
    ]
}

/// How many variants `store::file::StoreError` declares, read from the
/// store's own source: a line of four spaces, a capitalised name and ` {` or
/// `,` inside the enum's block.
fn store_variants() -> usize {
    let source = include_str!("../../store/src/file.rs");
    let start = source.find("pub enum StoreError {").unwrap();
    let block = &source[start..];
    let end = block.find("\n}\n").unwrap();
    block[..end]
        .lines()
        .filter(|line| {
            line.strip_prefix("    ").is_some_and(|rest| {
                rest.starts_with(|c: char| c.is_ascii_uppercase())
                    && (rest.ends_with(" {") || rest.ends_with(','))
                    && rest
                        .trim_end_matches([' ', '{', ','])
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric())
            })
        })
        .count()
}

fn invented_bar() -> Bar {
    Bar {
        ts_micros: 1,
        open: 777_700,
        high: 777_700,
        low: 777_700,
        close: 777_700,
        volume: 0,
        open_interest: i64::MIN,
    }
}

fn every_repair_refusal() -> Vec<(RepairError, FailKind)> {
    vec![
        (
            RepairError::Store(StoreError::Locked { path: p() }),
            FailKind::MonthBusy,
        ),
        (
            RepairError::Store(StoreError::DiskFull {
                path: p(),
                action: Action::Write,
            }),
            FailKind::DiskFull,
        ),
        (RepairError::StaleSource, FailKind::MonthBusy),
        (
            RepairError::RowLimit {
                rows: 3_000_000,
                ceiling: 2_678_400,
            },
            FailKind::RepairRefused,
        ),
        (RepairError::NoBasisWitness, FailKind::RepairRefused),
        (RepairError::UnsealedSource, FailKind::RepairRefused),
        (RepairError::Conflict, FailKind::RepairRefused),
        (RepairError::Incomplete(p()), FailKind::RepairRefused),
        (RepairError::InvalidReceipt(p()), FailKind::RepairRefused),
        // R4-18's drop and restate doors.
        (RepairError::UnheldStamp(7), FailKind::RepairRefused),
        (RepairError::WholeMonth { bars: 4 }, FailKind::RepairRefused),
        (RepairError::NothingNamed, FailKind::RepairRefused),
        (
            RepairError::Reason("invented".to_owned()),
            FailKind::RepairRefused,
        ),
        (
            RepairError::ReadBack { index: 3 },
            FailKind::Io {
                kind: ErrorKind::InvalidData,
                publication_may_be_visible: true,
            },
        ),
        (
            RepairError::HeldBarDiffers {
                ts_micros: 1,
                held: Box::new(invented_bar()),
                offered: Box::new(invented_bar()),
            },
            FailKind::RepairImpossible,
        ),
        (RepairError::IndexedSibling(p()), FailKind::RepairImpossible),
        (RepairError::RevisionLimit, FailKind::RepairImpossible),
        // R4-18 repair 2: the doors that add or restate bars.
        (
            RepairError::Quarantined(store::quarantine::Entry {
                scope: store::quarantine::Scope::WholeMonth,
                cause: store::quarantine::Cause::BeforeListing,
                causes: store::quarantine::Causes::of(store::quarantine::Cause::BeforeListing),
                bars: 4,
                ledger_at: 28,
                revision: 0,
                written_at: 1,
            }),
            FailKind::RepairImpossible,
        ),
        (
            RepairError::QuarantineUnknown(store::quarantine::QuarantineError::Stale {
                path: p(),
                built_from: 28,
                ledger_len: 300,
            }),
            FailKind::StoreDamaged,
        ),
        (
            RepairError::QuarantineUnknown(store::quarantine::QuarantineError::BeingWritten {
                path: p(),
                built_from: 28,
                ledger_len: 300,
                waited_ms: 0,
            }),
            FailKind::MonthBusy,
        ),
        (RepairError::MissingTimestamp(7), FailKind::BatchRefused),
        (
            RepairError::BeforePromote {
                revision_root: p(),
                why: "invented".to_owned(),
            },
            FailKind::Io {
                kind: ErrorKind::Other,
                publication_may_be_visible: true,
            },
        ),
        (
            RepairError::Io {
                path: p(),
                operation: "rename",
                kind: ErrorKind::Other,
                publication_may_be_visible: true,
            },
            FailKind::Io {
                kind: ErrorKind::Other,
                publication_may_be_visible: true,
            },
        ),
        (
            RepairError::Io {
                path: p(),
                operation: "write",
                kind: ErrorKind::Other,
                publication_may_be_visible: false,
            },
            FailKind::Io {
                kind: ErrorKind::Other,
                publication_may_be_visible: false,
            },
        ),
        (
            RepairError::Io {
                path: p(),
                operation: "write",
                kind: ErrorKind::StorageFull,
                publication_may_be_visible: false,
            },
            FailKind::DiskFull,
        ),
    ]
}

/// **IA-GR-39 (D-4851, D-4890; design §5.8 "Every store and repair refusal,
/// by kind").** Each variant of the store's and the repair's refusals maps
/// to its row, and none to [`FailKind::Other`]; the code a refused file is
/// named by is the row's.
#[test]
fn every_store_and_repair_refusal_maps_to_its_row() {
    let store = every_store_refusal();
    // 42 rows: the 36 variants, the conflict's three arms, the sealed
    // offer's four arms (SC-4: `Follows` and the raw form's three
    // conflicts) and the full disk's I/O counted apart.
    assert_eq!(store.len(), 42, "one row per variant arm");
    // AND THE STORE HAS NO VARIANT THIS TABLE LACKS: counted from the
    // store's own source, so a variant added there stays red here until it
    // is given its row (and the count below is raised with it).
    assert_eq!(
        store_variants(),
        36,
        "a StoreError variant was added: give it its row"
    );
    for (why, kind) in &store {
        let got = FailKind::of_store(why);
        assert_eq!(got, *kind, "{why:?}");
        assert_ne!(got, FailKind::Other, "{why:?} reached the wildcard");
        // The repair's own wrapper of a store refusal answers the same.
        assert_eq!(FailKind::of_repair(&RepairError::Store(why.clone())), *kind);
        // The door's refusal keeps the store's own words.
        let door = DoorRefusal::store(why);
        assert_eq!(door.kind, *kind);
        assert_eq!(door.why, why.to_string());
    }
    for (why, kind) in every_repair_refusal() {
        let got = FailKind::of_repair(&why);
        assert_eq!(got, kind, "{why:?}");
        assert_ne!(got, FailKind::Other, "{why:?}");
    }
}

/// The codes are the table's, never renamed or reused: one per kind, all
/// distinct, and `Other` alone answers `filing`.
#[test]
fn every_kind_has_its_one_code() {
    let kinds = [
        (FailKind::MonthBusy, "month_busy"),
        (FailKind::DiskFull, "disk_full"),
        (FailKind::NotHeld, "needs_month_generation"),
        (FailKind::Superseded, "month_superseded"),
        (FailKind::Differs, "bars_differ_held"),
        (FailKind::RepairRefused, "repair_refused"),
        (FailKind::RepairImpossible, "repair_impossible"),
        (
            FailKind::Io {
                kind: ErrorKind::Other,
                publication_may_be_visible: false,
            },
            "store_io",
        ),
        (FailKind::StoreDamaged, "store_damaged"),
        (FailKind::BatchRefused, "batch_refused"),
        (FailKind::Sealed, "month_sealed"),
        (FailKind::Other, "filing"),
    ];
    let mut seen = std::collections::HashSet::new();
    for (kind, code) in kinds {
        assert_eq!(kind.code(), code);
        assert!(seen.insert(code), "{code} named twice");
    }
}

/// What each kind does to a day (design §5.5, §5.8): exactly the five store
/// holes are holes; only `NotHeld` and `Superseded` are handed to the
/// repair by a repair reason (D-4890), and only `Sealed` to the successor
/// door with or without one (D-7131); every kind that is not a hole and not
/// a full disk holds the month's later days back (D-4851); a visible
/// publication is never a hole.
#[test]
fn each_kind_is_a_hole_holds_back_or_is_repaired_by_its_row() {
    let io = |visible| FailKind::Io {
        kind: ErrorKind::Other,
        publication_may_be_visible: visible,
    };
    // (kind, hole, repairable, holds back, filed as a successor)
    let rows = [
        (FailKind::MonthBusy, false, false, true, false),
        (FailKind::DiskFull, false, false, false, false),
        (FailKind::NotHeld, true, true, false, false),
        (FailKind::Superseded, true, true, false, false),
        (FailKind::Differs, true, false, false, false),
        (FailKind::RepairRefused, true, false, false, false),
        (FailKind::RepairImpossible, true, false, false, false),
        (io(false), false, false, true, false),
        (io(true), false, false, true, false),
        (FailKind::StoreDamaged, false, false, true, false),
        (FailKind::BatchRefused, false, false, true, false),
        (FailKind::Sealed, false, false, true, true),
        (FailKind::Other, false, false, true, false),
    ];
    for (kind, hole, repairable, holds_back, successor) in rows {
        assert_eq!(kind.is_hole(), hole, "{kind:?} hole");
        assert_eq!(kind.repairable(), repairable, "{kind:?} repairable");
        assert_eq!(kind.holds_back(), holds_back, "{kind:?} holds back");
        assert_eq!(
            kind.files_as_successor(),
            successor,
            "{kind:?} filed as a successor"
        );
    }
}

/// **THE SEALED KIND (SC-4 D-5904, D-6247; IA-16 repair 2, D-7131).** The
/// raw form's answer, whatever the month's form or generation: a sealed
/// month's `Follows` and a revision-served month's `Follows` offer are one
/// kind, `month_sealed`, never a hole and never `filing`; a `Disagrees`
/// offer is the raw form's own overlap row.
#[test]
fn a_sealed_months_refusal_is_its_raw_forms_answer() {
    assert_eq!(FailKind::of_offer(SealedOffer::Follows), FailKind::Sealed);
    for conflict in [Conflict::NotHeld, Conflict::Restated, Conflict::Skipped] {
        let raw = FailKind::of_store(&StoreError::OverlapDisagrees {
            path: p(),
            at: 0,
            ts_micros: 11,
            conflict,
        });
        assert_eq!(FailKind::of_conflict(conflict), raw, "{conflict:?}");
        assert_eq!(
            FailKind::of_offer(SealedOffer::Disagrees(conflict)),
            raw,
            "{conflict:?}"
        );
        assert_eq!(
            FailKind::of_store(&sealed(SealedOffer::Disagrees(conflict))),
            raw,
            "{conflict:?}"
        );
    }
    let late = FailKind::of_store(&sealed(SealedOffer::Follows));
    assert_eq!(late, FailKind::Sealed);
    assert!(!late.is_hole() && late.files_as_successor());
    assert_ne!(late.code(), FailKind::Other.code());
    assert_eq!(late.code(), "month_sealed");
}

/// **IA-GR-42 (D-4874, round 6 SF-10): ONE REPAIR-REASON RULE FOR BOTH
/// FEEDS.** Not empty, at most 200 bytes, no control character; the bound is
/// in bytes, so a reason of multi-byte letters is measured as stored.
#[test]
fn one_repair_reason_rule_for_both_feeds() {
    assert_eq!(MAX_REPAIR_REASON, 200);
    assert!(!repair_reason_ok(""), "empty");
    assert!(
        !repair_reason_ok("   "),
        "blanks alone, as the route trims them"
    );
    assert!(repair_reason_ok(" a reason "), "blanks around words");
    assert!(repair_reason_ok("x"), "one byte");
    assert!(repair_reason_ok(&"r".repeat(200)), "exactly the bound");
    assert!(!repair_reason_ok(&"r".repeat(201)), "one past it");
    assert!(!repair_reason_ok("line one\nline two"), "a newline");
    assert!(!repair_reason_ok("a\ttab"), "a tab");
    assert!(!repair_reason_ok("a\rreturn"), "a carriage return");
    assert!(!repair_reason_ok("nul\0byte"), "a NUL");
    assert!(!repair_reason_ok("del\u{7f}"), "DEL is a control character");
    // 66 three-byte letters are 198 bytes; 67 are 201.
    assert!(repair_reason_ok(&"\u{20b9}".repeat(66)));
    assert!(!repair_reason_ok(&"\u{20b9}".repeat(67)));
    assert!(repair_reason_ok("operator: re-land NIFTY 2019-04 (D-3350)"));
    // IA-16 repair 2: marks that show nothing are not a reason, and a
    // Unicode line or paragraph separator is a second line.
    assert!(!repair_reason_ok("\u{200b}"), "a zero-width space alone");
    assert!(
        !repair_reason_ok(" \u{feff}\u{200d} "),
        "invisible marks alone"
    );
    assert!(repair_reason_ok("re\u{200b}land"), "a mark among words");
    assert!(!repair_reason_ok("one\u{2028}two"), "U+2028 is a new line");
    assert!(!repair_reason_ok("one\u{2029}two"), "U+2029 is a new line");
    assert!(!repair_reason_ok("\u{2028}"), "a separator alone");
}

/// A door's refusal with more words keeps its kind; its text is its words.
#[test]
fn a_door_refusal_keeps_its_kind_through_its_words() {
    let refused = DoorRefusal::batch("an invented address").and("; and more");
    assert_eq!(refused.kind, FailKind::BatchRefused);
    assert_eq!(refused.to_string(), "an invented address; and more");
    assert_eq!(String::from(refused), "an invented address; and more");
}

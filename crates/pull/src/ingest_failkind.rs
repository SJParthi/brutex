//! THE FAILURE KIND (IA-16; D-4851, D-4890, F31, F64; design §5.8 "Every
//! store and repair refusal, by kind").
//!
//! Until IA-16 every refusal of the store's two doors reached its caller as a
//! `String`: the GDFL importer could not tell a month a reader held for a
//! moment from a month whose checksums failed, nor a day the month never held
//! (a hole a repair fills) from a held bar read differently (a hole nothing
//! fills), so it named all of them `filing` and read them again on every
//! run. The kind is decided HERE, once, from the store's and the repair's
//! own typed refusals, and every caller acts on the kind and never on the
//! words.
//!
//! **One table, both doors, both feeds.** [`FailKind::of_store`] is the
//! table for [`store::file::StoreError`], [`FailKind::of_repair`] for
//! [`store::repair::RepairError`]. The repair door ([`super::repair_or_append`],
//! Zerodha's) and the back-fill door ([`super::back_fill_or_append`], GDFL's)
//! hand a month to the repair only when the append's kind is
//! [`FailKind::NotHeld`] or [`FailKind::Superseded`]: one repair, two doors
//! (D-4890).
//!
//! `StoreError` is `#[non_exhaustive]`, so the store's table ends in one
//! wildcard arm: [`FailKind::Other`]. IA-GR-39
//! (`every_store_and_repair_refusal_maps_to_its_row`) builds every variant
//! this build knows and fails if any reaches `Other`, so a variant added to
//! the store after this table stays red until it is given its row.

use std::io::ErrorKind;

use store::file::{Conflict, SealedOffer, StoreError};
use store::repair::RepairError;

/// What kind of refusal a store or repair door answered (design §5.8). `Copy`
/// and free of words: the words travel beside it in [`DoorRefusal::why`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailKind {
    /// Another process holds the month (a writer, or a reader's shared lock),
    /// or the repair found its source advanced under it. Retried for at most
    /// two seconds at the import's call site, then `month_busy`; held back.
    MonthBusy,
    /// The host said the disk is full: the run ends `disk_full`.
    DiskFull,
    /// The month never held a stamp inside its held range that the batch
    /// offers (`Conflict::NotHeld`): only a new generation lands it. A hole,
    /// `needs_month_generation`; a repair-reason run hands it to the repair.
    NotHeld,
    /// The month is served from a repair revision and the batch holds a new
    /// bar: only a further repair adds to it. A hole, `month_superseded`.
    Superseded,
    /// The month holds a bar inside the batch's span that the batch reads
    /// differently (`Conflict::Restated` or `Skipped`, or the back-fill
    /// door's overlap): a held bar is never changed. A hole,
    /// `bars_differ_held`, that no repair fills.
    Differs,
    /// The repair refused for a reason a later build or repair can lift
    /// (`RowLimit`, `NoBasisWitness`, `UnsealedSource`, `Conflict`,
    /// `Incomplete`, `InvalidReceipt`). A hole, `repair_refused`.
    RepairRefused,
    /// The repair refused for good (`HeldBarDiffers`, `IndexedSibling`,
    /// `RevisionLimit`). A hole, `repair_impossible`, no run reads again.
    RepairImpossible,
    /// The host refused a read or a write (`kind`), with the store's root
    /// still to be probed (the loss probes, D-4852). `publication_may_be_visible`
    /// is the repair's own flag: such a refusal is never a hole, because the
    /// next resume's byte-exact retry completes the revision (D-3350).
    Io {
        /// The host's kind.
        kind: ErrorKind,
        /// Whether a repair's revision may already be visible.
        publication_may_be_visible: bool,
    },
    /// The month refused to open or append for a fault of its own: a
    /// checksum, a format, a torn tail, a generation pointer that does not
    /// verify, a sidecar written for another month, a sealed form that fails
    /// its check. `store_damaged`: a named refusal on the journal close and the
    /// cli receipt, never an alert (D-6900: the unattended layer is dropped);
    /// never a hole; held back.
    StoreDamaged,
    /// The store refused a batch the importer built wrongly (empty,
    /// unordered, off grid, outside its month, an impossible bar), or a
    /// repair met a merge without a held stamp. `batch_refused`; an importer
    /// defect; held back.
    BatchRefused,
    /// A month the writer door never appends to, whose RAW form would commit
    /// the batch (IA-16 repair 2, D-7131): a month held in its sealed form
    /// (`StoreError::Sealed` with `SealedOffer::Follows`, SC-4 D-5904), or
    /// one served from a repair revision whose served generation says the
    /// same (`BarFile::offer`, D-6247). Both doors file the batch as the
    /// month's NEXT successor generation, with or without a repair reason:
    /// the raw month would append it, so it is never a hole, never `filing`
    /// and never waits on a person. `month_sealed` names it only when the
    /// successor itself could not be written before the repair (its
    /// `repairing` line refused), held back and read again by the next run;
    /// a refusal of the repair is the repair's own kind.
    Sealed,
    /// A refusal of a store variant added after this table. Never reached by
    /// a variant this build knows (IA-GR-39); `filing`, held back.
    Other,
}

impl FailKind {
    /// The kind of a store refusal (design §5.8's table, one row per variant).
    #[must_use]
    pub const fn of_store(why: &StoreError) -> Self {
        match why {
            StoreError::Locked { .. } => Self::MonthBusy,
            StoreError::DiskFull { .. } => Self::DiskFull,
            StoreError::OverlapDisagrees { conflict, .. } => Self::of_conflict(*conflict),
            // A SEALED MONTH IS ANSWERED AS ITS RAW FORM WOULD BE (SC-4,
            // D-5904): `Disagrees(c)` is the raw form's `OverlapDisagrees(c)`
            // row, and `Follows` its successor generation.
            StoreError::Sealed { offer, .. } => Self::of_offer(*offer),
            StoreError::Superseded { .. } => Self::Superseded,
            StoreError::Io { kind, .. } => Self::io(*kind),
            StoreError::BarrierFailed { .. } => Self::io(ErrorKind::Other),
            StoreError::ShortWrite { .. } => Self::io(ErrorKind::WriteZero),
            StoreError::ShortRead { .. } => Self::io(ErrorKind::UnexpectedEof),
            StoreError::Missing { .. } => Self::io(ErrorKind::NotFound),
            StoreError::Denied { .. } => Self::io(ErrorKind::PermissionDenied),
            StoreError::ReadOnly { .. } => Self::io(ErrorKind::ReadOnlyFilesystem),
            StoreError::RaggedTail { .. }
            | StoreError::Format { .. }
            | StoreError::SymbolMismatch { .. }
            | StoreError::TimeframeMismatch { .. }
            | StoreError::BlockChecksum { .. }
            | StoreError::ChecksumsMissing { .. }
            | StoreError::NotCommitted { .. }
            | StoreError::CommittedRecordsLost { .. }
            | StoreError::TimeIndex { .. }
            | StoreError::SortIndex { .. }
            | StoreError::Generation { .. }
            | StoreError::IsADirectory { .. }
            | StoreError::NotADirectory { .. }
            | StoreError::NotARegularFile { .. }
            | StoreError::Symlinked { .. }
            | StoreError::NotABarPath { .. }
            // DB-B65: the month's sidecar names another month; SC-3: the
            // month's sealed form fails its own check. Both are the month's
            // own fault, never the batch's and never the host's.
            | StoreError::Misfiled { .. }
            | StoreError::SealedRefused { .. } => Self::StoreDamaged,
            StoreError::EmptyBatch
            | StoreError::BatchNotOrdered { .. }
            | StoreError::ImpossibleCount { .. }
            | StoreError::ImpossibleBar { .. }
            | StoreError::OutsideMonth { .. }
            | StoreError::OffGrid { .. } => Self::BatchRefused,
            _ => Self::Other,
        }
    }

    /// The kind of an overlap the month holds otherwise
    /// (`StoreError::OverlapDisagrees`'s conflict): a stamp inside the held
    /// range the month never held is [`Self::NotHeld`]; a held stamp read
    /// differently, or one the batch skips, is [`Self::Differs`].
    #[must_use]
    pub const fn of_conflict(conflict: Conflict) -> Self {
        match conflict {
            Conflict::NotHeld => Self::NotHeld,
            Conflict::Restated | Conflict::Skipped => Self::Differs,
        }
    }

    /// The kind of what a month the writer door never appends to would have
    /// met in its raw form (`store::file::SealedOffer`, SC-4 D-5904, D-6247):
    /// [`Self::Sealed`] when the raw form would commit the batch, else the
    /// raw form's own overlap row ([`Self::of_conflict`]). One row for a
    /// sealed month's refusal and a revision-served month's offer, so the
    /// form and the generation are never the kind.
    #[must_use]
    pub const fn of_offer(offer: SealedOffer) -> Self {
        match offer {
            SealedOffer::Follows => Self::Sealed,
            SealedOffer::Disagrees(conflict) => Self::of_conflict(conflict),
        }
    }

    /// The kind of a repair refusal (design §5.8's table; round 5, SF-10
    /// splits the refused from the impossible).
    #[must_use]
    pub const fn of_repair(why: &RepairError) -> Self {
        match why {
            RepairError::Store(inner) => Self::of_store(inner),
            RepairError::StaleSource => Self::MonthBusy,
            RepairError::RowLimit { .. }
            | RepairError::NoBasisWitness
            | RepairError::UnsealedSource
            | RepairError::Conflict
            | RepairError::Incomplete(_)
            | RepairError::InvalidReceipt(_)
            // R4-18's drop and restate doors: a caller's malformed
            // request, refused before anything is reserved or written.
            | RepairError::UnheldStamp(_)
            | RepairError::WholeMonth { .. }
            | RepairError::NothingNamed
            | RepairError::Reason(_) => Self::RepairRefused,
            RepairError::HeldBarDiffers { .. }
            | RepairError::IndexedSibling(_)
            | RepairError::RevisionLimit
            // R4-18 repair 2: a month quarantined whole takes no bar, for
            // good; no later build or repair lifts a quarantine.
            | RepairError::Quarantined(_) => Self::RepairImpossible,
            // R4-18 repair 2: the feed's quarantine index or ledger cannot
            // be trusted (torn, stale, damaged), so which offered bars it
            // set aside is not known: a fault of the store, held back, read
            // again once a writer has rebuilt the index.
            RepairError::QuarantineUnknown(_) => Self::StoreDamaged,
            RepairError::MissingTimestamp(_) => Self::BatchRefused,
            // D-4898 (IA-17): the revision is published, complete and not
            // served; the same repair offered again finds it and promotes.
            // So it is the visible-publication row: never a hole.
            RepairError::BeforePromote { .. } => Self::Io {
                kind: ErrorKind::Other,
                publication_may_be_visible: true,
            },
            // R4-18 (the restate policy): the revision is published and its
            // read-back differs, so it is NOT promoted; like BeforePromote it
            // exists unserved, and the bytes the host returned are the fault.
            RepairError::ReadBack { .. } => Self::Io {
                kind: ErrorKind::InvalidData,
                publication_may_be_visible: true,
            },
            RepairError::Io {
                kind,
                publication_may_be_visible,
                ..
            } => {
                if matches!(kind, ErrorKind::StorageFull) {
                    Self::DiskFull
                } else {
                    Self::Io {
                        kind: *kind,
                        publication_may_be_visible: *publication_may_be_visible,
                    }
                }
            }
        }
    }

    /// An I/O refusal of `kind`, a full disk being its own kind.
    const fn io(kind: ErrorKind) -> Self {
        if matches!(kind, ErrorKind::StorageFull) {
            Self::DiskFull
        } else {
            Self::Io {
                kind,
                publication_may_be_visible: false,
            }
        }
    }

    /// The code a refused file is named by (design §5.8), never renamed or
    /// reused.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::MonthBusy => "month_busy",
            Self::DiskFull => "disk_full",
            Self::NotHeld => "needs_month_generation",
            Self::Superseded => "month_superseded",
            Self::Differs => "bars_differ_held",
            Self::RepairRefused => "repair_refused",
            Self::RepairImpossible => "repair_impossible",
            Self::Io { .. } => "store_io",
            Self::StoreDamaged => "store_damaged",
            Self::BatchRefused => "batch_refused",
            Self::Sealed => "month_sealed",
            Self::Other => "filing",
        }
    }

    /// Whether a file refused with this kind makes its day a HOLE: a day the
    /// month already passed, read again only by a repair-reason run (4a) or
    /// by none (4b). Never true of a refusal a retry can mend.
    #[must_use]
    pub const fn is_hole(self) -> bool {
        matches!(
            self,
            Self::NotHeld
                | Self::Superseded
                | Self::Differs
                | Self::RepairRefused
                | Self::RepairImpossible
        )
    }

    /// Whether the append's refusal is one the repair can answer: a month
    /// that never held the batch's stamps, or one served from a revision
    /// (D-4890: the only two the doors hand to `repair_month`).
    #[must_use]
    pub const fn repairable(self) -> bool {
        matches!(self, Self::NotHeld | Self::Superseded)
    }

    /// Whether the append's refusal is filed as the month's next successor
    /// generation by every door, with or without a repair reason
    /// ([`Self::Sealed`]; SC-4 D-5904, D-6247): the one refusal D-4889's
    /// gate does not hold back from the back-fill door, because the month's
    /// raw form would have appended the batch and no person is asked.
    #[must_use]
    pub const fn files_as_successor(self) -> bool {
        matches!(self, Self::Sealed)
    }

    /// Whether a later file of the same instrument-month in the same run
    /// must not be filed (D-4851): every kind that keeps the day open and
    /// is not a hole (a hole's month already holds later days, so holding
    /// them back would protect nothing). A full disk ends the run instead.
    #[must_use]
    pub const fn holds_back(self) -> bool {
        !self.is_hole() && !matches!(self, Self::DiskFull)
    }
}

/// A refusal of one of the store's doors: its kind and its words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoorRefusal {
    /// What kind of refusal it is.
    pub kind: FailKind,
    /// The store's or the repair's own words, and the door's.
    pub why: String,
}

impl DoorRefusal {
    /// The store's refusal, typed by [`FailKind::of_store`].
    #[must_use]
    pub fn store(why: &StoreError) -> Self {
        Self {
            kind: FailKind::of_store(why),
            why: why.to_string(),
        }
    }

    /// A refusal that is not the store's own: an address the importer built
    /// and the store's path cannot take is a batch built wrongly.
    #[must_use]
    pub fn batch(why: impl Into<String>) -> Self {
        Self {
            kind: FailKind::BatchRefused,
            why: why.into(),
        }
    }

    /// The same refusal, with more words after its own.
    #[must_use]
    pub fn and(self, more: &str) -> Self {
        Self {
            why: format!("{}{more}", self.why),
            ..self
        }
    }
}

impl core::fmt::Display for DoorRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.why)
    }
}

impl From<DoorRefusal> for String {
    fn from(refusal: DoorRefusal) -> Self {
        refusal.why
    }
}

/// The longest repair reason either feed takes, in bytes (D-4874, round 6
/// SF-10): the rule `POST /pull/repair` applied alone until IA-16, moved here
/// so a GDFL repair-reason run and a Zerodha repair are judged by one rule.
pub const MAX_REPAIR_REASON: usize = 200;

/// Whether `reason` is a repair reason: at least one character a reader can
/// see (not empty, not blanks alone as `POST /pull/repair` trims them, IA-16
/// repair 1, and not zero-width marks alone, repair 2), at most
/// [`MAX_REPAIR_REASON`] bytes, and one line: no control character (no tab)
/// and no Unicode line or paragraph separator (U+2028, U+2029), which
/// `char::is_control` does not count. The one repair-reason rule for both
/// feeds (D-4874, IA-GR-42).
#[must_use]
pub fn repair_reason_ok(reason: &str) -> bool {
    reason.chars().any(|c| !c.is_whitespace() && !zero_width(c))
        && reason.len() <= MAX_REPAIR_REASON
        && !reason
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
}

/// The marks that occupy no width when shown: the zero-width space, joiners
/// and direction marks (U+200B..U+200F), the word joiner and invisible
/// operators (U+2060..U+2064), the byte-order mark (U+FEFF) and the soft
/// hyphen (U+00AD). A reason of these alone reads as an empty one.
const fn zero_width(c: char) -> bool {
    matches!(
        c,
        '\u{200b}'..='\u{200f}' | '\u{2060}'..='\u{2064}' | '\u{feff}' | '\u{ad}'
    )
}

#[cfg(test)]
#[path = "ingest_failkind_tests.rs"]
mod tests;

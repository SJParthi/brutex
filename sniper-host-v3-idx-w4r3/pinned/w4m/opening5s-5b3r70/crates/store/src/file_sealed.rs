//! The read door's sealed arm: one [`BarFile`] path for a sealed and an
//! unsealed month (SC-3; store-compression design §3 and §4.4; D-4965).
//!
//! A closed month may be held in a second form, `<yyyy-mm>.sbar`
//! ([`crate::sealed`]), holding the same records bit for bit. Every reader of
//! the store comes through [`BarFile::open_existing`], so this is the one
//! place the form is decided:
//!
//! * **Three opens at most, no listing.** `.sbar`; on `ENOENT` the raw `.bin`;
//!   when that is `ENOENT` too, or its shared lock is refused, `.sbar` once
//!   more, because a retirement may have run between the two (design §4.7: the
//!   only holder of an exclusive lock on a closed month is a retirement, and
//!   then the sealed file is there).
//! * **No lock on a sealed month.** It is published once by rename and never
//!   opened for writing again, so there is no writer to wait for.
//! * **The same cache, the same bytes.** [`BarFile::read_row`] serves a sealed
//!   month from the handle's one cached block exactly as a raw one: the block
//!   is a frame of up to 256 rows, read with its directory entries in two
//!   positional reads, checked (keyed entry CRC, frame CRC, row count, first
//!   stamp) and decoded into 56-byte record images. `read_record`, the
//!   bisection, `already_stored` and every page see the same record bytes
//!   from either form, so `data_digest` and every run identity are unchanged.
//! * **Time lookups** use the sealed month's own time table
//!   ([`crate::sealed::SealedMonth::first_at_or_after`]); a sealed month needs
//!   no `.tix`.
//! * **Nothing is ever served from the other form instead.** A sealed month
//!   that fails a check is [`StoreError::SealedRefused`], named; falling back
//!   to the `.bin` would be the hidden fallback `CLAUDE.md` §4 bans.
//!
//! Writing to a sealed month is the write door's (SC-4); sealing is SC-6's.
//! Nothing here writes, moves or deletes a file.
//!
//! # Cost
//!
//! Opening a sealed month: one `open`, one `fstat`, one 512-byte read and a
//! `dup` of the descriptor. A row: one cached frame per 256 rows — two
//! positional reads and one decode of at most 256 rows on a miss
//! (`docs/06-limits.md` L-SC-1), a lock and a 56-byte copy on a hit. A page
//! ([`BarFile::read_range`]): one read of the adjacent directory entries and
//! one of the adjacent frames. None of these grows with the month.

use std::collections::HashMap;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use super::{
    Action, Admission, BAR_TABLE, BarFile, CACHED_BLOCK_LEN, NO_BLOCK, StoreError, VerifiedBlock,
    fault, open_read, refused,
};
use crate::checksum_audit::AuditedBarFile;
use crate::format::{Bar, FormatError, RECORD_LEN};
use crate::layout::Layout;
use crate::path::{FileKind, StorePath};
use crate::sealed::{Audited, Head, MAX_RANGE_ROWS, SealedError, SealedMonth};

/// Which form a month was opened in: the raw, appendable `.bin` with its
/// sidecars, or the sealed `.sbar` (D-4965). The form is not the content: the
/// generation pointer (D-3350) still says which content is served.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Form {
    /// `<yyyy-mm>.bin`, `.crc`, `.prv`, `.lock` and, at 1,024 bars or more,
    /// `.tix` and `.srt`.
    Raw,
    /// `<yyyy-mm>.sbar`, one whole file written once.
    Sealed,
}

impl Form {
    /// A stable word for a page, a log line or a JSON field.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Sealed => "sealed",
        }
    }
}

/// A sealed month's refusal as the store's own error, naming the file.
pub(super) fn refusal(path: &Path, why: &SealedError) -> StoreError {
    StoreError::SealedRefused {
        path: path.to_path_buf(),
        reason: why.reason(),
        detail: why.to_string(),
    }
}

/// Whether the raw door's refusal may be a retirement that ran between the
/// first and the second open: the `.bin` itself is absent, or its shared lock
/// was refused. Only then is `.sbar` probed once more.
pub(super) fn may_be_retired(refusal: &StoreError, root: &Path, path: StorePath<'_>) -> bool {
    match refusal {
        StoreError::Missing { path: missing, .. } => *missing == path.to_path_buf(root),
        StoreError::Locked { .. } => true,
        _ => false,
    }
}

impl BarFile {
    /// Opens `path`'s month under `root` in its SEALED form, or `None` when
    /// there is no `.sbar`. No lock, no sidecar, no generation resolved: the
    /// caller hands the served generation's root.
    ///
    /// A FINAL LINK IS FOLLOWED, exactly as the raw form's reader open
    /// follows one (`open_read` asks no `O_NOFOLLOW`); a link that points
    /// nowhere is an absent `.sbar`. Only the WRITER door refuses links
    /// (CE-62, D-2686), and a sealed month is never written through this
    /// handle. [`crate::generation::served_bars_path`] keys the month by the
    /// same followed file.
    ///
    /// # Errors
    ///
    /// The open's own refusal naming the `.sbar` (denied, or not a file the
    /// head can be read from), and [`StoreError::SealedRefused`] for a head
    /// that fails its checks or was written for another month.
    pub(super) fn open_sealed_at(
        root: &Path,
        path: StorePath<'_>,
        symbol_id: u32,
    ) -> Result<Option<Self>, StoreError> {
        let at = path.with_file(FileKind::SealedBars).to_path_buf(root);
        let file = match open_read(&at) {
            Ok(file) => file,
            Err(why) if why.is_absent() => return Ok(None),
            Err(why) => return Err(why.refusal(&at)),
        };
        let source = fault(file.try_clone(), &at, Action::Open)?;
        let month = SealedMonth::open(source, path, symbol_id).map_err(|why| refusal(&at, &why))?;
        let head = month.head();
        // THE CACHE HOLDS ONE FRAME. The head's `block_rows` is read, never
        // assumed (design §4.2.1); a geometry wider than the handle's buffer
        // is refused by name rather than served in part.
        if usize::from(head.block_rows).saturating_mul(RECORD_LEN) > CACHED_BLOCK_LEN {
            return Err(StoreError::SealedRefused {
                path: at,
                reason: "block_rows",
                detail: format!(
                    "the head declares {} rows a frame and a handle caches at most {}",
                    head.block_rows,
                    CACHED_BLOCK_LEN / RECORD_LEN
                ),
            });
        }
        let header = head.bar_header();
        let layout: Layout = refused(Layout::resolve(BAR_TABLE, header.format_version), &at)?;
        Ok(Some(Self {
            bars: file,
            bars_path: at,
            layout,
            header,
            // NO SLOTS TO AUDIT (D-6243): the sealed form has one head under its
            // own checksum, refused by name when damaged, and no older slot a
            // read could fall back to.
            slot_audit: None,
            checksums: None,
            crc_path: None,
            verified: Mutex::new(VerifiedBlock::EMPTY),
            admission: Admission::new(path.month(), header.timeframe_secs),
            tix: None,
            batch: None,
            srt: None,
            msum: None,
            sealed: Some(month),
            _lock: None,
        }))
    }

    /// Which form this handle reads.
    #[must_use]
    pub const fn form(&self) -> Form {
        if self.sealed.is_some() {
            Form::Sealed
        } else {
            Form::Raw
        }
    }

    /// The sealed head, for a month opened in its sealed form: its counts,
    /// stamps, six column ranges and six extremes, answered with no read
    /// beyond the open. `None` for a raw month.
    #[must_use]
    pub fn sealed_head(&self) -> Option<&Head> {
        self.sealed.as_ref().map(SealedMonth::head)
    }

    /// Copies record `index`, at layout offset `at`, out of the cached frame,
    /// decoding the frame first on a miss. The sealed arm of the read funnel:
    /// the same cache, keyed the same way, serving the same 56-byte images.
    pub(super) fn read_sealed(
        &self,
        sealed: &SealedMonth<File>,
        index: u64,
        at: u64,
        image: &mut [u8],
    ) -> Result<(), StoreError> {
        let n_valid = self.header.n_valid;
        let rows = u64::from(sealed.head().block_rows).max(1);
        let block = index / rows;
        let mut cache = self.verified.lock().unwrap_or_else(PoisonError::into_inner);
        if cache.block != block || cache.n_valid != n_valid {
            // FORGET FIRST, so a refusal or a panic below leaves nothing cached.
            cache.block = NO_BLOCK;
            self.fill_sealed(sealed, block, rows, &mut cache)?;
            cache.block = block;
            cache.n_valid = n_valid;
        }
        let found = cache
            .record(at, image.len())
            .ok_or(StoreError::NotCommitted { index, n_valid })?;
        image.copy_from_slice(found);
        Ok(())
    }

    /// Decodes frame `block` into the cache as record images.
    fn fill_sealed(
        &self,
        sealed: &SealedMonth<File>,
        block: u64,
        rows: u64,
        cache: &mut VerifiedBlock,
    ) -> Result<(), StoreError> {
        let mut decoded = Vec::with_capacity(usize::try_from(rows).unwrap_or(0));
        sealed
            .read_block(block, &mut decoded)
            .map_err(|why| refusal(&self.bars_path, &why))?;
        let start = refused(
            self.layout.offset_of(block.saturating_mul(rows)),
            &self.bars_path,
        )?;
        let mut len = 0usize;
        let mut slots = cache.bytes.chunks_exact_mut(RECORD_LEN);
        for bar in &decoded {
            let slot = slots.next().ok_or_else(|| StoreError::Format {
                path: self.bars_path.clone(),
                source: FormatError::OffsetOverflow,
            })?;
            slot.copy_from_slice(&bar.image());
            len = len.saturating_add(RECORD_LEN);
        }
        cache.start = start;
        cache.len = len;
        Ok(())
    }

    /// Records `first .. first + count`, in order: THE PAGE PATH (design
    /// §4.4). A sealed month reads the adjacent directory entries and the
    /// adjacent frames once each, in chunks of at most
    /// [`MAX_RANGE_ROWS`]; a raw month reads each record through the
    /// verified cache. Either way the rows are the month's committed records,
    /// bit for bit.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotCommitted`] when the range runs past the commit
    /// counter; [`StoreError::SealedRefused`] naming the block for a sealed
    /// frame that fails its check; any record read's own refusal. Nothing is
    /// returned in part.
    pub fn read_range(&self, first: u64, count: u64) -> Result<Vec<Bar>, StoreError> {
        let n_valid = self.header.n_valid;
        let end = first
            .checked_add(count)
            .filter(|&end| end <= n_valid)
            .ok_or(StoreError::NotCommitted {
                index: first.max(n_valid),
                n_valid,
            })?;
        let mut out = Vec::with_capacity(usize::try_from(count).unwrap_or(0).min(1 << 20));
        let Some(sealed) = self.sealed.as_ref() else {
            for index in first..end {
                out.push(self.read_record(index)?);
            }
            return Ok(out);
        };
        let mut at = first;
        while at < end {
            let take = (end - at).min(MAX_RANGE_ROWS);
            sealed
                .read_rows(at, take, &mut out)
                .map_err(|why| refusal(&self.bars_path, &why))?;
            at += take;
        }
        Ok(out)
    }

    /// The records at `rows`, in the order asked: the SORTED PAGE PATH
    /// (design §4.4). A sealed month groups the rows by frame and decodes
    /// each frame once, however the rows are scattered (`docs/06-limits.md`
    /// L-SC-2: at most one frame a row); a raw month reads each through the
    /// verified cache.
    ///
    /// GROUPED WITHOUT A SORT (BASE-GATE11, D-6623): one pass over the rows
    /// refuses an uncommitted one and gives each row its frame's group, by one
    /// probe of a map pre-sized for the rows; a count of each group and one
    /// placement pass lay the rows out frame by frame. Every step is O(1) a
    /// row, expected, where the sort it replaces was O(log r) a row; frames
    /// are decoded in the order the page first meets them, which changes no
    /// answer, since every bar goes to the place it was asked at. Measured by
    /// `store::file::sealed_door_tests::read_rows_at_timing` (L-SC-2); the
    /// order and the refusals by
    /// `store::file::sealed_door_tests::a_scattered_page_is_answered_in_the_asked_order_by_both_forms`.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotCommitted`] for the first row, in the order asked, at
    /// or past the commit counter, before anything is read; a frame or record
    /// read's own refusal.
    pub fn read_rows_at(&self, rows: &[u64]) -> Result<Vec<Bar>, StoreError> {
        let n_valid = self.header.n_valid;
        let uncommitted = |index: u64| StoreError::NotCommitted { index, n_valid };
        let Some(sealed) = self.sealed.as_ref() else {
            for &row in rows {
                if row >= n_valid {
                    return Err(uncommitted(row));
                }
            }
            return rows.iter().map(|&row| self.read_record(row)).collect();
        };
        let per = u64::from(sealed.head().block_rows).max(1);
        // THE GROUPS, in the order the rows first meet their frames.
        let mut group_of: HashMap<u64, usize> = HashMap::with_capacity(rows.len());
        let mut blocks: Vec<u64> = Vec::with_capacity(rows.len());
        let mut group: Vec<usize> = Vec::with_capacity(rows.len());
        for &row in rows {
            if row >= n_valid {
                return Err(uncommitted(row));
            }
            let block = row / per;
            let next = blocks.len();
            let at = *group_of.entry(block).or_insert(next);
            if at == next {
                blocks.push(block);
            }
            group.push(at);
        }
        // THE PLACEMENT: `first[g]..first[g + 1]` of `placed` are the asked
        // positions of group `g`'s rows.
        let mut first = vec![0_usize; blocks.len() + 1];
        for &at in &group {
            if let Some(count) = first.get_mut(at + 1) {
                *count += 1;
            }
        }
        for at in 1..first.len() {
            let before = first.get(at - 1).copied().unwrap_or(0);
            if let Some(start) = first.get_mut(at) {
                *start += before;
            }
        }
        let mut next_slot = first.clone();
        let mut placed = vec![0_usize; rows.len()];
        for (asked, &at) in group.iter().enumerate() {
            if let Some(slot) = next_slot.get_mut(at) {
                if let Some(place) = placed.get_mut(*slot) {
                    *place = asked;
                }
                *slot += 1;
            }
        }
        let mut out = vec![Bar::default(); rows.len()];
        let mut held = Vec::with_capacity(usize::from(sealed.head().block_rows));
        for (at, &block) in blocks.iter().enumerate() {
            held.clear();
            sealed
                .read_block(block, &mut held)
                .map_err(|why| refusal(&self.bars_path, &why))?;
            let from = first.get(at).copied().unwrap_or(0);
            let to = first.get(at + 1).copied().unwrap_or(from);
            for &asked in placed.get(from..to).unwrap_or_default() {
                let Some(&row) = rows.get(asked) else {
                    continue;
                };
                let found = usize::try_from(row - block * per)
                    .ok()
                    .and_then(|offset| held.get(offset))
                    .copied()
                    .ok_or_else(|| uncommitted(row))?;
                if let Some(slot) = out.get_mut(asked) {
                    *slot = found;
                }
            }
        }
        Ok(out)
    }
}

/// What the audited door learned of a sealed month before it audited the raw
/// form beside it.
pub(super) struct SealedAudit {
    path: PathBuf,
    header: crate::header::Header,
    records_blake3: [u8; 32],
    audited: Audited,
}

/// The audited door's sealed half (design §4.4: "every directory entry, every
/// frame CRC, full decode against the head's BLAKE3"): `None` when the served
/// generation holds no `.sbar`.
///
/// # Errors
///
/// The sealed month's refusal, named; and a sealed month whose raw form is not
/// beside it, because a strict checksum receipt covers a raw month's `.bin`
/// and `.crc` and this month then has none to give. That is said, never
/// answered with a receipt for bytes nobody audited in that form.
pub(super) fn audit_sealed(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
) -> Result<Option<SealedAudit>, String> {
    let Some(file) =
        BarFile::open_sealed_at(root, path, symbol_id).map_err(|why| why.to_string())?
    else {
        return Ok(None);
    };
    let Some(sealed) = file.sealed.as_ref() else {
        return Ok(None);
    };
    let audited = sealed
        .audit()
        .map_err(|why| refusal(&file.bars_path, &why).to_string())?;
    let raw = path.to_path_buf(root);
    match std::fs::symlink_metadata(&raw) {
        Err(why) if why.kind() == ErrorKind::NotFound => Err(format!(
            "{}: the sealed month passed its full audit ({} rows in {} frames, {} bytes) and its \
             raw form {} is not beside it; a strict checksum receipt covers a raw month's .bin \
             and .crc, so this month has none (retiring the raw form is SC-9's, design §4.7)",
            file.bars_path.display(),
            audited.rows,
            audited.frames,
            audited.bytes,
            raw.display()
        )),
        _ => Ok(Some(SealedAudit {
            path: file.bars_path.clone(),
            header: file.header,
            records_blake3: sealed.head().records_blake3,
            audited,
        })),
    }
}

/// The two forms of one month, both audited, compared: the same committed
/// header facts and the same BLAKE3 over the same record bytes.
///
/// # Errors
///
/// Any difference, naming both files and what differs.
pub(super) fn agree(sealed: Option<&SealedAudit>, raw: &AuditedBarFile) -> Result<(), String> {
    let Some(sealed) = sealed else {
        return Ok(());
    };
    let evidence = raw.evidence();
    let header = evidence.header();
    let same = header.n_valid == sealed.header.n_valid
        && header.generation == sealed.header.generation
        && header.first_ts_micros == sealed.header.first_ts_micros
        && header.last_ts_micros == sealed.header.last_ts_micros
        && header.symbol_id == sealed.header.symbol_id
        && header.timeframe_secs == sealed.header.timeframe_secs
        && evidence.data_digest() == sealed.records_blake3
        && sealed.audited.rows == header.n_valid;
    if same {
        Ok(())
    } else {
        Err(format!(
            "{} and {} are two forms of one month and they disagree (rows {} against {}, \
             generation {} against {}, records BLAKE3 equal: {}); neither is audited",
            sealed.path.display(),
            raw.path().display(),
            sealed.header.n_valid,
            header.n_valid,
            sealed.header.generation,
            header.generation,
            evidence.data_digest() == sealed.records_blake3
        ))
    }
}

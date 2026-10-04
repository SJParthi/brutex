//! Selection V6: an opaque, source-retaining successor of Execution V4.
//!
//! The shared Runner ranking consumes every authenticated disposition. Empty
//! naturally extinct families remain in the source envelope. One fixed 16-KiB
//! record stores the source proof and actual Top-25; unused slots are zero.
//! Its last 32 bytes are a completion seal, appended and synced only after the
//! payload is synced. Top-10 is the actual prefix, never a separate ranking.
//! V5 files and detached caller-authored facts are never accepted as authority.
//!
//! Source reauthentication/ranking is O(candidates + upstream file bytes).
//! File integrity checks are O(history), bounded by explicit physical limits.
//! Retained history grows; no constant latency or total-space claim is made.

use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::execution_v4::CommittedStoredExecutionV4;
use runner::topn::{RankedCandidate, RankingPolicyV1};

#[path = "selection_v6_source.rs"]
mod source;
use source::{Prepared, Winner};

pub(crate) const SELECTION_V6_BLOCK_BYTES: usize = 16_384;
const BLOCK_BYTES: u64 = 16_384;
const SEAL_AT: usize = SELECTION_V6_BLOCK_BYTES - 32;
const MAGIC: &[u8; 16] = b"BTX-SELV6-BLOCK\0";
const FILE_NAME: &str = "global-selection-v6.bin";
type Block = [u8; SELECTION_V6_BLOCK_BYTES];

/// Physical limits, with no implicit or numeric admission policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectionV6Bounds {
    records: u64,
    bytes: u64,
}

impl SelectionV6Bounds {
    pub(crate) fn new(records: u64, bytes: u64) -> Result<Self, String> {
        if records == 0
            || records
                .checked_mul(BLOCK_BYTES)
                .is_none_or(|need| need > bytes)
        {
            return Err("Selection V6 bounds must hold every declared fixed record".to_owned());
        }
        Ok(Self { records, bytes })
    }
}

/// A ranked row with the exact durable disposition and selected exit identity.
/// The constructor is private; this does not grant chronological replay authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectionV6Winner {
    pub(crate) rank: u32,
    pub(crate) family: &'static str,
    pub(crate) ranked: RankedCandidate,
    pub(crate) disposition_id: [u8; 32],
    pub(crate) selected_exit_digest: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectionV6Snapshot {
    pub(crate) rung_seconds: u32,
    pub(crate) identity: [u8; 32],
    /// Exact header, family envelopes and population-ranking proof.
    pub(crate) envelope: [u8; 960],
    pub(crate) winners: Vec<SelectionV6Winner>,
}

/// A live authority retains its upstream capability and revalidates each read's
/// physical generation before returning reconstructed winners.
pub(crate) struct CommittedStoredSelectionV6 {
    source: CommittedStoredExecutionV4,
    policy: RankingPolicyV1,
    root: PathBuf,
    bounds: SelectionV6Bounds,
    identity: [u8; 32],
    written: bool,
}

impl CommittedStoredSelectionV6 {
    pub(crate) fn snapshot(&mut self) -> Result<SelectionV6Snapshot, String> {
        let winners = self.top_twenty_five()?;
        let prepared = Prepared::from_execution(&mut self.source, self.policy)?;
        let block = prepared.block()?;
        if prepared.identity()? != self.identity {
            return Err("Selection V6 snapshot changed identity".to_owned());
        }
        require_committed(&self.root, self.bounds, &block)?;
        let envelope = block
            .get(..960)
            .ok_or("Selection V6 envelope bounds")?
            .try_into()
            .map_err(|_| "Selection V6 envelope width")?;
        Ok(SelectionV6Snapshot {
            rung_seconds: self.source.structural_receipt().rung(),
            identity: self.identity,
            envelope,
            winners,
        })
    }

    pub(crate) fn stored_oos_witnesses(
        &mut self,
        expected: &SelectionV6Snapshot,
        request: crate::stored_post_training_oos::StoredPostTrainingOosRequestV1,
        max_candidates: u64,
        observer: &mut crate::stored_post_training_oos::StoredOosObserverV1<'_>,
    ) -> Result<Vec<crate::stored_post_training_oos::StoredPostTrainingOosWitnessV1>, String> {
        if &self.snapshot()? != expected {
            return Err("Selection V6 changed before actual winner replay".to_owned());
        }
        let strategies: Vec<_> = expected
            .winners
            .iter()
            .map(|winner| winner.ranked.candidate.strategy_digest.bytes())
            .collect();
        let witnesses = self.source.selected_stored_oos_witnesses(
            request,
            &strategies,
            max_candidates,
            observer,
        )?;
        if &self.snapshot()? != expected || witnesses.len() != strategies.len() {
            return Err("Selection V6 changed during actual winner replay".to_owned());
        }
        Ok(witnesses)
    }

    pub(crate) const fn was_written(&self) -> bool {
        self.written
    }
    pub(crate) const fn identity(&self) -> [u8; 32] {
        self.identity
    }

    /// Reproduces source and ranking before and after a freshly opened record.
    pub(crate) fn top_twenty_five(&mut self) -> Result<Vec<SelectionV6Winner>, String> {
        let before = Prepared::from_execution(&mut self.source, self.policy)?;
        if before.identity()? != self.identity {
            return Err("Selection V6 retained source now identifies another selection".to_owned());
        }
        require_committed(&self.root, self.bounds, &before.block()?)?;
        let after = Prepared::from_execution(&mut self.source, self.policy)?;
        if after != before {
            return Err("Selection V6 source changed during authoritative winner read".to_owned());
        }
        before
            .winners
            .iter()
            .enumerate()
            .map(|(rank, winner)| public_winner(rank, winner))
            .collect()
    }

    /// Exact actual prefix of the same authenticated Top-25.
    pub(crate) fn top_ten(&mut self) -> Result<Vec<SelectionV6Winner>, String> {
        Ok(self.top_twenty_five()?.into_iter().take(10).collect())
    }
}

fn public_winner(rank: usize, winner: &Winner) -> Result<SelectionV6Winner, String> {
    Ok(SelectionV6Winner {
        rank: u32::try_from(rank).map_err(|why| why.to_string())?,
        family: if winner.family == 1 {
            "NIFTY"
        } else {
            "BANKNIFTY"
        },
        ranked: winner.ranked,
        disposition_id: winner.disposition_id,
        selected_exit_digest: winner.selected_exit_digest,
    })
}

/// Sole production commit door: no detached source or structural receipt overload.
pub(crate) fn commit_stored_selection_v6(
    root: &Path,
    bounds: SelectionV6Bounds,
    mut source: CommittedStoredExecutionV4,
    policy: RankingPolicyV1,
) -> Result<CommittedStoredSelectionV6, String> {
    let before = Prepared::from_execution(&mut source, policy)?;
    let block = before.block()?;
    let root = checked_root(root)?;
    let written = persist(&root, bounds, &block)?;
    require_committed(&root, bounds, &block)?;
    if Prepared::from_execution(&mut source, policy)? != before {
        return Err(
            "Selection V6 source changed during persistence; no authority returned".to_owned(),
        );
    }
    Ok(CommittedStoredSelectionV6 {
        source,
        policy,
        root,
        bounds,
        identity: before.identity()?,
        written,
    })
}

fn checked_root(root: &Path) -> Result<PathBuf, String> {
    let metadata = std::fs::symlink_metadata(root).map_err(|why| why.to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("Selection V6 requires an existing nonsymlink directory".to_owned());
    }
    std::fs::canonicalize(root).map_err(|why| why.to_string())
}

fn open(root: &Path, writable: bool) -> Result<(File, PathBuf), String> {
    let root = checked_root(root)?;
    let path = root.join(FILE_NAME);
    if let Ok(meta) = std::fs::symlink_metadata(&path)
        && (!meta.is_file() || meta.file_type().is_symlink())
    {
        return Err("Selection V6 file must be a regular nonsymlink file".to_owned());
    }
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(writable)
        .create(writable)
        .truncate(false);
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(store::open_flags::O_NOFOLLOW);
    }
    let file = options
        .open(&path)
        .map_err(|why| format!("Selection V6 open: {why}"))?;
    let metadata = file.metadata().map_err(|why| why.to_string())?;
    if !metadata.is_file() {
        return Err("Selection V6 file is not regular".to_owned());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if metadata.nlink() != 1 {
            return Err(format!(
                "Selection V6 refuses aliased hard-linked file {} ({} links); remove \
                 the other hard link to it (CE-40, D-1769)",
                path.display(),
                metadata.nlink()
            ));
        }
    }
    crate::result_set::file_generation(&file, &path)?;
    Ok((file, path))
}

fn scan(
    file: &mut File,
    path: &Path,
    bounds: SelectionV6Bounds,
    expected: &Block,
) -> Result<(u64, bool), String> {
    let before = crate::result_set::file_generation(file, path)?;
    let count = before.len / BLOCK_BYTES;
    if before.len > bounds.bytes || count > bounds.records {
        return Err("Selection V6 file exceeds its explicit bounds".to_owned());
    }
    let mut identities = HashSet::new();
    identities
        .try_reserve(usize::try_from(count).map_err(|why| why.to_string())?)
        .map_err(|why| format!("Selection V6 identity reserve: {why}"))?;
    let mut found = false;
    file.seek(SeekFrom::Start(0))
        .map_err(|why| why.to_string())?;
    for _ in 0..count {
        let mut block = [0; SELECTION_V6_BLOCK_BYTES];
        file.read_exact(&mut block).map_err(|why| why.to_string())?;
        verify_block(&block)?;
        let identity = block.get(24..56).ok_or("Selection V6 identity slot")?;
        let key: [u8; 32] = identity
            .try_into()
            .map_err(|_| "Selection V6 identity width")?;
        if !identities.insert(key) {
            return Err("Selection V6 contains duplicate committed identities".to_owned());
        }
        if identity
            == expected
                .get(24..56)
                .ok_or("Selection V6 expected identity slot")?
        {
            if block != *expected {
                return Err("Selection V6 committed source bytes differ".to_owned());
            }
            found = true;
        }
    }
    crate::result_set::require_generation_unchanged(
        before,
        crate::result_set::file_generation(file, path)?,
        path,
    )?;
    Ok((before.len, found))
}

fn persist(root: &Path, bounds: SelectionV6Bounds, expected: &Block) -> Result<bool, String> {
    verify_block(expected)?;
    let (mut file, path) = open(root, true)?;
    file.lock().map_err(|why| why.to_string())?;
    let result = (|| {
        let (len, found) = scan(&mut file, &path, bounds, expected)?;
        let first = len - len % BLOCK_BYTES;
        if found {
            if len % BLOCK_BYTES != 0 {
                set_aside_abandoned_tail(&mut file, root, first, len)?;
            }
            // A path whose barrier failed in this process is never confirmed
            // by a second one (ledgers-2, D-2550).
            crate::fixed_tail::refuse_after_failed_barrier(&path)?;
            file.sync_all().map_err(|why| why.to_string())?;
            sync_directory(root)?;
            return Ok(false);
        }
        if first / BLOCK_BYTES >= bounds.records
            || first
                .checked_add(BLOCK_BYTES)
                .is_none_or(|end| end > bounds.bytes)
        {
            return Err("Selection V6 has no space inside declared record bounds".to_owned());
        }
        let partial = usize::try_from(len % BLOCK_BYTES).map_err(|why| why.to_string())?;
        if partial != 0 {
            let mut prefix = [0; SELECTION_V6_BLOCK_BYTES];
            file.seek(SeekFrom::Start(first))
                .map_err(|why| why.to_string())?;
            let prefix = prefix
                .get_mut(..partial)
                .ok_or("Selection V6 prefix bound")?;
            file.read_exact(prefix).map_err(|why| why.to_string())?;
            if Some(&*prefix) == expected.get(..partial) {
                // THIS EXACT RETRY'S OWN UNSEALED PREFIX is rewritten, never
                // vouched for (ledgers-2, D-2550). It may be the bytes of a
                // run whose barrier failed; a barrier on this descriptor
                // cannot prove them durable, so they are cut and the whole
                // block is written again. The bytes are identical.
                file.set_len(first)
                    .and_then(|()| file.sync_all())
                    .map_err(|why| format!("Selection V6 retry prefix cut: {why}"))?;
            } else {
                // ANOTHER SOURCE'S UNSEALED TAIL. It was never authority, and
                // leaving it in place wedged the rung for every other source.
                // It is moved aside whole, under this exclusive lock, before
                // anything is appended (D-1569).
                set_aside_abandoned_tail(&mut file, root, first, len)?;
            }
        }
        // Committed blocks are never touched. Payload and seal each pass a
        // barrier; any write or barrier failure cuts the block back to
        // `first` and a failed barrier is remembered (D-1900, D-2550).
        let payload = expected.get(..SEAL_AT).ok_or("Selection V6 payload bound")?;
        let seal = expected.get(SEAL_AT..).ok_or("Selection V6 seal bound")?;
        for part in [payload, seal] {
            crate::fixed_tail::write_at_end(
                &mut file,
                &path.display(),
                first,
                part,
                std::io::Write::write_all,
            )?;
            crate::fixed_tail::sync_all_or_roll_back(&file, &path, first)?;
        }
        sync_directory(root)?;
        Ok(true)
    })();
    let released = file.unlock().map_err(|why| why.to_string());
    result.and_then(|value| released.map(|()| value))
}

/// Moves an interrupted writer's unsealed tail, `committed..len`, into
/// `<file>.abandoned-<committed>-<blake3 of the tail>` and cuts it from the
/// ledger. audit-20261003 hunt-cli-a-5, D-1569; conc4-1, D-2554.
///
/// The bytes are copied and synced, with their directory, BEFORE the ledger
/// is shortened, so a crash between the two leaves the tail in both places
/// rather than in neither. Committed sealed blocks, `..committed`, are never
/// rewritten. The copy is written under a scratch name, synced, and only then
/// renamed to a name keyed by the tail's CONTENT as well as its offset, so a
/// torn or failed copy never sits under a final name, and a second abandoned
/// tail at the same offset gets its own quarantine rather than wedging the
/// rung against the first. A failed copy removes its scratch file. The move is
/// named in the log, so the repair is never silent.
fn set_aside_abandoned_tail(
    file: &mut File,
    root: &Path,
    committed: u64,
    len: u64,
) -> Result<(), String> {
    let tail_len = usize::try_from(len - committed).map_err(|why| why.to_string())?;
    let mut tail = vec![0; tail_len];
    file.seek(SeekFrom::Start(committed))
        .map_err(|why| why.to_string())?;
    file.read_exact(&mut tail).map_err(|why| why.to_string())?;
    let digest = brutex_core::blake3::hash(&tail);
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    let aside = root.join(format!("{FILE_NAME}.abandoned-{committed}-{hex}"));
    let scratch = root.join(format!("{FILE_NAME}.abandoned-{committed}-{hex}.writing"));
    let copied = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&scratch)
        .and_then(|mut out| {
            crate::fixed_tail::write_at_end(&mut out, &scratch.display(), 0, &tail, Write::write_all)
                .map_err(std::io::Error::other)?;
            crate::fixed_tail::sync_all_hooked(&out, &scratch)
        })
        .and_then(|()| std::fs::rename(&scratch, &aside));
    if let Err(why) = copied {
        let removed = match std::fs::remove_file(&scratch) {
            Err(gone) if gone.kind() != std::io::ErrorKind::NotFound => {
                format!("; removing scratch {} also failed: {gone}", scratch.display())
            }
            _ => String::new(),
        };
        return Err(format!(
            "Selection V6 abandoned tail quarantine {}: {why}{removed}; the ledger was not changed",
            aside.display()
        ));
    }
    sync_directory(root)?;
    file.set_len(committed).map_err(|why| why.to_string())?;
    file.sync_all().map_err(|why| why.to_string())?;
    crate::note(
        &telemetry::Event::warn("cli.selection_v6", "abandoned unsealed tail set aside")
            .with("committed_bytes", committed)
            .with("tail_bytes", len - committed)
            .with("quarantine", aside.display().to_string().as_str()),
    );
    Ok(())
}

fn sync_directory(root: &Path) -> Result<(), String> {
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|why| why.to_string())
}

fn require_committed(
    root: &Path,
    bounds: SelectionV6Bounds,
    expected: &Block,
) -> Result<(), String> {
    let (mut file, path) = open(root, false)?;
    file.lock_shared().map_err(|why| why.to_string())?;
    // An unsealed trailing partial block is never authority, and it no longer
    // hides the committed blocks before it (D-1569).
    let checked = scan(&mut file, &path, bounds, expected).and_then(|(_, found)| {
        if found {
            Ok(())
        } else {
            Err("Selection V6 lacks the requested exact committed block".to_owned())
        }
    });
    let released = file.unlock().map_err(|why| why.to_string());
    checked.and(released)
}

fn block_identity(block: &Block) -> Result<[u8; 32], String> {
    let mut hash = brutex_core::blake3::Hasher::new();
    hash.update(b"brutex-selection-v6-semantic-block\0");
    hash.update(block.get(..24).ok_or("Selection V6 header bounds")?);
    hash.update(
        block
            .get(56..SEAL_AT)
            .ok_or("Selection V6 payload bounds")?,
    );
    Ok(hash.finalize())
}

fn seal(block: &Block) -> Result<[u8; 32], String> {
    let mut hash = brutex_core::blake3::Hasher::new();
    hash.update(b"brutex-selection-v6-completion-seal\0");
    hash.update(block.get(..SEAL_AT).ok_or("Selection V6 seal range")?);
    Ok(hash.finalize())
}

fn verify_block(block: &Block) -> Result<(), String> {
    if block.get(..16) != Some(MAGIC.as_slice())
        || block.get(16..20) != Some(6_u32.to_le_bytes().as_slice())
        || block.get(20..24) != Some([0; 4].as_slice())
        || block.get(24..56) != Some(block_identity(block)?.as_slice())
        || block.get(SEAL_AT..) != Some(seal(block)?.as_slice())
    {
        return Err("Selection V6 block has wrong version, identity or completion seal".to_owned());
    }
    Ok(())
}

struct Encoder<'a> {
    bytes: &'a mut [u8],
    at: usize,
}

#[cfg(test)]
#[path = "selection_v6_tests.rs"]
mod tests;
impl Encoder<'_> {
    fn bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        let end = self
            .at
            .checked_add(bytes.len())
            .ok_or("Selection V6 encoding overflow")?;
        self.bytes
            .get_mut(self.at..end)
            .ok_or("Selection V6 fixed payload capacity")?
            .copy_from_slice(bytes);
        self.at = end;
        Ok(())
    }
    fn number(&mut self, value: u64) -> Result<(), String> {
        self.bytes(&value.to_le_bytes())
    }
}

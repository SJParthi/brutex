//! Independent bounded full checksum audit for opt-in historical admission.
//! The ordinary reader and all old store formats remain unchanged. Cold audit
//! is O(file bytes); a warm read verifies one fixed-size block and generations.
use std::fs::{self, File};
use std::os::unix::fs::{FileExt as _, MetadataExt as _};
use std::path::{Path, PathBuf};

use crate::file::BarFile;
use crate::format::{Bar, Row as _};
use crate::header::Header;
use crate::layout::Layout;
use brutex_core::blake3::Hasher;

/// Fixed canonical image carried by a historical checksum receipt.
pub const EVIDENCE_BYTES: usize = 256;
const BUFFER_BYTES: usize = 8192;
const HEADER_BYTES: usize = 32768;

pub(crate) struct Inputs<'a> {
    pub data: &'a File,
    pub data_path: &'a Path,
    pub sidecar: &'a File,
    pub sidecar_path: &'a Path,
    pub lock: &'a File,
    pub lock_path: PathBuf,
}

/// Strict opt-in opener. Unsupported hosts refuse instead of using a blocking open.
pub(crate) fn open_regular(path: &Path) -> std::io::Result<File> {
    #[cfg(any(
        target_os = "macos",
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        #[cfg(target_os = "macos")]
        let flags = 0x100 | 0x4;
        #[cfg(target_os = "linux")]
        let flags = 0x20_000 | 0x800;
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(flags)
            .open(path)?;
        Generation::read(&file, path).map_err(std::io::Error::other)?;
        Ok(file)
    }
    #[cfg(not(any(
        target_os = "macos",
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "strict checksum reads require verified macOS or Linux x86_64/aarch64 open flags",
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Generation {
    device: u64,
    inode: u64,
    len: u64,
    modified: (i64, i64),
    changed: (i64, i64),
    links: u64,
}
impl Generation {
    fn of(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            len: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
            links: metadata.nlink(),
        }
    }
    fn read(file: &File, path: &Path) -> Result<Self, String> {
        let held = file.metadata().map_err(error)?;
        let named = fs::symlink_metadata(path).map_err(error)?;
        if !held.is_file()
            || !named.is_file()
            || held.nlink() != 1
            || Self::of(&held) != Self::of(&named)
        {
            return Err(format!(
                "checksum audit refuses an alias, non-regular file or replacement: {}",
                path.display()
            ));
        }
        Ok(Self::of(&held))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Generations {
    data: Generation,
    sidecar: Generation,
    lock: Generation,
}
impl Generations {
    fn read(input: &Inputs<'_>) -> Result<Self, String> {
        Ok(Self {
            data: Generation::read(input.data, input.data_path)?,
            sidecar: Generation::read(input.sidecar, input.sidecar_path)?,
            lock: Generation::read(input.lock, &input.lock_path)?,
        })
    }
}

/// Detached audit facts. This projection alone cannot construct a verified reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Evidence {
    header: Header,
    header_bytes: u64,
    data_bytes: u64,
    sidecar_bytes: u64,
    blocks: u64,
    header_digest: [u8; 32],
    data_digest: [u8; 32],
    sidecar_digest: [u8; 32],
    file_digest: [u8; 32],
}
impl Evidence {
    /// Exact committed header admitted by the audit.
    #[must_use]
    pub const fn header(self) -> Header {
        self.header
    }
    /// Number of independently verified committed blocks.
    #[must_use]
    pub const fn blocks(self) -> u64 {
        self.blocks
    }
    /// Full raw header-region digest, including redundant slots and padding.
    #[must_use]
    pub const fn header_digest(self) -> [u8; 32] {
        self.header_digest
    }
    /// Digest of all committed raw record bytes.
    #[must_use]
    pub const fn data_digest(self) -> [u8; 32] {
        self.data_digest
    }
    /// Digest of the entire exact checksum sidecar.
    #[must_use]
    pub const fn sidecar_digest(self) -> [u8; 32] {
        self.sidecar_digest
    }
    /// Digest of the entire exact data file, including its header region.
    #[must_use]
    pub const fn file_digest(self) -> [u8; 32] {
        self.file_digest
    }
    /// Stable V1 encoding. All numeric fields are little-endian u64 words;
    /// timestamp words retain their exact signed two's-complement bits.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; EVIDENCE_BYTES] {
        let mut out = [0; EVIDENCE_BYTES];
        let words = [
            u64::from(self.header.format_version),
            u64::from(self.header.record_stride),
            u64::from(self.header.flags),
            self.header.generation,
            self.header.n_valid,
            u64::from_le_bytes(self.header.first_ts_micros.to_le_bytes()),
            u64::from_le_bytes(self.header.last_ts_micros.to_le_bytes()),
            u64::from(self.header.symbol_id),
            u64::from(self.header.timeframe_secs),
            self.header_bytes,
            self.data_bytes,
            self.sidecar_bytes,
            self.blocks,
        ];
        for (slot, byte) in out.iter_mut().zip(
            b"BRCAS001"
                .iter()
                .copied()
                .chain(words.into_iter().flat_map(u64::to_le_bytes))
                .chain(self.header_digest)
                .chain(self.data_digest)
                .chain(self.sidecar_digest)
                .chain(self.file_digest),
        ) {
            *slot = byte;
        }
        out
    }
}

/// Owns the exact month/sidecar/lock handles whose complete bytes were audited.
/// No detached digest or caller-authored receipt can mint this authority.
pub struct AuditedBarFile {
    file: BarFile,
    evidence: Evidence,
    generation: Generations,
}
impl AuditedBarFile {
    pub(crate) fn from_file(file: BarFile, max_bytes: u64) -> Result<Self, String> {
        let input = file.checksum_inputs()?;
        let generation = Generations::read(&input)?;
        let evidence = audit(&input, file.header(), file.layout(), generation, max_bytes)?;
        let result = Self {
            file,
            evidence,
            generation,
        };
        result.require_current()?;
        Ok(result)
    }
    /// Exact independently measured facts, without releasing the held reader.
    #[must_use]
    pub const fn evidence(&self) -> Evidence {
        self.evidence
    }
    /// Original canonical store path; the reader cannot be redirected to another.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.file.path()
    }
    /// Check held/path generations for data, sidecar and month lock.
    ///
    /// # Errors
    /// Refuses deletion, replacement, metadata changes, aliases and I/O errors.
    pub fn require_current(&self) -> Result<(), String> {
        if Generations::read(&self.file.checksum_inputs()?)? != self.generation {
            return Err("checksum-audited month changed after its full audit".to_owned());
        }
        Ok(())
    }
    /// Decode a row from the very block image whose checksum is checked here.
    /// This does not trust the ordinary reader's last-block cache. Work and
    /// memory are bounded by the fixed format block, not the month length.
    ///
    /// # Errors
    /// Refuses an uncommitted index, changed files, checksum errors or bad rows.
    pub fn read_record(&self, index: u64) -> Result<Bar, String> {
        self.require_current()?;
        let header = self.evidence.header;
        if index >= header.n_valid {
            return Err("checksum-audited row is not committed".to_owned());
        }
        let layout = self.file.layout();
        let input = self.file.checksum_inputs()?;
        let block = layout.block_of(index);
        let mut buffer = [0; BUFFER_BYTES];
        let (start, bytes, _) = verified_block(&input, header, layout, block, &mut buffer)?;
        let (row_start, row_end) = layout.record_byte_range(index).map_err(debug)?;
        let first = usize::try_from(
            row_start
                .checked_sub(start)
                .ok_or("audit row start overflow")?,
        )
        .map_err(error)?;
        let last = usize::try_from(row_end.checked_sub(start).ok_or("audit row end overflow")?)
            .map_err(error)?;
        let row = Bar::read_from(
            bytes
                .get(first..last)
                .ok_or("audit row escaped its verified block")?,
        )
        .map_err(debug)?;
        self.require_current()?;
        Ok(row)
    }
}

fn audit(
    input: &Inputs<'_>,
    header: Header,
    layout: Layout,
    before: Generations,
    max_bytes: u64,
) -> Result<Evidence, String> {
    let blocks = layout.blocks_for(header.n_valid);
    let data_bytes = layout.offset_of(header.n_valid).map_err(debug)?;
    let sidecar_bytes = blocks
        .checked_mul(4)
        .ok_or("checksum sidecar extent overflow")?;
    if !header.checksums_present()
        || layout.record_stride() != Bar::LEN as u64
        || layout.header_len() != HEADER_BYTES as u64
        || layout.block_len() > BUFFER_BYTES as u64
        || before.data.len != data_bytes
        || before.sidecar.len != sidecar_bytes
        || data_bytes
            .checked_add(sidecar_bytes)
            .is_none_or(|n| n > max_bytes)
    {
        return Err("strict checksum audit refuses unsealed, unsupported, non-exact or over-limit input extents".to_owned());
    }
    let mut header_image = Vec::new();
    header_image
        .try_reserve_exact(HEADER_BYTES)
        .map_err(error)?;
    header_image.resize(HEADER_BYTES, 0);
    read(input.data, input.data_path, 0, &mut header_image)?;
    if Header::read_region(&header_image, data_bytes).map_err(debug)? != header {
        return Err("held month header changed before checksum audit".to_owned());
    }
    let mut full = Hasher::new();
    full.update(&header_image);
    let mut data = Hasher::new();
    let mut sidecar = Hasher::new();
    let mut buffer = [0; BUFFER_BYTES];
    for block in 0..blocks {
        let (_, bytes, sum) = verified_block(input, header, layout, block, &mut buffer)?;
        data.update(bytes);
        full.update(bytes);
        sidecar.update(&sum);
    }
    if Generations::read(input)? != before {
        return Err("checksum-audit input changed during the complete scan".to_owned());
    }
    Ok(Evidence {
        header,
        header_bytes: layout.header_len(),
        data_bytes,
        sidecar_bytes,
        blocks,
        header_digest: brutex_core::blake3::hash(&header_image),
        data_digest: data.finalize(),
        sidecar_digest: sidecar.finalize(),
        file_digest: full.finalize(),
    })
}

fn verified_block<'a>(
    input: &Inputs<'_>,
    header: Header,
    layout: Layout,
    block: u64,
    buffer: &'a mut [u8; BUFFER_BYTES],
) -> Result<(u64, &'a [u8], [u8; 4]), String> {
    let (start, end) = layout
        .covered_byte_range(block, header.n_valid)
        .map_err(debug)?;
    let count = usize::try_from(
        end.checked_sub(start)
            .ok_or("checksum block extent overflow")?,
    )
    .map_err(error)?;
    let bytes = buffer
        .get_mut(..count)
        .ok_or("checksum block exceeds fixed audit buffer")?;
    read(input.data, input.data_path, start, bytes)?;
    let mut sum = [0; 4];
    read(
        input.sidecar,
        input.sidecar_path,
        block
            .checked_mul(4)
            .ok_or("checksum block offset overflow")?,
        &mut sum,
    )?;
    crate::block::verify(&header, layout, block, bytes, u32::from_le_bytes(sum)).map_err(debug)?;
    Ok((start, bytes, sum))
}
fn read(file: &File, path: &Path, offset: u64, bytes: &mut [u8]) -> Result<(), String> {
    file.read_exact_at(bytes, offset)
        .map_err(|why| format!("checksum audit read {} at {offset}: {why}", path.display()))
}
fn error(why: impl std::fmt::Display) -> String {
    why.to_string()
}
fn debug(why: impl std::fmt::Debug) -> String {
    format!("{why:?}")
}

#[cfg(test)]
#[path = "checksum_audit_tests.rs"]
mod tests;

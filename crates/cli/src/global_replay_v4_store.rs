//! Content-addressed V4 publication, completion seal appended last.
use super::{GlobalReplayV4Bounds, codec};
use codec::Record;
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

const FILE_DOMAIN: &[u8] = b"brutex-global-replay-v4-complete-file\0";

pub(super) fn persist(
    root: &Path,
    bounds: GlobalReplayV4Bounds,
    records: &[Record],
    publication: [u8; 32],
) -> Result<(PathBuf, bool, [u8; 32]), String> {
    let root = admit_root(root)?;
    let count = codec::count(records.len())?;
    let expected_bytes = bounded_bytes(bounds, count)?;
    if records.first().map(codec::kind) != Some(0) || records.last().map(codec::kind) != Some(6) {
        return Err("Global Replay V4 requires its own header and final completion".to_owned());
    }
    for row in records {
        codec::verify_record(row)?;
    }
    let digest = codec::digest_records(FILE_DOMAIN, records.iter());
    let path = root.join(format!("{}.bin", hex(&publication)));
    let mut file = open(&path, true)?;
    file.lock().map_err(|why| why.to_string())?;
    let result = (|| {
        let before = crate::result_set::file_generation(&file, &path)?;
        if before.len > expected_bytes {
            return Err(
                "Global Replay V4 existing publication exceeds the exact prepared length"
                    .to_owned(),
            );
        }
        let mut remaining = before.len;
        for row in records {
            if remaining == 0 {
                break;
            }
            let amount = usize::try_from(remaining.min(codec::STRIDE as u64))
                .map_err(|why| why.to_string())?;
            let mut actual = [0; codec::STRIDE];
            let prefix = actual
                .get_mut(..amount)
                .ok_or("Global Replay V4 prefix range")?;
            file.read_exact(prefix).map_err(|why| why.to_string())?;
            if Some(&*prefix) != row.get(..amount) {
                return Err(
                    "Global Replay V4 existing bytes differ; no overwrite or truncation".to_owned(),
                );
            }
            remaining -= amount as u64;
        }
        crate::result_set::require_generation_unchanged(
            before,
            crate::result_set::file_generation(&file, &path)?,
            &path,
        )?;
        file.seek(SeekFrom::Start(before.len))
            .map_err(|why| why.to_string())?;
        append_range(&mut file, records, before.len, expected_bytes - 32)?;
        file.sync_all().map_err(|why| why.to_string())?;
        append_range(
            &mut file,
            records,
            before.len.max(expected_bytes - 32),
            expected_bytes,
        )?;
        file.sync_all().map_err(|why| why.to_string())?;
        File::open(&root)
            .and_then(|directory| directory.sync_all())
            .map_err(|why| why.to_string())?;
        crate::result_set::file_generation(&file, &path)?;
        Ok(before.len != expected_bytes)
    })();
    let unlock = file.unlock().map_err(|why| why.to_string());
    let written = result.and_then(|written| unlock.map(|()| written))?;
    verify(&path, bounds, count, digest)?;
    Ok((path, written, digest))
}

fn append_range(file: &mut File, records: &[Record], first: u64, end: u64) -> Result<(), String> {
    let mut at = first;
    while at < end {
        let index = usize::try_from(at / codec::STRIDE as u64).map_err(|why| why.to_string())?;
        let offset = usize::try_from(at % codec::STRIDE as u64).map_err(|why| why.to_string())?;
        let amount = usize::try_from((end - at).min((codec::STRIDE - offset) as u64))
            .map_err(|why| why.to_string())?;
        file.write_all(
            records
                .get(index)
                .and_then(|row| row.get(offset..offset + amount))
                .ok_or("Global Replay V4 append range")?,
        )
        .map_err(|why| why.to_string())?;
        at += amount as u64;
    }
    Ok(())
}

pub(super) fn verify(
    path: &Path,
    bounds: GlobalReplayV4Bounds,
    count: u64,
    expected_digest: [u8; 32],
) -> Result<(), String> {
    let expected_bytes = bounded_bytes(bounds, count)?;
    let mut file = open(path, false)?;
    file.lock_shared().map_err(|why| why.to_string())?;
    let result = (|| {
        let before = crate::result_set::file_generation(&file, path)?;
        if before.len != expected_bytes {
            return Err(
                "Global Replay V4 publication is missing, truncated or extended".to_owned(),
            );
        }
        let mut hash = brutex_core::blake3::Hasher::new();
        hash.update(FILE_DOMAIN);
        for index in 0..count {
            let mut row = [0; codec::STRIDE];
            file.read_exact(&mut row).map_err(|why| why.to_string())?;
            codec::verify_record(&row)?;
            let kind = codec::kind(&row);
            if (index == 0 && kind != 0)
                || (index > 0 && kind == 0)
                || (index == count - 1 && kind != 6)
                || (index < count - 1 && kind == 6)
            {
                return Err("Global Replay V4 completion ordering is invalid".to_owned());
            }
            hash.update(&row);
        }
        if hash.finalize() != expected_digest {
            return Err(
                "Global Replay V4 publication differs from its authenticated preparation"
                    .to_owned(),
            );
        }
        crate::result_set::require_generation_unchanged(
            before,
            crate::result_set::file_generation(&file, path)?,
            path,
        )
    })();
    let unlock = file.unlock().map_err(|why| why.to_string());
    result.and(unlock)
}

fn bounded_bytes(bounds: GlobalReplayV4Bounds, count: u64) -> Result<u64, String> {
    if count < 10 || count > bounds.records {
        return Err("Global Replay V4 record count outside explicit bounds".to_owned());
    }
    count
        .checked_mul(codec::STRIDE as u64)
        .filter(|bytes| *bytes <= bounds.bytes)
        .ok_or_else(|| "Global Replay V4 byte ceiling exceeded".to_owned())
}
fn admit_root(root: &Path) -> Result<PathBuf, String> {
    let metadata = std::fs::symlink_metadata(root).map_err(|why| why.to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("Global Replay V4 requires a nonsymlink directory".to_owned());
    }
    std::fs::canonicalize(root).map_err(|why| why.to_string())
}
fn open(path: &Path, writable: bool) -> Result<File, String> {
    if let Ok(metadata) = std::fs::symlink_metadata(path)
        && (!metadata.is_file() || metadata.file_type().is_symlink())
    {
        return Err("Global Replay V4 requires a regular nonsymlink file".to_owned());
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
        #[cfg(target_os = "macos")]
        options.custom_flags(0x100);
        #[cfg(any(target_os = "android", target_os = "linux"))]
        options.custom_flags(0x20_000);
    }
    let file = options.open(path).map_err(|why| why.to_string())?;
    let metadata = file.metadata().map_err(|why| why.to_string())?;
    if !metadata.is_file() {
        return Err("Global Replay V4 file is not regular".to_owned());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if metadata.nlink() != 1 {
            return Err("Global Replay V4 refuses hard-link aliases".to_owned());
        }
    }
    crate::result_set::file_generation(&file, path)?;
    Ok(file)
}
fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut text = String::with_capacity(64);
    for byte in bytes {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

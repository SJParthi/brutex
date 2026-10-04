//! Test-only builders for invented GDFL shapes: zip and tick-store
//! containers, and files whose every value is made up at run time. No vendor
//! row is quoted anywhere (the licence is UNVERIFIED; D-2800), and
//! `gdfl_cm::tests::fixtures_are_built_not_pasted` walks this file too.
#![expect(
    clippy::expect_used,
    reason = "a fixture that cannot be built is a test that cannot run; it panics by name"
)]

use std::io::Write as _;
use std::path::{Path, PathBuf};

use crate::gdfl_archive::crc32;
use crate::session::Day;

/// How a zip member is written.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Method {
    /// Method 0.
    Stored,
    /// Method 8.
    Deflated,
}

fn le16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn le32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn len32(n: usize) -> u32 {
    u32::try_from(n).expect("a fixture member fits 32 bits")
}

/// A plain (non-ZIP64) zip of `members`, in order.
pub(crate) fn zip(members: &[(&str, &[u8], Method)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data, method) in members {
        let packed = match method {
            Method::Stored => data.to_vec(),
            Method::Deflated => {
                let mut enc =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                enc.write_all(data).expect("deflate into memory");
                enc.finish().expect("deflate into memory")
            }
        };
        let code = match method {
            Method::Stored => 0,
            Method::Deflated => 8,
        };
        let offset = len32(out.len());
        let crc = crc32(data);
        le32(&mut out, 0x0403_4b50);
        le16(&mut out, 20);
        le16(&mut out, 0);
        le16(&mut out, code);
        le32(&mut out, 0);
        le32(&mut out, crc);
        le32(&mut out, len32(packed.len()));
        le32(&mut out, len32(data.len()));
        le16(&mut out, u16::try_from(name.len()).expect("short name"));
        le16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&packed);
        le32(&mut central, 0x0201_4b50);
        le16(&mut central, 20);
        le16(&mut central, 20);
        le16(&mut central, 0);
        le16(&mut central, code);
        le32(&mut central, 0);
        le32(&mut central, crc);
        le32(&mut central, len32(packed.len()));
        le32(&mut central, len32(data.len()));
        le16(&mut central, u16::try_from(name.len()).expect("short name"));
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le32(&mut central, 0);
        le32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let at = len32(out.len());
    let count = u16::try_from(members.len()).expect("few members");
    out.extend_from_slice(&central);
    le32(&mut out, 0x0605_4b50);
    le16(&mut out, 0);
    le16(&mut out, 0);
    le16(&mut out, count);
    le16(&mut out, count);
    le32(&mut out, len32(central.len()));
    le32(&mut out, at);
    le16(&mut out, 0);
    out
}

/// A tick-store day file (FORMAT.md v1) holding `entries` in order, each a
/// raw (kind 2) block, or a directory entry (kind 0) when its name ends `/`.
pub(crate) fn bts(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let stated: Vec<(&str, &[u8], u32)> = entries
        .iter()
        .map(|(name, data)| (*name, *data, crc32(data)))
        .collect();
    bts_stating(&stated)
}

/// [`bts`], with each entry's stated CRC-32 given rather than computed.
pub(crate) fn bts_stating(entries: &[(&str, &[u8], u32)]) -> Vec<u8> {
    let zstd = |bytes: &[u8]| {
        ruzstd::encoding::compress_to_vec(bytes, ruzstd::encoding::CompressionLevel::Fastest)
    };
    let mut out = b"BRTXTS01".to_vec();
    let mut index = Vec::new();
    for (name, data, crc) in entries {
        let dir = name.ends_with('/');
        let off = u64::try_from(out.len()).expect("fits");
        let len = if dir {
            0
        } else {
            let mut block = vec![2_u8];
            block.extend_from_slice(&zstd(data));
            out.extend_from_slice(&block);
            u64::try_from(block.len()).expect("fits")
        };
        index.extend_from_slice(&u16::try_from(name.len()).expect("short").to_le_bytes());
        index.extend_from_slice(name.as_bytes());
        index.push(if dir { 0 } else { 2 });
        index.extend_from_slice(&(if dir { 0 } else { off }).to_le_bytes());
        index.extend_from_slice(&len.to_le_bytes());
        index.extend_from_slice(&u64::try_from(data.len()).expect("fits").to_le_bytes());
        index.extend_from_slice(&crc.to_le_bytes());
        index.extend_from_slice(&0_u64.to_le_bytes());
        index.extend_from_slice(&0_u32.to_le_bytes());
    }
    let index_off = u64::try_from(out.len()).expect("fits");
    let frame = zstd(&index);
    out.extend_from_slice(&frame);
    out.extend_from_slice(&index_off.to_le_bytes());
    out.extend_from_slice(&u64::try_from(frame.len()).expect("fits").to_le_bytes());
    out.extend_from_slice(&u64::try_from(index.len()).expect("fits").to_le_bytes());
    out.extend_from_slice(&crc32(&index).to_le_bytes());
    out.extend_from_slice(&u32::try_from(entries.len()).expect("fits").to_le_bytes());
    out.extend_from_slice(b"BRTXTSE1");
    out
}

/// The vendor's three-letter month.
pub(crate) fn mon(day: Day) -> &'static str {
    crate::gdfl_archive::MONTHS
        .get(usize::from(day.month()) - 1)
        .copied()
        .expect("a month")
}

/// `HH:MM:SS` of a second of the day.
pub(crate) fn hms(sod: u32) -> String {
    format!("{:02}:{:02}:{:02}", sod / 3_600, sod / 60 % 60, sod % 60)
}

/// One invented row of a GDFL file: `stem`, the day, the time, LTP text,
/// LTQ and open interest; the quote fields are zero.
pub(crate) fn row(stem: &str, day: Day, sod: u32, ltp: &str, ltq: u64, oi: u64) -> String {
    format!(
        "{stem},{:02}/{:02}/{:04},{},{ltp},0,0,0,0,{ltq},{oi}",
        day.day(),
        day.month(),
        day.year(),
        hms(sod)
    )
}

/// A whole invented file: the modern header and `rows`, CRLF, ending in one.
pub(crate) fn csv(rows: &[String]) -> Vec<u8> {
    let mut text = String::from(crate::gdfl_cm::HEADER_OPEN_INTEREST);
    text.push_str("\r\n");
    for r in rows {
        text.push_str(r);
        text.push_str("\r\n");
    }
    text.into_bytes()
}

/// Every scratch directory this process has made, so no two calls share one.
static SCRATCH_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// A scratch directory unique to this call, empty, under the system temp
/// dir. The name carries the process, a per-process sequence number and the
/// clock, and the leaf is made with `create_dir`, which refuses a directory
/// that already exists: two tests can never share a tree (one's cleanup
/// deleting the other's files mid-walk was the shape of a `NotFound` seen
/// once in `tree`), and a collision panics here by name instead (D-3177).
pub(crate) fn scratch(tag: &str) -> PathBuf {
    let seq = SCRATCH_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "brutex-gdfl-{tag}-{}-{seq}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir(&dir).expect("a fresh scratch dir, never an existing one");
    dir
}

/// Writes `bytes` at `root/rel`, making its parents.
pub(crate) fn put(root: &Path, rel: &str, bytes: &[u8]) -> PathBuf {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("parents");
    std::fs::write(&path, bytes).expect("the file");
    path
}

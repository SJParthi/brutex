//! v3.4 E8 + E9 (build:shard; cloud-kit/CLOUD-KIT-DESIGN.md E8 / E9 with critic F1, F14, F15): the per-segment SLICE of a day's cube and
//! seconds tape, and its TRANSPORT form, decoded in memory.
//!
//! - A slice (`SFCUBS01` / `SFSECS01`) keeps the whole header, EVERY underlying and contract record, the strings and every spot series, so
//!   every contract and underlying keeps its index; it holds the per-contract arrays and the seconds rows of ONE contiguous range [c0, c1)
//!   of that root's contracts (the cube builder sorts index contracts first: index = [0, n_idx), stock = [n_idx, nc)). The loader expands it
//!   to full-size arrays (the absent contracts zero) and sets `lim`; every accessor counts a read outside the allowed ranges
//!   (`kit::SLICE_VIOLATIONS`) and a shard refuses at its next batch file. With `--cube-extra` the extra root (all index) is sliced alone
//!   and the merged day allows BOTH ranges [c0, c1) and [nc_main, nc_main + nc_extra) (critic F1).
//! - The transport file `<day>.sfz` holds one day's slice of one root: the cube slice under zstd, the seconds slice's header and offsets
//!   under zstd, and its rows as 4 columnar streams of LEB128 varints under zstd (second delta and high delta as zigzag deltas from the
//!   previous row, high - low zigzag, volume), each section with its raw length and sha256. Decoding rebuilds the slice's exact bytes
//!   (a bijection; any row order, so an unsorted second costs bytes, never exactness); a section whose sha256 differs is refused. zstd runs
//!   as the configured tool (`--tool-zstd` / `SF_ZSTD`; the Mac default /opt/homebrew/bin/zstd), the route mtable.rs already uses (critic
//!   F14: no vendored decoder).
//! - The inputs digest of a root's day is the sha256 of its slice bytes, whichever form the root holds (whole cube, slice files or a
//!   transport file), so a shard's inputs manifest equals the Mac's (E4 / F4).

use std::io::Write as _;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::cube::{Cube, SecTape};
use crate::kit::{Sha256, sha256_bytes};

/// The transport file's magic.
pub const SFZ_MAGIC: &[u8; 8] = b"SFZ00001";
const NSECT: usize = 6;

/// E9: decode times of the transport sections a run read (ns), for the p50 / p99 line (MEASURED).
pub static DECODE_NS: Mutex<Vec<u64>> = Mutex::new(Vec::new());
/// E9: transport bytes read.
pub static SFZ_BYTES: AtomicU64 = AtomicU64::new(0);

/// The contiguous contract range of segment `seg` (0 index, 1 stock) in a cube whose contracts are sorted index first.
///
/// # Errors
/// A cube whose contracts are not sorted index first (a slice would not be one range).
pub fn seg_range(c: &Cube, seg: u8) -> Result<(usize, usize), String> {
    let n_idx = c.cons.iter().take_while(|k| k.kind == 0).count();
    if let Some(k) = c.cons.iter().skip(n_idx).find(|k| k.kind == 0) {
        return Err(format!("{}: contract {} ({}) is an index contract after the first stock contract: the contracts are not sorted index first, so a segment is not ONE range: refused", c.ymd, k.sid, k.symbol));
    }
    Ok(if seg == 0 { (0, n_idx) } else { (n_idx, c.cons.len()) })
}

/// The slice bytes (cube, seconds) of `seg` from a whole cube and tape of one root.
///
/// # Errors
/// A cube that is not sorted index first.
pub fn slice_of(c: &Cube, t: &SecTape, seg: u8) -> Result<(Vec<u8>, Vec<u8>), String> {
    let (c0, c1) = seg_range(c, seg)?;
    if t.off.len() != c.cons.len() + 1 {
        return Err(format!("{}: the seconds tape has {} contracts, the cube {}", c.ymd, t.off.len().saturating_sub(1), c.cons.len()));
    }
    Ok((crate::cube::encode_with(c, Some((seg, c0, c1))), crate::cube::encode_sec_with(t, c.ymd, Some((seg, c0, c1)))))
}

fn magic_of(b: &[u8]) -> &[u8] {
    b.get(..8).unwrap_or(&[])
}

/// The slice bytes of root `root`'s day `d` for segment `seg`, from whatever form the root holds: a transport file (decoded), slice
/// files (taken as they are, after checking their segment) or a whole cube (sliced). None when the root holds no file of the day.
///
/// # Errors
/// An unreadable or foreign file, a slice of another segment, a cube not sorted index first.
pub fn slice_bytes(root: &Path, d: &str, seg: u8) -> Result<Option<(Vec<u8>, Vec<u8>)>, String> {
    let (pz, pb, ps) = (root.join(format!("{d}.sfz")), root.join(format!("{d}.bin")), root.join(format!("{d}.sec.bin")));
    if pz.exists() {
        let z = std::fs::read(&pz).map_err(|e| format!("{}: {e}", pz.display()))?;
        let (zseg, _) = sfz_head(&z, &pz)?;
        if zseg != u32::from(seg) {
            return Err(format!("{}: a transport file of segment {zseg}, the run walks {seg}: refused", pz.display()));
        }
        return Ok(Some((sfz_decode(&z, &pz, true)?, sfz_decode(&z, &pz, false)?)));
    }
    if !pb.exists() {
        return Ok(None);
    }
    let b = std::fs::read(&pb).map_err(|e| format!("{}: {e}", pb.display()))?;
    let s = std::fs::read(&ps).map_err(|e| format!("{}: {e}", ps.display()))?;
    if magic_of(&b) == crate::cube::SLICE_MAGIC.as_slice() {
        let bseg = b.get(12..16).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]])).unwrap_or(u32::MAX);
        let sseg = s.get(12..16).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]])).unwrap_or(u32::MAX);
        if bseg != u32::from(seg) || sseg != u32::from(seg) || magic_of(&s) != crate::cube::SLICE_MAGIC_SEC.as_slice() {
            return Err(format!("{}: a slice of segment {bseg} / tape {sseg}, the run walks {seg}: refused", pb.display()));
        }
        return Ok(Some((b, s)));
    }
    let c = crate::cube::decode_cube(&b, &pb)?;
    let t = crate::cube::decode_sec(&s, &ps)?;
    Ok(Some(slice_of(&c, &t, seg)?))
}

/// E4 + F4: one root's day in the inputs manifest: (cube file present, tape file present, sha256 of the segment slice's bytes or "-").
/// A transport file counts as both files.
///
/// # Errors
/// As [`slice_bytes`].
pub fn root_day_digest(root: &Path, d: &str, seg: u8) -> Result<(bool, bool, String), String> {
    let (pz, pb, ps) = (root.join(format!("{d}.sfz")), root.join(format!("{d}.bin")), root.join(format!("{d}.sec.bin")));
    let (hb, hs) = if pz.exists() { (true, true) } else { (pb.exists(), ps.exists()) };
    if !(hb && hs) {
        return Ok((hb, hs, "-".to_owned()));
    }
    let Some((b, s)) = slice_bytes(root, d, seg)? else { return Ok((hb, hs, "-".to_owned())) };
    let mut h = Sha256::new();
    h.update(sha256_bytes(&b).as_bytes());
    h.update(b" ");
    h.update(sha256_bytes(&s).as_bytes());
    Ok((hb, hs, h.hex()))
}

// ------------------------------------------------------------------------------------------------ zstd through the configured tool

fn zstd_pipe(args: &[&str], input: &[u8]) -> Result<Vec<u8>, String> {
    let tool = crate::kit::tool("zstd");
    let mut child = std::process::Command::new(&tool).args(args).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().map_err(|e| format!("zstd ({tool}) {args:?}: {e}"))?;
    let mut sin = child.stdin.take().ok_or("zstd: no stdin")?;
    let out = std::thread::scope(|s| -> Result<std::process::Output, String> {
        let w = s.spawn(move || -> Result<(), String> {
            sin.write_all(input).map_err(|e| format!("zstd stdin: {e}"))?;
            drop(sin);
            Ok(())
        });
        let o = child.wait_with_output().map_err(|e| format!("zstd: {e}"))?;
        w.join().map_err(|_| "zstd writer".to_owned())??;
        Ok(o)
    })?;
    if !out.status.success() {
        return Err(format!("zstd ({tool}) {args:?}: exit {} {}", out.status, String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(out.stdout)
}

/// zstd level 3 (the design's packer level), in memory.
///
/// # Errors
/// The tool failed.
pub fn zstd_c(b: &[u8]) -> Result<Vec<u8>, String> {
    zstd_pipe(&["-3", "-q", "-c"], b)
}

/// zstd decompression, in memory.
///
/// # Errors
/// The tool failed (a damaged stream).
pub fn zstd_d(b: &[u8]) -> Result<Vec<u8>, String> {
    zstd_pipe(&["-d", "-q", "-c"], b)
}

// ------------------------------------------------------------------------------------------------ varints

fn put_var(v: &mut Vec<u8>, mut x: u64) {
    while x >= 0x80 {
        v.push(u8::try_from(x & 0x7f).unwrap_or(0) | 0x80);
        x >>= 7;
    }
    v.push(u8::try_from(x).unwrap_or(0));
}

fn get_var(b: &[u8], p: &mut usize) -> Result<u64, String> {
    let mut x = 0_u64;
    let mut sh = 0_u32;
    loop {
        let c = *b.get(*p).ok_or("varint stream truncated")?;
        *p += 1;
        if sh > 63 {
            return Err("varint too long".to_owned());
        }
        x |= u64::from(c & 0x7f) << sh;
        if c & 0x80 == 0 {
            return Ok(x);
        }
        sh += 7;
    }
}

const fn zz(x: i64) -> u64 {
    ((x << 1) ^ (x >> 63)) as u64
}

const fn unzz(x: u64) -> i64 {
    ((x >> 1) as i64) ^ -((x & 1) as i64)
}

// ------------------------------------------------------------------------------------------------ the transport file

/// The seconds slice's header + offsets length (8 magic + 16 slice header + 24 v1 header + (nc + 1) x 8) and its row count.
fn sec_split(s: &[u8]) -> Result<(usize, usize), String> {
    if magic_of(s) != crate::cube::SLICE_MAGIC_SEC.as_slice() {
        return Err("transport: the seconds part is not a slice".to_owned());
    }
    let u32_at = |o: usize| s.get(o..o + 4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]));
    let nc = usize::try_from(u32_at(32).ok_or("transport: short seconds header")?).unwrap_or(0);
    let nr = s.get(40..48).map(|x| u64::from_le_bytes([x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7]])).ok_or("transport: short seconds header")?;
    let head = 48 + (nc + 1) * 8;
    let nr = usize::try_from(nr).unwrap_or(usize::MAX);
    if s.len() != head + nr * 16 {
        return Err(format!("transport: seconds slice of {} bytes, its header says {}", s.len(), head + nr * 16));
    }
    Ok((head, nr))
}

/// E9: one root's day slice (cube bytes, seconds bytes) as a transport file.
///
/// # Errors
/// A seconds part that is not a slice; the zstd tool failed.
pub fn sfz_encode(bin: &[u8], sec: &[u8], seg: u8, ymd: u32) -> Result<Vec<u8>, String> {
    let (head, nr) = sec_split(sec)?;
    let mut cols: [Vec<u8>; 4] = [Vec::with_capacity(nr * 2), Vec::with_capacity(nr * 2), Vec::with_capacity(nr * 2), Vec::with_capacity(nr * 2)];
    let (mut ps, mut ph) = (0_i64, 0_i64);
    for r in 0..nr {
        let o = head + r * 16;
        let f = |k: usize| sec.get(o + 4 * k..o + 4 * k + 4).map_or(0, |x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]));
        let (sod, high, low, vol) = (i64::from(f(0)), i64::from(f(1).cast_signed()), i64::from(f(2).cast_signed()), u64::from(f(3)));
        let [c0, c1, c2, c3] = &mut cols;
        put_var(c0, zz(sod - ps));
        put_var(c1, zz(high - ph));
        put_var(c2, zz(high - low));
        put_var(c3, vol);
        ps = sod;
        ph = high;
    }
    let raws: [&[u8]; NSECT] = [bin, sec.get(..head).unwrap_or(&[]), &cols[0], &cols[1], &cols[2], &cols[3]];
    let comps: Vec<Vec<u8>> = raws.iter().map(|r| zstd_c(r)).collect::<Result<_, _>>()?;
    let mut out = Vec::with_capacity(comps.iter().map(Vec::len).sum::<usize>() + 64 + NSECT * 48);
    out.extend_from_slice(SFZ_MAGIC);
    for x in [1_u32, u32::from(seg), ymd, u32::try_from(NSECT).unwrap_or(0)] {
        out.extend_from_slice(&x.to_le_bytes());
    }
    for (r, c) in raws.iter().zip(&comps) {
        out.extend_from_slice(&(r.len() as u64).to_le_bytes());
        out.extend_from_slice(&(c.len() as u64).to_le_bytes());
        let mut h = Sha256::new();
        h.update(r);
        out.extend_from_slice(&h.finish());
    }
    for c in &comps {
        out.extend_from_slice(c);
    }
    Ok(out)
}

/// The transport file's (segment, ymd).
///
/// # Errors
/// A foreign or short file.
pub fn sfz_head(z: &[u8], p: &Path) -> Result<(u32, u32), String> {
    if magic_of(z) != SFZ_MAGIC.as_slice() {
        return Err(format!("{}: not a transport file", p.display()));
    }
    let u = |o: usize| z.get(o..o + 4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]])).ok_or_else(|| format!("{}: short transport header", p.display()));
    if u(8)? != 1 || u(20)? != u32::try_from(NSECT).unwrap_or(0) {
        return Err(format!("{}: transport version / sections not 1 / {NSECT}", p.display()));
    }
    Ok((u(12)?, u(16)?))
}

/// The sections' (raw length, compressed range, raw sha256).
fn sections(z: &[u8], p: &Path) -> Result<Vec<(usize, std::ops::Range<usize>, [u8; 32])>, String> {
    let mut v = Vec::with_capacity(NSECT);
    let mut at = 24 + NSECT * 48;
    for i in 0..NSECT {
        let o = 24 + i * 48;
        let g = |k: usize| z.get(k..k + 8).map(|x| u64::from_le_bytes([x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7]])).ok_or_else(|| format!("{}: short section table", p.display()));
        let (rl, cl) = (usize::try_from(g(o)?).unwrap_or(usize::MAX), usize::try_from(g(o + 8)?).unwrap_or(usize::MAX));
        let mut sh = [0_u8; 32];
        sh.copy_from_slice(z.get(o + 16..o + 48).ok_or_else(|| format!("{}: short section table", p.display()))?);
        v.push((rl, at..at + cl, sh));
        at += cl;
    }
    if at != z.len() {
        return Err(format!("{}: {} bytes, its sections end at {at}: truncated or padded: refused", p.display(), z.len()));
    }
    Ok(v)
}

fn section(z: &[u8], p: &Path, s: &(usize, std::ops::Range<usize>, [u8; 32]), what: &str) -> Result<Vec<u8>, String> {
    let raw = zstd_d(z.get(s.1.clone()).ok_or_else(|| format!("{}: {what}: out of the file", p.display()))?)?;
    let mut h = Sha256::new();
    h.update(&raw);
    if raw.len() != s.0 || h.finish() != s.2 {
        return Err(format!("{}: {what}: decoded {} bytes whose sha256 is not the section's: refused (re-fetch the object)", p.display(), raw.len()));
    }
    Ok(raw)
}

/// E9: decodes a transport file's cube slice (`bin` true) or seconds slice, byte for byte as the slice files.
///
/// # Errors
/// A damaged file (lengths, sha256, a varint stream that does not end with the rows).
pub fn sfz_decode(z: &[u8], p: &Path, bin: bool) -> Result<Vec<u8>, String> {
    let t0 = std::time::Instant::now();
    let secs = sections(z, p)?;
    let out = if bin {
        section(z, p, secs.first().ok_or("sections")?, "cube slice")?
    } else {
        let head = section(z, p, secs.get(1).ok_or("sections")?, "seconds header")?;
        let nr = head.get(40..48).map(|x| u64::from_le_bytes([x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7]])).ok_or_else(|| format!("{}: short seconds header", p.display()))?;
        let nr = usize::try_from(nr).unwrap_or(usize::MAX);
        let cols: Vec<Vec<u8>> = (2..NSECT).map(|i| section(z, p, secs.get(i).ok_or("sections")?, "seconds column")).collect::<Result<_, _>>()?;
        let mut out = Vec::with_capacity(head.len() + nr * 16);
        out.extend_from_slice(&head);
        let mut pos = [0_usize; 4];
        let (mut ps, mut ph) = (0_i64, 0_i64);
        // MUTANT m_sfz_drops_last_row: the last seconds row never rebuilt
        let n = if cfg!(feature = "m_sfz_drops_last_row") { nr.saturating_sub(1) } else { nr };
        for _ in 0..n {
            let mut g = |k: usize| -> Result<u64, String> { get_var(cols.get(k).ok_or("column")?, pos.get_mut(k).ok_or("column")?) };
            let sod = ps + unzz(g(0)?);
            let high = ph + unzz(g(1)?);
            let low = high - unzz(g(2)?);
            let vol = g(3)?;
            out.extend_from_slice(&u32::try_from(sod).map_err(|_| "transport: second out of range")?.to_le_bytes());
            out.extend_from_slice(&i32::try_from(high).map_err(|_| "transport: high out of range")?.to_le_bytes());
            out.extend_from_slice(&i32::try_from(low).map_err(|_| "transport: low out of range")?.to_le_bytes());
            out.extend_from_slice(&u32::try_from(vol).map_err(|_| "transport: volume out of range")?.to_le_bytes());
            ps = sod;
            ph = high;
        }
        if cols.iter().zip(pos).any(|(c, q)| c.len() != q) {
            return Err(format!("{}: a seconds column holds bytes beyond its {nr} rows: refused", p.display()));
        }
        out
    };
    if let Ok(mut v) = DECODE_NS.lock() {
        v.push(u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX));
    }
    SFZ_BYTES.fetch_add(z.len() as u64, Ordering::Relaxed);
    Ok(out)
}

fn read_sfz(root: &Path, d: &str) -> Result<(Vec<u8>, std::path::PathBuf), String> {
    let p = root.join(format!("{d}.sfz"));
    let z = std::fs::read(&p).map_err(|e| format!("{}: {e} (E9: the shard's cube root holds transport files)", p.display()))?;
    Ok((z, p))
}

/// E9 loader hook: the cube slice bytes of `root`'s transport file of day `d`.
///
/// # Errors
/// As [`sfz_decode`].
pub fn sfz_bin(root: &Path, d: &str) -> Result<Vec<u8>, String> {
    let (z, p) = read_sfz(root, d)?;
    sfz_decode(&z, &p, true)
}

/// E9 loader hook: the seconds slice bytes of `root`'s transport file of day `d`.
///
/// # Errors
/// As [`sfz_decode`].
pub fn sfz_sec(root: &Path, d: &str) -> Result<Vec<u8>, String> {
    let (z, p) = read_sfz(root, d)?;
    sfz_decode(&z, &p, false)
}

/// E9: the MEASURED decode line (p50 / p99 per section decode, bytes read).
#[must_use]
pub fn decode_line() -> String {
    let mut v = DECODE_NS.lock().map(|v| v.clone()).unwrap_or_default();
    if v.is_empty() {
        return "E9 transport: no section decoded".to_owned();
    }
    v.sort_unstable();
    let q = |p: usize| v.get((v.len() - 1) * p / 100).copied().unwrap_or(0) as f64 / 1e6;
    format!("E9 transport (MEASURED): {} section decodes, p50 {:.1} ms / p99 {:.1} ms / max {:.1} ms; {} transport bytes read", v.len(), q(50), q(99), q(100), SFZ_BYTES.load(Ordering::Relaxed))
}

// ------------------------------------------------------------------------------------------------ the packer (`spikefade cube-slice`)

/// `spikefade cube-slice <root> <out dir> <idx|stk> [--transport] <days...>`: writes each day's segment slice of ONE cube root (`<d>.bin` +
/// `<d>.sec.bin`, or with `--transport` one `<d>.sfz`) and copies its `<d>.manifest.tsv`; never overwrites a file; checks every written day
/// by decoding it back (the slice bytes and, through the loader, every array of the segment's contracts equal to the whole cube's: critic
/// F15's self-check). A day the root does not hold is skipped and listed (the extra root holds 2021-2026 only).
///
/// # Errors
/// A bad argument, an existing output, a cube not sorted index first, a failed check.
pub fn cmd_cube_slice(a: &[String]) -> Result<String, String> {
    let (Some(root), Some(out), Some(seg)) = (a.get(2), a.get(3), a.get(4)) else { return Err("cube-slice <root> <out dir> <idx|stk> [--transport] <days...>".to_owned()) };
    let seg: u8 = match seg.as_str() {
        "idx" => 0,
        "stk" => 1,
        o => return Err(format!("cube-slice: segment idx|stk, not {o}")),
    };
    let transport = a.iter().any(|x| x == "--transport");
    let days: Vec<&String> = a.iter().skip(5).filter(|x| x.len() == 10 && x.starts_with("20")).collect();
    let (root, out) = (Path::new(root), Path::new(out));
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut lines = Vec::new();
    let (mut raw_t, mut out_t, mut skipped) = (0_u64, 0_u64, Vec::new());
    for d in days {
        let (pb, ps) = (root.join(format!("{d}.bin")), root.join(format!("{d}.sec.bin")));
        if !pb.exists() {
            skipped.push(d.clone());
            continue;
        }
        let t0 = std::time::Instant::now();
        let full_b = std::fs::read(&pb).map_err(|e| format!("{}: {e}", pb.display()))?;
        let full_s = std::fs::read(&ps).map_err(|e| format!("{}: {e}", ps.display()))?;
        let c = crate::cube::decode_cube(&full_b, &pb)?;
        let t = crate::cube::decode_sec(&full_s, &ps)?;
        if c.lim.is_some() || t.lim.is_some() {
            return Err(format!("{}: already a slice: cube-slice reads whole cubes", pb.display()));
        }
        let (c0, c1) = seg_range(&c, seg)?;
        let (b, s) = slice_of(&c, &t, seg)?;
        let names: Vec<std::path::PathBuf> = if transport { vec![out.join(format!("{d}.sfz"))] } else { vec![out.join(format!("{d}.bin")), out.join(format!("{d}.sec.bin"))] };
        if let Some(x) = names.iter().find(|x| x.exists()) {
            return Err(format!("{}: exists (never overwritten): refused", x.display()));
        }
        let written: u64 = if transport {
            let z = sfz_encode(&b, &s, seg, c.ymd)?;
            // the self-check: both parts decode back to the slice's exact bytes
            let p = names.first().ok_or("names")?;
            if sfz_decode(&z, p, true)? != b || sfz_decode(&z, p, false)? != s {
                return Err(format!("{d}: the transport file does not decode to its slice: refused"));
            }
            write_new(p, &z)?;
            z.len() as u64
        } else {
            write_new(names.first().ok_or("names")?, &b)?;
            write_new(names.get(1).ok_or("names")?, &s)?;
            (b.len() + s.len()) as u64
        };
        // the loader's view of the slice equals the whole cube on every contract of the range
        let lc = crate::cube::decode_cube(&b, &pb)?;
        let lt = crate::cube::decode_sec(&s, &ps)?;
        check_slice_equal(&c, &t, &lc, &lt, c0, c1)?;
        let man = root.join(format!("{d}.manifest.tsv"));
        if man.exists() {
            let mo = out.join(format!("{d}.manifest.tsv"));
            if !mo.exists() {
                std::fs::copy(&man, &mo).map_err(|e| format!("{}: {e}", mo.display()))?;
            }
        }
        raw_t += (full_b.len() + full_s.len()) as u64;
        out_t += written;
        let mut h = Sha256::new();
        h.update(sha256_bytes(&b).as_bytes());
        h.update(b" ");
        h.update(sha256_bytes(&s).as_bytes());
        lines.push(format!("{d}\tseg {seg}\tcontracts [{c0}, {c1}) of {}\twhole {} B\t{} {written} B\tslice sha256 {}\t{:.2} s", c.cons.len(), full_b.len() + full_s.len(), if transport { "transport" } else { "slice" }, h.hex(), t0.elapsed().as_secs_f64()));
    }
    Ok(format!("cube-slice {} -> {} ({}): {} day(s), {raw_t} B whole -> {out_t} B written ({:.2}x); not held by this root: {:?}\n{}", root.display(), out.display(), if transport { "E9 transport" } else { "E8 slice" }, lines.len(), if out_t > 0 { raw_t as f64 / out_t as f64 } else { 0.0 }, skipped, lines.join("\n")))
}

fn write_new(p: &Path, b: &[u8]) -> Result<(), String> {
    let part = p.with_extension("part");
    std::fs::write(&part, b).map_err(|e| format!("{}: {e}", part.display()))?;
    std::fs::rename(&part, p).map_err(|e| format!("{}: {e}", p.display()))
}

/// The loaded slice equals the whole cube on every contract of [c0, c1) and on every record and spot series; the other contracts' arrays
/// are zero and outside `lim`.
///
/// # Errors
/// The first difference.
pub fn check_slice_equal(c: &Cube, t: &SecTape, lc: &Cube, lt: &SecTape, c0: usize, c1: usize) -> Result<(), String> {
    use crate::cal::NM;
    if lc.cons.len() != c.cons.len() || lc.unds.len() != c.unds.len() || lc.spot != c.spot || lc.ymd != c.ymd {
        return Err(format!("{}: the slice's records or spot series differ", c.ymd));
    }
    if lc.cons.iter().zip(&c.cons).any(|(x, y)| x.sid != y.sid || x.symbol != y.symbol || x.lot != y.lot || x.strike != y.strike || x.expiry != y.expiry || x.und != y.und || x.kind != y.kind || x.right != y.right || x.first_min != y.first_min || x.first_open != y.first_open || x.n_sec != y.n_sec) {
        return Err(format!("{}: a contract record differs", c.ymd));
    }
    if lc.lim.as_deref() != Some(&[(c0, c1)][..]) || lt.lim.as_deref() != Some(&[(c0, c1)][..]) {
        return Err(format!("{}: the slice's allowed range is not [{c0}, {c1})", c.ymd));
    }
    let (a, z, a1, z1) = (c0 * NM, c1 * NM, c0 * (NM + 1), c1 * (NM + 1));
    let eq = lc.close.get(a..z) == c.close.get(a..z) && lc.flags.get(a..z) == c.flags.get(a..z) && lc.high.get(a..z) == c.high.get(a..z) && lc.low.get(a..z) == c.low.get(a..z) && lc.vol.get(a..z) == c.vol.get(a..z) && lc.oi.get(a..z) == c.oi.get(a..z) && lc.cumvol.get(a1..z1) == c.cumvol.get(a1..z1) && lc.cumval.get(a1..z1) == c.cumval.get(a1..z1);
    if !eq {
        return Err(format!("{}: an array of contracts [{c0}, {c1}) differs from the whole cube", c.ymd));
    }
    let out_zero = |v: &[i32]| v.iter().take(a).chain(v.iter().skip(z)).all(|&x| x == 0);
    if !out_zero(&lc.close) || !out_zero(&lc.high) || lc.cumvol.iter().take(a1).chain(lc.cumvol.iter().skip(z1)).any(|&x| x != 0) {
        return Err(format!("{}: a contract outside [{c0}, {c1}) holds data in the slice", c.ymd));
    }
    for ci in 0..c.cons.len() {
        let want: &[crate::cube::Sec] = if ci >= c0 && ci < c1 { t.of(ci) } else { &[] };
        let got = lt.rows.get(usize::try_from(lt.off.get(ci).copied().unwrap_or(0)).unwrap_or(0)..usize::try_from(lt.off.get(ci + 1).copied().unwrap_or(0)).unwrap_or(0)).unwrap_or(&[]);
        if got.len() != want.len() || got.iter().zip(want).any(|(x, y)| x.sod != y.sod || x.high != y.high || x.low != y.low || x.vol != y.vol) {
            return Err(format!("{}: contract {ci}'s seconds differ in the slice", c.ymd));
        }
    }
    Ok(())
}

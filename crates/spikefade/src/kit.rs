//! v3.4 SHARD KIT (build:shard, MAIN-RULINGS-ENGINE-v22 Addendum 30.3; cloud-kit/CLOUD-KIT-DESIGN.md E1-E6, E7, E8, E9, E12 with the
//! critic's MUST items F1-F5 of cloud-kit/CLOUD-KIT-CRITIC.md): the exact batch sharding of a day-major run.
//!
//! A day-major run's batches (fixed sets of unique lists) are independent: no account of a batch ever reads another batch's accounts, and
//! every accumulator a finished batch leaves merges exactly (sums; top-K under a STRICT total order; the flagged rows concatenated in batch
//! order). So N processes may walk disjoint batch sets of one EXPLICIT plan (E2), each writing every finished batch's OWN contributions to a
//! batch file (E1, `<dir>/<seg>.b<k>.bck`: a canonical body with its sha256, F2, plus a provenance block outside the hash, E4); `spikefade
//! ckpt-merge` (E3) folds the files in batch order into the engine's own checkpoint, and ONE ordinary resume runs the closing passes.
//!
//! Switches (each OFF by default: with none on, this binary replays v3.3 final 8d9a5b67 byte for byte, proof P-S0):
//! - SHARD (E1, `--shard on --batch-set <ids>`): walk only the named batches of the plan; write each one's batch file; skip the orphan
//!   events (counters only: the Mac's union pass rebuilds them) and the cumulative checkpoint; stop with "SHARD DONE" BEFORE the closing
//!   passes; a re-run skips every assigned batch whose file verifies.
//! - BFILES (E7, `--batch-files on`): an ordinary run that ALSO writes every batch's file (the reference of the proofs, spot audits).
//! - PORTABLE (E5, `--portable on`): sha256 in-process (no /usr/bin/shasum), tool paths from flags / environment (no Mac paths), directory
//!   reads sorted by name, IST labels from the system clock (+ 5 h 30 min, no tzdata), RSS / CPU from /proc when it exists.
//! - STRICT (F4, `--strict on`): every external-tool or lookup failure of the run is FATAL (the bhavcopy path swallowed them: previous close
//!   0 and staged expiries), and both cube roots' day sets are checked against the expected inputs (presence AND absence) at start.
//!   SHARD and BFILES imply PORTABLE and STRICT.
//! - `--batch-plan PLAN` (E2): the plan's batches replace the size_of batch gate; the plan's sha256 and the run's LIST DIGEST (E6 / F5: every
//!   event's identity and attribute record, every per-config list's members, every unique list's definition AND members, the settings)
//!   join the checkpoint fingerprint, and a run whose set-up differs from the plan's is refused before its first batch.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::acct::Sum;
use crate::ckpt::{fig_line, fig_of_line, sum_line, sum_of_line};

/// A kit switch (an index into the kit switch table).
pub type Kit = usize;
/// E1: shard mode.
pub const SHARD: Kit = 0;
/// E7: an ordinary run that also writes every batch's file.
pub const BFILES: Kit = 1;
/// E5: portability (in-process sha256, tool paths, sorted reads, system clock, /proc).
pub const PORTABLE: Kit = 2;
/// F4: every tool or lookup failure fatal; the day sets checked.
pub const STRICT: Kit = 3;
const N: usize = 4;
/// The number of kit switches (tests reset every one).
pub const COUNT: usize = N;

#[cfg(not(test))]
static FLAGS: [std::sync::atomic::AtomicBool; N] = [const { std::sync::atomic::AtomicBool::new(false) }; N];
#[cfg(test)]
thread_local! {
    static FLAGS: [std::cell::Cell<bool>; N] = const { [const { std::cell::Cell::new(false) }; N] };
    static TOOL_T: std::cell::RefCell<BTreeMap<String, String>> = const { std::cell::RefCell::new(BTreeMap::new()) };
}

/// One kit switch's value.
#[inline]
#[must_use]
pub fn on(s: Kit) -> bool {
    #[cfg(not(test))]
    {
        FLAGS.get(s).is_some_and(|f| f.load(Ordering::Relaxed))
    }
    #[cfg(test)]
    {
        FLAGS.with(|f| f.get(s).is_some_and(std::cell::Cell::get))
    }
}

/// Sets one kit switch.
pub fn set(s: Kit, v: bool) {
    #[cfg(not(test))]
    {
        if let Some(f) = FLAGS.get(s) {
            f.store(v, Ordering::Relaxed);
        }
    }
    #[cfg(test)]
    {
        FLAGS.with(|f| {
            if let Some(c) = f.get(s) {
                c.set(v);
            }
        });
    }
}

/// Whether any kit switch is on.
#[must_use]
pub fn any() -> bool {
    (0..N).any(on)
}

/// The kit words (RUN.log; the fingerprint part comes from [`fp_part`]).
#[must_use]
pub fn words() -> String {
    let w = |s: Kit, a: &str, b: &str| if on(s) { a.to_owned() } else { b.to_owned() };
    let mut v = vec![
        w(SHARD, "SHARD (E1: assigned batches only, batch files, no closing passes)", "not a shard"),
        w(BFILES, "batch files (E7)", "no batch files"),
        w(PORTABLE, "PORTABLE (E5: in-process sha256, tool paths from flags / env, sorted reads, system clock, /proc)", "Mac tools"),
        w(STRICT, "STRICT (F4: tool and lookup failures fatal, day sets checked)", "lenient lookups"),
    ];
    if let Some(p) = PLAN.get() {
        v.push(format!("batch plan {} ({} batches, sha256 {})", p.path, p.b.len(), p.sha));
    }
    v.join("; ")
}

/// The explicit batch plan of the run (`--batch-plan`).
pub static PLAN: OnceLock<Plan> = OnceLock::new();
/// `--write-plan FILE`: the set-up runs, the plan is written, nothing is walked.
pub static WRITE_PLAN: OnceLock<PathBuf> = OnceLock::new();
/// `--lists-per-batch N` (the cut a written plan uses instead of the --batch-accounts / size_of gate).
pub static LPB: AtomicU64 = AtomicU64::new(0);
/// `--batch-set`: the shard's batch indexes.
pub static BATCH_SET: OnceLock<BTreeSet<usize>> = OnceLock::new();
/// `--expect-inputs FILE`: the expected inputs manifest (F4 day sets + E4 content digests), checked at start.
pub static EXPECT_INPUTS: OnceLock<PathBuf> = OnceLock::new();
/// The run's list digest (E6), once computed.
pub static DIGEST: OnceLock<String> = OnceLock::new();
/// The run's inputs manifest sha256 (E4), once computed.
pub static INPUTS_SHA: OnceLock<String> = OnceLock::new();
/// E2 / E6: the run's ARGUMENT digest (every result-bearing flag and value; paths, days and runtime-only flags left out), set by `from_args`.
pub static ARGS_DIGEST: OnceLock<String> = OnceLock::new();

/// Flags that never change a result and legitimately differ between the plan writer, the reference, the shards and the resume (their
/// values included); left out of the argument digest.
const RUNTIME_FLAGS: [&str; 20] = ["--threads", "--checkpoint", "--batch-set", "--batch-files", "--shard", "--cube-transport", "--expect-inputs", "--stop-after-batches", "--portable", "--strict", "--batch-plan", "--write-plan", "--lists-per-batch", "--batch-accounts", "--tool-zstd", "--tool-unzip", "--tool-gzip", "--tool-ps", "--tool-hostname", "--measure33"];
/// Flags whose value is a PATH: only the role enters the argument digest (the content is the inputs manifest's, E4).
const PATH_FLAGS: [&str; 15] = ["--var", "--holidays", "--holidays2", "--menu", "--sets", "--margin-futures", "--special-sessions", "--tbill", "--idx-elm", "--freeze", "--ban-dir", "--bhav-dir", "--margin-manifest", "--margin-extra", "--cube-extra"];

/// E2 / E6: the argument digest of a `run3` command line: sha256 over the sorted (flag, value) pairs, the cube and output folders, the days,
/// the runtime-only flags and the path VALUES left out (a path flag counts by its role). Two runs with equal digests use the same switches,
/// sets, dials and grids, so a shard started with another flag (another `--ngrid-idx`, `--liq-cover-s`, a switch off) is refused at its
/// plan check, before its first batch.
#[must_use]
pub fn args_digest(a: &[String]) -> String {
    let is_day = |x: &str| x.len() == 10 && x.starts_with("20") && x.as_bytes().get(4) == Some(&b'-');
    let mut pairs: Vec<String> = Vec::new();
    let mut i = 4;
    while let Some(x) = a.get(i) {
        i += 1;
        if !x.starts_with("--") {
            continue;
        }
        let v = a.get(i).filter(|v| !v.starts_with("--") && !is_day(v));
        if v.is_some() {
            i += 1;
        }
        if RUNTIME_FLAGS.contains(&x.as_str()) {
            continue;
        }
        let val = if PATH_FLAGS.contains(&x.as_str()) { "(path)".to_owned() } else { v.cloned().unwrap_or_else(|| "-".to_owned()) };
        pairs.push(format!("{x}\t{val}"));
    }
    pairs.sort();
    sha256_bytes(pairs.join("\n").as_bytes())
}
/// `--tool-<name> PATH` overrides (E5).
static TOOLS: OnceLock<BTreeMap<String, String>> = OnceLock::new();
/// F4: the failures STRICT turned fatal (checked before every batch file and at the end of the set-up).
static STRICT_ERRS: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// E8 (critic F1): reads of a contract OUTSIDE the loaded segment slice's ranges (must stay 0; a shard refuses at the next batch file).
pub static SLICE_VIOLATIONS: AtomicU64 = AtomicU64::new(0);
/// E9: the cube directory holds transport files (`<day>.sfz`), decoded in memory at each load.
pub static TRANSPORT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// E4: the process start (UTC seconds) for the provenance block.
pub static T0_UTC: AtomicU64 = AtomicU64::new(0);

/// Reads the kit flags of `run3`.
///
/// # Errors
/// A malformed value or a combination the kit refuses.
pub fn from_args(a: &[String]) -> Result<Vec<String>, String> {
    let get = |k: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str);
    let flag = |k: &str| -> Result<Option<bool>, String> {
        match get(k) {
            None => Ok(None),
            Some("on") => Ok(Some(true)),
            Some("off") => Ok(Some(false)),
            Some(v) => Err(format!("{k} on|off, not {v}")),
        }
    };
    T0_UTC.store(utc_now(), Ordering::Relaxed);
    let _ = ARGS_DIGEST.set(args_digest(a));
    let shard = flag("--shard")?.unwrap_or(false);
    let bfiles = flag("--batch-files")?.unwrap_or(false);
    set(SHARD, shard);
    set(BFILES, bfiles);
    set(PORTABLE, flag("--portable")?.unwrap_or(false) || shard || bfiles);
    set(STRICT, flag("--strict")?.unwrap_or(false) || shard || bfiles);
    let mut tools = BTreeMap::new();
    for t in ["zstd", "unzip", "gzip", "ps", "hostname"] {
        if let Some(p) = get(&format!("--tool-{t}")) {
            tools.insert(t.to_owned(), p.to_owned());
        }
    }
    let _ = TOOLS.set(tools);
    let mut notes = Vec::new();
    if let Some(p) = get("--batch-plan") {
        let pl = Plan::load(Path::new(p))?;
        notes.push(format!("E2 batch plan {p}: {} batches over {} unique lists, sha256 {}, list digest {}", pl.b.len(), pl.nl, pl.sha, pl.digest));
        let _ = PLAN.set(pl);
    }
    if let Some(p) = get("--write-plan") {
        let _ = WRITE_PLAN.set(PathBuf::from(p));
    }
    if let Some(v) = get("--lists-per-batch") {
        LPB.store(v.parse::<u64>().ok().filter(|&x| x > 0).ok_or_else(|| format!("--lists-per-batch {v}"))?, Ordering::Relaxed);
    }
    if let Some(s) = get("--batch-set") {
        let _ = BATCH_SET.set(parse_set(s)?);
    }
    if let Some(p) = get("--expect-inputs") {
        let _ = EXPECT_INPUTS.set(PathBuf::from(p));
    }
    let transport = a.iter().any(|x| x == "--cube-transport");
    let dm = a.iter().any(|x| x == "--day-major");
    let ck = get("--checkpoint").is_some();
    let one_seg = matches!(get("--segs"), Some("idx" | "stk"));
    let planned = PLAN.get().is_some();
    if (planned || WRITE_PLAN.get().is_some()) && !(dm && crate::sw33::any() && one_seg) {
        return Err("--batch-plan / --write-plan need --day-major, a v3.3 switch (e.g. --v33) and ONE segment (--segs idx|stk): a plan is one segment's batches".to_owned());
    }
    if shard {
        if !planned || BATCH_SET.get().is_none() || !ck {
            return Err("--shard on needs --batch-plan PLAN, --batch-set <ids> and --checkpoint DIR (E1)".to_owned());
        }
        if let (Some(pl), Some(set)) = (PLAN.get(), BATCH_SET.get())
            && let Some(&k) = set.iter().find(|&&k| k >= pl.b.len())
        {
            return Err(format!("--batch-set names batch {k}; the plan has {} (0..{})", pl.b.len(), pl.b.len().saturating_sub(1)));
        }
    }
    if bfiles && (!planned || !ck) {
        return Err("--batch-files on needs --batch-plan PLAN and --checkpoint DIR (E7)".to_owned());
    }
    if shard && bfiles {
        return Err("--shard on writes batch files already; --batch-files is for an ordinary run".to_owned());
    }
    if transport && !shard {
        return Err("--cube-transport (E9) is a shard's input form: it needs --shard on".to_owned());
    }
    TRANSPORT.store(transport, Ordering::Relaxed);
    if any() {
        notes.push(format!("v3.4 SHARD KIT: {}", words()));
    }
    Ok(notes)
}

/// E1: whether a shard walks batch `bk`: assigned to it (`--batch-set`) and its file not already written and verified (`done`).
#[must_use]
pub fn shard_walks(bk: usize, set: Option<&BTreeSet<usize>>, done: bool) -> bool {
    // MUTANT m_shard_skips_last_batch: the shard's last assigned batch never walked
    let last = cfg!(feature = "m_shard_skips_last_batch") && set.and_then(|s| s.iter().next_back()) == Some(&bk);
    set.is_some_and(|s| s.contains(&bk)) && !done && !last
}

/// `0,4,8`, `0-3,7` or `all` (every batch of the plan).
///
/// # Errors
/// A malformed set.
pub fn parse_set(s: &str) -> Result<BTreeSet<usize>, String> {
    let mut out = BTreeSet::new();
    if s == "all" {
        let n = PLAN.get().map_or(0, |p| p.b.len());
        return Ok((0..n).collect());
    }
    for part in s.split(',').filter(|x| !x.is_empty()) {
        if let Some((a, b)) = part.split_once('-') {
            let (a, b) = (a.parse::<usize>().map_err(|_| format!("--batch-set {s}"))?, b.parse::<usize>().map_err(|_| format!("--batch-set {s}"))?);
            if a > b {
                return Err(format!("--batch-set {s}: {a}-{b}"));
            }
            out.extend(a..=b);
        } else {
            out.insert(part.parse::<usize>().map_err(|_| format!("--batch-set {s}"))?);
        }
    }
    if out.is_empty() {
        return Err(format!("--batch-set {s}: empty"));
    }
    Ok(out)
}

// ------------------------------------------------------------------------------------------------ SHA-256 (E5: in-process, FIPS 180-4)

const K256: [u32; 64] = [
    0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4, 0xab1c_5ed5, 0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3, 0x72be_5d74, 0x80de_b1fe, 0x9bdc_06a7, 0xc19b_f174, 0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc, 0x2de9_2c6f, 0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da, 0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7, 0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967,
    0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc, 0x5338_0d13, 0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85, 0xa2bf_e8a1, 0xa81a_664b, 0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070, 0x19a4_c116, 0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3, 0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208, 0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7, 0xc671_78f2,
];

/// An incremental SHA-256 (no dependency; the FIPS 180-4 vectors are its named test).
#[derive(Clone, Debug)]
pub struct Sha256 {
    h: [u32; 8],
    buf: [u8; 64],
    nbuf: usize,
    len: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    /// A fresh hasher.
    #[must_use]
    pub const fn new() -> Self {
        Self { h: [0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a, 0x510e_527f, 0x9b05_688c, 0x1f83_d9ab, 0x5be0_cd19], buf: [0; 64], nbuf: 0, len: 0 }
    }

    fn block(h: &mut [u32; 8], b: &[u8]) {
        let mut w = [0_u32; 64];
        for (i, c) in b.chunks_exact(4).take(16).enumerate() {
            if let (Some(x), [a0, a1, a2, a3]) = (w.get_mut(i), c) {
                *x = u32::from_be_bytes([*a0, *a1, *a2, *a3]);
            }
        }
        for i in 16..64 {
            let (w15, w2, w16, w7) = (w.get(i - 15).copied().unwrap_or(0), w.get(i - 2).copied().unwrap_or(0), w.get(i - 16).copied().unwrap_or(0), w.get(i - 7).copied().unwrap_or(0));
            let s0 = w15.rotate_right(7) ^ w15.rotate_right(18) ^ (w15 >> 3);
            let s1 = w2.rotate_right(17) ^ w2.rotate_right(19) ^ (w2 >> 10);
            if let Some(x) = w.get_mut(i) {
                *x = w16.wrapping_add(s0).wrapping_add(w7).wrapping_add(s1);
            }
        }
        let [mut a, mut bb, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
        for (k, wi) in K256.iter().zip(w.iter()) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(*k).wrapping_add(*wi);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & bb) ^ (a & c) ^ (bb & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = bb;
            bb = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, bb, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }

    /// Adds bytes.
    pub fn update(&mut self, mut d: &[u8]) {
        self.len = self.len.wrapping_add(d.len() as u64);
        if self.nbuf > 0 {
            let take = (64 - self.nbuf).min(d.len());
            if let (Some(dst), Some(src)) = (self.buf.get_mut(self.nbuf..self.nbuf + take), d.get(..take)) {
                dst.copy_from_slice(src);
            }
            self.nbuf += take;
            d = d.get(take..).unwrap_or(&[]);
            if self.nbuf == 64 {
                let b = self.buf;
                Self::block(&mut self.h, &b);
                self.nbuf = 0;
            }
        }
        let mut it = d.chunks_exact(64);
        for b in &mut it {
            Self::block(&mut self.h, b);
        }
        let rest = it.remainder();
        if !rest.is_empty() {
            if let Some(dst) = self.buf.get_mut(..rest.len()) {
                dst.copy_from_slice(rest);
            }
            self.nbuf = rest.len();
        }
    }

    /// The digest.
    #[must_use]
    pub fn finish(mut self) -> [u8; 32] {
        let bits = self.len.wrapping_mul(8);
        self.update_pad(&[0x80]);
        while self.nbuf != 56 {
            self.update_pad(&[0]);
        }
        self.update_pad(&bits.to_be_bytes());
        let mut out = [0_u8; 32];
        for (o, x) in out.chunks_exact_mut(4).zip(self.h) {
            o.copy_from_slice(&x.to_be_bytes());
        }
        out
    }

    fn update_pad(&mut self, d: &[u8]) {
        let l = self.len;
        self.update(d);
        self.len = l;
    }

    /// The digest in lower-case hex (shasum's form).
    #[must_use]
    pub fn hex(self) -> String {
        hex(&self.finish())
    }
}

/// Lower-case hex of bytes.
#[must_use]
pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        let _ = write!(s, "{x:02x}");
    }
    s
}

/// sha256 of bytes, hex.
#[must_use]
pub fn sha256_bytes(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    h.hex()
}

/// sha256 of a file, read in 1 MiB pieces (None: unreadable).
#[must_use]
pub fn sha256_file(p: &Path) -> Option<String> {
    use std::io::Read as _;
    let mut f = std::fs::File::open(p).ok()?;
    let mut h = Sha256::new();
    let mut buf = vec![0_u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        h.update(buf.get(..n).unwrap_or(&[]));
    }
    Some(h.hex())
}

// ------------------------------------------------------------------------------------------------ E5: tools, clock, /proc, sorted reads

/// The path of external tool `t` (zstd, unzip, gzip, ps, hostname): a `--tool-<t>` flag; else, under PORTABLE, the environment
/// (`SF_ZSTD`, `SF_UNZIP`, ...) or the bare name (PATH lookup); else the v3.3 Mac path (unchanged: P0).
#[must_use]
pub fn tool(t: &str) -> String {
    #[cfg(test)]
    {
        if let Some(p) = TOOL_T.with(|m| m.borrow().get(t).cloned()) {
            return p;
        }
    }
    if let Some(p) = TOOLS.get().and_then(|m| m.get(t)) {
        return p.clone();
    }
    // MUTANT m_kit_tool_mac_path: the Mac path kept under PORTABLE (a Linux box without it fails)
    if on(PORTABLE) && !cfg!(feature = "m_kit_tool_mac_path") {
        if let Ok(p) = std::env::var(format!("SF_{}", t.to_ascii_uppercase()))
            && !p.is_empty()
        {
            return p;
        }
        return t.to_owned();
    }
    match t {
        "zstd" => "/opt/homebrew/bin/zstd",
        "unzip" => "/usr/bin/unzip",
        "gzip" => "/usr/bin/gzip",
        "ps" => "/bin/ps",
        "hostname" => "/bin/hostname",
        _ => t,
    }
    .to_owned()
}

/// Test hook: a tool path for this test thread only.
#[cfg(test)]
pub fn set_tool_t(t: &str, p: &str) {
    TOOL_T.with(|m| {
        m.borrow_mut().insert(t.to_owned(), p.to_owned());
    });
}

/// UTC seconds since 1970 (the system clock).
#[must_use]
pub fn utc_now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// `YYYY-MM-DD HH:MM:SS IST` of UTC second `t`: IST = UTC + 5 h 30 min (no daylight saving in India; no tzdata read).
#[must_use]
pub fn ist_label(t: u64) -> String {
    // MUTANT m_kit_ist_utc: the 5 h 30 min offset left out (labels in UTC called IST)
    let off = if cfg!(feature = "m_kit_ist_utc") { 0 } else { 19_800 };
    let s = i64::try_from(t).unwrap_or(0) + off;
    let (d, r) = (s.div_euclid(86_400), s.rem_euclid(86_400));
    let (y, m, dd) = crate::cal::civil(d);
    format!("{y:04}-{m:02}-{dd:02} {:02}:{:02}:{:02} IST", r / 3_600, (r / 60) % 60, r % 60)
}

/// UTC label `YYYY-MM-DDTHH:MM:SSZ` (provenance).
#[must_use]
pub fn utc_label(t: u64) -> String {
    let s = i64::try_from(t).unwrap_or(0);
    let (d, r) = (s.div_euclid(86_400), s.rem_euclid(86_400));
    let (y, m, dd) = crate::cal::civil(d);
    format!("{y:04}-{m:02}-{dd:02}T{:02}:{:02}:{:02}Z", r / 3_600, (r / 60) % 60, r % 60)
}

/// The run's IST label: under PORTABLE from the system clock, else None (the caller keeps v3.3's `/bin/date` call: P0).
#[must_use]
pub fn now_ist() -> Option<String> {
    on(PORTABLE).then(|| ist_label(utc_now()))
}

/// `VmRSS` (KB) from a /proc/<pid>/status text.
#[must_use]
pub fn proc_status_rss_kb(text: &str) -> Option<u64> {
    text.lines().find_map(|l| l.strip_prefix("VmRSS:")).and_then(|r| r.split_whitespace().next()).and_then(|x| x.parse().ok())
}

/// utime + stime (clock ticks) from a /proc/<pid>/stat text (the fields after the parenthesised command, which may hold spaces).
#[must_use]
pub fn proc_stat_ticks(text: &str) -> Option<u64> {
    let rest = text.rsplit_once(')')?.1;
    let f: Vec<&str> = rest.split_whitespace().collect();
    // after ")": state is field 3 of the full line; utime = field 14, stime = field 15 (1-based) -> rest indexes 11 and 12
    let u: u64 = f.get(11)?.parse().ok()?;
    let s: u64 = f.get(12)?.parse().ok()?;
    Some(u + s)
}

/// The process RSS in KB: /proc under PORTABLE when it exists, else `ps -o rss=` (v3.3's reading).
#[must_use]
pub fn rss_kb() -> Option<u64> {
    if on(PORTABLE)
        && let Ok(t) = std::fs::read_to_string("/proc/self/status")
    {
        return proc_status_rss_kb(&t);
    }
    let pid = std::process::id().to_string();
    let o = std::process::Command::new(tool("ps")).args(["-o", "rss=", "-p", &pid]).output().ok()?;
    String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().ok()
}

/// The process CPU seconds: /proc under PORTABLE when it exists (100 ticks a second, the Linux USER_HZ), else None (the caller reads ps).
#[must_use]
pub fn proc_cpu_s() -> Option<f64> {
    if !on(PORTABLE) {
        return None;
    }
    let t = std::fs::read_to_string("/proc/self/stat").ok()?;
    proc_stat_ticks(&t).map(|x| x as f64 / 100.0)
}

/// The entries of a directory sorted by file name (E5: APFS and ext4 list a directory in different orders).
///
/// # Errors
/// An unreadable directory.
pub fn sorted_dir(dir: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let rd = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut v: Vec<(String, PathBuf)> = rd.flatten().map(|e| (e.file_name().to_string_lossy().into_owned(), e.path())).collect();
    // MUTANT m_kit_read_dir_unsorted: the directory's own order kept
    if !cfg!(feature = "m_kit_read_dir_unsorted") {
        v.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    }
    Ok(v)
}

/// F4: records a failure STRICT makes fatal (returns whether STRICT is on, so the caller may stop early).
pub fn strict_fail(what: &str, msg: &str) -> bool {
    if !on(STRICT) {
        return false;
    }
    if let Ok(mut v) = STRICT_ERRS.lock() {
        if v.len() < 20 {
            v.push(format!("{what}: {msg}"));
        } else if v.len() == 20 {
            v.push("... (more)".to_owned());
        }
    }
    true
}

/// F4 + E8: Err when STRICT recorded a failure or a read left the segment slice (the run is then void: no batch file is written).
///
/// # Errors
/// The failures, by name.
pub fn strict_check(at: &str) -> Result<(), String> {
    let v = STRICT_ERRS.lock().map(|v| v.clone()).unwrap_or_default();
    if !v.is_empty() {
        return Err(format!("STRICT (F4) at {at}: {} failure(s) a lenient run would have swallowed: {}", v.len(), v.join(" | ")));
    }
    let n = SLICE_VIOLATIONS.load(Ordering::Relaxed);
    if n > 0 {
        return Err(format!("SLICE GUARD (E8 / critic F1) at {at}: {n} read(s) of a contract outside the loaded segment slice: the run is void"));
    }
    Ok(())
}

/// Test hook: the STRICT failures recorded so far.
#[cfg(test)]
pub fn strict_errs() -> Vec<String> {
    STRICT_ERRS.lock().map(|v| v.clone()).unwrap_or_default()
}

/// Test hook: clears the STRICT failures.
#[cfg(test)]
pub fn strict_reset() {
    if let Ok(mut v) = STRICT_ERRS.lock() {
        v.clear();
    }
}

/// F4: the C1 probes at start: the tools the run will call answer, and the in-process sha256 reproduces its vector.
///
/// # Errors
/// A tool that does not run, or a wrong sha256.
pub fn probe_tools(need_zstd: bool, need_unzip: bool) -> Result<String, String> {
    let mut w = Vec::new();
    if sha256_bytes(b"abc") != "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" {
        return Err("sha256 self-test failed".to_owned());
    }
    w.push("sha256 self-test OK".to_owned());
    for (need, t, arg) in [(need_zstd, "zstd", "-V"), (need_unzip, "unzip", "-v")] {
        if !need {
            continue;
        }
        let p = tool(t);
        let o = std::process::Command::new(&p).arg(arg).output().map_err(|e| format!("tool {t} ({p}) does not run: {e}"))?;
        if !o.status.success() && !(t == "unzip" && !o.stdout.is_empty()) {
            return Err(format!("tool {t} ({p}) {arg} failed: {}", o.status));
        }
        w.push(format!("{t} = {p}"));
    }
    Ok(w.join("; "))
}

// ------------------------------------------------------------------------------------------------ E2: the explicit batch plan

/// E2: an explicit batch plan of ONE segment: batch k = the unique lists at batch positions [p0, p1) of the run's batch order, with their
/// ids (so another target's order can never be taken for it), its cost (account-event pairs, E11) and the set-up's list digest (E6).
#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub path: String,
    pub sha: String,
    pub seg: u8,
    pub days: Vec<String>,
    pub nset: usize,
    pub nl: usize,
    pub digest: String,
    /// The plan writer's argument digest ([`args_digest`]): every shard must run with the same result-bearing arguments.
    pub args: String,
    pub order: String,
    pub per_list: u64,
    pub cut: String,
    /// (p0, p1, cost = account-event pairs, list ids at p0..p1)
    pub b: Vec<(usize, usize, u128, Vec<u32>)>,
}

impl Plan {
    /// The plan's text (written once; its sha256 joins the fingerprint).
    #[must_use]
    pub fn text(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "SPIKEFADE-PLAN\t1");
        let _ = writeln!(s, "SEG\t{}", self.seg);
        let _ = writeln!(s, "DAYS\t{}\t{}", self.days.len(), self.days.join(","));
        let _ = writeln!(s, "SETTINGS\t{}", self.nset);
        let _ = writeln!(s, "LISTS\t{}", self.nl);
        let _ = writeln!(s, "DIGEST\t{}", self.digest);
        let _ = writeln!(s, "ARGS\t{}", self.args);
        let _ = writeln!(s, "ORDER\t{}", self.order);
        let _ = writeln!(s, "PERLIST\t{}", self.per_list);
        let _ = writeln!(s, "CUT\t{}", self.cut);
        let _ = writeln!(s, "BATCHES\t{}", self.b.len());
        for (k, (p0, p1, cost, ids)) in self.b.iter().enumerate() {
            let _ = writeln!(s, "B\t{k}\t{p0}\t{p1}\t{cost}\t{}", ids.iter().map(u32::to_string).collect::<Vec<_>>().join(","));
        }
        let _ = writeln!(s, "END");
        s
    }

    /// Reads and validates a plan (every tag known; batches contiguous from 0 to the list count; ids one per position).
    ///
    /// # Errors
    /// An unreadable, malformed, truncated or inconsistent plan.
    pub fn load(p: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(p).map_err(|e| format!("--batch-plan {}: {e}", p.display()))?;
        let mut pl = Self::parse(&text).map_err(|e| format!("--batch-plan {}: {e}", p.display()))?;
        pl.path = p.display().to_string();
        Ok(pl)
    }

    /// Parses a plan's text (see [`Plan::load`]).
    ///
    /// # Errors
    /// As [`Plan::load`].
    pub fn parse(text: &str) -> Result<Self, String> {
        if !text.ends_with("END\n") {
            return Err("truncated plan (no END line)".to_owned());
        }
        let mut pl = Self { sha: sha256_bytes(text.as_bytes()), ..Self::default() };
        let mut nb: Option<usize> = None;
        let num = |x: Option<&str>, what: &str| -> Result<usize, String> { x.ok_or_else(|| format!("plan: short {what}"))?.parse::<usize>().map_err(|_| format!("plan: bad {what}")) };
        for (i, l) in text.lines().enumerate() {
            let mut f = l.split('\t');
            let tag = f.next().unwrap_or("");
            match tag {
                "SPIKEFADE-PLAN" if i == 0 => {
                    if f.next() != Some("1") {
                        return Err("plan version is not 1".to_owned());
                    }
                }
                "SEG" => pl.seg = u8::try_from(num(f.next(), "SEG")?).map_err(|_| "plan: bad SEG")?,
                "DAYS" => {
                    let n = num(f.next(), "DAYS")?;
                    pl.days = f.next().unwrap_or("").split(',').filter(|x| !x.is_empty()).map(str::to_owned).collect();
                    if pl.days.len() != n {
                        return Err(format!("plan: DAYS says {n}, lists {}", pl.days.len()));
                    }
                }
                "SETTINGS" => pl.nset = num(f.next(), "SETTINGS")?,
                "LISTS" => pl.nl = num(f.next(), "LISTS")?,
                "DIGEST" => pl.digest = f.next().unwrap_or("").to_owned(),
                "ARGS" => pl.args = f.next().unwrap_or("").to_owned(),
                "ORDER" => pl.order = f.next().unwrap_or("").to_owned(),
                "PERLIST" => pl.per_list = num(f.next(), "PERLIST")? as u64,
                "CUT" => pl.cut = f.next().unwrap_or("").to_owned(),
                "BATCHES" => nb = Some(num(f.next(), "BATCHES")?),
                "B" => {
                    let k = num(f.next(), "B k")?;
                    if k != pl.b.len() {
                        return Err(format!("plan: batch {k} out of order (expected {})", pl.b.len()));
                    }
                    let (p0, p1) = (num(f.next(), "B p0")?, num(f.next(), "B p1")?);
                    let cost: u128 = f.next().ok_or("plan: short B cost")?.parse().map_err(|_| "plan: bad B cost")?;
                    let ids: Vec<u32> = f.next().unwrap_or("").split(',').filter(|x| !x.is_empty()).map(|x| x.parse::<u32>().map_err(|_| "plan: bad list id".to_owned())).collect::<Result<_, _>>()?;
                    let want0 = pl.b.last().map_or(0, |x| x.1);
                    if p0 != want0 || p1 <= p0 || ids.len() != p1 - p0 {
                        return Err(format!("plan: batch {k} [{p0}, {p1}) with {} ids is not contiguous / whole (expected to start at {want0})", ids.len()));
                    }
                    pl.b.push((p0, p1, cost, ids));
                }
                "END" => {}
                t => return Err(format!("plan: unknown tag {t:?} at line {}", i + 1)),
            }
        }
        if nb != Some(pl.b.len()) {
            return Err(format!("plan: BATCHES {nb:?} but {} batch rows", pl.b.len()));
        }
        if pl.b.last().map_or(0, |x| x.1) != pl.nl {
            return Err(format!("plan: the batches end at {} but the run has {} unique lists (a gap)", pl.b.last().map_or(0, |x| x.1), pl.nl));
        }
        if pl.digest.len() != 64 {
            return Err("plan: no list digest".to_owned());
        }
        if pl.args.len() != 64 {
            return Err("plan: no argument digest".to_owned());
        }
        Ok(pl)
    }

    /// Checks the plan against the run's own set-up (segment, days, settings, lists, digest, order, ids at every position).
    ///
    /// # Errors
    /// Any difference, by name (the run is refused before its first batch).
    pub fn check(&self, seg: u8, days: &[String], nset: usize, digest: &str, args: &str, ord: &[usize], ordered: bool, per_list: u64) -> Result<(), String> {
        // MUTANT m_kit_args_digest_off: the argument digest not compared (a shard with another flag or grid walks and is caught at best at the merge)
        if self.args != args && !cfg!(feature = "m_kit_args_digest_off") {
            return Err(format!("batch plan {}: written with argument digest {} but this run's is {args} (another result-bearing flag, value, set or grid): refused before the first batch", self.path, self.args));
        }
        let order = if ordered { "config-sorted" } else { "creation" };
        if self.seg != seg || self.days != days || self.nset != nset || self.nl != ord.len() || self.order != order || self.per_list != per_list {
            return Err(format!("batch plan {}: written for segment {} / {} days / {} settings / {} lists / order {} / {} accounts a list; this run has segment {seg} / {} days / {nset} settings / {} lists / order {order} / {per_list}: refused", self.path, self.seg, self.days.len(), self.nset, self.nl, self.order, self.per_list, days.len(), ord.len()));
        }
        // MUTANT m_kit_fp_counts_only: the list digest not compared (a set-up that built OTHER lists of the same count is accepted)
        if self.digest != digest && !cfg!(feature = "m_kit_fp_counts_only") {
            return Err(format!("batch plan {}: list digest {} but this run's set-up gives {digest} (other events, lists or settings): refused before the first batch (E6 / critic F5)", self.path, self.digest));
        }
        for (k, (p0, p1, _, ids)) in self.b.iter().enumerate() {
            let got = ord.get(*p0..*p1).unwrap_or(&[]);
            if got.len() != ids.len() || got.iter().zip(ids).any(|(&a, &b)| a != b as usize) {
                return Err(format!("batch plan {}: batch {k} lists differ from this run's batch order at positions {p0}..{p1}: refused", self.path));
            }
        }
        Ok(())
    }

    /// The batch starting at position `b0`: (k, its end).
    ///
    /// # Errors
    /// A position that does not start a batch (a checkpoint of another cut).
    pub fn batch_at(&self, b0: usize) -> Result<(usize, usize), String> {
        let k = self.b.partition_point(|x| x.0 < b0);
        match self.b.get(k) {
            Some(x) if x.0 == b0 => Ok((k, x.1)),
            _ => Err(format!("batch plan {}: position {b0} does not start a batch (a checkpoint of another cut): refused", self.path)),
        }
    }
}

/// E2 + E6: the fingerprint words of a planned run (empty without a plan: v3.3's fingerprint, P0).
#[must_use]
pub fn fp_part() -> String {
    match (PLAN.get(), DIGEST.get()) {
        (Some(p), Some(d)) => format!("; batch plan sha256 {}; list digest {d}; args digest {}", p.sha, p.args),
        (Some(p), None) => format!("; batch plan sha256 {}; list digest (not computed); args digest {}", p.sha, p.args),
        _ => String::new(),
    }
}

// ------------------------------------------------------------------------------------------------ E6 (critic F5): the list digest

/// One parallel digest pass: `n` items, `f(i, hasher)` feeds item i; items are cut into fixed chunks of `chunk`, each chunk hashed alone
/// (in parallel), then the chunk digests are hashed in order (independent of the thread count).
fn par_digest(n: usize, chunk: usize, threads: usize, f: &(dyn Fn(usize, &mut Sha256) + Sync)) -> [u8; 32] {
    let nchunk = n.div_ceil(chunk.max(1));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out: Mutex<Vec<(usize, [u8; 32])>> = Mutex::new(Vec::with_capacity(nchunk));
    std::thread::scope(|s| {
        for _ in 0..threads.clamp(1, 64) {
            s.spawn(|| {
                let mut mine = Vec::new();
                loop {
                    let c = next.fetch_add(1, Ordering::Relaxed);
                    if c >= nchunk {
                        break;
                    }
                    let mut h = Sha256::new();
                    for i in c * chunk..((c + 1) * chunk).min(n) {
                        f(i, &mut h);
                    }
                    mine.push((c, h.finish()));
                }
                if let Ok(mut o) = out.lock() {
                    o.extend(mine);
                }
            });
        }
    });
    let mut v = out.into_inner().unwrap_or_default();
    v.sort_by_key(|x| x.0);
    let mut h = Sha256::new();
    h.update(&(n as u64).to_le_bytes());
    for (_, d) in v {
        h.update(&d);
    }
    h.finish()
}

/// E6 / critic F5: the digest of a segment's whole set-up: the run's days, the selected settings (sets + menu sha256, the selection, every
/// compiled setting, the tie keys and roles), the screener levels, EVERY event's identity and attribute record (day, second, contract, R1 key,
/// table, liquidity readings, timeframe mask and windows, readings, 60-s volume, margin at T), every per-config list's members, and every
/// unique list's definition (representative pair, liquidity value, timeframe key, every (config, liquidity) pair) AND members. Two set-ups
/// with equal digests walk the same accounts on the same events.
#[must_use]
pub fn list_digest(sr: &crate::report3::Seg3, days: &[String], csets: &[crate::kern::CSet], sel: &[usize], sets_sha: &str, menu_sha: &str, threads: usize) -> String {
    let mut h = Sha256::new();
    h.update(b"SF-LIST-DIGEST v1\n");
    for d in days {
        h.update(d.as_bytes());
        h.update(b",");
    }
    h.update(format!("\nsegdays {:?}\npieces {:?}\n", sr.days, sr.piece_of_day).as_bytes());
    h.update(format!("settings sets {sets_sha} menu {menu_sha} sel {sel:?}\nties {:?}\nsearch {:?}\n", sr.ties, sr.search).as_bytes());
    for cs in csets {
        h.update(format!("{cs:?}\n").as_bytes());
    }
    h.update(format!("levels {:?}\nlevels per day {:?}\n", sr.lv, sr.lvd).as_bytes());
    // every event's record (its stage A table is still the set-up's: no stage A has run when the digest is taken)
    // MUTANT m_kit_digest_counts_only: the events and lists left out (a digest of counts alone)
    let counts_only = cfg!(feature = "m_kit_digest_counts_only");
    h.update(format!("events {}\n", sr.evs.len()).as_bytes());
    if !counts_only {
        let ev = par_digest(sr.evs.len(), 4_096, threads, &|i, hh| {
            hh.update(format!("{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}\n", sr.evs.get(i), sr.liq.get(i), sr.tfmask.get(i), sr.rd.get(i), sr.tfw.get(i), sr.vol60.get(i), sr.mx.get(i)).as_bytes());
        });
        h.update(&ev);
    }
    h.update(format!("config lists {}\n", sr.lists.len()).as_bytes());
    if !counts_only {
        let cl = par_digest(sr.lists.len(), 64, threads, &|i, hh| {
            if let Some(l) = sr.lists.get(i) {
                hh.update(&(l.len() as u64).to_le_bytes());
                for e in l {
                    hh.update(&e.to_le_bytes());
                }
            }
        });
        h.update(&cl);
    }
    h.update(format!("unique lists {}\n", sr.uls.len()).as_bytes());
    let ul = par_digest(sr.uls.len(), 16, threads, &|u, hh| {
        hh.update(format!("{:?}|{:?}\n", sr.uls.get(u), sr.ul_cfgs.get(u)).as_bytes());
        if !counts_only && let Some(l) = sr.ul(u) {
            hh.update(&(l.len() as u64).to_le_bytes());
            for e in l.iter() {
                hh.update(&e.to_le_bytes());
            }
        }
    });
    h.update(&ul);
    h.hex()
}

// ------------------------------------------------------------------------------------------------ E4 + F4: the inputs manifest

/// The roles of the command line's table inputs (`--freeze` and the directories may repeat; a `@date` suffix names the file's date).
const TABLE_ROLES: [&str; 13] = ["--var", "--holidays", "--holidays2", "--menu", "--sets", "--margin-futures", "--special-sessions", "--tbill", "--idx-elm", "--freeze", "--ban-dir", "--bhav-dir", "--margin-manifest"];

fn table_lines(a: &[String], threads: usize) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut jobs: Vec<(String, PathBuf)> = Vec::new();
    for (i, x) in a.iter().enumerate() {
        let role = x.as_str();
        if !(TABLE_ROLES.contains(&role) || role == "--margin-extra") {
            continue;
        }
        let Some(v) = a.get(i + 1) else { continue };
        let path = v.split_once('@').map_or(v.as_str(), |x| x.0);
        let p = PathBuf::from(path);
        if role == "--margin-manifest" || role == "--margin-extra" {
            // the manifest rows (session, label, kind, effective time, sha256): the table CONTENTS, never the paths
            let t = std::fs::read_to_string(&p).map_err(|e| format!("inputs: {role} {path}: {e}"))?;
            for l in t.lines().skip(1) {
                let f: Vec<&str> = l.split('\t').collect();
                out.push(format!("table\t{role}\t{}\t{}\t{}\t{}\t{}", f.first().unwrap_or(&""), f.get(1).unwrap_or(&""), f.get(2).unwrap_or(&""), f.get(3).unwrap_or(&""), f.get(5).unwrap_or(&"")));
            }
            continue;
        }
        if p.is_dir() {
            for (name, ep) in sorted_dir(&p)? {
                if ep.is_file() {
                    jobs.push((format!("{role}\t{name}"), ep));
                }
            }
        } else {
            jobs.push((format!("{role}\t-"), p));
        }
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let res: Mutex<Vec<(usize, Result<String, String>)>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads.clamp(1, 16) {
            s.spawn(|| loop {
                let j = next.fetch_add(1, Ordering::Relaxed);
                let Some((what, p)) = jobs.get(j) else { break };
                let r = sha256_file(p).map(|h| format!("table\t{what}\t{h}")).ok_or_else(|| format!("inputs: {} unreadable", p.display()));
                if let Ok(mut v) = res.lock() {
                    v.push((j, r));
                }
            });
        }
    });
    let mut v = res.into_inner().unwrap_or_default();
    v.sort_by_key(|x| x.0);
    for (_, r) in v {
        out.push(r?);
    }
    Ok(out)
}

/// E4 + F4: the run's inputs manifest: per run day, the presence (and absence) of each cube root's files and the sha256 of the day's
/// SEGMENT SLICE of each root (the same digest whether the shard reads the full cube, a slice or a transport file: E8 / E9), then every table
/// input by content (paths never enter). Its sha256 goes into every batch file's provenance.
///
/// # Errors
/// An unreadable input or a cube that cannot be sliced.
pub fn inputs_manifest(a: &[String], cdir: &Path, days: &[String], seg: u8, threads: usize) -> Result<String, String> {
    let extra = crate::cube::EXTRA.get().cloned();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let res: Mutex<Vec<(usize, Result<String, String>)>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads.clamp(1, 16) {
            s.spawn(|| loop {
                let j = next.fetch_add(1, Ordering::Relaxed);
                let Some(d) = days.get(j) else { break };
                let r = (|| -> Result<String, String> {
                    let (mb, ms, md) = crate::slice::root_day_digest(cdir, d, seg)?;
                    let x = match &extra {
                        Some(x) => {
                            let (eb, es, ed) = crate::slice::root_day_digest(x, d, seg)?;
                            format!("{}\t{}\t{ed}", u8::from(eb), u8::from(es))
                        }
                        None => "-\t-\t-".to_owned(),
                    };
                    Ok(format!("day\t{d}\t{}\t{}\t{md}\t{x}", u8::from(mb), u8::from(ms)))
                })();
                if let Ok(mut v) = res.lock() {
                    v.push((j, r));
                }
            });
        }
    });
    let mut v = res.into_inner().unwrap_or_default();
    v.sort_by_key(|x| x.0);
    let mut s = String::from("SPIKEFADE-INPUTS\t1\n");
    let _ = writeln!(s, "segment\t{seg}\textra-root\t{}", u8::from(extra.is_some()));
    let _ = writeln!(s, "#day\tname\tmain_bin\tmain_sec\tmain_segment_slice_sha256\textra_bin\textra_sec\textra_segment_slice_sha256");
    for (_, r) in v {
        let _ = writeln!(s, "{}", r?);
    }
    for l in table_lines(a, threads)? {
        let _ = writeln!(s, "{l}");
    }
    let _ = writeln!(s, "END");
    Ok(s)
}

/// F4: the computed inputs manifest against the expected one (presence and absence of every day's files in both roots, every digest).
///
/// # Errors
/// The first differences, by line.
pub fn check_inputs(got: &str, want_path: &Path) -> Result<(), String> {
    let want = std::fs::read_to_string(want_path).map_err(|e| format!("--expect-inputs {}: {e}", want_path.display()))?;
    if want == got {
        return Ok(());
    }
    let (g, w): (Vec<&str>, Vec<&str>) = (got.lines().collect(), want.lines().collect());
    let mut diffs = Vec::new();
    for i in 0..g.len().max(w.len()) {
        if g.get(i) != w.get(i) {
            diffs.push(format!("line {}: expected {:?}, this machine {:?}", i + 1, w.get(i).copied().unwrap_or("(none)"), g.get(i).copied().unwrap_or("(none)")));
            if diffs.len() >= 4 {
                break;
            }
        }
    }
    Err(format!("inputs differ from --expect-inputs {} (F4: day sets / contents): {}: refused", want_path.display(), diffs.join("; ")))
}

// ------------------------------------------------------------------------------------------------ E1 + F2 + E4: the batch file

/// A window row's pick in the checkpoint form.
pub type WinBest = Option<((i64, i64, i64), Sum, crate::wf::Fig, crate::wf::Fig)>;

/// One finished batch's OWN contributions (E1), in the canonical form of critic F2, with its provenance (E4, outside the hashed body).
#[derive(Clone, Debug, Default)]
pub struct Bck {
    pub fp: String,
    pub k: usize,
    pub nb: usize,
    pub p0: usize,
    pub p1: usize,
    pub nl: usize,
    pub tie52: bool,
    pub splits18: bool,
    pub steps: u64,
    pub accts: u64,
    pub nbk: [(u64, u64); 4],
    /// The batch's D2-1 safety counters (critic 2.5 / F16): replayed account-days, fallen back (R < R*), class accounts ghost-walked, exits
    /// at or before their booking second, class-days, list-days in classes of >= 2, list-days.
    pub safe: [u64; 7],
    /// Per group: (K, per view: the kept entries BEST FIRST, the `Keyed` total order).
    pub tops: Vec<(usize, Vec<Vec<((i64, i64, i64), Sum)>>)>,
    pub win: Vec<(u64, u64, u64, WinBest)>,
    pub flagged: Vec<crate::report3::Flag>,
    /// S18: (accounts read, accounts offered) of the batch.
    pub s18: (u64, u64),
    /// Folds kept per split (the shape every S18B index is checked against).
    pub s18shape: Vec<usize>,
    pub s18b: Vec<(usize, usize, crate::split18::Best)>,
    /// critic F7a: (accounts, sha256 of every account's summary line in the batch's finalize order).
    pub acct: (u64, String),
    pub body_sha: String,
    /// MEASURED timing (outside the body): the split figures' nanoseconds and their samples.
    pub ns_sum: u64,
    pub s18n: Vec<u32>,
    pub prov: Vec<(String, String)>,
}

impl Bck {
    /// The canonical body (critic F2): every line the merge reads, timing removed, the kept entries best first, the flagged rows in the
    /// batch's push order. Its sha256 is the batch's identity (duplicates compare it).
    #[must_use]
    pub fn body(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "SPIKEFADE-BCK\t1\t{}", self.fp);
        let _ = writeln!(s, "BATCH\t{}\t{}\t{}\t{}\t{}", self.k, self.nb, self.p0, self.p1, self.nl);
        let _ = writeln!(s, "MERGE-SW\ttie52={}\tsplits18={}", u8::from(self.tie52), u8::from(self.splits18));
        let _ = writeln!(s, "SUMS\t{}\t{}", self.steps, self.accts);
        let _ = writeln!(s, "NBK\t{}", self.nbk.iter().map(|x| format!("{},{}", x.0, x.1)).collect::<Vec<_>>().join("\t"));
        let _ = writeln!(s, "SAFE\t{}", self.safe.iter().map(u64::to_string).collect::<Vec<_>>().join("\t"));
        for (g, (k, views)) in self.tops.iter().enumerate() {
            let _ = writeln!(s, "TOPS\t{g}\t{k}\t{}", views.len());
            for (v, list) in views.iter().enumerate() {
                for (key, sm) in list {
                    let _ = writeln!(s, "T\t{g}\t{v}\t{}\t{}\t{}\t{}", key.0, key.1, key.2, sum_line(sm));
                }
            }
        }
        for (r, w) in self.win.iter().enumerate() {
            let _ = writeln!(s, "W\t{r}\t{}\t{}\t{}", w.0, w.1, w.2);
            if let Some((key, sm, lf, tf)) = &w.3 {
                let _ = writeln!(s, "WB\t{r}\t{}\t{}\t{}\t{}", key.0, key.1, key.2, sum_line(sm));
                let _ = writeln!(s, "WL\t{r}\t{}", fig_line(lf));
                let _ = writeln!(s, "WT\t{r}\t{}", fig_line(tf));
            }
        }
        for f in &self.flagged {
            let _ = writeln!(s, "F\t{}\t{}\t{}\t{}\t{}\t{}", f.row, f.list, f.si, f.l, f.ni, sum_line(&f.sm));
        }
        let _ = writeln!(s, "S18\t{}\t{}\t0", self.s18.0, self.s18.1);
        let _ = writeln!(s, "S18SHAPE\t{}", self.s18shape.iter().map(usize::to_string).collect::<Vec<_>>().join(","));
        for (si, fi, b) in &self.s18b {
            let _ = writeln!(s, "{}", crate::split18::s18b_line(*si, *fi, b));
        }
        let _ = writeln!(s, "ACCT\t{}\t{}", self.acct.0, self.acct.1);
        s
    }

    /// The whole file: the body, its sha256, the timing and the provenance, END.
    #[must_use]
    pub fn text(&self) -> String {
        let body = self.body();
        let mut s = body.clone();
        let _ = writeln!(s, "BODY\t{}", sha256_bytes(body.as_bytes()));
        let _ = writeln!(s, "TIMING\tns_sum={}\ts18n={}", self.ns_sum, self.s18n.iter().map(u32::to_string).collect::<Vec<_>>().join(","));
        let _ = writeln!(s, "PROV\t{}", self.prov.iter().map(|(k, v)| format!("{k}={}", v.replace(['\t', '\n'], " "))).collect::<Vec<_>>().join("\t"));
        let _ = writeln!(s, "END");
        s
    }

    /// A provenance value.
    #[must_use]
    pub fn prov_of(&self, k: &str) -> Option<&str> {
        self.prov.iter().find(|x| x.0 == k).map(|x| x.1.as_str())
    }

    /// Parses a batch file with an EXHAUSTIVE tag match (critic F3): an unknown tag, an S18 line type without a merge law, an index outside
    /// the file's own shape (group, view, row, split, fold), a missing END or a body whose sha256 differs is REFUSED by name.
    ///
    /// # Errors
    /// Any of the above.
    pub fn parse(text: &str) -> Result<Self, String> {
        if !text.ends_with("END\n") {
            return Err("truncated batch file (no END line)".to_owned());
        }
        let mut b = Self::default();
        let mut body = String::new();
        let mut in_body = true;
        let mut wb: BTreeMap<usize, ((i64, i64, i64), Sum)> = BTreeMap::new();
        let mut wl: BTreeMap<usize, crate::wf::Fig> = BTreeMap::new();
        let mut wt: BTreeMap<usize, crate::wf::Fig> = BTreeMap::new();
        let mut seen_s18 = false;
        let num = |x: Option<&str>, what: &str| -> Result<u64, String> { x.ok_or_else(|| format!("short {what} line"))?.parse::<u64>().map_err(|_| format!("bad number in {what}")) };
        let us = |x: Option<&str>, what: &str| -> Result<usize, String> { usize::try_from(num(x, what)?).map_err(|_| format!("bad index in {what}")) };
        for (i, l) in text.lines().enumerate() {
            let (tag, rest) = l.split_once('\t').unwrap_or((l, ""));
            let mut f = rest.split('\t');
            if in_body && tag != "BODY" {
                body.push_str(l);
                body.push('\n');
            }
            match tag {
                "SPIKEFADE-BCK" if i == 0 => {
                    let (v, fp) = rest.split_once('\t').ok_or("short SPIKEFADE-BCK line")?;
                    if v != "1" {
                        return Err(format!("batch file version {v}"));
                    }
                    b.fp = fp.to_owned();
                }
                "BATCH" => {
                    b.k = us(f.next(), "BATCH")?;
                    b.nb = us(f.next(), "BATCH")?;
                    b.p0 = us(f.next(), "BATCH")?;
                    b.p1 = us(f.next(), "BATCH")?;
                    b.nl = us(f.next(), "BATCH")?;
                    if b.k >= b.nb || b.p1 <= b.p0 || b.p1 > b.nl {
                        return Err(format!("BATCH {} of {} at [{}, {}) of {}: inconsistent", b.k, b.nb, b.p0, b.p1, b.nl));
                    }
                }
                "MERGE-SW" => {
                    for x in f {
                        match x {
                            "tie52=1" => b.tie52 = true,
                            "tie52=0" => b.tie52 = false,
                            "splits18=1" => b.splits18 = true,
                            "splits18=0" => b.splits18 = false,
                            o => return Err(format!("MERGE-SW: unknown word {o:?}")),
                        }
                    }
                }
                "SUMS" => {
                    b.steps = num(f.next(), "SUMS")?;
                    b.accts = num(f.next(), "SUMS")?;
                }
                "NBK" => {
                    let v: Vec<&str> = rest.split('\t').collect();
                    if v.len() != 4 {
                        return Err("NBK needs 4 slot counts".to_owned());
                    }
                    for (x, y) in b.nbk.iter_mut().zip(v) {
                        let (a1, a2) = y.split_once(',').ok_or("NBK")?;
                        *x = (a1.parse().map_err(|_| "NBK")?, a2.parse().map_err(|_| "NBK")?);
                    }
                }
                "SAFE" => {
                    let v: Vec<u64> = rest.split('\t').map(|x| x.parse::<u64>().map_err(|_| "SAFE".to_owned())).collect::<Result<_, _>>()?;
                    if v.len() != 7 {
                        return Err("SAFE needs 7 counters".to_owned());
                    }
                    for (x, y) in b.safe.iter_mut().zip(v) {
                        *x = y;
                    }
                }
                "TOPS" => {
                    let g = us(f.next(), "TOPS")?;
                    if g != b.tops.len() {
                        return Err(format!("TOPS {g} out of order"));
                    }
                    let k = us(f.next(), "TOPS")?;
                    let nv = us(f.next(), "TOPS")?;
                    b.tops.push((k, vec![Vec::new(); nv]));
                }
                "T" => {
                    let g = us(f.next(), "T")?;
                    let v = us(f.next(), "T")?;
                    let key = (num_i(f.next())?, num_i(f.next())?, num_i(f.next())?);
                    let sm = sum_of_line(&f.collect::<Vec<_>>().join("\t"))?;
                    let slot = b.tops.get_mut(g).and_then(|t| t.1.get_mut(v)).ok_or_else(|| format!("T line: group {g} / view {v} outside the file's shape: refused"))?;
                    slot.push((key, sm));
                }
                "W" => {
                    let r = us(f.next(), "W")?;
                    if r != b.win.len() {
                        return Err(format!("W row {r} out of order"));
                    }
                    b.win.push((num(f.next(), "W")?, num(f.next(), "W")?, num(f.next(), "W")?, None));
                }
                "WB" => {
                    let r = us(f.next(), "WB")?;
                    let key = (num_i(f.next())?, num_i(f.next())?, num_i(f.next())?);
                    let sm = sum_of_line(&f.collect::<Vec<_>>().join("\t"))?;
                    if r >= b.win.len() || wb.insert(r, (key, sm)).is_some() {
                        return Err(format!("WB row {r} outside the W rows or repeated: refused"));
                    }
                }
                "WL" | "WT" => {
                    let r = us(f.next(), tag)?;
                    let fg = fig_of_line(&f.collect::<Vec<_>>().join("\t"))?;
                    let m = if tag == "WL" { &mut wl } else { &mut wt };
                    if r >= b.win.len() || m.insert(r, fg).is_some() {
                        return Err(format!("{tag} row {r} outside the W rows or repeated: refused"));
                    }
                }
                "F" => {
                    let row: u8 = f.next().ok_or("F")?.parse().map_err(|_| "F")?;
                    let list: u32 = f.next().ok_or("F")?.parse().map_err(|_| "F")?;
                    let si: u32 = f.next().ok_or("F")?.parse().map_err(|_| "F")?;
                    let lv: u8 = f.next().ok_or("F")?.parse().map_err(|_| "F")?;
                    let ni: u8 = f.next().ok_or("F")?.parse().map_err(|_| "F")?;
                    if usize::from(row) >= b.win.len() || ni >= 4 {
                        return Err(format!("F row {row} / slot {ni} outside the file's shape: refused"));
                    }
                    let sm = sum_of_line(&f.collect::<Vec<_>>().join("\t"))?;
                    b.flagged.push(crate::report3::Flag { row, list, si, l: lv, ni, sm });
                }
                "S18" => {
                    b.s18 = (num(f.next(), "S18")?, num(f.next(), "S18")?);
                    if f.next() != Some("0") {
                        return Err("S18 in a batch body carries ns_sum 0 (timing lives in TIMING)".to_owned());
                    }
                    seen_s18 = true;
                }
                "S18SHAPE" => {
                    b.s18shape = rest.split(',').filter(|x| !x.is_empty()).map(|x| x.parse::<usize>().map_err(|_| "S18SHAPE".to_owned())).collect::<Result<_, _>>()?;
                }
                "S18B" => {
                    let (si, fi, best) = crate::split18::s18b_parse(rest)?;
                    if b.s18shape.get(si).is_none_or(|&nf| fi >= nf) {
                        return Err(format!("S18B split {si} / fold {fi} outside the file's S18SHAPE: refused"));
                    }
                    b.s18b.push((si, fi, best));
                }
                "ACCT" => {
                    b.acct = (num(f.next(), "ACCT")?, f.next().ok_or("ACCT")?.to_owned());
                }
                "BODY" => {
                    in_body = false;
                    b.body_sha = rest.to_owned();
                    let got = sha256_bytes(body.as_bytes());
                    if got != b.body_sha {
                        return Err(format!("body sha256 {got} is not the file's {}: a damaged or edited batch file: refused", b.body_sha));
                    }
                }
                "TIMING" => {
                    for x in f {
                        if let Some(v) = x.strip_prefix("ns_sum=") {
                            b.ns_sum = v.parse().map_err(|_| "TIMING ns_sum")?;
                        } else if let Some(v) = x.strip_prefix("s18n=") {
                            b.s18n = v.split(',').filter(|y| !y.is_empty()).map(|y| y.parse::<u32>().map_err(|_| "TIMING s18n".to_owned())).collect::<Result<_, _>>()?;
                        } else {
                            return Err(format!("TIMING: unknown field {x:?}"));
                        }
                    }
                }
                "PROV" => {
                    b.prov = f.filter(|x| !x.is_empty()).map(|x| x.split_once('=').map(|(k, v)| (k.to_owned(), v.to_owned())).ok_or_else(|| format!("PROV field {x:?}"))).collect::<Result<_, _>>()?;
                }
                "END" => {}
                // MUTANT m_kit_merge_ignores_unknown: an unknown tag skipped (ckpt::load's `_ => {}`)
                t if cfg!(feature = "m_kit_merge_ignores_unknown") => {
                    let _ = t;
                }
                t => return Err(format!("unknown tag {t:?} at line {} (a line without a merge law): refused", i + 1)),
            }
        }
        if in_body || b.body_sha.is_empty() {
            return Err("no BODY line: refused".to_owned());
        }
        if !seen_s18 || b.acct.1.len() != 64 || b.fp.is_empty() || b.nb == 0 {
            return Err("batch file without its S18 / ACCT / header lines: refused".to_owned());
        }
        for (r, w) in b.win.iter_mut().enumerate() {
            match (wb.remove(&r), wl.remove(&r), wt.remove(&r)) {
                (Some((key, sm)), Some(lf), Some(tf)) => w.3 = Some((key, sm, lf, tf)),
                (None, None, None) => {}
                _ => return Err(format!("window row {r}: WB / WL / WT incomplete: refused")),
            }
        }
        Ok(b)
    }
}

fn num_i(x: Option<&str>) -> Result<i64, String> {
    x.ok_or("short key")?.parse::<i64>().map_err(|_| "bad key".to_owned())
}

/// The batch file path of batch `k`.
#[must_use]
pub fn bck_path(dir: &Path, seg_short: &str, k: usize) -> PathBuf {
    dir.join(format!("{seg_short}.b{k:06}.bck"))
}

/// Writes a batch file atomically and durably (`.part`, fsync, rename). An existing file is never overwritten: a byte-identical one is
/// kept; a different one is REFUSED (two writers or a changed input), except an unverifiable one, which is moved aside first.
///
/// # Errors
/// An unwritable directory or a different existing file.
pub fn write_bck(dir: &Path, seg_short: &str, b: &Bck) -> Result<PathBuf, String> {
    use std::io::Write as _;
    let p = bck_path(dir, seg_short, b.k);
    let text = b.text();
    if let Ok(old) = std::fs::read_to_string(&p) {
        match Bck::parse(&old) {
            Ok(o) if o.body_sha == sha256_bytes(b.body().as_bytes()) => return Ok(p),
            Ok(o) => return Err(format!("{}: an existing batch file with another body ({}): refused (two writers, or the inputs changed)", p.display(), o.body_sha)),
            Err(_) => {
                let aside = p.with_extension(format!("bck.bad-{}", utc_now()));
                std::fs::rename(&p, &aside).map_err(|e| format!("{}: {e}", p.display()))?;
            }
        }
    }
    let part = p.with_extension("bck.part");
    let mut f = std::fs::File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
    f.write_all(text.as_bytes()).map_err(|e| format!("{}: {e}", part.display()))?;
    f.sync_all().map_err(|e| format!("{}: {e}", part.display()))?;
    drop(f);
    std::fs::rename(&part, &p).map_err(|e| format!("{}: {e}", p.display()))?;
    Ok(p)
}

/// E1: whether batch `k`'s file exists and verifies for this run (body sha256, END, the same fingerprint); a damaged file is moved aside
/// and counts as absent (the batch is walked again).
///
/// # Errors
/// A verifying file of ANOTHER run (fingerprint differs).
pub fn bck_done(dir: &Path, seg_short: &str, k: usize, fp: &str) -> Result<bool, String> {
    let p = bck_path(dir, seg_short, k);
    let Ok(text) = std::fs::read_to_string(&p) else { return Ok(false) };
    match Bck::parse(&text) {
        Ok(b) if b.fp == fp && b.k == k => Ok(true),
        Ok(b) => Err(format!("{}: written by another run or batch (fingerprint / batch {} differ): refused; move it aside to start fresh", p.display(), b.k)),
        Err(_) => {
            let aside = p.with_extension(format!("bck.bad-{}", utc_now()));
            std::fs::rename(&p, &aside).map_err(|e| format!("{}: {e}", p.display()))?;
            Ok(false)
        }
    }
}

/// E4: the binary's own sha256 (in-process, once).
fn exe_sha() -> String {
    static S: OnceLock<String> = OnceLock::new();
    S.get_or_init(|| std::env::current_exe().ok().and_then(|p| sha256_file(&p)).unwrap_or_else(|| "?".to_owned())).clone()
}

/// E4: the host name (`hostname`; "?" when it cannot run).
fn host() -> String {
    static S: OnceLock<String> = OnceLock::new();
    S.get_or_init(|| std::process::Command::new(tool("hostname")).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned()).filter(|x| !x.is_empty()).unwrap_or_else(|| "?".to_owned())).clone()
}

/// E4: the engine source manifest's sha256 baked in at build time (`SF_SOURCE_SHA256`; "unset" in a build without it).
#[must_use]
pub fn source_sha() -> &'static str {
    option_env!("SF_SOURCE_SHA256").unwrap_or("unset")
}

/// E4: the provenance block of one batch (never hashed, never merged; checked by the merge).
#[must_use]
pub fn provenance(threads: usize, t_start: u64, wall_s: f64, cpu_s: f64) -> Vec<(String, String)> {
    vec![
        ("source".to_owned(), source_sha().to_owned()),
        ("rustc".to_owned(), option_env!("SF_RUSTC").unwrap_or("unset").to_owned()),
        ("target".to_owned(), format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)),
        ("binary".to_owned(), exe_sha()),
        ("inputs".to_owned(), INPUTS_SHA.get().cloned().unwrap_or_else(|| "unset".to_owned())),
        ("plan".to_owned(), PLAN.get().map_or_else(|| "-".to_owned(), |p| p.sha.clone())),
        ("threads".to_owned(), threads.to_string()),
        ("host".to_owned(), host()),
        ("utc_start".to_owned(), utc_label(t_start)),
        ("utc_end".to_owned(), utc_label(utc_now())),
        ("wall_s".to_owned(), format!("{wall_s:.1}")),
        ("cpu_s".to_owned(), format!("{cpu_s:.1}")),
        ("rss_kb".to_owned(), rss_kb().map_or_else(|| "?".to_owned(), |x| x.to_string())),
        ("mode".to_owned(), if on(SHARD) { "shard".to_owned() } else { "reference".to_owned() }),
    ]
}

//! Dated Kite instrument-master snapshots, the day's diff, and `token_on`.
//! R4-28 (DLY-09); D-7185.
//!
//! # Why this exists
//!
//! [`crate::masters::land`] renames the fetched Kite dump over
//! `zerodha_instruments.csv`, so the master an earlier day was read against is
//! gone the moment today's lands. Kite's own historical page records why that
//! loses identity: the exchanges flush a derivative's `instrument_token` at
//! every expiry and reuse it, and the instruments API lists only what trades
//! now (`docs/00-charter.md` §4z, the "Expired F&O" and instruments rows;
//! `CLAUDE.md` §1). A token read against today's master names today's
//! contract, whatever it named on the day a bar was printed.
//!
//! # The pattern is `cash_session_cache`'s
//!
//! After every landing of the Kite dump, and of NSE's NIFTY Total Market file
//! (the only ISIN-bearing master the same refresh lands; the Kite dump has no
//! ISIN column), the same bytes are written gzip-compressed as
//! `<stem>_<yyyymmdd>.csv.gz` with a `.receipt` binding the date, the URL, the
//! compressed length and SHA-256, and the body's own length and SHA-256. Both
//! files are created `O_EXCL` and never opened for writing again.
//!
//! **The receipt is the commit marker.** The body is written and synced first
//! and the receipt last, so a kill between the two leaves a body with no
//! receipt: [`Catalog`] names it incomplete and nothing here reads it. Every
//! later refresh names every such entry, in the refresh outcome
//! ([`Snapshotted::incomplete`]) and in one Warn [`EVENT_TARGET`] event each
//! (R4-28 repair 3), since the code deletes nothing and the residue stays. The
//! receipt itself is written whole under a `.partial` name and renamed into
//! place (R4-28 repair 1), so a kill mid-receipt leaves a partial file
//! [`Catalog`] names, never a torn receipt listed as receipted. A later
//! refresh the same day takes the next ordinal (`_2`, `_3`, ...), and the
//! newest receipted ordinal of a day is that day's snapshot. A refresh whose
//! body equals the day's snapshot is "already held" and writes nothing.
//!
//! # `token_on`
//!
//! [`Book::token_on`] answers what `(exchange, token)` named on a day, from
//! the newest receipted snapshot on or before it, through that snapshot's own
//! hash index. Snapshots come only as often as attended refreshes, so between
//! two of them the older one is believed only when the next later one maps the
//! token to the same contract; otherwise the answer is
//! [`TokenOn::Ambiguous`], naming both snapshot dates. A day that is itself a
//! snapshot day is answered by that snapshot alone. Before the first snapshot
//! the answer is [`TokenOn::BeforeFirst`], by name: never the earliest row and
//! never today's. A row whose contract expired before the day is
//! [`TokenOn::Expired`] (`token_expired`), never resolved: the token is
//! flushed at expiry and reused (R4-28 repair 3, PREC-10).
//!
//! # The diff
//!
//! Each new Kite snapshot is compared with the receipted snapshot before it,
//! keyed `(exchange, tradingsymbol, expiry, strike, type)`: listings,
//! delistings, symbol changes (the same token under a new symbol, or the same
//! ISIN under a new symbol), ISIN changes, lot-size, expiry and token changes,
//! and the NSE shares whose ISIN the same-date NSE file does not list
//! (`isin_unknown`, never inferred). It is written once, as a fixed-record
//! `<yyyymmdd>.diff` ([`RECORD_LEN`] bytes a line): whole under an `O_EXCL`
//! partial name, synced, and renamed into place (R4-28 repair 4), so a kill
//! mid-write leaves a partial [`Catalog`] names and nothing reads, never a
//! torn diff under its own name, and the next refresh makes it again. The
//! refresh outcome and the [`EVENT_TARGET`] events carry its counts and name
//! every symbol, ISIN, lot-size, expiry and token change.
//!
//! # Cost
//!
//! | operation | cost |
//! |---|---|
//! | [`Book::token_on`] on loaded snapshots | one day-table index and at most two hash probes |
//! | first probe into a snapshot not yet loaded | one read, one decompression and one parse of that snapshot, O(rows), once |
//! | [`snapshot`] | one compression, one directory listing, at most one receipt check, and for a Kite snapshot one parse to take its futures into [`first_listed`] |
//! | [`diff`] | O(rows of both snapshots), once per refresh |
//! | [`first_listed::listing`] | no snapshot opened: the absorbed log and one underlying's index file |
//!
//! The day table makes the snapshot lookup an index rather than a search: it
//! holds one entry per calendar day the snapshots span. Measured by
//! `crates/pull/benches/ratio/master_snapshots.rs` (C-MS-01) at 1, 10 and 100
//! snapshots; `docs/06-limits.md` L-R428-01..06 record the bounds.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use brutex_core::vendor::Vendor;
use flate2::bufread::GzDecoder;
use flate2::{Compression, GzBuilder};
use sha2::{Digest, Sha256};
use store::flock::Flock;

use crate::masters::{Landed, MAX_BODY_BYTES, NSE_TOTAL_MARKET_FILE, Source};
use crate::session::{Day, IstMoment};

/// The directory, inside the masters directory, the snapshots live in.
pub const SNAPSHOT_DIR: &str = "master-snapshots";

/// The `/logs` target of every snapshot, diff and change event.
pub const EVENT_TARGET: &str = "pull.masters.snapshot";

/// The first line of every receipt. A receipt of another format is refused.
const RECEIPT_FORMAT: &str = "brutex-master-snapshot-v1";

/// The first field of every `.diff` header line.
const DIFF_FORMAT: &str = "brutex-master-diff-v1";

/// The advisory lock every writer takes, beside the snapshots.
const LOCK_FILE: &str = ".master-snapshots.lock";

/// The most snapshots one kind may hold for one day. A refresh past it is
/// refused by name rather than numbered without end.
pub const MAX_ORDINAL: u32 = 99;

/// The largest compressed snapshot read back, in bytes.
const MAX_COMPRESSED: u64 = 128 * 1_024 * 1_024;

/// The largest receipt read back, in bytes.
const MAX_RECEIPT: u64 = 1_024;

/// The widest span, in days, a catalog's day table may cover (forty years).
/// A stamp further apart than this is a misnamed file, refused by name.
const MAX_SPAN_DAYS: u32 = 40 * 366;

/// Bytes in one `.diff` line, newline included.
pub const RECORD_LEN: usize = 192;

/// Which landed master a snapshot is of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// Kite's instrument dump, `zerodha_instruments.csv`.
    Kite,
    /// NSE's NIFTY Total Market constituents, the same-date ISIN witness.
    NseTotalMarket,
}

impl Kind {
    /// Both kinds, in file-name order.
    pub const ALL: [Self; 2] = [Self::Kite, Self::NseTotalMarket];

    /// The file-name stem.
    #[must_use]
    pub const fn stem(self) -> &'static str {
        match self {
            Self::Kite => "instruments",
            Self::NseTotalMarket => "nse_total_market",
        }
    }

    /// The kind a landed source is snapshotted as, or `None` for a source
    /// that is not: the Kite dump identifies tokens, and the NSE file is the
    /// only master the refresh lands that carries an ISIN beside a symbol.
    #[must_use]
    pub fn of(source: &Source) -> Option<Self> {
        if source.vendor == Some(Vendor::Zerodha) {
            Some(Self::Kite)
        } else if source.file == NSE_TOTAL_MARKET_FILE {
            Some(Self::NseTotalMarket)
        } else {
            None
        }
    }
}

/// One snapshot's place: its date and its ordinal within the date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Stamp {
    /// The civil IST date the master was landed on.
    pub day: Day,
    /// 1 for the day's first snapshot, 2 for `_2`, and so on.
    pub ordinal: u32,
}

impl core::fmt::Display for Stamp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.ordinal <= 1 {
            write!(f, "{}", self.day)
        } else {
            write!(f, "{}_{}", self.day, self.ordinal)
        }
    }
}

/// `yyyymmdd`, the date as file names carry it.
fn compact(day: Day) -> String {
    format!("{:04}{:02}{:02}", day.year(), day.month(), day.day())
}

/// `yyyymmdd` or `yyyymmdd_<n>`.
fn base(stamp: Stamp) -> String {
    if stamp.ordinal <= 1 {
        compact(stamp.day)
    } else {
        format!("{}_{}", compact(stamp.day), stamp.ordinal)
    }
}

/// The snapshot file's name.
#[must_use]
pub fn file_name(kind: Kind, stamp: Stamp) -> String {
    format!("{}_{}.csv.gz", kind.stem(), base(stamp))
}

/// The receipt's name.
#[must_use]
pub fn receipt_name(kind: Kind, stamp: Stamp) -> String {
    format!("{}.receipt", file_name(kind, stamp))
}

/// The diff's name: `<yyyymmdd>.diff`, or `<yyyymmdd>_<n>.diff` for a later
/// snapshot of the same day.
#[must_use]
pub fn diff_name(stamp: Stamp) -> String {
    format!("{}.diff", base(stamp))
}

/// A `yyyymmdd` date, or `None`.
fn parse_compact(text: &str) -> Option<Day> {
    if text.len() != 8 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let year = text.get(0..4)?.parse().ok()?;
    let month = text.get(4..6)?.parse().ok()?;
    let day = text.get(6..8)?.parse().ok()?;
    Day::new(year, month, day).ok()
}

/// A `yyyy-mm-dd` date, or `None`.
fn parse_dashed(text: &str) -> Option<Day> {
    let mut parts = text.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    Day::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?).ok()
}

/// What a directory entry's name says it is.
fn parse_name(name: &str) -> Option<(Kind, Stamp, bool)> {
    for kind in Kind::ALL {
        let Some(rest) = name
            .strip_prefix(kind.stem())
            .and_then(|rest| rest.strip_prefix('_'))
        else {
            continue;
        };
        let (rest, receipt) = rest
            .strip_suffix(".receipt")
            .map_or((rest, false), |rest| (rest, true));
        let rest = rest.strip_suffix(".csv.gz")?;
        return parse_base(rest).map(|stamp| (kind, stamp, receipt));
    }
    None
}

/// A later ordinal as names carry it, `2` and up, or `None`.
///
/// `_1` is never written and a leading zero is never written, so either is a
/// name this module did not make.
fn parse_ordinal(n: &str) -> Option<u32> {
    if n.starts_with('0') || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: u32 = n.parse().ok()?;
    (n >= 2).then_some(n)
}

/// The stamp a name's `yyyymmdd` or `yyyymmdd_<n>` part carries, or `None`.
fn parse_base(rest: &str) -> Option<Stamp> {
    let (date, ordinal) = match rest.split_once('_') {
        None => (rest, 1),
        Some((date, n)) => (date, parse_ordinal(n)?),
    };
    let day = parse_compact(date)?;
    Some(Stamp { day, ordinal })
}

/// The name a `.diff` is written under before its rename commits it: the
/// diff's name and [`PARTIAL`], then `_2`, `_3`, ... past partials a killed
/// writer left (R4-28 repair 4, the r4 check's follow-up).
fn diff_partial_name(stamp: Stamp, attempt: u32) -> String {
    if attempt <= 1 {
        format!("{}{PARTIAL}", diff_name(stamp))
    } else {
        format!("{}{PARTIAL}_{attempt}", diff_name(stamp))
    }
}

/// The Kite stamp a diff partial's name carries, or `None` for any other name.
fn parse_diff_partial(name: &str) -> Option<Stamp> {
    let (base, attempt) = name.split_once(".diff")?;
    let attempt = attempt.strip_prefix(PARTIAL)?;
    let first = attempt.is_empty();
    if !first && attempt.strip_prefix('_').and_then(parse_ordinal).is_none() {
        return None;
    }
    parse_base(base)
}

/// What an entry whose name differs from a name this module makes only in
/// case would have been, or `None` (R4-28 repair 4, the r4 check's
/// follow-up).
///
/// On a case-insensitive volume, the default on macOS, such an entry holds the
/// very name `create_new` would make, so a write at that ordinal is refused on
/// every refresh while the listing, which compares bytes, saw nothing there.
fn case_twin(name: &str) -> Option<(Kind, Stamp)> {
    let lower = name.to_ascii_lowercase();
    if lower == name {
        return None;
    }
    let snapshot = |text: &str| parse_name(text).map(|(kind, stamp, _)| (kind, stamp));
    snapshot(&lower).or_else(|| lower.strip_suffix(PARTIAL).and_then(snapshot))
}

/// Why a body with no receipt is not a snapshot.
const CUT_BODY: &str = "a body with no receipt (a write cut before its commit marker)";

/// Why a `.partial` receipt is not a snapshot's receipt.
const CUT_RECEIPT: &str = "a receipt cut before its rename committed it";

/// Why a receipt with no body is not a snapshot.
const LONE_RECEIPT: &str = "a receipt with no body";

/// Why a diff partial is not a diff (R4-28 repair 4).
const CUT_DIFF: &str = "a .diff cut before its rename committed it; the diff is made again \
                        whole under its own name";

/// Why a case twin is not a snapshot (R4-28 repair 4).
const CASE_TWIN: &str = "a snapshot's name in another case; it holds its ordinal, since a \
                         case-insensitive volume takes it for that name";

/// The most incomplete entries one refresh outcome names, and the most Warn
/// events one refresh emits for them, newest first. The rest are counted,
/// never dropped silently (R4-28 repair 3; `docs/06-limits.md` L-R428-10).
pub const INCOMPLETE_NAMED: usize = 64;

/// The most named diff changes (symbol, ISIN, lot size, expiry) one refresh
/// outcome carries and one refresh emits a Warn event for, in the diff's own
/// order. The rest are counted, `unnamed` in the outcome and one Warn event,
/// and every one of them is in the `.diff` file (R4-28 repair 4, the r4
/// check's follow-up): an exchange-wide lot-size revision changes one row per
/// contract, tens of thousands in one dump, and naming each would be that
/// many events and an outcome of that many objects.
pub const DIFF_NAMED: usize = 256;

/// One directory entry that is not a snapshot: named, never read.
///
/// A kill between a body and its receipt, inside the receipt's rename, or
/// inside a `.diff`'s write, leaves one; so does a name that is a snapshot's in
/// another case (R4-28 repair 4). No reader opens it, and the code deletes
/// nothing, so it stays until someone moves it aside. Every refresh names it
/// (R4-28 repair 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incomplete {
    /// The master it would have been a snapshot of.
    pub kind: Kind,
    /// The stamp its name carries.
    pub stamp: Stamp,
    /// Its file name in the snapshots directory.
    pub file: String,
    /// Why it is not a snapshot.
    pub why: &'static str,
}

impl core::fmt::Display for Incomplete {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: incomplete — {}; never read", self.file, self.why)
    }
}

/// Every snapshot on disk, by name: one directory listing.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    /// Receipted stamps per kind, ascending.
    receipted: HashMap<Kind, Vec<Stamp>>,
    /// The highest ordinal any entry of a (kind, day) carries, complete or not.
    highest: HashMap<(Kind, Day), u32>,
    /// Entries that are not snapshots: a body with no receipt, a receipt with
    /// no body, a receipt cut before its rename, a `.diff` cut before its
    /// rename, a snapshot's name in another case. Named, never read. By kind,
    /// then stamp, then name.
    incomplete: Vec<Incomplete>,
}

impl Catalog {
    /// Lists `root` once. A missing directory is an empty catalog.
    ///
    /// # Errors
    ///
    /// A directory that cannot be listed, named with the reason.
    pub fn read(root: &Path) -> Result<Self, String> {
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(why) if why.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(why) => return Err(format!("{} could not be listed: {why}", root.display())),
        };
        let mut seen: HashMap<(Kind, Stamp), (bool, bool)> = HashMap::new();
        let mut partials = Vec::new();
        let mut cut_diffs = Vec::new();
        for entry in entries {
            let entry =
                entry.map_err(|why| format!("{} could not be listed: {why}", root.display()))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if let Some((kind, stamp, true)) = name.strip_suffix(PARTIAL).and_then(parse_name) {
                partials.push(Incomplete {
                    kind,
                    stamp,
                    file: name.to_owned(),
                    why: CUT_RECEIPT,
                });
                continue;
            }
            // A DIFF CUT BEFORE ITS RENAME, in any case (R4-28 repair 4):
            // named, never read, and never a snapshot's ordinal.
            if let Some(stamp) = parse_diff_partial(&name.to_ascii_lowercase()) {
                cut_diffs.push(Incomplete {
                    kind: Kind::Kite,
                    stamp,
                    file: name.to_owned(),
                    why: CUT_DIFF,
                });
                continue;
            }
            let Some((kind, stamp, receipt)) = parse_name(name) else {
                if let Some((kind, stamp)) = case_twin(name) {
                    partials.push(Incomplete {
                        kind,
                        stamp,
                        file: name.to_owned(),
                        why: CASE_TWIN,
                    });
                }
                continue;
            };
            let slot = seen.entry((kind, stamp)).or_insert((false, false));
            if receipt {
                slot.1 = true;
            } else {
                slot.0 = true;
            }
        }
        let mut catalog = Self::default();
        for (&(kind, stamp), &(body, receipt)) in &seen {
            let highest = catalog.highest.entry((kind, stamp.day)).or_insert(0);
            *highest = (*highest).max(stamp.ordinal);
            match (body, receipt) {
                (true, true) => catalog.receipted.entry(kind).or_default().push(stamp),
                (true, false) => catalog.incomplete.push(Incomplete {
                    kind,
                    stamp,
                    file: file_name(kind, stamp),
                    why: CUT_BODY,
                }),
                _ => catalog.incomplete.push(Incomplete {
                    kind,
                    stamp,
                    file: receipt_name(kind, stamp),
                    why: LONE_RECEIPT,
                }),
            }
        }
        for stamps in catalog.receipted.values_mut() {
            stamps.sort_unstable();
        }
        // A RECEIPT CUT BEFORE ITS RENAME HOLDS ITS ORDINAL TOO (R4-28 repair
        // 4, the r4 check's follow-up): with its body gone, the next write
        // would otherwise take that ordinal and be refused at its own partial
        // receipt name. So does a case twin, whose name a case-insensitive
        // volume takes for the one `create_new` would make.
        for one in &partials {
            let highest = catalog
                .highest
                .entry((one.kind, one.stamp.day))
                .or_insert(0);
            *highest = (*highest).max(one.stamp.ordinal);
        }
        catalog.incomplete.extend(partials);
        catalog.incomplete.extend(cut_diffs);
        catalog
            .incomplete
            .sort_unstable_by(|a, b| (a.kind, a.stamp, &a.file).cmp(&(b.kind, b.stamp, &b.file)));
        Ok(catalog)
    }

    /// The receipted stamps of `kind`, oldest first.
    #[must_use]
    pub fn receipted(&self, kind: Kind) -> &[Stamp] {
        self.receipted.get(&kind).map_or(&[], Vec::as_slice)
    }

    /// The newest receipted stamp of `kind`.
    #[must_use]
    pub fn newest(&self, kind: Kind) -> Option<Stamp> {
        self.receipted(kind).last().copied()
    }

    /// Every entry named incomplete, with why: by kind, then stamp, then name.
    #[must_use]
    pub fn incomplete(&self) -> &[Incomplete] {
        &self.incomplete
    }

    /// The entries of `kind` named incomplete, oldest first.
    #[must_use]
    pub fn incomplete_of(&self, kind: Kind) -> Vec<Incomplete> {
        self.incomplete
            .iter()
            .filter(|one| one.kind == kind)
            .cloned()
            .collect()
    }

    /// The day's snapshot of `kind`: its newest receipted ordinal.
    #[must_use]
    pub fn of_day(&self, kind: Kind, day: Day) -> Option<Stamp> {
        let stamps = self.receipted(kind);
        let after = stamps.partition_point(|stamp| stamp.day <= day);
        after
            .checked_sub(1)
            .and_then(|at| stamps.get(at))
            .copied()
            .filter(|stamp| stamp.day == day)
    }

    /// The receipted stamp of `kind` just before `stamp`.
    #[must_use]
    pub fn before(&self, kind: Kind, stamp: Stamp) -> Option<Stamp> {
        let stamps = self.receipted(kind);
        let at = stamps.partition_point(|held| *held < stamp);
        at.checked_sub(1).and_then(|at| stamps.get(at)).copied()
    }

    /// The highest ordinal any entry of (kind, day) carries, 0 for none.
    fn highest(&self, kind: Kind, day: Day) -> u32 {
        self.highest.get(&(kind, day)).copied().unwrap_or(0)
    }

    /// How many receipted snapshots, and how many incomplete entries, carry
    /// (kind, day): what a refusal past [`MAX_ORDINAL`] counts.
    fn held_on(&self, kind: Kind, day: Day) -> (usize, usize) {
        let receipted = self
            .receipted(kind)
            .iter()
            .filter(|stamp| stamp.day == day)
            .count();
        let incomplete = self
            .incomplete
            .iter()
            .filter(|one| one.kind == kind && one.stamp.day == day)
            .count();
        (receipted, incomplete)
    }
}

/// What one receipt binds.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Receipt {
    url: String,
    compressed_bytes: u64,
    sha256: String,
    body_bytes: u64,
    body_sha256: String,
}

/// Lower-case hex of a digest.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _infallible = write!(out, "{byte:02x}");
    }
    out
}

/// The receipt's exact bytes. Read back by re-rendering, so any other
/// spelling of the same fields is refused rather than reinterpreted.
fn render_receipt(kind: Kind, stamp: Stamp, receipt: &Receipt) -> String {
    format!(
        "{RECEIPT_FORMAT}\nkind={}\ndate={}\nordinal={}\nsource={}\ncompressed_bytes={}\nsha256={}\nbody_bytes={}\nbody_sha256={}\n",
        kind.stem(),
        stamp.day,
        stamp.ordinal,
        receipt.url,
        receipt.compressed_bytes,
        receipt.sha256,
        receipt.body_bytes,
        receipt.body_sha256,
    )
}

/// One field of a receipt line, `name=value`.
fn field<'a>(line: Option<&'a str>, name: &str) -> Option<&'a str> {
    line?.strip_prefix(name)?.strip_prefix('=')
}

/// Parses a receipt's text; the caller re-renders and compares.
fn parse_receipt(text: &str) -> Option<Receipt> {
    let mut lines = text.lines().skip(4);
    let url = field(lines.next(), "source")?.to_owned();
    let compressed_bytes = field(lines.next(), "compressed_bytes")?.parse().ok()?;
    let sha256 = field(lines.next(), "sha256")?.to_owned();
    let body_bytes = field(lines.next(), "body_bytes")?.parse().ok()?;
    let body_sha256 = field(lines.next(), "body_sha256")?.to_owned();
    Some(Receipt {
        url,
        compressed_bytes,
        sha256,
        body_bytes,
        body_sha256,
    })
}

/// Reads at most `limit` bytes of a regular file.
fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let file =
        File::open(path).map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|why| format!("{} could not be inspected: {why}", path.display()))?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(format!(
            "{} is not a regular file within {limit} bytes",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|why| format!("{} could not be read: {why}", path.display()))?;
    Ok(bytes)
}

/// A receipted snapshot's compressed bytes and receipt, both checked: the
/// receipt re-renders byte for byte and binds this date, ordinal and kind,
/// and the body's length and SHA-256 are the ones it states.
fn read_checked(root: &Path, kind: Kind, stamp: Stamp) -> Result<(Receipt, Vec<u8>), String> {
    let receipt_path = root.join(receipt_name(kind, stamp));
    let text = read_limited(&receipt_path, MAX_RECEIPT)?;
    let text = String::from_utf8(text).map_err(|_| {
        format!(
            "{} is not UTF-8; refused, never read",
            receipt_path.display()
        )
    })?;
    let receipt = parse_receipt(&text)
        .filter(|receipt| render_receipt(kind, stamp, receipt) == text)
        .ok_or_else(|| {
            format!(
                "{} is not a {RECEIPT_FORMAT} receipt for {} {stamp}; refused, never read",
                receipt_path.display(),
                kind.stem()
            )
        })?;
    let body_path = root.join(file_name(kind, stamp));
    let bytes = read_limited(&body_path, MAX_COMPRESSED)?;
    if bytes.len() as u64 != receipt.compressed_bytes
        || hex(&Sha256::digest(&bytes)) != receipt.sha256
    {
        return Err(format!(
            "{} does not match its receipt (length or SHA-256); refused, never read",
            body_path.display()
        ));
    }
    Ok((receipt, bytes))
}

/// The gzip of `body`, with a fixed header (no name, no time), so one body
/// always compresses to one byte string on one build.
fn compress(body: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .operating_system(255)
        .write(Vec::with_capacity(body.len() / 4), Compression::new(6));
    encoder
        .write_all(body)
        .map_err(|why| format!("the master could not be compressed: {why}"))?;
    encoder
        .finish()
        .map_err(|why| format!("the master could not be compressed: {why}"))
}

/// The body of a checked snapshot, decompressed and held to its receipt.
fn expand(compressed: &[u8], receipt: &Receipt) -> Result<String, String> {
    let mut decoder = GzDecoder::new(compressed);
    let mut body = Vec::new();
    decoder
        .by_ref()
        .take(MAX_BODY_BYTES as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|why| format!("the snapshot is not one gzip member: {why}"))?;
    if !decoder.into_inner().is_empty() {
        return Err("the snapshot carries bytes after its gzip member; refused".to_owned());
    }
    if body.len() as u64 != receipt.body_bytes || hex(&Sha256::digest(&body)) != receipt.body_sha256
    {
        return Err("the snapshot's body does not match its receipt; refused".to_owned());
    }
    String::from_utf8(body).map_err(|_| "the snapshot's body is not UTF-8; refused".to_owned())
}

/// The body of the receipted snapshot `stamp` of `kind`.
///
/// # Errors
///
/// Every refusal of the receipt or the bytes, named.
pub fn read_body(root: &Path, kind: Kind, stamp: Stamp) -> Result<String, String> {
    let (receipt, bytes) = read_checked(root, kind, stamp)?;
    expand(&bytes, &receipt)
}

/// Creates `path` with `bytes`, refusing any existing file, and syncs it.
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|why| {
            format!(
                "{} could not be created; no existing file was touched: {why}",
                path.display()
            )
        })?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|why| format!("{} is incomplete and retained: {why}", path.display()))
}

/// The suffix a receipt carries while it is written, before its rename commits
/// it (R4-28 repair 1).
const PARTIAL: &str = ".partial";

/// Writes a receipt whole under its partial name, synced, and renames it into
/// place, so a receipt exists complete or not at all: a kill mid-write leaves
/// the body with no receipt (incomplete, never read) and a partial file
/// [`Catalog`] names, never a torn receipt listed as receipted. A receipt
/// already at the name is refused and left as it is.
fn commit_receipt(root: &Path, kind: Kind, stamp: Stamp, bytes: &[u8]) -> Result<(), String> {
    let path = root.join(receipt_name(kind, stamp));
    let partial = root.join(format!("{}{PARTIAL}", receipt_name(kind, stamp)));
    write_new(&partial, bytes)?;
    if path.exists() {
        return Err(format!(
            "{} already exists and is left as it is; the receipt stays partial and the body is never read",
            path.display()
        ));
    }
    fs::rename(&partial, &path).map_err(|why| {
        format!(
            "{} could not be committed: {why}; the body stays incomplete and is never read",
            path.display()
        )
    })
}

/// Syncs the directory entry of everything just created in `root`.
fn sync_dir(root: &Path) -> Result<(), String> {
    File::open(root).and_then(|dir| dir.sync_all()).map_err(|why| {
        format!(
            "{} could not be synced: {why}; the new files are visible, crash durability UNVERIFIED",
            root.display()
        )
    })
}

/// Takes the writers' lock on `root`.
fn lock(root: &Path) -> Result<Flock<File>, String> {
    let path = root.join(LOCK_FILE);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|why| format!("{} could not be opened for locking: {why}", path.display()))?;
    Flock::lock(file, path.clone())
        .map_err(|why| format!("{} could not be locked: {why}", path.display()))
}

/// The catalog of `root`, listed under the writers' lock taken shared, so no
/// write is in flight while it lists (R4-28 repair 4). A root that does not
/// exist is an empty catalog, and is not created.
///
/// # Errors
///
/// A root that cannot be read, a lock that cannot be taken or released, or a
/// listing that fails, named.
fn read_quiesced(root: &Path) -> Result<Catalog, String> {
    match fs::symlink_metadata(root) {
        Ok(_) => {}
        Err(why) if why.kind() == ErrorKind::NotFound => return Ok(Catalog::default()),
        Err(why) => return Err(format!("{} could not be read: {why}", root.display())),
    }
    let path = root.join(LOCK_FILE);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|why| format!("{} could not be opened for locking: {why}", path.display()))?;
    let held = Flock::lock_shared(file, path.clone())
        .map_err(|why| format!("{} could not be locked: {why}", path.display()))?;
    let catalog = Catalog::read(root)?;
    held.release().map_err(|why| why.to_string())?;
    Ok(catalog)
}

/// What one snapshot write did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrote {
    /// A new snapshot, receipted.
    New(Stamp),
    /// The day's snapshot already holds these exact bytes; nothing written.
    AlreadyHeld(Stamp),
}

impl Wrote {
    /// The snapshot that now stands for the day.
    #[must_use]
    pub const fn stamp(self) -> Stamp {
        match self {
            Self::New(stamp) | Self::AlreadyHeld(stamp) => stamp,
        }
    }

    /// `new` or `already_held`.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::New(_) => "new",
            Self::AlreadyHeld(_) => "already_held",
        }
    }
}

/// What one snapshot write did, and every entry of its kind that is not a
/// snapshot (R4-28 repair 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The snapshot written or already held, or why none was.
    pub wrote: Result<Wrote, String>,
    /// Every entry of the kind the catalog named incomplete, oldest first:
    /// the listing the write read under the writers' lock, or for a write
    /// refused before it took the lock, one listing read then. `Err` names a
    /// listing that could not be read.
    pub incomplete: Result<Vec<Incomplete>, String>,
}

/// Writes `body` as `day`'s snapshot of `kind`, or says the day already holds
/// it, and names every incomplete entry of the kind in `/logs`.
///
/// It is [`write_snapshot`] with each incomplete entry named in its own Warn
/// [`EVENT_TARGET`] event, so no caller can drop the list unseen. The landing
/// hook calls [`write_snapshot`] and names them in the refresh outcome too.
///
/// # Errors
///
/// Every refusal of [`write_snapshot`], named.
pub fn snapshot(
    root: &Path,
    kind: Kind,
    day: Day,
    url: &str,
    body: &[u8],
) -> Result<Wrote, String> {
    let written = write_snapshot(root, kind, day, url, body);
    note_incomplete(kind, &written.incomplete);
    written.wrote
}

/// Writes `body` as `day`'s snapshot of `kind`, or says the day already holds
/// it. Never rewrites a file: a body equal to the day's newest receipted
/// snapshot is [`Wrote::AlreadyHeld`], a different one takes the next free
/// ordinal (past any incomplete entry), body first and receipt last. A Kite
/// snapshot's futures, and those of any earlier one not yet taken in, then go
/// into the [`first_listed`] index under the same lock.
///
/// The entries of the kind that are not snapshots come back beside the
/// outcome, from the one listing the write already reads (R4-28 repair 3).
/// Refusals: an empty or oversized body, a day past [`MAX_ORDINAL`] entries,
/// and every filesystem refusal, each named in [`Written::wrote`]. Nothing is
/// overwritten on any path.
#[must_use]
pub fn write_snapshot(root: &Path, kind: Kind, day: Day, url: &str, body: &[u8]) -> Written {
    let refused = |why: String| Written {
        wrote: Err(why),
        incomplete: Catalog::read(root).map(|catalog| catalog.incomplete_of(kind)),
    };
    if body.is_empty() {
        return refused("an empty body is not a master; no snapshot was written".to_owned());
    }
    if body.len() > MAX_BODY_BYTES {
        return refused(format!(
            "a body of {} bytes is past the {MAX_BODY_BYTES}-byte bound; no snapshot was written",
            body.len()
        ));
    }
    if url.contains('\n') {
        return refused("a source URL with a line break cannot be receipted".to_owned());
    }
    if let Err(why) = fs::create_dir_all(root) {
        return refused(format!("{} could not be created: {why}", root.display()));
    }
    let compressed = match compress(body) {
        Ok(compressed) => compressed,
        Err(why) => return refused(why),
    };
    let held = match lock(root) {
        Ok(held) => held,
        Err(why) => return refused(why),
    };
    // ONE LISTING, UNDER THE LOCK: it numbers the write and names the
    // incomplete entries, so the two can never disagree.
    let catalog = match Catalog::read(root) {
        Ok(catalog) => catalog,
        Err(why) => {
            return Written {
                wrote: Err(why.clone()),
                incomplete: Err(why),
            };
        }
    };
    let wrote = write_held(root, kind, day, url, body, &compressed, &catalog).and_then(|wrote| {
        held.release()
            .map(|()| wrote)
            .map_err(|why| why.to_string())
    });
    Written {
        wrote,
        incomplete: Ok(catalog.incomplete_of(kind)),
    }
}

/// The write itself, under the writers' lock, over the listing read under it.
fn write_held(
    root: &Path,
    kind: Kind,
    day: Day,
    url: &str,
    body: &[u8],
    compressed: &[u8],
    catalog: &Catalog,
) -> Result<Wrote, String> {
    let body_sha256 = hex(&Sha256::digest(body));
    if let Some(newest) = catalog.of_day(kind, day)
        && let Ok((receipt, _)) = read_checked(root, kind, newest)
        && receipt.body_sha256 == body_sha256
        && receipt.body_bytes == body.len() as u64
    {
        index_futures(root, kind);
        return Ok(Wrote::AlreadyHeld(newest));
    }
    let ordinal = catalog.highest(kind, day).saturating_add(1);
    if ordinal > MAX_ORDINAL {
        // COUNTED AS WHAT THEY ARE: an incomplete entry takes an ordinal and
        // is never a snapshot (R4-28 repair 3).
        let (receipted, incomplete) = catalog.held_on(kind, day);
        return Err(format!(
            "{day} already holds {} entries up to ordinal {MAX_ORDINAL} ({receipted} receipted \
             snapshots, {incomplete} incomplete entries never read); refused rather than \
             numbered without end",
            kind.stem()
        ));
    }
    let stamp = Stamp { day, ordinal };
    let receipt = Receipt {
        url: url.to_owned(),
        compressed_bytes: compressed.len() as u64,
        sha256: hex(&Sha256::digest(compressed)),
        body_bytes: body.len() as u64,
        body_sha256,
    };
    // BODY FIRST, RECEIPT LAST: a kill between them leaves an incomplete entry
    // that no reader opens, and the next refresh takes the next ordinal and
    // names it.
    write_new(&root.join(file_name(kind, stamp)), compressed)?;
    commit_receipt(
        root,
        kind,
        stamp,
        render_receipt(kind, stamp, &receipt).as_bytes(),
    )?;
    sync_dir(root)?;
    index_futures(root, kind);
    Ok(Wrote::New(stamp))
}

/// The incomplete entries of one write, newest first, at most
/// [`INCOMPLETE_NAMED`] of them, and how many more are held unnamed.
fn newest_named(incomplete: &[Incomplete]) -> (impl Iterator<Item = &Incomplete>, usize) {
    (
        incomplete.iter().rev().take(INCOMPLETE_NAMED),
        incomplete.len().saturating_sub(INCOMPLETE_NAMED),
    )
}

/// One Warn [`EVENT_TARGET`] event per incomplete entry of `kind`, newest
/// first, at most [`INCOMPLETE_NAMED`], one more counting any past the bound,
/// and an Error event when the directory could not be listed to name them.
fn note_incomplete(kind: Kind, incomplete: &Result<Vec<Incomplete>, String>) {
    let entries = match incomplete {
        Ok(entries) => entries,
        Err(why) => {
            let _ = telemetry::emit_if!(
                telemetry::Level::Error,
                EVENT_TARGET,
                "the snapshots directory could not be listed, so its incomplete entries are not named",
                "kind" => telemetry::Value::Str(kind.stem()),
                "why" => telemetry::Value::Str(why),
            );
            return;
        }
    };
    let held = entries.len() as u64;
    let (named, more) = newest_named(entries);
    for one in named {
        let (stamp, day) = (one.stamp.to_string(), one.stamp.day.to_string());
        let _ = telemetry::emit_if!(
            telemetry::Level::Warn,
            EVENT_TARGET,
            INCOMPLETE_MESSAGE,
            "kind" => telemetry::Value::Str(kind.stem()),
            "entry" => telemetry::Value::Str(&one.file),
            "stamp" => telemetry::Value::Str(&stamp),
            "day" => telemetry::Value::Str(&day),
            "why" => telemetry::Value::Str(one.why),
            "held" => telemetry::Value::Uint(held),
        );
    }
    if more > 0 {
        let _ = telemetry::emit_if!(
            telemetry::Level::Warn,
            EVENT_TARGET,
            "more incomplete master snapshot entries are held than one refresh names",
            "kind" => telemetry::Value::Str(kind.stem()),
            "unnamed" => telemetry::Value::Uint(more as u64),
            "held" => telemetry::Value::Uint(held),
        );
    }
}

/// The message of each incomplete entry's event.
pub const INCOMPLETE_MESSAGE: &str = "an incomplete master snapshot entry is held and never read";

/// A Kite snapshot's futures go into the first-listed index, under the lock
/// the write already holds, so no futures plan has to open it again (R4-28
/// repair 1; D-7187). Its outcome is a [`EVENT_TARGET`] event; a failure
/// leaves the snapshot for the next plan to take in and name.
fn index_futures(root: &Path, kind: Kind) {
    if kind == Kind::Kite {
        first_listed::note(&first_listed::absorb_held(
            root,
            crate::vendor::ZERODHA_CONTINUOUS,
        ));
    }
}

/// The newest receipted Kite snapshot's date, read from its receipt (which is
/// checked against its body). `Ok(None)` when no snapshot exists. For the pull
/// door's `master_stale` refusal (R4-23, FM-267), which passes its own
/// `today`.
///
/// # Errors
///
/// A listing that fails, or a newest snapshot whose receipt or body does not
/// check, named — never an older date in its place. A newest snapshot dated
/// after `today` is refused by name (R4-28 repair 3, verify r3 A13): every
/// snapshot is dated by the clock that writes it, so one from the future means
/// that clock jumped, and believing it would call the master fresh until the
/// calendar caught up.
pub fn newest_snapshot_day(root: &Path, today: Day) -> Result<Option<Day>, String> {
    let catalog = Catalog::read(root)?;
    let Some(newest) = catalog.newest(Kind::Kite) else {
        return Ok(None);
    };
    read_checked(root, Kind::Kite, newest)?;
    if newest.day > today {
        return Err(format!(
            "the newest Kite snapshot, {}, is dated {}, after today {today}: the clock that \
             wrote it ran ahead or this one runs behind; refused rather than believed fresh. \
             Move the future-dated snapshot and its receipt aside by hand to resume",
            file_name(Kind::Kite, newest),
            newest.day
        ));
    }
    Ok(Some(newest.day))
}

/// The civil IST date now, the date a landing is snapshotted under.
///
/// # Errors
///
/// A clock before the epoch or past what a [`Day`] names.
pub fn today_ist() -> Result<Day, String> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|why| format!("the clock is before 1970: {why}"))?
        .as_secs();
    let secs = i64::try_from(secs).map_err(|_| "the clock is past i64 seconds".to_owned())?;
    IstMoment::from_epoch_secs(secs)
        .map(IstMoment::day)
        .map_err(|why| format!("the clock does not name an IST day: {why}"))
}

// ─────────────────────────────── the Kite rows ───────────────────────────────

/// The columns a Kite dump must carry, by name.
const KITE_COLUMNS: [&str; 10] = [
    "instrument_token",
    "tradingsymbol",
    "name",
    "expiry",
    "strike",
    "tick_size",
    "lot_size",
    "instrument_type",
    "segment",
    "exchange",
];

/// One row of a snapshot. Words are interned per snapshot.
#[derive(Debug, Clone)]
struct Row {
    token: u64,
    exchange: u16,
    segment: u16,
    kind: u16,
    symbol: Box<str>,
    name: Box<str>,
    expiry: Option<Day>,
    strike: Box<str>,
    tick: Box<str>,
    lot: u32,
}

/// Splits one CSV line, honouring double quotes and doubled quotes.
fn split_line(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    let mut at_start = true;
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
        } else if c == '"' && at_start {
            quoted = true;
            at_start = false;
        } else if c == ',' {
            out.push(core::mem::take(&mut field));
            at_start = true;
        } else {
            field.push(c);
            at_start = false;
        }
    }
    if quoted {
        return Err("a quoted field is not closed".to_owned());
    }
    out.push(field);
    Ok(out)
}

/// A decimal as the dump spells it, normalised: trailing fractional zeros and
/// a bare point dropped, so `25000.0` and `25000` are one strike. Anything
/// that is not an unsigned decimal is refused.
fn decimal(text: &str) -> Option<Box<str>> {
    let text = text.trim();
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let fraction = fraction.trim_end_matches('0');
    let whole = whole.trim_start_matches('0');
    let whole = if whole.is_empty() { "0" } else { whole };
    Some(if fraction.is_empty() {
        whole.into()
    } else {
        format!("{whole}.{fraction}").into()
    })
}

/// One parsed Kite dump with its per-snapshot hash index.
#[derive(Debug, Clone)]
pub struct Snapshot {
    stamp: Stamp,
    words: Vec<Box<str>>,
    word_ids: HashMap<Box<str>, u16>,
    rows: Vec<Row>,
    by_token: HashMap<(u16, u64), u32>,
    duplicated: HashSet<(u16, u64)>,
}

impl Snapshot {
    /// Parses a Kite instrument dump taken on `stamp`.
    ///
    /// # Errors
    ///
    /// A header missing one of the required columns, and any row that does
    /// not read (a token, an expiry, a strike, a tick or a lot that is not
    /// the shape the dump publishes), named by line. A snapshot with a row
    /// skipped would answer "not listed" for a token it holds, so nothing is
    /// skipped.
    pub fn parse(stamp: Stamp, csv: &str) -> Result<Self, String> {
        let mut lines = csv.lines();
        let header = lines.next().ok_or_else(|| "the dump is empty".to_owned())?;
        let header = split_line(header.trim_start_matches('\u{feff}'))?;
        let mut at = [0_usize; KITE_COLUMNS.len()];
        for (slot, column) in at.iter_mut().zip(KITE_COLUMNS) {
            *slot = header
                .iter()
                .position(|name| name.trim() == column)
                .ok_or_else(|| format!("the dump has no column {column:?}"))?;
        }
        let [
            token_at,
            symbol_at,
            name_at,
            expiry_at,
            strike_at,
            tick_at,
            lot_at,
            kind_at,
            segment_at,
            exchange_at,
        ] = at;
        let mut snapshot = Self {
            stamp,
            words: Vec::new(),
            word_ids: HashMap::new(),
            rows: Vec::new(),
            by_token: HashMap::new(),
            duplicated: HashSet::new(),
        };
        for (number, line) in lines.enumerate() {
            let line = line.trim_end_matches('\r');
            if line.is_empty() {
                continue;
            }
            let fields = split_line(line).map_err(|why| format!("line {}: {why}", number + 2))?;
            let cell = |at: usize| fields.get(at).map_or("", String::as_str);
            let refuse =
                |what: &str| format!("line {}: {what} {:?} does not read", number + 2, line);
            let token: u64 = cell(token_at)
                .trim()
                .parse()
                .map_err(|_| refuse("instrument_token"))?;
            let expiry = match cell(expiry_at).trim() {
                "" => None,
                text => Some(parse_dashed(text).ok_or_else(|| refuse("expiry"))?),
            };
            let strike = decimal(cell(strike_at)).ok_or_else(|| refuse("strike"))?;
            let tick = decimal(cell(tick_at)).ok_or_else(|| refuse("tick_size"))?;
            let lot: u32 = cell(lot_at)
                .trim()
                .parse()
                .map_err(|_| refuse("lot_size"))?;
            let exchange = snapshot.intern(cell(exchange_at).trim())?;
            let segment = snapshot.intern(cell(segment_at).trim())?;
            let kind = snapshot.intern(cell(kind_at).trim())?;
            let row = u32::try_from(snapshot.rows.len())
                .map_err(|_| "the dump holds more rows than a u32 counts".to_owned())?;
            if snapshot.by_token.insert((exchange, token), row).is_some() {
                // TWO ROWS FOR ONE TOKEN: neither is believed. The later row
                // takes the slot (`insert` replaces), and the token answers
                // `Duplicated`, so neither row is ever answered for it.
                snapshot.duplicated.insert((exchange, token));
            }
            snapshot.rows.push(Row {
                token,
                exchange,
                segment,
                kind,
                symbol: cell(symbol_at).trim().into(),
                name: cell(name_at).trim().into(),
                expiry,
                strike,
                tick,
                lot,
            });
        }
        Ok(snapshot)
    }

    /// Interns one word.
    fn intern(&mut self, word: &str) -> Result<u16, String> {
        if let Some(&id) = self.word_ids.get(word) {
            return Ok(id);
        }
        let id = u16::try_from(self.words.len())
            .map_err(|_| "the dump carries more distinct words than a u16 counts".to_owned())?;
        self.words.push(word.into());
        self.word_ids.insert(word.into(), id);
        Ok(id)
    }

    /// When this snapshot was taken.
    #[must_use]
    pub const fn stamp(&self) -> Stamp {
        self.stamp
    }

    /// How many rows it holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether it holds no row.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Every row, in file order.
    pub fn rows(&self) -> impl Iterator<Item = Listed<'_>> {
        self.rows.iter().map(move |row| Listed {
            snapshot: self,
            row,
        })
    }

    /// The row `(exchange, token)` names here: one probe for the word, one
    /// for the token.
    #[must_use]
    pub fn lookup(&self, exchange: &str, token: u64) -> Lookup<'_> {
        let Some(&exchange) = self.word_ids.get(exchange) else {
            return Lookup::Absent;
        };
        if self.duplicated.contains(&(exchange, token)) {
            return Lookup::Duplicated;
        }
        self.by_token
            .get(&(exchange, token))
            .and_then(|&row| self.rows.get(row as usize))
            .map_or(Lookup::Absent, |row| {
                Lookup::Listed(Listed {
                    snapshot: self,
                    row,
                })
            })
    }

    fn word(&self, id: u16) -> &str {
        self.words.get(usize::from(id)).map_or("", |word| word)
    }
}

/// What one snapshot says of one token.
#[derive(Debug, Clone, Copy)]
pub enum Lookup<'a> {
    /// The row it names.
    Listed(Listed<'a>),
    /// Not in this snapshot.
    Absent,
    /// Two rows carry it; neither is believed.
    Duplicated,
}

/// One row of a snapshot, borrowed.
#[derive(Debug, Clone, Copy)]
pub struct Listed<'a> {
    snapshot: &'a Snapshot,
    row: &'a Row,
}

impl<'a> Listed<'a> {
    /// The exchange, as the dump spells it (`NSE`, `NFO`, ...).
    #[must_use]
    pub fn exchange(&self) -> &'a str {
        self.snapshot.word(self.row.exchange)
    }
    /// The dump's segment (`NFO-FUT`, `INDICES`, ...).
    #[must_use]
    pub fn segment(&self) -> &'a str {
        self.snapshot.word(self.row.segment)
    }
    /// The dump's instrument type (`FUT`, `CE`, `PE`, `EQ`).
    #[must_use]
    pub fn instrument_type(&self) -> &'a str {
        self.snapshot.word(self.row.kind)
    }
    /// The vendor's token.
    #[must_use]
    pub const fn token(&self) -> u64 {
        self.row.token
    }
    /// The trading symbol.
    #[must_use]
    pub fn tradingsymbol(&self) -> &'a str {
        &self.row.symbol
    }
    /// The `name` column: an underlying for a derivative.
    #[must_use]
    pub fn name(&self) -> &'a str {
        &self.row.name
    }
    /// The expiry, for a derivative.
    #[must_use]
    pub const fn expiry(&self) -> Option<Day> {
        self.row.expiry
    }
    /// The strike, as a normalised decimal string of rupees.
    #[must_use]
    pub fn strike(&self) -> &'a str {
        &self.row.strike
    }
    /// The tick, as a normalised decimal string of rupees.
    #[must_use]
    pub fn tick_size(&self) -> &'a str {
        &self.row.tick
    }
    /// The lot size.
    #[must_use]
    pub const fn lot_size(&self) -> u32 {
        self.row.lot
    }
    /// The snapshot it is read from.
    #[must_use]
    pub const fn stamp(&self) -> Stamp {
        self.snapshot.stamp
    }

    /// Whether `other` is the same contract: the diff's key
    /// `(exchange, tradingsymbol, expiry, strike, type)`.
    #[must_use]
    pub fn same_contract(&self, other: &Listed<'_>) -> bool {
        self.exchange() == other.exchange()
            && self.tradingsymbol() == other.tradingsymbol()
            && self.expiry() == other.expiry()
            && self.strike() == other.strike()
            && self.instrument_type() == other.instrument_type()
    }

    /// Whether it is an NSE cash share, the rows an ISIN applies to.
    #[must_use]
    pub fn is_nse_share(&self) -> bool {
        // THE CASH SEGMENT, NOT THE INDEX ROWS: the dump files NSE's indices
        // under the same exchange and type with segment `INDICES` (136 rows
        // in the dump landed 6 Oct 2026), and an index has no ISIN.
        self.exchange() == "NSE" && self.instrument_type() == "EQ" && self.segment() == "NSE"
    }
}

// ───────────────────────────────── token_on ─────────────────────────────────

/// What `(exchange, token)` named on a day.
#[derive(Debug, Clone)]
pub enum TokenOn<'a> {
    /// The snapshot that governs the day names this row, and the next later
    /// snapshot (if any) names the same contract — or the day is a snapshot
    /// day.
    Resolved {
        /// The snapshot answering.
        snapshot: Stamp,
        /// The row.
        listed: Listed<'a>,
    },
    /// The governing snapshot and the next later one disagree (or the later
    /// one does not list the token): the day lies between them and neither is
    /// believed.
    Ambiguous {
        /// The newest snapshot on or before the day.
        older: Stamp,
        /// The next later snapshot.
        newer: Stamp,
    },
    /// The snapshot that would resolve the token names a contract that expired
    /// before the day: the exchange flushes a token at expiry and reuses it,
    /// so the row cannot name what traded under it that day (R4-28 repair 3,
    /// PREC-10). Never answered as resolved.
    Expired {
        /// The snapshot answering.
        snapshot: Stamp,
        /// The expired row, for a caller that asks what it named, not what
        /// traded that day.
        listed: Listed<'a>,
    },
    /// The governing snapshot does not list the token.
    NotListed {
        /// The snapshot that governs the day.
        snapshot: Stamp,
    },
    /// The governing snapshot carries the token twice.
    Duplicated {
        /// The snapshot.
        snapshot: Stamp,
    },
    /// The day is before the first snapshot: nothing is known, and nothing is
    /// borrowed from a later snapshot.
    BeforeFirst {
        /// The first snapshot.
        first: Stamp,
    },
    /// No receipted snapshot exists.
    NoSnapshot,
    /// A snapshot the answer needs did not check or parse.
    Unreadable {
        /// The snapshot.
        snapshot: Stamp,
        /// Why.
        why: String,
    },
}

impl TokenOn<'_> {
    /// The answer's one-word name, as `/logs` and a refusal carry it.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match *self {
            Self::Resolved { .. } => "resolved",
            Self::Ambiguous { .. } => "token_ambiguous",
            Self::Expired { .. } => "token_expired",
            Self::NotListed { .. } => "token_not_listed",
            Self::Duplicated { .. } => "token_duplicated",
            Self::BeforeFirst { .. } => "before_first_snapshot",
            Self::NoSnapshot => "no_snapshot",
            Self::Unreadable { .. } => "snapshot_unreadable",
        }
    }
}

/// Every receipted Kite snapshot, with the day table that finds the one
/// governing a day by index, loading each snapshot on its first use.
#[derive(Debug)]
pub struct Book {
    root: Option<PathBuf>,
    stamps: Vec<Stamp>,
    first: u32,
    table: Vec<u32>,
    loaded: Vec<OnceLock<Result<Snapshot, String>>>,
    /// The Kite entries the listing named incomplete, oldest first (R4-28
    /// repair 3): never read, and named by every plan over the book.
    incomplete: Vec<Incomplete>,
}

impl Book {
    /// The book over `root`'s receipted Kite snapshots. Nothing is read but
    /// the directory listing; each snapshot is read, checked and indexed the
    /// first time an answer needs it.
    ///
    /// # Errors
    ///
    /// A listing that fails, or stamps spanning more than forty years.
    ///
    /// # Listed under the writers' lock, shared (R4-28 repair 4)
    ///
    /// A body a refresh is still writing has no receipt yet, and a listing
    /// taken beside that write named it cut, a false alarm on the futures
    /// receipt. The listing now holds the writers' lock shared, so it waits
    /// for a write in flight to commit or fail and lets other readers list
    /// beside it: an entry it names incomplete is one no live writer is
    /// finishing.
    pub fn open(root: &Path) -> Result<Self, String> {
        let catalog = read_quiesced(root)?;
        let stamps = catalog.receipted(Kind::Kite).to_vec();
        let loaded = stamps.iter().map(|_| OnceLock::new()).collect();
        let mut book = Self::build(Some(root.to_path_buf()), stamps, loaded)?;
        book.incomplete = catalog.incomplete_of(Kind::Kite);
        Ok(book)
    }

    /// A book over snapshots already parsed (tests and the bench).
    ///
    /// # Errors
    ///
    /// Stamps spanning more than forty years, or two snapshots with one stamp.
    pub fn of(mut snapshots: Vec<Snapshot>) -> Result<Self, String> {
        snapshots.sort_unstable_by_key(Snapshot::stamp);
        let stamps: Vec<Stamp> = snapshots.iter().map(Snapshot::stamp).collect();
        if stamps.windows(2).any(|pair| pair.first() == pair.get(1)) {
            return Err("two snapshots carry one stamp".to_owned());
        }
        let loaded = snapshots
            .into_iter()
            .map(|snapshot| {
                let cell = OnceLock::new();
                let _set = cell.set(Ok(snapshot));
                cell
            })
            .collect();
        Self::build(None, stamps, loaded)
    }

    fn build(
        root: Option<PathBuf>,
        stamps: Vec<Stamp>,
        loaded: Vec<OnceLock<Result<Snapshot, String>>>,
    ) -> Result<Self, String> {
        let (Some(first), Some(last)) = (stamps.first(), stamps.last()) else {
            return Ok(Self {
                root,
                stamps,
                first: 0,
                table: Vec::new(),
                loaded,
                incomplete: Vec::new(),
            });
        };
        let first = first.day.days_from_epoch();
        let span = last.day.days_from_epoch().saturating_sub(first);
        if span > MAX_SPAN_DAYS {
            return Err(format!(
                "the snapshots span {span} days, past the {MAX_SPAN_DAYS}-day bound; a misnamed file is refused"
            ));
        }
        // ONE ENTRY PER CALENDAR DAY: the index of the newest stamp on or
        // before that day. Built in one pass over days and stamps together.
        let mut table = Vec::with_capacity(span as usize + 1);
        let mut at = 0_usize;
        for offset in 0..=span {
            let day = first.saturating_add(offset);
            while stamps
                .get(at + 1)
                .is_some_and(|next| next.day.days_from_epoch() <= day)
            {
                at += 1;
            }
            table.push(
                u32::try_from(at).map_err(|_| "more snapshots than a u32 counts".to_owned())?,
            );
        }
        Ok(Self {
            root,
            stamps,
            first,
            table,
            loaded,
            incomplete: Vec::new(),
        })
    }

    /// The snapshots directory a book opened over; `None` for one built in
    /// memory.
    #[must_use]
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// The stamps, oldest first.
    #[must_use]
    pub fn stamps(&self) -> &[Stamp] {
        &self.stamps
    }

    /// The newest stamp.
    #[must_use]
    pub fn newest(&self) -> Option<Stamp> {
        self.stamps.last().copied()
    }

    /// The Kite entries the listing named incomplete, oldest first: never
    /// read by this book (R4-28 repair 3).
    #[must_use]
    pub fn incomplete(&self) -> &[Incomplete] {
        &self.incomplete
    }

    /// The snapshot at `at`, read on first use.
    ///
    /// # Errors
    ///
    /// An index past the book, or the snapshot's own refusal.
    pub fn snapshot(&self, at: usize) -> Result<&Snapshot, String> {
        let (Some(cell), Some(&stamp)) = (self.loaded.get(at), self.stamps.get(at)) else {
            return Err(format!("no snapshot {at} in this book"));
        };
        cell.get_or_init(|| {
            let root = self
                .root
                .as_deref()
                .ok_or_else(|| "an in-memory book has no root to read from".to_owned())?;
            let body = read_body(root, Kind::Kite, stamp)?;
            Snapshot::parse(stamp, &body)
        })
        .as_ref()
        .map_err(Clone::clone)
    }

    /// Runs `f` over the snapshot at `at`: the cached one when it is loaded,
    /// otherwise read, checked and parsed for this call only and dropped
    /// after, so a walk over many snapshots holds one at a time.
    ///
    /// # Errors
    ///
    /// An index past the book, or the snapshot's own refusal.
    pub fn with_snapshot<T>(&self, at: usize, f: impl FnOnce(&Snapshot) -> T) -> Result<T, String> {
        let (Some(cell), Some(&stamp)) = (self.loaded.get(at), self.stamps.get(at)) else {
            return Err(format!("no snapshot {at} in this book"));
        };
        if let Some(loaded) = cell.get() {
            return loaded.as_ref().map(f).map_err(Clone::clone);
        }
        let root = self
            .root
            .as_deref()
            .ok_or_else(|| "an in-memory book has no root to read from".to_owned())?;
        let snapshot = Snapshot::parse(stamp, &read_body(root, Kind::Kite, stamp)?)?;
        Ok(f(&snapshot))
    }

    /// The index of the snapshot governing `day`, or the first stamp when the
    /// day is before it.
    fn governing(&self, day: Day) -> Result<usize, Stamp> {
        let Some(&first) = self.stamps.first() else {
            return Ok(usize::MAX);
        };
        let number = day.days_from_epoch();
        if number < self.first {
            return Err(first);
        }
        let offset = number - self.first;
        Ok(self
            .table
            .get(offset as usize)
            .map_or(self.stamps.len().saturating_sub(1), |&at| at as usize))
    }

    /// What `(exchange, token)` named on `day`. See the module header for the
    /// rule; one table index and at most two hash probes.
    #[must_use]
    pub fn token_on(&self, day: Day, exchange: &str, token: u64) -> TokenOn<'_> {
        let at = match self.governing(day) {
            Err(first) => return TokenOn::BeforeFirst { first },
            Ok(usize::MAX) => return TokenOn::NoSnapshot,
            Ok(at) => at,
        };
        let Some(&stamp) = self.stamps.get(at) else {
            return TokenOn::NoSnapshot;
        };
        let governing = match self.snapshot(at) {
            Ok(snapshot) => snapshot,
            Err(why) => {
                return TokenOn::Unreadable {
                    snapshot: stamp,
                    why,
                };
            }
        };
        let listed = match governing.lookup(exchange, token) {
            Lookup::Listed(listed) => listed,
            Lookup::Absent => return TokenOn::NotListed { snapshot: stamp },
            Lookup::Duplicated => return TokenOn::Duplicated { snapshot: stamp },
        };
        if stamp.day == day {
            return resolved(stamp, listed, day);
        }
        let Some(next_day) = self.stamps.get(at + 1).map(|next| next.day) else {
            return resolved(stamp, listed, day);
        };
        // THE NEXT LATER DAY'S SNAPSHOT IS ITS NEWEST ORDINAL, as on every
        // other day, never its first: one day-table index (R4-28 repair 1).
        let later_at = self.governing(next_day).unwrap_or(at + 1);
        let newer = self.stamps.get(later_at).copied().unwrap_or(stamp);
        let later = match self.snapshot(later_at) {
            Ok(snapshot) => snapshot,
            Err(why) => {
                return TokenOn::Unreadable {
                    snapshot: newer,
                    why,
                };
            }
        };
        match later.lookup(exchange, token) {
            Lookup::Listed(again) if again.same_contract(&listed) => resolved(stamp, listed, day),
            _ => TokenOn::Ambiguous {
                older: stamp,
                newer,
            },
        }
    }
}

/// A resolving row, unless its contract expired before `day`: then
/// [`TokenOn::Expired`], since a token is flushed at expiry and reused, and a
/// row past its own expiry cannot name what traded under the token that day
/// (R4-28 repair 3, PREC-10). The expiry day itself still resolves.
fn resolved(snapshot: Stamp, listed: Listed<'_>, day: Day) -> TokenOn<'_> {
    if listed.expiry().is_some_and(|expiry| expiry < day) {
        TokenOn::Expired { snapshot, listed }
    } else {
        TokenOn::Resolved { snapshot, listed }
    }
}

// ─────────────────────────────────── ISINs ───────────────────────────────────

/// NSE EQ symbol → ISIN, from one dated NSE Total Market snapshot.
#[derive(Debug, Clone, Default)]
pub struct Isins {
    by_symbol: HashMap<Box<str>, Box<str>>,
}

impl Isins {
    /// Reads NSE's constituent file (`Company Name,Industry,Symbol,Series,ISIN
    /// Code`), keeping the EQ series rows.
    ///
    /// # Errors
    ///
    /// Whatever [`crate::nse::constituents_strict`] refuses.
    pub fn parse(csv: &str) -> Result<Self, String> {
        let rows = crate::nse::constituents_strict(csv).map_err(|why| why.to_string())?;
        let by_symbol = rows
            .iter()
            .filter(|row| row.series.trim() == "EQ")
            .map(|row| (row.symbol.trim().into(), row.isin.trim().into()))
            .collect();
        Ok(Self { by_symbol })
    }

    /// The ISIN the file lists for `symbol`.
    #[must_use]
    pub fn of(&self, symbol: &str) -> Option<&str> {
        self.by_symbol.get(symbol).map(|isin| &**isin)
    }
}

/// The ISINs of the newest receipted NSE snapshot ON `day`, or `None` when
/// that day has none (then every share's ISIN is unknown that day).
fn isins_on(root: &Path, catalog: &Catalog, day: Day) -> Result<Option<Isins>, String> {
    let Some(stamp) = catalog.of_day(Kind::NseTotalMarket, day) else {
        return Ok(None);
    };
    let body = read_body(root, Kind::NseTotalMarket, stamp)?;
    Isins::parse(&body).map(Some)
}

// ───────────────────────────────── the diff ─────────────────────────────────

/// One kind of master change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Change {
    /// A key the previous snapshot did not hold.
    Listing,
    /// A key the new snapshot no longer holds.
    Delisting,
    /// The same instrument under a new trading symbol: the same token, or an
    /// NSE share with the same ISIN.
    Symbol,
    /// An NSE share whose same-date ISIN changed.
    Isin,
    /// A lot size changed.
    LotSize,
    /// An expiry changed under the same token.
    Expiry,
    /// The same key under a new token.
    Token,
    /// An NSE share the same-date NSE file does not list: its ISIN is
    /// unknown that day and is never inferred. Named the day it becomes
    /// unknown (known on the old date) or appears (a new key); the day's
    /// total is [`DiffSummary::isin_unknown_total`].
    IsinUnknown,
}

impl Change {
    /// Every kind, in record order.
    pub const ALL: [Self; 8] = [
        Self::Listing,
        Self::Delisting,
        Self::Symbol,
        Self::Isin,
        Self::LotSize,
        Self::Expiry,
        Self::Token,
        Self::IsinUnknown,
    ];

    /// The record's first field.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Listing => "listing",
            Self::Delisting => "delisting",
            Self::Symbol => "symbol",
            Self::Isin => "isin",
            Self::LotSize => "lot_size",
            Self::Expiry => "expiry",
            Self::Token => "token",
            Self::IsinUnknown => "isin_unknown",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|change| change.word() == word)
    }

    /// Whether every change of this kind is named in the refresh outcome and
    /// on `/logs` (listings, delistings and unknown ISINs are counted there
    /// and named in the `.diff`).
    #[must_use]
    pub const fn named(self) -> bool {
        matches!(
            self,
            Self::Symbol | Self::Isin | Self::LotSize | Self::Expiry | Self::Token
        )
    }
}

/// One record of a diff.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Item {
    /// What changed.
    pub change: Change,
    /// The exchange.
    pub exchange: String,
    /// The token the new snapshot carries (the old one's for a delisting).
    pub token: u64,
    /// The trading symbol the record is about.
    pub key: String,
    /// The value before (empty for a listing).
    pub old: String,
    /// The value after (empty for a delisting).
    pub new: String,
}

/// The diff key of a row.
type Key<'a> = (&'a str, &'a str, Option<Day>, &'a str, &'a str);

fn key<'a>(listed: &Listed<'a>) -> Key<'a> {
    (
        listed.exchange(),
        listed.tradingsymbol(),
        listed.expiry(),
        listed.strike(),
        listed.instrument_type(),
    )
}

fn day_text(day: Option<Day>) -> String {
    day.map_or_else(String::new, |day| day.to_string())
}

fn item(change: Change, listed: &Listed<'_>, old: String, new: String) -> Item {
    Item {
        change,
        exchange: listed.exchange().to_owned(),
        token: listed.token(),
        key: listed.tradingsymbol().to_owned(),
        old,
        new,
    }
}

/// Every change from `old` to `new`, sorted. ISINs come from the NSE file of
/// each snapshot's own date; `None` means that date has none.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one pass over the new rows with its three matching arms side by side, in the \
              order they are tried; split apart, each arm would sit away from the reason it \
              comes before the next"
)]
pub fn diff(
    old: &Snapshot,
    new: &Snapshot,
    old_isins: Option<&Isins>,
    new_isins: Option<&Isins>,
) -> Vec<Item> {
    let old_rows: Vec<Listed<'_>> = old.rows().collect();
    let new_rows: Vec<Listed<'_>> = new.rows().collect();
    let old_keys: HashMap<Key<'_>, usize> = old_rows
        .iter()
        .enumerate()
        .map(|(at, listed)| (key(listed), at))
        .collect();
    let new_keys: HashSet<Key<'_>> = new_rows.iter().map(key).collect();
    let mut old_tokens: HashMap<(&str, u64), usize> = HashMap::new();
    for (at, listed) in old_rows.iter().enumerate() {
        old_tokens
            .entry((listed.exchange(), listed.token()))
            .or_insert(at);
    }
    let isin_of = |isins: Option<&Isins>, listed: &Listed<'_>| {
        isins
            .and_then(|isins| isins.of(listed.tradingsymbol()))
            .map(str::to_owned)
    };
    // OLD SHARES THAT ARE GONE, BY THEIR OWN DATE'S ISIN: the other half of a
    // rename that kept the ISIN and changed the token.
    let mut gone_by_isin: HashMap<String, usize> = HashMap::new();
    for (at, listed) in old_rows.iter().enumerate() {
        if listed.is_nse_share()
            && !new_keys.contains(&key(listed))
            && let Some(isin) = isin_of(old_isins, listed)
        {
            gone_by_isin.entry(isin).or_insert(at);
        }
    }
    let mut matched = vec![false; old_rows.len()];
    let mut out = Vec::new();
    for listed in &new_rows {
        if let Some(&at) = old_keys.get(&key(listed)) {
            let Some(before) = old_rows.get(at) else {
                continue;
            };
            if let Some(slot) = matched.get_mut(at) {
                *slot = true;
            }
            if before.lot_size() != listed.lot_size() {
                out.push(item(
                    Change::LotSize,
                    listed,
                    before.lot_size().to_string(),
                    listed.lot_size().to_string(),
                ));
            }
            if before.token() != listed.token() {
                out.push(item(
                    Change::Token,
                    listed,
                    before.token().to_string(),
                    listed.token().to_string(),
                ));
            }
            if listed.is_nse_share() {
                match (isin_of(old_isins, before), isin_of(new_isins, listed)) {
                    (Some(was), Some(now)) if was != now => {
                        out.push(item(Change::Isin, listed, was, now));
                    }
                    // KNOWN ON THE OLD DATE, UNKNOWN ON THE NEW: named the day
                    // it happens; never carried over from the old date.
                    (Some(_), None) => {
                        out.push(item(
                            Change::IsinUnknown,
                            listed,
                            String::new(),
                            String::new(),
                        ));
                    }
                    _ => {}
                }
            }
            continue;
        }
        // THE SAME TOKEN, A NEW SPELLING: one instrument re-named or re-dated,
        // when its type, strike and name are unchanged and its old key is gone.
        if let Some(&at) = old_tokens.get(&(listed.exchange(), listed.token()))
            && let Some(before) = old_rows.get(at)
            && !matched.get(at).copied().unwrap_or(true)
            && !new_keys.contains(&key(before))
            && before.instrument_type() == listed.instrument_type()
            && before.strike() == listed.strike()
            && before.name() == listed.name()
        {
            if let Some(slot) = matched.get_mut(at) {
                *slot = true;
            }
            if before.tradingsymbol() != listed.tradingsymbol() {
                out.push(item(
                    Change::Symbol,
                    listed,
                    before.tradingsymbol().to_owned(),
                    listed.tradingsymbol().to_owned(),
                ));
            }
            if before.expiry() != listed.expiry() {
                out.push(item(
                    Change::Expiry,
                    listed,
                    day_text(before.expiry()),
                    day_text(listed.expiry()),
                ));
            }
            if before.lot_size() != listed.lot_size() {
                out.push(item(
                    Change::LotSize,
                    listed,
                    before.lot_size().to_string(),
                    listed.lot_size().to_string(),
                ));
            }
            unknown_if_new(&mut out, listed, new_isins);
            continue;
        }
        // THE SAME ISIN UNDER A NEW SYMBOL, from each date's own NSE file.
        if listed.is_nse_share()
            && let Some(isin) = isin_of(new_isins, listed)
            && let Some(&at) = gone_by_isin.get(&isin)
            && let Some(before) = old_rows.get(at)
            && !matched.get(at).copied().unwrap_or(true)
        {
            if let Some(slot) = matched.get_mut(at) {
                *slot = true;
            }
            out.push(item(
                Change::Symbol,
                listed,
                before.tradingsymbol().to_owned(),
                listed.tradingsymbol().to_owned(),
            ));
            continue;
        }
        out.push(item(
            Change::Listing,
            listed,
            String::new(),
            listed.tradingsymbol().to_owned(),
        ));
        unknown_if_new(&mut out, listed, new_isins);
    }
    for (at, before) in old_rows.iter().enumerate() {
        if !matched.get(at).copied().unwrap_or(true) && !new_keys.contains(&key(before)) {
            out.push(item(
                Change::Delisting,
                before,
                before.tradingsymbol().to_owned(),
                String::new(),
            ));
        }
    }
    out.sort_unstable();
    out
}

/// A share appearing under a new key with no same-date ISIN is named
/// `isin_unknown` the day it appears.
fn unknown_if_new(out: &mut Vec<Item>, listed: &Listed<'_>, new_isins: Option<&Isins>) {
    if listed.is_nse_share()
        && new_isins
            .and_then(|isins| isins.of(listed.tradingsymbol()))
            .is_none()
    {
        out.push(item(
            Change::IsinUnknown,
            listed,
            String::new(),
            String::new(),
        ));
    }
}

/// How many NSE shares of `new` the NSE file of its own date does not list:
/// each has an unknown ISIN that day, whether or not the diff names it.
#[must_use]
pub fn unknown_isins(new: &Snapshot, new_isins: Option<&Isins>) -> u64 {
    let count = new
        .rows()
        .filter(|listed| {
            listed.is_nse_share()
                && new_isins
                    .and_then(|isins| isins.of(listed.tradingsymbol()))
                    .is_none()
        })
        .count();
    u64::try_from(count).unwrap_or(u64::MAX)
}

/// Field widths of one record: kind, exchange, token, key, old, new.
const WIDTHS: [usize; 6] = [12, 8, 20, 48, 40, 40];

/// One fixed-width line, or why a value does not fit.
fn fixed(fields: [&str; 6]) -> Result<String, String> {
    let mut line = String::with_capacity(RECORD_LEN);
    for (value, width) in fields.into_iter().zip(WIDTHS) {
        if value.len() > width || value.contains(['|', '\n', '\r']) {
            return Err(format!(
                "{value:?} does not fit a {width}-byte diff field; the diff was not written"
            ));
        }
        let _infallible = write!(line, "{value:<width$}|");
    }
    if line.len() >= RECORD_LEN {
        return Err("a record is wider than the record length".to_owned());
    }
    let pad = RECORD_LEN - 1 - line.len();
    line.extend(core::iter::repeat_n(' ', pad));
    line.push('\n');
    Ok(line)
}

/// The header line: the format, the two snapshots, the record count and how
/// many NSE shares of the current snapshot have no same-date ISIN.
fn header_line(head: &DiffHead) -> Result<String, String> {
    let line = format!(
        "{DIFF_FORMAT}|previous={}|current={}|records={}|isin_unknown_total={}|",
        head.previous, head.current, head.records, head.isin_unknown_total
    );
    if line.len() >= RECORD_LEN {
        return Err("the diff header is wider than the record length".to_owned());
    }
    let mut line = line;
    line.extend(core::iter::repeat_n(' ', RECORD_LEN - 1 - line.len()));
    line.push('\n');
    Ok(line)
}

/// A `.diff` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHead {
    /// The snapshot compared against, as its stamp prints.
    pub previous: String,
    /// The snapshot compared.
    pub current: String,
    /// How many records follow.
    pub records: usize,
    /// How many NSE shares of the current snapshot the NSE file of its own
    /// date does not list (named only the day each became unknown).
    pub isin_unknown_total: u64,
}

/// The `.diff` bytes for `items`.
///
/// # Errors
///
/// A value wider than its field, or holding a separator: named, and nothing
/// is written.
pub fn render_diff(
    previous: Stamp,
    current: Stamp,
    items: &[Item],
    isin_unknown_total: u64,
) -> Result<String, String> {
    let mut out = header_line(&DiffHead {
        previous: previous.to_string(),
        current: current.to_string(),
        records: items.len(),
        isin_unknown_total,
    })?;
    for one in items {
        let token = one.token.to_string();
        out.push_str(&fixed([
            one.change.word(),
            &one.exchange,
            &token,
            &one.key,
            &one.old,
            &one.new,
        ])?);
    }
    Ok(out)
}

/// The value of one `name=value` header field.
fn header_field<'a>(part: Option<&'a str>, name: &str) -> Result<&'a str, String> {
    part.and_then(|part| part.strip_prefix(name))
        .and_then(|rest| rest.strip_prefix('='))
        .ok_or_else(|| format!("the diff header carries no {name}"))
}

/// A `.diff` read back: its header and its records.
///
/// # Errors
///
/// A file that is not whole fixed-length lines of this format, named.
pub fn parse_diff(text: &str) -> Result<(DiffHead, Vec<Item>), String> {
    if !text.len().is_multiple_of(RECORD_LEN) {
        return Err(format!(
            "a diff is whole {RECORD_LEN}-byte lines; this one is torn"
        ));
    }
    let mut lines = text.as_bytes().chunks(RECORD_LEN).map(|line| {
        core::str::from_utf8(line)
            .map_err(|_| "a diff line is not UTF-8".to_owned())
            .map(|line| line.trim_end_matches('\n'))
    });
    let header = lines.next().ok_or_else(|| "an empty diff".to_owned())??;
    let mut parts = header.split('|');
    if parts.next() != Some(DIFF_FORMAT) {
        return Err(format!("not a {DIFF_FORMAT} header"));
    }
    let head = DiffHead {
        previous: header_field(parts.next(), "previous")?.to_owned(),
        current: header_field(parts.next(), "current")?.to_owned(),
        records: header_field(parts.next(), "records")?
            .parse()
            .map_err(|_| "the diff header's record count is not a number".to_owned())?,
        isin_unknown_total: header_field(parts.next(), "isin_unknown_total")?
            .parse()
            .map_err(|_| "the diff header's unknown-ISIN total is not a number".to_owned())?,
    };
    let mut items = Vec::with_capacity(head.records);
    for line in lines {
        let line = line?;
        let fields: Vec<&str> = line.split('|').map(str::trim_end).collect();
        let (Some(word), Some(exchange), Some(token), Some(key), Some(old), Some(new)) = (
            fields.first(),
            fields.get(1),
            fields.get(2),
            fields.get(3),
            fields.get(4),
            fields.get(5),
        ) else {
            return Err("a diff record has fewer than six fields".to_owned());
        };
        items.push(Item {
            change: Change::parse(word).ok_or_else(|| format!("{word:?} is not a change"))?,
            exchange: (*exchange).to_owned(),
            token: token
                .parse()
                .map_err(|_| format!("{token:?} is not a token"))?,
            key: (*key).to_owned(),
            old: (*old).to_owned(),
            new: (*new).to_owned(),
        });
    }
    if items.len() != head.records {
        return Err(format!(
            "the header counts {} records and the file holds {}",
            head.records,
            items.len()
        ));
    }
    Ok((head, items))
}

/// One diff, as the refresh outcome reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSummary {
    /// The snapshot compared against.
    pub previous: Stamp,
    /// The snapshot compared.
    pub current: Stamp,
    /// The `.diff` file.
    pub file: String,
    /// How many records of each kind, in [`Change::ALL`] order.
    pub counts: [u64; 8],
    /// How many NSE shares of `current` have no same-date ISIN.
    pub isin_unknown_total: u64,
    /// Every record of a [`Change::named`] kind.
    pub named: Vec<Item>,
}

impl DiffSummary {
    fn of(
        previous: Stamp,
        current: Stamp,
        file: String,
        items: &[Item],
        isin_unknown_total: u64,
    ) -> Self {
        let mut counts = [0_u64; 8];
        for one in items {
            if let Some(at) = Change::ALL.iter().position(|change| *change == one.change)
                && let Some(count) = counts.get_mut(at)
            {
                *count = count.saturating_add(1);
            }
        }
        Self {
            previous,
            current,
            file,
            counts,
            isin_unknown_total,
            named: items
                .iter()
                .filter(|one| one.change.named())
                .cloned()
                .collect(),
        }
    }

    /// The named changes an outcome and the events carry, at most
    /// [`DIFF_NAMED`] in the diff's own order, and how many more are counted
    /// only (every one is in the `.diff` file).
    #[must_use]
    pub fn named_bounded(&self) -> (&[Item], usize) {
        let shown = self.named.len().min(DIFF_NAMED);
        (
            self.named.get(..shown).unwrap_or_default(),
            self.named.len().saturating_sub(shown),
        )
    }

    /// `listing=1 delisting=0 ... isin_unknown_total=9`, the counts as one
    /// line.
    #[must_use]
    pub fn counts_line(&self) -> String {
        let mut line = Change::ALL
            .iter()
            .zip(self.counts)
            .map(|(change, count)| format!("{}={count}", change.word()))
            .collect::<Vec<_>>()
            .join(" ");
        let _infallible = write!(line, " isin_unknown_total={}", self.isin_unknown_total);
        line
    }
}

/// The diff of the Kite snapshot `current` against the receipted one before
/// it: written once, or read back when the day already holds it.
///
/// `Ok(None)` when `current` is the first Kite snapshot and there is nothing
/// to compare.
///
/// # Errors
///
/// A snapshot or NSE file that does not check or parse, a value that does not
/// fit a record, and a file that cannot be written or read, each named.
pub fn diff_for(root: &Path, current: Stamp) -> Result<Option<DiffSummary>, String> {
    let catalog = Catalog::read(root)?;
    let Some(previous) = catalog.before(Kind::Kite, current) else {
        return Ok(None);
    };
    let path = root.join(diff_name(current));
    let file = path.display().to_string();
    if let Some(summary) = held_diff(&path, &file, previous, current)? {
        return Ok(Some(summary));
    }
    let old = Snapshot::parse(previous, &read_body(root, Kind::Kite, previous)?)?;
    let new = Snapshot::parse(current, &read_body(root, Kind::Kite, current)?)?;
    let old_isins = isins_on(root, &catalog, previous.day)?;
    let new_isins = isins_on(root, &catalog, current.day)?;
    let items = diff(&old, &new, old_isins.as_ref(), new_isins.as_ref());
    let total = unknown_isins(&new, new_isins.as_ref());
    let rendered = render_diff(previous, current, &items, total)?;
    let held = lock(root)?;
    // ANOTHER WRITER FINISHED FIRST: its file is the diff, read it. An entry
    // at the name with nothing readable behind it is refused, never written
    // over.
    if fs::symlink_metadata(&path).is_ok() {
        let summary = held_diff(&path, &file, previous, current);
        held.release().map_err(|why| why.to_string())?;
        return summary?.map(Some).ok_or_else(|| {
            format!(
                "{file} is a name with nothing readable behind it; the diff is not written over it"
            )
        });
    }
    commit_diff(root, current, &path, rendered.as_bytes())?;
    sync_dir(root)?;
    held.release().map_err(|why| why.to_string())?;
    Ok(Some(DiffSummary::of(
        previous, current, file, &items, total,
    )))
}

/// The diff already at `path`, read and parsed, or `None` when there is none.
///
/// # Errors
///
/// A file that cannot be read, is not UTF-8 or does not parse, named.
fn held_diff(
    path: &Path,
    file: &str,
    previous: Stamp,
    current: Stamp,
) -> Result<Option<DiffSummary>, String> {
    match fs::read(path) {
        Ok(bytes) => {
            let text = String::from_utf8(bytes).map_err(|_| format!("{file} is not UTF-8"))?;
            let (head, items) = parse_diff(&text).map_err(|why| format!("{file}: {why}"))?;
            Ok(Some(DiffSummary::of(
                previous,
                current,
                file.to_owned(),
                &items,
                head.isin_unknown_total,
            )))
        }
        Err(why) if why.kind() == ErrorKind::NotFound => Ok(None),
        Err(why) => Err(format!("{file} could not be read: {why}")),
    }
}

/// Writes the diff of `current` whole under its first free partial name,
/// synced, and renames it to `path`, under the writers' lock the caller holds
/// (R4-28 repair 4, the r4 check's follow-up; open since r1).
///
/// A kill mid-write leaves a partial that [`Catalog`] names and no reader
/// opens, never a torn `.diff` under the diff's own name, so the next refresh
/// makes the diff again, whole. A partial already at a name is a killed
/// writer's (the lock admits one writer at a time), so it is stepped past and
/// left exactly as found: never opened, overwritten or deleted.
///
/// # Errors
///
/// A partial that cannot be created, written, synced or renamed, or every
/// partial name up to [`MAX_ORDINAL`] already held, named.
fn commit_diff(root: &Path, current: Stamp, path: &Path, bytes: &[u8]) -> Result<(), String> {
    let (partial, mut file) = open_diff_partial(root, current)?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|why| format!("{} is incomplete and retained: {why}", partial.display()))?;
    fs::rename(&partial, path).map_err(|why| {
        format!(
            "{} could not be committed: {why}; the diff stays partial and is never read",
            path.display()
        )
    })
}

/// Creates the first free partial name of `current`'s diff, `O_EXCL`.
fn open_diff_partial(root: &Path, current: Stamp) -> Result<(PathBuf, File), String> {
    for attempt in 1..=MAX_ORDINAL {
        let partial = root.join(diff_partial_name(current, attempt));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
        {
            Ok(file) => return Ok((partial, file)),
            Err(why) if why.kind() == ErrorKind::AlreadyExists => {}
            Err(why) => {
                return Err(format!(
                    "{} could not be created; no existing file was touched: {why}",
                    partial.display()
                ));
            }
        }
    }
    Err(format!(
        "{}: every partial name up to _{MAX_ORDINAL} is held by a cut write; the diff is not \
         written, and every refresh names each of them",
        root.join(diff_name(current)).display()
    ))
}

// ──────────────────────────── the landing hook ────────────────────────────

/// What the landing hook did for one source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshotted {
    /// Which master.
    pub kind: Kind,
    /// The day it was snapshotted under; `None` when the clock named none.
    pub day: Option<Day>,
    /// The snapshot written or already held, or why none was.
    pub wrote: Result<Wrote, String>,
    /// The diff against the previous Kite snapshot: `None` for the NSE file,
    /// `Some(Ok(None))` for the first Kite snapshot.
    pub diff: Option<Result<Option<DiffSummary>, String>>,
    /// Every entry of this kind that is not a snapshot (a write cut by a kill
    /// before its receipt, a receipt cut before its rename), oldest first, or
    /// why the directory could not be listed to name them (R4-28 repair 3).
    pub incomplete: Result<Vec<Incomplete>, String>,
}

/// A JSON string literal.
fn json_string(text: &str) -> String {
    serde_json::Value::String(text.to_owned()).to_string()
}

impl Snapshotted {
    /// The outcome as the `/masters/refresh` answer carries it.
    #[must_use]
    pub fn json(&self) -> String {
        let wrote = match self.wrote {
            Ok(wrote) => format!(
                r#""wrote":{},"stamp":{},"file":{},"refusal":null"#,
                json_string(wrote.word()),
                json_string(&wrote.stamp().to_string()),
                json_string(&file_name(self.kind, wrote.stamp())),
            ),
            Err(ref why) => format!(
                r#""wrote":"refused","stamp":null,"file":null,"refusal":{}"#,
                json_string(why)
            ),
        };
        let diff = match self.diff {
            None => "null".to_owned(),
            Some(Ok(None)) => {
                r#"{"previous":null,"note":"the first snapshot: nothing to compare"}"#.to_owned()
            }
            Some(Err(ref why)) => format!(r#"{{"refusal":{}}}"#, json_string(why)),
            Some(Ok(Some(ref summary))) => {
                let counts: Vec<String> = Change::ALL
                    .iter()
                    .zip(summary.counts)
                    .map(|(change, count)| format!("{}:{count}", json_string(change.word())))
                    .collect();
                let (named, unnamed) = summary.named_bounded();
                let named: Vec<String> = named
                    .iter()
                    .map(|one| {
                        format!(
                            r#"{{"change":{},"exchange":{},"token":{},"symbol":{},"old":{},"new":{}}}"#,
                            json_string(one.change.word()),
                            json_string(&one.exchange),
                            one.token,
                            json_string(&one.key),
                            json_string(&one.old),
                            json_string(&one.new),
                        )
                    })
                    .collect();
                format!(
                    r#"{{"previous":{},"current":{},"file":{},"counts":{{{}}},"isin_unknown_total":{},"named":[{}],"named_total":{},"unnamed":{unnamed}}}"#,
                    json_string(&summary.previous.to_string()),
                    json_string(&summary.current.to_string()),
                    json_string(&summary.file),
                    counts.join(","),
                    summary.isin_unknown_total,
                    named.join(","),
                    summary.named.len(),
                )
            }
        };
        format!(
            r#"{{"kind":{},"day":{},{wrote},"diff":{diff},"incomplete":{}}}"#,
            json_string(self.kind.stem()),
            self.day
                .map_or_else(|| "null".to_owned(), |day| json_string(&day.to_string())),
            self.incomplete_json(),
        )
    }

    /// `{"held":N,"named":[...],"unnamed":M,"refusal":null}`: every entry that
    /// is not a snapshot, newest first, at most [`INCOMPLETE_NAMED`] named and
    /// the rest counted.
    fn incomplete_json(&self) -> String {
        let entries = match self.incomplete {
            Ok(ref entries) => entries,
            Err(ref why) => {
                return format!(
                    r#"{{"held":null,"named":[],"unnamed":null,"refusal":{}}}"#,
                    json_string(why)
                );
            }
        };
        let (named, unnamed) = newest_named(entries);
        let named: Vec<String> = named
            .map(|one| {
                format!(
                    r#"{{"file":{},"stamp":{},"why":{}}}"#,
                    json_string(&one.file),
                    json_string(&one.stamp.to_string()),
                    json_string(one.why),
                )
            })
            .collect();
        format!(
            r#"{{"held":{},"named":[{}],"unnamed":{unnamed},"refusal":null}}"#,
            entries.len(),
            named.join(","),
        )
    }
}

/// The landing hook: after `masters::land`, snapshot what landed and diff it.
///
/// `None` for a source that is not snapshotted. A master that did not land
/// writes no snapshot (the newest one stands) and says so. Every outcome is
/// one [`EVENT_TARGET`] event, every named change one more, and every entry
/// of the kind that is not a snapshot one Warn event more (R4-28 repair 3).
/// Those entries are named on every path, a refused one included: a refresh
/// that writes nothing still lists the directory once to name them.
#[must_use]
pub fn after_land(
    dir: &Path,
    source: &Source,
    body: &str,
    landed: &Landed,
    day: Result<Day, String>,
) -> Option<Snapshotted> {
    let kind = Kind::of(source)?;
    let root = dir.join(SNAPSHOT_DIR);
    let written = match (landed, &day) {
        (Landed::Written { .. } | Landed::Uncertain { .. }, Ok(day)) => {
            write_snapshot(&root, kind, *day, source.url, body.as_bytes())
        }
        (Landed::Written { .. } | Landed::Uncertain { .. }, Err(why)) => unwritten(
            &root,
            kind,
            format!("the master landed and no snapshot was written: {why}"),
        ),
        (Landed::Refused(why), _) => unwritten(
            &root,
            kind,
            format!(
                "the master did not land, so no snapshot was written and the newest one stands: {why}"
            ),
        ),
    };
    let diff = match (kind, &written.wrote) {
        (Kind::Kite, Ok(wrote)) => Some(diff_for(&root, wrote.stamp())),
        (Kind::Kite, Err(_)) | (Kind::NseTotalMarket, _) => None,
    };
    let outcome = Snapshotted {
        kind,
        day: day.ok(),
        wrote: written.wrote,
        diff,
        incomplete: written.incomplete,
    };
    note(&outcome);
    Some(outcome)
}

/// A refusal that wrote nothing, with the kind's incomplete entries named
/// from one listing.
fn unwritten(root: &Path, kind: Kind, why: String) -> Written {
    Written {
        wrote: Err(why),
        incomplete: Catalog::read(root).map(|catalog| catalog.incomplete_of(kind)),
    }
}

/// Why a new snapshot took the ordinal it did, when the day's earlier
/// ordinals hold incomplete entries: empty otherwise.
fn stepped_past(outcome: &Snapshotted) -> String {
    let (Ok(Wrote::New(stamp)), Ok(entries)) = (&outcome.wrote, &outcome.incomplete) else {
        return String::new();
    };
    let cut: Vec<&str> = entries
        .iter()
        .filter(|one| one.stamp.day == stamp.day && one.stamp.ordinal < stamp.ordinal)
        .map(|one| one.file.as_str())
        .collect();
    if cut.is_empty() {
        return String::new();
    }
    format!(
        "ordinal {}: {} entries of the day's earlier ordinals are incomplete and never read: {}",
        stamp.ordinal,
        cut.len(),
        cut.join(", ")
    )
}

/// The events of one hook call.
fn note(outcome: &Snapshotted) {
    note_incomplete(outcome.kind, &outcome.incomplete);
    let (level, message, detail) = match (&outcome.wrote, &outcome.diff) {
        (Err(why), _) => (
            telemetry::Level::Error,
            "no master snapshot was written",
            why.clone(),
        ),
        (Ok(_), Some(Err(why))) => (
            telemetry::Level::Error,
            "the master was snapshotted, and its diff could not be made",
            why.clone(),
        ),
        (Ok(Wrote::New(_)), _) => (
            telemetry::Level::Info,
            "a dated master snapshot was written",
            stepped_past(outcome),
        ),
        (Ok(Wrote::AlreadyHeld(_)), _) => (
            telemetry::Level::Info,
            "the day already holds this master, byte for byte; nothing was written",
            String::new(),
        ),
    };
    let stamp = outcome
        .wrote
        .as_ref()
        .map_or_else(|_| String::new(), |wrote| wrote.stamp().to_string());
    let (previous, counts, file) = match outcome.diff {
        Some(Ok(Some(ref summary))) => (
            summary.previous.to_string(),
            summary.counts_line(),
            summary.file.clone(),
        ),
        Some(Ok(None)) => (String::new(), "first snapshot".to_owned(), String::new()),
        _ => (String::new(), String::new(), String::new()),
    };
    let day = outcome.day.map_or_else(String::new, |day| day.to_string());
    let incomplete = outcome
        .incomplete
        .as_ref()
        .map_or(0, |entries| entries.len() as u64);
    let _ = telemetry::emit_if!(
        level,
        EVENT_TARGET,
        message,
        "kind" => telemetry::Value::Str(outcome.kind.stem()),
        "day" => telemetry::Value::Str(&day),
        "stamp" => telemetry::Value::Str(&stamp),
        "outcome" => telemetry::Value::Str(outcome.wrote.as_ref().map_or("refused", |wrote| wrote.word())),
        "previous" => telemetry::Value::Str(&previous),
        "counts" => telemetry::Value::Str(&counts),
        "diff" => telemetry::Value::Str(&file),
        "detail" => telemetry::Value::Str(&detail),
        "incomplete" => telemetry::Value::Uint(incomplete),
    );
    if let Some(Ok(Some(ref summary))) = outcome.diff {
        let (named, unnamed) = summary.named_bounded();
        if unnamed > 0 {
            let _ = telemetry::emit_if!(
                telemetry::Level::Warn,
                EVENT_TARGET,
                "more master changes than one refresh names; the rest are counted here and every one is in the .diff file",
                "named" => telemetry::Value::Uint(named.len() as u64),
                "unnamed" => telemetry::Value::Uint(unnamed as u64),
                "diff" => telemetry::Value::Str(&summary.file),
                "day" => telemetry::Value::Str(&day),
            );
        }
        for one in named {
            let token = one.token.to_string();
            let _ = telemetry::emit_if!(
                telemetry::Level::Warn,
                EVENT_TARGET,
                "a master change between two snapshots",
                "change" => telemetry::Value::Str(one.change.word()),
                "exchange" => telemetry::Value::Str(&one.exchange),
                "token" => telemetry::Value::Str(&token),
                "symbol" => telemetry::Value::Str(&one.key),
                "old" => telemetry::Value::Str(&one.old),
                "new" => telemetry::Value::Str(&one.new),
                "day" => telemetry::Value::Str(&day),
            );
        }
    }
}

#[path = "master_snapshots_first_listed.rs"]
pub mod first_listed;

#[cfg(test)]
#[path = "master_snapshots_tests.rs"]
mod tests;

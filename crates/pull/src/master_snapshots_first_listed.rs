//! The futures first-listed index: for each future a dated Kite snapshot
//! lists, the first snapshot day that listed it. R4-28 repair 1; D-6894.
//!
//! # Why it exists
//!
//! [`crate::fnowork::futures_plan`] asks each contract from the first snapshot
//! that lists it. It used to find that day by opening, decompressing and
//! parsing every receipted snapshot on or before the window's end, on every
//! plan and for every underlying, keeping nothing between runs. A second agent
//! measured it on copies of the real 108,915-row dump (dev profile, NIFTY):
//! 240 ms at 1 snapshot, 1,331 ms at 10 and 5,008 ms at 40, per underlying per
//! run — linear in the snapshots held, with no bound. The day a receipted
//! snapshot first listed a contract never changes, so it is written down once,
//! when the snapshot is written, and read back from here.
//!
//! # Layout
//!
//! Under the snapshots directory, in [`DIR`]:
//!
//! - `<EXCHANGE>/<NAME>.idx`, one file per (exchange, underlying name), both
//!   percent-encoded: one line per `(type, expiry, token)` the first time a
//!   snapshot lists it, and again only when an older snapshot taken in later
//!   lists it earlier. The reader takes the earliest day per key.
//! - [`ABSORBED`]: one line per Kite snapshot taken in, for one (exchange,
//!   type), written after every line it added is synced. **It is the commit
//!   marker**, as a receipt is a snapshot's.
//!
//! Both are append-only: no line is ever rewritten. Every line ends in a check
//! over its own bytes, so a line cut by a kill mid-append never reads as a
//! record: it is skipped and counted, and the next append starts a new line.
//!
//! # Crash
//!
//! A kill after a snapshot's receipt and before its [`ABSORBED`] line leaves
//! the snapshot not taken in: the next writer or plan, under the writers' lock,
//! takes it in again. The lines it had already added are true (the snapshot is
//! receipted and listed them), so the repeat adds duplicates the reader folds,
//! never a wrong day.
//!
//! # What is not taken in, and how it is named
//!
//! A snapshot whose receipt or body does not check is not recorded: every plan
//! that needs it names it and retries it, since a read can fail for a reason
//! that passes. A snapshot whose checked body does not parse is recorded as
//! unparseable, with why, once: its bytes are fixed by their receipt, so the
//! parse could only refuse them again. A future row with a field a line cannot
//! carry (a separator or line break, an empty name, a name too long for a file
//! name, no expiry) is counted on its [`ABSORBED`] line and in the event,
//! never dropped silently.
//!
//! # Cost
//!
//! | operation | cost |
//! |---|---|
//! | [`listing`], every snapshot taken in | the absorbed log (one 60-byte line per snapshot) and one underlying's file (one line per contract it ever listed); no snapshot is opened |
//! | taking one snapshot in | one read and parse of that snapshot, and one read and append per underlying with a new contract — once per snapshot, when it is written |

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write as _};
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::{
    Catalog, EVENT_TARGET, Kind, Snapshot, Stamp, base, lock, parse_compact, parse_dashed,
    read_body, read_limited, sync_dir,
};
use crate::session::Day;
use crate::vendor::ContinuousSpec;

/// The index's directory, inside the snapshots directory.
pub const DIR: &str = "futures-first-listed";

/// The commit log: one line per snapshot taken in.
pub const ABSORBED: &str = "absorbed.idx";

/// The first field of every line. A line of another format is not a record.
const TAG: &str = "fl1";

/// The largest index file read back, in bytes.
const MAX_FILE: u64 = 64 * 1_024 * 1_024;

/// The longest encoded exchange or name a file name may carry.
const MAX_ENCODED: usize = 200;

/// Whether `field` can be carried by a line: not empty, no separator, no
/// line break.
fn carried(field: &str) -> bool {
    !field.is_empty() && !field.contains(['|', '\n', '\r'])
}

/// `word` as a file name: ASCII letters, digits, `-` and `_` kept, every other
/// byte `%XX`. `None` for an empty word or one too long once encoded.
fn encoded(word: &str) -> Option<String> {
    if word.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(word.len());
    for byte in word.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' {
            out.push(char::from(byte));
        } else {
            let _infallible = write!(out, "%{byte:02X}");
        }
    }
    (out.len() <= MAX_ENCODED).then_some(out)
}

/// The file one underlying's lines live in, or `None` for a name no file can
/// carry.
fn path_of(dir: &Path, exchange: &str, name: &str) -> Option<PathBuf> {
    Some(
        dir.join(encoded(exchange)?)
            .join(format!("{}.idx", encoded(name)?)),
    )
}

/// The check a line ends in: the first eight bytes of its text's SHA-256.
fn check(text: &str) -> String {
    Sha256::digest(text.as_bytes()).iter().take(8).fold(
        String::with_capacity(16),
        |mut out, byte| {
            let _infallible = write!(out, "{byte:02x}");
            out
        },
    )
}

/// `text` sealed as one line.
fn sealed(text: &str) -> String {
    format!("{text}|{}\n", check(text))
}

/// The text of a complete line whose check holds, or `None`.
fn unsealed(line: &str) -> Option<&str> {
    let (text, sum) = line.strip_suffix('\n')?.rsplit_once('|')?;
    (sum == check(text)).then_some(text)
}

/// A stamp as [`base`] writes it, or `None`.
fn parse_base(text: &str) -> Option<Stamp> {
    let (date, ordinal) = match text.split_once('_') {
        None => (text, 1),
        Some((date, n)) => (date, n.parse().ok()?),
    };
    let stamp = Stamp {
        day: parse_compact(date)?,
        ordinal,
    };
    (base(stamp) == text).then_some(stamp)
}

/// A line with a separator or line break taken out, for a refusal's words.
fn flat(text: &str) -> String {
    text.replace(['|', '\n', '\r'], " ")
}

/// The lines of an index file, and how many did not check. A missing file has
/// none.
fn read_lines(path: &Path) -> Result<(Vec<String>, u64), String> {
    match fs::metadata(path) {
        Err(why) if why.kind() == ErrorKind::NotFound => return Ok((Vec::new(), 0)),
        Err(why) => return Err(format!("{} could not be inspected: {why}", path.display())),
        Ok(_) => {}
    }
    let bytes = read_limited(path, MAX_FILE)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut lines = Vec::new();
    let mut torn = 0_u64;
    for line in text.split_inclusive('\n') {
        match unsealed(line) {
            Some(text) => lines.push(text.to_owned()),
            None => torn = torn.saturating_add(1),
        }
    }
    Ok((lines, torn))
}

/// Appends `text` to `path`, starting a new line first when the file ends in
/// a cut one, and syncs it. A new file's directory entries are synced too, up
/// to `dir`.
fn append(dir: &Path, path: &Path, text: &str) -> Result<(), String> {
    let fresh = !path.exists();
    if let (true, Some(parent)) = (fresh, path.parent()) {
        fs::create_dir_all(parent)
            .map_err(|why| format!("{} could not be created: {why}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(path)
        .map_err(|why| format!("{} could not be opened to append: {why}", path.display()))?;
    let len = file
        .metadata()
        .map_err(|why| format!("{} could not be inspected: {why}", path.display()))?
        .len();
    let mut last = [b'\n'];
    if let Some(at) = len.checked_sub(1) {
        file.read_exact_at(&mut last, at)
            .map_err(|why| format!("{} could not be read: {why}", path.display()))?;
    }
    let mut bytes = Vec::with_capacity(text.len().saturating_add(1));
    if last != [b'\n'] {
        bytes.push(b'\n');
    }
    bytes.extend_from_slice(text.as_bytes());
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "{} could not be appended to; its earlier lines are untouched: {why}",
                path.display()
            )
        })?;
    if fresh {
        let mut at = path.parent();
        while let Some(parent) = at {
            sync_dir(parent)?;
            if parent == dir {
                break;
            }
            at = parent.parent();
        }
    }
    Ok(())
}

// ─────────────────────────────── the absorbed log ───────────────────────────────

/// How a snapshot was taken in.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Taken {
    /// Its futures are in the index.
    Indexed,
    /// Its checked body did not parse; why.
    Unparseable(String),
}

/// The absorbed log, for one (exchange, type).
#[derive(Debug, Default)]
struct Log {
    taken: HashMap<Stamp, Taken>,
    torn: u64,
}

/// One absorbed line.
fn absorbed_line(stamp: Stamp, route: ContinuousSpec, taken: &Taken, counts: (u64, u64)) -> String {
    let (status, detail) = match *taken {
        Taken::Indexed => ("indexed", String::new()),
        Taken::Unparseable(ref why) => ("unparseable", flat(why)),
    };
    sealed(&format!(
        "{TAG}|absorbed|{}|{}|{}|{status}|{}|{}|{detail}",
        base(stamp),
        route.exchange,
        route.future_type,
        counts.0,
        counts.1
    ))
}

/// The absorbed log's records for `route`'s exchange and type.
fn read_log(dir: &Path, route: ContinuousSpec) -> Result<Log, String> {
    let (lines, torn) = read_lines(&dir.join(ABSORBED))?;
    let mut log = Log {
        taken: HashMap::with_capacity(lines.len()),
        torn,
    };
    for line in &lines {
        let fields: Vec<&str> = line.split('|').collect();
        let [
            tag,
            "absorbed",
            stamp,
            exchange,
            kind,
            status,
            _lines,
            _skipped,
            detail,
        ] = fields.as_slice()
        else {
            log.torn = log.torn.saturating_add(1);
            continue;
        };
        let (Some(stamp), true) = (parse_base(stamp), *tag == TAG) else {
            log.torn = log.torn.saturating_add(1);
            continue;
        };
        if *exchange != route.exchange || *kind != route.future_type {
            continue;
        }
        let taken = match *status {
            "indexed" => Taken::Indexed,
            "unparseable" => Taken::Unparseable((*detail).to_owned()),
            _ => {
                log.torn = log.torn.saturating_add(1);
                continue;
            }
        };
        log.taken.insert(stamp, taken);
    }
    Ok(log)
}

// ─────────────────────────────── the underlying files ───────────────────────────────

/// `(expiry, token)` → the earliest day a snapshot listed it.
type Firsts = BTreeMap<(Day, u64), Day>;

/// One future's line.
fn future_line(
    route: ContinuousSpec,
    name: &str,
    contract: (Day, u64),
    first: Day,
    stamp: Stamp,
) -> String {
    sealed(&format!(
        "{TAG}|future|{}|{}|{name}|{}|{}|{first}|{}",
        route.exchange,
        route.future_type,
        contract.0,
        contract.1,
        base(stamp)
    ))
}

/// One underlying's futures as its file holds them, and how many lines did not
/// check.
fn read_underlying(
    path: &Path,
    route: ContinuousSpec,
    name: &str,
) -> Result<(Firsts, u64), String> {
    let (lines, mut torn) = read_lines(path)?;
    let mut firsts = Firsts::new();
    for line in &lines {
        let fields: Vec<&str> = line.split('|').collect();
        let [
            tag,
            "future",
            exchange,
            kind,
            held,
            expiry,
            token,
            first,
            stamp,
        ] = fields.as_slice()
        else {
            torn = torn.saturating_add(1);
            continue;
        };
        let (true, Some(expiry), Ok(token), Some(first), Some(_)) = (
            *tag == TAG,
            parse_dashed(expiry),
            token.parse::<u64>(),
            parse_dashed(first),
            parse_base(stamp),
        ) else {
            torn = torn.saturating_add(1);
            continue;
        };
        // A FILE MAY HOLD ANOTHER NAME: two names that differ only in case
        // share a file on a case-insensitive volume. Only exact rows count.
        if *exchange != route.exchange || *kind != route.future_type || *held != name {
            continue;
        }
        firsts
            .entry((expiry, token))
            .and_modify(|day| *day = (*day).min(first))
            .or_insert(first);
    }
    Ok((firsts, torn))
}

/// Takes one parsed snapshot's futures into the underlying files: each
/// contract no earlier snapshot listed on or before its day gets one line.
/// Returns (lines added, rows skipped).
fn take_in(
    dir: &Path,
    snapshot: &Snapshot,
    route: ContinuousSpec,
    cache: &mut HashMap<String, Firsts>,
) -> Result<(u64, u64), String> {
    let stamp = snapshot.stamp();
    let (mut lines, mut skipped) = (0_u64, 0_u64);
    let mut new: BTreeMap<String, String> = BTreeMap::new();
    let rows = snapshot
        .rows()
        .filter(|listed| listed.exchange() == route.exchange)
        .filter(|listed| listed.instrument_type() == route.future_type);
    for listed in rows {
        let name = listed.name();
        let (Some(expiry), true, Some(path)) = (
            listed.expiry(),
            carried(name),
            path_of(dir, route.exchange, name),
        ) else {
            skipped = skipped.saturating_add(1);
            continue;
        };
        let held = match cache.entry(name.to_owned()) {
            std::collections::hash_map::Entry::Occupied(slot) => slot.into_mut(),
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(read_underlying(&path, route, name)?.0)
            }
        };
        let contract = (expiry, listed.token());
        if held.get(&contract).is_some_and(|&first| first <= stamp.day) {
            continue;
        }
        held.insert(contract, stamp.day);
        new.entry(name.to_owned())
            .or_default()
            .push_str(&future_line(route, name, contract, stamp.day, stamp));
        lines = lines.saturating_add(1);
    }
    for (name, text) in &new {
        if let Some(path) = path_of(dir, route.exchange, name) {
            append(dir, &path, text)?;
        }
    }
    Ok((lines, skipped))
}

// ─────────────────────────────── taking snapshots in ───────────────────────────────

/// What one call took into the index.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Absorbed {
    /// Snapshots whose futures are now in the index.
    pub indexed: Vec<Stamp>,
    /// Snapshots whose checked body did not parse, recorded so, with why.
    pub unparseable: Vec<(Stamp, String)>,
    /// Snapshots that could not be read, with why: not recorded, and retried
    /// by the next call.
    pub unreadable: Vec<(Stamp, String)>,
    /// Lines added to the underlying files.
    pub lines: u64,
    /// Future rows a line could not carry, counted.
    pub skipped: u64,
}

/// Takes every receipted Kite snapshot the index does not yet hold for
/// `route`'s exchange and type into it, under the writers' lock.
///
/// # Errors
///
/// A listing, a lock or an index file that fails, named. A snapshot that does
/// not read is not an error: it is in [`Absorbed::unreadable`].
pub fn absorb_pending(root: &Path, route: ContinuousSpec) -> Result<Absorbed, String> {
    let held = lock(root)?;
    let absorbed = absorb_held(root, route)?;
    held.release().map_err(|why| why.to_string())?;
    Ok(absorbed)
}

/// [`absorb_pending`] for a caller that already holds the writers' lock.
pub(super) fn absorb_held(root: &Path, route: ContinuousSpec) -> Result<Absorbed, String> {
    if !carried(route.exchange) || !carried(route.future_type) {
        return Err(format!(
            "the futures index cannot carry the exchange {:?} or type {:?}",
            route.exchange, route.future_type
        ));
    }
    let catalog = Catalog::read(root)?;
    let dir = root.join(DIR);
    let log = read_log(&dir, route)?;
    let mut out = Absorbed::default();
    let mut cache = HashMap::new();
    for &stamp in catalog.receipted(Kind::Kite) {
        if log.taken.contains_key(&stamp) {
            continue;
        }
        let body = match read_body(root, Kind::Kite, stamp) {
            Ok(body) => body,
            Err(why) => {
                out.unreadable.push((stamp, why));
                continue;
            }
        };
        let (taken, counts) = match Snapshot::parse(stamp, &body) {
            Ok(snapshot) => (Taken::Indexed, take_in(&dir, &snapshot, route, &mut cache)?),
            Err(why) => (Taken::Unparseable(why), (0, 0)),
        };
        append(
            &dir,
            &dir.join(ABSORBED),
            &absorbed_line(stamp, route, &taken, counts),
        )?;
        match taken {
            Taken::Indexed => out.indexed.push(stamp),
            Taken::Unparseable(why) => out.unparseable.push((stamp, why)),
        }
        out.lines = out.lines.saturating_add(counts.0);
        out.skipped = out.skipped.saturating_add(counts.1);
    }
    Ok(out)
}

/// One event for what a snapshot write took into the index: an error when it
/// failed or left a snapshot out, nothing when there was nothing to take.
pub(super) fn note(absorbed: &Result<Absorbed, String>) {
    let (level, message, detail) = match *absorbed {
        Err(ref why) => (
            telemetry::Level::Error,
            "the futures first-listed index could not take the snapshots in; each futures plan \
             retries them and names any it cannot read",
            why.clone(),
        ),
        Ok(ref out) if !out.unreadable.is_empty() || !out.unparseable.is_empty() => (
            telemetry::Level::Error,
            "a snapshot could not be taken into the futures first-listed index; every futures \
             plan that needs it names it",
            out.unreadable
                .iter()
                .chain(&out.unparseable)
                .map(|(stamp, why)| format!("{stamp}: {why}"))
                .collect::<Vec<_>>()
                .join(" · "),
        ),
        Ok(ref out) if out.indexed.is_empty() => return,
        Ok(ref out) => (
            telemetry::Level::Info,
            "snapshots taken into the futures first-listed index",
            format!(
                "{} snapshot(s), {} line(s) added, {} future row(s) a line could not carry",
                out.indexed.len(),
                out.lines,
                out.skipped
            ),
        ),
    };
    let _ = telemetry::emit_if!(
        level,
        EVENT_TARGET,
        message,
        "index" => telemetry::Value::Str(DIR),
        "detail" => telemetry::Value::Str(&detail),
    );
}

// ─────────────────────────────────── reading ───────────────────────────────────

/// One underlying's futures, as the index holds them through a day.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Listing {
    /// `(expiry, token)` → the first snapshot day that listed it, for every
    /// one first listed on or before the day asked.
    pub first: BTreeMap<(Day, u64), Day>,
    /// Every snapshot on or before the day that is not in the index, named as
    /// a plan refuses it.
    pub unreadable: Vec<String>,
    /// Lines that did not check (cut by a kill, or garbled), skipped.
    pub torn: u64,
}

/// What the index holds for `underlying`'s futures on `route`'s exchange,
/// first listed on or before `through`, checked against `stamps` (the
/// receipted snapshots a book was opened over). Any of them on or before
/// `through` not yet taken in is taken in first, once, under the writers'
/// lock; one that cannot be is named in [`Listing::unreadable`].
///
/// # Errors
///
/// A lock, a listing or an index file that fails, named.
pub fn listing(
    root: &Path,
    stamps: &[Stamp],
    route: ContinuousSpec,
    underlying: &str,
    through: Day,
) -> Result<Listing, String> {
    let dir = root.join(DIR);
    let upto = stamps.partition_point(|stamp| stamp.day <= through);
    let needed = stamps.get(..upto).unwrap_or_default();
    let held = lock(root)?;
    let mut log = read_log(&dir, route)?;
    let mut why_not: HashMap<Stamp, String> = HashMap::new();
    if needed.iter().any(|stamp| !log.taken.contains_key(stamp)) {
        let absorbed = absorb_held(root, route)?;
        note(&Ok(absorbed.clone()));
        why_not.extend(absorbed.unreadable);
        log = read_log(&dir, route)?;
    }
    let mut out = Listing {
        torn: log.torn,
        ..Listing::default()
    };
    for stamp in needed {
        match log.taken.get(stamp) {
            Some(Taken::Indexed) => {}
            Some(Taken::Unparseable(why)) => out.unreadable.push(format!(
                "snapshot_unreadable: {stamp}: {why} (recorded when the futures index took it \
                 in); its futures were not planned"
            )),
            None => out.unreadable.push(format!(
                "snapshot_unreadable: {stamp}: {}; its futures were not planned",
                why_not
                    .get(stamp)
                    .map_or("it is not in the futures index", String::as_str)
            )),
        }
    }
    if let Some(path) = path_of(&dir, route.exchange, underlying) {
        let (firsts, torn) = read_underlying(&path, route, underlying)?;
        out.torn = out.torn.saturating_add(torn);
        out.first = firsts
            .into_iter()
            .filter(|&(_, first)| first <= through)
            .collect();
    }
    held.release().map_err(|why| why.to_string())?;
    Ok(out)
}

#[cfg(test)]
#[path = "master_snapshots_first_listed_tests.rs"]
mod tests;

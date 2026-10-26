//! The futures first-listed index: for each future a dated Kite snapshot
//! lists, the first snapshot day that listed it. R4-28 repair 1; D-7187.
//! Checked against its own commit log and rebuilt as a new generation when it
//! does not match: R4-28 repair 2.
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
//! Under the snapshots directory, in [`DIR`], one directory per generation,
//! `g1`, `g2`, ...; the highest-numbered one is the index. In it:
//!
//! - `<EXCHANGE>/<NAME>.idx`, one file per (exchange, underlying name), both
//!   percent-encoded: one line per `(type, expiry, token)` the first time a
//!   snapshot lists it, and again only when an older snapshot taken in later
//!   lists it earlier. The reader takes the earliest day per key.
//! - [`ABSORBED`]: one line per Kite snapshot taken in, for one (exchange,
//!   type), written after every line it added is synced. **It is the commit
//!   marker**, as a receipt is a snapshot's, and it records, for each name
//!   whose file grew, how many checked lines of that name the file then held.
//!
//! Beside the generations, [`GENERATIONS`] records each generation started
//! after the first, with why, before its directory is made.
//!
//! Every file is append-only: no line is ever rewritten. Every line ends in a
//! check over its own bytes.
//!
//! # A generation is trusted only while it matches its commit log
//!
//! Every read of an underlying's file, by a plan or by a snapshot write, is
//! checked against the commit log: a line anywhere in the log or in that file
//! that does not check (cut by a kill mid-append, or garbled since), or fewer
//! checked lines of the name than the log records (the file cut at a line
//! boundary, or gone), means the generation can no longer say which contracts
//! exist from which day. It is never planned from and never appended to: the
//! next generation is started and every receipted snapshot the call needs is
//! taken into it again. The damaged generation is left exactly as it was. If
//! the new generation does not match its own log either, the call refuses by
//! name (`futures_index_damaged`) rather than plan from it.
//!
//! # Crash
//!
//! A kill after a snapshot's receipt and before its [`ABSORBED`] line leaves
//! the snapshot not taken in: the next writer or plan, under the writers' lock,
//! takes it in again. Whole lines it had already added are true (the snapshot
//! is receipted and listed them), so the repeat adds no duplicate and its
//! commit line records them. A line the kill cut short does not check, so the
//! generation is rebuilt as above.
//!
//! # What is not taken in, and how it is named
//!
//! A snapshot whose receipt or body does not check is not recorded: every plan
//! that needs it names it and retries it, since a read can fail for a reason
//! that passes. A snapshot whose checked body does not parse is recorded as
//! unparseable, with why and the parse rules' version [`PARSE_RULES`]: its
//! bytes are fixed by their receipt, so the same rules could only refuse them
//! again, and a later version takes the day in again. A future row with a
//! field a line cannot carry (a separator or line break, an empty name, a name
//! too long for a file name, no expiry) is counted on its [`ABSORBED`] line and
//! in the event, never dropped silently.
//!
//! # Cost
//!
//! | operation | cost |
//! |---|---|
//! | [`listing`], every snapshot taken in | one directory listing of [`DIR`] (one entry per generation), the absorbed log (one line per snapshot, plus one `name=count` per name whose file grew) and one underlying's file (one line per contract it ever listed); no snapshot is opened |
//! | taking one snapshot in | one read and parse of that snapshot, and one read, check and append per underlying with a new contract — once per snapshot, when it is written |
//! | a generation that does not match its log | every receipted snapshot read and parsed once more, into the next generation: linear in the snapshots held, once per damage found |

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write as _};
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::{
    Book, Catalog, EVENT_TARGET, Kind, Snapshot, Stamp, base, lock, parse_compact, parse_dashed,
    read_body, read_limited, sync_dir,
};
use crate::session::Day;
use crate::vendor::ContinuousSpec;

/// The index's directory, inside the snapshots directory.
pub const DIR: &str = "futures-first-listed";

/// The commit log of a generation: one line per snapshot taken in.
pub const ABSORBED: &str = "absorbed.idx";

/// The generations' record, in [`DIR`]: one line per generation started after
/// the first, with why.
pub const GENERATIONS: &str = "generations.idx";

/// The version of the rules [`Snapshot::parse`] applies, recorded with every
/// snapshot it refused. Raise it when the parser accepts a dump it refused
/// before: every snapshot recorded unparseable under an older version is then
/// taken in again.
pub const PARSE_RULES: u32 = 1;

/// The first field of every line. A line of another format is not a record.
const TAG: &str = "fl2";

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
    let mut last = *b"\n";
    if let Some(at) = len.checked_sub(1) {
        file.read_exact_at(&mut last, at)
            .map_err(|why| format!("{} could not be read: {why}", path.display()))?;
    }
    let mut bytes = Vec::with_capacity(text.len().saturating_add(1));
    if last != *b"\n" {
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

// ─────────────────────────────── generations ───────────────────────────────

/// Why a call into one generation did not finish.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Fault {
    /// A file could not be listed, read or written: the call fails, named.
    Io(String),
    /// The generation does not match its own commit log: it is never planned
    /// from or appended to again.
    Damaged(String),
}

impl From<String> for Fault {
    fn from(why: String) -> Self {
        Self::Io(why)
    }
}

/// One generation of the index.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Generation {
    number: u32,
    dir: PathBuf,
}

/// A generation's directory name.
fn generation_name(number: u32) -> String {
    format!("g{number}")
}

/// The number a generation directory's name gives, or `None` for a name this
/// module did not make.
fn generation_number(name: &str) -> Option<u32> {
    let number: u32 = name.strip_prefix('g')?.parse().ok()?;
    (number >= 1 && generation_name(number) == name).then_some(number)
}

/// The generation every reader and writer uses: the highest-numbered
/// generation directory, or the first when none exists yet.
fn current(root: &Path) -> Result<Generation, String> {
    let top = root.join(DIR);
    let listed = |why: std::io::Error| format!("{} could not be listed: {why}", top.display());
    let entries = match fs::read_dir(&top) {
        Err(why) if why.kind() == ErrorKind::NotFound => {
            return Ok(Generation {
                number: 1,
                dir: top.join(generation_name(1)),
            });
        }
        Err(why) => return Err(listed(why)),
        Ok(entries) => entries,
    };
    let mut number = 1;
    for entry in entries {
        let entry = entry.map_err(listed)?;
        let is_dir = entry.file_type().map_err(listed)?.is_dir();
        if let (true, Some(found)) = (
            is_dir,
            entry.file_name().to_str().and_then(generation_number),
        ) {
            number = number.max(found);
        }
    }
    Ok(Generation {
        number,
        dir: top.join(generation_name(number)),
    })
}

/// The directory of the generation the index reads and writes now. Nothing is
/// created.
///
/// # Errors
///
/// The index's directory that cannot be listed, named.
pub fn generation_dir(root: &Path) -> Result<PathBuf, String> {
    current(root).map(|generation| generation.dir)
}

/// Starts the generation after `old`: why is recorded first, then its
/// directory is made. `old` is left exactly as it is.
fn start_next(
    root: &Path,
    route: ContinuousSpec,
    old: &Generation,
    why: &str,
) -> Result<Generation, String> {
    let top = root.join(DIR);
    let number = old
        .number
        .checked_add(1)
        .ok_or_else(|| "the futures index has no generation number left".to_owned())?;
    append(
        &top,
        &top.join(GENERATIONS),
        &sealed(&format!(
            "{TAG}|generation|{number}|{}|{}|{}",
            route.exchange,
            route.future_type,
            flat(why)
        )),
    )?;
    let dir = top.join(generation_name(number));
    fs::create_dir(&dir).map_err(|why| format!("{} could not be created: {why}", dir.display()))?;
    sync_dir(&top)?;
    Ok(Generation { number, dir })
}

/// What a call did across generations.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Rebuilt<T> {
    /// It finished; with why, when the generation it found did not match its
    /// log and the next one was built.
    Done(T, Option<String>),
    /// The next generation did not match its own log either: why.
    StillDamaged(String),
}

/// Runs `work` on the current generation and, when that generation does not
/// match its own log, once more on the next one. Called under the writers'
/// lock.
fn with_rebuild<T>(
    root: &Path,
    route: ContinuousSpec,
    mut work: impl FnMut(&Generation) -> Result<T, Fault>,
) -> Result<Rebuilt<T>, String> {
    let found = current(root)?;
    let why = match work(&found) {
        Ok(done) => return Ok(Rebuilt::Done(done, None)),
        Err(Fault::Io(why)) => return Err(why),
        Err(Fault::Damaged(why)) => why,
    };
    let next = start_next(root, route, &found, &why)?;
    let said = format!(
        "generation g{} of the futures index did not match its commit log ({why}), so it was \
         left as it is and generation g{} was started, and every receipted snapshot this call \
         needed was taken into it again",
        found.number, next.number
    );
    match work(&next) {
        Ok(done) => Ok(Rebuilt::Done(done, Some(said))),
        Err(Fault::Io(why)) => Err(format!("{said}; then {why}")),
        Err(Fault::Damaged(again)) => Ok(Rebuilt::StillDamaged(format!(
            "{said}; it does not match its own commit log either: {again}"
        ))),
    }
}

// ─────────────────────────────── the absorbed log ───────────────────────────────

/// How a snapshot was taken in.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Taken {
    /// Its futures are in the index.
    Indexed,
    /// Its checked body did not parse under [`PARSE_RULES`]; why.
    Unparseable(String),
}

/// The absorbed log, for one (exchange, type).
#[derive(Debug, Default)]
struct Log {
    taken: HashMap<Stamp, Taken>,
    /// Encoded name → the most checked lines of it any commit line recorded.
    committed: HashMap<String, u64>,
}

/// The `name=count` field of a commit line.
fn counts_field(grown: &BTreeMap<String, u64>) -> String {
    grown
        .iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// A commit line's `name=count` field, or `None` when it does not parse.
fn parse_counts(field: &str) -> Option<Vec<(String, u64)>> {
    if field.is_empty() {
        return Some(Vec::new());
    }
    field
        .split(',')
        .map(|one| {
            let (name, count) = one.split_once('=')?;
            if name.is_empty() {
                return None;
            }
            Some((name.to_owned(), count.parse().ok()?))
        })
        .collect()
}

/// One absorbed line.
fn absorbed_line(
    stamp: Stamp,
    route: ContinuousSpec,
    taken: &Taken,
    counts: (u64, u64),
    grown: &BTreeMap<String, u64>,
) -> String {
    let (status, detail) = match *taken {
        Taken::Indexed => ("indexed".to_owned(), String::new()),
        Taken::Unparseable(ref why) => (format!("unparseable@{PARSE_RULES}"), flat(why)),
    };
    sealed(&format!(
        "{TAG}|absorbed|{}|{}|{}|{status}|{}|{}|{}|{detail}",
        base(stamp),
        route.exchange,
        route.future_type,
        counts.0,
        counts.1,
        counts_field(grown)
    ))
}

/// The absorbed log's records for `route`'s exchange and type. A line that
/// does not check, for any route, means the generation is damaged.
fn read_log(dir: &Path, route: ContinuousSpec) -> Result<Log, Fault> {
    let path = dir.join(ABSORBED);
    let (lines, mut torn) = read_lines(&path)?;
    let mut taken_in: HashMap<Stamp, Taken> = HashMap::with_capacity(lines.len());
    // EVERY `name=count` PAIR FIRST, so the committed map is sized to them
    // (docs/07 law 2; R4-28 repair 5): it holds at most one entry a pair.
    let mut pairs: Vec<(String, u64)> = Vec::new();
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
            counts,
            detail,
        ] = fields.as_slice()
        else {
            torn = torn.saturating_add(1);
            continue;
        };
        let (Some(stamp), true, Some(counts)) =
            (parse_base(stamp), *tag == TAG, parse_counts(counts))
        else {
            torn = torn.saturating_add(1);
            continue;
        };
        if *exchange != route.exchange || *kind != route.future_type {
            continue;
        }
        let taken = match (*status, status.strip_prefix("unparseable@")) {
            ("indexed", _) => Some(Taken::Indexed),
            (_, Some(rules)) => match rules.parse::<u32>() {
                Ok(PARSE_RULES) => Some(Taken::Unparseable((*detail).to_owned())),
                // AN OLDER PARSER'S REFUSAL: the day is taken in again.
                Ok(_) => None,
                Err(_) => {
                    torn = torn.saturating_add(1);
                    continue;
                }
            },
            _ => {
                torn = torn.saturating_add(1);
                continue;
            }
        };
        pairs.extend(counts);
        if let Some(taken) = taken {
            taken_in.insert(stamp, taken);
        }
    }
    if torn > 0 {
        return Err(Fault::Damaged(format!(
            "{} has {torn} line(s) that do not check",
            path.display()
        )));
    }
    let mut committed: HashMap<String, u64> = HashMap::with_capacity(pairs.len());
    for (name, count) in pairs {
        let held = committed.entry(name).or_insert(count);
        *held = (*held).max(count);
    }
    Ok(Log {
        taken: taken_in,
        committed,
    })
}

// ─────────────────────────────── the underlying files ───────────────────────────────

/// `(expiry, token)` → the earliest day a snapshot listed it.
type Firsts = BTreeMap<(Day, u64), Day>;

/// One underlying's futures as its checked file holds them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Held {
    /// The name, encoded as its file and its commit counts carry it.
    key: String,
    firsts: Firsts,
    /// Checked lines of this exact name in the file.
    count: u64,
}

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

/// One underlying's futures as its file holds them, checked against the
/// commit log: a line in the file that does not check, or fewer checked lines
/// of `name` than the log records, means the generation is damaged.
fn read_underlying(
    path: &Path,
    route: ContinuousSpec,
    name: &str,
    log: &Log,
) -> Result<Held, Fault> {
    let key = encoded(name).ok_or_else(|| format!("no file can carry the name {name:?}"))?;
    let (lines, mut torn) = read_lines(path)?;
    let mut held = Held {
        key,
        ..Held::default()
    };
    for line in &lines {
        let fields: Vec<&str> = line.split('|').collect();
        let [
            tag,
            "future",
            exchange,
            kind,
            named,
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
        if *exchange != route.exchange || *kind != route.future_type || *named != name {
            continue;
        }
        held.count = held.count.saturating_add(1);
        held.firsts
            .entry((expiry, token))
            .and_modify(|day| *day = (*day).min(first))
            .or_insert(first);
    }
    if torn > 0 {
        return Err(Fault::Damaged(format!(
            "{} has {torn} line(s) that do not check",
            path.display()
        )));
    }
    let committed = log.committed.get(&held.key).copied().unwrap_or(0);
    if held.count < committed {
        let state = if path.exists() {
            "holds"
        } else {
            "is missing, so holds"
        };
        return Err(Fault::Damaged(format!(
            "{} {state} {} checked line(s) of {name} where the commit log records {committed}",
            path.display(),
            held.count
        )));
    }
    Ok(held)
}

/// Takes one parsed snapshot's futures into the underlying files: each
/// contract no earlier snapshot listed on or before its day gets one line.
/// Returns (lines added, rows skipped).
fn take_in(
    dir: &Path,
    snapshot: &Snapshot,
    route: ContinuousSpec,
    log: &Log,
    cache: &mut HashMap<String, Held>,
) -> Result<(u64, u64), Fault> {
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
                slot.insert(read_underlying(&path, route, name, log)?)
            }
        };
        let contract = (expiry, listed.token());
        if held
            .firsts
            .get(&contract)
            .is_some_and(|&first| first <= stamp.day)
        {
            continue;
        }
        held.firsts.insert(contract, stamp.day);
        held.count = held.count.saturating_add(1);
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
    /// Why a new generation was started, when the one found did not match its
    /// commit log.
    pub rebuilt: Option<String>,
}

/// Takes every receipted Kite snapshot the index does not yet hold for
/// `route`'s exchange and type into it, under the writers' lock.
///
/// # Errors
///
/// A listing, a lock or an index file that fails, named; and
/// `futures_index_damaged` when a newly started generation does not match its
/// own commit log either. A snapshot that does not read is not an error: it is
/// in [`Absorbed::unreadable`].
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
    match with_rebuild(root, route, |generation| absorb_in(root, generation, route))? {
        Rebuilt::Done(mut absorbed, rebuilt) => {
            absorbed.rebuilt = rebuilt;
            Ok(absorbed)
        }
        Rebuilt::StillDamaged(why) => Err(format!("futures_index_damaged: {why}")),
    }
}

/// Takes every receipted Kite snapshot `generation` does not hold into it.
fn absorb_in(
    root: &Path,
    generation: &Generation,
    route: ContinuousSpec,
) -> Result<Absorbed, Fault> {
    let catalog = Catalog::read(root)?;
    let dir = &generation.dir;
    let mut log = read_log(dir, route)?;
    let mut out = Absorbed::default();
    // ONE ENTRY PER UNDERLYING NAME, sized to the names the commit log
    // already records (docs/07 law 2; R4-28 repair 5): exact once the
    // generation holds a snapshot, so it grows only while a generation takes
    // its first snapshot in, or when a name is listed for the first time.
    let mut cache: HashMap<String, Held> = HashMap::with_capacity(log.committed.len());
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
            Ok(snapshot) => (
                Taken::Indexed,
                take_in(dir, &snapshot, route, &log, &mut cache)?,
            ),
            Err(why) => (Taken::Unparseable(why), (0, 0)),
        };
        // EVERY NAME WHOSE FILE NOW HOLDS MORE THAN THE LOG RECORDS: the lines
        // this snapshot added, and whole lines a killed earlier attempt added.
        let grown: BTreeMap<String, u64> = cache
            .values()
            .filter(|held| held.count > log.committed.get(&held.key).copied().unwrap_or(0))
            .map(|held| (held.key.clone(), held.count))
            .collect();
        append(
            dir,
            &dir.join(ABSORBED),
            &absorbed_line(stamp, route, &taken, counts, &grown),
        )?;
        log.committed.extend(grown);
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
/// failed, left a snapshot out or found the index damaged, nothing when there
/// was nothing to take.
pub(super) fn note(absorbed: &Result<Absorbed, String>) {
    let (level, message, detail) = match *absorbed {
        Err(ref why) => (
            telemetry::Level::Error,
            "the futures first-listed index could not take the snapshots in; each futures plan \
             retries them and names any it cannot read",
            why.clone(),
        ),
        Ok(ref out) if out.rebuilt.is_some() => (
            telemetry::Level::Error,
            "the futures first-listed index did not match its commit log; a new generation was \
             started and the snapshots were taken into it again",
            out.rebuilt.clone().unwrap_or_default(),
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
    /// Every snapshot on or before the day that is not in the index, and an
    /// index that could not be trusted, named as a plan refuses them.
    pub refused: Vec<String>,
    /// Why a new generation was started, when the one found did not match its
    /// commit log; the listing is read from the new one.
    pub rebuilt: Option<String>,
}

/// What the index holds for `underlying`'s futures on `route`'s exchange,
/// first listed on or before `through`, checked against the receipted
/// snapshots `book` was opened over that are on or before `through`, which
/// the book's day table finds with one index (R4-28 repair 5: they were found
/// by a search of its stamps). Any of them not yet taken in is taken in
/// first, once, under the writers' lock; one that cannot be is named in
/// [`Listing::refused`]. An index that does not match its commit log is
/// rebuilt as a new generation first ([`Listing::rebuilt`]); one that does
/// not match after that is refused by name (`futures_index_damaged`) with
/// nothing listed.
///
/// # Errors
///
/// A lock, a listing or an index file that fails, named.
pub fn listing(
    root: &Path,
    book: &Book,
    route: ContinuousSpec,
    underlying: &str,
    through: Day,
) -> Result<Listing, String> {
    let needed = book.through(through);
    let held = lock(root)?;
    let outcome = with_rebuild(root, route, |generation| {
        list_in(root, generation, needed, route, underlying, through)
    })?;
    held.release().map_err(|why| why.to_string())?;
    Ok(match outcome {
        Rebuilt::Done(mut listing, rebuilt) => {
            listing.rebuilt = rebuilt;
            listing
        }
        Rebuilt::StillDamaged(why) => Listing {
            refused: vec![format!(
                "futures_index_damaged: {why}; no contract of {underlying} was planned from it"
            )],
            ..Listing::default()
        },
    })
}

/// [`listing`] in one generation, over `needed`: the receipted snapshots on
/// or before `through`, oldest first.
fn list_in(
    root: &Path,
    generation: &Generation,
    needed: &[Stamp],
    route: ContinuousSpec,
    underlying: &str,
    through: Day,
) -> Result<Listing, Fault> {
    let dir = &generation.dir;
    let mut log = read_log(dir, route)?;
    // WHY A SNAPSHOT IS NOT IN THE INDEX, known only when this call took
    // snapshots in, and so made only then (R4-28 repair 5).
    let mut why_not: Option<HashMap<Stamp, String>> = None;
    if needed.iter().any(|stamp| !log.taken.contains_key(stamp)) {
        let absorbed = absorb_in(root, generation, route)?;
        note(&Ok(absorbed.clone()));
        why_not = Some(absorbed.unreadable.into_iter().collect());
        log = read_log(dir, route)?;
    }
    let mut out = Listing::default();
    for stamp in needed {
        match log.taken.get(stamp) {
            Some(Taken::Indexed) => {}
            Some(Taken::Unparseable(why)) => out.refused.push(format!(
                "snapshot_unreadable: {stamp}: {why} (recorded when the futures index took it \
                 in); its futures were not planned"
            )),
            None => out.refused.push(format!(
                "snapshot_unreadable: {stamp}: {}; its futures were not planned",
                why_not
                    .as_ref()
                    .and_then(|why_not| why_not.get(stamp))
                    .map_or("it is not in the futures index", String::as_str)
            )),
        }
    }
    if let Some(path) = path_of(dir, route.exchange, underlying) {
        out.first = read_underlying(&path, route, underlying, &log)?
            .firsts
            .into_iter()
            .filter(|&(_, first)| first <= through)
            .collect();
    }
    Ok(out)
}

#[cfg(test)]
#[path = "master_snapshots_first_listed_tests.rs"]
mod tests;

//! `/scrub.json` — what the full-block scrub found in the series a page shows,
//! and where each vendor's pass stands (DB-B74, DB-B65; WP-81).
//!
//! # The failure this surfaces
//!
//! A block that rotted in the middle of a month was refused only to whoever
//! read that exact block; the census still counted its bars, coverage was
//! unchanged, and no page knew. `cli scrub-blocks` now reads every block and
//! keeps what would not verify in `<store>/audit/scrub-blocks.<vendor>.log`.
//! This is how a page learns of it: `/db` asks for the series it is showing
//! and prints what the last pass found there; `/ingest` shows each vendor's
//! totals through `/disk.json`, which uses [`standing`] too.
//!
//! # `GET /scrub.json?feed=<vendor>&timeframe=<rung>&key=<series>[&key=…]`
//!
//! ```text
//! {"feed":"gdfl","recorded":true,"pass":2,"state":"complete",
//!  "totals":{"months":…,"blocks":…,"bad":…,"unverifiable":…,"misfiled":…,
//!            "unbound":…,"busy":…,"missing":…},
//!  "standing_from_pass":2,"indexed_bytes":…,"truncated":false,
//!  "series":[{"key":"NSE-INDEX-NIFTY","timeframe":"1s","more":0,
//!    "findings":[{"kind":"bad","month":"2020-07","block":21,"first":1533,
//!                 "last":1605,"pass":2,"run":1791547200,"why":"…"}]}]}
//! ```
//!
//! With no `key=` it answers the vendor's state alone. `recorded` is `false`,
//! with `why`, when no scrub of the vendor ever ran: "nothing found" and
//! "never looked" are different answers and are never merged.
//!
//! # Which findings stand
//!
//! A finding stands while no COMPLETE pass has read its month again without
//! finding it. So the findings served are those of the last complete pass and
//! of any pass after it (`standing_from_pass`): once a later pass completes,
//! a block that was repaired — its month re-imported as a new generation —
//! drops out, and one still damaged is found again and stays. A month a pass
//! met `busy` was not re-read by it; that limit is `cli scrub-blocks`'s and is
//! in its header.
//!
//! # What is read, and what is never read
//!
//! The cursor's newest whole line, through the scrub's own strict reader
//! (`cli::scrub_cursor_line`): exactly the words the scrub writes, in its
//! order. A tail no newline ends — a run killed, or out of disk, mid-append —
//! is not a line, and neither is the void line the next run ends it as. A
//! reader that took the first `months=` word of a line served a killed run's
//! fragment as the pass's totals once the next line was joined onto it.
//!
//! Both files are opened without following a final symlink and without
//! waiting on a FIFO, and must be regular files ([`open_plain`]): a FIFO at
//! either name held a store-read slot per request until the bar routes
//! answered 429. Anything else at the name is `recorded:false` with the reason.
//!
//! # Cost
//!
//! Per request: one open and `fstat` of each of the two files, one bounded
//! tail read of the cursor ([`CURSOR_TAIL_BYTES`]), then one hash probe per
//! `key=` (at most [`MAX_KEYS`]) and at most [`MAX_FINDINGS`] findings rendered
//! per key; `standing_findings` is a sum over the passes that still stand. The
//! ledger is indexed INCREMENTALLY: only the bytes appended since the last
//! request are read (from the kept byte offset, whole lines only, one hash
//! probe per line), so every finding of the whole ledger is indexed, none
//! drops out behind newer ones, and an unchanged ledger costs nothing. A
//! request reads at most [`INDEX_STEP_BYTES`] of it — the first request after
//! a start over a longer ledger answers `truncated: true` and the next ones
//! continue — and a ledger that is another file than the one indexed (moved
//! aside, shortened, rewritten in place) is indexed again from its start.
//! Nothing here opens a bar file.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use brutex_core::vendor::Vendor;

use super::{Loaded, param, params, timeframe_param};
use crate::{bars, render};

/// The most `key=` one request may name.
pub(crate) const MAX_KEYS: usize = 64;

/// The most findings named per series; the rest are counted in `more`.
pub(crate) const MAX_FINDINGS: usize = 32;

/// The most cursor bytes read: its last line carries the pass's totals.
pub(crate) const CURSOR_TAIL_BYTES: u64 = 8 * 1024;

/// The most ledger bytes one request reads to bring the index up to the
/// ledger's end. About 60,000 lines; a longer backlog (the first request
/// after a start, over a long ledger) is caught up over the next requests,
/// each answering `truncated: true` until it is.
pub(crate) const INDEX_STEP_BYTES: u64 = 16 * 1024 * 1024;

/// Bytes kept from before the indexed offset and compared before more is
/// read: a ledger rewritten in place is not the file the index was built from.
const SEAM_BYTES: usize = 64;

/// The longest reason served with one finding, in bytes.
const MAX_WHY: usize = 320;

/// Where one vendor's pass stands, from its cursor file's newest whole line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Standing {
    /// The pass the line belongs to.
    pub(crate) pass: u64,
    /// Whether that pass is finished.
    pub(crate) complete: bool,
    /// The pass's totals, in `cli::SCRUB_TOTAL_NAMES`' order.
    pub(crate) totals: [u64; 8],
}

impl Standing {
    /// The first pass whose findings still stand: the last complete one.
    pub(crate) const fn standing_from(&self) -> u64 {
        if self.complete {
            self.pass
        } else {
            self.pass.saturating_sub(1)
        }
    }

    /// One total by its name, or `None` for a name the line does not carry.
    #[cfg(test)]
    pub(crate) fn total(&self, key: &str) -> Option<u64> {
        cli::SCRUB_TOTAL_NAMES
            .iter()
            .zip(self.totals)
            .find_map(|(name, value)| (*name == key).then_some(value))
    }

    /// The line's totals as a JSON object.
    pub(crate) fn totals_json(&self) -> String {
        let mut out = String::from("{");
        for (n, (key, value)) in cli::SCRUB_TOTAL_NAMES.iter().zip(self.totals).enumerate() {
            if n > 0 {
                out.push(',');
            }
            let _ = write!(out, r#""{key}":{value}"#);
        }
        out.push('}');
        out
    }
}

/// The value of `key=` among one line's words.
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split(' ')
        .find_map(|word| word.strip_prefix(key)?.strip_prefix('='))
}

/// `path` opened for a bounded read, and its `fstat`: never through a final
/// symlink, never waiting on a FIFO, and only a regular file.
///
/// Every file this module and `/disk.json` read by name on a request path
/// goes through here. A plain `File::open` of a FIFO waits for a writer for
/// ever: three requests to `/disk.json` with a FIFO at `ops/backups.log`
/// never answered, each kept a store-read slot, and after thirteen every bar
/// seek answered 429.
///
/// # Errors
///
/// The host's error, with a symlink and a non-regular file named as what
/// they are.
pub(crate) fn open_plain(path: &Path) -> std::io::Result<(std::fs::File, std::fs::Metadata)> {
    let refused = |what: &str| std::io::Error::new(std::io::ErrorKind::InvalidInput, what);
    let file = store::open_flags::open_read_no_follow(path).map_err(|why| {
        if why.raw_os_error() == Some(store::open_flags::ELOOP) {
            refused("it is a symbolic link, and this file is never read through one")
        } else {
            why
        }
    })?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(refused(
            "it is not a regular file (a directory, a FIFO or a device)",
        ));
    }
    Ok((file, meta))
}

/// The last `bytes` of the regular file at `path`, and whether that cut off
/// an earlier part of it.
///
/// # Errors
///
/// [`open_plain`]'s, and the read's.
pub(crate) fn tail_of(path: &Path, bytes: u64) -> std::io::Result<(String, bool)> {
    use std::io::{Read as _, Seek as _, SeekFrom};
    let (mut file, meta) = open_plain(path)?;
    let from = meta.len().saturating_sub(bytes);
    file.seek(SeekFrom::Start(from))?;
    let mut tail = Vec::new();
    file.take(bytes).read_to_end(&mut tail)?;
    Ok((String::from_utf8_lossy(&tail).into_owned(), from > 0))
}

/// Where `vendor`'s pass stands.
///
/// # Errors
///
/// A sentence, never a guess: no cursor file (no scrub ever ran), one that
/// cannot be read or is not a regular file, or one whose newest whole line is
/// not a cursor line the scrub writes.
pub(crate) fn standing(root: &Path, vendor: Vendor) -> Result<Standing, String> {
    let path = root.join(cli::scrub_cursor_name(vendor));
    let (tail, cut) = tail_of(&path, CURSOR_TAIL_BYTES).map_err(|why| {
        if why.kind() == std::io::ErrorKind::NotFound {
            format!(
                "{} cannot be read ({why}): no full-block scrub of {} has run, so a block \
                 damaged in the middle of a month is found only by a read of that block; \
                 `cli scrub-blocks {}` runs one",
                path.display(),
                vendor.as_str(),
                vendor.as_str()
            )
        } else {
            format!(
                "{} cannot be read ({why}): where the full-block scrub of {} stands cannot \
                 be told",
                path.display(),
                vendor.as_str()
            )
        }
    })?;
    let line = cli::scrub_whole_lines(&tail, cut)
        .next_back()
        .ok_or_else(|| {
            format!(
                "{} holds no whole line: a scrub of {} started and recorded no progress",
                path.display(),
                vendor.as_str()
            )
        })?;
    let read = cli::scrub_cursor_line(line).ok_or_else(|| {
        format!(
            "{}'s newest line is no progress or done line the scrub writes: {:?}",
            path.display(),
            crate::fault_log::capped(line.to_owned())
        )
    })?;
    Ok(Standing {
        pass: read.pass,
        complete: read.done,
        totals: read.totals,
    })
}

/// One thing a pass found in one month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    /// `bad`, `unverifiable`, `misfiled` or `missing`.
    pub(crate) kind: &'static str,
    /// The month's file stem, `YYYY-MM`.
    pub(crate) month: String,
    /// The damaged block and its first and last record, for a `bad` line.
    pub(crate) block: Option<(u64, u64, u64)>,
    /// The pass of the run that found it; 0 when its `run` line was not
    /// among the indexed bytes.
    pub(crate) pass: u64,
    /// The run that found it.
    pub(crate) run: u64,
    /// The store's own reason, capped.
    pub(crate) why: String,
}

/// What makes two findings one: the kind, the month and the block.
type Same = (&'static str, String, Option<u64>);

/// One series' findings, and where each is in the list.
#[derive(Debug, Clone, Default)]
struct Held {
    list: Vec<Finding>,
    at: HashMap<Same, usize>,
}

/// The findings of one ledger, by series directory (the month file's parent,
/// relative to the store root).
#[derive(Debug, Clone, Default)]
pub(crate) struct Index {
    by_series: HashMap<String, Held>,
    /// The pass of each run whose `run` line was read.
    passes: HashMap<u64, u64>,
    /// How many findings each pass holds now; pass 0 is "not told".
    by_pass: BTreeMap<u64, usize>,
    /// Bytes of the ledger indexed: the offset after the last whole line read.
    pub(crate) indexed_bytes: u64,
    /// Whether ledger bytes past `indexed_bytes` were left for the next
    /// request by [`INDEX_STEP_BYTES`].
    pub(crate) truncated: bool,
}

impl Index {
    /// Indexes more whole lines of the ledger, oldest first.
    ///
    /// One hash probe per line. A finding of the same kind for the same month
    /// and block replaces the older one, so a month re-read after a killed
    /// run, or found again by a later pass, is one finding carrying its
    /// newest pass. A void line — the ended tail of a killed run — and a line
    /// of no kind this build reads are skipped.
    fn extend(&mut self, text: &str) {
        for line in cli::scrub_whole_lines(text, false) {
            let mut words = line.splitn(3, ' ');
            let (Some(word), Some(second)) = (words.next(), words.next()) else {
                continue;
            };
            if word == "run" {
                if let (Ok(run), Some(pass)) = (
                    second.parse::<u64>(),
                    field(line, "pass").and_then(|value| value.parse::<u64>().ok()),
                ) {
                    self.passes.insert(run, pass);
                }
                continue;
            }
            let kind = match word {
                "bad" => "bad",
                "unverifiable" => "unverifiable",
                "misfiled" => "misfiled",
                "missing" => "missing",
                _ => continue,
            };
            let Some((series, file)) = second.rsplit_once('/') else {
                continue;
            };
            let Some(month) = file.strip_suffix(".bin") else {
                continue;
            };
            // THE REASON IS EVERYTHING AFTER ` why=`: it holds spaces, so it
            // is cut from the line, never split into words.
            let (head, why) = line.split_once(" why=").unwrap_or((line, ""));
            let number = |key: &str| field(head, key).and_then(|value| value.parse::<u64>().ok());
            let run = number("run").unwrap_or(0);
            let block = number("block").and_then(|block| {
                let (first, last) = field(head, "records")?.split_once('-')?;
                Some((block, first.parse().ok()?, last.parse().ok()?))
            });
            let finding = Finding {
                kind,
                month: month.to_owned(),
                block,
                pass: self.passes.get(&run).copied().unwrap_or(0),
                run,
                why: capped_at(why, MAX_WHY),
            };
            *self.by_pass.entry(finding.pass).or_default() += 1;
            let of_series = self.by_series.entry(series.to_owned()).or_default();
            let same = (
                kind,
                finding.month.clone(),
                block.map(|(block, _, _)| block),
            );
            let older = of_series
                .at
                .get(&same)
                .and_then(|at| of_series.list.get_mut(*at));
            if let Some(older) = older {
                if let Some(count) = self.by_pass.get_mut(&older.pass) {
                    *count = count.saturating_sub(1);
                }
                *older = finding;
            } else {
                of_series.at.insert(same, of_series.list.len());
                of_series.list.push(finding);
            }
        }
    }

    /// The index of `text`'s whole lines.
    #[cfg(test)]
    pub(crate) fn of(text: &str) -> Self {
        let mut index = Self::default();
        index.extend(text);
        index
    }

    /// The findings under `series` that still stand at `from_pass`, oldest
    /// month first. A finding whose pass could not be told (no `run` line of
    /// its run is in the ledger) is kept: unknown is not "repaired".
    pub(crate) fn standing(&self, series: &str, from_pass: u64) -> Vec<&Finding> {
        let mut out: Vec<&Finding> = self
            .by_series
            .get(series)
            .map(|held| {
                held.list
                    .iter()
                    .filter(|finding| finding.pass == 0 || finding.pass >= from_pass)
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by(|a, b| (&a.month, a.block).cmp(&(&b.month, b.block)));
        out
    }

    /// At most `most` standing findings of the vendor with the series each is
    /// in, in series and month order: what a summary names.
    pub(crate) fn named(&self, from_pass: u64, most: usize) -> Vec<(&str, &Finding)> {
        let mut out: Vec<(&str, &Finding)> = self
            .by_series
            .iter()
            .flat_map(|(series, held)| {
                held.list
                    .iter()
                    .map(move |finding| (series.as_str(), finding))
            })
            .filter(|(_, finding)| finding.pass == 0 || finding.pass >= from_pass)
            .collect();
        out.sort_by(|a, b| (a.0, &a.1.month, a.1.block).cmp(&(b.0, &b.1.month, b.1.block)));
        out.truncate(most);
        out
    }

    /// Every standing finding of the vendor, counted: those of no told pass
    /// and those of each pass from `from_pass` on. A sum over those passes,
    /// never a walk of the findings.
    pub(crate) fn standing_count(&self, from_pass: u64) -> usize {
        let untold = if from_pass > 0 {
            self.by_pass.get(&0).copied().unwrap_or(0)
        } else {
            0
        };
        self.by_pass
            .range(from_pass..)
            .map(|(_, count)| *count)
            .sum::<usize>()
            .saturating_add(untold)
    }
}

/// `text` cut at `max` bytes on a character boundary, marked when cut.
fn capped_at(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_owned();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", text.get(..end).unwrap_or(""))
}

/// One vendor's kept index, and where in its ledger it stands.
#[derive(Debug)]
struct Kept {
    path: PathBuf,
    /// The ledger's device and inode: another file at the name is another
    /// ledger, indexed from its start.
    file: (u64, u64),
    /// The offset after the last whole line indexed.
    consumed: u64,
    /// The last bytes before `consumed`.
    seam: Vec<u8>,
    /// Inside a line longer than one step: its rest is dropped through its
    /// newline. No line this build writes is that long.
    in_long_line: bool,
    /// The length and modified time at which the ledger was last indexed to
    /// its last whole line: while they stand, nothing is read.
    settled: Option<(u64, Option<SystemTime>)>,
    index: Arc<Index>,
}

impl Kept {
    fn new(path: PathBuf, file: (u64, u64)) -> Self {
        Self {
            path,
            file,
            consumed: 0,
            seam: Vec::new(),
            in_long_line: false,
            settled: None,
            index: Arc::new(Index::default()),
        }
    }

    /// Indexes the whole lines appended since the last call, at most `step`
    /// bytes of them.
    fn advance(
        &mut self,
        file: &mut std::fs::File,
        (len, modified): (u64, Option<SystemTime>),
        step: u64,
    ) -> std::io::Result<()> {
        use std::io::{Read as _, Seek as _, SeekFrom};
        let mut read = |from: u64, most: u64| -> std::io::Result<Vec<u8>> {
            file.seek(SeekFrom::Start(from))?;
            let mut bytes = Vec::new();
            file.take(most).read_to_end(&mut bytes)?;
            Ok(bytes)
        };
        let seam = u64::try_from(self.seam.len()).unwrap_or(0);
        let mut bytes = read(
            self.consumed.saturating_sub(seam),
            step.saturating_add(seam),
        )?;
        if !bytes.starts_with(&self.seam) {
            // NOT THE BYTES THIS INDEX WAS BUILT FROM: the ledger was
            // rewritten in place. Everything held is of another file.
            *self = Self::new(std::mem::take(&mut self.path), self.file);
            bytes = read(0, step)?;
        }
        let held = self.seam.len();
        let fresh = bytes.get(held..).unwrap_or_default();
        let mut from = 0;
        if self.in_long_line {
            match fresh.iter().position(|byte| *byte == b'\n') {
                Some(newline) => {
                    from = newline + 1;
                    self.in_long_line = false;
                }
                None => from = fresh.len(),
            }
        }
        let full = u64::try_from(fresh.len()).unwrap_or(u64::MAX) >= step;
        let whole = fresh
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map(|newline| newline + 1)
            .filter(|end| *end >= from);
        let taken = match whole {
            Some(end) => {
                let text = String::from_utf8_lossy(fresh.get(from..end).unwrap_or_default());
                Arc::make_mut(&mut self.index).extend(&text);
                end
            }
            // A WHOLE STEP WITH NO NEWLINE is no line this build writes; it
            // is passed over, never held against every later request.
            None if full && !self.in_long_line && from < fresh.len() => {
                self.in_long_line = true;
                fresh.len()
            }
            None => from,
        };
        self.consumed = self
            .consumed
            .saturating_add(u64::try_from(taken).unwrap_or(0));
        let upto = held.saturating_add(taken);
        self.seam = bytes
            .get(upto.saturating_sub(SEAM_BYTES)..upto)
            .unwrap_or_default()
            .to_vec();
        let index = Arc::make_mut(&mut self.index);
        index.indexed_bytes = self.consumed;
        // BYTES THE STEP DID NOT REACH are the next request's; a torn tail
        // after the last whole line is not "truncated", it is not a line.
        index.truncated = full && self.consumed < len;
        self.settled = (!index.truncated).then_some((len, modified));
        Ok(())
    }
}

/// The kept index of each vendor.
static KEPT: [Mutex<Option<Kept>>; Vendor::ALL.len()] =
    [const { Mutex::new(None) }; Vendor::ALL.len()];

/// `vendor`'s findings index, brought up to its ledger's end.
///
/// The kept one while the ledger has not moved; extended by the lines
/// appended since when it has ([`Kept::advance`]). A ledger that does not
/// exist is an empty index — one that never ran is [`standing`]'s refusal,
/// not this function's. One request at a time extends a vendor's index; the
/// others wait for it and share the result.
///
/// # Errors
///
/// A ledger that is not a regular file or cannot be read: what the scrub
/// found cannot be told, and an empty index would say "nothing".
pub(crate) fn index(root: &Path, vendor: Vendor) -> Result<Arc<Index>, String> {
    index_in_steps(root, vendor, INDEX_STEP_BYTES)
}

/// [`index`], reading at most `step` ledger bytes.
fn index_in_steps(root: &Path, vendor: Vendor, step: u64) -> Result<Arc<Index>, String> {
    use std::os::unix::fs::MetadataExt as _;
    let path = root.join(cli::scrub_ledger_name(vendor));
    let unreadable = |why: std::io::Error| {
        format!(
            "{} cannot be read ({why}): what the full-block scrub of {} found cannot be told",
            path.display(),
            vendor.as_str()
        )
    };
    let (mut file, meta) = match open_plain(&path) {
        Ok(opened) => opened,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Arc::new(Index::default()));
        }
        Err(why) => return Err(unreadable(why)),
    };
    let stamp = (meta.len(), meta.modified().ok());
    let identity = (meta.dev(), meta.ino());
    let Some(cell) = KEPT.get(vendor as usize) else {
        return Ok(Arc::new(Index::default()));
    };
    let mut held = cell.lock().unwrap_or_else(PoisonError::into_inner);
    // THE SAME FILE, NO SHORTER, is the ledger this index was built from;
    // anything else at the name is indexed from its start.
    let same = held
        .as_ref()
        .is_some_and(|kept| kept.path == path && kept.file == identity && kept.consumed <= stamp.0);
    if !same {
        *held = Some(Kept::new(path.clone(), identity));
    }
    let Some(kept) = held.as_mut() else {
        return Ok(Arc::new(Index::default()));
    };
    if kept.settled != Some(stamp) {
        kept.advance(&mut file, stamp, step).map_err(unreadable)?;
    }
    Ok(Arc::clone(&kept.index))
}

/// The series directory of `key` at `timeframe`, relative to the store root:
/// where its month files are, as the ledger names them.
fn series_dir(
    vendor: Vendor,
    key: &bars::SeriesKey,
    timeframe: store::path::Timeframe,
) -> Option<String> {
    // ANY MONTH: the directory does not depend on it.
    let month = store::path::YearMonth::new(2000, 1).ok()?;
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor,
        exchange: &key.exchange,
        segment: &key.segment,
        symbol: &key.symbol,
        contract: key.contract,
        timeframe,
        month,
        file: store::path::FileKind::Bars,
    })
    .ok()?;
    let relative = path.to_path_buf(Path::new(""));
    Some(relative.parent()?.display().to_string())
}

/// The answer, or the refusal of a query that names no feed, an unknown
/// rung, a series that is none, or too many of them.
pub(crate) fn answer(root: &Path, query: &str) -> Result<String, String> {
    let word = param(query, "feed");
    let vendor = Vendor::ALL
        .into_iter()
        .find(|vendor| word == vendor.as_str())
        .ok_or_else(|| {
            format!(
                "/scrub.json needs feed=<{}> and never guesses one; {word:?} is none",
                Vendor::ALL.map(Vendor::as_str).join("|")
            )
        })?;
    let keys = params(query, "key");
    if keys.len() > MAX_KEYS {
        return Err(format!(
            "too_many_series: {} key= were sent and at most {MAX_KEYS} are read per request",
            keys.len()
        ));
    }
    let timeframe = timeframe_param(query)?;
    let mut series = Vec::with_capacity(keys.len());
    for text in &keys {
        let key = bars::SeriesKey::parse(text)
            .ok_or_else(|| format!("{text:?} is not a series this store can name"))?;
        let dir = series_dir(vendor, &key, timeframe)
            .ok_or_else(|| format!("{text:?} names no store path at {}", timeframe.as_str()))?;
        series.push((text.as_str(), dir));
    }
    let mut out = format!(r#"{{"feed":{}"#, render::json_string(vendor.as_str()));
    let state = match standing(root, vendor) {
        Ok(state) => state,
        Err(why) => {
            let _ = write!(
                out,
                r#","recorded":false,"why":{},"series":[]}}"#,
                render::json_string(&why)
            );
            return Ok(out);
        }
    };
    let found = match index(root, vendor) {
        Ok(found) => found,
        Err(why) => {
            let _ = write!(
                out,
                r#","recorded":false,"why":{},"series":[]}}"#,
                render::json_string(&why)
            );
            return Ok(out);
        }
    };
    let from_pass = state.standing_from();
    let _ = write!(
        out,
        r#","recorded":true,"pass":{},"state":{},"totals":{},"standing_from_pass":{from_pass},"standing_findings":{},"indexed_bytes":{},"truncated":{},"series":["#,
        state.pass,
        render::json_string(if state.complete {
            "complete"
        } else {
            "in progress"
        }),
        state.totals_json(),
        found.standing_count(from_pass),
        found.indexed_bytes,
        found.truncated
    );
    for (n, (text, dir)) in series.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let standing = found.standing(dir, from_pass);
        let _ = write!(
            out,
            r#"{{"key":{},"timeframe":{},"more":{},"findings":["#,
            render::json_string(text),
            render::json_string(timeframe.as_str()),
            standing.len().saturating_sub(MAX_FINDINGS)
        );
        for (m, finding) in standing.iter().take(MAX_FINDINGS).enumerate() {
            if m > 0 {
                out.push(',');
            }
            finding_json(&mut out, finding);
        }
        out.push_str("]}");
    }
    out.push_str("]}");
    Ok(out)
}

/// One finding as JSON.
pub(crate) fn finding_json(out: &mut String, finding: &Finding) {
    let number = |value: Option<u64>| value.map_or_else(|| "null".to_owned(), |n| n.to_string());
    let _ = write!(
        out,
        r#"{{"kind":{},"month":{},"block":{},"first":{},"last":{},"pass":{},"run":{},"why":{}}}"#,
        render::json_string(finding.kind),
        render::json_string(&finding.month),
        number(finding.block.map(|(block, _, _)| block)),
        number(finding.block.map(|(_, first, _)| first)),
        number(finding.block.map(|(_, _, last)| last)),
        finding.pass,
        finding.run,
        render::json_string(&finding.why)
    );
}

/// `GET /scrub.json`. See the module header.
pub(super) async fn scrub_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 2],
    String,
) {
    let headers = [
        (
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        ),
        (axum::http::header::CACHE_CONTROL, "no-store"),
    ];
    let query = uri.query().unwrap_or_default().to_owned();
    let ran = crate::detail::run_store_read(move || answer(&site.store_root, &query)).await;
    match ran {
        Ok(Ok(body)) => (axum::http::StatusCode::OK, headers, body),
        Ok(Err(why)) => (
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            serde_json::json!({ "error": why }).to_string(),
        ),
        Err(why) => {
            let (status, body) = crate::detail::admission_refused(
                "a scrub lookup",
                crate::detail::MAX_STORE_READ_CONCURRENT,
                &why,
            );
            (status, headers, body)
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;

    const T1: &str = "months=3 blocks=9 bad=2 unverifiable=1 misfiled=1 unbound=3 busy=0 missing=1";

    /// A store root with `audit/` and nothing else.
    fn root_of(name: &str) -> PathBuf {
        let root = crate::scratch::path(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("audit")).unwrap();
        root
    }

    fn write(root: &Path, vendor: Vendor, cursor: &str, ledger: &str) {
        std::fs::write(root.join(cli::scrub_cursor_name(vendor)), cursor).unwrap();
        std::fs::write(root.join(cli::scrub_ledger_name(vendor)), ledger).unwrap();
    }

    /// The ledger of two passes over the NIFTY index second months and one
    /// option contract: pass 1 found two bad blocks and a lost sidecar; pass
    /// 2 found one of the bad blocks again, a misfiled contract month and a
    /// month the disk lost.
    fn two_passes() -> String {
        let nifty = "bars/gdfl/NSE/INDEX/NIFTY/1s";
        let call = "bars/gdfl/NSE/FNO/NIFTY/2019-11-28-1200000-CE/1s";
        format!(
            "run 100 vendor=gdfl commit=unstamped pass=1 resume=-\n\
             bad {nifty}/2020-07.bin block=21 records=1533-1605 run=100 why=block 21 sealed 1 computed 2\n\
             bad {nifty}/2020-08.bin block=4 records=292-364 run=100 why=block 4 does not match\n\
             unverifiable {nifty}/2020-09.bin run=100 why=declares block checksums and has no sidecar\n\
             busy {nifty}/2020-10.bin run=100\n\
             run 200 vendor=gdfl commit=unstamped pass=2 resume=-\n\
             bad {nifty}/2020-07.bin block=21 records=1533-1605 run=200 why=block 21 sealed 1 computed 2\n\
             misfiled {call}/2019-11.bin run=200 why=its store path is not this month's\n\
             missing {nifty}/2020-11.bin run=200\n"
        )
    }

    /// **WHAT A PASS FOUND IN THE SERIES A PAGE SHOWS, AND ONLY WHAT STILL
    /// STANDS (DB-B74).** With pass 2 complete, the block pass 2 found again
    /// stands, with pass 2's run; the block and the lost sidecar only pass 1
    /// found do not — a complete pass read those months and found nothing;
    /// the misfiled contract month is its own series' finding (DB-B65).
    #[test]
    fn a_series_is_answered_with_the_findings_that_still_stand() {
        let root = root_of("scrub-route-standing");
        write(
            &root,
            Vendor::Gdfl,
            &format!(
                "progress run=100 pass=1 {T1} cursor=x\ndone run=100 pass=1 {T1}\ndone run=200 pass=2 {T1}\n"
            ),
            &two_passes(),
        );
        let body = answer(
            &root,
            "feed=gdfl&timeframe=1s&key=NSE-INDEX-NIFTY&key=NSE-FNO-NIFTY-2019-11-28-1200000-CE&key=NSE-INDEX-BANKNIFTY",
        )
        .expect("answered");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        assert_eq!(parsed["recorded"], true);
        assert_eq!(
            (&parsed["pass"], &parsed["state"]),
            (&serde_json::json!(2), &serde_json::json!("complete"))
        );
        assert_eq!(parsed["standing_from_pass"], 2);
        assert_eq!(parsed["totals"]["bad"], 2);
        assert_eq!(parsed["totals"]["unbound"], 3);
        assert_eq!(parsed["standing_findings"], 3);
        assert_eq!(parsed["truncated"], false);
        let nifty = &parsed["series"][0];
        assert_eq!(nifty["key"], "NSE-INDEX-NIFTY");
        let findings = nifty["findings"].as_array().expect("a list");
        assert_eq!(findings.len(), 2, "{body}");
        assert_eq!(
            findings[0],
            serde_json::json!({"kind":"bad","month":"2020-07","block":21,"first":1533,"last":1605,
                "pass":2,"run":200,"why":"block 21 sealed 1 computed 2"})
        );
        assert_eq!(findings[1]["kind"], "missing");
        assert_eq!(findings[1]["month"], "2020-11");
        assert!(findings[1]["block"].is_null());
        let call = &parsed["series"][1]["findings"];
        assert_eq!(call[0]["kind"], "misfiled");
        assert_eq!(call[0]["why"], "its store path is not this month's");
        assert_eq!(parsed["series"][2]["findings"], serde_json::json!([]));

        // PASS 2 STILL RUNNING: pass 1 is the last complete pass, so what it
        // found stands beside what pass 2 has found so far.
        write(
            &root,
            Vendor::Gdfl,
            &format!("done run=100 pass=1 {T1}\nprogress run=200 pass=2 {T1} cursor=y\n"),
            &format!("{} ", two_passes()),
        );
        let body = answer(&root, "feed=gdfl&timeframe=1s&key=NSE-INDEX-NIFTY").expect("answered");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        assert_eq!(parsed["state"], "in progress");
        assert_eq!(parsed["standing_from_pass"], 1);
        let kinds: Vec<(&str, &str)> = parsed["series"][0]["findings"]
            .as_array()
            .expect("a list")
            .iter()
            .map(|f| (f["month"].as_str().unwrap(), f["kind"].as_str().unwrap()))
            .collect();
        assert_eq!(
            kinds,
            [
                ("2020-07", "bad"),
                ("2020-08", "bad"),
                ("2020-09", "unverifiable"),
                ("2020-11", "missing")
            ],
            "{body}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// "Never looked" is not "nothing found"; a vendor's state alone needs no
    /// key; and a query that cannot be read is refused by name.
    #[test]
    fn a_vendor_never_scrubbed_says_so_and_a_bad_query_is_refused() {
        let root = root_of("scrub-route-never");
        let body = answer(&root, "feed=zerodha&key=NSE-INDEX-NIFTY").expect("answered");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        assert_eq!(parsed["recorded"], false);
        assert!(
            parsed["why"]
                .as_str()
                .unwrap()
                .contains("no full-block scrub of zerodha has run"),
            "{body}"
        );
        // A clean, complete pass: recorded, and nothing found.
        write(
            &root,
            Vendor::Zerodha,
            "done run=7 pass=1 months=2 blocks=6 bad=0 unverifiable=0 misfiled=0 unbound=2 busy=0 missing=0\n",
            "run 7 vendor=zerodha commit=unstamped pass=1 resume=-\n",
        );
        let body = answer(&root, "feed=zerodha").expect("the state alone");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        assert_eq!(parsed["recorded"], true);
        assert_eq!(parsed["standing_findings"], 0);
        assert_eq!(parsed["series"], serde_json::json!([]));

        for (query, said) in [
            ("", "needs feed="),
            ("feed=nobody", "needs feed="),
            ("feed=zerodha&timeframe=7min&key=NSE-INDEX-NIFTY", "7min"),
            ("feed=zerodha&key=NIFTY", "is not a series"),
        ] {
            let why = answer(&root, query).expect_err("refused");
            assert!(why.contains(said), "{query:?}: {why}");
        }
        let mut many = String::new();
        for n in 0..=MAX_KEYS {
            let _ = write!(many, "&key=NSE-CASH-S{n}");
        }
        let why = answer(&root, &format!("feed=zerodha{many}")).expect_err("too many");
        assert!(why.starts_with("too_many_series"), "{why}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A cursor whose newest whole line is not a pass line, or holds only a
    /// torn one, is a named refusal; a torn tail after a whole line is not
    /// read; the findings past the cap are counted.
    #[test]
    fn torn_and_foreign_cursor_lines_and_the_findings_cap() {
        let root = root_of("scrub-route-torn");
        write(&root, Vendor::Gdfl, "progress run=1 pass=1 months=", "");
        let why = standing(&root, Vendor::Gdfl).expect_err("no whole line");
        assert!(why.contains("holds no whole line"), "{why}");
        write(&root, Vendor::Gdfl, "nonsense\n", "");
        assert!(standing(&root, Vendor::Gdfl).is_err());
        write(&root, Vendor::Gdfl, "progress run=1 pass=0 cursor=a\n", "");
        assert!(standing(&root, Vendor::Gdfl).is_err());
        write(
            &root,
            Vendor::Gdfl,
            &format!("done run=1 pass=3 {T1}\nprogress run=2 pass=4 months=99"),
            "",
        );
        let state = standing(&root, Vendor::Gdfl).expect("the whole line");
        assert_eq!(
            (state.pass, state.complete, state.standing_from()),
            (3, true, 3)
        );
        assert_eq!(state.total("missing"), Some(1));
        assert_eq!(state.total("nothing"), None);

        let series = "bars/gdfl/NSE/INDEX/NIFTY/1s";
        let mut ledger = String::from("run 9 vendor=gdfl commit=unstamped pass=3 resume=-\n");
        for block in 0..(MAX_FINDINGS + 5) {
            let _ = writeln!(
                ledger,
                "bad {series}/2020-07.bin block={block} records={}-{} run=9 why={}",
                block * 73,
                block * 73 + 72,
                "₹".repeat(400)
            );
        }
        // A torn last line is not a finding.
        ledger.push_str("bad bars/gdfl/NSE/INDEX/NIFTY/1s/2020-12.bin block=1 records=73-1");
        write(
            &root,
            Vendor::Gdfl,
            &format!("done run=9 pass=3 {T1}\n"),
            &ledger,
        );
        let body = answer(&root, "feed=gdfl&timeframe=1s&key=NSE-INDEX-NIFTY").expect("answered");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        let nifty = &parsed["series"][0];
        assert_eq!(
            nifty["findings"].as_array().expect("a list").len(),
            MAX_FINDINGS
        );
        assert_eq!(nifty["more"], 5);
        assert!(nifty["findings"][0]["why"].as_str().unwrap().len() <= MAX_WHY + '…'.len_utf8());
        assert_eq!(parsed["standing_findings"], MAX_FINDINGS + 5);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The index is rebuilt when the ledger moves and reused when it has not;
    /// a finding whose run line is not among the indexed bytes is kept.
    #[test]
    fn the_index_follows_the_ledger_and_keeps_what_it_cannot_date() {
        let root = root_of("scrub-route-index");
        let series = "bars/dhan/NSE/INDEX/NIFTY/1min";
        write(
            &root,
            Vendor::Dhan,
            &format!("done run=5 pass=2 {T1}\n"),
            &format!("bad {series}/2025-05.bin block=4 records=292-364 run=1 why=old\n"),
        );
        let first = index(&root, Vendor::Dhan).expect("indexed");
        assert_eq!(first.standing(series, 2).len(), 1, "undated, so kept");
        assert_eq!(first.standing(series, 2)[0].pass, 0);
        assert!(
            Arc::ptr_eq(&first, &index(&root, Vendor::Dhan).expect("kept")),
            "reused"
        );
        std::fs::write(
            root.join(cli::scrub_ledger_name(Vendor::Dhan)),
            format!(
                "run 1 vendor=dhan commit=unstamped pass=1 resume=-\n\
                 bad {series}/2025-05.bin block=4 records=292-364 run=1 why=old\n"
            ),
        )
        .unwrap();
        // REWRITTEN IN PLACE (same inode, longer): not the bytes the index
        // was built from, so it is indexed again from its start.
        let second = index(&root, Vendor::Dhan).expect("indexed again");
        assert!(!Arc::ptr_eq(&first, &second), "the ledger moved");
        assert!(
            second.standing(series, 2).is_empty(),
            "pass 1, and pass 2 is complete"
        );
        assert_eq!(second.standing(series, 1).len(), 1);
        let undated = Index::of("garbage fragment\nmissing a/b/2020-01.bin run=3\n");
        assert_eq!(undated.standing("a/b", 9).len(), 1);
        assert_eq!(undated.standing_count(9), 1);
        assert_eq!(capped_at("abc", 2), "ab…");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A KILLED RUN'S FRAGMENT IS NEVER SERVED AS THE PASS'S TOTALS.** The
    /// reader took the first `months=`/`bad=` word of the newest line; once
    /// the next run's line was joined onto a fragment, `/scrub.json`,
    /// `/disk.json` and the `/ingest` tile said months 999, bad 5 for a pass
    /// whose true totals were months 3, bad 0.
    #[test]
    fn a_torn_tail_is_never_served_as_totals_and_a_void_line_is_no_finding() {
        const CLAIM: &str = "progress run=5 pass=1 months=999 blocks=77 bad=5 unverifiable=0 \
                             misfiled=0 unbound=0 busy=0 missing=0 cursor=bars/gdfl/x-11";
        const TRUE: &str =
            "months=3 blocks=9 bad=0 unverifiable=0 misfiled=0 unbound=3 busy=0 missing=0";
        let root = root_of("scrub-route-void");
        let nifty = "bars/gdfl/NSE/INDEX/NIFTY/1s";
        // AS THE FIXED SCRUB LEAVES IT: the fragment ended as a void line,
        // in the cursor and in the ledger.
        write(
            &root,
            Vendor::Gdfl,
            &format!(
                "progress run=4 pass=1 {} cursor=-\n{CLAIM} #void\ndone run=6 pass=1 {TRUE}\n",
                TRUE.replace("months=3", "months=0")
            ),
            &format!(
                "run 4 vendor=gdfl commit=unstamped pass=1 resume=-\n\
                 bad {nifty}/2020-12.bin block=1 records=73-1 #void\n\
                 torn audit/scrub-blocks.gdfl.log bytes=58 run=6\n\
                 run 6 vendor=gdfl commit=unstamped pass=1 resume=-\n\
                 missing {nifty}/2020-11.bin run=6\n"
            ),
        );
        let state = standing(&root, Vendor::Gdfl).expect("the whole line");
        assert_eq!((state.pass, state.complete), (1, true));
        assert_eq!(state.totals, [3, 9, 0, 0, 0, 3, 0, 0]);
        let body = answer(&root, "feed=gdfl&timeframe=1s&key=NSE-INDEX-NIFTY").expect("answered");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        assert_eq!(parsed["state"], "complete");
        assert_eq!(parsed["totals"]["months"], 3);
        assert_eq!(parsed["totals"]["bad"], 0);
        assert_eq!(parsed["standing_findings"], 1);
        let findings = parsed["series"][0]["findings"].as_array().expect("a list");
        assert_eq!(findings.len(), 1, "the void line is no finding: {body}");
        assert_eq!(
            (
                &findings[0]["kind"],
                &findings[0]["month"],
                &findings[0]["pass"]
            ),
            (
                &serde_json::json!("missing"),
                &serde_json::json!("2020-11"),
                &serde_json::json!(1)
            )
        );

        // AS A BUILD BEFORE THE FIX LEFT IT: the next line joined onto the
        // fragment. That is two lines' words and no line: refused by name,
        // never read for its first `months=`.
        write(
            &root,
            Vendor::Gdfl,
            &format!("{CLAIM}progress run=6 pass=1 {TRUE} cursor=bars/gdfl/y.bin\n"),
            "",
        );
        let why = standing(&root, Vendor::Gdfl).expect_err("no line the scrub writes");
        assert!(why.contains("no progress or done line"), "{why}");
        let body = answer(&root, "feed=gdfl").expect("answered");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        assert_eq!(parsed["recorded"], false);
        assert!(
            parsed.get("totals").is_none(),
            "no totals are guessed: {body}"
        );
        // The same for a `done` joined onto a fragment, and for a line with
        // a total twice.
        for joined in [
            format!(
                "progress run=9 pass=1 months=3 blocks=9 bad=0 unverifiable=0 misfidone run=9 pass=1 {TRUE}\n"
            ),
            format!("done run=9 pass=1 months=7 {TRUE}\n"),
        ] {
            write(&root, Vendor::Gdfl, &joined, "");
            assert!(standing(&root, Vendor::Gdfl).is_err(), "{joined:?}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **EVERY FINDING OF THE LEDGER IS INDEXED, AND ONLY WHAT WAS APPENDED
    /// IS READ.** The index was the ledger's last 4 MiB, rebuilt whole on
    /// every change: a finding older than that dropped out of the page and
    /// of `standing_findings` while it still stood.
    #[test]
    fn the_index_holds_the_whole_ledger_and_reads_only_what_was_appended() {
        use std::io::Write as _;
        let root = root_of("scrub-route-incremental");
        let series = |n: usize| format!("bars/truedata/NSE/CASH/S{n}/1min");
        let line = |n: usize| {
            format!(
                "bad {}/2025-05.bin block={n} records=0-72 run=1 why={}\n",
                series(n),
                "x".repeat(260)
            )
        };
        // 20,000 findings of about 330 bytes: 6.6 MiB, past the old 4 MiB.
        let mut ledger = String::from("run 1 vendor=truedata commit=unstamped pass=1 resume=-\n");
        for n in 0..20_000 {
            ledger.push_str(&line(n));
        }
        assert!(ledger.len() > 6 * 1024 * 1024);
        write(
            &root,
            Vendor::TrueData,
            &format!("done run=1 pass=1 {T1}\n"),
            &ledger,
        );
        let whole = index(&root, Vendor::TrueData).expect("indexed");
        assert!(!whole.truncated);
        assert_eq!(whole.indexed_bytes, ledger.len() as u64);
        assert_eq!(whole.standing_count(1), 20_000);
        assert_eq!(
            whole.standing(&series(0), 1).len(),
            1,
            "the oldest finding is still there"
        );
        let body = answer(&root, "feed=truedata&timeframe=1min&key=NSE-CASH-S0").expect("answered");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        assert_eq!(parsed["standing_findings"], 20_000);
        assert_eq!(parsed["series"][0]["findings"][0]["block"], 0);

        // APPENDED: a torn tail first (not read, not "truncated"), then its
        // void end, a pass-2 run and two findings, one of them found again.
        let path = root.join(cli::scrub_ledger_name(Vendor::TrueData));
        let append = |text: &str| {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            file.write_all(text.as_bytes()).unwrap();
        };
        append("bad bars/truedata/NSE/CASH/S7/1min/2025-06.bin block=1 rec");
        let torn = index(&root, Vendor::TrueData).expect("indexed");
        assert_eq!(
            torn.indexed_bytes,
            ledger.len() as u64,
            "the fragment is not consumed"
        );
        assert!(!torn.truncated);
        assert_eq!(torn.standing_count(1), 20_000);
        append(&format!(
            " #void\nrun 2 vendor=truedata commit=unstamped pass=2 resume=-\n{}{}",
            line(5).replace("run=1", "run=2"),
            line(20_000).replace("run=1", "run=2")
        ));
        let grown = index(&root, Vendor::TrueData).expect("indexed");
        assert_eq!(grown.standing_count(1), 20_001, "one new, one found again");
        assert_eq!(grown.standing_count(2), 2, "what pass 2 found");
        assert_eq!(grown.standing(&series(5), 2).len(), 1);
        assert_eq!(grown.standing(&series(5), 2)[0].run, 2);
        let seventh = grown.standing(&series(7), 1);
        assert_eq!(
            seventh.iter().map(|f| f.month.as_str()).collect::<Vec<_>>(),
            ["2025-05"],
            "the void line's 2025-06 is no finding"
        );
        assert_eq!(
            grown.indexed_bytes,
            std::fs::metadata(&path).unwrap().len(),
            "to the ledger's end"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A BACKLOG LONGER THAN ONE STEP IS CAUGHT UP, NEVER DROPPED.** A
    /// request reads a bounded part of the ledger; the answer says
    /// `truncated` until the next requests have read the rest.
    #[test]
    fn a_backlog_is_indexed_in_steps_and_a_line_longer_than_a_step_is_passed_over() {
        let line = |n: usize| {
            format!(
                "bad bars/groww/NSE/CASH/S{n}/1min/2025-05.bin block={n} records=0-72 run=1 why={}\n",
                "x".repeat(260)
            )
        };
        // IN STEPS: a backlog longer than one step answers `truncated` and
        // is caught up by the next requests, never dropped.
        let stepped = root_of("scrub-route-steps");
        let mut short = String::from("run 1 vendor=groww commit=unstamped pass=1 resume=-\n");
        for n in 0..40 {
            short.push_str(&line(n));
        }
        write(
            &stepped,
            Vendor::Groww,
            &format!("done run=1 pass=1 {T1}\n"),
            &short,
        );
        let mut reads = 0;
        let caught_up = loop {
            let step = index_in_steps(&stepped, Vendor::Groww, 1_000).expect("indexed");
            reads += 1;
            assert!(reads < 100, "it advances every request");
            if !step.truncated {
                break step;
            }
            assert!(step.indexed_bytes < short.len() as u64);
        };
        assert!(reads > 5, "more than one step was needed: {reads}");
        assert_eq!(caught_up.standing_count(1), 40);
        assert_eq!(caught_up.indexed_bytes, short.len() as u64);
        // A line longer than a step is passed over, and the lines after it
        // are still read.
        let long = root_of("scrub-route-long-line");
        write(
            &long,
            Vendor::Groww,
            &format!("done run=1 pass=1 {T1}\n"),
            &format!(
                "run 1 vendor=groww commit=unstamped pass=1 resume=-\nbad {}\n{}",
                "z".repeat(5_000),
                line(3)
            ),
        );
        let mut reads = 0;
        let after = loop {
            let step = index_in_steps(&long, Vendor::Groww, 1_000).expect("indexed");
            reads += 1;
            assert!(reads < 100, "it advances every request");
            if !step.truncated {
                break step;
            }
        };
        assert_eq!(
            after.standing_count(1),
            1,
            "only the line after the long one"
        );
        for root in [stepped, long] {
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    /// **A SCRUB FILE THAT IS NO REGULAR FILE IS REFUSED AT ONCE, BY NAME.**
    /// A FIFO, a directory or a symlink at the cursor or the ledger: the
    /// answer is `recorded:false` with the reason, within a bound, and never
    /// "nothing found".
    #[test]
    fn a_fifo_a_directory_or_a_symlink_at_a_scrub_file_is_refused_at_once() {
        let root = root_of("scrub-route-odd");
        let elsewhere = root.join("elsewhere");
        std::fs::write(&elsewhere, format!("done run=1 pass=1 {T1}\n")).unwrap();
        for name in [
            cli::scrub_cursor_name(Vendor::Zerodha),
            cli::scrub_ledger_name(Vendor::Zerodha),
        ] {
            for kind in ["fifo", "symlink", "directory"] {
                write(
                    &root,
                    Vendor::Zerodha,
                    &format!("done run=1 pass=1 {T1}\n"),
                    "run 1 vendor=zerodha commit=unstamped pass=1 resume=-\n",
                );
                let at = root.join(&name);
                std::fs::remove_file(&at).unwrap();
                match kind {
                    "fifo" => {
                        // `mkfifo` is the external program, run from a test only.
                        let made = std::process::Command::new("mkfifo")
                            .arg(&at)
                            .status()
                            .expect("mkfifo ran");
                        assert!(made.success());
                    }
                    "symlink" => std::os::unix::fs::symlink(&elsewhere, &at).unwrap(),
                    _ => std::fs::create_dir(&at).unwrap(),
                }
                let asked = std::time::Instant::now();
                let body = answer(&root, "feed=zerodha&timeframe=1min&key=NSE-INDEX-NIFTY")
                    .expect("answered");
                assert!(
                    asked.elapsed() < std::time::Duration::from_secs(5),
                    "{kind} at {name}: answered without waiting"
                );
                let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
                assert_eq!(parsed["recorded"], false, "{kind} at {name}: {body}");
                let why = parsed["why"].as_str().expect("a reason");
                assert!(why.contains(&name), "{kind}: it names the file: {why}");
                assert!(why.contains("cannot be told"), "{kind} at {name}: {why}");
                assert!(
                    !why.contains("no full-block scrub of zerodha has run"),
                    "{kind}: a file that cannot be read is not a scrub that never ran: {why}"
                );
                if kind == "directory" {
                    std::fs::remove_dir(&at).unwrap();
                } else {
                    std::fs::remove_file(&at).unwrap();
                }
            }
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}

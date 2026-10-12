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
//!  "standing_from_pass":2,"standing_findings":…,"standing_busy":…,
//!  "indexed_bytes":…,"truncated":false,
//!  "series":[{"key":"NSE-INDEX-NIFTY","timeframe":"1s","more":0,
//!    "findings":[{"kind":"bad","month":"2020-07","block":21,"first":1533,
//!                 "last":1605,"pass":2,"run":1791547200,"kept_pass":null,
//!                 "kept_run":null,"why":"…"}],
//!    "busy":[{"month":"2020-09","pass":2,"run":1791547200}],"busy_more":0}]}
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
//! drops out, and one still damaged is found again and stays.
//!
//! # A month a pass did not read keeps what was found in it
//!
//! A month a writer held when the pass reached it is a `busy <month>
//! run=<id>` ledger line: the pass did NOT read it. So that line is not
//! silence about the month, and the index does two things with it
//! ([`Index::keep`]):
//!
//! * every finding that stood in that month when the pass began (those the
//!   pass before it holds) is filed under the busy pass, UNCHANGED but for
//!   `kept_pass` and `kept_run` — the pass and run that met the month busy.
//!   Its `pass` and `run` stay those of the run that last READ the block, so
//!   an answer says both "found by pass 6" and "not read again by pass 7";
//! * the month is named in its series' `busy` list (`busy_more` counts the
//!   ones past [`MAX_FINDINGS`]) and counted in `standing_busy`, for as long
//!   as that pass's findings stand. A complete pass with a busy month is never
//!   answered as a pass that vouches for it.
//!
//! Before this a `busy` line was skipped: a month an importer, a repair or an
//! index build held for the moment the pass reached it LOST every standing
//! finding when that pass completed — the page said "names no damaged block"
//! and the tile "0 bad blocks" while the block was still refused on read and
//! no pass had read the month again (second agent, 12 Oct 2026).
//!
//! A `busy` line of a run whose `run` line was not indexed cannot be dated:
//! what stood in its month is kept under pass 0, "not told", which stands
//! until a dated pass finds it again — unknown is not "repaired".
//!
//! A finding line of the SAME dated pass for that month, after its `busy`
//! line, says the month was read after all — a run met it busy and was
//! killed, and the run that resumed the pass read it: the busy mark is taken
//! away and what it kept is released ([`Index::release`]), so the month's
//! findings are what that run finds. The other order — found by one run,
//! then met busy by the run that resumed the pass and re-read its months —
//! keeps: the killed run may not have read every block.
//!
//! One thing stays conservative, by name: a pass names only what it FOUND, so
//! a month met busy by a run that was then killed, and read CLEAN by the run
//! that resumed the same pass, still shows as busy and keeps its findings
//! until the NEXT pass completes (`docs/06-limits.md` L-WP81-9).
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
//! tail read of the cursor ([`CURSOR_TAIL_BYTES`]), then for each `key=` (at
//! most [`MAX_KEYS`]) one hash probe and one ordered seek per standing pass,
//! and at most [`MAX_FINDINGS`] findings and as many busy months rendered;
//! `standing_findings`, `standing_busy` and each series' `more` and
//! `busy_more` are sums over the passes that still stand. None of it grows
//! with the ledger.
//!
//! The ledger is indexed INCREMENTALLY and IN PLACE: only the bytes appended
//! since the last request are read (from the kept byte offset, whole lines
//! only, a constant number of hash and tree operations per line) and they are
//! added to the one index the vendor keeps — nothing already indexed is
//! copied, walked or sorted again. An unchanged ledger costs nothing. A
//! request reads at most [`INDEX_STEP_BYTES`] of it — the first request after
//! a start over a longer ledger answers `truncated: true` and the next ones
//! continue — and a ledger that is another file than the one indexed (moved
//! aside, shortened, rewritten in place) is indexed again from its start.
//! Nothing here opens a bar file.
//!
//! ## What this replaced, measured (WP-81 repair 2)
//!
//! The index was an `Arc` handed to each request and extended through
//! `Arc::make_mut`: whenever another request still held it — always, under
//! concurrency — the WHOLE index was deep-copied to add a few lines, and each
//! answer collected and sorted the series' findings (the `/disk.json` summary:
//! every finding of the vendor). On a ledger of 385,419 standing findings
//! being appended by a running scrub, 20 clients got 205 of 800 answers
//! refused 429, and — the route sharing the store-read pool with bar seeks —
//! 283 of 960 concurrent `/bars/window.json` seeks were refused with it. Now
//! the index is extended and read under its vendor's lock, findings are kept
//! in their answer's order as they are indexed, and the route is admitted in
//! a pool of its own (`crate::detail::run_ops_read`).
//!
//! And a third cause, found by measuring that repair: each request measured
//! the ledger BEFORE it took the vendor's lock, so behind a request that had
//! indexed further the ledger looked shorter than its own index — "another
//! file at the name" — and the whole index was thrown away and read again
//! from byte 0 (still 1,055 of 80,000 answers refused 429, waits of up to
//! 869 ms, on a 128 MB ledger under a running scrub). The ledger is now
//! measured under the lock (`with_index_in_steps`).
//!
//! ## What stays O(ledger), by name (`docs/06-limits.md`)
//!
//! The first index after a start reads the whole ledger once, at most
//! [`INDEX_STEP_BYTES`] a request. The index holds one entry per distinct
//! finding, and one per month, pass and series met busy, that the ledger ever
//! named, for the life of the process.

use std::collections::btree_map;
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
    /// The newest pass, and its run, that met this finding's month `busy` and
    /// so kept the finding without reading the month again; `None` while the
    /// pass that found it is the newest to have reached its month. Pass 0 is
    /// a run that cannot be dated.
    pub(crate) kept: Option<(u64, u64)>,
    /// The store's own reason, capped.
    pub(crate) why: String,
}

/// What makes two findings one, and what orders them in an answer: the series
/// directory, the month, the block and the kind. A summary names findings in
/// series, month and block order, and a series' own are that order's slice.
type Key = (Arc<str>, String, Option<u64>, &'static str);

/// The findings of one ledger, kept in the order they are answered in.
///
/// # Shape
///
/// A finding lives in the map of the pass that last found it — or that last
/// met its month busy and kept it ([`Index::keep`]) — (`by_pass`), and each
/// map is ordered by [`Key`]. The findings that STAND at a pass are those
/// of pass 0 ("not told") and of every pass from it on — a handful of maps —
/// so the first few of a series, or of the whole vendor, are the heads of a
/// merge of those maps: a seek and a constant number of steps, whatever the
/// ledger holds. A finding found again by a later pass moves to that pass's
/// map (`at` says where it was), so it is one finding carrying its newest
/// pass and a stale one is never walked past to reach a standing one.
#[derive(Debug, Default)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct Index {
    /// The findings each pass holds now; pass 0 is "not told".
    by_pass: BTreeMap<u64, BTreeMap<Key, Finding>>,
    /// The pass whose map holds each finding.
    at: HashMap<Key, u64>,
    /// Every series a finding names — one allocation of the name, shared by
    /// its keys — with how many of its findings each pass holds.
    series: HashMap<Arc<str>, BTreeMap<u64, usize>>,
    /// The pass of each run whose `run` line was read.
    passes: HashMap<u64, u64>,
    /// The newest pass any indexed `run` line told.
    newest: u64,
    /// The months a pass met `busy` and did not read, with the run that met
    /// each: series, then pass (0 is "not told"), then month.
    busy: HashMap<Arc<str>, BTreeMap<u64, BTreeMap<String, u64>>>,
    /// How many months each pass met busy, over every series.
    busy_in: BTreeMap<u64, usize>,
    /// Bytes of the ledger indexed: the offset after the last whole line read.
    pub(crate) indexed_bytes: u64,
    /// Whether ledger bytes past `indexed_bytes` were left for the next
    /// request by [`INDEX_STEP_BYTES`].
    pub(crate) truncated: bool,
}

impl Index {
    /// Indexes more whole lines of the ledger, oldest first, IN PLACE.
    ///
    /// A constant number of hash and tree operations per line. A finding of
    /// the same kind for the same month and block replaces the older one, so
    /// a month re-read after a killed run, or found again by a later pass, is
    /// one finding carrying its newest pass. A void line — the ended tail of
    /// a killed run — and a line of no kind this build reads are skipped.
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
                    self.newest = self.newest.max(pass);
                }
                continue;
            }
            let kind = match word {
                "bad" => "bad",
                "unverifiable" => "unverifiable",
                "misfiled" => "misfiled",
                "missing" => "missing",
                // NOT READ, which is not "read and found clean": see `keep`.
                "busy" => "busy",
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
            let pass = self.passes.get(&run).copied().unwrap_or(0);
            if kind == "busy" {
                self.keep(series, month, pass, run);
                continue;
            }
            self.put(
                series,
                Finding {
                    kind,
                    month: month.to_owned(),
                    block,
                    pass,
                    run,
                    kept: None,
                    why: capped_at(why, MAX_WHY),
                },
            );
        }
    }

    /// The one allocation of `series`' name, shared by everything that names
    /// the series.
    fn name_of(&mut self, series: &str) -> Arc<str> {
        if let Some((name, _)) = self.series.get_key_value(series) {
            return Arc::clone(name);
        }
        let name: Arc<str> = Arc::from(series);
        self.series.insert(Arc::clone(&name), BTreeMap::new());
        name
    }

    /// Files one finding of `series` under the pass that found it, taking it
    /// out of the pass that held it before.
    ///
    /// A finding line of a dated pass says the pass READ the month. So when
    /// an earlier run of the same pass met that month busy — it was killed,
    /// and the run that resumed the pass read the month — the busy mark is
    /// taken away and what that mark kept is released to the pass before
    /// ([`Index::release`]): the month's findings are then what this run
    /// finds, line by line, and what it does not find lapses with the pass.
    /// A run that cannot be dated releases nothing: which pass read the
    /// month cannot be told.
    fn put(&mut self, series: &str, finding: Finding) {
        let name = self.name_of(series);
        let pass = finding.pass;
        if pass > 0 && self.unmark(&name, pass, &finding.month) {
            self.release(&name, &finding.month, pass);
        }
        let key: Key = (
            name,
            finding.month.clone(),
            finding.block.map(|(block, _, _)| block),
            finding.kind,
        );
        self.file(key, finding, pass);
    }

    /// Takes away the busy mark of `month` for `pass`; whether there was one.
    fn unmark(&mut self, name: &Arc<str>, pass: u64, month: &str) -> bool {
        let Some(marks) = self.busy.get_mut(name) else {
            return false;
        };
        let Some(months) = marks.get_mut(&pass) else {
            return false;
        };
        if months.remove(month).is_none() {
            return false;
        }
        if months.is_empty() {
            marks.remove(&pass);
        }
        if let Some(count) = self.busy_in.get_mut(&pass) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.busy_in.remove(&pass);
            }
        }
        true
    }

    /// Undoes what `pass`'s busy mark on `month` kept: each finding of the
    /// month that stands under `pass` only because the pass met it busy goes
    /// back under the pass before — where it stood — kept by THAT pass's busy
    /// mark when it was not that pass's own finding. O(log findings +
    /// findings of the month).
    fn release(&mut self, name: &Arc<str>, month: &str, pass: u64) {
        let Some(before) = pass.checked_sub(1).filter(|before| *before > 0) else {
            return;
        };
        let from: Key = (Arc::clone(name), month.to_owned(), None, "");
        let keys: Vec<Key> = self.by_pass.get(&pass).map_or_else(Vec::new, |held| {
            held.range(&from..)
                .take_while(|(key, _)| key.0 == *name && key.1 == month)
                .filter(|(_, finding)| finding.kept.is_some_and(|(by, _)| by == pass))
                .map(|(key, _)| key.clone())
                .collect()
        });
        let earlier = self
            .busy
            .get(name)
            .and_then(|marks| marks.get(&before))
            .and_then(|months| months.get(month))
            .copied();
        for key in keys {
            let Some(mut finding) = self
                .by_pass
                .get_mut(&pass)
                .and_then(|held| held.remove(&key))
            else {
                continue;
            };
            finding.kept = (finding.pass != before).then(|| (before, earlier.unwrap_or_default()));
            self.file(key, finding, before);
        }
    }

    /// Files `finding` under the pass `under`, taking the finding of the same
    /// key out of the pass that held it before.
    fn file(&mut self, key: Key, finding: Finding, under: u64) {
        let older = self.at.insert(key.clone(), under);
        if let Some(older) = older.filter(|older| *older != under) {
            if let Some(held) = self.by_pass.get_mut(&older) {
                held.remove(&key);
                if held.is_empty() {
                    self.by_pass.remove(&older);
                }
            }
            if let Some(counts) = self.series.get_mut(&*key.0)
                && let Some(count) = counts.get_mut(&older)
            {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    counts.remove(&older);
                }
            }
        }
        if older != Some(under)
            && let Some(counts) = self.series.get_mut(&*key.0)
        {
            *counts.entry(under).or_default() += 1;
        }
        self.by_pass.entry(under).or_default().insert(key, finding);
    }

    /// A `busy` line: the run `run` of `pass` met `month` of `series` held by
    /// a writer and DID NOT READ IT.
    ///
    /// The month is marked busy for that pass, and every finding that stood
    /// in it when the pass began is filed under the pass with `kept` set, so
    /// it stands for as long as that pass's own findings do: a pass that did
    /// not read a month says nothing about its blocks, and skipping the line
    /// read that silence as "read again, and clean" (see the module header).
    ///
    /// What stood when pass `pass` began is what the pass before it holds:
    /// passes run one after another, and each `busy` carries a month's
    /// findings forward by one, so one ordered seek in ONE pass's map finds
    /// them — O(log findings + findings of that month), never a walk. A run
    /// that cannot be dated (pass 0) keeps what the newest told pass and the
    /// one before it hold, under pass 0: it stands until found again.
    fn keep(&mut self, series: &str, month: &str, pass: u64, run: u64) {
        let name = self.name_of(series);
        if self
            .busy
            .entry(Arc::clone(&name))
            .or_default()
            .entry(pass)
            .or_default()
            .insert(month.to_owned(), run)
            .is_none()
        {
            *self.busy_in.entry(pass).or_default() += 1;
        }
        let sources: Vec<u64> = if pass == 0 {
            self.by_pass
                .range(self.newest.saturating_sub(1).max(1)..)
                .map(|(of, _)| *of)
                .collect()
        } else {
            // PASS 0 IS NEVER A SOURCE: what cannot be dated already stands.
            pass.checked_sub(1)
                .filter(|before| *before > 0)
                .into_iter()
                .collect()
        };
        let from: Key = (Arc::clone(&name), month.to_owned(), None, "");
        // WHAT AN EARLIER RUN OF THIS PASS ALREADY KEPT is kept by this run
        // now: the finding and the month's busy mark name one run.
        if let Some(held) = self.by_pass.get_mut(&pass) {
            for (_, finding) in held
                .range_mut(&from..)
                .take_while(|(key, _)| *key.0 == *series && key.1 == month)
            {
                if finding.kept.is_some() {
                    finding.kept = Some((pass, run));
                }
            }
        }
        for source in sources {
            let keys: Vec<Key> = self.by_pass.get(&source).map_or_else(Vec::new, |held| {
                held.range(&from..)
                    .take_while(|(key, _)| *key.0 == *series && key.1 == month)
                    .map(|(key, _)| key.clone())
                    .collect()
            });
            for key in keys {
                let Some(mut finding) = self
                    .by_pass
                    .get_mut(&source)
                    .and_then(|held| held.remove(&key))
                else {
                    continue;
                };
                finding.kept = Some((pass, run));
                self.file(key, finding, pass);
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

    /// The maps whose findings still stand at `from_pass`: the one of no
    /// told pass (unknown is not "repaired") and every pass from it on.
    fn standing_maps(&self, from_pass: u64) -> impl Iterator<Item = &BTreeMap<Key, Finding>> {
        let untold = if from_pass > 0 {
            self.by_pass.get(&0)
        } else {
            None
        };
        untold
            .into_iter()
            .chain(self.by_pass.range(from_pass..).map(|(_, held)| held))
    }

    /// The first `most` findings under `series` that still stand at
    /// `from_pass`, oldest month first: a seek in each standing pass's map
    /// and a merge of their heads. Never a walk or a sort of the series.
    pub(crate) fn first_standing(
        &self,
        series: &str,
        from_pass: u64,
        most: usize,
    ) -> Vec<&Finding> {
        let Some((name, _)) = self.series.get_key_value(series) else {
            return Vec::new();
        };
        let from: Key = (Arc::clone(name), String::new(), None, "");
        let ranges = self
            .standing_maps(from_pass)
            .map(|held| held.range(&from..))
            .collect();
        heads(ranges, most, |key| *key.0 == *series)
            .into_iter()
            .map(|(_, finding)| finding)
            .collect()
    }

    /// How many findings under `series` still stand at `from_pass`: a sum
    /// over that series' standing passes.
    pub(crate) fn standing_in(&self, series: &str, from_pass: u64) -> usize {
        let Some(counts) = self.series.get(series) else {
            return 0;
        };
        let untold = if from_pass > 0 {
            counts.get(&0).copied().unwrap_or(0)
        } else {
            0
        };
        counts
            .range(from_pass..)
            .map(|(_, count)| *count)
            .sum::<usize>()
            .saturating_add(untold)
    }

    /// The busy maps of `series` that still stand at `from_pass`, with their
    /// pass: the one of no told pass and every pass from it on — at most the
    /// last complete pass and the one running.
    fn standing_busy_maps(
        &self,
        series: &str,
        from_pass: u64,
    ) -> impl Iterator<Item = (u64, &BTreeMap<String, u64>)> {
        let marks = self.busy.get(series);
        let untold = marks
            .filter(|_| from_pass > 0)
            .and_then(|marks| marks.get(&0))
            .map(|months| (0, months));
        untold.into_iter().chain(
            marks
                .into_iter()
                .flat_map(move |marks| marks.range(from_pass..))
                .map(|(pass, months)| (*pass, months)),
        )
    }

    /// The first `most` months of `series` that a standing pass met busy and
    /// did not read, as (month, pass, run), in pass then month order.
    pub(crate) fn first_busy(
        &self,
        series: &str,
        from_pass: u64,
        most: usize,
    ) -> Vec<(&str, u64, u64)> {
        self.standing_busy_maps(series, from_pass)
            .flat_map(|(pass, months)| {
                months
                    .iter()
                    .map(move |(month, run)| (month.as_str(), pass, *run))
            })
            .take(most)
            .collect()
    }

    /// How many months of `series` the standing passes met busy: a sum over
    /// those passes, never a walk of the months.
    pub(crate) fn busy_in(&self, series: &str, from_pass: u64) -> usize {
        self.standing_busy_maps(series, from_pass)
            .map(|(_, months)| months.len())
            .sum()
    }

    /// How many months of the vendor the standing passes met busy and did
    /// not read: a sum over those passes.
    pub(crate) fn busy_count(&self, from_pass: u64) -> usize {
        let untold = if from_pass > 0 {
            self.busy_in.get(&0).copied().unwrap_or(0)
        } else {
            0
        };
        self.busy_in
            .range(from_pass..)
            .map(|(_, count)| *count)
            .sum::<usize>()
            .saturating_add(untold)
    }

    /// Every finding under `series` that still stands at `from_pass`.
    #[cfg(test)]
    pub(crate) fn standing(&self, series: &str, from_pass: u64) -> Vec<&Finding> {
        self.first_standing(series, from_pass, usize::MAX)
    }

    /// At most `most` standing findings of the vendor with the series each is
    /// in, in series and month order: what a summary names. The heads of the
    /// standing passes' maps, merged; never a walk of the findings.
    pub(crate) fn named(&self, from_pass: u64, most: usize) -> Vec<(&str, &Finding)> {
        let ranges = self
            .standing_maps(from_pass)
            .map(|held| held.range::<Key, _>(..))
            .collect();
        heads(ranges, most, |_| true)
            .into_iter()
            .map(|(key, finding)| (&*key.0, finding))
            .collect()
    }

    /// Every standing finding of the vendor, counted: those of no told pass
    /// and those of each pass from `from_pass` on. A sum over those passes,
    /// never a walk of the findings.
    pub(crate) fn standing_count(&self, from_pass: u64) -> usize {
        self.standing_maps(from_pass).map(BTreeMap::len).sum()
    }
}

/// The first `most` entries of the ordered `ranges` merged, while `within`
/// holds: each step compares the ranges' heads — one a standing pass — and
/// takes the smallest. A head outside `within` ends its range.
fn heads<'a>(
    mut ranges: Vec<btree_map::Range<'a, Key, Finding>>,
    most: usize,
    within: impl Fn(&Key) -> bool,
) -> Vec<(&'a Key, &'a Finding)> {
    let pull = |range: &mut btree_map::Range<'a, Key, Finding>| {
        range.next().filter(|(key, _)| within(key))
    };
    let mut next: Vec<Option<(&Key, &Finding)>> = ranges.iter_mut().map(pull).collect();
    let mut out = Vec::with_capacity(most.min(64));
    while out.len() < most {
        let Some(at) = next
            .iter()
            .enumerate()
            .filter_map(|(at, head)| head.map(|(key, _)| (key, at)))
            .min()
            .map(|(_, at)| at)
        else {
            break;
        };
        let (Some(slot), Some(range)) = (next.get_mut(at), ranges.get_mut(at)) else {
            break;
        };
        if let Some(taken) = slot.take() {
            out.push(taken);
        }
        *slot = pull(range);
    }
    out
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
    /// The one index of this ledger. It is extended and read under the
    /// vendor's lock and never handed out, so adding lines copies nothing.
    index: Index,
    /// Ledger bytes read by every extension so far: what a test counts to
    /// prove an unchanged ledger is not read again.
    #[cfg(test)]
    read_bytes: u64,
}

/// How often the index of each ledger path was thrown away and that SAME
/// ledger indexed again from its start: what a test counts to prove a growing
/// ledger is never taken for another file. By path, because the kept index is
/// one per vendor and tests of other roots share it.
#[cfg(test)]
static REBUILDS: Mutex<Vec<(PathBuf, u64)>> = Mutex::new(Vec::new());

impl Kept {
    fn new(path: PathBuf, file: (u64, u64)) -> Self {
        Self {
            path,
            file,
            consumed: 0,
            seam: Vec::new(),
            in_long_line: false,
            settled: None,
            index: Index::default(),
            #[cfg(test)]
            read_bytes: 0,
        }
    }

    /// Forgets everything indexed: what is at the name is not the ledger this
    /// index was built from. It is indexed again from its start.
    fn restart(&mut self, path: PathBuf, file: (u64, u64)) {
        #[cfg(test)]
        let read = self.read_bytes;
        #[cfg(test)]
        if self.path == path {
            let mut rebuilds = REBUILDS.lock().unwrap_or_else(PoisonError::into_inner);
            match rebuilds.iter_mut().find(|(of, _)| *of == path) {
                Some((_, count)) => *count += 1,
                None => rebuilds.push((path.clone(), 1)),
            }
        }
        *self = Self::new(path, file);
        #[cfg(test)]
        {
            self.read_bytes = read;
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
            self.restart(self.path.clone(), self.file);
            bytes = read(0, step)?;
        }
        #[cfg(test)]
        {
            self.read_bytes = self
                .read_bytes
                .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
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
                // IN PLACE: nothing already indexed is copied or walked.
                self.index.extend(&text);
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
        let index = &mut self.index;
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

/// Reads `vendor`'s findings index, brought up to its ledger's end first.
///
/// The kept one while the ledger has not moved; extended IN PLACE by the
/// lines appended since when it has ([`Kept::advance`]). `read` runs under
/// the vendor's lock, so it must be what the callers' are: bounded work on a
/// bounded number of findings. A ledger that does not exist is an empty index
/// — one that never ran is [`standing`]'s refusal, not this function's. One
/// request at a time extends or reads a vendor's index; the others wait for
/// it in this route's own admission pool, never in the bar seeks'.
///
/// # Errors
///
/// A ledger that is not a regular file or cannot be read: what the scrub
/// found cannot be told, and an empty index would say "nothing".
pub(crate) fn with_index<T>(
    root: &Path,
    vendor: Vendor,
    read: impl FnOnce(&Index) -> T,
) -> Result<T, String> {
    with_index_in_steps(root, vendor, INDEX_STEP_BYTES, read)
}

/// [`with_index`], reading at most `step` ledger bytes.
fn with_index_in_steps<T>(
    root: &Path,
    vendor: Vendor,
    step: u64,
    read: impl FnOnce(&Index) -> T,
) -> Result<T, String> {
    use std::os::unix::fs::MetadataExt as _;
    let path = root.join(cli::scrub_ledger_name(vendor));
    let unreadable = |why: std::io::Error| {
        format!(
            "{} cannot be read ({why}): what the full-block scrub of {} found cannot be told",
            path.display(),
            vendor.as_str()
        )
    };
    let Some(cell) = KEPT.get(vendor as usize) else {
        return Ok(read(&Index::default()));
    };
    let mut held = cell.lock().unwrap_or_else(PoisonError::into_inner);
    // THE LEDGER IS MEASURED UNDER THE LOCK, NEVER BEFORE IT. A request that
    // measured first and then waited here met an index a request ahead of it
    // had already taken PAST that stale length; "indexed further than the
    // file is long" read as another file at the name, and the whole index was
    // thrown away and built again from byte 0 — 208 times in one run of
    // `a_growing_ledger_read_by_many_is_never_indexed_again_from_its_start`,
    // and on a 128 MB ledger under a running scrub 1,055 of 80,000 answers
    // refused 429 behind those rebuilds. Measured here, the length can only
    // be the index's own or longer while the file is the same one.
    let (mut file, meta) = match open_plain(&path) {
        Ok(opened) => opened,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            return Ok(read(&Index::default()));
        }
        Err(why) => return Err(unreadable(why)),
    };
    let stamp = (meta.len(), meta.modified().ok());
    let identity = (meta.dev(), meta.ino());
    // THE SAME FILE, NO SHORTER, is the ledger this index was built from;
    // anything else at the name is indexed from its start.
    let kept = match held.as_mut() {
        Some(kept) if kept.path == path && kept.file == identity && kept.consumed <= stamp.0 => {
            kept
        }
        Some(kept) => {
            kept.restart(path.clone(), identity);
            kept
        }
        None => held.insert(Kept::new(path.clone(), identity)),
    };
    if kept.settled != Some(stamp) {
        kept.advance(&mut file, stamp, step).map_err(unreadable)?;
    }
    Ok(read(&kept.index))
}

/// Ledger bytes `vendor`'s index has read since this process started.
#[cfg(test)]
fn ledger_bytes_read(vendor: Vendor) -> u64 {
    KEPT.get(vendor as usize)
        .and_then(|cell| {
            cell.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .as_ref()
                .map(|kept| kept.read_bytes)
        })
        .unwrap_or(0)
}

/// How often the index of the ledger at `path` was thrown away and that same
/// ledger indexed again from its start, since this process started.
#[cfg(test)]
fn index_rebuilds(path: &Path) -> u64 {
    REBUILDS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|(of, _)| of == path)
        .map_or(0, |(_, count)| *count)
}

/// A copy of `vendor`'s index after bringing it up to its ledger's end: what
/// the tests look at. No request copies an index.
#[cfg(test)]
fn index(root: &Path, vendor: Vendor) -> Result<Index, String> {
    with_index(root, vendor, Index::clone)
}

/// [`index`], reading at most `step` ledger bytes.
#[cfg(test)]
fn index_in_steps(root: &Path, vendor: Vendor, step: u64) -> Result<Index, String> {
    with_index_in_steps(root, vendor, step, Index::clone)
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
    let from_pass = state.standing_from();
    // EVERYTHING READ FROM THE INDEX IS READ HERE, under its lock, and is
    // bounded: two sums and at most `MAX_FINDINGS` findings a key.
    let rendered = with_index(root, vendor, |found| {
        let mut out = String::new();
        let _ = write!(
            out,
            r#","recorded":true,"pass":{},"state":{},"totals":{},"standing_from_pass":{from_pass},"standing_findings":{},"standing_busy":{},"indexed_bytes":{},"truncated":{},"series":["#,
            state.pass,
            render::json_string(if state.complete {
                "complete"
            } else {
                "in progress"
            }),
            state.totals_json(),
            found.standing_count(from_pass),
            found.busy_count(from_pass),
            found.indexed_bytes,
            found.truncated
        );
        for (n, (text, dir)) in series.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            series_json(&mut out, found, (text, dir.as_str()), timeframe, from_pass);
        }
        out.push_str("]}");
        out
    });
    match rendered {
        Ok(rest) => out.push_str(&rest),
        Err(why) => {
            let _ = write!(
                out,
                r#","recorded":false,"why":{},"series":[]}}"#,
                render::json_string(&why)
            );
        }
    }
    Ok(out)
}

/// One series' object of the answer: its first standing findings, how many
/// more stand, and the months a standing pass met busy and did not read.
fn series_json(
    out: &mut String,
    found: &Index,
    (text, dir): (&str, &str),
    timeframe: store::path::Timeframe,
    from_pass: u64,
) {
    let standing = found.first_standing(dir, from_pass, MAX_FINDINGS);
    let _ = write!(
        out,
        r#"{{"key":{},"timeframe":{},"more":{},"findings":["#,
        render::json_string(text),
        render::json_string(timeframe.as_str()),
        found
            .standing_in(dir, from_pass)
            .saturating_sub(standing.len())
    );
    for (m, finding) in standing.iter().enumerate() {
        if m > 0 {
            out.push(',');
        }
        finding_json(out, finding);
    }
    // THE MONTHS A STANDING PASS DID NOT READ: a pass that met a
    // month busy does not vouch for it, and says so by name.
    let busy = found.first_busy(dir, from_pass, MAX_FINDINGS);
    out.push_str(r#"],"busy":["#);
    for (m, (month, pass, run)) in busy.iter().enumerate() {
        if m > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"month":{},"pass":{pass},"run":{run}}}"#,
            render::json_string(month)
        );
    }
    let _ = write!(
        out,
        r#"],"busy_more":{}}}"#,
        found.busy_in(dir, from_pass).saturating_sub(busy.len())
    );
}

/// One finding as JSON.
pub(crate) fn finding_json(out: &mut String, finding: &Finding) {
    let number = |value: Option<u64>| value.map_or_else(|| "null".to_owned(), |n| n.to_string());
    let _ = write!(
        out,
        r#"{{"kind":{},"month":{},"block":{},"first":{},"last":{},"pass":{},"run":{},"kept_pass":{},"kept_run":{},"why":{}}}"#,
        render::json_string(finding.kind),
        render::json_string(&finding.month),
        number(finding.block.map(|(block, _, _)| block)),
        number(finding.block.map(|(_, first, _)| first)),
        number(finding.block.map(|(_, _, last)| last)),
        finding.pass,
        finding.run,
        number(finding.kept.map(|(pass, _)| pass)),
        number(finding.kept.map(|(_, run)| run)),
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
    // A POOL BAR SEEKS DO NOT SHARE (WP-81 repair 2): see the module header.
    let ran = crate::detail::run_ops_read(move || answer(&site.store_root, &query)).await;
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
                crate::detail::MAX_OPS_READ_CONCURRENT,
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
                "pass":2,"run":200,"kept_pass":null,"kept_run":null,
                "why":"block 21 sealed 1 computed 2"})
        );
        // PASS 1's busy month is pass 1's: pass 2 completed and read it.
        assert_eq!(nifty["busy"], serde_json::json!([]));
        assert_eq!(parsed["standing_busy"], 0);
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
        // ... and the month pass 1 met busy, with no finding to keep, is
        // named as not read by it: pass 1 does not vouch for 2020-10.
        assert_eq!(
            parsed["series"][0]["busy"],
            serde_json::json!([{"month":"2020-10","pass":1,"run":100}])
        );
        assert_eq!(parsed["series"][0]["busy_more"], 0);
        assert_eq!(parsed["standing_busy"], 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// One `done` cursor line of `pass` with `busy` months.
    fn done_line(run: u64, pass: u64, busy: u64) -> String {
        format!(
            "done run={run} pass={pass} months=4 blocks=9 bad=0 unverifiable=0 misfiled=0 \
             unbound=0 busy={busy} missing=0\n"
        )
    }

    /// **A MONTH A PASS MET BUSY KEEPS WHAT EARLIER PASSES FOUND IN IT, AND
    /// IS NAMED AS NOT READ (DB-B74; WP-81 repair 3).** Pass 1 found two bad
    /// blocks and a short sidecar in NIFTY 2020-07, one bad block in 2020-08
    /// and one in BANKNIFTY 2020-07. Pass 2 met NIFTY 2020-07 held by a
    /// writer (`busy`), read every other month clean, and completed.
    ///
    /// Before this the `busy` line was skipped, so with pass 2 complete the
    /// answer was `findings: []` for NIFTY — "pass 2 names no damaged block"
    /// on /db and "0 bad blocks" on /ingest, while no pass had read 2020-07
    /// again and its block was still refused on read (second agent, 12 Oct
    /// 2026, on a clone of the operator's NIFTY 1s 2020-07). Now the three
    /// findings of the busy month stand, each saying the pass that found it
    /// AND the pass that kept it; the two a complete pass read clean are gone;
    /// and the month is named busy. A third pass that meets it busy again
    /// keeps them again; a fourth that reads it keeps only what it finds.
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one month followed through five passes, every answer asserted"
    )]
    fn a_month_a_pass_met_busy_keeps_its_findings_and_is_named_as_not_read() {
        let root = root_of("scrub-route-busy");
        let nifty = "bars/gdfl/NSE/INDEX/NIFTY/1s";
        let bank = "bars/gdfl/NSE/INDEX/BANKNIFTY/1s";
        let ask = "feed=gdfl&timeframe=1s&key=NSE-INDEX-NIFTY&key=NSE-INDEX-BANKNIFTY";
        let asked = |root: &Path| -> serde_json::Value {
            serde_json::from_str(&answer(root, ask).expect("answered")).expect("JSON")
        };
        let mut cursor = done_line(100, 1, 0);
        let mut ledger = format!(
            "run 100 vendor=gdfl commit=unstamped pass=1 resume=-\n\
             bad {nifty}/2020-07.bin block=16 records=1168-1240 run=100 why=was sealed 1 and now reads 2\n\
             bad {nifty}/2020-07.bin block=40 records=2920-2992 run=100 why=was sealed 3 and now reads 4\n\
             unverifiable {nifty}/2020-07.bin run=100 why=its sidecar is short\n\
             bad {nifty}/2020-08.bin block=4 records=292-364 run=100 why=was sealed 5 and now reads 6\n\
             bad {bank}/2020-07.bin block=2 records=146-218 run=100 why=was sealed 7 and now reads 8\n"
        );
        write(&root, Vendor::Gdfl, &cursor, &ledger);
        let first = asked(&root);
        assert_eq!(first["standing_findings"], 5);
        assert_eq!(first["standing_busy"], 0);
        assert_eq!(first["series"][0]["busy"], serde_json::json!([]));

        // PASS 2: the month is held by a writer when the pass reaches it.
        cursor.push_str(&done_line(200, 2, 1));
        let _ = write!(
            ledger,
            "run 200 vendor=gdfl commit=unstamped pass=2 resume=-\nbusy {nifty}/2020-07.bin run=200\n"
        );
        write(&root, Vendor::Gdfl, &cursor, &ledger);
        let second = asked(&root);
        assert_eq!(
            (&second["pass"], &second["state"]),
            (&serde_json::json!(2), &serde_json::json!("complete"))
        );
        assert_eq!(second["standing_from_pass"], 2);
        assert_eq!(second["totals"]["busy"], 1);
        assert_eq!(second["standing_findings"], 3, "{second}");
        assert_eq!(second["standing_busy"], 1);
        let kept = second["series"][0]["findings"].as_array().expect("a list");
        assert_eq!(
            kept.iter()
                .map(|f| (
                    f["month"].as_str().unwrap(),
                    f["kind"].as_str().unwrap(),
                    f["block"].as_u64()
                ))
                .collect::<Vec<_>>(),
            [
                ("2020-07", "unverifiable", None),
                ("2020-07", "bad", Some(16)),
                ("2020-07", "bad", Some(40))
            ],
            "{second}"
        );
        assert_eq!(
            kept[1],
            serde_json::json!({"kind":"bad","month":"2020-07","block":16,"first":1168,"last":1240,
                "pass":1,"run":100,"kept_pass":2,"kept_run":200,
                "why":"was sealed 1 and now reads 2"}),
            "found by pass 1's run, kept by pass 2's"
        );
        assert_eq!(second["series"][0]["more"], 0);
        assert_eq!(
            second["series"][0]["busy"],
            serde_json::json!([{"month":"2020-07","pass":2,"run":200}])
        );
        assert_eq!(second["series"][0]["busy_more"], 0);
        // WHAT A COMPLETE PASS READ CLEAN IS GONE: 2020-08 and BANKNIFTY.
        assert_eq!(second["series"][1]["findings"], serde_json::json!([]));
        assert_eq!(second["series"][1]["busy"], serde_json::json!([]));
        // ... and `/disk.json`'s summary names the kept ones too.
        let tile: serde_json::Value =
            serde_json::from_str(&super::super::disk_route::scrub_json(&root, Vendor::Gdfl))
                .expect("JSON");
        assert_eq!(tile["standing_findings"], 3);
        assert_eq!(tile["standing_busy"], 1);
        assert_eq!(tile["named"][1]["finding"]["block"], 16);
        assert_eq!(tile["named"][1]["finding"]["kept_pass"], 2);

        // PASS 3, IN PROGRESS AND THEN COMPLETE, meets it busy again — twice
        // in the ledger, a killed run's line and the resumed run's — and it
        // is one busy month and the same three findings, kept by pass 3.
        let _ = write!(
            ledger,
            "run 300 vendor=gdfl commit=unstamped pass=3 resume=-\nbusy {nifty}/2020-07.bin run=300\n\
             run 301 vendor=gdfl commit=unstamped pass=3 resume=-\nbusy {nifty}/2020-07.bin run=301\n"
        );
        write(
            &root,
            Vendor::Gdfl,
            &format!("{cursor}progress run=301 pass=3 {T1} cursor=x\n"),
            &ledger,
        );
        let running = asked(&root);
        assert_eq!(running["standing_from_pass"], 2);
        assert_eq!(running["standing_findings"], 3);
        assert_eq!(running["standing_busy"], 2, "pass 2's and pass 3's");
        assert_eq!(
            running["series"][0]["busy"],
            serde_json::json!([
                {"month":"2020-07","pass":2,"run":200},
                {"month":"2020-07","pass":3,"run":301}
            ])
        );
        cursor.push_str(&done_line(301, 3, 1));
        write(&root, Vendor::Gdfl, &cursor, &ledger);
        let third = asked(&root);
        assert_eq!(third["standing_from_pass"], 3);
        assert_eq!(third["standing_findings"], 3, "{third}");
        assert_eq!(third["standing_busy"], 1);
        for finding in third["series"][0]["findings"].as_array().expect("a list") {
            assert_eq!(
                (&finding["pass"], &finding["run"]),
                (&serde_json::json!(1), &serde_json::json!(100))
            );
            assert_eq!(
                (&finding["kept_pass"], &finding["kept_run"]),
                (&serde_json::json!(3), &serde_json::json!(301))
            );
        }

        // PASS 4: a run meets it busy and is killed; the run that resumes
        // the pass READS it and finds block 16 alone. The month was read:
        // no busy mark, block 16 is pass 4's own finding, and block 40 and
        // the sidecar — read again by a complete pass and not found — are gone.
        let _ = write!(
            ledger,
            "run 400 vendor=gdfl commit=unstamped pass=4 resume=-\nbusy {nifty}/2020-07.bin run=400\n\
             run 401 vendor=gdfl commit=unstamped pass=4 resume=-\n\
             bad {nifty}/2020-07.bin block=16 records=1168-1240 run=401 why=was sealed 1 and now reads 2\n"
        );
        cursor.push_str(&done_line(401, 4, 0));
        write(&root, Vendor::Gdfl, &cursor, &ledger);
        let fourth = asked(&root);
        assert_eq!(fourth["standing_findings"], 1, "{fourth}");
        assert_eq!(fourth["standing_busy"], 0);
        assert_eq!(fourth["series"][0]["busy"], serde_json::json!([]));
        assert_eq!(
            fourth["series"][0]["findings"],
            serde_json::json!([{"kind":"bad","month":"2020-07","block":16,"first":1168,"last":1240,
                "pass":4,"run":401,"kept_pass":null,"kept_run":null,
                "why":"was sealed 1 and now reads 2"}])
        );

        // PASS 5 reads it clean: nothing stands, nothing is busy.
        ledger.push_str("run 500 vendor=gdfl commit=unstamped pass=5 resume=-\n");
        cursor.push_str(&done_line(500, 5, 0));
        write(&root, Vendor::Gdfl, &cursor, &ledger);
        let fifth = asked(&root);
        assert_eq!(fifth["standing_findings"], 0);
        assert_eq!(fifth["series"][0]["findings"], serde_json::json!([]));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The edges of a `busy` line: a month with nothing found in it is only
    /// named; a month of ANOTHER series or another month keeps nothing; a
    /// busy line of the first pass has no earlier pass to keep from; a run
    /// that cannot be dated keeps what the newest passes hold as "not told",
    /// which stands until a dated pass finds it again; and a line that names
    /// no month file is no busy line.
    #[test]
    fn the_edges_of_a_busy_line() {
        let a = "bars/gdfl/NSE/INDEX/A/1s";
        let b = "bars/gdfl/NSE/INDEX/AB/1s";
        let ledger = format!(
            "busy {a}/2020-01.bin run=10\n\
             run 10 vendor=gdfl commit=c pass=1 resume=-\n\
             busy {a}/2020-02.bin run=10\n\
             bad {a}/2020-03.bin block=1 records=73-145 run=10 why=crc\n\
             bad {b}/2020-03.bin block=2 records=146-218 run=10 why=crc\n\
             missing {a}/2020-04.bin run=10\n\
             run 20 vendor=gdfl commit=c pass=2 resume=-\n\
             busy {b}/2020-03.bin run=20\n\
             busy {a}/2020-05.bin run=20\n\
             busy {a}/2020-04.bin run=20\n\
             busy {a}/nothing run=20\n\
             busy\n\
             busy {a}/2020-04.bin\n"
        );
        let index = Index::of(&ledger);
        // PASS 2 COMPLETE: A's bad block of 2020-03 was read clean by pass 2
        // (no line), so it is gone; A's missing 2020-04 and AB's bad block
        // were met busy, so they stand, kept by pass 2's run.
        let kept: Vec<(&str, Option<(u64, u64)>)> = index
            .standing(a, 2)
            .iter()
            .map(|f| (f.month.as_str(), f.kept))
            .collect();
        // THE LAST LINE names no run: it cannot be dated, so what pass 2
        // holds of 2020-04 is kept as "not told" (pass 0, run 0).
        assert_eq!(kept, [("2020-04", Some((0, 0)))]);
        assert_eq!(index.standing(a, 9).len(), 1, "undated, so it stands on");
        let other: Vec<(&str, Option<(u64, u64)>)> = index
            .standing(b, 2)
            .iter()
            .map(|f| (f.month.as_str(), f.kept))
            .collect();
        assert_eq!(
            other,
            [("2020-03", Some((2, 20)))],
            "A's 2020-03 is not AB's"
        );
        assert!(index.standing(b, 3).is_empty(), "a pass that read it clean");
        // BUSY MONTHS: the undated ones (the first line, before any `run`
        // line, and the last) stand at every pass; pass 1's and pass 2's
        // stand from their pass on.
        assert_eq!(
            index.first_busy(a, 1, 9),
            [
                ("2020-01", 0, 10),
                ("2020-04", 0, 0),
                ("2020-02", 1, 10),
                ("2020-04", 2, 20),
                ("2020-05", 2, 20)
            ]
        );
        assert_eq!(index.busy_in(a, 1), 5);
        assert_eq!(index.busy_in(a, 2), 4);
        assert_eq!(index.busy_in(a, 3), 2);
        assert_eq!(index.first_busy(a, 2, 3).len(), 3, "capped");
        assert_eq!(index.first_busy(b, 2, 9), [("2020-03", 2, 20)]);
        assert_eq!(index.busy_count(2), 5);
        assert_eq!(index.busy_count(3), 2);
        assert_eq!(index.busy_count(0), 6, "every month ever met busy");
        assert_eq!(index.busy_in("bars/gdfl/NSE/INDEX/NONE/1s", 0), 0);
        // A DATED PASS THAT FINDS THE UNDATED FINDING AGAIN dates it.
        let mut found = index.clone();
        found.extend(&format!(
            "run 30 vendor=gdfl commit=c pass=3 resume=-\nmissing {a}/2020-04.bin run=30\n"
        ));
        let again = found.standing(a, 3);
        assert_eq!((again.len(), again[0].pass, again[0].kept), (1, 3, None));
        assert!(found.standing(a, 4).is_empty());
    }

    /// **THE INDEX KEEPS EXACTLY WHAT THE RULE KEEPS, ON A LEDGER OF BUSY
    /// MONTHS (WP-81 repair 3).** 12,000 lines over 40 series, 12 months and
    /// six passes, one line in eight a `busy`, some by a run with no `run`
    /// line, indexed in pieces. A model applies the rule of the module header
    /// by WALKING every finding at each busy line — what the index must agree
    /// with while it seeks one pass's map. Every standing set, count and busy
    /// list is compared at every pass.
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one ledger, one model, and every question asked of both"
    )]
    fn the_index_keeps_what_a_walk_of_every_finding_at_each_busy_line_keeps() {
        type Same = (String, String, Option<u64>, &'static str);
        // (the pass it stands under, the pass that found it, who kept it)
        type Held = (u64, u64, Option<(u64, u64)>);
        // (month, block, kind, the pass that found it, who kept it)
        type Row<'a> = (&'a str, Option<u64>, &'a str, u64, Option<(u64, u64)>);
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = |below: u64| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) % below
        };
        let mut ledger = String::new();
        let mut model: BTreeMap<Same, Held> = BTreeMap::new();
        // (series, pass, month) -> run
        let mut marks: BTreeMap<(String, u64, String), u64> = BTreeMap::new();
        for pass in 1..=6_u64 {
            let told = pass * 10;
            let _ = writeln!(
                ledger,
                "run {told} vendor=groww commit=c pass={pass} resume=-"
            );
            let newest = pass;
            for _ in 0..2_000 {
                let series = format!("bars/groww/NSE/CASH/S{}/1min", next(40));
                let month = format!("2024-{:02}", 1 + next(12));
                // RUN 7 HAS NO `run` LINE: it cannot be dated.
                let (run, dated) = if next(25) == 0 { (7, 0) } else { (told, pass) };
                let file = format!("{series}/{month}.bin");
                if next(8) == 0 {
                    let _ = writeln!(ledger, "busy {file} run={run}");
                    marks.insert((series.clone(), dated, month.clone()), run);
                    for (same, held) in &mut model {
                        if same.0 != series || same.1 != month {
                            continue;
                        }
                        let stood = if dated == 0 {
                            held.0 >= newest.saturating_sub(1).max(1)
                        } else {
                            held.0 > 0 && held.0 + 1 == dated
                        };
                        if stood || (held.0 == dated && held.2.is_some()) {
                            *held = (dated, held.1, Some((dated, run)));
                        }
                    }
                    continue;
                }
                let (kind, block) = match next(3) {
                    0 => ("missing", None),
                    1 => ("unverifiable", None),
                    _ => ("bad", Some(next(4))),
                };
                let _ = match block {
                    Some(block) => writeln!(
                        ledger,
                        "bad {file} block={block} records={}-{} run={run} why=crc",
                        block * 73,
                        block * 73 + 72
                    ),
                    None => writeln!(ledger, "{kind} {file} run={run} why=said"),
                };
                // A DATED PASS READ THE MONTH: its busy mark goes, and what
                // the mark kept goes back under the pass before.
                if dated > 0
                    && marks
                        .remove(&(series.clone(), dated, month.clone()))
                        .is_some()
                {
                    let earlier = marks
                        .get(&(series.clone(), dated - 1, month.clone()))
                        .copied();
                    for (same, held) in &mut model {
                        if same.0 == series
                            && same.1 == month
                            && held.0 == dated
                            && held.2.is_some_and(|(by, _)| by == dated)
                        {
                            let kept = (held.1 != dated - 1)
                                .then(|| (dated - 1, earlier.unwrap_or_default()));
                            *held = (dated - 1, held.1, kept);
                        }
                    }
                }
                model.insert((series, month, block, kind), (dated, dated, None));
            }
        }
        let mut index = Index::default();
        let mut from = 0;
        while from < ledger.len() {
            let upto = (from + 9_000).min(ledger.len());
            let end = ledger[..upto].rfind('\n').map_or(ledger.len(), |at| at + 1);
            index.extend(&ledger[from..end]);
            from = end;
        }
        assert!(
            model.values().filter(|held| held.2.is_some()).count() > 200,
            "the ledger must keep findings for the comparison to mean anything"
        );
        for from_pass in 0..=7_u64 {
            let stands = |under: u64| under == 0 || under >= from_pass;
            let all: Vec<(&Same, &Held)> =
                model.iter().filter(|(_, held)| stands(held.0)).collect();
            assert_eq!(
                index.standing_count(from_pass),
                all.len(),
                "pass {from_pass}"
            );
            assert_eq!(
                index.busy_count(from_pass),
                marks.keys().filter(|mark| stands(mark.1)).count(),
                "busy at pass {from_pass}"
            );
            for series in 0..40_u64 {
                let dir = format!("bars/groww/NSE/CASH/S{series}/1min");
                let modelled: Vec<Row<'_>> = all
                    .iter()
                    .filter(|(same, _)| same.0 == dir)
                    .map(|(same, held)| (same.1.as_str(), same.2, same.3, held.1, held.2))
                    .collect();
                let indexed: Vec<Row<'_>> = index
                    .standing(&dir, from_pass)
                    .iter()
                    .map(|f| {
                        (
                            f.month.as_str(),
                            f.block.map(|b| b.0),
                            f.kind,
                            f.pass,
                            f.kept,
                        )
                    })
                    .collect();
                assert_eq!(indexed, modelled, "S{series} from pass {from_pass}");
                assert_eq!(index.standing_in(&dir, from_pass), modelled.len());
                let mut busy: Vec<(&str, u64, u64)> = marks
                    .iter()
                    .filter(|(mark, _)| mark.0 == dir && stands(mark.1))
                    .map(|(mark, run)| (mark.2.as_str(), mark.1, *run))
                    .collect();
                busy.sort_by_key(|(month, pass, _)| (*pass, *month));
                assert_eq!(index.first_busy(&dir, from_pass, usize::MAX), busy);
                assert_eq!(index.busy_in(&dir, from_pass), busy.len());
            }
        }
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
        // KEPT: an unchanged ledger is not read again.
        let read = ledger_bytes_read(Vendor::Dhan);
        let _kept = index(&root, Vendor::Dhan).expect("kept");
        assert_eq!(ledger_bytes_read(Vendor::Dhan), read, "reused, not re-read");
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
        assert!(
            ledger_bytes_read(Vendor::Dhan) > read,
            "the ledger moved, so it was read"
        );
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
        let read_at_first = ledger_bytes_read(Vendor::TrueData);
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
        // ONLY WHAT WAS APPENDED WAS READ, twice at most (the fragment is read
        // again with its end), plus the seam each read begins with — never
        // the 6.6 MiB already indexed.
        let appended = std::fs::metadata(&path).unwrap().len() - ledger.len() as u64;
        let read_since = ledger_bytes_read(Vendor::TrueData) - read_at_first;
        assert!(
            read_since <= 2 * (appended + SEAM_BYTES as u64),
            "{read_since} bytes read for {appended} appended"
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

    /// **THE ORDERED INDEX ANSWERS WHAT A WALK AND A SORT OF EVERY FINDING
    /// ANSWERS (WP-81 repair 2).** 60,000 ledger lines over 1,500 series and
    /// four passes — findings found again by later passes, findings left
    /// behind by them, runs whose `run` line was never read (pass 0), two
    /// kinds on one block — indexed in pieces, as requests index a growing
    /// ledger. At every standing pass the first findings of a series, a
    /// series' count, the vendor's count and the summary's first names equal
    /// those of a model that keeps the newest of each finding in a map and
    /// filters and sorts ALL of them per question: what the route did on
    /// every request, and what it must agree with now that it does not.
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one ledger, one model, and every question asked of both"
    )]
    fn the_ordered_index_answers_what_a_walk_and_a_sort_of_every_finding_answers() {
        type Same = (String, String, Option<u64>, &'static str);
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = |below: u64| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) % below
        };
        let mut ledger = String::new();
        // RUNS 1..=4 ARE PASSES 1..=4; RUN 9 HAS NO `run` LINE: pass 0.
        for run in 1..=4_u64 {
            let _ = writeln!(
                ledger,
                "run {run} vendor=groww commit=c pass={run} resume=-"
            );
            for _ in 0..15_000 {
                let series = next(1_500);
                let month = 1 + next(12);
                let run_said = if next(40) == 0 { 9 } else { run };
                let file = format!("bars/groww/NSE/CASH/S{series}/1min/2024-{month:02}.bin");
                let _ = match next(4) {
                    0 => writeln!(ledger, "missing {file} run={run_said}"),
                    1 => writeln!(ledger, "unverifiable {file} run={run_said} why=no sidecar"),
                    _ => {
                        let block = next(6);
                        writeln!(
                            ledger,
                            "bad {file} block={block} records={}-{} run={run_said} why=crc",
                            block * 73,
                            block * 73 + 72
                        )
                    }
                };
            }
        }
        // INDEXED IN PIECES, cut at line ends.
        let mut index = Index::default();
        let mut from = 0;
        while from < ledger.len() {
            let upto = (from + 70_000).min(ledger.len());
            let end = ledger[..upto].rfind('\n').map_or(ledger.len(), |at| at + 1);
            index.extend(&ledger[from..end]);
            from = end;
        }
        // THE MODEL: the newest of each (series, month, block, kind).
        let mut model: HashMap<Same, Finding> = HashMap::new();
        for (series, findings) in Index::of(&ledger)
            .by_pass
            .values()
            .flat_map(|held| held.iter())
            .map(|(key, finding)| (key.0.to_string(), finding))
        {
            let same = (
                series,
                findings.month.clone(),
                findings.block.map(|(block, _, _)| block),
                findings.kind,
            );
            assert!(model.insert(same, findings.clone()).is_none(), "one each");
        }
        assert!(model.len() > 20_000, "{}", model.len());
        // ... AND IT IS THE LEDGER'S, read line by line with no index at all.
        let mut by_line: HashMap<Same, u64> = HashMap::new();
        for line in ledger.lines().filter(|line| !line.starts_with("run ")) {
            let mut words = line.split(' ');
            let kind = match words.next().unwrap() {
                "bad" => "bad",
                "missing" => "missing",
                _ => "unverifiable",
            };
            let (series, file) = words.next().unwrap().rsplit_once('/').unwrap();
            let number = |key: &str| field(line, key).and_then(|v| v.parse::<u64>().ok());
            let run = number("run").unwrap();
            by_line.insert(
                (
                    series.to_owned(),
                    file.trim_end_matches(".bin").to_owned(),
                    number("block"),
                    kind,
                ),
                if run == 9 { 0 } else { run },
            );
        }
        assert_eq!(by_line.len(), model.len());
        for (same, pass) in &by_line {
            assert_eq!(model[same].pass, *pass, "{same:?}");
        }

        for from_pass in 0..=5_u64 {
            let stands = |finding: &Finding| finding.pass == 0 || finding.pass >= from_pass;
            let mut all: Vec<(&Same, &Finding)> = model
                .iter()
                .filter(|(_, finding)| stands(finding))
                .collect();
            all.sort_by(|a, b| a.0.cmp(b.0));
            assert_eq!(
                index.standing_count(from_pass),
                all.len(),
                "pass {from_pass}"
            );
            let named = index.named(from_pass, 16);
            assert_eq!(named.len(), all.len().min(16));
            for ((series, finding), (same, modelled)) in named.iter().zip(&all) {
                assert_eq!((*series, *finding), (same.0.as_str(), *modelled));
            }
            for series in [0_u64, 1, 7, 499, 1_499, 5_000] {
                let dir = format!("bars/groww/NSE/CASH/S{series}/1min");
                let of_series: Vec<&Finding> = all
                    .iter()
                    .filter(|(same, _)| same.0 == dir)
                    .map(|(_, finding)| *finding)
                    .collect();
                assert_eq!(index.standing_in(&dir, from_pass), of_series.len());
                assert_eq!(index.standing(&dir, from_pass), of_series);
                assert_eq!(
                    index.first_standing(&dir, from_pass, 3),
                    of_series.iter().take(3).copied().collect::<Vec<_>>(),
                    "S{series} from pass {from_pass}"
                );
            }
        }
        // A SERIES WHOSE NAME IS A PREFIX OF ANOTHER'S takes none of its
        // findings: `S1` is not `S10`..`S19`.
        let one = index.standing("bars/groww/NSE/CASH/S1/1min", 0);
        assert!(!one.is_empty());
        assert_eq!(
            one.len(),
            model
                .keys()
                .filter(|same| same.0 == "bars/groww/NSE/CASH/S1/1min")
                .count()
        );
    }

    /// **A LEDGER THAT GROWS WHILE MANY REQUESTS READ IT IS NEVER TAKEN FOR
    /// ANOTHER FILE (WP-81 repair 2).** A request measured the ledger, THEN
    /// waited for the vendor's lock. A request ahead of it had meanwhile
    /// indexed further than that stale length, so the ledger looked SHORTER
    /// than its own index — "another file at the name" — and everything
    /// indexed was thrown away and read again from byte 0, 16 MiB a request,
    /// with every other request waiting behind it. On a 128 MB ledger being
    /// appended, 20 clients got 1,055 of 80,000 answers refused 429 and waits
    /// of up to 869 ms. Eight readers and one appender: the index is never
    /// rebuilt, and it ends holding every line.
    #[test]
    fn a_growing_ledger_read_by_many_is_never_indexed_again_from_its_start() {
        use std::io::Write as _;
        let root = root_of("scrub-route-growing");
        let vendor = Vendor::Zerodha;
        let line =
            |n: usize| format!("missing bars/zerodha/NSE/CASH/G{n}/1min/2025-05.bin run=1\n");
        let mut ledger = String::from("run 1 vendor=zerodha commit=c pass=1 resume=-\n");
        for n in 0..2_000 {
            ledger.push_str(&line(n));
        }
        write(&root, vendor, &format!("done run=1 pass=1 {T1}\n"), &ledger);
        assert_eq!(
            with_index(&root, vendor, |index| index.standing_count(1)),
            Ok(2_000)
        );
        let path = root.join(cli::scrub_ledger_name(vendor));
        assert_eq!(index_rebuilds(&path), 0);
        let done = std::sync::atomic::AtomicBool::new(false);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    while !done.load(std::sync::atomic::Ordering::Acquire) {
                        let count = with_index(&root, vendor, |index| index.standing_count(1))
                            .expect("indexed");
                        assert!(count >= 2_000, "never fewer than were indexed: {count}");
                    }
                });
            }
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            for n in 2_000..22_000 {
                file.write_all(line(n).as_bytes()).unwrap();
            }
            done.store(true, std::sync::atomic::Ordering::Release);
        });
        assert_eq!(
            with_index(&root, vendor, |index| index.standing_count(1)),
            Ok(22_000)
        );
        assert_eq!(
            index_rebuilds(&path),
            0,
            "a growing ledger was indexed again from its start"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

//! Zerodha verification: every stored rung of every day checked against what
//! the vendor answered (when it was kept), against its own minutes, and
//! against the calendar of record; the records spooled, then merged into one
//! verdict generation per (segment, timeframe, day) (IA-9; D-4811, D-4822,
//! D-4857, D-4858, D-4881, D-4921).
//!
//! # What is checked, per (instrument, day, rung)
//!
//! * **1min against the kept answer** (D-4811). The kept body is decoded by
//!   `http::decode_body`, the one shared step, which already snapped every
//!   rupee price half-up; the independent landing oracle here applies ONLY the
//!   scale (`fetch::to_paisa` with `http::DECODED_PRICE_SCALE`), never a second
//!   snap (IA-ZV-06). It applies D-3351's row rules itself: a candle off the
//!   09:15-anchored grid is refused, identical candles at one stamp count once
//!   (`dup_minutes`), and every candle of a stamp whose candles disagree is
//!   refused (`conflicting_candles`); every other row lands (IA-ZV-05). The
//!   refusals it cannot judge from the answer alone (`cash_eligibility_unknown`,
//!   `price_off_grid`, and WP-13d's `before_first_day_bar` and `foreign_level`)
//!   are read from the landing's own refused-row records (`.rsx`, DB-G46) and
//!   counted by word; where the oracle's count of the words it judges differs
//!   from the landing's, the record says `quality_differs`. A stored minute is
//!   then compared exactly with the answer's: `missing`, `extra` (IA-ZV-01),
//!   `value_diff`, `volume_diff` and `oi_diff`, an absent OI and a zero OI kept
//!   apart. A stored minute no kept answer asked for is never vouched for: the
//!   day reads `no_response_kept`, never an old `match` (IA-ZV-10).
//! * **A re-pull** (a later receipt of the same day) that differs from the
//!   store is `vendor_revised` with detail `receipts{a, b}`, never
//!   `value_mismatch` (VERIFY-13, IA-ZV-08): the store kept the first.
//! * **Every derived rung** (D-4858): 2, 3, 5, 10, 15, 30 and 60 min refolded
//!   from stored 1min by this module's own fold under D-3321 (an INDEX or
//!   futures bucket missing an owed minute is withheld, `withheld_by_rule`
//!   `index_bucket_incomplete`; a CASH bucket folds as partial), anchored at
//!   09:15 IST as the store's grid is (D-1447). A month whose `derive` ledger
//!   line is `v=1` is compared by value under rule set 1 ("D-3321 as of
//!   WP-13b"); a month with no line was derived before the ledger and reads
//!   `built_under_earlier_rule` (`pre_ledger`), its counts recorded, never
//!   judged by value (IA-ZV-07). Options are never derived and have no
//!   derived record (IA-ZV-03).
//! * **1day** against its own kept answer, and by `daycheck::compare` re-run
//!   over the day's minutes: a day that differs or contradicts sets
//!   `daycheck_differs` with detail `daycheck{...}` (D-3347).
//! * **Completeness** against the calendar of record (D-4857): the minutes a
//!   day owes come from `ingest::repair_plan::owed` (the one function the
//!   repair plan and the stored-month gap ledger share). Owed minutes inside a
//!   kept answer's asked window that the answer lacks are the vendor's hole,
//!   said by the answer (`answer_short` when it stopped before the end it was
//!   asked for, RA-300). The rest are classified by the gap ledger's `why`
//!   (IA-ZV-04): `vendor_empty` is `vendor_hole`; `withheld_unmeasured_session`
//!   is `withheld_by_rule`; `calendar_unverified`, `calendar_past_record`,
//!   `unordered_stamps` and `pull_halted` (a pull that halted before it asked
//!   for the day, RA-301, IA-ZV-12) are `not_compared`; `refused_day`, `refused_front` and
//!   R3-18's `stored_short` are `owed` (DB-G46: a month stored before the
//!   ledger gets its reasons from the stored-month ledger, never `no_ledger`);
//!   no line in a month the ledger covers is `missing_in_store`
//!   (`unexplained`), the one Zerodha store defect; a month the ledger does
//!   not cover at all is `no_ledger`. Where GDFL's one-second bars of the same
//!   instrument hold no second of those minutes either, they are absent from
//!   both vendors: `vendor_hole`, reason `neither_vendor_has_it` (DB-A12); a
//!   minute GDFL traded sets `peer_minute_absent` with detail `peer{...}`.
//! * **A minute before the series' first served day bar** is another
//!   security's (WP-13d's `before_first_day_bar`, CMP-18): its day reads
//!   `store_without_source` on 1min and on every derived rung that holds a
//!   bar on it, and a derived rung or a 1day month holding none gets no
//!   record (it owes none), never `missing_in_store`.
//! * **No kept answer** (a pull from before IA-8, a landing killed between
//!   its landing and its keep, a window landed from another store) is
//!   `no_response_kept`, never `match` (IA-ZV-02, IA-ZV-10); reason
//!   `landed_from_store` when the day's ledger line is `why=repaired` with
//!   `origin=store` (IA-ZV-11).
//!
//! # Where the records go
//!
//! Into the run's spool ([`spool`], D-4881), never straight into the verdict
//! store, and from there into one generation per (segment, timeframe, day)
//! by [`merge_all`], which runs only in a `verify-merge` process under
//! `verify/merge.lock` (or at the end of `cli zerodha-verify`, its own
//! process), never in the api and never on its start path (D-4921).
//!
//! # Cost
//!
//! Verification is O(minutes × rungs) per day (L-IA-18); the cold sweep is
//! O(stored Zerodha bars) once per run, plus one read of the gap ledger at
//! its start (L-IA-20); a merge is O(records) plus one sort of a spool's
//! records, O(n log n), and one sort of the spools waiting per listing,
//! O(s log s) (L-IA-17; the two sorts gate 11's `ALLOW_SORT` grants this
//! file), on up to [`MAX_MERGE_WORKERS`] workers, one per (segment,
//! timeframe, month) group at a time (RA-050). C-ZV-01, C-ZV-02 and C-ZV-03
//! time them.
//!
//! # What this build does not check, said here
//!
//! * A contract series (an expired future): its answers are not looked up,
//!   so its minutes read `no_response_kept`. No Zerodha contract is held on
//!   the operator's store (measured 20 Oct 2026: `bars/zerodha/NSE` holds
//!   `CASH` and `INDEX` only).
//! * A renamed share is checked under each name apart. When the store holds
//!   BOTH names of a change (the alias table's lineage, D-3332, RA-302), an
//!   empty day the other name owes gets no record under this one; when it
//!   holds only one (Zerodha files a share's whole history under its newest
//!   name: ETERNAL from 2021-07, TMPV, and no ZOMATO or TATAMOTORS series on
//!   the operator's store, measured 20 Oct 2026), that name owes every day,
//!   so a real hole is never hidden by a rename. The sum of owed minutes
//!   across a whole lineage is not formed here; it is a page's.
//! * The `derive` line's `rules=` at `v=2`: `/dayflags.json` reads `v=1`
//!   lines only (`api/src/dayflags.rs`, `parse_derive`), so writing `v=2`
//!   before its reader takes it would make every new derive line a fault
//!   there. Rule set 1 is read from `v=1`.

#[path = "zerodha_spool.rs"]
pub mod spool;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Exchange, InstrumentKey, Kind, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use store::file::{BarFile, StoreError};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::verdict::{
    self, Carry, Code, DayBlock, Detail, Entry, Facts, Generation, Key, Mode, flags,
};

use crate::calendar::{self, DayKind};
use crate::fetch::{RawRow, RowCause};
use crate::gdfl_run::stop::StopControl;
use crate::ingest::repair_plan::{self, Owes};
use crate::response_keep;
use crate::session::{Day, IST_OFFSET_SECS};
use crate::vendor::TimestampEncoding;
use crate::verdict_words as words;
use spool::{LogRecord, LogState, SpoolError, SpoolId, SpoolRecord, SpoolWriter};

/// The verifier version a generation carries.
pub const VERIFIER: u16 = 1;
/// The events' target.
pub const TARGET: &str = "pull.zerodha_verify";
/// Rule set 1: a derived month with a `derive` line at `v=1`, D-3321 as of
/// WP-13b.
pub const RULES_D3321: u8 = 1;
/// The most generations a re-merge walks back to learn whether an
/// interrupted commit published (one per commit since the intent; under
/// `verify/merge.lock` there is at most one).
pub const MAX_WALK_BACK: u32 = 64;

/// Microseconds in a minute.
const MINUTE_US: i64 = 60_000_000;
/// Seconds in a day.
const DAY_S: i64 = 86_400;
/// 09:15 IST, minutes past midnight: the grid's anchor (D-1447).
const OPEN_MINUTE: i64 = 555;

// ------------------------------------------------------------------ series --

/// One Zerodha series: an instrument's store identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Series {
    /// The exchange.
    pub exchange: Exchange,
    /// The segment.
    pub segment: Segment,
    /// The symbol, as the store's directory names it.
    pub symbol: String,
    /// The contract, for a future or an option.
    pub contract: Option<Contract>,
}

impl Series {
    /// The key the gap ledger and the verdict records name:
    /// `<EXCH>-<SEG>-<SYMBOL>[-<contract>]`.
    #[must_use]
    pub fn key_text(&self) -> String {
        match &self.contract {
            Some(contract) => format!(
                "{}-{}-{}-{}",
                self.exchange.as_str(),
                self.segment.as_str(),
                self.symbol,
                contract.as_str()
            ),
            None => format!(
                "{}-{}-{}",
                self.exchange.as_str(),
                self.segment.as_str(),
                self.symbol
            ),
        }
    }

    /// Whether this is an option, which is never derived.
    #[must_use]
    pub fn is_option(&self) -> bool {
        self.contract.as_ref().is_some_and(Contract::is_option)
    }

    /// The store parts of one month at one rung.
    #[must_use]
    pub fn parts(&self, timeframe: Timeframe, month: YearMonth) -> PathParts<'_> {
        PathParts {
            vendor: Vendor::Zerodha,
            exchange: self.exchange.as_str(),
            segment: self.segment.as_str(),
            symbol: &self.symbol,
            contract: self.contract,
            timeframe,
            month,
            file: FileKind::Bars,
        }
    }

    /// The instrument key a kept answer's pack is filed under; `None` for a
    /// contract (see the module's limits).
    fn instrument(&self) -> Option<InstrumentKey> {
        let kind = match (self.segment, &self.contract) {
            (Segment::Index, None) => Kind::Index,
            (Segment::Cash, None) => Kind::Equity,
            _ => return None,
        };
        Some(InstrumentKey {
            exchange: self.exchange,
            segment: self.segment,
            underlying: Symbol::new(&self.symbol).ok()?,
            kind,
        })
    }
}

/// The derived rungs: every stored multiple of a minute above it, but 1day
/// (this module's own list, read off `Timeframe::KNOWN`, D-4858).
#[must_use]
pub fn derived_rungs() -> Vec<Timeframe> {
    Timeframe::KNOWN
        .iter()
        .copied()
        .filter(|tf| tf.secs() > 60 && tf.secs().is_multiple_of(60) && *tf != Timeframe::DAY_1)
        .collect()
}

// ------------------------------------------------------------- the ledger --

/// One day's gap-ledger line, as the verifier reads it (last line of a key
/// and day wins, D-3317).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GapLine {
    /// The `why=` word's reason number.
    pub why: u16,
    /// `origin=store`: a `cli zerodha-repair --from-store` landing.
    pub origin_store: bool,
}

/// The gap ledger read once per run (O(lines), L-IA-20): one map probe per
/// (key, day) after that. A run that keeps it ([`GapLedger::refresh`]) reads
/// only the lines appended since, so a window's own landing lines are seen
/// at O(new bytes), never by reading the ledger again.
#[derive(Debug, Clone, Default)]
pub struct GapLedger {
    gaps: HashMap<(String, u32), GapLine>,
    derive: HashMap<(String, u32, u32), u8>,
    covered: HashSet<(String, u32)>,
    /// Lines that are neither a well-formed `gap` nor `derive` line, by line
    /// number: named, never silently skipped.
    pub faults: Vec<u64>,
    /// Whole lines read.
    pub lines: u64,
    /// Bytes read, to the end of the last whole line: the next refresh
    /// starts here. A last line with no line break (a writer mid-append) is
    /// read for what it says and read again, whole, by the next refresh.
    read_to: u64,
}

/// A month as one number: `year × 12 + month`.
fn month_number(month: YearMonth) -> u32 {
    u32::from(month.year()) * 12 + u32::from(month.month())
}

impl GapLedger {
    /// Reads `<root>/audit/zerodha.gaps`; an absent ledger holds nothing.
    ///
    /// # Errors
    ///
    /// The host's words for a ledger that exists and cannot be read, or
    /// something other than a regular file at its name.
    pub fn read(root: &Path) -> Result<Self, String> {
        let mut out = Self::default();
        out.refresh(root)?;
        Ok(out)
    }

    /// Reads the lines appended since the last read: O(new bytes). A ledger
    /// that shrank below what was read is read again whole and its faults
    /// start again.
    ///
    /// # Errors
    ///
    /// As [`GapLedger::read`].
    pub fn refresh(&mut self, root: &Path) -> Result<(), String> {
        use std::os::unix::fs::FileExt as _;
        let path = crate::ingest::gap_backfill::ledger_path(root, Vendor::Zerodha);
        let (file, len) = match store::safe_open::open_read_len(&path) {
            Ok(found) => found,
            Err(why) if why.is_missing() => return Ok(()),
            Err(why) => return Err(format!("{}: {why}", path.display())),
        };
        if len < self.read_to {
            *self = Self::default();
        }
        let size = usize::try_from(len - self.read_to)
            .map_err(|_| format!("{}: {len} bytes do not fit this host", path.display()))?;
        let mut bytes = vec![0_u8; size];
        file.read_exact_at(&mut bytes, self.read_to)
            .map_err(|why| format!("{}: {why}", path.display()))?;
        self.take(&bytes);
        Ok(())
    }

    /// The ledger `bytes` hold.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        let mut out = Self {
            gaps: HashMap::with_capacity(bytes.len() / 160 + 1),
            derive: HashMap::with_capacity(bytes.len() / 160 + 1),
            covered: HashSet::with_capacity(bytes.len() / 4_000 + 1),
            ..Self::default()
        };
        out.take(bytes);
        out
    }

    /// Reads `bytes`, which start where the last read stopped.
    fn take(&mut self, bytes: &[u8]) {
        for piece in bytes.split_inclusive(|b| *b == b'\n') {
            let whole = piece.last() == Some(&b'\n');
            let line = if whole {
                piece.get(..piece.len() - 1).unwrap_or_default()
            } else {
                piece
            };
            let number = self.lines + 1;
            if whole {
                self.lines = number;
                self.read_to += piece.len() as u64;
            }
            if line.is_empty() {
                continue;
            }
            let parsed = match std::str::from_utf8(line) {
                Ok(text) if text.starts_with("gap ") => self.gap_line(text),
                Ok(text) if text.starts_with("derive ") => self.derive_line(text),
                Ok(_) => Some(()),
                Err(_) => None,
            };
            if parsed.is_none() && whole {
                self.faults.push(number);
            }
        }
    }

    fn gap_line(&mut self, text: &str) -> Option<()> {
        let mut fields = text.split(' ').skip(1);
        let key = fields
            .next()
            .filter(|k| !k.is_empty() && !k.contains('='))?;
        let day = parse_day(fields.next()?)?;
        let mut why = None;
        let mut origin_store = false;
        for field in fields {
            match field.split_once('=') {
                Some(("why", value)) => why = words::reason(value),
                Some(("origin", "store")) => origin_store = true,
                _ => {}
            }
        }
        let why = why?;
        self.covered
            .insert((key.to_owned(), month_number(day.year_month().ok()?)));
        self.gaps.insert(
            (key.to_owned(), day.days_from_epoch()),
            GapLine { why, origin_store },
        );
        Some(())
    }

    fn derive_line(&mut self, text: &str) -> Option<()> {
        let mut fields = text.split(' ').skip(1);
        let key = fields
            .next()
            .filter(|k| !k.is_empty() && !k.contains('='))?;
        let rung = fields.next()?;
        let month = month_number(parse_month(fields.next()?)?);
        let version = fields
            .find_map(|w| w.strip_prefix("v="))
            .and_then(|v| v.parse::<u8>().ok())?;
        let rung = Timeframe::KNOWN
            .iter()
            .copied()
            .find(|tf| tf.as_str() == rung)?;
        self.covered.insert((key.to_owned(), month));
        self.derive
            .insert((key.to_owned(), rung.secs(), month), version);
        Some(())
    }

    /// The day's line, last line winning.
    #[must_use]
    pub fn gap(&self, key: &str, day: Day) -> Option<GapLine> {
        self.gaps
            .get(&(key.to_owned(), day.days_from_epoch()))
            .copied()
    }

    /// Whether the ledger holds any line of `key` in `month`: a month landed
    /// since the ledger exists, whose days without a line had no gap.
    #[must_use]
    pub fn covers(&self, key: &str, month: YearMonth) -> bool {
        // ONE HASH PROBE, not a linear scan (gate 11 rule 7 is textual).
        let probe = (key.to_owned(), month_number(month));
        let probe = &probe;
        self.covered.contains(probe)
    }

    /// The `derive` line's version of `key` at `rung` in `month`.
    #[must_use]
    pub fn derive_version(&self, key: &str, rung: Timeframe, month: YearMonth) -> Option<u8> {
        self.derive
            .get(&(key.to_owned(), rung.secs(), month_number(month)))
            .copied()
    }
}

/// `YYYY-MM-DD`.
fn parse_day(word: &str) -> Option<Day> {
    let (month, day) = word.rsplit_once('-')?;
    let month = parse_month(month)?;
    if day.len() != 2 {
        return None;
    }
    Day::new(month.year(), month.month(), day.parse().ok()?).ok()
}

/// `YYYY-MM`.
fn parse_month(word: &str) -> Option<YearMonth> {
    let (year, month) = word.split_once('-')?;
    if year.len() != 4 || month.len() != 2 {
        return None;
    }
    YearMonth::new(year.parse().ok()?, month.parse().ok()?).ok()
}

// ------------------------------------------------------------- the oracle --

/// One answer of one day, as the landing oracle reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DayAnswer {
    /// The receipt, as the run numbers them (1 for the first).
    pub receipt: u32,
    /// The window asked, unix seconds UTC, both ends inclusive.
    pub asked: (i64, i64),
    /// Whether the body was kept and decoded; when not, `not_kept` says why.
    pub kept: bool,
    /// The reason number the body was not kept for.
    pub not_kept: u16,
    /// The last decoded row's stamp (unix seconds UTC; 0 = none).
    pub last_stamp: i64,
    /// The bars the landing should have landed, by minute of the IST day
    /// (or, at 1day, at minute 0).
    pub bars: HashMap<u32, Bar>,
    /// The minutes refused by name, with the reason number of each word.
    pub refused: Vec<(u32, u16)>,
    /// Identical duplicate rows counted once.
    pub dup: u32,
    /// The rows the landing refused by name against the oracle's count of
    /// them, when they differ: (landing, oracle).
    pub quality: Option<(u64, u64)>,
    /// The rows the landing decoded from this answer (its receipt's
    /// `rows_decoded`) against the rows the oracle's own decode of the kept
    /// body gave, when they differ: (landing, oracle). The decoder drops a
    /// candle whose four prices cannot be a bar (DB-G46) before either side
    /// sees it, so the two agree exactly when both skipped the same candles
    /// by the same rule (§8 Z6).
    pub decoded: Option<(u64, u64)>,
    /// The day's session length is unmeasured: its rows were withheld.
    pub withheld: bool,
    /// The answer's fields the documented shape does not name (RA-318).
    pub shape: Shape,
}

/// What a kept answer carries beyond the shape this build reads (RA-318).
///
/// # Why the checker looks with eyes of its own
///
/// The oracle decodes a kept body with `http::decode_body`, the pull's own
/// decoder, so the two agree on every row by construction, and a vendor that
/// changed its answer in a way that still decodes would be invisible to both:
/// a field the decoder never reads is dropped by both sides alike. The shape
/// the charter records for Kite's historical answer (`docs/00-charter.md`
/// §4z, "Response": `{status, data:{candles:[[ts,o,h,l,c,volume(,oi)]]}}`) is
/// the yardstick: a key at the root other than `status` and `data`, a key in
/// `data` other than `candles`, and a `status` that is absent or not text are
/// each counted. The decoder already refuses a candle of the wrong arity and a
/// price that is not a number, so those never reach this count.
///
/// The status WORD is not compared: no source this repository records names
/// the word a successful answer carries (UNVERIFIED), and a guessed word would
/// be the invention `CLAUDE.md` §3 rule 1 forbids.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shape {
    /// Fields outside the documented shape, counted.
    pub unknown: u32,
    /// The first of them by its path (`meta`, `data.next`, `status`), cut to
    /// [`SHAPE_NAME_BYTES`] at a character boundary; empty when none.
    pub first: String,
}

/// The most bytes of a field's path a [`Shape`] keeps.
pub const SHAPE_NAME_BYTES: usize = 48;

/// The alert kind of a changed answer shape (`alerts::KINDS`, 21).
pub const VENDOR_SHAPE_CHANGED: u8 = 21;

/// Counts the fields of one kept answer that the documented shape does not
/// name (see [`Shape`]). One parse of the body, O(its bytes), the cost the
/// decode beside it already pays; a body that is not a JSON object counts
/// nothing here, because the decoder refuses it by name first.
#[must_use]
pub fn answer_shape(body: &str) -> Shape {
    let mut shape = Shape::default();
    let Ok(serde_json::Value::Object(root)) = serde_json::from_str::<serde_json::Value>(body)
    else {
        return shape;
    };
    let note = |path: String, shape: &mut Shape| {
        shape.unknown = shape.unknown.saturating_add(1);
        if shape.first.is_empty() {
            let mut end = path.len().min(SHAPE_NAME_BYTES);
            while !path.is_char_boundary(end) {
                end -= 1;
            }
            path[..end].clone_into(&mut shape.first);
        }
    };
    match root.get("status") {
        Some(serde_json::Value::String(_)) => {}
        Some(_) | None => note("status".to_owned(), &mut shape),
    }
    for key in root.keys() {
        if key != "status" && key != "data" {
            note(key.clone(), &mut shape);
        }
    }
    if let Some(serde_json::Value::Object(data)) = root.get("data") {
        for key in data.keys() {
            if key != "candles" {
                note(format!("data.{key}"), &mut shape);
            }
        }
    }
    shape
}

/// The detail a changed shape adds to a record: the count (u32) then the
/// first field's path.
fn note_shape(answer: &DayAnswer, details: &mut Vec<Detail>) {
    if answer.shape.unknown == 0 {
        return;
    }
    details.push(Detail {
        kind: words::DETAIL_VENDOR_SHAPE,
        payload: payload(&[
            &answer.shape.unknown.to_le_bytes(),
            answer.shape.first.as_bytes(),
        ]),
    });
}

/// Puts the one alert of a changed shape: keyed by the rung and the first
/// unnamed field, so one change seen in every instrument's answers is one
/// alert until it is acknowledged (`alerts::put` writes nothing for a key
/// already unacknowledged). A put that fails is a fault, never dropped.
fn shape_alert(
    root: &Path,
    series: &Series,
    rung: Timeframe,
    month: YearMonth,
    shape: &Shape,
) -> Option<String> {
    let key = format!("vendor_shape_changed:{}:{}", rung.as_str(), shape.first);
    let text = format!(
        "{} {month} {}: the kept answer carries {} field(s) the documented shape does not name, first {:?}",
        series.key_text(),
        rung.as_str(),
        shape.unknown,
        shape.first
    );
    let _ = telemetry::emit(
        &telemetry::Event::warn(TARGET, "vendor shape changed")
            .with("key", telemetry::Value::Str(&key))
            .with("why", telemetry::Value::Str(&text)),
    );
    crate::alerts::put(
        root,
        &crate::alerts::NewAlert {
            kind: VENDOR_SHAPE_CHANGED,
            key: &key,
            day: 0,
            run: 0,
            text: &text,
        },
    )
    .err()
    .map(|failed| format!("{key}: the alert was not written: {failed}"))
}

/// UTC milliseconds of a vendor stamp in `encoding`.
pub(crate) fn utc_millis(timestamp: i64, encoding: TimestampEncoding) -> i128 {
    let offset = i128::from(IST_OFFSET_SECS) * 1_000;
    match encoding {
        TimestampEncoding::EpochMillisUtc => i128::from(timestamp),
        TimestampEncoding::EpochSecondsUtc | TimestampEncoding::IsoDateTimeOffset => {
            i128::from(timestamp) * 1_000
        }
        TimestampEncoding::IstDateTimeText | TimestampEncoding::IsoDateTimeText => {
            i128::from(timestamp) * 1_000 - offset
        }
    }
}

/// The IST day number and minute of a UTC second.
const fn ist_day_minute(utc_s: i64) -> (i64, i64) {
    let local = utc_s + IST_OFFSET_SECS;
    (local.div_euclid(DAY_S), local.rem_euclid(DAY_S) / 60)
}

/// Where minutes may land on a day: what the landing's session filter keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Permit {
    /// The exchange did not trade: rows are dropped.
    Closed,
    /// No calendar of record attests the day: rows are refused
    /// `timestamp_refused`.
    Unattested,
    /// A session of unmeasured length: rows are withheld (D-3344).
    Unsized,
    /// These `[from, to)` minute spans.
    Windows(repair_plan::Windows),
}

/// What `day` permits a series of `segment`: the calendar's session, and on
/// a regular day the venue's hours under EVERY closing-auction eligibility
/// (the widest, where [`repair_plan::owed`] takes the narrowest), so a minute
/// some share may land is never called extra.
fn permit(segment: Segment, number: u32) -> Permit {
    let session = match calendar::exchange_kind_of(i64::from(number)) {
        DayKind::Closed => return Permit::Closed,
        DayKind::Unmeasured => return Permit::Unattested,
        DayKind::OpenLengthUnmeasured => return Permit::Unsized,
        DayKind::Open(session) => session,
    };
    if session != calendar::Session::full() {
        let mut windows = [(0, 0); calendar::MAX_WINDOWS];
        let count = usize::from(session.count).min(calendar::MAX_WINDOWS);
        for (slot, window) in windows.iter_mut().zip(session.windows).take(count) {
            *slot = (u32::from(window.from), u32::from(window.to) + 1);
        }
        return Permit::Windows((windows, count));
    }
    let Ok(day) = Day::from_days(number) else {
        return Permit::Unattested;
    };
    let Some(venue) = crate::vendor::Venue::for_segment(Exchange::Nse, segment) else {
        return Permit::Unsized;
    };
    let mut span: Option<(u32, u32)> = None;
    for cas in [
        calendar::CasEligibility::Eligible,
        calendar::CasEligibility::NotEligible,
    ] {
        let Ok(hours) = venue.share_hours_on(day, cas) else {
            return Permit::Unsized;
        };
        let (open, close) = (hours.open_minute(), hours.close_minute());
        span = Some(span.map_or((open, close), |(from, to)| (from.min(open), to.max(close))));
    }
    match span {
        Some(span) if span.0 < span.1 => Permit::Windows(([span, (0, 0)], 1)),
        _ => Permit::Unsized,
    }
}

/// Whether `minute` lies in one of `windows`.
fn in_windows((windows, count): &repair_plan::Windows, minute: u32) -> bool {
    windows
        .iter()
        .take(*count)
        .any(|(from, to)| *from <= minute && minute < *to)
}

/// What one decoded answer should have landed, per IST day (D-3351 and the
/// calendar), before the landing's own refused rows are taken in.
#[derive(Debug, Clone, Default)]
pub struct Oracle {
    /// One [`DayAnswer`] per IST day number the answer touched.
    pub days: HashMap<u32, DayAnswer>,
    /// Rows refused by the words the oracle judges itself, all days.
    pub judged: u64,
}

/// The bar a decoded row lands as, at `utc_s`: THE SCALE ONLY.
/// `decode_body` already snapped every price; a second rounding site would be
/// a second answer (VERIFY-07). `None` for a price the scale cannot carry.
fn scaled(row: &RawRow, utc_s: i64) -> Option<Bar> {
    let price = |raw: i64| crate::fetch::to_paisa(raw, crate::http::DECODED_PRICE_SCALE);
    Some(Bar {
        ts_micros: utc_s.saturating_mul(1_000_000),
        open: price(row.open)?,
        high: price(row.high)?,
        low: price(row.low)?,
        close: price(row.close)?,
        volume: row.volume,
        open_interest: row.open_interest.unwrap_or(OI_NULL),
    })
}

/// Applies the landing oracle to `rows` of one answer (D-4857): the grid,
/// D-3351's duplicate and conflict rule, the scale, the asked window and the
/// calendar. O(rows), one hash probe per row.
#[must_use]
pub fn oracle(
    rows: &[RawRow],
    encoding: TimestampEncoding,
    rung: Timeframe,
    segment: Segment,
    asked: (i64, i64),
) -> Oracle {
    let daily = rung == Timeframe::DAY_1;
    let width_ms = i128::from(rung.secs()) * 1_000;
    let on_grid = |ms: i128| {
        daily
            || (ms + i128::from(IST_OFFSET_SECS) * 1_000 - i128::from(OPEN_MINUTE) * 60_000)
                .rem_euclid(width_ms)
                == 0
    };
    // FIRST PASS: what each stamp has been seen as (D-3351).
    let mut seen: HashMap<i64, (usize, bool)> = HashMap::with_capacity(rows.len());
    for (at, row) in rows.iter().enumerate() {
        if !on_grid(utc_millis(row.timestamp, encoding)) {
            continue;
        }
        match seen.get_mut(&row.timestamp) {
            None => {
                seen.insert(row.timestamp, (at, false));
            }
            Some((first, conflicting)) => {
                if rows.get(*first) != Some(row) {
                    *conflicting = true;
                }
            }
        }
    }
    let mut out = Oracle {
        days: HashMap::with_capacity(8),
        judged: 0,
    };
    for (at, row) in rows.iter().enumerate() {
        let ms = utc_millis(row.timestamp, encoding);
        let utc_s = i64::try_from(ms.div_euclid(1_000)).unwrap_or(i64::MAX);
        let (day, minute) = ist_day_minute(utc_s);
        let Ok(number) = u32::try_from(day) else {
            continue;
        };
        let minute = u32::try_from(minute).unwrap_or(0);
        let refuse = |out: &mut Oracle, word: &str| {
            out.judged += 1;
            let entry = out.days.entry(number).or_default();
            entry.refused.push((minute, reason_of(word)));
        };
        if !on_grid(ms) {
            refuse(&mut out, RowCause::OffGrid.code());
            continue;
        }
        match seen.get(&row.timestamp).copied() {
            Some((_, true)) => {
                refuse(&mut out, RowCause::Conflicting.code());
                continue;
            }
            Some((first, false)) if first != at => {
                out.days.entry(number).or_default().dup += 1;
                continue;
            }
            _ => {}
        }
        // THE ASKED WINDOW AND THE CALENDAR: a row outside either is dropped
        // by the landing's filter, never refused (a day of unmeasured length
        // is withheld, D-3344; a day no calendar attests is refused).
        if utc_s < asked.0 || utc_s > asked.1 {
            continue;
        }
        if !daily {
            match permit(segment, number) {
                Permit::Closed => continue,
                Permit::Unattested => {
                    refuse(&mut out, RowCause::Timestamp.code());
                    continue;
                }
                Permit::Unsized => {
                    out.days.entry(number).or_default().withheld = true;
                    continue;
                }
                Permit::Windows(windows) => {
                    if !in_windows(&windows, minute) {
                        continue;
                    }
                }
            }
        }
        let Some(bar) = scaled(row, utc_s) else {
            refuse(&mut out, RowCause::Price.code());
            continue;
        };
        let slot = if daily { 0 } else { minute };
        out.days.entry(number).or_default().bars.insert(slot, bar);
    }
    out
}

impl Oracle {
    /// Takes in the landing's own refused rows (`.rsx`, DB-G46): a word the
    /// oracle cannot judge from the answer removes its minute from the bars
    /// and is counted; the words it judges are compared by count, and the
    /// total with the receipt's "rows the landing refused".
    pub fn take_landing_refusals(
        &mut self,
        rows: &[response_keep::RefusedRow],
        receipt_refused: u32,
    ) {
        let mut landing_judged = 0_u64;
        let mut taken = 0_u64;
        for row in rows {
            let (day, minute) = (
                row.local_second.div_euclid(DAY_S),
                row.local_second.rem_euclid(DAY_S) / 60,
            );
            let (Ok(number), Ok(minute)) = (u32::try_from(day), u32::try_from(minute)) else {
                continue;
            };
            match row.cause {
                RowCause::OffGrid | RowCause::Conflicting => landing_judged += 1,
                other => {
                    taken += 1;
                    let entry = self.days.entry(number).or_default();
                    entry.bars.remove(&minute);
                    entry.refused.push((minute, reason_of(other.code())));
                }
            }
        }
        let oracle_total = self.judged + taken;
        if landing_judged != self.judged || u64::from(receipt_refused) != oracle_total {
            for answer in self.days.values_mut() {
                answer.quality = Some((u64::from(receipt_refused), oracle_total));
            }
        }
    }
}

// --------------------------------------------------------- one day's check --

/// One derived rung of a day: the rung, its stored bars of the day (`None`
/// where the rung's month is not held) and its month's `derive` line version.
pub type DerivedDay<'a> = (Timeframe, Option<&'a [Bar]>, Option<u8>);

/// A spool record's sort key, and its place in the spool.
type Keyed<'a> = ((u8, Day, &'a str, u32), usize);

/// What one day of one series holds and was answered, for [`verify_day`].
#[derive(Debug, Clone, Default)]
pub struct DayInputs<'a> {
    /// The day's stored 1min bars, in stored order.
    pub minutes: &'a [Bar],
    /// The 1min month's first row of the day.
    pub first_row: u32,
    /// The 1min month's served generation.
    pub generation: u32,
    /// The 1min month refused to open: its words.
    pub minutes_refused: Option<&'a str>,
    /// Each derived rung's stored bars of the day (`None` where the rung's
    /// month is not held) and its month's `derive` line version.
    pub derived: Vec<DerivedDay<'a>>,
    /// The stored 1day bar.
    pub day_bar: Option<Bar>,
    /// Whether the 1day month is held at all.
    pub day_month_held: bool,
    /// The answers covering the day at 1min, in receipt order.
    pub answers: &'a [DayAnswer],
    /// The answers covering the day at 1day, in receipt order.
    pub day_answers: &'a [DayAnswer],
    /// The day's gap-ledger line.
    pub gap: Option<GapLine>,
    /// Whether the ledger covers the day's month for this key.
    pub covered: bool,
    /// GDFL's one-second bars of the same instrument, as the minutes that
    /// traded; `None` when GDFL holds no such month.
    pub peer: Option<&'a HashSet<u32>>,
    /// The series' first served day bar's day.
    pub first_day_bar: Option<Day>,
    /// Each rung whose month is held but would not open for this read, with
    /// whether a writer held it past [`MONTH_BUSY_RETRY`]: that rung's record
    /// is `not_compared (month_busy)` when it did, `store_unreadable
    /// (month_refused)` for any other refusal, and is never read as a month
    /// not held (design §6.3, "A month a writer holds"). The 1min rung's
    /// words are `minutes_refused`. `None`: every month opened.
    pub refused: Option<&'a HashMap<Timeframe, bool>>,
    /// The day lies outside the series' held 1min months (IA-9 repair 1: a
    /// series held at 1day only, a 1day month before the first 1min month):
    /// no 1min record, no record for a derived rung whose month is not held,
    /// and the 1day record names `daycheck` not run. `false`: the 1min rung is
    /// owed and verified.
    pub beyond_minutes: bool,
}

/// One record of one rung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RungRecord {
    /// The rung.
    pub timeframe: Timeframe,
    /// The record.
    pub record: SpoolRecord,
}

/// The minute of the IST day a bar opens at.
fn minute_of(bar: &Bar) -> u32 {
    let local = bar.ts_micros.div_euclid(1_000_000) + IST_OFFSET_SECS;
    u32::try_from(local.rem_euclid(DAY_S) / 60).unwrap_or(0)
}

/// Every check of one series' day (§6.3 steps 2 to 6), one record per rung.
/// O(minutes × rungs).
#[must_use]
pub fn verify_day(series: &Series, day: Day, inputs: &DayInputs<'_>) -> Vec<RungRecord> {
    let key_text = series.key_text();
    let Ok(key) = Key::instrument(&key_text) else {
        return Vec::new();
    };
    let make =
        |timeframe: Timeframe, facts: Facts, receipt: u32, details: Vec<Detail>| RungRecord {
            timeframe,
            record: SpoolRecord {
                exchange: series.exchange,
                day,
                key: key.clone(),
                facts,
                receipt,
                details,
            },
        };
    let mut out = Vec::with_capacity(9);
    // OUTSIDE THE SERIES' 1MIN MONTHS THE 1MIN RUNG IS NOT OWED (IA-9 repair
    // 1): no record says a minute is missing that the store never held.
    if !inputs.beyond_minutes {
        let (facts, receipt, details) = minute_facts(series, day, inputs);
        out.push(make(Timeframe::MINUTE_1, facts, receipt, details));
    }
    let foreign = !inputs.minutes.is_empty() && foreign_day(day, inputs);
    if !series.is_option() {
        for (rung, bars, version) in &inputs.derived {
            // A RUNG WHOSE MONTH WOULD NOT OPEN IS SAID, never read as a
            // month not held: that would be `missing_in_store
            // (rung_day_absent)` for every day of a month a pull was
            // appending (design §6.3, "A month a writer holds").
            if let Some(busy) = rung_refusal(inputs, *rung) {
                out.push(make(*rung, month_refused(busy), 0, Vec::new()));
                continue;
            }
            // A DERIVED RUNG OF ANOTHER SECURITY'S DAY (CMP-18): holding no
            // bar on it, it owes none, so no record, never `missing_in_store
            // (rung_day_absent)`, which would ask a repair to fold another
            // security's minutes into this series; holding bars, they are the
            // same foreign minutes refolded.
            if foreign && bars.is_none_or(<[Bar]>::is_empty) {
                continue;
            }
            // A DERIVED MONTH NOT HELD OUTSIDE THE 1MIN MONTHS owes nothing:
            // there is no source to fold it from.
            if inputs.beyond_minutes && bars.is_none() {
                continue;
            }
            let (mut facts, details) =
                derived_facts(series, day, inputs.minutes, *rung, *bars, *version);
            if foreign {
                not_this_series(&mut facts);
            }
            out.push(make(*rung, facts, 0, details));
        }
    }
    // ANOTHER SECURITY'S DAY (CMP-18) HOLDS NO DAY BAR by the rule's own
    // definition (the first served day bar is later): unless an answer for
    // the day was kept, its 1day rung owes none and gets no record, never
    // `missing_in_store (unexplained)` for a bar the series never had.
    let owes_no_day_bar = foreign && inputs.day_bar.is_none() && inputs.day_answers.is_empty();
    if let Some(busy) = rung_refusal(inputs, Timeframe::DAY_1) {
        out.push(make(Timeframe::DAY_1, month_refused(busy), 0, Vec::new()));
    } else if (inputs.day_month_held || inputs.day_bar.is_some() || !inputs.day_answers.is_empty())
        && !owes_no_day_bar
    {
        let (mut facts, receipt, details) = day_facts(inputs);
        if foreign {
            not_this_series(&mut facts);
        }
        out.push(make(Timeframe::DAY_1, facts, receipt, details));
    }
    out
}

/// Whether `rung`'s month is held but would not open for this read, and if
/// so whether a writer held it (`Some(true)`). One hash probe.
fn rung_refusal(inputs: &DayInputs<'_>, rung: Timeframe) -> Option<bool> {
    inputs
        .refused
        .and_then(|refused| refused.get(&rung).copied())
}

/// Whether `day` is before the series' first served day bar: a day the
/// series was not yet listed on, whose stored bars are another security's
/// (WP-13d's `before_first_day_bar`, CMP-18).
fn foreign_day(day: Day, inputs: &DayInputs<'_>) -> bool {
    inputs.first_day_bar.is_some_and(|first| day < first)
}

/// A record of another security's day (CMP-18): the series owes nothing on
/// it, so a minute or bucket it lacks is not `missing_in_store` and one it
/// holds is not this series' `extra`; the record reads `store_without_source
/// (before_first_day_bar)`, its counts kept as they were measured. A stored
/// day that will not read, or a stored bar that disagrees with its own
/// source, is a defect of the bytes whoever's they are, and keeps its code.
fn not_this_series(facts: &mut Facts) {
    if !matches!(facts.code, Code::StoreUnreadable | Code::ValueMismatch) {
        facts.code = Code::StoreWithoutSource;
        facts.reason = reason_of(RowCause::BeforeFirstDayBar.code());
    }
}

/// A reason's number, or 0.
fn reason_of(word: &str) -> u16 {
    words::reason(word).unwrap_or(0)
}

/// Bytes of a detail payload.
fn payload(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(parts.iter().map(|p| p.len()).sum());
    for part in parts {
        out.extend_from_slice(part);
    }
    out
}

/// Whether two bars carry the same prices.
const fn same_prices(a: &Bar, b: &Bar) -> bool {
    a.open == b.open && a.high == b.high && a.low == b.low && a.close == b.close
}

/// The worse of `code` and `candidate` by severity, with its reason.
fn worsen(candidate: Code, why: u16, code: &mut Code, reason: &mut u16) {
    if candidate.severity() < code.severity() {
        *code = candidate;
        *reason = why;
    }
}

/// `minute` as the first difference, when it is earlier than the last one.
fn note_diff(minute: u32, first_diff: &mut u32) {
    if minute < *first_diff {
        *first_diff = minute;
    }
}

/// The 1min record: the answer compare, the completeness classification
/// and the identity check, folded into one code by severity.
#[expect(
    clippy::too_many_lines,
    reason = "one day's minute record: the answer compare, the re-pulls, completeness, \
              the peer and the identity check each add one field to the same facts"
)]
fn minute_facts(series: &Series, day: Day, inputs: &DayInputs<'_>) -> (Facts, u32, Vec<Detail>) {
    let mut facts = Facts::of(Code::Match);
    let mut details = Vec::new();
    facts.month_generation = inputs.generation;
    if inputs.minutes_refused.is_some() {
        let refused = month_refused(rung_refusal(inputs, Timeframe::MINUTE_1).unwrap_or(false));
        facts.code = refused.code;
        facts.reason = refused.reason;
        return (facts, 0, details);
    }
    // THE STORED DAY: its minutes, and whether its stamps increase.
    let mut stored: HashMap<u32, &Bar> = HashMap::with_capacity(inputs.minutes.len());
    let mut last_ts = i64::MIN;
    for bar in inputs.minutes {
        if bar.ts_micros <= last_ts {
            facts.code = Code::StoreUnreadable;
            facts.reason = reason_of(if bar.ts_micros == last_ts {
                "duplicate"
            } else {
                "out_of_order"
            });
            facts.rows = u32::try_from(inputs.minutes.len()).unwrap_or(u32::MAX);
            return (facts, 0, details);
        }
        last_ts = bar.ts_micros;
        stored.insert(minute_of(bar), bar);
    }
    facts.store_units = u32::try_from(stored.len()).unwrap_or(u32::MAX);
    facts.rows = u32::try_from(inputs.minutes.len()).unwrap_or(u32::MAX);
    if let (Some(first), Some(last)) = (inputs.minutes.first(), inputs.minutes.last()) {
        facts.first_row = inputs.first_row;
        facts.first_unit = minute_of(first);
        facts.last_unit = minute_of(last);
    }
    let mut code = Code::Match;
    let mut reason = 0_u16;
    let mut first_diff = u32::MAX;
    // THE KEPT ANSWER, OR NONE (IA-ZV-02).
    let kept: Vec<&DayAnswer> = inputs.answers.iter().filter(|a| a.kept).collect();
    let mut receipt = 0_u32;
    let answered: Option<&DayAnswer> = kept.first().copied();
    if let Some(answer) = answered {
        receipt = answer.receipt;
        facts.source_units = u32::try_from(answer.bars.len()).unwrap_or(u32::MAX);
        for (minute, bar) in &answer.bars {
            if let Some(held) = stored.get(minute) {
                let mut differs = false;
                if !same_prices(held, bar) {
                    facts.value_diff += 1;
                    differs = true;
                }
                if held.volume != bar.volume {
                    facts.volume_diff += 1;
                    differs = true;
                }
                if held.open_interest != bar.open_interest {
                    facts.oi_diff += 1;
                    differs = true;
                }
                if differs {
                    note_diff(*minute, &mut first_diff);
                }
            } else {
                facts.missing += 1;
                note_diff(*minute, &mut first_diff);
            }
        }
        let (from, to) = answer.asked;
        // ONE HASH PROBE PER STORED MINUTE, never a walk of the refusals: an
        // answer whose every row was refused would make that O(minutes²).
        let refused_at = refused_minutes(answer);
        for (minute, bar) in &stored {
            let second = bar.ts_micros.div_euclid(1_000_000);
            if second >= from
                && second <= to
                && !answer.bars.contains_key(minute)
                && !refused_at.contains(minute)
            {
                facts.extra += 1;
                note_diff(*minute, &mut first_diff);
            }
        }
        if facts.value_diff + facts.volume_diff + facts.oi_diff > 0 {
            worsen(Code::ValueMismatch, 0, &mut code, &mut reason);
        }
        if facts.extra > 0 {
            worsen(
                Code::ExtraInStore,
                reason_of("unexplained"),
                &mut code,
                &mut reason,
            );
        }
        if facts.missing > 0 {
            let (class, why) = ledger_class(inputs, true);
            worsen(class, why, &mut code, &mut reason);
        }
        if let Some((_, word)) = answer.refused.first() {
            worsen(Code::SourceRefused, *word, &mut code, &mut reason);
            let mut counts: Vec<(u16, u32)> = Vec::with_capacity(4);
            for (_, word) in &answer.refused {
                match counts.iter_mut().find(|(w, _)| w == word) {
                    Some(found) => found.1 += 1,
                    None => counts.push((*word, 1)),
                }
            }
            let mut bytes = Vec::with_capacity(counts.len() * 6 + 4);
            for (word, count) in counts.iter().take(16) {
                bytes.extend_from_slice(&word.to_le_bytes());
                bytes.extend_from_slice(&count.to_le_bytes());
            }
            let first = answer.refused.iter().map(|(m, _)| *m).min().unwrap_or(0);
            bytes.extend_from_slice(&first.to_le_bytes());
            details.push(Detail {
                kind: words::DETAIL_LANDING_REFUSED,
                payload: bytes,
            });
        }
        if answer.dup > 0 {
            facts.flags |= flags::DUP_MINUTES;
            details.push(Detail {
                kind: words::DETAIL_DUP_MINUTES,
                payload: answer.dup.to_le_bytes().to_vec(),
            });
        }
        note_shape(answer, &mut details);
        if let Some((landing, oracle)) = answer.quality {
            facts.flags |= flags::QUALITY_DIFFERS;
            details.push(Detail {
                kind: words::DETAIL_QUALITY_DIFFERS,
                payload: payload(&[
                    &1_u16.to_le_bytes(),
                    &landing.to_le_bytes(),
                    &oracle.to_le_bytes(),
                ]),
            });
        }
        if let Some((landing, oracle)) = answer.decoded {
            // Sub-kind 2: rows decoded, the landing's against the oracle's.
            facts.flags |= flags::QUALITY_DIFFERS;
            details.push(Detail {
                kind: words::DETAIL_QUALITY_DIFFERS,
                payload: payload(&[
                    &2_u16.to_le_bytes(),
                    &landing.to_le_bytes(),
                    &oracle.to_le_bytes(),
                ]),
            });
        }
        // A STORED MINUTE NO KEPT ANSWER ASKED FOR (a landing killed between
        // its landing and its keep, IA-ZV-10): never vouched for, so the day
        // is never `match`.
        let unvouched = stored.values().any(|bar| {
            let second = bar.ts_micros.div_euclid(1_000_000);
            !kept
                .iter()
                .any(|a| second >= a.asked.0 && second <= a.asked.1)
        });
        if unvouched {
            worsen(Code::NoResponseKept, 0, &mut code, &mut reason);
        }
        if answer.withheld {
            worsen(
                Code::WithheldByRule,
                reason_of("withheld_unmeasured_session"),
                &mut code,
                &mut reason,
            );
        }
        // A RE-PULL THAT DIFFERS IS THE VENDOR'S REVISION, NEVER A STORE
        // DEFECT (VERIFY-13): the store kept the first.
        for later in kept.iter().skip(1) {
            let differs = later.bars.len() != answer.bars.len()
                || later
                    .bars
                    .iter()
                    .any(|(minute, bar)| answer.bars.get(minute) != Some(bar));
            if differs {
                facts.flags |= flags::VENDOR_REVISED;
                details.push(Detail {
                    kind: words::DETAIL_RECEIPTS,
                    payload: payload(&[
                        &answer.receipt.to_le_bytes(),
                        &later.receipt.to_le_bytes(),
                    ]),
                });
                break;
            }
        }
    } else {
        // NO KEPT ANSWER: never `match` (IA-ZV-02). A repaired landing from
        // another store says so (IA-ZV-11).
        let why = match inputs.gap {
            Some(line) if line.origin_store && line.why == reason_of("repaired") => {
                reason_of("landed_from_store")
            }
            _ => inputs.answers.first().map_or(0, |unkept| unkept.not_kept),
        };
        if !inputs.answers.is_empty() {
            facts.flags |= flags::BODY_NOT_KEPT;
        }
        code = Code::NoResponseKept;
        reason = why;
    }
    // COMPLETENESS AGAINST THE CALENDAR OF RECORD (D-4857).
    let number = day.days_from_epoch();
    match repair_plan::owed(series.segment.as_str(), number) {
        Owes::Closed => {
            if !stored.is_empty() {
                facts.extra = facts.extra.max(facts.store_units);
                worsen(
                    Code::ExtraInStore,
                    reason_of("unexplained"),
                    &mut code,
                    &mut reason,
                );
            }
        }
        Owes::Unattested => {
            let word = if i64::from(number) > calendar::SOURCED_LAST_DAY {
                "calendar_past_record"
            } else {
                "calendar_unverified"
            };
            if matches!(code, Code::Match | Code::NoResponseKept) {
                code = Code::NotCompared;
                reason = reason_of(word);
            }
        }
        Owes::Unsized => worsen(
            Code::WithheldByRule,
            reason_of("withheld_unmeasured_session"),
            &mut code,
            &mut reason,
        ),
        Owes::Windows(windows) => {
            let mut owed = 0_u32;
            let mut absent: Vec<u32> = Vec::new();
            for &(from, to) in windows.0.iter().take(windows.1) {
                for minute in from..to {
                    owed += 1;
                    if !stored.contains_key(&minute) {
                        absent.push(minute);
                    }
                }
            }
            if answered.is_none() {
                facts.source_units = owed;
            }
            classify_absent(
                series,
                number,
                inputs,
                answered,
                &absent,
                &mut facts,
                &mut details,
                (&mut code, &mut reason),
            );
            if let Some(first) = absent.first() {
                note_diff(*first, &mut first_diff);
            }
        }
    }
    facts.code = code;
    facts.reason = reason;
    // A MINUTE BEFORE THE SERIES' FIRST SERVED DAY BAR IS ANOTHER SECURITY'S
    // (WP-13d's rule mirrored, CMP-18).
    if !stored.is_empty() && foreign_day(day, inputs) {
        not_this_series(&mut facts);
    }
    facts.rule_set = 0;
    if first_diff != u32::MAX {
        facts.first_diff = first_diff;
    }
    (facts, receipt, details)
}

/// The owed minutes the store lacks, split by who speaks for them: inside a
/// kept answer's asked window the answer does (a minute it lacks is the
/// vendor's hole, said by the answer itself); outside every one the gap
/// ledger does (§6.3 step 5); and GDFL's seconds say whether either vendor
/// has them (DB-A12).
#[expect(
    clippy::too_many_arguments,
    reason = "the one classification step of the minute record, writing into its facts"
)]
fn classify_absent(
    series: &Series,
    number: u32,
    inputs: &DayInputs<'_>,
    answered: Option<&DayAnswer>,
    absent: &[u32],
    facts: &mut Facts,
    details: &mut Vec<Detail>,
    (code, reason): (&mut Code, &mut u16),
) {
    let base_s = i64::from(number) * DAY_S - IST_OFFSET_SECS;
    let asked_for = |minute: u32| {
        let second = base_s + i64::from(minute) * 60;
        answered.is_some_and(|a| second >= a.asked.0 && second <= a.asked.1)
    };
    let (inside, outside): (Vec<u32>, Vec<u32>) =
        absent.iter().copied().partition(|m| asked_for(*m));
    let refused_at = answered.map(refused_minutes).unwrap_or_default();
    let unanswered: Vec<u32> = inside
        .into_iter()
        .filter(|m| answered.is_some_and(|a| !a.bars.contains_key(m)) && !refused_at.contains(m))
        .collect();
    facts.missing = facts
        .missing
        .saturating_add(u32::try_from(unanswered.len() + outside.len()).unwrap_or(u32::MAX));
    if let (Some(answer), false) = (answered, unanswered.is_empty()) {
        let short = answer.last_stamp > 0
            && answer.last_stamp < answer.asked.1
            && unanswered
                .iter()
                .all(|m| base_s + i64::from(*m) * 60 > answer.last_stamp);
        let word = if short {
            "answer_short"
        } else {
            "vendor_empty"
        };
        worsen(Code::VendorHole, reason_of(word), code, reason);
    }
    let lacking: Vec<u32> = unanswered.iter().chain(&outside).copied().collect();
    if lacking.is_empty() {
        return;
    }
    let (mut class, mut why) = if outside.is_empty() {
        (Code::Match, 0)
    } else {
        ledger_class(inputs, false)
    };
    // THE PEER: GDFL's seconds of the same minutes (DB-A12).
    match inputs.peer {
        Some(peer) if matches!(series.segment, Segment::Index | Segment::Cash) => {
            let traded: Vec<u32> = lacking
                .iter()
                .copied()
                .filter(|m| peer.contains(m))
                .collect();
            let both = lacking.len() - traded.len();
            if !traded.is_empty() {
                facts.flags |= flags::PEER_MINUTE_ABSENT;
            }
            details.push(Detail {
                kind: words::DETAIL_PEER,
                payload: payload(&[
                    &u32::try_from(traded.len())
                        .unwrap_or(u32::MAX)
                        .to_le_bytes(),
                    &traded
                        .first()
                        .copied()
                        .unwrap_or(verdict::NONE)
                        .to_le_bytes(),
                    &u32::try_from(both).unwrap_or(u32::MAX).to_le_bytes(),
                ]),
            });
            let replaceable = matches!(class, Code::MissingInStore | Code::NoLedger)
                || why == reason_of("stored_short");
            if replaceable && !outside.iter().any(|m| peer.contains(m)) {
                class = Code::VendorHole;
                why = reason_of("neither_vendor_has_it");
            }
        }
        _ => facts.flags |= flags::PEER_UNAVAILABLE,
    }
    worsen(class, why, code, reason);
}

/// The minutes an answer's rows were refused at, as a set: built once per
/// day, O(refused), so each membership test after it is one hash probe.
fn refused_minutes(answer: &DayAnswer) -> HashSet<u32> {
    let mut out = HashSet::with_capacity(answer.refused.len());
    out.extend(answer.refused.iter().map(|(minute, _)| *minute));
    out
}

/// The code a day short of owed minutes takes from its ledger line (§6.3
/// step 5's table). `answered`: the minutes are ones the answer HAD and the
/// store lacks.
fn ledger_class(inputs: &DayInputs<'_>, answered: bool) -> (Code, u16) {
    match inputs.gap {
        Some(line) => match words::reason_word(line.why).unwrap_or("") {
            "vendor_empty" if !answered => (Code::VendorHole, line.why),
            "withheld_unmeasured_session" => (Code::WithheldByRule, line.why),
            // RA-301: a halted pull never asked for the day; nothing to compare.
            "calendar_unverified" | "calendar_past_record" | "unordered_stamps" | "pull_halted" => {
                (Code::NotCompared, line.why)
            }
            "refused_day" | "refused_front" | "stored_short" => (Code::Owed, line.why),
            _ => (Code::MissingInStore, reason_of("unexplained")),
        },
        None if inputs.covered => (Code::MissingInStore, reason_of("unexplained")),
        None => (Code::NoLedger, 0),
    }
}

/// One refolded bucket of a day.
#[derive(Debug, Clone, Copy)]
struct Bucket {
    /// Its first minute of the IST day.
    start: u32,
    /// Its fold.
    bar: Bar,
    /// Minutes it holds.
    held: u32,
    /// Minutes the calendar owes it.
    owed: u32,
    /// Whether it stands under D-3321.
    complete: bool,
    /// Whether it holds a minute the day does not owe.
    stray: bool,
}

/// Folds the day's stored minutes into `rung` on the 09:15 grid (this
/// module's own fold, independent of `pull::fold`, so the oracle never
/// checks a fold against itself (D-4811); D-3321's rule for which buckets
/// stand). The
/// open is the first minute's, the high the highest, the low the lowest,
/// the close the last minute's, the volume the sum, the open interest the
/// last one carried.
fn refold(
    series: &Series,
    number: u32,
    minutes: &[Bar],
    rung: Timeframe,
) -> (Vec<Bucket>, Option<&'static str>) {
    let width = i64::from(rung.secs() / 60).max(1);
    let windows = match repair_plan::owed(series.segment.as_str(), number) {
        Owes::Windows(windows) => Some(windows),
        Owes::Closed => None,
        Owes::Unattested => return (Vec::new(), Some("calendar_unverified")),
        Owes::Unsized => return (Vec::new(), Some("withheld_unmeasured_session")),
    };
    let day_start_us = day_start_us(number);
    let mut out: Vec<Bucket> = Vec::with_capacity(minutes.len() / 2 + 2);
    for bar in minutes {
        let minute = i64::from(minute_of(bar));
        let start = u32::try_from(OPEN_MINUTE + (minute - OPEN_MINUTE).div_euclid(width) * width)
            .unwrap_or(0);
        let owed_minute = windows
            .as_ref()
            .is_some_and(|w| in_windows(w, u32::try_from(minute).unwrap_or(0)));
        match out.last_mut() {
            Some(bucket) if bucket.start == start => {
                bucket.bar.high = bucket.bar.high.max(bar.high);
                bucket.bar.low = bucket.bar.low.min(bar.low);
                bucket.bar.close = bar.close;
                bucket.bar.volume = bucket.bar.volume.saturating_add(bar.volume);
                if bar.open_interest != OI_NULL {
                    bucket.bar.open_interest = bar.open_interest;
                }
                bucket.held += 1;
                bucket.stray |= !owed_minute;
            }
            _ => out.push(Bucket {
                start,
                bar: Bar {
                    ts_micros: day_start_us + i64::from(start) * MINUTE_US,
                    ..*bar
                },
                held: 1,
                owed: 0,
                complete: false,
                stray: !owed_minute,
            }),
        }
    }
    let cash = series.segment == Segment::Cash;
    let span = u32::try_from(width).unwrap_or(1);
    for bucket in &mut out {
        let owed = windows.as_ref().map_or(0, |w| {
            (bucket.start..bucket.start + span)
                .filter(|m| in_windows(w, *m))
                .count()
        });
        bucket.owed = u32::try_from(owed).unwrap_or(u32::MAX);
        bucket.complete =
            !bucket.stray && (bucket.held == bucket.owed || (cash && bucket.held > 0));
    }
    (out, None)
}

/// A WHOLE DAY ABSENT FROM THE RUNG WHILE ITS 1MIN SOURCE HOLDS IT
/// (RA-267): the rung holds fewer days than its source (a repaired 1min month
/// whose coarser rungs were never refolded, a special session the derive pass
/// skipped, a derive that has not run yet). Said under every rule set, a
/// month derived before the ledger included: no rule set withholds every
/// bucket of a day whose minutes fold to a standing bucket, so the absence is
/// never a matter of the bucket rule. Every standing bucket is missing, the
/// first named.
fn rung_day_absent(mut facts: Facts, buckets: &[Bucket], version: Option<u8>) -> Facts {
    facts.code = Code::MissingInStore;
    facts.reason = reason_of("rung_day_absent");
    facts.missing = facts.source_units;
    facts.rule_set = version.unwrap_or(0);
    if let Some(first) = buckets.iter().find(|b| b.complete) {
        facts.first_diff = first.start;
    }
    facts
}

/// A derived rung's record (D-4858).
fn derived_facts(
    series: &Series,
    day: Day,
    minutes: &[Bar],
    rung: Timeframe,
    stored: Option<&[Bar]>,
    version: Option<u8>,
) -> (Facts, Vec<Detail>) {
    let mut facts = Facts::of(Code::Match);
    let mut details = Vec::new();
    let stored = stored.unwrap_or_default();
    facts.store_units = u32::try_from(stored.len()).unwrap_or(u32::MAX);
    facts.rows = facts.store_units;
    if let (Some(first), Some(last)) = (stored.first(), stored.last()) {
        facts.first_unit = minute_of(first);
        facts.last_unit = minute_of(last);
    }
    let (buckets, unjudged) = refold(series, day.days_from_epoch(), minutes, rung);
    if let Some(word) = unjudged {
        facts.code = if word == "calendar_unverified" {
            Code::NotCompared
        } else {
            Code::WithheldByRule
        };
        facts.reason = reason_of(word);
        return (facts, details);
    }
    let standing = buckets.iter().filter(|b| b.complete).count();
    facts.source_units = u32::try_from(standing).unwrap_or(u32::MAX);
    if stored.is_empty() && standing > 0 {
        return (rung_day_absent(facts, &buckets, version), details);
    }
    let held: HashMap<u32, &Bar> = stored.iter().map(|b| (minute_of(b), b)).collect();
    let mut first_diff = u32::MAX;
    let mut withheld = 0_u32;
    for bucket in &buckets {
        match (bucket.complete, held.get(&bucket.start)) {
            (true, Some(bar)) => {
                let mut differs = false;
                if !same_prices(bar, &bucket.bar) {
                    facts.value_diff += 1;
                    differs = true;
                }
                if bar.volume != bucket.bar.volume {
                    facts.volume_diff += 1;
                    differs = true;
                }
                if bar.open_interest != bucket.bar.open_interest {
                    facts.oi_diff += 1;
                    differs = true;
                }
                if differs {
                    note_diff(bucket.start, &mut first_diff);
                }
            }
            (true, None) => {
                facts.missing += 1;
                note_diff(bucket.start, &mut first_diff);
            }
            (false, Some(_)) => {
                facts.extra += 1;
                note_diff(bucket.start, &mut first_diff);
            }
            (false, None) => withheld += 1,
        }
    }
    let starts: HashSet<u32> = buckets.iter().map(|b| b.start).collect();
    for minute in held.keys() {
        if !starts.contains(minute) {
            facts.extra += 1;
            note_diff(*minute, &mut first_diff);
        }
    }
    if first_diff != u32::MAX {
        facts.first_diff = first_diff;
    }
    if version == Some(RULES_D3321) {
        facts.rule_set = RULES_D3321;
        let (code, word) = if facts.value_diff + facts.volume_diff + facts.oi_diff > 0 {
            (Code::ValueMismatch, "")
        } else if facts.missing > 0 {
            (Code::MissingInStore, "unexplained")
        } else if facts.extra > 0 {
            (Code::ExtraInStore, "unexplained")
        } else if withheld > 0 {
            (Code::WithheldByRule, "index_bucket_incomplete")
        } else {
            (Code::Match, "")
        };
        facts.code = code;
        facts.reason = reason_of(word);
    } else {
        // DERIVED BEFORE THE LEDGER, OR UNDER A RULE SET THIS VERIFIER DOES
        // NOT KNOW: counts recorded, never judged by value.
        facts.code = Code::BuiltUnderEarlierRule;
        facts.reason = reason_of("pre_ledger");
        facts.rule_set = version.unwrap_or(0);
        details.push(Detail {
            kind: words::DETAIL_RULE,
            payload: payload(&[
                &[version.unwrap_or(0)],
                &reason_of("pre_ledger").to_le_bytes(),
            ]),
        });
    }
    (facts, details)
}

/// The 1day record: the kept answer, and `daycheck` over the day's minutes.
fn day_facts(inputs: &DayInputs<'_>) -> (Facts, u32, Vec<Detail>) {
    let mut facts = Facts::of(Code::Match);
    let mut details = Vec::new();
    facts.store_units = u32::from(inputs.day_bar.is_some());
    facts.rows = facts.store_units;
    let mut receipt = 0;
    let kept: Vec<&DayAnswer> = inputs.day_answers.iter().filter(|a| a.kept).collect();
    if let Some(answer) = kept.first() {
        receipt = answer.receipt;
        note_shape(answer, &mut details);
        let served = answer.bars.get(&0);
        facts.source_units = u32::from(served.is_some());
        match (served, inputs.day_bar.as_ref()) {
            (Some(want), Some(held)) => {
                facts.value_diff = u32::from(!same_prices(want, held));
                facts.volume_diff = u32::from(want.volume != held.volume);
                facts.oi_diff = u32::from(want.open_interest != held.open_interest);
                if facts.value_diff + facts.volume_diff + facts.oi_diff > 0 {
                    facts.code = Code::ValueMismatch;
                }
            }
            (Some(_), None) => {
                facts.missing = 1;
                facts.code = Code::MissingInStore;
                facts.reason = reason_of("unexplained");
            }
            (None, Some(_)) => {
                facts.extra = 1;
                facts.code = Code::ExtraInStore;
                facts.reason = reason_of("unexplained");
            }
            (None, None) => {
                facts.code = Code::VendorHole;
                facts.reason = reason_of("vendor_empty");
            }
        }
    } else {
        facts.code = Code::NoResponseKept;
        if !inputs.day_answers.is_empty() {
            facts.flags |= flags::BODY_NOT_KEPT;
        }
        if inputs.day_bar.is_none() && !inputs.minutes.is_empty() {
            facts.missing = 1;
            facts.code = Code::MissingInStore;
            facts.reason = reason_of("unexplained");
        }
    }
    // `daycheck` RE-RUN AT VERIFICATION TIME (D-3347): the day bar against
    // its own minutes.
    if let Some(bar) = inputs.day_bar
        && !inputs.minutes.is_empty()
        && let Ok(report) = crate::daycheck::compare(inputs.minutes, &[bar])
        && (report.differed > 0 || report.contradicted > 0)
    {
        facts.flags |= flags::DAYCHECK_DIFFERS;
        let mut bytes = Vec::with_capacity(28);
        for count in [
            report.agreed,
            report.differed,
            report.contradicted,
            report.close_differs,
            report.range_wider,
            report.volume_above,
            report.day_absent,
        ] {
            bytes.extend_from_slice(&u32::try_from(count).unwrap_or(u32::MAX).to_le_bytes());
        }
        details.push(Detail {
            kind: words::DETAIL_DAYCHECK,
            payload: bytes,
        });
    }
    // A DAY BAR NO `daycheck` COULD READ IS NAMED, never a silent pass (IA-9
    // repair 1): why, in one byte (`words::DAYCHECK_NOT_RUN`).
    if inputs.day_bar.is_some() && inputs.minutes.is_empty() {
        let why = if inputs.beyond_minutes {
            words::DAYCHECK_BEYOND_MINUTES
        } else if inputs.minutes_refused.is_some() {
            words::DAYCHECK_MINUTES_UNREAD
        } else {
            words::DAYCHECK_NO_MINUTE
        };
        details.push(Detail {
            kind: words::DETAIL_DAYCHECK_NOT_RUN,
            payload: vec![why],
        });
    }
    (facts, receipt, details)
}

// -------------------------------------------------------------- reading --

/// One month of one rung, as held.
struct Month {
    file: BarFile,
    bars: u64,
    generation: u32,
}

/// How long a read waits for a month a writer holds before it says so: the
/// importer's bound, ≤ 2 s (design §6.3, "A month a writer holds", round 4
/// SF-4, F67). A Zerodha read can meet a live pull appending the month
/// (`StoreError::Locked`: the reader's shared lock against the writer's);
/// past the wait the month's records are `not_compared (month_busy)`, never
/// `missing_in_store` and never `store_unreadable`, and the next run reads it
/// again.
pub const MONTH_BUSY_RETRY: std::time::Duration = std::time::Duration::from_secs(2);

/// The pause between two tries of a busy month.
const MONTH_BUSY_STEP: std::time::Duration = std::time::Duration::from_millis(100);

/// Why a held month would not open for a read: never read as a month not
/// held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthRefusal {
    /// A writer held it past the wait: a live pull, not a store fault.
    pub busy: bool,
    /// The store's words.
    pub why: String,
}

impl From<MonthRefusal> for String {
    fn from(refusal: MonthRefusal) -> Self {
        refusal.why
    }
}

/// The code and reason of a rung whose month would not open: `not_compared
/// (month_busy)` for a month a writer held, `store_unreadable
/// (month_refused)` for any other refusal.
fn month_refused(busy: bool) -> Facts {
    let (code, word) = if busy {
        (Code::NotCompared, "month_busy")
    } else {
        (Code::StoreUnreadable, "month_refused")
    };
    Facts {
        reason: reason_of(word),
        ..Facts::of(code)
    }
}

/// Opens one month through the store's one door (`BarFile::open_existing`,
/// the served generation): `Ok(None)` when it is not held. A month a writer
/// holds is tried again for up to [`MONTH_BUSY_RETRY`].
fn open_month(root: &Path, parts: PathParts<'_>) -> Result<Option<Month>, MonthRefusal> {
    open_month_within(root, parts, MONTH_BUSY_RETRY)
}

/// [`open_month`], waiting at most `wait` for a month a writer holds: one
/// try, then one every [`MONTH_BUSY_STEP`] until `wait` has passed.
fn open_month_within(
    root: &Path,
    parts: PathParts<'_>,
    wait: std::time::Duration,
) -> Result<Option<Month>, MonthRefusal> {
    let refused = |why: String| MonthRefusal { busy: false, why };
    let path = StorePath::new(parts).map_err(|why| refused(why.to_string()))?;
    let started = std::time::Instant::now();
    loop {
        let tried = store::generation::served(root, path).and_then(|served| {
            BarFile::open_existing(root, path, repair_plan::symbol_id(parts.symbol))
                .map(|file| (served, file))
        });
        match tried {
            Ok((served, file)) => {
                let bars = file.header().n_valid;
                return Ok(Some(Month {
                    file,
                    bars,
                    generation: served.ordinal(),
                }));
            }
            Err(StoreError::Missing { .. }) => return Ok(None),
            Err(StoreError::Locked { .. }) if started.elapsed() < wait => {
                std::thread::sleep(MONTH_BUSY_STEP.min(wait));
            }
            Err(why @ StoreError::Locked { .. }) => {
                return Err(MonthRefusal {
                    busy: true,
                    why: format!(
                        "month_busy: a writer held it past {} ms ({why}); not compared, the next run reads it again",
                        wait.as_millis()
                    ),
                });
            }
            Err(why) => return Err(refused(why.to_string())),
        }
    }
}

/// The bars of `[from, to)` (UTC micros), and the first one's row: two
/// `first_at_or_after` lookups and one read per bar.
fn read_range(month: &Month, from: i64, to: i64) -> Result<(u32, Vec<Bar>), String> {
    let at = |ts: i64| {
        month
            .file
            .first_at_or_after(ts)
            .map(|index| index.min(month.bars))
            .map_err(|why| why.to_string())
    };
    let (first, end) = (at(from)?, at(to)?);
    let mut out = Vec::with_capacity(usize::try_from(end.saturating_sub(first)).unwrap_or(0));
    for index in first..end {
        out.push(
            month
                .file
                .read_record(index)
                .map_err(|why| why.to_string())?,
        );
    }
    Ok((u32::try_from(first).unwrap_or(u32::MAX), out))
}

/// The UTC microsecond an IST day starts at.
fn day_start_us(number: u32) -> i64 {
    (i64::from(number) * DAY_S - IST_OFFSET_SECS) * 1_000_000
}

// ------------------------------------------------------------ the run --

/// What a cold verification is asked to check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdRequest {
    /// The first day.
    pub from: Day,
    /// The last day.
    pub to: Day,
    /// Store keys (`NSE-INDEX-NIFTY`) or symbols (`NIFTY`) to check; empty
    /// checks every Zerodha series held.
    pub only: Vec<String>,
}

/// The most examples a cold report keeps: one per (rung, code), and nine
/// rungs × a few codes fill fewer than this on a real store.
pub const MAX_EXAMPLES: usize = 48;

/// What a cold verification did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColdReport {
    /// Series checked.
    pub series: u64,
    /// Instrument-months read (at every rung they hold).
    pub months: u64,
    /// Of them, months outside the series' held 1min months (IA-9 repair
    /// 1): read at the rungs they hold, 1min not owed, `daycheck` named not
    /// run.
    pub beyond_months: u64,
    /// Days in those months that no calendar of record attests and that hold
    /// no bar at any rung and no kept answer: not judged, counted (a weekend
    /// of 2016 reads the same as a holiday there).
    pub unattested_days: u64,
    /// Names the request asked for that no held Zerodha series answers to, in
    /// the order asked: a run that checked nothing of a name says so.
    pub unmatched: Vec<String>,
    /// Instrument-days checked.
    pub days: u64,
    /// Records spooled, per verdict code (index = code).
    pub by_code: [u64; 19],
    /// Records spooled, per rung and code: (rung, code, count).
    pub by_rung: Vec<(String, Code, u64)>,
    /// The units the store lacks (minutes at 1min, buckets at a derived
    /// rung, the bar at 1day), summed per (rung, code, reason) over the
    /// records that name any: every owed unit's class in one table, so a
    /// run says how many minutes are the vendor's hole, the ledger's, or
    /// neither vendor's (DB-A12), not only how many days. Bounded by the
    /// rungs, codes and reasons the tables name.
    pub missing: Vec<(String, Code, u16, u64)>,
    /// Records spooled.
    pub records: u64,
    /// Months that could not be read, in words.
    pub refused: Vec<String>,
    /// Months a writer held past [`MONTH_BUSY_RETRY`] (a pull appending
    /// them), in words: their records read `not_compared (month_busy)`, and
    /// the span was not wholly compared, so the run says so; never a fault of
    /// the store.
    pub busy: Vec<String>,
    /// Names the walk read past (not a series, a rung or a month).
    pub stray: u64,
    /// Ledger lines the verifier could not parse.
    pub ledger_faults: u64,
    /// Whether a stop was asked.
    pub stopped: bool,
    /// The first non-match record of each (rung, code), at most
    /// [`MAX_EXAMPLES`], in the series' listing order: (rung, code, words).
    pub examples: Vec<(String, Code, String)>,
    /// The workers the series were checked on (RA-145).
    pub workers: u64,
}

impl ColdReport {
    /// Takes in one series' report (its place in the listing decides the
    /// order, so the folded report is the same whatever the worker count).
    fn fold(&mut self, one: Self) {
        self.series += one.series;
        self.months += one.months;
        self.beyond_months += one.beyond_months;
        self.unattested_days += one.unattested_days;
        self.unmatched.extend(one.unmatched);
        self.days += one.days;
        for (slot, more) in self.by_code.iter_mut().zip(one.by_code) {
            *slot += more;
        }
        for (rung, code, count) in one.by_rung {
            match self
                .by_rung
                .iter_mut()
                .find(|(r, c, _)| *r == rung && *c == code)
            {
                Some(found) => found.2 += count,
                None => self.by_rung.push((rung, code, count)),
            }
        }
        for (rung, code, reason, units) in one.missing {
            self.add_missing(&rung, code, reason, units);
        }
        self.records += one.records;
        self.refused.extend(one.refused);
        self.busy.extend(one.busy);
        self.stray += one.stray;
        self.stopped |= one.stopped;
        for example in one.examples {
            self.example(example);
        }
    }

    /// Keeps `example` if it is the first of its (rung, code) and there is
    /// room: one example of each kind, at most [`MAX_EXAMPLES`], so a rare
    /// verdict is shown however many common ones came first.
    fn example(&mut self, example: (String, Code, String)) {
        if self.examples.len() < MAX_EXAMPLES
            && !self
                .examples
                .iter()
                .any(|(rung, code, _)| *rung == example.0 && *code == example.1)
        {
            self.examples.push(example);
        }
    }

    /// Adds `units` lacking to the (rung, code, reason) row of [`Self::missing`].
    fn add_missing(&mut self, rung: &str, code: Code, reason: u16, units: u64) {
        match self
            .missing
            .iter_mut()
            .find(|(r, c, w, _)| r == rung && *c == code && *w == reason)
        {
            Some(found) => found.3 += units,
            None => self.missing.push((rung.to_owned(), code, reason, units)),
        }
    }

    fn count(&mut self, rung: Timeframe, record: &SpoolRecord, key: &str) {
        let code = record.facts.code;
        if let Some(slot) = self.by_code.get_mut(usize::from(code.code())) {
            *slot += 1;
        }
        if record.facts.missing > 0 {
            self.add_missing(
                rung.as_str(),
                code,
                record.facts.reason,
                u64::from(record.facts.missing),
            );
        }
        self.records += 1;
        match self
            .by_rung
            .iter_mut()
            .find(|(r, c, _)| r == rung.as_str() && *c == code)
        {
            Some(found) => found.2 += 1,
            None => self.by_rung.push((rung.as_str().to_owned(), code, 1)),
        }
        if code != Code::Match
            && self.examples.len() < MAX_EXAMPLES
            && !self
                .examples
                .iter()
                .any(|(r, c, _)| r == rung.as_str() && *c == code)
        {
            let words = format!(
                "{key} {} {}: {}{} missing={} extra={} value_diff={}",
                record.day,
                rung.as_str(),
                code.word(),
                words::reason_word(record.facts.reason)
                    .map(|w| format!(" ({w})"))
                    .unwrap_or_default(),
                record.facts.missing,
                record.facts.extra,
                record.facts.value_diff,
            );
            self.example((rung.as_str().to_owned(), code, words));
        }
    }
}

/// Every Zerodha series under the store that holds a rung this verifier
/// reads: 1min, a derived rung or 1day (IA-9 repair 1: a series held at 1day
/// only, eleven NSE indices on the operator's store, is verified at 1day,
/// never left out unnamed).
///
/// # Errors
///
/// The host's refusal listing a directory that exists.
pub fn series_held(root: &Path, stray: &mut u64) -> Result<Vec<Series>, String> {
    let mut out = Vec::new();
    let base = root.join("bars").join(Vendor::Zerodha.as_str());
    for exchange in [Exchange::Nse, Exchange::Bse] {
        for segment in [Segment::Index, Segment::Cash, Segment::Fno] {
            let dir = base.join(exchange.as_str()).join(segment.as_str());
            for (name, path) in list_dirs(&dir, stray)? {
                if holds_a_rung(&path) {
                    out.push(Series {
                        exchange,
                        segment,
                        symbol: name.clone(),
                        contract: None,
                    });
                }
                if segment != Segment::Fno {
                    continue;
                }
                for (contract, contract_path) in list_dirs(&path, stray)? {
                    let Some(parsed) = Contract::parse(&contract) else {
                        *stray += 1;
                        continue;
                    };
                    if holds_a_rung(&contract_path) {
                        out.push(Series {
                            exchange,
                            segment,
                            symbol: name.clone(),
                            contract: Some(parsed),
                        });
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Whether a series directory holds a rung directory this verifier reads:
/// 1min, a derived rung or 1day. At most nine `stat` calls.
fn holds_a_rung(series_dir: &Path) -> bool {
    [Timeframe::MINUTE_1, Timeframe::DAY_1]
        .into_iter()
        .chain(derived_rungs())
        .any(|rung| series_dir.join(rung.as_str()).is_dir())
}

/// Subdirectories of `dir` by name; a name that is hidden or not UTF-8, and a
/// plain file where only directories live (`Icon\r`, a stray download), is
/// stray: counted, never read, never removed (RA-315); an absent directory
/// has none.
fn list_dirs(dir: &Path, stray: &mut u64) -> Result<Vec<(String, PathBuf)>, String> {
    let listing = match std::fs::read_dir(dir) {
        Ok(listing) => listing,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(format!("{}: {why}", dir.display())),
    };
    let mut out = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|why| format!("{}: {why}", dir.display()))?;
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        match entry.file_name().to_str() {
            Some(name) if is_dir && !name.starts_with('.') => {
                out.push((name.to_owned(), entry.path()));
            }
            _ => *stray += 1,
        }
    }
    Ok(out)
}

/// `<root>/bars/zerodha/<EXCH>/<SEG>/<SYMBOL>[/<contract>]/<tf>`: the bar
/// path's own directory (`store::path`).
fn rung_dir(root: &Path, series: &Series, rung: Timeframe) -> PathBuf {
    let mut dir = root
        .join("bars")
        .join(Vendor::Zerodha.as_str())
        .join(series.exchange.as_str())
        .join(series.segment.as_str())
        .join(&series.symbol);
    if let Some(contract) = &series.contract {
        dir.push(contract.as_str());
    }
    dir.join(rung.as_str())
}

/// The month after `month`, or `None` past the last month a path can name.
fn next_month(month: YearMonth) -> Option<YearMonth> {
    let (year, number) = if month.month() == 12 {
        (month.year().checked_add(1)?, 1)
    } else {
        (month.year(), month.month() + 1)
    };
    YearMonth::new(year, number).ok()
}

/// The months held at `rung`, by name `<yyyy-mm>.bin`.
fn months_held(
    root: &Path,
    series: &Series,
    rung: Timeframe,
    stray: &mut u64,
) -> Result<Vec<YearMonth>, String> {
    let dir = rung_dir(root, series, rung);
    let listing = match std::fs::read_dir(&dir) {
        Ok(listing) => listing,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(format!("{}: {why}", dir.display())),
    };
    // A MONTH HELD IN BOTH FORMS IS ONE MONTH (SC-3, D-5899): `.bin` or its
    // sealed `.sbar`, which `BarFile::open_existing` opens by its own rule.
    let mut out: HashSet<YearMonth> = HashSet::with_capacity(128);
    for entry in listing {
        let entry = entry.map_err(|why| format!("{}: {why}", dir.display()))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            *stray += 1;
            continue;
        };
        let stem = name
            .strip_suffix(".bin")
            .or_else(|| name.strip_suffix(".sbar"));
        if let Some(stem) = stem {
            match parse_month(stem) {
                Some(month) => {
                    out.insert(month);
                }
                None => *stray += 1,
            }
        }
    }
    Ok(out.into_iter().collect())
}

/// GDFL's one-second month of the same instrument (DB-A12's peer), opened once
/// per (series, month); `None` when GDFL holds no such month or it would not
/// open: the records then say `peer_unavailable`, never "absent at both". A
/// month that is held and would not open is also a fault in words, pushed to
/// `faults`, so an unreadable peer is never only a quieter verdict.
fn peer_month(
    root: &Path,
    series: &Series,
    month: YearMonth,
    (faults, busy): (&mut Vec<String>, &mut Vec<String>),
) -> Option<Month> {
    if series.contract.is_some() {
        return None;
    }
    let parts = PathParts {
        vendor: Vendor::Gdfl,
        exchange: series.exchange.as_str(),
        segment: series.segment.as_str(),
        symbol: &series.symbol,
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    };
    match open_month(root, parts) {
        Ok(held) => held,
        // A PEER AN IMPORT IS WRITING IS BUSY, NOT A FAULT: the minutes read
        // `peer_unavailable` and the run says the month apart.
        Err(why) => {
            let words = format!("{} {month} gdfl 1s peer: {}", series.key_text(), why.why);
            if why.busy {
                busy.push(words);
            } else {
                faults.push(words);
            }
            None
        }
    }
}

/// Whether the peer's one-second month holds any second of IST day
/// `number`: two `first_at_or_after` lookups, O(1) a day. A month held with
/// none of the day is a GDFL day not imported, not a day nobody traded.
fn peer_holds_day(peer: &Month, number: u32) -> Result<bool, String> {
    let at = |ts: i64| {
        peer.file
            .first_at_or_after(ts)
            .map(|index| index.min(peer.bars))
            .map_err(|why| why.to_string())
    };
    Ok(at(day_start_us(number + 1))? > at(day_start_us(number))?)
}

/// Of `minutes` (minutes past IST midnight of `number`), those in which the
/// peer's one-second bars hold a second: two `first_at_or_after` lookups a
/// minute, so a day costs O(its absent minutes), never a read of its seconds.
fn peer_traded(
    peer: &Month,
    number: u32,
    minutes: impl Iterator<Item = u32>,
) -> Result<HashSet<u32>, String> {
    let mut out = HashSet::with_capacity(16);
    let at = |ts: i64| {
        peer.file
            .first_at_or_after(ts)
            .map(|index| index.min(peer.bars))
            .map_err(|why| why.to_string())
    };
    for minute in minutes {
        let from = day_start_us(number) + i64::from(minute) * MINUTE_US;
        if at(from + MINUTE_US)? > at(from)? {
            out.insert(minute);
        }
    }
    Ok(out)
}

/// One kept body read and decoded, with the fields the checker's own eyes
/// count on it (RA-318, [`answer_shape`]); the shape is empty for a body
/// that would not read or decode, whose refusal is the answer.
fn kept_window(
    root: &Path,
    pack: response_keep::Pack<'_>,
    month: YearMonth,
    receipt: &response_keep::Receipt,
    spec: &crate::vendor::HttpSpec,
    listing: crate::vendor::Listing,
) -> (Result<crate::fetch::RawWindow, String>, Shape) {
    let body = match response_keep::body(root, pack, month, receipt) {
        Ok(body) => body,
        Err(why) => return (Err(why.to_string()), Shape::default()),
    };
    match crate::http::decode_body(&body, spec, listing) {
        Ok(window) => (Ok(window), answer_shape(&body)),
        Err(why) => (Err(why.to_string()), Shape::default()),
    }
}

/// The kept answers of one month at `rung`, decoded, as the oracle reads
/// them per day, numbered in receipt order from `next`; and every pack, body
/// or refused-row file that would not read, in words (never dropped: an
/// unreadable body is an answer `kept=false`, reason `body_unreadable`).
fn month_answers(
    root: &Path,
    series: &Series,
    rung: Timeframe,
    month: YearMonth,
    next: &mut u32,
) -> (HashMap<u32, Vec<DayAnswer>>, Vec<String>) {
    let mut out: HashMap<u32, Vec<DayAnswer>> = HashMap::with_capacity(32);
    let mut faults = Vec::new();
    let Some(instrument) = series.instrument() else {
        return (out, faults);
    };
    let pack = response_keep::Pack {
        vendor: Vendor::Zerodha,
        key: &instrument,
        timeframe: rung,
    };
    let segments = match response_keep::receipts(root, pack, month) {
        Ok(segments) => segments,
        Err(why) => {
            faults.push(format!(
                "{} {month} {} kept answers: {why}",
                series.key_text(),
                rung.as_str()
            ));
            return (out, faults);
        }
    };
    let crate::vendor::Transport::Http(spec) = &crate::vendor::Feed::Zerodha.descriptor().transport
    else {
        faults.push("the Zerodha descriptor is not an HTTP feed".to_owned());
        return (out, faults);
    };
    let listing = match series.segment {
        Segment::Index => crate::vendor::Listing::Index,
        _ => crate::vendor::Listing::Equity,
    };
    // One alert put per month read at most (RA-318): the queue de-duplicates
    // by key as well, so this only spares it the repeated probe.
    let mut shape_alerted = false;
    for (_, read) in segments {
        for (ordinal, receipt) in read.receipts.iter().enumerate() {
            *next += 1;
            let number = *next;
            let asked = (receipt.asked_from, receipt.asked_to);
            let unkept = |why: &str| DayAnswer {
                receipt: number,
                asked,
                kept: false,
                not_kept: reason_of(why),
                ..DayAnswer::default()
            };
            let days = days_of(asked);
            if let Some(why) = not_kept_word(receipt) {
                for day in &days {
                    out.entry(*day).or_default().push(unkept(why));
                }
                continue;
            }
            let (decoded, shape) = kept_window(root, pack, month, receipt, spec, listing);
            if shape.unknown > 0 && !std::mem::replace(&mut shape_alerted, true) {
                faults.extend(shape_alert(root, series, rung, month, &shape));
            }
            let window = match decoded {
                Ok(window) => window,
                Err(why) => {
                    faults.push(format!(
                        "{} {month} {} receipt {ordinal}: {why}",
                        series.key_text(),
                        rung.as_str()
                    ));
                    for day in &days {
                        out.entry(*day).or_default().push(unkept("body_unreadable"));
                    }
                    continue;
                }
            };
            let mut made = oracle(&window.rows, spec.timestamps, rung, series.segment, asked);
            let refused = response_keep::refused_rows(
                root,
                pack,
                month,
                u32::try_from(ordinal).unwrap_or(u32::MAX),
                receipt,
            )
            .unwrap_or_else(|why| {
                faults.push(format!(
                    "{} {month} {} receipt {ordinal} refused rows: {why}",
                    series.key_text(),
                    rung.as_str()
                ));
                Vec::new()
            });
            made.take_landing_refusals(&refused, receipt.rows_refused);
            // THE DECODER'S SKIPS, COUNTED (§8 Z6): see `DayAnswer::decoded`.
            let decoded = (u64::from(receipt.rows_decoded), window.rows.len() as u64);
            for day in days {
                let mut answer = made.days.remove(&day).unwrap_or_default();
                answer.receipt = number;
                answer.asked = asked;
                answer.kept = true;
                answer.last_stamp = receipt.last_stamp;
                answer.decoded = (decoded.0 != decoded.1).then_some(decoded);
                answer.shape = shape.clone();
                out.entry(day).or_default().push(answer);
            }
        }
    }
    (out, faults)
}

/// Why a receipt holds no body to read: `kept=0` (the pack or the disk was
/// full) or `kept=4` (the answer was refused and its head kept); `None` for a
/// receipt whose body (its own, or one it points at) is in a pack.
const fn not_kept_word(receipt: &response_keep::Receipt) -> Option<&'static str> {
    match receipt.kept {
        response_keep::Kept::NotKept => Some(match receipt.reason {
            response_keep::Reason::StorageFull => "storage_full",
            _ => "pack_full",
        }),
        response_keep::Kept::RefusedHead => Some("refused_answer"),
        _ => None,
    }
}

/// The IST day numbers a window `[from, to]` (unix seconds) touches, at
/// most 401 (a window is at most the vendor's cap, far under it).
fn days_of((from, to): (i64, i64)) -> Vec<u32> {
    let first = ist_day_minute(from).0;
    let last = ist_day_minute(to.max(from)).0;
    (first..=last.min(first + 400))
        .filter_map(|d| u32::try_from(d).ok())
        .collect()
}

/// The day of the series' first served day bar: the first bar of its first
/// held 1day month, or `None` when it holds none. A caller of the window
/// path reads it once per series per run and hands it to every window.
///
/// # Errors
///
/// The 1day directory or its first month would not read, in words, with
/// `busy` set when a writer held that month past [`MONTH_BUSY_RETRY`] (a
/// series listed this month: its first 1day month is the one a pull
/// appends).
pub fn first_served_day(root: &Path, series: &Series) -> Result<Option<Day>, MonthRefusal> {
    let refused = |why: String| MonthRefusal { busy: false, why };
    let mut stray = 0;
    let Some(first) = months_held(root, series, Timeframe::DAY_1, &mut stray)
        .map_err(refused)?
        .into_iter()
        .min()
    else {
        return Ok(None);
    };
    let Some(month) = open_month(root, series.parts(Timeframe::DAY_1, first))? else {
        return Ok(None);
    };
    if month.bars == 0 {
        return Ok(None);
    }
    let bar = month
        .file
        .read_record(0)
        .map_err(|why| refused(why.to_string()))?;
    let (number, _) = ist_day_minute(bar.ts_micros.div_euclid(1_000_000));
    Ok(u32::try_from(number)
        .ok()
        .and_then(|n| Day::from_days(n).ok()))
}

/// Whether `series` is one `only` names.
fn wanted(series: &Series, only: &[String]) -> bool {
    only.is_empty()
        || only
            .iter()
            .any(|name| *name == series.key_text() || *name == series.symbol)
}

/// One series' one month, opened at every rung the cold sweep reads.
struct HeldMonth {
    /// The month lies outside the series' held 1min months (IA-9 repair 1):
    /// its 1min rung is not owed, only the rungs it holds are verified.
    beyond: bool,
    minutes: Option<Month>,
    minutes_refused: Option<String>,
    derived: Vec<(Timeframe, Option<Month>, Option<u8>)>,
    daily: Option<Month>,
    answers: HashMap<u32, Vec<DayAnswer>>,
    day_answers: HashMap<u32, Vec<DayAnswer>>,
    covered: bool,
    /// GDFL's one-second month of the same instrument, the peer.
    peer: Option<Month>,
    /// Every month or kept-answer file that would not read, in words.
    faults: Vec<String>,
    /// Each rung whose month is held but would not open, and whether a
    /// writer held it ([`DayInputs::refused`]).
    refused: HashMap<Timeframe, bool>,
    /// Every month a writer held past [`MONTH_BUSY_RETRY`], in words: not a
    /// fault, said apart (the next run reads it again).
    busy: Vec<String>,
}

impl HeldMonth {
    fn open(
        root: &Path,
        series: &Series,
        (month, beyond): (YearMonth, bool),
        ledger: &GapLedger,
        next: &mut u32,
    ) -> Self {
        let key = series.key_text();
        let Rungs {
            month: _,
            minutes,
            minutes_refused,
            derived,
            daily,
            refused,
            mut faults,
            mut busy,
        } = Rungs::open(root, series, month, ledger, (!series.is_option(), true));
        let (answers, minute_faults) =
            month_answers(root, series, Timeframe::MINUTE_1, month, next);
        let (day_answers, day_faults) = month_answers(root, series, Timeframe::DAY_1, month, next);
        faults.extend(minute_faults);
        faults.extend(day_faults);
        // NO PEER OUTSIDE THE 1MIN MONTHS: no minute there is compared.
        let peer = if beyond {
            None
        } else {
            peer_month(root, series, month, (&mut faults, &mut busy))
        };
        Self {
            beyond,
            minutes,
            minutes_refused,
            derived,
            daily,
            answers,
            day_answers,
            covered: ledger.covers(&key, month),
            peer,
            faults,
            refused,
            busy,
        }
    }
}

/// One series' one month at the rungs a read needs, each month opened once
/// (the cold sweep per series-month, the window path per month a window
/// touches): a month that would not open is said by rung, a writer's hold
/// apart, and never read as a month not held (design §6.3, "A month a
/// writer holds").
struct Rungs {
    /// The month.
    month: YearMonth,
    /// The 1min month; its refusal's words.
    minutes: Option<Month>,
    minutes_refused: Option<String>,
    /// Each derived rung's month and its `derive` line version.
    derived: Vec<(Timeframe, Option<Month>, Option<u8>)>,
    /// The 1day month.
    daily: Option<Month>,
    /// Each rung whose month would not open, and whether a writer held it.
    refused: HashMap<Timeframe, bool>,
    /// The refusals' words: busy months apart from the rest.
    faults: Vec<String>,
    busy: Vec<String>,
}

impl Rungs {
    /// Opens 1min always, the derived rungs when `derived`, 1day when
    /// `daily`.
    fn open(
        root: &Path,
        series: &Series,
        month: YearMonth,
        ledger: &GapLedger,
        (derived, daily): (bool, bool),
    ) -> Self {
        let key = series.key_text();
        let mut faults = Vec::new();
        let mut busy = Vec::new();
        let mut refused = HashMap::with_capacity(9);
        let mut open = |rung: Timeframe| match open_month(root, series.parts(rung, month)) {
            Ok(held) => (held, None),
            Err(why) => {
                let words = format!("{key} {month} {}: {}", rung.as_str(), why.why);
                if why.busy {
                    busy.push(words);
                } else {
                    faults.push(words);
                }
                refused.insert(rung, why.busy);
                (None, Some(why.why))
            }
        };
        let (minutes, minutes_refused) = open(Timeframe::MINUTE_1);
        let derived = if derived {
            derived_rungs()
                .into_iter()
                .map(|rung| (rung, open(rung).0, ledger.derive_version(&key, rung, month)))
                .collect()
        } else {
            Vec::new()
        };
        let daily = if daily {
            open(Timeframe::DAY_1).0
        } else {
            None
        };
        Self {
            month,
            minutes,
            minutes_refused,
            derived,
            daily,
            refused,
            faults,
            busy,
        }
    }
}

/// The bars of `number` in `month` and the first one's row; nothing for a
/// month not held.
fn day_of(month: Option<&Month>, number: u32) -> Result<(u32, Vec<Bar>), String> {
    month.map_or(Ok((0, Vec::new())), |m| {
        read_range(m, day_start_us(number), day_start_us(number + 1))
    })
}

/// What a cold run reads once and every series shares: the gap ledger, and
/// the CASH symbols the store holds (for [`Lives`], RA-302).
struct ColdShared<'a> {
    ledger: &'a GapLedger,
    cash_held: &'a HashSet<String>,
}

/// What bounds the days one series owes in a cold run, read once per series:
/// its first served day bar (WP-13d, CMP-18), and, for a renamed share whose
/// OTHER name the store also holds, the days that name owes (RA-302, the
/// alias table's lineage, D-3332). One probe of the table each way per
/// series; one comparison per day.
#[derive(Debug, Clone, Copy, Default)]
struct Lives {
    /// The series' first served day bar.
    first_day: Option<Day>,
    /// This is the NEW name and the old one is held: days before this one are
    /// the old name's.
    old_name_until: Option<Day>,
    /// This is the OLD name and the new one is held: days from this one on are
    /// the new name's.
    new_name_from: Option<Day>,
}

impl Lives {
    fn of(series: &Series, first_day: Option<Day>, cash_held: &HashSet<String>) -> Self {
        let mut lives = Self {
            first_day,
            ..Self::default()
        };
        if series.segment != Segment::Cash {
            return lives;
        }
        let lineage = &brutex_core::alias::LINEAGE;
        let day = |at: brutex_core::alias::Ymd| Day::new(at.year, at.month, at.day).ok();
        lives.old_name_until = lineage
            .to_new(&series.symbol)
            .filter(|change| cash_held.contains(change.old))
            .and_then(|change| day(change.effective));
        lives.new_name_from = lineage
            .from_old(&series.symbol)
            .filter(|change| cash_held.contains(change.new))
            .and_then(|change| day(change.effective));
        lives
    }

    /// Whether another held name of the lineage owes `day`.
    fn other_name_owes(self, day: Day) -> bool {
        self.old_name_until.is_some_and(|until| day < until)
            || self.new_name_from.is_some_and(|from| day >= from)
    }
}

/// One day of one series, read at every rung, for [`cold`]: a rung whose
/// day would not read is named in `faults` and left out, never read as
/// empty.
fn cold_day(
    series: &Series,
    day: Day,
    held: &HeldMonth,
    ledger: &GapLedger,
    lives: Lives,
    (faults, unattested): (&mut Vec<String>, &mut u64),
) -> Option<Vec<RungRecord>> {
    let key = series.key_text();
    let number = day.days_from_epoch();
    let (first_row, minutes, unread) = match day_of(held.minutes.as_ref(), number) {
        Ok((first_row, minutes)) => (first_row, minutes, None),
        Err(why) => {
            faults.push(format!("{key} {day} 1min: {why}"));
            (0, Vec::new(), Some(why))
        }
    };
    let open = !matches!(
        repair_plan::owed(series.segment.as_str(), number),
        Owes::Closed
    );
    if !open && minutes.is_empty() && unread.is_none() {
        return None;
    }
    // A DAY BEFORE THE SERIES WAS FIRST SERVED, HOLDING NOTHING, IS NOT OWED
    // BY IT (`repair_plan`'s `before_first`): no record, never a hole.
    if minutes.is_empty() && unread.is_none() && lives.first_day.is_some_and(|first| day < first) {
        return None;
    }
    // A DAY ANOTHER HELD NAME OF THE SHARE'S LINEAGE OWES, HOLDING NOTHING
    // HERE, IS NOT OWED UNDER THIS ONE (RA-302): with both ZOMATO and ETERNAL
    // held, ZOMATO from 9 April 2025 is ETERNAL's. When only one name is
    // held (Zerodha files a share's whole history under its newest name), that
    // name owes every day, so a real hole is never hidden by a rename.
    if minutes.is_empty() && unread.is_none() && lives.other_name_owes(day) {
        return None;
    }
    let mut derived: Vec<(Timeframe, Vec<Bar>, bool, Option<u8>)> = Vec::with_capacity(7);
    for (rung, month_held, version) in &held.derived {
        match day_of(month_held.as_ref(), number) {
            Ok((_, bars)) => derived.push((*rung, bars, month_held.is_some(), *version)),
            Err(why) => faults.push(format!("{key} {day} {}: {why}", rung.as_str())),
        }
    }
    let (day_bar, day_held) = match day_of(held.daily.as_ref(), number) {
        Ok((_, bars)) => (bars.first().copied(), held.daily.is_some()),
        Err(why) => {
            faults.push(format!("{key} {day} 1day: {why}"));
            (None, false)
        }
    };
    // OUTSIDE THE 1MIN MONTHS AND THE CALENDAR OF RECORD, A DAY HOLDING NO
    // BAR AT ANY RUNG AND NO KEPT ANSWER IS NOT JUDGED (IA-9 repair 1): no
    // calendar says it traded (a weekend of 2016 reads the same as a
    // holiday), so a record would claim an answer was owed. It is counted
    // and said, never silent.
    if held.beyond
        && day_bar.is_none()
        && !held.day_answers.contains_key(&number)
        && matches!(
            repair_plan::owed(series.segment.as_str(), number),
            Owes::Unattested
        )
        && derived.iter().all(|(_, bars, _, _)| bars.is_empty())
    {
        *unattested += 1;
        return None;
    }
    let peer = match (
        &held.peer,
        repair_plan::owed(series.segment.as_str(), number),
    ) {
        (Some(peer), Owes::Windows((windows, count))) => {
            let have: HashSet<u32> = minutes.iter().map(minute_of).collect();
            let absent = windows
                .into_iter()
                .take(count)
                .flat_map(|(from, to)| from..to)
                .filter(|m| !have.contains(m));
            // A PEER THAT HOLDS NO SECOND OF THE WHOLE DAY did not import it,
            // so it cannot say a minute was not traded: the day reads
            // `peer_unavailable`, never "neither vendor has it" (DB-A12;
            // measured on the operator's store 22 Oct 2026: GDFL's NIFTY 1s
            // month is held but holds no second of 2021-02-24, 2022-03-07 or
            // the Muhurat sessions). A peer read that fails is named, never
            // read as no peer in silence.
            match peer_holds_day(peer, number).and_then(|holds| {
                if holds {
                    peer_traded(peer, number, absent).map(Some)
                } else {
                    Ok(None)
                }
            }) {
                Ok(traded) => traded,
                Err(why) => {
                    faults.push(format!("{key} {day} gdfl 1s peer: {why}"));
                    None
                }
            }
        }
        _ => None,
    };
    let refused = unread.as_deref().or(held.minutes_refused.as_deref());
    let inputs = DayInputs {
        minutes: &minutes,
        first_row,
        generation: held.minutes.as_ref().map_or(0, |m| m.generation),
        minutes_refused: refused,
        derived: derived
            .iter()
            .map(|(rung, bars, is_held, version)| {
                (*rung, is_held.then_some(bars.as_slice()), *version)
            })
            .collect(),
        day_bar,
        day_month_held: day_held,
        answers: held.answers.get(&number).map_or(&[][..], Vec::as_slice),
        day_answers: held.day_answers.get(&number).map_or(&[][..], Vec::as_slice),
        gap: ledger.gap(&key, day),
        covered: held.covered,
        peer: peer.as_ref(),
        first_day_bar: lives.first_day,
        refused: Some(&held.refused),
        beyond_minutes: held.beyond,
    };
    Some(verify_day(series, day, &inputs))
}

/// The most workers a cold verification runs on (RA-145): each series is
/// its own files, so series are checked side by side; the spool's one
/// writer serialises only the appends.
pub const MAX_COLD_WORKERS: usize = 8;

/// The workers a cold verification runs on when the caller names none: the
/// host's parallelism, at most [`MAX_COLD_WORKERS`].
#[must_use]
pub fn cold_workers() -> usize {
    std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .clamp(1, MAX_COLD_WORKERS)
}

/// The cold verification (§6.3 step 6, V-3) on [`cold_workers`] workers;
/// see [`cold_on`].
///
/// # Errors
///
/// As [`cold_on`].
pub fn cold(
    root: &Path,
    request: &ColdRequest,
    writer: &SpoolWriter,
    stop: Option<&StopControl>,
) -> Result<ColdReport, String> {
    cold_on(root, request, writer, stop, cold_workers())
}

/// The cold verification (§6.3 step 6, V-3): every held Zerodha series' days
/// in the span, whether or not a window landed on them, read from the store
/// only and spooled through `writer`. O(stored bars of the span) plus one
/// read of the gap ledger (L-IA-20). Every month, day or file that would not
/// read is named in [`ColdReport::refused`].
///
/// # Workers (RA-145)
///
/// Series are checked on `workers` threads (at least 1, at most
/// [`MAX_COLD_WORKERS`], never more than the series), each taking the next
/// series by one atomic step. Each series' report is kept by its place in the
/// listing and folded in that order, so the report, like the records, is the
/// same whatever the worker count; a worker that could not start is named in
/// a `warn` and the others carry its series.
///
/// # Errors
///
/// The ledger or the store's top directories could not be read; a spool
/// append that failed (the other workers stop at their next series, and
/// nothing after it is spooled by that series); no worker could start, or one
/// panicked.
pub fn cold_on(
    root: &Path,
    request: &ColdRequest,
    writer: &SpoolWriter,
    stop: Option<&StopControl>,
    workers: usize,
) -> Result<ColdReport, String> {
    cold_observed(root, request, writer, stop, workers, None)
}

/// [`cold_on`], keeping `progress` (design §5.7): forced at the begin with
/// the series to check, then after each series at most once a second (the
/// series done in "the day now's index", their months, days, records and
/// refusals in its counters). The caller, which holds the run's lock, ends
/// it ([`progress_end`]).
///
/// # Errors
///
/// As [`cold_on`].
pub fn cold_observed(
    root: &Path,
    request: &ColdRequest,
    writer: &SpoolWriter,
    stop: Option<&StopControl>,
    workers: usize,
    progress: Option<&crate::gdfl_run::ProgressFile>,
) -> Result<ColdReport, String> {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    let ledger = GapLedger::read(root)?;
    let mut stray = 0_u64;
    let span = (
        request.from.year_month().map_err(|why| why.to_string())?,
        request.to.year_month().map_err(|why| why.to_string())?,
    );
    let all = series_held(root, &mut stray)?;
    // EVERY HELD CASH NAME, NOT ONLY THE ASKED ONES: whether a renamed
    // share's other name is held decides which name owes a day (RA-302).
    let mut cash_held: HashSet<String> = HashSet::with_capacity(all.len());
    cash_held.extend(
        all.iter()
            .filter(|s| s.segment == Segment::Cash)
            .map(|s| s.symbol.clone()),
    );
    let shared = ColdShared {
        ledger: &ledger,
        cash_held: &cash_held,
    };
    // A NAME ASKED FOR THAT NO HELD SERIES ANSWERS TO is said, never a
    // quiet "0 series" (IA-9 repair 1). One pass over the names asked.
    let unmatched: Vec<String> = request
        .only
        .iter()
        .filter(|name| {
            !all.iter()
                .any(|s| **name == s.key_text() || **name == s.symbol)
        })
        .cloned()
        .collect();
    let series: Vec<Series> = all
        .into_iter()
        .filter(|s| wanted(s, &request.only))
        .collect();
    let want = workers.clamp(1, MAX_COLD_WORKERS).min(series.len().max(1));
    if let Some(progress) = progress {
        progress.update(true, |record| {
            record.day_total = u32::try_from(series.len()).unwrap_or(u32::MAX);
            record.workers = u32::try_from(want).unwrap_or(u32::MAX);
        });
    }
    let next = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    // A STOP SEEN BETWEEN SERIES, with series left: the run did not reach the
    // span's end, and says so even when no series saw the stop itself.
    let halted = AtomicBool::new(false);
    let job = || {
        let mut done: Vec<(usize, Result<ColdReport, String>)> = Vec::new();
        loop {
            if failed.load(Ordering::Relaxed) {
                break;
            }
            let at = next.fetch_add(1, Ordering::Relaxed);
            let Some(one) = series.get(at) else {
                break;
            };
            if stop.is_some_and(StopControl::requested) {
                halted.store(true, Ordering::Relaxed);
                break;
            }
            let made = cold_series(root, request, one, &shared, writer, stop, span);
            let stopped = match &made {
                Ok(report) => report.stopped,
                Err(_) => true,
            };
            if made.is_err() {
                failed.store(true, Ordering::Relaxed);
            }
            progress_after_series(progress, &made);
            done.push((at, made));
            if stopped {
                break;
            }
        }
        done
    };
    let finished: Vec<Vec<(usize, Result<ColdReport, String>)>> =
        on_workers("zerodha-cold", want, &job)
            .into_iter()
            .map(|done| {
                done.unwrap_or_else(|| vec![(usize::MAX, Err("a cold worker panicked".to_owned()))])
            })
            .collect();
    let mut report = fold_finished(finished, series.len())?;
    report.unmatched = unmatched;
    report.ledger_faults = ledger.faults.len() as u64;
    report.stopped |= halted.load(Ordering::Relaxed);
    report.stray += stray;
    let _ = telemetry::emit(
        &telemetry::Event::info(TARGET, "cold verification spooled")
            .with("run", telemetry::Value::Uint(writer.run()))
            .with("series", telemetry::Value::Uint(report.series))
            .with("months", telemetry::Value::Uint(report.months))
            .with("days", telemetry::Value::Uint(report.days))
            .with("records", telemetry::Value::Uint(report.records))
            .with(
                "refused",
                telemetry::Value::Uint(report.refused.len() as u64),
            )
            .with("busy", telemetry::Value::Uint(report.busy.len() as u64))
            .with("stray", telemetry::Value::Uint(report.stray))
            .with("workers", telemetry::Value::Uint(report.workers)),
    );
    Ok(report)
}

/// One series done, into the cold run's progress record: the series index,
/// its months, days, records and refusals, or one refusal for a series that
/// failed; written at most once a second (design §5.7's cadence).
fn progress_after_series(
    progress: Option<&crate::gdfl_run::ProgressFile>,
    made: &Result<ColdReport, String>,
) {
    let Some(progress) = progress else {
        return;
    };
    progress.update(false, |record| {
        record.day_index = record.day_index.saturating_add(1);
        match made {
            Ok(one) => {
                record.files = record.files.saturating_add(one.months);
                record.days_imported = record.days_imported.saturating_add(one.days);
                record.rows = record.rows.saturating_add(one.records);
                record.files_refused = record
                    .files_refused
                    .saturating_add(one.refused.len() as u64);
            }
            Err(_) => record.refusals_total = record.refusals_total.saturating_add(1),
        }
    });
}

/// The workers' answers folded into one report in the series' listing order,
/// so the answer is the same at any width (RA-145); a worker that panicked,
/// an answer for no series and a series that failed each refuse by name.
fn fold_finished(
    finished: Vec<Vec<(usize, Result<ColdReport, String>)>>,
    series: usize,
) -> Result<ColdReport, String> {
    if finished.is_empty() {
        return Err("no cold verification worker could start".to_owned());
    }
    let started = finished.len();
    let mut slots: Vec<Option<Result<ColdReport, String>>> = Vec::with_capacity(series);
    slots.resize_with(series, || None);
    for (at, made) in finished.into_iter().flatten() {
        match slots.get_mut(at) {
            Some(slot) => *slot = Some(made),
            None => {
                return Err(made
                    .err()
                    .unwrap_or_else(|| "a cold worker answered for no series".to_owned()));
            }
        }
    }
    let mut report = ColdReport {
        workers: started as u64,
        ..ColdReport::default()
    };
    for made in slots.into_iter().flatten() {
        report.fold(made?);
    }
    Ok(report)
}

/// Runs `job` on `want` named threads and returns what each returned, `None`
/// for one that panicked. A thread that could not start is named in a `warn`
/// and the others carry its share; none started is an empty answer.
fn on_workers<T: Send>(name: &str, want: usize, job: &(dyn Fn() -> T + Sync)) -> Vec<Option<T>> {
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(want);
        for at in 0..want {
            match std::thread::Builder::new()
                .name(format!("{name}-{at}"))
                .spawn_scoped(scope, job)
            {
                Ok(handle) => handles.push(handle),
                Err(why) => {
                    let _ = telemetry::emit(
                        &telemetry::Event::warn(TARGET, "fewer workers than asked for")
                            .with("asked", telemetry::Value::Uint(want as u64))
                            .with("started", telemetry::Value::Uint(handles.len() as u64))
                            .with("why", telemetry::Value::Str(&why.to_string())),
                    );
                    break;
                }
            }
        }
        handles
            .into_iter()
            .map(|handle| handle.join().ok())
            .collect()
    })
}

/// One series of a cold verification: every month of its life in the span,
/// in order, its records spooled per (rung, month). Its own report.
fn cold_series(
    root: &Path,
    request: &ColdRequest,
    series: &Series,
    shared: &ColdShared<'_>,
    writer: &SpoolWriter,
    stop: Option<&StopControl>,
    (from_month, to_month): (YearMonth, YearMonth),
) -> Result<ColdReport, String> {
    let mut report = ColdReport {
        series: 1,
        ..ColdReport::default()
    };
    let key = series.key_text();
    // A FIRST 1DAY MONTH A PULL IS APPENDING (a series listed this month) IS
    // BUSY, NOT A FAULT: said apart, as any month a writer holds.
    let first_day = first_served_day(root, series).unwrap_or_else(|why| {
        let words = format!("{key} first day bar: {}", why.why);
        if why.busy {
            report.busy.push(words);
        } else {
            report.refused.push(words);
        }
        None
    });
    let lives = Lives::of(series, first_day, shared.cash_held);
    let mut next_receipt = 0_u32;
    let months =
        months_held(root, series, Timeframe::MINUTE_1, &mut report.stray).unwrap_or_else(|why| {
            report.refused.push(format!("{key} 1min months: {why}"));
            Vec::new()
        });
    // EVERY MONTH OF THE SERIES' LIFE IN THE SPAN, IN ORDER, HELD OR NOT:
    // from its first held 1min month to its last. A month missing in the
    // middle is walked like any other, so each of its calendar-open days
    // gets a record (classified by the ledger), never silence; and the walk
    // is in month order, so the receipts are numbered the same on every run
    // (a listing's order is the file system's).
    let span = months
        .iter()
        .min()
        .copied()
        .zip(months.iter().max().copied());
    let walk = months_to_walk(root, series, span, (from_month, to_month), &mut report);
    for month in walk {
        if stop.is_some_and(StopControl::requested) {
            report.stopped = true;
            return Ok(report);
        }
        let beyond = span.is_none_or(|(first, last)| month < first || month > last);
        report.months += 1;
        report.beyond_months += u64::from(beyond);
        let held = HeldMonth::open(
            root,
            series,
            (month, beyond),
            shared.ledger,
            &mut next_receipt,
        );
        report.refused.extend(held.faults.iter().cloned());
        report.busy.extend(held.busy.iter().cloned());
        let mut by_rung: HashMap<Timeframe, Vec<SpoolRecord>> = HashMap::with_capacity(9);
        let Ok(mut day) = Day::new(month.year(), month.month(), 1) else {
            continue;
        };
        while day.year_month().ok() == Some(month) {
            if day >= request.from && day <= request.to {
                let made = cold_day(
                    series,
                    day,
                    &held,
                    shared.ledger,
                    lives,
                    (&mut report.refused, &mut report.unattested_days),
                );
                if let Some(records) = made {
                    report.days += 1;
                    for made in records {
                        report.count(made.timeframe, &made.record, &key);
                        by_rung.entry(made.timeframe).or_default().push(made.record);
                    }
                }
            }
            match day.succ() {
                Ok(next) => day = next,
                Err(_) => break,
            }
        }
        append_rungs(writer, series.segment, by_rung)?;
    }
    Ok(report)
}

/// The months a cold run walks for one series, in order: every month from
/// its first held 1min month to its last (`span`), held or not, and every
/// month held at 1day or a derived rung outside that span (IA-9 repair 1: a
/// series held at 1day only, and a 1day month before its first 1min month,
/// NIFTY's 2015 to 2017, are verified at the rungs they hold, never skipped
/// unnamed; their 1min rung is not the store's to owe there). Each within
/// the asked months. In order by the set itself, never a sort; one listing
/// per rung, O(months held). A rung whose months would not list is named in
/// the report.
fn months_to_walk(
    root: &Path,
    series: &Series,
    span: Option<(YearMonth, YearMonth)>,
    (from_month, to_month): (YearMonth, YearMonth),
    report: &mut ColdReport,
) -> std::collections::BTreeSet<YearMonth> {
    let key = series.key_text();
    let mut walk = std::collections::BTreeSet::new();
    if let Some((first, last)) = span {
        let end = last.min(to_month);
        let mut cursor = Some(first.max(from_month));
        while let Some(month) = cursor.filter(|m| *m <= end) {
            cursor = next_month(month);
            walk.insert(month);
        }
    }
    for rung in std::iter::once(Timeframe::DAY_1).chain(derived_rungs()) {
        let held = months_held(root, series, rung, &mut report.stray).unwrap_or_else(|why| {
            report
                .refused
                .push(format!("{key} {} months: {why}", rung.as_str()));
            Vec::new()
        });
        walk.extend(
            held.into_iter()
                .filter(|m| *m >= from_month && *m <= to_month),
        );
    }
    walk
}

/// One landed window, as the pull path's hook hands it over (IA-10c wires
/// it after `from_window` or `repair_window` returns and the keep has run).
#[derive(Debug, Clone, Copy)]
pub struct WindowInput<'a> {
    /// The series.
    pub series: &'a Series,
    /// The rung pulled: 1min or 1day.
    pub rung: Timeframe,
    /// The decoded rows.
    pub rows: &'a [RawRow],
    /// The window asked, unix seconds.
    pub asked: (i64, i64),
    /// The receipt as the run numbers them.
    pub receipt: u32,
    /// The landing's refused rows with their causes.
    pub refused: &'a [response_keep::RefusedRow],
    /// The rows the landing refused, as the receipt counts them.
    pub rows_refused: u32,
    /// The series' first served day bar ([`first_served_day`]), read once per
    /// series per run by the caller, never per window.
    pub first_day_bar: Option<Day>,
    /// The answer's fields outside the documented shape: the hook computes
    /// it with [`answer_shape`] on the body it keeps (RA-318), so the window
    /// is checked with the checker's own eyes as the cold sweep checks it.
    pub shape: &'a Shape,
}

/// A window whose answer changed shape puts its one alert (RA-318); an alert
/// that cannot be written is an error event by name, never dropped, and
/// never fails the landing whose verification it rides on.
fn window_shape_alert(root: &Path, window: &WindowInput<'_>) {
    if window.shape.unknown > 0
        && let Some(month) = days_of(window.asked)
            .first()
            .and_then(|number| Day::from_days(*number).ok())
            .and_then(|day| day.year_month().ok())
        && let Some(fault) = shape_alert(root, window.series, window.rung, month, window.shape)
    {
        let _ = telemetry::emit(
            &telemetry::Event::error(TARGET, "alert not written")
                .with("why", telemetry::Value::Str(&fault)),
        );
    }
}

/// Verifies one landed window's days (§6.3, per window): O(minutes of the
/// window × rungs) (L-IA-18), one spool append per rung.
///
/// # Which thread (RA-049)
///
/// The caller's, and it blocks: it opens up to nine months and syncs the
/// spool. The pull path's hook (IA-10c) calls it where the landing itself
/// runs, on the pull's blocking thread, never on a shared async worker; its
/// cost per window is C-ZV-01's, measured beside the keep's C-RK-02, so a
/// check that ever set the pull's pace would show in those two numbers
/// before it showed in a run.
///
/// # Errors
///
/// A day of an opened month that could not be read, or the spool append's
/// refusal. A month that would not open is no error: its rung's records say
/// it (`not_compared (month_busy)` past [`MONTH_BUSY_RETRY`], else
/// `store_unreadable (month_refused)`), with one event naming the words.
pub fn verify_window(
    root: &Path,
    window: &WindowInput<'_>,
    ledger: &GapLedger,
    writer: &SpoolWriter,
) -> Result<spool::Appended, String> {
    let series = window.series;
    let key = series.key_text();
    let encoding = zerodha_encoding()?;
    let mut made = oracle(
        window.rows,
        encoding,
        window.rung,
        series.segment,
        window.asked,
    );
    made.take_landing_refusals(window.refused, window.rows_refused);
    let first_day = window.first_day_bar;
    let daily = window.rung == Timeframe::DAY_1;
    let last_stamp = window.rows.last().map_or(0, |r| {
        i64::try_from(utc_millis(r.timestamp, encoding).div_euclid(1_000)).unwrap_or(0)
    });
    let mut by_rung: HashMap<Timeframe, Vec<SpoolRecord>> = HashMap::with_capacity(9);
    window_shape_alert(root, window);
    // EACH MONTH THE WINDOW TOUCHES IS OPENED ONCE, at every rung it needs,
    // never once per day (a 60-day window touches at most three months). A
    // month that would not open is said in its rung's records and in one
    // event, never read as a month not held and never a whole window lost:
    // a pull still appending it is `not_compared (month_busy)` (design §6.3,
    // "A month a writer holds").
    let mut held: Option<Rungs> = None;
    for number in days_of(window.asked) {
        let Ok(day) = Day::from_days(number) else {
            continue;
        };
        let month = day.year_month().map_err(|why| why.to_string())?;
        if held.as_ref().is_none_or(|rungs| rungs.month != month) {
            let rungs = Rungs::open(
                root,
                series,
                month,
                ledger,
                (!daily && !series.is_option(), daily),
            );
            window_month_refused(&rungs);
            held = Some(rungs);
        }
        let Some(rungs) = held.as_ref() else {
            continue;
        };
        let (first_row, day_minutes) = day_of(rungs.minutes.as_ref(), number)?;
        let mut answer = made.days.remove(&number).unwrap_or_default();
        answer.receipt = window.receipt;
        answer.asked = window.asked;
        answer.kept = true;
        answer.last_stamp = last_stamp;
        answer.shape = window.shape.clone();
        let answers = [answer];
        let mut derived = Vec::with_capacity(rungs.derived.len());
        for (rung, month_held, version) in &rungs.derived {
            let bars = day_of(month_held.as_ref(), number)?.1;
            derived.push((*rung, bars, month_held.is_some(), *version));
        }
        let (day_bar, day_held) = if daily {
            (
                day_of(rungs.daily.as_ref(), number)?.1.first().copied(),
                rungs.daily.is_some(),
            )
        } else {
            (None, false)
        };
        let (minute_answers, day_answers): (&[DayAnswer], &[DayAnswer]) = if daily {
            (&[], &answers)
        } else {
            (&answers, &[])
        };
        let inputs = DayInputs {
            minutes: &day_minutes,
            first_row,
            generation: rungs.minutes.as_ref().map_or(0, |m| m.generation),
            minutes_refused: rungs.minutes_refused.as_deref(),
            derived: derived
                .iter()
                .map(|(rung, bars, held, version)| {
                    (*rung, held.then_some(bars.as_slice()), *version)
                })
                .collect(),
            day_bar,
            day_month_held: day_held,
            answers: minute_answers,
            day_answers,
            gap: ledger.gap(&key, day),
            covered: ledger.covers(&key, month),
            peer: None,
            first_day_bar: first_day,
            refused: Some(&rungs.refused),
            beyond_minutes: false,
        };
        for made in verify_day(series, day, &inputs) {
            if (made.timeframe == Timeframe::DAY_1) == daily {
                by_rung.entry(made.timeframe).or_default().push(made.record);
            }
        }
    }
    append_rungs(writer, series.segment, by_rung)
}

/// One event per month the window path could not open: a warning for a month
/// a writer held (its records read `not_compared (month_busy)`), an error for
/// any other refusal (`store_unreadable (month_refused)`), each with the
/// store's words, so the record's code is never the only trace.
fn window_month_refused(rungs: &Rungs) {
    for (words, busy) in rungs
        .busy
        .iter()
        .map(|w| (w, true))
        .chain(rungs.faults.iter().map(|w| (w, false)))
    {
        let event = if busy {
            telemetry::Event::warn(TARGET, "month busy: not compared")
        } else {
            telemetry::Event::error(TARGET, "month refused to open")
        };
        let _ = telemetry::emit(&event.with("why", telemetry::Value::Str(words)));
    }
}

/// The Zerodha answer's timestamp encoding, read from its descriptor. A
/// descriptor that is not an HTTP feed is an error by name, never a guessed
/// encoding (CLAUDE.md §4: no fallback that hides a failure).
fn zerodha_encoding() -> Result<TimestampEncoding, String> {
    match &crate::vendor::Feed::Zerodha.descriptor().transport {
        crate::vendor::Transport::Http(spec) => Ok(spec.timestamps),
        crate::vendor::Transport::LocalArchive(_) => {
            Err("the Zerodha descriptor is not an HTTP feed".to_owned())
        }
    }
}

/// Appends each rung's records of one series to the run's spool, one
/// [`SpoolWriter::append`] per rung (each rung its own spool file), and sums
/// what was written. The first refusal stops it and is returned.
fn append_rungs(
    writer: &SpoolWriter,
    segment: Segment,
    by_rung: HashMap<Timeframe, Vec<SpoolRecord>>,
) -> Result<spool::Appended, String> {
    let mut done = spool::Appended::default();
    for (rung, records) in by_rung {
        let wrote = writer
            .append(segment, rung, &records)
            .map_err(|why| why.to_string())?;
        done.records += wrote.records;
        done.slots += wrote.slots;
        done.files += wrote.files;
        done.born += wrote.born;
    }
    Ok(done)
}

// ------------------------------------------------------------- progress --

/// A cold run's progress file (design §5.7), written only by the process
/// holding `verify/zerodha.lock`.
pub const PROGRESS_COLD: &str = "verify/zerodha.progress";
/// A merge's progress file, written only by the process holding
/// `verify/merge.lock`.
pub const PROGRESS_MERGE: &str = "verify/merge.progress";

/// The progress record a verification run starts from: the GDFL run's
/// record (`docs/02-store-format.md` §37, format v1), so one reader reads
/// every run. Its mode byte is `mode`'s position in the runs ledger's
/// `MODES` (where `import` is 1, as §37 numbers it), its kind byte the
/// position of `zerodha` in the ledger's `KINDS` plus one (indices 1, stocks
/// 2, options 3, as §37 numbers them), so neither number is a second table;
/// source 0 (none); `by` 1 (cli) or 2 (api). `span`: the days a cold run
/// covers (a merge covers none). What the counters hold for a verification
/// run is §35.10's "Progress" paragraph.
#[must_use]
pub fn progress_record(
    mode: &str,
    by: u8,
    run: u64,
    workers: usize,
    span: Option<(Day, Day)>,
) -> crate::gdfl_run::Progress {
    let position = |table: &[&str], word: &str| {
        table
            .iter()
            .position(|w| *w == word)
            .and_then(|at| u8::try_from(at).ok())
            .unwrap_or(0)
    };
    let ymd = |day: Day| {
        u32::from(day.year()) * 10_000 + u32::from(day.month()) * 100 + u32::from(day.day())
    };
    crate::gdfl_run::Progress {
        state: crate::gdfl_run::RunState::Running.code(),
        mode: position(&crate::gdfl_run::ledger::MODES, mode),
        kind: position(&crate::gdfl_run::ledger::KINDS, "zerodha").saturating_add(1),
        by,
        run,
        workers: u32::try_from(workers).unwrap_or(u32::MAX),
        from: span.map_or(0, |(from, _)| ymd(from)),
        to: span.map_or(0, |(_, to)| ymd(to)),
        ..crate::gdfl_run::Progress::default()
    }
}

/// Ends a verification run's progress record (forced): `finished`, or
/// `stopped` when a stop was asked, or `refused` with `why` in a slot.
pub fn progress_end(progress: &crate::gdfl_run::ProgressFile, stopped: bool, why: Option<&str>) {
    use crate::gdfl_run::{ProgressRefusal, RunState, SLOT_WHY};
    progress.update(true, |record| {
        record.state = match (why, stopped) {
            (Some(_), _) => RunState::Refused,
            (None, true) => RunState::Stopped,
            (None, false) => RunState::Finished,
        }
        .code();
        if let Some(why) = why {
            let mut cut = why.len().min(SLOT_WHY);
            while !why.is_char_boundary(cut) {
                cut -= 1;
            }
            if record.refusals.len() >= crate::gdfl_run::PROGRESS_SLOTS {
                record.refusals.remove(0);
            }
            record.refusals.push(ProgressRefusal {
                day: 0,
                stage: "run".to_owned(),
                code: "refused".to_owned(),
                about: "verify".to_owned(),
                why: why.get(..cut).unwrap_or_default().to_owned(),
            });
            record.refusals_total = record.refusals_total.saturating_add(1);
        }
    });
}

// ------------------------------------------------------------- the merge --

/// What a merge did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeOutcome {
    /// Spools merged whole (and removed).
    pub spools_merged: u64,
    /// Generations committed.
    pub generations: u64,
    /// Records committed.
    pub records: u64,
    /// Days skipped because the merge log says they were committed.
    pub days_already: u64,
    /// Days whose intent the year index proved committed before a crash.
    pub days_recovered: u64,
    /// Spools refused by name now: path and words.
    pub refused: Vec<(PathBuf, String)>,
    /// Spools a merge refused earlier, not tried again without `--retry`.
    pub refused_earlier: Vec<PathBuf>,
    /// Torn tails and unsynced last records ignored: path, offset, bytes.
    pub torn: Vec<(PathBuf, u64, u64)>,
    /// Stray names in the spool tree.
    pub stray: u64,
    /// Spools still waiting when the merge ended (a stop, a refusal, a run
    /// still spooling).
    pub waiting: u64,
    /// Runs still spooling when the merge ended, each with its spools left
    /// waiting and its live lock's holder in words: never read, never
    /// committed, never removed while their writer lives (IA-9 repair 1);
    /// they merge after the run ends. In run order.
    pub live: Vec<(u64, u64, String)>,
    /// Whether a stop was asked.
    pub stopped: bool,
    /// The most runs' live locks this merge held open at once: never more
    /// than [`LIVE_LOCKS_AT_ONCE`], whatever the number of runs waiting
    /// (IA-9 repair 2).
    pub live_locks_peak: u64,
    /// Spool files that could not be removed: path and words.
    pub not_removed: Vec<(PathBuf, String)>,
    /// Writes beside the merge that failed, each by name, never dropped: a
    /// merge log's `refused` line, an alert, the last listing of the spool
    /// tree (so `waiting` is not known).
    pub faults: Vec<String>,
}

/// Why no merge ran.
#[derive(Debug)]
pub enum MergeError {
    /// `run_busy`: another merger holds `verify/merge.lock`.
    RunBusy {
        /// What the holder's lock says.
        holder: String,
    },
    /// The lock could not be opened.
    Lock(String),
    /// Listing the spools failed.
    Spool(SpoolError),
}

impl std::fmt::Display for MergeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RunBusy { holder } => write!(
                f,
                "run_busy: another verify-merge holds {} ({holder}); the spools wait for it or the next trigger",
                spool::MERGE_LOCK
            ),
            Self::Lock(why) => write!(f, "merge_lock_refused: {why}"),
            Self::Spool(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for MergeError {}

/// Merges every spool waiting under `root` into the verdict store, under
/// `verify/merge.lock`: one generation per (segment, timeframe, day) a spool
/// touches; then lists the spool tree once more and merges what arrived
/// meanwhile (D-4881, D-4921). `retry`: a spool refused earlier to try again.
///
/// # Errors
///
/// [`MergeError::RunBusy`] when another merger holds the lock (nothing is
/// read), [`MergeError::Lock`] and [`MergeError::Spool`].
pub fn merge_all(
    root: &Path,
    retry: Option<&Path>,
    stop: Option<&StopControl>,
) -> Result<MergeOutcome, MergeError> {
    merge_observed(root, retry, stop, None)
}

/// [`merge_all`], keeping `progress` (design §5.7) while it holds
/// `verify/merge.lock`, and only then: forced once the lock is taken (the
/// spools waiting in "the range's calendar days"), after each spool at most
/// once a second (spools done in "the day now's index"; spools read,
/// refused, generations and records in its counters), and forced at the end
/// BEFORE the lock is let go, so a merger that follows never has its record
/// written over by this one. A merger that finds the lock held writes none.
///
/// # Errors
///
/// As [`merge_all`].
pub fn merge_observed(
    root: &Path,
    retry: Option<&Path>,
    stop: Option<&StopControl>,
    progress: Option<&crate::gdfl_run::ProgressFile>,
) -> Result<MergeOutcome, MergeError> {
    merge_observed_on(root, retry, stop, progress, merge_workers())
}

/// The most merge workers one merge runs (RA-050).
pub const MAX_MERGE_WORKERS: usize = 8;

/// The workers a merge runs on when the caller names none: the host's
/// parallelism, at most [`MAX_MERGE_WORKERS`].
#[must_use]
pub fn merge_workers() -> usize {
    std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .clamp(1, MAX_MERGE_WORKERS)
}

/// [`merge_observed`] on `workers` threads (1 to [`MAX_MERGE_WORKERS`]; 0 is
/// read as 1). The spools are merged in groups of one (segment, timeframe,
/// month), each group whole and in run order by one worker, groups
/// side by side ([`merge_spools`]): the answer is the same at any width
/// (RA-050, IA-ZV-X23).
///
/// # Errors
///
/// As [`merge_all`].
pub fn merge_observed_on(
    root: &Path,
    retry: Option<&Path>,
    stop: Option<&StopControl>,
    progress: Option<&crate::gdfl_run::ProgressFile>,
    workers: usize,
) -> Result<MergeOutcome, MergeError> {
    let lock = merge_lock(root)?;
    merge_under(lock, root, (retry, stop, progress), workers)
}

/// Takes `verify/merge.lock`, never waiting: the one merger. A caller that
/// must number its run only once it is the merger (`cli verify-merge`, so a
/// `run_busy` loser takes no runs-ledger id) takes it first and hands it to
/// [`merge_under`].
///
/// # Errors
///
/// [`MergeError::RunBusy`] while another merger holds it,
/// [`MergeError::Lock`] when it would not open or lock.
pub fn merge_lock(root: &Path) -> Result<store::flock::Flock<std::fs::File>, MergeError> {
    let file = crate::gdfl_run::locks::open_lock_file(root, spool::MERGE_LOCK)
        .map_err(|why| MergeError::Lock(why.to_string()))?;
    let path = root.join(spool::MERGE_LOCK);
    match store::flock::Flock::try_lock(file, path.clone()) {
        Ok(lock) => Ok(lock),
        Err(std::fs::TryLockError::WouldBlock) => Err(MergeError::RunBusy {
            holder: store::flock::holder_note(&path),
        }),
        Err(std::fs::TryLockError::Error(why)) => Err(MergeError::Lock(why.to_string())),
    }
}

/// [`merge_observed_on`] under `lock`, the merge lock [`merge_lock`] took;
/// let go at the end, after the progress record's end is written.
///
/// # Errors
///
/// [`MergeError::Spool`] when the spool tree would not list.
pub fn merge_under(
    lock: store::flock::Flock<std::fs::File>,
    root: &Path,
    (retry, stop, progress): MergeAsk<'_>,
    workers: usize,
) -> Result<MergeOutcome, MergeError> {
    let workers = workers.clamp(1, MAX_MERGE_WORKERS);
    let mut outcome = MergeOutcome::default();
    let mut tried: HashSet<SpoolId> = HashSet::with_capacity(16);
    // TWICE: a spool that arrived while this merger ran is merged by it, or
    // by the next trigger (design §6.3).
    for pass in 0..2 {
        let waiting = match spool::waiting(root) {
            Ok(waiting) => waiting,
            Err(why) => {
                if let Some(progress) = progress {
                    progress_end(progress, false, Some(&why.to_string()));
                }
                return Err(MergeError::Spool(why));
            }
        };
        outcome.stray = waiting.stray.len() as u64;
        if let Some(progress) = progress {
            let more = u32::try_from(waiting.spools.len()).unwrap_or(u32::MAX);
            progress.update(pass == 0, |record| {
                record.day_total = if pass == 0 {
                    more
                } else {
                    record.day_index.saturating_add(
                        u32::try_from(
                            waiting
                                .spools
                                .iter()
                                .filter(|id| !tried.contains(id))
                                .count(),
                        )
                        .unwrap_or(u32::MAX),
                    )
                };
            });
        }
        let mut fresh: Vec<SpoolId> = waiting
            .spools
            .into_iter()
            .filter(|id| !tried.contains(id))
            .collect();
        // THE SECOND SORT (gate 11's ALLOW_SORT, L-IA-17): the listing is in
        // no promised order, so the spools are put in (run, segment,
        // timeframe, month) order once per listing, O(s log s) for the s
        // spools waiting (D-4921 bounds s).
        // A later run's record for a day and key then always commits after an
        // earlier run's, whatever the drive's directory order or the width.
        fresh.sort_unstable_by_key(|id| (id.run, id.segment, id.timeframe, id.month));
        // A RUN STILL SPOOLING IS NEVER MERGED (IA-9 repair 1): its spools
        // are left as they are, untried, so the next pass or merge takes them
        // once its writer is gone; every other run's live lock is held while
        // its spools merge, so no writer of it appends to a spool read here.
        // AT MOST `LIVE_LOCKS_AT_ONCE` RUNS' LOCKS ARE OPEN AT ONCE (IA-9
        // repair 2): the runs are merged in run order, a bounded slice of
        // runs at a time, and each slice's locks are let go before the next
        // slice's are taken, so a pass over any number of waiting runs holds
        // a bounded number of descriptors (300 runs under `ulimit -n 256`
        // used to fail every open past the limit). Run order across slices
        // keeps a later run's record committing after an earlier run's.
        let mut rest = fresh.as_slice();
        while !rest.is_empty() {
            let (now, later) = rest.split_at(runs_end(rest, LIVE_LOCKS_AT_ONCE));
            rest = later;
            let (taken, held) = not_live(root, now.to_vec(), &mut outcome.faults);
            outcome.live_locks_peak = outcome.live_locks_peak.max(held.len() as u64);
            tried.extend(taken.iter().copied());
            merge_spools(root, (retry, stop, progress), &taken, workers, &mut outcome);
            drop(held);
            if outcome.stopped {
                break;
            }
        }
        if outcome.stopped {
            break;
        }
    }
    match spool::waiting(root) {
        Ok(left) => {
            outcome.waiting = left.spools.len() as u64;
            outcome.live = live_runs(root, &left.spools, &mut outcome.faults);
        }
        // NOT KNOWN IS NOT ZERO: the count stays 0 only beside its fault.
        Err(why) => outcome.faults.push(format!(
            "the spool tree could not be listed after the merge, so the spools still waiting are not known: {why}"
        )),
    }
    if let Some(progress) = progress {
        progress.update(false, |record| merge_counts(record, &outcome));
        progress_end(progress, outcome.stopped, None);
    }
    drop(lock);
    let _ = telemetry::emit(
        &telemetry::Event::info(TARGET, "verify-merge ended")
            .with("spools", telemetry::Value::Uint(outcome.spools_merged))
            .with("generations", telemetry::Value::Uint(outcome.generations))
            .with("records", telemetry::Value::Uint(outcome.records))
            .with(
                "refused",
                telemetry::Value::Uint(outcome.refused.len() as u64),
            )
            .with("waiting", telemetry::Value::Uint(outcome.waiting))
            .with(
                "live_runs",
                telemetry::Value::Uint(outcome.live.len() as u64),
            )
            .with("stray", telemetry::Value::Uint(outcome.stray))
            .with(
                "live_locks_peak",
                telemetry::Value::Uint(outcome.live_locks_peak),
            ),
    );
    for (run, spools, holder) in &outcome.live {
        let _ = telemetry::emit(
            &telemetry::Event::info(TARGET, "verify-merge left a live run's spools")
                .with("run", telemetry::Value::Uint(*run))
                .with("spools", telemetry::Value::Uint(*spools))
                .with("holder", telemetry::Value::Str(holder)),
        );
    }
    Ok(outcome)
}

/// The most runs whose live locks one merge holds open at once (IA-9 repair
/// 2): a pass merges the waiting runs this many at a time, so the
/// descriptors it holds for live locks are bounded whatever the number of
/// runs waiting (one run per landing request under D-4921).
pub const LIVE_LOCKS_AT_ONCE: usize = 64;

/// Where the first `runs` runs of `spools`, sorted by run, end: the length
/// of the prefix holding them. One pass over that prefix, O(prefix).
fn runs_end(spools: &[SpoolId], runs: usize) -> usize {
    // AT LEAST ONE RUN, so a caller's loop always moves on.
    let runs = runs.max(1);
    let mut seen = 0_usize;
    let mut last = None;
    for (at, id) in spools.iter().enumerate() {
        if last != Some(id.run) {
            if seen == runs {
                return at;
            }
            seen += 1;
            last = Some(id.run);
        }
    }
    spools.len()
}

/// The spools of `spools` whose run is not spooling, in the order given,
/// and the live lock of each such run, held until the caller drops them: one
/// [`spool::live_probe`] per run (O(runs), each O(1)). A run still spooling,
/// or whose lock would not open, keeps its spools waiting; a lock that would
/// not open is named in `faults`, never read as free.
fn not_live(
    root: &Path,
    spools: Vec<SpoolId>,
    faults: &mut Vec<String>,
) -> (Vec<SpoolId>, Vec<store::flock::Flock<std::fs::File>>) {
    let mut free: HashMap<u64, bool> = HashMap::with_capacity(4);
    let mut held = Vec::with_capacity(4);
    let mut out = Vec::with_capacity(spools.len());
    for id in spools {
        let is_free =
            *free
                .entry(id.run)
                .or_insert_with(|| match spool::live_probe(root, id.run) {
                    Ok(spool::Live::Taken(lock)) => {
                        held.push(lock);
                        true
                    }
                    Ok(spool::Live::Spooling(_)) => false,
                    Err(why) => {
                        faults.push(format!(
                            "run {}'s live lock would not open, so its spools wait: {why}",
                            id.run
                        ));
                        false
                    }
                });
        if is_free {
            out.push(id);
        }
    }
    (out, held)
}

/// Each run of `spools` that is still spooling, with its spools waiting and
/// its holder in words, in run order: one probe per run, each lock let go at
/// once. A lock that would not open is named in `faults`.
fn live_runs(root: &Path, spools: &[SpoolId], faults: &mut Vec<String>) -> Vec<(u64, u64, String)> {
    // IN RUN ORDER BY THE MAP ITSELF, never a sort (gate 11).
    let mut count: std::collections::BTreeMap<u64, u64> = std::collections::BTreeMap::new();
    for id in spools {
        *count.entry(id.run).or_insert(0) += 1;
    }
    let mut out = Vec::new();
    for (run, waiting) in count {
        match spool::live_probe(root, run) {
            Ok(spool::Live::Taken(lock)) => drop(lock),
            Ok(spool::Live::Spooling(holder)) => out.push((run, waiting, holder)),
            Err(why) => faults.push(format!(
                "run {run}'s live lock would not open, so whether it is still spooling is not known: {why}"
            )),
        }
    }
    out
}

/// What a merge was handed besides its spools: `--retry`'s spool, the stop
/// file and the progress file.
pub type MergeAsk<'a> = (
    Option<&'a Path>,
    Option<&'a StopControl>,
    Option<&'a crate::gdfl_run::ProgressFile>,
);

/// Merges `spools`, already in (run, segment, timeframe, month) order, on up
/// to `workers` threads (RA-050). They are grouped by what they write,
/// (segment, timeframe, month): a group is merged whole and in order by one
/// worker, so the spools of one day always commit in the same order, and two
/// groups never write one day image (the verdict store's year-index commit
/// lock serialises the
/// commits of one year, D-4865). Each spool's outcome is folded in the
/// spools' order, so the outcome is the same at any width. A worker that
/// panicked leaves its spools on disk for the next merge, each named in
/// `faults`; a stop asked mid-group leaves the rest of every group waiting.
fn merge_spools(
    root: &Path,
    (retry, stop, progress): MergeAsk<'_>,
    spools: &[SpoolId],
    workers: usize,
    outcome: &mut MergeOutcome,
) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let mut groups: Vec<Vec<usize>> = Vec::with_capacity(spools.len());
    let mut group_of: HashMap<(Segment, Timeframe, YearMonth), usize> =
        HashMap::with_capacity(spools.len());
    for (at, id) in spools.iter().enumerate() {
        let slot = *group_of
            .entry((id.segment, id.timeframe, id.month))
            .or_insert(groups.len());
        if slot == groups.len() {
            groups.push(Vec::with_capacity(1));
        }
        if let Some(group) = groups.get_mut(slot) {
            group.push(at);
        }
    }
    let next = AtomicUsize::new(0);
    let running = std::sync::Mutex::new(outcome.clone());
    let job = || {
        let mut done: Vec<(usize, MergeOutcome)> = Vec::with_capacity(4);
        while let Some(group) = groups.get(next.fetch_add(1, Ordering::Relaxed)) {
            for &at in group {
                let Some(id) = spools.get(at) else { continue };
                let mut one = MergeOutcome::default();
                if stop.is_some_and(StopControl::requested) {
                    one.stopped = true;
                } else {
                    merge_one(root, id, retry, stop, &mut one);
                }
                if let Some(progress) = progress {
                    let mut total = running
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    total.absorb(one.clone());
                    progress.update(false, |record| merge_counts(record, &total));
                }
                done.push((at, one));
            }
        }
        done
    };
    let width = workers.clamp(1, groups.len().max(1));
    let mut slots: Vec<Option<MergeOutcome>> = vec![None; spools.len()];
    for (at, one) in on_workers("zerodha-merge", width, &job)
        .into_iter()
        .flatten()
        .flatten()
    {
        if let Some(slot) = slots.get_mut(at) {
            *slot = Some(one);
        }
    }
    for (slot, id) in slots.into_iter().zip(spools) {
        match slot {
            Some(one) => outcome.absorb(one),
            None => outcome.faults.push(format!(
                "{} was not merged: its merge worker panicked or never started; it waits for the next merge",
                id.path(root).display()
            )),
        }
    }
}

impl MergeOutcome {
    /// Spools still waiting that no live run holds: a stop's, a refusal's, or
    /// one that arrived after the last listing. A live run's spools wait by
    /// rule and are named in [`Self::live`].
    #[must_use]
    pub fn waiting_not_live(&self) -> u64 {
        self.waiting
            .saturating_sub(self.live.iter().map(|(_, spools, _)| *spools).sum())
    }

    /// Folds one spool's outcome into this one. `stray` and `waiting` are
    /// the listings' and are left as they are.
    fn absorb(&mut self, one: Self) {
        self.spools_merged += one.spools_merged;
        self.generations += one.generations;
        self.records += one.records;
        self.days_already += one.days_already;
        self.days_recovered += one.days_recovered;
        self.refused.extend(one.refused);
        self.refused_earlier.extend(one.refused_earlier);
        self.torn.extend(one.torn);
        self.stopped |= one.stopped;
        self.not_removed.extend(one.not_removed);
        self.faults.extend(one.faults);
    }
}

/// A merge's counters in its progress record (§35.10 "Progress"): spools
/// tried in the day now's index, spools merged in files read, refused in
/// files refused, generations in days imported, records in rows, days the
/// merge log had committed in days already done.
fn merge_counts(record: &mut crate::gdfl_run::Progress, outcome: &MergeOutcome) {
    let tried =
        outcome.spools_merged + outcome.refused.len() as u64 + outcome.refused_earlier.len() as u64;
    record.day_index = u32::try_from(tried).unwrap_or(u32::MAX);
    record.files = outcome.spools_merged;
    record.files_refused = (outcome.refused.len() + outcome.refused_earlier.len()) as u64;
    record.days_imported = outcome.generations;
    record.rows = outcome.records;
    record.days_skipped = outcome.days_already + outcome.days_recovered;
}

/// Merges one spool, recording what it did in `outcome`.
fn merge_one(
    root: &Path,
    id: &SpoolId,
    retry: Option<&Path>,
    stop: Option<&StopControl>,
    outcome: &mut MergeOutcome,
) {
    let path = id.path(root);
    let log_path = id.log_path(root);
    let mut log = match spool::read_log(&log_path) {
        Ok(log) => log,
        Err(why) => {
            refuse(root, id, &path, (None, true), &why.to_string(), outcome);
            return;
        }
    };
    if log.refused() && retry != Some(path.as_path()) {
        outcome.refused_earlier.push(path);
        return;
    }
    let read = match spool::read_spool(&path, id) {
        Ok(read) => read,
        Err(why) => {
            refuse(
                root,
                id,
                &path,
                (Some(&mut log), true),
                &why.to_string(),
                outcome,
            );
            return;
        }
    };
    note_tail(&path, &read, outcome);
    if let Err(why) = note_seen(&log_path, &mut log, read.records.len()) {
        refuse(root, id, &path, (None, false), &why, outcome);
        return;
    }
    let mode = read.header.map_or(Mode::ZerodhaMerge, |h| h.mode);
    let mut keyed: Vec<Keyed<'_>> = read
        .records
        .iter()
        .enumerate()
        .map(|(at, r)| {
            (
                (
                    verdict::exchange_code(r.exchange),
                    r.day,
                    r.key.text(),
                    r.receipt,
                ),
                at,
            )
        })
        .collect();
    // THE ONE SORT (gate 11's ALLOW_SORT, L-IA-17): the spool's records by
    // (exchange, day, key, receipt order), so each (exchange, day) is one
    // bucket and one key's records follow its receipts. O(n log n) per spool.
    keyed.sort_unstable();
    let mut whole = true;
    let mut start = 0;
    while start < keyed.len() {
        let Some(((exchange, day, _, _), _)) = keyed.get(start).copied() else {
            break;
        };
        let mut end = start;
        while keyed
            .get(end)
            .is_some_and(|((x, d, _, _), _)| *x == exchange && *d == day)
        {
            end += 1;
        }
        if stop.is_some_and(StopControl::requested) {
            outcome.stopped = true;
            whole = false;
            break;
        }
        let slice = keyed.get(start..end).unwrap_or_default();
        let bucket: Vec<&SpoolRecord> = slice
            .iter()
            .filter_map(|(_, at)| read.records.get(*at))
            .collect();
        // THE BUCKET'S LAST PLACE IN THE SPOOL: a commit that covered fewer
        // records did not see every record of the day.
        let reach = slice
            .iter()
            .map(|(_, at)| u32::try_from(*at + 1).unwrap_or(u32::MAX))
            .max()
            .unwrap_or(0);
        match merge_day(root, id, mode, (day, reach), &bucket, &mut log) {
            Ok(DayMerged::Already) => outcome.days_already += 1,
            Ok(DayMerged::Recovered) => outcome.days_recovered += 1,
            Ok(DayMerged::Committed { records }) => {
                outcome.generations += 1;
                outcome.records += records;
            }
            Err(why) => {
                // A COMMIT THAT FAILED (a stalled lock, a full disk) IS NOT DAMAGE:
                // named now, with an alert, and the spool waits for the next merge
                // with no refusal on its log.
                refuse(root, id, &path, (None, false), &why, outcome);
                return;
            }
        }
        start = end;
    }
    if whole {
        outcome.spools_merged += 1;
        remove_merged(root, id, outcome);
    }
}

/// What this merge read, on record before any day is committed (IA-9 repair
/// 1): a day committed now covers these `records` and no later one, so
/// records appended after this read are merged by a later merge, never
/// skipped as merged. One 16-byte write and sync when the spool grew since
/// the last `seen`; nothing otherwise.
fn note_seen(log_path: &Path, log: &mut spool::LogRead, records: usize) -> Result<(), String> {
    let seen = u32::try_from(records).unwrap_or(u32::MAX);
    if seen <= log.covered() {
        return Ok(());
    }
    spool::append_log(
        log_path,
        log,
        LogRecord {
            day: seen,
            generation: 0,
            state: LogState::Seen,
            exchange: 0,
        },
    )
    .map_err(|why| why.to_string())
}

/// A torn tail and an unsynced last record, ignored: named in the outcome and
/// in one audited error event for the spool (D-4881).
fn note_tail(path: &Path, read: &spool::SpoolRead, outcome: &mut MergeOutcome) {
    if let Some((at, bytes)) = read.torn {
        outcome.torn.push((path.to_path_buf(), at, bytes));
    }
    if let Some(at) = read.unsynced {
        outcome
            .torn
            .push((path.to_path_buf(), at, read.unsynced_bytes));
    }
    if read.torn.is_none() && read.unsynced.is_none() {
        return;
    }
    let shown = path.to_string_lossy();
    let _ = telemetry::emit(
        &telemetry::Event::error(TARGET, "spool tail ignored")
            .with("spool", telemetry::Value::Str(&shown))
            .with(
                "torn_at",
                telemetry::Value::Uint(read.torn.map_or(0, |t| t.0)),
            )
            .with(
                "torn_bytes",
                telemetry::Value::Uint(read.torn.map_or(0, |t| t.1)),
            )
            .with(
                "unsynced_at",
                telemetry::Value::Uint(read.unsynced.unwrap_or(0)),
            ),
    );
}

/// Removes a spool whose every day is committed, and its log, then the
/// emptied directories up to the run's: `remove_dir` refuses a directory that
/// holds anything, a stray name included, which is then left as it is.
fn remove_merged(root: &Path, id: &SpoolId, outcome: &mut MergeOutcome) {
    let path = id.path(root);
    // THE LOG GOES FIRST, AND THE SPOOL ONLY AFTER IT (IA-9 repair 1): a log
    // left without its spool would answer for the next spool born at that
    // name; a spool left without its log is merged again whole, a duplicate
    // generation of the same records, never a loss. A log that will not go
    // keeps its spool, both named.
    for file in [id.log_path(root), path.clone()] {
        match std::fs::remove_file(&file) {
            Ok(()) => {}
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            Err(why) => {
                outcome.not_removed.push((file, why.to_string()));
                return;
            }
        }
    }
    let run_dir = root.join(spool::SPOOL_DIR).join(id.run.to_string());
    let mut dir = path.parent().map(Path::to_path_buf);
    while let Some(at) = dir {
        if !at.starts_with(&run_dir) || std::fs::remove_dir(&at).is_err() {
            break;
        }
        dir = at.parent().map(Path::to_path_buf);
    }
}

/// Refuses a spool by name: an error event and an alert (D-4883). A damaged
/// spool or log (`damaged`) is kind 10 `spool_damaged`, and a log that can
/// take a line records the refusal once, so no later merge tries it until
/// `--retry`; a commit the verdict store refused is kind 11
/// `verdict_refused`, and the spool waits for the next merge.
fn refuse(
    root: &Path,
    id: &SpoolId,
    path: &Path,
    (log, damaged): (Option<&mut spool::LogRead>, bool),
    why: &str,
    outcome: &mut MergeOutcome,
) {
    if let Some(log) = log
        && !log.refused()
        && let Err(failed) = spool::append_log(
            &id.log_path(root),
            log,
            LogRecord {
                day: 0,
                generation: 0,
                state: LogState::Refused,
                exchange: 0,
            },
        )
    {
        // NAMED, NEVER DROPPED: without the line the next merge tries the
        // spool again, which is safe, but the refusal must not be silent.
        outcome.faults.push(format!(
            "{}: the merge log's refused line was not written: {failed}",
            id.log_path(root).display()
        ));
    }
    let shown = path.to_string_lossy();
    // KEYED PER RUN, NEVER PER PATH (D-4883, §7.1 row 10): every spool file
    // of one run that a merge refuses is one alert, its first path in the
    // text; each one is still named in the merge log, the outcome and an
    // error event.
    let kind = if damaged {
        SPOOL_DAMAGED
    } else {
        VERDICT_REFUSED
    };
    let key = format!(
        "{}:{}",
        if damaged {
            "spool_damaged"
        } else {
            "verdict_refused"
        },
        id.run
    );
    let _ = telemetry::emit(
        &telemetry::Event::error(TARGET, "spool refused")
            .with("spool", telemetry::Value::Str(&shown))
            .with("run", telemetry::Value::Uint(id.run))
            .with("why", telemetry::Value::Str(why)),
    );
    if let Err(failed) = crate::alerts::put(
        root,
        &crate::alerts::NewAlert {
            kind,
            key: &key,
            day: 0,
            run: id.run,
            text: why,
        },
    ) {
        let words = format!("{key}: the alert was not written: {failed}");
        let _ = telemetry::emit(
            &telemetry::Event::error(TARGET, "alert not written")
                .with("key", telemetry::Value::Str(&key))
                .with("why", telemetry::Value::Str(&words)),
        );
        outcome.faults.push(words);
    }
    outcome.refused.push((path.to_path_buf(), why.to_owned()));
}

/// The alert kind of a refused spool (`alerts::KINDS`, 10).
const SPOOL_DAMAGED: u8 = 10;

/// The alert kind of a merge whose commit the verdict store refused
/// (`alerts::KINDS`, 11).
const VERDICT_REFUSED: u8 = 11;

/// What one day's merge did.
enum DayMerged {
    /// The log says it was committed.
    Already,
    /// The log had only an intent and the year index proves the commit.
    Recovered,
    /// A generation was committed.
    Committed { records: u64 },
}

/// The verdict-store day of a pull `Day`.
fn store_day(day: Day) -> Option<store::provenance::Day> {
    store::provenance::Day::new(day.year(), day.month(), day.day())
}

/// The generation the year index commits for `day`, 0 for none.
fn committed_generation(
    series: &verdict::Series<'_>,
    day: store::provenance::Day,
) -> Result<u32, String> {
    let mut reader = verdict::Reader::new();
    let year = reader
        .year(series, day.year())
        .map_err(|why| why.to_string())?;
    Ok(year.map_or(0, |index| index.entry(day).generation))
}

/// Whether a generation after `base` up to `now` was written by `run`: the
/// image headers walked back by `previous`, at most [`MAX_WALK_BACK`].
fn written_by(
    series: &verdict::Series<'_>,
    day: store::provenance::Day,
    (base, now): (u32, u32),
    run: u64,
) -> bool {
    let mut at = now;
    let mut steps = 0;
    while at > base && steps < MAX_WALK_BACK {
        let Ok(image) = verdict::read_image(&series.image_path(day, at)) else {
            return false;
        };
        if image.header.run == run {
            return true;
        }
        at = image.header.previous;
        steps += 1;
    }
    false
}

/// One entry per key of a sorted day bucket, and the day's worst code: a
/// key's later receipts that differ from its first are the vendor's
/// revision (VERIFY-13), named on the one record kept.
fn entries_of(bucket: &[&SpoolRecord]) -> (Vec<Entry>, Code) {
    let mut entries: Vec<Entry> = Vec::with_capacity(bucket.len());
    let mut at_key: Option<(&str, usize, u32)> = None;
    let mut worst = Code::Match;
    for record in bucket {
        match at_key {
            Some((text, index, first)) if text == record.key.text() => {
                let Some(base) = entries.get_mut(index) else {
                    continue;
                };
                let differs = base.facts.code != record.facts.code
                    || base.facts.value_diff != record.facts.value_diff
                    || base.facts.volume_diff != record.facts.volume_diff
                    || base.facts.oi_diff != record.facts.oi_diff
                    || base.facts.missing != record.facts.missing
                    || base.facts.extra != record.facts.extra;
                if differs && base.facts.flags & flags::VENDOR_REVISED == 0 {
                    base.facts.flags |= flags::VENDOR_REVISED;
                    base.details.push(Detail {
                        kind: words::DETAIL_RECEIPTS,
                        payload: payload(&[&first.to_le_bytes(), &record.receipt.to_le_bytes()]),
                    });
                }
            }
            _ => {
                worst = worst.worse(record.facts.code);
                entries.push(Entry {
                    key: record.key.clone(),
                    facts: record.facts,
                    details: record.details.clone(),
                });
                at_key = Some((record.key.text(), entries.len() - 1, record.receipt));
            }
        }
    }
    (entries, worst)
}

/// Merges one (exchange, day) bucket into one generation (D-4881's merge
/// log, O(1) per day besides the commit). Proved by bench C-ZV-02
/// (`crates/pull/benches/ratio.rs`, `zerodha_merge_per_record_is_flat`) and
/// C-ZV-03 (`zerodha_merge_log_step_is_flat`), invariant rows C-ZV-02 and
/// C-ZV-03.
fn merge_day(
    root: &Path,
    id: &SpoolId,
    mode: Mode,
    (day, reach): (Day, u32),
    bucket: &[&SpoolRecord],
    log: &mut spool::LogRead,
) -> Result<DayMerged, String> {
    let exchange = bucket
        .first()
        .map_or(Exchange::Nse, |record| record.exchange);
    let code = verdict::exchange_code(exchange);
    let number = day.days_from_epoch();
    let series = verdict::Series {
        root,
        feed: Vendor::Zerodha,
        exchange,
        segment: id.segment,
        timeframe: id.timeframe,
    };
    let at =
        store_day(day).ok_or_else(|| format!("day {day} is outside the verdict store's range"))?;
    let log_path = id.log_path(root);
    // A COMMIT ANSWERS ONLY FOR THE RECORDS IT COVERED (IA-9 repair 1): a
    // record the spool gained after that merge read it (a run resumed under
    // its id after a stopped or refused merge) re-merges the whole day into
    // a new generation, never `Already`.
    let covers = log
        .covered_of(code, number)
        .is_some_and(|covered| reach <= covered);
    match log.last_of(code, number) {
        Some(LogRecord {
            state: LogState::Committed,
            ..
        }) if covers => return Ok(DayMerged::Already),
        Some(LogRecord {
            state: LogState::Intent,
            generation: base,
            ..
        }) if covers => {
            let now = committed_generation(&series, at)?;
            if now != base && written_by(&series, at, (base, now), id.run) {
                spool::append_log(
                    &log_path,
                    log,
                    LogRecord {
                        day: number,
                        generation: now,
                        state: LogState::Committed,
                        exchange: code,
                    },
                )
                .map_err(|why| why.to_string())?;
                return Ok(DayMerged::Recovered);
            }
        }
        _ => {}
    }
    let (entries, worst) = entries_of(bucket);
    let base = committed_generation(&series, at)?;
    spool::append_log(
        &log_path,
        log,
        LogRecord {
            day: number,
            generation: base,
            state: LogState::Intent,
            exchange: code,
        },
    )
    .map_err(|why| why.to_string())?;
    let records = entries.len() as u64;
    let generation = Generation {
        day: at,
        run: id.run,
        written_at: unix_now(),
        verifier: VERIFIER,
        mode,
        rule_set: 0,
        origin: [0; 16],
        day_block: DayBlock::of(worst),
        day_details: Vec::new(),
        entries,
        carry: Carry::Others,
    };
    let committed = verdict::commit(&series, &generation).map_err(|why| why.to_string())?;
    spool::append_log(
        &log_path,
        log,
        LogRecord {
            day: number,
            generation: committed.generation,
            state: LogState::Committed,
            exchange: code,
        },
    )
    .map_err(|why| why.to_string())?;
    Ok(DayMerged::Committed { records })
}

/// Seconds since 1970.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
#[path = "zerodha_verify_tests.rs"]
mod tests;

//! The app measures itself, all the time (R3-14; RA-014, NEW-01, NEW-02).
//!
//! # What was missing
//!
//! The server kept a count and the slowest time per `/db` route per minute
//! (`logs::AccessWindows`, WP-33c) and nothing else. "What was p99 of this
//! route in the last hour" had no answer: every p50 and p99 in a report was a
//! one-off script run against an idle app, and a route that slowed under an
//! import, or next week, was seen by nobody.
//!
//! # What is kept
//!
//! Every route the server answers (its matched path, so `/bars.json` is one
//! row whatever its query; every front-end file is the one row
//! [`FRONT_END`]) has a fixed set of [`BUCKETS`] time buckets and five
//! outcome counts — answered, refused, busy, failed, cancelled — kept in
//! memory. A request costs ONE bucket add, one outcome add and two
//! `fetch_max`es on atomics, after one read-locked hash probe for its row:
//! O(1), and measured (`crates/api/benches/ratio.rs`, the `speed add` rows,
//! n = 1,000,000).
//!
//! Once a minute a background thread freezes every row (a copy of its
//! buckets) and writes ONE line per route that was asked anything in that
//! minute: `api.speed minute ended`, which the audit stream keeps (the word
//! `ended`, `telemetry::is_audited`). The last [`HOUR_ROLLS`] frozen copies
//! are kept, so the last hour is the newest copy less the oldest one: a
//! subtraction, never a re-scan of requests. The minute open at shutdown is
//! written then, marked `partial` ([`flush_at_shutdown`]), after a graceful
//! stop that `SIGTERM` now asks too ([`stop_asked`]) and that waits for open
//! requests at most [`DRAIN_GRACE`] ([`drain_bound`]).
//!
//! Import and checking stages keep the same buckets under `stage:<name>`
//! ([`stage_start`]): today every Zerodha member's landing
//! (`stage:zerodha landing`) and the vendor scrub (`stage:vendor scrub`).
//! The GDFL import runs in `cli`, outside this process, and has no row here
//! until the one-press chain runs it inside the app.
//!
//! # Bounds, said out loud (for `docs/06-limits.md`)
//!
//! * Resolution: [`SUB_BUCKETS`] buckets an octave, so a percentile is the
//!   TOP of its bucket, at most one eighth (12.5%) above the true time, and
//!   never above the largest time seen. Times at or past 2^36 µs (19 hours)
//!   share the top bucket.
//! * Rows: at most [`ROUTE_CAP`]. The route table is fixed at start (under
//!   a hundred paths) so the cap is never reached by this build; a row past
//!   it is counted in [`OVER_CAP`] and the report says how many, never
//!   dropped silently.
//! * Memory: a row is [`BUCKETS`] u64 counters plus [`HOUR_ROLLS`] + 1 frozen
//!   copies of [`BUCKETS`] u32 counters, about 70 KB, allocated when the row
//!   is first asked; [`ROUTE_CAP`] rows is the 18 MB ceiling.
//! * A frozen copy is taken bucket by bucket while requests keep landing, so
//!   a request finishing during the freeze may be counted in either minute
//!   (never in both, never in neither). A u32 frozen counter wraps; the
//!   wrapping difference is exact while one hour holds under 2^32 requests a
//!   route.
//! * `/speed.json` costs O(rows x buckets): at most 256 x 272 counter reads,
//!   three times. It does not grow with the requests served or the store.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, RwLock};

/// Bits of the sub-bucket index inside one octave.
const SUB_BITS: u32 = 3;

/// Buckets an octave: 8, so a bucket is at most an eighth of its time wide.
pub const SUB_BUCKETS: usize = 1 << SUB_BITS;

/// The highest octave kept apart; every time at or past `2^(TOP_OCTAVE + 1)`
/// µs shares the last bucket.
const TOP_OCTAVE: u32 = 35;

/// The fixed number of time buckets a row keeps: the times 0..8 µs exactly,
/// then eight a power of two up to 2^36 µs.
pub const BUCKETS: usize = SUB_BUCKETS + (TOP_OCTAVE - SUB_BITS + 1) as usize * SUB_BUCKETS;

/// How many minutes back the last hour reaches.
pub const HOUR_ROLLS: usize = 60;

/// The most rows the table holds before it counts the rest in [`OVER_CAP`].
pub const ROUTE_CAP: usize = 256;

/// The row every request no route matched is counted under: the front end's
/// files, served by the fallback.
pub const FRONT_END: &str = "(front end)";

/// The row a request past [`ROUTE_CAP`] is counted under.
pub const OVER_CAP: &str = "(over the route cap)";

/// The minute the background thread freezes the table on, ms.
pub const MINUTE_MS: u64 = 60_000;

/// The bucket a time of `micros` lands in.
#[must_use]
pub fn bucket_of(micros: u64) -> usize {
    if micros < SUB_BUCKETS as u64 {
        return usize::try_from(micros).unwrap_or(0);
    }
    let octave = micros.ilog2();
    if octave > TOP_OCTAVE {
        return BUCKETS - 1;
    }
    let shift = octave - SUB_BITS;
    let sub = usize::try_from((micros >> shift) & (SUB_BUCKETS as u64 - 1)).unwrap_or(0);
    SUB_BUCKETS + (shift as usize) * SUB_BUCKETS + sub
}

/// The smallest time bucket `bucket` holds, µs.
#[must_use]
pub fn bucket_low(bucket: usize) -> u64 {
    if bucket < SUB_BUCKETS {
        return bucket as u64;
    }
    let k = bucket.min(BUCKETS - 1) - SUB_BUCKETS;
    let shift = u32::try_from(k / SUB_BUCKETS).unwrap_or(0);
    let sub = (k % SUB_BUCKETS) as u64;
    (SUB_BUCKETS as u64 + sub) << shift
}

/// The largest time bucket `bucket` holds, µs. The last bucket has no top:
/// [`u64::MAX`], which every reader clamps to the largest time seen.
#[must_use]
pub fn bucket_top(bucket: usize) -> u64 {
    if bucket < SUB_BUCKETS {
        return bucket as u64;
    }
    if bucket >= BUCKETS - 1 {
        return u64::MAX;
    }
    let k = bucket - SUB_BUCKETS;
    let shift = u32::try_from(k / SUB_BUCKETS).unwrap_or(0);
    let sub = (k % SUB_BUCKETS) as u64;
    let low = (SUB_BUCKETS as u64 + sub) << shift;
    low + (1_u64 << shift) - 1
}

/// What became of one request (or one stage).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Answered: any status below 400.
    Answered,
    /// Refused as asked: a 4xx other than 429.
    Refused,
    /// Busy: a 429, or a 503 that carries `Retry-After` (admission control).
    Busy,
    /// Failed: any other 5xx.
    Failed,
    /// Cancelled: the client went away before the answer was made.
    Cancelled,
}

/// How many outcomes a row counts.
pub const OUTCOMES: usize = 5;

impl Outcome {
    /// The outcome of an answer with `status`, which carried `Retry-After`
    /// when `retry_after`.
    #[must_use]
    pub fn of(status: u16, retry_after: bool) -> Self {
        match status {
            429 => Self::Busy,
            503 if retry_after => Self::Busy,
            500..=599 => Self::Failed,
            400..=499 => Self::Refused,
            _ => Self::Answered,
        }
    }

    /// Its slot in a row's outcome counts.
    const fn slot(self) -> usize {
        match self {
            Self::Answered => 0,
            Self::Refused => 1,
            Self::Busy => 2,
            Self::Failed => 3,
            Self::Cancelled => 4,
        }
    }
}

/// What a row measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// One HTTP route (its matched path) or the front end.
    Route,
    /// One import or checking stage.
    Stage,
}

/// One row's live counters: the only thing a request touches.
struct Live {
    buckets: Box<[AtomicU64]>,
    outcomes: [AtomicU64; OUTCOMES],
    max: AtomicU64,
    minute_max: AtomicU64,
    /// The first time this row ever took, plus one; zero until it has one.
    first_plus_one: AtomicU64,
    /// Requests started on this row and not yet counted: one add at
    /// [`Pending::start`], one subtract when it is counted (R3-14). Read only
    /// at a stop, to say how many requests the drain bound cut and where.
    open: AtomicU64,
}

impl Live {
    fn new() -> Self {
        Self {
            buckets: (0..BUCKETS).map(|_| AtomicU64::new(0)).collect(),
            outcomes: std::array::from_fn(|_| AtomicU64::new(0)),
            max: AtomicU64::new(0),
            minute_max: AtomicU64::new(0),
            first_plus_one: AtomicU64::new(0),
            open: AtomicU64::new(0),
        }
    }
}

/// A frozen copy of a row at one instant: its buckets and outcomes, low 32
/// bits (see the module's wrap rule), and the largest time of the minute that
/// ended at it.
#[derive(Clone)]
struct Frozen {
    at_ms: u64,
    buckets: Box<[u32]>,
    outcomes: [u32; OUTCOMES],
    minute_max: u64,
}

impl Frozen {
    fn zero(at_ms: u64) -> Self {
        Self {
            at_ms,
            buckets: vec![0; BUCKETS].into_boxed_slice(),
            outcomes: [0; OUTCOMES],
            minute_max: 0,
        }
    }
}

/// One row: its name, its live counters and its last hour of frozen copies.
struct Row {
    name: Box<str>,
    kind: Kind,
    live: Live,
    /// Oldest first; at most [`HOUR_ROLLS`] + 1.
    frozen: Mutex<VecDeque<Frozen>>,
}

impl Row {
    fn new(name: &str, kind: Kind, at_ms: u64) -> Self {
        let mut frozen = VecDeque::with_capacity(HOUR_ROLLS + 1);
        frozen.push_back(Frozen::zero(at_ms));
        Self {
            name: name.into(),
            kind,
            live: Live::new(),
            frozen: Mutex::new(frozen),
        }
    }

    /// Counts one request: one bucket add, one outcome add, and the two
    /// maxima and the first time only when they change.
    ///
    /// A maximum is READ first and written only when this time beats it, and
    /// the first time is written once: sixty clients on one route then share
    /// the cache lines of those three counters for reading, not for a write of
    /// a value that did not change. The two adds are the only writes every
    /// request makes.
    fn add(&self, micros: u64, outcome: Outcome) {
        if let Some(bucket) = self.live.buckets.get(bucket_of(micros)) {
            bucket.fetch_add(1, Ordering::Relaxed);
        }
        if let Some(count) = self.live.outcomes.get(outcome.slot()) {
            count.fetch_add(1, Ordering::Relaxed);
        }
        if micros > self.live.max.load(Ordering::Relaxed) {
            self.live.max.fetch_max(micros, Ordering::Relaxed);
        }
        if micros > self.live.minute_max.load(Ordering::Relaxed) {
            self.live.minute_max.fetch_max(micros, Ordering::Relaxed);
        }
        if self.live.first_plus_one.load(Ordering::Relaxed) == 0 {
            let _first_already_set = self.live.first_plus_one.compare_exchange(
                0,
                micros.saturating_add(1),
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
        }
    }

    /// The live counters, frozen now, without touching the minute's maximum.
    fn freeze(&self, at_ms: u64, minute_max: u64) -> Frozen {
        Frozen {
            at_ms,
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the low 32 bits ARE the frozen copy: differences are taken wrapping"
            )]
            buckets: self
                .live
                .buckets
                .iter()
                .map(|b| b.load(Ordering::Relaxed) as u32)
                .collect(),
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the low 32 bits ARE the frozen copy: differences are taken wrapping"
            )]
            outcomes: std::array::from_fn(|i| {
                self.live
                    .outcomes
                    .get(i)
                    .map_or(0, |c| c.load(Ordering::Relaxed) as u32)
            }),
            minute_max,
        }
    }

    /// The live counters, whole.
    fn since_start(&self) -> Figures {
        let buckets: Vec<u64> = self
            .live
            .buckets
            .iter()
            .map(|b| b.load(Ordering::Relaxed))
            .collect();
        let outcomes: [u64; OUTCOMES] = std::array::from_fn(|i| {
            self.live
                .outcomes
                .get(i)
                .map_or(0, |c| c.load(Ordering::Relaxed))
        });
        Figures::of(&buckets, outcomes, self.live.max.load(Ordering::Relaxed))
    }
}

/// The difference of two frozen copies, as wide counts.
fn between(old: &Frozen, new: &Frozen, max: u64) -> Figures {
    let buckets: Vec<u64> = new
        .buckets
        .iter()
        .zip(old.buckets.iter())
        .map(|(n, o)| u64::from(n.wrapping_sub(*o)))
        .collect();
    let outcomes: [u64; OUTCOMES] =
        std::array::from_fn(|i| match (new.outcomes.get(i), old.outcomes.get(i)) {
            (Some(n), Some(o)) => u64::from(n.wrapping_sub(*o)),
            _ => 0,
        });
    Figures::of(&buckets, outcomes, max)
}

/// One window's figures: how many, how they ended, and the times.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct Figures {
    /// Requests in the window, every outcome.
    pub n: u64,
    /// Answered (below 400).
    pub answered: u64,
    /// Refused as asked (4xx but 429).
    pub refused: u64,
    /// Busy (429, or 503 with `Retry-After`).
    pub busy: u64,
    /// Failed (other 5xx).
    pub failed: u64,
    /// Cancelled (the client left first).
    pub cancelled: u64,
    /// The median time, µs: the top of its bucket, never above `max_micros`.
    pub p50_micros: u64,
    /// The 99th percentile, µs, read the same way.
    pub p99_micros: u64,
    /// The largest time, µs.
    pub max_micros: u64,
}

impl Figures {
    /// The figures of `buckets` and `outcomes` whose largest time was `max`.
    ///
    /// THE MAX IS NEVER BELOW THE SLOWEST BUCKET HELD. A request finishing
    /// while its minute is frozen can land its bucket in one minute and its
    /// maximum in the next; read raw, that minute's max would sit under its
    /// own slowest request and clamp its p99 below the bucket the request is
    /// in. The max is therefore raised to the bottom of the highest bucket
    /// that holds a count, which every request in it took at least, so
    /// `p50 <= p99 <= max` holds for every window, and each percentile stays
    /// inside the bucket its rank falls in.
    fn of(buckets: &[u64], outcomes: [u64; OUTCOMES], max: u64) -> Self {
        let n: u64 = buckets.iter().copied().fold(0, u64::saturating_add);
        // The slowest bucket holding a count: a walk down the fixed table of
        // `BUCKETS`, never longer whatever was counted.
        let max = (0..BUCKETS)
            .rev()
            .find(|i| buckets.get(*i).is_some_and(|count| *count > 0))
            .map_or(max, |slowest| max.max(bucket_low(slowest)));
        let [answered, refused, busy, failed, cancelled] = outcomes;
        Self {
            n,
            answered,
            refused,
            busy,
            failed,
            cancelled,
            p50_micros: percentile(buckets, n, 50, max),
            p99_micros: percentile(buckets, n, 99, max),
            max_micros: if n == 0 { 0 } else { max },
        }
    }
}

/// The `pct`th percentile of `n` times in `buckets`: the top of the bucket the
/// `ceil(n x pct / 100)`th time falls in, clamped to `max`. Zero when empty.
#[must_use]
pub fn percentile(buckets: &[u64], n: u64, pct: u64, max: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let rank = n.saturating_mul(pct).div_ceil(100).max(1);
    let mut seen = 0_u64;
    for (bucket, count) in buckets.iter().enumerate() {
        seen = seen.saturating_add(*count);
        if seen >= rank {
            return bucket_top(bucket).min(max);
        }
    }
    max
}

/// One line the minute freeze writes: a row that was asked something.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinuteLine {
    /// The row's name: a matched path, [`FRONT_END`], or `stage:<name>`.
    pub route: String,
    /// What it measures.
    pub kind: Kind,
    /// The minute's figures.
    pub figures: Figures,
    /// How long the window was, ms (a minute, or less for a partial one).
    pub window_ms: u64,
    /// Written at shutdown, before its minute was over.
    pub partial: bool,
}

impl MinuteLine {
    /// Writes this line to the process log, where the audit stream keeps it.
    #[must_use]
    pub fn emit(&self) -> telemetry::Emitted {
        let f = &self.figures;
        telemetry::emit(
            &telemetry::Event::info("api.speed", "minute ended")
                .with("route", telemetry::Value::Str(&self.route))
                .with("n", telemetry::Value::Uint(f.n))
                .with("answered", telemetry::Value::Uint(f.answered))
                .with("refused", telemetry::Value::Uint(f.refused))
                .with("busy", telemetry::Value::Uint(f.busy))
                .with("failed", telemetry::Value::Uint(f.failed))
                .with("cancelled", telemetry::Value::Uint(f.cancelled))
                .with("p50_micros", telemetry::Value::Uint(f.p50_micros))
                .with("p99_micros", telemetry::Value::Uint(f.p99_micros))
                .with("max_micros", telemetry::Value::Uint(f.max_micros))
                .with("window_ms", telemetry::Value::Uint(self.window_ms))
                .with("partial", telemetry::Value::Bool(self.partial)),
        )
    }
}

/// One row of the report `/speed.json` serves.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RowReport {
    /// The row's name.
    pub route: String,
    /// What it measures.
    pub kind: Kind,
    /// Everything since the process started.
    pub since_start: Figures,
    /// The last whole minute the background thread froze.
    pub last_minute: Figures,
    /// The last hour of whole minutes (fewer, the first hour after a start).
    pub last_hour: Figures,
    /// The minutes `last_hour` covers.
    pub last_hour_minutes: u64,
    /// The first request this row took after the start, µs: the cold one.
    pub first_after_start_micros: Option<u64>,
}

/// The whole report.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Report {
    /// When the table started, ms since the epoch.
    pub started_ms: u64,
    /// When this report was made.
    pub now_ms: u64,
    /// Every row: routes, then stages, each in the order first asked.
    pub rows: Vec<RowReport>,
    /// Requests counted under [`OVER_CAP`] because the table was full.
    pub over_cap: u64,
    /// How to read a figure.
    pub resolution: &'static str,
}

/// The resolution sentence every report carries.
pub const RESOLUTION: &str = "Every time is counted in one of a fixed set of buckets, eight a \
power of two. A p50 or p99 is the top of the bucket it falls in, at most 12.5% above the true \
time and never above the max. last_minute and last_hour are whole minutes the server froze; \
since_start counts every request since this process started. Busy is a 429 or a 503 with \
Retry-After; cancelled is a request whose client left before the answer.";

/// The rows: found by name, and kept in the order first asked, so a report
/// walks them without sorting.
struct Rows {
    by_name: HashMap<Box<str>, Arc<Row>>,
    in_order: Vec<Arc<Row>>,
}

/// The table: every row, found by name.
pub struct Speed {
    started_ms: u64,
    cap: usize,
    rows: RwLock<Rows>,
    over_cap: Arc<Row>,
    /// ONE FREEZE AT A TIME. The minute thread and the shutdown flush can
    /// both roll; two rolls interleaved row by row could push a later copy
    /// before an earlier one, and the next difference would then be taken
    /// backwards and wrap to four billion. A roll holds this for its whole
    /// walk; a request never takes it.
    rolling: Mutex<()>,
}

impl std::fmt::Debug for Speed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Speed")
            .field("started_ms", &self.started_ms)
            .field("cap", &self.cap)
            .finish_non_exhaustive()
    }
}

/// A row, held by a request between its start and its answer.
#[derive(Clone)]
pub struct Handle(Arc<Row>);

impl std::fmt::Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Handle").field(&self.0.name).finish()
    }
}

impl Handle {
    /// Counts one request on this row.
    pub fn add(&self, micros: u64, outcome: Outcome) {
        self.0.add(micros, outcome);
    }
}

impl Speed {
    /// An empty table started at `started_ms`, holding at most `cap` rows.
    #[must_use]
    pub fn new(started_ms: u64, cap: usize) -> Self {
        Self {
            started_ms,
            cap,
            // Pre-sized to the cap: a row is never added past it.
            rows: RwLock::new(Rows {
                by_name: HashMap::with_capacity(cap),
                in_order: Vec::with_capacity(cap),
            }),
            over_cap: Arc::new(Row::new(OVER_CAP, Kind::Route, started_ms)),
            rolling: Mutex::new(()),
        }
    }

    /// The row named `name`, made on first use at `now_ms`. One read-locked
    /// hash probe; the write lock is taken once per row, the first time.
    pub fn row(&self, name: &str, kind: Kind, now_ms: u64) -> Handle {
        if let Some(row) = self
            .rows
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .by_name
            .get(name)
        {
            return Handle(Arc::clone(row));
        }
        let mut rows = self.rows.write().unwrap_or_else(PoisonError::into_inner);
        if let Some(row) = rows.by_name.get(name) {
            return Handle(Arc::clone(row));
        }
        if rows.by_name.len() >= self.cap {
            return Handle(Arc::clone(&self.over_cap));
        }
        let row = Arc::new(Row::new(name, kind, now_ms));
        rows.by_name.insert(name.into(), Arc::clone(&row));
        rows.in_order.push(Arc::clone(&row));
        Handle(row)
    }

    /// Counts one request on `name`.
    pub fn record(&self, name: &str, kind: Kind, micros: u64, outcome: Outcome, now_ms: u64) {
        self.row(name, kind, now_ms).add(micros, outcome);
    }

    /// Every row: the routes in the order first asked, then the stages in
    /// the same order, then the over-cap row when it holds anything. Two
    /// walks of at most `cap` rows, and no sort.
    fn all(&self) -> Vec<Arc<Row>> {
        let rows = self.rows.read().unwrap_or_else(PoisonError::into_inner);
        let mut out: Vec<Arc<Row>> = Vec::with_capacity(rows.in_order.len().saturating_add(1));
        for kind in [Kind::Route, Kind::Stage] {
            out.extend(rows.in_order.iter().filter(|r| r.kind == kind).cloned());
        }
        drop(rows);
        if self
            .over_cap
            .live
            .outcomes
            .iter()
            .any(|c| c.load(Ordering::Relaxed) > 0)
        {
            out.push(Arc::clone(&self.over_cap));
        }
        out
    }

    /// Freezes every row at `now_ms` and returns one line for each row asked
    /// anything since its last freeze. `partial` marks the freeze at
    /// shutdown, whose minute was not over.
    pub fn roll(&self, now_ms: u64, partial: bool) -> Vec<MinuteLine> {
        let _one_roll = self.rolling.lock().unwrap_or_else(PoisonError::into_inner);
        let mut lines = Vec::new();
        for row in self.all() {
            let minute_max = row.live.minute_max.swap(0, Ordering::Relaxed);
            let now = row.freeze(now_ms, minute_max);
            let mut frozen = row.frozen.lock().unwrap_or_else(PoisonError::into_inner);
            let figures = frozen.back().map(|last| {
                (
                    between(last, &now, minute_max),
                    now_ms.saturating_sub(last.at_ms),
                )
            });
            if let Some((figures, window_ms)) = figures
                && figures.n > 0
            {
                lines.push(MinuteLine {
                    route: row.name.to_string(),
                    kind: row.kind,
                    figures,
                    window_ms,
                    partial,
                });
            }
            frozen.push_back(now);
            while frozen.len() > HOUR_ROLLS + 1 {
                let _oldest = frozen.pop_front();
            }
        }
        lines
    }

    /// Every row with a request still open, and how many: what a stop cut
    /// short, or what outlived the runtime (R3-14). One walk of at most the
    /// cap's rows, at a stop only; never on a request.
    #[must_use]
    pub fn open_now(&self) -> Vec<(String, u64)> {
        self.all_with_over_cap()
            .into_iter()
            .filter_map(|row| {
                let open = row.live.open.load(Ordering::Acquire);
                (open > 0).then(|| (row.name.to_string(), open))
            })
            .collect()
    }

    /// [`Self::all`], with the over-cap row whether or not it has counted
    /// anything yet: a request open on it has not.
    fn all_with_over_cap(&self) -> Vec<Arc<Row>> {
        let mut rows = self.all();
        if !rows.iter().any(|row| Arc::ptr_eq(row, &self.over_cap)) {
            rows.push(Arc::clone(&self.over_cap));
        }
        rows
    }

    /// The report `/speed.json` serves, at `now_ms`.
    #[must_use]
    pub fn report(&self, now_ms: u64) -> Report {
        let mut over_cap = 0;
        let rows = self
            .all()
            .into_iter()
            .map(|row| {
                let since_start = row.since_start();
                if row.name.as_ref() == OVER_CAP {
                    over_cap = since_start.n;
                }
                let frozen = row.frozen.lock().unwrap_or_else(PoisonError::into_inner);
                let (last_minute, last_hour, minutes) = match (frozen.front(), frozen.back()) {
                    (Some(oldest), Some(newest)) if frozen.len() >= 2 => {
                        let before = frozen.len().checked_sub(2).and_then(|i| frozen.get(i));
                        let minute = before.map_or_else(Figures::default, |b| {
                            between(b, newest, newest.minute_max)
                        });
                        let hour_max = frozen
                            .iter()
                            .skip(1)
                            .map(|f| f.minute_max)
                            .max()
                            .unwrap_or(0);
                        (
                            minute,
                            between(oldest, newest, hour_max),
                            (frozen.len() - 1) as u64,
                        )
                    }
                    _ => (Figures::default(), Figures::default(), 0),
                };
                let first = row.live.first_plus_one.load(Ordering::Relaxed);
                RowReport {
                    route: row.name.to_string(),
                    kind: row.kind,
                    since_start,
                    last_minute,
                    last_hour,
                    last_hour_minutes: minutes,
                    first_after_start_micros: first.checked_sub(1),
                }
            })
            .collect();
        Report {
            started_ms: self.started_ms,
            now_ms,
            rows,
            over_cap,
            resolution: RESOLUTION,
        }
    }
}

/// Milliseconds since the epoch, as the log stamps them.
#[must_use]
pub fn now_ms() -> u64 {
    u64::try_from(telemetry::now_millis()).unwrap_or(0)
}

/// This process's table, and the thread that freezes it each minute.
static TABLE: OnceLock<Speed> = OnceLock::new();

/// This process's table. The first call starts the minute thread; a thread
/// that cannot start is said once in the log, and the table still counts
/// (its minutes are then written at shutdown only).
pub fn global() -> &'static Speed {
    TABLE.get_or_init(|| {
        let spawned = std::thread::Builder::new()
            .name("speed-minute".into())
            .spawn(minute_thread);
        if let Err(why) = spawned {
            let _said = telemetry::emit(
                &telemetry::Event::error("api.speed", "the minute thread did not start")
                    .with("why", telemetry::Value::Str(&why.to_string())),
            );
        }
        Speed::new(now_ms(), ROUTE_CAP)
    })
}

/// Sleeps to each minute's end and freezes the table, writing its lines.
fn minute_thread() {
    loop {
        let now = now_ms();
        let wait = MINUTE_MS - now % MINUTE_MS;
        std::thread::sleep(std::time::Duration::from_millis(wait));
        if let Some(table) = TABLE.get() {
            for line in table.roll(now_ms(), false) {
                let _written = line.emit();
            }
        }
    }
}

/// Writes the minute still open, marked partial, so a shutdown loses no
/// count. Returns how many lines were written. A process that never counted
/// anything writes nothing.
pub fn flush_at_shutdown() -> usize {
    let Some(table) = TABLE.get() else {
        return 0;
    };
    let lines = table.roll(now_ms(), true);
    for line in &lines {
        let _written = line.emit();
    }
    lines.len()
}

/// The open requests named in one line: `route=n` each, comma-separated, and
/// their total. The order is the table's (routes first asked, then stages).
#[must_use]
pub fn open_summary(open: &[(String, u64)]) -> (u64, String) {
    let total = open
        .iter()
        .map(|(_, n)| *n)
        .fold(0_u64, u64::saturating_add);
    let routes = open
        .iter()
        .map(|(route, n)| format!("{route}={n}"))
        .collect::<Vec<_>>()
        .join(", ");
    (total, routes)
}

/// The last count a process makes, AFTER its runtime has ended (R3-14).
///
/// A request the drain bound cut ([`drain_bound`]) is still a live future in
/// its connection task when `serve_limited` returns, so its [`Pending`] is
/// counted only when the runtime drops that task, after the serve's own
/// [`flush_at_shutdown`]. This second partial freeze writes those counts, each
/// as [`Outcome::Cancelled`] with the time it had run, so every request the
/// server received lands in exactly one minute line. A request still open even
/// now (its future outlived the runtime's end) cannot be counted: it is named
/// at `error` with how many and on which rows, never dropped in silence.
/// Returns how many minute lines were written.
pub fn flush_at_exit() -> usize {
    let written = flush_at_shutdown();
    if let Some(table) = TABLE.get() {
        let open = table.open_now();
        if !open.is_empty() {
            let (total, routes) = open_summary(&open);
            let _said = telemetry::emit(
                &telemetry::Event::error(
                    "api.speed",
                    "requests still open at exit were not counted",
                )
                .with("open", telemetry::Value::Uint(total))
                .with("routes", telemetry::Value::Str(&routes)),
            );
        }
    }
    written
}

/// What stops the server gracefully: Ctrl-C, or the `SIGTERM` every stop
/// script and every redeploy sends (R3-14).
///
/// With Ctrl-C alone, `kill -TERM` (what `stop.sh` and the redeploy of the
/// operator's app send) ended the process at once: the open minute
/// ([`flush_at_shutdown`]), like every other step of a graceful stop, never
/// ran, so each redeploy lost up to a minute of every route's counts. A
/// `SIGTERM` the OS will not let this process catch is said in the log, and
/// Ctrl-C still stops it gracefully.
///
/// # Errors
///
/// Ctrl-C's own registration error, as [`tokio::signal::ctrl_c`] gives it.
pub async fn stop_asked() -> std::io::Result<()> {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(mut term) => {
            first_stop(tokio::signal::ctrl_c(), async move {
                // A stream that ended delivers no more signals: not a stop.
                if term.recv().await.is_none() {
                    std::future::pending::<()>().await;
                }
            })
            .await
        }
        Err(why) => {
            let _said = telemetry::emit(
                &telemetry::Event::error("api.speed", "SIGTERM cannot be caught")
                    .with("why", telemetry::Value::Str(&why.to_string())),
            );
            tokio::signal::ctrl_c().await
        }
    }
}

/// Whichever stop comes first: Ctrl-C, with its own result, or a terminate
/// request, which is a stop and not an error. [`stop_asked`] races the two
/// signals through this, so the race is proved without sending one.
///
/// # Errors
///
/// Ctrl-C's own error, when it comes first.
pub async fn first_stop(
    ctrl_c: impl std::future::Future<Output = std::io::Result<()>>,
    terminate: impl std::future::Future<Output = ()>,
) -> std::io::Result<()> {
    tokio::select! {
        asked = ctrl_c => asked,
        () = terminate => Ok(()),
    }
}

/// Counts one import or checking stage that took `micros`, in the same
/// buckets a route has, under `stage:<name>`.
pub fn stage(name: &str, micros: u64, outcome: Outcome) {
    let now = now_ms();
    global().record(&format!("stage:{name}"), Kind::Stage, micros, outcome, now);
}

/// Starts timing one import or checking stage under `stage:<name>`, in the
/// same buckets a route has: [`Pending::finish_now`] counts it with how it
/// ended, and a stage dropped unfinished (its task cancelled, its thread
/// unwound) counts [`Outcome::Cancelled`]. One allocation for the name and one
/// read-locked probe, once per stage: a stage is a landing or a scrub, never a
/// row (R3-14, RA-014).
#[must_use]
pub fn stage_start(name: &str) -> Pending {
    Pending::start(global().row(&format!("stage:{name}"), Kind::Stage, now_ms()))
}

/// One stage whose WORK may outlive the request that started it (R3-14
/// repair 2): a scrub on the blocking pool goes on after its client left.
///
/// A [`Pending`] held by the request future was counted
/// [`Outcome::Cancelled`] at about one second when a client left, while the
/// scrub it timed ran on for three minutes: the stage timed the request, not
/// the work. Each clone shares one `Pending`; whichever side calls
/// [`Self::finish`] first counts it, once, and the others find it counted.
/// The work's side finishes it when the work ends; the request's side
/// finishes it only when the work never ran or never ended (a full pool, a
/// panic). Dropped by every side unfinished, it counts `Cancelled` as a
/// `Pending` does. One lock per finish, once per stage, never per row.
#[derive(Debug, Clone)]
pub struct SharedStage(std::sync::Arc<std::sync::Mutex<Option<Pending>>>);

impl SharedStage {
    /// Starts timing the stage `name`, as [`stage_start`] does.
    #[must_use]
    pub fn start(name: &str) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(Some(
            stage_start(name),
        ))))
    }

    /// Counts the stage with `outcome` if no clone has yet; returns whether
    /// this call counted it.
    pub fn finish(&self, outcome: Outcome) -> bool {
        let taken = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        taken.map(|pending| pending.finish_now(outcome)).is_some()
    }
}

/// How long a stop waits for requests still being answered before it writes
/// the open minute and returns anyway (R3-14): the same three seconds the
/// runtime's own end allows its blocking work (`server::SHUTDOWN_GRACE`).
///
/// THREE, NOT TEN (R3-14 repair 3). At ten, a stop with a request open held
/// the port for twenty seconds, and the operator's redeploy, which waits about
/// ten for the port, then started the new build against a port still bound:
/// it was refused ("Address already in use"), and when the old process ended
/// nothing listened. See [`REDEPLOY_WAIT`].
pub const DRAIN_GRACE: std::time::Duration = std::time::Duration::from_secs(3);

/// THE WHOLE STOP, BOUNDED (R3-14): from the signal to the process's end, at
/// most [`DRAIN_GRACE`] for open requests, then `server::SHUTDOWN_GRACE` (the
/// same three seconds) for blocking work, so six seconds and the final
/// writes. For all of it the log, the store's `serve.lock` and the port are
/// held (`server::Held`), and freed in that order: a redeploy that waits for
/// the port to close finds the store and the log free, never a refusal or a
/// "log: NOT WRITABLE" from a process that is still ending.
pub const STOP_BOUND: std::time::Duration =
    std::time::Duration::from_secs(DRAIN_GRACE.as_secs() + crate::server::SHUTDOWN_GRACE.as_secs());

/// THE LEAST TIME A REDEPLOY WAITS for the old process's port after its
/// `SIGTERM` before it starts the new build (R3-14 repair 3): the operator's
/// redeploy polls the port fifty times 0.2 s apart, ten seconds of sleep
/// (measured: the loop ended 13.4 s after the signal, counting the poll's own
/// time). [`STOP_BOUND`] must end well inside it, with half of itself again to
/// spare for the final writes and a loaded machine; a tool that waits less, or
/// a bound raised past it, brings back the refused redeploy. Asserted by
/// `speed_tests::the_whole_stop_ends_inside_what_a_redeploy_waits_for`.
pub const REDEPLOY_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// Resolves `grace` after `stopping` says a stop was asked, having said in
/// the log, at `warn` with the grace, how many requests were still open and
/// on which rows (`cut`, `routes`: `route=n`), that they were cut short.
/// `serve_limited` races it against the server's graceful stop, so the stop
/// waits for open requests no longer than `grace`.
///
/// WHY A BOUND (R3-14). A graceful stop waits for every request still being
/// answered. With `SIGTERM` now a graceful stop ([`stop_asked`]), one slow
/// read in flight would hold a redeploy's `kill` for as long as that read
/// takes, where before `SIGTERM` ended the process at once. Past `grace` the
/// cut is said, never silent, and the caller then writes the open minute. A
/// `stopping` whose sender went away unsent (the server ended on its own)
/// never resolves, so it can never end a server that was not asked to stop.
///
/// A CUT REQUEST IS STILL COUNTED, LATER. Its future lives on in its
/// connection task until the runtime ends; [`flush_at_exit`] writes it then.
pub async fn drain_bound(stopping: tokio::sync::oneshot::Receiver<()>, grace: std::time::Duration) {
    if stopping.await.is_err() {
        std::future::pending::<()>().await;
    }
    tokio::time::sleep(grace).await;
    let open = TABLE.get().map(Speed::open_now).unwrap_or_default();
    let (cut, routes) = open_summary(&open);
    let _said = telemetry::emit(
        &telemetry::Event::warn(
            "api.serve",
            "requests still open at the stop were cut short",
        )
        .with(
            "grace_ms",
            telemetry::Value::Uint(u64::try_from(grace.as_millis()).unwrap_or(u64::MAX)),
        )
        .with("cut", telemetry::Value::Uint(cut))
        .with("routes", telemetry::Value::Str(&routes)),
    );
}

/// A request between its start and its answer: dropped unanswered (the
/// client left and the server dropped the future), it counts as
/// [`Outcome::Cancelled`] with the time it had run.
#[derive(Debug)]
pub struct Pending {
    row: Option<Handle>,
    started: std::time::Instant,
}

impl Pending {
    /// Starts timing one request on `row`.
    #[must_use]
    pub fn start(row: Handle) -> Self {
        row.0.live.open.fetch_add(1, Ordering::AcqRel);
        Self {
            row: Some(row),
            started: std::time::Instant::now(),
        }
    }

    /// The time since the start, µs.
    #[must_use]
    pub fn micros(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_micros()).unwrap_or(u64::MAX)
    }

    /// Counts the answer: `micros` and how it ended.
    pub fn finish(mut self, micros: u64, outcome: Outcome) {
        if let Some(row) = self.row.take() {
            row.add(micros, outcome);
            row.0.live.open.fetch_sub(1, Ordering::AcqRel);
        }
    }

    /// Counts the answer with the time since [`Pending::start`].
    pub fn finish_now(self, outcome: Outcome) {
        let micros = self.micros();
        self.finish(micros, outcome);
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        if let Some(row) = self.row.take() {
            row.add(self.micros(), Outcome::Cancelled);
            row.0.live.open.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

// ---------------------------------------------------------------------------
// THE /db READ WINDOWS (WP-33c, DB-A42), moved here from `logs.rs` (R3-14)
// ---------------------------------------------------------------------------
//
// One module keeps every per-route counter. `logs::note_access` still decides
// what a `/db` read writes (its sampled line and its window's count); the
// counting itself is here, beside the buckets every route has.

/// One route's access window.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AccessWindow {
    /// When it opened, ms.
    pub(crate) opened_ms: u64,
    /// Reads served in it.
    pub(crate) served: u64,
    /// Reads refused in it.
    pub(crate) refused: u64,
    /// The slowest read in it, microseconds.
    pub(crate) slowest: u64,
    /// Whether its served read was written.
    pub(crate) sampled: bool,
}

/// What one read writes, decided by [`AccessWindows::account`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AccessPlan {
    /// The window that just closed, to be said first, when it held more than
    /// the line it wrote.
    pub(crate) closed: Option<AccessWindow>,
    /// Whether this read is written whole.
    pub(crate) sample: bool,
}

/// Every route's access window. O(1) a read: one probe of a table of at most
/// [`crate::logs::DB_READS`] routes.
///
/// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
/// "Cost claims of the /db fix round that name no proof").
#[derive(Debug, Default)]
pub(crate) struct AccessWindows {
    routes: std::collections::HashMap<&'static str, AccessWindow>,
}

impl AccessWindows {
    /// Accounts one read of `route`, `refused` or served, taking `micros`, at
    /// `now_ms`.
    pub(crate) fn account(
        &mut self,
        route: &'static str,
        refused: bool,
        micros: u64,
        now_ms: u64,
    ) -> AccessPlan {
        let window = self.routes.entry(route).or_insert(AccessWindow {
            opened_ms: now_ms,
            ..AccessWindow::default()
        });
        let mut plan = AccessPlan::default();
        if now_ms.saturating_sub(window.opened_ms) >= crate::logs::ACCESS_WINDOW_MS
            || now_ms < window.opened_ms
        {
            let reads = window.served.saturating_add(window.refused);
            if reads > u64::from(window.sampled) {
                plan.closed = Some(*window);
            }
            *window = AccessWindow {
                opened_ms: now_ms,
                ..AccessWindow::default()
            };
        }
        if refused {
            window.refused = window.refused.saturating_add(1);
        } else {
            window.served = window.served.saturating_add(1);
            if !window.sampled {
                window.sampled = true;
                plan.sample = true;
            }
        }
        window.slowest = window.slowest.max(micros);
        plan
    }
}

#[cfg(test)]
#[path = "speed_tests.rs"]
mod tests;

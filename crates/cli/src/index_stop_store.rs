//! Immutable observations of the single completed-signal-candle stop model.
//! Cold opening authenticates and decodes all saved bytes under explicit byte and
//! record bounds; run-ID deduplication uses an expected-linear hash set. Warm
//! pinned setting access is direct indexing; decoded rows
//! cannot be converted into a producer `Evaluation` or institutional authority.
use crate::candidate_universe::boolean_candidate_v1::persistence::Observation;
use runner::signal_candle_stop::{Evaluation, observation_digest};
use std::path::Path;

pub use runner::expression::{Expression, Summary as TruthSummary};
pub use runner::identity::Direction;
pub use runner::research_family::ResearchFamilyV1;
pub use runner::signal_candle_stop::{Event, ExitReason, Metrics, Period, Policy, Reason, Trade};

/// Independent, append-only namespace; never an old exit-grid body.
pub const NAMESPACE: &str = "index-stop-candidates-v1";
const MAGIC: &[u8; 8] = b"BRISCT01";
pub(crate) const HEADER_BYTES: usize = 8 + 32 + Policy::BYTE_LEN + 8;
/// Minimum bytes of one record, before any event, trade or daily observations.
pub const RECORD_BYTES: usize =
    96 + 128 + 32 + vocab::expression::ENCODED_LEN + 32 + Metrics::BYTE_LEN + 24;
const RECEIPT_BYTES: u64 = 112;

/// Read-only candidate observation. Only the native producer seals evaluations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    run_id: [u8; 32],
    source_id: [u8; 32],
    evaluation_digest: [u8; 32],
    family: ResearchFamilyV1,
    direction: Direction,
    timeframe: &'static str,
    first_day: i64,
    last_day: i64,
    program: Expression,
    truth: TruthSummary,
    metrics: Metrics,
    events: Vec<Event>,
    trades: Vec<Trade>,
    periods: Vec<Period>,
}
impl Record {
    /// Exact native program-aware run identity.
    #[must_use]
    pub const fn run_id(&self) -> [u8; 32] {
        self.run_id
    }
    /// Original prepared OHLCV, reference and calendar source identity.
    #[must_use]
    pub const fn source_id(&self) -> [u8; 32] {
        self.source_id
    }
    /// Native digest of the complete saved observations.
    #[must_use]
    pub const fn evaluation_digest(&self) -> [u8; 32] {
        self.evaluation_digest
    }
    /// Exact supported index family.
    #[must_use]
    pub const fn family(&self) -> ResearchFamilyV1 {
        self.family
    }
    /// One explicitly saved direction.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }
    /// Original intraday signal timeframe.
    #[must_use]
    pub const fn timeframe(&self) -> &'static str {
        self.timeframe
    }
    /// First requested IST civil day.
    #[must_use]
    pub const fn first_day(&self) -> i64 {
        self.first_day
    }
    /// Last requested IST civil day.
    #[must_use]
    pub const fn last_day(&self) -> i64 {
        self.last_day
    }
    /// Full original expression, not just its referenced bit mask.
    #[must_use]
    pub fn program(&self) -> &Expression {
        &self.program
    }
    /// Every evaluated truth bucket, including unknown.
    #[must_use]
    pub const fn truth(&self) -> TruthSummary {
        self.truth
    }
    /// Exact gross accounting, separate from any approval.
    #[must_use]
    pub const fn metrics(&self) -> Metrics {
        self.metrics
    }
    /// Every definite signal, including refusals and deliberately skipped entries.
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    /// Exact completed round trips, with both fill bounds.
    #[must_use]
    pub fn trades(&self) -> &[Trade] {
        &self.trades
    }
    /// Actually observed execution days; absent days are never manufactured.
    #[must_use]
    pub fn periods(&self) -> &[Period] {
        &self.periods
    }
}

/// One authenticated body retained under a fixed receipt and file generations.
pub struct Reader {
    observation: Observation,
    records: Vec<Record>,
}
impl Reader {
    /// Authenticate the entire bounded artifact before admitting any page.
    /// # Errors
    /// Refuses absent, changed, busy, malformed or over-budget evidence.
    pub fn open(
        root: &Path,
        identity: [u8; 32],
        max_bytes: u64,
        max_records: u64,
    ) -> Result<Self, String> {
        let (observation, records) =
            Observation::open(root, NAMESPACE, identity, max_bytes, |body| {
                decode(body, identity, max_records)
            })?;
        Ok(Self {
            observation,
            records,
        })
    }
    /// Exact requested candidate artifact identity.
    #[must_use]
    pub fn identity(&self) -> [u8; 32] {
        self.observation.identity()
    }
    /// Exact receipt pin for all continued pages.
    #[must_use]
    pub fn completion_digest(&self) -> [u8; 32] {
        self.observation.completion_digest()
    }
    /// Serialized body plus completion bytes admitted by this reader.
    #[must_use]
    pub fn admitted_bytes(&self) -> u64 {
        self.observation.body_bytes() + RECEIPT_BYTES
    }
    pub(crate) const fn observation(&self) -> &Observation {
        &self.observation
    }
    /// Check the fixed owner/body/receipt generations without rescanning history.
    /// # Errors
    /// Refuses replaced, modified, removed or concurrently owned files.
    pub fn require_current(&self) -> Result<(), String> {
        self.observation.require_current()
    }
    /// Read the already authenticated vector; callers must retain the pin and
    /// hold `with_current` across a complete response projection.
    #[must_use]
    pub fn records(&self) -> &[Record] {
        &self.records
    }
    /// Exact pinned O(1) record lookup after cold admission.
    /// # Errors
    /// Refuses foreign pins, changes or a setting outside the saved extent.
    pub fn record(&self, pin: [u8; 32], index: usize) -> Result<&Record, String> {
        if pin != self.completion_digest() {
            return Err("single-stop completion differs".into());
        }
        self.require_current()?;
        self.records
            .get(index)
            .ok_or_else(|| "single-stop setting is outside saved extent".into())
    }
    /// Hold one publication lease across a complete bounded projection. Access
    /// `records()` directly inside; do not nest `record()` or `require_current()`
    /// because those independently acquire and release the same file lease.
    /// # Errors
    /// Refuses changed evidence or an error from the projection.
    pub fn with_current<T>(
        &self,
        project: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        self.observation.with_current(project)
    }
}

pub(crate) fn encode(
    identity: [u8; 32],
    evaluations: &[Evaluation],
    max_bytes: u64,
) -> Result<Vec<u8>, String> {
    if identity == [0; 32] || evaluations.is_empty() || !evaluations.len().is_multiple_of(2) {
        return Err(
            "single-stop artifact requires an identity and complete long/short pairs".into(),
        );
    }
    let admitted = encoded_bytes(evaluations)?;
    if admitted > max_bytes {
        return Err("single-stop complete observations exceed explicit byte admission".into());
    }
    let length = usize::try_from(
        admitted
            .checked_sub(RECEIPT_BYTES)
            .ok_or("single-stop byte admission excludes receipt")?,
    )
    .map_err(display)?;
    encode_admitted(identity, evaluations, length, max_bytes)
}

pub(crate) fn encoded_bytes(evaluations: &[Evaluation]) -> Result<u64, String> {
    let mut length = HEADER_BYTES;
    for value in evaluations {
        length = length
            .checked_add(RECORD_BYTES)
            .and_then(|n| {
                value
                    .events()
                    .len()
                    .checked_mul(Event::BYTE_LEN)
                    .and_then(|m| n.checked_add(m))
            })
            .and_then(|n| {
                value
                    .trades()
                    .len()
                    .checked_mul(Trade::BYTE_LEN)
                    .and_then(|m| n.checked_add(m))
            })
            .and_then(|n| {
                value
                    .periods()
                    .len()
                    .checked_mul(Period::BYTE_LEN)
                    .and_then(|m| n.checked_add(m))
            })
            .ok_or("single-stop byte count overflow")?;
    }
    u64::try_from(length)
        .map_err(display)?
        .checked_add(RECEIPT_BYTES)
        .ok_or_else(|| "single-stop byte count overflow".into())
}

fn encode_admitted(
    identity: [u8; 32],
    evaluations: &[Evaluation],
    length: usize,
    max_bytes: u64,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length).map_err(display)?;
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&identity);
    bytes.extend_from_slice(&Policy::V1.canonical_bytes());
    put(
        &mut bytes,
        u64::try_from(evaluations.len()).map_err(display)?,
    );
    for value in evaluations {
        if value.policy() != Policy::V1 {
            return Err("single-stop producer policy differs".into());
        }
        for id in [value.run_id(), value.source_id(), value.digest()] {
            bytes.extend_from_slice(&id);
        }
        bytes.extend_from_slice(&value.family().encode());
        put(&mut bytes, u64::from(value.direction().byte()));
        let rung = crate::EVERY_RUNG
            .iter()
            .position(|label| *label == value.timeframe())
            .ok_or("single-stop timeframe unsupported")?;
        put(&mut bytes, u64::try_from(rung).map_err(display)?);
        bytes.extend_from_slice(&value.first_day().to_le_bytes());
        bytes.extend_from_slice(&value.last_day().to_le_bytes());
        bytes.extend_from_slice(&value.program().encode());
        let truth = value.truth();
        for word in [truth.evaluated, truth.hits, truth.misses, truth.unknown] {
            put(&mut bytes, word);
        }
        bytes.extend_from_slice(&value.metrics().canonical_bytes());
        for count in [
            value.events().len(),
            value.trades().len(),
            value.periods().len(),
        ] {
            put(&mut bytes, u64::try_from(count).map_err(display)?);
        }
        for row in value.events() {
            bytes.extend_from_slice(&row.canonical_bytes());
        }
        for row in value.trades() {
            bytes.extend_from_slice(&row.canonical_bytes());
        }
        for row in value.periods() {
            bytes.extend_from_slice(&row.canonical_bytes());
        }
    }
    if bytes.len() != length {
        return Err("single-stop encoder length differs".into());
    }
    // Decode the exact bytes before publication, using the same cold-read gates.
    decode(&bytes, identity, max_bytes)?;
    Ok(bytes)
}

#[expect(
    clippy::too_many_lines,
    reason = "one canonical decoder checks every fixed field and child extent before admitting the complete paired catalog"
)]
fn decode(bytes: &[u8], identity: [u8; 32], max_records: u64) -> Result<Vec<Record>, String> {
    let mut raw = Decoder { bytes, at: 0 };
    if raw.take(8)? != MAGIC || raw.array::<32>()? != identity || identity == [0; 32] {
        return Err("single-stop identity or format differs".into());
    }
    Policy::decode(raw.take(Policy::BYTE_LEN)?).map_err(display)?;
    let count = raw.count()?;
    if count == 0
        || !count.is_multiple_of(2)
        || u64::try_from(count).map_err(display)? > max_records
        || count > raw.remaining() / RECORD_BYTES
    {
        return Err("single-stop record count exceeds extent or pair admission".into());
    }
    let mut records = Vec::new();
    records.try_reserve_exact(count).map_err(display)?;
    let mut runs = std::collections::HashSet::new();
    runs.try_reserve(count).map_err(display)?;
    let mut charged = 0_u64;
    for index in 0..count {
        let (run_id, source_id, evaluation_digest) = (raw.array()?, raw.array()?, raw.array()?);
        let family = ResearchFamilyV1::decode(&raw.array::<128>()?).map_err(display)?;
        let direction = match raw.word()? {
            1 => Direction::Long,
            2 => Direction::Short,
            _ => return Err("single-stop direction unsupported".into()),
        };
        let timeframe = *crate::EVERY_RUNG
            .get(raw.count()?)
            .ok_or("single-stop timeframe outside intraday scope")?;
        let first_day = i64::from_le_bytes(raw.array()?);
        let last_day = i64::from_le_bytes(raw.array()?);
        let program = Expression::decode(&raw.array()?)
            .map_err(|why| format!("single-stop expression: {why:?}"))?;
        let truth = TruthSummary {
            evaluated: raw.word()?,
            hits: raw.word()?,
            misses: raw.word()?,
            unknown: raw.word()?,
        };
        let metrics = Metrics::decode(raw.take(Metrics::BYTE_LEN)?).map_err(display)?;
        let (events, trades, periods) = (raw.count()?, raw.count()?, raw.count()?);
        charged = charged
            .checked_add(u64::try_from(events).map_err(display)?)
            .and_then(|n| n.checked_add(u64::try_from(trades).ok()?))
            .and_then(|n| n.checked_add(u64::try_from(periods).ok()?))
            .ok_or("single-stop record admission overflow")?;
        if charged > max_records
            || events != usize::try_from(truth.hits).map_err(display)?
            || trades > events
            || events
                .checked_mul(Event::BYTE_LEN)
                .and_then(|n| {
                    trades
                        .checked_mul(Trade::BYTE_LEN)
                        .and_then(|m| n.checked_add(m))
                })
                .and_then(|n| {
                    periods
                        .checked_mul(Period::BYTE_LEN)
                        .and_then(|m| n.checked_add(m))
                })
                .is_none_or(|n| n > raw.remaining())
        {
            return Err("single-stop child counts exceed complete saved extent".into());
        }
        let events = raw.rows(events, Event::BYTE_LEN, Event::decode)?;
        let trades = raw.rows(trades, Trade::BYTE_LEN, Trade::decode)?;
        let periods = raw.rows(periods, Period::BYTE_LEN, Period::decode)?;
        let record = Record {
            run_id,
            source_id,
            evaluation_digest,
            family,
            direction,
            timeframe,
            first_day,
            last_day,
            program,
            truth,
            metrics,
            events,
            trades,
            periods,
        };
        validate(&record)?;
        if !runs.insert(run_id) {
            return Err("single-stop run identity is repeated".into());
        }
        let expected = if index.is_multiple_of(2) {
            Direction::Long
        } else {
            Direction::Short
        };
        if direction != expected {
            return Err("single-stop records must retain program long/short order".into());
        }
        if index % 2 == 1 {
            let prior = records.last().ok_or("single-stop long pair is absent")?;
            validate_pair(prior, &record)?;
        }
        records.push(record);
    }
    if raw.remaining() != 0 {
        return Err("single-stop trailing bytes refused".into());
    }
    Ok(records)
}
fn validate_pair(long: &Record, short: &Record) -> Result<(), String> {
    if long.source_id != short.source_id
        || long.family != short.family
        || long.program != short.program
        || long.timeframe != short.timeframe
        || long.first_day != short.first_day
        || long.last_day != short.last_day
        || long.truth != short.truth
        || long.periods.len() != short.periods.len()
        || long.periods.iter().zip(&short.periods).any(|(a, b)| {
            a.day != b.day
                || a.offered_bars != b.offered_bars
                || a.accepted_bars != b.accepted_bars
                || a.unavailable_minutes != b.unavailable_minutes
                || a.close_verified != b.close_verified
        })
    {
        return Err(
            "single-stop paired directions changed their program or source geometry".into(),
        );
    }
    Ok(())
}
fn validate(record: &Record) -> Result<(), String> {
    if [record.run_id, record.source_id, record.evaluation_digest].contains(&[0; 32])
        || record.family.legacy_index_family().is_none()
        || record.first_day > record.last_day
        || !record.truth.reconciles()
        || observation_digest(
            record.source_id,
            record.run_id,
            record.truth,
            record.metrics,
            &record.events,
            &record.trades,
            &record.periods,
        ) != record.evaluation_digest
    {
        return Err(
            "single-stop identity, source, truth or native observation digest differs".into(),
        );
    }
    reconcile(record)
}
/// Whether an event's entry is present exactly when its reason needs one --
/// D-0603. An entry is absent for two reasons only: a one-minute bar missing
/// before 15:10 (`Unreachable`), or a signal whose close is after its own day's
/// 15:09, which every session's last bucket is (`TooLate`). A late event without
/// an entry must prove it, or a real hole could be filed as late and slip past
/// `execution_complete`.
fn entry_is_owned(event: &Event, signal_day: i64) -> Result<bool, String> {
    Ok(match (event.reason, event.entry_bar) {
        (Reason::Unreachable, entry) => entry.is_none(),
        (Reason::TooLate, None) => {
            let boundary = signal_day
                .checked_mul(86_400_000_000)
                .and_then(|day| day.checked_sub(indicators::IST_OFFSET_MICROS))
                .and_then(|day| {
                    day.checked_add((runner::outcome::FORCED_EXIT_MINUTE - 1) * 60_000_000)
                })
                .ok_or("single-stop forced-exit boundary overflow")?;
            event.signal_close_micros > boundary
        }
        (_, entry) => entry.is_some(),
    })
}
#[expect(
    clippy::too_many_lines,
    reason = "the ordered trade, event and day reconciliations share one occupancy and totals proof without skipping source rows"
)]
fn reconcile(record: &Record) -> Result<(), String> {
    let mut metrics = Metrics::default();
    let mut peak = 0_i64;
    let mut previous_exit = None;
    for trade in &record.trades {
        if previous_exit.is_some_and(|exit| trade.entry_micros <= exit)
            || (record.direction == Direction::Long) != (trade.entry_paisa > trade.stop_paisa)
        {
            return Err("single-stop trades overlap or contradict their direction".into());
        }
        previous_exit = Some(trade.exit_bar_micros);
        metrics.trades = add(metrics.trades, 1)?;
        metrics.wins = add(metrics.wins, u64::from(trade.pessimistic_paisa > 0))?;
        metrics.optimistic_paisa = money(metrics.optimistic_paisa, trade.optimistic_paisa)?;
        metrics.pessimistic_paisa = money(metrics.pessimistic_paisa, trade.pessimistic_paisa)?;
        peak = peak.max(metrics.pessimistic_paisa);
        metrics.drawdown_paisa = metrics.drawdown_paisa.max(
            peak.checked_sub(metrics.pessimistic_paisa)
                .ok_or("single-stop drawdown overflow")?,
        );
        metrics.worst_trade_paisa = if metrics.trades == 1 {
            trade.pessimistic_paisa
        } else {
            metrics.worst_trade_paisa.min(trade.pessimistic_paisa)
        };
        metrics.adverse_paisa = metrics.adverse_paisa.max(trade.adverse_paisa);
        metrics.favourable_paisa = metrics.favourable_paisa.max(trade.favourable_paisa);
        metrics.holding_minutes = add(metrics.holding_minutes, trade.holding_minutes)?;
        if trade.exit_reason == ExitReason::Stop {
            metrics.stopped = add(metrics.stopped, 1)?;
        } else {
            metrics.forced = add(metrics.forced, 1)?;
        }
        metrics.stop_gaps = add(metrics.stop_gaps, u64::from(trade.gapped))?;
    }
    let mut next_trade = 0_usize;
    let mut previous_signal = None;
    let mut occupied = None;
    let mut day_counts: Vec<Period> = Vec::new();
    day_counts
        .try_reserve_exact(record.periods.len())
        .map_err(display)?;
    for period in &record.periods {
        if period.day < record.first_day
            || period.day > record.last_day
            || day_counts.last().is_some_and(|row| row.day >= period.day)
        {
            return Err("single-stop observed days repeat or leave requested range".into());
        }
        day_counts.push(Period {
            trades: 0,
            wins: 0,
            optimistic_paisa: 0,
            pessimistic_paisa: 0,
            refused: 0,
            signals: 0,
            ..*period
        });
    }
    let mut period_index = 0_usize;
    for event in &record.events {
        let signal_day = event
            .signal_micros
            .checked_add(indicators::IST_OFFSET_MICROS)
            .ok_or("single-stop signal day overflow")?
            .div_euclid(86_400_000_000);
        if !entry_is_owned(event, signal_day)?
            || event.signal_close_micros.checked_sub(event.signal_micros)
                != Some(crate::stored::rung_length_micros(record.timeframe)?)
            || signal_day < record.first_day
            || signal_day > record.last_day
            || matches!(
                event.reason,
                Reason::Unreachable | Reason::TooLate | Reason::GapInvalid
            ) && event.occupied_through_micros.is_some()
        {
            return Err("single-stop signal clock or refusal ownership is inconsistent".into());
        }
        if previous_signal
            .is_some_and(|(index, stamp)| event.signal_bar <= index || event.signal_micros <= stamp)
        {
            return Err("single-stop signals repeat or change order".into());
        }
        previous_signal = Some((event.signal_bar, event.signal_micros));
        let counter = match event.reason {
            Reason::None => None,
            Reason::Unreachable => Some(&mut metrics.unreachable),
            Reason::TooLate => Some(&mut metrics.too_late),
            Reason::GapInvalid => Some(&mut metrics.gap_invalid),
            Reason::WhileOpen => Some(&mut metrics.while_open),
            Reason::EntryRefused => Some(&mut metrics.entry_refused),
            Reason::PathRefused => Some(&mut metrics.path_refused),
            Reason::ClosingMinute => Some(&mut metrics.closing_refused),
        };
        if let Some(counter) = counter {
            *counter = add(*counter, 1)?;
        }
        let trade = if let Some(index) = event.trade_index {
            if usize::try_from(index).map_err(display)? != next_trade {
                return Err("single-stop event trades skip or repeat a row".into());
            }
            let trade = record
                .trades
                .get(next_trade)
                .ok_or("single-stop event trade absent")?;
            next_trade = next_trade
                .checked_add(1)
                .ok_or("single-stop trade index overflow")?;
            if trade.signal_bar != event.signal_bar
                || trade.signal_micros != event.signal_micros
                || trade.signal_close_micros != event.signal_close_micros
                || Some(trade.entry_bar) != event.entry_bar
                || Some(trade.entry_micros) != event.entry_micros
                || trade.stop_paisa != event.stop_paisa
                || Some(trade.exit_bar_micros) != event.occupied_through_micros
            {
                return Err("single-stop event and exact trade differ".into());
            }
            Some(trade)
        } else {
            None
        };
        if let Some(entry) = event.entry_micros {
            if (event.reason == Reason::WhileOpen) != occupied.is_some_and(|end| entry <= end) {
                return Err("single-stop occupied-minute ownership differs".into());
            }
            let day = entry
                .checked_add(indicators::IST_OFFSET_MICROS)
                .ok_or("single-stop day overflow")?
                .div_euclid(86_400_000_000);
            while day_counts
                .get(period_index)
                .is_some_and(|period| period.day < day)
            {
                period_index = period_index
                    .checked_add(1)
                    .ok_or("single-stop period index overflow")?;
            }
            let period = day_counts
                .get_mut(period_index)
                .filter(|period| period.day == day)
                .ok_or("single-stop signal entry has no observed day")?;
            period.signals = add(period.signals, 1)?;
            if matches!(
                event.reason,
                Reason::EntryRefused | Reason::PathRefused | Reason::ClosingMinute
            ) {
                period.refused = add(period.refused, 1)?;
            }
            if let Some(trade) = trade {
                period.trades = add(period.trades, 1)?;
                period.wins = add(period.wins, u64::from(trade.pessimistic_paisa > 0))?;
                period.optimistic_paisa = money(period.optimistic_paisa, trade.optimistic_paisa)?;
                period.pessimistic_paisa =
                    money(period.pessimistic_paisa, trade.pessimistic_paisa)?;
            }
        }
        if let Some(end) = event.occupied_through_micros {
            occupied = Some(occupied.map_or(end, |prior: i64| prior.max(end)));
        }
    }
    if metrics != record.metrics
        || next_trade != record.trades.len()
        || record.periods != day_counts
    {
        return Err(
            "single-stop metrics, event trade links or daily totals do not reconcile".into(),
        );
    }
    Ok(())
}
fn add(a: u64, b: u64) -> Result<u64, String> {
    a.checked_add(b)
        .ok_or_else(|| "single-stop count overflow".into())
}
fn money(a: i64, b: i64) -> Result<i64, String> {
    a.checked_add(b)
        .ok_or_else(|| "single-stop money overflow".into())
}
fn put(bytes: &mut Vec<u8>, word: u64) {
    bytes.extend_from_slice(&word.to_le_bytes());
}
fn display(value: impl std::fmt::Display) -> String {
    value.to_string()
}
struct Decoder<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Decoder<'a> {
    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.at)
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(count)
            .ok_or("single-stop decoder overflow")?;
        let value = self
            .bytes
            .get(self.at..end)
            .ok_or("single-stop field missing")?;
        self.at = end;
        Ok(value)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        self.take(N)?.try_into().map_err(display)
    }
    fn word(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn count(&mut self) -> Result<usize, String> {
        usize::try_from(self.word()?).map_err(display)
    }
    fn rows<T>(
        &mut self,
        count: usize,
        width: usize,
        decode: impl Fn(&[u8]) -> Result<T, runner::signal_candle_stop::Error>,
    ) -> Result<Vec<T>, String> {
        let mut rows = Vec::new();
        rows.try_reserve_exact(count).map_err(display)?;
        for _ in 0..count {
            rows.push(decode(self.take(width)?).map_err(display)?);
        }
        Ok(rows)
    }
}

#[cfg(test)]
#[path = "index_stop_store_tests.rs"]
mod tests;

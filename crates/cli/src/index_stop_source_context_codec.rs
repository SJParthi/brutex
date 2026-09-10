//! Canonical additive original-input snapshot. Every allocation is admitted
//! from checked encoded extents before decoding or rebuilding evaluator state.
use super::{ConditionName, Context, Limits, Link, Loaded, Metadata, ReadBounds, Record, display};
use brutex_core::instrument::InstrumentKey;
use brutex_core::vendor::Vendor;
use indicators::Candle;
use indicators::anchored::{DailyEligibility, DailyReference};
use indicators::column::Column;
use runner::identity::{DailyReferenceBinding, Direction, Params, ReferenceIntegrity};
use runner::signal_candle_stop::{ExitReason, Source};

const MAGIC: &[u8; 8] = b"BRISSC01";
const CANDLE_BYTES: u64 = 56;
// Includes the fixed header, seven count words and execution-alias flag.
const FIXED_BYTES: u64 = 8 + 3 * 32 + 5 * 4 + 2 * 8 + 3 * 8 + 7 * 8 + 1;
// Conservative accounting for vectors, sourced/reprojected columns, alignment,
// reference and period preparation. This is admission, not a measured RSS bound.
const REBUILD_BYTES_PER_SOURCE_RECORD: u64 = 4096;
const REBUILD_FIXED_BYTES: u64 = 1024 * 1024;

pub(super) struct Image {
    pub(super) source_id: [u8; 32],
    pub(super) source_binding: [u8; 32],
    calendar: [u8; 32],
    vocabulary_version: u32,
    reference_versions: [u32; 4],
    first_day: i64,
    last_day: i64,
    pub(super) limits: Limits,
    feed: Vendor,
    pub(super) key: InstrumentKey,
    pub(super) timeframe: String,
    commit: String,
    pub(super) names: Vec<ConditionName>,
    excluded_days: Vec<i64>,
    signal: Vec<Candle>,
    execution: Option<Vec<Candle>>,
    minute: Vec<Candle>,
    daily: Vec<Candle>,
    eligibility: Vec<u8>,
}

pub(super) fn encoded_bytes(loaded: &Loaded) -> Result<u64, String> {
    let mut length = FIXED_BYTES;
    for text in [
        loaded.vendor.as_str(),
        &loaded.data.signal.key.to_string(),
        &loaded.rung,
        &loaded.commit,
    ] {
        length = add(length, add(8, text.len() as u64)?)?;
    }
    for name in &vocab::table::TABLE {
        length = add(length, add(10, name.name.len() as u64)?)?;
    }
    length = add(
        length,
        mul(loaded.reference().excluded_ist_days.len() as u64, 8)?,
    )?;
    for bars in [
        loaded.data.signal.bars.as_slice(),
        loaded
            .data
            .execution
            .as_ref()
            .map_or(&[], |value| value.bars.as_slice()),
        loaded.data.exact_minute.bars.as_slice(),
        loaded.data.daily.bars.as_slice(),
    ] {
        length = add(length, mul(bars.len() as u64, CANDLE_BYTES)?)?;
    }
    add(length, loaded.data.daily.eligibility.len() as u64)
}

pub(super) fn records(loaded: &Loaded) -> Result<u64, String> {
    [
        loaded.data.signal.bars.len(),
        loaded
            .data
            .execution
            .as_ref()
            .map_or(0, |value| value.bars.len()),
        loaded.data.exact_minute.bars.len(),
        loaded.data.daily.bars.len(),
    ]
    .into_iter()
    .try_fold(0_u64, |total, count| add(total, count as u64))
}

pub(super) fn rebuild_bytes(source_bytes: u64, records: u64) -> Result<u64, String> {
    add(
        add(
            mul(records, REBUILD_BYTES_PER_SOURCE_RECORD)?,
            mul(source_bytes, 3)?,
        )?,
        REBUILD_FIXED_BYTES,
    )
}

pub(super) fn encode(
    loaded: &Loaded,
    context: &Context<'_>,
    max_bytes: u64,
) -> Result<Vec<u8>, String> {
    let length = encoded_bytes(loaded)?;
    if add(length, 112)? > max_bytes {
        return Err(
            "complete source snapshot exceeds byte admission before allocation or publication"
                .into(),
        );
    }
    let mut body = Vec::new();
    body.try_reserve_exact(usize::try_from(length).map_err(display)?)
        .map_err(display)?;
    body.extend_from_slice(MAGIC);
    for digest in [
        context.source_id(),
        loaded.source_binding,
        crate::stored::calendar_policy_digest_v2(),
    ] {
        body.extend_from_slice(&digest);
    }
    let reference = loaded.reference();
    for value in [
        vocab::VOCAB_VERSION,
        reference.schema,
        reference.eligibility_policy,
        reference.gap_overlay_policy,
        reference.swept_series_calendar_policy,
    ] {
        body.extend_from_slice(&value.to_le_bytes());
    }
    for value in [loaded.first_day, loaded.last_day] {
        body.extend_from_slice(&value.to_le_bytes());
    }
    for value in [
        context.limits.programs,
        context.limits.records,
        context.limits.bytes,
    ] {
        body.extend_from_slice(&value.to_le_bytes());
    }
    for text in [
        loaded.vendor.as_str(),
        &loaded.data.signal.key.to_string(),
        &loaded.rung,
        &loaded.commit,
    ] {
        put_text(&mut body, text);
    }
    put_count(&mut body, vocab::table::TABLE.len());
    for name in &vocab::table::TABLE {
        body.extend_from_slice(&name.index.to_le_bytes());
        put_text(&mut body, name.name);
    }
    put_count(&mut body, reference.excluded_ist_days.len());
    for day in reference.excluded_ist_days {
        body.extend_from_slice(&day.to_le_bytes());
    }
    put_candles(&mut body, &loaded.data.signal.bars);
    body.push(u8::from(loaded.data.execution.is_some()));
    put_candles(
        &mut body,
        loaded
            .data
            .execution
            .as_ref()
            .map_or(&[], |value| value.bars.as_slice()),
    );
    put_candles(&mut body, &loaded.data.exact_minute.bars);
    put_candles(&mut body, &loaded.data.daily.bars);
    put_count(&mut body, reference.eligibility.len());
    body.extend_from_slice(reference.eligibility);
    if body.len() as u64 != length {
        return Err("source snapshot encoder length differs from its prior admission".into());
    }
    Ok(body)
}
fn put_count(out: &mut Vec<u8>, count: usize) {
    out.extend_from_slice(&(count as u64).to_le_bytes());
}
fn put_text(out: &mut Vec<u8>, text: &str) {
    put_count(out, text.len());
    out.extend_from_slice(text.as_bytes());
}
fn put_candles(out: &mut Vec<u8>, bars: &[Candle]) {
    put_count(out, bars.len());
    for bar in bars {
        for value in [
            bar.ts_micros,
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume,
            bar.open_interest,
        ] {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
}

// A read-only first pass validates all extents and aggregate buffers before any
// text, candle, reference, expression or evaluator allocation is attempted.
fn admit(body: &[u8], bounds: ReadBounds) -> Result<(), String> {
    let mut read = Read::new(body);
    read.take(164)?;
    for _ in 0..4 {
        read.text()?;
    }
    let names = read.u64()?;
    if names == 0 || names > u64::from(vocab::ConditionMask::BITS) {
        return Err("source vocabulary extent exceeds the fixed mask".into());
    }
    for _ in 0..names {
        read.u16()?;
        read.text()?;
    }
    let excluded = read.u64()?;
    read.take(mul(excluded, 8)?)?;
    let signal = read.candle_extent()?;
    read.take(1)?;
    let execution = read.candle_extent()?;
    let minute = read.candle_extent()?;
    let daily = read.candle_extent()?;
    let eligibility = read.u64()?;
    read.take(eligibility)?;
    read.finished()?;
    let records = add(add(signal, execution)?, add(minute, daily)?)?;
    if records > bounds.source_records || eligibility != daily {
        return Err("source records or daily eligibility exceed independent admission".into());
    }
    let memory = rebuild_bytes(add(body.len() as u64, 112)?, records)?;
    if memory > bounds.memory_bytes {
        return Err("source reconstruction buffers exceed independent memory admission".into());
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "Canonical field order stays adjacent to the immutable V1 snapshot encoder"
)]
pub(super) fn decode(body: &[u8], bounds: ReadBounds) -> Result<Image, String> {
    admit(body, bounds)?;
    let mut read = Read::new(body);
    if read.take(8)? != MAGIC {
        return Err("original source snapshot version is unsupported".into());
    }
    let source_id = read.digest()?;
    let source_binding = read.digest()?;
    let calendar = read.digest()?;
    let vocabulary_version = read.u32()?;
    if vocabulary_version != vocab::VOCAB_VERSION {
        return Err("original vocabulary version has no compatible native identity reconstructor in this build".into());
    }
    let reference_versions = [read.u32()?, read.u32()?, read.u32()?, read.u32()?];
    let first_day = read.i64()?;
    let last_day = read.i64()?;
    let limits = Limits {
        programs: read.u64()?,
        records: read.u64()?,
        bytes: read.u64()?,
    };
    super::super::validate_limits(limits)?;
    let feed = crate::parse_vendor(read.text()?)?;
    let canonical_key = read.text()?;
    let key = crate::stored::swept_index(
        canonical_key
            .strip_prefix("NSE-")
            .ok_or("source instrument must be canonical NSE")?,
    )?;
    let timeframe = owned(read.text()?)?;
    super::super::validate_scope(canonical_key, &timeframe)?;
    let commit = owned(read.text()?)?;
    if first_day > last_day
        || commit.is_empty()
        || [source_id, source_binding, calendar].contains(&[0; 32])
    {
        return Err("original source identity metadata is empty or out of order".into());
    }
    let names_count = usize::try_from(read.u64()?).map_err(display)?;
    let mut names = Vec::new();
    names.try_reserve_exact(names_count).map_err(display)?;
    let mut previous = None;
    for _ in 0..names_count {
        let bit = read.u16()?;
        let text = read.text()?;
        if u32::from(bit) >= vocab::ConditionMask::BITS
            || previous.is_some_and(|value| value >= bit)
            || text.is_empty()
        {
            return Err(
                "original condition names must be ordered, unique and within the fixed mask".into(),
            );
        }
        names.push(ConditionName {
            bit,
            name: owned(text)?,
        });
        previous = Some(bit);
    }
    let excluded_count = usize::try_from(read.u64()?).map_err(display)?;
    let mut excluded_days = Vec::new();
    excluded_days
        .try_reserve_exact(excluded_count)
        .map_err(display)?;
    for _ in 0..excluded_count {
        excluded_days.push(read.i64()?);
    }
    // The original policy order is an identity term and is not chronological.
    // Preserve it verbatim; sorting would produce a different native source.
    let signal = read.candles()?;
    let separate_execution = read.take(1)?;
    let execution = read.candles()?;
    let execution = match separate_execution {
        [0] if execution.is_empty() && timeframe == "1min" => None,
        [1] if !execution.is_empty() => Some(execution),
        _ => return Err("original source execution alias is invalid".into()),
    };
    let minute = read.candles()?;
    let daily = read.candles()?;
    let count = read.u64()?;
    let raw = read.take(count)?;
    if raw.iter().any(|value| *value > 1) || raw.len() != daily.len() {
        return Err("original daily eligibility differs from its source extent".into());
    }
    let mut eligibility = Vec::new();
    eligibility.try_reserve_exact(raw.len()).map_err(display)?;
    eligibility.extend_from_slice(raw);
    read.finished()?;
    Ok(Image {
        source_id,
        source_binding,
        calendar,
        vocabulary_version,
        reference_versions,
        first_day,
        last_day,
        limits,
        feed,
        key,
        timeframe,
        commit,
        names,
        excluded_days,
        signal,
        execution,
        minute,
        daily,
        eligibility,
    })
}

impl Image {
    pub(super) fn execution(&self) -> &[Candle] {
        self.execution.as_deref().unwrap_or(&self.signal)
    }
    pub(super) fn column(&self) -> Result<Column, String> {
        let mut reference = Vec::new();
        reference
            .try_reserve_exact(self.daily.len())
            .map_err(display)?;
        for (bar, flag) in self.daily.iter().zip(&self.eligibility) {
            reference.push(
                DailyReference::new(
                    *bar,
                    if *flag == 1 {
                        DailyEligibility::Eligible
                    } else {
                        DailyEligibility::Excluded
                    },
                )
                .map_err(|why| format!("original daily source refused: {why:?}"))?,
            );
        }
        let seconds =
            u32::try_from(crate::stored::rung_length_micros(&self.timeframe)? / 1_000_000)
                .map_err(display)?;
        let (column, _) = crate::candidate_universe::build_candidate_columns(
            &self.signal,
            &reference,
            &self.minute,
            self.execution(),
            seconds,
            &crate::candidate_universe::CandidateEvaluationInputsV1 {
                widths: indicators::evaluator::Widths::pinned()
                    .map_err(|why| format!("source evaluator widths: {why:?}"))?,
                availability: crate::stored::vwap_availability(&self.key),
                thresholds: indicators::pattern::Thresholds::CLASSICAL,
            },
        )?;
        Ok(column)
    }
    pub(super) fn native_source<'a>(&'a self, column: &'a Column) -> Result<Source<'a>, String> {
        let [
            schema,
            eligibility_policy,
            gap_overlay_policy,
            swept_series_calendar_policy,
        ] = self.reference_versions;
        Ok(Source {
            series: runner::exit_grid_policy::ExecutionSeriesV1::new(
                &self.key,
                self.feed.as_str(),
                &self.commit,
                self.calendar,
                self.execution(),
            )
            .map_err(display)?,
            signal_bars: &self.signal,
            signal_column: column,
            minute_context: &self.minute,
            daily_reference: DailyReferenceBinding {
                daily_bars: &self.daily,
                eligibility: &self.eligibility,
                schema,
                eligibility_policy,
                gap_overlay_policy,
                excluded_ist_days: &self.excluded_days,
                daily_integrity: ReferenceIntegrity::ChecksumReceiptV1(self.source_binding),
                minute_integrity: ReferenceIntegrity::ChecksumReceiptV1(self.source_binding),
                swept_series_calendar_policy,
            },
            timeframe: &self.timeframe,
            first_day: self.first_day,
            last_day: self.last_day,
            params: Params {
                min_hits: 0,
                ceiling: self.limits.records,
                pair_budget: self.limits.programs,
                policy: 1,
            },
        })
    }
    pub(super) fn metadata(
        &self,
        identity: [u8; 32],
        pin: [u8; 32],
        setting: usize,
        row: &Record,
        link: Link,
    ) -> Result<Metadata, String> {
        Ok(Metadata {
            catalog_identity: identity,
            catalog_completion: pin,
            setting: u64::try_from(setting).map_err(display)?,
            source_id: self.source_id,
            run_id: row.run_id(),
            source_context_identity: link.identity,
            source_context_completion: link.completion,
            feed: self.feed.as_str().to_owned(),
            instrument: self.key.to_string(),
            timeframe: self.timeframe.clone(),
            original_build_commit: self.commit.clone(),
            vocabulary_version: self.vocabulary_version,
            expression: row.program().to_string(),
            source_first_day: self.first_day,
            source_last_day: self.last_day,
            measurement_first_day: row.first_day(),
            measurement_last_day: row.last_day(),
        })
    }
    pub(super) fn check_trade_coordinates(&self, row: &Record) -> Result<(), String> {
        let length = crate::stored::rung_length_micros(&self.timeframe)?;
        for trade in row.trades() {
            let signal = at(&self.signal, trade.signal_bar)?;
            let entry = at(self.execution(), trade.entry_bar)?;
            let exit = at(self.execution(), trade.exit_bar)?;
            let stop = if row.direction() == Direction::Long {
                signal.low
            } else {
                signal.high
            };
            if signal.ts_micros != trade.signal_micros
                || signal.ts_micros.checked_add(length) != Some(trade.signal_close_micros)
                || entry.ts_micros != trade.entry_micros
                || entry.open != trade.entry_paisa
                || exit.ts_micros != trade.exit_bar_micros
                || stop != trade.stop_paisa
                || !(exit.low..=exit.high).contains(&trade.optimistic_exit_paisa)
                || !(exit.low..=exit.high).contains(&trade.pessimistic_exit_paisa)
            {
                return Err(
                    "saved trade coordinates or prices differ from original source candles".into(),
                );
            }
            if trade.exit_reason == ExitReason::Forced1510
                && (exit.close != trade.optimistic_exit_paisa
                    || exit.close != trade.pessimistic_exit_paisa)
            {
                return Err(
                    "saved forced exit differs from the original exact closing print".into(),
                );
            }
        }
        Ok(())
    }
}
fn at(bars: &[Candle], index: u64) -> Result<&Candle, String> {
    bars.get(usize::try_from(index).map_err(display)?)
        .ok_or_else(|| "saved trade is outside original source extent".into())
}
fn add(left: u64, right: u64) -> Result<u64, String> {
    left.checked_add(right)
        .ok_or_else(|| "source byte/record extent overflow".into())
}
fn mul(left: u64, right: u64) -> Result<u64, String> {
    left.checked_mul(right)
        .ok_or_else(|| "source byte/record extent overflow".into())
}
fn owned(text: &str) -> Result<String, String> {
    let mut value = String::new();
    value.try_reserve_exact(text.len()).map_err(display)?;
    value.push_str(text);
    Ok(value)
}

struct Read<'a> {
    body: &'a [u8],
    offset: usize,
}
impl<'a> Read<'a> {
    const fn new(body: &'a [u8]) -> Self {
        Self { body, offset: 0 }
    }
    fn take(&mut self, length: u64) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(usize::try_from(length).map_err(display)?)
            .ok_or("source read extent overflow")?;
        let bytes = self
            .body
            .get(self.offset..end)
            .ok_or("original source snapshot is truncated")?;
        self.offset = end;
        Ok(bytes)
    }
    fn digest(&mut self) -> Result<[u8; 32], String> {
        self.take(32)?.try_into().map_err(display)
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(display)?,
        ))
    }
    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(
            self.take(8)?.try_into().map_err(display)?,
        ))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(display)?,
        ))
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().map_err(display)?,
        ))
    }
    fn text(&mut self) -> Result<&'a str, String> {
        let length = self.u64()?;
        std::str::from_utf8(self.take(length)?).map_err(display)
    }
    fn candle_extent(&mut self) -> Result<u64, String> {
        let count = self.u64()?;
        self.take(mul(count, CANDLE_BYTES)?)?;
        Ok(count)
    }
    fn candles(&mut self) -> Result<Vec<Candle>, String> {
        let count = usize::try_from(self.u64()?).map_err(display)?;
        let mut bars = Vec::new();
        bars.try_reserve_exact(count).map_err(display)?;
        let mut previous = None;
        for _ in 0..count {
            let bar = Candle::new(
                self.i64()?,
                self.i64()?,
                self.i64()?,
                self.i64()?,
                self.i64()?,
                self.i64()?,
                self.i64()?,
            );
            bar.check()
                .map_err(|why| format!("original source candle is malformed: {why:?}"))?;
            if previous.is_some_and(|time| time >= bar.ts_micros) {
                return Err("original source candles must be unique and increasing".into());
            }
            previous = Some(bar.ts_micros);
            bars.push(bar);
        }
        Ok(bars)
    }
    fn finished(&self) -> Result<(), String> {
        if self.offset == self.body.len() {
            Ok(())
        } else {
            Err("original source snapshot has trailing bytes".into())
        }
    }
}

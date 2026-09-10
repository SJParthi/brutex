//! Exact multi-month inputs for the ordinary historical audit kernel.
//! Source months are audited once, decoded from retained handles, and joined by
//! the existing causal calendar converters. All requested months are required.
use std::path::Path;

use brutex_core::{blake3::Hasher, instrument::InstrumentKey, vendor::Vendor};
use store::path::{Timeframe, YearMonth};

use super::{copy_loaded, error};
use crate::checksum_receipts::{AdmittedMonth, MonthRequest, Receipt};
use crate::stored::{self, Loaded, Span};

/// Physical ceilings bound each complete source audit and summed unique records.
#[derive(Clone, Copy)]
pub(crate) struct RangeRequest<'a> {
    pub store_root: &'a Path,
    pub vendor: Vendor,
    pub underlying: &'a str,
    pub rung: &'a str,
    pub from: (u16, u8),
    pub to: (u16, u8),
    pub receipt_root: &'a Path,
    pub max_bytes: u64,
    pub max_records: u64,
}

/// Decoded ordinary-kernel inputs; possessing these alone is not admission.
pub(crate) struct RangeData {
    pub signal: Span,
    pub execution: Option<Span>,
    pub daily: stored::DailyContext,
    pub exact_minute: stored::ExactMinuteContext,
}

/// Source and immutable relationship authority retained until terminal sealing.
pub(crate) struct RangeGuard {
    sources: Vec<AdmittedMonth>,
    bindings: Vec<Receipt>,
    identity: [u8; 32],
    records: u64,
}

impl RangeGuard {
    pub(crate) fn require_current(&self) -> Result<(), String> {
        for binding in &self.bindings {
            binding.require_current()?;
        }
        for source in &self.sources {
            source.require_current()?;
        }
        for binding in &self.bindings {
            binding.require_current()?;
        }
        Ok(())
    }

    pub(crate) fn bind_digest(&self, executed: [u8; 32]) -> [u8; 32] {
        let mut hash = Hasher::new();
        hash.update(b"brutex-stored-checksum-admitted-span-v1\0");
        hash.update(&executed);
        hash.update(&self.identity);
        hash.finalize()
    }

    pub(crate) fn note(&self) -> String {
        format!(
            "STRICT HISTORICAL SPAN CHECKSUMS V1 · {} exact month readers held · {} raw source records · {} chronological signal months\nAll requested source months and preceding minute/daily context are required; immutable role links and source generations are retained through terminal sealing. Cold audits and full-span reads are linear. This is checksum integrity, not vendor truth, calendar completeness or institutional policy admission.\nspan input binding {}\n",
            self.sources.len(),
            self.records,
            self.bindings.len(),
            crate::identity_hex(&self.identity)
        )
    }
}

/// Inputs are split by ownership so moving bars never drops their source locks.
pub(crate) struct RangeInputs {
    data: RangeData,
    guard: RangeGuard,
}

impl RangeInputs {
    pub(crate) fn load(request: RangeRequest<'_>) -> Result<Self, String> {
        Self::load_bounded(request, [u64::MAX; 3])
    }

    /// Independent raw signal/minute/daily ceilings are checked before decode.
    pub(crate) fn load_bounded(
        request: RangeRequest<'_>,
        limits: [u64; 3],
    ) -> Result<Self, String> {
        let mut loader = Loader::new(request)?;
        if limits.contains(&0) {
            return Err("audited range requires positive per-role record ceilings".to_owned());
        }
        loader.limits = limits;
        let data = loader.load()?;
        loader.require_current()?;
        let mut bindings = Vec::new();
        bindings
            .try_reserve_exact(loader.roles.len())
            .map_err(error)?;
        let mut identity = [0; 32];
        for (ordinal, (when, roles)) in loader.roles.iter().enumerate() {
            let (binding, next) = crate::checksum_receipts::publish_span_binding(
                request.receipt_root,
                identity,
                u64::try_from(ordinal).map_err(error)?,
                *when,
                roles,
            )?;
            bindings.push(binding);
            identity = next;
        }
        let guard = RangeGuard {
            sources: loader.sources,
            bindings,
            identity,
            records: loader.records,
        };
        guard.require_current()?;
        Ok(Self { data, guard })
    }

    pub(crate) fn into_parts(self) -> (RangeData, RangeGuard) {
        (self.data, self.guard)
    }
}

type MonthRoles = ((u16, u8), [[u8; 32]; 6]);

struct Loader<'a> {
    request: RangeRequest<'a>,
    key: InstrumentKey,
    timeframe: Timeframe,
    months: Vec<(u16, u8)>,
    sources: Vec<AdmittedMonth>,
    records: u64,
    roles: Vec<MonthRoles>,
    limits: [u64; 3],
    counts: [u64; 3],
}

impl<'a> Loader<'a> {
    fn new(request: RangeRequest<'a>) -> Result<Self, String> {
        if request.max_bytes == 0 || request.max_records == 0 {
            return Err(
                "audited range requires positive byte and total raw-record ceilings".to_owned(),
            );
        }
        crate::swept_rung(request.rung)?;
        YearMonth::new(request.from.0, request.from.1).map_err(error)?;
        YearMonth::new(request.to.0, request.to.1).map_err(error)?;
        let months = stored::months_between(request.from, request.to)?;
        stored::previous_month(request.from)?;
        let key = stored::swept_index(request.underlying)?;
        let timeframe = stored::rung(request.rung)?;
        // The existing 1,200-month limit applies before allocating file guards.
        // Each month contributes two reference files and optionally one signal
        // file; the preceding context adds exactly two unique files.
        let source_count = months
            .len()
            .checked_add(1)
            .and_then(|count| count.checked_mul(2))
            .and_then(|count| {
                count.checked_add(if timeframe == Timeframe::MINUTE_1 {
                    0
                } else {
                    months.len()
                })
            })
            .ok_or("audited range source-file count overflow")?;
        let mut sources = Vec::new();
        sources.try_reserve_exact(source_count).map_err(error)?;
        let mut roles = Vec::new();
        roles.try_reserve_exact(months.len()).map_err(error)?;
        Ok(Self {
            request,
            key,
            timeframe,
            months,
            sources,
            records: 0,
            roles,
            limits: [u64::MAX; 3],
            counts: [0; 3],
        })
    }

    fn load(&mut self) -> Result<RangeData, String> {
        let prior = stored::previous_month(self.request.from)?;
        let (prior_minute, mut prior_minute_id) = self.one(Timeframe::MINUTE_1, prior)?;
        let (prior_daily, mut prior_daily_id) = self.one(Timeframe::DAY_1, prior)?;
        let mut minute = Builder::from_loaded(prior_minute);
        let mut daily = Builder::from_loaded(prior_daily);
        let mut signal = Builder::default();
        let mut execution = Builder::default();
        // Fixed ordinal access avoids cloning the range list or searching for
        // adjacent reference months. Each unique physical source is opened once.
        for index in 0..self.months.len() {
            let when = *self
                .months
                .get(index)
                .ok_or("audited range month ordinal")?;
            let (current_minute, minute_id) = self.one(Timeframe::MINUTE_1, when)?;
            crate::validate_one_minute_execution(&current_minute.bars)?;
            let signal_id = if self.timeframe == Timeframe::MINUTE_1 {
                signal.append(copy_loaded(&current_minute)?)?;
                minute_id
            } else {
                execution.append(copy_loaded(&current_minute)?)?;
                let (current_signal, id) = self.one(self.timeframe, when)?;
                signal.append(current_signal)?;
                id
            };
            let (current_daily, daily_id) = self.one(Timeframe::DAY_1, when)?;
            minute.append(current_minute)?;
            daily.append(current_daily)?;
            self.roles.push((
                when,
                [
                    signal_id,
                    minute_id,
                    prior_daily_id,
                    daily_id,
                    prior_minute_id,
                    minute_id,
                ],
            ));
            prior_minute_id = minute_id;
            prior_daily_id = daily_id;
        }
        let signal = signal.finish()?;
        let execution = if self.timeframe == Timeframe::MINUTE_1 {
            None
        } else {
            let span = execution.finish()?;
            crate::validate_one_minute_execution(&span.bars)?;
            Some(span)
        };
        let daily = stored::daily_context_from_span(daily.finish()?, &signal.bars)?;
        let exact_minute = stored::exact_minute_context_from_span(minute.finish()?, &signal.bars)?;
        Ok(RangeData {
            signal,
            execution,
            daily,
            exact_minute,
        })
    }

    fn one(&mut self, timeframe: Timeframe, when: (u16, u8)) -> Result<(Loaded, [u8; 32]), String> {
        let source = crate::checksum_receipts::audit_month(MonthRequest {
            store_root: self.request.store_root,
            receipt_root: self.request.receipt_root,
            vendor: self.request.vendor,
            key: &self.key,
            timeframe,
            month: YearMonth::new(when.0, when.1).map_err(error)?,
            max_bytes: self.request.max_bytes,
        })?;
        let records = source.evidence().header().n_valid;
        let total = self.records.checked_add(records)
            .filter(|total| *total <= self.request.max_records)
            .ok_or("audited range exceeds the explicit total unique raw-record ceiling before allocation")?;
        let active_roles = [
            timeframe == self.timeframe && when >= self.request.from,
            timeframe == Timeframe::MINUTE_1,
            timeframe == Timeframe::DAY_1,
        ];
        for ((count, limit), active) in self.counts.iter_mut().zip(self.limits).zip(active_roles) {
            if active {
                *count = count.checked_add(records).filter(|next| *next <= limit)
                    .ok_or("audited range exceeds a signal/minute/daily raw-record ceiling before allocation")?;
            }
        }
        let loaded = stored::decode_loaded(
            self.request.vendor,
            self.key,
            timeframe,
            when,
            records,
            |index| source.read_record(index),
        )?;
        source.require_current()?;
        let identity = source.receipt_identity();
        self.sources.push(source);
        self.records = total;
        Ok((loaded, identity))
    }

    fn require_current(&self) -> Result<(), String> {
        for source in &self.sources {
            source.require_current()?;
        }
        Ok(())
    }
}

#[derive(Default)]
struct Builder(Option<Span>);

impl Builder {
    fn from_loaded(loaded: Loaded) -> Self {
        Self(Some(Span {
            bars: loaded.bars,
            vendor: loaded.vendor,
            key: loaded.key,
            timeframe: loaded.timeframe,
            asked: 1,
            found: 1,
            missing: Vec::new(),
            excluded: loaded.excluded,
        }))
    }

    fn append(&mut self, loaded: Loaded) -> Result<(), String> {
        let Some(span) = &mut self.0 else {
            *self = Self::from_loaded(loaded);
            return Ok(());
        };
        if span.vendor != loaded.vendor
            || span.key != loaded.key
            || span.timeframe != loaded.timeframe
        {
            return Err("audited range refuses a cross-source month join".to_owned());
        }
        if let (Some(last), Some(first)) = (span.bars.last(), loaded.bars.first())
            && first.ts_micros <= last.ts_micros
        {
            return Err("audited range steps backward at its month boundary".to_owned());
        }
        span.bars
            .try_reserve_exact(loaded.bars.len())
            .map_err(error)?;
        span.bars.extend(loaded.bars);
        span.excluded.absorb(loaded.excluded);
        span.asked = span
            .asked
            .checked_add(1)
            .ok_or("audited span month count overflow")?;
        span.found = span.asked;
        Ok(())
    }

    fn finish(self) -> Result<Span, String> {
        self.0
            .ok_or_else(|| "audited range requires at least one exact month".to_owned())
    }
}

#[cfg(test)]
#[path = "audited_range_tests.rs"]
mod tests;

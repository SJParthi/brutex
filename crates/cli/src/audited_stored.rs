//! Strict monthly historical inputs decoded from retained checksum-audited handles.
//! The legacy loaded types and reference-integrity identity fields are unchanged.
use std::path::Path;

use brutex_core::blake3::Hasher;
use brutex_core::vendor::Vendor;
use store::path::{Timeframe, YearMonth};

use crate::checksum_receipts::{AdmittedMonth, MonthRequest};
use crate::stored::{self, Loaded, Span};

#[path = "audited_range.rs"]
mod range;
pub(crate) use range::{RangeData, RangeGuard, RangeInputs, RangeRequest};

/// Explicit one-month strict input request. Bounds are physical resource ceilings.
#[derive(Clone, Copy)]
pub(crate) struct Request<'a> {
    pub store_root: &'a Path,
    pub vendor: Vendor,
    pub underlying: &'a str,
    pub rung: &'a str,
    pub year: u16,
    pub month: u8,
    pub receipt_root: &'a Path,
    pub max_bytes: u64,
    pub max_records: u64,
}

/// Opaque source authority retained through the common sweep's final publication.
pub(crate) struct Inputs {
    data: crate::StoredMonthInputs,
    guards: Vec<AdmittedMonth>,
    // signal, execution, prior daily, current daily, prior minute, current minute.
    roles: [[u8; 32]; 6],
    records: u64,
    binding: crate::checksum_receipts::Receipt,
    binding_identity: [u8; 32],
}
impl Inputs {
    pub(crate) fn load(request: Request<'_>) -> Result<Self, String> {
        if request.max_records == 0 || request.max_bytes == 0 {
            return Err(
                "audited historical sweep requires positive byte and total source-record ceilings"
                    .to_owned(),
            );
        }
        crate::swept_rung(request.rung)?;
        let key = stored::swept_index(request.underlying)?;
        let timeframe = stored::rung(request.rung)?;
        let current = (request.year, request.month);
        YearMonth::new(current.0, current.1).map_err(error)?;
        let prior = stored::previous_month(current)?;
        let mut guards = Vec::new();
        guards.try_reserve_exact(5).map_err(error)?;
        let mut collector = Loader {
            request: &request,
            key: &key,
            guards,
            records: 0,
        };
        let (minute, minute_id) = collector.one(Timeframe::MINUTE_1, current)?;
        let (loaded, signal_id) = if timeframe == Timeframe::MINUTE_1 {
            (copy_loaded(&minute)?, minute_id)
        } else {
            collector.one(timeframe, current)?
        };
        crate::validate_one_minute_execution(&minute.bars)?;
        let execution_bars = if timeframe == Timeframe::MINUTE_1 {
            None
        } else {
            Some(copy_loaded(&minute)?)
        };
        let (prior_minute, prior_minute_id) = collector.one(Timeframe::MINUTE_1, prior)?;
        let (prior_daily, prior_daily_id) = collector.one(Timeframe::DAY_1, prior)?;
        let (current_daily, current_daily_id) = collector.one(Timeframe::DAY_1, current)?;
        let daily =
            stored::daily_context_from_span(join(prior_daily, current_daily)?, &loaded.bars)?;
        let exact_minute =
            stored::exact_minute_context_from_span(join(prior_minute, minute)?, &loaded.bars)?;
        let roles = [
            signal_id,
            minute_id,
            prior_daily_id,
            current_daily_id,
            prior_minute_id,
            minute_id,
        ];
        for guard in &collector.guards {
            guard.require_current()?;
        }
        let (binding, binding_identity) =
            crate::checksum_receipts::publish_binding(request.receipt_root, &roles)?;
        let inputs = Self {
            data: crate::StoredMonthInputs {
                loaded,
                execution_bars,
                daily,
                exact_minute,
            },
            guards: collector.guards,
            roles,
            records: collector.records,
            binding,
            binding_identity,
        };
        inputs.require_current()?;
        Ok(inputs)
    }
    pub(crate) const fn data(&self) -> &crate::StoredMonthInputs {
        &self.data
    }
    pub(crate) fn require_current(&self) -> Result<(), String> {
        self.binding.require_current()?;
        for guard in &self.guards {
            guard.require_current()?;
        }
        self.binding.require_current()
    }
    pub(crate) fn bind_digest(&self, executed: [u8; 32]) -> [u8; 32] {
        let mut hash = Hasher::new();
        hash.update(b"brutex-stored-checksum-admitted-inputs-v1\0");
        hash.update(&executed);
        for (role, receipt) in (0_u8..).zip(self.roles.iter()) {
            hash.update(&[role]);
            hash.update(receipt);
        }
        hash.finalize()
    }
    pub(crate) fn note(&self) -> String {
        use std::fmt::Write as _;
        let mut out = format!(
            "STRICT HISTORICAL INPUT CHECKSUMS V1 · {} exact month readers held · {} raw source records\nCold complete byte/CRC audits; fixed-block verified reads; source and receipt generations retained through this sweep. This does not attest vendor truth, institutional policy, or change legacy ReferenceIntegrity fields.\n",
            self.guards.len(),
            self.records
        );
        let _ = writeln!(
            out,
            "six-role input binding {}",
            crate::identity_hex(&self.binding_identity)
        );
        for (role, id) in [
            "signal",
            "execution",
            "prior daily",
            "current daily",
            "prior exact minute",
            "current exact minute",
        ]
        .iter()
        .zip(self.roles.iter())
        {
            let _ = writeln!(out, "{role} receipt {}", crate::identity_hex(id));
        }
        out
    }
}

struct Loader<'a> {
    request: &'a Request<'a>,
    key: &'a brutex_core::instrument::InstrumentKey,
    guards: Vec<AdmittedMonth>,
    records: u64,
}
impl Loader<'_> {
    fn one(&mut self, timeframe: Timeframe, when: (u16, u8)) -> Result<(Loaded, [u8; 32]), String> {
        let guard = crate::checksum_receipts::audit_month(MonthRequest {
            store_root: self.request.store_root,
            receipt_root: self.request.receipt_root,
            vendor: self.request.vendor,
            key: self.key,
            timeframe,
            month: YearMonth::new(when.0, when.1).map_err(error)?,
            max_bytes: self.request.max_bytes,
        })?;
        let records = guard.evidence().header().n_valid;
        let total=self.records.checked_add(records).filter(|n| *n<=self.request.max_records)
            .ok_or("audited historical inputs exceed the explicit total raw-record ceiling before allocation")?;
        let loaded = stored::decode_loaded(
            self.request.vendor,
            *self.key,
            timeframe,
            when,
            records,
            |index| guard.read_record(index),
        )?;
        guard.require_current()?;
        let identity = guard.receipt_identity();
        self.guards.push(guard);
        self.records = total;
        Ok((loaded, identity))
    }
}

fn copy_loaded(source: &Loaded) -> Result<Loaded, String> {
    let mut bars = Vec::new();
    bars.try_reserve_exact(source.bars.len()).map_err(error)?;
    bars.extend_from_slice(&source.bars);
    Ok(Loaded {
        bars,
        vendor: source.vendor,
        key: source.key,
        timeframe: source.timeframe,
        excluded: source.excluded,
    })
}

fn join(mut prior: Loaded, current: Loaded) -> Result<Span, String> {
    if prior.vendor != current.vendor
        || prior.key != current.key
        || prior.timeframe != current.timeframe
    {
        return Err("audited historical context refuses a cross-source month join".to_owned());
    }
    if let (Some(last), Some(first)) = (prior.bars.last(), current.bars.first())
        && first.ts_micros <= last.ts_micros
    {
        return Err("audited historical context steps backward at its month boundary".to_owned());
    }
    prior.bars.try_reserve(current.bars.len()).map_err(error)?;
    prior.bars.extend(current.bars);
    prior.excluded.absorb(current.excluded);
    Ok(Span {
        bars: prior.bars,
        vendor: prior.vendor,
        key: prior.key,
        timeframe: prior.timeframe,
        asked: 2,
        found: 2,
        missing: Vec::new(),
        excluded: prior.excluded,
    })
}
fn error(why: impl std::fmt::Display) -> String {
    why.to_string()
}

#[cfg(test)]
#[path = "audited_stored_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) use tests::with_warmed_store;

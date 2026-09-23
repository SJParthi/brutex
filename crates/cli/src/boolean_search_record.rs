//! Versioned durable search declaration and ordered batch transitions.
//! Old catalog/grammar/qualification formats are unchanged.
use crate::boolean_campaign::RungScope;
use crate::boolean_grammar_batch::{Batch, Budget};
use crate::boolean_qualification_plan::ObservedPlan;
use crate::boolean_qualified_journal::Link;
use brutex_core::blake3::{Hasher, hash};
use runner::expression::{ENCODED_LEN, Expression};
use vocab::expression_search::{CURSOR_BYTES, Cursor};

pub(crate) const NAMESPACE: &str = "boolean-qualified-search-v1";
const SPEC_BYTES: usize = 144 + CURSOR_BYTES;
const HEADER: usize = 152 + 8 * 168 + 1024;

/// The projection interpretation is durable; a newer reader preserves V1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProjectionRule {
    LegacyV1,
    SharedCeilingsV2,
    ScopedSharedCeilingsV3,
    IndexConsistencyV4,
}
impl ProjectionRule {
    pub(crate) const fn version(self) -> u8 {
        match self {
            Self::LegacyV1 => 1,
            Self::SharedCeilingsV2 => 2,
            Self::ScopedSharedCeilingsV3 => 3,
            Self::IndexConsistencyV4 => 4,
        }
    }
    pub(crate) const fn spec_magic(self) -> [u8; 8] {
        match self {
            Self::LegacyV1 => *b"BRBQSS01",
            Self::SharedCeilingsV2 => *b"BRBQSS02",
            Self::ScopedSharedCeilingsV3 => *b"BRBQSS03",
            Self::IndexConsistencyV4 => *b"BRBQSS04",
        }
    }
    pub(crate) const fn record_magic(self) -> [u8; 8] {
        match self {
            Self::LegacyV1 => *b"BRBQSR01",
            Self::SharedCeilingsV2 => *b"BRBQSR02",
            Self::ScopedSharedCeilingsV3 => *b"BRBQSR03",
            Self::IndexConsistencyV4 => *b"BRBQSR04",
        }
    }
    pub(crate) const fn summary_domain(self) -> &'static [u8] {
        match self {
            Self::LegacyV1 => b"brutex-search-wide-projection-v1\0",
            Self::SharedCeilingsV2 => b"brutex-search-wide-projection-v2\0",
            Self::ScopedSharedCeilingsV3 => b"brutex-search-wide-projection-v3\0",
            Self::IndexConsistencyV4 => b"brutex-search-wide-projection-v4\0",
        }
    }
    fn from_spec_magic(magic: [u8; 8]) -> Result<Self, String> {
        match &magic {
            b"BRBQSS01" => Ok(Self::LegacyV1),
            b"BRBQSS02" => Ok(Self::SharedCeilingsV2),
            b"BRBQSS03" => Ok(Self::ScopedSharedCeilingsV3),
            b"BRBQSS04" => Ok(Self::IndexConsistencyV4),
            _ => Err("search declaration format differs".into()),
        }
    }
    fn from_record_magic(magic: [u8; 8]) -> Result<Self, String> {
        match &magic {
            b"BRBQSR01" => Ok(Self::LegacyV1),
            b"BRBQSR02" => Ok(Self::SharedCeilingsV2),
            b"BRBQSR03" => Ok(Self::ScopedSharedCeilingsV3),
            b"BRBQSR04" => Ok(Self::IndexConsistencyV4),
            _ => Err("search record header or byte admission refused".into()),
        }
    }
    const fn spec_bytes(self) -> usize {
        SPEC_BYTES
            + match self {
                Self::ScopedSharedCeilingsV3 => 8,
                Self::IndexConsistencyV4 => 40,
                _ => 0,
            }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Spec {
    pub(crate) rungs: RungScope,
    pub(crate) projection_rule: ProjectionRule,
    pub(crate) sources: [[u8; 32]; 2],
    pub(crate) policy: [u8; 32],
    pub(crate) initial: [u8; CURSOR_BYTES],
    pub(crate) programs: u64,
    pub(crate) nodes: u64,
    pub(crate) bytes: u64,
    pub(crate) records: u64,
    pub(crate) alpha: u64,
}
impl Spec {
    pub(crate) fn identity(&self) -> [u8; 32] {
        hash(&self.encode())
    }
    pub(crate) fn budget(&self) -> Budget {
        Budget {
            programs: self.programs,
            nodes: self.nodes,
            bytes: self.bytes / 4,
        }
    }
    pub(crate) fn cursor(&self) -> Result<Cursor, String> {
        let cursor = Cursor::decode(&self.initial).map_err(debug)?;
        if cursor.initial_descriptor() != self.initial {
            return Err("search declaration is not an initial grammar cursor".into());
        }
        Ok(cursor)
    }
    pub(crate) fn validate(&self) -> Result<(), String> {
        self.cursor()?;
        if self.projection_rule != ProjectionRule::IndexConsistencyV4
            && ((self.projection_rule == ProjectionRule::ScopedSharedCeilingsV3)
                != (self.rungs != RungScope::ALL))
        {
            return Err("search timeframe scope and additive format differ".into());
        }
        if self.sources.contains(&[0; 32])
            || self.policy == [0; 32]
            || self.programs == 0
            || self.nodes == 0
            || self.nodes > self.records
            || self.records < 2
            || self.bytes < (HEADER + self.projection_rule.spec_bytes() + 96) as u64
            || self.bytes > isize::MAX as u64
        {
            return Err("search declaration has invalid source or physical admission".into());
        }
        runner::search_allocation_v1::allocate(0, 0, self.alpha).map_err(debug)?;
        Ok(())
    }
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.projection_rule.spec_bytes());
        out.extend_from_slice(&self.projection_rule.spec_magic());
        for source in self.sources {
            out.extend_from_slice(&source);
        }
        out.extend_from_slice(&self.policy);
        out.extend_from_slice(&self.initial);
        for value in [
            self.programs,
            self.nodes,
            self.bytes,
            self.records,
            self.alpha,
        ] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        if self.rungs != RungScope::ALL
            || self.projection_rule == ProjectionRule::IndexConsistencyV4
        {
            out.extend_from_slice(&u64::from(self.rungs.mask()).to_le_bytes());
        }
        if self.projection_rule == ProjectionRule::IndexConsistencyV4 {
            out.extend_from_slice(&crate::index_consistency::Policy::V1.digest());
        }
        out
    }
    fn decode(raw: &[u8]) -> Result<Self, String> {
        if raw.len() < SPEC_BYTES {
            return Err("search declaration format differs".into());
        }
        let projection_rule = ProjectionRule::from_spec_magic(field(raw, 0)?)?;
        if raw.len() != projection_rule.spec_bytes() {
            return Err("search declaration format differs".into());
        }
        let at = 104 + CURSOR_BYTES;
        if projection_rule == ProjectionRule::IndexConsistencyV4
            && field::<32>(raw, SPEC_BYTES + 8)? != crate::index_consistency::Policy::V1.digest()
        {
            return Err("search index consistency policy differs".into());
        }
        let spec = Self {
            rungs: if matches!(
                projection_rule,
                ProjectionRule::ScopedSharedCeilingsV3 | ProjectionRule::IndexConsistencyV4
            ) {
                RungScope::from_mask(u8::try_from(number(raw, SPEC_BYTES)?).map_err(display)?)?
            } else {
                RungScope::ALL
            },
            projection_rule,
            sources: [field(raw, 8)?, field(raw, 40)?],
            policy: field(raw, 72)?,
            initial: field(raw, 104)?,
            programs: number(raw, at)?,
            nodes: number(raw, at + 8)?,
            bytes: number(raw, at + 16)?,
            records: number(raw, at + 24)?,
            alpha: number(raw, at + 32)?,
        };
        spec.validate()?;
        Ok(spec)
    }
}

/// Saved per-timeframe comparison. Child bodies are authenticated by detail reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    /// Exact existing qualification child and completion pin.
    pub child: Link,
    /// Every retained coordinate, including zero/refused cases.
    pub count: u64,
    /// Admitted, rejected, unmeasured and refused counts.
    pub counts: [u64; 4],
    /// Ordered complete projection digest, including every reason and value.
    pub projection: [u8; 32],
    /// Exact immutable search-wide allocation digest.
    pub allocation: [u8; 32],
}
impl Summary {
    pub(crate) const EMPTY: Self = Self {
        child: Link {
            identity: [0; 32],
            pin: [0; 32],
        },
        count: 0,
        counts: [0; 4],
        projection: [0; 32],
        allocation: [0; 32],
    };
}

pub(crate) struct Record {
    pub(crate) previous: Option<(u64, [u8; 32])>,
    pub(crate) spec: Spec,
    pub(crate) ordinal: u64,
    /// 0 reserved before pricing; 1 complete; 2 recorded refusal, same reservation.
    pub(crate) phase: u64,
    pub(crate) batch: Batch,
    pub(crate) plan: Vec<u8>,
    pub(crate) campaign: Link,
    pub(crate) summaries: [Summary; 8],
    pub(crate) reason: String,
}
impl Record {
    pub(crate) fn validate(&self) -> Result<(), String> {
        self.spec.validate()?;
        if self.phase > 2
            || self.reason.len() > 1024
            || (self.phase == 2) == self.reason.is_empty()
            || !self.batch.uses_budget(self.spec.programs, self.spec.nodes)
        {
            return Err("search phase, reason or grammar allowance differs".into());
        }
        for rung in 0..8 {
            runner::search_allocation_v1::allocate(self.ordinal, rung, self.spec.alpha)
                .map_err(debug)?;
        }
        if self.batch.programs().is_empty() {
            if !self.plan.is_empty()
                || self.campaign != Summary::EMPTY.child
                || self.summaries != [Summary::EMPTY; 8]
            {
                return Err(
                    "node-only search batch carries invented qualification evidence".into(),
                );
            }
        } else {
            let plan = self.observed_plan()?;
            if plan.source_descriptors() != self.spec.sources
                || (self.spec.projection_rule == ProjectionRule::IndexConsistencyV4
                    && plan.index_policy() != Some(crate::index_consistency::Policy::V1.digest()))
                || plan.rungs() != self.spec.rungs
                || plan.policy_digest() != self.spec.policy
                || plan.program_digest()
                    != crate::boolean_campaign::program_digest(self.batch.programs())
                || plan.program_count() != self.batch.programs().len() as u64
                || plan.source_limits() != [self.spec.bytes, self.spec.records]
                || plan.alpha_ceilings().into_iter().min() != Some(self.spec.alpha)
            {
                return Err(
                    "search batch qualification plan differs from its frozen declaration".into(),
                );
            }
            if self.phase == 1 {
                if self.campaign.identity
                    != crate::boolean_qualified_journal::identity_for_rungs(
                        plan.descriptor(),
                        *plan.units(),
                        plan.rungs(),
                    )
                    || self.campaign.pin == [0; 32]
                {
                    return Err("search completion names a foreign qualification campaign".into());
                }
                for (rung, summary) in self.summaries.iter().enumerate() {
                    if !self.spec.rungs.contains(rung) {
                        if *summary != Summary::EMPTY {
                            return Err("unselected search timeframe carries a result".into());
                        }
                        continue;
                    }
                    let sum = summary
                        .counts
                        .iter()
                        .try_fold(0_u64, |a, b| a.checked_add(*b))
                        .ok_or("search count overflow")?;
                    if summary.child.identity == [0; 32]
                        || summary.child.pin == [0; 32]
                        || summary.projection == [0; 32]
                        || summary.count != sum
                        || summary.allocation
                            != runner::search_allocation_v1::allocate(
                                self.ordinal,
                                rung as u64,
                                self.spec.alpha,
                            )
                            .map_err(debug)?
                            .digest()
                    {
                        return Err("search rung counts, binding or exact allocation differ".into());
                    }
                }
            } else if self.campaign != Summary::EMPTY.child || self.summaries != [Summary::EMPTY; 8]
            {
                return Err("unfinished search batch carries completed results".into());
            }
        }
        Ok(())
    }
    pub(crate) fn observed_plan(&self) -> Result<ObservedPlan, String> {
        ObservedPlan::decode(&self.plan, self.spec.bytes, self.spec.records)
    }
    pub(crate) fn binding(&self) -> Result<[u8; 32], String> {
        let mut hash = Hasher::new();
        hash.update(b"brutex-qualified-search-batch-binding-v1\0");
        hash.update(&self.spec.identity());
        hash.update(&self.ordinal.to_le_bytes());
        hash.update(&self.batch.encode()?);
        hash.update(&self.plan);
        Ok(hash.finalize())
    }
    pub(crate) fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let batch = self.batch.encode()?;
        let size = HEADER
            .checked_add(self.spec.projection_rule.spec_bytes())
            .and_then(|n| n.checked_add(batch.len()))
            .and_then(|n| n.checked_add(self.plan.len()))
            .ok_or("search record size overflow")?;
        if (size as u64)
            .checked_add(96)
            .is_none_or(|n| n > self.spec.bytes)
        {
            return Err("search record exceeds complete byte admission".into());
        }
        let mut out = Vec::new();
        out.try_reserve_exact(size).map_err(display)?;
        out.extend_from_slice(&self.spec.projection_rule.record_magic());
        let previous = self.previous.unwrap_or((0, [0; 32]));
        out.extend_from_slice(&previous.0.to_le_bytes());
        out.extend_from_slice(&previous.1);
        for n in [
            self.ordinal,
            self.phase,
            batch.len() as u64,
            self.plan.len() as u64,
            self.reason.len() as u64,
        ] {
            out.extend_from_slice(&n.to_le_bytes());
        }
        out.extend_from_slice(&self.campaign.identity);
        out.extend_from_slice(&self.campaign.pin);
        for summary in self.summaries {
            out.extend_from_slice(&summary.child.identity);
            out.extend_from_slice(&summary.child.pin);
            out.extend_from_slice(&summary.count.to_le_bytes());
            for n in summary.counts {
                out.extend_from_slice(&n.to_le_bytes());
            }
            out.extend_from_slice(&summary.projection);
            out.extend_from_slice(&summary.allocation);
        }
        out.extend_from_slice(self.reason.as_bytes());
        out.resize(HEADER, 0);
        out.extend_from_slice(&self.spec.encode());
        out.extend_from_slice(&batch);
        out.extend_from_slice(&self.plan);
        Ok(out)
    }
    pub(crate) fn decode(raw: &[u8], max_bytes: u64) -> Result<Self, String> {
        Self::decode_bounded(raw, max_bytes, max_bytes)
    }
    pub(crate) fn declaration(raw: &[u8], max_bytes: u64) -> Result<Spec, String> {
        Self::require_header(raw, max_bytes)?;
        let spec_bytes = ProjectionRule::from_record_magic(field(raw, 0)?)?.spec_bytes();
        Spec::decode(
            raw.get(HEADER..HEADER + spec_bytes)
                .ok_or("search declaration absent")?,
        )
    }
    fn require_header(raw: &[u8], max_bytes: u64) -> Result<(), String> {
        if raw.len() < HEADER + SPEC_BYTES || raw.len() as u64 > max_bytes {
            return Err("search record header or byte admission refused".into());
        }
        let rule = ProjectionRule::from_record_magic(field(raw, 0)?)?;
        if raw.len() < HEADER + rule.spec_bytes() || field::<8>(raw, HEADER)? != rule.spec_magic() {
            return Err("search record and declaration projection versions differ".into());
        }
        Ok(())
    }
    pub(crate) fn predecessor(raw: &[u8]) -> Result<Option<(u64, [u8; 32])>, String> {
        match (number(raw, 8)?, field::<32>(raw, 16)?) {
            (0, p) if p == [0; 32] => Ok(None),
            (n, p) if n != 0 && p != [0; 32] => Ok(Some((n, p))),
            _ => Err("search predecessor is partial".into()),
        }
    }
    pub(crate) fn decode_bounded(
        raw: &[u8],
        max_bytes: u64,
        max_nodes: u64,
    ) -> Result<Self, String> {
        Self::require_header(raw, max_bytes)?;
        let spec_bytes = ProjectionRule::from_record_magic(field(raw, 0)?)?.spec_bytes();
        let previous = match (number(raw, 8)?, field::<32>(raw, 16)?) {
            (0, p) if p == [0; 32] => None,
            (0, _) => return Err("search predecessor is partial".into()),
            (n, p) if p != [0; 32] => Some((n, p)),
            _ => return Err("search predecessor pin absent".into()),
        };
        let batch_len = usize::try_from(number(raw, 64)?).map_err(display)?;
        let plan_len = usize::try_from(number(raw, 72)?).map_err(display)?;
        let reason_len = usize::try_from(number(raw, 80)?).map_err(display)?;
        if reason_len > 1024
            || (HEADER + spec_bytes)
                .checked_add(batch_len)
                .and_then(|n| n.checked_add(plan_len))
                != Some(raw.len())
        {
            return Err("search record counts or exact length differ".into());
        }
        let mut summaries = [Summary::EMPTY; 8];
        for (rung, summary) in summaries.iter_mut().enumerate() {
            let at = 152 + rung * 168;
            *summary = Summary {
                child: Link {
                    identity: field(raw, at)?,
                    pin: field(raw, at + 32)?,
                },
                count: number(raw, at + 64)?,
                counts: [
                    number(raw, at + 72)?,
                    number(raw, at + 80)?,
                    number(raw, at + 88)?,
                    number(raw, at + 96)?,
                ],
                projection: field(raw, at + 104)?,
                allocation: field(raw, at + 136)?,
            };
        }
        let reason_at = 152 + 8 * 168;
        let reason = std::str::from_utf8(
            raw.get(reason_at..reason_at + reason_len)
                .ok_or("search reason absent")?,
        )
        .map_err(display)?
        .to_owned();
        if raw
            .get(reason_at + reason_len..HEADER)
            .is_none_or(|p| p.iter().any(|b| *b != 0))
        {
            return Err("search reason padding differs".into());
        }
        let spec = Spec::decode(
            raw.get(HEADER..HEADER + spec_bytes)
                .ok_or("search declaration absent")?,
        )?;
        if spec.nodes > max_nodes {
            return Err("search grammar replay exceeds independent node admission".into());
        }
        let reservation = spec
            .programs
            .checked_mul((size_of::<Expression>() + ENCODED_LEN) as u64)
            .and_then(|n| n.checked_add(raw.len() as u64))
            .and_then(|n| n.checked_add((2 * CURSOR_BYTES) as u64));
        if reservation.is_none_or(|n| n > max_bytes) {
            return Err(
                "search complete program reservation exceeds independent decode-byte admission"
                    .into(),
            );
        }
        let batch_at = HEADER + spec_bytes;
        let batch = Batch::decode(
            raw.get(batch_at..batch_at + batch_len)
                .ok_or("search batch absent")?,
            spec.budget().bytes.min(max_bytes),
            spec.nodes,
        )?;
        let record = Self {
            previous,
            spec,
            ordinal: number(raw, 48)?,
            phase: number(raw, 56)?,
            batch,
            plan: raw
                .get(batch_at + batch_len..)
                .ok_or("search plan absent")?
                .to_vec(),
            campaign: Link {
                identity: field(raw, 88)?,
                pin: field(raw, 120)?,
            },
            summaries,
            reason,
        };
        record.validate()?;
        Ok(record)
    }
}
pub(crate) fn field<const N: usize>(raw: &[u8], at: usize) -> Result<[u8; N], String> {
    raw.get(at..at.saturating_add(N))
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| "search field is truncated".into())
}
pub(crate) fn number(raw: &[u8], at: usize) -> Result<u64, String> {
    Ok(u64::from_le_bytes(field(raw, at)?))
}
pub(crate) fn display(error: impl std::fmt::Display) -> String {
    error.to_string()
}
pub(crate) fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

#[cfg(test)]
#[path = "boolean_search_record_tests.rs"]
pub(crate) mod tests;

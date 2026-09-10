//! Fixed versioned fields and bounded variable fold/split extents.
use super::{Body, Bounds, Facts, Fold, Link, Row, Split, Statistics, display, identity};
use runner::admission::research_projection::RESEARCH_ADMISSION_PROJECTION_BYTES_V1;
use runner::admission::{ADMISSION_POLICY_CANONICAL_LEN_V1, AdmissionPolicyV1};
use runner::signal_candle_stop::Policy;

const MAGIC: &[u8; 8] = b"BRISQF01";
const ROW_BYTES: usize = 128 + RESEARCH_ADMISSION_PROJECTION_BYTES_V1 + 8 + 96 + 8;
const FOLD_BYTES: usize = 64;
const SPLIT_BYTES: usize = 64;
const HEADER_BYTES: usize = 8
    + 32
    + Policy::BYTE_LEN
    + crate::index_consistency::Policy::BYTE_LEN
    + 160
    + ADMISSION_POLICY_CANONICAL_LEN_V1
    + 40
    + 56
    + 80
    + 32
    + 40
    + 72
    + 8;

fn encoded_length(value: &Body) -> Result<usize, String> {
    let variable = value
        .rows
        .iter()
        .try_fold(0_usize, |n, row| {
            n.checked_add(ROW_BYTES).and_then(|v| {
                row.folds
                    .len()
                    .checked_mul(FOLD_BYTES)
                    .and_then(|f| v.checked_add(f))
            })
        })
        .and_then(|n| {
            value
                .statistics
                .splits
                .len()
                .checked_mul(SPLIT_BYTES)
                .and_then(|s| n.checked_add(s))
        })
        .ok_or("single-stop qualification byte count overflow")?;
    let admitted = variable
        .checked_add(HEADER_BYTES)
        .ok_or("single-stop qualification header overflow")?;
    if u64::try_from(admitted).map_err(display)? > value.facts.bounds.bytes {
        return Err("single-stop qualification body exceeds byte admission".into());
    }
    Ok(admitted)
}

pub(super) fn encode(id: [u8; 32], value: &Body) -> Result<Vec<u8>, String> {
    let admitted = encoded_length(value)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(admitted).map_err(display)?;
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&id);
    bytes.extend_from_slice(&Policy::V1.canonical_bytes());
    bytes.extend_from_slice(&crate::index_consistency::Policy::V1.canonical_bytes());
    let f = &value.facts;
    for id in [
        f.search,
        f.training.identity,
        f.training.completion,
        f.later.identity,
        f.later.completion,
    ] {
        bytes.extend_from_slice(&id);
    }
    bytes.extend_from_slice(&f.policy.canonical_bytes());
    words(&mut bytes, f.bounds.words());
    words(
        &mut bytes,
        [
            f.procedure.draws(),
            f.procedure.seed(),
            f.procedure.block_length(),
            f.allocation.batch(),
            f.allocation.rung(),
            f.allocation.alpha_ppm(),
            u64::try_from(f.count).map_err(display)?,
        ],
    );
    let s = &value.statistics;
    words(&mut bytes, s.white);
    words(&mut bytes, s.spa);
    bytes.extend_from_slice(&s.family);
    word(&mut bytes, u64::from(s.shared.is_some()));
    bytes.extend_from_slice(&s.shared.unwrap_or([0; 32]));
    word(&mut bytes, u64::from(s.layout.is_some()));
    words(&mut bytes, s.layout.unwrap_or([0; 4]));
    bytes.extend_from_slice(&s.layout_digest);
    word(&mut bytes, u64::try_from(s.splits.len()).map_err(display)?);
    for split in &s.splits {
        words(
            &mut bytes,
            [
                split.train,
                split.test,
                u64::from(split.bottom_half),
                u64::from(split.rankable),
            ],
        );
        bytes.extend_from_slice(&split.scores);
    }
    for row in &value.rows {
        for id in [
            row.original_run,
            row.later_run,
            row.original_digest,
            row.later_digest,
        ] {
            bytes.extend_from_slice(&id);
        }
        bytes.extend_from_slice(&row.projection.canonical_bytes());
        word(&mut bytes, row.wilson_bits);
        words(&mut bytes, row.romano);
        word(&mut bytes, u64::try_from(row.folds.len()).map_err(display)?);
        for fold in &row.folds {
            bytes.extend_from_slice(&fold.first_day.to_le_bytes());
            bytes.extend_from_slice(&fold.last_day.to_le_bytes());
            words(&mut bytes, [fold.observed_days, fold.trades, fold.wins]);
            bytes.extend_from_slice(&fold.pessimistic_paisa.to_le_bytes());
            words(&mut bytes, [fold.refused, u64::from(fold.decided)]);
        }
    }
    if bytes.len() != admitted || u64::try_from(bytes.len()).map_err(display)? > f.bounds.bytes {
        return Err("single-stop qualification encoded body exceeds byte admission".into());
    }
    Ok(bytes)
}

#[expect(
    clippy::too_many_lines,
    reason = "one canonical decoder follows the fixed header, split and row byte order and validates its entire extent before returning"
)]
pub(super) fn decode(bytes: &[u8], id: [u8; 32], max_records: u64) -> Result<Body, String> {
    let payload_bytes = u64::try_from(bytes.len()).map_err(display)?;
    let mut d = Decoder { bytes, at: 0 };
    if d.take(8)? != MAGIC || d.array::<32>()? != id {
        return Err("single-stop qualification format/identity differs".into());
    }
    Policy::decode(d.take(Policy::BYTE_LEN)?).map_err(display)?;
    crate::index_consistency::Policy::decode(d.take(crate::index_consistency::Policy::BYTE_LEN)?)
        .map_err(display)?;
    let search = d.array()?;
    let training = Link {
        identity: d.array()?,
        completion: d.array()?,
    };
    let later = Link {
        identity: d.array()?,
        completion: d.array()?,
    };
    let policy =
        AdmissionPolicyV1::from_canonical_bytes(d.take(ADMISSION_POLICY_CANONICAL_LEN_V1)?)
            .map_err(|why| format!("single-stop saved policy refused: {why:?}"))?;
    let [candidates, bootstrap_work, split_work, memory_bytes, bytes] = d.words()?;
    let bounds = Bounds {
        candidates,
        bootstrap_work,
        split_work,
        memory_bytes,
        bytes,
    };
    let [draws, seed, block, batch, rung, alpha, count] = d.words()?;
    let procedure =
        crate::population_statistics_v2::PopulationStatisticsProcedureV2::new(draws, seed, block)?;
    let allocation = runner::search_allocation_v1::allocate(batch, rung, alpha)
        .map_err(|why| format!("single-stop saved allocation refused: {why:?}"))?;
    let count = usize::try_from(count).map_err(display)?;
    let facts = Facts {
        search,
        training,
        later,
        policy,
        procedure,
        allocation,
        bounds,
        count,
    };
    if identity(&facts) != id
        || payload_bytes > facts.bounds.bytes
        || count == 0
        || !count.is_multiple_of(2)
        || count > d.remaining() / ROW_BYTES
        || u64::try_from(count).map_err(display)? > max_records
        || u64::try_from(count).map_err(display)? > candidates
        || [
            search,
            training.identity,
            training.completion,
            later.identity,
            later.completion,
        ]
        .contains(&[0; 32])
        || bounds.words().contains(&0)
    {
        return Err("single-stop qualification facts, count or exact identity differs".into());
    }
    let white = d.words()?;
    let spa = d.words()?;
    let family = d.array()?;
    let has_shared = d.flag()?;
    let shared = d.array()?;
    if !has_shared && shared != [0; 32] {
        return Err("single-stop absent shared statistics have nonzero bytes".into());
    }
    let has_layout = d.flag()?;
    let layout = d.words()?;
    let layout_digest = d.array()?;
    if !has_layout && (layout != [0; 4] || layout_digest != [0; 32]) {
        return Err("single-stop absent CSCV has nonzero bytes".into());
    }
    let split_count = d.count()?;
    if split_count > d.remaining() / SPLIT_BYTES
        || u64::try_from(split_count).map_err(display)? > max_records
    {
        return Err("single-stop split extent exceeds admission".into());
    }
    let mut splits = Vec::new();
    splits.try_reserve_exact(split_count).map_err(display)?;
    for _ in 0..split_count {
        splits.push(Split {
            train: d.word()?,
            test: d.word()?,
            bottom_half: d.flag()?,
            rankable: d.flag()?,
            scores: d.array()?,
        });
    }
    let statistics = Statistics {
        white,
        spa,
        family,
        shared: has_shared.then_some(shared),
        layout: has_layout.then_some(layout),
        layout_digest,
        splits,
    };
    let mut rows = Vec::new();
    rows.try_reserve_exact(count).map_err(display)?;
    let mut total = u64::try_from(split_count).map_err(display)?;
    for _ in 0..count {
        let original_run = d.array()?;
        let later_run = d.array()?;
        let original_digest = d.array()?;
        let later_digest = d.array()?;
        let projection = runner::admission::research_projection_codec::decode(
            &policy,
            d.take(RESEARCH_ADMISSION_PROJECTION_BYTES_V1)?,
        )?;
        let wilson_bits = d.word()?;
        let romano = d.words()?;
        let count = d.count()?;
        total = total
            .checked_add(u64::try_from(count).map_err(display)?)
            .ok_or("single-stop fold count overflow")?;
        if count == 0 || count > d.remaining() / FOLD_BYTES || total > max_records {
            return Err("single-stop fold extent exceeds admission".into());
        }
        let mut folds = Vec::new();
        folds.try_reserve_exact(count).map_err(display)?;
        for _ in 0..count {
            folds.push(Fold {
                first_day: i64::from_le_bytes(d.array()?),
                last_day: i64::from_le_bytes(d.array()?),
                observed_days: d.word()?,
                trades: d.word()?,
                wins: d.word()?,
                pessimistic_paisa: i64::from_le_bytes(d.array()?),
                refused: d.word()?,
                decided: d.flag()?,
            });
        }
        rows.push(Row {
            original_run,
            later_run,
            original_digest,
            later_digest,
            projection,
            wilson_bits,
            romano,
            folds,
        });
    }
    if d.remaining() != 0 {
        return Err("single-stop qualification trailing bytes refused".into());
    }
    Ok(Body {
        facts,
        statistics,
        rows,
    })
}
fn word(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn words<const N: usize>(out: &mut Vec<u8>, values: [u64; N]) {
    for value in values {
        word(out, value);
    }
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
            .ok_or("single-stop decoder extent overflow")?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or("single-stop qualification truncated")?;
        self.at = end;
        Ok(bytes)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        self.take(N)?.try_into().map_err(display)
    }
    fn word(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn words<const N: usize>(&mut self) -> Result<[u64; N], String> {
        let mut out = [0; N];
        for value in &mut out {
            *value = self.word()?;
        }
        Ok(out)
    }
    fn count(&mut self) -> Result<usize, String> {
        usize::try_from(self.word()?).map_err(display)
    }
    fn flag(&mut self) -> Result<bool, String> {
        match self.word()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("single-stop saved boolean tag invalid".into()),
        }
    }
}

//! Numeric observation codec; decoding cannot reconstruct an authoring token.
use super::{
    ExecutionRefusalBitsV1, FixedTrainingFoldV1, LaterSessionWindowV1, ProjectionData, display,
    validate_data,
};

const HEADER: usize = 320;
const STRIDE: usize = 64;
const MAGIC: &[u8; 8] = b"BRXFVL01";

/// Detached numeric observation. Source receipts must still authenticate every
/// original/later identity before the storage caller can use these numbers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedFixedTrainingFoldsV1 {
    data: ProjectionData,
}
impl ObservedFixedTrainingFoldsV1 {
    /// Validate the version, exact length, padding, windows and arithmetic.
    /// # Errors
    /// Refuses malformed domains, physical ceilings and inconsistent folds.
    pub fn decode(bytes: &[u8], max_bytes: u64, max_folds: usize) -> Result<Self, String> {
        if bytes.len() < HEADER || u64::try_from(bytes.len()).map_err(display)? > max_bytes {
            return Err("fixed-training observation exceeds header/byte admission".into());
        }
        let mut r = Reader { bytes, at: 0 };
        if r.take::<8>()? != *MAGIC {
            return Err("fixed-training observation version differs".into());
        }
        let plan = r.take()?;
        let anchor = r.take()?;
        let selected = r.take()?;
        let training_run = r.take()?;
        let later = r.take()?;
        let later_run = r.take()?;
        let resolution = r.take()?;
        let ordinal = r.u64()?;
        let count = usize::try_from(r.u64()?).map_err(display)?;
        let decided = r.u64()?;
        let profitable = r.u64()?;
        let aggregate = r.i64()?;
        let refusal = ExecutionRefusalBitsV1::from_bits(r.u64()?)
            .ok_or("unknown later execution refusal bit")?;
        let requested = LaterSessionWindowV1::new(r.i64()?, r.i64()?)?;
        let training_last_day = r.i64()?;
        if count == 0
            || count > max_folds
            || expected_bytes(count)? != bytes.len()
            || r.take::<16>()? != [0; 16]
        {
            return Err("fixed-training observation count/length/padding differs".into());
        }
        let mut folds = Vec::new();
        folds.try_reserve_exact(count).map_err(display)?;
        for _ in 0..count {
            let window = LaterSessionWindowV1::new(r.i64()?, r.i64()?)?;
            let sessions = r.u64()?;
            let trades = r.u64()?;
            let wins = r.u64()?;
            let return_paisa = r.i64()?;
            if r.take::<16>()? != [0; 16] {
                return Err("fixed-training fold padding differs".into());
            }
            folds.push(FixedTrainingFoldV1 {
                window,
                sessions,
                trades,
                wins,
                return_paisa,
            });
        }
        let data = ProjectionData {
            plan,
            anchor,
            selected,
            training_run,
            later,
            later_run,
            resolution,
            ordinal,
            requested,
            training_last_day,
            refusal,
            folds,
        };
        let totals = validate_data(&data)?;
        if decided != count as u64
            || profitable != data.folds.iter().filter(|f| f.return_paisa > 0).count() as u64
            || aggregate != totals.return_paisa
        {
            return Err("fixed-training observation summary differs from all folds".into());
        }
        Ok(Self { data })
    }
    /// Complete detached canonical bytes; never authoring authority.
    /// # Errors
    /// Refuses the physical byte ceiling or invalid internal observation.
    pub fn canonical_bytes(&self, max_bytes: u64) -> Result<Vec<u8>, String> {
        encode(&self.data, max_bytes)
    }
    /// Full predeclared partition identity.
    #[must_use]
    pub const fn plan_digest(&self) -> [u8; 32] {
        self.data.plan
    }
    /// Full original selected-coordinate identity.
    #[must_use]
    pub const fn selected_digest(&self) -> [u8; 32] {
        self.data.selected
    }
    /// Full original program/training anchor identity.
    #[must_use]
    pub const fn anchor_digest(&self) -> [u8; 32] {
        self.data.anchor
    }
    /// Full later evaluation identity.
    #[must_use]
    pub const fn later_digest(&self) -> [u8; 32] {
        self.data.later
    }
    /// Full original run identity.
    #[must_use]
    pub const fn training_run_id(&self) -> [u8; 32] {
        self.data.training_run
    }
    /// Full later run identity.
    #[must_use]
    pub const fn later_run_id(&self) -> [u8; 32] {
        self.data.later_run
    }
    /// Complete original resolution identity.
    #[must_use]
    pub const fn resolution_digest(&self) -> [u8; 32] {
        self.data.resolution
    }
    /// Canonical selected coordinate ordinal.
    #[must_use]
    pub const fn ordinal(&self) -> u64 {
        self.data.ordinal
    }
    /// Full declared later date interval.
    #[must_use]
    pub const fn requested(&self) -> LaterSessionWindowV1 {
        self.data.requested
    }
    /// Original training's last actual IST day.
    #[must_use]
    pub const fn training_last_day(&self) -> i64 {
        self.data.training_last_day
    }
    /// Every exact zero-inclusive fold observation.
    #[must_use]
    pub fn folds(&self) -> &[FixedTrainingFoldV1] {
        &self.data.folds
    }
    /// Existing later execution policy refusal bits, unchanged.
    #[must_use]
    pub const fn execution_refusal_bits(&self) -> ExecutionRefusalBitsV1 {
        self.data.refusal
    }
    /// Complete fixed-strategy evaluation windows, including zero-trade outcomes.
    #[must_use]
    pub fn decided_folds(&self) -> u64 {
        self.data.folds.len() as u64
    }
    /// Strictly positive pessimistic windows.
    #[must_use]
    pub fn profitable_oos_folds(&self) -> u64 {
        self.data
            .folds
            .iter()
            .filter(|f| f.return_paisa > 0)
            .count() as u64
    }
    /// Checked sum over every original frozen-coordinate window.
    /// # Errors
    /// Refuses arithmetic overflow or internally contradictory observations.
    pub fn aggregate_oos_paisa(&self) -> Result<i64, String> {
        Ok(validate_data(&self.data)?.return_paisa)
    }
}
pub(super) fn encode(data: &ProjectionData, max_bytes: u64) -> Result<Vec<u8>, String> {
    let totals = validate_data(data)?;
    let size = expected_bytes(data.folds.len())?;
    if u64::try_from(size).map_err(display)? > max_bytes {
        return Err("fixed-training canonical bytes exceed admission".into());
    }
    let mut out = Vec::new();
    out.try_reserve_exact(size).map_err(display)?;
    out.extend_from_slice(MAGIC);
    for identity in [
        data.plan,
        data.anchor,
        data.selected,
        data.training_run,
        data.later,
        data.later_run,
        data.resolution,
    ] {
        out.extend_from_slice(&identity);
    }
    for value in [
        data.ordinal,
        data.folds.len() as u64,
        data.folds.len() as u64,
        data.folds.iter().filter(|f| f.return_paisa > 0).count() as u64,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&totals.return_paisa.to_le_bytes());
    out.extend_from_slice(&data.refusal.bits().to_le_bytes());
    for value in [
        data.requested.first,
        data.requested.last,
        data.training_last_day,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&[0; 16]);
    for fold in &data.folds {
        for value in [fold.window.first, fold.window.last] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        for value in [fold.sessions, fold.trades, fold.wins] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(&fold.return_paisa.to_le_bytes());
        out.extend_from_slice(&[0; 16]);
    }
    Ok(out)
}
fn expected_bytes(count: usize) -> Result<usize, String> {
    count
        .checked_mul(STRIDE)
        .and_then(|n| n.checked_add(HEADER))
        .ok_or_else(|| "fixed-training observation length overflow".into())
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let end = self.at.checked_add(N).ok_or("fold codec offset overflow")?;
        let value = self
            .bytes
            .get(self.at..end)
            .ok_or("truncated fixed-training observation")?
            .try_into()
            .map_err(display)?;
        self.at = end;
        Ok(value)
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take()?))
    }
    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.take()?))
    }
}

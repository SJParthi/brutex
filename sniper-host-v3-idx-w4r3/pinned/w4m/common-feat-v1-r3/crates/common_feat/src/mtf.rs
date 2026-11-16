//! The higher-timeframe read `mtf` (REQ72-PLAN A.2; REQ 76e (2); OBF35): at a decision instant S a reading of
//! another timeframe of the SAME family is the one of its last candle with `end <= S`, never a candle still forming
//! (`mtf_candle_forming`, mutant m27 / m-S4). O(1): the slot keeps the last two values put.
//!
//! K13 (REPAIR 1, check-build-feat-v1 B1): when the decision stream's CONT readings reset at a factor ex-date, the slot
//! gets the reset instant ([`MtfSlot::reset_at`], the ex-date session's open) and from then on never reads a candle
//! that ENDED at or before it, so no higher-timeframe value compares post-event prices with a pre-event series. Until
//! the higher timeframe's first post-event candle has ended, its reading is `None` (counted, `reset_refused`).

/// The last completed values of one other timeframe's reading.
#[derive(Clone, Debug)]
pub struct MtfSlot<T: Copy> {
    last: Option<(i64, T)>,
    prev: Option<(i64, T)>,
    floor: Option<i64>,
    forming_refused: u64,
    reset_refused: u64,
}

impl<T: Copy> Default for MtfSlot<T> {
    fn default() -> Self {
        Self { last: None, prev: None, floor: None, forming_refused: 0, reset_refused: 0 }
    }
}

impl<T: Copy> MtfSlot<T> {
    /// An empty slot.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Puts the reading of a candle that ends at `end`.
    pub fn put(&mut self, end: i64, v: T) {
        self.prev = self.last.replace((end, v));
    }

    /// A K13 CONT reset of the decision stream at instant `floor` (the ex-date session's open): from now on no candle
    /// that ended at or before `floor` is read. Floors only rise.
    pub fn reset_at(&mut self, floor: i64) {
        self.floor = Some(self.floor.map_or(floor, |f| f.max(floor)));
    }

    /// True when a held candle ending at `end` lies after the last reset (always true under `m_mtf_reset_cross`).
    fn after_reset(&self, end: i64) -> bool {
        cfg!(feature = "m_mtf_reset_cross") || self.floor.is_none_or(|f| end > f)
    }

    /// The value at decision instant `s`: the last put candle with `end <= s` that ended after the last reset. A put
    /// candle that has not ended by `s` is never read (counted); a candle from before the reset is never read (counted).
    /// `None` when no such candle is held.
    pub fn at(&mut self, s: i64) -> Option<T> {
        let pick = if cfg!(feature = "m_mtf_forming") {
            self.last
        } else {
            match (self.last, self.prev) {
                (Some((e, v)), _) if e <= s => Some((e, v)),
                (Some(_), Some((e, v))) if e <= s => {
                    self.forming_refused += 1;
                    Some((e, v))
                }
                (Some(_), _) => {
                    self.forming_refused += 1;
                    None
                }
                (None, _) => None,
            }
        };
        match pick {
            Some((e, v)) if self.after_reset(e) => Some(v),
            Some(_) => {
                self.reset_refused += 1;
                None
            }
            None => None,
        }
    }

    /// How many reads met a forming candle.
    #[must_use]
    pub const fn forming_refused(&self) -> u64 {
        self.forming_refused
    }

    /// How many reads met only a candle from before the last K13 reset (read as `None`).
    #[must_use]
    pub const fn reset_refused(&self) -> u64 {
        self.reset_refused
    }
}

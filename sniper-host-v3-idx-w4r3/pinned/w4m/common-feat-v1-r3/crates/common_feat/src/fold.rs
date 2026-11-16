//! Candles and the fold of shorter bars into one timeframe (REQ72-PLAN A.2 `fold::Tf`; K14).
//!
//! Boundaries start at the dated session table's OPEN for that session (never a fixed 09:15:00): candle k covers
//! `[open + (k - 1) tf, open + k tf)`. A candle exists only if a bar falls in it. The last candle of a segment ends at
//! the segment's end when that comes first (flagged short). A bar stamped before the open or at / after the segment end
//! is refused (counted by the host). O(1) a bar (SC-FOLD <= 3).

/// One candle: epoch seconds UTC, paisa, volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candle {
    /// Start (inclusive), epoch seconds UTC.
    pub start: i64,
    /// End (exclusive), epoch seconds UTC: the instant its reading is known (K1).
    pub end: i64,
    /// Open, paisa.
    pub open: i64,
    /// High, paisa.
    pub high: i64,
    /// Low, paisa.
    pub low: i64,
    /// Close, paisa.
    pub close: i64,
    /// Volume (shares).
    pub volume: u64,
}

impl Candle {
    /// The candle's own sanity: low <= open, close <= high, low <= high, prices > 0.
    #[must_use]
    pub const fn sane(&self) -> bool {
        self.low > 0
            && self.low <= self.high
            && self.low <= self.open
            && self.open <= self.high
            && self.low <= self.close
            && self.close <= self.high
            && self.start < self.end
    }
}

/// What one push did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pushed {
    /// The bar went into the forming candle.
    Folded,
    /// The bar started a new candle; the previous one is complete and returned.
    Completed(Candle),
    /// The bar lies outside `[open, segment end)`: refused.
    Refused,
}

/// Folds bars of a shorter timeframe into candles of `tf` seconds, anchored at each session's open.
#[derive(Clone, Debug)]
pub struct Fold {
    tf: i64,
    open: i64,
    seg_end: i64,
    slot: i64,
    cur: Option<Candle>,
}

impl Fold {
    /// A fold into `tf_s`-second candles.
    #[must_use]
    pub fn new(tf_s: i64) -> Self {
        Self { tf: tf_s.max(1), open: 0, seg_end: 0, slot: -1, cur: None }
    }

    /// The candle length in seconds.
    #[must_use]
    pub const fn tf(&self) -> i64 {
        self.tf
    }

    /// Starts a segment `[open, seg_end)` (a session, or the second segment of a two-segment day: the grid stays
    /// anchored at `anchor`, the session's first open, K14). Returns the pending candle of the previous segment.
    pub fn segment(&mut self, anchor: i64, open: i64, seg_end: i64) -> Option<Candle> {
        let done = self.flush();
        self.open = anchor;
        self.seg_end = seg_end;
        // A second segment keeps the session's grid; bars before its own open are refused.
        let _ = open;
        self.slot = -1;
        done
    }

    /// Folds one bar `b` (its `start` decides its slot). Returns what happened and the step count.
    pub fn push(&mut self, b: &Candle) -> (Pushed, u32) {
        if b.start < self.open || b.start >= self.seg_end {
            return (Pushed::Refused, 1);
        }
        let slot = (b.start - self.open).div_euclid(self.tf);
        if slot == self.slot
            && let Some(c) = self.cur.as_mut()
        {
            c.high = c.high.max(b.high);
            c.low = c.low.min(b.low);
            c.close = b.close;
            c.volume = c.volume.saturating_add(b.volume);
            return (Pushed::Folded, 2);
        }
        let start = self.open + slot * self.tf;
        let end = (start + self.tf).min(self.seg_end);
        let fresh = Candle { start, end, open: b.open, high: b.high, low: b.low, close: b.close, volume: b.volume };
        self.slot = slot;
        match self.cur.replace(fresh) {
            Some(done) => (Pushed::Completed(done), 3),
            None => (Pushed::Folded, 3),
        }
    }

    /// Returns the forming candle as complete (at the segment end or the end of the data).
    pub fn flush(&mut self) -> Option<Candle> {
        self.slot = -1;
        self.cur.take()
    }
}

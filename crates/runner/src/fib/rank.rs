//! The mover rank: the cross-sectional gainer and loser ranks of every
//! Zerodha minute over the day's point-in-time pool (FIB-2, PART B; D-6322,
//! D-6323).
//!
//! # The rule (D-6322)
//!
//! Per IST day the caller opens the day with the pool's member slots, in
//! point-in-time series-id order (the FIXED TIE RULE: slot order), and each
//! member's previous close ([`MoverRank::begin_day`]). It then folds the day's
//! minutes in time order, one finished Zerodha one-minute close per member or
//! none ([`MoverRank::minute`]) — a fold, never an indexed read of a later
//! minute (`CLAUDE.md` §3 rule 7). Membership is the caller's: this file never
//! names `core::universe`.
//!
//! * `move_bp` = (last close at or before the minute − previous close) ×
//!   10,000 / previous close, in `i128`, rounded toward zero, narrowed to
//!   `i32`; a member whose move does not fit is refused for that minute
//!   (`move_out_of_range`, counted) and the others are unchanged (FM-F021).
//! * The SIDE is the sign of (close − previous close) in paisa (FM-F019): a
//!   move of 0.4 bp is a gainer, though its `move_bp` is 0. A close equal to
//!   the previous close is on neither side.
//! * Gainers rank by move descending, losers by move ascending (most negative
//!   first). The order is `move_bp`, then, inside each run of equal `move_bp`,
//!   the exact ratio `(c1 − p1) x p2` against `(c2 − p2) x p1` in `i128`, then
//!   slot order.
//! * A member with no minute yet today is MISSING: not ranked, counted. A
//!   member whose last close is older than the minute is STALE: ranked on that
//!   close, flagged, counted.
//!
//! A decision closing at instant `c` reads the minute `[c − 60 s, c)`, which
//! closes at `c` — on the 09:15 grid the same instant as the decision candle,
//! so no second timeframe enters a trade.
//!
//! # The order and its width (D-6323)
//!
//! A stable LSD radix sort over the members in slot order (FM-F010), with
//! digits of [`RANK_RADIX_DIGIT_BITS`] bits: gainers keyed by
//! `i32::MAX − move_bp`, losers by the sign-flipped `move_bp`, with the side in
//! the key's top bits, so one sort over the whole pool puts the gainers first,
//! the losers next and the unranked last. Its buffers are reserved once, at
//! the run's largest pool ([`MoverRank::new`]); its work per minute is
//! O(pool + buckets), the same at every minute of a day and O(1) per member, with the exact tie runs on top, each O(r log r) in its own length r
//! (`docs/06-limits.md`). The ranks are the ones a sorting network over the
//! same keys gives (`crates/runner/tests/fib_rank.rs`). Ranks are `u16`, and
//! the run's rank WIDTH — one byte while the largest pool fits one, two above —
//! is chosen from that pool and recorded for the rank file's header
//! ([`RankWidth`], FM-F009); a rank beyond it is refused by name, never wrapped.
//!
//! This module is pure: no I/O, no print, no telemetry (gates 17 and 22).
//! Every refusal is counted in a plain field.

use vocab::fib_cube::{BP_PER_UNIT, RANK_RADIX_DIGIT_BITS, Ratio};

/// Values one radix digit takes.
const BUCKETS: usize = 1 << RANK_RADIX_DIGIT_BITS;

const _: () = assert!(RANK_RADIX_DIGIT_BITS > 0 && RANK_RADIX_DIGIT_BITS < u32::BITS as u64);

/// The bytes one rank takes in the rank file: the run's rank width (FM-F009).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RankWidth {
    /// One byte: the run's largest pool fits `u8`.
    U8,
    /// Two bytes: the run's largest pool fits `u16`.
    U16,
}

impl RankWidth {
    /// The width a run whose largest pool is `max_pool` members needs.
    ///
    /// # Errors
    ///
    /// [`RankRefusal::RankWidthExceeded`] above `u16::MAX` members.
    pub fn of_pool(max_pool: usize) -> Result<Self, RankRefusal> {
        if u8::try_from(max_pool).is_ok() {
            Ok(Self::U8)
        } else if u16::try_from(max_pool).is_ok() {
            Ok(Self::U16)
        } else {
            Err(RankRefusal::RankWidthExceeded {
                rank: max_pool,
                width: Self::U16,
            })
        }
    }

    /// The largest rank this width holds.
    #[must_use]
    pub const fn max_rank(self) -> u16 {
        match self {
            Self::U8 => u8::MAX as u16,
            Self::U16 => u16::MAX,
        }
    }

    /// The width's name, for the rank file's header.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
        }
    }

    /// Appends one rank at this width, little-endian.
    ///
    /// # Errors
    ///
    /// [`RankRefusal::RankWidthExceeded`] for a rank the width cannot hold:
    /// refused by name, never wrapped (FM-F009).
    pub fn put(self, rank: u16, out: &mut Vec<u8>) -> Result<(), RankRefusal> {
        let [low, high] = rank.to_le_bytes();
        match self {
            Self::U8 if rank <= self.max_rank() => out.push(low),
            Self::U16 => out.extend_from_slice(&[low, high]),
            Self::U8 => {
                return Err(RankRefusal::RankWidthExceeded {
                    rank: usize::from(rank),
                    width: self,
                });
            }
        }
        Ok(())
    }
}

/// A refusal of the mover rank, by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RankRefusal {
    /// A rank (or a pool) beyond the width chosen for it (FM-F009).
    RankWidthExceeded {
        /// The rank or pool size offered.
        rank: usize,
        /// The width it exceeds.
        width: RankWidth,
    },
    /// A run or a day with no member.
    PoolEmpty,
    /// A day's pool larger than the run's largest pool, which sized the
    /// buffers.
    PoolOverRunMaximum {
        /// The day's pool.
        pool: usize,
        /// The run's largest pool.
        max_pool: usize,
    },
    /// A minute folded before any day was opened.
    NoDay,
    /// A minute whose closes are not one per member of the day's pool.
    ClosesNotPool {
        /// Closes offered.
        closes: usize,
        /// The day's pool.
        pool: usize,
    },
    /// A minute at or before the last one folded today.
    MinuteOutOfOrder {
        /// The minute's close instant offered.
        minute: i64,
        /// The last minute folded.
        last: i64,
    },
    /// A year counted before the last year counted.
    YearOutOfOrder {
        /// The year offered.
        year: i32,
        /// The last year counted.
        last: i32,
    },
}

impl RankRefusal {
    /// The refusal's stable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::RankWidthExceeded { .. } => "rank_width_exceeded",
            Self::PoolEmpty => "pool_empty",
            Self::PoolOverRunMaximum { .. } => "pool_over_run_maximum",
            Self::NoDay => "no_day",
            Self::ClosesNotPool { .. } => "closes_not_pool",
            Self::MinuteOutOfOrder { .. } => "minute_out_of_order",
            Self::YearOutOfOrder { .. } => "year_out_of_order",
        }
    }
}

impl core::fmt::Display for RankRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "mover rank refused ({}): {self:?}", self.name())
    }
}

impl std::error::Error for RankRefusal {}

/// Why one member is not ranked at one minute, counted by name. APPEND-ONLY.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MemberRefusal {
    /// The member has no previous close today.
    NoPreviousClose,
    /// The member's previous close is zero or negative.
    PreviousCloseNotPositive,
    /// This minute's close is zero or negative; the last good close is kept.
    CloseNotPositive,
    /// The move does not fit `i32` basis points (FM-F021).
    MoveOutOfRange,
}

impl MemberRefusal {
    /// Every reason, in declaration order.
    pub const ALL: [Self; MEMBER_REFUSALS] = [
        Self::NoPreviousClose,
        Self::PreviousCloseNotPositive,
        Self::CloseNotPositive,
        Self::MoveOutOfRange,
    ];

    /// The reason's counted name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoPreviousClose => "no_previous_close",
            Self::PreviousCloseNotPositive => "previous_close_not_positive",
            Self::CloseNotPositive => "close_not_positive",
            Self::MoveOutOfRange => "move_out_of_range",
        }
    }
}

/// How many member refusal reasons there are.
pub const MEMBER_REFUSALS: usize = MemberRefusal::MoveOutOfRange as usize + 1;

/// A member's state at one minute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MemberState {
    /// No close yet today: not ranked.
    Missing,
    /// Ranked on this minute's close.
    Fresh,
    /// Ranked on an earlier close of today: flagged.
    Stale,
    /// Not ranked, for the reason given.
    Refused(MemberRefusal),
}

/// The census of one minute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Census {
    /// The minute's close instant.
    pub minute: i64,
    /// Members in the day's pool.
    pub pool: u16,
    /// Members ranked: gainers and losers.
    pub ranked: u16,
    /// Members ranked as gainers.
    pub gainers: u16,
    /// Members ranked as losers.
    pub losers: u16,
    /// Members with a close equal to their previous close: on neither side.
    pub flat: u16,
    /// Members with no close yet today.
    pub missing: u16,
    /// Members ranked on an earlier close (counted among the ranked and the
    /// unmoved).
    pub stale: u16,
    /// Members refused this minute.
    pub refused: u16,
}

/// The run-wide totals: minutes folded and member-minutes by fate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Totals {
    /// Minutes folded.
    pub minutes: u64,
    /// Member-minutes ranked.
    pub ranked: u64,
    /// Member-minutes whose close equals the previous close.
    pub flat: u64,
    /// Member-minutes missing.
    pub missing: u64,
    /// Member-minutes stale.
    pub stale: u64,
    /// Member-minutes refused, by [`MemberRefusal`].
    pub refused: [u64; MEMBER_REFUSALS],
}

/// Where a member falls at one minute: the top bits of its radix key, so one
/// sort puts the gainers first, the losers next and every unranked member
/// last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// Ranked by move descending.
    Gainer,
    /// Ranked by move ascending.
    Loser,
    /// Missing, refused or unchanged: never ranked.
    Neither,
}

impl Side {
    /// The side a key's top bits name.
    const fn of(key: u64) -> Self {
        match key >> u32::BITS {
            0 => Self::Gainer,
            1 => Self::Loser,
            _ => Self::Neither,
        }
    }

    /// The side's top bits, above the 32 bits of the move.
    const fn bits(self) -> u64 {
        (self as u64) << u32::BITS
    }
}

/// The bits the side takes above the move's 32.
const SIDE_BITS: u32 = u64::BITS - (Side::Neither as u64).leading_zeros();

/// The radix passes over a key: the move's 32 bits and the side's.
const PASSES: u64 = ((u32::BITS + SIDE_BITS) as u64).div_ceil(RANK_RADIX_DIGIT_BITS);

/// One digit's mask.
const DIGIT_MASK: u64 = (1 << RANK_RADIX_DIGIT_BITS) - 1;

/// The mover-rank fold of one run (D-6322, D-6323).
///
/// Every per-member buffer holds exactly today's pool and is reserved once, at
/// the run's largest pool, so no day and no minute allocates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoverRank {
    /// The run's rank width.
    width: RankWidth,
    /// The run's largest pool: every buffer's reserved size.
    max_pool: usize,
    /// Whether a day is open.
    open: bool,
    /// The last minute folded today.
    last_minute: Option<i64>,
    /// Per slot: the previous close, or the day's refusal.
    previous: Vec<Result<i64, MemberRefusal>>,
    /// Per slot: today's last good close.
    last_close: Vec<Option<i64>>,
    /// Per slot: the state at the last minute.
    state: Vec<MemberState>,
    /// Per slot: `move_bp` at the last minute (zero when not ranked).
    move_bp: Vec<i32>,
    /// Per slot: close minus previous close at the last minute, in paisa.
    diff: Vec<i64>,
    /// Per slot: the radix key at the last minute (side, then move).
    keys: Vec<u64>,
    /// Every slot in ranked order: gainers, then losers, then the unranked.
    order: Vec<u16>,
    /// The radix sort's other buffer.
    scratch: Vec<u16>,
    /// The radix sort's digit counts.
    counts: Box<[usize]>,
    /// Per slot: the gainer rank at the last minute, 0 = unranked.
    gainer_rank: Vec<u16>,
    /// Per slot: the loser rank at the last minute, 0 = unranked.
    loser_rank: Vec<u16>,
    /// Gainers at the last minute: `order[..gainers]`.
    gainers: usize,
    /// Losers at the last minute: `order[gainers..gainers + losers]`.
    losers: usize,
    /// The last minute's census.
    census: Option<Census>,
    /// The run's totals.
    totals: Totals,
    /// The structural work counter the bench reads: members visited, buckets
    /// swept and tie comparisons (a plain field, gate 17).
    work: u64,
}

impl MoverRank {
    /// A fold sized once for a run whose largest pool (PIT-1's header) is
    /// `max_pool` members.
    ///
    /// # Errors
    ///
    /// [`RankRefusal::PoolEmpty`] and [`RankRefusal::RankWidthExceeded`].
    pub fn new(max_pool: usize) -> Result<Self, RankRefusal> {
        if max_pool == 0 {
            return Err(RankRefusal::PoolEmpty);
        }
        let width = RankWidth::of_pool(max_pool)?;
        Ok(Self {
            width,
            max_pool,
            open: false,
            last_minute: None,
            previous: Vec::with_capacity(max_pool),
            last_close: Vec::with_capacity(max_pool),
            state: Vec::with_capacity(max_pool),
            move_bp: Vec::with_capacity(max_pool),
            diff: Vec::with_capacity(max_pool),
            keys: Vec::with_capacity(max_pool),
            order: Vec::with_capacity(max_pool),
            scratch: Vec::with_capacity(max_pool),
            counts: vec![0; BUCKETS].into_boxed_slice(),
            gainer_rank: Vec::with_capacity(max_pool),
            loser_rank: Vec::with_capacity(max_pool),
            gainers: 0,
            losers: 0,
            census: None,
            totals: Totals {
                minutes: 0,
                ranked: 0,
                flat: 0,
                missing: 0,
                stale: 0,
                refused: [0; MEMBER_REFUSALS],
            },
            work: 0,
        })
    }

    /// The run's rank width, for the rank file's header.
    #[must_use]
    pub const fn width(&self) -> RankWidth {
        self.width
    }

    /// The run's largest pool: the buffers' reserved size.
    #[must_use]
    pub const fn max_pool(&self) -> usize {
        self.max_pool
    }

    /// Today's pool.
    #[must_use]
    pub fn pool(&self) -> usize {
        self.previous.len()
    }

    /// Opens an IST day: one previous close per member slot, in slot order. A
    /// member with none, or with one that is not positive, is refused for the
    /// whole day, by name, at every minute.
    ///
    /// # Errors
    ///
    /// [`RankRefusal::PoolEmpty`] and [`RankRefusal::PoolOverRunMaximum`]; the
    /// fold is unchanged.
    pub fn begin_day(&mut self, previous_closes: &[Option<i64>]) -> Result<(), RankRefusal> {
        let pool = previous_closes.len();
        if pool == 0 {
            return Err(RankRefusal::PoolEmpty);
        }
        if pool > self.max_pool {
            return Err(RankRefusal::PoolOverRunMaximum {
                pool,
                max_pool: self.max_pool,
            });
        }
        self.previous.clear();
        self.previous
            .extend(previous_closes.iter().map(|close| match close {
                None => Err(MemberRefusal::NoPreviousClose),
                Some(close) if *close <= 0 => Err(MemberRefusal::PreviousCloseNotPositive),
                Some(close) => Ok(*close),
            }));
        for (buffer, value) in [
            (&mut self.gainer_rank, 0),
            (&mut self.loser_rank, 0),
            (&mut self.order, 0),
            (&mut self.scratch, 0),
        ] {
            buffer.clear();
            buffer.resize(pool, value);
        }
        self.last_close.clear();
        self.last_close.resize(pool, None);
        self.state.clear();
        self.state.resize(pool, MemberState::Missing);
        self.move_bp.clear();
        self.move_bp.resize(pool, 0);
        self.diff.clear();
        self.diff.resize(pool, 0);
        self.keys.clear();
        self.keys.resize(pool, Side::Neither.bits());
        self.open = true;
        self.last_minute = None;
        self.gainers = 0;
        self.losers = 0;
        self.census = None;
        Ok(())
    }

    /// Folds one finished Zerodha minute: each member's close for the minute
    /// closing at `minute` (an instant, strictly later than the last one
    /// folded today), or `None`. Every rank and state read afterwards is this
    /// minute's.
    ///
    /// # Errors
    ///
    /// [`RankRefusal::NoDay`], [`RankRefusal::ClosesNotPool`] and
    /// [`RankRefusal::MinuteOutOfOrder`]; the fold is unchanged.
    pub fn minute(&mut self, minute: i64, closes: &[Option<i64>]) -> Result<Census, RankRefusal> {
        if !self.open {
            return Err(RankRefusal::NoDay);
        }
        if closes.len() != self.pool() {
            return Err(RankRefusal::ClosesNotPool {
                closes: closes.len(),
                pool: self.pool(),
            });
        }
        if let Some(last) = self.last_minute.filter(|last| minute <= *last) {
            return Err(RankRefusal::MinuteOutOfOrder { minute, last });
        }
        self.last_minute = Some(minute);
        let mut census = Census {
            minute,
            pool: u16::try_from(self.pool()).unwrap_or(u16::MAX),
            ranked: 0,
            gainers: 0,
            losers: 0,
            flat: 0,
            missing: 0,
            stale: 0,
            refused: 0,
        };
        let members = self
            .previous
            .iter()
            .zip(closes)
            .zip(self.last_close.iter_mut())
            .zip(self.state.iter_mut())
            .zip(self.diff.iter_mut())
            .zip(self.move_bp.iter_mut())
            .zip(self.keys.iter_mut());
        for ((((((previous, close), last), state), diff), move_bp), key) in members {
            let fresh = close.filter(|close| *close > 0);
            if fresh.is_some() {
                *last = fresh;
            }
            *state = classify(*previous, *close, *last, diff, move_bp);
            let side = match *state {
                MemberState::Fresh | MemberState::Stale if *diff > 0 => Side::Gainer,
                MemberState::Fresh | MemberState::Stale if *diff < 0 => Side::Loser,
                _ => Side::Neither,
            };
            // The move's 32 bits sit below the side's, so the sum joins them;
            // a sum rather than an or, because an or and an xor of disjoint
            // bits agree.
            *key = side.bits()
                + u64::from(match side {
                    Side::Gainer => (i32::MAX - *move_bp).cast_unsigned(),
                    Side::Loser => (*move_bp ^ i32::MIN).cast_unsigned(),
                    Side::Neither => 0,
                });
            census.gainers += u16::from(side == Side::Gainer);
            census.losers += u16::from(side == Side::Loser);
            match *state {
                MemberState::Missing => census.missing += 1,
                MemberState::Refused(reason) => {
                    census.refused += 1;
                    for (count, known) in self.totals.refused.iter_mut().zip(MemberRefusal::ALL) {
                        *count += u64::from(known == reason);
                    }
                }
                MemberState::Fresh | MemberState::Stale => {
                    census.stale += u16::from(*state == MemberState::Stale);
                    census.flat += u16::from(*diff == 0);
                }
            }
        }
        self.gainers = usize::from(census.gainers);
        self.losers = usize::from(census.losers);
        self.order_and_rank();
        census.ranked = census.gainers + census.losers;
        self.totals.minutes += 1;
        self.totals.ranked += u64::from(census.ranked);
        self.totals.flat += u64::from(census.flat);
        self.totals.missing += u64::from(census.missing);
        self.totals.stale += u64::from(census.stale);
        self.census = Some(census);
        Ok(census)
    }

    /// Sorts every member of today's pool by its key — side, then move — in
    /// one stable radix from slot order, orders each run of equal keys by the
    /// exact move, and writes every member's ranks.
    fn order_and_rank(&mut self) {
        for (at, slot) in self.order.iter_mut().zip(0_u16..) {
            *at = slot;
        }
        let Self {
            order,
            scratch,
            keys,
            counts,
            diff,
            previous,
            ..
        } = self;
        let pool = u64::try_from(order.len()).unwrap_or(u64::MAX);
        self.work += pool + radix(order, scratch, keys, counts);
        self.work += tie_runs(order, keys, diff, previous);
        self.gainer_rank.fill(0);
        self.loser_rank.fill(0);
        let gainers = u16::try_from(self.gainers).unwrap_or(u16::MAX);
        let ranked = self.order.iter().take(self.gainers + self.losers);
        for (position, slot) in (1_u16..).zip(ranked) {
            let (ranks, rank) = if position <= gainers {
                (&mut self.gainer_rank, position)
            } else {
                (&mut self.loser_rank, position - gainers)
            };
            write_at(ranks, Some(usize::from(*slot)), rank);
        }
    }

    /// A member's gainer rank at the last minute: 1 is the largest gainer,
    /// 0 is unranked.
    #[must_use]
    pub fn gainer_rank(&self, slot: u16) -> u16 {
        self.gainer_rank
            .get(usize::from(slot))
            .copied()
            .unwrap_or(0)
    }

    /// A member's loser rank at the last minute: 1 is the largest loser,
    /// 0 is unranked.
    #[must_use]
    pub fn loser_rank(&self, slot: u16) -> u16 {
        self.loser_rank.get(usize::from(slot)).copied().unwrap_or(0)
    }

    /// A member's state at the last minute, or `None` outside today's pool.
    #[must_use]
    pub fn state(&self, slot: u16) -> Option<MemberState> {
        self.state.get(usize::from(slot)).copied()
    }

    /// A ranked member's `move_bp` at the last minute.
    #[must_use]
    pub fn move_bp(&self, slot: u16) -> Option<i32> {
        match self.state(slot) {
            Some(MemberState::Fresh | MemberState::Stale) => {
                self.move_bp.get(usize::from(slot)).copied()
            }
            _ => None,
        }
    }

    /// The gainers at the last minute, by rank: their slots.
    #[must_use]
    pub fn gainers(&self) -> &[u16] {
        self.order.get(..self.gainers).unwrap_or_default()
    }

    /// The losers at the last minute, by rank: their slots.
    #[must_use]
    pub fn losers(&self) -> &[u16] {
        self.order
            .get(self.gainers..self.gainers + self.losers)
            .unwrap_or_default()
    }

    /// The last minute's census, or `None` before today's first minute.
    #[must_use]
    pub const fn census(&self) -> Option<Census> {
        self.census
    }

    /// The run's totals.
    #[must_use]
    pub const fn totals(&self) -> Totals {
        self.totals
    }

    /// The structural work done so far.
    #[must_use]
    pub const fn work(&self) -> u64 {
        self.work
    }

    /// A ranked member's rank as an exact share of its side's ranked count at
    /// the last minute — `rank / gainers` for a gainer, `rank / losers` for a
    /// loser (FM-F005's stamp beside the raw rank, D-6314) — or `None` when
    /// unranked. Its permille is a display unit only.
    #[must_use]
    pub fn rank_share(&self, slot: u16) -> Option<Ratio> {
        let (gainer, loser) = (self.gainer_rank(slot), self.loser_rank(slot));
        let (rank, count) = if gainer > 0 {
            (gainer, self.gainers)
        } else {
            (loser, self.losers)
        };
        let count = u64::try_from(count).ok()?;
        Ratio::new(u64::from(rank), count).ok().filter(|_| rank > 0)
    }
}

/// Decision ranks beyond the widest rank of the first learning window, per
/// year (FM-F005: `beyond_w0_range`, information only): later pools are
/// larger, and every ranked decision still has a group. Plain counters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeyondW0 {
    /// The widest decision rank the first learning window held.
    widest: u16,
    /// `(year, decisions, beyond)`, years ascending.
    years: Vec<(i32, u64, u64)>,
}

impl BeyondW0 {
    /// Counters against the first learning window's widest rank.
    #[must_use]
    pub const fn new(widest: u16) -> Self {
        Self {
            widest,
            years: Vec::new(),
        }
    }

    /// Counts one ranked decision of `year`; a rank of 0 is unranked and is
    /// not a decision of the rank row.
    ///
    /// # Errors
    ///
    /// [`RankRefusal::YearOutOfOrder`] for a year before the last one counted.
    pub fn record(&mut self, year: i32, rank: u16) -> Result<(), RankRefusal> {
        if rank == 0 {
            return Ok(());
        }
        let beyond = u64::from(rank > self.widest);
        match self.years.last_mut() {
            Some((last, decisions, count)) if *last == year => {
                *decisions += 1;
                *count += beyond;
            }
            Some((last, _, _)) if *last > year => {
                return Err(RankRefusal::YearOutOfOrder { year, last: *last });
            }
            _ => self.years.push((year, 1, beyond)),
        }
        Ok(())
    }

    /// `(year, decisions, beyond)` per year counted, ascending.
    #[must_use]
    pub fn years(&self) -> &[(i32, u64, u64)] {
        &self.years
    }
}

/// One member's state at one minute, with its close difference and its move.
fn classify(
    previous: Result<i64, MemberRefusal>,
    close: Option<i64>,
    last: Option<i64>,
    diff: &mut i64,
    move_bp: &mut i32,
) -> MemberState {
    *diff = 0;
    *move_bp = 0;
    let previous = match previous {
        Ok(previous) => previous,
        Err(reason) => return MemberState::Refused(reason),
    };
    if close.is_some_and(|close| close <= 0) {
        return MemberState::Refused(MemberRefusal::CloseNotPositive);
    }
    let Some(last) = last else {
        return MemberState::Missing;
    };
    let moved = i128::from(last) - i128::from(previous);
    let bp = moved * i128::from(BP_PER_UNIT) / i128::from(previous);
    let Ok(bp) = i32::try_from(bp) else {
        return MemberState::Refused(MemberRefusal::MoveOutOfRange);
    };
    *diff = last - previous;
    *move_bp = bp;
    if close.is_some() {
        MemberState::Fresh
    } else {
        MemberState::Stale
    }
}

/// A stable LSD radix sort of `order` (slots) by `keys`, through `scratch`,
/// which holds as many slots. Returns its work: members moved and buckets
/// swept.
fn radix(order: &mut [u16], scratch: &mut [u16], keys: &[u64], counts: &mut [usize]) -> u64 {
    let digit = |slot: u16, shift: u64| -> usize {
        let key = keys.get(usize::from(slot)).copied().unwrap_or(0);
        usize::try_from((key >> shift) & DIGIT_MASK).unwrap_or(0)
    };
    for pass in 0..PASSES {
        let shift = pass * RANK_RADIX_DIGIT_BITS;
        counts.fill(0);
        for slot in order.iter() {
            next_place(counts, digit(*slot, shift));
        }
        let mut start = 0;
        for count in counts.iter_mut() {
            let here = *count;
            *count = start;
            start += here;
        }
        for slot in order.iter() {
            write_at(scratch, next_place(counts, digit(*slot, shift)), *slot);
        }
        order.copy_from_slice(scratch);
    }
    let len = order.len();
    let per_pass = u64::try_from(len + len + len + BUCKETS + BUCKETS).unwrap_or(u64::MAX);
    PASSES * per_pass
}

/// Takes the next place of digit `digit`: its counter before one is added to
/// it. Counting a digit is taking a place and dropping it. `None` past the
/// counters, which no digit reaches: it is masked to the buckets.
fn next_place(counts: &mut [usize], digit: usize) -> Option<usize> {
    let place = counts.get(digit).copied();
    write_at(counts, Some(digit), place.unwrap_or(0) + 1);
    place
}

/// Writes `value` at `at`. Every caller's index is inside its buffer by
/// construction and the workspace denies indexing, so the write is checked
/// here, once: `None` or an index past the slice writes nothing.
fn write_at<T>(cells: &mut [T], at: Option<usize>, value: T) {
    if let Some(cell) = at.and_then(|at| cells.get_mut(at)) {
        *cell = value;
    }
}

/// Orders each run of equal keys in `order` by the exact move, `(c1 − p1) x
/// p2` against `(c2 − p2) x p1` in `i128`, largest first for gainers and
/// smallest first for losers, then by slot; the unranked run is left in slot
/// order. Returns its work: comparisons.
fn tie_runs(
    order: &mut [u16],
    keys: &[u64],
    diff: &[i64],
    previous: &[Result<i64, MemberRefusal>],
) -> u64 {
    let key_of = |slot: &u16| keys.get(usize::from(*slot)).copied();
    let exact = |slot: u16| -> (i128, i128) {
        let at = usize::from(slot);
        let moved = diff.get(at).copied().unwrap_or(0);
        let base = previous.get(at).and_then(|p| p.ok()).unwrap_or(1);
        (i128::from(moved), i128::from(base))
    };
    let mut comparisons = 0_u64;
    for run in order.chunk_by_mut(|a, b| key_of(a) == key_of(b)) {
        let descending = match run.first().and_then(key_of).map(Side::of) {
            Some(Side::Gainer) => true,
            Some(Side::Loser) => false,
            _ => continue,
        };
        run.sort_unstable_by(|a, b| {
            comparisons += 1;
            let ((da, pa), (db, pb)) = (exact(*a), exact(*b));
            let order = (da * pb).cmp(&(db * pa));
            let order = if descending { order.reverse() } else { order };
            order.then(a.cmp(b))
        });
    }
    comparisons
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;
    use vocab::fib_cube::untyped_numbers;

    /// NO TYPED NUMBERS (operator 16 Oct 11:24 IST): this file's shipping part
    /// holds no numeric literal but 0 and 1.
    #[test]
    fn rank_types_no_number() {
        let typed = untyped_numbers(include_str!("rank.rs"));
        assert!(typed.is_empty(), "rank.rs types {typed:?}");
    }

    #[test]
    fn the_radix_is_five_passes_of_a_byte_over_side_and_move() {
        assert_eq!((BUCKETS, SIDE_BITS, PASSES, DIGIT_MASK), (256, 2, 5, 0xFF));
        assert_eq!(MEMBER_REFUSALS, 4);
        for side in [Side::Gainer, Side::Loser, Side::Neither] {
            assert_eq!(Side::of(side.bits() | u64::from(u32::MAX)), side);
        }
        assert_eq!(Side::of(u64::MAX), Side::Neither);
    }

    #[test]
    fn the_radix_is_stable_and_keeps_slot_order_inside_a_key() {
        let keys = [Side::Neither.bits(), 7, 3, 7, Side::Loser.bits()];
        let mut order = [0, 1, 2, 3, 4];
        let mut scratch = [0; 5];
        let mut counts = [0; BUCKETS];
        let work = radix(&mut order, &mut scratch, &keys, &mut counts);
        assert_eq!(order, [2, 1, 3, 4, 0]);
        assert_eq!(work, 5 * (15 + 512));
    }

    #[test]
    fn a_checked_write_past_its_slice_writes_nothing() {
        let mut counts = [3, 5];
        assert_eq!(next_place(&mut counts, 1), Some(5));
        assert_eq!(counts, [3, 6]);
        assert_eq!(next_place(&mut counts, 2), None, "past the counters");
        assert_eq!(counts, [3, 6]);
        write_at(&mut counts, None, 9);
        write_at(&mut counts, Some(2), 9);
        assert_eq!(counts, [3, 6], "nothing is written past the slice");
        write_at(&mut counts, Some(0), 9);
        assert_eq!(counts, [9, 6]);
    }
}

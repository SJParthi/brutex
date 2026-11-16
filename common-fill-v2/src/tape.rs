//! One instrument-day's seconds for the block law: the row view ([`Row`], [`SecIn`]), the O(1) second index ([`SecIndex`], sample-6's
//! `SecIx`), the O(1) range table ([`RangeMm`], sample-6's `RangeMm`), one block read ([`Block`], sample-6's `block_read` / `Blk`) and the
//! day as a cheap view ([`Tape`], sample-6's `Day70`). Step A0 (crate v2): crate v1's readings unchanged; the view also carries the gap
//! tables ([`crate::GapTables`]) and answers the O(1) reads of design 4.2 (`next_traded`, `last_traded_before`).

use crate::gaps::GapTables;
use crate::{BLOCK_US, FillErr, Paisa, Side, US_S, Units, Us};

/// One stored 1-second row as the host holds it (stamped at its second's start; the store's RECORDED prices, design S1). A host whose
/// rows are its own type implements this trait for it (no copy of the rows).
pub trait Row {
    /// The second's start (UTC microseconds).
    fn ts(&self) -> Us;
    /// The second's recorded HIGH (paisa).
    fn high(&self) -> Paisa;
    /// The second's recorded LOW (paisa).
    fn low(&self) -> Paisa;
    /// The second's traded volume (units; a host's negative volume is 0: not a print).
    fn vol(&self) -> Units;
}

/// The crate's own row (design 4.1 `SecIn`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SecIn {
    /// The second's start (UTC microseconds).
    pub ts: Us,
    /// Its recorded HIGH (paisa).
    pub high: Paisa,
    /// Its recorded LOW (paisa).
    pub low: Paisa,
    /// Its traded volume.
    pub vol: Units,
}

impl Row for SecIn {
    fn ts(&self) -> Us {
        self.ts
    }
    fn high(&self) -> Paisa {
        self.high
    }
    fn low(&self) -> Paisa {
        self.low
    }
    fn vol(&self) -> Units {
        self.vol
    }
}

/// THE print test (S2; sample-6's `block_read`, `worst_between`): a row with volume and 0 < LOW <= HIGH. CHANGE C2: a day holding a row
/// with volume and an invalid price is REFUSED ([`SecIndex::new`]); a row with volume 0 is no print anywhere (no price, no volume).
#[inline]
pub(crate) fn valid<R: Row>(s: &R) -> bool {
    s.vol() > 0 && s.high() > 0 && s.low() > 0 && s.high() >= s.low()
}

/// The longest index span accepted (4 days of seconds): a guard against an absurd span, never reached by a day's index.
const MAX_SPAN_S: i64 = 4 * 86_400;

/// The O(1) second index of one instrument-day (sample-6's `SecIx`, lifted): `ix[k]` = the first row stamped at or after `t0 + k` seconds
/// (k = 0..=n, n = the span in seconds + 1). Built once per day, after checking the rows (time order, whole seconds) and counting the rows
/// that are not prints.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SecIndex {
    t0: Us,
    ix: Vec<u32>,
    rows: usize,
    first: Option<Us>,
    last: Option<Us>,
    bad_print: u64,
    zero_volume: u64,
}

impl SecIndex {
    /// The index of `rows` over [t0, t1] (one pass, O(span seconds + rows)).
    ///
    /// # Errors
    /// [`FillErr::NotWholeSecond`] for `t0` or a row stamp not on a whole second; [`FillErr::Unsorted`] for rows out of time order (a
    /// repeated stamp is allowed: it is one more row of its second); [`FillErr::BadPrint`] for a row with volume and an invalid price (C2:
    /// the day is refused); [`FillErr::IndexMismatch`] for more rows than a u32 index holds; [`FillErr::SpanTooLong`] for a span longer
    /// than 4 days.
    pub fn new<R: Row>(rows: &[R], t0: Us, t1: Us) -> Result<Self, FillErr> {
        if t0.rem_euclid(US_S) != 0 {
            return Err(FillErr::NotWholeSecond);
        }
        let mut prev = Us::MIN;
        let bad_print = 0_u64;
        let mut zero_volume = 0_u64;
        for s in rows {
            let ts = s.ts();
            if ts.rem_euclid(US_S) != 0 {
                return Err(FillErr::NotWholeSecond);
            }
            if ts < prev {
                return Err(FillErr::Unsorted);
            }
            prev = ts;
            if s.vol() == 0 {
                zero_volume += 1;
            } else if !valid(s) {
                // CHANGE C2 (S2): fail closed: the instrument-day is refused (crate v1 ignored the row and counted it).
                return Err(FillErr::BadPrint);
            }
        }
        u32::try_from(rows.len()).map_err(|_| FillErr::IndexMismatch)?;
        let span = t1.checked_sub(t0).ok_or(FillErr::SpanTooLong)?.max(0) / US_S;
        if span > MAX_SPAN_S {
            return Err(FillErr::SpanTooLong);
        }
        // sample-6's SecIx::new, exactly.
        let n = usize::try_from(span).map_err(|_| FillErr::SpanTooLong)? + 1;
        let mut ix = Vec::with_capacity(n + 1);
        let mut i = 0_usize;
        let mut t = t0;
        for _ in 0..=n {
            while rows.get(i).is_some_and(|s| s.ts() < t) {
                i += 1;
            }
            ix.push(u32::try_from(i).map_err(|_| FillErr::IndexMismatch)?);
            t = t.checked_add(US_S).ok_or(FillErr::SpanTooLong)?;
        }
        Ok(Self { t0, ix, rows: rows.len(), first: rows.first().map(Row::ts), last: rows.last().map(Row::ts), bad_print, zero_volume })
    }

    /// The first row stamped at or after `ts`: O(1) inside the span (two whole-second reads), the binary search outside it (sample-6's
    /// `SecIx::at` / `Ctr::idx`).
    #[must_use]
    pub fn at<R: Row>(&self, rows: &[R], ts: Us) -> usize {
        if let Some(off) = ts.checked_sub(self.t0)
            && off >= 0
            && let Some(up) = off.checked_add(US_S - 1)
            && let Ok(k) = usize::try_from(up / US_S)
            && let Some(&i) = self.ix.get(k)
            && let Ok(i) = usize::try_from(i)
        {
            return i;
        }
        rows.partition_point(|s| s.ts() < ts)
    }

    /// The number of rows indexed.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// The index span's first second (a whole second).
    #[must_use]
    pub const fn t0(&self) -> Us {
        self.t0
    }

    /// The index span in whole seconds (the last indexed second is t0 + span - 1 s).
    #[must_use]
    pub const fn span_secs(&self) -> usize {
        self.ix.len().saturating_sub(2)
    }

    /// Rows with volume and an invalid price: always 0 in crate v2 (change C2: such a day is refused by [`SecIndex::new`]).
    #[must_use]
    pub const fn bad_print_rows(&self) -> u64 {
        self.bad_print
    }

    /// Rows with no volume (not prints).
    #[must_use]
    pub const fn zero_volume_rows(&self) -> u64 {
        self.zero_volume
    }

    fn fits<R: Row>(&self, rows: &[R]) -> bool {
        self.rows == rows.len() && self.first == rows.first().map(Row::ts) && self.last == rows.last().map(Row::ts)
    }
}

/// The O(1) range table of one instrument-day (sample-6's `RangeMm`, lifted): sparse tables of the index of the HIGHEST HIGH and of the
/// LOWEST LOW over any row range, two table reads a query, ties to the EARLIEST row (as a left-to-right scan with a strict comparison finds
/// them). Built over every stored row, in O(n log n), once per day when the host wants it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RangeMm {
    hi: Vec<Vec<u32>>,
    lo: Vec<Vec<u32>>,
    rows: usize,
}

impl RangeMm {
    /// The table of `rows` (a day of more than u32::MAX rows is indexed to u32::MAX and refused by [`Tape::new`]).
    #[must_use]
    pub fn new<R: Row>(rows: &[R]) -> Self {
        let n = rows.len().min(u32::MAX as usize);
        // C2 (S2): a row with no volume is no print: it never wins a range (and a range of only such rows has no price, `pick`).
        let h = |i: u32| usize::try_from(i).ok().and_then(|i| rows.get(i)).filter(|s| s.vol() > 0).map_or(i64::MIN, Row::high);
        let l = |i: u32| usize::try_from(i).ok().and_then(|i| rows.get(i)).filter(|s| s.vol() > 0).map_or(i64::MAX, Row::low);
        let base: Vec<u32> = (0..u32::try_from(n).unwrap_or(u32::MAX)).collect();
        let mut hi = vec![base.clone()];
        let mut lo = vec![base];
        let mut k = 1_usize;
        while (1_usize << k) <= n {
            let half = 1_usize << (k - 1);
            let m = n + 1 - (1_usize << k);
            let mut nh = Vec::with_capacity(m);
            let mut nlo = Vec::with_capacity(m);
            if let (Some(ph), Some(pl)) = (hi.get(k - 1), lo.get(k - 1)) {
                for i in 0..m {
                    let (a, b) = (ph.get(i).copied().unwrap_or(0), ph.get(i + half).copied().unwrap_or(0));
                    nh.push(if h(a) >= h(b) { a } else { b });
                    let (a, b) = (pl.get(i).copied().unwrap_or(0), pl.get(i + half).copied().unwrap_or(0));
                    nlo.push(if l(a) <= l(b) { a } else { b });
                }
            }
            hi.push(nh);
            lo.push(nlo);
            k += 1;
        }
        Self { hi, lo, rows: rows.len() }
    }

    fn pick<R: Row>(&self, rows: &[R], a: usize, b: usize, high: bool) -> Option<usize> {
        if b <= a || b > rows.len() {
            return None;
        }
        let len = b - a;
        let k = usize::try_from(len.ilog2()).ok()?;
        let t = if high { self.hi.get(k)? } else { self.lo.get(k)? };
        let i = usize::try_from(*t.get(a)?).ok()?;
        let j = usize::try_from(*t.get(b - (1_usize << k))?).ok()?;
        let (vi, vj) = (rows.get(i)?, rows.get(j)?);
        let key = |s: &R, high: bool| if s.vol() == 0 { if high { i64::MIN } else { i64::MAX } } else if high { s.high() } else { s.low() };
        let pick = if high {
            if key(vi, true) >= key(vj, true) { i } else { j }
        } else if key(vi, false) <= key(vj, false) {
            i
        } else {
            j
        };
        rows.get(pick).filter(|s| s.vol() > 0).map(|_| pick)
    }
}

/// One 2-second block of an order: [at, to) with `to` = at + 2 s, or clipped at the order's stop (crate v1's reading at step A0: no
/// segment clip, C6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Block {
    /// The block's start (on the order's own lattice).
    pub at: Us,
    /// Its end: at + 2 s, or the order's stop (clipped).
    pub to: Us,
    /// Whether the order's stop cut it.
    pub clipped: bool,
    /// The traded SECONDS of its volume seconds (S5, change C3; crate v1 counted the valid ROWS, a repeated stamp once more).
    pub n: u32,
    /// The traded seconds among them (distinct stamps): S1's count.
    pub n_secs: u8,
    /// The traded seconds whose merged HIGH equals their LOW (one recorded price): S1's count.
    pub one_price: u8,
    /// The highest HIGH of its valid rows (0 when none): a buy's price.
    pub high: Paisa,
    /// The lowest LOW of its valid rows (0 when none): a sale's price.
    pub low: Paisa,
    /// The price of its seconds before the stop alone (equal to `high` / `low` unless the block is clipped by its stop: C1's 1-second price,
    /// printed beside as `px_pre`).
    pub high_pre: Paisa,
    /// See `high_pre`.
    pub low_pre: Paisa,
    /// The traded volume of its valid rows.
    pub vol: Units,
    /// The instant its fill is known (S10, C1 / D29: the FULL block's end, min(at + 2 s, segment end), also for a block clipped by its stop).
    pub known: Us,
}

impl Block {
    /// The block's price for a side: a buy its highest HIGH, a sale its lowest LOW (nothing added).
    #[must_use]
    pub const fn px(&self, side: Side) -> Paisa {
        match side {
            Side::Buy => self.high,
            Side::Sell => self.low,
        }
    }
}

/// Where a lattice block lies against the session segments (C6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Gap {
    /// It overlaps a segment (or there is no session table).
    Inside,
    /// It lies wholly in a halt gap: the lattice block holding the next segment's open.
    Jump(Us),
    /// It lies after the day's last segment.
    After,
}

/// One continuous segment of the dated session table (design 4.1 `Seg`, S3): its first second, its last second and whether it opens with
/// a call auction (its first second's print is then in no block).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Seg {
    /// The segment's first second (a whole second).
    pub open: Us,
    /// Its last second (a whole second; the segment ends at last_sec + 1 s).
    pub last_sec: Us,
    /// Whether it opens with a call auction (the open second's print is the auction's, in no block).
    pub auction_open: bool,
}

impl Seg {
    /// The instant the segment ends (its last second + 1 s).
    #[must_use]
    pub const fn end(&self) -> Us {
        self.last_sec + crate::US_S
    }
}

/// One instrument-day for the block law: the host's rows, their [`SecIndex`], (when the host built them) their [`RangeMm`], the gap
/// tables of design 4.3 ([`GapTables`]), the dated session segments (S3) and the suspected feed holes. A view: O(1) to make, shared read
/// only across threads.
#[derive(Clone, Copy, Debug)]
pub struct Tape<'a, R: Row = SecIn> {
    rows: &'a [R],
    ix: &'a SecIndex,
    mm: Option<&'a RangeMm>,
    gaps: Option<&'a GapTables>,
    segs: &'a [Seg],
    holes: &'a [(Us, Us)],
}

impl<'a, R: Row> Tape<'a, R> {
    /// The view of `rows` with their index (and range table).
    ///
    /// # Errors
    /// [`FillErr::IndexMismatch`] when the index or the table was built from other rows (O(1): their count, first and last stamps).
    pub fn new(rows: &'a [R], ix: &'a SecIndex, mm: Option<&'a RangeMm>) -> Result<Self, FillErr> {
        if !ix.fits(rows) || mm.is_some_and(|m| m.rows != rows.len()) {
            return Err(FillErr::IndexMismatch);
        }
        Ok(Self { rows, ix, mm, gaps: None, segs: &[], holes: &[] })
    }

    /// The same view with the dated session segments (S3): sorted, apart, on whole seconds.
    ///
    /// # Errors
    /// [`FillErr::SegmentsInvalid`] for segments not sorted, overlapping, off whole seconds, or ending before they open.
    pub fn with_segments(self, segs: &'a [Seg]) -> Result<Self, FillErr> {
        let mut prev_end = Us::MIN;
        for s in segs {
            if s.open.rem_euclid(US_S) != 0 || s.last_sec.rem_euclid(US_S) != 0 || s.last_sec < s.open || s.open < prev_end {
                return Err(FillErr::SegmentsInvalid);
            }
            prev_end = s.end();
        }
        Ok(Self { segs, ..self })
    }

    /// The same view with the instrument-day's suspected feed holes ([`find_holes`]): the walk flags an order whose blocks span one.
    #[must_use]
    pub const fn with_holes(self, holes: &'a [(Us, Us)]) -> Self {
        Self { holes, ..self }
    }

    /// The segments (empty: no session table).
    #[must_use]
    pub const fn segments(&self) -> &'a [Seg] {
        self.segs
    }

    /// The suspected feed holes the host attached.
    #[must_use]
    pub const fn holes(&self) -> &'a [(Us, Us)] {
        self.holes
    }

    /// The end of the day's last segment (`Us::MAX` without a session table).
    #[must_use]
    pub fn session_end(&self) -> Us {
        self.segs.last().map_or(Us::MAX, Seg::end)
    }

    /// CHANGE C6 (S3): the part of the block [at, full) inside its segment, [lo, hi), and the segment's auction-print second: (at, full,
    /// None) without a session table; None when the block lies wholly outside every segment (a halt gap, before the open, after the end).
    pub(crate) fn seg_window(&self, at: Us, full: Us) -> Option<(Us, Us, Option<Us>)> {
        if self.segs.is_empty() {
            return Some((at, full, None));
        }
        let k = self.segs.partition_point(|s| s.end() <= at);
        let s = self.segs.get(k)?;
        (s.open < full).then(|| (at.max(s.open), full.min(s.end()), s.auction_open.then_some(s.open)))
    }

    /// CHANGE C6 (S3): where the lattice block at `at` lies: inside a segment, wholly in a halt gap (or before the first segment: the start
    /// of the lattice block of the same parity holding the next segment's open), or after the last segment.
    pub(crate) fn gap_jump(&self, at: Us) -> Gap {
        if self.segs.is_empty() {
            return Gap::Inside;
        }
        let k = self.segs.partition_point(|s| s.end() <= at);
        let Some(s) = self.segs.get(k) else { return Gap::After };
        let full = at.saturating_add(crate::BLOCK_US);
        if s.open < full {
            return Gap::Inside;
        }
        let steps = (s.open - at) / crate::BLOCK_US;
        Gap::Jump(at + steps * crate::BLOCK_US)
    }

    /// CHANGE C6 (S3): the end of the segment holding the second `at` (`Us::MAX` without a session table, `at` itself outside one).
    pub(crate) fn segment_end_at(&self, at: Us) -> Us {
        if self.segs.is_empty() {
            return Us::MAX;
        }
        let k = self.segs.partition_point(|s| s.end() <= at);
        self.segs.get(k).filter(|s| s.open <= at).map_or(at, Seg::end)
    }

    /// CHANGE C6 (S3, S11): the open of the segment holding the second `t` (None outside every segment; `Us::MIN` without a table).
    #[must_use]
    pub fn segment_open_at(&self, t: Us) -> Option<Us> {
        if self.segs.is_empty() {
            return Some(Us::MIN);
        }
        let k = self.segs.partition_point(|s| s.end() <= t);
        self.segs.get(k).filter(|s| s.open <= t).map(|s| s.open)
    }

    /// Whether the span [from, to) overlaps one of the attached suspected feed holes (a hole's silence is the open interval between its two
    /// traded seconds). O(holes).
    #[must_use]
    pub fn spans_hole(&self, from: Us, to: Us) -> bool {
        self.holes.iter().any(|&(a, b)| a < to && b > from)
    }

    /// The same view with the gap tables of design 4.3 (the walk then skips long unproductive runs in O(1) / O(log S); the answers are
    /// the same).
    ///
    /// # Errors
    /// [`FillErr::IndexMismatch`] when the tables were built over another day's index.
    pub fn with_gaps(self, g: &'a GapTables) -> Result<Self, FillErr> {
        if !g.fits(self.ix) {
            return Err(FillErr::IndexMismatch);
        }
        Ok(Self { gaps: Some(g), ..self })
    }

    /// The rows.
    #[must_use]
    pub const fn rows(&self) -> &'a [R] {
        self.rows
    }

    /// The second index.
    #[must_use]
    pub const fn index(&self) -> &'a SecIndex {
        self.ix
    }

    /// The gap tables, when the host built them.
    #[must_use]
    pub const fn gaps(&self) -> Option<&'a GapTables> {
        self.gaps
    }

    /// The first row stamped at or after `ts` (O(1) inside the index span).
    #[must_use]
    pub fn first_at(&self, ts: Us) -> usize {
        self.ix.at(self.rows, ts)
    }

    /// The first traded second (a valid print) at or after `t` (design 4.2; O(1) to the first row, then past any non-print rows).
    #[must_use]
    pub fn next_traded(&self, t: Us) -> Option<Us> {
        let i = self.first_at(t);
        self.rows.get(i..).and_then(|w| w.iter().find(|s| valid(*s))).map(Row::ts)
    }

    /// The last traded second stamped strictly before `t`, merged over a repeated stamp (max HIGH, min LOW, summed volume): S11's sizing
    /// reference and S8's protection LTP read it (O(1) to its row).
    #[must_use]
    pub fn last_traded_before(&self, t: Us) -> Option<SecIn> {
        let i = self.first_at(t);
        let w = self.rows.get(..i)?;
        let last = w.iter().rev().find(|s| valid(*s))?;
        let ts = last.ts();
        let mut out = SecIn { ts, high: last.high(), low: last.low(), vol: 0 };
        for s in w.iter().rev().take_while(|s| s.ts() == ts).filter(|s| valid(*s)) {
            out.high = out.high.max(s.high());
            out.low = out.low.min(s.low());
            out.vol = out.vol.saturating_add(s.vol());
        }
        Some(out)
    }

    /// The block [at, min(at + 2 s, stop)) (one index read, then [`Tape::block_from`]).
    ///
    /// # Errors
    /// [`FillErr::Overflow`] when its volume leaves u64.
    pub fn block(&self, at: Us, stop: Us) -> Result<Block, FillErr> {
        self.block_from(self.first_at(at), at, stop).map(|x| x.0)
    }

    /// sample-6's `block_read`, lifted: the block [at, min(at + 2 s, stop)) read from row `i` on (the first row at or after `at`): its
    /// VALID rows (a repeated stamp is one more row of its second) give its highest HIGH, lowest LOW and volume; an invalid row prices
    /// nothing. CHANGE C1 (S9, S10; D5, D29): a block CLIPPED by its order's stop takes its VOLUME (and its counts) from its seconds before
    /// the stop but its PRICE from its FULL two seconds [at, at + 2 s), the second at or after the stop included (the price of the seconds
    /// before the stop alone prints beside as `high_pre` / `low_pre`), and is known at the full block's end. O(1): the rows of two
    /// seconds. Returns the block and the row index after its seconds before the stop.
    ///
    /// # Errors
    /// [`FillErr::Overflow`] when its volume (or `at` + 2 s) leaves its type.
    pub fn block_from(&self, mut i: usize, at: Us, stop: Us) -> Result<(Block, usize), FillErr> {
        let full = at.checked_add(BLOCK_US).ok_or(FillErr::Overflow)?;
        // CHANGE C6 (S3): only the block's part inside its dated segment is read ([lo, hi)), the auction-print second excluded; a block
        // crossing the segment end is clipped there and known there (S10). Without a session table: [at, full), as crate v1.
        let Some((lo, hi, auction)) = self.seg_window(at, full) else {
            let to = full.min(stop);
            return Ok((Block { at, to, clipped: to < full, known: to, ..Block::default() }, i));
        };
        let to = hi.min(stop);
        let stop_clip = stop < hi;
        // C1 (D29): a stop-clipped block is known at its FULL end, min(at + 2 s, segment end) (no account step reads its price before its
        // second has ended).
        let mut b = Block { at, to, clipped: to < full, n: 0, n_secs: 0, one_price: 0, high: i64::MIN, low: i64::MAX, high_pre: 0, low_pre: 0, vol: 0, known: hi };
        let mut sec: Option<(Us, Paisa, Paisa)> = None;
        let close = |b: &mut Block, s: Option<(Us, Paisa, Paisa)>| {
            if let Some((_, h, l)) = s {
                b.n_secs = b.n_secs.saturating_add(1);
                if h == l {
                    b.one_price = b.one_price.saturating_add(1);
                }
            }
        };
        let mut first_vol: Option<Units> = None;
        while let Some(s) = self.rows.get(i) {
            let ts = s.ts();
            if ts >= to {
                break;
            }
            if ts >= lo && valid(s) && Some(ts) != auction {
                b.high = b.high.max(s.high());
                b.low = b.low.min(s.low());
                b.vol = b.vol.checked_add(s.vol()).ok_or(FillErr::Overflow)?;
                if first_vol.is_none() {
                    first_vol = Some(s.vol());
                }
                match sec {
                    Some((st, h, l)) if st == ts => sec = Some((st, h.max(s.high()), l.min(s.low()))),
                    _ => {
                        close(&mut b, sec);
                        sec = Some((ts, s.high(), s.low()));
                    }
                }
            }
            i += 1;
        }
        close(&mut b, sec);
        // CHANGE C3 (S5): n counts the block's traded SECONDS (crate v1, sample-6's `Blk::n`: its valid rows, a repeated stamp counting again).
        b.n = u32::from(b.n_secs);
        if b.n == 0 {
            b.high = 0;
            b.low = 0;
        }
        if cfg!(feature = "m4") {
            // MUTANT m4 (design 10.4): the take on per-second volume (the block's first row alone), not the block's.
            b.vol = first_vol.unwrap_or(0);
        }
        b.high_pre = b.high;
        b.low_pre = b.low;
        if stop_clip && b.n > 0 && !cfg!(feature = "m8") {
            // C1 (S9): the price over the full two seconds INSIDE THE SEGMENT (C6), the second at or after the stop included (its volume
            // belongs to the next order). MUTANT m8 (design 10.4) keeps crate v1's 1-second price.
            let mut j = i;
            while let Some(s) = self.rows.get(j) {
                if s.ts() >= hi {
                    break;
                }
                if valid(s) && Some(s.ts()) != auction {
                    b.high = b.high.max(s.high());
                    b.low = b.low.min(s.low());
                }
                j += 1;
            }
        }
        Ok((b, i))
    }

    /// sample-6's `Day70::extreme`, lifted: the highest HIGH (`high`) or the lowest LOW over the ROWS stamped in [from, to): (price, its
    /// stamp), the earliest of equals; through the range table when the tape has it (two reads), else a scan (the same answers). C2 (S2):
    /// a row with no volume is no print (crate v1 read every stored row; a day with an invalid print is refused by C2).
    #[must_use]
    pub fn extreme(&self, from: Us, to: Us, high: bool) -> Option<(Paisa, Us)> {
        let a = self.first_at(from);
        let b = self.first_at(to).max(a);
        let i = if let Some(mm) = self.mm {
            mm.pick(self.rows, a, b, high)
        } else {
            let w = self.rows.get(a..b).unwrap_or(&[]);
            let mut best: Option<usize> = None;
            for (k, s) in w.iter().enumerate() {
                if s.vol() == 0 {
                    continue;
                }
                let better = best.and_then(|x| w.get(x)).is_none_or(|bs| if high { s.high() > bs.high() } else { s.low() < bs.low() });
                if better {
                    best = Some(k);
                }
            }
            best.map(|k| a + k)
        }?;
        self.rows.get(i).map(|s| (if high { s.high() } else { s.low() }, s.ts()))
    }

    /// The first PRINT stamped in [from, to) whose HIGH reaches `level` (`high`: HIGH >= level) or whose LOW reaches it (LOW <= level):
    /// its stamp. Through the range table by binary lifting when the tape has it (O(log n) reads, counted in `steps`), else a scan.
    #[must_use]
    pub fn first_reach(&self, from: Us, to: Us, high: bool, level: Paisa, steps: &mut u32) -> Option<Us> {
        let a = self.first_at(from);
        let b = self.first_at(to).max(a);
        let key = |s: &R| if s.vol() == 0 { if high { i64::MIN } else { i64::MAX } } else if high { s.high() } else { s.low() };
        let hit = |v: i64| if high { v >= level } else { v <= level };
        if let Some(mm) = self.mm {
            let t = if high { &mm.hi } else { &mm.lo };
            let mut pos = a;
            for k in (0..t.len()).rev() {
                let len = 1_usize << k;
                if pos + len <= b {
                    *steps = steps.saturating_add(1);
                    let best = t.get(k).and_then(|r| r.get(pos)).and_then(|&i| usize::try_from(i).ok()).and_then(|i| self.rows.get(i));
                    if best.is_some_and(|s| !hit(key(s))) {
                        pos += len;
                    }
                }
            }
            *steps = steps.saturating_add(1);
            return self.rows.get(pos).filter(|_| pos < b).filter(|s| hit(key(s))).map(Row::ts);
        }
        let w = self.rows.get(a..b).unwrap_or(&[]);
        for s in w {
            *steps = steps.saturating_add(1);
            if hit(key(s)) {
                return Some(s.ts());
            }
        }
        None
    }

    /// sample-6's `worst_between`, lifted: the worst traded price of the VALID rows stamped in [from, to) (a sale's lowest LOW, a buy's
    /// highest HIGH), nothing added.
    #[must_use]
    pub fn worst_between(&self, from: Us, to: Us, sell: bool) -> Option<Paisa> {
        let a = self.first_at(from);
        let b = self.first_at(to);
        let w = self.rows.get(a..b.max(a)).unwrap_or(&[]);
        let it = w.iter().filter(|s| valid(*s));
        if sell { it.map(Row::low).min() } else { it.map(Row::high).max() }
    }
}

/// The gap above which a silence inside a segment is a suspected feed HOLE (S3, critic A F11), seconds.
pub const HOLE_GAP_S: i64 = 30;

/// CHANGE C6 (S3, critic A F11): the suspected feed holes of one instrument-day: on a day whose MEDIAN gap between consecutive traded
/// seconds (inside the segments) is 1 s or less, every silence longer than 30 s inside one segment, as (the last traded second before it,
/// the first after it). A segment end is never a hole (the daily CAS gap after 15:15 is a segment end in the table). O(rows), once per day.
#[must_use]
pub fn find_holes<R: Row>(tape: &Tape<'_, R>) -> Vec<(Us, Us)> {
    let segs = tape.segments();
    let mut gaps: Vec<(Us, Us)> = Vec::new();
    let (mut ones, mut all) = (0_u64, 0_u64);
    let mut prev: Option<(Us, Option<usize>)> = None;
    for s in tape.rows().iter().filter(|s| valid(*s)) {
        let ts = s.ts();
        let seg = if segs.is_empty() { Some(0) } else { segs.iter().position(|g| g.open <= ts && ts < g.end()) };
        if seg.is_none() {
            continue;
        }
        if let Some((p, ps)) = prev {
            if p == ts {
                continue;
            }
            if ps == seg {
                all += 1;
                if ts - p == US_S {
                    ones += 1;
                }
                if ts - p > HOLE_GAP_S * US_S {
                    gaps.push((p, ts));
                }
            }
        }
        prev = Some((ts, seg));
    }
    if all > 0 && 2 * ones >= all { gaps } else { Vec::new() }
}

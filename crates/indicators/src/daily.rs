//! The pivot ladder, the central pivot range, and yesterday's high and low.
//!
//! **41 vocabulary positions from one input**: the previous completed session's
//! high, low and close. Every level here is frozen before today's first bar
//! prints, so the per-bar cost is a comparison against a number already computed
//! — the cheapest family in the vocabulary and the one with no repaint risk at
//! all.
//!
//! # Where the formulas come from
//!
//! `docs/09-design-sources.md` §1, and D-0078 rules which R3 ladder the
//! vocabulary implements where two operator sources disagree by 8.67 points. The
//! recurrence, with `H`, `L`, `C` from the **previous completed** session:
//!
//! ```text
//! pivot = (H + L + C) / 3      r1 = 2*pivot - L      s1 = 2*pivot - H
//! bc    = (H + L) / 2          r2 = pivot + H - L    s2 = pivot - H + L
//! tc    = 2*pivot - bc         r3 = r1 + H - L       s3 = s1 - H + L
//!                              r4 = r3 + r2 - r1     s4 = s3 + s2 - s1
//!                              r5 = r4 + r3 - r2     s5 = s4 + s3 - s2
//! ```
//!
//! # The band
//!
//! `cpr_half = |pivot - bc|`, which is **half the CPR width** — the pivot is the
//! exact midpoint of `[bc, tc]` because `tc = 2*pivot - bc`. That is why
//! `vocab::tolerance::TOL_PIVOT_MILLI` is 500 and not 10 (D-0079), and why the
//! pivot's own band is exactly `[bc, tc]` — the identity that retired position 6.
//!
//! # Integer division, once, at the level boundary
//!
//! `pivot` divides by three and `bc` by two, and neither is exact. Both are
//! computed **once per session** with `div_euclid`, so the remainder is discarded
//! the same way on every platform and in both build profiles. `div_euclid` and
//! not `/`: truncation toward zero would round a negative level the other way,
//! and while an index price is never negative, a corrupt record can be — §7 gives
//! `i64::MIN` a meaning and the store's own sanity check lets it through.
//!
//! Rounding here is not on the evaluation path. The band test that follows is the
//! cross-multiplied integer form, so a level once fixed is compared exactly.

use crate::Candle;
use vocab::{ConditionMask, Tolerance};

/// A record that cannot yield a pivot ladder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unusable {
    /// `high < low` in the source session. A zero range is a real market state;
    /// a negative one is a broken record, and the two do not share a branch.
    HighBelowLow,
    /// `high - low` does not fit in an `i64`, so no level can be computed. Only
    /// reachable from a corrupt record — see [`crate::Corrupt::RangeOverflows`].
    RangeOverflows,
    /// A rung of the ladder does not fit an `i64`. The span fits and the record is
    /// ordered, so neither refusal above fires, but `r4 = r3 + r2 - r1` reaches past
    /// the type.
    ///
    /// Refused and not saturated, and the reason is not the sentinel: a saturated rung
    /// is a plausible price that is WRONG, and two of them are one predicate wearing two
    /// position numbers. At `H = i64::MAX / 2, L = 0, C = 0` R4 and R5 both pinned onto
    /// `i64::MAX`, which made positions 180/184 and 181/185 identical and set four band
    /// bits against prices the session never printed. `crate::CurDayFib::rung_level`
    /// already refuses the same overflow; this is that policy, applied once per crate
    /// instead of twice in opposite directions.
    LevelOverflows,
    /// The close sits outside `[low, high]`.
    ///
    /// `Evaluator::stepped` already refuses such a bar with
    /// [`crate::Corrupt::PriceOutsideRange`], and these constructors did not -- so the
    /// crate had two entry points that disagreed about what a valid session is, and the
    /// looser one fed the band arithmetic.
    ///
    /// It is not a theoretical gap. `(high: i64::MIN + 1, low: i64::MIN, close: i64::MAX)`
    /// passes `HighBelowLow` and `RangeOverflows`, and produces `band_half` =
    /// `6_148_914_691_236_517_205` -- two thirds of `i64::MAX`. The emit then doubles that
    /// for the band base, which clamped, handing `Tolerance::covers` a band **25% too
    /// narrow**, so twelve `near_*` bits read false on bars the real band covers.
    ///
    /// With the close inside the range the arithmetic is bounded by derivation rather than
    /// by luck. `pivot - bc = (2c - h - l) / 6`, and `|2c - h - l|` is largest when the
    /// close sits on an edge, where it equals `h - l`. So
    /// `band_half <= (h - l) / 6 <= i64::MAX / 6`, and doubling it reaches at most a third
    /// of the type. That is the same ceiling `cpr_width`'s doc derives, one step further
    /// along.
    CloseOutsideRange,
}

/// Where the cuts between a narrow, a neutral and a wide CPR fall, in thousandths of
/// the previous session's range.
///
/// **UNVERIFIED.** No tracked document defines either cut, and unlike a doji there is
/// no single classical value the literature agrees on. They are declared here rather
/// than refused, because refusing one convention while `crate::pattern` declares six
/// and `crate::trend` declares six more is not a rule — it is an accident of which
/// module was written first.
///
/// What is NOT a convention is the ceiling they sit under: `DailyLevels::cpr_width`
/// derives that a CPR is at most **333 thousandths** of the range, so these numbers
/// are meaningful as fractions of 333 and not of 1000.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CprWidth {
    /// At or below this fraction of the range, the CPR is narrow. 80 — the bottom
    /// quarter of the reachable 333.
    pub narrow: i64,
    /// At or above this fraction, wide. 250 — the top quarter of the reachable 333.
    pub wide: i64,
}

impl Default for CprWidth {
    fn default() -> Self {
        Self::CLASSICAL
    }
}

impl CprWidth {
    /// The declared set. **UNVERIFIED** — see the struct documentation.
    pub const CLASSICAL: Self = Self {
        narrow: 80,
        wide: 250,
    };
}

// Both cuts must sit inside the derived ceiling, or the bit above them can never be
// set and D-0080 would exclude it from every sweep as constant-false. A build error is
// the right place to learn that, not a support table.
const _: () = assert!(
    CprWidth::CLASSICAL.narrow < CprWidth::CLASSICAL.wide,
    "narrow must cut below wide, or the classes overlap"
);
const _: () = assert!(
    CprWidth::CLASSICAL.wide < 333,
    "a CPR is at most one third of the range, so a wide cut at or above 333 is \
     unreachable and position 274 would be constant false"
);

/// The three states a CPR's width can be in.
///
/// Three, not one. Position 63 `narrow_cpr_day` shipped alone, and one bit answers
/// half a question: the hit test is `(bits & mask) == mask` with no negation, so
/// "not narrow" cannot be expressed by leaving 63 clear. A sweep that wants "the CPR
/// was wide" needs a bit that says so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CprClass {
    /// Position 63. The two edges are close together.
    Narrow,
    /// Position 275. Neither narrow nor wide.
    Neutral,
    /// Position 274. The edges are far apart.
    Wide,
}

/// Every level the previous session fixes, computed once.
///
/// Frozen for the whole of today. Nothing here reads a bar of the current
/// session, so no position it decides can repaint and none can look ahead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DailyLevels {
    pdh: i64,
    pdl: i64,
    pivot: i64,
    bc: i64,
    tc: i64,
    r: [i64; 5],
    s: [i64; 5],
    half: i64,
}

const _: () = assert!(core::mem::size_of::<DailyLevels>() <= 128);

impl DailyLevels {
    /// Compute the ladder from the previous completed session's high, low and
    /// close.
    ///
    /// # Errors
    ///
    /// [`Unusable::HighBelowLow`] for a session whose high is below its low,
    /// [`Unusable::RangeOverflows`] when the span does not fit an `i64`, and
    /// [`Unusable::LevelOverflows`] when the span fits but a rung computed from it does
    /// not. The three are separate because the first two can be seen in the input and
    /// the third cannot: `H = i64::MAX / 2, L = 0, C = 0` passes both earlier gates.
    pub fn from_previous_session(high: i64, low: i64, close: i64) -> Result<Self, Unusable> {
        if high < low {
            return Err(Unusable::HighBelowLow);
        }
        // Before the range, because `close` outside `[low, high]` is a broken record and
        // not an extreme market -- and because everything downstream, including the band
        // base the emit doubles, is bounded only when this holds. See
        // `Unusable::CloseOutsideRange` for the derivation and the witness.
        if close < low || close > high {
            return Err(Unusable::CloseOutsideRange);
        }
        let Some(range) = high.checked_sub(low) else {
            return Err(Unusable::RangeOverflows);
        };

        // Widened before the sum, never after: `high + low + close` overflows an
        // i64 at index levels nowhere near i64::MAX, and `i128::from(a + b)` would
        // have already lost the value.
        let sum = i128::from(high) + i128::from(low) + i128::from(close);
        // The whole ladder in `i128`, narrowed once at the end. Every rung is derived
        // from `p` and never from a narrowed intermediate, so no rounding or clamping can
        // enter partway down the ladder.
        let p = sum.div_euclid(3);
        let b = (i128::from(high) + i128::from(low)).div_euclid(2);
        let rr = i128::from(range);
        let r1 = 2 * p - i128::from(low);
        let r2 = p + rr;
        let r3 = r1 + rr;
        let r4 = r3 + r2 - r1;
        let r5 = r4 + r3 - r2;
        let s1 = 2 * p - i128::from(high);
        let s2 = p - rr;
        let s3 = s1 - rr;
        let s4 = s3 + s2 - s1;
        let s5 = s4 + s3 - s2;

        // Narrowed at ONE site and destructured by pattern, rather than a `?` per level.
        // The clamp this replaced had a single body that all fourteen calls shared, so
        // whichever level left the type first exercised its overflow arm. Fourteen `?`s
        // would instead put thirteen branches here that no input can take: `p` and `b`
        // cannot leave `i64` at all — `|h+l+c|/3` and `|h+l|/2` are inside it by
        // construction for any `i64` inputs — and reaching the one on `s5` on its own
        // already needs a fixture built backwards from the recurrence. A branch nothing
        // can take is a branch nobody can be held to, which is why this crate refuses
        // `unreachable!` for the same reason.
        let [pivot, bc, tc, r1, r2, r3, r4, r5, s1, s2, s3, s4, s5, half] = fit([
            p,
            b,
            2 * p - b,
            r1,
            r2,
            r3,
            r4,
            r5,
            s1,
            s2,
            s3,
            s4,
            s5,
            (p - b).abs(),
        ])?;

        Ok(Self {
            pdh: high,
            pdl: low,
            pivot,
            bc,
            tc,
            r: [r1, r2, r3, r4, r5],
            s: [s1, s2, s3, s4, s5],
            half,
        })
    }

    /// The same, read off a completed daily bar.
    ///
    /// # Errors
    ///
    /// As [`Self::from_previous_session`].
    pub fn from_daily_bar(bar: &Candle) -> Result<Self, Unusable> {
        Self::from_previous_session(bar.high, bar.low, bar.close)
    }

    /// Yesterday's high.
    #[must_use]
    pub const fn pdh(&self) -> i64 {
        self.pdh
    }
    /// Yesterday's low.
    #[must_use]
    pub const fn pdl(&self) -> i64 {
        self.pdl
    }
    /// The pivot.
    #[must_use]
    pub const fn pivot(&self) -> i64 {
        self.pivot
    }
    /// The CPR's bottom-central level. **Not necessarily the lower of the two** —
    /// `bc` and `tc` are named by convention, and which is numerically higher
    /// depends on where the close sat in the range.
    #[must_use]
    pub const fn bc(&self) -> i64 {
        self.bc
    }
    /// The CPR's top-central level. See [`Self::bc`].
    #[must_use]
    pub const fn tc(&self) -> i64 {
        self.tc
    }
    /// Resistance `n`, 1-indexed. `None` past R5.
    #[must_use]
    pub fn resistance(&self, n: usize) -> Option<i64> {
        n.checked_sub(1).and_then(|i| self.r.get(i)).copied()
    }
    /// Support `n`, 1-indexed. `None` past S5.
    #[must_use]
    pub fn support(&self, n: usize) -> Option<i64> {
        n.checked_sub(1).and_then(|i| self.s.get(i)).copied()
    }
    /// The band half-width, `|pivot - bc|`. Zero when yesterday closed at the
    /// midpoint of its own range, which collapses every band to a line.
    #[must_use]
    pub const fn band_half(&self) -> i64 {
        self.half
    }
    /// The CPR's width — how far apart its two edges are.
    ///
    /// # The width is bounded, and the bound is derived rather than assumed
    ///
    /// `pivot = (h+l+c)/3` and `bc = (h+l)/2`, and `tc = 2·pivot − bc`, so
    ///
    /// ```text
    /// width = |tc − bc| = 2·|pivot − bc| = |2c − h − l| / 3
    /// ```
    ///
    /// `|2c − h − l|` is largest when the close sits on the high or the low, where it
    /// equals `h − l`. So **a CPR can never exceed one third of the previous session's
    /// range** — 333 thousandths, not 1000. Any threshold expressed against the range
    /// has to sit inside that ceiling, which is why [`CprWidth`]'s cut points are
    /// stated as fractions of 333 rather than of 1000: a "wide" cut of 500 would be
    /// unreachable and the bit would be constant false, exactly the defect D-0080
    /// exists to exclude.
    ///
    /// # The ceiling is exact arithmetic; the levels are rounded
    ///
    /// The derivation above is in rationals, and `pivot` and `bc` are each rounded once
    /// by `div_euclid`, so the **computed** width can land up to one paisa above
    /// `range / 3`. At `H=100 L=0 C=0` the levels are `pivot = 33`, `bc = 50`,
    /// `tc = 16`, and the width is 34 — 340 thousandths of a 100-paisa range, past the
    /// exact third. One paisa is only a large fraction of a range that small; at index
    /// scale the excess is unmeasurable, and both cuts sit far enough inside 333 that
    /// neither becomes unreachable either way. So 333 is a floor on the ceiling and not
    /// a hard bound on this function's output, and
    /// `the_ceiling_is_exact_arithmetic_and_rounding_can_pass_it` pins the arithmetic so
    /// a change of rounding policy is visible rather than inferred.
    #[must_use]
    pub const fn cpr_width(&self) -> i64 {
        let (low, high) = self.cpr_span();
        high.saturating_sub(low)
    }

    /// How wide the CPR is, against the previous session's range.
    ///
    /// `None` when the range is zero — a limit-locked session has no range to measure
    /// against, and every fraction of zero is the same fraction. Refusing is the answer
    /// rather than calling it narrow.
    #[must_use]
    pub fn cpr_class(&self, thresholds: CprWidth) -> Option<CprClass> {
        let range = self.pdh().saturating_sub(self.pdl());
        if range <= 0 {
            return None;
        }
        let width = i128::from(self.cpr_width());
        let scaled = i128::from(range);
        // Cross-multiplied, no division: `width <= range * permille / 1000` becomes
        // `width * 1000 <= range * permille`.
        if width.saturating_mul(1000) <= scaled.saturating_mul(i128::from(thresholds.narrow)) {
            return Some(CprClass::Narrow);
        }
        if width.saturating_mul(1000) >= scaled.saturating_mul(i128::from(thresholds.wide)) {
            return Some(CprClass::Wide);
        }
        Some(CprClass::Neutral)
    }

    /// The CPR's low and high edge, in numeric order.
    ///
    /// Ordered, because `bc` and `tc` are not: `tc = 2·pivot − bc`, so `tc` sits below
    /// `bc` on any session whose close is under the midpoint of its range — 48.51% of
    /// them. Three positions once compared the close against the raw pair while a
    /// fourth used this ordered one, and on those sessions the bar reported itself as
    /// above, inside AND below the same zone at once.
    #[must_use]
    pub const fn cpr_span(&self) -> (i64, i64) {
        if self.bc <= self.tc {
            (self.bc, self.tc)
        } else {
            (self.tc, self.bc)
        }
    }
}

/// Every level of one ladder from `i128` back to `i64`, or the refusal that names why
/// one of them will not fit.
///
/// All of them or none of them, which is the point: a ladder with one rung missing is
/// not a partial answer, it is fourteen positions measured against thirteen prices.
///
/// # It used to saturate, and saturating was the defect
///
/// The clamp it replaces pinned an out-of-range rung onto `i64::MAX` or `i64::MIN`
/// under a comment claiming "a level pinned at the extreme fails every band test".
/// That was true of `Rel::Near` alone, because `Tolerance::covers` refuses a
/// non-positive range, and false of all six plain `Above`/`Below` relations — which is
/// the majority of what this family decides. At `H = i64::MAX / 2, L = 0, C = 0` both
/// R4 and R5 pinned onto `i64::MAX`, so 180 and 184 became one predicate and 181 and
/// 185 another, and positions 181, 185, 182 and 186 fired against prices no session
/// printed. A sweep reading that mask sees a k=2 pair whose support equals both of its
/// k=1 supports and calls it a discovery.
///
/// So the whole ladder is refused instead. `crate::CurDayFib::rung_level` already
/// refuses this exact overflow with `ok()`; a crate that answered the same question two
/// opposite ways was the defect underneath the arithmetic, and `close_the_books` already
/// keeps yesterday's levels on a refusal rather than half-updating them.
///
/// Not `const`: `TryFrom` is not yet a const trait, and the alternative was an
/// `as` cast whose correctness rested on the branches beside it — the kind of
/// proof-by-adjacency that stops holding after an edit. Nothing calls this from a
/// const context, so the const-ness bought nothing.
fn fit<const N: usize>(wide: [i128; N]) -> Result<[i64; N], Unusable> {
    let mut out = [0_i64; N];
    // `zip` and not an index, because `clippy::indexing_slicing` is denied and it is
    // right: the two arrays share one const length here, and the next edit to this
    // function is where that would stop being true.
    for (slot, v) in out.iter_mut().zip(wide) {
        // One arm, not a sign test. The clamp needed `if v > 0` to choose which extreme
        // to pin at, and that branch is what made `i64::MIN` — which §7 gives a second
        // meaning — reachable as an ordinary price. A refusal has nothing to choose.
        *slot = i64::try_from(v).map_err(|_| Unusable::LevelOverflows)?;
    }
    Ok(out)
}

/// Every position this family decides, paired with the level it is measured
/// against and how.
///
/// Written out rather than computed. Position numbers belong to
/// `docs/03-vocabulary.md`, and a loop that derives them from a base index is one
/// off-by-one away from a mask that means something else entirely.
#[derive(Clone, Copy)]
enum Rel {
    /// `close` inside the level's band. Uses `vocab::table::set_near`.
    Near(u16),
    /// `close` strictly above the band's top edge.
    Above(u16),
    /// `close` strictly below the band's bottom edge.
    Below(u16),
    /// `close` strictly above the bare level, with no band. The shipped
    /// `close_above_pdh` family is named without `_band` and is `Kind::Plain`, so
    /// it is the bare comparison.
    AboveBare(u16),
    /// `close` strictly below the bare level.
    BelowBare(u16),
    /// `close` inside the CPR body, edges included.
    ///
    /// A separate relation because `inside_cpr` (62) is `Kind::Plain`, not
    /// `Kind::Near` — so it is an INTERVAL test against `[bc, tc]` and not a band
    /// test around the pivot. The two describe the same interval by the D-0079
    /// identity, and `vocab::table::set_near` correctly refuses a plain position,
    /// which is how the mistake was caught rather than shipped.
    InsideCpr(u16),
}

/// The whole family, as (level, relation) pairs.
fn plan(l: &DailyLevels) -> [(i64, Rel); 41] {
    let (r1, r2, r3, r4, r5) = (l.r[0], l.r[1], l.r[2], l.r[3], l.r[4]);
    let (s1, s2, s3, s4, s5) = (l.s[0], l.s[1], l.s[2], l.s[3], l.s[4]);
    // The CPR's edges in numeric order. Positions 60, 61 and 62 must all measure
    // against the same pair or they contradict each other on an inverted CPR.
    let (cpr_low, cpr_high) = l.cpr_span();
    [
        // 7–12: the shipped `near_pivot_*` for R1–R3 and S1–S3.
        (r1, Rel::Near(7)),
        (r2, Rel::Near(8)),
        (r3, Rel::Near(9)),
        (s1, Rel::Near(10)),
        (s2, Rel::Near(11)),
        (s3, Rel::Near(12)),
        // 13–18: yesterday's high and low. 13–16 are bare comparisons; 17–18 are
        // the banded `near_*` pair.
        (l.pdh, Rel::AboveBare(13)),
        (l.pdh, Rel::BelowBare(14)),
        (l.pdl, Rel::AboveBare(15)),
        (l.pdl, Rel::BelowBare(16)),
        (l.pdh, Rel::Near(17)),
        (l.pdl, Rel::Near(18)),
        // 54–55: R5 and S5's `near_*`, shipped without a formula until D-0078.
        (r5, Rel::Near(54)),
        (s5, Rel::Near(55)),
        // 60–62: the CPR body, all three measured against the SAME ordered edges.
        //
        // These read `l.tc` and `l.bc` raw, and that was wrong. `cpr_span()` exists
        // precisely because the design source lets `bc` exceed `tc` — that is what
        // an inverted CPR is, and the inverted arm governs 48.51% of real sessions.
        // On such a session the raw form made 60 ("above the CPR") compare against
        // the LOWER edge and 61 ("below the CPR") against the UPPER edge, so both
        // could hold while 62 said the close was INSIDE — three positions
        // describing one bar in two contradictory ways, on nearly half of all days.
        // 62 already went through the ordered accessor; 60 and 61 now do too.
        (cpr_high, Rel::AboveBare(60)),
        (cpr_low, Rel::BelowBare(61)),
        (l.pivot, Rel::InsideCpr(62)),
        // 74–85: the band sides for R1–R3 and S1–S3.
        (r1, Rel::Above(74)),
        (r1, Rel::Below(75)),
        (r2, Rel::Above(76)),
        (r2, Rel::Below(77)),
        (r3, Rel::Above(78)),
        (r3, Rel::Below(79)),
        (s1, Rel::Above(80)),
        (s1, Rel::Below(81)),
        (s2, Rel::Above(82)),
        (s2, Rel::Below(83)),
        (s3, Rel::Above(84)),
        (s3, Rel::Below(85)),
        // 178–187: R4 and S4 complete, plus the band sides R5 and S5 never had.
        (r4, Rel::Near(178)),
        (s4, Rel::Near(179)),
        (r4, Rel::Above(180)),
        (r4, Rel::Below(181)),
        (s4, Rel::Above(182)),
        (s4, Rel::Below(183)),
        (r5, Rel::Above(184)),
        (r5, Rel::Below(185)),
        (s5, Rel::Above(186)),
        (s5, Rel::Below(187)),
        // 188–189: BC and TC as levels in their own right.
        (l.tc, Rel::Near(188)),
        (l.bc, Rel::Near(189)),
    ]
}

/// Decide all 41 positions for one closing price.
///
/// # Cost
///
/// 41 comparisons against numbers fixed before the session opened. No loop whose
/// length depends on the data, no allocation, no division. `41` is a compile-time
/// constant, which is what makes this O(1) rather than "O(levels)".
/// Measured by `C-I-02` — one candle costs the same whatever it contains, in
/// both directions. `crates/indicators/benches/ratio.rs`.
///
/// # What it deliberately does not set
///
/// * **63 `narrow_cpr_day`** — no tracked document says how narrow is narrow, and
///   inventing a threshold is what `CLAUDE.md` §3 rule 1 forbids.
/// * **6 `near_pivot_p`** — a tombstone; the pivot's band is `inside_cpr` (62) by
///   the D-0079 identity, and that is the position set instead.
/// * **71 `near_fib_424`** — an orphan: 4.24 sits on none of the four ladders.
#[must_use]
pub fn bits(levels: &DailyLevels, close: i64, tolerance: Tolerance) -> ConditionMask {
    bits_with(levels, close, tolerance, CprWidth::CLASSICAL)
}

/// [`bits`], with the CPR-width cuts supplied rather than taken from the declared set.
///
/// Present so a caller can record which cuts a run used. The cuts are UNVERIFIED
/// (see [`CprWidth`]), and a result stamped with a threshold nobody can name is a
/// result nobody can reproduce.
#[must_use]
pub fn bits_with(
    levels: &DailyLevels,
    close: i64,
    tolerance: Tolerance,
    widths: CprWidth,
) -> ConditionMask {
    let mut mask = ConditionMask::ZERO;
    // 63 / 274 / 275 — exactly one of the three, or none when the previous session had
    // no range to measure against. `match` on the enum rather than an `if` chain, so
    // the compiler checks all three arms exist: this family is where "not narrow"
    // would otherwise quietly become "wide".
    if let Some(class) = levels.cpr_class(widths) {
        let index = match class {
            CprClass::Narrow => 63,
            CprClass::Wide => 274,
            CprClass::Neutral => 275,
        };
        // `unwrap_or(mask)` and not `if let Ok(next)`, here and at the two sites below:
        // `set_exact` refuses only a position that is absent, retired, void or `Near`,
        // and `the_plan_agrees_with_the_vocabulary` proves that none of the positions
        // this module names is any of those. An `if let` would therefore write a branch
        // no test can take — `cargo llvm-cov` counts it as a region that never runs, and
        // a region that cannot run is a region nobody can be held to. The behaviour is
        // identical: a refusal leaves the bit clear, which is
        // `docs/03-vocabulary.md` §4. It is also what the other five modules write.
        mask = vocab::table::set_exact(mask, index).unwrap_or(mask);
    }
    let half = levels.band_half();
    for (level, rel) in plan(levels) {
        let set = match rel {
            Rel::Near(index) => {
                // The band is a fraction of the CPR WIDTH, which is twice the
                // half-width — the base `vocab` expects for a pivot band.
                //
                // This CANNOT clamp, and the reason is a constructor guard rather than a
                // hope. `Unusable::CloseOutsideRange` refuses a close outside
                // `[low, high]`, and with that held `half = |2c - h - l| / 6` is at most
                // `(h - l) / 6 <= i64::MAX / 6`, so the double reaches a third of the
                // type. It clamped before that guard existed: at
                // `(i64::MIN + 1, i64::MIN, i64::MAX)` the band base came back 25% narrow
                // and twelve `near_*` bits read false on bars the band covers.
                //
                // `saturating_mul` stays rather than becoming a bare `*`. A bare multiply
                // panics if the derivation is ever wrong, and a derivation in this file has
                // been wrong before -- an earlier report claimed `cpr_width`'s
                // `saturating_sub` could not fire and it demonstrably could. Saturating is
                // the same answer with no panic and no branch to leave uncovered.
                let width = half.saturating_mul(2);
                mask = vocab::table::set_near(mask, index, tolerance, close, level, width)
                    .unwrap_or(mask);
                continue;
            }
            Rel::Above(index) => (close > level.saturating_add(half)).then_some(index),
            Rel::Below(index) => (close < level.saturating_sub(half)).then_some(index),
            Rel::AboveBare(index) => (close > level).then_some(index),
            Rel::BelowBare(index) => (close < level).then_some(index),
            Rel::InsideCpr(index) => {
                let (lo, hi) = levels.cpr_span();
                (lo <= close && close <= hi).then_some(index)
            }
        };
        if let Some(index) = set {
            mask = vocab::table::set_exact(mask, index).unwrap_or(mask);
        }
    }
    mask
}

/// Every position this module can set, for the test that proves it sets nothing
/// else.
#[must_use]
pub fn positions() -> [u16; 44] {
    let zero = DailyLevels {
        pdh: 0,
        pdl: 0,
        pivot: 0,
        bc: 0,
        tc: 0,
        r: [0; 5],
        s: [0; 5],
        half: 0,
    };
    let mut out = [0u16; 44];
    for (slot, (_, rel)) in out.iter_mut().zip(plan(&zero)) {
        *slot = match rel {
            Rel::Near(i)
            | Rel::Above(i)
            | Rel::Below(i)
            | Rel::AboveBare(i)
            | Rel::BelowBare(i)
            | Rel::InsideCpr(i) => i,
        };
    }
    // The CPR-width trio is not in `plan`, because it is decided from the levels
    // themselves rather than by comparing the close to a price. 63 shipped with the
    // original table; 274 and 275 were appended at NEXT_FREE.
    for (slot, index) in out.iter_mut().skip(WIDTH_STATES_AT).zip(WIDTH_STATES) {
        *slot = index;
    }
    out
}

/// Where the three CPR-width positions sit in [`positions`], after the 41 the plan
/// covers.
const WIDTH_STATES_AT: usize = 41;

/// The three CPR-width positions, in the order narrow, wide, neutral.
const WIDTH_STATES: [u16; 3] = [63, 274, 275];

const _: () = assert!(
    WIDTH_STATES_AT + WIDTH_STATES.len() == 44,
    "positions() must be exactly the plan's 41 plus the three width states"
);

#[cfg(test)]
// `expect` in a fixture, and the reason is not taste. `let Ok(x) = f() else {
// unreachable!() }` expands to a panic written IN THIS CRATE, so `cargo llvm-cov`
// records a region that no test can ever execute while the code is correct — and a
// region that cannot run is one nobody can be held to. `Result::expect` panics inside
// the standard library, which is not instrumented, and refuses just as loudly. The
// workspace denies `expect_used` for library code, where a panic is a real defect; the
// same allow appears on the test module of `core::price`, `greeks::bsm` and eleven
// others.
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn tol() -> Tolerance {
        vocab::tolerance::pinned_pivot().expect("the pivot width is pinned")
    }

    fn levels(h: i64, l: i64, c: i64) -> DailyLevels {
        DailyLevels::from_previous_session(h, l, c).expect("this fixture session is sane")
    }

    /// The recurrence, checked against hand arithmetic on numbers chosen so every
    /// division is exact — so a rounding change cannot hide a formula change.
    #[test]
    fn the_ladder_matches_the_design_source() {
        // H=100, L=90, C=98 -> pivot=96, bc=95, tc=97, range=10
        let x = levels(100, 90, 98);
        assert_eq!(x.pivot(), 96);
        assert_eq!(x.bc(), 95);
        assert_eq!(x.tc(), 97);
        assert_eq!(x.band_half(), 1, "|pivot - bc|");
        assert_eq!(x.resistance(1), Some(2 * 96 - 90));
        assert_eq!(x.resistance(2), Some(96 + 10));
        assert_eq!(x.resistance(3), Some(2 * 96 - 90 + 10));
        assert_eq!(x.resistance(4), Some(96 + 2 * 10), "r4 = P + 2(H-L)");
        assert_eq!(x.resistance(5), Some(2 * 96 + 2 * 100 - 3 * 90), "r5");
        assert_eq!(x.support(1), Some(2 * 96 - 100));
        assert_eq!(x.support(4), Some(96 - 2 * 10), "s4 = P - 2(H-L)");
        assert_eq!(x.support(5), Some(2 * 96 - 3 * 100 + 2 * 90), "s5");
        assert_eq!(x.resistance(6), None);
        assert_eq!(x.support(6), None);
        assert_eq!(x.resistance(0), None, "the ladder is 1-indexed");
    }

    /// The band half-width is always exactly half the CPR width — the identity
    /// A close outside the session's range is refused, not silently narrowed.
    ///
    /// # The witness, and why the BAND is what broke
    ///
    /// `(high: i64::MIN + 1, low: i64::MIN, close: i64::MAX)` passes `HighBelowLow` -- the
    /// high IS above the low -- and passes `RangeOverflows`, because the span is 1. It then
    /// produced `band_half` = `6_148_914_691_236_517_205`, and the emit doubles that for the
    /// band base `Tolerance::covers` expects. The double clamped at `i64::MAX`, which is
    /// three quarters of the real width, so twelve `near_*` positions read FALSE on bars
    /// the real band covers -- 42 verified (position, close) pairs.
    ///
    /// `daily` was the one family in this crate that clamped. `trend.rs`, `orb.rs`,
    /// `fib.rs` and `gap.rs` each reach the same arithmetic, each use `checked_*`, and each
    /// explain at length why. Abstaining is the wrong repair HERE, though, and that is
    /// worth stating: `bits` returns a mask and has no channel for "do not know", so an
    /// abstained `near_*` bit and a narrowed one are the same observable false. The refusal
    /// has to happen where a `Result` still exists, which is the constructor.
    #[test]
    fn a_close_outside_the_session_range_is_refused_rather_than_narrowing_the_band() {
        assert!(
            matches!(
                DailyLevels::from_previous_session(i64::MIN + 1, i64::MIN, i64::MAX),
                Err(Unusable::CloseOutsideRange)
            ),
            "the audit's witness must be refused, and by this name rather than another"
        );
        // Both directions, adjacent by one, so the boundary is exact rather than roughly
        // right -- a `<`/`<=` slip here would let the witness family back in.
        assert!(matches!(
            DailyLevels::from_previous_session(100, 10, 101),
            Err(Unusable::CloseOutsideRange)
        ));
        assert!(matches!(
            DailyLevels::from_previous_session(100, 10, 9),
            Err(Unusable::CloseOutsideRange)
        ));
        assert!(
            DailyLevels::from_previous_session(100, 10, 100).is_ok(),
            "a close ON the high is a real session, not a broken record"
        );
        assert!(
            DailyLevels::from_previous_session(100, 10, 10).is_ok(),
            "a close ON the low is a real session too"
        );

        // The consequence the refusal exists for, MEASURED rather than derived: for every
        // session this constructor accepts, doubling the band half stays inside the type.
        // The derivation is in `Unusable::CloseOutsideRange`; this is the check that the
        // derivation is not merely plausible.
        for (high, low) in [
            (i64::MAX, 0),
            (i64::MAX, i64::MIN),
            (0, i64::MIN),
            (i64::MAX, i64::MAX),
            (i64::MIN, i64::MIN),
            (1, 0),
            (2_500_000, 2_400_000),
        ] {
            let mid =
                i64::try_from(i128::midpoint(i128::from(low), i128::from(high))).unwrap_or(low);
            for close in [low, high, mid] {
                if let Ok(levels) = DailyLevels::from_previous_session(high, low, close) {
                    assert!(
                        levels.band_half().checked_mul(2).is_some(),
                        "({high}, {low}, {close}) built with band_half {} and doubling it \
                         leaves the type, so the emit's band base would clamp",
                        levels.band_half()
                    );
                }
            }
        }
    }

    /// D-0079 rests on, and the reason the pivot band IS the CPR body.
    #[test]
    fn the_band_is_always_half_the_cpr_width() {
        for (h, l, c) in [
            (100, 90, 98),
            (58_130, 57_870, 58_013),
            (2_510_000, 2_490_000, 2_508_000),
        ] {
            let x = levels(h, l, c);
            let (lo, hi) = x.cpr_span();
            assert_eq!(
                x.band_half() * 2,
                hi - lo,
                "the half-width is not half the CPR width at H={h} L={l} C={c}"
            );
        }
    }

    /// The degenerate day the sweep found: a close at the midpoint of the range
    /// makes pivot == bc, so every band collapses to a line.
    #[test]
    fn a_close_at_the_midpoint_collapses_every_band() {
        let x = levels(100, 90, 95);
        assert_eq!(x.pivot(), x.bc(), "pivot and bc must coincide here");
        assert_eq!(x.band_half(), 0);
        // A zero-width band means `set_near` sees a zero range and sets nothing,
        // which is `docs/03-vocabulary.md` §4: a bit that cannot be evaluated is
        // false, not "probably".
        let m = bits(&x, x.pivot(), tol());
        for index in [17_u16, 18, 188, 189] {
            assert!(!m.get(u32::from(index)), "banded position {index} fired");
        }
        // 62 is the exception, and deliberately so: it is an INTERVAL test, and a
        // collapsed CPR is the single point `[pivot, pivot]`, which the close is
        // exactly on. That is a real answer, not an unevaluated one.
        assert!(m.get(62), "a close exactly on a collapsed CPR is inside it");
        assert!(!m.get(60) && !m.get(61), "it is neither above nor below");
    }

    /// A session whose intermediate SUM needs `i128` still yields fourteen distinct rungs.
    ///
    /// # Why this test had to be written, and what its absence cost
    ///
    /// The fix that replaced clamping with `Unusable::LevelOverflows` also deleted
    /// `a_level_past_the_type_pins_at_the_extreme_of_its_own_sign`, and that test had a
    /// second job nobody noticed: it pinned the POSITIVE half of the near-edge arithmetic,
    /// where `R2` and `S1` came back exact rather than saturated. Its assertions could not
    /// be kept — the session it used now correctly refuses, because `R3 = R1 + range`
    /// leaves the type — and **nothing replaced them.**
    ///
    /// So after that fix the only near-edge sessions the suite still built were zero-range
    /// ones, where every rung collapses onto the high by design. Which means the comment
    /// three hundred lines up — *"Widened before the sum, never after"* — had no guard at
    /// all. `high + low + close` overflows an `i64` long before any of the three reaches
    /// `i64::MAX`, and if the widening were dropped the pivot would be computed from a
    /// wrapped sum: fourteen plausible levels, all wrong, and the refusal would not fire
    /// because each individual rung still fits.
    ///
    /// The fixture is chosen so the sum genuinely needs the widening and the ladder
    /// genuinely fits: `4e18 + 3e18 + 3.5e18` is `1.05e19`, past `i64::MAX` at `9.22e18`,
    /// while `r5` lands at `6e18` and `s5` at `1e18`, both inside.
    #[test]
    fn a_session_whose_sum_needs_the_widening_still_yields_distinct_rungs() {
        let high = 4_000_000_000_000_000_000_i64;
        let low = 3_000_000_000_000_000_000_i64;
        let close = 3_500_000_000_000_000_000_i64;

        // The premise: the sum really does need i128.
        assert!(
            high.checked_add(low)
                .and_then(|s| s.checked_add(close))
                .is_none(),
            "the fixture no longer overflows an i64 sum, so it does not test the widening"
        );

        let levels = DailyLevels::from_previous_session(high, low, close)
            .expect("the span fits and every rung fits, so the ladder is usable");

        // The pivot is the exact third of a sum that does not fit an i64.
        assert_eq!(
            levels.pivot(),
            3_500_000_000_000_000_000,
            "the pivot is not (h+l+c)/3, so the sum was narrowed before it was divided"
        );

        // Fourteen rungs, and no two of them share a price. A collapse is the defect the
        // refusal replaced: two vocabulary positions becoming one predicate, which the
        // sweep then reports as a k=2 pair whose support equals both k=1 supports.
        // The TEN LADDER RUNGS, and only those. `pdh`/`pdl`/`pivot` are deliberately not
        // in this set: `R1 = 2·pivot − low` is `4e18` on this fixture and `pdh` is also
        // `4e18`, which is arithmetic rather than a collapse — R1 coinciding with the
        // previous high is an ordinary thing for a pivot ladder to do. The defect the
        // refusal replaced was two RUNGS landing on one price because both saturated, so
        // the rungs are what must be pairwise distinct.
        let mut rungs: Vec<i64> = Vec::new();
        for n in 1..=5 {
            // `expect` and not `unwrap_or_else(|| panic!(..))`: the closure is a panic
            // this crate owns, which llvm-cov counts as a region no green run can close,
            // and the workspace denies `panic` outright. The rung number is in the
            // distinctness message below, so nothing is lost by dropping it here.
            rungs.push(levels.resistance(n).expect("R exists on a usable ladder"));
            rungs.push(levels.support(n).expect("S exists on a usable ladder"));
        }
        for (i, a) in rungs.iter().enumerate() {
            for (j, b) in rungs.iter().enumerate() {
                if i < j {
                    assert_ne!(
                        a, b,
                        "rungs {i} and {j} are both {a}, so two positions are one predicate"
                    );
                }
            }
            assert_ne!(
                *a,
                i64::MIN,
                "rung {i} is the §7 open-interest null sentinel sitting in a price field"
            );
            assert_ne!(*a, i64::MAX, "rung {i} is saturated, not computed");
        }
    }

    /// A corrupt session yields no ladder at all, by name.
    #[test]
    fn a_negative_range_and_an_overflowing_one_are_refused_separately() {
        assert_eq!(
            DailyLevels::from_previous_session(90, 100, 95),
            Err(Unusable::HighBelowLow)
        );
        assert_eq!(
            DailyLevels::from_previous_session(i64::MAX, i64::MIN, 0),
            Err(Unusable::RangeOverflows)
        );
    }

    /// Only the 41 positions this module owns are ever set.
    #[test]
    fn nothing_outside_the_forty_one_positions_is_set() {
        let owned = positions();
        let x = levels(2_510_000, 2_490_000, 2_508_000);
        let mut union = ConditionMask::ZERO;
        for step in -400..400 {
            union = union.union(&bits(&x, x.pivot() + step * 37, tol()));
        }
        for index in 0..ConditionMask::BITS {
            if union.get(index) {
                let as_u16 = u16::try_from(index).unwrap_or(u16::MAX);
                assert!(
                    owned.contains(&as_u16),
                    "position {index} was set and this module does not own it",
                );
            }
        }
    }

    /// Every position in the plan is live in the vocabulary, is the kind the
    /// relation assumes, and appears exactly once.
    #[test]
    fn the_plan_agrees_with_the_vocabulary() {
        let x = levels(100, 90, 98);
        let mut seen = std::collections::BTreeSet::new();
        for (_, rel) in plan(&x) {
            let (index, wants_near) = match rel {
                Rel::Near(i) => (i, true),
                Rel::Above(i)
                | Rel::Below(i)
                | Rel::AboveBare(i)
                | Rel::BelowBare(i)
                | Rel::InsideCpr(i) => (i, false),
            };
            assert!(
                seen.insert(index),
                "position {index} appears twice in the plan"
            );
            // Asserted before it is unwrapped, so the failure still names the position
            // that is missing: `expect` takes a string and cannot interpolate `index`,
            // and an `unwrap_or_else(|| panic!(...))` would put the message back in a
            // closure this crate owns and no test can enter.
            let found = vocab::table::definition(index);
            assert!(found.is_some(), "position {index} is not in the table");
            let def = found.expect("asserted to be Some on the line above");
            assert!(
                vocab::table::is_live(index),
                "position {index} (`{}`) is not live",
                def.name,
            );
            let is_near = def.kind == vocab::Kind::Near;
            // Both readings are computed here rather than inside the failure message:
            // an argument evaluated only on failure is a region that runs only on
            // failure, and `wants_near` takes both values across this loop.
            let treated_as = if wants_near { "banded" } else { "bare" };
            assert_eq!(
                is_near, wants_near,
                "position {index} (`{}`) is {:?} and the plan treats it as {treated_as}",
                def.name, def.kind,
            );
        }
        assert_eq!(seen.len(), 41);
    }

    /// The three positions this module deliberately leaves alone.
    #[test]
    fn the_undefined_and_retired_positions_are_untouched() {
        let owned = positions();
        // Only ONE position is still deliberately untouched, and it is a tombstone
        // rather than a refusal. Two others used to be on this list:
        //
        //   63 narrow_cpr_day — was refused for want of a threshold, while
        //      `crate::pattern` declared six UNVERIFIED thresholds and used them. That
        //      was an accident of which module was written first, not a rule. It is now
        //      computed, with cuts declared in `CprWidth` and bounded by the derived
        //      fact that a CPR cannot exceed a third of the range.
        //   71 near_fib_424 — was called "an orphan" while 2.618 shipped in three
        //      ladders. 4.236 is a member of the same classical extension set, so
        //      refusing it was arbitrary. It is now rung 4.236 of the bullish ladder.
        assert!(
            !owned.contains(&6),
            "position 6 is set, and it is a tombstone; inside_cpr (62) is set instead"
        );
        // And the reverse, so this test cannot pass by the family shrinking:
        for index in [63_u16, 274, 275] {
            assert!(
                owned.contains(&index),
                "position {index} is one of the three CPR-width states and must be owned"
            );
        }
    }

    /// Same levels and close, same bits, byte for byte.
    #[test]
    fn the_same_input_gives_the_same_bits() {
        let x = levels(2_510_000, 2_490_000, 2_508_000);
        let run = || {
            (0..500)
                .map(|k| bits(&x, x.pivot() + k * 13, tol()).words())
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    /// A price far above every level sets every `above` and no `below`, which is
    /// the cheapest proof the two relations are not transposed.
    #[test]
    fn a_price_above_everything_sets_only_the_above_relations() {
        let x = levels(100, 90, 98);
        let m = bits(&x, 10_000, tol());
        assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
        assert!(m.get(184) && m.get(186), "R5/S5 bands not above");
        assert!(m.get(13) && m.get(15), "pdh/pdl not above");
        assert!(m.get(60), "not above cpr tc");
        for below in [75_u32, 77, 79, 81, 83, 85, 181, 183, 185, 187, 14, 16, 61] {
            assert!(
                !m.get(below),
                "below-position {below} fired above everything"
            );
        }
    }

    /// The mirror of the test above: a price under every level sets every `below`
    /// relation and no `above` one.
    ///
    /// Without both halves, a transposition that set the `below` bits for an `above`
    /// price and nothing at all for a `below` price would pass — the above-test only
    /// proves the below-bits stay clear, never that they can fire.
    #[test]
    fn a_price_below_everything_sets_only_the_below_relations() {
        let x = levels(100, 90, 98);
        // S5 is the lowest rung of this ladder at 2*96 - 300 + 180 = 72.
        let m = bits(&x, -10_000, tol());
        assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
        assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
        assert!(m.get(181) && m.get(183), "R4/S4 bands not below");
        assert!(m.get(185) && m.get(187), "R5/S5 bands not below");
        assert!(m.get(14) && m.get(16), "pdh/pdl not below");
        assert!(m.get(61), "not below the CPR's low edge");
        for above in [74_u32, 76, 78, 80, 82, 84, 180, 182, 184, 186, 13, 15, 60] {
            assert!(
                !m.get(above),
                "above-position {above} fired below everything"
            );
        }
        for near in [7_u32, 8, 9, 10, 11, 12, 17, 18, 54, 55, 178, 179, 188, 189] {
            assert!(
                !m.get(near),
                "banded position {near} fired 10,000 paisa below its level"
            );
        }
    }

    /// The default cuts are the declared set, and `bits` uses them.
    ///
    /// Two things break without this. A `Default` that drifted from `CLASSICAL` would
    /// hand a run cuts no document declares, and `bits` — which is the entry point every
    /// caller uses — would silently class the CPR by them.
    #[test]
    fn the_default_cuts_are_the_declared_set() {
        assert_eq!(
            CprWidth::default(),
            CprWidth::CLASSICAL,
            "the default cuts drifted from the declared UNVERIFIED set"
        );
        assert_eq!(CprWidth::default().narrow, 80);
        assert_eq!(CprWidth::default().wide, 250);
        // 120 thousandths of the range: neutral under the declared cuts, so the two
        // entry points must agree on position 275 and not merely on "some bit".
        let x = levels(2_500_000, 2_490_000, 2_496_800);
        let plain = bits(&x, x.pivot(), tol());
        let supplied = bits_with(&x, x.pivot(), tol(), CprWidth::default());
        assert_eq!(
            plain.words(),
            supplied.words(),
            "`bits` does not use the declared cuts that `Default` names"
        );
        assert!(plain.get(275), "this fixture must be the neutral class");
    }

    /// A daily bar yields the ladder of its high, low and close — not its open.
    ///
    /// `from_daily_bar` is the accessor every caller with a bar in hand reaches for. If
    /// it transposed two of the seven fields the ladder would still build, every level
    /// would still be a number, and the whole family would be measured against the wrong
    /// prices. The pivot is asserted by value because that is where a transposition
    /// shows.
    #[test]
    fn a_daily_bar_yields_the_ladder_of_its_high_low_and_close() {
        // ts, open, high, low, close, volume, open interest. The open is deliberately
        // NOT the close, and the timestamp is deliberately meaningless: a ladder is
        // three prices and the previous session's identity is the caller's business.
        let bar = Candle::new(
            0,
            2_410_000,
            2_500_000,
            2_400_000,
            2_450_000,
            0,
            crate::OI_NULL,
        );
        let from_bar = DailyLevels::from_daily_bar(&bar).expect("a sane daily bar has a ladder");
        assert_eq!(
            from_bar,
            levels(2_500_000, 2_400_000, 2_450_000),
            "from_daily_bar disagrees with the same three prices passed directly"
        );
        assert_eq!(
            from_bar.pivot(),
            2_450_000,
            "the pivot must come from the CLOSE; the open would give 2,436,666"
        );
        // And a broken record is refused by name through this door too.
        let corrupt = Candle::new(0, 95, 90, 100, 95, 0, crate::OI_NULL);
        assert_eq!(
            DailyLevels::from_daily_bar(&corrupt),
            Err(Unusable::HighBelowLow),
            "a bar whose high is below its low must be refused, not measured"
        );
    }

    /// The three width classes fall exactly where the cuts say, and both cuts are
    /// inclusive.
    ///
    /// Every fixture divides exactly, so a rounding change cannot be mistaken for a
    /// threshold change. The two-paisa fixtures are the point: a cut compared with `<`
    /// instead of `<=`, or scaled against 1000 instead of the range, moves the boundary
    /// and only a test that sits ON it can tell.
    #[test]
    fn the_three_width_classes_fall_where_the_cuts_say() {
        let cuts = CprWidth::CLASSICAL;
        // Range 10,000. bc = 2,495,000, so the width is 2*|pivot - 2,495,000|.
        let at_the_narrow_cut = levels(2_500_000, 2_490_000, 2_496_200);
        assert_eq!(at_the_narrow_cut.cpr_width(), 800, "80/1000 of 10,000");
        assert_eq!(
            at_the_narrow_cut.cpr_class(cuts),
            Some(CprClass::Narrow),
            "the narrow cut is inclusive: a width exactly at 80 thousandths is narrow"
        );

        let two_paisa_wider = levels(2_500_000, 2_490_000, 2_496_203);
        assert_eq!(two_paisa_wider.cpr_width(), 802);
        assert_eq!(
            two_paisa_wider.cpr_class(cuts),
            Some(CprClass::Neutral),
            "802 of 10,000 is past the narrow cut and must not be narrow"
        );

        let at_the_wide_cut = levels(2_500_000, 2_490_000, 2_498_750);
        assert_eq!(at_the_wide_cut.cpr_width(), 2_500, "250/1000 of 10,000");
        assert_eq!(
            at_the_wide_cut.cpr_class(cuts),
            Some(CprClass::Wide),
            "the wide cut is inclusive: a width exactly at 250 thousandths is wide"
        );

        let two_paisa_narrower = levels(2_500_000, 2_490_000, 2_498_747);
        assert_eq!(two_paisa_narrower.cpr_width(), 2_498);
        assert_eq!(
            two_paisa_narrower.cpr_class(cuts),
            Some(CprClass::Neutral),
            "2,498 of 10,000 is short of the wide cut and must not be wide"
        );

        // And the same width on an INVERTED CPR is the same class: the width is read
        // through `cpr_span`, so `bc > tc` cannot make it negative and collapse the
        // class to narrow.
        let inverted = levels(2_500_000, 2_490_000, 2_491_250);
        // Read into locals rather than called inside the message: an argument computed
        // only on failure is a region that only runs on failure, and the message reads
        // exactly the same.
        let (bc, tc) = (inverted.bc(), inverted.tc());
        assert!(
            bc > tc,
            "this fixture must be inverted or it tests nothing: bc={bc} tc={tc}"
        );
        assert_eq!(inverted.cpr_width(), 2_500, "a width is never signed");
        assert_eq!(inverted.cpr_class(cuts), Some(CprClass::Wide));
    }

    /// `bits_with` classes the CPR by the cuts it is handed, not by the declared ones.
    ///
    /// This is the whole reason the function is public: a run records which cuts it used,
    /// and cuts that are ignored are worse than cuts that are UNVERIFIED. One fixture,
    /// three cut sets, three different positions — and the other two must stay clear,
    /// because `(bits & mask) == mask` has no negation and two width bits at once would
    /// mean "narrow and wide" to every sweep that reads them.
    #[test]
    fn a_supplied_cut_decides_the_class_and_not_the_classical_one() {
        // 1,200 of a 10,000 range: 120 thousandths.
        let x = levels(2_500_000, 2_490_000, 2_496_800);
        assert_eq!(x.cpr_width(), 1_200);
        let close = x.pivot();
        for (cuts, expected, others) in [
            (CprWidth::CLASSICAL, 275_u32, [63_u32, 274]),
            (
                CprWidth {
                    narrow: 130,
                    wide: 300,
                },
                63,
                [274, 275],
            ),
            (
                CprWidth {
                    narrow: 50,
                    wide: 100,
                },
                274,
                [63, 275],
            ),
        ] {
            let m = bits_with(&x, close, tol(), cuts);
            assert!(
                m.get(expected),
                "cuts {}/{} must class a 120-thousandth CPR at position {expected}",
                cuts.narrow,
                cuts.wide
            );
            for other in others {
                assert!(
                    !m.get(other),
                    "cuts {}/{} set position {other} as well as {expected}",
                    cuts.narrow,
                    cuts.wide
                );
            }
        }
    }

    /// A session with no range has no width class at all.
    ///
    /// A limit-locked or untraded session prints `high == low`. Its CPR width is zero,
    /// and zero is at or below every cut — so the arithmetic alone would call it the
    /// narrowest possible day and set position 63 on a day with nothing to measure. The
    /// refusal is the answer, and `docs/03-vocabulary.md` §4 makes an unevaluable bit
    /// false rather than "probably".
    #[test]
    fn a_zero_range_session_has_no_width_class() {
        let flat = levels(2_500_000, 2_500_000, 2_500_000);
        assert_eq!(flat.cpr_width(), 0, "a flat session collapses the CPR");
        assert_eq!(
            flat.cpr_class(CprWidth::CLASSICAL),
            None,
            "a zero range must be refused, not classed narrow because 0 <= 80"
        );
        let m = bits(&flat, 2_500_000, tol());
        for index in [63_u32, 274, 275] {
            assert!(
                !m.get(index),
                "width position {index} fired on a session with no range"
            );
        }
    }

    /// A ladder with a rung past the end of `i64` is refused whole, and never handed
    /// out with two rungs pinned onto one price.
    ///
    /// Reachable only from a corrupt previous session, because no market prints a price
    /// within a factor of a billion of `i64`. The measurement is what makes it worth a
    /// refusal rather than a clamp. At `H = i64::MAX / 2, L = 0, C = 0` the span fits
    /// and neither older refusal fires, so the ladder was built with R4 and R5 both
    /// saturated onto `i64::MAX`: positions 180/184 and 181/185 became the SAME
    /// predicate, and 181, 185, 182 and 186 all fired against prices the session never
    /// printed. The sweep cannot see that from the mask — a saturated level is a number
    /// like any other, and a k=2 pair whose support equals both of its k=1 supports is
    /// the invented discovery `vwap.rs` refuses by name.
    ///
    /// `crate::CurDayFib::rung_level` already refuses this exact overflow with `ok()`.
    /// One crate held two opposite policies for it, which is the defect underneath the
    /// arithmetic.
    #[test]
    fn a_rung_past_the_type_refuses_the_whole_ladder() {
        for (h, l, c, rung) in [
            // The measured session: R4 and R5 both pinned onto `i64::MAX`.
            (i64::MAX / 2, 0, 0, "R4 and R5 both leave the type upward"),
            // Symmetric about zero, so R4 and S4 leave the type in opposite directions
            // on the same session. R4 is evaluated first, which is why the row below
            // exists as well.
            (
                2_500_000_000_000_000_000,
                -2_500_000_000_000_000_000,
                0,
                "R4 and S4 both leave the type",
            ),
            // The first rung computed is already out, so nothing downstream is reached.
            (
                i64::MAX,
                0,
                i64::MAX,
                "R1 leaves the type before any other rung",
            ),
            // The DOWNWARD direction on its own, and it is not decoration: with
            // R = 3e18 and P = -2e18 every resistance rung, the pivot, `bc`, `tc` and
            // the band half all fit, and `s5 = 2P - H - 2R` is -1e19. A refusal that
            // covered only the upward direction — the shape the old clamp's `if v > 0`
            // branch invited — passes every row above this one and fails here.
            (
                0,
                -3_000_000_000_000_000_000,
                -3_000_000_000_000_000_000,
                "S5 alone leaves the type, downward",
            ),
        ] {
            assert_eq!(
                DailyLevels::from_previous_session(h, l, c),
                Err(Unusable::LevelOverflows),
                "({h}, {l}, {c}): {rung}, and a saturated rung is a plausible price \
                 that is wrong"
            );
        }
        // The refusal is not blanket, or this test would pass on a
        // `from_previous_session` that refused every session at the edges of the type.
        // Every rung of this one IS `i64::MAX`, computed and not saturated: the range is
        // zero, so each rung reduces to the high. Refusing it would be the mirror defect.
        let ceiling = DailyLevels::from_previous_session(i64::MAX, i64::MAX, i64::MAX)
            .expect("a zero-range session at the ceiling has a ladder: every rung is H");
        assert_eq!(
            ceiling.resistance(5),
            Some(i64::MAX),
            "R5 = H at zero range"
        );
        assert_eq!(ceiling.support(5), Some(i64::MAX), "S5 = H at zero range");
        assert_eq!(ceiling.band_half(), 0, "a zero range collapses the band");
    }

    /// The one-third ceiling is exact arithmetic; the levels are rounded, and the
    /// rounding can pass it by a paisa.
    ///
    /// `cpr_width`'s derivation says a CPR is at most `range / 3`. It divides twice with
    /// `div_euclid`, and at `H=100 L=0 C=0` the result is 34 against a 100-paisa range —
    /// 340 thousandths, past the exact third and past the 333 the documentation rounds it
    /// to. It matters to nothing at index scale, where a paisa of slack is unmeasurable
    /// against a range of thousands, and both cuts stay reachable either way. This test
    /// exists so that the claim is a measured one and so a change of rounding policy
    /// shows up here rather than in a support table.
    #[test]
    fn the_ceiling_is_exact_arithmetic_and_rounding_can_pass_it() {
        let x = levels(100, 0, 0);
        assert_eq!(x.pivot(), 33, "(100 + 0 + 0) / 3 floors to 33");
        assert_eq!(x.bc(), 50, "(100 + 0) / 2 is exact");
        assert_eq!(x.tc(), 16, "2 * 33 - 50");
        assert_eq!(x.cpr_width(), 34, "50 - 16, the ordered span");
        let range = x.pdh() - x.pdl();
        assert_eq!(range, 100);
        // The width is read into a local rather than called inside the message: an
        // argument computed only on failure is a region that only runs on failure, and
        // the interpolation reads the same.
        let width = x.cpr_width();
        assert!(
            width * 1000 > range * 333,
            "the documented 333-thousandth ceiling is exact arithmetic: the computed \
             width is {width} of {range}, and if that ever stops being true this test \
             should be deleted, not relaxed"
        );
        assert_eq!(
            x.cpr_class(CprWidth::CLASSICAL),
            Some(CprClass::Wide),
            "340 thousandths is past the 250 cut whatever the rounding"
        );
    }

    /// The derives are exercised, because a derive nothing calls is a derive nobody has
    /// checked.
    ///
    /// `Debug` is the one that earns its place: these types are what a refusal carries
    /// into a log line, and a rendering that dropped the variant name or the level would
    /// make a corrupt session indistinguishable from an overflowing one after the fact.
    #[test]
    fn the_derives_render_and_copy_what_they_claim() {
        // All THREE refusals, rendered and compared pairwise. A log line is the only
        // place a corrupt session is ever seen, and `LevelOverflows` differs from
        // `RangeOverflows` by one word: an operator who cannot tell them apart cannot
        // tell "the span was impossible" from "the span was fine and a rung was not".
        let high_below_low = format!("{:?}", Unusable::HighBelowLow);
        let bad_span = format!("{:?}", Unusable::RangeOverflows);
        let bad_rung = format!("{:?}", Unusable::LevelOverflows);
        for (text, name) in [
            (&high_below_low, "HighBelowLow"),
            (&bad_span, "RangeOverflows"),
            (&bad_rung, "LevelOverflows"),
        ] {
            assert!(
                text.contains(name),
                "a refusal must name itself, and this rendered as {text}"
            );
        }
        assert_ne!(
            high_below_low, bad_span,
            "two refusals render identically and a log cannot tell them apart"
        );
        assert_ne!(
            bad_span, bad_rung,
            "an overflowing span and an overflowing rung render identically"
        );
        assert_eq!(
            Unusable::HighBelowLow,
            Clone::clone(&Unusable::HighBelowLow)
        );
        assert_eq!(
            Unusable::LevelOverflows,
            Clone::clone(&Unusable::LevelOverflows)
        );

        let cuts = format!("{:?}", CprWidth::CLASSICAL);
        assert!(
            cuts.contains("narrow: 80") && cuts.contains("wide: 250"),
            "the declared cuts must be readable off the rendering, not inferred: {cuts}"
        );
        assert_eq!(CprWidth::CLASSICAL, Clone::clone(&CprWidth::CLASSICAL));

        for (class, name) in [
            (CprClass::Narrow, "Narrow"),
            (CprClass::Neutral, "Neutral"),
            (CprClass::Wide, "Wide"),
        ] {
            let rendered = format!("{class:?}");
            assert_eq!(rendered, name, "a width class must render as its own name");
            assert_eq!(class, Clone::clone(&class));
        }
        assert_ne!(CprClass::Narrow, CprClass::Wide);

        let x = levels(100, 90, 98);
        let rendered = format!("{x:?}");
        assert!(
            rendered.contains("pivot: 96") && rendered.contains("half: 1"),
            "the ladder must render its own levels: {rendered}"
        );
        assert_eq!(x, Clone::clone(&x), "a copy of a ladder is the same ladder");

        // `Rel` is private and Copy, and `positions()` is the only reader of the
        // or-pattern below. A clone that dropped the position number would be a mask
        // that means something else entirely.
        //
        // EVERY entry is cloned, not just the first. Six alternatives share one arm, and
        // a single entry exercises exactly one of them — the other five are then patterns
        // no test enters, which `cargo llvm-cov` counts and nobody can be held to. The
        // plan carries all six relations, so walking it walks the whole pattern.
        let cloned: Vec<u16> = plan(&x)
            .into_iter()
            .map(|(_, rel)| match Clone::clone(&rel) {
                Rel::Near(i)
                | Rel::Above(i)
                | Rel::Below(i)
                | Rel::AboveBare(i)
                | Rel::BelowBare(i)
                | Rel::InsideCpr(i) => i,
            })
            .collect();
        assert_eq!(
            cloned.first().copied(),
            Some(7),
            "the plan opens on near_pivot_r1, position 7"
        );
        // The same 41 numbers `positions()` reads through the same pattern, so a clone
        // that lost a position number disagrees with the array the sweep is handed.
        let from_positions: Vec<u16> = positions().into_iter().take(WIDTH_STATES_AT).collect();
        assert_eq!(
            cloned, from_positions,
            "cloning a plan entry changed the position it names"
        );
    }
}

#[cfg(test)]
// See the note on `mod tests`: an `unreachable!` fixture is a region this crate owns
// and no test can enter, and `expect` refuses just as loudly from inside the standard
// library.
#[allow(clippy::expect_used)]
mod inverted_cpr {
    use super::*;

    /// On an inverted CPR the three body positions must agree with each other.
    ///
    /// An inverted CPR is one where `bc > tc`, which the design source permits and
    /// which `cpr_span()` exists to normalise. It is not rare: it governs 48.51% of
    /// real sessions.
    ///
    /// Positions 60 and 61 used to read `l.tc` and `l.bc` raw, so on an inverted
    /// session 60 ("close is above the CPR") compared against the LOWER edge and 61
    /// ("below the CPR") against the UPPER edge. A close sitting inside the CPR
    /// satisfied both — while 62 correctly said "inside". Three positions, one bar,
    /// two contradictory descriptions.
    ///
    /// The assertion is the mutual exclusion itself rather than the individual bits,
    /// because that is the property the sweep depends on: at most one of
    /// above / inside / below can hold.
    #[test]
    fn the_three_cpr_body_positions_never_contradict_each_other() {
        let tol = vocab::tolerance::pinned_pivot().expect("the pinned pivot tolerance is valid");
        // A session whose close is well below its high-low midpoint inverts the CPR
        // (bc > tc), because tc = 2*pivot - bc puts tc below bc.
        let levels = DailyLevels::from_previous_session(2_500_000, 2_400_000, 2_410_000)
            .expect("a sane session yields a ladder");
        let (low, high) = levels.cpr_span();
        // Read into locals rather than called inside the message: an argument computed
        // only on failure is a region that only runs on failure.
        let (bc, tc) = (levels.bc(), levels.tc());
        assert!(
            bc > tc,
            "this fixture must be INVERTED or it tests nothing: bc={bc} tc={tc}"
        );

        // Walk the close across the whole CPR and past both edges.
        let mut span = low - 5_000;
        while span <= high + 5_000 {
            let bits = bits(&levels, span, tol);
            let above = bits.get(60);
            let below = bits.get(61);
            let inside = bits.get(62);
            let holding = u32::from(above) + u32::from(below) + u32::from(inside);
            assert!(
                holding <= 1,
                "close {span}: above={above} below={below} inside={inside} — at most \
                 one may hold, and the sweep's pruning assumes it"
            );
            span += 1_000;
        }
    }
}

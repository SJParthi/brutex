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

use store::format::Bar;
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
    /// [`Unusable::HighBelowLow`] for a session whose high is below its low, and
    /// [`Unusable::RangeOverflows`] when the span does not fit an `i64`.
    pub fn from_previous_session(high: i64, low: i64, close: i64) -> Result<Self, Unusable> {
        if high < low {
            return Err(Unusable::HighBelowLow);
        }
        let Some(range) = high.checked_sub(low) else {
            return Err(Unusable::RangeOverflows);
        };

        // Widened before the sum, never after: `high + low + close` overflows an
        // i64 at index levels nowhere near i64::MAX, and `i128::from(a + b)` would
        // have already lost the value.
        let sum = i128::from(high) + i128::from(low) + i128::from(close);
        let pivot = clamp_i64(sum.div_euclid(3));
        let bc = clamp_i64((i128::from(high) + i128::from(low)).div_euclid(2));
        let tc = clamp_i64(2 * i128::from(pivot) - i128::from(bc));

        let p = i128::from(pivot);
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

        Ok(Self {
            pdh: high,
            pdl: low,
            pivot,
            bc,
            tc,
            r: [
                clamp_i64(r1),
                clamp_i64(r2),
                clamp_i64(r3),
                clamp_i64(r4),
                clamp_i64(r5),
            ],
            s: [
                clamp_i64(s1),
                clamp_i64(s2),
                clamp_i64(s3),
                clamp_i64(s4),
                clamp_i64(s5),
            ],
            half: clamp_i64((i128::from(pivot) - i128::from(bc)).abs()),
        })
    }

    /// The same, read off a completed daily bar.
    ///
    /// # Errors
    ///
    /// As [`Self::from_previous_session`].
    pub fn from_daily_bar(bar: &Bar) -> Result<Self, Unusable> {
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
    /// The CPR's low and high edge, in numeric order.
    #[must_use]
    pub const fn cpr_span(&self) -> (i64, i64) {
        if self.bc <= self.tc {
            (self.bc, self.tc)
        } else {
            (self.tc, self.bc)
        }
    }
}

/// `i128` back to `i64`, saturating.
///
/// Not `const`: `TryFrom` is not yet a const trait, and the alternative was an
/// `as` cast whose correctness rested on the two branches beside it — the kind of
/// proof-by-adjacency that stops holding after an edit. Nothing calls this from a
/// const context, so the const-ness bought nothing.
///
/// Saturating and not wrapping: a level beyond `i64` is unreachable from real
/// prices, and if a corrupt record produces one, a level pinned at the extreme
/// fails every band test rather than wrapping to a plausible-looking price in the
/// middle of the range.
fn clamp_i64(v: i128) -> i64 {
    // `match` on the fallible conversion rather than `as`: `clippy::cast_possible_
    // truncation` is denied in this workspace, and the lint is right — an `as`
    // here would be correct only because of the two branches above it, which is
    // exactly the kind of proof-by-adjacency that stops being true after an edit.
    match i64::try_from(v) {
        Ok(x) => x,
        Err(_) => {
            if v > 0 {
                i64::MAX
            } else {
                i64::MIN
            }
        }
    }
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
    let mut mask = ConditionMask::ZERO;
    let half = levels.band_half();
    for (level, rel) in plan(levels) {
        let set = match rel {
            Rel::Near(index) => {
                // The band is a fraction of the CPR WIDTH, which is twice the
                // half-width — the base `vocab` expects for a pivot band.
                let width = half.saturating_mul(2);
                match vocab::table::set_near(mask, index, tolerance, close, level, width) {
                    Ok(next) => {
                        mask = next;
                        continue;
                    }
                    Err(_) => continue,
                }
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
        if let Some(index) = set
            && let Ok(next) = vocab::table::set_exact(mask, index)
        {
            mask = next;
        }
    }
    mask
}

/// Every position this module can set, for the test that proves it sets nothing
/// else.
#[must_use]
pub fn positions() -> [u16; 41] {
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
    let mut out = [0u16; 41];
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
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tol() -> Tolerance {
        let Ok(t) = vocab::tolerance::pinned_pivot() else {
            unreachable!("the pivot width is pinned")
        };
        t
    }

    fn levels(h: i64, l: i64, c: i64) -> DailyLevels {
        let Ok(x) = DailyLevels::from_previous_session(h, l, c) else {
            unreachable!("this fixture session is sane")
        };
        x
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
            let Some(def) = vocab::table::definition(index) else {
                unreachable!("position {index} is not in the table")
            };
            assert!(
                vocab::table::is_live(index),
                "position {index} (`{}`) is not live",
                def.name,
            );
            let is_near = def.kind == vocab::Kind::Near;
            assert_eq!(
                is_near,
                wants_near,
                "position {index} (`{}`) is {:?} and the plan treats it as {}",
                def.name,
                def.kind,
                if wants_near { "banded" } else { "bare" },
            );
        }
        assert_eq!(seen.len(), 41);
    }

    /// The three positions this module deliberately leaves alone.
    #[test]
    fn the_undefined_and_retired_positions_are_untouched() {
        let owned = positions();
        for (index, why) in [
            (6_u16, "a tombstone; inside_cpr (62) is set instead"),
            (
                63,
                "narrow_cpr_day has no threshold in any tracked document",
            ),
            (71, "near_fib_424 is an orphan on none of the ladders"),
        ] {
            assert!(
                !owned.contains(&index),
                "position {index} is set, and {why}"
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
}

#[cfg(test)]
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
        let Ok(tol) = vocab::tolerance::pinned_pivot() else {
            unreachable!("the pinned pivot tolerance is valid")
        };
        // A session whose close is well below its high-low midpoint inverts the CPR
        // (bc > tc), because tc = 2*pivot - bc puts tc below bc.
        let Ok(levels) = DailyLevels::from_previous_session(2_500_000, 2_400_000, 2_410_000) else {
            unreachable!("a sane session yields a ladder")
        };
        let (low, high) = levels.cpr_span();
        assert!(
            levels.bc() > levels.tc(),
            "this fixture must be INVERTED or it tests nothing: bc={} tc={}",
            levels.bc(),
            levels.tc()
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

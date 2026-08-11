//! The bit table. 276 positions, and the index **is** the identity.
//!
//! # The rule that outranks every other rule in this file
//!
//! A stored result set is a set of masks, and a mask is a set of bit
//! positions. If bit 22 means `near_fib_50` today and something else after a
//! helpful reorder, every historical result silently means something
//! different and no test can detect it -- the bytes are identical. So:
//! **append only**. Positions are never renumbered, never reused, never
//! reordered (`CLAUDE.md` §3.8, `docs/03-vocabulary.md` §1).
//!
//! # What is here
//!
//! | Positions | Count | Group |
//! |---:|---:|---|
//! | 0–73 | 74 | the shipped vocabulary, transcribed from `docs/03-vocabulary.md` §5 |
//! | 74–85 | 12 | pivot bands, outer relations |
//! | 86–105 | 20 | opening range, four windows |
//! | 106–109 | 4 | Fibonacci extensions on the bullish previous-day anchor |
//! | 110–120 | 11 | Fibonacci over the last five sessions, static |
//! | 121–131 | 11 | Fibonacci over the current session, running |
//! | 132–142 | 11 | Fibonacci over the opening gap leg |
//! | 143–152 | 10 | session-anchored `VWAP` |
//! | 153–177 | 25 | candlestick patterns, prefixed `pat_` |
//! | 178–187 | 10 | the fourth and fifth pivot rungs, sourced at `docs/09-design-sources.md` §1, ladder chosen by D-0078 (the CPR R3 ladder) |
//! | 188–189 | 2 | BC and TC as levels in their own right |
//! | 190–197 | 8 | VWAP bands 2 and 3, completed |
//! | 198–234 | 37 | the rest of the classical candlestick set |
//! | 235–273 | 39 | the current-FORMING-day pivot family, 13 levels x 3 relations |
//!
//! # Tombstones
//!
//! Three shipped positions are retired because each duplicates another shipped
//! position **exactly** -- not approximately, not usually:
//!
//! | Retired | Duplicates | Why they are the same predicate |
//! |---:|---:|---|
//! | 6 `near_pivot_p` | 62 `inside_cpr` | the pivot's zone *is* the `CPR` body: a zone half-width of `\|P - bc\|` makes `[P-w, P+w]` exactly `[bc, tc]` |
//! | 19 `near_fib_0` | 17 `near_pdh` | rung 0 of a `PDH`-anchored ladder *is* the `PDH` |
//! | 25 `near_fib_100` | 18 `near_pdl` | rung 1.0 of that same ladder *is* the `PDL` |
//!
//! A tombstone **keeps its index forever** and always evaluates false.
//! Retiring frees nothing: position 6 is still position 6, and the next
//! condition appends at [`NEXT_FREE`], which is 276 today and only ever grows.
//! That is modelled in the type -- [`BitStatus`] --
//! rather than in a comment, and [`set_exact`] refuses a retired index instead
//! of setting it.
//!
//! # Naming
//!
//! Positions 0–73 are transcribed character for character from
//! `docs/03-vocabulary.md` §5 and are not this crate's to choose. Positions
//! 74–142 follow the group shapes handed down with the design. Positions
//! 143–187 are named here for the first time, in the shipped table's own
//! style, and each row below carries the sentence that says what it means.

use crate::error::VocabError;
use crate::mask::ConditionMask;
use crate::tolerance::Tolerance;

/// Whether a position still evaluates, or is a tombstone.
///
/// The enum exists so that a tombstone cannot be *accidentally* re-used: an
/// append writes a new row at the end of [`TABLE`], and a retired row is not a
/// hole a later append can fall into -- it is an occupied position whose
/// occupant is a corpse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BitStatus {
    /// The evaluator may set this position.
    Live,
    /// The position is retired. It always evaluates false, and it keeps its
    /// index forever.
    Retired {
        /// The live position it duplicated exactly, which is the one to set.
        duplicate_of: u16,
    },
    /// The position is **definitionally constant** and carries no information.
    /// It always evaluates false, and it keeps its index forever.
    ///
    /// Distinct from [`BitStatus::Retired`], which duplicates a live position
    /// and names it. A void position duplicates nothing — its predicate is a
    /// constant on every input, so there is no other bit to set instead. The
    /// forming-day pivot family (235–273) is the whole of this category and
    /// D-0080 records why.
    Void {
        /// Why the predicate is constant. Kept in the type so the reason cannot
        /// drift away from the row it explains.
        reason: &'static str,
    },
}

/// Whether a position needs a measured band to be decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A relation that is true or false on its own -- above, below, inside.
    Plain,
    /// A `near_*` condition. Undecidable without a tolerance, and the
    /// tolerance is [`crate::tolerance::TOL_FIB_MILLI`] or
    /// [`crate::tolerance::TOL_PIVOT_MILLI`] depending on the family.
    Near,
}

/// One row of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BitDef {
    /// The position. Written down rather than inferred from the row's place in
    /// the array, so that a mistyped index is a failing test and not a silent
    /// renumbering.
    pub index: u16,
    /// The condition's name. Unique among live rows.
    pub name: &'static str,
    /// Whether deciding it needs a tolerance.
    pub kind: Kind,
    /// Live, or a tombstone and what it duplicated.
    pub status: BitStatus,
}

/// A live row that decides on its own.
const fn plain(index: u16, name: &'static str) -> BitDef {
    BitDef {
        index,
        name,
        kind: Kind::Plain,
        status: BitStatus::Live,
    }
}

/// A live row that needs the tolerance.
const fn near(index: u16, name: &'static str) -> BitDef {
    BitDef {
        index,
        name,
        kind: Kind::Near,
        status: BitStatus::Live,
    }
}

/// A definitionally-constant position. It keeps `index` and `name`, always
/// evaluates false, and its index is never reissued.
const fn void(index: u16, name: &'static str, kind: Kind, reason: &'static str) -> BitDef {
    BitDef {
        index,
        name,
        kind,
        status: BitStatus::Void { reason },
    }
}

/// A tombstone. It keeps `index` and `name` -- the history is the point -- and
/// names the position that made it redundant.
const fn retired(index: u16, name: &'static str, kind: Kind, duplicate_of: u16) -> BitDef {
    BitDef {
        index,
        name,
        kind,
        status: BitStatus::Retired { duplicate_of },
    }
}

/// The table. Row `i` is position `i`, for every `i`, forever.
pub const TABLE: [BitDef; 276] = [
    // ---- 0–5. Moving averages. Shipped. ---------------------------------
    plain(0, "close_above_ema20"),
    plain(1, "close_below_ema20"),
    plain(2, "close_above_ema200"),
    plain(3, "close_below_ema200"),
    plain(4, "ema20_above_ema200"),
    plain(5, "ema20_below_ema200"),
    // ---- 6–12. Classic pivots. Shipped. ---------------------------------
    // 6 is the first tombstone: the pivot's own zone is the CPR body.
    retired(6, "near_pivot_p", Kind::Near, 62),
    near(7, "near_pivot_r1"),
    near(8, "near_pivot_r2"),
    near(9, "near_pivot_r3"),
    near(10, "near_pivot_s1"),
    near(11, "near_pivot_s2"),
    near(12, "near_pivot_s3"),
    // ---- 13–18. Previous-day high / low. Shipped. -----------------------
    plain(13, "close_above_pdh"),
    plain(14, "close_below_pdh"),
    plain(15, "close_above_pdl"),
    plain(16, "close_below_pdl"),
    near(17, "near_pdh"),
    near(18, "near_pdl"),
    // ---- 19–29. Fibonacci, bearish anchor (PDH). Shipped. ---------------
    // Rung 0 of a PDH-anchored ladder is the PDH, and rung 1.0 is the PDL.
    retired(19, "near_fib_0", Kind::Near, 17),
    near(20, "near_fib_236"),
    near(21, "near_fib_382"),
    near(22, "near_fib_50"),
    near(23, "near_fib_618"),
    near(24, "near_fib_786"),
    retired(25, "near_fib_100", Kind::Near, 18),
    near(26, "near_fib_1272"),
    near(27, "near_fib_1618"),
    near(28, "near_fib_200"),
    near(29, "near_fib_2618"),
    // ---- 30–36. Bar shape. Shipped. -------------------------------------
    plain(30, "bar_bullish"),
    plain(31, "bar_bearish"),
    plain(32, "bar_doji"),
    plain(33, "bar_large_body"),
    plain(34, "bar_small_body"),
    plain(35, "long_upper_wick"),
    plain(36, "long_lower_wick"),
    // ---- 37–39. Prior-bar sequence. Shipped. ----------------------------
    plain(37, "prior_n_bullish"),
    plain(38, "prior_n_bearish"),
    plain(39, "prior_alternating"),
    // ---- 40–43. Position within the day. Shipped. -----------------------
    plain(40, "close_at_day_high"),
    plain(41, "close_at_day_low"),
    plain(42, "close_in_upper_third"),
    plain(43, "close_in_lower_third"),
    // ---- 44–47. Time of day. Shipped. -----------------------------------
    plain(44, "early_morning"),
    plain(45, "mid_morning"),
    plain(46, "midday"),
    plain(47, "afternoon"),
    // ---- 48–51. Day type. Shipped. --------------------------------------
    plain(48, "gap_up_day"),
    plain(49, "gap_down_day"),
    plain(50, "inside_day"),
    plain(51, "outside_day"),
    // ---- 52–53. VWAP. Shipped. ------------------------------------------
    plain(52, "close_above_vwap"),
    plain(53, "close_below_vwap"),
    // ---- 54–55. Extended pivots. Shipped. -------------------------------
    near(54, "near_pivot_r5"),
    near(55, "near_pivot_s5"),
    // ---- 56–59. Market structure. Shipped. ------------------------------
    plain(56, "bos_bullish"),
    plain(57, "bos_bearish"),
    plain(58, "choch_bullish"),
    plain(59, "choch_bearish"),
    // ---- 60–63. Central pivot range. Shipped. ---------------------------
    plain(60, "above_cpr_tc"),
    plain(61, "below_cpr_bc"),
    plain(62, "inside_cpr"),
    plain(63, "narrow_cpr_day"),
    // ---- 64–65. SuperTrend. Shipped. ------------------------------------
    plain(64, "close_above_supertrend"),
    plain(65, "close_below_supertrend"),
    // ---- 66–68. Gap midpoint. Shipped. ----------------------------------
    plain(66, "close_above_gap_mid"),
    plain(67, "close_below_gap_mid"),
    near(68, "near_gap_mid"),
    // ---- 69–70. Fibonacci, bullish anchor (PDL). Shipped. ---------------
    near(69, "near_fib_bull_236"),
    near(70, "near_fib_bull_786"),
    // ---- 71. Fibonacci extension. Shipped. ------------------------------
    near(71, "near_fib_424"),
    // ---- 72–73. Swing levels. Shipped. ----------------------------------
    near(72, "near_swing_high"),
    near(73, "near_swing_low"),
    // =====================================================================
    // Everything below appends at 74. Nothing above it moved.
    // =====================================================================
    // ---- 74–85. Pivot bands, outer relations. ---------------------------
    // The shipped pivots ask whether price is AT a level. These ask which
    // SIDE of the level's band it closed on, which is a different question
    // and is why they are new positions rather than a redefinition. Ordered
    // level-major with the above/below pair adjacent, which is the shipped
    // table's own ordering for every pair it carries (0/1, 13/14, 15/16).
    plain(74, "close_above_pivot_r1_band"),
    plain(75, "close_below_pivot_r1_band"),
    plain(76, "close_above_pivot_r2_band"),
    plain(77, "close_below_pivot_r2_band"),
    plain(78, "close_above_pivot_r3_band"),
    plain(79, "close_below_pivot_r3_band"),
    plain(80, "close_above_pivot_s1_band"),
    plain(81, "close_below_pivot_s1_band"),
    plain(82, "close_above_pivot_s2_band"),
    plain(83, "close_below_pivot_s2_band"),
    plain(84, "close_above_pivot_s3_band"),
    plain(85, "close_below_pivot_s3_band"),
    // ---- 86–105. Opening range, four windows. ---------------------------
    // Window-major: all five relations for the 5-minute range, then 15, then
    // 30, then 60. A window's five bits stay contiguous so that a later
    // window appends as a block rather than interleaving into these.
    plain(86, "orb5_close_above_high"),
    plain(87, "orb5_close_below_low"),
    plain(88, "orb5_close_inside"),
    near(89, "orb5_near_high"),
    near(90, "orb5_near_low"),
    plain(91, "orb15_close_above_high"),
    plain(92, "orb15_close_below_low"),
    plain(93, "orb15_close_inside"),
    near(94, "orb15_near_high"),
    near(95, "orb15_near_low"),
    plain(96, "orb30_close_above_high"),
    plain(97, "orb30_close_below_low"),
    plain(98, "orb30_close_inside"),
    near(99, "orb30_near_high"),
    near(100, "orb30_near_low"),
    plain(101, "orb60_close_above_high"),
    plain(102, "orb60_close_below_low"),
    plain(103, "orb60_close_inside"),
    near(104, "orb60_near_high"),
    near(105, "orb60_near_low"),
    // ---- 106–109. Fibonacci, bullish anchor, extensions only. -----------
    // The retracement rungs of this ladder shipped at 69 and 70. These are
    // the four extensions beyond 1.0, and no rung already shipped repeats.
    near(106, "near_fib_bull_1272"),
    near(107, "near_fib_bull_1618"),
    near(108, "near_fib_bull_200"),
    near(109, "near_fib_bull_2618"),
    // ---- 110–120. Fibonacci over the last five sessions, static. --------
    // Anchored to the high and low of the previous five sessions and fixed
    // for the whole of today, so every bar of the day sees the same ladder.
    near(110, "near_fib_prev5_0"),
    near(111, "near_fib_prev5_236"),
    near(112, "near_fib_prev5_382"),
    near(113, "near_fib_prev5_50"),
    near(114, "near_fib_prev5_618"),
    near(115, "near_fib_prev5_786"),
    near(116, "near_fib_prev5_100"),
    near(117, "near_fib_prev5_1272"),
    near(118, "near_fib_prev5_1618"),
    near(119, "near_fib_prev5_200"),
    near(120, "near_fib_prev5_2618"),
    // ---- 121–131. Fibonacci over the current session, running. ----------
    // Anchored to the session high and low SO FAR, so the ladder moves as the
    // day extends. At bar N it is computed from bars 0..=N of today and no
    // later bar -- `docs/03-vocabulary.md` §3, and the reason this group is
    // separate from the static one above rather than a mode of it.
    near(121, "near_fib_curday_0"),
    near(122, "near_fib_curday_236"),
    near(123, "near_fib_curday_382"),
    near(124, "near_fib_curday_50"),
    near(125, "near_fib_curday_618"),
    near(126, "near_fib_curday_786"),
    near(127, "near_fib_curday_100"),
    near(128, "near_fib_curday_1272"),
    near(129, "near_fib_curday_1618"),
    near(130, "near_fib_curday_200"),
    near(131, "near_fib_curday_2618"),
    // ---- 132–142. Fibonacci over the opening gap leg. -------------------
    // Anchored to the two ends of the overnight gap: yesterday's close and
    // today's open. On a day with no gap the leg has zero length and every
    // rung collapses onto one price; that is a degenerate ladder, and the
    // evaluator's job is to abstain rather than to set eleven bits at once.
    near(132, "near_fib_gap_0"),
    near(133, "near_fib_gap_236"),
    near(134, "near_fib_gap_382"),
    near(135, "near_fib_gap_50"),
    near(136, "near_fib_gap_618"),
    near(137, "near_fib_gap_786"),
    near(138, "near_fib_gap_100"),
    near(139, "near_fib_gap_1272"),
    near(140, "near_fib_gap_1618"),
    near(141, "near_fib_gap_200"),
    near(142, "near_fib_gap_2618"),
    // ---- 143–152. Session-anchored VWAP. --------------------------------
    // NAMED HERE FOR THE FIRST TIME. The shipped bits 52 and 53 are a bare
    // above/below on a VWAP whose anchor the vocabulary never stated. These
    // ten are explicit about it: the anchor is the session open, the sum
    // restarts every session, and the bands are standard deviations of price
    // about that running mean.
    //
    // `docs/03-vocabulary.md` §4 applies to every one of them: VWAP needs
    // traded volume, spot indices carry none, so on `NSE-NIFTY` and
    // `NSE-BANKNIFTY` these ten permanently abstain. They are defined anyway
    // because the store holds futures and options, which do carry volume, and
    // a position defined later would be a position numbered later.
    //
    // 143 close is above the session-anchored VWAP
    plain(143, "close_above_vwap_session"),
    // 144 close is below it
    plain(144, "close_below_vwap_session"),
    // 145 close is within tolerance of it
    near(145, "near_vwap_session"),
    // 146 close is above the first upper band (one deviation)
    plain(146, "close_above_vwap_band1_upper"),
    // 147 close is below the first lower band (one deviation)
    plain(147, "close_below_vwap_band1_lower"),
    // 148 close is above the second upper band (two deviations)
    plain(148, "close_above_vwap_band2_upper"),
    // 149 close is below the second lower band (two deviations)
    plain(149, "close_below_vwap_band2_lower"),
    // 150 close is within tolerance of the first upper band
    near(150, "near_vwap_band1_upper"),
    // 151 close is within tolerance of the first lower band
    near(151, "near_vwap_band1_lower"),
    // 152 close is between the two first-deviation bands
    plain(152, "inside_vwap_band1"),
    // ---- 153–177. Candlestick patterns. ---------------------------------
    // NAMED HERE FOR THE FIRST TIME, all prefixed `pat_` so a mask that
    // survives ranking can be read as "shape bits plus pattern bits" at a
    // glance. Each is a shape over the current bar and, where the name says
    // so, the one or two before it -- never a later one.
    //
    // None of these duplicates a shipped bar-shape bit (30–36). Those are
    // properties of ONE bar in isolation; a hammer additionally requires the
    // body's position within the range, and every multi-bar name below is a
    // sequence no single-bar bit can express.
    //
    // 153 small body at the top of the range, long lower wick
    plain(153, "pat_hammer"),
    // 154 small body at the bottom of the range, long upper wick
    plain(154, "pat_inverted_hammer"),
    // 155 hammer shape arriving after an advance
    plain(155, "pat_hanging_man"),
    // 156 inverted-hammer shape arriving after an advance
    plain(156, "pat_shooting_star"),
    // 157 up bar whose body wholly contains the previous down body
    plain(157, "pat_bullish_engulfing"),
    // 158 down bar whose body wholly contains the previous up body
    plain(158, "pat_bearish_engulfing"),
    // 159 small up body wholly inside the previous larger down body
    plain(159, "pat_bullish_harami"),
    // 160 small down body wholly inside the previous larger up body
    plain(160, "pat_bearish_harami"),
    // 161 up bar closing above the midpoint of the previous down body
    plain(161, "pat_piercing_line"),
    // 162 down bar closing below the midpoint of the previous up body
    plain(162, "pat_dark_cloud_cover"),
    // 163 down bar, small-bodied bar, up bar closing into the first body
    plain(163, "pat_morning_star"),
    // 164 up bar, small-bodied bar, down bar closing into the first body
    plain(164, "pat_evening_star"),
    // 165 three up bars, each closing above the last
    plain(165, "pat_three_white_soldiers"),
    // 166 three down bars, each closing below the last
    plain(166, "pat_three_black_crows"),
    // 167 bullish harami confirmed by a third bar closing above it
    plain(167, "pat_three_inside_up"),
    // 168 bearish harami confirmed by a third bar closing below it
    plain(168, "pat_three_inside_down"),
    // 169 two bars sharing a high after an advance
    plain(169, "pat_tweezer_top"),
    // 170 two bars sharing a low after a decline
    plain(170, "pat_tweezer_bottom"),
    // 171 small body centred in the range, wicks on both sides
    plain(171, "pat_spinning_top"),
    // 172 up bar whose body is the whole range: no wick at either end
    plain(172, "pat_marubozu_bullish"),
    // 173 down bar whose body is the whole range
    plain(173, "pat_marubozu_bearish"),
    // 174 open, high and close together at the top: only a lower wick
    plain(174, "pat_dragonfly_doji"),
    // 175 open, low and close together at the bottom: only an upper wick
    plain(175, "pat_gravestone_doji"),
    // 176 up bar, a run of small bars inside its range, then an up bar above it
    plain(176, "pat_rising_three_methods"),
    // 177 down bar, a run of small bars inside its range, then a down bar below
    plain(177, "pat_falling_three_methods"),
    // ---- 178–187. The fourth and fifth pivot rungs, completed. ----------
    //
    // WHY THESE ARRIVED LATE, recorded because the gap was real. The first
    // pass REFUSED R4, R5, S4 and S5 under golden rule 1 — no tracked
    // document in this repository stated their formula, and inventing one is
    // exactly what that rule forbids. The refusal was correct about the REPO
    // and wrong about the world: the operator's own CPR indicator carries the
    // recurrence, and D-0078 (the CPR R3 ladder) now records it in
    // `docs/00-charter.md` where a
    // formula belongs.
    //
    //   r4 = r3 + r2 - r1  =  P + 2(H - L)
    //   r5 = r4 + r3 - r2  =  2P + 2H - 3L
    //   s4 = s3 + s2 - s1  =  P - 2(H - L)
    //   s5 = s4 + s3 - s2  =  2P - 3H + 2L
    //
    // The shipped table already named `near_pivot_r5` (54) and
    // `near_pivot_s5` (55) with no formula behind them, so those two bits were
    // unimplementable rather than merely unused. D-0078 (the CPR R3 ladder)
    // closes that too.
    //
    // SOURCE: docs/09-design-sources.md §1. NOT docs/00-charter.md — the charter
    // carries no pivot formula, and an earlier version of this comment claimed it
    // did. That claim was false for as long as it stood.
    //
    // R4 and S4 get all three relations. R5 and S5 get only the two band
    // sides, because their `near_` positions are 54 and 55 and an index is
    // never reissued.
    near(178, "near_pivot_r4"),
    near(179, "near_pivot_s4"),
    plain(180, "close_above_pivot_r4_band"),
    plain(181, "close_below_pivot_r4_band"),
    plain(182, "close_above_pivot_s4_band"),
    plain(183, "close_below_pivot_s4_band"),
    plain(184, "close_above_pivot_r5_band"),
    plain(185, "close_below_pivot_r5_band"),
    plain(186, "close_above_pivot_s5_band"),
    plain(187, "close_below_pivot_s5_band"),
    // ---- 188–189. BC and TC as levels in their own right. ----------------
    //
    // The shipped table asks which SIDE of the CPR body price closed on (60,
    // 61) and whether it is inside (62). It never asks whether price is AT
    // either edge. The design source plots both as tracked-price lines, so both
    // are levels a trader watches, not merely the boundary of a fill.
    near(188, "near_cpr_tc"),
    near(189, "near_cpr_bc"),
    // ---- 190–197. VWAP bands 2 and 3, completed. -------------------------
    //
    // `docs/09-design-sources.md` §2 ships THREE band multipliers — 1.0, 2.0,
    // 3.0. Positions 143–152 gave band 1 all five relations and band 2 only its
    // two outer sides; band 3 had nothing at all. A band the design draws and
    // the vocabulary cannot name is a level the sweep can never test.
    near(190, "near_vwap_band2_upper"),
    near(191, "near_vwap_band2_lower"),
    plain(192, "inside_vwap_band2"),
    plain(193, "close_above_vwap_band3_upper"),
    plain(194, "close_below_vwap_band3_lower"),
    near(195, "near_vwap_band3_upper"),
    near(196, "near_vwap_band3_lower"),
    plain(197, "inside_vwap_band3"),
    // ---- 198–234. The rest of the classical candlestick set. -------------
    //
    // Positions 153–177 carried twenty-five patterns. These are the thirty-seven
    // the classical set names and that block omitted, so the vocabulary now
    // covers the whole vernacular rather than a curated subset.
    //
    // WORTH KNOWING, and not an argument against allocating them: several are
    // compositions the mask could already express. A hanging man is a hammer
    // plus a prior uptrend; a homing pigeon is a harami whose bodies share a
    // colour. Allocating them anyway costs one position each and buys the sweep
    // a name a trader recognises, which is what the operator asked for. The
    // alternative — primitives plus composition — is cheaper per bit and is NOT
    // what was chosen here. Recorded so the choice is visible.
    //
    // Every one still needs a predicate. `crates/indicators` does not exist, so
    // NONE of the 62 pattern positions can be evaluated yet.
    plain(198, "pat_abandoned_baby_bull"),
    plain(199, "pat_abandoned_baby_bear"),
    plain(200, "pat_three_line_strike_bull"),
    plain(201, "pat_three_line_strike_bear"),
    plain(202, "pat_kicker_bull"),
    plain(203, "pat_kicker_bear"),
    plain(204, "pat_belt_hold_bull"),
    plain(205, "pat_belt_hold_bear"),
    plain(206, "pat_counterattack_bull"),
    plain(207, "pat_counterattack_bear"),
    plain(208, "pat_separating_lines_bull"),
    plain(209, "pat_separating_lines_bear"),
    plain(210, "pat_on_neck"),
    plain(211, "pat_in_neck"),
    plain(212, "pat_thrusting"),
    plain(213, "pat_tasuki_gap_up"),
    plain(214, "pat_tasuki_gap_down"),
    plain(215, "pat_side_by_side_white"),
    plain(216, "pat_mat_hold"),
    plain(217, "pat_stick_sandwich"),
    plain(218, "pat_ladder_bottom"),
    plain(219, "pat_ladder_top"),
    plain(220, "pat_concealing_baby_swallow"),
    plain(221, "pat_unique_three_river"),
    plain(222, "pat_breakaway_bull"),
    plain(223, "pat_breakaway_bear"),
    plain(224, "pat_long_legged_doji"),
    plain(225, "pat_four_price_doji"),
    plain(226, "pat_rickshaw_man"),
    plain(227, "pat_high_wave"),
    plain(228, "pat_homing_pigeon"),
    plain(229, "pat_matching_low"),
    plain(230, "pat_identical_three_crows"),
    plain(231, "pat_advance_block"),
    plain(232, "pat_deliberation"),
    plain(233, "pat_tri_star_bull"),
    plain(234, "pat_tri_star_bear"),
    // ---- 235–273. The current-FORMING-day pivot family. ------------------
    //
    // `docs/09-design-sources.md` §1 records that the CPR indicator has a
    // `use_prev_day` input, and that setting it FALSE computes every level from
    // today's still-forming high, low and close. The script's own comment calls
    // that behaviour REPAINTING, and on a chart it is: the drawn line moves as
    // the day develops.
    //
    // It is nonetheless admissible here, and the distinction is worth stating
    // because it is the same one the current-day Fibonacci family rests on.
    // Reading today's running extremes at bar N uses bars s..=N and nothing
    // later, so it satisfies §3 rule 7. "Repaints" describes the drawing, not
    // the causality. What is NOT admissible is the FINAL day high — that reads
    // the future — and no position here does.
    //
    // Thirteen levels x three relations. The prev-day family's own relation set,
    // applied to the forming day, so a mask can ask for both and the sweep can
    // discover which anchor actually predicts.
    void(
        235,
        "near_forming_pivot_pivot",
        Kind::Near,
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        236,
        "close_above_forming_pivot_pivot_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        237,
        "close_below_forming_pivot_pivot_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        238,
        "near_forming_pivot_cpr_bc",
        Kind::Near,
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        239,
        "close_above_forming_pivot_cpr_bc_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        240,
        "close_below_forming_pivot_cpr_bc_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        241,
        "near_forming_pivot_cpr_tc",
        Kind::Near,
        "constant true except on a single-price day: |C - tc| equals the band exactly",
    ),
    void(
        242,
        "close_above_forming_pivot_cpr_tc_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        243,
        "close_below_forming_pivot_cpr_tc_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        244,
        "near_forming_pivot_r1",
        Kind::Near,
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        245,
        "close_above_forming_pivot_r1_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        246,
        "close_below_forming_pivot_r1_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        247,
        "near_forming_pivot_r2",
        Kind::Near,
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        248,
        "close_above_forming_pivot_r2_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        249,
        "close_below_forming_pivot_r2_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        250,
        "near_forming_pivot_r3",
        Kind::Near,
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        251,
        "close_above_forming_pivot_r3_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        252,
        "close_below_forming_pivot_r3_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        253,
        "near_forming_pivot_r4",
        Kind::Near,
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        254,
        "close_above_forming_pivot_r4_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        255,
        "close_below_forming_pivot_r4_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        256,
        "near_forming_pivot_r5",
        Kind::Near,
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        257,
        "close_above_forming_pivot_r5_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        258,
        "close_below_forming_pivot_r5_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        259,
        "near_forming_pivot_s1",
        Kind::Near,
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        260,
        "close_above_forming_pivot_s1_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        261,
        "close_below_forming_pivot_s1_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        262,
        "near_forming_pivot_s2",
        Kind::Near,
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        263,
        "close_above_forming_pivot_s2_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        264,
        "close_below_forming_pivot_s2_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        265,
        "near_forming_pivot_s3",
        Kind::Near,
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        266,
        "close_above_forming_pivot_s3_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        267,
        "close_below_forming_pivot_s3_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        268,
        "near_forming_pivot_s4",
        Kind::Near,
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        269,
        "close_above_forming_pivot_s4_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        270,
        "close_below_forming_pivot_s4_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        271,
        "near_forming_pivot_s5",
        Kind::Near,
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        272,
        "close_above_forming_pivot_s5_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        273,
        "close_below_forming_pivot_s5_band",
        Kind::Plain,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    // ---- 274–275. The CPR's width, appended. ------------------------------
    // Position 63 `narrow_cpr_day` shipped alone, which asks only half a
    // question: a reader learns that a day was not narrow and nothing about
    // whether it was wide or ordinary. Three states need three bits, because the
    // hit test is `(bits & mask) == mask` with no negation — "not narrow" is not
    // expressible by leaving 63 clear.
    //
    // Appended at NEXT_FREE rather than squeezed beside 63, which is what §3
    // rule 8 requires: 63 keeps its index and its meaning, every mask recorded
    // before today still means what it meant, and the two new bits are simply
    // available. This is the first use of the append mechanism since the table
    // was written, and it cost two rows and one edit to `LIVE`.
    plain(274, "wide_cpr_day"),
    plain(275, "neutral_cpr_day"),
];

/// How many positions the table defines. Not how many bits the mask holds --
/// [`ConditionMask::BITS`] is 256, and the 78 positions between are unallocated
/// headroom, not free-for-all space.
pub const COUNT: usize = TABLE.len();

/// The highest position that will ever be a hole: none. The next condition
/// appends here, whatever has been retired below it.
pub const NEXT_FREE: u16 = 276;

// THE TABLE CANNOT OUTGROW THE MASK, enforced at COMPILE time.
//
// `COUNT` only ever rises and `ConditionMask` holds exactly `BITS` positions.
// Nothing tied the two, so on the day the table passed the mask width
// `with_bit` would have silently ignored the position and `set_exact` would have
// returned `Ok` with the bit NOT set — a success report for work not done, which
// is the fallback `CLAUDE.md` §4 bans.
//
// A `const` assertion and not a `#[test]`: appending past the width should fail
// the BUILD, in the same commit that appends.
const _: () = assert!(
    COUNT <= ConditionMask::BITS as usize,
    "the bit table has more positions than the mask has bits; widen \
     ConditionMask::WORDS in the same change"
);

/// Every position that is live, as a mask.
///
/// Written as a literal and proved against [`TABLE`] by
/// `vocab::table::the_live_mask_is_the_table`, rather than folded out of the
/// table at run time: intersecting with it is how a tombstone is guaranteed
/// false ([`only_live`]), and that guarantee should not itself be a loop.
///
/// Words 0–2 are full because positions 0–191 are all live-or-tombstoned; word 3
/// carries bits 192–234, which is its low 43. **Words 4 and 5 are empty even
/// though positions 235–273 exist**, because every one of them is
/// [`BitStatus::Void`] — see D-0080. Then the three tombstones are cleared.
pub const LIVE: ConditionMask =
    ConditionMask::from_words([u64::MAX, u64::MAX, u64::MAX, (1u64 << 43) - 1, 0, 0])
        .without_bit(6)
        .without_bit(19)
        .without_bit(25)
        // 274 and 275 sit in word 4, which the comment above says is empty because
        // 235–273 are all void. They are the first live positions above 234, so the
        // sentence is now "word 4 carries exactly these two".
        .with_bit(274)
        .with_bit(275);

/// The row at `index`, or `None` when the index is past the table.
#[must_use]
pub fn definition(index: u16) -> Option<&'static BitDef> {
    TABLE.get(usize::from(index))
}

/// The name at `index`, tombstone or not. A retired name is history and is
/// still reported, because a stored mask from before the retirement still
/// carries the position.
#[must_use]
pub fn name(index: u16) -> Option<&'static str> {
    definition(index).map(|d| d.name)
}

/// Is `index` a live position? A tombstone and an index past the table are
/// both `false`, and they are different refusals from [`set_exact`].
#[must_use]
pub fn is_live(index: u16) -> bool {
    matches!(
        definition(index),
        Some(BitDef {
            status: BitStatus::Live,
            ..
        })
    )
}

/// Force every tombstone and every unallocated position to false.
///
/// The contract "a retired bit always evaluates false" is enforced here, by
/// intersection with [`LIVE`], and not by trusting an evaluator to skip it.
#[must_use]
pub fn only_live(mask: ConditionMask) -> ConditionMask {
    mask.intersect(&LIVE)
}

/// Set a position that decides on its own.
///
/// # Errors
///
/// [`VocabError::NoSuchBit`] past the end of the table;
/// [`VocabError::Retired`] for a tombstone, naming the position to set
/// instead; [`VocabError::NeedsTolerance`] for a `near_*` position, which can
/// only be set through [`set_near`].
pub fn set_exact(mask: ConditionMask, index: u16) -> Result<ConditionMask, VocabError> {
    let def = definition(index).ok_or(VocabError::NoSuchBit { index })?;
    if let BitStatus::Retired { duplicate_of } = def.status {
        return Err(VocabError::Retired {
            index,
            duplicate_of,
        });
    }
    if let BitStatus::Void { reason } = def.status {
        return Err(VocabError::Void { index, reason });
    }
    if def.kind == Kind::Near {
        return Err(VocabError::NeedsTolerance { index });
    }
    Ok(mask.with_bit(u32::from(index)))
}

/// Set a `near_*` position, if `value_paisa` is inside the band around
/// `level_paisa`.
///
/// `range_paisa` is the span of the anchor `level_paisa` was derived from, and
/// it is the base the band is a fraction of -- see [`Tolerance::covers`]. A
/// non-positive range sets nothing.
///
/// The [`Tolerance`] argument is the gate: it cannot be constructed while
/// a [`Tolerance`] cannot be built from the unpinned sentinel, so no `near_*`
/// bit can be set from an invented number.
///
/// # Errors
///
/// [`VocabError::NoSuchBit`] past the end of the table;
/// [`VocabError::Retired`] for a tombstone; [`VocabError::NotNear`] for a
/// position that decides on its own, which takes [`set_exact`].
pub fn set_near(
    mask: ConditionMask,
    index: u16,
    tolerance: Tolerance,
    value_paisa: i64,
    level_paisa: i64,
    range_paisa: i64,
) -> Result<ConditionMask, VocabError> {
    let def = definition(index).ok_or(VocabError::NoSuchBit { index })?;
    if let BitStatus::Retired { duplicate_of } = def.status {
        return Err(VocabError::Retired {
            index,
            duplicate_of,
        });
    }
    if let BitStatus::Void { reason } = def.status {
        return Err(VocabError::Void { index, reason });
    }
    if def.kind == Kind::Plain {
        return Err(VocabError::NotNear { index });
    }
    if tolerance.covers(value_paisa, level_paisa, range_paisa) {
        return Ok(mask.with_bit(u32::from(index)));
    }
    Ok(mask)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. `expect` and not `let ... else \
              { unreachable!() }` on purpose -- an `unreachable!` expands to a \
              panic inside THIS crate, which is a region no passing test can \
              ever execute, while `expect` panics inside the standard library \
              and leaves nothing dead behind."
)]
mod tests {
    use super::*;

    /// A tolerance for the tests, pinned locally. It is deliberately NOT
    /// The pinned constants: neither is the sentinel, and
    /// must stay that way, and a test that needed it pinned would be pressure
    /// to invent the number.
    /// The width the repository actually ships, not a convenient stand-in --
    /// so these tests exercise the configuration a run will use.
    fn test_tolerance() -> Tolerance {
        crate::tolerance::pinned_fib()
            .expect("the pinned width is neither the sentinel nor negative")
    }

    #[test]
    fn the_live_mask_is_the_table() {
        let folded = TABLE
            .iter()
            .filter(|d| d.status == BitStatus::Live)
            .fold(ConditionMask::ZERO, |m, d| m.with_bit(u32::from(d.index)));
        assert_eq!(LIVE, folded, "the LIVE literal drifted from the table");
        assert_eq!(
            LIVE.popcount(),
            234,
            "276 positions, less three tombstones and less the 39 void forming-pivot rows"
        );
    }

    #[test]
    fn a_tombstone_is_forced_false_however_it_arrived() {
        let dirty = ConditionMask::ZERO
            .with_bit(6)
            .with_bit(19)
            .with_bit(25)
            .with_bit(62)
            // 300 is past NEXT_FREE and inside the mask, so it exercises the
            // "allocated in the mask but not in the table" case. It was 200,
            // which stopped being unallocated the day the forming-pivot block landed.
            .with_bit(300);
        let clean = only_live(dirty);
        assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
        assert!(clean.get(62), "a live bit beside a tombstone survives");
        assert!(!clean.get(300), "unallocated positions are not live either");
    }

    #[test]
    fn set_exact_refuses_a_tombstone_and_names_the_replacement() {
        assert_eq!(
            set_exact(ConditionMask::ZERO, 6),
            Err(VocabError::Retired {
                index: 6,
                duplicate_of: 62
            })
        );
        assert_eq!(
            set_exact(ConditionMask::ZERO, 19),
            Err(VocabError::Retired {
                index: 19,
                duplicate_of: 17
            })
        );
        assert_eq!(
            set_exact(ConditionMask::ZERO, 25),
            Err(VocabError::Retired {
                index: 25,
                duplicate_of: 18
            })
        );
    }

    /// **A void position is refused AS VOID, not as needing a tolerance.**
    ///
    /// [`set_exact`] tests the void arm before it tests [`Kind::Near`], and the
    /// order is the assertion. Reverse them and 238 -- void *and* near --
    /// answers [`VocabError::NeedsTolerance`], which tells the caller to go to
    /// [`set_near`] instead; [`set_near`] refuses it too, so the caller is sent
    /// after a call that cannot exist. A void row has no correct call, and the
    /// refusal has to say so and carry the reason D-0080 recorded.
    #[test]
    fn set_exact_refuses_a_void_position_as_void_and_not_as_needing_a_tolerance() {
        // 238 is `near_forming_pivot_cpr_bc`: void AND near, so it is the row
        // that can be refused for two different reasons and only one is right.
        assert_eq!(
            set_exact(ConditionMask::ZERO, 238),
            Err(VocabError::Void {
                index: 238,
                reason: "constant false: |C - level| is a fixed multiple of the band on every bar",
            }),
            "a void near row is void first; NeedsTolerance would send the caller \
             to a call that refuses it as well"
        );
        // 237 is `close_below_forming_pivot_pivot_band`: void AND plain, so the
        // only other answer available is `Ok` with the bit set.
        assert_eq!(
            set_exact(ConditionMask::ZERO, 237),
            Err(VocabError::Void {
                index: 237,
                reason: "carries only sign(v - u), the running-range axis bits 40-43 already hold",
            }),
            "a void plain row must refuse rather than set a bit that carries nothing"
        );
    }

    #[test]
    fn set_exact_refuses_a_near_bit_and_an_index_past_the_table() {
        assert_eq!(
            set_exact(ConditionMask::ZERO, 7),
            Err(VocabError::NeedsTolerance { index: 7 })
        );
        assert_eq!(
            set_exact(ConditionMask::ZERO, NEXT_FREE),
            Err(VocabError::NoSuchBit { index: NEXT_FREE })
        );
        let m = set_exact(ConditionMask::ZERO, 0).expect("bit 0 is live and plain");
        assert!(m.get(0) && m.popcount() == 1);
    }

    #[test]
    fn set_near_needs_the_band_to_actually_cover() {
        let tol = test_tolerance();
        let level = 2_500_000i64;
        let hit = set_near(ConditionMask::ZERO, 17, tol, level + 200, level, 20_000)
            .expect("17 is live and near");
        assert!(hit.get(17));
        let miss = set_near(ConditionMask::ZERO, 17, tol, level + 200_000, level, 20_000)
            .expect("17 is live and near");
        assert!(miss.is_empty(), "outside the band sets nothing");
    }

    #[test]
    fn set_near_refuses_a_plain_bit_a_tombstone_and_a_hole() {
        let tol = test_tolerance();
        assert_eq!(
            set_near(ConditionMask::ZERO, 0, tol, 1, 1, 1_000),
            Err(VocabError::NotNear { index: 0 })
        );
        assert_eq!(
            set_near(ConditionMask::ZERO, 25, tol, 1, 1, 1_000),
            Err(VocabError::Retired {
                index: 25,
                duplicate_of: 18
            })
        );
        assert_eq!(
            set_near(ConditionMask::ZERO, 1_000, tol, 1, 1, 1_000),
            Err(VocabError::NoSuchBit { index: 1_000 })
        );
    }

    #[test]
    fn lookups_report_a_tombstone_without_hiding_it() {
        assert_eq!(name(6), Some("near_pivot_p"));
        assert!(!is_live(6));
        assert!(is_live(62));
        assert_eq!(name(NEXT_FREE), None);
        assert!(!is_live(NEXT_FREE));
        assert_eq!(definition(NEXT_FREE), None);
        let d = definition(6).expect("position 6 exists; it is retired, which is not absent");
        assert_eq!(d.kind, Kind::Near);
        assert_eq!(d.status, BitStatus::Retired { duplicate_of: 62 });
        assert!(format!("{d:?}").contains("near_pivot_p"));
    }

    #[test]
    fn count_and_next_free_agree_with_the_table() {
        assert_eq!(COUNT, 276);
        assert_eq!(usize::from(NEXT_FREE), COUNT);
    }

    /// **A row constructor carries the `kind` and the `reason` it is handed, and
    /// does not substitute the answer today's rows happen to want.**
    ///
    /// [`TABLE`] is a `const`, so until this test existed the four constructors
    /// were only ever evaluated by the compiler and no test ever called one.
    /// That hid two mistakes that no table-wide check can see:
    ///
    /// * All three tombstones are [`Kind::Near`] today, so a [`retired`] that
    ///   ignored its `kind` argument and wrote `Kind::Near` would pass every
    ///   test in this crate -- until the first plain position is retired, at
    ///   which point a position that decides on its own starts demanding a
    ///   tolerance.
    /// * `crates/vocab/tests/table.rs` asserts only that a void row's reason is
    ///   non-empty, so a [`void`] that ignored its `reason` argument and wrote
    ///   one fixed sentence for all 39 rows would pass that too -- and every
    ///   void row would then explain itself with another row's identity, which
    ///   is the reason travelling in [`VocabError::Void`] to whoever asked.
    ///
    /// The indices are [`NEXT_FREE`] on purpose: this is the constructors' own
    /// contract and not a claim about any shipped row.
    #[test]
    fn the_row_constructors_carry_the_kind_and_the_reason_they_are_handed() {
        let synthetic = NEXT_FREE;

        assert_eq!(
            plain(synthetic, "a_plain_row"),
            BitDef {
                index: synthetic,
                name: "a_plain_row",
                kind: Kind::Plain,
                status: BitStatus::Live,
            },
            "`plain` builds a live row that decides on its own"
        );
        assert_eq!(
            near(synthetic, "a_near_row"),
            BitDef {
                index: synthetic,
                name: "a_near_row",
                kind: Kind::Near,
                status: BitStatus::Live,
            },
            "`near` differs from `plain` in exactly one field, and it is `kind`"
        );
        assert_eq!(
            retired(synthetic, "a_retired_row", Kind::Plain, 62),
            BitDef {
                index: synthetic,
                name: "a_retired_row",
                kind: Kind::Plain,
                status: BitStatus::Retired { duplicate_of: 62 },
            },
            "`retired` must pass `kind` through; no shipped tombstone is plain, \
             so hardcoding `Kind::Near` here would go unnoticed"
        );
        assert_eq!(
            void(synthetic, "a_void_row", Kind::Near, "this exact sentence"),
            BitDef {
                index: synthetic,
                name: "a_void_row",
                kind: Kind::Near,
                status: BitStatus::Void {
                    reason: "this exact sentence"
                },
            },
            "`void` must carry the reason verbatim, not a fixed stand-in"
        );
    }

    /// **A void position is refused even where the band would have covered it.**
    ///
    /// The ORDER of the checks in [`set_near`] is the whole content of this
    /// test. Both calls pass `value_paisa == level_paisa` against a positive
    /// range, which [`Tolerance::covers`] answers true for: delete the
    /// [`BitStatus::Void`] arm and 238 falls straight through to the band test
    /// and the bit is SET. A position D-0080 proved is a constant would then be
    /// reported as a condition that held on this bar.
    ///
    /// `crates/vocab/tests/table.rs` walks 235..=273 through [`set_exact`] only.
    /// Thirteen of those 39 rows are [`Kind::Near`], and [`set_exact`] refuses
    /// them for the wrong reason -- it would answer
    /// [`VocabError::NeedsTolerance`] if the void arm were gone, so it cannot
    /// see this. [`set_near`] is the only entry point that can.
    #[test]
    fn set_near_refuses_a_void_position_even_where_the_band_would_cover() {
        let tol = test_tolerance();
        let level = 2_500_000i64;

        // 238 is `near_forming_pivot_cpr_bc`: void AND near, so `set_near` is
        // the only door it has.
        assert_eq!(
            set_near(ConditionMask::ZERO, 238, tol, level, level, 20_000),
            Err(VocabError::Void {
                index: 238,
                reason: "constant false: |C - level| is a fixed multiple of the band on every bar",
            }),
            "an exact hit on a void level must still refuse, and must say why"
        );

        // 237 is `close_below_forming_pivot_pivot_band`: void AND plain. Void
        // outranks `NotNear`, because `NotNear` advertises `set_exact` and
        // `set_exact` refuses 237 as well -- there is no correct call to name.
        assert_eq!(
            set_near(ConditionMask::ZERO, 237, tol, level, level, 20_000),
            Err(VocabError::Void {
                index: 237,
                reason: "carries only sign(v - u), the running-range axis bits 40-43 already hold",
            }),
            "a void row is refused as void, not redirected to a call that refuses it too"
        );

        // The control: 17 is live, near, and the very same arguments set it. So
        // the two refusals above are the void arm and not a band that missed.
        let live = set_near(ConditionMask::ZERO, 17, tol, level, level, 20_000)
            .expect("17 is live and near");
        assert!(
            live.get(17),
            "these arguments do cover, so the refusals above are the void arm"
        );
    }
}

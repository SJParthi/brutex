//! The bit table. 280 positions, and the index **is** the identity.
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
//! **Rows 19 and 25 declare a different band family from the rows they
//! duplicate**, and the table now says so rather than only this paragraph. They
//! are `near_fib_*` rows on the session range; 17 and 18 are built from the
//! daily levels and measure against the CPR width. "Rung 0 *is* the PDH" is a
//! claim about the LEVEL, and the two are the same centre with different
//! half-widths — so "exactly" above is exact about where, not about how wide.
//! An index is never reissued, so this is recorded and not corrected.
//!
//! # Two band families
//!
//! A `near_*` row is not decidable by a width alone: it needs to know what the
//! width is a fraction of. Every one of the 97 declares it in
//! [`BitDef::band`] — [`Base::CprWidth`] for the pivot ladder, the CPR edges and
//! the previous day's high and low; [`Base::SessionRange`] for everything else.
//! [`set_near`] refuses a tolerance from the other family, which it could not do
//! while the two arrived as indistinguishable numbers. See [`Base`].
//!
//! A tombstone **keeps its index forever** and always evaluates false.
//! Retiring frees nothing: position 6 is still position 6, and the next
//! condition appends at [`NEXT_FREE`], which is 280 today and only ever grows.
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
use crate::tolerance::{Base, Tolerance};

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
///
/// # This is a view of [`BitDef::band`], not an independent field
///
/// A row needs a tolerance exactly when it declares a [`Base`], so `Kind` is
/// computed from `band` by [`kind_of`] inside the four row constructors and is
/// never passed in beside it. The two therefore cannot disagree, which is the
/// only way a `Kind::Near` row with no declared band -- or a `Kind::Plain` row
/// with one -- could have come about.
///
/// **The base is deliberately not a payload on [`Kind::Near`].** That is where
/// it belongs on the merits: `Near { base }` makes an unclassified `near_*` row
/// impossible to write rather than merely checked. It is not there because
/// `Kind::Near` is compared as a bare unit value in eight files outside this
/// crate's `src/` -- seven modules of `crates/indicators` and this crate's own
/// `tests/table.rs` -- and a struct variant is a compile error at every one.
/// Moving the payload onto `Kind` is a mechanical follow-up that has to touch
/// those files in the same commit; until then `band` carries the same
/// information one field over, and [`kind_of`] is what keeps the pair honest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A relation that is true or false on its own -- above, below, inside.
    Plain,
    /// A `near_*` condition. Undecidable without a tolerance, and the
    /// tolerance is [`crate::tolerance::TOL_FIB_MILLI`] or
    /// [`crate::tolerance::TOL_PIVOT_MILLI`] according to the [`Base`] the row
    /// declares in [`BitDef::band`].
    Near,
}

/// The [`Kind`] a row with this band is, which is the only way a `Kind` is
/// produced in this file.
///
/// Declaring a base and needing a tolerance are the same statement, so deriving
/// one from the other removes the channel by which they could drift apart. It
/// was a real channel: before this, `retired` and `void` took a `Kind` argument
/// with no band beside it, so a tombstone could claim `Kind::Near` while naming
/// no family at all and nothing would have noticed.
const fn kind_of(band: Option<Base>) -> Kind {
    match band {
        Some(_) => Kind::Near,
        None => Kind::Plain,
    }
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
    /// Whether deciding it needs a tolerance. Derived from `band` by
    /// [`kind_of`]; see [`Kind`] for why it is not the field that carries the
    /// base.
    pub kind: Kind,
    /// Which quantity this row's band is a fraction of, or `None` when it needs
    /// no band.
    ///
    /// **This is the row's half of the check [`set_near`] performs.** A
    /// `near_pivot_*` position measures against the CPR width and a
    /// `near_fib_*` position against the session range; the two widths are
    /// fifty times apart at today's pins, and until this field existed
    /// [`set_near`] accepted either for either. See [`Base`] for why the
    /// resulting mask is indistinguishable from a correct one.
    pub band: Option<Base>,
    /// Live, or a tombstone and what it duplicated.
    pub status: BitStatus,
}

/// A live row that decides on its own.
const fn plain(index: u16, name: &'static str) -> BitDef {
    BitDef {
        index,
        name,
        kind: kind_of(None),
        band: None,
        status: BitStatus::Live,
    }
}

/// A live row that needs a tolerance, measured against `band`.
///
/// **There is no `near` that does not name a base.** The argument is not
/// defaulted and has no `Option`, so appending a `near_*` row without deciding
/// which family it belongs to is a compile error rather than a row that
/// silently joins whichever family the caller happened to pass a width from.
const fn near(index: u16, name: &'static str, band: Base) -> BitDef {
    BitDef {
        index,
        name,
        kind: kind_of(Some(band)),
        band: Some(band),
        status: BitStatus::Live,
    }
}

/// A definitionally-constant position. It keeps `index` and `name`, always
/// evaluates false, and its index is never reissued.
const fn void(index: u16, name: &'static str, band: Option<Base>, reason: &'static str) -> BitDef {
    BitDef {
        index,
        name,
        kind: kind_of(band),
        band,
        status: BitStatus::Void { reason },
    }
}

/// A tombstone. It keeps `index` and `name` -- the history is the point -- and
/// names the position that made it redundant.
///
/// It keeps its `band` too, for the same reason it keeps its name: a retired row
/// is a record of what the position **was**, and "it was a `near_*` row" is only
/// half of that when there are two families. [`set_near`] never reads it -- the
/// [`BitStatus::Retired`] arm refuses first -- so this is documentation the type
/// carries rather than a value anything decides on.
const fn retired(index: u16, name: &'static str, band: Option<Base>, duplicate_of: u16) -> BitDef {
    BitDef {
        index,
        name,
        kind: kind_of(band),
        band,
        status: BitStatus::Retired { duplicate_of },
    }
}

/// The table. Row `i` is position `i`, for every `i`, forever.
/// The table must stay a `const`, and clippy is right that it is large.
///
/// `large_const_arrays` fires at 365 rows and suggests `static`. Taking that
/// suggestion would silently delete the guard this file most depends on:
/// `COUNT` is `TABLE.len()` **in a const context**, and
/// `const _: () = assert!(COUNT <= ConditionMask::BITS as usize)` is what fails
/// the BUILD on the day an append passes the mask width. A `static`'s `.len()`
/// is not a const expression, so both would have to become runtime checks — and
/// a runtime check for "the table outgrew the mask" fires after `with_bit` has
/// already silently ignored the position.
///
/// The array is copied nowhere: every read is `TABLE.get(index)` behind
/// [`definition`], which is one bounds check and one index.
#[allow(
    clippy::large_const_arrays,
    reason = "COUNT is TABLE.len() in a const context, and the compile-time \
              assertion that the table has not outgrown the mask depends on it."
)]
pub const TABLE: [BitDef; 370] = [
    // ---- 0–5. Moving averages. Shipped. ---------------------------------
    plain(0, "close_above_ema20"),
    plain(1, "close_below_ema20"),
    plain(2, "close_above_ema200"),
    plain(3, "close_below_ema200"),
    plain(4, "ema20_above_ema200"),
    plain(5, "ema20_below_ema200"),
    // ---- 6–12. Classic pivots. Shipped. ---------------------------------
    // 6 is the first tombstone: the pivot's own zone is the CPR body. That
    // identity holds only at `TOL_PIVOT_MILLI` = 500 on `Base::CprWidth`, which
    // is what this row now declares -- see the paragraph on that constant for
    // why a global width of 10 would have retired it on a false premise.
    retired(6, "near_pivot_p", Some(Base::CprWidth), 62),
    near(7, "near_pivot_r1", Base::CprWidth),
    near(8, "near_pivot_r2", Base::CprWidth),
    near(9, "near_pivot_r3", Base::CprWidth),
    near(10, "near_pivot_s1", Base::CprWidth),
    near(11, "near_pivot_s2", Base::CprWidth),
    near(12, "near_pivot_s3", Base::CprWidth),
    // ---- 13–18. Previous-day high / low. Shipped. -----------------------
    plain(13, "close_above_pdh"),
    plain(14, "close_below_pdh"),
    plain(15, "close_above_pdl"),
    plain(16, "close_below_pdl"),
    near(17, "near_pdh", Base::CprWidth),
    near(18, "near_pdl", Base::CprWidth),
    // ---- 19–29. Fibonacci, bearish anchor (PDH). Shipped. ---------------
    // Rung 0 of a PDH-anchored ladder is the PDH, and rung 1.0 is the PDL.
    //
    // THE BAND DECLARED HERE DOES NOT MATCH THE BAND OF THE ROW IT DUPLICATES,
    // and that is recorded rather than reconciled. 19 and 25 are Fibonacci rows
    // and measure against `Base::SessionRange`; 17 and 18 are built by
    // `crates/indicators/src/daily.rs` from the daily levels and measure against
    // `Base::CprWidth`. So the retirement premise -- "rung 0 *is* the PDH" -- is
    // a claim about the LEVEL, and as PREDICATES the two were never exactly
    // identical: same centre, different half-width. Position 19 is nonetheless
    // still position 19 forever (§3.8), so there is nothing to undo. Declaring
    // these two as anything other than what they were would rewrite history to
    // make the table look consistent, which is the opposite of the point.
    //
    // Nothing reads these bands: `set_near`'s `BitStatus::Retired` arm refuses
    // before the band is consulted. They are here so the discrepancy is visible
    // in the type instead of only in this comment.
    retired(19, "near_fib_0", Some(Base::SessionRange), 17),
    near(20, "near_fib_236", Base::SessionRange),
    near(21, "near_fib_382", Base::SessionRange),
    near(22, "near_fib_50", Base::SessionRange),
    near(23, "near_fib_618", Base::SessionRange),
    near(24, "near_fib_786", Base::SessionRange),
    retired(25, "near_fib_100", Some(Base::SessionRange), 18),
    near(26, "near_fib_1272", Base::SessionRange),
    near(27, "near_fib_1618", Base::SessionRange),
    near(28, "near_fib_200", Base::SessionRange),
    near(29, "near_fib_2618", Base::SessionRange),
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
    near(54, "near_pivot_r5", Base::CprWidth),
    near(55, "near_pivot_s5", Base::CprWidth),
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
    near(68, "near_gap_mid", Base::SessionRange),
    // ---- 69–70. Fibonacci, bullish anchor (PDL). Shipped. ---------------
    near(69, "near_fib_bull_236", Base::SessionRange),
    near(70, "near_fib_bull_786", Base::SessionRange),
    // ---- 71. Fibonacci extension. Shipped. ------------------------------
    near(71, "near_fib_424", Base::SessionRange),
    // ---- 72–73. Swing levels. Shipped. ----------------------------------
    near(72, "near_swing_high", Base::SessionRange),
    near(73, "near_swing_low", Base::SessionRange),
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
    near(89, "orb5_near_high", Base::SessionRange),
    near(90, "orb5_near_low", Base::SessionRange),
    plain(91, "orb15_close_above_high"),
    plain(92, "orb15_close_below_low"),
    plain(93, "orb15_close_inside"),
    near(94, "orb15_near_high", Base::SessionRange),
    near(95, "orb15_near_low", Base::SessionRange),
    plain(96, "orb30_close_above_high"),
    plain(97, "orb30_close_below_low"),
    plain(98, "orb30_close_inside"),
    near(99, "orb30_near_high", Base::SessionRange),
    near(100, "orb30_near_low", Base::SessionRange),
    plain(101, "orb60_close_above_high"),
    plain(102, "orb60_close_below_low"),
    plain(103, "orb60_close_inside"),
    near(104, "orb60_near_high", Base::SessionRange),
    near(105, "orb60_near_low", Base::SessionRange),
    // ---- 106–109. Fibonacci, bullish anchor, extensions only. -----------
    // The retracement rungs of this ladder shipped at 69 and 70. These are
    // the four extensions beyond 1.0, and no rung already shipped repeats.
    near(106, "near_fib_bull_1272", Base::SessionRange),
    near(107, "near_fib_bull_1618", Base::SessionRange),
    near(108, "near_fib_bull_200", Base::SessionRange),
    near(109, "near_fib_bull_2618", Base::SessionRange),
    // ---- 110–120. Fibonacci over the last five sessions, static. --------
    // Anchored to the high and low of the previous five sessions and fixed
    // for the whole of today, so every bar of the day sees the same ladder.
    near(110, "near_fib_prev5_0", Base::SessionRange),
    near(111, "near_fib_prev5_236", Base::SessionRange),
    near(112, "near_fib_prev5_382", Base::SessionRange),
    near(113, "near_fib_prev5_50", Base::SessionRange),
    near(114, "near_fib_prev5_618", Base::SessionRange),
    near(115, "near_fib_prev5_786", Base::SessionRange),
    near(116, "near_fib_prev5_100", Base::SessionRange),
    near(117, "near_fib_prev5_1272", Base::SessionRange),
    near(118, "near_fib_prev5_1618", Base::SessionRange),
    near(119, "near_fib_prev5_200", Base::SessionRange),
    near(120, "near_fib_prev5_2618", Base::SessionRange),
    // ---- 121–131. Fibonacci over the current session, running. ----------
    // Anchored to the session high and low SO FAR, so the ladder moves as the
    // day extends. At bar N it is computed from bars 0..=N of today and no
    // later bar -- `docs/03-vocabulary.md` §3, and the reason this group is
    // separate from the static one above rather than a mode of it.
    near(121, "near_fib_curday_0", Base::SessionRange),
    near(122, "near_fib_curday_236", Base::SessionRange),
    near(123, "near_fib_curday_382", Base::SessionRange),
    near(124, "near_fib_curday_50", Base::SessionRange),
    near(125, "near_fib_curday_618", Base::SessionRange),
    near(126, "near_fib_curday_786", Base::SessionRange),
    near(127, "near_fib_curday_100", Base::SessionRange),
    near(128, "near_fib_curday_1272", Base::SessionRange),
    near(129, "near_fib_curday_1618", Base::SessionRange),
    near(130, "near_fib_curday_200", Base::SessionRange),
    near(131, "near_fib_curday_2618", Base::SessionRange),
    // ---- 132–142. Fibonacci over the opening gap leg. -------------------
    // Anchored to the two ends of the overnight gap: yesterday's close and
    // today's open. On a day with no gap the leg has zero length and every
    // rung collapses onto one price; that is a degenerate ladder, and the
    // evaluator's job is to abstain rather than to set eleven bits at once.
    near(132, "near_fib_gap_0", Base::SessionRange),
    near(133, "near_fib_gap_236", Base::SessionRange),
    near(134, "near_fib_gap_382", Base::SessionRange),
    near(135, "near_fib_gap_50", Base::SessionRange),
    near(136, "near_fib_gap_618", Base::SessionRange),
    near(137, "near_fib_gap_786", Base::SessionRange),
    near(138, "near_fib_gap_100", Base::SessionRange),
    near(139, "near_fib_gap_1272", Base::SessionRange),
    near(140, "near_fib_gap_1618", Base::SessionRange),
    near(141, "near_fib_gap_200", Base::SessionRange),
    near(142, "near_fib_gap_2618", Base::SessionRange),
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
    near(145, "near_vwap_session", Base::SessionRange),
    // 146 close is above the first upper band (one deviation)
    plain(146, "close_above_vwap_band1_upper"),
    // 147 close is below the first lower band (one deviation)
    plain(147, "close_below_vwap_band1_lower"),
    // 148 close is above the second upper band (two deviations)
    plain(148, "close_above_vwap_band2_upper"),
    // 149 close is below the second lower band (two deviations)
    plain(149, "close_below_vwap_band2_lower"),
    // 150 close is within tolerance of the first upper band
    near(150, "near_vwap_band1_upper", Base::SessionRange),
    // 151 close is within tolerance of the first lower band
    near(151, "near_vwap_band1_lower", Base::SessionRange),
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
    near(178, "near_pivot_r4", Base::CprWidth),
    near(179, "near_pivot_s4", Base::CprWidth),
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
    near(188, "near_cpr_tc", Base::CprWidth),
    near(189, "near_cpr_bc", Base::CprWidth),
    // ---- 190–197. VWAP bands 2 and 3, completed. -------------------------
    //
    // `docs/09-design-sources.md` §2 ships THREE band multipliers — 1.0, 2.0,
    // 3.0. Positions 143–152 gave band 1 all five relations and band 2 only its
    // two outer sides; band 3 had nothing at all. A band the design draws and
    // the vocabulary cannot name is a level the sweep can never test.
    near(190, "near_vwap_band2_upper", Base::SessionRange),
    near(191, "near_vwap_band2_lower", Base::SessionRange),
    plain(192, "inside_vwap_band2"),
    plain(193, "close_above_vwap_band3_upper"),
    plain(194, "close_below_vwap_band3_lower"),
    near(195, "near_vwap_band3_upper", Base::SessionRange),
    near(196, "near_vwap_band3_lower", Base::SessionRange),
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
        Some(Base::CprWidth),
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        236,
        "close_above_forming_pivot_pivot_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        237,
        "close_below_forming_pivot_pivot_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        238,
        "near_forming_pivot_cpr_bc",
        Some(Base::CprWidth),
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        239,
        "close_above_forming_pivot_cpr_bc_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        240,
        "close_below_forming_pivot_cpr_bc_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        241,
        "near_forming_pivot_cpr_tc",
        Some(Base::CprWidth),
        "constant true except on a single-price day: |C - tc| equals the band exactly",
    ),
    void(
        242,
        "close_above_forming_pivot_cpr_tc_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        243,
        "close_below_forming_pivot_cpr_tc_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        244,
        "near_forming_pivot_r1",
        Some(Base::CprWidth),
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        245,
        "close_above_forming_pivot_r1_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        246,
        "close_below_forming_pivot_r1_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        247,
        "near_forming_pivot_r2",
        Some(Base::CprWidth),
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        248,
        "close_above_forming_pivot_r2_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        249,
        "close_below_forming_pivot_r2_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        250,
        "near_forming_pivot_r3",
        Some(Base::CprWidth),
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        251,
        "close_above_forming_pivot_r3_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        252,
        "close_below_forming_pivot_r3_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        253,
        "near_forming_pivot_r4",
        Some(Base::CprWidth),
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        254,
        "close_above_forming_pivot_r4_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        255,
        "close_below_forming_pivot_r4_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        256,
        "near_forming_pivot_r5",
        Some(Base::CprWidth),
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        257,
        "close_above_forming_pivot_r5_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        258,
        "close_below_forming_pivot_r5_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        259,
        "near_forming_pivot_s1",
        Some(Base::CprWidth),
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        260,
        "close_above_forming_pivot_s1_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        261,
        "close_below_forming_pivot_s1_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        262,
        "near_forming_pivot_s2",
        Some(Base::CprWidth),
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        263,
        "close_above_forming_pivot_s2_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        264,
        "close_below_forming_pivot_s2_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        265,
        "near_forming_pivot_s3",
        Some(Base::CprWidth),
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        266,
        "close_above_forming_pivot_s3_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        267,
        "close_below_forming_pivot_s3_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        268,
        "near_forming_pivot_s4",
        Some(Base::CprWidth),
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        269,
        "close_above_forming_pivot_s4_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        270,
        "close_below_forming_pivot_s4_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        271,
        "near_forming_pivot_s5",
        Some(Base::CprWidth),
        "constant false: |C - level| is a fixed multiple of the band on every bar",
    ),
    void(
        272,
        "close_above_forming_pivot_s5_band",
        None,
        "carries only sign(v - u), the running-range axis bits 40-43 already hold",
    ),
    void(
        273,
        "close_below_forming_pivot_s5_band",
        None,
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
    // ---- 276–279. Two threshold-free predicates, appended. ----------------
    //
    // Both are derived from state the evaluator ALREADY HOLDS and neither needs a
    // number, which is what makes them appendable under §3 rule 1. The CPR width
    // cut points at 274/275 needed a threshold nobody could source, and the
    // resolution there was to append and record it UNVERIFIED in
    // `docs/09-design-sources.md` §5. These need no such note: a comparison and a
    // latch, both already computed.
    //
    // 276–277: THE CLOSE AGAINST TODAY'S OWN OPEN. `Evaluator.running_open` was
    // written on the first bar of every session and READ NOWHERE in the
    // repository -- a field maintained for a question the vocabulary could not
    // ask. Every percent-change quote a trader reads is measured against it, and
    // "up on the day" was inexpressible: 40-43 give position within the day's
    // RANGE, which is a different question, and 30-31 compare the close to the
    // BAR's open.
    //
    // A flat close sets NEITHER, which is the rule D-0109 locked for 37/38 and
    // 30/31 already followed: equality is a third state, not a tie broken toward
    // one side.
    plain(276, "close_above_day_open"),
    plain(277, "close_below_day_open"),
    // 278–279: THE MARKET STRUCTURE IN FORCE. 56-59 are BREAK EVENTS -- bos_up,
    // bos_down, choch_up, choch_down -- each true on the handful of bars where a
    // level was taken out. `Structure::last` holds which direction is in force
    // BETWEEN those events, which is the regime, and it was unpublished: a sweep
    // could ask "did structure break up on this bar" and could not ask "is the
    // structure up".
    //
    // Same source as 56-59 and no new formula: the latch these read is the latch
    // those events already advance. Neither is set before the first break, which
    // is `docs/03-vocabulary.md` §4 -- there is no structure yet, and "probably
    // up" is not a value a bit may take.
    plain(278, "structure_up_in_force"),
    plain(279, "structure_down_in_force"),
    // ---- 280–313. THE MOMENT OF A CROSS, WHICH THE TABLE COULD NOT EXPRESS. -
    //
    // Every level relation above this line is a STATE. `close_above_ema20` is
    // true on bar 500, on bar 501, and on every bar of a two-hour trend — so a
    // sweep could ask where price IS and could not ask what it just DID. There
    // was no edge detector of any kind: measured before these rows existed, a
    // grep for `cross` across this whole file returned ZERO, and the only
    // `previous_close` in `crates/indicators` feeds SuperTrend's true range and
    // no side test.
    //
    // Even the break family is a state rather than an edge: `bos_up` (56) fires
    // on EVERY bar where the close is above the swing high, not only the first.
    //
    // # Derived from the mask, so no module computes anything new
    //
    // A crossing is two states one bar apart, and both states are already in
    // this table. `crossed_up_X` is `close_above_X` false on the previous bar
    // and true on this one; `crossed_down_X` is the same for `close_below_X`.
    // `CROSSINGS` below names the four indices per level, and
    // `indicators::evaluator` derives all 34 from the previous bar's mask in one
    // pass whose length is a compile-time constant. No indicator module changed,
    // no level is recomputed, and nothing reads a bar this crate has not
    // already read.
    //
    // # A flat close crosses nothing, and that falls out of D-0109
    //
    // A close exactly ON a level sets NEITHER side. So a bar landing on the
    // level clears `close_above_X` without setting `close_below_X`, and the next
    // bar rising off it sets `crossed_up_X` — which is right, and is a property
    // of the source rows rather than a rule restated here.
    //
    // # Seventeen levels, and the two that were deliberately left out
    //
    // Thirty-five names carry `close_above_`. Sixteen are `void` — the
    // `forming_pivot_*` family, algebraically constant by D-0080 — and a
    // crossing of a level that never changes side can never fire.
    //
    // The other two are VWAP: 52/53 `vwap` and 143/144 `vwap_session`. Those
    // positions are LIVE and are nonetheless dead on every runnable path,
    // because the only production `Evaluator::new` passes
    // `vwap::Availability::Absent` and the whole twenty-position VWAP family is
    // silenced. Adding a crossing that provably cannot fire today would be
    // adding, permanently, exactly the defect that family is criticised for.
    // Append-only cuts the other way here: they can be added the day VWAP is
    // wired, and cannot be removed if they are added now.
    plain(280, "crossed_up_ema20"),
    plain(281, "crossed_down_ema20"),
    plain(282, "crossed_up_ema200"),
    plain(283, "crossed_down_ema200"),
    plain(284, "crossed_up_pdh"),
    plain(285, "crossed_down_pdh"),
    plain(286, "crossed_up_pdl"),
    plain(287, "crossed_down_pdl"),
    plain(288, "crossed_up_supertrend"),
    plain(289, "crossed_down_supertrend"),
    plain(290, "crossed_up_gap_mid"),
    plain(291, "crossed_down_gap_mid"),
    plain(292, "crossed_up_pivot_r1_band"),
    plain(293, "crossed_down_pivot_r1_band"),
    plain(294, "crossed_up_pivot_r2_band"),
    plain(295, "crossed_down_pivot_r2_band"),
    plain(296, "crossed_up_pivot_r3_band"),
    plain(297, "crossed_down_pivot_r3_band"),
    plain(298, "crossed_up_pivot_s1_band"),
    plain(299, "crossed_down_pivot_s1_band"),
    plain(300, "crossed_up_pivot_s2_band"),
    plain(301, "crossed_down_pivot_s2_band"),
    plain(302, "crossed_up_pivot_s3_band"),
    plain(303, "crossed_down_pivot_s3_band"),
    plain(304, "crossed_up_pivot_r4_band"),
    plain(305, "crossed_down_pivot_r4_band"),
    plain(306, "crossed_up_pivot_s4_band"),
    plain(307, "crossed_down_pivot_s4_band"),
    plain(308, "crossed_up_pivot_r5_band"),
    plain(309, "crossed_down_pivot_r5_band"),
    plain(310, "crossed_up_pivot_s5_band"),
    plain(311, "crossed_down_pivot_s5_band"),
    plain(312, "crossed_up_day_open"),
    plain(313, "crossed_down_day_open"),
    // ---- 314–364. WHICH TEST OF THIS LEVEL THIS IS. -------------------------
    //
    // 280–313 gave the vocabulary an EDGE: the bar a level changes side. This
    // gives that edge an ORDINAL, and the difference is one a trader acts on.
    // `crossed_up_pdh` fires byte-identically on the clean first break of the
    // day and on the ninth probe of a level that has already absorbed eight
    // attempts. Those are not the same setup and the mask could not tell them
    // apart.
    //
    // # No combination of existing bits can express a count
    //
    // `ConditionMask::hits` is pure conjunction, so "not the first test" cannot
    // be spelled by leaving a bit clear, and a tally cannot be assembled out of
    // level states however many are ANDed. It has to be positions. A survey of
    // every one of the 314 names for `count|times|twice|nth|repeat|again|
    // retest|touch|tally|streak|consec|freq|revisit` returned two rows, both
    // false positives on `pat_counterattack_*`.
    //
    // # Counting CROSSINGS, not "touches", and that choice is what made it fit
    //
    // No tracked document defines a touch -- `near_X`, a close on the level, or
    // a wick through it are three different predicates -- so picking one would
    // be a stated assumption with its own decision entry, and picking wrongly
    // would make 51 positions mean something nobody asked for. A crossing is
    // already defined, already derived, and already tested, so the ordinal
    // inherits its whole definition from 280–313 and introduces no new one.
    //
    // # Three buckets, and exactly one fires
    //
    // First, second, third-or-later. A bit per exact count would be unbounded;
    // the third is open-ended because the distinction a trader draws is fresh /
    // retest / being hammered, not fourth from fifth. They PARTITION the
    // crossing bars: exactly one on a bar that crosses, none on a bar that does
    // not, which is what makes them safe to AND with anything else.
    //
    // Per SESSION, reset in the rollover beside `previous_mask` -- an overnight
    // change of side is a gap and not a test of the level.
    plain(314, "first_cross_ema20"),
    plain(315, "second_cross_ema20"),
    plain(316, "third_plus_cross_ema20"),
    plain(317, "first_cross_ema200"),
    plain(318, "second_cross_ema200"),
    plain(319, "third_plus_cross_ema200"),
    plain(320, "first_cross_pdh"),
    plain(321, "second_cross_pdh"),
    plain(322, "third_plus_cross_pdh"),
    plain(323, "first_cross_pdl"),
    plain(324, "second_cross_pdl"),
    plain(325, "third_plus_cross_pdl"),
    plain(326, "first_cross_supertrend"),
    plain(327, "second_cross_supertrend"),
    plain(328, "third_plus_cross_supertrend"),
    plain(329, "first_cross_gap_mid"),
    plain(330, "second_cross_gap_mid"),
    plain(331, "third_plus_cross_gap_mid"),
    plain(332, "first_cross_pivot_r1_band"),
    plain(333, "second_cross_pivot_r1_band"),
    plain(334, "third_plus_cross_pivot_r1_band"),
    plain(335, "first_cross_pivot_r2_band"),
    plain(336, "second_cross_pivot_r2_band"),
    plain(337, "third_plus_cross_pivot_r2_band"),
    plain(338, "first_cross_pivot_r3_band"),
    plain(339, "second_cross_pivot_r3_band"),
    plain(340, "third_plus_cross_pivot_r3_band"),
    plain(341, "first_cross_pivot_s1_band"),
    plain(342, "second_cross_pivot_s1_band"),
    plain(343, "third_plus_cross_pivot_s1_band"),
    plain(344, "first_cross_pivot_s2_band"),
    plain(345, "second_cross_pivot_s2_band"),
    plain(346, "third_plus_cross_pivot_s2_band"),
    plain(347, "first_cross_pivot_s3_band"),
    plain(348, "second_cross_pivot_s3_band"),
    plain(349, "third_plus_cross_pivot_s3_band"),
    plain(350, "first_cross_pivot_r4_band"),
    plain(351, "second_cross_pivot_r4_band"),
    plain(352, "third_plus_cross_pivot_r4_band"),
    plain(353, "first_cross_pivot_s4_band"),
    plain(354, "second_cross_pivot_s4_band"),
    plain(355, "third_plus_cross_pivot_s4_band"),
    plain(356, "first_cross_pivot_r5_band"),
    plain(357, "second_cross_pivot_r5_band"),
    plain(358, "third_plus_cross_pivot_r5_band"),
    plain(359, "first_cross_pivot_s5_band"),
    plain(360, "second_cross_pivot_s5_band"),
    plain(361, "third_plus_cross_pivot_s5_band"),
    plain(362, "first_cross_day_open"),
    plain(363, "second_cross_day_open"),
    plain(364, "third_plus_cross_day_open"),
    // ---- 365–369. Day of week. -----------------------------------------
    //
    // NSE trades Monday to Friday, so five bits and no more. They are `plain`
    // rather than banded because a weekday is exact: a bar is on Tuesday or it
    // is not, and there is no near-Tuesday.
    //
    // # Why these are worth five of the nineteen free positions
    //
    // Every other condition in this table describes what PRICE did. These
    // describe WHEN, and the two compose: `close_above_vwap` on a Monday and the
    // same condition on a Friday are different statements about the market, and
    // until now the sweep could not tell them apart. Expiry sits on a Thursday
    // for NIFTY, so Thursday and Friday carry a structural difference no price
    // condition can express.
    //
    // The four session-phase bits at 44–47 already give the sweep WHEN WITHIN a
    // day. These give it WHICH day, and the pair is what an operator means by
    // "Monday morning behaves differently".
    plain(365, "is_monday"),
    plain(366, "is_tuesday"),
    plain(367, "is_wednesday"),
    plain(368, "is_thursday"),
    plain(369, "is_friday"),
];

/// Every level whose SIDE this table carries on both sides, and the two
/// positions that name the moment it changes.
///
/// Each tuple is `(above, below, crossed_up, crossed_down)`. The first two are
/// the shipped state positions; the last two are set by
/// `indicators::Evaluator` when the corresponding state position was clear on
/// the previous bar and is set on this one.
///
/// # Why this lives in `vocab` and not in `indicators`
///
/// It is a statement about which POSITIONS mean what, which is this crate's
/// authority and no other's. `indicators` reads it; nothing else needs to know
/// the shape. Putting it beside the rows also means a reviewer sees the four
/// indices together, which is the only way to check them: an off-by-one here
/// would set `crossed_up_pdh` from `close_above_pdl` and nothing about the
/// resulting mask would look wrong.
///
/// # The order is the table's order and the length is asserted
///
/// `the_crossing_map_names_only_live_two_sided_levels` walks every tuple and
/// checks all four positions are live, that the two state positions really are
/// the `close_above_`/`close_below_` pair of one level, and that the two
/// crossing positions carry that level's name — so the map cannot drift from
/// the rows above it.
/// One level's seven positions: two states, two edges, three ordinals.
///
/// # A named struct and not a seven-tuple, and the reason is a real hazard
///
/// This was `(u16, u16, u16, u16)` and its own doc warned that *"an off-by-one
/// here would set `crossed_up_pdh` from `close_above_pdl` and nothing about the
/// resulting mask would look wrong"*. Seven positional `u16`s would make that
/// warning three times as sharp. Named fields cannot be permuted silently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelCrossing {
    /// `close_above_X` — the state, true for as long as it holds.
    pub above: u16,
    /// `close_below_X` — the state on the other side.
    pub below: u16,
    /// `crossed_up_X` — the EDGE: `above` clear on the previous bar, set now.
    pub up: u16,
    /// `crossed_down_X` — the same for `below`.
    pub down: u16,
    /// `first_cross_X` — this bar is the session's FIRST crossing of the level,
    /// in either direction.
    pub first: u16,
    /// `second_cross_X` — the second.
    pub second: u16,
    /// `third_plus_cross_X` — the third or any later one.
    pub later: u16,
}

/// Every two-sided level, with its states, its edges and its ordinals.
///
/// # The ordinal family, and why the crossing is the countable event
///
/// D-0244 gave the vocabulary an EDGE — the bar a level changes side — and left
/// a question a trader asks constantly unanswerable: *is this the first test of
/// this level today, or the fourth?* A mask carrying `crossed_up_pdh` fires
/// byte-identically on the clean first break and on the ninth probe of a level
/// that has already absorbed eight attempts, and those are not the same setup.
///
/// **No AND of existing bits can express it.** `ConditionMask::hits` is pure
/// conjunction — `(self & candidate) ^ candidate == 0` across six words — so
/// "not the first test" cannot be spelled by leaving a bit clear, and a count
/// cannot be assembled from level states at all. It has to be positions.
///
/// # Counting CROSSINGS and not "touches", which is the whole reason this fits
///
/// A survey of this vocabulary found no tracked document defines what a *touch*
/// is — whether it is `near_X`, a close landing on the level, or a wick through
/// it — so choosing one would be a stated assumption needing its own decision
/// entry, and choosing wrongly would make 51 positions mean something nobody
/// asked for.
///
/// A CROSSING needs no such choice. It is already defined, already derived from
/// the mask, and already tested: `above` clear then set, or `below` clear then
/// set. So "the third crossing of R1 today" inherits its whole definition from
/// [`Self::up`] and [`Self::down`] and introduces no new one.
///
/// # Three buckets and not a counter
///
/// First, second, third-or-later. A bit per exact count would be unbounded; the
/// third bucket is open-ended because a trader distinguishes *fresh*, *retest*
/// and *being hammered*, and not fourth from fifth.
///
/// Exactly one of the three is set on any bar that crosses, and none is set on a
/// bar that does not — so they partition the crossing bars rather than
/// overlapping them, which is what makes them safe to combine with anything
/// else.
///
/// # Reset at the session boundary, like the crossing itself
///
/// The count is per SESSION. `indicators::Evaluator` clears it in the rollover
/// beside `previous_mask`, for the same reason: an overnight change of side is a
/// gap, not a test of the level, and this engine is intraday-only by
/// `CLAUDE.md` §1.
pub const CROSSINGS: [LevelCrossing; 17] = [
    LevelCrossing {
        above: 0,
        below: 1,
        up: 280,
        down: 281,
        first: 314,
        second: 315,
        later: 316,
    },
    LevelCrossing {
        above: 2,
        below: 3,
        up: 282,
        down: 283,
        first: 317,
        second: 318,
        later: 319,
    },
    LevelCrossing {
        above: 13,
        below: 14,
        up: 284,
        down: 285,
        first: 320,
        second: 321,
        later: 322,
    },
    LevelCrossing {
        above: 15,
        below: 16,
        up: 286,
        down: 287,
        first: 323,
        second: 324,
        later: 325,
    },
    LevelCrossing {
        above: 64,
        below: 65,
        up: 288,
        down: 289,
        first: 326,
        second: 327,
        later: 328,
    },
    LevelCrossing {
        above: 66,
        below: 67,
        up: 290,
        down: 291,
        first: 329,
        second: 330,
        later: 331,
    },
    LevelCrossing {
        above: 74,
        below: 75,
        up: 292,
        down: 293,
        first: 332,
        second: 333,
        later: 334,
    },
    LevelCrossing {
        above: 76,
        below: 77,
        up: 294,
        down: 295,
        first: 335,
        second: 336,
        later: 337,
    },
    LevelCrossing {
        above: 78,
        below: 79,
        up: 296,
        down: 297,
        first: 338,
        second: 339,
        later: 340,
    },
    LevelCrossing {
        above: 80,
        below: 81,
        up: 298,
        down: 299,
        first: 341,
        second: 342,
        later: 343,
    },
    LevelCrossing {
        above: 82,
        below: 83,
        up: 300,
        down: 301,
        first: 344,
        second: 345,
        later: 346,
    },
    LevelCrossing {
        above: 84,
        below: 85,
        up: 302,
        down: 303,
        first: 347,
        second: 348,
        later: 349,
    },
    LevelCrossing {
        above: 180,
        below: 181,
        up: 304,
        down: 305,
        first: 350,
        second: 351,
        later: 352,
    },
    LevelCrossing {
        above: 182,
        below: 183,
        up: 306,
        down: 307,
        first: 353,
        second: 354,
        later: 355,
    },
    LevelCrossing {
        above: 184,
        below: 185,
        up: 308,
        down: 309,
        first: 356,
        second: 357,
        later: 358,
    },
    LevelCrossing {
        above: 186,
        below: 187,
        up: 310,
        down: 311,
        first: 359,
        second: 360,
        later: 361,
    },
    LevelCrossing {
        above: 276,
        below: 277,
        up: 312,
        down: 313,
        first: 362,
        second: 363,
        later: 364,
    },
];

/// How many positions the table defines. Not how many bits the mask holds --
/// [`ConditionMask::BITS`] is 384, and the **19** positions between are
/// unallocated headroom, not free-for-all space.
///
/// It read 104 until the crossing family took the table from 280 to 314. The
/// number is pinned in `vocab::tests::the_version_is_the_widened_table` so this
/// sentence cannot drift again without a red test.
pub const COUNT: usize = TABLE.len();

/// The highest position that will ever be a hole: none. The next condition
/// appends here, whatever has been retired below it.
pub const NEXT_FREE: u16 = 370;

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
        .with_bit(275)
        .with_bit(276)
        .with_bit(277)
        .with_bit(278)
        .with_bit(279)
        // 280-313: the crossing family. All 34 are live and all land in word 4,
        // bits 24-57. Spelled one per line like their neighbours rather than
        // folded into the word literal, because `the_live_mask_is_the_table`
        // compares this against the rows and a hand-computed word is exactly
        // the arithmetic that check exists to refuse.
        .with_bit(280)
        .with_bit(281)
        .with_bit(282)
        .with_bit(283)
        .with_bit(284)
        .with_bit(285)
        .with_bit(286)
        .with_bit(287)
        .with_bit(288)
        .with_bit(289)
        .with_bit(290)
        .with_bit(291)
        .with_bit(292)
        .with_bit(293)
        .with_bit(294)
        .with_bit(295)
        .with_bit(296)
        .with_bit(297)
        .with_bit(298)
        .with_bit(299)
        .with_bit(300)
        .with_bit(301)
        .with_bit(302)
        .with_bit(303)
        .with_bit(304)
        .with_bit(305)
        .with_bit(306)
        .with_bit(307)
        .with_bit(308)
        .with_bit(309)
        .with_bit(310)
        .with_bit(311)
        .with_bit(312)
        .with_bit(313)
        // 314-364: the crossing-ordinal family. All 51 live.
        .with_bit(314)
        .with_bit(315)
        .with_bit(316)
        .with_bit(317)
        .with_bit(318)
        .with_bit(319)
        .with_bit(320)
        .with_bit(321)
        .with_bit(322)
        .with_bit(323)
        .with_bit(324)
        .with_bit(325)
        .with_bit(326)
        .with_bit(327)
        .with_bit(328)
        .with_bit(329)
        .with_bit(330)
        .with_bit(331)
        .with_bit(332)
        .with_bit(333)
        .with_bit(334)
        .with_bit(335)
        .with_bit(336)
        .with_bit(337)
        .with_bit(338)
        .with_bit(339)
        .with_bit(340)
        .with_bit(341)
        .with_bit(342)
        .with_bit(343)
        .with_bit(344)
        .with_bit(345)
        .with_bit(346)
        .with_bit(347)
        .with_bit(348)
        .with_bit(349)
        .with_bit(350)
        .with_bit(351)
        .with_bit(352)
        .with_bit(353)
        .with_bit(354)
        .with_bit(355)
        .with_bit(356)
        .with_bit(357)
        .with_bit(358)
        .with_bit(359)
        .with_bit(360)
        .with_bit(361)
        .with_bit(362)
        .with_bit(363)
        .with_bit(364)
        // 365–369: the five weekday rows. Plain and live, like every other
        // position in this literal that names a fact about the bar rather than a
        // level it is near.
        .with_bit(365)
        .with_bit(366)
        .with_bit(367)
        .with_bit(368)
        .with_bit(369);

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
/// **And it must be the right *kind* of number, not merely a pinned one.** The
/// tolerance names the [`Base`] it was measured on and the row names the one it
/// requires; a mismatch is [`VocabError::WrongBand`]. Before that check existed
/// this function asked only whether the position was a `near_*` row, so the
/// pivot width on a Fibonacci rung -- fifty times too wide at today's pins --
/// returned `Ok`, and the resulting mask was indistinguishable from a correct
/// one for the rest of the run's life.
///
/// # Errors
///
/// [`VocabError::NoSuchBit`] past the end of the table;
/// [`VocabError::Retired`] for a tombstone; [`VocabError::Void`] for a
/// definitionally-constant position; [`VocabError::NotNear`] for a position
/// that decides on its own, which takes [`set_exact`];
/// [`VocabError::WrongBand`] when the tolerance measures against the other
/// family's quantity, or against none.
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
    // `def.band` and not `def.kind`: the two say the same thing -- `kind_of`
    // derives one from the other -- but only `band` carries the value the next
    // check needs. Reading `kind` here and `band` below would be two lookups of
    // one fact, and a `let ... else` on `band` after a `kind` test writes an arm
    // no input can reach, which `cargo llvm-cov` counts forever as a region no
    // test closed. The `else` here IS reachable: every plain row takes it.
    let Some(expected) = def.band else {
        return Err(VocabError::NotNear { index });
    };
    // THE BAND FAMILY IS CHECKED BEFORE THE BAND IS APPLIED.
    //
    // Without this the function accepted either width for any `near_*` row, so
    // a caller handing the CPR-width band to a Fibonacci rung -- fifty times too
    // wide at today's pins -- got `Ok` and a set bit. `CLAUDE.md` §4: degrade
    // loudly and name the reason, or refuse. This refuses, and the refusal
    // carries both bases so the message says which side was wrong.
    //
    // It has to be here rather than in `Tolerance::covers`, which sees only two
    // `i64` spans and cannot tell a session range from a CPR width. The position
    // is the only thing that knows which family it belongs to.
    //
    // WHAT THIS DOES NOT CATCH: `range_paisa`. A caller can still pass the right
    // TOLERANCE and the wrong SPAN -- the session range where the CPR width
    // belongs -- and every type here is satisfied. That failure is a bar wrong
    // rather than a vocabulary wrong, and it stays with the seven modules of
    // `crates/indicators` that derive the span.
    if tolerance.base() != Some(expected) {
        return Err(VocabError::WrongBand {
            index,
            expected,
            got: tolerance.base(),
        });
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

    /// The other family's width, on [`Base::CprWidth`].
    ///
    /// A second helper and not a parameter on the first, because the whole
    /// content of the tests below is that these two are **not**
    /// interchangeable: a test that could reach either through one call would
    /// be one argument away from proving nothing.
    fn pivot_tolerance() -> Tolerance {
        crate::tolerance::pinned_pivot()
            .expect("the pinned pivot width is neither the sentinel nor negative")
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
            328,
            "370 positions, less three tombstones and less the 39 void \
             forming-pivot rows. 323 + the five weekday bits."
        );
    }

    #[test]
    fn a_tombstone_is_forced_false_however_it_arrived() {
        let dirty = ConditionMask::ZERO
            .with_bit(6)
            .with_bit(19)
            .with_bit(25)
            .with_bit(62)
            // 380 is past NEXT_FREE and inside the mask, so it exercises the
            // "allocated in the mask but not in the table" case. It was 200,
            // which stopped being unallocated the day the forming-pivot block
            // landed, and then 300, which stopped being unallocated the day the
            // crossing block did. The moral each time: this bit must sit above
            // NEXT_FREE, so it moves with NEXT_FREE.
            .with_bit(380);
        let clean = only_live(dirty);
        assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
        assert!(clean.get(62), "a live bit beside a tombstone survives");
        assert!(!clean.get(380), "unallocated positions are not live either");
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

    /// 22 and not 17. Both are live `near_*` rows and the arithmetic is
    /// identical, but 17 `near_pdh` declares [`Base::CprWidth`] -- it is built
    /// from the daily levels by `crates/indicators/src/daily.rs` and measured
    /// against the CPR width -- so the session-range width this test carries is
    /// now refused there. 22 `near_fib_50` is the same shape on the family the
    /// width belongs to.
    #[test]
    fn set_near_needs_the_band_to_actually_cover() {
        let tol = test_tolerance();
        let level = 2_500_000i64;
        let hit = set_near(ConditionMask::ZERO, 22, tol, level + 200, level, 20_000)
            .expect("22 is live and near on the session range");
        assert!(hit.get(22));
        let miss = set_near(ConditionMask::ZERO, 22, tol, level + 200_000, level, 20_000)
            .expect("22 is live and near on the session range");
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
        assert_eq!(
            d.band,
            Some(Base::CprWidth),
            "a tombstone keeps the family it was in, the same way it keeps its \
             name; the retirement premise for 6 holds only on the CPR width"
        );
        assert_eq!(d.status, BitStatus::Retired { duplicate_of: 62 });
        assert!(format!("{d:?}").contains("near_pivot_p"));
    }

    #[test]
    fn count_and_next_free_agree_with_the_table() {
        // 370 = 365 + the five weekday rows. `NEXT_FREE` must equal `COUNT` or
        // the next family would be appended over a live position, which §3
        // rule 8 forbids absolutely: condition bits are never renumbered or
        // reused.
        assert_eq!(COUNT, 370);
        assert_eq!(usize::from(NEXT_FREE), COUNT);
        // AND THE MASK STILL HOLDS THEM, at COMPILE time. 384 bits against 370
        // positions leaves fourteen free; a family that pushed past 384 would
        // re-key every run ever recorded, because `vocab_version` is one of the
        // nine terms §3 rule 3 hashes.
        //
        // A `const` block and not a runtime assertion: both operands are
        // constants, so a runtime check would only restate what the compiler
        // already knew -- and clippy is right to call that a constant assertion.
        // This way the BUILD fails if a family is ever appended past the mask.
        const {
            assert!(COUNT <= crate::mask::WORDS * 64);
        }
    }

    /// **A row constructor carries the `band` and the `reason` it is handed,
    /// derives `kind` from the `band`, and does not substitute the answer
    /// today's rows happen to want.**
    ///
    /// [`TABLE`] is a `const`, so until this test existed the four constructors
    /// were only ever evaluated by the compiler and no test ever called one.
    /// That hid three mistakes that no table-wide check can see:
    ///
    /// * All three tombstones are [`Kind::Near`] today, so a [`retired`] that
    ///   ignored its band argument and hardcoded a near row would pass every
    ///   test in this crate -- until the first plain position is retired, at
    ///   which point a position that decides on its own starts demanding a
    ///   tolerance.
    /// * `crates/vocab/tests/table.rs` asserts only that a void row's reason is
    ///   non-empty, so a [`void`] that ignored its `reason` argument and wrote
    ///   one fixed sentence for all 39 rows would pass that too -- and every
    ///   void row would then explain itself with another row's identity, which
    ///   is the reason travelling in [`VocabError::Void`] to whoever asked.
    /// * **The band a constructor stores decides which tolerance [`set_near`]
    ///   accepts for that position.** A constructor that dropped it and wrote
    ///   one family for every row would pass every count in
    ///   `crates/vocab/tests/table.rs`, because those count [`Kind`] and `Kind`
    ///   is *derived* -- so it would still come out `Near` -- while half the
    ///   table quietly started accepting a fifty-times-wrong width again. Each
    ///   case below therefore hands in the band that is **not** what the row
    ///   shape suggests: the near row is built on the pivot family, the void
    ///   row on the session range.
    ///
    /// The indices are [`NEXT_FREE`] on purpose: this is the constructors' own
    /// contract and not a claim about any shipped row.
    #[test]
    fn the_row_constructors_carry_the_band_and_the_reason_they_are_handed() {
        let synthetic = NEXT_FREE;

        assert_eq!(
            plain(synthetic, "a_plain_row"),
            BitDef {
                index: synthetic,
                name: "a_plain_row",
                kind: Kind::Plain,
                band: None,
                status: BitStatus::Live,
            },
            "`plain` builds a live row that decides on its own and needs no band"
        );
        assert_eq!(
            near(synthetic, "a_near_row", Base::CprWidth),
            BitDef {
                index: synthetic,
                name: "a_near_row",
                kind: Kind::Near,
                band: Some(Base::CprWidth),
                status: BitStatus::Live,
            },
            "`near` differs from `plain` in exactly two fields, and `kind` is \
             the one it computes rather than the one it is told"
        );
        // The band handed in is `SessionRange` while the name says nothing about
        // a family, so a `near` that hardcoded `CprWidth` -- the value the case
        // above uses -- is caught here and not there.
        assert_eq!(
            near(synthetic, "another_near_row", Base::SessionRange).band,
            Some(Base::SessionRange),
            "`near` must store the base it was handed; it is the value that \
             decides which tolerance `set_near` will accept for the row"
        );
        assert_eq!(
            retired(synthetic, "a_retired_row", None, 62),
            BitDef {
                index: synthetic,
                name: "a_retired_row",
                kind: Kind::Plain,
                band: None,
                status: BitStatus::Retired { duplicate_of: 62 },
            },
            "`retired` must pass its band through; no shipped tombstone is \
             plain, so hardcoding a near row here would go unnoticed"
        );
        assert_eq!(
            void(
                synthetic,
                "a_void_row",
                Some(Base::SessionRange),
                "this exact sentence"
            ),
            BitDef {
                index: synthetic,
                name: "a_void_row",
                kind: Kind::Near,
                band: Some(Base::SessionRange),
                status: BitStatus::Void {
                    reason: "this exact sentence"
                },
            },
            "`void` must carry the reason verbatim and the band verbatim; all \
             thirteen shipped void near rows are `CprWidth`, so a hardcoded \
             family would go unnoticed in the table"
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
    ///
    /// **Void also outranks [`VocabError::WrongBand`], and 238 is the row that
    /// proves it.** It is a `near_forming_pivot_*` row, so its declared band is
    /// [`Base::CprWidth`], while `tol` here is the session-range width -- both
    /// refusals are available and only one is right. Void is: `WrongBand` tells
    /// a caller to come back with the other width, and 238 refuses that call
    /// too, so it would send them after a call that cannot exist. Same rule as
    /// the `NotNear` case below, same reason.
    #[test]
    fn set_near_refuses_a_void_position_even_where_the_band_would_cover() {
        let tol = test_tolerance();
        let level = 2_500_000i64;

        // 238 is `near_forming_pivot_cpr_bc`: void AND near AND on the other
        // band family, so `set_near` is the only door it has and three arms
        // could answer. Void is the one that must.
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

        // The control: 22 is live, near, on `tol`'s own family, and the very
        // same arguments set it. So the two refusals above are the void arm and
        // not a band that missed -- nor, now, a band family that mismatched.
        let live = set_near(ConditionMask::ZERO, 22, tol, level, level, 20_000)
            .expect("22 is live, near, and on the session range");
        assert!(
            live.get(22),
            "these arguments do cover, so the refusals above are the void arm"
        );
    }

    /// **The other family's width is refused, in both directions, on arguments
    /// that would otherwise have set the bit.**
    ///
    /// Every call below passes `value_paisa == level_paisa`, which
    /// [`Tolerance::covers`] answers true for at any legal width. So the band
    /// test cannot be what refuses these: delete the band-family check and all
    /// four calls return `Ok` with the bit set. That is what [`set_near`] did
    /// before, and it is why this is a test about `Err` values rather than
    /// about masks -- the mask a wrong band produces is byte for byte the mask
    /// a right one produces, and no assertion on it could tell them apart.
    ///
    /// Both directions, because a check written `expected == Base::CprWidth`
    /// rather than `expected == tolerance.base()` would refuse one family and
    /// wave the other through, and one direction cannot see that.
    #[test]
    fn set_near_refuses_the_other_familys_band_and_still_takes_its_own() {
        let fib = test_tolerance();
        let pivot = pivot_tolerance();
        let level = 2_500_000i64;
        let range = 20_000i64;

        // 7 `near_pivot_r1` measures against the CPR width. The session-range
        // width is fifty times narrower and means something else entirely.
        assert_eq!(
            set_near(ConditionMask::ZERO, 7, fib, level, level, range),
            Err(VocabError::WrongBand {
                index: 7,
                expected: Base::CprWidth,
                got: Some(Base::SessionRange),
            }),
            "a pivot row must refuse the Fibonacci width even on an exact hit"
        );
        // 22 `near_fib_50` measures against the session range, and the pivot
        // width is the fifty-times-too-wide direction of the same mistake.
        assert_eq!(
            set_near(ConditionMask::ZERO, 22, pivot, level, level, range),
            Err(VocabError::WrongBand {
                index: 22,
                expected: Base::SessionRange,
                got: Some(Base::CprWidth),
            }),
            "a Fibonacci row must refuse the pivot width even on an exact hit"
        );

        // The happy paths, on the same two rows and the same arguments, so the
        // refusals above are the family check and not the rows being unusable.
        let on_pivot = set_near(ConditionMask::ZERO, 7, pivot, level, level, range)
            .expect("7 is live, near, and this is its own family's width");
        assert!(on_pivot.get(7), "the right band still sets the bit");
        let on_fib = set_near(ConditionMask::ZERO, 22, fib, level, level, range)
            .expect("22 is live, near, and this is its own family's width");
        assert!(on_fib.get(22), "the right band still sets the bit");

        // A width with no base at all. `Tolerance::from_milli` produces one and
        // it is a legal width -- the refusal has to be about the missing base,
        // and has to carry `got: None` so the message can say so rather than
        // naming a family the caller never chose.
        let baseless = Tolerance::from_milli(crate::tolerance::TOL_FIB_MILLI)
            .expect("a width of TOL_FIB_MILLI is neither the sentinel nor negative");
        assert_eq!(baseless.milli(), fib.milli(), "same number, no base");
        assert_eq!(
            set_near(ConditionMask::ZERO, 22, baseless, level, level, range),
            Err(VocabError::WrongBand {
                index: 22,
                expected: Base::SessionRange,
                got: None,
            }),
            "the right number with no stated base is not the right band"
        );
    }

    /// **What the wrong band actually did, in paisa.**
    ///
    /// The test above proves the refusal; this one proves the refusal was worth
    /// having, because "50x" is a ratio and a ratio does not say whether any bar
    /// ever landed between the two.
    ///
    /// One bar, 40.00 points off a level on a 200.00-point session. Its own
    /// band -- 22 `near_fib_50` on [`Base::SessionRange`] at
    /// [`TOL_FIB_MILLI`] -- is 2.00 points, so the bit is **false**. The pivot
    /// width over the same span is 100.00 points, so before the family check the
    /// same call set the bit and the mask reported `near_fib_50` on a bar twenty
    /// times outside the band that name refers to.
    ///
    /// [`Tolerance::covers`] is called directly for both widths first. That is
    /// the part that has not changed and must not: the refusal has to come from
    /// the family check, not from the arithmetic quietly starting to disagree.
    #[test]
    fn the_wrong_band_would_have_set_a_bit_on_a_bar_its_own_band_excludes() {
        let fib = test_tolerance();
        let pivot = pivot_tolerance();
        let level = 2_500_000i64;
        let range = 20_000i64;
        let off_by = 4_000i64;

        assert!(
            !fib.covers(level + off_by, level, range),
            "40.00 points is outside the 2.00-point session-range band"
        );
        assert!(
            pivot.covers(level + off_by, level, range),
            "and inside the 100.00-point band the CPR width would give, which \
             is the whole of the discrepancy"
        );

        assert_eq!(
            set_near(ConditionMask::ZERO, 22, pivot, level + off_by, level, range),
            Err(VocabError::WrongBand {
                index: 22,
                expected: Base::SessionRange,
                got: Some(Base::CprWidth),
            }),
            "this is the call that used to return Ok with bit 22 set"
        );

        let honest = set_near(ConditionMask::ZERO, 22, fib, level + off_by, level, range)
            .expect("22 is live and near on the session range");
        assert!(
            honest.is_empty(),
            "on its own band the bar is outside, so the bit is false -- which \
             is the answer the wrong band replaced with true"
        );
    }

    /// **Every `near_*` row declares the family its name belongs to, and the two
    /// families have the sizes the modules that set them have.**
    ///
    /// The band is a per-row declaration, so nothing but this reads all 97 of
    /// them together. Without it a single row typed into the wrong family is
    /// invisible: `crates/vocab/tests/table.rs` counts [`Kind`], and `Kind` is
    /// derived from the band, so a `near_fib_*` row declared `CprWidth` still
    /// counts as near and every count there still passes -- while that one
    /// position silently starts demanding, and accepting, a fifty-times-wrong
    /// width.
    ///
    /// The rule is the name, with **two positions the name does not decide**.
    /// 17 `near_pdh` and 18 `near_pdl` are Fibonacci-sounding rungs by name and
    /// pivot-family by construction: `crates/indicators/src/daily.rs` builds
    /// them from the same `DailyLevels` as R1–R5 and hands its whole plan one
    /// width, the CPR width. They are named here rather than folded into the
    /// name rule, because an exception inside a rule is an exception nobody
    /// reads.
    ///
    /// The counts are the second half. A name rule alone is satisfied by a table
    /// with no `near_*` rows at all.
    #[test]
    fn every_near_row_declares_the_family_its_name_belongs_to() {
        // The two rows whose family their name does not give away.
        let pivot_by_construction = ["near_pdh", "near_pdl"];

        let mut live_cpr = 0;
        let mut live_range = 0;
        let mut all_near = 0;

        for def in &TABLE {
            let Some(band) = def.band else {
                assert_eq!(
                    def.kind,
                    Kind::Plain,
                    "position {} declares no band, so it cannot need one",
                    def.index
                );
                continue;
            };
            all_near += 1;
            assert_eq!(
                def.kind,
                Kind::Near,
                "position {} declares a band, so it needs a tolerance",
                def.index
            );

            let named_pivot = def.name.contains("pivot") || def.name.contains("cpr");
            let want = if named_pivot || pivot_by_construction.contains(&def.name) {
                Base::CprWidth
            } else {
                Base::SessionRange
            };
            assert_eq!(band, want, "position {} is `{}`", def.index, def.name);

            if def.status == BitStatus::Live {
                match band {
                    Base::CprWidth => live_cpr += 1,
                    Base::SessionRange => live_range += 1,
                }
            }
        }

        assert_eq!(
            live_cpr, 14,
            "the fourteen live positions `crates/indicators/src/daily.rs` sets \
             from one plan on the CPR width: 7-12, 17, 18, 54, 55, 178, 179, \
             188, 189"
        );
        assert_eq!(
            live_range, 67,
            "every other live near position: the four Fibonacci ladders, the \
             eight opening-range edges, the seven VWAP bands, the gap mid and \
             the two swing levels"
        );
        assert_eq!(
            live_cpr + live_range,
            81,
            "and together they are the 81 live near positions \
             `crates/vocab/tests/table.rs` counts by `Kind`"
        );
        assert_eq!(
            all_near, 97,
            "the 81 live, the three tombstones, and the thirteen void \
             forming-pivot rows"
        );
    }
}

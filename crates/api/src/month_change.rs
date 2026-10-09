//! One instrument-month's own percentage change, in integer basis points,
//! and the named reasons it is not a number.
//!
//! # Moved, not rewritten
//!
//! Every item below was cut out of `server.rs` verbatim by the WP-01 seam and
//! is mounted there as a child module, so it reads that file's private
//! helpers through `super::` exactly as it did when it lived in it, and
//! `server.rs` imports each name back. The only edits are visibility
//! qualifiers (`pub(super)`) on items another module now has to reach.
//! No route answers differently because of the move.

/// Why one percentage is not a number.
///
/// **Five reasons, five different sentences on screen.** A single `null`
/// covering all of them would collapse five distinct facts into one, which is
/// the fallback that hides a failure `CLAUDE.md` §4 bans — "degrade loudly and
/// name the reason". Each variant's [`Unknown::code`] is what goes on the wire
/// and what the page keys its prose off; the page never invents a sentence for
/// a code it does not know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unknown {
    /// The instrument can be split, bonused or otherwise re-based, and no
    /// threshold that would detect it is sourced anywhere in this repository.
    ///
    /// **This is checked FIRST, ahead of whether a close was even recorded.**
    /// It is a standing property of the instrument, not of today's data: an
    /// operator told "no close recorded" would re-ingest the month and expect a
    /// number, and for an equity none would ever arrive. The permanent reason
    /// outranks the temporary one.
    CorporateActionUnverified,
    /// The census holds the month and no close was ever read for it — a
    /// version-1 entry, or a version-2 one written before the file was priced.
    /// `pull::manifest::Closes::UNKNOWN`, D-0067.
    NotRecorded,
    /// The previous month is not in this census. The first month an instrument
    /// holds is the ordinary case; a gap in the middle is the other one.
    NoEarlierMonth,
    /// The base of the ratio is zero paisa. `CLAUDE.md` §7 makes zero a real
    /// price, and `docs/02-store-format.md` §3 makes an all-zero record a legal
    /// flat bar, so this arm is reachable rather than defensive — and a ratio
    /// with no base is undefined, never `0`, never `∞`.
    BaseNotPositive,
    /// The move times 10,000 leaves `i64`. Refused rather than wrapped.
    Overflow,
    /// The previous HELD month is not the calendar month immediately before
    /// this one, so "against the previous month's close" would measure across
    /// a hole. `vs_prev_close` only (D-3325, DB-081).
    PreviousMonthNotAdjacent,
}

impl Unknown {
    /// The wire code. `snake_case`, stable, and the page's key.
    pub(super) const fn code(self) -> &'static str {
        match self {
            Self::CorporateActionUnverified => "corporate_action_unverified",
            Self::NotRecorded => "not_recorded",
            Self::NoEarlierMonth => "no_earlier_month",
            Self::BaseNotPositive => "base_not_positive",
            Self::Overflow => "overflow",
            Self::PreviousMonthNotAdjacent => "previous_month_not_adjacent",
        }
    }
}

/// Whether a corporate action can re-base this series' prices.
///
/// `docs/05-decisions.md` D-0018, in its own words: **"Indices never split;
/// equities do."** An index is a computed level, so a constituent's split moves
/// the divisor and not the level. Cash equities split, bonus and consolidate.
/// F&O contracts on those equities are adjusted when the underlying is, so a
/// stock derivative inherits the hazard — and an INDEX option does not: its
/// underlying never splits (D-3325, DB-G66). Every one of the 21,385 index
/// option instrument-months was refused as an equity hazard before this.
///
/// The match is exhaustive on purpose: a fourth segment fails to compile here
/// rather than defaulting into whichever answer was written last.
fn can_be_rebased(segment: brutex_core::instrument::Segment, symbol: &str) -> bool {
    match segment {
        brutex_core::instrument::Segment::Index => false,
        brutex_core::instrument::Segment::Cash => true,
        brutex_core::instrument::Segment::Fno => !crate::bars::underlying_is_index(symbol),
    }
}

/// Basis points between two paisa closes, in integer arithmetic throughout.
///
/// `CLAUDE.md` §7: prices are paisa integers, never a float. A ratio derived
/// from money is the same arithmetic and gets the same treatment —
/// `clippy::float_arithmetic` is a workspace deny (D-0061) and no `f64` appears
/// at any step here. **A month that moved 1.25% is `125`.**
///
/// # Rounding: half AWAY FROM ZERO, not half up
///
/// §7's half-up rule is about snapping a *price* to the tick grid at the *write
/// boundary*. This is neither a price nor a write. Half-up would render −0.5 bp
/// as `0` and +0.5 bp as `1`, so a gain and its mirror-image loss would print
/// different magnitudes — visible the moment the column is sorted. Away from
/// zero is symmetric. D-0069.
///
/// # Why `i64` and not `i32`
///
/// `i32` is unsafe and the counterexample uses only values the store can hold:
/// a base of `1` paisa and a last close of ₹2,147.50 — an ordinary NSE share
/// price — gives 2,147,490,000 bp, past `i32::MAX`. Proven by
/// `api::server::basis_points_do_not_fit_i32_and_an_ordinary_price_proves_it`.
///
/// # No precondition on the sign
///
/// The manifest caller passes `pull::manifest::Closes::paisa`, which refuses
/// every negative, but `bars::with_change` passes a raw stored close or open
/// interest, and a store read does not validate prices (an unsealed month is
/// read with no checksum). One corrupt bar of `i64::MIN` made the subtraction
/// overflow, and the release profile aborts on that, taking the whole server
/// down from one GET (CE-2, D-1761). The subtraction is therefore checked.
///
/// # Errors
///
/// [`Unknown::BaseNotPositive`] when the base is zero, [`Unknown::Overflow`]
/// when the scaled move leaves `i64`.
pub(crate) fn basis_points(first_paisa: i64, last_paisa: i64) -> Result<i64, Unknown> {
    // A RATIO NEEDS A BASE, AND ZERO IS NOT ONE. Not rendered as 0.00%, not as
    // infinity, not as an empty cell that means five other things.
    if first_paisa <= 0 {
        return Err(Unknown::BaseNotPositive);
    }
    let Some(delta) = last_paisa.checked_sub(first_paisa) else {
        return Err(Unknown::Overflow);
    };
    let Some(scaled) = delta.checked_mul(10_000) else {
        return Err(Unknown::Overflow);
    };
    // `first_paisa >= 1`, so neither the divide-by-zero nor the `i64::MIN / -1`
    // case exists. Rust truncates toward zero, so the remainder carries the
    // sign of `scaled` and the correction below is away from zero.
    let whole = scaled / first_paisa;
    let rest = scaled % first_paisa;
    let (step, magnitude) = if rest >= 0 { (1, rest) } else { (-1, -rest) };
    // `magnitude * 2 >= first_paisa`, written as a subtraction because
    // `magnitude < first_paisa` makes `first_paisa - magnitude` safe and the
    // doubling would not be.
    if magnitude >= first_paisa - magnitude {
        // Rounding at all requires `first_paisa >= 2` — at a base of 1 the
        // remainder is always 0 — so `|whole| <= i64::MAX / 2` here and the
        // step cannot overflow.
        return Ok(whole + step);
    }
    Ok(whole)
}

/// One month's own percentage change: its first close to its last close.
///
/// # Why intra-month and not month-over-month
///
/// A row of this table is an instrument-**month**, so "previous % change" is
/// the previous month's *same statistic*, not the previous bar's. Given that,
/// the month's own change is a property of the row itself: it needs no
/// neighbour, it is defined for an instrument's first held month, and the
/// "previous" column costs one probe instead of two. D-0069 records the
/// alternative and what it was measured to cost.
///
/// # `closes` is an `Option`, and that is what makes both columns one function
///
/// `None` is "the census does not hold that month" — reachable only from the
/// **previous**-month call site, because the current month is in hand by the
/// time this is asked about it. Passing the census's answer rather than a
/// pre-decided verdict is what puts the corporate-action gate ahead of the
/// missing-neighbour case in *both* columns. It has to be: an equity told
/// `no_earlier_month` invites an operator to ingest the previous month and wait
/// for a number that a sourced threshold, not a pull, is what unlocks.
///
/// # Errors
///
/// [`Unknown::CorporateActionUnverified`] for anything that can be re-based,
/// [`Unknown::NoEarlierMonth`] when the census holds no such month,
/// [`Unknown::NotRecorded`] when it holds the month and no close, and whatever
/// [`basis_points`] refuses.
pub(super) fn month_change(
    segment: brutex_core::instrument::Segment,
    symbol: &str,
    closes: Option<pull::manifest::Closes>,
) -> Result<i64, Unknown> {
    // FIRST, AND UNCONDITIONALLY. `docs/05-decisions.md` D-0018 decided the
    // behaviour — refuse the window loudly, never back-adjust from a source no
    // vendor has been verified to supply — and grep finds zero implementation
    // of the detector in `crates/`. No threshold is sourced in this repository,
    // so an equity's percentage is refused rather than printed with a caveat
    // beside it: the number travels and the caveat does not.
    if can_be_rebased(segment, symbol) {
        return Err(Unknown::CorporateActionUnverified);
    }
    let Some(closes) = closes else {
        return Err(Unknown::NoEarlierMonth);
    };
    let Some((first, last)) = closes.paisa() else {
        return Err(Unknown::NotRecorded);
    };
    basis_points(first, last)
}

/// What the census holds before a month, for [`vs_previous_close`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "WP-30 puts `vs_prev_close` on both census encodings at once (D-3325, \
                  DB-081): `compact_census_preserves_every_expanded_row_and_unknown_reason` \
                  holds the JSON row equal to the compact tuple, and the page's \
                  `store-census-wire.js` refuses a tuple that is not ten columns, so one \
                  encoding alone breaks the other; built and tested here"
    )
)]
pub(super) enum Previous {
    /// The calendar month immediately before is held, with these closes.
    Adjacent(pull::manifest::Closes),
    /// An earlier month is held, and the one immediately before is not.
    Gap,
    /// No earlier month is held.
    None,
}

/// The months [`previous_of`] looks back past an absent month before it says
/// no earlier month is held: a year, which is wider than any hole a series of
/// this store has (the widest measured, Zerodha's 60-day Muhurat holes,
/// DB-012, is two months). Bounded so a row costs at most this many probes.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "WP-30 puts `vs_prev_close` on both census encodings at once (D-3325, \
                  DB-081): `compact_census_preserves_every_expanded_row_and_unknown_reason` \
                  holds the JSON row equal to the compact tuple, and the page's \
                  `store-census-wire.js` refuses a tuple that is not ten columns, so one \
                  encoding alone breaks the other; built and tested here"
    )
)]
pub(super) const GAP_LOOKBACK_MONTHS: usize = 12;

/// What the census holds before `month`, by asking `held` about the month
/// immediately before and, when it is absent, up to [`GAP_LOOKBACK_MONTHS`]
/// more. At most thirteen hash probes; no file is touched.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "WP-30 puts `vs_prev_close` on both census encodings at once (D-3325, \
                  DB-081): `compact_census_preserves_every_expanded_row_and_unknown_reason` \
                  holds the JSON row equal to the compact tuple, and the page's \
                  `store-census-wire.js` refuses a tuple that is not ten columns, so one \
                  encoding alone breaks the other; built and tested here"
    )
)]
pub(super) fn previous_of(
    month: store::path::YearMonth,
    mut held: impl FnMut(store::path::YearMonth) -> Option<pull::manifest::Closes>,
) -> Previous {
    let Some(before) = month_before(month) else {
        return Previous::None;
    };
    if let Some(closes) = held(before) {
        return Previous::Adjacent(closes);
    }
    let mut at = before;
    for _ in 0..GAP_LOOKBACK_MONTHS {
        let Some(earlier) = month_before(at) else {
            break;
        };
        if held(earlier).is_some() {
            return Previous::Gap;
        }
        at = earlier;
    }
    Previous::None
}

/// The month's last close against the PREVIOUS month's last close, in basis
/// points — "vs previous month close", beside the intra-month [`month_change`]
/// (D-3325, DB-081).
///
/// The intra-month figure excludes the first day's move by design (D-0069):
/// NIFTY 1day 2026-09 reads −5.97% first close to last, and −6.06% from the
/// 31 Aug close. Both are now on the row, each with its own label.
///
/// # Errors
///
/// [`Unknown::CorporateActionUnverified`] for a series that can be re-based,
/// [`Unknown::NoEarlierMonth`] when no earlier month is held,
/// [`Unknown::PreviousMonthNotAdjacent`] when the held month before is not the
/// one immediately before, [`Unknown::NotRecorded`] when either month holds no
/// close, and whatever [`basis_points`] refuses.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "WP-30 puts `vs_prev_close` on both census encodings at once (D-3325, \
                  DB-081): `compact_census_preserves_every_expanded_row_and_unknown_reason` \
                  holds the JSON row equal to the compact tuple, and the page's \
                  `store-census-wire.js` refuses a tuple that is not ten columns, so one \
                  encoding alone breaks the other; built and tested here"
    )
)]
pub(super) fn vs_previous_close(
    segment: brutex_core::instrument::Segment,
    symbol: &str,
    closes: pull::manifest::Closes,
    previous: Previous,
) -> Result<i64, Unknown> {
    if can_be_rebased(segment, symbol) {
        return Err(Unknown::CorporateActionUnverified);
    }
    let before = match previous {
        Previous::Adjacent(before) => before,
        Previous::Gap => return Err(Unknown::PreviousMonthNotAdjacent),
        Previous::None => return Err(Unknown::NoEarlierMonth),
    };
    let (Some((_, base)), Some((_, last))) = (before.paisa(), closes.paisa()) else {
        return Err(Unknown::NotRecorded);
    };
    basis_points(base, last)
}

/// The month before this one, or `None` at the first month the store addresses.
///
/// Arithmetic on the calendar, never a loop over it, and there is deliberately
/// **no** `next`: `CLAUDE.md` §3 rule 7 says a computation at month N may read
/// months 0..N, and the cheapest enforcement is that the function that walks
/// forward does not exist.
pub(super) fn month_before(month: store::path::YearMonth) -> Option<store::path::YearMonth> {
    let (year, ordinal) = if month.month() > 1 {
        (month.year(), month.month() - 1)
    } else {
        // `YearMonth::new` refuses a year below 1970, and that refusal is this
        // function's `None`: 1970-01 has no month before it that the store can
        // name. `month.year()` is at least 1970, so the subtraction is safe.
        (month.year() - 1, 12)
    };
    store::path::YearMonth::new(year, ordinal).ok()
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;
    use crate::server::percentage_tests::{month, priced};
    use brutex_core::instrument::Segment;
    use pull::manifest::Closes;

    /// **A MONTH THAT MOVED 1.25% IS 125, AND A FLAT MONTH IS A REAL ZERO.**
    ///
    /// The comment on `store_json` promised integer basis points for years
    /// before anything computed one. This is the arithmetic that comment
    /// describes, in the units it names: ₹100.00 to ₹101.25 is `125`.
    ///
    /// The flat case is the other half and it is not decoration. Zero basis
    /// points is a month that closed where it opened — a fact — and the whole
    /// design of the `null`-plus-reason wire shape exists so that fact is never
    /// confused with "nobody knows". If `0` could mean both, the column would
    /// be unreadable on exactly the rows an operator looks hardest at.
    #[test]
    fn a_month_that_moved_1_25_percent_is_125_basis_points_and_a_flat_month_is_a_real_zero() {
        assert_eq!(
            month_change(Segment::Index, "NIFTY", Some(priced(10_000, 10_125))),
            Ok(125),
            "₹100.00 to ₹101.25 is +1.25%, which is 125 basis points"
        );
        assert_eq!(
            month_change(Segment::Index, "NIFTY", Some(priced(10_000, 9_875))),
            Ok(-125),
            "and the same move down is -125, sign carried by the number itself"
        );
        assert_eq!(
            month_change(Segment::Index, "NIFTY", Some(priced(295_050, 295_050))),
            Ok(0),
            "a month that closed where it opened is ZERO, not unknown"
        );
    }

    /// **HALF ROUNDS AWAY FROM ZERO, AND IT DOES SO IN BOTH DIRECTIONS.**
    ///
    /// `CLAUDE.md` §7's half-up rule governs snapping a PRICE to the tick grid
    /// at the WRITE boundary. A percentage is neither. Under half-up, +0.5 bp
    /// renders `1` and -0.5 bp renders `0` — a gain and its mirror-image loss
    /// printing different magnitudes, which is visible the moment the column is
    /// sorted and inexplicable when it is noticed.
    ///
    /// The fixture is an exact tie by construction: a base of 32 paisa and a
    /// one-paisa move is 10,000/32 = 312.5 basis points exactly, so nothing
    /// here depends on a rounding accident. A half-up implementation returns
    /// `313` and `-312` and fails the second assertion alone.
    #[test]
    fn basis_points_round_half_away_from_zero_in_both_directions() {
        assert_eq!(
            month_change(Segment::Index, "NIFTY", Some(priced(32, 33))),
            Ok(313),
            "312.5 bp exactly, rounded away from zero"
        );
        assert_eq!(
            month_change(Segment::Index, "NIFTY", Some(priced(32, 31))),
            Ok(-313),
            "-312.5 bp exactly, rounded away from zero — NOT -312"
        );
        assert_eq!(
            month_change(Segment::Index, "NIFTY", Some(priced(32, 34))),
            Ok(625),
            "and a move that divides exactly is not nudged"
        );
    }

    /// **A BASE OF ZERO IS REFUSED, NOT DIVIDED BY.**
    ///
    /// `docs/02-store-format.md` §3: "An all-zero record is a legal flat bar",
    /// so a first close of zero paisa is a state the format permits and this is
    /// a check rather than a defensive comment. A ratio against it is undefined
    /// — it is not `0`, it is not infinity, and it is not an empty cell that
    /// means four other things.
    #[test]
    fn a_base_of_zero_paisa_is_refused_rather_than_divided_by() {
        assert_eq!(
            month_change(Segment::Index, "NIFTY", Some(priced(0, 500))),
            Err(Unknown::BaseNotPositive)
        );
        assert_eq!(
            month_change(Segment::Index, "NIFTY", Some(priced(0, 0))),
            Err(Unknown::BaseNotPositive),
            "zero to zero is still no base, and 0% would be an invention"
        );
        assert_eq!(Unknown::BaseNotPositive.code(), "base_not_positive");
    }

    /// **A MOVE THAT LEAVES `i64` IS REFUSED, NOT WRAPPED.**
    ///
    /// The scaling by 10,000 is the only step that can overflow, and the bound
    /// is exact: it fits while `|delta| <= i64::MAX / 10_000 =
    /// 922,337,203,685,477` paisa. Both sides of that boundary are asserted, so
    /// the arm is a bound and not a guess.
    #[test]
    fn a_move_that_leaves_i64_is_refused_rather_than_wrapped() {
        let widest = i64::MAX / 10_000;
        assert_eq!(
            month_change(Segment::Index, "NIFTY", Some(priced(1, widest + 1))),
            Ok(widest * 10_000),
            "a base of one paisa makes the delta the whole value; this one fits"
        );
        assert_eq!(
            month_change(Segment::Index, "NIFTY", Some(priced(1, widest + 2))),
            Err(Unknown::Overflow),
            "one paisa further and the scaled move leaves i64"
        );
        assert_eq!(Unknown::Overflow.code(), "overflow");
    }

    /// **BASIS POINTS DO NOT FIT `i32`, AND AN ORDINARY PRICE PROVES IT.**
    ///
    /// `i32::MAX` is 2,147,483,647 basis points, which sounds unreachable until
    /// the base is small: a first close of one paisa and a last close of
    /// ₹2,147.50 — an utterly ordinary NSE share price — is 2,147,490,000 basis
    /// points. A one-paisa base is garbage data, and the store can hold it;
    /// `CLAUDE.md` §3 rule 6 does not permit claiming a bound that is not
    /// enforced. So the value is computed and carried as `i64`.
    #[test]
    fn basis_points_do_not_fit_i32_and_an_ordinary_price_proves_it() {
        let over =
            month_change(Segment::Index, "NIFTY", Some(priced(1, 214_750))).expect("a number");
        assert_eq!(over, 2_147_490_000);
        assert!(
            over > i64::from(i32::MAX),
            "{over} is past i32::MAX = {}, from a base of ₹0.01 and a close of ₹2,147.50",
            i32::MAX
        );
    }

    /// **NO EQUITY RENDERS A NUMBER WHILE NO THRESHOLD IS SOURCED.**
    ///
    /// `docs/05-decisions.md` D-0018 already decided this: a suspected
    /// corporate action is refused loudly, never back-adjusted from a source no
    /// vendor has been verified to supply. No threshold exists anywhere in this
    /// repository, so every segment that can be re-based is refused and only
    /// an index — which never splits — renders.
    ///
    /// **The gate outranks the data, and the third assertion is why.** An
    /// equity whose closes ARE recorded is still refused; an equity whose
    /// closes are NOT recorded is refused for the corporate action rather than
    /// for the missing close, because an operator told "no close recorded"
    /// would re-ingest the month and wait forever for a number.
    #[test]
    fn no_equity_month_renders_a_number_while_no_threshold_is_sourced() {
        assert_eq!(
            month_change(Segment::Index, "NIFTY", Some(priced(10_000, 10_125))),
            Ok(125),
            "an index never splits, so it renders"
        );
        assert_eq!(
            month_change(Segment::Cash, "RELIANCE", Some(priced(10_000, 10_125))),
            Err(Unknown::CorporateActionUnverified),
            "the same two prices under a segment that can split are refused"
        );
        assert_eq!(
            month_change(Segment::Fno, "RELIANCE", Some(priced(10_000, 10_125))),
            Err(Unknown::CorporateActionUnverified),
            "a contract is adjusted when its underlying is, so it inherits the hazard"
        );
        assert_eq!(
            month_change(Segment::Cash, "RELIANCE", Some(Closes::UNKNOWN)),
            Err(Unknown::CorporateActionUnverified),
            "the permanent reason outranks the temporary one"
        );
        // AND IT OUTRANKS THE MISSING NEIGHBOUR TOO, which is the arm that
        // makes the Prev column consistent. An equity whose previous month is
        // simply absent must not read `no_earlier_month`: that sends an
        // operator to ingest a month, and no pull will ever make this cell a
        // number. Only a sourced threshold will.
        assert_eq!(
            month_change(Segment::Cash, "RELIANCE", None),
            Err(Unknown::CorporateActionUnverified)
        );
        assert_eq!(
            month_change(Segment::Index, "NIFTY", None),
            Err(Unknown::NoEarlierMonth),
            "for an index, an absent neighbour is exactly what it says"
        );
        assert_eq!(
            Unknown::CorporateActionUnverified.code(),
            "corporate_action_unverified"
        );
        assert!(
            !can_be_rebased(Segment::Index, "NIFTY")
                && can_be_rebased(Segment::Cash, "RELIANCE")
                && can_be_rebased(Segment::Fno, "RELIANCE")
        );
    }

    /// **AN INDEX OPTION'S MONTH SHOWS A NUMBER; A STOCK OPTION'S IS
    /// REFUSED.** D-3325, DB-G66: every NIFTY and BANKNIFTY option month was
    /// refused as `corporate_action_unverified` because the gate read `FNO`
    /// alone, and the cell said "this instrument can split" about a contract
    /// on an index, which never splits. The gate now reads the underlying.
    #[test]
    fn an_index_option_month_renders_and_a_stock_option_month_is_refused() {
        for index in [
            "NIFTY",
            "BANKNIFTY",
            "FINNIFTY",
            "MIDCPNIFTY",
            "NIFTYNXT50",
            "NIFTYIT",
            "NIFTYFPI",
            "NIFTYCPSE",
        ] {
            assert!(crate::bars::underlying_is_index(index), "{index}");
            assert_eq!(
                month_change(Segment::Fno, index, Some(priced(10_000, 10_125))),
                Ok(125),
                "{index} options are written on an index"
            );
        }
        for share in ["RELIANCE", "NTPC", "BAJAJ-AUTO", "NIFTYBEES", "NIFT"] {
            assert!(!crate::bars::underlying_is_index(share), "{share}");
            assert_eq!(
                month_change(Segment::Fno, share, Some(priced(10_000, 10_125))),
                Err(Unknown::CorporateActionUnverified),
                "{share}: an unrecognised underlying refuses rather than prints"
            );
        }
        // The first held month of an index option is `no_earlier_month`, not
        // an equity hazard.
        assert_eq!(
            month_change(Segment::Fno, "NIFTY", None),
            Err(Unknown::NoEarlierMonth)
        );
    }

    /// **"VS PREVIOUS MONTH CLOSE" IS THE LAST CLOSE BEFORE AGAINST THE LAST
    /// CLOSE NOW, AND EACH REASON IT IS NOT A NUMBER IS NAMED.** D-3325,
    /// DB-081: NIFTY 1day 2026-09 read −5.97% first close to last, and −6.06%
    /// from the 31 Aug close. Invented closes: ₹1,040.00 last month, then
    /// ₹1,030.00 to ₹977.60 this month.
    #[test]
    fn the_change_against_the_previous_month_close_names_every_refusal() {
        let august = priced(100_000, 104_000);
        let september = priced(103_000, 97_760);
        assert_eq!(
            vs_previous_close(
                Segment::Index,
                "NIFTY",
                september,
                Previous::Adjacent(august)
            ),
            Ok(-600),
            "₹1,040.00 to ₹977.60 is -6.00%"
        );
        assert_eq!(
            month_change(Segment::Index, "NIFTY", Some(september)),
            Ok(-509),
            "the intra-month figure stays what it was: -5.087% rounds to -509"
        );
        assert_eq!(
            vs_previous_close(Segment::Index, "NIFTY", september, Previous::Gap),
            Err(Unknown::PreviousMonthNotAdjacent)
        );
        assert_eq!(
            Unknown::PreviousMonthNotAdjacent.code(),
            "previous_month_not_adjacent"
        );
        assert_eq!(
            vs_previous_close(Segment::Index, "NIFTY", september, Previous::None),
            Err(Unknown::NoEarlierMonth)
        );
        assert_eq!(
            vs_previous_close(
                Segment::Index,
                "NIFTY",
                september,
                Previous::Adjacent(Closes::UNKNOWN)
            ),
            Err(Unknown::NotRecorded)
        );
        assert_eq!(
            vs_previous_close(
                Segment::Cash,
                "RELIANCE",
                september,
                Previous::Adjacent(august)
            ),
            Err(Unknown::CorporateActionUnverified)
        );
        assert_eq!(
            vs_previous_close(Segment::Fno, "NIFTY", september, Previous::Adjacent(august)),
            Ok(-600),
            "an index option measures across months like its index"
        );
    }

    /// **THE PREVIOUS HELD MONTH IS FOUND BY PROBES, AT MOST THIRTEEN.** The
    /// month immediately before is `Adjacent`; an earlier one within a year
    /// is a `Gap`; nothing within a year, or the store's first month, is
    /// `None`.
    #[test]
    fn the_previous_held_month_is_adjacent_a_gap_or_none_within_bounded_probes() {
        let held = |months: &'static [(u16, u8)]| {
            move |at: store::path::YearMonth| {
                months
                    .iter()
                    .any(|&(y, m)| (at.year(), at.month()) == (y, m))
                    .then(|| priced(1, 2))
            }
        };
        assert_eq!(
            previous_of(month(2026, 9), held(&[(2026, 8)])),
            Previous::Adjacent(priced(1, 2))
        );
        assert_eq!(
            previous_of(month(2026, 9), held(&[(2026, 6)])),
            Previous::Gap
        );
        assert_eq!(
            previous_of(month(2026, 9), held(&[(2025, 9)])),
            Previous::Gap
        );
        assert_eq!(
            previous_of(month(2026, 9), held(&[(2025, 8)])),
            Previous::Gap,
            "the month before and twelve more: 2026-08 back to 2025-08"
        );
        assert_eq!(
            previous_of(month(2026, 9), held(&[(2025, 7)])),
            Previous::None,
            "fourteen months back is past the bound"
        );
        assert_eq!(previous_of(month(1970, 1), held(&[])), Previous::None);
        let mut probes = 0;
        let _ = previous_of(month(2026, 9), |_| {
            probes += 1;
            None
        });
        assert_eq!(probes, 1 + GAP_LOOKBACK_MONTHS);
    }

    /// **THE MONTH BEFORE JANUARY IS THE DECEMBER BEFORE IT, AND 1970-01 HAS
    /// NONE.**
    ///
    /// The year boundary is the case a naive `month - 1` gets wrong, and the
    /// floor is the case an unchecked one gets wrong. There is no `month_after`
    /// on purpose — `CLAUDE.md` §3 rule 7 forbids look-ahead, and a function
    /// that does not exist cannot be called.
    #[test]
    fn the_month_before_january_is_december_and_the_first_month_has_none() {
        assert_eq!(month_before(month(2026, 8)), Some(month(2026, 7)));
        assert_eq!(month_before(month(2026, 1)), Some(month(2025, 12)));
        assert_eq!(
            month_before(month(1970, 1)),
            None,
            "1970-01 is the first month the store can name"
        );
        assert_eq!(month_before(month(1970, 2)), Some(month(1970, 1)));
    }
}

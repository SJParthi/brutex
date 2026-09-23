//! One decoded lake bar, and the one place a lake rupee becomes paisa.
//!
//! # Why two number systems live in the same struct
//!
//! `CLAUDE.md` §7 draws a line that this module exists to hold:
//!
//! * **Prices are paisa integers (`i64`), never a float.** Open, high, low,
//!   close and the recorded spot are money. They cross out of IEEE double
//!   exactly once, at [`paisa_from_lake`] below, and are integers forever
//!   after.
//! * **Statistical values keep full precision and are never rounded for
//!   storage.** The greeks are that: implied volatility, delta, gamma, theta,
//!   vega, rho, the year fraction and the interest rate used. They stay `f64`.
//!
//! **If you are reading this intending to "fix" the greeks into integers,
//! stop.** They are not money and there is no tick grid for them. Snapping a
//! gamma of 0.00017142680429549402 onto a two-decimal grid destroys it
//! outright — it becomes 0.00 — and the same is true of every other greek in
//! the block. The `f64` there is the rule, not an oversight. The prices beside
//! them are integers for exactly the same paragraph's other half.

use brutex_core::price::Paisa;

use crate::error::LakeError;

/// The open-interest null sentinel.
///
/// `CLAUDE.md` §7: "`i64::MIN` is the open-interest null sentinel. Zero means
/// zero." Those are two different facts about the world — a contract with no
/// open position and a bar where the vendor reported nothing at all — and they
/// must never collapse into each other.
///
/// This is not hypothetical tidiness. Open interest is null in 12.24% of the
/// 170,547 F&O rows measured and 0.96% of the 78,448 cash rows, so a reader
/// that mapped null to zero would invent hundreds of thousands of "zero open
/// interest" bars that the vendor never claimed.
pub const OPEN_INTEREST_NULL: i64 = i64::MIN;

/// The eight full-precision values the lake records beside an F&O bar.
///
/// Measured across 120 real F&O files: these eight are null **as one unit** —
/// in every null pattern observed, either all eight are present or all eight
/// are absent. That is why they are one `Option<Greeks>` and not eight
/// independent options. `spot_at_bar` is deliberately *not* in here: it goes
/// null at a different rate (0.19% against 5.52%) and in patterns where the
/// greeks are present and it is not, so folding it in would have been an
/// invention the data contradicts.
///
/// Every field is `f64` on purpose. See this module's header.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Greeks {
    /// Implied volatility, as a fraction (0.6786… is 67.86%).
    pub iv: f64,
    /// Delta.
    pub delta: f64,
    /// Gamma.
    pub gamma: f64,
    /// Theta.
    pub theta: f64,
    /// Vega.
    pub vega: f64,
    /// Rho.
    pub rho: f64,
    /// The year fraction to expiry the greeks were computed against.
    pub t_years_used: f64,
    /// The interest rate the greeks were computed against.
    pub rate_used: f64,
}

/// One bar, decoded.
///
/// The cash and index files carry the first seven fields; an F&O file carries
/// all of them. A cash bar therefore has `spot_at_bar`, `greeks` and
/// `greeks_provenance_id` all `None`, which is a statement that the column was
/// not in the file — not that its value was missing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    /// Bar timestamp, in microseconds since the Unix epoch.
    ///
    /// The lake stores this as an `INT64` of microseconds — verified against
    /// the file, where `1_584_685_680_000_000` is 2020-03-20 10:38 IST. This
    /// crate does **not** convert it to a calendar type: `brutex_core` owns
    /// the trading calendar, and a second date rule here could disagree with
    /// it.
    pub timestamp_micros: i64,
    /// Open.
    pub open: Paisa,
    /// High.
    pub high: Paisa,
    /// Low.
    pub low: Paisa,
    /// Close.
    pub close: Paisa,
    /// Traded volume.
    pub volume: i64,
    /// Open interest, or [`OPEN_INTEREST_NULL`] when the vendor reported none.
    ///
    /// Read this through [`Bar::open_interest`] unless you specifically want
    /// the sentinel. The raw field carries it because `CLAUDE.md` §7 fixes the
    /// representation, and a struct that stored an `Option` here would make
    /// the on-disk convention unobservable.
    pub open_interest: i64,
    /// The underlying's spot price at this bar, when the file records one.
    ///
    /// Money, so paisa. Independently nullable — see [`Greeks`].
    pub spot_at_bar: Option<Paisa>,
    /// The full-precision greeks, when the file records them.
    pub greeks: Option<Greeks>,
    /// Which greeks computation produced this row.
    ///
    /// `INT32` in the file, not `INT64`. Present in every F&O row measured,
    /// including rows where the greeks themselves are null — which is why it
    /// is its own field and not part of [`Greeks`].
    pub greeks_provenance_id: Option<i32>,
}

impl Bar {
    /// Open interest as an option, mapping the sentinel back to `None`.
    ///
    /// `Some(0)` and `None` are different answers and this is where the
    /// difference becomes visible to a caller.
    #[must_use]
    pub const fn open_interest(&self) -> Option<i64> {
        if self.open_interest == OPEN_INTEREST_NULL {
            None
        } else {
            Some(self.open_interest)
        }
    }

    /// Whether this bar came from a file carrying the greeks columns.
    #[must_use]
    pub const fn has_greeks(&self) -> bool {
        self.greeks.is_some()
    }
}

/// **The one boundary.** A lake rupee double becomes a paisa integer here and
/// nowhere else in this crate.
///
/// The lake stores prices as IEEE doubles because Polars wrote them that way.
/// `CLAUDE.md` §7 fixes money at paisa integers on a two-decimal tick grid,
/// snapped **half-up**, once, at the write boundary. Reading the lake is that
/// boundary for this data, so it happens here, once per value, and everything
/// downstream is an integer forever.
///
/// The arithmetic itself is deliberately **not** written here. It is
/// [`brutex_core::price::Paisa::from_rupees_half_up`], which the workspace
/// already designates as the only function permitted to do floating-point
/// arithmetic. A second half-up implementation in this crate could drift from
/// that one by a paisa, and a price that disagrees with itself between two
/// readers is exactly the divergent-result-set failure `docs/05-decisions.md`
/// D-0010 records. This function's whole job is to attach *which column* and
/// *which row* to that refusal, because `core` cannot know either.
///
/// # Errors
///
/// [`LakeError::NotRepresentable`], naming the column and row, when the value
/// is NaN, an infinity, or scales past `i64`. It never substitutes: a
/// substituted price is a wrong price that looks right.
pub fn paisa_from_lake(column: &'static str, row: usize, rupees: f64) -> Result<Paisa, LakeError> {
    Paisa::from_rupees_half_up(rupees).map_err(|source| LakeError::NotRepresentable {
        column,
        row,
        source,
    })
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes"
)]
mod tests {
    use super::*;
    use brutex_core::error::PriceError;

    #[test]
    fn the_sentinel_is_i64_min_and_zero_is_not_it() {
        assert_eq!(OPEN_INTEREST_NULL, i64::MIN);
        assert_ne!(OPEN_INTEREST_NULL, 0);
    }

    #[test]
    fn null_open_interest_and_zero_open_interest_are_distinguishable() {
        let null = bar_with_oi(OPEN_INTEREST_NULL);
        let zero = bar_with_oi(0);

        assert_eq!(
            null.open_interest(),
            None,
            "the sentinel reads back as None"
        );
        assert_eq!(zero.open_interest(), Some(0), "zero reads back as zero");
        assert_ne!(
            null.open_interest(),
            zero.open_interest(),
            "a null OI and a zero OI must never be the same answer"
        );
        assert_ne!(null, zero, "and the bars themselves must differ");
    }

    #[test]
    fn conversion_is_exact_and_half_up_on_real_lake_values() {
        // 49.5 is the close of the first bar of the real sample file
        // NSE-NIFTY-01Apr20-10000-CE/1minute/2020/03.parquet, and 8473.1 is
        // its spot_at_bar. Both must land on the paisa their decimal spelling
        // implies, with no off-by-one.
        assert_eq!(paisa_from_lake("close", 0, 49.5).unwrap().raw(), 4_950);
        assert_eq!(
            paisa_from_lake("spot_at_bar", 0, 8473.1).unwrap().raw(),
            847_310
        );

        // A genuine binary tie must go toward positive infinity, on both
        // signs. This is what distinguishes half-up from f64::round.
        assert_eq!(paisa_from_lake("open", 0, 0.125).unwrap().raw(), 13);
        assert_eq!(paisa_from_lake("open", 0, -0.125).unwrap().raw(), -12);
    }

    #[test]
    fn an_unrepresentable_value_is_refused_by_name_with_its_column_and_row() {
        let err = paisa_from_lake("high", 17, f64::NAN).unwrap_err();
        match err {
            LakeError::NotRepresentable {
                column,
                row,
                source,
            } => {
                assert_eq!(column, "high");
                assert_eq!(row, 17);
                assert_eq!(source, PriceError::NotFinite);
            }
            other => panic!("expected NotRepresentable, got {other:?}"),
        }

        // Out of range is a different refusal, and must not be silently
        // saturated to i64::MAX.
        let err = paisa_from_lake("low", 3, f64::MAX).unwrap_err();
        match err {
            LakeError::NotRepresentable { source, .. } => {
                assert_eq!(source, PriceError::OutOfRange);
            }
            other => panic!("expected NotRepresentable, got {other:?}"),
        }

        // Infinity, both signs.
        assert!(paisa_from_lake("open", 0, f64::INFINITY).is_err());
        assert!(paisa_from_lake("open", 0, f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn greeks_keep_full_precision_and_are_not_snapped() {
        // The real gamma from the sample file. Snapping this onto the paisa
        // grid would make it zero; the test exists so that a future change
        // which does so fails here rather than in a backtest six months on.
        let g = Greeks {
            iv: 0.678_618_464_061_773_3,
            delta: 0.103_791_597_602_742_98,
            gamma: 0.000_171_426_804_295_494_02,
            theta: -7.788_961_946_081_791,
            vega: 2.791_593_074_801_929_3,
            rho: 0.276_837_206_382_207_27,
            t_years_used: 0.033_280_060_882_800_61,
            rate_used: 0.065,
        };
        let bar = Bar {
            greeks: Some(g),
            ..bar_with_oi(0)
        };
        let got = bar.greeks.expect("set above");
        assert_eq!(
            got.gamma.to_bits(),
            0.000_171_426_804_295_494_02_f64.to_bits(),
            "gamma must survive bit for bit"
        );
        assert_eq!(got.iv.to_bits(), 0.678_618_464_061_773_3_f64.to_bits());
        assert_eq!(got.theta.to_bits(), (-7.788_961_946_081_791_f64).to_bits());
        assert!(bar.has_greeks());
    }

    #[test]
    fn a_bar_without_greeks_says_so() {
        let bar = bar_with_oi(0);
        assert!(!bar.has_greeks());
        assert_eq!(bar.spot_at_bar, None);
        assert_eq!(bar.greeks_provenance_id, None);
    }

    fn bar_with_oi(oi: i64) -> Bar {
        Bar {
            timestamp_micros: 1_584_685_680_000_000,
            open: Paisa::from_raw(4_950),
            high: Paisa::from_raw(4_950),
            low: Paisa::from_raw(4_950),
            close: Paisa::from_raw(4_950),
            volume: 76,
            open_interest: oi,
            spot_at_bar: None,
            greeks: None,
            greeks_provenance_id: None,
        }
    }
}

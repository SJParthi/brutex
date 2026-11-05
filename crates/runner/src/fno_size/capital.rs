//! The capital actually needed, per policy, shown beside the Winner Gate's base (D-8636).
//!
//! Under the two FIXED policies the quantities never read capital, so the capital actually
//! needed is the smallest starting capital with which every admitted entry still passes SPEC
//! 11.3's margin check and nothing else in the walk changes: `K = max over every admission of
//! (ceil(margin in use right after it / 10^6) - carried net settled then)`, over every window
//! walk the gate reads. Under SIZE-SPLIT-ALL and SIZE-RISK the quantities are shares of the free
//! margin, so any smaller capital gives another walk: `K` is the window's starting capital.
//!
//! The gate's minimum returns stay judged on Rs 20 lakh for every policy (OP-22, the stricter);
//! `K` is shown beside. Because FNO-PORTFOLIO admits only while margin in use is at most the
//! capital, `ceil(margin / 10^6) - carried net` is at most Rs 20 lakh at every admission, so
//! `K` can never loosen a check. The peak of that figure is reported for every policy as the
//! capital in use at peak.

use super::{Invalid, PPM};

/// The running peak of `ceil(margin in use / 10^6) - carried net` over a walk's admissions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CapitalNeeded {
    peak: Option<i128>,
}

impl CapitalNeeded {
    /// Nothing admitted yet.
    #[must_use]
    pub const fn new() -> Self {
        Self { peak: None }
    }

    /// Observes one admission: `margin_after` is the margin in use right after it, in
    /// millionths of a paisa, and `carried_net` the carried net settled then, in paise.
    ///
    /// # Errors
    ///
    /// [`Invalid::NegativeView`] (`"margin_after"`) for a margin below 0 and
    /// [`Invalid::Overflow`] (`"capital in use"`) when the difference passes `i128`. Nothing
    /// changes.
    pub fn observe(&mut self, margin_after: i128, carried_net: i128) -> Result<(), Invalid> {
        if margin_after < 0 {
            return Err(Invalid::NegativeView("margin_after"));
        }
        let ceiling = margin_after.div_euclid(PPM) + i128::from(margin_after.rem_euclid(PPM) != 0);
        let in_use = ceiling
            .checked_sub(carried_net)
            .ok_or(Invalid::Overflow("capital in use"))?;
        self.peak = Some(self.peak.map_or(in_use, |peak| peak.max(in_use)));
        Ok(())
    }

    /// Paise: the capital in use at peak, reported for every policy; `None` with no admission.
    #[must_use]
    pub const fn peak(&self) -> Option<i128> {
        self.peak
    }

    /// Paise: the capital actually needed, `K`. With `reads_capital` (SIZE-SPLIT-ALL and
    /// SIZE-RISK) it is `window_capital`; otherwise the peak. `None` when nothing was admitted:
    /// "no admission", and no return on `K` is printed.
    #[must_use]
    pub fn value(&self, reads_capital: bool, window_capital: i128) -> Option<i128> {
        self.peak
            .map(|peak| if reads_capital { window_capital } else { peak })
    }
}

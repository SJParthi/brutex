//! The precise charges of each trade — brokerage on both legs, the exchange
//! transaction charge, SEBI fee, STT, stamp duty, GST and the rest — net of
//! which every decision is taken (FIB-11). A stub until FIB-11 fills it.
//!
//! Until then [`is_wired`] answers false, and every caller that needs a charge
//! model refuses by name: without charges there is no gate, only a labelled
//! plumbing run (D-6300).

/// Whether the precise charges are built. False until FIB-11.
#[must_use]
pub const fn is_wired() -> bool {
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_charges_are_not_wired_until_fib_11() {
        assert!(!super::is_wired());
    }
}

//! F40 `sizing::qty` (PREVIEW-SPEC section 3.6): Rs 1,00,000 margin x 5 = Rs 5,00,000
//! exposure per trade, sized from the entry fill; the slot never compounds.

/// The exposure per trade: Rs 5,00,000 in paisa.
pub const EXPOSURE: i64 = 50_000_000;
/// The account at its start: Rs 20,00,000 in paisa.
pub const ACCOUNT: i64 = 200_000_000;
/// Open positions at most.
pub const SLOTS: usize = 20;

/// `50_000_000.div_euclid(entry_fill)`; 0 ("zero quantity", not taken) for a price above
/// the exposure or a price that is not positive (an impossible fill).
#[must_use]
pub const fn qty(entry_fill: i64) -> i64 {
    if entry_fill <= 0 {
        0
    } else {
        EXPOSURE.div_euclid(entry_fill)
    }
}

#[cfg(test)]
#[path = "sizing_tests.rs"]
mod tests;

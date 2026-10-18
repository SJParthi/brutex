//! The Fib Flag Breakout detector's half of this crate (FIB-1 scaffold;
//! D-6300). It claims **0 vocabulary positions**: a Fib candidate lands in a
//! cell of `vocab::fib_cube`, a namespace separate from the condition table,
//! and never sets a condition bit.
//!
//! # Who owns what
//!
//! | Module | Owner | What |
//! |---|---|---|
//! | [`types`] | FIB-1, then FIB-3 and FIB-4 add fields | shared value types with no behaviour |
//! | [`hist`] | FIB-2 | the own-past histograms |
//! | [`extremes`], [`base`], [`measure`], [`shape`] | FIB-3 | the bases and their measurements |
//! | [`trigger`], [`plan`] | FIB-4 | the trigger candle and the trade plan |
//!
//! # No look-ahead, by the shape of the fold
//!
//! Every detector here is fed ONE closed [`types::FibCandle`] of one width of
//! one stock-day at a time and keeps its own running state; it is never given
//! the day's slice, so a later candle is not in scope when an earlier one folds
//! (`CLAUDE.md` §3 rule 7). Partial candles are never decision candles.

pub mod base;
pub mod extremes;
pub mod hist;
pub mod measure;
pub mod plan;
pub mod shape;
pub mod trigger;
pub mod types;

/// The vocabulary positions this module emits: none.
pub const POSITIONS: [u16; 0] = [];

#[cfg(test)]
mod tests {
    use vocab::fib_cube::untyped_numbers;

    #[test]
    fn the_module_claims_no_vocabulary_position() {
        assert!(super::POSITIONS.is_empty());
    }

    /// NO TYPED NUMBERS (operator 16 Oct 11:24 IST): every FIB-1 file of this
    /// module holds no numeric literal but 0 and 1 in its shipping part.
    #[test]
    fn every_fib1_file_of_fibflag_types_no_number() {
        for (name, source) in [
            ("mod.rs", include_str!("mod.rs")),
            ("types.rs", include_str!("types.rs")),
            ("hist.rs", include_str!("hist.rs")),
            ("extremes.rs", include_str!("extremes.rs")),
            ("base.rs", include_str!("base.rs")),
            ("measure.rs", include_str!("measure.rs")),
            ("shape.rs", include_str!("shape.rs")),
            ("trigger.rs", include_str!("trigger.rs")),
            ("plan.rs", include_str!("plan.rs")),
        ] {
            let typed = untyped_numbers(source);
            assert!(typed.is_empty(), "{name} types {typed:?}");
        }
    }
}

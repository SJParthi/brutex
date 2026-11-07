//! PREVIEW FIB REFERENCE: an independent second build of the Fibonacci
//! preview's written rules (the FIB-PREVIEW kit of Sat 7 Nov 2026,
//! `reports/fib-preview-kit-20261107/`), written from the kit's words alone,
//! on seeded synthetic data only. It is not a result and says nothing about
//! profit. Its outputs are golden scenarios a separate build must match:
//! agreement between two builds is evidence, a disagreement is a bug in one
//! of them.
//!
//! Exact integers throughout: prices and money in paise (`i64`, sums
//! `i128`), rung prices in milli-paise, ratios cross-multiplied, no float on
//! any decision path.

pub mod account;
pub mod bars;
pub mod charges;
pub mod clock;
pub mod e1;
pub mod e2;
pub mod e3;
pub mod gates;
pub mod golden;
pub mod golden_format;
pub mod ladder;
pub mod model;
pub mod money;
pub mod rank;
pub mod synth;
pub mod walk;
pub mod wf;

/// The label every output of this crate carries.
pub const PREVIEW: &str = "PREVIEW: an independent reference build on seeded synthetic data; not a result, says nothing about profit";

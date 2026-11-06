//! `psrb_preview_ref`: an independent reference model of the PSRB preview backtest of
//! Fri 6 Nov 2026 (`reports/psrb-preview-20261106/`), written from the kit's text alone,
//! for differential testing of the Mac tool.
//!
//! **Synthetic data only.** Every price and trade this crate sees is generated with a
//! fixed seed ([`generator`]); it reads no store, no vendor row and no credential.
//! **Exact integers only**: money in paisa, percent in basis points, ratios in permille,
//! `i128` where products and sums grow; no floating point anywhere, no `as` cast, no
//! indexing. **Independent**: it depends on no workspace crate, so nothing it computes
//! shares code with the tool it checks.
//!
//! The recipe, function by function as TEST-PLAN.md names them:
//!
//! | TEST-PLAN | here |
//! |---|---|
//! | F01 | [`universe::load`] |
//! | F02, F03 | [`membership::probe`], [`membership::Membership::may_trade`] |
//! | F04, F05 | [`span::trading_span`], [`span::warmup_start`] |
//! | F06 | [`daymask::admit`] |
//! | F07..F10 | [`candles::fold_day`], [`candles::close_instant`], [`candles::auction_eligible`] |
//! | F11 | [`dayref::PrevSession`], [`dayref::day_bar`] |
//! | F12..F19 | [`readings::Bank::step`], [`indicators`], [`readings::Taint`] |
//! | F20..F27 | [`cond`] |
//! | F28, F29 | [`signal::ready_gate`], [`signal::evaluate`] |
//! | F30, F34..F38 | [`fills`] |
//! | F31, F32 | [`pricemap::ratio_source`], [`pricemap::map_level`] |
//! | F33 | [`trail::step`] |
//! | F39 | [`trade::simulate`] |
//! | F40 | [`sizing::qty`] |
//! | F41..F52 | [`costs`] |
//! | F53, F54 | [`book::merge`], [`book::Slots`] |
//! | F55, F56 | [`account::MonthAgg`], [`account::Drawdown`] |
//! | F57, F60, F61 | [`report::row_stats`], [`report::head_compare`], [`money::rupees`] |
//! | F58, F59 | [`verdict::bar`], [`verdict::rank`] |
//!
//! [`run`] composes them (Phase A per stock and timeframe, Phase B per row), [`golden`]
//! writes the inputs and every expected number as plain-text tables, and [`benchkit`]
//! holds the ratio bench's measuring functions.

#![deny(clippy::as_conversions)]

pub mod account;
pub mod benchkit;
pub mod book;
pub mod candles;
pub mod catalogue;
pub mod cond;
pub mod costs;
pub mod daymask;
pub mod dayref;
pub mod fills;
pub mod generator;
pub mod golden;
pub mod indicators;
pub mod market;
pub mod membership;
pub mod money;
pub mod pricemap;
pub mod readings;
pub mod report;
pub mod rng;
pub mod ruleset;
pub mod run;
pub mod signal;
pub mod sizing;
pub mod span;
pub mod time;
pub mod trade;
pub mod trail;
pub mod universe;
pub mod verdict;

//! C-CHK-07 (CC-20, FM-615, D-6800): `api::nse_identity::Reference::confirm`,
//! the identity check of one F&O share under `zerodha_nse_checked`: the
//! member's ISIN from the compile-time universe table, one probe of the
//! merged Zerodha index, then one probe of NSE's decoded file. O(1) per
//! symbol, so its cost must be the same with NSE's file and Zerodha's listing
//! at 100, 1,000 and 10,000 rows. 10,000 is `pull::nse::MAX_CONSTITUENTS`,
//! the most rows the decoder admits, so the 100x size is the maximum input
//! itself, and one row past it is refused before anything is timed.
//!
//! Two answers are timed: a hit (a share both lists hold, its token
//! returned) and the deepest refusal (`SymbolAbsent`: a share Zerodha lists
//! and NSE's file does not, which passes both earlier probes first). Every
//! other refusal is asked once, before timing, and must be the one named.
//!
//! The probed shares are the universe's own names, read from
//! `brutex_core::universe` at run time with the ISIN that table carries, so a
//! hit is possible; every company name, industry, padding symbol and token is
//! invented, and no row of NSE's real file is copied. The harness,
//! `crates/api/benches/ratio.rs`, fails on a ratio above its ceiling and
//! prints p50, p99 and max beside each.

use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

use api::merge::{Merged, Source, merge};
use api::nse_identity::{Reference, Refusal};
use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::universe::{FNO_INDEX_UNDERLYINGS, FNO_UNDERLYINGS, nse_isin};
use brutex_core::vendor::{Listing, Vendor, VendorId};
use pull::session::Day;

use super::{RATIO_CEILING_PERMILLE, cost_ps, ratio, refuse, spread_ns};

/// Rows at 1x; 10x and 100x hold ten and a hundred times as many.
const BASE_ROWS: usize = 100;
/// Shares probed at every size.
const PROBED: usize = 40;
/// NSE's header, as `pull::nse::constituents_strict` reads it.
const HEAD: &str = "Company Name,Industry,Symbol,Series,ISIN Code\n";

/// The universe's shares that carry an NSE ISIN: the first [`PROBED`] are
/// hits, the next is the share Zerodha lists and NSE's file leaves out.
fn shares() -> Vec<(&'static str, String)> {
    let found: Vec<(&'static str, String)> = FNO_UNDERLYINGS
        .iter()
        .copied()
        .filter(|symbol| !FNO_INDEX_UNDERLYINGS.contains(symbol))
        .filter_map(|symbol| nse_isin(symbol).map(|isin| (symbol, isin.as_str().to_owned())))
        .take(PROBED + 2)
        .collect();
    if found.len() < PROBED + 2 {
        refuse("enough universe shares carrying an NSE ISIN");
    }
    found
}

/// The landing day of both files.
fn landed() -> Day {
    Day::new(2026, 10, 6).unwrap_or_else(|_| refuse("a real day"))
}

/// NSE's file of `rows` rows: the probed shares with the universe's ISIN,
/// then invented padding. `wrong` is a share printed with another share's
/// ISIN, a well-formed one that is not its own.
fn body(rows: usize, shares: &[(&str, String)], wrong: &str) -> String {
    let mut text = String::from(HEAD);
    for (symbol, isin) in shares.iter().take(PROBED) {
        let _infallible = writeln!(text, "Invented Co,Invented,{symbol},EQ,{isin}");
    }
    let other = shares
        .first()
        .map_or_else(|| refuse("a probed share"), |(_, isin)| isin.as_str());
    let _infallible = writeln!(text, "Invented Co,Invented,{wrong},EQ,{other}");
    for i in 0..rows.saturating_sub(PROBED + 1) {
        let _infallible = writeln!(text, "Invented Pad {i},Invented,ZZQ{i:06},EQ,ZZ{i:010}");
    }
    text
}

/// Zerodha's listing of `rows` rows: every probed share, the share NSE's file
/// leaves out and the wrongly printed share (all of `shares`), then invented
/// padding.
fn zerodha(rows: usize, shares: &[(&str, String)]) -> Merged {
    let listing = |symbol: &str, token: usize| {
        let (Some(vendor_id), Ok(key)) = (
            VendorId::new(&format!("{}", 100_000 + token)),
            InstrumentKey::cash(Exchange::Nse, symbol),
        ) else {
            refuse("an invented Zerodha row")
        };
        Listing {
            vendor_id,
            key,
            isin: None,
            unsuffixed: None,
        }
    };
    let mut kept: Vec<Listing> = shares
        .iter()
        .enumerate()
        .map(|(token, (symbol, _))| listing(symbol, token))
        .collect();
    let named = kept.len();
    for i in 0..rows.saturating_sub(named) {
        kept.push(listing(&format!("ZZQ{i:06}"), named + i));
    }
    merge(&[Source {
        vendor: Vendor::Zerodha,
        kept,
        declined: Vec::new(),
    }])
}

/// One size's reference and merged listing.
fn fixture(rows: usize, shares: &[(&str, String)]) -> (Reference, Merged) {
    let wrong = shares
        .get(PROBED + 1)
        .map_or_else(|| refuse("a share to print wrongly"), |(s, _)| *s);
    let reference = Reference::from_body(&body(rows, shares, wrong), landed(), landed())
        .unwrap_or_else(|why| refuse(&format!("the invented NSE file: {why}")));
    (reference, zerodha(rows, shares))
}

/// Every answer the fixture must give before anything is timed.
fn check(reference: &Reference, merged: &Merged, shares: &[(&str, String)]) {
    for (symbol, _) in shares.iter().take(PROBED) {
        if reference.confirm(merged, symbol).is_err() {
            refuse("a share both lists hold confirms");
        }
    }
    let (Some((absent, _)), Some((wrong, _))) = (shares.get(PROBED), shares.get(PROBED + 1))
    else {
        refuse("the two refused shares")
    };
    let refusals = [
        (*absent, "symbol absent"),
        (*wrong, "isin mismatch"),
        ("NIFTY", "no member isin"),
    ];
    for (symbol, case) in refusals {
        let got = reference.confirm(merged, symbol);
        let right = match case {
            "symbol absent" => got == Err(Refusal::SymbolAbsent),
            "isin mismatch" => matches!(got, Err(Refusal::IsinMismatch { .. })),
            _ => got == Err(Refusal::NoMemberIsin),
        };
        if !right {
            refuse(&format!("confirm's {case} refusal: {got:?}"));
        }
    }
}

/// The extremes: one row past the decoder's cap, a stale pair of files, and
/// a file not read at all.
fn the_extremes_refuse(shares: &[(&str, String)]) {
    let wrong = shares
        .get(PROBED + 1)
        .map_or_else(|| refuse("a share to print wrongly"), |(s, _)| *s);
    let past = body(pull::nse::MAX_CONSTITUENTS + 1, shares, wrong);
    if Reference::from_body(&past, landed(), landed()).is_ok() {
        refuse("one row past MAX_CONSTITUENTS is refused");
    }
    let merged = zerodha(BASE_ROWS, shares);
    let (first, _) = shares
        .first()
        .unwrap_or_else(|| refuse("a probed share"));
    let earlier = Day::new(2026, 10, 5).unwrap_or_else(|_| refuse("a real day"));
    let stale = Reference::from_body(&body(BASE_ROWS, shares, wrong), earlier, landed())
        .unwrap_or_else(|why| refuse(&why));
    if !matches!(stale.confirm(&merged, first), Err(Refusal::FileStale { .. })) {
        refuse("files landed on two days refuse as stale");
    }
    if !matches!(
        Reference::not_read().confirm(&merged, first),
        Err(Refusal::FileUnavailable(_))
    ) {
        refuse("a file not read refuses as unavailable");
    }
}

/// C-CHK-07: one confirm, a hit and the deepest refusal, at 1x, 10x and 100x
/// the rows.
pub(crate) fn confirm_is_flat() -> bool {
    let shares = shares();
    the_extremes_refuse(&shares);
    let built: Vec<(Reference, Merged)> = [BASE_ROWS, BASE_ROWS * 10, BASE_ROWS * 100]
        .iter()
        .map(|&rows| fixture(rows, &shares))
        .collect();
    for (reference, merged) in &built {
        check(reference, merged, &shares);
    }
    let Some((absent, _)) = shares.get(PROBED) else {
        refuse("the share NSE's file leaves out")
    };
    let mut ok = true;
    for (label, hit) in [("hit", true), ("symbol absent", false)] {
        let mut costs = Vec::with_capacity(built.len());
        for (reference, merged) in &built {
            let mut at = 0usize;
            costs.push(cost_ps(20_000, || {
                let symbol = if hit {
                    shares.get(at % PROBED).map_or("", |(s, _)| *s)
                } else {
                    absent
                };
                at += 1;
                black_box(reference).confirm(black_box(merged), black_box(symbol))
            }));
        }
        let (Some(&base), Some(&mid), Some(&big)) = (costs.first(), costs.get(1), costs.get(2))
        else {
            refuse("three sizes measured")
        };
        ok &= ratio(
            &format!("C-CHK-07 confirm {label}, 10x rows"),
            base,
            mid,
            RATIO_CEILING_PERMILLE,
        );
        ok &= ratio(
            &format!("C-CHK-07 confirm {label}, 100x rows"),
            base,
            big,
            RATIO_CEILING_PERMILLE,
        );
    }
    let Some((reference, merged)) = built.last() else {
        refuse("the largest fixture")
    };
    let mut times: Vec<u128> = (0..20_000)
        .map(|i| {
            let symbol = shares.get(i % PROBED).map_or("", |(s, _)| *s);
            let start = Instant::now();
            black_box(reference.confirm(merged, black_box(symbol)).is_ok());
            start.elapsed().as_nanos()
        })
        .collect();
    let n = times.len();
    let (p50, p99, max) = spread_ns(&mut times);
    println!(
        "  C-CHK-07 confirm one share, 10000 rows: p50 {p50} ns p99 {p99} ns max {max} ns n={n}"
    );
    ok
}

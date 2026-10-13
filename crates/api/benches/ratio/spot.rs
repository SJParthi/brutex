//! Gate 8 for `/spot.json`'s probe: the underlying's close at one instant costs
//! the same whatever the month behind it holds.
//!
//! `api::server::spot::spot_at` is the call the route makes per instant, and
//! `/greeks.json` and `/chain.json` make it too. Timed on an instant that
//! holds a bar, on one between two bars, and -- under D-3328's `asof=prev` --
//! on an on-grid second with no bar of its own, answered with the bar before
//! it, over one one-second month of 1,024, 10,240 and 102,400 bars on disk
//! (`MONTH_SIZES`: the smallest month the store gives a `.tix`, D-4933, and
//! 10x and 100x of it), opened once.
//!
//! The shared fixture month (`month_file`) holds every second from its start,
//! so it has no on-grid second without a bar in its middle; the as-of row is
//! timed on a month of its own built here the same way with ONE second left
//! out, the second after the held instant, so the lookup lands mid-file
//! exactly as a real gap does. The half second the exact rows ask is off the
//! one-second grid, which `asof=prev` refuses (`t_off_grid`): the bar before
//! an off-grid instant can close after it.
//!
//! WP-32 owns this file. The chain's own per-instant pass is O(strikes of one
//! expiry), not a function of the month's size, so it has no row here; its
//! route is timed on the shadow store in WP-32's report.

use std::hint::black_box;

use api::server::spot::{AsOf, Spot, spot_at};
use brutex_core::vendor::Vendor;

use super::{
    MONTH_SIZES, RATIO_CEILING_PERMILLE, bench_month, cost_ps, month_file, ratio, refuse, second,
};

/// One one-second month of `n` bars from the month's start with the second
/// `gap` left out, on disk under its own root, opened for reading; the root
/// is returned so the caller removes it. `month_file`'s shape otherwise: GDFL,
/// NSE INDEX NIFTY, June 2024, one append.
fn gapped_month_file(n: usize, gap: usize) -> (store::file::BarFile, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "brutex-api-bench-spot-gap-{n}-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(why) = std::fs::create_dir_all(&root) {
        refuse(&format!("the bench store root would not create: {why}"));
    }
    let Ok(path) = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: store::path::Timeframe::SECOND_1,
        month: bench_month(),
        file: store::path::FileKind::Bars,
    }) else {
        refuse("the bench path is a legal one")
    };
    // The id `month_file` stamps: the low 32 bits of the symbol's FNV-1a.
    let Ok(symbol_id) = u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX))
    else {
        refuse("the low 32 bits of a hash fit 32 bits")
    };
    let mut file = match store::file::BarFile::open_or_create(&root, path, symbol_id) {
        Ok(file) => file,
        Err(why) => refuse(&format!("the bench month would not open: {why}")),
    };
    let batch: Vec<store::format::Bar> = (0..=n)
        .filter(|&i| i != gap)
        .take(n)
        .map(|i| {
            let raw = i64::try_from(i).unwrap_or(0);
            store::format::Bar {
                ts_micros: second(i),
                open: 2_000_000 + raw,
                high: 2_000_100 + raw,
                low: 1_999_900 + raw,
                close: 2_000_050 + raw,
                volume: 1_000,
                open_interest: store::format::OI_NULL,
            }
        })
        .collect();
    if let Err(why) = file.append(&batch) {
        refuse(&format!("the bench month would not fill: {why}"));
    }
    // THE INDEXED PATH AT EVERY SIZE (D-4933), as `month_file` holds it.
    let lookup = file.time_lookup();
    if lookup != store::file::TimeLookup::Indexed {
        refuse(&format!(
            "a {n}-bar gapped bench month with its .tix (found {lookup:?})"
        ));
    }
    (file, root)
}

/// The probe at a held instant, between two, and under `asof=prev` at a
/// missing second with the bar before it, at every size, and whether each
/// stayed flat between 1x and 100x. Run by `crates/api/benches/ratio.rs`'s
/// `main`.
pub(crate) fn probes_are_flat() -> bool {
    let month = bench_month();
    // The middle bar of the smallest month, which every month holds, half a
    // second after it, which none does, and the second after it, which the
    // gapped months leave out.
    let middle = MONTH_SIZES[0] / 2;
    let held = second(middle);
    let between = held + 500_000;
    let missing = second(middle + 1);
    let plain: Vec<_> = MONTH_SIZES.iter().map(|&n| month_file("spot", n)).collect();
    let gapped: Vec<_> = MONTH_SIZES
        .iter()
        .map(|&n| gapped_month_file(n, middle + 1))
        .collect();
    let mut ok = true;
    for (label, months, at, asof) in [
        ("held", &plain, held, AsOf::Exact),
        ("between", &plain, between, AsOf::Exact),
        ("as-of prev", &gapped, missing, AsOf::Prev),
    ] {
        let mut costs = Vec::with_capacity(months.len());
        for (file, _root) in months {
            match (spot_at(file, month, at, asof), asof) {
                (Ok(Spot::At { ts_micros, .. }), _) if ts_micros == held => {}
                (Ok(Spot::NoBar), AsOf::Exact) if at == between => {}
                (other, _) => refuse(&format!("the {label} probe answered {other:?}")),
            }
            costs.push(cost_ps(20_000, || {
                spot_at(file, month, black_box(at), asof).is_ok()
            }));
        }
        let (Some(&base), Some(&mid), Some(&big)) = (costs.first(), costs.get(1), costs.get(2))
        else {
            refuse("three sizes measured");
        };
        ok &= ratio(
            &format!("spot probe {label} {}->{}", MONTH_SIZES[0], MONTH_SIZES[1]),
            base,
            mid,
            RATIO_CEILING_PERMILLE,
        );
        ok &= ratio(
            &format!("spot probe {label} {}->{}", MONTH_SIZES[0], MONTH_SIZES[2]),
            base,
            big,
            RATIO_CEILING_PERMILLE,
        );
    }
    for (file, root) in plain.into_iter().chain(gapped) {
        drop(file);
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
}

//! Gate 8 for `/spot.json`'s probe: the underlying's close at one instant costs
//! the same whatever the month behind it holds.
//!
//! `api::server::spot::close_at` is the call the route makes per instant. Timed
//! on an instant that holds a bar and on one between two bars, over one
//! one-second month of 1,000, 10,000 and 100,000 bars on disk, opened once.
//!
//! WP-32 owns this file from here and adds its as-of and chain rows to it.

use std::hint::black_box;

use api::server::spot::{Spot, close_at};

use super::{MONTH_SIZES, RATIO_CEILING_PERMILLE, cost_ps, month_file, ratio, refuse, second};

/// The probe at a held instant and between two, at every size, and whether
/// both stayed flat between 1x and 100x. Run by `crates/api/benches/ratio.rs`'s
/// `main`.
pub(crate) fn probes_are_flat() -> bool {
    let months: Vec<_> = MONTH_SIZES.iter().map(|&n| month_file("spot", n)).collect();
    // The middle bar of the smallest month, which every month holds, and half a
    // second after it, which none does.
    let held = second(MONTH_SIZES[0] / 2);
    let between = held + 500_000;
    let mut ok = true;
    for (label, at) in [("held", held), ("between", between)] {
        let mut costs = Vec::with_capacity(months.len());
        for (file, _root) in &months {
            match close_at(file, at) {
                Ok(Spot::At(_)) if at == held => {}
                Ok(Spot::NoBar) if at == between => {}
                other => refuse(&format!("the probe answered {other:?}")),
            }
            costs.push(cost_ps(20_000, || close_at(file, black_box(at)).is_ok()));
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
    for (file, root) in months {
        drop(file);
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
}

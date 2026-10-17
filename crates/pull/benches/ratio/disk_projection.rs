//! C-DP-01 and C-DP-02 (R4-07, PERF-13, E2E-01): one projected unit over
//! cached totals at 1x, 10x and 100x the measured underlyings, and the
//! Dropbox fetch's status read (one journal tail) at 1x, 10x and 100x the
//! journal's lines. Both must be flat: a projection is one hash probe or one
//! cached median per unit, and the status read seeks to the journal's last
//! 64 KiB whatever its length.
//!
//! Every input is invented here; the journals are written into a new folder
//! under the system temp directory, printed, and left there (this harness
//! removes no file, R4-08's no-delete rule).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pull::disk_projection::{OptionClass, PlanItem, Sealed, Totals, Unit, project};

use super::{cost_ps, ratio, refuse, spread};

/// Underlyings at 1x; 10x and 100x are ten and a hundred times it.
const BASE_UNDERLYINGS: u32 = 100;
/// Journal lines at 1x.
const BASE_LINES: u32 = 1_000;
/// Calls per timed repetition.
const REPS: u32 = 2_000;

fn totals(n: u32) -> Sealed {
    let mut totals = Totals::new();
    for i in 0..n {
        let class = if i % 5 == 0 {
            OptionClass::Index
        } else {
            OptionClass::Stock
        };
        let name = format!("INVENTED{i:06}");
        if totals
            .add_underlying(&name, class, 1_000 + u64::from(i), 10)
            .is_err()
        {
            refuse("an invented underlying was refused");
        }
    }
    totals.seal()
}

fn plan() -> [PlanItem; 2] {
    [
        PlanItem {
            unit: Unit::Underlying {
                name: "INVENTED000042".to_owned(),
                class: OptionClass::Stock,
            },
            days: 30,
            instruments: 1,
        },
        PlanItem {
            unit: Unit::Underlying {
                name: "NOT-HELD".to_owned(),
                class: OptionClass::Index,
            },
            days: 30,
            instruments: 1,
        },
    ]
}

fn journal(dir: &Path, lines: u32) -> PathBuf {
    let root = dir.join(format!("store-{lines}"));
    let path = pull::gdfl_dropbox::journal_path(&root);
    let Some(parent) = path.parent() else {
        refuse("the journal has no folder")
    };
    if std::fs::create_dir_all(parent).is_err() {
        refuse("the journal folder could not be made");
    }
    let mut text = String::with_capacity(usize::try_from(lines).unwrap_or(0) * 96);
    // Every third line keeps a non-ASCII name as the journal does (an en
    // dash and accents, two and three bytes a character), so the 64 KiB
    // window starts inside a character at some sizes (repair round 1, C4).
    for n in 0..lines {
        let extra = if n % 3 == 0 {
            " \u{2013} kopie \u{e9}t\u{e9}"
        } else {
            ""
        };
        let _ = writeln!(
            text,
            "v=1 listed run=r at_ms={n} name=\"GFDLNFO_TICK_{:08}{extra}.zip\" bytes={n}",
            n % 100_000_000
        );
    }
    if std::fs::write(&path, text).is_err() {
        refuse("the journal could not be written");
    }
    root
}

/// C-DP-01 and C-DP-02, printed beside their p50, p99 and max at n=200.
pub(crate) fn disk_projection_is_flat() -> bool {
    println!("C-DP-01: one projected unit over cached totals (R4-07, PERF-13)");
    let sizes = [
        BASE_UNDERLYINGS,
        BASE_UNDERLYINGS * 10,
        BASE_UNDERLYINGS * 100,
    ];
    let sealed: Vec<Sealed> = sizes.iter().map(|n| totals(*n)).collect();
    let items = plan();
    let mut ok = true;
    let costs: Vec<u128> = sealed
        .iter()
        .map(|t| cost_ps(REPS, || project(t, &items).bytes))
        .collect();
    for (at, size) in sizes.iter().enumerate().skip(1) {
        ok &= ratio(
            &format!("project 2 units, {size} underlyings"),
            costs.first().copied().unwrap_or(0),
            costs.get(at).copied().unwrap_or(0),
        );
    }
    for (t, size) in sealed.iter().zip(sizes) {
        spread(&format!("project at {size} underlyings"), 200, || {
            std::hint::black_box(project(t, &items).bytes);
        });
    }

    println!("C-DP-02: the Dropbox fetch's status read, one journal tail (R4-07, E2E-01)");
    let dir = std::env::temp_dir().join(format!(
        "brutex-bench-dropbox-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    println!("  journals under {} (left in place)", dir.display());
    let lines = [BASE_LINES, BASE_LINES * 10, BASE_LINES * 100];
    let roots: Vec<PathBuf> = lines.iter().map(|n| journal(&dir, *n)).collect();
    let read = |root: &PathBuf| match pull::gdfl_dropbox::status(root) {
        Ok(Some(fields)) => fields.verb.len(),
        _ => refuse("the status read found no last line"),
    };
    let costs: Vec<u128> = roots.iter().map(|r| cost_ps(20, || read(r))).collect();
    for (at, n) in lines.iter().enumerate().skip(1) {
        ok &= ratio(
            &format!("status read, {n} journal lines"),
            costs.first().copied().unwrap_or(0),
            costs.get(at).copied().unwrap_or(0),
        );
    }
    for (root, n) in roots.iter().zip(lines) {
        spread(&format!("status read at {n} lines"), 200, || {
            std::hint::black_box(read(root));
        });
    }
    ok
}

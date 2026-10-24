//! C-GR-05 (IA-3b, design §10.1, rules audit RA-103): the import preflight
//! `pull::gdfl_run::plan` runs before any lock. Its fixed checks read the
//! census header (two slot reads), ask one `metadata` per claim lock name,
//! open the kind's lock file for writing without create, ask the volume and
//! the open-file limit once, and read one listing; none of them reads the
//! census's entries or the journal on a store a run has claimed. So the plan
//! costs the same at 1x, 10x and 100x the census's entries and the journal's
//! lines, within the ceiling.
//!
//! Printed beside the ratios, each with p50, p99 and max (n = 200): the first
//! (cold) plan of each size, timed once and printed apart; the repeated plan;
//! the preflight's checks alone (`preflight::check_with`, the host's own
//! answers); and the one listing read the census room needs at the largest
//! options day the design measured, 26,820 entries, which is O(entries) and
//! named in `docs/06-limits.md` (L-IA3B-03), never a ratio.
//!
//! Every byte is invented here: the day files are written by the tick-store
//! writer itself (`pull::tickstore_write::write_day`, R4-08) from day zips of
//! made-up rows, the census is a header over a sparse file, and the journal
//! is lines in its own grammar. Everything lives under the system temp dir.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use brutex_core::instrument::Segment;
use brutex_core::vendor::Vendor;
use pull::gdfl_import::ImportKind;
use pull::gdfl_nfo::{NfoSource as _, NfoTickStore};
use pull::gdfl_run::{Kinds, LogState, Process, Request, SourceCache, SourceWord, plan, preflight};
use pull::manifest::{ENTRY_STRIDE, HEADER_LEN, IMAGE_LEN, ManifestHeader, SPOT_FLOOR, epoch_path};
use pull::session::Day;
use pull::tickstore_write::{Outcome, WriteOptions, write_day};

use super::{cost_ps, ratio, refuse, spread, spread_of};

/// Census entries and journal lines at 1x; 10x and 100x are ten and a
/// hundred times them.
const BASE: u64 = 10_000;
/// Calls per timed repetition of a ratio.
const REPS: u32 = 20;
/// Samples of each spread.
const SAMPLES: u32 = 200;
/// The largest options day the design measured (§2.1): 26,820 files.
const LARGEST_DAY: u32 = 26_820;

/// A stored-only zip of `members`, in order.
fn zip_of(members: &[(String, Vec<u8>)]) -> Vec<u8> {
    let crc = |bytes: &[u8]| {
        let mut c = flate2::Crc::new();
        c.update(bytes);
        c.sum()
    };
    let n16 = |v: usize| u16::try_from(v).unwrap_or_else(|_| refuse("a short zip field"));
    let n32 = |v: usize| u32::try_from(v).unwrap_or_else(|_| refuse("a small zip"));
    let (mut out, mut central) = (Vec::new(), Vec::new());
    for (name, data) in members {
        let offset = n32(out.len());
        let fixed = |made_by: bool| {
            let mut head = Vec::new();
            if made_by {
                head.extend_from_slice(&20_u16.to_le_bytes());
            }
            for half in [20_u16, 0, 0, 0x6000, 0x5B3D] {
                head.extend_from_slice(&half.to_le_bytes());
            }
            head.extend_from_slice(&crc(data).to_le_bytes());
            head.extend_from_slice(&n32(data.len()).to_le_bytes());
            head.extend_from_slice(&n32(data.len()).to_le_bytes());
            head.extend_from_slice(&n16(name.len()).to_le_bytes());
            head.extend_from_slice(&0_u16.to_le_bytes());
            head
        };
        out.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        out.extend_from_slice(&fixed(false));
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        central.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        central.extend_from_slice(&fixed(true));
        central.extend_from_slice(&[0; 10]);
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let at = n32(out.len());
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&n16(members.len()).to_le_bytes());
    out.extend_from_slice(&n16(members.len()).to_le_bytes());
    out.extend_from_slice(&n32(central.len()).to_le_bytes());
    out.extend_from_slice(&at.to_le_bytes());
    out.extend_from_slice(&[0; 2]);
    out
}

/// One invented file's text: the header and `rows` rows of `ticker` on
/// 2026-09-29, at prices no instrument printed.
fn rows_of(ticker: &str, rows: u32) -> Vec<u8> {
    let mut text = String::from("Ticker,Date,Time,LTP,LTQ,OpenInterest\r\n");
    for r in 0..rows {
        let secs = 33_300 + r * 13;
        let _infallible = write!(
            text,
            "{ticker},29/09/2026,{:02}:{:02}:{:02},{}.25,{},{}\r\n",
            secs / 3_600,
            secs / 60 % 60,
            secs % 60,
            900 + r % 17,
            75,
            10_000 + r
        );
    }
    text.into_bytes()
}

/// A day zip named `name` holding `folder/` and `files` (each a name under
/// it and its text), written under `dir` and run through the tick-store
/// writer into `root`.
fn written_day(dir: &Path, root: &Path, name: &str, folder: &str, files: Vec<(String, Vec<u8>)>) {
    let mut members = vec![(format!("{folder}/"), Vec::new())];
    members.extend(files);
    std::fs::create_dir_all(dir).unwrap_or_else(|_| refuse("a zip folder"));
    let zip = dir.join(name);
    std::fs::write(&zip, zip_of(&members)).unwrap_or_else(|_| refuse("a day zip"));
    let run = write_day(&zip, root, &WriteOptions::new("bench-c-gr-05"));
    if !matches!(run.result, Ok(Outcome::Written(_))) {
        refuse(&format!(
            "the tick-store writer refused an invented day: {:?}",
            run.result
        ));
    }
}

/// The day the indices tick store holds.
fn the_day() -> Day {
    Day::new(2026, 9, 29).unwrap_or_else(|_| refuse("a real day"))
}

/// A tick store under `base` with `FORMAT.md` and one indices day.
fn tick_store(base: &Path) -> PathBuf {
    let root = base.join("ts");
    std::fs::create_dir_all(&root).unwrap_or_else(|_| refuse("a tick store"));
    std::fs::write(root.join("FORMAT.md"), b"# invented, for a bench\n")
        .unwrap_or_else(|_| refuse("FORMAT.md"));
    let folder = "GFDLCM_INDICES_TICK_29092026";
    written_day(
        &base.join("zips"),
        &root,
        &format!("{folder}.zip"),
        folder,
        vec![(
            format!("{folder}/INVENTED.NSE_IDX.csv"),
            rows_of("INVENTED.NSE_IDX", 40),
        )],
    );
    root
}

/// A claimed v4 store under `base` whose INDEX census holds `entries` (a
/// header over a sparse file) and whose journal holds `entries / 10` lines.
fn store(base: &Path, entries: u64) -> PathBuf {
    let root = base.join(format!("store-{entries}"));
    let imports = root.join("imports");
    std::fs::create_dir_all(&imports).unwrap_or_else(|_| refuse("a store"));
    std::fs::write(imports.join("gdfl.indices.lock"), b"").unwrap_or_else(|_| refuse("a lock"));
    let mut journal = String::from("run 1700000000 source=zip\n");
    for n in 0..entries / 10 {
        let _infallible = writeln!(journal, "begin indices 2019-01-{:02} INV{n}", 1 + n % 28);
    }
    std::fs::write(imports.join("gdfl.journal"), journal).unwrap_or_else(|_| refuse("a journal"));
    let header = ManifestHeader {
        n_valid: entries,
        n_keys: entries / 2,
        ..ManifestHeader::genesis_partition(Vendor::Gdfl, Segment::Index, SPOT_FLOOR * 2)
            .unwrap_or_else(|_| refuse("a partition header"))
    };
    let path = epoch_path(&root, Vendor::Gdfl, Segment::Index, 1);
    std::fs::create_dir_all(path.parent().unwrap_or(&root)).unwrap_or_else(|_| refuse("a census"));
    let mut bytes = vec![0_u8; usize::try_from(HEADER_LEN).unwrap_or(0)];
    if let Some(slot) = bytes.get_mut(..IMAGE_LEN) {
        slot.copy_from_slice(&header.image());
    }
    std::fs::write(&path, &bytes).unwrap_or_else(|_| refuse("a census file"));
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .and_then(|file| file.set_len(HEADER_LEN + entries * ENTRY_STRIDE))
        .unwrap_or_else(|_| refuse("a sparse census"));
    root
}

/// An options tick store under `base` holding one day of [`LARGEST_DAY`]
/// one-row files.
fn largest_day(base: &Path) -> (PathBuf, Day) {
    let root = base.join("ts-options");
    std::fs::create_dir_all(&root).unwrap_or_else(|_| refuse("an options tick store"));
    let folder = "GFDLNFO_TICK_29092026";
    let files = (0..LARGEST_DAY)
        .map(|i| {
            let ticker = format!("X{i}29SEP2699000CE");
            (
                format!("{folder}/Options/{ticker}.NFO.csv"),
                rows_of(&format!("{ticker}.NFO"), 1),
            )
        })
        .collect();
    written_day(
        &base.join("zips-options"),
        &root,
        &format!("{folder}.zip"),
        folder,
        files,
    );
    (root, the_day())
}

/// C-GR-05: the plan's preflight, flat at 1x, 10x and 100x the census and
/// the journal, with its spreads and the largest day's listing read.
pub(crate) fn preflight_is_flat() -> bool {
    let base = std::env::temp_dir().join(format!("brutex-bench-c-gr-05-{}", std::process::id()));
    let _fresh = std::fs::remove_dir_all(&base);
    let ts = tick_store(&base);
    let env = move |var: &str| -> Option<OsString> {
        (var == pull::gdfl_run::TICKSTORE_VAR).then(|| ts.clone().into_os_string())
    };
    let log = LogState::Installed(base.join("logs"));
    let day = the_day();
    let request = || Request {
        free_floor: 0,
        ..Request::new(
            Kinds::one(ImportKind::Indices),
            SourceWord::TickStore,
            (day, day),
        )
    };
    let stores = [BASE, BASE * 10, BASE * 100].map(|n| (n, store(&base, n)));
    let mut plan_ps = Vec::new();
    for (n, root) in &stores {
        let process = Process {
            store_root: root,
            commit: None,
            log: &log,
            unstamped: None,
        };
        let cold = Instant::now();
        let first = plan(request(), &process, &env, &mut SourceCache::default())
            .unwrap_or_else(|why| refuse(&format!("the plan refused: {why}")));
        println!(
            "  C-GR-05 first (cold) plan, {n} entries        {} ns, n=1",
            cold.elapsed().as_nanos()
        );
        spread(
            &format!("C-GR-05 repeated plan, {n} entries"),
            SAMPLES,
            || {
                let _planned = std::hint::black_box(plan(
                    request(),
                    &process,
                    &env,
                    &mut SourceCache::default(),
                ));
            },
        );
        let checks = || {
            preflight::check_with(&first.request, &process, &first.kinds, &preflight::Os).is_ok()
        };
        spread(
            &format!("C-GR-05 the checks alone, {n} entries"),
            SAMPLES,
            || {
                let _checked = std::hint::black_box(checks());
            },
        );
        plan_ps.push(cost_ps(REPS, checks));
    }
    let (Some(&one), Some(&ten), Some(&hundred)) =
        (plan_ps.first(), plan_ps.get(1), plan_ps.get(2))
    else {
        refuse("three sizes measured");
    };
    let a = ratio("C-GR-05 preflight checks, 10x census", one, ten);
    let b = ratio("C-GR-05 preflight checks, 100x census", one, hundred);
    spreads(&base, &request, &env, &log);
    let _cleanup = std::fs::remove_dir_all(&base);
    a && b
}

/// C-GR-05's three spreads beside its ratios: the one listing read at the
/// largest day, the per-read `source_changing` check, and the legacy
/// cutover's refusal path over a journal of 100,000 lines.
fn spreads(
    base: &Path,
    request: &dyn Fn() -> Request,
    env: &dyn Fn(&str) -> Option<OsString>,
    log: &LogState,
) {
    let (options, day) = largest_day(base);
    let reader = NfoTickStore::new(&options);
    let ns: Vec<u128> = (0..SAMPLES)
        .map(|_| {
            let start = Instant::now();
            let listed = reader.day(day);
            let took = start.elapsed().as_nanos();
            if !matches!(&listed, Ok(Some(listing)) if listing.entries().len() >= 26_820) {
                refuse("the largest invented day did not list its entries");
            }
            took
        })
        .collect();
    spread_of("C-GR-05 the one listing read, 26,820 entries", ns);
    // The per-read cost item 4 adds to every listing and member read: one
    // `fstat` of the handle and one `stat` of the path.
    let watched = options.join("FORMAT.md");
    let _written = std::fs::write(&watched, b"watched\n");
    let handle = std::fs::File::open(&watched)
        .map_or_else(|_| refuse("a watched file"), std::sync::Arc::new);
    let watch = preflight::Watch::new(&watched, handle).unwrap_or_else(|_| refuse("a watch"));
    spread(
        "C-GR-05 the source_changing check, two stats",
        SAMPLES * 10,
        || {
            if watch.check().is_err() {
                refuse("an unchanged file read as changed");
            }
        },
    );
    // The legacy cutover's refusal path: a store no lock file ever held,
    // whose journal of 100,000 lines is read once (O(lines), the only
    // journal read the preflight makes).
    let unclaimed = store(base, BASE * 100);
    let _unclaimed = std::fs::remove_file(unclaimed.join("imports/gdfl.indices.lock"));
    let process = Process {
        store_root: &unclaimed,
        commit: None,
        log,
        unstamped: None,
    };
    spread(
        "C-GR-05 the legacy refusal path, 100,000 lines",
        SAMPLES / 10,
        || {
            if plan(request(), &process, env, &mut SourceCache::default()).is_ok() {
                refuse("an unclaimed store with open keys planned");
            }
        },
    );
}

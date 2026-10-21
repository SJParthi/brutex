//! `cli fno-membership`: the verb end to end over an invented tick store, and
//! every refusal of its arguments. D-5160 to D-5165 (PIT-1).
//!
//! The tick store here is written by this file: day files holding invented
//! option names (`ZZ...`, not NSE symbols) whose contents are one byte. No
//! vendor row is quoted.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::command_with;
use crate::{FAILED, MISUSED, OK};

/// One zstd frame holding `bytes` as raw blocks (RFC 8878 §3.1.1): any
/// decoder reads it, and this file needs no compressor.
fn raw_zstd(bytes: &[u8]) -> Vec<u8> {
    let mut out = vec![0x28, 0xB5, 0x2F, 0xFD];
    // Single segment, a two-byte content size stating `len - 256`, or a
    // one-byte one below 256.
    if bytes.len() < 256 {
        out.push(0x20);
        out.push(u8::try_from(bytes.len()).unwrap());
    } else {
        out.push(0x60);
        out.extend_from_slice(&u16::try_from(bytes.len() - 256).unwrap().to_le_bytes());
    }
    let chunks: Vec<&[u8]> = bytes.chunks(100_000).collect();
    for (at, chunk) in chunks.iter().enumerate() {
        let last = u32::from(at + 1 == chunks.len());
        let header = last | (u32::try_from(chunk.len()).unwrap() << 3);
        out.extend_from_slice(&header.to_le_bytes()[..3]);
        out.extend_from_slice(chunk);
    }
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = flate2::Crc::new();
    crc.update(bytes);
    crc.sum()
}

/// A tick-store day file (FORMAT.md v1) of `names`, each a raw block.
fn bts(names: &[String]) -> Vec<u8> {
    let mut out = b"BRTXTS01".to_vec();
    let mut index = Vec::new();
    for name in names {
        let data = b"x";
        let off = u64::try_from(out.len()).unwrap();
        let mut block = vec![2_u8];
        block.extend_from_slice(&raw_zstd(data));
        out.extend_from_slice(&block);
        index.extend_from_slice(&u16::try_from(name.len()).unwrap().to_le_bytes());
        index.extend_from_slice(name.as_bytes());
        index.push(2);
        index.extend_from_slice(&off.to_le_bytes());
        index.extend_from_slice(&u64::try_from(block.len()).unwrap().to_le_bytes());
        index.extend_from_slice(&1_u64.to_le_bytes());
        index.extend_from_slice(&crc32(data).to_le_bytes());
        index.extend_from_slice(&0_u64.to_le_bytes());
        index.extend_from_slice(&0_u32.to_le_bytes());
    }
    let index_off = u64::try_from(out.len()).unwrap();
    let frame = raw_zstd(&index);
    out.extend_from_slice(&frame);
    out.extend_from_slice(&index_off.to_le_bytes());
    out.extend_from_slice(&u64::try_from(frame.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&u64::try_from(index.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&crc32(&index).to_le_bytes());
    out.extend_from_slice(&u32::try_from(names.len()).unwrap().to_le_bytes());
    out.extend_from_slice(b"BRTXTSE1");
    out
}

const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

/// Writes the day file of `(year, month, day)` listing `tickers`.
fn day_file(root: &Path, (year, month, day): (u16, u8, u8), tickers: &[&str]) {
    let folder = format!("GFDLNFO_TICK_{day:02}{month:02}{year:04}");
    let dir = root
        .join("options")
        .join(format!("{year:04}"))
        .join(format!("{}_{year:04}", MONTHS[usize::from(month) - 1]));
    std::fs::create_dir_all(&dir).unwrap();
    let names: Vec<String> = tickers
        .iter()
        .map(|ticker| format!("{folder}/Options/{ticker}.NFO.csv"))
        .collect();
    std::fs::write(dir.join(format!("{folder}.bts")), bts(&names)).unwrap();
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-cli-fno-membership-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The invented tick store: March 2024 is its first month.
fn ticks() -> PathBuf {
    let root = scratch("ticks");
    day_file(
        &root,
        (2024, 3, 4),
        &["ZZALPHA28MAR24500CE", "ZZBETA28MAR24700PE"],
    );
    day_file(
        &root,
        (2024, 3, 5),
        &["ZZBETA28MAR24700PE", "NIFTY07MAR2499999.5CE"],
    );
    day_file(&root, (2024, 3, 28), &["ZZBETA25APR24700PE"]);
    day_file(&root, (2024, 4, 1), &["ZZGAMMA25APR24900CE"]);
    root
}

/// The environment a test hands the verb.
#[derive(Default, Clone)]
struct Env {
    ticks: Option<PathBuf>,
    store: Option<PathBuf>,
    home: Option<PathBuf>,
    allow_unstamped: bool,
}

fn run_with(args: &[&str], env: &Env, commit: Option<&str>) -> (u8, String) {
    let env = env.clone();
    let lookup = move |var: &str| -> Option<OsString> {
        match var {
            "BRUTEX_GDFL_TICKSTORE" => env.ticks.clone().map(PathBuf::into_os_string),
            "BRUTEX_STORE" => env.store.clone().map(PathBuf::into_os_string),
            "HOME" => env.home.clone().map(PathBuf::into_os_string),
            "BRUTEX_ALLOW_UNSTAMPED_WRITE" => env.allow_unstamped.then(|| OsString::from("1")),
            _ => None,
        }
    };
    let mut out = String::new();
    let code = command_with(args, &lookup, commit, &mut out);
    (code, out)
}

fn run(args: &[&str], ticks: Option<&Path>, commit: Option<&str>) -> (u8, String) {
    let env = Env {
        ticks: ticks.map(Path::to_path_buf),
        ..Env::default()
    };
    run_with(args, &env, commit)
}

/// The month files of `dir`, by name.
fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn build_then_show_then_bench_over_an_invented_tick_store() {
    let ticks = ticks();
    let out_dir = scratch("out");
    let dir = out_dir.to_str().unwrap();
    let (code, out) = run(
        &[
            "build",
            "2024-03-01",
            "2024-04-30",
            "--out",
            dir,
            "--workers",
            "2",
        ],
        Some(&ticks),
        Some("c0ffee"),
    );
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("typed window 2024-03-01..=2024-04-30, scratch only"),
        "{out}"
    );
    assert!(out.contains("2024-03 written g000001"), "{out}");
    assert!(out.contains("2024-04 written g000001"), "{out}");
    assert!(
        out.contains("2024-03-06 membership unknown: no_source"),
        "{out}"
    );
    assert!(out.contains("span 2024-03-04..=2024-04-01"), "{out}");
    assert!(out.contains("max_pool 2"), "{out}");
    assert!(out.contains("before the span: 3 days probed"), "{out}");
    assert!(out.contains("year 2024: 4 known sessions"), "{out}");
    // ZZGAMMA is first stored on 1 Apr; its April expiry was first stored on
    // 28 Mar (ZZBETA's), so its membership may have started up to 4 days late.
    assert!(
        out.contains(
            "listing lag (L-PIT-3, an upper bound): 1 series first stored after 2024-03-04, 1 of \
             them later than their first expiry's series start: p50 4 days, p99 4 days, max 4 \
             days"
        ),
        "{out}"
    );
    assert!(
        out.contains("lagged series (name:days): ZZGAMMA:4\n"),
        "{out}"
    );
    assert!(
        out.contains("joined an earlier series by a measured handover, not counted (0): \n"),
        "{out}"
    );
    assert!(out.contains("per month written: n 2, p50 "), "{out}");
    assert_eq!(
        listing(&out_dir),
        ["2024-03-g000001.fnm", "2024-04-g000001.fnm"]
    );

    // The 5th: ZZALPHA is live with no file; NIFTY is not a share.
    let (code, shown) = run(&["show", "2024-03-05", "--out", dir], None, None);
    assert_eq!(code, OK, "{shown}");
    assert!(shown.starts_with("2024-03-05: pool 2 shares"), "{shown}");
    assert!(shown.contains("snapshot): 0:ZZALPHA 1:ZZBETA\n"), "{shown}");
    assert!(shown.contains("no file that day (1): ZZALPHA\n"), "{shown}");
    assert!(
        shown.contains("not in that snapshot (2): ZZALPHA ZZBETA"),
        "{shown}"
    );
    let snapshot = brutex_core::universe::FNO_UNDERLYINGS.len()
        - brutex_core::universe::FNO_INDEX_UNDERLYINGS.len();
    assert!(
        shown.contains(&format!("not a member that day ({snapshot})")),
        "{shown}"
    );
    assert!(shown.contains("leverage unknown"), "{shown}");

    // April's first session: ZZBETA's April contract, first listed on 28 Mar,
    // carries it across the month with no April file of it.
    let (code, shown) = run(&["show", "2024-04-01", "--out", dir], None, None);
    assert_eq!(code, OK, "{shown}");
    assert!(shown.contains("snapshot): 1:ZZBETA 2:ZZGAMMA\n"), "{shown}");

    // A session with no day file, and days outside the span, say why.
    let (code, shown) = run(&["show", "2024-03-06", "--out", dir], None, None);
    assert_eq!(code, FAILED);
    assert!(
        shown.starts_with("membership unknown: 2024-03-06 no_source"),
        "{shown}"
    );
    assert!(shown.contains("refused sessions of 2024 inside the span through 2024-03-06: 1"));
    let (code, shown) = run(&["show", "2024-05-02", "--out", dir], None, None);
    assert_eq!(code, FAILED);
    assert!(
        shown.contains("after the last stored stock-derivative record"),
        "{shown}"
    );
    let (code, shown) = run(&["show", "2024-03-01", "--out", dir], None, None);
    assert_eq!(code, FAILED);
    assert!(
        shown.contains("before the first stored stock-derivative record"),
        "{shown}"
    );

    let (code, timed) = run(&["bench", "--out", dir], None, None);
    assert_eq!(code, OK, "{timed}");
    assert!(timed.contains("members(day): n 4, p50 "), "{timed}");
    assert!(timed.contains("is_member(name, day): n 4, p50 "), "{timed}");
    // Refusals are timed too: every other day of the span (25) and the 400
    // days either side of it.
    assert!(
        timed.contains("members(day) refused (membership unknown): n 825, p50 "),
        "{timed}"
    );
}

#[test]
fn a_resumed_build_extends_only_what_is_new_and_a_rerun_writes_nothing() {
    let ticks = ticks();
    let out_dir = scratch("resume");
    let dir = out_dir.to_str().unwrap();
    let (code, out) = run(
        &["build", "2024-03-01", "2024-03-05", "--out", dir],
        Some(&ticks),
        Some("c0ffee"),
    );
    assert_eq!(code, OK, "{out}");
    assert_eq!(listing(&out_dir), ["2024-03-g000001.fnm"]);
    // Resume: no window typed, to the last day holding a record.
    let (code, out) = run(&["build", "--out", dir], Some(&ticks), Some("c0ffee"));
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("(resume to known_to)"), "{out}");
    assert!(out.contains("2024-03 written g000002"), "{out}");
    assert!(out.contains("2024-04 written g000001"), "{out}");
    assert!(out.contains("span 2024-03-04..=2024-04-01"), "{out}");
    assert!(out.contains("after known_to: "), "{out}");
    let before = listing(&out_dir);
    let (code, out) = run(&["build", "--out", dir], Some(&ticks), Some("c0ffee"));
    assert_eq!(code, OK, "{out}");
    assert!(!out.contains(" written "), "{out}");
    assert_eq!(listing(&out_dir), before);
    // A typed window over a sealed record is refused by name, writing nothing.
    let (code, out) = run(
        &["build", "2024-03-04", "2024-03-31", "--out", dir],
        Some(&ticks),
        Some("c0ffee"),
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("never rebuilt"), "{out}");
    assert_eq!(listing(&out_dir), before);
}

#[test]
fn a_typed_window_inside_the_operators_store_is_refused() {
    let ticks = ticks();
    let store = scratch("store");
    let inside = store.join("reference").join("fno-membership-v1");
    let env = Env {
        ticks: Some(ticks.clone()),
        store: Some(store.clone()),
        ..Env::default()
    };
    let (code, out) = run_with(
        &[
            "build",
            "2024-03-01",
            "2024-03-31",
            "--out",
            inside.to_str().unwrap(),
        ],
        &env,
        Some("c0ffee"),
    );
    assert_eq!(code, MISUSED, "{out}");
    assert!(out.contains("a typed window is for synthetic stores and scratch copies only"));
    assert!(!inside.exists(), "nothing is created");
    // Without BRUTEX_STORE the store is $HOME/.brutex/store.
    let home = scratch("home");
    let inside = home.join(".brutex").join("store").join("reference");
    let env = Env {
        ticks: Some(ticks),
        home: Some(home),
        ..Env::default()
    };
    let (code, out) = run_with(
        &[
            "build",
            "resume",
            "2024-03-31",
            "--out",
            inside.to_str().unwrap(),
        ],
        &env,
        Some("c0ffee"),
    );
    assert_eq!(code, MISUSED, "{out}");
    assert!(!inside.exists());
    // The resumed form is the store's own and is not refused for its place.
    let (code, out) = run_with(
        &["build", "--out", inside.to_str().unwrap()],
        &env,
        Some("c0ffee"),
    );
    assert_eq!(code, OK, "{out}");
    // verify-PIT-1-r1 follow-up: the guard read only the two environment
    // spellings. A directory laid out as a store's membership is refused
    // wherever it is, and so is one inside the `--store ROOT` on the line.
    let laid_out = scratch("elsewhere")
        .join("reference")
        .join("fno-membership-v1");
    let (code, out) = run_with(
        &[
            "build",
            "2024-03-01",
            "2024-03-31",
            "--out",
            laid_out.to_str().unwrap(),
        ],
        &env,
        Some("c0ffee"),
    );
    assert_eq!(code, MISUSED, "{out}");
    assert!(out.contains("laid out as a store's membership"), "{out}");
    assert!(!laid_out.exists());
    let root = store_tree(&[("FNO", &[]), ("CASH", &[])]);
    let under = root.join("scratch-membership");
    let (code, out) = run_with(
        &[
            "build",
            "2024-03-01",
            "2024-03-31",
            "--out",
            under.to_str().unwrap(),
            "--store",
            root.to_str().unwrap(),
        ],
        &env,
        Some("c0ffee"),
    );
    assert_eq!(code, MISUSED, "{out}");
    assert!(out.contains("is inside the store"), "{out}");
    assert!(!under.exists());
}

#[test]
fn every_argument_refusal_is_named_and_writes_nothing() {
    let ticks = ticks();
    let out_dir = scratch("refusals");
    let dir = out_dir.to_str().unwrap();
    let stamp = Some("c0ffee");
    for (args, needle) in [
        (&[][..], "fno-membership takes"),
        (&["build"][..], "--out DIR is required"),
        (
            &["build", "2024-03-01", "2024-03-31"][..],
            "--out DIR is required",
        ),
        (
            &["build", "2024-3-1", "2024-03-31", "--out", dir][..],
            "is not a day",
        ),
        (
            &["build", "--out", dir, "--workers", "15"][..],
            "--workers takes 1..=14",
        ),
        (&["build", "--out", dir, "--fast"][..], "is not a flag"),
        (&["show", "2024-02-30", "--out", dir][..], "is not a day"),
        (&["show", "2024-03-05"][..], "--out DIR is required"),
        (&["bench"][..], "--out DIR is required"),
        (
            &["build", "--out", dir, "--store"][..],
            "--store takes a store root",
        ),
        (
            &["bench", "--out", dir, "--store", dir][..],
            "bench takes no --store",
        ),
    ] {
        let (code, out) = run(args, Some(&ticks), stamp);
        assert_eq!(code, MISUSED, "{args:?}: {out}");
        assert!(out.starts_with("refused: "), "{args:?}: {out}");
        assert!(out.contains(needle), "{args:?}: {out}");
    }
    let (code, out) = run(&["build", "--out", dir], None, stamp);
    assert_eq!(code, MISUSED);
    assert!(out.contains("BRUTEX_GDFL_TICKSTORE is not set"), "{out}");
    let (code, out) = run(&["build", "--out", dir], Some(&ticks), None);
    assert_eq!(code, MISUSED);
    assert!(out.contains("carries no commit stamp"), "{out}");
    let empty = scratch("no-options");
    let (code, out) = run(&["build", "--out", dir], Some(&empty), stamp);
    assert_eq!(code, MISUSED);
    assert!(out.contains("holds no options folder"), "{out}");
    assert!(listing(&out_dir).is_empty(), "no refusal wrote a file");
    // An unstamped build may write when told to, and says so.
    let env = Env {
        ticks: Some(ticks),
        allow_unstamped: true,
        ..Env::default()
    };
    let (code, out) = run_with(
        &["build", "2024-03-01", "2024-03-05", "--out", dir],
        &env,
        None,
    );
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("commit unstamped"), "{out}");
}

#[test]
fn a_damaged_or_empty_directory_is_refused_by_show_and_bench() {
    // A directory that is not there is named, never served as empty.
    let missing = scratch("gone").join("no-such-dir");
    for args in [
        &["bench", "--out", missing.to_str().unwrap()][..],
        &["show", "2024-03-05", "--out", missing.to_str().unwrap()][..],
    ] {
        let (code, out) = run(args, None, None);
        assert_eq!(code, FAILED, "{args:?}");
        assert!(out.contains("no such membership directory"), "{out}");
    }
    let empty = scratch("empty");
    let dir = empty.to_str().unwrap();
    let (code, out) = run(&["bench", "--out", dir], None, None);
    assert_eq!(code, FAILED);
    assert!(out.contains("holds no record to time"), "{out}");
    let (code, out) = run(&["show", "2024-03-05", "--out", dir], None, None);
    assert_eq!(code, FAILED);
    assert!(out.contains("no record is held"), "{out}");
    std::fs::write(empty.join("2024-03-g000001.fnm"), b"not a month").unwrap();
    for args in [
        &["bench", "--out", dir][..],
        &["show", "2024-03-05", "--out", dir][..],
    ] {
        let (code, out) = run(args, None, None);
        assert_eq!(code, FAILED, "{args:?}");
        assert!(out.starts_with("refused: "), "{out}");
        assert!(out.contains("2024-03-g000001.fnm"), "{out}");
    }
}

/// An invented store tree: `bars/gdfl/NSE/<segment>/<name>` folders only.
fn store_tree(segments: &[(&str, &[&str])]) -> PathBuf {
    let store = scratch("cross-store");
    let tree = store.join("bars").join("gdfl").join("NSE");
    for (segment, names) in segments {
        std::fs::create_dir_all(tree.join(segment)).unwrap();
        for name in *names {
            std::fs::create_dir_all(tree.join(segment).join(name)).unwrap();
        }
    }
    store
}

#[test]
fn the_census_is_read_from_the_store_as_a_cross_check_never_a_constant() {
    let ticks = ticks();
    let out_dir = scratch("cross");
    let dir = out_dir.to_str().unwrap();
    // ZZOMEGA's options are stored and no listing of the tick store names
    // it; NIFTY is an index; ZZALPHA has no share folder.
    let store = store_tree(&[
        ("FNO", &["ZZALPHA", "ZZBETA", "ZZOMEGA", "NIFTY"]),
        ("CASH", &["ZZBETA", "ZZGAMMA"]),
    ]);
    let cash = store.join("bars").join("gdfl").join("NSE").join("CASH");
    std::fs::write(cash.join("README"), b"a file, not a series").unwrap();
    let root = store.to_str().unwrap();
    let (code, out) = run(
        &[
            "build",
            "2024-03-01",
            "2024-04-30",
            "--out",
            dir,
            "--store",
            root,
        ],
        Some(&ticks),
        Some("c0ffee"),
    );
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains(
            "4 GDFL option-underlying folders, 2 GDFL share-series folders; 3 membership series"
        ),
        "{out}"
    );
    assert!(out.contains("no GDFL share folder (1): ZZALPHA\n"), "{out}");
    assert!(
        out.contains("no membership spelling reaches (1): ZZOMEGA\n"),
        "{out}"
    );
    let (code, shown) = run(
        &["show", "2024-03-05", "--out", dir, "--store", root],
        None,
        None,
    );
    assert_eq!(code, OK, "{shown}");
    assert!(
        shown.contains("under their spelling that day (1): ZZALPHA\n"),
        "{shown}"
    );
    // Read each time: a folder added since is counted, nothing is remembered.
    std::fs::create_dir_all(cash.join("ZZALPHA")).unwrap();
    let (code, shown) = run(
        &["show", "2024-03-05", "--out", dir, "--store", root],
        None,
        None,
    );
    assert_eq!(code, OK, "{shown}");
    assert!(
        shown.contains("under their spelling that day (0): \n"),
        "{shown}"
    );
}

#[test]
fn a_store_root_the_cross_check_cannot_read_is_refused_by_name() {
    let ticks = ticks();
    let out_dir = scratch("cross-refused");
    let dir = out_dir.to_str().unwrap();
    let (code, out) = run(
        &["build", "2024-03-01", "2024-04-30", "--out", dir],
        Some(&ticks),
        Some("c0ffee"),
    );
    assert_eq!(code, OK, "{out}");
    // A root without the GDFL NSE tree is refused before anything is built.
    let bare = scratch("bare");
    let fresh = scratch("cross-fresh");
    let (code, out) = run(
        &[
            "build",
            "--out",
            fresh.to_str().unwrap(),
            "--store",
            bare.to_str().unwrap(),
        ],
        Some(&ticks),
        Some("c0ffee"),
    );
    assert_eq!(code, MISUSED, "{out}");
    assert!(out.contains("is not a directory"), "{out}");
    assert!(listing(&fresh).is_empty(), "nothing was built");
    let (code, out) = run(
        &[
            "show",
            "2024-03-05",
            "--out",
            dir,
            "--store",
            bare.to_str().unwrap(),
        ],
        None,
        None,
    );
    assert_eq!(code, MISUSED, "{out}");
    // A tree whose share folder cannot be listed is refused by name.
    let half = store_tree(&[("FNO", &["ZZBETA"])]);
    let (code, out) = run(
        &[
            "show",
            "2024-03-05",
            "--out",
            dir,
            "--store",
            half.to_str().unwrap(),
        ],
        None,
        None,
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("refused: the cross-check of"), "{out}");
    let (code, out) = run(
        &[
            "build",
            "--out",
            fresh.to_str().unwrap(),
            "--store",
            half.to_str().unwrap(),
        ],
        Some(&ticks),
        Some("c0ffee"),
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("refused: the cross-check of"), "{out}");
    assert!(
        !listing(&fresh).is_empty(),
        "the build itself completed and says so"
    );
}

#[test]
fn a_spelling_joined_by_a_handover_is_no_listing_lag() {
    // ZZOLDCO's three keys move to ZZNEWCO on Wed 6 Mar 2024 (a measured
    // handover: one series). ZZNEWCO is first stored on the 6th, two days after
    // its March expiry's series started on the 4th, but its series was a
    // member under ZZOLDCO all along: it is named apart, never counted as a
    // listing lag. ZZDELTA, a share first stored on the 7th, IS counted.
    let root = scratch("handover-ticks");
    let keys = |share: &str| {
        ["28MAR24500CE", "28MAR24520CE", "28MAR24480PE"]
            .iter()
            .map(|key| format!("{share}{key}"))
            .collect::<Vec<_>>()
    };
    let (old, new) = (keys("ZZOLDCO"), keys("ZZNEWCO"));
    day_file(&root, (2024, 3, 4), &refs(&old));
    day_file(&root, (2024, 3, 5), &refs(&old));
    day_file(&root, (2024, 3, 6), &refs(&new));
    let mut seventh = new.clone();
    seventh.push("ZZDELTA28MAR24900CE".to_owned());
    day_file(&root, (2024, 3, 7), &refs(&seventh));
    let out_dir = scratch("handover-out");
    let (code, out) = run(
        &[
            "build",
            "2024-03-01",
            "2024-03-31",
            "--out",
            out_dir.to_str().unwrap(),
        ],
        Some(&root),
        Some("c0ffee"),
    );
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("2024-03-06 handover (measured): ZZOLDCO -> ZZNEWCO (3 of 3 keys)"),
        "{out}"
    );
    assert!(
        out.contains(
            "listing lag (L-PIT-3, an upper bound): 1 series first stored after 2024-03-04, 1 of \
             them later than their first expiry's series start: p50 3 days, p99 3 days, max 3 \
             days"
        ),
        "{out}"
    );
    assert!(
        out.contains("lagged series (name:days): ZZDELTA:3\n"),
        "{out}"
    );
    assert!(
        out.contains("joined an earlier series by a measured handover, not counted (1): ZZNEWCO\n"),
        "{out}"
    );
}

/// The names as `&str`, for `day_file`.
fn refs(names: &[String]) -> Vec<&str> {
    names.iter().map(String::as_str).collect()
}

#[test]
fn a_stray_file_and_a_handover_candidate_not_joined_are_named_in_the_month_report() {
    // D-5166 (verify-PIT-1-r1 blocking 4): ZZOLDCO's three keys move to
    // ZZNEWCO on Wed 6 Mar 2024, a measured handover; ZZOLDCO lists one file
    // again on Thu 7 Mar, a stray filed under ZZNEWCO, never a stop. ZZILLIQ's
    // one key is listed on the 6th by the new ZZNEWBIE beside three of its
    // own: a minority of the new spelling's keys, not joined.
    let root = scratch("stray-ticks");
    let keys = |share: &str| {
        ["28MAR24500CE", "28MAR24520CE", "28MAR24480PE"].map(|key| format!("{share}{key}"))
    };
    let (old, new) = (keys("ZZOLDCO"), keys("ZZNEWCO"));
    let mut fifth: Vec<&str> = old.iter().map(String::as_str).collect();
    fifth.push("ZZILLIQ28MAR24900CE");
    day_file(&root, (2024, 3, 5), &fifth);
    let mut sixth: Vec<&str> = new.iter().map(String::as_str).collect();
    sixth.extend([
        "ZZNEWBIE28MAR24900CE",
        "ZZNEWBIE28MAR24910CE",
        "ZZNEWBIE28MAR24920CE",
        "ZZNEWBIE25APR24900CE",
    ]);
    day_file(&root, (2024, 3, 6), &sixth);
    day_file(&root, (2024, 3, 7), &["ZZOLDCO28MAR24500CE"]);
    let out_dir = scratch("stray-out");
    let dir = out_dir.to_str().unwrap();
    let (code, out) = run(
        &["build", "2024-03-01", "2024-03-31", "--out", dir],
        Some(&root),
        Some("c0ffee"),
    );
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("1 handovers, 1 not joined, 1 stray files"),
        "{out}"
    );
    assert!(
        out.contains(
            "2024-03-06 handover candidate ZZILLIQ -> ZZNEWBIE (1 of 1 old keys, 1 of 4 new keys) \
             not joined: a minority of the new spelling's keys"
        ),
        "{out}"
    );
    assert!(
        out.contains(
            "2024-03-07 stray: ZZOLDCO listed 1 contract key(s) after its series moved on to \
             ZZNEWCO: filed under ZZNEWCO, one member (D-5166)"
        ),
        "{out}"
    );
    // The 7th is served: ZZOLDCO's series once, under ZZNEWCO, beside
    // ZZILLIQ (live to its expiry, no file) and ZZNEWBIE.
    let (code, shown) = run(&["show", "2024-03-07", "--out", dir], None, None);
    assert_eq!(code, OK, "{shown}");
    assert!(shown.starts_with("2024-03-07: pool 3 shares"), "{shown}");
    assert!(
        shown.contains("snapshot): 0:ZZILLIQ 1:ZZNEWCO 2:ZZNEWBIE\n"),
        "{shown}"
    );
}

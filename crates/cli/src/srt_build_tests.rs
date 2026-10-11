//! `cli srt-build` on invented stores: misuse, the receipt, the exit, a
//! rerun that changes nothing, the lock, a month not held, `*` over contract
//! directories, and `--dry-run`. D-3349, WP-39; DB-N14.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{USAGE, command_with};
use crate::{FAILED, MISUSED, OK};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-srt-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn run(&self, arguments: &[&str]) -> (u8, String) {
        let mut out = String::new();
        let root = self.0.clone();
        let code = command_with(arguments, &move || Ok(root.clone()), &mut out);
        (code, out)
    }

    fn file(&self, contract: Option<&str>, month: YearMonth, kind: FileKind) -> PathBuf {
        path(contract, month, kind).to_path_buf(&self.0)
    }

    /// Writes an invented month in `appends` appends, then removes its
    /// `.srt` when `bare` — a month written before D-3349.
    fn month(&self, contract: Option<&str>, month: YearMonth, appends: usize, bare: bool) {
        let symbol_id =
            u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)).unwrap();
        let mut writer =
            BarFile::open_or_create(&self.0, path(contract, month, FileKind::Bars), symbol_id)
                .unwrap();
        let open = month.ist_bounds_micros().0 + (86_400 + 555 * 60) * 1_000_000;
        let rows: Vec<Bar> = (0..300_i64)
            .map(|k| Bar {
                ts_micros: open + k * 1_000_000,
                open: 3_210_000 + k % 17,
                high: 3_210_050 + k % 17,
                low: 3_209_950 + k % 17,
                close: 3_210_010 + (k * 7) % 13,
                volume: k % 5,
                open_interest: if contract.is_some() {
                    77 + k % 3
                } else {
                    OI_NULL
                },
            })
            .collect();
        for chunk in rows.chunks(300 / appends) {
            writer.append(chunk).unwrap();
        }
        drop(writer);
        if bare {
            fs::remove_file(self.file(contract, month, FileKind::SortIndex)).unwrap();
        }
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn path(contract: Option<&str>, month: YearMonth, file: FileKind) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: if contract.is_some() { "FNO" } else { "INDEX" },
        symbol: "NIFTY",
        contract: contract.and_then(Contract::parse),
        timeframe: Timeframe::SECOND_1,
        month,
        file,
    })
    .unwrap()
}

fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).unwrap()
}

/// Arguments that do not name a series and a span are a misuse, before
/// anything is opened.
#[test]
fn misuse_is_refused_before_anything_is_opened() {
    let root = Root::new();
    for arguments in [
        &["gdfl"][..],
        &["nope", "NSE/INDEX/NIFTY", "1s", "2032-01", "2032-02"],
        &["gdfl", "NSE/INDEX", "1s", "2032-01", "2032-02"],
        &["gdfl", "NSE/INDEX/NIFTY", "7s", "2032-01", "2032-02"],
        &["gdfl", "NSE/INDEX/NIFTY", "1s", "2032-1", "2032-02"],
        &["gdfl", "NSE/INDEX/NIFTY", "1s", "2032-03", "2032-02"],
        &["gdfl", "NSE//NIFTY", "1s", "2032-01", "2032-02"],
        &[
            "gdfl",
            "NSE/FNO/NIFTY/bad contract",
            "1s",
            "2032-01",
            "2032-02",
        ],
    ] {
        let (code, out) = root.run(arguments);
        assert_eq!(code, MISUSED, "{arguments:?}: {out}");
        assert!(out.contains(USAGE), "{out}");
    }
    assert_eq!(
        fs::read_dir(&root.0).unwrap().count(),
        0,
        "nothing was created"
    );
}

/// Months written before D-3349 are built as one run each; a month already
/// one run is kept; a month a writer appended to three times is rebuilt as
/// one; a month not held is named and nothing is created for it; a rerun
/// writes nothing.
#[test]
fn months_are_built_kept_and_named_and_a_rerun_writes_nothing() {
    let root = Root::new();
    root.month(None, ym(2032, 1), 1, true);
    root.month(None, ym(2032, 2), 1, false);
    root.month(None, ym(2032, 3), 3, false);
    let bars = fs::read(root.file(None, ym(2032, 1), FileKind::Bars)).unwrap();
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2032-01", "2032-04"]);
    assert_eq!(code, FAILED, "2032-04 is not held: {out}");
    assert!(out.contains("2032-01  built"), "{out}");
    assert!(out.contains("no .srt sort index"), "{out}");
    assert!(out.contains("2032-02  kept"), "{out}");
    assert!(
        out.contains("2032-03  built") && out.contains("3 runs"),
        "{out}"
    );
    assert!(out.contains("2032-04  NOT HELD"), "{out}");
    assert!(!root.file(None, ym(2032, 4), FileKind::Bars).exists());
    assert!(!root.file(None, ym(2032, 4), FileKind::SortIndex).exists());
    assert_eq!(
        fs::read(root.file(None, ym(2032, 1), FileKind::Bars)).unwrap(),
        bars
    );

    let before: Vec<Vec<u8>> = (1..=3)
        .map(|m| fs::read(root.file(None, ym(2032, m), FileKind::SortIndex)).unwrap())
        .collect();
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2032-01", "2032-03"]);
    assert_eq!(code, OK, "{out}");
    assert_eq!(out.matches("  kept  ").count(), 3, "{out}");
    let after: Vec<Vec<u8>> = (1..=3)
        .map(|m| fs::read(root.file(None, ym(2032, m), FileKind::SortIndex)).unwrap())
        .collect();
    assert_eq!(before, after, "a rerun changes no byte");
}

/// `--dry-run` says what it would build and writes nothing.
#[test]
fn a_dry_run_writes_nothing() {
    let root = Root::new();
    root.month(None, ym(2032, 5), 1, true);
    root.month(None, ym(2032, 6), 1, false);
    let (code, out) = root.run(&[
        "gdfl",
        "NSE/INDEX/NIFTY",
        "1s",
        "2032-05",
        "2032-06",
        "--dry-run",
    ]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("2032-05  would build") && out.contains("no .srt"),
        "{out}"
    );
    assert!(out.contains("2032-06  indexed"), "{out}");
    assert!(!root.file(None, ym(2032, 5), FileKind::SortIndex).exists());
}

/// A month a writer holds is refused, named, and left as it was.
#[test]
fn a_held_month_is_refused_and_left_alone() {
    let root = Root::new();
    root.month(None, ym(2032, 7), 1, true);
    let symbol_id =
        u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)).unwrap();
    let writer =
        BarFile::open_or_create(&root.0, path(None, ym(2032, 7), FileKind::Bars), symbol_id)
            .unwrap();
    let held = fs::read(root.file(None, ym(2032, 7), FileKind::SortIndex)).unwrap();
    // The writer's own open orders the month; the door is refused while it
    // holds the lock.
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2032-07", "2032-07"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("REFUSED") && out.contains("held by another process"),
        "{out}"
    );
    assert_eq!(
        fs::read(root.file(None, ym(2032, 7), FileKind::SortIndex)).unwrap(),
        held
    );
    drop(writer);
}

/// `*` walks every contract directory under the symbol in name order, and
/// names only the months it finds.
#[test]
fn a_star_walks_every_contract_and_names_what_it_finds() {
    let root = Root::new();
    let (ce, pe) = ("2032-02-26-1000000-CE", "2032-02-26-1000000-PE");
    root.month(Some(ce), ym(2032, 1), 1, true);
    root.month(Some(ce), ym(2032, 2), 2, false);
    root.month(Some(pe), ym(2032, 2), 1, true);
    let (code, out) = root.run(&["gdfl", "NSE/FNO/NIFTY/*", "1s", "2031-12", "2032-02"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("2 series × 3 month(s)"), "{out}");
    assert!(out.contains(&format!("{ce} 2032-01  built")), "{out}");
    assert!(out.contains(&format!("{ce} 2032-02  built")), "{out}");
    assert!(out.contains(&format!("{pe} 2032-02  built")), "{out}");
    assert!(!out.contains("NOT HELD"), "{out}");
    assert!(
        root.file(Some(pe), ym(2032, 2), FileKind::SortIndex)
            .exists()
    );
    let (code, out) = root.run(&[
        "gdfl",
        &format!("NSE/FNO/NIFTY/{ce}"),
        "1s",
        "2032-01",
        "2032-02",
    ]);
    assert_eq!(code, OK, "{out}");
    assert_eq!(out.matches("  kept  ").count(), 2, "{out}");
}

/// **A `*` NAMES WHAT IT DOES NOT WALK, AND AN EMPTY WALK IS NOT A SUCCESS.**
/// Before, a linked contract directory and a stray name were skipped without
/// a word, and a `*` that found no series — or no held month in the span —
/// ended "every month in the span is indexed", exit 0, having proved nothing.
/// A link is never followed (a read-only shadow links its contracts into the
/// store it shadows): it is counted, nothing is built through it, and no byte
/// on the far side changes.
#[test]
fn a_star_names_what_it_does_not_walk_and_refuses_an_empty_walk() {
    let far = Root::new();
    let (ce, pe) = ("2032-02-26-1000000-CE", "2032-02-26-1000000-PE");
    far.month(Some(pe), ym(2032, 2), 1, true);
    let far_dir = far
        .file(Some(pe), ym(2032, 2), FileKind::Bars)
        .parent()
        .and_then(std::path::Path::parent)
        .unwrap()
        .to_path_buf();
    let listing = |dir: &std::path::Path| -> Vec<(String, Vec<u8>)> {
        let mut held: Vec<(String, Vec<u8>)> = fs::read_dir(dir.join("1s"))
            .unwrap()
            .map(|entry| entry.unwrap())
            .map(|entry| {
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    fs::read(entry.path()).unwrap(),
                )
            })
            .collect();
        held.sort();
        held
    };
    let before = listing(&far_dir);

    // ONLY A LINK AND A STRAY NAME: no series, refused, exit FAILED.
    let root = Root::new();
    let symbol = root.0.join("bars/gdfl/NSE/FNO/NIFTY");
    fs::create_dir_all(&symbol).unwrap();
    std::os::unix::fs::symlink(&far_dir, symbol.join(pe)).unwrap();
    fs::create_dir_all(symbol.join("not-a-contract")).unwrap();
    let (code, out) = root.run(&["gdfl", "NSE/FNO/NIFTY/*", "1s", "2032-01", "2032-02"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("SRT BUILD REFUSED"), "{out}");
    assert!(out.contains("holds no contract directory"), "{out}");
    assert!(out.contains("1 symbolic link(s)"), "{out}");
    assert!(out.contains("1 other entr(ies)"), "{out}");
    assert!(!out.contains("every month in the span is indexed"), "{out}");

    // A REAL CONTRACT BESIDE THEM: built, and the receipt counts the rest.
    root.month(Some(ce), ym(2032, 2), 1, true);
    let (code, out) = root.run(&["gdfl", "NSE/FNO/NIFTY/*", "1s", "2032-01", "2032-02"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("1 series × 2 month(s)"), "{out}");
    assert!(
        out.contains("NOT WALKED: 1 symbolic link(s)") && out.contains("1 other entr(ies)"),
        "{out}"
    );
    assert!(out.contains(&format!("{ce} 2032-02  built")), "{out}");
    assert!(!out.contains(pe), "{out}");

    // A SPAN NO CONTRACT HOLDS: said, and not a clean exit — dry or not.
    for dry in [false, true] {
        let mut arguments = vec!["gdfl", "NSE/FNO/NIFTY/*", "1s", "2031-01", "2031-03"];
        if dry {
            arguments.push("--dry-run");
        }
        let (code, out) = root.run(&arguments);
        assert_eq!(code, FAILED, "{out}");
        assert!(
            out.contains(
                "NO MONTH HELD: none of the 1 contract(s) holds a month in 2031-01..2031-03"
            ),
            "{out}"
        );
        assert!(!out.contains("every month in the span is indexed"), "{out}");
    }

    // NOTHING WAS BUILT THROUGH THE LINK: the far side is byte for byte what
    // it was, with no `.srt` and no staging file beside its bars.
    assert_eq!(before, listing(&far_dir));
    assert!(
        !far.file(Some(pe), ym(2032, 2), FileKind::SortIndex)
            .exists()
    );
}

/// **A REPAIRED MONTH IS INDEXED WHERE EVERY READER READS IT** (D-3349 under
/// D-3350). A versioned repair serves the month from a revision under
/// `bar-revisions-v1/<n>/`; the verb confirms the revision's own index, builds
/// it there when it is gone, checks it from the read side against the
/// generation a reader opens, and never touches the original month's index.
/// Before, it sorted the original and its read-back compared that index with
/// the revision's rows, so a repaired month never read as indexed.
#[test]
fn a_repaired_month_is_indexed_in_its_served_generation() {
    let root = Root::new();
    let month = ym(2032, 8);
    root.month(None, month, 1, false);
    let original = fs::read(root.file(None, month, FileKind::SortIndex)).unwrap();
    let symbol_id =
        u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)).unwrap();
    let held: Vec<Bar> = {
        let file =
            BarFile::open_existing(&root.0, path(None, month, FileKind::Bars), symbol_id).unwrap();
        (0..file.header().n_valid)
            .map(|row| file.read_record(row).unwrap())
            .collect()
    };
    // THE FIRST SESSION OF THE MONTH, missing before the held one.
    let first = month.ist_bounds_micros().0 + 555 * 60 * 1_000_000;
    let leading: Vec<Bar> = (0..40_i64)
        .map(|k| Bar {
            ts_micros: first + k * 1_000_000,
            open: 3_190_000 + k,
            high: 3_190_040 + k,
            low: 3_189_960 + k,
            close: 3_190_005 + k,
            volume: 1,
            open_interest: OI_NULL,
        })
        .collect();
    let offered: Vec<Bar> = leading.iter().chain(&held).copied().collect();
    store::repair::repair_month(
        &root.0,
        path(None, month, FileKind::Bars),
        symbol_id,
        &offered,
        store::repair::Policy {
            open_interest_applies: false,
            basis_witness: false,
        },
    )
    .unwrap();
    let revision =
        path(None, month, FileKind::SortIndex).to_path_buf(&root.0.join("bar-revisions-v1/1"));
    assert!(revision.is_file(), "the revision is born sorted");
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2032-08", "2032-08"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("2032-08  kept") && out.contains("340 bars"),
        "the served generation's 340 rows, one run: {out}"
    );
    fs::remove_file(&revision).unwrap();
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2032-08", "2032-08"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("2032-08  built") && out.contains("340 bars"),
        "{out}"
    );
    assert!(revision.is_file(), "rebuilt where the revision lives");
    assert_eq!(
        fs::read(root.file(None, month, FileKind::SortIndex)).unwrap(),
        original,
        "never the original's index"
    );
}

/// **A RUN KILLED PARTWAY RESUMES, AND NO BAR FILE IS EVER REWRITTEN.** Four
/// months written before D-3349. A first run builds two and is "killed"
/// while writing the third: its staging file `2032-11.new.srt` is left half
/// written, and the fourth month was never reached. The rerun keeps the two
/// that are built, cuts the staging file and builds the third from its bars,
/// builds the fourth, and exits clean; a third run changes no byte. Through
/// all of it every `.bin`, `.crc` and `.tix` keeps its bytes AND its
/// modification time — the door opens them read-only.
#[test]
fn a_run_killed_partway_resumes_and_never_rewrites_a_bar_file() {
    let root = Root::new();
    let months = [ym(2032, 9), ym(2032, 10), ym(2032, 11), ym(2032, 12)];
    for month in months {
        root.month(None, month, 2, true);
    }
    let held = |root: &Root| -> Vec<(Vec<u8>, std::time::SystemTime)> {
        months
            .iter()
            .flat_map(|&month| {
                [FileKind::Bars, FileKind::Checksums, FileKind::TimeIndex].map(|kind| {
                    let file = root.file(None, month, kind);
                    let bytes = fs::read(&file).unwrap_or_default();
                    let modified = fs::metadata(&file)
                        .and_then(|meta| meta.modified())
                        .unwrap_or(std::time::UNIX_EPOCH);
                    (bytes, modified)
                })
            })
            .collect()
    };
    let before = held(&root);
    assert!(before.iter().step_by(3).all(|(bytes, _)| !bytes.is_empty()));

    // THE FIRST RUN, AS FAR AS IT GOT.
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2032-09", "2032-10"]);
    assert_eq!(code, OK, "{out}");
    assert_eq!(out.matches("  built  ").count(), 2, "{out}");
    let staging = root
        .file(None, ym(2032, 11), FileKind::SortIndex)
        .with_file_name("2032-11.new.srt");
    fs::write(&staging, vec![0x5A_u8; 70_000]).unwrap();
    assert!(!root.file(None, ym(2032, 11), FileKind::SortIndex).exists());

    // A DRY RUN SAYS WHERE IT STANDS and writes nothing: the half-written
    // staging file is not an index.
    let (code, out) = root.run(&[
        "gdfl",
        "NSE/INDEX/NIFTY",
        "1s",
        "2032-09",
        "2032-12",
        "--dry-run",
    ]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains(
            "Indexed 2, would build 2, empty 0, not held 0, refused 0. Nothing was written."
        ),
        "{out}"
    );
    assert_eq!(fs::read(&staging).unwrap().len(), 70_000);

    // THE RERUN.
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2032-09", "2032-12"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("2032-09  kept") && out.contains("2032-10  kept"),
        "{out}"
    );
    assert!(
        out.contains("2032-11  built") && out.contains("2032-12  built"),
        "{out}"
    );
    assert!(
        out.contains(
            "Built 2, kept 2, empty 0, unsortable 0, not indexed 0, not held 0, refused 0"
        ),
        "{out}"
    );
    assert!(!staging.exists(), "the staging file was cut and renamed");
    // The month built over the dead writer's staging file is the same bytes
    // as one built clean.
    let rebuilt = fs::read(root.file(None, ym(2032, 11), FileKind::SortIndex)).unwrap();
    fs::remove_file(root.file(None, ym(2032, 11), FileKind::SortIndex)).unwrap();
    let (code, _) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2032-11", "2032-11"]);
    assert_eq!(code, OK);
    assert_eq!(
        fs::read(root.file(None, ym(2032, 11), FileKind::SortIndex)).unwrap(),
        rebuilt,
        "a build is a function of the bars alone"
    );

    let indexes = |root: &Root| -> Vec<Vec<u8>> {
        months
            .iter()
            .map(|&month| fs::read(root.file(None, month, FileKind::SortIndex)).unwrap())
            .collect()
    };
    let built = indexes(&root);
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2032-09", "2032-12"]);
    assert_eq!(code, OK, "{out}");
    assert_eq!(out.matches("  kept  ").count(), 4, "{out}");
    assert_eq!(indexes(&root), built, "a rerun changes no byte");
    assert_eq!(
        held(&root),
        before,
        "no bar file, checksum file or time index was written or touched"
    );
}

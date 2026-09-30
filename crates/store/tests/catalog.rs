//! **The store can list what it holds** — `store::catalog::walk`.
//!
//! The defect these tests exist for: `crates/store` owned the path layout and
//! could not read it back. Measured before `catalog` landed, this crate held
//! **zero** `read_dir` calls, so nothing could ask the store which months it
//! had. `cli sweep-stored` therefore took an explicit vendor, underlying, rung,
//! year and month, and no caller could sweep more than one instrument-month per
//! invocation.
//!
//! Every refusal is exercised on a real temporary tree rather than a mocked one,
//! because the thing under test is a directory walk and a mocked directory
//! proves nothing about `read_dir`.

// A test that asserts nothing is banned, and a test that cannot fail loudly is
// a test that asserts nothing. These allow the harness to panic on a broken
// invariant instead of threading `Result` through every assertion.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the exception every test module in this workspace takes"
)]

use brutex_core::vendor::Vendor;
use std::path::{Path, PathBuf};
use store::catalog::{self, Census};
use store::path::{Timeframe, YearMonth};

/// A private directory, named for the test that owns it so two can run at once.
fn scratch(name: &str) -> PathBuf {
    let mut root = std::env::temp_dir();
    root.push(format!("brutex-catalog-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch root is creatable");
    root
}

/// Writes an empty file at `root/bars/<rel>`, creating parents.
fn put(root: &Path, rel: &str) {
    let full = root.join("bars").join(rel);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).expect("parents are creatable");
    }
    std::fs::write(&full, b"").expect("file is writable");
}

/// The first segment this walk expects is the one the renderer writes.
///
/// Named in `catalog`'s own header as the check that keeps the duplicated
/// `bars` constant honest. A renderer that moved to another root would leave
/// this walk reading an empty tree and reporting an empty store — a wrong answer
/// that looks exactly like a correct one.
#[test]
fn the_walker_and_the_renderer_agree_on_the_root() {
    let rendered = store::path::StorePath::for_key(
        Vendor::Groww,
        &brutex_core::instrument::InstrumentKey::index(
            brutex_core::instrument::Exchange::Nse,
            "NIFTY",
        )
        .expect("NSE/NIFTY is a legal key"),
        Timeframe::MINUTE_1,
        YearMonth::new(2026, 8).expect("2026-08 is a month"),
        store::path::FileKind::Bars,
    )
    .expect("the key renders")
    .to_string();

    assert!(
        rendered.starts_with("bars/"),
        "the walker looks under `bars/`; the renderer wrote `{rendered}`"
    );
}

/// A store with nothing pulled yet is empty, not broken.
#[test]
fn a_store_with_no_bars_directory_is_empty_and_not_an_error() {
    let root = scratch("empty");
    let out = catalog::walk(&root).expect("an unpulled store is not an error");
    assert!(out.held.is_empty(), "nothing is held");
    assert_eq!(out.census, Census::default(), "and nothing was seen");
    assert!(out.census.reconciles());
}

/// One real spot month comes back with every segment intact.
#[test]
fn one_spot_month_is_found_with_its_segments_intact() {
    let root = scratch("one");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");

    let out = catalog::walk(&root).expect("the walk runs");
    assert_eq!(out.held.len(), 1, "exactly one instrument-month");
    let held = &out.held[0];
    assert_eq!(held.vendor, Vendor::Groww);
    assert_eq!(held.exchange, "NSE");
    assert_eq!(held.segment, "INDEX");
    assert_eq!(held.symbol, "NIFTY");
    assert_eq!(held.timeframe.as_str(), "1min");
    assert_eq!(held.month.to_string(), "2026-08");
    assert_eq!(out.census.spot, 1);
    assert!(out.census.reconciles());
}

/// Malformed month spellings remain counted and preserved beside a valid month.
#[test]
fn malformed_month_names_cannot_masquerade_as_a_canonical_held_month() {
    let root = scratch("month-spelling");
    let invalid = [
        "026-08", "02026-08", "2026-8", "2026-008", "year-08", "2026-mm", "1969-08", "0000-08",
        "2026-00", "2026-13", "202608",
    ];
    for stem in invalid {
        put(&root, &format!("groww/NSE/INDEX/NIFTY/1min/{stem}.bin"));
    }
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    let found = catalog::walk(&root).expect("malformed names do not stop the walk");
    assert_eq!(found.census.seen, invalid.len() as u64 + 1);
    assert_eq!(found.census.malformed_month, invalid.len() as u64);
    assert_eq!(found.census.spot, 1);
    assert!(found.census.reconciles());
    assert_eq!(found.held.len(), 1);
    assert_eq!(found.held[0].month.to_string(), "2026-08");
    for stem in invalid {
        assert_eq!(
            std::fs::read(root.join(format!("bars/groww/NSE/INDEX/NIFTY/1min/{stem}.bin")))
                .expect("malformed entry is preserved"),
            b""
        );
    }
    let repeated = catalog::walk(&root).expect("repeat walk");
    assert_eq!(repeated.census, found.census);
    assert_eq!(repeated.held, found.held);
}

/// **Every refusal is reachable, and the census adds up over all of them.**
///
/// The row `catalog`'s header points at. One store carrying every outcome at
/// once, because a census that reconciles on a clean tree proves nothing about
/// the arithmetic — the buckets have to be non-zero together.
#[test]
fn the_census_reconciles_over_a_store_holding_every_refusal() {
    let root = scratch("every");
    // spot — the only kind that becomes a row
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    put(&root, "dhan/NSE/INDEX/BANKNIFTY/1day/2026-07.bin");
    // a contract level: counted, never a row
    put(&root, "groww/NSE/FNO/NIFTY/26AUG24000CE/1min/2026-08.bin");
    // a sibling file kind
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.crc");
    // a feed this build does not know
    put(&root, "nosuchfeed/NSE/INDEX/NIFTY/1min/2026-08.bin");
    // a rung the store does not carry
    put(&root, "groww/NSE/INDEX/NIFTY/7min/2026-08.bin");
    // months that do not parse
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-13.bin");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/notamonth.bin");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-8.bin");
    // too shallow
    put(&root, "groww/NSE/INDEX/2026-08.bin");

    let out = catalog::walk(&root).expect("the walk runs");
    let c = out.census;

    assert_eq!(c.seen, 10, "every file was seen");
    assert_eq!(c.spot, 2, "two spot months");
    assert_eq!(c.with_contract, 1, "one contract month");
    assert_eq!(c.other_kind, 1, "one non-bars sibling");
    assert_eq!(c.unknown_vendor, 1, "one unknown feed");
    assert_eq!(c.unknown_rung, 1, "one unknown rung");
    assert_eq!(
        c.malformed_month, 3,
        "13, notamonth, and the unpadded 2026-8"
    );
    assert_eq!(c.wrong_depth, 1, "one path too shallow");
    assert!(c.reconciles(), "the parts must sum to the whole: {c:?}");
    assert_eq!(out.held.len(), 2, "only the spot months are rows");
}

/// A census that has lost a file is caught.
#[test]
fn a_census_that_does_not_add_up_is_refused() {
    let broken = Census {
        seen: 5,
        spot: 1,
        ..Census::default()
    };
    assert!(!broken.reconciles(), "4 files are unaccounted for");
}

/// **Every month the renderer writes parses back.**
///
/// The round trip `parse_month` is written against. A decade of months, both
/// halves, so a change to either the `{:04}-{:02}` format or the parser fails
/// here rather than in a store nobody can list.
#[test]
fn every_month_the_renderer_writes_parses_back() {
    let root = scratch("roundtrip");
    let mut expected = Vec::new();
    for year in 2020_u16..2030 {
        for month in 1_u8..=12 {
            let ym = YearMonth::new(year, month).expect("a real month");
            put(&root, &format!("groww/NSE/INDEX/NIFTY/1min/{ym}.bin"));
            expected.push(ym.to_string());
        }
    }
    let out = catalog::walk(&root).expect("the walk runs");
    assert_eq!(out.census.spot, 120, "ten years of months");
    assert_eq!(out.census.malformed_month, 0, "none failed to parse back");

    let mut got: Vec<String> = out.held.iter().map(|h| h.month.to_string()).collect();
    got.sort();
    expected.sort();
    assert_eq!(got, expected, "every rendered month came back identical");
}

/// Rows arrive sorted and without duplicates.
#[test]
fn rows_are_sorted_and_deduplicated() {
    let root = scratch("sorted");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    put(&root, "groww/NSE/INDEX/BANKNIFTY/1min/2026-08.bin");
    put(&root, "dhan/NSE/INDEX/NIFTY/1min/2026-01.bin");

    let out = catalog::walk(&root).expect("the walk runs");
    let mut sorted = out.held.clone();
    sorted.sort();
    assert_eq!(out.held, sorted, "rows arrive in sorted order");
    let mut deduped = out.held.clone();
    deduped.dedup();
    assert_eq!(out.held.len(), deduped.len(), "no duplicates");
}

/// Both timeframes and both feeds round-trip, not just the ones a fixture picked.
#[test]
fn every_known_rung_and_every_known_feed_is_recognised() {
    let root = scratch("allrungs");
    let mut want = 0_u64;
    for vendor in Vendor::ALL {
        for rung in Timeframe::KNOWN {
            put(
                &root,
                &format!(
                    "{}/NSE/INDEX/NIFTY/{}/2026-08.bin",
                    vendor.as_str(),
                    rung.as_str()
                ),
            );
            want += 1;
        }
    }
    let out = catalog::walk(&root).expect("the walk runs");
    assert_eq!(out.census.unknown_vendor, 0, "every feed was recognised");
    assert_eq!(out.census.unknown_rung, 0, "every rung was recognised");
    assert_eq!(out.census.spot, want, "all {want} combinations came back");
    assert!(out.census.reconciles());
}

/// **A `bars` that is not a directory is refused by name, not an empty
/// store.** D-0769.
///
/// The walk opened with `bars.is_dir()`, which answers false on any failure
/// to stat, so a `bars` that was a regular file returned `Ok` with a zero
/// census that reconciled: the same answer as "nothing pulled yet". Only a
/// `bars` that is absent is the empty store.
#[test]
fn a_bars_path_that_is_not_a_directory_is_refused_by_name() {
    let root = scratch("notadir");
    std::fs::write(root.join("bars"), b"not a directory").expect("writable");
    let refused = catalog::walk(&root).expect_err("a file where `bars` belongs is refused");
    assert_eq!(
        refused,
        catalog::CatalogError::BarsUnreadable {
            because: "it is not a directory".to_owned()
        }
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A `bars` link that resolves to nothing is refused, not an empty store.**
/// D-0769.
///
/// `is_dir` follows the link, fails to resolve it, and answered false, so a
/// store whose `bars` pointed at a volume that was not mounted listed nothing
/// and reconciled.
#[test]
fn a_dangling_bars_link_is_refused_not_empty() {
    let root = scratch("danglingbars");
    std::os::unix::fs::symlink(root.join("unmounted"), root.join("bars"))
        .expect("a symlink is creatable");
    let refused = catalog::walk(&root).expect_err("a dangling `bars` is refused");
    let catalog::CatalogError::BarsUnreadable { because } = refused;
    assert!(because.contains("os error 2"), "{because}");
    let _ = std::fs::remove_dir_all(&root);
}

/// **A `bars` link to a real directory is still walked.** D-0769.
///
/// The root is operator configuration and is followed as it always was; only
/// links BELOW `bars/` are counted and not followed (D-0766). This pins that
/// the stricter root probe did not narrow that.
#[test]
fn a_bars_link_to_a_real_directory_is_walked() {
    let root = scratch("linkedbars");
    let real = root.join("real");
    let file = real.join("groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("creatable");
    std::fs::write(&file, b"").expect("writable");
    std::os::unix::fs::symlink(&real, root.join("bars")).expect("a symlink is creatable");
    let out = catalog::walk(&root).expect("a linked `bars` is walked");
    assert_eq!(
        out.census,
        Census {
            seen: 1,
            spot: 1,
            ..Census::default()
        }
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A store root this process cannot search is refused, not an empty
/// store.** D-0769.
///
/// With the root at mode 000 the stat of `root/bars` fails with a permission
/// error, `is_dir` answered false, and the walk returned `Ok` with a zero
/// census while a month sat on disk (found by a review of D-0765).
#[test]
fn a_store_root_that_cannot_be_searched_is_refused() {
    use std::os::unix::fs::PermissionsExt;
    let root = scratch("root000");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o000))
        .expect("the fixture root can be locked");
    let precondition = std::fs::symlink_metadata(root.join("bars")).is_err();
    let out = catalog::walk(&root);
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755))
        .expect("the fixture root can be unlocked");
    assert!(
        precondition,
        "a process that ignores permissions cannot run this test"
    );
    let catalog::CatalogError::BarsUnreadable { because } =
        out.expect_err("an unsearchable root is refused");
    assert!(because.contains("os error 13"), "{because}");
    let unlocked = catalog::walk(&root).expect("the walk runs once unlocked");
    assert_eq!(unlocked.census.spot, 1);
    let _ = std::fs::remove_dir_all(&root);
}

/// The error type says what happened in the operator's words.
#[test]
fn the_error_names_what_could_not_be_read() {
    let e = catalog::CatalogError::BarsUnreadable {
        because: "permission denied".to_owned(),
    };
    let text = e.to_string();
    assert!(text.contains("bars"), "names the directory: {text}");
    assert!(
        text.contains("permission denied"),
        "names the cause: {text}"
    );
}

/// Nested directories below the bar file's own level are walked, not assumed.
#[test]
fn the_walk_descends_rather_than_guessing_the_depth() {
    let root = scratch("deep");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/deeper/2026-09.bin");

    let out = catalog::walk(&root).expect("the walk runs");
    assert_eq!(out.census.seen, 2, "both files were reached");
    assert_eq!(out.census.spot, 1, "only the correctly-placed one is a row");
    assert_eq!(
        out.census.with_contract, 1,
        "the deeper one reads as a contract level"
    );
    assert!(out.census.reconciles());
}

/// **A directory the walk cannot list is counted, not dropped.** D-0765.
///
/// The walk skipped a non-root directory whose `read_dir` failed with a bare
/// `continue` and no census bucket, so a store with one locked symbol
/// directory reported `seen 1, spot 1` and reconciled while a second month sat
/// on disk unlisted (W3-store1-4). The locked month still cannot become a row
/// (its contents are unknown), but the census now names it.
#[test]
fn a_directory_the_walk_cannot_list_is_counted_not_dropped() {
    use std::os::unix::fs::PermissionsExt;
    let root = scratch("unreadable");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    put(&root, "groww/NSE/INDEX/BANKNIFTY/1min/2026-08.bin");
    let locked = root.join("bars/groww/NSE/INDEX/BANKNIFTY");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))
        .expect("the fixture directory can be locked");
    let precondition = std::fs::read_dir(&locked).is_err();
    let out = catalog::walk(&root);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))
        .expect("the fixture directory can be unlocked");
    assert!(
        precondition,
        "the locked directory must refuse `read_dir`; a process that ignores permissions cannot run this test"
    );
    let out = out.expect("one locked branch does not fail the walk");

    assert_eq!(out.census.unreadable, 1, "the locked directory is named");
    assert_eq!(out.census.seen, 2, "one file and one unreadable directory");
    assert_eq!(out.census.spot, 1, "the readable month is still a row");
    assert!(out.census.reconciles(), "{:?}", out.census);
    assert_eq!(out.held.len(), 1);
    assert_eq!(out.held[0].symbol, "NIFTY");

    let unlocked = catalog::walk(&root).expect("the walk runs");
    assert_eq!(
        unlocked.census.unreadable, 0,
        "nothing is unreadable once unlocked"
    );
    assert_eq!(
        unlocked.census.spot, 2,
        "both months are rows once unlocked"
    );
}

/// An unreadable entry is a part of the whole like every other bucket.
#[test]
fn an_unreadable_entry_is_part_of_the_reconciliation() {
    let counted = Census {
        seen: 2,
        spot: 1,
        unreadable: 1,
        ..Census::default()
    };
    assert!(counted.reconciles(), "1 + 1 == 2");
    let lost = Census {
        seen: 2,
        spot: 1,
        ..Census::default()
    };
    assert!(
        !lost.reconciles(),
        "the unreadable one must be counted somewhere"
    );
}

/// **A symlink back to an ancestor is one entry, not a second walk of the
/// tree.** D-0766.
///
/// `is_dir` follows links, so one real file beside `back -> ../..` was seen
/// again at every level before `read_dir` failed on the kernel's link limit
/// (W3-store1-5, reproduced on origin/main 2c209309 on macOS). How many times
/// depends on the scratch root: `seen 34, wrong_depth 33` under
/// `/private/tmp/...` and `seen 33, wrong_depth 32` under the same directory
/// reached through the `/tmp` link, which spends one of the limit's links. No
/// test asserts that count. The walk no longer follows a link at all.
#[test]
fn a_symlink_loop_is_counted_once_and_not_walked() {
    let root = scratch("loop");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    std::os::unix::fs::symlink("../..", root.join("bars/groww/NSE/back"))
        .expect("a symlink is creatable");

    let out = catalog::walk(&root).expect("the walk runs");
    assert_eq!(
        out.census.linked, 1,
        "the link back is named once: {:?}",
        out.census
    );
    assert_eq!(
        out.census.seen, 2,
        "one file and one link: {:?}",
        out.census
    );
    assert_eq!(out.census.spot, 1);
    assert_eq!(out.census.wrong_depth, 0, "no path was walked twice");
    assert!(out.census.reconciles(), "{:?}", out.census);
    assert_eq!(out.held.len(), 1);
}

/// **A link that resolves to nothing is `linked`, not a bar file.** D-0766.
///
/// `is_dir` answers false for a link it cannot resolve, so the walk filed a
/// dangling `2026-09.bin` as a spot month that no reader could open. A link to
/// a real file or directory is `linked` as well and is not followed, because
/// the store's writer never makes one (`docs/02-store-format.md` §9).
#[test]
fn a_symlink_is_linked_whether_it_resolves_or_not() {
    let root = scratch("linked");
    let elsewhere = root.join("elsewhere/dhan/NSE/INDEX/NIFTY/1min");
    std::fs::create_dir_all(&elsewhere).expect("creatable");
    std::fs::write(elsewhere.join("2026-07.bin"), b"").expect("writable");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    std::os::unix::fs::symlink(root.join("elsewhere/dhan"), root.join("bars/dhan"))
        .expect("a symlink is creatable");
    std::os::unix::fs::symlink(
        root.join("nowhere.bin"),
        root.join("bars/groww/NSE/INDEX/NIFTY/1min/2026-09.bin"),
    )
    .expect("a symlink is creatable");
    std::os::unix::fs::symlink(
        root.join("bars/groww/NSE/INDEX/NIFTY/1min/2026-08.bin"),
        root.join("bars/groww/NSE/INDEX/NIFTY/1min/2026-10.bin"),
    )
    .expect("a symlink is creatable");

    let out = catalog::walk(&root).expect("the walk runs");
    assert_eq!(out.census.linked, 3, "{:?}", out.census);
    assert_eq!(
        out.census.spot, 1,
        "only the real file is a row: {:?}",
        out.census
    );
    assert_eq!(out.census.seen, 4, "{:?}", out.census);
    assert!(out.census.reconciles(), "{:?}", out.census);
    let months: Vec<(Vendor, String)> = out
        .held
        .iter()
        .map(|h| (h.vendor, h.month.to_string()))
        .collect();
    assert_eq!(months, vec![(Vendor::Groww, "2026-08".to_owned())]);
}

/// A link is a part of the whole like every other bucket.
#[test]
fn a_link_is_part_of_the_reconciliation() {
    let counted = Census {
        seen: 2,
        spot: 1,
        linked: 1,
        ..Census::default()
    };
    assert!(counted.reconciles(), "1 + 1 == 2");
    let lost = Census {
        seen: 2,
        spot: 1,
        ..Census::default()
    };
    assert!(!lost.reconciles(), "the link must be counted somewhere");
}

/// **A signed month field is malformed, not a second spelling of a month.**
/// D-0767.
///
/// The width check passed `2026-+8` because the integer parse accepts a
/// leading `+`, so it was listed as `2026-08` beside the real `2026-08.bin`
/// and the census counted two spot months for one row (W3-store1-6,
/// reproduced on origin/main 2c209309).
#[test]
fn a_signed_month_field_is_malformed_not_a_second_spelling() {
    let root = scratch("signed");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-+8.bin");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/+026-08.bin");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    let out = catalog::walk(&root).expect("the walk runs");
    assert_eq!(out.census.malformed_month, 2, "{:?}", out.census);
    assert_eq!(out.census.spot, 1, "{:?}", out.census);
    assert!(out.census.reconciles(), "{:?}", out.census);
    assert_eq!(out.held.len(), 1);
    assert_eq!(out.held[0].month.to_string(), "2026-08");
}

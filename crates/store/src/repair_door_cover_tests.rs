#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
//! BASE-COV-LIBS-LS1: the repair doors at the edges no other test reached: a
//! month already served from the last revision, a next revision another
//! request holds, an indexed sibling at the drop door, a restatement that
//! only adds bars, an empty month under a basis-witness policy, a published
//! revision written from another source or other bars, and a revision the
//! host will not let the door create. Every bar and reason is invented.

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;

use super::*;
use crate::format::OI_NULL;
use crate::path::{PathParts, Timeframe, YearMonth};

const SYMBOL: u32 = 0x7E9A;
const MINUTE: i64 = 60_000_000;
const REASON: &str = "an invented correction for a cover test";
const PLAIN: Policy = Policy {
    open_interest_applies: false,
    basis_witness: false,
};
const RESTATE: Restate = Restate {
    open_interest_applies: false,
};

/// A store root that removes itself.
struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-repair-door-cover-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _stale = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _cleanup = fs::remove_dir_all(&self.0);
    }
}

fn month() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "QREPC",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// Invented one-minute bars at minutes `minutes` after 09:15 IST on 3 June
/// 2024.
fn bars(minutes: std::ops::Range<i64>) -> Vec<Bar> {
    let origin =
        YearMonth::new(2024, 6).unwrap().ist_bounds_micros().0 + (2 * 1_440 + 555) * MINUTE;
    minutes
        .map(|k| Bar {
            ts_micros: origin + k * MINUTE,
            open: 2_202_000 + k,
            high: 2_202_900,
            low: 2_201_100,
            close: 2_202_500 + k,
            volume: 5,
            open_interest: OI_NULL,
        })
        .collect()
}

/// The original month of `held` bars; its header.
fn original(root: &Root, held: &[Bar]) -> Header {
    let mut writer = BarFile::open_or_create(&root.0, month(), SYMBOL).unwrap();
    if !held.is_empty() {
        writer.append(held).unwrap();
    }
    writer.header()
}

/// Publishes `bars` as revision `ordinal` of the original and serves it.
fn served_from(root: &Root, source: Header, bars: &[Bar], ordinal: u32) {
    let revision = Revision::new(ordinal).unwrap();
    publish(&root.0, month(), SYMBOL, revision, source, bars).unwrap();
    crate::generation::promote(&root.0, month(), SYMBOL, revision).unwrap();
    assert_eq!(served_ordinal(root), ordinal);
}

fn served_ordinal(root: &Root) -> u32 {
    crate::generation::served(&root.0, month())
        .unwrap()
        .ordinal()
}

/// **A MONTH SERVED FROM THE LAST REVISION HAS NONE LEFT FOR ANY DOOR.** The
/// ingest's repair, the correction door and the drop door each refuse with
/// `RevisionLimit`, naming nothing they skipped, and the month is still
/// served from the last revision: nothing was written.
#[test]
fn a_month_served_from_the_last_revision_has_none_left_for_any_door() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("last");
    let held = bars(0..10);
    let source = original(&root, &held);
    served_from(&root, source, &held, MAX_REVISIONS);
    assert_eq!(
        repair_month(&root.0, month(), SYMBOL, &bars(20..22), PLAIN),
        Err(RepairError::RevisionLimit)
    );
    let mut corrected = held[3];
    corrected.close += 7;
    assert_eq!(
        restate_month(&root.0, month(), SYMBOL, &[corrected], RESTATE, REASON),
        Err(RepairError::RevisionLimit)
    );
    assert_eq!(
        drop_month(&root.0, month(), SYMBOL, &[held[4].ts_micros], REASON),
        Err(RepairError::RevisionLimit)
    );
    assert_eq!(served_ordinal(&root), MAX_REVISIONS);
}

/// **A CORRECTION WHOSE NEXT REVISION ANOTHER REQUEST HOLDS STEPS OVER IT.**
/// Served from revision 1,023, with 1,024 already published with one bar
/// more and never promoted: the correction door meets that revision, steps
/// over it rather than reopening it, and has no revision left. Revision 1,024
/// still holds the other bars.
#[test]
fn a_correction_whose_next_revision_another_request_holds_steps_over_it() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("held-next");
    let held = bars(0..10);
    let source = original(&root, &held);
    served_from(&root, source, &held, MAX_REVISIONS - 1);
    let served = RevisionReader::open(
        &root.0,
        month(),
        SYMBOL,
        Revision::new(MAX_REVISIONS - 1).unwrap(),
    )
    .unwrap()
    .header();
    let last = Revision::new(MAX_REVISIONS).unwrap();
    let mut more = held.clone();
    more.extend(bars(15..16));
    publish(&root.0, month(), SYMBOL, last, served, &more).unwrap();
    let mut corrected = held[2];
    corrected.close += 3;
    assert_eq!(
        restate_month(&root.0, month(), SYMBOL, &[corrected], RESTATE, REASON),
        Err(RepairError::RevisionLimit)
    );
    let other = RevisionReader::open(&root.0, month(), SYMBOL, last).unwrap();
    assert_eq!(other.header().n_valid, 11);
    assert_eq!(served_ordinal(&root), MAX_REVISIONS - 1);
}

/// **THE DROP DOOR REFUSES A MONTH WITH AN INDEXED SIBLING, NAMED.** An
/// overlay or a greeks file beside the month indexes its bars by record; a
/// drop would renumber them, so it is refused by the sibling's path before
/// anything is written.
#[test]
fn the_drop_door_refuses_a_month_with_an_indexed_sibling() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    for kind in [FileKind::Overlay, FileKind::Greeks] {
        let root = Root::new("sibling");
        let held = bars(0..6);
        original(&root, &held);
        let sibling = month().with_file(kind).to_path_buf(&root.0);
        fs::write(&sibling, b"an invented index").unwrap();
        assert_eq!(
            drop_month(&root.0, month(), SYMBOL, &[held[1].ts_micros], REASON),
            Err(RepairError::IndexedSibling(sibling)),
            "{kind:?}"
        );
        assert_eq!(served_ordinal(&root), 0);
    }
}

/// **A CORRECTION THAT ONLY ADDS BARS PUBLISHES THEM, BEFORE AND AFTER THE
/// HELD ONES.** Nothing held is restated; two bars land before the first held
/// bar and one after the last, and the revision holds all of them in order.
#[test]
fn a_correction_that_only_adds_bars_publishes_them_in_order() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("adds");
    let held = bars(5..10);
    original(&root, &held);
    let mut offered = bars(1..3);
    offered.extend(bars(12..13));
    let done = restate_month(&root.0, month(), SYMBOL, &offered, RESTATE, REASON).unwrap();
    match &done {
        MonthRestate::Restated {
            restated_at,
            added,
            held,
            ..
        } => {
            assert!(restated_at.is_empty(), "{restated_at:?}");
            assert_eq!((*added, *held), (3, 5));
        }
        other @ MonthRestate::NothingNew { .. } => {
            panic!("the added bars are published: {other:?}")
        }
    }
    let mut expected = offered[..2].to_vec();
    expected.extend(&held);
    expected.push(offered[2]);
    let reader = BarFile::open_existing(&root.0, month(), SYMBOL).unwrap();
    assert_eq!(reader.read_range(0, reader.records()).unwrap(), expected);
}

/// **AN EMPTY MONTH NEEDS NO BASIS WITNESS.** The witness proves two pulls
/// share a basis by a stamp both hold; a month that holds nothing has no
/// basis to share, so its bars are added under a basis-witness policy.
#[test]
fn an_empty_month_needs_no_basis_witness() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("empty-witness");
    original(&root, &[]);
    let witness = Policy {
        open_interest_applies: false,
        basis_witness: true,
    };
    match repair_month(&root.0, month(), SYMBOL, &bars(0..4), witness).unwrap() {
        MonthRepair::Repaired { held, added, .. } => assert_eq!((held, added), (0, 4)),
        other @ MonthRepair::NothingNew { .. } => {
            panic!("the empty month takes the bars: {other:?}")
        }
    }
    // A month that holds bars and shares no stamp with the offer is refused.
    let held = Root::new("held-witness");
    original(&held, &bars(0..4));
    assert_eq!(
        repair_month(&held.0, month(), SYMBOL, &bars(10..12), witness),
        Err(RepairError::NoBasisWitness)
    );
}

/// **A REVISION PUBLISHED FROM ANOTHER SOURCE OR OTHER BARS IS NEVER
/// REUSED.** The drop door's revision 1 already holds the month less one bar.
/// The same drop again reuses it; a drop of another bar (same count, other
/// records) is a conflict; and after the original grows, the same drop from
/// the new source is a conflict too, because revision 1 was cut from the old
/// one.
#[test]
fn a_revision_published_from_another_source_or_other_bars_is_never_reused() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("reserved");
    let held = bars(0..8);
    let source = original(&root, &held);
    let first = Revision::new(1).unwrap();
    let cut = |source: Header, at: usize| {
        publish_without(
            &root.0,
            month(),
            SYMBOL,
            first,
            source,
            &[held[at].ts_micros],
            REASON,
        )
    };
    assert_eq!(cut(source, 2), Ok(Published::Created));
    assert_eq!(cut(source, 2), Ok(Published::Reused));
    assert_eq!(cut(source, 3), Err(RepairError::Conflict));
    let grown = {
        let mut writer = BarFile::open_or_create(&root.0, month(), SYMBOL).unwrap();
        writer.append(&bars(8..9)).unwrap();
        writer.header()
    };
    assert_ne!(grown, source);
    assert_eq!(cut(grown, 2), Err(RepairError::Conflict));
}

/// **A REVISION THE HOST WILL NOT LET THE DOOR CREATE IS THE HOST'S REFUSAL,
/// NAMED.** With the store root closed to writes, the ingest's repair and the
/// correction door each return the directory step's refusal at once, never
/// a skipped ordinal, and nothing is served but the original. Root writes into
/// a mode-0555 directory, so the body runs where the mode bits bind: as
/// `nobody` when this process is root (D-0995).
#[test]
fn a_revision_the_host_will_not_let_the_door_create_is_refused_by_the_host() {
    crate::support::where_permission_binds(
        "repair::door_cover_tests::a_revision_the_host_will_not_let_the_door_create_is_refused_by_the_host",
        a_revision_the_host_will_not_let_the_door_create_is_refused_by_the_host_body,
    );
}

fn a_revision_the_host_will_not_let_the_door_create_is_refused_by_the_host_body() {
    use std::os::unix::fs::PermissionsExt as _;
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("closed-root");
    let held = bars(0..6);
    original(&root, &held);
    let mut corrected = held[1];
    corrected.close += 1;
    fs::set_permissions(&root.0, fs::Permissions::from_mode(0o555)).unwrap();
    let repaired = repair_month(&root.0, month(), SYMBOL, &bars(9..10), PLAIN);
    let restated = restate_month(&root.0, month(), SYMBOL, &[corrected], RESTATE, REASON);
    fs::set_permissions(&root.0, fs::Permissions::from_mode(0o755)).unwrap();
    for refused in [repaired.map(|_| ()), restated.map(|_| ())] {
        assert!(
            matches!(
                &refused,
                Err(RepairError::Io {
                    kind: io::ErrorKind::PermissionDenied,
                    publication_may_be_visible: false,
                    ..
                })
            ),
            "{refused:?}"
        );
    }
    assert_eq!(served_ordinal(&root), 0);
}

/// **A REFUSED SOURCE RELEASE AFTER A DROP'S REVISION IS WRITTEN SAYS THE
/// PUBLICATION MAY BE VISIBLE, AND IT IS** (the flock test seam: one unlock,
/// the revision writer's, is let through first). The same request again
/// finds the revision it was told may exist.
#[test]
fn a_refused_source_release_after_a_drop_revision_says_it_may_be_visible() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("drop-release");
    let held = bars(0..5);
    let source = original(&root, &held);
    let first = Revision::new(1).unwrap();
    let drop = [held[2].ts_micros];
    crate::flock::tests::refuse_unlock(1, std::io::ErrorKind::PermissionDenied);
    assert_eq!(
        publish_without(&root.0, month(), SYMBOL, first, source, &drop, REASON),
        Err(RepairError::Io {
            path: month().with_file(FileKind::Lock).to_path_buf(&root.0),
            operation: "release shared lock",
            kind: std::io::ErrorKind::PermissionDenied,
            publication_may_be_visible: true,
        })
    );
    assert_eq!(
        publish_without(&root.0, month(), SYMBOL, first, source, &drop, REASON),
        Ok(Published::Reused)
    );
}

/// **A RESTATE BESIDE A DROP THAT OFFERS NONE OF ITS STAMPS HOLDS NOTHING
/// BACK.** The quarantine names one stamp of the month; a restate that only
/// adds a later bar takes every bar it offers, holds none back, and serves
/// the dropped stamp still dropped.
#[test]
fn a_restate_beside_a_drop_that_offers_none_of_its_stamps_holds_nothing_back() {
    let root = Root::new("none-held-back");
    let held = bars(0..6);
    original(&root, &held);
    crate::quarantine::apply_month(
        &root.0,
        month(),
        SYMBOL,
        &[held[2].ts_micros],
        crate::quarantine::Why {
            cause: crate::quarantine::Cause::ForeignLevel,
            who: "ls1",
            reason: REASON,
        },
    )
    .unwrap();
    let added = bars(6..7);
    match restate_month(&root.0, month(), SYMBOL, &added, RESTATE, REASON).unwrap() {
        MonthRestate::Restated {
            added: 1,
            held_back,
            ..
        } => assert!(held_back.is_empty(), "{held_back:?}"),
        other => panic!("the added bar is restated: {other:?}"),
    }
    let served = BarFile::open_existing(&root.0, month(), SYMBOL).unwrap();
    let stamps: Vec<i64> = (0..served.header().n_valid)
        .map(|index| served.read_record(index).unwrap().ts_micros)
        .collect();
    let mut expected: Vec<i64> = held.iter().map(|bar| bar.ts_micros).collect();
    expected.remove(2);
    expected.push(added[0].ts_micros);
    assert_eq!(stamps, expected);
}

/// Runs `body` in a child of this test binary that runs `test` alone, so it
/// meets the process as a fresh one has it (no telemetry sink installed).
fn in_a_fresh_process(test: &str, body: impl FnOnce()) {
    const CHILD: &str = "BRUTEX_LS1_FRESH_PROCESS_CHILD";
    if std::env::var_os(CHILD).is_some() {
        body();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .output()
        .unwrap();
    let (stdout, stderr) = (
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let said = format!("child {test} failed or ran no test:\n{stdout}\n{stderr}");
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "{said}"
    );
}

/// **A RESTATE THAT RESTATED NOTHING WRITES NO `store.restate` LINE; ONE THAT
/// RESTATED WRITES ONE.** In a fresh process with its own sink: the line
/// names a restated month and nothing else (the event asked directly with a
/// month that had nothing new; the door's own restate as the control).
#[test]
fn a_restate_that_restated_nothing_says_nothing() {
    in_a_fresh_process(
        "repair::door_cover_tests::a_restate_that_restated_nothing_says_nothing",
        || {
            let log = Root::new("restate-log");
            let sink = telemetry::install(
                &telemetry::Config::new(&log.0).with_min_level(telemetry::Level::Trace),
            )
            .expect("a fresh process has no sink");
            let lines = || {
                telemetry::tail(
                    &log.0,
                    sink.keep_files(),
                    &telemetry::Query::last(64).from_target("store.restate"),
                )
                .records
                .len()
            };
            note_restated(
                month(),
                REASON,
                &MonthRestate::NothingNew {
                    ordinal: 0,
                    held_back: Vec::new(),
                },
            );
            assert_eq!(lines(), 0, "nothing restated, nothing said");
            let root = Root::new("restate-control");
            original(&root, &bars(0..3));
            assert!(matches!(
                restate_month(&root.0, month(), SYMBOL, &bars(3..4), RESTATE, REASON),
                Ok(MonthRestate::Restated { .. })
            ));
            assert_eq!(lines(), 1, "a restate says so once");
        },
    );
}

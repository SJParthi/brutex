//! IA-16's tests of the importer's failure kinds: `admit` as one ordered
//! table, `decisive()`'s non-hole-before-hole order, the store holes named
//! once and never read in a loop (FM-067, FM-095, on a CASH and an OPTION
//! contract-day each), the held-back set and the busy-month retry (FM-066,
//! IA-GR-18), the run's end on a full disk or a lost store (FM-085, FM-086,
//! FM-125), the day-close refusals (FM-038, FM-073), the repair reason's
//! gate (D-4889, critic X-1) and rule (IA-GR-42), and CMP-13's seam. Every
//! file, price and contract is invented (`gdfl_fixtures`); no vendor row is
//! quoted.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_fixtures::scratch;
use crate::gdfl_import::causes::{HOLE_REASONS, REPAIRABLE_HOLES};
use crate::gdfl_import::day::{Admit, Asks, KeyState, Skip, admit_rule, decisive};
use crate::ingest::FailKind;

/// The reason a repair-reason run of these tests carries.
const REASON: &str = "IA-16 test: land the day the month lacks";

/// A run of `kind` over `from..=to` from the tick store under `root` into
/// `store`, filtered to `only`, with `options`.
fn import_with(
    root: &Path,
    (kind, from, to): (ImportKind, Day, Day),
    (store, only): (&Path, &[String]),
    options: RunOptions<'_>,
) -> Result<Report, ImportRefusal> {
    let run = Run {
        kind,
        from,
        to,
        only,
        store_root: store,
    };
    match kind {
        ImportKind::Options => run_nfo_from(&NfoTickStore::new(&root.join("ts")), &run, options),
        _ => run_cm_from(&TickStore::new(&root.join("ts")), &run, options),
    }
}

/// [`import_with`] with no option set, and its report.
fn plain(root: &Path, kind: ImportKind, (from, to): (Day, Day), store: &Path) -> Report {
    import_with(root, (kind, from, to), (store, &[]), RunOptions::default()).unwrap()
}

/// [`import_with`] carrying [`REASON`].
fn repairing(root: &Path, kind: ImportKind, (from, to): (Day, Day), store: &Path) -> Report {
    let options = RunOptions {
        repair_reason: Some(REASON),
        ..RunOptions::default()
    };
    import_with(root, (kind, from, to), (store, &[]), options).unwrap()
}

/// The journal's whole text.
fn journal(store: &Path) -> String {
    std::fs::read_to_string(journal_path(store)).unwrap_or_default()
}

/// Every revision directory under `store`'s bars: a repair's footprint.
fn revisions(store: &Path) -> usize {
    tree(&store.join("bars"))
        .keys()
        .filter(|path| path.to_string_lossy().contains(".rev"))
        .count()
}

// ── admit, one ordered table (D-4885, D-4910, D-4912) ──────────────────────

/// Each row of the table, alone and against the rows before it: a done key
/// is skipped before its definition is asked, a restated key is read unless
/// the run asks to skip it, a repairable hole is read only with a repair
/// reason, a hole no repair fills is read by no run, and every other key
/// goes on to the calendar gate.
#[test]
fn admit_is_one_ordered_table_and_a_hole_is_read_only_when_asked() {
    let none = KeyState {
        done: false,
        with_refusals: false,
        restated: false,
        hole: None,
    };
    let every = [
        Asks::default(),
        Asks {
            retry_refused: true,
            ..Asks::default()
        },
        Asks {
            skip_restated: true,
            ..Asks::default()
        },
        Asks {
            repair_reason: true,
            ..Asks::default()
        },
        Asks {
            retry_refused: true,
            skip_restated: true,
            repair_reason: true,
        },
    ];
    for asks in every {
        // Row 6: nothing known of the key.
        assert_eq!(admit_rule(&none, asks), Admit::Read, "{asks:?}");
        // Row 2, before anything else the key could also be.
        let done = KeyState {
            done: true,
            restated: true,
            hole: Some("needs_month_generation"),
            ..none
        };
        assert_eq!(admit_rule(&done, asks), Admit::Skip(Skip::Done), "{asks:?}");
        let refusals = KeyState {
            with_refusals: true,
            ..done
        };
        let want = if asks.retry_refused {
            Admit::Read
        } else {
            Admit::Skip(Skip::Done)
        };
        assert_eq!(admit_rule(&refusals, asks), want, "{asks:?}");
        // Row 3.
        let restated = KeyState {
            restated: true,
            ..none
        };
        let want = if asks.skip_restated {
            Admit::Skip(Skip::Restated)
        } else {
            Admit::Read
        };
        assert_eq!(admit_rule(&restated, asks), want, "{asks:?}");
        // Rows 4a and 4b, every one of the five reasons.
        for reason in HOLE_REASONS {
            let hole = KeyState {
                hole: Some(reason),
                ..none
            };
            let repairable = REPAIRABLE_HOLES.contains(&reason);
            let want = match (repairable, asks.repair_reason) {
                (true, true) => Admit::Read,
                (true, false) => Admit::Skip(Skip::Hole(reason)),
                (false, _) => Admit::Skip(Skip::Unrepairable(reason)),
            };
            assert_eq!(admit_rule(&hole, asks), want, "{reason} {asks:?}");
            // A RESTATED DAY READ AGAIN AND CLOSED A HOLE (FM-095, IA-GR-30):
            // the hole is its last line and decides before row 3, so a
            // `bars_differ_held` day done under an older definition is read by
            // no run, a repair-reason run included (D-4912), and is never
            // skipped as `restated` either.
            let restated_hole = KeyState {
                restated: true,
                ..hole
            };
            assert_eq!(
                admit_rule(&restated_hole, asks),
                want,
                "restated then {reason} {asks:?}"
            );
        }
    }
    // The five reasons are the five kinds' codes, each once, and the three
    // a repair can fill are among them.
    assert_eq!(
        HOLE_REASONS,
        [
            "needs_month_generation",
            "month_superseded",
            "repair_refused",
            "bars_differ_held",
            "repair_impossible"
        ]
    );
    assert!(
        REPAIRABLE_HOLES
            .iter()
            .all(|one| HOLE_REASONS.contains(one))
    );
    for kind in [
        FailKind::NotHeld,
        FailKind::Superseded,
        FailKind::Differs,
        FailKind::RepairRefused,
        FailKind::RepairImpossible,
    ] {
        assert!(
            kind.is_hole() && HOLE_REASONS.contains(&kind.code()),
            "{kind:?}"
        );
    }
}

/// A key's state is read from its LAST line: a hole reason followed by a
/// `begin` (a later run was killed in it), by a `refused` or by a `done` is
/// no longer a hole, and an `incomplete` line with any other reason is none.
#[test]
fn a_hole_is_known_by_the_last_line_alone() {
    let root = scratch("ia16-keystate");
    let store = root.join("S");
    let key = |n: u8| format!("stocks 2024-04-0{n} *");
    let lines = [
        format!(
            "incomplete {} definition=3 files=1 stage=file reason=needs_month_generation run=7",
            key(1)
        ),
        format!(
            "incomplete {} definition=3 files=1 stage=file reason=bars_differ_held run=7",
            key(2)
        ),
        format!(
            "incomplete {} definition=3 files=1 stage=file reason=needs_month_generation run=7",
            key(3)
        ),
        format!("begin {}", key(3)),
        format!(
            "incomplete {} definition=3 files=1 stage=file reason=month_busy run=7",
            key(4)
        ),
        format!(
            "incomplete {} definition=3 files=1 stage=file reason=repair_refused run=7",
            key(5)
        ),
        format!(
            "refused {} definition=3 stage=vendor reason=day_not_held failures=0 run=8",
            key(5)
        ),
        format!(
            "incomplete {} definition=3 files=1 stage=file reason=repair_impossible run=7",
            key(6)
        ),
        format!("done {} definition=3 files=1 failures=0 run=8", key(6)),
    ];
    // FM-095's journal shapes (IA-16 repair 1): an OPTION day done under
    // definition 2 (no longer current for options) and a SHARE day of the
    // closing auction's era done under 3 (no longer current for a share
    // from 2026-08-03), each read again and closed `bars_differ_held`; an
    // option day done under 2 and then stopped; one done under 2 alone.
    let restated_lines = [
        "done options 2024-04-01 NIFTY definition=2 files=1 failures=0 run=5",
        "incomplete options 2024-04-01 NIFTY definition=4 files=1 stage=file \
         reason=bars_differ_held run=7",
        "done stocks 2026-08-04 * definition=3 files=1 failures=0 run=5",
        "incomplete stocks 2026-08-04 * definition=4 files=1 stage=file \
         reason=bars_differ_held run=7",
        "done options 2024-04-02 NIFTY definition=2 files=1 failures=0 run=5",
        "incomplete options 2024-04-02 NIFTY definition=4 files=1 stage=run reason=stopped run=7",
        "done options 2024-04-03 NIFTY definition=2 files=1 failures=0 run=5",
    ];
    let mut text = "run 7 source=zip\n".to_owned();
    for line in lines.iter().map(String::as_str).chain(restated_lines) {
        text.push_str(line);
        text.push('\n');
    }
    crate::gdfl_fixtures::put(&store, "imports/gdfl.journal", text.as_bytes());
    let journal = Journal::load(journal_path(&store)).unwrap();
    let hole = |n: u8| KeyState::of(&journal, &key(n)).hole;
    assert_eq!(hole(1), Some("needs_month_generation"));
    assert_eq!(hole(2), Some("bars_differ_held"));
    assert_eq!(hole(3), None, "a begin after it: the key is open");
    assert_eq!(hole(4), None, "a busy month is not a hole");
    assert_eq!(hole(5), None, "a refused line after it");
    assert_eq!(hole(6), None, "done after it");
    assert!(KeyState::of(&journal, &key(6)).done);
    assert_eq!(hole(9), None, "a key the journal never named");
    let state = |key: &str| KeyState::of(&journal, key);
    for key in ["options 2024-04-01 NIFTY", "stocks 2026-08-04 *"] {
        let held = state(key);
        assert_eq!(held.hole, Some("bars_differ_held"), "{key}");
        assert!(
            !held.restated,
            "{key}: its last line is the hole, not the done"
        );
        assert!(!held.done, "{key}");
        for asks in [
            Asks::default(),
            Asks {
                repair_reason: true,
                ..Asks::default()
            },
            Asks {
                skip_restated: true,
                ..Asks::default()
            },
        ] {
            assert_eq!(
                admit_rule(&held, asks),
                Admit::Skip(Skip::Unrepairable("bars_differ_held")),
                "{key} {asks:?}"
            );
        }
    }
    // Restated, then stopped: the `incomplete` line is row 6's, read by
    // every run, one that asks to skip restated days included.
    let stopped = state("options 2024-04-02 NIFTY");
    assert_eq!((stopped.hole, stopped.restated), (None, false));
    let skipping = Asks {
        skip_restated: true,
        ..Asks::default()
    };
    assert_eq!(admit_rule(&stopped, skipping), Admit::Read);
    // Done under 2 and nothing after it: row 3.
    let restated = state("options 2024-04-03 NIFTY");
    assert!(restated.restated && !restated.done && restated.hole.is_none());
    assert_eq!(admit_rule(&restated, Asks::default()), Admit::Read);
    assert_eq!(admit_rule(&restated, skipping), Admit::Skip(Skip::Restated));
    std::fs::remove_dir_all(root).unwrap();
}

// ── decisive(): every other open cause before a hole (round 4, BL-C) ───────

/// A day whose open failures include a hole and a busy month closes with the
/// busy month, whichever came first, so the next resume reads it; a day
/// whose every open failure is a hole closes with the first hole; a census
/// or a run cause still comes before both.
#[test]
fn decisive_takes_every_other_open_cause_before_a_hole() {
    let hole = staged_at(
        Cause::open(Stage::File, FailKind::NotHeld.code()),
        "BANKNIFTY",
    );
    let differs = staged_at(
        Cause::open(Stage::File, FailKind::Differs.code()),
        "FINNIFTY",
    );
    let busy = staged_at(
        Cause::open(Stage::File, FailKind::MonthBusy.code()),
        "NIFTY",
    );
    let vendor = staged_at(Cause::vendor("price_refused"), "SBIN");
    let run = staged_at(Cause::open(Stage::Run, "disk_full"), "file");
    assert!(hole.cause.is_hole() && differs.cause.is_hole() && !busy.cause.is_hole());
    for failures in [
        vec![hole.clone(), busy.clone()],
        vec![busy.clone(), hole.clone()],
        vec![vendor.clone(), hole.clone(), differs.clone(), busy.clone()],
    ] {
        assert_eq!(
            decisive(&failures).map(|one| one.cause.code),
            Some("month_busy"),
            "{failures:?}"
        );
        assert_eq!(close_of(&failures, true), Close::Incomplete(busy.cause));
    }
    let holes = [vendor.clone(), differs.clone(), hole.clone()];
    assert_eq!(
        decisive(&holes).map(|one| one.cause.code),
        Some("bars_differ_held")
    );
    assert_eq!(close_of(&holes, true), Close::Incomplete(differs.cause));
    let ended = [hole, busy, run.clone()];
    assert_eq!(close_of(&ended, true), Close::Incomplete(run.cause));
    // A closing cause alone still closes the day done with refusals.
    assert_eq!(
        close_of(&[vendor], true),
        Close::WithRefusals("price_refused")
    );
}

// ── FM-067: a passed day is a named hole, never read in a loop ─────────────

/// A month that holds days 1 and 3 refuses day 2 through the append door.
/// Without a repair reason that day is the hole `needs_month_generation`:
/// closed `incomplete stage=file`, never repaired unasked (no revision), and
/// every later run passes it unread, names it in `holes_in_range` and writes
/// nothing. A run that carries a repair reason lands it as a new generation
/// and closes it done. For a CASH (indices) day and an OPTION contract-day.
#[test]
fn a_passed_day_is_named_and_never_read_in_a_loop() {
    for kind in [ImportKind::Indices, ImportKind::Options] {
        let root = scratch(&format!("ia16-fm067-{}", kind.as_str()));
        world(&root, &[d1(), d2(), d3()]);
        let store = root.join("S");
        for on in [d1(), d3()] {
            let got = plain(&root, kind, (on, on), &store);
            assert!(got.failures.is_empty(), "{kind:?} {:?}", got.failures);
        }
        let first = plain(&root, kind, (d2(), d2()), &store);
        assert_eq!(first.closed_incomplete, vec![d2()], "{kind:?} {first:?}");
        assert_eq!(first.months_repaired, 0, "{kind:?}: never repaired unasked");
        assert_eq!(revisions(&store), 0, "{kind:?}");
        let line = line_of(&store, kind.as_str(), "2024-04-02");
        assert!(
            line.starts_with("incomplete ")
                && line.contains(" stage=file ")
                && line.contains(" reason=needs_month_generation"),
            "{kind:?}: {line}"
        );
        assert!(
            first
                .failures
                .iter()
                .all(|one| one.why.contains("needs_month_generation")),
            "{kind:?}: {:?}",
            first.failures
        );
        // Resume, three times: nothing read, nothing written, named each time.
        let (journal_before, bars_before) = (journal(&store), tree(&store.join("bars")));
        for _ in 0..3 {
            let again = plain(&root, kind, (d1(), d3()), &store);
            assert_eq!(again.files, 0, "{kind:?}: 0 files of the hole read");
            assert_eq!(again.seconds, 0, "{kind:?}");
            assert_eq!(again.holes_skipped, 1, "{kind:?} {again:?}");
            assert_eq!(again.days_skipped, 2, "{kind:?}");
            assert_eq!(
                again.holes_in_range,
                vec![(d2(), "needs_month_generation".to_owned())],
                "{kind:?}"
            );
            assert_eq!(journal(&store), journal_before, "{kind:?}: no line written");
            assert_eq!(tree(&store.join("bars")), bars_before, "{kind:?}");
            // ON THE RECEIPT (D-6900 keep item 2): the hole is never a day
            // already done, and the store's standing holes name it by kind
            // and reason, whatever range the run asked.
            assert_eq!(
                again.standing_holes,
                vec![(
                    kind.as_str().to_owned(),
                    "needs_month_generation".to_owned(),
                    1
                )],
                "{kind:?}"
            );
            assert!(again.holes_made.is_empty(), "{kind:?}: passed, not made");
            let outside = plain(&root, kind, (d3(), d3()), &store);
            assert_eq!(outside.standing_holes, again.standing_holes, "{kind:?}");
            assert!(outside.holes_in_range.is_empty(), "{kind:?}");
        }
        assert_eq!(
            first.holes_made,
            vec![(d2(), "needs_month_generation".to_owned())],
            "{kind:?}: the run that made the hole names it"
        );
        // Asked, with a reason: landed as a new generation, closed done.
        let asked = repairing(&root, kind, (d1(), d3()), &store);
        assert!(asked.failures.is_empty(), "{kind:?} {:?}", asked.failures);
        assert_eq!(asked.holes_skipped, 0, "{kind:?}");
        assert!(asked.months_repaired > 0, "{kind:?} {asked:?}");
        assert!(
            line_of(&store, kind.as_str(), "2024-04-02").starts_with("done "),
            "{kind:?}"
        );
        let repairs = std::fs::read_to_string(store.join("audit/gdfl.repairs")).unwrap();
        assert!(
            repairs.contains("IA-16"),
            "the operator's words first: {repairs}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// IA-GR-31's first half (round 3; D-4889, D-4890), as D-6247 left it (IA-16
/// repair 2, D-7131): a month served from a repair revision answers every
/// batch as its RAW form would. A later read of a day it holds is nothing
/// new and nothing refused. A day AFTER its last held bar is what the raw
/// month appends, so it is filed as the month's next successor generation
/// with no repair reason (`month_sealed` never reaches the journal): no
/// hole, and nothing read again. A day INSIDE its held range at seconds it
/// never held is the raw month's `needs_month_generation`, named once: a
/// later run without a repair reason reads 0 files of it and writes no
/// line, and one repair-reason run lands it, after which a further run
/// reads nothing. For an index and an option contract.
#[test]
fn a_repaired_month_answers_held_bars_and_names_new_ones_as_a_hole() {
    for kind in [ImportKind::Indices, ImportKind::Options] {
        // ── AFTER THE LAST HELD BAR: the next successor (D-6247) ──
        let root = scratch(&format!("ia16-gr31-{}", kind.as_str()));
        world(&root, &[d1(), d2(), d3(), d4()]);
        let store = root.join("S");
        for on in [d1(), d3()] {
            let got = plain(&root, kind, (on, on), &store);
            assert!(got.failures.is_empty(), "{kind:?} {:?}", got.failures);
        }
        let hole = plain(&root, kind, (d2(), d2()), &store);
        assert_eq!(hole.closed_incomplete, vec![d2()], "{kind:?} {hole:?}");
        let asked = repairing(&root, kind, (d2(), d2()), &store);
        assert!(asked.failures.is_empty(), "{kind:?} {:?}", asked.failures);
        assert!(asked.months_repaired > 0, "{kind:?} {asked:?}");
        // The repair's generation: the files under `bars` once it landed.
        let served: Vec<PathBuf> = tree(&store.join("bars")).into_keys().collect();

        // Held bars read again (by another key): nothing new, nothing refused.
        let only = ["NIFTY".to_owned()];
        let held_again = import_with(
            &root,
            (kind, d1(), d1()),
            (&store, &only),
            RunOptions::default(),
        )
        .unwrap();
        assert!(
            held_again.failures.is_empty(),
            "{kind:?}: {:?}",
            held_again.failures
        );
        assert_eq!(held_again.months_repaired, 0, "{kind:?}");
        assert_eq!(
            tree(&store.join("bars")).into_keys().collect::<Vec<_>>(),
            served,
            "{kind:?}: no generation and no file for held bars"
        );

        // A day after the revision's last held bar, with no repair reason:
        // its next successor generation, no hole.
        let later = plain(&root, kind, (d4(), d4()), &store);
        assert!(later.failures.is_empty(), "{kind:?} {:?}", later.failures);
        assert!(later.closed_incomplete.is_empty(), "{kind:?} {later:?}");
        assert!(later.holes_made.is_empty(), "{kind:?} {later:?}");
        assert!(later.months_repaired > 0, "{kind:?} {later:?}");
        assert!(
            line_of(&store, kind.as_str(), "2024-04-04").starts_with("done "),
            "{kind:?}"
        );
        let repairs = std::fs::read_to_string(store.join("audit").join("gdfl.repairs")).unwrap();
        assert!(
            repairs
                .lines()
                .any(|line| line.starts_with("repairing ") && line.contains("revision")),
            "{kind:?}: the successor is journaled before it is published: {repairs}"
        );
        assert!(!journal(&store).contains("reason=month_sealed"), "{kind:?}");
        assert!(!journal(&store).contains("reason=filing"), "{kind:?}");
        let before = (journal(&store), tree(&store.join("bars")));
        let settled = plain(&root, kind, (d1(), d4()), &store);
        assert_eq!(settled.files, 0, "{kind:?} {settled:?}");
        assert!(settled.holes_in_range.is_empty(), "{kind:?}");
        assert_eq!(
            (journal(&store), tree(&store.join("bars"))),
            before,
            "{kind:?}: no line, no bar"
        );
        std::fs::remove_dir_all(root).unwrap();

        // ── INSIDE THE HELD RANGE: the raw month's named hole ──
        let root = scratch(&format!("ia16-gr31-in-{}", kind.as_str()));
        world(&root, &[d1(), d2(), d3(), d4()]);
        let store = root.join("S");
        for on in [d1(), d4()] {
            let got = plain(&root, kind, (on, on), &store);
            assert!(got.failures.is_empty(), "{kind:?} {:?}", got.failures);
        }
        let hole = plain(&root, kind, (d2(), d2()), &store);
        assert_eq!(hole.closed_incomplete, vec![d2()], "{kind:?} {hole:?}");
        let asked = repairing(&root, kind, (d2(), d2()), &store);
        assert!(asked.failures.is_empty(), "{kind:?} {:?}", asked.failures);
        assert!(asked.months_repaired > 0, "{kind:?} {asked:?}");

        let inside = plain(&root, kind, (d3(), d3()), &store);
        assert_eq!(inside.closed_incomplete, vec![d3()], "{kind:?} {inside:?}");
        assert_eq!(inside.months_repaired, 0, "{kind:?}");
        let line = line_of(&store, kind.as_str(), "2024-04-03");
        assert!(
            line.starts_with("incomplete ") && line.contains(" reason=needs_month_generation"),
            "{kind:?}: {line}"
        );
        assert_eq!(
            inside.holes_made,
            vec![(d3(), "needs_month_generation".to_owned())],
            "{kind:?}"
        );
        let before = (journal(&store), tree(&store.join("bars")));
        let again = plain(&root, kind, (d1(), d4()), &store);
        assert_eq!(again.files, 0, "{kind:?}: 0 files of the hole read");
        assert_eq!(
            again.holes_in_range,
            vec![(d3(), "needs_month_generation".to_owned())],
            "{kind:?}"
        );
        assert_eq!(
            (journal(&store), tree(&store.join("bars"))),
            before,
            "{kind:?}: no line, no bar"
        );

        // One repair-reason run lands it; then nothing is read again.
        let filled = repairing(&root, kind, (d3(), d3()), &store);
        assert!(filled.failures.is_empty(), "{kind:?} {:?}", filled.failures);
        assert!(filled.months_repaired > 0, "{kind:?} {filled:?}");
        assert!(
            line_of(&store, kind.as_str(), "2024-04-03").starts_with("done "),
            "{kind:?}"
        );
        let settled = plain(&root, kind, (d1(), d4()), &store);
        assert_eq!(settled.files, 0, "{kind:?} {settled:?}");
        assert!(settled.holes_in_range.is_empty(), "{kind:?}");
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// **THE SEALED KIND AT THE IMPORT (SC-4 D-5904, D-6247; IA-16 repair 2,
/// D-7131).** Every month of 2024-04-01..02 imported and sealed, then the
/// late day 2024-04-03 imported by a run with NO repair reason: the append
/// door's `Sealed` refusal is handed to the back-fill door, which files the
/// day as each month's successor generation. No failure, no hole, no day
/// closed incomplete, never `filing` and never `month_sealed` on the
/// journal; a second late day (2024-04-04, the month now served from that
/// successor) is the next successor the same way; and a run over the whole
/// range afterwards reads nothing. For an index and an option contract.
#[test]
fn a_sealed_months_late_day_is_its_successor_never_a_hole_nor_filing() {
    for kind in [ImportKind::Indices, ImportKind::Options] {
        let root = scratch(&format!("ia16-sealed-{}", kind.as_str()));
        world(&root, &[d1(), d2(), d3(), d4()]);
        let store = root.join("S");
        let got = plain(&root, kind, (d1(), d2()), &store);
        assert!(got.failures.is_empty(), "{kind:?} {:?}", got.failures);
        let sealed = super::sealed::seal_every_month(&store);
        assert!(!sealed.is_empty(), "{kind:?}");

        for (on, iso) in [(d3(), "2024-04-03"), (d4(), "2024-04-04")] {
            let late = plain(&root, kind, (on, on), &store);
            assert!(
                late.failures.is_empty(),
                "{kind:?} {iso}: {:?}",
                late.failures
            );
            assert!(late.files > 0, "{kind:?} {iso}");
            assert!(
                late.closed_incomplete.is_empty(),
                "{kind:?} {iso}: {late:?}"
            );
            assert!(late.holes_made.is_empty(), "{kind:?} {iso}: {late:?}");
            assert_eq!(
                late.months_repaired, late.files,
                "{kind:?} {iso}: one successor per file"
            );
            let line = line_of(&store, kind.as_str(), iso);
            assert!(line.starts_with("done "), "{kind:?} {iso}: {line}");
        }
        let text = journal(&store);
        assert!(!text.contains("reason=filing"), "{kind:?}: {text}");
        assert!(!text.contains("reason=month_sealed"), "{kind:?}: {text}");
        let repairs = std::fs::read_to_string(store.join("audit").join("gdfl.repairs")).unwrap();
        assert!(
            repairs
                .lines()
                .any(|line| line.starts_with("repairing ") && line.contains("sealed-month")),
            "{kind:?}: {repairs}"
        );
        // No repair reason was given, so none is journaled as the operator's.
        assert!(!repairs.contains(REASON), "{kind:?}: {repairs}");

        let before = (journal(&store), tree(&store.join("bars")));
        let settled = plain(&root, kind, (d1(), d4()), &store);
        assert_eq!(settled.files, 0, "{kind:?} {settled:?}");
        assert!(settled.holes_in_range.is_empty(), "{kind:?}");
        assert_eq!(
            (journal(&store), tree(&store.join("bars"))),
            before,
            "{kind:?}: no line, no bar"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

// ── FM-095: a day read again with other bars is one named hole ─────────────

/// An NIFTY index file and an option contract file whose day the store
/// already holds, read again with OTHER bars at held seconds (what a changed
/// bar definition or a vendor's re-delivery hands the store; definition 3's
/// open interest is the option instance): the store keeps every held bar and
/// the day is the hole `bars_differ_held`, named once. No later run reads
/// it, with or without a repair reason, and the day's other files land.
#[test]
fn a_restated_day_with_other_bars_is_named_once() {
    let cases = [
        (
            ImportKind::Indices,
            Tree::Indices,
            "NIFTY 50.NSE_IDX",
            "NIFTY",
            (
                index_file("NIFTY 50.NSE_IDX", d1(), 22_000),
                index_file("NIFTY 50.NSE_IDX", d1(), 22_001),
            ),
        ),
        (
            ImportKind::Options,
            Tree::Options,
            "NIFTY04APR2422000CE",
            "NIFTY",
            (
                option_file("NIFTY04APR2422000CE", d1(), "101.5"),
                option_file("NIFTY04APR2422000CE", d1(), "102.5"),
            ),
        ),
    ];
    for (kind, tree_of, stem, symbol, (held, other)) in cases {
        let root = scratch(&format!("ia16-fm095-{}", kind.as_str()));
        let first = root.join("first");
        world_of(
            &first,
            &[(d1(), tree_of, vec![(entry_of(tree_of, d1(), stem), held)])],
        );
        let store = root.join("S");
        let only = [symbol.to_owned()];
        let got = import_with(
            &first,
            (kind, d1(), d1()),
            (&store, &only),
            RunOptions::default(),
        )
        .unwrap();
        assert!(got.failures.is_empty(), "{kind:?} {:?}", got.failures);
        let contract = (kind == ImportKind::Options).then(nifty_ce);
        let segment = if kind == ImportKind::Options {
            "FNO"
        } else {
            "INDEX"
        };
        let held_bars = bars(&store, segment, symbol, contract);
        assert!(!held_bars.is_empty(), "{kind:?}");
        // The day read again, whole, by another key, with other bars.
        let second = root.join("second");
        let mut entries = vec![(entry_of(tree_of, d1(), stem), other)];
        let neighbour = if kind == ImportKind::Options {
            (
                entry_of(Tree::Options, d1(), "BANKNIFTY10APR2447000CE"),
                option_file("BANKNIFTY10APR2447000CE", d1(), "88.5"),
            )
        } else {
            (
                entry_of(Tree::Indices, d1(), "NIFTY BANK.NSE_IDX"),
                index_file("NIFTY BANK.NSE_IDX", d1(), 47_000),
            )
        };
        entries.push(neighbour);
        world_of(&second, &[(d1(), tree_of, entries)]);
        let restated = plain(&second, kind, (d1(), d1()), &store);
        assert_eq!(
            restated.closed_incomplete,
            vec![d1()],
            "{kind:?} {restated:?}"
        );
        let line = line_of(&store, kind.as_str(), "2024-04-01");
        assert!(
            line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
            "{kind:?}: {line}"
        );
        assert_eq!(
            bars(&store, segment, symbol, contract),
            held_bars,
            "{kind:?}: a held bar is never changed"
        );
        assert!(
            restated.seconds_committed > 0,
            "{kind:?}: the neighbour landed"
        );
        assert_eq!(revisions(&store), 0, "{kind:?}");
        // Named once: no run reads it again, a repair-reason run included.
        let before = journal(&store);
        for options in [
            RunOptions::default(),
            RunOptions {
                repair_reason: Some(REASON),
                ..RunOptions::default()
            },
        ] {
            let again = import_with(&second, (kind, d1(), d1()), (&store, &[]), options).unwrap();
            assert_eq!(again.files, 0, "{kind:?}");
            assert_eq!(again.holes_unrepairable, 1, "{kind:?} {again:?}");
            assert_eq!(
                again.holes_in_range,
                vec![(d1(), "bars_differ_held".to_owned())]
            );
            assert_eq!(journal(&store), before, "{kind:?}: no line");
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    restated_option_day_read_again_with_other_bars_is_named_once();
}

/// FM-095's RESTATED instance (IA-16 repair 1; IA-GR-30, D-4873): an OPTION
/// contract-day whose last close is `done` under definition 2, no longer
/// current for options (definition 3's open interest), read again under its
/// OWN key by a run whose bars differ at held seconds. The re-read is named
/// restated once and closes the hole `bars_differ_held`; after it no run
/// reads the day, a repair-reason run included (D-4912), none writes a line,
/// and the day is counted as a hole passed, never as a day already done.
/// Before the repair, `admit` asked row 3 before the hole and read the day
/// again on every run. Every row is invented.
fn restated_option_day_read_again_with_other_bars_is_named_once() {
    let root = scratch("ia16-fm095-restated");
    let stem = "NIFTY04APR2422000CE";
    let (first, second) = (root.join("first"), root.join("second"));
    for (world, price) in [(&first, "101.5"), (&second, "102.5")] {
        world_of(
            world,
            &[(
                d1(),
                Tree::Options,
                vec![(
                    entry_of(Tree::Options, d1(), stem),
                    option_file(stem, d1(), price),
                )],
            )],
        );
    }
    let store = root.join("S");
    let only = ["NIFTY".to_owned()];
    let run = |world: &Path, options: RunOptions<'_>| {
        import_with(
            world,
            (ImportKind::Options, d1(), d1()),
            (&store, &only),
            options,
        )
        .unwrap()
    };
    let got = run(&first, RunOptions::default());
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    let held_bars = bars(&store, "FNO", "NIFTY", Some(nifty_ce()));
    assert!(!held_bars.is_empty());
    // The day's `done` as definition 2 wrote it, the line an options day
    // carried before definition 3.
    let current = format!("definition={BAR_DEFINITION}");
    let text: String = journal(&store)
        .lines()
        .map(|line| {
            let line = if line.starts_with("done options 2024-04-01 NIFTY ") {
                assert!(line.contains(&current), "{line}");
                line.replace(&current, "definition=2")
            } else {
                line.to_owned()
            };
            line + "\n"
        })
        .collect();
    std::fs::write(journal_path(&store), &text).unwrap();
    // Read again: restated, the store keeps every held bar, one named hole.
    let restated = run(&second, RunOptions::default());
    assert_eq!(restated.restated, vec![d1()], "{restated:?}");
    assert_eq!(restated.closed_incomplete, vec![d1()], "{restated:?}");
    let line = line_of(&store, "options", "2024-04-01");
    assert!(
        line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
        "{line}"
    );
    assert_eq!(
        bars(&store, "FNO", "NIFTY", Some(nifty_ce())),
        held_bars,
        "a held bar is never changed"
    );
    // Named once: no later run reads it, with or without a repair reason or
    // a request to skip restated days, and none writes a line.
    let before = journal(&store);
    for options in [
        RunOptions::default(),
        RunOptions {
            repair_reason: Some(REASON),
            ..RunOptions::default()
        },
        RunOptions {
            skip_restated: true,
            ..RunOptions::default()
        },
    ] {
        let again = run(&second, options);
        assert_eq!(again.files, 0, "{again:?}");
        assert_eq!(again.holes_unrepairable, 1, "{again:?}");
        assert_eq!(again.days_skipped, 0, "a hole is never a day already done");
        assert!(
            again.restated.is_empty() && again.resumed.is_empty(),
            "{again:?}"
        );
        assert!(again.failures.is_empty(), "{again:?}");
        assert_eq!(
            again.holes_in_range,
            vec![(d1(), "bars_differ_held".to_owned())]
        );
        assert_eq!(journal(&store), before, "no line");
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// FM-118, under the one-command cut (D-6900: no action item; the named hole
/// on the journal close and the receipt is the proof): an option contract-day
/// whose month holds its first half, offered again whole by a copy whose
/// bytes differ in that held half, is ONE named hole, `bars_differ_held`. Its
/// failure names the contract-month and the first offered bar the held bars
/// disagree with; the held half is kept, not a bar of the copy lands in that
/// month, and no later run (a repair-reason run included) reads it again or
/// writes a line, so it is never an incomplete day repeated every night.
/// Every row is invented.
#[test]
fn a_half_filled_day_offered_different_bytes_becomes_one_named_hole() {
    let ticker = "NIFTY04APR2422000CE";
    let quotes = ("1.00", "999.00");
    let rows = |price: &str, n: u32| -> Vec<(u32, String, u64, u64)> {
        (0..n)
            .map(|s| (NINE_FIFTEEN + s, price.to_owned(), 25, 1_000 + u64::from(s)))
            .collect()
    };
    let file = |rows: &[(u32, String, u64, u64)]| {
        let rows: Vec<(u32, &str, u64, u64)> = rows
            .iter()
            .map(|(sod, ltp, ltq, oi)| (*sod, ltp.as_str(), *ltq, *oi))
            .collect();
        nfo_csv(ticker, d1(), quotes, &rows)
    };
    let root = scratch("ia16-fm118");
    let half = root.join("half");
    world_of(
        &half,
        &[(
            d1(),
            Tree::Options,
            vec![(
                entry_of(Tree::Options, d1(), ticker),
                file(&rows("101.50", 30)),
            )],
        )],
    );
    let store = root.join("S");
    // The half lands under its own key (a filtered run), so the copy is read
    // again, whole, by another key, as FM-095's day is.
    let only = ["NIFTY".to_owned()];
    let first = import_with(
        &half,
        (ImportKind::Options, d1(), d1()),
        (&store, &only),
        RunOptions::default(),
    )
    .unwrap();
    assert!(first.failures.is_empty(), "{:?}", first.failures);
    let held = bars(&store, "FNO", "NIFTY", Some(nifty_ce()));
    assert_eq!(held.len(), 30, "the first half of the day is held");

    let whole = root.join("whole");
    world_of(
        &whole,
        &[(
            d1(),
            Tree::Options,
            vec![(
                entry_of(Tree::Options, d1(), ticker),
                file(&rows("102.50", 60)),
            )],
        )],
    );
    let offered = plain(&whole, ImportKind::Options, (d1(), d1()), &store);
    assert_eq!(offered.closed_incomplete, vec![d1()], "{offered:?}");
    let line = line_of(&store, "options", "2024-04-01");
    assert!(
        line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
        "{line}"
    );
    assert_eq!(offered.failures.len(), 1, "{:?}", offered.failures);
    let failure = &offered.failures[0];
    assert!(failure.instrument.contains(ticker), "{failure:?}");
    assert_eq!(failure.month, d1().year_month().ok(), "{failure:?}");
    let first_offered = (i64::from(d1().days_from_epoch()) * 86_400 + i64::from(NINE_FIFTEEN)
        - IST_OFFSET_SECS)
        * 1_000_000;
    assert!(
        failure.why.contains(&first_offered.to_string()) && failure.why.contains("2024-04"),
        "the contract-month and the first bar the held bars disagree with: {}",
        failure.why
    );
    assert_eq!(
        bars(&store, "FNO", "NIFTY", Some(nifty_ce())),
        held,
        "the held half is kept, nothing of the copy lands"
    );
    assert_eq!(revisions(&store), 0);

    let before = journal(&store);
    for options in [
        RunOptions::default(),
        RunOptions {
            repair_reason: Some(REASON),
            ..RunOptions::default()
        },
    ] {
        let again = import_with(
            &whole,
            (ImportKind::Options, d1(), d1()),
            (&store, &[]),
            options,
        )
        .unwrap();
        assert_eq!(again.files, 0, "{again:?}");
        assert_eq!(
            again.holes_in_range,
            vec![(d1(), "bars_differ_held".to_owned())]
        );
        assert_eq!(journal(&store), before, "never read again, no new line");
    }
    std::fs::remove_dir_all(root).unwrap();
}

// ── FM-066, IA-GR-18: a busy month is retried; a failure never strands D ────

/// The NIFTY month a reader holds open for the whole run: day 2's NIFTY
/// file is asked again 20 times (2 s), then fails `month_busy`, and day 3's
/// NIFTY file is held back unread (`held_back`), so the month never takes
/// day 3 while day 2 is owed. BANKNIFTY lands both days. Once the reader is
/// gone a resume lands day 2 and then day 3 through the append door: no
/// revision, the same bars an in-order import holds.
#[test]
fn a_busy_month_or_a_bad_copy_never_strands_an_earlier_day() {
    let root = scratch("ia16-held-back");
    world(&root, &[d1(), d2(), d3()]);
    let store = root.join("S");
    let whole = root.join("W");
    plain(&root, ImportKind::Indices, (d1(), d3()), &whole);
    plain(&root, ImportKind::Indices, (d1(), d1()), &store);
    let nifty = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: d1().year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    let reader = BarFile::open_existing(&store, nifty, filing::symbol_id_of("NIFTY")).unwrap();
    let busy = plain(&root, ImportKind::Indices, (d2(), d3()), &store);
    drop(reader);
    assert_eq!(busy.closed_incomplete, vec![d2(), d3()], "{busy:?}");
    assert!(line_of(&store, "indices", "2024-04-02").contains(" reason=month_busy"));
    assert!(line_of(&store, "indices", "2024-04-03").contains(" reason=held_back"));
    assert_eq!(busy.files_held_back, 1, "day 3's NIFTY file, unread");
    assert_eq!(busy.month_busy_retries, MONTH_BUSY_RETRY.0 - 1);
    let held = bars(&store, "INDEX", "NIFTY", None);
    let day_one = i64::from(d1().days_from_epoch());
    assert!(
        held.iter()
            .all(|bar| bar.ts_micros.div_euclid(86_400_000_000) == day_one),
        "the month took no later day while day 2 was owed"
    );
    let resumed = plain(&root, ImportKind::Indices, (d2(), d3()), &store);
    assert!(resumed.failures.is_empty(), "{:?}", resumed.failures);
    assert_eq!(resumed.months_repaired, 0);
    assert_eq!(revisions(&store), 0);
    assert_eq!(
        bars(&store, "INDEX", "NIFTY", None),
        bars(&whole, "INDEX", "NIFTY", None)
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A reader that lets the month go within the retry: the file lands on a
/// later attempt, nothing fails, and the retries are counted.
#[test]
fn a_month_held_for_a_moment_lands_on_a_later_attempt() {
    let root = scratch("ia16-busy-moment");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    plain(&root, ImportKind::Indices, (d1(), d1()), &store);
    let nifty = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: d1().year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    let reader = BarFile::open_existing(&store, nifty, filing::symbol_id_of("NIFTY")).unwrap();
    let letting_go = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(450));
        drop(reader);
    });
    let got = plain(&root, ImportKind::Indices, (d2(), d2()), &store);
    letting_go.join().unwrap();
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert!(got.month_busy_retries >= 1, "{got:?}");
    assert!(got.month_busy_retries < MONTH_BUSY_RETRY.0);
    assert!(line_of(&store, "indices", "2024-04-02").starts_with("done "));
    std::fs::remove_dir_all(root).unwrap();
}

/// The day of an auction record: the UTC date of its print, which is the IST
/// date for a print inside the session.
fn auc_days(store: &Path, share: &str, on: Day) -> Vec<i64> {
    let key =
        brutex_core::instrument::InstrumentKey::cash(brutex_core::instrument::Exchange::Nse, share)
            .unwrap();
    let path = crate::gdfl_cm::auction::path(store, &key, on).unwrap();
    crate::gdfl_cm::auction::read(&path)
        .unwrap()
        .map(|sidecar| {
            sidecar
                .records
                .iter()
                .map(|record| record.t_micros.div_euclid(86_400_000_000))
                .collect()
        })
        .unwrap_or_default()
}

/// FM-099's second half (IA-GR-18, round 5 SF-11): THE HELD-BACK CHECK RUNS
/// BEFORE THE `.auc` APPEND. A CASH share of the closing auction's era whose
/// day D failed (here its month stayed busy past the retry, after D's prints
/// had gone to the sidecar) appends no auction record on D+1: the sidecar
/// still ends at D, so D's next append is never refused as an older day after
/// a later one, and a resume lands D and then D+1, prints and bars alike, the
/// bytes of a run that never failed. Every price, ISIN row and file is
/// invented.
#[test]
fn a_held_back_file_appends_no_auction_record() {
    use std::fmt::Write as _;
    let days = [
        Day::new(2026, 9, 22).unwrap(),
        Day::new(2026, 9, 23).unwrap(),
        Day::new(2026, 9, 24).unwrap(),
    ];
    let (d0, d, next) = (days[0], days[1], days[2]);
    let root = scratch("ia16-auc-held-back");
    let share = "RELIANCE";
    let stem = format!("{share}.NSE");
    let plan: Vec<DayPlan> = days
        .iter()
        .zip(0_u32..)
        .map(|(&on, n)| {
            let mut rows: Vec<String> = (0..6)
                .map(|i| {
                    quoted_row(
                        &stem,
                        on,
                        hms_at(15, 5, 0) + i * 50,
                        &format!("{}.{i}5", 4_100 + n),
                        ("4000.00", "4999.90"),
                        (3, 0),
                    )
                })
                .collect();
            rows.push(quoted_row(
                &stem,
                on,
                hms_at(15, 28, 7),
                &format!("{}.65", 4_101 + n),
                ("0", "0.00"),
                (98_765, 0),
            ));
            (
                on,
                Tree::Stocks,
                vec![(entry_of(Tree::Stocks, on, &stem), csv(&rows))],
            )
        })
        .collect();
    world_of(&root, &plan);
    let at = brutex_core::universe::FNO_INDEX.position(share).unwrap();
    let isin = brutex_core::universe::FNO_UNDERLYINGS_ISIN[at];
    let mut master = String::from("FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n");
    writeln!(master, "2000,{share},EQ,{isin},1").unwrap();
    let masters = root.join("masters");
    for on in days {
        crate::cash_session_cache::install(&masters, on, &gzip(&master)).unwrap();
    }
    let scope = std::sync::Arc::new(
        crate::gdfl_cm::CmScope::stored(std::iter::empty::<&str>())
            .unwrap()
            .with_masters(masters),
    );
    let source = TickStore::new(&root.join("ts")).with_scope(scope);
    let stocks = |store: &Path, (from, to): (Day, Day)| {
        let run = Run {
            kind: ImportKind::Stocks,
            from,
            to,
            only: &[],
            store_root: store,
        };
        run_cm_from(&source, &run, RunOptions::default()).unwrap()
    };
    let utc = |on: Day| i64::from(on.days_from_epoch());

    // The reference: one run that never failed.
    let whole = root.join("W");
    let reference = stocks(&whole, (d0, next));
    assert!(reference.failures.is_empty(), "{:?}", reference.failures);
    assert_eq!(
        auc_days(&whole, share, d0),
        vec![utc(d0), utc(d), utc(next)],
        "one print a day in the reference"
    );

    // D0 lands; then a reader holds the share's month for the whole run.
    let store = root.join("S");
    assert!(stocks(&store, (d0, d0)).failures.is_empty());
    let month = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "CASH",
        symbol: share,
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: d.year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    let reader = BarFile::open_existing(&store, month, filing::symbol_id_of(share)).unwrap();
    let busy = stocks(&store, (d, next));
    drop(reader);
    assert_eq!(busy.closed_incomplete, vec![d], "{busy:?}");
    assert_eq!(
        busy.closed_refused,
        vec![next],
        "none of D+1's files was read clean: {busy:?}"
    );
    assert!(
        busy.failures
            .iter()
            .any(|failure| failure.instrument == "2026-09-24 RELIANCE 2026-09"),
        "the held-back month named once, under its day: {:?}",
        busy.failures
    );
    assert!(line_of(&store, "stocks", "2026-09-23").contains(" reason=month_busy"));
    assert!(line_of(&store, "stocks", "2026-09-24").contains(" reason=held_back"));
    assert_eq!(busy.files_held_back, 1, "D+1's file, never read");
    assert_eq!(
        auc_days(&store, share, d),
        vec![utc(d0), utc(d)],
        "D+1's print never reached the sidecar while D was owed"
    );

    // The resume: D's prints are already there (written 0), its bars land,
    // and only then D+1's print and bars.
    let resumed = stocks(&store, (d, next));
    assert!(resumed.failures.is_empty(), "{:?}", resumed.failures);
    assert!(line_of(&store, "stocks", "2026-09-23").starts_with("done "));
    assert!(line_of(&store, "stocks", "2026-09-24").starts_with("done "));
    assert_eq!(auc_days(&store, share, d), vec![utc(d0), utc(d), utc(next)]);
    let sidecar = |store: &Path| {
        let key = brutex_core::instrument::InstrumentKey::cash(
            brutex_core::instrument::Exchange::Nse,
            share,
        )
        .unwrap();
        std::fs::read(crate::gdfl_cm::auction::path(store, &key, d).unwrap()).unwrap()
    };
    assert_eq!(sidecar(&store), sidecar(&whole), "the sidecar's bytes");
    let month_bars = |store: &Path| -> Vec<Bar> {
        let file = BarFile::open_existing(store, month, filing::symbol_id_of(share)).unwrap();
        (0..file.header().n_valid)
            .map(|i| file.read_record(i).unwrap())
            .collect()
    };
    let landed = month_bars(&store);
    assert!(!landed.is_empty());
    assert_eq!(
        landed,
        month_bars(&whole),
        "the bars of a run that never failed"
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ── FM-085, FM-086, FM-125: the run's end ──────────────────────────────────

/// The disk fills on day 1's NIFTY file of a three-day run: no further file
/// of the day is written, the day closes `incomplete reason=disk_full`, the
/// run ends `disk_full`, and days 2 and 3 have no `begin`. A resume once
/// there is room reads day 1 again and converges.
#[test]
fn a_full_disk_ends_the_run_after_the_current_file() {
    let root = scratch("ia16-disk-full");
    world(&root, &[d1(), d2(), d3()]);
    let store = root.join("S");
    std::fs::create_dir_all(&store).unwrap();
    filing::inject::plant(&store, "NIFTY 50.NSE_IDX.csv", FailKind::DiskFull, false);
    let options = RunOptions {
        workers: Workers::new(1).unwrap(),
        ..RunOptions::default()
    };
    let ended = import_with(
        &root,
        (ImportKind::Indices, d1(), d3()),
        (&store, &[]),
        options,
    )
    .unwrap_err();
    assert_eq!(ended.code(), "disk_full", "{ended}");
    let done = ended.report().unwrap();
    assert_eq!(done.closed_incomplete, vec![d1()]);
    let line = line_of(&store, "indices", "2024-04-01");
    assert!(line.contains(" reason=disk_full"), "{line}");
    let text = journal(&store);
    assert!(!text.contains("begin indices 2024-04-02"), "{text}");
    assert!(!text.contains("begin indices 2024-04-03"), "{text}");
    let resumed = plain(&root, ImportKind::Indices, (d1(), d3()), &store);
    assert!(resumed.failures.is_empty(), "{:?}", resumed.failures);
    assert_eq!(resumed.resumed, vec![d1()]);
    std::fs::remove_dir_all(root).unwrap();
}

/// The store's root goes away on day 1's NIFTY file (renamed aside, as an
/// unplugged volume leaves its mount point): the loss probe names it, the
/// run ends `store_lost` with nothing more written (no flush, no close, no
/// end line), and no directory is made at the root's path.
#[test]
fn a_lost_store_ends_the_run_and_no_writer_recreates_its_root() {
    let root = scratch("ia16-store-lost");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    std::fs::create_dir_all(&store).unwrap();
    filing::inject::plant(
        &store,
        "NIFTY 50.NSE_IDX.csv",
        FailKind::Io {
            kind: std::io::ErrorKind::NotFound,
            publication_may_be_visible: false,
        },
        true,
    );
    let options = RunOptions {
        workers: Workers::new(1).unwrap(),
        run: Some(41),
        ..RunOptions::default()
    };
    let ended = import_with(
        &root,
        (ImportKind::Indices, d1(), d2()),
        (&store, &[]),
        options,
    )
    .unwrap_err();
    assert_eq!(ended.code(), "store_lost", "{ended}");
    assert!(ended.to_string().contains("went away"), "{ended}");
    assert!(!store.exists(), "no writer recreated the root");
    let gone = root.join("S.gone");
    let text = std::fs::read_to_string(journal_path(&gone)).unwrap();
    assert!(text.contains("begin indices 2024-04-01"), "{text}");
    assert!(
        !text
            .lines()
            .any(|line| !line.starts_with("begin ") && line.contains("indices 2024-04-01")),
        "the day stays begun, never closed: {text}"
    );
    assert!(!text.contains("2024-04-02"), "no later day: {text}");
    assert!(
        !text.contains(" ended="),
        "no end line at a gone root: {text}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Swaps the store's root for an empty directory once `after` has passed:
/// the volume unplugged and mounted back, or a folder left at its mount
/// point, between two days of one run (RA-284).
#[derive(Debug)]
struct SwapAfter {
    store: PathBuf,
    after: Day,
}

impl Observer for SwapAfter {
    fn begun(&self, _kind: ImportKind, _run: u64) {}
    fn day_begun(&self, _kind: ImportKind, _day: Day, _at: (usize, usize)) {}
    fn day_passed(&self, _kind: ImportKind, day: Day, _report: &Report) {
        if day == self.after {
            let mut gone = self.store.as_os_str().to_owned();
            gone.push(".gone");
            std::fs::rename(&self.store, &gone).unwrap();
            std::fs::create_dir(&self.store).unwrap();
        }
    }
    fn finished(&self, _kind: ImportKind, _end: Result<&Report, &ImportRefusal>) {}
}

/// Every entry under `dir`, recursively: what a writer left there.
fn entries_under(dir: &Path) -> usize {
    std::fs::read_dir(dir).map_or(0, |all| {
        all.map(|entry| {
            let path = entry.unwrap().path();
            1 + if path.is_dir() {
                entries_under(&path)
            } else {
                0
            }
        })
        .sum()
    })
}

/// RA-284 (D-4852, FM-125): a store root that came back as ANOTHER directory
/// (its device or inode number is not the one the run began on) is lost as
/// surely as a root that is gone. Between two days, the root checked before
/// day 2 ends the run `store_lost` before day 2 is begun; inside a day, the
/// probe after a refused write ends it with nothing more written. Either way
/// the folder now at the root's path takes no line, no bar and no end line,
/// and the old root's journal keeps what landed before the swap.
#[test]
fn a_store_root_that_came_back_as_another_directory_is_store_lost() {
    // Between two days.
    let root = scratch("ia16-store-swapped-between");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    std::fs::create_dir_all(&store).unwrap();
    let swap = SwapAfter {
        store: store.clone(),
        after: d1(),
    };
    let options = RunOptions {
        workers: Workers::new(1).unwrap(),
        run: Some(43),
        observer: Some(&swap),
        ..RunOptions::default()
    };
    let ended = import_with(
        &root,
        (ImportKind::Indices, d1(), d2()),
        (&store, &[]),
        options,
    )
    .unwrap_err();
    assert_eq!(ended.code(), "store_lost", "{ended}");
    let said = ended.to_string();
    assert!(
        said.contains("another directory than the run began on"),
        "{said}"
    );
    assert!(said.contains("2024-04-02 was not begun"), "{said}");
    assert_eq!(
        entries_under(&store),
        0,
        "the folder at the root's path took nothing"
    );
    let text = std::fs::read_to_string(journal_path(&root.join("S.gone"))).unwrap();
    assert!(
        text.lines()
            .any(|line| !line.starts_with("begin ") && line.contains("indices 2024-04-01")),
        "day 1 closed in the store it began in: {text}"
    );
    assert!(!text.contains("2024-04-02"), "day 2 never begun: {text}");
    assert!(!text.contains(" ended="), "no end line anywhere: {text}");
    std::fs::remove_dir_all(root).unwrap();

    // Inside a day: the refused write's probe.
    let root = scratch("ia16-store-swapped-within");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    std::fs::create_dir_all(&store).unwrap();
    filing::inject::plant_swapped(
        &store,
        "NIFTY 50.NSE_IDX.csv",
        FailKind::Io {
            kind: std::io::ErrorKind::NotFound,
            publication_may_be_visible: false,
        },
    );
    let options = RunOptions {
        workers: Workers::new(1).unwrap(),
        run: Some(44),
        ..RunOptions::default()
    };
    let ended = import_with(
        &root,
        (ImportKind::Indices, d1(), d2()),
        (&store, &[]),
        options,
    )
    .unwrap_err();
    assert_eq!(ended.code(), "store_lost", "{ended}");
    assert!(
        ended
            .to_string()
            .contains("another directory than the run began on"),
        "{ended}"
    );
    assert_eq!(
        entries_under(&store),
        0,
        "the folder at the root's path took nothing"
    );
    let text = std::fs::read_to_string(journal_path(&root.join("S.gone"))).unwrap();
    assert!(text.contains("begin indices 2024-04-01"), "{text}");
    assert!(!text.contains("2024-04-02"), "no later day: {text}");
    std::fs::remove_dir_all(root).unwrap();
}

/// Moves the store's root aside when `at` begins (after the day's root
/// check, before any of its writes), leaving nothing at its path, or an
/// empty directory when `swap`: the volume unplugged mid-run, or unplugged
/// and mounted back (FM-086, FM-125; IA-16 repair 1).
#[derive(Debug)]
struct LoseAt {
    store: PathBuf,
    at: Day,
    swap: bool,
}

impl Observer for LoseAt {
    fn begun(&self, _kind: ImportKind, _run: u64) {}
    fn day_begun(&self, _kind: ImportKind, day: Day, _at: (usize, usize)) {
        if day == self.at {
            let mut gone = self.store.as_os_str().to_owned();
            gone.push(".gone");
            std::fs::rename(&self.store, &gone).unwrap();
            if self.swap {
                std::fs::create_dir(&self.store).unwrap();
            }
        }
    }
    fn day_passed(&self, _kind: ImportKind, _day: Day, _report: &Report) {}
    fn finished(&self, _kind: ImportKind, _end: Result<&Report, &ImportRefusal>) {}
}

/// FM-125, FM-086, IA16-09, D-4852 AT THE RUN LEVEL (IA-16 repair 1): a store
/// root that goes away INSIDE a run, before any write of a day and with no
/// refused write to probe, ends the run `store_lost` on that day, on the
/// DEFAULT worker count, for an indices run and an options run alike. No
/// writer makes the root again (the journal, the day and lineage ledgers and
/// the progress record refused a missing root only after the repair: they
/// made it with `create_dir_all`, and the rest of the day landed in it), and
/// a root that came back as an empty folder takes no line, no ledger and no
/// bar: every file is probed before its first write. The day is never
/// begun, so a resume reads it whole.
#[test]
fn a_root_lost_inside_a_run_is_never_made_again_and_takes_nothing() {
    for (kind, swap) in [
        (ImportKind::Indices, false),
        (ImportKind::Options, false),
        (ImportKind::Indices, true),
        (ImportKind::Options, true),
    ] {
        let label = format!("{}-{swap}", kind.as_str());
        let root = scratch(&format!("ia16-lost-in-run-{label}"));
        world(&root, &[d1(), d2()]);
        let store = root.join("S");
        let lose = LoseAt {
            store: store.clone(),
            at: d2(),
            swap,
        };
        let options = RunOptions {
            run: Some(46),
            observer: Some(&lose),
            ..RunOptions::default()
        };
        assert!(options.workers.get() > 1, "the default worker count");
        let ended = import_with(&root, (kind, d1(), d2()), (&store, &[]), options).unwrap_err();
        assert_eq!(ended.code(), "store_lost", "{label}: {ended}");
        if swap {
            assert_eq!(
                entries_under(&store),
                0,
                "{label}: the folder at the root's path took nothing"
            );
        } else {
            assert!(!store.exists(), "{label}: no writer made the root again");
        }
        let text = std::fs::read_to_string(journal_path(&root.join("S.gone"))).unwrap();
        let first = format!("{} 2024-04-01", kind.as_str());
        assert!(
            text.lines()
                .any(|line| line.starts_with("done ") && line.contains(&first)),
            "{label}: day 1 closed in the store it began in: {text}"
        );
        assert!(
            !text.contains("2024-04-02"),
            "{label}: day 2 never begun: {text}"
        );
        assert!(!text.contains(" ended="), "{label}: no end line: {text}");
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// FM-086's durability half (IA-16 repair 1; critic IA-16-5, D-4901): a
/// root swapped for an empty folder after the day's last file LANDED, with
/// no refused write, leaves the durability barrier syncing directories that
/// are not there. Its refusal is loss-probed: the run ends `store_lost`,
/// naming the root that changed, never `durability`, and the new folder
/// takes no ledger line, census or closing line.
#[test]
fn a_durability_refusal_at_a_lost_root_is_store_lost() {
    let root = scratch("ia16-lost-at-barrier");
    world(&root, &[d1()]);
    let store = root.join("S");
    std::fs::create_dir_all(&store).unwrap();
    filing::inject::plant_swap_after(&store, "NIFTY 50.NSE_IDX.csv");
    let only = ["NIFTY".to_owned()];
    let options = RunOptions {
        workers: Workers::new(1).unwrap(),
        run: Some(47),
        ..RunOptions::default()
    };
    let ended = import_with(
        &root,
        (ImportKind::Indices, d1(), d1()),
        (&store, &only),
        options,
    )
    .unwrap_err();
    assert_eq!(ended.code(), "store_lost", "{ended}");
    let said = ended.to_string();
    assert!(said.contains("could not be made durable"), "{said}");
    assert!(
        said.contains("another directory than the run began on"),
        "{said}"
    );
    assert_eq!(entries_under(&store), 0, "the new folder took nothing");
    let text = std::fs::read_to_string(journal_path(&root.join("S.gone"))).unwrap();
    assert!(text.contains("begin indices 2024-04-01 NIFTY"), "{text}");
    assert!(
        !text
            .lines()
            .any(|line| !line.starts_with("begin ") && line.contains("indices 2024-04-01")),
        "the day stays begun: {text}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A run over a store root whose PARENT is not there (a volume that is not
/// mounted) is refused `store_lost` before anything is read or written, and
/// makes neither the root nor its parent (FM-125; IA-16 repair 1): until
/// the repair the journal's `create_dir_all` made both.
#[test]
fn a_store_on_a_missing_volume_is_refused_and_nothing_is_made() {
    let root = scratch("ia16-no-volume");
    world(&root, &[d1()]);
    let volume = root.join("volume");
    let store = volume.join("S");
    let ended = import_with(
        &root,
        (ImportKind::Indices, d1(), d1()),
        (&store, &[]),
        RunOptions::default(),
    )
    .unwrap_err();
    assert_eq!(ended.code(), "store_lost", "{ended}");
    assert!(ended.to_string().contains("not mounted"), "{ended}");
    assert!(!volume.exists(), "neither the root nor its parent was made");
    std::fs::remove_dir_all(root).unwrap();
}

/// A run over a store whose root does not exist yet makes it, learns its
/// identity before the next day, and is never told its own root is lost
/// (RA-284's other side): a fresh store runs to its end line.
#[test]
fn a_fresh_store_learns_its_root_and_is_never_lost() {
    let root = scratch("ia16-store-fresh");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    assert!(!store.exists());
    let options = RunOptions {
        workers: Workers::new(1).unwrap(),
        run: Some(45),
        ..RunOptions::default()
    };
    let report = import_with(
        &root,
        (ImportKind::Indices, d1(), d2()),
        (&store, &[]),
        options,
    )
    .unwrap();
    assert_eq!(report.failures_total, 0, "{report:?}");
    let text = journal(&store);
    assert!(text.contains(" ended="), "the run's end line: {text}");
    assert!(text.contains("indices 2024-04-02"), "{text}");
    std::fs::remove_dir_all(root).unwrap();
}

/// **Bench C-GR-11: THE LOSS PROBES ARE ONE `metadata` CALL EACH** (round 4,
/// SF-19; D-4852, RA-284). 200 probes of a root that answers (this crate's
/// own directory, on whatever volume the worktree is on: `WD_BLACK` on the
/// operator's Mac) against the identity recorded for it, then 200 of a root
/// that is gone. Printed as `C-GR-11 p50= p99= max= n=`. The store's probe
/// and the source's (`source_lost`) are the same call on another root.
#[test]
fn the_loss_probes_cost_one_metadata_call_each() {
    fn pct(mut all: Vec<std::time::Duration>) -> [std::time::Duration; 3] {
        all.sort_unstable();
        let at = |q: usize| all[(all.len() * q / 100).min(all.len() - 1)];
        [at(50), at(99), *all.last().unwrap()]
    }
    fn timed(
        root: &Path,
        at_start: Option<filing::StoreId>,
        lost: bool,
    ) -> Vec<std::time::Duration> {
        (0..200)
            .map(|_| {
                let began = std::time::Instant::now();
                let said = filing::store_root_lost(root, at_start);
                let took = began.elapsed();
                assert_eq!(said.is_some(), lost, "{said:?}");
                took
            })
            .collect()
    }
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let id = filing::store_identity(here);
    assert!(id.is_some(), "the crate's directory has an identity");
    let gone = here.join("no-such-store-root-c-gr-11");
    assert_eq!(filing::store_identity(&gone), None);
    let [h50, h99, hmax] = pct(timed(here, id, false));
    let [g50, g99, gmax] = pct(timed(&gone, id, true));
    println!(
        "C-GR-11 held root p50={h50:?} p99={h99:?} max={hmax:?} n=200 on {}",
        here.display()
    );
    println!("C-GR-11 gone root p50={g50:?} p99={g99:?} max={gmax:?} n=200");
    // Another directory is lost even though it answers.
    let other = filing::store_identity(&here.join("src"));
    assert!(filing::store_root_lost(&here.join("src"), id).is_some());
    assert_ne!(other, id);
}

/// The bytes of a zip, read positionally, until the volume under them goes:
/// from then every read fails as an unplugged drive's does (EIO).
struct Unplugged {
    bytes: Vec<u8>,
    gone: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl crate::gdfl_archive::ReadAt for Unplugged {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
        if self.gone.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(std::io::Error::from_raw_os_error(5));
        }
        let at = usize::try_from(offset).unwrap();
        let part = self
            .bytes
            .get(at..at + buf.len())
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?;
        buf.copy_from_slice(part);
        Ok(())
    }
}

/// FM-086's source half (IA-GR-09, CONC-013, INFRA-024; D-4852; the store
/// half is [`a_lost_store_ends_the_run_and_no_writer_recreates_its_root`]):
/// for each of the four sources (the capital-market and the options tick
/// stores, the capital-market outer zip read through a `ReadAt` that fails
/// EIO, and the directory of yearly options zips), a source root gone after
/// day 1 with the store still there ends the next run `source_lost` at day 2
/// by the probe's words naming the root: no line for day 2 or day 3 (never
/// `refused day_not_held` for a drive merely unplugged), no bar written; once
/// the root is back a resume lands both days as a run that never lost it.
#[test]
fn a_lost_source_or_store_stops_the_run_at_the_failing_file() {
    use std::sync::atomic::Ordering;
    let cases = [
        (Src::Store, ImportKind::Indices),
        (Src::Zips, ImportKind::Indices),
        (Src::Store, ImportKind::Options),
        (Src::Zips, ImportKind::Options),
    ];
    for (n, (src, kind)) in cases.into_iter().enumerate() {
        let root = scratch(&format!("ia16-source-lost-{n}"));
        world(&root, &[d1(), d2(), d3()]);
        let (source_kind, path) = source_path(&root, src, kind);
        let origin = Origin::measure(source_kind, &path, Some(COMMIT));
        let held = |store: &Path| match kind {
            ImportKind::Options => bars(store, "FNO", "NIFTY", Some(nifty_ce())),
            _ => bars(store, "INDEX", "NIFTY", None),
        };
        let whole = root.join("W");
        let reference = run_on(&root, (src, kind), (d1(), d3()), &whole, (1, &origin));
        assert!(
            reference.failures.is_empty(),
            "{n}: {:?}",
            reference.failures
        );
        let store = root.join("S");
        let gone = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let unplugged = Unplugged {
            bytes: std::fs::read(&path).unwrap_or_default(),
            gone: std::sync::Arc::clone(&gone),
        };
        let len = u64::try_from(unplugged.bytes.len()).unwrap();
        // The outer zip is read through one archive for both runs, as one
        // process holds it open across an unplug.
        let zip = matches!((src, kind), (Src::Zips, ImportKind::Indices));
        let archive =
            zip.then(|| crate::gdfl_archive::Archive::from_source(unplugged, len).unwrap());
        let run_of = |from: Day, to: Day| -> Result<Report, ImportRefusal> {
            match &archive {
                Some(archive) => {
                    let run = Run {
                        kind,
                        from,
                        to,
                        only: &[],
                        store_root: &store,
                    };
                    let options = RunOptions {
                        origin: Some(&origin),
                        workers: Workers::new(1).unwrap(),
                        ..RunOptions::default()
                    };
                    run_cm_from(archive, &run, options)
                }
                None => run_durable(
                    &root,
                    (src, kind),
                    (from, to),
                    &store,
                    (1, &origin, Durability::DEFAULT),
                ),
            }
        };
        let first = run_of(d1(), d1()).unwrap();
        assert!(first.failures.is_empty(), "{n}: {:?}", first.failures);
        let held_once = held(&store);
        assert!(!held_once.is_empty(), "{n}");

        let aside = root.join("source-gone");
        std::fs::rename(&path, &aside).unwrap();
        gone.store(true, Ordering::SeqCst);
        let ended = run_of(d2(), d3()).unwrap_err();
        assert_eq!(ended.code(), "source_lost", "{n}: {ended}");
        assert!(
            ended.to_string().contains(&path.display().to_string()),
            "{n}: the probe names the root: {ended}"
        );
        assert!(
            matches!(&ended, ImportRefusal::SourceLost { day, .. } if *day == d2()),
            "{n}: {ended}"
        );
        let text = journal(&store);
        assert!(!text.contains("day_not_held"), "{n}: {text}");
        assert!(
            !text.contains("2024-04-02") && !text.contains("2024-04-03"),
            "{n}: no line for a day the lost source never gave: {text}"
        );
        assert_eq!(held(&store), held_once, "{n}: nothing written");

        std::fs::rename(&aside, &path).unwrap();
        gone.store(false, Ordering::SeqCst);
        let resumed = run_of(d2(), d3()).unwrap();
        assert!(resumed.failures.is_empty(), "{n}: {:?}", resumed.failures);
        assert_eq!(resumed.days_imported, 2, "{n}");
        assert_eq!(held(&store), held(&whole), "{n}");
        std::fs::remove_dir_all(root).unwrap();
    }
}

// ── FM-038, FM-073: the day-close refusals ─────────────────────────────────

/// A held day whose every entry lies under another day's folder: an
/// unfiltered run refuses it `no_wanted_file`, never `done files=0`, and
/// reads it again; a filtered run's empty day still closes done (D-4872).
/// For the indices tree and the options tree.
#[test]
fn a_listing_with_no_wanted_file_is_refused_never_done() {
    let cases = [
        (
            ImportKind::Indices,
            Tree::Indices,
            entry_of(Tree::Indices, d2(), "NIFTY 50.NSE_IDX"),
            index_file("NIFTY 50.NSE_IDX", d2(), 22_000),
        ),
        (
            ImportKind::Options,
            Tree::Options,
            entry_of(Tree::Options, d2(), "NIFTY04APR2422000CE"),
            option_file("NIFTY04APR2422000CE", d2(), "101.5"),
        ),
    ];
    for (kind, tree_of, entry, bytes) in cases {
        let root = scratch(&format!("ia16-no-wanted-{}", kind.as_str()));
        world_of(&root, &[(d1(), tree_of, vec![(entry, bytes)])]);
        let store = root.join("S");
        let got = plain(&root, kind, (d1(), d1()), &store);
        assert_eq!(got.closed_refused, vec![d1()], "{kind:?} {got:?}");
        let line = line_of(&store, kind.as_str(), "2024-04-01");
        assert!(
            line.starts_with("refused ") && line.contains(" reason=no_wanted_file"),
            "{kind:?}: {line}"
        );
        assert!(!journal(&store).contains("\ndone "), "{kind:?}");
        let filtered_store = root.join("F");
        let only = ["NIFTY".to_owned()];
        let filtered = import_with(
            &root,
            (kind, d1(), d1()),
            (&filtered_store, &only),
            RunOptions::default(),
        )
        .unwrap();
        assert!(
            filtered.failures.is_empty(),
            "{kind:?} {:?}",
            filtered.failures
        );
        assert!(
            line_of(&filtered_store, kind.as_str(), "2024-04-01").starts_with("done "),
            "{kind:?}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// An indices day the source holds whose listing lacks BANKNIFTY: closed
/// with the refusal `absent_from_listing` naming it (the vendor's own
/// listing, so the day closes, D-4833), NIFTY landed; a run filtered to
/// NIFTY asks nothing of BANKNIFTY.
#[test]
fn an_indices_day_without_a_swept_index_names_it() {
    let root = scratch("ia16-absent");
    world_of(
        &root,
        &[(
            d1(),
            Tree::Indices,
            vec![(
                entry_of(Tree::Indices, d1(), "NIFTY 50.NSE_IDX"),
                index_file("NIFTY 50.NSE_IDX", d1(), 22_000),
            )],
        )],
    );
    let store = root.join("S");
    let got = plain(&root, ImportKind::Indices, (d1(), d1()), &store);
    assert_eq!(got.closed_with_refusals, vec![d1()], "{got:?}");
    let line = line_of(&store, "indices", "2024-04-01");
    assert!(line.contains(" reason=absent_from_listing"), "{line}");
    assert!(
        got.failures
            .iter()
            .any(|one| one.instrument.ends_with("BANKNIFTY") && one.why.contains("absent")),
        "{:?}",
        got.failures
    );
    assert!(got.seconds_committed > 0, "NIFTY landed");
    let only = ["NIFTY".to_owned()];
    let filtered_store = root.join("F");
    let filtered = import_with(
        &root,
        (ImportKind::Indices, d1(), d1()),
        (&filtered_store, &only),
        RunOptions::default(),
    )
    .unwrap();
    assert!(filtered.failures.is_empty(), "{:?}", filtered.failures);
    std::fs::remove_dir_all(root).unwrap();
}

/// FM-073, design §9's "Renamed file" row and D21 (IA-16 repair 1): an
/// indices day whose BANKNIFTY file the vendor re-spelt names BANKNIFTY
/// `absent_from_listing` AND the stems no scope files, the renamed one among
/// them: at most four, the CLOSEST to BANKNIFTY's name first and ties in
/// listing order (IA-16 repair 2: the renamed stem is listed LAST, after
/// five others, and is still named, first), with the count of all of them.
/// A day with no such stem says so. Every name and price is invented.
#[test]
fn a_swept_stem_absent_from_a_held_day_is_named_with_the_unknown_stems() {
    let root = scratch("ia16-absent-renamed");
    let mut files = vec![(
        entry_of(Tree::Indices, d1(), "NIFTY 50.NSE_IDX"),
        index_file("NIFTY 50.NSE_IDX", d1(), 22_000),
    )];
    let unknown = [
        "NIFTY ZETA ONE.NSE_IDX",
        "NIFTY ZETA TWO.NSE_IDX",
        "NIFTY ZETA THREE.NSE_IDX",
        "NIFTY ZETA FOUR.NSE_IDX",
        "NIFTY ZETA FIVE.NSE_IDX",
        "NIFTY BANKK.NSE_IDX",
    ];
    for (at, stem) in unknown.iter().enumerate() {
        let base = 30_000 + u32::try_from(at).unwrap();
        files.push((
            entry_of(Tree::Indices, d1(), stem),
            index_file(stem, d1(), base),
        ));
    }
    world_of(&root, &[(d1(), Tree::Indices, files)]);
    let store = root.join("S");
    let got = plain(&root, ImportKind::Indices, (d1(), d1()), &store);
    assert_eq!(got.closed_with_refusals, vec![d1()], "{got:?}");
    let absent: Vec<_> = got
        .failures
        .iter()
        .filter(|one| one.why.contains("absent from the day's listing"))
        .collect();
    assert_eq!(absent.len(), 1, "BANKNIFTY alone: {:?}", got.failures);
    let why = &absent[0].why;
    assert!(absent[0].instrument.ends_with("BANKNIFTY"), "{absent:?}");
    assert!(why.contains("held 6 stem(s) no scope files"), "{why}");
    assert!(why.contains("the 4 closest to its name: "), "{why}");
    // The renamed stem first, then the three earliest of the equally far.
    let named = [unknown[5], unknown[0], unknown[1], unknown[2]].join(" | ");
    assert!(why.contains(&named), "{named} in this order: {why}");
    for stem in &unknown[3..5] {
        assert!(!why.contains(stem), "{stem} only counted: {why}");
    }
    assert!(got.seconds_committed > 0, "NIFTY landed");
    // A day whose every stem is filed names none, and says so.
    let plain_root = scratch("ia16-absent-none");
    world_of(
        &plain_root,
        &[(
            d1(),
            Tree::Indices,
            vec![(
                entry_of(Tree::Indices, d1(), "NIFTY 50.NSE_IDX"),
                index_file("NIFTY 50.NSE_IDX", d1(), 22_000),
            )],
        )],
    );
    let none = plain(
        &plain_root,
        ImportKind::Indices,
        (d1(), d1()),
        &plain_root.join("S"),
    );
    assert!(
        none.failures
            .iter()
            .any(|one| one.why.contains("held no stem that no scope files")),
        "{:?}",
        none.failures
    );
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(plain_root).unwrap();
}

/// The ranking `absent_from_listing` names by (IA-16 repair 2, FM-073): a
/// renamed spelling of a swept symbol is closer than another index of the
/// same family, case, spaces and punctuation aside; the kept set never
/// grows past four, a closer stem displaces the farthest kept, and a later
/// stem never displaces an equally close earlier one.
#[test]
fn the_unfiled_stems_are_ranked_by_closeness_to_the_swept_name() {
    let near = |stem| crate::gdfl_import::closeness(stem, "BANKNIFTY");
    assert!(near("NIFTY BANK.NSE_IDX") > near("NIFTY IT.NSE_IDX"));
    assert_eq!(near("nifty-bank"), near("NIFTY BANK"));
    assert_eq!(near(""), 0);
    assert_eq!(crate::gdfl_import::closeness("NIFTY BANK", ""), 0);
    let mut kept = Vec::new();
    for (score, stem) in [(1, "a"), (3, "b"), (1, "c"), (2, "d"), (3, "e"), (1, "f")] {
        crate::gdfl_import::keep_nearest(&mut kept, score, stem);
    }
    let names: Vec<&str> = kept.iter().map(|(_, stem)| stem.as_str()).collect();
    assert_eq!(names, ["b", "e", "d", "a"]);
    crate::gdfl_import::keep_nearest(&mut kept, 1, "g");
    assert_eq!(kept.len(), 4);
    assert!(!kept.iter().any(|(_, stem)| stem == "g"), "{kept:?}");
}

// ── CMP-13: an option file that kept its good rows ─────────────────────────

/// IA-16'S SHARE OF CMP-13. An option contract file whose reader kept its
/// good rows and refused three others one by one (the value IA-4's decoder
/// will hand on through [`option_reader_counts`]) is filed, and its day is
/// admitted and closes `done` exactly as a CASH day with D-3318 row
/// refusals does: no failure, no hole, no whole-file refusal; the three are
/// counted in the day ledger's `rows_refused`, and the day is not read
/// again. The value is invented, never a vendor row.
#[test]
fn an_option_file_with_rows_refused_one_by_one_is_admitted_not_failed_whole() {
    let root = scratch("ia16-cmp13");
    let store = root.join("S");
    let only: [String; 0] = [];
    let run = Run {
        kind: ImportKind::Options,
        from: d1(),
        to: d1(),
        only: &only,
        store_root: &store,
    };
    let reader = option_reader_counts(4, 3);
    assert_eq!(reader.rows_refused, 3);
    assert_eq!(reader.quoted_rows, Some(4));
    let drive = || {
        day::drive_core(
            &run,
            RunOptions::default(),
            (&mut StoreCensus::default(), None),
            |day, filer| {
                let ticks = (0..40)
                    .map(|n| Tick {
                        sod: NINE_FIFTEEN + n * 30,
                        ltp: 10_150 + i64::from(n),
                        ltq: 25,
                        oi: Some(1_000),
                    })
                    .collect();
                let handed = std::sync::Mutex::new(Some(Handed {
                    file: TickFile {
                        symbol: "NIFTY".to_owned(),
                        contract: Some(nifty_ce()),
                        name: "NIFTY04APR2422000CE.NFO.csv".to_owned(),
                        ticks,
                    },
                    facts: FileFacts {
                        member: None,
                        reader,
                        auction: None,
                    },
                }));
                let jobs = [handed];
                let _reads = filer.file_all(&jobs, &|_| None, &|slot| {
                    slot.lock()
                        .unwrap()
                        .take()
                        .map(|one| (one, None))
                        .ok_or_else(|| {
                            Refused::at(
                                Cause::open(Stage::File, "file_taken_twice"),
                                failure("file", "taken twice".to_owned()),
                            )
                        })
                });
                assert_eq!(day, d1());
                (
                    DayRead {
                        held: true,
                        ..DayRead::default()
                    },
                    DayExtras::default(),
                )
            },
        )
        .unwrap()
    };
    let got = drive();
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(got.files, 1);
    assert!(got.seconds_committed > 0);
    assert_eq!(got.holes_skipped + got.holes_unrepairable, 0);
    let line = line_of(&store, "options", "2024-04-01");
    assert!(
        line.starts_with("done ") && !line.contains("vendor_refused"),
        "closed as a CASH day with row refusals closes: {line}"
    );
    let flags = std::fs::read_to_string(quality::dayflags_path(&store)).unwrap();
    assert!(flags.contains("rows_refused=3"), "{flags}");
    let again = drive();
    assert_eq!(again.days_skipped, 1, "admitted once, done");
    assert_eq!(again.files, 0);
    std::fs::remove_dir_all(root).unwrap();
}

// ── IA-GR-42: one repair-reason rule for both feeds ────────────────────────

/// A request's repair reason is judged by `ingest::repair_reason_ok` before
/// anything is read, and a good one survives the request line byte for
/// byte, spaces and all.
#[test]
fn a_requests_repair_reason_is_judged_by_the_one_rule_and_survives_its_line() {
    use crate::gdfl_run::{Kinds, Request, RunRefusal, SourceWord};
    let mut request = Request::new(
        Kinds::of(&[ImportKind::Indices]).unwrap(),
        SourceWord::TickStore,
        (d1(), d2()),
    );
    for bad in [
        String::new(),
        "x".repeat(crate::ingest::MAX_REPAIR_REASON + 1),
        "a\tb".to_owned(),
        "a\nb".to_owned(),
    ] {
        request.repair_reason = Some(bad.clone());
        assert!(
            matches!(
                request.misuse(),
                Some(RunRefusal::RepairReasonInvalid { bytes }) if bytes == bad.len()
            ),
            "{bad:?}"
        );
        assert!(request.to_line().is_err());
    }
    let longest = "y".repeat(crate::ingest::MAX_REPAIR_REASON);
    for good in [
        REASON.to_owned(),
        longest,
        "back-fill 2018-09-03 % = +".to_owned(),
    ] {
        request.repair_reason = Some(good.clone());
        request.skip_restated = true;
        assert_eq!(request.misuse(), None, "{good:?}");
        let line = request.to_line().unwrap();
        assert_eq!(Request::from_line(&line).unwrap(), request, "{line}");
    }
    request.repair_reason = None;
    request.skip_restated = false;
    let line = request.to_line().unwrap();
    let tail = format!(
        " repair_reason= skip_restated=0 accept_open_month=0 free_floor={DEFAULT_FREE_FLOOR}"
    );
    assert!(line.ends_with(&tail), "{line}");
    assert_eq!(Request::from_line(&line).unwrap(), request);
    request.accept_open_month = true;
    let line = request.to_line().unwrap();
    assert!(line.contains(" accept_open_month=1 "), "{line}");
    assert_eq!(Request::from_line(&line).unwrap(), request);
    // The floor crosses the line as a count: 0 (no check) and the largest.
    for floor in [0, u64::MAX] {
        request.free_floor = floor;
        let line = request.to_line().unwrap();
        assert!(line.ends_with(&format!(" free_floor={floor}")), "{line}");
        assert_eq!(Request::from_line(&line).unwrap(), request);
    }
    request.free_floor = DEFAULT_FREE_FLOOR;
    // A LINE WRITTEN BEFORE IA-16 (it ends at `snapshot=`) is still read,
    // every field IA-16 added at its default; one that stops part way
    // through them, or adds one more, is refused by name.
    request.accept_open_month = false;
    let line = request.to_line().unwrap();
    let older = line.strip_suffix(&tail).unwrap();
    assert_eq!(Request::from_line(older).unwrap(), request, "{older}");
    for torn in [
        format!("{older} repair_reason="),
        format!("{older} repair_reason= skip_restated=0"),
        format!("{older} repair_reason= skip_restated=0 accept_open_month=0"),
        format!("{older} repair_reason= skip_restated=0 accept_open_month=0 free_floor=+5"),
        format!("{older} repair_reason= skip_restated=0 accept_open_month=0 free_floor=007"),
        format!("{older} repair_reason= skip_restated=0 accept_open_month=0 free_floor=x"),
        format!("{line} extra=1"),
    ] {
        assert!(
            matches!(
                Request::from_line(&torn),
                Err(RunRefusal::RequestLineInvalid { .. })
            ),
            "{torn}"
        );
    }
}

// ── D-4876 (2), D-6900 keep item 3: a repair in an open month ──────────────

/// A month whose last trading day by the calendar of record has not passed
/// is open; every earlier month has ended; a later month is open; a clock
/// that names no day keeps the first month open. The calendar decides the
/// month's tail: two closed days at a month's end do not keep it open.
#[test]
fn a_repair_in_an_open_month_is_refused_unless_accepted_and_converges_after_the_month_ends() {
    use crate::calendar::{DayKind, kind_of};
    use crate::gdfl_run::open_month_in;
    let day = |y, m, d| Day::new(y, m, d).unwrap();
    let today = Some(day(2026, 10, 20));
    // Every month of 2018-09..2026-09 has ended by 20 Oct 2026.
    assert_eq!(
        open_month_in((day(2018, 9, 3), day(2026, 9, 30)), today),
        None
    );
    // One day of the current month keeps the whole range open, named.
    assert_eq!(
        open_month_in((day(2026, 9, 1), day(2026, 10, 1)), today).as_deref(),
        Some("2026-10")
    );
    // Even a range that ends before today, inside today's month.
    assert_eq!(
        open_month_in((day(2026, 10, 1), day(2026, 10, 2)), today).as_deref(),
        Some("2026-10")
    );
    // A later month.
    assert_eq!(
        open_month_in((day(2026, 11, 2), day(2026, 11, 3)), today).as_deref(),
        Some("2026-11")
    );
    // No clock: refused, never assumed passed.
    assert_eq!(
        open_month_in((day(2020, 9, 1), day(2020, 9, 30)), None).as_deref(),
        Some("2020-09")
    );
    // The month's last trading day is today: not passed.
    assert_eq!(
        open_month_in((day(2026, 9, 1), day(2026, 9, 30)), Some(day(2026, 9, 30))).as_deref(),
        Some("2026-09")
    );
    // The day after the month ends, it has (it converges after the month).
    assert_eq!(
        open_month_in((day(2026, 9, 1), day(2026, 9, 30)), Some(day(2026, 10, 1))),
        None
    );
    // Its last two days closed (a Saturday and a Sunday): passed on the
    // Saturday, by the calendar and not by the date alone.
    let (sat, sun) = (day(2026, 5, 30), day(2026, 5, 31));
    for closed in [sat, sun] {
        assert!(
            matches!(
                kind_of(i64::from(closed.days_from_epoch())),
                DayKind::Closed
            ),
            "{closed} is not closed in the calendar of record"
        );
    }
    assert_eq!(
        open_month_in((day(2026, 5, 4), day(2026, 5, 29)), Some(sat)),
        None
    );

    // THE PLAN REFUSES IT BEFORE ANY LOCK, unless accepted (the clock here is
    // the host's: a range in the month after today's is open on any day).
    let store = scratch("ia16-open-month");
    let log = crate::gdfl_run::LogState::Installed(store.join("logs"));
    let process = crate::gdfl_run::Process {
        store_root: &store,
        commit: None,
        log: &log,
        unstamped: None,
    };
    let now = crate::session::IstMoment::from_epoch_secs(
        i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        )
        .unwrap(),
    )
    .unwrap()
    .day();
    let ahead = now.end_of_month().succ().unwrap();
    let mut request = crate::gdfl_run::Request::new(
        crate::gdfl_run::Kinds::of(&[ImportKind::Indices]).unwrap(),
        crate::gdfl_run::SourceWord::TickStore,
        (ahead, ahead),
    );
    request.repair_reason = Some(REASON.to_owned());
    let none = |_: &str| None;
    let refused = crate::gdfl_run::plan(
        request.clone(),
        &process,
        &none,
        &mut crate::gdfl_run::SourceCache::default(),
    )
    .unwrap_err();
    assert_eq!(refused.code(), "repair_in_open_month", "{refused}");
    assert!(
        refused
            .to_string()
            .contains(&format!("{:04}-{:02}", ahead.year(), ahead.month())),
        "{refused}"
    );
    assert!(!refused.misuse(), "a guard, not a misuse: {refused}");
    assert!(
        tree(&store).is_empty(),
        "refused before anything was written"
    );
    // Accepted by name, the check is passed (the next refusal is another's).
    request.accept_open_month = true;
    let next = crate::gdfl_run::plan(
        request.clone(),
        &process,
        &none,
        &mut crate::gdfl_run::SourceCache::default(),
    );
    assert!(
        next.as_ref()
            .err()
            .is_none_or(|why| why.code() != "repair_in_open_month"),
        "{next:?}"
    );
    // Without a repair reason the check is not asked.
    request.accept_open_month = false;
    request.repair_reason = None;
    let plain = crate::gdfl_run::plan(
        request,
        &process,
        &none,
        &mut crate::gdfl_run::SourceCache::default(),
    );
    assert!(
        plain
            .as_ref()
            .err()
            .is_none_or(|why| why.code() != "repair_in_open_month"),
        "{plain:?}"
    );
    std::fs::remove_dir_all(store).unwrap();
}

// ── RA-209: the held-back set's cap ────────────────────────────────────────

/// One invented NIFTY index file of day `d`, as a source hands it on.
fn nifty_file(d: Day) -> Handed {
    let ticks = (0..60)
        .map(|n| Tick {
            sod: NINE_FIFTEEN + n * 60,
            ltp: 2_200_000 + i64::from(n),
            ltq: 0,
            oi: None,
        })
        .collect();
    Handed {
        file: TickFile {
            symbol: "NIFTY".to_owned(),
            contract: None,
            name: format!("NIFTY 50.NSE_IDX {d}.csv"),
            ticks,
        },
        facts: FileFacts::default(),
    }
}

/// Drives an indices run over `from..=to` into `store` whose every day
/// hands on [`nifty_file`], after `before` has seen the day's filer.
fn drive_nifty(
    store: &Path,
    (from, to): (Day, Day),
    before: &mut dyn FnMut(&mut filing::DayFiler<'_>),
) -> Result<Report, ImportRefusal> {
    let only: [String; 0] = [];
    let run = Run {
        kind: ImportKind::Indices,
        from,
        to,
        only: &only,
        store_root: store,
    };
    day::drive_core(
        &run,
        RunOptions::default(),
        (&mut StoreCensus::default(), None),
        |day, filer| {
            before(filer);
            let jobs = [std::sync::Mutex::new(Some(nifty_file(day)))];
            let month = day.year_month().ok();
            let key_of = |_: &std::sync::Mutex<Option<Handed>>| {
                month.map(|month| ("NIFTY".to_owned(), None, month))
            };
            let _reads = filer.file_all(&jobs, &key_of, &|slot| {
                slot.lock()
                    .unwrap()
                    .take()
                    .map(|one| (one, None))
                    .ok_or_else(|| {
                        Refused::at(
                            Cause::open(Stage::File, "file_taken_twice"),
                            failure("file", "taken twice".to_owned()),
                        )
                    })
            });
            (
                DayRead {
                    held: true,
                    ..DayRead::default()
                },
                DayExtras::default(),
            )
        },
    )
}

/// A run whose held-back set is full ends `held_back_cap` once the day that
/// met the cap is closed, naming the pair it could not hold and the count;
/// the day itself closes `incomplete` like any day with a held-back cause,
/// and no later day begins. With room, the same failure is held back and
/// the run goes on.
#[test]
fn a_full_held_back_set_ends_the_run_by_name() {
    let root = scratch("ia16-cap");
    let store = root.join("S");
    drive_nifty(&store, (d1(), d1()), &mut |_| {}).unwrap();
    let nifty = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: d1().year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    let reader = BarFile::open_existing(&store, nifty, filing::symbol_id_of("NIFTY")).unwrap();
    let month = d1().year_month().unwrap();
    let mut filled = false;
    let ended = drive_nifty(&store, (d2(), d3()), &mut |filer| {
        if !filled {
            let mut set = filer.ctx.held_back.borrow_mut();
            for n in 0..HELD_BACK_CAP {
                set.insert((format!("FILLER{n}"), None, month), d1());
            }
            filled = true;
        }
    })
    .unwrap_err();
    drop(reader);
    assert_eq!(ended.code(), "held_back_cap", "{ended}");
    let said = ended.to_string();
    assert!(said.contains("NIFTY") && said.contains("4097"), "{said}");
    assert!(line_of(&store, "indices", "2024-04-02").contains(" reason=month_busy"));
    assert!(
        !journal(&store).contains("2024-04-03"),
        "no later day began"
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ── FM-252: a file the vendor did not ship holds back nothing ──────────────

/// A three-day options run in which one contract (the NIFTY PE) has no file
/// on day 2, as a contract absent from a day's zip or a share suspended for
/// a day: nothing failed, so nothing is held back. Day 2 closes `done`, day
/// 3's PE file lands after day 1's, and the run held back no file. Hold-back
/// is for a file that FAILED (a busy month, a bad copy), never for one that
/// was not there.
#[test]
fn a_share_file_absent_on_one_day_does_not_hold_back_the_rest_of_its_month() {
    let root = scratch("ia16-fm252");
    let tickers = [
        "NIFTY04APR2422000CE",
        "NIFTY04APR2422000PE",
        "BANKNIFTY10APR2447000CE",
    ];
    let files = |d: Day, with_pe: bool| {
        tickers
            .iter()
            .filter(|t| with_pe || !t.ends_with("PE"))
            .map(|t| (entry_of(Tree::Options, d, t), option_file(t, d, "101.5")))
            .collect::<Vec<_>>()
    };
    world_of(
        &root,
        &[
            (d1(), Tree::Options, files(d1(), true)),
            (d2(), Tree::Options, files(d2(), false)),
            (d3(), Tree::Options, files(d3(), true)),
        ],
    );
    let store = root.join("S");
    let got = plain(&root, ImportKind::Options, (d1(), d3()), &store);
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(got.files_held_back, 0, "{got:?}");
    assert_eq!(got.days_imported, 3, "{got:?}");
    for on in ["2024-04-01", "2024-04-02", "2024-04-03"] {
        assert!(line_of(&store, "options", on).starts_with("done "), "{on}");
    }
    let pe = Contract::parse("2024-04-04-2200000-PE").unwrap();
    let days: std::collections::BTreeSet<i64> = bars(&store, "FNO", "NIFTY", Some(pe))
        .iter()
        .map(|bar| bar.ts_micros.div_euclid(86_400_000_000))
        .collect();
    let day = |d: Day| i64::from(d.days_from_epoch());
    assert_eq!(
        days,
        [day(d1()), day(d3())].into_iter().collect(),
        "the PE month took day 3 after day 1, and day 2 is simply absent"
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ── D-4841: the day ledger's v=2 fields ────────────────────────────────────

/// A share file's quality from invented rows, its bars one per stamp.
fn share_quality(ticks: &[Tick]) -> quality::FileQuality {
    let on = d1();
    let session = crate::gdfl_cm::session_of(on).unwrap();
    let mut seconds: Vec<u32> = ticks.iter().map(|t| t.sod).collect();
    seconds.sort_unstable();
    seconds.dedup();
    quality::measure_file(
        ImportKind::Stocks,
        (&session, quality::halt_window(ImportKind::Stocks, on)),
        ticks,
        &seconds,
        (Placement::default(), quality::Deferred::default()),
        quality::ReaderCounts::default(),
    )
}

/// One invented share row at `sod` with price `ltp` (paisa) and `ltq`.
const fn row(sod: u32, ltp: i64, ltq: u64) -> Tick {
    Tick {
        sod,
        ltp,
        ltq,
        oi: None,
    }
}

/// D-4830, D-4841 (`volume_saturated_files`): a file whose running LTQ sum
/// reaches 2^31 - 1 (or 2^31) on its last traded row, with only LTQ = 0 rows
/// after it (the price still moving, or the rows ending there), is counted
/// saturated; one sum short of the limit, one past it, and one with a later
/// traded row are not; an empty file is not. The bars are never touched by
/// the count. The day's v=2 line carries the count and the separator, and a
/// v=2 field this build does not count is absent, never 0.
#[test]
fn a_volume_stopped_at_the_32_bit_limit_is_counted_and_the_line_is_v2() {
    let at = NINE_FIFTEEN + 60;
    let half = 1_073_741_823_u64;
    let cases: [(&str, Vec<Tick>, bool); 7] = [
        (
            "stops at 2^31 - 1, the price moving after",
            vec![
                row(at, 9_901, half),
                row(at + 1, 9_902, half + 1),
                row(at + 2, 9_903, 0),
                row(at + 3, 9_904, 0),
            ],
            true,
        ),
        (
            "stops at 2^31, the rows ending there",
            vec![row(at, 9_901, half + 1), row(at + 1, 9_902, half)]
                .into_iter()
                .chain([row(at + 2, 9_903, 1)])
                .collect(),
            true,
        ),
        (
            "one short of the limit",
            vec![
                row(at, 9_901, half),
                row(at + 1, 9_902, half),
                row(at + 2, 9_903, 0),
            ],
            false,
        ),
        (
            "past the limit",
            vec![row(at, 9_901, half + 2), row(at + 1, 9_902, half + 1)],
            false,
        ),
        (
            "a later traded row",
            vec![
                row(at, 9_901, half),
                row(at + 1, 9_902, half + 1),
                row(at + 2, 9_903, 5),
            ],
            false,
        ),
        ("an empty file", Vec::new(), false),
        ("one row, untraded", vec![row(at, 9_901, 0)], false),
    ];
    let session = crate::gdfl_cm::session_of(d1()).unwrap();
    let mut ledger = quality::DayLedger::new(ImportKind::Stocks, d1(), session);
    let mut want = 0;
    for (case, ticks, saturated) in &cases {
        let measured = share_quality(ticks);
        assert_eq!(measured.volume_saturated, *saturated, "{case}");
        want += u64::from(*saturated);
        ledger.take("SBIN", &measured);
    }
    ledger.set_separator_backslash(true);
    let lines = ledger.lines(true);
    assert_eq!(lines.len(), 1, "{lines:?}");
    let (head, fields) = &lines[0];
    assert!(head.ends_with(" * v=2"), "{head}");
    assert!(
        fields.contains(&format!(
            " volume_saturated_files={want} separator_backslash=1 "
        )),
        "{fields}"
    );
    for absent in [
        "dup_traded_rows",
        "dup_echo_rows",
        "snapped_rows",
        "trailing_empty_files",
    ] {
        assert!(
            !fields.contains(absent),
            "{absent} is not counted, so absent: {fields}"
        );
    }
    // An index line: the separator; an options line: v=2 with no v=2 field
    // this build counts.
    let mut index = quality::DayLedger::new(ImportKind::Indices, d1(), session);
    index.take("NIFTY", &quality::FileQuality::default());
    let lines = index.lines(true);
    assert!(lines[0].0.ends_with(" NIFTY v=2"), "{lines:?}");
    assert!(lines[0].1.contains(" separator_backslash=0 "), "{lines:?}");
    let mut options = quality::DayLedger::new(ImportKind::Options, d1(), session);
    options.take("*", &quality::FileQuality::default());
    let lines = options.lines(true);
    assert!(lines[0].0.ends_with(" * v=2"), "{lines:?}");
    assert!(!lines[0].1.contains("separator_backslash"), "{lines:?}");
}

// ── FM-129: a store that takes no writes is one named stop ─────────────────

/// Sets every file under `dir` to `mode` (Unix permission bits).
fn chmod_all(dir: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    for path in tree(dir).keys() {
        std::fs::set_permissions(dir.join(path), std::fs::Permissions::from_mode(mode)).unwrap();
    }
}

/// The NIFTY month made read-only after day 1 (what an immutable flag, a
/// restrictive ACL or a read-only remount leaves: the host refuses the
/// write): day 2's NIFTY file is refused by the host, the run ends
/// `store_not_writable` once day 2 is closed, naming the store's refusal and
/// the fix, and day 3 has no `begin`. A second run stops again at the same
/// file by the same words, never a refusal repeated on every later day. Once
/// the month takes writes again, a resume converges. A planted permission
/// refusal (the inject seam) ends the run the same way.
#[test]
fn an_immutable_month_stops_the_run_by_name() {
    let root = scratch("ia16-fm129");
    world(&root, &[d1(), d2(), d3()]);
    let store = root.join("S");
    let whole = root.join("W");
    plain(&root, ImportKind::Indices, (d1(), d3()), &whole);
    plain(&root, ImportKind::Indices, (d1(), d1()), &store);
    let nifty = store.join("bars/gdfl/NSE/INDEX/NIFTY");
    assert!(nifty.is_dir(), "{}", nifty.display());
    chmod_all(&nifty, 0o444);
    let options = RunOptions {
        workers: Workers::new(1).unwrap(),
        ..RunOptions::default()
    };
    for attempt in 0..2 {
        let ended = import_with(
            &root,
            (ImportKind::Indices, d2(), d3()),
            (&store, &[]),
            options,
        )
        .unwrap_err();
        assert_eq!(ended.code(), "store_not_writable", "{attempt}: {ended}");
        let said = ended.to_string();
        assert!(
            said.contains("chflags nouchg")
                && said.contains("chmod -N")
                && said.contains("read-write"),
            "the fix is named: {said}"
        );
        let done = ended.report().unwrap();
        assert_eq!(done.closed_incomplete, vec![d2()], "{attempt}: {done:?}");
        let text = journal(&store);
        assert!(
            !text.contains("begin indices 2024-04-03"),
            "{attempt}: {text}"
        );
    }
    chmod_all(&nifty, 0o644);
    let resumed = plain(&root, ImportKind::Indices, (d2(), d3()), &store);
    assert!(resumed.failures.is_empty(), "{:?}", resumed.failures);
    assert_eq!(
        bars(&store, "INDEX", "NIFTY", None),
        bars(&whole, "INDEX", "NIFTY", None)
    );

    // The same stop from a planted host refusal on a fresh store.
    let planted = root.join("P");
    std::fs::create_dir_all(&planted).unwrap();
    filing::inject::plant(
        &planted,
        "NIFTY 50.NSE_IDX.csv",
        FailKind::Io {
            kind: std::io::ErrorKind::ReadOnlyFilesystem,
            publication_may_be_visible: false,
        },
        false,
    );
    let ended = import_with(
        &root,
        (ImportKind::Indices, d1(), d2()),
        (&planted, &[]),
        options,
    )
    .unwrap_err();
    assert_eq!(ended.code(), "store_not_writable", "{ended}");
    assert!(!journal(&planted).contains("begin indices 2024-04-02"));
    std::fs::remove_dir_all(root).unwrap();
}

// ── FM-085, CONC-031: the free-space floor ─────────────────────────────────

/// How many times [`tightening`] was asked.
static TIGHTENING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// A meter whose volume has room for the run's start and its first day and
/// none after: 2,000 bytes free, then 999.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a meter's type: `SpaceMeter` answers a `Result`"
)]
fn tightening(_: &Path) -> Result<crate::disk_projection::Space, String> {
    let asked = TIGHTENING.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(crate::disk_projection::Space {
        total: 10_000,
        free: if asked < 2 { 2_000 } else { 999 },
    })
}

/// FM-085 and CONC-031 (D-6900 keep item 5): the floor is checked before
/// the run's first day and before each day it reads. A volume below it is
/// named `disk_full` with both figures before anything is written (no
/// `begin`, no bar); a volume that falls below it after day 1 lets day 1
/// close and begins no later day; a meter that cannot answer is
/// `free_space_unknown`, never assumed to hold; a floor of 0 asks nothing;
/// and the host's own meter measures a real volume.
#[test]
fn a_free_space_floor_names_a_full_disk_before_the_first_write() {
    let root = scratch("ia16-floor");
    world(&root, &[d1(), d2(), d3()]);
    let full = |_: &Path| {
        Ok(crate::disk_projection::Space {
            total: 10_000,
            free: 999,
        })
    };
    let blind = |_: &Path| Err("the meter is unplugged".to_owned());
    let floor = |meter: SpaceMeter| RunOptions {
        free_floor: 1_000,
        free_space: Some(meter),
        ..RunOptions::default()
    };
    // Below the floor from the start: nothing begun, nothing written.
    let store = root.join("full");
    std::fs::create_dir_all(&store).unwrap();
    let ended = import_with(
        &root,
        (ImportKind::Indices, d1(), d3()),
        (&store, &[]),
        floor(full),
    )
    .unwrap_err();
    assert_eq!(ended.code(), "disk_full", "{ended}");
    let said = ended.to_string();
    assert!(
        said.contains("999 bytes free of 10000") && said.contains("floor of 1000"),
        "{said}"
    );
    assert!(!journal(&store).contains("begin "), "{}", journal(&store));
    assert!(!store.join("bars").exists(), "no bar written");
    // A meter that cannot answer.
    let store = root.join("blind");
    std::fs::create_dir_all(&store).unwrap();
    let ended = import_with(
        &root,
        (ImportKind::Indices, d1(), d1()),
        (&store, &[]),
        floor(blind),
    )
    .unwrap_err();
    assert_eq!(ended.code(), "free_space_unknown", "{ended}");
    assert!(
        ended.to_string().contains("the meter is unplugged"),
        "{ended}"
    );
    assert!(!journal(&store).contains("begin "));
    // Room for day 1 only.
    let store = root.join("tight");
    TIGHTENING.store(0, std::sync::atomic::Ordering::SeqCst);
    let ended = import_with(
        &root,
        (ImportKind::Indices, d1(), d3()),
        (&store, &[]),
        floor(tightening),
    )
    .unwrap_err();
    assert_eq!(ended.code(), "disk_full", "{ended}");
    assert_eq!(ended.report().unwrap().days_imported, 1, "{ended}");
    assert!(line_of(&store, "indices", "2024-04-01").starts_with("done "));
    assert!(!journal(&store).contains("begin indices 2024-04-02"));
    // Space freed (floor 0 asks nothing): the resume converges.
    let resumed = plain(&root, ImportKind::Indices, (d1(), d3()), &store);
    assert!(resumed.failures.is_empty(), "{:?}", resumed.failures);
    assert_eq!(resumed.days_imported, 2);
    // The host's meter on a real volume.
    let space = volume_space(&root).unwrap();
    assert!(space.total >= space.free && space.total > 0, "{space:?}");
    assert!(volume_space(&root.join("no-such-dir")).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

// ── IA-GR-18's mixed day (round 4, BL-C) ───────────────────────────────────

/// THE MIXED DAY. On day 2 instrument Y (NIFTY) meets a store hole (its
/// month already holds day 3) BEFORE instrument X (BANKNIFTY) meets a CRC
/// fault in its copy. The day closes with X's `source_crc_mismatch`, never
/// the hole, so the next run reads it again; X's day 3 is held back, so X's
/// month takes no later day while day 2 is owed. Once the copy is whole a
/// resume files X's day 2 and then day 3 through the append door (X's day 2
/// is never `needs_month_generation`), and only then closes day 2 the hole
/// Y's month makes it.
#[test]
fn a_hole_beside_a_transient_failure_never_strands_the_other_instrument() {
    let root = scratch("ia16-mixed-day");
    world(&root, &[d1(), d2(), d3()]);
    let (store, whole) = (root.join("S"), root.join("W"));
    plain(&root, ImportKind::Indices, (d1(), d3()), &whole);
    plain(&root, ImportKind::Indices, (d1(), d1()), &store);
    let only = ["NIFTY".to_owned()];
    let ahead = import_with(
        &root,
        (ImportKind::Indices, d3(), d3()),
        (&store, &only),
        RunOptions::default(),
    )
    .unwrap();
    assert!(ahead.failures.is_empty(), "{:?}", ahead.failures);
    // X's copy of day 2 states a CRC-32 its bytes do not have.
    let rel = format!(
        "ts/cm/INDICES/{:04}/{}_{:04}/GFDLCM_INDICES_TICK_{}.bts",
        d2().year(),
        mon(d2()),
        d2().year(),
        ddmmyyyy(d2())
    );
    let good = std::fs::read(root.join(&rel)).unwrap();
    let entries = cm_entries(CmKind::Indices, d2());
    let stated: Vec<(&str, &[u8], u32)> = entries
        .iter()
        .map(|(name, bytes)| {
            let crc = crate::gdfl_archive::crc32(bytes);
            let crc = if name.contains("NIFTY BANK") {
                crc ^ 1
            } else {
                crc
            };
            (name.as_str(), bytes.as_slice(), crc)
        })
        .collect();
    crate::gdfl_fixtures::put(&root, &rel, &crate::gdfl_fixtures::bts_stating(&stated));

    let mixed = plain(&root, ImportKind::Indices, (d2(), d3()), &store);
    let two = line_of(&store, "indices", "2024-04-02");
    assert!(
        two.starts_with("incomplete ") && two.contains(" reason=source_crc_mismatch"),
        "the non-hole cause decides, though the hole came first: {two}"
    );
    assert!(
        mixed
            .failures
            .iter()
            .any(|one| one.why.contains("needs_month_generation")),
        "Y met its hole first: {:?}",
        mixed.failures
    );
    assert!(mixed.holes_made.is_empty(), "{mixed:?}");
    assert!(
        line_of(&store, "indices", "2024-04-03").contains(" reason=held_back"),
        "X's day 3 is held back"
    );
    assert_eq!(mixed.files_held_back, 1, "{mixed:?}");
    let day_one = i64::from(d1().days_from_epoch());
    assert!(
        bars(&store, "INDEX", "BANKNIFTY", None)
            .iter()
            .all(|bar| bar.ts_micros.div_euclid(86_400_000_000) == day_one),
        "X's month took no later day while day 2 was owed"
    );

    // The copy is whole again: a resume lands X in order, then names Y's hole.
    crate::gdfl_fixtures::put(&root, &rel, &good);
    let resumed = plain(&root, ImportKind::Indices, (d2(), d3()), &store);
    assert_eq!(
        resumed.holes_made,
        vec![(d2(), "needs_month_generation".to_owned())],
        "{resumed:?}"
    );
    assert!(line_of(&store, "indices", "2024-04-02").contains(" reason=needs_month_generation"));
    assert!(line_of(&store, "indices", "2024-04-03").starts_with("done "));
    for one in mixed.failures.iter().chain(&resumed.failures) {
        assert!(
            !(one.instrument.contains("NIFTY BANK") && one.why.contains("needs_month_generation")),
            "X's day 2 is never a hole: {one:?}"
        );
    }
    assert_eq!(resumed.months_repaired, 0);
    assert_eq!(revisions(&store), 0, "landed through the append door");
    assert_eq!(
        bars(&store, "INDEX", "BANKNIFTY", None),
        bars(&whole, "INDEX", "BANKNIFTY", None),
        "X holds exactly what an in-order import holds"
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ── IA-GR-19's second half (round 5, SF-10) ────────────────────────────────

/// A day whose last line is the hole `repair_impossible` or
/// `bars_differ_held` is read by no run, with or without a repair reason:
/// each run reads 0 files of it, writes no line and no bar, counts it
/// `holes_unrepairable` (never a day already done) and names it in
/// `holes_in_range`. For a CASH (indices) and an OPTION contract-day.
#[test]
fn a_hole_no_repair_can_fill_is_never_read_again() {
    for kind in [ImportKind::Indices, ImportKind::Options] {
        for reason in ["repair_impossible", "bars_differ_held"] {
            let root = scratch(&format!("ia16-gr19-{}-{reason}", kind.as_str()));
            world(&root, &[d1(), d2(), d3()]);
            let store = root.join("S");
            let first = plain(&root, kind, (d1(), d3()), &store);
            assert!(first.failures.is_empty(), "{kind:?} {:?}", first.failures);
            // The day's key closed by a later run as this hole, as the run
            // that met it writes it (`stage=file`).
            let done = line_of(&store, kind.as_str(), "2024-04-02");
            let fields: Vec<&str> = done.split(' ').collect();
            let run = done.rsplit_once(" run=").unwrap().1;
            let hole = format!(
                "incomplete {} {} {} definition={BAR_DEFINITION} files=1 stage=file reason={reason} run={run}\n",
                fields[1], fields[2], fields[3]
            );
            let mut text = journal(&store);
            text.push_str(&hole);
            std::fs::write(journal_path(&store), &text).unwrap();
            let before = (journal(&store), tree(&store.join("bars")));
            for asked in [false, true] {
                let options = RunOptions {
                    repair_reason: asked.then_some(REASON),
                    ..RunOptions::default()
                };
                let again = import_with(&root, (kind, d1(), d3()), (&store, &[]), options).unwrap();
                assert_eq!(again.files, 0, "{kind:?} {reason} asked={asked}: {again:?}");
                assert_eq!(again.holes_unrepairable, 1, "{kind:?} {reason} {asked}");
                assert_eq!(again.holes_skipped, 0, "{kind:?} {reason} {asked}");
                assert_eq!(again.days_skipped, 2, "{kind:?} {reason}: never a day done");
                assert_eq!(
                    again.holes_in_range,
                    vec![(d2(), reason.to_owned())],
                    "{kind:?} {asked}"
                );
                assert_eq!(again.months_repaired, 0, "{kind:?} {reason} {asked}");
                assert_eq!(
                    (journal(&store), tree(&store.join("bars"))),
                    before,
                    "{kind:?} {reason} asked={asked}: no line, no bar"
                );
            }
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}

// ── IA-GR-40: a member key never writes the whole day's ledger line ────────

/// A day closed under the swept surface, read again for a wider scope's new
/// members under member keys (`<filter>+<member>`, WP-12f): the stocks
/// day's member keys (the BE spell the stored scope adds) write NO D-3335
/// line, so the day's `*` line stays the base close's byte for byte; the
/// indices day's member key (INDIA VIX) writes its own index's line and no
/// other. The ledger is only appended to.
#[test]
fn a_member_key_close_never_writes_the_whole_day_ledger_line() {
    use crate::gdfl_cm::CmScope;
    for kind in [ImportKind::Indices, ImportKind::Stocks] {
        let root = scratch(&format!("ia16-gr40-{}", kind.as_str()));
        if kind == ImportKind::Stocks {
            // Two shares of their own and a trade-for-trade (`.BE`) spell of
            // a third, which only the stored scope reads (DB-B06).
            let spell = |d: Day| -> DayPlan {
                let file = |stem: &str| (entry_of(Tree::Stocks, d, stem), stock_file(stem, d));
                (
                    d,
                    Tree::Stocks,
                    vec![
                        file("RELIANCE.NSE"),
                        file("SBIN.NSE"),
                        file("ADANIPOWER.BE.NSE"),
                    ],
                )
            };
            let _ = world_of(&root, &[spell(d1()), spell(d2())]);
        } else {
            world(&root, &[d1(), d2()]);
        }
        let store = root.join("S");
        let run = Run {
            kind,
            from: d1(),
            to: d2(),
            only: &[],
            store_root: &store,
        };
        let swept = run_cm(&TickStore::new(&root.join("ts")), &run).unwrap();
        assert!(swept.failures.is_empty(), "{kind:?} {:?}", swept.failures);
        let path = quality::dayflags_path(&store);
        let before = std::fs::read_to_string(&path).unwrap();
        assert!(!before.is_empty(), "{kind:?}: the base closes wrote lines");
        let stored = std::sync::Arc::new(CmScope::stored(std::iter::empty::<&str>()).unwrap());
        let wide = run_cm(&TickStore::new(&root.join("ts")).with_scope(stored), &run).unwrap();
        assert!(wide.failures.is_empty(), "{kind:?} {:?}", wide.failures);
        assert!(wide.members_reopened > 0, "{kind:?}: {wide:?}");
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(
            after.starts_with(&before),
            "{kind:?}: appended to, never rewritten"
        );
        let added: Vec<&str> = after[before.len()..].lines().collect();
        if kind == ImportKind::Stocks {
            assert!(
                added.is_empty(),
                "a member key writes no stocks line: {added:?}"
            );
        } else {
            assert_eq!(added.len(), 2, "one per day, the new index's: {added:?}");
            for line in &added {
                let scope = line.split(' ').nth(3).unwrap();
                assert!(
                    scope.contains("VIX"),
                    "the member's own index and no other: {line}"
                );
            }
        }
        for on in ["2024-04-01", "2024-04-02"] {
            let star = |text: &str| -> Vec<String> {
                text.lines()
                    .filter(|line| line.contains(&format!(" {on} * ")))
                    .map(str::to_owned)
                    .collect()
            };
            assert_eq!(
                star(&after),
                star(&before),
                "{kind:?} {on}: the `*` line stands"
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

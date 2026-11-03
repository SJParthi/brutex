#![cfg(test)]
//! BASE-COV-PULL-PS3 (G3): the options import's lineage at its edges: ledger
//! lines that do not parse, ledgers that cannot be read, a lot of a contract
//! expired before the run, the kind of an event a ledger recorded no `event`
//! for, a merge that retires nothing or keeps the recorded pairs, a key that
//! names no expiry, and a warm-up that walks back to the first day there is.
//! Every symbol, contract, lot and open interest here is invented (`ZZQ...`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_fixtures::{put, scratch};
use crate::gdfl_lineage::LotEvidence;

fn d(y: u16, m: u8, dd: u8) -> Day {
    Day::new(y, m, dd).unwrap()
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

/// The contract segment of `strike` paisa, `side`, expiring `expiry`.
fn key(expiry: Day, strike: i64, side: OptionSide) -> Contract {
    ContractDay {
        expiry,
        strike,
        side,
        lot: LotEvidence::NoTrade,
        first_oi: None,
        close_oi: None,
    }
    .contract()
    .unwrap()
}

/// What a merge's re-read answers: the rows of a file, or the source's
/// refusal.
type Rows = Result<Vec<(u32, i64)>, String>;

/// A re-read for a merge: answers `rows` for every file it is asked for,
/// noting each file asked in `asked`.
fn reread_with<'a>(rows: &'a Rows, asked: &'a mut Vec<usize>) -> impl FnMut(usize) -> Rows + 'a {
    move |file| {
        asked.push(file);
        rows.clone()
    }
}

/// The ex-date every event here is of, and the expiry its keys share.
fn ex() -> Day {
    d(2020, 3, 2)
}

fn expiry() -> Day {
    d(2020, 3, 26)
}

/// EVERY LEDGER LINE THAT DOES NOT PARSE AS ITS OWN KIND CHANGES NOTHING: an
/// `alias`, `event`, `lineage`, `stopped` and `stitched` line that is not the
/// record its first word names, and a line of no kind, leave the aliases and
/// the retired keys as the one good line among them makes them, and each is
/// held as seen so no run writes it again.
#[test]
fn a_ledger_line_that_does_not_parse_as_its_kind_changes_nothing() {
    let store = scratch("cov-lineage-unparsed");
    let stopped_key = key(expiry(), 120_000, OptionSide::Call);
    let good = StoppedRecord {
        underlying: sym("ZZQA"),
        key: stopped_key,
        ex_date: ex(),
    }
    .line();
    let junk = [
        "alias ZZQA junk",
        "event NSE-FNO-ZZQA junk",
        "lineage NSE-FNO-ZZQA junk",
        "stopped NSE-FNO-ZZQA junk",
        "stitched NSE-FNO-ZZQA junk",
        "no-such-kind ZZQA",
    ];
    let mut text = junk.join("\n");
    text.push('\n');
    text.push_str(&good);
    text.push('\n');
    put(&store, "imports/gdfl.lineage", text.as_bytes());
    let run = LineageRun::load(&store, ex()).unwrap();
    assert!(run.aliased.is_empty());
    assert_eq!(run.underlyings.len(), 1, "the one good line's underlying");
    let state = &run.underlyings[&sym("ZZQA")];
    assert_eq!(state.retired.len(), 1);
    let entry = &state.retired[&ex().days_from_epoch()];
    assert_eq!(entry.stopped, HashSet::from([stopped_key]));
    assert!(entry.forward.is_empty());
    assert_eq!((entry.kind, entry.recorded), (None, false));
    for line in junk {
        assert!(run.seen.contains(line), "{line}");
    }
    std::fs::remove_dir_all(store).unwrap();
}

/// A LEDGER THAT EXISTS AND CANNOT BE READ REFUSES THE RUN BY ITS PATH, the
/// lineage ledger and the lot ledger alike (here a folder at the name),
/// never read as an empty ledger.
#[test]
fn a_ledger_that_cannot_be_read_refuses_the_run_by_its_path() {
    let store = scratch("cov-lineage-unreadable");
    for (path, other) in [
        (lineage_path(&store), lots_path(&store)),
        (lots_path(&store), lineage_path(&store)),
    ] {
        let _gone = std::fs::remove_dir_all(&other);
        std::fs::create_dir_all(&path).unwrap();
        let why = LineageRun::load(&store, ex()).unwrap_err();
        assert!(why.starts_with(&format!("{}: ", path.display())), "{why}");
        std::fs::remove_dir_all(&path).unwrap();
    }
    assert!(LineageRun::load(&store, ex()).is_ok(), "neither: empty");
    std::fs::remove_dir_all(store).unwrap();
}

/// A LOT LINE THAT DOES NOT PARSE IS PASSED OVER, and a lot of a contract
/// that expired before the run's first day is not seeded (the run will never
/// see it again), while one expiring ON that day is: the boundary.
#[test]
fn a_lot_line_unparsed_or_of_a_contract_expired_before_the_run_is_not_seeded() {
    let store = scratch("cov-lineage-lots");
    let early = key(d(2020, 3, 5), 120_000, OptionSide::Put);
    let late = key(expiry(), 120_000, OptionSide::Put);
    let lot = |contract| LotRecord {
        underlying: sym("ZZQA"),
        contract,
        from: ex(),
        lot: 75,
    };
    let text = format!(
        "lot NSE-FNO-ZZQA junk\n{}\n{}\nlot\n",
        lot(early).line(),
        lot(late).line()
    );
    put(&store, "imports/gdfl.lots", text.as_bytes());
    let on_expiry = LineageRun::load(&store, d(2020, 3, 5)).unwrap();
    assert_eq!(on_expiry.lots.lot_of(sym("ZZQA"), early), Some(75));
    assert_eq!(on_expiry.lots.lot_of(sym("ZZQA"), late), Some(75));
    let after = LineageRun::load(&store, d(2020, 3, 6)).unwrap();
    assert_eq!(after.lots.lot_of(sym("ZZQA"), early), None, "expired");
    assert_eq!(after.lots.lot_of(sym("ZZQA"), late), Some(75));
    std::fs::remove_dir_all(store).unwrap();
}

/// THE KIND OF AN EVENT THE LEDGER RECORDED NO `event` FOR is what its pairs
/// show: a lot step when the lots differ; with equal lots, the shift of a
/// pair that moved its strike; and a re-listing when no pair did (nothing
/// paired, a pair under another expiry at the same strike, or a key no
/// segment rule reads). A recorded kind is always the one in force.
#[test]
fn an_unrecorded_events_kind_is_what_its_pairs_show() {
    let mut entry = Retired::new(ex());
    assert_eq!(entry.kind(), EventKind::Relisted, "nothing paired");
    entry.pair(
        key(expiry(), 120_000, OptionSide::Call),
        key(expiry(), 119_100, OptionSide::Call),
    );
    entry.lots = (75, 150);
    assert_eq!(entry.kind(), EventKind::LotStep);
    entry.lots = (75, 75);
    assert_eq!(entry.kind(), EventKind::StrikeOffset { shift: 900 });
    entry.kind = Some(EventKind::Relisted);
    assert_eq!(entry.kind(), EventKind::Relisted, "the recorded kind");
    let mut unmoved = Retired::new(ex());
    unmoved.lots = (75, 75);
    unmoved.pair(
        key(expiry(), 120_000, OptionSide::Call),
        key(d(2020, 4, 30), 120_000, OptionSide::Call),
    );
    unmoved.pair(
        Contract::parse("ZZQ").unwrap(),
        key(expiry(), 60_000, OptionSide::Put),
    );
    assert_eq!(unmoved.kind(), EventKind::Relisted);
}

/// A key that names no expiry (no contract segment the store writes) keeps
/// an event alive no longer than it was; a dated key moves its end to its
/// own expiry, and an earlier one does not move it back.
#[test]
fn a_key_that_names_no_expiry_keeps_an_event_alive_no_longer() {
    let mut entry = Retired::new(ex());
    assert!(entry.stop(Contract::parse("ZZQ").unwrap()));
    assert_eq!(entry.until, ex());
    assert!(entry.stop(key(expiry(), 120_000, OptionSide::Call)));
    assert_eq!(entry.until, expiry());
    assert!(entry.stop(key(d(2020, 3, 12), 120_000, OptionSide::Call)));
    assert_eq!(entry.until, expiry());
}

/// A MERGE OF AN ACTION THAT RETIRES NOTHING writes nothing and records
/// nothing, and a stitched key the entry does not hold has no line.
#[test]
fn a_merge_of_an_action_that_retires_nothing_writes_nothing() {
    let store = scratch("cov-lineage-merge-none");
    let mut run = LineageRun::load(&store, ex()).unwrap();
    let action = CorporateAction {
        underlying: sym("ZZQA"),
        kind: EventKind::Relisted,
        ex_date: ex(),
        lot_before: 75,
        lot_after: 75,
        lineage: Vec::new(),
        unpaired: Vec::new(),
        stitched: vec![key(expiry(), 120_000, OptionSide::Call)],
    };
    let mut entry = Retired::new(ex());
    let (mut lines, mut asked) = (Vec::new(), Vec::new());
    let got = run.merge(
        &action,
        &mut entry,
        &HashMap::new(),
        &mut reread_with(&Ok(Vec::new()), &mut asked),
        &mut lines,
    );
    assert_eq!(got, Ok(None));
    assert!(asked.is_empty(), "nothing re-read");
    assert!(lines.is_empty());
    assert!(entry.is_empty());
    assert_eq!((entry.kind, entry.recorded), (None, false));
    std::fs::remove_dir_all(store).unwrap();
}

/// COMBINED: a re-listing verdict over an entry whose pairs a ledger
/// recorded with no `event` line, which also stops a key new to it. The
/// re-listing never overrules the pairs: the event recorded is the offset
/// they show, under the lots they were recorded with (not the verdict's),
/// counting both the pair and the new stop, after the new `stopped` line.
#[test]
fn a_relisting_verdict_keeps_the_pairs_and_the_kind_they_show() {
    let store = scratch("cov-lineage-merge-keeps");
    let mut run = LineageRun::load(&store, ex()).unwrap();
    let mut entry = Retired::new(ex());
    entry.pair(
        key(expiry(), 120_000, OptionSide::Call),
        key(expiry(), 119_100, OptionSide::Call),
    );
    entry.lots = (75, 75);
    let stopped = key(expiry(), 130_000, OptionSide::Put);
    let action = CorporateAction {
        underlying: sym("ZZQA"),
        kind: EventKind::Relisted,
        ex_date: ex(),
        lot_before: 50,
        lot_after: 50,
        lineage: Vec::new(),
        unpaired: vec![stopped],
        stitched: Vec::new(),
    };
    let (mut lines, mut asked) = (Vec::new(), Vec::new());
    let got = run
        .merge(
            &action,
            &mut entry,
            &HashMap::new(),
            &mut reread_with(&Ok(Vec::new()), &mut asked),
            &mut lines,
        )
        .unwrap()
        .expect("a new event record");
    assert!(asked.is_empty(), "no pair to re-read");
    assert_eq!(got.kind, EventKind::StrikeOffset { shift: 900 });
    assert_eq!((got.lot_old, got.lot_new), (75, 75));
    assert_eq!((got.paired, got.stopped), (1, 1));
    let stop_line = StoppedRecord {
        underlying: sym("ZZQA"),
        key: stopped,
        ex_date: ex(),
    }
    .line();
    assert_eq!(lines, [stop_line, got.line()]);
    assert_eq!(entry.kind, Some(EventKind::StrikeOffset { shift: 900 }));
    assert!(entry.recorded);
    std::fs::remove_dir_all(store).unwrap();
}

/// A RE-READ THE SOURCE REFUSED REFUSES THE MERGE before any line is noted
/// or any key retired: a pair whose OI crossed unadjusted asks for its new
/// key's file by its listing index, and nothing of the action is kept. The
/// same pair answered is recorded with the rows read.
#[test]
fn a_reread_the_source_refused_refuses_the_merge_before_anything_is_noted() {
    let store = scratch("cov-lineage-merge-refused");
    let mut run = LineageRun::load(&store, ex()).unwrap();
    let (old, new) = (
        key(expiry(), 120_000, OptionSide::Call),
        key(expiry(), 60_000, OptionSide::Call),
    );
    let pair = crate::gdfl_lineage::Lineage {
        underlying: sym("ZZQA"),
        ex_date: ex(),
        old,
        new,
        lot_before: 75,
        lot_after: 150,
        pre_close_oi: Some(4_000),
        oi: OiCarry::Unadjusted,
        reused: false,
    };
    let action = CorporateAction {
        underlying: sym("ZZQA"),
        kind: EventKind::LotStep,
        ex_date: ex(),
        lot_before: 75,
        lot_after: 150,
        lineage: vec![pair.clone()],
        unpaired: Vec::new(),
        stitched: Vec::new(),
    };
    let file_of = HashMap::from([(new, 7)]);
    let refused: Rows = Err("the source refused the file".to_owned());
    let (mut entry, mut lines, mut asked) = (Retired::new(ex()), Vec::new(), Vec::new());
    let got = run.merge(
        &action,
        &mut entry,
        &file_of,
        &mut reread_with(&refused, &mut asked),
        &mut lines,
    );
    assert_eq!(got, refused.map(|_| None));
    assert_eq!(asked, [7]);
    assert!(lines.is_empty());
    assert!(entry.is_empty());
    let rows: Rows = Ok(vec![(33_300, 4_000), (33_360, 8_000)]);
    let got = run.merge(
        &action,
        &mut entry,
        &file_of,
        &mut reread_with(&rows, &mut asked),
        &mut lines,
    );
    assert_eq!(
        got.unwrap().map(|event| event.kind),
        Some(EventKind::LotStep)
    );
    assert_eq!(asked, [7, 7]);
    assert_eq!(lines[0], pair.record(rows.as_deref().unwrap()).line());
    std::fs::remove_dir_all(store).unwrap();
}

/// THE WARM-UP WALKS BACK NO FURTHER THAN THE FIRST DAY THERE IS: three days
/// after 1970-01-01 it stops at the epoch, before any calendar of record,
/// with no session found and nothing wrapped round to the far end of time.
#[test]
fn the_warm_up_stops_at_the_first_day_there_is() {
    let near = Day::from_days(3).unwrap();
    assert_eq!(near, d(1970, 1, 4));
    assert_eq!(warmup_days(near, None), Vec::<Day>::new());
    assert_eq!(
        warmup_days(Day::from_days(0).unwrap(), None),
        Vec::<Day>::new()
    );
}

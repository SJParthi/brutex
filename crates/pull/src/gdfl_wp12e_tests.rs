#![cfg(test)]
//! WP-12e, repair round 1: what the options import DOES with a detected
//! event, a renamed underlying and a filter that names one (D-3354, D-3355;
//! DB-B05, DB-B13). Every contract, lot, price and open interest here is
//! invented; the shapes follow the audit's measured events (a 9.00 strike
//! offset, a re-listing after a scheme) without copying a vendor row.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::Path;

use brutex_core::instrument::OptionSide;
use brutex_core::symbol::Symbol;

use super::lineage::{DayContract, LineageRun, Written};
use super::*;
use crate::expiry_alias::Settlement;
use crate::gdfl_fixtures::{bts, csv, mon, put, row, scratch};
use crate::gdfl_lineage::{
    ContractDay, EventKind, EventRecord, LotEvidence, StoppedRecord, lineage_path, lots_path,
};
use crate::gdfl_nfo::{NfoTickStore, OptionTicker, TickerForm};

fn d(y: u16, m: u8, dd: u8) -> Day {
    Day::new(y, m, dd).unwrap()
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

/// A contract-day with an inferred lot and the two OIs.
fn cd(expiry: Day, strike: i64, side: OptionSide, lot: u64, first: i64, close: i64) -> ContractDay {
    ContractDay {
        expiry,
        strike,
        side,
        lot: LotEvidence::Inferred(lot),
        first_oi: Some(first),
        close_oi: Some(close),
    }
}

/// A distinct, positive open interest per (strike, side).
fn oi_of(strike: i64, side: OptionSide) -> i64 {
    7_000_000 + strike * 3 + i64::from(side == OptionSide::Put)
}

/// Both sides of every strike, each closing on its own OI.
fn ladder(expiry: Day, strikes: &[i64], lot: u64) -> Vec<ContractDay> {
    strikes
        .iter()
        .flat_map(|&k| {
            [OptionSide::Call, OptionSide::Put].map(|side| {
                let oi = oi_of(k, side);
                cd(expiry, k, side, lot, oi, oi)
            })
        })
        .collect()
}

/// The day's files as the lineage reads them.
fn files(underlying: &str, days: &[ContractDay]) -> Vec<DayContract> {
    days.iter()
        .enumerate()
        .map(|(file, day)| DayContract {
            ticker: OptionTicker {
                underlying: sym(underlying),
                contract: day.contract().unwrap(),
                form: TickerForm::Day,
                expiry: day.expiry,
                stated: day.expiry,
                settlement: Settlement::AsStated,
                strike: day.strike,
                side: day.side,
            },
            day: *day,
            file,
        })
        .collect()
}

fn fold(run: &mut LineageRun, on: Day, underlying: &str, days: &[ContractDay]) -> Written {
    run.day(on, &files(underlying, days), &mut |_| Ok(Vec::new()))
        .unwrap()
}

/// The ledger's lines that start with `word `.
fn lines_of(store: &Path, word: &str) -> Vec<String> {
    std::fs::read_to_string(lineage_path(store))
        .unwrap_or_default()
        .lines()
        .filter(|line| line.split(' ').next() == Some(word))
        .map(str::to_owned)
        .collect()
}

/// A 9.00 offset that keeps the lot: every key of `before` moves down 900
/// paisa, every fifth opening on the pre-ex close of the key 9.00 above.
fn shifted(before: &[ContractDay]) -> Vec<ContractDay> {
    before
        .iter()
        .enumerate()
        .map(|(i, old)| {
            let carried = old.close_oi.unwrap();
            let first = if i % 5 == 0 { carried } else { carried + 3_400 };
            cd(
                old.expiry,
                old.strike - 900,
                old.side,
                3_400,
                first,
                first + 1,
            )
        })
        .collect()
}

/// DB-B13, blocking 1. An offset event met by the import leaves, on its
/// ex-date: one `lineage` line per pair, one `stopped` line per pre-ex key
/// with no partner, and one `event` line naming the kind, the shift, the
/// lots and both counts. Before this the event and the unpaired key were
/// written nowhere.
#[test]
fn an_offset_met_by_the_import_is_recorded_with_its_kind_shift_and_counts() {
    let store = scratch("wp12e-offset");
    let (pre, ex) = (d(2020, 2, 19), d(2020, 2, 20));
    let (e1, e2) = (d(2020, 2, 27), d(2020, 3, 26));
    let strikes: Vec<i64> = (0..7).map(|i| 12_000 + i * 500).collect();
    let mut before = ladder(e1, &strikes, 3_400);
    before.extend(ladder(e2, &strikes[..3], 3_400));
    let mut after = shifted(&before);
    // One pre-ex contract did not trade on the ex-date.
    let absent = before[before.len() - 1];
    after.pop();

    let mut run = LineageRun::load(&store, pre).unwrap();
    assert_eq!(fold(&mut run, pre, "OIL", &before).events, 0);
    assert!(lines_of(&store, "event").is_empty());
    let written = fold(&mut run, ex, "OIL", &after);
    assert_eq!((written.actions, written.events), (1, 1));

    let event = lines_of(&store, "event");
    assert_eq!(
        event,
        [format!(
            "event NSE-FNO-OIL ex=2020-02-20 kind=strike_offset shift=900 lot_old=3400 \
             lot_new=3400 paired={} stopped=1 v=1",
            before.len() - 1
        )]
    );
    let record = EventRecord::parse(&event[0]).expect("reads back");
    assert_eq!(record.kind, EventKind::StrikeOffset { shift: 900 });
    assert_eq!(lines_of(&store, "lineage").len(), before.len() - 1);
    let stopped = lines_of(&store, "stopped");
    assert_eq!(
        stopped,
        [StoppedRecord {
            underlying: sym("OIL"),
            key: absent.contract().unwrap(),
            ex_date: ex,
        }
        .line()]
    );

    // IDEMPOTENT: a second run over the same two days writes nothing.
    let ledger = std::fs::read(lineage_path(&store)).unwrap();
    let lots = std::fs::read(lots_path(&store)).unwrap();
    let mut again = LineageRun::load(&store, pre).unwrap();
    let first = fold(&mut again, pre, "OIL", &before);
    let second = fold(&mut again, ex, "OIL", &after);
    assert_eq!((first.lineage, first.lots, first.events), (0, 0, 0));
    assert_eq!((second.lineage, second.lots, second.events), (0, 0, 0));
    assert_eq!(std::fs::read(lineage_path(&store)).unwrap(), ledger);
    assert_eq!(std::fs::read(lots_path(&store)).unwrap(), lots);

    // A RUN THAT HOLDS LESS of the pre-ex state (half the ladder) adds
    // nothing and does not overrule the recorded event.
    let mut partial = LineageRun::load(&store, pre).unwrap();
    fold(&mut partial, pre, "OIL", &before[..before.len() / 2]);
    let less = fold(&mut partial, ex, "OIL", &after);
    assert_eq!((less.lineage, less.events), (0, 0));
    assert_eq!(std::fs::read(lineage_path(&store)).unwrap(), ledger);
    std::fs::remove_dir_all(store).unwrap();
}

/// A run that holds MORE of the pre-ex state than the one that first met the
/// ex-date adds the pairs it alone saw and one more `event` record with the
/// larger counts. Here the thin run holds too few OI carries to prove the
/// offset and records a re-listing that stops its ten keys; the full run
/// proves the offset. The last record of the ex-date is the one in force,
/// and a key first written `stopped` and later paired is PAIRED: a run that
/// restarts afterwards stitches it with the key its contract continues under.
#[test]
fn a_fuller_run_adds_its_pairs_and_a_second_event_record_with_larger_counts() {
    let store = scratch("wp12e-fuller");
    let (pre, ex) = (d(2020, 2, 19), d(2020, 2, 20));
    let e1 = d(2020, 3, 26);
    let strikes: Vec<i64> = (0..10).map(|i| 12_000 + i * 500).collect();
    let before = ladder(e1, &strikes, 3_400);
    let after = shifted(&before);
    let half = before.len() / 2;

    let mut thin = LineageRun::load(&store, pre).unwrap();
    fold(&mut thin, pre, "OIL", &before[..half]);
    assert_eq!(fold(&mut thin, ex, "OIL", &after).events, 1);
    let first = lines_of(&store, "event");
    assert_eq!(
        first,
        [format!(
            "event NSE-FNO-OIL ex=2020-02-20 kind=relisted shift=0 lot_old=3400 lot_new=3400 \
             paired=0 stopped={half} v=1"
        )]
    );
    assert_eq!(lines_of(&store, "stopped").len(), half);

    let mut full = LineageRun::load(&store, pre).unwrap();
    fold(&mut full, pre, "OIL", &before);
    let more = fold(&mut full, ex, "OIL", &after);
    assert_eq!(more.events, 1);
    let both = lines_of(&store, "event");
    assert_eq!(both.len(), 2);
    assert_eq!(both[0], first[0], "the first record is never rewritten");
    assert_eq!(
        both[1],
        format!(
            "event NSE-FNO-OIL ex=2020-02-20 kind=strike_offset shift=900 lot_old=3400 \
             lot_new=3400 paired={} stopped=0 v=1",
            before.len()
        )
    );
    assert_eq!(lines_of(&store, "lineage").len(), before.len());
    assert_eq!(lines_of(&store, "stopped").len(), half, "kept, append-only");

    // The thin run again: it sees a re-listing, which never overrules the
    // recorded pairs. Nothing is written.
    let ledger = std::fs::read(lineage_path(&store)).unwrap();
    let mut thin_again = LineageRun::load(&store, pre).unwrap();
    fold(&mut thin_again, pre, "OIL", &before[..half]);
    let none = fold(&mut thin_again, ex, "OIL", &after);
    assert_eq!((none.lineage, none.events), (0, 0));
    assert_eq!(std::fs::read(lineage_path(&store)).unwrap(), ledger);

    // A restart weeks later: the key the thin run stopped is stitched as the
    // pair the full run measured.
    let later = d(2020, 3, 11);
    let again = before[0];
    let mut resumed = LineageRun::load(&store, later).unwrap();
    fold(&mut resumed, later, "OIL", &[again]);
    let key = again.contract().unwrap();
    let moved = cd(e1, again.strike - 900, again.side, 1, 0, 0)
        .contract()
        .unwrap();
    assert_eq!(
        lines_of(&store, "stitched"),
        [format!(
            "stitched NSE-FNO-OIL-{key} ex=2020-02-20 before={moved} after=- v=1"
        )]
    );
    std::fs::remove_dir_all(store).unwrap();
}

/// DB-B13 and DB-B03, blocking 1. A re-listing pairs nothing, so before this
/// the import wrote NOTHING for it: no ledger line, no log event, and a run
/// that restarted more than the warm-up after the ex-date no longer knew the
/// old generation's keys. Now the ex-date writes one `event` line and one
/// `stopped` line per pre-ex key, and a later run rebuilds the retired keys
/// from them: an old-generation text listed again weeks later is `stitched`
/// and its lot starts a new lineage, wherever the run started.
#[test]
fn a_relisting_is_recorded_and_its_retired_keys_survive_a_restart() {
    let store = scratch("wp12e-relisted");
    let (pre, ex) = (d(2020, 3, 3), d(2020, 3, 4));
    let (e1, e2) = (d(2020, 3, 26), d(2020, 4, 30));
    let old: Vec<i64> = (0..4).map(|i| 70_000 + i * 2_000).collect();
    let mut before = ladder(e1, &old, 900);
    before.extend(ladder(e2, &old[..1], 900));
    let fresh = |expiry, n: i64| -> Vec<ContractDay> {
        (0..n)
            .flat_map(|i| {
                [OptionSide::Call, OptionSide::Put]
                    .map(|side| cd(expiry, 27_000 + i * 1_000, side, 900, 900 * (i + 1), 1_800))
            })
            .collect()
    };
    let mut after = fresh(e1, 3);
    after.extend(fresh(e2, 1));

    let mut run = LineageRun::load(&store, pre).unwrap();
    fold(&mut run, pre, "TATACHEM", &before);
    let written = fold(&mut run, ex, "TATACHEM", &after);
    assert_eq!((written.actions, written.events), (1, 1));
    assert_eq!(
        lines_of(&store, "event"),
        [
            "event NSE-FNO-TATACHEM ex=2020-03-04 kind=relisted shift=0 lot_old=900 lot_new=900 \
          paired=0 stopped=10 v=1"
        ]
    );
    assert_eq!(lines_of(&store, "stopped").len(), before.len());
    assert!(lines_of(&store, "lineage").is_empty(), "nothing is paired");
    assert!(lines_of(&store, "stitched").is_empty());
    drop(run);

    // A NEW RUN, twelve days on: far past the warm-up, holding no pre-ex
    // state. An old-generation text is listed again with another lot.
    let later = d(2020, 3, 16);
    let again = before[0];
    let relisted = cd(again.expiry, again.strike, again.side, 450, 450, 900);
    let mut resumed = LineageRun::load(&store, later).unwrap();
    let got = fold(&mut resumed, later, "TATACHEM", &[relisted]);
    let key = again.contract().unwrap();
    assert_eq!(
        lines_of(&store, "stitched"),
        [format!(
            "stitched NSE-FNO-TATACHEM-{key} ex=2020-03-04 before=- after=- v=1"
        )]
    );
    // Its lot is first seen again: 450, not the common divisor with 900.
    let lots = std::fs::read_to_string(lots_path(&store)).unwrap();
    let of_key: Vec<&str> = lots
        .lines()
        .filter(|line| line.contains(&format!("NSE-FNO-TATACHEM-{key} ")))
        .collect();
    assert_eq!(of_key.len(), 2, "{of_key:?}");
    assert!(of_key[0].contains("from=2020-03-03 lot=900 "), "{of_key:?}");
    assert!(of_key[1].contains("from=2020-03-16 lot=450 "), "{of_key:?}");
    assert_eq!((got.lineage, got.lots), (1, 1));

    // AND AGAIN, from yet another start: nothing new, and no second restart.
    let next = d(2020, 3, 17);
    let mut third = LineageRun::load(&store, later).unwrap();
    let same = fold(&mut third, later, "TATACHEM", &[relisted]);
    assert_eq!((same.lineage, same.lots), (0, 0));
    let on = fold(&mut third, next, "TATACHEM", &[relisted]);
    assert_eq!((on.lineage, on.lots), (0, 0));

    // A RUN OVER DAYS BEFORE THE EX-DATE reads the event back too, and it
    // has retired nothing yet: the pre-ex text is not stitched there.
    let mut earlier = LineageRun::load(&store, pre).unwrap();
    let before_ex = fold(&mut earlier, pre, "TATACHEM", &before);
    assert_eq!((before_ex.lineage, before_ex.lots), (0, 0));
    assert_eq!(lines_of(&store, "stitched").len(), 1);
    std::fs::remove_dir_all(store).unwrap();
}

/// A ledger written before `event` and `stopped` records existed names a
/// retired key only in its `lineage` and `stitched` lines. They are read
/// back the same way, and the ex-date's `event` record is written by the
/// next run that meets it.
#[test]
fn a_ledger_without_event_records_still_rebuilds_the_retired_keys() {
    let store = scratch("wp12e-legacy");
    let (pre, ex) = (d(2020, 2, 19), d(2020, 2, 20));
    let e1 = d(2020, 3, 26);
    let strikes: Vec<i64> = (0..7).map(|i| 12_000 + i * 500).collect();
    let before = ladder(e1, &strikes, 3_400);
    let after = shifted(&before);
    let mut run = LineageRun::load(&store, pre).unwrap();
    fold(&mut run, pre, "OIL", &before);
    fold(&mut run, ex, "OIL", &after);
    drop(run);
    // The ledger as an earlier build left it: no `event`, no `stopped`.
    let path = lineage_path(&store);
    let mut legacy = String::new();
    for line in std::fs::read_to_string(&path).unwrap().lines() {
        if !line.starts_with("event ") && !line.starts_with("stopped ") {
            legacy.push_str(line);
            legacy.push('\n');
        }
    }
    std::fs::write(&path, &legacy).unwrap();

    // Twenty days on, a pre-ex text trades again: stitched, with the key
    // its old contract continues under.
    let later = d(2020, 3, 11);
    let again = before[0];
    let mut resumed = LineageRun::load(&store, later).unwrap();
    fold(&mut resumed, later, "OIL", &[again]);
    let key = again.contract().unwrap();
    let moved = cd(e1, again.strike - 900, again.side, 1, 0, 0)
        .contract()
        .unwrap();
    assert_eq!(
        lines_of(&store, "stitched"),
        [format!(
            "stitched NSE-FNO-OIL-{key} ex=2020-02-20 before={moved} after=- v=1"
        )]
    );
    // The run that meets the ex-date again writes the missing `event` line.
    let mut rerun = LineageRun::load(&store, pre).unwrap();
    fold(&mut rerun, pre, "OIL", &before);
    assert_eq!(fold(&mut rerun, ex, "OIL", &after).events, 1);
    assert_eq!(
        lines_of(&store, "event"),
        [format!(
            "event NSE-FNO-OIL ex=2020-02-20 kind=strike_offset shift=900 lot_old=3400 \
             lot_new=3400 paired={} stopped=0 v=1",
            before.len()
        )]
    );
    std::fs::remove_dir_all(store).unwrap();
}

// ── The renamed underlying (D-3354; DB-B05, blocking 2 and 3) ───────────────

const NINE_FIFTEEN: u32 = 9 * 3_600 + 15 * 60;

/// One invented option file.
fn option_file(ticker: &str, on: Day, ltp: &str) -> Vec<u8> {
    let stem = format!("{ticker}.NFO");
    let mut rows = Vec::new();
    for s in (0..22_500).step_by(2_003) {
        rows.push(row(
            &stem,
            on,
            NINE_FIFTEEN + s,
            ltp,
            u64::from(s % 3) * 25,
            1_000 + u64::from(s),
        ));
    }
    csv(&rows)
}

/// A tick store under `root/ts` holding `tickers` on `on`.
fn options_day(root: &Path, on: Day, tickers: &[&str]) {
    let folder = crate::gdfl_nfo::day_folder_name(on);
    let entries: Vec<(String, Vec<u8>)> = tickers
        .iter()
        .map(|t| {
            (
                format!("{folder}\\Options\\{t}.NFO.csv"),
                option_file(t, on, "77.7"),
            )
        })
        .collect();
    let refs: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    put(
        root,
        &format!(
            "ts/options/{:04}/{}_{:04}/GFDLNFO_TICK_{:02}{:02}{:04}.bts",
            on.year(),
            mon(on),
            on.year(),
            on.day(),
            on.month(),
            on.year()
        ),
        &bts(&refs),
    );
}

fn options_run<'a>(store: &'a Path, on: Day, only: &'a [String]) -> Run<'a> {
    Run {
        kind: ImportKind::Options,
        from: on,
        to: on,
        only,
        store_root: store,
    }
}

/// Blocking 3. `NIITTECH` became `COFORGE` on 2020-08-20 and the decoder
/// files its options under `COFORGE` (D-3354). A filter naming the OLD
/// symbol matched no file, exited 0 and journaled the day `done` with
/// `files=0`. It is refused now, naming the key to ask for; that key reads
/// the old-name files, and claims an old-name file that does not decode.
#[test]
fn a_filter_naming_an_old_symbol_is_refused_and_the_new_one_reads_its_files() {
    let root = scratch("wp12e-filter");
    let on = d(2020, 8, 19);
    options_day(
        &root,
        on,
        &[
            "NIITTECH27AUG201900CE",
            // A three-decimal strike does not decode; its shape is NIITTECH's.
            "NIITTECH27AUG201900.125CE",
            "NIFTY27AUG2011000CE",
        ],
    );
    let source = NfoTickStore::new(&root.join("ts"));
    let store = root.join("S");
    let old = ["NIITTECH".to_owned()];
    let refused = run_nfo(&source, &options_run(&store, on, &old)).unwrap_err();
    assert_eq!(
        refused,
        ImportRefusal::FilterRenamed {
            name: "NIITTECH".into(),
            filed: "COFORGE".into(),
        }
    );
    assert_eq!(refused.code(), "filter_renamed");
    assert!(refused.to_string().contains("ask for COFORGE"), "{refused}");
    assert!(
        !journal_path(&store).exists(),
        "a refused filter journals no day"
    );

    let new = ["COFORGE".to_owned()];
    let got = run_nfo(&source, &options_run(&store, on, &new)).unwrap();
    assert_eq!(
        (got.files, got.files_refused, got.files_skipped),
        (1, 1, 1),
        "the old-name file is read, the undecodable one refused, NIFTY's not wanted"
    );
    let fno = store.join("bars/gdfl/NSE/FNO");
    assert!(fno.join("COFORGE/2020-08-27-190000-CE").is_dir());
    assert!(!fno.join("NIITTECH").exists());
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(text.contains(" options 2020-08-19 COFORGE "), "{text}");

    // A capital-market filter is not refused for an old name: `cli` hands
    // both names of a renamed share, and an index key has no change.
    for kind in [ImportKind::Stocks, ImportKind::Indices] {
        let run = Run {
            kind,
            ..options_run(&store, on, &old)
        };
        assert_eq!(run.check_filter(), Ok(()), "{kind:?}");
    }
    // Every old symbol of the table is refused with its NEWEST symbol: a
    // chain is walked to its end (LTI -> LTIM -> LTM).
    for row in brutex_core::alias::LINEAGE.rows() {
        let only = [row.old.to_owned()];
        let filed = brutex_core::alias::LINEAGE.current(row.old);
        assert_ne!(filed, row.old);
        assert_eq!(
            options_run(&store, on, &only).check_filter(),
            Err(ImportRefusal::FilterRenamed {
                name: row.old.to_owned(),
                filed: filed.to_owned(),
            })
        );
        let newest = [filed.to_owned()];
        assert_eq!(options_run(&store, on, &newest).check_filter(), Ok(()));
    }
    let lti = ["LTI".to_owned()];
    assert_eq!(
        options_run(&store, on, &lti).check_filter(),
        Err(ImportRefusal::FilterRenamed {
            name: "LTI".into(),
            filed: "LTM".into(),
        })
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Blocking 2. A store filled before D-3354 holds a renamed underlying's
/// options under the OLD symbol. The next import day would file the same
/// contracts under the new one: a second key for a contract the store
/// already holds, on a day the exchange changed nothing. The run is refused
/// before it reads or writes anything, naming every such directory; a run
/// whose filter cannot touch them is not.
#[test]
fn a_store_holding_an_old_symbols_options_refuses_the_run_that_would_split_it() {
    let root = scratch("wp12e-split");
    let on = d(2020, 9, 16);
    options_day(
        &root,
        on,
        &["TATAMOTORS24SEP20150CE", "NIFTY24SEP2011500CE"],
    );
    let source = NfoTickStore::new(&root.join("ts"));
    let store = root.join("S");
    let fno = store.join("bars/gdfl/NSE/FNO");
    // As the real store stands: old-name directories, no new-name ones.
    for old in ["TATAMOTORS", "LTI", "NIITTECH"] {
        std::fs::create_dir_all(fno.join(old).join("2020-09-24-15000-CE")).unwrap();
    }
    let before = tree_names(&store);

    let refused = run_nfo(&source, &options_run(&store, on, &[])).unwrap_err();
    assert_eq!(
        refused,
        ImportRefusal::UnderlyingSplit {
            // In the table's own order.
            held: vec![
                ("NIITTECH".into(), "COFORGE".into()),
                ("LTI".into(), "LTM".into()),
                ("TATAMOTORS".into(), "TMPV".into()),
            ],
        }
    );
    assert_eq!(refused.code(), "underlying_split");
    let said = refused.to_string();
    for word in [
        "TATAMOTORS (filed under TMPV)",
        "LTI (filed under LTM)",
        "Supersede",
    ] {
        assert!(said.contains(word), "{said}");
    }
    assert_eq!(tree_names(&store), before, "nothing was read or written");

    // The key the run would open is refused too; another underlying is not.
    let tmpv = ["TMPV".to_owned()];
    assert!(matches!(
        run_nfo(&source, &options_run(&store, on, &tmpv)),
        Err(ImportRefusal::UnderlyingSplit { held }) if held == [("TATAMOTORS".to_owned(), "TMPV".to_owned())]
    ));
    let nifty = ["NIFTY".to_owned()];
    let got = run_nfo(&source, &options_run(&store, on, &nifty)).unwrap();
    assert_eq!((got.files, got.files_skipped), (1, 1));
    assert!(!fno.join("TMPV").exists());
    std::fs::remove_dir_all(root).unwrap();
}

/// WHAT LIFTS THAT REFUSAL is the ledger, never a deletion (the store is
/// append-only): once every contract under an old symbol is recorded
/// superseded or retired, its directory is history and the run files beside
/// it. A stray name is no contract; a ledger that cannot be read whole
/// cannot say a directory is history, and refuses the run.
#[test]
fn an_old_symbols_directory_blocks_only_while_it_holds_a_live_contract() {
    let root = scratch("wp12e-split-lifted");
    let on = d(2020, 9, 16);
    options_day(&root, on, &["TATAMOTORS24SEP20150CE"]);
    let source = NfoTickStore::new(&root.join("ts"));
    let store = root.join("S");
    let fno = store.join("bars/gdfl/NSE/FNO");
    for old in ["TATAMOTORS", "LTI", "NIITTECH"] {
        std::fs::create_dir_all(fno.join(old).join("2020-09-24-15000-CE")).unwrap();
    }
    let tmpv = ["TMPV".to_owned()];
    // TATAMOTORS' one contract is superseded, LTI's retired; NIITTECH's is
    // still live.
    let held = cd(d(2020, 9, 24), 15_000, OptionSide::Call, 1, 0, 0)
        .contract()
        .unwrap();
    let moved = cd(d(2020, 9, 24), 15_100, OptionSide::Call, 1, 0, 0)
        .contract()
        .unwrap();
    let superseded = crate::gdfl_lineage::SupersededRecord {
        underlying: sym("TATAMOTORS"),
        old: held,
        new: moved,
        kind: crate::expiry_alias::AliasKind::VendorRedated,
        months: 1,
        bars: 4,
        digest: [3; 32],
    };
    let retired = crate::gdfl_lineage::RetiredRecord {
        underlying: sym("LTI"),
        key: held,
        why: crate::gdfl_lineage::RetiredWhy::ExpiryUnsettled,
        day: on,
        median: None,
        days: 1,
        bars: 4,
    };
    let ledger = lineage_path(&store);
    std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
    std::fs::write(
        &ledger,
        format!("{}\n{}\n", superseded.line(), retired.line()),
    )
    .unwrap();
    assert_eq!(
        options_run(&store, on, &[]).split_underlyings(),
        Ok(vec![("NIITTECH".to_owned(), "COFORGE".to_owned())])
    );
    // A stray name is no contract, and an empty directory holds none.
    std::fs::remove_dir_all(fno.join("NIITTECH")).unwrap();
    std::fs::create_dir_all(fno.join("NIITTECH")).unwrap();
    std::fs::write(fno.join("NIITTECH/.DS_Store"), b"x").unwrap();
    std::fs::create_dir_all(fno.join("PVR")).unwrap();
    assert_eq!(
        options_run(&store, on, &[]).split_underlyings(),
        Ok(Vec::new())
    );
    // The run now files the old-name file under the new key, beside the
    // superseded directory, which is left exactly as it was.
    let all = run_nfo(&source, &options_run(&store, on, &tmpv)).unwrap();
    assert_eq!(all.files, 1);
    assert!(fno.join("TMPV/2020-09-24-15000-CE").is_dir());
    assert!(fno.join("TATAMOTORS/2020-09-24-15000-CE").is_dir());

    // A ledger that cannot be read whole cannot say a directory is history:
    // the run is refused by name, never waved through.
    std::fs::create_dir_all(fno.join("PVR/2020-09-24-15000-CE")).unwrap();
    std::fs::write(&ledger, "superseded not a record\n").unwrap();
    assert!(matches!(
        options_run(&store, on, &[]).split_underlyings(),
        Err(ImportRefusal::Lineage { .. })
    ));
    // A spot run never looks: it files no option.
    let spot = Run {
        kind: ImportKind::Stocks,
        ..options_run(&store, on, &[])
    };
    assert_eq!(spot.split_underlyings(), Ok(Vec::new()));
    std::fs::remove_dir_all(root).unwrap();
}

/// Every path under `dir`, sorted.
fn tree_names(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path.clone());
            }
            out.push(path);
        }
    }
    out.sort();
    out
}

/// p50, p99 and max of `samples`, microseconds.
fn spread(samples: &mut [u128]) -> (u128, u128, u128) {
    samples.sort_unstable();
    let at = |q: usize| samples[(samples.len() * q / 100).min(samples.len() - 1)];
    (at(50), at(99), samples[samples.len() - 1])
}

/// A ledger of `events` strike-offset events of `ex`, one underlying each:
/// 40 pairs, 10 stopped keys and the event record (51 lines an event).
fn synthetic_ledger(events: usize, ex: Day, expiry: Day) -> String {
    let mut text = String::new();
    for u in 0..events {
        let underlying = sym(&format!("ZZ{u:05}"));
        let key = |strike: i64| {
            cd(expiry, strike, OptionSide::Call, 1, 0, 0)
                .contract()
                .unwrap()
        };
        for i in 0..40_i64 {
            let record = crate::gdfl_lineage::LineageRecord {
                underlying,
                old: key(20_000 + i * 500),
                new: key(20_000 + i * 500 - 900),
                ex_date: ex,
                factor: (1, 1),
                lot_old: 3_400,
                lot_new: 3_400,
                oi_unadjusted_until: 0,
            };
            text.push_str(&record.line());
            text.push('\n');
        }
        for i in 0..10_i64 {
            let record = StoppedRecord {
                underlying,
                key: key(90_000 + i * 500),
                ex_date: ex,
            };
            text.push_str(&record.line());
            text.push('\n');
        }
        let record = EventRecord {
            underlying,
            ex_date: ex,
            kind: EventKind::StrikeOffset { shift: 900 },
            lot_old: 3_400,
            lot_new: 3_400,
            paired: 40,
            stopped: 10,
        };
        text.push_str(&record.line());
        text.push('\n');
    }
    text
}

/// A MEASUREMENT (D-3355, D-3354), printed for the report; run with
/// `--release -- --ignored --nocapture`. `LineageRun::load` reads every
/// ledger line once, so it tracks the ledger (10x the lines, about 10x the
/// time): bounded by the ledger, once per run, named in `docs/06-limits.md`.
/// A resumed day's fold does NOT track it: its cost at 102,000 ledger lines
/// is within 3x of its cost at 10,200 (the gate).
#[test]
#[ignore = "a measurement: run with --release -- --ignored --nocapture"]
fn the_ledger_load_and_a_resumed_day_are_timed() {
    let (ex, later, expiry) = (d(2020, 2, 20), d(2020, 3, 11), d(2020, 3, 26));
    let mut folds = Vec::new();
    for events in [200_usize, 2_000] {
        let store = scratch("wp12e-timed");
        let text = synthetic_ledger(events, ex, expiry);
        let lines = text.lines().count();
        let path = lineage_path(&store);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();

        let mut loads: Vec<u128> = (0..30)
            .map(|_| {
                let at = std::time::Instant::now();
                let run = LineageRun::load(&store, later).unwrap();
                let took = at.elapsed().as_micros();
                assert!(run.last_day().is_none());
                took
            })
            .collect();
        let (p50, p99, max) = spread(&mut loads);
        println!(
            "LineageRun::load, {lines} ledger lines: n=30 p50 {p50} us p99 {p99} us max {max} us"
        );

        // One underlying's 400 contracts on a resumed day, 50 of them
        // retired keys traded again. The first fold writes; the rest are the
        // steady state.
        let day_files: Vec<ContractDay> = (0..400_i64)
            .map(|i| cd(expiry, 20_000 + i * 500, OptionSide::Call, 3_400, 10, 20))
            .collect();
        let mut run = LineageRun::load(&store, later).unwrap();
        let first = fold(&mut run, later, "ZZ00007", &day_files);
        assert!(first.lineage >= 50, "the 50 reused texts are stitched");
        let contracts = files("ZZ00007", &day_files);
        let mut days: Vec<u128> = (0..500)
            .map(|_| {
                let at = std::time::Instant::now();
                let written = run.day(later, &contracts, &mut |_| Ok(Vec::new())).unwrap();
                let took = at.elapsed().as_micros();
                assert_eq!((written.lineage, written.lots), (0, 0));
                took
            })
            .collect();
        let (p50, p99, max) = spread(&mut days);
        println!(
            "LineageRun::day, 400 contracts, {lines} ledger lines: n=500 p50 {p50} us p99 {p99} us max {max} us"
        );
        folds.push(p50);
        std::fs::remove_dir_all(store).unwrap();
    }
    assert!(
        folds[1] <= folds[0].max(1) * 3,
        "a resumed day's fold tracked the ledger: {folds:?}"
    );
}

/// A MEASUREMENT (D-3354): the split probe, timed on `BRUTEX_WP12E_STORE`
/// when set (read-only: it opens directories and reads the ledger), else on
/// an empty store. It answers the same every time.
#[test]
#[ignore = "a measurement: run with --release -- --ignored --nocapture"]
fn the_split_probe_is_timed() {
    let later = d(2020, 9, 16);
    let real = std::env::var_os("BRUTEX_WP12E_STORE").map(std::path::PathBuf::from);
    let store = real.clone().unwrap_or_else(|| scratch("wp12e-timed-split"));
    let run = options_run(&store, later, &[]);
    let held = run.split_underlyings().unwrap();
    let mut probes: Vec<u128> = (0..200)
        .map(|_| {
            let at = std::time::Instant::now();
            let got = run.split_underlyings().unwrap();
            let took = at.elapsed().as_micros();
            assert_eq!(got, held);
            took
        })
        .collect();
    let (p50, p99, max) = spread(&mut probes);
    println!(
        "split probe over {} ({} old-symbol directories held): n=200 p50 {p50} us p99 {p99} us max {max} us",
        store.display(),
        held.len()
    );
    if real.is_none() {
        let _ = std::fs::remove_dir_all(store);
    }
}

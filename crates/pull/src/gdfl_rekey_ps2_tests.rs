//! BASE-COV-PULL-PS2 (fork B): the re-key's arms the suite left unreached,
//! each driven by the input that reaches it and asserted by what it answers.
//! A child of `gdfl_rekey::tests`, so it builds its stores with that
//! module's fixtures. Every bar, price, count and digest is invented; no
//! vendor row.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail; one test per arm, each holding its extremes"
)]

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use brutex_core::instrument::{Contract, Exchange, Segment};
use store::file::BarFile;
use store::path::{StorePath, Timeframe, YearMonth};

use super::*;
use crate::gdfl_lineage::{RetiredRecord, RetiredWhy, SupersededRecord, lineage_path};
use crate::manifest::EntryKey;

/// The `.bin` path of one month of one NIFTY key.
fn month_file(root: &Path, contract: &str, month: YearMonth) -> std::path::PathBuf {
    let nifty = sym("NIFTY");
    StorePath::new(month_parts(&nifty, c(contract), month))
        .unwrap()
        .to_path_buf(root)
}

/// One month of one NIFTY key made empty: its files removed and an empty
/// month created in their place through the store's writer. Only scratch
/// stores are touched.
fn emptied(root: &Path, contract: &str, month: YearMonth) {
    let bin = month_file(root, contract, month);
    let dir = bin.parent().unwrap().to_path_buf();
    let stem = bin.file_stem().unwrap().to_string_lossy().into_owned();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with(&format!("{stem}.")) {
            std::fs::remove_file(&path).unwrap();
        }
    }
    let nifty = sym("NIFTY");
    let at = StorePath::new(month_parts(&nifty, c(contract), month)).unwrap();
    drop(BarFile::open_or_create(root, at, symbol_id(sym("NIFTY"))).unwrap());
}

/// A hand-built move of a NIFTY key.
fn hand_move(from: &str, to: &str, months: &[YearMonth], bars: u64) -> Move {
    Move {
        underlying: sym("NIFTY"),
        from: c(from),
        to: c(to),
        kind: AliasKind::VendorRedated,
        months: months.to_vec(),
        bars,
    }
}

fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).unwrap()
}

// ═══ The ledger as the re-key reads it ══════════════════════════════════════

/// A ledger whose last line has no line break (an append a crash cut short)
/// reads every line before it, and passes over the torn one: a superseded
/// line then a torn retired line reads as the superseded line alone; three
/// whole lines and a torn fourth read as three.
#[test]
fn a_ledger_torn_at_its_tail_reads_every_line_before_the_tear() {
    let moved = pointer(STATED, SETTLED).line();
    let torn = refusal_of(IMPOSSIBLE[0]).line();
    let read = Withdrawn::parse(&format!("{moved}\n{torn}")).unwrap();
    assert_eq!(read.counts(), (1, 0));
    assert!(read.retired(sym("NIFTY"), c(IMPOSSIBLE[0])).is_none());
    let whole = format!(
        "{moved}\n{}\n{}\n{}",
        refusal_of(IMPOSSIBLE[1]).line(),
        refusal_of(IMPOSSIBLE[2]).line(),
        refusal_of(IMPOSSIBLE[0]).line()
    );
    assert_eq!(Withdrawn::parse(&whole).unwrap().counts(), (1, 2));
    // The same text closed by a line break reads its last line too.
    assert_eq!(
        Withdrawn::parse(&format!("{whole}\n")).unwrap().counts(),
        (1, 3)
    );
    // A torn ledger of one line reads empty, and so does an empty one.
    assert_eq!(Withdrawn::parse(&torn).unwrap().counts(), (0, 0));
    assert_eq!(Withdrawn::parse("").unwrap().counts(), (0, 0));
}

/// Of ten cycles of `superseded` lines, the one on the earliest line is
/// named, whatever order the walk meets them in (a hash order).
#[test]
fn of_several_cycles_the_one_on_the_earliest_line_is_named() {
    let mut text = String::new();
    for at in 0..10_i64 {
        let one = format!("2023-06-29-{}-PE", 2_000_000 + at * 500);
        let two = format!("2023-06-29-{}-CE", 2_000_000 + at * 500);
        text.push_str(&pointer(&one, &two).line());
        text.push('\n');
        text.push_str(&pointer(&two, &one).line());
        text.push('\n');
    }
    for _ in 0..4 {
        let why = Withdrawn::parse(&text).unwrap_err();
        assert!(
            why.contains("lines 1, 2 supersede NSE-FNO-NIFTY-2023-06-29-2000000-PE in a cycle"),
            "{why}"
        );
    }
}

/// A ledger re-read under the lock that LOST a re-key line the plan read
/// (no new line in it) is refused, naming the line it no longer holds;
/// narrowed to another underlying, the same pair agrees.
#[test]
fn a_ledger_that_lost_a_line_since_the_plan_is_refused_naming_it() {
    let kept = pointer(STATED, SETTLED).line();
    let lost = refusal_of(IMPOSSIBLE[0]).line();
    let read = Withdrawn::parse(&format!("{kept}\n{lost}\n")).unwrap();
    let fresh = Withdrawn::parse(&format!("{kept}\n")).unwrap();
    let why = read.same_records(&fresh, None).unwrap_err();
    assert!(
        why.contains("the ledger changed since this plan; re-plan"),
        "{why}"
    );
    assert!(
        why.contains(&format!(
            "the plan read {lost:?}, which the ledger no longer holds"
        )),
        "{why}"
    );
    assert!(
        why.contains("(2 re-key line(s) of every underlying"),
        "{why}"
    );
    let nifty: BTreeSet<Symbol> = [sym("NIFTY")].into_iter().collect();
    let why = read.same_records(&fresh, Some(&nifty)).unwrap_err();
    assert!(
        why.contains("of NIFTY when the plan read it, 1 now"),
        "{why}"
    );
    let other: BTreeSet<Symbol> = [sym("BANKNIFTY")].into_iter().collect();
    assert_eq!(read.same_records(&fresh, Some(&other)), Ok(()));
    // Both empty agree; one empty and one not do not.
    assert_eq!(
        Withdrawn::default().same_records(&Withdrawn::default(), None),
        Ok(())
    );
    assert!(read.same_records(&Withdrawn::default(), None).is_err());
}

/// `holds` answers only for an NSE F&O key: the same symbol and contract
/// on another exchange or segment is never withdrawn, nor a key without a
/// contract.
#[test]
fn only_an_nse_fno_key_is_held_withdrawn() {
    let ledger = Withdrawn::parse(&format!("{}\n", pointer(STATED, SETTLED).line())).unwrap();
    let key = |exchange: Exchange, segment: Segment, contract: Option<Contract>| EntryKey {
        contract,
        exchange,
        segment,
        symbol: sym("NIFTY"),
        timeframe: Timeframe::SECOND_1,
        month: ym(2020, 6),
    };
    assert!(ledger.holds(&key(Exchange::Nse, Segment::Fno, Some(c(STATED)))));
    assert!(!ledger.holds(&key(Exchange::Bse, Segment::Fno, Some(c(STATED)))));
    assert!(!ledger.holds(&key(Exchange::Nse, Segment::Cash, Some(c(STATED)))));
    assert!(!ledger.holds(&key(Exchange::Bse, Segment::Cash, Some(c(STATED)))));
    assert!(!ledger.holds(&key(Exchange::Nse, Segment::Fno, None)));
    assert!(!ledger.holds(&key(Exchange::Nse, Segment::Fno, Some(c(SETTLED)))));
}

/// A plan retiring a key the ledger already retires with the SAME account
/// is admitted (the line is already there, so nothing new); with another
/// account it is refused naming both.
#[test]
fn a_plan_repeating_a_recorded_retirement_is_admitted_and_another_account_refused() {
    let record = refusal_of(IMPOSSIBLE[0]);
    let ledger = Withdrawn::parse(&format!("{}\n", record.line())).unwrap();
    let same = Plan {
        retired: vec![record],
        ..Plan::default()
    };
    assert_eq!(ledger.admits(&same), Ok(()));
    let other = RetiredRecord { days: 2, ..record };
    let why = ledger
        .admits(&Plan {
            retired: vec![other],
            ..Plan::default()
        })
        .unwrap_err();
    assert!(why.contains("one retirement has one account"), "{why}");
    assert!(why.contains("ledger line 1"), "{why}");
}

/// An existing ledger that will not read (a directory at its name) is
/// refused naming the file and the host's words; an absent one reads empty.
#[test]
fn a_ledger_that_will_not_read_is_refused_by_name() {
    let scratch = Scratch::new("ps2-ledger-dir");
    let root = scratch.root();
    assert_eq!(withdrawn(root).unwrap().counts(), (0, 0));
    std::fs::create_dir_all(lineage_path(root)).unwrap();
    let why = withdrawn(root).unwrap_err();
    assert!(why.contains("gdfl.lineage could not be read"), "{why}");
    // Not UTF-8: refused too, never read as the part that decoded.
    std::fs::remove_dir(lineage_path(root)).unwrap();
    std::fs::write(lineage_path(root), [0xff, 0xfe, b'\n']).unwrap();
    assert!(withdrawn(root).unwrap_err().contains("could not be read"));
}

/// `changes` counts a move, a retirement or a pending census step, and
/// nothing else.
#[test]
fn a_plan_changes_with_a_move_a_retirement_or_a_pending_census_only() {
    let clear = Plan {
        census: CensusStep::Clear,
        ..Plan::default()
    };
    assert!(!clear.changes());
    assert!(
        Plan {
            moves: vec![hand_move(STATED, SETTLED, &[ym(2020, 6)], 2)],
            ..clear.clone()
        }
        .changes()
    );
    assert!(
        Plan {
            retired: vec![refusal_of(IMPOSSIBLE[0])],
            ..clear.clone()
        }
        .changes()
    );
    let unresolved_only = Plan {
        unresolved: vec![Unresolved {
            key: "k".to_owned(),
            why: "w".to_owned(),
        }],
        ..clear.clone()
    };
    assert!(!unresolved_only.changes());
    assert_eq!(unresolved_only.pending(), 1);
    assert!(Plan::default().changes(), "a census not read is pending");
}

// ═══ The survey ═════════════════════════════════════════════════════════════

/// Everything under `bars/gdfl/NSE/FNO/` that is not an option's store path
/// is counted, and only the first [`NAMED`] are named: 51 names that are no
/// symbol, a stray file in a rung, months named in another spelling. A rung
/// other than 1s is left for the plan to name; an empty rung, and an empty
/// month file, give the key no month.
#[test]
fn the_survey_counts_every_stray_names_the_first_and_skips_empty_months() {
    let scratch = Scratch::new("ps2-strays");
    let root = scratch.root();
    let base = root.join("bars/gdfl/NSE/FNO");
    std::fs::create_dir_all(&base).unwrap();
    for at in 0..=NAMED {
        std::fs::write(base.join(format!("lower-{at:02}")), b"").unwrap();
    }
    // A key holding a rung other than 1s, and nothing else.
    let other = key_dir(root, "NIFTY", LADDER[3]).join("1m");
    std::fs::create_dir_all(&other).unwrap();
    // A key whose 1s rung is empty.
    std::fs::create_dir_all(key_dir(root, "NIFTY", LADDER[4]).join("1s")).unwrap();
    // A key whose 1s rung holds strays, misspelt months and an empty month.
    let rung = key_dir(root, "NIFTY", LADDER[5]).join("1s");
    std::fs::create_dir_all(&rung).unwrap();
    for name in ["notes.txt", "201-06.bin", "2019-6.bin", "2019-13.bin"] {
        std::fs::write(rung.join(name), b"").unwrap();
    }
    std::fs::write(rung.join("2019-06.tix"), b"").unwrap();
    let nifty = sym("NIFTY");
    let at = StorePath::new(month_parts(&nifty, c(LADDER[5]), ym(2019, 7))).unwrap();
    drop(BarFile::open_or_create(root, at, symbol_id(sym("NIFTY"))).unwrap());
    let found = survey(root, None).unwrap();
    assert_eq!(found.strays, u64::try_from(NAMED).unwrap() + 1 + 4);
    assert_eq!(found.strays_named.len(), NAMED);
    assert_eq!(
        found.other_rungs,
        [format!("NSE-FNO-NIFTY-{} 1m", LADDER[3])]
    );
    assert!(found.keys.is_empty(), "no key holds a month with a bar");
    assert_eq!((found.files, found.unreadable), (1, 0));
    let decided = plan(&found, &Withdrawn::default());
    assert_eq!(decided.unresolved.len(), 1);
    assert_eq!(
        decided.unresolved[0].key,
        format!("NSE-FNO-NIFTY-{} 1m", LADDER[3])
    );
    assert_eq!(
        decided.unresolved[0].why,
        "holds a rung other than 1s, which the re-key does not move"
    );
    assert_eq!(decided.keys, 0);
}

/// More month files that will not open than [`NAMED`]: every one counted,
/// the first named, and an apply refused before anything is written.
#[test]
fn every_unreadable_month_is_counted_the_first_named_and_the_apply_refused() {
    let scratch = Scratch::new("ps2-unreadable");
    let root = scratch.root();
    let rung = key_dir(root, "NIFTY", STATED).join("1s");
    std::fs::create_dir_all(&rung).unwrap();
    let count = NAMED + 1;
    for at in 0..count {
        let month = ym(
            2019 + u16::try_from(at / 12).unwrap(),
            u8::try_from(at % 12).unwrap() + 1,
        );
        std::fs::write(rung.join(format!("{month}.bin")), b"not a bar file").unwrap();
    }
    let found = survey(root, None).unwrap();
    assert_eq!(found.unreadable, u64::try_from(count).unwrap());
    assert_eq!(found.unreadable_named.len(), NAMED);
    assert_eq!(found.files, u64::try_from(count).unwrap());
    assert!(found.keys.is_empty());
    partitioned(root, &[]);
    let decided = plan(&found, &Withdrawn::default());
    let why = apply(root, &found, &decided, &Withdrawn::default()).unwrap_err();
    assert!(
        why.contains(&format!("the survey could not read {count} month file(s)")),
        "{why}"
    );
}

/// A store with no GDFL F&O tree surveys empty; one whose tree is a file
/// is refused, naming the directory it could not list.
#[test]
fn an_absent_tree_surveys_empty_and_a_file_in_its_place_is_refused() {
    let scratch = Scratch::new("ps2-tree");
    let root = scratch.root();
    let empty = survey(root, None).unwrap();
    assert!(empty.keys.is_empty() && empty.strays == 0 && empty.files == 0);
    let base = root.join("bars/gdfl/NSE/FNO");
    std::fs::create_dir_all(base.parent().unwrap()).unwrap();
    std::fs::write(&base, b"a file where the tree goes").unwrap();
    let why = survey(root, None).unwrap_err();
    assert!(why.contains("FNO could not be listed"), "{why}");
}

/// A month whose checksums are gone opens, but its days cannot be counted:
/// counted unreadable, named with the store's own words, never surveyed as
/// a key with fewer days.
#[test]
fn a_month_whose_days_cannot_be_counted_is_unreadable_not_a_short_key() {
    let scratch = Scratch::new("ps2-nocrc");
    let root = scratch.root();
    put(
        root,
        "NIFTY",
        STATED,
        &[
            bar(d(2020, 6, 15), 34_000, 333),
            bar(d(2020, 6, 16), 34_000, 334),
        ],
    );
    let bin = month_file(root, STATED, ym(2020, 6));
    std::fs::remove_file(bin.with_extension("crc")).unwrap();
    let found = survey(root, None).unwrap();
    assert_eq!((found.files, found.unreadable), (1, 1));
    assert!(found.keys.is_empty());
    let (path, why) = &found.unreadable_named[0];
    assert_eq!(path, &bin);
    assert!(why.contains("checksum"), "{why}");
}

/// The bars per day of an empty month are none.
#[test]
fn an_empty_month_counts_no_day() {
    let scratch = Scratch::new("ps2-empty-month");
    let root = scratch.root();
    let nifty = sym("NIFTY");
    let at = StorePath::new(month_parts(&nifty, c(STATED), ym(2020, 6))).unwrap();
    let file = BarFile::open_or_create(root, at, symbol_id(sym("NIFTY"))).unwrap();
    assert_eq!(day_counts(&file).unwrap(), Vec::new());
}

// ═══ The plan's verdicts ════════════════════════════════════════════════════

/// Every day of a key traded after the day its expiry settles is refused as
/// unsettled, and the key is retired with that reason; a key with one such
/// day among admitted ones is left, naming the refused day.
#[test]
fn a_key_traded_only_after_its_expiry_is_retired_as_unsettled() {
    let scratch = Scratch::new("ps2-unsettled");
    let root = scratch.root();
    let after = d(2019, 7, 1);
    put(root, "NIFTY", LADDER[3], &[bar(after, 33_300, 101)]);
    put(
        root,
        "NIFTY",
        LADDER[4],
        &[bar(d(2019, 6, 26), 33_300, 102), bar(after, 33_300, 103)],
    );
    let found = survey(root, None).unwrap();
    let decided = plan(&found, &Withdrawn::default());
    assert_eq!(decided.retired.len(), 1, "{:?}", decided.unresolved);
    let retired = decided.retired[0];
    assert_eq!(retired.key, c(LADDER[3]));
    assert_eq!(retired.why, RetiredWhy::ExpiryUnsettled);
    assert_eq!((retired.day, retired.days, retired.bars), (after, 1, 1));
    assert_eq!(retired.median, None);
    assert_eq!(decided.unresolved.len(), 1);
    let left = &decided.unresolved[0];
    assert_eq!(left.key, format!("NSE-FNO-NIFTY-{}", LADDER[4]));
    assert!(
        left.why.contains("the decoder refuses 1 of its 2 day(s)"),
        "{}",
        left.why
    );
    assert!(
        left.why.contains("2019-07-01: expiry unsettled"),
        "{}",
        left.why
    );
}

/// A move the ledger records with the bars the key holds but another month
/// count is not done: left, naming both accounts. So is a retirement the
/// ledger records with the bars but another day count.
#[test]
fn a_recorded_account_with_another_month_or_day_count_is_left_by_name() {
    let fixture = Fixture::new("ps2-accounts");
    let root = fixture.root();
    let moved = SupersededRecord {
        months: 1,
        ..pointer(STATED, SETTLED)
    };
    let retired = RetiredRecord {
        why: RetiredWhy::StrikeImplausible,
        median: Some(1_200_000),
        days: 2,
        ..refusal_of(IMPOSSIBLE[0])
    };
    append_ledger(root, &[moved.line(), retired.line()]);
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    let why_of = |key: &str| {
        decided
            .unresolved
            .iter()
            .find(|one| one.key == format!("NSE-FNO-NIFTY-{key}"))
            .map(|one| one.why.clone())
            .unwrap()
    };
    let moved_why = why_of(STATED);
    assert!(
        moved_why.contains(
            "ledger line 1 records its move to NSE-FNO-NIFTY-2023-06-28-1000000-PE \
             with 3 bar(s) in 1 month(s), and it holds 3 bar(s) in 2 month(s) now"
        ),
        "{moved_why}"
    );
    let retired_why = why_of(IMPOSSIBLE[0]);
    assert!(
        retired_why.contains(
            "ledger line 2 retires it with 1 bar(s) on 2 day(s), and it holds \
             1 bar(s) on 1 day(s) now"
        ),
        "{retired_why}"
    );
    assert!(decided.recorded_moves.is_empty() && decided.recorded_retired.is_empty());
}

/// A key the decoder files where it is, that the ledger supersedes, is left
/// naming the ledger line: the ledger is append-only and is not answered by
/// keeping the key in silence.
#[test]
fn a_kept_key_the_ledger_supersedes_is_left_by_name() {
    let fixture = Fixture::new("ps2-kept-superseded");
    let root = fixture.root();
    append_ledger(root, &[pointer(LADDER[3], "2019-06-26-1190000-PE").line()]);
    let found = survey(root, None).unwrap();
    let decided = plan(&found, &withdrawn(root).unwrap());
    let left = decided
        .unresolved
        .iter()
        .find(|one| one.key == format!("NSE-FNO-NIFTY-{}", LADDER[3]))
        .unwrap();
    assert_eq!(
        left.why,
        "the decoder now files it where it is, and ledger line 1 supersedes it by \
         NSE-FNO-NIFTY-2019-06-26-1190000-PE"
    );
}

// ═══ What the plan leaves, and what apply appends ═══════════════════════════

/// A survey and a census of the store at `root`: the census counts every
/// month every surveyed key holds.
fn counted_now(root: &Path) -> Survey {
    let found = survey(root, None).unwrap();
    let mut rows = Vec::new();
    for key in &found.keys {
        for month in &key.months {
            rows.push(census_row_of(
                root,
                key.underlying.as_str(),
                key.contract.as_str(),
                month.month,
            ));
        }
    }
    partitioned(root, &rows);
    found
}

/// A move whose decoded key the plan itself leaves unresolved (some of its
/// days refused) is left too, naming its target: no bars move into a key
/// left as it is.
#[test]
fn a_move_into_a_key_the_plan_leaves_is_left_naming_its_target() {
    let scratch = Scratch::new("ps2-blocked");
    let root = scratch.root();
    put(root, "NIFTY", STATED, &[bar(d(2020, 6, 15), 34_000, 333)]);
    put(
        root,
        "NIFTY",
        SETTLED,
        &[
            bar(d(2020, 6, 16), 34_000, 444),
            bar(d(2023, 6, 29), 34_000, 445),
        ],
    );
    let decided = plan(&survey(root, None).unwrap(), &Withdrawn::default());
    assert!(decided.moves.is_empty());
    let why_of = |key: &str| {
        decided
            .unresolved
            .iter()
            .find(|one| one.key == format!("NSE-FNO-NIFTY-{key}"))
            .map(|one| one.why.clone())
            .unwrap()
    };
    assert!(why_of(SETTLED).contains("the decoder refuses 1 of its 2 day(s)"));
    assert_eq!(
        why_of(STATED),
        format!("its decoded key NSE-FNO-NIFTY-{SETTLED} is itself refused or unresolved")
    );
}

/// A move whose decoded key the ledger withdraws is left, naming the line:
/// bars moved there would be served by no reader.
#[test]
fn a_move_into_a_key_the_ledger_withdraws_is_left_naming_the_line() {
    let scratch = Scratch::new("ps2-withdrawn-target");
    let root = scratch.root();
    put(root, "NIFTY", STATED, &[bar(d(2020, 6, 15), 34_000, 333)]);
    append_ledger(root, &[refusal_of(SETTLED).line()]);
    let decided = plan(&survey(root, None).unwrap(), &withdrawn(root).unwrap());
    assert!(decided.moves.is_empty());
    assert_eq!(decided.unresolved.len(), 1);
    assert_eq!(decided.unresolved[0].key, format!("NSE-FNO-NIFTY-{STATED}"));
    assert_eq!(
        decided.unresolved[0].why,
        format!(
            "its decoded key NSE-FNO-NIFTY-{SETTLED} is withdrawn by ledger line 1, so bars \
             moved there would be served by no reader"
        )
    );
}

/// A move into a month whose bars all precede the moved ones is appended
/// there, and into an empty month file of the target it is appended too:
/// two month files written, each holding what it held and the move after.
#[test]
fn a_move_after_every_bar_of_its_target_month_is_appended_there() {
    let scratch = Scratch::new("ps2-append-after");
    let root = scratch.root();
    put(
        root,
        "NIFTY",
        STATED,
        &[
            bar(d(2020, 6, 15), 34_000, 333),
            bar(d(2020, 6, 15), 34_001, 334),
            bar(d(2020, 7, 1), 35_000, 335),
        ],
    );
    put(root, "NIFTY", SETTLED, &[bar(d(2020, 6, 1), 34_000, 222)]);
    let nifty = sym("NIFTY");
    let july = StorePath::new(month_parts(&nifty, c(SETTLED), ym(2020, 7))).unwrap();
    drop(BarFile::open_or_create(root, july, symbol_id(nifty)).unwrap());
    let found = counted_now(root);
    let ledger = Withdrawn::default();
    let decided = plan(&found, &ledger);
    assert_eq!(decided.moves.len(), 1, "{:?}", decided.unresolved);
    assert!(decided.unresolved.is_empty());
    let done = apply(root, &found, &decided, &ledger).unwrap();
    assert!(done.refused.is_empty(), "{:?}", done.refused);
    assert_eq!((done.months_written, done.months_already), (2, 0));
    let june = held(root, "NIFTY", SETTLED, ym(2020, 6));
    assert_eq!(june.len(), 3);
    assert_eq!(june[0].close, 222);
    assert_eq!(held(root, "NIFTY", SETTLED, ym(2020, 7)).len(), 1);
    assert_eq!(done.lines_written, 1);
}

/// A target month holding a bar at or after the first moved one: too few
/// bars there to be the move's own copy (the move does not fit), or as
/// many but other bars (not the same), and apply's own check refuses it,
/// which the plan names.
#[test]
fn a_target_month_the_move_would_precede_is_left_by_apply_s_own_check() {
    for (tag, target) in [
        ("ps2-short", vec![bar(d(2020, 6, 20), 34_000, 222)]),
        (
            "ps2-other",
            vec![
                bar(d(2020, 6, 20), 34_000, 222),
                bar(d(2020, 6, 20), 34_001, 223),
            ],
        ),
    ] {
        let scratch = Scratch::new(tag);
        let root = scratch.root();
        put(
            root,
            "NIFTY",
            STATED,
            &[
                bar(d(2020, 6, 15), 34_000, 333),
                bar(d(2020, 6, 15), 34_001, 334),
            ],
        );
        put(root, "NIFTY", SETTLED, &target);
        let decided = plan(&survey(root, None).unwrap(), &Withdrawn::default());
        assert!(decided.moves.is_empty(), "{tag}");
        let left = &decided.unresolved[0];
        assert_eq!(left.key, format!("NSE-FNO-NIFTY-{STATED}"));
        assert!(
            left.why.starts_with(&format!(
                "target month held: NSE-FNO-NIFTY-{SETTLED} 2020-06 already holds {} bar(s)",
                target.len()
            )),
            "{tag}: {}",
            left.why
        );
        assert!(
            left.why.contains("(apply's own check: "),
            "{tag}: {}",
            left.why
        );
        assert!(
            left.why.contains("that these 2 bar(s) from"),
            "{tag}: {}",
            left.why
        );
    }
}

/// Hand-built plans the planner never makes, refused by apply's own read
/// before a byte is written: two sources starting on one second, sources
/// whose seconds interleave, and a plan whose month list is not the store's.
#[test]
fn apply_refuses_hand_built_moves_it_cannot_merge() {
    let a1 = "2023-06-29-1000000-PE";
    let a2 = "2023-06-29-1000000-CE";
    let target = "2023-06-28-1000000-PE";
    let june = ym(2020, 6);
    let cases: [(&str, Vec<Bar>, Vec<Bar>, &str); 2] = [
        (
            "ps2-one-second",
            vec![bar(d(2020, 6, 15), 34_000, 333)],
            vec![bar(d(2020, 6, 15), 34_000, 334)],
            "2020-06: two of the keys moving here start on one second",
        ),
        (
            "ps2-interleaved",
            vec![
                bar(d(2020, 6, 15), 34_000, 333),
                bar(d(2020, 6, 17), 34_000, 335),
            ],
            vec![bar(d(2020, 6, 16), 34_000, 334)],
            "2020-06: the keys moving here hold interleaved or repeated seconds; refused \
             rather than merged",
        ),
    ];
    for (tag, one, two, said) in cases {
        let scratch = Scratch::new(tag);
        let root = scratch.root();
        put(root, "NIFTY", a1, &one);
        put(root, "NIFTY", a2, &two);
        let found = counted_now(root);
        let hand = Plan {
            moves: vec![
                hand_move(a1, target, &[june], u64::try_from(one.len()).unwrap()),
                hand_move(a2, target, &[june], u64::try_from(two.len()).unwrap()),
            ],
            census: CensusStep::Clear,
            ..Plan::default()
        };
        let done = apply(root, &found, &hand, &Withdrawn::default()).unwrap();
        assert_eq!(done.refused.len(), 2, "{tag}");
        for refused in &done.refused {
            assert_eq!(
                refused.why,
                format!("NSE-FNO-NIFTY-{target} {said}"),
                "{tag}"
            );
        }
        assert_eq!((done.months_written, done.lines_written), (0, 0), "{tag}");
        assert!(
            !key_dir(root, "NIFTY", target).exists(),
            "{tag}: nothing written"
        );
    }
    // A plan naming the store's months in another order.
    let scratch = Scratch::new("ps2-order");
    let root = scratch.root();
    put(
        root,
        "NIFTY",
        a1,
        &[
            bar(d(2020, 6, 15), 34_000, 333),
            bar(d(2020, 7, 1), 34_000, 334),
        ],
    );
    let found = counted_now(root);
    let hand = Plan {
        moves: vec![hand_move(a1, target, &[ym(2020, 7), june], 2)],
        census: CensusStep::Clear,
        ..Plan::default()
    };
    let done = apply(root, &found, &hand, &Withdrawn::default()).unwrap();
    assert_eq!(done.refused.len(), 1);
    assert_eq!(
        done.refused[0].why,
        format!(
            "NSE-FNO-NIFTY-{a1}: the plan counted 2 bar(s) in 2 month(s), and it holds 2 \
             bar(s) in 2 month(s) now (2 read): bars landed under it after the survey; re-plan"
        )
    );
}

/// A month of a moving key emptied after the plan (an empty month file in
/// its place) holds no run and no counted month: the source holds fewer
/// bars than the plan counted, so the move is refused, never moved short.
#[test]
fn a_source_month_emptied_after_the_plan_is_refused_not_moved_short() {
    let fixture = Fixture::new("ps2-emptied");
    fixture.counted();
    let root = fixture.root();
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    assert_eq!(decided.moves.len(), 1);
    emptied(root, STATED, ym(2020, 7));
    let done = apply(root, &found, &decided, &ledger).unwrap();
    let refused = done
        .refused
        .iter()
        .find(|one| one.key == format!("NSE-FNO-NIFTY-{STATED}"))
        .unwrap();
    assert_eq!(
        refused.why,
        format!(
            "NSE-FNO-NIFTY-{STATED}: the plan counted 3 bar(s) in 2 month(s), and it holds 2 \
             bar(s) in 1 month(s) now (2 read): bars landed under it after the survey; re-plan"
        )
    );
    assert!(done.moved.is_empty());
    assert!(!key_dir(root, "NIFTY", SETTLED).exists(), "nothing moved");
}

/// A planned retirement whose month will not open when apply re-counts it
/// is refused by name, and the others are retired.
#[test]
fn a_retirement_whose_month_will_not_open_at_apply_is_refused_not_retired() {
    let scratch = retirements_only("ps2-retire-unreadable");
    let root = scratch.root();
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    assert_eq!(decided.retired.len(), 3);
    std::fs::write(
        month_file(root, IMPOSSIBLE[0], ym(2019, 5)),
        b"no longer a bar file",
    )
    .unwrap();
    let done = apply(root, &found, &decided, &ledger).unwrap();
    assert_eq!(done.retired.len(), 2);
    assert_eq!(done.refused.len(), 1);
    assert_eq!(
        done.refused[0].key,
        format!("NSE-FNO-NIFTY-{}", IMPOSSIBLE[0])
    );
    assert!(
        done.refused[0].why.ends_with(
            ": its bars could not be re-counted under the lock, so it is not retired; re-plan"
        ),
        "{}",
        done.refused[0].why
    );
    assert_eq!(withdrawn(root).unwrap().counts(), (0, 2));
}

/// The census lock held by another ingest when the moved months are to be
/// counted: refused naming the months already written and that no ledger
/// line was written, so the next run finishes it.
#[test]
fn a_census_that_refuses_after_the_moves_names_the_months_written() {
    let fixture = Fixture::new("ps2-census-refuses");
    fixture.counted();
    let root = fixture.root();
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    let lock_path = census_store::target_for(root, Vendor::Gdfl, Segment::Fno)
        .unwrap()
        .lock_path();
    let holder = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .unwrap();
    holder.lock().unwrap();
    let why = apply(root, &found, &decided, &ledger).unwrap_err();
    assert!(
        why.contains("another ingest holds the census lock"),
        "{why}"
    );
    assert!(
        why.ends_with(
            "(2 month file(s) were written under decoded keys and are not yet counted; no \
             ledger line was written. Run again to finish)"
        ),
        "{why}"
    );
    assert_eq!(withdrawn(root).unwrap().counts(), (0, 0));
    holder.unlock().unwrap();
}

/// The re-key's lock path held by something other than a regular file
/// refuses the apply before a byte.
#[test]
fn a_lock_path_that_is_not_a_file_refuses_the_apply() {
    let fixture = Fixture::new("ps2-lock-dir");
    fixture.counted();
    let root = fixture.root();
    std::fs::create_dir_all(ledger_lock_path(root)).unwrap();
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let why = apply(root, &found, &plan(&found, &ledger), &ledger).unwrap_err();
    assert_eq!(
        why,
        format!(
            "{} is not a regular file; nothing was written",
            ledger_lock_path(root).display()
        )
    );
    assert!(!key_dir(root, "NIFTY", SETTLED).exists());
}

/// The run of one apply on the fixture, after `before` arranged the store
/// (as the uid the permission bits bind), and `after` given its answer.
fn apply_where_permission_binds(tag: &str, before: impl Fn(&Path), after: impl Fn(&Path, String)) {
    let fixture = Fixture::new(tag);
    fixture.counted();
    let root = fixture.root();
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let decided = plan(&found, &ledger);
    before(root);
    let why = apply(root, &found, &decided, &ledger).unwrap_err();
    after(root, why);
}

/// A ledger the host will not let the apply append to: the moved months are
/// written and counted and the refusal says so, so the next run finishes.
#[test]
fn a_ledger_the_host_will_not_append_to_says_what_is_done() {
    crate::support::where_permission_binds(
        "gdfl_rekey::tests::ps2::a_ledger_the_host_will_not_append_to_says_what_is_done",
        || {
            apply_where_permission_binds(
                "ps2-ledger-ro",
                |root| {
                    let path = lineage_path(root);
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                    std::fs::write(&path, b"").unwrap();
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444))
                        .unwrap();
                },
                |root, why| {
                    assert!(why.contains("Permission denied"), "{why}");
                    assert!(
                        why.ends_with(
                            "(the moved months are written and counted; run again to finish)"
                        ),
                        "{why}"
                    );
                    assert_eq!(held(root, "NIFTY", SETTLED, ym(2020, 6)).len(), 2);
                    assert_eq!(std::fs::read(lineage_path(root)).unwrap(), b"");
                },
            );
        },
    );
}

/// A re-key lock the host will not measure, and one it will not open, each
/// refuse the apply by name before a byte is written.
#[test]
fn a_lock_the_host_will_not_measure_or_open_refuses_the_apply() {
    crate::support::where_permission_binds(
        "gdfl_rekey::tests::ps2::a_lock_the_host_will_not_measure_or_open_refuses_the_apply",
        || {
            apply_where_permission_binds(
                "ps2-lock-unsearchable",
                |root| {
                    let imports = ledger_lock_path(root).parent().unwrap().to_path_buf();
                    std::fs::create_dir_all(&imports).unwrap();
                    std::fs::set_permissions(&imports, std::fs::Permissions::from_mode(0o600))
                        .unwrap();
                },
                |root, why| {
                    let imports = ledger_lock_path(root).parent().unwrap().to_path_buf();
                    std::fs::set_permissions(&imports, std::fs::Permissions::from_mode(0o755))
                        .unwrap();
                    assert!(
                        why.starts_with(&format!("{}: ", ledger_lock_path(root).display())),
                        "{why}"
                    );
                    assert!(why.contains("Permission denied"), "{why}");
                    assert!(why.ends_with("; nothing was written"), "{why}");
                    assert!(!key_dir(root, "NIFTY", SETTLED).exists());
                },
            );
            apply_where_permission_binds(
                "ps2-lock-readonly",
                |root| {
                    let lock = ledger_lock_path(root);
                    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
                    std::fs::write(&lock, b"").unwrap();
                    std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o444))
                        .unwrap();
                },
                |root, why| {
                    assert!(
                        why.starts_with(&format!(
                            "{} could not be opened: ",
                            ledger_lock_path(root).display()
                        )),
                        "{why}"
                    );
                    assert!(why.contains("Permission denied"), "{why}");
                    assert!(!key_dir(root, "NIFTY", SETTLED).exists());
                },
            );
        },
    );
}

/// A census row of another exchange in the F&O partition is never withdrawn
/// with an NSE key of the same symbol and contract.
#[test]
fn a_census_row_of_another_exchange_is_never_withdrawn() {
    let fixture = Fixture::new("ps2-bse-row");
    let root = fixture.root();
    let mut rows = Vec::new();
    for month in [ym(2020, 6), ym(2020, 7)] {
        rows.push(census_row_of(root, "NIFTY", STATED, month));
        rows.push(census_row_of(root, "NIFTY", "2020-07-30-1000000-CE", month));
    }
    for key in LADDER.iter().chain(&IMPOSSIBLE) {
        rows.push(census_row_of(root, "NIFTY", key, ym(2019, 5)));
    }
    let mut bse = census_row_of(root, "NIFTY", STATED, ym(2020, 6));
    bse.entry.key.exchange = Exchange::Bse;
    rows.push(bse);
    partitioned(root, &rows);
    let found = survey(root, None).unwrap();
    let ledger = withdrawn(root).unwrap();
    let done = apply(root, &found, &plan(&found, &ledger), &ledger).unwrap();
    assert!(done.withdrawal.is_some());
    let (_, keys) = census_keys(root);
    assert!(
        keys.contains(&format!("NIFTY {STATED} 2020-06")),
        "the BSE row stays: {keys:?}"
    );
    assert!(
        !keys.contains(&format!("NIFTY {STATED} 2020-07")),
        "{keys:?}"
    );
}

/// The (U) items of the verdict arms rest on two facts of the settlement
/// tables, held here so a table change that breaks them fails a test: every
/// alias row settles on another day than the one it states (so an alias
/// kind always moves the expiry), and the one early settlement's expiries
/// settle the same way before and after its ex-date (so one key's days
/// never settle two ways).
#[test]
fn the_settlement_facts_the_unreachable_verdict_arms_rest_on_hold() {
    use crate::expiry_alias::{ALIASES, EARLY_SETTLEMENTS};
    for row in ALIASES {
        assert_ne!(row.settled, row.stated, "{row:?}");
    }
    for row in EARLY_SETTLEMENTS {
        let ex = Day::new(row.ex_date.0, row.ex_date.1, row.ex_date.2).unwrap();
        let before = Day::from_days(ex.days_from_epoch() - 1).unwrap();
        for &(y, m, dd) in row.expiries {
            let stated = Day::new(y, m, dd).unwrap();
            let early = settle(row.underlying, stated, Some(before)).unwrap();
            let late = settle(row.underlying, stated, Some(ex)).unwrap();
            assert_eq!(early.expiry, late.expiry, "{stated}");
            assert_eq!(early.settlement.kind(), late.settlement.kind(), "{stated}");
        }
    }
}

/// A target month that no longer opens when apply reads it (spoilt after
/// the survey) refuses its move by name, naming the file; nothing is
/// written there and the move is not recorded.
#[test]
fn a_target_month_spoilt_after_the_survey_refuses_its_move() {
    let scratch = Scratch::new("ps2-target-spoilt");
    let root = scratch.root();
    put(root, "NIFTY", STATED, &[bar(d(2020, 6, 15), 34_000, 333)]);
    put(root, "NIFTY", SETTLED, &[bar(d(2020, 6, 1), 34_000, 222)]);
    let found = counted_now(root);
    let ledger = Withdrawn::default();
    let decided = plan(&found, &ledger);
    assert_eq!(decided.moves.len(), 1, "{:?}", decided.unresolved);
    let target = month_file(root, SETTLED, ym(2020, 6));
    std::fs::write(&target, b"no longer a bar file").unwrap();
    let done = apply(root, &found, &decided, &ledger).unwrap();
    assert_eq!(done.refused.len(), 1);
    assert!(
        done.refused[0]
            .why
            .starts_with(&format!("{}: ", target.display())),
        "{}",
        done.refused[0].why
    );
    assert!(done.moved.is_empty() && done.lines_written == 0);
    assert_eq!(std::fs::read(&target).unwrap(), b"no longer a bar file");
}

/// No bars to place is nothing to write: `Already`, whether the target
/// month holds bars or none; with bars and no target month, `Append`.
#[test]
fn no_bars_to_place_are_already_placed() {
    let scratch = Scratch::new("ps2-no-bars");
    let root = scratch.root();
    let nifty = sym("NIFTY");
    let june = ym(2020, 6);
    assert!(matches!(
        target_state(root, nifty, c(SETTLED), june, &[]),
        Ok(Write::Append)
    ));
    put(root, "NIFTY", SETTLED, &[bar(d(2020, 6, 1), 34_000, 222)]);
    assert!(matches!(
        target_state(root, nifty, c(SETTLED), june, &[]),
        Ok(Write::Already)
    ));
    let later = [bar(d(2020, 6, 2), 34_000, 223)];
    assert!(matches!(
        target_state(root, nifty, c(SETTLED), june, &later),
        Ok(Write::Append)
    ));
    let same = held(root, "NIFTY", SETTLED, june);
    assert!(matches!(
        target_state(root, nifty, c(SETTLED), june, &same),
        Ok(Write::Already)
    ));
}

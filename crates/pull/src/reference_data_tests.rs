//! Tests of the reference tables (R3-30): every table through the ONE
//! loader, refused by name when its bytes are not the bytes written, and a
//! row added to a store changing the next answer with no code change.
//!
//! EVERY ROW BELOW THAT IS NOT A COMPILED SEED IS INVENTED: the underlyings
//! (`INVENTEDCO`), the index (`NIFTY INVENTED`), the yields and the RBI page
//! numbers. No vendor row is copied.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

use crate::gdfl_fixtures::scratch;
use crate::pricing::TradeDay;

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).expect("a real day")
}

fn trade(y: u16, m: u8, d: u8) -> TradeDay {
    TradeDay::new(y, m, d).expect("a real day")
}

fn epoch(y: u16, m: u8, d: u8) -> i64 {
    i64::from(day(y, m, d).days_from_epoch())
}

fn seeded(tag: &str) -> PathBuf {
    let store = scratch(tag);
    let landed = seed(&store).expect("every compiled table seeds");
    assert_eq!(
        landed.iter().map(|w| w.table).collect::<Vec<_>>(),
        Table::ALL
            .into_iter()
            .filter(|t| t.compiled())
            .collect::<Vec<_>>(),
        "generation 1 of every compiled table, and no rate table"
    );
    store
}

const RBI: &str = "https://www.rbi.org.in/Scripts/BS_PressReleaseDisplay.aspx?prid=1";

/// An invented series: two results, a documented cancellation, a result.
fn invented_rates(with_gap: bool) -> String {
    let mut lines = vec![
        crate::riskfree::HEADER.to_owned(),
        format!("\t\t2020-01-01\t98.5000\t6.1081\t{RBI}\t\t"),
        format!("\t\t2020-01-08\t98.4000\t6.5219\t{RBI}\t\t"),
    ];
    if with_gap {
        lines.push(format!("\t\t2020-01-15\tCANCELLED\t\t{RBI}\t\t"));
    }
    lines.push(format!("\t\t2020-01-29\t98.6000\t5.6951\t{RBI}\t\t"));
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

#[test]
fn a_row_added_to_the_index_table_is_imported_without_a_code_change() {
    let store = seeded("ref-index-row");
    let before = load(&store);
    assert_eq!(before.indices.origin(), "store");
    assert!(
        before
            .indices
            .table()
            .unwrap()
            .by_stem("NIFTY INVENTED")
            .is_none()
    );
    let compiled_scope = crate::gdfl_cm::CmScope::stored(std::iter::empty::<&str>()).unwrap();
    assert!(
        !compiled_scope
            .indices()
            .iter()
            .any(|row| row.index_symbol == "NIFTY INVENTED")
    );

    let mut draft = seed_draft(Table::IndexIdentity).unwrap();
    let mut row = draft.rows[0].clone();
    row[0] = "-".to_owned();
    row[1] = "NIFTY INVENTED".to_owned();
    row[2] = "NIFTY INVENTED".to_owned();
    row[3] = "-".to_owned();
    row[4] = "-".to_owned();
    row[5] = "NIFTYINVENTED".to_owned();
    row[6] = "yes".to_owned();
    row[9] = "2026-01-01".to_owned();
    row[13] = "an invented NSE index list, for this test only".to_owned();
    draft.rows.push(row);
    let written = write(&store, &draft).expect("generation 2 lands");
    assert_eq!(written.generation, 2);

    let after = load(&store);
    let table = after.indices.table().unwrap();
    let added = table
        .by_stem("NIFTY INVENTED")
        .expect("the new row is served");
    assert_eq!(added.key_gdfl.as_deref(), Some("NIFTYINVENTED"));
    assert_eq!(added.gdfl_span.0, Some(day(2026, 1, 1)));
    assert_eq!(after.indices.receipt().unwrap().generation, 2);
    // THE NEXT IMPORT'S SCOPE takes it, through the same loader.
    let scope = crate::gdfl_cm::CmScope::stored_from(&after, std::iter::empty::<&str>()).unwrap();
    assert!(
        scope
            .indices()
            .iter()
            .any(|row| row.index_symbol == "NIFTY INVENTED"),
        "a row added to the store is in the next import's scope"
    );
    // Generation 1 is still on disk, byte for byte (append-only).
    assert!(Table::IndexIdentity.path(&store, 1).is_file());
}

#[test]
fn a_table_with_a_wrong_checksum_is_refused_by_name() {
    let store = seeded("ref-checksum");
    let path = Table::IndexIdentity.path(&store, 1);
    let good = std::fs::read_to_string(&path).unwrap();
    // One byte of one row changed: the checksum line no longer matches.
    let flipped = good.replacen("NIFTY 50", "NIFTY 51", 1);
    assert_ne!(flipped, good);
    std::fs::write(&path, &flipped).unwrap();
    let loaded = load(&store);
    let why = loaded.indices.table().expect_err("refused");
    assert_eq!(why.kind, RefusalKind::Checksum);
    assert_eq!(why.table, Table::IndexIdentity);
    assert_eq!(why.generation, Some(1));
    assert_eq!(
        loaded.indices.origin(),
        "refused",
        "never the compiled copy"
    );
    let said = why.to_string();
    assert!(
        said.contains("index_identity") && said.contains("checksum"),
        "{said}"
    );
    assert_eq!(
        loaded.refusals().len(),
        2,
        "the index table and the absent rate table"
    );
    // A consumer that needs it refuses by that name too.
    let scope = crate::gdfl_cm::CmScope::stored_from(&loaded, std::iter::empty::<&str>());
    assert!(scope.expect_err("refused").contains("index_identity"));

    // TORN: the file cut mid-row.
    std::fs::write(&path, &good.as_bytes()[..good.len() / 2]).unwrap();
    assert_eq!(
        load(&store).indices.table().unwrap_err().kind,
        RefusalKind::Checksum
    );
    // ZERO-LENGTH.
    std::fs::write(&path, b"").unwrap();
    assert_eq!(
        load(&store).indices.table().unwrap_err().kind,
        RefusalKind::Checksum
    );
    // NOT TEXT.
    std::fs::write(&path, [0xff_u8, 0xfe, b'\n']).unwrap();
    assert_eq!(
        load(&store).indices.table().unwrap_err().kind,
        RefusalKind::NotText
    );
    // A NEWER broken generation never falls back to the older good one.
    std::fs::write(&path, &good).unwrap();
    assert_eq!(load(&store).indices.origin(), "store");
    std::fs::write(Table::IndexIdentity.path(&store, 2), b"garbage\n").unwrap();
    let newer = load(&store);
    assert_eq!(newer.indices.table().unwrap_err().generation, Some(2));
    // A file of another table filed in this table's directory.
    let lots = std::fs::read(Table::LotSizes.path(&store, 1)).unwrap();
    std::fs::write(Table::IndexIdentity.path(&store, 3), lots).unwrap();
    assert_eq!(
        load(&store).indices.table().unwrap_err().kind,
        RefusalKind::WrongTable
    );
}

#[test]
fn stray_names_are_not_generations_and_an_empty_store_serves_the_compiled_tables_labelled() {
    let store = scratch("ref-strays");
    let loaded = load(&store);
    for (table, origin, path, receipt, refused) in loaded.slots() {
        if table.compiled() {
            assert_eq!(origin, "compiled", "{table}");
            assert_eq!(path, table.dir(&store).as_path());
            assert!(receipt.is_some() && refused.is_none());
        } else {
            assert_eq!(origin, "refused", "{table}");
            assert_eq!(refused.unwrap().kind, RefusalKind::Absent);
        }
    }
    let dir = Table::LotSizes.dir(&store);
    std::fs::create_dir_all(&dir).unwrap();
    for stray in [
        "notes.txt",
        "1.ref",
        "0000000000000000000x.ref",
        ".00000000000000000001.9.staging",
    ] {
        std::fs::write(dir.join(stray), b"x").unwrap();
    }
    let strays = Strays {
        staging: 1,
        other: 3,
        first_other: Some("0000000000000000000x.ref".to_owned()),
    };
    assert_eq!(
        newest(&store, Table::LotSizes).unwrap(),
        (None, strays.clone())
    );
    // Served from the compiled copy, and the strays are KEPT and told, never
    // ignored in silence (R3-30 repair 1).
    let loaded = load(&store);
    assert_eq!(loaded.lots.origin(), "compiled");
    assert_eq!(loaded.strays, [(Table::LotSizes, strays)]);
    assert!(load(&scratch("ref-no-strays")).strays.is_empty());
}

#[test]
fn the_seeded_lot_table_carries_the_compiled_rows_unchanged() {
    let store = seeded("ref-lots-seed");
    let loaded = load(&store);
    let table = loaded.lots.table().unwrap();
    assert_eq!(table.rows().len(), costs::lot::compiled_rows().len());
    let mut compiled = costs::lot::compiled_rows();
    compiled.sort_by(|a, b| a.underlying.cmp(&b.underlying));
    assert_eq!(
        table.rows(),
        compiled.as_slice(),
        "day, units, citation and standing"
    );
    assert_eq!(table.underlyings(), 2, "NIFTY and BANKNIFTY alone");
    let receipt = loaded.lots.receipt().unwrap();
    assert_eq!(receipt.decision, "D-3357");
    assert_eq!(receipt.rows, compiled.len());
    assert!(is_hex64(&receipt.sha256));
}

#[test]
fn an_underlying_without_a_sourced_lot_is_named_absent() {
    let store = seeded("ref-lots-absent");
    let loaded = load(&store);
    let lots = loaded.lots.table().unwrap();
    // NIFTY, from the sourced 2024-11-20 row (NSE/FAOP/64625).
    let answer = lots.lot_on("NIFTY", trade(2025, 1, 6)).expect("sourced");
    assert_eq!(answer.lot.units(), 75);
    assert!(answer.row.source.contains("FAOP64625"));
    // BANKNIFTY, the 2025-12-30 row (NSE/FAOP/70616).
    assert_eq!(
        lots.lot_on("BANKNIFTY", trade(2026, 2, 2))
            .unwrap()
            .lot
            .units(),
        30
    );
    // Before every row: the compiled LOT_GAP refusal, unchanged.
    let before = lots
        .lot_on("NIFTY", trade(2020, 9, 1))
        .expect_err("before 2021");
    assert_eq!(before.word(), "before_first_row");
    assert!(before.to_string().contains("UNVERIFIED"), "{before}");
    // A row with no circular in the charter is listed and NOT applied.
    let unverified = lots
        .lot_on("NIFTY", trade(2022, 3, 1))
        .expect_err("TRACK2 row");
    assert_eq!(unverified.word(), "unverified");
    // An underlying no row names: a named absence, never an inferred lot.
    let none = lots
        .lot_on("RELIANCE", trade(2025, 1, 6))
        .expect_err("no row");
    assert_eq!(none.word(), "no_row");
    assert!(none.to_string().contains("no lot is inferred"));
    // A garbage underlying is the same named absence.
    assert_eq!(
        lots.lot_on("", trade(2025, 1, 6)).unwrap_err().word(),
        "no_row"
    );

    // A FILE whose receipt is not sourced applies no row at all.
    let mut draft = seed_draft(Table::LotSizes).unwrap();
    draft.standing = Standing::Measured;
    write(&store, &draft).unwrap();
    let measured = load(&store);
    assert_eq!(
        measured
            .lots
            .table()
            .unwrap()
            .lot_on("NIFTY", trade(2025, 1, 6))
            .unwrap_err()
            .word(),
        "unverified"
    );
}

#[test]
fn a_lot_row_added_to_the_store_table_changes_the_next_lookup_without_a_code_change() {
    let store = seeded("ref-lots-added");
    assert_eq!(
        load(&store)
            .lots
            .table()
            .unwrap()
            .lot_on("INVENTEDCO", trade(2025, 3, 3))
            .unwrap_err()
            .word(),
        "no_row"
    );
    let mut draft = seed_draft(Table::LotSizes).unwrap();
    let sha = "ab".repeat(32);
    draft.rows.push(vec![
        "INVENTEDCO".into(),
        "2025-01-01".into(),
        "1234".into(),
        "sourced".into(),
        sha.clone(),
        "2026-10-20".into(),
        "an invented circular, for this test only".into(),
    ]);
    draft.rows.push(vec![
        "INVENTEDCO".into(),
        "2025-06-02".into(),
        "4321".into(),
        "sourced".into(),
        sha,
        "2026-10-20".into(),
        "an invented revision, for this test only".into(),
    ]);
    write(&store, &draft).unwrap();
    let lots = load(&store);
    let lots = lots.lots.table().unwrap();
    assert_eq!(
        lots.lot_on("INVENTEDCO", trade(2025, 3, 3))
            .unwrap()
            .lot
            .units(),
        1234
    );
    // The revision's own day is the new lot; the day before is the old one.
    assert_eq!(
        lots.lot_on("INVENTEDCO", trade(2025, 6, 1))
            .unwrap()
            .lot
            .units(),
        1234
    );
    assert_eq!(
        lots.lot_on("INVENTEDCO", trade(2025, 6, 2))
            .unwrap()
            .lot
            .units(),
        4321
    );
    assert_eq!(
        lots.lot_on("INVENTEDCO", trade(2024, 12, 31))
            .unwrap_err()
            .word(),
        "before_first_row"
    );
    // NIFTY is untouched by the new rows.
    assert_eq!(
        lots.lot_on("NIFTY", trade(2025, 1, 6)).unwrap().lot.units(),
        75
    );
}

#[test]
fn a_malformed_lot_table_is_refused_by_name_and_never_written() {
    let store = seeded("ref-lots-bad");
    let base = seed_draft(Table::LotSizes).unwrap();
    let row = |underlying: &str, effective: &str, units: &str, standing: &str, sha: &str| {
        vec![
            underlying.to_owned(),
            effective.to_owned(),
            units.to_owned(),
            standing.to_owned(),
            sha.to_owned(),
            "2026-10-20".to_owned(),
            "an invented circular".to_owned(),
        ]
    };
    let sha = "cd".repeat(32);
    for (bad, kind) in [
        (
            row("INVENTEDCO", "2025-01-01", "0", "sourced", &sha),
            RefusalKind::Row,
        ),
        (
            row("INVENTEDCO", "2025-01-01", "-5", "sourced", &sha),
            RefusalKind::Row,
        ),
        (
            row("INVENTEDCO", "2025-02-30", "10", "sourced", &sha),
            RefusalKind::Row,
        ),
        (
            row("INVENTEDCO", "2025-01-01", "10", "sourced", "-"),
            RefusalKind::Row,
        ),
        (
            row("INVENTEDCO", "2025-01-01", "10", "guessed", &sha),
            RefusalKind::Row,
        ),
        // A duplicate day for NIFTY: two rows cannot both govern a day.
        (
            row("NIFTY", "2024-11-20", "75", "sourced", &sha),
            RefusalKind::Row,
        ),
    ] {
        let mut draft = base.clone();
        draft.rows.push(bad);
        match write(&store, &draft) {
            Err(WriteError::Refused(why)) => {
                assert_eq!(why.kind, kind, "{why}");
                assert_eq!(why.table, Table::LotSizes);
            }
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(
        newest(&store, Table::LotSizes).unwrap().0.map(|(g, _)| g),
        Some(1),
        "nothing refused reached a final name"
    );
}

#[test]
fn a_documented_rate_cancellation_is_carried_and_labelled() {
    let store = seeded("ref-rates");
    let tsv = invented_rates(true);
    let draft = rate_draft(
        &tsv,
        "operator's RBI 91-day T-bill table, invented here",
        day(2026, 10, 20),
    )
    .unwrap();
    assert_eq!(
        draft.source_sha256.as_deref(),
        Some(sha256_hex(tsv.as_bytes()).as_str())
    );
    write(&store, &draft).unwrap();
    let loaded = load(&store);
    assert_eq!(loaded.rates.origin(), "store");
    let series = loaded.rates.table().unwrap();
    assert_eq!((series.auction_count(), series.gap_count()), (3, 1));
    // Day 15 after the 8 Jan result: carried over the documented 15 Jan gap.
    let carried = series.on(trade(2020, 1, 23)).unwrap();
    assert_eq!(carried.state(), "rate_carried");
    assert_eq!(
        carried.carried.map(|gap| gap.kind),
        Some(crate::riskfree::GapKind::Cancelled)
    );
    assert_eq!(carried.auction.released, trade(2020, 1, 8));
    // The same table without the gap row refuses that day.
    let bare = rate_draft(&invented_rates(false), "invented", day(2026, 10, 20)).unwrap();
    write(&store, &bare).unwrap();
    let loaded = load(&store);
    assert!(matches!(
        loaded.rates.table().unwrap().on(trade(2020, 1, 23)),
        Err(crate::riskfree::LookupError::Stale { .. })
    ));
    // A rate row the series refuses is refused at the write, by name.
    let mut broken = draft.clone();
    broken.rows[1][3] = "99.9999".to_owned();
    match write(&store, &broken) {
        Err(WriteError::Refused(why)) => {
            assert_eq!(why.table, Table::RiskFreeRate);
            assert!(why.detail.contains("YieldDisagreesWithPrice"), "{why}");
        }
        other => panic!("{other:?}"),
    }
    // A TSV that is not the series is not a draft.
    assert!(rate_draft("not a header\n", "x", day(2026, 10, 20)).is_err());
}

#[test]
fn a_store_with_no_rate_table_refuses_greeks_by_name() {
    let store = seeded("ref-rates-absent");
    let loaded = load(&store);
    let why = loaded.rates.table().expect_err("absent");
    assert_eq!(why.kind, RefusalKind::Absent);
    assert_eq!(why.kind.word(), "absent");
    assert!(why.to_string().contains("risk_free_rate"), "{why}");
    assert!(why.detail.contains("no rate is invented"));
    // An absent rate table is an ALERT with its reason, never a quiet None.
    let [(cal_table, cal), (rate_table, rates)] = loaded.horizons(epoch(2026, 10, 20));
    assert_eq!(
        (cal_table, rate_table),
        (Table::Calendar, Table::RiskFreeRate)
    );
    assert!(cal.is_ok(), "the calendar is served");
    assert_eq!(rates.map_err(|why| why.kind), Err(RefusalKind::Absent));
    let alerts = loaded.horizon_alerts(epoch(2026, 10, 20));
    assert_eq!(alerts.len(), 1, "{alerts:?}");
    assert_eq!(alerts[0].0, Table::RiskFreeRate);
    assert!(alerts[0].1.starts_with("not served: "), "{alerts:?}");
    assert!(alerts[0].1.contains("absent"), "{alerts:?}");
}

#[test]
fn the_horizon_alert_fires_seven_days_before_the_calendar_ends() {
    let store = seeded("ref-horizon");
    let loaded = load(&store);
    let calendar = loaded.calendar.table().unwrap();
    let last = calendar.last_day();
    assert_eq!(last, crate::calendar::SOURCED_LAST_DAY);
    assert!(!calendar.horizon(last - 8).alert(), "eight days out: quiet");
    assert!(
        calendar.horizon(last - 7).alert(),
        "seven days out: the alert fires"
    );
    assert!(calendar.horizon(last).alert() && !calendar.horizon(last).ended);
    assert!(calendar.horizon(last + 1).ended);
    // The rate series' horizon: last release plus MAX_AGE_DAYS.
    write(
        &store,
        &rate_draft(&invented_rates(true), "invented", day(2026, 10, 20)).unwrap(),
    )
    .unwrap();
    let loaded = load(&store);
    let rates_last = epoch(2020, 1, 29) + i64::from(crate::riskfree::MAX_AGE_DAYS);
    let [(_, cal), (_, rates)] = loaded.horizons(rates_last - 7);
    assert!(cal.is_ok());
    let rates = rates.unwrap();
    assert_eq!(rates.last_day, rates_last);
    assert!(rates.alert());
    assert!(!loaded.horizons(rates_last - 8)[1].1.unwrap().alert());
    assert_eq!(loaded.horizon_alerts(rates_last - 8), []);
    assert_eq!(
        loaded.horizon_alerts(rates_last + 1),
        [(Table::RiskFreeRate, "ended 1 days ago".to_owned())]
    );
}

#[test]
fn no_reader_answers_unmeasured_for_a_day_the_calendar_knows() {
    let loaded = load(&scratch("ref-calendar-span"));
    let calendar = loaded.calendar.table().unwrap();
    assert!(
        calendar.last_day() >= epoch(2026, 9, 30),
        "SC-CLOSE-15: to 2026-09-30 at least"
    );
    assert_eq!(calendar.first_day(), crate::calendar::SOURCED_FIRST_DAY);
    for day in calendar.first_day()..=calendar.last_day() {
        let kind = calendar.kind_of(day);
        assert_ne!(kind, DayKind::Unmeasured, "day {day}");
        assert_eq!(kind, crate::calendar::exchange_kind_of(day), "day {day}");
    }
    assert_eq!(
        calendar.kind_of(calendar.first_day() - 1),
        DayKind::Unmeasured
    );
    assert_eq!(
        calendar.kind_of(calendar.last_day() + 1),
        DayKind::Unmeasured
    );
    assert_eq!(calendar.kind_of(i64::MIN), DayKind::Unmeasured);
    assert_eq!(calendar.kind_of(i64::MAX), DayKind::Unmeasured);
}

#[test]
fn muhurat_2026_unmeasured_is_one_action_item() {
    let store = seeded("ref-muhurat");
    let muhurat = epoch(2026, 11, 8);
    for _run in 0..3 {
        let loaded = load(&store);
        let items = loaded
            .calendar
            .table()
            .unwrap()
            .unsized_from(epoch(2026, 10, 20));
        let mine: Vec<_> = items.iter().filter(|(d, _, _)| *d == muhurat).collect();
        assert_eq!(
            mine.len(),
            1,
            "one item for the day, however many runs meet it"
        );
        assert!(
            mine[0].2.contains("71775"),
            "it names its circular: {:?}",
            mine[0]
        );
        assert_eq!(
            loaded.calendar.table().unwrap().kind_of(muhurat),
            DayKind::OpenLengthUnmeasured
        );
    }
    // Once a window is recorded the day is an ordinary session of that length.
    let mut draft = seed_draft(Table::Calendar).unwrap();
    let at = draft
        .rows
        .iter()
        .position(|row| row[1] == "2026-11-08")
        .expect("the Muhurat row");
    draft.rows[at][2] = "18:00-18:59".to_owned();
    write(&store, &draft).unwrap();
    let loaded = load(&store);
    let calendar = loaded.calendar.table().unwrap();
    assert!(matches!(calendar.kind_of(muhurat), DayKind::Open(_)));
    assert!(
        calendar
            .unsized_from(epoch(2026, 10, 20))
            .iter()
            .all(|(d, _, _)| *d != muhurat)
    );
}

/// BASE-GATE11 (gate 11 rule 4, D-6629's addendum): the sessions of
/// unrecorded length are read off the calendar's own day kinds rather than
/// sorted, so rows handed in out of day order still list ascending, and the
/// list names exactly the days `kind_of` calls unmeasured, each with its row.
#[test]
fn the_unsized_days_are_the_calendars_own_in_day_order() {
    // The Muhurat test's tag: `scratch` makes each directory unique by
    // process, serial and clock, and a new tag would be one more
    // segment-shaped literal for gate 1d to declare.
    let store = seeded("ref-muhurat");
    let before = load(&store);
    let calendar = before.calendar.table().unwrap();
    // Two weekend days with no row of their own, found, never typed.
    let quiet: Vec<i64> = (epoch(2026, 3, 1)..epoch(2026, 6, 1))
        .filter(|d| calendar.kind_of(*d) == DayKind::Closed && calendar.note(*d).is_none())
        .take(2)
        .collect();
    assert_eq!(quiet.len(), 2, "two quiet weekend days: {quiet:?}");
    let mut draft = seed_draft(Table::Calendar).unwrap();
    let template = draft
        .rows
        .iter()
        .find(|row| row[1] == "2026-11-08")
        .cloned()
        .expect("the Muhurat row, a session of unrecorded length");
    assert_eq!(template[2], "unmeasured", "{template:?}");
    // The later day first, both after every other row: out of day order.
    for (at, d) in quiet.iter().enumerate().rev() {
        let mut row = template.clone();
        row[1] = day_of(*d).unwrap().to_string();
        row[3] = format!("invented unmeasured session {at}");
        draft.rows.push(row);
    }
    write(&store, &draft).unwrap();
    let after = load(&store);
    let calendar = after.calendar.table().unwrap();
    let listed = calendar.unsized_from(calendar.first_day());
    let days: Vec<i64> = listed.iter().map(|(d, _, _)| *d).collect();
    let own: Vec<i64> = (calendar.first_day()..=calendar.last_day())
        .filter(|d| calendar.kind_of(*d) == DayKind::OpenLengthUnmeasured)
        .collect();
    assert_eq!(days, own, "the list is the calendar's own unmeasured days");
    assert!(days.windows(2).all(|w| w[0] < w[1]), "ascending: {days:?}");
    assert!(days.contains(&epoch(2026, 11, 8)), "{days:?}");
    for (at, d) in quiet.iter().enumerate() {
        let item = listed.iter().find(|(day, _, _)| day == d);
        assert_eq!(
            item.map(|(_, name, _)| *name),
            Some(format!("invented unmeasured session {at}").as_str()),
            "day {d} keeps its own row's name"
        );
    }
    // From a day on, only the days from it.
    let later = calendar.unsized_from(quiet[1]);
    assert!(later.iter().all(|(d, _, _)| *d >= quiet[1]), "{later:?}");
    assert_eq!(later.first().map(|(d, _, _)| *d), Some(quiet[1]));
}

/// BG11-07's extreme cases: no unmeasured session, a one-day record, the
/// span's two edge days, asking before, after and at the extremes of `i64`,
/// the longest span the grammar admits (`MAX_ROWS` days, unmeasured on its
/// first and last), and one day past it, refused by name before any kind is
/// built. Every calendar here is invented.
#[test]
fn the_unsized_days_hold_at_the_span_edges_and_extremes() {
    let template = seed_draft(Table::Calendar).unwrap();
    let source = template
        .rows
        .iter()
        .find(|row| row[1] == "2026-11-08")
        .map(|row| row[4].clone())
        .expect("the Muhurat row's citation");
    let text = |d: i64| day_of(d).unwrap().to_string();
    let row = |kind: &str, d: i64, windows: &str| -> Vec<String> {
        vec![
            kind.to_owned(),
            text(d),
            windows.to_owned(),
            format!("invented {kind}"),
            source.clone(),
        ]
    };
    let load_rows = |rows: Vec<Vec<String>>| {
        let store = scratch("ref-muhurat");
        let mut draft = seed_draft(Table::Calendar).unwrap();
        draft.rows = rows;
        write(&store, &draft).unwrap();
        load(&store)
    };
    let listed = |loaded: &Reference, from: i64| -> Vec<i64> {
        loaded
            .calendar
            .table()
            .unwrap()
            .unsized_from(from)
            .iter()
            .map(|(d, _, _)| *d)
            .collect()
    };
    let first = epoch(2018, 1, 1);

    // No session of unrecorded length: nothing to list, from any day.
    let none = load_rows(vec![row("first", first, "-"), row("last", first + 30, "-")]);
    for from in [i64::MIN, first, first + 30, i64::MAX] {
        assert!(listed(&none, from).is_empty(), "from {from}");
    }

    // A one-day record whose one day is unmeasured.
    let one = load_rows(vec![
        row("first", first, "-"),
        row("last", first, "-"),
        row("session", first, "unmeasured"),
    ]);
    assert_eq!(listed(&one, i64::MIN), vec![first]);
    assert_eq!(listed(&one, first), vec![first]);
    assert!(listed(&one, first + 1).is_empty());

    // Both edge days, the last one's row first.
    let last = first + 400;
    let edges = load_rows(vec![
        row("session", last, "unmeasured"),
        row("first", first, "-"),
        row("session", first, "unmeasured"),
        row("last", last, "-"),
    ]);
    assert_eq!(listed(&edges, i64::MIN), vec![first, last]);
    assert_eq!(listed(&edges, first + 1), vec![last]);
    assert_eq!(listed(&edges, last), vec![last]);
    assert!(listed(&edges, last + 1).is_empty());
    assert!(listed(&edges, i64::MAX).is_empty());
    let calendar = edges.calendar.table().unwrap();
    assert_eq!(calendar.kind_of(first - 1), DayKind::Unmeasured);
    assert_eq!(calendar.kind_of(last + 1), DayKind::Unmeasured);

    // The longest span the grammar admits: one pass over MAX_ROWS days.
    let far = first + i64::try_from(MAX_ROWS).unwrap() - 1;
    let longest = load_rows(vec![
        row("first", first, "-"),
        row("last", far, "-"),
        row("session", far, "unmeasured"),
        row("session", first, "unmeasured"),
    ]);
    assert_eq!(listed(&longest, i64::MIN), vec![first, far]);
    assert_eq!(listed(&longest, first + 1), vec![far]);

    // One day longer is refused by name before it lands: the writer builds
    // the table it is about to publish, so no generation is written.
    let store = scratch("ref-muhurat");
    let mut draft = seed_draft(Table::Calendar).unwrap();
    draft.rows = vec![row("first", first, "-"), row("last", far + 1, "-")];
    match write(&store, &draft) {
        Err(WriteError::Refused(why)) => {
            assert_eq!(why.kind, RefusalKind::Row, "{why:?}");
            assert!(why.detail.contains("longer than"), "{}", why.detail);
        }
        other => panic!("a span past MAX_ROWS days must be refused: {other:?}"),
    }
    assert!(!Table::Calendar.path(&store, 1).exists(), "nothing landed");
    assert_ne!(
        load(&store).calendar.origin(),
        "store",
        "nothing is served from the store"
    );
}

#[test]
fn a_calendar_change_that_closes_a_held_day_is_named_before_it_is_applied() {
    let store = seeded("ref-retract");
    let old = load(&store);
    // The first regular session from 2026-09-07 on, found, never typed.
    let held_day = (epoch(2026, 9, 7)..epoch(2026, 12, 31))
        .find(|day| {
            matches!(
                old.calendar.table().unwrap().kind_of(*day),
                DayKind::Open(_)
            ) && old.calendar.table().unwrap().note(*day).is_none()
        })
        .unwrap();
    let held_text = day_of(held_day).unwrap().to_string();
    let mut draft = seed_draft(Table::Calendar).unwrap();
    let at = draft
        .rows
        .iter()
        .skip(2)
        .position(|row| row[1] > held_text)
        .map_or(draft.rows.len(), |at| at + 2);
    draft.rows.insert(
        at,
        vec![
            "holiday".into(),
            held_text,
            "-".into(),
            "an invented closure, for this test only".into(),
            "an invented circular".into(),
        ],
    );
    write(&store, &draft).unwrap();
    let new = load(&store);
    let held = |day: i64| day == held_day;
    let named = old
        .calendar
        .table()
        .unwrap()
        .retractions(new.calendar.table().unwrap(), held);
    assert_eq!(named.len(), 1);
    assert_eq!(named[0].day, held_day);
    assert!(named[0].held);
    assert_eq!(named[0].now, DayKind::Closed);
    // The same calendar retracts nothing.
    assert!(
        new.calendar
            .table()
            .unwrap()
            .retractions(new.calendar.table().unwrap(), held)
            .is_empty()
    );
}

#[test]
fn two_writers_of_one_generation_never_both_land() {
    let store = seeded("ref-race");
    // A draft that differs from the seeded generation 1 (one invented row),
    // so eight writers contend for generation 2.
    let mut draft = seed_draft(Table::LotSizes).unwrap();
    let mut row = draft.rows[0].clone();
    row[0] = "INVENTEDRACE".to_owned();
    draft.rows.push(row);
    let outcomes: Vec<_> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| write(&store, &draft)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    // Exactly one writer LANDS generation 2. Every other one either lost the
    // race for it (`Raced(2)`) or listed after it landed and found exactly
    // its bytes there (`landed: false`, generation 2): no generation lands
    // twice, and the same rows never land as a second generation.
    let landed: Vec<u64> = outcomes
        .iter()
        .filter_map(|outcome| outcome.as_ref().ok().filter(|w| w.landed))
        .map(|w| w.generation)
        .collect();
    assert_eq!(landed, [2], "{outcomes:?}");
    for outcome in &outcomes {
        match outcome {
            Ok(written) => assert_eq!(written.generation, 2, "{outcomes:?}"),
            Err(WriteError::Raced(generation)) => assert_eq!(*generation, 2),
            Err(other) => panic!("{other}"),
        }
    }
    // The landed generation reads back and is served.
    let loaded = load(&store);
    assert_eq!(loaded.lots.origin(), "store");
    assert_eq!(loaded.lots.receipt().unwrap().generation, 2);
    let bytes = std::fs::read(Table::LotSizes.path(&store, 2)).unwrap();
    parse(Table::LotSizes, Some(2), Path::new("x"), &bytes).unwrap();
    assert!(!Table::LotSizes.path(&store, 3).exists(), "nothing past 2");
}

/// p50, p99 and max of `n` timed calls, in microseconds.
fn time(n: usize, mut call: impl FnMut()) -> (u128, u128, u128) {
    let mut took: Vec<u128> = (0..n)
        .map(|_| {
            let start = std::time::Instant::now();
            call();
            start.elapsed().as_micros()
        })
        .collect();
    took.sort_unstable();
    (took[n / 2], took[(n * 99) / 100], took[n - 1])
}

#[test]
fn the_loader_reads_every_table_inside_its_budget() {
    let store = seeded("ref-budget");
    write(
        &store,
        &rate_draft(&invented_rates(true), "invented", day(2026, 10, 20)).unwrap(),
    )
    .unwrap();
    let shipped = time(100, || {
        let loaded = load(&store);
        assert!(loaded.refusals().is_empty());
    });
    // Ten times the shipped rows of the largest table a run edits (the lot
    // table: 10 x the compiled rows, as invented underlyings).
    let big = seeded("ref-budget-10x");
    let mut draft = seed_draft(Table::LotSizes).unwrap();
    let base = draft.rows.len();
    for at in 0..(base * 9) {
        draft.rows.push(vec![
            format!("INVENTED{at}"),
            "2025-01-01".into(),
            "100".into(),
            "unverified".into(),
            "-".into(),
            "-".into(),
            "an invented row".into(),
        ]);
    }
    write(&big, &draft).unwrap();
    let tenfold = time(100, || {
        let loaded = load(&big);
        assert_eq!(loaded.lots.table().unwrap().rows().len(), base * 10);
    });
    eprintln!(
        "R3-30 load, n=100, microseconds: shipped p50 {} p99 {} max {}; \
         lot table x10 p50 {} p99 {} max {}",
        shipped.0, shipped.1, shipped.2, tenfold.0, tenfold.1, tenfold.2
    );
    assert!(shipped.0 < 50_000, "p50 under 50 ms: {shipped:?}");
    // The tail too (R3-30 repair 1: the budget held p50 alone). A load is
    // once per run, so its bound is generous; a load that walked something
    // per lookup would still blow it.
    assert!(shipped.1 < 250_000, "p99 under 250 ms: {shipped:?}");
    assert!(tenfold.1 < 250_000, "p99 under 250 ms at 10x: {tenfold:?}");
    // Ten times the rows of one table moves the whole load by much less than
    // ten times: the calendar dominates and is unchanged.
    assert!(
        tenfold.0 < shipped.0.saturating_mul(10).max(1_000),
        "{shipped:?} {tenfold:?}"
    );
}

/// FM-112. A symbol the exchange stopped using (or reassigned) is refused
/// by name on every later day, on the OPTIONS side (`underlying_stale`) and
/// on the CASH side (`symbol_not_in_use`) alike, and BOTH causes close the
/// day done with refusals: never an incomplete day re-read every night. The
/// refusal carries its symbol change, so every later day of one symbol is
/// ONE action item, keyed by the change.
#[test]
fn a_reused_symbol_is_one_action_item_and_not_a_nightly_incomplete() {
    use crate::gdfl_cm::CmRefusal;
    use crate::gdfl_import::{cm_cause, nfo_cause};
    use crate::gdfl_nfo::NfoRefusal;
    let reference = load(&scratch("ref-reuse"));
    let lineage: &'static Lineage = **reference.renames.table().unwrap();
    let change = lineage
        .rows()
        .iter()
        .find(|row| row.status.applies())
        .expect("an applied symbol change");
    let mut items = BTreeSet::new();
    for month in 1..=12 {
        let later = Ymd {
            year: change.effective.year + 1,
            month,
            day: 1,
        };
        let stale = lineage
            .filed_under(change.old, later)
            .expect_err("the old symbol after its change is refused");
        // THE ITEM IS THE CHANGE ROW, NOT THE DAY: the loader's lineage
        // refuses every later day with the same row, so every day of the
        // symbol keys one item (the set below is built from what
        // `filed_under`, `nfo_cause` and `cm_cause` answer, never from the
        // test's own `change`).
        assert_eq!(stale, alias::Stale::AfterChange(*change), "{later:?}");
        // The option day.
        let option = NfoRefusal::UnderlyingStale {
            ticker: format!("{}INVENTED", change.old),
            why: stale,
        };
        let cause = nfo_cause(&option);
        assert!(
            cause.closes,
            "an option day of a stale symbol closes: {cause:?}"
        );
        assert_eq!(cause.code, "underlying_stale");
        // The cash day.
        let cash = cm_cause(&CmRefusal::SymbolNotInUse { stale });
        assert!(cash.closes, "a cash day of a stale symbol closes: {cash:?}");
        assert_eq!(cash.code, "symbol_not_in_use");
        let (alias::Stale::AfterChange(on) | alias::Stale::BeforeChange(on)) = stale;
        items.insert((cause.code, on.old, on.new, on.effective));
        items.insert((cash.code, on.old, on.new, on.effective));
    }
    assert_eq!(
        items.len(),
        2,
        "one item per feed side for the symbol, not one per day"
    );
    // The new symbol before its change is the mirror refusal, also closing.
    let before = Ymd {
        year: change.effective.year - 1,
        month: 1,
        day: 1,
    };
    if let Err(stale) = lineage.filed_under(change.new, before) {
        assert!(matches!(stale, alias::Stale::BeforeChange(_)));
        assert!(
            nfo_cause(&NfoRefusal::UnderlyingStale {
                ticker: "X".into(),
                why: stale
            })
            .closes
        );
    }
}

/// DB-B44 replayed on a REAL table the operator keeps outside the tree: the
/// path is `BRUTEX_R330_RATES`, read-only, never copied into the repository.
/// Prints every session of the calendar of record from the table's first
/// release that the rate refuses as stale, and every one it carries over a
/// documented gap. Ignored by default: it needs the operator's file.
#[test]
#[ignore = "reads the operator's RBI table named by BRUTEX_R330_RATES"]
fn replay_a_real_rate_table_under_the_gap_rule() {
    let Ok(path) = std::env::var("BRUTEX_R330_RATES") else {
        panic!("BRUTEX_R330_RATES names no file");
    };
    let text = std::fs::read_to_string(&path).unwrap();
    let draft = rate_draft(&text, &path, day(2026, 10, 20)).unwrap();
    let store = scratch("ref-real-rates");
    write(&store, &draft).unwrap();
    let loaded = load(&store);
    let series = loaded.rates.table().unwrap();
    let first = i64::from(series.first_release().ordinal()) + 1;
    let last = (i64::from(series.last_release().ordinal())
        + i64::from(crate::riskfree::MAX_AGE_DAYS))
    .min(crate::calendar::SOURCED_LAST_DAY);
    let (mut sessions, mut stale, mut carried) = (0_u32, Vec::new(), Vec::new());
    for at in first.max(crate::calendar::SOURCED_FIRST_DAY)..=last {
        if !matches!(
            crate::calendar::sourced_kind_of(at),
            DayKind::Open(_) | DayKind::OpenLengthUnmeasured
        ) {
            continue;
        }
        sessions += 1;
        let civil = day_of(at).unwrap();
        match series.on(crate::pricing::trade_day_of(civil).unwrap()) {
            Ok(as_of) if as_of.carried.is_some() => carried.push(civil.to_string()),
            Ok(_) => {}
            Err(crate::riskfree::LookupError::Stale { .. }) => stale.push(civil.to_string()),
            Err(crate::riskfree::LookupError::HoleBeforeGap { gap, .. }) => {
                stale.push(format!(
                    "{civil} (gap {} follows an unexplained hole)",
                    gap.released
                ));
            }
            Err(why) => panic!("{why}"),
        }
    }
    eprintln!(
        "R3-30 rate replay {path}: auctions {} gaps {} sessions {sessions} \
         rate_stale {} {stale:?} rate_carried {} {carried:?}",
        series.auction_count(),
        series.gap_count(),
        stale.len(),
        carried.len()
    );
}

/// The lot seed with one invented row added, so it differs from generation 1.
fn changed_lots(tag: &str) -> Draft {
    let mut draft = seed_draft(Table::LotSizes).unwrap();
    let mut row = draft.rows[0].clone();
    row[0] = tag.to_owned();
    draft.rows.push(row);
    draft
}

#[test]
fn writing_the_same_rows_again_lands_no_new_generation() {
    let store = seeded("ref-idem");
    let draft = seed_draft(Table::LotSizes).unwrap();
    // The seeded generation 1 holds exactly these rows: nothing lands.
    let again = write(&store, &draft).unwrap();
    assert!(!again.landed, "{again:?}");
    assert_eq!(again.generation, 1);
    assert_eq!(again.path, Table::LotSizes.path(&store, 1));
    assert_eq!(again.sha256, load(&store).lots.receipt().unwrap().sha256);
    assert!(!Table::LotSizes.path(&store, 2).exists(), "no second copy");
    // Changed rows are a new generation.
    let landed = write(&store, &changed_lots("INVENTEDIDEM")).unwrap();
    assert!(landed.landed);
    assert_eq!(landed.generation, 2);
    // Going back to the first rows is a THIRD generation, never a rewind
    // to generation 1 (append-only).
    let back = write(&store, &draft).unwrap();
    assert!(back.landed);
    assert_eq!(back.generation, 3);
    assert_eq!(load(&store).lots.receipt().unwrap().generation, 3);
}

#[test]
fn the_last_generation_is_refused_by_name_not_reported_as_a_race() {
    let store = seeded("ref-exhausted");
    let draft = seed_draft(Table::LotSizes).unwrap();
    // Generation u64::MAX, planted: the last a name can carry.
    std::fs::write(
        Table::LotSizes.path(&store, u64::MAX),
        render(&draft, u64::MAX).unwrap(),
    )
    .unwrap();
    assert_eq!(load(&store).lots.receipt().unwrap().generation, u64::MAX);
    // The same rows: idempotent, nothing to write.
    let same = write(&store, &draft).unwrap();
    assert!(!same.landed);
    assert_eq!(same.generation, u64::MAX);
    // Other rows: refused by name. Before repair 1 this was `Raced(u64::MAX)`,
    // a race nobody ran.
    let refused = write(&store, &changed_lots("INVENTEDLAST")).unwrap_err();
    assert!(
        matches!(refused, WriteError::Exhausted(u64::MAX)),
        "{refused:?}"
    );
    assert!(refused.to_string().contains("no generation follows"));
}

#[test]
fn an_absent_table_is_told_as_a_warning_and_a_refused_file_as_an_error() {
    let store = seeded("ref-told");
    let loaded = load(&store);
    // No rate generation yet and no compiled copy: an absence, at warn. The
    // import loads every table and never reads this one.
    assert_eq!(
        told_as(&loaded.rates),
        (telemetry::Level::Warn, "table absent")
    );
    assert_eq!(
        told_as(&loaded.lots),
        (telemetry::Level::Info, "table loaded")
    );
    // A file that IS there and will not read is still an error.
    std::fs::create_dir_all(Table::RiskFreeRate.dir(&store)).unwrap();
    std::fs::write(Table::RiskFreeRate.path(&store, 1), b"torn").unwrap();
    let loaded = load(&store);
    assert_eq!(
        loaded.rates.table().map(drop).map_err(|why| why.generation),
        Err(Some(1))
    );
    assert_eq!(
        told_as(&loaded.rates),
        (telemetry::Level::Error, "table refused")
    );
}

#[test]
fn the_compiled_lineage_is_served_or_refused_never_emptied() {
    let lineage = Lineage::compiled().expect("the shipped rows form a lineage");
    assert_eq!(
        lineage.rows().len(),
        brutex_core::alias::LINEAGE.rows().len()
    );
    assert!(
        !lineage.rows().is_empty(),
        "the compiled lineage has rows to serve"
    );
}

#[test]
fn a_seeded_row_keeps_every_citations_checksum() {
    // Every compiled citation that records a sha256 is in its seeded row's
    // source text, beside its URL and read day: the round trip through the
    // store loses no provenance the charter recorded.
    let mut with_sha = 0_usize;
    let rows = brutex_core::alias::INDEX_ROWS
        .iter()
        .map(|row| row.status)
        .chain(
            brutex_core::alias::LINEAGE
                .rows()
                .iter()
                .map(|row| row.status),
        );
    let seeded: Vec<String> = [Table::IndexIdentity, Table::SymbolChanges]
        .into_iter()
        .flat_map(|table| seed_draft(table).unwrap().rows)
        .map(|row| row.last().cloned().unwrap())
        .collect();
    for (status, source) in rows.zip(&seeded) {
        if let RowStatus::Sourced(citations) = status {
            for citation in citations {
                assert!(source.contains(citation.url), "{source}");
                if let Some(sha) = citation.sha256 {
                    with_sha += 1;
                    assert!(source.contains(&format!("sha256 {sha}")), "{source}");
                }
            }
        }
    }
    assert!(with_sha > 0, "the compiled rows record checksums to keep");
}

/// R3-30 repair 1: a row LOADED from a seeded store cites its own URL, read
/// day and checksum, exactly the compiled row's, never the file's source in
/// their place. Before the repair every loaded citation was the file's
/// (`docs/00-charter.md` section) with no checksum.
#[test]
fn a_loaded_row_cites_its_own_source_and_checksum() {
    type Cite = (&'static str, &'static str, Option<&'static str>);
    let cites = |status: RowStatus| -> Option<Vec<Cite>> {
        match status {
            RowStatus::Sourced(citations) => Some(
                citations
                    .iter()
                    .map(|c| (c.url, c.read, c.sha256))
                    .collect(),
            ),
            RowStatus::Unverified(_) => None,
        }
    };
    let loaded = load(&seeded("ref-own-cite"));
    assert_eq!(loaded.renames.origin(), "store");
    assert_eq!(loaded.indices.origin(), "store");
    let lineage = loaded.renames.table().unwrap();
    let (mut compared, mut with_sha) = (0_usize, 0_usize);
    for (got, want) in lineage
        .rows()
        .iter()
        .zip(brutex_core::alias::LINEAGE.rows())
    {
        assert_eq!((got.old, got.new), (want.old, want.new));
        assert_eq!(cites(got.status), cites(want.status), "{}", want.old);
        compared += 1;
        with_sha += cites(want.status)
            .unwrap_or_default()
            .iter()
            .filter(|c| c.2.is_some())
            .count();
    }
    assert_eq!(compared, brutex_core::alias::LINEAGE.rows().len());
    let indices = loaded.indices.table().unwrap();
    for got in indices.applied() {
        let want = brutex_core::alias::INDEX_ROWS
            .iter()
            .find(|row| row.nse_name == got.nse_name)
            .expect("every loaded index row is a compiled one");
        assert_eq!(cites(got.status), cites(want.status), "{}", want.nse_name);
        compared += 1;
        with_sha += cites(want.status)
            .unwrap_or_default()
            .iter()
            .filter(|c| c.2.is_some())
            .count();
    }
    assert!(with_sha > 0, "checksums were compared, {compared} rows");
    // A URL holding `; ` and ` read ` parses whole; other words cite the file.
    let two = own_citations(
        "https://e.invalid/a; b read x read 2026-10-07; https://e.invalid/p read 2026-10-08 sha256 \
         0000000000000000000000000000000000000000000000000000000000000000",
    )
    .expect("two parts");
    assert_eq!(
        two.iter()
            .map(|c| (c.url, c.read, c.sha256))
            .collect::<Vec<_>>(),
        [
            ("https://e.invalid/a; b read x", "2026-10-07", None),
            (
                "https://e.invalid/p",
                "2026-10-08",
                Some("0000000000000000000000000000000000000000000000000000000000000000")
            ),
        ]
    );
    // A measurement's path is a location too, as `Citation::url` says: the
    // compiled charter and masters citations are paths.
    let path = own_citations("docs/00-charter.md, Index aliases read 2026-10-07")
        .expect("a path is a location");
    assert_eq!(
        path.iter().map(|c| (c.url, c.read)).collect::<Vec<_>>(),
        [("docs/00-charter.md, Index aliases", "2026-10-07")]
    );
    for words in [
        "",
        "an operator's own words",
        " read 2026-10-07",
        "u read 2026-1-07",
        // verify-R3-30-r2 followup: a part is a citation only when its URL is
        // an http(s) URL and its read day is a real calendar day.
        "operator note read 2026-01-01",
        "ftp://e.invalid/p read 2026-01-01",
        "https:// read 2026-01-01",
        "https://e.invalid/p read 2026-13-45",
        "https://e.invalid/p read 2026-02-30",
        "https://e.invalid/p read 2026-10-08; operator note read 2026-10-09",
    ] {
        assert!(own_citations(words).is_none(), "{words:?}");
    }
}

/// A STORED rate table whose gap row follows an unexplained hole is served,
/// the hole's days are refused naming the gap, and the load says so in one
/// sentence (R3-30 repair 2); a table whose gap continues a running clock
/// says nothing of the kind.
#[test]
fn a_stored_rate_gap_after_an_unexplained_hole_is_told_at_load() {
    let store = seeded("ref-rates-hole");
    let holed = [
        crate::riskfree::HEADER.to_owned(),
        format!("\t\t2020-01-01\t98.5000\t6.1081\t{RBI}\t\t"),
        format!("\t\t2020-01-20\tNA\t\t{RBI}\t\t"),
        format!("\t\t2020-02-05\t98.6000\t5.6951\t{RBI}\t\t"),
    ]
    .join("\n");
    write(
        &store,
        &rate_draft(&holed, "invented", day(2026, 10, 21)).unwrap(),
    )
    .unwrap();
    let loaded = load(&store);
    assert_eq!(loaded.rates.origin(), "store");
    let series = loaded.rates.table().unwrap();
    assert!(matches!(
        series.on(trade(2020, 1, 21)),
        Err(crate::riskfree::LookupError::HoleBeforeGap { .. })
    ));
    let said = unlinked_gaps_said(series).expect("told");
    assert!(
        said.starts_with("1 documented gap row(s) carry nothing") && said.contains("2020-01-20"),
        "{said}"
    );
    let linked = invented_rates(true);
    write(
        &store,
        &rate_draft(&linked, "invented", day(2026, 10, 21)).unwrap(),
    )
    .unwrap();
    let loaded = load(&store);
    assert_eq!(unlinked_gaps_said(loaded.rates.table().unwrap()), None);
}

/// The five NIFTY rows that rest on NSE/FAOP/68747 carry the checksum the
/// charter records for the bytes read (verify-R3-30-r2 followup), read from
/// `docs/00-charter.md` itself rather than a second copy.
#[test]
fn the_sourced_expiry_rows_carry_the_charters_checksum() {
    let charter = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/00-charter.md"),
    )
    .unwrap();
    let row = charter
        .lines()
        .find(|line| line.starts_with("| `NSE/FAOP/68747` |"))
        .expect("the charter's row for NSE/FAOP/68747");
    let sha = row
        .split('`')
        .find(|cell| cell.len() == 64 && cell.bytes().all(|b| b.is_ascii_hexdigit()))
        .expect("the charter records a sha256");
    let store = seeded("ref-expiry-sha");
    let loaded = load(&store);
    let table = loaded.expiry.table().unwrap();
    let sourced: Vec<&str> = table
        .rows()
        .iter()
        .map(|row| row.source.as_str())
        .filter(|source| source.starts_with("sourced: NSE/FAOP/68747"))
        .collect();
    assert_eq!(sourced.len(), 5, "{sourced:?}");
    for source in sourced {
        assert!(source.contains(&format!("sha256 {sha}")), "{source}");
    }
}

#[test]
fn the_expiry_seed_says_what_each_row_rests_on() {
    // Before repair 1 every row's source was the bare word of the table and
    // the file said `measured`, though five rows rest on NSE/FAOP/68747 and
    // five are inferred with their cause UNVERIFIED.
    let store = seeded("ref-expiry-basis");
    let loaded = load(&store);
    let table = loaded.expiry.table().unwrap();
    let sources: Vec<&str> = table.rows().iter().map(|row| row.source.as_str()).collect();
    let count = |prefix: &str| sources.iter().filter(|s| s.starts_with(prefix)).count();
    assert_eq!(count("sourced: NSE/FAOP/68747"), 5, "{sources:?}");
    assert_eq!(count("inferred from the underlying"), 5, "{sources:?}");
    assert_eq!(count("measured: "), sources.len() - 10, "{sources:?}");
    for row in table.rows() {
        if row.source.starts_with("sourced") {
            assert_eq!(row.underlying.as_deref(), Some("NIFTY"));
            assert_eq!(row.first_day, day(2025, 8, 1));
            assert!(row.source.contains("FAOP68747.pdf"), "{}", row.source);
        }
        if row.source.starts_with("inferred") {
            assert!(row.source.contains("UNVERIFIED"), "{}", row.source);
        }
    }
}

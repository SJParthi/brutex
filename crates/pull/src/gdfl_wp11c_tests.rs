//! WP-11c: the day ledger's labels for a halt and for a minute-sampled index
//! day (DB-A04, DB-A07). Every price here is invented (a few rupees, moving by
//! a paisa a second); the shapes are the findings': an index repeated every
//! second of a circuit-breaker halt, an index silent through one, and one row
//! a minute at second :59.

use super::*;

/// One row of an index file.
const fn tick(sod: u32, ltp: i64) -> Tick {
    Tick {
        sod,
        ltp,
        ltq: 0,
        oi: None,
    }
}

/// A file's quality on `on`, its bars one a distinct stamped second.
fn measured(kind: ImportKind, on: Day, ticks: &[Tick]) -> quality::FileQuality {
    let session = crate::gdfl_cm::session_of(on).unwrap();
    let mut seconds: Vec<u32> = ticks.iter().map(|t| t.sod).collect();
    seconds.sort_unstable();
    seconds.dedup();
    quality::measure_file(
        kind,
        (&session, quality::halt_window(kind, on)),
        ticks,
        &seconds,
        (Placement::default(), quality::Deferred::default()),
        quality::ReaderCounts::default(),
    )
}

/// The one line a day of `kind` writes for `files`, by scope.
fn lines_of(kind: ImportKind, on: Day, files: &[(&str, &[Tick])]) -> Vec<(String, String)> {
    let session = crate::gdfl_cm::session_of(on).unwrap();
    let mut ledger = quality::DayLedger::new(kind, on, session);
    for (symbol, ticks) in files {
        ledger.take(symbol, &measured(kind, on, ticks));
    }
    ledger.lines(true)
}

/// The value of `name=` in a line's fields.
fn field<'t>(fields: &'t str, name: &str) -> &'t str {
    fields
        .split(' ')
        .find_map(|word| word.strip_prefix(name)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("{name}= not in {fields:?}"))
}

/// The index files of a circuit-breaker day's shape (DB-A07): NIFTY printed
/// every second, its value repeated through the halt but for seven minutes
/// of the reopening auction; BANKNIFTY silent from the halt to the resume.
fn halt_shapes(on: Day) -> Vec<(String, Vec<u8>)> {
    let (from, resume) = quality::halt_window(ImportKind::Indices, on).unwrap();
    let auction = hms_at(10, 6, 0)..hms_at(10, 13, 0);
    // The value the halt repeats is the last one before it: the row at
    // 09:21:09 is second 369 of the session.
    let frozen = format!("77.{:02}", (from - 1 - NINE_FIFTEEN) % 100);
    let mut nifty: Vec<(u32, String, u64)> = Vec::new();
    let mut bank: Vec<(u32, String, u64)> = Vec::new();
    for s in 0..22_500 {
        let sod = NINE_FIFTEEN + s;
        let halted = (from..resume).contains(&sod);
        let moving = !halted || auction.contains(&sod);
        let value = if moving {
            format!("77.{:02}", s % 100)
        } else {
            frozen.clone()
        };
        nifty.push((sod, value, 0));
        if !halted {
            bank.push((sod, format!("55.{:02}", s % 100), 0));
        }
    }
    // A vendor's index file runs past the close; the import refuses one that
    // ends inside the session.
    nifty.push((CLOSE + 30, "77.00".to_owned(), 0));
    bank.push((CLOSE + 30, "55.00".to_owned(), 0));
    vec![
        (
            entry_of(Tree::Indices, on, "NIFTY 50.NSE_IDX"),
            cm_csv("NIFTY 50.NSE_IDX", on, &nifty),
        ),
        (
            entry_of(Tree::Indices, on, "NIFTY BANK.NSE_IDX"),
            cm_csv("NIFTY BANK.NSE_IDX", on, &bank),
        ),
    ]
}

/// **A HALT DAY, IMPORTED** (DB-A07): the index repeated through 2020-03-13's
/// halt keeps every one of its seconds, and its line says how many of them
/// lie inside the calendar's halt and how long the value stood still; the
/// index silent through it holds none and its hole is the halt's own length.
/// Before WP-11c neither line carried a word about the halt.
#[test]
fn a_halt_day_is_imported_as_printed_and_its_line_counts_the_halt() {
    let on = day(2020, 3, 13);
    assert_eq!(
        quality::halt_window(ImportKind::Indices, on),
        Some((hms_at(9, 21, 10), hms_at(10, 21, 1))),
        "the calendar of record's index seconds"
    );
    let root = scratch("wp11c-halt");
    let _sizes = world_of(&root, &[(on, Tree::Indices, halt_shapes(on))]);
    let store = root.join("S");
    let got = run_from(
        &root,
        (Src::Store, ImportKind::Indices),
        (on, on),
        (&store, &[]),
        false,
    );
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    let lines = flags_lines(&store);
    assert_eq!(lines.len(), 2, "{lines:#?}");
    let line = |scope: &str| {
        lines
            .iter()
            .find(|l| l.starts_with(&format!("flags indices 2020-03-13 {scope} v=1 ")))
            .unwrap_or_else(|| panic!("{scope} in {lines:#?}"))
    };
    let nifty = line("NIFTY");
    assert_eq!(field(nifty, "seconds"), "22500");
    assert_eq!(field(nifty, "session_s"), "22500");
    assert_eq!(field(nifty, "holes_5s"), "0");
    assert_eq!(field(nifty, "halt_s"), "3591");
    assert_eq!(field(nifty, "halt_held_s"), "3591");
    assert_eq!(field(nifty, "flat_max_s"), "2691", "09:21:09 to 10:05:59");
    assert_eq!(field(nifty, "flat_max_at"), "09:21:09");
    let bank = line("BANKNIFTY");
    assert_eq!(field(bank, "seconds"), "18909");
    assert_eq!(field(bank, "halt_s"), "3591");
    assert_eq!(field(bank, "halt_held_s"), "0");
    assert_eq!(field(bank, "holes_5s"), "1");
    assert_eq!(field(bank, "hole_max_s"), "3591");
    assert_eq!(field(bank, "hole_max_at"), "09:21:10");
    assert_eq!(field(bank, "flat_max_s"), "1");
    // LABELLED, NEVER DROPPED: every repeated second is a bar, as printed.
    let held = bars_on(&store, ("INDEX", "NIFTY", None), on);
    assert_eq!(held.len(), 22_500);
    let inside = held
        .iter()
        .filter(|bar| {
            (hms_at(9, 21, 10)..hms_at(10, 21, 1)).contains(&second_of(bar.ts_micros, on))
        })
        .count();
    assert_eq!(inside, 3_591);
    // A SECOND RUN SAYS NOTHING NEW and writes nothing.
    let again = run_from(
        &root,
        (Src::Store, ImportKind::Indices),
        (on, on),
        (&store, &[]),
        false,
    );
    assert_eq!(again.flags_lines, 0);
    assert_eq!(flags_lines(&store), lines);
    std::fs::remove_dir_all(root).unwrap();
}

/// **EACH SEGMENT'S OWN SECONDS** (the calendar's `HaltSeconds`): on
/// 2020-03-13 the shares stop a second before the index and the index
/// options 35 seconds after it, so a print is inside the halt for one kind
/// and outside it for another; a day with no recorded halt counts nothing,
/// and the outage day's halt lies between its two session windows.
#[test]
fn a_halt_is_measured_by_each_kinds_own_seconds_inside_the_session() {
    let on = day(2020, 3, 13);
    let edge = [
        tick(hms_at(9, 21, 8), 700),
        tick(hms_at(9, 21, 9), 701),
        tick(hms_at(9, 21, 44), 702),
        tick(hms_at(9, 21, 45), 703),
        tick(hms_at(10, 20, 59), 704),
        tick(hms_at(10, 21, 0), 705),
        tick(hms_at(10, 21, 1), 706),
    ];
    let held = |kind| measured(kind, on, &edge).halt_held_s;
    assert_eq!(held(ImportKind::Stocks), 4, "09:21:09 up to 10:21:00");
    assert_eq!(held(ImportKind::Indices), 4, "09:21:10 up to 10:21:01");
    assert_eq!(held(ImportKind::Options), 2, "09:21:45 up to 10:21:00");
    let traded: Vec<Tick> = edge.iter().map(|t| Tick { ltq: 5, ..*t }).collect();
    for (kind, halt_s, inside) in [
        (ImportKind::Stocks, "3591", "4"),
        (ImportKind::Options, "3555", "2"),
    ] {
        // Two files of the kind-day: the line sums their seconds.
        let lines = lines_of(kind, on, &[("AAA", &traded), ("BBB", &traded[..3])]);
        assert_eq!(lines.len(), 1);
        let fields = &lines[0].1;
        assert_eq!(field(fields, "halt_s"), halt_s, "{kind:?}");
        let both = inside.parse::<u64>().unwrap() + measured(kind, on, &traded[..3]).halt_held_s;
        assert_eq!(field(fields, "halt_held_s"), both.to_string(), "{kind:?}");
    }
    // NO HALT RECORDED: nothing is counted, and the fields are still there.
    let quiet = day(2020, 3, 12);
    assert_eq!(quality::halt_window(ImportKind::Indices, quiet), None);
    let lines = lines_of(ImportKind::Indices, quiet, &[("NIFTY", &edge)]);
    assert_eq!(field(&lines[0].1, "halt_s"), "0");
    assert_eq!(field(&lines[0].1, "halt_held_s"), "0");
    // THE OUTAGE: 11:40 to 15:45 is between the day's windows (09:15-11:39
    // and 15:45-16:59), so no session second is halted and none is held.
    let outage = day(2021, 2, 24);
    assert_eq!(
        quality::halt_window(ImportKind::Indices, outage),
        Some((hms_at(11, 40, 0), hms_at(15, 45, 0)))
    );
    let rows = [
        tick(hms_at(11, 39, 59), 800),
        tick(hms_at(11, 40, 0), 801),
        tick(hms_at(15, 44, 59), 802),
        tick(hms_at(15, 45, 0), 803),
    ];
    let lines = lines_of(ImportKind::Indices, outage, &[("NIFTY", &rows)]);
    assert_eq!(field(&lines[0].1, "halt_s"), "0");
    assert_eq!(field(&lines[0].1, "halt_held_s"), "0");
    assert_eq!(field(&lines[0].1, "seconds"), "2", "the two session rows");
}

/// **THE FLAT RUN, AT ITS EDGES**: no row, one row, a whole session of one
/// value, two values in one second, a late row, a row outside the session,
/// and two runs of one length (the first is named).
#[test]
fn the_flat_run_is_the_longest_stretch_of_one_value_in_file_order() {
    let on = day(2019, 5, 29);
    let flat = |ticks: &[Tick]| measured(ImportKind::Indices, on, ticks).flat_max;
    let at = |s: u32| NINE_FIFTEEN + s;
    assert_eq!(flat(&[]), None, "no row, no run");
    assert_eq!(flat(&[tick(at(7), 500)]), Some((at(7), 1)));
    let whole: Vec<Tick> = (0..22_500).map(|s| tick(at(s), 500)).collect();
    assert_eq!(flat(&whole), Some((NINE_FIFTEEN, 22_500)), "the maximum");
    assert_eq!(
        flat(&[tick(at(1), 500), tick(at(1), 501), tick(at(1), 500)]),
        Some((at(1), 1)),
        "two values in one second are three runs of one second"
    );
    assert_eq!(
        flat(&[tick(at(10), 500), tick(at(15), 500), tick(at(12), 500)]),
        Some((at(10), 6)),
        "a late row of the same value stays inside its run"
    );
    assert_eq!(
        flat(&[
            tick(at(20), 500),
            tick(hms_at(8, 0, 0), 999),
            tick(CLOSE + 5, 998),
            tick(at(29), 500),
        ]),
        Some((at(20), 10)),
        "a row outside the session neither counts nor breaks a run"
    );
    assert_eq!(
        flat(&[
            tick(at(0), 500),
            tick(at(4), 500),
            tick(at(5), 501),
            tick(at(9), 501),
            tick(at(10), 502),
        ]),
        Some((at(0), 5)),
        "of two runs of one length the first is named"
    );
    // The line: `0` and `-` for a file with no session row.
    let lines = lines_of(
        ImportKind::Indices,
        on,
        &[("NIFTY", &[tick(CLOSE + 5, 500)])],
    );
    assert_eq!(field(&lines[0].1, "flat_max_s"), "0");
    assert_eq!(field(&lines[0].1, "flat_max_at"), "-");
    // Two files filed under one index: the longer run is the line's.
    let lines = lines_of(
        ImportKind::Indices,
        on,
        &[
            ("NIFTY", &[tick(at(0), 500), tick(at(2), 500)]),
            ("NIFTY", &[tick(at(50), 600), tick(at(58), 600)]),
        ],
    );
    assert_eq!(field(&lines[0].1, "flat_max_s"), "9");
    assert_eq!(field(&lines[0].1, "flat_max_at"), "09:15:50");
}

/// **THE SAMPLED DAY IS NAMED BY ONE RULE** (DB-A04): 30 May 2019's NIFTY,
/// one row a minute at second :59, is sampled with 375 of 22,500 seconds;
/// an index printed every second is not; the rule's edge is two distinct
/// seconds of the minute; no rows is not a sampled day; and a stocks or
/// options day, whose files are trades, names none.
#[test]
fn a_minute_sampled_index_day_is_named_with_its_numbers() {
    let on = day(2019, 5, 30);
    let session = crate::gdfl_cm::session_of(on).unwrap();
    let every_minute = |seconds: &[u32]| -> Vec<Tick> {
        (0..375)
            .flat_map(|m| {
                seconds
                    .iter()
                    .map(move |s| tick(NINE_FIFTEEN + m * 60 + s, 400 + i64::from(m)))
            })
            .collect()
    };
    let minute = every_minute(&[59]);
    let two = every_minute(&[0, 59]);
    let three = every_minute(&[0, 30, 59]);
    let dense: Vec<Tick> = (0..22_500)
        .map(|s| tick(NINE_FIFTEEN + s, 400 + i64::from(s % 7)))
        .collect();
    let mut ledger = quality::DayLedger::new(ImportKind::Indices, on, session);
    for (symbol, ticks) in [
        ("NIFTY", &minute),
        ("BANKNIFTY", &dense),
        ("NIFTYIT", &two),
        ("NIFTYCPSE", &three),
        ("INDIAVIX", &Vec::new()),
    ] {
        ledger.take(symbol, &measured(ImportKind::Indices, on, ticks));
    }
    assert_eq!(
        ledger.sampled(),
        [
            quality::Sampled {
                scope: "NIFTY".to_owned(),
                rows: 375,
                seconds: 375,
                session_s: 22_500,
                ss_distinct: 1,
            },
            quality::Sampled {
                scope: "NIFTYIT".to_owned(),
                rows: 750,
                seconds: 750,
                session_s: 22_500,
                ss_distinct: 2,
            },
        ],
        "by scope: three seconds of the minute, every second and no row are not sampled"
    );
    // The rule, at its edges.
    assert!(quality::is_sampled(1, 0));
    assert!(quality::is_sampled(1, quality::SAMPLED_SS_DISTINCT));
    assert!(!quality::is_sampled(1, quality::SAMPLED_SS_DISTINCT + 1));
    assert!(
        !quality::is_sampled(0, 0),
        "a day with no row is not sampled"
    );
    assert!(quality::is_sampled(u64::MAX, 1));
    assert_eq!(quality::session_seconds(&session), 22_500);
    // THE IMPORT SAYS IT OUT LOUD: one warning an index, with the ledger's
    // numbers, from the import and from a rescan alike.
    let named = ledger.sampled();
    for src in [quality::Src::Import, quality::Src::Rescan] {
        let event = sampled_event(ImportKind::Indices, "2019-05-30", &named[0], src);
        assert_eq!(event.level(), telemetry::Level::Warn);
        assert_eq!(event.target(), TARGET);
        assert_eq!(event.message(), "index day is minute-sampled");
        assert_eq!(
            event.fields(),
            [
                ("kind", telemetry::Value::from("indices")),
                ("day", telemetry::Value::from("2019-05-30")),
                ("scope", telemetry::Value::from("NIFTY")),
                ("src", telemetry::Value::from(src.as_str())),
                ("rows", telemetry::Value::from(375_u64)),
                ("seconds", telemetry::Value::from(375_u64)),
                ("session_s", telemetry::Value::from(22_500_u64)),
                ("ss_distinct", telemetry::Value::from(1_u32)),
            ]
        );
        assert_eq!(event.dropped_fields(), 0);
    }
    for kind in [ImportKind::Stocks, ImportKind::Options] {
        let traded: Vec<Tick> = minute.iter().map(|t| Tick { ltq: 1, ..*t }).collect();
        let mut ledger = quality::DayLedger::new(kind, on, session);
        ledger.take("AAA", &measured(kind, on, &traded));
        assert!(ledger.sampled().is_empty(), "{kind:?}");
    }
}

/// **A RESCAN NAMES A SAMPLED DAY WHEN IT WRITES THE LINE, AND NOT AGAIN**
/// (WP-11c repair 1; D-4928 item 7): the first run over the day appends two
/// lines and names NIFTY; a second run over the same rows appends none and
/// names none (every run used to repeat the warning); a run that measures
/// the day differently writes the changed line and names it again. A line
/// of another kind, another day or another index names nothing.
#[test]
fn a_rescan_that_writes_no_new_line_names_no_sampled_day_again() {
    let on = day(2019, 5, 30);
    let session = crate::gdfl_cm::session_of(on).unwrap();
    let minute = |n: u32| -> Vec<Tick> {
        (0..n)
            .map(|m| tick(NINE_FIFTEEN + m * 60 + 59, 400 + i64::from(m)))
            .collect()
    };
    let dense: Vec<Tick> = (0..22_500)
        .map(|s| tick(NINE_FIFTEEN + s, 400 + i64::from(s % 7)))
        .collect();
    let root = scratch("wp11c-rescan-once");
    let path = quality::dayflags_path(&root);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let rescan = |nifty: &[Tick]| -> (usize, Vec<String>) {
        let mut flags = quality::Ledger::load(path.clone()).unwrap();
        let mut ledger = quality::DayLedger::new(ImportKind::Indices, on, session);
        ledger.take("NIFTY", &measured(ImportKind::Indices, on, nifty));
        ledger.take("BANKNIFTY", &measured(ImportKind::Indices, on, &dense));
        let sampled = ledger.sampled();
        assert_eq!(sampled.len(), 1, "NIFTY is sampled on every run");
        let news = flags.news(ledger.lines(true), quality::Src::Rescan);
        flags.append(&news).unwrap();
        let named = quality::sampled_among(ledger.kind(), ledger.day(), sampled, &news)
            .into_iter()
            .map(|one| one.scope)
            .collect();
        (news.len(), named)
    };
    assert_eq!(rescan(&minute(375)), (2, vec!["NIFTY".to_owned()]));
    assert_eq!(
        rescan(&minute(375)),
        (0, Vec::new()),
        "nothing new, no word"
    );
    assert_eq!(
        rescan(&minute(374)),
        (1, vec!["NIFTY".to_owned()]),
        "a changed line is written and named"
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap().lines().count(),
        3,
        "append-only: three lines over three runs"
    );
    // WHAT A LINE NAMES IS ITS OWN KEY: kind, day and index.
    let nifty = |scope: &str| quality::Sampled {
        scope: scope.to_owned(),
        rows: 375,
        seconds: 375,
        session_s: 22_500,
        ss_distinct: 1,
    };
    let line = |head: &str| vec![quality::render(head, quality::Src::Rescan, "rows=375")];
    let among = |kind: ImportKind, on: Day, news: &[String]| {
        quality::sampled_among(kind, on, vec![nifty("NIFTY"), nifty("NIFTYIT")], news)
    };
    let written = line("flags indices 2019-05-30 NIFTYIT v=1");
    assert_eq!(
        among(ImportKind::Indices, on, &written),
        [nifty("NIFTYIT")],
        "the index whose line was written, and no other"
    );
    assert!(among(ImportKind::Indices, day(2019, 5, 29), &written).is_empty());
    assert!(among(ImportKind::Stocks, on, &written).is_empty());
    assert!(among(ImportKind::Indices, on, &[]).is_empty());
    assert!(among(ImportKind::Indices, on, &["not a flags line".to_owned()]).is_empty());
}

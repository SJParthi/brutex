//! BASE-COV-PULL-PS1 (D-7335): the reference rows no test had reached at tip
//! 2e7a0b6d3b56 (Route B, recipe v4). The grammar's bounds at and one past
//! each; a row of the wrong width handed past the grammar to every builder;
//! an unverified row and an unverified file; a poisoned interner; what
//! `costs::lot` and the rate series refuse, with a line and without one; a
//! generation that will not read; an absence a builder names; and each way a
//! write fails after it has listed its table (a listing refused, a draft that
//! will not render, another writer first, a staging name gone, no descriptor
//! left to sync the directory). Every row, symbol, source and page is
//! invented; the one rate row's cut-off price (80) and yield (100.2747%) are
//! no auction's.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::os::unix::fs::PermissionsExt as _;

use super::*;
use crate::gdfl_fixtures::scratch;

/// The path a refusal names when the bytes were never a file.
const AT: &str = "/zq/invented";

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn row(fields: &[&str]) -> Vec<String> {
    fields.iter().map(|field| (*field).to_owned()).collect()
}

/// A sourced draft of `table` holding `rows`.
fn draft(table: Table, rows: Vec<Vec<String>>) -> Draft {
    Draft {
        table,
        source: "invented".to_owned(),
        source_sha256: None,
        fetched: day(2026, 10, 22),
        standing: Standing::Sourced,
        decision: "D-7335".to_owned(),
        rows,
    }
}

/// `draft` rendered as generation 1 and read back by the grammar: what the
/// loader hands a builder.
fn parsed(draft: &Draft) -> Parsed {
    let text = render(draft, 1).unwrap();
    parse(draft.table, Some(1), Path::new(AT), text.as_bytes()).unwrap()
}

/// One F&O list row of `symbol`, on the list from the day it was read.
fn listed(symbol: &str) -> Vec<String> {
    row(&[symbol, "-", "-", "invented"])
}

/// One index row with a GDFL key (so the CM scope is handed it), listed
/// `status` and citing `source`.
fn keyed_index_row(stem: &str, key: &str, status: &str, source: &str) -> Vec<String> {
    row(&[
        "-",
        "INVENTED INDEX",
        stem,
        "-",
        "-",
        key,
        "no",
        "-",
        "-",
        "-",
        "-",
        "-",
        status,
        source,
    ])
}

// ── the grammar ─────────────────────────────────────────────────────────────

/// A sink that refuses its first write and fails the test at any later one,
/// so a formatter that writes after a refusal is caught at that write.
#[derive(Default)]
struct RefusesFirst {
    writes: usize,
}

impl std::fmt::Write for RefusesFirst {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.writes += 1;
        assert_eq!(self.writes, 1, "a write after the refusal: {text:?}");
        Err(std::fmt::Error)
    }
}

/// A refusal told to a sink that refuses its first write answers the sink's
/// fault at once and writes nothing after it; told to a string, it names the
/// table, the kind, the generation, the file and the detail.
#[test]
fn a_refusal_told_to_a_sink_that_refuses_stops_at_the_refusal() {
    let refused = Refusal {
        table: Table::FnoList,
        path: PathBuf::from(AT),
        generation: Some(3),
        kind: RefusalKind::Row,
        detail: "an invented detail".to_owned(),
    };
    let mut sink = RefusesFirst::default();
    assert_eq!(
        std::fmt::write(&mut sink, format_args!("{refused}")),
        Err(std::fmt::Error)
    );
    assert_eq!(sink.writes, 1);
    assert_eq!(
        refused.to_string(),
        "reference table fno_list refused row (generation 3) at /zq/invented: an invented detail"
    );
}

/// The grammar's bounds at and one past each: a field of exactly
/// [`MAX_FIELD_BYTES`] renders and one byte more is refused, as is a field
/// holding `|` or a carriage return; a receipt value of exactly the bound
/// renders, and one that is empty or a byte past it is refused by its name.
#[test]
fn a_field_or_a_receipt_value_past_the_grammar_is_refused_before_render() {
    let table = Table::FnoList;
    let widest = "Z".repeat(MAX_FIELD_BYTES);
    let past = "Z".repeat(MAX_FIELD_BYTES + 1);
    assert!(render(&draft(table, vec![listed(&widest)]), 1).is_ok());
    for bad in [past.as_str(), "ZQ|A", "ZQ\rA"] {
        assert_eq!(
            render(&draft(table, vec![listed(bad)]), 1).unwrap_err(),
            format!(
                "fno_list row 1 field {bad:?} is empty, padded, too long or holds | or a line break"
            )
        );
    }
    let mut at_bound = draft(table, Vec::new());
    at_bound.source = widest;
    assert!(render(&at_bound, 1).is_ok());
    for bad in [String::new(), past] {
        let mut value = draft(table, Vec::new());
        value.source = bad;
        assert_eq!(
            render(&value, 1).unwrap_err(),
            "fno_list source is not one line of text"
        );
    }
}

/// A day is `YYYY-MM-DD` and nothing that only parses as numbers: a third
/// separator out of its place is no day, and neither is a sign, which an
/// integer parse alone would take (`"+1"` reads as 1).
#[test]
fn a_day_with_a_misplaced_separator_or_a_sign_is_no_day() {
    assert_eq!(parse_day("2026-01-07"), Some(day(2026, 1, 7)));
    assert_eq!(parse_day("2026-01x07"), None);
    assert_eq!(parse_day("2026-+1-07"), None);
    assert_eq!(parse_day("+026-01-07"), None);
}

/// A window's minute is two digits: `09:5` is refused where `09:05` is read,
/// by the minute reader and by the calendar row that holds it.
#[test]
fn a_window_minute_of_one_digit_is_refused() {
    assert_eq!(parse_hhmm("09:05"), Some(9 * 60 + 5));
    assert_eq!(parse_hhmm("09:5"), None);
    let calendar = parsed(&draft(
        Table::Calendar,
        vec![
            row(&["first", "2024-01-01", "-", "-", "invented"]),
            row(&["last", "2024-01-31", "-", "-", "invented"]),
            row(&["session", "2024-01-02", "09:15-15:3", "-", "invented"]),
        ],
    ));
    let refused = CalendarTable::build(&calendar, Path::new(AT)).unwrap_err();
    assert_eq!(refused.kind, RefusalKind::Row);
    assert_eq!(
        refused.detail,
        format!(
            "line 12: windows \"09:15-15:3\" are not ascending HH:MM-HH:MM, at most {MAX_WINDOWS}"
        )
    );
}

// ── the builders ────────────────────────────────────────────────────────────

/// `draft`'s rows read back by the grammar with one row of a single field
/// appended at line 99: what the grammar itself never hands a builder.
fn with_a_short_row(draft: &Draft) -> Parsed {
    let mut read = parsed(draft);
    read.rows.push((99, row(&["ZQ"])));
    read
}

/// A row of the wrong width handed past the grammar (whose own check gives
/// every row the table's width) is refused by name by every builder, never
/// read as a shorter row: the calendar passes it over while it looks for its
/// span and refuses it after, every other table at once.
#[test]
fn a_row_of_the_wrong_width_is_refused_by_every_builder() {
    let at = Path::new(AT);
    let span = vec![
        row(&["first", "2024-01-01", "-", "-", "invented"]),
        row(&["last", "2024-01-31", "-", "-", "invented"]),
    ];
    let short = |table: Table| with_a_short_row(&draft(table, Vec::new()));
    let calendar = with_a_short_row(&draft(Table::Calendar, span));
    for (table, refused) in [
        (
            Table::Calendar,
            CalendarTable::build(&calendar, at).map(drop),
        ),
        (
            Table::IndexIdentity,
            IndexTable::build(&short(Table::IndexIdentity), at).map(drop),
        ),
        (
            Table::SymbolChanges,
            build_lineage(&short(Table::SymbolChanges), at).map(drop),
        ),
        (
            Table::FnoList,
            FnoTable::build(&short(Table::FnoList), at).map(drop),
        ),
        (
            Table::ExpiryMoves,
            ExpiryTable::build(&short(Table::ExpiryMoves), at).map(drop),
        ),
        (
            Table::LotSizes,
            build_lots(&short(Table::LotSizes), at).map(drop),
        ),
    ] {
        assert_eq!(
            refused,
            Err(Refusal {
                table,
                path: at.to_path_buf(),
                generation: Some(1),
                kind: RefusalKind::Row,
                detail: "line 99: wrong field count".to_owned(),
            }),
            "{table}"
        );
    }
}

/// An index row whose spans run forward, or last one day, is kept with them,
/// in table order and by its stem.
#[test]
fn an_index_span_that_runs_forward_or_lasts_one_day_is_kept() {
    let rows = vec![row(&[
        "-",
        "INVENTED INDEX",
        "ZQ SPAN",
        "-",
        "ZQSPANZ",
        "ZQSPANG",
        "no",
        "2020-01-01",
        "2021-01-01",
        "2020-06-01",
        "2020-06-01",
        "-",
        "sourced",
        "invented",
    ])];
    let table =
        IndexTable::build(&parsed(&draft(Table::IndexIdentity, rows)), Path::new(AT)).unwrap();
    assert_eq!(table.rows().len(), 1);
    assert_eq!(
        table.rows()[0].zerodha_span,
        (Some(day(2020, 1, 1)), Some(day(2021, 1, 1)))
    );
    assert_eq!(
        table.rows()[0].gdfl_span,
        (Some(day(2020, 6, 1)), Some(day(2020, 6, 1)))
    );
    assert_eq!(table.by_stem("ZQ SPAN"), table.rows().first());
}

/// A row listed `unverified` in a sourced file, and a sourced row in an
/// unverified file, are both listed and neither is applied: the CM scope is
/// handed the index row as unverified in its own words, and the symbol change
/// moves no stem.
#[test]
fn an_unverified_row_or_file_is_listed_and_never_applied() {
    let at = Path::new(AT);
    let note = "an invented note of what is missing";
    let index = IndexTable::build(
        &parsed(&draft(
            Table::IndexIdentity,
            vec![keyed_index_row("ZQ UNV", "ZQUNV", "unverified", note)],
        )),
        at,
    )
    .unwrap();
    assert_eq!(index.applied().len(), 1);
    assert_eq!(index.applied()[0].status, RowStatus::Unverified(note));
    let mut changes = draft(
        Table::SymbolChanges,
        vec![row(&[
            "ZQUOLD",
            "ZQUNEW",
            "2024-01-01",
            "sourced",
            "invented",
        ])],
    );
    changes.standing = Standing::Unverified;
    let lineage = build_lineage(&parsed(&changes), at).unwrap();
    assert_eq!(lineage.rows()[0].status, RowStatus::Unverified("invented"));
    assert_eq!(lineage.from_old("ZQUOLD"), None);
    assert_eq!(lineage.current("ZQUOLD"), "ZQUOLD");
}

/// Panics while it holds this process's interner, which poisons it for every
/// later caller.
fn panic_holding_the_interner() {
    let _held = interned().lock();
    panic!("an invented panic while the interner is held");
}

/// A poisoned interner is never trusted with a table: the index rows are
/// built apart on every load (two loads share nothing), and the symbol
/// changes are refused by name, never served half-interned. Once the poison
/// is cleared, two loads share one copy again. Run in a process of its own
/// (the crate's `where_permission_binds` child): the interner is the
/// process's, and a poisoned one would refuse every other test's lineage.
#[test]
fn a_poisoned_interner_builds_index_rows_apart_and_refuses_a_lineage_by_name() {
    crate::support::where_permission_binds(
        "reference_data::ps1_tests::a_poisoned_interner_builds_index_rows_apart_and_refuses_a_lineage_by_name",
        a_poisoned_interner_builds_index_rows_apart_and_refuses_a_lineage_by_name_body,
    );
}

fn a_poisoned_interner_builds_index_rows_apart_and_refuses_a_lineage_by_name_body() {
    let at = Path::new(AT);
    let index = parsed(&draft(
        Table::IndexIdentity,
        vec![keyed_index_row(
            "ZQ POISON",
            "ZQPOISON",
            "sourced",
            "invented",
        )],
    ));
    let changes = parsed(&draft(
        Table::SymbolChanges,
        vec![row(&["ZQPA", "ZQPB", "2024-01-01", "sourced", "invented"])],
    ));
    assert!(
        std::thread::spawn(panic_holding_the_interner)
            .join()
            .is_err()
    );
    assert!(interned().is_poisoned());
    let first = IndexTable::build(&index, at).unwrap();
    let second = IndexTable::build(&index, at).unwrap();
    assert_eq!(first.applied(), second.applied());
    assert!(
        !std::ptr::eq(first.applied().as_ptr(), second.applied().as_ptr()),
        "built apart, nothing shared"
    );
    assert_eq!(
        build_lineage(&changes, at).err(),
        Some(Refusal {
            table: Table::SymbolChanges,
            path: at.to_path_buf(),
            generation: Some(1),
            kind: RefusalKind::Read,
            detail: "the process's table interner is poisoned by an earlier panic".to_owned(),
        })
    );
    interned().clear_poison();
    let shared = IndexTable::build(&index, at).unwrap();
    let again = IndexTable::build(&index, at).unwrap();
    assert!(
        std::ptr::eq(shared.applied().as_ptr(), again.applied().as_ptr()),
        "interned once"
    );
    assert_eq!(build_lineage(&changes, at).unwrap().current("ZQPA"), "ZQPB");
}

/// A symbol filed on a day after the change that made it, and never changed
/// away, is filed under itself, as is a stem no change names; the old symbol
/// on that day is stale.
#[test]
fn a_stem_no_change_takes_away_is_filed_under_itself() {
    let lineage = build_lineage(
        &parsed(&draft(
            Table::SymbolChanges,
            vec![row(&["ZQFA", "ZQFB", "2024-01-01", "sourced", "invented"])],
        )),
        Path::new(AT),
    )
    .unwrap();
    let after = Ymd::new(2024, 6, 1);
    assert_eq!(lineage.filed_under("ZQFB", after), Ok("ZQFB"));
    assert_eq!(lineage.filed_under("ZQNONE", after), Ok("ZQNONE"));
    assert_eq!(
        lineage.filed_under("ZQFA", after),
        Err(Stale::AfterChange(lineage.rows()[0]))
    );
}

/// A name's span listed after a later one of the same name is kept beside it
/// (spans need not come in day order), and each answers its own days.
#[test]
fn an_fno_span_listed_after_a_later_one_is_kept_beside_it() {
    let rows = vec![
        row(&["ZQB", "2024-03-01", "-", "invented"]),
        row(&["ZQB", "2024-01-01", "2024-01-31", "invented"]),
    ];
    let table = FnoTable::build(&parsed(&draft(Table::FnoList, rows)), Path::new(AT)).unwrap();
    assert_eq!(table.symbols(), ["ZQB"]);
    assert_eq!(table.member_on("ZQB", day(2024, 1, 15)), Membership::Member);
    assert_eq!(
        table.member_on("ZQB", day(2024, 2, 15)),
        Membership::NotMember
    );
    assert_eq!(table.member_on("ZQB", day(2024, 3, 15)), Membership::Member);
}

/// What `costs::lot` refuses is refused in its words at its line: one row
/// past its bound (half the grammar's, so the grammar lets it through) names
/// no line, and a padded underlying, which only a builder handed rows past
/// the grammar can see, names the line of its own row.
#[test]
fn a_lot_table_costs_refuses_is_refused_in_its_words_at_its_line() {
    let at = Path::new(AT);
    let lot = |underlying: &str| {
        row(&[
            underlying,
            "2024-01-01",
            "25",
            "unverified",
            "-",
            "-",
            "invented",
        ])
    };
    let mut many = parsed(&draft(Table::LotSizes, Vec::new()));
    many.rows = vec![(10, lot("ZQLOT")); costs::lot::MAX_LOT_ROWS + 1];
    let refused = build_lots(&many, at).unwrap_err();
    assert_eq!(
        (refused.kind, refused.generation),
        (RefusalKind::Row, Some(1))
    );
    assert_eq!(
        refused.detail,
        costs::lot::LotTableError::TooManyRows {
            rows: costs::lot::MAX_LOT_ROWS + 1
        }
        .to_string()
    );
    let mut padded = parsed(&draft(Table::LotSizes, Vec::new()));
    padded.rows = vec![(10, lot("ZQLOT")), (11, lot(" ZQLOT"))];
    let refused = build_lots(&padded, at).unwrap_err();
    assert_eq!(
        refused.detail,
        format!(
            "line 11: {}",
            costs::lot::LotTableError::Text {
                row: 2,
                column: "underlying"
            }
        )
    );
}

/// A rate generation of no rows is refused in the series' own words with no
/// line to name: the grammar holds it, the series has no auction.
#[test]
fn a_rate_table_of_no_rows_is_refused_with_no_line_to_name() {
    let refused = build_rates(
        &parsed(&draft(Table::RiskFreeRate, Vec::new())),
        Path::new(AT),
    )
    .err();
    assert_eq!(
        refused,
        Some(Refusal {
            table: Table::RiskFreeRate,
            path: PathBuf::from(AT),
            generation: Some(1),
            kind: RefusalKind::Row,
            detail: format!(
                "the series refused: {:?}",
                crate::riskfree::SeriesError::Empty
            ),
        })
    );
}

/// An invented RBI page, under the prefix the series asks of a row's source.
const RBI: &str = "https://www.rbi.org.in/ZQ-INVENTED-PAGE";

/// A row the series reads (it never reads the full-result column) whose
/// full-result field holds `|` cannot be carried by the grammar: the draft is
/// refused naming the line and the field, never written with a field cut in
/// two. The invented cut-off price and yield agree as the series checks.
#[test]
fn a_rate_row_the_grammar_cannot_carry_is_refused_by_its_line() {
    let tsv = format!(
        "{}\n\t\t2020-01-01\t80.0000\t100.2747\t{RBI}\tZQ|INVENTED\t\n",
        crate::riskfree::HEADER
    );
    assert!(
        crate::riskfree::Series::parse(&tsv).is_ok(),
        "the series reads the row"
    );
    assert_eq!(
        rate_draft(&tsv, "invented", day(2026, 10, 22)).err(),
        Some("line 2: \"ZQ|INVENTED\" cannot be carried by the reference grammar".to_owned())
    );
}

// ── the loader ──────────────────────────────────────────────────────────────

/// Three days before the compiled calendar's record ends its alert says how
/// many days are left (neither ended nor quiet), beside the absent rate table
/// named as not served.
#[test]
fn an_alert_inside_the_window_says_how_many_days_are_left() {
    let compiled = Reference::compiled();
    let last = compiled.calendar.table().unwrap().last_day();
    let alerts = compiled.horizon_alerts(last - 3);
    assert_eq!(alerts.len(), 2, "{alerts:?}");
    assert_eq!(alerts[0], (Table::Calendar, "ends in 3 days".to_owned()));
    assert_eq!(alerts[1].0, Table::RiskFreeRate);
    assert!(alerts[1].1.starts_with("not served: "), "{alerts:?}");
}

/// A generation that will not read is refused at its own name and number
/// before a byte is parsed: one past the byte bound (a sparse file, so
/// nothing is written), then a directory standing at the next name.
#[test]
fn a_generation_that_will_not_read_is_refused_at_its_name_and_number() {
    let store = scratch("ps1-ref-unread");
    let table = Table::FnoList;
    std::fs::create_dir_all(table.dir(&store)).unwrap();
    let big = table.path(&store, 1);
    std::fs::File::create(&big)
        .unwrap()
        .set_len(MAX_FILE_BYTES + 1)
        .unwrap();
    let slot = load_one(&store, table, FnoTable::build);
    assert_eq!(
        slot.table().map(drop).unwrap_err(),
        &Refusal {
            table,
            path: big,
            generation: Some(1),
            kind: RefusalKind::TooLarge,
            detail: format!("{} bytes, more than {MAX_FILE_BYTES}", MAX_FILE_BYTES + 1),
        }
    );
    let dir = table.path(&store, 2);
    std::fs::create_dir(&dir).unwrap();
    let slot = load_one(&store, table, FnoTable::build);
    let refused = slot.table().map(drop).unwrap_err();
    assert_eq!(
        (refused.kind, refused.generation, &refused.path),
        (RefusalKind::Read, Some(2), &dir)
    );
}

/// An absence a table's own builder names at a generation is a refused file,
/// told as an error: only an absence with no generation (no file at all) is
/// told at warn.
#[test]
fn an_absence_a_builder_names_at_a_generation_is_told_as_a_refused_file() {
    let store = scratch("ps1-ref-absent-at");
    let table = Table::FnoList;
    write(&store, &draft(table, vec![listed("ZQA")])).unwrap();
    let slot = load_one(
        &store,
        table,
        |read: &Parsed, path: &Path| -> Result<(), Refusal> {
            Err(Refusal {
                table,
                path: path.to_path_buf(),
                generation: Some(read.receipt.generation),
                kind: RefusalKind::Absent,
                detail: "an invented absence".to_owned(),
            })
        },
    );
    let refused = slot.table().map(drop).unwrap_err();
    assert_eq!(
        (refused.kind, refused.generation),
        (RefusalKind::Absent, Some(1))
    );
    assert_eq!(told_as(&slot), (telemetry::Level::Error, "table refused"));
}

// ── the writer ──────────────────────────────────────────────────────────────

/// A write into a table directory it may enter and not list is refused by
/// the listing, by name (the directory, no generation), before a byte is
/// staged. Run where the permission bits bind (D-0995): as root a directory
/// of mode 0o300 lists anyway.
#[test]
fn a_write_into_a_table_directory_that_will_not_list_is_refused_by_the_listing() {
    crate::support::where_permission_binds(
        "reference_data::ps1_tests::a_write_into_a_table_directory_that_will_not_list_is_refused_by_the_listing",
        a_write_into_a_table_directory_that_will_not_list_is_refused_by_the_listing_body,
    );
}

fn a_write_into_a_table_directory_that_will_not_list_is_refused_by_the_listing_body() {
    let store = scratch("ps1-ref-unlisted");
    let table = Table::FnoList;
    let dir = table.dir(&store);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o300)).unwrap();
    let outcome = write(&store, &draft(table, vec![listed("ZQA")]));
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let Err(WriteError::Refused(refused)) = &outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(
        (refused.table, refused.kind, refused.generation),
        (table, RefusalKind::Read, None)
    );
    assert_eq!(refused.path, dir);
    assert_eq!(
        newest(&store, table).unwrap(),
        (None, Strays::default()),
        "nothing staged, nothing landed"
    );
}

/// A draft the grammar cannot carry, written over a held generation, is
/// refused as a draft: it is no copy of the held bytes (it does not render
/// as them), and nothing new lands.
#[test]
fn a_draft_that_will_not_render_over_a_held_generation_lands_nothing() {
    let store = scratch("ps1-ref-held-unrendered");
    let table = Table::FnoList;
    let first = write(&store, &draft(table, vec![listed("ZQA")])).unwrap();
    assert_eq!((first.generation, first.landed), (1, true));
    let mut unrendered = draft(table, vec![listed("ZQA")]);
    unrendered.decision = String::new();
    let Err(WriteError::Draft(why)) = write(&store, &unrendered) else {
        panic!("a draft with no decision was written");
    };
    assert_eq!(why, "fno_list decision is not one line of text");
    assert_eq!(
        newest(&store, table).unwrap().0,
        Some((1, table.path(&store, 1)))
    );
}

/// The most descriptors a fill takes before it says the host's limit is past
/// what a test should hold.
const DESCRIPTOR_BOUND: usize = 1 << 22;

/// Every descriptor this process may still open, held, and the host's refusal
/// of one more.
fn every_descriptor() -> (Vec<std::fs::File>, std::io::Error) {
    let seed = std::fs::File::open("/dev/null").unwrap();
    let mut held = Vec::new();
    let refused = loop {
        match seed.try_clone() {
            Ok(one) => held.push(one),
            Err(why) => break why,
        }
        assert!(
            held.len() < DESCRIPTOR_BOUND,
            "a descriptor limit past {DESCRIPTOR_BOUND}"
        );
    };
    held.push(seed);
    (held, refused)
}

/// A generation linked in a process with no descriptor left to open its
/// directory is linked but not durable, and the write says exactly that,
/// never that it landed. One descriptor is left free: the listing takes it
/// and gives it back, the staging file takes it and keeps it, and the
/// directory's open finds none. Run in a process of its own (the crate's
/// `where_permission_binds` child), whose descriptors are this test's alone.
#[test]
fn a_generation_linked_with_no_descriptor_left_to_sync_its_directory_says_so() {
    crate::support::where_permission_binds(
        "reference_data::ps1_tests::a_generation_linked_with_no_descriptor_left_to_sync_its_directory_says_so",
        a_generation_linked_with_no_descriptor_left_to_sync_its_directory_says_so_body,
    );
}

fn a_generation_linked_with_no_descriptor_left_to_sync_its_directory_says_so_body() {
    let store = scratch("ps1-ref-no-descriptor");
    let table = Table::FnoList;
    let fno = draft(table, vec![listed("ZQA")]);
    let (mut held, refused) = every_descriptor();
    drop(held.pop());
    let outcome = write(&store, &fno);
    drop(held);
    let Err(WriteError::Io(said)) = &outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(
        said,
        &format!(
            "{}: generation 1 is linked but its directory did not sync: {refused}",
            table.dir(&store).display()
        )
    );
    assert_eq!(
        newest(&store, table).unwrap(),
        (Some((1, table.path(&store, 1))), Strays::default()),
        "linked at its final name, no staging name left"
    );
}

// ── the seeds ───────────────────────────────────────────────────────────────

/// A compiled row the charter cannot source seeds as `unverified`, carrying
/// its own words of what is missing, never a citation.
#[test]
fn an_unverified_compiled_row_seeds_unverified_in_its_own_words() {
    assert_eq!(
        cited(RowStatus::Unverified(
            "an invented account of what is missing"
        )),
        (
            Standing::Unverified,
            "an invented account of what is missing".to_owned()
        )
    );
}

/// **A LINK ANOTHER WRITER BEAT IS A RACE LOST, AND ONE THAT FAILS FOR ANY
/// OTHER REASON IS AN I/O REFUSAL AT THE FINAL NAME** (the link seam,
/// D-7335): a link answering `AlreadyExists` is `Raced` at the generation the
/// write asked for; one answering another fault is `Io` naming the final path
/// in the host's words; and neither leaves its staging name or a generation
/// behind. Extremes at once: the first generation of an empty table, and the
/// two faults on one table one after the other.
#[test]
fn a_link_another_writer_beat_is_a_race_lost_and_any_other_fault_is_io() {
    let store = scratch("ps1-rd-link");
    let table = Table::FnoList;
    let fno = draft(table, vec![listed("ZQA")]);
    let beaten = |_: &Path, _: &Path| -> std::io::Result<()> {
        Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists))
    };
    let raced = write_linking(&store, &fno, &beaten).unwrap_err();
    assert!(matches!(raced, WriteError::Raced(1)), "{raced:?}");
    let gone = |_: &Path, _: &Path| -> std::io::Result<()> {
        Err(std::io::Error::other("an invented staging name gone"))
    };
    let WriteError::Io(why) = write_linking(&store, &fno, &gone).unwrap_err() else {
        panic!("another fault is Io");
    };
    assert_eq!(
        why,
        format!(
            "{}: an invented staging name gone",
            table.path(&store, 1).display()
        )
    );
    assert_eq!(
        std::fs::read_dir(table.dir(&store)).unwrap().count(),
        0,
        "no staging name and no generation are left"
    );
}

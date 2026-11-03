//! BASE-COV-PULL: every refusal of the reference grammar and of the tables
//! built on it that no test had reached (Route B, recipe v4). A file is made
//! here with a true checksum and one wrong line, so each refusal is reached
//! on its own and never hidden behind the checksum. Every row is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_fixtures::scratch;

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// `body` with a true checksum line.
fn sealed(body: &str) -> Vec<u8> {
    format!("{body}sha256 {}\n", sha256_hex(body.as_bytes())).into_bytes()
}

/// Every header line of a file of `table`, generation 1, in order.
fn header_lines(table: Table) -> Vec<String> {
    vec![
        format!("{MAGIC} {VERSION}"),
        format!("table {table}"),
        "generation 1".to_owned(),
        "source invented".to_owned(),
        "source_sha256 -".to_owned(),
        "fetched 2026-10-22".to_owned(),
        "status measured".to_owned(),
        "decision D-7333".to_owned(),
        format!("columns {}", table.columns().join("|")),
    ]
}

/// The file made from `lines`, each with its line break. Every caller passes
/// a header, so the body is never empty; the empty file is `sealed("")`,
/// read by `a_header_is_refused_at_its_own_line`.
fn file(lines: &[String]) -> Vec<u8> {
    let mut body = lines.join("\n");
    body.push('\n');
    sealed(&body)
}

fn refused(bytes: &[u8]) -> Refusal {
    parse(
        Table::SymbolChanges,
        Some(1),
        Path::new("/invented/table"),
        bytes,
    )
    .unwrap_err()
}

/// The header with line `at` replaced.
fn with_line(at: usize, line: &str) -> Vec<u8> {
    let mut lines = header_lines(Table::SymbolChanges);
    lines[at] = line.to_owned();
    lines.push("rows 0".to_owned());
    file(&lines)
}

#[test]
fn a_file_past_the_bound_is_refused_before_it_is_read() {
    let big = vec![b'x'; usize::try_from(MAX_FILE_BYTES).unwrap() + 1];
    let why = refused(&big);
    assert_eq!(why.kind, RefusalKind::TooLarge);
    assert!(why.detail.contains("more than 8388608"), "{why}");
}

#[test]
fn a_header_is_refused_at_its_own_line() {
    let cut = refused(&sealed(""));
    assert_eq!(cut.kind, RefusalKind::Magic);
    assert!(cut.detail.contains("ends before its"), "{cut}");
    let stranger = refused(&with_line(0, "not-a-reference-file 1"));
    assert_eq!(stranger.kind, RefusalKind::Magic);
    assert!(stranger.detail.contains("line 1 is not"), "{stranger}");
    let version = refused(&with_line(0, &format!("{MAGIC} 999")));
    assert_eq!(version.kind, RefusalKind::Version);
    assert_eq!(
        version.detail,
        format!("version 999; this build reads {VERSION}")
    );
    for (at, line, word) in [
        (2, "generation x", "is not a generation"),
        (
            4,
            "source_sha256 xyz",
            "is neither `-` nor 64 lower-case hex",
        ),
        (5, "fetched yesterday", "is not a YYYY-MM-DD day"),
        (
            6,
            "status rumoured",
            "is not sourced, measured or unverified",
        ),
    ] {
        let why = refused(&with_line(at, line));
        assert_eq!(why.kind, RefusalKind::Header, "{line}");
        assert!(why.detail.contains(word), "{why}");
    }
    let generation = refused(&with_line(2, "generation 2"));
    assert_eq!(generation.kind, RefusalKind::Generation);
    let columns = refused(&with_line(8, "columns a|b"));
    assert_eq!(columns.kind, RefusalKind::Columns);
    let sha = with_line(4, &format!("source_sha256 {}", "a".repeat(64)));
    let read = parse(Table::SymbolChanges, Some(1), Path::new("/x"), &sha).unwrap();
    assert_eq!(read.receipt.source_sha256, Some("a".repeat(64)));
}

#[test]
fn a_row_section_of_any_other_shape_is_refused_by_name() {
    let body = |tail: &[&str]| {
        let mut lines = header_lines(Table::SymbolChanges);
        lines.extend(tail.iter().map(|line| (*line).to_owned()));
        file(&lines)
    };
    let short = refused(&body(&["row a|b", "rows 1"]));
    assert_eq!(short.kind, RefusalKind::Row);
    assert!(short.detail.contains("2 fields where"), "{short}");
    let padded = refused(&body(&["row a|b| c|d|e", "rows 1"]));
    assert_eq!(padded.kind, RefusalKind::Row);
    let other = refused(&body(&["neither", "rows 0"]));
    assert!(other.detail.contains("is neither `row ...`"), "{other}");
    let none = refused(&body(&[]));
    assert_eq!(none.kind, RefusalKind::RowCount);
    assert_eq!(none.detail, "no `rows <n>` line");
    let count = refused(&body(&["rows 3"]));
    assert_eq!(count.kind, RefusalKind::RowCount);
    let after = refused(&body(&["rows 0", "row a|b|c|d|e"]));
    assert!(after.detail.contains("follows the `rows` line"), "{after}");
}

/// The bound of rows: the extreme of a file that fits its bytes and passes
/// its rows (`row x|x|x|x|x` is 14 bytes; 200,001 of them fit 8 MiB).
#[test]
fn more_rows_than_the_bound_are_refused_by_name() {
    let mut lines = header_lines(Table::SymbolChanges);
    lines.extend(std::iter::repeat_n(
        "row x|x|x|x|x".to_owned(),
        MAX_ROWS + 1,
    ));
    let why = refused(&file(&lines));
    assert_eq!(why.kind, RefusalKind::TooMany);
}

fn draft(table: Table, rows: Vec<Vec<String>>) -> Draft {
    Draft {
        table,
        source: "invented".to_owned(),
        source_sha256: None,
        fetched: day(2026, 10, 22),
        standing: Standing::Sourced,
        decision: "D-7333".to_owned(),
        rows,
    }
}

fn row(fields: &[&str]) -> Vec<String> {
    fields.iter().map(|field| (*field).to_owned()).collect()
}

#[test]
fn a_draft_refused_before_it_is_rendered_says_why() {
    let table = Table::SymbolChanges;
    let mut lines = draft(table, Vec::new());
    lines.source = "two\nlines".to_owned();
    assert!(
        render(&lines, 1)
            .unwrap_err()
            .contains("source is not one line")
    );
    let mut sha = draft(table, Vec::new());
    sha.source_sha256 = Some("xyz".to_owned());
    assert!(
        render(&sha, 1)
            .unwrap_err()
            .contains("source_sha256 is not 64")
    );
    let many = draft(table, vec![row(&["a"]); MAX_ROWS + 1]);
    assert!(render(&many, 1).unwrap_err().contains("more than 200000"));
    let short = draft(table, vec![row(&["a"])]);
    assert!(
        render(&short, 1)
            .unwrap_err()
            .contains("row 1 has 1 fields")
    );
    let bad = draft(table, vec![row(&["a", "b", "", "d", "e"])]);
    assert!(render(&bad, 1).unwrap_err().contains("field \"\" is empty"));
}

fn refusal(table: Table, rows: Vec<Vec<String>>) -> String {
    write(&scratch("cov-ref"), &draft(table, rows))
        .unwrap_err()
        .to_string()
}

#[test]
fn a_calendar_is_refused_for_each_row_that_cannot_stand() {
    let t = Table::Calendar;
    let first = row(&["first", "2024-01-01", "-", "-", "invented"]);
    let last = row(&["last", "2024-01-31", "-", "-", "invented"]);
    let session = |day: &str, windows: &str| row(&["session", day, windows, "-", "invented"]);
    let cal = |extra: &[Vec<String>]| {
        let mut rows = vec![first.clone(), last.clone()];
        rows.extend_from_slice(extra);
        rows
    };
    assert!(
        refusal(t, vec![first.clone(), first.clone(), last.clone()]).contains("second `first`")
    );
    assert!(refusal(t, vec![first.clone(), last.clone(), last.clone()]).contains("second `last`"));
    assert!(refusal(t, vec![first.clone()]).contains("needs one `first` and one `last`"));
    for windows in [
        "9:15-15:30",
        "09:15",
        "09:15-",
        "25:00-26:00",
        "09:60-10:00",
        "1a:00-12:00",
        "15:30-09:15",
        "09:15-10:00,09:30-11:00",
        "09:15-09:20,09:30-09:40,10:00-10:10,11:00-11:10,12:00-12:10,13:00-13:10,14:00-14:10,15:00-15:10,15:20-15:25",
    ] {
        let said = refusal(t, cal(&[session("2024-01-02", windows)]));
        assert!(said.contains("are not ascending"), "{windows}: {said}");
    }
    let mixed = cal(&[row(&[
        "holiday",
        "2024-01-02",
        "09:15-15:30",
        "-",
        "invented",
    ])]);
    assert!(refusal(t, mixed).contains("a row is first, last or holiday"));
    let outside = cal(&[session("2024-02-02", "09:15-15:30")]);
    assert!(refusal(t, outside).contains("lies outside the calendar's own span"));
    let twice = cal(&[
        session("2024-01-02", "09:15-15:30"),
        session("2024-01-02", "unmeasured"),
    ]);
    assert!(refusal(t, twice).contains("a second row for one day"));
}

/// One good row of the index table, with `edit` applied.
fn index_row(stem: &str, key: &str) -> Vec<String> {
    row(&[
        "-",
        "INVENTED INDEX",
        stem,
        "-",
        key,
        "-",
        "no",
        "-",
        "-",
        "-",
        "-",
        "-",
        "sourced",
        "invented",
    ])
}

#[test]
fn an_index_table_is_refused_for_each_row_that_cannot_stand() {
    let t = Table::IndexIdentity;
    let mut flag = index_row("ZQ IDX", "ZQIDX");
    flag[6] = "maybe".to_owned();
    assert!(refusal(t, vec![flag]).contains("reference_only \"maybe\" is not yes or no"));
    let mut backwards = index_row("ZQ IDX", "ZQIDX");
    backwards[7] = "2024-02-01".to_owned();
    backwards[8] = "2024-01-01".to_owned();
    assert!(refusal(t, vec![backwards]).contains("runs backwards"));
    let stems = vec![index_row("ZQ IDX", "ZQA"), index_row("ZQ IDX", "ZQB")];
    assert!(refusal(t, stems).contains("is on two rows"));
    let keys = vec![index_row("ZQ IDX", "ZQA"), index_row("ZQ OTHER", "ZQA")];
    assert!(refusal(t, keys).contains("key \"ZQA\" is on two rows"));
}

#[test]
fn a_lineage_table_is_refused_for_each_row_that_cannot_stand() {
    let t = Table::SymbolChanges;
    let change = |old: &str, new: &str| row(&[old, new, "2024-01-01", "sourced", "invented"]);
    let mut status = change("ZQA", "ZQB");
    status[3] = "rumoured".to_owned();
    assert!(refusal(t, vec![status]).contains("is not a standing"));
    assert!(refusal(t, vec![change("ZQ A", "ZQB")]).contains("is not a share symbol"));
    assert!(refusal(t, vec![change("ZQA", "ZQA")]).contains("changes to itself"));
    let twice = vec![change("ZQA", "ZQB"), change("ZQA", "ZQC")];
    assert!(refusal(t, twice).contains("changes away twice"));
    let reached = vec![change("ZQA", "ZQC"), change("ZQB", "ZQC")];
    assert!(refusal(t, reached).contains("is reached twice"));
    let round = vec![change("ZQA", "ZQB"), change("ZQB", "ZQA")];
    assert!(refusal(t, round).contains("returns to it"));
}

/// A chain walks both ways, names its newest symbol, and answers each date
/// side of a change by the same rule the compiled table does.
#[test]
fn a_stored_lineage_walks_both_ways_from_any_link() {
    let store = scratch("cov-lineage");
    let change = |old: &str, new: &str, on: &str| row(&[old, new, on, "sourced", "invented"]);
    write(
        &store,
        &draft(
            Table::SymbolChanges,
            vec![
                change("ZQA", "ZQB", "2021-01-01"),
                change("ZQB", "ZQC", "2022-01-01"),
            ],
        ),
    )
    .unwrap();
    let slot = load_one(&store, Table::SymbolChanges, build_lineage);
    let lineage = slot.table().unwrap();
    assert_eq!(lineage.current("ZQA"), "ZQC");
    assert_eq!(lineage.current("ZQC"), "ZQC");
    assert_eq!(lineage.current("ZQNONE"), "ZQNONE");
    let back: Vec<&str> = lineage.backward("ZQC").map(|hop| hop.old).collect();
    assert_eq!(back, ["ZQB", "ZQA"]);
    assert_eq!(lineage.backward("ZQA").count(), 0);
    let inside = brutex_core::alias::Ymd {
        year: 2021,
        month: 6,
        day: 1,
    };
    assert_eq!(lineage.filed_under("ZQB", inside), Ok("ZQC"));
}

#[test]
fn an_fno_list_is_refused_for_each_span_that_cannot_stand() {
    let t = Table::FnoList;
    let span = |symbol: &str, entry: &str, exit: &str| row(&[symbol, entry, exit, "invented"]);
    assert!(refusal(t, vec![span("ZQA", "2024-02-01", "2024-01-01")]).contains("runs backwards"));
    let overlap = vec![
        span("ZQA", "2024-01-01", "2024-03-01"),
        span("ZQA", "2024-02-01", "2024-04-01"),
    ];
    assert!(refusal(t, overlap).contains("two spans that overlap"));
    let many: Vec<Vec<String>> = (1..=MAX_SPANS_PER_SYMBOL + 1)
        .map(|i| {
            let year = 2000 + i;
            span("ZQA", &format!("{year}-01-01"), &format!("{year}-06-01"))
        })
        .collect();
    assert!(refusal(t, many).contains("more than 16 spans"));
}

/// Membership on each side of each edge of a span, a span with no start and
/// a share the table does not hold; and what the table says against the
/// compiled universe, both ways.
#[test]
fn an_fno_table_answers_each_side_of_each_edge() {
    let store = scratch("cov-fno");
    let span = |symbol: &str, entry: &str, exit: &str| row(&[symbol, entry, exit, "invented"]);
    write(
        &store,
        &draft(
            Table::FnoList,
            vec![
                span("ZQA", "2024-01-01", "2024-01-31"),
                span("ZQA", "2024-03-01", "-"),
                span("ZQOLD", "-", "2024-01-31"),
                span("ZQNOW", "-", "-"),
            ],
        ),
    )
    .unwrap();
    let slot = load_one(&store, Table::FnoList, FnoTable::build);
    let table = slot.table().unwrap();
    let on = |symbol: &str, d: Day| table.member_on(symbol, d);
    assert_eq!(on("ZQNONE", day(2024, 1, 2)), Membership::NotMember);
    assert_eq!(on("ZQA", day(2023, 12, 31)), Membership::NotMember);
    assert_eq!(on("ZQA", day(2024, 1, 1)), Membership::Member);
    assert_eq!(on("ZQA", day(2024, 1, 31)), Membership::Member);
    assert_eq!(on("ZQA", day(2024, 2, 1)), Membership::NotMember);
    assert_eq!(on("ZQA", day(2030, 1, 1)), Membership::Member);
    assert_eq!(on("ZQNOW", day(2000, 1, 1)), Membership::Unmeasured);
    assert_eq!(on("ZQOLD", day(2024, 2, 1)), Membership::NotMember);
    assert_eq!(table.symbols(), ["ZQA", "ZQOLD", "ZQNOW"]);
    let current = table.current();
    assert!(
        current.contains(&"ZQA") && current.contains(&"ZQNOW"),
        "{current:?}"
    );
    assert!(!current.contains(&"ZQOLD"), "{current:?}");
    let (extra, missing) = table.drift_from_swept();
    assert_eq!(extra, ["ZQA", "ZQNOW"]);
    assert_eq!(missing.len(), brutex_core::universe::FNO_UNDERLYINGS.len());
}

#[test]
fn an_expiry_table_refuses_a_key_twice_and_finds_a_move_by_its_share_or_all() {
    let t = Table::ExpiryMoves;
    let moved = |underlying: &str, stated: &str| {
        row(&[
            underlying,
            stated,
            "2024-03-27",
            "-",
            "2024-03-01",
            "closed_day",
            "3",
            "invented",
        ])
    };
    let twice = vec![moved("ZQA", "2024-03-28"), moved("ZQA", "2024-03-28")];
    assert!(refusal(t, twice).contains("ZQA 2024-03-28 is on two rows"));
    let store = scratch("cov-expiry");
    let rows = vec![moved("ZQA", "2024-03-28"), moved("*", "2024-03-28")];
    write(&store, &draft(t, rows)).unwrap();
    let slot = load_one(&store, t, ExpiryTable::build);
    let table = slot.table().unwrap();
    let stated = day(2024, 3, 28);
    assert_eq!(
        table.alias_of("ZQA", stated).unwrap().underlying.as_deref(),
        Some("ZQA")
    );
    assert_eq!(table.alias_of("ZQB", stated).unwrap().underlying, None);
    assert!(table.alias_of("ZQA", day(2024, 3, 29)).is_none());
}

#[test]
fn a_table_word_names_its_table_or_nothing() {
    for table in Table::ALL {
        assert_eq!(Table::parse(table.word()), Some(table));
    }
    assert_eq!(Table::parse("no-such-table"), None);
    assert_eq!(Table::parse(""), None);
    assert_eq!(sha256_of(b"").len(), 64);
}

#[test]
fn a_write_failure_says_what_it_was() {
    assert_eq!(
        WriteError::Draft("a reason".to_owned()).to_string(),
        "draft refused: a reason"
    );
    assert_eq!(
        WriteError::Io("a reason".to_owned()).to_string(),
        "write failed: a reason"
    );
    assert!(WriteError::Raced(7).to_string().contains('7'));
}

/// A generation over the byte bound is refused by `read_capped` before a byte
/// is read; the seed passes a table that is already held.
#[test]
fn a_generation_past_the_byte_bound_is_refused_unread() {
    let store = scratch("cov-capped");
    let path = store.join("big");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_FILE_BYTES + 1).unwrap();
    let (kind, detail) = read_capped(&path).unwrap_err();
    assert_eq!(kind, RefusalKind::TooLarge);
    assert!(detail.contains("more than 8388608"), "{detail}");
    let first = seed(&store).unwrap();
    assert!(!first.is_empty());
    assert_eq!(
        seed(&store).unwrap(),
        Vec::<Written>::new(),
        "held, so nothing lands"
    );
}

#[test]
fn a_compiled_table_is_one_of_those_the_build_carries() {
    let compiled = Reference::compiled();
    assert_eq!(compiled.slots().len(), 7);
    assert!(seed_draft(Table::RiskFreeRate).is_err());
    assert!(seed_draft(Table::CorporateActions).is_err());
    assert!(seed_draft(Table::BasisRuns).is_err());
}

/// A table's directory that is a file: every door that lists it refuses by
/// name, and a seed stops there.
#[test]
fn a_table_directory_that_is_a_file_is_refused_by_every_door() {
    let store = scratch("cov-dir-file");
    let dir = Table::Calendar.dir(&store);
    std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
    std::fs::write(&dir, b"not a directory").unwrap();
    let why = newest(&store, Table::Calendar).unwrap_err();
    assert_eq!(why.kind, RefusalKind::Read);
    assert!(matches!(seed(&store), Err(WriteError::Refused(_))));
    assert!(matches!(
        write(&store, &draft(Table::Calendar, Vec::new())),
        Err(WriteError::Io(_))
    ));
    let slot = load_one(&store, Table::Calendar, CalendarTable::build);
    assert!(slot.table().is_err(), "refused, never served compiled");
}

// ───── BASE-COV-PULL repair 2 (D-7334) ─────

/// A table directory holding more names than the bound is refused by name
/// before any is read past it, at exactly the bound it is read whole, and a
/// hidden name that is not a staging file is a stray named first in name
/// order (extremes at once: the bound, one past it, a dot-name and a staging
/// name side by side).
#[test]
fn a_table_directory_past_its_bound_is_refused_and_a_hidden_name_is_a_stray() {
    let store = crate::gdfl_fixtures::scratch("ref-cov-generations");
    let table = Table::SymbolChanges;
    let dir = table.dir(&store);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(table.path(&store, 1), b"x").unwrap();
    std::fs::write(table.path(&store, 2), b"x").unwrap();
    std::fs::write(dir.join(".zq-hidden"), b"x").unwrap();
    std::fs::write(dir.join(".1.2.3.staging"), b"x").unwrap();
    let refused = newest_within(&store, table, 3).unwrap_err();
    assert_eq!(refused.kind, RefusalKind::TooMany);
    assert_eq!(refused.detail, "more than 3 names");
    let (found, strays) = newest_within(&store, table, 4).unwrap();
    assert_eq!(found, Some((2, table.path(&store, 2))));
    assert_eq!((strays.staging, strays.other), (1, 1));
    assert_eq!(strays.first_other.as_deref(), Some(".zq-hidden"));
    assert_eq!(newest(&store, table).unwrap().0, found);
}

/// A seed field is the text as the grammar carries it: trimmed, its
/// separators and line breaks turned to spaces, and an empty value written
/// `-` so a seed never fails on an empty compiled value.
#[test]
fn a_seed_field_is_trimmed_carried_or_a_dash() {
    assert_eq!(field(""), "-");
    assert_eq!(field("  \t "), "-");
    assert_eq!(field(" a|b\nc\rd "), "a b c d");
}

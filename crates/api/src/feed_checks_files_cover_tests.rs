//! Rows of `feed_checks_files.rs` no test reached before BASE-COV-API repair
//! 3 (shard AS1, D-8661): every refusal of a feed-reconcile version's text,
//! of a `day` cell, of an index's name and of a folder that cannot be listed.
//! Every row is invented: the shares are invented names, and the prices are
//! numbers no vendor printed.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a whole-file test module; fixtures are bounded and a failed assertion is the test failing"
)]

use super::*;

/// One feed row of `cli::feed_reconcile::COLUMNS`: the named cells, every
/// other cell empty.
fn row(cells: &[(&str, &str)]) -> String {
    cli::feed_reconcile::COLUMNS
        .split(',')
        .map(|column| {
            cells
                .iter()
                .find(|(name, _)| *name == column)
                .map_or("", |(_, value)| value)
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// A matched stock-day of `symbol` on `day`, with `flags` and the median
/// pair `(z, g)`.
fn matched(symbol: &str, day: &str, flags: &str, median: (&str, &str)) -> String {
    row(&[
        ("symbol", symbol),
        ("day", day),
        ("day_kind", "open_375"),
        ("status", "matched"),
        ("applied_lag", "0"),
        ("lag_source", "own"),
        ("median_z", median.0),
        ("median_g", median.1),
        ("short_judgement", "unjudged:no_env1_envelope"),
        ("ex_date", "no:close"),
        ("flags", flags),
    ])
}

/// A whole version: a comment, the column line, `rows`, then the summary
/// line when `summary`.
fn version(rows: &[String], summary: bool) -> String {
    let mut text = format!(
        "# month 2026-08; invented\n{}\n",
        cli::feed_reconcile::COLUMNS
    );
    for one in rows {
        text.push_str(one);
        text.push('\n');
    }
    if summary {
        text.push_str("# summary: feed-reconcile 2026-08: invented\n");
    }
    text
}

/// What [`parse_feed`] makes of `text`, as an August version.
fn parsed_text(text: &str) -> Result<FeedMonth, String> {
    parse_feed(
        YearMonth::new(2026, 8).expect("a month"),
        1,
        PathBuf::from("feed-reconcile-2026-08.v1.csv"),
        text,
        [0; 32],
    )
}

/// The refusal [`parse_feed`] gives `text`.
fn refused(text: &str) -> String {
    parsed_text(text).expect_err("refused")
}

/// **EVERY SHAPE A FEED-RECONCILE VERSION CAN TAKE IS READ OR REFUSED BY
/// NAME** (BASE-COV-API repair 3, D-8661). Extremes: a whole version of one
/// share-day, read; comments and no column line; a column line not this
/// build's; a row after the summary; an empty line between rows; a row one
/// cell short; a row of no symbol; sixty-five distinct flag names; a median
/// pair with one price missing; a median price of zero; no summary line; and,
/// at once, a row of no symbol after the summary (the summary's refusal comes
/// first), and a short row of no symbol (the width's refusal comes first);
/// out of the box, a version of nothing at all.
#[test]
fn every_shape_of_a_feed_version_is_read_or_refused_by_name() {
    let one = matched("FCOVA", "2026-08-03", "", ("127777", "137777"));
    let whole = parsed_text(&version(std::slice::from_ref(&one), true)).expect("read");
    assert_eq!((whole.rows.len(), whole.stocks.len()), (1, 1));
    assert_eq!(
        whole.summary.as_deref(),
        Some("feed-reconcile 2026-08: invented")
    );

    assert_eq!(refused("# a comment\n# another\n"), "it has no column line");
    assert_eq!(
        refused("symbol,day\nFCOVA,2026-08-03\n"),
        "its column line is not this build's `cli::feed_reconcile::COLUMNS`"
    );
    let after = format!("{}{one}\n", version(std::slice::from_ref(&one), true));
    assert_eq!(refused(&after), "line 5: a row after the comment block");
    let blank = version(&[one.clone(), String::new(), one.clone()], true);
    assert_eq!(refused(&blank), "line 4: a row after the comment block");
    let short = one
        .rsplit_once(',')
        .map(|(head, _)| head.to_owned())
        .expect("cells");
    let width = cli::feed_reconcile::COLUMNS.split(',').count();
    assert_eq!(
        refused(&version(&[short], true)),
        format!("line 3: {} cells, the column line names {width}", width - 1)
    );
    let nameless = matched("", "2026-08-03", "", ("127777", "137777"));
    assert_eq!(
        refused(&version(std::slice::from_ref(&nameless), true)),
        "line 3: an empty `symbol`"
    );
    let flags = (0..65)
        .map(|n| format!("flag{n}"))
        .collect::<Vec<_>>()
        .join("|");
    assert_eq!(
        refused(&version(
            &[matched("FCOVA", "2026-08-03", &flags, ("127777", "137777"))],
            true
        )),
        "line 3: more than 64 distinct flag names"
    );
    assert_eq!(
        refused(&version(
            &[matched("FCOVA", "2026-08-03", "", ("5", ""))],
            true
        )),
        "line 3: `median_z` Some(5) and `median_g` None: a factor is two paisa prices of at least 1"
    );
    assert_eq!(
        refused(&version(
            &[matched("FCOVA", "2026-08-03", "", ("0", "7"))],
            true
        )),
        "line 3: `median_z` Some(0) and `median_g` Some(7): a factor is two paisa prices of at least 1"
    );
    assert_eq!(
        refused(&version(
            &[matched("FCOVA", "2026-08-03", "", ("7", "0"))],
            true
        )),
        "line 3: `median_z` Some(7) and `median_g` Some(0): a factor is two paisa prices of at least 1"
    );
    assert_eq!(
        refused(&version(std::slice::from_ref(&one), false)),
        "it has no `# summary:` line, the line the writer prints last"
    );

    // AT ONCE: the first refusal in the reader's own order is the one said.
    let late_nameless = format!("{}{nameless}\n", version(std::slice::from_ref(&one), true));
    assert_eq!(
        refused(&late_nameless),
        "line 5: a row after the comment block"
    );
    let short_nameless = nameless
        .rsplit_once(',')
        .map(|(head, _)| head.to_owned())
        .expect("cells");
    assert!(
        refused(&version(&[short_nameless], true)).contains("cells, the column line names"),
        "the width is read before the symbol"
    );

    // OUT OF THE BOX: nothing at all.
    assert_eq!(refused(""), "it has no column line");
}

/// **A `day` CELL IS A DAY OF ITS YEAR OR IT IS REFUSED, AND AN INDEX'S NAME
/// IS ITS WRITER'S OR IT IS NOT ONE** (BASE-COV-API repair 3, D-8661).
/// Extremes: a day of the year, read; nine characters; slashes for dashes;
/// a letter in the year; another year; the thirtieth of February; the first
/// and the last day of the year; an index name whose year holds a letter;
/// one with no version digits; one whose version holds a letter; and, at
/// once, a name with both a lettered year and no digits, and a day of
/// another year written with slashes; out of the box, a day that is all
/// dashes and a held refusal that is not one of the version's.
#[test]
fn a_day_cell_and_an_index_name_are_their_writers_or_refused() {
    let wrong = |text: &str| format!("`day` holds `{text}`, not a day of 2026");
    for text in [
        "2026-8-27",
        "2026/08/27",
        "2026-08/27",
        "20x6-08-27",
        "2025-08-27",
        "2026-02-30",
        "2025/08/27",
        "----------",
    ] {
        assert_eq!(day_of_year(text, 2026), Err(wrong(text)), "{text}");
    }
    for (text, day) in [
        ("2026-08-27", (2026, 8, 27)),
        ("2026-01-01", (2026, 1, 1)),
        ("2026-12-31", (2026, 12, 31)),
    ] {
        assert_eq!(
            day_of_year(text, 2026),
            Ok(Day::new(day.0, day.1, day.2).expect("a day")),
            "{text}"
        );
    }
    assert_eq!(tick_name("tick-to-second-2026.v12.idx"), Some((2026, 12)));
    for name in [
        "tick-to-second-20x6.v1.idx",
        "tick-to-second-2026.v.idx",
        "tick-to-second-2026.v1x.idx",
        "tick-to-second-2x26.v.idx",
    ] {
        assert_eq!(tick_name(name), None, "{name}");
    }
    // A HELD REFUSAL that is not the version's own keeps its words.
    assert_eq!(
        Refusal::Absent("not there".to_owned()).held().text(),
        "not there"
    );
    assert_eq!(
        Refusal::Asked("asked badly".to_owned()).held().text(),
        "asked badly"
    );
}

/// **A FOLDER THAT CANNOT BE LISTED IS REFUSED BY NAME, AND A NAME THAT IS
/// NOT TEXT IS PASSED OVER** (BASE-COV-API repair 3, D-8661). Extremes: a
/// folder under a file, which cannot be measured; a folder that is absent,
/// read as holding nothing; a folder holding one version; that folder with a
/// file whose name is not UTF-8 beside it (Linux: a macOS volume refuses such
/// a name, so on the Mac this arm is (U)); a version and an older one of one
/// month; a name of another month's shape; two months; and, at once, a
/// non-UTF-8 name beside two versions of one month, and an absent folder
/// asked twice; out of the box, a folder that is a file of the version's own
/// name.
#[test]
fn a_folder_that_cannot_be_listed_is_refused_and_a_name_that_is_not_text_passed_over() {
    use std::os::unix::ffi::OsStrExt as _;
    let root = crate::scratch::path("feed-files-cover-listing");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let file = root.join("a-file");
    std::fs::write(&file, b"not a folder").expect("a file");
    let Err(Refusal::Refused(why)) = feed_listing(&file.join("under")) else {
        panic!("a folder under a file is refused");
    };
    assert!(why.contains("cannot be read"), "{why}");
    let absent = root.join("absent");
    for _ in 0..2 {
        let listed = feed_listing(&absent).expect("an absent folder lists nothing");
        assert!(listed.newest.is_empty());
    }
    let dir = root.join("feed");
    std::fs::create_dir_all(&dir).expect("a folder");
    for name in [
        "feed-reconcile-2026-08.v1.csv",
        "feed-reconcile-2026-08.v3.csv",
        "feed-reconcile-2026-07.v2.csv",
        "feed-reconcile-2026-8.v1.csv",
    ] {
        std::fs::write(dir.join(name), b"").expect("a version");
    }
    std::fs::write(
        dir.join(std::ffi::OsStr::from_bytes(b"feed-reconcile-\xff.v1.csv")),
        b"",
    )
    .expect("a name that is not UTF-8 (Linux)");
    let listed = feed_listing(&dir).expect("listed");
    let months: Vec<(String, u32)> = listed
        .newest
        .iter()
        .map(|(month, version)| (month.to_string(), *version))
        .collect();
    assert_eq!(
        months,
        [("2026-07".to_owned(), 2), ("2026-08".to_owned(), 3)],
        "the newest version a month, the non-UTF-8 and misshapen names passed over"
    );
    let named = root.join("feed-reconcile-2026-08.v1.csv");
    std::fs::write(&named, b"").expect("a file of a version's name");
    let Err(Refusal::Refused(why)) = feed_listing(&named) else {
        panic!("a file is not a folder");
    };
    assert!(why.contains("is not a folder"), "{why}");
    let _ = std::fs::remove_dir_all(&root);
}

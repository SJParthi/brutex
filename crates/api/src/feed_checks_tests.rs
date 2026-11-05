//! `/feed-checks.json`, `/tick-checks.json` and `/calendar-days.json` over
//! check files written here, by hand, in the writers' own formats (DATA-1w,
//! D-5190 to D-5194). Every share, price and count is invented: the symbols
//! are `Q&Q`, `ZZSYN` and `ZZONE`, and every paisa price ends in 7777, so no
//! row can be mistaken for a vendor's.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail; finite invented fixtures"
)]

use super::files::{feed_name, month_of, tick_name};
use super::*;
use cli::tick_second_check::index::{self, SecondStatus, Source, Status};
use std::fmt::Write as _;
use std::fs;
use std::time::Instant;

/// A store root of this test's own, removed when it ends.
struct Root(PathBuf);

impl Root {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(&format!("feed-checks-{name}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("audit/feed-reconcile")).expect("feed folder");
        fs::create_dir_all(root.join("audit/tick-to-second")).expect("tick folder");
        Self(root)
    }

    fn feed(&self) -> PathBuf {
        self.0.join("audit/feed-reconcile")
    }

    fn tick(&self) -> PathBuf {
        self.0.join("audit/tick-to-second")
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// One row in the writer's column order: every named cell, the rest empty.
fn cells(header: &str, named: &[(&str, &str)]) -> String {
    header
        .split(',')
        .map(|column| {
            named
                .iter()
                .find(|(name, _)| *name == column)
                .map_or("", |(_, value)| *value)
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// One feed-reconcile stock-day.
#[derive(Clone, Copy)]
struct FeedDay<'a> {
    symbol: &'a str,
    day: &'a str,
    status: &'a str,
    flags: &'a str,
    short: &'a str,
}

fn feed_row(day: &FeedDay<'_>) -> String {
    let matched = day.status == "matched";
    cells(
        cli::feed_reconcile::COLUMNS,
        &[
            ("symbol", day.symbol),
            ("day", day.day),
            ("day_kind", "open_375"),
            ("status", day.status),
            ("applied_lag", if matched { "0" } else { "" }),
            ("lag_source", if matched { "own" } else { "none" }),
            ("median_z", if matched { "127777" } else { "" }),
            ("median_g", if matched { "137777" } else { "" }),
            ("short_volume", day.short),
            ("short_judgement", "unjudged:no_env1_envelope"),
            (
                "ex_date",
                if day.flags.contains("ex_date") {
                    "yes:unsourced:close"
                } else {
                    "no:close"
                },
            ),
            ("flags", day.flags),
            ("source", "synthetic"),
        ],
    )
}

/// A whole format-3 feed-reconcile file of `month`: the format line, the
/// run's line, the columns, the rows, one carry line per share and the
/// summary line the writer prints last.
fn feed_text(month: &str, days: &[FeedDay<'_>]) -> String {
    let mut shares: Vec<&str> = Vec::new();
    for day in days {
        if !shares.contains(&day.symbol) {
            shares.push(day.symbol);
        }
    }
    let mut text = format!(
        "{}\n# month {month}; store synthetic; commit none\n{}\n",
        cli::feed_reconcile::FORMAT,
        cli::feed_reconcile::COLUMNS
    );
    for day in days {
        text.push_str(&feed_row(day));
        text.push('\n');
    }
    for share in &shares {
        writeln!(text, "# carry,{share},lags=-").unwrap();
    }
    writeln!(
        text,
        "# summary: feed-reconcile {month}: {} shares (synthetic); {} stock-days",
        shares.len(),
        days.len()
    )
    .unwrap();
    text
}

fn write_feed(root: &Root, month: &str, version: u32, days: &[FeedDay<'_>]) -> PathBuf {
    let path = root
        .feed()
        .join(format!("feed-reconcile-{month}.v{version}.csv"));
    fs::write(&path, feed_text(month, days)).expect("feed file");
    path
}

/// The three-share month every feed test reads: `Q&Q` with three days (one
/// adjusted, one ex-date, two short), `ZZSYN` with two (one GDFL only) and
/// `ZZONE` with one.
fn month_days() -> Vec<FeedDay<'static>> {
    vec![
        FeedDay {
            symbol: "Q&Q",
            day: "2025-03-03",
            status: "matched",
            flags: "",
            short: "0",
        },
        FeedDay {
            symbol: "Q&Q",
            day: "2025-03-04",
            status: "matched",
            flags: "adjusted",
            short: "77",
        },
        FeedDay {
            symbol: "Q&Q",
            day: "2025-03-05",
            status: "matched",
            flags: "adjusted|ex_date",
            short: "7",
        },
        FeedDay {
            symbol: "ZZSYN",
            day: "2025-03-03",
            status: "matched",
            flags: "zerodha_session_cut",
            short: "-7",
        },
        FeedDay {
            symbol: "ZZSYN",
            day: "2025-03-04",
            status: "gdfl_only",
            flags: "zerodha_no_minutes",
            short: "",
        },
        FeedDay {
            symbol: "ZZONE",
            day: "2025-03-31",
            status: "matched",
            flags: "",
            short: "0",
        },
    ]
}

fn feed(root: &Root, query: &str) -> Result<Value, Refusal> {
    feed_answer(&root.0, query)
}

fn asked(result: Result<Value, Refusal>) -> String {
    let Err(Refusal::Asked(why)) = result else {
        panic!("expected a 400 refusal, got {result:?}");
    };
    why
}

fn absent(result: Result<Value, Refusal>) -> String {
    let Err(Refusal::Absent(why)) = result else {
        panic!("expected a 404, got {result:?}");
    };
    why
}

fn refused(result: Result<Value, Refusal>) -> String {
    let Err(Refusal::Refused(why)) = result else {
        panic!("expected a 500 refusal, got {result:?}");
    };
    why
}

// ═══ paging and fields ═══

#[test]
fn limit_and_offset_are_required_and_every_bad_page_is_refused_by_name() {
    assert_eq!(
        page_of("limit=3&offset=0").unwrap(),
        Page {
            limit: 3,
            offset: 0
        }
    );
    assert!(asked(page_of("limit=3").map(|_| Value::Null)).contains("both required"));
    assert!(asked(page_of("offset=0").map(|_| Value::Null)).contains("both required"));
    assert!(asked(page_of("").map(|_| Value::Null)).contains("neither has a default"));
    assert!(asked(page_of("limit=0&offset=0").map(|_| Value::Null)).contains("limit=0"));
    for bad in [
        "+1",
        "01",
        "-1",
        "1e3",
        "",
        "1.0",
        "99999999999999999999",
        "%2B1",
        "%30%31",
    ] {
        let why = asked(page_of(&format!("limit={bad}&offset=0")).map(|_| Value::Null));
        assert!(why.contains("not a whole number"), "{bad}: {why}");
    }
    // `%31` decodes to `1` and is a number written as one after decoding: the
    // rule binds the decoded text, so an encoded sign or leading zero above is
    // still refused while an encoded digit is the digit.
    assert_eq!(
        page_of("limit=%31&offset=%30").unwrap(),
        Page {
            limit: 1,
            offset: 0
        }
    );
    assert_eq!(
        window(
            Page {
                limit: 2,
                offset: 1
            },
            3,
            "rows"
        )
        .unwrap(),
        (1, 3)
    );
    assert_eq!(
        window(
            Page {
                limit: 3,
                offset: 2
            },
            3,
            "rows"
        )
        .unwrap(),
        (2, 3)
    );
    let past = asked(
        window(
            Page {
                limit: 1,
                offset: 3,
            },
            3,
            "rows",
        )
        .map(|_| Value::Null),
    );
    assert!(
        past.contains("offset=3 is past the end: there are 3 rows"),
        "{past}"
    );
    let over = asked(
        window(
            Page {
                limit: 4,
                offset: 0,
            },
            3,
            "rows",
        )
        .map(|_| Value::Null),
    );
    assert!(over.contains("limit=4 is above the 3 rows"), "{over}");
    let none = absent(
        window(
            Page {
                limit: 1,
                offset: 0,
            },
            0,
            "rows",
        )
        .map(|_| Value::Null),
    );
    assert!(none.contains("there are no rows"), "{none}");
}

#[test]
fn a_field_a_route_does_not_read_is_refused_by_name() {
    let root = Root::new("fields");
    write_feed(&root, "2025-03", 1, &month_days());
    let why = asked(feed(&root, "month=2025-03&limit=1&offset=0&sort=day"));
    assert!(why.contains("does not read the field \"sort\""), "{why}");
    let why = asked(tick_answer(&root.0, "year=2025&page=1"));
    assert!(why.contains("\"page\""), "{why}");
    let why = asked(calendar_answer("year=2024&limit=1&offset=0&feed=gdfl"));
    assert!(why.contains("\"feed\""), "{why}");
}

#[test]
fn the_check_names_are_read_exactly_as_their_writers_name_them() {
    let march = month_of("2025-03").unwrap();
    assert_eq!(feed_name("feed-reconcile-2025-03.v1.csv"), Some((march, 1)));
    assert_eq!(
        feed_name("feed-reconcile-2025-03.v12.csv"),
        Some((march, 12))
    );
    for not_one in [
        "feed-reconcile-2025-03.symbols.v1.csv",
        "feed-reconcile-2025-03.v01.csv",
        "feed-reconcile-2025-03.v+1.csv",
        "feed-reconcile-2025-3.v1.csv",
        "feed-reconcile-2025-13.v1.csv",
        "feed-reconcile-2025-03.v1.csv.tmp",
        ".feed-reconcile-2025-03.tmp-1-2",
        "feed-reconcile.summary.log",
    ] {
        assert_eq!(feed_name(not_one), None, "{not_one}");
    }
    assert_eq!(tick_name("tick-to-second-2026.v001.idx"), Some((2026, 1)));
    assert_eq!(tick_name("tick-to-second-2026.v001.csv"), None);
    assert_eq!(tick_name("tick-to-second-2026.s000001.csv"), None);
    assert_eq!(tick_name("tick-to-second-26.v001.idx"), None);
    for garbage in [
        "2025-3",
        "2025-00",
        "abcd-ef",
        "2025-03x",
        "",
        "2025/03",
        "２０２５-03",
    ] {
        assert_eq!(month_of(garbage), None, "{garbage}");
    }
}

// ═══ /feed-checks.json ═══

#[test]
fn the_months_held_are_listed_with_their_counts_and_a_missing_or_garbage_month_is_named() {
    let root = Root::new("feed-months");
    // NO VERSION YET: an honest 404 naming the folder and what chose it.
    let none = absent(feed(&root, "limit=1&offset=0"));
    assert!(none.contains("feed-reconcile months"), "{none}");
    write_feed(&root, "2025-03", 1, &month_days());
    write_feed(
        &root,
        "2025-04",
        1,
        &month_days()[..1]
            .iter()
            .map(|d| FeedDay {
                day: "2025-04-01",
                ..*d
            })
            .collect::<Vec<_>>(),
    );
    let listed = feed(&root, "limit=2&offset=0").unwrap();
    assert_eq!(
        listed["page"],
        json!({"total": 2, "offset": 0, "limit": 2, "returned": 2})
    );
    assert_eq!(listed["months"][0]["month"], "2025-03");
    assert_eq!(listed["months"][0]["stocks"], 3);
    assert_eq!(listed["months"][0]["adjusted_stocks"], 1);
    assert_eq!(listed["months"][0]["ex_date_stocks"], 1);
    let counts = &listed["months"][0]["counts"];
    assert_eq!(counts["stock_days"], 6);
    assert_eq!(counts["by_status"], json!({"matched": 5, "gdfl_only": 1}));
    assert_eq!(counts["adjusted_days"], 2);
    assert_eq!(counts["ex_date_days"], 1);
    assert_eq!(counts["zerodha_minute_short_days"], 2);
    assert_eq!(counts["flags"]["zerodha_session_cut"], 1);
    assert_eq!(counts["flags"]["zerodha_no_minutes"], 1);
    assert_eq!(
        listed["dir_from"],
        "the store's audit area (BRUTEX_FEED_RECONCILE_DIR is not set)"
    );
    assert!(
        listed["months"][0]["writer_summary"]
            .as_str()
            .unwrap()
            .contains("3 shares")
    );
    assert_eq!(listed["months"][1]["month"], "2025-04");
    // The second page alone.
    let second = feed(&root, "limit=1&offset=1").unwrap();
    assert_eq!(second["months"].as_array().unwrap().len(), 1);
    assert_eq!(second["months"][0]["month"], "2025-04");
    // A month with no check yet, and garbage months.
    let missing = absent(feed(&root, "month=2025-05&limit=1&offset=0"));
    assert!(
        missing.contains("no feed-reconcile version of 2025-05"),
        "{missing}"
    );
    for garbage in ["2025-13", "2025-3", "abcd-ef", "2025-03%00", "-2025-03", ""] {
        let why = asked(feed(&root, &format!("month={garbage}&limit=1&offset=0")));
        assert!(
            why.contains("not a month written YYYY-MM"),
            "{garbage}: {why}"
        );
    }
}

#[test]
fn a_month_pages_its_shares_and_a_share_with_an_ampersand_pages_its_days() {
    let root = Root::new("feed-month");
    write_feed(&root, "2025-03", 1, &month_days());
    let month = feed(&root, "month=2025-03&limit=3&offset=0").unwrap();
    let names: Vec<&str> = month["stocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["symbol"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Q&Q", "ZZSYN", "ZZONE"]);
    assert_eq!(month["stocks"][0]["counts"]["adjusted_days"], 2);
    assert_eq!(month["stocks"][0]["first_line"], 4);
    assert_eq!(month["stocks"][0]["last_line"], 6);
    // `Q&Q` travels as `Q%26Q`; unescaped, `&Q` would be a second field.
    let days = feed(&root, "month=2025-03&symbol=Q%26Q&limit=3&offset=0").unwrap();
    assert_eq!(days["page"]["total"], 3);
    assert_eq!(days["days"][1]["day"], "2025-03-04");
    assert_eq!(
        days["days"][1]["factor"],
        json!({"zerodha_paisa": 127_777, "gdfl_paisa": 137_777})
    );
    assert_eq!(days["days"][1]["adjusted"], true);
    assert_eq!(days["days"][2]["ex_date"], "yes");
    assert_eq!(days["days"][2]["flags"], json!(["adjusted", "ex_date"]));
    assert_eq!(days["days"][1]["short_volume"], "77");
    assert_eq!(
        days["days"][0]["tradable"],
        Value::Null,
        "2025-03-03 is a regular Monday"
    );
    let unescaped = asked(feed(&root, "month=2025-03&symbol=Q&Q&limit=3&offset=0"));
    assert!(unescaped.contains("\"Q\""), "{unescaped}");
    let unknown = absent(feed(&root, "month=2025-03&symbol=NOPE&limit=1&offset=0"));
    assert!(unknown.contains("no row of `NOPE`"), "{unknown}");
    assert!(
        asked(feed(&root, "month=2025-03&symbol=Q%26Q&limit=4&offset=0")).contains("above the 3")
    );
    assert!(
        asked(feed(&root, "month=2025-03&symbol=Q%26Q&limit=1&offset=3")).contains("past the end")
    );
    assert!(asked(feed(&root, "symbol=Q%26Q&limit=1&offset=0")).contains("month=YYYY-MM"));
    assert!(
        asked(feed(&root, "month=2025-03&symbol=a%2Cb&limit=1&offset=0")).contains("share key")
    );
}

#[test]
fn every_number_leads_to_its_source_row_and_a_rewritten_row_is_refused() {
    let root = Root::new("feed-line");
    let path = write_feed(&root, "2025-03", 1, &month_days());
    let row = feed(&root, "month=2025-03&version=1&line=5").unwrap();
    assert_eq!(row["row"]["day"], "2025-03-04");
    let cells = row["source"]["cells"].as_array().unwrap();
    assert_eq!(cells[0], json!(["symbol", "Q&Q"]));
    assert_eq!(cells.len(), cli::feed_reconcile::COLUMNS.split(',').count());
    let comment = absent(feed(&root, "month=2025-03&version=1&line=2"));
    assert!(
        comment.contains("is not a row: its rows are lines 4 to 9"),
        "{comment}"
    );
    assert!(absent(feed(&root, "month=2025-03&version=1&line=10")).contains("not a row"));
    assert!(absent(feed(&root, "month=2025-03&version=2&line=5")).contains("not the newest"));
    assert!(asked(feed(&root, "month=2025-03&line=5")).contains("asked together"));
    assert!(
        asked(feed(
            &root,
            "month=2025-03&version=1&line=5&limit=1&offset=0"
        ))
        .contains("alone")
    );
    // REWRITTEN UNDER ITS NAME (the append-only law broken by hand): the
    // held row is never served as the file's.
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("127777", "127778");
    fs::write(&path, text).unwrap();
    let why = refused(feed(&root, "month=2025-03&version=1&line=5"));
    assert!(why.contains("not the line this server read"), "{why}");
}

#[test]
fn a_new_version_replaces_the_month_and_a_symbols_file_never_does() {
    let root = Root::new("feed-versions");
    write_feed(&root, "2025-03", 1, &month_days());
    assert_eq!(
        feed(&root, "month=2025-03&limit=1&offset=0").unwrap()["month"]["version"],
        1
    );
    // A `--symbols` run beside it is not the month's authority (D-5178).
    fs::write(
        root.feed().join("feed-reconcile-2025-03.symbols.v2.csv"),
        feed_text("2025-03", &month_days()[..1]),
    )
    .unwrap();
    assert_eq!(
        feed(&root, "month=2025-03&limit=1&offset=0").unwrap()["month"]["version"],
        1
    );
    // LATE ROWS: a re-run links v2; the folder's time moves and the next ask
    // reads it, with one share fewer.
    std::thread::sleep(std::time::Duration::from_millis(5));
    write_feed(&root, "2025-03", 2, &month_days()[..3]);
    let now = feed(&root, "month=2025-03&limit=1&offset=0").unwrap();
    assert_eq!(now["month"]["version"], 2);
    assert_eq!(now["page"]["total"], 1);
}

#[test]
fn a_cut_a_torn_an_empty_or_an_out_of_order_file_is_refused_by_name() {
    let root = Root::new("feed-broken");
    let path = write_feed(&root, "2025-03", 1, &month_days());
    let whole = fs::read(&path).unwrap();
    // Cut mid-row: the writer's own door refuses it (no summary line last).
    fs::write(&path, &whole[..whole.len() / 2]).unwrap();
    super::files::forget_held(&root.0);
    assert!(refused(feed(&root, "month=2025-03&limit=1&offset=0")).contains("cut or damaged"));
    // Zero bytes.
    fs::write(&path, b"").unwrap();
    super::files::forget_held(&root.0);
    let empty = refused(feed(&root, "month=2025-03&limit=1&offset=0"));
    assert!(empty.contains("2025-03"), "{empty}");
    // One share's rows in two places.
    let mut days = month_days();
    days.swap(1, 3);
    write_feed(&root, "2025-03", 2, &days);
    let split = refused(feed(&root, "month=2025-03&limit=1&offset=0"));
    assert!(split.contains("has rows in two places"), "{split}");
    // A day of another month, under this month's name.
    let mut days = month_days();
    days[5].day = "2025-04-01";
    write_feed(&root, "2025-03", 3, &days);
    assert!(
        refused(feed(&root, "month=2025-03&limit=1&offset=0")).contains("not a day of 2025-03")
    );
    // A listing page degrades loudly per month, and never drops one.
    write_feed(
        &root,
        "2025-02",
        1,
        &month_days()[..1]
            .iter()
            .map(|d| FeedDay {
                day: "2025-02-03",
                ..*d
            })
            .collect::<Vec<_>>(),
    );
    let listed = feed(&root, "limit=2&offset=0").unwrap();
    assert_eq!(listed["months"][0]["stocks"], 1);
    assert!(
        listed["months"][1]["error"]
            .as_str()
            .unwrap()
            .contains("not a day of 2025-03")
    );
}

#[test]
fn a_link_at_a_version_name_is_never_followed() {
    let root = Root::new("feed-link");
    let real = root.0.join("elsewhere.csv");
    fs::write(&real, feed_text("2025-03", &month_days())).unwrap();
    std::os::unix::fs::symlink(&real, root.feed().join("feed-reconcile-2025-03.v1.csv")).unwrap();
    let why = refused(feed(&root, "month=2025-03&limit=1&offset=0"));
    assert!(why.contains("symbolic link"), "{why}");
}

#[test]
fn a_one_row_month_and_a_whole_month_row_are_served() {
    let root = Root::new("feed-one");
    let failed = FeedDay {
        symbol: "ZZONE",
        day: "-",
        status: "read_failed:no_such_file",
        flags: "",
        short: "",
    };
    write_feed(&root, "2024-02", 1, &[failed]);
    let month = feed(&root, "month=2024-02&limit=1&offset=0").unwrap();
    assert_eq!(month["month"]["counts"]["rows"], 1);
    assert_eq!(month["month"]["counts"]["stock_days"], 0);
    assert_eq!(
        month["month"]["counts"]["by_status"],
        json!({"read_failed": 1})
    );
    let days = feed(&root, "month=2024-02&symbol=ZZONE&limit=1&offset=0").unwrap();
    assert_eq!(days["days"][0]["day"], "-");
    assert_eq!(days["days"][0]["status"], "read_failed:no_such_file");
}

// ═══ /tick-checks.json ═══

/// One tick-to-second row, and whether the index names it for its share-day.
struct TickDay<'a> {
    symbol: &'a str,
    day: &'a str,
    role: &'a str,
    status: &'a str,
    compare: &'a str,
    a: [&'a str; 3],
}

fn tick_csv(days: &[TickDay<'_>]) -> String {
    let mut text = format!("{}\n", cli::tick_second_check::HEADER);
    for day in days {
        text.push_str(&cells(
            cli::tick_second_check::HEADER,
            &[
                ("symbol", day.symbol),
                ("day", day.day),
                ("role", day.role),
                ("source", "archive_member"),
                ("status", day.status),
                ("compare", day.compare),
                ("session", "regular"),
                ("zerodha_basis", "raw_unadjusted"),
                ("t_compared", day.a[0]),
                ("t_high_match", day.a[1]),
                ("t_low_match", day.a[2]),
                ("a_compared", day.a[0]),
                ("a_high_match", day.a[1]),
                ("a_low_match", day.a[2]),
                ("r_compared", day.a[0]),
                ("r_high_match", day.a[0]),
                ("r_low_match", day.a[0]),
                ("fill_compared", "100"),
                ("fill_either_worse", "7"),
                ("store_seconds", "100"),
            ],
        ));
        text.push('\n');
    }
    text
}

/// Writes version `version` of `year`: the CSV, then its index, linked last
/// as the writer links it. Each share-day's LAST row is the one indexed.
fn write_tick(root: &Root, year: u16, version: u32, days: &[TickDay<'_>]) {
    let csv = tick_csv(days);
    let digest = brutex_core::blake3::hash(csv.as_bytes());
    let mut cells: Vec<(&str, pull::session::Day, SecondStatus)> = Vec::new();
    for (i, day) in days.iter().enumerate() {
        let parts: Vec<u8> = day
            .day
            .split('-')
            .skip(1)
            .map(|p| p.parse().unwrap())
            .collect();
        let civil = pull::session::Day::new(year, parts[0], parts[1]).unwrap();
        cells.push((
            day.symbol,
            civil,
            SecondStatus {
                status: Status::of_word(day.status.split(':').next().unwrap())
                    .unwrap_or(Status::Differs),
                source: Source::ArchiveMember,
                store_seconds: 100,
                fill_compared: 100,
                fill_worse: 7,
                row: u32::try_from(i + 1).unwrap(),
            },
        ));
    }
    let image = index::encode(year, &cells, 1, digest).expect("index image");
    fs::write(
        root.tick()
            .join(format!("tick-to-second-{year}.v{version:03}.csv")),
        csv,
    )
    .unwrap();
    fs::write(
        root.tick()
            .join(format!("tick-to-second-{year}.v{version:03}.idx")),
        image,
    )
    .unwrap();
}

/// A year in the WRITER'S order, (day, symbol, role): the order
/// `cli::tick_second_check` prints a version in.
fn year_days() -> Vec<TickDay<'static>> {
    vec![
        TickDay {
            symbol: "Q&Q",
            day: "2024-02-28",
            role: "extra",
            status: "exact_T",
            compare: "exact_T",
            a: ["10", "9", "8"],
        },
        TickDay {
            symbol: "Q&Q",
            day: "2024-02-28",
            role: "landing",
            status: "exact_T",
            compare: "exact_T",
            a: ["10", "7", "7"],
        },
        TickDay {
            symbol: "Q&Q",
            day: "2024-02-29",
            role: "landing",
            status: "differs:auction_prints_folded",
            compare: "exact_T_with_auction_prints",
            a: ["10", "10", "10"],
        },
        TickDay {
            symbol: "ZZSYN",
            day: "2024-02-29",
            role: "landing",
            status: "store_only",
            compare: "store_only",
            a: ["", "", ""],
        },
        TickDay {
            symbol: "Q&Q",
            day: "2024-03-01",
            role: "landing",
            status: "exact_T",
            compare: "exact_T",
            a: ["5", "5", "5"],
        },
    ]
}

#[test]
fn a_year_counts_one_row_per_share_day_and_pages_its_months_and_shares() {
    let root = Root::new("tick-year");
    let none = absent(tick_answer(&root.0, "limit=1&offset=0"));
    assert!(none.contains("tick-to-second years"), "{none}");
    write_tick(&root, 2024, 1, &year_days());
    let years = tick_answer(&root.0, "limit=1&offset=0").unwrap();
    assert_eq!(years["years"][0]["year"], 2024);
    let counts = &years["years"][0]["counts"];
    assert_eq!(counts["rows"], 5);
    assert_eq!(
        counts["share_days"], 4,
        "the extra row a landing row follows is not counted"
    );
    assert_eq!(counts["rule_t_exact"], json!({"share_days": 2, "of": 4}));
    assert_eq!(
        counts["minute_match"]["a"],
        json!({"compared": 25, "high_match": 22, "low_match": 22})
    );
    assert_eq!(counts["by_role"], json!({"extra": 1, "landing": 4}));
    let year = tick_answer(&root.0, "year=2024&limit=2&offset=0").unwrap();
    assert_eq!(year["months"][0]["month"], "2024-02");
    assert_eq!(year["months"][0]["shares"], 2);
    assert_eq!(year["months"][0]["counts"]["share_days"], 3);
    assert_eq!(year["months"][1]["month"], "2024-03");
    // A LEAP DAY and a MONTH BOUNDARY: 2024-02-29 is February's, 03-01 March's.
    let feb = tick_answer(&root.0, "month=2024-02&symbol=Q%26Q&limit=3&offset=0").unwrap();
    assert_eq!(feb["page"]["total"], 3);
    assert_eq!(feb["days"][0]["counted"], false);
    assert_eq!(feb["days"][1]["counted"], true);
    assert_eq!(feb["days"][2]["day"], "2024-02-29");
    let mar = tick_answer(&root.0, "month=2024-03&limit=1&offset=0").unwrap();
    assert_eq!(mar["stocks"][0]["symbol"], "Q&Q");
    assert_eq!(mar["stocks"][0]["counts"]["share_days"], 1);
    // Missing year and month, garbage, mismatch.
    assert!(
        absent(tick_answer(&root.0, "year=2023&limit=1&offset=0"))
            .contains("no tick-to-second index of 2023")
    );
    assert!(
        absent(tick_answer(&root.0, "month=2024-04&limit=1&offset=0"))
            .contains("no row of 2024-04")
    );
    for garbage in ["24", "20245", "+202", "abcd", ""] {
        assert!(
            asked(tick_answer(
                &root.0,
                &format!("year={garbage}&limit=1&offset=0")
            ))
            .contains("YYYY"),
            "{garbage}"
        );
    }
    assert!(
        asked(tick_answer(
            &root.0,
            "year=2024&month=2025-02&limit=1&offset=0"
        ))
        .contains("not a month of")
    );
    assert!(asked(tick_answer(&root.0, "symbol=Q%26Q&limit=1&offset=0")).contains("month=YYYY-MM"));
    // The source row.
    let row = tick_answer(&root.0, "year=2024&version=1&line=4").unwrap();
    assert_eq!(row["row"]["day"], "2024-02-29");
    assert_eq!(
        row["source"]["cells"][4],
        json!(["status", "differs:auction_prints_folded"])
    );
    assert!(absent(tick_answer(&root.0, "year=2024&version=1&line=1")).contains("not a row"));
}

#[test]
fn a_csv_that_is_not_its_index_s_is_refused() {
    let root = Root::new("tick-mismatch");
    write_tick(&root, 2024, 1, &year_days());
    let csv = root.tick().join("tick-to-second-2024.v001.csv");
    let mut text = fs::read_to_string(&csv).unwrap();
    let second = text.lines().nth(1).unwrap().to_owned();
    text.push_str(&second);
    text.push('\n');
    fs::write(&csv, text).unwrap();
    super::files::forget_held(&root.0);
    let why = refused(tick_answer(&root.0, "year=2024&limit=1&offset=0"));
    assert!(why.contains("is not the CSV its index"), "{why}");
    // A year listing names the refusal on its row rather than dropping it.
    let listed = tick_answer(&root.0, "limit=1&offset=0").unwrap();
    assert!(
        listed["years"][0]["error"]
            .as_str()
            .unwrap()
            .contains("not the CSV")
    );
    // A row the index does not hold: the two disagree.
    let mut days = year_days();
    days.push(TickDay {
        symbol: "ZZSYN",
        day: "2024-03-04",
        role: "landing",
        status: "exact_T",
        compare: "exact_T",
        a: ["1", "1", "1"],
    });
    let csv_text = tick_csv(&days);
    write_tick(&root, 2024, 2, &year_days());
    // Version 2's CSV holds one more row than the index it was hashed into.
    let digest = brutex_core::blake3::hash(csv_text.as_bytes());
    let image = fs::read(root.tick().join("tick-to-second-2024.v002.idx")).unwrap();
    let mut patched = image.clone();
    patched[48..80].copy_from_slice(&digest);
    fs::write(root.tick().join("tick-to-second-2024.v002.idx"), patched).unwrap();
    fs::write(root.tick().join("tick-to-second-2024.v002.csv"), csv_text).unwrap();
    let why = refused(tick_answer(&root.0, "year=2024&limit=1&offset=0"));
    assert!(why.contains("the index answers"), "{why}");
}

// ═══ /calendar-days.json ═══

#[test]
fn every_year_of_the_record_is_counted_as_the_calendar_counts_it() {
    let (first, last) = record_years();
    assert!(first > 1970 && last >= first, "{first}..{last}");
    let total = usize::from(last - first) + 1;
    let years = calendar_answer(&format!("limit={total}&offset=0")).unwrap();
    assert_eq!(years["page"]["total"], total);
    for (at, year) in years["years"].as_array().unwrap().iter().enumerate() {
        let number = first + u16::try_from(at).unwrap();
        assert_eq!(year["year"], number);
        match calendar::year_exclusions(number) {
            Ok(counted) => {
                assert_eq!(year["tradable"], counted.counts.tradable);
                assert_eq!(year["weekends"], counted.counts.count(NotTradable::Weekend));
                assert_eq!(
                    year["excluded_sessions"],
                    counted.counts.excluded_sessions()
                );
            }
            Err(why) => assert_eq!(year["refused"], why.to_string()),
        }
    }
    assert!(asked(calendar_answer(&format!("limit={}&offset=0", total + 1))).contains("above"));
}

#[test]
fn a_year_lists_its_excluded_days_with_their_notices_and_never_a_weekend() {
    let days = calendar_answer("year=2024&limit=1&offset=0").unwrap();
    let total = days["page"]["total"].as_u64().unwrap();
    let all = calendar_answer(&format!("year=2024&limit={total}&offset=0")).unwrap();
    let listed = all["days"].as_array().unwrap();
    assert!(listed.iter().all(|day| day["reason"] != "Weekend"));
    // 2024-01-20, the Saturday full session, is listed with its notice.
    let saturday = listed
        .iter()
        .find(|day| day["day"] == "2024-01-20")
        .expect("2024-01-20");
    assert_eq!(saturday["reason"], "WeekendFullSession");
    assert!(
        saturday["notice"]
            .as_str()
            .unwrap()
            .contains("NSE/MSD/60340")
    );
    assert!(
        saturday["notice_url"]
            .as_str()
            .unwrap()
            .starts_with("https://nsearchives.nseindia.com/")
    );
    // The reason filter, and its refusals.
    let drills =
        calendar_answer("year=2024&reason=DisasterRecoveryDrill&limit=2&offset=0").unwrap();
    assert_eq!(drills["page"]["total"], 2);
    assert_eq!(drills["days"][0]["day"], "2024-03-02");
    assert!(
        asked(calendar_answer("year=2024&reason=Weekend&limit=1&offset=0"))
            .contains("never listed")
    );
    assert!(
        asked(calendar_answer("year=2024&reason=Diwali&limit=1&offset=0"))
            .contains("not a listed reason")
    );
    assert!(asked(calendar_answer("reason=Holiday&limit=1&offset=0")).contains("year=YYYY"));
    let (first, last) = record_years();
    assert!(
        absent(calendar_answer(&format!(
            "year={}&limit=1&offset=0",
            last + 1
        )))
        .contains("outside")
    );
    assert!(
        absent(calendar_answer(&format!(
            "year={}&limit=1&offset=0",
            first - 1
        )))
        .contains("outside")
    );
}

// ═══ the handlers, and the cost ═══

#[tokio::test]
async fn the_handlers_answer_json_with_the_status_each_refusal_names() {
    let root = Root::new("handlers");
    write_feed(&root, "2025-03", 1, &month_days());
    let masters = root.0.join("masters");
    fs::create_dir_all(&masters).unwrap();
    let site = Loaded::new(crate::server::Site::load(&masters, &root.0));
    for (query, status) in [
        ("month=2025-03&limit=1&offset=0", 200),
        ("month=2025-03", 400),
        ("month=2025-13&limit=1&offset=0", 400),
        ("month=2025-04&limit=1&offset=0", 404),
    ] {
        let uri = format!("/feed-checks.json?{query}").parse().unwrap();
        let (code, headers, body) =
            feed_checks_json(axum::extract::State(Loaded::clone(&site)), uri).await;
        assert_eq!(code.as_u16(), status, "{query}: {body}");
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        let parsed: Value = serde_json::from_str(&body).unwrap();
        if status != 200 {
            assert_eq!(parsed["route"], "/feed-checks.json");
            assert!(!parsed["error"].as_str().unwrap().is_empty());
        }
    }
    let uri = "/tick-checks.json?year=2024&limit=1&offset=0"
        .parse()
        .unwrap();
    let (code, _, _) = tick_checks_json(axum::extract::State(Loaded::clone(&site)), uri).await;
    assert_eq!(code.as_u16(), 404);
    let uri = "/calendar-days.json?year=2024&limit=1&offset=0"
        .parse()
        .unwrap();
    let (code, _, _) = calendar_days_json(uri).await;
    assert_eq!(code.as_u16(), 200);
    let uri = "/calendar-days.json?year=2024".parse().unwrap();
    let (code, _, _) = calendar_days_json(uri).await;
    assert_eq!(code.as_u16(), 400);
}

#[test]
fn concurrent_asks_of_one_month_all_get_the_same_answer() {
    let root = Root::new("concurrent");
    write_feed(&root, "2025-03", 1, &month_days());
    let base = root.0.clone();
    let answers: Vec<Value> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let base = base.clone();
                scope.spawn(move || feed_answer(&base, "month=2025-03&limit=3&offset=0").unwrap())
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for answer in &answers {
        assert_eq!(answer["stocks"], answers[0]["stocks"]);
    }
}

/// THE COST, measured: a held month answers a page in time that does not
/// grow with the month's shares or the folder's months, and a page costs
/// O(limit). 1x / 10x / 100x shares, n = 200 asks each, after one cold ask.
#[test]
fn a_held_month_answers_in_time_that_does_not_grow_with_its_shares() {
    let root = Root::new("cost");
    let mut p50s = Vec::new();
    let sizes = [
        (10_usize, "2025-03", "2025-03-03"),
        (100, "2025-04", "2025-04-01"),
        (1_000, "2025-05", "2025-05-01"),
    ];
    for (shares, month, day) in sizes {
        let names: Vec<String> = (0..shares).map(|i| format!("ZZS{i:05}")).collect();
        let days: Vec<FeedDay<'_>> = names
            .iter()
            .map(|name| FeedDay {
                symbol: name,
                day,
                status: "matched",
                flags: "adjusted",
                short: "7",
            })
            .collect();
        write_feed(&root, month, 1, &days);
        let query = format!("month={month}&limit=10&offset=0");
        let cold = Instant::now();
        feed(&root, &query).unwrap();
        let cold = cold.elapsed();
        let mut times: Vec<u128> = (0..200)
            .map(|_| {
                let start = Instant::now();
                let answer = feed(&root, &query).unwrap();
                assert_eq!(answer["page"]["returned"], 10);
                start.elapsed().as_nanos()
            })
            .collect();
        times.sort_unstable();
        let (p50, p99, max) = (times[99], times[197], times[199]);
        println!(
            "feed month {shares} shares: cold {cold:?} p50 {p50} ns p99 {p99} ns max {max} ns n=200"
        );
        p50s.push(p50);
    }
    // A generous ceiling: the claim is flatness, and a debug build on a busy
    // machine is noisy; a page that scanned the month would grow 100x.
    assert!(p50s[2] < p50s[0] * 10 + 200_000, "{p50s:?}");
}

#[test]
fn a_refused_version_is_read_once_and_a_newer_version_is_read_afresh() {
    // D-5196. Before it, a refused version was parsed again on every ask, so
    // a page of months that named it cost O(its bytes) per request and warned
    // on /logs once per request.
    let from = crate::emitted::mark();
    let root = Root::new("held-refusal");
    let path = write_feed(&root, "2025-03", 1, &month_days());
    let whole = fs::read(&path).unwrap();
    fs::write(&path, &whole[..whole.len() / 2]).unwrap();
    let march = month_of("2025-03").unwrap();
    for _ in 0..5 {
        let why = refused(feed(&root, "month=2025-03&limit=1&offset=0"));
        assert!(
            why.contains("cut or damaged") && why.contains("held:"),
            "{why}"
        );
        let listed = feed(&root, "limit=1&offset=0").unwrap();
        let error = listed["months"][0]["error"].as_str().unwrap();
        assert!(error.contains("held:"), "{error}");
    }
    assert_eq!(super::files::feed_loads(&root.feed(), march), 1);
    // A newer version is a new name: read afresh, and served whole.
    std::thread::sleep(std::time::Duration::from_millis(5));
    write_feed(&root, "2025-03", 2, &month_days());
    let now = feed(&root, "month=2025-03&limit=1&offset=0").unwrap();
    assert_eq!(now["month"]["version"], 2);
    assert_eq!(super::files::feed_loads(&root.feed(), march), 2);
    // The same for a tick year whose CSV is not its index's.
    write_tick(&root, 2024, 1, &year_days());
    let csv = root.tick().join("tick-to-second-2024.v001.csv");
    let mut text = fs::read_to_string(&csv).unwrap();
    text.push_str("ZZEXTRA,2024-03-01\n");
    fs::write(&csv, text).unwrap();
    for _ in 0..5 {
        let why = refused(tick_answer(&root.0, "year=2024&limit=1&offset=0"));
        assert!(
            why.contains("is not the CSV its index") && why.contains("held:"),
            "{why}"
        );
    }
    assert_eq!(super::files::tick_loads(&root.tick(), 2024), 1);
    // Each refused version is ONE warning on /logs, however often it is asked
    // for (the site `feed_checks_files::note_refused`, reached from `hold`).
    let lines = crate::emitted::landed(from, "api.feed_checks", "check file refused");
    let root_text = root.0.display().to_string();
    for (check, folder, words) in [
        ("feed-reconcile", "audit/feed-reconcile", "cut or damaged"),
        (
            "tick-to-second",
            "audit/tick-to-second",
            "is not the CSV its index",
        ),
    ] {
        let mine: Vec<_> = lines
            .iter()
            .filter(|line| {
                crate::emitted::says(line, "check", check)
                    && crate::emitted::says(line, "why", &format!("{root_text}/{folder}"))
            })
            .collect();
        assert_eq!(mine.len(), 1, "{check}: {lines:?}");
        assert_eq!(mine[0].level, telemetry::Level::Warn);
        assert!(crate::emitted::says(mine[0], "why", words), "{:?}", mine[0]);
    }
}

#[test]
fn a_check_folder_that_is_a_file_is_refused_by_name_on_every_route_that_reads_it() {
    // OUT OF THE BOX: the audit area holds a FILE where the check's folder
    // belongs (a copy gone wrong). Never an empty answer, never a 404 that
    // reads as "no check yet": a 500 naming the path.
    let root = Root::new("dir-is-file");
    fs::remove_dir_all(root.feed()).unwrap();
    fs::write(root.feed(), b"not a folder").unwrap();
    fs::remove_dir_all(root.tick()).unwrap();
    fs::write(root.tick(), b"").unwrap();
    for query in ["limit=1&offset=0", "month=2025-03&limit=1&offset=0"] {
        let why = refused(feed(&root, query));
        assert!(
            why.contains("is not a folder") && why.contains("feed-reconcile"),
            "{why}"
        );
    }
    for query in ["limit=1&offset=0", "year=2024&limit=1&offset=0"] {
        let why = refused(tick_answer(&root.0, query));
        assert!(
            why.contains("is not a folder") && why.contains("tick-to-second"),
            "{why}"
        );
    }
}

#[test]
fn a_missing_month_between_two_held_months_is_named_and_its_neighbours_are_served() {
    // A month missing mid-range, with paging at both edges at once: the
    // months listed are the ones held, the gap is a 404 naming the folder,
    // and a page that ends exactly at the last month is served.
    let root = Root::new("gap");
    for (month, day) in [("2025-01", "2025-01-01"), ("2025-03", "2025-03-03")] {
        let days: Vec<FeedDay<'_>> = month_days()[..1]
            .iter()
            .map(|d| FeedDay { day, ..*d })
            .collect();
        write_feed(&root, month, 1, &days);
    }
    let listed = feed(&root, "limit=2&offset=0").unwrap();
    assert_eq!(listed["page"]["total"], 2);
    assert_eq!(listed["page"]["returned"], 2);
    let last = feed(&root, "limit=1&offset=1").unwrap();
    assert_eq!(last["page"]["returned"], 1);
    assert!(asked(feed(&root, "limit=1&offset=2")).contains('2'));
    let gap = absent(feed(&root, "month=2025-02&limit=1&offset=0"));
    assert!(
        gap.contains("2025-02") && gap.contains("feed-reconcile"),
        "{gap}"
    );
}

#[test]
fn a_check_folder_that_does_not_exist_is_named_as_missing_not_as_an_empty_check() {
    // Found on the shadow of the real store (24 Oct 2026): neither DATA-1's
    // full run nor DATA-2's `--out` had written `<store>/audit/`, and the 404
    // read "there are no feed-reconcile months in ...", the words of a check
    // that ran and found nothing. A folder that was never written is named.
    let root = Root::new("missing-folder");
    let empty = absent(feed(&root, "limit=1&offset=0"));
    assert!(
        empty.contains("there are no feed-reconcile months"),
        "{empty}"
    );
    let empty = absent(tick_answer(&root.0, "limit=1&offset=0"));
    assert!(
        empty.contains("there are no tick-to-second years"),
        "{empty}"
    );
    fs::remove_dir_all(root.feed()).unwrap();
    fs::remove_dir_all(root.tick()).unwrap();
    for (why, folder, writer) in [
        (
            absent(feed(&root, "limit=1&offset=0")),
            "feed-reconcile",
            "cli feed-reconcile",
        ),
        (
            absent(feed(&root, "month=2025-03&limit=1&offset=0")),
            "feed-reconcile",
            "cli feed-reconcile",
        ),
        (
            absent(feed(&root, "month=2025-03&symbol=Q%26Q&limit=1&offset=0")),
            "feed-reconcile",
            "cli feed-reconcile",
        ),
        (
            absent(tick_answer(&root.0, "limit=1&offset=0")),
            "tick-to-second",
            "cli tick-to-second-check",
        ),
        (
            absent(tick_answer(&root.0, "year=2024&limit=1&offset=0")),
            "tick-to-second",
            "cli tick-to-second-check",
        ),
        (
            absent(tick_answer(&root.0, "month=2024-03&limit=1&offset=0")),
            "tick-to-second",
            "cli tick-to-second-check",
        ),
    ] {
        assert!(why.contains("does not exist"), "{why}");
        assert!(why.contains(&format!("audit/{folder}")), "{why}");
        assert!(why.contains(writer), "{why}");
        assert!(!why.contains("there are no"), "{why}");
    }
    // Written later, the same server serves it: a missing folder is never held.
    fs::create_dir_all(root.feed()).unwrap();
    write_feed(&root, "2025-03", 1, &month_days());
    let listed = feed(&root, "limit=1&offset=0").unwrap();
    assert_eq!(listed["months"][0]["month"], "2025-03");
}

#[test]
fn a_page_of_months_far_from_the_first_is_the_slice_at_its_offset() {
    // EXTREME: every month of a long record, the page asked at its last row
    // and across the year boundary; the page is a slice of the listing in
    // month order, never a walk that could drift by one.
    let root = Root::new("far-page");
    let mut months = Vec::new();
    for year in [2018_u16, 2019] {
        for month in 1..=12_u8 {
            let name = format!("{year}-{month:02}");
            let day = format!("{name}-15");
            let one = FeedDay {
                symbol: "ZZONE",
                day: &day,
                status: "matched",
                flags: "",
                short: "0",
            };
            write_feed(&root, &name, 1, &[one]);
            months.push(name);
        }
    }
    let total = months.len();
    let last = feed(&root, &format!("limit=1&offset={}", total - 1)).unwrap();
    assert_eq!(last["months"][0]["month"], "2019-12");
    assert_eq!(last["page"]["total"], total);
    let across = feed(&root, "limit=2&offset=11").unwrap();
    assert_eq!(across["months"][0]["month"], "2018-12");
    assert_eq!(across["months"][1]["month"], "2019-01");
    assert_eq!(across["page"]["returned"], 2);
    let whole = feed(&root, &format!("limit={total}&offset=0")).unwrap();
    let served: Vec<&str> = whole["months"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["month"].as_str().unwrap())
        .collect();
    assert_eq!(served, months);
}

/// A tick row of the interleaving tests.
fn tick_day<'a>(symbol: &'a str, day: &'a str, role: &'a str) -> TickDay<'a> {
    TickDay {
        symbol,
        day,
        role,
        status: "exact_T",
        compare: "exact_T",
        a: ["10", "9", "9"],
    }
}

#[test]
fn a_year_in_the_writers_day_first_order_serves_each_share_month_whole() {
    // Found on the shadow of the real store (24 Oct 2026): every real
    // tick-to-second version (2019, 2025 and 2026 of rust-t2s-r6) was refused
    // "a share's rows of month 1 are in two places", because the reader took
    // the CSV to be sorted by share and the writer sorts it by (day, symbol,
    // role). Before D-5197 this year was refused at its second day.
    let root = Root::new("tick-interleaved");
    let days = vec![
        tick_day("Q&Q", "2024-03-01", "landing"),
        tick_day("ZZA", "2024-03-01", "extra"),
        tick_day("ZZA", "2024-03-01", "landing"),
        tick_day("ZZC", "2024-03-01", "landing"),
        tick_day("Q&Q", "2024-03-04", "landing"),
        tick_day("ZZC", "2024-03-04", "landing"),
        tick_day("Q&Q", "2024-03-05", "landing"),
        tick_day("ZZA", "2024-03-05", "landing"),
        // A MONTH BOUNDARY between two of a share's days.
        tick_day("ZZA", "2024-04-01", "landing"),
    ];
    write_tick(&root, 2024, 1, &days);
    let march = tick_answer(&root.0, "month=2024-03&limit=3&offset=0").unwrap();
    assert_eq!(march["month"]["shares"], 3);
    assert_eq!(march["month"]["counts"]["rows"], 8);
    // Shares in symbol byte order, each with every row it has that month.
    let shares: Vec<(&str, u64, u64, u64)> = march["stocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|stock| {
            (
                stock["symbol"].as_str().unwrap(),
                stock["counts"]["rows"].as_u64().unwrap(),
                stock["first_line"].as_u64().unwrap(),
                stock["last_line"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        shares,
        vec![("Q&Q", 3, 2, 8), ("ZZA", 3, 3, 9), ("ZZC", 2, 5, 7)]
    );
    // One share's rows: in file order, paged, its days its own.
    let zza = tick_answer(&root.0, "month=2024-03&symbol=ZZA&limit=3&offset=0").unwrap();
    let rows: Vec<(&str, &str, u64)> = zza["days"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["day"].as_str().unwrap(),
                row["role"].as_str().unwrap(),
                row["line"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            ("2024-03-01", "extra", 3),
            ("2024-03-01", "landing", 4),
            ("2024-03-05", "landing", 9)
        ]
    );
    assert_eq!(zza["days"][0]["counted"], false);
    assert_eq!(zza["days"][1]["counted"], true);
    let last = tick_answer(&root.0, "month=2024-03&symbol=Q%26Q&limit=1&offset=2").unwrap();
    assert_eq!(last["days"][0]["day"], "2024-03-05");
    assert_eq!(last["page"]["total"], 3);
    let april = tick_answer(&root.0, "month=2024-04&symbol=ZZA&limit=1&offset=0").unwrap();
    assert_eq!(april["days"][0]["day"], "2024-04-01");
    assert_eq!(april["page"]["total"], 1);
    // The source row behind a number is still the line it names.
    let row = tick_answer(&root.0, "year=2024&version=1&line=9").unwrap();
    assert_eq!(row["row"]["day"], "2024-03-05");
    assert_eq!(row["source"]["cells"][0], json!(["symbol", "ZZA"]));
}

#[test]
fn a_year_not_in_the_writers_order_is_refused_by_name() {
    // Share-first (the order this reader once assumed), a role swapped within
    // a share-day, and one key twice: none is a version the writer prints.
    for (name, days, line) in [
        (
            "share-first",
            vec![
                tick_day("Q&Q", "2024-03-01", "landing"),
                tick_day("Q&Q", "2024-03-04", "landing"),
                tick_day("ZZA", "2024-03-01", "landing"),
            ],
            4,
        ),
        (
            "role-swapped",
            vec![
                tick_day("ZZA", "2024-03-01", "landing"),
                tick_day("ZZA", "2024-03-01", "extra"),
            ],
            3,
        ),
        (
            "key-twice",
            vec![
                tick_day("ZZA", "2024-03-01", "landing"),
                tick_day("ZZA", "2024-03-01", "landing"),
            ],
            3,
        ),
    ] {
        let root = Root::new(&format!("tick-order-{name}"));
        write_tick(&root, 2024, 1, &days);
        let why = refused(tick_answer(&root.0, "year=2024&limit=1&offset=0"));
        assert!(
            why.contains(&format!("line {line}:")) && why.contains("(day, symbol, role) order"),
            "{name}: {why}"
        );
    }
}

// ═══ BASE-COV-API repair 3 (shard AS1, D-8661): rows no test reached ═══

/// **EVERY QUERY A CHECK ROUTE CANNOT ANSWER IS REFUSED BY ITS OWN RULE**
/// (BASE-COV-API repair 3, D-8661). Extremes: an empty symbol; a symbol of 65
/// characters, one past a share key's 64; a symbol holding a comma, and one
/// holding a slash; a line, then a version, asked of no month; a source line
/// asked with a symbol, and with an offset alone; a tick month asked a line,
/// then a version; a share no tick month holds, and a share the year holds in
/// another month only; a year beside a month of that year; a line, then a
/// version, with neither year nor month; a year's source line asked with a
/// limit alone, then an offset alone; and, at once, a 64-character symbol of
/// a month that holds it not (the key is read, the share is absent), and a
/// line, a version and an offset with no year or month (the year's rule
/// speaks first, not the source row's); out of the box, a symbol that is one
/// control character and nothing else.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "every query rule of both check routes, each beside the others it is read against"
)]
fn every_query_a_check_route_cannot_answer_is_refused_by_its_own_rule() {
    let root = Root::new("query-rules");
    write_feed(&root, "2025-03", 1, &month_days());
    write_tick(&root, 2024, 1, &year_days());
    // The share key's rule.
    let not_a_key = |query: &str, shown: &str| {
        let why = asked(feed(&root, query));
        assert!(
            why.contains(&format!("symbol={shown} cannot be a stored share key")),
            "{query}: {why}"
        );
    };
    not_a_key("month=2025-03&symbol=&limit=1&offset=0", "\"\"");
    let long = "Z".repeat(65);
    not_a_key(
        &format!("month=2025-03&symbol={long}&limit=1&offset=0"),
        &format!("{:?}", "Z".repeat(32)),
    );
    not_a_key("month=2025-03&symbol=Q%2CQ&limit=1&offset=0", "\"Q,Q\"");
    not_a_key("month=2025-03&symbol=Q%2FQ&limit=1&offset=0", "\"Q/Q\"");
    let control = asked(tick_answer(&root.0, "symbol=%01&limit=1&offset=0"));
    assert!(
        control.contains(r#"symbol="\u{1}" cannot be a stored share key"#),
        "{control}"
    );
    // At once: the longest key a share may have, of a month without it.
    let widest = "Z".repeat(64);
    let why = absent(feed(
        &root,
        &format!("month=2025-03&symbol={widest}&limit=1&offset=0"),
    ));
    assert_eq!(why, format!("2025-03 version 1 holds no row of `{widest}`"));
    // A part of one month, asked of no month.
    for query in ["line=5&limit=1&offset=0", "version=1&limit=1&offset=0"] {
        let why = asked(feed(&root, query));
        assert!(why.contains("ask with month=YYYY-MM"), "{query}: {why}");
    }
    // A source row is asked alone.
    for query in [
        "month=2025-03&version=1&line=5&symbol=Q%26Q",
        "month=2025-03&version=1&line=5&offset=0",
    ] {
        let why = asked(feed(&root, query));
        assert_eq!(
            why, "a source row is asked by month, version and line alone",
            "{query}"
        );
    }
    // The tick route's month part.
    for query in ["month=2024-02&line=4", "month=2024-02&version=1"] {
        let why = asked(tick_answer(&root.0, query));
        assert_eq!(
            why, "a source row is asked by year, version and line",
            "{query}"
        );
    }
    for (query, share, month) in [
        (
            "month=2024-02&symbol=ZZNONE&limit=1&offset=0",
            "ZZNONE",
            "2024-02",
        ),
        (
            "month=2024-03&symbol=ZZSYN&limit=1&offset=0",
            "ZZSYN",
            "2024-03",
        ),
    ] {
        let why = absent(tick_answer(&root.0, query));
        assert_eq!(
            why,
            format!("2024 version 1 holds no row of `{share}` in {month}"),
            "{query}"
        );
    }
    let same_year = tick_answer(&root.0, "year=2024&month=2024-02&limit=2&offset=0").unwrap();
    assert_eq!(same_year["month"]["month"], "2024-02");
    assert_eq!(same_year["month"]["shares"], 2);
    let mut shares: Vec<&str> = same_year["stocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|stock| stock["symbol"].as_str().unwrap())
        .collect();
    shares.sort_unstable();
    assert_eq!(shares, ["Q&Q", "ZZSYN"]);
    // A line or a version with neither year nor month; and, at once, both
    // with an offset: the year's rule is the one said.
    for query in [
        "line=4&limit=1&offset=0",
        "version=1&limit=1&offset=0",
        "line=4&version=1&offset=0",
    ] {
        let why = asked(tick_answer(&root.0, query));
        assert_eq!(
            why, "a source row is asked by year, version and line",
            "{query}"
        );
    }
    for query in [
        "year=2024&version=1&line=4&limit=1",
        "year=2024&version=1&line=4&offset=0",
    ] {
        let why = asked(tick_answer(&root.0, query));
        assert_eq!(
            why, "a source row is asked by year, version and line alone",
            "{query}"
        );
    }
}

/// **EVERY DAY AND YEAR THE CALENDAR HOLDS IS SAID AS IT IS, AND A REFUSED
/// VERSION IS A SERVER FAILURE** (BASE-COV-API repair 3, D-8661). Extremes: the
/// two market-wide halts of 2020, listed with evidence that is suspected; the
/// systems outage of 2021, whose evidence is sourced; a year before the record
/// read whole (its holidays owed, its counts refused by name); the first year
/// past the record, read the same way; a tick row on a Saturday of a
/// disaster-recovery drill and one on a plain Saturday, each said by its
/// reason; a tick row on a weekday, said to take trades; a version rewritten
/// under its name, served by the handler as a 500 that names its route; and,
/// at once, the halts asked one page past their end (the window's rule, not
/// the day's), and both Saturdays and a weekday in one page; out of the box,
/// year 9999, the last a year field may spell, read whole.
#[tokio::test]
async fn every_day_and_year_the_calendar_holds_is_said_and_a_refused_version_is_a_500() {
    // The halts.
    let halts = calendar_answer("year=2020&reason=MarketHalt&limit=2&offset=0").unwrap();
    let days = halts["days"].as_array().unwrap();
    assert_eq!(days.len(), 2);
    assert_eq!(days[0]["day"], "2020-03-13");
    assert_eq!(days[1]["day"], "2020-03-23");
    for day in days {
        assert_eq!(day["halt"]["suspected"], true, "{day}");
        assert!(
            day["halt"]["evidence"]
                .as_str()
                .unwrap()
                .starts_with("MEASURED from the GDFL day files"),
            "{day}"
        );
    }
    let outage = calendar_answer("year=2021&reason=SystemsOutage&limit=1&offset=0").unwrap();
    assert_eq!(outage["days"][0]["day"], "2021-02-24");
    assert_eq!(outage["days"][0]["halt"]["suspected"], false);
    assert!(
        outage["days"][0]["halt"]["evidence"]
            .as_str()
            .unwrap()
            .starts_with("SEBI Settlement Order")
    );
    let past = asked(calendar_answer(
        "year=2020&reason=MarketHalt&limit=1&offset=2",
    ));
    assert!(past.contains("offset=2 is past the end"), "{past}");
    // The proof the year route's refused-year and unspellable-year arms are
    // (U) in this build (fu-AS1): every year of the record is a civil year
    // wholly inside it, so its exclusions are counted, never refused.
    let (first, last) = record_years();
    assert!(first > 0, "the record's first day is a civil day");
    for year in first..=last {
        assert!(calendar::year_exclusions(year).is_ok(), "{year}");
    }
    // Years outside the record, read whole: their holidays are owed and their
    // counts refused, each by the calendar's own sentence.
    for year in [first - 1, last + 1, 9999] {
        let read = calendar_year_json(year);
        assert_eq!(read["year"], year);
        assert_eq!(read["holidays"]["record"], "owed", "{year}");
        assert_eq!(
            read["refused"],
            calendar::year_exclusions(year).unwrap_err().to_string(),
            "{year}"
        );
        assert!(read.get("tradable").is_none(), "{year}: {read}");
    }
    // A Saturday row is said not to take trades; the weekday beside it does.
    let root = Root::new("calendar-rows");
    let mut days = year_days();
    days.push(tick_day("Q&Q", "2024-03-02", "landing"));
    days.push(tick_day("Q&Q", "2024-03-09", "landing"));
    write_tick(&root, 2024, 1, &days);
    let march = tick_answer(&root.0, "month=2024-03&symbol=Q%26Q&limit=3&offset=0").unwrap();
    let said: Vec<(&str, &Value)> = march["days"]
        .as_array()
        .unwrap()
        .iter()
        .map(|day| (day["day"].as_str().unwrap(), &day["tradable"]))
        .collect();
    assert_eq!(
        said,
        [
            ("2024-03-01", &Value::Null),
            ("2024-03-02", &json!("DisasterRecoveryDrill")),
            ("2024-03-09", &json!("Weekend")),
        ]
    );
    // A version rewritten under its name: the handler's 500, with its route.
    let path = write_feed(&root, "2025-03", 1, &month_days());
    let masters = root.0.join("masters");
    fs::create_dir_all(&masters).unwrap();
    let site = Loaded::new(crate::server::Site::load(&masters, &root.0));
    let ask = |query: &str| {
        let uri: axum::http::Uri = format!("/feed-checks.json?{query}").parse().unwrap();
        feed_checks_json(axum::extract::State(Loaded::clone(&site)), uri)
    };
    let (code, _, _) = ask("month=2025-03&version=1&line=5").await;
    assert_eq!(code.as_u16(), 200);
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("127777", "127778");
    fs::write(&path, text).unwrap();
    let (code, headers, body) = ask("month=2025-03&version=1&line=5").await;
    assert_eq!(code.as_u16(), 500, "{body}");
    assert_eq!(headers[0].1, "application/json; charset=utf-8");
    let said: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(said["route"], "/feed-checks.json");
    assert!(
        said["error"]
            .as_str()
            .unwrap()
            .contains("not the line this server read"),
        "{said}"
    );
}

/// Names the environment child's scratch folder.
const ENV_CHILD: &str = "BRUTEX_TEST_FEED_CHECKS_ENV_CHILD";
/// Names which environment the child was given: `named`, `empty` or `missing`.
const ENV_MODE: &str = "BRUTEX_TEST_FEED_CHECKS_ENV_MODE";

/// The child's half of
/// [`a_check_folder_the_environment_names_is_the_folder_read_and_said`]: the
/// reads its `mode` names, over a store root that holds no audit folder.
fn environment_reads(root: &Path, mode: &str) {
    let nowhere = root.join("no-store-here");
    let feed_dir = root.join("audit/feed-reconcile");
    match mode {
        "named" => {
            let feed = feed_answer(&nowhere, "month=2025-03&limit=1&offset=0").unwrap();
            assert_eq!(feed["dir"], feed_dir.display().to_string());
            assert_eq!(
                feed["dir_from"],
                "the environment variable BRUTEX_FEED_RECONCILE_DIR"
            );
            assert_eq!(feed["stocks"][0]["symbol"], "Q&Q");
            let lacking = absent(feed_answer(&nowhere, "month=2025-04&limit=1&offset=0"));
            assert!(
                lacking.contains(&format!("`{}`", feed_dir.display())),
                "{lacking}"
            );
            let tick = tick_answer(&nowhere, "year=2024&limit=1&offset=0").unwrap();
            assert_eq!(
                tick["dir_from"],
                "the environment variable BRUTEX_TICK_SECOND_DIR"
            );
            assert_eq!(tick["year"]["year"], 2024);
            let lacking = absent(tick_answer(&nowhere, "year=2023&limit=1&offset=0"));
            assert!(
                lacking.contains("no tick-to-second index of 2023"),
                "{lacking}"
            );
        }
        "empty" => {
            // Set but empty is unset: the store's audit area, absent here.
            let feed = feed_answer(&nowhere, "month=2025-03&limit=1&offset=0").unwrap();
            assert_eq!(feed["stocks"][0]["symbol"], "Q&Q");
            let tick = absent(tick_answer(&nowhere, "limit=1&offset=0"));
            assert!(
                tick.starts_with(&format!(
                    "`{}` does not exist",
                    nowhere.join("audit/tick-to-second").display()
                )),
                "{tick}"
            );
        }
        _ => {
            let gone = root.join("not-made");
            let feed = absent(feed_answer(&nowhere, "limit=1&offset=0"));
            assert!(
                feed.starts_with(&format!("`{}` does not exist", gone.display())),
                "{feed}"
            );
        }
    }
}

/// **A CHECK FOLDER THE ENVIRONMENT NAMES IS THE FOLDER READ, AND IT SAYS
/// SO** (BASE-COV-API repair 3, D-8661). The two variables are read from the
/// process environment, so the reads run in children of this test binary
/// with their own (`crate::isolated`); each child proves it ran by a file its
/// parent reads. Extremes: the feed folder named by its variable; the tick
/// folder named by its own; a month held there, served; a year held there,
/// served; a store root that holds no audit folder at all, never read; a
/// month the named folder lacks, absent by that folder's name; a year it
/// lacks, the same; a variable set but empty, read as unset (the store's
/// audit area); and, at once, both variables naming one folder's two halves
/// while the store root is absent, and an empty variable beside a set one;
/// out of the box, a folder the environment names that does not exist, named
/// as missing.
#[test]
fn a_check_folder_the_environment_names_is_the_folder_read_and_said() {
    if let Some(root) = std::env::var_os(ENV_CHILD) {
        let root = PathBuf::from(root);
        let mode = std::env::var(ENV_MODE).unwrap_or_default();
        environment_reads(&root, &mode);
        fs::write(root.join("child-ran"), mode.as_bytes()).expect("the child's proof");
        return;
    }
    let root = Root::new("env-dirs");
    write_feed(&root, "2025-03", 1, &month_days());
    write_tick(&root, 2024, 1, &year_days());
    let name = "server::feed_checks::tests::a_check_folder_the_environment_names_is_the_folder_read_and_said";
    let (feed, tick) = (root.feed().into_os_string(), root.tick().into_os_string());
    let gone = root.0.join("not-made").into_os_string();
    let none = std::ffi::OsString::new();
    for (mode, feed_dir, tick_dir) in [
        ("named", &feed, &tick),
        ("empty", &feed, &none),
        ("missing", &gone, &tick),
    ] {
        let _ = fs::remove_file(root.0.join("child-ran"));
        let _ = crate::isolated::rerun(
            name,
            &[
                (ENV_CHILD, root.0.as_os_str()),
                (ENV_MODE, std::ffi::OsStr::new(mode)),
                (FEED_DIR_ENV, feed_dir.as_os_str()),
                (TICK_DIR_ENV, tick_dir.as_os_str()),
            ],
        );
        assert_eq!(
            fs::read(root.0.join("child-ran")).ok().as_deref(),
            Some(mode.as_bytes()),
            "the child read every folder of {mode}"
        );
    }
}

/// Writes version 1 of 2024 whose CSV is `text`, with an index of
/// [`year_days`] that records `text`'s BLAKE3, so the CSV is read and
/// parsed rather than refused as another version's.
fn crafted_tick(root: &Root, text: &str) {
    write_tick(root, 2024, 1, &year_days());
    let idx = root.tick().join("tick-to-second-2024.v001.idx");
    let mut image = fs::read(&idx).unwrap();
    image[48..80].copy_from_slice(&brutex_core::blake3::hash(text.as_bytes()));
    fs::write(&idx, image).unwrap();
    fs::write(root.tick().join("tick-to-second-2024.v001.csv"), text).unwrap();
}

/// **EVERY SHAPE OF A TICK YEAR'S CSV IS READ OR REFUSED BY NAME, AND A ROW
/// CUT SHORT UNDER ITS NAME IS NEVER SERVED** (BASE-COV-API repair 3,
/// D-8661). Extremes: a CSV of nothing at all; a header that is not this
/// build's; a row one cell short; a row one cell long; a row of no symbol; a
/// year whose index is gone, read directly; a year read whole and then cut to
/// its header under its name, its source row asked; the same CSV whole
/// again, its row served; and, at once, a short row of no symbol (the width's
/// rule speaks first), and a header-only CSV beside an index that records
/// exactly those bytes (read, and a year of no months is absent by name);
/// out of the box, a CSV that is one newline and nothing else.
#[test]
fn every_shape_of_a_tick_years_csv_is_read_or_refused_and_a_cut_row_is_never_served() {
    let header = cli::tick_second_check::HEADER;
    let width = header.split(',').count();
    let whole = tick_csv(&year_days());
    let second = whole.lines().nth(1).unwrap().to_owned();
    let no_symbol = second.replacen("Q&Q", "", 1);
    let short = second.rsplit_once(',').unwrap().0.to_owned();
    for (name, text, said) in [
        ("empty", String::new(), "is refused: it is empty".to_owned()),
        (
            "newline",
            "\n".to_owned(),
            "is refused: its header is not this build's `cli::tick_second_check::HEADER`"
                .to_owned(),
        ),
        (
            "header",
            format!("{}\n{second}\n", header.replacen("symbol", "share", 1)),
            "is refused: its header is not this build's `cli::tick_second_check::HEADER`"
                .to_owned(),
        ),
        (
            "short",
            format!("{header}\n{short}\n"),
            format!(
                "is refused: line 2: {} cells, the header names {width}",
                width - 1
            ),
        ),
        (
            "long",
            format!("{header}\n{second},7\n"),
            format!(
                "is refused: line 2: {} cells, the header names {width}",
                width + 1
            ),
        ),
        (
            "no-symbol",
            format!("{header}\n{no_symbol}\n"),
            "is refused: line 2: an empty `symbol`".to_owned(),
        ),
        (
            "short-no-symbol",
            format!("{header}\n{}\n", no_symbol.rsplit_once(',').unwrap().0),
            format!(
                "is refused: line 2: {} cells, the header names {width}",
                width - 1
            ),
        ),
    ] {
        let root = Root::new(&format!("tick-shape-{name}"));
        crafted_tick(&root, &text);
        let why = refused(tick_answer(&root.0, "year=2024&limit=1&offset=0"));
        assert!(why.contains(&said), "{name}: {why}");
        assert!(why.ends_with("the server restarts)"), "{name}: held: {why}");
    }
    // A header and no row, beside an index that records exactly those bytes:
    // read, and a year of no months is absent by name, never an empty page.
    let root = Root::new("tick-shape-header-only");
    crafted_tick(&root, &format!("{header}\n"));
    let none = absent(tick_answer(&root.0, "year=2024&limit=1&offset=0"));
    assert_eq!(none, "there are no months in 2024");
    // A year whose index is gone, read directly (the route asks its listing
    // first, so only a reader holding a stale listing meets this).
    let root = Root::new("tick-shape-no-index");
    let gone = absent(super::files::tick_year(&root.tick(), 2024, 1).map(|_| Value::Null));
    assert_eq!(
        gone,
        format!(
            "`{}` holds no tick-to-second index of 2024",
            root.tick().display()
        )
    );
    // Read whole, then cut to its header under its name: the held row is
    // never served from a file that no longer holds it.
    let root = Root::new("tick-shape-cut");
    write_tick(&root, 2024, 1, &year_days());
    let row = tick_answer(&root.0, "year=2024&version=1&line=4").unwrap();
    assert_eq!(row["row"]["day"], "2024-02-29");
    let csv = root.tick().join("tick-to-second-2024.v001.csv");
    fs::write(&csv, format!("{header}\n")).unwrap();
    let why = refused(tick_answer(&root.0, "year=2024&version=1&line=4"));
    assert!(
        why.starts_with(&format!("`{}` line 4: ", csv.display())),
        "{why}"
    );
    fs::write(&csv, &whole).unwrap();
    let again = tick_answer(&root.0, "year=2024&version=1&line=4").unwrap();
    assert_eq!(again["source"]["cells"][1], json!(["day", "2024-02-29"]));
}

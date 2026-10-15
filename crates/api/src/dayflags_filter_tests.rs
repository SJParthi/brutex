//! AF-02 (ingest-6): a journal row is an instrument's when its filter NAMES
//! the instrument's symbol, as a whole name of a comma list; and a refusal
//! about another underlying never marks this one refused.
//!
//! Before AF-02 a row was the instrument's only when its filter EQUALLED the
//! symbol, so NSE-FNO-NIFTY on a day closed only under `BANKNIFTY,NIFTY` or
//! the 277-name key read "no import journal line for this day", and the
//! phantom `*` row `/imports.json` made up for such a day's refusals read
//! "refused at import (vendor)" for every underlying.
//!
//! EVERY FIXTURE IS INVENTED: the filters name real underlyings because the
//! rule is about names; every refused ticker is a `ZZ` one or an invented
//! strike, and every count is a small round number.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]

use super::*;
use std::fmt::Write as _;

/// A store root of its own.
fn root_of(name: &str) -> PathBuf {
    let root = crate::scratch::path(&format!("dayflags-af02-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    root
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

/// The importer's `refused` events, as the `cli` ring holds them: `(kind,
/// day, about, why, stage)`. No run id, as the importer writes them.
fn ring(root: &Path, refusals: &[(&str, &str, &str, &str, &str)]) {
    let mut text = String::new();
    for (seq, (kind, on, about, why, stage)) in refusals.iter().enumerate() {
        writeln!(
            text,
            "{{\"seq\":{seq},\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":{},\"level\":\"error\",\
             \"target\":\"gdfl.import\",\"msg\":\"refused\",\"fields\":{{\"kind\":\"{kind}\",\
             \"day\":\"{on}\",\"about\":\"{about}\",\"why\":\"{why}\",\"stage\":\"{stage}\"}}}}",
            1_791_171_078_967_u64 + u64::try_from(seq).unwrap()
        )
        .expect("a String takes any text");
    }
    write(&root.join("logs").join("cli").join("events.ndjson"), &text);
}

/// A `begin` and closing line of `verb` under `filter`.
fn closed(verb: &str, day: &str, filter: &str, stats: &str) -> String {
    format!(
        "begin options {day} {filter}\n{verb} options {day} {filter} definition=2 files=7 \
         seconds=70 late=0 late_unresolved=0 max_back_s=0 {stats}\n"
    )
}

/// The 277-name key's shape: the names asked about among invented ones,
/// sorted and comma-joined as the importer writes them.
fn wide(with: &[&str]) -> String {
    let mut names: Vec<String> = with.iter().map(|name| (*name).to_owned()).collect();
    let mut i = 0;
    while names.len() < 277 {
        names.push(format!("ZZ{i:03}"));
        i += 1;
    }
    names.sort();
    names.join(",")
}

fn ask(query: &str) -> Asked {
    asked(query).unwrap_or_else(|why| panic!("{query}: {why}"))
}

/// The record of `on` for `instrument` over `on..=on`.
fn day_of(root: &Path, instrument: &str, on: &str) -> Value {
    let got = answer(
        root,
        &ask(&format!(
            "feed=gdfl&instrument={instrument}&from={on}&to={on}"
        )),
    );
    got["days"]
        .as_array()
        .expect("days")
        .iter()
        .find(|record| record["day"] == on)
        .cloned()
        .unwrap_or_else(|| panic!("{on} in {got:#}"))
}

fn says(record: &Value) -> Vec<String> {
    record["says"]
        .as_array()
        .expect("says")
        .iter()
        .map(|s| s.as_str().expect("a phrase").to_owned())
        .collect()
}

/// The 1-based line of the journal `text` that closes `day` under `filter`.
fn line_of(text: &str, day: &str, filter: &str) -> u64 {
    let wanted = format!(" options {day} {filter} ");
    let at = text
        .lines()
        .position(|line| !line.starts_with("begin ") && line.contains(&wanted))
        .unwrap_or_else(|| panic!("{day} {filter}"));
    u64::try_from(at).unwrap() + 1
}

/// **AN INSTRUMENT NAMED IN A MULTI-NAME FILTER READS THAT FILTER'S CLOSE**
/// (ingest-6 part 2): NIFTY, BANKNIFTY and an F&O stock on a day closed only
/// under `BANKNIFTY,NIFTY` and the 277-name key each read the newest close
/// that names them. Before AF-02 every one read `none`, "no import journal
/// line for this day" (the operator's 2026-04-15).
#[test]
fn an_instrument_named_in_a_multi_name_filter_reads_that_filters_close() {
    let root = root_of("named");
    let key = wide(&["BANKNIFTY", "NIFTY", "NIFTYNXT50", "RELIANCE"]);
    let mut journal = closed("done", "2026-04-15", "BANKNIFTY,NIFTY", "failures=0");
    journal.push_str(&closed("done", "2026-04-15", &key, "failures=0"));
    write(&pull::gdfl_import::journal_path(&root), &journal);
    let newest = line_of(&journal, "2026-04-15", &key);
    for instrument in ["NSE-FNO-NIFTY", "NSE-FNO-BANKNIFTY", "NSE-FNO-RELIANCE"] {
        let record = day_of(&root, instrument, "2026-04-15");
        assert_eq!(
            record["import"]["outcome"], "done",
            "{instrument}: {record:#}"
        );
        assert_eq!(record["import"]["line"], newest, "{instrument}");
        assert_eq!(record["import"]["from"], "journal");
        assert!(
            !says(&record)
                .iter()
                .any(|s| s == "no import journal line for this day"),
            "{instrument}: {:?}",
            says(&record)
        );
    }
    // NAMED ONLY IN THE TWO-NAME LIST, and the older close: still its own.
    let only_two = closed("done", "2026-04-17", "BANKNIFTY,NIFTY", "failures=0");
    journal.push_str(&only_two);
    write(&pull::gdfl_import::journal_path(&root), &journal);
    let record = day_of(&root, "NSE-FNO-NIFTY", "2026-04-17");
    assert_eq!(record["import"]["outcome"], "done", "{record:#}");
}

/// **A REFUSAL ABOUT ANOTHER UNDERLYING NEVER MARKS THIS ONE REFUSED**
/// (ingest-6): the operator's 2026-04-16 (ten refusals of other files under
/// the three-name close) reads NIFTY as imported with refusals that are none
/// of its own; a day the log alone names, for a BANKNIFTY file, reads NIFTY
/// as having no journal line and names the other file, while BANKNIFTY reads
/// its own refusal; a day the calendar refused is every instrument's.
#[test]
fn a_refusal_about_another_underlying_never_marks_this_one_refused() {
    let root = root_of("another");
    let mut journal = closed("done", "2026-04-16", "BANKNIFTY,NIFTY", "failures=0");
    journal.push_str(&closed(
        "done",
        "2026-04-16",
        "BANKNIFTY,NIFTY,ZZTEST",
        "failures=10 vendor_refused=10 reason=malformed_row",
    ));
    write(&pull::gdfl_import::journal_path(&root), &journal);
    let tickers: Vec<String> = (0..10).map(|i| format!("ZZTEST{i}APR2690000CE")).collect();
    let mut refusals: Vec<(&str, &str, &str, &str, &str)> = tickers
        .iter()
        .map(|ticker| {
            (
                "options",
                "2026-04-16",
                ticker.as_str(),
                "line 4: not ten well-formed fields",
                "vendor",
            )
        })
        .collect();
    refusals.push((
        "options",
        "2019-03-20",
        "BANKNIFTY28MAR1929900CE",
        "line 4: not ten well-formed fields",
        "vendor",
    ));
    refusals.push((
        "options",
        "2019-03-25",
        "calendar",
        "the calendar says OpenLengthUnmeasured, not a regular session",
        "calendar",
    ));
    ring(&root, &refusals);

    let nifty = day_of(&root, "NSE-FNO-NIFTY", "2026-04-16");
    assert_eq!(
        nifty["import"]["outcome"], "done_with_refusals",
        "{nifty:#}"
    );
    assert_eq!(nifty["import"]["from"], "journal");
    assert_eq!(nifty["import"]["refusals"]["this_count"], 0);
    assert_eq!(nifty["import"]["refusals"]["other_count"], 10);
    let said = says(&nifty);
    assert!(
        !said.iter().any(|s| s.starts_with("refused at import")),
        "{said:?}"
    );
    assert!(
        said.iter().any(|s| s
            == "imported with 10 vendor refusal(s), none about this instrument; the first is \
                ZZTEST0APR2690000CE: line 4: not ten well-formed fields"),
        "{said:?}"
    );

    // THE LOG ALONE NAMES 2019-03-20, for a BANKNIFTY contract.
    let nifty = day_of(&root, "NSE-FNO-NIFTY", "2019-03-20");
    assert_eq!(nifty["import"]["outcome"], "none", "{nifty:#}");
    assert_eq!(nifty["import"]["from"], "log");
    assert_eq!(nifty["import"]["stage"], Value::Null);
    assert_eq!(nifty["import"]["failures"], Value::Null);
    assert_eq!(nifty["import"]["refusals"]["other_count"], 1);
    let said = says(&nifty);
    assert!(
        !said.iter().any(|s| s.starts_with("refused at import")),
        "{said:?}"
    );
    assert!(
        said.iter().any(|s| s
            == "the importer's log refused 1 other file(s) of this day, none about this \
                instrument; the first is BANKNIFTY28MAR1929900CE: line 4: not ten well-formed \
                fields"),
        "{said:?}"
    );
    let bank = day_of(&root, "NSE-FNO-BANKNIFTY", "2019-03-20");
    assert_eq!(bank["import"]["outcome"], "refused", "{bank:#}");
    assert_eq!(bank["import"]["refusals"]["this_count"], 1);
    assert!(
        says(&bank)
            .iter()
            .any(|s| s.starts_with("refused at import (vendor): BANKNIFTY28MAR1929900CE")),
        "{:?}",
        says(&bank)
    );

    // THE CALENDAR'S REFUSAL is about every instrument of the kind.
    let declined = day_of(&root, "NSE-FNO-NIFTY", "2019-03-25");
    assert_eq!(declined["import"]["outcome"], "refused", "{declined:#}");
}

/// **WHOLE NAMES, AND THE RANKING**, each on a day of its own:
///
/// * a symbol that is a PREFIX of another name (`NIFTY` in
///   `NIFTYNXT50,BANKNIFTY`) is not named: no false match;
/// * a list that does not name it leaves the whole tree's `*` row in force;
/// * of a list that names it and `*`, the NEWER close wins: the list over an
///   older `*` (2026-04-08), and a newer `*` over the list (2026-04-09, the
///   AF-02 repair: an older list close never hides the whole tree's later
///   incomplete one);
/// * of a ONE-NAME filter equal to the symbol and a list with a DUPLICATE
///   name, the newer close wins.
#[test]
fn a_symbol_is_named_by_whole_names_and_its_newest_naming_close_wins() {
    let root = root_of("whole-names");
    let mut journal = closed("done", "2026-04-06", "NIFTYNXT50,BANKNIFTY", "failures=0");
    journal.push_str(&closed("incomplete", "2026-04-07", "*", "failures=1"));
    journal.push_str(&closed(
        "done",
        "2026-04-07",
        "FINNIFTY,NIFTYNXT50",
        "failures=0",
    ));
    journal.push_str(&closed("incomplete", "2026-04-08", "*", "failures=1"));
    journal.push_str(&closed(
        "done",
        "2026-04-08",
        "BANKNIFTY,NIFTY",
        "failures=0",
    ));
    journal.push_str(&closed(
        "done",
        "2026-04-09",
        "BANKNIFTY,NIFTY",
        "failures=0",
    ));
    journal.push_str(&closed("incomplete", "2026-04-09", "*", "failures=1"));
    journal.push_str(&closed("done", "2026-04-10", "NIFTY", "failures=0"));
    journal.push_str(&closed(
        "done",
        "2026-04-10",
        "NIFTY,NIFTY,ZZTEST",
        "failures=1 vendor_refused=1 reason=malformed_row",
    ));
    write(&pull::gdfl_import::journal_path(&root), &journal);

    let prefix = day_of(&root, "NSE-FNO-NIFTY", "2026-04-06");
    assert_eq!(prefix["import"]["outcome"], "none", "{prefix:#}");
    let next50 = day_of(&root, "NSE-FNO-NIFTYNXT50", "2026-04-06");
    assert_eq!(next50["import"]["outcome"], "done", "{next50:#}");

    let tree = day_of(&root, "NSE-FNO-NIFTY", "2026-04-07");
    assert_eq!(tree["import"]["outcome"], "incomplete", "{tree:#}");
    assert_eq!(tree["import"]["line"], line_of(&journal, "2026-04-07", "*"));

    let named = day_of(&root, "NSE-FNO-NIFTY", "2026-04-08");
    assert_eq!(named["import"]["outcome"], "done", "{named:#}");
    assert_eq!(
        named["import"]["line"],
        line_of(&journal, "2026-04-08", "BANKNIFTY,NIFTY")
    );
    let later_tree = day_of(&root, "NSE-FNO-NIFTY", "2026-04-09");
    assert_eq!(
        later_tree["import"]["outcome"], "incomplete",
        "{later_tree:#}"
    );
    assert_eq!(
        later_tree["import"]["line"],
        line_of(&journal, "2026-04-09", "*")
    );

    let newest = day_of(&root, "NSE-FNO-NIFTY", "2026-04-10");
    assert_eq!(
        newest["import"]["line"],
        line_of(&journal, "2026-04-10", "NIFTY,NIFTY,ZZTEST"),
        "{newest:#}"
    );
    assert_eq!(newest["import"]["outcome"], "done_with_refusals");
}

/// **A FILTER IS SPLIT ONCE PER DISTINCT STRING PER REQUEST**, never per
/// row: a page of 28 days, each closed under `*`, the two-name list and the
/// 277-name key (2,200-odd bytes), splits three filters, not 84.
#[test]
fn the_filter_is_split_once_per_distinct_string_per_request() {
    let root = root_of("memo");
    let key = wide(&["BANKNIFTY", "NIFTY"]);
    let mut journal = String::new();
    for d in 1..=28 {
        let on = format!("2026-02-{d:02}");
        journal.push_str(&closed("done", &on, "*", "failures=0"));
        journal.push_str(&closed("done", &on, "BANKNIFTY,NIFTY", "failures=0"));
        journal.push_str(&closed("done", &on, &key, "failures=0"));
    }
    write(&pull::gdfl_import::journal_path(&root), &journal);
    let before = SPLITS.with(std::cell::Cell::get);
    let got = answer(
        &root,
        &ask("feed=gdfl&instrument=NSE-FNO-NIFTY&from=2026-02-01&to=2026-02-28"),
    );
    let split = SPLITS.with(std::cell::Cell::get) - before;
    assert_eq!(split, 3, "one split per distinct filter");
    let days = got["days"].as_array().expect("days");
    assert_eq!(days.len(), 28);
    assert!(
        days.iter().all(|record| record["import"]["line"]
            == line_of(&journal, record["day"].as_str().unwrap(), &key)),
        "every day reads its newest naming close"
    );
}

/// **A CLOSE WITH REFUSALS OF THIS INSTRUMENT'S OWN NAMES THE FIRST**
/// (AF-02): the operator's 2026-04-16 for ABCAPITAL, two of the 277-name
/// close's ten refusals its own. The phantom `*` row it replaced named the
/// file; the close says how many of the refusals are this instrument's and
/// which came first, and an instrument the close names with none of its own
/// still reads "none about this instrument".
#[test]
fn a_close_with_this_instruments_own_refusals_names_the_first() {
    let root = root_of("own-refusals");
    let journal = closed(
        "done",
        "2026-04-16",
        &wide(&["NIFTY", "ZZTEST"]),
        "failures=3 vendor_refused=3 reason=malformed_row",
    );
    write(&pull::gdfl_import::journal_path(&root), &journal);
    ring(
        &root,
        &[
            (
                "options",
                "2026-04-16",
                "ZZTEST28APR2690000CE",
                "line 4: not ten well-formed fields",
                "vendor",
            ),
            (
                "options",
                "2026-04-16",
                "NIFTY28APR2690000CE",
                "line 9: not ten well-formed fields",
                "vendor",
            ),
            (
                "options",
                "2026-04-16",
                "NIFTY28APR2690000PE",
                "line 7: not ten well-formed fields",
                "vendor",
            ),
        ],
    );

    let nifty = day_of(&root, "NSE-FNO-NIFTY", "2026-04-16");
    assert_eq!(
        nifty["import"]["outcome"], "done_with_refusals",
        "{nifty:#}"
    );
    assert_eq!(nifty["import"]["refusals"]["this_count"], 2);
    let said = says(&nifty);
    assert!(
        said.iter().any(|s| s
            == "imported with 3 vendor refusal(s), 2 about this instrument; the first is \
                NIFTY28APR2690000CE: line 9: not ten well-formed fields"),
        "{said:?}"
    );
    assert!(
        !said.iter().any(|s| s.starts_with("refused at import")),
        "{said:?}"
    );

    // ONE CONTRACT sees only its own file as its own.
    let put = day_of(&root, "NSE-FNO-NIFTY-2026-04-28-9000000-PE", "2026-04-16");
    assert_eq!(put["import"]["refusals"]["this_count"], 1, "{put:#}");
    assert!(
        says(&put).iter().any(|s| s
            == "imported with 3 vendor refusal(s), 1 about this instrument; the first is \
                NIFTY28APR2690000PE: line 7: not ten well-formed fields"),
        "{:?}",
        says(&put)
    );
}

/// **A NEWER `*` CLOSE IS WHERE THE DAY STANDS, CLEAN OR FAILED** (AF-02
/// repair round 1, the second agent's blocking finding). The importer skips
/// a day by kind, day AND filter (`pull::gdfl_import::admit`), so the first
/// `*` import over the window re-imports every options day a list closed.
///
/// * (a) a day closed `done_with_refusals` under `BANKNIFTY,NIFTY` and later
///   re-imported clean under `*` reads the `*` close: `done`, its line, its
///   run, and the list run's refusal said as history, never as the day's
///   "imported with 1 vendor refusal(s)";
/// * (b) a day closed clean under the list and later left `incomplete` under
///   `*` by this instrument's own refusal reads `incomplete` and names the
///   refused file, never `done` with the refusal unsaid.
#[test]
fn a_newer_whole_tree_close_wins_over_an_older_naming_list() {
    let root = root_of("newer-tree");
    // (a) list R1 refused one BANKNIFTY file; `*` R2 began after the refusal.
    let mut journal = closed(
        "done",
        "2025-03-11",
        "BANKNIFTY,NIFTY",
        "failures=1 vendor_refused=1 reason=malformed_row run=1791171000",
    );
    journal.push_str(&closed(
        "done",
        "2025-03-11",
        "*",
        "failures=0 run=1791172000",
    ));
    // (b) list R3 clean; `*` R4 began before NIFTY's own refusal.
    journal.push_str(&closed(
        "done",
        "2026-04-15",
        "BANKNIFTY,NIFTY",
        "failures=0 run=1791160000",
    ));
    journal.push_str(&closed(
        "incomplete",
        "2026-04-15",
        "*",
        "failures=1 vendor_refused=1 reason=malformed_row run=1791171000",
    ));
    write(&pull::gdfl_import::journal_path(&root), &journal);
    ring(
        &root,
        &[
            (
                "options",
                "2025-03-11",
                "BANKNIFTY26JUN2590000CE",
                "line 3: not ten well-formed fields",
                "vendor",
            ),
            (
                "options",
                "2026-04-15",
                "NIFTY28APR2699999CE",
                "line 5: not ten well-formed fields",
                "vendor",
            ),
        ],
    );

    for instrument in ["NSE-FNO-BANKNIFTY", "NSE-FNO-NIFTY"] {
        let clean = day_of(&root, instrument, "2025-03-11");
        assert_eq!(
            clean["import"]["outcome"], "done",
            "{instrument}: {clean:#}"
        );
        assert_eq!(
            clean["import"]["line"],
            line_of(&journal, "2025-03-11", "*"),
            "{instrument}"
        );
        assert_eq!(clean["import"]["run"], 1_791_172_000_u64, "{instrument}");
        assert_eq!(clean["import"]["refusals"]["this_count"], 0, "{instrument}");
        let said = says(&clean);
        assert!(
            !said.iter().any(|s| s.starts_with("imported with")),
            "{instrument}: {said:?}"
        );
        assert!(
            said.iter().any(|s| s
                == "1 refusal(s) logged by an earlier run are history, superseded by run \
                    1791172000"),
            "{instrument}: {said:?}"
        );
    }

    let failed = day_of(&root, "NSE-FNO-NIFTY", "2026-04-15");
    assert_eq!(failed["import"]["outcome"], "incomplete", "{failed:#}");
    assert_eq!(
        failed["import"]["line"],
        line_of(&journal, "2026-04-15", "*")
    );
    assert_eq!(failed["import"]["refusals"]["this_count"], 1);
    let said = says(&failed);
    assert!(
        said.iter().any(|s| s.starts_with("incomplete at import")
            && s.contains("NIFTY28APR2699999CE: line 5: not ten well-formed fields")),
        "{said:?}"
    );
}

/// **A `done` CLOSE NEVER LEAVES THIS INSTRUMENT'S OWN REFUSALS UNSAID**
/// (AF-02 repair round 1, the second agent's follow-up). A close with no run
/// id (every journal line before run ids) cannot place a logged refusal
/// before or after itself, so a refusal of the day about this instrument
/// stays the day's: a `done` close says it, as a `done_with_refusals` one
/// does, and never reads as a clean day with the refusal in the JSON only.
#[test]
fn a_done_close_with_this_instruments_own_logged_refusal_says_it() {
    let root = root_of("done-own");
    let mut journal = closed(
        "done",
        "2025-03-12",
        "BANKNIFTY,NIFTY",
        "failures=1 vendor_refused=1 reason=malformed_row",
    );
    journal.push_str(&closed("done", "2025-03-12", "*", "failures=0"));
    write(&pull::gdfl_import::journal_path(&root), &journal);
    ring(
        &root,
        &[(
            "options",
            "2025-03-12",
            "NIFTY27MAR2599999PE",
            "line 2: not ten well-formed fields",
            "vendor",
        )],
    );

    let nifty = day_of(&root, "NSE-FNO-NIFTY", "2025-03-12");
    assert_eq!(nifty["import"]["outcome"], "done", "{nifty:#}");
    assert_eq!(
        nifty["import"]["line"],
        line_of(&journal, "2025-03-12", "*")
    );
    assert_eq!(nifty["import"]["refusals"]["this_count"], 1);
    let said = says(&nifty);
    assert!(
        said.iter().any(|s| s
            == "closed done, yet the importer's log holds 1 refusal(s) of this day about \
                this instrument not dated before this run; the first is \
                NIFTY27MAR2599999PE: line 2: not ten well-formed fields"),
        "{said:?}"
    );

    // ANOTHER UNDERLYING'S DAY: nothing of its own, nothing said.
    let bank = day_of(&root, "NSE-FNO-BANKNIFTY", "2025-03-12");
    assert_eq!(bank["import"]["refusals"]["this_count"], 0, "{bank:#}");
    assert!(
        !says(&bank)
            .iter()
            .any(|s| s.starts_with("closed done, yet")),
        "{:?}",
        says(&bank)
    );
}

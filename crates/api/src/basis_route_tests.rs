//! `/basis.json`, the corporate-action note on `/dayflags.json` (what `/db`
//! draws its day band from) and the measured run on `/sources/day.json`
//! (R4-26). Every symbol, day and ratio is INVENTED; no vendor row is
//! copied.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use pull::corporate_actions::{Action, Kind, Ratio, Run, RunEnd, Tag};
use pull::reference_data::{Draft, Standing, Table, write};
use pull::session::Day;
use serde_json::Value;

use super::{answer, asked, tables};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A fresh store root of this test's own (gate 23 clause C: the process id
/// and this file's prefix are in its name).
fn root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-api-basis-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn r(text: &str) -> Ratio {
    Ratio::parse(text).unwrap()
}

/// SPLITCO's invented 2:1 split on 2025-03-06: 1/2 before, 1/1 from it.
fn runs() -> Vec<Run> {
    let run = |first, last, ratio: &str, end| Run {
        symbol: "SPLITCO".to_owned(),
        first,
        last,
        ratio: r(ratio),
        low: r(ratio).times(r("1999/2000")).unwrap(),
        high: r(ratio).times(r("2001/2000")).unwrap(),
        days: 3,
        end,
    };
    vec![
        run(day(2025, 3, 3), day(2025, 3, 5), "1/2", RunEnd::Step),
        run(day(2025, 3, 6), day(2025, 3, 10), "1", RunEnd::Open),
    ]
}

fn measured_split() -> Action {
    Action {
        symbol: "SPLITCO".to_owned(),
        isin: None,
        ex_date: day(2025, 3, 6),
        kind: Kind::Unclassed,
        ratio: r("2"),
        tag: Tag::Measured,
        after: Some(day(2025, 3, 5)),
        evidence: "basis_runs generation 1: ratio 1/2 to 1/1".to_owned(),
    }
}

fn install(store: &Path, actions: &[Action]) {
    write(
        store,
        &Draft {
            table: Table::BasisRuns,
            source: "invented basis runs".to_owned(),
            source_sha256: None,
            fetched: day(2026, 10, 22),
            standing: Standing::Measured,
            decision: "D-6892".to_owned(),
            rows: runs().iter().map(Run::fields).collect(),
        },
    )
    .unwrap();
    write(
        store,
        &Draft {
            table: Table::CorporateActions,
            source: "invented ledger".to_owned(),
            source_sha256: None,
            fetched: day(2026, 10, 22),
            standing: Standing::Measured,
            decision: "D-6892".to_owned(),
            rows: actions.iter().map(Action::fields).collect(),
        },
    )
    .unwrap();
}

fn notes_of(record: &Value) -> Vec<Value> {
    record
        .get("notes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

// ─────────────────────────────── /dayflags.json, as /db reads it ───────────────────────────────

#[test]
fn db_shows_the_action_on_its_day() {
    let store = root("db-day");
    install(&store, &[measured_split()]);
    for order in ["asc", "desc"] {
        let ask = super::super::dayflags::asked(&format!(
            "feed=zerodha&instrument=NSE-CASH-SPLITCO&from=2025-03-03&to=2025-03-10&order={order}"
        ))
        .unwrap();
        let body = super::super::dayflags::answer(&store, &ask);
        let days = body["days"].as_array().unwrap();
        let mut noted = Vec::new();
        for record in days {
            for note in notes_of(record) {
                if note["note"] == "corporate_action" {
                    noted.push((record["day"].as_str().unwrap().to_owned(), note));
                }
            }
        }
        assert_eq!(
            noted.len(),
            1,
            "{order}: ONE note, on the ex-date: {noted:#?}"
        );
        let (on, note) = &noted[0];
        assert_eq!(on, "2025-03-06");
        assert_eq!(note["ratio"], "2/1");
        assert_eq!(note["tag"], "measured");
        assert_eq!(note["kind"], "unclassed");
        // A MEASURED ROW LABELS: it excuses nothing and decides no class.
        assert_eq!(note["excuses"], false);
        assert_eq!(note["decides_class"], false);
        assert_eq!(note["label_only"], true);
        assert_eq!(note["applies"], "at this session");
        assert_eq!(
            body["corporate_actions"]["served"], true,
            "{:#}",
            body["corporate_actions"]
        );
        assert_eq!(body["corporate_actions"]["receipt"]["status"], "measured");
    }
    // An index page carries no share action and no table.
    let ask = super::super::dayflags::asked(
        "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2025-03-03&to=2025-03-10",
    )
    .unwrap();
    let body = super::super::dayflags::answer(&store, &ask);
    assert!(body["corporate_actions"].is_null());
}

#[test]
fn a_holiday_ex_date_is_noted_on_its_day_as_applying_at_the_next_session() {
    let store = root("db-holiday");
    let mut saturday = measured_split();
    // 2025-03-08 is a Saturday: the exchange held no session.
    saturday.ex_date = day(2025, 3, 8);
    saturday.after = None;
    saturday.tag = Tag::Measured;
    install(&store, &[saturday]);
    let ask = super::super::dayflags::asked(
        "feed=zerodha&instrument=NSE-CASH-SPLITCO&from=2025-03-08&to=2025-03-08",
    )
    .unwrap();
    let body = super::super::dayflags::answer(&store, &ask);
    let notes = notes_of(&body["days"][0]);
    let note = notes
        .iter()
        .find(|n| n["note"] == "corporate_action")
        .unwrap();
    assert!(
        note["applies"]
            .as_str()
            .unwrap()
            .starts_with("at the next session"),
        "{note:#}"
    );
}

#[test]
fn an_absent_or_refused_table_is_named_on_the_page_never_an_empty_one() {
    let store = root("db-absent");
    let ask = super::super::dayflags::asked(
        "feed=zerodha&instrument=NSE-CASH-SPLITCO&from=2025-03-03&to=2025-03-10",
    )
    .unwrap();
    let body = super::super::dayflags::answer(&store, &ask);
    assert_eq!(body["corporate_actions"]["served"], false);
    assert_eq!(body["corporate_actions"]["refusal"]["kind"], "absent");
    // A table whose bytes are not the bytes written: refused by name.
    install(&store, &[measured_split()]);
    let path = Table::CorporateActions.path(&store, 1);
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("|2|1|", "|3|1|");
    std::fs::write(&path, text).unwrap();
    let body = super::super::dayflags::answer(&store, &ask);
    assert_eq!(
        body["corporate_actions"]["refusal"]["kind"], "checksum",
        "{:#}",
        body["corporate_actions"]
    );
    assert!(
        body["days"]
            .as_array()
            .unwrap()
            .iter()
            .all(|record| notes_of(record)
                .iter()
                .all(|n| n["note"] != "corporate_action")),
        "nothing is drawn from a refused table"
    );
}

#[test]
fn a_two_for_one_step_across_a_missing_month_is_never_at_this_session() {
    // The last day seen on the old basis is 2025-01-31 and the first on the
    // new 2025-03-06: February was missing from the input. The row sits on
    // the first day the new basis was seen, and must not claim that day.
    let store = root("db-gap");
    let mut gap = measured_split();
    gap.after = Some(day(2025, 1, 31));
    install(&store, &[gap]);
    let ask = super::super::dayflags::asked(
        "feed=zerodha&instrument=NSE-CASH-SPLITCO&from=2025-03-03&to=2025-03-10",
    )
    .unwrap();
    let body = super::super::dayflags::answer(&store, &ask);
    let mut noted = Vec::new();
    for record in body["days"].as_array().unwrap() {
        for note in notes_of(record) {
            if note["note"] == "corporate_action" {
                noted.push((record["day"].as_str().unwrap().to_owned(), note));
            }
        }
    }
    assert_eq!(noted.len(), 1, "{noted:#?}");
    let (on, note) = &noted[0];
    assert_eq!(on, "2025-03-06");
    assert_ne!(note["applies"], "at this session", "{note:#}");
    assert!(
        note["applies"]
            .as_str()
            .unwrap()
            .starts_with("on one session between 2025-01-31 (excluded) and 2025-03-06 (included)"),
        "{note:#}"
    );
    assert_eq!(note["day_known"], false);
    // /basis.json says the same of the row.
    let body = answer(&tables(&store), &asked("symbol=SPLITCO").unwrap());
    assert_eq!(body["actions"][0]["day_known"], false, "{body:#}");
    // The adjacent-day split of the other tests knows its day.
    let known = root("db-gap-known");
    install(&known, &[measured_split()]);
    let body = answer(&tables(&known), &asked("symbol=SPLITCO").unwrap());
    assert_eq!(body["actions"][0]["day_known"], true, "{body:#}");
}

#[test]
fn a_notes_value_that_is_not_an_array_keeps_its_shape() {
    let store = root("db-shape");
    install(&store, &[measured_split()]);
    let on = [day(2025, 3, 6)];
    // Something other than an array: left exactly as it was, the action
    // beside it.
    let mut records = vec![serde_json::json!({ "day": "2025-03-06", "notes": "kept as text" })];
    let said = super::super::dayflags::corporate_action_notes(&store, "SPLITCO", &on, &mut records);
    assert_eq!(said["served"], true, "{said:#}");
    assert_eq!(records[0]["notes"], "kept as text");
    assert_eq!(
        records[0]["corporate_action_notes"][0]["note"],
        "corporate_action"
    );
    // An array: appended to, its own notes kept first.
    let mut records = vec![serde_json::json!({ "notes": [{ "note": "other" }] })];
    super::super::dayflags::corporate_action_notes(&store, "SPLITCO", &on, &mut records);
    let notes = records[0]["notes"].as_array().unwrap();
    assert_eq!(notes.len(), 2, "{notes:#?}");
    assert_eq!(notes[0]["note"], "other");
    assert_eq!(notes[1]["note"], "corporate_action");
    assert!(records[0].get("corporate_action_notes").is_none());
}

// ─────────────────────────────── /basis.json ───────────────────────────────

#[test]
fn basis_json_answers_the_run_holding_a_day_and_names_every_outside() {
    let store = root("basis-json");
    install(&store, &[measured_split()]);
    let kept = tables(&store);
    let inside = answer(&kept, &asked("symbol=SPLITCO&day=2025-03-04").unwrap());
    // D-3339's direction, GDFL ÷ Zerodha, as `/sources/day.json` says it:
    // before the 2:1 split the factor is the action's own 2/1, and the
    // run's Zerodha/GDFL ratio rides beside it by name (check 4: this said
    // 1/2 under the same word `/sources/day.json` used for 2/1).
    assert_eq!(inside["day"]["factor"], "2/1");
    assert_eq!(inside["day"]["zerodha_over_gdfl"], "1/2");
    assert_eq!(inside["day"]["run"]["end"], "step");
    assert!(inside["day"]["outside"].is_null());
    let on = answer(&kept, &asked("symbol=SPLITCO&day=2025-03-06").unwrap());
    assert_eq!(on["day"]["factor"], "1/1");
    assert_eq!(on["day"]["zerodha_over_gdfl"], "1/1");
    assert_eq!(on["day"]["actions"][0]["ratio"], "2/1");
    assert_eq!(on["day"]["actions"][0]["excuses"], false);
    let after = answer(&kept, &asked("symbol=SPLITCO&day=2025-03-11").unwrap());
    assert!(after["day"]["factor"].is_null());
    assert!(after["day"]["zerodha_over_gdfl"].is_null());
    assert!(
        after["day"]["outside"]
            .as_str()
            .unwrap()
            .starts_with("outside_every_run")
    );
    let none = answer(&kept, &asked("symbol=NOSUCHCO&day=2025-03-04").unwrap());
    assert!(
        none["day"]["outside"]
            .as_str()
            .unwrap()
            .starts_with("no_basis_run")
    );
    let all = answer(&kept, &asked("symbol=SPLITCO").unwrap());
    assert_eq!(all["runs"].as_array().unwrap().len(), 2);
    assert_eq!(all["actions"].as_array().unwrap().len(), 1);
    assert_eq!(all["tables"]["basis_runs"]["receipt"]["status"], "measured");
    assert!(all["law"].as_str().unwrap().contains("never excuses"));
    // Absent tables are named, never served empty.
    let empty = answer(
        &tables(&root("basis-empty")),
        &asked("symbol=SPLITCO&day=2025-03-04").unwrap(),
    );
    assert_eq!(empty["tables"]["basis_runs"]["refusal"]["kind"], "absent");
    assert_eq!(empty["day"]["outside"], "the basis-run table is not served");
    assert!(empty["day"]["actions"].is_null());
}

#[test]
fn garbage_parameters_are_refused_by_name() {
    for (query, said) in [
        ("", "symbol is required"),
        ("symbol=", "symbol is required"),
        ("symbol=splitco", "not a stored share name"),
        ("symbol=A%20B", "not a stored share name"),
        (
            &*format!("symbol={}", "A".repeat(65)),
            "not a stored share name",
        ),
        ("symbol=SPLITCO&day=2025-02-30", "not YYYY-MM-DD"),
        ("symbol=SPLITCO&day=yesterday", "not YYYY-MM-DD"),
        ("symbol=SPLITCO&symbol=OTHERCO", "given twice"),
        ("symbol=SPLITCO&limit=5", "not a field"),
    ] {
        let why = asked(query).unwrap_err();
        assert!(why.contains(said), "{query:?}: {why}");
    }
    assert!(
        asked("symbol=M%26M&day=2025-03-04").is_ok(),
        "an ampersand name is a share"
    );
}

#[test]
fn the_tables_are_read_again_only_when_a_generation_lands() {
    let store = root("basis-kept");
    install(&store, &[measured_split()]);
    let first = tables(&store);
    let again = tables(&store);
    assert!(
        Arc::ptr_eq(&first.runs, &again.runs),
        "kept: no second read"
    );
    assert!(Arc::ptr_eq(&first.actions, &again.actions));
    // A new generation lands: the next request reads it.
    let mut second = measured_split();
    second.evidence = "basis_runs generation 2".to_owned();
    write(
        &store,
        &Draft {
            table: Table::CorporateActions,
            source: "invented ledger, generation 2".to_owned(),
            source_sha256: None,
            fetched: day(2026, 10, 22),
            standing: Standing::Measured,
            decision: "D-6892".to_owned(),
            rows: vec![second.fields()],
        },
    )
    .unwrap();
    let after = tables(&store);
    assert!(!Arc::ptr_eq(&first.actions, &after.actions));
    assert_eq!(after.actions.receipt().unwrap().generation, 2);
}

/// The second generation of `measured_split`'s ledger, written to `store`.
fn second_generation(store: &Path) {
    let mut second = measured_split();
    second.evidence = "basis_runs generation 2".to_owned();
    write(
        store,
        &Draft {
            table: Table::CorporateActions,
            source: "invented ledger, generation 2".to_owned(),
            source_sha256: None,
            fetched: day(2026, 10, 22),
            standing: Standing::Measured,
            decision: "D-6892".to_owned(),
            rows: vec![second.fields()],
        },
    )
    .unwrap();
}

#[test]
fn a_generation_landing_inside_a_linked_table_directory_is_read() {
    // ATTACK E7 (check 4): each table directory is a symbolic link to a
    // directory elsewhere. A generation lands in the directory the link
    // names and never changes the link, so a stamp of the link alone kept
    // the first generation for ever.
    let store = root("basis-link");
    let elsewhere = root("basis-link-target");
    install(&elsewhere, &[measured_split()]);
    for table in [Table::CorporateActions, Table::BasisRuns] {
        let named = table.dir(&store);
        std::fs::create_dir_all(named.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(table.dir(&elsewhere), &named).unwrap();
    }
    let first = tables(&store);
    assert_eq!(first.actions.receipt().unwrap().generation, 1);
    assert!(Arc::ptr_eq(&first.actions, &tables(&store).actions), "kept");
    second_generation(&elsewhere);
    let after = tables(&store);
    assert_eq!(
        after.actions.receipt().map(|held| held.generation),
        Some(2),
        "the generation that landed behind the link is served"
    );
}

#[test]
fn a_table_refused_as_unreadable_is_read_again_once_its_mode_is_repaired() {
    // ATTACK E16 (check 4): a `chmod` changes a file's ctime and nothing
    // else the stamp read, so a table refused while unreadable stayed
    // refused after the mode was repaired.
    use std::os::unix::fs::PermissionsExt as _;
    let store = root("basis-mode");
    install(&store, &[measured_split()]);
    let served = Table::CorporateActions.path(&store, 1);
    let mode = |bits: u32| {
        std::fs::set_permissions(&served, std::fs::Permissions::from_mode(bits)).unwrap();
    };
    mode(0o000);
    assert!(
        std::fs::read(&served).is_err(),
        "the test needs a reader the mode refuses (not root)"
    );
    let refused = tables(&store);
    assert!(refused.actions.table().is_err(), "unreadable: refused");
    mode(0o644);
    let repaired = tables(&store);
    assert_eq!(
        repaired.actions.receipt().map(|held| held.generation),
        Some(1),
        "the repaired table is served: {:?}",
        repaired.actions.table().err()
    );
}

#[test]
fn a_well_formed_day_the_calendar_does_not_hold_is_named_as_such() {
    // Check 4's follow-up: `day=1969-12-31` was answered "is not
    // YYYY-MM-DD", which it is.
    for text in ["1969-12-31", "0000-01-01", "2025-02-30", "2025-13-01"] {
        let why = asked(&format!("symbol=SPLITCO&day={text}")).unwrap_err();
        assert_eq!(
            why,
            format!(
                "day {text:?} is no calendar day: the calendar holds 1970-01-01 to 9999-12-31"
            )
        );
    }
    for text in ["2025-3-04", "2025/03/04", "20250304xx", "abcd-ef-gh", "2025-03-0a"] {
        let why = asked(&format!("symbol=SPLITCO&day={text}")).unwrap_err();
        assert_eq!(why, format!("day {text:?} is not YYYY-MM-DD"));
    }
    assert!(asked("symbol=SPLITCO&day=1970-01-01").is_ok());
}

#[test]
fn a_lookup_costs_the_same_at_1x_10x_and_100x_runs_measured() {
    // The kept answer for one (share, day): four stats, one hash, one
    // bounded binary search. p50, p99 and max at 1x, 10x and 100x shares.
    let mut medians = Vec::new();
    for shares in [10_u32, 100, 1_000] {
        let store = root("basis-scale");
        let mut rows = Vec::new();
        for s in 0..shares {
            for k in 0..8_u32 {
                let first = Day::from_days(day(2018, 1, 1).days_from_epoch() + k * 200).unwrap();
                let last = Day::from_days(first.days_from_epoch() + 150).unwrap();
                let ratio = if k % 2 == 0 { "1/2" } else { "1" };
                rows.push(
                    Run {
                        symbol: format!("SHARE{s:05}"),
                        first,
                        last,
                        ratio: r(ratio),
                        low: r(ratio),
                        high: r(ratio),
                        days: 100,
                        end: RunEnd::Drift,
                    }
                    .fields(),
                );
            }
        }
        write(
            &store,
            &Draft {
                table: Table::BasisRuns,
                source: "invented".to_owned(),
                source_sha256: None,
                fetched: day(2026, 10, 22),
                standing: Standing::Measured,
                decision: "D-6892".to_owned(),
                rows,
            },
        )
        .unwrap();
        let probe = asked(&format!("symbol=SHARE{:05}&day=2019-06-03", shares / 2)).unwrap();
        let _warm = answer(&tables(&store), &probe);
        let mut took: Vec<u128> = (0..400)
            .map(|_| {
                let started = std::time::Instant::now();
                std::hint::black_box(answer(&tables(&store), &probe));
                started.elapsed().as_nanos()
            })
            .collect();
        took.sort_unstable();
        let (p50, p99, max) = (took[200], took[396], took[399]);
        println!(
            "basis lookup at {shares} shares x 8 runs: p50 {p50} ns, p99 {p99} ns, max {max} ns, n 400"
        );
        medians.push(p50);
    }
    assert!(
        medians[2] <= medians[0].max(1_000) * 4,
        "a kept lookup's median at 1x, 10x, 100x: {medians:?} ns"
    );
}

// ─────────────────────────────── /sources/day.json ───────────────────────────────

#[test]
fn a_compare_inside_a_run_applies_its_factor() {
    let store = root("sources-run");
    install(&store, &[measured_split()]);
    for (on, factor, actions) in [
        ("2025-03-04", Some("2/1"), 0),
        ("2025-03-06", Some("1/1"), 1),
        ("2025-03-11", None, 0),
    ] {
        let asked = super::super::sources::DayAsk::parse(&format!(
            "exchange=NSE&segment=CASH&symbol=SPLITCO&from={on}"
        ))
        .unwrap();
        let body = super::super::sources::day_table(&store, &asked).unwrap();
        let run = &body["days"][0]["basis"]["measured_run"];
        // GDFL ÷ Zerodha, D-3339's direction: 2/1 inside the 1/2 run.
        assert_eq!(run["factor"].as_str(), factor, "{on}: {run:#}");
        // ONE WORD, ONE DIRECTION: `/basis.json` says the same factor and
        // the same named ratio for the day (check 4 of R4-26).
        let basis = answer(
            &tables(&store),
            &super::asked(&format!("symbol=SPLITCO&day={on}")).unwrap(),
        );
        assert_eq!(basis["day"]["factor"], run["factor"], "{on}");
        assert_eq!(
            basis["day"]["zerodha_over_gdfl"], run["zerodha_over_gdfl"],
            "{on}"
        );
        assert_eq!(
            run["corporate_actions"].as_array().unwrap().len(),
            actions,
            "{on}"
        );
        if factor.is_none() {
            assert!(
                run["outside"]
                    .as_str()
                    .unwrap()
                    .starts_with("outside_every_run"),
                "{run:#}"
            );
        } else {
            assert_eq!(run["standing"], "measured");
        }
        assert_eq!(body["measured_basis"]["served"], true);
    }
}

#[test]
fn a_basis_step_is_one_threshold_on_sources_and_in_the_runs() {
    // ONE RULE, TWO SPELLINGS, PINNED: `/sources` judges `basis_step` by
    // D-3339's threshold, and `cli basis-runs` folds its steps by the same.
    assert_eq!(
        super::super::sources::FACTOR_THRESHOLD_PERMILLE,
        i128::from(pull::corporate_actions::STEP_PERMILLE)
    );
}

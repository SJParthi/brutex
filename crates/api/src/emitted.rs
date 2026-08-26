//! The one event sink this test binary installs, and the proof that every
//! reachable `telemetry::emit` site in this crate reaches a file.
//!
//! # What was unproven
//!
//! This crate holds twenty-six production `telemetry::emit` call sites. Exactly
//! one of them — `api.main`, in `crates/api/src/main.rs`, which is its own test
//! binary — had a test that drove the production call and then read the record
//! back off disk. The other twenty-five all live in the LIB target and were
//! executed by tests that asserted nothing about the log, because
//! [`telemetry::emit`] answers [`telemetry::Emitted::NotInstalled`] when no sink
//! is installed and every site here discards its return under the name
//! `_dropped_when_filtered`. A site could therefore be reached by a hundred
//! tests, write nothing, and no assertion anywhere would move — which is
//! `CLAUDE.md` §4's "a test that asserts nothing" wearing a coverage report as
//! a disguise.
//!
//! Two mutants make the gap concrete and are what this module kills. **Delete
//! the `emit` call**: nothing fails, because nothing reads the file. **Flip a
//! level ternary** — `api.census`, `api.merge`, `api.assets` and `pull.run`
//! each choose their level from a condition — and nothing fails either, because
//! the level only decides whether the line survives the floor, and no test had
//! a floor.
//!
//! # Why there is exactly one sink, and why it lives here
//!
//! [`telemetry::install`] refuses a second call by design: two sinks on one
//! path would each keep their own byte count and each would roll the other's
//! file out from under it. So a test binary gets **one** installed sink, and
//! every test in it that wants to observe a production emit must assert against
//! that one. [`sink`] is that single point, and it is deliberately
//! install-or-adopt rather than install-or-fail: whichever call arrives first
//! creates it and the rest read it back.
//!
//! It carries a `Trace` floor, which is not a detail. The default floor is
//! `Info`, and `api.request served` is emitted at `Debug` for every request
//! that is not a 4xx or a 5xx. Under the default floor that site writes nothing
//! on the ordinary path, and a test asserting against it would be asserting the
//! floor rather than the emit.
//!
//! # What is proven elsewhere, and what is no longer unproven at all
//!
//! Eight sites live behind functions private to [`crate::server`], behind a
//! layer only a served request drives, or past a socket, so `server::tests`
//! proves them against **this same sink**. Three of those eight were listed
//! here for a long time as unreachable without a live vendor; none of them
//! actually was, and
//! [`the_three_sites_this_binary_cannot_reach_are_named_rather_than_forgotten`]
//! keeps each claim struck through beside what turned out to be true. That list
//! is now empty. `CLAUDE.md` §3 rule 6: said out loud, not left for a coverage
//! report to find.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use brutex_core::vendor::Vendor;
use pull::session::Day;
use store::file::BarFile;
use store::format::{Bar, HEADER_LEN, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// The process-wide sink every test in this binary shares.
///
/// Install-or-adopt. The first caller creates it; every later caller is refused
/// by [`telemetry::install`] — which is the refusal working — and reads back the
/// one that won. Both outcomes return the sink `telemetry::emit` actually
/// reaches, which is the only sink an assertion about a production emit can be
/// written against.
///
/// The directory is cleared **once**, under a [`std::sync::Once`], and never
/// per call: a second thread that cleared it after the first had opened its
/// file would unlink the inode out from under a live descriptor, and every
/// record written afterwards would go somewhere no reader can find.
pub(crate) fn sink() -> &'static telemetry::Sink {
    static PREPARED: std::sync::Once = std::sync::Once::new();
    let dir = crate::scratch::path("telemetry");
    PREPARED.call_once(|| {
        let _ignored = std::fs::remove_dir_all(&dir);
    });
    let config = telemetry::Config::new(&dir).with_min_level(telemetry::Level::Trace);
    let installed = match telemetry::install(&config) {
        Ok(installed) => installed,
        Err(_refused) => telemetry::global()
            .expect("install either created the sink or named the one that already exists"),
    };
    // THE ADOPTED SINK MUST BE THE ONE THIS FUNCTION ASKED FOR.
    //
    // `server::run_in` installs a sink of its own over the SERVED log
    // directory, at the floor `BRUTEX_LOG_LEVEL` names, so a test that reached
    // `run` before this function ran would hand every later test a different
    // directory and an `Info` floor — under which every `Debug` and `Trace`
    // site writes nothing and an assertion about one of them would fail for a
    // reason that has nothing to do with the emit. The one test that drives
    // `run` therefore calls this first; this line is what stops a future one
    // from forgetting, by failing with the reason rather than with a missing
    // record.
    assert_eq!(
        installed.path().parent(),
        Some(dir.as_path()),
        "another install won this binary's process global; every emit-site \
         assertion here is about the sink THIS function configures"
    );
    installed
}

/// The sequence number the next event written to the shared sink will carry.
///
/// Taken immediately before a production call and handed to [`landed`], so that
/// a record found afterwards is one **this** call wrote and not one an earlier
/// test left behind.
pub(crate) fn mark() -> u64 {
    sink().health().next_seq
}

/// Every record on the installed sink's files from sequence `from` onward, on
/// one target, carrying one message.
///
/// Read through [`telemetry::tail`] — the crate's own reader, which is what
/// `/logs` renders from — rather than by parsing the file here, so a test
/// cannot pass against bytes the shipped reader would refuse.
///
/// # Why the sequence number and not [`telemetry::Query::since`]
///
/// `since` was the obvious filter and it is the wrong one **for a test binary
/// running in parallel**, which is what this is. `Sink::emit` reads the clock
/// BEFORE it takes the lock, so two threads can be ordered one way by their
/// timestamps and the other way in the file — and `since` is not merely a
/// filter, it ENDS the walk on the first record older than it, on the stated
/// premise that events are in time order. One older-stamped line written by a
/// concurrent test after this one is therefore enough to stop the walk before
/// it reaches the record being asserted, which is a test that fails for a
/// reason that has nothing to do with the emit. Measured: it failed about one
/// run in three.
///
/// `seq` is assigned INSIDE the lock and is unbroken across a rotation, so it
/// is the order the file is actually in. The walk is left unbounded by time and
/// bounded by [`telemetry::MAX_LIMIT`] and the query's own scan cap instead.
pub(crate) fn landed(from: u64, target: &str, message: &str) -> Vec<telemetry::Record> {
    let sink = sink();
    let dir = sink
        .path()
        .parent()
        .expect("the sink writes a file inside a directory")
        .to_path_buf();
    let query = telemetry::Query::last(telemetry::MAX_LIMIT).from_target(target);
    telemetry::tail(&dir, sink.keep_files(), &query)
        .records
        .into_iter()
        .filter(|record| record.seq >= from && record.message == message)
        .collect()
}

/// Whether a string field carries `needle`.
///
/// A `contains` rather than an equality because the sink cuts a string value at
/// [`telemetry::MAX_STR_VALUE_BYTES`] and says so — a long temporary path is a
/// real input on macOS, and an assertion that broke on it would be an assertion
/// about the host's temporary directory. Every needle used below sits in the
/// leading component of the value, which no cut can remove.
pub(crate) fn says(record: &telemetry::Record, key: &str, needle: &str) -> bool {
    record
        .field(key)
        .and_then(telemetry::OwnedValue::as_str)
        .is_some_and(|got| got.contains(needle))
}

/// Whether an integer field carries exactly `want`.
pub(crate) fn counts(record: &telemetry::Record, key: &str, want: u64) -> bool {
    record.field(key).and_then(telemetry::OwnedValue::as_u64) == Some(want)
}

/// One production emit site, and the record it must leave in the file.
struct Case {
    /// Where the `telemetry::emit` call lives, so a failure names the line
    /// rather than the test.
    site: &'static str,
    /// The target and message the production call writes.
    target: &'static str,
    /// See [`Case::target`].
    message: &'static str,
    /// The level the production call chose. Asserted because four sites in this
    /// crate pick their level from a condition, and a flipped ternary is
    /// invisible to every other kind of test.
    level: telemetry::Level,
    /// The production call. **Never a hand-built `Event`** — a fabricated event
    /// on a local sink proves the sink works and says nothing about the site.
    drive: Box<dyn Fn()>,
    /// What makes a record this drive's rather than another test's.
    mine: Box<dyn Fn(&telemetry::Record) -> bool>,
}

/// How many rows [`cases`] holds.
///
/// Pinned so that a row deleted rather than fixed fails here instead of quietly
/// lowering the proportion that is proven.
///
/// Twenty-one since D-0297 added the commit gate's refusal — the arm every
/// server built without `BRUTEX_COMMIT` takes on every press.
const ROWS: usize = 22;

/// How many distinct production emit sites those rows cover.
///
/// One fewer than [`ROWS`], and the difference is deliberate: `assets.rs`'s
/// `note_front_end` is **one** `emit` whose level and message are chosen by a
/// condition, so it takes two rows — a built front end and an absent one —
/// which is the whole point of asserting the level at all.
const SITES_HERE: usize = ROWS - 1;

/// Every site this module drives, one row each.
///
/// The fixtures are built inside the closures rather than shared, so a row
/// cannot pass because of what an earlier row left on disk.
#[expect(
    clippy::too_many_lines,
    reason = "the length IS the table -- one row per production emit site, each \
              carrying the fixture that drives it. Splitting it into a builder \
              per module would put the rows out of reach of the single count \
              that binds them to `LIB_SITES`, which is the accounting that \
              stops a site being added with no row."
)]
fn cases() -> Vec<Case> {
    let mut cases: Vec<Case> = Vec::new();

    // crates/api/src/census.rs — the counter file's own state, per vendor.
    {
        let root = fixture("emit-census");
        cases.push(Case {
            site: "census.rs api.census read",
            target: "api.census",
            message: "read",
            // ABSENT, so `Debug` — lowered from `Info` because `read_all` runs
            // on every `/store.json`, `/instruments.json` and `/audit.json`
            // request, not once at startup as the site used to claim. The site
            // picks `Warn` only for a manifest it cannot read, and the ternary
            // is the thing being pinned.
            level: telemetry::Level::Debug,
            drive: {
                let root = root.clone();
                Box::new(move || {
                    let seen = crate::census::read_vendor(&root, Vendor::Dhan);
                    assert!(
                        matches!(seen.state, crate::census::Census::Absent),
                        "an empty root has no manifest, which is the arm this row drives"
                    );
                })
            },
            mine: Box::new(|record| {
                says(record, "path", "emit-census") && says(record, "state", "absent")
            }),
        });
    }

    // crates/api/src/master.rs — one line for a file of ~100,000 rows.
    {
        let path = fixture("emit-master").join("groww.csv");
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("a scratch directory");
        std::fs::write(
            &path,
            "is_intraday,segment,exchange,instrument_type,groww_symbol,trading_symbol,\
             series,isin,expiry_date,strike_price,underlying_symbol,internal_trading_symbol\n",
        )
        .expect("a master with a header and no rows");
        cases.push(Case {
            site: "master.rs api.master parsed",
            target: "api.master",
            message: "parsed",
            level: telemetry::Level::Info,
            drive: {
                let path = path.clone();
                Box::new(move || {
                    let loaded = crate::master::load(&path, Vendor::Groww)
                        .expect("a complete header loads, which is the arm that emits");
                    assert!(loaded.kept.is_empty(), "a header alone keeps nothing");
                })
            },
            mine: Box::new(|record| {
                says(record, "path", "emit-master") && counts(record, "kept", 0)
            }),
        });
    }

    // crates/api/src/merge.rs — the merged universe and the two ways it is wrong.
    cases.push(Case {
        site: "merge.rs api.merge universe merged",
        target: "api.merge",
        message: "universe merged",
        // NO SOURCES, SO NOTHING DISAGREES, SO `Info`. The site picks `Warn`
        // when a conflict or an eligibility dispute exists.
        level: telemetry::Level::Info,
        drive: Box::new(|| {
            let merged = crate::merge::merge(&[]);
            assert!(merged.is_empty(), "no sources merge to no keys");
        }),
        mine: Box::new(|record| {
            counts(record, "sources", 0)
                && counts(record, "keys", 0)
                && counts(record, "conflicts", 0)
        }),
    });

    // crates/api/src/catalog.rs — the index this process built, once.
    cases.push(Case {
        site: "catalog.rs api.catalog index built",
        target: "api.catalog",
        message: "index built",
        level: telemetry::Level::Info,
        drive: Box::new(|| {
            let built = crate::catalog::Catalog::build(&crate::merge::merge(&[]));
            assert!(built.is_empty(), "an empty universe indexes to no rows");
        }),
        mine: Box::new(|record| {
            counts(record, "instruments", 0) && counts(record, "order_entries", 0)
        }),
    });

    // crates/api/src/audit.rs — four sites, all on the journal.
    {
        // A FILE WHERE THE `audit` DIRECTORY HAS TO BE. `create_dir_all`
        // refuses, so the append refuses, which is the arm that emits.
        let root = fixture("emit-journal-append");
        std::fs::write(root.join("audit"), b"not a directory").expect("a file in the way");
        cases.push(Case {
            site: "audit.rs api.audit record not appended",
            target: "api.audit",
            message: "record not appended",
            level: telemetry::Level::Error,
            drive: {
                let root = root.clone();
                Box::new(move || {
                    let record = crate::audit::Record::refused(
                        crate::audit::Scope::Spot,
                        crate::audit::Outcome::Refused,
                        std::time::SystemTime::UNIX_EPOCH,
                        "emit-site-probe",
                        "the directory this journal needs is a file",
                    );
                    let why = crate::audit::Journal::at(&root)
                        .append(&record)
                        .expect_err("a file where the directory must be refuses the append");
                    assert!(why.contains("audit directory"), "{why}");
                })
            },
            mine: Box::new(|record| says(record, "source", "emit-site-probe")),
        });
    }
    {
        // A TAIL THAT IS NOT A WHOLE NUMBER OF RECORDS — a process killed
        // inside an append.
        let root = fixture("emit-journal-torn");
        std::fs::create_dir_all(root.join("audit")).expect("an audit directory");
        std::fs::write(root.join("audit").join("pull.journal"), [0u8; 100])
            .expect("100 bytes, which is less than one 256-byte record");
        cases.push(Case {
            site: "audit.rs api.audit journal tail is torn",
            target: "api.audit",
            message: "journal tail is torn",
            level: telemetry::Level::Warn,
            drive: {
                let root = root.clone();
                Box::new(move || {
                    let log = crate::audit::Journal::at(&root).look();
                    assert!(log.is_loud(), "a torn tail is loud, which is why it emits");
                })
            },
            mine: Box::new(|record| {
                says(record, "path", "emit-journal-torn") && counts(record, "torn_bytes", 100)
            }),
        });
    }
    {
        // A ROOT THAT IS A FILE. `metadata` on a path beneath it is
        // `NotADirectory` — an error that is not `NotFound`, which is the one
        // arm that reports the journal as unreadable rather than as absent.
        let root = fixture("emit-journal-blocked").join("root");
        std::fs::write(&root, b"not a directory").expect("a file where the root must be");
        cases.push(Case {
            site: "audit.rs api.audit journal unreadable",
            target: "api.audit",
            message: "journal unreadable",
            level: telemetry::Level::Warn,
            drive: {
                let root = root.clone();
                Box::new(move || {
                    let log = crate::audit::Journal::at(&root).look();
                    assert!(
                        matches!(log, crate::audit::Log::Unreadable { .. }),
                        "a root that is a file makes the journal unreadable, not absent: {log:?}"
                    );
                })
            },
            mine: Box::new(|record| says(record, "path", "emit-journal-blocked")),
        });
    }
    {
        // A PAGE OF A JOURNAL THAT IS NOT THERE. The count says there are
        // records and the open refuses, which is the whole read failing.
        let root = fixture("emit-journal-page");
        cases.push(Case {
            site: "audit.rs api.audit journal page refused",
            target: "api.audit",
            message: "journal page refused",
            level: telemetry::Level::Warn,
            drive: {
                let root = root.clone();
                Box::new(move || {
                    let why = crate::audit::Journal::at(&root)
                        .page(5, 0, 5)
                        .expect_err("a journal that is not there cannot be paged");
                    assert!(why.contains("pull.journal"), "{why}");
                })
            },
            mine: Box::new(|record| {
                says(record, "path", "emit-journal-page")
                    && counts(record, "records", 5)
                    && counts(record, "take", 5)
            }),
        });
    }

    // crates/api/src/bars.rs — a refused read, and a page that lost records.
    {
        let root = fixture("emit-bar-missing");
        cases.push(Case {
            site: "bars.rs api.bars read refused",
            target: "api.bars",
            message: "read refused",
            level: telemetry::Level::Warn,
            drive: {
                let root = root.clone();
                Box::new(move || {
                    let why = crate::bars::open(
                        &root,
                        Vendor::Dhan,
                        "NSE",
                        "CASH",
                        "EMITGONE",
                        Timeframe::MINUTE_1,
                        YearMonth::new(2026, 8).expect("a real month"),
                        None,
                    )
                    .expect_err("an empty store holds no month, and never creates one");
                    assert!(why.contains("EMITGONE"), "{why}");
                })
            },
            mine: Box::new(|record| {
                says(record, "symbol", "EMITGONE") && says(record, "timeframe", "1min")
            }),
        });
    }
    {
        let root = fixture("emit-bar-short");
        cases.push(Case {
            site: "bars.rs api.bars records unreadable",
            target: "api.bars",
            message: "records unreadable",
            level: telemetry::Level::Warn,
            drive: {
                let root = root.clone();
                Box::new(move || truncated_month_reads_no_rows(&root))
            },
            mine: Box::new(|record| {
                says(record, "file", "emit-bar-short")
                    && counts(record, "faults", 1)
                    && counts(record, "rows", 0)
            }),
        });
    }

    // crates/api/src/ingest.rs — a form that never started a run.
    cases.push(Case {
        site: "ingest.rs api.ingest form refused",
        target: "api.ingest",
        message: "form refused",
        level: telemetry::Level::Warn,
        drive: Box::new(|| {
            let why = crate::ingest::parse_spot(
                "target=emit-site-probe&from=2026-08-03&to=2026-08-07",
                Day::new(2026, 8, 10).expect("a real day"),
            )
            .expect_err("that is not a spot target this build takes");
            assert!(why.to_string().contains("emit-site-probe"), "{why}");
        }),
        mine: Box::new(|record| {
            says(record, "why", "emit-site-probe")
                && says(record, "form", "spot")
                && says(record, "field", "target")
        }),
    });

    // crates/api/src/assets.rs — where the front end was looked for, both ways,
    // and a request for a file the build never emitted.
    {
        let dir = fixture("emit-front-built");
        std::fs::create_dir_all(dir.join("build")).expect("a build directory");
        std::fs::write(dir.join("build").join("index.html"), b"<!doctype html>").expect("a shell");
        cases.push(Case {
            site: "assets.rs api.assets front end found",
            target: "api.assets",
            message: "front end found",
            // BUILT, SO `Info`. The row below drives the same site's other arm.
            level: telemetry::Level::Info,
            drive: {
                let dir = dir.clone();
                Box::new(move || {
                    assert!(
                        crate::assets::Assets::new(&dir).built(),
                        "the premise: this fixture has a build on disk"
                    );
                })
            },
            mine: Box::new(|record| {
                says(record, "path", "emit-front-built")
                    && record
                        .field("built")
                        .and_then(telemetry::OwnedValue::as_bool)
                        == Some(true)
            }),
        });
    }
    {
        // NOTHING ON DISK. Same call site, the other arm of its level and its
        // message — the state in which every page answers 503.
        let dir = fixture("emit-front-absent").join("nothing-here");
        cases.push(Case {
            site: "assets.rs api.assets front end NOT on disk",
            target: "api.assets",
            message: "front end NOT on disk — every page answers 503",
            level: telemetry::Level::Warn,
            drive: {
                let dir = dir.clone();
                Box::new(move || {
                    assert!(
                        !crate::assets::Assets::new(&dir).built(),
                        "the premise: this fixture has no build on disk"
                    );
                })
            },
            mine: Box::new(|record| {
                says(record, "path", "emit-front-absent")
                    && record
                        .field("built")
                        .and_then(telemetry::OwnedValue::as_bool)
                        == Some(false)
            }),
        });
    }
    {
        let dir = fixture("emit-front-miss");
        std::fs::create_dir_all(dir.join("build")).expect("a build directory");
        std::fs::write(dir.join("build").join("index.html"), b"<!doctype html>").expect("a shell");
        cases.push(Case {
            site: "assets.rs api.assets no such asset",
            target: "api.assets",
            message: "no such asset",
            level: telemetry::Level::Warn,
            drive: {
                let dir = dir.clone();
                Box::new(move || {
                    let front = crate::assets::Assets::new(&dir);
                    let answer = front.respond(&axum::http::Method::GET, "/emit-site-probe-1.js");
                    assert_eq!(
                        answer.status(),
                        axum::http::StatusCode::NOT_FOUND,
                        "a named asset that is not there is a 404, never the shell"
                    );
                })
            },
            // THE FIRST MISS, so `seen` is 1. The site fires on doublings and
            // `the_silent_arms_stay_silent` holds the misses in between.
            mine: Box::new(|record| {
                says(record, "path", "/emit-site-probe-1.js") && counts(record, "seen", 1)
            }),
        });
    }

    // crates/api/src/autopilot.rs — the operator's two controls.
    cases.push(Case {
        site: "autopilot.rs autopilot paused",
        target: "autopilot",
        message: "paused",
        level: telemetry::Level::Warn,
        drive: Box::new(|| {
            let control = crate::autopilot::Control::new();
            control.pause();
            assert!(control.is_paused(), "the premise: it stopped");
            assert_eq!(control.epoch(), 1, "and the stop generation moved");
        }),
        mine: Box::new(|record| counts(record, "epoch", 1)),
    });
    cases.push(Case {
        site: "autopilot.rs autopilot resumed",
        target: "autopilot",
        message: "resumed",
        level: telemetry::Level::Info,
        drive: Box::new(|| {
            let control = crate::autopilot::Control::new();
            control.resume();
            assert!(!control.is_paused(), "the premise: it is flying");
            assert_eq!(control.epoch(), 0, "and resuming does NOT bump the epoch");
        }),
        mine: Box::new(|record| counts(record, "epoch", 0)),
    });

    // crates/api/src/audit_json.rs — two refusals a status code cannot carry.
    {
        let site = json_site("emit-json-feed");
        cases.push(Case {
            site: "audit_json.rs api.audit.json request refused",
            target: "api.audit.json",
            message: "request refused",
            level: telemetry::Level::Warn,
            drive: {
                let site = std::sync::Arc::clone(&site);
                Box::new(move || {
                    let (status, _headers, body) = block_on(crate::audit_json::audit_json(
                        axum::extract::State(std::sync::Arc::clone(&site)),
                        "/audit.json?feed=emit-site-probe"
                            .parse()
                            .expect("a real uri"),
                    ));
                    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
                    assert!(body.contains("emit-site-probe"), "{body}");
                })
            },
            mine: Box::new(|record| {
                says(record, "wire", "emit-site-probe") && says(record, "param", "feed")
            }),
        });
    }
    {
        let site = json_site("emit-json-page");
        cases.push(Case {
            site: "audit_json.rs api.audit.json page ignored",
            target: "api.audit.json",
            message: "page ignored",
            level: telemetry::Level::Warn,
            drive: {
                let site = std::sync::Arc::clone(&site);
                Box::new(move || {
                    let (status, _headers, body) = block_on(crate::audit_json::audit_json(
                        axum::extract::State(std::sync::Arc::clone(&site)),
                        "/audit.json?feed=dhan&page=emit-site-probe"
                            .parse()
                            .expect("a real uri"),
                    ));
                    assert_eq!(
                        status,
                        axum::http::StatusCode::OK,
                        "an unreadable page is answered, not refused"
                    );
                    assert!(body.contains("\"page\":0"), "{body}");
                })
            },
            mine: Box::new(|record| {
                says(record, "asked", "emit-site-probe") && says(record, "param", "page")
            }),
        });
    }

    // crates/api/src/sweeprun.rs — the two refusals that never start a sweep.
    //
    // ADDED BECAUSE THE MODULE ARRIVED WITH NO TESTS AT ALL. Four emit sites
    // landed with `/backtest/run` and nothing drove any of them, which the
    // accounting below caught: `lib_sites` went 33 -> 37 while every column
    // stayed still. The two refusal paths cost nothing to drive — neither
    // touches the store, the ledger or a bar file — so they are proven here
    // rather than named as unreachable. The two that remain unreachable are
    // named in `UNREACHABLE`, because reaching them means running a real
    // sweep over stored bars.
    {
        let site = json_site("emit-sweep-malformed");
        cases.push(Case {
            site: "sweeprun.rs api.sweep a sweep was refused before it started",
            target: "api.sweep",
            message: "a sweep was refused before it started",
            level: telemetry::Level::Warn,
            drive: {
                let site = std::sync::Arc::clone(&site);
                Box::new(move || {
                    // No `feed`, which is the first thing `asked_from` demands
                    // and the one term of the run identity that cannot be
                    // defaulted.
                    let (status, _headers, body) = block_on(crate::sweeprun::run(
                        axum::extract::State(std::sync::Arc::clone(&site)),
                        "{\"underlying\":\"NIFTY\"}".to_owned(),
                    ));
                    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
                    assert!(body.contains("\"accepted\":false"), "{body}");
                    assert!(body.contains("feed"), "{body}");
                })
            },
            mine: Box::new(|record| says(record, "why", "feed")),
        });
    }
    {
        let site = json_site("emit-sweep-busy");
        // ONE ALREADY IN FLIGHT, planted rather than started: a real first
        // sweep would put the store on this test's critical path for the sake
        // of a guard that never reads it.
        {
            let mut held = site.sweep.lock().expect("a fresh mutex");
            *held = Some(crate::sweeprun::Progress::started(
                "zerodha",
                "NIFTY",
                (2024, 1),
                (2024, 1),
                47_000,
                0,
            ));
        }
        cases.push(Case {
            site: "sweeprun.rs api.sweep a second sweep was refused while one was in flight",
            target: "api.sweep",
            message: "a second sweep was refused while one was in flight",
            level: telemetry::Level::Warn,
            drive: {
                let site = std::sync::Arc::clone(&site);
                Box::new(move || {
                    // A STAMP IS PASSED, BECAUSE `cargo test` HAS NONE.
                    //
                    // The commit gate sits before the slot -- an unstamped build
                    // refuses every run it could start, so letting it claim the
                    // slot would answer 409 to a second press while the first was
                    // busy failing for a reason no wait can fix. Under `cargo
                    // test` that gate fires every time, and a press that never
                    // claims the slot cannot make the next press a conflict. This
                    // row would then be proving nothing.
                    let (status, _headers, body) = crate::sweeprun::run_with(
                        &site,
                        "{\"feed\":\"zerodha\",\"underlying\":\"NIFTY\",\"from_year\":2024,\
                         \"from_month\":1,\"to_year\":2024,\"to_month\":1,\
                         \"support_ppm\":47000}",
                        Some("0000000000000000000000000000000000000000"),
                    );
                    assert_eq!(
                        status,
                        axum::http::StatusCode::CONFLICT,
                        "a second press is refused, not queued: {body}"
                    );
                    assert!(body.contains("\"accepted\":false"), "{body}");
                })
            },
            mine: Box::new(|record| says(record, "why", "already running")),
        });

        // THE COMMIT GATE'S OWN EVENT. Driven with NO stamp, which is also what
        // every real `cargo run -p api -- serve` carries unless the operator
        // exports `BRUTEX_COMMIT` -- so this is the arm a mis-built server takes
        // on every press, and it must leave a record saying why.
        cases.push(Case {
            site: "sweeprun.rs api.sweep a sweep was refused because this build carries no commit stamp",
            target: "api.sweep",
            message: "a sweep was refused because this build carries no commit stamp",
            level: telemetry::Level::Warn,
            drive: {
                let site = std::sync::Arc::clone(&site);
                Box::new(move || {
                    let (status, _headers, body) = crate::sweeprun::run_with(
                        &site,
                        "{\"feed\":\"zerodha\",\"underlying\":\"NIFTY\",\"from_year\":2024,\
                         \"from_month\":1,\"to_year\":2024,\"to_month\":1}",
                        None,
                    );
                    assert_eq!(
                        status,
                        axum::http::StatusCode::SERVICE_UNAVAILABLE,
                        "the fix is on the server's side, which 503 says and 400 \
                         does not: {body}"
                    );
                    assert!(body.contains("\"accepted\":false"), "{body}");
                })
            },
            mine: Box::new(|record| says(record, "why", "BRUTEX_COMMIT")),
        });

        // THE DESCENT'S REFUSAL, driven for the same reason the sweep's is: it
        // returns before anything is spawned and touches no store, no ledger
        // and no bar file. A body with no `rung` is the shortest way in -- a
        // descent walks ONE rung's threshold, so the field is required and the
        // refusal names the eight it accepts.
        cases.push(Case {
            site: "sweeprun.rs api.sweep a descent was refused before it started",
            target: "api.sweep",
            message: "a descent was refused before it started",
            level: telemetry::Level::Warn,
            drive: {
                let site = std::sync::Arc::clone(&site);
                Box::new(move || {
                    let (status, _headers, body) = crate::sweeprun::descend_with(
                        &site,
                        "{\"feed\":\"zerodha\",\"underlying\":\"NIFTY\",\"from_year\":2024,\
                         \"from_month\":1,\"to_year\":2024,\"to_month\":1,\
                         \"max_points\":20,\"top\":25}",
                        Some("0000000000000000000000000000000000000000"),
                    );
                    assert_eq!(
                        status,
                        axum::http::StatusCode::BAD_REQUEST,
                        "a missing rung is a malformed body, not a conflict: {body}"
                    );
                    assert!(body.contains("\"accepted\":false"), "{body}");
                })
            },
            mine: Box::new(|record| says(record, "why", "rung")),
        });
    }
    cases
}

/// A scratch directory, emptied and recreated, named after the row that owns it.
fn fixture(name: &str) -> std::path::PathBuf {
    let dir = crate::scratch::path(name);
    let _ignored = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// A site over an empty universe and an empty store, for the JSON route.
fn json_site(name: &str) -> crate::server::Loaded {
    let root = fixture(name);
    std::sync::Arc::new(crate::server::Site::load(&root, &root))
}

/// One future, driven to completion on this thread.
///
/// A current-thread runtime rather than `#[tokio::test]`, so the table stays
/// one synchronous test: [`telemetry::install`] is a process singleton and the
/// whole point of the table is that one test owns it.
fn block_on<F: core::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a current-thread runtime")
        .block_on(future)
}

/// A month with one bar, truncated to its header under an open reader.
///
/// `store::file::BarFile::read_record` documents `ShortRead` for exactly this —
/// "when the file was truncated under this handle" — and it is the only fault a
/// well-formed month can develop that [`crate::bars::page`] reports rather than
/// refuses: the record image carries no checksum of its own, so corrupting the
/// bytes would decode into a different bar rather than into a fault.
fn truncated_month_reads_no_rows(root: &std::path::Path) {
    const SYMBOL: &str = "EMITSHORT";
    let month = YearMonth::new(2026, 8).expect("a real month");
    let parts = PathParts {
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment: "CASH",
        symbol: SYMBOL,
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month,
        file: FileKind::Bars,
    };
    // The same fold `crate::bars::open` applies, through `try_from` rather than
    // a cast: the workspace denies `cast_possible_truncation`, and the low half
    // is what the store stamps into the header.
    let symbol_id = u32::try_from(brutex_core::universe::fnv1a(SYMBOL) & 0xFFFF_FFFF)
        .expect("the low half of a 64-bit fold fits a u32");
    let path = StorePath::new(parts).expect("a well-formed store path");
    let mut writer = BarFile::open_or_create(root, path, symbol_id).expect("a fresh month");
    writer
        .append(&[Bar {
            ts_micros: 1_786_000_000_000_000,
            open: 2_450_000,
            high: 2_451_000,
            low: 2_449_000,
            close: 2_450_500,
            volume: 1,
            open_interest: OI_NULL,
        }])
        .expect("one sane bar appends");
    drop(writer);

    let reader = crate::bars::open(
        root,
        Vendor::Dhan,
        "NSE",
        "CASH",
        SYMBOL,
        Timeframe::MINUTE_1,
        month,
        None,
    )
    .expect("the month this test just wrote");
    assert_eq!(
        reader.header().n_valid,
        1,
        "the premise: one record is committed"
    );

    std::fs::OpenOptions::new()
        .write(true)
        .open(reader.path())
        .expect("the month is writable")
        .set_len(HEADER_LEN)
        .expect("the records are cut away, the header is not");

    let (rows, faults) = crate::bars::page(&reader, 0, 1);
    assert!(rows.is_empty(), "a record that will not read is not a row");
    assert_eq!(faults.len(), 1, "and the page names the gap");
}

/// **EVERY EMIT SITE THIS BINARY CAN REACH PUTS A RECORD IN THE FILE.**
///
/// One test rather than eighteen because [`telemetry::install`] is a process
/// singleton: eighteen tests would each need the same sink, and a suite whose
/// answer depends on which test installed first is a suite whose answer depends
/// on the scheduler.
///
/// Each row drives the **production** function and then reads the file back
/// through [`telemetry::tail`]. A row that built its own `Event` and emitted it
/// on a local sink would pass with the production `emit` deleted, which is the
/// defect this whole module exists to remove.
#[test]
fn every_reachable_emit_site_puts_a_record_in_the_file() {
    let sink = sink();
    assert_eq!(
        sink.health().dropped,
        0,
        "the premise: this sink has lost nothing, so a missing record below is \
         a missing emit and not a full disk"
    );

    let cases = cases();
    assert_eq!(
        cases.len(),
        ROWS,
        "a row deleted rather than fixed lowers the proportion that is proven, \
         silently, which is the state this module was written to end"
    );

    for case in &cases {
        let from = mark();
        (case.drive)();
        let found = landed(from, case.target, case.message);
        let mine: Vec<&telemetry::Record> =
            found.iter().filter(|record| (case.mine)(record)).collect();
        assert!(
            !mine.is_empty(),
            "{}: the production call wrote no {:?} record with message {:?} to {}. \
             {} record(s) on that target did land in the window; either the emit \
             is gone or its fields changed.",
            case.site,
            case.target,
            case.message,
            sink.path().display(),
            found.len(),
        );
        for record in mine {
            assert_eq!(
                record.level, case.level,
                "{}: the level decides whether an operator ever sees this line, \
                 and this one arrived at {:?}",
                case.site, record.level,
            );
            assert_eq!(record.target, case.target, "{}", case.site);
        }
    }

    assert!(
        !sink.health().is_loud(),
        "nothing was dropped while proving the sites: {:?}",
        sink.health(),
    );
}

/// **THE ARMS THAT ARE MEANT TO STAY SILENT, STAY SILENT.**
///
/// Three of the sites above sit behind a guard, and a guard that never holds is
/// a site whose volume is set by the data rather than by the request — which is
/// precisely what D-0072 forbids and what the doc comments on those sites
/// promise. Asserting only the presence of a record would leave every one of
/// those guards free to be deleted.
///
/// * `assets::Assets::note_missing` fires on a **doubling**. The third miss
///   must write nothing, or a scanner walking a wordlist rolls the run's own
///   beginning out of the 64 MiB window.
/// * `bars::note_unreadable_records` returns on an empty fault list. A clean
///   page must write nothing, or a month is one line per request forever.
/// * `audit::note_looked` is silent for an absent journal and for a whole one.
///   Emitting either would put a line on every render of every audit page.
#[test]
fn the_silent_arms_stay_silent() {
    let _installed = sink();

    let dir = fixture("emit-silence-front");
    std::fs::create_dir_all(dir.join("build")).expect("a build directory");
    std::fs::write(dir.join("build").join("index.html"), b"<!doctype html>").expect("a shell");
    let front = crate::assets::Assets::new(&dir);

    let from = mark();
    for nth in 1..=3u8 {
        let path = format!("/emit-silence-{nth}.js");
        assert_eq!(
            front.respond(&axum::http::Method::GET, &path).status(),
            axum::http::StatusCode::NOT_FOUND,
            "the premise: every one of these is a miss"
        );
    }
    let missed = landed(from, "api.assets", "no such asset");
    let mine: Vec<&telemetry::Record> = missed
        .iter()
        .filter(|record| says(record, "path", "/emit-silence-"))
        .collect();
    assert_eq!(
        mine.len(),
        2,
        "the 1st and the 2nd miss are doublings and the 3rd is not: {mine:?}"
    );
    assert!(
        mine.iter()
            .any(|record| says(record, "path", "/emit-silence-1.js")),
        "the first miss is immediate rather than batched: {mine:?}"
    );
    assert!(
        !mine
            .iter()
            .any(|record| says(record, "path", "/emit-silence-3.js")),
        "and the miss between doublings is a number on a later line, not a line: {mine:?}"
    );

    // A CLEAN PAGE OF BARS. The month written here reads back whole, so the
    // fault list is empty and the site returns before it emits.
    let root = fixture("emit-silence-bars");
    let from = mark();
    clean_month_reads_every_row(&root);
    // FIELD-FILTERED, like every other absence assertion in this file, and for
    // a reason this one alone had missed. The sibling test in this same binary
    // — `every_reachable_emit_site_puts_a_record_in_the_file` — drives a row
    // that emits this exact target AND this exact message. libtest runs the two
    // in parallel by default, so a sequence-plus-target-plus-message filter can
    // catch the OTHER test's record inside this window and fail an assertion
    // about a guard that behaved perfectly. The `file` field carries this
    // fixture's own directory, which no other test writes.
    assert!(
        landed(from, "api.bars", "records unreadable")
            .iter()
            .all(|record| !says(record, "file", "emit-silence-bars")),
        "a page where every record read must write nothing"
    );

    // AN ABSENT JOURNAL, AND A WHOLE ONE. Neither is news.
    let root = fixture("emit-silence-journal");
    let from = mark();
    let journal = crate::audit::Journal::at(&root);
    assert_eq!(journal.look(), crate::audit::Log::Absent);
    journal
        .append(&crate::audit::Record::refused(
            crate::audit::Scope::Spot,
            crate::audit::Outcome::Refused,
            std::time::SystemTime::UNIX_EPOCH,
            "emit-silence",
            "a whole record, so the tail is not torn",
        ))
        .expect("the append succeeds here");
    assert!(
        !journal.look().is_loud(),
        "one whole record leaves no torn tail"
    );
    for message in ["journal tail is torn", "journal unreadable"] {
        let noise: Vec<telemetry::Record> = landed(from, "api.audit", message)
            .into_iter()
            .filter(|record| says(record, "path", "emit-silence-journal"))
            .collect();
        assert!(
            noise.is_empty(),
            "an ordinary journal writes no {message:?} line: {noise:?}"
        );
    }
}

/// A month with one bar, read back whole. The premise of the silence above.
fn clean_month_reads_every_row(root: &std::path::Path) {
    const SYMBOL: &str = "EMITCLEAN";
    let month = YearMonth::new(2026, 8).expect("a real month");
    let symbol_id = u32::try_from(brutex_core::universe::fnv1a(SYMBOL) & 0xFFFF_FFFF)
        .expect("the low half of a 64-bit fold fits a u32");
    let path = StorePath::new(PathParts {
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment: "CASH",
        symbol: SYMBOL,
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month,
        file: FileKind::Bars,
    })
    .expect("a well-formed store path");
    let mut writer = BarFile::open_or_create(root, path, symbol_id).expect("a fresh month");
    writer
        .append(&[Bar {
            ts_micros: 1_786_000_000_000_000,
            open: 2_450_000,
            high: 2_451_000,
            low: 2_449_000,
            close: 2_450_500,
            volume: 1,
            open_interest: OI_NULL,
        }])
        .expect("one sane bar appends");
    drop(writer);

    let reader = crate::bars::open(
        root,
        Vendor::Dhan,
        "NSE",
        "CASH",
        SYMBOL,
        Timeframe::MINUTE_1,
        month,
        None,
    )
    .expect("the month this test just wrote");
    let (rows, faults) = crate::bars::page(&reader, 0, 1);
    assert_eq!(rows.len(), 1, "the premise: the record reads");
    assert!(faults.is_empty(), "and nothing is wrong with it");
}

/// Counts every `telemetry::emit` call in the LIB target, from the source.
///
/// **Because the alternative was a constant compared to a constant.** This
/// accounting test used to read `assert_eq!(17 + 5 + 3, 25)` with all four
/// hardcoded, so adding a twenty-sixth emit — or deleting one — failed nothing.
/// That is the shape `docs/04-invariants.md` S-20 already records once in this
/// repository, and `CLAUDE.md` §4 bans a test that asserts nothing. In a module
/// whose entire premise is that unproven emit sites are worthless, it was the
/// wrong place to keep one.
///
/// Comment lines are skipped, which is what lets this module's own prose name
/// the function without inflating its own count. `main.rs` is skipped because
/// it is the BIN target, with its own install and its own test.
fn lib_emit_sites() -> usize {
    // THE NEEDLE IS ASSEMBLED, NOT WRITTEN. Spelled as one literal it would
    // appear in this very file on a line that is not a comment, and this
    // function would count itself — it did, and reported twenty-six. `concat!`
    // is resolved by the compiler, so the string is identical while the
    // contiguous text never appears in the source being scanned. The original
    // version of this test hand-counted for exactly this reason and said so;
    // this is that observation, mechanised instead of trusted.
    //
    // TWO NEEDLES, BECAUSE THERE ARE TWO SPELLINGS. `emit_if!` gates on
    // `admits` before evaluating its arguments, so a site that must not pay for
    // a filtered event is written that way — and `telemetry::emit_if!(` does NOT
    // contain `telemetry::emit(`. Counting only the first made this function
    // BLIND to every converted site: the first conversion dropped the count from
    // 27 to 26 and the assertion below caught it. A second spelling that the
    // accounting cannot see is a hole in the accounting, not a detail.
    const NEEDLE: &str = concat!("telemetry", "::", "emit", "(");
    const NEEDLE_IF: &str = concat!("telemetry", "::", "emit", "_if!", "(");
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&src)
        .expect("the crate's own src is readable")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .filter(|path| path.file_name().is_some_and(|name| name != "main.rs"))
        .collect();
    paths.sort();
    assert!(
        paths.len() > 8,
        "read only {} source file(s) — the layout changed and this stopped counting",
        paths.len()
    );
    paths
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(path).expect("a tracked source file");
            text.lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .map(|line| line.matches(NEEDLE).count() + line.matches(NEEDLE_IF).count())
                .sum::<usize>()
        })
        .sum()
}

/// **THE SITES THIS BINARY CANNOT REACH — AND THERE ARE NOW NONE.**
///
/// **It was three, then two, and it is zero.** The name is left as it was
/// because `docs/05-decisions.md` cites it and that ledger is append-only; what
/// the test asserts is the accounting, and the accounting has moved. Every
/// striking-out below is kept rather than deleted, because the useful part is
/// what each row USED to claim and why that turned out to be wrong.
///
/// `CLAUDE.md` §3 rule 6. All three sat past the socket, and every one was
/// reached without a vendor, a credential or a byte of real data:
///
/// | line | event | why it was listed, and what was actually true |
/// |---|---|---|
/// | ~~2840~~ | ~~`pull.spot instrument refused`~~ | **REACHED.** It claimed to need a non-empty universe **and** a socket to a vendor, "because the refusal it logs is the vendor's". Only the first half held. The loop emits for whatever `broker_window` refuses, and that function refuses a rung the feed does not declare on its fourth statement — above the credential file, above `AwsIdentity::discover`, above every byte of network. Dhan declares `Day1` alone and a form with no `granularity` field means `Minute1`, so `served` refuses by name. Driven by `api::server::the_refused_instrument_site_is_driven_over_a_real_universe` over a universe of exactly one index. |
/// | ~~3254~~ | ~~`pull.http vendor refused a window`~~ | **REACHED.** It needed a SOCKET, not a recorded transport — and `crates/pull` had already established the loopback listener as this workspace's way of supplying one. Driven through the shipped `fetch_chunks` against a 503 by `api::server::the_two_sites_past_the_socket_are_driven_over_a_real_one`. |
/// | ~~3293~~ | ~~`pull.chunk answered`~~ | **REACHED.** "It needs a window of bars a vendor actually returned" — and a loopback listener returns one, provided the bytes are the shape the SHIPPED descriptor declares rather than a shape invented for the test. Same test, second listener, answering 200 with Dhan's own parallel-array response. |
///
/// The lesson the three of them share is one sentence: **an "unreachable" list
/// is worth re-reading rather than trusting.** Each entry was written in good
/// faith and each named a dependency the site did not actually have. A recorded
/// transport fake for `pull::fetch` — the thing all three were said to be
/// waiting on — was never needed and still does not exist.
///
/// This test asserts the accounting rather than the prose: the sites this
/// binary drives plus the sites it names must be every site in the LIB target.
/// [`UNREACHABLE`] staying at zero is not decoration — a site added tomorrow
/// with no test fails the sum here until somebody decides which column it is in.
#[test]
fn the_three_sites_this_binary_cannot_reach_are_named_rather_than_forgotten() {
    /// Sites driven from `server::tests`, because the functions holding them
    /// are private to that module, need a bound listener, or sit past a socket:
    /// `note_run_started`, `note_run_finished`, `note_member_failure`,
    /// `logs::note_request` through a served request, `api.serve listening`
    /// through `run` itself, `pull.http vendor refused a window` and
    /// `pull.chunk answered` through the shipped `fetch_chunks` against two
    /// loopback listeners, and `pull.spot instrument refused` through
    /// `broker_run` over a universe of one.
    ///
    /// Two more joined them when gate 23 closed the stderr-only diagnostics:
    /// `api.server the server stopped on an error`, driven directly through
    /// `stopped(Err(..))`, and `api.server cannot bind the listening address`,
    /// driven by `a_refused_bind_is_logged_and_not_only_printed` — which holds a
    /// `:0` port and asks the server for the same one, so the refusal is the
    /// kernel's and needs no vendor, credential or bar. Neither was added to the
    /// struck-through table below, for the reason that table itself teaches.
    ///
    /// Two more joined them with the batch that closed the adversarial sweep's
    /// silent-startup findings: `api.server the server stopped after serving a
    /// DEGRADED universe`, driven directly through `stopped_over(Ok(()),
    /// false)`, and `api.serve refused: another instance is serving this store`,
    /// driven by the second-server test in `server::tests` — which holds the
    /// store's own lock file on a handle of its own, so the refusal is the OS's
    /// and needs no second process.
    ///
    /// AND ONE THAT NO TEST IN THIS BINARY DRIVES, named here rather than
    /// quietly counted: `pull.fno discovery refused`. It fires when a vendor
    /// declines an expired-F&O discovery walk, and reaching it needs a
    /// credentialed source, a live endpoint and that endpoint answering
    /// non-2xx — three things a unit test cannot arrange and a loopback
    /// listener cannot stand in for, because the credential is read from
    /// Parameter Store before the socket is opened.
    ///
    /// It exists because the journal's note is stride-bound to 68 bytes and a
    /// discovery failure's reason BEGINS with the URL, so the vendor's status
    /// was the part that fell off the end. Measured: three 502s whose notes
    /// were all `https://api.groww.in/v1/historical/expiries?exchange=…` and
    /// nothing else. The run was legible as "it failed" and illegible as "why".
    ///
    /// Counted below as the one UNREACHABLE site, which is the honest column
    /// for it: that column exists for exactly this, and putting it anywhere
    /// else would claim a proof that does not exist.
    const REACHED_IN_SERVER_TESTS: usize = 12;
    /// AND THREE MORE THAT NO TEST IN THIS BINARY DRIVES, added 2026-08-20 and
    /// named here rather than quietly counted: `pull.roll walk starting`,
    /// `pull.roll group starting` and `pull.roll walk finished`. They report a
    /// rolling-option walk's progress, and reaching them needs the same three
    /// things the site above needs — a credentialed source, a live endpoint and
    /// a vendor that serves options by strike offset.
    ///
    /// THEY EXIST BECAUSE THAT WALK REPORTED NOTHING AT ALL. Measured
    /// 2026-08-20: `roll_every` held 0 emit sites and `roll_one` 0, and the
    /// audit journal writes one record per COMPLETED request — so a walk of
    /// roughly fifteen hundred requests was a single silence until it returned.
    /// A Dhan leg sat for twenty-eight minutes having stored no file, written
    /// no record and opened no socket, and none of those three facts
    /// distinguished it from a walk that was working and simply not finished.
    /// The operator's question was *"why is it not even called yet"* and this
    /// build could not answer it. `CLAUDE.md` §4 bans a fallback that hides a
    /// failure; a path that hides its own progress is that rule pointing
    /// inward.
    ///
    /// AND TWO MORE FROM `sweeprun.rs`, added with `/backtest/run`:
    /// `a sweep was accepted from the browser` and `a sweep finished and its
    /// record is in the ledger`. Reaching either means RUNNING A SWEEP — the
    /// first is emitted after the blocking thread is spawned and the second
    /// from inside it — so driving them puts a real walk over stored bars on
    /// this suite's critical path, which is a different kind of test from the
    /// one this table is.
    ///
    /// **The module's other two sites are NOT here, and that is the point of
    /// the distinction.** Both refusals return before anything is spawned and
    /// touch no store, no ledger and no bar file, so they are proven in the
    /// table above. `sweeprun` arrived with no tests at all and this accounting
    /// is what said so: `lib_sites` went 33 to 37 while every column stood
    /// still.
    ///
    /// AND TWO MORE AGAIN, added with `/backtest/descend` (D-0299):
    /// `a descent was accepted from the browser` and `a descent finished and
    /// its record is in the ledger`. Same shape and same reason as the sweep's
    /// pair directly above — the first is emitted after the blocking thread is
    /// spawned and the second from inside it, so driving either puts a real
    /// support WALK over stored bars on this suite's critical path. A descent is
    /// a full screen per rung of the support ladder, so it is the more expensive
    /// of the two, not the less.
    ///
    /// **The descent's refusal is NOT here**, for the same reason the sweep's is
    /// not: it returns before anything is spawned and touches no store, no
    /// ledger and no bar file, so it is proven in the table above.
    ///
    /// The rows of the table above, every one of them struck through — plus
    /// `pull.fno discovery refused`, the three named before it and the four
    /// named here, which are the sites no test in this binary can drive.
    const UNREACHABLE: usize = 8;
    // COUNTED FROM THE SOURCE, not declared. A thirty-NINTH emit added
    // anywhere under `crates/api/src` fails this test until somebody decides
    // which of the three columns it belongs in, which is the whole point of
    // the accounting.
    //
    // 37 -> 38 at D-0297: the commit gate's refusal, driven in the table above
    // rather than added to the unreachable list, because an unstamped build is
    // exactly what `cargo test` is and the row costs nothing to reach.
    let lib_sites = lib_emit_sites();
    assert_eq!(
        lib_sites, 41,
        "the LIB target holds {lib_sites} emit site(s); if that is a deliberate \
         change, move the row into the table above or into the unreachable list \
         and update this figure in the same commit"
    );

    assert_eq!(
        SITES_HERE + REACHED_IN_SERVER_TESTS + UNREACHABLE,
        lib_sites,
        "every emit site is proven here, proven in server::tests, or named above"
    );
}

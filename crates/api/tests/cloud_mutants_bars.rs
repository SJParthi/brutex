//! The public half of a mutation pass over `bars`, `bars_change`,
//! `bars_series`, `bars_sort`, `gaps_route`, `spot`, `render` and `ladder`
//! (`reports/cloud-mut-api-a-20261016.md`).
//!
//! Every test here was written against a mutant `cargo mutants` left alive,
//! or a line `cargo llvm-cov` measured as never run, and asserts exactly the
//! behaviour that mutant breaks. The report names each mutant beside the test
//! that kills it. Items a test outside the crate cannot reach are tested in a
//! `cloud_mutant_tests` module at the end of their own file instead.
//!
//! No row here is market data: every bar is generated, in a scratch store
//! under the system temporary directory that the test removes.

// The same exceptions every test module in this workspace takes: a test that
// cannot panic cannot fail, and these lints keep panics out of the SERVER.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use std::path::{Path, PathBuf};

use api::bars::Cancel;
use api::server::spot::{self, AsOf, Spot, SpotAsk};
use brutex_core::vendor::Vendor;
use pull::session::Day;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// A scratch store nobody else in this process shares, emptied first.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("brutex-cloud-mutants-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch store");
        Self(root)
    }

    fn root(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The 32-bit id `api::bars::open` checks a file's header against: the low
/// half of the symbol's FNV-1a, as `pull::ingest` stamps it.
fn symbol_id(symbol: &str) -> u32 {
    let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
    u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]])
}

/// One generated month of `symbol`'s bars on Dhan: each `(second, close)` is a
/// bar stamped at that UTC second, its other prices around the close.
fn write_month(
    root: &Path,
    segment: &str,
    symbol: &str,
    timeframe: Timeframe,
    month: YearMonth,
    bars: &[(i64, i64)],
) -> PathBuf {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe,
        month,
        file: FileKind::Bars,
    })
    .expect("a generated path");
    let mut file =
        BarFile::open_or_create(root, path, symbol_id(symbol)).expect("a generated bar file");
    let rows: Vec<Bar> = bars
        .iter()
        .map(|&(secs, close)| Bar {
            ts_micros: secs * 1_000_000,
            open: close,
            high: close + 5,
            low: close - 5,
            close,
            volume: 0,
            open_interest: OI_NULL,
        })
        .collect();
    file.append(&rows).expect("generated records");
    path.to_path_buf(root)
}

/// May 2025, the month every spot fixture is filed in.
fn may() -> YearMonth {
    YearMonth::new(2025, 5).expect("a month")
}

/// 09:15 IST on 2025-05-02, in UTC seconds.
fn open_second() -> i64 {
    i64::from(Day::new(2025, 5, 2).expect("a day").days_from_epoch()) * 86_400 + 13_500
}

/// A NIFTY minute month holding bars at the open and one minute after.
fn spot_store(tag: &str) -> Scratch {
    let scratch = Scratch::new(tag);
    let t0 = open_second();
    write_month(
        scratch.root(),
        "INDEX",
        "NIFTY",
        Timeframe::MINUTE_1,
        may(),
        &[(t0, 2_400_000), (t0 + 60, 2_401_050)],
    );
    scratch
}

fn spot_query(times: &[i64]) -> String {
    let t: Vec<String> = times.iter().map(i64::to_string).collect();
    format!(
        "feed=dhan&exchange=NSE&symbol=NIFTY&timeframe=1min&month=2025-05&t={}",
        t.join(",")
    )
}

/// `spot.rs`.
mod spot_rs {
    use super::*;

    /// **`SpotAsk::answer` IS `answer_with` UNDER A FLAG NOTHING SETS**, and it
    /// answers: a hit at a stamped minute, a named absence between two bars.
    /// Kills the three `answer -> Ok((vec![], …))` mutants: `answer` had no
    /// caller in any test, so replacing its body changed nothing measured.
    #[test]
    fn spot_answer_reads_the_file_as_answer_with_does() {
        let scratch = spot_store("spot-answer");
        let t0 = open_second();
        let asked = SpotAsk::parse(&spot_query(&[t0, t0 + 30])).expect("a well-formed ask");
        let (spots, faults) = asked.answer(scratch.root()).expect("answered");
        assert_eq!(
            spots,
            vec![
                Spot::At {
                    close: 2_400_000,
                    ts_micros: t0 * 1_000_000,
                },
                Spot::NoBar,
            ]
        );
        assert!(faults.is_empty(), "{faults:?}");
        assert_eq!(
            asked.answer_with(scratch.root(), &Cancel::never()),
            Ok((spots, faults))
        );
    }

    /// **A CANCELLED SPOT READ IS REFUSED BEFORE ITS FIRST INSTANT, EVEN WHEN IT
    /// HAS ONLY ONE** (DB-N15, D-3349). The flag is looked at on the instants
    /// `0, CANCEL_EVERY, 2·CANCEL_EVERY …`; the route's own test asks for two,
    /// so a look moved to the instants NOT on that stride (`at % 64 != 0`) still
    /// refused at the second one. With one instant there is no second look, and
    /// only a look at instant 0 refuses it. Kills `== with !=` at spot.rs:535.
    #[test]
    fn a_cancelled_spot_read_of_one_instant_is_refused_at_its_first() {
        let scratch = spot_store("spot-cancel-one");
        let asked = SpotAsk::parse(&spot_query(&[open_second()])).expect("a well-formed ask");
        let (gone, guard) = Cancel::pair();
        drop(guard);
        let refused = asked
            .answer_with(scratch.root(), &gone)
            .expect_err("a cancelled read stops");
        assert!(refused.starts_with("cancelled"), "{refused}");
        let (status, _, body) =
            spot::respond_with(scratch.root(), &spot_query(&[open_second()]), &gone);
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST, "{body}");
        assert!(body.contains("cancelled"), "{body}");
    }

    /// **`close_at` IS THE EXACT RULE, AND AN INSTANT BEFORE THE FILE'S FIRST
    /// RECORD IS BRACKETED BY THE MONTH'S OWN EDGE.** `close_at` had no caller at
    /// all, and no test asked for an instant whose lookup lands on row 0, so the
    /// `None` arm of `spot_at`'s "the record before" was never run.
    #[test]
    fn close_at_is_the_exact_rule_and_row_zero_has_no_record_before_it() {
        let scratch = spot_store("spot-close-at");
        let file = api::bars::open(
            scratch.root(),
            Vendor::Dhan,
            "NSE",
            "INDEX",
            "NIFTY",
            Timeframe::MINUTE_1,
            may(),
            None,
        )
        .expect("the generated month opens");
        let t0 = open_second() * 1_000_000;
        assert_eq!(
            spot::close_at(&file, may(), t0).expect("read"),
            Spot::At {
                close: 2_400_000,
                ts_micros: t0,
            }
        );
        assert_eq!(
            spot::close_at(&file, may(), t0 + 30_000_000).expect("read"),
            Spot::NoBar
        );
        // A DAY BEFORE THE FIRST BAR, inside May: the lookup lands on row 0, so
        // nothing stands before it but the month's edge.
        let before_first = t0 - 86_400_000_000;
        assert_eq!(
            spot::close_at(&file, may(), before_first).expect("read"),
            Spot::NoBar
        );
        assert_eq!(
            spot::spot_at(&file, may(), before_first, AsOf::Prev).expect("read"),
            Spot::NoBarInSession
        );
        // And a month that is not the file's is the wrong question.
        assert_eq!(
            spot::close_at(&file, may(), t0 + 40 * 86_400_000_000).expect("read"),
            Spot::OutsideMonth
        );
    }
}

/// `ladder.rs`.
mod ladder_rs {
    use api::ladder::{Gate, Leg, Owed, OwedWhy, Wanted, gate, owing, probing};
    use brutex_core::instrument::{Exchange, Segment};
    use brutex_core::symbol::Symbol;
    use brutex_core::vendor::Vendor;
    use pull::manifest::{Entry, EntryKey, Held, Manifest};
    use pull::vendor::Feed;
    use store::path::{Timeframe, YearMonth};

    fn sym(name: &str) -> Symbol {
        Symbol::new(name).expect("a legal symbol")
    }

    fn month(m: u8) -> YearMonth {
        YearMonth::new(2018, m).expect("a real month")
    }

    fn wanted(name: &str, months: &[u8]) -> Vec<Wanted> {
        months
            .iter()
            .map(|&m| Wanted {
                leg: Leg::Spot,
                symbol: sym(name),
                exchange: Exchange::Nse,
                segment: Segment::Index,
                month: month(m),
            })
            .collect()
    }

    /// **EACH REASON A MONTH IS OWED HAS ITS OWN WORDS**, and they are the
    /// words an operator reads in the refusal. Kills `OwedWhy::label -> ""`
    /// and `-> "xyzzy"`: no test read a label.
    #[test]
    fn ladder_every_owed_reason_is_said_in_its_own_words() {
        assert_eq!(
            OwedWhy::NothingHeld.label(),
            "nothing of the series is held in this window"
        );
        assert_eq!(OwedWhy::Interior.label(), "a hole between held months");
        assert_eq!(
            OwedWhy::AfterLast.label(),
            "after the series' last held month"
        );
    }

    /// **ONLY `Gate::Open` IS OPEN.** Every refusal variant answers `false`.
    /// Kills `Gate::is_open -> true`: the tests that asserted a refusal
    /// compared the whole variant and never asked `is_open` of one.
    #[test]
    fn ladder_only_the_open_gate_is_open() {
        assert!(Gate::Open.is_open());
        let refusals = [
            Gate::RungFirst {
                needs: Timeframe::DAY_1,
                missing: 1,
                of: 1,
                owed: Vec::new(),
                runs: 1,
            },
            Gate::LegFirst {
                needs: Leg::Spot,
                at: Timeframe::DAY_1,
                missing: 1,
                of: 1,
            },
            Gate::SegmentFirst {
                needs: Segment::Index,
                at: Timeframe::DAY_1,
                missing: 1,
                of: 1,
            },
        ];
        for refused in refusals {
            assert!(!refused.is_open(), "{refused:?}");
        }
    }

    /// **`owing` NAMES THE RUNS AN INSTRUMENT OWES, ON A GATED FEED, AT A RUNG
    /// WITH ONE BELOW IT — AND NOTHING ELSE** (DB-B27). A series held in
    /// January and March owes February; a folder feed and the day rung owe
    /// nothing. Kills `owing -> vec![]` and `delete !` in `owing`.
    #[test]
    fn ladder_owing_names_the_hole_only_where_the_order_applies() {
        let held =
            |k: &EntryKey| k.timeframe == Timeframe::DAY_1 && matches!(k.month.month(), 1 | 3);
        let ask = wanted("HOLED", &[1, 2, 3]);
        let hole = Owed {
            symbol: sym("HOLED"),
            exchange: Exchange::Nse,
            segment: Segment::Index,
            from: month(2),
            to: month(2),
            months: 1,
            why: OwedWhy::Interior,
        };
        assert_eq!(
            owing(held, Feed::Zerodha, Timeframe::MINUTE_1, &ask),
            vec![hole]
        );
        assert_eq!(
            owing(held, Feed::Gdfl, Timeframe::MINUTE_1, &ask),
            Vec::new(),
            "a folder feed is never gated"
        );
        assert_eq!(
            owing(held, Feed::Zerodha, Timeframe::DAY_1, &ask),
            Vec::new(),
            "the day rung has no rung below it"
        );
    }

    /// **A SERIES' HELD LIFE IS ITS EARLIEST AND LATEST HELD MONTH IN WHATEVER
    /// ORDER THE ASK NAMES THEM.** Asked March, January, February with January
    /// and March held, February is a hole between them and is owed. A span
    /// that kept the first held month it met as its start (`<` read as `==`
    /// in `owed_runs`) began at March and excused February as "before the
    /// first held month": the minute rung opened over a hole.
    #[test]
    fn ladder_an_unordered_ask_still_owes_the_hole_between_held_months() {
        let held =
            |k: &EntryKey| k.timeframe == Timeframe::DAY_1 && matches!(k.month.month(), 1 | 3);
        let gated = gate(
            held,
            Feed::Zerodha,
            Timeframe::MINUTE_1,
            &wanted("HOLED", &[3, 1, 2]),
        );
        let Gate::RungFirst {
            needs,
            missing,
            of,
            owed,
            runs,
        } = gated
        else {
            panic!("February is owed: {gated:?}");
        };
        assert_eq!((needs, missing, of, runs), (Timeframe::DAY_1, 1, 3, 1));
        assert_eq!(
            owed.iter().map(|o| (o.from, o.why)).collect::<Vec<_>>(),
            vec![(month(2), OwedWhy::Interior)]
        );
    }

    /// **A VENDOR'S CENSUS READ THROUGH `census::Manifests` HOLDS EXACTLY
    /// WHAT ITS MANIFEST HOLDS** (D-3316). Kills both constant answers of
    /// `<Manifests as HoldsKey>::holds`: only the plain `Manifest` impl was
    /// probed by a test.
    #[test]
    fn ladder_a_census_of_manifests_holds_what_its_manifest_holds() {
        let key = |m: u8| EntryKey {
            contract: None,
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol: sym("NIFTY"),
            timeframe: Timeframe::DAY_1,
            month: month(m),
        };
        let mut manifest = Manifest::open(Vendor::Groww, &[], &[]).expect("a genesis census");
        manifest
            .record_held(Held::unknown(Entry {
                key: key(1),
                rows: 20,
                first_ts_micros: 1,
                last_ts_micros: 2,
            }))
            .expect("the census has room");
        let census = api::census::Manifests::single(manifest);
        let held = probing(&census);
        assert!(held(&key(1)));
        assert!(!held(&key(2)));
    }
}

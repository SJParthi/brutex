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

    /// **A ZERO STAMP IS NEVER A BRACKET** (DB-039): the record before `t`
    /// stamped at the epoch is the zero-filled extent `store::file`
    /// documents, so a miss beside it is `spot_order_broken`, not a named
    /// absence. January 1970 is a month the store files, and 1970-01-01 05:30
    /// IST — the epoch — lies on its minute grid. Runs `spot_at`'s return of a
    /// verdict that is not `NoBar`, which no test reached.
    #[test]
    fn a_miss_beside_a_zero_stamp_is_order_broken() {
        let scratch = Scratch::new("spot-zero-stamp");
        let january = YearMonth::new(1970, 1).expect("the store's first month");
        write_month(
            scratch.root(),
            "INDEX",
            "NIFTY",
            Timeframe::MINUTE_1,
            january,
            &[(0, 100), (60, 101)],
        );
        let file = api::bars::open(
            scratch.root(),
            Vendor::Dhan,
            "NSE",
            "INDEX",
            "NIFTY",
            Timeframe::MINUTE_1,
            january,
            None,
        )
        .expect("the generated month opens");
        assert_eq!(
            spot::close_at(&file, january, 30_000_000).expect("read"),
            Spot::OrderBroken
        );
        assert_eq!(
            spot::close_at(&file, january, 60_000_000).expect("read"),
            Spot::At {
                close: 101,
                ts_micros: 60_000_000,
            },
            "a hit is still a hit"
        );
    }

    /// **A FULL STORE-READ POOL IS A NAMED `429` WITH `Retry-After`** through
    /// the handler itself (DB-066, DB-G80). Every slot is held by a read that
    /// waits on this test; the handler is then refused at admission. Runs
    /// `spot_json`'s refusal arm, which no test reached: the route's own test
    /// called `not_admitted` directly.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spot_json_is_refused_by_name_when_every_store_read_slot_is_held() {
        let scratch = spot_store("spot-saturated");
        let masters = scratch.root().join("masters");
        std::fs::create_dir_all(&masters).expect("an empty master root");
        let site: api::server::Loaded =
            std::sync::Arc::new(api::server::Site::load(&masters, scratch.root()));
        let release = std::sync::Arc::new(std::sync::Barrier::new(
            api::detail::MAX_STORE_READ_CONCURRENT + 1,
        ));
        let holders: Vec<_> = (0..api::detail::MAX_STORE_READ_CONCURRENT)
            .map(|_| {
                let release = std::sync::Arc::clone(&release);
                tokio::spawn(api::detail::run_store_read(move || {
                    let _ = release.wait();
                }))
            })
            .collect();
        // THE POOL IS FULL ONCE A PROBE IS REFUSED: each holder takes its
        // slot before its closure runs, so this waits only for them to start.
        let mut full = false;
        for _ in 0..200 {
            if api::detail::run_store_read(|| ()).await == Err(api::detail::RunError::Saturated) {
                full = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let uri: axum::http::Uri = format!("/spot.json?{}", spot_query(&[open_second()]))
            .parse()
            .expect("a URI");
        let (status, headers, body) = spot::spot_json(axum::extract::State(site), uri).await;
        let _ = release.wait();
        for holder in holders {
            assert_eq!(holder.await.expect("joined"), Ok(()));
        }
        assert!(full, "the holders never filled the pool");
        assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS, "{body}");
        assert_eq!(headers[axum::http::header::RETRY_AFTER], "1");
        assert!(body.contains("a spot read"), "{body}");
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

    /// **AN OPTION WAITS ON THE FUTURES LEG UNDER THE FUTURES' OWN SEGMENT
    /// ONCE SPOT IS WHOLE.** With spot held at both rungs, an expired-options
    /// pass is refused for the futures day pass, probed under `FNO` — the arm
    /// of the leg gate that names a derivative predecessor, which no test
    /// reached: each one that asked an option stopped at spot first.
    #[test]
    fn ladder_an_option_waits_on_futures_once_spot_is_whole() {
        let option = Wanted {
            leg: Leg::Option,
            symbol: sym("NIFTY"),
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            month: month(3),
        };
        let probed = std::cell::RefCell::new(Vec::new());
        let held = |k: &EntryKey| {
            probed.borrow_mut().push((k.segment, k.timeframe));
            k.segment == Segment::Index
        };
        let gated = gate(held, Feed::Zerodha, Timeframe::DAY_1, &[option]);
        assert_eq!(
            gated,
            Gate::LegFirst {
                needs: Leg::Future,
                at: Timeframe::DAY_1,
                missing: 1,
                of: 1,
            }
        );
        assert_eq!(
            probed.borrow().last().copied(),
            Some((Segment::Fno, Timeframe::DAY_1)),
            "the futures leg is probed under its own segment"
        );
    }
}

/// `render.rs`: the two public pages whose branches a mutant could flip
/// without any test noticing.
mod render_rs {
    use api::census::{Census, Coverage, Series, StoreFilter, VendorCensus};
    use api::render::{BarsView, Notes, StoreView, bars_page, store_page};
    use brutex_core::instrument::{Exchange, Segment};
    use brutex_core::symbol::Symbol;
    use brutex_core::vendor::Vendor;
    use pull::session::Day;
    use store::format::{Bar, OI_NULL};
    use store::path::{Timeframe, YearMonth};

    fn nifty() -> Series {
        Series {
            contract: None,
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol: Symbol::new("NIFTY").expect("a legal symbol"),
            timeframe: Timeframe::MINUTE_1,
        }
    }

    fn held_row(groww: Option<u64>) -> Coverage {
        Coverage {
            series: nifty(),
            month: YearMonth::new(2026, 7).expect("a month"),
            rows: vec![(Vendor::Groww, groww), (Vendor::Dhan, None)],
        }
    }

    fn absent(vendor: Vendor) -> VendorCensus {
        VendorCensus {
            vendor,
            path: std::path::PathBuf::from("/nowhere"),
            state: Census::Absent,
        }
    }

    /// The `/store` page over `rows`, for Groww.
    fn store(
        rows: &[Coverage],
        censuses: &[VendorCensus],
        filter: Option<&StoreFilter>,
        (total, held, held_only): (usize, usize, bool),
    ) -> String {
        store_page(&StoreView {
            feed: Vendor::Groww,
            today: Day::new(2026, 8, 7).expect("a day"),
            censuses,
            rows,
            page: 0,
            last_page: 0,
            total,
            notes: &Notes::build(&[]),
            filter,
            held,
            held_only,
        })
    }

    /// **THE PAGE'S CARDS AND ITS PICKER NAME THE FEED IT SHOWS, AND ONLY
    /// THAT ONE IS MARKED.** Groww's page draws Groww's census card and not
    /// Dhan's, and its feed picker marks Groww `is-on` and Dhan not. Kills
    /// `== with !=` in `store_page`'s census filter (the cards of every OTHER
    /// feed) and in `coverage_table`'s picker (every other feed marked on).
    #[test]
    fn render_the_store_page_marks_only_the_feed_it_shows() {
        let rows = [held_row(Some(8_250))];
        let html = store(
            &rows,
            &[absent(Vendor::Groww), absent(Vendor::Dhan)],
            None,
            (1, 1, false),
        );
        assert!(html.contains("groww · months held"), "{html}");
        assert!(!html.contains("dhan · months held"), "{html}");
        assert!(
            html.contains(
                r#"<a class="feedpick-one is-on" href="/store?feed=groww" aria-current="true">"#
            ),
            "{html}"
        );
        assert!(
            html.contains(r#"<a class="feedpick-one" href="/store?feed=dhan">"#),
            "{html}"
        );
        assert_eq!(
            html.matches(r#"class="feedpick-one is-on""#).count(),
            1,
            "{html}"
        );
    }

    /// **A COUNT OF ZERO IS NOT A LINK.** A cell links to the bars it counts
    /// only when there are some; a zero would open a page that can only say
    /// the month holds nothing. Kills `> with >=` in `coverage_table`.
    #[test]
    fn render_a_cell_counting_no_rows_is_not_a_link_to_them() {
        let linked = store(&[held_row(Some(7))], &[], None, (1, 1, false));
        assert!(linked.contains("href=\"/bars?"), "{linked}");
        let zero = store(&[held_row(Some(0))], &[], None, (1, 1, false));
        assert!(!zero.contains("href=\"/bars?"), "{zero}");
    }

    /// **THE KIND PILL A FILTER NAMES IS THE ONE CHECKED.** A filter on
    /// indices checks `Indices` and not `All`. Kills `== with !=` in
    /// `store_filter_bar` (every pill but the named one checked); no test drew
    /// the bar with a segment set, so the line never ran.
    #[test]
    fn render_the_filter_bar_checks_the_kind_the_filter_names() {
        let filter = StoreFilter {
            segment: Some(Segment::Index),
            ..StoreFilter::default()
        };
        let html = store(&[], &[], Some(&filter), (0, 0, true));
        assert!(
            html.contains(r#"id="k-INDEX" value="INDEX" checked>"#),
            "{html}"
        );
        assert!(!html.contains(r#"id="k-" value="" checked>"#), "{html}");
        assert!(
            !html.contains(r#"id="k-CASH" value="CASH" checked>"#),
            "{html}"
        );
    }

    /// **A NARROWED HELD LIST SAYS WHAT IT NARROWED FROM, AND AN EMPTY RESULT
    /// OVER A FULL STORE SAYS THE STORE IS NOT EMPTY.** A filter that names
    /// something adds " of N in the store"; one that names nothing does not.
    /// Kills `delete !` in `store_page`'s `narrowed`, and runs the two
    /// sentences no test reached.
    #[test]
    fn render_a_narrowed_held_list_names_the_store_it_narrowed() {
        let narrowed = StoreFilter {
            symbol: Some("BANK".to_owned()),
            ..StoreFilter::default()
        };
        let html = store(&[], &[], Some(&narrowed), (0, 5, true));
        assert!(
            html.contains("<b>0</b> instrument-month(s) held of 5 in the store"),
            "{html}"
        );
        assert!(
            html.contains("nothing matches this filter; the store is not empty"),
            "{html}"
        );
        let plain = store(&[], &[], Some(&StoreFilter::default()), (5, 5, true));
        assert!(
            plain.contains("<b>5</b> instrument-month(s) held · showing"),
            "{plain}"
        );
        assert!(!plain.contains("in the store"), "{plain}");
    }

    fn bars(rows: &[Bar], trouble: Option<&str>) -> String {
        bars_page(&BarsView {
            symbol: "NIFTY",
            contract: None,
            segment: "INDEX",
            vendor: "dhan",
            month: "2025-05",
            rows,
            total: rows.len(),
            page: 0,
            last_page: 0,
            timeframe: "1min",
            trouble,
            store_root: "/store",
        })
    }

    fn one_bar() -> Bar {
        Bar {
            ts_micros: 1_746_157_500_000_000,
            open: 2_400_000,
            high: 2_400_500,
            low: 2_399_500,
            close: 2_400_100,
            volume: 0,
            open_interest: OI_NULL,
        }
    }

    /// **THE PRICES PAGE SAYS "held" ONLY WITH ROWS AND NO TROUBLE, AND "empty
    /// month" ONLY FOR A FILE THAT OPENED AND HOLDS NOTHING.** Kills both
    /// mutants of `ok` (`&& with ||`, `delete !`) and `&& with ||` on the empty
    /// month block, and runs that block, which no test drew.
    #[test]
    fn render_the_prices_page_tells_held_empty_and_refused_apart() {
        let held = bars(&[one_bar()], None);
        assert!(held.contains(">held<"), "{held}");
        assert!(!held.contains("nothing to show"), "{held}");
        assert!(!held.contains("empty month"), "{held}");

        let empty = bars(&[], None);
        assert!(empty.contains("nothing to show"), "{empty}");
        assert!(empty.contains("empty month"), "{empty}");
        assert!(
            empty.contains("holds no committed record"),
            "the empty month says why: {empty}"
        );

        let refused = bars(&[], Some("the month is not held"));
        assert!(refused.contains("nothing to show"), "{refused}");
        assert!(refused.contains("the month is not held"), "{refused}");
        assert!(!refused.contains("empty month"), "{refused}");

        // Rows beside a trouble are not "held" either.
        let troubled = bars(&[one_bar()], Some("1 record would not read"));
        assert!(troubled.contains("nothing to show"), "{troubled}");
        assert!(!troubled.contains("empty month"), "{troubled}");
    }
}

/// `render.rs`: `json_string`, the one JSON string writer every route uses.
mod render_json_rs {
    use api::render::json_string;

    /// **EVERY CONTROL CHARACTER BELOW 0x20 IS A `\uXXXX` ESCAPE, AND A
    /// SPACE IS A SPACE**; `\r` and `\t` have their own short escapes. Kills
    /// the match guard `(c as u32) < 0x20 -> false` (a raw control character,
    /// which is invalid JSON), `< with ==` and `< with <=` (a space escaped),
    /// and runs the `\r` and `\t` arms no test reached.
    #[test]
    fn render_json_string_escapes_every_control_character_and_no_space() {
        assert_eq!(json_string("a\u{1}b"), "\"a\\u0001b\"");
        assert_eq!(json_string("\u{1f}"), "\"\\u001f\"");
        assert_eq!(json_string("a b"), "\"a b\"");
        assert_eq!(json_string("\r\t"), "\"\\r\\t\"");
        for byte in 0_u8..0x20 {
            let text = char::from(byte).to_string();
            let escaped = json_string(&text);
            let parsed: String = serde_json::from_str(&escaped).expect("valid JSON");
            assert_eq!(parsed, text, "{escaped}");
            assert!(
                !escaped.contains(char::from(byte)),
                "no raw control character: {escaped:?}"
            );
        }
    }
}

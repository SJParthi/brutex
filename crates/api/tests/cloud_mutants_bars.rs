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

/// `bars.rs`: the public window and page reads, over generated months.
mod bars_rs {
    use super::{Scratch, write_month};
    use api::bars::{
        Cancel, Filter, MAX_SERIES, SeriesKey, SortKey, Window, WindowQuery, page_cancellable,
        window,
    };
    use brutex_core::vendor::Vendor;
    use store::file::BarFile;
    use store::format::Bar;
    use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

    fn month(m: u8) -> YearMonth {
        YearMonth::new(2025, m).expect("a month")
    }

    /// The UTC second of 09:15 IST on day `d` of 2025-`m`.
    fn open_of(m: u8, d: u8) -> i64 {
        i64::from(
            pull::session::Day::new(2025, m, d)
                .expect("a day")
                .days_from_epoch(),
        ) * 86_400
            + 13_500
    }

    /// `n` one-minute bars from 09:15 IST on day `d` of 2025-`m`, closing at
    /// `base + i`.
    fn minutes(m: u8, d: u8, n: i64, base: i64) -> Vec<(i64, i64)> {
        (0..n).map(|i| (open_of(m, d) + i * 60, base + i)).collect()
    }

    fn query(series: Vec<SeriesKey>, from: YearMonth, to: YearMonth) -> WindowQuery {
        WindowQuery {
            vendor: Vendor::Dhan,
            timeframe: Timeframe::MINUTE_1,
            series,
            from,
            to,
            sort: SortKey::Ts,
            desc: false,
            offset: Some(0),
            limit: 50,
            want_extremes: false,
            at: None,
            filter: Filter::default(),
            lookback: true,
            coverage: false,
        }
    }

    fn index(symbol: &str) -> SeriesKey {
        SeriesKey::new("NSE", "INDEX", symbol, None)
    }

    fn read(root: &std::path::Path, asked: &WindowQuery) -> Window {
        window(root, asked).expect("the window answers")
    }

    /// **A WINDOW NAMES ONE TO `MAX_SERIES` SERIES, AND NO SERIES IS REFUSED
    /// BY NAME**, before any file is read. Runs the series-count refusal,
    /// which no test reached.
    #[test]
    fn bars_a_window_naming_no_series_or_too_many_is_refused_by_name() {
        let scratch = Scratch::new("bars-series-count");
        let none = window(scratch.root(), &query(Vec::new(), month(5), month(5)))
            .expect_err("no series is refused");
        assert!(
            none.starts_with("too_many_series") && none.contains("0 were named"),
            "{none}"
        );
        let many = vec![index("NIFTY"); MAX_SERIES + 1];
        let refused = window(scratch.root(), &query(many, month(5), month(5)))
            .expect_err("too many series is refused");
        assert!(
            refused.contains(&format!("{} were named", MAX_SERIES + 1)),
            "{refused}"
        );
    }

    /// **OPEN INTEREST OVER INDEX SERIES IS TIME ORDER, AND ITS EXTREMES ARE
    /// FOLDED.** An index has no open interest (D-3322), so the page is a
    /// seek, said `by_time`, and `extremes=1` still answers. Runs the extremes
    /// arm of the by-time open-interest path.
    #[test]
    fn bars_open_interest_over_an_index_is_time_order_with_its_extremes() {
        let scratch = Scratch::new("bars-oi-by-time");
        write_month(
            scratch.root(),
            "INDEX",
            "NIFTY",
            Timeframe::MINUTE_1,
            month(5),
            &minutes(5, 2, 5, 100),
        );
        let mut asked = query(vec![index("NIFTY")], month(5), month(5));
        asked.sort = SortKey::OpenInterest;
        asked.desc = true;
        asked.want_extremes = true;
        let got = read(scratch.root(), &asked);
        let report = got.sort_index.expect("a sort report");
        assert!(report.by_time);
        assert_eq!(report.column, "oi");
        let closes: Vec<i64> = got.bars.iter().map(|row| row.bar.close).collect();
        assert_eq!(
            closes,
            vec![100, 101, 102, 103, 104],
            "ascending time in both directions"
        );
        let extremes = got.extremes.expect("folded");
        assert_eq!(extremes.close_max.map(|e| e.v), Some(104));
        assert_eq!(extremes.close_min.map(|e| e.v), Some(100));
        assert_eq!(got.extremes_why, None);
    }

    /// **A TIME-OF-DAY ORDER FOLDS THE EXTREMES OF EVERY KEPT ROW IT
    /// STREAMS.** Runs the scan's extremes fold and its `Some(extremes)`.
    #[test]
    fn bars_a_time_of_day_order_folds_the_extremes_it_streams() {
        let scratch = Scratch::new("bars-tod-extremes");
        let mut rows = minutes(5, 2, 3, 200);
        rows.extend(minutes(5, 5, 3, 300));
        write_month(
            scratch.root(),
            "INDEX",
            "NIFTY",
            Timeframe::MINUTE_1,
            month(5),
            &rows,
        );
        let mut asked = query(vec![index("NIFTY")], month(5), month(5));
        asked.sort = SortKey::TimeOfDay;
        asked.want_extremes = true;
        let got = read(scratch.root(), &asked);
        let closes: Vec<i64> = got.bars.iter().map(|row| row.bar.close).collect();
        assert_eq!(
            closes,
            vec![200, 300, 201, 301, 202, 302],
            "by the second of the day, ties in time order"
        );
        let extremes = got.extremes.expect("folded");
        assert_eq!(extremes.close_max.map(|e| e.v), Some(302));
        assert_eq!(extremes.close_min.map(|e| e.v), Some(200));
        assert_eq!(extremes.volume_max, None, "an index trades no volume");
        assert_eq!(got.scan_rows, 6);
    }

    /// **A TIME-OF-DAY ORDER HAS NO NULL ROW, AND ITS VOLUME EXTREME IS THE
    /// TRADED SERIES'.** Over an index and a series whose rows carry open
    /// interest and volume, the scan orders by the second of the day alone
    /// — open interest is not this order's column, so no row sorts last as
    /// a null — and folds volume only where it trades. Kills `scan`'s
    /// `delete !` on the volume flag and its `&&` → `||` on the null rule.
    #[test]
    fn bars_a_time_of_day_order_over_an_index_and_a_traded_series() {
        let scratch = Scratch::new("bars-tod-mixed");
        write_month(
            scratch.root(),
            "INDEX",
            "NIFTY",
            Timeframe::MINUTE_1,
            month(5),
            &minutes(5, 2, 2, 200),
        );
        let traded = StorePath::new(PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: "TESTSEG",
            symbol: "OITEST",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: month(5),
            file: FileKind::Bars,
        })
        .expect("a generated path");
        let rows: Vec<Bar> = minutes(5, 5, 2, 300)
            .into_iter()
            .map(|(secs, close)| Bar {
                ts_micros: secs * 1_000_000,
                open: close,
                high: close + 5,
                low: close - 5,
                close,
                volume: 7,
                open_interest: 40_000,
            })
            .collect();
        BarFile::open_or_create(scratch.root(), traded, super::symbol_id("OITEST"))
            .expect("a generated bar file")
            .append(&rows)
            .expect("generated records");
        let mut asked = query(
            vec![
                index("NIFTY"),
                SeriesKey::new("NSE", "TESTSEG", "OITEST", None),
            ],
            month(5),
            month(5),
        );
        asked.sort = SortKey::TimeOfDay;
        asked.want_extremes = true;
        let got = read(scratch.root(), &asked);
        let closes: Vec<(i64, usize)> = got
            .bars
            .iter()
            .map(|row| (row.bar.close, row.series))
            .collect();
        assert_eq!(
            closes,
            vec![(200, 0), (300, 1), (201, 0), (301, 1)],
            "by the second of the day, ties in time order, neither series last"
        );
        let extremes = got.extremes.expect("folded");
        assert_eq!(extremes.volume, 7);
        assert_eq!(
            extremes.volume_max.map(|e| (e.v, e.series)),
            Some((7, 1)),
            "the traded series' volume, never the index's none"
        );
    }

    /// **NO MORE THAN `MAX_NAMED_MISSING` HOLES ARE NAMED, AND THE COUNT IS
    /// ALWAYS WHOLE.** One series held in 2023-01 and 2025-12 has 34 holes
    /// between: the first 24 are named, in order, and all 34 are counted.
    /// Kills `tally`'s `<` → `<=`, which names a 25th.
    #[test]
    fn bars_past_max_named_missing_holes_are_counted_but_not_named() {
        let scratch = Scratch::new("bars-named-cap");
        for (year, m) in [(2023, 1), (2025, 12)] {
            let held = YearMonth::new(year, m).expect("a month");
            let open = i64::from(
                pull::session::Day::new(year, m, 2)
                    .expect("a day")
                    .days_from_epoch(),
            ) * 86_400
                + 13_500;
            write_month(
                scratch.root(),
                "INDEX",
                "NIFTY",
                Timeframe::MINUTE_1,
                held,
                &[(open, 10), (open + 60, 11)],
            );
        }
        let got = read(
            scratch.root(),
            &query(
                vec![index("NIFTY")],
                YearMonth::new(2023, 1).expect("a month"),
                YearMonth::new(2025, 12).expect("a month"),
            ),
        );
        assert_eq!(got.months_missing, 34, "every hole is counted");
        assert_eq!(got.months_outside_life, 0);
        assert_eq!(got.missing_months.len(), api::bars::MAX_NAMED_MISSING);
        assert_eq!(
            (
                got.missing_months.first().map(String::as_str),
                got.missing_months.last().map(String::as_str)
            ),
            (Some("2023-02"), Some("2025-01")),
            "the first holes in order, unkeyed over one series"
        );
        assert_eq!(got.total, 4);
    }

    /// **OVER SEVERAL SERIES, A HOLE IS NAMED WITH THE SERIES IT IS IN.** Runs
    /// `tally`'s keyed hole, which no test reached: every hole test was over
    /// one series.
    #[test]
    fn bars_a_hole_over_several_series_names_its_series() {
        let scratch = Scratch::new("bars-keyed-hole");
        for (m, base) in [(4, 10), (6, 30)] {
            write_month(
                scratch.root(),
                "INDEX",
                "NIFTY",
                Timeframe::MINUTE_1,
                month(m),
                &minutes(m, 2, 2, base),
            );
        }
        write_month(
            scratch.root(),
            "INDEX",
            "BANKNIFTY",
            Timeframe::MINUTE_1,
            month(5),
            &minutes(5, 2, 2, 50),
        );
        let got = read(
            scratch.root(),
            &query(vec![index("NIFTY"), index("BANKNIFTY")], month(4), month(6)),
        );
        assert_eq!(
            got.missing_months,
            vec!["NSE-INDEX-NIFTY 2025-05".to_owned()]
        );
        assert_eq!(
            got.months_missing, 3,
            "NIFTY's hole and BANKNIFTY's two outside its life"
        );
        assert_eq!(got.months_outside_life, 2);
        assert_eq!(got.total, 6);
    }

    /// **TWO DAY SERIES MERGE BY TIME**: a day bar may sit anywhere in its
    /// day, so the merge bisects microseconds there. Runs `merged`'s day-rung
    /// step.
    #[test]
    fn bars_two_day_series_merge_by_time() {
        let scratch = Scratch::new("bars-day-merge");
        let midnight = |d: u8| open_of(5, d) - 33_300;
        for (symbol, days, base) in [("NIFTY", [2, 6], 10), ("BANKNIFTY", [5, 7], 20)] {
            let rows: Vec<(i64, i64)> = days
                .iter()
                .zip(0..)
                .map(|(&d, i)| (midnight(d), base + i))
                .collect();
            write_month(
                scratch.root(),
                "INDEX",
                symbol,
                Timeframe::DAY_1,
                month(5),
                &rows,
            );
        }
        let mut asked = query(vec![index("NIFTY"), index("BANKNIFTY")], month(5), month(5));
        asked.timeframe = Timeframe::DAY_1;
        asked.offset = Some(1);
        asked.limit = 2;
        let got = read(scratch.root(), &asked);
        let picked: Vec<(usize, i64)> = got
            .bars
            .iter()
            .map(|row| (row.series, row.bar.close))
            .collect();
        assert_eq!(
            picked,
            vec![(1, 20), (0, 11)],
            "rows 1 and 2 of 2, 5, 6, 7 May"
        );
        assert_eq!(got.total, 4);
    }

    /// **WITH THE LOOKBACK OFF, THE SERIES' FIRST ROW HAS NOTHING BEHIND IT**:
    /// no earlier month is held at all, so it is `no_earlier_stored_bar`, not
    /// a predecessor left unread. Runs `behind_of`'s no-earlier-month arm.
    #[test]
    fn bars_the_first_row_with_no_earlier_month_has_nothing_behind_it() {
        let scratch = Scratch::new("bars-lookback-first");
        write_month(
            scratch.root(),
            "INDEX",
            "NIFTY",
            Timeframe::MINUTE_1,
            month(5),
            &minutes(5, 2, 3, 100),
        );
        let mut asked = query(vec![index("NIFTY")], month(5), month(5));
        asked.lookback = false;
        let got = read(scratch.root(), &asked);
        assert_eq!(got.bars[0].chg_why, "no_earlier_stored_bar");
        assert_eq!(got.bars[1].chg, Some(100), "101 against 100 is +1%");
    }

    /// **A CANCELLABLE PAGE PAST THE MONTH'S END STOPS AT THE LAST ROW.** Runs
    /// `page_cancellable`'s short-chunk stop.
    #[test]
    fn bars_a_cancellable_page_past_the_end_returns_what_is_held() {
        let scratch = Scratch::new("bars-page-short");
        write_month(
            scratch.root(),
            "INDEX",
            "NIFTY",
            Timeframe::MINUTE_1,
            month(5),
            &minutes(5, 2, 10, 100),
        );
        let file = api::bars::open(
            scratch.root(),
            Vendor::Dhan,
            "NSE",
            "INDEX",
            "NIFTY",
            Timeframe::MINUTE_1,
            month(5),
            None,
        )
        .expect("the month opens");
        let (rows, faults) =
            page_cancellable(&file, 4, 1_000, &Cancel::never()).expect("not cancelled");
        assert_eq!(rows.len(), 6);
        assert_eq!(rows.first().map(|bar| bar.close), Some(104));
        assert!(faults.is_empty());
    }

    /// **THE SECOND OF THE DAY IS THE IST ONE**: 09:15 IST sorts before 12:00
    /// IST whatever day either is on. A second of the day taken against the
    /// wrong side of UTC (`+ with -` in `SortKey::of`) put every bar after
    /// 11:00 IST ahead of the morning, because the shift wraps past midnight
    /// there; the other time-of-day test's bars were all before 11:00.
    #[test]
    fn bars_the_second_of_the_day_is_the_ist_one() {
        let scratch = Scratch::new("bars-tod-ist");
        let noon = open_of(5, 2) + 2 * 3_600 + 45 * 60;
        let rows = vec![(noon, 1_200), (open_of(5, 5), 915)];
        write_month(
            scratch.root(),
            "INDEX",
            "NIFTY",
            Timeframe::MINUTE_1,
            month(5),
            &rows,
        );
        let mut asked = query(vec![index("NIFTY")], month(5), month(5));
        asked.sort = SortKey::TimeOfDay;
        let got = read(scratch.root(), &asked);
        let closes: Vec<i64> = got.bars.iter().map(|row| row.bar.close).collect();
        assert_eq!(closes, vec![915, 1_200]);
    }

    /// **A CANCELLABLE PAGE RETURNS EXACTLY THE ROWS IT WAS ASKED FOR, ACROSS
    /// CHUNKS, AND AN ASK PAST THE MONTH STOPS AT ITS END.** 70,000 one-second
    /// rows are more than one 65,536-row chunk. Kills `- with +` in the chunk
    /// size (rows past the ask returned), `< with ==` and `< with <=` on the
    /// short-chunk stop (a read stopped after its first full chunk), and
    /// asks for `usize::MAX` rows, which only that stop ends.
    #[test]
    fn bars_a_cancellable_page_reads_exactly_its_ask_across_chunks() {
        let scratch = Scratch::new("bars-page-chunks");
        let start = open_of(5, 2) - 33_300;
        let rows: Vec<(i64, i64)> = (0..70_000).map(|i| (start + i, 100 + i)).collect();
        write_month(
            scratch.root(),
            "INDEX",
            "NIFTY",
            Timeframe::SECOND_1,
            month(5),
            &rows,
        );
        let file = api::bars::open(
            scratch.root(),
            Vendor::Dhan,
            "NSE",
            "INDEX",
            "NIFTY",
            Timeframe::SECOND_1,
            month(5),
            None,
        )
        .expect("the month opens");
        let never = Cancel::never();
        let (three, _) = page_cancellable(&file, 4, 3, &never).expect("read");
        assert_eq!(
            three.iter().map(|bar| bar.close).collect::<Vec<_>>(),
            vec![104, 105, 106]
        );
        let (all, faults) = page_cancellable(&file, 0, 70_000, &never).expect("read");
        assert_eq!(all.len(), 70_000);
        assert_eq!(all.last().map(|bar| bar.close), Some(100 + 69_999));
        assert!(faults.is_empty());
        let (rest, _) = page_cancellable(&file, 69_990, usize::MAX, &never).expect("read");
        assert_eq!(rest.len(), 10);
    }

    /// One generated month of TESTSEG `symbol` on Dhan: each
    /// `(day, minutes after 09:15 IST, close)` is a bar.
    fn testseg_month(root: &std::path::Path, symbol: &str, m: u8, rows: &[(u8, i64, i64)]) {
        let rows: Vec<(i64, i64)> = rows
            .iter()
            .map(|&(d, minute, close)| (open_of(m, d) + minute * 60, close))
            .collect();
        write_month(
            root,
            "TESTSEG",
            symbol,
            Timeframe::MINUTE_1,
            month(m),
            &rows,
        );
    }

    /// 09:20 to 09:21 IST, inclusive: the time-of-day bound both tests below
    /// keep.
    const NINE_TWENTY: (u32, u32) = (33_600, 33_660);

    /// The change and its reason of every row a time-ordered page serves.
    fn changes(got: &Window) -> Vec<(i64, Option<i64>, &'static str)> {
        got.bars
            .iter()
            .map(|row| (row.bar.close, row.chg, row.chg_why))
            .collect()
    }

    /// **A ROW'S PREDECESSOR IS THE ROW BEFORE IT IN ITS FILE, NOT THE LAST
    /// ROW THE PAGE SERVED.** Under a time-of-day bound the page serves runs
    /// with unkept rows between them: in one month (day 2's window, then day
    /// 5's after an unkept 09:15) and across two (May's last row, then June's
    /// window after two unkept rows). Each run's first row measures against
    /// the row just before it in its own file. Kills `behind_of`'s `== with
    /// !=` on both halves of the one-read shortcut (the slot and the row).
    #[test]
    fn bars_a_row_measures_against_the_row_before_it_in_its_file() {
        let scratch = Scratch::new("bars-behind-shortcut");
        testseg_month(
            scratch.root(),
            "BEHIND",
            5,
            &[(2, 5, 1_000), (2, 6, 1_010), (6, 0, 2_000), (6, 5, 3_000)],
        );
        testseg_month(
            scratch.root(),
            "BEHIND",
            6,
            // FOUR UNKEPT ROWS, so June's window opens at row 4 — one past
            // May's last row served, row 3: the shortcut's row test alone
            // would pass, and only its slot test refuses it.
            &[
                (2, 0, 1_400),
                (2, 1, 1_450),
                (2, 2, 1_500),
                (2, 3, 2_000),
                (2, 5, 3_000),
                (2, 6, 3_030),
            ],
        );
        let mut asked = query(
            vec![SeriesKey::new("NSE", "TESTSEG", "BEHIND", None)],
            month(5),
            month(6),
        );
        asked.filter.tod = Some(NINE_TWENTY);
        let got = read(scratch.root(), &asked);
        assert_eq!(
            changes(&got),
            vec![
                (1_000, None, "no_earlier_stored_bar"),
                (1_010, Some(100), ""),
                (3_000, Some(5_000), ""),
                (3_000, Some(5_000), ""),
                (3_030, Some(100), ""),
            ],
            "day 6's 09:20 against its 09:15, June's 09:20 against its 09:18 — \
             never against the last row served"
        );
    }

    /// Holds `symbol`'s INDEX month `m` exclusive, as an import writer holds
    /// it, without writing a byte.
    fn hold(root: &std::path::Path, symbol: &str, m: u8) -> store::flock::Flock<std::fs::File> {
        let lock = StorePath::new(PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: "INDEX",
            symbol,
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: month(m),
            file: FileKind::Lock,
        })
        .expect("a generated path")
        .to_path_buf(root);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock)
            .expect("the lock file");
        store::flock::Flock::try_lock(file, lock).expect("the month's lock is free")
    }

    /// **A MONTH WHOSE DAYS CANNOT BE COUNTED IS A FAULT, NOT A ZERO.** With
    /// `coverage=1`, April's days are counted and May — held by another
    /// writer, its header counted while it was free — will not open to be
    /// counted: it is left out of the coverage and its refusal is said.
    /// Runs `coverage_of`'s refusal arm, which no test reached.
    #[test]
    fn bars_a_month_whose_days_cannot_be_counted_is_a_fault() {
        let scratch = Scratch::new("bars-coverage-held");
        for m in [4, 5] {
            write_month(
                scratch.root(),
                "INDEX",
                "NIFTY",
                Timeframe::MINUTE_1,
                month(m),
                &minutes(m, 2, 2, 100),
            );
        }
        let mut asked = query(vec![index("NIFTY")], month(4), month(5));
        let free = read(scratch.root(), &asked);
        assert_eq!(free.total, 4);
        let _held = hold(scratch.root(), "NIFTY", 5);
        asked.coverage = true;
        asked.limit = 1;
        let got = read(scratch.root(), &asked);
        let counted: Vec<(u8, u32)> = got
            .coverage
            .iter()
            .map(|one| (one.month.month(), one.days_covered))
            .collect();
        assert_eq!(counted, vec![(4, 1)], "May is not counted as a zero");
        assert!(
            got.faults
                .iter()
                .any(|fault| fault.contains("2025-05") && fault.contains("another writer holds")),
            "{:?}",
            got.faults
        );
    }

    /// **A MERGED PAGE WHOSE BISECTION MEETS A MONTH THAT WILL NOT OPEN IS
    /// REFUSED, IN TIME ORDER AND IN THE OPEN-INTEREST ORDER THAT IS TIME
    /// ORDER.** Two index series over April to June, counted once while
    /// free; then NIFTY's May is held by another writer. A page in the middle
    /// of the merge must rank inside May to find its first row, and May will
    /// not open: the request is refused, never answered from the months that
    /// did. Runs the refusal out of `merged` on both of the window's seek
    /// paths, which no test reached.
    #[test]
    fn bars_a_merged_page_that_must_rank_a_held_month_is_refused() {
        let scratch = Scratch::new("bars-merge-held");
        for symbol in ["NIFTY", "BANKNIFTY"] {
            for m in 4..=6 {
                write_month(
                    scratch.root(),
                    "INDEX",
                    symbol,
                    Timeframe::MINUTE_1,
                    month(m),
                    &minutes(m, 2, 30, 100),
                );
            }
        }
        let mut asked = query(vec![index("NIFTY"), index("BANKNIFTY")], month(4), month(6));
        asked.offset = Some(90);
        asked.limit = 10;
        let free = read(scratch.root(), &asked);
        assert_eq!(
            (free.total, free.bars.len()),
            (180, 10),
            "{:?}",
            free.faults
        );
        let _held = hold(scratch.root(), "NIFTY", 5);
        for sort in [SortKey::Ts, SortKey::OpenInterest] {
            asked.sort = sort;
            let refused = window(scratch.root(), &asked)
                .expect_err("a merge that cannot rank its page is refused");
            assert!(refused.contains("2025-05"), "{sort:?}: {refused}");
        }
    }

    /// **A TIME-OF-DAY ORDER COUNTS THE ROWS IT READ, NOT THEIR PLACES.**
    /// Under a 09:20–09:21 bound the scan reads two day windows of one
    /// month, the second starting at row 4: `scan_rows` is the four kept
    /// rows. Kills `scan`'s `- with +` on a run's length, which counts the
    /// second window as its end plus its start.
    #[test]
    fn bars_a_time_of_day_scan_counts_the_rows_of_each_window() {
        let scratch = Scratch::new("bars-tod-count");
        testseg_month(
            scratch.root(),
            "TODCOUNT",
            5,
            &[
                (2, 5, 100),
                (2, 6, 101),
                (5, 0, 200),
                (5, 1, 201),
                (5, 5, 300),
                (5, 6, 301),
            ],
        );
        let mut asked = query(
            vec![SeriesKey::new("NSE", "TESTSEG", "TODCOUNT", None)],
            month(5),
            month(5),
        );
        asked.sort = SortKey::TimeOfDay;
        asked.filter.tod = Some(NINE_TWENTY);
        let got = read(scratch.root(), &asked);
        let closes: Vec<i64> = got.bars.iter().map(|row| row.bar.close).collect();
        assert_eq!(closes, vec![100, 300, 101, 301]);
        assert_eq!((got.total, got.scan_rows), (4, 4));
    }

    /// **A TIME-OF-DAY ORDER WITH A MONTH THAT WILL NOT READ SERVES NO
    /// EXTREMES, AND SAYS WHY.** April and May, counted while free; then May
    /// is held by another writer. The scan reads April's two rows and cannot
    /// read May's, and April's extremes are not the window's: none is
    /// served, the rows that would not read are counted in the reason, and
    /// April's rows are still the page. Runs `window_cancellable`'s
    /// withheld-extremes arm for a scan, which no test reached.
    #[test]
    fn bars_a_time_of_day_order_with_an_unread_month_withholds_the_extremes() {
        let scratch = Scratch::new("bars-tod-unread");
        for m in [4, 5] {
            write_month(
                scratch.root(),
                "INDEX",
                "NIFTY",
                Timeframe::MINUTE_1,
                month(m),
                &minutes(m, 2, 2, 100),
            );
        }
        let mut asked = query(vec![index("NIFTY")], month(4), month(5));
        asked.sort = SortKey::TimeOfDay;
        asked.want_extremes = true;
        let free = read(scratch.root(), &asked);
        assert_eq!((free.total, free.extremes.is_some()), (4, true));
        let _held = hold(scratch.root(), "NIFTY", 5);
        let got = read(scratch.root(), &asked);
        assert_eq!(got.extremes, None);
        assert_eq!(
            got.extremes_why.as_deref(),
            Some(
                "months_unread: 2 kept rows of this window would not read, so its extremes are \
                 not known and none is served"
            )
        );
        assert_eq!(got.bars.len(), 2, "April's rows are still the page");
    }

    /// **WITHOUT THE LOOKBACK, A MONTH'S FIRST ROW AFTER AN UNKEPT ROW OF A
    /// MONTH IN THE REQUEST IS FIRST IN ITS WINDOW**, not a month left unread:
    /// May is read, its last row is outside 09:20–09:21, so June's first row
    /// has nothing kept behind it inside the request. Kills `behind_of`'s
    /// `slot_of(before).is_some()` guard → `false`.
    #[test]
    fn bars_without_lookback_a_month_after_an_unkept_row_is_first_in_window() {
        let scratch = Scratch::new("bars-behind-unkept");
        testseg_month(
            scratch.root(),
            "UNKEPT",
            5,
            &[(2, 5, 1_000), (2, 15, 1_100)],
        );
        testseg_month(scratch.root(), "UNKEPT", 6, &[(2, 5, 3_000)]);
        let mut asked = query(
            vec![SeriesKey::new("NSE", "TESTSEG", "UNKEPT", None)],
            month(5),
            month(6),
        );
        asked.filter.tod = Some(NINE_TWENTY);
        asked.lookback = false;
        let got = read(scratch.root(), &asked);
        assert_eq!(
            changes(&got),
            vec![
                (1_000, None, "no_earlier_stored_bar"),
                (3_000, None, "first_bar_in_window"),
            ]
        );
    }

    /// **A BAR THAT DID NOT MOVE IS UNSIGNED**: its change is `0.00`, never
    /// `+0.00`. Kills `> with >=` on `table`'s sign.
    #[test]
    fn bars_a_flat_bar_is_unsigned() {
        let flat = store::format::Bar {
            ts_micros: open_of(5, 2) * 1_000_000,
            open: 2_450_000,
            high: 2_450_000,
            low: 2_450_000,
            close: 2_450_000,
            volume: 1,
            open_interest: store::format::OI_NULL,
        };
        let html = api::bars::table(&[flat]);
        assert!(html.contains("<td class=\"num delta\">0.00</td>"), "{html}");
        assert!(!html.contains("+0.00"), "{html}");
    }

    /// **ONLY A GDFL OPTION ON NSE IS DECODED FOR THE KEY IT CONTINUES AS**
    /// (D-3331; WP-12c): the same contract on another feed, or on another
    /// exchange, is no GDFL key and has no successor. Kills both `|| with &&`
    /// in `rekey_status`'s guard, each of which decoded one of them.
    #[test]
    fn bars_only_a_gdfl_nse_option_is_decoded_for_its_successor() {
        let scratch = Scratch::new("bars-rekey-guard");
        let contract = brutex_core::instrument::Contract::parse("2023-06-29-1000000-PE");
        // EACH ASKED KEY HOLDS ONE ONE-SECOND BAR, so the window answers
        // rather than refusing a key that names no series (AG-03).
        for (vendor, exchange) in [
            (Vendor::Dhan, "NSE"),
            (Vendor::Gdfl, "BSE"),
            (Vendor::Gdfl, "NSE"),
        ] {
            let month = YearMonth::new(2020, 6).expect("a month");
            let path = StorePath::new(PathParts {
                vendor,
                exchange,
                segment: "FNO",
                symbol: "NIFTY",
                contract,
                timeframe: Timeframe::SECOND_1,
                month,
                file: FileKind::Bars,
            })
            .expect("a generated path");
            let mut file = BarFile::open_or_create(scratch.root(), path, super::symbol_id("NIFTY"))
                .expect("a generated bar file");
            let at = month.ist_bounds_micros().0 + 33_300 * 1_000_000;
            file.append(&[Bar {
                ts_micros: at,
                open: 100,
                high: 105,
                low: 95,
                close: 100,
                volume: 1,
                open_interest: 7,
            }])
            .expect("a generated record");
        }
        let ask = |vendor: Vendor, exchange: &str| {
            let mut asked = query(
                vec![SeriesKey::new(exchange, "FNO", "NIFTY", contract)],
                YearMonth::new(2020, 6).expect("a month"),
                YearMonth::new(2020, 6).expect("a month"),
            );
            asked.vendor = vendor;
            asked.timeframe = Timeframe::SECOND_1;
            read(scratch.root(), &asked).superseded
        };
        assert_eq!(ask(Vendor::Dhan, "NSE"), Vec::new(), "not a GDFL key");
        assert_eq!(ask(Vendor::Gdfl, "BSE"), Vec::new(), "not an NSE key");
        assert_eq!(
            ask(Vendor::Gdfl, "NSE"),
            vec![api::bars::SupersededBy {
                key: "NSE-FNO-NIFTY-2023-06-29-1000000-PE".to_owned(),
                by: "NSE-FNO-NIFTY-2023-06-28-1000000-PE".to_owned(),
                kind: "vendor_redated",
                recorded: false,
            }],
            "the decoder files it under its settled expiry"
        );
    }

    /// **EXACTLY `MAX_SERIES` SERIES ARE READ**: the bound is inclusive.
    /// Each series holds one bar (a key that names no series is refused by
    /// name, AG-03), so the window counts one row a series. Kills `> with >=`
    /// in the series-count refusal.
    #[test]
    fn bars_exactly_max_series_series_are_read() {
        let scratch = Scratch::new("bars-max-series");
        let many: Vec<SeriesKey> = (0..MAX_SERIES)
            .map(|i| {
                let symbol = format!("S{i}");
                write_month(
                    scratch.root(),
                    "INDEX",
                    &symbol,
                    Timeframe::MINUTE_1,
                    month(5),
                    &minutes(5, 2, 1, 100),
                );
                index(&symbol)
            })
            .collect();
        let got = read(scratch.root(), &query(many, month(5), month(5)));
        assert_eq!(got.series.len(), MAX_SERIES);
        assert_eq!(got.total, MAX_SERIES as u64, "one row in every series");
    }

    /// **A TIME-OF-DAY ORDER NOT ASKED FOR ITS EXTREMES SERVES NONE.** Kills
    /// `&& with ||` on the scan's extremes, which served them unasked.
    #[test]
    fn bars_a_time_of_day_order_serves_no_extremes_unasked() {
        let scratch = Scratch::new("bars-tod-unasked");
        write_month(
            scratch.root(),
            "INDEX",
            "NIFTY",
            Timeframe::MINUTE_1,
            month(5),
            &minutes(5, 2, 3, 200),
        );
        let mut asked = query(vec![index("NIFTY")], month(5), month(5));
        asked.sort = SortKey::TimeOfDay;
        let got = read(scratch.root(), &asked);
        assert_eq!(got.bars.len(), 3);
        assert_eq!(got.extremes, None);
        assert_eq!(got.extremes_why, None);
    }

    /// **EXTREMES A DAMAGED MONTH WITHHOLDS ARE WITHHELD BY NAME, AND THE
    /// PAGE IS STILL ANSWERED** on the by-time open-interest path. Kills the
    /// match guard `why.starts_with("cancelled") -> true` there, which failed
    /// the whole window over extremes it could not fold.
    #[test]
    fn bars_extremes_a_damaged_month_withholds_leave_the_page_answered() {
        let scratch = Scratch::new("bars-oi-damaged");
        let rows = minutes(5, 2, 5, 100);
        let path = write_month(
            scratch.root(),
            "INDEX",
            "NIFTY",
            Timeframe::MINUTE_1,
            month(5),
            &rows,
        );
        let mut bytes = std::fs::read(&path).expect("the month");
        let stamp = (rows[2].0 * 1_000_000).to_le_bytes();
        let at = bytes
            .windows(8)
            .rposition(|w| w == stamp)
            .expect("the record's stamp is in the file");
        bytes[at] ^= 0x01;
        std::fs::write(&path, &bytes).expect("damage the generated month");
        let mut asked = query(vec![index("NIFTY")], month(5), month(5));
        asked.sort = SortKey::OpenInterest;
        asked.want_extremes = true;
        let got = window(scratch.root(), &asked).expect("the page is answered");
        assert_eq!(got.extremes, None);
        let why = got.extremes_why.expect("withheld by name");
        assert!(why.starts_with("months_unread"), "{why}");
    }
}

/// `bars_series.rs`: series keys, and the plan a window reads by.
mod bars_series_rs {
    use super::{Scratch, symbol_id, write_month};
    use api::bars::{Filter, SeriesKey, SortKey, WindowQuery, window};
    use brutex_core::vendor::Vendor;
    use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

    /// A segment no NSE venue answers to, so these tests read the plan alone:
    /// the D-3325 change rules a real segment brings have their own tests.
    const SEGMENT: &str = "TESTSEG";
    const SYMBOL: &str = "WINDOWTEST";

    fn month(m: u8) -> YearMonth {
        YearMonth::new(2025, m).expect("a month")
    }

    /// The UTC second of 09:15 IST on day `d` of 2025-`m`.
    fn open_of(m: u8, d: u8) -> i64 {
        i64::from(
            pull::session::Day::new(2025, m, d)
                .expect("a day")
                .days_from_epoch(),
        ) * 86_400
            + 13_500
    }

    fn fixture() -> SeriesKey {
        SeriesKey::new("NSE", SEGMENT, SYMBOL, None)
    }

    fn query(from: YearMonth, to: YearMonth) -> WindowQuery {
        WindowQuery {
            vendor: Vendor::Dhan,
            timeframe: Timeframe::MINUTE_1,
            series: vec![fixture()],
            from,
            to,
            sort: SortKey::Ts,
            desc: false,
            offset: Some(0),
            limit: 50,
            want_extremes: false,
            at: None,
            filter: Filter::default(),
            lookback: true,
            coverage: false,
        }
    }

    /// The bar file of the fixture series' minute month `m`.
    fn month_path(root: &std::path::Path, m: u8) -> std::path::PathBuf {
        StorePath::new(PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: SEGMENT,
            symbol: SYMBOL,
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: month(m),
            file: FileKind::Bars,
        })
        .expect("a legal path")
        .to_path_buf(root)
    }

    /// A month file that exists and holds no record.
    fn empty_month(root: &std::path::Path, m: u8) {
        let path = StorePath::new(PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: SEGMENT,
            symbol: SYMBOL,
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: month(m),
            file: FileKind::Bars,
        })
        .expect("a legal path");
        drop(store::file::BarFile::open_or_create(root, path, symbol_id(SYMBOL)).expect("a file"));
    }

    /// **A CENSUS KEY READS BACK ONLY WHEN EVERY PART OF IT IS THERE, AND A
    /// TAIL IS A CONTRACT ONLY WHEN IT HAS A CONTRACT'S SHAPE AND PARSES AS
    /// ONE.** Runs `SeriesKey::parse`'s empty-part and empty-symbol refusals,
    /// the shape test's refusal and a well-shaped tail that is no contract.
    #[test]
    fn bars_series_a_key_reads_back_only_when_whole() {
        for text in [
            "-INDEX-NIFTY",
            "NSE--NIFTY",
            "NSE-INDEX-",
            "NSE-FNO--2019-03-28-FUT",
        ] {
            assert_eq!(SeriesKey::parse(text), None, "{text}");
        }
        let odd = SeriesKey::parse("NSE-CASH-X-1-2-3-4").expect("a key");
        assert_eq!((odd.symbol.as_str(), odd.contract), ("X-1-2-3-4", None));
        let unparsed =
            SeriesKey::parse("NSE-FNO-NIFTY-2019-03-28-99999999999999-CE").expect("a key");
        assert_eq!(
            (unparsed.symbol.as_str(), unparsed.contract),
            ("NIFTY-2019-03-28-99999999999999-CE", None),
            "a tail longer than any contract is no contract"
        );
        let future = SeriesKey::parse("NSE-FNO-NIFTY-2019-03-28-FUT").expect("a key");
        assert_eq!(future.symbol, "NIFTY");
        assert_eq!(future.text(), "NSE-FNO-NIFTY-2019-03-28-FUT");
    }

    /// **A TAIL IS A CONTRACT ONLY WHEN EVERY FIELD HAS A CONTRACT'S SHAPE.**
    /// Each text below fails exactly one of the shape's tests — a year, a
    /// month or a day that is not its width of digits, a side that is not
    /// `CE` or `PE`, a future's date or its `FUT` — and so reads back as a
    /// symbol with no contract; a symbol with a dash still splits from a
    /// real contract; and a key whose whole remainder is a contract has no
    /// symbol to cut, so the remainder is the symbol. Kills `contract_shaped`'s
    /// `&& with ||` mutants, its `-> true`, its `FUT` test and
    /// `split_contract`'s `cut > 0` → `>= 0`.
    #[test]
    fn bars_series_a_tail_is_a_contract_only_by_its_shape() {
        for symbol in [
            "NIFTY-ABCD-EF-GH-100-CE",
            "NIFTY-ABCD-03-28-100-CE",
            "NIFTY-2019-EF-28-100-CE",
            "NIFTY-ABCD-EF-28-100-CE",
            "NIFTY-2019-3-28-100-CE",
            "NIFTY-2019-03-28-1X0-CE",
            "NIFTY-2019-03-28-100-XE",
            "NIFTY-ABCD-EF-GH-FUT",
            "NIFTY-2019-03-28-XYZ",
            "2019-03-28-FUT",
        ] {
            let key = SeriesKey::parse(&format!("NSE-FNO-{symbol}")).expect("a key");
            assert_eq!(
                (key.symbol.as_str(), key.contract),
                (symbol, None),
                "{symbol}"
            );
        }
        let dashed = SeriesKey::parse("NSE-FNO-BAJAJ-AUTO-2019-03-28-100-CE").expect("a key");
        assert_eq!(dashed.symbol, "BAJAJ-AUTO");
        assert_eq!(
            dashed.contract.map(|c| c.to_string()),
            Some("2019-03-28-100-CE".to_owned())
        );
    }

    /// **A MONTH LISTED AFTER ITS DIRECTORY WAS FIRST READ IS SEEN.** The
    /// listing of a rung directory is kept under the directory's stamp; a
    /// month written since moves the stamp, so the next window lists again.
    /// Kills `held_months`' `== with !=` on the stamp, which served the old
    /// listing exactly when the directory had changed.
    #[test]
    fn bars_series_a_month_written_after_the_listing_is_seen() {
        let scratch = Scratch::new("series-relist");
        write_month(
            scratch.root(),
            SEGMENT,
            SYMBOL,
            Timeframe::MINUTE_1,
            month(4),
            &[(open_of(4, 2), 10)],
        );
        let before = window(scratch.root(), &query(month(4), month(5))).expect("answers");
        assert_eq!((before.total, before.months_read), (1, 1));
        write_month(
            scratch.root(),
            SEGMENT,
            SYMBOL,
            Timeframe::MINUTE_1,
            month(5),
            &[(open_of(5, 2), 20)],
        );
        let after = window(scratch.root(), &query(month(4), month(5))).expect("answers");
        assert_eq!((after.total, after.months_read), (2, 2));
    }

    /// **A MONTH'S DAY WINDOWS, KEPT FROM ONE REQUEST, ARE FILED UNDER THE
    /// NEXT REQUEST'S OWN SLOT.** June is slot 1 of an April-to-June request
    /// and slot 0 of a June-only one; its day runs under a 09:20–09:21 bound
    /// are memoised by file, so the second request reuses them, and must
    /// read them from its own June. Kills `cut_days`' `delete field slot`.
    #[test]
    fn bars_series_kept_day_runs_are_read_from_this_requests_slot() {
        let scratch = Scratch::new("series-day-runs-slot");
        for (m, close) in [(5, 500), (6, 600)] {
            write_month(
                scratch.root(),
                SEGMENT,
                SYMBOL,
                Timeframe::MINUTE_1,
                month(m),
                &[
                    (open_of(m, 2) + 300, close),
                    (open_of(m, 2) + 360, close + 1),
                ],
            );
        }
        let mut asked = query(month(5), month(6));
        asked.filter.tod = Some((33_600, 33_660));
        let both = window(scratch.root(), &asked).expect("answers");
        let closes: Vec<i64> = both.bars.iter().map(|row| row.bar.close).collect();
        assert_eq!(closes, vec![500, 501, 600, 601]);
        asked.from = month(6);
        let june = window(scratch.root(), &asked).expect("answers");
        let closes: Vec<i64> = june.bars.iter().map(|row| row.bar.close).collect();
        assert_eq!(closes, vec![600, 601], "{:?}", june.faults);
        assert!(june.faults.is_empty(), "{:?}", june.faults);
    }

    /// Holds the fixture series' minute month `m` exclusive, as an import
    /// writer holds it, without writing a byte.
    fn hold(root: &std::path::Path, m: u8) -> store::flock::Flock<std::fs::File> {
        let lock = StorePath::new(PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: SEGMENT,
            symbol: SYMBOL,
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: month(m),
            file: FileKind::Lock,
        })
        .expect("a legal path")
        .to_path_buf(root);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock)
            .expect("the lock file");
        store::flock::Flock::try_lock(file, lock).expect("the month's lock is free")
    }

    /// **A BOUND THAT DOES NOT CUT A MONTH'S RECORDS OPENS NO MONTH.** May's
    /// first record is at midnight IST of the 2nd, and the day bound starts
    /// exactly there; a time-of-day bound whose first second is one past its
    /// last keeps no second of any day. Neither needs a lookup, so with May
    /// held by another writer (counted while it was free) the plan is still
    /// built and the window answers. Kills `cut_months`' `lo > first_ts` →
    /// `>=` and `day_runs`' `window_lo < window_hi` → `<=`, each of which
    /// opened the month to locate an edge that needed no locating.
    #[test]
    fn bars_series_a_bound_that_cuts_no_record_opens_no_month() {
        let scratch = Scratch::new("series-no-lookup");
        let midnight = open_of(5, 2) - 33_300;
        write_month(
            scratch.root(),
            SEGMENT,
            SYMBOL,
            Timeframe::MINUTE_1,
            month(5),
            &[(midnight, 10), (open_of(5, 2), 11)],
        );
        let free = window(scratch.root(), &query(month(5), month(5))).expect("answers");
        assert_eq!(free.total, 2);
        let _held = hold(scratch.root(), 5);
        let mut from_midnight = query(month(5), month(5));
        from_midnight.filter.from = Some(midnight * 1_000_000);
        let got = window(scratch.root(), &from_midnight)
            .expect("a bound at the first record locates nothing");
        assert_eq!(got.total, 2);
        let mut no_second = query(month(5), month(5));
        no_second.filter.tod = Some((33_601, 33_600));
        let got =
            window(scratch.root(), &no_second).expect("a window of no second locates nothing");
        assert_eq!(got.total, 0);
        // AN EMPTY MONTH IS NEVER LOCATED IN, held or not: June holds no
        // record, so a day bound inside it has nothing to cut.
        empty_month(scratch.root(), 6);
        let free = window(scratch.root(), &query(month(6), month(6))).expect("answers");
        assert_eq!((free.total, free.months_read), (0, 1));
        let _june = hold(scratch.root(), 6);
        let mut inside_june = query(month(6), month(6));
        inside_june.filter.from = Some((open_of(6, 10) - 33_300) * 1_000_000);
        let got = window(scratch.root(), &inside_june).expect("an empty month locates nothing");
        assert_eq!(got.total, 0);
    }

    /// **TWO MONTHS THAT WILL NOT OPEN ARE TWO FAULTS, EACH WITH ITS OWN
    /// ROWS.** April and May are counted while free, then held by another
    /// writer; a time-of-day order streams every kept row and opens neither.
    /// The answer says each month once, with its two rows — never one line
    /// for both. Kills `note_unread`'s `at.slot == slot` guard → `true`,
    /// which folded May's rows into April's line.
    #[test]
    fn bars_series_two_months_that_will_not_open_are_two_faults() {
        let scratch = Scratch::new("series-two-unopened");
        for m in [4, 5] {
            write_month(
                scratch.root(),
                SEGMENT,
                SYMBOL,
                Timeframe::MINUTE_1,
                month(m),
                &[(open_of(m, 2), 10), (open_of(m, 2) + 60, 11)],
            );
        }
        let free = window(scratch.root(), &query(month(4), month(5))).expect("answers");
        assert_eq!(free.total, 4);
        let _april = hold(scratch.root(), 4);
        let _may = hold(scratch.root(), 5);
        let mut asked = query(month(4), month(5));
        asked.sort = SortKey::TimeOfDay;
        let got = window(scratch.root(), &asked).expect("months that will not open are faults");
        let unopened: Vec<&String> = got
            .faults
            .iter()
            .filter(|fault| fault.contains("the month did not open"))
            .collect();
        assert_eq!(unopened.len(), 2, "{:?}", got.faults);
        assert!(
            unopened[0].contains("2025-04") && unopened[0].contains("none of its 2 kept rows"),
            "{unopened:?}"
        );
        assert!(
            unopened[1].contains("2025-05") && unopened[1].contains("none of its 2 kept rows"),
            "{unopened:?}"
        );
    }

    /// **AN EDGE MONTH A DAY BOUND LEAVES NO ROW OF IS NO RUN**, so the next
    /// month's first row has no kept row behind it. April's only row is
    /// before the 20th, the bound's first day: with the lookback off, May's
    /// first row is the first in its window, never measured against April.
    /// Kills `cut_months`' `end > start` → `>=`, which kept an empty April
    /// run that made April's last row look kept.
    #[test]
    fn bars_series_an_edge_month_the_bound_empties_is_no_run() {
        let scratch = Scratch::new("series-empty-edge");
        for (m, close) in [(4, 100), (5, 200)] {
            write_month(
                scratch.root(),
                SEGMENT,
                SYMBOL,
                Timeframe::MINUTE_1,
                month(m),
                &[(open_of(m, 2), close)],
            );
        }
        let mut asked = query(month(4), month(5));
        asked.lookback = false;
        asked.filter.from = Some((open_of(4, 20) - 33_300) * 1_000_000);
        let got = window(scratch.root(), &asked).expect("answers");
        let rows: Vec<(i64, Option<i64>, &str)> = got
            .bars
            .iter()
            .map(|row| (row.bar.close, row.chg, row.chg_why))
            .collect();
        assert_eq!(rows, vec![(200, None, "first_bar_in_window")]);
        assert_eq!(got.total, 1);
    }

    /// **A RUNG DIRECTORY THAT CANNOT BE ASKED ABOUT IS A REFUSAL, NOT AN
    /// EMPTY HISTORY.** The symbol's path is a file, so the rung directory
    /// beneath it is not a directory that is absent: it cannot be read.
    #[test]
    fn bars_series_a_rung_directory_that_cannot_be_read_is_refused() {
        let scratch = Scratch::new("series-not-a-dir");
        let symbol_dir = month_path(scratch.root(), 5)
            .parent()
            .and_then(std::path::Path::parent)
            .expect("the symbol directory")
            .to_path_buf();
        std::fs::create_dir_all(symbol_dir.parent().expect("its parent")).expect("parents");
        std::fs::write(&symbol_dir, b"a file where a directory should be").expect("a file");
        let why = window(scratch.root(), &query(month(5), month(5))).expect_err("refused");
        assert!(why.starts_with("cannot read "), "{why}");
    }

    /// **A MONTH LISTED AND GONE BEFORE ITS STAT IS AN ABSENCE, AND A HELD
    /// MONTH WITH NO RECORD KEEPS NOTHING AND COVERS NO DAY.** A dangling link
    /// stands in for a file removed between the listing and the stat. Runs
    /// the plan's gone-file count, its skip of an empty month (with and
    /// without a time-of-day bound), the empty month's zero coverage and the
    /// coverage memo's hit.
    #[test]
    fn bars_series_a_gone_month_is_absent_and_an_empty_one_keeps_nothing() {
        let scratch = Scratch::new("series-gone-empty");
        write_month(
            scratch.root(),
            SEGMENT,
            SYMBOL,
            Timeframe::MINUTE_1,
            month(4),
            &[(open_of(4, 2), 10)],
        );
        empty_month(scratch.root(), 5);
        let june = month_path(scratch.root(), 6);
        std::os::unix::fs::symlink(scratch.root().join("nowhere.bin"), &june)
            .expect("a dangling link");
        let mut asked = query(month(4), month(6));
        asked.coverage = true;
        for _ in 0..2 {
            let got = window(scratch.root(), &asked).expect("answers");
            assert_eq!(got.total, 1);
            assert_eq!(got.months_read, 2, "April and the empty May");
            assert_eq!(got.months_outside_life, 1, "June went before its stat");
            let covered: Vec<(u8, u64, u32)> = got
                .coverage
                .iter()
                .map(|one| (one.month.month(), one.rows, one.days_covered))
                .collect();
            assert_eq!(covered, vec![(4, 1, 1), (5, 0, 0)]);
        }
        asked.coverage = false;
        asked.filter.tod = Some((33_300, 33_400));
        let got = window(scratch.root(), &asked).expect("answers");
        assert_eq!(
            got.total, 1,
            "the empty month keeps nothing under a bound either"
        );
    }

    /// **WITHOUT THE LOOKBACK, A ROW INSIDE ITS RUN STILL MEASURES AGAINST
    /// THE KEPT ROW BEFORE IT, AND EACH MONTH'S FIRST ROW AFTER AN UNKEPT ONE
    /// IS FIRST IN ITS WINDOW.** Six months, each with a kept 09:20 and an
    /// unkept 09:30: a page from offset 1 with no bound measures its first
    /// row against the row before it in the file, and under 09:20–09:21 every
    /// month after the first is found among the request's months by the slot
    /// bisection, from either half. Runs `previous_is_kept`'s inside-the-run
    /// answer and `slot_of`'s right-half step, which no test reached. Six
    /// slots make June's search for May narrow to the first three and then
    /// step right from the second: `slot_of`'s `mid + 1` → `mid - 1` never
    /// leaves that range, and the request never ends.
    #[test]
    fn bars_series_without_the_lookback_kept_rows_still_measure() {
        let scratch = Scratch::new("series-no-lookback");
        for m in 3..=8 {
            write_month(
                scratch.root(),
                SEGMENT,
                SYMBOL,
                Timeframe::MINUTE_1,
                month(m),
                &[
                    (open_of(m, 2) + 300, i64::from(m) * 100),
                    (open_of(m, 2) + 900, i64::from(m) * 100 + 1),
                ],
            );
        }
        let mut asked = query(month(3), month(8));
        asked.lookback = false;
        asked.offset = Some(1);
        asked.limit = 1;
        let got = window(scratch.root(), &asked).expect("answers");
        let first: Vec<(i64, Option<i64>)> = got
            .bars
            .iter()
            .map(|row| (row.bar.close, row.chg))
            .collect();
        assert_eq!(first, vec![(301, Some(33))], "09:30 against 09:20, kept");
        asked.offset = Some(0);
        asked.limit = 50;
        asked.filter.tod = Some((33_600, 33_660));
        let got = window(scratch.root(), &asked).expect("answers");
        let whys: Vec<(i64, &str)> = got
            .bars
            .iter()
            .map(|row| (row.bar.close, row.chg_why))
            .collect();
        assert_eq!(
            whys,
            vec![
                (300, "no_earlier_stored_bar"),
                (400, "first_bar_in_window"),
                (500, "first_bar_in_window"),
                (600, "first_bar_in_window"),
                (700, "first_bar_in_window"),
                (800, "first_bar_in_window"),
            ]
        );
    }

    /// **A MONTH'S COVERED DAYS ARE THE DAYS THAT HOLD A RECORD.** May holds
    /// records on the 2nd and the 6th only: the three days between are in its
    /// span and hold none, so `coverage=1` counts two days, not five. Kills
    /// `days_covered`'s `to > from` → `>=`, which counted every day of the
    /// span.
    #[test]
    fn bars_series_a_month_covers_only_the_days_that_hold_a_record() {
        let scratch = Scratch::new("series-days-covered");
        write_month(
            scratch.root(),
            SEGMENT,
            SYMBOL,
            Timeframe::MINUTE_1,
            month(5),
            &[
                (open_of(5, 2), 10),
                (open_of(5, 2) + 60, 11),
                (open_of(5, 6), 12),
            ],
        );
        let mut asked = query(month(5), month(5));
        asked.coverage = true;
        let got = window(scratch.root(), &asked).expect("answers");
        let covered: Vec<(u64, u32)> = got
            .coverage
            .iter()
            .map(|one| (one.rows, one.days_covered))
            .collect();
        assert_eq!(covered, vec![(3, 2)]);
    }

    /// **A MONTH WHOSE FILE WILL NOT STAT, AND ONE WHOSE GENERATION POINTER
    /// IS NOT A FILE, ARE EACH ONE FAULT BY NAME**, and the months that read
    /// are still answered. May's name is a link to itself (the kernel's
    /// "too many levels of symbolic links", which is not an absence); June's
    /// pointer name is a directory, so which generation of June is served
    /// cannot be told — and serving the original anyway would be the hidden
    /// fallback `CLAUDE.md` §4 bans. Runs `header_of`'s stat refusal and
    /// `admit`'s served-path refusal, which no test reached.
    #[test]
    fn bars_series_a_month_that_will_not_stat_or_be_resolved_is_one_fault() {
        let scratch = Scratch::new("series-stat-pointer");
        for m in [4, 6] {
            write_month(
                scratch.root(),
                SEGMENT,
                SYMBOL,
                Timeframe::MINUTE_1,
                month(m),
                &[(open_of(m, 2), 10)],
            );
        }
        let may = month_path(scratch.root(), 5);
        std::os::unix::fs::symlink(&may, &may).expect("a link to itself");
        let june = StorePath::new(PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: SEGMENT,
            symbol: SYMBOL,
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: month(6),
            file: FileKind::Bars,
        })
        .expect("a legal path");
        std::fs::create_dir_all(store::generation::pointer_path(scratch.root(), june))
            .expect("a directory at June's pointer name");
        let got = window(scratch.root(), &query(month(4), month(6))).expect("answers");
        assert_eq!((got.total, got.months_read), (1, 1), "April alone reads");
        assert_eq!(got.faults.len(), 2, "{:?}", got.faults);
        assert!(
            got.faults[0].starts_with(&format!("2025-05: cannot read {}: ", may.display())),
            "{:?}",
            got.faults
        );
        assert!(
            got.faults[1].starts_with("2025-06: ") && got.faults[1].contains("not a regular file"),
            "{:?}",
            got.faults
        );
    }

    /// **WITH THE LOOKBACK OFF, A MONTH'S FIRST ROW IS MEASURED AGAINST THE
    /// MONTH BEFORE WHEN THAT MONTH IS KEPT WHOLE, AND A DAY'S FIRST ROW
    /// UNDER A TIME-OF-DAY BOUND IS NOT.** Runs `previous_is_kept` across a
    /// month boundary and inside one file.
    #[test]
    fn bars_series_the_lookback_measures_only_against_a_kept_row() {
        let scratch = Scratch::new("series-kept-previous");
        for (m, close) in [(4, 100), (5, 110)] {
            write_month(
                scratch.root(),
                SEGMENT,
                SYMBOL,
                Timeframe::MINUTE_1,
                month(m),
                &[(open_of(m, 2), close), (open_of(m, 2) + 60, close + 1)],
            );
        }
        let mut asked = query(month(4), month(5));
        asked.lookback = false;
        let got = window(scratch.root(), &asked).expect("answers");
        let mays_first = &got.bars[2];
        assert_eq!(mays_first.bar.close, 110);
        assert_eq!(mays_first.chg, Some(891), "110 against April's last, 101");

        // Two days of one month under a bound keeping 09:15 alone: the second
        // day's 09:15 follows the first day's 09:16, which is not kept.
        let scratch = Scratch::new("series-kept-tod");
        write_month(
            scratch.root(),
            SEGMENT,
            SYMBOL,
            Timeframe::MINUTE_1,
            month(5),
            &[
                (open_of(5, 2), 100),
                (open_of(5, 2) + 60, 101),
                (open_of(5, 5), 102),
            ],
        );
        let mut asked = query(month(5), month(5));
        asked.lookback = false;
        asked.filter.tod = Some((33_300, 33_300));
        let got = window(scratch.root(), &asked).expect("answers");
        let closes: Vec<(i64, &str)> = got
            .bars
            .iter()
            .map(|row| (row.bar.close, row.chg_why))
            .collect();
        assert_eq!(
            closes,
            vec![(100, "no_earlier_stored_bar"), (102, "first_bar_in_window")]
        );
    }

    /// **A ROW'S PREDECESSOR IS THE LAST RECORD BEFORE IT, WALKING BACK PAST
    /// HELD MONTHS THAT HOLD NOTHING**, inside the range or outside it. Runs
    /// `last_before`'s empty-month walk.
    #[test]
    fn bars_series_the_predecessor_walks_back_past_empty_months() {
        let scratch = Scratch::new("series-walk-back");
        write_month(
            scratch.root(),
            SEGMENT,
            SYMBOL,
            Timeframe::MINUTE_1,
            month(3),
            &[(open_of(3, 3), 100)],
        );
        empty_month(scratch.root(), 4);
        write_month(
            scratch.root(),
            SEGMENT,
            SYMBOL,
            Timeframe::MINUTE_1,
            month(5),
            &[(open_of(5, 2), 105)],
        );
        // April outside the range, and inside it.
        for from in [month(5), month(4)] {
            let got = window(scratch.root(), &query(from, month(5))).expect("answers");
            let may = got.bars.last().expect("May's row");
            assert_eq!(may.bar.close, 105);
            assert_eq!(may.chg, Some(500), "105 against March's 100: {from}");
        }
    }

    /// **AN INSTANT BEFORE ITS DAY'S KEPT RUN IS RANKED WITHOUT OPENING THE
    /// MONTH.** Under 09:20–09:21 May keeps the 5th's 09:20 and 09:21; `at`
    /// 09:17 of the 5th lies inside May's records and inside that day's
    /// bucket but before its run, so its index is the run's first, with no
    /// lookup. The day runs are memoised from a request made while May was
    /// free, so with May held by another writer the plan is still built and
    /// `at` still answers. Kills `rank`'s `|| with &&`, which went on to
    /// locate 09:17 in the held month.
    #[test]
    fn bars_series_an_instant_before_its_days_run_is_ranked_unopened() {
        let scratch = Scratch::new("series-at-before-run");
        write_month(
            scratch.root(),
            SEGMENT,
            SYMBOL,
            Timeframe::MINUTE_1,
            month(5),
            &[
                (open_of(5, 2), 10),
                (open_of(5, 5), 11),
                (open_of(5, 5) + 300, 12),
                (open_of(5, 5) + 360, 13),
            ],
        );
        let mut asked = query(month(5), month(5));
        asked.filter.tod = Some((33_600, 33_660));
        let free = window(scratch.root(), &asked).expect("answers");
        assert_eq!(free.total, 2);
        let _held = hold(scratch.root(), 5);
        asked.at = Some((open_of(5, 5) + 120) * 1_000_000);
        let got =
            window(scratch.root(), &asked).expect("an instant before the run locates nothing");
        assert_eq!(got.at_index, Some(0));
    }

    /// **`at` ON A FILE'S LAST RECORD COUNTS ONLY THE ROWS BEFORE IT.** May
    /// holds three rows; `at` its last row's own stamp is index 2 ascending
    /// (two rows strictly before it) and 0 descending. Kills `rank`'s
    /// header check `ts > last_ts` → `>=` and `==`, which counted the last
    /// row as before an instant equal to it.
    #[test]
    fn bars_series_at_the_last_record_counts_only_the_rows_before_it() {
        let scratch = Scratch::new("series-at-last");
        write_month(
            scratch.root(),
            SEGMENT,
            SYMBOL,
            Timeframe::MINUTE_1,
            month(5),
            &[
                (open_of(5, 2), 10),
                (open_of(5, 2) + 60, 11),
                (open_of(5, 2) + 120, 12),
            ],
        );
        for (desc, want) in [(false, 2), (true, 0)] {
            let mut asked = query(month(5), month(5));
            asked.at = Some((open_of(5, 2) + 120) * 1_000_000);
            asked.desc = desc;
            let got = window(scratch.root(), &asked).expect("answers");
            assert_eq!(got.at_index, Some(want), "desc {desc}");
        }
    }

    /// **AN INSTANT PAST THE CALENDAR'S LAST YEAR IS AFTER EVERY ROW, AND ONE
    /// BEFORE THE EPOCH IS BEFORE EVERY ROW.** `at=` takes any whole number of
    /// unix seconds, so 10000-01-01 reaches the window; the calendar names no
    /// day past 9999. Ascending, every row is before it (index 2 of 2) and
    /// descending none is after it (index 0); two days before the epoch —
    /// an IST day before day 0, as no instant within five and a half hours
    /// of the epoch is — is the mirror. Before the fix the far instant ranked
    /// as BEFORE every row — `bucket_of` answered `-1`, "before the first
    /// month", for any day it could not name — so ascending said 0 and
    /// descending 2, and the page served for `at=` was the wrong end of the
    /// window. Kills `bucket_of`'s `delete -`, which made the pre-epoch `-1`
    /// a `1` and so ranked it inside May.
    #[test]
    fn bars_series_an_instant_past_the_calendar_is_after_every_row() {
        let scratch = Scratch::new("series-at-far");
        for m in [4, 5] {
            write_month(
                scratch.root(),
                SEGMENT,
                SYMBOL,
                Timeframe::MINUTE_1,
                month(m),
                &[(open_of(m, 2), 10)],
            );
        }
        let year_10000 = 253_402_300_800_i64 * 1_000_000;
        let before_epoch = -2 * 86_400 * 1_000_000;
        for (at, desc, want) in [
            (year_10000, false, 2),
            (year_10000, true, 0),
            (before_epoch, false, 0),
            (before_epoch, true, 2),
        ] {
            let mut asked = query(month(4), month(5));
            asked.at = Some(at);
            asked.desc = desc;
            let got = window(scratch.root(), &asked).expect("answers");
            assert_eq!(got.at_index, Some(want), "at {at} desc {desc}");
        }
    }

    /// **`at` NAMES THE GLOBAL INDEX OF AN INSTANT INSIDE A MONTH'S RECORDS,
    /// OF ONE IN A LATER MONTH WITH NOTHING STORED, AND OF ONE PAST THE
    /// RANGE.** Runs `rank`'s locate inside a run, its empty-bucket answer and
    /// its past-the-last-bucket answer.
    #[test]
    fn bars_series_at_ranks_inside_after_and_past_the_rows() {
        let scratch = Scratch::new("series-rank");
        let rows: Vec<(i64, i64)> = (0..5).map(|i| (open_of(4, 2) + i * 60, 10 + i)).collect();
        write_month(
            scratch.root(),
            SEGMENT,
            SYMBOL,
            Timeframe::MINUTE_1,
            month(4),
            &rows,
        );
        let at = |instant: i64| {
            let mut asked = query(month(4), month(6));
            asked.at = Some(instant);
            asked.offset = None;
            window(scratch.root(), &asked)
                .expect("answers")
                .at_index
                .expect("an index")
        };
        assert_eq!(
            at((open_of(4, 2) + 150) * 1_000_000),
            3,
            "between the third and fourth"
        );
        assert_eq!(at(open_of(5, 5) * 1_000_000), 5, "May holds nothing");
        assert_eq!(
            at(open_of(6, 2) * 1_000_000 + 40 * 86_400_000_000),
            5,
            "past June"
        );
    }
}

/// `bars_sort.rs`: the value page from sort indexes, through the public
/// window.
mod bars_sort_rs {
    use super::{Scratch, may, open_second, write_month};
    use api::bars::{Filter, SeriesKey, SortKey, WindowQuery, window};
    use brutex_core::vendor::Vendor;
    use store::path::Timeframe;

    fn close_page(series: SeriesKey) -> WindowQuery {
        WindowQuery {
            vendor: Vendor::Dhan,
            timeframe: Timeframe::MINUTE_1,
            series: vec![series],
            from: may(),
            to: may(),
            sort: SortKey::Close,
            desc: false,
            offset: Some(0),
            limit: 50,
            want_extremes: false,
            at: None,
            filter: Filter::default(),
            lookback: true,
            coverage: false,
        }
    }

    /// **A DIRECTORY AT AN INDEX'S NAME IS NOT A REGULAR FILE, AND ITS MONTH
    /// IS READ.** The `.srt` name is taken by a directory: the month's rows
    /// are ordered by reading them, the reason names the name and what it is,
    /// and the command that would index the month is offered. Runs `view`'s
    /// "not a regular file" arm, which no test reached (a symbolic link was
    /// the only non-file tried).
    #[test]
    fn bars_sort_a_directory_at_an_index_name_is_read_and_named() {
        let scratch = Scratch::new("sort-srt-directory");
        let open = open_second();
        let bars = write_month(
            scratch.root(),
            "INDEX",
            "NIFTY",
            Timeframe::MINUTE_1,
            may(),
            &[(open, 30), (open + 60, 10), (open + 120, 20)],
        );
        let srt = bars.with_extension("srt");
        std::fs::create_dir(&srt).expect("a directory at the index's name");
        let got = window(
            scratch.root(),
            &close_page(SeriesKey::new("NSE", "INDEX", "NIFTY", None)),
        )
        .expect("the month is read, not refused");
        let closes: Vec<i64> = got.bars.iter().map(|row| row.bar.close).collect();
        assert_eq!(closes, vec![10, 20, 30]);
        let report = got.sort_index.expect("a value sort says what it used");
        assert_eq!((report.indexed_rows, report.scanned_rows), (0, 3));
        assert_eq!(report.scanned_why.len(), 1, "{:?}", report.scanned_why);
        let why = &report.scanned_why[0];
        assert!(
            why.contains(&format!("{} is not a regular file", srt.display()))
                && why.starts_with("NSE-INDEX-NIFTY 2025-05: 3 rows read: "),
            "{why}"
        );
        assert_eq!(
            report.build,
            vec!["cli srt-build dhan NSE/INDEX/NIFTY 1min 2025-05 2025-05".to_owned()],
            "a directory is not the small-month rule: a rebuild would index it"
        );
    }
}

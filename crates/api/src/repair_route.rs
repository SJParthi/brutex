//! `/pull/repair.json` and `POST /pull/repair`: the versioned repair of a
//! series' months, for the `/ingest` page. D-3350, WP-13c; findings DB-N20,
//! DB-N22.
//!
//! # Why a route at all
//!
//! The append-only store refuses a session before a month's last held bar for
//! ever: Zerodha NIFTY 1min lacks 2020-12-01..15 because 2020-12 holds the 16th
//! on, and lacks 42 sessions in ten months in all (35 regular sessions in four
//! runs and seven Muhurats). Measured read-only on a shadow of the operator's
//! store on 10 Oct 2026 (`cli zerodha-repair ALL`), 190 of the 330 Zerodha
//! 1min series that hold bars lack exactly those 42 (21 of 124 indices, 169 of
//! 206 shares), and 300 lack at least one session only a repair fills. No
//! `/pull` body could fill them — each would be refused and write nothing. The repair door publishes a NEW generation of the
//! month (held bars kept, never changed; offered bars added) and every reader
//! opens it.
//!
//! - `GET /pull/repair.json?feed&instrument&segment&rung&from&to` answers what
//!   a repair would do, READ-ONLY: per month, the generation served, the
//!   sessions of record, which hold a bar, and each that does not, classed
//!   `front` / `interior` (only a repair fills it) or `tail` / `absent` (an
//!   ordinary pull does); at the minute rung, each session holding a bar but
//!   fewer of its owed minutes than the calendar owes (`short`, R3-18), classed
//!   the same way; the `POST /pull/repair` body that fills them; and the
//!   repair journal's lines for the series.
//! - `POST /pull/repair` is `POST /pull/spot`'s form plus `repair_reason`: the
//!   same credential, budget, chunking and landing, with the repair door at the
//!   write (`pull::ingest::repair_window`). A Zerodha pull needs a Kite access
//!   token valid on the day; none is minted here (`CLAUDE.md` §8).
//!
//! # Cost
//!
//! The plan is O(days asked): one generation probe and one open per month, two
//! time-indexed lookups per session, and at the minute rung two more per
//! trading window and at most two record reads per short session
//! (`pull::ingest::repair_plan`, R3-18); one listing of the rung directory per
//! answer, O(months held). The journal
//! read is bounded by [`MAX_JOURNAL_BYTES`] from its tail. Measured on a shadow
//! of the operator's store in the WP-13c report.

use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::Path;

use pull::ingest::repair_plan;
use pull::session::{Day, Window};
use store::path::{FileKind, PathParts, Timeframe};

use super::{Loaded, param};
use crate::ingest;

/// The widest window one plan answers, in days: the same ten years a pull
/// form takes (`ingest::MAX_WINDOW_DAYS`).
const MAX_PLAN_DAYS: u32 = 3_653;

/// The most bytes of the repair journal's tail one answer reads.
const MAX_JOURNAL_BYTES: u64 = 64 * 1024;

/// The answer's three parts, as the router wants them.
type JsonAnswer = (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

fn answer(status: axum::http::StatusCode, body: &serde_json::Value) -> JsonAnswer {
    (
        status,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        body.to_string(),
    )
}

fn refused(why: &str) -> JsonAnswer {
    answer(
        axum::http::StatusCode::BAD_REQUEST,
        &serde_json::json!({ "error": why }),
    )
}

/// `GET /pull/repair.json`. See the module header.
pub(super) async fn repair_plan_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> JsonAnswer {
    let query = uri.query().unwrap_or("").to_owned();
    let root = site.store_root.clone();
    // Store reads, off the async workers, IN THE BOUNDED STORE-READ POOL
    // (R3-08, RA-088). It ran on a spare thread outside every pool: a burst
    // started as many threads as it had requests, with no busy answer and no
    // count on any page. Now it waits for a slot like every store read, is
    // refused 429 with `Retry-After` past the wait, and is counted.
    crate::detail::run_store_read(move || plan_answer(&root, &query))
        .await
        .unwrap_or_else(|why| {
            let (status, body) = crate::detail::admission_refused(
                "a month repair plan",
                crate::detail::MAX_STORE_READ_CONCURRENT,
                &why,
            );
            (
                status,
                [(
                    axum::http::header::CONTENT_TYPE,
                    "application/json; charset=utf-8",
                )],
                body,
            )
        })
}

/// The plan for one query, synchronously: the route's whole answer.
pub(super) fn plan_answer(root: &Path, query: &str) -> JsonAnswer {
    let feed_text = param(query, "feed");
    if feed_text.is_empty() {
        return refused("name the feed: feed=zerodha");
    }
    let Some(vendor) = ingest::parse_vendor(&feed_text) else {
        return refused(&format!("{feed_text:?} is not a feed this build stores"));
    };
    let instrument = param(query, "instrument").trim().to_ascii_uppercase();
    if brutex_core::symbol::Symbol::new(&instrument).is_err() {
        return refused(&format!("{instrument:?} is not a store symbol"));
    }
    let segment = match param(query, "segment").to_ascii_uppercase().as_str() {
        "" | "INDEX" => "INDEX",
        "CASH" => "CASH",
        other => {
            return refused(&format!(
                "segment {other:?}: a repair plan reads INDEX or CASH"
            ));
        }
    };
    let rung_text = param(query, "rung");
    let mut timeframe = if rung_text.is_empty() {
        Some(Timeframe::MINUTE_1)
    } else {
        None
    };
    for known in Timeframe::KNOWN {
        if known.as_str() == rung_text {
            timeframe = Some(*known);
        }
    }
    let Some(timeframe) = timeframe else {
        return refused(&format!("rung {rung_text:?} is not one this store files"));
    };
    let window = match (
        ingest::parse_day_field(query, "from"),
        ingest::parse_day_field(query, "to"),
    ) {
        (Ok(from), Ok(to)) => match Window::new(from, to) {
            Ok(window) if window.days() <= MAX_PLAN_DAYS => window,
            Ok(window) => {
                return refused(&format!(
                    "{} days asked; one plan answers at most {MAX_PLAN_DAYS}",
                    window.days()
                ));
            }
            Err(why) => return refused(&why.to_string()),
        },
        (Err(why), _) | (_, Err(why)) => return refused(&why.to_string()),
    };
    let Ok(first_month) = window.from().year_month() else {
        return refused("the window's first month is outside the store's calendar");
    };
    let parts = PathParts {
        vendor,
        exchange: "NSE",
        segment,
        symbol: &instrument,
        contract: None,
        timeframe,
        month: first_month,
        file: FileKind::Bars,
    };
    let months = repair_plan::plan(root, parts, window);
    let key = format!("NSE-{segment}-{instrument}");
    answer(
        axum::http::StatusCode::OK,
        &render(
            root,
            &months,
            (vendor, &key, segment, timeframe, window),
            &instrument,
        ),
    )
}

fn day_text(day: Day) -> String {
    day.to_string()
}

fn stamp_text(micros: i64) -> String {
    let secs = micros.div_euclid(1_000_000) + pull::session::IST_OFFSET_SECS;
    let days = secs.div_euclid(86_400);
    let minute = secs.rem_euclid(86_400) / 60;
    let day = u32::try_from(days)
        .ok()
        .and_then(|days| Day::from_days(days).ok())
        .map_or_else(|| "-".to_owned(), |day| day.to_string());
    format!("{day} {:02}:{:02} IST", minute / 60, minute % 60)
}

fn render(
    root: &Path,
    months: &[repair_plan::Month],
    (vendor, key, segment, timeframe, window): (
        brutex_core::vendor::Vendor,
        &str,
        &str,
        Timeframe,
        Window,
    ),
    instrument: &str,
) -> serde_json::Value {
    let mut ranges = Vec::new();
    let (mut sessions, mut held, mut missing, mut repair_sessions, mut repair_months) =
        (0u32, 0u32, 0usize, 0usize, 0usize);
    let mut before_first = 0usize;
    // SHORT SESSIONS (R3-18): held, but fewer of their owed minutes than the
    // calendar owes; a front or interior one only a repair fills.
    let (mut short, mut short_minutes, mut repair_short) = (0usize, 0u64, 0usize);
    for month in months {
        sessions += month.sessions;
        held += month.held;
        missing += month.missing.len();
        short += month.short.len();
        short_minutes += month
            .short
            .iter()
            .map(|s| u64::from(s.missing()))
            .sum::<u64>();
        repair_short += month.short.iter().filter(|s| s.fill.needs_repair()).count();
        before_first += month
            .missing
            .iter()
            .filter(|(_, why)| *why == repair_plan::Missing::BeforeFirst)
            .count();
        repair_sessions += month
            .missing
            .iter()
            .filter(|(_, why)| why.needs_repair())
            .count();
        if month.needs_repair() {
            repair_months += 1;
            // THE WHOLE MONTH, so the offered bars include held ones: the
            // witness a back-adjusted feed's repair needs, and one Kite
            // request at the 60-day cap (D-3351).
            if let Ok(first) = Day::new(month.month.year(), month.month.month(), 1) {
                ranges.push((
                    first.max(window.from()),
                    first.end_of_month().min(window.to()),
                ));
            }
        }
    }
    serde_json::json!({
        "feed": vendor.as_str(),
        "instrument": key,
        "rung": timeframe.as_str(),
        "from": day_text(window.from()),
        "to": day_text(window.to()),
        "calendar": "pull::calendar::exchange_kind_of, the calendar of record /calendar.json reads (D-3336)",
        "absence_may_be_no_trade": repair_plan::absence_may_be_no_trade(segment),
        "short_rule": if repair_plan::absence_may_be_no_trade(segment) {
            "a short session holds fewer of its owed minutes than the calendar owes (halts excluded, R3-18); for a share a minute without a candle may be a minute nobody traded (D-3321), which the store cannot tell from a hole: a repair lands only what the feed serves"
        } else {
            "a short session holds fewer of its owed minutes than the calendar owes (halts excluded, R3-18); an index is computed every minute, so each is a hole"
        },
        "months": months.iter().map(month_json).collect::<Vec<_>>(),
        "totals": {
            "sessions": sessions,
            "held": held,
            "missing": missing,
            "repair_sessions": repair_sessions,
            "repair_months": repair_months,
            "before_first": before_first,
            "short_sessions": short,
            "short_minutes": short_minutes,
            "repair_short_sessions": repair_short,
        },
        "repair": repair_json(&ranges, (vendor, segment, timeframe), instrument),
        "journal": journal_lines(root, vendor, key),
    })
}

/// One month of the plan, as the page reads it.
fn month_json(month: &repair_plan::Month) -> serde_json::Value {
    serde_json::json!({
        "month": month.month.to_string(),
        "generation": month.generation,
        "bars": month.bars,
        "first": month.span.map(|(first, _)| stamp_text(first)),
        "last": month.span.map(|(_, last)| stamp_text(last)),
        "sessions": month.sessions,
        "held": month.held,
        "missing": month.missing.iter().map(|(day, why)| serde_json::json!({
            "day": day_text(*day),
            "fill": why.code(),
            "by": filled_by(*why),
        })).collect::<Vec<_>>(),
        "short": month.short.iter().map(|s| serde_json::json!({
            "day": day_text(s.day),
            "owed": s.owed,
            "held": s.held,
            "missing": s.missing(),
            "fill": s.fill.code(),
            "by": filled_by(s.fill),
        })).collect::<Vec<_>>(),
        "length_unjudged": month.length_unjudged.iter().map(|day| day_text(*day)).collect::<Vec<_>>(),
        "unattested": month.unattested.iter().map(|day| day_text(*day)).collect::<Vec<_>>(),
        "repair_needed": month.needs_repair(),
        "refused": month.refused,
    })
}

/// What can fill a missing session, in the page's words.
const fn filled_by(why: repair_plan::Missing) -> &'static str {
    match why {
        repair_plan::Missing::Front | repair_plan::Missing::Interior => "versioned repair",
        repair_plan::Missing::Tail | repair_plan::Missing::Absent => "ordinary pull",
        repair_plan::Missing::BeforeFirst => {
            "before the series' first held bar: a listing, the start of the feed's history, or a hole; a repair of the month lands whatever the feed serves"
        }
    }
}

/// What fills the months a repair must: one `POST /pull/repair` body per
/// month, and for a derived rung the refold that needs no vendor.
fn repair_json(
    ranges: &[(Day, Day)],
    (vendor, segment, timeframe): (brutex_core::vendor::Vendor, &str, Timeframe),
    instrument: &str,
) -> serde_json::Value {
    let target = if segment == "CASH" {
        "equities"
    } else {
        "indices"
    };
    // A DERIVED RUNG IS NEVER PULLED: its months are refolded from the
    // repaired minutes (D-3321, D-3350), so its body asks for the minute rung,
    // and a refold of minutes already held needs no vendor at all.
    let derived = derived_rung(timeframe);
    let pulled = if derived {
        Timeframe::MINUTE_1
    } else {
        timeframe
    };
    let bodies: Vec<String> = ranges
        .iter()
        .map(|(from, to)| {
            format!(
                "target={target}&vendor={}&granularity={}&cash_identity=zerodha_nse_checked&member={instrument}&from={from}&to={to}&repair_reason=<why>",
                vendor.as_str(),
                pulled.as_str(),
            )
        })
        .collect();
    let refold: Vec<String> = ranges
        .iter()
        .filter(|_| derived)
        .map(|(from, to)| {
            format!(
                "cli zerodha-repair {instrument} {from} {to} --segment {segment} --from-store <this store> --reason <why>"
            )
        })
        .collect();
    serde_json::json!({
        "route": "POST /pull/repair",
        "rule": "D-3350: a new generation of the month, every held bar kept and never changed, offered bars added, served to every reader once promoted; journaled in audit/<feed>.repairs",
        "ranges": ranges.iter().map(|(from, to)| serde_json::json!({
            "from": day_text(*from),
            "to": day_text(*to),
        })).collect::<Vec<_>>(),
        "bodies": bodies,
        "derived": derived.then_some(
            "a derived rung is refolded from the month's minutes: where the minutes are held, `refold` repairs it from this store with no vendor; where they are missing too, the minute repair above refolds it"
        ),
        "refold": refold,
        "needs": "a Kite access token valid on the day of the pull: Kite issues one per login session and this repository never mints one (CLAUDE.md section 8); whether today's is valid is UNVERIFIED here",
        "missing_why": {
            "front": "before the month's first held bar: the append-only store refuses it; only a versioned repair fills it",
            "interior": "between the month's held bars: likewise",
            "tail": "after the month's last held bar: an ordinary pull appends it",
            "absent": "the month holds nothing: an ordinary pull creates it",
            "before_first": "before the first bar of the series' first held month: a listing, the start of the feed's history, or a hole an earlier refused answer left; the bars cannot tell which, so it is never planned as a repair",
        },
    })
}

/// Whether `rung` is folded from minutes rather than pulled: every rung
/// coarser than a minute except the day (`pull::ingest::derived_count`).
fn derived_rung(rung: Timeframe) -> bool {
    rung.secs() > Timeframe::MINUTE_1.secs() && rung.secs() < Timeframe::DAY_1.secs()
}

/// The repair journal's lines for `key`, from at most [`MAX_JOURNAL_BYTES`]
/// of its tail, oldest first; `null` when the journal is not there.
fn journal_lines(root: &Path, vendor: brutex_core::vendor::Vendor, key: &str) -> serde_json::Value {
    let path = root
        .join("audit")
        .join(format!("{}.repairs", vendor.as_str()));
    let Ok(mut file) = std::fs::File::open(&path) else {
        return serde_json::Value::Null;
    };
    let len = file.metadata().map_or(0, |meta| meta.len());
    let from = len.saturating_sub(MAX_JOURNAL_BYTES);
    if file.seek(SeekFrom::Start(from)).is_err() {
        return serde_json::json!({ "error": format!("cannot read {}", path.display()) });
    }
    let mut text = String::new();
    if file
        .take(MAX_JOURNAL_BYTES)
        .read_to_string(&mut text)
        .is_err()
    {
        return serde_json::json!({ "error": format!("{} is not text", path.display()) });
    }
    let needle = format!(" {key} ");
    let lines: Vec<&str> = text
        .lines()
        .skip(usize::from(from > 0))
        .filter(|line| line.contains(&needle))
        .collect();
    serde_json::json!({
        "path": path.display().to_string(),
        "truncated": from > 0,
        "lines": lines,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test-only assertions"
)]
mod tests {
    use super::plan_answer;
    use pull::session::{Day, IST_OFFSET_SECS};
    use store::format::{Bar, OI_NULL};
    use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

    fn bar(day: Day, minute: i64) -> Bar {
        let secs = i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + minute * 60;
        Bar {
            ts_micros: secs * 1_000_000,
            open: 700_000,
            high: 700_009,
            low: 699_991,
            close: 700_003,
            volume: 0,
            open_interest: OI_NULL,
        }
    }

    /// The DB-N20 shape on a fixture store: the plan names the leading hole as
    /// a repair's, gives the whole-month body that fills it, and reads nothing
    /// it should not.
    #[test]
    fn the_plan_names_the_leading_hole_and_the_body_that_fills_it() {
        let root = std::env::temp_dir().join(format!("brutex-REPAIR-ROUTE-{}", std::process::id()));
        let _fresh = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let id = pull::ingest::repair_plan::symbol_id("ROUTEIDX");
        // NOVEMBER HELD, so December is not the series' first month: its
        // leading sessions are a hole, not a listing (`before_first`).
        for (month, day) in [(11, 27), (12, 16)] {
            let path = StorePath::new(PathParts {
                vendor: brutex_core::vendor::Vendor::Zerodha,
                exchange: "NSE",
                segment: "INDEX",
                symbol: "ROUTEIDX",
                contract: None,
                timeframe: Timeframe::MINUTE_1,
                month: YearMonth::new(2020, month).unwrap(),
                file: FileKind::Bars,
            })
            .unwrap();
            let mut file = store::file::BarFile::open_or_create(&root, path, id).unwrap();
            file.append(&[bar(Day::new(2020, month, day).unwrap(), 0)])
                .unwrap();
            drop(file);
        }

        let (status, _, body) = plan_answer(
            &root,
            "feed=zerodha&instrument=routeidx&from=2020-12-01&to=2020-12-31",
        );
        assert_eq!(status, axum::http::StatusCode::OK, "{body}");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["instrument"], "NSE-INDEX-ROUTEIDX");
        let december = &json["months"][0];
        assert_eq!(december["generation"], 0);
        assert_eq!(december["sessions"], 22);
        assert_eq!(december["held"], 1);
        assert_eq!(december["repair_needed"], true);
        assert_eq!(december["missing"][0]["day"], "2020-12-01");
        assert_eq!(december["missing"][0]["fill"], "front");
        assert_eq!(json["totals"]["repair_sessions"], 11);
        assert_eq!(json["totals"]["before_first"], 0);
        // SHORT SESSIONS (R3-18): the 16th holds one bar, at 00:00, and none
        // of the 375 minutes it owes; after the month's last bar, so an
        // ordinary pull's, and served with its minutes.
        assert_eq!(december["short"][0]["day"], "2020-12-16");
        assert_eq!(december["short"][0]["owed"], 375);
        assert_eq!(december["short"][0]["held"], 0);
        assert_eq!(december["short"][0]["missing"], 375);
        assert_eq!(december["short"][0]["fill"], "tail");
        assert_eq!(json["totals"]["short_sessions"], 1);
        assert_eq!(json["totals"]["short_minutes"], 375);
        assert_eq!(json["totals"]["repair_short_sessions"], 0);
        assert_eq!(december["length_unjudged"], serde_json::json!([]));
        assert_eq!(json["repair"]["ranges"][0]["from"], "2020-12-01");
        assert_eq!(json["repair"]["ranges"][0]["to"], "2020-12-31");
        assert!(
            json["repair"]["bodies"][0]
                .as_str()
                .unwrap()
                .contains("vendor=zerodha&granularity=1min&cash_identity=zerodha_nse_checked&member=ROUTEIDX&from=2020-12-01&to=2020-12-31&repair_reason="),
            "{body}"
        );
        assert_eq!(json["journal"], serde_json::Value::Null);

        // REFUSED BY NAME: no feed, a bad rung, a window past ten years.
        for (query, why) in [
            (
                "instrument=X&from=2020-12-01&to=2020-12-31",
                "name the feed",
            ),
            (
                "feed=zerodha&instrument=X&rung=7min&from=2020-12-01&to=2020-12-31",
                "is not one this store files",
            ),
            (
                "feed=zerodha&instrument=X&from=2000-01-01&to=2020-12-31",
                "one plan answers at most",
            ),
        ] {
            let (status, _, body) = plan_answer(&root, query);
            assert_eq!(status, axum::http::StatusCode::BAD_REQUEST, "{query}");
            assert!(body.contains(why), "{query}: {body}");
        }
        let _cleanup = std::fs::remove_dir_all(&root);
    }

    /// Every owed minute of `from..=to` for an index, one invented candle each.
    fn answer(from: Day, to: Day) -> Vec<pull::fetch::RawRow> {
        let mut rows = Vec::new();
        for number in from.days_from_epoch()..=to.days_from_epoch() {
            let kind = pull::session::with_recorded_window(
                i64::from(number),
                pull::calendar::sourced_kind_of(i64::from(number)),
            );
            let pull::calendar::DayKind::Open(session) = kind else {
                continue;
            };
            for minute in (0..1_440_u16).filter(|m| session.expects(*m)) {
                let base = 3_000_000 + i64::from(minute % 71) * 11;
                rows.push(pull::fetch::RawRow {
                    timestamp: i64::from(number) * 86_400 - IST_OFFSET_SECS
                        + i64::from(minute) * 60,
                    open: base,
                    high: base + 20,
                    low: base - 20,
                    close: base + 1,
                    volume: 0,
                    open_interest: None,
                });
            }
        }
        rows
    }

    fn window_of(from: Day, to: Day) -> super::super::BrokerWindow {
        use pull::session::Window;
        use pull::vendor::{Feed, Granularity, Listing, Transport};
        let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
            panic!("an HTTP descriptor");
        };
        super::super::BrokerWindow {
            listing: Listing::Index,
            contract: None,
            unfetched: None,
            before_first_bar: None,
            instrument: "NIFTY".to_owned(),
            origin: "WP-13c test only".to_owned(),
            spec,
            exchange: "NSE",
            segment: "INDEX",
            store_vendor: brutex_core::vendor::Vendor::Zerodha,
            window: Window::new(from, to).unwrap(),
            granularity: Granularity::Minute1,
            bodies: vec![(
                Window::new(from, to).unwrap(),
                pull::fetch::RawWindow {
                    rows: answer(from, to),
                },
            )],
        }
    }

    /// **THE LANDING SWITCH (D-3350).** Through the server's own landing, a
    /// month holding its later days refuses its leading days to an ordinary
    /// pull — and the same answer, landed as a repair, is a new generation
    /// every reader opens.
    #[tokio::test]
    async fn the_server_landing_repairs_only_when_asked() {
        let (site, _) = super::super::tests::readiness_fixture("wp13c-repair-door", "INE002A01018");
        let key = brutex_core::instrument::InstrumentKey::index(
            brutex_core::instrument::Exchange::Nse,
            "NIFTY",
        )
        .unwrap();
        let mut dated = std::collections::HashMap::new();
        let (december_1, held_from, month_end) = (
            Day::new(2020, 12, 1).unwrap(),
            Day::new(2020, 12, 16).unwrap(),
            Day::new(2020, 12, 31).unwrap(),
        );
        let first = super::super::land_spot(
            &window_of(held_from, month_end),
            &key,
            &site,
            &mut dated,
            None,
        )
        .await;
        assert_eq!(first.bars_committed, 11 * 375, "{:#?}", first.failures);

        let whole = window_of(december_1, month_end);
        let ordinary = super::super::land_spot(&whole, &key, &site, &mut dated, None).await;
        assert_eq!(ordinary.bars_committed, 0);
        assert!(
            ordinary.failures.iter().any(|f| f.why.contains("2020-12:")),
            "{:#?}",
            ordinary.failures
        );

        let repaired = super::super::land_spot(
            &whole,
            &key,
            &site,
            &mut dated,
            Some("WP-13c test: fill 2020-12-01..15"),
        )
        .await;
        assert_eq!(
            repaired.bars_committed,
            11 * 375,
            "{:#?}",
            repaired.failures
        );
        assert!(repaired.failures.is_empty(), "{:#?}", repaired.failures);
        let path = StorePath::new(PathParts {
            vendor: brutex_core::vendor::Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2020, 12).unwrap(),
            file: FileKind::Bars,
        })
        .unwrap();
        let id = pull::ingest::repair_plan::symbol_id("NIFTY");
        let file = store::file::BarFile::open_existing(&site.store_root, path, id).unwrap();
        assert_eq!(
            file.header().n_valid,
            22 * 375,
            "every reader opens all of December"
        );

        // AND THE PLAN ROUTE SAYS SO: generation 1, nothing left to repair,
        // the journal line named.
        let (_, _, body) = plan_answer(
            &site.store_root,
            "feed=zerodha&instrument=NIFTY&from=2020-12-01&to=2020-12-31",
        );
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["months"][0]["generation"], 1, "{body}");
        assert_eq!(json["months"][0]["held"], 22, "{body}");
        assert_eq!(json["totals"]["repair_sessions"], 0, "{body}");
        assert!(
            json["journal"]["lines"][0].as_str().unwrap().starts_with(
                "repair NSE-INDEX-NIFTY 1min 2020-12 v=1 from_generation=0 to_generation=1"
            ),
            "{body}"
        );
    }

    /// **THE TWO ROUTES ARE TWO DOORS.** `/pull/spot` refuses a body carrying a
    /// repair reason, and `/pull/repair` refuses one without — each before a
    /// seat is taken or a vendor asked.
    #[tokio::test]
    async fn an_ordinary_pull_never_repairs_and_a_repair_names_its_reason() {
        let (site, _) = super::super::tests::readiness_fixture("wp13c-two-doors", "INE002A01018");
        let site = std::sync::Arc::new(site);
        let body = "target=indices&vendor=zerodha&granularity=1min&member=NIFTY&from=2020-12-01&to=2020-12-31";
        let (status, _, axum::response::Html(page)) = super::super::pull_spot(
            axum::extract::State(std::sync::Arc::clone(&site)),
            format!("{body}&repair_reason=fill"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(
            page.contains("repair_reason belongs to POST /pull/repair"),
            "{page}"
        );
        for reason in ["", "&repair_reason=", "&repair_reason=fill%07line"] {
            let (status, _, axum::response::Html(page)) = super::super::pull_repair(
                axum::extract::State(std::sync::Arc::clone(&site)),
                format!("{body}{reason}"),
            )
            .await;
            assert_eq!(status, axum::http::StatusCode::BAD_REQUEST, "{reason}");
            assert!(page.contains("a repair names why it is run"), "{page}");
        }
    }
}

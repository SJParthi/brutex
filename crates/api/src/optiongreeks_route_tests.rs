#![cfg(test)]
//! `/greeks.json` over generated records and an invented rate table
//! (D-2830, D-2831, D-3328, D-3330). No row here is market data and no rate
//! here is RBI's.
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use super::optiongreeks::{
    MAX_GREEK_TIMES, RATE_ENV, RATE_TENOR_DAYS, RateTable, above_rate_tenor, option_parts,
    rate_loads, respond,
};
use super::*;
use axum::http::StatusCode;
use brutex_core::instrument::{Contract, Expiry, OptionSide};
use pull::pricing::{BarQuote, TradeDay};
use pull::riskfree::Series;
use pull::tenor::{Tenor, YearBasis};
use serde_json::Value;
use std::fs;
use std::sync::Arc;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// `hh:mm` IST on 2026-08-`day`, in UTC seconds.
fn ist(day: u8, hh: i64, mm: i64) -> i64 {
    ist_on(2026, 8, day, hh, mm, 0)
}

/// `hh:mm:ss` IST on `y-m-d`, in UTC seconds.
fn ist_on(y: u16, m: u8, d: u8, hh: i64, mm: i64, ss: i64) -> i64 {
    i64::from(Day::new(y, m, d).expect("generated day").days_from_epoch()) * 86_400 - 19_800
        + hh * 3_600
        + mm * 60
        + ss
}

/// 09:15 IST on 2026-08-20, the instant most bars below open at.
fn t0() -> i64 {
    ist(20, 9, 15)
}

/// The text of an invented weekly table: one price/yield pair the parser's
/// own check accepts, released on each `(year, month, day)`.
fn table_text(days: &[(u16, u8, u8)]) -> String {
    let mut text = String::from(pull::riskfree::HEADER);
    for (y, m, d) in days {
        let _ = write!(
            text,
            "\n\t\t{y:04}-{m:02}-{d:02}\t98.5000\t6.1081\thttps://www.rbi.org.in/invented\t\t"
        );
    }
    text
}

/// An invented table released on each `day` of August 2026.
fn table(days: &[u8]) -> Series {
    let dated: Vec<(u16, u8, u8)> = days.iter().map(|day| (2026, 8, *day)).collect();
    Series::parse(&table_text(&dated)).expect("the invented table parses")
}

/// Bars `(UTC second, close)` for `symbol` (and `contract`, an option's) at
/// `timeframe` in `month`, on Dhan.
fn write_at(
    root: &Path,
    segment: &str,
    symbol: &str,
    contract: Option<&str>,
    timeframe: Timeframe,
    month: YearMonth,
    bars: &[(i64, i64)],
) {
    let contract = contract.map(|c| Contract::parse(c).expect("generated contract"));
    let path = StorePath::new(PathParts {
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment,
        symbol,
        contract,
        timeframe,
        month,
        file: FileKind::Bars,
    })
    .expect("generated path");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the same low 32 bits `api::bars::open_classified` checks"
    )]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    let mut file = store::file::BarFile::open_or_create(root, path, id).expect("generated writer");
    let rows: Vec<Bar> = bars
        .iter()
        .map(|&(secs, close)| Bar {
            ts_micros: secs * 1_000_000,
            open: close,
            high: close,
            low: close,
            close,
            volume: 1,
            open_interest: OI_NULL,
        })
        .collect();
    file.append(&rows).expect("generated records");
}

fn write(root: &Path, segment: &str, contract: Option<&str>, bars: &[(i64, i64)]) {
    write_at(
        root,
        segment,
        "NIFTY",
        contract,
        Timeframe::MINUTE_1,
        YearMonth::new(2026, 8).expect("month"),
        bars,
    );
}

const ATM: &str = "2026-08-27-2500000-CE";
const ITM: &str = "2026-08-27-2400000-CE";
/// A call worth more than the underlying itself: above the model's maximum.
const RICH: &str = "2026-08-27-2450000-CE";
/// A contract running past the rate's 91 days.
const LONG: &str = "2026-12-31-2500000-CE";

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(name);
        fs::create_dir_all(&root).expect("scratch root");
        let t = t0();
        // A GAP IN THE INDEX AT +120 s.
        write(
            &root,
            "INDEX",
            None,
            &[
                (ist(12, 9, 15), 2_500_000),
                (t, 2_500_000),
                (t + 60, 2_501_000),
                (t + 180, 2_499_000),
                (ist(27, 15, 45), 2_500_000),
            ],
        );
        write(
            &root,
            "FNO",
            Some(ATM),
            &[
                (ist(12, 9, 15), 60_000),
                (t, 25_000),
                (t + 60, 25_100),
                (t + 120, 26_000),
                (ist(27, 15, 45), 50),
            ],
        );
        // Deep in the money by 1,000.00 and closing at 1.00: below intrinsic.
        write(&root, "FNO", Some(ITM), &[(t, 100)]);
        write(&root, "FNO", Some(RICH), &[(t, 2_600_000)]);
        write(&root, "FNO", Some(LONG), &[(t, 160_000)]);
        Self { root }
    }

    fn ask(&self, contract: &str, times: &[i64], rate: &RateTable) -> (StatusCode, Value) {
        self.ask_with(contract, times, rate, "")
    }

    fn ask_with(
        &self,
        contract: &str,
        times: &[i64],
        rate: &RateTable,
        extra: &str,
    ) -> (StatusCode, Value) {
        let t: Vec<String> = times.iter().map(i64::to_string).collect();
        self.get(
            &format!(
                "feed=dhan&exchange=NSE&segment=FNO&symbol=NIFTY&contract={contract}&timeframe=1min&month=2026-08&t={}{extra}",
                t.join(",")
            ),
            rate,
        )
    }

    fn get(&self, query: &str, rate: &RateTable) -> (StatusCode, Value) {
        let (status, headers, body) = respond(&self.root, query, rate);
        assert_eq!(
            headers[axum::http::header::CONTENT_TYPE],
            "application/json; charset=utf-8"
        );
        (status, serde_json::from_str(&body).expect("greeks JSON"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn close_to(got: &Value, want: f64) {
    let got = got.as_f64().expect("a number");
    #[expect(
        clippy::float_arithmetic,
        reason = "a relative tolerance for a JSON float read back"
    )]
    let off = (got - want).abs() / want.abs().max(1e-300);
    assert!(off < 1e-12, "{got} against {want}");
}

/// GK-04 — a priced cell is exactly `pull::pricing::bar_greeks` on the bar's
/// own close, the underlying's close at the same instant, the tenor to the
/// expiry session's close and the rate of the latest release strictly before
/// the bar's day — checked by building those inputs by hand. Each row now
/// carries the spot's stamp and age, the tenor in years and whether it runs
/// past the rate's 91 days (D-3328, D-3330).
#[test]
fn a_priced_cell_is_bar_greeks_on_the_bars_own_inputs() {
    let fixture = Fixture::new("greeks-priced");
    let rate = RateTable::Loaded(table(&[12, 19]));
    let (status, body) = fixture.ask(ATM, &[t0(), t0() + 60], &rate);
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["contract"], ATM);
    assert_eq!(body["basis"], "calendar365");
    assert_eq!(body["carry"], 0);
    assert_eq!(body["asof"], "exact");
    assert_eq!(body["spot_segment"], "INDEX");
    assert_eq!(body["spot_key"], "NIFTY");
    assert_eq!(body["expiry"], "2026-08-27");
    assert_eq!(body["settled_expiry"], "2026-08-27");
    assert_eq!(body["settlement"], "as_stated");
    assert_eq!(body["rate_tenor_days"], RATE_TENOR_DAYS);
    assert!(
        body["carry_source"]
            .as_str()
            .expect("said")
            .contains("none sourced")
    );
    assert!(
        body["expiry_close"]
            .as_str()
            .expect("said")
            .contains("15:40")
    );
    assert_eq!(body["rate"]["state"], "loaded");
    assert_eq!(body["rate"]["variable"], RATE_ENV);
    assert_eq!(body["rate"]["auctions"], 2);
    assert_eq!(body["rate"]["sha256"].as_str().expect("digest").len(), 64);
    assert_eq!(body["rate_sha256"], body["rate"]["sha256"]);

    let RateTable::Loaded(series) = &rate else {
        unreachable!("loaded above")
    };
    let as_of = series
        .on(TradeDay::new(2026, 8, 20).expect("day"))
        .expect("a fresh rate");
    // THE RELEASE OF THE 19TH, NOT THE 20TH: strictly before the day.
    assert_eq!(as_of.auction.released.to_string(), "2026-08-19");
    let expiry = Expiry::new(2026, 8, 27).expect("expiry");
    for (row, (ts, spot, premium)) in body["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .zip([(t0(), 2_500_000, 25_000), (t0() + 60, 2_501_000, 25_100)])
    {
        let tenor = Tenor::between(ts * 1_000_000, expiry).expect("live");
        // 15:40 on the 27th, the dated close since 2026-08-03.
        assert_eq!(tenor.seconds(), ist(27, 15, 40) - ts);
        let want = pull::pricing::bar_greeks(
            BarQuote {
                spot,
                strike: 2_500_000,
                premium,
                tenor,
                side: OptionSide::Call,
            },
            as_of.rate,
            YearBasis::Calendar365,
        )
        .expect("solvable");
        assert_eq!(row["t"], ts);
        assert_eq!(row["premium"], premium);
        assert_eq!(row["spot"], spot);
        assert_eq!(row["spot_t"], ts);
        assert_eq!(row["spot_age_s"], 0);
        assert_eq!(row["tenor_s"], tenor.seconds());
        close_to(&row["tenor_y"], tenor.years(YearBasis::Calendar365));
        assert_eq!(row["above_rate_tenor"], false);
        assert_eq!(row["auction"], "2026-08-19");
        assert!(row["why"].is_null());
        assert_eq!(row["below_band"], true);
        close_to(&row["rate"], as_of.rate.annual());
        close_to(&row["iv"], want.volatility);
        close_to(&row["delta"], want.delta);
        close_to(&row["gamma"], want.gamma_per_rupee);
        close_to(&row["theta"], want.theta_rupees_per_day);
        close_to(&row["vega"], want.vega_rupees_per_vol_point);
        close_to(&row["rho"], want.rho_rupees_per_rate_point);
        assert_eq!(row["iterations"], want.iterations);
        let iv = row["iv"].as_f64().expect("iv");
        assert!(iv > 0.0 && iv < 1.0, "{iv}");
    }
}

/// GK-05 — with no rate table named, or one that will not read, every cell is
/// a named refusal and no rate is guessed; the bar's own close and spot are
/// still shown, and so is the tenor, measured before the rate.
#[test]
fn no_rate_table_is_a_named_refusal_in_every_cell_and_never_a_guess() {
    let fixture = Fixture::new("greeks-norate");
    assert!(matches!(RateTable::from_var(None), RateTable::Unset));
    assert!(matches!(
        RateTable::from_var(Some(std::ffi::OsString::new())),
        RateTable::Unset
    ));
    let missing = fixture.root.join("no-such-table.tsv");
    let unreadable = RateTable::from_var(Some(missing.clone().into_os_string()));
    let why = match &unreadable {
        RateTable::Unreadable(why) => why.clone(),
        other => format!("not unreadable: {other:?}"),
    };
    assert_eq!(unreadable.state(), "unreadable", "{why}");
    assert!(why.contains("no-such-table.tsv"), "{why}");
    let written = fixture.root.join("table.tsv");
    fs::write(&written, "not the header\n").expect("owned scratch file");
    assert!(matches!(
        RateTable::from_var(Some(written.into_os_string())),
        RateTable::Unreadable(_)
    ));

    for (rate, word) in [
        (RateTable::Unset, "rate_unset"),
        (unreadable, "rate_unreadable"),
    ] {
        let (status, body) = fixture.ask(ATM, &[t0(), t0() + 60], &rate);
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["rate"]["state"], rate.state());
        assert!(body["rate"]["error"].is_string());
        assert!(body["rate_sha256"].is_null(), "no table, no digest");
        for row in body["rows"].as_array().expect("rows") {
            assert_eq!(row["why"], word, "{row}");
            assert!(
                row.get("iv").is_none() && row.get("delta").is_none(),
                "{row}"
            );
            assert!(row["premium"].is_i64() && row["spot"].is_i64(), "{row}");
            assert!(row["tenor_s"].is_i64() && row["tenor_y"].is_f64(), "{row}");
        }
        if word == "rate_unset" {
            assert!(
                body["rows"][0]["detail"]
                    .as_str()
                    .expect("said")
                    .contains(RATE_ENV)
            );
        }
    }
}

/// GK-06 — every other cell that cannot be priced says why, by name: no
/// option bar at `t`, no spot bar at `t`, no file for either, a bar at or
/// past the expiry session's close, a day before the table or past its
/// staleness bound, a premium below the zero-carry floor and a close the
/// model cannot solve (with its sentence).
#[test]
fn every_unpriceable_cell_names_its_reason() {
    let fixture = Fixture::new("greeks-refusals");
    let rate = RateTable::Loaded(table(&[12, 19]));
    let t = t0();
    let (status, body) = fixture.ask(
        ATM,
        &[t + 30, t + 120, ist(27, 15, 45), ist(12, 9, 15)],
        &rate,
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    let rows = body["rows"].as_array().expect("rows");
    let whys: Vec<&str> = rows
        .iter()
        .map(|r| r["why"].as_str().expect("why"))
        .collect();
    assert_eq!(
        whys,
        [
            "no_option_bar",
            "no_spot_bar",
            "tenor_refused",
            "rate_before_series"
        ]
    );
    assert!(
        rows[2]["detail"]
            .as_str()
            .expect("said")
            .contains("past the close")
    );
    assert!(rows[2]["tenor_s"].is_null(), "an expired bar has no tenor");

    // A table whose only release is 19 days before the bar: stale.
    let stale = RateTable::Loaded(table(&[1]));
    let (_, body) = fixture.ask(ATM, &[t], &stale);
    assert_eq!(body["rows"][0]["why"], "rate_stale");

    // ABOVE THE MODEL'S MAXIMUM: still `iv_unsolved`, the model's sentence.
    let (_, body) = fixture.ask(RICH, &[t], &rate);
    assert_eq!(body["rows"][0]["why"], "iv_unsolved", "{body}");
    assert!(
        body["rows"][0]["detail"]
            .as_str()
            .expect("said")
            .contains("supremum"),
        "{body}"
    );

    // NO FILE FOR THE OPTION, OR FOR THE UNDERLYING.
    let (_, body) = fixture.ask("2026-08-27-2600000-PE", &[t], &rate);
    assert_eq!(body["rows"][0]["why"], "no_option_file");
    let (_, body) = fixture.get(
        &format!(
            "feed=dhan&exchange=NSE&segment=FNO&symbol=FINNIFTY&contract={ATM}&timeframe=1min&month=2026-08&t={t}"
        ),
        &rate,
    );
    assert_eq!(body["rows"][0]["why"], "no_option_file");
    assert_eq!(
        body["spot_key"], "NIFTYFINSERVICE",
        "DB-G25: FINNIFTY's spot key"
    );
    write(
        &fixture.root,
        "FNO",
        Some("2026-08-27-2500000-PE"),
        &[(t, 20_000)],
    );
    let (_, body) = fixture.get(
        &format!(
            "feed=dhan&exchange=NSE&segment=FNO&symbol=NIFTY&contract=2026-08-27-2500000-PE&timeframe=1min&month=2026-08&t={t}"
        ),
        &rate,
    );
    assert!(body["rows"][0]["why"].is_null(), "{body}");
    assert!(body["rows"][0]["delta"].as_f64().expect("delta") < 0.0);
    fs::remove_dir_all(fixture.root.join("bars/dhan/NSE/INDEX")).expect("owned scratch");
    let (_, body) = fixture.ask(ATM, &[t], &rate);
    assert_eq!(body["rows"][0]["why"], "no_spot_file");
    assert_eq!(body["rows"][0]["premium"], 25_000);
}

/// **DB-G64, D-3330: A PREMIUM AT OR BELOW THE ZERO-CARRY FLOOR IS NAMED
/// APART FROM AN UNSOLVED VOLATILITY**, with the floor and the reason it is a
/// limit of the model's inputs; before D-3330 it read `iv_unsolved`, which
/// the page took for a bad print.
#[test]
fn a_premium_below_the_zero_carry_floor_is_named_apart() {
    let fixture = Fixture::new("greeks-floor");
    let rate = RateTable::Loaded(table(&[12, 19]));
    let (status, body) = fixture.ask(ITM, &[t0()], &rate);
    assert_eq!(status, StatusCode::OK, "{body}");
    let row = &body["rows"][0];
    assert_eq!(row["why"], "below_zero_carry_bound", "{row}");
    let detail = row["detail"].as_str().expect("said");
    assert!(
        detail.contains("ZERO carry") && detail.contains("not a verdict on the print"),
        "{detail}"
    );
    assert_eq!(row["premium"], 100);
    assert_eq!(row["spot"], 2_500_000);
    assert!(
        row["tenor_s"].is_i64() && row["above_rate_tenor"] == false,
        "{row}"
    );
    assert!(row.get("iv").is_none());
}

/// **DB-G64, D-3330: A TENOR PAST THE RATE'S 91 DAYS IS FLAGGED, IN YEARS.**
/// The 91-day yield is still the dated one; the flag says it is applied
/// beyond its own maturity. The boundary is 91 days exactly.
#[test]
fn a_tenor_past_the_rates_91_days_is_flagged_in_years() {
    let fixture = Fixture::new("greeks-long");
    let rate = RateTable::Loaded(table(&[12, 19]));
    let (status, body) = fixture.ask(LONG, &[t0()], &rate);
    assert_eq!(status, StatusCode::OK, "{body}");
    // THE VENDOR RE-DATED NIFTY `31DEC26` TO THE 29TH (`pull::expiry_alias`,
    // D-3331): the tenor runs to the settled day, two days shorter.
    assert_eq!(body["settled_expiry"], "2026-12-29");
    assert_eq!(body["settlement"], "aliased");
    let row = &body["rows"][0];
    let (tenor, _) = Tenor::to_settled(
        t0() * 1_000_000,
        "NIFTY",
        Expiry::new(2026, 12, 31).expect("expiry"),
    )
    .expect("live");
    assert_eq!(
        tenor.seconds(),
        Tenor::between(t0() * 1_000_000, Expiry::new(2026, 12, 31).expect("expiry"))
            .expect("live")
            .seconds()
            - 2 * 86_400
    );
    assert_eq!(row["tenor_s"], tenor.seconds());
    close_to(&row["tenor_y"], tenor.years(YearBasis::Calendar365));
    assert_eq!(row["above_rate_tenor"], true, "{row}");
    assert!(row["iv"].is_f64(), "{row}");

    let day = 86_400 * 1_000_000;
    let at = |days: i64| {
        Tenor::between(
            ist_on(2026, 12, 31, 15, 40, 0) * 1_000_000 - days * day,
            Expiry::new(2026, 12, 31).expect("expiry"),
        )
        .expect("live")
    };
    assert!(!above_rate_tenor(at(RATE_TENOR_DAYS)));
    assert!(above_rate_tenor(
        Tenor::between(
            ist_on(2026, 12, 31, 15, 40, 0) * 1_000_000 - RATE_TENOR_DAYS * day - 1_000_000,
            Expiry::new(2026, 12, 31).expect("expiry"),
        )
        .expect("live")
    ));
}

/// **DB-039, D-3328: AN INSTANT OUTSIDE THE ASKED MONTH IS `t_outside_month`,
/// NOT `no_option_bar`** -- whether or not the files are held.
#[test]
fn an_instant_outside_the_asked_month_is_a_wrong_question() {
    let fixture = Fixture::new("greeks-outside");
    let rate = RateTable::Loaded(table(&[12, 19]));
    let july = ist_on(2026, 7, 30, 10, 0, 0);
    let september = ist_on(2026, 9, 1, 0, 0, 0);
    let (status, body) = fixture.ask(ATM, &[july, t0(), september], &rate);
    assert_eq!(status, StatusCode::OK, "{body}");
    let whys: Vec<Value> = body["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|r| r["why"].clone())
        .collect();
    assert_eq!(
        whys,
        [
            Value::from("t_outside_month"),
            Value::Null,
            Value::from("t_outside_month")
        ]
    );
    assert!(
        body["rows"][0]["detail"]
            .as_str()
            .expect("said")
            .contains("2026-08")
    );
    let (_, body) = fixture.ask("2026-08-27-2600000-PE", &[july, t0()], &rate);
    assert_eq!(body["rows"][0]["why"], "t_outside_month");
    assert_eq!(body["rows"][1]["why"], "no_option_file");
}

/// **DB-068, D-3328: `asof=prev` PRICES AGAINST THE SESSION'S LAST SPOT AT OR
/// BEFORE `t`, AND THE ROW SAYS WHICH AND HOW OLD.** The premium is still the
/// option's own bar at `t`; without `asof` the same instant is `no_spot_bar`.
#[test]
fn as_of_prev_prices_against_an_earlier_spot_and_shows_its_age() {
    let fixture = Fixture::new("greeks-asof");
    let rate = RateTable::Loaded(table(&[12, 19]));
    let t = t0() + 120;
    let (_, body) = fixture.ask(ATM, &[t], &rate);
    assert_eq!(body["rows"][0]["why"], "no_spot_bar");
    let (status, body) = fixture.ask_with(ATM, &[t], &rate, "&asof=prev");
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["asof"], "prev");
    let row = &body["rows"][0];
    assert!(row["why"].is_null(), "{row}");
    assert_eq!(row["premium"], 26_000);
    assert_eq!(row["spot"], 2_501_000);
    assert_eq!(row["spot_t"], t0() + 60);
    assert_eq!(row["spot_age_s"], 60);
    // AN EXACT HIT UNDER `asof=prev` IS AGE 0, and a refused cell still
    // shows the spot it was read against.
    let (_, body) = fixture.ask_with(ATM, &[ist(12, 9, 15)], &rate, "&asof=prev");
    assert_eq!(body["rows"][0]["why"], "rate_before_series");
    assert_eq!(body["rows"][0]["spot_t"], ist(12, 9, 15));
    assert_eq!(body["rows"][0]["spot_age_s"], 0);
}

/// **DB-G51, D-3331: THE TENOR IS MEASURED TO THE DAY THE CONTRACT SETTLED.**
/// MIDCPNIFTY `22JAN24` settled in the Saturday session of 2024-01-20; its
/// 15:29:59 bar that day has one second left, not 172,801. The body names the
/// settled day and how it was settled; the spot is read from NIFTYMIDSELECT
/// (D-3332).
#[test]
fn a_tenor_is_measured_to_the_settled_expiry() {
    let fixture = Fixture::new("greeks-settled");
    let january = YearMonth::new(2024, 1).expect("month");
    let at = ist_on(2024, 1, 20, 15, 29, 59);
    write_at(
        &fixture.root,
        "FNO",
        "MIDCPNIFTY",
        Some("2024-01-22-1000000-CE"),
        Timeframe::SECOND_1,
        january,
        &[(at, 4_500)],
    );
    write_at(
        &fixture.root,
        "INDEX",
        "NIFTYMIDSELECT",
        None,
        Timeframe::SECOND_1,
        january,
        &[(at, 1_004_000)],
    );
    let rate = RateTable::Loaded(
        Series::parse(&table_text(&[(2024, 1, 10), (2024, 1, 17)])).expect("parses"),
    );
    let (status, body) = fixture.get(
        &format!(
            "feed=dhan&exchange=NSE&segment=FNO&symbol=MIDCPNIFTY&contract=2024-01-22-1000000-CE&timeframe=1s&month=2024-01&t={at}"
        ),
        &rate,
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["expiry"], "2024-01-22");
    assert_eq!(body["settled_expiry"], "2024-01-20");
    assert_eq!(body["settlement"], "aliased");
    assert_eq!(body["spot_key"], "NIFTYMIDSELECT");
    let row = &body["rows"][0];
    assert_eq!(row["spot"], 1_004_000, "{row}");
    assert_eq!(row["tenor_s"], 1, "{row}");
}

/// **DB-067, D-3330: THE RATE TABLE IS READ ONCE AND AGAIN ONLY WHEN ITS FILE
/// STAMP MOVES**, and its digest is the one the body names.
#[test]
fn the_rate_table_is_kept_until_its_file_stamp_moves() {
    let fixture = Fixture::new("greeks-rate-cache");
    let path = fixture.root.join("tbill.tsv");
    fs::write(&path, table_text(&[(2026, 8, 12)])).expect("owned scratch");
    let (first, loads) = rate_loads(Some(path.clone().into_os_string()));
    assert_eq!(first.state(), "loaded");
    let (again, still) = rate_loads(Some(path.clone().into_os_string()));
    assert!(Arc::ptr_eq(&first, &again), "a hit is the kept table");
    assert_eq!(still, loads, "and reads nothing");
    // A NEW RELEASE APPENDED: the stamp moved, the table is read again.
    fs::write(&path, table_text(&[(2026, 8, 12), (2026, 8, 19)])).expect("owned scratch");
    let (moved, reloaded) = rate_loads(Some(path.clone().into_os_string()));
    assert!(!Arc::ptr_eq(&first, &moved));
    assert!(reloaded > loads);
    assert_ne!(first.sha256_hex(), moved.sha256_hex());
    let RateTable::Loaded(series) = &*moved else {
        panic!("loaded: {moved:?}")
    };
    assert_eq!(series.auction_count(), 2);
    // GONE: unreadable by name, never the kept table.
    fs::remove_file(&path).expect("owned scratch");
    let (gone, _) = rate_loads(Some(path.into_os_string()));
    assert_eq!(gone.state(), "unreadable");
    assert!(gone.sha256_hex().is_none());
    let (unset, _) = rate_loads(None);
    assert_eq!(unset.state(), "unset");
}

/// GK-07 — a malformed question is a 400 naming the field, before any file is
/// read; a future's contract is not an option's.
#[test]
fn malformed_questions_are_refused_by_name() {
    let fixture = Fixture::new("greeks-malformed");
    let rate = RateTable::Unset;
    let base = format!(
        "feed=dhan&exchange=NSE&segment=FNO&symbol=NIFTY&contract={ATM}&timeframe=1min&month=2026-08&t=1"
    );
    let too_many = vec!["1"; MAX_GREEK_TIMES + 1].join(",");
    for (query, word) in [
        (base.replace("feed=dhan", "feed=nobody"), "feed"),
        (base.replace("segment=FNO", "segment="), "all required"),
        (base.replace(ATM, "2026-08-27-FUT"), "option contract"),
        (
            base.replace(ATM, "NIFTY-2026-08-27-2500000-CE"),
            "option contract",
        ),
        (base.replace("timeframe=1min", "timeframe="), "timeframe"),
        (base.replace("month=2026-08", "month=Aug"), "month"),
        (base.replace("&t=1", ""), "t is required"),
        (base.replace("t=1", "t=x"), "UTC second"),
        (base.replace("t=1", &format!("t={too_many}")), "chunks"),
        (base.replace("symbol=NIFTY", "symbol=A%2FB"), "symbol"),
        (format!("{base}&asof=nearest"), "asof"),
    ] {
        let (status, body) = fixture.get(&query, &rate);
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {body}");
        assert!(
            body["error"].as_str().expect("named").contains(word),
            "{query}: {body}"
        );
    }
}

/// GK-08 — the contract grammar: `YYYY-MM-DD-<paisa>-CE|PE` and nothing else.
#[test]
fn an_option_contract_segment_reads_its_expiry_strike_and_side() {
    let (expiry, strike, side) = option_parts(ATM).expect("an option");
    assert_eq!(expiry, Expiry::new(2026, 8, 27).expect("expiry"));
    assert_eq!(strike, 2_500_000);
    assert_eq!(side, OptionSide::Call);
    assert_eq!(
        option_parts("2019-01-31-1080000-PE").map(|p| p.2),
        Some(OptionSide::Put)
    );
    for bad in [
        "2026-08-27-FUT",
        "2026-08-27-2500000-XE",
        "2026-08-27--CE",
        "2026-08-27-+25-CE",
        "2026-08-27-0-CE",
        "2026-02-30-2500000-CE",
        "2026-8-27-2500000-CE",
        "2026-08-27-01-2500000-CE",
        "26-08-27-2500000-CE",
        "CE",
        "",
    ] {
        assert_eq!(option_parts(bad), None, "{bad}");
    }
}

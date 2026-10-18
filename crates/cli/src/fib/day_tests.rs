//! Tests of the FIB day input (FIB-8; D-6440, D-6441, D-6445 to D-6451).
//! Every price is synthetic: a five- or seven-digit paisa pattern no listed
//! share trades at, never a copied vendor row.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    reason = "a test that cannot panic cannot fail"
)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Exchange, InstrumentKey};
use pull::calendar::SpecialKind;
use store::file::BarFile;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

use super::super::authorities::{
    AuthorityKind, AuthorityVersion, BasisBreak, CalendarAuthority, ExpiryAuthority,
    FactorAuthority, FactorPair, FactorShape, InstrumentSession, InterimCalendar, LandingAuthority,
    Member, PoolAuthority, TailWitness, version_digest,
};
use super::*;

/// The IST epoch day of a civil date.
fn day_of(year: u16, month: u8, of_month: u8) -> i64 {
    i64::from(
        pull::session::Day::new(year, month, of_month)
            .unwrap()
            .days_from_epoch(),
    )
}

/// The minute-of-day of `hh:mm`.
fn hm(hour: u16, minute: u16) -> u16 {
    hour * 60 + minute
}

/// One synthetic minute: a range around `price`, closing ten paisa above it.
fn minute(day: i64, minute_of_day: u16, price: i64) -> Candle {
    Candle {
        ts_micros: instant(day, u32::from(minute_of_day) * 60),
        open: price,
        high: price + 50,
        low: price - 50,
        close: price + 10,
        volume: 7,
        open_interest: i64::MIN,
    }
}

/// Every minute from `first` to `last` of `day`.
fn minutes(day: i64, first: u16, last: u16, base: i64) -> Vec<Candle> {
    (first..=last)
        .map(|at| minute(day, at, base + i64::from(at)))
        .collect()
}

/// A full 09:15..15:29 day.
fn full(day: i64, base: i64) -> Vec<Candle> {
    minutes(day, hm(9, 15), hm(15, 29), base)
}

/// Dated closes with no master: every pre-2026-08-03 day needs none.
fn no_master() -> CashCloses {
    stored::load_cash_closes(
        Path::new("/nonexistent-fib8-store"),
        "SBIN",
        std::iter::empty(),
    )
    .unwrap()
}

fn interim() -> Authorities {
    Authorities::interim(&|_: &str| true).unwrap()
}

fn spec(year: u16, month: u8, widths: &[u16], reference: PreviousReference) -> MonthSpec<'_> {
    MonthSpec {
        symbol: "SBIN",
        series: SeriesId(3),
        year,
        month,
        widths,
        reference,
        daily: None,
        rename_conflicts: &[],
    }
}

fn build(
    year: u16,
    month: u8,
    widths: &[u16],
    bars: &[Candle],
    authorities: &Authorities,
    state: &mut StockState,
) -> MonthInput {
    build_month(
        &spec(year, month, widths, PreviousReference::TradableDay),
        bars,
        &no_master(),
        authorities,
        state,
    )
    .unwrap()
}

fn input(month: &MonthInput, day: i64) -> &DayInput {
    month
        .days
        .iter()
        .find(|input| input.day == day)
        .unwrap_or_else(|| panic!("no input for {}", date(day)))
}

fn version(kind: AuthorityKind) -> AuthorityVersion {
    AuthorityVersion {
        kind,
        word: "synthetic",
        label: "a synthetic authority of the test",
        digest: version_digest(kind, "synthetic", "test", b""),
        interim: false,
    }
}

/// A tail witness that says one thing.
struct Witness(AuthorityVersion, TailReading);

impl TailWitness for Witness {
    fn version(&self) -> &AuthorityVersion {
        &self.0
    }
    fn after(&self, _: SeriesId, _: i64, _: u32) -> TailReading {
        self.1
    }
}

fn witness(reading: TailReading) -> Box<dyn TailWitness> {
    Box::new(Witness(version(AuthorityKind::TailWitness), reading))
}

/// Landings by day, and one break record.
struct Landings(
    AuthorityVersion,
    HashMap<i64, u64>,
    Option<(i64, BasisBreak)>,
);

impl LandingAuthority for Landings {
    fn version(&self) -> &AuthorityVersion {
        &self.0
    }
    fn landing(&self, _: SeriesId, day: i64) -> Option<LandingId> {
        self.1.get(&day).copied().map(LandingId)
    }
    fn basis_break(&self, _: SeriesId, _: i64, day: i64) -> Option<BasisBreak> {
        self.2
            .filter(|(on, _)| *on == day)
            .map(|(_, record)| record)
    }
}

/// Every day covered and proportional at one ratio, but one day affine.
struct Shapes(AuthorityVersion, i64);

impl FactorAuthority for Shapes {
    fn version(&self) -> &AuthorityVersion {
        &self.0
    }
    fn factor(&self, _: SeriesId, day: i64) -> Result<FactorPair, String> {
        Ok(FactorPair {
            zerodha_close: 2_000,
            gdfl_close: 1_000,
            shape: if day == self.1 {
                FactorShape::Affine
            } else {
                FactorShape::Proportional
            },
            measured: true,
        })
    }
    fn first_minute(&self, series: SeriesId, day: i64) -> Result<FactorPair, String> {
        self.factor(series, day)
    }
    fn covers(&self, _: i64) -> bool {
        true
    }
}

/// The interim calendar, refusing one series on one day by its own record.
struct OneStockRefused(AuthorityVersion, InterimCalendar, SeriesId, i64);

impl CalendarAuthority for OneStockRefused {
    fn version(&self) -> &AuthorityVersion {
        &self.0
    }
    fn tradable(&self, series: SeriesId, day: i64) -> Tradable {
        let mut answer = self.1.tradable(series, day);
        answer.instrument = if series == self.2 && day == self.3 {
            InstrumentSession::Refused("synthetic_special_session")
        } else {
            InstrumentSession::Admitted
        };
        answer
    }
    fn market(&self, day: i64) -> Result<(), CalendarReason> {
        self.1.market(day)
    }
    fn traded(&self, day: i64) -> bool {
        self.1.traded(day)
    }
}

/// One day stamped as the stock's monthly expiry.
struct Expiries(AuthorityVersion, i64);

impl ExpiryAuthority for Expiries {
    fn version(&self) -> &AuthorityVersion {
        &self.0
    }
    fn expiry(&self, _: SeriesId, day: i64) -> ExpiryStamp {
        if day == self.1 {
            ExpiryStamp::StockMonthly
        } else {
            ExpiryStamp::None
        }
    }
}

/// A pool of one member that is not the symbol asked.
struct Lonely(AuthorityVersion, Vec<Member>);

impl PoolAuthority for Lonely {
    fn version(&self) -> &AuthorityVersion {
        &self.0
    }
    fn members(&self, _: i64) -> &[Member] {
        &self.1
    }
    fn slot_of(&self, _: SeriesId, _: i64) -> Option<u16> {
        None
    }
}

// ------------------------------------------------------------------ gate --

/// THE INTERIM GATE'S REASON FOR EACH NAMED DAY, through the day input.
#[test]
fn the_interim_gate_excludes_each_named_day_by_reason() {
    let cases = [
        (
            (2020, 2, 1),
            CalendarCode::SpecialSession,
            Some(SpecialKind::BudgetWeekend),
        ),
        (
            (2024, 1, 20),
            CalendarCode::SpecialSession,
            Some(SpecialKind::WeekendFullSession),
        ),
        (
            (2025, 10, 21),
            CalendarCode::SpecialSession,
            Some(SpecialKind::Muhurat),
        ),
        ((2020, 3, 13), CalendarCode::Halt, None),
        ((2021, 2, 24), CalendarCode::SystemsOutage, None),
    ];
    for ((year, month, named), code, special) in cases {
        let named_day = day_of(year, month, named);
        let mut bars = Vec::new();
        for of_month in 1..=pull::session::Day::new(year, month, 1)
            .unwrap()
            .end_of_month()
            .day()
        {
            let on = day_of(year, month, of_month);
            let weekday = (on + 3).rem_euclid(7) < 5;
            if weekday || on == named_day {
                bars.extend(full(on, 1_234_500));
            }
        }
        let mut state = StockState::first_day();
        let month_input = build(year, month, &[1, 5], &bars, &interim(), &mut state);
        let (_, reason) = month_input
            .excluded
            .iter()
            .find(|(on, _)| *on == named_day)
            .unwrap_or_else(|| panic!("{} not excluded", date(named_day)));
        assert_eq!(reason.code, code, "{}", date(named_day));
        assert_eq!(reason.special, special, "{}", date(named_day));
        let day = input(&month_input, named_day);
        assert!(!day.admitted());
        assert!(
            day.candles.is_empty(),
            "an excluded day folds no decision candle"
        );
        assert!(month_input.counts.calendar.get(year, code.index()) >= 1);
        assert!(
            month_input
                .counts
                .calendar
                .get(year, CalendarCode::Closed.index())
                >= 1
        );
    }
}

#[test]
fn a_2018_day_is_tradable_through_sourced_kind_of() {
    let on = day_of(2018, 10, 1);
    assert_eq!(
        pull::calendar::kind_of(on),
        pull::calendar::DayKind::Unmeasured
    );
    let mut state = StockState::first_day();
    let month = build(
        2018,
        10,
        &[1, 15],
        &full(on, 2_345_600),
        &interim(),
        &mut state,
    );
    let day = input(&month, on);
    assert!(day.admitted(), "{day:?}");
    assert_eq!(day.candles[1].candles.len(), 25);
    assert!(day.candles[1].candles.iter().all(|candle| candle.complete));
}

// ------------------------------------------------------------------ fold --

/// A missing minute withholds every bucket that holds it, at widths 1, 2, 7
/// and 64 (two of which do not divide the 375-minute session); first buckets
/// start at 09:15:00; complete candles equal `fold_from_bars`.
#[test]
fn a_missing_minute_withholds_every_bucket_that_holds_it() {
    let on = day_of(2025, 3, 3);
    let widths = [1, 2, 7, 64];
    let mut bars = full(on, 3_456_700);
    let missing = instant(on, u32::from(hm(10, 0)) * 60);
    bars.retain(|bar| bar.ts_micros != missing);
    let mut state = StockState::first_day();
    let month = build(2025, 3, &widths, &bars, &interim(), &mut state);
    let day = input(&month, on);
    assert!(day.admitted());
    for width in &day.candles {
        let w = i64::from(width.width) * 60_000_000;
        assert_eq!(
            width.candles[0].candle.ts_micros,
            instant(on, u32::from(hm(9, 15)) * 60),
            "width {}: the first bucket starts at 09:15:00",
            width.width
        );
        assert_eq!(width.capacity, 375_u16.div_ceil(width.width));
        let reference = pull::fold::fold_from_bars(
            &bars.iter().map(|bar| bar_of(*bar)).collect::<Vec<_>>(),
            Bucket::of_secs(u32::from(width.width) * 60).unwrap(),
            Bucket::MINUTE,
        )
        .unwrap();
        for candle in &width.candles {
            let start = candle.candle.ts_micros;
            let holds_missing = start <= missing && missing < start + w;
            let short = candle.close_ts_micros - start < w;
            assert_eq!(
                candle.complete,
                !holds_missing && !short,
                "{} {}",
                width.width,
                stamp(start)
            );
            assert_eq!(candle.missing_minutes, u16::from(holds_missing));
            assert_eq!(candle.tf_minutes, width.width);
            let folded = reference.iter().find(|bar| bar.ts_micros == start).unwrap();
            assert_eq!(
                candle.candle,
                candle_of(*folded),
                "the one fold authority's bar"
            );
            if !candle.complete {
                assert!(
                    width
                        .withheld
                        .iter()
                        .any(|note| note.contains(&stamp(start))),
                    "{:?}",
                    width.withheld
                );
            }
        }
        if width.width == 1 {
            assert!(
                width
                    .candles
                    .iter()
                    .all(|candle| candle.candle.ts_micros != missing)
            );
            assert_eq!(width.candles.len(), 374);
        }
    }
    let seven = &day.candles[2];
    assert!(
        !seven.candles.last().unwrap().complete,
        "375 = 53 x 7 + 4: the last bucket is short"
    );
    assert!(
        seven
            .withheld
            .iter()
            .any(|note| note.contains("partial last bucket"))
    );
}

#[test]
fn an_early_last_minute_is_unwitnessed_cut_or_kept_by_its_witness() {
    let on = day_of(2025, 3, 4);
    let bars = minutes(on, hm(9, 15), hm(14, 56), 4_567_800);
    let mut state = StockState::first_day();
    let month = build(2025, 3, &[5], &bars, &interim(), &mut state);
    assert_eq!(input(&month, on).refusal, Some(DayRefusal::TailUnwitnessed));
    assert!(input(&month, on).candles.is_empty());
    let mut kept = interim();
    kept.tail = witness(TailReading::NoTradesAfter);
    let mut state = StockState::first_day();
    let month = build(2025, 3, &[5], &bars, &kept, &mut state);
    let day = input(&month, on);
    assert_eq!(day.refusal, None);
    assert!(day.no_trades_after);
    assert_eq!(month.counts.no_trades_after, 1);
    let last = day.candles[0].candles.last().unwrap();
    assert!(
        !last.complete,
        "the 14:55 bucket holds two of its five minutes"
    );
    assert_eq!(last.missing_minutes, 3);
    let mut cut = interim();
    cut.tail = witness(TailReading::TradesAfter);
    let mut state = StockState::first_day();
    let month = build(2025, 3, &[5], &bars, &cut, &mut state);
    assert_eq!(input(&month, on).refusal, Some(DayRefusal::TailCut));
    assert_eq!(
        month.counts.refusals.get(2025, DayRefusal::TailCut.index()),
        1
    );
}

/// Install a receipted NSE session master naming `share` with `flag` for
/// `day` under `store/session-masters`, exactly as the pull does.
fn install_master(store: &Path, share: &str, day: i64, flag: u8) {
    use std::io::Write as _;
    let isin = brutex_core::universe::nse_isin(share).expect("an ISIN");
    let csv = format!(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n7777,{share},EQ,{},{flag}\n",
        isin.as_str()
    );
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gz.write_all(csv.as_bytes()).unwrap();
    let civil = pull::session::Day::from_days(u32::try_from(day).unwrap()).unwrap();
    pull::cash_session_cache::install(&store.join("session-masters"), civil, &gz.finish().unwrap())
        .unwrap();
}

/// A fresh scratch directory, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "brutex-fib8-day-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A synthetic 360-minute day (the closing-auction era, an eligible share),
/// and the regime tag reaching the next days (FM-F101).
#[test]
fn a_360_minute_day_and_the_regime_tag_reach_the_day_input() {
    let store = Scratch::new("cas");
    let (friday, monday, tuesday) = (day_of(2026, 7, 31), day_of(2026, 8, 3), day_of(2026, 8, 4));
    install_master(&store.0, "TECHM", monday, 1);
    install_master(&store.0, "TECHM", tuesday, 1);
    let cash = stored::load_cash_closes(&store.0, "TECHM", [friday, monday, tuesday]).unwrap();
    assert_eq!(
        window_of(monday, &cash),
        Ok(Window {
            first: hm(9, 15),
            last: hm(15, 14)
        })
    );
    assert_eq!(
        window_of(friday, &cash),
        Ok(Window {
            first: hm(9, 15),
            last: hm(15, 29)
        })
    );
    assert_eq!(cas_of(monday, &cash), CasEligibility::Eligible);
    assert_eq!(cas_of(friday, &cash), CasEligibility::Unknown);
    let authorities = interim();
    let mut state = StockState::first_day();
    let widths = [1, 15];
    let july = MonthSpec {
        symbol: "TECHM",
        ..spec(2026, 7, &widths, PreviousReference::TradableDay)
    };
    build_month(
        &july,
        &full(friday, 5_678_900),
        &cash,
        &authorities,
        &mut state,
    )
    .unwrap();
    let august = MonthSpec {
        symbol: "TECHM",
        ..spec(2026, 8, &widths, PreviousReference::TradableDay)
    };
    let mut bars = full(monday, 5_679_000);
    bars.extend(full(tuesday, 5_679_100));
    let month = build_month(&august, &bars, &cash, &authorities, &mut state).unwrap();
    let day = input(&month, monday);
    assert!(day.admitted(), "{day:?}");
    assert_eq!(day.window.unwrap().minutes(), 360);
    assert_eq!(day.counts.real, 360);
    assert_eq!(
        day.counts.outside_continuous, 15,
        "15:15..15:29 are not continuous minutes"
    );
    assert_eq!(day.candles[0].capacity, 360);
    assert_eq!(day.candles[0].candles.len(), 360);
    assert!(
        day.candles[1].candles.iter().all(|candle| candle.complete),
        "360 = 24 x 15"
    );
    assert_eq!(
        day.previous().carried().unwrap().regime_last_minute,
        hm(15, 29)
    );
    let next = input(&month, tuesday);
    assert_eq!(
        next.previous().carried().unwrap().regime_last_minute,
        hm(15, 14)
    );
    assert_eq!(next.previous().carried().unwrap().day, monday);
    // Without a master, the day's window is unverified and the day refused.
    let masterless = MonthSpec {
        symbol: "TECHM",
        ..spec(2026, 8, &widths, PreviousReference::TradableDay)
    };
    let mut fresh = StockState::first_day();
    let month = build_month(
        &masterless,
        &full(monday, 5_679_000),
        &no_master(),
        &authorities,
        &mut fresh,
    )
    .unwrap();
    assert_eq!(
        input(&month, monday).refusal,
        Some(DayRefusal::SessionUnverified)
    );
    assert!(
        window_of(monday, &no_master())
            .unwrap_err()
            .starts_with("session_unverified: 2026-08-03")
    );
    assert_eq!(cas_of(monday, &no_master()), CasEligibility::Unknown);
    let flag_zero = Scratch::new("cas0");
    install_master(&flag_zero.0, "TECHM", monday, 0);
    let ineligible = stored::load_cash_closes(&flag_zero.0, "TECHM", [monday]).unwrap();
    assert_eq!(cas_of(monday, &ineligible), CasEligibility::NotEligible);
    assert_eq!(window_of(monday, &ineligible).unwrap().last, hm(15, 29));
}

// ------------------------------------------------------- previous close --

#[test]
fn a_first_day_has_no_previous_close_and_no_move() {
    let on = day_of(2025, 3, 3);
    let mut state = StockState::first_day();
    let month = build(2025, 3, &[1], &full(on, 6_789_000), &interim(), &mut state);
    let day = input(&month, on);
    assert_eq!(
        day.previous_tradable,
        PreviousClose::Refused(CloseRefusal::NoPrevious, None)
    );
    assert_eq!(
        day.previous_session,
        PreviousClose::Refused(CloseRefusal::NoPrevious, None)
    );
    assert_eq!(
        day.candidate_refusal().as_deref(),
        Some("no_previous_close:first_day")
    );
    assert!(day.sides.iter().all(Option::is_none));
    assert_eq!(
        month
            .counts
            .previous
            .get(2025, CloseRefusal::NoPrevious.index()),
        1
    );
    assert_eq!(state.tradable.unwrap().day, on);
    assert_eq!(
        state.tradable.unwrap().close,
        6_789_000 + i64::from(hm(15, 29)) + 10
    );
}

#[test]
fn a_duplicate_unsorted_or_corrupt_minute_refuses_only_its_day() {
    let days: Vec<i64> = (3..=7).map(|d| day_of(2025, 3, d)).collect();
    let next = day_of(2025, 3, 10);
    for (fault, refusal) in [
        ("duplicate", DayRefusal::DuplicateMinute),
        ("unsorted", DayRefusal::Unsorted),
        ("corrupt", DayRefusal::CorruptMinute),
        ("off-grid", DayRefusal::CorruptMinute),
    ] {
        let mut bars = Vec::new();
        for (n, on) in days.iter().chain([&next]).enumerate() {
            let mut day = full(*on, 7_890_000 + i64::try_from(n).unwrap() * 100);
            if *on == days[4] {
                let at = day
                    .iter()
                    .position(|bar| minute_of(bar) == hm(11, 0))
                    .unwrap();
                match fault {
                    "duplicate" => day.insert(at, day[at]),
                    "unsorted" => day.swap(at, at + 1),
                    "corrupt" => day[at].high = day[at].low - 1,
                    _ => day[at].ts_micros += 1,
                }
            }
            bars.extend(day);
        }
        let mut state = StockState::first_day();
        let month = build(2025, 3, &[1], &bars, &interim(), &mut state);
        for on in &days[..4] {
            assert!(input(&month, *on).admitted(), "{fault} {}", date(*on));
        }
        let refused = input(&month, days[4]);
        assert_eq!(refused.refusal, Some(refusal), "{fault}");
        assert!(refused.candles.is_empty(), "a refused day emits no candle");
        assert_eq!(
            input(&month, next).previous().word(),
            "previous_day_refused",
            "{fault}: the next day's close is refused"
        );
    }
    assert_eq!(DayRefusal::DuplicateMinute.name(), "duplicate_minute");
}

#[test]
fn a_missing_previous_day_is_not_adjacent() {
    let (monday, wednesday) = (day_of(2025, 3, 3), day_of(2025, 3, 5));
    let mut bars = full(monday, 8_901_000);
    bars.extend(full(wednesday, 8_902_000));
    let mut state = StockState::first_day();
    let month = build(2025, 3, &[1], &bars, &interim(), &mut state);
    let day = input(&month, wednesday);
    for verdict in [day.previous_tradable, day.previous_session] {
        let PreviousClose::Refused(why, carried) = verdict else {
            panic!("{verdict:?}");
        };
        assert_eq!(why, CloseRefusal::NotAdjacent);
        assert_eq!(
            carried.unwrap().day,
            monday,
            "today D-2 is what was carried"
        );
    }
    assert!(
        month
            .days
            .iter()
            .all(|input| input.day != day_of(2025, 3, 4)),
        "Tuesday holds no minute"
    );
    assert!(month.counts.tradable_without_minutes >= 1);
    // A carried close dated on or after the day is never adjacent.
    let ahead = verdict(
        Some(CarriedClose {
            close: 5,
            day: wednesday,
            landing: None,
            regime_last_minute: 0,
            refused: false,
        }),
        wednesday,
        |_| false,
        None,
        |_| unreachable!(),
    );
    assert_eq!(ahead.word(), "not_adjacent");
}

#[test]
fn minutes_after_0916_never_change_the_days_verdict() {
    let (monday, tuesday) = (day_of(2025, 3, 3), day_of(2025, 3, 4));
    let verdicts = |late: i64| {
        let mut bars = full(monday, 1_111_000);
        let mut day = full(tuesday, 1_112_000);
        for bar in day.iter_mut().filter(|bar| minute_of(bar) > hm(9, 16)) {
            bar.open += late;
            bar.high += late;
            bar.low += late;
            bar.close += late;
        }
        bars.extend(day);
        let mut state = StockState::first_day();
        let month = build(2025, 3, &[1], &bars, &interim(), &mut state);
        let day = input(&month, tuesday).clone();
        (day.previous_session, day.previous_tradable)
    };
    assert_eq!(verdicts(0), verdicts(333_000));
    assert!(verdicts(0).1.accepted().is_some());
}

fn landed(by_day: &[(i64, u64)], split: Option<(i64, BasisBreak)>) -> Authorities {
    let mut authorities = interim();
    authorities.landing = Box::new(Landings(
        version(AuthorityKind::Landing),
        by_day.iter().copied().collect(),
        split,
    ));
    authorities
}

/// FM-F092: an ex-date inside one landing is accepted; a 1:2 split across two
/// landings with no record is `basis_unproven`; a PREC-10 record carries it.
#[test]
fn the_basis_of_a_carried_close_decides_whether_it_is_accepted() {
    let (monday, tuesday) = (day_of(2025, 3, 3), day_of(2025, 3, 4));
    let mut bars = full(monday, 2_000_000);
    bars.extend(full(tuesday, 1_000_000));
    // One landing, an adjusted series: the move is the series' own.
    let mut state = StockState::first_day();
    let month = build(
        2025,
        3,
        &[1],
        &bars,
        &landed(&[(monday, 4), (tuesday, 4)], None),
        &mut state,
    );
    let day = input(&month, tuesday);
    let accepted = day.previous().accepted().unwrap();
    assert_eq!(accepted.proof, BasisProof::SameLanding);
    assert!(accepted.landing_only, "the identity factor covers nothing");
    assert_eq!(month.counts.landing_only, 1);
    assert_eq!(day.sides[0], Some(MoveSide::Loser));
    // Two landings, no record: unproven, no move, no side.
    let mut state = StockState::first_day();
    let month = build(
        2025,
        3,
        &[1],
        &bars,
        &landed(&[(monday, 4), (tuesday, 5)], None),
        &mut state,
    );
    let day = input(&month, tuesday);
    assert_eq!(day.previous().word(), "basis_unproven");
    assert_eq!(
        day.candidate_refusal().as_deref(),
        Some("no_previous_close:basis_unproven")
    );
    assert!(day.sides.iter().all(Option::is_none));
    // A sourced record is no proof while the factor authority covers nothing:
    // before its first covered day only the landing test applies.
    let record = BasisBreak {
        numerator: 1,
        denominator: 2,
        source: "synthetic PREC-10 row",
    };
    let mut covered = landed(&[(monday, 4), (tuesday, 5)], Some((tuesday, record)));
    covered.factor = Box::new(Shapes(version(AuthorityKind::Factor), i64::MIN));
    let mut state = StockState::first_day();
    let month = build(2025, 3, &[1], &bars, &covered, &mut state);
    let accepted = input(&month, tuesday)
        .previous()
        .accepted()
        .copied()
        .unwrap();
    assert_eq!(accepted.proof, BasisProof::BreakRecord(record));
    assert_eq!(accepted.scaled().1, 2);
    let open = 1_000_000 + i64::from(hm(9, 15)) + 10;
    let previous = 2_000_000 + i64::from(hm(15, 29)) + 10;
    assert_eq!(accepted.scaled(), (i128::from(previous), 2));
    assert_eq!(
        accepted.side(open),
        MoveSide::Gainer,
        "half of the old close is below the open"
    );
    assert_eq!(
        accepted.move_bp(open),
        ((i128::from(open) * 2 - i128::from(previous)) * 10_000).div_euclid(i128::from(previous))
    );
}

#[test]
fn an_affine_day_is_flagged_unreliable() {
    let (monday, tuesday) = (day_of(2025, 3, 3), day_of(2025, 3, 4));
    let mut bars = full(monday, 2_100_000);
    bars.extend(full(tuesday, 2_100_100));
    let mut authorities = interim();
    authorities.factor = Box::new(Shapes(version(AuthorityKind::Factor), tuesday));
    let mut state = StockState::first_day();
    let month = build(2025, 3, &[1], &bars, &authorities, &mut state);
    assert_eq!(input(&month, tuesday).previous().word(), "basis_unreliable");
    assert_eq!(
        month
            .counts
            .previous
            .get(2025, CloseRefusal::BasisUnreliable.index()),
        1
    );
}

/// ZD-41: 2021-02-25 names both previous closes — the traded session
/// 2021-02-24 (the outage day) and the tradable day 2021-02-23.
#[test]
fn the_day_after_the_outage_names_both_previous_closes() {
    let (tue, wed, thu) = (
        day_of(2021, 2, 23),
        day_of(2021, 2, 24),
        day_of(2021, 2, 25),
    );
    let mut bars = full(tue, 3_100_000);
    bars.extend(minutes(wed, hm(9, 15), hm(11, 39), 3_200_000));
    bars.extend(minutes(wed, hm(15, 45), hm(16, 59), 3_300_000));
    bars.extend(full(thu, 3_400_000));
    for reference in [
        PreviousReference::TradedSession,
        PreviousReference::TradableDay,
    ] {
        let mut state = StockState::first_day();
        let month = build_month(
            &spec(2021, 2, &[1], reference),
            &bars,
            &no_master(),
            &interim(),
            &mut state,
        )
        .unwrap();
        let outage = input(&month, wed);
        assert_eq!(
            outage.refusal, None,
            "a halt day's early end is its halt, not a cut tail"
        );
        assert_eq!(outage.counts.outside_continuous, 75);
        let day = input(&month, thu);
        let session = day.previous_session.accepted().unwrap();
        let tradable = day.previous_tradable.accepted().unwrap();
        assert_eq!(session.carried.day, wed);
        assert_eq!(
            session.carried.close,
            3_200_000 + i64::from(hm(11, 39)) + 10
        );
        assert_eq!(tradable.carried.day, tue);
        let deciding = day.previous().accepted().unwrap().carried.day;
        assert_eq!(
            deciding,
            if reference == PreviousReference::TradedSession {
                wed
            } else {
                tue
            }
        );
    }
}

// ---------------------------------------------------------- minute flags --

#[test]
fn padded_no_trade_minutes_are_holes_and_stale() {
    let (monday, tuesday) = (day_of(2025, 3, 3), day_of(2025, 3, 4));
    let mut bars = full(monday, 4_100_000);
    let mut day = full(tuesday, 4_200_000);
    let before = day
        .iter()
        .find(|bar| minute_of(bar) == hm(10, 59))
        .unwrap()
        .close;
    for bar in &mut day {
        let at = minute_of(bar);
        if (hm(11, 0)..=hm(11, 20)).contains(&at) {
            *bar = Candle {
                open: before,
                high: before,
                low: before,
                close: before,
                volume: 0,
                ..*bar
            };
        }
        if at == hm(12, 0) {
            bar.volume = 0;
        }
    }
    bars.extend(day);
    let mut state = StockState::first_day();
    let month = build(2025, 3, &[5], &bars, &interim(), &mut state);
    let input_day = input(&month, tuesday);
    assert_eq!(input_day.counts.no_trade, 21);
    assert_eq!(input_day.counts.zero_volume_with_range, 1);
    assert_eq!(input_day.counts.real, 375 - 21);
    assert_eq!(month.counts.no_trade_minutes, 21);
    let slot = |hour, minute| usize::from(hm(hour, minute) - hm(9, 15));
    for at in slot(11, 0)..=slot(11, 20) {
        assert_eq!(
            input_day.closes[at],
            MinuteClose::Stale(before),
            "the rank shows STALE"
        );
    }
    assert!(matches!(
        input_day.closes[slot(12, 0)],
        MinuteClose::Real(_)
    ));
    for candle in &input_day.candles[0].candles {
        let start = minute_of(&candle.candle);
        let padded = start >= hm(11, 0) && start <= hm(11, 20);
        assert_eq!(
            candle.complete,
            !padded,
            "{}",
            stamp(candle.candle.ts_micros)
        );
    }
    // A flat zero-volume minute at 09:15 is never padding.
    let opening = minute(tuesday, hm(9, 15), 4_300_000);
    let flat = Candle {
        high: opening.open,
        low: opening.open,
        close: opening.open,
        volume: 0,
        ..opening
    };
    let mut state = StockState::first_day();
    let month = build(2025, 3, &[1], &[flat], &interim(), &mut state);
    assert_eq!(input(&month, tuesday).counts.real, 1);
}

#[test]
fn an_extreme_wick_is_marked_against_committed_days_only() {
    let (d1, d2, d3) = (day_of(2025, 3, 3), day_of(2025, 3, 4), day_of(2025, 3, 5));
    let mut bars = full(d1, 5_100_000);
    let mut wide = full(d2, 5_100_000);
    let at = wide
        .iter()
        .position(|bar| minute_of(bar) == hm(10, 30))
        .unwrap();
    wide[at].high += 90_000;
    let low_at = wide
        .iter()
        .position(|bar| minute_of(bar) == hm(13, 0))
        .unwrap();
    wide[low_at].low -= 90_000;
    bars.extend(wide);
    bars.extend(full(d3, 5_100_000));
    let mut state = StockState::first_day();
    let month = build(2025, 3, &[1], &bars, &interim(), &mut state);
    assert!(
        input(&month, d1).suspect.is_empty(),
        "no committed past to judge against"
    );
    assert_eq!(
        input(&month, d2).suspect,
        [
            SuspectWick {
                slot: hm(10, 30) - hm(9, 15),
                upper: true
            },
            SuspectWick {
                slot: hm(13, 0) - hm(9, 15),
                upper: false
            },
        ]
    );
    assert!(
        input(&month, d3).suspect.is_empty(),
        "the wide wick is committed past now"
    );
    assert_eq!(month.counts.suspect_wicks, 2);
    let up = state.wick.up.unwrap();
    assert_eq!(up.0, 90_000 + 40);
}

/// FIB-7's touch-point: a halt day's input admits only the decisions before
/// its first halted second, and is never committed to own past.
#[test]
fn halt_day_input_admits_only_pre_halt_decisions() {
    let (thursday, halt) = (day_of(2020, 3, 12), day_of(2020, 3, 13));
    let mut bars = full(thursday, 6_100_000);
    let mut wild = full(halt, 6_200_000);
    for bar in &mut wild {
        bar.high += 500_000;
    }
    bars.extend(wild);
    let mut state = StockState::first_day();
    let before_halt = {
        let mut probe = StockState::first_day();
        build(
            2020,
            3,
            &[1, 5],
            &full(thursday, 6_100_000),
            &interim(),
            &mut probe,
        );
        probe
    };
    let month = build(2020, 3, &[1, 5], &bars, &interim(), &mut state);
    let day = input(&month, halt);
    assert!(!day.admitted());
    assert!(day.candles.is_empty());
    let exposure = day.exposure.as_ref().expect("the exposure input");
    let first = 9 * 3_600 + 21 * 60 + 9;
    assert_eq!(exposure.first_halted_second, first);
    let until = instant(halt, first);
    for width in &exposure.candles {
        assert!(
            width
                .candles
                .iter()
                .all(|candle| candle.close_ts_micros <= until)
        );
    }
    assert_eq!(
        exposure.candles[0].candles.len(),
        6,
        "09:15..09:20 close by 09:21:00"
    );
    assert_eq!(
        exposure.candles[1].candles.len(),
        1,
        "09:15..09:19 closes by 09:20:00"
    );
    assert_eq!(state.wick, before_halt.wick, "never committed to own past");
    assert_eq!(state.tradable, before_halt.tradable);
    assert_eq!(
        state.session.unwrap().day,
        halt,
        "the traded session is the halt day"
    );
}

#[test]
fn a_tail_cut_is_refused_and_leaves_the_committed_past_as_it_was() {
    let days: Vec<i64> = (3..=7).map(|d| day_of(2025, 3, d)).collect();
    let mut first_four = Vec::new();
    for (n, on) in days[..4].iter().enumerate() {
        first_four.extend(full(*on, 7_100_000 + i64::try_from(n).unwrap() * 1_000));
    }
    let mut snapshot = StockState::first_day();
    build(2025, 3, &[1], &first_four, &interim(), &mut snapshot);
    let mut cut_bars = first_four.clone();
    cut_bars.extend(minutes(days[4], hm(9, 15), hm(12, 6), 7_200_000));
    let mut cut = interim();
    cut.tail = witness(TailReading::TradesAfter);
    let mut state = StockState::first_day();
    let month = build(2025, 3, &[1], &cut_bars, &cut, &mut state);
    let day = input(&month, days[4]);
    assert_eq!(day.refusal, Some(DayRefusal::TailCut));
    assert_eq!(
        state.wick, snapshot.wick,
        "the own past equals the day-4 snapshot"
    );
    let mut whole = first_four;
    whole.extend(full(days[4], 7_200_000));
    let mut other = StockState::first_day();
    let uncut = build(2025, 3, &[1], &whole, &cut, &mut other);
    let ten = usize::from(hm(10, 0) - hm(9, 15));
    assert_eq!(
        day.closes[ten],
        input(&uncut, days[4]).closes[ten],
        "the 10:00 rank is unchanged"
    );
    assert!(matches!(day.closes[ten], MinuteClose::Real(_)));
    let after = usize::from(hm(12, 30) - hm(9, 15));
    assert!(
        matches!(day.closes[after], MinuteClose::Stale(_)),
        "STALE after its last minute"
    );
}

#[test]
fn the_expiry_stamps_ride_the_day_and_change_no_candle() {
    let on = day_of(2025, 3, 27);
    let mut stamped = interim();
    stamped.expiry = Box::new(Expiries(version(AuthorityKind::Expiry), on));
    let mut state = StockState::first_day();
    let with = build(2025, 3, &[5], &full(on, 8_100_000), &stamped, &mut state);
    let mut state = StockState::first_day();
    let without = build(2025, 3, &[5], &full(on, 8_100_000), &interim(), &mut state);
    assert_eq!(input(&with, on).stamps.expiry, ExpiryStamp::StockMonthly);
    assert_eq!(input(&with, on).stamps.fno_ban, "ban list UNVERIFIED");
    assert_eq!(
        input(&without, on).stamps.expiry,
        ExpiryStamp::Absent("expiry_source_not_loaded")
    );
    assert_eq!(
        input(&with, on).candles,
        input(&without, on).candles,
        "never a coordinate"
    );
}

#[test]
fn a_per_instrument_refusal_refuses_that_stock_only() {
    let (monday, tuesday) = (day_of(2025, 3, 3), day_of(2025, 3, 4));
    let mut bars = full(monday, 9_100_000);
    bars.extend(full(tuesday, 9_100_100));
    let mut authorities = interim();
    authorities.calendar = Box::new(OneStockRefused(
        version(AuthorityKind::Calendar),
        InterimCalendar::interim(),
        SeriesId(3),
        tuesday,
    ));
    let mut state = StockState::first_day();
    let month = build(2025, 3, &[1], &bars, &authorities, &mut state);
    let day = input(&month, tuesday);
    assert!(!day.admitted());
    assert_eq!(
        day.tradable.instrument,
        InstrumentSession::Refused("synthetic_special_session")
    );
    assert!(day.candles.is_empty());
    assert_eq!(state.tradable.unwrap().day, monday, "own past unchanged");
    assert_eq!(month.counts.instrument_session_unchecked, 0);
    let other = MonthSpec {
        series: SeriesId(4),
        ..spec(2025, 3, &[1], PreviousReference::TradableDay)
    };
    let mut state = StockState::first_day();
    let month = build_month(&other, &bars, &no_master(), &authorities, &mut state).unwrap();
    assert!(
        input(&month, tuesday).admitted(),
        "another stock trades the day"
    );
    let mut state = StockState::first_day();
    let month = build(2025, 3, &[1], &bars, &interim(), &mut state);
    assert_eq!(
        month.counts.instrument_session_unchecked, 2,
        "labelled and counted"
    );
}

// ---------------------------------------------------------------- store --

/// Writes a synthetic month into a scratch store.
fn write_month(
    root: &Path,
    vendor: Vendor,
    key: &InstrumentKey,
    rung: Timeframe,
    when: (u16, u8),
    bars: &[Candle],
) {
    let path = StorePath::for_key(
        vendor,
        key,
        rung,
        YearMonth::new(when.0, when.1).unwrap(),
        FileKind::Bars,
    )
    .unwrap();
    let id = brutex_core::universe::fnv1a(key.underlying.as_str()) as u32;
    let rows: Vec<Bar> = bars.iter().map(|bar| bar_of(*bar)).collect();
    BarFile::open_or_create(root, path, id)
        .unwrap()
        .append(&rows)
        .unwrap();
}

fn cash_key(symbol: &str) -> InstrumentKey {
    InstrumentKey::cash(Exchange::Nse, symbol).unwrap()
}

/// FM-F099: every refusal is classified, and only absence counts as absence.
#[test]
fn load_month_classifies_every_refusal() {
    let store = Scratch::new("classes");
    let when = (2025, 3);
    let gone = store.0.join("not-there");
    let halted = load_month(&gone, Vendor::Zerodha, "SBIN", "1min", when).unwrap_err();
    assert_eq!(halted.word, "store_root_vanished");
    assert_eq!(
        load_month(&store.0, Vendor::Zerodha, "SBIN", "1min", when),
        Ok(MonthMinutes::KeyAbsent)
    );
    let on = day_of(2025, 3, 3);
    write_month(
        &store.0,
        Vendor::Zerodha,
        &cash_key("SBIN"),
        Timeframe::MINUTE_1,
        (2025, 2),
        &full(day_of(2025, 2, 3), 1_212_000),
    );
    assert_eq!(
        load_month(&store.0, Vendor::Zerodha, "SBIN", "1min", when),
        Ok(MonthMinutes::Absent)
    );
    write_month(
        &store.0,
        Vendor::Zerodha,
        &cash_key("SBIN"),
        Timeframe::MINUTE_1,
        when,
        &full(on, 1_213_000),
    );
    let MonthMinutes::Minutes(bars) =
        load_month(&store.0, Vendor::Zerodha, "SBIN", "1min", when).unwrap()
    else {
        panic!("minutes");
    };
    assert_eq!(bars.len(), 375);
    let refused = load_month(&store.0, Vendor::Zerodha, "NOTASHARE", "1min", when).unwrap_err();
    assert_eq!(refused.word, "not_sweepable");
    // A directory where the month file should be: the host refuses.
    let path = StorePath::for_key(
        Vendor::Zerodha,
        &cash_key("INFY"),
        Timeframe::MINUTE_1,
        YearMonth::new(2025, 3).unwrap(),
        FileKind::Bars,
    )
    .unwrap()
    .to_path_buf(&store.0);
    std::fs::create_dir_all(&path).unwrap();
    let io = load_month(&store.0, Vendor::Zerodha, "INFY", "1min", when).unwrap_err();
    assert_eq!(io.word, "io", "{io}");
    // Garbage where a month should be.
    let path = StorePath::for_key(
        Vendor::Zerodha,
        &cash_key("TCS"),
        Timeframe::MINUTE_1,
        YearMonth::new(2025, 3).unwrap(),
        FileKind::Bars,
    )
    .unwrap()
    .to_path_buf(&store.0);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        b"not a month file at all, just synthetic garbage bytes",
    )
    .unwrap();
    let corrupt = load_month(&store.0, Vendor::Zerodha, "TCS", "1min", when).unwrap_err();
    assert_eq!(corrupt.word, "corrupt", "{corrupt}");
    assert!(corrupt.to_string().starts_with("corrupt: "));
}

#[test]
fn a_month_held_by_a_writer_halts_by_name() {
    let store = Scratch::new("locked");
    let when = (2025, 3);
    let key = cash_key("SBIN");
    let path = StorePath::for_key(
        Vendor::Zerodha,
        &key,
        Timeframe::MINUTE_1,
        YearMonth::new(2025, 3).unwrap(),
        FileKind::Bars,
    )
    .unwrap();
    let id = brutex_core::universe::fnv1a("SBIN") as u32;
    let mut writer = BarFile::open_or_create(&store.0, path, id).unwrap();
    writer
        .append(
            &full(day_of(2025, 3, 3), 1_313_000)
                .iter()
                .map(|bar| bar_of(*bar))
                .collect::<Vec<_>>(),
        )
        .unwrap();
    let held = load_month(&store.0, Vendor::Zerodha, "SBIN", "1min", when);
    drop(writer);
    match held {
        Err(halt) => assert_eq!(halt.word, "locked", "{halt}"),
        Ok(found) => panic!("a held month loaded: {found:?}"),
    }
}

/// An `&` symbol loads through the store, and `day-check` reads it end to end.
#[test]
fn an_ampersand_symbol_loads_and_day_check_reads_it() {
    let store = Scratch::new("mm");
    let key = cash_key("M&M");
    let (monday, tuesday) = (day_of(2025, 3, 3), day_of(2025, 3, 4));
    let mut bars = full(monday, 1_414_000);
    bars.extend(full(tuesday, 1_414_100));
    write_month(
        &store.0,
        Vendor::Zerodha,
        &key,
        Timeframe::MINUTE_1,
        (2025, 3),
        &bars,
    );
    let MonthMinutes::Minutes(loaded) =
        load_month(&store.0, Vendor::Zerodha, "M&M", "1min", (2025, 3)).unwrap()
    else {
        panic!("M&M minutes");
    };
    assert_eq!(loaded, bars);
    let request = parse(&[
        "M&M",
        "2025-03",
        "--previous",
        "tradable-day",
        "--authorities",
        "interim",
    ])
    .unwrap();
    assert_eq!(request.symbol, "M&M");
    let authorities = interim();
    let mut out = String::new();
    assert_eq!(
        run(&store.0, &request, &authorities, &mut out),
        crate::OK,
        "{out}"
    );
    assert!(
        out.starts_with("fib day-check M&M 2025-03: Zerodha 1min"),
        "{out}"
    );
    assert!(out.contains("interim: refused for --out-store"));
    assert!(out.contains("previous month absent: the month's first day has no previous close"));
    assert!(
        out.contains("2025-03-03 instrument_session_unchecked; minutes 375"),
        "{out}"
    );
    assert!(out.contains("previous first_day / first_day"));
    assert!(out.contains("2025-03-04 instrument_session_unchecked; minutes 375"));
    assert!(out.contains("previous accepted / accepted"));
    assert!(out.contains("excluded days: 2025-03-01 closed"), "{out}");
    assert!(out.contains("count 2025 calendar closed"));
    assert!(out.contains("count 2025 previous_close first_day 1"));
    assert!(out.contains("widths: 1,2,4,8,16,32,64"));
    let json_request = Request {
        json: true,
        ..request.clone()
    };
    let mut json = String::new();
    assert_eq!(
        run(&store.0, &json_request, &authorities, &mut json),
        crate::OK
    );
    assert!(
        json.starts_with("{\"symbol\":\"M&M\",\"month\":\"2025-03\",\"loaded\":\"minutes\""),
        "{json}"
    );
    assert!(json.contains("\"research\":true"));
    assert!(json.contains("\"candidate_refusal\":\"no_previous_close:first_day\""));
    assert!(json.contains("\"reason\":\"closed\",\"known_in_advance\":true"));
    assert_eq!(json.matches('{').count(), json.matches('}').count());
    // A pool that does not hold the symbol refuses by name.
    let mut lonely = interim();
    lonely.pool = Box::new(Lonely(
        version(AuthorityKind::Pool),
        vec![Member {
            series_id: SeriesId(0),
            symbol_on_day: "SBIN".to_owned(),
            zerodha_series_present: true,
            in_current_list: true,
        }],
    ));
    let mut out = String::new();
    assert_eq!(run(&store.0, &request, &lonely, &mut out), crate::FAILED);
    assert!(out.contains("not_in_pool: M&M"), "{out}");
    // Renaming the root mid-run exits FAILED naming the root.
    let moved = store.0.with_extension("moved");
    let mut out = String::new();
    assert_eq!(run(&moved, &request, &authorities, &mut out), crate::FAILED);
    assert!(out.contains("store_root_vanished"), "{out}");
}

#[test]
fn day_check_misuses_are_refused_by_name() {
    for (words, expected) in [
        (&[][..], "takes SYM YYYY-MM"),
        (&["SBIN"][..], "takes SYM YYYY-MM"),
        (
            &["SBIN", "2025-13", "--authorities", "interim"][..],
            "`2025-13` is not a YYYY-MM month",
        ),
        (
            &["SBIN", "March", "--authorities", "interim"][..],
            "`March` is not a YYYY-MM month",
        ),
        (
            &["SBIN", "2025-03", "--authorities", "interim"][..],
            "previous_reference_required",
        ),
        (
            &["SBIN", "2025-03", "--previous", "tradable-day"][..],
            "authorities_required",
        ),
        (
            &[
                "NIFTY",
                "2025-03",
                "--previous",
                "tradable-day",
                "--authorities",
                "interim",
            ][..],
            "is an index",
        ),
        (
            &[
                "NOTASHARE",
                "2025-03",
                "--previous",
                "tradable-day",
                "--authorities",
                "interim",
            ][..],
            "not an instrument this engine sweeps",
        ),
    ] {
        let refused = parse(words).unwrap_err();
        assert!(refused.contains(expected), "{words:?}: {refused}");
        let mut out = String::new();
        assert_eq!(command(words, &mut out), crate::MISUSED, "{words:?}");
    }
    let request = parse(&[
        "sbin",
        "2025-03",
        "--json",
        "--previous",
        "traded-session",
        "--authorities",
        "interim",
    ])
    .unwrap();
    assert_eq!(request.symbol, "SBIN");
    assert!(request.json);
    assert_eq!(request.reference, PreviousReference::TradedSession);
    assert_eq!(PreviousReference::TradableDay.word(), "tradable-day");
    assert_eq!(month_before((2025, 1)), Some((2024, 12)));
    assert_eq!(month_before((1970, 1)), None);
}

// -------------------------------------------------------------- seconds --

fn second(day: i64, of_day: u32, price: i64) -> Candle {
    Candle {
        ts_micros: instant(day, of_day),
        open: price,
        high: price,
        low: price,
        close: price,
        volume: 3,
        open_interest: i64::MIN,
    }
}

#[test]
fn the_seconds_loader_keeps_continuous_seconds_and_names_every_refusal() {
    let (mon, tue, wed, thu, fri) = (
        day_of(2025, 3, 3),
        day_of(2025, 3, 4),
        day_of(2025, 3, 5),
        day_of(2025, 3, 6),
        day_of(2025, 3, 7),
    );
    let (negative_day, negative_volume_day) = (day_of(2025, 3, 10), day_of(2025, 3, 11));
    let open = 9 * 3_600 + 15 * 60;
    let mut wide = second(mon, 10 * 3_600, 1_515_003);
    wide.high = 1_515_033;
    wide.low = 1_514_973;
    let mut bars = vec![
        second(mon, open - 1, 1_515_000),
        second(mon, open, 1_515_001),
        second(mon, open + 1, 1_515_002),
        wide,
        second(tue, open, 1_515_004),
        second(tue, open, 1_515_005),
        second(wed, open + 5, 1_515_006),
        second(wed, open + 4, 1_515_007),
    ];
    let mut off = second(thu, open, 1_515_008);
    off.ts_micros += 1;
    bars.push(off);
    // The store's sanity rules refuse a negative price and a negative volume.
    let mut negative = second(negative_day, open, 1_515_009);
    negative.low = -1;
    bars.push(negative);
    let mut negative_volume = second(negative_volume_day, open, 1_515_010);
    negative_volume.volume = -1;
    bars.push(negative_volume);
    let month = seconds_of(
        &bars,
        &[mon, tue, wed, thu, fri, negative_day, negative_volume_day],
        &no_master(),
    );
    let kept = month.day(mon).unwrap();
    assert_eq!(kept.len(), 3);
    assert!(!kept.is_empty());
    assert_eq!(kept.at(open).unwrap().close, 1_515_001);
    assert_eq!(kept.at(10 * 3_600).unwrap().close, 1_515_003);
    assert_eq!(
        kept.at(open + 2),
        None,
        "a missing second is no value, never zero"
    );
    assert_eq!(kept.at(open - 1), None);
    assert_eq!(kept.at(u32::MAX), None);
    // The range column answers what the bar answers, and nothing where it
    // has no bar.
    assert_eq!(
        kept.range(10 * 3_600),
        Some(SecondRange {
            high: 1_515_033,
            low: 1_514_973
        })
    );
    assert_eq!(
        kept.range(open),
        Some(SecondRange {
            high: 1_515_001,
            low: 1_515_001
        })
    );
    assert_eq!(kept.range(open + 2), None, "no value, never zero");
    assert_eq!(kept.range(open - 1), None);
    assert_eq!(kept.range(u32::MAX), None);
    assert_eq!(
        month.outside_continuous, 1,
        "09:14:59 is the opening auction"
    );
    assert_eq!(
        month.refused,
        [
            (tue, "seconds_duplicate"),
            (wed, "seconds_unsorted"),
            (thu, "seconds_corrupt"),
            (negative_day, "seconds_corrupt"),
            (negative_volume_day, "seconds_corrupt")
        ]
    );
    assert_eq!(month.no_execution_seconds, [fri]);
    assert!(month.day(fri).is_none());
    // A halt's seconds are not continuous.
    let halt = day_of(2020, 3, 13);
    let month = seconds_of(
        &[
            second(halt, 9 * 3_600 + 20 * 60, 1_616_000),
            second(halt, 9 * 3_600 + 30 * 60, 1_616_001),
        ],
        &[halt],
        &no_master(),
    );
    assert_eq!(month.day(halt).unwrap().len(), 1);
    assert_eq!(month.outside_continuous, 1);
    // A day with no dated close in the auction era is refused by name.
    let cas = day_of(2026, 8, 3);
    let month = seconds_of(&[second(cas, open, 1_717_000)], &[cas], &no_master());
    assert!(month.days.is_empty());
    assert!(
        month.no_execution_seconds.is_empty() || month.refused.iter().any(|(on, _)| *on == cas)
    );
}

#[test]
fn an_absent_gdfl_month_leaves_every_minute_day_without_execution_seconds() {
    let store = Scratch::new("gdfl");
    let days = [day_of(2025, 3, 3), day_of(2025, 3, 4)];
    let month = load_seconds(&store.0, "SBIN", (2025, 3), &days, &no_master()).unwrap();
    assert_eq!(month.no_execution_seconds, days);
    let open = 9 * 3_600 + 15 * 60;
    write_month(
        &store.0,
        Vendor::Gdfl,
        &cash_key("SBIN"),
        Timeframe::SECOND_1,
        (2025, 3),
        &[
            second(days[0], open, 1_818_000),
            second(days[0], open + 1, 1_818_001),
        ],
    );
    let month = load_seconds(&store.0, "SBIN", (2025, 3), &days, &no_master()).unwrap();
    assert_eq!(month.day(days[0]).unwrap().len(), 2);
    assert_eq!(month.no_execution_seconds, [days[1]]);
    let gone =
        load_seconds(&store.0.join("x"), "SBIN", (2025, 3), &days, &no_master()).unwrap_err();
    assert_eq!(gone.word, "store_root_vanished");
}

// ----------------------------------------------------------------- misc --

#[test]
fn widths_and_the_ladder_follow_r7() {
    assert_eq!(max_width_minutes(), 118);
    assert_eq!(doubling_ladder(), [1, 2, 4, 8, 16, 32, 64]);
    assert_eq!(check_widths(&[1, 2, 7, 64]), Ok(()));
    for refused in [&[][..], &[0][..], &[119][..], &[2, 2][..], &[3, 2][..]] {
        assert!(
            check_widths(refused)
                .unwrap_err()
                .starts_with("widths_refused"),
            "{refused:?}"
        );
    }
    let mut state = StockState::first_day();
    let refused = build_month(
        &spec(2025, 3, &[], PreviousReference::TradableDay),
        &[],
        &no_master(),
        &interim(),
        &mut state,
    );
    assert!(refused.unwrap_err().starts_with("widths_refused"));
    let refused = build_month(
        &spec(2025, 13, &[1], PreviousReference::TradableDay),
        &[],
        &no_master(),
        &interim(),
        &mut state,
    );
    assert!(refused.unwrap_err().starts_with("month_refused"));
}

#[test]
fn the_time_helpers_round_trip() {
    let on = day_of(2025, 3, 3);
    for of_day in [0, 1, 33_300, 54_540, 86_399] {
        assert_eq!(ist_day_second(instant(on, of_day)), (on, of_day));
    }
    assert_eq!(rank_slots(), 354);
    assert_eq!(date(on), "2025-03-03");
    assert_eq!(date(-1), "day -1");
    assert_eq!(stamp(instant(on, 9 * 3_600 + 15 * 60)), "09:15");
    assert_eq!(
        Window {
            first: 555,
            last: 929
        }
        .minutes(),
        375
    );
    assert!(
        Window {
            first: 555,
            last: 929
        }
        .holds(929)
    );
    assert!(
        !Window {
            first: 555,
            last: 929
        }
        .holds(930)
    );
    assert_eq!(json_string("a\"b\\c\n"), "\"a\\\"b\\\\c\\u000a\"");
    assert_eq!(short_hex(&[0xAB; 32]), "abababababababab");
}

#[test]
fn the_variant_1day_close_is_read_and_never_decides() {
    let (monday, tuesday) = (day_of(2025, 3, 3), day_of(2025, 3, 4));
    let mut bars = full(monday, 1_919_000);
    bars.extend(full(tuesday, 1_919_100));
    let daily = [Candle {
        ts_micros: instant(monday, 0),
        open: 1_919_500,
        high: 1_919_600,
        low: 1_919_400,
        close: 1_919_555,
        volume: 1,
        open_interest: i64::MIN,
    }];
    let with = MonthSpec {
        daily: Some(&daily),
        ..spec(2025, 3, &[1], PreviousReference::TradableDay)
    };
    let mut state = StockState::first_day();
    let month = build_month(&with, &bars, &no_master(), &interim(), &mut state).unwrap();
    let day = input(&month, tuesday);
    assert_eq!(day.variant_1day_close, Some(1_919_555));
    assert_eq!(
        day.previous().accepted().unwrap().carried.close,
        1_919_000 + i64::from(hm(15, 29)) + 10,
        "the folded minutes' close decides"
    );
    assert_eq!(input(&month, monday).variant_1day_close, None);
}

/// NO TYPED NUMBERS: the shipping part of this file holds no numeric
/// literal but 0 and 1.
#[test]
fn day_rs_types_no_number() {
    let source = include_str!("day.rs");
    let typed = vocab::fib_cube::untyped_numbers(source);
    assert!(typed.is_empty(), "{typed:?}");
}

/// A pool whose series trades as `old` before `switch` and as `new` from it.
struct Renamed(AuthorityVersion, i64, Vec<Member>, Vec<Member>);

impl PoolAuthority for Renamed {
    fn version(&self) -> &AuthorityVersion {
        &self.0
    }
    fn members(&self, day: i64) -> &[Member] {
        if day < self.1 { &self.2 } else { &self.3 }
    }
    fn slot_of(&self, _: SeriesId, _: i64) -> Option<u16> {
        Some(0)
    }
}

fn renamed(switch: i64, old: &str, new: &str) -> Renamed {
    let member = |symbol: &str| Member {
        series_id: SeriesId(7),
        symbol_on_day: symbol.to_owned(),
        zerodha_series_present: true,
        in_current_list: true,
    };
    Renamed(
        version(AuthorityKind::Pool),
        switch,
        vec![member(old)],
        vec![member(new)],
    )
}

/// FM-F093: OLDSYM's days and NEWSYM's days load as one series month with
/// continuous state; a day held under both keys with differing bars is
/// refused by name; equal bars under both keys are one day.
#[test]
fn a_rename_month_loads_as_one_series_with_continuous_state() {
    let store = Scratch::new("rename");
    let old_days: Vec<i64> = (3..=7).map(|d| day_of(2025, 3, d)).collect();
    // 2025-03-14 is a trading holiday: the new key trades four days.
    let new_days: Vec<i64> = (10..=13).map(|d| day_of(2025, 3, d)).collect();
    let mut old = Vec::new();
    for (n, on) in old_days.iter().enumerate() {
        old.extend(full(*on, 2_020_000 + i64::try_from(n).unwrap() * 100));
    }
    let mut new = Vec::new();
    for (n, on) in new_days.iter().enumerate() {
        new.extend(full(*on, 2_030_000 + i64::try_from(n).unwrap() * 100));
    }
    write_month(
        &store.0,
        Vendor::Zerodha,
        &cash_key("SBIN"),
        Timeframe::MINUTE_1,
        (2025, 3),
        &old,
    );
    write_month(
        &store.0,
        Vendor::Zerodha,
        &cash_key("INFY"),
        Timeframe::MINUTE_1,
        (2025, 3),
        &new,
    );
    let pool = renamed(new_days[0], "SBIN", "INFY");
    let month = load_series_month(&store.0, &pool, SeriesId(7), (2025, 3)).unwrap();
    assert_eq!(month.symbols, ["SBIN", "INFY"]);
    assert!(month.conflicts.is_empty());
    assert_eq!(month.minutes.len(), 375 * 9);
    let mut state = StockState::first_day();
    let series_spec = MonthSpec {
        series: SeriesId(7),
        rename_conflicts: &month.conflicts,
        ..spec(2025, 3, &[1], PreviousReference::TradableDay)
    };
    let built = build_month(
        &series_spec,
        &month.minutes,
        &no_master(),
        &interim(),
        &mut state,
    )
    .unwrap();
    let first_new = input(&built, new_days[0]);
    let carried = first_new.previous().accepted().unwrap().carried;
    assert_eq!(
        carried.day, old_days[4],
        "the previous close carries across the rename"
    );
    assert_eq!(state.tradable.unwrap().day, new_days[3]);
    // The old key also holds the first new day, identically: one day.
    let mut both = old.clone();
    both.extend(full(new_days[0], 2_030_000));
    let same = Scratch::new("rename-same");
    write_month(
        &same.0,
        Vendor::Zerodha,
        &cash_key("SBIN"),
        Timeframe::MINUTE_1,
        (2025, 3),
        &both,
    );
    write_month(
        &same.0,
        Vendor::Zerodha,
        &cash_key("INFY"),
        Timeframe::MINUTE_1,
        (2025, 3),
        &new,
    );
    let month = load_series_month(&same.0, &pool, SeriesId(7), (2025, 3)).unwrap();
    assert!(month.conflicts.is_empty());
    assert_eq!(month.minutes.len(), 375 * 9);
    // The old key holds it with other bars: refused by name.
    let mut differing = old.clone();
    differing.extend(full(new_days[0], 2_040_000));
    let conflict = Scratch::new("rename-conflict");
    write_month(
        &conflict.0,
        Vendor::Zerodha,
        &cash_key("SBIN"),
        Timeframe::MINUTE_1,
        (2025, 3),
        &differing,
    );
    write_month(
        &conflict.0,
        Vendor::Zerodha,
        &cash_key("INFY"),
        Timeframe::MINUTE_1,
        (2025, 3),
        &new,
    );
    let month = load_series_month(&conflict.0, &pool, SeriesId(7), (2025, 3)).unwrap();
    assert_eq!(month.conflicts, [new_days[0]]);
    let conflict_spec = MonthSpec {
        series: SeriesId(7),
        rename_conflicts: &month.conflicts,
        ..spec(2025, 3, &[1], PreviousReference::TradableDay)
    };
    let mut state = StockState::first_day();
    let built = build_month(
        &conflict_spec,
        &month.minutes,
        &no_master(),
        &interim(),
        &mut state,
    )
    .unwrap();
    let refused = input(&built, new_days[0]);
    assert_eq!(refused.refusal, Some(DayRefusal::RenameOverlapConflict));
    assert_eq!(
        DayRefusal::RenameOverlapConflict.name(),
        "rename_overlap_conflict"
    );
    assert_eq!(
        input(&built, new_days[1]).previous().word(),
        "previous_day_refused"
    );
    // A key the store does not hold is counted, never read as a month.
    let lost = renamed(new_days[0], "SBIN", "TCS");
    let month = load_series_month(&store.0, &lost, SeriesId(7), (2025, 3)).unwrap();
    assert_eq!(month.key_absent, ["TCS"]);
    assert_eq!(month.minutes.len(), 375 * 5);
    assert!(stitch((2025, 13), &|_| None, &[]).minutes.is_empty());
}

/// `day-check --reference` adds the month's reference table and its holes.
#[test]
fn day_check_reports_the_reference_table_with_holes_counted() {
    let store = Scratch::new("refcheck");
    let monday = day_of(2025, 3, 3);
    write_month(
        &store.0,
        Vendor::Zerodha,
        &cash_key("SBIN"),
        Timeframe::MINUTE_1,
        (2025, 3),
        &full(monday, 1_515_000),
    );
    let nifty = InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap();
    let session: Vec<Candle> = (0..375_u16)
        .map(|n| minute(monday, hm(9, 15) + n, 2_200_000 + i64::from(n)))
        .collect();
    write_month(
        &store.0,
        Vendor::Zerodha,
        &nifty,
        Timeframe::MINUTE_1,
        (2025, 3),
        &session,
    );
    let vix = InstrumentKey::index(Exchange::Nse, "INDIAVIX").unwrap();
    write_month(
        &store.0,
        Vendor::Zerodha,
        &vix,
        Timeframe::MINUTE_1,
        (2025, 3),
        &session[..10],
    );
    let friday = day_of(2025, 2, 28);
    let february: Vec<Candle> = (0..375_u16)
        .map(|n| minute(friday, hm(9, 15) + n, 2_100_000 + i64::from(n)))
        .collect();
    write_month(
        &store.0,
        Vendor::Zerodha,
        &nifty,
        Timeframe::MINUTE_1,
        (2025, 2),
        &february,
    );
    let request = parse(&[
        "SBIN",
        "2025-03",
        "--reference",
        "--previous",
        "tradable-day",
        "--authorities",
        "interim",
    ])
    .unwrap();
    assert!(request.reference_table);
    let mut out = String::new();
    assert_eq!(
        run(&store.0, &request, &interim(), &mut out),
        crate::OK,
        "{out}"
    );
    assert!(
        out.contains("reference 2025-03: 4 indices over 31 days of the index session; index day unscreened (PREC-9 not merged); absent stamps per index (holes): NIFTY 11250, BANKNIFTY 11625, INDIAVIX 11615, GIFTNIFTY 11625"),
        "{out}"
    );
    let table = reference_check(&store.0, (2025, 3), &interim()).unwrap();
    let slot = table.index_slot("NIFTY").unwrap();
    assert_eq!(
        table.stamp(instant(monday, u32::from(hm(9, 15)) * 60), slot),
        Some(super::super::files::ReferenceCell::Exact {
            close: 2_200_000 + 10,
            previous_close: 2_100_000 + 374 + 10
        }),
        "March measures against February's last tradable close"
    );
    let json_request = Request {
        json: true,
        ..request
    };
    let mut json = String::new();
    assert_eq!(
        run(&store.0, &json_request, &interim(), &mut json),
        crate::OK
    );
    assert!(json.contains("\"reference_table\":{\"days\":31,\"screen\":\"index day unscreened (PREC-9 not merged)\",\"indices\":[{\"index\":\"NIFTY\",\"holes\":11250}"), "{json}");
    // A VIX month that is there and unreadable halts by name.
    let broken = Scratch::new("refcheck-broken");
    let path = StorePath::for_key(
        Vendor::Zerodha,
        &vix,
        Timeframe::MINUTE_1,
        YearMonth::new(2025, 3).unwrap(),
        FileKind::Bars,
    )
    .unwrap()
    .to_path_buf(&broken.0);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"synthetic garbage").unwrap();
    let refused = reference_check(&broken.0, (2025, 3), &interim()).unwrap_err();
    assert_eq!(refused.word, "reference_index", "{refused}");
    let mut out = String::new();
    write_month(
        &broken.0,
        Vendor::Zerodha,
        &cash_key("SBIN"),
        Timeframe::MINUTE_1,
        (2025, 3),
        &full(monday, 1_515_000),
    );
    assert_eq!(
        run(&broken.0, &request_with_reference(), &interim(), &mut out),
        crate::FAILED
    );
    assert!(out.contains("reference_index"), "{out}");
}

fn request_with_reference() -> Request {
    parse(&[
        "SBIN",
        "2025-03",
        "--reference",
        "--previous",
        "tradable-day",
        "--authorities",
        "interim",
    ])
    .unwrap()
}

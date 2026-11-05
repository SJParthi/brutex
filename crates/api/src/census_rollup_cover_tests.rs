//! `census_rollup.rs`'s tests of an exchange named or not, the cell bound
//! and a span the calendar cannot count (BASE-COV-API-AS3).
//!
//! Every census row below is invented: its contracts, its row counts and its
//! stamps name no real print.

use super::*;
use crate::server::{Loaded, Site};
use brutex_core::instrument::Contract;
use pull::manifest::{Closes, Entry, EntryKey, Held, Manifest, epoch_path, segment_dir};
use std::path::PathBuf;

/// 2019-03-01 09:15:00 IST, in microseconds: inside the month the FNO keys
/// below are filed under.
const MARCH: i64 = 1_551_411_900_000_000;

/// 2019-05-02 09:15:00 IST, in microseconds: sixty-two days after [`MARCH`],
/// so after the whole of the month a March key is filed under.
const MAY: i64 = MARCH + 62 * 86_400_000_000;

/// 09:15 IST, in microseconds after an IST midnight.
const OPEN_MICROS: i64 = 33_300_000_000;

/// Every rung a census key can name, finest first.
const RUNGS: [Timeframe; 10] = [
    Timeframe::SECOND_1,
    Timeframe::MINUTE_1,
    Timeframe::MINUTE_2,
    Timeframe::MINUTE_3,
    Timeframe::MINUTE_5,
    Timeframe::MINUTE_10,
    Timeframe::MINUTE_15,
    Timeframe::MINUTE_30,
    Timeframe::MINUTE_60,
    Timeframe::DAY_1,
];

/// One census key.
fn key(
    exchange: Exchange,
    segment: Segment,
    symbol: &str,
    contract: Option<&str>,
    timeframe: Timeframe,
    month: (u16, u8),
) -> EntryKey {
    EntryKey {
        contract: contract.map(|text| Contract::parse(text).expect("an invented contract")),
        exchange,
        segment,
        symbol: Symbol::new(symbol).expect("a symbol"),
        timeframe,
        month: YearMonth::new(month.0, month.1).expect("a month"),
    }
}

/// `key` holding ten bars, one a second from `first`, with invented closes.
fn ten(key: EntryKey, first: i64) -> Held {
    Held::new(
        Entry {
            key,
            rows: 10,
            first_ts_micros: first,
            last_ts_micros: first + 9_000_000,
        },
        Closes::known(5_001, 5_010).expect("a close"),
    )
}

/// A store whose GDFL census holds `rows`, all of `segment`, in that
/// segment's partition, and a server over it with no warm thread: a request
/// reads the census and builds the facet index itself.
fn served(name: &str, segment: Segment, rows: &[Held]) -> (PathBuf, Loaded) {
    let root = crate::scratch::path(&format!("census-rollup-cover-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("masters")).expect("mkdir");
    std::fs::create_dir_all(root.join("manifest")).expect("mkdir");
    let mut part =
        Manifest::partition_genesis(Vendor::Gdfl, segment, 1_048_576).expect("a partition");
    for one in rows {
        let _append = part.record_held(*one).expect("records");
    }
    std::fs::create_dir_all(segment_dir(&root, Vendor::Gdfl, segment)).expect("mkdir");
    std::fs::write(epoch_path(&root, Vendor::Gdfl, segment, 1), part.image()).expect("written");
    let site = Loaded::new(Site::load(&root.join("masters"), &root));
    (root, site)
}

/// `GET /census/rollup.json?<query>` through the route's own function.
fn rollup(site: &Site, query: &str) -> (u16, serde_json::Value) {
    let (code, _, body) = answer(site, query);
    let value = serde_json::from_str(&body).unwrap_or_else(|why| panic!("{why}: {body}"));
    (code.as_u16(), value)
}

/// The `why` of a refusal, or a failure naming the body.
fn why_of(body: &serde_json::Value) -> &str {
    body["why"]
        .as_str()
        .unwrap_or_else(|| panic!("no why: {body}"))
}

/// **EVERY EXCHANGE WORD IS READ, RESOLVED OR REFUSED BY NAME.** An FNO
/// census holds NIFTY on NSE (two options at one second, and a future at one
/// minute whose bars the census records two months after its own month), NIFTY
/// on BSE, and BANKNIFTY on NSE alone. An exchange that is not one is refused
/// by name and echoed capped; one named that holds the underlying is answered
/// with that exchange's rows alone; none named and two holders is `409
/// exchange_ambiguous` naming both, never one picked; one holder is found by
/// probe. A month whose recorded bars lie outside it holds `null` days, never
/// a count.
///
/// Extremes: singles — an exchange that is not one (`XYZ`), one in lower
/// case (`nse`: never case-folded), an empty one (the same as none), an
/// underlying one exchange holds (found by probe), the exchange named that
/// holds it; combinations — an underlying held on both exchanges with none
/// named (409 naming both, empty or absent alike) and the same with each one
/// named (that one's rows alone), a garbage exchange with no underlying at
/// once (the exchange is refused first); out of the box — an exchange of 300
/// bytes (echoed capped at 160 characters), and a census key whose bars are
/// recorded two months after the month it is filed under (`days_held` null
/// in its row and in its rung's total, `days_owed` still counted).
#[test]
fn every_exchange_word_is_read_resolved_or_refused_by_name() {
    let march = (2019, 3);
    let rows = [
        ten(
            key(
                Exchange::Nse,
                Segment::Fno,
                "NIFTY",
                Some("2019-03-28-1000000-CE"),
                Timeframe::SECOND_1,
                march,
            ),
            MARCH,
        ),
        ten(
            key(
                Exchange::Nse,
                Segment::Fno,
                "NIFTY",
                Some("2019-03-28-1000000-PE"),
                Timeframe::SECOND_1,
                march,
            ),
            MARCH,
        ),
        // OUT OF ITS MONTH: a March key whose bars the census records in May.
        ten(
            key(
                Exchange::Nse,
                Segment::Fno,
                "NIFTY",
                Some("2019-03-28-FUT"),
                Timeframe::MINUTE_1,
                march,
            ),
            MAY,
        ),
        ten(
            key(
                Exchange::Bse,
                Segment::Fno,
                "NIFTY",
                Some("2019-03-28-1000000-CE"),
                Timeframe::SECOND_1,
                march,
            ),
            MARCH,
        ),
        ten(
            key(
                Exchange::Nse,
                Segment::Fno,
                "BANKNIFTY",
                Some("2019-03-28-2000000-CE"),
                Timeframe::SECOND_1,
                march,
            ),
            MARCH,
        ),
    ];
    let (_root, site) = served("exchanges", Segment::Fno, &rows);
    let fno = "feed=gdfl&segment=FNO";

    // REFUSED BY NAME: not an exchange, lower case, and garbage with no
    // underlying (the exchange's own refusal comes first).
    for (asked, says) in [
        ("&underlying=NIFTY&exchange=XYZ", "exchange \"XYZ\""),
        ("&underlying=NIFTY&exchange=nse", "exchange \"nse\""),
        ("&exchange=XYZ", "exchange \"XYZ\""),
    ] {
        let (code, body) = rollup(&site, &format!("{fno}{asked}"));
        assert_eq!(code, 400, "{asked}: {body}");
        assert_eq!(body["error"], "bad_request", "{asked}: {body}");
        assert!(why_of(&body).contains(says), "{asked}: {body}");
    }
    // PAST EVERY BOUND: echoed capped, never whole.
    let long = "A".repeat(300);
    let (code, body) = rollup(&site, &format!("{fno}&underlying=NIFTY&exchange={long}"));
    assert_eq!(code, 400, "{body}");
    let why = why_of(&body);
    assert!(why.contains("...(+140 more)"), "{why}");
    assert!(!why.contains(&long), "{why}");

    // NAMED AND HELD: that exchange's rows, and no other's.
    let (code, body) = rollup(&site, &format!("{fno}&exchange=NSE&underlying=NIFTY"));
    assert_eq!(code, 200, "{body}");
    assert_eq!(body["exchange"], "NSE", "{body}");
    assert_eq!(body["underlying"], "NIFTY", "{body}");
    let months = body["months"].as_array().expect("months");
    assert_eq!(months.len(), 2, "{body}");
    assert_eq!(months[0]["timeframe"], Timeframe::SECOND_1.as_str());
    assert_eq!(months[0]["instrument_months"], 2, "{body}");
    assert_eq!(months[0]["days_held"], 1, "{body}");
    // THE KEY OUT OF ITS MONTH: no session of March is held, and `null`
    // says so; the month still owes its sessions.
    assert_eq!(months[1]["timeframe"], Timeframe::MINUTE_1.as_str());
    assert_eq!(months[1]["instrument_months"], 1, "{body}");
    assert!(months[1]["days_held"].is_null(), "{body}");
    assert!(months[1]["days_owed"].is_u64(), "{body}");
    let rungs = body["rungs"].as_array().expect("rungs");
    assert_eq!(rungs.len(), 2, "{body}");
    assert_eq!(rungs[1]["timeframe"], Timeframe::MINUTE_1.as_str());
    assert!(rungs[1]["days_held"].is_null(), "{body}");
    let (code, body) = rollup(&site, &format!("{fno}&exchange=BSE&underlying=NIFTY"));
    assert_eq!(code, 200, "{body}");
    assert_eq!(body["exchange"], "BSE", "{body}");
    assert_eq!(body["months"].as_array().map(Vec::len), Some(1), "{body}");
    assert_eq!(body["months"][0]["instrument_months"], 1, "{body}");

    // TWO HOLDERS, NONE NAMED: refused, naming both, absent or empty alike.
    for asked in ["&underlying=NIFTY", "&underlying=NIFTY&exchange="] {
        let (code, body) = rollup(&site, &format!("{fno}{asked}"));
        assert_eq!(code, 409, "{asked}: {body}");
        assert_eq!(body["error"], "exchange_ambiguous", "{asked}: {body}");
        assert_eq!(
            why_of(&body),
            "NIFTY is an underlying of gdfl FNO on NSE and BSE; name one with exchange=",
            "{asked}"
        );
    }
    // ONE HOLDER, FOUND BY PROBE.
    let (code, body) = rollup(&site, &format!("{fno}&underlying=BANKNIFTY"));
    assert_eq!(code, 200, "{body}");
    assert_eq!(body["exchange"], "NSE", "{body}");
    assert_eq!(body["months"].as_array().map(Vec::len), Some(1), "{body}");
}

/// The IST opening of `month`'s first day, in microseconds.
fn opening(month: (u16, u8)) -> i64 {
    YearMonth::new(month.0, month.1)
        .expect("a month")
        .ist_bounds_micros()
        .0
        + OPEN_MICROS
}

/// The `n`th month counted from January 1990.
fn nth_month(n: usize) -> (u16, u8) {
    (
        1_990 + u16::try_from(n / 12).expect("a small count"),
        1 + u8::try_from(n % 12).expect("under twelve"),
    )
}

/// One spot key of `symbol` in INDEX.
fn spot(symbol: &str, timeframe: Timeframe, month: (u16, u8)) -> Held {
    ten(
        key(
            Exchange::Nse,
            Segment::Index,
            symbol,
            None,
            timeframe,
            month,
        ),
        opening(month),
    )
}

/// **AN ANSWER CARRIES THE CELL BOUND EXACTLY AND IS REFUSED ONE PAST IT.**
/// NIFTY spot holds exactly [`CELLS_MOST`] (month, rung) cells -- 409 months
/// at all ten rungs from January 1990, then six rungs of February 2024 -- and
/// BANKNIFTY one more, in March 2024. NIFTY's roll-up is answered whole,
/// newest month first; the segment's, one cell past the bound, is `422
/// rollup_too_many` with its count, never served slowly or cut.
///
/// Extremes: singles — exactly the bound (4,096 cells answered), one past it
/// (4,097 refused by count), a single cell (BANKNIFTY), the oldest month a
/// month can be stored under here (1990, before the calendar's record: owed
/// `null`); combinations — one underlying at the bound inside a segment past
/// it at once (the underlying answered, the segment refused), every rung of
/// the ladder in one month at once; out of the box — 34 years of months in
/// one census partition.
#[test]
fn an_answer_carries_the_cell_bound_and_is_refused_one_past_it() {
    let mut rows: Vec<Held> = (0..CELLS_MOST)
        .map(|n| spot("NIFTY", RUNGS[n % RUNGS.len()], nth_month(n / RUNGS.len())))
        .collect();
    rows.push(spot("BANKNIFTY", Timeframe::SECOND_1, nth_month(410)));
    let (_root, site) = served("cells", Segment::Index, &rows);

    let (code, body) = rollup(&site, "feed=gdfl&segment=INDEX&underlying=NIFTY");
    assert_eq!(code, 200, "{}", body["why"]);
    let months = body["months"].as_array().expect("months");
    assert_eq!(months.len(), CELLS_MOST);
    assert_eq!(months[0]["month"], "2024-02");
    assert_eq!(months[0]["timeframe"], Timeframe::SECOND_1.as_str());
    let oldest = &months[CELLS_MOST - 1];
    assert_eq!(oldest["month"], "1990-01");
    assert_eq!(oldest["timeframe"], Timeframe::DAY_1.as_str());
    assert!(
        pull::calendar::SOURCED_FIRST_DAY > 7_335,
        "the premise: January 1990 is before the calendar's record"
    );
    assert!(oldest["days_owed"].is_null(), "{oldest}");
    assert_eq!(body["rungs"].as_array().map(Vec::len), Some(RUNGS.len()));

    let (code, body) = rollup(&site, "feed=gdfl&segment=INDEX");
    assert_eq!(code, 422, "the segment, one past the bound");
    assert_eq!(body["error"], "rollup_too_many", "{body}");
    assert_eq!(
        why_of(&body),
        "the selection holds 4097 (month, rung) cells; one answer carries at most 4096: name an \
         underlying"
    );

    let (code, body) = rollup(&site, "feed=gdfl&segment=INDEX&underlying=BANKNIFTY");
    assert_eq!(code, 200, "{body}");
    assert_eq!(body["months"].as_array().map(Vec::len), Some(1), "{body}");
    assert_eq!(body["months"][0]["month"], "2024-03", "{body}");
}

/// **A SPAN THE CALENDAR CANNOT COUNT IS `None`, NEVER ZERO.** An inverted
/// span counts no session and says it cannot, where a span of one closed day
/// counts zero: the two are different answers. A cell whose stamps run
/// backwards, or lie wholly after the month it is filed under, holds no
/// span of its month.
///
/// Extremes: singles — a span inverted by one day, a span of one open
/// session (one), a span of one closed day (zero, not `None`), a cell whose
/// first stamp is after its last inside its month; combinations — a cell
/// wholly after its month, so its span is inverted once clipped to the
/// month; out of the box — the widest inversion there is, `i64::MAX` to
/// `i64::MIN`.
#[test]
fn a_span_the_calendar_cannot_count_is_none_never_zero() {
    let day = |year: u16, month: u8, day: u8| {
        i64::from(
            pull::session::Day::new(year, month, day)
                .expect("a day")
                .days_from_epoch(),
        )
    };
    let friday = day(2019, 3, 1);
    assert_eq!(sessions(friday + 1, friday), None, "an inverted span");
    assert_eq!(sessions(i64::MAX, i64::MIN), None, "the widest inversion");
    assert_eq!(sessions(friday, friday), Some(1), "one open session");
    let sunday = day(2019, 3, 3);
    assert_eq!(sessions(sunday, sunday), Some(0), "one closed day");
    let march = YearMonth::new(2019, 3).expect("a month");
    let backwards = MonthCell {
        keys: 1,
        rows: 10,
        first: Some(MARCH + 19 * 86_400_000_000),
        last: Some(MARCH),
    };
    assert_eq!(days_held(march, &backwards), None);
    let after = MonthCell {
        keys: 1,
        rows: 10,
        first: Some(MAY),
        last: Some(MAY + 9_000_000),
    };
    assert_eq!(days_held(march, &after), None);
    let inside = MonthCell {
        first: Some(MARCH),
        last: Some(MARCH + 9_000_000),
        ..after
    };
    assert_eq!(days_held(march, &inside), Some(1));
}

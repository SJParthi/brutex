//! The day band's read-time rules (R3-13), each proved on an invented store.
//!
//! EVERY FIXTURE IS INVENTED: prices are round numbers no index or share
//! printed, volumes are small, and no row is a vendor's. The shapes are the
//! findings', the numbers are not the market's.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]

use super::super::tests::{ask, bar, day, file_bars, record, root_of, says, says_has, write};
use super::super::{Value, answer, answer_at};
use super::*;

/// A gzip member of one stored (uncompressed) deflate block: `api` has no
/// compressor, and the master reader only needs a valid member. The same
/// helper `bars_change`'s tests use.
fn stored_gzip(text: &[u8]) -> Vec<u8> {
    let mut crc = !0u32;
    for &byte in text {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    let len = u16::try_from(text.len()).expect("one stored block");
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff, 1];
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(text);
    out.extend_from_slice(&(!crc).to_le_bytes());
    out.extend_from_slice(&u32::try_from(text.len()).expect("small").to_le_bytes());
    out
}

/// Installs `symbol`'s closing-auction flag for IST day `on` in the store at
/// `root`, as the pull installs a dated master.
fn install_flag(root: &Path, on: &str, symbol: &str, flag: u8) {
    let isin = brutex_core::universe::nse_isin(symbol).expect("an F&O share");
    let csv = format!(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n7,{symbol},EQ,{},{flag}\n",
        isin.as_str()
    );
    pull::cash_session_cache::install(
        &root.join("session-masters"),
        day(on),
        &stored_gzip(csv.as_bytes()),
    )
    .expect("the master lands");
}

/// `n` one-minute bars from 09:15 IST on `on`, at `close` paisa.
fn minutes_at(on: &str, n: i64, close: i64) -> Vec<store::format::Bar> {
    (0..n)
        .map(|i| bar(on, 9 * 3_600 + 15 * 60 + i * 60, [close; 4], 10))
        .collect()
}

// ══ The band's rungs ════════════════════════════════════════════════════════

/// **ONE FUNCTION DECIDES THE PEER AND THE FINEST RUNGS** (R3-13 goal 5): each
/// feed's finest, the other feed, and the other feed's finest, as every reader
/// in `dayflags` takes them.
#[test]
fn the_band_rungs_come_from_one_function() {
    let gdfl = feed_rungs(Vendor::Gdfl);
    assert_eq!(gdfl.finest, Timeframe::SECOND_1);
    assert_eq!(gdfl.peer, Vendor::Zerodha);
    assert_eq!(gdfl.peer_finest, Timeframe::MINUTE_1);
    let zerodha = feed_rungs(Vendor::Zerodha);
    assert_eq!(zerodha.finest, Timeframe::MINUTE_1);
    assert_eq!(zerodha.peer, Vendor::Gdfl);
    assert_eq!(zerodha.peer_finest, Timeframe::SECOND_1);
    assert_eq!(super::super::finest(Vendor::Gdfl), gdfl.finest);
    assert_eq!(super::super::peer_of(Vendor::Zerodha), zerodha.peer);
}

/// `timeframe=` is a rung the store files, or absent, or refused by name.
#[test]
fn the_asked_rung_is_a_filed_rung_or_refused() {
    assert_eq!(asked_rung(""), Ok(None));
    assert_eq!(asked_rung("1day"), Ok(Some(Timeframe::DAY_1)));
    assert_eq!(asked_rung("1min"), Ok(Some(Timeframe::MINUTE_1)));
    let why = asked_rung("7min").expect_err("not a rung");
    assert!(
        why.starts_with("timeframe=\"7min\" is not a rung this store files: one of ")
            && why.contains("1day")
            && why.contains("1s"),
        "{why}"
    );
}

/// **THE BAND COUNTS WHAT THE GRID SHOWS** (R3-13 goal 1; DB-053). On a
/// Zerodha `1day` grid a session whose minutes are held and whose day bar is
/// not is the day bar MISSING, said with the minutes beside it; a day bar
/// with no minutes is held, and never "no Zerodha bar". Counted at the
/// finest rung (the band before this), the second day read "no Zerodha bar
/// on this session" above the day bar the grid showed, and the first read
/// as an ordinary session.
#[test]
fn the_band_counts_the_asked_timeframe() {
    let root = root_of("r3-13-asked-rung");
    let key = "NSE-INDEX-NIFTY";
    // 2024-10-03 holds minutes and no day bar; 2024-10-04 a day bar and no
    // minutes; 2024-10-01 both. 2024-10-02 is a holiday.
    file_bars(
        &root,
        Vendor::Zerodha,
        key,
        Timeframe::MINUTE_1,
        [
            minutes_at("2024-10-01", 375, 1_000_000),
            minutes_at("2024-10-03", 375, 1_000_000),
        ]
        .concat(),
    );
    file_bars(
        &root,
        Vendor::Zerodha,
        key,
        Timeframe::DAY_1,
        vec![
            bar("2024-10-01", 0, [1_000_000; 4], 0),
            bar("2024-10-04", 0, [1_000_000; 4], 0),
        ],
    );
    let asked = ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-10-01&to=2024-10-04");

    // BEFORE: the finest rung, which the band counted whatever the grid showed.
    let before = answer(&root, &asked);
    assert_eq!(before["rungs"]["held"]["rung"], "1min");
    says_has(
        record(&before, "2024-10-04"),
        "no Zerodha bar on this session",
    );
    assert_eq!(record(&before, "2024-10-03")["held"]["bars"], 375);

    // AFTER: the grid's rung.
    let got = answer_at(&root, &asked, Some(Timeframe::DAY_1));
    assert_eq!(got["rungs"]["held"]["rung"], "1day");
    assert_eq!(got["rungs"]["finest"]["rung"], "1min");
    let missing = record(&got, "2024-10-03");
    assert_eq!(missing["held"]["bars"], 0);
    assert_eq!(missing["held"]["rung"], "1day");
    assert_eq!(missing["finest"]["bars"], 375);
    assert_eq!(missing["finest"]["rung"], "1min");
    says_has(
        missing,
        "no Zerodha 1day bar on this session, while Zerodha holds 375 1min bar(s) that day: \
         the 1day bar is missing",
    );
    let shown = record(&got, "2024-10-04");
    assert_eq!(shown["held"]["bars"], 1);
    assert!(
        says(shown)
            .iter()
            .all(|said| !said.starts_with("no Zerodha")),
        "{:#?}",
        says(shown)
    );
    let both = record(&got, "2024-10-01");
    assert_eq!(both["held"]["bars"], 1);
    assert_eq!(both["finest"]["bars"], 375);
    // A holiday holds nothing and owes nothing.
    assert_eq!(record(&got, "2024-10-02")["held"]["bars"], 0);
    assert!(
        says(record(&got, "2024-10-02"))
            .iter()
            .all(|said| !said.starts_with("no Zerodha"))
    );
    // AT THE FINEST RUNG THE ANSWER IS THE ONE IT ALWAYS WAS: no `finest`
    // object, the zero-volume count beside `held`.
    let same = answer_at(&root, &asked, Some(Timeframe::MINUTE_1));
    assert_eq!(same["days"], before["days"]);
    assert_eq!(record(&same, "2024-10-03")["finest"], Value::Null);
    // A month with no file at the grid's rung says so, at that rung.
    let empty = answer_at(
        &root,
        &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-11-04&to=2024-11-04"),
        Some(Timeframe::DAY_1),
    );
    says_has(
        record(&empty, "2024-11-04"),
        "Zerodha holds no 1day file for this month",
    );
}

// ══ A share's dated closing-auction flag ═════════════════════════════════════

/// **THE PHASES FOLLOW THE SHARE'S DATED FLAG** (SC-CLOSE-8). Flag 1: the
/// closing auction from 15:15, 360 minutes owed. Flag 0: continuous to 15:29,
/// 375 owed, nothing unverified. No master that day: 360 owed, 15 unverified,
/// and a note says why. Before this every share read the third way on every
/// day, flag or not, while `/bars/window.json` labelled the same minutes from
/// the flag.
#[test]
fn a_shares_phases_follow_its_dated_closing_auction_flag() {
    let root = root_of("r3-13-cas-flag");
    install_flag(&root, "2026-08-04", "RELIANCE", 1);
    install_flag(&root, "2026-08-05", "RELIANCE", 0);
    let asked = ask("feed=zerodha&instrument=NSE-CASH-RELIANCE&from=2026-08-04&to=2026-08-06");
    let got = answer(&root, &asked);

    let eligible = &record(&got, "2026-08-04")["calendar"];
    assert_eq!(eligible["cas"]["eligible"], true, "{eligible:#}");
    assert_eq!(eligible["cas"]["basis"], "dated_master");
    assert_eq!(eligible["owed_minutes"], 360);
    assert_eq!(eligible["unverified_minutes"], 0);
    let phases = &record(&got, "2026-08-04")["events"]["phases"];
    assert!(
        phases
            .as_array()
            .expect("phases")
            .iter()
            .any(|band| band["phase"] == "cas" && band["from"] == "15:15"),
        "{phases:#}"
    );

    let not = &record(&got, "2026-08-05")["calendar"];
    assert_eq!(not["cas"]["eligible"], false, "{not:#}");
    assert_eq!(not["owed_minutes"], 375);
    assert_eq!(not["unverified_minutes"], 0);

    let unread = &record(&got, "2026-08-06")["calendar"];
    assert_eq!(unread["cas"]["eligible"], Value::Null, "{unread:#}");
    assert_eq!(unread["cas"]["basis"], "unknown");
    assert!(
        unread["cas"]["why"]
            .as_str()
            .is_some_and(|why| why.starts_with("2026-08-06: ")),
        "{unread:#}"
    );
    assert_eq!(unread["owed_minutes"], 360);
    assert_eq!(unread["unverified_minutes"], 15);
    let notes = got["notes"].as_array().expect("notes");
    assert!(
        notes.iter().any(|note| note.as_str().is_some_and(|n| {
            n.starts_with(
                "The closing-auction flag could not be read, so the day's 15:15-15:29 stays \
                 unverified: 2026-08-06: ",
            )
        })),
        "{notes:#?}"
    );
    // The old blanket sentence is gone: the route reads the flag now.
    assert!(
        notes.iter().all(|note| !note
            .as_str()
            .unwrap_or("")
            .contains("does not read the dated")),
        "{notes:#?}"
    );
}

/// No flag is read, or claimed, where none applies: an index, and a share
/// before 2026-08-03. A share the universe has no ISIN for says so.
#[test]
fn no_flag_is_read_where_none_applies() {
    let root = root_of("r3-13-cas-none");
    let index = answer(
        &root,
        &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2026-08-04&to=2026-08-04"),
    );
    assert_eq!(record(&index, "2026-08-04")["calendar"]["cas"], Value::Null);
    let early = answer(
        &root,
        &ask("feed=zerodha&instrument=NSE-CASH-RELIANCE&from=2026-07-31&to=2026-07-31"),
    );
    assert_eq!(record(&early, "2026-07-31")["calendar"]["cas"], Value::Null);
    assert_eq!(
        record(&early, "2026-07-31")["calendar"]["owed_minutes"],
        375
    );
    let mut flags = CasFlags::of(&root, "NOTASHARE", true);
    let flag = flags.on(day("2026-08-04"));
    assert_eq!(flag.eligibility, CasEligibility::Unknown);
    assert_eq!(flag.basis, CasBasis::NoIdentity);
    assert_eq!(
        flag.json()["why"],
        "no ISIN for this symbol in the universe, so no master row can name it"
    );
    // A master that holds no row for the share is the master's own refusal.
    install_flag(&root, "2026-08-04", "RELIANCE", 1);
    let mut other = CasFlags::of(&root, "INFY", true);
    let flag = other.on(day("2026-08-04"));
    assert_eq!(flag.eligibility, CasEligibility::Unknown);
    assert!(
        matches!(&flag.basis, CasBasis::Unread(why) if why.starts_with("2026-08-04: ")),
        "{flag:?}"
    );
    assert_eq!(other.refusals().len(), 1);
    // Asked again in the same request, the refusal is not re-read but kept.
    let again = other.on(day("2026-08-04"));
    assert_eq!(again.basis, flag.basis);
}

/// **A REFUSED LOAD IS READ AGAIN BY THE NEXT REQUEST** (AF-03's rule, kept):
/// a master that lands after a request refused it is read by the next one.
#[test]
fn a_flag_refused_by_one_request_is_read_by_the_next() {
    let root = root_of("r3-13-cas-retry");
    let mut first = CasFlags::of(&root, "RELIANCE", true);
    assert_eq!(
        first.on(day("2026-08-07")).eligibility,
        CasEligibility::Unknown
    );
    install_flag(&root, "2026-08-07", "RELIANCE", 1);
    // The same request keeps its refusal: one read of the day per request.
    assert_eq!(
        first.on(day("2026-08-07")).eligibility,
        CasEligibility::Unknown
    );
    let mut second = CasFlags::of(&root, "RELIANCE", true);
    let flag = second.on(day("2026-08-07"));
    assert_eq!(flag.eligibility, CasEligibility::Eligible);
    assert_eq!(flag.basis, CasBasis::DatedMaster);
    assert!(second.refusals().is_empty());
}

// ══ The auction sidecar ══════════════════════════════════════════════════════

/// **"KEPT APART" ONLY WHERE A SIDECAR KEEPS THEM** (SC-CLOSE-9). A GDFL share
/// day whose stocks line counts auction prints says the prints were counted
/// and are NOT kept apart while the month has no `.auc`; with one there it
/// says they are. Before, every such day said "kept apart from trades".
#[test]
fn an_auction_print_is_said_kept_apart_only_where_a_sidecar_is() {
    let root = root_of("r3-13-sidecar");
    let line = super::super::tests::flags_line(
        "stocks",
        "2026-08-04",
        "*",
        &super::super::STOCKS_FIELDS,
        &[("auction_prints", "3")],
    );
    write(&super::super::quality_path(&root), &line);
    let asked = ask("feed=gdfl&instrument=NSE-CASH-RELIANCE&from=2026-08-04&to=2026-08-04");
    let got = answer(&root, &asked);
    let rec = record(&got, "2026-08-04");
    assert_eq!(rec["auction_sidecar"], false);
    says_has(
        rec,
        "3 closing-auction print(s) counted by the import; this month holds no auction \
         sidecar, so no print is kept apart from trades",
    );
    let key = brutex_core::instrument::InstrumentKey {
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Cash,
        underlying: brutex_core::symbol::Symbol::new("RELIANCE").expect("a symbol"),
        kind: brutex_core::instrument::Kind::Equity,
    };
    let path = pull::gdfl_cm::auction::path(&root, &key, day("2026-08-04")).expect("a path");
    write(&path, "");
    let got = answer(&root, &asked);
    let rec = record(&got, "2026-08-04");
    assert_eq!(rec["auction_sidecar"], true);
    says_has(
        rec,
        "3 closing-auction print(s), kept apart from trades in the month's auction sidecar",
    );
    // Zerodha keeps no sidecar and none is looked for; nor before 2026-08-03.
    let zerodha = answer(
        &root,
        &ask("feed=zerodha&instrument=NSE-CASH-RELIANCE&from=2026-08-04&to=2026-08-04"),
    );
    assert_eq!(
        record(&zerodha, "2026-08-04")["auction_sidecar"],
        Value::Null
    );
    let early = answer(
        &root,
        &ask("feed=gdfl&instrument=NSE-CASH-RELIANCE&from=2026-07-31&to=2026-07-31"),
    );
    assert_eq!(record(&early, "2026-07-31")["auction_sidecar"], Value::Null);
    let mut none = AuctionSidecars::of(&root, Vendor::Gdfl, "NIFTY", false);
    assert_eq!(none.on(day("2026-08-04")), None);
}

// ══ A contract's expiry ═════════════════════════════════════════════════════

/// **A DAY AFTER EXPIRY IS EXPIRED AND OWES NOTHING** (SC-CLOSE-12), as
/// `/gaps.json` says: owed 0, `contract.expired`, one phrase, and no "no GDFL
/// bar". Before, 2026-08-05 owed 385 and read as a session with no trade.
#[test]
fn a_day_after_expiry_is_expired_and_owes_nothing() {
    let root = root_of("r3-13-expired");
    let key = "NSE-FNO-NIFTY-2026-08-04-2700000-CE";
    file_bars(
        &root,
        Vendor::Gdfl,
        key,
        Timeframe::SECOND_1,
        vec![bar("2026-08-04", 10 * 3_600, [5_000; 4], 75)],
    );
    let got = answer(
        &root,
        &ask(&format!(
            "feed=gdfl&instrument={key}&from=2026-08-04&to=2026-08-05"
        )),
    );
    let last = record(&got, "2026-08-04");
    assert_eq!(last["contract"]["expiry"], "2026-08-04");
    assert_eq!(last["contract"]["expired"], false);
    assert_eq!(last["calendar"]["expired"], false);
    assert_ne!(last["calendar"]["owed_minutes"], 0);
    let after = record(&got, "2026-08-05");
    assert_eq!(after["contract"]["expired"], true);
    assert_eq!(after["calendar"]["expired"], true);
    assert_eq!(after["calendar"]["owed_minutes"], 0);
    assert_eq!(after["calendar"]["unverified_minutes"], 0);
    says_has(
        after,
        "expired: the contract's last session was 2026-08-04, so this day owes nothing",
    );
    assert!(
        says(after)
            .iter()
            .all(|said| !said.starts_with("no GDFL") && !said.contains("holds no file")),
        "{:#?}",
        says(after)
    );
    // A spot series has no contract object at all.
    let spot = answer(
        &root,
        &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2026-08-05&to=2026-08-05"),
    );
    assert_eq!(record(&spot, "2026-08-05")["contract"], Value::Null);
}

/// The expiry is the contract's first ten bytes, a future's too.
#[test]
fn a_contracts_expiry_is_the_date_it_opens_with() {
    let parse = |text: &str| Contract::parse(text);
    assert_eq!(
        expiry_of(parse("2019-03-28-1150000-CE")),
        Some(day("2019-03-28"))
    );
    assert_eq!(expiry_of(parse("2026-08-27-FUT")), Some(day("2026-08-27")));
    assert_eq!(expiry_of(None), None);
}

// ══ The month before ════════════════════════════════════════════════════════

/// **A SCALE SWITCH IS FOUND ACROSS FAR MONTHS** (R3-13 goal 2, DB-031). An
/// index whose previous held `1day` bar lies 22 months before the page —
/// re-scaled by the vendor across the hole — is `impossible_jump` on the
/// page's first day. The twelve month opens before this found no previous
/// bar, and the day read clean.
#[test]
fn a_scale_switch_is_found_across_far_months() {
    let root = root_of("r3-13-far-months");
    let key = "NSE-INDEX-NIFTYGSCOMPSITE";
    file_bars(
        &root,
        Vendor::Zerodha,
        key,
        Timeframe::DAY_1,
        vec![
            bar("2016-02-29", 0, [3_000_000; 4], 0),
            bar("2016-03-01", 0, [3_000_000; 4], 0),
            bar("2018-01-01", 0, [250_000; 4], 0),
            bar("2018-01-02", 0, [251_000; 4], 0),
        ],
    );
    let got = answer(
        &root,
        &ask(&format!(
            "feed=zerodha&instrument={key}&from=2018-01-01&to=2018-01-02"
        )),
    );
    let first = record(&got, "2018-01-01");
    assert_eq!(
        first["junk"],
        serde_json::json!(["impossible_jump"]),
        "{first:#}"
    );
    says_has(
        first,
        "impossible jump: the close is at least 2.5 times, or at most 0.4 of, the previous close",
    );
    assert_eq!(record(&got, "2018-01-02")["junk"], serde_json::json!([]));
    // The series is held before the page, so no day of it is a front.
    assert_eq!(got["zerodha_1day_start"]["state"], "held_before");
}

/// The probe walks back month by month, at most [`MONTHS_BACK`], and stops
/// at the first held month.
#[test]
fn the_month_before_is_found_by_stats_within_its_bound() {
    let month = |y: u16, m: u8| YearMonth::new(y, m).expect("a month");
    let mut asked = Vec::new();
    let found = held_month_before(month(2018, 1), |probe| {
        asked.push(probe);
        probe == month(2016, 3)
    });
    assert_eq!(found, Some(month(2016, 3)));
    assert_eq!(asked.first(), Some(&month(2017, 12)));
    assert_eq!(asked.len(), 22);
    let mut calls = 0;
    assert_eq!(
        held_month_before(month(2018, 1), |_| {
            calls += 1;
            false
        }),
        None
    );
    assert_eq!(calls, MONTHS_BACK);
}

// ══ Late rows on a recounted day ═════════════════════════════════════════════

/// **A RECOUNTED DAY KEEPS ITS LATE ROWS** (R3-13 goal 2; DB-029). The census
/// recount closes 2019-11-14 with a line that counts no late row and reads no
/// vendor row, so the import's close before it still says how many rows were
/// deferred and how far back: those counts stand, named by their journal line,
/// and the phrase says they are the day's total over all its files. Before,
/// the recounted day's `late` was null and the band said nothing.
#[test]
fn a_recounted_day_keeps_its_late_rows() {
    let root = root_of("r3-13-recount-late");
    write(
        &pull::gdfl_import::journal_path(&root),
        "begin options 2019-11-14 *\n\
         incomplete options 2019-11-14 * definition=2 files=12 seconds=333 failures=1 late=4210 late_unresolved=0 max_back_s=118\n\
         recounted options 2019-11-14 * definition=2 months=3 run=1791300001\n\
         begin options 2019-11-13 *\n\
         done options 2019-11-13 * definition=2 files=11 seconds=222 failures=0 late=7 late_unresolved=0 max_back_s=3\n",
    );
    let got = answer(
        &root,
        &ask(
            "feed=gdfl&instrument=NSE-FNO-NIFTY-2019-11-28-1180000-CE&from=2019-11-13&to=2019-11-14",
        ),
    );
    let recounted = &record(&got, "2019-11-14")["import"];
    assert_eq!(recounted["outcome"], "recounted", "{recounted:#}");
    assert_eq!(recounted["late"], 4210);
    assert_eq!(recounted["max_back_s"], 118);
    assert_eq!(recounted["late_line"], 2);
    assert_eq!(recounted["late_from"], "close_before_recount");
    assert_eq!(recounted["late_scope"], "all files of the kind-day");
    says_has(
        record(&got, "2019-11-14"),
        "4210 late row(s) over all of the day's options files deferred to a later second, at \
         most 118 s back (journal line 2, the close before the census recount, which reads no row)",
    );
    // A day its own close counted reads that close, named as such.
    let done = &record(&got, "2019-11-13")["import"];
    assert_eq!(done["late"], 7);
    assert_eq!(done["late_line"], 5);
    assert_eq!(done["late_from"], "closing_line");
    says_has(
        record(&got, "2019-11-13"),
        "7 late row(s) over all of the day's options files deferred to a later second, at most 3 s back",
    );
}

/// The late ledger keeps, per kind, day and filter, the LAST close that counted
/// late rows, and ignores every other line.
#[test]
fn the_late_ledger_keeps_the_last_counting_close() {
    use super::super::{LateClose, LateLedger, Ledger as _};
    let mut ledger = LateLedger::default();
    ledger.line(1, "begin options 2019-11-14 *");
    ledger.line(2, "done options 2019-11-14 * late=5 max_back_s=2");
    ledger.line(3, "refused options 2019-11-14 * late=9");
    ledger.line(4, "recounted options 2019-11-14 * definition=2 months=3");
    ledger.line(5, "done options 2019-11-14 NIFTY late=1 max_back_s=1");
    ledger.line(6, "done options 2019-11-15 * files=3");
    ledger.line(7, "done bogus 2019-11-15 * late=3");
    ledger.line(8, "done options 2019-13-15 * late=3");
    ledger.line(9, "done options 2019-11-16");
    ledger.line(10, "incomplete stocks 2019-11-16 * late=05 max_back_s=4");
    let kind = pull::gdfl_import::ImportKind::Options;
    assert_eq!(
        ledger.get(day("2019-11-14"), kind, "*"),
        Some(LateClose {
            late: 9,
            max_back_s: None,
            line: 3
        })
    );
    assert_eq!(
        ledger.get(day("2019-11-14"), kind, "NIFTY"),
        Some(LateClose {
            late: 1,
            max_back_s: Some(1),
            line: 5
        })
    );
    assert_eq!(ledger.get(day("2019-11-15"), kind, "*"), None);
    assert_eq!(ledger.get(day("2019-11-16"), kind, "*"), None);
    // A count not written the canonical way is no count.
    assert_eq!(
        ledger.get(
            day("2019-11-16"),
            pull::gdfl_import::ImportKind::Stocks,
            "*"
        ),
        None
    );
}

// ══ The bar definition that built a day ═════════════════════════════════════

/// **AN OPTION DAY NAMES ITS BAR DEFINITION** (R3-13 goal 3). The journal's
/// deciding line names 2 for one day and 4 for the next: the first is labelled
/// until it is restated, the second holds this build's bars. Where the journal
/// names none, the day ledger's line does, and says so.
#[test]
fn an_option_day_names_its_bar_definition() {
    let root = root_of("r3-13-definition");
    write(
        &pull::gdfl_import::journal_path(&root),
        "done options 2024-10-01 * definition=2 files=1 seconds=1 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
         done options 2024-10-03 * definition=4 files=1 seconds=1 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
         done options 2024-10-04 * definition=3 files=1 seconds=1 failures=0 late=0 late_unresolved=0 max_back_s=0\n",
    );
    let ledger = super::super::tests::flags_line(
        "options",
        "2024-10-07",
        "*",
        &super::super::OPTIONS_FIELDS,
        &[],
    )
    .replace("definition=2", "definition=3");
    write(&super::super::quality_path(&root), &ledger);
    let got = answer(
        &root,
        &ask(
            "feed=gdfl&instrument=NSE-FNO-NIFTY-2024-10-31-2500000-CE&from=2024-10-01&to=2024-10-07",
        ),
    );
    let old = &record(&got, "2024-10-01")["bar_definition"];
    assert_eq!(old["value"], 2, "{old:#}");
    assert_eq!(old["from"], "import_journal");
    assert_eq!(old["current"], false);
    assert_eq!(old["this_build"], 4);
    says_has(
        record(&got, "2024-10-01"),
        "bar definition 2 built this day: placement over every row; an option second's open \
         interest is its last traded row's (D-2812); this build builds definition 4, so the day \
         is labelled until it is restated",
    );
    let new = &record(&got, "2024-10-03")["bar_definition"];
    assert_eq!(new["value"], 4);
    assert_eq!(new["current"], true);
    assert!(
        says(record(&got, "2024-10-03")).iter().any(|said| said
            .starts_with("bar definition 4 built this day: from 2026-08-03")
            && said.ends_with("(D-4878 item 10)")),
        "{:#?}",
        says(record(&got, "2024-10-03"))
    );
    says_has(
        record(&got, "2024-10-04"),
        "bar definition 3 built this day: an option second's open interest is the last row \
         placed in it, traded or not (D-3342); it holds the bars definition 4 builds",
    );
    let ledgered = &record(&got, "2024-10-07")["bar_definition"];
    assert_eq!(ledgered["value"], 3);
    assert_eq!(ledgered["from"], "day_ledger");
    // A day neither source names has none, and a Zerodha record carries none.
    assert_eq!(record(&got, "2024-10-02")["bar_definition"], Value::Null);
    let zerodha = answer(
        &root,
        &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-10-01&to=2024-10-01"),
    );
    assert_eq!(
        record(&zerodha, "2024-10-01")["bar_definition"],
        Value::Null
    );
}

/// A day of another kind is named only where its definition is not current;
/// every definition has its sentence.
#[test]
fn a_spot_day_names_its_definition_only_when_it_is_not_current() {
    use pull::gdfl_import::ImportKind;
    let index = definition_json(
        ImportKind::Indices,
        day("2024-10-01"),
        Some((2, DefinitionFrom::Journal)),
    );
    assert_eq!(index["current"], true);
    assert_eq!(definition_says(ImportKind::Indices, &index), None);
    let share = definition_json(
        ImportKind::Stocks,
        day("2026-08-04"),
        Some((3, DefinitionFrom::DayLedger)),
    );
    assert_eq!(share["current"], false);
    assert!(
        definition_says(ImportKind::Stocks, &share)
            .is_some_and(|said| said.ends_with("so the day is labelled until it is restated"))
    );
    assert_eq!(
        definition_json(ImportKind::Options, day("2024-10-01"), None),
        Value::Null
    );
    assert_eq!(definition_says(ImportKind::Options, &Value::Null), None);
    for definition in 1..=4 {
        assert_ne!(definition_means(definition), definition_means(9));
    }
    assert_eq!(definition_means(9), "a definition this build does not name");
}

// ══ A day field the reader has never seen ════════════════════════════════════

/// **A DAY FIELD THE READER HAS NEVER SEEN IS SERVED** (R3-13 goal 5): a later
/// writer's field rides on the line's object under its own name — a count as a
/// number, anything else as its text — and `unknown` lists it, so the band
/// draws it with no change here. A name the object keeps for itself, or one
/// written twice, refuses the line by name. Before, the field was dropped.
#[test]
fn a_day_field_the_reader_has_never_seen_is_served_and_drawn() {
    let root = root_of("r3-13-unknown-field");
    let line = super::super::tests::index_flags("2024-10-01", "NIFTY", &[])
        .replace('\n', " frozen_runs=2 frozen_note=vendor_flat\n");
    write(&super::super::quality_path(&root), &line);
    let got = answer(
        &root,
        &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-10-01&to=2024-10-01"),
    );
    let indices = &record(&got, "2024-10-01")["quality"]["indices"];
    assert_eq!(indices["frozen_runs"], 2, "{indices:#}");
    assert_eq!(indices["frozen_note"], "vendor_flat");
    assert_eq!(
        indices["unknown"],
        serde_json::json!(["frozen_runs", "frozen_note"])
    );
    assert_eq!(indices["holes_5s"], 0);
    // A known line with no new field lists none.
    let plain = super::super::parse_quality(
        1,
        super::super::tests::index_flags("2024-10-01", "NIFTY", &[]).trim_end(),
    )
    .expect("a line");
    assert_eq!(plain.json()["unknown"], serde_json::json!([]));
    for (bad, why) in [
        (
            " line=4",
            "line is a key this reader serves for itself, not a field",
        ),
        (
            " scope=x",
            "scope is a key this reader serves for itself, not a field",
        ),
        (" z=1 z=2", "z appears twice"),
    ] {
        let text = super::super::tests::index_flags("2024-10-01", "NIFTY", &[])
            .trim_end()
            .to_owned()
            + bad;
        assert_eq!(
            super::super::parse_quality(1, &text).err().as_deref(),
            Some(why),
            "{text}"
        );
    }
}

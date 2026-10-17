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

use super::super::tests::{
    append, ask, bar, day, file_bars, record, root_of, says, says_has, write,
};
use super::super::{Value, answer, answer_at};
use super::*;
use std::fmt::Write as _;

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
    let got = answer_at(
        &root,
        &asked,
        Band {
            rung: Some(Timeframe::DAY_1),
            expiry: None,
        },
    );
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
    let same = answer_at(
        &root,
        &asked,
        Band {
            rung: Some(Timeframe::MINUTE_1),
            expiry: None,
        },
    );
    assert_eq!(same["days"], before["days"]);
    assert_eq!(record(&same, "2024-10-03")["finest"], Value::Null);
    // A month with no file at the grid's rung says so, at that rung.
    let empty = answer_at(
        &root,
        &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-11-04&to=2024-11-04"),
        Band {
            rung: Some(Timeframe::DAY_1),
            expiry: None,
        },
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

/// **A PAGE READS A FLAG ONLY ON AN OPEN SESSION UP TO TODAY** (the R3-13
/// review): a closed day has no closing auction, so nothing is read or
/// refused for it; a session after today has no master dated it yet, and is
/// said so rather than refused.
#[test]
fn a_flag_is_read_only_on_an_open_session_up_to_today() {
    let root = root_of("r3-13-cas-session");
    install_flag(&root, "2026-08-04", "RELIANCE", 1);
    let mut flags = CasFlags::of(&root, "RELIANCE", true);
    // A closed day: not required, nothing read.
    let closed = flags.on_session(day("2026-08-08"), false, false);
    assert_eq!(closed.basis, CasBasis::NotRequired);
    assert_eq!(closed.json(), Value::Null);
    // A session after today: not yet, never a refusal.
    let future = flags.on_session(day("2026-08-10"), true, true);
    assert_eq!(future.eligibility, CasEligibility::Unknown);
    assert_eq!(future.basis, CasBasis::NotYet);
    assert_eq!(
        future.json(),
        serde_json::json!({
            "eligible": null, "basis": "not_yet",
            "why": "a session after today: no master is dated it yet",
        })
    );
    assert!(flags.refusals().is_empty(), "nothing was refused");
    // An open session up to today reads its master.
    let open = flags.on_session(day("2026-08-04"), true, false);
    assert_eq!(open.eligibility, CasEligibility::Eligible);
    assert_eq!(open.basis, CasBasis::DatedMaster);
    // A MASTER THAT LOADED IS KEPT FOR THE PROCESS, by its directory and day.
    assert!(
        masters()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(&(
                root.join("session-masters"),
                day("2026-08-04").days_from_epoch()
            )),
        "the loaded master is kept"
    );
    // An index, or a day before the auction began, needs no flag, open or not,
    // and asked directly reads none either.
    let mut index = CasFlags::of(&root, "NIFTY", false);
    assert_eq!(
        index.on_session(day("2026-08-04"), true, false).basis,
        CasBasis::NotRequired
    );
    assert_eq!(index.on(day("2026-08-04")).basis, CasBasis::NotRequired);
    assert_eq!(flags.on(day("2026-07-31")).basis, CasBasis::NotRequired);
    assert_eq!(
        flags.on_session(day("2026-07-31"), true, true).basis,
        CasBasis::NotRequired
    );
    // The page's days, with today given: Tue 2026-08-04 is today's own session
    // and read, Sat 08-08 is closed and Mon 08-10 after today, so neither is
    // read and no note says a flag could not be read.
    let asked = ask("feed=zerodha&instrument=NSE-CASH-RELIANCE&from=2026-08-04&to=2026-08-10");
    let days = [day("2026-08-04"), day("2026-08-08"), day("2026-08-10")];
    let mut notes = Vec::new();
    let (got, sidecars) =
        super::super::auction_of_days(&root, &asked, (&days, Some(day("2026-08-04"))), &mut notes);
    assert_eq!(
        got.iter()
            .map(|flag| flag.basis.clone())
            .collect::<Vec<_>>(),
        vec![
            CasBasis::DatedMaster,
            CasBasis::NotRequired,
            CasBasis::NotYet
        ]
    );
    assert!(notes.is_empty(), "{notes:#?}");
    assert_eq!(
        sidecars,
        vec![None, None, None],
        "a Zerodha share keeps no sidecar"
    );
    // With no today known, every open session is read: 08-10's master is not
    // there, and that is said.
    let mut notes = Vec::new();
    let (got, _) = super::super::auction_of_days(&root, &asked, (&days, None), &mut notes);
    assert!(
        matches!(&got[2].basis, CasBasis::Unread(why) if why.starts_with("2026-08-10: ")),
        "{got:?}"
    );
    assert_eq!(notes.len(), 1, "{notes:#?}");
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
        "3 closing-auction print(s) in the day's share files; this share's month holds no \
         auction sidecar, so its print is not kept apart from trades",
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
        "3 closing-auction print(s) in the day's share files; this share's month keeps its print \
         apart from trades in its auction sidecar",
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
    // An index record carries the day's stocks line too: it names the count and
    // claims nothing about a sidecar it never looked for (the R3-13 review).
    let index = answer(
        &root,
        &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2026-08-04&to=2026-08-04"),
    );
    let rec = record(&index, "2026-08-04");
    assert_eq!(rec["auction_sidecar"], Value::Null);
    says_has(
        rec,
        "3 closing-auction print(s) in the day's share files, counted by the import",
    );
    let mut none = AuctionSidecars::of(&root, Vendor::Gdfl, "NIFTY", false);
    assert_eq!(none.on(day("2026-08-04")), None);
    // A stocks line that counts no print says nothing of an auction print.
    let quiet = super::super::tests::flags_line(
        "stocks",
        "2026-08-05",
        "*",
        &super::super::STOCKS_FIELDS,
        &[("auction_prints", "0")],
    );
    super::super::tests::append(&super::super::quality_path(&root), &quiet);
    let got = answer(
        &root,
        &ask("feed=gdfl&instrument=NSE-CASH-RELIANCE&from=2026-08-05&to=2026-08-05"),
    );
    let rec = record(&got, "2026-08-05");
    assert_eq!(rec["quality"]["stocks"]["auction_prints"], 0, "{rec:#}");
    assert!(
        says(rec)
            .iter()
            .all(|say| !say.contains("closing-auction print(s)")),
        "{:#?}",
        says(rec)
    );
    // ONE STAT A MONTH: the month's second day is answered from the first's.
    let mut held = AuctionSidecars::of(&root, Vendor::Gdfl, "RELIANCE", true);
    assert_eq!(held.on(day("2026-08-04")), Some(true));
    assert_eq!(held.on(day("2026-08-05")), Some(true));
    assert_eq!(held.seen.len(), 1, "one month, stated once");
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

/// **A FILTERED RUN'S LATE ROWS ARE OVER THE FILES IT READ** (the R3-13
/// review): a close written by a run given a filter counted the files that
/// filter names, not every file of the kind-day.
#[test]
fn a_filtered_runs_late_rows_are_over_the_files_it_read() {
    let root = root_of("r3-13-filtered-late");
    write(
        &pull::gdfl_import::journal_path(&root),
        "begin indices 2021-05-21 BANKNIFTY,NIFTY\n\
         done indices 2021-05-21 BANKNIFTY,NIFTY definition=4 files=2 seconds=44000 failures=0 late=12 late_unresolved=0 max_back_s=9\n",
    );
    let got = answer(
        &root,
        &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2021-05-21&to=2021-05-21"),
    );
    let rec = record(&got, "2021-05-21");
    let import = &rec["import"];
    assert_eq!(import["late"], 12, "{import:#}");
    assert_eq!(import["filter"], "BANKNIFTY,NIFTY");
    assert_eq!(
        import["late_scope"],
        "the files the closing run's filter names"
    );
    says_has(
        rec,
        "12 late row(s) over the day's indices files its run read (BANKNIFTY,NIFTY) deferred to a \
         later second, at most 9 s back",
    );
    assert!(
        says(rec)
            .iter()
            .all(|say| !say.contains("all of the day's indices files")),
        "{:#?}",
        says(rec)
    );
}

/// **THE LATE LEDGER IS KEPT FOR THE PROCESS AND READ BY ITS TAIL** (L-R3-13
/// (d)): the whole journal once, then a read with nothing appended reads
/// nothing, and an append reads only its bytes.
#[test]
fn the_late_ledger_is_read_by_its_tail() {
    use super::super::{late_tails, read_ledger};
    let root = root_of("r3-13-late-tail");
    let path = pull::gdfl_import::journal_path(&root);
    let first = "done options 2019-11-14 * late=5 max_back_s=2\n";
    write(&path, first);
    let read = read_ledger(late_tails(), &path).expect("the journal reads");
    assert_eq!(read.read_now, read.bytes);
    let again = read_ledger(late_tails(), &path).expect("the journal reads");
    assert_eq!(again.read_now, 0, "nothing new, nothing read");
    let more = "done options 2019-11-15 * late=1 max_back_s=1\n";
    append(&path, more);
    let tail = read_ledger(late_tails(), &path).expect("the journal reads");
    assert_eq!(tail.read_now, u64::try_from(more.len()).expect("a length"));
    let kind = pull::gdfl_import::ImportKind::Options;
    assert_eq!(
        tail.parsed
            .get(day("2019-11-14"), kind, "*")
            .map(|close| close.late),
        Some(5)
    );
    assert_eq!(
        tail.parsed
            .get(day("2019-11-15"), kind, "*")
            .map(|close| close.late),
        Some(1)
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

// ══ A frozen index, and a value outside the day's range ═════════════════════

/// Three consecutive unchanged minutes are a run; two are not; a minute
/// missing between them breaks it; a minute inside a recorded halt is the
/// halt's. The extremes are the day's highest high and lowest low, with when.
#[test]
fn a_frozen_run_is_three_unchanged_minutes_in_a_row() {
    let at = |m: i64| 9 * 3_600 + 15 * 60 + m * 60;
    let flat = |m: i64, v: i64| bar("2022-03-07", at(m), [v; 4], 0);
    let moving = |m: i64, v: i64| bar("2022-03-07", at(m), [v, v + 5, v - 5, v], 0);
    let bars = vec![
        moving(0, 1_000),
        flat(1, 1_000),
        flat(2, 1_000),
        flat(3, 1_000), // 3 flat minutes: a run of 180 s
        moving(4, 1_010),
        flat(5, 1_020),
        flat(6, 1_020), // 2 flat minutes: not a run
        flat(8, 1_020),
        flat(9, 1_020),
        flat(10, 1_020), // minute 7 missing: a new run of 3
        flat(11, 1_030),
    ];
    let scan = fold_minutes(bars.into_iter(), |_| false);
    assert_eq!(scan.bars, 11);
    assert_eq!(scan.frozen_runs, 2);
    assert_eq!(scan.longest.map(|(_, s)| s), Some(180));
    assert_eq!(
        scan.longest.map(|(from, _)| from),
        Some(bar("2022-03-07", at(1), [0; 4], 0).ts_micros)
    );
    assert_eq!(scan.high.map(|(high, _)| high), Some(1_030));
    assert_eq!(scan.low.map(|(low, _)| low), Some(995));
    // Inside a recorded halt, an unchanged value is the halt's.
    let halted = fold_minutes(
        vec![flat(1, 1_000), flat(2, 1_000), flat(3, 1_000)].into_iter(),
        |_| true,
    );
    assert_eq!(halted.frozen_runs, 0);
    assert_eq!(halted.longest, None);
    // A longer run later replaces the longest.
    let longer = fold_minutes(
        (1..=3)
            .map(|m| flat(m, 7))
            .chain((10..=14).map(|m| flat(m, 8))),
        |_| false,
    );
    assert_eq!(longer.frozen_runs, 2);
    assert_eq!(longer.longest.map(|(_, s)| s), Some(300));
    assert_eq!(points(22_985), "229.85");
    assert_eq!(points(-5), "-0.05");
}

/// **A FROZEN INDEX RUN IS FLAGGED ON BOTH FEEDS** (R3-13 goal 2; DB-B20).
/// 2022-03-07: Zerodha's minutes stand at one value 09:57-10:21 and a minute's
/// high is above the day bar's; GDFL's day ledger measured the vendor's rows
/// unchanged for 1,500 s from 09:57:00. Both pages flag the frozen run, and
/// Zerodha's names the high outside the day's range. A run inside the
/// recorded halt of 2020-03-13 is the halt's, never frozen. Before, neither
/// page said either.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one fixture on both feeds and every assertion about it, read top to bottom"
)]
fn a_frozen_index_run_is_flagged_on_both_feeds() {
    let root = root_of("r3-13-frozen");
    let key = "NSE-INDEX-NIFTY";
    let at = |m: i64| 9 * 3_600 + 15 * 60 + m * 60;
    let mut minutes: Vec<store::format::Bar> = (0..375)
        .map(|m| {
            let v = 1_600_000 + (m % 7) * 100;
            bar("2022-03-07", at(m), [v, v + 50, v - 50, v], 0)
        })
        .collect();
    // 09:57 is minute 42; 10:21 is minute 66.
    for m in 42..=66 {
        minutes[usize::try_from(m).unwrap()] = bar("2022-03-07", at(m), [1_590_000; 4], 0);
    }
    // 10:07 prints a high above the day bar's.
    minutes[52] = bar(
        "2022-03-07",
        at(52),
        [1_590_000, 1_640_000, 1_590_000, 1_590_000],
        0,
    );
    // The halt day: a long unchanged stretch inside the recorded halt only.
    let halt_minutes: Vec<store::format::Bar> = (0..375)
        .map(|m| bar("2020-03-13", at(m), [900_000; 4], 0))
        .collect();
    file_bars(
        &root,
        Vendor::Zerodha,
        key,
        Timeframe::MINUTE_1,
        [minutes, halt_minutes].concat(),
    );
    file_bars(
        &root,
        Vendor::Zerodha,
        key,
        Timeframe::DAY_1,
        vec![
            bar(
                "2022-03-07",
                0,
                [1_600_000, 1_617_015, 1_580_000, 1_600_000],
                0,
            ),
            bar("2020-03-13", 0, [900_000; 4], 0),
        ],
    );
    let zerodha = answer(
        &root,
        &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2022-03-07&to=2022-03-07"),
    );
    let rec = record(&zerodha, "2022-03-07");
    let frozen = &rec["index"]["frozen"];
    assert_eq!(frozen["basis"], "minutes_scanned", "{rec:#}");
    assert_eq!(frozen["flagged"], true);
    assert_eq!(frozen["longest_from"], "10:08");
    assert_eq!(frozen["longest_s"], 840);
    says_has(
        rec,
        "index value frozen in session: 2 run(s) of 3 minutes or more with no change, the \
         longest 14 minute(s) from 10:08 to 10:21 (1min), outside any recorded halt",
    );
    let outside = &rec["index"]["outside_day_range"];
    assert_eq!(outside["flagged"], true);
    assert_eq!(outside["above_by"], 22_985);
    assert_eq!(outside["above_at"], "10:07");
    assert_eq!(outside["below_by"], Value::Null);
    says_has(
        rec,
        "intraday high 229.85 points above the day bar's high, at 10:07 (1min)",
    );
    // The halt day's unchanged minutes outside the halt are still flagged;
    // none inside it is: the longest run starts after the halt resumes.
    let halt = answer(
        &root,
        &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2020-03-13&to=2020-03-13"),
    );
    let halted = &record(&halt, "2020-03-13")["index"]["frozen"];
    assert_eq!(halted["flagged"], true, "{halted:#}");
    let from = halted["longest_from"].as_str().expect("a run");
    let resume = pull::calendar::halt_on(i64::from(day("2020-03-13").days_from_epoch()))
        .expect("a recorded halt")
        .seconds(pull::vendor::Venue::NseIndex)
        .resume;
    let resume_hm = format!("{:02}:{:02}", resume / 3_600, resume % 3_600 / 60);
    assert!(
        from >= resume_hm.as_str() || from < "09:15",
        "{from} vs {resume_hm}"
    );

    // GDFL: the day ledger's measure of the vendor's rows.
    let line = super::super::tests::index_flags("2022-03-07", "NIFTY", &[])
        .replace('\n', " flat_max_s=1500 flat_max_at=09:57:00\n");
    let short = super::super::tests::index_flags("2022-03-08", "NIFTY", &[])
        .replace('\n', " flat_max_s=120 flat_max_at=11:00:00\n");
    write(&super::super::quality_path(&root), &(line + &short));
    // A few unchanged seconds held: GDFL's day is still judged by its ledger,
    // never by a scan of its seconds.
    file_bars(
        &root,
        Vendor::Gdfl,
        "NSE-INDEX-NIFTY",
        Timeframe::SECOND_1,
        (0..5)
            .map(|s| bar("2022-03-07", 9 * 3_600 + 57 * 60 + s, [1_600_000; 4], 0))
            .collect(),
    );
    let gdfl = answer(
        &root,
        &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2022-03-07&to=2022-03-08"),
    );
    let rec = record(&gdfl, "2022-03-07");
    let frozen = &rec["index"]["frozen"];
    assert_eq!(frozen["basis"], "day_ledger", "{rec:#}");
    assert_eq!(frozen["flagged"], true);
    assert_eq!(frozen["longest_s"], 1_500);
    assert_eq!(frozen["longest_from"], "09:57:00");
    says_has(
        rec,
        "index value frozen in session: unchanged for 1500 s from 09:57:00 (the import's \
         measure of the vendor's rows), outside any recorded halt",
    );
    // GDFL reads no day bar, so its range check is not made.
    assert_eq!(rec["index"]["outside_day_range"], Value::Null);
    let short = &record(&gdfl, "2022-03-08")["index"]["frozen"];
    assert_eq!(short["flagged"], false, "two minutes is not a run");
    // A share has no index rule at all.
    let share = answer(
        &root,
        &ask("feed=gdfl&instrument=NSE-CASH-RELIANCE&from=2022-03-07&to=2022-03-07"),
    );
    assert_eq!(record(&share, "2022-03-07")["index"], Value::Null);
}

/// A GDFL ledger run that starts inside the recorded halt is the halt's.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one halt day, its boundaries and every placement of a run against it, read together"
)]
fn a_ledger_run_inside_the_halt_is_not_frozen() {
    let halt = pull::calendar::halt_on(i64::from(day("2020-03-13").days_from_epoch()))
        .expect("a recorded halt")
        .seconds(pull::vendor::Venue::NseIndex);
    let inside = IndexDay {
        feed: Vendor::Gdfl,
        midnight: 0,
        scan: None,
        day_bar: None,
        ledger: Some((2_700, Some(halt.from), 4)),
        halt: Some((halt.from, halt.resume)),
    };
    let (json, says) = inside.json();
    assert_eq!(json["frozen"]["flagged"], false);
    assert!(says.is_empty());
    // THE HALT IS `[from, resume)`: its first second is inside, the second it
    // resumes at is not, for a minute bar's stamp as for the ledger's run.
    let on = day("2020-03-13");
    let at = |second: u32| super::super::ist_midnight_micros(on) + i64::from(second) * 1_000_000;
    let venue = pull::vendor::Venue::NseIndex;
    let inside_halt = |second| super::super::in_recorded_halt(venue, on, at(second));
    assert!(!inside_halt(halt.from - 1));
    assert!(inside_halt(halt.from));
    assert!(inside_halt(halt.resume - 1));
    assert!(!inside_halt(halt.resume));
    assert!(!super::super::in_recorded_halt(
        venue,
        day("2020-03-12"),
        at(halt.from)
    ));
    let outside = IndexDay {
        ledger: Some((2_700, Some(halt.resume), 4)),
        ..inside
    };
    assert_eq!(outside.json().0["frozen"]["flagged"], true);
    // 2020-03-13'S RUN STARTS A SECOND BEFORE THE HALT (the R3-13 review): its
    // stamp is 09:21:09 and the halt's 09:21:10, so one second of it is
    // outside the halt, and that is not a frozen index.
    let span = u64::from(halt.resume - halt.from);
    let before = IndexDay {
        ledger: Some((span + 1, Some(halt.from - 1), 4)),
        ..inside
    };
    let (json, says) = before.json();
    assert_eq!(json["frozen"]["flagged"], false, "{json:#}");
    assert_eq!(json["frozen"]["outside_halt_s"], 1);
    assert!(says.is_empty(), "{says:?}");
    // A run that reaches 200 s before the halt is frozen for those 200 s.
    let partial = IndexDay {
        ledger: Some((span + 200, Some(halt.from - 200), 4)),
        ..inside
    };
    let (json, says) = partial.json();
    assert_eq!(json["frozen"]["flagged"], true);
    assert_eq!(json["frozen"]["outside_halt_s"], 200);
    assert_eq!(says.len(), 1);
    assert!(
        says[0].ends_with(", 200 s of it outside the recorded halt"),
        "{says:?}"
    );
    // A run past the halt's resumption counts only its seconds after it.
    let after = IndexDay {
        ledger: Some((span + 179, Some(halt.from), 4)),
        ..inside
    };
    assert_eq!(after.json().0["frozen"]["flagged"], false);
    assert_eq!(after.json().0["frozen"]["outside_halt_s"], 179);
    // On a halt day, a run with no stamp cannot be placed, so it is not flagged.
    let unplaced = IndexDay {
        ledger: Some((5_000, None, 4)),
        ..inside
    };
    let (json, says) = unplaced.json();
    assert_eq!(json["frozen"]["flagged"], false);
    assert_eq!(json["frozen"]["outside_halt_s"], Value::Null);
    assert!(says.is_empty());
    let unknown = IndexDay {
        feed: Vendor::Gdfl,
        midnight: 0,
        scan: None,
        day_bar: None,
        ledger: Some((600, None, 4)),
        halt: None,
    };
    let (json, says) = unknown.json();
    assert_eq!(json["frozen"]["longest_from"], Value::Null);
    assert_eq!(says, vec!["index value frozen in session: unchanged for 600 s from - (the import's measure of the vendor's rows), outside any recorded halt".to_owned()]);
    let nothing = IndexDay {
        feed: Vendor::Zerodha,
        midnight: 0,
        scan: None,
        day_bar: Some((10, 5)),
        ledger: None,
        halt: None,
    };
    assert_eq!(
        nothing.json().0,
        serde_json::json!({ "frozen": null, "outside_day_range": null })
    );
    // A low below the day bar's is named too.
    let low = IndexDay {
        feed: Vendor::Zerodha,
        midnight: 0,
        scan: Some(DayScan {
            bars: 1,
            high: Some((10, 0)),
            low: Some((1, 60_000_000)),
            frozen_runs: 0,
            longest: None,
        }),
        day_bar: Some((10, 5)),
        ledger: None,
        halt: None,
    };
    let (json, says) = low.json();
    assert_eq!(json["outside_day_range"]["below_by"], 4);
    assert_eq!(json["outside_day_range"]["below_at"], "00:01");
    assert_eq!(json["outside_day_range"]["above_by"], Value::Null);
    assert_eq!(json["frozen"]["flagged"], false);
    assert_eq!(
        says,
        vec!["intraday low 0.04 points below the day bar's low, at 00:01 (1min)".to_owned()]
    );
}

// ══ A renamed share's peer ══════════════════════════════════════════════════

/// **A ZERO-VOLUME PEER IS LOOKED UP UNDER A SHARE'S OTHER NAMES** (R3-13
/// goal 2; DB-031). Zerodha holds ZOMATO's day with volume 0; GDFL files the
/// share's seconds under ETERNAL, its newest name (D-3333). The peer is found
/// under that name, so the day is `zero_volume_peer_traded` and the phrase
/// says which name. Before, GDFL "held no file" and the junk row read clean.
#[test]
fn a_zero_volume_peer_is_found_under_the_shares_other_name() {
    let root = root_of("r3-13-other-names");
    file_bars(
        &root,
        Vendor::Zerodha,
        "NSE-CASH-ZOMATO",
        Timeframe::DAY_1,
        vec![bar("2024-10-01", 0, [25_000; 4], 0)],
    );
    file_bars(
        &root,
        Vendor::Gdfl,
        "NSE-CASH-ETERNAL",
        Timeframe::SECOND_1,
        vec![
            bar("2024-10-01", 10 * 3_600, [25_000; 4], 40),
            bar("2024-10-01", 10 * 3_600 + 1, [25_005; 4], 40),
        ],
    );
    let got = answer(
        &root,
        &ask("feed=zerodha&instrument=NSE-CASH-ZOMATO&from=2024-10-01&to=2024-10-01"),
    );
    let rec = record(&got, "2024-10-01");
    assert_eq!(
        rec["junk"],
        serde_json::json!(["zero_volume_peer_traded"]),
        "{rec:#}"
    );
    assert_eq!(rec["peer"]["bars"], 2);
    says_has(
        rec,
        "GDFL holds 2 bar(s), filed under ETERNAL, this share's other name",
    );
    assert_eq!(other_names("ZOMATO"), vec!["ETERNAL"]);
    assert_eq!(other_names("ETERNAL"), vec!["ZOMATO"]);
    assert!(other_names("RELIANCE").is_empty());

    // A page across three months: the share's own name holds August, its
    // other name October, neither September. Only a day its own name holds no
    // month for is looked up under the other, and a month neither holds stays
    // not held, under no name.
    file_bars(
        &root,
        Vendor::Gdfl,
        "NSE-CASH-ZOMATO",
        Timeframe::SECOND_1,
        vec![bar("2024-08-30", 10 * 3_600, [24_000; 4], 10)],
    );
    let got = answer(
        &root,
        &ask("feed=zerodha&instrument=NSE-CASH-ZOMATO&from=2024-08-30&to=2024-10-01"),
    );
    let own = &record(&got, "2024-08-30")["peer"];
    assert_eq!(
        (&own["bars"], &own["as"]),
        (&Value::from(1), &Value::Null),
        "{own:#}"
    );
    let neither = &record(&got, "2024-09-02")["peer"];
    assert_eq!(neither["why"], "not_held", "{neither:#}");
    assert_eq!(neither["as"], Value::Null);
    let other = &record(&got, "2024-10-01")["peer"];
    assert_eq!(
        (&other["bars"], &other["as"]),
        (&Value::from(2), &Value::from("ETERNAL"))
    );
}

// ══ A share's sampled day, and a closed day the peer traded ══════════════════

/// **A SHARE'S DAY NAMES THE MARKET'S SAMPLING AND ITS OWN FIRST SECOND**
/// (CMP-06). 2019-05-30: the market index holds two bars a minute and the
/// share's first second is 11:57:47, the vendor's missing morning. Before, a
/// share's day named nothing and read as fully held.
#[test]
fn a_shares_day_names_the_markets_sampling_and_its_first_second() {
    let root = root_of("r3-13-cmp06");
    let index: Vec<store::format::Bar> = ["2019-05-30", "2019-05-31"]
        .iter()
        .flat_map(|on| {
            (0..375).map(move |m| bar(on, 9 * 3_600 + 15 * 60 + m * 60 + 59, [1_000_000; 4], 0))
        })
        .collect();
    file_bars(
        &root,
        Vendor::Gdfl,
        "NSE-INDEX-NIFTY",
        Timeframe::SECOND_1,
        index,
    );
    file_bars(
        &root,
        Vendor::Gdfl,
        "NSE-CASH-RELIANCE",
        Timeframe::SECOND_1,
        vec![
            bar("2019-05-30", 11 * 3_600 + 57 * 60 + 47, [130_000; 4], 10),
            bar("2019-05-30", 12 * 3_600, [130_005; 4], 10),
        ],
    );
    let got = answer(
        &root,
        &ask("feed=gdfl&instrument=NSE-CASH-RELIANCE&from=2019-05-30&to=2019-05-30"),
    );
    let rec = record(&got, "2019-05-30");
    assert_eq!(rec["sampled"]["of"], "market", "{rec:#}");
    assert_eq!(rec["sampled"]["is"], true);
    assert_eq!(rec["sampled"]["first_held"], "11:57:47");
    says_has(
        rec,
        "degraded vendor day -- market-wide, the index NSE-INDEX-NIFTY: minute-sampled or \
         mostly missing (suspected), 375 of 22500 session second(s) held (1.7%), at most 2 a \
         minute; no day-ledger line measures its rows yet; this share's first held second is \
         11:57:47",
    );
    // A day the market is sampled and the share holds no second says so.
    let got = answer(
        &root,
        &ask("feed=gdfl&instrument=NSE-CASH-RELIANCE&from=2019-05-31&to=2019-05-31"),
    );
    let rec = record(&got, "2019-05-31");
    assert_eq!(rec["sampled"]["first_held"], Value::Null, "{rec:#}");
    assert!(
        says(rec)
            .iter()
            .any(|say| say.starts_with("degraded vendor day -- market-wide")
                && say.ends_with("; this share holds no second that day")),
        "{:#?}",
        says(rec)
    );
}

/// **A CLOSED DAY THE OTHER FEED TRADED IS SAID AS EVIDENCE** (DB-A15): the
/// calendar's measured window is derived from Zerodha's own bars, so a
/// Zerodha day the calendar calls closed while GDFL holds bars is named with
/// GDFL's count. Before, the count was said with no word on the calendar.
#[test]
fn a_closed_day_the_peer_traded_is_said_as_independent_evidence() {
    let root = root_of("r3-13-closed-peer");
    file_bars(
        &root,
        Vendor::Gdfl,
        "NSE-INDEX-NIFTY",
        Timeframe::SECOND_1,
        vec![bar("2024-10-02", 10 * 3_600, [1_000_000; 4], 0)],
    );
    let got = answer(
        &root,
        &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-10-02&to=2024-10-02"),
    );
    says_has(
        record(&got, "2024-10-02"),
        "GDFL holds 1 bar(s) on a day the calendar of record calls closed: evidence independent \
         of Zerodha's own series",
    );
}

/// **AN OPTION FAMILY NARROWED TO ONE EXPIRY OWES NOTHING AFTER IT**
/// (SC-CLOSE-12; R3-13 goal 1's "any strike" selection). `NSE-FNO-NIFTY`
/// with `expiry=2019-03-28` reads every day of an October 2024 window
/// expired; the route refuses an expiry on a spot key and one that
/// contradicts a contract's own, and reads a matching one.
#[test]
fn an_option_family_narrowed_to_an_expiry_is_expired_after_it() {
    let root = root_of("r3-13-family-expiry");
    let asked = ask("feed=gdfl&instrument=NSE-FNO-NIFTY&from=2024-10-01&to=2024-10-01");
    let band = band_of("", "2019-03-28", &asked.instrument).expect("a family expiry");
    assert_eq!(band.expiry, Some(day("2019-03-28")));
    let got = answer_at(&root, &asked, band);
    assert_eq!(got["asked"]["expiry"], "2019-03-28");
    assert_eq!(got["asked"]["timeframe"], Value::Null);
    let rec = record(&got, "2024-10-01");
    assert_eq!(rec["contract"]["expired"], true, "{rec:#}");
    assert_eq!(rec["calendar"]["owed_minutes"], 0);
    says_has(
        rec,
        "expired: the contract's last session was 2019-03-28, so this day owes nothing",
    );
    // Without it the family's day is a session that names no contract.
    let plain = answer(&root, &asked);
    assert_eq!(record(&plain, "2024-10-01")["contract"], Value::Null);

    let spot = ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-10-01");
    assert_eq!(
        band_of("", "2019-03-28", &spot.instrument),
        Err("expiry=2019-03-28 names an expiry, and NSE-INDEX-NIFTY has none".to_owned())
    );
    let contract = ask("feed=gdfl&instrument=NSE-FNO-NIFTY-2019-03-28-1150000-CE&from=2019-03-01");
    assert_eq!(
        band_of("", "2019-04-25", &contract.instrument),
        Err("expiry=2019-04-25 is not the expiry NSE-FNO-NIFTY-2019-03-28-1150000-CE names (2019-03-28)".to_owned())
    );
    assert!(band_of("", "2019-03-28", &contract.instrument).is_ok());
    assert_eq!(
        band_of("", "28-03-2019", &asked.instrument),
        Err("expiry=\"28-03-2019\" is not a day as YYYY-MM-DD".to_owned())
    );
    // ONE PARSE FOR EVERY CALLER: the route's query reads both parameters.
    let (parsed, band) = super::super::query_of(
        "feed=gdfl&instrument=NSE-FNO-NIFTY&from=2024-10-01&to=2024-10-01&timeframe=1min&expiry=2019-03-28",
    )
    .expect("a band query");
    assert_eq!(parsed.instrument, asked.instrument);
    assert_eq!(
        band,
        Band {
            rung: Some(Timeframe::MINUTE_1),
            expiry: Some(day("2019-03-28"))
        }
    );
    assert_eq!(
        super::super::query_of(
            "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-10-01&expiry=2019-03-28"
        )
        .map(|(_, band)| band),
        Err("expiry=2019-03-28 names an expiry, and NSE-INDEX-NIFTY has none".to_owned())
    );
    assert!(super::super::query_of("feed=gdfl&from=2024-10-01").is_err());
    assert!(band_of("7min", "", &asked.instrument).is_err());
    assert_eq!(
        band_of("1day", "", &asked.instrument),
        Ok(Band {
            rung: Some(Timeframe::DAY_1),
            expiry: None
        })
    );
}

// ══ The measured target ══════════════════════════════════════════════════════

/// p50, p99 and max of `samples`, microseconds, and n.
fn spread(mut samples: Vec<u128>) -> (u128, u128, u128, usize) {
    samples.sort_unstable();
    let n = samples.len();
    let at = |q: usize| {
        samples
            .get((n * q / 100).min(n.saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    };
    (at(50), at(99), samples.last().copied().unwrap_or(0), n)
}

/// An invented store for the timing: NIFTY on both feeds for the page's 92
/// days (375 bars a session at each feed's finest rung, and Zerodha's day
/// bars), one share and one contract, and `history` x 1,000 import-journal
/// and day-ledger lines about days before the page, which no page reads.
#[allow(
    clippy::too_many_lines,
    reason = "one invented store, every series of it in the order a page reads them"
)]
fn timing_store(root: &Path, history: u32) {
    let sessions: Vec<Day> = (day("2024-07-01").days_from_epoch()
        ..=day("2024-09-30").days_from_epoch())
        .filter_map(|n| Day::from_days(n).ok())
        .filter(|d| {
            matches!(
                pull::calendar::record_of(i64::from(d.days_from_epoch())).kind,
                pull::calendar::DayKind::Open(_)
            )
        })
        .collect();
    let minute = |d: Day, m: i64, second: i64, v: i64| store::format::Bar {
        ts_micros: super::super::ist_midnight_micros(d)
            + (9 * 3_600 + 15 * 60 + m * 60 + second) * 1_000_000,
        open: v,
        high: v + 5,
        low: v - 5,
        close: v,
        volume: 10,
        open_interest: store::format::OI_NULL,
    };
    let per = |second: i64| -> Vec<store::format::Bar> {
        sessions
            .iter()
            .flat_map(|d| (0..375).map(move |m| minute(*d, m, second, 1_000_000 + m)))
            .collect()
    };
    file_bars(
        root,
        Vendor::Gdfl,
        "NSE-INDEX-NIFTY",
        Timeframe::SECOND_1,
        per(59),
    );
    file_bars(
        root,
        Vendor::Zerodha,
        "NSE-INDEX-NIFTY",
        Timeframe::MINUTE_1,
        per(0),
    );
    file_bars(
        root,
        Vendor::Zerodha,
        "NSE-CASH-RELIANCE",
        Timeframe::MINUTE_1,
        per(0),
    );
    file_bars(
        root,
        Vendor::Gdfl,
        "NSE-CASH-RELIANCE",
        Timeframe::SECOND_1,
        per(7),
    );
    file_bars(
        root,
        Vendor::Gdfl,
        "NSE-FNO-NIFTY-2024-09-26-2500000-CE",
        Timeframe::SECOND_1,
        per(3),
    );
    let days = sessions
        .iter()
        .map(|d| bar(&d.to_string(), 0, [1_000_000; 4], 0))
        .collect::<Vec<_>>();
    file_bars(
        root,
        Vendor::Zerodha,
        "NSE-INDEX-NIFTY",
        Timeframe::DAY_1,
        days.clone(),
    );
    file_bars(
        root,
        Vendor::Zerodha,
        "NSE-CASH-RELIANCE",
        Timeframe::DAY_1,
        days,
    );
    let mut journal = String::new();
    let mut ledger = String::new();
    for d in &sessions {
        for kind in ["indices", "stocks", "options"] {
            let _ = writeln!(
                journal,
                "begin {kind} {d} *\ndone {kind} {d} * definition=4 files=2 seconds=3 failures=0 late=4 late_unresolved=0 max_back_s=2"
            );
        }
        ledger.push_str(&super::super::tests::index_flags(
            &d.to_string(),
            "NIFTY",
            &[],
        ));
    }
    // THE HISTORY NO PAGE READS: closes of earlier days under other filters.
    let first = day("2018-01-01").days_from_epoch();
    for i in 0..history.saturating_mul(1_000) {
        let d = Day::from_days(first + i % 2_000).expect("a day");
        let _ = writeln!(
            journal,
            "done stocks {d} S{i} definition=4 files=1 seconds=1 failures=0 late=0 late_unresolved=0 max_back_s=0"
        );
        ledger.push_str(&super::super::tests::index_flags(
            &d.to_string(),
            &format!("X{i}"),
            &[],
        ));
    }
    write(&pull::gdfl_import::journal_path(root), &journal);
    write(&super::super::quality_path(root), &ledger);
}

/// The pages timed: every feed and segment the band serves, 92 days each.
const TIMED: [&str; 5] = [
    "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-07-01&to=2024-09-30",
    "feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-07-01&to=2024-09-30",
    "feed=zerodha&instrument=NSE-CASH-RELIANCE&from=2024-07-01&to=2024-09-30",
    "feed=gdfl&instrument=NSE-CASH-RELIANCE&from=2024-07-01&to=2024-09-30",
    "feed=gdfl&instrument=NSE-FNO-NIFTY-2024-09-26-2500000-CE&from=2024-07-01&to=2024-09-30",
];

/// Times `n` answers of each page over `root`: microseconds each.
fn timed(root: &Path, n: usize) -> Vec<u128> {
    let mut out = Vec::with_capacity(n * TIMED.len());
    for query in TIMED {
        let asked = ask(query);
        for _ in 0..n {
            let started = std::time::Instant::now();
            let body = answer(root, &asked);
            out.push(started.elapsed().as_micros());
            assert_eq!(body["days"].as_array().map(Vec::len), Some(92));
        }
    }
    out
}

/// **THE MEASURED TARGET** (R3-13): `/dayflags.json`'s answer for 92 days,
/// p50, p99 and max at n = 200 a page, over every feed and segment the band
/// serves, on an invented store:
///
/// - warm (the process has read the store once) and cold (a fresh copy of the
///   store under a new path, so every per-path memo is empty; the page cache
///   is warm: a cold DEVICE is not measured here);
/// - with 1 client, and with 60 clients at once;
/// - with an import writing (journal lines and bars appended) while it reads;
/// - and at 1x, 10x and 100x history no page reads (1,000 to 100,000 journal
///   and day-ledger lines): the warm p50 at 100x is held within 3 times the
///   1x one, the ratio gate.
///
/// IGNORED in the ordinary run: it builds three stores and times 3,000+
/// answers. Run it with `cargo test -p api --locked --
/// --ignored the_day_band_is_timed` and read the table it prints.
#[test]
#[ignore = "the measured target: builds three stores and times 3,000+ answers; run with --ignored"]
fn the_day_band_is_timed_cold_warm_concurrent_and_growing() {
    let mut warm_p50 = Vec::new();
    for history in [1_u32, 10, 100] {
        let root = root_of(&format!("r3-13-timing-{history}"));
        timing_store(&root, history);
        // COLD: a fresh copy of the store per sample, so no memo holds it.
        let mut cold = Vec::new();
        for at in 0..20 {
            let copy = root_of(&format!("r3-13-timing-{history}-cold-{at}"));
            copy_tree(&root, &copy);
            let asked = ask(TIMED[1]);
            let started = std::time::Instant::now();
            let _ = answer(&copy, &asked);
            cold.push(started.elapsed().as_micros());
            // ITS OWN FIXTURE, RELEASED: twenty copies of a 100x store would
            // otherwise fill a small scratch disk.
            let _ = std::fs::remove_dir_all(&copy);
        }
        let _ = timed(&root, 1);
        for query in TIMED {
            let asked = ask(query);
            let one: Vec<u128> = (0..200)
                .map(|_| {
                    let started = std::time::Instant::now();
                    let _ = answer(&root, &asked);
                    started.elapsed().as_micros()
                })
                .collect();
            let page = spread(one);
            eprintln!(
                "R3-13 timing history={history}x warm page {query}: p50 {} us, p99 {} us, max {} us, n {}",
                page.0, page.1, page.2, page.3
            );
        }
        let warm = spread(timed(&root, 200));
        warm_p50.push(warm.0);
        eprintln!(
            "R3-13 timing history={history}x warm 1 client: p50 {} us, p99 {} us, max {} us, n {}",
            warm.0, warm.1, warm.2, warm.3
        );
        let cold = spread(cold);
        eprintln!(
            "R3-13 timing history={history}x cold (fresh path, zerodha NIFTY): p50 {} us, p99 {} us, max {} us, n {}",
            cold.0, cold.1, cold.2, cold.3
        );
        if history == 1 {
            // 60 CLIENTS AT ONCE, and AN IMPORT WRITING beside them.
            let shared = std::sync::Arc::new(root.clone());
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let writer = {
                let (root, stop) = (std::sync::Arc::clone(&shared), std::sync::Arc::clone(&stop));
                std::thread::spawn(move || {
                    let mut i = 0_u64;
                    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                        super::super::tests::append(
                            &pull::gdfl_import::journal_path(&root),
                            &format!(
                                "done stocks 2018-01-02 W{i} definition=4 files=1 seconds=1 failures=0 late=0 late_unresolved=0 max_back_s=0\n"
                            ),
                        );
                        i += 1;
                        std::thread::sleep(std::time::Duration::from_millis(2));
                    }
                    i
                })
            };
            let writing = spread(timed(&root, 200));
            eprintln!(
                "R3-13 timing history=1x warm 1 client, import writing: p50 {} us, p99 {} us, max {} us, n {}",
                writing.0, writing.1, writing.2, writing.3
            );
            let clients: Vec<_> = (0..60)
                .map(|_| {
                    let root = std::sync::Arc::clone(&shared);
                    std::thread::spawn(move || timed(&root, 4))
                })
                .collect();
            let all: Vec<u128> = clients
                .into_iter()
                .flat_map(|c| c.join().expect("a client"))
                .collect();
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            let lines = writer.join().expect("the writer");
            let sixty = spread(all);
            eprintln!(
                "R3-13 timing history=1x 60 clients, import writing ({lines} lines appended): p50 {} us, p99 {} us, max {} us, n {}",
                sixty.0, sixty.1, sixty.2, sixty.3
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }
    // THE RATIO, IN HUNDREDTHS: integer arithmetic, no float cast.
    let base = warm_p50.first().copied().unwrap_or(1).max(1);
    let ratio = |at: usize| warm_p50.get(at).copied().unwrap_or(0).saturating_mul(100) / base;
    eprintln!(
        "R3-13 timing growth: warm p50 10x/1x {}/100, 100x/1x {}/100",
        ratio(1),
        ratio(2)
    );
    assert!(
        ratio(2) <= 300,
        "the warm answer grew with history no page reads: {warm_p50:?}"
    );
}

/// Copies a directory tree (the timing's cold samples).
fn copy_tree(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).expect("a directory") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a type").is_dir() {
            std::fs::create_dir_all(&target).expect("mkdir");
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("a copy");
        }
    }
}

/// **A DAY'S SCAN IS KEPT ONCE, AND A DAY THAT GREW REPLACES ITS ENTRY**
/// (DB-B20's cost): the memo holds one entry per (month file, inode, day),
/// with the end row it was read up to, so a month being written adds no
/// entry per append, and a day read before its last minute landed is read
/// again with it.
#[test]
fn a_days_scan_is_kept_once_and_a_grown_day_replaces_it() {
    let root = root_of("r3-13-scan-memo");
    let key = "NSE-INDEX-NIFTY";
    let at = |m: i64| 9 * 3_600 + 15 * 60 + m * 60;
    let day_one: Vec<_> = (0..4)
        .map(|m| bar("2024-10-01", at(m), [500; 4], 0))
        .collect();
    let day_two: Vec<_> = (0..2)
        .map(|m| bar("2024-10-03", at(m), [600; 4], 0))
        .collect();
    file_bars(
        &root,
        Vendor::Zerodha,
        key,
        Timeframe::MINUTE_1,
        [day_one, day_two].concat(),
    );
    let open = || {
        let inst = super::super::instrument_of(key, "").expect("a key");
        match super::super::open_month(
            &root,
            Vendor::Zerodha,
            &inst,
            Timeframe::MINUTE_1,
            YearMonth::new(2024, 10).expect("a month"),
        ) {
            super::super::Opened::File(file) => file,
            _ => panic!("the month opens"),
        }
    };
    let file = open();
    let id = file_identity(&file).expect("the host says");
    // The entries of one day (by its first row), and the end row each holds.
    let kept_of = |id: (u64, u64), lo: u64| -> Vec<u64> {
        SCANS.get().map_or_else(Vec::new, |memo| {
            memo.lock()
                .unwrap()
                .iter()
                .filter(|(k, _)| (k.0, k.1, k.2) == (id.0, id.1, lo))
                .map(|(_, (end, _))| *end)
                .collect()
        })
    };
    let kept = |lo: u64| kept_of(id, lo);
    let closed = scan_day(&file, Some(id), (0, 4), |_| false)
        .expect("a scan")
        .expect("in bound");
    assert_eq!(closed.frozen_runs, 1);
    assert_eq!(kept(0), vec![4], "a day is kept, with its end row");
    let last = scan_day(&file, Some(id), (4, 6), |_| false)
        .expect("a scan")
        .expect("in bound");
    assert_eq!(last.bars, 2);
    assert_eq!(kept(4), vec![6], "the month's last day is kept too");
    // Past the bound, nothing is read and nothing is said.
    assert_eq!(
        scan_day(&file, Some(id), (0, MAX_SCAN_BARS + 1), |_| false),
        Ok(None)
    );
    // The day grows (the writer needs the month: the reader lets it go):
    // read again, it carries the new minute and REPLACES its one entry.
    drop(file);
    file_bars(
        &root,
        Vendor::Zerodha,
        key,
        Timeframe::MINUTE_1,
        vec![bar("2024-10-03", at(2), [600; 4], 0)],
    );
    let reopened = open();
    let now = file_identity(&reopened).expect("the host says");
    let grown = scan_day(&reopened, Some(now), (4, 7), |_| false)
        .expect("a scan")
        .expect("in bound");
    assert_eq!((grown.bars, grown.frozen_runs), (3, 1));
    assert_eq!(
        kept_of(now, 4),
        vec![7],
        "one entry for the day, never one per append"
    );
    // The kept day answers from the memo, unchanged; with no identity the
    // host would give, the day is read and nothing is kept.
    assert_eq!(
        scan_day(&reopened, Some(now), (0, 4), |_| false).expect("a scan"),
        Some(closed)
    );
    let before = SCANS.get().map_or(0, |memo| memo.lock().unwrap().len());
    assert_eq!(
        scan_day(&reopened, None, (1, 4), |_| false)
            .expect("a scan")
            .map(|s| s.bars),
        Some(3)
    );
    assert_eq!(
        SCANS.get().map_or(0, |memo| memo.lock().unwrap().len()),
        before
    );
}

/// The timing's pages, answered ten times each over the 1x store, and nothing
/// timed: a target for a profiler (`valgrind --tool=callgrind`). IGNORED.
#[test]
#[ignore = "a profiler's target: run with --ignored under valgrind"]
fn the_day_band_pages_for_a_profiler() {
    let root = root_of("r3-13-profile");
    timing_store(&root, 1);
    for query in TIMED {
        let asked = ask(query);
        for _ in 0..10 {
            let body = answer(&root, &asked);
            assert_eq!(body["days"].as_array().map(Vec::len), Some(92));
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

// ══ Reads that fail, and months that will not open ═══════════════════════════

/// The path of `key`'s `rung` month file on `feed`.
fn month_path(
    root: &Path,
    feed: Vendor,
    key: &str,
    rung: Timeframe,
    month: YearMonth,
) -> std::path::PathBuf {
    let inst = super::super::instrument_of(key, "").expect("a key");
    store::path::StorePath::new(store::path::PathParts {
        vendor: feed,
        exchange: inst.exchange.as_str(),
        segment: inst.segment.as_str(),
        symbol: &inst.symbol,
        contract: inst.contract,
        timeframe: rung,
        month,
        file: store::path::FileKind::Bars,
    })
    .expect("a legal path")
    .to_path_buf(root)
}

/// **A MONTH BEFORE THE PAGE THAT WILL NOT OPEN SETTLES NOTHING** (DB-031):
/// no previous close is claimed from it, so no day is a jump from one, and
/// the series' start still finds the month held, so no day of the page is
/// called the series' front. Both for the month just before the page and for
/// one found 22 months back by `stat`.
#[test]
fn a_month_before_that_will_not_open_settles_nothing() {
    let month = |y: u16, m: u8| YearMonth::new(y, m).expect("a month");
    for (name, before) in [("near", month(2017, 12)), ("far", month(2016, 3))] {
        let root = root_of(&format!("r3-13-refused-before-{name}"));
        let key = "NSE-INDEX-NIFTYGSCOMPSITE";
        file_bars(
            &root,
            Vendor::Zerodha,
            key,
            Timeframe::DAY_1,
            vec![bar("2018-01-01", 0, [250_000; 4], 0)],
        );
        write(
            &month_path(&root, Vendor::Zerodha, key, Timeframe::DAY_1, before),
            "not a bar file",
        );
        let inst = super::super::instrument_of(key, "").expect("a key");
        assert_eq!(
            super::super::bar_before(&root, &inst, day("2018-01-01")),
            (None, false),
            "{name}: nothing read, and the history is not called empty"
        );
        let got = answer(
            &root,
            &ask(&format!(
                "feed=zerodha&instrument={key}&from=2018-01-01&to=2018-01-01"
            )),
        );
        assert_eq!(
            got["zerodha_1day_start"]["state"], "held_before",
            "{name}: {got:#}"
        );
        assert_eq!(
            record(&got, "2018-01-01")["junk"],
            serde_json::json!([]),
            "{name}"
        );
    }
}

/// **NOTHING IS BEFORE THE FIRST MONTH A DAY CAN NAME**: a page in 1970-01
/// looks back nowhere and says no history is held before it.
#[test]
fn a_page_in_the_first_month_looks_back_nowhere() {
    let root = root_of("r3-13-epoch");
    let month = |y: u16, m: u8| YearMonth::new(y, m).expect("a month");
    assert_eq!(super::super::month_before(day("1970-01-31")), None);
    assert_eq!(
        super::super::month_before(day("1970-02-01")),
        Some((day("1970-01-31"), month(1970, 1)))
    );
    assert_eq!(
        super::super::month_before(day("2024-03-15")),
        Some((day("2024-02-29"), month(2024, 2)))
    );
    let inst = super::super::instrument_of("NSE-INDEX-NIFTY", "").expect("a key");
    assert_eq!(
        super::super::bar_before(&root, &inst, day("1970-01-02")),
        (None, false)
    );
    // The page itself: no month back to probe, none forward held, so the
    // series' start is none held and no day is a front.
    let got = answer(
        &root,
        &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=1970-01-01&to=1970-01-02"),
    );
    assert_eq!(got["zerodha_1day_start"]["state"], "none_held", "{got:#}");
}

/// **A READ THAT FAILS IS NAMED, NEVER COUNTED** (R3-13): a month cut short
/// under an open handle answers no zero-volume count, no scan and no first
/// bar, and each says why in the page's notes.
#[test]
fn a_day_whose_bars_will_not_read_is_named_and_not_counted() {
    let root = root_of("r3-13-cut-short");
    let key = "NSE-INDEX-NIFTY";
    let open = 9 * 3_600 + 15 * 60;
    file_bars(
        &root,
        Vendor::Zerodha,
        key,
        Timeframe::MINUTE_1,
        vec![
            bar("2022-03-07", open, [1_600_000; 4], 0),
            bar("2022-03-07", open + 60, [1_600_000; 4], 0),
        ],
    );
    let inst = super::super::instrument_of(key, "").expect("a key");
    let month = YearMonth::new(2022, 3).expect("a month");
    let super::super::Opened::File(file) =
        super::super::open_month(&root, Vendor::Zerodha, &inst, Timeframe::MINUTE_1, month)
    else {
        panic!("the month opens");
    };
    // Whole, and with no memo key, the day is read and scanned in place.
    let reads = super::super::Reads {
        fills: true,
        scan: true,
        first: true,
    };
    let mut whole = Vec::new();
    let (fills, scan, first) = super::super::day_reads(
        (&file, None),
        (Vendor::Zerodha, &inst, Timeframe::MINUTE_1),
        (day("2022-03-07"), 0, 2),
        reads,
        &mut whole,
    );
    assert_eq!(fills, Some(2), "two minutes with no volume");
    assert_eq!(scan.map(|scan| scan.bars), Some(2));
    assert_eq!(
        first,
        Some(super::super::ist_midnight_micros(day("2022-03-07")) + open * 1_000_000)
    );
    assert!(whole.is_empty(), "{whole:#?}");
    // A HANDLE OF ITS OWN, opened before the cut: the first one keeps the
    // block it read, so only a handle that has read nothing meets the cut.
    let super::super::Opened::File(file) =
        super::super::open_month(&root, Vendor::Zerodha, &inst, Timeframe::MINUTE_1, month)
    else {
        panic!("the month opens again");
    };
    let path = month_path(&root, Vendor::Zerodha, key, Timeframe::MINUTE_1, month);
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("the month file")
        .set_len(0)
        .expect("cut short");
    let mut notes = Vec::new();
    let got = super::super::day_reads(
        (&file, None),
        (Vendor::Zerodha, &inst, Timeframe::MINUTE_1),
        (day("2022-03-07"), 0, 2),
        reads,
        &mut notes,
    );
    assert_eq!(got, (None, None, None));
    assert_eq!(notes.len(), 3, "{notes:#?}");
    for (note, what) in notes.iter().zip([
        "zero-volume bars could not be counted",
        "the day could not be scanned",
        "the first bar could not be read",
    ]) {
        assert!(
            note.starts_with(&format!(
                "zerodha NSE-INDEX-NIFTY 1min 2022-03-07: {what}: "
            )),
            "{note}"
        );
    }
    // Nothing is read where nothing is asked, or where the day holds no bar.
    let mut quiet = Vec::new();
    let none = super::super::day_reads(
        (&file, None),
        (Vendor::Zerodha, &inst, Timeframe::MINUTE_1),
        (day("2022-03-07"), 2, 2),
        reads,
        &mut quiet,
    );
    assert_eq!(none, (Some(0), None, None));
    assert!(quiet.is_empty(), "{quiet:#?}");
}

/// **A LATE LEDGER THAT WILL NOT READ IS SAID** (DB-029): a recounted day's
/// late rows are then null, and the notes say why, never zero.
#[test]
fn a_late_ledger_that_will_not_read_is_said() {
    let rows = super::super::JournalRows {
        by_day: HashMap::new(),
        last_day: None,
        journal: Value::Null,
        runs: Vec::new(),
        closers: HashMap::new(),
        late: Err("imports/gdfl.journal: denied".to_owned()),
    };
    let asked = ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2021-05-21&to=2021-05-21");
    let notes = super::super::source_notes(&asked, Some(&Ok(rows)), None, &Err("x".to_owned()));
    assert!(
        notes.iter().any(|note| note
            == "The import journal's late-row counts could not be read, so a recounted day's late \
                rows are null, not zero: imports/gdfl.journal: denied"),
        "{notes:#?}"
    );
}

/// A Zerodha `1day` session with neither a day bar nor a minute that day,
/// though both months are held, is "no Zerodha 1day bar": never a day bar
/// missing beside minutes the day does not hold.
#[test]
fn a_day_bar_session_without_minutes_says_only_no_bar() {
    let root = root_of("r3-13-no-minutes");
    let key = "NSE-INDEX-NIFTY";
    file_bars(
        &root,
        Vendor::Zerodha,
        key,
        Timeframe::DAY_1,
        vec![bar("2024-10-01", 0, [2_500_000; 4], 0)],
    );
    file_bars(
        &root,
        Vendor::Zerodha,
        key,
        Timeframe::MINUTE_1,
        vec![bar("2024-10-01", 9 * 3_600 + 15 * 60, [2_500_000; 4], 0)],
    );
    let got = answer_at(
        &root,
        &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-10-03&to=2024-10-03"),
        Band {
            rung: Some(Timeframe::DAY_1),
            expiry: None,
        },
    );
    let rec = record(&got, "2024-10-03");
    assert_eq!(
        (&rec["held"]["bars"], &rec["finest"]["bars"]),
        (&Value::from(0), &Value::from(0)),
        "{rec:#}"
    );
    says_has(rec, "no Zerodha 1day bar on this session");
    assert!(
        says(rec)
            .iter()
            .all(|say| !say.contains("the 1day bar is missing")),
        "{:#?}",
        says(rec)
    );
}

/// **A PAGE'S FIRST SCANS RUN A MONTH A THREAD AND ARE KEPT** (R3-13's
/// measured target): every day that holds bars and that the memo does not
/// hold at its end row is cold, and is scanned once; a second count of the
/// page scans none; a day read to another end row is cold again; a day with
/// no bar is never cold. Each day's own scan then answers from the memo.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one two-month fixture, the page's parallel scan and the month's cold days, read together"
)]
fn a_pages_first_scans_run_a_month_a_thread_and_are_kept() {
    let root = root_of("r3-13-scan-months");
    let key = "NSE-INDEX-NIFTY";
    let open_at = 9 * 3_600 + 15 * 60;
    // Ten March days and four April days, two minutes each: two months.
    let held: Vec<Day> = [
        "03-02", "03-03", "03-04", "03-07", "03-08", "03-09", "03-10", "03-11", "03-14", "03-15",
        "04-01", "04-04", "04-05", "04-06",
    ]
    .iter()
    .map(|d| day(&format!("2022-{d}")))
    .collect();
    let bars = held
        .iter()
        .flat_map(|on| {
            let on = on.to_string();
            [
                bar(&on, open_at, [1_600_000; 4], 0),
                bar(&on, open_at + 60, [1_600_100; 4], 0),
            ]
        })
        .collect();
    file_bars(&root, Vendor::Zerodha, key, Timeframe::MINUTE_1, bars);
    let inst = super::super::instrument_of(key, "").expect("a key");
    let mut days: Vec<Day> = held.clone();
    days.push(day("2022-04-07"));
    days.sort();
    let scan = super::super::Reads {
        scan: true,
        ..super::super::Reads::default()
    };
    let venue = pull::vendor::Venue::NseIndex;
    let page = |reads| {
        super::super::open_months(
            &root,
            (Vendor::Zerodha, &inst, Timeframe::MINUTE_1),
            &days,
            reads,
        )
    };
    let months = page(scan);
    assert_eq!(months.len(), 2);
    assert_eq!(
        super::super::scan_months(&months, venue),
        14,
        "every held day, on two months' threads"
    );
    assert_eq!(
        super::super::scan_months(&page(scan), venue),
        0,
        "every day is kept"
    );
    // A page that scans nothing has no memo key, so nothing is cold.
    assert!(
        page(super::super::Reads::default())
            .iter()
            .all(|month| month.identity.is_none())
    );
    assert_eq!(
        super::super::scan_months(&page(super::super::Reads::default()), venue),
        0
    );
    // The whole count then reads each day's scan from the memo.
    let counted = super::super::counts(
        &root,
        Vendor::Zerodha,
        &inst,
        Timeframe::MINUTE_1,
        &days,
        scan,
    );
    assert_eq!(counted.scans.iter().flatten().count(), 14);

    // The month's cold days, directly: a day with no bar is never cold, a kept
    // day is not, and a day read to another end row is cold again.
    let month = YearMonth::new(2022, 4).expect("a month");
    let super::super::Opened::File(file) =
        super::super::open_month(&root, Vendor::Zerodha, &inst, Timeframe::MINUTE_1, month)
    else {
        panic!("the month opens");
    };
    let identity = file_identity(&file).expect("an identity");
    let april: Vec<Day> = days.iter().copied().filter(|d| d.month() == 4).collect();
    let spans: Vec<(Day, u64, u64)> = april
        .iter()
        .zip(super::super::day_spans(&file, &april))
        .map(|(on, span)| {
            let (lo, hi) = span.expect("a span");
            (*on, lo, hi)
        })
        .collect();
    assert_eq!(
        spans.last().map(|(_, lo, hi)| hi - lo),
        Some(0),
        "the 7th holds no bar"
    );
    assert!(cold_days(identity, &spans).is_empty(), "April is kept");
    let (on, lo, hi) = spans[0];
    let shorter = [(on, lo, hi - 1)];
    assert_eq!(cold_days(identity, &shorter), shorter.to_vec());
    let quiet = |_: Day, _: i64| false;
    assert_eq!(scan_days(&file, identity, &shorter, quiet), 1);
    assert!(cold_days(identity, &shorter).is_empty());
    assert_eq!(cold_days(identity, &spans[..1]), spans[..1].to_vec());
    assert_eq!(scan_days(&file, identity, &spans[..1], quiet), 1);
    for &(_, lo, hi) in &spans[..4] {
        let kept = scan_day(&file, Some(identity), (lo, hi), |_| false).expect("from the memo");
        assert_eq!(kept.map(|scan| scan.bars), Some(2));
    }
}

/// **A SCALE SWITCH AGAINST THE MONTH JUST BEFORE THE PAGE** (DB-031): the
/// previous close is that month's LAST bar, so a re-scale at the month's turn
/// is `impossible_jump` on the page's first day.
#[test]
fn a_scale_switch_is_found_against_the_month_just_before() {
    let root = root_of("r3-13-near-month");
    let key = "NSE-INDEX-NIFTYGSCOMPSITE";
    file_bars(
        &root,
        Vendor::Zerodha,
        key,
        Timeframe::DAY_1,
        vec![
            bar("2017-12-28", 0, [250_000; 4], 0),
            bar("2017-12-29", 0, [3_000_000; 4], 0),
            bar("2018-01-01", 0, [250_000; 4], 0),
        ],
    );
    let inst = super::super::instrument_of(key, "").expect("a key");
    let (before, none_before) = super::super::bar_before(&root, &inst, day("2018-01-01"));
    assert_eq!(
        before.map(|bar| bar.close),
        Some(3_000_000),
        "the month's last bar"
    );
    assert!(!none_before);
    let got = answer(
        &root,
        &ask(&format!(
            "feed=zerodha&instrument={key}&from=2018-01-01&to=2018-01-01"
        )),
    );
    assert_eq!(
        record(&got, "2018-01-01")["junk"],
        serde_json::json!(["impossible_jump"]),
        "{got:#}"
    );
}

/// **A MONTH FILE THAT HOLDS NO BAR IS STILL A HELD MONTH** for the series'
/// start, as it was before R3-13: `bar_before` finds no bar in it and does
/// not call the history empty, so the series' start probes back, finds the
/// file, and no day of the page is called the series' front.
#[test]
fn an_empty_month_before_the_page_is_not_an_empty_history() {
    let root = root_of("r3-13-empty-month");
    let key = "NSE-INDEX-NIFTYGSCOMPSITE";
    file_bars(
        &root,
        Vendor::Zerodha,
        key,
        Timeframe::DAY_1,
        vec![bar("2018-01-01", 0, [250_000; 4], 0)],
    );
    let inst = super::super::instrument_of(key, "").expect("a key");
    let month = YearMonth::new(2017, 12).expect("a month");
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Zerodha,
        exchange: inst.exchange.as_str(),
        segment: inst.segment.as_str(),
        symbol: &inst.symbol,
        contract: inst.contract,
        timeframe: Timeframe::DAY_1,
        month,
        file: store::path::FileKind::Bars,
    })
    .expect("a legal path");
    let empty =
        store::file::BarFile::open_or_create(&root, path, super::super::symbol_id(&inst.symbol))
            .expect("an empty month file");
    assert_eq!(empty.records(), 0);
    drop(empty);
    assert_eq!(
        super::super::bar_before(&root, &inst, day("2018-01-01")),
        (None, false)
    );
    let got = answer(
        &root,
        &ask(&format!(
            "feed=zerodha&instrument={key}&from=2018-01-01&to=2018-01-01"
        )),
    );
    assert_eq!(got["zerodha_1day_start"]["state"], "held_before", "{got:#}");
}

/// **A DAY'S BAR PHRASES ARE SAID WHERE THEY APPLY, AND ONLY THERE**: an own
/// month that will not read is said so; a month with no file is said only on
/// a session; the peer's count is said on a session even when it is zero, and
/// on a day outside the calendar where it holds bars.
#[test]
fn a_days_bar_phrases_are_said_only_where_they_apply() {
    let root = root_of("r3-13-bar-phrases");
    let key = "NSE-INDEX-NIFTY";
    let month = |y: u16, m: u8| YearMonth::new(y, m).expect("a month");
    // Zerodha's own October 2024 will not read; GDFL holds October with a bar
    // on the 1st only, and a bar far past the calendar.
    write(
        &month_path(
            &root,
            Vendor::Zerodha,
            key,
            Timeframe::MINUTE_1,
            month(2024, 10),
        ),
        "not a bar file",
    );
    file_bars(
        &root,
        Vendor::Gdfl,
        key,
        Timeframe::SECOND_1,
        vec![
            bar("2024-10-01", 10 * 3_600, [2_500_000; 4], 0),
            bar("2099-01-05", 10 * 3_600, [2_500_000; 4], 0),
        ],
    );
    let got = answer(
        &root,
        &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-10-03&to=2024-10-03"),
    );
    let rec = record(&got, "2024-10-03");
    says_has(rec, "Zerodha's month could not be read");
    says_has(rec, "GDFL holds 0 bar(s)");
    // November: no file at all. A session says so; a Saturday says nothing.
    let got = answer(
        &root,
        &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-11-08&to=2024-11-09"),
    );
    says_has(
        record(&got, "2024-11-08"),
        "Zerodha holds no file for this month",
    );
    assert!(
        says(record(&got, "2024-11-09"))
            .iter()
            .all(|say| !say.contains("holds no file")),
        "{:#?}",
        says(record(&got, "2024-11-09"))
    );
    // A day outside the calendar: the peer's bars are said.
    let got = answer(
        &root,
        &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2099-01-05&to=2099-01-05"),
    );
    let far = record(&got, "2099-01-05");
    assert_eq!(far["calendar"]["code"], "-", "{far:#}");
    says_has(far, "GDFL holds 1 bar(s)");
}

/// **THE SCAN'S EDGES ARE WHERE THEY ARE NAMED** (DB-B20): a day's extreme is
/// its FIRST minute at that value; a minute at the day bar's own high or low
/// is inside the day's range; a day of exactly 1,440 minutes is scanned and a
/// span of one more is not; points read zero unsigned.
#[test]
fn the_scans_edges_are_where_they_are_named() {
    let on = "2022-03-07";
    let at = |minute: i64| 9 * 3_600 + 15 * 60 + minute * 60;
    let ts = |minute: i64| bar(on, at(minute), [1; 4], 0).ts_micros;
    // Highs 10, 12, 12 and lows 9, 7, 7: each extreme is its first minute.
    let scan = fold_minutes(
        [
            bar(on, at(0), [10, 10, 9, 9], 0),
            bar(on, at(1), [12, 12, 7, 7], 0),
            bar(on, at(2), [12, 12, 7, 7], 0),
        ]
        .into_iter(),
        |_| false,
    );
    assert_eq!(scan.high, Some((12, ts(1))));
    assert_eq!(scan.low, Some((7, ts(1))));
    // A lower low after a higher one replaces it; an equal one does not.
    let falling = fold_minutes(
        [
            bar(on, at(0), [10, 10, 10, 10], 0),
            bar(on, at(1), [5, 5, 5, 5], 0),
        ]
        .into_iter(),
        |_| false,
    );
    assert_eq!(falling.low, Some((5, ts(1))));
    // At the day bar's own high and low: inside the range, nothing flagged.
    let edge = IndexDay {
        feed: Vendor::Zerodha,
        midnight: 0,
        scan: Some(scan),
        day_bar: Some((12, 7)),
        ledger: None,
        halt: None,
    };
    let (json, says) = edge.json();
    assert_eq!(json["outside_day_range"]["flagged"], false, "{json:#}");
    assert!(says.is_empty(), "{says:?}");
    // Points: zero has no sign.
    assert_eq!(points(0), "0.00");
    assert_eq!(points(-5), "-0.05");
    assert_eq!(points(22_985), "229.85");

    // A day of exactly MAX_SCAN_BARS minutes is scanned; a longer span is not read.
    let root = root_of("r3-13-scan-bound");
    let full: Vec<store::format::Bar> = (0..i64::try_from(MAX_SCAN_BARS).expect("a count"))
        .map(|minute| bar(on, minute * 60, [1_600_000; 4], 0))
        .collect();
    file_bars(
        &root,
        Vendor::Zerodha,
        "NSE-INDEX-NIFTY",
        Timeframe::MINUTE_1,
        full,
    );
    let inst = super::super::instrument_of("NSE-INDEX-NIFTY", "").expect("a key");
    let month = YearMonth::new(2022, 3).expect("a month");
    let super::super::Opened::File(file) =
        super::super::open_month(&root, Vendor::Zerodha, &inst, Timeframe::MINUTE_1, month)
    else {
        panic!("the month opens");
    };
    let whole = scan_day(&file, None, (0, MAX_SCAN_BARS), |_| false).expect("the day reads");
    assert_eq!(whole.map(|scan| scan.bars), Some(MAX_SCAN_BARS));
    assert_eq!(
        scan_day(&file, None, (0, MAX_SCAN_BARS + 1), |_| false),
        Ok(None)
    );
}

/// **A MONTH FILE'S IDENTITY IS ITS OWN DEVICE AND INODE**: two months are two
/// keys, so one month's kept scan is never another's.
#[test]
fn a_month_files_identity_is_its_own_device_and_inode() {
    use std::os::unix::fs::MetadataExt as _;
    let root = root_of("r3-13-identity");
    let on = "2022-03-07";
    let open = 9 * 3_600 + 15 * 60;
    let month = YearMonth::new(2022, 3).expect("a month");
    // NIFTY frozen for three minutes, BANKNIFTY moving: the same rows [0, 3).
    file_bars(
        &root,
        Vendor::Zerodha,
        "NSE-INDEX-NIFTY",
        Timeframe::MINUTE_1,
        (0..3)
            .map(|m| bar(on, open + m * 60, [1_600_000; 4], 0))
            .collect(),
    );
    file_bars(
        &root,
        Vendor::Zerodha,
        "NSE-INDEX-BANKNIFTY",
        Timeframe::MINUTE_1,
        (0..3)
            .map(|m| bar(on, open + m * 60, [3_400_000 + m; 4], 0))
            .collect(),
    );
    let opened = |key: &str| {
        let inst = super::super::instrument_of(key, "").expect("a key");
        let super::super::Opened::File(file) =
            super::super::open_month(&root, Vendor::Zerodha, &inst, Timeframe::MINUTE_1, month)
        else {
            panic!("the month opens");
        };
        file
    };
    let (nifty, bank) = (opened("NSE-INDEX-NIFTY"), opened("NSE-INDEX-BANKNIFTY"));
    for (file, key) in [(&nifty, "NSE-INDEX-NIFTY"), (&bank, "NSE-INDEX-BANKNIFTY")] {
        let meta = std::fs::metadata(month_path(
            &root,
            Vendor::Zerodha,
            key,
            Timeframe::MINUTE_1,
            month,
        ))
        .expect("the month file");
        assert_eq!(file_identity(file), Some((meta.dev(), meta.ino())), "{key}");
    }
    let frozen = scan_day(&nifty, file_identity(&nifty), (0, 3), |_| false).expect("reads");
    let moving = scan_day(&bank, file_identity(&bank), (0, 3), |_| false).expect("reads");
    assert_eq!(frozen.map(|scan| scan.frozen_runs), Some(1));
    assert_eq!(
        moving.map(|scan| scan.frozen_runs),
        Some(0),
        "never the other month's scan"
    );
}

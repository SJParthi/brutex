//! Rows of `recovery.rs` no test reached before BASE-COV-API repair 3 (shard
//! AS1, D-8661): the lifecycle qualification of an assessment, and a retry
//! day that reaches the source request on a site that may not reach a broker.
//! Every master here is invented: the dates, the native ids and the flags
//! name no vendor row; the symbols and their ISINs are the workspace's own
//! universe table, not a vendor's.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a whole-file test module; fixtures are bounded and a failed assertion is the test failing"
)]

use super::*;
use std::fmt::Write as _;

/// One master row: the symbol and its listing, removal and readmission days
/// (`None` is the master's `0`).
type MasterRow<'a> = (&'a str, Option<Day>, Option<Day>, Option<Day>);

/// The day every master here is dated: past the closing auction's start
/// (2026-08-03), so the cache installs it.
fn master_day() -> Day {
    Day::new(2026, 8, 27).expect("a day")
}

fn on(year: u16, month: u8, day: u8) -> Day {
    Day::new(year, month, day).expect("a day")
}

/// Seconds from 1980-01-01 to `day`, as a master's lifecycle columns hold
/// them.
fn since_1980(day: Day) -> String {
    let epoch = on(1980, 1, 1);
    (i64::from(day.days_from_epoch() - epoch.days_from_epoch()) * 86_400).to_string()
}

/// `text` as one stored gzip member, the shape the cache installs.
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

/// A master of `rows`, installed and read back as the recovery reads it.
fn lifecycle_of(
    name: &str,
    rows: &[MasterRow<'_>],
) -> pull::cash_session_cache::VerifiedLifecycleMaster {
    let root = crate::scratch::path(name);
    let _ = std::fs::remove_dir_all(&root);
    let mut csv = String::from(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn,ListgDt,RmvlDt,RadmssnDt\n",
    );
    let cell = |day: Option<Day>| day.map_or_else(|| "0".to_owned(), since_1980);
    for (at, (symbol, listing, removal, readmission)) in rows.iter().enumerate() {
        let isin = brutex_core::universe::nse_isin(symbol).expect("an F&O share");
        writeln!(
            csv,
            "{},{symbol},EQ,{},1,{},{},{}",
            900 + at,
            isin.as_str(),
            cell(*listing),
            cell(*removal),
            cell(*readmission)
        )
        .expect("a row");
    }
    let masters = root.join("session-masters");
    pull::cash_session_cache::install(&masters, master_day(), &stored_gzip(csv.as_bytes()))
        .expect("the master lands");
    let read = pull::cash_session_cache::read_local_lifecycle(&masters, master_day())
        .expect("the master reads back");
    let _ = std::fs::remove_dir_all(&root);
    read
}

/// What [`qualify_lifecycle`] leaves of `found`: the evidence issues and the
/// retry days.
fn qualified(
    mut found: Assessment,
    lifecycle: &Lifecycle,
    symbol: &str,
    window: Window,
    daily: &[Day],
    minutes: &[Day],
) -> (u64, Vec<Day>) {
    let daily: HashSet<Day> = daily.iter().copied().collect();
    let minutes: HashSet<Day> = minutes.iter().copied().collect();
    qualify_lifecycle(&mut found, lifecycle, symbol, window, &daily, &minutes);
    (found.evidence_issues, found.retry_days)
}

/// An assessment holding `held` days and missing `missing`, owing `retry`.
fn assessment(held: u64, missing: u64, retry: &[Day]) -> Assessment {
    Assessment {
        held,
        missing,
        retry_days: retry.to_vec(),
        ..Assessment::default()
    }
}

/// **A LIFECYCLE QUALIFIES AN ASSESSMENT BY ITS OWN EVIDENCE, NEVER BY A
/// GUESS** (BASE-COV-API repair 3, D-8661). `qualify_lifecycle` ran under no
/// test: it is reached only through a whole source audit. Driven here
/// directly, over a master installed and read back as the recovery reads it.
/// Extremes: an index, which no lifecycle qualifies; no master at all with a
/// day held (an issue, every retry dropped); no master and nothing held or
/// missing (nothing said); a share the master does not hold, with a day
/// missing; a listing inside the window, a day held before it (an issue, and
/// the retries before the listing dropped); a listing before the window
/// (nothing said, every retry kept); a listing after the master's own day
/// (unverified: an issue, every retry dropped); a removal before the listing
/// (contradictory, the same); a master row of no date with a day held (an
/// issue) and with nothing held (nothing said); a sourced inactive day held
/// inside the window, and one held outside it; and, at once, an unverified
/// listing with a day held before it (two issues), and a sourced inactive day
/// held with no master at all (two issues, every retry dropped); out of the
/// box, one inactive day held in the daily and the minute sets both, read
/// once through their union.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one master, every branch of the qualification beside the others it is read against"
)]
fn a_lifecycle_qualifies_an_assessment_by_its_own_evidence() {
    let window = Window::new(on(2026, 8, 24), on(2026, 8, 27)).expect("a window");
    let master = Some(lifecycle_of(
        "recovery-cover-lifecycle",
        &[
            ("RELIANCE", Some(on(2026, 8, 26)), None, None),
            ("INFY", Some(on(2001, 1, 2)), None, None),
            ("TCS", Some(on(2026, 9, 1)), None, None),
            ("HDFCBANK", Some(on(2010, 5, 3)), Some(on(2009, 4, 1)), None),
            ("SBIN", None, None, None),
        ],
    ));
    let (mon, tue, wed) = (on(2026, 8, 24), on(2026, 8, 25), on(2026, 8, 26));

    // AN INDEX: nothing about it is qualified, whatever is held.
    assert_eq!(
        qualified(
            assessment(3, 1, &[tue]),
            &None,
            "NIFTY",
            window,
            &[mon],
            &[]
        ),
        (0, vec![tue])
    );
    // NO MASTER: a held or missing day is an issue and no retry is guessed.
    assert_eq!(
        qualified(assessment(1, 0, &[tue]), &None, "ABB", window, &[mon], &[]),
        (1, vec![])
    );
    assert_eq!(
        qualified(assessment(0, 0, &[]), &None, "ABB", window, &[], &[]),
        (0, vec![]),
        "nothing held and nothing missing says nothing"
    );
    // A SHARE THE MASTER DOES NOT HOLD is no master for it.
    assert_eq!(
        qualified(assessment(0, 1, &[tue]), &master, "ABB", window, &[], &[]),
        (1, vec![])
    );
    // A LISTING INSIDE THE WINDOW: the day held before it is an issue, and a
    // retry before it is dropped; the one on it is kept.
    assert_eq!(
        qualified(
            assessment(1, 2, &[tue, wed]),
            &master,
            "RELIANCE",
            window,
            &[mon],
            &[]
        ),
        (1, vec![wed])
    );
    // A LISTING LONG BEFORE THE WINDOW: nothing to say, every retry kept.
    assert_eq!(
        qualified(
            assessment(2, 1, &[tue]),
            &master,
            "INFY",
            window,
            &[mon, wed],
            &[]
        ),
        (0, vec![tue])
    );
    // A LISTING AFTER THE MASTER'S OWN DAY is unverified: an issue, no retry;
    // and a held day before that listing is one more, at once.
    assert_eq!(
        qualified(assessment(0, 1, &[tue]), &master, "TCS", window, &[], &[]),
        (1, vec![])
    );
    assert_eq!(
        qualified(
            assessment(1, 1, &[tue]),
            &master,
            "TCS",
            window,
            &[mon],
            &[]
        ),
        (2, vec![])
    );
    // A REMOVAL BEFORE THE LISTING is contradictory: an issue, no retry.
    assert_eq!(
        qualified(
            assessment(1, 1, &[tue]),
            &master,
            "HDFCBANK",
            window,
            &[mon],
            &[]
        ),
        (1, vec![])
    );
    // NO DATE AT ALL: an issue when something is held or missing, and none
    // when nothing is; its retries stand.
    assert_eq!(
        qualified(
            assessment(1, 0, &[tue]),
            &master,
            "SBIN",
            window,
            &[mon],
            &[]
        ),
        (1, vec![tue])
    );
    assert_eq!(
        qualified(assessment(0, 0, &[tue]), &master, "SBIN", window, &[], &[]),
        (0, vec![tue])
    );

    // A SOURCED INACTIVE DAY (FORCEMOT, withdrawn 2023-10-26 to 2024-02-13):
    // held inside the window it is an issue; held outside it, none of this
    // window's. FORCEMOT has no row here, so a held day is one issue more.
    let inactive = Window::new(on(2024, 1, 2), on(2024, 1, 4)).expect("a window");
    let (jan_3, feb_20) = (on(2024, 1, 3), on(2024, 2, 20));
    assert_eq!(
        qualified(
            assessment(0, 0, &[]),
            &master,
            "FORCEMOT",
            inactive,
            &[jan_3],
            &[]
        ),
        (1, vec![])
    );
    assert_eq!(
        qualified(
            assessment(0, 0, &[]),
            &master,
            "FORCEMOT",
            inactive,
            &[feb_20],
            &[]
        ),
        (0, vec![]),
        "a day outside the window is not this window's"
    );
    assert_eq!(
        qualified(
            assessment(1, 0, &[jan_3]),
            &None,
            "FORCEMOT",
            inactive,
            &[jan_3],
            &[]
        ),
        (2, vec![]),
        "an inactive day held and no master, at once"
    );
    // OUT OF THE BOX: the same inactive day in both sets is one day.
    assert_eq!(
        qualified(
            assessment(0, 0, &[]),
            &master,
            "FORCEMOT",
            inactive,
            &[jan_3],
            &[jan_3]
        ),
        (1, vec![])
    );
}

/// A site that may not reach a broker, over an empty store of its own, with
/// the recovery's folder made.
fn refused_site(name: &str) -> (PathBuf, Loaded) {
    let root = crate::scratch::path(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch store");
    let site = Loaded::new(Site::load(&root.join("missing-masters"), &root));
    std::fs::create_dir_all(super::root(&site)).expect("the recovery folder");
    (root, site)
}

/// **A RETRY DAY SPENDS ITS BUDGET ONLY ON A REQUEST IT SENDS, AND STOPS AT
/// EVERY REFUSAL BY NAME** (BASE-COV-API repair 3, D-8661). On a site that
/// may not reach a broker the request is sent and blocked: one attempt is
/// charged in the shared ledger and the plan, the day is `Blocked` with the
/// answer's 503, and the refusal names the block. Extremes: a free seat, the
/// request blocked; the same day again, now blocked, appended as it stands
/// with no attempt; a day the shared ledger already holds exhausted; a day at
/// its attempt limit still queued, exhausted without a request; a run being
/// stopped, refused before the request with nothing charged; a plan at its
/// 100,000-unit bound refusing a new day by name; the same plan with the day
/// already in it, not refused by the bound; a day the ledger holds verified,
/// owed again and sent; and, at once, a run being stopped with the day at its
/// limit (exhausted: the limit is read before the stop), and a plan at its
/// bound with the day in it and the seat held (the seat's refusal, nothing
/// charged); out of the box, a parent that is no recovery request at all.
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one site, every arm of the retry beside the others it is read against"
)]
async fn a_retry_day_spends_its_budget_only_on_a_request_it_sends() {
    let (root, site) = refused_site("recovery-cover-retry");
    let day = on(2026, 8, 27);
    let parent = canonical("NIFTY", "1day", Window::new(day, day).unwrap(), "scan");
    let key = record(canonical(
        "NIFTY",
        "1day",
        Window::new(day, day).unwrap(),
        "gap",
    ))
    .key;
    let mut plan = Journal::open(&root.join("plan.bin")).unwrap();
    let mut attempts = Journal::open(&super::root(&site).join("attempts.bin")).unwrap();

    // A FREE SEAT: the request goes out and is blocked.
    let why = retry_day(&site, &mut plan, &mut attempts, &parent, day, &None)
        .await
        .expect_err("a blocked request is refused");
    assert!(why.contains("blocked"), "{why}");
    let charged = attempts.latest.get(&key).expect("charged in the ledger");
    assert_eq!(
        (charged.attempts, charged.status, charged.http_status),
        (1, Status::Blocked, 503)
    );
    assert_eq!(plan.latest[&key].status, Status::Blocked);
    let feed = checked(&parent, today_or_now()).unwrap().feed;
    assert!(site.autopilot.take_seat(feed).is_some(), "the seat is back");

    // THE SAME DAY AGAIN: blocked, appended as it stands, no attempt.
    retry_day(&site, &mut plan, &mut attempts, &parent, day, &None)
        .await
        .expect("a blocked day is appended and answered");
    assert_eq!(attempts.latest[&key].attempts, 1, "no attempt was charged");

    // A DAY THE LEDGER HOLDS EXHAUSTED, and one at its limit still queued.
    for (status, attempts_made) in [(Status::Exhausted, 2), (Status::Queued, ATTEMPT_LIMIT)] {
        let mut held = attempts.latest[&key].clone();
        held.status = status;
        held.attempts = attempts_made;
        attempts.append(held).unwrap();
        retry_day(&site, &mut plan, &mut attempts, &parent, day, &None)
            .await
            .expect("answered without a request");
        let now = &attempts.latest[&key];
        assert_eq!(
            (now.status, now.attempts),
            (Status::Exhausted, attempts_made)
        );
    }

    // A RUN BEING STOPPED: refused before the request, nothing charged; and
    // with the day at its limit at once, exhausted first.
    *site.run.lock().unwrap() = Some(crate::pullrun::Progress {
        stopping: true,
        ..crate::pullrun::Progress::default()
    });
    let fresh = on(2026, 8, 26);
    let fresh_parent = canonical("NIFTY", "1day", Window::new(fresh, fresh).unwrap(), "scan");
    let fresh_key = record(canonical(
        "NIFTY",
        "1day",
        Window::new(fresh, fresh).unwrap(),
        "gap",
    ))
    .key;
    let why = retry_day(&site, &mut plan, &mut attempts, &fresh_parent, fresh, &None)
        .await
        .expect_err("a stop refuses");
    assert!(
        why.contains("stopped before the next source request"),
        "{why}"
    );
    assert!(!attempts.latest.contains_key(&fresh_key), "nothing charged");
    let mut at_limit = attempts.latest[&key].clone();
    at_limit.status = Status::Queued;
    attempts.append(at_limit).unwrap();
    retry_day(&site, &mut plan, &mut attempts, &parent, day, &None)
        .await
        .expect("the limit is read before the stop");
    assert_eq!(attempts.latest[&key].status, Status::Exhausted);
    *site.run.lock().unwrap() = None;

    // A DAY THE LEDGER HOLDS VERIFIED is owed again and sent.
    let mut verified = record(canonical(
        "NIFTY",
        "1day",
        Window::new(fresh, fresh).unwrap(),
        "gap",
    ));
    verified.status = Status::Verified;
    verified.attempts = 1;
    attempts.append(verified).unwrap();
    let why = retry_day(&site, &mut plan, &mut attempts, &fresh_parent, fresh, &None)
        .await
        .expect_err("sent and blocked");
    assert!(why.contains("blocked"), "{why}");
    assert_eq!(
        attempts.latest[&fresh_key].attempts, 2,
        "its second attempt"
    );

    // A PLAN AT ITS BOUND refuses a new day by name and appends nothing; a
    // day already in it is not refused by the bound, and with the seat held
    // at once it is the seat's refusal, nothing charged.
    for n in 0..MAX_UNITS {
        let filler = record(format!("filler {n}"));
        plan.latest.insert(filler.key, filler);
    }
    let other = on(2026, 8, 25);
    let other_parent = canonical("NIFTY", "1day", Window::new(other, other).unwrap(), "scan");
    let why = retry_day(&site, &mut plan, &mut attempts, &other_parent, other, &None)
        .await
        .expect_err("the bound refuses a new day");
    assert!(why.contains("100,000-unit state bound"), "{why}");
    let mut owed = attempts.latest[&fresh_key].clone();
    owed.status = Status::Queued;
    attempts.append(owed).unwrap();
    let held = site.autopilot.take_seat(feed).expect("the seat is free");
    let why = retry_day(&site, &mut plan, &mut attempts, &fresh_parent, fresh, &None)
        .await
        .expect_err("the seat refuses");
    drop(held);
    assert!(why.contains("no attempt was reserved"), "{why}");
    assert_eq!(attempts.latest[&fresh_key].attempts, 2, "nothing charged");

    // OUT OF THE BOX: a parent that is no recovery request.
    let why = retry_day(&site, &mut plan, &mut attempts, "not a request", day, &None)
        .await
        .expect_err("refused");
    assert!(!why.is_empty());
    drop((plan, attempts));
    let _ = std::fs::remove_dir_all(root);
}

/// Today in IST, as the recovery checks a request against it.
fn today_or_now() -> Day {
    ingest::today_ist().expect("today")
}

/// IST `day` at `minute` past IST midnight and `second` past that minute, in
/// UTC epoch microseconds.
fn at(day: Day, minute: i64, second: i64) -> i64 {
    (i64::from(day.days_from_epoch()) * 86_400 + minute * 60 + second
        - pull::session::IST_OFFSET_SECS)
        * 1_000_000
}

/// Writes `stamps` as `symbol`'s Zerodha month at rung `dir`, where the
/// recovery reads its source; invented prices.
fn source_month(
    root: &Path,
    symbol: &str,
    dir: &str,
    month: store::path::YearMonth,
    stamps: &[i64],
) {
    let timeframe = ingest::parse_granularity(dir)
        .and_then(Granularity::store_timeframe)
        .expect("a rung");
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: if is_index(symbol) { "INDEX" } else { "CASH" },
        symbol,
        contract: None,
        timeframe,
        month,
        file: store::path::FileKind::Bars,
    })
    .expect("a path");
    let id = u32::try_from(brutex_core::universe::fnv1a(symbol) & 0xFFFF_FFFF).expect("a u32");
    let mut file = store::file::BarFile::open_or_create(root, path, id).expect("a month");
    let bars: Vec<store::format::Bar> = stamps
        .iter()
        .map(|&ts_micros| store::format::Bar {
            ts_micros,
            open: 1_234_500,
            high: 1_235_000,
            low: 1_234_000,
            close: 1_234_700,
            volume: 1,
            open_interest: store::format::OI_NULL,
        })
        .collect();
    file.append(&bars).expect("the bars append");
}

/// A store root of its own, emptied.
fn store_root(name: &str) -> PathBuf {
    let root = crate::scratch::path(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch store");
    root
}

/// **A SOURCE READ TAKES THE MINUTES ONLY WHEN THE DAYS DO NOT ANSWER, AND
/// REFUSES WHAT IT CANNOT COUNT** (BASE-COV-API repair 3, D-8661):
/// `read_sources` and `read_stamps` over Zerodha months written as the store
/// writes them. Extremes: no month at all, read as nothing; a daily month
/// holding every open day, asked daily, so the minutes are not read; one open
/// day missing, so they are; asked at the minute rung, so they are whatever
/// the days hold; the missing day a Saturday, closed, so they are not; two
/// daily bars on one IST date, refused by name; a daily bar off the minute,
/// refused by name; a daily month of 50,001 bars, past the read bound; a
/// share's month under `CASH` and an index's under `INDEX`; and, at once, a
/// share missing a day with no minute month (read as none), and the minute
/// rung with no minute month; out of the box, a minute month that is a folder.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one store, every arm of the source read beside the others it is read against"
)]
fn a_source_read_takes_the_minutes_only_when_the_days_do_not_answer() {
    let august = store::path::YearMonth::new(2026, 8).expect("a month");
    let (mon, tue, wed, sat) = (
        on(2026, 8, 24),
        on(2026, 8, 25),
        on(2026, 8, 26),
        on(2026, 8, 29),
    );
    let week = Window::new(mon, wed).expect("a window");
    let root = store_root("recovery-cover-sources");

    // NO MONTH AT ALL.
    assert_eq!(
        read_sources(&root, "NIFTY", august, week, true),
        Ok((Vec::new(), Vec::new()))
    );

    // EVERY OPEN DAY HELD, ASKED DAILY: the minutes stay unread.
    let daily: Vec<i64> = [mon, tue, wed].map(|day| at(day, 555, 0)).to_vec();
    let minutes: Vec<i64> = (555..560).map(|minute| at(tue, minute, 0)).collect();
    source_month(&root, "NIFTY", "1day", august, &daily);
    source_month(&root, "NIFTY", "1min", august, &minutes);
    assert_eq!(
        read_sources(&root, "NIFTY", august, week, true),
        Ok((daily.clone(), Vec::new()))
    );
    // ASKED AT THE MINUTE RUNG: the minutes are read.
    assert_eq!(
        read_sources(&root, "NIFTY", august, week, false),
        Ok((daily.clone(), minutes.clone()))
    );
    // ONE OPEN DAY MISSING: a wider window, Thursday not held.
    let longer = Window::new(mon, on(2026, 8, 27)).expect("a window");
    assert_eq!(
        read_sources(&root, "NIFTY", august, longer, true),
        Ok((daily.clone(), minutes.clone()))
    );
    // THE MISSING DAY A SATURDAY: closed, so the days answer.
    let weekend = Window::new(on(2026, 8, 28), sat).expect("a window");
    let friday_root = store_root("recovery-cover-sources-friday");
    source_month(
        &friday_root,
        "NIFTY",
        "1day",
        august,
        &[at(on(2026, 8, 28), 555, 0)],
    );
    source_month(&friday_root, "NIFTY", "1min", august, &minutes);
    assert_eq!(
        read_sources(&friday_root, "NIFTY", august, weekend, true),
        Ok((vec![at(on(2026, 8, 28), 555, 0)], Vec::new()))
    );

    // A SHARE'S MONTH IS UNDER CASH: missing Wednesday with no minute month,
    // read as none (at once); asked at the minute rung, none either.
    let share = store_root("recovery-cover-sources-share");
    source_month(
        &share,
        "ABB",
        "1day",
        august,
        &[at(mon, 555, 0), at(tue, 555, 0)],
    );
    let read = read_sources(&share, "ABB", august, week, true).expect("a share reads");
    assert_eq!((read.0.len(), read.1.len()), (2, 0));
    let read = read_sources(&share, "ABB", august, week, false).expect("a share reads");
    assert_eq!((read.0.len(), read.1.len()), (2, 0));
    assert!(
        source_path(&share, "ABB", "1day", august)
            .expect("a path")
            .to_string_lossy()
            .contains("/CASH/"),
        "a share's source is under CASH"
    );

    // TWO DAILY BARS ON ONE IST DATE, AND ONE OFF THE MINUTE.
    for (name, stamps, why) in [
        (
            "recovery-cover-sources-twice",
            vec![at(mon, 555, 0), at(mon, 600, 0)],
            "multiple daily bars name the same date",
        ),
        (
            "recovery-cover-sources-second",
            vec![at(mon, 555, 30)],
            "off-minute source timestamp",
        ),
    ] {
        let root = store_root(name);
        source_month(&root, "NIFTY", "1day", august, &stamps);
        assert_eq!(
            read_sources(&root, "NIFTY", august, week, true),
            Err(why.to_owned())
        );
    }

    // PAST THE READ BOUND: 50,001 daily bars, one a second.
    let crowded = store_root("recovery-cover-sources-bound");
    let first = at(on(2026, 8, 3), 0, 0);
    let stamps: Vec<i64> = (0..50_001).map(|n| first + n * 1_000_000).collect();
    source_month(&crowded, "NIFTY", "1day", august, &stamps);
    assert_eq!(
        read_sources(&crowded, "NIFTY", august, week, true),
        Err("source month exceeds the reported recovery read bound".to_owned())
    );

    // OUT OF THE BOX: the minute month is a folder.
    let folder = store_root("recovery-cover-sources-folder");
    source_month(&folder, "NIFTY", "1day", august, &[at(mon, 555, 0)]);
    let minute_path = source_path(&folder, "NIFTY", "1min", august).expect("a path");
    std::fs::create_dir_all(&minute_path).expect("a folder where the month goes");
    assert!(read_sources(&folder, "NIFTY", august, week, true).is_err());

    for root in [root, friday_root, share, crowded, folder] {
        let _ = std::fs::remove_dir_all(root);
    }
    for name in [
        "recovery-cover-sources-twice",
        "recovery-cover-sources-second",
    ] {
        let _ = std::fs::remove_dir_all(crate::scratch::path(name));
    }
}

/// A site whose Zerodha master maps NIFTY by an invented index row (the
/// shape a Zerodha `INDICES` row takes: type `EQ`, segment `INDICES`; the
/// tokens are invented), over a store of its own, the recovery's folder
/// made.
fn mapped_site(name: &str) -> (PathBuf, Loaded) {
    let root = store_root(name);
    let masters = root.join("masters");
    std::fs::create_dir_all(&masters).expect("a masters folder");
    std::fs::write(
        masters.join(Vendor::Zerodha.master_file()),
        "instrument_token,exchange_token,tradingsymbol,name,last_price,expiry,strike,\
         tick_size,lot_size,instrument_type,segment,exchange\n\
         900001,9001,NIFTY 50,NIFTY 50,0,,0,0.05,0,EQ,INDICES,NSE\n",
    )
    .expect("an invented master");
    let site = Loaded::new(Site::load(&masters, &root));
    std::fs::create_dir_all(super::root(&site)).expect("the recovery folder");
    (root, site)
}

/// Every minute an open day's exchange session holds, as stamps.
fn session_of(day: Day) -> Vec<i64> {
    let pull::calendar::DayKind::Open(session) =
        pull::calendar::kind_of(i64::from(day.days_from_epoch()))
    else {
        panic!("{day} is an open day of the calendar of record");
    };
    session
        .windows
        .iter()
        .take(usize::from(session.count))
        .flat_map(|window| {
            (window.from..=window.to).map(move |minute| at(day, i64::from(minute), 0))
        })
        .collect()
}

/// **AN ASSESSMENT COUNTS EACH DAY BY ITS OWN EVIDENCE, AND REFUSES WHAT IT
/// CANNOT COUNT** (BASE-COV-API repair 3, D-8661). `assess` reached past the
/// identity check for the first time: a site whose Zerodha master maps NIFTY,
/// over NIFTY months written here. Extremes: a day whose every session minute
/// and its daily bar are held (nothing owed); a day missing ten minutes with
/// its daily bar held (ten missing, the day retried); an open day with no
/// bar at all (its session unverified, never retried); a weekend (nothing);
/// a day with a bar at 08:00, before the session (one evidence issue); a day
/// with its daily bar and no minute (its whole session missing, retried); a
/// daily scan over the same days (held, missing and unverified by day); a
/// Muhurat day the calendar holds no length for (unverified); a daily bar on
/// a Sunday (refused by name); a scan spanning two months (refused by name);
/// and, at once, the ten-minute hole and the daily-only day in one window
/// (both retried, in day order, their minutes summed), and the bar before the
/// session on a day whose session is whole (held, and an issue, no gap); out
/// of the box, a scan of one day that is a Sunday with nothing held.
#[tokio::test]
async fn an_assessment_counts_each_day_by_its_own_evidence() {
    let (root, site) = mapped_site("recovery-cover-assess");
    let (d1, d2, d3, d6, d7) = (
        on(2026, 7, 1),
        on(2026, 7, 2),
        on(2026, 7, 3),
        on(2026, 7, 6),
        on(2026, 7, 7),
    );
    let july = d1.year_month().expect("a month");
    let hole = |stamp: &i64| {
        let ten = at(d2, 10 * 60, 0)..at(d2, 10 * 60 + 10, 0);
        !ten.contains(stamp)
    };
    let mut minutes: Vec<i64> = session_of(d1);
    minutes.extend(session_of(d2).into_iter().filter(hole));
    minutes.push(at(d6, 8 * 60, 0));
    minutes.extend(session_of(d6));
    source_month(&root, "NIFTY", "1min", july, &minutes);
    let daily: Vec<i64> = [d1, d2, d7].into_iter().map(|day| at(day, 0, 0)).collect();
    source_month(&root, "NIFTY", "1day", july, &daily);
    let window = Window::new(d1, d7).expect("a window");

    // THE MINUTE SCAN.
    let found = assess(&site, &canonical("NIFTY", "1min", window, "scan"), &None)
        .await
        .expect("assessed");
    let bars = |day: Day| u64::try_from(session_of(day).len()).expect("small");
    assert_eq!(found.held, u64::try_from(minutes.len()).expect("small"));
    assert_eq!(found.missing, 10 + bars(d7), "{found:?}");
    assert_eq!(found.unverified, bars(d3), "{found:?}");
    assert_eq!(found.evidence_issues, 1, "the bar at 08:00: {found:?}");
    assert_eq!(found.retry_days, vec![d2, d7]);
    assert_eq!(found.status(), Status::Unverified, "an issue speaks first");

    // THE DAILY SCAN OF THE SAME DAYS: d3 and d6 hold no daily bar, d6 holds
    // minutes (missing, retried), d3 nothing (unverified).
    let found = assess(&site, &canonical("NIFTY", "1day", window, "scan"), &None)
        .await
        .expect("assessed");
    assert_eq!(found.held, 3, "{found:?}");
    assert_eq!(found.missing, 1, "{found:?}");
    assert_eq!(found.unverified, 1, "{found:?}");
    assert_eq!(found.retry_days, vec![d6]);
    assert_eq!(
        found.status(),
        Status::Exhausted,
        "a day missing, none unread"
    );

    // A MUHURAT DAY OF NO MEASURED LENGTH, and a Sunday with nothing held.
    for (day, unverified) in [(on(2024, 11, 1), 1), (on(2026, 7, 5), 0)] {
        let found = assess(
            &site,
            &canonical(
                "NIFTY",
                "1day",
                Window::new(day, day).expect("a day"),
                "scan",
            ),
            &None,
        )
        .await
        .expect("assessed");
        assert_eq!(found.unverified, unverified, "{day}: {found:?}");
        assert_eq!(found.held, 0, "{day}");
    }

    // REFUSED BY NAME: a daily bar on a Sunday, and two months in one scan.
    let sunday = on(2026, 8, 2);
    source_month(
        &root,
        "NIFTY",
        "1day",
        sunday.year_month().expect("a month"),
        &[at(sunday, 0, 0)],
    );
    let why = assess(
        &site,
        &canonical(
            "NIFTY",
            "1day",
            Window::new(sunday, sunday).expect("a day"),
            "scan",
        ),
        &None,
    )
    .await
    .expect_err("a daily bar on a closed date");
    assert!(why.contains("calendar-closed date 2026-08-02"), "{why}");
    let why = assess(
        &site,
        &canonical(
            "NIFTY",
            "1min",
            Window::new(on(2026, 6, 29), on(2026, 7, 2)).expect("a window"),
            "scan",
        ),
        &None,
    )
    .await
    .expect_err("two months");
    assert!(why.contains("contained in one month"), "{why}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A start body naming each `(symbol, dir, from, to)` scan as one leg.
fn legs_of(scans: &[(&str, &str, Day, Day)]) -> String {
    scans
        .iter()
        .map(|&(symbol, dir, from, to)| {
            let inner = canonical(
                symbol,
                dir,
                Window::new(from, to).expect("a window"),
                "scan",
            );
            format!(
                "leg={}",
                encode(&format!(
                    "/pull/spot|zerodha|{dir}|fixture|{}",
                    encode(&inner)
                ))
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// Starts `body` on `site` and waits for its run to end: its plan id and the
/// sentence it finished with.
async fn started(site: &Loaded, body: String) -> ([u8; 32], String) {
    let (status, _, answer) = start(State(Loaded::clone(site)), body).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{answer}");
    let reply: serde_json::Value = serde_json::from_str(&answer).expect("JSON");
    let id = unhex(reply["plan"].as_str().expect("a plan")).expect("an id");
    tokio::time::timeout(std::time::Duration::from_mins(1), async {
        while site
            .run
            .lock()
            .expect("the run")
            .as_ref()
            .is_some_and(crate::pullrun::Progress::running)
        {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the run ends");
    let finished = site
        .run
        .lock()
        .expect("the run")
        .as_ref()
        .and_then(|run| run.finished.clone())
        .expect("a finished run");
    (id, finished)
}

/// **A START OVER A MAPPED SITE COUNTS EVERY WINDOW BY ITS OWN EVIDENCE AND
/// RECONCILES ONLY THE PENDING DAYS ITS OWN WINDOWS HOLD** (BASE-COV-API
/// repair 3, D-8661). `start` through `execute` and `reconcile_pending`, with
/// the shared attempt ledger written before the start. Extremes: a minute
/// window whose one day is whole (verified); a daily window over a weekend
/// (not applicable); a daily window over an open day with no bar (unverified);
/// a minute window over a whole day and two days with no bar (unverified); a
/// window of a member the master does not map (blocked, the run goes on); a
/// pending day inside a window, whose reassessment records (queued to
/// unverified); a pending unverified day the store still says the same of
/// (nothing recorded); a pending day of the plan's scope outside every window
/// (passed over); a pending day of a scope the plan does not hold (passed
/// over); a blocked day inside a window, reconciled because the start is
/// explicit; a pending day of the unmapped member (blocked, its diagnostics
/// counted); a day the ledger holds verified (not pending); and, at once, the
/// five windows' four verdicts in one summary sentence, and the unmapped
/// member's window and pending day blocked in one pass; out of the box, a
/// ledger row whose key is not its body's, which blocks the whole run by
/// name.
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one plan over one ledger: every window's verdict and every pending row's fate"
)]
async fn a_start_counts_every_window_and_reconciles_only_its_own_pending_days() {
    let (root, site) = mapped_site("recovery-cover-start");
    let (june, d1, d2, d3) = (
        on(2026, 6, 1),
        on(2026, 7, 1),
        on(2026, 7, 2),
        on(2026, 7, 3),
    );
    let (saturday, sunday, d6) = (on(2026, 7, 4), on(2026, 7, 5), on(2026, 7, 6));
    source_month(
        &root,
        "NIFTY",
        "1min",
        june.year_month().expect("a month"),
        &session_of(june),
    );
    source_month(
        &root,
        "NIFTY",
        "1min",
        d1.year_month().expect("a month"),
        &session_of(d1),
    );
    let gap = |symbol: &str, day: Day| {
        record(canonical(
            symbol,
            "1min",
            Window::new(day, day).expect("a day"),
            "gap",
        ))
    };
    let bars = |day: Day| u64::try_from(session_of(day).len()).expect("small");
    let queued_in = gap("NIFTY", d1);
    let mut same = gap("NIFTY", d2);
    same.status = Status::Unverified;
    same.unverified = bars(d2);
    let outside = gap("NIFTY", d6);
    let foreign = gap("SBIN", d1);
    let mut blocked = gap("NIFTY", d3);
    blocked.status = Status::Blocked;
    let unmapped = gap("BANKNIFTY", d1);
    let mut verified = gap("NIFTY", june);
    verified.status = Status::Verified;
    {
        let mut ledger = Journal::open(&super::root(&site).join("attempts.bin")).expect("a ledger");
        for row in [
            &queued_in, &same, &outside, &foreign, &blocked, &unmapped, &verified,
        ] {
            ledger.append(row.clone()).expect("a pending row");
        }
    }
    let (_id, finished) = started(
        &site,
        legs_of(&[
            ("NIFTY", "1min", june, june),
            ("NIFTY", "1day", saturday, sunday),
            ("NIFTY", "1day", d3, d3),
            ("NIFTY", "1min", d1, d3),
            ("BANKNIFTY", "1min", d1, d1),
        ]),
    )
    .await;
    assert!(
        finished.contains(
            "1 stored windows verified against measured schedule; 1 not applicable; \
             2 unverified; 1 missing/blocked"
        ),
        "{finished}"
    );
    let ledger = Journal::open(&super::root(&site).join("attempts.bin")).expect("the ledger");
    let now = |row: &Record| ledger.latest.get(&row.key).cloned().expect("a row");
    assert_eq!(
        now(&queued_in).status,
        Status::Unverified,
        "reassessed and recorded"
    );
    assert_eq!(
        now(&queued_in).diagnostics,
        1,
        "the lost receipt counted once"
    );
    assert_eq!(now(&same), same, "nothing new, nothing recorded");
    assert_eq!(now(&outside), outside, "outside every window: passed over");
    assert_eq!(now(&foreign), foreign, "a scope the plan does not hold");
    assert_eq!(
        now(&blocked).status,
        Status::Unverified,
        "reconciled on an explicit start"
    );
    assert_eq!(
        now(&unmapped).status,
        Status::Blocked,
        "its reassessment refused"
    );
    assert_eq!(now(&unmapped).diagnostics, 1);
    assert_eq!(now(&verified), verified, "a verified day is not pending");
    drop(ledger);

    // A LEDGER ROW WHOSE KEY IS NOT ITS BODY'S blocks the run by name.
    let (forged_root, forged_site) = mapped_site("recovery-cover-start-forged");
    let mut forged = gap("NIFTY", d1);
    forged.key = [7; 32];
    {
        let mut ledger =
            Journal::open(&super::root(&forged_site).join("attempts.bin")).expect("a ledger");
        ledger.append(forged).expect("a forged row");
    }
    let (_id, finished) = started(&forged_site, legs_of(&[("NIFTY", "1min", d1, d1)])).await;
    assert!(
        finished.contains("Recovery BLOCKED: shared attempt has invalid identity or non-day scope"),
        "{finished}"
    );
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&forged_root);
}

/// **THE PAGE NAMES EVERY STATE IN ITS OWN WORDS, AND A POINTER THAT NAMES
/// ANOTHER PLAN, OR A PLAN THAT WILL NOT READ, IS REFUSED BY NAME**
/// (BASE-COV-API repair 3, D-8661). `page_html` over a row of each state and
/// a run's feed line; `latest_view` over a real run's plain pointer and over
/// pointers written by hand. Extremes: a queued row; a reserved one; a
/// checked window; a window not applicable; an unverified one; one at its
/// retry limit; a blocked one; a row naming no member (not shown); a run
/// with a feed line (its windows done of all, and what it is doing); a real
/// run's pointer of one plain identity (its plan read, no predecessor); a
/// pointer whose key is not the identity its body names (refused); a plan
/// whose journal is a folder (refused, never read as missing); and, at once,
/// every state in one table beside the feed line, and the folder plan behind
/// a pointer that is itself sound; out of the box, a feed line whose doing is
/// markup (escaped, never rendered).
#[tokio::test]
async fn the_page_names_every_state_and_a_pointer_to_another_plan_is_refused() {
    let day = on(2026, 7, 1);
    let row = |status: Status| Record {
        status,
        ..record(canonical(
            "NIFTY",
            "1min",
            Window::new(day, day).expect("a day"),
            "gap",
        ))
    };
    let states = [
        (Status::Queued, "Queued"),
        (Status::InFlight, "Attempt reserved"),
        (Status::Verified, "Stored window checked"),
        (Status::NotApplicable, "Not applicable"),
        (Status::Unverified, "Unverified"),
        (Status::Exhausted, "Missing / retry limit"),
        (Status::Blocked, "Blocked"),
    ];
    let mut rows: Vec<Record> = states.iter().map(|&(status, _)| row(status)).collect();
    rows.push(record("recovery_kind=seal".to_owned()));
    let progress = Progress {
        feeds: vec![FeedReport {
            vendor: "zerodha".to_owned(),
            legs: 5,
            legs_done: 3,
            doing: "<b>Audit</b> NIFTY".to_owned(),
            ..FeedReport::default()
        }],
        ..Progress::default()
    };
    let html = page_html(&progress, Ok(rows), None);
    for (_, label) in states {
        assert!(
            html.contains(&format!("<td class=\"state\">{label}</td>")),
            "{label}"
        );
    }
    assert_eq!(
        html.matches("<tr><td>").count(),
        7,
        "the row with no member is not shown"
    );
    assert!(html.contains("zerodha: 3 of 5 scan windows checked. &lt;b&gt;Audit&lt;/b&gt; NIFTY"));

    // A REAL RUN'S POINTER: one plain identity, its plan read.
    let (root, site) = mapped_site("recovery-cover-page");
    source_month(
        &root,
        "NIFTY",
        "1min",
        day.year_month().expect("a month"),
        &session_of(day),
    );
    let (id, _) = started(&site, legs_of(&[("NIFTY", "1min", day, day)])).await;
    let view = latest_view(&site).expect("a view");
    assert_eq!(view.plan, Some(id));
    assert_eq!(
        view.predecessor, None,
        "a plain pointer names no predecessor"
    );
    assert!(
        view.records.iter().any(|row| [
            row.status == Status::Verified,
            server::param(&row.body, "member") == "NIFTY"
        ] == [true; 2]),
        "{:?}",
        view.records
    );
    let page = page(State(Loaded::clone(&site))).await.0;
    assert!(page.contains("Stored window checked"), "{page}");

    // A PLAN WHOSE JOURNAL IS A FOLDER, behind the sound pointer.
    let plan = plan_path(&site, id);
    std::fs::rename(&plan, plan.with_extension("moved")).expect("moved aside");
    std::fs::create_dir_all(&plan).expect("a folder at the plan's name");
    let why = latest_view(&site)
        .map(|view| view.plan)
        .expect_err("a folder plan");
    assert!(!why.contains("missing"), "never read as missing: {why}");
    // A POINTER WHOSE KEY IS NOT ITS BODY'S IDENTITY.
    {
        let mut active = Journal::open(&active_path(&site)).expect("the pointer journal");
        active
            .append(Record {
                key: [1; 32],
                ..record(hex([2; 32]))
            })
            .expect("a forged pointer");
    }
    let why = latest_view(&site)
        .map(|view| view.plan)
        .expect_err("refused");
    assert_eq!(why, "active recovery identity mismatch");
    let _ = std::fs::remove_dir_all(&root);
}

/// A sealed plan's journal state: its scans and its control row.
fn sealed(scans: &[Record], body: &str) -> std::collections::HashMap<[u8; 32], Record> {
    let mut latest: std::collections::HashMap<[u8; 32], Record> =
        scans.iter().map(|row| (row.key, row.clone())).collect();
    let mut control = record(body.to_owned());
    control.key = CONTROL;
    latest.insert(CONTROL, control);
    latest
}

/// **A SEAL, A DESCRIPTOR, AN ENVELOPE AND A POINTER ARE EACH REFUSED BY
/// THEIR OWN RULE** (BASE-COV-API repair 3, D-8661). `validate_seal`,
/// `successor_identity`, `checked`, `plan`, `active_history` and
/// `preflight_plan`, driven directly. Extremes: a sound original seal
/// (accepted); a seal row carrying an attempt (refused: quantities); a seal
/// whose state is `Unverified` (refused: state); a scan whose key is not its
/// body's (refused: identity); a scan set that is not the plan's identity
/// (refused: inventory); a successor seal naming another predecessor and one
/// naming another generation (refused: binding); a descriptor of two parts
/// and one of another policy (refused by name); a body on another feed, for
/// another target, under another identity rule and at the 5min rung (each
/// refused); a leg on another route and one from another vendor (refused);
/// a pointer row never activated (refused) and one whose key is not its
/// body's (refused); a plan on disk with no seal that the pointer names
/// (refused: not reseeded) and one it does not name (its seal not asked for;
/// refused only for the attempt history the site never wrote); and, at once,
/// a seal with an attempt in the `Unverified` state (refused once), and a
/// successor seal naming both another predecessor and another generation;
/// out of the box, a descriptor whose predecessor is not hexadecimal.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "every refusal of the durable plan's checks, each beside the sound case"
)]
fn a_seal_a_descriptor_an_envelope_and_a_pointer_are_each_refused_by_their_own_rule() {
    let today = today_or_now();
    let day = on(2026, 7, 1);
    let window = Window::new(day, day).expect("a day");
    let scans = vec![record(canonical("NIFTY", "1min", window, "scan"))];
    let id = scope_identity(&scans);
    // THE ORIGINAL SEAL.
    assert_eq!(validate_seal(&sealed(&scans, CONTROL_BODY), id), Ok(()));
    let tampered = |change: &dyn Fn(&mut Record)| {
        let mut latest = sealed(&scans, CONTROL_BODY);
        change(latest.get_mut(&CONTROL).expect("a seal"));
        validate_seal(&latest, id).expect_err("refused")
    };
    for why in [
        tampered(&|seal| seal.attempts = 1),
        tampered(&|seal| seal.status = Status::Unverified),
        tampered(&|seal| {
            seal.attempts = 1;
            seal.status = Status::Unverified;
        }),
    ] {
        assert!(why.contains("invalid control state or quantities"), "{why}");
    }
    let mut forged = scans.clone();
    forged[0].key = [3; 32];
    let why = validate_seal(&sealed(&forged, CONTROL_BODY), id).expect_err("refused");
    assert!(why.contains("invalid work identity"), "{why}");
    let why = validate_seal(&sealed(&scans, CONTROL_BODY), [4; 32]).expect_err("refused");
    assert!(
        why.contains("does not match its original plan identity"),
        "{why}"
    );
    // A SUCCESSOR SEAL binds its predecessor (the scope) and its generation.
    let descriptor =
        |predecessor: [u8; 32]| format!("{SUCCESSOR_POLICY}:{}:{}", hex(predecessor), hex([9; 32]));
    let bound = descriptor(id);
    let (generation, predecessor) = successor_identity(&bound).expect("a descriptor");
    assert_eq!(predecessor, id);
    assert_eq!(validate_seal(&sealed(&scans, &bound), generation), Ok(()));
    for (body, at) in [
        (
            descriptor([5; 32]),
            successor_identity(&descriptor([5; 32])).expect("read").0,
        ),
        (bound.clone(), [6; 32]),
        (descriptor([5; 32]), [6; 32]),
    ] {
        let why = validate_seal(&sealed(&scans, &body), at).expect_err("refused");
        assert!(why.contains("does not bind its exact scope"), "{why}");
    }
    for (body, why) in [
        (
            format!("{SUCCESSOR_POLICY}:{}", hex(id)),
            "unknown generation descriptor",
        ),
        (
            format!("another-policy:{}:{}", hex(id), hex(id)),
            "unknown generation policy",
        ),
    ] {
        let refused = successor_identity(&body).expect_err("refused");
        assert!(refused.contains(why), "{refused}");
    }
    assert!(successor_identity(&format!("{SUCCESSOR_POLICY}:not-hex:{}", hex(id))).is_err());

    // ENVELOPES `checked` REFUSES, each parsed as a spot request first.
    let good = canonical("NIFTY", "1min", window, "scan");
    assert!(checked(&good, today).is_ok());
    for (from, to) in [
        (
            "vendor=zerodha&cash_identity=zerodha_cross_checked",
            "vendor=dhan",
        ),
        ("target=fno", "target=swept"),
        (
            "cash_identity=zerodha_cross_checked",
            "cash_identity=zerodha_nse_checked",
        ),
        ("granularity=1min", "granularity=5min"),
    ] {
        let body = good.replace(from, to);
        let parsed = ingest::parse_spot(&body, today);
        assert!(parsed.is_ok(), "{to}: a spot request: {:?}", parsed.err());
        let why = checked(&body, today).expect_err(to);
        assert!(
            why.contains("recovery requires explicit F&O spot members"),
            "{to}: {why}"
        );
    }
    let leg = |route: Route, vendor: &str| Leg {
        route,
        vendor: vendor.to_owned(),
        dir: "1min".to_owned(),
        label: "fixture".to_owned(),
        body: good.clone(),
    };
    assert!(plan(vec![leg(Route::Spot, "zerodha")], today).is_ok());
    for refused in [leg(Route::Fno, "zerodha"), leg(Route::Spot, "dhan")] {
        assert!(plan(vec![refused], today).is_err());
    }

    // POINTERS AND PLANS ON DISK.
    let (root, site) = refused_site("recovery-cover-seal");
    for pointer in [
        Record {
            attempts: 0,
            ..record(hex(id))
        },
        Record {
            attempts: 1,
            key: [8; 32],
            ..record(hex(id))
        },
    ] {
        let path = active_path(&site);
        let _ = std::fs::remove_file(&path);
        Journal::open(&path)
            .expect("the pointer journal")
            .append(pointer)
            .expect("a pointer");
        let why = active_history(&site).expect_err("refused");
        assert!(
            why.contains("invalid identity or activation state"),
            "{why}"
        );
    }
    let path = active_path(&site);
    std::fs::remove_file(&path).expect("removed");
    Journal::open(&path)
        .expect("the pointer journal")
        .append(Record {
            attempts: 1,
            key: id,
            ..record(hex(id))
        })
        .expect("a sound pointer");
    assert!(active_history(&site).is_ok(), "the sound pointer reads");
    // THE PLAN THE POINTER NAMES, ON DISK WITH NO SEAL: refused.
    Journal::open(&plan_path(&site, id))
        .expect("a plan journal")
        .append(scans[0].clone())
        .expect("a scan");
    let why = preflight_plan(&site, id).expect_err("refused");
    assert!(
        why.contains("has no durable seal; history was not reseeded"),
        "{why}"
    );
    // ONE IT DOES NOT NAME: no seal to check, so the only refusal is the
    // shared attempt history this site has never written.
    let other = [11; 32];
    Journal::open(&plan_path(&site, other))
        .expect("a plan journal")
        .append(scans[0].clone())
        .expect("a scan");
    let why = preflight_plan(&site, other).expect_err("refused for the ledger alone");
    assert!(
        why.contains("shared recovery attempt history is unavailable"),
        "{why}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A site whose masters map FORCEMOT under the cross-checked identity: a
/// Zerodha cash row and a Groww witness carrying the share's ISIN from the
/// workspace's own universe table (`brutex_core::universe::nse_isin`); the
/// tokens are invented. NIFTY's invented index row stands beside it.
fn cross_checked_site(name: &str) -> (PathBuf, Loaded) {
    let root = store_root(name);
    let masters = root.join("masters");
    std::fs::create_dir_all(&masters).expect("a masters folder");
    let isin = brutex_core::universe::nse_isin("FORCEMOT").expect("an F&O share");
    std::fs::write(
        masters.join(Vendor::Zerodha.master_file()),
        "instrument_token,exchange_token,tradingsymbol,name,last_price,expiry,strike,\
         tick_size,lot_size,instrument_type,segment,exchange\n\
         900001,9001,NIFTY 50,NIFTY 50,0,,0,0.05,0,EQ,INDICES,NSE\n\
         900002,9002,FORCEMOT,FORCE MOTORS,0,,0,0.05,1,EQ,NSE,NSE\n",
    )
    .expect("an invented master");
    std::fs::write(
        masters.join(Vendor::Groww.master_file()),
        format!(
            "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,\
             expiry_date,strike_price,groww_symbol\n\
             NSE,CASH,,FORCEMOT,EQ,EQ,{},,,NSE-FORCEMOT\n",
            isin.as_str()
        ),
    )
    .expect("an invented witness");
    std::fs::write(
        masters.join(Vendor::Dhan.master_file()),
        "EXCH_ID,SEGMENT,ISIN,INSTRUMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,INSTRUMENT_TYPE,SERIES,\
         SM_EXPIRY_DATE,STRIKE_PRICE,OPTION_TYPE,SECURITY_ID\n",
    )
    .expect("an empty master");
    let site = Loaded::new(Site::load(&masters, &root));
    std::fs::create_dir_all(super::root(&site)).expect("the recovery folder");
    (root, site)
}

/// **A SHARE'S ASSESSMENT READS ITS CASH SESSION AND ITS WITHDRAWN DAYS BY
/// THEIR OWN EVIDENCE** (BASE-COV-API repair 3, D-8661). `assess` over a
/// cross-checked share, FORCEMOT, inside its sourced withdrawal (2023-10-26
/// to 2024-02-13, `pull::cash_auction::independently_inactive`), before the
/// closing auction's eligibility applies, so no schedule is fetched. Extremes:
/// half a session held on a withdrawn day (every other minute of that day
/// unverified, a withdrawn day having no session to measure against; never
/// missing, never retried); a daily bar on a withdrawn day with no minute (its
/// whole day unverified); withdrawn days with no bar (nothing counted);
/// a weekend (nothing); each withdrawn day held, an issue; the share held
/// with no lifecycle master, an issue; no day retried; the share mapped only
/// by the cross-check (a Zerodha row and an independent witness); and, at
/// once, the minutes and the daily bar on two withdrawn days of one window
/// (both unverified, both issues), and the share's cash shape read beside its
/// stored minutes with no conflict; out of the box, a share the exchange
/// withdrew, asked as if it traded.
#[tokio::test]
async fn a_shares_assessment_reads_its_cash_session_and_its_withdrawn_days() {
    let (root, site) = cross_checked_site("recovery-cover-share");
    let (jan2, jan4) = (on(2024, 1, 2), on(2024, 1, 4));
    let half: Vec<i64> = {
        let whole = session_of(jan2);
        whole[..whole.len() / 2].to_vec()
    };
    let january = jan2.year_month().expect("a month");
    source_month(&root, "FORCEMOT", "1min", january, &half);
    source_month(&root, "FORCEMOT", "1day", january, &[at(jan4, 0, 0)]);
    let window = Window::new(on(2024, 1, 1), on(2024, 1, 7)).expect("a window");
    let found = assess(&site, &canonical("FORCEMOT", "1min", window, "scan"), &None)
        .await
        .expect("assessed");
    let held = u64::try_from(half.len()).expect("small");
    assert_eq!(found.held, held, "{found:?}");
    assert_eq!(
        found.missing, 0,
        "a withdrawn day is never missing: {found:?}"
    );
    // A WITHDRAWN DAY HAS NO SESSION TO MEASURE AGAINST: every minute of a
    // withdrawn day that holds a bar, and is not itself held, is unverified.
    assert_eq!(
        found.unverified,
        (1_440 - held) + 1_440,
        "the two withdrawn days that hold a bar, whole: {found:?}"
    );
    assert_eq!(found.retry_days, Vec::<Day>::new(), "never retried");
    assert_eq!(
        found.evidence_issues, 3,
        "two withdrawn days held, and no lifecycle master: {found:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Every F&O underlying a recovery may name: the universe table less the four
/// indices `checked` refuses.
fn recoverable() -> Vec<&'static str> {
    brutex_core::universe::FNO_UNDERLYINGS
        .iter()
        .copied()
        .filter(|name| {
            !matches!(
                *name,
                "FINNIFTY" | "MIDCPNIFTY" | "NIFTYNXT50" | "NIFTYNEXT50"
            )
        })
        .collect()
}

/// **EVERY 100,000-UNIT BOUND REFUSES BY NAME, A UNIT OF TWO SYMBOLS IS NO
/// UNIT, AND A SOURCE MINUTE OUTSIDE THE AUDITED SHAPE IS REFUSED**
/// (BASE-COV-API repair 3, D-8661). `plan`, `validate_seal`, `append_attempt`,
/// `symbol` and `outside_session_count`, driven directly; nothing is
/// truncated and nothing is written past a bound. Extremes: a plan of 477
/// one-day legs, each naming every recoverable underlying over its own
/// window of January 2024 (100,170 windows: refused at the 100,001st, by
/// name); a seal over 100,001 scan windows (refused: oversized); a shared
/// attempt index holding 100,000 units asked for a new one (refused by
/// name, nothing appended to either journal); the same index asked for a
/// unit it already holds (appended: the bound counts units, not
/// attempts); a unit naming two symbols (refused); one naming none
/// (refused); one naming one (that symbol); a source minute on a day before
/// the audited shape's first (refused: outside the shape); and, at once, a
/// shape over every day the calendar of record measures with no minute
/// stored (refused: the shape itself is cut at its gap bound), and a plan
/// just under its bound (476 legs, 99,960 windows: answered) beside a seal
/// one window over it (refused); out of the box, a minute stored on the
/// Sunday before an audited Monday (outside the shape, never a conflict
/// counted).
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "every bound beside the case one unit under it"
)]
fn every_unit_bound_refuses_by_name_and_a_minute_outside_the_audited_shape_is_refused() {
    let today = today_or_now();
    let members = recoverable();
    assert!(members.len() > 200, "{}", members.len());
    let windows: Vec<Window> = (1_u8..=31)
        .flat_map(|from| (from..=31).map(move |to| (from, to)))
        .map(|(from, to)| Window::new(on(2024, 1, from), on(2024, 1, to)).expect("a window"))
        .collect();
    let leg = |window: Window| {
        let mut body = format!(
            "target=fno&vendor=zerodha&cash_identity=zerodha_cross_checked&granularity=1day&from={}&to={}&recovery_kind=scan",
            window.from(),
            window.to()
        );
        // ENCODED AS `canonical` ENCODES ONE: an underlying such as `M&M`
        // would otherwise split into two fields.
        for member in &members {
            let _ = write!(body, "&member={}", encode(member));
        }
        Leg {
            route: Route::Spot,
            vendor: "zerodha".to_owned(),
            dir: "1day".to_owned(),
            label: "fixture".to_owned(),
            body,
        }
    };
    let over = 100_001_usize.div_ceil(members.len());
    let why = plan(windows.iter().take(over).copied().map(leg).collect(), today)
        .expect_err("past the bound");
    assert_eq!(
        why,
        "recovery plan exceeds its reported 100,000-window bound; nothing was truncated"
    );
    let under = 100_000 / members.len();
    let planned = plan(
        windows.iter().take(under).copied().map(leg).collect(),
        today,
    )
    .expect("under the bound");
    assert_eq!(planned.len(), under * members.len());

    // A SEAL OVER 100,001 SCAN WINDOWS.
    let scans: Vec<Record> = windows
        .iter()
        .flat_map(|window| {
            members
                .iter()
                .map(move |member| record(canonical(member, "1day", *window, "scan")))
        })
        .take(100_001)
        .collect();
    assert_eq!(scans.len(), 100_001);
    let why = validate_seal(&sealed(&scans, CONTROL_BODY), scope_identity(&scans))
        .expect_err("oversized");
    assert!(why.contains("empty, oversized"), "{why}");

    // A SHARED ATTEMPT INDEX AT ITS BOUND.
    let (root, site) = refused_site("recovery-cover-bounds");
    let mut plan_journal = Journal::open(&plan_path(&site, [21; 32])).expect("a plan journal");
    let mut shared = Journal::open(&root.join("recovery-shared-fixture.bin")).expect("a ledger");
    for n in 0_u32..100_000 {
        let mut key = [0_u8; 32];
        key[..4].copy_from_slice(&n.to_le_bytes());
        shared.latest.insert(
            key,
            Record {
                key,
                ..scans[0].clone()
            },
        );
    }
    let fresh = scans[1].clone();
    let why = append_attempt(&mut plan_journal, &mut shared, fresh.clone()).expect_err("bound");
    assert_eq!(
        why,
        "shared recovery attempt index reached its 100,000-unit bound"
    );
    assert!(
        !plan_journal.latest.contains_key(&fresh.key),
        "nothing appended"
    );
    assert_eq!(shared.latest.len(), 100_000);
    let held = shared.latest[&[0_u8; 32]].clone();
    append_attempt(&mut plan_journal, &mut shared, held.clone()).expect("a unit it holds");
    assert_eq!(plan_journal.latest.len(), 1, "appended to the plan too");
    drop(plan_journal);
    drop(shared);
    let _ = std::fs::remove_dir_all(&root);

    // A UNIT OF TWO SYMBOLS, OF NONE, OF ONE.
    let day = on(2026, 7, 1);
    let one = canonical(
        "NIFTY",
        "1min",
        Window::new(day, day).expect("a day"),
        "scan",
    );
    let parsed = |body: &str| ingest::parse_spot(body, today).expect("a spot request");
    assert_eq!(symbol(&parsed(&one)), Ok("NIFTY"));
    for body in [
        one.replace("member=NIFTY", "member=NIFTY&member=BANKNIFTY"),
        one.replace("member=NIFTY", ""),
    ] {
        assert_eq!(
            symbol(&parsed(&body)),
            Err("recovery unit must name exactly one symbol".to_owned()),
            "{body}"
        );
    }

    // A MINUTE OUTSIDE THE AUDITED SHAPE.
    let monday = on(2026, 8, 31);
    let number = i64::from(monday.days_from_epoch());
    for before in [on(2026, 8, 28), on(2026, 8, 30)] {
        assert_eq!(
            outside_session_count(&[at(before, 600, 0)], "NIFTY", number, number, None),
            Err("stored minute lies outside the audited session shape".to_owned()),
            "{before}"
        );
    }
    assert_eq!(
        outside_session_count(
            &[],
            "NIFTY",
            pull::calendar::FIRST_DAY,
            pull::calendar::LAST_DAY,
            None
        ),
        Err("independent session shape could not be fully checked".to_owned())
    );
}

/// A sound active pointer for plan `id`: activated once, keyed by its body.
fn pointer(id: [u8; 32]) -> Record {
    Record {
        attempts: 1,
        key: id,
        ..record(hex(id))
    }
}

/// **A SUCCESSOR IS PREPARED ONLY OVER A MISSING PREDECESSOR AND A QUEUED
/// SCOPE THAT MATCHES, AND ACTIVATED ONLY ONCE PREPARED WHOLE**
/// (BASE-COV-API repair 3, D-8661). `prepare_successor` and
/// `preflight_submission`, driven directly over durable states this module's
/// own journals write: an active pointer naming the predecessor, the shared
/// attempt ledger, and the successor's plan in each shape it may be found in.
/// Extremes: a successor the pointer history already names (refused:
/// already activated); a predecessor whose plan is still on disk (refused:
/// nothing replaced); a sound preparation (sealed, its scope queued); the
/// same preparation asked again (answered, nothing appended twice); a
/// successor plan holding a window outside the scope asked (refused:
/// changed scope); one holding the scope's window verified (refused); one
/// holding the window queued but with committed rows (refused: the window
/// disagrees); an activation over a plan holding the scope with no seal
/// (refused: not prepared); and, at once, an activation over a sealed plan
/// asked with a window it never prepared (refused: incomplete scope), and
/// the predecessor's plan on disk while the successor is already named
/// (refused for the activation first); out of the box, a sound preparation
/// read back by the activation check it was made for (accepted).
#[test]
fn a_successor_is_prepared_only_over_a_missing_predecessor_and_activated_only_once_whole() {
    let (root, site) = refused_site("recovery-cover-successor");
    let day = on(2026, 7, 1);
    let window = Window::new(day, day).expect("a day");
    let unit = record(canonical("NIFTY", "1min", window, "scan"));
    // THE PREDECESSOR IS THE SCOPE'S OWN IDENTITY, as `submission` demands
    // and `validate_seal` checks of every successor seal.
    let predecessor = scope_identity(std::slice::from_ref(&unit));
    let seal = format!(
        "{SUCCESSOR_POLICY}:{}:{}",
        hex(predecessor),
        hex([0x52; 32])
    );
    let id = brutex_core::blake3::hash(seal.as_bytes());
    let asked = |units: Vec<Record>| Submission {
        id,
        units,
        successor: Some((predecessor, seal.clone())),
        prepare_only: true,
    };
    let active = active_path(&site);
    Journal::open(&active)
        .expect("the pointer journal")
        .append(pointer(predecessor))
        .expect("the predecessor's pointer");
    drop(Journal::open(&super::root(&site).join("attempts.bin")).expect("the shared ledger"));
    let plan = plan_path(&site, id);
    let reset = |rows: &[Record]| {
        let _ = std::fs::remove_file(&plan);
        let mut journal = Journal::open(&plan).expect("a successor plan");
        for row in rows {
            journal.append(row.clone()).expect("a row");
        }
    };

    // ALREADY ACTIVATED, AND A PREDECESSOR STILL ON DISK.
    Journal::open(&active)
        .expect("the pointer journal")
        .append(pointer(id))
        .expect("the successor's pointer");
    let held = plan_path(&site, predecessor);
    std::fs::write(&held, b"").expect("the predecessor's plan");
    assert_eq!(
        prepare_successor(&site, &asked(vec![unit.clone()])),
        Err("successor was already activated; preparation cannot reset it".to_owned()),
        "the activation is refused first"
    );
    std::fs::remove_file(&active).expect("the pointers reset");
    Journal::open(&active)
        .expect("the pointer journal")
        .append(pointer(predecessor))
        .expect("the predecessor's pointer");
    assert_eq!(
        prepare_successor(&site, &asked(vec![unit.clone()])),
        Err("predecessor plan exists; the missing-plan repair cannot replace it".to_owned())
    );
    std::fs::remove_file(&held).expect("the predecessor's plan removed");

    // A SOUND PREPARATION, TWICE, READ BACK BY ITS ACTIVATION CHECK.
    assert_eq!(prepare_successor(&site, &asked(vec![unit.clone()])), Ok(()));
    assert_eq!(prepare_successor(&site, &asked(vec![unit.clone()])), Ok(()));
    let prepared = crate::recovery_journal::snapshot(&plan).expect("the prepared plan");
    assert_eq!(
        prepared.latest.len(),
        2,
        "the window and the seal, once each"
    );
    assert_eq!(
        preflight_submission(&site, &asked(vec![unit.clone()])),
        Ok(())
    );
    let other = record(canonical("BANKNIFTY", "1min", window, "scan"));
    assert_eq!(
        preflight_submission(&site, &asked(vec![unit.clone(), other.clone()])),
        Err("prepared successor scope is incomplete; activation refused".to_owned())
    );

    // A SUCCESSOR PLAN IN EVERY SHAPE IT MAY BE FOUND IN.
    let changed =
        "existing successor contains changed scope, activation or work; nothing was reset";
    reset(std::slice::from_ref(&other));
    assert_eq!(
        prepare_successor(&site, &asked(vec![unit.clone()])),
        Err(changed.to_owned()),
        "a window outside the scope"
    );
    reset(&[Record {
        status: Status::Verified,
        ..unit.clone()
    }]);
    assert_eq!(
        prepare_successor(&site, &asked(vec![unit.clone()])),
        Err(changed.to_owned()),
        "the window verified"
    );
    reset(&[Record {
        committed: 7,
        ..unit.clone()
    }]);
    assert_eq!(
        prepare_successor(&site, &asked(vec![unit.clone()])),
        Err("existing successor window disagrees with the requested queued scope".to_owned())
    );
    reset(std::slice::from_ref(&unit));
    assert_eq!(
        preflight_submission(&site, &asked(vec![unit.clone()])),
        Err("successor must be prepared with its exact predecessor and generation before activation".to_owned())
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A START REFUSED A RUN SLOT OR A DURABLE ACTIVATION SAYS SO AND STOPS,
/// AND A PLAN NEVER SEALED IS NEVER SEEDED FOR WORK** (BASE-COV-API repair
/// 3, D-8661). `start` over a site whose run slot another pull holds, and
/// over one whose first activation cannot stage its pointer (a folder where
/// the staging file goes); `seeded` with no new scope over a plan on disk
/// with no seal. Extremes: a start while a pull owns the run slot (`409`,
/// not started); its refusal naming the owner (a pull owns the slot); the
/// slot left as that pull holds it; a start whose activation cannot stage
/// its pointer (`503`, not started, the reason named); the run left
/// finished and blocked by that reason; no pointer published by it
/// (`active.bin` absent); a plan journal without a seal asked to be seeded
/// with no new windows (refused: not durably seeded); the same plan with
/// its seal (seeded); and, at once, one start answered `503` with its plan
/// kept, its run blocked and no pointer linked, and an unsealed plan asked
/// to be seeded on a store whose recovery folder already exists (refused,
/// the folder untouched); out of the box, a staging folder holding a file
/// of its own (left as it was).
#[tokio::test]
async fn a_start_refused_a_slot_or_an_activation_says_so_and_a_plan_never_sealed_is_never_seeded() {
    let day = on(2026, 7, 1);
    let body = legs_of(&[("NIFTY", "1min", day, day)]);
    let ask = |site: &Loaded| start(axum::extract::State(Loaded::clone(site)), body.clone());

    // A PULL OWNS THE RUN SLOT.
    let (slot_root, slot_site) = refused_site("recovery-cover-slot");
    *slot_site.run.lock().expect("the run slot") = Some(Progress::claimed());
    let (status, _, said) = ask(&slot_site).await;
    assert_eq!(status, StatusCode::CONFLICT, "{said}");
    let said: serde_json::Value = serde_json::from_str(&said).expect("JSON");
    assert_eq!(said["started"], false);
    assert!(
        said["why"]
            .as_str()
            .is_some_and(|why| why.starts_with("a pull already owns the run slot")),
        "{said}"
    );
    assert!(
        slot_site
            .run
            .lock()
            .expect("the run slot")
            .as_ref()
            .is_some_and(Progress::running),
        "the pull still holds it"
    );
    let _ = std::fs::remove_dir_all(&slot_root);

    // THE FIRST POINTER CANNOT BE STAGED.
    let (stage_root, stage_site) = refused_site("recovery-cover-staging");
    let staging = super::root(&stage_site).join("active.bin.first");
    std::fs::create_dir_all(&staging).expect("a folder at the staging name");
    std::fs::write(staging.join("held"), b"a file of its own").expect("a file inside");
    let (status, _, said) = ask(&stage_site).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{said}");
    let said: serde_json::Value = serde_json::from_str(&said).expect("JSON");
    assert_eq!(said["started"], false);
    let why = said["why"].as_str().expect("the reason").to_owned();
    assert!(!why.is_empty());
    let finished = stage_site
        .run
        .lock()
        .expect("the run slot")
        .as_ref()
        .and_then(|progress| progress.finished.clone())
        .expect("the run finished");
    assert!(finished.starts_with("Recovery BLOCKED: "), "{finished}");
    assert!(finished.contains(&why), "{finished}");
    assert!(!active_path(&stage_site).exists(), "no pointer was linked");
    let staged = Window::new(day, day).expect("a day");
    let planned = scope_identity(&[record(canonical("NIFTY", "1min", staged, "scan"))]);
    assert!(plan_path(&stage_site, planned).exists(), "the plan kept");
    assert!(staging.join("held").exists(), "the folder untouched");
    let _ = std::fs::remove_dir_all(&stage_root);

    // A PLAN WITH NO SEAL IS NEVER SEEDED FOR WORK.
    let (seed_root, seed_site) = refused_site("recovery-cover-unsealed");
    let window = Window::new(day, day).expect("a day");
    let scans = vec![record(canonical("NIFTY", "1min", window, "scan"))];
    let id = scope_identity(&scans);
    Journal::open(&plan_path(&seed_site, id))
        .expect("a plan journal")
        .append(scans[0].clone())
        .expect("a window");
    assert_eq!(
        seeded(&seed_site, id, None).map(|_| ()),
        Err("recovery plan was not durably seeded; no vendor work is allowed".to_owned())
    );
    assert!(super::root(&seed_site).is_dir(), "the folder untouched");
    let mut control = record(CONTROL_BODY.to_owned());
    control.key = CONTROL;
    Journal::open(&plan_path(&seed_site, id))
        .expect("the plan journal")
        .append(control)
        .expect("its seal");
    assert!(seeded(&seed_site, id, None).is_ok(), "sealed: seeded");
    let _ = std::fs::remove_dir_all(&seed_root);
}

/// The variable naming the proof file of
/// [`a_pointer_to_a_plan_that_is_gone_or_no_file_is_refused_and_a_page_under_a_full_pool_says_so`]'s
/// child.
const FULL_POOL_CHILD: &str = "BRUTEX_RECOVERY_FULL_POOL_CHILD";

/// **A POINTER TO A PLAN THAT IS GONE OR IS NO FILE IS REFUSED AT THE BOOT
/// CHECK AND AT RESUME, AND A PAGE ASKED UNDER A FULL STORE-READ POOL SAYS
/// ITS JOURNAL WAS NOT READ** (BASE-COV-API repair 3, D-8661).
/// `in_flight_unopened` and `resume_with` over a sound pointer whose plan is
/// missing or is a folder; `page` in a child of this test binary whose
/// store-read pool is crowded (the child is alone, so no other test is
/// starved), proved by the file its parent reads. Extremes: no pointer at
/// the boot check (nothing in flight); no pointer at resume (nothing
/// resumed); a pointer to a plan that is gone, at the boot check (the
/// missing plan named); the same at resume (named the same); a pointer to a
/// plan that is a folder, at the boot check (refused, never called
/// missing); the same at resume (refused); the page with the pool full (the
/// journal not read, the pool's bound said); the folder left as it was;
/// and, at once, a folder plan asked at the boot check and at resume in
/// turn (both refused, neither called missing), and the full-pool page over
/// a store with no recovery history at all (the pool's refusal said, not
/// the history's); out of the box, a resume whose driver is never reached
/// (refused before any work).
#[tokio::test]
async fn a_pointer_to_a_plan_that_is_gone_or_no_file_is_refused_and_a_page_under_a_full_pool_says_so()
 {
    if let Some(proof) = std::env::var_os(FULL_POOL_CHILD) {
        let (root, site) = refused_site("recovery-cover-full-pool");
        let crowd = crate::detail::crowd(crate::detail::Named::StoreRead);
        let axum::response::Html(html) = page(axum::extract::State(Loaded::clone(&site))).await;
        drop(crowd);
        std::fs::write(proof, html).expect("the child's proof");
        let _ = std::fs::remove_dir_all(&root);
        return;
    }
    let never = |_site: Loaded, _id: [u8; 32], _prepared: Result<Journal, String>| -> () {
        unreachable!("no driver runs over a refused plan")
    };
    let (root, site) = refused_site("recovery-cover-pointer");
    assert_eq!(in_flight_unopened(&site), Ok(None), "no pointer");
    assert_eq!(resume_with(Loaded::clone(&site), never), Ok(false));
    let day = on(2026, 7, 1);
    let scans = vec![record(canonical(
        "NIFTY",
        "1min",
        Window::new(day, day).expect("a day"),
        "scan",
    ))];
    let id = scope_identity(&scans);
    Journal::open(&active_path(&site))
        .expect("the pointer journal")
        .append(pointer(id))
        .expect("a sound pointer");
    assert_eq!(in_flight_unopened(&site), Err(missing_plan(id)), "gone");
    assert_eq!(
        resume_with(Loaded::clone(&site), never),
        Err(missing_plan(id))
    );
    let plan = plan_path(&site, id);
    std::fs::create_dir_all(&plan).expect("a folder at the plan's path");
    for refused in [
        in_flight_unopened(&site).map(|_| ()),
        resume_with(Loaded::clone(&site), never).map(|_| ()),
    ] {
        let why = refused.expect_err("a folder is no plan");
        assert_ne!(why, missing_plan(id), "never called missing");
        assert!(!why.is_empty());
    }
    assert!(plan.is_dir(), "the folder left as it was");
    let _ = std::fs::remove_dir_all(&root);

    // THE PAGE, IN A CHILD WHOSE STORE-READ POOL IS FULL.
    let dir = crate::scratch::path("recovery-cover-full-pool-proof");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    let proof = dir.join("child-ran");
    let _ = crate::isolated::rerun(
        "recovery::cover_tests::a_pointer_to_a_plan_that_is_gone_or_no_file_is_refused_and_a_page_under_a_full_pool_says_so",
        &[(FULL_POOL_CHILD, proof.as_os_str())],
    );
    let html = std::fs::read_to_string(&proof).expect("the child ran and wrote its proof");
    assert!(html.contains("the recovery journal was not read"), "{html}");
    assert!(
        html.contains(&format!(
            "at most {} store reads run at once",
            crate::detail::MAX_STORE_READ_CONCURRENT
        )),
        "{html}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **A DAY OUTSIDE THE WINDOW SAYS NOTHING ABOUT IT, A DAILY COUNT READS A
/// WITHDRAWN DAY BY ITS OWN BARS, AND A REASSESSMENT RECORDS ONLY WHAT MOVED**
/// (BASE-COV-API repair 3, D-8661). `qualify_lifecycle`, `assess_daily`,
/// `reassessed` and `outside_session_count`, driven directly over the arms no
/// whole audit reached. Extremes: a withdrawn day held before the window (no
/// issue); a listed share holding a day before the window and one holding a
/// day after it (no listing conflict either way); a share whose master row
/// holds no date, with only a day missing (an issue, its retry kept); a
/// withdrawn day held in the minutes alone (unverified) and one with nothing
/// held (not counted); an unverified attempt whose unverified count moved
/// (recorded, one diagnostic more) and one whose counts did not (nothing
/// recorded); an attempt owed a retry at its limit (exhausted); two minutes of
/// one open day handed out of order (refused: outside the audited shape) and
/// the same two in order (read: one outside the session); and, at once, a
/// listed share holding a day on each side of the window with its retry owed
/// before the listing (no conflict, the retry dropped), and a daily count over
/// two withdrawn days and the readmitted day (one unverified by its minutes,
/// one not counted, one held); out of the box, an unverified attempt whose
/// unverified count fell to nothing with nothing missing (still recorded
/// unverified: a lost receipt is never made clean).
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one master, every window-bounded arm beside the case it is read against"
)]
fn a_day_outside_the_window_says_nothing_and_a_reassessment_records_only_what_moved() {
    let window = Window::new(on(2026, 8, 24), on(2026, 8, 27)).expect("a window");
    let master = Some(lifecycle_of(
        "recovery-cover-outside",
        &[
            ("RELIANCE", Some(on(2026, 8, 26)), None, None),
            ("SBIN", None, None, None),
        ],
    ));
    let (before, tue, wed, after) = (
        on(2026, 8, 21),
        on(2026, 8, 25),
        on(2026, 8, 26),
        on(2026, 8, 28),
    );

    // A WITHDRAWN DAY HELD BEFORE THE WINDOW is not this window's.
    let withdrawn = Window::new(on(2024, 1, 2), on(2024, 1, 4)).expect("a window");
    assert_eq!(
        qualified(
            assessment(0, 0, &[]),
            &master,
            "FORCEMOT",
            withdrawn,
            &[on(2023, 12, 29)],
            &[]
        ),
        (0, vec![])
    );
    // A LISTED SHARE: a day held before the window, and one after it, conflict
    // with no listing; the retry on the listing day stands.
    assert_eq!(
        qualified(
            assessment(1, 1, &[wed]),
            &master,
            "RELIANCE",
            window,
            &[before],
            &[]
        ),
        (0, vec![wed])
    );
    assert_eq!(
        qualified(
            assessment(1, 1, &[wed]),
            &master,
            "RELIANCE",
            window,
            &[],
            &[after]
        ),
        (0, vec![wed])
    );
    // NO DATE IN THE MASTER ROW, ONLY A DAY MISSING: an issue, the retry kept.
    assert_eq!(
        qualified(assessment(0, 1, &[tue]), &master, "SBIN", window, &[], &[]),
        (1, vec![tue])
    );
    // AT ONCE: a day on each side of the window, the retry before the listing.
    assert_eq!(
        qualified(
            assessment(2, 1, &[tue]),
            &master,
            "RELIANCE",
            window,
            &[before],
            &[after]
        ),
        (0, vec![])
    );

    // A DAILY COUNT OVER WITHDRAWN DAYS (FORCEMOT, withdrawn through
    // 2024-02-13 and readmitted the next day).
    let (feb_12, feb_13, feb_14) = (on(2024, 2, 12), on(2024, 2, 13), on(2024, 2, 14));
    let set = |days: &[Day]| days.iter().copied().collect::<HashSet<Day>>();
    let counts = |found: &Assessment| {
        (
            found.held,
            found.missing,
            found.unverified,
            found.retry_days.len(),
        )
    };
    let one = Window::new(feb_13, feb_13).expect("a day");
    let found = assess_daily(one, &set(&[]), &set(&[feb_13]), "FORCEMOT").expect("counted");
    assert_eq!(counts(&found), (0, 0, 1, 0), "held in the minutes alone");
    let found = assess_daily(one, &set(&[]), &set(&[]), "FORCEMOT").expect("counted");
    assert_eq!(
        counts(&found),
        (0, 0, 0, 0),
        "nothing held, nothing counted"
    );
    let found = assess_daily(
        Window::new(feb_12, feb_14).expect("a window"),
        &set(&[feb_14]),
        &set(&[feb_13]),
        "FORCEMOT",
    )
    .expect("counted");
    assert_eq!(
        counts(&found),
        (1, 0, 1, 0),
        "one withdrawn day by its minutes, one not counted, the readmitted day held"
    );

    // A REASSESSMENT RECORDS ONLY WHAT MOVED.
    let pending = Record {
        status: Status::Unverified,
        unverified: 5,
        diagnostics: 2,
        ..record(canonical(
            "NIFTY",
            "1min",
            Window::new(tue, tue).expect("a day"),
            "gap",
        ))
    };
    let moved = Assessment {
        unverified: 3,
        evidence_issues: 1,
        ..Assessment::default()
    };
    let next = reassessed(&pending, &moved).expect("recorded");
    assert_eq!(
        (next.status, next.missing, next.unverified, next.diagnostics),
        (Status::Unverified, 0, 3, 4),
        "the evidence issue and the lost receipt, one each"
    );
    let same = Assessment {
        unverified: 5,
        ..Assessment::default()
    };
    assert_eq!(reassessed(&pending, &same), None, "nothing moved");
    let at_limit = Record {
        status: Status::Queued,
        attempts: ATTEMPT_LIMIT,
        ..pending.clone()
    };
    let owed = Assessment {
        missing: 7,
        retry_days: vec![tue],
        ..Assessment::default()
    };
    assert_eq!(
        reassessed(&at_limit, &owed).map(|next| (next.status, next.missing)),
        Some((Status::Exhausted, 7))
    );
    // OUT OF THE BOX: the count fell to nothing with nothing missing.
    let cleared = reassessed(&pending, &Assessment::default()).expect("recorded");
    assert_eq!(
        (cleared.status, cleared.unverified, cleared.diagnostics),
        (Status::Unverified, 0, 3),
        "a lost receipt is never made clean"
    );

    // TWO MINUTES OF ONE OPEN DAY OUT OF ORDER: the second lies before the
    // range the cursor already stands in. In order, both are read.
    let day = on(2026, 8, 24);
    let number = i64::from(day.days_from_epoch());
    assert_eq!(
        outside_session_count(
            &[at(day, 600, 0), at(day, 300, 0)],
            "NIFTY",
            number,
            number,
            None
        ),
        Err("stored minute lies outside the audited session shape".to_owned())
    );
    assert_eq!(
        outside_session_count(
            &[at(day, 300, 0), at(day, 600, 0)],
            "NIFTY",
            number,
            number,
            None
        ),
        Ok(1),
        "05:00 lies outside the session, 10:00 inside it"
    );
}

/// A plan journal of its own under `root`, holding `scans`, and their keys in
/// plan order.
fn plan_of(root: &Path, name: &str, scans: &[Record]) -> (Journal, Vec<[u8; 32]>) {
    let path = root.join(name);
    let _ = std::fs::remove_file(&path);
    let mut journal = Journal::open(&path).expect("a plan journal");
    for scan in scans {
        journal.append(scan.clone()).expect("a window");
    }
    (journal, scans.iter().map(|scan| scan.key).collect())
}

/// The site's shared attempt ledger, made again and holding `rows`.
fn ledger_of(site: &Site, rows: &[Record]) -> Journal {
    let path = super::root(site).join("attempts.bin");
    let _ = std::fs::remove_file(&path);
    let mut ledger = Journal::open(&path).expect("the shared ledger");
    for row in rows {
        ledger.append(row.clone()).expect("a ledger row");
    }
    ledger
}

/// A NIFTY minute-rung recovery body of `kind` over `from..=to`.
fn nifty(from: Day, to: Day, kind: &str) -> Record {
    record(canonical(
        "NIFTY",
        "1min",
        Window::new(from, to).expect("a window"),
        kind,
    ))
}

/// **A RECONCILE JUDGES ONLY THE PENDING DAYS ITS OWN WINDOWS HOLD, REFUSES A
/// ROW THAT IS NO DAY OF WORK BY NAME, AND A STOP ENDS IT AT THE NEXT
/// BOUNDARY** (BASE-COV-API repair 3, D-8661). `reconcile_pending` and
/// `execute`, driven directly over a mapped site's shared attempt ledger.
/// Extremes: a blocked day inside the window on a pass that is not explicit
/// (passed over, never read); the same day on an explicit pass (reassessed and
/// recorded in the ledger and the plan); a pending attempt spanning two days
/// (refused by name); a one-day attempt of kind `scan` (refused by name); a
/// pending day before the window in the plan's month (passed over); one after
/// it (passed over); a stop while a day waits (refused, nothing recorded); a
/// stop with nothing pending, through `execute` (refused at the first window
/// boundary, the window never assessed, the feed row published with nothing
/// done); and, at once, an attempt spanning two days that also lies before the
/// window (refused: the day scope is judged before the window), and a stop
/// with only another scope's day waiting (refused: the stop is read before the
/// scope); out of the box, the plan's own symbol and day asked at the daily
/// rung (another scope, passed over).
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one plan over one ledger: every row's fate and both stops"
)]
async fn a_reconcile_judges_only_its_own_days_and_a_stop_ends_it_at_the_next_boundary() {
    let (root, site) = mapped_site("recovery-cover-reconcile");
    let (d1, d2, d6, d7, d8, d9) = (
        on(2026, 7, 1),
        on(2026, 7, 2),
        on(2026, 7, 6),
        on(2026, 7, 7),
        on(2026, 7, 8),
        on(2026, 7, 9),
    );
    let scan = nifty(d6, d8, "scan");
    let (mut plan, keys) = plan_of(&root, "plan.bin", std::slice::from_ref(&scan));
    let refused = Err("shared attempt has invalid identity or non-day scope".to_owned());

    // A BLOCKED DAY INSIDE THE WINDOW, on a pass that is not explicit: never
    // read. On an explicit pass: reassessed, and recorded in both journals.
    let blocked = Record {
        status: Status::Blocked,
        ..nifty(d7, d7, "gap")
    };
    let mut ledger = ledger_of(&site, std::slice::from_ref(&blocked));
    assert_eq!(
        reconcile_pending(&site, &mut plan, &mut ledger, &keys, &None, false).await,
        Ok(())
    );
    assert_eq!(ledger.latest[&blocked.key], blocked, "passed over");
    assert!(!plan.latest.contains_key(&blocked.key), "nothing recorded");
    assert_eq!(
        reconcile_pending(&site, &mut plan, &mut ledger, &keys, &None, true).await,
        Ok(())
    );
    assert_eq!(ledger.latest[&blocked.key].status, Status::Unverified);
    assert_eq!(plan.latest[&blocked.key].status, Status::Unverified);
    drop(ledger);

    // AN ATTEMPT OF TWO DAYS AND ONE OF KIND `scan`, inside the plan's scope;
    // and, at once, two days that also lie before the window.
    for (row, what) in [
        (nifty(d6, d7, "gap"), "two days"),
        (nifty(d7, d7, "scan"), "a scan"),
        (nifty(d1, d2, "gap"), "two days before the window"),
    ] {
        let mut ledger = ledger_of(&site, std::slice::from_ref(&row));
        assert_eq!(
            reconcile_pending(&site, &mut plan, &mut ledger, &keys, &None, true).await,
            refused,
            "{what}"
        );
        assert_eq!(ledger.latest[&row.key], row, "{what}: nothing recorded");
    }

    // A PENDING DAY BEFORE THE WINDOW AND ONE AFTER IT, in the plan's month;
    // and, out of the box, the plan's own symbol and day at the daily rung.
    let daily = record(canonical(
        "NIFTY",
        "1day",
        Window::new(d7, d7).expect("a day"),
        "gap",
    ));
    let passed = [nifty(d1, d1, "gap"), nifty(d9, d9, "gap"), daily];
    let mut ledger = ledger_of(&site, &passed);
    assert_eq!(
        reconcile_pending(&site, &mut plan, &mut ledger, &keys, &None, true).await,
        Ok(())
    );
    for row in &passed {
        assert_eq!(&ledger.latest[&row.key], row, "passed over: {}", row.body);
    }
    drop(ledger);

    // A STOP WHILE A DAY WAITS: refused before it is judged; and, at once, a
    // stop with only another scope's day waiting, refused the same.
    *site.run.lock().expect("the run slot") = Some(Progress {
        stopping: true,
        ..Progress::claimed()
    });
    let foreign = record(canonical(
        "SBIN",
        "1min",
        Window::new(d7, d7).expect("a day"),
        "gap",
    ));
    for waiting in [nifty(d7, d7, "gap"), foreign] {
        let mut ledger = ledger_of(&site, std::slice::from_ref(&waiting));
        assert_eq!(
            reconcile_pending(&site, &mut plan, &mut ledger, &keys, &None, true).await,
            Err("stopped while reconciling interrupted requests".to_owned()),
            "{}",
            waiting.body
        );
        assert_eq!(ledger.latest[&waiting.key], waiting, "nothing recorded");
    }

    // A STOP WITH NOTHING PENDING, THROUGH `execute`: refused at the first
    // window boundary, the window never assessed, the feed row published.
    let mut ledger = ledger_of(&site, &[]);
    assert_eq!(
        execute(&site, &mut plan, &mut ledger, true).await,
        Err("stopped at a durable window boundary".to_owned())
    );
    assert_eq!(
        plan.latest[&scan.key], scan,
        "the window was never assessed"
    );
    let feeds = site
        .run
        .lock()
        .expect("the run slot")
        .as_ref()
        .map(|progress| progress.feeds.clone())
        .expect("the document");
    assert_eq!(feeds.len(), 1, "{feeds:?}");
    assert_eq!(
        (feeds[0].legs, feeds[0].legs_done, feeds[0].doing.as_str()),
        (1, 0, ""),
        "{feeds:?}"
    );
    drop((plan, ledger));
    let _ = std::fs::remove_dir_all(&root);
}

/// Stamps record `index` of `symbol`'s Zerodha month at rung `dir`, a month
/// of `n_valid` records, with `stamp`, and reseals that record's checksum
/// block in the month's `.crc` with the store's own geometry and CRC-32C
/// (`docs/02-store-format.md` §6): a month altered by hand, as no writer
/// leaves one, that still opens and reads.
fn restamped(
    root: &Path,
    symbol: &str,
    dir: &str,
    month: store::path::YearMonth,
    (index, n_valid): (u64, u64),
    stamp: i64,
) {
    use std::io::{Read as _, Seek as _, Write as _};
    let layout = store::layout::Layout::CURRENT;
    let bars = source_path(root, symbol, dir, month).expect("a path");
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&bars)
        .expect("the month");
    let at = layout.offset_of(index).expect("the record's offset");
    file.seek(std::io::SeekFrom::Start(at))
        .expect("to the record");
    file.write_all(&stamp.to_le_bytes())
        .expect("the record's new stamp");
    let block = index / store::format::RECORDS_PER_BLOCK;
    let (start, end) = layout
        .covered_byte_range(block, n_valid)
        .expect("the block's covered bytes");
    let mut covered = vec![0; usize::try_from(end - start).expect("one block")];
    file.seek(std::io::SeekFrom::Start(start))
        .expect("to the block");
    file.read_exact(&mut covered).expect("the block");
    let sum = store::block::seal(layout, n_valid, block, &covered).expect("the block's sum");
    let mut sidecar = std::fs::OpenOptions::new()
        .write(true)
        .open(bars.with_extension("crc"))
        .expect("the month's checksums");
    sidecar
        .seek(std::io::SeekFrom::Start(block * 4))
        .expect("to the block's entry");
    sidecar.write_all(&sum.to_le_bytes()).expect("the new sum");
    sidecar.sync_all().expect("the sum is durable");
    file.sync_all().expect("the record is durable");
}

/// The feed rows of the document in `site`'s run slot.
fn feed_rows(site: &Site) -> Vec<FeedReport> {
    site.run
        .lock()
        .expect("the run slot")
        .as_ref()
        .map(|progress| progress.feeds.clone())
        .expect("a document")
}

/// **AN EXECUTE READS A RETRY ALREADY SPENT AGAIN WITHOUT A REQUEST, NAMES THE
/// LIFECYCLE SNAPSHOT IT READ BY ITS PROOF, REFUSES AN AUDIT IT CANNOT FINISH,
/// AND AUDITS EVERY WINDOW WHEN ITS FEED ROW IS GONE** (BASE-COV-API repair 3,
/// D-8661). `execute`, `assess` and `load_lifecycle`, driven directly on a
/// runtime whose blocking pool is one thread, so a window's source read can be
/// held behind a task the test releases: the feed row is cleared while the
/// first window's read waits, deterministically, never by timing. Extremes: a
/// day whose retry the shared ledger holds spent (appended as it stands, no
/// attempt charged); a blocked day on a pass that is not explicit (the same);
/// the window assessed again after its retries (exhausted, fifteen minutes
/// missing); a holed day held in the minutes with no daily bar (missing and
/// owed a retry); a lifecycle master for the plan's last day (named by its
/// day, digest and source); a plan ending on another day (unverified, named);
/// an audit past its 4,096 runs (refused by name); a month whose middle bar is
/// stamped in another month, resealed under the store's own checksum (read as
/// written before, refused by name after); a feed row cleared while a
/// window is read (that window's count not published); the next window's
/// doing (not published); and, at once, a spent day and a blocked day in one
/// window (both answered without a request, the window read again once), and
/// the feed row gone while both windows are still audited and summed; out of
/// the box, a run slot holding no document at all (every window audited,
/// nothing published).
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one site, every read `execute` makes beside the others it is read against"
)]
fn an_execute_reads_a_spent_retry_again_names_its_snapshot_and_refuses_an_audit_it_cannot_finish() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .expect("a runtime of one blocking thread");
    runtime.block_on(async {
        let (root, site) = mapped_site("recovery-cover-execute");
        let (d1, d2, d6, d7) = (
            on(2026, 7, 1),
            on(2026, 7, 2),
            on(2026, 7, 6),
            on(2026, 7, 7),
        );
        let july = d1.year_month().expect("a month");
        // d1: ten minutes missing, its daily bar held; d2: five missing and
        // no daily bar.
        let ten = at(d1, 600, 0)..at(d1, 610, 0);
        let five = at(d2, 600, 0)..at(d2, 605, 0);
        let mut minutes: Vec<i64> = session_of(d1)
            .into_iter()
            .filter(|stamp| !ten.contains(stamp))
            .collect();
        minutes.extend(
            session_of(d2)
                .into_iter()
                .filter(|stamp| !five.contains(stamp)),
        );
        source_month(&root, "NIFTY", "1min", july, &minutes);
        source_month(&root, "NIFTY", "1day", july, &[at(d1, 0, 0)]);
        let scan = nifty(d1, d2, "scan");
        let found = assess(&site, &scan.body, &None).await.expect("assessed");
        assert_eq!(
            (found.missing, found.retry_days.clone()),
            (15, vec![d1, d2]),
            "the day held in the minutes alone is owed too"
        );

        // A SPENT DAY AND A BLOCKED DAY IN ONE WINDOW, on a pass that is not
        // explicit: both answered as they stand, the window read again once.
        let spent = Record {
            status: Status::Exhausted,
            attempts: ATTEMPT_LIMIT,
            ..nifty(d1, d1, "gap")
        };
        let blocked = Record {
            status: Status::Blocked,
            attempts: 1,
            http_status: 503,
            ..nifty(d2, d2, "gap")
        };
        let (mut plan, _) = plan_of(&root, "plan.bin", std::slice::from_ref(&scan));
        let mut ledger = ledger_of(&site, &[spent.clone(), blocked.clone()]);
        let summary = execute(&site, &mut plan, &mut ledger, false)
            .await
            .expect("finished");
        assert!(
            summary.contains(
                "0 stored windows verified against measured schedule; 0 not applicable; \
                 0 unverified; 1 missing/blocked"
            ),
            "{summary}"
        );
        let window = &plan.latest[&scan.key];
        assert_eq!((window.status, window.missing), (Status::Exhausted, 15));
        for row in [&spent, &blocked] {
            assert_eq!(&plan.latest[&row.key], row, "appended as it stands");
            assert_eq!(&ledger.latest[&row.key], row, "no attempt charged");
        }
        drop((plan, ledger));

        // A LIFECYCLE MASTER FOR THE PLAN'S LAST DAY, named by its proof; a
        // plan ending on another day finds none.
        let isin = brutex_core::universe::nse_isin("RELIANCE").expect("an F&O share");
        let csv = format!(
            "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn,ListgDt,RmvlDt,RadmssnDt\n\
             901,RELIANCE,EQ,{},1,{},0,0\n",
            isin.as_str(),
            since_1980(on(2001, 1, 2))
        );
        pull::cash_session_cache::install(
            &root.join("session-masters"),
            master_day(),
            &stored_gzip(csv.as_bytes()),
        )
        .expect("the master lands");
        let share = |to: Day| {
            record(canonical(
                "RELIANCE",
                "1min",
                Window::new(on(2026, 8, 24), to).expect("a window"),
                "scan",
            ))
        };
        let (mut shares, keys) = plan_of(&root, "shares.bin", &[share(master_day())]);
        let loaded = load_lifecycle(&site, &mut shares, &keys)
            .expect("read")
            .expect("the master");
        let proof = loaded.provenance();
        assert_eq!(proof.master_day, master_day());
        let said = format!(
            "lifecycle snapshot ONLY: {} sha256={} source={}",
            proof.master_day,
            hex(proof.sha256),
            proof.source_url
        );
        assert!(
            shares
                .latest
                .values()
                .any(|row| row.body == said && row.status == Status::Unverified),
            "{said}"
        );
        assert!(said.starts_with("lifecycle snapshot ONLY: 2026-08-27 sha256="));
        let (mut earlier, keys) = plan_of(&root, "earlier.bin", &[share(on(2026, 8, 26))]);
        assert!(
            load_lifecycle(&site, &mut earlier, &keys)
                .expect("read")
                .is_none()
        );
        assert!(
            earlier
                .latest
                .values()
                .any(|row| row.body.starts_with("lifecycle UNVERIFIED: ")),
            "no master for 2026-08-26"
        );
        drop((shares, earlier));

        // AN AUDIT PAST ITS 4,096 RUNS: every other minute of six open days.
        let august = on(2026, 8, 3).year_month().expect("a month");
        let crowded: Vec<i64> = [3, 4, 5, 6, 7, 10]
            .into_iter()
            .flat_map(|date| (0..720).map(move |half| at(on(2026, 8, date), 2 * half, 0)))
            .collect();
        source_month(&root, "NIFTY", "1min", august, &crowded);
        let week = nifty(on(2026, 8, 3), on(2026, 8, 10), "scan");
        assert_eq!(
            assess(&site, &week.body, &None)
                .await
                .map(|found| found.held),
            Err("source gap audit is truncated or has invalid timestamps".to_owned())
        );

        // A MONTH WHOSE MIDDLE BAR IS STAMPED IN ANOTHER MONTH, resealed under
        // the store's own checksum, so the month opens and the bar reads.
        let september = on(2026, 9, 1).year_month().expect("a month");
        let days = [on(2026, 9, 1), on(2026, 9, 2), on(2026, 9, 3)];
        source_month(
            &root,
            "NIFTY",
            "1day",
            september,
            &days.map(|day| at(day, 0, 0)),
        );
        assert_eq!(
            read_stamps(&root, "NIFTY", "1day", september).map(|stamps| stamps.len()),
            Ok(3),
            "the month as written reads"
        );
        restamped(
            &root,
            "NIFTY",
            "1day",
            september,
            (1, 3),
            at(on(2026, 10, 1), 0, 0),
        );
        assert_eq!(
            read_stamps(&root, "NIFTY", "1day", september),
            Err("source timestamp belongs to a different instrument-month window".to_owned())
        );

        // THE FEED ROW CLEARED WHILE THE FIRST WINDOW'S READ WAITS behind the
        // runtime's one blocking thread; and, at once, both windows audited.
        let two = [nifty(d6, d6, "scan"), nifty(d7, d7, "scan")];
        let (mut both, _) = plan_of(&root, "both.bin", &two);
        let mut empty = ledger_of(&site, &[]);
        *site.run.lock().expect("the run slot") = Some(Progress::claimed());
        let (release, held) = std::sync::mpsc::channel::<()>();
        let occupier = tokio::task::spawn_blocking(move || held.recv());
        let mut run = Box::pin(execute(&site, &mut both, &mut empty, false));
        let first = std::future::poll_fn(|cx| std::task::Poll::Ready(run.as_mut().poll(cx))).await;
        assert!(first.is_pending(), "the first window's read waits");
        let doing = feed_rows(&site)
            .first()
            .map(|feed| feed.doing.clone())
            .expect("the feed row");
        assert!(
            doing.starts_with("Audit NIFTY · 1min · 2026-07-06..2026-07-06"),
            "{doing}"
        );
        site.run
            .lock()
            .expect("the run slot")
            .as_mut()
            .expect("the document")
            .feeds
            .clear();
        release.send(()).expect("the occupier waits");
        assert_eq!(occupier.await.expect("joined"), Ok(()));
        let summary = run.await.expect("finished");
        assert!(
            summary.contains("0 not applicable; 2 unverified; 0 missing/blocked"),
            "{summary}"
        );
        assert!(
            feed_rows(&site).is_empty(),
            "nothing published after the clear"
        );
        for window in &two {
            assert_eq!(both.latest[&window.key].status, Status::Unverified);
        }

        // OUT OF THE BOX: a run slot holding no document at all.
        *site.run.lock().expect("the run slot") = None;
        let (mut alone, _) = plan_of(&root, "alone.bin", std::slice::from_ref(&two[0]));
        let summary = execute(&site, &mut alone, &mut empty, false)
            .await
            .expect("finished");
        assert!(summary.contains("1 unverified"), "{summary}");
        assert!(site.run.lock().expect("the run slot").is_none());
        drop((both, alone, empty));
        let _ = std::fs::remove_dir_all(&root);
    });
}

/// One NIFTY minute window's plan on `day`: its units and its identity.
fn one_window(day: Day) -> (Vec<Record>, [u8; 32]) {
    let units = vec![nifty(day, day, "scan")];
    let id = scope_identity(&units);
    (units, id)
}

/// Where plan `id`'s STOP history is kept on `site`.
fn stop_path(site: &Site, id: [u8; 32]) -> PathBuf {
    super::root(site).join(format!("{}.stop.bin", hex(id)))
}

/// The sentence `site`'s run ends with, waited for. On this current-thread
/// runtime a driver just spawned has not run yet, so the first look finds the
/// run still going.
async fn ended(site: &Loaded) -> String {
    tokio::time::timeout(std::time::Duration::from_mins(1), async {
        while site
            .run
            .lock()
            .expect("the run slot")
            .as_ref()
            .is_some_and(Progress::running)
        {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the run ends");
    site.run
        .lock()
        .expect("the run slot")
        .as_ref()
        .and_then(|progress| progress.finished.clone())
        .expect("a finished run")
}

/// **AN ACTIVATION STOPPED OR REFUSED NAMES A STOP IT COULD NOT PERSIST, A
/// START NAMING NO RECOVERABLE PLAN IS REFUSED BEFORE ANY CLAIM, AND A PLAN
/// LEFT IN FLIGHT IS DRIVEN AT START** (BASE-COV-API repair 3, D-8661).
/// `activate_durable` driven directly with the run slot claimed as `start`
/// claims it; `start` over plans `plan` refuses; `at_start` and `resume` over
/// a plan sealed in flight whose shared attempt ledger is gone, so the real
/// driver runs and ends blocked with no vendor reached. Extremes: a stop
/// requested during a sound activation with no plan active (refused, the run
/// blocked, the seed durable and its pointer published, no STOP written); the
/// same with the plan active (its STOP persisted, the plan left idle); a seed
/// refused by a folder at the first pointer's staging name with the plan
/// active (the seed's reason alone, the STOP persisted); a start naming
/// FINNIFTY and one at the five-minute rung (each `400`, refused by the plan's
/// own rule, nothing claimed); a grounded start over the plan in flight (named, nothing
/// claimed); a start that drives it (the driver ends blocked, naming the
/// missing ledger); a resume afterwards (driven again, ended the same); and, at
/// once, a stop requested while the plan's STOP history holds another plan's
/// row (both named in one reason), and a seed refused while the STOP path is a
/// folder (both named); out of the box, a second start before the first one's
/// driver has run (refused by the run slot the first took).
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "every way an activation ends, beside the starts that drive a plan left in flight"
)]
async fn an_activation_stopped_or_refused_names_a_stop_it_could_not_persist_and_a_plan_in_flight_is_driven()
 {
    let day = on(2026, 7, 1);
    let (units, id) = one_window(day);
    let finished = |site: &Loaded| {
        site.run
            .lock()
            .expect("the run slot")
            .as_ref()
            .and_then(|progress| progress.finished.clone())
    };
    let claimed = |site: &Loaded, stopping: bool| {
        *site.run.lock().expect("the run slot") = Some(Progress {
            stopping,
            ..Progress::claimed()
        });
    };
    let stopped = "stopped during durable activation";

    // A STOP DURING A SOUND ACTIVATION, NO PLAN ACTIVE.
    let (root, site) = refused_site("recovery-cover-activation-stop");
    claimed(&site, true);
    assert_eq!(
        activate_durable(Loaded::clone(&site), id, Some(units.clone())).await,
        Err(stopped.to_owned())
    );
    assert_eq!(
        finished(&site),
        Some(format!("Recovery BLOCKED: {stopped}"))
    );
    let plan = crate::recovery_journal::snapshot(&plan_path(&site, id)).expect("the plan");
    assert_eq!(plan.latest[&CONTROL].status, Status::InFlight, "seeded");
    assert!(active_path(&site).is_file(), "its pointer published");
    assert!(!stop_path(&site, id).exists(), "no plan active: no STOP");
    let _ = std::fs::remove_dir_all(&root);

    // THE SAME WITH THE PLAN ACTIVE: its STOP persisted, the plan left idle.
    let (root, site) = refused_site("recovery-cover-activation-active");
    claimed(&site, true);
    crate::recovery_control::activate(&site, id);
    assert_eq!(
        activate_durable(Loaded::clone(&site), id, Some(units.clone())).await,
        Err(stopped.to_owned())
    );
    assert_eq!(crate::recovery_control::is_stopped(&site, id), Ok(true));
    assert!(
        site.recovery_active
            .lock()
            .expect("the active plan")
            .is_none()
    );
    let _ = std::fs::remove_dir_all(&root);

    // AT ONCE: A STOP WHOSE HISTORY HOLDS ANOTHER PLAN'S ROW.
    let (root, site) = refused_site("recovery-cover-activation-foreign-stop");
    claimed(&site, true);
    crate::recovery_control::activate(&site, id);
    Journal::open(&stop_path(&site, id))
        .expect("a STOP history")
        .append(Record {
            key: [9; 32],
            status: Status::Blocked,
            ..record(format!("recovery-stop-v1:{}", hex([9; 32])))
        })
        .expect("another plan's row");
    let why = format!("{stopped}; stop persistence: recovery STOP journal plan identity mismatch");
    assert_eq!(
        activate_durable(Loaded::clone(&site), id, Some(units.clone())).await,
        Err(why.clone())
    );
    assert_eq!(finished(&site), Some(format!("Recovery BLOCKED: {why}")));
    let _ = std::fs::remove_dir_all(&root);

    // A SEED REFUSED WITH THE PLAN ACTIVE: the seed's reason alone, the STOP
    // persisted; and, at once, its STOP path a folder: both named.
    for (name, stop_folder) in [
        ("recovery-cover-activation-seed", false),
        ("recovery-cover-activation-seed-folder", true),
    ] {
        let (root, site) = refused_site(name);
        claimed(&site, false);
        crate::recovery_control::activate(&site, id);
        std::fs::create_dir_all(super::root(&site).join("active.bin.first").join("held"))
            .expect("a folder at the staging name");
        if stop_folder {
            std::fs::create_dir_all(stop_path(&site, id)).expect("a folder at the STOP path");
        }
        let why = activate_durable(Loaded::clone(&site), id, Some(units.clone()))
            .await
            .expect_err("the seed is refused");
        assert_eq!(why.contains("; stop persistence: "), stop_folder, "{why}");
        assert!(!why.starts_with(stopped), "{why}");
        assert_eq!(finished(&site), Some(format!("Recovery BLOCKED: {why}")));
        assert_eq!(
            crate::recovery_control::is_stopped(&site, id).is_ok_and(|stop| stop),
            !stop_folder,
            "{name}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // A START NAMING FINNIFTY, AND ONE AT THE FIVE-MINUTE RUNG: legs the form
    // reader takes and the plan refuses by its own rule.
    let (root, site) = refused_site("recovery-cover-activation-refused-plan");
    for body in [
        legs_of(&[("FINNIFTY", "1min", day, day)]),
        legs_of(&[("NIFTY", "5min", day, day)]),
    ] {
        let (status, _, said) = start(State(Loaded::clone(&site)), body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{said}");
        let said: serde_json::Value = serde_json::from_str(&said).expect("JSON");
        assert_eq!(said["started"], false);
        assert!(
            said["why"]
                .as_str()
                .is_some_and(|text| text.starts_with("recovery requires explicit F&O spot members")),
            "{said}"
        );
    }
    assert!(
        site.run.lock().expect("the run slot").is_none(),
        "nothing claimed"
    );
    let _ = std::fs::remove_dir_all(&root);

    // A PLAN SEALED IN FLIGHT, POINTED AT, ITS SHARED LEDGER GONE.
    let (root, site) = refused_site("recovery-cover-at-start");
    {
        let mut plan = Journal::open(&plan_path(&site, id)).expect("a plan journal");
        plan.append(units[0].clone()).expect("its window");
        let mut control = record(CONTROL_BODY.to_owned());
        control.key = CONTROL;
        control.status = Status::InFlight;
        plan.append(control).expect("its seal, in flight");
        Journal::open(&active_path(&site))
            .expect("the pointer journal")
            .append(pointer(id))
            .expect("its pointer");
    }
    assert_eq!(
        at_start(&site, true),
        Ok(AtStart::Grounded {
            plan: Some(hex(id))
        })
    );
    assert!(site.run.lock().expect("the run slot").is_none(), "grounded");
    assert_eq!(at_start(&site, false), Ok(AtStart::Driving));
    // OUT OF THE BOX: asked again before its driver has run.
    assert_eq!(
        at_start(&site, false),
        Err("a pull already owns the run slot; recovery was not started twice".to_owned())
    );
    let first = ended(&site).await;
    assert!(
        first.starts_with(
            "Recovery BLOCKED: shared recovery attempt history is unavailable; \
             budgets cannot be reset"
        ),
        "{first}"
    );
    assert_eq!(resume(Loaded::clone(&site)), Ok(true), "still in flight");
    assert_eq!(ended(&site).await, first);
    let _ = std::fs::remove_dir_all(&root);
}

/// **A SEED REFUSES A STORE IT CANNOT SHAPE BY THE STORE'S OWN ERROR, AND
/// MAKES NOTHING** (BASE-COV-API repair 3, D-8661). `seeded` over stores whose
/// folders are links to nowhere, regular files or closed to writing, run where
/// the mode bits bind: a child of this test binary as uid 65534 when this
/// process is root, and as itself otherwise (D-0995). Extremes: `audit` a link
/// to nowhere (refused: it exists and is no folder; its target not made); the
/// recovery folder a link to nowhere (the same); a store root closed to
/// writing with no `audit` (refused by the permission error, nothing made);
/// `audit` closed to writing with no recovery folder (the same); the recovery
/// folder closed to writing with no plan (refused making the plan); a store
/// root that is a file (refused: no replacement root); `audit` a regular file
/// (refused before any folder is made); a sound store (seeded: its seal in
/// flight, its pointer published); and, at once, a store root closed to
/// writing whose `audit` is a regular file (refused by the file, nothing
/// changed), and `audit` a link to a folder closed to writing (refused at the
/// recovery folder by the permission error); out of the box, `audit` a link to
/// a folder elsewhere (seeded there).
#[test]
#[cfg(unix)]
fn a_seed_refuses_a_store_it_cannot_shape_by_the_stores_own_error() {
    crate::isolated::where_permission_binds(
        "recovery::cover_tests::a_seed_refuses_a_store_it_cannot_shape_by_the_stores_own_error",
        a_seed_refuses_a_store_it_cannot_shape_body,
    );
}

/// The test above, run where the mode bits bind (D-0995).
#[cfg(unix)]
#[expect(
    clippy::too_many_lines,
    reason = "every store shape a seed meets, each beside the sound store"
)]
fn a_seed_refuses_a_store_it_cannot_shape_body() {
    use std::os::unix::fs::{PermissionsExt as _, symlink};
    let (units, id) = one_window(on(2026, 7, 1));
    let mode = |path: &Path, bits: u32| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(bits)).expect("a mode");
    };
    let fresh = |name: &str| {
        let root = store_root(name);
        let site = Loaded::new(Site::load(&root.join("missing-masters"), &root));
        (root, site)
    };
    let seed = |site: &Loaded| seeded(site, id, Some(units.clone())).map(|_| ());
    let denied = |why: &str| why.contains("ermission denied");

    // `audit` A LINK TO NOWHERE, and the recovery folder one.
    for inner in [false, true] {
        let (root, site) = fresh(if inner {
            "recovery-cover-shape-inner-link"
        } else {
            "recovery-cover-shape-link"
        });
        let link = if inner {
            std::fs::create_dir(root.join("audit")).expect("an audit folder");
            root.join("audit/recovery-v1")
        } else {
            root.join("audit")
        };
        symlink(root.join("nowhere"), &link).expect("a link to nowhere");
        let why = seed(&site).expect_err("a link to nowhere is no folder");
        assert!(why.contains("exists"), "{why}");
        assert!(!root.join("nowhere").exists(), "its target not made");
        let _ = std::fs::remove_dir_all(&root);
    }

    // CLOSED TO WRITING: the store root, then `audit`, then the recovery
    // folder; each refused by the permission error, the next thing not made.
    for (name, closed, absent) in [
        ("recovery-cover-shape-root", "", "audit"),
        ("recovery-cover-shape-audit", "audit", "audit/recovery-v1"),
        (
            "recovery-cover-shape-recovery",
            "audit/recovery-v1",
            "audit/recovery-v1/active.bin",
        ),
    ] {
        let (root, site) = fresh(name);
        std::fs::create_dir_all(root.join(closed)).expect("the folders above");
        mode(&root.join(closed), 0o555);
        let why = seed(&site);
        mode(&root.join(closed), 0o755);
        let why = why.expect_err("closed to writing");
        assert!(denied(&why), "{name}: {why}");
        assert!(!root.join(absent).exists(), "{name}: {absent} not made");
        assert!(!plan_path(&site, id).exists(), "{name}: no plan");
        let _ = std::fs::remove_dir_all(&root);
    }

    // A STORE ROOT THAT IS A FILE.
    let file = crate::scratch::path("recovery-cover-shape-file");
    let _ = std::fs::remove_dir_all(&file);
    std::fs::write(&file, b"no store").expect("a file where the store goes");
    let site = Loaded::new(Site::load(&file.join("missing-masters"), &file));
    assert_eq!(
        seed(&site),
        Err(
            "configured source store is unavailable; recovery creates no replacement root"
                .to_owned()
        )
    );
    let _ = std::fs::remove_file(&file);

    // `audit` A REGULAR FILE; and, at once, under a store root closed to
    // writing: refused by the file before any folder is made.
    for closed in [false, true] {
        let (root, site) = fresh("recovery-cover-shape-audit-file");
        std::fs::write(root.join("audit"), b"no folder").expect("a file at audit");
        if closed {
            mode(&root, 0o555);
        }
        let why = seed(&site);
        mode(&root, 0o755);
        let why = why.expect_err("a file is no folder");
        assert!(why.contains("Not a directory"), "{why}");
        assert!(root.join("audit").is_file(), "left as it was");
        let _ = std::fs::remove_dir_all(&root);
    }

    // AT ONCE: `audit` A LINK TO A FOLDER CLOSED TO WRITING; and, out of the
    // box, the same link to a folder open to writing: seeded there.
    for closed in [true, false] {
        let (root, site) = fresh("recovery-cover-shape-linked");
        let elsewhere = root.join("elsewhere");
        std::fs::create_dir(&elsewhere).expect("a folder elsewhere");
        symlink(&elsewhere, root.join("audit")).expect("a link to it");
        if closed {
            mode(&elsewhere, 0o555);
        }
        let seeded = seed(&site);
        mode(&elsewhere, 0o755);
        if closed {
            let why = seeded.expect_err("closed to writing");
            assert!(denied(&why), "{why}");
            assert!(!elsewhere.join("recovery-v1").exists());
        } else {
            assert_eq!(seeded, Ok(()));
            assert!(
                elsewhere
                    .join("recovery-v1")
                    .join(format!("{}.bin", hex(id)))
                    .is_file(),
                "the plan lands where the link points"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    // A SOUND STORE.
    let (root, site) = fresh("recovery-cover-shape-sound");
    assert_eq!(seed(&site), Ok(()));
    let plan = crate::recovery_journal::snapshot(&plan_path(&site, id)).expect("the plan");
    assert_eq!(plan.latest[&CONTROL].status, Status::InFlight);
    assert!(active_path(&site).is_file(), "its pointer published");
    let _ = std::fs::remove_dir_all(&root);
}

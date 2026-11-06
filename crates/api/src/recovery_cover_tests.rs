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
#[expect(
    clippy::too_many_lines,
    reason = "one store, every day shape an assessment reads, each beside the others"
)]
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
#[expect(
    clippy::too_many_lines,
    reason = "every state the page names, then each pointer it may be handed"
)]
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
        view.records
            .iter()
            .any(|row| row.status == Status::Verified
                && server::param(&row.body, "member") == "NIFTY"),
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

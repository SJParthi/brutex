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

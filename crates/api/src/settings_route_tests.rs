//! Tests of `/settings.json` and `POST /settings` (R3-05). Every store is a
//! fresh temporary directory; no value names a folder of the operator's.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use super::*;
use std::sync::Arc;

/// A new, empty store root.
fn root(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "brutex-settings-route-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// The value row of `key` in an answer.
fn value_of<'a>(answer: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    answer["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["key"] == key)
        .unwrap_or_else(|| panic!("no {key} in {answer}"))
}

/// One HTTP exchange with a loopback server: the status line and body.
async fn http(address: std::net::SocketAddr, method: &str, path: &str, body: &str) -> String {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nOrigin: http://{address}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
    socket.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).await.unwrap();
    response
}

/// The JSON body of a response.
fn body_of(response: &str) -> serde_json::Value {
    let at = response.find("\r\n\r\n").expect("a body");
    serde_json::from_str(&response[at + 4..]).unwrap_or_else(|why| panic!("{why}: {response}"))
}

/// **A SETTING CHANGES WITHOUT A RESTART AND IS AUDITED.**
///
/// Through the production router on a loopback port: the window's last day
/// is the built-in default, a `POST /settings` saves generation 1, the very
/// next `GET` serves it with its source named, `/audit`'s journal holds the
/// `POST /settings` row, and `?history=` shows the change as before → after.
#[tokio::test]
async fn a_setting_changes_without_a_restart_and_is_audited() {
    let _apart = crate::detail::apart_from_slot_owners().await;
    let root = root("restart");
    let site = Arc::new(crate::server::Site::load(&root.join("masters"), &root));
    let front = Arc::new(crate::assets::Assets::new(&root.join("web")));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = crate::server::audited_router_serving(site, front, address);
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });

    let before = body_of(&http(address, "GET", "/settings.json", "").await);
    assert_eq!(before["generation"], 0, "{before}");
    let window_to = value_of(&before, "window.to");
    assert_eq!(window_to["value"], "2026-09-30", "{before}");
    assert_eq!(window_to["origin"], "default", "{before}");

    let change = r#"{"expected":0,"changed_by":"operator","reason":"October is held now","set":{"window.to":"2026-10-31","open.instrument":"BANKNIFTY"}}"#;
    let saved = http(address, "POST", "/settings", change).await;
    assert!(saved.starts_with("HTTP/1.1 200"), "{saved}");
    assert!(saved.contains("x-brutex-request-audit:"), "{saved}");

    let after = body_of(&http(address, "GET", "/settings.json?history=5", "").await);
    assert_eq!(after["generation"], 1, "{after}");
    assert_eq!(value_of(&after, "window.to")["value"], "2026-10-31");
    assert_eq!(
        value_of(&after, "window.to")["source"],
        "settings generation 1"
    );
    assert_eq!(value_of(&after, "open.instrument")["value"], "BANKNIFTY");
    assert_eq!(after["last_change"]["reason"], "October is held now");
    assert_eq!(after["last_change"]["changed_by"], "operator");
    let diff = &after["history"][0]["diff"];
    assert_eq!(
        diff,
        &serde_json::json!([
            {"key": "window.to", "before": null, "after": "2026-10-31"},
            {"key": "open.instrument", "before": null, "after": "BANKNIFTY"},
        ]),
        "{after}"
    );

    // /feeds.json SERVES THE SAVED WINDOW ON ITS NEXT REQUEST.
    let feeds = http(address, "GET", "/feeds.json", "").await;
    let feeds = body_of(&feeds);
    let zerodha = feeds
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["wire"] == "zerodha")
        .unwrap();
    assert_eq!(zerodha["window"]["to"], "2026-10-31", "{zerodha}");

    // THE AUDIT JOURNAL HOLDS THE CHANGE.
    let rows = cli::operation_audit::page(&root, None, 32).unwrap();
    assert!(
        rows.iter()
            .any(|row| row.label == "POST /settings" && row.response_status == 200),
        "{rows:?}"
    );
    // A STALE REPEAT IS REFUSED 409, AND AUDITED TOO.
    let stale = http(address, "POST", "/settings", change).await;
    assert!(stale.starts_with("HTTP/1.1 409"), "{stale}");
    assert_eq!(body_of(&stale)["code"], "stale");
    let rows = cli::operation_audit::page(&root, None, 32).unwrap();
    assert!(
        rows.iter()
            .any(|row| row.label == "POST /settings" && row.response_status == 409),
        "{rows:?}"
    );

    stop.send(()).unwrap();
    server.await.unwrap();
}

/// **A BAD PATH IS REFUSED BEFORE IT IS SAVED**, on the route: 400 naming the
/// key, and no generation file is written.
#[test]
fn a_bad_path_is_refused_before_it_is_saved() {
    let root = root("badpath");
    for bad in ["relative/folder", "/no/such/folder/anywhere", "/tmp/../etc"] {
        let body = format!(
            r#"{{"expected":0,"changed_by":"operator","reason":"r","set":{{"source.gdfl":{}}}}}"#,
            serde_json::Value::from(bad)
        );
        let (status, answer) = change(&root, &body);
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}: {answer}");
        assert_eq!(answer["key"], "source.gdfl", "{answer}");
        assert_eq!(answer["saved"], false);
    }
    assert!(
        !settings::dir(&root)
            .join(format!("{:020}.settings", 1))
            .exists()
    );
    assert_eq!(super::answer(&root, 0)["generation"], 0);
}

/// **GARBAGE PARAMETERS AND BODIES ARE REFUSED BY NAME**, and an offered feed
/// is the server's own check.
#[tokio::test]
async fn garbage_is_refused_by_name() {
    let root = root("garbage");
    for body in [
        "",
        "not json",
        "[]",
        r#"{"expected":0}"#,
        r#"{"changed_by":"a","reason":"b","set":{"open.order":"oldest"}}"#,
        r#"{"expected":-1,"changed_by":"a","reason":"b","set":{"open.order":"oldest"}}"#,
        r#"{"expected":0,"changed_by":"a","reason":"b","set":{"open.colour":"red"}}"#,
        r#"{"expected":0,"changed_by":"a","reason":"b","set":{"open.order":7}}"#,
        r#"{"expected":0,"changed_by":"a","reason":"b","unset":["nope"]}"#,
        r#"{"expected":0,"changed_by":"a","reason":"b","set":{"open.order":"oldest"},"extra":1}"#,
    ] {
        let (status, answer) = change(&root, body);
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {answer}");
        assert_eq!(answer["code"], "bad_body", "{body}: {answer}");
    }
    // A FEED THIS BUILD DOES NOT OFFER.
    let (status, answer) = change(
        &root,
        r#"{"expected":0,"changed_by":"a","reason":"b","set":{"open.feed":"dhan"}}"#,
    );
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    assert_eq!(answer["key"], "open.feed");
    // A CHANGE THAT CHANGES NOTHING.
    let set = r#"{"expected":0,"changed_by":"a","reason":"b","set":{"open.order":"oldest"}}"#;
    assert_eq!(change(&root, set).0, StatusCode::OK);
    let again = r#"{"expected":1,"changed_by":"a","reason":"b","set":{"open.order":"oldest"}}"#;
    let (status, answer) = change(&root, again);
    assert_eq!(
        (status, answer["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("unchanged"))
    );

    // GARBAGE QUERY PARAMETERS ON THE READ.
    let dir = crate::server::tests::masters("settings-garbage", None, None);
    let site = Loaded::new(crate::server::Site::load(&dir, &root));
    for query in [
        "history=51",
        "history=-1",
        "history=x",
        "colour=red",
        "history",
        // DIGITS ONLY (repair 3): a sign and a percent-escape are refused
        // alike, and so is an empty value.
        "history=+5",
        "history=%35",
        "history=",
    ] {
        let (status, _, body) = settings_json(
            axum::extract::State(site.clone()),
            axum::extract::RawQuery(Some(query.to_owned())),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {body}");
    }
    let (status, _, body) = settings_json(
        axum::extract::State(site),
        axum::extract::RawQuery(Some("history=50".to_owned())),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// **THE CEILINGS ARE SERVED ONCE**: each by one name, equal to the constant
/// that enforces it; the lists are the enums'. The browser's own copies in
/// `bars-plan.js` are held to these constants by a web test.
#[test]
fn the_ceilings_are_served_once() {
    let root = root("ceilings");
    let answer = super::answer(&root, 0);
    let ceilings = answer["ceilings"].as_array().unwrap();
    let mut names: Vec<&str> = ceilings
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count, "a ceiling served twice: {ceilings:?}");
    let value = |name: &str| {
        ceilings
            .iter()
            .find(|c| c["name"] == name)
            .and_then(|c| c["value"].as_u64())
            .unwrap_or_else(|| panic!("no {name}"))
    };
    assert_eq!(value("max_scan_bars"), 24_000_000);
    assert_eq!(value("max_scan_bars"), crate::bars::MAX_SCAN_BARS);
    assert_eq!(value("max_sort_depth"), 100_000);
    assert_eq!(value("max_series"), 1_024);
    assert_eq!(value("max_window_months"), 240);
    assert_eq!(value("browser_sort_budget"), 20_000);
    assert_eq!(value("export_cap"), 20_000);

    let lists = &answer["lists"];
    assert_eq!(
        lists["segments"],
        serde_json::json!(["INDEX", "CASH", "FNO"])
    );
    assert_eq!(lists["kinds"], serde_json::json!(["rest", "folder"]));
    let mut offered: Vec<&str> = lists["feeds"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["offered"] == true)
        .map(|f| f["wire"].as_str().unwrap())
        .collect();
    // ONLY ZERODHA AND GDFL ARE OFFERED (operator's answer 7): Dhan, Groww
    // and TrueData are listed, and not offered.
    offered.sort_unstable();
    assert_eq!(offered, ["gdfl", "zerodha"]);
    assert!(
        lists["timeframes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tf| tf == "1s")
    );

    // THE BROWSER'S COPIES ARE CHECKED ON THE BROWSER'S SIDE
    // (`web/tests/settings-ceilings.test.js`): gate 1e runs this suite with
    // `web/` DETACHED, so a Rust test may not open a page's file.
    let served = |constant: usize| u64::try_from(constant).unwrap();
    assert_eq!(
        value("max_window_limit"),
        served(crate::bars::MAX_WINDOW_LIMIT)
    );
    assert_eq!(value("max_series"), served(crate::bars::MAX_SERIES));
    assert_eq!(
        value("max_window_months"),
        served(crate::bars::MAX_WINDOW_MONTHS)
    );
    assert_eq!(value("max_sort_depth"), served(crate::bars::MAX_SORT_DEPTH));
    assert_eq!(value("browser_sort_budget"), served(BROWSER_SORT_BUDGET));
    assert_eq!(value("export_cap"), served(EXPORT_CAP));
}

/// **A DAMAGED GENERATION IS NAMED, AND THE ANSWER SAYS IT IS DEGRADED**: the
/// newest generation that reads stays in force.
#[test]
fn a_damaged_generation_is_served_as_degraded_with_its_fault() {
    let root = root("damaged");
    let set = |expected: u64, order: &str| {
        format!(
            r#"{{"expected":{expected},"changed_by":"a","reason":"b","set":{{"open.order":"{order}"}}}}"#
        )
    };
    assert_eq!(change(&root, &set(0, "oldest")).0, StatusCode::OK);
    // A SECOND PROCESS'S GENERATION 2, DAMAGED ON ITS WAY TO DISK.
    std::fs::write(
        settings::path_of(&root, 2),
        b"brutex-settings 1\ngeneration 2\n",
    )
    .unwrap();
    let answer = super::answer(&root, 3);
    assert_eq!(answer["generation"], 1, "{answer}");
    assert_eq!(answer["highest"], 2, "{answer}");
    assert_eq!(answer["degraded"], true, "{answer}");
    assert_eq!(answer["faults"][0]["generation"], 2, "{answer}");
    assert_eq!(value_of(&answer, "open.order")["value"], "oldest");
    assert!(answer["history"][0]["fault"].is_string(), "{answer}");
    // THE NEXT SAVE TAKES GENERATION 3, never the damaged one's name.
    let (status, saved) = change(&root, &set(2, "newest"));
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["generation"]["generation"], 3);
    // BUILT ON GENERATION 1, AND SAID (repair 3): the save over a damaged
    // generation names what it was built on and what it stepped over.
    assert_eq!(saved["built_on"], 1, "{saved}");
    assert_eq!(saved["over_faults"][0]["generation"], 2, "{saved}");
    assert!(saved["warning"].is_string(), "{saved}");
    assert!(saved["cleared_leftover"].is_null(), "{saved}");
    // THE DAMAGE BELOW THE GENERATION IN FORCE NO LONGER DEGRADES THE
    // ANSWER, in this process as in a fresh one; `history` still names it.
    let answer = super::answer(&root, 3);
    assert_eq!(answer["generation"], 3, "{answer}");
    assert_eq!(answer["degraded"], false, "{answer}");
    assert!(answer["history"][1]["fault"].is_string(), "{answer}");
}

/// **EACH VALUE NAMES WHERE IT CAME FROM**: a generation, the start script's
/// environment, or this build's default.
#[test]
fn each_value_names_its_origin() {
    let root = root("origin");
    let answer = super::answer(&root, 0);
    let masters = value_of(&answer, "source.masters");
    assert_eq!(masters["env"], "BRUTEX_MASTERS");
    // THE FALLBACK IS WHAT THE SERVER READ AT START, so it is in force.
    assert_eq!(masters["in_force"], true, "{masters}");
    assert!(masters["not_applied"].is_null(), "{masters}");
    match std::env::var("BRUTEX_MASTERS") {
        Ok(value) if !value.trim().is_empty() => {
            assert_eq!(masters["origin"], "environment");
            assert_eq!(masters["value"], value.as_str());
        }
        _ => {
            assert_eq!(masters["origin"], "default");
            assert!(masters["value"].is_null());
        }
    }
    assert_eq!(value_of(&answer, "open.instrument")["value"], "NIFTY");
    assert_eq!(value_of(&answer, "open.segment")["value"], "INDEX");
    assert_eq!(value_of(&answer, "open.timeframe")["value"], "finest");
    assert_eq!(value_of(&answer, "open.order")["value"], "newest");
    let folder = root.display().to_string();
    let body = format!(
        r#"{{"expected":0,"changed_by":"a","reason":"b","set":{{"source.masters":{}}}}}"#,
        serde_json::Value::from(folder.as_str())
    );
    assert_eq!(change(&root, &body).0, StatusCode::OK);
    let answer = super::answer(&root, 0);
    let masters = value_of(&answer, "source.masters");
    assert_eq!(masters["origin"], "generation");
    assert_eq!(masters["value"], folder.as_str());
    // A SAVED MASTERS FOLDER IS NOT READ BY ANYTHING YET: never in force.
    assert_eq!(masters["in_force"], false, "{masters}");
    assert!(
        masters["not_applied"]
            .as_str()
            .is_some_and(|why| why.starts_with("saved; nothing reads a saved value yet")),
        "{masters}"
    );
    assert!(ist_text(1_760_000_000_000_000).ends_with(" IST"));
}

/// **AN INVERTED EFFECTIVE WINDOW IS REFUSED BEFORE ANY BYTE** — the end a
/// change leaves to its fallback counts (R3-05 repair 2, the second agent's
/// blocking 2).
///
/// Before: the store compared only the ends a generation SETS, so
/// `window.from` alone past the default `window.to`, or unsetting a later
/// `window.to`, was saved with 200 and served inverted.
#[test]
fn an_inverted_effective_window_is_refused_before_it_is_saved() {
    let root = root("inverted");
    let body = |expected: u64, change: &str| {
        format!(r#"{{"expected":{expected},"changed_by":"probe","reason":"r",{change}}}"#)
    };
    // `window.from` ALONE, PAST THE DEFAULT LAST DAY.
    let (status, answer) = change(&root, &body(0, r#""set":{"window.from":"2026-10-01"}"#));
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    assert_eq!(answer["code"], "invalid", "{answer}");
    assert_eq!(answer["key"], "window.from", "{answer}");
    assert_eq!(answer["saved"], false);
    assert!(!settings::path_of(&root, 1).exists(), "no byte written");
    // A LATER LAST DAY FIRST, THEN THE FIRST DAY: both in force, fine.
    let (status, answer) = change(&root, &body(0, r#""set":{"window.to":"2026-12-31"}"#));
    assert_eq!(status, StatusCode::OK, "{answer}");
    let (status, answer) = change(&root, &body(1, r#""set":{"window.from":"2026-10-01"}"#));
    assert_eq!(status, StatusCode::OK, "{answer}");
    // UNSETTING THE LATER LAST DAY would leave the default 2026-09-30 in
    // force before 2026-10-01: refused, naming the key it unsets.
    let (status, answer) = change(&root, &body(2, r#""unset":["window.to"]"#));
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    assert_eq!(answer["key"], "window.to", "{answer}");
    assert!(!settings::path_of(&root, 3).exists(), "no byte written");
    let served = super::answer(&root, 0);
    assert_eq!(value_of(&served, "window.from")["value"], "2026-10-01");
    assert_eq!(value_of(&served, "window.to")["value"], "2026-12-31");
    // SETTING BOTH, INVERTED, is the store's own refusal, by name.
    let (status, answer) = change(
        &root,
        &body(
            2,
            r#""set":{"window.from":"2026-11-01","window.to":"2026-10-15"}"#,
        ),
    );
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    assert_eq!(answer["code"], "invalid", "{answer}");
}

/// **A KEY NOTHING READS IS NEVER SERVED AS IN FORCE** (R3-05 repair 2, the
/// second agent's blocking 4): every value names its reader, a key with
/// none says so with what decides the behaviour instead, and a save that
/// moves such a key says it is saved and not applied.
#[test]
fn a_key_nothing_reads_is_never_served_as_in_force() {
    let root = root("readers");
    let before = super::answer(&root, 0);
    for key in Key::ALL {
        let row = value_of(&before, key.name());
        match (reader_of(key), fallback_reader_of(key)) {
            (Ok(reader), _) | (Err(_), Some(reader)) => {
                assert_eq!(row["in_force"], true, "{row}");
                assert_eq!(row["read_by"], reader, "{row}");
                assert!(row["not_applied"].is_null(), "{row}");
            }
            (Err(why), None) => {
                assert_eq!(row["in_force"], false, "{row}");
                assert!(row["read_by"].is_null(), "{row}");
                assert_eq!(row["not_applied"], why, "{row}");
            }
        }
    }
    assert_eq!(value_of(&before, "window.to")["in_force"], true);
    let (status, saved) = change(
        &root,
        r#"{"expected":0,"changed_by":"a","reason":"b","set":{"open.instrument":"BANKNIFTY","window.to":"2026-10-31"}}"#,
    );
    assert_eq!(status, StatusCode::OK, "{saved}");
    let not_applied = saved["not_applied"].as_array().expect("a list");
    assert_eq!(not_applied.len(), 1, "{saved}");
    assert_eq!(not_applied[0]["key"], "open.instrument", "{saved}");
    let after = super::answer(&root, 0);
    let instrument = value_of(&after, "open.instrument");
    assert_eq!(instrument["origin"], "generation");
    assert_eq!(instrument["in_force"], false, "{instrument}");
    assert!(
        instrument["not_applied"]
            .as_str()
            .is_some_and(|why| why.starts_with("saved; nothing reads this key yet")),
        "{instrument}"
    );
}

// -- R3-05 repair 3 -----------------------------------------------------------

/// A real FIFO at `at`, made by `mkfifo` (test-only, as `store`'s
/// `tests/fifo.rs`).
fn mkfifo(at: &Path) {
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(at)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "mkfifo made {}", at.display());
}

/// `read` on its own thread; `None` when it has not answered in ten seconds.
fn within<T: Send + 'static>(read: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (send, answer) = std::sync::mpsc::channel();
    let _thread = std::thread::spawn(move || send.send(read()));
    answer.recv_timeout(std::time::Duration::from_secs(10)).ok()
}

/// **A FIFO AT A GENERATION'S NAME IS A DEGRADED ANSWER NAMING IT, AND NO
/// STORE WAITS** (R3-05 repair 3, the second agent's blocking 2; D-4951).
///
/// Before: the read opened the FIFO and waited for a writer for ever while
/// holding the process-wide readers lock, so this store's snapshot and
/// every other store's snapshot never answered. The pipe is opened for
/// writing on the failure path so the stuck thread is released.
#[test]
fn a_fifo_at_a_generation_name_is_degraded_and_no_store_waits() {
    let stuck = root("fifo");
    let other = root("fifo-other");
    let pipe = settings::path_of(&stuck, 1);
    mkfifo(&pipe);
    let release = |pipe: &Path| {
        let _ = std::fs::OpenOptions::new().write(true).open(pipe);
    };
    let at = stuck.clone();
    let Some(stuck_snapshot) = within(move || snapshot(&at)) else {
        release(&pipe);
        panic!("the snapshot of a store with a FIFO generation waited");
    };
    let why = degraded(&stuck_snapshot).expect("a FIFO generation degrades the answer");
    assert!(why.contains("not a regular file: pipe"), "{why}");
    assert_eq!(stuck_snapshot.highest, 1);
    assert!(stuck_snapshot.newest.is_none());
    let at = other.clone();
    let Some(clean) = within(move || snapshot(&at)) else {
        release(&pipe);
        panic!("another store's snapshot waited behind the FIFO");
    };
    assert!(degraded(&clean).is_none(), "{:?}", degraded(&clean));
    // The whole answer, history included, also answers.
    let at = stuck.clone();
    let Some(answer) = within(move || super::answer(&at, 1)) else {
        release(&pipe);
        panic!("/settings.json?history=1 waited on the FIFO");
    };
    assert_eq!(answer["degraded"], true, "{answer}");
    assert!(
        answer["history"][0]["fault"]
            .as_str()
            .unwrap()
            .contains("pipe"),
        "{answer}"
    );
}

/// The store root the parking hook holds, and whether a read is parked.
static PARK_AT: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);
/// Set while a read is parked; cleared to release it.
static PARKED: (std::sync::Mutex<bool>, std::sync::Condvar) =
    (std::sync::Mutex::new(false), std::sync::Condvar::new());

/// The hook: a read of the parked root waits, with the readers lock in
/// whatever state `with_reader` left it, until the test releases it.
fn park(root: &Path) {
    if PARK_AT.lock().unwrap().as_deref() != Some(root) {
        return;
    }
    let (parked, wake) = &PARKED;
    let mut held = parked.lock().unwrap();
    *held = true;
    wake.notify_all();
    while *held {
        held = wake.wait(held).unwrap();
    }
}

/// **NO LOCK IS HELD WHILE ONE STORE'S SETTINGS ARE READ** (R3-05 repair 3).
///
/// One store's read is parked at the point its I/O begins; another store's
/// snapshot must still answer. With the refresh under the process-wide lock
/// (the code before repair 3, and the mutation this test was run against)
/// the second snapshot waits behind the first and the test fails.
#[test]
fn no_lock_is_held_while_one_store_reads() {
    let slow = root("parked");
    let other = root("not-parked");
    *PARK_AT.lock().unwrap() = Some(slow.clone());
    *BEFORE_REFRESH.write().unwrap() = Some(park);
    let at = slow.clone();
    let parked_read = std::thread::spawn(move || snapshot(&at));
    {
        let (parked, wake) = &PARKED;
        let mut held = parked.lock().unwrap();
        while !*held {
            held = wake.wait(held).unwrap();
        }
    }
    let at = other.clone();
    let answered = within(move || snapshot(&at));
    // RELEASE FIRST, so a failure leaves no thread parked.
    {
        let (parked, wake) = &PARKED;
        *parked.lock().unwrap() = false;
        wake.notify_all();
    }
    *PARK_AT.lock().unwrap() = None;
    let parked_answer = parked_read.join().unwrap();
    assert!(
        answered.is_some(),
        "another store's snapshot waited while one store's read was in progress"
    );
    assert!(degraded(&parked_answer).is_none());
}

/// **A SETTINGS DIRECTORY THAT CANNOT BE LISTED IS A SERVER-SIDE REFUSAL**:
/// `POST /settings` answers 500 `io`, never 400 (R3-05 repair 3, the second
/// agent's follow-up: it was 400 `invalid`, which blamed the change).
#[test]
fn a_settings_directory_that_is_a_file_is_a_server_side_refusal() {
    let root = root("dir-is-file");
    let at = settings::dir(&root);
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(&at, b"not a directory").unwrap();
    let (status, answer) = change(
        &root,
        r#"{"expected":0,"changed_by":"a","reason":"b","set":{"open.order":"oldest"}}"#,
    );
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{answer}");
    assert_eq!(answer["code"], "io", "{answer}");
    assert_eq!(answer["saved"], false, "{answer}");
}

/// **THE MASTERS FOLDER IN FORCE IS THE ONE THE SERVER READ, EVEN WHEN A
/// SAVE WOULD REFUSE IT** (R3-05 repair 3, the second agent's follow-up).
///
/// The server reads `BRUTEX_MASTERS` once at start as given
/// (`server::masters_dir_from`); `/settings.json` said "nothing is in force"
/// for a value `settings::check` refuses (a relative folder) while the server
/// was reading that very folder. A key with no environment fallback is not
/// moved by one.
#[test]
fn a_masters_value_a_save_would_refuse_is_reported_as_the_server_reads_it() {
    let raw = Some(std::ffi::OsString::from("relative/masters"));
    assert!(settings::check(Key::SourceMasters, "relative/masters").is_err());
    let read = crate::server::masters_dir_from(raw.clone()).unwrap();
    let got = effective_from(None, Key::SourceMasters, raw.clone());
    assert_eq!(got.value.as_deref().map(PathBuf::from), Some(read));
    assert_eq!(got.origin, "environment");
    assert!(
        got.source.contains("is in force") && got.source.contains("would refuse"),
        "{}",
        got.source
    );
    // A KEY WITH NO ENVIRONMENT FALLBACK (the GDFL folder) ignores one:
    // nothing is set until a value is saved.
    assert_eq!(Key::SourceGdfl.env(), None);
    let gdfl = effective_from(None, Key::SourceGdfl, raw);
    assert_eq!(gdfl.value, None, "{}", gdfl.source);
    assert!(gdfl.source.starts_with("not set"), "{}", gdfl.source);
    // A VALUE THE CHECK TAKES is served checked, as before.
    let dir = root("masters-ok");
    let good = effective_from(None, Key::SourceMasters, Some(dir.clone().into_os_string()));
    assert_eq!(good.value.as_deref().map(PathBuf::from), Some(dir));
}

/// **A HOLE IN THE GENERATION NUMBERS IS SERVED, AND DEGRADES THE ANSWER**
/// (R3-05 repair 4, the second agent's follow-up 2). Before: `/settings.json`
/// had no word for a missing number, so a generation planted past one was
/// invisible to a live reader and a restart put it in force with nothing
/// said. The listing names the hole beside the damaged generation above it.
#[test]
fn a_hole_in_the_generation_numbers_is_served_and_degrades_the_answer() {
    let root = root("holes");
    let set = r#"{"expected":0,"changed_by":"a","reason":"b","set":{"open.order":"oldest"}}"#;
    assert_eq!(change(&root, set).0, StatusCode::OK);
    std::fs::write(
        settings::path_of(&root, 3),
        b"brutex-settings 1\ngeneration 3\n",
    )
    .unwrap();
    // THE LIVE READER STEPS `n + 1` AND CANNOT SEE PAST THE HOLE: the
    // stated limit (docs/06-limits.md L-R305-6), not agreement.
    assert_eq!(super::answer(&root, 3)["highest"], 1);
    // A SAVE LISTS FIRST: it finds 3, is refused stale against it, and the
    // reader lists again, so 2 never lands in the hole.
    let newer = r#"{"expected":1,"changed_by":"a","reason":"b","set":{"open.order":"newest"}}"#;
    let (status, refused) = change(&root, newer);
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(
        !settings::path_of(&root, 2).exists(),
        "nothing landed in the hole"
    );
    let answer = super::answer(&root, 3);
    assert_eq!(answer["highest"], 3, "{answer}");
    assert_eq!(answer["generation"], 1, "{answer}");
    assert_eq!(answer["degraded"], true, "{answer}");
    assert!(
        answer["holes"]
            .as_str()
            .is_some_and(|holes| holes.starts_with("1 generation number(s) missing")
                && holes.contains("(2)")),
        "{answer}"
    );
    // THE HOLE ALONE DEGRADES: no fault and no stop, and `degraded` still
    // says why.
    let alone = super::Snapshot {
        highest: 3,
        newest: None,
        faults: Vec::new(),
        stopped: None,
        holes: Some("1 generation number(s) missing (2)".to_owned()),
    };
    assert_eq!(
        super::degraded(&alone).as_deref(),
        Some("1 generation number(s) missing (2)")
    );
    let clean = super::Snapshot {
        holes: None,
        ..alone
    };
    assert_eq!(super::degraded(&clean), None);
}

/// **A LANDED SAVE IS ANSWERED 200 `saved: true`, WITH WHAT ITS READ-BACK
/// FOUND BESIDE IT** (R3-05 repair 5, the second agent's blocking 1): a
/// successor generation in force is `superseded_by`, a read-back that faulted
/// is `read_back`, and `warning` says each in words. Before, both were a
/// `400 invalid saved:false`, and the request audit wrote `Refused` for a
/// change that is on disk.
#[test]
fn a_landed_save_superseded_or_unread_is_answered_saved_never_refused() {
    let generation = Arc::new(Generation {
        number: 2,
        settings: settings::Settings::default(),
        changed_at_micros: 1_700_000_000_000_000,
        changed_by: "A".to_owned(),
        reason: "a test change".to_owned(),
        changed: vec![Key::WindowTo],
        path: PathBuf::from("/nowhere/settings/v1/00000000000000000002.settings"),
    });
    let superseded = saved_answer(
        &settings::Saved {
            generation: Arc::clone(&generation),
            in_force: Some(3),
            superseded_by: Some(3),
            read_back: None,
        },
        1,
        &[],
        None,
    );
    assert_eq!(superseded["saved"], true, "{superseded}");
    assert_eq!(superseded["generation"]["generation"], 2, "{superseded}");
    assert_eq!(superseded["superseded_by"], 3, "{superseded}");
    assert_eq!(superseded["in_force"], 3, "{superseded}");
    assert!(superseded["read_back"].is_null(), "{superseded}");
    let warning = superseded["warning"].as_str().expect("a warning");
    assert!(
        warning.contains("generation 2 was saved") && warning.contains("generation 3"),
        "{warning}"
    );

    let unread = saved_answer(
        &settings::Saved {
            generation,
            in_force: Some(1),
            superseded_by: None,
            read_back: Some("saved as X, and it did not read back".to_owned()),
        },
        1,
        &[serde_json::json!({"generation": 9, "path": "p", "why": "w"})],
        None,
    );
    assert_eq!(unread["saved"], true, "{unread}");
    assert_eq!(unread["in_force"], 1, "{unread}");
    assert!(unread["superseded_by"].is_null(), "{unread}");
    let warning = unread["warning"].as_str().expect("a warning");
    assert!(
        warning.contains("did not read back") && warning.contains("built on generation 1"),
        "{warning}"
    );

    // A CLEAN SAVE CARRIES NO WARNING.
    let clean = saved_answer(
        &settings::Saved {
            generation: Arc::new(Generation {
                number: 1,
                settings: settings::Settings::default(),
                changed_at_micros: 1,
                changed_by: "A".to_owned(),
                reason: "r".to_owned(),
                changed: Vec::new(),
                path: PathBuf::from("/nowhere"),
            }),
            in_force: Some(1),
            superseded_by: None,
            read_back: None,
        },
        0,
        &[],
        None,
    );
    assert!(clean["warning"].is_null(), "{clean}");
    assert!(clean["superseded_by"].is_null(), "{clean}");
}

/// **THROUGH `POST /settings`'S OWN FUNCTION, UNDER A REAL SUCCESSOR RACE,
/// A WRITER WHOSE GENERATION IS ON DISK IS ANSWERED 200 `saved: true`** —
/// the second agent's route probe as a test (2 of 20 trials answered 400
/// before). B saves on A's generation the moment its name appears.
#[test]
fn under_a_successor_race_a_landed_post_is_answered_saved() {
    for trial in 0..20 {
        let root = root("route-successor");
        let (seeded, body) = change(
            &root,
            r#"{"expected":0,"changed_by":"seed","reason":"race","set":{"window.to":"2026-09-30"}}"#,
        );
        assert_eq!(seeded, StatusCode::OK, "{body}");
        let gate = Arc::new(std::sync::Barrier::new(2));
        let (racer, racer_gate) = (root.clone(), Arc::clone(&gate));
        let b = std::thread::spawn(move || {
            racer_gate.wait();
            let next = settings::path_of(&racer, 2);
            let started = std::time::Instant::now();
            while std::fs::symlink_metadata(&next).is_err() {
                if started.elapsed() > std::time::Duration::from_secs(10) {
                    return None;
                }
                std::hint::spin_loop();
            }
            Some(change(
                &racer,
                r#"{"expected":2,"changed_by":"B","reason":"race","set":{"window.to":"2026-11-30"}}"#,
            ))
        });
        gate.wait();
        let (status, answer) = change(
            &root,
            r#"{"expected":1,"changed_by":"A","reason":"race","set":{"window.to":"2026-10-31"}}"#,
        );
        let (b_status, b_answer) = b.join().unwrap().expect("generation 2 appeared");
        let second = std::fs::read_to_string(settings::path_of(&root, 2)).unwrap();
        assert!(second.contains("changed_by A\n"), "trial {trial}: {second}");
        assert_eq!(status, StatusCode::OK, "trial {trial}: {answer}");
        assert_eq!(answer["saved"], true, "trial {trial}: {answer}");
        assert_eq!(
            answer["generation"]["generation"], 2,
            "trial {trial}: {answer}"
        );
        assert_eq!(b_status, StatusCode::OK, "trial {trial}: {b_answer}");
        assert_eq!(b_answer["generation"]["generation"], 3, "trial {trial}");
    }
}

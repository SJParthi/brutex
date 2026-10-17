#![cfg(test)]
//! Tests of `cli watchdog` and `cli launch-agent` (R3-32). Every state
//! directory is a scratch directory, every clock is a number the test picks,
//! every notifier is a counter, and every app is a listener on 127.0.0.1 that
//! this test binary opened itself.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use std::io::{Read as _, Write as _};
use std::net::TcpListener;

use super::*;

/// 2026-10-14 10:00:00 IST, an invented moment.
const T0: u64 = 1_791_952_200;
const MINUTE: u64 = 60;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-cli-watchdog-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A notifier that counts, and fails the first `fail` deliveries.
#[derive(Default)]
struct Counter {
    pages: Vec<PathBuf>,
    fail: u32,
}

impl Notify for Counter {
    fn notify(&mut self, page: &Path) -> Result<(), String> {
        if self.fail > 0 {
            self.fail -= 1;
            return Err("the browser is not there (invented)".to_owned());
        }
        self.pages.push(page.to_path_buf());
        Ok(())
    }
}

fn watch(dir: &Path, store: Option<&Path>) -> Watch {
    Watch {
        url: "http://127.0.0.1:8080/health".to_owned(),
        state_dir: dir.to_path_buf(),
        store: store.map(Path::to_path_buf),
        interval_seconds: DEFAULT_INTERVAL_SECONDS,
    }
}

fn up() -> Outcome {
    Outcome::Answered {
        status: 200,
        millis: 3,
    }
}

fn down() -> Outcome {
    Outcome::NoAnswer {
        why: "connection refused: nothing listens on 127.0.0.1:8080".to_owned(),
        millis: 1,
    }
}

fn log_of(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join(LOG_FILE))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

fn word_of(line: &str) -> &str {
    line.split(' ').nth(1).unwrap_or_default()
}

#[test]
fn the_watchdog_notifies_once_when_health_does_not_answer() {
    let dir = scratch("once");
    let w = watch(&dir, None);
    let mut counter = Counter::default();
    let ran = watch_once(&w, T0, &up(), &mut counter).unwrap();
    assert_eq!(ran.transition, Some(Transition::Watching));
    // The app dies just after the answer at T0. One hundred probes find nothing.
    let mut notified_at = None;
    for k in 1..=100 {
        let now = T0 + k * MINUTE;
        let ran = watch_once(&w, now, &down(), &mut counter).unwrap();
        if ran.page.is_some() {
            assert!(notified_at.is_none(), "a second notice at probe {k}");
            notified_at = Some(now);
        }
        assert_eq!(ran.state.missed, u32::try_from(k).unwrap());
    }
    assert_eq!(counter.pages.len(), 1, "exactly one notice for one outage");
    let at = notified_at.unwrap();
    assert!(
        at - T0 <= 2 * MINUTE,
        "the notice came {} s after the last answer, more than two probes",
        at - T0
    );
    assert_eq!(at, T0 + u64::from(DOWN_AFTER) * MINUTE);
    let page = std::fs::read_to_string(&counter.pages[0]).unwrap();
    assert!(page.contains("brutex app is DOWN"), "{page}");
    assert!(page.contains("No answer to 2 probes in a row"), "{page}");
    assert!(page.contains("connection refused"), "{page}");
    // One line per change: watching, suspect, down, notified. Nothing per probe after.
    let words: Vec<String> = log_of(&dir).iter().map(|l| word_of(l).to_owned()).collect();
    assert_eq!(words, ["watching", "suspect", "down", "notified"]);
}

#[test]
fn the_watchdog_says_when_the_app_is_back() {
    let dir = scratch("back");
    let w = watch(&dir, None);
    let mut counter = Counter::default();
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    for k in 1..=5 {
        watch_once(&w, T0 + k * MINUTE, &down(), &mut counter).unwrap();
    }
    let back_at = T0 + 6 * MINUTE;
    let ran = watch_once(
        &w,
        back_at,
        &Outcome::Answered {
            status: 503,
            millis: 40,
        },
        &mut counter,
    )
    .unwrap();
    assert_eq!(
        ran.transition,
        Some(Transition::Back {
            down_seconds: 5 * MINUTE,
            missed: 5
        })
    );
    for k in 7..=20 {
        let ran = watch_once(&w, T0 + k * MINUTE, &up(), &mut counter).unwrap();
        assert_eq!(
            ran.transition, None,
            "probe {k} after the return changed nothing"
        );
    }
    assert_eq!(counter.pages.len(), 2, "one down, one back, nothing else");
    let back = std::fs::read_to_string(&counter.pages[1]).unwrap();
    assert!(back.contains("brutex app is back"), "{back}");
    assert!(
        back.contains("HTTP 503"),
        "an answer of 503 is the app answering: {back}"
    );
    assert!(back.contains("after 5 unanswered probes"), "{back}");
    let back_line = log_of(&dir)
        .into_iter()
        .find(|l| word_of(l) == "back")
        .unwrap();
    assert!(back_line.contains("down_seconds=300"), "{back_line}");
    assert!(back_line.contains("status=503"), "{back_line}");
    // A second outage is a second notice: "once" is per outage, not per life.
    for k in 21..=22 {
        watch_once(&w, T0 + k * MINUTE, &down(), &mut counter).unwrap();
    }
    assert_eq!(counter.pages.len(), 3);
}

#[test]
fn a_single_missed_probe_is_a_blip_and_sends_no_notice() {
    let dir = scratch("blip");
    let w = watch(&dir, None);
    let mut counter = Counter::default();
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    let ran = watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
    assert_eq!(ran.transition, Some(Transition::Suspect));
    assert!(ran.page.is_none());
    let ran = watch_once(&w, T0 + 2 * MINUTE, &up(), &mut counter).unwrap();
    assert_eq!(ran.transition, Some(Transition::Blip { missed: 1 }));
    assert!(
        counter.pages.is_empty(),
        "a restart inside a minute notifies nobody"
    );
    let words: Vec<String> = log_of(&dir).iter().map(|l| word_of(l).to_owned()).collect();
    assert_eq!(words, ["watching", "suspect", "blip"]);
    // And the next miss starts counting from one again.
    let ran = watch_once(&w, T0 + 3 * MINUTE, &down(), &mut counter).unwrap();
    assert_eq!(ran.state.missed, 1);
    assert!(counter.pages.is_empty());
}

#[test]
fn a_notice_that_could_not_be_delivered_is_tried_again_and_never_doubled() {
    let dir = scratch("retry");
    let w = watch(&dir, None);
    let mut counter = Counter {
        fail: 2,
        ..Counter::default()
    };
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
    let first = watch_once(&w, T0 + 2 * MINUTE, &down(), &mut counter).unwrap();
    assert_eq!(first.transition, Some(Transition::Down));
    assert!(first.undelivered.is_some());
    assert!(
        !first.state.down_notified,
        "a failed delivery is not a delivered one"
    );
    let second = watch_once(&w, T0 + 3 * MINUTE, &down(), &mut counter).unwrap();
    assert_eq!(second.transition, Some(Transition::DownAgain));
    assert!(second.undelivered.is_some());
    let third = watch_once(&w, T0 + 4 * MINUTE, &down(), &mut counter).unwrap();
    assert_eq!(third.transition, Some(Transition::DownAgain));
    assert!(third.undelivered.is_none());
    assert!(third.state.down_notified);
    for k in 5..=30 {
        let ran = watch_once(&w, T0 + k * MINUTE, &down(), &mut counter).unwrap();
        assert_eq!(ran.transition, None);
    }
    assert_eq!(counter.pages.len(), 1, "delivered once, after two failures");
    let failed = log_of(&dir)
        .iter()
        .filter(|l| word_of(l) == "notify-failed")
        .count();
    assert_eq!(failed, 2, "each failed delivery is a line");
    // The exit code says so too.
    let mut out = String::new();
    let code = report(&mut out, &w, &up(), Ok(first));
    assert_eq!(code, FAILED);
    assert!(out.contains("the notice was NOT delivered"), "{out}");
}

#[test]
fn an_outage_whose_notice_never_reached_the_operator_ends_without_a_back_notice() {
    let dir = scratch("unnotified");
    let w = watch(&dir, None);
    let mut counter = Counter {
        fail: 99,
        ..Counter::default()
    };
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
    watch_once(&w, T0 + 2 * MINUTE, &down(), &mut counter).unwrap();
    let ran = watch_once(&w, T0 + 3 * MINUTE, &up(), &mut counter).unwrap();
    assert_eq!(
        ran.transition,
        Some(Transition::BackUnnotified {
            down_seconds: 2 * MINUTE,
            missed: 2
        })
    );
    assert!(ran.page.is_none(), "no 'back' for a 'down' nobody saw");
    assert!(log_of(&dir).iter().any(|l| word_of(l) == "back-unnotified"));
}

#[test]
fn a_torn_state_file_is_kept_aside_and_named() {
    for (tag, bytes) in [
        ("empty", ""),
        ("cut", "watchdog-state v1 phase=up since=1"),
        ("garbage", "\u{0}\u{1}not a state\n"),
        (
            "future",
            "watchdog-state v3 phase=up since=1 missed=0 last_answer=- last_probe=1 down_notified=0 pending=-\n",
        ),
    ] {
        let dir = scratch(&format!("torn-{tag}"));
        std::fs::write(dir.join(STATE_FILE), bytes).unwrap();
        let mut counter = Counter::default();
        let ran = watch_once(&watch(&dir, None), T0, &up(), &mut counter).unwrap();
        assert_eq!(
            ran.transition,
            Some(Transition::Watching),
            "{tag}: starts fresh"
        );
        let kept = dir.join(format!("{STATE_FILE}.unreadable-{T0}"));
        assert_eq!(
            std::fs::read_to_string(&kept).unwrap(),
            bytes,
            "{tag}: the damaged bytes are kept, never deleted"
        );
        let first = log_of(&dir).into_iter().next().unwrap();
        assert_eq!(word_of(&first), "state-unreadable", "{tag}: {first}");
        assert!(first.contains("starting=fresh"), "{tag}: {first}");
        // And the state written after it reads back.
        let saved = std::fs::read_to_string(dir.join(STATE_FILE)).unwrap();
        assert_eq!(State::decode(&saved).unwrap(), ran.state);
    }
}

#[test]
fn a_tmp_state_left_by_a_killed_probe_changes_nothing() {
    let dir = scratch("killed");
    let w = watch(&dir, None);
    let mut counter = Counter::default();
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
    // A probe killed between writing the new state and renaming it.
    std::fs::write(
        dir.join(format!("{STATE_FILE}.tmp")),
        "watchdog-state v1 phase=do",
    )
    .unwrap();
    let ran = watch_once(&w, T0 + 2 * MINUTE, &down(), &mut counter).unwrap();
    assert_eq!(
        ran.transition,
        Some(Transition::Down),
        "the renamed state was the truth"
    );
    assert_eq!(counter.pages.len(), 1);
}

#[test]
fn a_second_probe_while_one_holds_the_lock_is_refused_by_name() {
    let dir = scratch("lock");
    let w = watch(&dir, None);
    let mut counter = Counter::default();
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    let holder = std::fs::File::options()
        .read(true)
        .write(true)
        .open(dir.join(LOCK_FILE))
        .unwrap();
    holder.try_lock().unwrap();
    let refused = watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap_err();
    assert!(
        refused.contains("another `cli watchdog` holds"),
        "{refused}"
    );
    assert_eq!(log_of(&dir).len(), 1, "the refused probe wrote no line");
    drop(holder);
    let ran = watch_once(&w, T0 + 2 * MINUTE, &down(), &mut counter).unwrap();
    assert_eq!(
        ran.transition,
        Some(Transition::Suspect),
        "the refused probe counted nothing"
    );
}

#[test]
fn a_clock_moved_back_is_named_and_counts_no_negative_time() {
    let dir = scratch("clock");
    let w = watch(&dir, None);
    let mut counter = Counter::default();
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
    watch_once(&w, T0 + 2 * MINUTE, &down(), &mut counter).unwrap();
    // The clock steps back an hour; the app answers.
    let ran = watch_once(&w, T0 - 3_600, &up(), &mut counter).unwrap();
    assert_eq!(
        ran.transition,
        Some(Transition::Back {
            down_seconds: 0,
            missed: 2
        })
    );
    let lines = log_of(&dir);
    let clock = lines.iter().find(|l| word_of(l) == "clock-back").unwrap();
    assert!(clock.contains("by_seconds=3720"), "{clock}");
}

#[test]
fn an_unmounted_store_is_named_in_the_notice() {
    let dir = scratch("store");
    let store = dir.join("not-mounted").join("store");
    let w = watch(&dir.join("state"), Some(&store));
    let mut counter = Counter::default();
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
    watch_once(&w, T0 + 2 * MINUTE, &down(), &mut counter).unwrap();
    let page = std::fs::read_to_string(&counter.pages[0]).unwrap();
    assert!(page.contains("is the drive mounted?"), "{page}");
    let line = log_of(&dir.join("state"))
        .into_iter()
        .find(|l| word_of(l) == "down")
        .unwrap();
    assert!(line.contains("store=absent"), "{line}");
    assert_eq!(store_state(&dir), "mounted");
    let file = dir.join("a-file");
    std::fs::write(&file, "x").unwrap();
    assert_eq!(store_state(&file), "not-a-directory");
    for word in [
        "mounted",
        "absent",
        "denied",
        "not-a-directory",
        "unreadable(x)",
    ] {
        assert!(!store_sentence(word).is_empty());
    }
}

#[test]
fn startup_waits_for_a_missing_root_with_backoff() {
    // FM-196. The waiting is launchd's: the app agent is kept alive by a
    // PathState on the store root, so while the root is missing nothing starts
    // the app again, and a refusal loop is bounded by ThrottleInterval.
    let dir = scratch("fm196");
    let api = dir.join("api");
    std::fs::write(&api, "").unwrap();
    let logs = dir.join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    let store = dir.join("Volumes").join("DRIVE").join("store");
    let words = [
        api.to_str().unwrap(),
        "127.0.0.1:8080",
        "--store",
        store.to_str().unwrap(),
        "--log-dir",
        logs.to_str().unwrap(),
    ];
    let agent = render_app(&parse_app(&words).unwrap());
    let path_state = format!(
        "<key>PathState</key>\n\t\t<dict>\n\t\t\t<key>{}</key>\n\t\t\t<true/>",
        store.display()
    );
    assert!(agent.contains(&path_state), "{agent}");
    let throttle: u64 = agent
        .split("<key>ThrottleInterval</key>\n\t<integer>")
        .nth(1)
        .and_then(|rest| rest.split('<').next())
        .and_then(|n| n.parse().ok())
        .unwrap();
    assert!(throttle >= 10, "at most one start every {throttle} s");
    assert!(
        !agent.contains("<key>RunAtLoad</key>"),
        "no unconditional start at login: the drive decides"
    );
    // The watchdog side: a missing root over 100 probes is one line per change.
    let state = dir.join("state");
    let w = watch(&state, Some(&store));
    let mut counter = Counter::default();
    for k in 0..100 {
        watch_once(&w, T0 + k * MINUTE, &down(), &mut counter).unwrap();
    }
    let lines = log_of(&state);
    let words: Vec<&str> = lines.iter().map(|l| word_of(l)).collect();
    assert_eq!(words, ["suspect", "down", "notified"]);
    assert!(lines[1].contains("store=absent"), "{}", lines[1]);
    assert!(lines[1].contains("last_answer=never"), "{}", lines[1]);
}

#[test]
fn the_launch_agent_waits_for_the_store_drive() {
    let dir = scratch("agent");
    let api = dir.join("release").join("api");
    std::fs::create_dir_all(api.parent().unwrap()).unwrap();
    std::fs::write(&api, "").unwrap();
    let logs = dir.join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    let store = "/Volumes/DRIVE/brutex/runtime/store";
    let words = [
        api.to_str().unwrap(),
        "127.0.0.1:8080",
        "--store",
        store,
        "--log-dir",
        logs.to_str().unwrap(),
        "--env",
        "BRUTEX_MASTERS=/Volumes/DRIVE/brutex/runtime/masters",
        "--env",
        "BRUTEX_WEB=/x/tgt/current/web",
        "--keep-awake",
    ];
    let mut out = String::new();
    assert_eq!(
        launch_agent(&[&["app"][..], &words[..]].concat(), &mut out),
        OK,
        "{out}"
    );
    let agent = out;
    assert!(agent.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist"));
    assert!(agent.contains("<key>Label</key>\n\t<string>in.brutex.app</string>"));
    let program = format!(
        "<array>\n\t\t<string>/usr/bin/caffeinate</string>\n\t\t<string>-i</string>\n\t\t<string>{}</string>\n\t\t<string>serve</string>\n\t\t<string>127.0.0.1:8080</string>\n\t</array>",
        api.display()
    );
    assert!(agent.contains(&program), "{agent}");
    assert!(agent.contains(&format!(
        "<key>BRUTEX_STORE</key>\n\t\t<string>{store}</string>"
    )));
    assert!(agent.contains("<key>BRUTEX_NO_OPEN</key>\n\t\t<string>1</string>"));
    assert!(agent.contains("<key>BRUTEX_WEB</key>\n\t\t<string>/x/tgt/current/web</string>"));
    assert!(agent.contains(&format!(
        "<key>KeepAlive</key>\n\t<dict>\n\t\t<key>PathState</key>\n\t\t<dict>\n\t\t\t<key>{store}</key>\n\t\t\t<true/>"
    )));
    assert!(agent.contains("<key>ThrottleInterval</key>\n\t<integer>10</integer>"));
    assert!(agent.contains("<key>ProcessType</key>\n\t<string>Interactive</string>"));
    assert!(agent.contains("<key>NumberOfFiles</key>\n\t\t<integer>65536</integer>"));
    let app_log = logs.join("app.log");
    assert!(agent.contains(&format!(
        "<key>StandardErrorPath</key>\n\t<string>{}</string>",
        app_log.display()
    )));
    // Balanced: every opened element is closed.
    for tag in ["dict", "array", "plist"] {
        assert_eq!(
            agent.matches(&format!("<{tag}>")).count() + agent.matches(&format!("<{tag} ")).count(),
            agent.matches(&format!("</{tag}>")).count(),
            "<{tag}> is balanced"
        );
    }
    // Without --keep-awake the api is the program itself.
    let plain = render_app(&parse_app(&words[..words.len() - 1]).unwrap());
    assert!(plain.contains(&format!("<array>\n\t\t<string>{}</string>", api.display())));
    assert!(!plain.contains("caffeinate"));
}

#[test]
fn the_watchdog_agent_runs_one_probe_a_minute_off_the_store_drive() {
    let dir = scratch("wagent");
    let cli = dir.join("cli");
    std::fs::write(&cli, "").unwrap();
    let state = dir.join("watchdog");
    let words = [
        "watchdog",
        cli.to_str().unwrap(),
        "http://127.0.0.1:8080/health",
        "--state-dir",
        state.to_str().unwrap(),
        "--store",
        "/Volumes/DRIVE/store",
    ];
    let mut out = String::new();
    assert_eq!(launch_agent(&words, &mut out), OK, "{out}");
    assert!(out.contains("<string>in.brutex.watchdog</string>"));
    assert!(out.contains(&format!(
        "<string>{}</string>\n\t\t<string>watchdog</string>\n\t\t<string>http://127.0.0.1:8080/health</string>\n\t\t<string>--state-dir</string>\n\t\t<string>{}</string>\n\t\t<string>--store</string>\n\t\t<string>/Volumes/DRIVE/store</string>",
        cli.display(),
        state.display()
    )), "{out}");
    assert!(out.contains("<key>StartInterval</key>\n\t<integer>60</integer>"));
    assert!(out.contains("<key>RunAtLoad</key>\n\t<true/>"));
    assert!(out.contains(&format!(
        "<key>BRUTEX_LOG_DIR</key>\n\t\t<string>{}</string>",
        state.join("events").display()
    )));
    assert!(
        !out.contains("BRUTEX_STORE"),
        "the watchdog needs no store to run"
    );
    assert!(
        !out.contains("KeepAlive"),
        "one probe a run; launchd's interval is the loop"
    );
    // The interval is bounded.
    let mut out = String::new();
    let mut longer = words.to_vec();
    longer.extend(["--interval-seconds", "120"]);
    assert_eq!(launch_agent(&longer, &mut out), OK, "{out}");
    assert!(out.contains("<integer>120</integer>"));
}

#[test]
fn an_agent_path_is_escaped_for_xml() {
    let dir = scratch("a&b<c>");
    let api = dir.join("api");
    std::fs::write(&api, "").unwrap();
    let logs = dir.join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    let words = [
        api.to_str().unwrap(),
        "127.0.0.1:8080",
        "--store",
        "/Volumes/Tom & Jerry/store",
        "--log-dir",
        logs.to_str().unwrap(),
        "--env",
        "BRUTEX_NOTE=a<b>\"c\"'d'",
    ];
    let agent = render_app(&parse_app(&words).unwrap());
    assert!(agent.contains("/Volumes/Tom &amp; Jerry/store"), "{agent}");
    assert!(agent.contains("a&amp;b&lt;c&gt;"), "{agent}");
    assert!(
        agent.contains("a&lt;b&gt;&quot;c&quot;&#39;d&#39;"),
        "{agent}"
    );
    // The store root does not exist here, so the agent carries a NOT CHECKED
    // comment quoting it. Inside an XML comment `&` is plain text and needs no
    // escape; only `--` is forbidden there. Outside comments, nothing raw.
    let mut outside = String::new();
    let mut rest = agent.as_str();
    let mut comments = 0;
    while let Some(open) = rest.find("<!--") {
        outside.push_str(&rest[..open]);
        let close = rest[open..].find("-->").expect("every comment is closed") + open;
        let inner = &rest[open + 4..close];
        assert!(
            !inner.contains("--"),
            "a comment may not hold `--`: {inner}"
        );
        assert!(!inner.ends_with('-'), "{inner}");
        comments += 1;
        rest = &rest[close + 3..];
    }
    outside.push_str(rest);
    assert_eq!(comments, 1, "one NOT CHECKED comment: {agent}");
    assert!(agent.contains("NOT CHECKED"), "{agent}");
    assert!(!outside.contains("Tom & Jerry"), "{outside}");
    assert!(!outside.contains("a&b<c>"), "{outside}");
}

#[test]
fn the_agent_verb_refuses_every_garbage_parameter() {
    let dir = scratch("garbage");
    let api = dir.join("api");
    std::fs::write(&api, "").unwrap();
    let logs = dir.join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    let api = api.to_str().unwrap();
    let logs = logs.to_str().unwrap();
    let missing = dir.join("no-such-api");
    let missing = missing.to_str().unwrap();
    let inside = format!("{}/store/logs", dir.display());
    std::fs::create_dir_all(&inside).unwrap();
    let store = format!("{}/store", dir.display());
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (vec![], "which agent"),
        (vec!["daemon"], "is not an agent"),
        (vec!["app"], "needs API and ADDR"),
        (
            vec![
                "app",
                "api",
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
            ],
            "not an absolute path",
        ),
        (
            vec![
                "app",
                missing,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
            ],
            "is not a file",
        ),
        (
            vec![
                "app",
                api,
                "localhost:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
            ],
            "is not IP:PORT",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:0",
                "--store",
                "/s",
                "--log-dir",
                logs,
            ],
            "is not IP:PORT",
        ),
        (
            vec!["app", api, "127.0.0.1:8080", "--log-dir", logs],
            "--store is required",
        ),
        (
            vec!["app", api, "127.0.0.1:8080", "--store", "/s"],
            "--log-dir is required",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                "/no/such/dir",
            ],
            "is not a directory",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                &store,
                "--log-dir",
                &inside,
            ],
            "inside the store root",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--env",
                "NOEQUALS",
            ],
            "is not KEY=VALUE",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--env",
                "lower=1",
            ],
            "is not A-Z",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--env",
                "=1",
            ],
            "is not A-Z",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--env",
                "BRUTEX_STORE=/t",
            ],
            "is set by the agent itself",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--env",
                "BRUTEX_NO_OPEN=0",
            ],
            "is set by the agent itself",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--env",
                "A=1",
                "--env",
                "A=2",
            ],
            "is given twice",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--env",
                "A=x\ny",
            ],
            "control character",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--throttle-seconds",
                "9",
            ],
            "from 10 to 600",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--throttle-seconds",
                "x",
            ],
            "from 10 to 600",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--max-files",
                "255",
            ],
            "from 256 to 1048576",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--max-files",
                "1048577",
            ],
            "from 256 to 1048576",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--keep-awake",
                "--keep-awake",
            ],
            "is given twice",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--store",
                "/t",
                "--log-dir",
                logs,
            ],
            "is given twice",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--throttle-seconds",
            ],
            "needs a value",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "/s",
                "--log-dir",
                logs,
                "--port",
                "1",
            ],
            "is not a flag",
        ),
        (
            vec![
                "app",
                api,
                "127.0.0.1:8080",
                "--store",
                "",
                "--log-dir",
                logs,
            ],
            "empty or holds a control",
        ),
        (vec!["watchdog"], "needs CLI and URL"),
        (
            vec![
                "watchdog",
                api,
                "https://127.0.0.1/health",
                "--state-dir",
                "/w",
            ],
            "not an http://",
        ),
        (
            vec!["watchdog", api, "http://127.0.0.1:8080/health"],
            "--state-dir is required",
        ),
        (
            vec![
                "watchdog",
                api,
                "http://127.0.0.1:8080/health",
                "--state-dir",
                "w",
            ],
            "not an absolute path",
        ),
        (
            vec![
                "watchdog",
                api,
                "http://127.0.0.1:8080/health",
                "--state-dir",
                "/s/w",
                "--store",
                "/s",
            ],
            "inside the store root",
        ),
        (
            vec![
                "watchdog",
                api,
                "http://127.0.0.1:8080/health",
                "--state-dir",
                "/w",
                "--store",
                "s",
            ],
            "not an absolute path",
        ),
        (
            vec![
                "watchdog",
                api,
                "http://127.0.0.1:8080/health",
                "--state-dir",
                "/w",
                "--interval-seconds",
                "9",
            ],
            "from 10 to 3600",
        ),
        (
            vec![
                "watchdog",
                missing,
                "http://127.0.0.1:8080/health",
                "--state-dir",
                "/w",
            ],
            "is not a file",
        ),
    ];
    for (words, expected) in cases {
        let mut out = String::new();
        assert_eq!(launch_agent(&words, &mut out), MISUSED, "{words:?}: {out}");
        assert!(
            out.contains(expected),
            "{words:?} should say `{expected}`: {out}"
        );
        assert!(out.contains("usage: cli launch-agent"), "{words:?}: {out}");
        assert!(!out.contains("<plist"), "{words:?} printed an agent");
    }
}

#[test]
fn the_address_is_refused_by_name_for_every_garbage_form() {
    let good = Target::parse("http://127.0.0.1:8080/health").unwrap();
    assert_eq!(good.addr(), "127.0.0.1:8080".parse().unwrap());
    assert_eq!(good.path, "/health");
    assert_eq!(good.host, "127.0.0.1:8080");
    let local = Target::parse("http://localhost:8080").unwrap();
    assert_eq!(local.addr(), "127.0.0.1:8080".parse().unwrap());
    assert_eq!(local.path, "/");
    assert_eq!(
        Target::parse("http://127.0.0.1/h").unwrap().addr().port(),
        80
    );
    assert_eq!(
        Target::parse("http://[::1]:8080/health").unwrap().addr(),
        "[::1]:8080".parse().unwrap()
    );
    assert_eq!(Target::parse("http://[::1]/").unwrap().addr().port(), 80);
    for (url, expected) in [
        ("", "is not an address"),
        ("http://127.0.0.1:8080/he alth", "is not an address"),
        ("http://127.0.0.1:8080/\u{7}", "is not an address"),
        ("https://127.0.0.1:8080/health", "not an http://"),
        ("127.0.0.1:8080/health", "not an http://"),
        ("http://:8080/health", "names no host"),
        ("http:///health", "names no host"),
        ("http://127.0.0.1:0/health", "is not a port"),
        ("http://127.0.0.1:70000/health", "is not a port"),
        ("http://127.0.0.1:/health", "is not a port"),
        ("http://example.com:8080/health", "resolves no names"),
        ("http://user@127.0.0.1:8080/health", "carries a user name"),
        ("http://[::1:8080/health", "never closes it"),
        ("http://[::1]8080/health", "only `:PORT` may follow"),
        ("http://[]:8080/health", "names no host"),
    ] {
        let refused = Target::parse(url).unwrap_err();
        assert!(
            refused.contains(expected),
            "`{url}` should say `{expected}`: {refused}"
        );
    }
}

#[test]
fn the_watch_verb_refuses_garbage_and_runs_without_a_store() {
    for (words, expected) in [
        (vec![], "the URL of /health is missing"),
        (
            vec!["http://127.0.0.1:8080/health"],
            "--state-dir is required",
        ),
        (
            vec!["http://127.0.0.1:8080/health", "--state-dir", "rel"],
            "not an absolute path",
        ),
        (
            vec![
                "http://127.0.0.1:8080/health",
                "--state-dir",
                "/s/w",
                "--store",
                "/s",
            ],
            "inside the store root",
        ),
        (
            vec![
                "http://127.0.0.1:8080/health",
                "--state-dir",
                "/w",
                "--notify",
                "banner",
            ],
            "neither `open` nor `record`",
        ),
        (
            vec![
                "http://127.0.0.1:8080/health",
                "--state-dir",
                "/w",
                "--timeout-ms",
                "0",
            ],
            "from 1 to 60000",
        ),
        (
            vec![
                "http://127.0.0.1:8080/health",
                "--state-dir",
                "/w",
                "--timeout-ms",
                "60001",
            ],
            "from 1 to 60000",
        ),
        (
            vec![
                "http://127.0.0.1:8080/health",
                "--state-dir",
                "/w",
                "--bogus",
                "1",
            ],
            "is not a flag",
        ),
        (
            vec!["ftp://127.0.0.1/health", "--state-dir", "/w"],
            "not an http://",
        ),
    ] {
        let mut out = String::new();
        assert_eq!(command(&words, &mut out), MISUSED, "{words:?}: {out}");
        assert!(
            out.contains(expected),
            "{words:?} should say `{expected}`: {out}"
        );
        assert!(out.contains("usage: cli watchdog"), "{out}");
    }
    let parsed = parse_watch(&["http://127.0.0.1:8080/health", "--state-dir", "/w"]).unwrap();
    assert_eq!(parsed.2, PROBE_LIMIT, "five seconds unless asked");
    assert!(parsed.3, "a real notice unless asked");
    assert!(needs_no_store(Some(std::ffi::OsStr::new("watchdog"))));
    assert!(needs_no_store(Some(std::ffi::OsStr::new("launch-agent"))));
    assert!(!needs_no_store(Some(std::ffi::OsStr::new("alerts"))));
    assert!(!needs_no_store(Some(std::ffi::OsStr::new("watchdogs"))));
    assert!(!needs_no_store(None));
    let word = |w: &str| Some(std::ffi::OsString::from(w));
    assert_eq!(banner_on_stderr(word("watchdog").as_deref()), None);
    assert_eq!(
        banner_on_stderr(word("launch-agent").as_deref()),
        Some(true)
    );
    assert_eq!(banner_on_stderr(word("alerts").as_deref()), Some(false));
    assert_eq!(banner_on_stderr(None), Some(false));
}

#[test]
fn a_number_flag_with_a_sign_or_a_leading_zero_is_refused() {
    for (flag, text) in [
        ("--timeout-ms", "+500"),
        ("--timeout-ms", "0500"),
        ("--timeout-ms", " 500"),
        ("--interval-seconds", "+60"),
        ("--interval-seconds", "060"),
        ("--interval-seconds", "6e1"),
        ("--interval-seconds", ""),
    ] {
        let mut out = String::new();
        let words = [
            "http://127.0.0.1:8080/health",
            "--state-dir",
            "/w",
            flag,
            text,
        ];
        assert_eq!(command(&words, &mut out), MISUSED, "{flag} {text:?}: {out}");
        assert!(
            out.contains("digits only, no sign, no leading zero"),
            "{flag} {text:?} should be refused by spelling: {out}"
        );
    }
    let logs = scratch("number-flags");
    let throttle = parse_app(&[
        "/bin/sh",
        "127.0.0.1:8080",
        "--store",
        "/brutex-no-such-store-root",
        "--log-dir",
        logs.to_str().unwrap(),
        "--throttle-seconds",
        "+10",
    ])
    .unwrap_err();
    assert!(throttle.contains("no sign"), "{throttle}");
    assert_eq!(bounded("--x", "10", 10, 600), Ok(10));
    assert_eq!(bounded("--x", "600", 10, 600), Ok(600));
    assert!(bounded("--x", "0", 1, 9).is_err(), "0 below the range");
    assert_eq!(
        bounded("--x", "0", 0, 9),
        Ok(0),
        "a lone 0 is a plain spelling"
    );
}

#[test]
fn an_unusable_state_directory_is_refused_by_name() {
    let dir = scratch("unusable");
    let file = dir.join("a-file");
    std::fs::write(&file, "x").unwrap();
    let w = watch(&file.join("state"), None);
    let refused = watch_once(&w, T0, &up(), &mut Counter::default()).unwrap_err();
    assert!(refused.contains("cannot be made"), "{refused}");
    let mut out = String::new();
    assert_eq!(report(&mut out, &w, &up(), Err(refused)), FAILED);
    assert!(
        out.starts_with("refused: the watchdog's state directory"),
        "{out}"
    );
}

#[test]
fn the_state_line_round_trips_and_refuses_every_damaged_form() {
    let states = [
        State::fresh(),
        State {
            phase: Phase::Down,
            since: T0,
            missed: u32::MAX,
            last_answer: Some(T0 - 1),
            last_probe: u64::MAX,
            down_notified: true,
            pending: Some(Notice::Down),
        },
        State {
            phase: Phase::Suspect,
            since: 0,
            missed: 1,
            last_answer: None,
            last_probe: 0,
            down_notified: false,
            pending: None,
        },
    ];
    for state in states {
        assert_eq!(State::decode(&state.encode()).unwrap(), state);
    }
    let line = State::fresh().encode();
    for (bad, expected) in [
        (line.trim_end().to_owned(), "cut short"),
        (line.replace("v2", "v9"), "begins neither"),
        (line.replace("phase=fresh", "phase=gone"), "is not a phase"),
        (line.replace("since=0", "since=-1"), "is not a count"),
        (
            line.replace("missed=0", "missed=4294967296"),
            "out of range",
        ),
        (
            line.replace("last_answer=-", "last_answer=x"),
            "is not a count",
        ),
        (
            line.replace("down_notified=0", "down_notified=2"),
            "is not 0 or 1",
        ),
        (line.replace('\n', " extra=1\n"), "more fields"),
        (line.replace("since=0 ", ""), "no `since=`"),
    ] {
        let refused = State::decode(&bad).unwrap_err();
        assert!(
            refused.contains(expected),
            "`{bad}` should say `{expected}`: {refused}"
        );
    }
}

/// A one-connection app: answers each accepted connection with `answer`
/// after `delay`, holding the socket open for `hold`.
fn app(
    answer: &'static [u8],
    delay: Duration,
    hold: Duration,
) -> (Target, std::thread::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut asked = vec![0_u8; 1024];
        let read = stream.read(&mut asked).unwrap_or(0);
        asked.truncate(read);
        std::thread::sleep(delay);
        for piece in answer.split_inclusive(|b| *b == b'|') {
            let piece = piece.strip_suffix(b"|").unwrap_or(piece);
            let _ = stream.write_all(piece);
            let _ = stream.flush();
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(hold);
        asked
    });
    (
        Target::parse(&format!("http://127.0.0.1:{port}/health")).unwrap(),
        handle,
    )
}

#[test]
fn a_listener_that_answers_is_up_with_its_status() {
    for (answer, status) in [
        (&b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok"[..], 200),
        (&b"HTTP/1.1 503 Service Unavailable\r\n\r\n"[..], 503),
        // A status line split across reads is read whole.
        (&b"HTTP/1.1 2|04 No Content\r|\n\r\n"[..], 204),
    ] {
        let (target, handle) = app(answer, Duration::ZERO, Duration::ZERO);
        let outcome = probe(&target, Duration::from_secs(5));
        let asked = handle.join().unwrap();
        match outcome {
            Outcome::Answered { status: got, .. } => assert_eq!(got, status),
            Outcome::NoAnswer { why, .. } => panic!("expected {status}, got no answer: {why}"),
        }
        let asked = String::from_utf8(asked).unwrap();
        assert!(
            asked.starts_with("GET /health HTTP/1.1\r\nHost: 127.0.0.1:"),
            "{asked}"
        );
        assert!(asked.contains("Connection: close\r\n"), "{asked}");
    }
}

#[test]
fn a_listener_that_never_answers_is_no_answer_within_the_limit() {
    let (target, handle) = app(b"", Duration::ZERO, Duration::from_millis(1_500));
    let limit = Duration::from_millis(300);
    let started = Instant::now();
    let outcome = probe(&target, limit);
    let took = started.elapsed();
    match &outcome {
        Outcome::NoAnswer { why, millis } => {
            assert!(why.contains("no answer within 300 ms"), "{why}");
            assert!(*millis >= 250, "{millis}");
        }
        Outcome::Answered { .. } => panic!("a silent app answered: {outcome:?}"),
    }
    assert!(
        took < Duration::from_millis(1_200),
        "the limit held: {took:?}"
    );
    handle.join().unwrap();
}

#[test]
fn a_slow_answer_past_the_limit_is_no_answer() {
    let (target, handle) = app(
        b"HTTP/1.1 200 OK\r\n\r\n",
        Duration::from_millis(600),
        Duration::ZERO,
    );
    let outcome = probe(&target, Duration::from_millis(200));
    assert!(matches!(outcome, Outcome::NoAnswer { .. }), "{outcome:?}");
    handle.join().unwrap();
}

#[test]
fn a_garbage_or_closed_answer_is_not_an_answer() {
    let long: &'static [u8] = Box::leak(vec![b'x'; 4_096].into_boxed_slice());
    for (answer, expected) in [
        (&b"SSH-2.0-OpenSSH\r\n"[..], "not an HTTP answer"),
        (&b"HTTP/1.1 999 Nope\r\n"[..], "not an HTTP answer"),
        (&b"HTTP/1.1 20 OK\r\n"[..], "not an HTTP answer"),
        (&b"HTTP/2 200\r\n"[..], "not an HTTP answer"),
        (long, "no status line in the first 1024 bytes"),
        (&b""[..], "closed with no answer"),
        (&b"HTTP/1.1 200"[..], "closed inside the status line"),
    ] {
        let (target, handle) = app(answer, Duration::ZERO, Duration::ZERO);
        let outcome = probe(&target, Duration::from_secs(5));
        handle.join().unwrap();
        match outcome {
            Outcome::NoAnswer { why, .. } => {
                assert!(why.contains(expected), "should say `{expected}`: {why}");
            }
            Outcome::Answered { status, .. } => panic!("garbage answered {status}"),
        }
    }
}

#[test]
fn a_closed_port_is_no_answer_by_name() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let target = Target::parse(&format!("http://127.0.0.1:{port}/health")).unwrap();
    match probe(&target, Duration::from_secs(5)) {
        Outcome::NoAnswer { why, .. } => assert!(why.contains("connection refused"), "{why}"),
        Outcome::Answered { status, .. } => panic!("a closed port answered {status}"),
    }
}

#[test]
fn the_verb_probes_records_and_exits_by_what_it_found() {
    let dir = scratch("verb");
    let state = dir.join("state");
    let (target, handle) = app(b"HTTP/1.1 200 OK\r\n\r\n", Duration::ZERO, Duration::ZERO);
    let url = format!("http://{}/health", target.addr());
    let state_text = state.to_str().unwrap();
    let mut out = String::new();
    let code = command(
        &[&url, "--state-dir", state_text, "--notify", "record"],
        &mut out,
    );
    handle.join().unwrap();
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("answered 200 in"), "{out}");
    assert!(out.contains("; watching"), "{out}");
    // The app is gone now: no listener on that port.
    let mut out = String::new();
    let code = command(
        &[
            &url,
            "--state-dir",
            state_text,
            "--notify",
            "record",
            "--timeout-ms",
            "500",
        ],
        &mut out,
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("gave no answer"), "{out}");
    assert!(out.contains("1 in a row; suspect"), "{out}");
    let mut out = String::new();
    let code = command(
        &[&url, "--state-dir", state_text, "--notify", "record"],
        &mut out,
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("2 in a row; down"), "{out}");
    let pages: Vec<_> = std::fs::read_dir(state.join(NOTICE_DIR)).unwrap().collect();
    assert_eq!(pages.len(), 1, "`record` writes the page and opens nothing");
}

#[test]
fn two_notices_in_one_second_get_two_pages() {
    let dir = scratch("same-second");
    std::fs::create_dir_all(dir.join(NOTICE_DIR)).unwrap();
    let w = watch(&dir, None);
    let prev = State {
        phase: Phase::Suspect,
        since: T0,
        missed: 1,
        last_answer: None,
        last_probe: T0,
        down_notified: false,
        pending: None,
    };
    let first = write_page(
        &dir,
        Notice::Down,
        &w,
        &prev,
        &down(),
        &Transition::Down,
        T0,
        None,
    )
    .unwrap();
    let second = write_page(
        &dir,
        Notice::Down,
        &w,
        &prev,
        &down(),
        &Transition::Down,
        T0,
        None,
    )
    .unwrap();
    assert_ne!(first, second, "a page is never overwritten");
    assert!(
        second.to_str().unwrap().ends_with("-down-1.html"),
        "{}",
        second.display()
    );
    for _ in 2..16 {
        write_page(
            &dir,
            Notice::Down,
            &w,
            &prev,
            &down(),
            &Transition::Down,
            T0,
            None,
        )
        .unwrap();
    }
    let refused = write_page(
        &dir,
        Notice::Down,
        &w,
        &prev,
        &down(),
        &Transition::Down,
        T0,
        None,
    )
    .unwrap_err();
    assert!(refused.contains("sixteen notice pages"), "{refused}");
}

#[test]
fn a_notice_page_escapes_what_it_quotes() {
    let dir = scratch("escape");
    std::fs::create_dir_all(dir.join(NOTICE_DIR)).unwrap();
    let mut w = watch(&dir, None);
    w.url = "http://127.0.0.1:8080/<b>".to_owned();
    let outcome = Outcome::NoAnswer {
        why: "a <script> & \"quote\"".to_owned(),
        millis: 1,
    };
    let page = write_page(
        &dir,
        Notice::Down,
        &w,
        &State::fresh(),
        &outcome,
        &Transition::Down,
        T0,
        None,
    )
    .unwrap();
    let html = std::fs::read_to_string(page).unwrap();
    assert!(
        html.contains("a &lt;script&gt; &amp; &quot;quote&quot;"),
        "{html}"
    );
    assert!(html.contains("/&lt;b&gt;"), "{html}");
    assert!(!html.contains("<script>"));
    assert!(html.contains("2026-10-14 10:00:00 IST"), "{html}");
    assert!(html.contains("2026-10-14T04:30:00Z"), "{html}");
}

#[test]
fn the_platform_opener_names_a_failure() {
    // A page that does not exist: `open` refuses it and starts nothing, or,
    // where there is no `open`, the spawn fails. Either is an error by name.
    let refused = Opener
        .notify(Path::new("/nonexistent-brutex-watchdog-test/page.html"))
        .unwrap_err();
    assert!(refused.contains("`open`"), "{refused}");
}

#[test]
fn every_transition_has_a_distinct_word() {
    let all = [
        Transition::Watching,
        Transition::Suspect,
        Transition::Blip { missed: 1 },
        Transition::Down,
        Transition::DownAgain,
        Transition::Back {
            down_seconds: 1,
            missed: 2,
        },
        Transition::BackUnnotified {
            down_seconds: 1,
            missed: 2,
        },
    ];
    let mut words: Vec<&str> = all.iter().map(Transition::word).collect();
    words.sort_unstable();
    words.dedup();
    assert_eq!(words.len(), all.len());
    let notices: Vec<Option<Notice>> = all.iter().map(Transition::notice).collect();
    assert_eq!(
        notices,
        [
            None,
            None,
            None,
            Some(Notice::Down),
            Some(Notice::Down),
            Some(Notice::Back),
            None
        ]
    );
    for phase in [Phase::Fresh, Phase::Up, Phase::Suspect, Phase::Down] {
        assert_eq!(Phase::from_word(phase.word()), Some(phase));
    }
}

/// THE DRILL (R3-32), run by the operator against the launchd-run app, never
/// by CI: `BRUTEX_DRILL_URL=http://127.0.0.1:8080/health cargo test -p cli
/// --release -- --ignored --nocapture kill_the_app_and_it_is_back_within_a_minute`,
/// then kill the app (docs/36 §9.4) five times when told. Each round waits for
/// /health to stop answering, then measures how long until it answers again.
#[test]
#[ignore = "the operator's kill drill against the launchd-run app (docs/36 section 9.4)"]
fn kill_the_app_and_it_is_back_within_a_minute() {
    let url = std::env::var("BRUTEX_DRILL_URL")
        .expect("set BRUTEX_DRILL_URL to the launchd-run app's /health");
    let target = Target::parse(&url).unwrap();
    let poll = Duration::from_millis(250);
    let mut seconds = Vec::new();
    for round in 1..=5 {
        eprintln!("round {round} of 5: kill the app now (docs/36 section 9.4)");
        let waiting = Instant::now();
        while probe(&target, PROBE_LIMIT).answered() {
            assert!(
                waiting.elapsed() < Duration::from_mins(5),
                "round {round}: the app was never killed"
            );
            std::thread::sleep(poll);
        }
        let gone = Instant::now();
        while !probe(&target, PROBE_LIMIT).answered() {
            assert!(
                gone.elapsed() <= Duration::from_mins(1),
                "round {round}: not back within 60 s"
            );
            std::thread::sleep(poll);
        }
        let took = gone.elapsed();
        eprintln!("round {round}: back in {:.1} s", took.as_secs_f64());
        seconds.push(took);
    }
    assert!(seconds.iter().all(|took| *took <= Duration::from_mins(1)));
}

#[test]
fn a_run_never_lists_the_notices_or_reads_the_log() {
    // The module's cost claim, held by shape: a run appends to the log and
    // creates a page by name, so a log nobody may read and a notices folder
    // nobody may list change nothing. 10,000 old lines and 500 old pages are
    // there to be scanned if anything did scan them.
    use std::os::unix::fs::PermissionsExt as _;
    let dir = scratch("o1");
    let w = watch(&dir, None);
    let mut counter = Counter::default();
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    let mut old = String::new();
    for k in 0..10_000 {
        old.push_str("2026-01-01T00:00:00Z blip n=");
        old.push_str(&k.to_string());
        old.push('\n');
    }
    std::fs::write(dir.join(LOG_FILE), old).unwrap();
    for k in 0..500 {
        std::fs::write(dir.join(NOTICE_DIR).join(format!("old-{k}.html")), "x").unwrap();
    }
    let log = dir.join(LOG_FILE);
    let notices = dir.join(NOTICE_DIR);
    std::fs::set_permissions(&log, std::fs::Permissions::from_mode(0o200)).unwrap();
    std::fs::set_permissions(&notices, std::fs::Permissions::from_mode(0o300)).unwrap();
    assert!(
        std::fs::read_to_string(&log).is_err(),
        "the log is unreadable now"
    );
    assert!(
        std::fs::read_dir(&notices).is_err(),
        "the folder is unlistable now"
    );
    watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
    let ran = watch_once(&w, T0 + 2 * MINUTE, &down(), &mut counter).unwrap();
    assert_eq!(ran.transition, Some(Transition::Down));
    assert!(ran.page.unwrap().is_file());
    std::fs::set_permissions(&log, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::set_permissions(&notices, std::fs::Permissions::from_mode(0o700)).unwrap();
    let lines = log_of(&dir);
    assert_eq!(lines.len(), 10_003, "appended, never rewritten");
    assert_eq!(word_of(&lines[10_000]), "suspect");
}

// ---------------------------------------------------------------------------
// verify-R3-32-r1: the blocking items and the follow-ups on this module
// ---------------------------------------------------------------------------

#[test]
fn a_lock_held_for_a_moment_is_waited_for_not_refused() {
    // Blocking 1: any spawn in this process (a test's `open`, or an in-process
    // caller's child) holds a duplicate of the lock's descriptor until its
    // `exec`. A holder that lets go within LOCK_WAIT costs no probe.
    let dir = scratch("lock-moment");
    let w = watch(&dir, None);
    let mut counter = Counter::default();
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    let holder = std::fs::File::options()
        .read(true)
        .write(true)
        .open(dir.join(LOCK_FILE))
        .unwrap();
    holder.try_lock().unwrap();
    let letting_go = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        drop(holder);
    });
    let started = Instant::now();
    let ran = watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
    letting_go.join().unwrap();
    assert_eq!(
        ran.transition,
        Some(Transition::Suspect),
        "the probe counted"
    );
    assert!(
        started.elapsed() < LOCK_WAIT + Duration::from_millis(500),
        "waited {:?}, bounded by LOCK_WAIT",
        started.elapsed()
    );
    // A holder that never lets go is still refused by name, after LOCK_WAIT.
    let stuck = std::fs::File::options()
        .read(true)
        .write(true)
        .open(dir.join(LOCK_FILE))
        .unwrap();
    stuck.try_lock().unwrap();
    let started = Instant::now();
    let refused = watch_once(&w, T0 + 2 * MINUTE, &down(), &mut counter).unwrap_err();
    let waited = started.elapsed();
    assert!(
        refused.contains("another `cli watchdog` holds"),
        "{refused}"
    );
    assert!(refused.contains("waited"), "{refused}");
    assert!(waited >= LOCK_WAIT, "refused only after {waited:?}");
    assert!(
        waited < LOCK_WAIT + Duration::from_millis(500),
        "{waited:?}"
    );
    assert!(counter.pages.is_empty());
}

#[test]
fn a_state_that_cannot_be_saved_withholds_the_notice_and_never_repeats_it() {
    // B3 (and a full disk, which fails the same save): the notice used to go
    // out BEFORE the state was saved, so an unsavable state re-sent it every
    // minute. Now it is withheld, said in the log, and sent once when the
    // state can be saved again.
    let dir = scratch("unsavable");
    let w = watch(&dir, None);
    let mut counter = Counter::default();
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
    // Something that is not a file at the temporary's name: the staging door
    // refuses it, exactly as a full disk refuses the write.
    let blocker = dir.join(format!("{STATE_FILE}.tmp"));
    std::fs::create_dir(&blocker).unwrap();
    for k in 2..5 {
        let refused = watch_once(&w, T0 + k * MINUTE, &down(), &mut counter).unwrap_err();
        assert!(refused.contains("withheld, not sent"), "{refused}");
    }
    assert!(
        counter.pages.is_empty(),
        "no notice while it cannot be remembered"
    );
    let pages = std::fs::read_dir(dir.join(NOTICE_DIR)).unwrap().count();
    assert_eq!(pages, 0, "no page is written for a withheld notice");
    let withheld = log_of(&dir)
        .iter()
        .filter(|l| word_of(l) == "notice-withheld")
        .count();
    assert_eq!(withheld, 3, "each withheld notice is a line");
    // The state on disk is still the last one saved: one miss.
    let saved = std::fs::read_to_string(dir.join(STATE_FILE)).unwrap();
    assert_eq!(State::decode(&saved).unwrap().phase, Phase::Suspect);
    // Moved aside (nothing is deleted): the next probe sends it, once.
    std::fs::rename(&blocker, dir.join("blocker-moved-aside")).unwrap();
    let ran = watch_once(&w, T0 + 5 * MINUTE, &down(), &mut counter).unwrap();
    assert_eq!(ran.transition, Some(Transition::Down));
    for k in 6..20 {
        let ran = watch_once(&w, T0 + k * MINUTE, &down(), &mut counter).unwrap();
        assert_eq!(ran.transition, None);
    }
    assert_eq!(counter.pages.len(), 1, "one notice for the outage");
}

#[test]
fn a_read_only_state_directory_sends_no_notice_at_all_rather_than_many() {
    // B3 as the verifier ran it: the directory made read-only after a miss.
    use std::os::unix::fs::PermissionsExt as _;
    let dir = scratch("readonly");
    let w = watch(&dir, None);
    let mut counter = Counter::default();
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let mut refusals = Vec::new();
    for k in 2..5 {
        refusals.push(watch_once(&w, T0 + k * MINUTE, &down(), &mut counter));
    }
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    for refused in refusals {
        let why = refused.unwrap_err();
        assert!(why.contains("withheld, not sent"), "{why}");
    }
    assert!(
        counter.pages.is_empty(),
        "never three notices for one outage"
    );
    let ran = watch_once(&w, T0 + 5 * MINUTE, &down(), &mut counter).unwrap();
    assert_eq!(ran.transition, Some(Transition::Down));
    assert_eq!(counter.pages.len(), 1);
}

/// A notifier that dies while it runs, as a run killed during `open` does,
/// after checking that the notice was saved before it was handed over.
struct DiesWhileOpening {
    state_dir: PathBuf,
}

impl Notify for DiesWhileOpening {
    fn notify(&mut self, _page: &Path) -> Result<(), String> {
        let saved = std::fs::read_to_string(self.state_dir.join(STATE_FILE)).unwrap();
        let state = State::decode(&saved).unwrap();
        assert_eq!(state.pending, Some(Notice::Down), "saved before it is sent");
        assert!(state.down_notified);
        panic!("killed while `open` runs (invented)");
    }
}

#[test]
fn a_run_killed_while_open_runs_sends_no_second_notice() {
    // B5: the page existed, no line said so, and the next run sent a second
    // DOWN for the same outage.
    let dir = scratch("killed-open");
    let w = watch(&dir, None);
    let mut counter = Counter::default();
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
    let mut opener_that_dies = DiesWhileOpening {
        state_dir: dir.clone(),
    };
    let run_outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        watch_once(&w, T0 + 2 * MINUTE, &down(), &mut opener_that_dies)
    }));
    assert!(run_outcome.is_err(), "the run died inside `open`");
    let before = log_of(&dir);
    assert_eq!(
        before.last().map(|l| word_of(l)),
        Some("down"),
        "the transition was logged before the notice went out: {before:?}"
    );
    let ran = watch_once(&w, T0 + 3 * MINUTE, &down(), &mut counter).unwrap();
    assert_eq!(ran.transition, None, "not DownAgain: it counts as sent");
    assert!(counter.pages.is_empty(), "no second notice");
    let unknown = &ran.lines[0];
    assert_eq!(word_of(unknown), "notify-unknown", "{unknown}");
    assert!(unknown.contains("notice=down"), "{unknown}");
    assert!(
        unknown.contains(&format!("sent_at={}", utc(T0 + 2 * MINUTE))),
        "{unknown}"
    );
    assert_eq!(ran.state.pending, None, "said once, then cleared");
    let again = watch_once(&w, T0 + 4 * MINUTE, &down(), &mut counter).unwrap();
    assert!(again.lines.is_empty(), "{:?}", again.lines);
    // The app comes back: one BACK, because the DOWN counts as sent.
    let back = watch_once(&w, T0 + 5 * MINUTE, &up(), &mut counter).unwrap();
    assert!(matches!(back.transition, Some(Transition::Back { .. })));
    assert_eq!(counter.pages.len(), 1);
}

#[test]
fn a_state_that_is_binary_huge_or_a_directory_is_kept_aside_not_refused_for_ever() {
    // B7 (non-UTF-8), B4 (a directory) and a 64 KiB file, which used to be
    // read whole: each is damage, kept aside by name, and the run starts fresh.
    let huge = format!(
        "watchdog-state v2 phase=up since=1 missed=0 last_answer=- last_probe=1 down_notified=0 pending=-{}\n",
        " ".repeat(65_536)
    );
    let cases: [(&str, Option<Vec<u8>>); 4] = [
        ("binary", Some(vec![0xff, 0xfe, b'\n'])),
        (
            "latin1",
            Some(b"watchdog-state v2 phase=up \xe9\n".to_vec()),
        ),
        ("huge", Some(huge.into_bytes())),
        ("directory", None),
    ];
    for (tag, bytes) in cases {
        let dir = scratch(&format!("damaged-{tag}"));
        let at = dir.join(STATE_FILE);
        match &bytes {
            Some(bytes) => std::fs::write(&at, bytes).unwrap(),
            None => std::fs::create_dir(&at).unwrap(),
        }
        let mut counter = Counter::default();
        let ran = watch_once(&watch(&dir, None), T0, &up(), &mut counter)
            .unwrap_or_else(|why| panic!("{tag}: refused instead of kept aside: {why}"));
        assert_eq!(ran.transition, Some(Transition::Watching), "{tag}");
        let kept = dir.join(format!("{STATE_FILE}.unreadable-{T0}"));
        match &bytes {
            Some(bytes) => assert_eq!(&std::fs::read(&kept).unwrap(), bytes, "{tag}: kept whole"),
            None => assert!(kept.is_dir(), "{tag}: the directory is kept, not removed"),
        }
        assert_eq!(word_of(&ran.lines[0]), "state-unreadable", "{tag}");
        let saved = std::fs::read_to_string(&at).unwrap();
        assert_eq!(State::decode(&saved).unwrap(), ran.state, "{tag}");
    }
}

#[test]
fn a_v1_state_line_still_reads_and_the_next_save_is_v2() {
    let v1 = "watchdog-state v1 phase=down since=100 missed=3 last_answer=40 last_probe=220 down_notified=1\n";
    let state = State::decode(v1).unwrap();
    assert_eq!(state.phase, Phase::Down);
    assert_eq!(state.pending, None, "a v1 line names no pending notice");
    assert!(state.encode().starts_with("watchdog-state v2 "));
    assert!(state.encode().ends_with(" pending=-\n"));
    // v1 never carried pending=, so a v1 line with one is damage.
    assert!(State::decode(&v1.replace('\n', " pending=-\n")).is_err());
    for (word, notice) in [("down", Notice::Down), ("back", Notice::Back)] {
        let line = state
            .encode()
            .replace("pending=-", &format!("pending={word}"));
        assert_eq!(State::decode(&line).unwrap().pending, Some(notice));
    }
    let err = State::decode(&state.encode().replace("pending=-", "pending=maybe")).unwrap_err();
    assert!(err.contains("pending=maybe"), "{err}");
    let err = State::decode(&state.encode().replace(" pending=-", "")).unwrap_err();
    assert!(err.contains("pending="), "{err}");
}

#[test]
fn probes_that_could_not_save_are_not_a_sleep_and_the_withheld_notice_stays_due() {
    // The stale-suspect rule read the state's last_probe alone, and a probe
    // whose save failed cannot move it: after two intervals of an unwritable
    // directory the first miss "expired", the log said the Mac had slept, the
    // run's refusal lost its `withheld` reason, and the DOWN after the
    // directory came back was one probe late. The lock file's stamp says the
    // probes ran.
    let dir = scratch("unsaved-not-asleep");
    let w = watch(&dir, None);
    let mut counter = Counter::default();
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
    let blocker = dir.join(format!("{STATE_FILE}.tmp"));
    std::fs::create_dir(&blocker).unwrap();
    for k in 2..12 {
        let refused = watch_once(&w, T0 + k * MINUTE, &down(), &mut counter).unwrap_err();
        assert!(
            refused.contains("withheld, not sent"),
            "minute {k}: {refused}"
        );
    }
    let stamp = std::fs::read_to_string(dir.join(LOCK_FILE)).unwrap();
    assert_eq!(stamp, format!("{ATTEMPT_HEAD} {:020}\n", T0 + 11 * MINUTE));
    let log = log_of(&dir);
    assert!(
        !log.iter().any(|l| word_of(l) == "suspect-expired"),
        "no probe was skipped, and the log must not say one was: {log:#?}"
    );
    assert!(
        log.iter().any(|l| word_of(l) == "suspect-kept"),
        "the stamp keeping the miss is named: {log:#?}"
    );
    std::fs::rename(&blocker, dir.join("blocker-moved-aside")).unwrap();
    let ran = watch_once(&w, T0 + 12 * MINUTE, &down(), &mut counter).unwrap();
    assert_eq!(
        ran.transition,
        Some(Transition::Down),
        "DOWN at the first probe that can be saved: {:?}",
        ran.lines
    );
    assert_eq!(
        ran.state.since,
        T0 + MINUTE,
        "the outage began at the first miss"
    );
    assert_eq!(counter.pages.len(), 1);
}

#[test]
fn the_lock_stamp_never_keeps_a_miss_across_a_real_sleep() {
    // Each stamp the rule must NOT believe: one older than the saved probe
    // (an earlier failure, since saved over), one in the future (a clock moved
    // back), and one that is not a stamp at all (named, read as none).
    let day = 24 * 60 * MINUTE;
    for (case, stamp) in [
        ("older", format!("{ATTEMPT_HEAD} {:020}\n", T0 - MINUTE)),
        ("future", format!("{ATTEMPT_HEAD} {:020}\n", T0 + 2 * day)),
        (
            "garbage",
            "not a stamp at all, and longer than one too\n".to_owned(),
        ),
        ("torn", format!("{ATTEMPT_HEAD} 0000")),
    ] {
        let dir = scratch(&format!("stamp-{case}"));
        let w = watch(&dir, None);
        let mut counter = Counter::default();
        watch_once(&w, T0, &up(), &mut counter).unwrap();
        watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
        std::fs::write(dir.join(LOCK_FILE), &stamp).unwrap();
        let login = watch_once(&w, T0 + day, &down(), &mut counter).unwrap();
        assert_eq!(
            login.transition,
            Some(Transition::Suspect),
            "{case}: {:?}",
            login.lines
        );
        assert!(
            login.lines.iter().any(|l| word_of(l) == "suspect-expired"),
            "{case}: {:?}",
            login.lines
        );
        let unreadable = login
            .lines
            .iter()
            .any(|l| word_of(l) == "attempt-stamp-unreadable");
        assert_eq!(
            unreadable,
            matches!(case, "garbage" | "torn"),
            "{case}: {:?}",
            login.lines
        );
        assert!(counter.pages.is_empty(), "{case}: no DOWN at login");
        if unreadable {
            assert!(
                login
                    .lines
                    .iter()
                    .any(|l| l.contains("rewritten as no stamp")),
                "{case}: {:?}",
                login.lines
            );
            let again = watch_once(&w, T0 + day + MINUTE, &up(), &mut counter).unwrap();
            assert!(
                !again
                    .lines
                    .iter()
                    .any(|l| word_of(l) == "attempt-stamp-unreadable"),
                "{case}: named once, not every minute: {:?}",
                again.lines
            );
        }
    }
    // The stamp round-trips through the lock file, at a fixed length.
    let dir = scratch("stamp-round-trip");
    let lock = std::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join(LOCK_FILE))
        .unwrap();
    assert_eq!(
        read_attempt(&lock),
        Attempt::None,
        "an empty lock is no stamp"
    );
    stamp_attempt(&lock, u64::MAX).unwrap();
    assert_eq!(read_attempt(&lock), Attempt::At(u64::MAX));
    stamp_attempt(&lock, 7).unwrap();
    assert_eq!(
        read_attempt(&lock),
        Attempt::At(7),
        "a shorter number overwrites whole"
    );
    assert_eq!(
        std::fs::metadata(dir.join(LOCK_FILE)).unwrap().len(),
        u64::try_from(ATTEMPT_BYTES).unwrap()
    );
    // Read-only lock handle: the stamp fails by name and is not hidden.
    let read_only = std::fs::File::open(dir.join(LOCK_FILE)).unwrap();
    let why = unsaved(&read_only, 9, "the save failed (invented)");
    assert!(why.starts_with("the save failed (invented); "), "{why}");
    assert!(why.contains("could not be recorded"), "{why}");
}

#[test]
fn a_suspect_from_long_ago_is_not_half_an_outage() {
    // B2: a miss at shutdown and a miss at the next login a day later are not
    // "two in a row"; two misses within two intervals still are.
    let dir = scratch("stale");
    let w = watch(&dir, None);
    let mut counter = Counter::default();
    watch_once(&w, T0, &up(), &mut counter).unwrap();
    watch_once(&w, T0 + MINUTE, &down(), &mut counter).unwrap();
    let day = 24 * 60 * MINUTE;
    let login = watch_once(&w, T0 + day, &down(), &mut counter).unwrap();
    assert_eq!(
        login.transition,
        Some(Transition::Suspect),
        "{:?}",
        login.lines
    );
    assert_eq!(login.state.missed, 1);
    assert_eq!(login.state.since, T0 + day, "the count starts again");
    assert_eq!(
        word_of(&login.lines[0]),
        "suspect-expired",
        "{:?}",
        login.lines
    );
    assert!(
        login.lines[0].contains("limit_seconds=120"),
        "{}",
        login.lines[0]
    );
    assert!(
        counter.pages.is_empty(),
        "no DOWN the moment the operator logs in"
    );
    let next = watch_once(&w, T0 + day + MINUTE, &down(), &mut counter).unwrap();
    assert_eq!(next.transition, Some(Transition::Down));
    assert_eq!(counter.pages.len(), 1);
    // The boundary, on the pure rule: exactly two intervals still count.
    let suspect = State {
        phase: Phase::Suspect,
        since: T0,
        missed: 1,
        last_answer: Some(T0 - MINUTE),
        last_probe: T0,
        down_notified: false,
        pending: None,
    };
    assert_eq!(expire_stale(&suspect, T0 + 2 * MINUTE, MINUTE), None);
    let expired = expire_stale(&suspect, T0 + 2 * MINUTE + 1, MINUTE).unwrap();
    assert_eq!(expired.phase, Phase::Up);
    assert_eq!(expired.missed, 0);
    assert_eq!(
        expired.last_answer,
        Some(T0 - MINUTE),
        "the last answer is kept"
    );
    // A stale answer after a long gap is no blip either: it is just up.
    assert_eq!(step(&expired, &up(), T0 + day).1, None);
    // Only a suspect expires; an outage stays an outage across any gap.
    for phase in [Phase::Fresh, Phase::Up, Phase::Down] {
        let other = State {
            phase,
            ..suspect.clone()
        };
        assert_eq!(expire_stale(&other, T0 + day, MINUTE), None, "{phase:?}");
    }
    // The verb is told the interval; the agent passes the one it runs at.
    let parsed = parse_watch(&[
        "http://127.0.0.1:8080/health",
        "--state-dir",
        "/w",
        "--interval-seconds",
        "300",
    ])
    .unwrap();
    assert_eq!(parsed.0.interval_seconds, 300);
    let parsed = parse_watch(&["http://127.0.0.1:8080/health", "--state-dir", "/w"]).unwrap();
    assert_eq!(parsed.0.interval_seconds, DEFAULT_INTERVAL_SECONDS);
    for bad in ["0", "9", "3601", "-1", "x"] {
        let mut out = String::new();
        let words = [
            "http://127.0.0.1:8080/health",
            "--state-dir",
            "/w",
            "--interval-seconds",
            bad,
        ];
        assert_eq!(command(&words, &mut out), MISUSED, "{bad}: {out}");
        assert!(out.contains("from 10 to 3600"), "{bad}: {out}");
    }
}

#[test]
fn a_port_with_a_sign_or_a_leading_zero_is_refused() {
    for port in ["+80", "0080", "080", "0", "-1", "8o", "65536", ""] {
        let url = format!("http://127.0.0.1:{port}/health");
        let refused = Target::parse(&url).unwrap_err();
        assert!(refused.contains("is not a port"), "{port}: {refused}");
    }
    for (port, expected) in [("1", 1), ("80", 80), ("8080", 8080), ("65535", 65_535)] {
        let url = format!("http://127.0.0.1:{port}/health");
        assert_eq!(Target::parse(&url).unwrap().addr().port(), expected);
    }
    let bracketed = Target::parse("http://[::1]:+80/health").unwrap_err();
    assert!(bracketed.contains("is not a port"), "{bracketed}");
}

#[test]
fn a_state_dir_on_the_store_drive_is_refused_however_it_is_spelt() {
    // A1-A5: the check was "under the store root, as written". It is now the
    // DEVICE: the store root's st_dev against the directory's (or its nearest
    // existing ancestor's), so a sibling on the same drive, a `..` back into
    // it and a symbolic link into it are all the store drive.
    let dir = scratch("drive");
    let store = dir.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let elsewhere = dir.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let link = dir.join("link-to-store");
    std::os::unix::fs::symlink(&store, &link).unwrap();
    let cli_bin = dir.join("cli");
    std::fs::write(&cli_bin, "").unwrap();
    let same_drive = [
        dir.join("sibling").join("watchdog"),
        elsewhere.join("..").join("store").join("watchdog"),
        link.join("watchdog"),
        elsewhere.clone(),
    ];
    for state in &same_drive {
        let words = [
            "watchdog",
            cli_bin.to_str().unwrap(),
            "http://127.0.0.1:8080/health",
            "--state-dir",
            state.to_str().unwrap(),
            "--store",
            store.to_str().unwrap(),
        ];
        let mut out = String::new();
        assert_eq!(
            launch_agent(&words, &mut out),
            MISUSED,
            "{}: {out}",
            state.display()
        );
        assert!(
            out.contains("is on the store drive") || out.contains("is inside the store root"),
            "{}: {out}",
            state.display()
        );
        // The verb itself refuses it too, so a hand-edited agent is caught.
        let mut out = String::new();
        let watch_words = [
            "http://127.0.0.1:8080/health",
            "--state-dir",
            state.to_str().unwrap(),
            "--store",
            store.to_str().unwrap(),
        ];
        assert_eq!(
            command(&watch_words, &mut out),
            MISUSED,
            "{}: {out}",
            state.display()
        );
    }
    // The app agent's --log-dir, on the same drive by a `..`: refused.
    let api = dir.join("api");
    std::fs::write(&api, "").unwrap();
    let sneaky_logs = elsewhere.join("..").join("store");
    let app_words = [
        api.to_str().unwrap(),
        "127.0.0.1:8080",
        "--store",
        store.to_str().unwrap(),
        "--log-dir",
        sneaky_logs.to_str().unwrap(),
    ];
    let refused = parse_app(&app_words).unwrap_err();
    assert!(refused.contains("--log-dir"), "{refused}");
    assert!(refused.contains("is on the store drive"), "{refused}");
    assert!(refused.contains("launchd could not write"), "{refused}");
    // Another device: /dev is its own file system on macOS and Linux alike.
    let off = Path::new("/dev/brutex-no-such-dir/watchdog");
    let (dev, at) = device_of(off).unwrap();
    assert_eq!(
        at,
        Path::new("/dev"),
        "read at the nearest existing ancestor"
    );
    assert_ne!(dev, device_of(&store).unwrap().0);
    assert_eq!(
        off_store("--state-dir", off, Some(&store)).unwrap(),
        Drive::Other
    );
    let agent = parse_watchdog(&[
        cli_bin.to_str().unwrap(),
        "http://127.0.0.1:8080/health",
        "--state-dir",
        off.to_str().unwrap(),
        "--store",
        store.to_str().unwrap(),
    ])
    .unwrap();
    assert_eq!(agent.unchecked, None, "checked, and off the drive");
    assert!(!render_watchdog(&agent).contains("NOT CHECKED"));
    assert_eq!(off_store("--state-dir", off, None).unwrap(), Drive::NoStore);
}

#[test]
fn an_agent_rendered_with_the_drive_away_says_the_drive_was_not_checked() {
    let dir = scratch("unchecked");
    let cli_bin = dir.join("cli");
    std::fs::write(&cli_bin, "").unwrap();
    let state = dir.join("watchdog");
    let missing = dir.join("Volumes").join("DRIVE").join("store");
    let agent = parse_watchdog(&[
        cli_bin.to_str().unwrap(),
        "http://127.0.0.1:8080/health",
        "--state-dir",
        state.to_str().unwrap(),
        "--store",
        missing.to_str().unwrap(),
    ])
    .unwrap();
    let why = agent.unchecked.clone().unwrap();
    assert!(why.contains("was NOT checked"), "{why}");
    let file = render_watchdog(&agent);
    let comment = file
        .split("<!-- ")
        .nth(1)
        .and_then(|rest| rest.split(" -->").next())
        .unwrap();
    assert!(comment.starts_with("NOT CHECKED: "), "{comment}");
    assert!(comment.contains("state-dir"), "{comment}");
    assert!(
        !comment.contains("--"),
        "an XML comment may not hold `--`: {comment}"
    );
    assert_eq!(file.matches("<!--").count(), 1);
    assert_eq!(xml_comment("a---b----c"), "\t<!-- NOT CHECKED: a-bc -->\n");
    // The verb at run time does not refuse an absent store: that is an outage
    // it reports, not a misuse.
    let parsed = parse_watch(&[
        "http://127.0.0.1:8080/health",
        "--state-dir",
        state.to_str().unwrap(),
        "--store",
        missing.to_str().unwrap(),
    ]);
    assert!(parsed.is_ok(), "{parsed:?}");
    // And the agent passes its interval to the verb.
    assert!(
        file.contains("<string>--interval-seconds</string>\n\t\t<string>60</string>"),
        "{file}"
    );
}

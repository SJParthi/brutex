//! The hand runs' own stop, and the seat refusal that names its holder
//! (WP-61; DB-B28, DB-B29, DB-B30).
//!
//! A hand run (`POST /pull/run`'s legs, a single `POST /pull/spot`) captured
//! the autopilot's stop generation: the autopilot's Pause aborted it
//! mid-universe and the run's own Stop moved nothing a leg checked, so Stop
//! bit only between legs (one 1min leg ran 9,300 s). These tests pin the
//! split: Pause stops the autopilot and not a hand run, Stop stops a hand run
//! (a `/pull/run` leg or a lone per-row pull) and not the autopilot, and a
//! 409 at the seat says who holds it.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failed fixture must fail the test by name, and a JSON field read \
              by key in an assertion panics with that key"
)]

use super::{Loaded, Site, pull_run_stop, pull_stop_json, seat_holder};
use crate::autopilot::{Control, Driver, InFlight};
use crate::pullrun::{FeedReport, Progress};
use axum::extract::State;

fn site(name: &str) -> Loaded {
    let root = crate::scratch::path(&format!("hand-stop-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("mkdir");
    Loaded::new(Site::load(&root.join("missing-masters"), &root))
}

fn cell(driver: Driver, feed: &str) -> InFlight {
    InFlight {
        instrument: String::from("NIFTY"),
        month: String::from("2018-01..2026-09"),
        timeframe: String::from("1min"),
        feed: feed.to_owned(),
        window: None,
        driver,
        run_since: std::time::Instant::now(),
        index: 12,
        of: 208,
        since: std::time::Instant::now(),
    }
}

#[test]
fn pause_stops_the_autopilot_and_not_a_hand_run_and_stop_the_reverse() {
    let control = Control::new();
    let autopilot = control.epoch_for(Driver::Autopilot);
    let hand = control.epoch_for(Driver::Hand);
    assert!(!control.stopped_for(Driver::Autopilot, autopilot));
    assert!(!control.stopped_for(Driver::Hand, hand));

    control.pause();
    assert!(
        control.stopped_for(Driver::Autopilot, autopilot),
        "Pause stops the autopilot"
    );
    assert!(
        control.stopped_for(Driver::Recovery, autopilot),
        "and a recovery, as before"
    );
    assert!(
        !control.stopped_for(Driver::Hand, hand),
        "Pause no longer aborts a hand run (DB-B29)"
    );

    let moved = control.stop_hand();
    assert_ne!(moved, hand);
    assert!(
        control.stopped_for(Driver::Hand, hand),
        "Stop reaches a hand run (DB-B28)"
    );
    // A hand run that starts after the Stop captures the new generation.
    let next = control.epoch_for(Driver::Hand);
    assert!(!control.stopped_for(Driver::Hand, next));
    // Stop moved nothing the autopilot checks.
    let after = control.epoch_for(Driver::Autopilot);
    let _ = control.stop_hand();
    assert!(!control.stopped_for(Driver::Autopilot, after));
}

#[tokio::test]
async fn stop_reaches_a_single_hand_pull_with_no_run_slot() {
    let site = site("single");
    site.autopilot
        .publish(|status| status.now = Some(cell(Driver::Hand, "zerodha")));
    assert!(site.autopilot.hand_in_flight());
    let before = site.autopilot.epoch_for(Driver::Hand);
    let (status, _, body) = pull_run_stop(State(Loaded::clone(&site))).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(json["stopping"], true, "{body}");
    assert_eq!(json["what"], "hand_pull", "{body}");
    assert!(
        json["when"]
            .as_str()
            .unwrap_or_default()
            .contains("next instrument"),
        "{body}"
    );
    assert!(site.autopilot.stopped_for(Driver::Hand, before));
    assert!(
        site.run.lock().expect("lock").is_none(),
        "no run slot was invented"
    );
}

#[tokio::test]
async fn stop_with_an_autopilot_cell_in_flight_stops_nothing() {
    let site = site("autopilot-cell");
    site.autopilot
        .publish(|status| status.now = Some(cell(Driver::Autopilot, "zerodha")));
    let autopilot = site.autopilot.epoch_for(Driver::Autopilot);
    let (_, _, body) = pull_run_stop(State(Loaded::clone(&site))).await;
    let json: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(json["stopping"], false, "{body}");
    assert_eq!(json["what"], "nothing", "{body}");
    assert!(!site.autopilot.stopped_for(Driver::Autopilot, autopilot));
}

#[tokio::test]
async fn stop_of_a_run_moves_the_hand_generation_too() {
    let site = site("run");
    *site.run.lock().expect("lock") = Some(Progress::claimed());
    let before = site.autopilot.epoch_for(Driver::Hand);
    let (_, _, body) = pull_run_stop(State(Loaded::clone(&site))).await;
    let json: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(json["what"], "run", "{body}");
    assert!(
        site.autopilot.stopped_for(Driver::Hand, before),
        "the leg in flight stops at its next instrument, not its next leg"
    );
}

#[test]
fn the_stop_answer_names_what_it_stopped() {
    let run: serde_json::Value = serde_json::from_str(&pull_stop_json(true, false)).expect("json");
    assert_eq!(run["what"], "run");
    let hand: serde_json::Value = serde_json::from_str(&pull_stop_json(false, true)).expect("json");
    assert_eq!(hand["what"], "hand_pull");
    let none: serde_json::Value =
        serde_json::from_str(&pull_stop_json(false, false)).expect("json");
    assert_eq!(none["stopping"], false);
    assert!(
        none["when"]
            .as_str()
            .unwrap_or_default()
            .contains("does not touch"),
        "{none}"
    );
}

#[test]
fn the_seat_refusal_names_its_holder() {
    let site = site("holder");
    let zerodha = pull::vendor::Feed::Zerodha;

    let mut claimed = Progress::claimed();
    claimed.feeds = vec![FeedReport {
        vendor: String::from("zerodha"),
        doing: String::from("Spot · 1 minute"),
        ..FeedReport::default()
    }];
    let generation = claimed.generation;
    *site.run.lock().expect("lock") = Some(claimed);
    let said = seat_holder(&site, zerodha);
    assert!(said.contains("hand run started on /ingest"), "{said}");
    assert!(said.contains(&format!("run {generation}")), "{said}");
    assert!(said.contains("Spot · 1 minute"), "{said}");
    assert!(
        said.contains("pausing the autopilot does not touch it"),
        "{said}"
    );
    *site.run.lock().expect("lock") = None;

    site.autopilot
        .publish(|status| status.now = Some(cell(Driver::Hand, "zerodha")));
    let said = seat_holder(&site, zerodha);
    assert!(
        said.contains("single hand pull (POST /pull/spot)"),
        "{said}"
    );
    assert!(
        said.contains("NIFTY at 1min, instrument 12 of 208"),
        "{said}"
    );

    site.autopilot
        .publish(|status| status.now = Some(cell(Driver::Autopilot, "zerodha")));
    let said = seat_holder(&site, zerodha);
    assert!(said.contains("held by the autopilot"), "{said}");
    assert!(said.contains("Pause it at /autopilot"), "{said}");

    // ANOTHER FEED'S CELL NAMES NOTHING ABOUT THIS FEED'S SEAT.
    site.autopilot
        .publish(|status| status.now = Some(cell(Driver::Hand, "gdfl")));
    site.autopilot.pause();
    let said = seat_holder(&site, zerodha);
    assert!(
        said.contains("No run this server publishes names the holder"),
        "{said}"
    );
    site.autopilot.resume();
    site.autopilot.publish(|status| status.now = None);
    let said = seat_holder(&site, zerodha);
    assert!(said.contains("The autopilot is running"), "{said}");
}

/// The instrument loop checks the stop of its OWN driver: a source-shape pin,
/// because driving `broker_run_by` needs a reachable broker, which no test may
/// open (`Site::load` sets `Broker::Refused`).
#[test]
fn the_instrument_loop_checks_its_own_drivers_stop() {
    let source = include_str!("server.rs");
    let body = source
        .split_once("pub(crate) async fn broker_run_by(")
        .expect("the loop exists")
        .1;
    let body = &body[..body.find("\n}\n").expect("its body ends")];
    assert!(body.contains("let epoch = site.autopilot.epoch_for(driver);"));
    assert!(body.contains("if site.autopilot.stopped_for(driver, epoch) {"));
    assert!(
        !body.contains("site.autopilot.stopped(epoch)"),
        "the autopilot's generation alone must not decide a hand run's stop"
    );
}

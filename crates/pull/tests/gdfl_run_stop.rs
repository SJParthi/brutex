//! BASE-COV-PULL-PS1 (D-7335): a run's stop as the shipped build asks it. The
//! test build's `StopControl` carries a seam that can ask itself to stop at a
//! chosen check; the shipped build's never trips, and only a test outside the
//! crate runs that build. Every run id is invented.

#![allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]

use pull::gdfl_run::stop::StopControl;

/// **THE SHIPPED STOP TRIPS ONLY ON ITS OWN RUN'S FILE**: with no stop file
/// it is not asked, counting each check; a stop file naming another run is
/// not its stop; the line naming its run is, and the request it kept is that
/// line's. Extremes at once: the first check, and a stop file that names a
/// neighbouring run.
#[test]
fn the_shipped_stop_trips_only_on_its_own_runs_file() {
    let dir = std::env::temp_dir().join(format!("brutex-ps1-stop-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("indices.stop");
    let control = StopControl::new(path.clone(), 41);
    assert!(!control.requested());
    assert_eq!(control.checks(), 1);
    std::fs::write(&path, "stop run=42 requested=8 by=cli\n").unwrap();
    assert!(!control.requested(), "another run's stop");
    std::fs::write(&path, "stop run=41 requested=9 by=cli\n").unwrap();
    assert!(control.requested());
    let kept = control.request().unwrap();
    assert_eq!((kept.run, kept.requested, kept.by.as_str()), (41, 9, "cli"));
    std::fs::remove_dir_all(dir).unwrap();
}

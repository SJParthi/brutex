//! BASE-COV-PULL: stop lines of any other shape, what a stop asks of a held
//! lock whose claim is not there yet, and the small doors of a stop flag
//! (Route B, recipe v4). Every run id is invented.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_fixtures::scratch;

#[test]
fn a_stop_line_of_any_other_shape_is_refused_by_name() {
    let good = StopRequest::parse("stop run=7 requested=8 by=cli\n").unwrap();
    assert_eq!((good.run, good.requested), (7, 8));
    assert_eq!(StopRequest::parse(&good.line()).unwrap(), good);
    assert!(
        StopRequest::parse("stop run=7")
            .unwrap_err()
            .contains("not a stop line")
    );
    for bad in [
        "stop run= requested=8 by=cli",
        "stop run=x requested=8 by=cli",
        "stop run=123456789012345678901 requested=8 by=cli",
        "stop run=99999999999999999999 requested=8 by=cli",
        "stop run=7 requested= by=cli",
        "stop runs=7 requested=8 by=cli",
    ] {
        assert!(
            StopRequest::parse(bad)
                .unwrap_err()
                .contains("is not a number"),
            "{bad}"
        );
    }
    assert!(
        StopRequest::parse("stop run=7 requested=8 by=nobody")
            .unwrap_err()
            .contains("by= is not one of")
    );
}

#[test]
fn a_stop_file_past_its_bound_or_not_text_is_no_stop() {
    let root = scratch("cov-stop-file");
    let path = root.join("a.stop");
    std::fs::write(&path, vec![b'x'; PROBE_READ + 1]).unwrap();
    assert_eq!(read_stop(&path), None);
    let bytes = root.join("b.stop");
    std::fs::write(&bytes, [0xFF, 0xFE]).unwrap();
    assert_eq!(read_stop(&bytes), None);
    assert_eq!(read_stop(&root.join("none")), None);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_stop_asks_who_is_allowed_to_ask() {
    let root = scratch("cov-stop-by");
    let refused = request_stop(&root, StopTarget::Run(7), "nobody", 1).unwrap_err();
    assert!(matches!(refused, StopRefusal::ByUnknown { .. }));
    let none = request_stop(&root, StopTarget::Kind("stocks"), "cli", 1).unwrap_err();
    assert!(matches!(none, StopRefusal::NotLive { .. }));
    std::fs::remove_dir_all(root).unwrap();
}

// ───── BASE-COV-PULL repair 2 (D-7334) ─────

/// A stop names the run it belongs to and the file it reads.
#[test]
fn a_stop_names_its_run_and_its_file() {
    let control = StopControl::new(PathBuf::from("/invented/imports/a.stop"), 41);
    assert_eq!(control.run(), 41);
    assert_eq!(control.path(), Path::new("/invented/imports/a.stop"));
}

/// A kind lock held while its claim is still being written: a stop of that
/// kind is refused by the lock's name (the run cannot be named yet), a stop
/// by run id passes it over, and a held lock whose claim does not parse is
/// passed over too, so both end not live (extremes at once: an empty claim
/// and a garbled one, on one v4 store).
#[test]
fn a_stop_of_a_kind_whose_claim_is_being_written_is_refused_by_its_lock() {
    let store = scratch("cov-stop-writing");
    std::fs::create_dir_all(store.join("manifest/v4/gdfl")).unwrap();
    let writing = locks::RunLocks::take(&store, "indices", 0).unwrap();
    let garbled = locks::RunLocks::take(&store, "stocks", 0).unwrap();
    std::fs::write(store.join(locks::kind_lock("stocks")), b"not a claim").unwrap();
    let refused = request_stop(&store, StopTarget::Kind("indices"), "cli", 1).unwrap_err();
    let StopRefusal::ClaimBeingWritten { lock } = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(lock, locks::kind_lock("indices"));
    let by_run = request_stop(&store, StopTarget::Run(7), "cli", 1).unwrap_err();
    assert!(matches!(by_run, StopRefusal::NotLive { .. }), "{by_run:?}");
    let of_stocks = request_stop(&store, StopTarget::Kind("stocks"), "cli", 1).unwrap_err();
    assert!(
        matches!(of_stocks, StopRefusal::NotLive { .. }),
        "{of_stocks:?}"
    );
    writing.release().unwrap();
    garbled.release().unwrap();
}

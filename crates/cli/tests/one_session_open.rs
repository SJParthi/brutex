//! Every copy of the 09:15 open, the IST offset and the regular session's
//! length is the one `pull::session` holds. D-3518 (ONEAUTH-20).
//!
//! The graph forces copies. `indicators` may name `vocab` alone (gate 22),
//! `runner` may not name `pull` or `store`, and `store` may not name `pull`,
//! so the open is defined in `pull::session`, `pull::calendar`, `store::path`
//! and `indicators`, and the session length in `pull::session`,
//! `pull::calendar` and `runner::synthetic`. `cli` names every one of those
//! crates, which makes it the one place they can be held to one answer, as
//! `resample_matches_fold.rs` holds `runner::resample` to `pull::fold`.
//!
//! Before D-3518 `indicators::orb` and `runner::resample` each kept a private
//! open and `runner::synthetic` spelled it as `(555 - 330)` minutes; none of the
//! three was tied to any other copy.

#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]

use pull::session::{
    BARS_PER_REGULAR_SESSION, IST_OFFSET_SECS, IstMoment, SESSION_CLOSE_MINUTE, SESSION_OPEN_MINUTE,
};

#[test]
fn every_copy_of_the_open_the_offset_and_the_length_is_the_sessions() {
    let open = i64::from(SESSION_OPEN_MINUTE);
    assert_eq!(i64::from(pull::calendar::OPEN_MINUTE), open);
    assert_eq!(
        i64::from(store::path::Timeframe::OPEN_MINUTES_PAST_IST_MIDNIGHT),
        open
    );
    assert_eq!(indicators::SESSION_OPEN_MINUTE, open);

    assert_eq!(store::path::IST_OFFSET_SECS, IST_OFFSET_SECS);
    assert_eq!(indicators::IST_OFFSET_MICROS, IST_OFFSET_SECS * 1_000_000);

    let length = usize::try_from(BARS_PER_REGULAR_SESSION).expect("a u32 fits");
    assert_eq!(usize::from(pull::calendar::FULL_BARS), length);
    assert_eq!(runner::synthetic::BARS_PER_SESSION, length);
    assert_eq!(
        SESSION_CLOSE_MINUTE - SESSION_OPEN_MINUTE,
        BARS_PER_REGULAR_SESSION
    );
}

/// `runner::synthetic` invents prices and no instant: every bar it makes is a
/// minute of a regular session as `pull::session` reads the clock.
#[test]
fn a_synthetic_session_is_a_regular_session_as_pull_reads_it() {
    let per = runner::synthetic::BARS_PER_SESSION;
    let bars = runner::synthetic::sessions(3);
    assert_eq!(bars.len(), 3 * per);
    for (n, bar) in bars.iter().enumerate() {
        assert_eq!(bar.ts_micros.rem_euclid(60_000_000), 0, "bar {n}");
        let at = IstMoment::from_epoch_secs(bar.ts_micros.div_euclid(1_000_000))
            .expect("an instant pull reads");
        assert!(at.in_regular_session(), "bar {n} is outside the session");
        let minute = u32::try_from(n % per).expect("fits");
        assert_eq!(at.minute_of_day(), SESSION_OPEN_MINUTE + minute, "bar {n}");
        let day = u32::try_from(n / per).expect("fits");
        assert_eq!(at.day().days_from_epoch(), day, "bar {n}");
    }
    let first = IstMoment::from_epoch_secs(runner::synthetic::IST_OPEN_UTC_MICROS / 1_000_000)
        .expect("the open");
    assert_eq!(first.minute_of_day(), SESSION_OPEN_MINUTE);
    assert_eq!(first.day().days_from_epoch(), 0);
}

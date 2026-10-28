//! BASE-COV-LIBS: refusal sentences of the store's public error types that no
//! other test reads: a format refusal's month figures, and a lock wait's two
//! ends (the host's refusal, with its source, and a holder never named).

use std::error::Error as _;
use std::time::Duration;

use store::flock::Impatient;
use store::format::FormatError;

#[test]
fn a_format_refusal_names_its_month_figures() {
    let too_many = FormatError::CounterExceedsMonth {
        n_valid: 44_641,
        slots: 44_640,
    };
    assert_eq!(
        too_many.to_string(),
        "n_valid claims 44641 records and the file's month holds at most 44640 on its rung's grid"
    );
    let outside = FormatError::RangeOutsideMonth {
        first_ts_micros: i64::MIN,
        last_ts_micros: i64::MAX,
    };
    assert_eq!(
        outside.to_string(),
        format!(
            "the header's range {}..={} (UTC micros) leaves the file's month",
            i64::MIN,
            i64::MAX
        )
    );
}

#[test]
fn a_lock_wait_says_which_end_it_reached() {
    let host = Impatient::Host(std::io::Error::from(std::io::ErrorKind::Unsupported));
    let said = host.to_string();
    assert!(said.starts_with("the host refused the lock: "), "{said}");
    assert_eq!(
        host.source().map(ToString::to_string),
        Some(std::io::Error::from(std::io::ErrorKind::Unsupported).to_string())
    );
    let held = Impatient::Held {
        bound: Duration::from_millis(250),
        waited: Duration::from_millis(251),
        attempts: 9,
        ahead: 0,
        holder: None,
    };
    let said = held.to_string();
    assert!(
        said.starts_with(
            "still held after waiting 251 ms of a 250 ms bound (9 attempt(s), 0 ahead"
        ),
        "{said}"
    );
    assert!(
        said.ends_with(" ms)"),
        "no holder is named when none is known: {said}"
    );
    assert!(held.source().is_none());
    let named = Impatient::Held {
        bound: Duration::ZERO,
        waited: Duration::ZERO,
        attempts: 1,
        ahead: u64::MAX,
        holder: Some("pid 7 (invented)".to_owned()),
    };
    assert!(named.to_string().ends_with("; pid 7 (invented)"));
}

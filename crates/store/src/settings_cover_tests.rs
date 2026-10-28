#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
//! BASE-COV-LIBS: the settings refusals' sentences, the name and day rules'
//! edges, the decoder's refusals and the reader's two caps, each driven by
//! the input that reaches it. Every value is invented.

use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

struct Root(PathBuf);

impl Root {
    fn new(name: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "brutex-settings-cover-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sealed(body: &str) -> String {
    format!("{body}crc32c {:08x}\n", crate::crc::crc32c(body.as_bytes()))
}

fn body(number: u64, rest: &str) -> String {
    format!(
        "{MAGIC} {VERSION}\ngeneration {number}\nchanged_at_micros 7\nchanged_by test\nreason \
         invented\nchanged \n{rest}"
    )
}

#[test]
fn every_refusal_reads_as_its_own_sentence() {
    let cases = [
        (
            Refusal::Invalid {
                key: None,
                why: "reason must be one line".to_owned(),
            },
            "reason must be one line".to_owned(),
        ),
        (
            Refusal::Stale {
                expected: 3,
                newest: 5,
            },
            "stale: the change was made against generation 3, and generation 5 is on disk; \
             read the settings again and repeat the change"
                .to_owned(),
        ),
        (
            Refusal::Unchanged,
            "nothing changed: every value is already in force".to_owned(),
        ),
        (
            Refusal::Io {
                path: PathBuf::from("/brutex-cover/settings"),
                kind: ErrorKind::StorageFull,
                why: "no space".to_owned(),
            },
            "the disk refused /brutex-cover/settings: StorageFull: no space".to_owned(),
        ),
    ];
    for (refusal, said) in cases {
        assert_eq!(refusal.to_string(), said);
    }
    let io = io_refusal(
        Path::new("/brutex-cover/x"),
        &std::io::Error::from(ErrorKind::PermissionDenied),
    );
    assert!(matches!(
        io,
        Refusal::Io {
            kind: ErrorKind::PermissionDenied,
            ..
        }
    ));
}

#[test]
fn only_a_twenty_digit_name_is_a_generation() {
    assert_eq!(number_of("00000000000000000001.settings"), Some(1));
    assert_eq!(
        number_of(&format!("{:020}.settings", u64::MAX)),
        Some(u64::MAX)
    );
    assert_eq!(number_of("1.settings"), None);
    assert_eq!(number_of("0000000000000000000x.settings"), None);
    assert_eq!(number_of("000000000000000000001.settings"), None);
    assert_eq!(number_of("00000000000000000001"), None);
    // Twenty digits past u64: no generation, never a wrap.
    assert_eq!(number_of("99999999999999999999.settings"), None);
    assert_eq!(number_of(""), None);
}

#[test]
fn a_day_the_month_does_not_have_is_refused_by_name() {
    assert!(civil_day("2018-02-30").unwrap_err().contains("no day 30"));
    assert!(civil_day("2018-04-00").unwrap_err().contains("no day 0"));
    assert!(civil_day("2019-02-29").unwrap_err().contains("no day 29"));
    assert_eq!(civil_day("2020-02-29"), Ok((2020, 2, 29)));
    assert_eq!(civil_day("2000-02-29"), Ok((2000, 2, 29)));
    assert!(civil_day("2100-02-29").unwrap_err().contains("no day 29"));
    assert_eq!(civil_day("2100-12-31"), Ok((2100, 12, 31)));
    assert!(civil_day("2018-13-01").unwrap_err().contains("no month 13"));
    // A wrong month and a wrong day at once: the month is named.
    assert!(civil_day("2018-00-00").unwrap_err().contains("no month 0"));
}

#[test]
fn a_feed_word_is_short_and_lowercase() {
    assert_eq!(check(Key::OpenFeed, "gdfl"), Ok("gdfl".to_owned()));
    for bad in ["GDFL", "gdfl1", "abcdefghijklmnopq"] {
        let refused = check(Key::OpenFeed, bad).unwrap_err();
        assert!(
            refused.to_string().contains("is not a feed's wire word"),
            "{bad}: {refused}"
        );
    }
    assert_eq!(
        check(Key::OpenFeed, "abcdefghijklmnop"),
        Ok("abcdefghijklmnop".to_owned())
    );
}

#[test]
fn a_small_floor_hints_a_percent_only_for_a_bare_percent_sized_number() {
    let hinted = disk_floor("40").unwrap_err();
    assert!(hinted.contains("for 40 percent write 40%"), "{hinted}");
    for bare in ["96", "1000", "0x"] {
        let refused = disk_floor(bare).unwrap_err();
        assert!(!refused.contains("percent write"), "{bare}: {refused}");
    }
    let with_unit = disk_floor("40GB").map(|floor| floor == Floor::Bytes(40_000_000_000));
    assert_eq!(with_unit, Ok(true));
}

#[test]
fn a_file_with_a_bare_field_another_version_or_a_key_set_twice() {
    let path = Path::new("/brutex-cover/settings/00000000000000000002.settings");
    // A field line with nothing after its name reads as empty.
    let bare = sealed(&format!(
        "{MAGIC} {VERSION}\ngeneration 2\nchanged_at_micros 7\nchanged_by test\nreason \
         invented\nchanged\n"
    ));
    let read = decode(2, path, &bare).expect("a bare changed line");
    assert!(read.changed.is_empty());
    let other =
        sealed(&body(2, "").replacen(&format!("{MAGIC} {VERSION}"), &format!("{MAGIC} 9"), 1));
    assert_eq!(
        decode(2, path, &other).map(|_| ()),
        Err(format!(
            "version 9 — this build reads version {VERSION} only"
        ))
    );
    let twice = sealed(&body(2, "set open.feed gdfl\nset open.feed gdfl\n"));
    assert_eq!(
        decode(2, path, &twice).map(|_| ()),
        Err("open.feed is set twice".to_owned())
    );
    let once = sealed(&body(2, "set open.feed gdfl\n"));
    assert_eq!(
        decode(2, path, &once).map(|read| read.settings.get(Key::OpenFeed).map(str::to_owned)),
        Ok(Some("gdfl".to_owned()))
    );
}

#[test]
fn a_note_past_its_bytes_is_refused_before_anything_is_written() {
    let long = "x".repeat(MAX_NOTE_BYTES + 1);
    for (by, reason) in [(long.as_str(), "ok"), ("ok", long.as_str())] {
        let change = Change {
            expected: 0,
            set: vec![(Key::OpenFeed, "gdfl".to_owned())],
            unset: Vec::new(),
            changed_by: by.to_owned(),
            reason: reason.to_owned(),
            at_micros: 1,
        };
        assert!(matches!(
            apply(&Settings::default(), &change),
            Err(Refusal::Invalid { key: None, why }) if why.contains("1 to 240 bytes")
        ));
    }
    let exact = "x".repeat(MAX_NOTE_BYTES);
    let change = Change {
        expected: 0,
        set: vec![(Key::OpenFeed, "gdfl".to_owned())],
        unset: Vec::new(),
        changed_by: exact.clone(),
        reason: exact,
        at_micros: 1,
    };
    assert!(apply(&Settings::default(), &change).is_ok());
}

#[test]
fn a_first_listing_stops_after_its_fault_cap_and_names_it() {
    let root = Root::new("faults");
    std::fs::create_dir_all(dir(&root.0)).unwrap();
    for number in 1..=(MAX_FAULTS as u64 + 3) {
        std::fs::write(path_of(&root.0, number), b"not a settings file").unwrap();
    }
    let mut reader = Reader::new(&root.0);
    reader.refresh().expect("a listing");
    assert_eq!(reader.faults().len(), MAX_FAULTS);
    assert_eq!(
        reader.stopped(),
        Some(
            format!("{MAX_FAULTS} newest generations did not read; no older one was tried")
                .as_str()
        )
    );
    assert!(reader.newest().is_none());
    // The faults are the NEWEST ones, newest first.
    assert_eq!(reader.faults()[0].generation, MAX_FAULTS as u64 + 3);
}

#[test]
fn a_refresh_steps_at_most_its_cap_and_names_the_rest() {
    let root = Root::new("step");
    std::fs::create_dir_all(dir(&root.0)).unwrap();
    let mut reader = Reader::new(&root.0);
    reader.refresh().expect("an empty listing");
    assert_eq!(reader.highest(), 0);
    for number in 1..=(MAX_STEP + 2) {
        std::fs::write(path_of(&root.0, number), b"x").unwrap();
    }
    reader.refresh().expect("a step");
    assert_eq!(reader.highest(), MAX_STEP);
    assert_eq!(
        reader.stopped(),
        Some(
            format!("{MAX_STEP} new generations read in one refresh; the rest wait for the next")
                .as_str()
        )
    );
    reader.refresh().expect("the rest");
    assert_eq!(reader.highest(), MAX_STEP + 2);
    assert_eq!(reader.stopped(), None);
}

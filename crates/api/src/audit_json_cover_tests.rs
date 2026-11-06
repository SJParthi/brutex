//! Rows of `audit_json.rs` no test reached before BASE-COV-API repair 3
//! (shard AS1, D-8661): the attribution of a pull journal's records to feeds
//! at its edges (runs that share an ordinal, records before the first run, a
//! file that ends early, a record that will not decode, a run record that
//! will not read), and the sentence each feed's scope says. Every record is
//! invented: sources and windows name no vendor row.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a whole-file test module; fixtures are bounded and a failed assertion is the test failing"
)]

use super::*;

/// An attribution of `runs` over `scanned` records, nothing else held.
fn attributed(scanned: u64, runs: &[(u64, Option<Vendor>)]) -> Attribution {
    Attribution {
        scanned,
        runs: runs.to_vec(),
        ..Attribution::default()
    }
}

/// A journal at a scratch root of its own holding `records` refused-run
/// records, each a whole 256-byte image.
fn journal_of(name: &str, records: usize) -> (std::path::PathBuf, audit::Journal) {
    let root = crate::scratch::path(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let journal = audit::Journal::at(&root);
    for n in 0..records {
        journal
            .append(&audit::Record::refused(
                audit::Scope::Spot,
                audit::Outcome::Refused,
                std::time::SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(1_000 + u64::try_from(n).unwrap()),
                "audit-json-cover",
                "an invented refusal",
            ))
            .expect("an append");
    }
    (root, journal)
}

/// **A JOURNAL'S RECORDS ARE ATTRIBUTED TO THEIR RUNS AT EVERY EDGE**
/// (BASE-COV-API repair 3, D-8661): `Attribution`'s spans, counts, pages and
/// last runs. Extremes: no run and nothing scanned (no span); records before
/// the first run (a span of no vendor); two runs at one ordinal (the older
/// one names nothing); a run past what was scanned (no span); a total below
/// what was scanned (cut to the total); a page past every record of the feed
/// (empty); last runs whose records read; and, at once, a run past the
/// scanned records beside records before the first run, and last runs whose
/// run record lies past the file (refused by name, never an empty list); out
/// of the box, a feed that holds no run at all.
#[test]
fn a_journals_records_are_attributed_to_their_runs_at_every_edge() {
    let zerodha = Some(Vendor::Zerodha);
    assert!(attributed(0, &[]).spans(0).is_empty(), "nothing at all");
    assert_eq!(
        attributed(3, &[]).spans(3),
        vec![(0, 3, None)],
        "records before any run are a span of no vendor"
    );
    assert_eq!(
        attributed(10, &[(5, zerodha), (5, None)]).spans(10),
        vec![(5, 10, None), (0, 5, None)],
        "the older of two runs at one ordinal holds no record of its own"
    );
    assert_eq!(
        attributed(4, &[(9, zerodha)]).spans(4),
        vec![(0, 4, None)],
        "a run past the scanned records holds none of them"
    );
    assert_eq!(
        attributed(10, &[(2, zerodha)]).spans(6),
        vec![(2, 6, zerodha), (0, 2, None)],
        "a total below the scan cuts the newest span"
    );
    let mixed = attributed(10, &[(2, zerodha), (6, Some(Vendor::Dhan))]);
    assert_eq!(mixed.held(Vendor::Zerodha, 10), (4, 1));
    assert_eq!(mixed.held(Vendor::Dhan, 10), (4, 1));
    assert_eq!(mixed.unattributed(10), (2, 0));
    assert_eq!(
        mixed.held(Vendor::Groww, 10),
        (0, 0),
        "a feed that holds no run holds no record"
    );

    let (root, journal) = journal_of("audit-json-cover-pages", 6);
    let six = attributed(6, &[(1, zerodha), (4, zerodha)]);
    assert!(
        six.page(&journal, Vendor::Zerodha, 6, 50, 200)
            .expect("a page")
            .is_empty(),
        "a page past every record of the feed is empty"
    );
    let last = six
        .last_runs(&journal, Vendor::Zerodha, 6)
        .expect("the run records read");
    assert_eq!(
        last.iter().map(|(_, after)| *after).collect::<Vec<_>>(),
        vec![1, 2],
        "the newest run first, each with the records after it"
    );
    let past = attributed(9, &[(8, zerodha)]);
    let why = past
        .last_runs(&journal, Vendor::Zerodha, 9)
        .expect_err("a run record past the file is refused");
    assert!(why.contains("cannot read"), "{why}");
    let _ = std::fs::remove_dir_all(root);
}

/// **A JOURNAL THAT ENDS EARLY IS NAMED, AND A RECORD THAT WILL NOT DECODE IS
/// COUNTED, NEVER DROPPED** (BASE-COV-API repair 3, D-8661):
/// `Attribution::advance`. Extremes: nothing to read past what was scanned
/// (no file is opened); a file of one whole run record; one record that will
/// not decode; a file that ends half a record into the second of two; a file
/// that is absent; and, at once, a record that will not decode followed by a
/// file that ends early (counted, then refused); out of the box, a scan asked
/// to go backwards.
#[test]
fn a_journal_that_ends_early_is_named_and_a_record_that_will_not_decode_counted() {
    let (root, journal) = journal_of("audit-json-cover-advance", 1);
    let path = journal.path.clone();
    let mut nothing = attributed(1, &[]);
    nothing
        .advance(&root.join("absent"), 1)
        .expect("nothing past the scan is read");
    nothing
        .advance(&root.join("absent"), 0)
        .expect("a scan asked to go backwards reads nothing");
    let mut whole = attributed(0, &[]);
    whole.advance(&path, 1).expect("one whole record");
    assert_eq!((whole.scanned, whole.faults), (1, 0));

    let broken = root.join("broken.journal");
    let mut bytes = vec![0xee; audit::RECORD_LEN];
    bytes.extend_from_slice(&[0xee; audit::RECORD_LEN / 2]);
    std::fs::write(&broken, &bytes).expect("a broken journal");
    let mut read = attributed(0, &[]);
    let why = read.advance(&broken, 2).expect_err("it ends early");
    assert!(why.contains("the file ended before record 1 of 2"), "{why}");
    assert_eq!(
        (read.scanned, read.faults),
        (1, 1),
        "the undecodable record is counted, then the end refused"
    );
    let mut absent = attributed(0, &[]);
    let why = absent
        .advance(&root.join("absent.journal"), 1)
        .expect_err("an absent file is refused");
    assert!(why.contains("absent.journal"), "{why}");
    let _ = std::fs::remove_dir_all(root);
}

/// **EACH FEED'S SCOPE IS SAID IN ITS OWN SENTENCE** (BASE-COV-API repair 3,
/// D-8661): `holds_sentence` for every vendor this build knows, each sentence
/// naming its own feed: GDFL's imports, an HTTP feed's endpoint, and a feed
/// that is never pulled.
#[test]
fn each_feeds_scope_is_said_in_its_own_sentence() {
    let mut kinds = std::collections::BTreeSet::new();
    for feed in Vendor::ALL {
        let said = holds_sentence(feed);
        assert!(said.contains(feed.as_str()), "{said}");
        kinds.insert(if said.contains("imports/gdfl.journal") {
            "imports"
        } else if said.contains("by its endpoint") {
            "http"
        } else {
            assert!(said.contains("never pulled"), "{said}");
            "never"
        });
    }
    assert_eq!(
        kinds.into_iter().collect::<Vec<_>>(),
        ["http", "imports", "never"],
        "every clause is some feed's"
    );
}

/// **A RECORD THAT WILL NOT DECODE IS AN OBJECT THAT SAYS SO, AND A WHOLE
/// REASON IS SERVED OR ITS FAULT NAMED** (BASE-COV-API repair 3, D-8661):
/// `run`, over entries made as `Journal::page` makes them. Extremes: a record
/// of garbage bytes (its ordinal and its fault, nothing else); a record that
/// names no whole reason (both null); one whose whole reason read; one whose
/// notes file would not read; the first ordinal and the largest; and, at
/// once, the largest ordinal on a record whose notes would not read; out of
/// the box, a whole reason holding a quote and a line break, escaped.
#[test]
fn a_record_that_will_not_decode_says_so_and_a_whole_reason_is_served_or_named() {
    let fault = audit::Record::decode(&[0xee; audit::RECORD_LEN]).expect_err("garbage");
    let mut out = String::new();
    run(
        &audit::Entry {
            ordinal: 0,
            decoded: Err(fault),
            whole_note: None,
        },
        &mut out,
    );
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("one object");
    assert_eq!(parsed["ordinal"], 0, "{out}");
    assert!(parsed["fault"].is_string(), "{out}");
    assert_eq!(
        parsed.as_object().map(serde_json::Map::len),
        Some(2),
        "{out}"
    );

    let record = audit::Record::refused(
        audit::Scope::Spot,
        audit::Outcome::Refused,
        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000),
        "audit-json-cover",
        "an invented refusal",
    );
    for (ordinal, whole_note, whole, error) in [
        (1, None, serde_json::Value::Null, serde_json::Value::Null),
        (
            2,
            Some(Ok("the whole reason".to_owned())),
            serde_json::json!("the whole reason"),
            serde_json::Value::Null,
        ),
        (
            3,
            Some(Err("the notes file would not read".to_owned())),
            serde_json::Value::Null,
            serde_json::json!("the notes file would not read"),
        ),
        (
            u64::MAX,
            Some(Err("the notes file would not read".to_owned())),
            serde_json::Value::Null,
            serde_json::json!("the notes file would not read"),
        ),
        (
            4,
            Some(Ok("a \"quoted\"\nreason".to_owned())),
            serde_json::json!("a \"quoted\"\nreason"),
            serde_json::Value::Null,
        ),
    ] {
        let mut out = String::new();
        run(
            &audit::Entry {
                ordinal,
                decoded: Ok(record.clone()),
                whole_note,
            },
            &mut out,
        );
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("one object");
        assert_eq!(parsed["ordinal"].as_u64(), Some(ordinal), "{out}");
        assert!(parsed["fault"].is_null(), "{out}");
        assert_eq!(parsed["note_whole"], whole, "{out}");
        assert_eq!(parsed["note_whole_error"], error, "{out}");
    }
}

/// What `body` answers for `site` and `feed` at `page`, as JSON.
fn body_of(site: &Site, feed: Vendor, page: usize) -> serde_json::Value {
    let today = Day::new(2026, 11, 6).expect("a day");
    serde_json::from_str(&body(
        site,
        feed,
        page,
        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_793_900_000),
        Ok(today),
    ))
    .expect("JSON")
}

/// **A TORN OR UNREADABLE JOURNAL IS SAID ON THE PAGE, A NUMBERED PAGE IS READ
/// IN SILENCE, AND A CENSUS FILE THAT CANNOT BE STAT'ED IS NAMED**
/// (BASE-COV-API repair 3, D-8661). `body` over a site's journal, the route
/// with a numbered page, and `committed_at` over census paths. Extremes: a
/// journal of three whole records and five bytes past them (`TORN WRITE`, its
/// three records still counted); a folder where the journal goes (its read
/// refused by name in the scope, the newest runs and the page, no run
/// listed); a page asked as a number through the route
/// (answered 200); a page asked as a word (answered too, the word ignored); a
/// census path under a file (`could not be stat'ed`, no commit time); a
/// census path inside a symbolic link loop (the same); no census file at all
/// (`no census file at`); and, at once, the torn journal asked for a page
/// past its last (clamped to the last, the trouble still said), and a census
/// path under a file that is also named absent (stat'ed, refused, no time);
/// out of the box, a journal of five bytes and no whole record.
#[tokio::test]
async fn a_torn_journal_is_said_a_numbered_page_is_silent_and_an_unstatable_census_is_named() {
    let (root, journal) = journal_of("audit-json-torn", 3);
    let site = Site::load(&root.join("missing-masters"), &root);
    {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&journal.path)
            .expect("the journal");
        file.write_all(&[0xAB; 5])
            .expect("five bytes past the records");
    }
    for page in [0, 99] {
        let answer = body_of(&site, Vendor::Zerodha, page);
        let scope = &answer["journal"];
        assert_eq!(scope["records"], 3, "{answer}");
        assert_eq!(
            scope["page"], 0,
            "page {page} clamped to the last: {answer}"
        );
        assert!(
            scope["trouble"]
                .as_str()
                .is_some_and(|why| why.starts_with("TORN WRITE — 5 byte(s)")),
            "{answer}"
        );
    }
    let loaded: crate::server::Loaded = std::sync::Arc::new(site);
    for query in ["feed=zerodha&page=2", "feed=zerodha&page=two"] {
        let uri: axum::http::Uri = format!("/audit.json?{query}").parse().expect("a uri");
        let (status, _, answer) =
            audit_json(axum::extract::State(std::sync::Arc::clone(&loaded)), uri).await;
        assert_eq!(status, axum::http::StatusCode::OK, "{query}: {answer}");
    }

    // FIVE BYTES AND NO WHOLE RECORD; A FOLDER WHERE THE JOURNAL GOES.
    let (bare, _none) = journal_of("audit-json-five-bytes", 0);
    let path = audit::Journal::at(&bare).path;
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("mkdir");
    std::fs::write(&path, [0xCD; 5]).expect("five bytes");
    let five = body_of(
        &Site::load(&bare.join("missing-masters"), &bare),
        Vendor::Zerodha,
        0,
    );
    assert_eq!(five["journal"]["records"], 0, "{five}");
    assert!(
        five["journal"]["trouble"]
            .as_str()
            .is_some_and(|why| why.starts_with("TORN WRITE — 5 byte(s)")),
        "{five}"
    );
    std::fs::remove_file(&path).expect("removed");
    std::fs::create_dir_all(&path).expect("a folder at the journal's name");
    // A FOLDER IS NO JOURNAL: its read is refused, and the refusal is said
    // in the scope, the newest runs and the page, never an empty list alone.
    let folder = body_of(
        &Site::load(&bare.join("missing-masters"), &bare),
        Vendor::Zerodha,
        0,
    );
    for (field, said) in [
        (
            "feed_scope.error",
            &folder["journal"]["feed_scope"]["error"],
        ),
        ("last_runs_error", &folder["last_runs_error"]),
        ("runs_error", &folder["runs_error"]),
    ] {
        assert!(
            said.as_str()
                .is_some_and(|why| why.contains("Is a directory")),
            "{field}: {folder}"
        );
    }
    assert_eq!(folder["last_runs"], serde_json::json!([]), "{folder}");
    assert_eq!(folder["runs"], serde_json::json!([]), "{folder}");

    // A CENSUS FILE THAT CANNOT BE STAT'ED.
    let plain = bare.join("a-file");
    std::fs::write(&plain, b"a file").expect("a file");
    let looped = bare.join("loop");
    std::os::unix::fs::symlink(&looped, &looped).expect("a link to itself");
    let census = |path: std::path::PathBuf| crate::census::VendorCensus {
        vendor: Vendor::Gdfl,
        path,
        state: crate::census::Census::Absent,
    };
    for (path, why) in [
        (plain.join("gdfl.man"), "could not be stat'ed"),
        (looped.join("gdfl.man"), "could not be stat'ed"),
        (bare.join("no-such.man"), "no census file at"),
    ] {
        let committed = committed_at(&census(path.clone()));
        assert_eq!(committed.at, None, "{path:?}");
        assert_eq!(committed.basis, "none", "{path:?}");
        assert!(
            committed
                .note
                .as_deref()
                .is_some_and(|note| note.contains(why)),
            "{path:?}: {:?}",
            committed.note
        );
    }
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&bare);
}

//! Rows of `detail.rs` no test reached before BASE-COV-API repair 3 (shard
//! AS1, D-8661): the scan pool's own entry, a path the preflight cannot
//! measure, a 503 that already says when to come back or whose body will not
//! read, the empty cached handle, and a parent or ledger that will not open.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a whole-file test module; fixtures are bounded and a failed assertion is the test failing"
)]

use super::*;

/// A body that says how long it is and then fails to read: what a 503 from
/// a stream that broke mid-answer looks like to [`count_route`].
struct Broken;

impl http_body::Body for Broken {
    type Data = axum::body::Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        std::task::Poll::Ready(Some(Err(std::io::Error::other("a body cut short"))))
    }

    fn size_hint(&self) -> http_body::SizeHint {
        http_body::SizeHint::with_exact(12)
    }
}

/// A 503 carrying `body`, and `retry` as its `Retry-After` when given.
fn unavailable(body: axum::body::Body, retry: Option<&'static str>) -> axum::response::Response {
    let mut response = axum::response::Response::new(body);
    *response.status_mut() = axum::http::StatusCode::SERVICE_UNAVAILABLE;
    if let Some(retry) = retry {
        response.headers_mut().insert(
            axum::http::header::RETRY_AFTER,
            axum::http::HeaderValue::from_static(retry),
        );
    }
    response
}

/// The body of `response`, read whole.
async fn text_of(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("a body that reads");
    String::from_utf8(bytes.to_vec()).expect("UTF-8")
}

/// **A 503 IS READ ONCE, AND WHAT IT ALREADY SAYS IS KEPT** (BASE-COV-API
/// repair 3, D-8661): `ask_again_when_being_written`, the arm of
/// `count_route` that reads a 503's body. Extremes: a being-written 503 with
/// no `Retry-After`, given one of 1; one that already carries its own (7),
/// kept as it is; a body whose length is known and which fails to read,
/// replaced by a sentence that says so and never counted being written; a
/// body of exactly the bound, read; one byte past it, left unread; an empty
/// 503; a 200, never looked at; a 503 of other words; and, at once, a
/// being-written body that already carries `Retry-After` and is exactly at
/// the bound, and a broken body that already carries one (kept, the sentence
/// in place of the body); out of the box, a being-written field inside a
/// body that is not UTF-8 around it, which is not read as being written.
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "every arm of the one bounded read beside the others it is read against"
)]
async fn a_503_is_read_once_and_what_it_already_says_is_kept() {
    let said = format!(r#"{{"error":"a writer holds the month",{BEING_WRITTEN_FIELD}}}"#);
    let retry_of = |response: &axum::response::Response| {
        response
            .headers()
            .get(axum::http::header::RETRY_AFTER)
            .map(|value| value.to_str().expect("text").to_owned())
    };

    let (plain, written) =
        ask_again_when_being_written(unavailable(axum::body::Body::from(said.clone()), None)).await;
    assert!(written);
    assert_eq!(retry_of(&plain).as_deref(), Some("1"));
    assert_eq!(text_of(plain).await, said);

    let (kept, written) =
        ask_again_when_being_written(unavailable(axum::body::Body::from(said.clone()), Some("7")))
            .await;
    assert!(written);
    assert_eq!(retry_of(&kept).as_deref(), Some("7"), "its own is kept");

    for retry in [None, Some("9")] {
        let (broken, written) =
            ask_again_when_being_written(unavailable(axum::body::Body::new(Broken), retry)).await;
        assert!(!written, "a body that did not read is not being written");
        assert_eq!(retry_of(&broken).as_deref(), retry);
        let body = text_of(broken).await;
        assert!(body.contains("this 503's own body did not read"), "{body}");
        assert!(body.contains("a body cut short"), "{body}");
    }

    // AT THE BOUND, AND ONE PAST IT.
    let pad = BEING_WRITTEN_BODY_MAX - said.len();
    let at_bound = format!("{said}{}", " ".repeat(pad));
    assert_eq!(at_bound.len(), BEING_WRITTEN_BODY_MAX);
    let (read, written) = ask_again_when_being_written(unavailable(
        axum::body::Body::from(at_bound.clone()),
        Some("3"),
    ))
    .await;
    assert!(written, "a body of exactly the bound is read");
    assert_eq!(retry_of(&read).as_deref(), Some("3"));
    let past = format!("{at_bound} ");
    let (unread, written) =
        ask_again_when_being_written(unavailable(axum::body::Body::from(past.clone()), None)).await;
    assert!(!written, "one byte past the bound is not read");
    assert_eq!(retry_of(&unread), None);
    assert_eq!(text_of(unread).await, past);

    // EMPTY, ANOTHER STATUS, OTHER WORDS, AND NOT UTF-8.
    let (empty, written) =
        ask_again_when_being_written(unavailable(axum::body::Body::empty(), None)).await;
    assert!(!written);
    assert_eq!(text_of(empty).await, "");
    let mut ok = axum::response::Response::new(axum::body::Body::from(said.clone()));
    *ok.status_mut() = axum::http::StatusCode::OK;
    let (ok, written) = ask_again_when_being_written(ok).await;
    assert!(!written);
    assert_eq!(retry_of(&ok), None);
    let (other, written) = ask_again_when_being_written(unavailable(
        axum::body::Body::from(r#"{"error":"the census will not read"}"#),
        None,
    ))
    .await;
    assert!(!written);
    assert_eq!(retry_of(&other), None);
    let mut bytes = vec![0xff, 0xfe];
    bytes.extend_from_slice(said.as_bytes());
    let (garbled, written) =
        ask_again_when_being_written(unavailable(axum::body::Body::from(bytes.clone()), None))
            .await;
    assert!(
        !written,
        "a body that is not UTF-8 is not read as being written"
    );
    let read_back = axum::body::to_bytes(garbled.into_body(), usize::MAX)
        .await
        .expect("bytes");
    assert_eq!(read_back.to_vec(), bytes, "and its bytes are kept");
}

/// **THE SCAN POOL'S OWN ENTRY ANSWERS, A PATH THAT CANNOT BE MEASURED IS
/// REFUSED BY NAME, AND A HANDLE CACHE STARTS EMPTY** (BASE-COV-API repair 3,
/// D-8661). `run_scan` had no caller in the workspace (L-BCA-7); it is the
/// scan pool's entry, so it is asked here directly. Extremes: a scan that
/// answers a number; one that answers nothing; a preflight path under a file,
/// which cannot be measured; a preflight path that is absent, left to its
/// reader; an empty preflight list; a default cache, which opens fresh; that
/// cache asked again for the same root, which refreshes; and, at once, a
/// preflight of an absent path beside one under a file (the second refuses),
/// and a default cache whose refresh refuses and whose reopen answers; out of
/// the box, a preflight path that is a folder.
#[tokio::test]
async fn the_scan_entry_answers_and_a_path_that_cannot_be_measured_is_refused() {
    assert_eq!(run_scan(|| 7).await, Ok(7));
    assert_eq!(run_scan(|| ()).await, Ok(()));

    let root = crate::scratch::path("detail-cover-preflight");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let file = root.join("a-file");
    std::fs::write(&file, b"not a folder").expect("a file");
    let under = file.join("runs.bin");
    let why = preflight([("runs.bin", under.as_path())]).expect_err("refused");
    assert!(
        why.contains("runs.bin at")
            && why.contains("could not be measured before its bounded read"),
        "{why}"
    );
    assert_eq!(
        preflight([("absent", root.join("absent").as_path())]),
        Ok(())
    );
    assert_eq!(preflight(std::iter::empty::<(&str, &Path)>()), Ok(()));
    let why = preflight([
        ("absent", root.join("absent").as_path()),
        ("runs.bin", under.as_path()),
    ])
    .expect_err("the second refuses");
    assert!(why.starts_with("runs.bin at"), "{why}");
    assert_eq!(
        preflight([("folder", root.as_path())]),
        Ok(()),
        "a folder's length is read"
    );

    let cache: Cached<u8> = Cached::default();
    assert_eq!(cache.with(&root, || Ok(5), |_| Ok(()), |held| *held), Ok(5));
    assert_eq!(
        cache.with(&root, || Ok(9), |_| Ok(()), |held| *held),
        Ok(5),
        "the same root refreshes the handle it holds"
    );
    assert_eq!(
        cache.with(&root, || Ok(9), |_| Err("stale".to_owned()), |held| *held),
        Ok(9),
        "a refused refresh reopens"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A PARENT OR LEDGER THAT WILL NOT OPEN IS REFUSED, NEVER READ AS
/// ABSENT** (BASE-COV-API repair 3, D-8661). Extremes: a ledger of garbage
/// bytes; a ledger path under a file; an empty ledger (absent, `None`); no
/// ledger at all (`None`); a parent of garbage bytes; and, at once, a
/// garbage ledger asked twice (refused both times, nothing cached); out of
/// the box, a ledger that is a folder.
#[test]
fn a_parent_or_ledger_that_will_not_open_is_refused() {
    let identity = [0x5a; 32];
    let root = crate::scratch::path("detail-cover-ledger");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    assert_eq!(recorded_underlying(&root, &identity), Ok(None), "no ledger");
    let ledger = cli::results::Results::path(&root);
    std::fs::create_dir_all(ledger.parent().expect("a parent")).expect("results");
    std::fs::write(&ledger, b"").expect("an empty ledger");
    assert_eq!(recorded_underlying(&root, &identity), Ok(None), "empty");
    std::fs::write(&ledger, [0xa5; 512]).expect("a garbage ledger");
    for _ in 0..2 {
        assert!(recorded_underlying(&root, &identity).is_err(), "garbage");
    }
    assert!(
        committed_receipt(&root, &identity).is_err(),
        "the parent is the same garbage"
    );
    let blocked = crate::scratch::path("detail-cover-ledger-blocked");
    let _ = std::fs::remove_dir_all(&blocked);
    std::fs::create_dir_all(&blocked).expect("a scratch root");
    std::fs::write(blocked.join("results"), b"a file where the folder goes").expect("a file");
    let why = recorded_underlying(&blocked, &identity).expect_err("under a file");
    assert!(why.contains("ledger path cannot be inspected"), "{why}");
    let folder = crate::scratch::path("detail-cover-ledger-folder");
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(cli::results::Results::path(&folder)).expect("a folder");
    assert!(recorded_underlying(&folder, &identity).is_err(), "a folder");
    for dir in [root, blocked, folder] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

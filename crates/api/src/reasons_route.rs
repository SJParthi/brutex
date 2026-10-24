//! `GET /reasons.json`: the reasons dictionary (READ-02, R4-05).
//!
//! Every reason code a page shows, with its plain sentence, what a person does
//! and the button or step that does it: `pull::reasons::TABLE`, the one table,
//! served whole so `web/src/lib/reasons.js` can title a code without a copy of
//! its own.
//!
//! # Once per build, from memory (D-TBD-R4-05-3)
//!
//! The table is compiled in, so its answer cannot change while the process
//! runs. The first request builds the document ([`pull::reasons::body`], which
//! logs `pull.reasons` / `reasons table built` once) and every later request
//! answers the same bytes from a `OnceLock`: no file is read, no pool is taken,
//! nothing is logged per request. O(1) per request at any table size; the
//! bytes are copied by no one (a `&'static` slice into the response).
//! Measured by
//! `api::server::reasons_route::tests::reasons_json_p99_is_under_five_ms_cold_warm_and_with_sixty_clients`
//! (`docs/06-limits.md` L-R405-01).
//!
//! The `ETag` is `"<build>.<16 hex of the bytes' BLAKE3>"`, so it names the
//! build that serves the table and changes whenever the bytes do, an
//! unstamped build's included. The answer says `Cache-Control: no-cache`: a
//! browser revalidates every time and a new build's table is never hidden
//! behind an old one's (FM-280: a page older than its server). A reader that
//! sends the tag back in `If-None-Match` is answered `304` with no body, by
//! the same comparison `/store.json` uses (`*` is not honoured).

use std::sync::OnceLock;

use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse as _, Response};

/// The served table, built on the first request and kept for the life of the
/// process.
pub(super) fn served() -> &'static pull::reasons::Body {
    static SERVED: OnceLock<pull::reasons::Body> = OnceLock::new();
    SERVED.get_or_init(|| pull::reasons::body(super::build_label()))
}

/// `GET /reasons.json`.
pub(super) async fn reasons_json(asked_with: HeaderMap) -> Response {
    answer(served(), &asked_with)
}

/// The answer for `served` to a reader that sent `asked_with`.
fn answer(served: &'static pull::reasons::Body, asked_with: &HeaderMap) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    // A tag that is not a header value is not sent, and then no reader is
    // ever answered `304`: the whole body, as `/instruments.json` does.
    let Ok(tag) = HeaderValue::from_str(&served.etag) else {
        return (StatusCode::OK, headers, served.bytes.as_slice()).into_response();
    };
    headers.insert(header::ETAG, tag);
    if super::store_route::none_match_hit(asked_with, &served.etag) {
        return (StatusCode::NOT_MODIFIED, headers).into_response();
    }
    (StatusCode::OK, headers, served.bytes.as_slice()).into_response()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::indexing_slicing,
        reason = "a test that cannot panic cannot fail"
    )]

    use super::*;
    use std::time::{Duration, Instant};

    async fn read(response: Response) -> (StatusCode, HeaderMap, Vec<u8>) {
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, headers, bytes.to_vec())
    }

    fn asking(tag: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::IF_NONE_MATCH, HeaderValue::from_str(tag).unwrap());
        headers
    }

    /// p50, p99 and max of `took`, sorted in place.
    fn percentiles(took: &mut [Duration]) -> (Duration, Duration, Duration) {
        took.sort_unstable();
        let at = |q: usize| took[(took.len() * q / 100).min(took.len() - 1)];
        (at(50), at(99), took[took.len() - 1])
    }

    #[tokio::test]
    async fn reasons_json_is_served_once_per_build() {
        let first = served();
        let again = served();
        assert!(std::ptr::eq(first, again), "one body for the process");
        let (status, headers, bytes) = read(reasons_json(HeaderMap::new()).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(bytes, first.bytes);
        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            "application/json; charset=utf-8"
        );
        assert_eq!(headers.get(header::CACHE_CONTROL).unwrap(), "no-cache");
        assert_eq!(headers.get(header::ETAG).unwrap(), first.etag.as_str());
        let tag = first.etag.trim_matches('"');
        assert!(
            tag.starts_with(&format!("{}.", super::super::build_label())),
            "{tag}"
        );
        // The document is the table's, whole, under this build.
        let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(doc["schema"], pull::reasons::SCHEMA);
        assert_eq!(doc["build"], super::super::build_label());
        assert_eq!(doc["count"], pull::reasons::TABLE.len());
        let found = doc["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["code"] == "needs_month_generation")
            .unwrap();
        assert_eq!(
            found["title"],
            "needs_month_generation: the month already passed this day; press Repair"
        );
        assert_eq!(found["control"], "press Repair");
        // A second answer is the same bytes and the same tag: no rebuild.
        let (_, headers_again, bytes_again) = read(reasons_json(HeaderMap::new()).await).await;
        assert_eq!(bytes_again, bytes);
        assert_eq!(headers_again.get(header::ETAG), headers.get(header::ETAG));
    }

    #[tokio::test]
    async fn the_reader_holding_this_build_s_tag_is_answered_304_and_no_other() {
        let tag = served().etag.clone();
        let (status, headers, bytes) = read(reasons_json(asking(&tag)).await).await;
        assert_eq!(status, StatusCode::NOT_MODIFIED);
        assert!(bytes.is_empty());
        assert_eq!(headers.get(header::ETAG).unwrap(), tag.as_str());
        // Among several tags, the one held is enough.
        let several = format!("\"older.0000000000000000\", {tag}");
        assert_eq!(
            read(reasons_json(asking(&several)).await).await.0,
            StatusCode::NOT_MODIFIED
        );
        // Another build's tag, a bare word, `*` and garbage: the whole body.
        for other in ["\"older.0000000000000000\"", "*", "garbage", ""] {
            let (status, _, bytes) = read(reasons_json(asking(other)).await).await;
            assert_eq!(status, StatusCode::OK, "{other:?}");
            assert_eq!(bytes, served().bytes, "{other:?}");
        }
        // A header that is not text is no tag.
        let mut odd = HeaderMap::new();
        odd.insert(
            header::IF_NONE_MATCH,
            HeaderValue::from_bytes(&[0xFF, 0xFE]).unwrap(),
        );
        assert_eq!(read(reasons_json(odd).await).await.0, StatusCode::OK);
    }

    #[tokio::test]
    async fn a_tag_that_is_not_a_header_value_is_not_sent_and_never_answered_304() {
        // A build label no header can carry (a newline in it) still serves
        // the table: whole, without a tag, so no reader is answered 304.
        let odd: &'static pull::reasons::Body =
            Box::leak(Box::new(pull::reasons::body("bad\nlabel")));
        let (status, headers, bytes) =
            read(answer(odd, &asking(&odd.etag.replace('\n', " ")))).await;
        assert_eq!(status, StatusCode::OK);
        assert!(headers.get(header::ETAG).is_none());
        assert_eq!(headers.get(header::CACHE_CONTROL).unwrap(), "no-cache");
        assert_eq!(bytes, odd.bytes);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn reasons_json_p99_is_under_five_ms_cold_warm_and_with_sixty_clients() {
        // n = 200 sequential requests to the handler, in process: the first
        // may be the cold one that builds the body (another test may have
        // built it first), the rest warm; then 60 clients at once, 200
        // requests each. Under 5 ms at p99 is the spec's bound for a table
        // served from memory. In-process: the socket's share is the Mac's
        // shadow-server run (NEEDS THE MAC item 7).
        let mut took = Vec::with_capacity(200);
        for _ in 0..200 {
            let start = Instant::now();
            let (status, _, _) = read(reasons_json(HeaderMap::new()).await).await;
            took.push(start.elapsed());
            assert_eq!(status, StatusCode::OK);
        }
        let cold = took[0];
        let (p50, p99, max) = percentiles(&mut took);
        eprintln!("/reasons.json 1 client n=200 cold={cold:?} p50={p50:?} p99={p99:?} max={max:?}");
        assert!(p99 < Duration::from_millis(5), "p99 {p99:?}");

        let tag = served().etag.clone();
        let clients: Vec<_> = (0..60)
            .map(|client| {
                let tag = tag.clone();
                tokio::spawn(async move {
                    let mut took = Vec::with_capacity(200);
                    for n in 0..200 {
                        // Half the clients revalidate, as a page does.
                        let asked = if client % 2 == 0 {
                            asking(&tag)
                        } else {
                            HeaderMap::new()
                        };
                        let start = Instant::now();
                        let (status, _, _) = read(reasons_json(asked).await).await;
                        took.push(start.elapsed());
                        let want = if client % 2 == 0 {
                            StatusCode::NOT_MODIFIED
                        } else {
                            StatusCode::OK
                        };
                        assert_eq!(status, want, "client {client} request {n}");
                    }
                    took
                })
            })
            .collect();
        let mut all = Vec::with_capacity(60 * 200);
        for client in clients {
            all.extend(client.await.unwrap());
        }
        let (p50, p99, max) = percentiles(&mut all);
        eprintln!("/reasons.json 60 clients n=12000 p50={p50:?} p99={p99:?} max={max:?}");
        assert!(p99 < Duration::from_millis(5), "p99 {p99:?}");
    }
}

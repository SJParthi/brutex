//! Refreshing the instrument masters from the browser, and saying when they
//! are stale.
//!
//! # What this closes
//!
//! `pull::masters` describes where each master comes from and lands the bytes;
//! nothing called it. Before that module there was no fetch at all — the
//! masters arrived on the operator's disk by hand, and a stale master was
//! invisible because the parse succeeded and every symbol renamed since
//! resolved to the old row.
//!
//! # It is NOT the ingest pull, and the separation is the design
//!
//! `/pull/*` moves BARS: it spends the vendor's quota per instrument-month,
//! runs for minutes to hours, and is the thing an operator watches. Refreshing
//! the masters moves four files, three of which are public CDN downloads that
//! cost nothing. Folding them into one control would make a free, fast,
//! safe-to-repeat operation look like the expensive one and be avoided for the
//! same reasons.
//!
//! # The staleness answer is the half that matters
//!
//! `Site::load` parses the masters **once, at startup**, and there is no reload
//! path. So an operator who refreshes the masters while the server is running
//! gets new bytes on disk and the same universe in memory — and every page keeps
//! answering from the boot parse with nothing saying so. [`status_json`] is what
//! says so: it compares each master's mtime against the moment the site was
//! loaded and reports which are newer.
//!
//! **That does not fix it, and this module does not claim to.** Hot-reloading
//! needs the master set behind a swap inside `Site`, which every route shares;
//! until that lands, the honest thing is a refresh that tells the operator a
//! restart is required rather than one that silently does half the job.

use std::path::Path;

use pull::chain::Discovery;
use pull::masters::{self, Landed, Source, Transport};

/// The JSON content type every route here answers with.
type JsonHeaders = [(axum::http::HeaderName, &'static str); 1];

fn json_headers() -> JsonHeaders {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

/// One source's outcome, as the page reads it.
fn outcome_json(source: &Source, landed: &Landed) -> String {
    let head = format!(
        r#"{{"file":{},"url":{},"needs_token":{}"#,
        crate::render::json_string(source.file),
        crate::render::json_string(source.url),
        source.needs_token
    );
    match *landed {
        Landed::Written { bytes, changed } => {
            format!(r#"{head},"written":true,"bytes":{bytes},"changed":{changed},"refusal":null}}"#)
        }
        Landed::Refused(ref why) => format!(
            r#"{head},"written":false,"bytes":0,"changed":false,"refusal":{}}}"#,
            crate::render::json_string(why)
        ),
    }
}

/// Fetches and lands every source this transport may carry.
///
/// **`may_fetch` is consulted before the URL is touched**, not after. A public
/// source handed to a credentialed transport would put the vendor's token on a
/// host that never issued it — see `pull::masters::Transport` — and a check that
/// ran after the request would be a check that ran after the leak.
async fn refresh_with<D: Discovery>(
    from: &D,
    dir: &Path,
    via: Transport,
) -> Vec<(&'static Source, Landed)> {
    let mut out = Vec::new();
    for source in masters::SOURCES.iter() {
        if let Err(why) = masters::may_fetch(source, via) {
            // NOT AN ERROR ROW. This source simply is not this transport's to
            // fetch, and reporting it as a refusal would put a red line beside
            // a file that a second call will land correctly.
            let _skipped = why;
            continue;
        }
        let landed = match from.get(source.url).await {
            Ok(body) => masters::land(dir, source, &body),
            Err(refusal) => Landed::Refused(format!("{refusal}")),
        };
        out.push((source, landed));
    }
    out
}

/// `POST /masters/refresh` — download the public masters and land them.
///
/// # Why this does not fetch Zerodha's
///
/// Its dump is behind the same token every bar request spends, and
/// `CLAUDE.md` §8 puts that credential's whole lifecycle outside this
/// repository's discretion. The public three cost nothing and are safe to
/// repeat; spending a shared credential is a different act and belongs on a
/// control that says so. `pull::masters::Transport` refuses the pairing rather
/// than trusting this comment.
///
/// # It answers when it is done, not when it is accepted
///
/// Unlike a sweep, this is three files and seconds — there is no progress to
/// poll and no slot to claim. A route that returned `202` here would invent a
/// state machine for work that finishes before the response would have.
pub async fn refresh(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let Ok(dir) = crate::server::masters_dir() else {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            r#"{"masters":[],"refusal":"neither BRUTEX_MASTERS nor HOME is set, so the masters directory cannot be found"}"#.to_owned(),
        );
    };
    let from = match pull::masters::PublicFetch::new() {
        Ok(from) => from,
        Err(why) => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                json_headers(),
                format!(
                    r#"{{"landed":[],"refusal":{}}}"#,
                    crate::render::json_string(&why)
                ),
            );
        }
    };

    let landed = refresh_with(&from, &dir, Transport::Public).await;
    let _ = telemetry::emit_if!(
        telemetry::Level::Info,
        "api.masters",
        "the public instrument masters were refreshed from the browser",
        "landed" => telemetry::Value::Uint(
            landed.iter().filter(|(_, l)| l.is_written()).count() as u64
        ),
        "asked" => telemetry::Value::Uint(landed.len() as u64),
    );

    let rows: Vec<String> = landed
        .iter()
        .map(|(source, l)| outcome_json(source, l))
        .collect();
    // ANY REFUSAL MAKES THE WHOLE CALL A REFUSAL. Three of four landing is not
    // a success with a footnote: the universe is merged from all of them, so a
    // feed left on yesterday's master is a feed whose symbols disagree with the
    // others, and a green answer would hide that.
    let all_landed = landed.iter().all(|(_, l)| l.is_written());
    let status = if all_landed {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::BAD_GATEWAY
    };
    (
        status,
        json_headers(),
        format!(
            r#"{{"landed":[{}],"all_landed":{all_landed},"restart_required":true}}"#,
            rows.join(",")
        ),
    )
}

/// `GET /masters/status.json` — what is on disk, and whether it is newer than
/// the parse this process is answering from.
///
/// **The staleness answer, which is the one an operator needs after a refresh.**
/// `Site::load` parses the masters once at startup and there is no reload path,
/// so a master refreshed while the server runs is new bytes behind an old
/// universe. Nothing said so before this route; the page looked identical.
pub async fn status_json(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let Ok(dir) = crate::server::masters_dir() else {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            r#"{"masters":[],"refusal":"neither BRUTEX_MASTERS nor HOME is set, so the masters directory cannot be found"}"#.to_owned(),
        );
    };
    let parsed_at = site.parsed_at;

    let rows: Vec<String> = masters::SOURCES
        .iter()
        .map(|source| {
            let path = masters::path_of(&dir, source);
            let held = std::fs::metadata(&path).ok();
            let bytes = held.as_ref().map_or(0, std::fs::Metadata::len);
            // NEWER THAN THE PARSE MEANS THE PROCESS IS ANSWERING FROM OLD
            // BYTES. Equal is not newer: a file written in the same second the
            // site loaded was read by that load.
            let newer = held
                .as_ref()
                .and_then(|m| m.modified().ok())
                .is_some_and(|at| at > parsed_at);
            format!(
                r#"{{"file":{},"present":{},"bytes":{bytes},"newer_than_parse":{newer},"needs_token":{}}}"#,
                crate::render::json_string(source.file),
                held.is_some(),
                source.needs_token
            )
        })
        .collect();

    let any_newer = rows
        .iter()
        .any(|row| row.contains(r#""newer_than_parse":true"#));
    (
        axum::http::StatusCode::OK,
        json_headers(),
        format!(
            r#"{{"masters":[{}],"restart_required":{any_newer}}}"#,
            rows.join(",")
        ),
    )
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{outcome_json, refresh_with};
    use pull::chain::{Discovery, Refusal};
    use pull::masters::{self, Landed, Transport};

    /// A current-thread runtime rather than `#[tokio::test]`, matching
    /// `crate::emitted`'s helper: these tests drive one async function and a
    /// multi-thread runtime would spawn workers for work that never leaves
    /// this thread.
    fn block_on<F: core::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("a current-thread runtime")
            .block_on(future)
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = crate::scratch::path(&format!("masters-{name}"));
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    fn a_master() -> String {
        let mut body = String::from("symbol,name,isin\n");
        while body.len() <= masters::MIN_BODY_BYTES {
            body.push_str("NIFTY,NIFTY 50,INE000000000\n");
        }
        body
    }

    /// A transport that records every URL it was asked for.
    struct Recording {
        body: String,
        seen: std::sync::Mutex<Vec<String>>,
    }

    impl Discovery for Recording {
        async fn get(&self, url: &str) -> Result<String, Refusal> {
            self.seen
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(url.to_owned());
            Ok(self.body.clone())
        }
    }

    #[test]
    fn a_public_refresh_never_asks_for_the_credentialed_url() {
        // THE LEAK THIS ROUTE EXISTS NOT TO HAVE. `may_fetch` is consulted
        // before the url is touched, so the recording transport must never see
        // Zerodha's — if it did, a real public transport would have made the
        // request and a credentialed one would have carried the token there.
        let dir = scratch("public-only");
        let from = Recording {
            body: a_master(),
            seen: std::sync::Mutex::new(Vec::new()),
        };

        let landed = block_on(refresh_with(&from, &dir, Transport::Public));

        let asked = from
            .seen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        for source in masters::SOURCES.iter().filter(|s| s.needs_token) {
            assert!(
                !asked.contains(&source.url.to_owned()),
                "a public refresh asked for the credentialed url {}",
                source.url
            );
        }
        assert_eq!(
            asked.len(),
            masters::SOURCES.iter().filter(|s| !s.needs_token).count(),
            "every public source is asked for exactly once: {asked:?}"
        );
        assert!(landed.iter().all(|(_, l)| l.is_written()));
    }

    #[test]
    fn a_transport_refusal_becomes_that_sources_refusal_and_not_the_calls() {
        // ONE HOST DOWN IS ONE ROW REFUSED. A `?` here would abandon the other
        // two public masters because a third CDN was unreachable.
        struct Dead;
        impl Discovery for Dead {
            async fn get(&self, url: &str) -> Result<String, Refusal> {
                Err(Refusal::transport(format!("{url} unreachable")))
            }
        }
        let dir = scratch("dead-host");
        let landed = block_on(refresh_with(&Dead, &dir, Transport::Public));

        assert_eq!(
            landed.len(),
            masters::SOURCES.iter().filter(|s| !s.needs_token).count(),
            "every public source is still attempted"
        );
        assert!(
            landed.iter().all(|(_, l)| !l.is_written()),
            "none of them landed"
        );
    }

    #[test]
    fn a_refusal_reaches_the_page_as_a_sentence_and_a_written_false() {
        let source = &masters::SOURCES[0];
        let json = outcome_json(source, &Landed::Refused("the host answered 503".to_owned()));
        assert!(json.contains(r#""written":false"#), "{json}");
        assert!(json.contains("503"), "{json}");
        assert!(json.contains(source.file), "{json}");

        let written = outcome_json(
            source,
            &Landed::Written {
                bytes: 4_096,
                changed: false,
            },
        );
        assert!(written.contains(r#""written":true"#), "{written}");
        assert!(written.contains(r#""changed":false"#), "{written}");
        assert!(written.contains(r#""refusal":null"#), "{written}");
    }
}

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
//! the masters moves four files -- three public CDN downloads and one
//! credentialed dump. Folding them into one control would make a fast,
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
fn outcome_json(source: &Source, landed: &Result<Landed, String>) -> String {
    let head = format!(
        r#"{{"file":{},"url":{},"needs_token":{}"#,
        crate::render::json_string(source.file),
        crate::render::json_string(source.url),
        source.needs_token
    );
    match *landed {
        Ok(Landed::Written { bytes, changed }) => format!(
            r#"{head},"written":true,"skipped":false,"bytes":{bytes},"changed":{changed},"refusal":null}}"#
        ),
        Ok(Landed::Refused(ref why)) => format!(
            r#"{head},"written":false,"skipped":false,"bytes":0,"changed":false,"refusal":{}}}"#,
            crate::render::json_string(why)
        ),
        // SKIPPED IS ITS OWN ANSWER, distinct from both. A source this
        // transport may not carry has not failed and has not landed: reporting
        // it as a refusal would put a red line beside a file a second call
        // lands correctly, and omitting it would let the caller believe every
        // master is present.
        Err(ref why) => format!(
            r#"{head},"written":false,"skipped":true,"bytes":0,"changed":false,"refusal":{}}}"#,
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
) -> Vec<(&'static Source, Result<Landed, String>)> {
    let mut out = Vec::new();
    for source in &masters::SOURCES {
        // A SKIPPED SOURCE IS A ROW, NOT AN ABSENCE, and the first version of
        // this function got that wrong in the way this whole module exists to
        // prevent. It `continue`d without recording anything, so Zerodha —
        // whose dump this transport may not carry — fell out of the vector
        // entirely, `all_landed` was computed over the three that remained, and
        // the route answered **200 with `all_landed: true`** while
        // `zerodha_instruments.csv` was absent. That is the master for the feed
        // holding every bar in the store, missing, under a green answer.
        //
        // §4's failure wearing a success's clothes, one layer up from where it
        // usually hides: not a refusal reported as a report, but a source
        // reported by not being reported at all.
        if let Err(why) = masters::may_fetch(source, via) {
            out.push((source, Err(why)));
        } else {
            let landed = match from.get(source.url).await {
                Ok(body) => masters::land(dir, source, &body),
                Err(refusal) => Landed::Refused(format!("{refusal}")),
            };
            out.push((source, Ok(landed)));
        }
    }
    out
}

/// The transport Zerodha's dump requires, or why it cannot be built.
///
/// # Why this is a separate function and not inlined
///
/// It reads a credential. Everything above it in this module touches only
/// public hosts, and keeping the one privileged step in a named function means
/// the credential's blast radius is a thing you can point at rather than a
/// branch in the middle of a loop.
///
/// # Errors
///
/// Whatever stopped the credential being read — a missing `HOME`, an unusable
/// `credentials.toml`, no AWS identity, or a descriptor that is not HTTP.
/// Every one of them is a refusal an operator can act on, and none of them is a
/// reason to undo the three masters that already landed.
async fn credentialed_zerodha() -> Result<pull::http::HttpSource, String> {
    let feed = pull::vendor::Feed::Zerodha;
    let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
        return Err(
            "Zerodha's descriptor is not an HTTP transport, so its instrument \
             dump cannot be fetched"
                .to_owned(),
        );
    };
    crate::server::credentialed_source(feed, &spec)
        .await
        .map(|(source, _vendor)| source)
}

/// `POST /masters/refresh` — download every master, public and credentialed.
///
/// # All four, over two transports, and the split is enforced not trusted
///
/// Dhan, Groww and NSE are public files fetched over [`PublicFetch`], which
/// sends no credential. Zerodha's dump is behind the same token every bar
/// request spends and goes over the vendor's own `HttpSource`.
///
/// **Three of four is not the masters.** The universe is MERGED from all of
/// them, so leaving the feed that holds every bar in the store on yesterday's
/// master is not a partial refresh — it is a universe whose feeds disagree with
/// each other about what an instrument is called.
///
/// `pull::masters::may_fetch` decides which transport may carry which source
/// and refuses both mismatches, so a public URL can never be handed the vendor
/// token and the token URL can never be sent without one.
///
/// # A credential failure is one row, never the call
///
/// A missing `HOME`, an unusable `credentials.toml` or no AWS identity stops
/// Zerodha's leg and nothing else. Undoing the three masters that already
/// landed because of it would trade a partial refresh for no refresh at all.
/// `complete` is what turns the shortfall into a non-200.
///
/// # It answers when it is done, not when it is accepted
///
/// Unlike a sweep, this is four files and seconds — there is no progress to
/// poll and no slot to claim. A route that returned `202` here would invent a
/// state machine for work that finishes before the response would have.
pub async fn refresh(
    // THE SITE IS NOT READ, and that is the point: this route writes files
    // and never touches the parsed universe. `status_json` is the one that
    // needs `parsed_at`.
    axum::extract::State(_site): axum::extract::State<crate::server::Loaded>,
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

    let mut landed = refresh_with(&from, &dir, Transport::Public).await;

    // AND THE CREDENTIALED ONE, because three of four is not the masters.
    //
    // Zerodha's dump is behind the same token every bar request spends, so this
    // is the one leg that costs something — but a refresh that leaves the feed
    // holding every bar in the store on yesterday's master is not a refresh.
    // `credentialed_source` reads the credential from Parameter Store through
    // the same path `/pull/*` uses; this repository never mints one and never
    // reads it from a file or an environment variable (§8).
    //
    // A failure here is ONE SOURCE'S refusal, not the call's: the three public
    // masters that already landed must not be undone because an AWS identity is
    // missing. `complete` below is what turns it into a non-200.
    let credentialed = credentialed_zerodha().await;
    for (index, (source, outcome)) in landed.iter_mut().enumerate() {
        let _ = index;
        if !source.needs_token {
            continue;
        }
        *outcome = match credentialed {
            Ok(ref source_and_wire) => match source_and_wire.get(source.url).await {
                Ok(body) => Ok(masters::land(&dir, source, &body)),
                Err(refusal) => Ok(Landed::Refused(format!("{refusal}"))),
            },
            Err(ref why) => Ok(Landed::Refused(why.clone())),
        };
    }
    let _ = telemetry::emit_if!(
        telemetry::Level::Info,
        "api.masters",
        "the public instrument masters were refreshed from the browser",
        // THREE COUNTS, BECAUSE TWO WOULD HIDE THE THIRD. "landed of asked"
        // reads as complete on a call that skipped the master that matters,
        // which is the defect this route shipped with. `skipped` is the number
        // that makes the other two readable.
        "landed" => telemetry::Value::Uint(
            landed
                .iter()
                .filter(|(_, l)| matches!(*l, Ok(ref inner) if inner.is_written()))
                .count() as u64
        ),
        "skipped" => telemetry::Value::Uint(
            landed.iter().filter(|(_, l)| l.is_err()).count() as u64
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
    let attempted_landed = landed
        .iter()
        .filter(|(_, l)| l.is_ok())
        .all(|(_, l)| matches!(*l, Ok(ref inner) if inner.is_written()));

    // AND `complete` IS THE FIELD THAT CANNOT LIE. `attempted_landed` answers
    // "did everything this call tried succeed", which is TRUE on a call that
    // skipped the most important master — and that is exactly what the first
    // version of this route reported, as a 200. `complete` answers the question
    // an operator is actually asking: **is every master this engine reads now
    // present on disk?** A skip makes it false, whatever the fetches did.
    let on_disk = |source: &Source| masters::path_of(&dir, source).is_file();
    let complete = masters::SOURCES.iter().all(on_disk);
    let missing: Vec<String> = masters::SOURCES
        .iter()
        .filter(|source| !on_disk(source))
        .map(|source| crate::render::json_string(source.file))
        .collect();

    let status = if attempted_landed && complete {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::BAD_GATEWAY
    };
    (
        status,
        json_headers(),
        format!(
            r#"{{"landed":[{}],"attempted_landed":{attempted_landed},"complete":{complete},"missing":[{}],"restart_required":true}}"#,
            rows.join(","),
            missing.join(",")
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
        assert!(
            landed
                .iter()
                .filter(|(_, l)| l.is_ok())
                .all(|(_, l)| matches!(*l, Ok(ref inner) if inner.is_written())),
            "every source this transport carried must have landed"
        );
    }

    #[test]
    fn a_skipped_source_is_a_row_and_never_an_absence() {
        // THE DEFECT THIS ROUTE SHIPPED WITH. `refresh_with` used to `continue`
        // past a source this transport may not carry, so Zerodha fell out of
        // the vector entirely -- and the completeness flag, computed over what
        // remained, answered TRUE while `zerodha_instruments.csv` was absent.
        // 200 OK, every box green, and the master for the feed holding every
        // bar in the store missing.
        let dir = scratch("skipped-is-a-row");
        let from = Recording {
            body: a_master(),
            seen: std::sync::Mutex::new(Vec::new()),
        };

        let landed = block_on(refresh_with(&from, &dir, Transport::Public));

        assert_eq!(
            landed.len(),
            masters::SOURCES.len(),
            "EVERY source is a row, whether or not this transport may carry it"
        );

        let skipped: Vec<&str> = landed
            .iter()
            .filter(|(_, l)| l.is_err())
            .map(|(source, _)| source.file)
            .collect();
        assert_eq!(
            skipped,
            vec![
                masters::SOURCES
                    .iter()
                    .find(|s| s.needs_token)
                    .expect("one source needs the token")
                    .file
            ],
            "the credentialed master is the one skipped, and it is REPORTED"
        );

        // AND THE ROW SAYS SO ON THE WIRE, distinctly from a refusal — a
        // caller must be able to tell "not this call's to fetch" from "tried
        // and failed", because only one of them is fixed by trying again.
        let row = landed
            .iter()
            .find(|(source, _)| source.needs_token)
            .map(|(source, l)| outcome_json(source, l))
            .expect("the skipped row");
        assert!(row.contains(r#""skipped":true"#), "{row}");
        assert!(row.contains(r#""written":false"#), "{row}");
        assert!(row.contains("401"), "the reason must be carried: {row}");
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

        // EVERY source is a row; the ones this transport carried are the ones
        // that were attempted, and all of them failed against a dead host.
        assert_eq!(
            landed.len(),
            masters::SOURCES.len(),
            "every source is a row"
        );
        assert_eq!(
            landed.iter().filter(|(_, l)| l.is_ok()).count(),
            masters::SOURCES.iter().filter(|s| !s.needs_token).count(),
            "every public source is still attempted"
        );
        assert!(
            !landed
                .iter()
                .any(|(_, l)| matches!(*l, Ok(ref inner) if inner.is_written())),
            "none of them landed"
        );
    }

    #[test]
    fn a_refusal_reaches_the_page_as_a_sentence_and_a_written_false() {
        let source = &masters::SOURCES[0];
        let json = outcome_json(
            source,
            &Ok(Landed::Refused("the host answered 503".to_owned())),
        );
        assert!(json.contains(r#""written":false"#), "{json}");
        assert!(json.contains("503"), "{json}");
        assert!(json.contains(source.file), "{json}");

        let written = outcome_json(
            source,
            &Ok(Landed::Written {
                bytes: 4_096,
                changed: false,
            }),
        );
        assert!(written.contains(r#""written":true"#), "{written}");
        assert!(written.contains(r#""changed":false"#), "{written}");
        assert!(written.contains(r#""refusal":null"#), "{written}");
    }
}

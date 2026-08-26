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
use pull::masters::{self, Fetched, Landed, Source, Transport};

/// The JSON content type every route here answers with.
type JsonHeaders = [(axum::http::HeaderName, &'static str); 1];

fn json_headers() -> JsonHeaders {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

/// Writes one event per step of one source's ladder, and one for its outcome.
///
/// # What this closes, and it was a hole in this module's own subject
///
/// `/masters/refresh` emitted **one** event carrying three counts —
/// `landed`, `skipped`, `asked` — and nothing else. Three things followed from
/// that, and all three are the shapes this module exists to refuse:
///
/// 1. **Which file failed never reached the log.** `landed=3 skipped=1` does
///    not say whether the missing one was the exchange's index list or the
///    master for the feed holding every bar in the store.
/// 2. **The level never moved.** A refresh where all four refused emitted
///    `Level::Info` reading *"the instrument masters were refreshed"* — a
///    success-shaped line over a total failure, which is `CLAUDE.md` §4's
///    failure wearing a success's clothes, written by the module that spends
///    most of its comments refusing it.
/// 3. **The attempt ledger was not durable.** It reached the HTTP response and
///    nowhere else, so closing the tab destroyed the only record of what the
///    host actually said. An operator asking *"why did this fail an hour ago"*
///    had nothing to search.
///
/// # Cost, because logging in a loop is a fair thing to ask about
///
/// Bounded by the source table and nothing else: at most `ATTEMPTS_PER_URL`
/// per URL plus two primes, per source, plus one outcome event. On the happy
/// path that is **two events per source** — one body, one outcome. Only a host
/// that is actually failing produces more, which is exactly when they are
/// wanted.
///
/// **Gate 17 is not in tension with this.** It silences `vocab engine
/// indicators runner`, which hold the loops over bars and candidates. This is
/// `api`, and the granularity is one event per network round trip — a unit
/// already costing milliseconds, beside which an event costs nothing.
pub(crate) fn record(source: &Source, landed: &Result<Landed, String>, tried: &Fetched) {
    for step in &tried.attempts {
        let (level, what, status, detail) = match step.got {
            masters::Got::Primed => (
                telemetry::Level::Debug,
                "a session was established before asking",
                telemetry::Value::Null,
                String::new(),
            ),
            masters::Got::PrimeRefused { ref detail } => (
                // WARN AND NOT ERROR. The prime is unverified (see
                // `Source::prime`), the real request still goes out, and the
                // host's own answer to it is the fact worth escalating.
                telemetry::Level::Warn,
                "the priming request refused, and the real request went out anyway",
                telemetry::Value::Null,
                detail.clone(),
            ),
            masters::Got::Body { bytes } => (
                telemetry::Level::Info,
                "a body came back",
                telemetry::Value::Null,
                format!("{bytes} bytes"),
            ),
            masters::Got::Refused {
                status,
                ref detail,
                verdict,
            } => (
                match verdict {
                    // ASKING AGAIN IS NOT YET A PROBLEM, and logging every
                    // transient blip at Error is how a log stops being read.
                    masters::Verdict::Again | masters::Verdict::Reprime => telemetry::Level::Warn,
                    // SETTLED IS THE ONE THAT ENDS A URL. Nothing further is
                    // tried there, so this is the line an operator needs.
                    masters::Verdict::Never => telemetry::Level::Error,
                },
                match verdict {
                    masters::Verdict::Again => "refused, and will be asked again",
                    masters::Verdict::Reprime => "refused on a session, re-priming",
                    masters::Verdict::Never => "refused settled, and will not be asked again",
                },
                // A STATUS OF `null` IS NOT A STATUS OF ZERO. Nothing answered
                // is a different fact from a host answering, and `Value::Null`
                // is the type's own way of saying "known, and known to be
                // nothing" — see its doc comment.
                status.map_or(telemetry::Value::Null, |code| {
                    telemetry::Value::Uint(u64::from(code))
                }),
                detail.clone(),
            ),
        };
        let _ = telemetry::emit_if!(
            level,
            "api.masters.attempt",
            what,
            "file" => telemetry::Value::Str(source.file),
            "url" => telemetry::Value::Str(&step.url),
            "attempt" => telemetry::Value::Uint(u64::from(step.number)),
            "waited_ms" => telemetry::Value::Uint(step.waited_ms),
            "status" => status,
            "detail" => telemetry::Value::Str(&detail),
        );
    }

    let (level, what, extra) = match *landed {
        Ok(Landed::Written { bytes, changed }) => (
            telemetry::Level::Info,
            if changed {
                "the master was replaced with new bytes"
            } else {
                "the master was refetched and had not changed"
            },
            format!("{bytes} bytes"),
        ),
        // A SKIPPED SOURCE IS NOT AN ERROR AND IS NOT A SUCCESS. A public
        // transport declining the credentialed dump is the guard working.
        Err(ref why) => (
            telemetry::Level::Warn,
            "this transport may not carry this master, so it was not asked for",
            why.clone(),
        ),
        Ok(Landed::Refused(ref why)) => (
            telemetry::Level::Error,
            "the master could not be refreshed and the old bytes still stand",
            why.clone(),
        ),
    };
    let _ = telemetry::emit_if!(
        level,
        "api.masters.source",
        what,
        "file" => telemetry::Value::Str(source.file),
        "url" => telemetry::Value::Str(source.url),
        "needs_token" => telemetry::Value::Bool(source.needs_token),
        "attempts" => telemetry::Value::Uint(tried.attempts.len() as u64),
        "waited_ms" => telemetry::Value::Uint(tried.waited_ms()),
        "detail" => telemetry::Value::Str(&extra),
    );
}

/// Every step the ladder took, as the page reads it.
///
/// # Why the page gets the whole ledger and not a summary
///
/// "It failed" is not something an operator can act on. *Which URL, how many
/// times, and what the host actually said* are the three questions, and a
/// refresh that answers a single sentence has thrown all three away. The array
/// is bounded by the source table — at most `ATTEMPTS_PER_URL` per URL plus two
/// primes — so it cannot grow with anything the network does.
fn attempts_json(tried: &Fetched) -> String {
    let rows: Vec<String> = tried
        .attempts
        .iter()
        .map(|step| {
            let (got, status, detail) = match &step.got {
                masters::Got::Primed => ("primed", "null".to_owned(), "null".to_owned()),
                masters::Got::PrimeRefused { detail } => (
                    "prime_refused",
                    "null".to_owned(),
                    crate::render::json_string(detail),
                ),
                masters::Got::Body { bytes } => ("body", bytes.to_string(), "null".to_owned()),
                masters::Got::Refused {
                    status,
                    detail,
                    verdict,
                } => (
                    match verdict {
                        masters::Verdict::Again => "refused_retrying",
                        masters::Verdict::Reprime => "refused_repriming",
                        masters::Verdict::Never => "refused_settled",
                    },
                    status.map_or_else(|| "null".to_owned(), |code| code.to_string()),
                    crate::render::json_string(detail),
                ),
            };
            format!(
                r#"{{"url":{},"number":{},"waited_ms":{},"got":{},"status":{status},"detail":{detail}}}"#,
                crate::render::json_string(&step.url),
                step.number,
                step.waited_ms,
                crate::render::json_string(got),
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

/// One source's outcome, as the page reads it.
fn outcome_json(source: &Source, landed: &Result<Landed, String>, tried: &Fetched) -> String {
    let head = format!(
        r#"{{"file":{},"url":{},"needs_token":{},"attempts":{},"waited_ms":{}"#,
        crate::render::json_string(source.file),
        crate::render::json_string(source.url),
        source.needs_token,
        attempts_json(tried),
        tried.waited_ms(),
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
async fn refresh_with<D: Discovery, P: masters::Pause>(
    from: &D,
    clock: &P,
    dir: &Path,
    via: Transport,
) -> Vec<(&'static Source, Result<Landed, String>, Fetched)> {
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
            // A SKIPPED SOURCE HAS AN EMPTY LEDGER because nothing was asked.
            // That is a different fact from "asked and got nothing", and the
            // page can tell them apart by the array being empty rather than by
            // reading the sentence.
            out.push((source, Err(why), Fetched::default()));
        } else {
            let (landed, tried) = obtain(from, clock, dir, source).await;
            out.push((source, Ok(landed), tried));
        }
    }
    out
}

/// Replaces the credentialed rows with what the shared token could fetch.
///
/// # Why the credentialed leg is a second pass and not a branch in the first
///
/// `may_fetch` refuses to hand a public transport a credentialed source, so the
/// public pass leaves Zerodha's row `skipped` rather than absent. This pass
/// fills that row in. Keeping the two apart is what makes the guard checkable:
/// one transport, one rule, one place it is enforced.
///
/// # It runs the same ladder
///
/// Zerodha's dump is the one master that costs a shared token, and it was also
/// the one leg with no retry at all — a single `503` from `api.kite.trade` left
/// the feed holding every bar in the store on yesterday's file. Nothing about
/// spending a credential makes a transient failure less transient.
async fn credentialed_leg<P: masters::Pause>(
    landed: &mut [(&'static Source, Result<Landed, String>, Fetched)],
    clock: &P,
    dir: &Path,
) {
    let credentialed = credentialed_zerodha().await;
    for (source, outcome, tried) in landed.iter_mut() {
        if !source.needs_token {
            continue;
        }
        match credentialed {
            Ok(ref wire) => {
                let (got, steps) = obtain(wire, clock, dir, source).await;
                *outcome = Ok(got);
                *tried = steps;
            }
            // A CREDENTIAL THAT COULD NOT BE READ IS NOT A LADDER FAILURE.
            // Nothing was asked, so the ledger stays empty and the reason is
            // the one the credential read gave.
            Err(ref why) => {
                *outcome = Ok(Landed::Refused(why.clone()));
                *tried = Fetched::default();
            }
        }
    }
}

/// Re-parses the masters into the live site, and says so in the log.
///
/// # This is what makes a refresh mean anything
///
/// Without it the four files are new bytes on disk behind an old universe in
/// memory, and every page keeps answering from the boot parse. `restart_required`
/// was hardcoded `true` here for two commits — honest, and still the failure
/// handed back to the operator rather than solved.
///
/// # It runs even when a source refused
///
/// The others may have landed, and a universe two files newer is strictly
/// better than one four files older. `Site::reparse` is what refuses to swap in
/// an EMPTY universe over a working one; this does not second-guess it.
///
/// # Errors
///
/// Whatever `Site::reparse` refused with, already an operator-readable sentence.
pub(crate) fn reload(site: &crate::server::Loaded, dir: &Path) -> Result<String, String> {
    let reloaded = site.reparse(dir);
    let _ = telemetry::emit_if!(
        if reloaded.is_ok() {
            telemetry::Level::Info
        } else {
            telemetry::Level::Error
        },
        "api.masters.reload",
        match reloaded {
            Ok(_) => "the universe was re-parsed and every page now answers from it",
            Err(_) => "the universe could NOT be re-parsed, so the previous one still stands",
        },
        "detail" => telemetry::Value::Str(match reloaded {
            Ok(ref notes) | Err(ref notes) => notes,
        }),
    );
    reloaded
}

/// Runs the ladder for one source and lands whatever it brought back.
///
/// # Why landing is here rather than inside `pull::masters::fetch`
///
/// `fetch` opens no file and `land` opens no socket, and keeping that line
/// sharp is what lets each be tested against a fake of the other. This is the
/// one function that has both, and it holds no policy of its own.
async fn obtain<D: Discovery, P: masters::Pause>(
    from: &D,
    clock: &P,
    dir: &Path,
    source: &Source,
) -> (Landed, Fetched) {
    let tried = masters::fetch(from, clock, source).await;
    let landed = match tried.body {
        Some(ref body) => masters::land(dir, source, body),
        // THE LAST WORD THE HOST SAID, not a summary of the ladder. The full
        // ledger travels beside this in `attempts`, so the sentence is free to
        // be the single most useful line rather than a digest of every step.
        None => Landed::Refused(tried.last_refusal().map_or_else(
            || format!("`{}` — no URL was reachable", source.url),
            |why| format!("`{}` — {why}", source.file),
        )),
    };
    (landed, tried)
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
    // THE SITE IS READ **AND WRITTEN** NOW, which reverses this parameter's
    // former comment. It used to say *"this route writes files and never
    // touches the parsed universe"*, and that was the whole defect: an
    // operator pressed Refresh, four files landed, and every page kept
    // answering from the boot parse. `Site::reparse` is what closes it.
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

    let clock = masters::Clock;
    let mut landed = refresh_with(&from, &clock, &dir, Transport::Public).await;

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
    credentialed_leg(&mut landed, &clock, &dir).await;

    // EVERY SOURCE'S OWN STORY, DURABLY. The summary below is three counts,
    // and three counts cannot say which file failed or what the host said.
    for (source, outcome, tried) in &landed {
        record(source, outcome, tried);
    }

    let written = landed
        .iter()
        .filter(|(_, l, _)| matches!(*l, Ok(ref inner) if inner.is_written()))
        .count();
    let _ = telemetry::emit_if!(
        // THE LEVEL FOLLOWS THE OUTCOME, and it used to be `Info` always. A
        // refresh where every source refused wrote `Info: the ... masters were
        // refreshed` with `landed=0` beside it — a line that reads as success,
        // over a total failure, in the module whose whole subject is that a
        // stale master is invisible. §4.
        if written == landed.len() {
            telemetry::Level::Info
        } else if written == 0 {
            telemetry::Level::Error
        } else {
            telemetry::Level::Warn
        },
        "api.masters",
        // "PUBLIC" WAS TRUE FOR ONE COMMIT. This route runs the credentialed
        // leg too, and a message naming only the free half would have an
        // operator searching the log for a Zerodha refresh that is right there
        // under a sentence saying it was not one.
        "the instrument masters were refreshed from the browser",
        // THREE COUNTS, BECAUSE TWO WOULD HIDE THE THIRD. "landed of asked"
        // reads as complete on a call that skipped the master that matters,
        // which is the defect this route shipped with. `skipped` is the number
        // that makes the other two readable.
        "landed" => telemetry::Value::Uint(written as u64),
        "skipped" => telemetry::Value::Uint(
            landed.iter().filter(|(_, l, _)| l.is_err()).count() as u64
        ),
        "asked" => telemetry::Value::Uint(landed.len() as u64),
        // AND WHAT THE WHOLE CALL SPENT WAITING, which is the number that
        // separates "the hosts were fine" from "the hosts were sick and it
        // recovered" — two refreshes that land identically and cost minutes
        // apart.
        "waited_ms" => telemetry::Value::Uint(
            landed.iter().map(|(_, _, t)| t.waited_ms()).sum::<u64>()
        ),
    );

    let rows: Vec<String> = landed
        .iter()
        .map(|(source, l, tried)| outcome_json(source, l, tried))
        .collect();
    // ANY REFUSAL MAKES THE WHOLE CALL A REFUSAL. Three of four landing is not
    // a success with a footnote: the universe is merged from all of them, so a
    // feed left on yesterday's master is a feed whose symbols disagree with the
    // others, and a green answer would hide that.
    let attempted_landed = landed
        .iter()
        .filter(|(_, l, _)| l.is_ok())
        .all(|(_, l, _)| matches!(*l, Ok(ref inner) if inner.is_written()));

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

    let reloaded = reload(&site, &dir);

    let status = if attempted_landed && complete && reloaded.is_ok() {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::BAD_GATEWAY
    };
    (
        status,
        json_headers(),
        format!(
            r#"{{"landed":[{}],"attempted_landed":{attempted_landed},"complete":{complete},"missing":[{}],"reloaded":{},"universe":{},"restart_required":false}}"#,
            rows.join(","),
            missing.join(","),
            reloaded.is_ok(),
            crate::render::json_string(match reloaded {
                Ok(ref notes) | Err(ref notes) => notes,
            }),
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
    let parsed_at = site.universe().at;

    let body = status_rows(&dir, parsed_at);
    (axum::http::StatusCode::OK, json_headers(), body)
}

/// The status answer over a directory and a parse time a caller names.
///
/// Split out for the reason every other split in this module has: the two
/// inputs it depends on are a directory and a clock reading, and behind a
/// handler both are the process's, so nothing could assert what an absent file
/// or an old file actually renders as.
fn status_rows(dir: &Path, parsed_at: std::time::SystemTime) -> String {
    let rows: Vec<String> = masters::SOURCES
        .iter()
        .map(|source| {
            let path = masters::path_of(dir, source);
            let held = std::fs::metadata(&path).ok();
            let bytes = held.as_ref().map_or(0, std::fs::Metadata::len);
            // NEWER THAN THE PARSE MEANS THE PROCESS IS ANSWERING FROM OLD
            // BYTES. Equal is not newer: a file written in the same second the
            // site loaded was read by that load.
            let newer = held
                .as_ref()
                .and_then(|m| m.modified().ok())
                .is_some_and(|at| at > parsed_at);
            // WHEN, AND NOT ONLY WHETHER. "Present" says a file exists;
            // "present, written eleven months ago" is the answer an operator
            // acts on, and it is the whole reason this module exists — a stale
            // master parses cleanly and resolves every renamed symbol to the
            // old row. `null` where there is no file or the platform has no
            // mtime, never a zero that reads as 1970.
            let modified = held
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or_else(
                    || "null".to_owned(),
                    |since| since.as_millis().to_string(),
                );
            format!(
                r#"{{"file":{},"present":{},"bytes":{bytes},"modified_unix_millis":{modified},"newer_than_parse":{newer},"needs_token":{}}}"#,
                crate::render::json_string(source.file),
                held.is_some(),
                source.needs_token
            )
        })
        .collect();

    let any_newer = rows
        .iter()
        .any(|row| row.contains(r#""newer_than_parse":true"#));
    format!(
        r#"{{"masters":[{}],"restart_required":{any_newer}}}"#,
        rows.join(",")
    )
}

/// `GET /masters` — the page an operator refreshes the masters from.
///
/// # Why a page at all, when two JSON routes already existed
///
/// Because a route nobody can reach is a route nobody uses. D-0308 shipped
/// `POST /masters/refresh` and `GET /masters/status.json` and added them to no
/// nav and no page, so the only person who could refresh a master was one who
/// had read `server.rs`'s route table — and `crate::logs`' own comment already
/// names that exact failure: *"a page only somebody who had read the route
/// table could find"*.
///
/// # It renders in the browser rather than on the server, and that is the point
///
/// A refresh is four network round trips and can take a minute. A server-side
/// form post would leave the operator on a blank tab until every one of them
/// finished, and would then replace the page with the answer. Fetching from the
/// page keeps the source table visible while it fills in, and lets each row
/// carry its own attempt ledger — which is the half `POST /masters/refresh` was
/// built to hand over and had nowhere to put.
pub async fn page() -> axum::response::Html<String> {
    axum::response::Html(page_html())
}

/// The page body, split out so a test can read it without a server.
fn page_html() -> String {
    let mut rows = String::new();
    for source in &masters::SOURCES {
        let cost = if source.needs_token {
            "<span class=\"tag tok\">spends the shared token</span>"
        } else {
            "<span class=\"tag pub\">public</span>"
        };
        let shape = match source.shape {
            masters::Shape::Csv => "CSV, landed as received",
            masters::Shape::NseIndexJson => "JSON, converted to index_name,category",
        };
        let owner = source.vendor.map_or(
            "NSE — the reference the feeds are checked against",
            |v| match v {
                brutex_core::vendor::Vendor::Dhan => "Dhan",
                brutex_core::vendor::Vendor::Groww => "Groww",
                brutex_core::vendor::Vendor::Zerodha => "Zerodha",
                _ => "another feed",
            },
        );
        let _ = std::fmt::Write::write_fmt(
            &mut rows,
            format_args!(
                "<tr data-file=\"{file}\"><td><b>{file}</b><div class=\"sub\">{owner}</div></td>\
                 <td class=\"url\">{url}{prime}</td>\
                 <td>{cost}<div class=\"sub\">{shape}</div></td>\
                 <td class=\"disk\">—</td>\
                 <td class=\"out\">not asked yet</td></tr>",
                file = crate::render::escape(source.file),
                owner = crate::render::escape(owner),
                url = crate::render::escape(source.url),
                prime = source.prime.map_or_else(String::new, |p| format!(
                    "<div class=\"sub\">session primed at {}</div>",
                    crate::render::escape(p)
                )),
                cost = cost,
                shape = shape,
            ),
        );
    }

    format!(
        "<!doctype html><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>brutex · masters</title><style>{STYLE}</style>\
         {nav}\
         <h1>Instrument masters</h1>\
         <p class=\"lead\">Four files. Three vendor masters and the exchange's own index \
         list, which is what the vendors are checked against. Nothing here touches the \
         bar store or spends a bar quota — this is <b>not</b> the ingest pull.</p>\
         <div class=\"bar\">\
         <button id=\"go\" class=\"go\">Refresh all four</button>\
         <button id=\"verify\" class=\"go alt\">Cross&#8209;verify against NSE</button>\
         <span id=\"say\" class=\"say\">Reads what is on disk on load. Nothing is fetched \
         until you press a button.</span></div>\
         <table><thead><tr><th>File</th><th>Published at</th><th>Cost &amp; shape</th>\
         <th>On disk</th><th>Last refresh</th></tr></thead><tbody>{rows}</tbody></table>\
         <div id=\"ledger\"></div>\
         <div id=\"xverify\" class=\"att\"></div>\
         <p class=\"foot\">A refresh writes new bytes to disk and does <b>not</b> reload the \
         parsed universe: <code>Site::load</code> parses the masters once, at startup. When a \
         file changes under a running server this page says a restart is required, because \
         saying nothing would leave every other page answering from the boot parse with \
         nothing to indicate it.</p>\
         <script>{SCRIPT}</script>",
        // THE REAL NAV, NOT A SECOND COPY OF IT. This page was self-contained
        // following `crate::logs`, and inherited its defect with it: a page
        // that is IN the nav and does not RENDER one, so an operator who
        // follows the link has no way back and no indication of where they
        // are. Rendering `render::nav` rather than hand-writing a link bar
        // keeps one list of pages — a second would be correct the day it was
        // written and wrong the first time a page is added.
        nav = crate::render::nav("/masters"),
    )
}

/// The page's stylesheet.
///
/// Its own rather than `render::STYLE`, following `crate::logs`: that constant
/// styles the marketing-shaped pages and this is a control surface.
const STYLE: &str = "\
body{background:#f5f7fb;color:#0a0f1e;font:15px/1.55 ui-sans-serif,-apple-system,system-ui,sans-serif;margin:0;padding:0 0 4rem}\
nav.top{position:sticky;top:0;z-index:10;background:#fff;border-bottom:1px solid #e4e9f3}\
nav.top .inner{max-width:1180px;margin:0 auto;padding:0 1.25rem;display:flex;align-items:center;gap:22px;height:56px}\
nav.top .logo{font-weight:800;font-size:16px;letter-spacing:-.6px;text-decoration:none;color:inherit}\
nav.top .links{display:flex;gap:4px;flex-wrap:wrap}\
nav.top .lnk{padding:7px 13px;border-radius:9px;font-size:13.5px;font-weight:600;text-decoration:none;color:#5a6478}\
nav.top a.lnk:hover{color:#0a0f1e;background:#eef2fb}\
nav.top .lnk.on{color:#fff;background:#1b57ff}\
nav.top .lnk.off{opacity:.38;cursor:not-allowed}\
@media(prefers-color-scheme:dark){nav.top{background:#0e1524;border-bottom-color:#1b2436}\
nav.top a.lnk:hover{color:#e9efff;background:#16203a}}\
@media(prefers-color-scheme:dark){body{background:#060911;color:#e9efff}table,.bar,.att{background:#0e1524}thead th{background:#0e1524}}\
h1{max-width:1180px;margin:0 auto;padding:2rem 1.25rem .3rem;font-size:1.6rem;letter-spacing:-.5px}\
.lead,.foot{max-width:1180px;margin:0 auto;padding:.2rem 1.25rem;color:#5a6478;font-size:.9rem}\
.foot{padding-top:1.4rem}\
.bar{max-width:1180px;margin:1rem auto;padding:.9rem 1.25rem;display:flex;gap:1rem;align-items:center;\
flex-wrap:wrap;border:1px solid #e4e9f3;border-radius:12px}\
.go{font:inherit;font-weight:600;padding:.5rem 1.1rem;border:0;border-radius:8px;background:#1b57ff;color:#fff;cursor:pointer}\
.go[disabled]{opacity:.5;cursor:progress}\
.go.alt{background:transparent;color:#1b57ff;border:1px solid #1b57ff}\
@media(prefers-color-scheme:dark){.go.alt{color:#7ea2ff;border-color:#7ea2ff}}\
table.xv{width:100%;margin:.5rem 0 0;border:0}\
table.xv td,table.xv th{border-bottom:1px solid #e4e9f3;padding:.4rem .5rem}\
table.xv td:nth-child(n+2):nth-child(-n+5){font-variant-numeric:tabular-nums;text-align:right}\
.say{color:#5a6478;font-size:.85rem}\
table{max-width:1180px;margin:0 auto;width:calc(100% - 2.5rem);border-collapse:collapse;\
border:1px solid #e4e9f3;border-radius:12px;overflow:hidden;font-size:.88rem}\
th,td{text-align:left;padding:.6rem .8rem;border-bottom:1px solid #e4e9f3;vertical-align:top}\
thead th{font-size:.68rem;letter-spacing:.09em;text-transform:uppercase;color:#5a6478}\
.sub{color:#5a6478;font-size:.76rem;margin-top:.15rem}\
.url{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:.78rem;word-break:break-all}\
.tag{display:inline-block;padding:.1rem .45rem;border-radius:999px;font-size:.7rem;letter-spacing:.04em}\
.tag.pub{background:#e7f0ff;color:#1b57ff}.tag.tok{background:#fff0e0;color:#a35200}\
.ok{color:#0a7d3f;font-weight:600}.bad{color:#c02626;font-weight:600}.skip{color:#a35200;font-weight:600}\
.att{max-width:1180px;margin:1.2rem auto;padding:.9rem 1.25rem;border:1px solid #e4e9f3;border-radius:12px;font-size:.82rem}\
.att h3{margin:.2rem 0 .6rem;font-size:.95rem}\
.att ol{margin:0;padding-left:1.3rem}.att li{margin:.2rem 0;font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:.76rem}\
code{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:.85em}";

/// The page's script.
///
/// # Why the ledger is rendered here and not on the server
///
/// The server already emits it as JSON, and rendering it twice — once as HTML
/// for this page and once as JSON for anything else — is two renderings of one
/// fact that must agree forever. `crate::indexmap`'s own header refuses that
/// shape for the same reason. This reads the JSON the route already answers.
const SCRIPT: &str = r#"
const say = (t) => { document.getElementById('say').textContent = t; };
const cell = (file, klass) => document.querySelector(`tr[data-file="${file}"] .${klass}`);

const when = (ms) => ms ? new Date(ms).toLocaleString() : '—';

async function status() {
  try {
    const r = await fetch('/masters/status.json');
    const d = await r.json();
    for (const m of (d.masters || [])) {
      const c = cell(m.file, 'disk');
      if (!c) continue;
      if (!m.present) { c.innerHTML = '<span class="bad">absent</span>'; continue; }
      const stale = m.newer_than_parse
        ? '<div class="sub bad">newer than this server’s parse — restart required</div>'
        : '';
      c.innerHTML = `<span class="ok">present</span><div class="sub">${m.bytes} bytes</div>`
        + `<div class="sub">${when(m.modified_unix_millis)}</div>${stale}`;
    }
  } catch (e) { say('Could not read what is on disk: ' + e); }
}

function ledger(rows) {
  const box = document.getElementById('ledger');
  box.innerHTML = '';
  for (const m of rows) {
    if (!m.attempts || !m.attempts.length) continue;
    const el = document.createElement('div');
    el.className = 'att';
    const steps = m.attempts.map(a => {
      const status = a.status === null ? 'no answer' : a.status;
      const waited = a.waited_ms ? ` after waiting ${a.waited_ms} ms` : '';
      const why = a.detail ? ` — ${a.detail}` : '';
      return `<li>#${a.number} ${a.got} (${status})${waited}${why}</li>`;
    }).join('');
    el.innerHTML = `<h3>${m.file} — ${m.attempts.length} step(s), `
      + `${m.waited_ms} ms waited</h3><ol>${steps}</ol>`;
    box.appendChild(el);
  }
}

async function refresh() {
  const go = document.getElementById('go');
  go.disabled = true;
  say('Asking four hosts. A refused source is retried on a backoff, so this can take a minute.');
  for (const m of document.querySelectorAll('.out')) m.textContent = 'asking…';
  try {
    const r = await fetch('/masters/refresh', { method: 'POST' });
    const d = await r.json();
    if (d.refusal) { say('Refused before anything was asked: ' + d.refusal); go.disabled = false; return; }
    const rows = d.landed || [];
    for (const m of rows) {
      const c = cell(m.file, 'out');
      if (!c) continue;
      if (m.written) {
        c.innerHTML = `<span class="ok">${m.changed ? 'updated' : 'unchanged'}</span>`
          + `<div class="sub">${m.bytes} bytes</div>`;
      } else if (m.skipped) {
        c.innerHTML = `<span class="skip">skipped</span><div class="sub">${m.refusal || ''}</div>`;
      } else {
        c.innerHTML = `<span class="bad">refused</span><div class="sub">${m.refusal || ''}</div>`;
      }
    }
    ledger(rows);
    const missing = d.missing || [];
    say(missing.length
      ? `${missing.length} master(s) still missing: ${missing.join(', ')}.`
      : (d.restart_required
          ? 'All four are on disk. Restart the server so the new masters are parsed.'
          : 'All four are on disk.'));
    await status();
  } catch (e) {
    say('The refresh call itself failed: ' + e);
  } finally {
    go.disabled = false;
  }
}

document.getElementById('go').addEventListener('click', refresh);

// ---- the cross-verification, which is what the four files are FOR ----
//
// `/indexmap.json?feed=X` joins the exchange's own index catalogue against
// one feed's index symbols and reports every row, including the ones it
// could not resolve — those are the symbols whose bars are being filed under
// a name no exchange confirms. It had no page and no nav entry, so the only
// way to see it was to type the URL.
async function verify() {
  const box = document.getElementById('xverify');
  box.innerHTML = '<div class="sub">joining…</div>';
  const feeds = ['dhan', 'groww', 'zerodha'];
  const parts = [];
  for (const feed of feeds) {
    try {
      const r = await fetch(`/indexmap.json?feed=${feed}`);
      const d = await r.json();
      if (d.error) {
        parts.push(`<tr><td><b>${feed}</b></td><td colspan="5" class="bad">${d.error}</td></tr>`);
        continue;
      }
      const refused = d.refused || 0;
      const names = (d.rows || [])
        .filter(x => x.nse === null || x.nse === undefined)
        .map(x => x.symbol);
      parts.push(
        `<tr><td><b>${feed}</b></td>`
        + `<td>${d.published}</td><td>${d.listed}</td>`
        + `<td class="ok">${d.resolved}</td>`
        + `<td class="${refused ? 'bad' : 'ok'}">${refused}</td>`
        + `<td class="sub">${names.slice(0, 12).join(', ')}`
        + `${names.length > 12 ? ` … and ${names.length - 12} more` : ''}</td></tr>`
      );
    } catch (e) {
      parts.push(`<tr><td><b>${feed}</b></td><td colspan="5" class="bad">${e}</td></tr>`);
    }
  }
  box.innerHTML =
    '<h3>Cross&#8209;verification — every feed’s index symbols against NSE’s own catalogue</h3>'
    + '<table class="xv"><thead><tr><th>Feed</th><th>NSE publishes</th><th>Feed lists</th>'
    + '<th>Resolved</th><th>Unconfirmed</th><th>Which ones</th></tr></thead><tbody>'
    + parts.join('') + '</tbody></table>'
    + '<div class="sub">An unconfirmed symbol is one whose bars are filed under a name '
    + 'no exchange confirms. It is never filtered out.</div>';
}

document.getElementById('verify').addEventListener('click', verify);
status();
"#;

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{obtain, outcome_json, refresh_with};
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

    /// A clock that does not wait.
    ///
    /// The retry ladder's own schedule is asserted where the ladder lives —
    /// `pull::masters`. These tests are about which SOURCES were asked and what
    /// reached the page, and paying seven and a half seconds of real backoff per
    /// refused source to learn that would make them tests nobody runs.
    struct NoWait;

    impl masters::Pause for NoWait {
        async fn pause(&self, _ms: u64) {}
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = crate::scratch::path(&format!("masters-{name}"));
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    fn a_master() -> String {
        a_master_for(brutex_core::vendor::Vendor::Dhan)
    }

    /// A body that IS that vendor's master, header and all.
    ///
    /// It used to be a fixed `"symbol,name,isin"` — three column names no
    /// vendor publishes. Every landing test passed on it because `land` only
    /// checked for a comma; the column guard made it fail, correctly. Built
    /// from `required_columns` so it cannot drift from what the reader wants.
    fn a_master_for(vendor: brutex_core::vendor::Vendor) -> String {
        let columns = masters::required_columns(vendor);
        let row = vec!["X"; columns.len()].join(",");
        let mut body = columns.join(",");
        body.push('\n');
        while body.len() <= masters::MIN_BODY_BYTES {
            body.push_str(&row);
            body.push('\n');
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
            // THE SHAPE EACH HOST ACTUALLY ANSWERS WITH. A fake that hands a
            // CSV to the JSON endpoint is not a lenient fake, it is a test of a
            // situation that cannot occur — and it would have to be made to
            // pass by weakening `nse_index_csv`, which is the guard standing
            // between a malformed answer and a wrong catalogue.
            Ok(match masters::SOURCES.iter().find(|s| s.url == url) {
                Some(source) if source.shape == masters::Shape::NseIndexJson => an_index_document(),
                // The prime is a homepage; anything non-empty will do, and the
                // ladder never looks at its body.
                None => "<html></html>".to_owned(),
                // EACH FEED'S OWN HEADER. One body for all three stopped
                // working when `land` began checking columns, and that is the
                // guard doing its job: three vendors publish three different
                // headers, and a fixture that served one to all of them was
                // proving the landing accepts a file the reader cannot parse.
                Some(source) => source
                    .vendor
                    .map_or_else(|| self.body.clone(), a_master_for),
            })
        }
    }

    /// An NSE index document large enough to clear the byte floor once it is
    /// converted to `index_name,category` rows.
    fn an_index_document() -> String {
        let names: Vec<String> = (0..80)
            .map(|n| format!("\"NIFTY TEST INDEX {n}\""))
            .collect();
        format!("{{\"Broad Market Indices\":[{}]}}", names.join(","))
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

        let landed = block_on(refresh_with(&from, &NoWait, &dir, Transport::Public));

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
        // EVERY ADDRESS ASKED BELONGS TO A PUBLIC SOURCE — its URL, one of its
        // mirrors, or its prime. Counting instead of checking membership was
        // the earlier version of this assertion, and it broke the moment a
        // source declared a prime: four requests for three sources looked like
        // a leak and was a session being established. The count was never the
        // property worth holding; **whose host was contacted** is.
        let allowed: Vec<&str> = masters::SOURCES
            .iter()
            .filter(|s| !s.needs_token)
            .flat_map(|s| s.every_url().chain(s.prime))
            .collect();
        for url in &asked {
            assert!(
                allowed.contains(&url.as_str()),
                "a public refresh contacted {url}, which no public source names"
            );
        }
        for source in masters::SOURCES.iter().filter(|s| !s.needs_token) {
            assert!(
                asked.contains(&source.url.to_owned()),
                "{} was never asked for at all",
                source.file
            );
        }
        assert!(
            landed
                .iter()
                .filter(|(_, l, _)| l.is_ok())
                .all(|(_, l, _)| matches!(*l, Ok(ref inner) if inner.is_written())),
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

        let landed = block_on(refresh_with(&from, &NoWait, &dir, Transport::Public));

        assert_eq!(
            landed.len(),
            masters::SOURCES.len(),
            "EVERY source is a row, whether or not this transport may carry it"
        );

        let skipped: Vec<&str> = landed
            .iter()
            .filter(|(_, l, _)| l.is_err())
            .map(|(source, _, _)| source.file)
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
            .find(|(source, _, _)| source.needs_token)
            .map(|(source, l, tried)| outcome_json(source, l, tried))
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
        let landed = block_on(refresh_with(&Dead, &NoWait, &dir, Transport::Public));

        // EVERY source is a row; the ones this transport carried are the ones
        // that were attempted, and all of them failed against a dead host.
        assert_eq!(
            landed.len(),
            masters::SOURCES.len(),
            "every source is a row"
        );
        assert_eq!(
            landed.iter().filter(|(_, l, _)| l.is_ok()).count(),
            masters::SOURCES.iter().filter(|s| !s.needs_token).count(),
            "every public source is still attempted"
        );
        assert!(
            !landed
                .iter()
                .any(|(_, l, _)| matches!(*l, Ok(ref inner) if inner.is_written())),
            "none of them landed"
        );
    }

    #[test]
    fn a_refusal_reaches_the_page_as_a_sentence_and_a_written_false() {
        let source = &masters::SOURCES[0];
        let json = outcome_json(
            source,
            &Ok(Landed::Refused("the host answered 503".to_owned())),
            &masters::Fetched::default(),
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
            &masters::Fetched::default(),
        );
        assert!(written.contains(r#""written":true"#), "{written}");
        assert!(written.contains(r#""changed":false"#), "{written}");
        assert!(written.contains(r#""refusal":null"#), "{written}");
    }

    #[test]
    fn the_attempt_ledger_reaches_the_page_with_every_step_named() {
        // WHICH URL, HOW MANY TIMES, WHAT THE HOST SAID. An operator reading
        // `"refusal": "..."` alone cannot tell one 503 from five, and the
        // difference decides whether they retry or go looking at the host.
        struct Sick;
        impl Discovery for Sick {
            async fn get(&self, url: &str) -> Result<String, Refusal> {
                Err(Refusal::answered(503, format!("{url} answered 503")))
            }
        }

        let dir = scratch("ledger");
        let landed = block_on(refresh_with(&Sick, &NoWait, &dir, Transport::Public));

        let (_, outcome, tried) = landed
            .iter()
            .find(|(source, _, _)| source.file == "dhan_scrip.csv")
            .expect("Dhan is a public source");

        assert_eq!(
            tried.attempts.len(),
            masters::ATTEMPTS_PER_URL as usize,
            "a 503 is retried to the documented ceiling"
        );

        let json = outcome_json(&masters::SOURCES[0], outcome, tried);
        assert!(json.contains(r#""refused_retrying""#), "{json}");
        assert!(json.contains(r#""status":503"#), "{json}");
        assert!(json.contains(r#""number":5"#), "{json}");
        assert!(json.contains(r#""waited_ms":4000"#), "the schedule: {json}");
        assert!(json.contains(r#""written":false"#), "{json}");
    }

    #[test]
    fn a_settled_refusal_reaches_the_page_labelled_as_settled() {
        // THE LABEL IS THE ACTIONABLE HALF. `refused_settled` tells an operator
        // the URL is wrong; `refused_retrying` tells them the host is sick.
        // Both render as a red row, and only one of them is worth waiting out.
        struct Gone;
        impl Discovery for Gone {
            async fn get(&self, url: &str) -> Result<String, Refusal> {
                Err(Refusal::answered(404, format!("{url} answered 404")))
            }
        }

        let dir = scratch("settled");
        let landed = block_on(refresh_with(&Gone, &NoWait, &dir, Transport::Public));
        let (source, outcome, tried) = landed.first().expect("four sources were walked");

        assert_eq!(tried.attempts.len(), 1, "asked once and believed");
        let json = outcome_json(source, outcome, tried);
        assert!(json.contains(r#""refused_settled""#), "{json}");
        assert!(
            json.contains(r#""waited_ms":0"#),
            "nothing was waited: {json}"
        );
    }

    #[test]
    fn a_skipped_source_carries_an_empty_ledger_rather_than_a_missing_one() {
        // "NOT ASKED" AND "ASKED AND GOT NOTHING" ARE DIFFERENT FACTS, and a
        // page that cannot tell them apart will show an operator a red row for
        // a file that a second, credentialed call lands correctly.
        let dir = scratch("skipped-ledger");
        let from = Recording {
            body: a_master(),
            seen: std::sync::Mutex::new(Vec::new()),
        };
        let landed = block_on(refresh_with(&from, &NoWait, &dir, Transport::Public));

        let (source, outcome, tried) = landed
            .iter()
            .find(|(source, _, _)| source.needs_token)
            .expect("Zerodha needs a token and is skipped by a public transport");

        assert!(tried.attempts.is_empty(), "nothing was asked: {tried:?}");
        let json = outcome_json(source, outcome, tried);
        assert!(json.contains(r#""skipped":true"#), "{json}");
        assert!(json.contains(r#""attempts":[]"#), "{json}");
    }

    #[test]
    fn a_transient_refusal_on_the_index_source_still_reaches_the_prime() {
        // THE PRIME IS A ROW IN THE SAME LEDGER, so an operator can see that a
        // session was established -- or that establishing one is what failed,
        // which is a different problem with a different fix.
        let dir = scratch("prime-row");
        let from = Recording {
            body: a_master(),
            seen: std::sync::Mutex::new(Vec::new()),
        };
        let landed = block_on(refresh_with(&from, &NoWait, &dir, Transport::Public));

        let (source, outcome, tried) = landed
            .iter()
            .find(|(source, _, _)| source.file == masters::NSE_INDICES_FILE)
            .expect("the index list is a source");

        assert!(
            tried
                .attempts
                .iter()
                .any(|a| matches!(a.got, masters::Got::Primed)),
            "the prime is recorded: {tried:?}"
        );
        let json = outcome_json(source, outcome, tried);
        assert!(json.contains(r#""primed""#), "{json}");
        assert!(json.contains(r#""written":true"#), "and it landed: {json}");
    }

    #[test]
    fn obtain_names_the_file_when_no_url_answered_at_all() {
        // THE `None`-BODY ARM. A ladder that exhausted every URL must still
        // produce a sentence an operator can read, and `last_refusal` is what
        // supplies it -- the host's own words rather than a digest.
        struct Dead;
        impl Discovery for Dead {
            async fn get(&self, _url: &str) -> Result<String, Refusal> {
                Err(Refusal::transport("no route to host".to_owned()))
            }
        }

        let dir = scratch("no-answer");
        let (landed, tried) = block_on(obtain(&Dead, &NoWait, &dir, &masters::SOURCES[0]));

        // AN ASSERTION AND THEN AN `if let`, rather than a `match` with an
        // `unreachable!` arm. That arm would be a region no run can enter, and
        // this workspace holds a 100% coverage floor — see `docs/06-limits.md`.
        assert!(
            matches!(landed, Landed::Refused(_)),
            "nothing answered, so nothing can be written: {landed:?}"
        );
        if let Landed::Refused(ref why) = landed {
            assert!(why.contains("dhan_scrip.csv"), "names the file: {why}");
            assert!(why.contains("no route to host"), "and the cause: {why}");
        }
        assert_eq!(tried.attempts.len(), masters::ATTEMPTS_PER_URL as usize);
    }

    #[test]
    fn the_page_carries_a_row_for_every_source_and_names_what_each_costs() {
        // A CONTROL SURFACE THAT LISTS THREE OF FOUR MASTERS IS THE DEFECT THIS
        // MODULE ALREADY SHIPPED ONCE, moved from the JSON to the page. The
        // count is asserted against `SOURCES` rather than against `4`, so a
        // fifth master cannot be added and silently left off the page.
        let html = super::page_html();

        for source in masters::SOURCES {
            assert!(
                html.contains(source.file),
                "{} has no row on the page",
                source.file
            );
            assert!(
                html.contains(source.url),
                "{} does not say where it comes from",
                source.file
            );
        }
        assert_eq!(
            html.matches("<tr data-file=").count(),
            masters::SOURCES.len(),
            "one row per source, no more and no fewer"
        );
    }

    #[test]
    fn the_page_separates_the_free_sources_from_the_one_that_spends_a_token() {
        // THE DISTINCTION `needs_token` EXISTS TO CARRY, reaching the operator.
        // Pressing a button that spends a shared credential should not look
        // identical to pressing one that fetches a public CDN file.
        let html = super::page_html();
        assert_eq!(
            html.matches("spends the shared token").count(),
            masters::SOURCES.iter().filter(|s| s.needs_token).count()
        );
        assert_eq!(
            html.matches(r#"class="tag pub""#).count(),
            masters::SOURCES.iter().filter(|s| !s.needs_token).count()
        );
    }

    #[test]
    fn the_page_says_a_prime_happens_where_one_does() {
        // AN UNEXPLAINED EXTRA REQUEST IN THE LEDGER READS AS A BUG. Naming the
        // priming URL on the row is what makes the row above it make sense.
        let html = super::page_html();
        for source in masters::SOURCES {
            if let Some(prime) = source.prime {
                assert!(
                    html.contains(prime),
                    "{} is primed and the page does not say so",
                    source.file
                );
            }
        }
        assert_eq!(
            html.matches("session primed at").count(),
            masters::SOURCES
                .iter()
                .filter(|s| s.prime.is_some())
                .count()
        );
    }

    #[test]
    fn the_page_fetches_nothing_until_the_button_is_pressed() {
        // THE OPERATOR'S STANDING RULE: opening a page must not spend a vendor
        // request. The load path reads `status.json`, which stats four files
        // and opens no socket; `refresh` is bound to a click and to nothing
        // else.
        let html = super::page_html();
        assert!(
            html.contains("addEventListener('click', refresh)"),
            "the refresh is bound to a press"
        );
        assert_eq!(
            html.matches("refresh();").count(),
            0,
            "declared, never invoked on load — `function refresh()` does not match"
        );
        assert!(
            html.contains("status();"),
            "only the disk read runs on load"
        );
    }

    #[test]
    fn the_page_says_a_restart_is_required_rather_than_pretending_otherwise() {
        // `Site::load` PARSES ONCE AT STARTUP. A page that refreshed the bytes
        // and said nothing would leave every other page answering from the boot
        // parse, which is the failure wearing a success's clothes §4 bans.
        let html = super::page_html();
        assert!(html.contains("restart is required"), "on the row");
        assert!(html.contains("Restart the server"), "and after a refresh");
    }

    #[test]
    fn the_page_is_reachable_from_the_navigation() {
        // D-0308 SHIPPED TWO ROUTES AND NO WAY TO REACH EITHER. `crate::logs`
        // records the identical defect in its own words, one page earlier, so
        // this one is pinned rather than remembered.
        let nav = crate::render::dashboard_page("ok", &[], &crate::render::Notes::build(&[]));
        assert!(
            nav.contains("href=\"/masters\""),
            "the masters page is not in the nav"
        );
    }

    #[test]
    fn the_status_answer_carries_when_each_master_was_written() {
        // "PRESENT" IS NOT THE QUESTION. A master present and eleven months old
        // parses cleanly and resolves every renamed symbol to the old row,
        // which is the whole defect this module exists for -- so the age is a
        // field and not something an operator infers.
        let dir = scratch("status-mtime");
        let source = &masters::SOURCES[0];
        std::fs::write(masters::path_of(&dir, source), a_master()).expect("a master on disk");

        let json = super::status_rows(&dir, std::time::SystemTime::UNIX_EPOCH);
        assert!(json.contains(r#""modified_unix_millis":"#), "{json}");
        assert!(
            !json.contains(r#""modified_unix_millis":null,"newer_than_parse":true"#),
            "a file that exists must carry a time: {json}"
        );
        assert!(
            json.contains(r#""modified_unix_millis":null"#),
            "and the three absent ones must carry null rather than a zero \
             that reads as 1970: {json}"
        );
    }

    #[test]
    fn the_page_cross_verifies_every_feed_against_the_exchange() {
        // THE MAPPING IS WHAT THE FOUR FILES ARE FOR. Downloading three vendor
        // masters and NSE's catalogue and never joining them leaves an operator
        // with four files and no answer. `/indexmap.json` did that join and had
        // no page and no nav entry — reachable only by typing the URL.
        let html = super::page_html();

        assert!(html.contains(r#"id="verify""#), "there is a control");
        assert!(html.contains(r#"id="xverify""#), "and somewhere to render");
        for feed in ["dhan", "groww", "zerodha"] {
            assert!(
                html.contains(feed),
                "{feed} is not cross-verified by the page"
            );
        }
        assert!(
            html.contains("/indexmap.json?feed="),
            "the join endpoint is not called"
        );
    }

    #[test]
    fn an_unconfirmed_symbol_is_never_filtered_out_of_the_view() {
        // A SYMBOL THE EXCHANGE DOES NOT CONFIRM IS THE ONE AN OPERATOR MOST
        // NEEDS. `indexmap::json`'s own comment says it omits no row; a page
        // that showed only the counts would undo that at the last step.
        let html = super::page_html();
        assert!(
            html.contains("Unconfirmed"),
            "the count has a column of its own"
        );
        assert!(
            html.contains("Which ones"),
            "and the names are shown, not just tallied"
        );
        assert!(
            html.contains("no exchange confirms"),
            "and the page says what an unconfirmed symbol means"
        );
    }

    #[test]
    fn the_cross_verification_is_also_bound_to_a_press() {
        // SAME RULE AS THE REFRESH. This one reads only local files, but the
        // page must not acquire a habit of doing work nobody asked for.
        let html = super::page_html();
        assert!(
            html.contains("addEventListener('click', verify)"),
            "bound to a press"
        );
        assert_eq!(
            html.matches("verify();").count(),
            0,
            "and never invoked on load"
        );
    }

    #[test]
    fn the_page_renders_the_nav_it_appears_in() {
        // BEING IN THE NAV AND RENDERING ONE ARE DIFFERENT THINGS, and this
        // page had the first without the second — inherited from `crate::logs`,
        // which is self-contained for the same reason and has the same defect.
        // An operator who follows the link lands somewhere with no way back and
        // no indication of where they are.
        let html = super::page_html();

        assert!(html.contains("<nav class=\"top\">"), "there is a nav bar");
        assert!(
            html.contains(r#"class="lnk on" href="/masters""#),
            "and it marks this page as the current one: {html:.0}"
        );
        // EVERY OTHER BUILT PAGE IS REACHABLE FROM HERE. Asserted against the
        // shared `render::nav` rather than a list written here, because a
        // second list would be right the day it was written and wrong the
        // first time a page is added.
        for href in [
            "/dashboard",
            "/instruments",
            "/pull",
            "/audit",
            "/store",
            "/logs",
        ] {
            assert!(
                html.contains(&format!("href=\"{href}\"")),
                "{href} is not reachable from the masters page"
            );
        }
    }
}

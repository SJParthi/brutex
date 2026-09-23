//! HTTP surface. Server-rendered HTML, no JavaScript anywhere.
//!
//! `docs/01-architecture.md` permits this crate `core`, `store`, `engine` and
//! — since D-0038 — `pull`. That is a maximum, not a requirement: `engine`
//! does not exist yet, so it is not taken.
//!
//! | Module | Owns |
//! |---|---|
//! | [`assets`] | the built front end, read off disk at request time |
//! | [`audit`] | what every pull did, on disk, one fixed-stride record each |
//! | [`autopilot`] | the backfill driving itself: what is missing, fetched oldest first |
//! | [`backtest`] | every recorded sweep, read O(1) at a computed offset, newest first |
//! | [`master`] | reading one vendor's instrument master off disk |
//! | [`merge`] | one map from every vendor, and the ISIN cross-check on it |
//! | [`constituents`] | an NSE tier joined to one vendor's ids on `(exchange, ISIN)` |
//! | [`coverage`] | what ONE feed reaches in each spot target, and every name it cannot |
//! | [`folder`] | how far a folder feed reaches, READ off the disk, and the path when it cannot |
//! | [`ingest`] | what the operator asked a pull to do, and every named refusal |
//! | [`census`] | what the store holds, read from the counter file, never from a directory |
//! | [`catalog`] | every ordering and every filter a page offers, decided once at load |
//! | [`render`] | turning instruments into HTML |
//! | [`server`] | the routes, the process, and everything `main` would hold |
//!
//! # Why `pull` is a dependency
//!
//! `/pull` and `/store` are the operator's window onto ingest, and everything
//! they render is a type `pull` already owns: [`pull::session::Day`] is the
//! validated calendar, [`pull::session::Window`] is the inclusive range and the
//! one place the vendor's non-inclusive `toDate` is reconciled,
//! [`pull::session::DropCensus`] is the drop tally, and
//! [`pull::manifest::Manifest`] is the counter file the `/store` page reads
//! instead of walking ~248,000 directory entries. Re-deriving any of them here
//! would be a second definition of a rule that already has one. D-0038.
//!
//! **UNVERIFIED as a measurement.** The bound is argued from the
//! shape of the code and no bench in this workspace times it.
//! `CLAUDE.md` §3 rule 6: a structural argument is not a
//! measurement, however sound it is.

#![forbid(unsafe_code)]

pub mod assets;
pub mod audit;
pub mod audit_json;
pub mod autopilot;
/// THE RESULTS LEDGER, over HTTP -- every recorded sweep, newest first.
/// The engine had a page for what it INGESTED and none for what it FOUND.
pub mod backtest;
pub mod bars;
pub mod booleancampaignjson;
pub mod booleanevidencejson;
pub mod booleanjson;
/// Validated launch and configuration evidence for declared Boolean research.
pub mod booleanlaunch;
pub mod booleanoosjson;
pub mod booleansearchjson;
pub mod calendar;
/// The trading calendar READ OFF THE STORE, so the browser and `pull` stop
/// holding two copies of one fact that nothing checks.
pub mod calendar_of;
/// Exact candidate-side trade pages with immutable capture authority.
pub mod candidatejson;
pub mod catalog;
pub mod census;
pub mod constituents;
pub mod coverage;
/// The shared blocking-work, concurrency and byte/row/page bounds for result details.
pub mod detail;
pub mod expressionsearchjson;
pub mod folder;
/// EVERY TRADE ONE RUN TOOK -- the file that turns seventy-six padlocks
/// into figures. `runs.bin` records totals; this records the round trips
/// those totals are a fold over.
/// One run's ranked combinations as structured JSON, so the page can order
/// them by the operator's own weights rather than by a score baked in here.
pub mod frontierjson;
pub mod indexmap;
/// Authenticated original entry names and archived candle windows.
pub mod indexstopcandlesjson;
/// Bounded read-only native single-stop candidate comparisons.
pub mod indexstopjson;
/// Source-free launch configuration for the Backtest single-stop workflow.
pub mod indexstoplaunch;
/// Exact native qualification and daily/week comparison pages.
pub mod indexstopqualificationjson;
/// Globally ordered comparisons within an exact acknowledged search prefix.
pub mod indexstoprankingjson;
/// Exact saved VIX reference stamps, separate from strategy and qualification evidence.
pub mod indexstopvixjson;
pub mod ingest;
pub mod ladder;
/// WHAT A RUN HAS FOUND SO FAR, while it is still running. `cli::live` shipped a
/// complete reader and this is the caller its own doc names.
pub mod livejson;
pub mod logs;
pub mod master;
/// Refreshing the instrument masters from the browser, and saying when they
/// are stale.
///
/// Separate from `/pull/*` on purpose: that moves BARS and spends the vendor's
/// quota per instrument-month; this moves four files, three of them free public
/// CDN downloads. `Site::load` parses the masters once at startup with no reload
/// path, so this also answers whether a restart is required. D-0308.
pub mod mastersrun;
pub mod merge;
pub mod operation_audit;
pub mod pullrun;
pub(crate) mod recovery;
pub(crate) mod recovery_control;
pub(crate) mod recovery_journal;
pub mod render;
pub mod server;
pub mod sweepevidence;
/// A SWEEP STARTED FROM THE BROWSER -- the half of the console that was
/// missing, because a page that reports on work it cannot start needs a
/// terminal beside it to be useful.
pub mod sweeprun;
pub mod topjson;
pub mod trades;
/// THE SCRUB, over a whole vendor -- the only thing entitled to say a store
/// is verified rather than merely counted.
pub mod verify;

/// Temporary fixture paths, unique per process. Compiled only under `cfg(test)`
/// — it exists so two concurrent test processes cannot delete each other's
/// fixtures, which is a property of the test suite and not of the server.
#[cfg(test)]
pub(crate) mod scratch;

/// The one telemetry sink this test binary installs, and the proof that every
/// reachable `telemetry::emit` site in this crate reaches a file. Compiled only
/// under `cfg(test)` — `telemetry::install` is a process singleton, so the sink
/// has to have exactly one owner and this module is it.
#[cfg(test)]
pub(crate) mod emitted;

#[cfg(test)]
mod saved_response_boundary_tests;

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

#![forbid(unsafe_code)]

pub mod assets;
pub mod audit;
pub mod audit_json;
pub mod autopilot;
/// THE RESULTS LEDGER, over HTTP -- every recorded sweep, newest first.
/// The engine had a page for what it INGESTED and none for what it FOUND.
pub mod backtest;
pub mod bars;
pub mod calendar;
/// The trading calendar READ OFF THE STORE, so the browser and `pull` stop
/// holding two copies of one fact that nothing checks.
pub mod calendar_of;
pub mod catalog;
pub mod census;
pub mod constituents;
pub mod coverage;
pub mod folder;
pub mod indexmap;
pub mod ingest;
pub mod ladder;
pub mod logs;
pub mod master;
pub mod merge;
pub mod pullrun;
pub mod render;
pub mod server;
/// A SWEEP STARTED FROM THE BROWSER -- the half of the console that was
/// missing, because a page that reports on work it cannot start needs a
/// terminal beside it to be useful.
pub mod sweeprun;
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

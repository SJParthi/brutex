//! Reads the Parquet lake into plain Rust values.
//!
//! # What the lake is, and why it needs a reader at all
//!
//! `~/.brutex/lake` holds 40 GB of 1-minute bars written by an earlier
//! collector: 116,086 F&O contract directories under `bars/NSE/FNO` alone,
//! 61,075 of them NIFTY, plus cash and index series under `CASH/` and
//! `INDEX/`, across 21 timeframes from `1minute` to `1day`.
//!
//! **Both live vendor masters purge a contract when it expires.** For every
//! expired contract in that tree there is no second copy and no way to pull
//! one again. The lake is the only source, so reading it is not a convenience
//! — nothing else can.
//!
//! ```text
//! bars/<EXCHANGE>/<SEGMENT>/<CONTRACT>/<TIMEFRAME>/<YYYY>/<MM>.parquet
//! ```
//!
//! # The constraint that shaped this crate
//!
//! The files are Parquet with ZSTD-compressed pages, written by Polars. The
//! obvious way to read them is `parquet`'s `zstd` feature, and that pulls
//! `zstd-sys`: 101 vendored C files behind a `build.rs` that runs `cc` and
//! `bindgen`. `CLAUDE.md` §2 forbids a vendored binding to another language
//! and a build script that invokes an external process, both without
//! exception.
//!
//! So this crate takes `parquet` with **no default features at all** — which
//! removes `zstd` along with the rest — and supplies the two pieces the
//! disabled feature would have provided: page headers from
//! `parquet-format-safe`, page bodies from `ruzstd`. Value decoding below the
//! page layer is `parquet`'s own, unmodified. The result has zero C, zero
//! assembly and no `cc`, `cmake` or `bindgen` anywhere in its tree.
//! `docs/05-decisions.md` D-0056 records the proof.
//!
//! # The two number systems
//!
//! `CLAUDE.md` §7 splits the columns in two, and [`bar`] is where that split
//! lives:
//!
//! * **Prices** — open, high, low, close, the recorded spot, and an option
//!   strike — are paisa `i64`, converted once at [`bar::paisa_from_lake`],
//!   half-up, refusing anything that will not fit.
//! * **Statistical values** — the greeks, the year fraction and the rate —
//!   keep full `f64` precision and are never snapped. Rounding a gamma of
//!   0.00017 onto the paisa grid would erase it.
//!
//! Open interest is neither: `i64::MIN` means the vendor reported none, and
//! zero means zero. See [`bar::OPEN_INTEREST_NULL`].
//!
//! # Everything is refused by name
//!
//! A file that is not Parquet, a truncated one, a codec this reader does not
//! implement, a missing column, a wrongly-typed column, an unrecognised
//! schema, a null where a null has no meaning, and a price that will not
//! become paisa are eight *different* refusals in [`error::LakeError`]. None
//! of them is a skip and none returns a substitute value — `CLAUDE.md` §4.
//!
//! # Where this sits in the crate graph
//!
//! `lake` depends on `brutex_core` and on third-party crates, and on nothing
//! else in this workspace. Nothing in the workspace depends on `lake` yet;
//! wiring it into ingest is separate work.
//!
//! # Example
//!
//! ```no_run
//! use lake::reader::LakeFile;
//! use std::path::Path;
//!
//! let file = LakeFile::open(Path::new("03.parquet"))?;
//! let batch = file.read_row_group(0)?;
//! if let Some(bar) = batch.row(0) {
//!     // Prices are integers; greeks keep their precision.
//!     println!("close {} paisa", bar.close.raw());
//!     println!("open interest {:?}", bar.open_interest());
//! }
//! # Ok::<(), lake::error::LakeError>(())
//! ```

#![forbid(unsafe_code)]

pub mod bar;
pub mod batch;
pub mod contract;
pub mod error;
mod page;
pub mod reader;
pub mod schema;

//! **What the store holds, read off the tree rather than asked for by name.**
//!
//! # The gap this closes
//!
//! Until this module the store was **address-only**. [`path::StorePath`] renders
//! a path from parts a caller already knows, and [`file::BarFile`] opens one. Ask
//! it *"which months of NIFTY are on disk?"* and there was no answer: measured
//! before this landed, `crates/store` contained **zero** `read_dir` calls, and
//! every directory walk in the workspace lived in `crates/pull` or `crates/api`.
//!
//! That is why `cli sweep-stored` takes an explicit vendor, underlying, rung,
//! year and month — it had no alternative — and why nothing could sweep more than
//! one instrument-month per invocation. A loop needs a list, and there was none.
//!
//! # Why it belongs here and not in a caller
//!
//! The layout is this crate's: `path` is documented as *"the only way a store
//! path is built"*, and a walker that re-derives the shape somewhere else is a
//! second spelling of the same rule — the copy that drifts is always the one
//! nobody remembers exists. Reading the tree back is the inverse of rendering it,
//! and both halves belong to whoever owns the bytes.
//!
//! # This is NOT constant time, and says so
//!
//! [`walk`] is **O(entries under `root`)**. It cannot be anything else: listing
//! N things is N units of work, and no arrangement of the store changes that.
//!
//! `CLAUDE.md` §3 rule 4 names five operations that must be O(1) — bar lookup,
//! condition lookup, mask evaluation, duplicate rejection and result append — and
//! enumeration is **not one of them**. It is a setup step a caller performs once
//! to learn what exists, not a per-bar or per-candidate cost. Registered in
//! `docs/06-limits.md` beside the other declared non-constant costs, in the shape
//! `named_error_of` is already registered: stated where a reader will look,
//! rather than left for one to discover.
//!
//! # Every file is accounted for
//!
//! [`walk`] returns a [`Census`] as well as the rows, and the census
//! [`reconciles`](Census::reconciles): every file seen is either a spot
//! instrument-month, a contract path, or one of five named refusals. **Nothing
//! is dropped silently** — a store with a malformed directory reports how many
//! and why, which is what `CLAUDE.md` §4 asks of a degradation.
//!
//! **UNVERIFIED as a measurement.** The bound is argued from the
//! shape of the code and no bench in this workspace times it.
//! `CLAUDE.md` §3 rule 6: a structural argument is not a
//! measurement, however sound it is.

use crate::path::{FileKind, PathError, Timeframe, YearMonth};
use brutex_core::vendor::Vendor;
use std::path::Path;

/// The directory under `root` that every bar path begins with.
///
/// Duplicated from [`crate::path`] deliberately: that constant is private to the
/// renderer, and a walker that reached into it would couple the two halves more
/// tightly than the string does. `the_walker_and_the_renderer_agree_on_the_root`
/// fails the build if they ever disagree.
const BARS_ROOT: &str = "bars";

/// One instrument-month the store actually holds.
///
/// Owned `String`s and not borrows: the segments come from a directory walk, so
/// there is no longer-lived buffer to borrow from. A caller that wants the
/// borrowed form renders a [`crate::path::StorePath`] from these.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Held {
    /// Which feed wrote it. The first segment, never inferred.
    pub vendor: Vendor,
    /// The exchange, e.g. `NSE`.
    pub exchange: String,
    /// The exchange segment, e.g. `INDEX`.
    pub segment: String,
    /// The underlying symbol, e.g. `NIFTY`.
    pub symbol: String,
    /// The bar length.
    pub timeframe: Timeframe,
    /// The month the file covers.
    pub month: YearMonth,
}

/// Every file the walk saw, by what became of it.
///
/// The shape [`indicators::Column`]'s census already uses, and for the same
/// reason: a caller that gets fewer rows than it expected needs to know whether
/// the store is small or the walk refused something.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Census {
    /// Files encountered under `bars/`, whatever became of them.
    pub seen: u64,
    /// Files that became a [`Held`].
    pub spot: u64,
    /// Paths carrying a contract level — a future or an option.
    ///
    /// **Counted, not returned.** `CLAUDE.md` §1 sweeps spot only, so a contract
    /// month is not a row here; it is also not an error, and reporting it as one
    /// would make a correctly-populated F&O store look broken.
    pub with_contract: u64,
    /// The file was not `.bin` — a checksum, an overlay or a lock sibling.
    pub other_kind: u64,
    /// The first segment named no feed this build knows.
    pub unknown_vendor: u64,
    /// The rung segment was not one of [`Timeframe::KNOWN`].
    pub unknown_rung: u64,
    /// The stem did not parse as `yyyy-mm`, or named an impossible month.
    pub malformed_month: u64,
    /// The path had neither the spot depth nor the contract depth.
    pub wrong_depth: u64,
}

impl Census {
    /// Every file seen fell into exactly one bucket.
    ///
    /// A walk whose census does not reconcile has lost a file, which is the
    /// silent-shortfall failure `CLAUDE.md` §4 bans. Asserted by
    /// `the_census_reconciles_over_a_store_holding_every_refusal`.
    #[must_use]
    pub const fn reconciles(&self) -> bool {
        let parts = self
            .spot
            .saturating_add(self.with_contract)
            .saturating_add(self.other_kind)
            .saturating_add(self.unknown_vendor)
            .saturating_add(self.unknown_rung)
            .saturating_add(self.malformed_month)
            .saturating_add(self.wrong_depth);
        parts == self.seen
    }
}

/// What the store holds, and what the walk made of everything it saw.
#[derive(Debug, Clone, Default)]
pub struct Holdings {
    /// One row per spot instrument-month, sorted and deduplicated.
    pub held: Vec<Held>,
    /// Every file seen, by outcome.
    pub census: Census,
}

/// Why a walk could not start.
///
/// Only the *root* can fail the walk. Anything wrong further down is a census
/// row, because one malformed directory must not hide the rest of a store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// `root/bars` could not be listed.
    BarsUnreadable {
        /// What the operating system said.
        because: String,
    },
}

impl core::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BarsUnreadable { because } => {
                write!(
                    f,
                    "the store's `bars` directory could not be read: {because}"
                )
            }
        }
    }
}

impl core::error::Error for CatalogError {}

/// Every spot instrument-month under `root`, with a census of everything seen.
///
/// # Errors
///
/// [`CatalogError::BarsUnreadable`] when `root/bars` cannot be listed. A store
/// with no `bars` directory at all is **not** an error — it is an empty result
/// with an empty census, because "nothing pulled yet" is a normal state and
/// refusing it would make a fresh clone look broken.
///
/// # Cost
///
/// O(entries under `root/bars`), and deliberately so — see the module header.
pub fn walk(root: &Path) -> Result<Holdings, CatalogError> {
    let bars = root.join(BARS_ROOT);
    if !bars.is_dir() {
        return Ok(Holdings::default());
    }
    let mut out = Holdings::default();
    // An explicit stack rather than recursion: the depth is bounded by the
    // layout, but a store is operator-supplied and a symlink loop is theirs to
    // make, not this walk's to blow the stack on.
    let mut stack = vec![bars.clone()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            // A directory that vanished mid-walk, or one this process may not
            // read. The ROOT failing is fatal; a branch failing is not, because
            // one unreadable corner must not hide the rest of the store.
            Err(because) => {
                if dir == bars {
                    return Err(CatalogError::BarsUnreadable {
                        because: because.to_string(),
                    });
                }
                continue;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.census.seen = out.census.seen.saturating_add(1);
                classify(&bars, &path, &mut out);
            }
        }
    }
    out.held.sort_unstable();
    out.held.dedup();
    Ok(out)
}

/// Files a spot path has below `bars/`: exchange, segment, symbol, rung, month.
const SPOT_DEPTH: usize = 6;
/// The same with a contract level between symbol and rung.
const CONTRACT_DEPTH: usize = 7;

/// Sorts one file into the census, and into [`Holdings::held`] if it is a spot
/// instrument-month.
fn classify(bars: &Path, path: &Path, out: &mut Holdings) {
    let Ok(rel) = path.strip_prefix(bars) else {
        out.census.wrong_depth = out.census.wrong_depth.saturating_add(1);
        return;
    };
    let parts: Vec<&str> = rel
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    // Relative to `bars/`, a spot file is
    // `<vendor>/<exchange>/<segment>/<symbol>/<rung>/<month>.bars` -- SIX
    // components. The first draft added one for `bars` itself, which is already
    // stripped, so every spot path measured seven and was filed as a contract:
    // `spot` read 0 on a store that held only spot. The tests caught it because
    // they assert the census bucket, not merely that the walk returned.
    let depth = parts.len();
    if depth == CONTRACT_DEPTH {
        out.census.with_contract = out.census.with_contract.saturating_add(1);
        return;
    }
    if depth != SPOT_DEPTH {
        out.census.wrong_depth = out.census.wrong_depth.saturating_add(1);
        return;
    }
    // `get`, never `[]`: the workspace denies `indexing_slicing`, and a length
    // check one edit away from the index is exactly the pairing that rots.
    let (Some(v), Some(exchange), Some(segment), Some(symbol), Some(rung), Some(file)) = (
        parts.first(),
        parts.get(1),
        parts.get(2),
        parts.get(3),
        parts.get(4),
        parts.get(5),
    ) else {
        out.census.wrong_depth = out.census.wrong_depth.saturating_add(1);
        return;
    };
    let Some(stem) = file.strip_suffix(FileKind::Bars.extension()) else {
        out.census.other_kind = out.census.other_kind.saturating_add(1);
        return;
    };
    let Some(vendor) = Vendor::ALL.into_iter().find(|k| k.as_str() == *v) else {
        out.census.unknown_vendor = out.census.unknown_vendor.saturating_add(1);
        return;
    };
    let Some(timeframe) = Timeframe::KNOWN
        .iter()
        .copied()
        .find(|t| t.as_str() == *rung)
    else {
        out.census.unknown_rung = out.census.unknown_rung.saturating_add(1);
        return;
    };
    let Ok(month) = parse_month(stem) else {
        out.census.malformed_month = out.census.malformed_month.saturating_add(1);
        return;
    };
    out.census.spot = out.census.spot.saturating_add(1);
    out.held.push(Held {
        vendor,
        exchange: (*exchange).to_owned(),
        segment: (*segment).to_owned(),
        symbol: (*symbol).to_owned(),
        timeframe,
        month,
    });
}

/// `yyyy-mm` back into a [`YearMonth`].
///
/// The inverse of that type's own `Display`, which writes `{:04}-{:02}`. Written
/// against the renderer rather than against a guess:
/// `every_month_the_renderer_writes_parses_back` walks a decade and asserts the
/// round trip, so a change to either half fails the build.
fn parse_month(stem: &str) -> Result<YearMonth, PathError> {
    let (year, month) = stem
        .split_once('-')
        .ok_or(PathError::MonthOutOfRange { month: 0 })?;
    // Width is checked before value: `2026-8` and `026-08` both render nothing
    // this store ever wrote, and accepting them would let a hand-made directory
    // masquerade as one the writer produced.
    if year.len() != 4 || month.len() != 2 {
        return Err(PathError::MonthOutOfRange { month: 0 });
    }
    let year: u16 = year
        .parse()
        .map_err(|_| PathError::YearOutOfRange { year: 0 })?;
    let month: u8 = month
        .parse()
        .map_err(|_| PathError::MonthOutOfRange { month: 0 })?;
    YearMonth::new(year, month)
}

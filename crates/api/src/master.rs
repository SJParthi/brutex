//! Reading a vendor instrument master from disk.
//!
//! # Columns are found by NAME, never by position
//!
//! The primary broker publishes 19 columns and ships **21** — `internal_trading_symbol`
//! and `is_intraday` are undocumented — and the documented *order* is wrong:
//! the docs list `lot_size` before `expiry_date`, the file has
//! `expiry_date,strike_price,lot_size`.
//!
//! A positional reader would therefore put a lot size where a strike belongs
//! and never fail, because both are numbers. Every field here is located by
//! header name, and a missing header is a refusal rather than a default.
//!
//! # Which names, though, is the vendor's business
//!
//! The name table itself lives in [`brutex_core::vendor`], reached through
//! [`Vendor::master_columns`]. It was here, as a `match` on the vendor, and it
//! could not stay: [`Vendor`] is `#[non_exhaustive]`, so a match on it in this
//! crate requires a wildcard arm that no test can reach and the coverage gate
//! can never satisfy. Moving the table to the crate that owns the enum makes
//! that match exhaustive and provable, and it puts the column names beside the
//! segment and instrument-type alphabets they belong with — one place to look
//! when a vendor renames a column, rather than one per reader.

use brutex_core::instrument::InstrumentKey;
use brutex_core::isin::Isin;
use brutex_core::vendor::{Decoded, Listing, MasterRow, Skip, Vendor, decode_master_row};
use std::collections::{BTreeMap, HashMap};

/// What one master file produced.
#[derive(Debug, Default)]
pub struct Loaded {
    /// Rows that decoded into an instrument this engine stores, each with the
    /// vendor's ISIN beside its key.
    pub kept: Vec<Listing>,
    /// Where each key sits in [`Self::kept`], so resolving one is a probe.
    ///
    /// # Why this is not a nicety
    ///
    /// The only way to find a listing was to walk `kept`. On the real Dhan
    /// master that is a scan of every kept row, on a request path, to answer a
    /// question `InstrumentKey` was built to answer in one step — its own
    /// documentation says equality and hashing are structural "which is what
    /// makes duplicate rejection a single probe rather than a scan", and
    /// nothing was probing.
    ///
    /// `CLAUDE.md` §3 rule 4 is a PER-OPERATION bound, so it does not soften
    /// because the constant happens to be small on today's universe. The index
    /// costs one `usize` per listing and is built once at startup.
    ///
    /// Conflicting duplicates retain every assertion in `kept` and mark the
    /// key unresolved. Identical duplicates may share one listing.
    by_key: HashMap<InstrumentKey, Option<usize>>,
    /// How many rows reduced to a key another row already held.
    pub duplicate_keys: usize,
    /// Rows declined, counted by reason.
    pub skipped: HashMap<&'static str, usize>,
    /// Every declined row that carried a parseable ISIN, and why it was
    /// declined.
    ///
    /// This is what makes an **eligibility** disagreement visible. Before it
    /// existed a declined row was dropped right here, so the merge could only
    /// ever compare rows both vendors had already agreed to keep, and it
    /// printed `0 conflicts` while the two masters disagreed about 62
    /// instruments that carried the same ISIN in both files. See
    /// [`crate::merge::Merged::eligibility`].
    pub declined: Vec<(Isin, Skip)>,
    /// Rows that could not be understood, with the line number and the reason.
    ///
    /// Held rather than dropped: a row we failed to parse is how an instrument
    /// silently vanishes. [`Loaded::errors_by_reason`] is what actually puts
    /// them on the page — for a long time this vector was allocated, formatted
    /// and then read only for its `.len()`, so "104 unreadable" was the whole
    /// of what an operator was ever told and the reason had to be grepped out
    /// of the raw CSV by hand.
    pub errors: Vec<(usize, String)>,
    /// Every listing class this engine does not recognise, and how often.
    ///
    /// Separate from [`Loaded::skipped`] because the count alone does not name
    /// the code, and the code is the entire diagnostic: "2,438 rows declined
    /// under `EQX`" says an alphabet moved, while "2,438 not an equity
    /// listing" says nothing at all. See
    /// [`brutex_core::vendor::Skip::UnrecognisedListingClass`].
    pub unrecognised: BTreeMap<String, usize>,
    /// Declines that are **not** a routine business outcome.
    ///
    /// [`brutex_core::vendor::Skip::is_routine`] existed and was consulted by
    /// nothing but its own tests, so the distinction it draws -- "a venue we
    /// do not store" against "a code we cannot read" -- reached no status and
    /// no exit code. A vendor renaming `NSE` declined every row of both
    /// masters and the process printed `ok`.
    ///
    /// Counted here so [`crate::server::Read::is_clean`] can refuse to call
    /// that run clean. Kept as a plain count beside the per-code
    /// [`Loaded::unrecognised`] map, because the count is what a monitor reads
    /// and the code is what a human needs.
    pub non_routine: usize,
}

impl Loaded {
    /// Retains conflicting evidence while refusing ambiguous key lookups.
    fn keep(&mut self, listing: Listing) {
        match self.by_key.entry(listing.key) {
            std::collections::hash_map::Entry::Occupied(mut slot) => {
                self.duplicate_keys += 1;
                if slot.get().and_then(|at| self.kept.get(at)) != Some(&listing) {
                    slot.insert(None);
                    self.kept.push(listing);
                }
            }
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(Some(self.kept.len()));
                self.kept.push(listing);
            }
        }
    }

    /// The listing under a key, in one probe.
    ///
    /// This is the whole point of [`Self::by_key`]. Before it existed the only
    /// way to answer "what is this instrument's vendor id" was to walk
    /// [`Self::kept`] — a scan on a request path for a question
    /// `InstrumentKey` was designed to answer in one step.
    #[must_use]
    pub fn listing(&self, key: &InstrumentKey) -> Option<&Listing> {
        self.by_key
            .get(key)
            .copied()
            .flatten()
            .and_then(|at| self.kept.get(at))
    }

    /// The vendor's own id for an instrument, in one probe.
    ///
    /// What a request actually needs: `securityId` for Dhan, `groww_symbol`
    /// for Groww. Returns `None` when this vendor does not list it, which a
    /// caller must refuse on rather than substitute — filing one broker's bars
    /// under another's prefix is what `Feed::store_vendor` calls destroying
    /// D-0019 irreversibly.
    #[must_use]
    pub fn vendor_id(&self, key: &InstrumentKey) -> Option<&brutex_core::vendor::VendorId> {
        self.listing(key).map(|l| &l.vendor_id)
    }

    /// Total rows declined.
    #[must_use]
    pub fn skipped_total(&self) -> usize {
        self.skipped.values().sum()
    }

    /// The decline reasons and their counts, ordered so a report is stable.
    #[must_use]
    pub fn skipped_by_reason(&self) -> Vec<(&'static str, usize)> {
        let mut v: Vec<(&'static str, usize)> =
            self.skipped.iter().map(|(&k, &n)| (k, n)).collect();
        v.sort_unstable();
        v
    }

    /// Each distinct parse failure, how many rows hit it, and the first line
    /// that did.
    ///
    /// Grouped rather than listed row by row so the output is bounded without
    /// being truncated: every *reason* is named, always, and the line number
    /// is where to look. A bare total is what this replaces.
    #[must_use]
    pub fn errors_by_reason(&self) -> Vec<(&str, usize, usize)> {
        let mut by: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
        for (line, reason) in &self.errors {
            let e = by.entry(reason.as_str()).or_insert((0, *line));
            e.0 += 1;
            e.1 = e.1.min(*line);
        }
        by.into_iter()
            .map(|(reason, (n, first))| (reason, n, first))
            .collect()
    }
}

/// Locates the columns this decoder needs, by name.
#[derive(Debug, Clone, Copy)]
struct Columns {
    /// Where the vendor writes its own instrument id.
    vendor_id: usize,
    exchange: usize,
    segment: usize,
    /// `None` when this master has NO underlying column.
    ///
    /// # An empty declared name means ABSENT, not a column called ""
    ///
    /// `MasterColumns` names a header per field, and a vendor may genuinely
    /// publish no such header. Zerodha's instrument dump has twelve columns and
    /// among them is no underlying, no board series and **no ISIN** — its own
    /// `name` column is the COMPANY name, blank on a derivative row, so reading
    /// it as an underlying would put "INFOSYS" where "INFY" belongs.
    ///
    /// Before this was an `Option`, `locate` looked for a column literally
    /// named `""`, failed to find one, and refused the whole master with
    /// `no column ""` — reporting a vendor whose file was perfectly correct as
    /// unreadable. `option_side` already carried this shape for exactly the
    /// same reason, one field over.
    underlying: Option<usize>,
    trading_symbol: usize,
    instrument_type: usize,
    /// `None` when this master carries no board-series column.
    listing_class: Option<usize>,
    /// `None` when this master carries no ISIN column at all.
    isin: Option<usize>,
    expiry: usize,
    strike: usize,
    option_side: Option<usize>,
}

impl Columns {
    /// Finds each required column in the header row.
    ///
    /// # Errors
    ///
    /// The name of the first column that is absent. A missing column is a
    /// refusal and never a default: a defaulted column reads an empty string
    /// for every row, which the decoder would report as thousands of routine
    /// skips rather than as the mapping bug it is.
    fn locate(header: &str, vendor: Vendor) -> Result<Self, String> {
        // PRE-SIZED from the comma count, which is known before the map is
        // built. `collect` starts a `HashMap` at capacity 0 and doubles, so
        // every header paid a run of rehashes to hold a number of columns the
        // header itself had already stated. `docs/07-o1-architecture.md`
        // layer 3, and the same reservation `merge::merge` makes.
        let columns = header.bytes().filter(|b| *b == b',').count() + 1;
        let mut idx: HashMap<&str, usize> = HashMap::with_capacity(columns);
        for (i, name) in header.trim_end().split(',').enumerate() {
            idx.insert(name.trim(), i);
        }
        let need = |n: &str| -> Result<usize, String> {
            idx.get(n)
                .copied()
                .ok_or_else(|| format!("no column {n:?}"))
        };
        // AN EMPTY DECLARED NAME IS AN ABSENT COLUMN, not a column named "".
        // A vendor that publishes no ISIN is described by leaving the name
        // empty, and looking for `""` in the header refused a file that was
        // entirely correct. See `Columns::underlying`.
        let maybe = |n: &str| -> Result<Option<usize>, String> {
            if n.is_empty() {
                return Ok(None);
            }
            need(n).map(Some)
        };
        let names = vendor.master_columns();
        Ok(Self {
            vendor_id: need(names.vendor_id)?,
            exchange: need(names.exchange)?,
            segment: need(names.segment)?,
            underlying: maybe(names.underlying)?,
            trading_symbol: need(names.trading_symbol)?,
            instrument_type: need(names.instrument_type)?,
            listing_class: maybe(names.listing_class)?,
            isin: maybe(names.isin)?,
            expiry: need(names.expiry)?,
            strike: need(names.strike)?,
            option_side: names.option_side.map(need).transpose()?,
        })
    }

    /// The highest column index this decoder reads.
    ///
    /// A row with fewer fields than this cannot be decoded, and must not be
    /// *guessed* at — see the shortfall check in [`load`].
    fn widest(&self) -> usize {
        // Folded from `exchange` rather than `max()`ed over everything,
        // because `Iterator::max` returns an `Option` whose `None` arm cannot
        // happen here — and a branch no test can enter is a branch nobody has
        // checked. `option_side` chains in only for the vendor that has one.
        [
            self.segment,
            self.trading_symbol,
            self.instrument_type,
            self.expiry,
            self.strike,
        ]
        .into_iter()
        // The optional columns widen the row only where the vendor has them.
        .chain(self.underlying)
        .chain(self.listing_class)
        .chain(self.isin)
        .chain(self.option_side)
        .fold(self.exchange, usize::max)
    }
}

/// The largest master file this reader will pull into memory.
///
/// [`load`] reads the whole file before it looks at a single row, which is a
/// deliberate choice — the masters are read once at startup and a streaming
/// reader would buy nothing — but it was previously *unbounded*, so the size of
/// the allocation was whatever the vendor happened to serve.
///
/// Measured 2026-08-01: the real files are 33,990,514 B (Dhan, 200,461 rows)
/// and 19,224,497 B (Groww, 133,379 rows). 256 MiB is 7.9× the larger, which
/// leaves room for years of listing growth and still refuses a file that is
/// not a master at all. The refusal names the size, so an operator who
/// legitimately outgrows this sees the number rather than an out-of-memory
/// kill. D-0033.
pub const MAX_MASTER_BYTES: u64 = 256 * 1024 * 1024;

/// The longest single row this reader will split.
///
/// A row is split into a `Vec<&str>` before any field is examined, so an
/// unbounded row is an unbounded allocation *and* an unbounded number of
/// fields, both paid before [`decode_master_row`]'s own width gate is reached.
///
/// Measured 2026-08-01 over both real masters: the longest row is 486 bytes
/// (Dhan, 33 columns) and 269 bytes (Groww, 21 columns). 4,096 is 8.4× the
/// larger. A row above it is reported at its line number like any other
/// unreadable row — never dropped, never truncated and read anyway.
pub const MAX_ROW_BYTES: usize = 4096;

/// Reads a vendor master and decodes every row.
///
/// # Errors
///
/// A message naming what was wrong with the file itself — unreadable, empty,
/// larger than [`MAX_MASTER_BYTES`], or missing a required column.
pub fn load(path: &std::path::Path, vendor: Vendor) -> Result<Loaded, String> {
    // THE SIZE IS CHECKED BEFORE THE READ, NOT AFTER.
    //
    // `read_to_string` on a file this process cannot hold is not an error it
    // can report -- it is an allocator failure or an OOM kill, and neither
    // reaches the operator as "the master is too big". One `metadata` call is
    // the difference between a named refusal and a dead process.
    let size = std::fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len();
    if size > MAX_MASTER_BYTES {
        return Err(format!(
            "{}: {size} bytes; this reader holds at most {MAX_MASTER_BYTES}",
            path.display()
        ));
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut lines = text.lines();
    let header = lines.next().ok_or_else(|| "file is empty".to_owned())?;
    // THE HEADER IS A ROW, AND IT WAS THE ONE ROW WITH NO BOUND.
    //
    // `MAX_ROW_BYTES` is checked inside the loop below, which iterates what is
    // left AFTER `lines.next()` has already taken the header -- so every data
    // row was guarded and the header was not. That is the D-0033 shape exactly:
    // a scan with nothing bounding its length, sitting immediately above the
    // guard that exists to bound it.
    //
    // It is reachable: a vendor endpoint serving an HTML error page instead of
    // a CSV arrives as one enormous first line. `Columns::locate` then splits
    // it and hashes every field, inside the very function whose premise is
    // that "one `metadata` call is the difference between a named refusal and
    // a dead process".
    if header.len() > MAX_ROW_BYTES {
        return Err(format!(
            "{}: header row is {} bytes; this reader splits at most {MAX_ROW_BYTES}",
            path.display(),
            header.len()
        ));
    }
    let cols = Columns::locate(header, vendor)?;

    let widest = cols.widest();
    let mut out = Loaded::default();
    // ONE field vector for the whole file, cleared and refilled per row.
    //
    // `line.split(',').collect()` starts at capacity 0 and doubles, because
    // `Split`'s `size_hint` is `(0, None)` -- five allocations and four
    // memcpys for a 33-column Dhan row, about a million malloc/free pairs
    // across 200,461 rows. The column count is fixed by the header, so the
    // capacity is known before the loop starts. `docs/07-o1-architecture.md`
    // law 2, and layer 9's rule against allocating inside a loop.
    let mut f: Vec<&str> = Vec::with_capacity(widest + 1);
    for (n, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        // BEFORE THE SPLIT, NOT AFTER. `split(',').collect()` allocates one
        // pointer pair per comma, and `decode_master_row`'s own width gate
        // cannot run until that vector exists. `str::len` is a field read.
        if line.len() > MAX_ROW_BYTES {
            out.errors.push((
                n + 2,
                format!(
                    "row is {} bytes; this reader splits at most {MAX_ROW_BYTES}",
                    line.len()
                ),
            ));
            continue;
        }
        f.clear();
        f.extend(line.split(','));
        // A ROW TOO SHORT TO HOLD THE COLUMNS IS AN ERROR, NOT A DEFAULT.
        //
        // Defaulting a missing field to `""` put the empty string into the
        // listing class, where the gate read it as a series it does not
        // recognise and declined a genuine share as routine business, with
        // `0 unreadable` beside it. `Columns::locate` already refuses a
        // missing HEADER for exactly this hazard; this is the same hazard one
        // row at a time, and the real masters are uniform — every Dhan row has
        // 33 fields and every Groww row has 21 — so a short row is an anomaly
        // and never the normal case.
        if f.len() <= widest {
            out.errors.push((
                n + 2,
                format!(
                    "row has {} field(s); the columns this vendor needs run to {}",
                    f.len(),
                    widest + 1
                ),
            ));
            continue;
        }
        let get = |i: usize| -> &str { f.get(i).copied().unwrap_or("") };
        let row = MasterRow {
            vendor_id: get(cols.vendor_id),
            exchange: get(cols.exchange),
            segment: get(cols.segment),
            underlying: cols.underlying.map_or("", get),
            trading_symbol: get(cols.trading_symbol),
            instrument_type: get(cols.instrument_type),
            listing_class: cols.listing_class.map_or("", get),
            isin: cols.isin.map_or("", get),
            expiry: get(cols.expiry),
            strike_rupees: get(cols.strike),
            option_side: cols.option_side.map_or("", get),
        };
        match decode_master_row(vendor, row) {
            Ok(Decoded::Keep(l)) => out.keep(l),
            // The reason text belongs to the Skip variant, in the crate that
            // owns it. A `match` here would need a wildcard -- Skip is
            // `#[non_exhaustive]` -- and a wildcard silently files every
            // variant added later under whatever label it happens to name.
            Ok(Decoded::Skipped(d)) => {
                *out.skipped.entry(d.reason.reason()).or_insert(0) += 1;
                // Every non-routine decline, whatever its variant. Asking the
                // `Skip` itself means a variant added later is counted here
                // the moment it declares itself non-routine, rather than
                // needing this site to be remembered.
                if !d.reason.is_routine() {
                    out.non_routine += 1;
                }
                if let Some(isin) = d.isin {
                    out.declined.push((isin, d.reason));
                }
                // The COUNT says an alphabet moved; the CODE says which one.
                if d.reason == Skip::UnrecognisedListingClass {
                    // `entry(k.to_owned())` allocates the key on EVERY row,
                    // including the overwhelming majority where the code has
                    // been seen before -- and the scenario this counter exists
                    // to detect (a whole series renamed) is exactly the one
                    // where every equity row lands here. Look first, allocate
                    // only when the code is genuinely new.
                    let code = row.listing_class.trim();
                    if let Some(n) = out.unrecognised.get_mut(code) {
                        *n += 1;
                    } else {
                        out.unrecognised.insert(code.to_owned(), 1);
                    }
                }
            }
            Err(e) => out.errors.push((n + 2, e.to_string())),
        }
    }
    // THE UNIVERSE, AS IT ACTUALLY PARSED — one line for a file of ~100,000
    // rows, never one per row.
    //
    // A wrong universe is the quietest way a run goes wrong: nothing refuses,
    // nothing fails, the pull simply asks for a different set of instruments
    // than the operator believes it did. `Swept indices` sending 785 requests
    // instead of 2 was exactly that, and the only trace it left was a receipt
    // nobody kept. The kept count and the skip tally are what make a changed
    // vendor master visible on the day it changes rather than a month later.
    //
    // `Info` and once per master file, so two lines per process start.
    note_parsed(path, vendor, &out);
    Ok(out)
}
/// The universe, as it actually parsed — one line for a file of ~100,000 rows.
///
/// # Why a count and not a row
///
/// A wrong universe is the quietest way a run goes wrong: nothing refuses,
/// nothing fails, the pull simply asks the vendor for a different set of
/// instruments than the operator believes it did. `Swept indices` sending 785
/// requests instead of 2 was exactly that, and the only trace it left was a
/// receipt nobody kept.
///
/// Per row would be ~100,000 lines per vendor per start. These seven counts are
/// what actually changes when a vendor edits its master, and they are what make
/// that edit visible on the day it happens rather than a month later.
///
/// `Info`, once per master file — two lines per process start.
fn note_parsed(path: &std::path::Path, vendor: Vendor, out: &Loaded) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info("api.master", "parsed")
            .with("vendor", telemetry::Value::Str(vendor.as_str()))
            .with("path", telemetry::Value::Str(&path.display().to_string()))
            .with("kept", telemetry::Value::Uint(out.kept.len() as u64))
            .with(
                "duplicate_keys",
                telemetry::Value::Uint(out.duplicate_keys as u64),
            )
            .with(
                "skipped",
                telemetry::Value::Uint(out.skipped_total() as u64),
            )
            .with(
                "declined",
                telemetry::Value::Uint(out.declined.len() as u64),
            )
            .with(
                "row_errors",
                telemetry::Value::Uint(out.errors.len() as u64),
            )
            .with(
                "unrecognised_classes",
                telemetry::Value::Uint(out.unrecognised.len() as u64),
            ),
    );
}

/// A Kite instrument dump, decoded through the same path as every other master.
///
/// # Why this is a TEST and not a module
///
/// There is nothing to write. `Vendor::Zerodha`'s `MasterColumns` already names
/// the twelve headers, `Columns::locate` already maps the three it does not
/// publish to `None`, and `load` already walks any CSV. What was missing was
/// the evidence that those three pieces agree on a real body — and they did not
/// until 14 Aug 2026, when every equity row was declined for a series column
/// this vendor has never had.
///
/// The rows below are the vendor's own documented sample, from
/// `docs/00-charter.md` §4d.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod zerodha_master_tests {
    use super::*;

    // INDENTED, AND THE INDENT COSTS NOTHING. A `\` before the newline is
    // Rust's string-continuation escape: it eats the newline AND every leading
    // space on the next line, so this literal is byte-identical to the flush-
    // left version it replaces. It is written this way because gate 11 refuses
    // to brace-count Rust (braces live inside strings, so counting them is
    // unsound) and delimits a `#[cfg(test)]` module by INDENT instead. Flush-
    // left CSV rows inside a test module are indistinguishable from top-level
    // items to that scanner, and it declared the module undelimited rather
    // than guess -- which aborted gate 11 before a single rule ran, on every
    // file in the workspace. The scanner is right to refuse; the string is
    // what had to move.
    const DUMP: &str = "instrument_token,exchange_token,tradingsymbol,name,last_price,expiry,\
        strike,tick_size,lot_size,instrument_type,segment,exchange\n\
        408065,1594,INFY,INFOSYS,0,,0,0.05,1,EQ,NSE,NSE\n\
        738561,2885,RELIANCE,RELIANCE INDUSTRIES,0,,0,0.05,1,EQ,NSE,NSE\n\
        5720322,22345,NIFTY15DECFUT,,78.0,2015-12-31,0,0.05,75,FUT,NFO-FUT,NFO\n\
        5720578,22346,NIFTY159500CE,,23.0,2015-12-31,9500,0.05,75,CE,NFO-OPT,NFO\n";

    /// NAMED PER TEST, because these run in parallel and shared a directory.
    /// One test's write raced another's read and the loser saw "file is empty"
    /// — a fixture defect that reads exactly like a decoder defect.
    fn decode(name: &str, body: &str) -> Loaded {
        let dir = crate::scratch::path(&format!("zerodha-master-{name}"));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let at = dir.join(Vendor::Zerodha.master_file());
        std::fs::write(&at, body).expect("write");
        load(&at, Vendor::Zerodha).expect("the dump decodes")
    }

    /// **THE REGRESSION.** Measured before the fix: `kept = 0`, every equity
    /// declined as `UnrecognisedListingClass` — because the gate wanted an NSE
    /// board series and this master has no series column at all.
    #[test]
    fn an_equity_row_is_kept_although_this_vendor_publishes_no_series_column() {
        let got = decode("kept", DUMP);
        let kept: Vec<&str> = got.kept.iter().map(|l| l.key.underlying.as_str()).collect();
        assert!(
            kept.contains(&"INFY") && kept.contains(&"RELIANCE"),
            "both NSE equities are kept; got {kept:?}"
        );
        assert_eq!(kept.len(), 2, "and the two derivatives are declined");
    }

    /// The numeric token is the id a request is addressed by — not the
    /// `exchange_token` beside it, and not the `tradingsymbol`.
    #[test]
    fn the_vendor_id_is_the_instrument_token_and_not_its_neighbour() {
        let got = decode("vendorid", DUMP);
        let infy = got
            .kept
            .iter()
            .find(|l| l.key.underlying.as_str() == "INFY")
            .expect("INFY is kept");
        assert_eq!(
            infy.vendor_id.as_str(),
            "408065",
            "instrument_token, not exchange_token 1594 and not the symbol"
        );
    }

    /// **NO ISIN, AND THAT IS A STATE RATHER THAN A GAP.**
    ///
    /// Twelve columns and not one of them is an ISIN, so every row decodes with
    /// none. `Columns::locate` maps the absent header to `None` — before that,
    /// it looked for a column literally named `""` and refused the whole file
    /// with `no column ""`.
    #[test]
    fn every_row_decodes_with_no_isin_because_the_vendor_publishes_none() {
        let got = decode("noisin", DUMP);
        assert!(
            got.kept.iter().all(|l| l.isin.is_none()),
            "this master carries no ISIN column, so no row can carry one"
        );
        assert!(
            got.errors.is_empty(),
            "an absent column is not a malformed file: {:?}",
            got.errors
        );
    }

    /// The company name is not read as the underlying. `name` is `INFOSYS` for
    /// an equity and blank on a derivative; reading it as the underlying would
    /// put INFOSYS where INFY belongs.
    #[test]
    fn the_company_name_column_never_becomes_the_underlying() {
        let got = decode("company", DUMP);
        assert!(
            got.kept
                .iter()
                .all(|l| l.key.underlying.as_str() != "INFOSYS"),
            "`name` is the company, and this master has no underlying column"
        );
    }

    /// The two derivative rows are accounted for BY NAME, not dropped.
    ///
    /// They are on `NFO` and this engine stores NSE only (D-0017), so they are
    /// skipped as a foreign exchange — before their instrument type is ever
    /// consulted. That is the right order: a venue this build does not store is
    /// not a row it needs to understand.
    #[test]
    fn a_row_on_another_exchange_is_named_rather_than_kept_or_lost() {
        let got = decode("buckets", DUMP);
        let skipped: usize = got.skipped_by_reason().iter().map(|(_, n)| n).sum();
        assert_eq!(
            got.kept.len() + got.declined.len() + skipped + got.errors.len(),
            4,
            "every row lands in exactly one bucket: kept {} declined {} \
             skipped {skipped} errors {:?}",
            got.kept.len(),
            got.declined.len(),
            got.errors
        );
        // THE TWO `NFO` ROWS, AND THIS ASSERTION USED TO SAY "foreign exchange".
        //
        // `Exchange::parse` knows `NSE` and `BSE` and nothing else, so `NFO`
        // returns `Err` rather than a venue this build declines to store. Those
        // are two different facts and they were one outcome: a code the engine
        // CANNOT PARSE and a venue it CHOOSES NOT TO STORE both read as routine,
        // so `NSE` drifting to `NSE_EQ` would decline every row of both masters
        // while the report printed `ok` and exited zero. D-0329 separated them,
        // and this assertion is the one that had pinned the conflation.
        //
        // A real `BSE` row still reads as `foreign exchange`, which is what
        // makes the distinction worth having rather than a rename.
        assert_eq!(
            got.skipped_by_reason(),
            vec![("unrecognised exchange", 2)],
            "the two NFO rows: a code no decoder parses is not a venue"
        );
        assert!(
            got.errors.is_empty(),
            "and nothing is malformed: {:?}",
            got.errors
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn tmp(name: &str, body: &str) -> std::path::PathBuf {
        let p = crate::scratch::path(&format!("master-{name}.csv"));
        let mut f = std::fs::File::create(&p).expect("create");
        f.write_all(body.as_bytes()).expect("write");
        p
    }

    #[test]
    fn columns_are_found_by_name_not_position() {
        // The columns are deliberately in a DIFFERENT order from the docs and
        // carry two undocumented trailing fields, exactly like the real file.
        let body = "is_intraday,segment,exchange,instrument_type,groww_symbol,\
                    trading_symbol,series,isin,expiry_date,strike_price,\
                    underlying_symbol,internal_trading_symbol\n\
                    0,CASH,NSE,IDX,NSE-NIFTY,NIFTY,,NIFTY,,,,x\n";
        let p = tmp("byname", body);
        let got = load(&p, Vendor::Groww).expect("loads");
        assert_eq!(got.kept.len(), 1);
        assert_eq!(got.kept[0].key.underlying.as_str(), "NIFTY");
        assert!(got.kept[0].key.is_sweepable());
        assert_eq!(got.kept[0].isin, None, "an index has no ISIN");
        assert!(got.errors.is_empty());
    }

    #[test]
    fn duplicate_conflicting_assertions_survive_loading_and_refuse_lookup() {
        let header = "segment,exchange,instrument_type,groww_symbol,trading_symbol,series,isin,expiry_date,strike_price,underlying_symbol\n";
        let first = "CASH,NSE,EQ,one,RELIANCE,EQ,INE002A01018,,,\n";
        for (case, second) in [
            ("id", "CASH,NSE,EQ,two,RELIANCE,EQ,INE002A01018,,,\n"),
            ("isin", "CASH,NSE,EQ,one,RELIANCE,EQ,INE009A01021,,,\n"),
        ] {
            for reverse in [false, true] {
                let body = if reverse {
                    format!("{header}{second}{first}{first}")
                } else {
                    format!("{header}{first}{second}{first}")
                };
                let loaded = load(
                    &tmp(&format!("duplicate-{case}-{reverse}"), &body),
                    Vendor::Groww,
                )
                .expect("loads");
                assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
                assert_eq!(loaded.duplicate_keys, 2);
                assert!(
                    loaded
                        .kept
                        .iter()
                        .any(|l| l.isin == Isin::new("INE002A01018").ok())
                );
                assert!(loaded.kept.len() >= 2);
                assert_eq!(loaded.listing(&loaded.kept[0].key), None);
                assert_eq!(loaded.vendor_id(&loaded.kept[0].key), None);
                let merged = crate::merge::merge(&[crate::merge::Source {
                    vendor: Vendor::Groww,
                    kept: loaded.kept,
                    declined: loaded.declined,
                }]);
                assert_eq!(merged.assertions.len(), 2);
                assert!(
                    merged
                        .by_key
                        .values()
                        .all(|e| e.ids[Vendor::Groww as usize].is_none())
                );
            }
        }
        let loaded = load(
            &tmp("duplicate-identical", &format!("{header}{first}{first}")),
            Vendor::Groww,
        )
        .expect("loads");
        assert_eq!(loaded.kept.len(), 1);
        assert_eq!(loaded.duplicate_keys, 1);
        assert!(loaded.vendor_id(&loaded.kept[0].key).is_some());
    }

    #[test]
    fn a_missing_column_is_refused_and_named() {
        let p = tmp("missing", "exchange,segment,groww_symbol\nNSE,CASH,NSE-X\n");
        let err = load(&p, Vendor::Groww).expect_err("must refuse");
        // The FIRST missing column is named, whichever it is -- the point is
        // that it refuses and says which, never that it defaults.
        assert!(err.starts_with("no column"), "got {err}");
        assert!(err.contains("underlying_symbol"), "got {err}");
    }

    #[test]
    fn every_column_a_vendor_needs_is_required_and_named_when_absent() {
        // One case per required column, built by REMOVING that column from an
        // otherwise complete header. A column that could go missing without a
        // refusal would be read as an empty string on every row, and the
        // decoder would report that as thousands of routine skips rather than
        // as the mapping bug it is.
        for vendor in [Vendor::Groww, Vendor::Dhan] {
            let c = vendor.master_columns();
            let mut all = vec![
                c.vendor_id,
                c.exchange,
                c.segment,
                c.underlying,
                c.trading_symbol,
                c.instrument_type,
                c.listing_class,
                c.isin,
                c.expiry,
                c.strike,
            ];
            all.extend(c.option_side);
            // The complete header is accepted, or the cases below prove
            // nothing about which column was missing.
            let full = format!("{}\n", all.join(","));
            assert!(
                load(&tmp("full", &full), vendor).is_ok(),
                "the complete header must load"
            );
            for missing in &all {
                let header: Vec<&str> = all.iter().copied().filter(|n| n != missing).collect();
                let body = format!("{}\n", header.join(","));
                let err = load(&tmp("dropped", &body), vendor).expect_err("must refuse");
                assert!(
                    err.contains(missing),
                    "dropping {missing} must name it, got {err}"
                );
            }
        }
    }

    #[test]
    fn the_new_class_and_isin_columns_are_required_by_their_own_names() {
        // Each vendor spells them differently, and a file missing either is a
        // refusal that NAMES the column -- never a silent empty string, which
        // the decoder would report as thousands of routine skips.
        let groww = "exchange,segment,underlying_symbol,trading_symbol,instrument_type,expiry_date,strike_price,groww_symbol";
        let err = load(&tmp("noseries", &format!("{groww},isin\n")), Vendor::Groww)
            .expect_err("must refuse");
        assert!(err.contains("\"series\""), "got {err}");
        let err = load(&tmp("noisin", &format!("{groww},series\n")), Vendor::Groww)
            .expect_err("must refuse");
        assert!(err.contains("\"isin\""), "got {err}");

        let dhan = "EXCH_ID,SEGMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,INSTRUMENT,SM_EXPIRY_DATE,STRIKE_PRICE,OPTION_TYPE,SECURITY_ID";
        let err = load(&tmp("noclass", &format!("{dhan},ISIN\n")), Vendor::Dhan)
            .expect_err("must refuse");
        assert!(err.contains("\"SERIES\""), "got {err}");
        let err = load(
            &tmp("nodhanisin", &format!("{dhan},SERIES\n")),
            Vendor::Dhan,
        )
        .expect_err("must refuse");
        assert!(err.contains("\"ISIN\""), "got {err}");
        // The side column is Dhan's alone, and it is required there.
        let err = load(
            &tmp(
                "noside",
                "EXCH_ID,SEGMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,INSTRUMENT,SM_EXPIRY_DATE,STRIKE_PRICE,ISIN,SERIES,SECURITY_ID\n",
            ),
            Vendor::Dhan,
        )
        .expect_err("must refuse");
        assert!(err.contains("\"OPTION_TYPE\""), "got {err}");
        // And a file WITHOUT `INSTRUMENT_TYPE` loads: it is the measurably
        // wrong column, so it is neither read nor required. D-0025.
        assert!(
            load(
                &tmp(
                    "noinstrtype",
                    "EXCH_ID,SEGMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,INSTRUMENT,SM_EXPIRY_DATE,STRIKE_PRICE,ISIN,SERIES,OPTION_TYPE,SECURITY_ID\n",
                ),
                Vendor::Dhan,
            )
            .is_ok()
        );
    }

    #[test]
    fn the_equity_gate_runs_on_a_real_file_and_the_bond_loses() {
        // The CHOLAFIN pair, in the order the real Dhan master has them: the
        // 7.5% NCD FIRST, the share second. Before the gate, insert-if-absent
        // resolved the ticker to the bond. Both lines are verbatim, and the
        // column the gate reads is SERIES -- `D1` for the NCD, `EQ` for the
        // share -- not the INSTRUMENT_TYPE beside it.
        let body = "EXCH_ID,SEGMENT,ISIN,INSTRUMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,INSTRUMENT_TYPE,SERIES,SM_EXPIRY_DATE,STRIKE_PRICE,OPTION_TYPE,SECURITY_ID\nNSE,E,INE121A08PJ0,EQUITY,CHOLAFIN,CHOLAMANDALAM IN & FIN CO,DEB,D1,,,,1333\nNSE,E,INE121A01024,EQUITY,CHOLAFIN,CHOLAMANDALAM IN & FIN CO,ES,EQ,,,,1333\n";
        let got = load(&tmp("cholafin", body), Vendor::Dhan).expect("loads");
        assert_eq!(got.kept.len(), 1, "only the share survives");
        assert_eq!(got.kept[0].key.underlying.as_str(), "CHOLAFIN");
        assert_eq!(
            got.kept[0].isin.map(|i| i.to_string()).as_deref(),
            Some("INE121A01024"),
            "and it is the SHARE, identified by its own ISIN"
        );
        assert_eq!(got.skipped.get("not an equity listing"), Some(&1));
        assert!(got.errors.is_empty());
        // The bond's ISIN survives the decline, so another vendor keeping the
        // same paper can be recognised as a disagreement.
        assert_eq!(got.declined.len(), 1);
        assert_eq!(got.declined[0].0.as_str(), "INE121A08PJ0");
        assert_eq!(got.declined[0].1, Skip::NotEquityListing);
        assert!(got.unrecognised.is_empty(), "D1 is a series we know");
    }

    #[test]
    fn an_unrecognised_series_is_recorded_under_the_code_itself() {
        // A count under a shared label cannot distinguish "NSE minted a debt
        // series" from "the vendor renamed the equity series". The CODE can.
        let body = "EXCH_ID,SEGMENT,ISIN,INSTRUMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,INSTRUMENT_TYPE,SERIES,SM_EXPIRY_DATE,STRIKE_PRICE,OPTION_TYPE,SECURITY_ID\nNSE,E,INE002A01018,EQUITY,RELIANCE,RELIANCE INDUSTRIES,ES,  EQX  ,,,,1333\nNSE,E,INE121A01024,EQUITY,CHOLAFIN,CHOLA,ES,EQX,,,,1333\nNSE,E,INE775A08105,EQUITY,MOTHERSON,MOTHERSON NCD,DEB,D1,,,,1333\n";
        let got = load(&tmp("unrecognised", body), Vendor::Dhan).expect("loads");
        assert_eq!(got.kept.len(), 0);
        assert_eq!(got.skipped.get("unrecognised listing class"), Some(&2));
        assert_eq!(got.skipped.get("not an equity listing"), Some(&1));
        // Trimmed, so the padded and unpadded forms are ONE code and not two.
        assert_eq!(got.unrecognised.get("EQX"), Some(&2));
        assert_eq!(got.unrecognised.len(), 1);
    }

    #[test]
    fn a_row_with_too_few_fields_is_unreadable_and_names_the_shortfall() {
        // A defaulted field landed the empty string in the listing class,
        // where the gate declined a genuine share as routine business with
        // `0 unreadable` beside it. `Columns::locate` already refuses a
        // missing HEADER for this hazard; this is the same hazard per row.
        let body = "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,expiry_date,strike_price,groww_symbol\nNSE,CASH,,RELIANCE,EQ\nNSE,CASH,,CHOLAFIN,EQ,EQ,INE121A01024,,,NSE-X\n";
        let got = load(&tmp("shortrow", body), Vendor::Groww).expect("loads");
        assert_eq!(got.kept.len(), 1, "only the intact row decodes");
        assert_eq!(got.errors.len(), 1);
        assert_eq!(got.errors[0].0, 2, "the line is named");
        // Bound rather than called inside the failure message: a call there is
        // a region that only runs when the assertion FAILS, so no passing test
        // can ever cover it.
        let reason = &got.errors[0].1;
        assert!(
            reason.contains("row has 5 field(s)") && reason.contains("run to 9"),
            "got {reason}"
        );
        assert!(
            got.skipped.is_empty(),
            "a truncated share is not a routine decline: {:?}",
            got.skipped
        );
    }

    #[test]
    fn unreadable_rows_are_grouped_by_reason_with_the_first_line_that_hit_it() {
        // `errors` was allocated, formatted and read only for `.len()`, so an
        // operator was told `104 unreadable` and nothing else.
        // The fixture used to be `NIFTY 100` and `NIFTY 200`, chosen because a
        // space was not a legal Symbol and 104 rows of the real master were
        // exactly that shape. D-0147 made those rows READABLE -- an index name
        // is normalised to the exchange's canonical ticker -- so they are no
        // longer an example of anything unreadable. A period still is: the
        // allowlist is `A-Z 0-9 - _ &` and nothing else, and confining the
        // collapse to spaces is what keeps that true.
        let body = "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,expiry_date,strike_price,groww_symbol\nNSE,CASH,,NIFTY.100,IDX,,NIFTY,,,NSE-X\nNSE,CASH\nNSE,CASH,,NIFTY.200,IDX,,NIFTY,,,NSE-X\n";
        let got = load(&tmp("grouped", body), Vendor::Groww).expect("loads");
        let by = got.errors_by_reason();
        assert_eq!(by.len(), 2, "two distinct reasons: {by:?}");
        assert_eq!(
            by,
            vec![
                ("malformed instrument identifier", 2, 2),
                (
                    "row has 2 field(s); the columns this vendor needs run to 9",
                    1,
                    3
                ),
            ],
            "every reason named, with its count and the FIRST line it hit"
        );
        assert!(
            Loaded::default().errors_by_reason().is_empty(),
            "and nothing to say when nothing failed"
        );
    }

    #[test]
    fn an_unreadable_or_empty_file_is_refused() {
        let missing = crate::scratch::path("does-not-exist.csv");
        assert!(load(&missing, Vendor::Groww).is_err());
        let p = tmp("empty", "");
        assert!(
            load(&p, Vendor::Groww)
                .expect_err("empty")
                .contains("empty")
        );
        // A file whose SIZE is fine but whose BYTES are not text. `metadata`
        // succeeds and the read is what fails, so both refusal arms are real.
        let raw = crate::scratch::path("master-notutf8.csv");
        std::fs::File::create(&raw)
            .expect("create")
            .write_all(&[0xFF, 0xFE, 0x00, 0x41])
            .expect("write");
        assert!(load(&raw, Vendor::Groww).is_err());
    }

    #[test]
    fn a_master_larger_than_this_reader_holds_is_refused_before_it_is_read() {
        // `read_to_string` on a file this process cannot hold is not an error
        // it can report -- it is an OOM kill. The size is checked first, and
        // the refusal names the number so an operator who legitimately outgrows
        // the bound is told what to raise rather than losing the process.
        //
        // Sparse: `set_len` allocates no blocks, so this costs no disk. The
        // file is never read -- the refusal happens before `read_to_string`,
        // which is exactly the property under test.
        //
        // THE PATH CARRIES THIS PROCESS'S ID. It used to be one fixed name in
        // the shared temporary directory, and the `remove_file` below then
        // failed with NotFound about one run in three: a second process running
        // the same test deleted the fixture while this one was inside the
        // 256 MiB read. See `crate::scratch`.
        let p = crate::scratch::path("master-toobig.csv");
        let f = std::fs::File::create(&p).expect("create");
        f.set_len(MAX_MASTER_BYTES + 1).expect("set_len");
        drop(f);
        let err = load(&p, Vendor::Groww).expect_err("refused");
        assert!(
            err.contains(&(MAX_MASTER_BYTES + 1).to_string())
                && err.contains(&MAX_MASTER_BYTES.to_string()),
            "the refusal names both numbers: {err}"
        );
        // Exactly at the bound is not over it.
        f_at_bound(&p);
        // The cleanup is an ASSERTION, not housekeeping: this fixture is
        // 256 MiB and nothing else may have removed it.
        std::fs::remove_file(&p).expect("cleanup");
    }

    /// A file of exactly [`MAX_MASTER_BYTES`] is refused for being empty of
    /// columns, not for its size — the boundary is `>` and not `>=`.
    fn f_at_bound(p: &std::path::Path) {
        let f = std::fs::File::create(p).expect("create");
        f.set_len(MAX_MASTER_BYTES).expect("set_len");
        drop(f);
        let err = load(p, Vendor::Groww).expect_err("still refused, for another reason");
        assert!(
            !err.contains("this reader holds at most"),
            "the size arm fired at the bound itself: {err}"
        );
    }

    #[test]
    fn the_header_row_is_bounded_too_and_is_refused_before_it_is_split() {
        // THE ONE ROW THAT HAD NO BOUND. `MAX_ROW_BYTES` is checked inside the
        // loop over what is left AFTER `lines.next()` took the header, so every
        // data row was guarded and the header was not -- the D-0033 shape, one
        // line above the guard that exists to prevent it.
        //
        // A vendor endpoint serving an HTML error page instead of a CSV arrives
        // as exactly this: one enormous first line, split and hashed field by
        // field inside the function whose premise is a named refusal rather
        // than a dead process.
        let wide = "a,".repeat(MAX_ROW_BYTES);
        let body = format!("{wide}\nNSE,CASH,,RELIANCE,EQ,EQ,INE002A01018,,\n");
        let err = load(&tmp("wideheader", &body), Vendor::Groww)
            .expect_err("an unbounded header must be refused, not split");
        assert!(
            err.contains("header row is") && err.contains(&MAX_ROW_BYTES.to_string()),
            "the refusal names the bound: {err}"
        );

        // And a header of ordinary width is still read exactly as before, so
        // the bound refuses the absurd without touching the real file.
        let ok = load(
            &tmp(
                "okheader",
                // `groww_symbol` IS REQUIRED HERE and was not on the branch this
                // test arrived from. It is what a Groww request is addressed by
                // — see this module's own header — so a fixture without it
                // refuses on a missing column before the width bound this test
                // is about can be reached.
                "exchange,segment,underlying_symbol,trading_symbol,instrument_type,\
                 series,isin,expiry_date,strike_price,groww_symbol\n\
                 NSE,CASH,,RELIANCE,EQ,EQ,INE002A01018,,,NSE-X\n",
            ),
            Vendor::Groww,
        )
        .expect("an ordinary header still loads");
        assert_eq!(ok.kept.len(), 1);
    }

    #[test]
    fn a_row_longer_than_this_reader_splits_is_named_and_never_split() {
        // `split(',').collect()` allocates one pointer pair per comma, and
        // `decode_master_row`'s own width gate cannot run until that vector
        // exists. So the row length is bounded BEFORE the split, and the row is
        // reported at its line number like any other unreadable row -- never
        // dropped, never truncated and read anyway.
        let long = "X".repeat(MAX_ROW_BYTES + 1);
        let body = format!(
            "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,expiry_date,strike_price,groww_symbol\nNSE,CASH,,{long},EQ,EQ,INE002A01018,,,NSE-X\nNSE,CASH,,RELIANCE,EQ,EQ,INE002A01018,,,NSE-X\n"
        );
        let got = load(&tmp("longrow", &body), Vendor::Groww).expect("loads");
        assert_eq!(got.kept.len(), 1, "the intact row still decodes");
        assert_eq!(got.errors.len(), 1);
        assert_eq!(got.errors[0].0, 2, "the line is named");
        let reason = &got.errors[0].1;
        assert!(
            reason.contains("row is") && reason.contains(&MAX_ROW_BYTES.to_string()),
            "got {reason}"
        );
    }

    #[test]
    fn a_field_wider_than_core_will_read_is_an_error_and_not_a_silent_keep() {
        // The row that shipped before D-0033: a legitimate `underlying_symbol`
        // and an enormous `trading_symbol`, which is scanned by TEST_MARKERS
        // and then never becomes the identity. It used to be KEPT.
        let wide = "X".repeat(brutex_core::vendor::MAX_FIELD_BYTES + 1);
        let body = format!(
            "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,expiry_date,strike_price,groww_symbol\nNSE,CASH,RELIANCE,{wide},EQ,EQ,INE002A01018,,,NSE-X\n"
        );
        let got = load(&tmp("widefield", &body), Vendor::Groww).expect("loads");
        assert!(got.kept.is_empty(), "an over-wide row is never stored");
        assert_eq!(got.errors.len(), 1);
        // Bound rather than called inside the failure message: a call there is
        // a region that only runs when the assertion FAILS, so no passing test
        // can ever cover it.
        let reason = &got.errors[0].1;
        assert!(
            reason.contains("trading_symbol"),
            "the offending field is named: {reason}"
        );
    }

    #[test]
    fn skips_are_counted_by_reason_and_errors_keep_their_line_number() {
        let body = "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,expiry_date,strike_price,groww_symbol\nBSE,CASH,,SENSEX,IDX,,,,,NSE-X\nNSE,COMMODITY,GOLD,GOLD,FUT,,,2026-08-05,,NSE-X\nNSE,FNO,031NSETEST,X,FUT,,,2036-11-27,,NSE-X\nNSE,CASH,,SOMEBOND,EQ,N2,INE002A01018,,,NSE-X\nNSE,CASH,,SOMESME,EQ,SM,INE002A01018,,,NSE-X\nNSE,FNO,NIFTY,X,ZZ,,,2026-08-04,1,NSE-X\n";
        let p = tmp("skips", body);
        let got = load(&p, Vendor::Groww).expect("loads");
        assert_eq!(got.kept.len(), 0);
        assert_eq!(got.skipped.get("foreign exchange"), Some(&1));
        assert_eq!(got.skipped.get("segment not stored"), Some(&1));
        assert_eq!(got.skipped.get("exchange test instrument"), Some(&1));
        assert_eq!(got.skipped.get("not an equity listing"), Some(&1));
        assert_eq!(got.skipped.get("SME board"), Some(&1));
        assert_eq!(got.skipped_total(), 5);
        assert_eq!(
            got.skipped_by_reason(),
            vec![
                ("SME board", 1),
                ("exchange test instrument", 1),
                ("foreign exchange", 1),
                ("not an equity listing", 1),
                ("segment not stored", 1),
            ],
            "a report needs a stable order, not a hash order"
        );
        assert_eq!(got.errors.len(), 1, "the ZZ type is unreadable");
        assert_eq!(got.errors[0].0, 7, "line numbers count the header");
    }

    #[test]
    fn a_short_row_is_an_error_and_never_an_index_panic() {
        let body = "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,expiry_date,strike_price,groww_symbol\nNSE,CASH\n";
        let p = tmp("short", body);
        let got = load(&p, Vendor::Groww).expect("loads");
        // Too short to hold the columns -> an error naming the shortfall,
        // never an index panic and never a defaulted empty field.
        assert_eq!(got.errors.len(), 1);
        assert!(got.kept.is_empty() && got.skipped.is_empty());
    }

    #[test]
    fn dhan_columns_use_their_own_names() {
        let body = "EXCH_ID,SEGMENT,SECURITY_ID,ISIN,INSTRUMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,SERIES,SM_EXPIRY_DATE,STRIKE_PRICE,OPTION_TYPE\nNSE,I,13,NA,INDEX,NIFTY,NIFTY,NA,0001-01-01,,\nNSE,D,1,,OPTIDX,NIFTY,NIFTY,,2026-08-04,19450.00000,CE\n";
        let p = tmp("dhan", body);
        let got = load(&p, Vendor::Dhan).expect("loads");
        // The INDEX row is kept. The OPTION row is a LIVE contract and is
        // skipped by design -- backtests run on expired contracts from the
        // historical endpoints and the lake, never on the live chain.
        assert_eq!(got.kept.len(), 1, "only the index is stored");
        assert!(got.errors.is_empty());
        assert!(
            got.kept[0].key.is_sweepable(),
            "NIFTY from Dhan is sweepable"
        );
        assert_eq!(
            got.kept[0].isin, None,
            "`NA` in the ISIN column is not an ISIN"
        );
        assert_eq!(got.skipped_total(), 1, "the live option was declined");
    }

    #[test]
    fn blank_lines_are_ignored() {
        let body = "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,expiry_date,strike_price,groww_symbol\n\nNSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-X\n\n";
        let p = tmp("blank", body);
        let got = load(&p, Vendor::Groww).expect("loads");
        assert_eq!(got.kept.len(), 1);
        assert_eq!(got.errors.len(), 0);
    }
}

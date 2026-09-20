//! Turning one vendor's instrument row into the canonical [`InstrumentKey`].
//!
//! # Why this reads columns and never parses the display symbol
//!
//! Every vendor ships a human-facing trading symbol, and it is tempting to
//! parse it. Real rows from the primary broker's own master show why that is a
//! trap:
//!
//! | Trading symbol | Real `expiry_date` column |
//! |---|---|
//! | `NIFTY2680419450CE` | `2026-08-04` — `26` year, `8` month, `04` day, weekly |
//! | `BANKNIFTY25DEC27000PE` | `2025-12-24` — `25` year, `DEC` month, **no day at all** |
//!
//! Two different encodings in one file, and the monthly form does not carry the
//! day, so the expiry is **not recoverable from the symbol**. Worse,
//! `BANKNIFTY25SEP…` happens to expire on the 25th, so a day-first reading and
//! a year-first reading agree on that row and disagree on the other — the kind
//! of coincidence that hides a bug through a whole test suite.
//!
//! The master already carries `expiry_date`, `strike_price` and
//! `underlying_symbol` as separate structured columns. Reading them is exact,
//! it is O(1) per row, and it makes the symbology question disappear rather
//! than answering it. The display symbol is never an input to identity.
//!
//! `C-09` is where the per-row cost is measured rather than asserted:
//! `core::bench::decode_is_flat_in_field_width` decodes one row whose field is
//! 28 bytes and one whose field is 4 MiB, and `C-10` beside it holds that an
//! over-wide field is **refused** rather than merely decoded quickly.
//!
//! # Prices
//!
//! Strikes arrive in **rupees** and are stored in **paisa**. `27000` in the
//! master is `2_700_000` here. One missed multiplication makes every strike
//! wrong by a factor of a hundred, so the conversion goes through
//! [`Paisa::from_rupees_half_up`] like every other price.

use crate::error::InstrumentError;
use crate::instrument::{Exchange, Expiry, InstrumentKey, Kind, Segment};
use crate::isin::Isin;
use crate::price::Paisa;
use crate::symbol::{SYMBOL_CAPACITY, Symbol};
use crate::universe::MemberIndex;

/// Which vendor a row came from.
///
/// This is the first segment of the store path — `docs/05-decisions.md`
/// D-0019 — so each vendor owns a completely independent series and can be
/// added, re-pulled or deleted without touching any other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Vendor {
    /// Primary broker.
    Groww,
    /// Secondary broker.
    Dhan,
    /// Historical archives on disk. Never authenticates.
    ///
    /// A vendor here is a STORE PREFIX, not a credential holder. `TrueData` and
    /// `Gdfl` are listed because their bars need somewhere of their own to
    /// live: without a row, `Feed::store_vendor` returns `None`, and the
    /// archive reader filed every bar under Dhan's prefix instead — 194
    /// instrument-months of GDFL futures under `bars/dhan/`, which is exactly
    /// the per-vendor independence D-0019 exists to protect.
    ///
    /// `pull::config` requires a credential table only of feeds whose transport
    /// is HTTP, so adding these two costs no operator a `credentials.toml`
    /// edit.
    TrueData,
    /// Historical archives on disk. Never authenticates.
    Gdfl,
    /// Third broker, HTTP. Bars are filed under its own prefix like any other.
    ///
    /// # The one vendor here whose master carries NO ISIN
    ///
    /// `docs/00-charter.md` §4d records its instrument dump: twelve columns —
    /// `instrument_token`, `exchange_token`, `tradingsymbol`, `name`,
    /// `last_price`, `expiry`, `strike`, `tick_size`, `lot_size`,
    /// `instrument_type`, `segment`, `exchange` — and **not one of them is an
    /// ISIN**. D-0117 and D-0125 key their joins on `(exchange, ISIN)`, and
    /// neither addresses a row from this vendor.
    ///
    /// The vendor names the alternative itself: *"it is recommended to use a
    /// combination of exchange and tradingsymbol as the unique key, not the
    /// numeric instrument token."* So this feed joins on the symbol, which
    /// `pull::universe::JoinKey::TradingSymbol` marks as the **weaker** key
    /// wherever a count is reported — a rename looks like a missing instrument
    /// on it, and an ISIN would have absorbed one.
    Zerodha,
}

impl Vendor {
    /// Every vendor this engine reads, in path order.
    pub const ALL: [Self; 5] = [
        Self::Groww,
        Self::Dhan,
        Self::TrueData,
        Self::Gdfl,
        // APPENDED, NEVER INSERTED. `VendorSet` reads a bit per position and
        // `merge::Entry::ids` is an array indexed by `vendor as usize`, so a
        // variant placed in the middle hands every vendor after it another
        // one's ids — silently, because the types still line up.
        Self::Zerodha,
    ];

    /// Whether this vendor publishes an instrument master at all.
    ///
    /// Brokers do; archives do not — a folder of CSVs IS its own listing, and
    /// every file in it is an instrument named by its filename.
    ///
    /// This exists because "every vendor agrees on this instrument" was written
    /// as `Vendor::ALL.iter().all(...)`, which asks the wrong question the
    /// moment a vendor exists that cannot answer. With archives in `ALL`, every
    /// instrument became non-agreeing and the whole universe degraded — the
    /// same shape as `pull::config` demanding a credential from a feed that has
    /// none. A predicate, so the right set is named rather than assumed.
    #[must_use]
    pub const fn publishes_master(self) -> bool {
        match self {
            Self::Groww | Self::Dhan | Self::Zerodha => true,
            Self::TrueData | Self::Gdfl => false,
        }
    }

    /// Every vendor that publishes an instrument master.
    pub const MASTERED: [Self; 3] = [Self::Groww, Self::Dhan, Self::Zerodha];

    /// What this vendor's instrument master is called on disk.
    ///
    /// # Why the file name is a property of the vendor
    ///
    /// It was a hand-written list in `api::server::master_paths`:
    ///
    /// ```text
    /// vec![(Vendor::Groww, dir.join("groww_instruments.csv")),
    ///      (Vendor::Dhan,  dir.join("dhan_scrip.csv"))]
    /// ```
    ///
    /// So adding a feed meant editing that function — and forgetting to meant a
    /// vendor the engine knows about whose master is silently never read, which
    /// reports as "this vendor lists nothing" rather than as the wiring bug it
    /// is. A `match` on `Self` cannot be forgotten: a new variant is a compile
    /// error until it names its file.
    ///
    /// Everything downstream already iterates [`Self::ALL`] — the census grid,
    /// the merge, the ingest form, the coverage table. This was the one place
    /// that did not, and it was the entry point.
    #[must_use]
    pub const fn master_file(self) -> &'static str {
        match self {
            Self::Groww => "groww_instruments.csv",
            Self::Dhan => "dhan_scrip.csv",
            // AN ARCHIVE SHIPS NO MASTER. The folder of CSVs IS the listing:
            // every file in it is an instrument, named by its own filename.
            // The name below is what `master_paths` looks for and will not
            // find, which is correct — an archive contributes no rows to the
            // merged universe and must not be expected to.
            Self::TrueData => "truedata_instruments.csv",
            Self::Gdfl => "gdfl_instruments.csv",
            // A GZIPPED CSV ON THE WIRE, and a plain one once it is on disk.
            // The vendor regenerates it once a day and asks that it be stored
            // rather than re-fetched; this is the name it is stored under.
            Self::Zerodha => "zerodha_instruments.csv",
        }
    }

    /// The path segment for this vendor.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Groww => "groww",
            Self::Dhan => "dhan",
            Self::TrueData => "truedata",
            Self::Gdfl => "gdfl",
            Self::Zerodha => "zerodha",
        }
    }

    /// This vendor's bit in a [`VendorSet`].
    const fn bit(self) -> u8 {
        match self {
            Self::Groww => 1,
            Self::Dhan => 1 << 1,
            // `VendorSet` is a u8 — eight feeds, and these are three and four.
            Self::TrueData => 1 << 2,
            Self::Gdfl => 1 << 3,
            Self::Zerodha => 1 << 4,
        }
    }
}

/// A set of vendors, as a bitset.
///
/// A merged instrument is listed by one vendor or by several, and the set of
/// vendors that named it is a *property of the merge*, never of the identity —
/// exactly like [`crate::universe::Universe`], and for exactly the same reason:
/// a field on [`InstrumentKey`] would split one instrument into two keys.
///
/// It exists rather than a pair of booleans because a `bool` per vendor forces
/// every caller to `match` on [`Vendor`], which is `#[non_exhaustive]`. Outside
/// this crate that match needs a wildcard arm that no test can ever reach, so
/// the coverage gate can never go green on it. The bitset moves the one match
/// here, where it is exhaustive and provable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct VendorSet(u8);

impl VendorSet {
    /// No vendor has named this instrument.
    pub const EMPTY: Self = Self(0);

    /// This set with `vendor` added. Adding twice is the same as adding once.
    #[must_use]
    pub const fn with(self, vendor: Vendor) -> Self {
        Self(self.0 | vendor.bit())
    }

    /// Whether `vendor` is in this set.
    #[must_use]
    pub const fn contains(self, vendor: Vendor) -> bool {
        self.0 & vendor.bit() != 0
    }

    /// Whether no vendor is in this set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// The longest vendor instrument id this build will hold, in bytes.
///
/// **Derived from the widest grammar either vendor uses, then checked against
/// both live masters** rather than chosen:
///
/// | Part | Bytes | Measured on 2026-08-08 |
/// |---|---|---|
/// | exchange | 3 | `NSE`, `BSE` |
/// | underlying | 10 | longest present |
/// | expiry | 7 | `DDMmmYY`, e.g. `30Sep25` |
/// | strike | 6 | longest present, `100000` |
/// | side | 3 | `FUT`; `CE`/`PE` are shorter |
/// | separators | 4 | |
/// | **total** | **33** | longest id actually present: **32** |
///
/// 48 leaves room for a seven-digit strike and a longer underlying without
/// being a number nobody can justify. Dhan's `SECURITY_ID` is at most 7 bytes
/// and entirely numeric across all 204,819 rows, so it is far inside this.
///
/// [`crate::symbol::SYMBOL_CAPACITY`] is 24 and therefore **cannot** hold a
/// Groww symbol — `NSE-NIFTYNXT50-25Aug26-100000-PE` is 32. That is why this is
/// a separate type rather than a reuse.
pub const VENDOR_ID_CAPACITY: usize = 48;

const _: () = assert!(VENDOR_ID_CAPACITY >= 33, "the derived worst case");

/// A vendor's own identifier for an instrument, inline and never heap-allocated.
///
/// Dhan calls it `SECURITY_ID` and it is a number; Groww calls it
/// `groww_symbol` and it is `NSE-NIFTY-30Sep25-24650-CE`. Both are opaque here:
/// this type carries bytes the vendor chose and hands them back unchanged,
/// because the moment it parsed them it would own a grammar the vendor can
/// change without telling anyone.
///
/// # Why this exists at all
///
/// Neither column was read. `Vendor::master_columns` declared ten names and
/// neither `SECURITY_ID` nor `groww_symbol` was among them, so the decoder
/// stepped over the id — column 3 of Dhan's file, on the way to column 5 — and
/// dropped it. `groww_symbol` appeared nowhere in the workspace at all. Without
/// it nothing can name an instrument to either vendor, which is why Dhan
/// answers `DH-905 securityId is required`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct VendorId {
    bytes: [u8; VENDOR_ID_CAPACITY],
    len: u8,
}

impl VendorId {
    /// The id a vendor wrote down, or `None` if it is empty or too long.
    ///
    /// # Errors
    ///
    /// `None` for an empty id — a row with no id cannot be requested — and for
    /// one past [`VENDOR_ID_CAPACITY`], which is a grammar this build has not
    /// seen and must not silently truncate into a different instrument.
    #[must_use]
    pub fn new(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() || raw.len() > VENDOR_ID_CAPACITY {
            return None;
        }
        let mut bytes = [0_u8; VENDOR_ID_CAPACITY];
        // `get_mut` rather than an index: the guard above already bounds
        // `raw.len()`, but a slice expression carries a panic this workspace
        // denies, and a `?` that no input can take is cheaper than the lint
        // exception it would otherwise need.
        bytes.get_mut(..raw.len())?.copy_from_slice(raw.as_bytes());
        // The guard bounds len by VENDOR_ID_CAPACITY, pinned below to <= 255.
        // No second failure branch exists after that proof.
        #[allow(
            clippy::cast_possible_truncation,
            reason = "capacity checked above and pinned to u8 below"
        )]
        let len = raw.len() as u8;
        Some(Self { bytes, len })
    }

    /// The id, as the vendor wrote it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        // Every byte came from a `&str` in `new`, so the prefix is valid UTF-8;
        // and `len` is at most VENDOR_ID_CAPACITY, so the slice is in range.
        // Both are expressed as fallible lookups rather than asserted, because
        // an index expression carries a panic this workspace denies.
        self.bytes
            .get(..usize::from(self.len))
            .and_then(|held| core::str::from_utf8(held).ok())
            .unwrap_or("")
    }
}

const _: () = assert!(VENDOR_ID_CAPACITY <= 255, "len is a u8");

impl core::fmt::Debug for VendorId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VendorId({:?})", self.as_str())
    }
}

impl core::fmt::Display for VendorId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One row of a vendor instrument master, already split into fields.
///
/// Borrowed rather than owned: a master has hundreds of thousands of rows and
/// the vast majority are rejected, so allocating for each one would be work
/// done to throw away.
#[derive(Debug, Clone, Copy)]
pub struct MasterRow<'a> {
    /// The vendor's own id for this instrument — `SECURITY_ID` at Dhan,
    /// `groww_symbol` at Groww. Opaque; see [`VendorId`].
    pub vendor_id: &'a str,
    /// Exchange code, e.g. `NSE`.
    pub exchange: &'a str,
    /// Segment code, e.g. `CASH` or `FNO`.
    pub segment: &'a str,
    /// The underlying symbol. **Empty on every Groww CASH row**, including
    /// NIFTY and BANKNIFTY -- which is why [`MasterRow::trading_symbol`]
    /// exists and why identity is chosen per instrument type.
    pub underlying: &'a str,
    /// The vendor's own tradable symbol. For a cash or index row this is the
    /// only place the instrument is named at all.
    pub trading_symbol: &'a str,
    /// Instrument type: `IDX`, `EQ`, `FUT`, `CE`, `PE`.
    pub instrument_type: &'a str,
    /// The **NSE board series** this cash-segment row trades under: `EQ`,
    /// `BE`, `BZ`, `SM`, `N0`, `SG`, …
    ///
    /// This is one fact issued by one exchange, and both vendors carry it —
    /// Groww in `series`, Dhan in `SERIES`. It is named for what it *means*
    /// here rather than for either vendor's column heading, and it is read
    /// through [`Vendor::master_columns`]. Empty on index and derivative rows,
    /// which is why `board_of` is consulted only for cash equity.
    ///
    /// D-0025 replaced Dhan's `INSTRUMENT_TYPE` with `SERIES` here. That
    /// column is a vendor-minted paper class, and it is **measurably wrong**:
    /// it files 54 Franklin/PGIM/Bandhan mutual-fund plans as `ETF` and three
    /// real listings (`IVZINNIFTY`, `INFRABEES`, `NARMADA`) as `Other`/`MF`.
    /// The series column disagrees with it on exactly those 57 rows and is
    /// right on all 57.
    pub listing_class: &'a str,
    /// The vendor's ISIN column. Empty, or junk, on anything that is not a
    /// cash listing: measured, Groww writes the index ticker here on its
    /// index rows (`NIFTY`) and Dhan writes `NA`. Read only for equity.
    pub isin: &'a str,
    /// Expiry as `YYYY-MM-DD`, empty for cash and index rows.
    pub expiry: &'a str,
    /// Strike in **rupees**, empty for anything that is not an option.
    pub strike_rupees: &'a str,
    /// The vendor's separate option-side column (`CE`/`PE`), empty when the
    /// vendor encodes the side in `instrument_type` instead.
    pub option_side: &'a str,
}

impl MasterRow<'_> {
    /// The first field wider than [`MAX_FIELD_BYTES`], with its width.
    ///
    /// Every check is `str::len`, which is a field read on a fat pointer — the
    /// cost is the same ten reads whether a field holds two bytes or two
    /// gigabytes, which is the entire point. Returned rather than raised so the
    /// caller decides what an over-wide field means; [`decode_master_row`]
    /// treats it as a refusal.
    #[must_use]
    pub const fn over_wide(&self) -> Option<(&'static str, usize)> {
        // DESTRUCTURED WITHOUT `..`, so this is a COMPILE ERROR the day a
        // TWELFTH field is added to `MasterRow`. Reading the fields through
        // `self.` compiled perfectly well while skipping one, and a field that
        // skips this gate is a field with no width bound at all -- which is
        // precisely the defect D-0033 exists for.
        //
        // ELEVEN, NOT TEN: `vendor_id` is a field on this row here and was not
        // on the branch this destructuring arrived from. It gets a width bound
        // like every other, which is the whole point of writing them out.
        //
        // Written out rather than iterated because a `[(&str, &str); 11]` array
        // built per row is eleven pointer pairs written to the stack to answer a
        // question that is eleven comparisons. Order follows the struct.
        let Self {
            vendor_id,
            exchange,
            segment,
            underlying,
            trading_symbol,
            instrument_type,
            listing_class,
            isin,
            expiry,
            strike_rupees,
            option_side,
        } = *self;
        let checks: [(&'static str, usize); 11] = [
            ("vendor_id", vendor_id.len()),
            ("exchange", exchange.len()),
            ("segment", segment.len()),
            ("underlying", underlying.len()),
            ("trading_symbol", trading_symbol.len()),
            ("instrument_type", instrument_type.len()),
            ("listing_class", listing_class.len()),
            ("isin", isin.len()),
            ("expiry", expiry.len()),
            ("strike_rupees", strike_rupees.len()),
            ("option_side", option_side.len()),
        ];
        let mut i = 0;
        while i < checks.len() {
            // `const fn` cannot call `<[T]>::get`, and the index is bounded by
            // the array's own length one line above; const evaluation of the
            // bound is not possible here, but the loop condition is the bound.
            #[allow(clippy::indexing_slicing)]
            let (name, len) = checks[i];
            if len > MAX_FIELD_BYTES {
                return Some((name, len));
            }
            i += 1;
        }
        None
    }
}

/// The widest a single vendor master field may be before the row is refused.
///
/// # Why a bound exists at all — D-0033
///
/// [`crate::symbol::Symbol`] opens by arguing that an unbounded identifier must
/// never reach a hot path: *"A vendor that one day emits a 4 KiB identifier
/// would silently make every dedup probe 200× more expensive, and no test would
/// notice because nothing would be wrong, only slow."* The `TEST_MARKERS`
/// substring scan in [`decode_master_row`] reintroduced exactly that, one layer
/// **above** the 24-byte guard written to prevent it: it searched two raw
/// vendor `&str` before anything had bounded them.
///
/// Measured on this machine before the bound, with `underlying` pinned to
/// `RELIANCE` so only the scanned-but-unused `trading_symbol` grew: 8 B →
/// 56.4 ns, 1 KiB → 102.0 ns, 16 KiB → 997.8 ns, 4 MiB → 245,839.2 ns. Linear
/// over four orders of magnitude. And worse than the cost: the 4 MiB row was
/// **accepted and stored**, because `trading_symbol` only becomes the identity
/// when `underlying` is empty, so the width guard never saw it.
///
/// # Why 64
///
/// Measured across both real masters on 2026-08-01 — 33,990,514 B of
/// `dhan_scrip.csv` and 19,224,497 B of `groww_instruments.csv` — the widest
/// value in any column this decoder reads is **28 bytes**
/// (`MCX_MCXBULLDEX28AUG2632100CE` in Groww's `isin`,
/// `NIFTYNXT50-Aug2026-101500-CE` in Dhan's `SYMBOL_NAME`). 64 is 2.28× that,
/// so ordinary vendor drift does not trip it. The widest value in *any* column
/// of either file, including ones this decoder never reads, is 80 bytes — a
/// fund name — so a vendor moving a prose column into a column we read is
/// refused, loudly, which is the case worth catching.
///
/// This is not [`crate::symbol::SYMBOL_CAPACITY`] and must not be collapsed
/// into it. That bound is what an *identity* may be; this is what this engine
/// is willing to *look at*. A 40-byte expiry string is not a symbol and never
/// will be, but refusing to read it would refuse rows that decode correctly
/// today.
pub const MAX_FIELD_BYTES: usize = 64;

/// Which of a vendor's master columns fill a [`MasterRow`], by header name.
///
/// This lives beside the rest of the per-vendor knowledge rather than in the
/// reader, for two reasons. A reader in another crate cannot match on
/// [`Vendor`] without a wildcard arm — the enum is `#[non_exhaustive]` — and
/// that arm is unreachable, untestable, and permanently uncovered. And a
/// second reader would otherwise have to guess the same names again; the
/// column map is vendor knowledge, and vendor knowledge belongs here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MasterColumns {
    /// Which column carries the vendor's own instrument id.
    pub vendor_id: &'static str,
    /// Header of the exchange column.
    pub exchange: &'static str,
    /// Header of the segment column.
    pub segment: &'static str,
    /// Header of the underlying-symbol column.
    pub underlying: &'static str,
    /// Header of the vendor's own tradable-symbol column.
    pub trading_symbol: &'static str,
    /// Header of the instrument-type column.
    pub instrument_type: &'static str,
    /// Header of the column carrying the NSE board series of a cash row.
    pub listing_class: &'static str,
    /// Header of the ISIN column.
    pub isin: &'static str,
    /// Header of the expiry column.
    pub expiry: &'static str,
    /// Header of the strike column.
    pub strike: &'static str,
    /// Header of the separate option-side column, when the vendor has one.
    pub option_side: Option<&'static str>,
}

impl Vendor {
    /// The header names this vendor's master uses for each field of a
    /// [`MasterRow`].
    #[must_use]
    pub const fn master_columns(self) -> MasterColumns {
        match self {
            Self::Groww => MasterColumns {
                // The symbol Groww's own historical endpoints take:
                // `NSE-NIFTY-30Sep25-24650-CE`. NOT `trading_symbol`, which is
                // the SAME instrument spelled `NIFTY25SEP24650CE` on the same
                // row — two encodings per row, and only this one is accepted
                // by /v1/historical/candles.
                vendor_id: "groww_symbol",
                exchange: "exchange",
                segment: "segment",
                underlying: "underlying_symbol",
                trading_symbol: "trading_symbol",
                instrument_type: "instrument_type",
                listing_class: "series",
                isin: "isin",
                expiry: "expiry_date",
                strike: "strike_price",
                option_side: None,
            },
            Self::Dhan => MasterColumns {
                // What `securityId` on every Dhan request body must be. Column
                // 3 of the file, which the decoder used to step over on its way
                // to INSTRUMENT at column 5.
                vendor_id: "SECURITY_ID",
                exchange: "EXCH_ID",
                segment: "SEGMENT",
                underlying: "UNDERLYING_SYMBOL",
                // THE TICKER, WHICH IS NOT `SYMBOL_NAME`.
                //
                // `SYMBOL_NAME` holds the company's NAME, truncated to 24
                // characters: `RELIANCE INDUSTRIES LTD`, `TATA CONSULTANCY
                // SERV LT`, `HDFC BANK LTD`. `UNDERLYING_SYMBOL` holds
                // `RELIANCE`, `TCS`, `HDFCBANK` — the NSE ticker, the same
                // string Groww puts in `trading_symbol`.
                //
                // Measured over the 2,781 main-board NSE cash equities this
                // vendor's own file yields:
                //
                //   UNDERLYING_SYMBOL  0 blank, 2,781 distinct of 2,781,
                //                      and 2,733 of Groww's 2,740 tickers
                //                      matched exactly.
                //   SYMBOL_NAME        2 collisions (FUTURE ENTERPRISES LTD
                //                      and GACM TECHNOLOGIES LIMITED, each two
                //                      securities with distinct ISINs), and
                //                      3 of 2,740 matched.
                //
                // So reading the wrong column MANUFACTURED the only duplicate
                // symbols in the kept set, and made this vendor's rows fail to
                // line up with the other's on every join that is not the ISIN.
                //
                // It also points at the same column as `underlying` above, and
                // that is correct rather than a copy-paste: for a cash equity
                // the underlying IS the instrument. Nothing requires these two
                // to be different columns — `Columns::widest` folds a maximum
                // over the indices and does not care that two of them agree.
                trading_symbol: "UNDERLYING_SYMBOL",
                // The real type is INSTRUMENT. `INSTRUMENT_TYPE` is a
                // different column holding a vendor-minted paper class (ES,
                // DEB, ETF); it is deliberately NOT read — see the
                // `listing_class` doc and D-0025.
                instrument_type: "INSTRUMENT",
                listing_class: "SERIES",
                isin: "ISIN",
                expiry: "SM_EXPIRY_DATE",
                strike: "STRIKE_PRICE",
                option_side: Some("OPTION_TYPE"),
            },
            // AN ARCHIVE HAS NO MASTER TO DESCRIBE. Every column name below is
            // the empty string, which no header can match, so a master file
            // that somehow appeared under one of these names would decline
            // every row rather than reading them under invented headings.
            //
            // Refusing here rather than returning an `Option` because this is a
            // `const fn` on a hot path and the caller — `master_paths` — already
            // handles a master that is simply absent, which is the real state.
            // TWELVE COLUMNS, AND THE ISIN IS THE ONE THAT IS NOT THERE.
            //
            // Read from the vendor's own page, 14 Aug 2026 — docs/00-charter.md
            // §4d. `isin` is the empty string, which no header matches, so
            // every row of this master decodes with no ISIN and the join falls
            // to the symbol. That is a fact about the vendor stated as data,
            // not a column left blank by oversight.
            //
            // `underlying` is empty too, and for a different reason: this master
            // has no underlying column at all. `name` is the COMPANY name for an
            // equity and blank on a derivative row, so reading it as an
            // underlying would put "INFOSYS" where "INFY" belongs.
            Self::Zerodha => MasterColumns {
                vendor_id: "instrument_token",
                exchange: "exchange",
                segment: "segment",
                underlying: "",
                trading_symbol: "tradingsymbol",
                instrument_type: "instrument_type",
                listing_class: "",
                isin: "",
                expiry: "expiry",
                strike: "strike",
                // NO SEPARATE SIDE COLUMN. `instrument_type` is `CE` or `PE`
                // directly, so the side is read from the type rather than from
                // a column this master does not have — the same shape Groww
                // uses, and the reason this field is an `Option`.
                option_side: None,
            },
            Self::TrueData | Self::Gdfl => MasterColumns {
                vendor_id: "",
                exchange: "",
                segment: "",
                underlying: "",
                trading_symbol: "",
                instrument_type: "",
                listing_class: "",
                isin: "",
                expiry: "",
                strike: "",
                option_side: None,
            },
        }
    }
}

/// A row was skipped, and why.
///
/// Skipping is not failing. A master holds every instrument the vendor knows
/// about, and most of them are legitimately not ours. The reason is carried so
/// an ingest can report *what* it declined rather than a bare count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Skip {
    /// The vendor left its own id blank, or wrote one longer than
    /// [`VENDOR_ID_CAPACITY`]. Either way the instrument cannot be named back
    /// to them, so it is declined rather than carried: a listing nothing can
    /// request is one every later lookup finds and no pull can use.
    NoVendorId,
    /// Not an exchange this engine stores. `docs/05-decisions.md` D-0017.
    ///
    /// Raised only for an exchange code this engine **recognises and does not
    /// store** -- today that is `BSE`. A code it cannot parse at all gets
    /// [`Skip::UnrecognisedExchange`], for the same reason a bond and an
    /// unknown series code are different reasons.
    ForeignExchange,
    /// An exchange code this engine has never seen.
    ///
    /// # Why this is not `ForeignExchange`
    ///
    /// It was, and that made a mapping bug indistinguishable from a routine
    /// refusal -- the exact defect [`Vendor::segment_of`] raises a loud error
    /// for, one gate earlier. `Exchange::parse` returns `Err` for a code it
    /// does not know, and the gate discarded that `Err` with a `matches!`, so
    /// `NSE` drifting to `NSE_EQ` (a rename, a padded column, a shifted field)
    /// declined **every row of both masters** as "foreign exchange" and the
    /// process reported `ok` and exited zero.
    ///
    /// "A venue we do not store" and "a code we cannot read" are different
    /// facts. This one is not routine: it degrades the run, so it reaches the
    /// exit code rather than printing a routine skip.
    UnrecognisedExchange,
    /// An exchange test instrument, not a real listing.
    TestInstrument,
    /// A segment this engine does not store, such as commodity.
    ForeignSegment,
    /// A **currently listed** future or option.
    ///
    /// A live contract's bars are today's. Backtesting runs on history, and
    /// history is EXPIRED contracts — which come from the vendors' historical
    /// endpoints and from the existing lake, never from the live instrument
    /// master. Storing the live chain would add ~148,000 contracts holding a
    /// few weeks of data each, none of which is ever swept.
    LiveContract,
    /// A row that trades on the equity segment but is **not a share**.
    ///
    /// A debenture, a government or corporate bond, a treasury bill, a mutual
    /// fund unit, a REIT or an infrastructure trust unit, a warrant. Every one
    /// of them is a real listing; none of them is an equity, and none belongs
    /// in an equity universe.
    ///
    /// Raised only for a series code in [`NON_EQUITY_SERIES`] — a code this
    /// engine has **measured and recognises**. A code it does not recognise
    /// gets [`Skip::UnrecognisedListingClass`] instead, because "I know this
    /// is a bond" and "I have never seen this code" are different facts and
    /// filing them under one reason is how a rename becomes invisible.
    ///
    /// # Why this is a gate and not a nicety
    ///
    /// `INSTRUMENT = EQUITY` does not mean "a share". It means "trades on the
    /// equity segment". Measured across both masters: of the 12,617 rows that
    /// reach this gate, **7,137 are not equity at all** — 4,336 `SG` state
    /// development loans, 1,106 `N0` debentures, 174 `MF` fund units, 164 `GS`
    /// government securities, 85 `TB` treasury bills, and 116 further debt
    /// codes.
    ///
    /// Without the gate those rows do not merely inflate a count — they
    /// **capture the ticker**. Dhan line 167146 is
    /// `NSE,E,19257,INE121A08PJ0,EQUITY,,CHOLAFIN,…,DEB,D1,…,5.0` — a 7.5% NCD
    /// on series `D1` — and it appears **before** line 171414,
    /// `NSE,E,685,INE121A01024,EQUITY,,CHOLAFIN,…,ES,EQ,…,10.0`, the share on
    /// series `EQ`. An insert-if-absent merge therefore resolved `CHOLAFIN` to
    /// the bond and took its tick size, silently and order-dependently.
    /// `MOTHERSON` (`INE775A08105`, an NCD, series `D1`) and `ELECTCAST`
    /// (`INE086A13016`, a warrant, series `W1`) went the same way. All three
    /// are NIFTY Total Market members and two are F&O underlyings. After the
    /// gate, duplicate tickers in Dhan's equity segment fall from **4 to 0**.
    NotEquityListing,
    /// A listing on the SME board rather than the main board.
    ///
    /// A separate reason from [`Skip::NotEquityListing`] on purpose: an SME
    /// listing IS a share, so lumping it in with debentures would hide a real
    /// choice behind a wrong label. It is declined because the engine's equity
    /// universe is F&O underlyings plus NIFTY Total Market
    /// ([`crate::universe`]), and neither contains an SME listing — measured,
    /// and proven by
    /// `crate::universe::no_measured_sme_ticker_belongs_to_either_universe`.
    /// An SME row can only ever be stored, never swept and never ranked.
    ///
    /// Counted separately so the decision stays visible and reversible: the
    /// day the universe widens, this reason names exactly what to re-admit.
    /// Both vendors carry the board in the NSE series (`SM`, `ST`), so unlike
    /// before D-0025 this reason is raised **symmetrically**: 558 rows at
    /// Groww and 559 at Dhan, and the ISINs are the same paper.
    SmeBoard,
    /// A cash-equity row whose NSE series code this engine has never seen.
    ///
    /// # Why this is not filed with the bonds
    ///
    /// This is the variant that exists because of what happened when it did
    /// not. Before it, both arms of the gate ended in `_ => NotEquity`, so an
    /// unrecognised code was reported with the identical wording a routine
    /// debenture gets. Demonstrated on the real Dhan master by rewriting the
    /// equity series `EQ` to `EQX`: **2,438 shares vanished**, every F&O
    /// underlying among them, the report still printed `ok`, and the only
    /// trace was one counter moving. That is precisely the failure
    /// `Vendor::segment_of` documents as unrepeatable.
    ///
    /// It is a decline rather than an error because the alphabet is genuinely
    /// open-ended — NSE mints a debt series whenever it needs one and 120
    /// already exist, so a new bond series must not fail an ingest. But it is
    /// its **own** decline: its own reason string, its own counter, and the
    /// offending code itself is carried to the operator by the reader (see
    /// `api::master::Loaded::unrecognised`). A universe holding one is
    /// *disputed*, so the process reporting it exits non-zero rather than
    /// printing a routine skip and `ok`.
    UnrecognisedListingClass,
}

impl Skip {
    /// A short human-readable reason, stable enough to be a report key.
    ///
    /// It lives here rather than in a reporter because [`Skip`] is
    /// `#[non_exhaustive]`: outside this crate every `match` on it needs a
    /// wildcard arm, which silently swallows a variant added later under
    /// whatever label the wildcard chose. Inside the crate the match is
    /// exhaustive, so a new variant is a compile error until it is named.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NoVendorId => "no vendor id on the row",
            Self::ForeignExchange => "foreign exchange",
            Self::UnrecognisedExchange => "unrecognised exchange",
            Self::TestInstrument => "exchange test instrument",
            Self::ForeignSegment => "segment not stored",
            Self::LiveContract => "live derivative contract",
            Self::NotEquityListing => "not an equity listing",
            Self::SmeBoard => "SME board",
            Self::UnrecognisedListingClass => "unrecognised listing class",
        }
    }

    /// Whether this decline is a routine business outcome.
    ///
    /// Every reason here is a *deliberate* decline, but they are not equally
    /// expected. A bond, a foreign exchange and a live contract are what a
    /// master is full of. An unrecognised series code is the vendor, or the
    /// exchange, having changed something under us — and the difference has to
    /// reach an exit code, or the two are the same fact to a monitor.
    #[must_use]
    pub const fn is_routine(self) -> bool {
        !matches!(
            self,
            Self::UnrecognisedListingClass | Self::UnrecognisedExchange
        )
    }

    /// Whether this decline judges the **paper** rather than the venue.
    ///
    /// Only a judgement about the paper can contradict another vendor keeping
    /// the same ISIN. A decline for the exchange, the segment or the contract
    /// being live says where the row was found, and the same security
    /// legitimately appears at another venue — `RELIANCE` is `INE002A01018` on
    /// both NSE and BSE, and one vendor declining the BSE row while the other
    /// keeps the NSE row is two correct decisions about two different rows.
    /// Treating that as a disagreement produced 3,000 false conflicts on the
    /// real masters, which is a check nobody would read twice.
    #[must_use]
    pub const fn judges_the_paper(self) -> bool {
        matches!(
            self,
            Self::NotEquityListing | Self::SmeBoard | Self::UnrecognisedListingClass
        )
    }
}

/// A row this engine keeps: the canonical key, and the vendor's ISIN **beside
/// it** rather than in it.
///
/// See [`crate::isin`] for why the ISIN is not a field of [`InstrumentKey`].
/// Every field is fixed-width and [`Copy`], so a listing is as cheap to hash
/// and move as the key alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Listing {
    /// The vendor's own id for this instrument, carried through so a request
    /// can name it. Without this nothing reaches either broker — see
    /// [`VendorId`].
    pub vendor_id: VendorId,
    /// The canonical identity, from the columns the vendor actually fills.
    pub key: InstrumentKey,
    /// The vendor's ISIN for this row, when it has one. `None` for an index,
    /// which has no ISIN at all — no sentinel is invented for NIFTY.
    pub isin: Option<Isin>,
    /// The same identity with the vendor's own series suffix removed, when
    /// the symbol carries one — `BLUECHIP-BE` under series `BE` gives
    /// `BLUECHIP`.
    ///
    /// A **candidate**, never a substitution. Groww leaks
    /// `internal_trading_symbol` into `trading_symbol` on exactly 209 of the
    /// 4,080 ISINs the two masters share, and the leak reaches tradeable
    /// equity and ETFs (`BLUECHIP-BE`, `CBAZAAR-ST`, `HDFCLIQUID-EQ`,
    /// `LOWVOL-EQ`), so "only debt is suffixed" is false. But `BAJAJ-AUTO` is
    /// a real ticker that ends in a dash, and stripping blind would
    /// manufacture the very collision [`crate::symbol`] refuses to
    /// manufacture. So this is computed only when the trailing segment IS the
    /// row's own series, and the caller adopts it only when a second vendor
    /// confirms the identity by ISIN.
    pub unsuffixed: Option<InstrumentKey>,
}

/// A row this engine declined, and the evidence of what it declined.
///
/// # Why the ISIN travels with a decline
///
/// A merge that only compares the rows both vendors KEPT cannot see the
/// disagreement that actually matters: one vendor calling a security an equity
/// while the other calls it a bond. Before this struct existed, a declined row
/// was dropped at the reader and the merge reported `0 conflicts` while the
/// two masters disagreed about the eligibility of 62 instruments — every one
/// of which carried the **same ISIN in both files**, so the check had the key
/// it needed and never looked. See `api::merge::Merged::eligibility`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Declined {
    /// Why the row was declined.
    pub reason: Skip,
    /// The row's ISIN, when the vendor gave one that parses.
    ///
    /// `None` is not a failure being hidden: the row is declined and counted
    /// under [`Declined::reason`] either way, and this field is *evidence for
    /// a cross-check*, never identity. It is deliberately parsed leniently
    /// here — unlike on a kept equity, where a bad ISIN is an error — because
    /// the one real row in either master whose check digit fails
    /// (`IN1520250085`, a state development loan) is a declined row, and
    /// erroring on it would turn a correct decline into a false failure.
    pub isin: Option<Isin>,
}

/// The outcome of reading one master row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decoded {
    /// A real instrument this engine stores.
    Keep(Listing),
    /// A row deliberately declined, with the reason and the evidence.
    Skipped(Declined),
}

impl Decoded {
    /// The reason this row was declined, or `None` if it was kept.
    ///
    /// Exists so a caller that cares only about the reason does not have to
    /// spell out the evidence beside it.
    #[must_use]
    pub const fn skip(self) -> Option<Skip> {
        match self {
            Self::Keep(_) => None,
            Self::Skipped(d) => Some(d.reason),
        }
    }
}

/// Exchange test listings carry these markers in the underlying symbol.
///
/// Real rows observed in the primary broker's master include
/// `031NSETEST36DECFUT` and `061NSETEST36DECFUT`, whose underlyings are
/// `031NSETEST` and `061NSETEST`. Storing them would put fabricated
/// instruments beside real ones, and they would be indistinguishable later.
const TEST_MARKERS: [&str; 2] = ["NSETEST", "BSETEST"];

/// `name` with every ASCII space removed, on the stack.
///
/// Returns the bytes in a fixed buffer rather than a `String`: this runs on
/// every row of every master -- over 340,000 of them across the two files --
/// and `CLAUDE.md` rule 4 makes a per-row heap allocation a cost that grows
/// with the input. The buffer is [`SYMBOL_CAPACITY`], so an identifier that
/// still does not fit after collapsing is refused HERE rather than being
/// truncated into a different instrument's name.
///
/// # Errors
///
/// [`InstrumentError::Malformed`] when more than [`SYMBOL_CAPACITY`] non-space
/// bytes arrive. Every other rule about what a symbol may contain stays with
/// [`Symbol::new`]; this only removes spaces.
fn collapse_spaces(name: &str) -> Result<Collapsed, InstrumentError> {
    let mut bytes = [0u8; SYMBOL_CAPACITY];
    let mut len = 0usize;
    for &c in name.as_bytes() {
        if c == b' ' {
            continue;
        }
        // `get_mut` rather than an index: `indexing_slicing` is denied
        // workspace-wide, and this is the bound that stops a long name being
        // silently cut down to another instrument's symbol.
        let Some(slot) = bytes.get_mut(len) else {
            return Err(InstrumentError::Malformed);
        };
        *slot = c;
        len += 1;
    }
    Ok(Collapsed { bytes, len })
}

/// The stack buffer [`collapse_spaces`] fills.
struct Collapsed {
    bytes: [u8; SYMBOL_CAPACITY],
    len: usize,
}

impl Collapsed {
    /// The collapsed bytes as text, or `""` if they are not UTF-8.
    ///
    /// Non-UTF-8 cannot survive [`Symbol::new`] either -- its allowlist is
    /// ASCII -- so the empty string routes a mangled input to the same
    /// `Malformed` the byte itself would have caused, one step later.
    fn as_str(&self) -> &str {
        self.bytes
            .get(..self.len)
            .map_or("", |b| core::str::from_utf8(b).unwrap_or(""))
    }
}

/// What a vendor's segment code means to us.
///
/// It carries no `Segment`: the vendor's column is a GATE only. Our segment is
/// derived from the instrument type, because the primary broker files spot
/// indices under `CASH` and adopting its value would put NIFTY in the
/// equities directory.
#[derive(PartialEq, Eq)]
enum SegmentVerdict {
    /// Rows in this segment are stored.
    Store,
    /// A segment this engine legitimately does not store.
    Decline,
}

/// What an NSE board series means to us.
///
/// Consulted **only** for a row that already decoded as cash equity. An index
/// row has no series at all — Groww leaves `series` empty on all 24 of its NSE
/// index rows, Dhan writes `NA` — so gating on it before the instrument type
/// is known would decline `NIFTY` and `BANKNIFTY`, which is every instrument
/// the engine exists to sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EquityVerdict {
    /// A genuine main-board share or ETF.
    MainBoard,
    /// A share, but on the SME board.
    Sme,
    /// Not a share at all: debt, a fund unit, a warrant.
    NotEquity,
    /// A code this engine has never seen. Never merely "not equity".
    Unrecognised,
}

/// The NSE series codes that ARE the equity board.
///
/// Measured across both real masters, 2026-08-01, over every cash-equity row:
///
/// | Series | Rows | What it is |
/// |---|---:|---|
/// | `EQ` | 4,845 | the rolling-settlement equity board |
/// | `BE` | 580 | trade-for-trade equity |
/// | `BZ` | 63 | trade-for-trade under surveillance |
/// | `E1` | 5 | partly-paid equity |
/// | `IT` | 4 | trade-for-trade, illiquid |
/// | `SZ` | 3 | trade-for-trade, surveillance (second list) |
///
/// `BZ`, `IT`, `SZ` and `E1` were declined before D-0025, and the decline was
/// counted under "not an equity listing", which was false about 30 real
/// shares — `HDIL`, `HMT`, `RAJESHEXPO`, `IL&FSENGG`, `ANSALAPI`, `FEL`,
/// `ARSHIYA`, `CEREBRAINT` among them. Three independent confirmations that
/// they are equity: the ISIN's NSDL security-type digits are `01` (ordinary
/// equity) on 27 of the 30 and `IN9…` (partly paid) on the other 3, against
/// `08` for the `CHOLAFIN` NCD and `13` for the `ELECTCAST` warrant this gate
/// still declines; every one of them is `ES` in Dhan's paper-class column; and
/// they are ordinary listed companies.
///
/// Sorted, and no longer for a search. `board_of` probes
/// `EQUITY_BOARD_INDEX`, so order carries no correctness weight at all now;
/// it is kept because a sorted list is the one a human can append to without
/// re-reading it, and because sortedness is how
/// `the_measured_series_tables_are_sorted_disjoint_and_complete` catches a
/// duplicate.
pub const EQUITY_BOARD_SERIES: [&str; 6] = ["BE", "BZ", "E1", "EQ", "IT", "SZ"];

/// The NSE series codes that are the SME board.
///
/// Measured: `SM` 815 rows and `ST` 302 across both masters. A share, on a
/// board this engine's universes do not reach. See [`Skip::SmeBoard`].
pub const SME_BOARD_SERIES: [&str; 2] = ["SM", "ST"];

/// Every NSE series this engine has measured that is **not** an equity.
///
/// 120 codes, transcribed from the union of both real masters on 2026-08-01 —
/// every distinct `series` on a Groww `NSE`/`CASH`/`EQ` row and every distinct
/// `SERIES` on a Dhan `NSE`/`E`/`EQUITY` row, minus the eight board codes
/// above. Debentures (`N0`…`NZ`, `Y*`, `Z*`, `AK`…`BX`, `D1`, `W1`), state
/// development loans (`SG`), government securities (`GS`, `GB`), treasury
/// bills (`TB`), fund units (`MF`, `SF`), REITs (`RR`), infrastructure trusts
/// (`IV`),
/// pass-through certificates and preference shares (`P1`).
///
/// # Why an explicit list rather than "everything else"
///
/// "Everything else" is what made a renamed equity code indistinguishable from
/// a bond. Enumerating what is known turns the unknown into its own visible
/// outcome — [`Skip::UnrecognisedListingClass`]. The list is data, so a new
/// NSE debt series is a one-line append and nothing else moves.
///
/// Sorted for the same reason [`EQUITY_BOARD_SERIES`] is: a human appends to
/// it, and `board_of` probes `NON_EQUITY_INDEX` rather than searching here.
pub const NON_EQUITY_SERIES: [&str; 120] = [
    "AK", "AL", "AM", "AN", "AZ", "BA", "BC", "BR", "BS", "BU", "BV", "BW", "BX", "D1", "GB", "GS",
    "IV", "MF", "N0", "N1", "N2", "N3", "N4", "N5", "N6", "N7", "N8", "N9", "NA", "NB", "NC", "ND",
    "NE", "NF", "NG", "NH", "NI", "NJ", "NK", "NL", "NM", "NN", "NO", "NP", "NQ", "NR", "NS", "NT",
    "NU", "NV", "NW", "NX", "NY", "NZ", "P1", "RR", "SF", "SG", "TB", "W1", "Y0", "Y1", "Y2", "Y3",
    "Y4", "Y5", "Y6", "Y7", "Y8", "Y9", "YA", "YB", "YC", "YD", "YG", "YH", "YI", "YJ", "YK", "YL",
    "YM", "YP", "YQ", "YR", "YS", "YT", "YU", "YV", "YW", "YX", "YY", "YZ", "Z0", "Z1", "Z2", "Z3",
    "Z4", "Z5", "Z6", "Z7", "Z8", "Z9", "ZC", "ZF", "ZG", "ZH", "ZI", "ZJ", "ZK", "ZL", "ZM", "ZN",
    "ZO", "ZP", "ZQ", "ZR", "ZS", "ZT", "ZY", "ZZ",
];

// THE THREE SERIES TABLES, INDEXED. `docs/07-o1-architecture.md` layer 4 is
// "no search of any kind ... never `binary_search`", and it carries no
// exemption for a small table. `board_of` searched all three until D-0065.
//
// The replacement is not new machinery. [`MemberIndex`] already exists in
// `crate::universe` for exactly this, already builds at compile time with no
// dependency and no lazy initialisation, and already carries the probe-length
// test layer 4's "How a layer is proven" section demands. These three tables
// are 6, 2 and 120 entries — an order of magnitude SMALLER than the 750 that
// justified building it. Writing the first entry into a rule-1 allowlist whose
// comment reads "no allowlist, layer 4 is unconditional", in order to keep
// three calls the crate beside it already knows how to remove, is the move
// that turns an allowlist into a place failures go to be filed.
//
// What it cost: three static tables, 16 + 8 + 512 slots of
// `Option<&'static str>`. Constant, and it does not grow with the lists.
//
// Every table is at most half full, which is what bounds the probe;
// `MemberIndex::build` asserts that at COMPILE time, so an over-full table
// cannot ship. The measured worst probe is asserted as a number by
// `the_series_tables_probe_in_bounded_time`, for the reason layer 4 records:
// the first open-addressed table anyone wrote here measured 14 probes and was
// refused by its own test until it was widened.

/// [`EQUITY_BOARD_SERIES`], indexed. 6 members in 16 slots.
static EQUITY_BOARD_INDEX: MemberIndex<16> = MemberIndex::build(&EQUITY_BOARD_SERIES);

/// [`SME_BOARD_SERIES`], indexed. 2 members in 8 slots.
static SME_BOARD_INDEX: MemberIndex<8> = MemberIndex::build(&SME_BOARD_SERIES);

/// [`NON_EQUITY_SERIES`], indexed. 120 members in 512 slots.
///
/// **512 and not 256, because the test said so.** At 256 the table is under
/// half full and `build` accepts it, and the worst probe measured **10** —
/// over the `<= 8` every `MemberIndex` in this crate is held to. These are
/// two-byte codes over a narrow alphabet, so FNV-1a clusters them harder than
/// it clusters ticker symbols. One doubling takes the worst probe from 10 to
/// **6**. That is layer 4's "How a layer is proven" happening again, with the
/// same outcome it records the first time: the test refused the table until it
/// was widened, and the number in the assertion is why anybody found out.
static NON_EQUITY_INDEX: MemberIndex<512> = MemberIndex::build(&NON_EQUITY_SERIES);

/// What an NSE board series means, for either vendor.
///
/// # Why this is not per-vendor any more
///
/// It was, and the doc argued that it had to be: "the two alphabets are
/// disjoint, and one flat table invites one vendor's code to be silently
/// accepted for the other". That argument was true of the columns the gate
/// used to read — Groww's NSE `series` against Dhan's own `INSTRUMENT_TYPE`
/// paper class — and it stopped being true when D-0025 pointed both vendors at
/// the series. A series code is minted by the exchange, not by a broker, so it
/// is **one alphabet**, and duplicating it per vendor would be two copies of
/// one fact free to drift apart.
///
/// It is not an assumption. Measured on the 4,080 ISINs the two masters share:
/// the two vendors' series columns disagree on 22 rows — all of them
/// snapshot skew inside one board, `EQ`↔`BE` or `SM`↔`ST`, e.g. `SICALLOG` —
/// and the verdict this function returns differs on **0**.
///
/// An unrecognised code is a decline, not an error, for the reason
/// [`Skip::UnrecognisedListingClass`] gives — but it is its own decline, and
/// never confused with a bond.
fn board_of(series: &str) -> EquityVerdict {
    // Dhan pads this column, e.g. `"   ES   "`. Trimming Groww's already-tight
    // values costs nothing and cannot change a verdict.
    let series = series.trim();
    // Hash, mask, probe — three times at most, each of them constant. The
    // tables are disjoint (asserted), so the order these are asked in cannot
    // change a verdict; it is the order of decreasing frequency, which is a
    // property of the data rather than of correctness.
    if EQUITY_BOARD_INDEX.contains(series) {
        EquityVerdict::MainBoard
    } else if SME_BOARD_INDEX.contains(series) {
        EquityVerdict::Sme
    } else if NON_EQUITY_INDEX.contains(series) {
        EquityVerdict::NotEquity
    } else {
        EquityVerdict::Unrecognised
    }
}

impl Vendor {
    /// Maps this vendor's segment code.
    ///
    /// # Errors
    ///
    /// [`InstrumentError::Malformed`] for a code this vendor is not known to
    /// emit. That is deliberately an ERROR and not a decline: the secondary
    /// broker writes its segments as single letters, and treating an
    /// unrecognised code as "not ours" made the decoder discard **all 200,460
    /// of its rows while reporting a routine skip**. A mapping bug must never
    /// be indistinguishable from a legitimate refusal.
    fn segment_of(self, code: &str) -> Result<SegmentVerdict, InstrumentError> {
        // Nested per vendor rather than matched on the pair: the two vendors
        // use disjoint alphabets, and a flat match invites one vendor's code
        // to be silently accepted for the other.
        let (store, decline): (&[&str], &[&str]) = match self {
            Self::Groww => (&["INDEX", "CASH", "FNO"], &["COMMODITY"]),
            // I index, E equity cash, D equity+index derivatives,
            // C currency, M commodity.
            Self::Dhan => (&["I", "E", "D"], &["C", "M"]),
            // AN ARCHIVE DECLINES EVERY SEGMENT, because it publishes no master
            // for one to appear in. Empty store list AND empty decline list:
            // nothing is stored, and nothing is quietly dropped either — code
            // reaching here is reading a master that should not exist.
            // `segment` on this vendor's rows is `NSE`, `NFO-FUT`, `NFO-OPT`,
            // `MCX` — a compound of exchange and product, not the one-letter
            // code the other two brokers use. The two spot words are stored and
            // the derivative ones are declined by name; anything else is
            // unrecognised, which is refused rather than guessed at.
            //
            // `INDICES` IS READ NOW, AND IT WAS THE WHOLE FAILURE.
            //
            // This list carried `NSE` alone, under a comment saying `INDICES`
            // was UNVERIFIED and deliberately in NEITHER list because "how an
            // index row spells its segment has not been read, and a guess here
            // would file the engine's own surface under an invented code".
            // That restraint was right and it was waiting on a measurement.
            //
            // THE MEASUREMENT EXISTS. `api.kite.trade/instruments`, fetched
            // 19 Aug 2026 and held at `~/.brutex/masters/zerodha_instruments.csv`:
            // 114,546 rows, of which **136 carry `segment=INDICES` with
            // `exchange=NSE`** -- including `256265,NIFTY 50` and
            // `260105,NIFTY BANK`, the two instruments CLAUDE.md section 1 puts
            // the entire engine surface on, and `264969,INDIA VIX`.
            //
            // WHAT ITS ABSENCE COST, measured on the same day. Every one of
            // those 136 rows failed `segment_of` and was counted a row error --
            // the live log reads `kept 10049, row_errors 136` -- so the master
            // held no index at all and the spot pull refused with "zerodha does
            // not list BANKNIFTY", for a vendor that lists it. An unrecognised
            // segment is refused loudly, which is why this was a countable
            // error and not a silent drop; the count is what identified it.
            //
            // Recorded in docs/00-charter.md section 4z.
            Self::Zerodha => (&["NSE", "INDICES"], &["NFO-FUT", "NFO-OPT", "MCX", "BSE"]),
            Self::TrueData | Self::Gdfl => (&[], &[]),
        };
        if store.contains(&code) {
            Ok(SegmentVerdict::Store)
        } else if decline.contains(&code) {
            Ok(SegmentVerdict::Decline)
        } else {
            Err(InstrumentError::Malformed)
        }
    }

    /// Normalises this vendor's instrument-type code to `IDX`/`EQ`/`FUT`/`CE`/`PE`.
    ///
    /// `side` is the vendor's separate option-side column, needed because the
    /// secondary broker types every option as `OPTSTK`/`OPTIDX` and carries
    /// call-versus-put in its own field.
    ///
    /// # Errors
    ///
    /// [`InstrumentError::Malformed`] for an unrecognised code, for the same
    /// reason as [`Vendor::segment_of`].
    fn type_of(self, code: &str, side: &str) -> Result<Option<&'static str>, InstrumentError> {
        match self {
            Self::Groww => match code {
                "IDX" | "EQ" | "FUT" | "CE" | "PE" => Ok(Some(match code {
                    "IDX" => "IDX",
                    "EQ" => "EQ",
                    "FUT" => "FUT",
                    "CE" => "CE",
                    _ => "PE",
                })),
                _ => Err(InstrumentError::Malformed),
            },
            Self::Dhan => match code {
                "INDEX" => Ok(Some("IDX")),
                "EQUITY" => Ok(Some("EQ")),
                "FUTSTK" | "FUTIDX" => Ok(Some("FUT")),
                "OPTSTK" | "OPTIDX" => match side {
                    "CE" => Ok(Some("CE")),
                    "PE" => Ok(Some("PE")),
                    _ => Err(InstrumentError::Malformed),
                },
                // Currency derivatives are declined, not stored.
                "FUTCUR" | "OPTCUR" => Ok(None),
                _ => Err(InstrumentError::Malformed),
            },
            // AN ARCHIVE PUBLISHES NO INSTRUMENT TYPE, because it publishes no
            // master. Every code is unrecognised, which is refused by name
            // rather than mapped to a guess — an invented type here would file
            // a future as an option and nothing downstream could tell.
            // `EQ FUT CE PE`, from the vendor's own column table. There is no
            // index word in that alphabet and none is invented here — see the
            // segment arm above.
            Self::Zerodha => match code {
                "EQ" => Ok(Some("EQ")),
                "FUT" => Ok(Some("FUT")),
                // The side IS the type on this vendor, so `side` is not read.
                "CE" => Ok(Some("CE")),
                "PE" => Ok(Some("PE")),
                _ => Err(InstrumentError::Malformed),
            },
            Self::TrueData | Self::Gdfl => Err(InstrumentError::Malformed),
        }
    }

    /// The `segment` word that means INDEX for a vendor whose instrument-type
    /// alphabet has no index code, or `None` when the type column already says it.
    ///
    /// # Why this exists, and what its absence cost
    ///
    /// Kite's own column table publishes exactly four instrument types --
    /// `EQ, FUT, CE, PE` (`docs/00-charter.md` §4z, read from
    /// kite.trade/docs/connect/v3/market-quotes/). **There is no index word in
    /// that alphabet.** The class lives in `segment`, which the vendor writes as
    /// `INDICES`.
    ///
    /// [`decode_master_row`] resolved everything from the type word, so a Zerodha
    /// index row arrived as `EQ` and **three separate things went wrong at once**:
    ///
    /// | What | Consequence |
    /// |---|---|
    /// | The space-collapse is gated on `IDX` | `NIFTY BANK` reached `Symbol::new`, which admits no space, and was `Malformed` |
    /// | `(Segment, Kind)` is chosen by the type word | had it parsed, it would have been filed `(Cash, Equity)` -- an index stored as a share |
    /// | The row therefore never entered the master | *"zerodha does not list BANKNIFTY"*, for a vendor that lists it |
    ///
    /// Measured on the vendor's own file, 19 Aug 2026: `kept 10049`,
    /// **`row_errors 136`** -- and the file holds **exactly 136** NSE rows whose
    /// segment is `INDICES`. Every index row, and no other row.
    ///
    /// # Why the fix is one word and not three branches
    ///
    /// Answering it here turns the existing `"IDX"` arm into the whole repair:
    /// the collapse fires, the segment becomes [`Segment::Index`] and the kind
    /// becomes [`Kind::Index`], all from code that already existed and is already
    /// tested. Three separate vendor branches would have been three places to
    /// disagree.
    ///
    /// **Constant time.** A `match` over a `#[repr]` enum -- one jump -- and the
    /// caller's comparison is one `str` equality against a literal whose length is
    /// known at compile time.
    #[must_use]
    // NOT DUPLICATE ARMS, TWO REASONS WITH ONE ANSWER. Groww and Dhan already
    // publish an index code in the TYPE column, so consulting their segment
    // could reclassify a row; the archives have no master and therefore no
    // segment column at all. Collapsing them would delete the distinction that
    // makes the first one dangerous, so the lint is disarmed here and nowhere
    // else.
    #[allow(
        clippy::match_same_arms,
        reason = "same answer, different reasons -- see the comment above"
    )]
    pub const fn index_segment_word(self) -> Option<&'static str> {
        match self {
            // Both publish an index code in the type column, so the segment is
            // not consulted and must not be: Groww writes `IDX`, Dhan `INDEX`.
            Self::Groww | Self::Dhan => None,
            // The vendor's published type alphabet is `EQ FUT CE PE`. Its index
            // rows carry `EQ` there and `INDICES` here.
            Self::Zerodha => Some("INDICES"),
            // No master, so no segment column to read.
            Self::TrueData | Self::Gdfl => None,
        }
    }

    /// The exchange's canonical ticker for an index this vendor spells by NAME.
    ///
    /// # Why an alias is needed at all, and why it is not an invention
    ///
    /// Collapsing spaces is enough where the vendor writes the ticker with spaces
    /// in it. It is **not** enough where the vendor writes the index's NAME in the
    /// symbol column, and Zerodha does exactly that -- which the other two masters
    /// on this machine witness directly:
    ///
    /// | Vendor | symbol column | name column |
    /// |---|---|---|
    /// | Groww | `NIFTY` | `NIFTY 50` |
    /// | Dhan | `NIFTY` | `Nifty 50` |
    /// | **Zerodha** | **`NIFTY 50`** | `NIFTY 50` |
    ///
    /// Zerodha's `tradingsymbol` is character-for-character what the other two put
    /// in their *name* column, and both of them state the ticker for that name in
    /// the same row. So this table is READ off two masters that disagree with
    /// neither each other nor the exchange -- it is not a spelling somebody chose.
    /// `CLAUDE.md` §3 rule 1 wants a source; the source is
    /// `~/.brutex/masters/{groww_instruments,dhan_scrip}.csv`, recorded in
    /// `docs/00-charter.md` §4z.
    ///
    /// # Only where the collapsed form is not already the ticker
    ///
    /// `INDIA VIX` collapses to `INDIAVIX`, which is what Groww writes, so it needs
    /// no row here and does not get one. An alias that restates a collapse would be
    /// a second answer to a question the collapse already answers.
    ///
    /// **Constant time.** A `match` over string literals switches on the length
    /// first and compares at most one arm.
    #[must_use]
    pub fn index_alias(self, collapsed: &str) -> Option<&'static str> {
        match self {
            Self::Zerodha => match collapsed {
                "NIFTY50" => Some("NIFTY"),
                "NIFTYBANK" => Some("BANKNIFTY"),
                _ => None,
            },
            Self::Groww | Self::Dhan | Self::TrueData | Self::Gdfl => None,
        }
    }

    /// The vendor's own collapsed name for a symbol this engine renamed.
    ///
    /// The inverse of [`Self::index_alias`], and it exists because the rename
    /// **loses the exchange's name**. `NIFTY 50` is what NSE publishes and what
    /// Zerodha lists; `NIFTY` is what this repository decided to key the store
    /// on. Anything joining the store back to the exchange sees only the second
    /// and cannot find it — measured: `NIFTY` matched 80 published names and
    /// `BANKNIFTY` matched none, so the two instruments the engine actually
    /// sweeps were the two its own exchange join refused.
    ///
    /// **Kept beside `index_alias` so the pair cannot drift**, and pinned by a
    /// test that walks every arm of one through the other.
    ///
    /// **Constant time**, for the reason above: a `match` over string literals.
    #[must_use]
    pub fn index_alias_source(self, collapsed: &str) -> Option<&'static str> {
        match self {
            Self::Zerodha => match collapsed {
                "NIFTY" => Some("NIFTY50"),
                "BANKNIFTY" => Some("NIFTYBANK"),
                _ => None,
            },
            Self::Groww | Self::Dhan | Self::TrueData | Self::Gdfl => None,
        }
    }
}

/// Reads one master row into a canonical key.
///
/// # Errors
///
/// [`InstrumentError`] when a field is present but malformed, or when a
/// segment or instrument-type code is one this vendor is not known to emit. A
/// malformed row is an error rather than a skip: skipping is for rows that are
/// *validly* not ours, and quietly dropping a row we failed to understand is
/// how an instrument silently vanishes from a universe.
pub fn decode_master_row(vendor: Vendor, row: MasterRow<'_>) -> Result<Decoded, InstrumentError> {
    // THE WIDTH GATE IS FIRST, AND THAT POSITION IS THE WHOLE FIX.
    //
    // Everything below this line reads a vendor field: the ISIN parse in
    // `declined`, the exchange parse, and — the one that made this a defect
    // rather than a worry — the `TEST_MARKERS` substring scan, which searches
    // TWO raw fields with a seven-byte needle. Before D-0033 the only width
    // bound in the pipeline was `Symbol::new`, about a HUNDRED lines further
    // down, and it saw only whichever field became the identity. A 4 MiB
    // `trading_symbol` on a row with a populated `underlying` was scanned in
    // full, cost 246 us, and was then ACCEPTED. Even the rows it did refuse, it
    // refused only after paying the scan.
    //
    // `str::len` is a field read. Ten of them is a constant, whatever the
    // vendor sent. CLAUDE.md §3 rule 4.
    if let Some((field, len)) = row.over_wide() {
        return Err(InstrumentError::FieldTooWide { field, len });
    }

    // A declined row's ISIN is EVIDENCE, never identity, so it is parsed
    // leniently and only ever used to compare two vendors' verdicts. Computed
    // once here rather than at each of the seven decline sites below.
    let declined = |reason: Skip| {
        Ok(Decoded::Skipped(Declined {
            reason,
            isin: Isin::new(row.isin).ok(),
        }))
    };

    // Only NSE is stored. D-0017.
    //
    // The `Err` arm is separated from the `Ok(other)` arm on purpose. This was
    // `!matches!(Exchange::parse(..), Ok(Exchange::Nse))`, which threw the
    // `Err` away and filed an UNREADABLE code under the routine "foreign
    // exchange" decline. A master legitimately lists venues we do not store --
    // that is `Ok(Bse)` and it is routine. A code that does not parse is the
    // vendor having changed something under us, and it is not.
    match Exchange::parse(row.exchange) {
        Ok(Exchange::Nse) => {}
        Ok(_) => return declined(Skip::ForeignExchange),
        Err(_) => return declined(Skip::UnrecognisedExchange),
    }
    let exchange = Exchange::Nse;

    if TEST_MARKERS
        .iter()
        .any(|m| row.underlying.contains(m) || row.trading_symbol.contains(m))
    {
        return declined(Skip::TestInstrument);
    }

    // The vendor's segment column is a GATE, never our segment. The primary
    // broker files spot indices under `CASH`, so adopting its value would put
    // NIFTY in the equities directory and make `is_sweepable` false for the
    // two instruments the engine exists to sweep. Our segment comes from the
    // instrument type below, which is the only field whose meaning is stable
    // across vendors.
    if matches!(vendor.segment_of(row.segment)?, SegmentVerdict::Decline) {
        return declined(Skip::ForeignSegment);
    }

    // THE INDEX WORD, FOR A VENDOR WHOSE TYPE ALPHABET HAS NONE.
    //
    // Kite publishes exactly `EQ FUT CE PE` and puts the class in `segment`
    // instead. Resolving it here -- once, before anything reads `ty` -- is what
    // makes the existing `"IDX"` arm below do the whole repair: the space
    // collapse fires, the segment becomes `Segment::Index`, and the kind
    // becomes `Kind::Index`. Three defects, one word. See
    // `Vendor::index_segment_word` for what each of them cost.
    //
    // `None` for every vendor that already writes an index code, so this
    // comparison cannot reclassify a row at Groww or Dhan.
    let Some(ty) = vendor.type_of(row.instrument_type, row.option_side)? else {
        return declined(Skip::ForeignSegment);
    };
    // PROMOTED, FOR A VENDOR WHOSE TYPE ALPHABET HAS NO INDEX WORD.
    //
    // Kite publishes exactly `EQ FUT CE PE` and carries the class in `segment`
    // instead, so its index rows arrive here as `EQ`. One reassignment before
    // anything reads `ty` makes the existing `"IDX"` arm below do the whole
    // repair -- the space collapse fires, the segment becomes `Segment::Index`
    // and the kind becomes `Kind::Index`. THREE defects, one word; see
    // `Vendor::index_segment_word` for what each of them cost and for the
    // measurement (136 row errors, 136 index rows, no other row).
    //
    // `index_segment_word` is `None` for every vendor that already writes an
    // index code, so this cannot reclassify a Groww or Dhan row: the
    // comparison is against `None` and fails immediately.
    let ty = if vendor.index_segment_word() == Some(row.segment) {
        "IDX"
    } else {
        ty
    };

    // WHERE THE INSTRUMENT IS NAMED DEPENDS ON WHAT IT IS.
    //
    // The primary broker leaves `underlying_symbol` EMPTY on every cash and
    // index row -- all 4,104 of them, NIFTY and BANKNIFTY included. The name
    // lives in `trading_symbol` there. On derivative rows the opposite holds:
    // `trading_symbol` is the contract (`ASHOKLEY26SEP117.5CE`, which contains
    // a `.` and is not a legal Symbol), while `underlying_symbol` is the
    // underlying we actually want.
    //
    // Reading one column for everything fails either way round. This was found
    // by decoding the real master, not by reading it.
    // WHERE THE TICKER LIVES IS PER-VENDOR AND OPPOSITE BETWEEN THEM.
    //
    // Groww leaves `underlying_symbol` EMPTY on all 4,104 cash and index rows
    // and puts the ticker in `trading_symbol`.
    //
    // Dhan does the reverse: `UNDERLYING_SYMBOL` is the ticker (`GOLDSTAR`,
    // `ARE&M`) while `SYMBOL_NAME` is the COMPANY NAME -- "GOLDSTAR POWER
    // LIMITED", "AMARA RAJA ENERGY MOB LTD". 9,623 of its 9,674 NSE equity
    // rows carry a space there, so reading it refused almost the entire
    // vendor. Measured, not guessed.
    //
    // Preferring the underlying and falling back to the trading symbol
    // satisfies both without a vendor branch: Groww's underlying is empty so
    // the fallback fires; Dhan's is populated so it wins.
    let name = if row.underlying.is_empty() {
        row.trading_symbol
    } else {
        row.underlying
    };
    // AN ASCII SPACE IS NOT INFORMATION IN AN EXCHANGE IDENTIFIER, AND KEEPING
    // IT COST 104 INDEX ROWS.
    //
    // The two masters spell an index two different ways. Groww writes the
    // exchange's canonical ticker -- `NIFTYPVTBANK`, `NIFTYMIDCAP150`,
    // `INDIAVIX`. Dhan writes the display name -- `NIFTY PVT BANK`,
    // `NIFTY MIDCAP 150`, `INDIA VIX`. `Symbol::new` admits `A-Z 0-9 - _ &`
    // and nothing else, so every spaced form was `InstrumentError::Malformed`.
    //
    // Measured over the vendor's own file: of its 119 NSE index rows, 15 were
    // legal and 104 were refused. Those 104 are why this vendor reached 15 of
    // the 35 reference indices while the other reached 24.
    //
    // Collapsing the spaces:
    //
    //   * makes 119 of 119 legal, and the longest -- `NIFTY100 LOW VOLATILITY
    //     30`, 26 characters -- becomes 23 and fits `SYMBOL_CAPACITY`;
    //   * creates ZERO collisions among those 119;
    //   * collides with ZERO of the 2,781 NSE cash equity symbols;
    //   * and raises agreement with the other vendor's 24 index symbols from
    //     4 to 17, because the collapsed form IS what the other vendor already
    //     writes. `BANKNIFTY`, `FINNIFTY`, `NIFTY`, `INDIAVIX` are all in that
    //     recovered set.
    //
    // So this is a normalisation TO the canonical ticker, not a lossy edit: the
    // other master is the witness that the space carries nothing.
    //
    // CONFINED TO INDEX ROWS, AND THAT RESTRAINT IS THE POINT.
    //
    // The first draft collapsed every row. It was safe by measurement -- not
    // one of the 2,781 NSE cash equity symbols contains a space -- and it was
    // still wrong, because it silently repaired inputs nobody had claimed were
    // repairable. `a_malformed_row_errors_rather_than_being_skipped_silently`
    // caught it: that test feeds `"NIF TY"` on an F&O row and requires an
    // error, and under a blanket collapse it became `NIFTY` and was accepted.
    // A stray space in a derivative ticker is CORRUPTION, and turning it into
    // a real instrument is the §4 fallback that hides a failure.
    //
    // Only an index carries a name the exchange itself writes with spaces, so
    // only an index gets the normalisation. Everywhere else a space stays what
    // it was: malformed, loudly.
    //
    // `ty` is already resolved above, so this costs a comparison and no scan.
    // AND THE COLLAPSED FORM IS NOT ALWAYS THE TICKER.
    //
    // Collapsing is enough where the vendor writes the ticker WITH spaces in
    // it -- Dhan's `NIFTY PVT BANK`. It is not enough where the vendor writes
    // the index's NAME in the symbol column, which Zerodha does: `NIFTY 50`
    // collapses to `NIFTY50` and the exchange's ticker is `NIFTY`. Both other
    // masters on this machine carry that identity in one row -- symbol
    // `NIFTY`, name `NIFTY 50` -- which is what makes `index_alias` a reading
    // rather than a choice. `INDIA VIX` needs no row there: it collapses to
    // `INDIAVIX`, which is already what Groww writes.
    let underlying = if ty == "IDX" {
        let collapsed = collapse_spaces(name)?;
        match vendor.index_alias(collapsed.as_str()) {
            Some(canonical) => Symbol::new(canonical)?,
            None => Symbol::new(collapsed.as_str())?,
        }
    } else {
        Symbol::new(name)?
    };

    // A live instrument master lists only CURRENTLY LISTED contracts -- both
    // vendors purge on expiry, and the earliest expiry in either master is
    // three days from now. So every derivative row here is live by definition,
    // and none of it is backtest data. The expiry is still PARSED first, so a
    // malformed date is an error rather than being hidden behind the skip.
    let (segment, kind) = match ty {
        "IDX" => (Segment::Index, Kind::Index),
        // THE EQUITY GATE APPLIES HERE AND ONLY HERE. An index row carries no
        // series -- Groww's `series` is empty on all 24 of its NSE index rows
        // and Dhan writes `NA` -- so gating any earlier deletes NIFTY and
        // BANKNIFTY.
        // A VENDOR THAT PUBLISHES NO SERIES COLUMN CANNOT BE GATED ON ONE, and
        // treating its absence as an unrecognised value declined EVERY EQUITY
        // ROW IT HAS.
        //
        // Measured on a real-shaped Kite dump: kept = 0, and
        // `skipped_by_reason` was `[("unrecognised listing class", 2)]` for two
        // perfectly ordinary NSE equities. `api::master` maps an absent column
        // to the empty string, `board_of("")` matches none of the three series
        // tables, and `Unrecognised` is a decline — so a feed whose universe is
        // empty by construction reported itself as a vendor publishing rows
        // this build did not understand. Both are silent-looking states and
        // only one of them was true.
        //
        // ABSENT AND UNRECOGNISED ARE DIFFERENT FACTS. An empty value in a
        // column the vendor HAS is a row this build cannot classify, and it is
        // still declined by name. No column at all is a question this vendor's
        // master cannot answer, and the honest response is to let the row
        // through ungated rather than to answer it wrongly.
        //
        // WHAT THAT COSTS, STATED RATHER THAN DISCOVERED. D-0025's board gate
        // is what keeps SME and debt listings out of the equity universe, and
        // it does not run for such a vendor: its cash rows are kept on the
        // instrument type alone. Nothing else in this decoder is weakened, and
        // the vendors that DO publish a series are gated exactly as before —
        // the arm below is reached only when `master_columns().listing_class`
        // is empty, which is a property of the vendor and not of the row.
        "EQ" if vendor.master_columns().listing_class.is_empty() => (Segment::Cash, Kind::Equity),
        "EQ" => match board_of(row.listing_class) {
            EquityVerdict::MainBoard => (Segment::Cash, Kind::Equity),
            EquityVerdict::Sme => return declined(Skip::SmeBoard),
            EquityVerdict::NotEquity => return declined(Skip::NotEquityListing),
            EquityVerdict::Unrecognised => return declined(Skip::UnrecognisedListingClass),
        },
        "FUT" => {
            parse_expiry(row.expiry)?;
            return declined(Skip::LiveContract);
        }
        _ => {
            parse_expiry(row.expiry)?;
            parse_strike(row.strike_rupees)?;
            return declined(Skip::LiveContract);
        }
    };

    let key = InstrumentKey {
        exchange,
        segment,
        underlying,
        kind,
    };

    // THE ISIN IS READ FOR EQUITY AND NOTHING ELSE, and it is read only after
    // the gate above. Both halves of that sentence are load-bearing:
    //
    //   * An index row's ISIN column is not empty, it is JUNK. Measured: Groww
    //     writes the index ticker there (`NIFTY`) and Dhan writes `NA`.
    //     Parsing it would refuse every index in both masters.
    //   * The one row in either master whose check digit does not verify,
    //     `IN1520250085`, is a state development loan on series `SG`. It never
    //     reaches this line because the gate declined it first. Order is what
    //     keeps that true.
    //
    // On an equity row the ISIN is REQUIRED, not optional: every one of the
    // 2,726 Groww and 2,774 Dhan main-board rows carries one, so a missing or
    // malformed value means the row is not what we think it is, and that is an
    // error rather than a quiet `None`.
    //
    // AND IT IS REQUIRED OF A VENDOR THAT PUBLISHES THE COLUMN, which is the
    // clause that was missing. The sentence above counts Groww's and Dhan's
    // main-board rows, and both of those masters HAVE an isin column; Kite's
    // has twelve columns and not one of them is an ISIN. So `Isin::new("")`
    // refused, `?` turned it into `Malformed`, and every ordinary NSE equity in
    // that master came back as a malformed instrument identifier — measured,
    // kept = 0, alongside the board-series gate above which failed for exactly
    // the same reason one field earlier.
    //
    // A vendor that publishes no ISIN cannot be asked for one. The row is kept
    // with `None`, which is the honest value and is precisely what
    // `pull::universe::JoinKey::TradingSymbol` exists to join on — the design
    // already accounts for this feed having no ISIN; this line did not.
    //
    // Nothing is weakened for the two vendors that DO publish it: the arm turns
    // on `master_columns()`, a property of the VENDOR, so a missing or
    // malformed value in a column that exists is still the error it always was.
    let isin = match kind {
        Kind::Equity if !vendor.master_columns().isin.is_empty() => Some(Isin::new(row.isin)?),
        _ => None,
    };

    let Some(vendor_id) = VendorId::new(row.vendor_id) else {
        // A row with no usable id cannot be requested from the vendor, so it is
        // DECLINED rather than kept: keeping it would put an instrument in the
        // index that every later lookup would find and no pull could name.
        return declined(Skip::NoVendorId);
    };
    Ok(Decoded::Keep(Listing {
        vendor_id,
        key,
        isin,
        unsuffixed: unsuffixed_key(key, name, row.listing_class)?,
    }))
}

/// The same key with the row's own series suffix removed, if it has one.
///
/// `BLUECHIP-BE` under series `BE` gives `BLUECHIP`; `RELIANCE` under `EQ`
/// gives nothing; and `BAJAJ-AUTO` under `EQ` gives nothing either, which is
/// the whole point — the trailing segment must BE the row's own series, not
/// merely look like a suffix.
///
/// # Errors
///
/// [`InstrumentError::Malformed`] if stripping would leave nothing to name the
/// instrument with. A row called `-EQ` is a vendor bug, and a bug is loud here
/// rather than silently unstripped.
fn unsuffixed_key(
    key: InstrumentKey,
    name: &str,
    class: &str,
) -> Result<Option<InstrumentKey>, InstrumentError> {
    // Only a cash listing has a series. An empty class would make
    // `strip_suffix` succeed on every symbol, so it is excluded explicitly.
    let class = class.trim();
    if key.kind != Kind::Equity || class.is_empty() {
        return Ok(None);
    }
    let Some(stripped) = name
        .strip_suffix(class)
        .and_then(|rest| rest.strip_suffix('-'))
    else {
        return Ok(None);
    };
    Ok(Some(InstrumentKey {
        underlying: Symbol::new(stripped)?,
        ..key
    }))
}

/// Parses an `YYYY-MM-DD` expiry from the master's own column.
///
/// # Errors
///
/// [`InstrumentError::Malformed`] on any shape other than exactly
/// `YYYY-MM-DD` with numeric parts, or on a date that does not exist.
fn parse_expiry(text: &str) -> Result<Expiry, InstrumentError> {
    let mut parts = text.split('-');
    let (Some(y), Some(m), Some(d), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(InstrumentError::Malformed);
    };
    // Fixed widths, so a value like `2026-8-4` is refused rather than guessed
    // at — a vendor that changes its date format should be a loud failure.
    if y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return Err(InstrumentError::Malformed);
    }
    // WIDTH IS NOT SHAPE. `u8::from_str` accepts a leading sign, so `"+8"` is
    // two bytes wide and parses as 8 -- `2026-+8-+4` decoded as August 4th
    // through a function whose doc says "exactly `YYYY-MM-DD` with numeric
    // parts". A vendor changing its date format must be a loud failure, and
    // a sign is a different format.
    if !text.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
        return Err(InstrumentError::Malformed);
    }
    // COMPUTED, NOT PARSED. Every byte is now known to be an ASCII digit and
    // the widths are fixed, so `from_str` cannot fail here: four digits reach
    // at most 9999 and `u16::MAX` is 65535; two reach at most 99 and `u8::MAX`
    // is 255. Keeping `.map_err(..)` would leave three error arms no input can
    // enter -- unreachable code behind a `?`, which is either an untestable
    // branch or a fallback that hides nothing. Folding the digits makes the
    // question not arise, and is law 4: arithmetic beats lookup.
    let year = y
        .bytes()
        .fold(0u16, |acc, b| acc * 10 + u16::from(b - b'0'));
    let month = m.bytes().fold(0u8, |acc, b| acc * 10 + (b - b'0'));
    let day = d.bytes().fold(0u8, |acc, b| acc * 10 + (b - b'0'));
    Expiry::new(year, month, day)
}

/// Converts a rupee strike to paisa.
///
/// # Errors
///
/// [`InstrumentError::Malformed`] if the value is not a number or does not fit
/// in `i64` paisa.
fn parse_strike(text: &str) -> Result<Paisa, InstrumentError> {
    // Exact, from the TEXT. This used to parse the column into a binary float and hand it
    // to `Paisa::from_rupees_half_up`, discarding an exact decimal one line before the only
    // function in the workspace allowed to approximate one. An audit measured 271 mismatches
    // across the 40,000 three-decimal strings from "0.000" to "39.999" -- "0.145" became 14
    // paisa where exact half-up is 15 -- always losing downward. The master file is text and
    // nothing has been lost yet when this is called.
    Paisa::from_rupee_text_half_up(text).map_err(|_| InstrumentError::Malformed)
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

    /// A real main-board ISIN, so that a row which reaches the ISIN parse
    /// carries one. `RELIANCE`, verbatim from both masters.
    const REAL_ISIN: &str = "INE002A01018";

    /// The whole kept listing, or `None` if the row was skipped.
    ///
    /// Returning an Option rather than destructuring with `else { panic!() }`
    /// keeps every branch reachable, so the coverage gate stays honest: an
    /// unreachable panic arm is an uncovered region that no test can ever
    /// exercise.
    fn listing(d: Decoded) -> Option<Listing> {
        match d {
            Decoded::Keep(l) => Some(l),
            Decoded::Skipped(_) => None,
        }
    }

    /// The kept key alone, for the tests that care only about identity.
    fn kept(d: Decoded) -> Option<InstrumentKey> {
        listing(d).map(|l| l.key)
    }

    /// Builds a Groww-shaped row. `trading_symbol` mirrors `underlying` for
    /// derivative rows, which is what the real master does.
    ///
    /// The listing class and ISIN are those of an ordinary main-board share,
    /// because that is what most rows are; every test of the equity gate sets
    /// them explicitly rather than relying on this.
    fn row<'a>(
        exchange: &'a str,
        segment: &'a str,
        underlying: &'a str,
        ty: &'a str,
        expiry: &'a str,
        strike: &'a str,
    ) -> MasterRow<'a> {
        MasterRow {
            vendor_id: "1333",
            exchange,
            segment,
            underlying,
            trading_symbol: underlying,
            instrument_type: ty,
            listing_class: "EQ",
            isin: REAL_ISIN,
            expiry,
            strike_rupees: strike,
            option_side: "",
        }
    }

    /// Decodes a Groww row.
    fn groww(r: MasterRow<'_>) -> Result<Decoded, InstrumentError> {
        decode_master_row(Vendor::Groww, r)
    }

    /// A decline with no ISIN beside it, for the helpers' own tests.
    fn bare(reason: Skip) -> Decoded {
        Decoded::Skipped(Declined { reason, isin: None })
    }

    #[test]
    fn the_kept_helper_covers_its_negative_arm() {
        assert!(kept(bare(Skip::TestInstrument)).is_none());
        assert!(kept(bare(Skip::LiveContract)).is_none());
        // `skip` is the mirror image, and both arms are exercised here so no
        // caller has to prove them again.
        assert_eq!(bare(Skip::LiveContract).skip(), Some(Skip::LiveContract));
        let keep = groww(row("NSE", "CASH", "RELIANCE", "EQ", "", "")).expect("ok");
        assert_eq!(keep.skip(), None, "a kept row was not skipped");
    }

    /// An index name is normalised to the exchange's canonical ticker; a space
    /// anywhere else is still malformed.
    #[test]
    fn an_index_name_loses_its_spaces_and_nothing_else_does() {
        // THE 104 ROWS THIS RECOVERS. The second vendor writes the display
        // name, the first writes the ticker, and the ticker is what both now
        // produce -- so the two masters agree on one key instead of one of
        // them having no key at all.
        for (written, want) in [
            ("NIFTY MIDCAP 150", "NIFTYMIDCAP150"),
            ("NIFTY PVT BANK", "NIFTYPVTBANK"),
            ("NIFTY100 EQUAL WEIGHT", "NIFTY100EQUALWEIGHT"),
            ("INDIA VIX", "INDIAVIX"),
            // 26 characters as written, 23 collapsed -- the longest in the
            // file, and the reason the buffer is checked rather than assumed.
            ("NIFTY100 LOW VOLATILITY 30", "NIFTY100LOWVOLATILITY30"),
        ] {
            let got = groww(row("NSE", "CASH", written, "IDX", "", ""))
                .unwrap_or_else(|why| panic!("{written:?} must decode, got {why:?}"));
            let key = kept(got).unwrap_or_else(|| panic!("{written:?} must be kept"));
            assert_eq!(key.underlying.as_str(), want);
            assert_eq!(key.kind, Kind::Index);
        }

        // ALREADY CANONICAL, AND UNTOUCHED. The other vendor's spelling must
        // land on exactly the same symbol, or the normalisation has split the
        // instrument instead of joining it.
        let plain = kept(groww(row("NSE", "CASH", "NIFTYPVTBANK", "IDX", "", "")).expect("ok"))
            .expect("kept");
        assert_eq!(plain.underlying.as_str(), "NIFTYPVTBANK");

        // AND THE RESTRAINT. A space on any other kind of row is corruption,
        // not a spelling, and is still refused -- see the comment at the call
        // site for the draft that got this wrong.
        assert!(
            groww(row("NSE", "FNO", "NIF TY", "FUT", "2026-08-04", "")).is_err(),
            "a space outside an index row must stay malformed"
        );
        assert!(
            groww(row("NSE", "CASH", "RELI ANCE", "EQ", "", "")).is_err(),
            "an equity ticker with a space is corruption, not a display name"
        );

        // A name that still will not fit once collapsed is refused rather than
        // truncated into some other instrument's symbol.
        assert!(
            groww(row(
                "NSE",
                "CASH",
                "NIFTY VERY LONG INDEX NAME THAT OVERFLOWS",
                "IDX",
                "",
                ""
            ))
            .is_err(),
            "over capacity after collapsing is malformed, never truncated"
        );
    }

    #[test]
    fn vendor_path_segments_are_stable() {
        assert_eq!(Vendor::Groww.as_str(), "groww");
        assert_eq!(Vendor::Dhan.as_str(), "dhan");
        assert_ne!(Vendor::Groww, Vendor::Dhan);
    }

    #[test]
    fn the_two_engine_indices_decode() {
        // Exactly as they appear in the real master:
        //   NSE,NIFTY,NIFTY,NSE-NIFTY,NIFTY 50,IDX,CASH,...
        for sym in ["NIFTY", "BANKNIFTY"] {
            let got = groww(row("NSE", "CASH", sym, "IDX", "", "")).expect("well formed");
            let key = kept(got).expect("must be kept");
            assert_eq!(key.kind, Kind::Index);
            assert!(key.is_sweepable(), "{sym} is one of the two swept");
        }
    }

    #[test]
    fn exchange_test_instruments_are_skipped() {
        // Real rows: 031NSETEST36DECFUT and 061NSETEST36DECFUT. Storing these
        // would put fabricated instruments beside real ones, indistinguishable
        // afterwards.
        for u in ["031NSETEST", "061NSETEST", "BSETEST01"] {
            assert_eq!(
                groww(row("NSE", "FNO", u, "FUT", "2036-11-27", ""))
                    .expect("ok")
                    .skip(),
                Some(Skip::TestInstrument),
                "{u} must be skipped"
            );
        }
    }

    #[test]
    fn bse_and_unknown_exchanges_are_skipped_not_stored() {
        // D-0017 -- NSE only. BSE is a venue this engine RECOGNISES and does
        // not store, so it is the routine decline.
        assert_eq!(
            groww(row("BSE", "CASH", "SENSEX", "IDX", "", ""))
                .expect("ok")
                .skip(),
            Some(Skip::ForeignExchange)
        );
        assert!(Skip::ForeignExchange.is_routine());

        // `MCX` is a code `Exchange::parse` cannot read at all. This test used
        // to assert it was ALSO `ForeignExchange` -- it encoded the defect
        // rather than catching it, which is why the defect survived a suite at
        // 100% coverage. The two are now different reasons, and only one of
        // them is routine.
        assert_eq!(
            groww(row("MCX", "COMMODITY", "GOLD", "FUT", "2026-08-05", ""))
                .expect("ok")
                .skip(),
            Some(Skip::UnrecognisedExchange)
        );
    }

    #[test]
    fn an_unreadable_exchange_degrades_the_run_and_a_foreign_one_does_not() {
        // The failure this separation exists to stop: `NSE` drifting to
        // `NSE_EQ` -- a rename, a padded column, a field shifted by one --
        // declined EVERY row of BOTH masters as "foreign exchange", which is
        // routine, so the process printed `ok` and exited zero on a universe
        // of nothing. `Vendor::segment_of` raises a loud error for exactly
        // this shape one gate later; the exchange gate was the one left quiet.
        for unreadable in ["NSE_EQ", "nse", " NSE", "NSE ", "", "N", "NSEX"] {
            let d = groww(row(unreadable, "CASH", "RELIANCE", "EQ", "", ""))
                .expect("a decline, not an error");
            assert_eq!(
                d.skip(),
                Some(Skip::UnrecognisedExchange),
                "{unreadable:?} is not a venue, it is unreadable"
            );
            assert!(
                !Skip::UnrecognisedExchange.is_routine(),
                "{unreadable:?} must reach the exit code"
            );
        }

        // And the two reasons never render as one string, so a report cannot
        // merge them back together.
        assert_ne!(
            Skip::UnrecognisedExchange.reason(),
            Skip::ForeignExchange.reason()
        );
    }

    #[test]
    fn commodity_segment_is_skipped() {
        assert_eq!(
            groww(row("NSE", "COMMODITY", "GOLD", "FUT", "2026-08-05", ""))
                .expect("ok")
                .skip(),
            Some(Skip::ForeignSegment)
        );
    }

    #[test]
    fn an_equity_decodes_and_sweeps_iff_it_is_an_fno_underlying() {
        // Was `an_equity_decodes_and_is_stored_not_swept`, pinning D-0018.
        // D-0506 widened the surface to the F&O cash equities, so a decoded
        // equity's sweepability is now the one table lookup, and this test
        // says so from both sides of it.
        let got = groww(row("NSE", "CASH", "RELIANCE", "EQ", "", "")).expect("ok");
        let key = kept(got).expect("kept");
        assert_eq!(key.kind, Kind::Equity);
        assert!(
            crate::universe::FNO_INDEX.contains("RELIANCE"),
            "RELIANCE is an F&O underlying, or this fixture is wrong"
        );
        assert!(key.is_sweepable(), "D-0506: an F&O underlying is swept");

        // A well-formed symbol the F&O list does not hold decodes the same way
        // and stays stored-only. The fixture checks its own premise.
        assert!(!crate::universe::FNO_INDEX.contains("ZZQXNOTFNO"));
        let got = groww(row("NSE", "CASH", "ZZQXNOTFNO", "EQ", "", "")).expect("ok");
        let key = kept(got).expect("kept");
        assert_eq!(key.kind, Kind::Equity);
        assert!(!key.is_sweepable(), "D-0018 still holds off the F&O list");
    }

    #[test]
    fn a_malformed_row_errors_rather_than_being_skipped_silently() {
        // Skipping is for rows that are VALIDLY not ours. A row we failed to
        // understand must be loud, or an instrument vanishes without trace.
        assert!(groww(row("NSE", "FNO", "NIFTY", "XX", "2026-08-04", "1")).is_err());
        assert!(groww(row("NSE", "FNO", "NIFTY", "CE", "not-a-date", "1")).is_err());
        assert!(groww(row("NSE", "FNO", "NIFTY", "CE", "2026-08-04", "abc")).is_err());
        assert!(groww(row("NSE", "FNO", "NIF TY", "FUT", "2026-08-04", "")).is_err());
    }

    #[test]
    fn a_signed_date_component_is_refused_rather_than_read_as_a_number() {
        // WIDTH IS NOT SHAPE. `u8::from_str` accepts a leading sign, so `"+8"`
        // is two bytes wide and parses as 8 -- the width check passed it and
        // `2026-+8-+4` decoded as August 4th, through a function documented as
        // "exactly YYYY-MM-DD with numeric parts".
        for bad in [
            "2026-+8-04",
            "2026-08-+4",
            "2026-+8-+4",
            "+026-08-04",
            "2026--8-04",
        ] {
            assert!(
                groww(row("NSE", "FNO", "NIFTY", "FUT", bad, "")).is_err(),
                "{bad} is not a date this engine reads"
            );
        }
        // And the real shape still decodes untouched.
        assert!(groww(row("NSE", "FNO", "NIFTY", "FUT", "2026-08-04", "")).is_ok());
    }

    #[test]
    fn a_right_length_but_non_numeric_date_part_is_refused() {
        // Distinct from the wrong-LENGTH cases below: these have exactly the
        // 4-2-2 shape, so they pass the width check and must be caught by the
        // numeric parse. Without this, a vendor emitting "20X6-08-04" would
        // reach Expiry::new with whatever a lenient parse produced.
        for bad in ["20X6-08-04", "2026-0X-04", "2026-08-0X", "----------"] {
            assert!(
                groww(row("NSE", "FNO", "NIFTY", "FUT", bad, "")).is_err(),
                "{bad} must be refused"
            );
        }
    }

    #[test]
    fn expiry_column_must_be_exactly_yyyy_mm_dd() {
        // A vendor that changes its date format is a loud failure, not a guess.
        for bad in [
            "2026-8-4",
            "26-08-04",
            "2026/08/04",
            "2026-08",
            "",
            "2026-08-04-01",
        ] {
            assert!(
                groww(row("NSE", "FNO", "NIFTY", "FUT", bad, "")).is_err(),
                "{bad} must be refused"
            );
        }
        // And an impossible date is refused by Expiry itself.
        assert!(groww(row("NSE", "FNO", "NIFTY", "FUT", "2026-02-31", "")).is_err());
    }

    #[test]
    fn the_real_nifty_row_decodes_from_the_columns_the_master_actually_fills() {
        // THE ROW THAT BROKE EVERYTHING. Verbatim from the master:
        //   NSE,NIFTY,NIFTY,NSE-NIFTY,NIFTY 50,IDX,CASH,,NIFTY,,,,,,,,,0,0,,0
        //         ^col3 trading_symbol            ^col10 underlying_symbol = EMPTY
        //
        // underlying_symbol is empty on ALL 4,104 Groww cash and index rows,
        // NIFTY and BANKNIFTY among them. The previous test passed "NIFTY" as
        // the underlying -- a col-3 value fed into the col-10 field -- so a
        // total decode failure on the engine's primary instrument was green.
        let real = MasterRow {
            vendor_id: "1333",
            exchange: "NSE",
            segment: "CASH",
            underlying: "", // <-- exactly as the file has it
            trading_symbol: "NIFTY",
            instrument_type: "IDX",
            // Empty series and an isin column holding the TICKER, both
            // exactly as the file has them. Neither may reach a validator.
            listing_class: "",
            isin: "NIFTY",
            expiry: "",
            strike_rupees: "",
            option_side: "",
        };
        let key = kept(groww(real).expect("the real row must decode")).expect("kept");
        assert_eq!(key.underlying.as_str(), "NIFTY");
        assert_eq!(key.segment, Segment::Index);
        assert!(
            key.is_sweepable(),
            "NIFTY must be sweepable from its real row"
        );
    }

    #[test]
    fn an_unknown_vendor_code_is_loud_never_a_silent_decline() {
        // This is the whole lesson. Treating an unrecognised code as "not ours"
        // is what let 200,460 Dhan rows disappear while reporting a routine
        // skip. A mapping bug must never look like a legitimate refusal.
        let bad_segment = MasterRow {
            vendor_id: "1333",
            exchange: "NSE",
            segment: "Z",
            underlying: "NIFTY",
            trading_symbol: "NIFTY",
            instrument_type: "INDEX",
            listing_class: "NA",
            isin: "NA",
            expiry: "",
            strike_rupees: "",
            option_side: "",
        };
        assert!(decode_master_row(Vendor::Dhan, bad_segment).is_err());
        // Groww's letters are not Dhan's, and vice versa.
        assert!(groww(bad_segment).is_err());
        let dhan_shaped_at_groww = MasterRow {
            vendor_id: "1333",
            segment: "I",
            ..bad_segment
        };
        assert!(
            groww(dhan_shaped_at_groww).is_err(),
            "Dhan codes must not silently work for Groww"
        );

        let bad_type = MasterRow {
            vendor_id: "1333",
            segment: "D",
            instrument_type: "DBT",
            ..bad_segment
        };
        assert!(decode_master_row(Vendor::Dhan, bad_type).is_err());
    }

    #[test]
    fn currency_and_commodity_are_declined_not_stored() {
        let cur = MasterRow {
            vendor_id: "1333",
            exchange: "NSE",
            segment: "C",
            underlying: "USDINR",
            trading_symbol: "USDINR",
            instrument_type: "OPTCUR",
            listing_class: "",
            isin: "",
            expiry: "2026-08-04",
            strike_rupees: "83.625",
            option_side: "CE",
        };
        assert_eq!(
            decode_master_row(Vendor::Dhan, cur).expect("ok").skip(),
            Some(Skip::ForeignSegment)
        );
    }

    #[test]
    fn a_declined_instrument_type_is_a_skip_with_a_reason() {
        // Currency derivatives are a recognised type this engine does not
        // store -- distinct from an unrecognised code, which is an error.
        let cur = MasterRow {
            vendor_id: "1333",
            exchange: "NSE",
            segment: "D",
            underlying: "USDINR",
            trading_symbol: "USDINR26AUGFUT",
            instrument_type: "FUTCUR",
            listing_class: "",
            isin: "",
            expiry: "2026-08-26",
            strike_rupees: "",
            option_side: "",
        };
        assert_eq!(
            decode_master_row(Vendor::Dhan, cur).expect("ok").skip(),
            Some(Skip::ForeignSegment)
        );
    }

    #[test]
    fn every_live_derivative_is_skipped_not_stored() {
        // The operator's rule, enforced at the decoder: a live instrument
        // master lists ONLY currently-listed contracts -- both vendors purge
        // on expiry and the earliest expiry in either is days away. Backtests
        // run on EXPIRED contracts, which come from the historical endpoints
        // and the lake. Storing the live chain adds ~148,000 contracts holding
        // a few weeks each, none ever swept.
        for (ty, strike) in [("FUT", ""), ("CE", "19450"), ("PE", "19450")] {
            assert_eq!(
                groww(row("NSE", "FNO", "NIFTY", ty, "2026-08-04", strike))
                    .expect("ok")
                    .skip(),
                Some(Skip::LiveContract),
                "{ty} must be skipped as a live contract"
            );
        }
        // Dhan's spellings too.
        for ty in ["FUTIDX", "FUTSTK", "OPTIDX", "OPTSTK"] {
            let r = MasterRow {
                vendor_id: "1333",
                exchange: "NSE",
                segment: "D",
                underlying: "NIFTY",
                trading_symbol: "NIFTY",
                instrument_type: ty,
                listing_class: "",
                isin: "",
                expiry: "2026-08-04",
                strike_rupees: "19450",
                option_side: "CE",
            };
            assert_eq!(
                decode_master_row(Vendor::Dhan, r).expect("ok").skip(),
                Some(Skip::LiveContract),
                "{ty} must be skipped"
            );
        }
    }

    #[test]
    fn dhan_reads_the_option_side_from_its_own_column() {
        // Dhan types EVERY option as OPTSTK/OPTIDX and carries call-versus-put
        // in a separate field, so both values must be read and anything else
        // must be loud -- an option whose side we cannot read is not an option
        // we can store.
        let opt = |side| MasterRow {
            vendor_id: "1333",
            exchange: "NSE",
            segment: "D",
            underlying: "NIFTY",
            trading_symbol: "NIFTY",
            instrument_type: "OPTIDX",
            listing_class: "",
            isin: "",
            expiry: "2026-08-04",
            strike_rupees: "19450",
            option_side: side,
        };
        for side in ["CE", "PE"] {
            assert_eq!(
                decode_master_row(Vendor::Dhan, opt(side))
                    .expect("ok")
                    .skip(),
                Some(Skip::LiveContract),
                "{side} must decode before being declined as live"
            );
        }
        for side in ["", "XX", "ce"] {
            assert_eq!(
                decode_master_row(Vendor::Dhan, opt(side)),
                Err(InstrumentError::Malformed),
                "{side:?} is not a side this vendor emits"
            );
        }
    }

    #[test]
    fn a_malformed_derivative_still_errors_rather_than_hiding_behind_the_skip() {
        // The expiry and strike are parsed BEFORE the skip, so a vendor that
        // starts emitting a bad date is a loud failure rather than being
        // silently swallowed by "it was live anyway".
        assert!(groww(row("NSE", "FNO", "NIFTY", "FUT", "not-a-date", "")).is_err());
        assert!(groww(row("NSE", "FNO", "NIFTY", "CE", "2026-08-04", "abc")).is_err());
        assert!(groww(row("NSE", "FNO", "NIFTY", "CE", "2026-02-31", "1")).is_err());
    }

    #[test]
    fn indices_and_equities_are_still_kept_from_both_vendors() {
        // Both name columns empty is unnameable -- an ERROR, never a skip,
        // because a row we cannot name is how an instrument silently vanishes.
        assert!(groww(row("NSE", "CASH", "", "IDX", "", "")).is_err());

        let nifty = kept(
            groww(MasterRow {
                vendor_id: "1333",
                exchange: "NSE",
                segment: "CASH",
                underlying: "",
                trading_symbol: "NIFTY",
                instrument_type: "IDX",
                listing_class: "",
                isin: "NIFTY",
                expiry: "",
                strike_rupees: "",
                option_side: "",
            })
            .expect("ok"),
        )
        .expect("kept");
        assert!(nifty.is_sweepable());

        let dhan_eq = kept(
            decode_master_row(
                Vendor::Dhan,
                MasterRow {
                    vendor_id: "1333",
                    exchange: "NSE",
                    segment: "E",
                    underlying: "RELIANCE",
                    trading_symbol: "RELIANCE INDUSTRIES LTD",
                    instrument_type: "EQUITY",
                    listing_class: "EQ",
                    isin: REAL_ISIN,
                    expiry: "",
                    strike_rupees: "",
                    option_side: "",
                },
            )
            .expect("ok"),
        )
        .expect("kept");
        assert_eq!(dhan_eq.kind, Kind::Equity);
    }

    // ---------------------------------------------------------------------
    // The equity-listing gate.
    // ---------------------------------------------------------------------

    /// A Dhan `SEGMENT=E`, `INSTRUMENT=EQUITY` row, which is what every
    /// listing on the equity segment is — share, bond or fund alike.
    ///
    /// `series` is Dhan's own `SERIES` column, which is the NSE board series
    /// and the only column this gate reads. D-0025.
    fn dhan_cash<'a>(ticker: &'a str, series: &'a str, isin: &'a str) -> MasterRow<'a> {
        MasterRow {
            vendor_id: "1333",
            exchange: "NSE",
            segment: "E",
            underlying: ticker,
            trading_symbol: "CHOLAMANDALAM IN & FIN CO",
            instrument_type: "EQUITY",
            listing_class: series,
            isin,
            expiry: "",
            strike_rupees: "",
            option_side: "",
        }
    }

    /// A Groww `CASH`/`EQ` row on a given NSE series.
    fn groww_cash<'a>(ticker: &'a str, series: &'a str, isin: &'a str) -> MasterRow<'a> {
        MasterRow {
            vendor_id: "1333",
            underlying: "",
            trading_symbol: ticker,
            listing_class: series,
            isin,
            ..row("NSE", "CASH", ticker, "EQ", "", "")
        }
    }

    #[test]
    fn the_cholafin_bond_is_declined_and_the_cholafin_share_is_kept() {
        // THE COLLISION THIS GATE EXISTS FOR. Both rows are verbatim from the
        // real Dhan master; both are NSE/E/EQUITY with ticker CHOLAFIN. The
        // BOND is at line 167146 and the SHARE at 171414, so the bond comes
        // FIRST and an insert-if-absent merge resolved CHOLAFIN to a 7.5% NCD
        // and took its tick size of 5.0 instead of the share's 10.0 --
        // silently, and dependent on nothing but file order. The bond's real
        // SERIES is `D1`; the share's is `EQ`.
        let bond = dhan_cash("CHOLAFIN", "D1", "INE121A08PJ0");
        assert_eq!(
            decode_master_row(Vendor::Dhan, bond).expect("ok").skip(),
            Some(Skip::NotEquityListing),
            "the NCD must never take the CHOLAFIN ticker"
        );

        let share = dhan_cash("CHOLAFIN", "EQ", "INE121A01024");
        let l = listing(decode_master_row(Vendor::Dhan, share).expect("ok")).expect("kept");
        assert_eq!(l.key.underlying.as_str(), "CHOLAFIN");
        assert_eq!(l.key.kind, Kind::Equity);
        assert_eq!(
            l.isin.map(|i| i.to_string()).as_deref(),
            Some("INE121A01024"),
            "the share's own ISIN travels beside the key"
        );
    }

    #[test]
    fn the_other_two_measured_ticker_captures_are_declined_too() {
        // MOTHERSON's NCD (series D1) and ELECTCAST's warrant (series W1) are
        // the other two rows that captured a live ticker, and the share of each
        // name is on series EQ. All three are NIFTY Total Market members. Every
        // series below is verbatim from the real Dhan master.
        for (ticker, series, isin) in [
            ("MOTHERSON", "D1", "INE775A08105"),
            ("ELECTCAST", "W1", "INE086A13016"),
        ] {
            assert_eq!(
                decode_master_row(Vendor::Dhan, dhan_cash(ticker, series, isin))
                    .expect("ok")
                    .skip(),
                Some(Skip::NotEquityListing),
                "{ticker} series {series} must be declined"
            );
        }
        for (ticker, isin) in [("MOTHERSON", "INE775A01035"), ("ELECTCAST", "INE086A01029")] {
            let l = listing(
                decode_master_row(Vendor::Dhan, dhan_cash(ticker, "EQ", isin)).expect("ok"),
            )
            .expect("kept");
            assert_eq!(l.isin.map(|i| i.to_string()).as_deref(), Some(isin));
        }
    }

    #[test]
    fn dhans_class_column_is_trimmed_before_it_is_read() {
        // The column is whitespace padded. Reading it untrimmed would decline
        // every genuine share in the file -- a total loss reported as a
        // routine skip, which is the failure this repository has already had
        // once.
        for padded in ["   EQ   ", " EQ", "EQ ", "EQ", "  BE  ", "BE"] {
            let l = listing(
                decode_master_row(Vendor::Dhan, dhan_cash("RELIANCE", padded, REAL_ISIN))
                    .expect("ok"),
            )
            .expect("kept");
            assert_eq!(l.key.kind, Kind::Equity, "{padded:?} must be kept");
        }
    }

    #[test]
    fn a_mutual_fund_plan_is_declined_from_both_vendors_on_one_series_alphabet() {
        // Dhan files 54 open-ended fund plans as INSTRUMENT_TYPE=ETF while its
        // OWN series column says MF; Groww carries 29 of the same ISINs under
        // series=MF and declines them. Reading the paper class kept all 54 as
        // equities -- the exact category Skip::NotEquityListing exists to
        // remove. Reading the series declines them from BOTH vendors, which is
        // the whole point of D-0025.
        for (vendor, row) in [
            (Vendor::Dhan, dhan_cash("FISTIPD3GP", "MF", "INF090I01VS3")),
            (
                Vendor::Groww,
                groww_cash("FISTIPD3GP", "MF", "INF090I01VS3"),
            ),
        ] {
            // The vendor name is bound rather than called inside the failure
            // message: a call there is a region that only runs when the
            // assertion FAILS, so it can never be covered by a passing test.
            let who = vendor.as_str();
            assert_eq!(
                decode_master_row(vendor, row).expect("ok").skip(),
                Some(Skip::NotEquityListing),
                "{who} must decline the fund plan"
            );
        }
        // And a genuine exchange-traded fund on the EQ series is still kept --
        // HDFCLIQUID carries an INF issuer prefix too, so "INF means a fund"
        // would have been the wrong rule.
        let etf = listing(
            decode_master_row(Vendor::Dhan, dhan_cash("HDFCLIQUID", "EQ", "INF179KC1JG3"))
                .expect("ok"),
        )
        .expect("kept");
        assert_eq!(etf.key.kind, Kind::Equity);
    }

    #[test]
    fn the_surveillance_and_partly_paid_equity_series_are_kept_not_called_debt() {
        // BZ (25 Groww / 38 Dhan rows), IT (2/2), SZ (1/2) and E1 (2/3) are
        // trade-for-trade, surveillance and partly-paid EQUITY. Declining them
        // under "not an equity listing" was false about 30 real shares.
        // RAJESHEXPO/INE343B01030 and HMT/INE262A01018 are verbatim from the
        // masters; the NSDL security-type digits of both are `01`, ordinary
        // equity, against `08` for the CHOLAFIN NCD.
        for series in ["BZ", "IT", "SZ", "E1"] {
            for (vendor, r) in [
                (
                    Vendor::Groww,
                    groww_cash("RAJESHEXPO", series, "INE343B01030"),
                ),
                (
                    Vendor::Dhan,
                    dhan_cash("RAJESHEXPO", series, "INE343B01030"),
                ),
            ] {
                let who = vendor.as_str();
                let l = listing(decode_master_row(vendor, r).expect("ok")).expect("kept");
                assert_eq!(
                    l.key.kind,
                    Kind::Equity,
                    "series {series} is equity at {who}"
                );
            }
        }
    }

    #[test]
    fn an_unrecognised_series_is_its_own_loud_reason_never_a_bond() {
        // THE FAILURE THIS VARIANT EXISTS FOR. Both arms of the gate used to
        // end in `_ => NotEquity`, so renaming the equity series EQ to EQX on
        // the real Dhan master silently dropped 2,438 shares -- every F&O
        // underlying among them -- while the report printed `ok` and exit 0,
        // and the only trace was one bond counter rising.
        for code in ["EQX", "XX", "eq", "es", "ETF", "ES", "DEB", "", "  "] {
            assert_eq!(
                decode_master_row(Vendor::Dhan, dhan_cash("RELIANCE", code, REAL_ISIN))
                    .expect("ok")
                    .skip(),
                Some(Skip::UnrecognisedListingClass),
                "{code:?} is not a series this engine has measured"
            );
        }
        // It is a DECLINE and not an error, because NSE mints debt series at
        // will -- but never the same decline as a bond.
        assert_ne!(Skip::UnrecognisedListingClass, Skip::NotEquityListing);
        assert!(!Skip::UnrecognisedListingClass.is_routine());
        assert!(Skip::NotEquityListing.is_routine());
        assert!(Skip::SmeBoard.is_routine());
    }

    #[test]
    fn every_measured_debt_and_fund_class_is_declined_by_name() {
        // The 7,137 rows that are on the equity segment and are not equity.
        // Both vendors, one NSE series alphabet.
        for series in [
            "N0", "N1", "SG", "GS", "MF", "IV", "Y1", "Z9", "AK", "D1", "W1", "TB", "GB", "RR",
            "P1", "SF", "ZZ",
        ] {
            // THE TWO MASTERS THAT CARRY A BOARD SERIES, not every mastered
            // vendor. This alphabet is read from a `series` column, and the
            // third broker's dump has none — twelve columns, no series and no
            // ISIN among them. Asking it to decline a series it never publishes
            // is asking a question its master cannot answer.
            for vendor in [Vendor::Groww, Vendor::Dhan] {
                let r = match vendor {
                    Vendor::Groww => groww_cash("SOMEBOND", series, REAL_ISIN),
                    _ => dhan_cash("SOMEBOND", series, REAL_ISIN),
                };
                let who = vendor.as_str();
                assert_eq!(
                    decode_master_row(vendor, r).expect("ok").skip(),
                    Some(Skip::NotEquityListing),
                    "series {series:?} must be declined at {who}"
                );
            }
        }
    }

    #[test]
    fn every_series_code_survives_the_open_addressed_table_it_moved_into() {
        // I-41. `board_of` probes three `MemberIndex` tables instead of
        // binary-searching three arrays. A collision that silently dropped a
        // member would not fail to compile and would not look wrong -- it would
        // reclassify a measured bond as `Unrecognised`, which is a LOUD decline
        // that degrades the run, so an entire real master would start reporting
        // an alphabet change that never happened.
        //
        // So every code in every array is asserted to reach its own verdict
        // through the real entry point.
        for code in EQUITY_BOARD_SERIES {
            assert_eq!(board_of(code), EquityVerdict::MainBoard, "{code}");
        }
        for code in SME_BOARD_SERIES {
            assert_eq!(board_of(code), EquityVerdict::Sme, "{code}");
        }
        for code in NON_EQUITY_SERIES {
            assert_eq!(board_of(code), EquityVerdict::NotEquity, "{code}");
        }

        // The counts match the arrays, so nothing was overwritten on the way in.
        assert_eq!(EQUITY_BOARD_INDEX.len(), EQUITY_BOARD_SERIES.len());
        assert_eq!(SME_BOARD_INDEX.len(), SME_BOARD_SERIES.len());
        assert_eq!(NON_EQUITY_INDEX.len(), NON_EQUITY_SERIES.len());

        // And a code in none of them is still its own reason, never a bond.
        for absent in ["ZZ9", "QQ", "", "  ", "eq", "N", "XX"] {
            assert_eq!(
                board_of(absent),
                EquityVerdict::Unrecognised,
                "{absent:?} is unseen, not non-equity"
            );
        }

        // Trimming still happens before the probe -- Dhan pads this column.
        assert_eq!(board_of("   EQ   "), EquityVerdict::MainBoard);
    }

    #[test]
    fn the_measured_series_tables_are_sorted_disjoint_and_complete() {
        // Sortedness no longer carries a search — `board_of` probes three
        // `MemberIndex` tables — but a strictly increasing walk is still how a
        // duplicate is caught, and a duplicate is what would make one code
        // appear in a table twice and the census counts disagree with the
        // list.
        for (name, list) in [
            ("EQUITY_BOARD_SERIES", EQUITY_BOARD_SERIES.as_slice()),
            ("SME_BOARD_SERIES", SME_BOARD_SERIES.as_slice()),
            ("NON_EQUITY_SERIES", NON_EQUITY_SERIES.as_slice()),
        ] {
            for w in list.windows(2) {
                assert!(w[0] < w[1], "{name} is unsorted or not unique at {w:?}");
            }
            for code in list {
                assert!(!code.is_empty(), "{name} holds an empty code");
            }
        }
        // A code in two tables would make the verdict depend on the order the
        // tables happen to be searched in.
        for a in EQUITY_BOARD_SERIES {
            assert!(!SME_BOARD_SERIES.contains(&a));
            assert!(!NON_EQUITY_SERIES.contains(&a));
        }
        for a in SME_BOARD_SERIES {
            assert!(!NON_EQUITY_SERIES.contains(&a));
        }
        // 128 distinct codes were measured across both masters.
        assert_eq!(
            EQUITY_BOARD_SERIES.len() + SME_BOARD_SERIES.len() + NON_EQUITY_SERIES.len(),
            128
        );
    }

    /// I-38. The three series tables answer in a bounded number of probes, and
    /// the bound is a NUMBER rather than the word "small".
    ///
    /// `docs/07-o1-architecture.md` layer 4: a layer is built when a test
    /// asserts the bound as a number. The section records why — the first
    /// open-addressed table written in this workspace measured **14** probes,
    /// worse than the `binary_search` it replaced and still O(1) by
    /// definition, and only its own test caught that.
    ///
    /// Asserted at the same `<= 8` the universe tables are held to, so one
    /// number governs every `MemberIndex` in the crate, and PRINTED so a
    /// regression that stays inside the bound is still visible in the log.
    #[test]
    fn the_series_tables_probe_in_bounded_time() {
        fn worst<const N: usize>(idx: &MemberIndex<N>, members: &[&str]) -> usize {
            let mut worst = 0;
            for m in members {
                // Walks the table the way `contains` does and COUNTS the
                // steps, rather than trusting the shape of the code. The start
                // index comes from `universe::mask` itself rather than from a
                // copy of it: a copy is free to disagree with the thing under
                // test, and would then measure a probe nobody performs.
                let mut at = crate::universe::mask(crate::universe::fnv1a(m), N);
                let mut steps = 1;
                while let Some(held) = idx.slots[at] {
                    if held == *m {
                        break;
                    }
                    at = (at + 1) & (N - 1);
                    steps += 1;
                }
                worst = worst.max(steps);
            }
            worst
        }
        let eq = worst(&EQUITY_BOARD_INDEX, &EQUITY_BOARD_SERIES);
        let sme = worst(&SME_BOARD_INDEX, &SME_BOARD_SERIES);
        let non = worst(&NON_EQUITY_INDEX, &NON_EQUITY_SERIES);
        assert!(
            eq <= 8,
            "6 in 16 slots must probe at most 8 times, got {eq}"
        );
        assert!(
            sme <= 8,
            "2 in 8 slots must probe at most 8 times, got {sme}"
        );
        assert!(
            non <= 8,
            "120 in 512 slots must probe at most 8 times, got {non}"
        );
        println!("worst probe: equity {eq}, sme {sme}, non-equity {non}");
        // And the tables answer the questions `board_of` asks of them, which
        // is the property the probe bound is only worth having for.
        for code in EQUITY_BOARD_SERIES {
            assert!(EQUITY_BOARD_INDEX.contains(code), "{code} missing");
        }
        for code in SME_BOARD_SERIES {
            assert!(SME_BOARD_INDEX.contains(code), "{code} missing");
        }
        for code in NON_EQUITY_SERIES {
            assert!(NON_EQUITY_INDEX.contains(code), "{code} missing");
        }
        // A code in no table is `false` in all three, which is what makes
        // `Unrecognised` reachable at all.
        for absent in ["QQ", "", "  ", "EQUITY"] {
            assert!(!EQUITY_BOARD_INDEX.contains(absent));
            assert!(!SME_BOARD_INDEX.contains(absent));
            assert!(!NON_EQUITY_INDEX.contains(absent));
        }
    }

    #[test]
    fn the_equity_board_is_kept_and_the_sme_board_is_declined_separately() {
        // EQ and BE are the equity board; SM and ST are the SME board. The SME
        // rows get their OWN reason so the decision stays visible: an SME
        // listing IS a share, it is simply not in any universe the engine
        // ranks over.
        for series in ["EQ", "BE"] {
            let r = MasterRow {
                vendor_id: "1333",
                listing_class: series,
                ..row("NSE", "CASH", "RELIANCE", "EQ", "", "")
            };
            assert_eq!(
                listing(groww(r).expect("ok")).expect("kept").key.kind,
                Kind::Equity,
                "series {series} is the equity board"
            );
        }
        // Symmetrically at BOTH vendors, since D-0025 -- 558 rows at Groww and
        // 559 at Dhan, the same paper by ISIN. Before it, Dhan kept every one
        // of them because its paper class calls an SME share `ES`.
        for series in ["SM", "ST"] {
            for (vendor, r) in [
                (Vendor::Groww, groww_cash("SOMESME", series, REAL_ISIN)),
                (Vendor::Dhan, dhan_cash("SOMESME", series, REAL_ISIN)),
            ] {
                let who = vendor.as_str();
                assert_eq!(
                    decode_master_row(vendor, r).expect("ok").skip(),
                    Some(Skip::SmeBoard),
                    "series {series} is the SME board at {who}, and says so"
                );
            }
        }
    }

    #[test]
    fn a_declined_row_carries_its_isin_as_evidence_for_the_cross_check() {
        // A merge that only compares what both vendors KEPT cannot see an
        // ELIGIBILITY disagreement. The ISIN is what makes one visible, so it
        // travels with the decline -- leniently, because the declined row is
        // declined either way.
        let d = decode_master_row(Vendor::Dhan, dhan_cash("CHOLAFIN", "D1", "INE121A08PJ0"))
            .expect("ok");
        assert_eq!(
            d,
            Decoded::Skipped(Declined {
                reason: Skip::NotEquityListing,
                isin: Isin::new("INE121A08PJ0").ok(),
            })
        );
        // An unparseable ISIN on a declined row yields no evidence and no
        // error: the row is declined and counted regardless.
        let sdl = decode_master_row(Vendor::Dhan, dhan_cash("61GJ28", "SG", "IN1520250085"))
            .expect("a declined row must never fail on its own ISIN");
        assert_eq!(
            sdl,
            Decoded::Skipped(Declined {
                reason: Skip::NotEquityListing,
                isin: None,
            })
        );
    }

    #[test]
    fn an_index_row_survives_the_gate_from_both_vendors() {
        // THE ROW THAT MUST NOT BE GATED. An index has no series at all --
        // Groww leaves the column empty, Dhan writes `NA` -- so a gate applied
        // before the instrument type is known deletes NIFTY and BANKNIFTY,
        // which is every instrument the engine exists to sweep.
        for sym in ["NIFTY", "BANKNIFTY"] {
            let g = MasterRow {
                vendor_id: "1333",
                underlying: "",
                trading_symbol: sym,
                listing_class: "",
                isin: sym,
                ..row("NSE", "CASH", sym, "IDX", "", "")
            };
            let gl = listing(groww(g).expect("ok")).expect("kept");
            assert!(gl.key.is_sweepable(), "Groww {sym} must survive the gate");
            assert_eq!(gl.isin, None, "an index has no ISIN, and none is invented");

            let d = MasterRow {
                vendor_id: "1333",
                exchange: "NSE",
                segment: "I",
                underlying: sym,
                trading_symbol: sym,
                instrument_type: "INDEX",
                listing_class: "NA",
                isin: "NA",
                expiry: "0001-01-01",
                strike_rupees: "",
                option_side: "XX",
            };
            let dl = listing(decode_master_row(Vendor::Dhan, d).expect("ok")).expect("kept");
            assert!(dl.key.is_sweepable(), "Dhan {sym} must survive the gate");
            assert_eq!(
                dl.isin, None,
                "`NA` is not an ISIN and is not parsed as one"
            );
        }
    }

    #[test]
    fn the_gate_never_sees_a_derivative_row() {
        // A live derivative carries no series either. It is declined for being
        // live, and the reason must say so rather than saying "not equity".
        let r = MasterRow {
            vendor_id: "1333",
            listing_class: "",
            isin: "",
            ..row("NSE", "FNO", "NIFTY", "FUT", "2026-08-04", "")
        };
        assert_eq!(
            groww(r).expect("ok").skip(),
            Some(Skip::LiveContract),
            "the reason must be the true one"
        );
    }

    #[test]
    fn a_kept_equity_must_carry_a_parseable_isin() {
        // Every one of the main-board rows in either master has one, so a
        // missing or malformed value means the row is not what it claims.
        for bad in ["", "NA", "INE002A01019", "INE002A0101"] {
            let r = MasterRow {
                vendor_id: "1333",
                isin: bad,
                ..row("NSE", "CASH", "RELIANCE", "EQ", "", "")
            };
            assert_eq!(
                groww(r),
                Err(InstrumentError::Malformed),
                "{bad:?} must be loud, never a quiet None"
            );
        }
    }

    #[test]
    fn the_sdl_with_the_bad_check_digit_never_reaches_the_isin_parse() {
        // IN1520250085 is the one row in either master whose check digit does
        // not verify. It is a state development loan on series `SG`, so the
        // gate declines it FIRST -- which is why the ISIN is parsed after the
        // gate and not before. Order is the whole content of this test.
        assert_eq!(
            decode_master_row(Vendor::Dhan, dhan_cash("61GJ28", "SG", "IN1520250085"))
                .expect("ok")
                .skip(),
            Some(Skip::NotEquityListing)
        );
        // And it really would have been refused, had it got that far.
        assert!(Isin::new("IN1520250085").is_err());
    }

    // ---------------------------------------------------------------------
    // The series suffix.
    // ---------------------------------------------------------------------

    #[test]
    fn a_series_suffix_is_offered_as_a_candidate_never_applied() {
        // Groww leaks internal_trading_symbol into trading_symbol on 209 of
        // the 4,080 shared ISINs. The decoder offers the stripped identity; it
        // does not adopt it, because only a second vendor's ISIN can confirm
        // that BLUECHIP-BE and BLUECHIP are one instrument.
        let r = MasterRow {
            vendor_id: "1333",
            underlying: "",
            trading_symbol: "BLUECHIP-BE",
            listing_class: "BE",
            isin: "INE657B01025",
            ..row("NSE", "CASH", "BLUECHIP-BE", "EQ", "", "")
        };
        let l = listing(groww(r).expect("ok")).expect("kept");
        assert_eq!(
            l.key.underlying.as_str(),
            "BLUECHIP-BE",
            "the key is what the vendor said, unchanged"
        );
        assert_eq!(
            l.unsuffixed.map(|k| k.underlying.as_str().to_owned()),
            Some("BLUECHIP".to_owned()),
            "and the stripped form is offered beside it"
        );
    }

    #[test]
    fn a_dash_that_is_not_the_rows_own_series_is_never_stripped() {
        // BAJAJ-AUTO is a real ticker ending in a dash. Stripping blind would
        // manufacture the collision Symbol::new refuses to manufacture.
        for (ticker, series) in [
            ("BAJAJ-AUTO", "EQ"),
            ("NAM-INDIA", "EQ"),
            ("RELIANCE", "EQ"),
            ("LOWVOL-EQ", "BE"),
        ] {
            let r = MasterRow {
                vendor_id: "1333",
                underlying: "",
                trading_symbol: ticker,
                listing_class: series,
                ..row("NSE", "CASH", ticker, "EQ", "", "")
            };
            assert_eq!(
                listing(groww(r).expect("ok")).expect("kept").unsuffixed,
                None,
                "{ticker} under series {series} must not be stripped"
            );
        }
    }

    #[test]
    fn stripping_to_nothing_is_an_error_rather_than_a_silent_no_op() {
        let r = MasterRow {
            vendor_id: "1333",
            underlying: "",
            trading_symbol: "-EQ",
            listing_class: "EQ",
            ..row("NSE", "CASH", "-EQ", "EQ", "", "")
        };
        assert_eq!(groww(r), Err(InstrumentError::Malformed));
    }

    #[test]
    fn nothing_but_a_cash_listing_is_offered_a_stripped_key() {
        // An index has no series, so there is nothing to strip and no
        // candidate to offer -- even when its ticker happens to end in a dash
        // and a word.
        let r = MasterRow {
            vendor_id: "1333",
            underlying: "",
            trading_symbol: "NIFTY-IDX",
            listing_class: "IDX",
            isin: "NIFTY-IDX",
            ..row("NSE", "CASH", "NIFTY-IDX", "IDX", "", "")
        };
        assert_eq!(
            listing(groww(r).expect("ok")).expect("kept").unsuffixed,
            None
        );
    }

    // ---------------------------------------------------------------------
    // The vendor tables.
    // ---------------------------------------------------------------------

    #[test]
    fn every_skip_reason_is_distinct_and_says_what_it_declined() {
        let all = [
            Skip::NoVendorId,
            Skip::ForeignExchange,
            Skip::UnrecognisedExchange,
            Skip::TestInstrument,
            Skip::ForeignSegment,
            Skip::LiveContract,
            Skip::NotEquityListing,
            Skip::SmeBoard,
            Skip::UnrecognisedListingClass,
        ];
        let rendered: Vec<&str> = all.iter().map(|s| s.reason()).collect();
        for (i, a) in rendered.iter().enumerate() {
            assert!(!a.is_empty(), "reason {i} is empty");
            for (j, b) in rendered.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "reasons {i} and {j} are the same string");
                }
            }
        }
    }

    #[test]
    fn each_vendor_names_its_own_columns_and_no_two_agree() {
        let g = Vendor::Groww.master_columns();
        let d = Vendor::Dhan.master_columns();
        // Both read the NSE board series, under each vendor's own spelling.
        // Dhan's `INSTRUMENT_TYPE` is a vendor-minted paper class and is
        // deliberately not read at all -- D-0025.
        assert_eq!(g.listing_class, "series");
        assert_eq!(d.listing_class, "SERIES");
        assert_eq!(d.instrument_type, "INSTRUMENT", "not INSTRUMENT_TYPE");
        assert!(
            [
                g.exchange,
                g.segment,
                g.underlying,
                g.trading_symbol,
                g.instrument_type,
                g.listing_class,
                g.isin,
                g.expiry,
                g.strike,
                d.exchange,
                d.segment,
                d.underlying,
                d.trading_symbol,
                d.instrument_type,
                d.listing_class,
                d.isin,
                d.expiry,
                d.strike,
            ]
            .iter()
            .all(|n| *n != "INSTRUMENT_TYPE"),
            "the measurably-wrong column must not be read by any field"
        );
        assert_eq!(g.isin, "isin");
        assert_eq!(d.isin, "ISIN");
        assert_eq!(g.option_side, None, "Groww types CE and PE directly");
        assert_eq!(d.option_side, Some("OPTION_TYPE"));
        assert_ne!(g, d);
    }

    #[test]
    fn a_vendor_set_is_a_set_and_every_vendor_has_its_own_bit() {
        let mut s = VendorSet::EMPTY;
        assert!(s.is_empty());
        for v in Vendor::ALL {
            assert!(!s.contains(v));
            s = s.with(v);
            assert!(s.contains(v));
        }
        assert!(!s.is_empty());
        assert_eq!(s, s.with(Vendor::Groww), "adding twice is adding once");
        assert_eq!(VendorSet::default(), VendorSet::EMPTY);
        // One bit each, or two vendors would be indistinguishable.
        let only_dhan = VendorSet::EMPTY.with(Vendor::Dhan);
        assert!(only_dhan.contains(Vendor::Dhan));
        assert!(!only_dhan.contains(Vendor::Groww));
        for vendor in Vendor::ALL {
            let single = VendorSet::EMPTY.with(vendor);
            for other in Vendor::ALL {
                assert_eq!(single.contains(other), vendor == other);
            }
        }
    }

    #[test]
    fn archive_vendors_have_names_but_cannot_supply_master_metadata() {
        for (vendor, name, master) in [
            (Vendor::Groww, "groww", "groww_instruments.csv"),
            (Vendor::Dhan, "dhan", "dhan_scrip.csv"),
            (Vendor::TrueData, "truedata", "truedata_instruments.csv"),
            (Vendor::Gdfl, "gdfl", "gdfl_instruments.csv"),
            (Vendor::Zerodha, "zerodha", "zerodha_instruments.csv"),
        ] {
            assert_eq!(
                vendor.as_str(),
                name,
                "persisted vendor namespace is stable"
            );
            assert_eq!(vendor.master_file(), master);
            assert_eq!(
                vendor.publishes_master(),
                Vendor::MASTERED.contains(&vendor)
            );
        }
        for archive in [Vendor::TrueData, Vendor::Gdfl] {
            let columns = archive.master_columns();
            for name in [
                columns.vendor_id,
                columns.exchange,
                columns.segment,
                columns.underlying,
                columns.trading_symbol,
                columns.instrument_type,
                columns.listing_class,
                columns.isin,
                columns.expiry,
                columns.strike,
            ] {
                assert!(name.is_empty(), "archive must not invent a master column");
            }
            assert_eq!(columns.option_side, None);
            assert_eq!(archive.index_segment_word(), None);
            for code in ["", "NSE", "INDEX", "EQ", "FUT", "CE", "PE"] {
                assert_eq!(
                    archive.segment_of(code).err(),
                    Some(InstrumentError::Malformed)
                );
                assert_eq!(archive.type_of(code, "CE"), Err(InstrumentError::Malformed));
            }
        }
        assert_eq!(
            Vendor::Zerodha.master_columns().vendor_id,
            "instrument_token"
        );
        for code in ["EQ", "FUT", "CE", "PE"] {
            assert_eq!(Vendor::Zerodha.type_of(code, "ignored"), Ok(Some(code)));
        }
        assert_eq!(
            Vendor::Zerodha.type_of("IDX", ""),
            Err(InstrumentError::Malformed)
        );
    }

    #[test]
    fn vendor_ids_refuse_missing_and_oversized_values_before_a_listing_is_kept() {
        for raw in ["", "   ", "\t\n"] {
            assert_eq!(VendorId::new(raw), None);
            let mut input = row("NSE", "CASH", "RELIANCE", "EQ", "", "");
            input.vendor_id = raw;
            assert_eq!(
                groww(input).expect("explicit decline").skip(),
                Some(Skip::NoVendorId)
            );
        }
        let limit = "X".repeat(VENDOR_ID_CAPACITY);
        assert_eq!(VendorId::new(&limit).expect("at capacity").as_str(), limit);
        assert_eq!(VendorId::new(&format!("{limit}X")), None);
        let value = VendorId::new("  A-19_é  ").expect("opaque UTF-8 identifier");
        assert_eq!(value.as_str(), "A-19_é");
        assert_eq!(value.to_string(), "A-19_é");
        assert_eq!(format!("{value:?}"), "VendorId(\"A-19_é\")");
    }

    // =======================================================================
    // The field-width gate — D-0033
    // =======================================================================

    /// The row from the defect: a legitimate `underlying`, and one enormous
    /// field that is scanned but never becomes the identity.
    fn wide_row<'a>(field: &str, wide: &'a str) -> MasterRow<'a> {
        let mut r = MasterRow {
            vendor_id: "1333",
            exchange: "NSE",
            segment: "CASH",
            underlying: "RELIANCE",
            trading_symbol: "RELIANCE",
            instrument_type: "EQ",
            listing_class: "EQ",
            isin: REAL_ISIN,
            expiry: "",
            strike_rupees: "",
            option_side: "",
        };
        match field {
            "exchange" => r.exchange = wide,
            "segment" => r.segment = wide,
            "underlying" => r.underlying = wide,
            "trading_symbol" => r.trading_symbol = wide,
            "instrument_type" => r.instrument_type = wide,
            "listing_class" => r.listing_class = wide,
            "isin" => r.isin = wide,
            "expiry" => r.expiry = wide,
            "strike_rupees" => r.strike_rupees = wide,
            _ => r.option_side = wide,
        }
        r
    }

    /// Every field name `over_wide` can report, in struct order.
    const FIELD_NAMES: [&str; 10] = [
        "exchange",
        "segment",
        "underlying",
        "trading_symbol",
        "instrument_type",
        "listing_class",
        "isin",
        "expiry",
        "strike_rupees",
        "option_side",
    ];

    #[test]
    fn an_over_wide_field_is_refused_whichever_field_it_is() {
        // THE DEFECT THIS PINS. `trading_symbol` only becomes the identity when
        // `underlying` is empty, so a 4 MiB `trading_symbol` beside a populated
        // `underlying` was scanned twice by TEST_MARKERS and then ACCEPTED AND
        // STORED -- the width guard in `Symbol::new` never saw it. Every field
        // is checked here, not just the two the scan reads, because the next
        // reader added below the gate must not have to remember to add a bound.
        // Bound rather than computed inside a failure message: an expression
        // there is a region only a FAILING assertion reaches, so no passing
        // test can cover it.
        let over = MAX_FIELD_BYTES + 1;
        let wide = "X".repeat(over);
        for name in FIELD_NAMES {
            let row = wide_row(name, &wide);
            assert_eq!(
                row.over_wide(),
                Some((name, over)),
                "{name} was not reported"
            );
            assert_eq!(
                decode_master_row(Vendor::Groww, row),
                Err(InstrumentError::FieldTooWide {
                    field: name,
                    len: over,
                }),
                "{name} at {over} bytes was not refused"
            );
        }
    }

    #[test]
    fn the_bound_is_the_first_byte_that_is_too_many_and_not_one_before() {
        // An off-by-one here refuses rows that decode correctly today, so the
        // exact boundary is pinned rather than the general shape.
        let at = "X".repeat(MAX_FIELD_BYTES);
        let over = "X".repeat(MAX_FIELD_BYTES + 1);
        assert_eq!(wide_row("expiry", &at).over_wide(), None);
        assert_eq!(
            wide_row("expiry", &over).over_wide(),
            Some(("expiry", MAX_FIELD_BYTES + 1))
        );
        // 64 bytes in `expiry` is still refused -- but for its CONTENT, by the
        // expiry parser, which is the bound that belongs there. The width gate
        // is not a substitute for any of the parsers below it.
        let mut r = wide_row("expiry", &at);
        r.instrument_type = "FUT";
        assert_eq!(
            decode_master_row(Vendor::Groww, r),
            Err(InstrumentError::Malformed)
        );
    }

    #[test]
    fn a_row_of_ordinary_width_passes_the_gate_untouched() {
        // The widest value in either real master, in the column it was measured
        // in: 28 bytes. If this ever fails the bound has been set below what
        // the vendors actually emit.
        const WIDEST_MEASURED: &str = "NIFTYNXT50-Aug2026-101500-CE";
        assert_eq!(WIDEST_MEASURED.len(), 28);
        assert!(WIDEST_MEASURED.len() <= MAX_FIELD_BYTES);
        let row = wide_row("trading_symbol", WIDEST_MEASURED);
        assert_eq!(row.over_wide(), None);
        assert_eq!(
            kept(decode_master_row(Vendor::Groww, row).expect("decodes"))
                .map(|k| k.underlying.as_str().to_owned()),
            Some("RELIANCE".to_owned())
        );
    }

    #[test]
    fn the_test_marker_scan_still_declines_a_real_test_listing() {
        // The gate runs BEFORE the scan, so this proves the gate did not
        // shadow it. `031NSETEST` is verbatim from the primary broker's master.
        let row = wide_row("underlying", "031NSETEST");
        assert_eq!(
            decode_master_row(Vendor::Groww, row).map(Decoded::skip),
            Ok(Some(Skip::TestInstrument))
        );
    }

    #[test]
    fn judges_the_paper_answers_both_ways_and_not_one_constant() {
        // Every variant, asserted in the direction it belongs. Nothing here
        // pinned the return value before, so the whole predicate could be
        // replaced by `true` or by `false` and the suite stayed green -- and
        // this is the predicate that decides whether a cross-vendor
        // disagreement is a real conflict about the security or just two
        // correct answers about two different venues.
        for about_the_paper in [
            Skip::NotEquityListing,
            Skip::SmeBoard,
            Skip::UnrecognisedListingClass,
        ] {
            assert!(
                about_the_paper.judges_the_paper(),
                "{about_the_paper:?} is a verdict about the security itself"
            );
        }
        for about_the_venue in [
            Skip::ForeignExchange,
            Skip::TestInstrument,
            Skip::ForeignSegment,
            Skip::LiveContract,
        ] {
            assert!(
                !about_the_venue.judges_the_paper(),
                "{about_the_venue:?} says where the row was found, not what it is"
            );
        }
    }

    #[test]
    fn a_groww_call_is_not_folded_into_the_put_arm() {
        // The inner match ends `_ => "PE"`, so deleting the "CE" arm silently
        // relabels every call option as a put. Both sides are asserted here
        // because only the pair distinguishes the arm from the fallback.
        assert_eq!(Vendor::Groww.type_of("CE", ""), Ok(Some("CE")));
        assert_eq!(Vendor::Groww.type_of("PE", ""), Ok(Some("PE")));
        assert_eq!(Vendor::Groww.type_of("FUT", ""), Ok(Some("FUT")));
    }

    #[test]
    fn each_date_component_is_width_checked_on_its_own() {
        // The width guard is three checks joined by `||`, and every previous
        // test got more than one of them wrong at once -- which any of `&&`,
        // `||` or a mixture would refuse identically. These get exactly ONE
        // component wrong, so each `||` has to carry the refusal alone.
        assert!(parse_expiry("2026-08-04").is_ok(), "the well-formed case");

        // Month one character short; year and day are correct widths, and
        // the whole date is otherwise a real one.
        assert_eq!(parse_expiry("2026-8-04"), Err(InstrumentError::Malformed));
        // Day one character short.
        assert_eq!(parse_expiry("2026-08-4"), Err(InstrumentError::Malformed));
        // Year two characters short.
        assert_eq!(parse_expiry("26-08-04"), Err(InstrumentError::Malformed));
    }
}

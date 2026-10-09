//! One sourced table: what the F&O market, NSE, the vendors and the store each
//! call an index, and the dated symbol changes NSE published. D-3332.
//!
//! # Why one table
//!
//! Before this module only `NIFTY 50 → NIFTY` and `NIFTY BANK → BANKNIFTY`
//! were aliased (`core::vendor::Vendor::index_alias`). So `/spot.json` looked
//! for `INDEX/FINNIFTY`, a key no feed files, and found nothing; NIFTYIT and
//! NIFTYFPI options were resolved as SHARES because the index set was the five
//! names of `crate::universe::FNO_INDEX_UNDERLYINGS`; `/db` showed two of the
//! five F&O index underlyings in its F&O universe; and nothing knew that
//! ZOMATO became ETERNAL, so a search for the old name found nothing (DB-040,
//! DB-071, DB-G25, DB-G53, DB-G54).
//!
//! Every name for one index meets in one [`IndexRow`]: the derivatives symbol
//! (`FINNIFTY`), NSE's index name (`NIFTY FINANCIAL SERVICES`), NSE's
//! `indexSymbol`, which is also the GDFL file stem (`NIFTY FIN SERVICE`), the
//! Kite tradingsymbol (`NIFTY FIN SERVICE`) and the store key every feed files
//! the series under (`NIFTYFINSERVICE`). Every dated symbol change meets in one
//! [`SymbolChange`]: old symbol, new symbol, the day the new one applies.
//!
//! # Where each fact comes from
//!
//! `docs/00-charter.md`, "Index aliases and symbol changes — read 7 Oct 2026
//! (D-3332)" ([`CHARTER`]): NSE's derivatives product pages, NSE's own index
//! list (`allIndices`) and NSE's `symbolchange.csv`, each with the sha256 of the
//! bytes read. The Kite tradingsymbol column is a measurement of the vendor's
//! instrument dump, which the charter records as a source in §4z; the store-key
//! column is a measurement of the store recorded in the same charter section.
//! Each row carries its own [`Citation`]s, and `/aliases.json` serves them.
//!
//! # A row with no source is not applied
//!
//! [`Status::Unverified`] rows are listed and never returned by a lookup that
//! resolves a name: `CLAUDE.md` §3 rule 1. Every row shipped today is
//! [`Status::Sourced`]; the rule is proven on a table built in the tests.
//!
//! # A symbol with no row is not an alias
//!
//! HDFC and MINDTREE, whose options the store holds, have no row in NSE's
//! file. They keep their own keys and no successor is guessed for them
//! (D-3332 item 2). [`NOT_ALIASED`] names them so a reader sees the refusal.
//!
//! # `FNO_UNDERLYINGS` stays the only list naming the F&O universe
//!
//! [`IndexRow::fno`] is the symbol an index's contracts carry or carried, not a
//! membership claim: NIFTYIT and NIFTYCPSE options are stored from 2019 and are
//! not in today's list. Membership is still asked of
//! `crate::universe::FNO_UNDERLYINGS` and nothing here copies it.
//!
//! # Cost
//!
//! Every lookup by a symbol or a store key is one probe of a compile-time
//! [`MemberIndex`]: hash, mask, probe, the structure `crate::universe` already
//! measures. Following a chain of symbol changes is one probe per hop, and the
//! hops are bounded by the table's length (a cycle cannot loop: the walk stops
//! after `R` hops, and `the_shipped_lineage_has_no_cycle_and_no_duplicate` pins
//! that the shipped table has none). [`index_by_name`] collapses the asked
//! name to its letters and digits in a stack buffer -- refusing past
//! [`MAX_NAME_BYTES`] before it reads -- and then makes one probe of a table
//! holding every row's names collapsed the same way ([`INDEX_NAMES`]). The
//! tables are `static`, built by the compiler.

use crate::instrument::Segment;
use crate::universe::MemberIndex;
use crate::vendor::Vendor;

/// The charter section every row of this table traces to.
pub const CHARTER: &str =
    "docs/00-charter.md, Index aliases and symbol changes — read 7 Oct 2026 (D-3332)";

/// The decision that owns this table.
pub const DECISION: &str = "D-3332";

/// The feeds whose store keys this table records.
///
/// Zerodha's keys were measured on the store (charter, D-3332: every
/// `bars/zerodha/NSE/INDEX/<key>/` present on 7 Oct 2026), and GDFL files its
/// index and old-name share series under the same keys (D-3333). No other
/// feed's keys were measured, so a join that applies the table to one names
/// this list rather than assuming it.
pub const KEYED_FEEDS: [Vendor; 2] = [Vendor::Zerodha, Vendor::Gdfl];

/// Whether this table records `feed`'s store keys. Two comparisons.
#[must_use]
pub fn keys_feed(feed: Vendor) -> bool {
    matches!(feed, Vendor::Zerodha | Vendor::Gdfl)
}

/// One source a row rests on: where it was read, when, and what it says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Citation {
    /// Where it was read: a URL, or the path of a measurement.
    pub url: &'static str,
    /// The IST day it was read, `YYYY-MM-DD`.
    pub read: &'static str,
    /// The sha256 of the bytes read, when the charter records one.
    pub sha256: Option<&'static str>,
    /// The words or the measurement that carry the fact.
    pub says: &'static str,
}

/// Whether a row may be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Every fact in the row traces to these citations.
    Sourced(&'static [Citation]),
    /// The row is listed so a reader sees it, and no lookup applies it. The
    /// text says what is missing.
    Unverified(&'static str),
}

impl Status {
    /// Whether a lookup may return the row.
    #[must_use]
    pub const fn applies(self) -> bool {
        matches!(self, Self::Sourced(_))
    }

    /// The wire word: `sourced` or `UNVERIFIED`.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Sourced(_) => "sourced",
            Self::Unverified(_) => "UNVERIFIED",
        }
    }

    /// The citations, empty for an unverified row.
    #[must_use]
    pub const fn citations(self) -> &'static [Citation] {
        match self {
            Self::Sourced(cited) => cited,
            Self::Unverified(_) => &[],
        }
    }
}

/// One index under every name it goes by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexRow {
    /// The symbol its futures and options carry or carried, `None` for an
    /// index with no derivatives (INDIA VIX).
    pub fno: Option<&'static str>,
    /// NSE's index name, `allIndices` `index`.
    pub nse_name: &'static str,
    /// NSE's `indexSymbol`, which is also the GDFL file stem
    /// (`<indexSymbol>.NSE_IDX.csv`).
    pub index_symbol: &'static str,
    /// The Kite tradingsymbol of the index row in Zerodha's instrument dump.
    pub kite: &'static str,
    /// The key the store files the series under on every [`KEYED_FEEDS`]
    /// feed: `bars/<feed>/NSE/INDEX/<store_key>/`.
    pub store_key: &'static str,
    /// Reference only: stored and stamped, never swept (`CLAUDE.md` §1).
    pub reference_only: bool,
    /// Whether the row may be applied, and its sources.
    pub status: Status,
}

impl IndexRow {
    /// The F&O symbol when it has one, else the store key: the name an
    /// equity-list lookup is asked under.
    #[must_use]
    pub const fn listed_as(&self) -> &'static str {
        match self.fno {
            Some(fno) => fno,
            None => self.store_key,
        }
    }

    /// Every name this row answers to, the store key first.
    #[must_use]
    pub const fn names(&self) -> [&'static str; 5] {
        [
            self.store_key,
            match self.fno {
                Some(fno) => fno,
                None => self.store_key,
            },
            self.nse_name,
            self.index_symbol,
            self.kite,
        ]
    }
}

/// A calendar day, as a symbol change is dated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ymd {
    /// The year.
    pub year: u16,
    /// The month, 1 to 12.
    pub month: u8,
    /// The day of the month, 1 to 31.
    pub day: u8,
}

impl Ymd {
    /// A day. Not validated against a calendar: the table's own test checks
    /// every shipped date, and a caller building one from a store path has
    /// already parsed it.
    #[must_use]
    pub const fn new(year: u16, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }
}

impl core::fmt::Display for Ymd {
    /// `YYYY-MM-DD`.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// One dated symbol change, as NSE's `symbolchange.csv` records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolChange {
    /// The symbol before the change.
    pub old: &'static str,
    /// The symbol from [`Self::effective`] on.
    pub new: &'static str,
    /// The first day the new symbol applies.
    pub effective: Ymd,
    /// Whether the row may be applied, and its sources.
    pub status: Status,
}

/// A symbol the store holds that is deliberately NOT aliased, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotAliased {
    /// The symbol.
    pub symbol: &'static str,
    /// Why no successor is applied.
    pub why: &'static str,
}

// ---------------------------------------------------------------------------
// The sources, each written once and cited by every row that rests on it.
// ---------------------------------------------------------------------------

/// NSE's own index list: each index's `index` name and its `indexSymbol`.
pub const ALL_INDICES: Citation = Citation {
    url: "https://www.nseindia.com/api/allIndices",
    read: "2026-10-07",
    sha256: Some("563bf499d9815fb97198c50ffc1b8fe78c17ac0e879d389b5e1f8bcf460b25cd"),
    says: "139 indices, timestamp 06-Oct-2026 15:30: each index's `index` name and `indexSymbol`",
};

/// The GDFL index file stem is the `indexSymbol`: measured by the /db audit
/// in the vendor's capital-market zips.
pub const GDFL_STEM: Citation = Citation {
    url: "docs/00-charter.md, Index aliases and symbol changes (DB-G26, DB-G53)",
    read: "2026-10-07",
    sha256: None,
    says: "the GDFL capital-market zips name each index file `<indexSymbol>.NSE_IDX.csv`",
};

/// The Kite tradingsymbol of each index row, read off Zerodha's instrument
/// dump, whose columns the Kite instruments specification names (charter §8;
/// the vendor itself is charter §4z).
///
/// A measurement, not a vendor statement: the bytes read are digested so a
/// later dump that names an index differently is a different source, not a
/// silent change to this one.
pub const KITE_DUMP: Citation = Citation {
    url: "runtime/masters/zerodha_instruments.csv (Kite instruments dump; docs/00-charter.md §8, §4z)",
    read: "2026-10-09",
    sha256: Some("cd24bb5e6a4f13968f0c974d3db62c4c4f2243c4253201146f66dc149c1874a4"),
    says: "measured: the dump landed 2026-10-06 16:09 IST lists each of these indices on segment \
           INDICES, exchange NSE, with a tradingsymbol equal to NSE's indexSymbol (MIDCPNIFTY's \
           index row is tradingsymbol NIFTY MID SELECT, name NIFTY MIDCAP SELECT (MIDCPNIFTY))",
};

/// The store key: present under `bars/zerodha/NSE/INDEX/` (charter), and the
/// key GDFL files the same index under (D-3333).
pub const STORE_KEY: Citation = Citation {
    url: "docs/00-charter.md, Index aliases and symbol changes; docs/05-decisions.md D-3333",
    read: "2026-10-07",
    sha256: None,
    says: "the indexSymbol with its spaces removed is the Zerodha store key \
           (bars/zerodha/NSE/INDEX/<key>/, present on 7 Oct 2026), and GDFL files the same index \
           under the same key",
};

/// The store's own rename of `NIFTY 50` and `NIFTY BANK`, read off two
/// brokers' masters that write the ticker beside the name.
///
/// Re-measured on 9 Oct 2026 off the masters landed 6 Oct 2026 16:09 IST, and
/// both digests are in the text because a citation carries one: the two are
/// witnesses to the rename, not sources this table reads at run time.
pub const STORE_RENAME: Citation = Citation {
    url: "runtime/masters/{groww_instruments,dhan_scrip}.csv (docs/00-charter.md §4z; \
          core::vendor::Vendor::index_alias)",
    read: "2026-10-09",
    sha256: None,
    says: "measured: Groww's IDX rows write trading_symbol NIFTY beside name NIFTY 50 and \
           BANKNIFTY beside NIFTY Bank (sha256 \
           cb4a5fdbe9e9569f493fbfe9d1d9d844388deaa5b8ba3d3ebd75cc1c9bd72b43); Dhan's INDEX rows \
           write SYMBOL_NAME NIFTY beside DISPLAY_NAME Nifty 50 and BANKNIFTY beside Nifty Bank \
           (sha256 22654da3736109c06ba5ec489fc32949c7a3247dbe71a3c90ffe583aaa32cc09); the store \
           keys the two indices NIFTY and BANKNIFTY",
};

/// NSE's file of symbol changes.
pub const SYMBOL_CHANGE_CSV: Citation = Citation {
    url: "https://nsearchives.nseindia.com/content/equities/symbolchange.csv",
    read: "2026-10-07",
    sha256: Some("948ccc56f46555bc6553538047d7b967e99a9250c58b7bf0a1d66a7e1f033e10"),
    says: "69,052 bytes, 1,065 lines: company, old symbol, new symbol, date",
};

/// NSE's circular for ZOMATO to ETERNAL.
pub const CML67420: Citation = Citation {
    url: "https://nsearchives.nseindia.com/content/circulars/CML67420.pdf",
    read: "2026-10-07",
    sha256: Some("f0ee1ddfa019753b0c04718e3e55761e74dafef34c5763be3d28000e9759afd4"),
    says: "NSE/CML/67420, Circular Ref. 0518/2025, 3 April 2025: Zomato Limited to ETERNAL \
           LIMITED, ZOMATO to ETERNAL, effective 9 April 2025",
};

/// One product page, as the charter recorded it.
const fn product(url: &'static str, sha256: &'static str, says: &'static str) -> Citation {
    Citation {
        url,
        read: "2026-10-07",
        sha256: Some(sha256),
        says,
    }
}

const FINNIFTY_PAGE: Citation = product(
    "https://www.nseindia.com/static/products-services/equity-derivatives-finnifty",
    "60cd6ddf540a2771c4af76353ad2c0ea60fcada964ee77a0c427c8d8dc6aef44",
    "Underlying symbol denotes the underlying index, which is FINNIFTY; The underlying index \
     is Nifty Financial Services Index",
);
const MIDCPNIFTY_PAGE: Citation = product(
    "https://www.nseindia.com/static/products-services/equity-derivatives-nifty-midcap-select",
    "d3930bfad0f56cba53e19935e05edbef000b96644be0f2c5be83e5e789ada710",
    "futures contracts on MIDCPNIFTY; The underlying index is NIFTY Midcap Select Index",
);
const NIFTYNXT50_PAGE: Citation = product(
    "https://www.nseindia.com/static/products-services/equity-derivatives-nifty-next50",
    "ca5f1c41a3856ce808921c30e602fb9262dcde62855caa19ee9b5fdfd599a2f0",
    "Symbol : NIFTYNXT50 Underlying: Nifty Next 50 Index",
);
const NIFTYIT_PAGE: Citation = product(
    "https://www.nseindia.com/static/products-services/equity-derivatives-niftyit",
    "3c99c7b3a40fc47ded2316bb38aab584fbbba87ce33c6392f7b3e21bd1ddb6e6",
    "Underlying : NIFTYIT; The underlying index is Nifty IT",
);
const NIFTYFPI_PAGE: Citation = product(
    "https://www.nseindia.com/static/products-services/equity-derivatives-nifty-india-fpi",
    "ee0c9e1f89df9cb48af432a17be04a5298f7d4e095fb1826f8358e413e9a070b",
    "the underlying index, which is NIFTYFPI; The underlying index is Nifty India FPI 150 Index",
);
const NIFTYCPSE_PAGE: Citation = product(
    "https://www.nseindia.com/static/products-services/equity-derivatives-niftycpse",
    "81c632e8fcc201d6dc7dccec1b4a434482212e42fdd017edd64ef00656bc9e8e",
    "Underlying : NIFTYCPSE; The underlying index is NIFTY CPSE",
);

/// The two indices the store renamed, cited by their rename.
const RENAMED_INDEX: &[Citation] = &[STORE_RENAME, ALL_INDICES, GDFL_STEM, KITE_DUMP];
/// An index row whose F&O symbol a product page names.
const fn product_row(page: &'static [Citation; 5]) -> Status {
    Status::Sourced(page)
}
const FINNIFTY_SOURCES: [Citation; 5] =
    [FINNIFTY_PAGE, ALL_INDICES, GDFL_STEM, KITE_DUMP, STORE_KEY];
const MIDCPNIFTY_SOURCES: [Citation; 5] = [
    MIDCPNIFTY_PAGE,
    ALL_INDICES,
    GDFL_STEM,
    KITE_DUMP,
    STORE_KEY,
];
const NIFTYNXT50_SOURCES: [Citation; 5] = [
    NIFTYNXT50_PAGE,
    ALL_INDICES,
    GDFL_STEM,
    KITE_DUMP,
    STORE_KEY,
];
const NIFTYIT_SOURCES: [Citation; 5] = [NIFTYIT_PAGE, ALL_INDICES, GDFL_STEM, KITE_DUMP, STORE_KEY];
const NIFTYFPI_SOURCES: [Citation; 5] =
    [NIFTYFPI_PAGE, ALL_INDICES, GDFL_STEM, KITE_DUMP, STORE_KEY];
const NIFTYCPSE_SOURCES: [Citation; 5] =
    [NIFTYCPSE_PAGE, ALL_INDICES, GDFL_STEM, KITE_DUMP, STORE_KEY];
/// INDIA VIX has no product page: it has no F&O symbol in this table.
const VIX_SOURCES: &[Citation] = &[ALL_INDICES, GDFL_STEM, KITE_DUMP, STORE_KEY];
/// A symbol change read in NSE's file.
const IN_THE_FILE: &[Citation] = &[SYMBOL_CHANGE_CSV];
/// ZOMATO to ETERNAL, read in NSE's file and in its own circular.
const IN_THE_FILE_AND_CIRCULAR: &[Citation] = &[SYMBOL_CHANGE_CSV, CML67420];

/// Every index this table names. Order is the charter's.
pub const INDEX_ROWS: [IndexRow; 9] = [
    IndexRow {
        fno: Some("NIFTY"),
        nse_name: "NIFTY 50",
        index_symbol: "NIFTY 50",
        kite: "NIFTY 50",
        store_key: "NIFTY",
        reference_only: false,
        status: Status::Sourced(RENAMED_INDEX),
    },
    IndexRow {
        fno: Some("BANKNIFTY"),
        nse_name: "NIFTY BANK",
        index_symbol: "NIFTY BANK",
        kite: "NIFTY BANK",
        store_key: "BANKNIFTY",
        reference_only: false,
        status: Status::Sourced(RENAMED_INDEX),
    },
    IndexRow {
        fno: Some("FINNIFTY"),
        nse_name: "NIFTY FINANCIAL SERVICES",
        index_symbol: "NIFTY FIN SERVICE",
        kite: "NIFTY FIN SERVICE",
        store_key: "NIFTYFINSERVICE",
        reference_only: false,
        status: product_row(&FINNIFTY_SOURCES),
    },
    IndexRow {
        fno: Some("MIDCPNIFTY"),
        nse_name: "NIFTY MIDCAP SELECT",
        index_symbol: "NIFTY MID SELECT",
        kite: "NIFTY MID SELECT",
        store_key: "NIFTYMIDSELECT",
        reference_only: false,
        status: product_row(&MIDCPNIFTY_SOURCES),
    },
    IndexRow {
        fno: Some("NIFTYNXT50"),
        nse_name: "NIFTY NEXT 50",
        index_symbol: "NIFTY NEXT 50",
        kite: "NIFTY NEXT 50",
        store_key: "NIFTYNEXT50",
        reference_only: false,
        status: product_row(&NIFTYNXT50_SOURCES),
    },
    IndexRow {
        fno: Some("NIFTYIT"),
        nse_name: "NIFTY IT",
        index_symbol: "NIFTY IT",
        kite: "NIFTY IT",
        store_key: "NIFTYIT",
        reference_only: false,
        status: product_row(&NIFTYIT_SOURCES),
    },
    IndexRow {
        fno: Some("NIFTYFPI"),
        nse_name: "NIFTY INDIA FPI 150",
        index_symbol: "NIFTY FPI 150",
        kite: "NIFTY FPI 150",
        store_key: "NIFTYFPI150",
        reference_only: false,
        status: product_row(&NIFTYFPI_SOURCES),
    },
    IndexRow {
        fno: Some("NIFTYCPSE"),
        nse_name: "NIFTY CPSE",
        index_symbol: "NIFTY CPSE",
        kite: "NIFTY CPSE",
        store_key: "NIFTYCPSE",
        reference_only: false,
        status: product_row(&NIFTYCPSE_SOURCES),
    },
    IndexRow {
        fno: None,
        nse_name: "INDIA VIX",
        index_symbol: "INDIA VIX",
        kite: "INDIA VIX",
        store_key: "INDIAVIX",
        reference_only: true,
        status: Status::Sourced(VIX_SOURCES),
    },
];

/// One row of NSE's file.
const fn change(old: &'static str, new: &'static str, effective: Ymd) -> SymbolChange {
    SymbolChange {
        old,
        new,
        effective,
        status: Status::Sourced(IN_THE_FILE),
    }
}

/// The rows of NSE's `symbolchange.csv` whose old or new symbol the store
/// holds as a GDFL share or option series (charter, checked 7 Oct 2026).
/// Order is the file's date order, as the charter lists them.
pub const SYMBOL_CHANGES: [SymbolChange; 18] = [
    change("IDFCBANK", "IDFCFIRSTB", Ymd::new(2019, 1, 16)),
    change("NIITTECH", "COFORGE", Ymd::new(2020, 8, 20)),
    change("INFRATEL", "INDUSTOWER", Ymd::new(2020, 12, 18)),
    change("CADILAHC", "ZYDUSLIFE", Ymd::new(2022, 3, 7)),
    change("MOTHERSUMI", "MOTHERSON", Ymd::new(2022, 6, 9)),
    change("LTI", "LTIM", Ymd::new(2022, 12, 5)),
    change("SRTRANSFIN", "SHRIRAMFIN", Ymd::new(2022, 12, 20)),
    change("IIFLWAM", "360ONE", Ymd::new(2023, 1, 23)),
    change("PVR", "PVRINOX", Ymd::new(2023, 5, 12)),
    change("ADANITRANS", "ADANIENSOL", Ymd::new(2023, 8, 24)),
    change("AMARAJABAT", "ARE&M", Ymd::new(2023, 10, 27)),
    change("L&TFH", "LTF", Ymd::new(2024, 4, 23)),
    change("MCDOWELL-N", "UNITDSPR", Ymd::new(2024, 6, 7)),
    change("IBULHSGFIN", "SAMMAANCAP", Ymd::new(2024, 7, 26)),
    change("GMRINFRA", "GMRAIRPORT", Ymd::new(2024, 12, 11)),
    SymbolChange {
        old: "ZOMATO",
        new: "ETERNAL",
        effective: Ymd::new(2025, 4, 9),
        status: Status::Sourced(IN_THE_FILE_AND_CIRCULAR),
    },
    change("TATAMOTORS", "TMPV", Ymd::new(2025, 10, 24)),
    change("LTIM", "LTM", Ymd::new(2026, 2, 27)),
];

/// Symbols the store holds as option underlyings that have no row in NSE's
/// file, and so keep their own keys (D-3332 item 2, D-3333).
pub const NOT_ALIASED: [NotAliased; 2] = [
    NotAliased {
        symbol: "HDFC",
        why: "no row in NSE's symbolchange.csv (read 7 Oct 2026): it keeps its own key and no \
              successor is guessed; why it stopped trading is UNVERIFIED (D-3332 item 2)",
    },
    NotAliased {
        symbol: "MINDTREE",
        why: "no row in NSE's symbolchange.csv (read 7 Oct 2026): it keeps its own key and no \
              successor is guessed; why it stopped trading is UNVERIFIED (D-3332 item 2)",
    },
];

// ---------------------------------------------------------------------------
// The lookup structures.
// ---------------------------------------------------------------------------

/// The index rows, with compile-time probe tables over the F&O symbol, the
/// store key and every name collapsed.
///
/// `N` is the number of collapsed names, `S` the slots of each probe table.
pub struct Indices<const R: usize, const N: usize, const S: usize> {
    rows: [IndexRow; R],
    names: [(&'static str, usize); N],
    by_fno: MemberIndex<S>,
    by_store_key: MemberIndex<S>,
    by_name: MemberIndex<S>,
}

impl<const R: usize, const N: usize, const S: usize> Indices<R, N, S> {
    /// Builds the three probe tables at compile time. `names` pairs every
    /// collapsed name with the position of its row in `rows`.
    ///
    /// # Panics
    ///
    /// At compile time, through [`MemberIndex::build`], if `S` is not a power
    /// of two or the rows or names would fill more than half of it.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "`i` is bounded by the loop conditions `i < R` and `i < N`, the \
                  lengths of the arrays it indexes. A `const fn` cannot iterate, \
                  and an out-of-range index in a const context is a compile error."
    )]
    pub const fn build(rows: [IndexRow; R], names: [(&'static str, usize); N]) -> Self {
        // An index with no F&O symbol is keyed under the empty string, which
        // no lookup ever asks for: `by_fno` refuses an empty name before it
        // probes and checks the row's own field after.
        let mut fno = [""; R];
        let mut keys = [""; R];
        let mut i = 0;
        while i < R {
            fno[i] = match rows[i].fno {
                Some(name) => name,
                None => "",
            };
            keys[i] = rows[i].store_key;
            i += 1;
        }
        let mut collapsed = [""; N];
        let mut i = 0;
        while i < N {
            collapsed[i] = names[i].0;
            i += 1;
        }
        Self {
            rows,
            names,
            by_fno: MemberIndex::build(&fno),
            by_store_key: MemberIndex::build(&keys),
            by_name: MemberIndex::build(&collapsed),
        }
    }

    /// Every collapsed name and the position of the row it names.
    #[must_use]
    pub const fn names(&self) -> &[(&'static str, usize); N] {
        &self.names
    }

    /// Every row, applied or not, in table order.
    #[must_use]
    pub const fn rows(&self) -> &[IndexRow; R] {
        &self.rows
    }

    /// The applied row whose F&O symbol is `fno`. One probe.
    #[must_use]
    pub fn by_fno(&self, fno: &str) -> Option<&IndexRow> {
        if fno.is_empty() {
            return None;
        }
        self.by_fno
            .position(fno)
            .and_then(|at| self.rows.get(at))
            .filter(|row| row.fno == Some(fno) && row.status.applies())
    }

    /// The applied row the store files under `key`. One probe.
    #[must_use]
    pub fn by_store_key(&self, key: &str) -> Option<&IndexRow> {
        self.by_store_key
            .position(key)
            .and_then(|at| self.rows.get(at))
            .filter(|row| row.store_key == key && row.status.applies())
    }

    /// The applied row any of whose names is `name`, comparing ASCII letters
    /// and digits only and ignoring case: `nifty fin service`, `NIFTY 50`,
    /// `FINNIFTY` and `NIFTYFINSERVICE` all find their row.
    ///
    /// The name is collapsed into a stack buffer, then one probe. A name with
    /// more letters and digits than a symbol holds, or longer than
    /// [`MAX_NAME_BYTES`] in all, names no row and is refused before it is
    /// walked.
    #[must_use]
    pub fn by_name(&self, name: &str) -> Option<&IndexRow> {
        let mut buf = [0u8; crate::symbol::SYMBOL_CAPACITY];
        let collapsed = collapse_into(name, &mut buf)?;
        self.by_name
            .position(collapsed)
            .and_then(|at| self.names.get(at))
            .filter(|&&(held, _)| held == collapsed)
            .and_then(|&(_, row)| self.rows.get(row))
            .filter(|row| row.status.applies())
    }
}

/// The longest name [`Indices::by_name`] reads, in bytes. NSE's longest index
/// name here is 24 bytes; four times that leaves room for any separators a
/// reader types and still bounds the walk.
pub const MAX_NAME_BYTES: usize = 96;

/// `name`'s ASCII letters and digits, upper-cased, in `buf`; `None` when it
/// has none, more than `buf` holds, or more than [`MAX_NAME_BYTES`] bytes.
fn collapse_into<'b>(
    name: &str,
    buf: &'b mut [u8; crate::symbol::SYMBOL_CAPACITY],
) -> Option<&'b str> {
    if name.len() > MAX_NAME_BYTES {
        return None;
    }
    let mut len = 0usize;
    for byte in name.bytes().filter(u8::is_ascii_alphanumeric) {
        *buf.get_mut(len)? = byte.to_ascii_uppercase();
        len += 1;
    }
    if len == 0 {
        return None;
    }
    core::str::from_utf8(buf.get(..len)?).ok()
}

/// The dated symbol changes, with a compile-time probe table over the old
/// symbol and one over the new.
pub struct Lineage<const R: usize, const S: usize> {
    rows: [SymbolChange; R],
    by_old: MemberIndex<S>,
    by_new: MemberIndex<S>,
}

/// Why a file stem may not be filed on a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stale {
    /// The stem is the OLD symbol of this change and the day is on or after
    /// it: the exchange no longer used the stem that day.
    AfterChange(SymbolChange),
    /// The stem is the NEW symbol of this change and the day is before it:
    /// the exchange did not use the stem yet.
    BeforeChange(SymbolChange),
}

impl<const R: usize, const S: usize> Lineage<R, S> {
    /// Builds both probe tables at compile time.
    ///
    /// # Panics
    ///
    /// At compile time, through [`MemberIndex::build`], if `S` is not a power
    /// of two or the rows would fill more than half of it.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "`i` is bounded by the loop condition `i < R` and both arrays \
                  have length R. A `const fn` cannot iterate, and an out-of-range \
                  index in a const context is a compile error."
    )]
    pub const fn build(rows: [SymbolChange; R]) -> Self {
        let mut olds = [""; R];
        let mut news = [""; R];
        let mut i = 0;
        while i < R {
            olds[i] = rows[i].old;
            news[i] = rows[i].new;
            i += 1;
        }
        Self {
            rows,
            by_old: MemberIndex::build(&olds),
            by_new: MemberIndex::build(&news),
        }
    }

    /// Every row, applied or not, in table order.
    #[must_use]
    pub const fn rows(&self) -> &[SymbolChange; R] {
        &self.rows
    }

    /// The applied change away from `old`. One probe.
    #[must_use]
    pub fn from_old(&self, old: &str) -> Option<&SymbolChange> {
        self.by_old
            .position(old)
            .and_then(|at| self.rows.get(at))
            .filter(|row| row.old == old && row.status.applies())
    }

    /// The applied change that produced `new`. One probe.
    #[must_use]
    pub fn to_new(&self, new: &str) -> Option<&SymbolChange> {
        self.by_new
            .position(new)
            .and_then(|at| self.rows.get(at))
            .filter(|row| row.new == new && row.status.applies())
    }

    /// The applied changes from `symbol` forward, oldest first: `LTI` walks
    /// `LTI → LTIM`, then `LTIM → LTM`. Empty when `symbol` has none.
    ///
    /// At most `R` hops, one probe each, so a cycle cannot loop.
    pub fn forward<'a>(&'a self, symbol: &'a str) -> impl Iterator<Item = &'a SymbolChange> + 'a {
        let mut at = symbol;
        (0..R).map_while(move |_| {
            let hop = self.from_old(at)?;
            at = hop.new;
            Some(hop)
        })
    }

    /// The applied changes that led to `symbol`, newest first: `LTM` walks
    /// `LTIM → LTM`, then `LTI → LTIM`. Empty when `symbol` has none. What a
    /// reader is shown as "formerly X until D".
    ///
    /// At most `R` hops, one probe each.
    pub fn backward<'a>(&'a self, symbol: &'a str) -> impl Iterator<Item = &'a SymbolChange> + 'a {
        let mut at = symbol;
        (0..R).map_while(move |_| {
            let hop = self.to_new(at)?;
            at = hop.old;
            Some(hop)
        })
    }

    /// The newest symbol `symbol`'s applied changes reach, or `symbol` itself.
    #[must_use]
    pub fn current<'a>(&'a self, symbol: &'a str) -> &'a str {
        self.forward(symbol).last().map_or(symbol, |hop| hop.new)
    }

    /// The store key a vendor file named `stem` and dated `day` is filed
    /// under (D-3333): the newest symbol the stem's changes reach, when the
    /// stem was the exchange's symbol that day.
    ///
    /// # Errors
    ///
    /// [`Stale::BeforeChange`] when the stem is a NEW symbol and the day is
    /// before its change; [`Stale::AfterChange`] when the stem is an OLD
    /// symbol and the day is on or after its change. Neither is guessed past:
    /// a file named for a symbol the exchange was not using that day is a
    /// vendor fault or a reused symbol, and either way not this table's to
    /// decide.
    pub fn filed_under<'a>(&'a self, stem: &'a str, day: Ymd) -> Result<&'a str, Stale> {
        if let Some(made) = self.to_new(stem)
            && day < made.effective
        {
            return Err(Stale::BeforeChange(*made));
        }
        if let Some(gone) = self.from_old(stem)
            && day >= gone.effective
        {
            return Err(Stale::AfterChange(*gone));
        }
        Ok(self.current(stem))
    }
}

/// Every name an index row answers to, collapsed to its ASCII letters and
/// digits in upper case, and the position of its row in [`INDEX_ROWS`].
///
/// Written out rather than computed, because a `const fn` cannot build a
/// `&'static str`; `every_name_of_every_row_is_collapsed_here_and_nothing_else_is`
/// holds it to the rows both ways: every name of every row collapses to an
/// entry naming that row, and every entry is the collapse of a name of the row
/// it names.
pub const INDEX_NAMES: [(&str, usize); 18] = [
    ("NIFTY", 0),
    ("NIFTY50", 0),
    ("BANKNIFTY", 1),
    ("NIFTYBANK", 1),
    ("NIFTYFINSERVICE", 2),
    ("FINNIFTY", 2),
    ("NIFTYFINANCIALSERVICES", 2),
    ("NIFTYMIDSELECT", 3),
    ("MIDCPNIFTY", 3),
    ("NIFTYMIDCAPSELECT", 3),
    ("NIFTYNEXT50", 4),
    ("NIFTYNXT50", 4),
    ("NIFTYIT", 5),
    ("NIFTYFPI150", 6),
    ("NIFTYFPI", 6),
    ("NIFTYINDIAFPI150", 6),
    ("NIFTYCPSE", 7),
    ("INDIAVIX", 8),
];

/// The shipped index rows. Nine rows and eighteen names in 128 slots, under a
/// quarter full.
pub static INDICES: Indices<9, 18, 128> = Indices::build(INDEX_ROWS, INDEX_NAMES);

/// The symbols of [`NOT_ALIASED`], in its order.
#[expect(
    clippy::indexing_slicing,
    reason = "`i` is bounded by the loop condition `i < N`, the length of both \
              arrays. A `const fn` cannot call `get`, and an out-of-range index in \
              a const context is a compile error."
)]
const fn not_aliased_symbols<const N: usize>(rows: &[NotAliased; N]) -> [&'static str; N] {
    let mut out = [""; N];
    let mut i = 0;
    while i < N {
        out[i] = rows[i].symbol;
        i += 1;
    }
    out
}

/// [`NOT_ALIASED`], probed by symbol.
static NOT_ALIASED_INDEX: MemberIndex<8> = MemberIndex::build(&not_aliased_symbols(&NOT_ALIASED));

/// The shipped symbol changes. Eighteen rows in 128 slots, under a quarter
/// full.
pub static LINEAGE: Lineage<18, 128> = Lineage::build(SYMBOL_CHANGES);

/// The applied index row whose F&O symbol is `fno`. One probe.
#[must_use]
pub fn index_by_fno(fno: &str) -> Option<&'static IndexRow> {
    INDICES.by_fno(fno)
}

/// The applied index row the store files under `key`. One probe.
#[must_use]
pub fn index_by_store_key(key: &str) -> Option<&'static IndexRow> {
    INDICES.by_store_key(key)
}

/// The applied index row answering to `name` under any of its spellings.
/// See [`Indices::by_name`].
#[must_use]
pub fn index_by_name(name: &str) -> Option<&'static IndexRow> {
    INDICES.by_name(name)
}

/// The deliberate refusal for `symbol`, when it is one. One probe.
#[must_use]
pub fn not_aliased(symbol: &str) -> Option<&'static NotAliased> {
    NOT_ALIASED_INDEX
        .position(symbol)
        .and_then(|at| NOT_ALIASED.get(at))
        .filter(|row| row.symbol == symbol)
}

/// How an option underlying was resolved to a store series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Via {
    /// An index: by its F&O symbol or by its store key.
    Index(&'static IndexRow),
    /// A share whose symbol changed: the first change walked. The key is the
    /// newest symbol the chain reaches.
    Renamed(&'static SymbolChange),
    /// No row: the symbol is its own key, as a share.
    Own,
}

/// Where an option underlying's own price series lives in the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Underlying<'a> {
    /// `INDEX` for an index, `CASH` for a share.
    pub segment: Segment,
    /// The store key: `NIFTYFINSERVICE` for FINNIFTY, `TMPV` for TATAMOTORS.
    pub key: &'a str,
    /// How it was resolved.
    pub via: Via,
}

/// Where the store holds `symbol`'s own price series, when `symbol` is an
/// option's underlying (D-3332 item 3).
///
/// - An F&O index symbol (`FINNIFTY`, `NIFTYIT`, `NIFTYFPI`) or an index store
///   key (`NIFTYFINSERVICE`) is an INDEX, read under the row's store key.
/// - A symbol a dated change renamed (`TATAMOTORS`, `LTI`) is a SHARE, read
///   under the newest symbol its changes reach, whose months hold the old
///   stem's bars after D-3333.
/// - Anything else is a share under its own key: the definition of a stock
///   option's underlying, not a fallback (`CLAUDE.md` §1, D-0682).
///
/// This replaces the five names of `crate::universe::FNO_INDEX_UNDERLYINGS`
/// as the segment authority (DB-G54): each of the five has a row here, and
/// `every_fno_index_underlying_resolves_as_an_index` pins that.
///
/// At most three probes and one hop per applied change.
#[must_use]
pub fn option_underlying(symbol: &str) -> Underlying<'_> {
    if let Some(row) = index_by_fno(symbol).or_else(|| index_by_store_key(symbol)) {
        return Underlying {
            segment: Segment::Index,
            key: row.store_key,
            via: Via::Index(row),
        };
    }
    match LINEAGE.from_old(symbol) {
        Some(first) => Underlying {
            segment: Segment::Cash,
            key: LINEAGE.current(symbol),
            via: Via::Renamed(first),
        },
        None => Underlying {
            segment: Segment::Cash,
            key: symbol,
            via: Via::Own,
        },
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;
    use crate::universe::{FNO_INDEX, FNO_INDEX_UNDERLYINGS, FNO_UNDERLYINGS, fnv1a, mask};

    /// The NSE derivatives product page base every product citation shares,
    /// as the charter writes it.
    const PRODUCT: &str = "https://www.nseindia.com/static/products-services/";

    /// A name's ASCII letters and digits, upper-cased: the oracle the
    /// production collapse is checked against.
    fn collapse(name: &str) -> String {
        name.bytes()
            .filter(u8::is_ascii_alphanumeric)
            .map(|b| char::from(b.to_ascii_uppercase()))
            .collect()
    }

    /// Whether two names collapse to one.
    fn same_name(own: &str, asked: &str) -> bool {
        collapse(own) == collapse(asked)
    }

    /// The empty-string key the build gives an index with no F&O symbol is
    /// never answered.
    #[test]
    fn an_index_with_no_fno_symbol_is_never_found_by_an_empty_name() {
        assert_eq!(index_by_fno(""), None);
        let vix = index_by_store_key("INDIAVIX").expect("VIX has a row");
        assert_eq!(vix.fno, None);
        assert!(vix.reference_only);
        assert_eq!(vix.listed_as(), "INDIAVIX");
        assert_eq!(vix.names()[1], "INDIAVIX");
    }

    /// **THE THREE F&O NAMES MEET THEIR STORE KEYS.** FINNIFTY, MIDCPNIFTY and
    /// NIFTYNXT50 were the names `/spot.json` looked for under `INDEX/` and
    /// no feed files; the store holds them as NIFTYFINSERVICE, NIFTYMIDSELECT
    /// and NIFTYNEXT50 (DB-G25).
    #[test]
    fn the_fno_index_symbols_resolve_to_the_store_keys_the_feeds_file() {
        for (fno, key, nse, stem) in [
            ("NIFTY", "NIFTY", "NIFTY 50", "NIFTY 50"),
            ("BANKNIFTY", "BANKNIFTY", "NIFTY BANK", "NIFTY BANK"),
            (
                "FINNIFTY",
                "NIFTYFINSERVICE",
                "NIFTY FINANCIAL SERVICES",
                "NIFTY FIN SERVICE",
            ),
            (
                "MIDCPNIFTY",
                "NIFTYMIDSELECT",
                "NIFTY MIDCAP SELECT",
                "NIFTY MID SELECT",
            ),
            (
                "NIFTYNXT50",
                "NIFTYNEXT50",
                "NIFTY NEXT 50",
                "NIFTY NEXT 50",
            ),
            ("NIFTYIT", "NIFTYIT", "NIFTY IT", "NIFTY IT"),
            (
                "NIFTYFPI",
                "NIFTYFPI150",
                "NIFTY INDIA FPI 150",
                "NIFTY FPI 150",
            ),
            ("NIFTYCPSE", "NIFTYCPSE", "NIFTY CPSE", "NIFTY CPSE"),
        ] {
            let row = index_by_fno(fno).unwrap_or_else(|| panic!("{fno} has a row"));
            assert_eq!(row.store_key, key, "{fno}");
            assert_eq!(row.nse_name, nse, "{fno}");
            assert_eq!(row.index_symbol, stem, "{fno}");
            assert_eq!(row.kite, stem, "{fno}: Kite writes the indexSymbol");
            assert_eq!(index_by_store_key(key), Some(row), "{key}");
            assert_eq!(row.listed_as(), fno);
            assert!(!row.reference_only, "{fno}");
        }
    }

    /// Every name a reader might type finds its row, whatever the case or
    /// separators, and a name that is not one finds nothing.
    #[test]
    fn every_spelling_of_an_index_finds_its_row_and_nothing_else_does() {
        for (asked, key) in [
            ("nifty fin service", "NIFTYFINSERVICE"),
            ("NIFTY FINANCIAL SERVICES", "NIFTYFINSERVICE"),
            ("FINNIFTY", "NIFTYFINSERVICE"),
            ("NIFTYFINSERVICE", "NIFTYFINSERVICE"),
            ("Nifty 50", "NIFTY"),
            ("NIFTY50", "NIFTY"),
            ("NIFTY", "NIFTY"),
            ("nifty bank", "BANKNIFTY"),
            ("BANKNIFTY", "BANKNIFTY"),
            ("Nifty Midcap Select", "NIFTYMIDSELECT"),
            ("MIDCPNIFTY", "NIFTYMIDSELECT"),
            ("NIFTYNXT50", "NIFTYNEXT50"),
            ("nifty-next-50", "NIFTYNEXT50"),
            ("NIFTY INDIA FPI 150", "NIFTYFPI150"),
            ("NIFTYFPI", "NIFTYFPI150"),
            ("india vix", "INDIAVIX"),
        ] {
            assert_eq!(
                index_by_name(asked).map(|row| row.store_key),
                Some(key),
                "{asked:?}"
            );
        }
        for asked in [
            "",
            "  ",
            "-",
            "NIFT",
            "NIFTY 500",
            "BANK",
            "NIFTY FINANCIAL SERVICES 25/50",
            "RELIANCE",
            "NIFTYFINSERVICEX",
        ] {
            assert_eq!(index_by_name(asked), None, "{asked:?}");
        }
    }

    /// The production collapse agrees with the oracle, and refuses what names
    /// no row before it walks it: nothing to read, more letters than a symbol
    /// holds, or more bytes than [`MAX_NAME_BYTES`].
    #[test]
    fn a_name_is_collapsed_on_the_stack_and_refused_past_its_bounds() {
        let mut buf = [0u8; crate::symbol::SYMBOL_CAPACITY];
        for name in ["NIFTY 50", "nifty-fin service", "ARE&M", "a1"] {
            assert_eq!(
                collapse_into(name, &mut buf).map(str::to_owned),
                Some(collapse(name)),
                "{name}"
            );
        }
        assert_eq!(collapse_into("", &mut buf), None);
        assert_eq!(collapse_into(" - & ", &mut buf), None);
        let full = "A".repeat(crate::symbol::SYMBOL_CAPACITY);
        assert_eq!(collapse_into(&full, &mut buf), Some(full.as_str()));
        let over = "A".repeat(crate::symbol::SYMBOL_CAPACITY + 1);
        assert_eq!(collapse_into(&over, &mut buf), None);
        let padded = format!("{}NIFTY", " ".repeat(MAX_NAME_BYTES - 5));
        assert_eq!(collapse_into(&padded, &mut buf), Some("NIFTY"));
        let too_long = format!("{}NIFTY", " ".repeat(MAX_NAME_BYTES - 4));
        assert_eq!(collapse_into(&too_long, &mut buf), None);
        assert_eq!(index_by_name(&too_long), None);
        assert!(same_name("NIFTY 50", "nifty50"));
        assert!(!same_name("NIFTY 50", "NIFTY 500"));
    }

    /// The written-out names agree with the rows both ways, so the probe table
    /// over them answers exactly what comparing every name would.
    #[test]
    fn every_name_of_every_row_is_collapsed_here_and_nothing_else_is() {
        for (at, row) in INDEX_ROWS.iter().enumerate() {
            for name in row.names() {
                let want = collapse(name);
                assert!(
                    INDEX_NAMES
                        .iter()
                        .any(|&(held, of)| held == want && of == at),
                    "{name} of row {at} is not listed as {want}"
                );
            }
        }
        for (at, &(held, of)) in INDEX_NAMES.iter().enumerate() {
            let row = INDEX_ROWS.get(of).expect("a row");
            assert!(
                row.names().into_iter().any(|name| collapse(name) == held),
                "{held} is no name of row {of}"
            );
            for &(later, _) in &INDEX_NAMES[at + 1..] {
                assert_ne!(held, later, "a name twice");
            }
            assert_eq!(index_by_name(held), Some(row));
        }
        assert_eq!(INDICES.names().len(), INDEX_NAMES.len());
    }

    /// **DB-G54: THE SEGMENT COMES FROM THIS TABLE, NOT FROM FIVE NAMES.**
    /// NIFTYIT, NIFTYFPI and NIFTYCPSE are index options; they were looked
    /// up under `CASH` as shares.
    #[test]
    fn an_index_option_underlying_reads_index_under_its_store_key() {
        for (asked, key) in [
            ("NIFTY", "NIFTY"),
            ("BANKNIFTY", "BANKNIFTY"),
            ("FINNIFTY", "NIFTYFINSERVICE"),
            ("NIFTYFINSERVICE", "NIFTYFINSERVICE"),
            ("MIDCPNIFTY", "NIFTYMIDSELECT"),
            ("NIFTYNXT50", "NIFTYNEXT50"),
            ("NIFTYIT", "NIFTYIT"),
            ("NIFTYFPI", "NIFTYFPI150"),
            ("NIFTYFPI150", "NIFTYFPI150"),
            ("NIFTYCPSE", "NIFTYCPSE"),
        ] {
            let got = option_underlying(asked);
            assert_eq!(got.segment, Segment::Index, "{asked}");
            assert_eq!(got.key, key, "{asked}");
            match got.via {
                Via::Index(row) => assert_eq!(row.store_key, key),
                other => panic!("{asked}: {other:?}"),
            }
        }
    }

    #[test]
    fn every_fno_index_underlying_resolves_as_an_index() {
        for name in FNO_INDEX_UNDERLYINGS {
            assert_eq!(option_underlying(name).segment, Segment::Index, "{name}");
            assert!(FNO_INDEX.contains(name));
        }
        // And the F&O shares stay shares: every one of the 208 is CASH.
        let mut shares = 0;
        for name in FNO_UNDERLYINGS {
            if FNO_INDEX_UNDERLYINGS.iter().all(|&index| index != name) {
                assert_eq!(option_underlying(name).segment, Segment::Cash, "{name}");
                shares += 1;
            }
        }
        assert_eq!(shares, FNO_UNDERLYINGS.len() - FNO_INDEX_UNDERLYINGS.len());
    }

    /// A renamed share reads the newest key its chain reaches; a share with no
    /// row reads its own; HDFC and MINDTREE, which have no row, are not given
    /// a successor.
    #[test]
    fn a_share_option_underlying_reads_cash_under_the_newest_symbol() {
        for (asked, key, first_new) in [
            ("TATAMOTORS", "TMPV", "TMPV"),
            ("ZOMATO", "ETERNAL", "ETERNAL"),
            ("LTI", "LTM", "LTIM"),
            ("LTIM", "LTM", "LTM"),
            ("MCDOWELL-N", "UNITDSPR", "UNITDSPR"),
            ("L&TFH", "LTF", "LTF"),
            ("NIITTECH", "COFORGE", "COFORGE"),
        ] {
            let got = option_underlying(asked);
            assert_eq!(got.segment, Segment::Cash, "{asked}");
            assert_eq!(got.key, key, "{asked}");
            match got.via {
                Via::Renamed(hop) => {
                    assert_eq!(hop.old, asked);
                    assert_eq!(hop.new, first_new);
                }
                other => panic!("{asked}: {other:?}"),
            }
        }
        for own in [
            "RELIANCE",
            "HDFC",
            "MINDTREE",
            "TMPV",
            "LTM",
            "ETERNAL",
            "NIFTYBEES",
        ] {
            let got = option_underlying(own);
            assert_eq!(
                got,
                Underlying {
                    segment: Segment::Cash,
                    key: own,
                    via: Via::Own
                },
                "{own}"
            );
        }
        assert!(not_aliased("HDFC").is_some());
        assert!(not_aliased("MINDTREE").is_some());
        assert_eq!(not_aliased("RELIANCE"), None);
    }

    /// "Formerly X until D", in the order a reader reads it.
    #[test]
    fn the_lineage_walks_forward_and_backward_through_a_chain() {
        let forward: Vec<_> = LINEAGE.forward("LTI").map(|c| (c.old, c.new)).collect();
        assert_eq!(forward, [("LTI", "LTIM"), ("LTIM", "LTM")]);
        let backward: Vec<_> = LINEAGE
            .backward("LTM")
            .map(|c| (c.old, c.effective.to_string()))
            .collect();
        assert_eq!(
            backward,
            [
                ("LTIM", "2026-02-27".to_owned()),
                ("LTI", "2022-12-05".to_owned())
            ]
        );
        assert_eq!(LINEAGE.current("LTI"), "LTM");
        assert_eq!(LINEAGE.current("RELIANCE"), "RELIANCE");
        assert_eq!(LINEAGE.forward("RELIANCE").count(), 0);
        assert_eq!(LINEAGE.backward("ZOMATO").count(), 0);
        let eternal: Vec<_> = LINEAGE.backward("ETERNAL").collect();
        assert_eq!(eternal.len(), 1);
        assert_eq!(eternal[0].old, "ZOMATO");
        assert_eq!(
            eternal[0].status.citations(),
            &[SYMBOL_CHANGE_CSV, CML67420]
        );
    }

    /// The importer's question (D-3333): which key does a file named `stem`
    /// on `day` go under.
    #[test]
    fn a_stem_is_filed_under_the_newest_key_only_on_a_day_it_was_live() {
        let day = Ymd::new;
        assert_eq!(LINEAGE.filed_under("LTI", day(2020, 1, 1)), Ok("LTM"));
        assert_eq!(LINEAGE.filed_under("LTI", day(2022, 12, 2)), Ok("LTM"));
        assert_eq!(
            LINEAGE.filed_under("LTI", day(2022, 12, 5)),
            Err(Stale::AfterChange(SYMBOL_CHANGES[5]))
        );
        assert_eq!(LINEAGE.filed_under("LTIM", day(2024, 1, 1)), Ok("LTM"));
        assert_eq!(
            LINEAGE.filed_under("LTIM", day(2021, 1, 1)),
            Err(Stale::BeforeChange(SYMBOL_CHANGES[5]))
        );
        assert_eq!(
            LINEAGE.filed_under("LTIM", day(2026, 2, 27)),
            Err(Stale::AfterChange(SYMBOL_CHANGES[17]))
        );
        assert_eq!(LINEAGE.filed_under("LTM", day(2026, 2, 27)), Ok("LTM"));
        assert_eq!(
            LINEAGE.filed_under("LTM", day(2026, 2, 26)),
            Err(Stale::BeforeChange(SYMBOL_CHANGES[17]))
        );
        assert_eq!(
            LINEAGE.filed_under("ZOMATO", day(2025, 4, 8)),
            Ok("ETERNAL")
        );
        assert_eq!(LINEAGE.filed_under("HDFC", day(2020, 1, 1)), Ok("HDFC"));
        assert_eq!(
            LINEAGE.filed_under("RELIANCE", day(2030, 1, 1)),
            Ok("RELIANCE")
        );
    }

    /// **A ROW WITHOUT A SOURCE IS LISTED AND NEVER APPLIED.** Proven on a
    /// table built here, because the shipped table has no unverified row.
    #[test]
    fn an_unverified_row_is_listed_and_never_applied() {
        const UNSOURCED: Status = Status::Unverified("no exchange source read");
        let lineage: Lineage<2, 8> = Lineage::build([
            SymbolChange {
                old: "AAA",
                new: "BBB",
                effective: Ymd::new(2020, 1, 1),
                status: UNSOURCED,
            },
            SymbolChange {
                old: "CCC",
                new: "DDD",
                effective: Ymd::new(2021, 1, 1),
                status: Status::Sourced(IN_THE_FILE),
            },
        ]);
        assert_eq!(lineage.rows().len(), 2);
        assert_eq!(lineage.from_old("AAA"), None);
        assert_eq!(lineage.to_new("BBB"), None);
        assert_eq!(lineage.current("AAA"), "AAA");
        assert_eq!(lineage.backward("BBB").count(), 0);
        assert_eq!(lineage.filed_under("AAA", Ymd::new(2025, 1, 1)), Ok("AAA"));
        assert_eq!(lineage.filed_under("BBB", Ymd::new(2019, 1, 1)), Ok("BBB"));
        assert_eq!(lineage.current("CCC"), "DDD");
        assert_eq!(lineage.to_new("DDD").map(|c| c.old), Some("CCC"));

        let indices: Indices<2, 4, 16> = Indices::build(
            [
                IndexRow {
                    fno: Some("XIDX"),
                    nse_name: "X INDEX",
                    index_symbol: "X IDX",
                    kite: "X IDX",
                    store_key: "XIDX",
                    reference_only: false,
                    status: UNSOURCED,
                },
                IndexRow {
                    fno: None,
                    nse_name: "Y INDEX",
                    index_symbol: "Y IDX",
                    kite: "Y IDX",
                    store_key: "YIDX",
                    reference_only: true,
                    status: Status::Sourced(VIX_SOURCES),
                },
            ],
            [("XIDX", 0), ("XINDEX", 0), ("YIDX", 1), ("YINDEX", 1)],
        );
        assert_eq!(indices.rows().len(), 2);
        assert_eq!(indices.by_fno("XIDX"), None);
        assert_eq!(indices.by_store_key("XIDX"), None);
        assert_eq!(indices.by_name("X INDEX"), None);
        assert_eq!(indices.by_name("Z INDEX"), None);
        assert_eq!(indices.by_fno(""), None);
        assert_eq!(
            indices.by_store_key("YIDX").map(|r| r.nse_name),
            Some("Y INDEX")
        );
        assert_eq!(indices.by_name("y idx").map(|r| r.store_key), Some("YIDX"));
        assert!(!UNSOURCED.applies());
        assert_eq!(UNSOURCED.word(), "UNVERIFIED");
        assert!(UNSOURCED.citations().is_empty());
    }

    /// **A CYCLE CANNOT LOOP.** A table that maps a symbol back to itself
    /// stops after `R` hops.
    #[test]
    fn a_cycle_in_a_table_stops_after_its_length() {
        let cyclic: Lineage<2, 8> = Lineage::build([
            change("PPP", "QQQ", Ymd::new(2020, 1, 1)),
            change("QQQ", "PPP", Ymd::new(2021, 1, 1)),
        ]);
        assert_eq!(cyclic.forward("PPP").count(), 2);
        assert_eq!(cyclic.backward("PPP").count(), 2);
        assert_eq!(cyclic.current("PPP"), "PPP");
    }

    /// Every shipped row is sourced, cites the charter's sources, and is
    /// reachable through its own probe table; no old or new symbol repeats;
    /// no chain is a cycle; and every date is a real day.
    #[test]
    fn the_shipped_lineage_has_no_cycle_and_no_duplicate() {
        for (at, row) in SYMBOL_CHANGES.iter().enumerate() {
            assert!(row.status.applies(), "{row:?}");
            assert!(!row.status.citations().is_empty());
            assert_eq!(LINEAGE.from_old(row.old), Some(&SYMBOL_CHANGES[at]));
            assert_eq!(LINEAGE.to_new(row.new), Some(&SYMBOL_CHANGES[at]));
            assert_ne!(row.old, row.new);
            assert!(crate::symbol::Symbol::new(row.old).is_ok(), "{}", row.old);
            assert!(crate::symbol::Symbol::new(row.new).is_ok(), "{}", row.new);
            assert!(row.forward_is_finite());
            let Ymd { year, month, day } = row.effective;
            assert!((2000..=2100).contains(&year));
            assert!((1..=12).contains(&month));
            let days = match month {
                2 if year % 4 == 0 => 29,
                2 => 28,
                4 | 6 | 9 | 11 => 30,
                _ => 31,
            };
            assert!((1..=days).contains(&day), "{row:?}");
            for later in &SYMBOL_CHANGES[at + 1..] {
                assert_ne!(row.old, later.old, "an old symbol twice");
                assert_ne!(row.new, later.new, "a new symbol twice");
                assert!(row.effective <= later.effective, "date order");
            }
        }
        assert_eq!(LINEAGE.rows().len(), SYMBOL_CHANGES.len());
    }

    impl SymbolChange {
        /// The chain from this row's old symbol ends before the table's
        /// length runs out.
        fn forward_is_finite(&self) -> bool {
            LINEAGE.forward(self.old).count() < SYMBOL_CHANGES.len()
        }
    }

    /// Every shipped index row is sourced, its store key is a valid symbol and
    /// the `indexSymbol` with its spaces removed, except the two indices the
    /// store renamed; no two rows share a name.
    #[test]
    fn the_shipped_index_rows_are_sourced_distinct_and_keyed_as_measured() {
        for (at, row) in INDEX_ROWS.iter().enumerate() {
            assert!(row.status.applies(), "{row:?}");
            assert!(!row.status.citations().is_empty());
            assert!(crate::symbol::Symbol::new(row.store_key).is_ok());
            assert_eq!(index_by_store_key(row.store_key), Some(&INDEX_ROWS[at]));
            if let Some(fno) = row.fno {
                assert_eq!(index_by_fno(fno), Some(&INDEX_ROWS[at]));
                assert!(crate::symbol::Symbol::new(fno).is_ok());
            }
            let collapsed: String = row.index_symbol.split(' ').collect();
            let renamed = matches!(row.store_key, "NIFTY" | "BANKNIFTY");
            assert_eq!(collapsed == row.store_key, !renamed, "{row:?}");
            for later in &INDEX_ROWS[at + 1..] {
                for mine in row.names() {
                    for theirs in later.names() {
                        assert!(!same_name(mine, theirs), "{mine} and {theirs}");
                    }
                }
            }
        }
        assert_eq!(INDICES.rows().len(), INDEX_ROWS.len());
        assert_eq!(index_by_fno("RELIANCE"), None);
        assert_eq!(index_by_store_key("FINNIFTY"), None);
    }

    /// **THE ZERODHA RENAME IS THIS TABLE'S PROJECTION.**
    /// `Vendor::index_alias` renames exactly the rows whose Kite name,
    /// collapsed, is not the store key, to that key; and
    /// `Vendor::index_alias_source` undoes exactly those. Two `match`es in
    /// `vendor.rs` and one table here, held to one answer.
    #[test]
    fn the_zerodha_rename_is_exactly_the_rows_whose_kite_name_is_not_the_key() {
        for row in INDEX_ROWS {
            let kite: String = row.kite.split(' ').collect();
            let renamed = kite != row.store_key;
            assert_eq!(
                Vendor::Zerodha.index_alias(&kite),
                renamed.then_some(row.store_key),
                "{row:?}"
            );
            assert_eq!(
                Vendor::Zerodha
                    .index_alias_source(row.store_key)
                    .map(str::to_owned),
                renamed.then_some(kite),
                "{row:?}"
            );
        }
    }

    #[test]
    fn the_status_and_feed_words_are_what_the_wire_carries() {
        assert_eq!(Status::Sourced(IN_THE_FILE).word(), "sourced");
        assert!(keys_feed(Vendor::Zerodha));
        assert!(keys_feed(Vendor::Gdfl));
        assert!(!keys_feed(Vendor::Dhan));
        assert!(!keys_feed(Vendor::Groww));
        assert!(!keys_feed(Vendor::TrueData));
        for feed in KEYED_FEEDS {
            assert!(keys_feed(feed));
        }
        assert_eq!(Ymd::new(2025, 4, 9).to_string(), "2025-04-09");
        assert!(Ymd::new(2025, 4, 9) < Ymd::new(2025, 4, 10));
        assert!(Ymd::new(2024, 12, 31) < Ymd::new(2025, 1, 1));
        assert!(CHARTER.contains("D-3332"));
        assert_eq!(DECISION, "D-3332");
        assert!(PRODUCT.starts_with("https://www.nseindia.com/"));
        for row in INDEX_ROWS {
            for cited in row.status.citations() {
                if cited.url.contains("products-services") {
                    assert!(cited.url.starts_with(PRODUCT), "{}", cited.url);
                }
            }
        }
    }

    /// The probe tables are built at run time here as well as by the
    /// compiler, so the build is instrumented, and their worst probe is a
    /// number.
    #[test]
    fn the_tables_build_at_run_time_and_probe_within_a_bound() {
        let indices: Indices<9, 18, 128> = Indices::build(INDEX_ROWS, INDEX_NAMES);
        let lineage: Lineage<18, 128> = Lineage::build(SYMBOL_CHANGES);
        assert_eq!(indices.by_fno("FINNIFTY"), INDICES.by_fno("FINNIFTY"));
        assert_eq!(indices.by_name("finnifty"), INDICES.by_name("FINNIFTY"));
        let refused = not_aliased_symbols(&NOT_ALIASED);
        assert_eq!(refused, ["HDFC", "MINDTREE"]);
        assert_eq!(lineage.from_old("ZOMATO"), LINEAGE.from_old("ZOMATO"));
        let worst = |table: &MemberIndex<128>, members: &[&str]| {
            members
                .iter()
                .map(|m| {
                    let mut at = mask(fnv1a(m), 128);
                    let mut steps = 1;
                    while table.slots[at] != Some(*m) {
                        at = (at + 1) & 127;
                        steps += 1;
                    }
                    steps
                })
                .max()
                .unwrap_or(0)
        };
        let olds: Vec<&str> = SYMBOL_CHANGES.iter().map(|c| c.old).collect();
        let news: Vec<&str> = SYMBOL_CHANGES.iter().map(|c| c.new).collect();
        assert!(worst(&LINEAGE.by_old, &olds) <= 4);
        assert!(worst(&LINEAGE.by_new, &news) <= 4);
    }
}

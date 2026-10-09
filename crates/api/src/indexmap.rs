//! What NSE publishes, joined to what a feed lists.
//!
//! `pull::nseindex` decides how a vendor's symbol meets an exchange name and
//! refuses to guess when it cannot tell. It had no caller. This module is the
//! caller: it reads the operator's NSE catalogue, joins a feed's index symbols
//! against it, and serves the answer — **including the refusals**, which are
//! the half an operator actually needs, because a symbol that did not resolve
//! is a symbol whose bars are being filed under a name no exchange confirms.
//!
//! The join is computed per request over the ~136 index symbols a feed lists,
//! not over the ~800k rows of its master. The masters are parsed once at
//! startup; this walks what that parse already produced.
//!
//! # A feed with no master lists what its census holds (DB-G26)
//!
//! GDFL publishes no instrument master, so the master walk found nothing and
//! `/indexmap.json?feed=gdfl` answered `listed 0, rows []` while the store held
//! GDFL's NIFTY and BANKNIFTY. For a feed with no master the symbols are the
//! store keys its census holds under `INDEX`, and the body says
//! `"listed_from":"census"`. A census that will not load is refused by name
//! (`503`), never answered as an empty feed.
//!
//! # The sourced alias table beats a guess (D-3332)
//!
//! For a feed whose store keys `brutex_core::alias` records, a key the table
//! names is joined through the table's NSE name, verbatim, and the row says so
//! with the table's citation. `NIFTYFINSERVICE` was refused as ambiguous among
//! three published names; the table, resting on NSE's FINNIFTY product page,
//! names `NIFTY FINANCIAL SERVICES`, and the three the abbreviation rule alone
//! could not choose between still travel beside it.
//!
//! # An ambiguous row names its candidates (DB-G27)
//!
//! The page promised the candidates and the body carried only their count. Each
//! ambiguous row now carries `candidate_names`, in the catalogue file's own
//! order, chosen by `pull::nseindex::accepts`, the rule the resolver applies.

use brutex_core::alias::{self, IndexRow};
use brutex_core::vendor::Vendor;
use pull::nseindex::{Basis, Catalogue, Unresolved, accepts, collapse};
use std::fmt::Write as _;
use std::path::Path;

/// The largest catalogue file [`Published::read`] reads, in bytes.
///
/// NSE's index list is about 150 `index_name,category` rows, a few KiB. One
/// MiB is two orders of magnitude past that and still a bound: the file is read
/// on every `/indexmap.json` request, and it had none (UC-19, D-1502). A file
/// past it is refused naming the size, the way `master::MAX_MASTER_BYTES` is,
/// instead of being read whole into memory.
pub const MAX_CATALOGUE_BYTES: u64 = 1024 * 1024;

/// NSE's published index list — matchable, and still readable.
///
/// [`Catalogue`] collapses every name so a match can ignore the separators
/// vendors disagree about, and hands back the name **as NSE wrote it**. That
/// second half is what makes this type worth having: an operator reading a
/// refusal needs `Nifty Private Bank`, not `NIFTYPRIVATEBANK`, and carrying the
/// readable form inside the catalogue rather than in a map beside it means
/// there is no second lookup that could miss.
#[derive(Clone, Debug, Default)]
pub struct Published {
    matcher: Catalogue,
    /// Every published name, collapsed and as NSE wrote it, once each, in the
    /// file's order: what an ambiguous row's candidates are read from, so they
    /// come out in the same order on every request without a sort.
    names: Vec<(String, String)>,
}

impl Published {
    /// Read the operator's catalogue file — `index_name,category` rows.
    ///
    /// # Errors
    ///
    /// The path and the reason it could not be read. There is no default and
    /// no empty fallback: a join against nothing resolves nothing and would
    /// report 136 refusals as though the exchange had disowned them, which is
    /// the failure wearing a success's clothes `CLAUDE.md` §4 bans.
    /// A file larger than [`MAX_CATALOGUE_BYTES`] is refused naming the bound.
    /// The read itself is capped one byte past it, so a file that grows between
    /// a size check and the read cannot slip past (UC-19, D-1502).
    pub fn read(path: &Path) -> Result<Self, String> {
        use std::io::Read as _;
        let file = std::fs::File::open(path).map_err(|why| format!("{}: {why}", path.display()))?;
        let mut text = String::new();
        file.take(MAX_CATALOGUE_BYTES.saturating_add(1))
            .read_to_string(&mut text)
            .map_err(|why| format!("{}: {why}", path.display()))?;
        if u64::try_from(text.len()).unwrap_or(u64::MAX) > MAX_CATALOGUE_BYTES {
            return Err(format!(
                "{}: larger than {MAX_CATALOGUE_BYTES} bytes, which this reader \
                 holds at most; NSE's index list is a few KiB, so this is not it",
                path.display()
            ));
        }
        let parsed = Self::from_text(&text);
        if parsed.is_empty() {
            return Err(format!("{}: no index names", path.display()));
        }
        Ok(parsed)
    }

    /// Split from [`Self::read`] so the parse is testable without a file.
    ///
    /// The category column is dropped on the LAST comma rather than the first,
    /// so a published name containing one survives.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let names: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("index_name"))
            .map(|line| line.rsplit_once(',').map_or(line, |(name, _)| name))
            .collect();
        let mut seen = std::collections::HashSet::with_capacity(names.len());
        let ordered = names
            .iter()
            .map(|name| (collapse(name), (*name).to_owned()))
            .filter(|(collapsed, _)| !collapsed.is_empty() && seen.insert(collapsed.clone()))
            .collect();
        Self {
            matcher: Catalogue::from_names(names),
            names: ordered,
        }
    }

    /// The published names that accept `symbol` as an abbreviation, as NSE
    /// writes them, in the catalogue file's order. The rule is
    /// `pull::nseindex::accepts`, the one [`Catalogue::resolve`] counts with,
    /// so their number is the count an [`Unresolved::Ambiguous`] carries.
    ///
    /// A walk of the catalogue, the same walk the resolve that found the
    /// ambiguity already paid.
    #[must_use]
    pub fn candidates(&self, symbol: &str) -> Vec<String> {
        let key = collapse(symbol);
        self.names
            .iter()
            .filter(|(collapsed, _)| accepts(&key, collapsed))
            .map(|(_, written)| written.clone())
            .collect()
    }

    /// How many distinct indices the exchange publishes here.
    #[must_use]
    pub fn len(&self) -> usize {
        self.matcher.len()
    }

    /// Whether the list holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.matcher.is_empty()
    }
}

/// One feed symbol and what the exchange says about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The symbol as the feed lists it.
    pub symbol: String,
    /// The NSE name it resolved to, as NSE writes it.
    pub nse: Option<String>,
    /// How strong that resolution is.
    pub basis: Option<Basis>,
    /// Why it did not resolve.
    pub why: Option<Unresolved>,
    /// The sourced alias row the resolution went through, when it did.
    pub alias: Option<&'static IndexRow>,
    /// What the feed's symbol alone resolves to, for a row the alias table
    /// resolved, when that is not the table's answer: `None` when the two
    /// agree.
    pub unaided: Option<Unaided>,
    /// The published names an ambiguous answer is ambiguous among, as NSE
    /// writes them, in the catalogue's order. Empty unless `why` or `unaided`
    /// is ambiguous.
    pub candidates: Vec<String>,
}

/// What the feed's symbol resolves to without the alias table, when that
/// differs from the table's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unaided {
    /// It resolves, to another published name: `NIFTY` abbreviates
    /// `NIFTY BANK`, which is not the index the store keys `NIFTY`.
    Differs(String, Basis),
    /// It does not resolve: ambiguous or absent.
    Refused(Unresolved),
}

impl Row {
    /// A row with an answer and nothing beside it.
    fn resolved(symbol: String, name: &str, basis: Basis) -> Self {
        Self {
            symbol,
            nse: Some(name.to_owned()),
            basis: Some(basis),
            why: None,
            alias: None,
            unaided: None,
            candidates: Vec::new(),
        }
    }
}

/// Where a feed's index symbols came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Listed {
    /// The feed's instrument master, as merged at startup.
    Master,
    /// The feed's census: every store key it holds under `INDEX`. Taken for a
    /// feed that publishes no master (GDFL, `TrueData`).
    Census,
}

impl Listed {
    /// The wire word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Master => "master",
            Self::Census => "census",
        }
    }
}

/// Join a feed's index symbols to the exchange, sorted by symbol.
///
/// `feed` is here for the rename. The store keys on names this repository
/// chose — `NIFTY` for `NIFTY 50`, `BANKNIFTY` for `NIFTY BANK`,
/// `NIFTYFINSERVICE` for `NIFTY FINANCIAL SERVICES` — and those are what the
/// census hands over, so a join that only sees them cannot find the exchange's
/// own listing. Measured before the rename was undone: `NIFTY` matched 80
/// published names and `BANKNIFTY` matched none, which made **the two
/// instruments the engine sweeps the two it could not resolve.**
///
/// The rename is `brutex_core::alias`'s table (D-3332), applied to a feed
/// whose store keys the table records ([`alias::keys_feed`]). It replaced
/// `Vendor::index_alias_source`, Zerodha's two renames, which the table holds
/// as two of its rows and pins to one answer
/// (`brutex_core::alias`'s `the_zerodha_rename_is_exactly_the_rows_whose_kite_name_is_not_the_key`).
#[must_use]
pub fn join<I, S>(nse: &Published, feed: Vendor, symbols: I) -> Vec<Row>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut rows: Vec<Row> = symbols
        .into_iter()
        .map(|symbol| {
            let symbol = symbol.as_ref().to_owned();
            // THE RENAME IS TRIED FIRST, and the order is the whole point.
            //
            // It was a fallback for one commit, reached only when the direct
            // resolve refused, and a test caught what that costs: `NIFTY`
            // abbreviates `NIFTY BANK` — it is an ordered subsequence with no
            // digits on either side — so a catalogue where that is the only
            // candidate hands back the wrong index and the rename is never
            // consulted. A stated fact must beat a guess, not queue behind one.
            //
            // Only a VERBATIM match is accepted through here. Following the
            // rename costs nothing because the rename is known; letting the
            // renamed-from name then resolve by abbreviation would stack
            // inference on inference and file it beside the proofs.
            //
            // What the symbol alone resolves to travels beside the answer when
            // it is not the same, so the reader sees what the table settled.
            //
            // The basis stays `published` when the symbol is the exchange's
            // own name verbatim AND that is the table's answer (NIFTYIT,
            // NIFTYCPSE, NIFTYNEXT50): the exchange's list proves it on its
            // own, and calling it `aliased` would move a proof into the
            // inference column and shrink the verbatim count for no reason.
            // The table's row still travels with it. `aliased` is kept for
            // the rows the table actually decided.
            if let Some(row) = alias::keys_feed(feed)
                .then(|| alias::index_by_store_key(&symbol))
                .flatten()
                && let Ok((name, Basis::Published)) = nse.matcher.resolve(row.nse_name)
            {
                let (unaided, candidates, basis) = match nse.matcher.resolve(&symbol) {
                    Ok((alone, Basis::Published)) if alone == name => {
                        (None, Vec::new(), Basis::Published)
                    }
                    Ok((alone, _)) if alone == name => (None, Vec::new(), Basis::Aliased),
                    Ok((alone, basis)) => (
                        Some(Unaided::Differs(alone.to_owned(), basis)),
                        Vec::new(),
                        Basis::Aliased,
                    ),
                    Err(why @ Unresolved::Ambiguous(_)) => (
                        Some(Unaided::Refused(why)),
                        nse.candidates(&symbol),
                        Basis::Aliased,
                    ),
                    Err(why @ Unresolved::Absent) => {
                        (Some(Unaided::Refused(why)), Vec::new(), Basis::Aliased)
                    }
                };
                return Row {
                    alias: Some(row),
                    unaided,
                    candidates,
                    ..Row::resolved(symbol, name, basis)
                };
            }
            match nse.matcher.resolve(&symbol) {
                Ok((name, basis)) => Row::resolved(symbol, name, basis),
                Err(why) => Row {
                    candidates: match why {
                        Unresolved::Ambiguous(_) => nse.candidates(&symbol),
                        Unresolved::Absent => Vec::new(),
                    },
                    symbol,
                    nse: None,
                    basis: None,
                    why: Some(why),
                    alias: None,
                    unaided: None,
                },
            }
        })
        .collect();
    rows.sort_unstable_by(|a, b| a.symbol.cmp(&b.symbol));
    rows.dedup_by(|a, b| a.symbol == b.symbol);
    rows
}

/// How a basis appears on the wire.
fn basis_word(basis: Basis) -> &'static str {
    match basis {
        Basis::Published => "published",
        Basis::Abbreviation => "abbreviation",
        Basis::Aliased => "aliased",
    }
}

/// How a refusal appears on the wire.
fn refusal_word(why: Unresolved) -> &'static str {
    match why {
        Unresolved::Ambiguous(_) => "ambiguous",
        Unresolved::Absent => "absent",
    }
}

/// The join, as JSON.
///
/// Counts lead so a reader sees the shape before the rows, and every row
/// carries either a resolution with its basis or a refusal with its reason.
/// **No row is omitted** — a symbol the exchange does not confirm is the one
/// an operator most needs to see, so it is not filtered out for being
/// unanswerable.
///
/// `listed_from` says where the symbols came from ([`Listed`]); `census_note`
/// is present when they came from a census that loaded DEGRADED, in its own
/// words. A row resolved through the alias table carries the table's row and
/// citation as `alias`, and what the symbol alone resolves to as `unaided`
/// when that is not the table's answer. An ambiguous answer, either way,
/// carries its `candidate_names` beside the `candidates` count.
#[must_use]
pub fn json(nse: &Published, listed: Listed, census_note: Option<&str>, rows: &[Row]) -> String {
    let resolved = rows.iter().filter(|row| row.nse.is_some()).count();
    let by = |want: Basis| rows.iter().filter(|row| row.basis == Some(want)).count();
    let aliased = by(Basis::Aliased);
    let mut out = String::new();
    let _ = write!(
        out,
        concat!(
            "{{\"published\":{},\"listed\":{},\"resolved\":{},",
            "\"verbatim\":{},\"aliased\":{},\"abbreviated\":{},\"refused\":{},",
            "\"listed_from\":\"{}\""
        ),
        nse.len(),
        rows.len(),
        resolved,
        by(Basis::Published),
        aliased,
        by(Basis::Abbreviation),
        rows.len() - resolved,
        listed.word(),
    );
    if let Some(note) = census_note {
        let _ = write!(
            out,
            ",\"census_note\":{}",
            crate::pullrun::quote_for_json(note)
        );
    }
    out.push_str(",\"rows\":[");
    for (position, row) in rows.iter().enumerate() {
        if position > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{{\"symbol\":{}",
            crate::pullrun::quote_for_json(&row.symbol)
        );
        if let (Some(name), Some(basis)) = (row.nse.as_ref(), row.basis) {
            let _ = write!(
                out,
                ",\"nse\":{},\"basis\":\"{}\"",
                crate::pullrun::quote_for_json(name),
                basis_word(basis)
            );
        }
        if let Some(table) = row.alias {
            write_alias(&mut out, table);
        }
        match row.unaided {
            Some(Unaided::Differs(ref alone, basis)) => {
                let _ = write!(
                    out,
                    ",\"unaided\":{{\"nse\":{},\"basis\":\"{}\"}}",
                    crate::pullrun::quote_for_json(alone),
                    basis_word(basis)
                );
            }
            Some(Unaided::Refused(why)) => {
                out.push_str(",\"unaided\":{");
                write_refusal(&mut out, why, &row.candidates);
                out.push('}');
            }
            None => {}
        }
        if let Some(why) = row.why {
            out.push_str(",\"nse\":null,");
            write_refusal(&mut out, why, &row.candidates);
        }
        out.push('}');
    }
    out.push_str("]}");
    out
}

/// `"why":…`, and for an ambiguity its count and its names.
fn write_refusal(out: &mut String, why: Unresolved, candidates: &[String]) {
    let _ = write!(out, "\"why\":\"{}\"", refusal_word(why));
    if let Unresolved::Ambiguous(count) = why {
        let _ = write!(out, ",\"candidates\":{count},\"candidate_names\":[");
        for (position, name) in candidates.iter().enumerate() {
            if position > 0 {
                out.push(',');
            }
            out.push_str(&crate::pullrun::quote_for_json(name));
        }
        out.push(']');
    }
}

/// The alias row a resolution went through, with where it comes from.
fn write_alias(out: &mut String, table: &IndexRow) {
    let fno = table
        .fno
        .map_or_else(|| "null".to_owned(), crate::pullrun::quote_for_json);
    let _ = write!(
        out,
        concat!(
            ",\"alias\":{{\"fno\":{},\"nse_name\":{},\"index_symbol\":{},",
            "\"store_key\":{},\"status\":\"{}\",\"source\":{},\"decision\":\"{}\"}}"
        ),
        fno,
        crate::pullrun::quote_for_json(table.nse_name),
        crate::pullrun::quote_for_json(table.index_symbol),
        crate::pullrun::quote_for_json(table.store_key),
        table.status.word(),
        crate::pullrun::quote_for_json(alias::CHARTER),
        alias::DECISION,
    );
}

/// The store keys a census holds under `INDEX`, once each, in the census's
/// own walk order (the join sorts them).
///
/// # Errors
///
/// The census's own note when it did not load: an absent or unreadable
/// census lists nothing, and saying so is the answer, not an empty feed.
///
/// # Cost
///
/// One walk of every key the census holds: O(entries). Measured on the shadow
/// store on 9 Oct 2026, GDFL's census holding 246,147 instrument-months: the
/// first `/indexmap.json?feed=gdfl` after the census had loaded answered in
/// 5.4 ms, the kept answers after it in 0.7 to 2.9 ms, so the walk costs a few
/// milliseconds today and grows with every month the options import adds. So
/// the route does not call this per request: [`HeldIndexKeeper`] keeps the
/// answer per census commit and pays this walk once per commit.
pub fn held_index_symbols(census: &crate::census::VendorCensus) -> Result<Vec<String>, String> {
    match census.state {
        crate::census::Census::Held { ref manifest } => {
            let mut seen = std::collections::HashSet::with_capacity(64);
            Ok(manifest
                .held_keys()
                .filter(|key| {
                    key.segment == brutex_core::instrument::Segment::Index
                        && key.contract.is_none()
                        && seen.insert(key.symbol)
                })
                .map(|key| key.symbol.as_str().to_owned())
                .collect())
        }
        crate::census::Census::Absent | crate::census::Census::Unreadable { .. } => {
            Err(census.note())
        }
    }
}

/// What one census commit is, for keeping something derived from it: the
/// manifest's path, its generation, its three counters (months, rows,
/// entries) and the modified time the `stat` taken BEFORE the census was read
/// found (`CensusStamps::modified`, the key `calendar_of::cached` keeps a
/// calendar under, D-0695).
///
/// The counters and the generation alone are not a commit's identity: a
/// manifest replaced whole by one with the same generation and the same counts
/// but other keys read as the same commit, and the keys kept from the first
/// were served for the second. A test met exactly that. The modified time is
/// what a write moves, so the two are told apart; a census whose stamp found
/// no time is walked and never kept.
type CommitKey = (
    std::path::PathBuf,
    u64,
    (u64, u64, u64),
    std::time::SystemTime,
);

/// One kept answer per vendor: the commit it was derived from and the `INDEX`
/// keys it held.
type Kept = Option<(CommitKey, std::sync::Arc<Vec<String>>)>;

/// [`held_index_symbols`], kept per census commit ([`CommitKey`]), one slot
/// per `Vendor::ALL` entry.
///
/// A request whose census is the commit already kept costs one lock, one
/// comparison of the key and a reference-count bump; a new commit pays the
/// walk once and replaces the slot. The key is read off the census the
/// request already holds, so a kept answer is never older than the census the
/// rest of the body is built from. A census that did not load is answered by
/// its note every time and never kept.
///
/// A type rather than a bare static so a test holds its own and cannot race
/// another test for a slot.
pub struct HeldIndexKeeper {
    slots: std::sync::Mutex<[Kept; Vendor::ALL.len()]>,
}

impl HeldIndexKeeper {
    /// Nothing kept yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: std::sync::Mutex::new([const { None }; Vendor::ALL.len()]),
        }
    }

    /// The `INDEX` keys `census` holds, from the kept slot when it is this
    /// commit's, else walked once and kept. `modified` is the manifest's
    /// modified time as the stamps the census was read under found it
    /// (`CensusStamps::modified`); `None` walks and keeps nothing.
    ///
    /// # Errors
    ///
    /// As [`held_index_symbols`].
    pub fn symbols(
        &self,
        census: &crate::census::VendorCensus,
        modified: Option<std::time::SystemTime>,
    ) -> Result<std::sync::Arc<Vec<String>>, String> {
        let (Some(generation), Some(counters), Some(modified)) =
            (census.generation(), census.counters(), modified)
        else {
            return held_index_symbols(census).map(std::sync::Arc::new);
        };
        let key: CommitKey = (census.path.clone(), generation, counters, modified);
        let slot = census.vendor as usize;
        // A poisoned lock means a thread panicked while replacing a slot; a
        // slot is a whole value, so it still holds one commit's answer or
        // none, and the key comparison decides whether it is this one.
        let kept = self
            .slots
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(slot)
            .and_then(Option::as_ref)
            .filter(|(at, _)| *at == key)
            .map(|(_, symbols)| std::sync::Arc::clone(symbols));
        if let Some(symbols) = kept {
            return Ok(symbols);
        }
        let symbols = std::sync::Arc::new(held_index_symbols(census)?);
        if let Some(held) = self
            .slots
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_mut(slot)
        {
            *held = Some((key, std::sync::Arc::clone(&symbols)));
        }
        Ok(symbols)
    }
}

impl Default for HeldIndexKeeper {
    fn default() -> Self {
        Self::new()
    }
}

/// The process's keeper, which `/indexmap.json` asks.
static HELD_INDEX: HeldIndexKeeper = HeldIndexKeeper::new();

/// Everything `/indexmap.json` answers once the catalogue has been read.
///
/// A feed that publishes a master is joined over `master`, the symbols the
/// server's walk of the merged universe found for it. A feed that publishes
/// none is joined over the `INDEX` keys its census holds
/// ([`held_index_symbols`]); a census that did not load is refused `503`
/// naming why, and a degraded one is listed with its note on the wire.
pub fn answer(
    site: &crate::server::Site,
    nse: &Published,
    feed: Vendor,
    master: Vec<&str>,
) -> (axum::http::StatusCode, String) {
    if feed.publishes_master() {
        let rows = join(nse, feed, master);
        return (
            axum::http::StatusCode::OK,
            json(nse, Listed::Master, None, &rows),
        );
    }
    // Through the one-vendor interface later packages code against (WP-01),
    // so a per-vendor census cache (WP-30) changes what a miss costs here
    // without changing this caller.
    let (stamps, now) = crate::server::vendor_census(site, feed);
    let Some(census) = now.census() else {
        return census_refused(&format!(
            "{}: the census read returned no row for this feed",
            feed.as_str()
        ));
    };
    match HELD_INDEX.symbols(census, stamps.modified(census)) {
        Ok(symbols) => {
            let rows = join(nse, feed, symbols.iter());
            let note = census.degraded();
            (
                axum::http::StatusCode::OK,
                json(nse, Listed::Census, note.as_deref(), &rows),
            )
        }
        Err(why) => census_refused(&why),
    }
}

/// `503`, naming why a census-listed feed could not be listed.
fn census_refused(why: &str) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        format!(
            "{{\"error\":{},\"listed_from\":\"census\"}}",
            crate::pullrun::quote_for_json(why)
        ),
    )
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test-only assertions"
)]
mod tests {
    use super::{
        HeldIndexKeeper, Listed, Published, Row, Unaided, answer, held_index_symbols, join, json,
    };
    use brutex_core::vendor::Vendor;
    use pull::nseindex::{Basis, Unresolved};

    /// [`json`] for a master-listed feed, which every test before the
    /// census path rendered.
    fn wire(nse: &Published, rows: &[Row]) -> String {
        json(nse, Listed::Master, None, rows)
    }

    /// Four published names, written the way NSE writes them.
    fn nse() -> Published {
        Published::from_text(
            "index_name,category\n\
             NIFTY PRIVATE BANK,sectoral-indices\n\
             NIFTY 100 QUALITY 30,strategy-indices\n\
             NIFTY 100 QUALITY LOW VOLATILITY 30,strategy-indices\n\
             NIFTY 8-13 YR GSEC,fixed-income-indices\n",
        )
    }

    #[test]
    fn the_header_is_dropped_and_blank_lines_with_it() {
        assert_eq!(nse().len(), 4);
        assert!(Published::from_text("index_name,category\n\n  \n").is_empty());
        assert!(Published::default().is_empty());
    }

    #[test]
    fn the_category_is_dropped_on_the_last_comma_not_the_first() {
        // A published name holding a comma survives; a row holding no comma
        // at all is taken whole rather than skipped.
        let held = Published::from_text("NIFTY AUTO, TRUCKS,sectoral-indices\nNIFTY IT\n");
        assert_eq!(held.len(), 2);
        assert_eq!(
            join(&held, Vendor::Zerodha, ["NIFTYAUTOTRUCKS"])
                .first()
                .and_then(|row| row.nse.clone()),
            Some("NIFTY AUTO, TRUCKS".to_owned())
        );
    }

    #[test]
    fn a_resolution_reports_the_name_as_nse_writes_it_not_collapsed() {
        let rows = join(&nse(), Vendor::Zerodha, ["NIFTY PVT BANK"]);
        assert_eq!(
            rows,
            [Row {
                symbol: "NIFTY PVT BANK".to_owned(),
                nse: Some("NIFTY PRIVATE BANK".to_owned()),
                basis: Some(Basis::Abbreviation),
                why: None,
                alias: None,
                unaided: None,
                candidates: Vec::new(),
            }]
        );
    }

    #[test]
    fn rows_are_sorted_and_a_symbol_listed_twice_appears_once() {
        let rows = join(
            &nse(),
            Vendor::Zerodha,
            ["NIFTY PVT BANK", "INDIA VIX", "NIFTY PVT BANK"],
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows.first().map(|row| row.symbol.as_str()),
            Some("INDIA VIX")
        );
    }

    #[test]
    fn every_outcome_reaches_the_wire_including_the_refusals() {
        let nse = nse();
        let rows = join(
            &nse,
            Vendor::Zerodha,
            [
                "NIFTY PRIVATE BANK",
                "NIFTY PVT BANK",
                "NIFTY100QLTY30",
                "INDIA VIX",
            ],
        );
        let wire = wire(&nse, &rows);
        assert!(wire.starts_with(
            "{\"published\":4,\"listed\":4,\"resolved\":2,\
             \"verbatim\":1,\"aliased\":0,\"abbreviated\":1,\"refused\":2,\
             \"listed_from\":\"master\",\"rows\":["
        ));
        // Proof, inference, ambiguity with its count, and absence.
        assert!(wire.contains("{\"symbol\":\"NIFTY PRIVATE BANK\",\"nse\":\"NIFTY PRIVATE BANK\",\"basis\":\"published\"}"));
        assert!(wire.contains("{\"symbol\":\"NIFTY PVT BANK\",\"nse\":\"NIFTY PRIVATE BANK\",\"basis\":\"abbreviation\"}"));
        assert!(wire.contains(
            "{\"symbol\":\"NIFTY100QLTY30\",\"nse\":null,\"why\":\"ambiguous\",\"candidates\":2,\
             \"candidate_names\":[\"NIFTY 100 QUALITY 30\",\"NIFTY 100 QUALITY LOW VOLATILITY 30\"]}"
        ));
        assert!(wire.contains("{\"symbol\":\"INDIA VIX\",\"nse\":null,\"why\":\"absent\"}"));
        assert!(wire.ends_with("]}"));
    }

    #[test]
    fn an_empty_join_still_answers_with_its_shape() {
        let nse = nse();
        assert_eq!(
            wire(&nse, &[]),
            "{\"published\":4,\"listed\":0,\"resolved\":0,\
             \"verbatim\":0,\"aliased\":0,\"abbreviated\":0,\"refused\":0,\
             \"listed_from\":\"master\",\"rows\":[]}"
        );
    }

    #[test]
    fn a_catalogue_that_cannot_be_read_names_the_path_and_the_reason() {
        let missing = crate::scratch::path("indexmap-absent").join("nse_indices.csv");
        let why = Published::read(&missing).expect_err("no such file");
        assert!(why.contains("nse_indices.csv"), "{why}");

        // An empty file is refused too: joining against nothing would report
        // every symbol as disowned by the exchange.
        let empty = crate::scratch::path("indexmap-empty");
        std::fs::create_dir_all(&empty).expect("scratch dir");
        let path = empty.join("nse_indices.csv");
        std::fs::write(&path, "index_name,category\n").expect("write");
        assert!(
            Published::read(&path)
                .expect_err("empty")
                .contains("no index names")
        );
    }

    /// **A CATALOGUE PAST ITS BOUND IS REFUSED BY SIZE (UC-19, D-1502).**
    ///
    /// Exactly `MAX_CATALOGUE_BYTES` reads; one byte more is refused naming
    /// the bound, never parsed.
    #[test]
    fn a_catalogue_past_its_bound_is_refused_by_size() {
        let dir = crate::scratch::path("indexmap-bound");
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let path = dir.join("nse_indices.csv");
        let row = "NIFTY PRIVATE BANK,sectoral\n";
        let at = usize::try_from(super::MAX_CATALOGUE_BYTES).expect("fits");
        let mut text = row.repeat(at / row.len());
        text.push_str(&"\n".repeat(at - text.len()));
        assert_eq!(text.len(), at);
        std::fs::write(&path, &text).expect("write");
        assert_eq!(Published::read(&path).expect("at the bound reads").len(), 1);
        text.push('\n');
        std::fs::write(&path, &text).expect("write");
        let why = Published::read(&path).expect_err("one byte past is refused");
        assert!(why.contains("larger than 1048576 bytes"), "{why}");
    }

    #[test]
    fn a_catalogue_that_reads_resolves_through_it() {
        let dir = crate::scratch::path("indexmap-real");
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let path = dir.join("nse_indices.csv");
        std::fs::write(&path, "index_name,category\nNIFTY PRIVATE BANK,sectoral\n").expect("write");
        let held = Published::read(&path).expect("reads");
        assert_eq!(held.len(), 1);
        assert!(!held.is_empty());
        assert_eq!(
            join(&held, Vendor::Zerodha, ["NIFTY PVT BANK"])
                .first()
                .and_then(|row| row.basis),
            Some(Basis::Abbreviation)
        );
    }

    #[test]
    fn a_refusal_carries_its_own_variant_and_not_the_others() {
        let rows = join(&nse(), Vendor::Zerodha, ["INDIA VIX", "NIFTY100QLTY30"]);
        assert_eq!(
            rows.iter().map(|row| row.why).collect::<Vec<_>>(),
            [Some(Unresolved::Absent), Some(Unresolved::Ambiguous(2))]
        );
    }

    /// **THE TWO INSTRUMENTS THE ENGINE SWEEPS WERE THE TWO IT COULD NOT
    /// RESOLVE, and this is the test that says so out loud.**
    ///
    /// The census hands over the STORE's key, and this repository renamed both
    /// of them: `NIFTY 50` became `NIFTY` and `NIFTY BANK` became `BANKNIFTY`.
    /// Measured against NSE's published list before the inverse existed, the
    /// first matched 80 names and the second matched none — so `/indexmap.json`
    /// refused the entire engine surface `CLAUDE.md` §1 defines.
    #[test]
    fn the_stores_own_rename_is_undone_before_the_exchange_is_asked() {
        let nse =
            Published::from_text("index_name,category\nNIFTY 50,broad\nNIFTY BANK,sectoral\n");

        // A VENDOR THAT RENAMES NOTHING HAS NOTHING TO UNDO. `BANKNIFTY`
        // reverses the exchange's word order, and no ordered match can see
        // through a reorder, so Dhan is left with the refusal Zerodha escapes.
        assert_eq!(
            join(&nse, Vendor::Dhan, ["BANKNIFTY"])
                .first()
                .and_then(|row| row.why),
            Some(Unresolved::Absent)
        );

        // The rename is the alias table's row, carried with the answer, and
        // the symbol alone is shown failing beside it: `BANKNIFTY` is absent,
        // and `NIFTY` abbreviates the WRONG index, which is exactly why the
        // rename is tried first.
        let banknifty = brutex_core::alias::index_by_store_key("BANKNIFTY");
        let nifty = brutex_core::alias::index_by_store_key("NIFTY");
        assert!(banknifty.is_some() && nifty.is_some());
        assert_eq!(
            join(&nse, Vendor::Zerodha, ["NIFTY", "BANKNIFTY"]),
            [
                Row {
                    symbol: "BANKNIFTY".to_owned(),
                    nse: Some("NIFTY BANK".to_owned()),
                    basis: Some(Basis::Aliased),
                    why: None,
                    alias: banknifty,
                    unaided: Some(Unaided::Refused(Unresolved::Absent)),
                    candidates: Vec::new(),
                },
                Row {
                    symbol: "NIFTY".to_owned(),
                    nse: Some("NIFTY 50".to_owned()),
                    basis: Some(Basis::Aliased),
                    why: None,
                    alias: nifty,
                    unaided: Some(Unaided::Differs(
                        "NIFTY BANK".to_owned(),
                        Basis::Abbreviation
                    )),
                    candidates: Vec::new(),
                },
            ]
        );
        // GDFL files the same two keys (D-3333), so the table joins them too.
        let gdfl = join(&nse, Vendor::Gdfl, ["NIFTY", "BANKNIFTY"]);
        assert_eq!(
            gdfl.iter()
                .map(|row| (row.nse.as_deref(), row.basis))
                .collect::<Vec<_>>(),
            [
                (Some("NIFTY BANK"), Some(Basis::Aliased)),
                (Some("NIFTY 50"), Some(Basis::Aliased))
            ]
        );
    }

    /// **AN ALIAS MAY NOT LAUNDER AN ABBREVIATION INTO PROOF.**
    ///
    /// Following a rename costs nothing, because the rename is a stated fact.
    /// Letting the renamed-from name then resolve by ABBREVIATION would stack
    /// inference on inference and report the result beside the verbatim
    /// matches, so that path keeps the original refusal instead.
    #[test]
    fn the_alias_path_accepts_only_a_verbatim_match() {
        // `NIFTY 50 Arbitrage` is not `NIFTY 50`; `NIFTY50` merely abbreviates
        // it. The alias leads there, and the answer is still no.
        let nse = Published::from_text("index_name,category\nNIFTY 50 Arbitrage,strategy\n");
        let rows = join(&nse, Vendor::Zerodha, ["NIFTY"]);
        assert_eq!(rows.first().and_then(|row| row.nse.clone()), None);
        assert_eq!(
            rows.first().and_then(|row| row.why),
            Some(Unresolved::Absent)
        );
    }

    /// The wire word for the third basis, and its count beside the other two.
    #[test]
    fn an_aliased_row_is_counted_and_named_apart_from_the_verbatim_ones() {
        let nse =
            Published::from_text("index_name,category\nNIFTY 50,broad\nNIFTY BANK,sectoral\n");
        let wire = wire(&nse, &join(&nse, Vendor::Zerodha, ["NIFTY", "NIFTY 50"]));
        assert!(
            wire.starts_with(
                "{\"published\":2,\"listed\":2,\"resolved\":2,\
                 \"verbatim\":1,\"aliased\":1,\"abbreviated\":0,\"refused\":0,\
                 \"listed_from\":\"master\",\"rows\":["
            ),
            "{wire}"
        );
        assert!(
            wire.contains(
                "{\"symbol\":\"NIFTY\",\"nse\":\"NIFTY 50\",\"basis\":\"aliased\",\
                 \"alias\":{\"fno\":\"NIFTY\",\"nse_name\":\"NIFTY 50\",\"index_symbol\":\"NIFTY 50\",\
                 \"store_key\":\"NIFTY\",\"status\":\"sourced\",\"source\":\"docs/00-charter.md, \
                 Index aliases and symbol changes — read 7 Oct 2026 (D-3332)\",\"decision\":\"D-3332\"},\
                 \"unaided\":{\"nse\":\"NIFTY BANK\",\"basis\":\"abbreviation\"}}"
            ),
            "{wire}"
        );
        assert!(
            wire.contains("{\"symbol\":\"NIFTY 50\",\"nse\":\"NIFTY 50\",\"basis\":\"published\"}"),
            "{wire}"
        );
    }

    /// NSE's names around FINNIFTY's index, as the operator's catalogue
    /// writes them: the one the product page names, and the three an
    /// abbreviation could also reach.
    fn financial_services() -> Published {
        Published::from_text(
            "index_name,category\n\
             NIFTY FINANCIAL SERVICES,Indices Eligible In Derivatives\n\
             NIFTY FINANCIAL SERVICES EX-BANK,Sectoral Market Indices\n\
             NIFTY FINANCIAL SERVICES 25/50,Sectoral Market Indices\n\
             NIFTY MIDSMALL FINANCIAL SERVICES,Sectoral Market Indices\n\
             NIFTY MIDCAP SELECT,Indices Eligible In Derivatives\n\
             NIFTY NEXT 50,Indices Eligible In Derivatives\n\
             NIFTY INDIA FPI 150,Indices Eligible In Derivatives\n",
        )
    }

    /// **DB-G25 AND DB-G27: THE TABLE DECIDES, AND THE CANDIDATES ARE NAMED.**
    /// `NIFTYFINSERVICE` was refused as ambiguous with only a count. The
    /// sourced row resolves it to `NIFTY FINANCIAL SERVICES`, and the three
    /// names the abbreviation alone could not choose between travel with it,
    /// in the catalogue's order. A feed whose keys the table does not record
    /// (Dhan) is not given the table's answer: it keeps the ambiguity, now
    /// with its names.
    #[test]
    fn a_sourced_alias_resolves_an_ambiguous_key_and_its_candidates_are_named() {
        let nse = financial_services();
        let three = [
            "NIFTY FINANCIAL SERVICES",
            "NIFTY FINANCIAL SERVICES EX-BANK",
            "NIFTY MIDSMALL FINANCIAL SERVICES",
        ];
        let rows = join(
            &nse,
            Vendor::Zerodha,
            [
                "NIFTYFINSERVICE",
                "NIFTYMIDSELECT",
                "NIFTYNEXT50",
                "NIFTYFPI150",
            ],
        );
        let fin = rows.first().expect("NIFTYFINSERVICE");
        assert_eq!(fin.symbol, "NIFTYFINSERVICE");
        assert_eq!(fin.nse.as_deref(), Some("NIFTY FINANCIAL SERVICES"));
        assert_eq!(fin.basis, Some(Basis::Aliased));
        assert_eq!(fin.alias.and_then(|row| row.fno), Some("FINNIFTY"));
        assert_eq!(
            fin.unaided,
            Some(Unaided::Refused(Unresolved::Ambiguous(3)))
        );
        assert_eq!(fin.candidates, three);

        // NIFTY FPI 150 abbreviates NIFTY INDIA FPI 150 and nothing else:
        // the two answers agree, so nothing is carried beside it.
        let fpi = rows.get(1).expect("NIFTYFPI150");
        assert_eq!(fpi.nse.as_deref(), Some("NIFTY INDIA FPI 150"));
        assert_eq!(fpi.alias.and_then(|row| row.fno), Some("NIFTYFPI"));
        assert_eq!(fpi.unaided, None);
        assert_eq!(
            fpi.basis,
            Some(Basis::Aliased),
            "an abbreviation alone, so the table decided"
        );
        let mid = rows.get(2).expect("NIFTYMIDSELECT");
        assert_eq!(mid.nse.as_deref(), Some("NIFTY MIDCAP SELECT"));
        assert_eq!(mid.alias.and_then(|row| row.fno), Some("MIDCPNIFTY"));
        assert_eq!(mid.basis, Some(Basis::Aliased));
        let next = rows.get(3).expect("NIFTYNEXT50");
        assert_eq!(next.nse.as_deref(), Some("NIFTY NEXT 50"));
        assert_eq!(next.alias.and_then(|row| row.fno), Some("NIFTYNXT50"));
        assert_eq!(next.unaided, None, "verbatim alone, and the same index");
        // The exchange's own list proves it verbatim, so the proof stays in
        // the verbatim column and the table's row travels beside it.
        assert_eq!(next.basis, Some(Basis::Published));

        let wire = wire(&nse, &rows);
        assert!(
            wire.contains(
                "\"unaided\":{\"why\":\"ambiguous\",\"candidates\":3,\"candidate_names\":\
                 [\"NIFTY FINANCIAL SERVICES\",\"NIFTY FINANCIAL SERVICES EX-BANK\",\
                 \"NIFTY MIDSMALL FINANCIAL SERVICES\"]}"
            ),
            "{wire}"
        );

        let dhan = join(&nse, Vendor::Dhan, ["NIFTYFINSERVICE"]);
        let only = dhan.first().expect("one row");
        assert_eq!(only.why, Some(Unresolved::Ambiguous(3)));
        assert_eq!(only.alias, None);
        assert_eq!(only.candidates, three);
        assert!(wire_of(&nse, &dhan).contains(
            "{\"symbol\":\"NIFTYFINSERVICE\",\"nse\":null,\"why\":\"ambiguous\",\
                 \"candidates\":3,\"candidate_names\":[\"NIFTY FINANCIAL SERVICES\",\
                 \"NIFTY FINANCIAL SERVICES EX-BANK\",\"NIFTY MIDSMALL FINANCIAL SERVICES\"]}"
        ));
    }

    /// [`wire`], for a second body in one test.
    fn wire_of(nse: &Published, rows: &[Row]) -> String {
        wire(nse, rows)
    }

    /// A catalogue that lacks the table's name verbatim does not get the
    /// table's answer by abbreviation: the row falls to the symbol's own
    /// resolution, unchanged.
    #[test]
    fn the_table_path_also_accepts_only_a_verbatim_name() {
        let nse = Published::from_text(
            "index_name,category\nNIFTY FINANCIAL SERVICES EX-BANK,sectoral\n",
        );
        let rows = join(&nse, Vendor::Zerodha, ["NIFTYFINSERVICE"]);
        let row = rows.first().expect("one row");
        assert_eq!(row.alias, None);
        assert_eq!(row.basis, Some(Basis::Abbreviation));
        assert_eq!(row.nse.as_deref(), Some("NIFTY FINANCIAL SERVICES EX-BANK"));
    }

    /// The candidates are counted by the rule the resolver counts with, so
    /// their number is the ambiguity's, and a duplicate published name
    /// appears once, under the spelling that came first.
    #[test]
    fn the_candidate_names_are_the_resolvers_count_once_each_in_file_order() {
        let nse = Published::from_text(
            "index_name,category\n\
             NIFTY 100 QUALITY LOW VOLATILITY 30,strategy\n\
             NIFTY 100 QUALITY 30,strategy\n\
             Nifty 100 Quality 30,strategy\n",
        );
        assert_eq!(nse.len(), 2);
        assert_eq!(
            nse.candidates("NIFTY100QLTY30"),
            [
                "NIFTY 100 QUALITY LOW VOLATILITY 30",
                "NIFTY 100 QUALITY 30"
            ]
        );
        assert!(nse.candidates("ZZZ").is_empty());
        let rows = join(&nse, Vendor::Groww, ["NIFTY100QLTY30"]);
        assert_eq!(
            rows.first().and_then(|row| row.why),
            Some(Unresolved::Ambiguous(2))
        );
    }

    /// A census in a scratch store with the given keys recorded, and the site
    /// over it.
    fn census_site(
        name: &str,
        keys: &[(brutex_core::instrument::Segment, &str, bool)],
    ) -> (std::path::PathBuf, crate::server::Site) {
        let root = crate::scratch::path(name);
        let _ = std::fs::remove_dir_all(&root);
        let masters = root.join("masters");
        std::fs::create_dir_all(&masters).expect("empty masters");
        if !keys.is_empty() {
            write_census(&root, keys);
        }
        let site = crate::server::Site::load(&masters, &root);
        (root, site)
    }

    /// A GDFL census under `root` recording two months of each key, written
    /// whole: a second call replaces the first with a different commit.
    fn write_census(
        root: &std::path::Path,
        keys: &[(brutex_core::instrument::Segment, &str, bool)],
    ) {
        use brutex_core::instrument::{Contract, Exchange};
        use brutex_core::symbol::Symbol;
        use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
        use store::path::{Timeframe, YearMonth};
        let mut manifest = Manifest::open_image(Vendor::Gdfl, &[]).expect("genesis");
        for (n, &(segment, symbol, contract)) in keys.iter().enumerate() {
            for month in [8_u8, 9] {
                let contract =
                    contract.then(|| Contract::parse("2019-09-26-1150000-CE").expect("contract"));
                manifest
                    .record(Entry {
                        key: EntryKey {
                            contract,
                            exchange: Exchange::Nse,
                            segment,
                            symbol: Symbol::new(symbol).expect("symbol"),
                            timeframe: Timeframe::SECOND_1,
                            month: YearMonth::new(2019, month).expect("month"),
                        },
                        rows: 1,
                        first_ts_micros: 1_567_000_000_000_000 + i64::from(month),
                        last_ts_micros: 1_567_000_000_000_000 + i64::from(month),
                    })
                    .unwrap_or_else(|why| panic!("entry {n}: {why}"));
            }
        }
        let path = manifest_path(root, Vendor::Gdfl);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("manifest dir");
        std::fs::write(path, manifest.image()).expect("publish the census");
    }

    /// The modified time of the GDFL manifest under `root`, as a `stat`
    /// finds it: what `CensusStamps::modified` hands the keeper.
    fn modified_of(root: &std::path::Path) -> Option<std::time::SystemTime> {
        std::fs::metadata(pull::manifest::manifest_path(root, Vendor::Gdfl))
            .and_then(|meta| meta.modified())
            .ok()
    }

    /// **THE CENSUS WALK IS PAID ONCE PER COMMIT, NOT PER REQUEST.** Two
    /// requests over one commit share one answer; a new commit at the same
    /// path is walked again and replaces it -- including one with the same
    /// generation and the same counts, told apart by its modified time; a
    /// census that did not load is refused by its note and never kept, and
    /// leaves the kept commit alone; a census whose stamp found no time is
    /// walked every time and never kept.
    #[test]
    fn the_index_keys_are_walked_once_per_census_commit() {
        use brutex_core::instrument::Segment;
        use std::sync::Arc;
        let (root, _site) = census_site(
            "indexmap-keeper",
            &[
                (Segment::Index, "NIFTY", false),
                (Segment::Cash, "RELIANCE", false),
            ],
        );
        let keeper = HeldIndexKeeper::default();
        let read = || crate::census::read_vendor(&root, Vendor::Gdfl);
        let stamp = modified_of(&root).expect("a manifest with a modified time");
        let before = read();
        let first = keeper.symbols(&before, Some(stamp)).expect("held");
        let second = keeper.symbols(&read(), Some(stamp)).expect("held");
        assert_eq!(*first, ["NIFTY"]);
        assert!(Arc::ptr_eq(&first, &second), "one commit is walked once");

        // Replaced whole by a census with the SAME generation and the SAME
        // counters (two keys of two months, one row each) and other keys.
        // Only the modified time tells the two commits apart; it is moved a
        // whole second on so the test does not lean on the clock's grain.
        write_census(
            &root,
            &[
                (Segment::Index, "NIFTY", false),
                (Segment::Index, "NIFTYIT", false),
            ],
        );
        let later = stamp + std::time::Duration::from_secs(1);
        std::fs::File::options()
            .write(true)
            .open(pull::manifest::manifest_path(&root, Vendor::Gdfl))
            .and_then(|file| file.set_modified(later))
            .expect("move the modified time");
        let after = read();
        assert_eq!(after.generation(), before.generation(), "same generation");
        assert_eq!(after.counters(), before.counters(), "same counters");
        let next = keeper.symbols(&after, modified_of(&root)).expect("held");
        assert!(!Arc::ptr_eq(&first, &next), "a new commit is walked again");
        let mut held = (*next).clone();
        held.sort_unstable();
        assert_eq!(held, ["NIFTY", "NIFTYIT"]);

        let absent = crate::census::VendorCensus {
            vendor: Vendor::Gdfl,
            path: root.join("nowhere"),
            state: crate::census::Census::Absent,
        };
        assert!(keeper.symbols(&absent, Some(later)).is_err());
        let again = keeper.symbols(&read(), Some(later)).expect("held");
        assert!(
            Arc::ptr_eq(&next, &again),
            "a refusal does not evict a commit"
        );

        // No stamp: walked every time, and the kept commit is left alone.
        let unstamped = keeper.symbols(&read(), None).expect("held");
        let unstamped_again = keeper.symbols(&read(), None).expect("held");
        assert_eq!(*unstamped, *next);
        assert!(!Arc::ptr_eq(&unstamped, &unstamped_again), "never kept");
        assert!(!Arc::ptr_eq(&unstamped, &next));
        let kept = keeper.symbols(&read(), Some(later)).expect("held");
        assert!(
            Arc::ptr_eq(&next, &kept),
            "an unstamped walk evicts nothing"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    /// **DB-G26: A FEED WITH NO MASTER LISTS ITS CENSUS'S INDICES.** GDFL
    /// answered `listed 0, rows []` while its census held NIFTY and BANKNIFTY.
    /// The census's `INDEX` keys are listed, each once however many months it
    /// holds; a share and an option contract on the same symbol are not.
    #[test]
    fn a_feed_with_no_master_lists_the_indices_its_census_holds() {
        use brutex_core::instrument::Segment;
        let (root, site) = census_site(
            "indexmap-census-gdfl",
            &[
                (Segment::Index, "NIFTY", false),
                (Segment::Index, "BANKNIFTY", false),
                (Segment::Cash, "RELIANCE", false),
                (Segment::Fno, "NIFTYIT", true),
            ],
        );
        let nse =
            Published::from_text("index_name,category\nNIFTY 50,broad\nNIFTY BANK,sectoral\n");
        let (status, body) = answer(&site, &nse, Vendor::Gdfl, Vec::new());
        assert_eq!(status, axum::http::StatusCode::OK, "{body}");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        assert_eq!(parsed["listed_from"], "census", "{body}");
        assert_eq!(parsed["listed"], 2, "{body}");
        assert_eq!(parsed["aliased"], 2, "{body}");
        assert_eq!(parsed["rows"][0]["symbol"], "BANKNIFTY", "{body}");
        assert_eq!(parsed["rows"][0]["nse"], "NIFTY BANK", "{body}");
        assert_eq!(parsed["rows"][1]["symbol"], "NIFTY", "{body}");
        assert_eq!(parsed["rows"][1]["alias"]["fno"], "NIFTY", "{body}");
        assert!(parsed.get("census_note").is_none(), "{body}");

        // A feed WITH a master is joined over what the master listed, and
        // its census is not asked.
        let (status, body) = answer(&site, &nse, Vendor::Zerodha, vec!["NIFTY"]);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(body.contains("\"listed_from\":\"master\""), "{body}");
        assert!(body.contains("\"listed\":1,"), "{body}");
        let _ = std::fs::remove_dir_all(root);
    }

    /// A census that is not there lists nothing, and says so: `503` with the
    /// census's own words, never `listed 0` as though the feed held no index.
    #[test]
    fn a_feed_with_no_master_and_no_census_is_refused_by_name() {
        let (root, site) = census_site("indexmap-census-absent", &[]);
        let nse = Published::from_text("index_name,category\nNIFTY 50,broad\n");
        let (status, body) = answer(&site, &nse, Vendor::Gdfl, Vec::new());
        assert_eq!(
            status,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "{body}"
        );
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        assert_eq!(parsed["listed_from"], "census");
        let error = parsed["error"].as_str().expect("a named reason");
        assert!(
            error.contains("gdfl") && error.contains("no manifest"),
            "{error}"
        );
        let absent = crate::census::VendorCensus {
            vendor: Vendor::Gdfl,
            path: root.join("nowhere"),
            state: crate::census::Census::Absent,
        };
        assert!(held_index_symbols(&absent).is_err());
        let unreadable = crate::census::VendorCensus {
            state: crate::census::Census::Unreadable {
                reason: "a damaged header".to_owned(),
                fault: crate::census::Fault::Refused,
            },
            ..absent
        };
        let why = held_index_symbols(&unreadable).expect_err("unreadable");
        assert!(why.contains("a damaged header"), "{why}");
        let _ = std::fs::remove_dir_all(root);
    }

    /// A degraded census is listed, and its note travels on the wire.
    #[test]
    fn a_census_note_is_carried_when_one_is_given() {
        let nse = Published::from_text("index_name,category\nNIFTY 50,broad\n");
        let body = json(&nse, Listed::Census, Some("DEGRADED \"x\""), &[]);
        assert!(
            body.contains(
                "\"listed_from\":\"census\",\"census_note\":\"DEGRADED \\\"x\\\"\",\"rows\":[]"
            ),
            "{body}"
        );
        assert_eq!(Listed::Master.word(), "master");
    }
}

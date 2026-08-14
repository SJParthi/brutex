//! The exchange's own index directory, and the constituent files it links to.
//!
//! # Why this module exists
//!
//! `crates/core/src/universe.rs` holds **2,526 symbols and 2,526 ISINs typed
//! into a Rust file by hand**, transcribed once from CSVs downloaded on
//! 2026-08-11. It is a photograph of an index that rebalances, and
//! `docs/00-charter.md` §4a already records it drifting: the exchange lists
//! `GRINDWELL`, the array holds `AGL`, and this repository cannot say what
//! `AGL` is. One name in 750, found because somebody happened to look.
//!
//! Nothing looks on its own. This module is the first half of what does.
//!
//! # What it does NOT do
//!
//! It opens no socket. Every function here takes bytes that somebody else
//! fetched and turns them into rows or into a named refusal — the same seam
//! [`crate::fetch::BarSource`] draws for vendor bars, for the same reason:
//! `CLAUDE.md` requires this crate to build and test against fakes with no live
//! call, and a decoder that can only be exercised against the network is a
//! decoder nobody exercises.
//!
//! # The route is CRAWLED, never composed
//!
//! `docs/00-charter.md` §4d, read 14 Aug 2026: the exchange publishes 148
//! equity indices across four category pages, and each index's own page links
//! its constituent CSV. **The CSV filename cannot be derived from the index
//! name** — `ind_niftybanklist.csv` is word-joined and
//! `ind_niftytotalmarket_list.csv` is underscore-separated, and no rule turns
//! "Nifty Bank" into the first and "Nifty Total Market" into the second.
//!
//! So there is no function here that builds a filename. [`constituent_link`]
//! reads the one the exchange published, and an index page carrying none is
//! refused by name. Composing one would be `CLAUDE.md` §3 rule 1's invention,
//! and the two examples above are already proof that any rule guessed from one
//! of them is wrong about the other.
//!
//! # Cost
//!
//! Every function is a single pass over its input with no allocation per
//! candidate beyond the rows it returns. There is no backtracking and no
//! regular expression: the two shapes being looked for are anchored literals,
//! and a scan for an anchored literal is linear in the bytes with a constant
//! that does not depend on how many matches there are.

use core::fmt;

/// The most bytes one fetched document may occupy.
///
/// `docs/07-o1-architecture.md` law 5 — bound every input at the boundary, and
/// unbounded input always arrives from outside. The largest constituent file
/// this repository has read is the Total Market's 752 rows at roughly 70 bytes
/// each; a category page is smaller still. This is generous for either and
/// still refuses a host that answers a CSV request with a video.
pub const MAX_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;

/// The most rows one constituent file may carry.
///
/// The widest index published is the Total Market at 752 rows including its
/// placeholders. This leaves room for an index several times wider without
/// being a number nobody can justify.
pub const MAX_CONSTITUENTS: usize = 10_000;

/// The header every constituent file carries, exactly.
///
/// Confirmed against a live body on 14 Aug 2026 — `ind_niftybanklist.csv`, 916
/// bytes — and it is the same five columns `docs/00-charter.md` §4c already
/// records for the five files transcribed in August. Compared in full rather
/// than by column count: a file with five columns in a different order parses
/// perfectly and yields an ISIN in the symbol's place.
pub const CONSTITUENT_HEADER: &str = "Company Name,Industry,Symbol,Series,ISIN Code";

/// Which family of indices a category page lists.
///
/// Four, because the exchange publishes four. This is not a set this repository
/// chose and it is not one it may extend: a fifth category is a fifth page on
/// the exchange's site, and inventing one here would produce a request for a
/// page that does not exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// The large, liquid benchmarks. 22 on 14 Aug 2026.
    BroadBased,
    /// One per sector. 34 on 14 Aug 2026.
    Sectoral,
    /// Weighting and factor variants. 48 on 14 Aug 2026.
    Strategy,
    /// Cross-sector themes. 44 on 14 Aug 2026.
    Thematic,
}

impl Category {
    /// Every category, in the order the exchange's own navigation lists them.
    pub const ALL: [Self; 4] = [
        Self::BroadBased,
        Self::Sectoral,
        Self::Strategy,
        Self::Thematic,
    ];

    /// The path this category's listing page sits at, below the index host.
    ///
    /// A literal read from the exchange's own navigation, not a slug built from
    /// [`Self::label`] — the same rule the constituent filename follows, and
    /// for the same reason.
    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::BroadBased => "/indices/equity/broad-based-indices",
            Self::Sectoral => "/indices/equity/sectoral-indices",
            Self::Strategy => "/indices/equity/strategy-indices",
            Self::Thematic => "/indices/equity/thematic-indices",
        }
    }

    /// What the exchange calls this family, for a human.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::BroadBased => "Broad-based",
            Self::Sectoral => "Sectoral",
            Self::Strategy => "Strategy",
            Self::Thematic => "Thematic",
        }
    }

    /// How many indices this category listed when §4d was read.
    ///
    /// **A measurement, not a bound.** It is not used to refuse a page that
    /// lists a different number — an index family gains and loses members and
    /// that is the entire reason this module exists. It is carried so a caller
    /// can SAY the count moved, which is a fact worth reporting and never a
    /// fact worth failing on.
    #[must_use]
    pub const fn counted_on_14_aug_2026(self) -> usize {
        match self {
            Self::BroadBased => 22,
            Self::Sectoral => 34,
            Self::Strategy => 48,
            Self::Thematic => 44,
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// One index, as its category page names it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexRef {
    /// Which family listed it.
    pub category: Category,
    /// The path to its own page, exactly as the listing wrote it.
    pub path: String,
}

/// One row of a constituent file.
///
/// Borrowed from the document rather than owned: a caller keeps the symbol and
/// the ISIN and discards the rest, and allocating four strings per row to throw
/// three away is work done for nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Constituent<'a> {
    /// The company's registered name.
    pub company: &'a str,
    /// The exchange's industry classification.
    pub industry: &'a str,
    /// The trading symbol.
    pub symbol: &'a str,
    /// The board series, e.g. `EQ`.
    pub series: &'a str,
    /// The ISIN. **This is the identity** — a symbol is a label the exchange
    /// may reuse, and an ISIN is what makes a row checkable against a vendor
    /// master rather than merely readable. See D-0125.
    pub isin: &'a str,
}

/// Why a fetched document produced no rows.
///
/// Every variant carries what it refused. `CLAUDE.md` §4 — degrade loudly and
/// name the reason, or refuse; never both silently.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NseError {
    /// The document is larger than [`MAX_DOCUMENT_BYTES`].
    TooLarge {
        /// How many bytes arrived.
        bytes: usize,
        /// The bound.
        cap: usize,
    },
    /// The body is not the CSV this route serves.
    ///
    /// **The arm that matters most.** An exchange host answering a CSV request
    /// with an interstitial or a bot check returns a 200 and a body, and a CSV
    /// reader handed HTML finds no valid rows and reports **zero constituents**
    /// — which reads exactly like an index that lost every member. An empty
    /// universe must never be a valid answer, so the shape is checked before
    /// the content.
    NotCsv {
        /// The first bytes of what arrived, so an operator can see whether it
        /// is a login page, an error document or something else entirely.
        head: String,
    },
    /// The header is not [`CONSTITUENT_HEADER`].
    ///
    /// Compared in full, never by column count: five columns in a different
    /// order parse cleanly and put an ISIN where the symbol belongs.
    HeaderUnexpected {
        /// The header line that arrived.
        got: String,
    },
    /// A row does not carry five fields.
    ///
    /// Almost always an unquoted comma inside a company name, which shifts
    /// every field after it — including the ISIN. Refused by row rather than
    /// read short, because a short read silently puts the series where the ISIN
    /// should be.
    FieldCount {
        /// Which row, counting the header as row 0.
        row: usize,
        /// How many fields it held.
        got: usize,
    },
    /// A row carries an empty symbol or an empty ISIN.
    ///
    /// Both are the row's identity. A constituent that cannot be named cannot
    /// be joined to a vendor master, and carrying it forward as a blank would
    /// make it match every other blank.
    RowNotIdentified {
        /// Which row.
        row: usize,
        /// Which field was empty.
        field: &'static str,
    },
    /// One symbol appears twice in one index.
    ///
    /// An index cannot hold a name twice, so a file that says otherwise is not
    /// the file it claims to be. Counted as a refusal rather than deduplicated:
    /// silently collapsing them would make the index's size wrong by one with
    /// nothing to notice it.
    DuplicateSymbol {
        /// The symbol that repeated.
        symbol: String,
        /// Where it appeared the second time.
        row: usize,
    },
    /// More rows than [`MAX_CONSTITUENTS`].
    TooManyRows {
        /// How many arrived.
        rows: usize,
        /// The bound.
        cap: usize,
    },
    /// The file holds a header and nothing else.
    ///
    /// Distinct from [`Self::NotCsv`]: this IS the right document and it is
    /// empty, which is a fact about the exchange rather than about the
    /// transport, and the two send an operator to different places.
    NoRows,
    /// An index page carries no constituent link.
    ///
    /// **Never resolved by composing a filename.** `ind_niftybanklist.csv` and
    /// `ind_niftytotalmarket_list.csv` use different conventions, so any rule
    /// guessed from one is already wrong about the other — see this module's
    /// header and `docs/00-charter.md` §4d.
    NoConstituentLink {
        /// The page that carried none.
        page: String,
    },
}

impl fmt::Display for NseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TooLarge { bytes, cap } => write!(
                f,
                "the exchange answered with {bytes} bytes; this build accepts at most {cap}"
            ),
            Self::NotCsv { ref head } => write!(
                f,
                "this route serves a CSV and the body is not one. Nothing was \
                 read: a CSV reader handed a bot-check or an error page finds \
                 no rows and reports an index with no members, which is \
                 indistinguishable from one that lost them all. It begins: \
                 {head:?}"
            ),
            Self::HeaderUnexpected { ref got } => write!(
                f,
                "the constituent header is {got:?} and this build reads \
                 {CONSTITUENT_HEADER:?}. Compared in full rather than by \
                 column count, because five columns in another order parse \
                 cleanly and put an ISIN where the symbol belongs."
            ),
            Self::FieldCount { row, got } => write!(
                f,
                "row {row} carries {got} field(s) and a constituent row carries \
                 five. Usually an unquoted comma in a company name, which \
                 shifts every field after it — including the ISIN. Refused \
                 rather than read short."
            ),
            Self::RowNotIdentified { row, field } => write!(
                f,
                "row {row} has an empty {field}, which is the row's identity. \
                 A constituent that cannot be named cannot be joined to any \
                 vendor's master."
            ),
            Self::DuplicateSymbol { ref symbol, row } => write!(
                f,
                "{symbol} appears twice, the second time at row {row}. An index \
                 does not hold a name twice, so this file is not what it says \
                 it is. Refused rather than deduplicated: collapsing them makes \
                 the index's size wrong by one with nothing left to notice."
            ),
            Self::TooManyRows { rows, cap } => write!(
                f,
                "the file holds {rows} rows; this build accepts at most {cap}"
            ),
            Self::NoRows => write!(
                f,
                "the file is a header and nothing else. That is this document \
                 being empty, not the transport failing — a different fact, \
                 and it sends you somewhere different."
            ),
            Self::NoConstituentLink { ref page } => write!(
                f,
                "{page} carries no constituent-file link. Nothing is composed \
                 from the index's name: the exchange writes ind_niftybanklist \
                 for one index and ind_niftytotalmarket_list for another, so \
                 any filename rule guessed from one is already wrong about the \
                 other."
            ),
        }
    }
}

impl core::error::Error for NseError {}

/// Every `href` in `html` that begins with `prefix`, in document order.
///
/// # A scanner, not a parser, and the difference is deliberate
///
/// This does not understand HTML and does not try to. It looks for an anchored
/// literal — `href="` followed by `prefix` — and takes bytes to the closing
/// quote. That is enough for the two shapes this module needs and it adds no
/// dependency: `CLAUDE.md` §2 and CI gate 3 make a new crate a decision, and an
/// HTML parser to read two attribute patterns is not one worth taking.
///
/// What it cannot do is see a link assembled by script, and that is stated
/// rather than hidden: a page that stops emitting its links as markup would
/// yield zero here, and the caller refuses on zero rather than treating it as
/// "this category has no indices".
///
/// **Single-quoted attributes are read too.** Both spellings are legal HTML and
/// a document may use either; reading only one would make the scan depend on a
/// formatting choice the exchange never promised to keep.
///
/// One pass over the bytes. No allocation per candidate beyond the matches
/// returned.
#[must_use]
pub fn hrefs_with_prefix<'a>(html: &'a str, prefix: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find("href=") {
        // `get` rather than slicing: an index derived from `find` is in range,
        // and an index that is correct *because of a call above it* is how a
        // panic arrives during a later edit. This workspace denies panics.
        let Some(after) = rest.get(at.saturating_add(5)..) else {
            break;
        };
        let (quote, value) = match after.as_bytes().first() {
            Some(b'"') => ('"', after.get(1..)),
            Some(b'\'') => ('\'', after.get(1..)),
            // An unquoted attribute value. Legal HTML, and not a shape this
            // scan reads — skipped rather than guessed at, because the
            // terminator would then be whitespace OR `>` and picking one wrong
            // silently truncates a URL.
            _ => {
                rest = after;
                continue;
            }
        };
        let Some(value) = value else { break };
        let Some(end) = value.find(quote) else { break };
        if let Some(href) = value.get(..end).filter(|h| h.starts_with(prefix)) {
            out.push(href);
        }
        rest = value.get(end..).unwrap_or("");
    }
    out
}

/// Every index page a category listing links to, deduplicated, in order.
///
/// Deduplicated because a listing legitimately links the same index twice — a
/// navigation entry and a body entry are the same index — and that is a fact
/// about the page's markup rather than about the index family. Contrast
/// [`NseError::DuplicateSymbol`], which refuses: two links to one page are the
/// same statement made twice, while one symbol twice in an index is a
/// contradiction.
#[must_use]
pub fn index_links(html: &str, category: Category) -> Vec<IndexRef> {
    // The prefix is the category's own path plus a separator, so a listing that
    // links a SIBLING category is not collected into this one.
    let prefix = format!("{}/", category.path());
    let mut seen: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for href in hrefs_with_prefix(html, &prefix) {
        // A trailing slash and no slug is the category page linking itself.
        if href.len() <= prefix.len() {
            continue;
        }
        if seen.contains(&href) {
            continue;
        }
        seen.push(href);
        out.push(IndexRef {
            category,
            path: href.to_owned(),
        });
    }
    out
}

/// The path segment every constituent link sits under.
///
/// Capitalised exactly as the exchange writes it. Recorded as a constant so the
/// one place that knows the shape is this one.
pub const CONSTITUENT_DIR: &str = "/IndexConstituent/";

/// The constituent file an index page links to.
///
/// # Errors
///
/// [`NseError::NoConstituentLink`] when the page carries none — and that is the
/// end of it. No filename is composed from the index's name, for the reason
/// this module's header gives and the error itself repeats.
///
/// The exchange writes this link with a doubled slash after the host
/// (`niftyindices.com//IndexConstituent/…`), which a URL parser collapses. The
/// match is on the directory segment rather than on the whole URL so that
/// either spelling is found.
pub fn constituent_link(html: &str, page: &str) -> Result<String, NseError> {
    let mut rest = html;
    while let Some(at) = rest.find("href=") {
        let Some(after) = rest.get(at.saturating_add(5)..) else {
            break;
        };
        let (quote, value) = match after.as_bytes().first() {
            Some(b'"') => ('"', after.get(1..)),
            Some(b'\'') => ('\'', after.get(1..)),
            _ => {
                rest = after;
                continue;
            }
        };
        let Some(value) = value else { break };
        let Some(end) = value.find(quote) else { break };
        // `eq_ignore_ascii_case` on the extension, not `ends_with`: the
        // exchange writes `.csv` today and a host that ever wrote `.CSV` would
        // be serving the same file. Case is a spelling of the extension, not a
        // different route — unlike the DIRECTORY, which is matched exactly
        // because `/IndexConstituent/` is a path segment and a path segment is
        // case-sensitive on the wire.
        if let Some(href) = value.get(..end).filter(|h| {
            h.contains(CONSTITUENT_DIR)
                && h.rsplit_once('.')
                    .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("csv"))
        }) {
            return Ok(href.to_owned());
        }
        rest = value.get(end..).unwrap_or("");
    }
    Err(NseError::NoConstituentLink {
        page: page.to_owned(),
    })
}

/// Whether a body is plausibly the CSV this route serves.
///
/// **Shape before content.** A host that answers with an interstitial returns a
/// 200 and a body, and a CSV reader handed HTML reports an index with no
/// members — which is indistinguishable from an index that lost them all. The
/// test is that the document's first non-blank line is the header this build
/// reads, which no HTML document satisfies by accident.
///
/// A leading byte-order mark is stripped first: it is a byte a text editor may
/// add, it carries no meaning here, and letting it fail the header comparison
/// would refuse a file that is entirely correct.
#[must_use]
pub fn looks_like_constituents(body: &str) -> bool {
    first_line(body).trim_end_matches('\r') == CONSTITUENT_HEADER
}

/// The first non-empty line, with any byte-order mark removed.
fn first_line(body: &str) -> &str {
    body.trim_start_matches('\u{feff}')
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
}

/// One constituent file, decoded into rows, or the refusal that stops it.
///
/// # Errors
///
/// Every variant of [`NseError`] except [`NseError::NoConstituentLink`], which
/// belongs to the step before this one. The order of the checks is the order
/// they must happen in: size, then shape, then header, then rows — each one
/// makes the next meaningful, and a header check on a bot-check page is a
/// question about the wrong document.
///
/// # Cost
///
/// One pass. The output vector is reserved from a line count taken before the
/// loop — `docs/07-o1-architecture.md` law 2, so it never grows — and the
/// duplicate check walks a list bounded by [`MAX_CONSTITUENTS`], which is the
/// same bound the file itself is held to.
pub fn constituents(body: &str) -> Result<Vec<Constituent<'_>>, NseError> {
    if body.len() > MAX_DOCUMENT_BYTES {
        return Err(NseError::TooLarge {
            bytes: body.len(),
            cap: MAX_DOCUMENT_BYTES,
        });
    }
    if !looks_like_constituents(body) {
        let head: String = body.chars().take(120).collect();
        return Err(NseError::NotCsv { head });
    }

    let clean = body.trim_start_matches('\u{feff}');
    let mut lines = clean.lines().filter(|line| !line.trim().is_empty());
    // `looks_like_constituents` already proved this line exists and matches, so
    // the header is consumed rather than re-compared. The `HeaderUnexpected`
    // arm below is reachable through `constituents_strict`, which is the entry
    // point that does not pre-check the shape.
    let _header = lines.next();

    let rest: Vec<&str> = lines.collect();
    if rest.is_empty() {
        return Err(NseError::NoRows);
    }
    if rest.len() > MAX_CONSTITUENTS {
        return Err(NseError::TooManyRows {
            rows: rest.len(),
            cap: MAX_CONSTITUENTS,
        });
    }

    let mut out: Vec<Constituent<'_>> = Vec::with_capacity(rest.len());
    for (i, line) in rest.iter().enumerate() {
        // Rows are numbered with the header as 0, so a message names the line
        // an operator will count to in the file itself.
        let row = i.saturating_add(1);
        let fields = split_row(line);
        let [company, industry, symbol, series, isin] = fields.as_slice() else {
            return Err(NseError::FieldCount {
                row,
                got: fields.len(),
            });
        };
        let symbol = symbol.trim();
        let isin = isin.trim();
        if symbol.is_empty() {
            return Err(NseError::RowNotIdentified {
                row,
                field: "symbol",
            });
        }
        if isin.is_empty() {
            return Err(NseError::RowNotIdentified { row, field: "ISIN" });
        }
        if out.iter().any(|held| held.symbol == symbol) {
            return Err(NseError::DuplicateSymbol {
                symbol: symbol.to_owned(),
                row,
            });
        }
        out.push(Constituent {
            company: company.trim(),
            industry: industry.trim(),
            symbol,
            series: series.trim(),
            isin,
        });
    }
    Ok(out)
}

/// The same decode, with the header compared explicitly.
///
/// [`constituents`] refuses a non-CSV body first, which is the right order for
/// a fetched document and means a wrong *header* on a real CSV never reaches
/// its own error arm. This entry point exists so that distinction is testable
/// and so a caller holding a body it already knows is a CSV gets the more
/// precise refusal.
///
/// # Errors
///
/// [`NseError::HeaderUnexpected`] naming the header that arrived, and then
/// everything [`constituents`] refuses.
pub fn constituents_strict(body: &str) -> Result<Vec<Constituent<'_>>, NseError> {
    if !looks_like_constituents(body) {
        return Err(NseError::HeaderUnexpected {
            got: first_line(body).trim_end_matches('\r').to_owned(),
        });
    }
    constituents(body)
}

/// One CSV row into its fields, honouring double quotes.
///
/// A company name legitimately contains a comma — `AU Small Finance Bank Ltd.`
/// does not, but plenty do — and the exchange quotes those fields. Splitting on
/// every comma shifts each field after the first quoted one, and the field that
/// ends up wrong is the ISIN, which is the identity. So the quote is honoured
/// here rather than at the call site.
///
/// One pass, one allocation for the output. A doubled quote inside a quoted
/// field (`""`) is the CSV escape for a literal quote and is read as one.
fn split_row(line: &str) -> Vec<&str> {
    let bytes = line.as_bytes();
    let mut fields = Vec::new();
    let mut start = 0usize;
    let mut quoted = false;
    let mut at = 0usize;
    while at < bytes.len() {
        match bytes.get(at) {
            Some(b'"') => {
                // A doubled quote is an escaped quote and does not close the
                // field. Stepping two bytes here is what keeps it inside.
                if quoted && bytes.get(at.saturating_add(1)) == Some(&b'"') {
                    at = at.saturating_add(2);
                    continue;
                }
                quoted = !quoted;
            }
            Some(b',') if !quoted => {
                if let Some(field) = line.get(start..at) {
                    fields.push(field.trim().trim_matches('"'));
                }
                start = at.saturating_add(1);
            }
            _ => {}
        }
        at = at.saturating_add(1);
    }
    if let Some(field) = line.get(start..) {
        fields.push(field.trim().trim_matches('"'));
    }
    fields
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;

    /// The first four rows of a real body, byte for byte.
    ///
    /// Read from `ind_niftybanklist.csv` on 14 Aug 2026 — 916 bytes, header as
    /// below. A fixture copied from a live answer rather than composed, because
    /// a decoder proved only against input this repository wrote is a decoder
    /// proved against its author's assumptions.
    const REAL: &str = "Company Name,Industry,Symbol,Series,ISIN Code\n\
        AU Small Finance Bank Ltd.,Financial Services,AUBANK,EQ,INE949L01017\n\
        Axis Bank Ltd.,Financial Services,AXISBANK,EQ,INE238A01034\n\
        Bank of Baroda,Financial Services,BANKBARODA,EQ,INE028A01039\n\
        Canara Bank,Financial Services,CANBK,EQ,INE476A01022\n";

    #[test]
    fn a_real_body_decodes_to_its_rows_with_the_isin_intact() {
        let rows = constituents(REAL).expect("a real constituent file");
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].symbol, "AUBANK");
        assert_eq!(rows[0].isin, "INE949L01017");
        assert_eq!(rows[0].company, "AU Small Finance Bank Ltd.");
        assert_eq!(rows[0].industry, "Financial Services");
        assert_eq!(rows[0].series, "EQ");
        assert_eq!(rows[3].symbol, "CANBK");
    }

    /// **THE ARM THAT MATTERS MOST.** An interstitial is a 200 with a body, and
    /// a CSV reader handed HTML reports zero members — indistinguishable from
    /// an index that lost every one.
    #[test]
    fn html_is_refused_as_html_and_never_read_as_an_empty_index() {
        let page = "<!doctype html><html><head><title>Access Denied</title>";
        let why = constituents(page).expect_err("HTML is not a constituent file");
        let NseError::NotCsv { ref head } = why else {
            panic!("an interstitial must refuse as NotCsv, got {why:?}");
        };
        assert!(head.contains("doctype"), "the refusal quotes what arrived");
        assert!(
            why.to_string().contains("indistinguishable"),
            "the message says WHY zero rows would have been worse: {why}"
        );
    }

    #[test]
    fn a_header_in_another_order_is_refused_rather_than_read() {
        // Five columns, all the right names, ISIN where the symbol belongs.
        let swapped = "Company Name,Industry,ISIN Code,Series,Symbol\n\
                       A Ltd.,Fin,INE000A01001,EQ,AAA\n";
        let why = constituents_strict(swapped).expect_err("a reordered header");
        assert!(
            matches!(why, NseError::HeaderUnexpected { .. }),
            "got {why:?}"
        );
        assert!(why.to_string().contains("column count"));
    }

    #[test]
    fn a_byte_order_mark_does_not_refuse_a_correct_file() {
        let with_bom = format!("\u{feff}{REAL}");
        let rows = constituents(&with_bom).expect("a BOM is not a defect");
        assert_eq!(rows.len(), 4);
    }

    #[test]
    fn carriage_returns_do_not_refuse_a_correct_file() {
        let crlf = REAL.replace('\n', "\r\n");
        let rows = constituents(&crlf).expect("CRLF is not a defect");
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].isin, "INE949L01017");
    }

    /// A comma inside a quoted company name must not shift the ISIN.
    #[test]
    fn a_quoted_comma_keeps_every_field_in_its_own_column() {
        let tricky = "Company Name,Industry,Symbol,Series,ISIN Code\n\
                      \"Tata Consultancy Services, Ltd.\",IT,TCS,EQ,INE467B01029\n";
        let rows = constituents(tricky).expect("a quoted comma is ordinary");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].company, "Tata Consultancy Services, Ltd.");
        assert_eq!(rows[0].symbol, "TCS");
        assert_eq!(
            rows[0].isin, "INE467B01029",
            "the ISIN is the identity and a shifted column loses it"
        );
    }

    #[test]
    fn an_unquoted_comma_refuses_by_row_rather_than_reading_short() {
        let broken = "Company Name,Industry,Symbol,Series,ISIN Code\n\
                      Tata Consultancy Services, Ltd.,IT,TCS,EQ,INE467B01029\n";
        let why = constituents(broken).expect_err("six fields is not five");
        assert_eq!(why, NseError::FieldCount { row: 1, got: 6 });
        assert!(why.to_string().contains("ISIN"));
    }

    #[test]
    fn one_symbol_twice_refuses_rather_than_collapsing_the_index_by_one() {
        let dup = "Company Name,Industry,Symbol,Series,ISIN Code\n\
                   A Ltd.,Fin,AAA,EQ,INE000A01001\n\
                   A Ltd. again,Fin,AAA,EQ,INE000A01001\n";
        let why = constituents(dup).expect_err("an index holds a name once");
        assert_eq!(
            why,
            NseError::DuplicateSymbol {
                symbol: "AAA".to_owned(),
                row: 2,
            }
        );
    }

    #[test]
    fn a_row_with_no_identity_refuses_naming_which_half_is_missing() {
        let no_symbol = "Company Name,Industry,Symbol,Series,ISIN Code\n\
                         A Ltd.,Fin,,EQ,INE000A01001\n";
        assert_eq!(
            constituents(no_symbol).expect_err("a nameless row"),
            NseError::RowNotIdentified {
                row: 1,
                field: "symbol"
            }
        );
        let no_isin = "Company Name,Industry,Symbol,Series,ISIN Code\n\
                       A Ltd.,Fin,AAA,EQ,\n";
        assert_eq!(
            constituents(no_isin).expect_err("an unjoinable row"),
            NseError::RowNotIdentified {
                row: 1,
                field: "ISIN"
            }
        );
    }

    #[test]
    fn a_header_alone_is_empty_rather_than_malformed() {
        let why = constituents(CONSTITUENT_HEADER).expect_err("no rows");
        assert_eq!(why, NseError::NoRows);
        assert!(
            why.to_string().contains("transport"),
            "an empty document and a failed fetch are different facts: {why}"
        );
    }

    #[test]
    fn a_document_past_the_bound_refuses_before_it_is_parsed() {
        let huge = "x".repeat(MAX_DOCUMENT_BYTES + 1);
        assert_eq!(
            constituents(&huge).expect_err("past the bound"),
            NseError::TooLarge {
                bytes: MAX_DOCUMENT_BYTES + 1,
                cap: MAX_DOCUMENT_BYTES,
            }
        );
    }

    // -- the directory crawl ------------------------------------------------

    #[test]
    fn the_four_categories_carry_distinct_paths_labels_and_counts() {
        let mut paths = Vec::new();
        let mut total = 0usize;
        for category in Category::ALL {
            assert!(category.path().starts_with('/'), "{category}");
            assert!(!category.label().is_empty());
            assert_eq!(category.to_string(), category.label());
            assert!(
                !paths.contains(&category.path()),
                "{category} shares a path"
            );
            paths.push(category.path());
            total = total.saturating_add(category.counted_on_14_aug_2026());
        }
        assert_eq!(
            total, 148,
            "docs/00-charter.md §4d: 22 + 34 + 48 + 44 equity indices"
        );
    }

    #[test]
    fn a_listing_yields_its_indices_once_each_and_no_siblings() {
        let html = r#"
            <a href="/indices/equity/sectoral-indices/nifty-auto">Nifty Auto</a>
            <a href="/indices/equity/sectoral-indices/nifty-bank">Nifty Bank</a>
            <a href='/indices/equity/sectoral-indices/nifty-bank'>Nifty Bank again</a>
            <a href="/indices/equity/sectoral-indices/">the category itself</a>
            <a href="/indices/equity/broad-based-indices/nifty--50">a SIBLING category</a>
        "#;
        let found = index_links(html, Category::Sectoral);
        assert_eq!(
            found.len(),
            2,
            "two indices: the repeat is one statement made twice, the bare \
             category is not an index, and a sibling belongs to its own page"
        );
        assert_eq!(found[0].path, "/indices/equity/sectoral-indices/nifty-auto");
        assert_eq!(found[1].path, "/indices/equity/sectoral-indices/nifty-bank");
        assert!(found.iter().all(|r| r.category == Category::Sectoral));
    }

    /// The exchange writes this link with a doubled slash after the host.
    #[test]
    fn the_constituent_link_is_read_from_the_page_and_never_composed() {
        let page = r#"<a href="https://www.niftyindices.com//IndexConstituent/ind_niftybanklist.csv">Download</a>"#;
        assert_eq!(
            constituent_link(page, "nifty-bank").expect("the page links it"),
            "https://www.niftyindices.com//IndexConstituent/ind_niftybanklist.csv"
        );
    }

    #[test]
    fn a_page_with_no_constituent_link_refuses_instead_of_guessing_a_filename() {
        let page = r#"<a href="/indices/equity/sectoral-indices/nifty-bank">itself</a>"#;
        let why = constituent_link(page, "nifty-bank").expect_err("no link");
        let NseError::NoConstituentLink { ref page } = why else {
            panic!("got {why:?}");
        };
        assert_eq!(page, "nifty-bank");
        // The message must carry BOTH spellings, because the whole argument for
        // refusing is that they disagree with each other.
        let text = why.to_string();
        assert!(text.contains("ind_niftybanklist"), "{text}");
        assert!(text.contains("ind_niftytotalmarket_list"), "{text}");
    }

    #[test]
    fn a_non_csv_link_is_not_mistaken_for_the_constituent_file() {
        let page = r#"<a href="/IndexConstituent/methodology.pdf">Methodology</a>"#;
        assert!(constituent_link(page, "nifty-bank").is_err());
    }

    /// An unquoted or unterminated attribute must not run off the end.
    #[test]
    fn a_malformed_attribute_ends_the_scan_instead_of_reading_past_it() {
        assert!(hrefs_with_prefix("href=/bare/value>", "/bare").is_empty());
        assert!(hrefs_with_prefix("href=\"/never/closed", "/never").is_empty());
        assert!(hrefs_with_prefix("href=", "/x").is_empty());
        assert!(hrefs_with_prefix("", "/x").is_empty());
    }

    #[test]
    fn a_doubled_quote_inside_a_field_is_a_literal_quote() {
        let row = "Company Name,Industry,Symbol,Series,ISIN Code\n\
                   \"The \"\"Big\"\" Co\",Fin,BIG,EQ,INE000A01001\n";
        let rows = constituents(row).expect("an escaped quote is ordinary");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "BIG");
        assert_eq!(rows[0].isin, "INE000A01001");
    }

    #[test]
    fn every_refusal_prints_something_an_operator_can_act_on() {
        let each = [
            NseError::TooLarge { bytes: 9, cap: 8 },
            NseError::NotCsv {
                head: "<html".to_owned(),
            },
            NseError::HeaderUnexpected {
                got: "a,b".to_owned(),
            },
            NseError::FieldCount { row: 3, got: 6 },
            NseError::RowNotIdentified {
                row: 3,
                field: "symbol",
            },
            NseError::DuplicateSymbol {
                symbol: "AAA".to_owned(),
                row: 4,
            },
            NseError::TooManyRows { rows: 11, cap: 10 },
            NseError::NoRows,
            NseError::NoConstituentLink {
                page: "p".to_owned(),
            },
        ];
        for why in each {
            let text = why.to_string();
            assert!(text.len() > 30, "{why:?} says too little: {text}");
            assert!(!text.ends_with("error"), "{why:?} names no reason: {text}");
        }
    }
}

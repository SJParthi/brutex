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

use pull::nseindex::{Basis, Catalogue, Unresolved};
use std::fmt::Write as _;
use std::path::Path;

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
    pub fn read(path: &Path) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(path).map_err(|why| format!("{}: {why}", path.display()))?;
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
        Self {
            matcher: Catalogue::from_names(names),
        }
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
}

/// Join a feed's index symbols to the exchange, sorted by symbol.
#[must_use]
pub fn join<I, S>(nse: &Published, symbols: I) -> Vec<Row>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut rows: Vec<Row> = symbols
        .into_iter()
        .map(|symbol| {
            let symbol = symbol.as_ref().to_owned();
            match nse.matcher.resolve(&symbol) {
                Ok((name, basis)) => Row {
                    symbol,
                    nse: Some(name.to_owned()),
                    basis: Some(basis),
                    why: None,
                },
                Err(why) => Row {
                    symbol,
                    nse: None,
                    basis: None,
                    why: Some(why),
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
#[must_use]
pub fn json(nse: &Published, rows: &[Row]) -> String {
    let resolved = rows.iter().filter(|row| row.nse.is_some()).count();
    let by = |want: Basis| rows.iter().filter(|row| row.basis == Some(want)).count();
    let mut out = String::new();
    let _ = write!(
        out,
        concat!(
            "{{\"published\":{},\"listed\":{},\"resolved\":{},",
            "\"verbatim\":{},\"abbreviated\":{},\"refused\":{},\"rows\":["
        ),
        nse.len(),
        rows.len(),
        resolved,
        by(Basis::Published),
        by(Basis::Abbreviation),
        rows.len() - resolved,
    );
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
        if let Some(why) = row.why {
            let _ = write!(out, ",\"nse\":null,\"why\":\"{}\"", refusal_word(why));
            if let Unresolved::Ambiguous(count) = why {
                let _ = write!(out, ",\"candidates\":{count}");
            }
        }
        out.push('}');
    }
    out.push_str("]}");
    out
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test-only assertions")]
mod tests {
    use super::{Published, Row, join, json};
    use pull::nseindex::{Basis, Unresolved};

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
            join(&held, ["NIFTYAUTOTRUCKS"])
                .first()
                .and_then(|row| row.nse.clone()),
            Some("NIFTY AUTO, TRUCKS".to_owned())
        );
    }

    #[test]
    fn a_resolution_reports_the_name_as_nse_writes_it_not_collapsed() {
        let rows = join(&nse(), ["NIFTY PVT BANK"]);
        assert_eq!(
            rows,
            [Row {
                symbol: "NIFTY PVT BANK".to_owned(),
                nse: Some("NIFTY PRIVATE BANK".to_owned()),
                basis: Some(Basis::Abbreviation),
                why: None,
            }]
        );
    }

    #[test]
    fn rows_are_sorted_and_a_symbol_listed_twice_appears_once() {
        let rows = join(&nse(), ["NIFTY PVT BANK", "INDIA VIX", "NIFTY PVT BANK"]);
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
            [
                "NIFTY PRIVATE BANK",
                "NIFTY PVT BANK",
                "NIFTY100QLTY30",
                "INDIA VIX",
            ],
        );
        let wire = json(&nse, &rows);
        assert!(wire.starts_with(
            "{\"published\":4,\"listed\":4,\"resolved\":2,\
             \"verbatim\":1,\"abbreviated\":1,\"refused\":2,\"rows\":["
        ));
        // Proof, inference, ambiguity with its count, and absence.
        assert!(wire.contains("{\"symbol\":\"NIFTY PRIVATE BANK\",\"nse\":\"NIFTY PRIVATE BANK\",\"basis\":\"published\"}"));
        assert!(wire.contains("{\"symbol\":\"NIFTY PVT BANK\",\"nse\":\"NIFTY PRIVATE BANK\",\"basis\":\"abbreviation\"}"));
        assert!(wire.contains(
            "{\"symbol\":\"NIFTY100QLTY30\",\"nse\":null,\"why\":\"ambiguous\",\"candidates\":2}"
        ));
        assert!(wire.contains("{\"symbol\":\"INDIA VIX\",\"nse\":null,\"why\":\"absent\"}"));
        assert!(wire.ends_with("]}"));
    }

    #[test]
    fn an_empty_join_still_answers_with_its_shape() {
        let nse = nse();
        assert_eq!(
            json(&nse, &[]),
            "{\"published\":4,\"listed\":0,\"resolved\":0,\
             \"verbatim\":0,\"abbreviated\":0,\"refused\":0,\"rows\":[]}"
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
            join(&held, ["NIFTY PVT BANK"])
                .first()
                .and_then(|row| row.basis),
            Some(Basis::Abbreviation)
        );
    }

    #[test]
    fn a_refusal_carries_its_own_variant_and_not_the_others() {
        let rows = join(&nse(), ["INDIA VIX", "NIFTY100QLTY30"]);
        assert_eq!(
            rows.iter().map(|row| row.why).collect::<Vec<_>>(),
            [Some(Unresolved::Absent), Some(Unresolved::Ambiguous(2))]
        );
    }
}

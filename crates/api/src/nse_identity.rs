//! `cash_identity=zerodha_nse_checked`: a Zerodha NSE EQ row confirmed against
//! the exchange's own ISIN-bearing file, with no other vendor involved.
//!
//! # What it proves, and what it does not
//!
//! Zerodha's dump has no ISIN column (charter §8), so under `isin` Zerodha
//! reaches none of the 208 F&O shares. D-0514 added `zerodha_symbol` (the
//! token alone, NOT ISIN-verified) and D-0517 added `zerodha_cross_checked`
//! (the exchange ISIN corroborated by an independent ISIN-bearing vendor
//! master). The operator uses only Zerodha and GDFL, so the second needs a
//! master they do not use.
//!
//! This policy reads the exchange instead. For one F&O share it holds only
//! when all of these hold, and each failure is its own named refusal:
//!
//! 1. the universe member has an NSE ISIN in `core::universe` (the 2026-08-11
//!    transcription of `ind_niftytotalmarket_list.csv`);
//! 2. Zerodha lists exactly one NSE cash EQ row with that symbol, and its
//!    token names no other row;
//! 3. NSE's own Total Market file is on disk, readable, and landed on the same
//!    IST day as the Zerodha dump;
//! 4. that file lists the symbol, in series `EQ`;
//! 5. the ISIN that file prints beside the symbol is the member's ISIN.
//!
//! So: on the day both files were landed, NSE's symbol `X` meant ISIN `I`,
//! and Zerodha's only NSE EQ row was `X`. That ties Zerodha's token to the
//! member through the exchange's own symbol, on one day.
//!
//! It does NOT fill Zerodha's ISIN column and does not claim Zerodha supplied
//! an ISIN (D-0517). It is not `isin`. It is current-snapshot evidence: it
//! proves nothing about historical renames, corporate actions or past
//! membership, and the file carries no date of its own, so "landed today" is
//! when it was fetched, not when NSE last changed it. D-3314.
//!
//! # Cost
//!
//! [`Reference::read`] decodes the file once per masters parse: one pass over
//! at most `pull::nse::MAX_CONSTITUENTS` rows into a pre-sized `HashMap`.
//! [`Reference::confirm`] is then one expected-O(1) hash probe on the Zerodha
//! native index and one on this map. It never scans a master. UNVERIFIED as a
//! gated bound: no bench times it, and the one measurement (D-3314's
//! throwaway run) is recorded in `docs/06-limits.md`.

use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::isin::Isin;
use brutex_core::vendor::{Vendor, VendorId};
use pull::session::Day;

use crate::merge::{Merged, NativeLookup};

/// The file NSE publishes this from, as the exchange names it.
pub const NSE_FILE_NAME: &str = "ind_niftytotalmarket_list.csv";

/// One row of NSE's file, kept for the identity check.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    /// The board series, e.g. `EQ`.
    series: String,
    /// The ISIN exactly as printed.
    raw_isin: String,
}

/// NSE's file, decoded once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NseFile {
    rows: HashMap<String, Row>,
    landed: Day,
    bytes: usize,
    digest: String,
}

/// The NSE file and the day the Zerodha dump beside it was landed, or why
/// either cannot be had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    file: Result<NseFile, String>,
    zerodha_landed: Result<Day, String>,
}

/// Why one member did not resolve under `zerodha_nse_checked`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The universe member has no NSE ISIN in this build.
    NoMemberIsin,
    /// Zerodha lists no NSE cash EQ row with this symbol.
    ZerodhaAbsent,
    /// Zerodha lists the symbol more than once, or shares its token.
    ZerodhaContested,
    /// NSE's file is missing or unreadable.
    FileUnavailable(String),
    /// NSE's file and the Zerodha dump were not landed on the same IST day.
    FileStale {
        /// The day NSE's file was landed.
        nse: Day,
        /// The day the Zerodha dump was landed.
        zerodha: Day,
    },
    /// The Zerodha dump's landing day cannot be read.
    ZerodhaUndated(String),
    /// NSE's file has no row with this symbol.
    SymbolAbsent,
    /// NSE's file lists the symbol in a series other than `EQ`.
    NotEquitySeries(String),
    /// NSE's file prints something that is not an ISIN beside the symbol.
    NseIsinMalformed(String),
    /// NSE's file maps the symbol to a different ISIN than the member's.
    IsinMismatch {
        /// What NSE's file prints.
        nse: Isin,
        /// What the universe member carries.
        member: Isin,
    },
}

impl Refusal {
    /// The refusal as one sentence, naming the symbol.
    #[must_use]
    pub fn sentence(&self, symbol: &str) -> String {
        let why = match *self {
            Self::NoMemberIsin => {
                "this build carries no NSE ISIN for the member, so there is nothing to confirm"
                    .to_owned()
            }
            Self::ZerodhaAbsent => {
                "Zerodha's dump lists no NSE cash EQ row with this symbol".to_owned()
            }
            Self::ZerodhaContested => "Zerodha's dump lists more than one NSE cash EQ token for \
                 this symbol, or shares its token with another row; none is chosen"
                .to_owned(),
            Self::FileUnavailable(ref why) => {
                format!("NSE's {NSE_FILE_NAME} is not usable: {why}")
            }
            Self::FileStale { nse, zerodha } => format!(
                "NSE's {NSE_FILE_NAME} was landed on {nse} and the Zerodha dump on {zerodha}; \
                 the two must be landed on the same IST day. Refresh the masters"
            ),
            Self::ZerodhaUndated(ref why) => {
                format!("the Zerodha dump's landing day cannot be read: {why}")
            }
            Self::SymbolAbsent => format!("NSE's {NSE_FILE_NAME} has no row for this symbol"),
            Self::NotEquitySeries(ref series) => {
                format!("NSE's {NSE_FILE_NAME} lists this symbol in series {series:?}, not EQ")
            }
            Self::NseIsinMalformed(ref raw) => format!(
                "NSE's {NSE_FILE_NAME} prints {raw:?} beside this symbol, which is not an ISIN"
            ),
            Self::IsinMismatch { nse, member } => format!(
                "NSE's {NSE_FILE_NAME} maps this symbol to {nse}, and the universe member is {member}"
            ),
        };
        format!("{symbol}: zerodha_nse_checked refused — {why}")
    }
}

impl Reference {
    /// A reference nothing has read. Every confirmation refuses on it.
    #[must_use]
    pub fn not_read() -> Self {
        Self {
            file: Err("the masters have not been read".to_owned()),
            zerodha_landed: Err("the masters have not been read".to_owned()),
        }
    }

    /// Reads NSE's file and the Zerodha dump's landing day from `dir`.
    ///
    /// Never fails: a missing or unreadable file is kept as the reason, and
    /// every confirmation then refuses with it. A masters parse must not fail
    /// because a file only one explicit policy reads is absent.
    #[must_use]
    pub fn read(dir: &Path) -> Self {
        let zerodha = dir.join(Vendor::Zerodha.master_file());
        Self {
            file: read_file(&dir.join(pull::masters::NSE_TOTAL_MARKET_FILE)).map_err(bounded),
            zerodha_landed: landed_day(&zerodha),
        }
    }

    /// Builds a reference from a body and two days, for a caller that already
    /// holds them.
    ///
    /// # Errors
    ///
    /// The body's decode refusal, as a sentence.
    pub fn from_body(body: &str, nse_landed: Day, zerodha_landed: Day) -> Result<Self, String> {
        Ok(Self {
            file: Ok(decode(body, nse_landed)?),
            zerodha_landed: Ok(zerodha_landed),
        })
    }

    /// What a receipt records about the file, in one line.
    #[must_use]
    pub fn provenance(&self) -> String {
        match self.file {
            Ok(ref file) => format!(
                "NSE {NSE_FILE_NAME} landed {} ({} bytes, {} rows, blake3 {})",
                file.landed,
                file.bytes,
                file.rows.len(),
                file.digest
            ),
            Err(ref why) => format!("NSE {NSE_FILE_NAME} UNAVAILABLE — {why}"),
        }
    }

    /// Confirms one F&O share, returning Zerodha's token for it.
    ///
    /// # Errors
    ///
    /// The first [`Refusal`] in the order the module header lists.
    pub fn confirm(&self, merged: &Merged, symbol: &str) -> Result<VendorId, Refusal> {
        let member = brutex_core::universe::nse_isin(symbol).ok_or(Refusal::NoMemberIsin)?;
        let key = InstrumentKey::cash(Exchange::Nse, symbol).map_err(|_| Refusal::ZerodhaAbsent)?;
        let token = match merged.native_lookup(Vendor::Zerodha, key) {
            NativeLookup::Unique(token) => token,
            NativeLookup::Absent => return Err(Refusal::ZerodhaAbsent),
            NativeLookup::Contested => return Err(Refusal::ZerodhaContested),
        };
        let file = self
            .file
            .as_ref()
            .map_err(|why| Refusal::FileUnavailable(why.clone()))?;
        let zerodha = self
            .zerodha_landed
            .as_ref()
            .map_err(|why| Refusal::ZerodhaUndated(why.clone()))?;
        if file.landed != *zerodha {
            return Err(Refusal::FileStale {
                nse: file.landed,
                zerodha: *zerodha,
            });
        }
        let row = file.rows.get(symbol).ok_or(Refusal::SymbolAbsent)?;
        if row.series != "EQ" {
            return Err(Refusal::NotEquitySeries(row.series.clone()));
        }
        let nse = Isin::new(&row.raw_isin)
            .map_err(|_| Refusal::NseIsinMalformed(row.raw_isin.clone()))?;
        if nse != member {
            return Err(Refusal::IsinMismatch { nse, member });
        }
        Ok(token)
    }
}

/// The most characters of a file refusal that are kept: its head and its tail.
///
/// A refusal can quote the file's first line, and a body with no newline makes
/// that line the whole file, up to 8 MiB. The reason is repeated once per
/// refused member on a receipt, so it is cut to its first [`HEAD_CHARS`] and
/// last [`TAIL_CHARS`] characters, which keep the path and the expected header.
const HEAD_CHARS: usize = 120;
/// See [`HEAD_CHARS`].
const TAIL_CHARS: usize = 240;

fn bounded(why: String) -> String {
    let count = why.chars().count();
    if count <= HEAD_CHARS + TAIL_CHARS {
        return why;
    }
    let head: String = why.chars().take(HEAD_CHARS).collect();
    let cut = count.saturating_sub(HEAD_CHARS + TAIL_CHARS);
    let tail: String = why.chars().skip(HEAD_CHARS + cut).collect();
    format!("{head} … ({cut} characters cut) … {tail}")
}

fn read_file(path: &Path) -> Result<NseFile, String> {
    let held = std::fs::metadata(path)
        .map_err(|why| format!("{} cannot be read — {why}", path.display()))?;
    let cap = u64::try_from(pull::nse::MAX_DOCUMENT_BYTES).unwrap_or(u64::MAX);
    if held.len() > cap {
        return Err(format!(
            "{} is {} bytes; at most {cap} are read",
            path.display(),
            held.len()
        ));
    }
    let landed = landed_day(path)?;
    let body = std::fs::read_to_string(path)
        .map_err(|why| format!("{} cannot be read — {why}", path.display()))?;
    decode(&body, landed).map_err(|why| format!("{} — {why}", path.display()))
}

fn landed_day(path: &Path) -> Result<Day, String> {
    let at: SystemTime = std::fs::metadata(path)
        .and_then(|held| held.modified())
        .map_err(|why| format!("{} has no modification time — {why}", path.display()))?;
    crate::ingest::ist_day(at).map_err(|why| format!("{} — {why}", path.display()))
}

fn decode(body: &str, landed: Day) -> Result<NseFile, String> {
    let rows = pull::nse::constituents_strict(body).map_err(|why| why.to_string())?;
    let mut map = HashMap::with_capacity(rows.len());
    for row in rows {
        // `constituents_strict` already refused a repeated symbol, so every
        // insert is new.
        map.insert(
            row.symbol.to_owned(),
            Row {
                series: row.series.to_owned(),
                raw_isin: row.isin.to_owned(),
            },
        );
    }
    let digest = brutex_core::blake3::hash(body.as_bytes());
    let mut hex = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        use std::fmt::Write as _;
        let _infallible = write!(hex, "{byte:02x}");
    }
    Ok(NseFile {
        rows: map,
        landed,
        bytes: body.len(),
        digest: hex,
    })
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{NSE_FILE_NAME, Reference, Refusal};
    use crate::merge::{Source, merge};
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use brutex_core::isin::Isin;
    use brutex_core::vendor::Listing;
    use brutex_core::vendor::{Vendor, VendorId};
    use pull::session::Day;

    const HEAD: &str = "Company Name,Industry,Symbol,Series,ISIN Code\n";

    fn day(d: u8) -> Day {
        Day::new(2026, 10, d).unwrap()
    }

    fn zerodha(rows: &[(&str, &str)]) -> crate::merge::Merged {
        let kept = rows
            .iter()
            .map(|&(symbol, token)| Listing {
                vendor_id: VendorId::new(token).unwrap(),
                key: InstrumentKey::cash(Exchange::Nse, symbol).unwrap(),
                isin: None,
                unsuffixed: None,
            })
            .collect();
        merge(&[Source {
            vendor: Vendor::Zerodha,
            kept,
            declined: Vec::new(),
        }])
    }

    fn reference(body: &str) -> Reference {
        Reference::from_body(&format!("{HEAD}{body}"), day(6), day(6)).unwrap()
    }

    const RELIANCE: &str = "Reliance Industries Ltd.,Oil,RELIANCE,EQ,INE002A01018\n";

    #[test]
    fn a_member_confirms_through_nse_alone_and_returns_zerodhas_token() {
        let merged = zerodha(&[("RELIANCE", "738561")]);
        let token = reference(RELIANCE).confirm(&merged, "RELIANCE").unwrap();
        assert_eq!(token.as_str(), "738561");
        assert_eq!(
            merged
                .by_key
                .get(&InstrumentKey::cash(Exchange::Nse, "RELIANCE").unwrap())
                .and_then(|entry| entry.vendor_isin(Vendor::Zerodha)),
            None,
            "no ISIN is written into Zerodha's row (D-0517)"
        );
    }

    #[test]
    fn every_required_refusal_is_reached_and_named() {
        let one = zerodha(&[("RELIANCE", "738561")]);
        let cases: Vec<(&str, Reference, crate::merge::Merged, &str, Refusal, &str)> = vec![
            (
                "zerodha absent",
                reference(RELIANCE),
                zerodha(&[("TCS", "1")]),
                "RELIANCE",
                Refusal::ZerodhaAbsent,
                "lists no NSE cash EQ row",
            ),
            (
                "two zerodha rows",
                reference(RELIANCE),
                zerodha(&[("RELIANCE", "738561"), ("RELIANCE", "999")]),
                "RELIANCE",
                Refusal::ZerodhaContested,
                "more than one NSE cash EQ token",
            ),
            (
                "symbol absent from NSE",
                reference("Tata Consultancy,IT,TCS,EQ,INE467B01029\n"),
                zerodha(&[("RELIANCE", "738561")]),
                "RELIANCE",
                Refusal::SymbolAbsent,
                "has no row for this symbol",
            ),
            (
                "different ISIN",
                reference("Reliance,Oil,RELIANCE,EQ,INE467B01029\n"),
                zerodha(&[("RELIANCE", "738561")]),
                "RELIANCE",
                Refusal::IsinMismatch {
                    nse: Isin::new("INE467B01029").unwrap(),
                    member: Isin::new("INE002A01018").unwrap(),
                },
                "maps this symbol to INE467B01029",
            ),
            (
                "not EQ series",
                reference("Reliance,Oil,RELIANCE,BE,INE002A01018\n"),
                zerodha(&[("RELIANCE", "738561")]),
                "RELIANCE",
                Refusal::NotEquitySeries("BE".to_owned()),
                "not EQ",
            ),
            (
                "malformed NSE ISIN",
                reference("Reliance,Oil,RELIANCE,EQ,DUM002A01018\n"),
                zerodha(&[("RELIANCE", "738561")]),
                "RELIANCE",
                Refusal::NseIsinMalformed("DUM002A01018".to_owned()),
                "not an ISIN",
            ),
            (
                "stale",
                Reference::from_body(&format!("{HEAD}{RELIANCE}"), day(5), day(6)).unwrap(),
                zerodha(&[("RELIANCE", "738561")]),
                "RELIANCE",
                Refusal::FileStale {
                    nse: day(5),
                    zerodha: day(6),
                },
                "same IST day",
            ),
            (
                "missing",
                Reference::not_read(),
                zerodha(&[("RELIANCE", "738561")]),
                "RELIANCE",
                Refusal::FileUnavailable("the masters have not been read".to_owned()),
                "is not usable",
            ),
            (
                "no member ISIN (an index)",
                reference(RELIANCE),
                zerodha(&[("NIFTY", "256265")]),
                "NIFTY",
                Refusal::NoMemberIsin,
                "no NSE ISIN for the member",
            ),
        ];
        for (name, reference, merged, symbol, want, words) in cases {
            let got = reference.confirm(&merged, symbol).expect_err(name);
            assert_eq!(got, want, "{name}");
            let sentence = got.sentence(symbol);
            assert!(sentence.starts_with(symbol), "{name}: {sentence}");
            assert!(
                sentence.contains("zerodha_nse_checked refused"),
                "{name}: {sentence}"
            );
            assert!(sentence.contains(words), "{name}: {sentence}");
        }
        assert!(reference(RELIANCE).confirm(&one, "RELIANCE").is_ok());
    }

    #[test]
    fn a_missing_unreadable_or_wrong_file_on_disk_refuses_by_name() {
        let dir = crate::scratch::path("nse-identity-disk");
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let merged = zerodha(&[("RELIANCE", "738561")]);

        let absent = Reference::read(&dir);
        let Err(Refusal::FileUnavailable(why)) = absent.confirm(&merged, "RELIANCE") else {
            panic!("an absent file refuses");
        };
        assert!(why.contains(pull::masters::NSE_TOTAL_MARKET_FILE), "{why}");
        assert!(absent.provenance().contains("UNAVAILABLE"));

        std::fs::write(
            dir.join(pull::masters::NSE_TOTAL_MARKET_FILE),
            "<html>bot check</html>",
        )
        .unwrap();
        std::fs::write(dir.join(Vendor::Zerodha.master_file()), "x").unwrap();
        let Err(Refusal::FileUnavailable(why)) = Reference::read(&dir).confirm(&merged, "RELIANCE")
        else {
            panic!("an unreadable file refuses");
        };
        assert!(why.contains("Company Name"), "the header is named: {why}");

        std::fs::write(
            dir.join(pull::masters::NSE_TOTAL_MARKET_FILE),
            format!("{HEAD}{RELIANCE}"),
        )
        .unwrap();
        let landed = Reference::read(&dir);
        assert_eq!(
            landed.confirm(&merged, "RELIANCE").unwrap().as_str(),
            "738561",
            "both files written now are landed on one day"
        );
        assert!(landed.provenance().contains(NSE_FILE_NAME));
        assert!(landed.provenance().contains("1 rows"));

        std::fs::remove_file(dir.join(Vendor::Zerodha.master_file())).unwrap();
        let Err(Refusal::ZerodhaUndated(why)) = Reference::read(&dir).confirm(&merged, "RELIANCE")
        else {
            panic!("an undated Zerodha dump refuses");
        };
        assert!(why.contains("modification time"), "{why}");

        let big = vec![b'x'; pull::nse::MAX_DOCUMENT_BYTES + 1];
        std::fs::write(dir.join(pull::masters::NSE_TOTAL_MARKET_FILE), big).unwrap();
        let Err(Refusal::FileUnavailable(why)) = Reference::read(&dir).confirm(&merged, "RELIANCE")
        else {
            panic!("an oversized file refuses");
        };
        assert!(why.contains("at most"), "{why}");

        // Exactly at the bound is read, and refused only for what it holds.
        let at_bound = vec![b'x'; pull::nse::MAX_DOCUMENT_BYTES];
        std::fs::write(dir.join(pull::masters::NSE_TOTAL_MARKET_FILE), at_bound).unwrap();
        let Err(Refusal::FileUnavailable(why)) = Reference::read(&dir).confirm(&merged, "RELIANCE")
        else {
            panic!("a file at the bound that is not the constituent file refuses");
        };
        assert!(!why.contains("at most"), "{why}");
        assert!(why.contains("Company Name"), "{why}");
        assert!(why.contains("characters cut"), "the quoted line is bounded");
        assert!(why.chars().count() < 500, "{why}");
    }

    #[test]
    fn a_refusal_is_cut_to_its_head_and_tail_only_past_the_bound() {
        let fits = "a".repeat(super::HEAD_CHARS + super::TAIL_CHARS);
        assert_eq!(super::bounded(fits.clone()), fits);
        let over = format!(
            "{}{}{}",
            "h".repeat(super::HEAD_CHARS),
            "m",
            "t".repeat(super::TAIL_CHARS)
        );
        assert_eq!(
            super::bounded(over),
            format!(
                "{} … (1 characters cut) … {}",
                "h".repeat(super::HEAD_CHARS),
                "t".repeat(super::TAIL_CHARS)
            )
        );
    }

    #[test]
    fn a_refusal_from_a_body_that_is_not_the_file_is_returned() {
        let why = Reference::from_body("not,a,file\n", day(6), day(6)).unwrap_err();
        assert!(why.contains("Company Name"), "{why}");
    }

    #[test]
    fn every_sentence_names_the_file_or_the_vendor() {
        for refusal in [
            Refusal::ZerodhaUndated("x".to_owned()),
            Refusal::NoMemberIsin,
        ] {
            let sentence = refusal.sentence("ABB");
            assert!(sentence.starts_with("ABB: "), "{sentence}");
        }
        assert!(
            Refusal::ZerodhaUndated("why".to_owned())
                .sentence("ABB")
                .contains("Zerodha dump's landing day cannot be read: why")
        );
    }
}

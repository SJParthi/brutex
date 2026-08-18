//! THE SCRUB, RUN OVER A WHOLE VENDOR — the only thing in this product
//! entitled to say a store is verified rather than merely counted.
//!
//! # What this closes
//!
//! `crates/api/src/ladder.rs` says it in as many words: *"A journal says a run
//! once CLAIMED success. The census says the bars are THERE."* Right, and one
//! step short — the census is itself a file, validated only against itself, and
//! until `pull::scrub` landed nothing in this repository had ever opened a bar
//! file to check the counter was telling the truth about it.
//!
//! A failure census over this tree found three ways the store reports a month
//! as held while holding nothing, all sharing that root: a `.bar` lost to a
//! filesystem repair while the entry survives; a deep bar tree lost to an
//! unsynced ancestor while the census one level up survives to describe it; and
//! a counter that simply drifted. Every completeness answer in this product —
//! `/store`, `/db`, the coverage grid, and the ladder gate that decides whether
//! the next rung may run — reads that counter and nothing else.
//!
//! # What it deliberately does not do
//!
//! **It never repairs.** `pull::scrub` compares and this reports; neither
//! writes. A counter silently corrected before an operator saw the
//! disagreement is a repair nobody audited, and `CLAUDE.md` §4 wants the reason
//! surfaced rather than swallowed by a fix.
//!
//! **It never claims more than it looked at.** A scrub of one vendor says
//! nothing about another, and the answer carries the count it examined so a
//! reader can tell a clean store from an empty one.
//!
//! # Cost
//!
//! O(1) per entry — one open, one header read, two record reads at computed
//! offsets, none of which grows with the size of the file or the store. Walking
//! every entry is inherent: you cannot verify a store you do not look at. The
//! per-operation bound `CLAUDE.md` §3 rule 4 fixes is the one this holds.

use std::path::Path;

use brutex_core::vendor::Vendor;
use pull::scrub::{self, Tally};

use crate::census::{Census, VendorCensus};

/// How many disagreements a single answer will name.
///
/// # Why the sentences are capped and the COUNTS are not
///
/// A store whose every entry disagreed would otherwise build a sentence per
/// entry before anything could be shown, and the reader needs the first few and
/// the total — not a hundred thousand paragraphs. [`Report::tally`] counts all
/// of them; this bounds only what is quoted, and [`Report::undrawn`] states how
/// many were not, so a truncated list can never read as a complete one.
pub const MAX_NAMED: usize = 50;

/// What a scrub of one vendor found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Which vendor's counter was checked.
    pub vendor: Option<Vendor>,
    /// Every finding, counted.
    pub tally: Tally,
    /// The first [`MAX_NAMED`] disagreements, in the order the census holds them.
    pub named: Vec<String>,
    /// Disagreements past [`MAX_NAMED`] that this answer does not quote.
    pub undrawn: u64,
    /// Why nothing could be checked, when nothing could be.
    ///
    /// A census that is absent, unreadable or degraded is NOT a clean store,
    /// and the difference is the whole point: `Some` here means the question
    /// was never asked, which must never render as an answer of "no problems".
    pub refused: Option<String>,
}

impl Report {
    /// Whether the counter told the truth about every entry examined.
    ///
    /// **False when nothing could be checked.** A store that refused the
    /// question has not passed it, and a `clean()` that answered `true` for an
    /// unreadable census would be the §4 fallback that hides a failure.
    #[must_use]
    pub fn verified(&self) -> bool {
        self.refused.is_none() && self.tally.clean() && self.tally.seen() > 0
    }

    /// One line, whatever happened.
    #[must_use]
    pub fn say(&self) -> String {
        if let Some(why) = &self.refused {
            return format!("not verified — {why}");
        }
        let t = &self.tally;
        if t.seen() == 0 {
            return "this counter holds no entry, so there was nothing to verify \
                    — which is not the same as a store that checked out clean"
                .to_owned();
        }
        if t.clean() {
            return format!(
                "{} entry(s) checked against their own files and every one agrees",
                t.seen()
            );
        }
        format!(
            "{} of {} entry(s) disagree with the files they describe — {} missing, \
             {} with a different bar count, {} holding other bars, {} unreadable. \
             The counter is what every page and the ladder gate answer from, so \
             these month(s) read as held while the disk says otherwise.",
            t.seen() - t.agreed,
            t.seen(),
            t.missing,
            t.rows,
            t.bounds,
            t.unreadable
        )
    }
}

/// Checks every entry one vendor's counter holds against the file it describes.
///
/// # The symbol id is recomputed, not remembered
///
/// `BarFile::open_existing` refuses a file whose header names a different
/// symbol, which is a check worth keeping: it is what stops one instrument's
/// bars being read as another's after a path is reorganised. The id is derived
/// from the entry's own symbol the same way the writer derived it, so a
/// mismatch here is a real disagreement and not an artefact of asking wrongly.
///
/// # Cost
///
/// O(1) per entry. Nothing is sorted and nothing is read whole.
#[must_use]
pub fn vendor(root: &Path, census: &VendorCensus) -> Report {
    let mut report = Report {
        vendor: Some(census.vendor),
        ..Report::default()
    };

    let Census::Held { ref manifest } = census.state else {
        report.refused = Some(format!(
            "{}'s counter could not be read, so none of its months could be \
             checked against the disk. An unread counter is not a verified one.",
            census.vendor.as_str()
        ));
        return report;
    };

    // A DEGRADED CENSUS IS STILL WORTH SCRUBBING, and saying so is the point:
    // it stepped back to an older generation, so the entries it holds are real
    // and the ones it lost are exactly what a scrub cannot see. The reason
    // rides along rather than stopping the walk.
    let degraded = manifest.degraded_reason().map(|why| why.to_string());

    for entry in manifest.all() {
        // THE SAME DERIVATION THE WRITER USED, so a mismatch is a real
        // disagreement rather than an artefact of asking wrongly.
        // `crates/pull/src/ingest.rs` computes it exactly this way.
        // TRUNCATION IS THE WRITER'S OWN BEHAVIOUR, not an accident here.
        // `crates/pull/src/ingest.rs` derives the id the same way, so this must
        // narrow identically or every file would look like another symbol's.
        #[allow(clippy::cast_possible_truncation)]
        let symbol_id = brutex_core::universe::fnv1a(entry.key.symbol.as_str()) as u32;
        let finding = scrub::one(&entry, root, census.vendor, symbol_id);
        report.tally.count(&finding);
        if finding.agrees() {
            continue;
        }
        if report.named.len() < MAX_NAMED {
            report.named.push(format!(
                "{}-{}-{} {} {} — {}",
                entry.key.exchange.as_str(),
                entry.key.segment.as_str(),
                entry.key.symbol.as_str(),
                entry.key.timeframe.as_str(),
                entry.key.month,
                finding.say()
            ));
        } else {
            report.undrawn += 1;
        }
    }

    if let Some(why) = degraded {
        report.refused = Some(format!(
            "{}'s counter loaded DEGRADED — {why}. The {} entry(s) it still \
             holds were checked and are reported above; the ones it lost cannot \
             be, because a scrub can only ask about a month the counter \
             remembers.",
            census.vendor.as_str(),
            report.tally.seen()
        ));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use pull::scrub::Finding;

    /// Nothing checked is never "verified".
    ///
    /// The dangerous rendering is an empty store reading as a clean one — a
    /// counter that holds nothing has passed no test, and a page that showed it
    /// green would be lying in the one direction that matters.
    #[test]
    fn an_empty_or_refused_report_is_not_a_verified_one() {
        let empty = Report::default();
        assert!(!empty.verified(), "nothing examined is nothing proven");
        assert!(
            empty.say().contains("nothing to verify"),
            "and it says so rather than reporting clean: {}",
            empty.say()
        );

        let refused = Report {
            refused: Some("the counter could not be read".to_owned()),
            ..Report::default()
        };
        assert!(!refused.verified(), "a refusal is not a pass");
        assert!(refused.say().starts_with("not verified"));
    }

    /// A clean tally over real entries is the only thing that verifies.
    #[test]
    fn only_agreement_over_something_verifies() {
        let mut ok = Report::default();
        ok.tally.count(&Finding::Agrees);
        ok.tally.count(&Finding::Agrees);
        assert!(ok.verified(), "two entries checked and both agree");
        assert!(ok.say().contains("every one agrees"), "{}", ok.say());

        let mut bad = Report::default();
        bad.tally.count(&Finding::Agrees);
        bad.tally.count(&Finding::Missing {
            path: "/x".to_owned(),
        });
        assert!(!bad.verified(), "one missing month is not a verified store");
        assert!(
            bad.say()
                .contains("read as held while the disk says otherwise"),
            "the sentence must name the consequence: {}",
            bad.say()
        );
    }
}

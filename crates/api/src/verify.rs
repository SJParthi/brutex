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
//! Per checked entry, one open, one header read and two record reads at
//! computed offsets, none of which grows with the size of the file. Since
//! D-4435 one answer checks one PAGE of the held entries, at most
//! [`MAX_VERIFY_PAGE`] of them from `offset=`, so a request opens at most
//! that many files however large the store is (W1-api5-7, W1-api6-0). The
//! newest-per-key list a page is cut from is `Manifest::newest`, a walk of the
//! census's whole append log; it is built once per census snapshot and kept
//! with it (`Site::verify_memo`), so only the first request after a pull
//! rewrites a manifest pays O(log length). `verify_json` runs on the
//! store-read pool (`detail::run_store_read`), so no runtime worker waits on
//! the opens (D-2281). The measured page is in `docs/06-limits.md`'s D-4435
//! section.

use std::path::Path;
use std::sync::Arc;

use brutex_core::vendor::Vendor;
use pull::manifest::Entry;
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

/// How many held entries one answer opens, and the page size when `limit=` is
/// not given.
///
/// One answer used to open every file the counter held: E_v opens for E_v
/// entries, so the request grew with the store, without limit (W1-api5-7,
/// W1-api6-0). A page of this many is about a fifth of one feed's indices and
/// stocks for one month and one timeframe, and it is the bound the measured
/// p99 in `docs/06-limits.md` is taken at. A larger `limit=` is refused by
/// name rather than silently cut. D-4435.
pub const MAX_VERIFY_PAGE: u64 = 1_024;

/// What a scrub of one vendor found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Which vendor's counter was checked.
    pub vendor: Option<Vendor>,
    /// Every finding, counted.
    pub tally: Tally,
    /// The first [`MAX_NAMED`] entries that did not agree, newest write first:
    /// `Manifest::newest` walks the census's append log backward, so this is
    /// the reverse of append order. It said "in the order the census holds
    /// them" until D-1501 (c4a-9).
    pub named: Vec<String>,
    /// Disagreements past [`MAX_NAMED`] that this answer does not quote.
    pub undrawn: u64,
    /// How many entries the counter holds, newest write per key: the extent
    /// every page is cut from.
    pub held: u64,
    /// The position, in the counter's newest-first order, of the first entry
    /// this answer checked.
    pub offset: u64,
    /// Where the next page starts, when this one stopped short of the end.
    pub next_offset: Option<u64>,
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
        self.refused.is_none() && self.tally.clean() && self.tally.seen() > 0 && self.whole()
    }

    /// Whether this answer checked every entry the counter holds.
    ///
    /// **A page is not the store.** A clean page beside unchecked ones has not
    /// verified the counter, so a page that starts past the first entry or
    /// stops short of the last is never `verified`, however clean.
    #[must_use]
    pub const fn whole(&self) -> bool {
        self.offset == 0 && self.next_offset.is_none()
    }

    /// What a partial page leaves out, or nothing for a whole answer.
    fn part(&self) -> String {
        if self.whole() {
            return String::new();
        }
        let end = self.offset.saturating_add(self.tally.seen());
        let next = self.next_offset.map_or_else(
            || "this is the last page".to_owned(),
            |at| format!("the next page starts at offset={at}"),
        );
        format!(
            " This page checked entries {} to {end} of the {} held; the counter is \
             verified only by an answer that covers all of them, and {next}.",
            self.offset.saturating_add(1),
            self.held
        )
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
        let part = self.part();
        if t.clean() {
            let not = if self.whole() {
                ""
            } else {
                "not verified — "
            };
            return format!(
                "{not}{} entry(s) checked against their own files and every one agrees{}",
                t.seen(),
                if part.is_empty() {
                    String::new()
                } else {
                    format!(".{part}")
                }
            );
        }
        // A MONTH A WRITER HELD WAS NOT CHECKED, AND IS NOT A DISAGREEMENT
        // (W1-api6-7, D-1501). It used to be counted as unreadable and fall into
        // the sentence below, which tells the operator the disk disagrees.
        let busy = if t.busy == 0 {
            String::new()
        } else {
            format!(
                " {} more were held by a writer and not checked; scrub again once \
                 the pull finishes.",
                t.busy
            )
        };
        if t.disagreed() == 0 {
            return format!(
                "not verified — {} of {} entry(s) agree and none disagrees.{busy}{part}",
                t.agreed,
                t.seen()
            );
        }
        format!(
            "{} of {} entry(s) disagree with the files they describe — {} missing, \
             {} with a different bar count, {} holding other bars, {} unreadable. \
             The counter is what every page and the ladder gate answer from, so \
             these month(s) read as held while the disk says otherwise.{busy}{part}",
            t.disagreed(),
            t.seen(),
            t.missing,
            t.rows,
            t.bounds,
            t.unreadable
        )
    }
}

/// The newest write of each key one vendor's counter holds, in the
/// counter's own newest-first order, or `None` when it holds no readable
/// census. Built once per census snapshot by `Site::verify_memo`: it is a
/// pure function of a snapshot that is replaced, never mutated. D-4435.
///
/// # Cost
///
/// `Manifest::newest`: one hash probe and one push per log entry, O(log
/// length), paid on the first request after a pull rewrites the manifest.
#[must_use]
pub fn newest(census: &VendorCensus) -> Option<Arc<Vec<Entry>>> {
    match census.state {
        Census::Held { ref manifest } => Some(Arc::new(manifest.newest())),
        _ => None,
    }
}

/// Checks one page of the entries one vendor's counter holds against the
/// files they describe: `limit` entries of `newest` from `offset`.
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
/// One open, one header read and two record reads per entry of the page, at
/// most [`MAX_VERIFY_PAGE`] of them, and nothing per entry outside it: the
/// page is cut from `newest` by position, not found by a walk. The log walk
/// that built `newest` is [`newest`]'s, once per census snapshot. D-4435,
/// W1-api5-7, W1-api6-0; measured in `docs/06-limits.md`'s D-4435 section.
///
/// # Errors
///
/// A `limit` of zero or above [`MAX_VERIFY_PAGE`], or an `offset` past the
/// held entries, refused by name: a page outside the extent is not answered
/// as an empty clean one.
pub fn vendor(
    root: &Path,
    census: &VendorCensus,
    newest: Option<&[Entry]>,
    offset: u64,
    limit: u64,
) -> Result<Report, String> {
    if limit == 0 || limit > MAX_VERIFY_PAGE {
        return Err(format!(
            "limit={limit} is refused: one answer checks 1 to {MAX_VERIFY_PAGE} \
             entries (MAX_VERIFY_PAGE); page with offset= for more"
        ));
    }
    let mut report = Report {
        vendor: Some(census.vendor),
        offset,
        ..Report::default()
    };

    let (Census::Held { manifest }, Some(newest)) = (&census.state, newest) else {
        report.refused = Some(format!(
            "{}'s counter could not be read, so none of its months could be \
             checked against the disk. An unread counter is not a verified one.",
            census.vendor.as_str()
        ));
        return Ok(report);
    };
    let held = newest.len() as u64;
    report.held = held;
    if offset > held || (offset == held && held > 0) {
        return Err(format!(
            "offset={offset} is past the {held} entry(s) this counter holds"
        ));
    }
    let end = offset.saturating_add(limit).min(held);
    report.next_offset = (end < held).then_some(end);
    let page = usize::try_from(offset)
        .ok()
        .zip(usize::try_from(end).ok())
        .and_then(|(from, to)| newest.get(from..to))
        .ok_or("the verify page does not fit this host's address space")?;

    // A DEGRADED CENSUS IS STILL WORTH SCRUBBING, and saying so is the point:
    // it stepped back to an older generation, so the entries it holds are real
    // and the ones it lost are exactly what a scrub cannot see. The reason
    // rides along rather than stopping the walk.
    let degraded = manifest.degraded_reason().map(|why| why.to_string());

    // THE INDEX, NOT THE LOG — one row per instrument-month, never a history.
    //
    // # The false disagreement this removes
    //
    // This read `manifest.all()`, which walks the append LOG and yields one row
    // per WRITE. The manifest is append-only, so a month backfilled day by day
    // is appended once per day: twenty-one rows for one file, of which twenty
    // describe a file that has since grown. Scrubbing those against the bytes on
    // disk found twenty genuine disagreements and one agreement — measured
    // `seen: 21, agreed: 1, rows: 20` — for a month that was perfectly sound.
    //
    // So `verified: false` was the answer for **every store that had ever been
    // resumed**, which is every real store. And because the noise consumed the
    // report's `MAX_NAMED` budget, the genuine `Missing` months were pushed into
    // `undrawn` — the one class of finding this surface exists to show was the
    // class it hid. A verifier that cries wolf on a healthy store is worse than
    // no verifier, because it trains the reader to ignore it.
    //
    // `Manifest::newest` answers *what is held*; `all` answers *what was
    // written*. Both are real questions and this one is the first.
    //
    // # AND IT IS ALREADY DETERMINISTIC, so nothing is sorted here
    //
    // A report truncated at `MAX_NAMED` that named a different subset on every
    // reload would not be a measurement, and reading a `HashMap`'s values would
    // do exactly that. `Manifest::newest` walks the append log backward instead,
    // keeping the first sighting of each key — the newest write, because the log
    // is append-only — so the order is the file's own and no comparison sort is
    // needed. One hash probe per entry: O(1) per operation, no `log keys`
    // factor. See its header.
    for entry in page {
        // THE SAME DERIVATION THE WRITER USED, so a mismatch is a real
        // disagreement rather than an artefact of asking wrongly.
        // `crates/pull/src/ingest.rs` computes it exactly this way.
        // TRUNCATION IS THE WRITER'S OWN BEHAVIOUR, not an accident here.
        // `crates/pull/src/ingest.rs` derives the id the same way, so this must
        // narrow identically or every file would look like another symbol's.
        #[allow(clippy::cast_possible_truncation)]
        let symbol_id = brutex_core::universe::fnv1a(entry.key.symbol.as_str()) as u32;
        let finding = scrub::one(entry, root, census.vendor, symbol_id);
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
    Ok(report)
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

    /// **A MONTH A WRITER HELD IS NOT A DISAGREEMENT (W1-api6-7, D-1501).**
    ///
    /// Only busy months: not verified, and the sentence says none disagrees
    /// rather than "the disk says otherwise". Busy beside a real disagreement:
    /// the disagreement count excludes the busy one and both are named.
    #[test]
    fn a_busy_month_is_not_verified_and_is_not_called_a_disagreement() {
        let busy = Finding::Busy {
            path: "/x".to_owned(),
        };
        let mut only = Report::default();
        only.tally.count(&Finding::Agrees);
        only.tally.count(&busy);
        assert!(!only.verified(), "a month not checked is not verified");
        let said = only.say();
        assert!(said.starts_with("not verified — 1 of 2"), "{said}");
        assert!(said.contains("none disagrees"), "{said}");
        assert!(said.contains("1 more were held by a writer"), "{said}");
        assert!(!said.contains("disk says otherwise"), "{said}");

        let mut both = only;
        both.tally.count(&Finding::Missing {
            path: "/y".to_owned(),
        });
        let said = both.say();
        assert!(said.starts_with("1 of 3 entry(s) disagree"), "{said}");
        assert!(said.contains("1 more were held by a writer"), "{said}");
    }
}

//! THE COUNTER, CHECKED AGAINST THE FILES IT COUNTS.
//!
//! # The question this module exists to answer
//!
//! *"Irrespective of what goes wrong, how do I know everything is entirely
//! pulled?"* — and the honest answer is that you never trust a run's report,
//! because a run that crashed mid-write and a run that lied are the same
//! bytes. You verify against the disk.
//!
//! `crates/api/src/ladder.rs` already states the principle in as many words:
//! *"A journal says a run once CLAIMED success. The census says the bars are
//! THERE."* That is right, and it stops one step short. The census is itself a
//! file — one counter, validated only against itself — and **nothing in this
//! repository has ever opened a bar file to check that the counter is telling
//! the truth about it.**
//!
//! # What that costs, measured rather than imagined
//!
//! A failure census over this tree found three separate ways the store reports
//! a month as held while holding nothing, all sharing this one root:
//!
//! - a month's `.bar` file deleted, moved to `lost+found` by a filesystem
//!   repair, or lost with a disk, while the manifest keeps its entry. The
//!   ladder gate is `census.entry(k).is_some()` — a probe that opens no file —
//!   so the rung reads satisfied forever and the autopilot never refills it.
//! - a deep bar tree lost to an unsynced ancestor directory while the census,
//!   which lives one level down and *is* fsynced, survives to describe it.
//! - a counter that simply drifted: every "how much do I have" answer on every
//!   page comes from one file that is only ever checked against itself.
//!
//! None of them is exotic and none of them is visible. Each leaves a store that
//! reports complete and is not, which is the only class of failure that
//! silently corrupts a backtest years later.
//!
//! # What a scrub is, and what it deliberately is not
//!
//! It is a **comparison**, never a repair. [`one`] takes a census entry and
//! reports whether the file that entry describes agrees with it. It writes
//! nothing, deletes nothing and rewrites no counter: a repair that runs before
//! an operator has seen the disagreement is a repair nobody audited, and
//! `CLAUDE.md` §4's "degrade loudly and name the reason" wants the reason
//! surfaced rather than swallowed by a fix.
//!
//! It is also not a directory walk. The caller supplies the keys it cares
//! about — the census's own entries, a universe, one instrument — so this
//! module never scans the store and its cost is the caller's list, not the
//! store's size.
//!
//! # Cost
//!
//! **O(1) per entry**, and that is the whole design constraint. One file open,
//! one header read, and at most two record reads at computed offsets — none of
//! which grows with how many bars the file holds or how many months the store
//! has. `CLAUDE.md` §3 rule 4 requires a constant per-operation cost and this
//! is an operation; the 100,000th entry scrubbed costs exactly what the first
//! did. Nothing here sorts, allocates per record, or reads a file whole.

use std::path::Path;

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::path::{FileKind, PathParts, StorePath};

use crate::manifest::Entry;

/// What the disk said when the counter was checked against it.
///
/// # Every arm is a fact about ONE entry
///
/// There is no `Unknown` and no catch-all. A case this module cannot classify
/// is [`Self::Unreadable`], which carries the host's own words — the shape
/// `CLAUDE.md` §4 asks for, since "something went wrong" tells an operator
/// nothing they can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// The file exists and its committed header matches the entry exactly.
    Agrees,
    /// The counter names a month whose file is not there at all.
    ///
    /// THE MOST DANGEROUS ONE, and the reason this module exists. Every
    /// completeness surface in this product answers from the counter, so a
    /// month in this state reads as fully held by the store page, by the
    /// coverage view and by the ladder gate that decides whether the next rung
    /// may run — while there is nothing on the disk at all.
    Missing {
        /// Where the file was looked for.
        path: String,
    },
    /// The file is there and holds a different number of bars than claimed.
    ///
    /// `file` below the entry's `rows` is the truncation case; above it is a
    /// counter that was never updated after an append. Both are reported the
    /// same way because both mean the same thing: the counter is not describing
    /// this file.
    Rows {
        /// What the counter claims.
        counted: u64,
        /// What the file's own committed header says.
        held: u64,
    },
    /// The counts agree and the bars are not the ones the entry describes.
    ///
    /// Caught by reading the first and last record rather than trusting the
    /// count, because a file rewritten with a different month's bars has
    /// exactly the right number of them.
    Bounds {
        /// What the counter claims, as `(first, last)` epoch microseconds.
        counted: (i64, i64),
        /// What the file's own first and last record carry.
        held: (i64, i64),
    },
    /// The file could not be read, and this is the host's reason verbatim.
    Unreadable {
        /// Where.
        path: String,
        /// Why, in the host's words rather than a paraphrase.
        why: String,
    },
}

impl Finding {
    /// Whether this finding means the counter is telling the truth.
    #[must_use]
    pub const fn agrees(&self) -> bool {
        matches!(self, Self::Agrees)
    }

    /// One line an operator can act on.
    ///
    /// Written here rather than at each call site so the three surfaces that
    /// will report a scrub — a page, a log line and a receipt — cannot word the
    /// same disagreement three ways.
    #[must_use]
    pub fn say(&self) -> String {
        match self {
            Self::Agrees => "the counter and the file agree".to_owned(),
            Self::Missing { path } => format!(
                "the counter holds this month and {path} does not exist. Every \
                 completeness answer in this product reads the counter, so this \
                 month reports as fully held while there is nothing on the disk \
                 — and the ladder gate will let the next rung run on it."
            ),
            Self::Rows { counted, held } => format!(
                "the counter says {counted} bar(s) and the file's own committed \
                 header says {held}. The counter is not describing this file."
            ),
            Self::Bounds { counted, held } => format!(
                "the counts agree and the bars do not: the counter spans \
                 {}..{} and the file spans {}..{} in epoch microseconds.",
                counted.0, counted.1, held.0, held.1
            ),
            Self::Unreadable { path, why } => {
                format!("{path} could not be read: {why}")
            }
        }
    }
}

/// Checks ONE census entry against the file it describes.
///
/// # Errors
///
/// Never. Every outcome is a [`Finding`], including the ones a `Result` would
/// express — because a scrub that stopped at the first unreadable file would
/// report less the worse the store was, which is backwards.
///
/// # Cost
///
/// O(1). One open, one header read, two record reads at computed offsets.
#[must_use]
pub fn one(entry: &Entry, root: &Path, vendor: Vendor, symbol_id: u32) -> Finding {
    let parts = PathParts {
        vendor,
        exchange: entry.key.exchange.as_str(),
        segment: entry.key.segment.as_str(),
        symbol: entry.key.symbol.as_str(),
        contract: entry.key.contract,
        timeframe: entry.key.timeframe,
        month: entry.key.month,
        file: FileKind::Bars,
    };
    let path = match StorePath::new(parts) {
        Ok(path) => path,
        Err(why) => {
            return Finding::Unreadable {
                path: format!("{:?}", entry.key.symbol),
                why: format!("the entry does not name a legal store path: {why}"),
            };
        }
    };
    let shown = path.to_path_buf(root).display().to_string();

    let file = match BarFile::open_existing(root, path, symbol_id) {
        Ok(file) => file,
        Err(why) => {
            // A MISSING FILE IS ITS OWN FINDING, not an unreadable one. The two
            // demand different action from an operator — one is "re-pull this
            // month", the other is "look at your disk" — and collapsing them
            // would be the fallback that hides a failure.
            let text = why.to_string();
            if text.contains("Missing") || text.contains("No such file") {
                return Finding::Missing { path: shown };
            }
            return Finding::Unreadable {
                path: shown,
                why: text,
            };
        }
    };

    let held = file.records();
    if held != entry.rows {
        return Finding::Rows {
            counted: entry.rows,
            held,
        };
    }

    // AN EMPTY MONTH THE COUNTER AGREES IS EMPTY has no records to compare, and
    // reading record 0 of it would be the out-of-bounds read this returns early
    // to avoid.
    if held == 0 {
        return Finding::Agrees;
    }

    // THE COUNTS CAN AGREE WHILE THE BARS ARE WRONG. Two reads, at both ends,
    // rather than a walk: a file rewritten with another month's bars has
    // exactly the right number of them, and its first and last instants are
    // what give it away. O(1), and it is the only check here that opens the
    // record region at all.
    let first = file.read_record(0);
    let last = file.read_record(held - 1);
    match (first, last) {
        (Ok(first), Ok(last)) => {
            if first.ts_micros == entry.first_ts_micros && last.ts_micros == entry.last_ts_micros {
                Finding::Agrees
            } else {
                Finding::Bounds {
                    counted: (entry.first_ts_micros, entry.last_ts_micros),
                    held: (first.ts_micros, last.ts_micros),
                }
            }
        }
        (Err(why), _) | (_, Err(why)) => Finding::Unreadable {
            path: shown,
            why: why.to_string(),
        },
    }
}

/// What a scrub of many entries found, without holding any of them.
///
/// # Why this counts rather than collects
///
/// A store with a hundred thousand entries whose every one disagreed would
/// otherwise build a hundred thousand strings before anything could be
/// reported. The counts are what a page shows and what a gate decides on; the
/// SENTENCES belong to the caller, which knows how many it is willing to draw.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    /// Entries whose file agrees with the counter.
    pub agreed: u64,
    /// Entries the counter holds and the disk does not.
    pub missing: u64,
    /// Entries whose file holds a different number of bars.
    pub rows: u64,
    /// Entries whose counts agree and whose bars do not.
    pub bounds: u64,
    /// Entries that could not be read at all.
    pub unreadable: u64,
}

impl Tally {
    /// Folds one finding in. O(1).
    pub const fn count(&mut self, finding: &Finding) {
        match finding {
            Finding::Agrees => self.agreed += 1,
            Finding::Missing { .. } => self.missing += 1,
            Finding::Rows { .. } => self.rows += 1,
            Finding::Bounds { .. } => self.bounds += 1,
            Finding::Unreadable { .. } => self.unreadable += 1,
        }
    }

    /// Every entry looked at.
    #[must_use]
    pub const fn seen(&self) -> u64 {
        self.agreed + self.missing + self.rows + self.bounds + self.unreadable
    }

    /// Whether the counter told the truth about every entry scrubbed.
    ///
    /// **This is the only sentence in this product entitled to say a store is
    /// verified**, and it says it about the entries it was given rather than
    /// about the store — a scrub of one month proves one month.
    #[must_use]
    pub const fn clean(&self) -> bool {
        self.missing == 0 && self.rows == 0 && self.bounds == 0 && self.unreadable == 0
    }
}

/// A contract's own bars are addressed the same way; this exists so a caller
/// need not reach into [`Entry`] to discover that.
#[must_use]
pub const fn contract_of(entry: &Entry) -> Option<Contract> {
    entry.key.contract
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every finding says something an operator can act on, and no two say the
    /// same thing.
    ///
    /// The sentences are the whole product of a scrub — a disagreement nobody
    /// can read is a disagreement nobody acts on — so they are asserted rather
    /// than left to a reviewer's eye.
    #[test]
    fn every_finding_names_what_is_wrong_and_no_two_read_alike() {
        let all = [
            Finding::Agrees,
            Finding::Missing {
                path: "/store/bars/dhan/NSE/INDEX/NIFTY/1m/202607.bar".to_owned(),
            },
            Finding::Rows {
                counted: 375,
                held: 300,
            },
            Finding::Bounds {
                counted: (1, 2),
                held: (3, 4),
            },
            Finding::Unreadable {
                path: "/store/x.bar".to_owned(),
                why: "permission denied".to_owned(),
            },
        ];
        let said: Vec<String> = all.iter().map(Finding::say).collect();
        for (i, one) in said.iter().enumerate() {
            assert!(!one.is_empty(), "finding {i} says nothing");
            for (j, other) in said.iter().enumerate() {
                assert!(
                    i == j || one != other,
                    "findings {i} and {j} read identically: {one}"
                );
            }
        }
        // The dangerous one names the consequence, not just the fact. An
        // operator who reads "file does not exist" and not "and every page
        // still reports this month as held" does not know what it costs.
        // The dangerous finding must name the CONSEQUENCE, not just the fact.
        // An operator who reads "the file does not exist" without "and every
        // page still reports this month as held" does not know what it costs.
        let missing = Finding::Missing {
            path: "/store/x.bar".to_owned(),
        }
        .say();
        assert!(
            missing.contains("reports as fully held"),
            "the missing case must name what the counter goes on claiming: {missing}"
        );
    }

    /// Only `Agrees` agrees. A scrub that treated any other finding as a pass
    /// would be worse than no scrub, because it would carry the word verified.
    #[test]
    fn nothing_but_agreement_counts_as_agreement() {
        assert!(Finding::Agrees.agrees());
        for other in [
            Finding::Missing {
                path: String::new(),
            },
            Finding::Rows {
                counted: 1,
                held: 2,
            },
            Finding::Bounds {
                counted: (0, 0),
                held: (1, 1),
            },
            Finding::Unreadable {
                path: String::new(),
                why: String::new(),
            },
        ] {
            assert!(!other.agrees(), "{other:?} must not read as agreement");
        }
    }

    /// A tally is clean only when nothing disagreed, and every arm can dirty it.
    ///
    /// Asserted arm by arm because the failure mode is a `clean()` that forgets
    /// one variant — which would report a verified store while holding a
    /// finding that says otherwise.
    #[test]
    fn a_tally_is_clean_only_when_every_disagreement_is_zero() {
        let mut empty = Tally::default();
        assert!(empty.clean(), "nothing scrubbed is nothing disagreed");
        assert_eq!(empty.seen(), 0);

        empty.count(&Finding::Agrees);
        assert!(empty.clean(), "agreement does not dirty a tally");
        assert_eq!(empty.seen(), 1);
        assert_eq!(empty.agreed, 1);

        for dirty in [
            Finding::Missing {
                path: String::new(),
            },
            Finding::Rows {
                counted: 1,
                held: 2,
            },
            Finding::Bounds {
                counted: (0, 0),
                held: (1, 1),
            },
            Finding::Unreadable {
                path: String::new(),
                why: String::new(),
            },
        ] {
            let mut t = Tally::default();
            t.count(&Finding::Agrees);
            t.count(&dirty);
            assert!(
                !t.clean(),
                "{dirty:?} must stop a tally reading clean — it is the word \
                 'verified' that is at stake"
            );
            assert_eq!(t.seen(), 2, "every finding is seen, whatever it says");
        }
    }

    /// The tally counts each finding into its own bucket and nowhere else.
    #[test]
    fn each_finding_lands_in_exactly_one_bucket() {
        let mut t = Tally::default();
        t.count(&Finding::Agrees);
        t.count(&Finding::Missing {
            path: String::new(),
        });
        t.count(&Finding::Missing {
            path: String::new(),
        });
        t.count(&Finding::Rows {
            counted: 1,
            held: 2,
        });
        t.count(&Finding::Bounds {
            counted: (0, 0),
            held: (1, 1),
        });
        t.count(&Finding::Unreadable {
            path: String::new(),
            why: String::new(),
        });
        assert_eq!(t.agreed, 1);
        assert_eq!(t.missing, 2);
        assert_eq!(t.rows, 1);
        assert_eq!(t.bounds, 1);
        assert_eq!(t.unreadable, 1);
        assert_eq!(t.seen(), 6, "the buckets account for every finding");
    }
}

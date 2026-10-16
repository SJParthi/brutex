//! Unreadable records, coalesced into runs and capped, so one damaged month
//! costs an answer a constant number of bytes (DB-B75, WP-81).
//!
//! # The failure this closes, measured on a clone of the real store
//!
//! `/bars.json` named every record it could not read in its own sentence, each
//! repeating the month's full path. A GDFL NIFTY 1s month (517,292 bars) whose
//! `.crc` sidecar was lost answered **206 with 188,619,461 bytes** in 1.4 to
//! 1.8 s, every byte of it the same sentence 517,292 times; `/db` then printed
//! the whole string in its "read in part" banner. One lost sidecar made one
//! request O(records) in size and time, which `CLAUDE.md` §3 rule 4 forbids.
//!
//! # What is kept instead
//!
//! A **run** is consecutive records of one file refused for the same reason
//! (`StoreError` equality, compared without formatting). A run is one sentence,
//! `<file> records <first>–<last> (<n>): <why>`, so the path appears once per
//! run and never once per record. At most [`MAX_NAMED_FAULTS`] runs are
//! named, each cut at [`MAX_FAULT_CHARS`]; every later run is counted, and the
//! last sentence says how many runs and records were not named. The COUNT of
//! refused records is always whole: nothing is dropped from the arithmetic,
//! only from the prose (`CLAUDE.md` §4: degrade loudly, never silently).
//!
//! # One design for every bar reader (the merge with D-3349)
//!
//! Two packages bounded this text. This one coalesced record refusals into
//! runs; the sort index's readers (D-3349) name a month once however many of
//! its rows a stretch read left unread, and name each place of a PAGE that
//! could not be served. They are one design, and this log is its one sink:
//!
//! * what grows with the DATA — one refusal per record — comes in through
//!   [`FaultLog::record`] or a `ReadFault::Record` and is a run, capped here;
//! * what a reader has already bounded by the REQUEST — a month named once
//!   with its count ([`FaultLog::counted`]), a month that would not open, a
//!   place of a page ([`FaultLog::push`], [`FaultLog::push_once`]) — is a
//!   sentence kept as it is, in the order it came;
//! * the route bounds the whole list once more where it renders it
//!   (`bars_route::faults_json`), and [`FaultLog::records`] is every refused
//!   record whichever way it was said.
//!
//! # Cost
//!
//! O(1) per refused record: one `StoreError` comparison and two integer
//! updates while a run continues; one sentence formatted when it ends, and at
//! most [`MAX_NAMED_FAULTS`] of those are kept. The size of what this hands a
//! response is bounded by `MAX_NAMED_FAULTS × MAX_FAULT_CHARS` plus one summary
//! sentence, whatever the month holds.
//!
//! UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
//! "Cost claims of the /db fix round that name no proof").

use store::file::StoreError;

/// The most RUNS of unreadable records one answer names. Every run past it
/// is counted in the closing sentence. (A sentence a reader bounded itself —
/// one a month, one a place of a page — is not a run and is not counted
/// against this; the route bounds the list.) A choice, not a vendor fact: enough to show every
/// distinct damaged block of a typical decayed month, small enough that the
/// banner stays a banner.
pub const MAX_NAMED_FAULTS: usize = 16;

/// The longest one fault sentence may be, in bytes, before it is cut at a
/// character boundary and marked with `…`. A store path is under 200 bytes
/// and a `StoreError` sentence under 300.
pub const MAX_FAULT_CHARS: usize = 480;

/// One thing a read could not do: a whole source, in a sentence, or one record
/// of one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadFault {
    /// A month or source that would not open, or any other fault already said
    /// in words.
    Text(String),
    /// One record of one file would not read.
    Record {
        /// The file, as it is shown.
        file: String,
        /// The record's index in the file.
        row: u64,
        /// Why.
        why: StoreError,
    },
}

impl From<String> for ReadFault {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<ReadFault> for String {
    /// The one-record sentence every reader printed before runs existed, kept
    /// for the callers that turn a single refusal into an error with `?`.
    fn from(fault: ReadFault) -> Self {
        match fault {
            ReadFault::Text(text) => text,
            ReadFault::Record { file, row, why } => format!("{file} record {row}: {why}"),
        }
    }
}

/// A run being extended: one file, one cause, consecutive records.
#[derive(Debug, Clone)]
struct Run {
    file: String,
    first: u64,
    last: u64,
    why: StoreError,
    /// The last block a lost sidecar left unverified, when the run spans
    /// more than its first (see [`same_cause`]).
    through_block: Option<u64>,
}

/// Whether `next` is the same fault as `open` for a run.
///
/// Equality, except for one cause that names a block: a lost `.crc` refuses
/// every block of the month, so its refusal carries a different block number
/// every 73 records while the cause — this file, this missing sidecar — is
/// one. Those coalesce, and the sentence says through which block. A
/// checksum MISMATCH stays one run per block: each damaged block is its own
/// fact.
fn same_cause(open: &StoreError, next: &StoreError) -> Cause {
    match (open, next) {
        (
            StoreError::ChecksumsMissing {
                path: a,
                sidecar: c,
                ..
            },
            StoreError::ChecksumsMissing {
                path: b,
                sidecar: d,
                block,
            },
        ) if a == b && c == d => Cause::LostSidecarThrough(*block),
        _ if open == next => Cause::Same,
        _ => Cause::Different,
    }
}

/// [`same_cause`]'s answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cause {
    /// Another fault: the run ends.
    Different,
    /// The same fault exactly.
    Same,
    /// The same lost sidecar, now refusing this block.
    LostSidecarThrough(u64),
}

/// One kept sentence.
#[derive(Debug, Clone)]
struct Said {
    sentence: String,
    /// The records it stands for.
    records: u64,
    /// Whether it is a run of unreadable records (capped) or a sentence its
    /// reader bounded (kept).
    run: bool,
}

/// Faults collected by one request, coalesced and capped.
#[derive(Debug, Clone, Default)]
pub struct FaultLog {
    /// Each kept sentence, in the order it came.
    named: Vec<Said>,
    /// How many of them are runs: what [`MAX_NAMED_FAULTS`] caps.
    runs_named: usize,
    /// The sentence [`Self::push_once`] last took, so the same refusal of the
    /// same month is said once however many rows of a page lie in it.
    said_once: Option<String>,
    /// Every record refused, named or not.
    records: u64,
    /// Every fault (a run counts once, a sentence once) past the cap.
    unnamed: u64,
    /// Records inside the unnamed faults.
    unnamed_records: u64,
    open: Option<Run>,
}

impl FaultLog {
    /// Whether nothing was refused.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.named.is_empty() && self.unnamed == 0 && self.open.is_none()
    }

    /// Every record refused so far, named or not.
    #[must_use]
    pub const fn records(&self) -> u64 {
        self.records
    }

    /// Notes one fault.
    pub fn push(&mut self, fault: impl Into<ReadFault>) {
        match fault.into() {
            ReadFault::Text(text) => {
                self.close();
                self.say(text, 0);
            }
            ReadFault::Record { file, row, why } => self.record_owned(file, row, why),
        }
    }

    /// Notes one fault unless it is, word for word, the sentence this method
    /// was last given: a month that will not open refuses every row of a page
    /// in the same words, and that is one fault, said once. A record of a
    /// month that did open is never "the same": it goes to its run.
    pub fn push_once(&mut self, fault: impl Into<ReadFault>) {
        match fault.into() {
            ReadFault::Text(text) => {
                if self.said_once.as_deref() == Some(text.as_str()) {
                    return;
                }
                self.said_once = Some(text.clone());
                self.close();
                self.say(text, 0);
            }
            ReadFault::Record { file, row, why } => self.record_owned(file, row, why),
        }
    }

    /// Notes ONE sentence that stands for `records` refused records: a reader
    /// that already names a month once (a stretch read, D-3349) hands its line
    /// and its count here, so [`Self::records`] stays whole and the month is
    /// still one sentence.
    pub fn counted(&mut self, sentence: String, records: u64) {
        self.close();
        self.records = self.records.saturating_add(records);
        self.say(sentence, records);
    }

    /// Takes everything `other` noted, after everything this log holds: the
    /// counts add exactly, and the cap is this log's own.
    pub fn absorb(&mut self, mut other: Self) {
        self.close();
        other.close();
        self.records = self.records.saturating_add(other.records);
        for said in other.named {
            if said.run {
                self.keep(said.sentence, said.records);
            } else {
                self.named.push(said);
            }
        }
        self.unnamed = self.unnamed.saturating_add(other.unnamed);
        self.unnamed_records = self.unnamed_records.saturating_add(other.unnamed_records);
    }

    /// Notes record `row` of `file`, refused for `why`, without formatting
    /// anything when it continues the open run.
    pub fn record(&mut self, file: &std::path::Path, row: u64, why: StoreError) {
        if self.extends(row, &why, |open| std::path::Path::new(open) == file) {
            return;
        }
        self.open = Some(Run {
            file: file.display().to_string(),
            first: row,
            last: row,
            why,
            through_block: None,
        });
    }

    fn record_owned(&mut self, file: String, row: u64, why: StoreError) {
        if self.extends(row, &why, |open| open == file) {
            return;
        }
        self.open = Some(Run {
            file,
            first: row,
            last: row,
            why,
            through_block: None,
        });
    }

    /// Counts the record, and extends the open run with it when it is the
    /// next record of the same file for the same cause. Otherwise closes the
    /// open run, so the caller starts the next one.
    fn extends(&mut self, row: u64, why: &StoreError, same_file: impl Fn(&str) -> bool) -> bool {
        self.records = self.records.saturating_add(1);
        if let Some(run) = self.open.as_mut()
            && run.last.checked_add(1) == Some(row)
            && same_file(&run.file)
        {
            match same_cause(&run.why, why) {
                Cause::Different => {}
                Cause::Same => {
                    run.last = row;
                    return true;
                }
                Cause::LostSidecarThrough(block) => {
                    run.last = row;
                    run.through_block = Some(block);
                    return true;
                }
            }
        }
        self.close();
        false
    }

    /// Ends the open run, if any, as one sentence.
    fn close(&mut self) {
        let Some(run) = self.open.take() else {
            return;
        };
        let count = run.last.saturating_sub(run.first).saturating_add(1);
        if self.runs_named >= MAX_NAMED_FAULTS {
            self.unnamed = self.unnamed.saturating_add(1);
            self.unnamed_records = self.unnamed_records.saturating_add(count);
            return;
        }
        // THE SPAN LEADS, THE PATH-HEAVY REASON FOLLOWS: a sentence is cut at
        // `MAX_FAULT_CHARS`, and what a run covers must survive the cut. A lost
        // `.crc` names every block it leaves unverified as one span.
        let blocks = match (&run.why, run.through_block) {
            (StoreError::ChecksumsMissing { block, .. }, Some(through)) if through != *block => {
                format!(", blocks {block}–{through}")
            }
            _ => String::new(),
        };
        let sentence = if count == 1 {
            format!("{} record {}: {}", run.file, run.first, run.why)
        } else {
            format!(
                "{} records {}–{} ({count}){blocks}: {}",
                run.file, run.first, run.last, run.why
            )
        };
        self.keep(sentence, count);
    }

    /// Keeps one run's sentence, or counts it once the cap is reached.
    fn keep(&mut self, sentence: String, records: u64) {
        if self.runs_named >= MAX_NAMED_FAULTS {
            self.unnamed = self.unnamed.saturating_add(1);
            self.unnamed_records = self.unnamed_records.saturating_add(records);
            return;
        }
        self.runs_named += 1;
        self.named.push(Said {
            sentence: capped(sentence),
            records,
            run: true,
        });
    }

    /// Keeps a sentence its reader bounded, as it is.
    fn say(&mut self, sentence: String, records: u64) {
        self.named.push(Said {
            sentence,
            records,
            run: false,
        });
    }

    /// Every sentence, the open run closed, and — when the cap was reached —
    /// one more naming what was counted and not named.
    #[must_use]
    pub fn sentences(mut self) -> Vec<String> {
        self.close();
        let mut said: Vec<String> = self.named.into_iter().map(|said| said.sentence).collect();
        if self.unnamed > 0 {
            said.push(format!(
                "… and {} more fault(s) not named ({} record(s) in them; {} record(s) \
                 refused in all) — at most {MAX_NAMED_FAULTS} runs of unreadable records \
                 are named per answer",
                self.unnamed, self.unnamed_records, self.records
            ));
        }
        said
    }
}

/// `sentence`, cut at [`MAX_FAULT_CHARS`] bytes on a character boundary.
#[must_use]
pub fn capped(mut sentence: String) -> String {
    if sentence.len() <= MAX_FAULT_CHARS {
        return sentence;
    }
    let mut end = MAX_FAULT_CHARS;
    while !sentence.is_char_boundary(end) {
        end -= 1;
    }
    sentence.truncate(end);
    sentence.push('…');
    sentence
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
    use std::path::{Path, PathBuf};

    fn missing(path: &str) -> StoreError {
        StoreError::Missing {
            path: PathBuf::from(path),
            action: store::file::Action::Open,
        }
    }

    /// **ONE LOST SIDECAR IS ONE SENTENCE, NOT 517,292.** The measured month:
    /// every record of a GDFL NIFTY 1s month refused for one reason. The log
    /// names one run, counts every record, and its text is a constant size.
    #[test]
    fn a_whole_month_refused_for_one_reason_is_one_named_run() {
        let file = Path::new("/store/bars/gdfl/NSE/INDEX/NIFTY/1s/2020-07.bin");
        let why = missing("/store/bars/gdfl/NSE/INDEX/NIFTY/1s/2020-07.crc");
        let mut log = FaultLog::default();
        for row in 0..517_292 {
            log.record(file, row, why.clone());
        }
        assert_eq!(log.records(), 517_292);
        let said = log.sentences();
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].contains("records 0–517291 (517292)"), "{said:?}");
        assert_eq!(
            said[0].matches("2020-07.bin").count(),
            1,
            "the path once: {said:?}"
        );
        assert!(said[0].len() <= MAX_FAULT_CHARS + '…'.len_utf8());
    }

    /// **A LOST `.crc` IS ONE RUN ACROSS EVERY BLOCK.** Its refusal names a
    /// different block every 73 records, and the cause is one; a checksum
    /// MISMATCH in two adjacent blocks stays two runs.
    #[test]
    fn a_lost_sidecar_coalesces_across_blocks_and_a_mismatch_does_not() {
        let file = Path::new("/s/2020-07.bin");
        let mut log = FaultLog::default();
        for row in 0..730_u64 {
            log.record(
                file,
                row,
                StoreError::ChecksumsMissing {
                    path: PathBuf::from("/s/2020-07.bin"),
                    sidecar: PathBuf::from("/s/2020-07.crc"),
                    block: row / 73,
                },
            );
        }
        let said = log.sentences();
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].contains("records 0–729 (730)"), "{said:?}");
        assert!(
            said[0].contains("records 0–729 (730), blocks 0–9: "),
            "{said:?}"
        );

        let mut rot = FaultLog::default();
        for row in 73..219_u64 {
            rot.record(
                file,
                row,
                StoreError::BlockChecksum {
                    path: PathBuf::from("/s/2020-07.bin"),
                    block: row / 73,
                    stored: 1,
                    computed: 2,
                },
            );
        }
        let said = rot.sentences();
        assert_eq!(said.len(), 2, "{said:?}");
        assert!(said[0].contains("records 73–145 (73)"), "{said:?}");
        assert!(said[1].contains("records 146–218 (73)"), "{said:?}");
        assert!(!said[0].contains("blocks"), "{said:?}");
    }

    /// Runs split where the reason, the file or the contiguity changes, and
    /// past the cap every fault is still counted.
    #[test]
    fn runs_split_on_reason_file_and_gap_and_the_cap_counts_the_rest() {
        let a = Path::new("/a.bin");
        let b = Path::new("/b.bin");
        let mut log = FaultLog::default();
        log.record(a, 0, missing("/x"));
        log.record(a, 1, missing("/x"));
        log.record(a, 2, missing("/y"));
        log.record(a, 4, missing("/y"));
        log.record(b, 5, missing("/y"));
        let said = log.clone().sentences();
        assert_eq!(said.len(), 4, "{said:?}");
        assert!(said[0].starts_with("/a.bin records 0–1 (2):"), "{said:?}");
        assert!(said[1].starts_with("/a.bin record 2:"), "{said:?}");
        assert!(said[2].starts_with("/a.bin record 4:"), "{said:?}");
        assert!(said[3].starts_with("/b.bin record 5:"), "{said:?}");

        let mut many = FaultLog::default();
        for row in 0..1_000_u64 {
            // Every other record: 500 one-record runs.
            many.record(a, row * 2, missing("/z"));
        }
        assert_eq!(many.records(), 1_000);
        let said = many.sentences();
        assert_eq!(said.len(), MAX_NAMED_FAULTS + 1, "{}", said.len());
        let last = said.last().expect("the summary");
        assert!(
            last.contains(&format!("{} more fault(s)", 1_000 - MAX_NAMED_FAULTS)),
            "{last}"
        );
        assert!(last.contains("1000 record(s) refused in all"), "{last}");
    }

    /// A sentence fault ends the open run and is named in its place; a log
    /// that saw nothing is empty, and one that saw a run is not.
    #[test]
    fn a_text_fault_closes_the_run_and_emptiness_is_exact() {
        let mut log = FaultLog::default();
        assert!(log.is_empty());
        log.push(ReadFault::Record {
            file: "/m.bin".to_owned(),
            row: 7,
            why: missing("/c"),
        });
        assert!(!log.is_empty());
        log.push(ReadFault::Record {
            file: "/m.bin".to_owned(),
            row: 8,
            why: missing("/c"),
        });
        log.push("2026-01: does not open".to_owned());
        let said = log.sentences();
        assert_eq!(said.len(), 2, "{said:?}");
        assert!(said[0].starts_with("/m.bin records 7–8 (2):"), "{said:?}");
        assert_eq!(said[1], "2026-01: does not open");
    }

    /// A long sentence is cut on a character boundary, and one record's fault
    /// still reads as the sentence every reader printed before runs existed.
    /// (The LIST is bounded where it is rendered: `bars_route::faults_json`.)
    #[test]
    fn long_sentences_and_long_lists_are_capped() {
        let long = "₹".repeat(MAX_FAULT_CHARS);
        let cut = capped(long);
        assert!(cut.len() <= MAX_FAULT_CHARS + '…'.len_utf8());
        assert!(cut.ends_with('…'));
        assert_eq!(
            String::from(ReadFault::Record {
                file: "/f".to_owned(),
                row: 3,
                why: missing("/g")
            }),
            format!("/f record 3: {}", missing("/g"))
        );
    }

    /// **A MONTH NAMED ONCE BY ITS READER KEEPS ITS COUNT, THE SAME REFUSAL IS
    /// SAID ONCE, AND ONE LOG TAKES ANOTHER'S EXACTLY** (the merge of DB-B75
    /// with the sort index's one-fault-a-month readers, D-3349). A stretch
    /// read hands one sentence for 73 records: one sentence, 73 counted. A
    /// month that will not open, met on 50 rows of a page, is one sentence. An
    /// absorbed log adds its records, its sentences in order under THIS log's
    /// cap, and what it had already counted without naming.
    #[test]
    fn a_counted_line_a_repeated_refusal_and_an_absorbed_log_keep_every_count() {
        let mut log = FaultLog::default();
        log.counted(
            "/m.bin record 73: bad; 73 kept rows would not read".to_owned(),
            73,
        );
        assert_eq!(log.records(), 73);
        for _ in 0..50 {
            log.push_once("2025-03: another writer holds the month".to_owned());
        }
        log.push_once("2025-04: another writer holds the month".to_owned());
        log.push_once("2025-03: another writer holds the month".to_owned());
        assert_eq!(
            log.records(),
            73,
            "a month that did not open counts no record"
        );
        assert_eq!(
            log.clone().sentences(),
            [
                "/m.bin record 73: bad; 73 kept rows would not read",
                "2025-03: another writer holds the month",
                "2025-04: another writer holds the month",
                "2025-03: another writer holds the month",
            ],
            "only the sentence JUST said is not said again"
        );

        // A RECORD GIVEN TO `push_once` GOES TO ITS RUN, never dropped.
        let mut records = FaultLog::default();
        for row in 0..5 {
            records.push_once(ReadFault::Record {
                file: "/f".to_owned(),
                row,
                why: missing("/f"),
            });
        }
        assert_eq!(records.records(), 5);
        assert_eq!(records.clone().sentences().len(), 1);

        // ABSORBED: an open run of the taker and of the taken both close; the
        // taken log's runs come under THIS log's cap, its sentences are kept.
        let mut other = FaultLog::default();
        for n in 0..40_u64 {
            other.counted(format!("month {n}"), 10);
        }
        for n in 0..(MAX_NAMED_FAULTS as u64 + 4) {
            // Every other record of /g: one-record runs.
            other.record(std::path::Path::new("/g"), n * 2, missing("/g"));
        }
        log.absorb(records);
        log.absorb(other);
        let total = 73 + 5 + 40 * 10 + (MAX_NAMED_FAULTS as u64 + 4);
        assert_eq!(log.records(), total);
        let said = log.sentences();
        // 4 sentences, 1 run, 40 month lines, then the runs of /g: the cap is
        // 16 runs in all, so 15 of /g's 20 are named and 5 counted.
        assert_eq!(said.len(), 4 + 1 + 40 + (MAX_NAMED_FAULTS - 1) + 1);
        assert_eq!(said[4], format!("/f records 0–4 (5): {}", missing("/f")));
        assert_eq!(said[5], "month 0");
        assert_eq!(
            said[44], "month 39",
            "a month line is never dropped for the cap"
        );
        assert_eq!(said[45], format!("/g record 0: {}", missing("/g")));
        assert_eq!(
            *said.last().unwrap(),
            format!(
                "… and 5 more fault(s) not named (5 record(s) in them; {total} record(s) \
                 refused in all) — at most {MAX_NAMED_FAULTS} runs of unreadable records \
                 are named per answer"
            )
        );
    }
}

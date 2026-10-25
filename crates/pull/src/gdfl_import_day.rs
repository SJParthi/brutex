//! The importer's day loop (R3-35, a move-only split of `gdfl_import.rs`):
//! the run's context, the drive over every day, `admit` and the done-day
//! skip, the reach-back and widening passes, a day's failures and how it
//! closes.

use std::collections::{HashMap, HashSet};

use crate::fetch::BarRequest;
use crate::gdfl_cm::{CmRefusal, session_gate};
use crate::ingest::Failure;
use crate::session::{Day, Window};
use crate::vendor::Granularity;

use super::causes::{
    Cause, HOLE_REASONS, ImportRefusal, Outcome, REPAIRABLE_HOLES, Stage, cm_cause, import_cause,
};
use super::census::{Census, StoreCensus, stop_at_census};
use super::filing::{DayFiler, make_durable, stop_at_durability};
use super::journal::{BAR_DEFINITION, Journal, ScopeHeld, closing_line, journal_path, run_line};
#[cfg(test)]
use super::tests;
use super::{
    Ask, DayExtras, DayRead, FileFacts, Handed, ImportKind, Names, Origin, Refused, Report, Run,
    RunOptions, TARGET, TickFile, failure, note_failure, note_sampled, quality, scope,
};

/// Every day of `from..=to`.
pub(super) fn days(from: Day, to: Day) -> Vec<Day> {
    (from.days_from_epoch()..=to.days_from_epoch())
        .filter_map(|n| Day::from_days(n).ok())
        .collect()
}

/// One failure of a day and its cause (D-3317).
#[derive(Debug, Clone)]
pub(super) struct Staged {
    pub(super) cause: Cause,
    pub(super) failure: Failure,
}

/// Names every one of a day's failures in the run's report, each with its
/// cause (D-4839, [`Report::name`]): the one door a failure enters a report
/// by.
pub(super) fn name_staged(report: &mut Report, failures: Vec<Staged>) {
    for Staged { cause, failure } in failures {
        report.name(cause, failure);
    }
}

/// A day's failure about `about`, refused by `refusal`, at the one stage
/// [`import_cause`] gives its variant (IA-2, IA-GR-16: a call site cannot
/// stage a variant two ways), logged: [`note_failure`] emits its `Error`
/// event, which is why the name says `note_` (gate 19 reads it).
pub(super) fn note_staged(run: &Run<'_>, day: Day, about: &str, refusal: &ImportRefusal) -> Staged {
    let cause = import_cause(refusal);
    Staged {
        cause,
        failure: note_failure(run.kind, day, about, &refusal.to_string(), cause),
    }
}

/// How a day closes, with the cause its line names (D-3317 items 2, 3, 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Close {
    /// `done`, no failure.
    Done,
    /// `done ... vendor_refused=<n> reason=<code>`.
    WithRefusals(&'static str),
    /// `incomplete ... stage= reason=`.
    Incomplete(Cause),
    /// `refused ... stage= reason=`.
    Refused(Cause),
}

impl Close {
    /// The outcome a reader names this close by.
    pub(super) const fn outcome(self) -> Outcome {
        match self {
            Self::Done => Outcome::Done,
            Self::WithRefusals(_) => Outcome::DoneWithRefusals,
            Self::Incomplete(_) => Outcome::Incomplete,
            Self::Refused(_) => Outcome::Refused,
        }
    }
}

/// The day's decisive failure: the census's, else the run's (a stop, IA-2,
/// D-4853: a stopped day says `reason=stopped` even when an earlier file
/// failed), else the first that keeps the day from closing AND IS NOT A
/// HOLE, else the first hole, else the first (D-3317: its `stage=` and
/// `reason=`).
///
/// **NON-HOLE BEFORE HOLE (IA-16, round 4 BL-C).** A day whose open failures
/// include an instrument Y that met a store hole (its month already holds
/// later days) and an instrument X whose month was merely busy must close
/// with X's cause: closed with Y's hole reason, `admit` would skip the day
/// for ever and X's day D would never be read again, while X's later days
/// (held back in this run) land on the next. A hole closes a day only when
/// every open failure of the day is a hole. O(failures).
pub(super) fn decisive(failures: &[Staged]) -> Option<&Staged> {
    failures
        .iter()
        .find(|one| one.cause.stage == Stage::Census)
        .or_else(|| failures.iter().find(|one| one.cause.stage == Stage::Run))
        .or_else(|| {
            failures
                .iter()
                .find(|one| !one.cause.closes && !one.cause.is_hole())
        })
        .or_else(|| failures.iter().find(|one| !one.cause.closes))
        .or_else(|| failures.first())
}

/// How a day with `failures` closes, `read` when at least one of its files
/// was read clean: no failure is done; a census failure is incomplete; a day
/// none of whose files was read clean (every wanted file refused, or the
/// listing) is refused (D-3317 item 3); a day whose every failure closes is
/// done with refusals (item 2); any other is incomplete. A day whose clean
/// files held no traded second wrote nothing and still read them: it is not
/// "every file refused", so it closes by its failures like any other day.
/// O(failures).
pub(super) fn close_of(failures: &[Staged], read: bool) -> Close {
    let Some(first) = decisive(failures) else {
        return Close::Done;
    };
    let cause = first.cause;
    if matches!(cause.stage, Stage::Census | Stage::Run) {
        // A stopped day whose `begin` was written is open, whatever its
        // files did (D-4853): never a red `refused` day.
        Close::Incomplete(cause)
    } else if !read {
        Close::Refused(cause)
    } else if cause.closes {
        // The decisive failure is the first that does not close, when there
        // is one: this one closing means every one does.
        Close::WithRefusals(cause.code)
    } else {
        Close::Incomplete(cause)
    }
}

/// The counts of one closing line, the D-2822 fields in their order.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Counts {
    pub(super) files: usize,
    pub(super) seconds: usize,
    pub(super) failures: usize,
    pub(super) late: usize,
    pub(super) late_unresolved: usize,
    pub(super) max_back_s: u32,
    /// Entries of the day that were not wanted files (DB-A49): counted on
    /// the close for ever, so a skipped file outlives the event log.
    pub(super) skipped: usize,
    /// What the day's listing was (DB-A49): `None` when no listing was
    /// read (a day refused before one, or one whose listing refused).
    pub(super) listing: Option<crate::gdfl_cm::ListingSum>,
}

/// The report's counters at a day's start, so the day's own counts are a
/// subtraction (the design's seam 3, WP-16): a `Copy` of seven numbers where
/// this was a clone of the whole report, its failures and day lists
/// included, once per day.
#[derive(Debug, Clone, Copy)]
pub(super) struct Mark {
    pub(super) files: usize,
    pub(super) files_skipped: usize,
    pub(super) rows: usize,
    pub(super) seconds: usize,
    pub(super) seconds_committed: usize,
    pub(super) late_rows: usize,
    pub(super) late_unresolved: usize,
    pub(super) failures: usize,
}

impl Mark {
    /// `report`'s counters now.
    const fn of(report: &Report) -> Self {
        Self {
            files: report.files,
            files_skipped: report.files_skipped,
            rows: report.rows,
            seconds: report.seconds,
            seconds_committed: report.seconds_committed,
            late_rows: report.late_rows,
            late_unresolved: report.late_unresolved,
            failures: report.failures_total,
        }
    }
}

impl Counts {
    /// The day's counts: what `report` holds beyond `before`.
    const fn of_day(
        before: Mark,
        report: &Report,
        (failures, max_back_s): (usize, u32),
        listing: Option<crate::gdfl_cm::ListingSum>,
    ) -> Self {
        Self {
            files: report.files - before.files,
            skipped: report.files_skipped - before.files_skipped,
            listing,
            seconds: report.seconds - before.seconds,
            failures,
            late: report.late_rows - before.late_rows,
            late_unresolved: report.late_unresolved - before.late_unresolved,
            max_back_s,
        }
    }

    /// The D-2822 stats, as `done` and `incomplete` write them, then what
    /// the listing was (DB-A49): ` skipped=<n> entries=<n> listing=<hex>`,
    /// or ` skipped=<n>` alone when no listing was read. A reader finds
    /// each by name (`count_of`), so no earlier field moved.
    pub(super) fn text(&self) -> String {
        let listing = self.listing.map_or_else(String::new, |sum| {
            format!(" entries={} listing={}", sum.entries, sum.hex())
        });
        format!(
            "definition={BAR_DEFINITION} files={} seconds={} failures={} late={} \
             late_unresolved={} max_back_s={} skipped={}{listing}",
            self.files,
            self.seconds,
            self.failures,
            self.late,
            self.late_unresolved,
            self.max_back_s,
            self.skipped,
        )
    }
}

/// What every day of one run shares: the run, its journal, its options and
/// its `run` line, written before the run's first other line (D-3317 item 4).
pub(super) struct Ctx<'a> {
    pub(super) run: &'a Run<'a>,
    pub(super) journal: &'a Journal,
    pub(super) options: RunOptions<'a>,
    /// The run's id: `started`, raised past every id the journal holds.
    pub(super) id: u64,
    pub(super) started: u64,
    /// Whether the `run` line is in the journal.
    pub(super) announced: std::cell::Cell<bool>,
    /// The scope this run records with every close and what a done day owes
    /// it (WP-12f, D-4878); `None` for a run whose members are not recorded
    /// (an options run, a test's bare reader).
    pub(super) scoped: Option<Scoped<'a>>,
    /// THE HELD-BACK SET (IA-16; D-4851, IA-GR-18): each instrument-month a
    /// file of which failed in this run with a cause that keeps its day open
    /// and is not a hole, with the day it failed on. Every later file of it
    /// is not filed (`held_back`), so its month never takes a later day while
    /// an earlier one is owed. At most [`super::HELD_BACK_CAP`]; one hash
    /// probe per file, made before the file is read (and so before its
    /// `.auc` append, round 5 SF-11). Ends with the run.
    pub(super) held_back: std::cell::RefCell<HashMap<super::HeldKey, Day>>,
    /// THE STORE ROOT'S IDENTITY (IA-16, RA-284, D-4852): its device and
    /// inode numbers when the run began, or, for a store whose root the run
    /// itself made, when the first day after it was asked. The loss probe
    /// names a root that is another directory now as lost.
    pub(super) store_id: std::cell::Cell<Option<super::filing::StoreId>>,
    /// THE ONCE-PER-MONTH GATHER (IA-17b, D-4875 (3), D-4899): the files of
    /// the month the run is in that its months take only as a new
    /// generation, and the days whose closes wait for them. Repaired once
    /// per (instrument, month) when the run leaves the month, ends or is
    /// stopped ([`flush_gather`]). Ends with the run; a run that ends any
    /// other way leaves those days begun, for the next run to read again.
    pub(super) gather: std::cell::RefCell<super::filing::Gather>,
}

/// How many distinct scopes, and distinct other filters, a run's maps are
/// sized for before they grow: the operator's few (the whole tree, `--stored`
/// and a named symbol or two). A run that meets more grows them amortised.
const SCOPES_HINT: usize = 8;

/// A run's recorded scope beside the scope ledger (WP-12f, D-4878).
pub(super) struct Scoped<'a> {
    /// What this run is asked to read.
    pub(super) record: &'a scope::Record,
    /// The swept surface of the run's kind: what a close with no `scope=`
    /// is read as built under, the scope every build filed at least.
    pub(super) swept: std::sync::Arc<HashSet<String>>,
    /// The scope ledger, read at the run's start.
    pub(super) ledger: std::cell::RefCell<scope::Ledger>,
    /// Per scope a done close names (`None`: it names none), the members
    /// this run's scope holds beyond it, computed once per run.
    pub(super) beyond: std::cell::RefCell<HashMap<Option<String>, std::sync::Arc<HashSet<String>>>>,
    /// Per scope a whole-tree close of the auction era names (`""`: it names
    /// none, the swept surface), the shares its members are filed under,
    /// computed once per run ([`Ctx::held`]).
    pub(super) shares: std::cell::RefCell<HashMap<String, std::sync::Arc<HashSet<String>>>>,
}

/// What a day closed `done` still owes this run's scope (D-4878).
pub(super) enum Owed {
    /// Nothing: its close covered every member, or the day was examined
    /// under this scope and every member key it opened is done.
    Nothing,
    /// The members whose own keys the day already has and are not done.
    Pending(Vec<String>),
    /// The members its close never covered: the day's listing is read once
    /// to find the ones that have a file.
    Probe(std::sync::Arc<HashSet<String>>),
}

impl Ctx<'_> {
    /// Whether a whole-tree close under `scope` (`""`: none named, the swept
    /// surface) read `share`: its scope holds a member filed under it
    /// ([`crate::gdfl_cm::member_share`]). The share set of each scope is
    /// built once a run from the scope ledger, then one probe. A run with no
    /// scope, and a digest the ledger does not hold, answer `false`: that
    /// close is not known to have read the share, so its restatement stands
    /// (D-4878 item 10, check 5 of WP-12f).
    pub(super) fn held(&self, scope: &str, share: &str) -> bool {
        let Some(scoped) = &self.scoped else {
            return false;
        };
        if let Some(shares) = scoped.shares.borrow().get(scope) {
            return shares.contains(share);
        }
        let shares: HashSet<String> = {
            let ledger = scoped.ledger.borrow();
            let members = if scope.is_empty() {
                Some(&scoped.swept)
            } else {
                ledger.members_of(scope)
            };
            members
                .into_iter()
                .flat_map(|members| members.iter())
                .map(|member| crate::gdfl_cm::member_share(member).to_owned())
                .collect()
        };
        let answer = shares.contains(share);
        scoped
            .shares
            .borrow_mut()
            .insert(scope.to_owned(), std::sync::Arc::new(shares));
        answer
    }

    /// Whether `key` of `day` would read a share whose months may hold a bar
    /// definition 4 no longer builds ([`restated_past_append`]), each
    /// whole-tree close's scope asked of this run's ledger ([`Ctx::held`]).
    fn restated(&self, key: &str, day: Day) -> bool {
        restated_past_append(
            self.journal,
            &|scope, share| self.held(scope, share),
            self.run.kind,
            key,
            day,
        )
    }

    /// What `key`'s day, closed `done`, owes this run's scope: O(1) for a
    /// day that owes nothing (one compare of digests, two hash probes), and
    /// the members beyond a close's scope are computed once per scope; both
    /// are asserted over a journal of every calendar day 2018..2026 by
    /// `pull::gdfl_import::tests::scope_reopen::what_a_done_day_owes_is_decided_without_walking_the_journal_or_the_scope_again`
    /// (one shared set by address, none built for an examined day).
    ///
    /// `remembered` is the key the scope ledger holds the examination under:
    /// `key` itself for the run's own key. For a key another filter closed,
    /// what it owes is the same, and whether this run's scope has examined
    /// it for this run's filter is remembered under a key of that pair
    /// ([`Asked::remembered`]), never under the key itself, whose own
    /// examination is its own filter's to write.
    ///
    /// A KEY CLOSED DONE WITH REFUSALS UNDER `retry_refused` OWES NOTHING
    /// ONLY WHERE THE RUN READS ITS DAY AGAIN WHOLE: the run's own key inside
    /// its range, which [`admit`] reads again under the run's scope. A day
    /// before the range is never read whole (the run was not asked for it),
    /// so there its close still owes what its scope never covered and is
    /// asked like any other done day. Answering `Nothing` there let a run
    /// with the flag make each new member's month from the later days while
    /// that day still owed it, unsaid (check 3 of WP-12f, D-4878 item 6).
    pub(super) fn owed(&self, asked: &Asked<'_>) -> Owed {
        let (key, remembered) = (asked.key, asked.remembered);
        let Some(scoped) = &self.scoped else {
            return Owed::Nothing;
        };
        let read_whole = !asked.early && asked.own();
        let retried =
            read_whole && self.options.retry_refused && self.journal.with_refusals.contains(key);
        if !self.journal.done.contains(key) || retried {
            return Owed::Nothing;
        }
        let built = self.journal.scope_done.get(key);
        let now = scoped.record.digest();
        if built.is_some_and(|digest| digest == now) {
            return Owed::Nothing;
        }
        let done = |member: &String| self.settled(&format!("{key}+{member}"));
        let ledger = scoped.ledger.borrow();
        if ledger.examined_under(remembered) == Some(now) {
            let pending: Vec<String> = self
                .journal
                .member_keys
                .get(key)
                .into_iter()
                .flatten()
                .filter(|member| scoped.record.set().contains(*member) && !done(member))
                .cloned()
                .collect();
            return if pending.is_empty() {
                Owed::Nothing
            } else {
                Owed::Pending(pending)
            };
        }
        let beyond = std::sync::Arc::clone(
            scoped
                .beyond
                .borrow_mut()
                .entry(built.cloned())
                .or_insert_with(|| {
                    let old = built.and_then(|digest| ledger.members_of(digest));
                    if built.is_some() && old.is_none() {
                        let _dropped_when_filtered = telemetry::emit(
                            &telemetry::Event::warn(
                                TARGET,
                                "a close names a scope the scope ledger does not hold; \
                                 it is read as the swept surface",
                            )
                            .with("kind", self.run.kind.as_str())
                            .with("code", "scope_unrecorded"),
                        );
                    }
                    let old = old.unwrap_or(&scoped.swept);
                    std::sync::Arc::new(
                        scoped
                            .record
                            .members()
                            .iter()
                            .filter(|member| !old.contains(*member))
                            .cloned()
                            .collect(),
                    )
                }),
        );
        if beyond.is_empty() {
            Owed::Nothing
        } else {
            Owed::Probe(beyond)
        }
    }

    /// Whether a member key needs no read by this run: closed `done`, and not
    /// one closed with refusals that the run was asked to read again
    /// ([`RunOptions::retry_refused`]), as a base key is in [`admit`]. Two
    /// hash probes.
    fn settled(&self, member_key: &str) -> bool {
        let retried = self.options.retry_refused && self.journal.with_refusals.contains(member_key);
        self.journal.done.contains(member_key) && !retried
    }

    /// Writes the run's `run` line when no line of this run is in the
    /// journal yet (D-3317 item 4): before its first other line, and before
    /// the scope ledger names the run's id (D-4878), so no id is written
    /// anywhere that the journal does not hold and the next run cannot take.
    fn announce(&self) -> Result<(), ImportRefusal> {
        if !self.announced.get() {
            self.journal
                .append(&run_line(self.id, self.started, self.options.origin))?;
            self.announced.set(true);
        }
        Ok(())
    }

    /// The `scope=` field a closing line of `key` carries: this run's digest
    /// on a base key's `done` or `incomplete`, after its member list is in
    /// the scope ledger. A member key names its one member and carries none.
    fn scope_field(&self, key: &str, close: Close) -> Result<String, ImportRefusal> {
        let Some(scoped) = &self.scoped else {
            return Ok(String::new());
        };
        if matches!(close, Close::Refused(_)) || key.contains('+') {
            return Ok(String::new());
        }
        scoped
            .ledger
            .borrow_mut()
            .record(self.run.kind.as_str(), scoped.record)
            .map_err(|why| ImportRefusal::Journal { why })?;
        Ok(format!(" scope={}", scoped.record.digest()))
    }

    /// Appends `line`, after the run's `run` line when it is the run's first,
    /// and answers its line number in the journal, from 1.
    pub(super) fn append(&self, line: &str) -> Result<u64, ImportRefusal> {
        self.announce()?;
        self.journal.append(line)
    }

    /// Writes a day's one closing line and says how the day closed. A journal
    /// that will not take it is one more named failure of the day, in
    /// `failures`, and the day closed nowhere: `incomplete`, imported again by
    /// the next run.
    ///
    /// A `refused` close identical, but for its `run=`, to the key's last line
    /// is not written again: the journal already says exactly that, and a
    /// re-run over a day the calendar or the source refuses for good (a
    /// special session, a day the vendor never published) then writes
    /// nothing, as D-2803 asks of a re-run that changes nothing.
    pub(super) fn close(
        &self,
        (key, day): (&str, Day),
        close: Close,
        counts: &Counts,
        failures: &mut Vec<Staged>,
    ) -> Outcome {
        // The scope's member list is in the scope ledger before the first
        // line that names it (D-4878); a ledger that will not take it leaves
        // the day closed nowhere, as a journal that will not take its line.
        let scope = match self.scope_field(key, close) {
            Ok(scope) => scope,
            Err(why) => {
                failures.push(note_staged(self.run, day, "scope ledger", &why));
                return Outcome::Incomplete;
            }
        };
        let unrun = closing_line(key, close, counts, &scope);
        if matches!(close, Close::Refused(_)) && self.journal.refused_close.get(key) == Some(&unrun)
        {
            return close.outcome();
        }
        // THE CLOSE'S CLOCK TIME (DB-A50): a run the runs ledger knows says when
        // each day closed, before its `run=`, which stays the line's last field.
        let at = self
            .options
            .run
            .map(|_| format!(" at={}", now_unix_s()))
            .unwrap_or_default();
        match self.append(&format!("{unrun}{at} run={}", self.id)) {
            Ok(_) => close.outcome(),
            Err(why) => {
                failures.push(note_staged(self.run, day, "journal", &why));
                Outcome::Incomplete
            }
        }
    }
}

/// Writes an admitted day's one closing line and says how the day closed
/// (D-3317): `done`, done with refusals, `incomplete` or `refused`, by
/// [`close_of`]. Every admitted day the source holds gets one, begun or not:
/// until D-3317 a day whose every file was refused left no line at all.
pub(super) fn close_day(
    ctx: &Ctx<'_>,
    (key, day): (&str, Day),
    (before, report, max_back_s): (Mark, &Report, u32),
    listing: Option<crate::gdfl_cm::ListingSum>,
    failures: &mut Vec<Staged>,
) -> Outcome {
    let counts = Counts::of_day(before, report, (failures.len(), max_back_s), listing);
    ctx.close(
        (key, day),
        close_of(failures, counts.files > 0),
        &counts,
        failures,
    )
}

/// Whether `day` is imported this run: not already done (a day done with
/// refusals is read again under [`RunOptions::retry_refused`]), a regular
/// session (a closed day is counted, any other calendar answer is a named
/// failure and a `refused` line), and named as resumed when the journal
/// sends it back.
///
/// **ONE ORDERED TABLE (IA-16; D-4885, D-4910, design §5.5).** Rows 2 to 4b
/// are [`admit_rule`]'s, a pure function of the key's journal state and the
/// run's asks, so the plan and the run count the same days; row 1 (a wider
/// scope's member keys, WP-12f) is [`widen`]'s, asked before this; row 5 is
/// the calendar gate below; row 6 reads. A hole day (rows 4a, 4b) is named
/// in [`Report::holes_in_range`] and counted, never read and never written.
fn admit(ctx: &Ctx<'_>, key: &str, day: Day, report: &mut Report) -> bool {
    let retried = ctx.options.retry_refused && ctx.journal.with_refusals.contains(key);
    match admit_rule(&KeyState::of(ctx.journal, key), Asks::of(&ctx.options)) {
        Admit::Read => {}
        Admit::Skip(Skip::Done) => {
            report.days_skipped += 1;
            return false;
        }
        Admit::Skip(Skip::Restated) => {
            report.restated_skipped += 1;
            return false;
        }
        Admit::Skip(skip @ (Skip::Hole(reason) | Skip::Unrepairable(reason))) => {
            if matches!(skip, Skip::Hole(_)) {
                report.holes_skipped += 1;
            } else {
                report.holes_unrepairable += 1;
            }
            report.holes_in_range.push((day, reason.to_owned()));
            note_hole_skipped(ctx, (key, day), skip);
            return false;
        }
    }
    match session_gate(day) {
        Ok(()) => {}
        Err(CmRefusal::CalendarNotRegular {
            kind: crate::calendar::DayKind::Closed,
        }) => {
            report.days_closed += 1;
            return false;
        }
        Err(why) => {
            report.days_refused += 1;
            let cause = cm_cause(&why);
            let mut failures = vec![Staged {
                cause,
                failure: note_failure(ctx.run.kind, day, "calendar", &why.to_string(), cause),
            }];
            let counts = Counts {
                failures: 1,
                ..Counts::default()
            };
            let _refused = ctx.close((key, day), Close::Refused(cause), &counts, &mut failures);
            name_staged(report, failures);
            return false;
        }
    }
    if ctx.restated(key, day) {
        report.days_refused += 1;
        let _refused = refuse_restated(ctx, (key, day), report);
        return false;
    }
    name_retried(ctx.journal, ctx.run.kind, (key, day, retried), report);
    true
}

/// What [`admit_rule`] answers for one key (D-4885, D-4910): go on to the
/// calendar gate and read, or skip it for a named class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Admit {
    /// Rows 2 to 4b do not skip it: the calendar gate (row 5) decides, then
    /// row 6 reads it.
    Read,
    /// Skipped unread, for this class.
    Skip(Skip),
}

/// Why [`admit_rule`] skipped a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Skip {
    /// Row 2: `done` under a current definition.
    Done,
    /// Row 3: `done` under a definition no longer current for the kind, in
    /// a run that asked not to restate ([`RunOptions::skip_restated`]).
    Restated,
    /// Row 4a: a store hole a repair can fill, in a run with no repair
    /// reason. Its reason.
    Hole(&'static str),
    /// Row 4b: a store hole no repair fills (`bars_differ_held`,
    /// `repair_impossible`), with or without a repair reason (D-4912: only
    /// IA-18's restate run reads one). Its reason.
    Unrepairable(&'static str),
}

/// What [`admit_rule`] reads of one key: a probe of each journal set its
/// rows need, so a key costs O(1) in the journal's size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct KeyState {
    /// Closed `done` under a current definition.
    pub(super) done: bool,
    /// ... with `vendor_refused=` above zero.
    pub(super) with_refusals: bool,
    /// Its LAST close is a `done` (or `recounted`) naming a definition no
    /// longer current for its kind: a later `incomplete` or `refused` line
    /// is the key's last word instead (D-3317, FM-095).
    pub(super) restated: bool,
    /// Its LAST line is `incomplete` with one of the five store-hole
    /// reasons ([`HOLE_REASONS`]): that reason.
    pub(super) hole: Option<&'static str>,
}

impl KeyState {
    /// `key`'s state in `journal`: five hash probes. A key whose last line is
    /// `incomplete` is in `incomplete` and in neither `open` (a later
    /// `begin`) nor `refused_close` (a later `refused`); the reason that line
    /// carried is [`Journal::hole_reason`]'s.
    pub(super) fn of(journal: &Journal, key: &str) -> Self {
        let last_incomplete = journal.incomplete.contains(key)
            && !journal.open.contains(key)
            && !journal.refused_close.contains_key(key);
        let hole = if last_incomplete {
            journal.hole_reason.get(key).and_then(|reason| {
                HOLE_REASONS
                    .iter()
                    .copied()
                    .find(|one| *one == reason.as_str())
            })
        } else {
            None
        };
        Self {
            done: journal.done.contains(key),
            with_refusals: journal.with_refusals.contains(key),
            // `restated` keeps the key's last `done` (D-2822, which names the
            // day restated when it is read again); a later `incomplete` or
            // `refused` puts the key in `incomplete`, and that line, not the
            // older `done`, is what row 3 or 4 judges.
            restated: journal.restated.contains(key) && !journal.incomplete.contains(key),
            hole,
        }
    }
}

/// THE STORE'S STANDING HOLES (IA-16, D-6900 keep item 2): every key of
/// `journal` whose last line is `incomplete` with one of the five hole
/// reasons ([`KeyState::of`]'s answer, so the receipt and `admit` can never
/// disagree), counted by `(kind, reason)` in that order. One pass over the
/// keys that ever closed `incomplete` (the map the load already kept), once
/// per run: O(those keys), never per day or per file (L-IA16-09).
pub(super) fn standing_holes(journal: &Journal) -> Vec<(String, String, usize)> {
    let mut counted: std::collections::BTreeMap<(&str, &'static str), usize> =
        std::collections::BTreeMap::new();
    for key in journal.hole_reason.keys() {
        if let Some(reason) = KeyState::of(journal, key).hole {
            let kind = key.split(' ').next().unwrap_or_default();
            *counted.entry((kind, reason)).or_default() += 1;
        }
    }
    counted
        .into_iter()
        .map(|((kind, reason), keys)| (kind.to_owned(), reason.to_owned(), keys))
        .collect()
}

/// What a run asks that [`admit_rule`] reads (D-4885).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Asks {
    /// `--retry-refused` (D-3317 item 2): a day done with refusals is read
    /// again. Never admits a hole.
    pub(super) retry_refused: bool,
    /// Row 3's skip ([`RunOptions::skip_restated`]).
    pub(super) skip_restated: bool,
    /// The run carries a repair reason (D-4874): row 4a reads.
    pub(super) repair_reason: bool,
}

impl Asks {
    /// What `options` asks.
    pub(super) const fn of(options: &RunOptions<'_>) -> Self {
        Self {
            retry_refused: options.retry_refused,
            skip_restated: options.skip_restated,
            repair_reason: options.repair_reason.is_some(),
        }
    }
}

/// `admit`'s rows 2 to 4b (design §5.5, D-4885, D-4912), the first that
/// matches deciding: a pure function, so the plan and the run that call it
/// count the same days (IA-GR-19, IA-GR-37). O(1).
///
/// | # | the key's state | without a repair reason | with one |
/// |---|---|---|---|
/// | 2 | `done`, current definition | skip (`retry_refused` reads one done with refusals) | the same |
/// | 4a | last line `incomplete`, a repairable hole | skip, `holes_skipped` | read |
/// | 4b | last line `incomplete`, a hole no repair fills | skip, `holes_unrepairable` | skip |
/// | 3 | last close `done`, a definition no longer current | read, or skip when `skip_restated` | the same |
/// | — | anything else | read (the calendar gate, row 5, first) | read |
///
/// **A HOLE IS ASKED BEFORE ROW 3 (IA-16 repair 1; FM-095, IA-GR-30,
/// D-4873).** A hole is the key's LAST line (D-3317), and a restated day is
/// exactly the day a re-read can close as one: done under definition 2,
/// read again under the current one, and refused `bars_differ_held` because
/// a held bar is never changed. Asked first, row 3 read that hole again in
/// every later run, a repair-reason run included, which D-4912 forbids
/// (only IA-18's restate run reads one). [`KeyState::of`] no longer calls a
/// key restated whose last close is not a `done` either, so the two rows
/// cannot both match a key a journal describes; the order is kept for the
/// state a caller builds by hand (`admit_is_one_ordered_table_and_a_hole_is_read_only_when_asked`).
pub(super) fn admit_rule(state: &KeyState, asks: Asks) -> Admit {
    if state.done {
        return if asks.retry_refused && state.with_refusals {
            Admit::Read
        } else {
            Admit::Skip(Skip::Done)
        };
    }
    match state.hole {
        Some(reason) if is_repairable_hole(reason) => {
            return if asks.repair_reason {
                Admit::Read
            } else {
                Admit::Skip(Skip::Hole(reason))
            };
        }
        Some(reason) => return Admit::Skip(Skip::Unrepairable(reason)),
        None => {}
    }
    if state.restated && asks.skip_restated {
        return Admit::Skip(Skip::Restated);
    }
    Admit::Read
}

/// Whether `reason` is one of [`REPAIRABLE_HOLES`] (row 4a).
fn is_repairable_hole(reason: &str) -> bool {
    REPAIRABLE_HOLES.contains(&reason)
}

/// Says a hole day was passed over unread (D-4851): at `warn`, with its
/// reason and whether a repair reason could fill it, so a search for the
/// day finds why no run reads it.
fn note_hole_skipped(ctx: &Ctx<'_>, (key, day): (&str, Day), skip: Skip) {
    let (reason, repairable) = match skip {
        Skip::Hole(reason) => (reason, true),
        Skip::Unrepairable(reason) => (reason, false),
        Skip::Done | Skip::Restated => return,
    };
    let day_text = day.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(
            TARGET,
            "a store hole day is passed over: its month already passed it, so no append \
             lands it",
        )
        .with("kind", ctx.run.kind.as_str())
        .with("day", day_text.as_str())
        .with("key", key)
        .with("reason", reason)
        .with("repairable", repairable)
        .with("code", "hole_skipped"),
    );
}

/// Closes `key` `refused` with [`RESTATED_AUCTION_BAND`], named in the
/// report's failures: a key that would read a share whose months may hold a
/// bar definition 4 no longer builds ([`restated_past_append`]). Nothing of
/// the day is read.
fn refuse_restated(ctx: &Ctx<'_>, (key, day): (&str, Day), report: &mut Report) -> Outcome {
    let cause = Cause::open(Stage::File, RESTATED_AUCTION_BAND);
    let why = "restated by bar definition 4 (the closing auction's band through matching, \
               D-4878 item 10): a key of this day that may have built a share it reads under an \
               earlier definition is not closed done under 4 by a reading of those shares; the \
               month is made again as a new generation (D-3320 item 4, WP-80); an append cannot \
               land it, since a reading that builds fewer bars than the month holds is answered \
               AlreadyPresent";
    let mut failures = vec![Staged {
        cause,
        failure: note_failure(ctx.run.kind, day, "store", why, cause),
    }];
    let counts = Counts {
        failures: 1,
        ..Counts::default()
    };
    let outcome = ctx.close((key, day), Close::Refused(cause), &counts, &mut failures);
    name_staged(report, failures);
    outcome
}

/// The `reason=` of a share's day of the closing auction's era that
/// definition 4 restated (D-4878 item 10).
const RESTATED_AUCTION_BAND: &str = "restated_auction_band";

/// Whether a key (`<kind> <day> <filter>`, or a member key
/// `<kind> <day> <filter>+<member>`) would read a share, on a share's day of
/// the closing auction's era, whose months may hold a bar definition 4 no
/// longer builds (D-4878 item 10, CMP-07): a share some key of that day,
/// UNDER ANY FILTER, closed `done` or `recounted` under 2 or 3, closed
/// `incomplete` under one, or began under a run of one and never closed, and
/// no later close under 4 that read it has ended: a names or member close
/// naming it, or a whole-tree close whose scope `held` it (a whole-tree
/// close reads its scope's members and no others). Such a key is
/// refused by name, every run, and never read through the append door:
/// definition 4 takes the auction's print OUT of the bars, so the second
/// reading is often the held day less its last bar, which the append-only
/// month answers `AlreadyPresent` (it compares the bars offered, never the
/// held bar after them), and the key would close `done` under 4 with the
/// print still a bar, unsaid. Asking only the key itself and the day's
/// whole-tree key let a run under one filter read a share another filter's
/// key had built under 3 (check 4 of WP-12f, 14 Oct 2026). The journal keeps
/// a key's entry through its later `refused` lines, so the refusal holds
/// without another record.
///
/// O(1) in the journal's size: one hash probe for the day, then one probe a
/// share the key names (`*` names none; a member key names one), the
/// asking filter's own few, and for a share only whole-tree closes ended one
/// probe of `held` a scope of the day's keys naming it (the day's own few
/// lines), never a walk of the journal. Timed at two
/// journal sizes a hundredfold apart by
/// `pull::gdfl_import::tests::scope_reopen::the_auction_era_admission_check_costs_the_same_at_any_journal_size`.
pub(super) fn restated_past_append(
    journal: &Journal,
    held: ScopeHeld<'_>,
    kind: ImportKind,
    key: &str,
    day: Day,
) -> bool {
    if kind != ImportKind::Stocks || !crate::vendor::cash_auction_eligibility_required(day) {
        return false;
    }
    let Some((of_day, filter)) = key.rsplit_once(' ') else {
        return false;
    };
    journal
        .auction_restated
        .get(of_day)
        .is_some_and(|restated| restated.meets(filter, held))
}

/// Names `day` in `resumed` when the journal sends it back, whatever closed
/// it (D-2821), or when it is a day done with refusals read again (D-3317),
/// and in `restated` too when its last `done` was under another bar
/// definition (D-2822), with a warning either way.
fn name_retried(
    journal: &Journal,
    kind: ImportKind,
    (key, day, retried): (&str, Day, bool),
    report: &mut Report,
) {
    let restated = journal.restated.contains(key);
    if !(retried || restated || journal.open.contains(key) || journal.incomplete.contains(key)) {
        return;
    }
    report.resumed.push(day);
    if restated {
        report.restated.push(day);
    }
    let day_text = day.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(
            TARGET,
            "importing again a day an earlier run did not finish under this bar definition",
        )
        .with("kind", kind.as_str())
        .with("day", day_text.as_str())
        .with("restated", restated)
        .with("retry_refused", retried)
        .with("definition", BAR_DEFINITION),
    );
}

/// The one runtime: every day of the run through the calendar, the venue,
/// the journal, the source's `read_day` and the common filing path. Its
/// `run` line records the provenance as `unrecorded`; [`drive_from`] is the
/// door for a caller that measured its [`Origin`].
///
/// # Errors
///
/// [`ImportRefusal::RangeBackwards`], [`ImportRefusal::FilterName`] and
/// [`ImportRefusal::Journal`] for the run, and [`ImportRefusal::CensusFull`]
/// or [`ImportRefusal::CensusRefused`] at the first day the census refuses,
/// where the run stops (D-3317); everything else is a named failure in the
/// report.
pub fn drive<F>(run: &Run<'_>, read_day: F) -> Result<Report, ImportRefusal>
where
    F: FnMut(Day, &mut dyn FnMut(TickFile)) -> DayRead,
{
    // The run's census memory, kept across its days (R3-19).
    let mut census = StoreCensus::default();
    drive_with(run, RunOptions::default(), &mut census, read_day)
}

/// [`drive`] with the run's [`RunOptions`]: the origin the caller measured,
/// written as the run's `run` line and in "run started" (D-3317; DB-G56), and
/// whether to read again the days done with refusals.
///
/// # Errors
///
/// As [`drive`].
pub fn drive_from<F>(
    run: &Run<'_>,
    options: RunOptions<'_>,
    read_day: F,
) -> Result<Report, ImportRefusal>
where
    F: FnMut(Day, &mut dyn FnMut(TickFile)) -> DayRead,
{
    drive_with(run, options, &mut StoreCensus::default(), read_day)
}

/// "run started", with everything the caller said about where the data came
/// from, or `source=unrecorded` when it said nothing (D-3317; DB-G56). An
/// origin that could not be measured is logged at `warn`, naming why.
fn note_started(ctx: &Ctx<'_>, filter: &str) {
    let run = ctx.run;
    let (from_text, to_text) = (run.from.to_string(), run.to.to_string());
    let origin = ctx.options.origin;
    let (path, digest) = origin.map_or_else(Default::default, |origin| {
        (origin.path.display().to_string(), origin.digest_hex())
    });
    let event = started_event(
        (run.kind, from_text.as_str(), to_text.as_str(), filter),
        (ctx.id, ctx.options.retry_refused),
        origin.map(|origin| (origin, path.as_str(), digest.as_str())),
    );
    let _dropped_when_filtered = telemetry::emit(&event);
    let _dropped_when_filtered = telemetry::emit(&settings_event(ctx.id, &ctx.options));
}

/// "run settings" (WP-16, D-3353): the workers and the durability unit the
/// run was started with, beside its id, so the audit trail can say which
/// durability a day left open ran under without inferring it from the "day
/// made durable" counts (an `each-write` run leaves none of those). Its own
/// event because "run started" is already [`telemetry::MAX_FIELDS`] wide,
/// and not on the journal's `run` line because neither setting changes a
/// byte the run leaves, the journal included.
pub(super) fn settings_event(id: u64, options: &RunOptions<'_>) -> telemetry::Event<'static> {
    telemetry::Event::info(TARGET, "run settings")
        .with("run", id)
        .with("workers", options.workers.get())
        .with("durability", options.durability.as_str())
}

/// "run started" (D-3317 item 4; DB-G56): what the run reads and where its
/// data came from, in at most [`telemetry::MAX_FIELDS`] fields so a fault is
/// never the field the sink counts instead of keeping (the bar definition is
/// on every closing line, not here). `warn` when the origin names a fault.
pub(super) fn started_event<'a>(
    (kind, from, to, filter): (ImportKind, &'a str, &'a str, &'a str),
    (id, retry_refused): (u64, bool),
    origin: Option<(&'a Origin, &'a str, &'a str)>,
) -> telemetry::Event<'a> {
    let level = if origin.is_some_and(|(origin, ..)| origin.fault.is_some()) {
        telemetry::Event::warn(TARGET, "run started")
    } else {
        telemetry::Event::info(TARGET, "run started")
    };
    let event = level
        .with("kind", kind.as_str())
        .with("from", from)
        .with("to", to)
        .with("only", filter)
        .with("run", id)
        .with("retry_refused", retry_refused);
    let Some((origin, path, digest)) = origin else {
        return event.with("source", "unrecorded");
    };
    let event = event
        .with("source", origin.source.as_str())
        .with("archive", path)
        .with("archive_bytes", origin.archive_bytes)
        .with(origin.source.digest_field(), digest)
        .with("commit", origin.commit.as_deref().unwrap_or("unstamped"));
    match &origin.fault {
        Some(fault) => event.with("archive_fault", fault.as_str()),
        None => event,
    }
}

/// Whose close a summary event tells (D-3317, D-4878).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Closed<'c> {
    /// A day read whole, under its base key.
    Day,
    /// One member key of a day read again for a wider scope: the member and
    /// the journal key its closing line is written under.
    Member { member: &'c str, key: &'c str },
    /// A day read again for a wider scope, once, after every member key the
    /// run opened on it is closed: how many those were.
    Widened { members: usize },
}

/// The words of a summary event, by whose close it is and how it closed. A
/// member key's close is never "day imported": that message is the DAY's,
/// once a day, and the import's pace is measured from the gaps between its
/// stamps (`api`'s `/imports.json`), so one per member key made a day of six
/// members read as six days (WP-12f repair 1).
const fn summary_words(closed: Closed<'_>, outcome: Outcome) -> &'static str {
    match (closed, outcome) {
        (Closed::Member { .. }, Outcome::Done | Outcome::DoneWithRefusals) => "member imported",
        (Closed::Member { .. }, Outcome::Incomplete) => "member incomplete",
        (Closed::Member { .. }, Outcome::Refused) => "member refused",
        (_, Outcome::Done | Outcome::DoneWithRefusals) => "day imported",
        (_, Outcome::Incomplete) => "day incomplete",
        (_, Outcome::Refused) => "day refused",
    }
}

/// One close's summary event (D-3317; D-4878 for a member key and a widened
/// day). For a DAY, read whole or read again for a wider scope: "day
/// imported" at `info` for a day closed `done`, with or without refusals;
/// "day incomplete" or "day refused" at `warn` otherwise; a widened day says
/// `members=`, how many member keys the run closed on it. For a MEMBER KEY:
/// "member imported", "member incomplete" or "member refused", each naming
/// its `member=` and its journal `key=`, so a member's close is found in the
/// event log by its name. A census-refused day never reaches it.
fn note_day(
    kind: ImportKind,
    day: Day,
    (outcome, closed): (Outcome, Closed<'_>),
    (before, report): (Mark, &Report),
    census_ms: Option<u64>,
) {
    let day_text = day.to_string();
    let words = summary_words(closed, outcome);
    let event = match outcome {
        Outcome::Done | Outcome::DoneWithRefusals => telemetry::Event::info(TARGET, words),
        Outcome::Incomplete | Outcome::Refused => telemetry::Event::warn(TARGET, words),
    };
    let event = event
        .with("kind", kind.as_str())
        .with("day", day_text.as_str())
        .with("outcome", outcome.as_str())
        .with("files", report.files - before.files)
        .with("rows", report.rows - before.rows)
        .with("seconds", report.seconds - before.seconds)
        .with(
            "committed",
            report.seconds_committed - before.seconds_committed,
        )
        .with("failures", report.failures_total - before.failures);
    // The day's census publish, in milliseconds (R3-19): absent on a close
    // that published nothing of its own (a day read again for a wider scope).
    let event = match census_ms {
        Some(ms) => event.with("census_ms", ms),
        None => event,
    };
    let event = match closed {
        Closed::Day => event,
        Closed::Member { member, key } => event.with("member", member).with("key", key),
        Closed::Widened { members } => event.with("members", members),
    };
    #[cfg(test)]
    tests::summary_emitted(&event);
    let _dropped_when_filtered = telemetry::emit(&event);
}

/// The worse of two closes of one day's keys, `done` best and `refused`
/// worst: how a day read again for a wider scope closed as a whole.
const fn worse(a: Outcome, b: Outcome) -> Outcome {
    const fn rank(outcome: Outcome) -> u8 {
        match outcome {
            Outcome::Done => 0,
            Outcome::DoneWithRefusals => 1,
            Outcome::Incomplete => 2,
            Outcome::Refused => 3,
        }
    }
    if rank(b) > rank(a) { b } else { a }
}

/// A trading day the source does not hold: counted and named in
/// `days_missing`, warned, and closed `refused ... stage=vendor
/// reason=day_not_held failures=0` (D-3317 item 3, DB-025), so the hole has
/// a reason in the journal. It names no failure of its own.
fn close_missing(
    ctx: &Ctx<'_>,
    (key, day): (&str, Day),
    report: &mut Report,
    mut failures: Vec<Staged>,
) {
    report.days_missing.push(day);
    let day_text = day.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(TARGET, "the source holds no file of this trading day")
            .with("kind", ctx.run.kind.as_str())
            .with("day", day_text.as_str())
            .with("code", ImportRefusal::DayNotHeld { day }.code()),
    );
    let not_held = Cause::open(Stage::Vendor, ImportRefusal::DayNotHeld { day }.code());
    let cause = decisive(&failures).map_or(not_held, |one| one.cause);
    let counts = Counts {
        failures: failures.len(),
        ..Counts::default()
    };
    let _refused = ctx.close((key, day), Close::Refused(cause), &counts, &mut failures);
    name_staged(report, failures);
}

/// The UNIX second now; 0 for a clock set before 1970, which no run line can
/// then be mistaken for a real start by.
pub(super) fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The runtime behind [`drive`] and [`drive_from`], with the census it
/// publishes through handed in (a test hands a double; [`Census`]), for a
/// source that hands on bare [`TickFile`]s: no member, so no provenance
/// record, and no reader counts. The day's files are gathered as the source
/// hands them on, then filed on the run's workers in that order (WP-16).
pub(super) fn drive_with<F>(
    run: &Run<'_>,
    options: RunOptions<'_>,
    census: &mut dyn Census,
    mut read_day: F,
) -> Result<Report, ImportRefusal>
where
    F: FnMut(Day, &mut dyn FnMut(TickFile)) -> DayRead,
{
    drive_core(run, options, (census, None), |day, filer| {
        let mut files = Vec::new();
        let read = read_day(day, &mut |file| files.push(file));
        let handed: Vec<std::sync::Mutex<Option<Handed>>> = files
            .into_iter()
            .map(|file| {
                std::sync::Mutex::new(Some(Handed {
                    file,
                    facts: FileFacts::default(),
                }))
            })
            .collect();
        let month = day.year_month().ok();
        let key_of = |slot: &std::sync::Mutex<Option<Handed>>| {
            let held = slot
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            held.as_ref()
                .zip(month)
                .map(|(one, month)| (one.file.symbol.clone(), one.file.contract, month))
        };
        let _reads = filer.file_all(&handed, &key_of, &|slot| {
            // Each slot is one job, taken once, so it is always full here.
            slot.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
                .map(|one| (one, None))
                .ok_or_else(|| {
                    Refused::at(
                        Cause::open(Stage::File, "file_taken_twice"),
                        failure("file", "a handed file was taken twice".to_owned()),
                    )
                })
        });
        (read, DayExtras::default())
    })
}

/// The day's quality lines (D-3335), written before its close: a ledger that
/// will not take them is a failure of the day, which then closes
/// `incomplete` and is read again, so its lines are written by the next run.
fn write_flags(
    (run, whole): (&Run<'_>, bool),
    flags: &mut quality::Ledger,
    (day, ledger, (member_bytes, backslash)): (
        Day,
        Option<quality::DayLedger>,
        (Option<u64>, bool),
    ),
    report: &mut Report,
    failures: &mut Vec<Staged>,
) {
    let Some(mut ledger) = ledger else {
        return;
    };
    ledger.set_member_bytes(member_bytes);
    ledger.set_separator_backslash(backslash);
    note_sampled(
        ledger.kind(),
        ledger.day(),
        &ledger.sampled(),
        quality::Src::Import,
    );
    let fresh = ledger.lines(whole);
    let news = flags.news(fresh, quality::Src::Import);
    match flags.append(&news) {
        Ok(()) => report.flags_lines += news.len(),
        Err(why) => failures.push(note_staged(
            run,
            day,
            "day ledger",
            &ImportRefusal::DayLedger { why },
        )),
    }
}

/// The runtime: every day of the run through the calendar, the venue, the
/// journal, the source's `read_day` and the common filing path, and each
/// day's quality lines (D-3335) and its files' provenance (D-3334).
///
/// Days run in order. A day's files are handed to its [`DayFiler`], which
/// files them on the run's [`Workers`] and folds them back in listing order
/// (WP-16); every file of the day is written and the day's batch made durable
/// ([`make_durable`], D-3353) before the day's ledger lines, its one census
/// batch and its closing line, in that order.
pub(super) fn drive_core<F>(
    run: &Run<'_>,
    options: RunOptions<'_>,
    census_scoped: (&mut dyn Census, Option<(&scope::Record, &scope::Record)>),
    read_day: F,
) -> Result<Report, ImportRefusal>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    // THE RUN'S LAST WORD, WHATEVER ENDED IT (IA-1): a whole report is cut
    // to the failures it keeps by name (D-4839) before anyone reads it, and
    // the observer is told the end, refusal or report, exactly once.
    let mut end = drive_days(run, options, census_scoped, read_day);
    match &mut end {
        Ok(report) => report.seal(),
        Err(refusal) => refusal.seal(),
    }
    if let Some(observer) = options.observer {
        observer.finished(run.kind, end.as_ref());
    }
    end
}

/// The run's key in the process's event sink, held while the run's days are
/// read (the leftover DB-A41 of round 3): every event any thread of the run
/// writes in that time carries the run's journal id as its record's `run`,
/// so `/logs.json?run=<id>` answers the run's per-day and per-refusal lines,
/// not only its start and finish. One `compare_exchange` to take it and one
/// to give it back (`telemetry::Sink::claim_run`); a key another run of this
/// process holds is left alone and the run says so once, by name.
struct RunKey(Option<u64>);

impl RunKey {
    /// Takes the key for `id` when a sink is installed and no run holds it.
    fn claim(id: u64) -> Self {
        let Some(sink) = telemetry::global() else {
            return Self(None);
        };
        if sink.claim_run(id) {
            return Self(Some(id));
        }
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::warn(TARGET, "run key held by another run")
                .with("run", id)
                .with("holder", sink.run()),
        );
        Self(None)
    }
}

impl Drop for RunKey {
    fn drop(&mut self) {
        if let (Some(id), Some(sink)) = (self.0, telemetry::global()) {
            sink.release_run(id);
        }
    }
}

/// [`drive_core`]'s days, before the report is sealed and the end told.
fn drive_days<F>(
    run: &Run<'_>,
    options: RunOptions<'_>,
    (census, scoped): (&mut dyn Census, Option<(&scope::Record, &scope::Record)>),
    mut read_day: F,
) -> Result<Report, ImportRefusal>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    // ONE REQUEST FOR THE RUN: its window is the run's range, which is also
    // the range check, its rung the second, its listing the kind's venue. A
    // file, not a vendor request, is the source, so it carries no vendor id.
    // Every venue table holds verified hours from the epoch (`crate::vendor`
    // const-asserts their shipping shape), so the session filter in
    // `from_rows` cannot meet a day with no hours.
    let window = Window::new(run.from, run.to).map_err(|_| ImportRefusal::RangeBackwards {
        from: run.from,
        to: run.to,
    })?;
    let request = BarRequest {
        instrument_id: String::new(),
        listing: run.kind.listing(),
        window,
        granularity: Granularity::Second1,
    };
    run.check_filter()?;
    fresh_root(run)?;
    let journal = Journal::load(journal_path(run.store_root))?;
    let mut flags = quality::Ledger::load(quality::dayflags_path(run.store_root))
        .map_err(|why| ImportRefusal::DayLedger { why })?;
    // THE SCOPE LEDGER (WP-12f, D-4878), read before any day: one that
    // cannot be read refuses the run, as the journal does.
    let scoped = match scoped {
        Some((record, swept)) => Some(Scoped {
            record,
            swept: std::sync::Arc::clone(swept.set()),
            ledger: std::cell::RefCell::new(
                scope::Ledger::load(scope::scopes_path(run.store_root))
                    .map_err(|why| ImportRefusal::Journal { why })?,
            ),
            beyond: std::cell::RefCell::new(HashMap::with_capacity(SCOPES_HINT)),
            shares: std::cell::RefCell::new(HashMap::with_capacity(SCOPES_HINT)),
        }),
        None => None,
    };
    let filter = run.filter_key();
    let started = options
        .origin
        .map_or_else(now_unix_s, |origin| origin.started);
    let ctx = Ctx {
        run,
        journal: &journal,
        options,
        id: options.run.unwrap_or_else(|| journal.fresh_run(started)),
        started,
        announced: std::cell::Cell::new(false),
        scoped,
        held_back: std::cell::RefCell::new(HashMap::new()),
        store_id: std::cell::Cell::new(super::filing::store_identity(run.store_root)),
        gather: std::cell::RefCell::new(super::filing::Gather::default()),
    };
    let _key = RunKey::claim(ctx.id);
    if let Some(observer) = options.observer {
        observer.begun(run.kind, ctx.id);
    }
    note_started(&ctx, &filter);
    note_torn(&ctx);
    let mut report = Report {
        standing_holes: standing_holes(&journal),
        ..Report::default()
    };
    let ended = run_days(
        &ctx,
        (&filter, &request),
        (&mut report, &mut flags),
        census,
        &mut read_day,
    );
    let ended = ended.map(|()| report);
    end_line(&ctx, &ended);
    ended
}

/// THE RUN'S END IN THE JOURNAL (IA-2; DB-A50): a run the runs ledger knows
/// that wrote any line writes one more `run` line when it ends, whatever
/// ended it: `run <id> ended=<unix_s> outcome=<finished|stopped|code>
/// definition=<n>`. No new verb (design §5.4): a `run` line with more
/// `name=value` fields is read by every reader since 3f65ad43, and its
/// `definition=` keeps the reader's run definition what the run's own line
/// set. A run that wrote no line writes none (D-2803); a run with no ledger
/// id (a test's bare reader) writes none, so its journal stays a pure
/// function of its inputs. A journal that will not take the line is said at
/// `error`; the run's end is not changed by it.
fn end_line(ctx: &Ctx<'_>, ended: &Result<Report, ImportRefusal>) {
    if ctx.options.run.is_none() || !ctx.announced.get() {
        return;
    }
    let outcome = match ended {
        Ok(_) => "finished",
        Err(refusal) => refusal.code(),
    };
    // A STORE WHOSE ROOT WENT AWAY IS NOT WRITTEN TO AGAIN (D-4852, FM-125):
    // the journal's append makes its directory, which would make a phantom
    // root on whatever disk now holds the mount point.
    if let Some(gone) = super::filing::store_root_lost(ctx.run.store_root, ctx.store_id.get()) {
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::error(TARGET, "run end line not written")
                .with("run", ctx.id)
                .with("outcome", outcome)
                .with("why", gone.as_str())
                .with("code", "store_lost"),
        );
        return;
    }
    let line = format!(
        "run {} ended={} outcome={outcome} definition={BAR_DEFINITION}",
        ctx.id,
        now_unix_s()
    );
    if let Err(why) = ctx.journal.append(&line) {
        let why = why.to_string();
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::error(TARGET, "run end line not written")
                .with("run", ctx.id)
                .with("outcome", outcome)
                .with("why", why.as_str()),
        );
    }
}

/// The run's stop, once `stop` was asked: the refusal that ends the run,
/// with who asked and when, and its "run stopped" event (D-4893).
fn stopped(
    ctx: &Ctx<'_>,
    stop: &crate::gdfl_run::stop::StopControl,
    day: Option<Day>,
    report: &mut Report,
) -> ImportRefusal {
    let shown = day.map(|day| day.to_string());
    stop.note_stopped(ctx.run.kind.as_str(), shown.as_deref());
    let request = stop.request();
    ImportRefusal::Stopped {
        kind: ctx.run.kind,
        day,
        by: request
            .as_ref()
            .map_or_else(|| "-".to_owned(), |r| r.by.clone()),
        requested: request.map_or(0, |r| r.requested),
        done: Box::new(std::mem::take(report)),
    }
}

/// [`drive_days`]' days once its context is built: the reach back, then each
/// day of the range, the stop asked before each day and after it (D-4853:
/// a stop that came after a day's last file lets the day close as it would
/// have, and the run stops before the next).
fn run_days<F>(
    ctx: &Ctx<'_>,
    (filter, request): (&str, &BarRequest),
    (report, flags): (&mut Report, &mut quality::Ledger),
    census: &mut dyn Census,
    read_day: &mut F,
) -> Result<(), ImportRefusal>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    let (run, options) = (ctx.run, ctx.options);
    if let Some(refusal) = stop_after_gather(ctx, None, report, &mut *census)? {
        return Err(refusal);
    }
    // THE FLOOR BEFORE THE FIRST DAY (IA-16, CONC-031): a full disk is named
    // before the run writes anything, the reach-back included.
    floor_check(ctx, run.from, report)?;
    let state = (&mut *report, &mut *flags);
    reach_back(ctx, filter, state, &mut *census, &mut *read_day)?;
    let range = days(run.from, run.to);
    let total = range.len();
    for (index, day) in range.into_iter().enumerate() {
        if let Some(refusal) = stop_after_gather(ctx, None, report, &mut *census)? {
            return Err(refusal);
        }
        root_check(ctx, day, report)?;
        if let Some(observer) = options.observer {
            observer.day_begun(run.kind, day, (index, total));
        }
        'day: {
            let key = format!("{} {day} {filter}", run.kind.as_str());
            let state = (&mut *report, &mut *flags);
            let asked = Asked {
                key: &key,
                day,
                remembered: &key,
                names: None,
                early: false,
            };
            if widen(ctx, request, &asked, state, &mut *census, &mut *read_day)? {
                break 'day;
            }
            if !admit(ctx, &key, day, report) {
                break 'day;
            }
            // AND BEFORE EACH DAY IT READS (IA-16, CONC-031): never a day begun
            // that the volume cannot take.
            floor_check(ctx, day, report)?;
            let pass = Pass {
                key: &key,
                day,
                member: None,
            };
            let state = (&mut *report, &mut *flags);
            day_pass(ctx, request, &pass, state, &mut *census, &mut *read_day)?;
        }
        if let Some(observer) = options.observer {
            observer.day_passed(run.kind, day, report);
        }
        if let Some(refusal) = stop_after_gather(ctx, Some(day), report, &mut *census)? {
            return Err(refusal);
        }
    }
    // THE RUN ENDS: what it gathered is repaired, once per (instrument,
    // month), and its days closed (IA-17b, D-4899).
    flush_gather(ctx, report, census)?;
    note_finished(ctx, report);
    Ok(())
}

/// The run's stop, when one was asked ([`stopped`]), AFTER THE GATHER
/// (IA-17b, D-4899 and the critic's IA-17-3): a stop offers what is
/// gathered and closes those days before the run ends, so a stop never
/// leaves a gathered day begun. The stop is asked once; nothing is done
/// when none was asked.
fn stop_after_gather(
    ctx: &Ctx<'_>,
    day: Option<Day>,
    report: &mut Report,
    census: &mut dyn Census,
) -> Result<Option<ImportRefusal>, ImportRefusal> {
    let Some(stop) = ctx.options.stop.filter(|stop| stop.requested()) else {
        return Ok(None);
    };
    flush_gather(ctx, report, census)?;
    Ok(Some(stopped(ctx, stop, day, report)))
}

/// A FRESH STORE IS MADE ONCE, BY THE RUN, BEFORE ANYTHING IS READ OR
/// WRITTEN (IA-16 repair 1; FM-125, D-4852): its root alone, under a parent
/// that is there, never an ancestor, so a store on a volume that is not
/// mounted is refused `store_lost` rather than born on the boot disk. Every
/// writer below the root then refuses a root that is not there
/// ([`super::make_leaf_dir`]) instead of making one, so a root that goes away
/// mid-run is never made again and the run's identity check
/// ([`super::filing::store_root_lost`]) has the root it began on to compare.
/// The `cli` verb refuses a missing root before this (its lock); this is the
/// library's own door. One `metadata` call; one `mkdir` for a fresh store.
fn fresh_root(run: &Run<'_>) -> Result<(), ImportRefusal> {
    let root = run.store_root;
    // Only a path where NOTHING is is a fresh store. A directory is the
    // store; anything else there (a file, an unreadable entry) is left to
    // the journal's load, which refuses it by name as it always has.
    match std::fs::symlink_metadata(root) {
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
        _ => return Ok(()),
    }
    let parent_there = root
        .parent()
        .is_some_and(|parent| parent.as_os_str().is_empty() || parent.is_dir());
    let made = if parent_there {
        match std::fs::create_dir(root) {
            Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists && root.is_dir() => Ok(()),
            made => made.map_err(|why| why.to_string()),
        }
    } else {
        Err(
            "its parent is not a directory, and a run never makes an ancestor of a store root \
             (the store's volume is not mounted)"
                .to_owned(),
        )
    };
    made.map_err(|why| ImportRefusal::StoreLost {
        kind: run.kind,
        day: run.from,
        why: format!(
            "{} is not a store root a run can begin: {why}",
            root.display()
        ),
        done: Box::default(),
    })
}

/// THE STORE ROOT BEFORE EACH DAY (IA-16; RA-284, D-4852, FM-125): one
/// `metadata` call per calendar day of the range, before anything of the day
/// is asked or written. A root that went away, or that is another directory
/// than the run began on (a volume unplugged and mounted back, or an empty
/// folder left at its mount point), ends the run `store_lost` before the day
/// is begun, so no line and no bar of it lands in a folder that is not the
/// store and no writer recreates a root that is gone. A run that began
/// before its root existed (a fresh store) learns the root's identity here,
/// once a day's lines have made it.
fn root_check(ctx: &Ctx<'_>, day: Day, report: &mut Report) -> Result<(), ImportRefusal> {
    let root = ctx.run.store_root;
    let Some(at_start) = ctx.store_id.get() else {
        ctx.store_id.set(super::filing::store_identity(root));
        return Ok(());
    };
    match super::filing::store_root_lost(root, Some(at_start)) {
        None => Ok(()),
        Some(why) => Err(ImportRefusal::StoreLost {
            kind: ctx.run.kind,
            day,
            why: format!("{why}; {day} was not begun"),
            done: Box::new(std::mem::take(report)),
        }),
    }
}

/// THE FREE-SPACE FLOOR (IA-16; FM-085, CONC-031, D-6900 keep item 5): the
/// store volume's free space, measured once ([`RunOptions::free_space`], the
/// host's `statfs(2)` by default), against [`RunOptions::free_floor`]. Below it the
/// run ends `disk_full` before `day` is begun, naming both figures; a meter
/// that cannot answer ends it `free_space_unknown`, never assumed to hold.
/// A floor of 0 measures nothing. One measurement per call: before the first
/// day and before each day read, never per file.
fn floor_check(ctx: &Ctx<'_>, day: Day, report: &mut Report) -> Result<(), ImportRefusal> {
    let floor = ctx.options.free_floor;
    if floor == 0 {
        return Ok(());
    }
    let measure = ctx.options.free_space.unwrap_or(super::volume_space);
    let kind = ctx.run.kind;
    match measure(ctx.run.store_root) {
        Ok(space) if space.free >= floor => Ok(()),
        Ok(space) => Err(ImportRefusal::DiskFull {
            kind,
            day,
            why: format!(
                "the store's volume has {} bytes free of {}, below the free-space floor of \
                 {floor} bytes (CONC-031): {day} was not begun",
                space.free, space.total
            ),
            done: Box::new(std::mem::take(report)),
        }),
        Err(why) => Err(ImportRefusal::FreeSpaceUnknown {
            kind,
            day,
            why,
            done: Box::new(std::mem::take(report)),
        }),
    }
}

/// The days of `from`'s month before it, in order: none when `from` is the
/// month's first day. At most 30.
pub(super) fn before_range(from: Day) -> Vec<Day> {
    let first = from.days_from_epoch() - u32::from(from.day().saturating_sub(1));
    (first..from.days_from_epoch())
        .filter_map(|n| Day::from_days(n).ok())
        .collect()
}

/// One key that closed a day `done`, asked what it owes this run (WP-12f,
/// D-4878).
#[derive(Clone, Copy)]
pub(super) struct Asked<'k> {
    /// The base key whose close is asked: `<kind> <day> <filter>`.
    pub(super) key: &'k str,
    pub(super) day: Day,
    /// The key the scope ledger remembers the examination under: `key` when
    /// it is the run's own, `<key>/<the run's filter>` when another filter
    /// closed it. `/` is in no symbol, so in no filter and no member.
    pub(super) remembered: &'k str,
    /// The names of the filter that closed `key`, when it is not the run's
    /// own and names any: `*` names none, which is every one.
    pub(super) names: Option<&'k Names>,
    /// Whether the day lies before the run's range ([`reach_back`]).
    pub(super) early: bool,
}

impl Asked<'_> {
    /// Whether the key is the run's own.
    fn own(&self) -> bool {
        self.key == self.remembered
    }
}

/// The names a journal key's filter part lists: none for `*`, which names
/// every one.
fn names_of(filter: &str) -> Option<Names> {
    (filter != "*").then(|| std::sync::Arc::new(filter.split(',').map(str::to_owned).collect()))
}

/// THE RUN'S FIRST MONTH FROM ITS FIRST OWED DAY (D-4878, DB-B76): the days
/// of that month before the range that are closed `done` and owe this run a
/// member are read for it first, in order, so no member's month is made from
/// a later day while an earlier one is still owed. A day no run closed owes
/// nothing and is not read: the run was not asked for it.
///
/// WHOSE CLOSE IS ASKED (repair 2, 12 Oct 2026). A day the run's own key
/// closed is that key's. Any other day is asked of EVERY key that closed it
/// `done`, whatever its filter, the whole-day key `*` among them: such a key
/// owes the members its close's scope never covered that its own filter
/// names, and the run reads the ones its own filter wants, each under that
/// key's member key (`<its filter>+<member>`), so the key's own later run
/// finds them done. Asking only the run's own key let a run under a filter
/// (`cli`'s `--stored`, or one named symbol) make each new member's month
/// from one later day while the earlier days were closed done under `*` for
/// two indices: the 10 Oct shape, unsaid. That such a key was examined for
/// this run's filter is remembered under `<key>/<filter>` and never under
/// the key itself, so its own run still reads the members this filter did
/// not want.
///
/// THOSE DAYS HAVE A REQUEST OF THEIR OWN: a request's window is its range
/// check and a row outside it is dropped, so under the run's own request a
/// day before the range would file nothing and still close `done`.
///
/// At most 30 days: one hash probe for a day no run closed, and for a closed
/// one what [`Ctx::owed`] costs per key that closed it.
fn reach_back<F>(
    ctx: &Ctx<'_>,
    filter: &str,
    (report, flags): (&mut Report, &mut quality::Ledger),
    census: &mut dyn Census,
    read_day: &mut F,
) -> Result<(), ImportRefusal>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    let run = ctx.run;
    let early = before_range(run.from);
    let (Some(&first), Some(&last)) = (early.first(), early.last()) else {
        return Ok(());
    };
    if ctx.scoped.is_none() {
        return Ok(());
    }
    let reach = BarRequest {
        instrument_id: String::new(),
        listing: run.kind.listing(),
        window: Window::new(first, last).map_err(|_| ImportRefusal::RangeBackwards {
            from: first,
            to: last,
        })?,
        granularity: Granularity::Second1,
    };
    // Each other filter's names, split once a run.
    let mut lenders: HashMap<&str, Option<Names>> = HashMap::with_capacity(SCOPES_HINT);
    for day in early {
        let of_day = format!("{} {day}", run.kind.as_str());
        let Some(closed_by) = ctx.journal.closed_by.get(&of_day) else {
            continue;
        };
        let own = format!("{of_day} {filter}");
        if ctx.journal.done.contains(own.as_str()) {
            let asked = Asked {
                key: &own,
                day,
                remembered: &own,
                names: None,
                early: true,
            };
            let state = (&mut *report, &mut *flags);
            widen(ctx, &reach, &asked, state, &mut *census, read_day)?;
            continue;
        }
        // THE DAY IS COUNTED AND TOLD ONCE, after the last key that closed
        // it, and a member two of them owe is read once.
        let mut widened = Widened::default();
        for lender in closed_by {
            let key = format!("{of_day} {lender}");
            let remembered = format!("{key}/{filter}");
            let names = lenders
                .entry(lender.as_str())
                .or_insert_with(|| names_of(lender));
            let asked = Asked {
                key: &key,
                day,
                remembered: &remembered,
                names: names.as_ref(),
                early: true,
            };
            let state = (&mut *report, &mut *flags);
            widen_key(
                ctx,
                &reach,
                &asked,
                state,
                &mut *census,
                read_day,
                &mut widened,
            )?;
        }
        widened.tell(ctx, day, report);
    }
    Ok(())
}

/// Says, at `warn` and once a run, that the run reads days before its range:
/// `asked` is the earliest key of the range's first month that is closed
/// `done` and owes this run a member the day holds a file of (D-4878,
/// DB-B76). The run was not asked for those days, so it never reads them
/// unsaid; a day it only lists, and reads nothing of, is not one.
fn note_reach_back(ctx: &Ctx<'_>, asked: &Asked<'_>) {
    let (first, from) = (asked.day.to_string(), ctx.run.from.to_string());
    let event = telemetry::Event::warn(
        TARGET,
        "done days before the range owe this scope; they are read first",
    )
    .with("kind", ctx.run.kind.as_str())
    .with("run", ctx.id)
    .with("code", "scope_reach_back")
    .with("first_owed", first.as_str())
    .with("key", asked.key)
    .with("range_from", from.as_str());
    #[cfg(test)]
    tests::summary_emitted(&event);
    let _dropped_when_filtered = telemetry::emit(&event);
}

/// Says, at `warn` and once a run, that the scope ledger holds a torn line:
/// the fragment a killed write left, stepped over by the load and written
/// again whole by the run that needs it (D-4878).
fn note_torn(ctx: &Ctx<'_>) {
    let Some(scoped) = &ctx.scoped else {
        return;
    };
    let torn = scoped.ledger.borrow().torn();
    if torn > 0 {
        let event = telemetry::Event::warn(
            TARGET,
            "the scope ledger holds a torn line; it is stepped over",
        )
        .with("kind", ctx.run.kind.as_str())
        .with("run", ctx.id)
        .with("code", "scope_ledger_torn")
        .with("lines", torn);
        #[cfg(test)]
        tests::summary_emitted(&event);
        let _dropped_when_filtered = telemetry::emit(&event);
    }
}

/// A DAY DONE UNDER A NARROWER SCOPE IS NOT DONE FOR THE MEMBERS ITS CLOSE
/// NEVER COVERED (WP-12f, D-4878): each such member with a file that day is
/// read under a key of its own, `<filter>+<member>`, so its hole never takes
/// the day's other members out of `done`; the base key is not read again.
/// Answers whether the day was such a day, and so is not read whole.
///
/// The day is counted and told ONCE, after its last member key: one "day
/// imported" (or "day incomplete", "day refused": the worst of its member
/// keys' closes) and one more in `days_imported`, when the run closed at
/// least one member key on it. Each member key tells its own close
/// ([`Closed::Member`]).
///
/// A KEY ANOTHER FILTER CLOSED ([`Asked::own`] is false; [`reach_back`]) is
/// read the same way under its own member keys, for the members this run's
/// filter wants. Which those are is the day's listing's to say, also for the
/// member keys that key already has: another run opened them and this run's
/// filter may want none.
fn widen<F>(
    ctx: &Ctx<'_>,
    request: &BarRequest,
    asked: &Asked<'_>,
    (report, flags): (&mut Report, &mut quality::Ledger),
    census: &mut dyn Census,
    read_day: &mut F,
) -> Result<bool, ImportRefusal>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    let mut widened = Widened::default();
    let state = (&mut *report, &mut *flags);
    let was = widen_key(ctx, request, asked, state, census, read_day, &mut widened)?;
    widened.tell(ctx, asked.day, report);
    Ok(was)
}

/// What the keys of ONE day read again for a wider scope came to, so the day
/// is counted and told once however many keys closed it (D-4878).
#[derive(Default)]
struct Widened {
    /// Where the report stood before the day's first member key.
    pub(super) before: Option<Mark>,
    /// The member keys closed, and the worst of their closes.
    closed: usize,
    worst: Option<Outcome>,
    /// A member key of the day was refused before any file was read, its
    /// share restated by the auction era ([`restated_past_append`]): the day
    /// is one more in `days_refused`, once however many keys were.
    pub(super) restated: bool,
    /// The members read this day, under whichever key: a member two keys owe
    /// is read once.
    taken: HashSet<String>,
    /// A member key of the day waits for its month's gathered repair
    /// (IA-17b, D-4899): the day is counted now, and that key's close is
    /// told when it is written.
    deferred: bool,
}

impl Widened {
    /// One "day imported" (or "day incomplete", "day refused") and one more
    /// in `days_imported`, when the run closed a member key on the day after
    /// reading it; one more in `days_refused` when a member key was refused
    /// unread as restated. A day whose member keys all wait for a gathered
    /// repair is counted, and told by each member's own close.
    fn tell(&self, ctx: &Ctx<'_>, day: Day, report: &mut Report) {
        if self.restated {
            report.days_refused += 1;
        }
        let Some(before) = self.before else {
            return;
        };
        if self.worst.is_some() || self.deferred {
            report.days_imported += 1;
        }
        if let Some(worst) = self.worst {
            let told = (
                worst,
                Closed::Widened {
                    members: self.closed,
                },
            );
            note_day(ctx.run.kind, day, told, (before, report), None);
        }
    }
}

/// [`widen`] for one key of the day, its closes gathered in `widened`.
fn widen_key<F>(
    ctx: &Ctx<'_>,
    request: &BarRequest,
    asked: &Asked<'_>,
    (report, flags): (&mut Report, &mut quality::Ledger),
    census: &mut dyn Census,
    read_day: &mut F,
    widened: &mut Widened,
) -> Result<bool, ImportRefusal>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    let (key, day) = (asked.key, asked.day);
    let (examined, members) = match ctx.owed(asked) {
        Owed::Nothing => return Ok(false),
        Owed::Pending(members) if asked.own() => (false, Some(members)),
        Owed::Pending(members) => {
            let pending = std::sync::Arc::new(members.into_iter().collect());
            let present = probe(ctx, asked, request, &pending, report, read_day);
            (false, present)
        }
        Owed::Probe(beyond) => (true, probe(ctx, asked, request, &beyond, report, read_day)),
    };
    let Some(mut members) = members else {
        return Ok(true);
    };
    members.retain(|member| !widened.taken.contains(member));
    report.days_widened += 1;
    if asked.early && !members.is_empty() && widened.taken.is_empty() {
        if report.days_before_range == 0 {
            note_reach_back(ctx, asked);
        }
        report.days_before_range += 1;
    }
    if widened.before.is_none() {
        widened.before = Some(Mark::of(report));
    }
    for member in &members {
        let member_key = format!("{key}+{member}");
        // A member key reads its one share through the same append door a
        // base key does, so it is asked the same question [`admit`] asks
        // (D-4878 item 10), here, where every member key is made: refused
        // by name before any file is read, and counted as the day's refusal
        // ([`Widened::tell`]), never as a day imported (check 5 of WP-12f:
        // the receipt read "days imported 1, days refused 0" beside the
        // refusal).
        if ctx.restated(&member_key, day) {
            let _refused = refuse_restated(ctx, (&member_key, day), report);
            widened.restated = true;
            continue;
        }
        let pass = Pass {
            key: &member_key,
            day,
            member: Some(member),
        };
        let state = (&mut *report, &mut *flags);
        if let Some(outcome) = day_pass(ctx, request, &pass, state, &mut *census, read_day)? {
            widened.closed += 1;
            widened.worst = Some(widened.worst.map_or(outcome, |worst| worse(worst, outcome)));
        } else if ctx.gather.borrow().waits(&member_key, day) {
            widened.deferred = true;
        }
    }
    if examined {
        note_examined(ctx, asked, members.len(), report);
    }
    widened.taken.extend(members);
    Ok(true)
}

/// One key of one day: a base key and every file the run wants, or a member
/// key and that member's files (WP-12f, D-4878).
struct Pass<'p> {
    pub(super) key: &'p str,
    pub(super) day: Day,
    pub(super) member: Option<&'p String>,
}

/// WHICH OF THE MEMBERS A DONE DAY'S CLOSE NEVER COVERED HAVE A FILE THAT DAY
/// (WP-12f, D-4878): the day's listing, read once and nothing of it filed.
/// A member whose own key is already done is left out. `None` when the
/// listing cannot be asked: the source no longer holds the day, or refuses
/// its listing, which is named as a failure of the run and leaves the day
/// unexamined, so the next run asks again.
pub(super) fn probe<F>(
    ctx: &Ctx<'_>,
    asked: &Asked<'_>,
    request: &BarRequest,
    beyond: &std::sync::Arc<HashSet<String>>,
    report: &mut Report,
    read_day: &mut F,
) -> Option<Vec<String>>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    let (key, day) = (asked.key, asked.day);
    let (read, extras) = {
        let mut filer = DayFiler::of_day(ctx, (key, day), request, report);
        filer.ask = Ask::Probe(std::sync::Arc::clone(beyond), asked.names.cloned());
        read_day(day, &mut filer)
    };
    if !read.held || !read.refused.is_empty() {
        let why = read.refused.first().map_or_else(
            || {
                "the journal closed this day done and the source holds no file of it now, \
                 so the members a wider scope added cannot be read"
                    .to_owned()
            },
            |refused| refused.why.clone(),
        );
        let cause = Cause::open(Stage::Vendor, "scope_reopen_unreadable");
        report.name(
            cause,
            note_failure(ctx.run.kind, day, "listing", &why, cause),
        );
        return None;
    }
    Some(
        extras
            .present
            .into_iter()
            .filter(|member| !ctx.settled(&format!("{key}+{member}")))
            .collect(),
    )
}

/// Records that `asked`'s key was examined under this run's scope, once
/// every member key it opened is closed, and says so at `info`: under the key
/// itself when it is the run's own, under [`Asked::remembered`] when another
/// filter closed it. A ledger that will not take the line is a named failure
/// of the run; the day is then examined again by the next run, which finds
/// its member keys done and reads no file.
fn note_examined(ctx: &Ctx<'_>, asked: &Asked<'_>, opened: usize, report: &mut Report) {
    let Some(scoped) = &ctx.scoped else {
        return;
    };
    let (key, day) = (asked.remembered, asked.day);
    report.members_reopened += opened;
    let digest = scoped.record.digest();
    // The run's `run` line is in the journal before the ledger names its id:
    // a run that opened no member key wrote no other journal line, and its
    // id was then one the next run could take as its own.
    let written = ctx
        .announce()
        .map_err(|why| why.to_string())
        .and_then(|()| {
            let mut ledger = scoped.ledger.borrow_mut();
            ledger
                .record(ctx.run.kind.as_str(), scoped.record)
                .and_then(|()| ledger.examine(key, digest, (ctx.id, opened)))
        });
    let day_text = day.to_string();
    match written {
        Ok(()) => {
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::info(TARGET, "a done day was read again for a wider scope")
                    .with("kind", ctx.run.kind.as_str())
                    .with("day", day_text.as_str())
                    .with("key", key)
                    .with("scope", digest)
                    .with("members_reopened", opened),
            );
        }
        Err(why) => {
            let cause = Cause::open(Stage::Journal, "scope_ledger");
            report.name(
                cause,
                note_failure(ctx.run.kind, day, "scope ledger", &why, cause),
            );
        }
    }
}

/// One key of one day through the source's `read_day` and the common filing
/// path: its files written, the day's batch made durable, its ledger lines,
/// its census batch and its closing line, in that order.
#[expect(
    clippy::too_many_lines,
    reason = "the day's one ordered sequence (read, the loss probe, the stop, the flush, \
              the ledger lines, the census batch, the close, the run's end) is one state \
              machine over one filer; its run-end checks already live in `run_end`"
)]
fn day_pass<F>(
    ctx: &Ctx<'_>,
    request: &BarRequest,
    pass: &Pass<'_>,
    (report, flags): (&mut Report, &mut quality::Ledger),
    census: &mut dyn Census,
    read_day: &mut F,
) -> Result<Option<Outcome>, ImportRefusal>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    let (run, key, day) = (ctx.run, pass.key, pass.day);
    // A request's window is its range check: a day outside it would have
    // every row dropped and still close.
    debug_assert!(
        request.window.contains(day),
        "{day} is read under a request that does not hold it"
    );
    // THE RUN LEAVES THE MONTH IT GATHERED (IA-17b, D-4899): its months are
    // repaired, once each, and their days closed before this day is read.
    // One compare when nothing is gathered.
    if ctx.gather.borrow().leaves(day) {
        flush_gather(ctx, report, census)?;
    }
    if pass.member.is_some() {
        // A member key was asked whether the auction era restated its share
        // where it was made ([`widen_key`], the one maker of a member
        // `Pass`), as a base key is in [`admit`] (D-4878 item 10).
        let retried = ctx.options.retry_refused && ctx.journal.with_refusals.contains(key);
        name_retried(ctx.journal, run.kind, (key, day, retried), report);
    }
    let before = Mark::of(report);
    let mut files_of_day = DayFiler::of_day(ctx, (key, day), request, report);
    if let Some(member) = pass.member {
        files_of_day.ask = Ask::One(member.clone());
    }
    let left_open = files_of_day.confirm;
    let (read, extras) = read_day(day, &mut files_of_day);
    let DayFiler {
        held,
        mut failures,
        max_back_s: day_back,
        ledger,
        batch,
        confirmed,
        stopped,
        begin,
        disk_full,
        store_lost,
        not_writable,
        held_back_cap,
        gathered,
        ..
    } = files_of_day;
    // THE STORE'S ROOT WENT AWAY (D-4852, FM-086): nothing more is written,
    // not the day's flush, its ledger lines, its census batch or its close,
    // so no writer recreates a root that is gone (FM-125). The day stays
    // begun, and a resume once the volume is back reads it again.
    if let Some(why) = store_lost {
        take_read((run.kind, day), (read, extras), report, &mut failures);
        name_staged(report, failures);
        return Err(ImportRefusal::StoreLost {
            kind: run.kind,
            day,
            why,
            done: Box::new(std::mem::take(report)),
        });
    }
    // THE SOURCE'S ROOT WENT AWAY with the store still there (D-4852,
    // FM-086, INFRA-024): a day nothing of which reached the store gets no
    // line at all, so a drive merely unplugged never reads as days the
    // vendor does not hold (`refused day_not_held`), and the run ends
    // `source_lost`. A day some of whose files already landed closes
    // `incomplete stage=run reason=source_lost` (`decisive` takes the run's
    // cause first) and the run ends after it. Either way a resume once the
    // volume is back reads the day again.
    let source_gone = source_lost(ctx, &read);
    if let Some(why) = &source_gone {
        if begin.is_none() {
            take_read((run.kind, day), (read, extras), report, &mut failures);
            name_staged(report, failures);
            return Err(ImportRefusal::SourceLost {
                kind: run.kind,
                day,
                why: why.clone(),
                done: Box::new(std::mem::take(report)),
            });
        }
        failures.push(note_source_lost_day(run.kind, day, why));
    }
    // A STOP BEFORE THE DAY'S FIRST WRITE (D-4853): no file reached the
    // store, so nothing of the day was started and the journal has nothing
    // to close. The files read before the stop are counted and named.
    if stopped && begin.is_none() {
        take_read((run.kind, day), (read, extras), report, &mut failures);
        name_staged(report, failures);
        return Ok(None);
    }
    // EVERY FILE OF THE DAY IS WRITTEN; NOW IT IS MADE DURABLE, ONCE, and
    // only then are its day-ledger lines, its census batch and its
    // closing line written (WP-16, D-3353). A day whose writes cannot be
    // made durable stops the run with the day left open, so the next run
    // reads it again and confirms what this one wrote.
    if let Some(batch) = batch
        && let Err(why) = make_durable(&batch, (run.kind, day), left_open.then_some(confirmed))
    {
        return Err(stop_at_durability(
            ctx,
            day,
            (before, std::mem::take(report)),
            failures,
            &why,
        ));
    }
    // THE ROOT ONCE MORE BEFORE THE DAY'S LEDGER, CENSUS AND CLOSE (IA-16
    // repair 1; FM-125, RA-284): one `metadata` call per day. A root that
    // went away or was replaced after the day's last file takes none of
    // them, so no ledger line, census or closing line lands in a folder that
    // is not the store, and the day stays begun for a resume.
    if let Some(gone) = super::filing::store_root_lost(run.store_root, ctx.store_id.get()) {
        take_read((run.kind, day), (read, extras), report, &mut failures);
        name_staged(report, failures);
        return Err(ImportRefusal::StoreLost {
            kind: run.kind,
            day,
            why: format!(
                "after the day's files and before its ledger, census and closing line: {gone}"
            ),
            done: Box::new(std::mem::take(report)),
        });
    }
    let (held_day, member_bytes, listing) = (
        read.held,
        (extras.member_bytes, extras.backslash),
        extras.listing,
    );
    take_read((run.kind, day), (read, extras), report, &mut failures);
    if !held_day {
        close_missing(ctx, (key, day), report, failures);
        return Ok(None);
    }
    // A member key's close never writes the whole-day line of the day
    // ledger: that line is the base close's (D-4878, D-3335). Nor does a day
    // a stop cut short: its line would describe files it never read.
    if !stopped {
        write_flags(
            (run, run.only.is_empty() && pass.member.is_none()),
            flags,
            (day, ledger, member_bytes),
            report,
            &mut failures,
        );
    }
    // THE DAY'S CENSUS TIME, told on its closing line (R3-19): the publish
    // reads the run's kept census, so it costs the day's own rows, and a day
    // that did not is visible by this number and by "census published".
    let census_started = std::time::Instant::now();
    if let Some(why) = census.publish(run.store_root, &held) {
        let room = census.room(run.store_root, &held);
        return Err(stop_at_census(
            ctx,
            (key, day),
            (before, std::mem::take(report), day_back),
            failures,
            (why, room),
        ));
    }
    // A DAY IS COUNTED ONCE: here for a day read whole, and in [`widen`] for
    // a day read again for a wider scope, after its last member key.
    // A DAY A STOP CUT SHORT after its `begin` closes `incomplete stage=run
    // reason=stopped` (D-4853): [`decisive`] takes the run's cause before any
    // file's, so it says `stopped` even when an earlier file failed.
    if stopped {
        failures.push(note_stopped_day(run.kind, day));
    }
    let census_ms = u64::try_from(census_started.elapsed().as_millis()).unwrap_or(u64::MAX);
    // THE DAY'S CLOSE WAITS FOR ITS MONTH'S GATHERED REPAIR (IA-17b,
    // D-4899): some of its files' bars are in no month yet, so closing the
    // day now would let a crash before the repair lose them for good (a
    // `done` day is never read again). Its close is written once the repair
    // is done ([`flush_gather`]).
    if gathered > 0 && ctx.gather.borrow().waits(key, day) {
        let close = super::filing::DeferredClose {
            member: pass.member.cloned(),
            stopped,
            before,
            after: Mark::of(report),
            max_back_s: day_back,
            listing,
            failures,
            census_ms,
        };
        return defer_day(
            ctx,
            (key, day, gathered),
            close,
            (source_gone, disk_full, not_writable, held_back_cap),
            report,
        )
        .map(|()| None);
    }
    let closed = match pass.member {
        None if stopped => Closed::Day,
        None => {
            report.days_imported += 1;
            Closed::Day
        }
        Some(member) => Closed::Member { member, key },
    };
    let outcome = close_day(
        ctx,
        (key, day),
        (before, report, day_back),
        listing,
        &mut failures,
    );
    record_close(
        run.kind,
        (day, outcome, closed),
        (before, report),
        failures,
        Some(census_ms),
    );
    if let Some(why) = source_gone {
        return Err(ImportRefusal::SourceLost {
            kind: run.kind,
            day,
            why,
            done: Box::new(std::mem::take(report)),
        });
    }
    run_end(
        (run.kind, day),
        (disk_full, not_writable, held_back_cap),
        report,
    )?;
    Ok(Some(outcome))
}

/// What of a day ends the run once it is closed: a lost source, a full disk,
/// a store that takes no writes, the held-back set at its cap (each `None`
/// when it did not happen).
type DayEnds = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<(String, usize)>,
);

/// A day whose files wait in the gather, its close kept for when its
/// months are repaired (IA-17b, D-4899), and said at `info`.
///
/// A DAY THAT ENDS THE RUN IS LEFT BEGUN instead (a lost source, a full
/// disk, a store that takes no writes, the held-back set at its cap): its
/// gathered bars are in no month, so it is not closed at all, and the next
/// run reads it again (D-2821), as after a crash. Its failures are named in
/// the report; the run ends as it would have.
fn defer_day(
    ctx: &Ctx<'_>,
    (key, day, gathered): (&str, Day, usize),
    close: super::filing::DeferredClose,
    (source_gone, disk_full, not_writable, held_back_cap): DayEnds,
    report: &mut Report,
) -> Result<(), ImportRefusal> {
    let kind = ctx.run.kind;
    if source_gone.is_some()
        || disk_full.is_some()
        || not_writable.is_some()
        || held_back_cap.is_some()
    {
        name_staged(report, close.failures);
        if let Some(why) = source_gone {
            return Err(ImportRefusal::SourceLost {
                kind,
                day,
                why,
                done: Box::new(std::mem::take(report)),
            });
        }
        return run_end(
            (kind, day),
            (disk_full, not_writable, held_back_cap),
            report,
        );
    }
    let _kept = ctx.gather.borrow_mut().defer_close((key, day), close);
    let day_text = day.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(TARGET, "day's close waits for its month's gathered repair")
            .with("kind", kind.as_str())
            .with("day", day_text.as_str())
            .with("key", key)
            .with("files_gathered", gathered)
            .with("run", ctx.id)
            .with("code", "gather_deferred"),
    );
    Ok(())
}

/// `before` moved on by everything the report took after the day's pass
/// (`after`) and before its close (`now`), less the `committed` bars the
/// day's own repairs added: the mark whose difference from `now` is the
/// day's own counts (D-4899's deferred close; [`Counts::of_day`]).
const fn rebased(before: Mark, after: Mark, now: Mark, committed: usize) -> Mark {
    const fn shift(before: usize, after: usize, now: usize) -> usize {
        now.saturating_sub(after.saturating_sub(before))
    }
    Mark {
        files: shift(before.files, after.files, now.files),
        files_skipped: shift(before.files_skipped, after.files_skipped, now.files_skipped),
        rows: shift(before.rows, after.rows, now.rows),
        seconds: shift(before.seconds, after.seconds, now.seconds),
        seconds_committed: shift(
            before.seconds_committed,
            after.seconds_committed,
            now.seconds_committed,
        )
        .saturating_sub(committed),
        late_rows: shift(before.late_rows, after.late_rows, now.late_rows),
        late_unresolved: shift(
            before.late_unresolved,
            after.late_unresolved,
            now.late_unresolved,
        ),
        failures: shift(before.failures, after.failures, now.failures),
    }
}

/// THE GATHER, REPAIRED AND ITS DAYS CLOSED (IA-17b; D-4875 (3), D-4899):
/// when the run leaves the month, ends, or is stopped. Every (instrument,
/// month) gathered is repaired once ([`super::filing::repair_gathered_months`]),
/// the census rows of the months repaired are published together, and only
/// then is each waiting day closed, in day order, by its own failures and
/// those its months' repairs gave it.
///
/// A repair that ends the run (a full disk at publication, a store gone,
/// one that takes no writes) ends it here, no waiting day closed; a census
/// that refuses the rows closes the first waiting day `incomplete` with the
/// census's reason and ends the run, the others left begun. Either way the
/// next run reads every day not closed again (D-2821).
fn flush_gather(
    ctx: &Ctx<'_>,
    report: &mut Report,
    census: &mut dyn Census,
) -> Result<(), ImportRefusal> {
    let gather = std::mem::take(&mut *ctx.gather.borrow_mut());
    if gather.is_empty() {
        return Ok(());
    }
    let super::filing::Repaired {
        held,
        deferred,
        ended,
    } = super::filing::repair_gathered_months(ctx, gather, report);
    let kind = ctx.run.kind;
    if let Some((code, day, why)) = ended {
        for one in deferred {
            name_staged(report, one.failures);
            if let Some(close) = one.close {
                name_staged(report, close.failures);
            }
        }
        let done = Box::new(std::mem::take(report));
        return Err(match code {
            1 => ImportRefusal::DiskFull {
                kind,
                day,
                why,
                done,
            },
            2 => ImportRefusal::StoreLost {
                kind,
                day,
                why,
                done,
            },
            _ => ImportRefusal::StoreNotWritable {
                kind,
                day,
                why,
                done,
            },
        });
    }
    if !held.is_empty()
        && let Some(why) = census.publish(ctx.run.store_root, &held)
    {
        let room = census.room(ctx.run.store_root, &held);
        let Some(first) = deferred.into_iter().next() else {
            // No day waits to carry the refusal: the run ends by it alone,
            // no line written under a key no day has.
            return Err(super::census::census_refusal(
                (kind, ctx.run.to),
                why,
                room,
                std::mem::take(report),
            ));
        };
        let (before, max_back_s, mut failures) = match first.close {
            Some(close) => (
                rebased(close.before, close.after, Mark::of(report), first.committed),
                close.max_back_s,
                close.failures,
            ),
            None => (Mark::of(report), 0, Vec::new()),
        };
        failures.extend(first.failures);
        return Err(stop_at_census(
            ctx,
            (&first.key, first.day),
            (before, std::mem::take(report), max_back_s),
            failures,
            (why, room),
        ));
    }
    for one in deferred {
        close_deferred(ctx, one, report);
    }
    Ok(())
}

/// One waiting day's close, once its months are repaired (IA-17b): exactly
/// the close its pass would have written, with its own counts, and the
/// failures its months' repairs gave its files beside its own.
fn close_deferred(ctx: &Ctx<'_>, one: super::filing::Deferred, report: &mut Report) {
    let super::filing::Deferred {
        key,
        day,
        close,
        failures: repaired,
        committed,
    } = one;
    let Some(close) = close else {
        // A PASS THAT GATHERED AND NEVER REACHED ITS CLOSE (it returned
        // before it, and the run went on): the day stays begun, and is read
        // again by the next run. Said, never silent.
        name_staged(report, repaired);
        let day_text = day.to_string();
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::warn(
                TARGET,
                "a gathered day has no close to write; it stays begun and is read again",
            )
            .with("kind", ctx.run.kind.as_str())
            .with("day", day_text.as_str())
            .with("key", key.as_str())
            .with("code", "gather_unclosed"),
        );
        return;
    };
    let super::filing::DeferredClose {
        member,
        stopped,
        before,
        after,
        max_back_s,
        listing,
        mut failures,
        census_ms,
    } = close;
    failures.extend(repaired);
    let before = rebased(before, after, Mark::of(report), committed);
    let closed = match member.as_deref() {
        None if stopped => Closed::Day,
        None => {
            report.days_imported += 1;
            Closed::Day
        }
        Some(member) => Closed::Member { member, key: &key },
    };
    let outcome = close_day(
        ctx,
        (&key, day),
        (before, report, max_back_s),
        listing,
        &mut failures,
    );
    record_close(
        ctx.run.kind,
        (day, outcome, closed),
        (before, report),
        failures,
        Some(census_ms),
    );
}

/// The source refusal codes that ask whether the source's root is still
/// there (D-4852): the archive or the tick store could not be reached at all
/// ([`cm_cause`]; an options source's faults are the same codes).
const SOURCE_UNREACHED: [&str; 2] = ["archive_unavailable", "tick_store_unavailable"];

/// THE SOURCE'S LOSS PROBE (D-4852, FM-086, INFRA-024): asked only of a day
/// the source says it does not hold, or one of whose files it could not
/// reach, never of a day read whole, so it costs one `metadata` call per
/// such day and nothing on the hot path. The answer names the root and why,
/// and, since macOS answers a volume whose access it revoked with a refusal
/// rather than an absence, where that access is given back. `None` when the
/// root answers, or when the run names no origin (a test's bare reader):
/// there is then no root to ask, and the day closes by its own refusals.
fn source_lost(ctx: &Ctx<'_>, read: &DayRead) -> Option<String> {
    let suspect = !read.held
        || read
            .refused
            .iter()
            .any(|refused| SOURCE_UNREACHED.contains(&refused.cause.code));
    if !suspect {
        return None;
    }
    let root = &ctx.options.origin?.path;
    match std::fs::metadata(root) {
        Ok(_) => None,
        Err(why) => Some(format!(
            "the loss probe could not reach the source's root {} ({why}): the source went away \
             (the drive is gone, or the host revoked this process's access to the volume; on \
             macOS, System Settings > Privacy & Security > Files & Folders > Removable Volumes \
             gives it back), so no day after it is read and none is named not held",
            root.display()
        )),
    }
}

/// The failure a lost source leaves on a day some of whose files already
/// landed (D-4852), at the run's stage, with its event.
fn note_source_lost_day(kind: ImportKind, day: Day, why: &str) -> Staged {
    let cause = Cause::open(Stage::Run, "source_lost");
    Staged {
        cause,
        failure: super::note_failure(kind, day, "source", why, cause),
    }
}

/// THE RUN ENDS ONCE THE DAY IS CLOSED (D-4851, RA-209, FM-129): a full disk
/// after the files in hand, a store that takes no writes (one named stop,
/// never a file failure repeated on every later day), the held-back set at
/// its cap. No new day begins; the closed day is `incomplete` and a resume
/// reads it again.
fn run_end(
    (kind, day): (ImportKind, Day),
    (disk_full, not_writable, held_back_cap): (
        Option<String>,
        Option<String>,
        Option<(String, usize)>,
    ),
    report: &mut Report,
) -> Result<(), ImportRefusal> {
    let done = |report: &mut Report| Box::new(std::mem::take(report));
    if let Some(why) = disk_full {
        return Err(ImportRefusal::DiskFull {
            kind,
            day,
            why,
            done: done(report),
        });
    }
    if let Some(why) = not_writable {
        return Err(ImportRefusal::StoreNotWritable {
            kind,
            day,
            why,
            done: done(report),
        });
    }
    if let Some((first, count)) = held_back_cap {
        return Err(ImportRefusal::HeldBackCap {
            kind,
            day,
            first,
            count,
            done: done(report),
        });
    }
    Ok(())
}

/// The failure a stop leaves on a day it cut short after its `begin`
/// (D-4853), at the stage [`import_cause`] gives a stop, with its event: a
/// stop is asked for, so the event is a warning, never an error.
fn note_stopped_day(kind: ImportKind, day: Day) -> Staged {
    let cause = Cause::open(Stage::Run, "stopped");
    let day_text = day.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(TARGET, "day cut short by a stop")
            .with("kind", kind.as_str())
            .with("day", day_text.as_str())
            .with("stage", cause.stage.as_str())
            .with("code", cause.code),
    );
    Staged {
        cause,
        failure: super::failure_on(
            day,
            &format!("{day} run"),
            "a stop was asked: the files of the day after this point were not taken, and a \
             resume reads the day again"
                .to_owned(),
        ),
    }
}

/// Counts what a source did with a day beside its files: the entries it
/// skipped, the ones it refused (each a failure of the day, after the files'
/// own), and the lineage it wrote and the failures that are no file's.
fn take_read(
    (kind, day): (ImportKind, Day),
    (read, extras): (DayRead, DayExtras),
    report: &mut Report,
    failures: &mut Vec<Staged>,
) {
    report.files_skipped += read.skipped;
    report.files_refused += read.refused.len();
    for refused in read.refused {
        let Refused { cause, failure } = refused;
        let failure = note_failure(kind, day, &failure.instrument, &failure.why, cause);
        failures.push(Staged { cause, failure });
    }
    report.lineage_lines += extras.lineage.lineage;
    report.lot_lines += extras.lineage.lots;
    report.adjustments += extras.lineage.actions;
    failures.extend(extras.failures);
}

/// `day` into one of the report's ascending lists of days, NAMED ONCE however
/// many of its member keys closed that way (D-4878): the days of a run
/// ascend, so the day is the list's last. A close a gathered repair deferred
/// (IA-17b, D-4899) can follow a later day's: it goes in at its place, so
/// every list stays in day order.
pub(super) fn name_in_order(days: &mut Vec<Day>, day: Day) {
    // Back from the end past the later days: at most the days of one month
    // a deferred close waited behind, none on a close in order.
    let mut at = days.len();
    while at > 0 && days.get(at - 1).is_some_and(|later| *later > day) {
        at -= 1;
    }
    if at == 0 || days.get(at - 1) != Some(&day) {
        days.insert(at, day);
    }
}

/// Files a closed day in the report, by how it closed, with its failures, and
/// logs its one summary event ([`note_day`]).
fn record_close(
    kind: ImportKind,
    (day, outcome, closed): (Day, Outcome, Closed<'_>),
    (before, report): (Mark, &mut Report),
    failures: Vec<Staged>,
    census_ms: Option<u64>,
) {
    let named = |days: &mut Vec<Day>| name_in_order(days, day);
    match outcome {
        Outcome::Done => {}
        Outcome::DoneWithRefusals => named(&mut report.closed_with_refusals),
        Outcome::Incomplete => {
            named(&mut report.closed_incomplete);
            // A HOLE THIS RUN MADE (D-6900 keep item 2): the close's own
            // decisive cause, as [`close_of`] chose it, named on the receipt.
            if let Some(hole) = decisive(&failures).filter(|one| one.cause.is_hole())
                && report.holes_made.last().is_none_or(|(at, _)| *at != day)
            {
                report.holes_made.push((day, hole.cause.code.to_owned()));
            }
        }
        Outcome::Refused => named(&mut report.closed_refused),
    }
    name_staged(report, failures);
    note_day(kind, day, (outcome, closed), (before, report), census_ms);
}

/// "run finished", with how its days closed (D-3317).
fn note_finished(ctx: &Ctx<'_>, report: &Report) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(TARGET, "run finished")
            .with("kind", ctx.run.kind.as_str())
            .with("run", ctx.id)
            .with("days", report.days_imported)
            .with("skipped", report.days_skipped)
            .with("missing", report.days_missing.len())
            .with("done_with_refusals", report.closed_with_refusals.len())
            .with("incomplete", report.closed_incomplete.len())
            .with("refused", report.closed_refused.len() + report.days_refused)
            .with("files", report.files)
            .with("seconds", report.seconds)
            .with("failures", report.failures_total),
    );
}

/// Whether a day is read to import it or rescanned for its quality only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    /// The import: the closing-auction prints are filed in their sidecar.
    Import,
    /// `cli gdfl-dayflags`: nothing under `bars/` is written, not even an
    /// auction sidecar.
    Rescan,
}

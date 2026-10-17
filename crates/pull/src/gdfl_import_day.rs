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

use super::causes::{Cause, ImportRefusal, Outcome, Stage, cm_cause};
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

/// A day's failure about `about`, refused by `refusal` (an import refusal
/// that names its own code), at `stage`, logged.
pub(super) fn note_staged(
    run: &Run<'_>,
    day: Day,
    about: &str,
    stage: Stage,
    refusal: &ImportRefusal,
) -> Staged {
    let cause = Cause::open(stage, refusal.code());
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

/// The day's decisive failure: the census's, else the first that keeps the
/// day from closing, else the first (D-3317: its `stage=` and `reason=`).
fn decisive(failures: &[Staged]) -> Option<&Staged> {
    failures
        .iter()
        .find(|one| one.cause.stage == Stage::Census)
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
    if cause.stage == Stage::Census {
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
}

/// The report's counters at a day's start, so the day's own counts are a
/// subtraction (the design's seam 3, WP-16): a `Copy` of seven numbers where
/// this was a clone of the whole report, its failures and day lists
/// included, once per day.
#[derive(Debug, Clone, Copy)]
pub(super) struct Mark {
    pub(super) files: usize,
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
    const fn of_day(before: Mark, report: &Report, failures: usize, max_back_s: u32) -> Self {
        Self {
            files: report.files - before.files,
            seconds: report.seconds - before.seconds,
            failures,
            late: report.late_rows - before.late_rows,
            late_unresolved: report.late_unresolved - before.late_unresolved,
            max_back_s,
        }
    }

    /// The D-2822 stats, as `done` and `incomplete` write them.
    pub(super) fn text(&self) -> String {
        format!(
            "definition={BAR_DEFINITION} files={} seconds={} failures={} late={} \
             late_unresolved={} max_back_s={}",
            self.files,
            self.seconds,
            self.failures,
            self.late,
            self.late_unresolved,
            self.max_back_s,
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
                failures.push(note_staged(
                    self.run,
                    day,
                    "scope ledger",
                    Stage::Journal,
                    &why,
                ));
                return Outcome::Incomplete;
            }
        };
        let unrun = closing_line(key, close, counts, &scope);
        if matches!(close, Close::Refused(_)) && self.journal.refused_close.get(key) == Some(&unrun)
        {
            return close.outcome();
        }
        match self.append(&format!("{unrun} run={}", self.id)) {
            Ok(_) => close.outcome(),
            Err(why) => {
                failures.push(note_staged(self.run, day, "journal", Stage::Journal, &why));
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
    failures: &mut Vec<Staged>,
) -> Outcome {
    let counts = Counts::of_day(before, report, failures.len(), max_back_s);
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
fn admit(ctx: &Ctx<'_>, key: &str, day: Day, report: &mut Report) -> bool {
    let retried = ctx.options.retry_refused && ctx.journal.with_refusals.contains(key);
    if ctx.journal.done.contains(key) && !retried {
        report.days_skipped += 1;
        return false;
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
    drive_with(run, RunOptions::default(), &mut StoreCensus, read_day)
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
    drive_with(run, options, &mut StoreCensus, read_day)
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
        let _reads = filer.file_all(&handed, &|slot| {
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
    (day, ledger, member_bytes): (Day, Option<quality::DayLedger>, Option<u64>),
    report: &mut Report,
    failures: &mut Vec<Staged>,
) {
    let Some(mut ledger) = ledger else {
        return;
    };
    ledger.set_member_bytes(member_bytes);
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
            Stage::Journal,
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
    if let Ok(report) = &mut end {
        report.seal();
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
        id: journal.fresh_run(started),
        started,
        announced: std::cell::Cell::new(false),
        scoped,
    };
    let _key = RunKey::claim(ctx.id);
    if let Some(observer) = options.observer {
        observer.begun(run.kind, ctx.id);
    }
    note_started(&ctx, &filter);
    note_torn(&ctx);
    let mut report = Report::default();
    let state = (&mut report, &mut flags);
    reach_back(&ctx, &filter, state, &mut *census, &mut read_day)?;
    let range = days(run.from, run.to);
    let total = range.len();
    for (index, day) in range.into_iter().enumerate() {
        if let Some(observer) = options.observer {
            observer.day_begun(run.kind, day, (index, total));
        }
        'day: {
            let key = format!("{} {day} {filter}", run.kind.as_str());
            let state = (&mut report, &mut flags);
            let asked = Asked {
                key: &key,
                day,
                remembered: &key,
                names: None,
                early: false,
            };
            if widen(&ctx, &request, &asked, state, &mut *census, &mut read_day)? {
                break 'day;
            }
            if !admit(&ctx, &key, day, &mut report) {
                break 'day;
            }
            let pass = Pass {
                key: &key,
                day,
                member: None,
            };
            let state = (&mut report, &mut flags);
            day_pass(&ctx, &request, &pass, state, &mut *census, &mut read_day)?;
        }
        if let Some(observer) = options.observer {
            observer.day_passed(run.kind, day, &report);
        }
    }
    note_finished(&ctx, &report);
    Ok(report)
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
}

impl Widened {
    /// One "day imported" (or "day incomplete", "day refused") and one more
    /// in `days_imported`, when the run closed a member key on the day after
    /// reading it; one more in `days_refused` when a member key was refused
    /// unread as restated.
    fn tell(&self, ctx: &Ctx<'_>, day: Day, report: &mut Report) {
        if self.restated {
            report.days_refused += 1;
        }
        if let (Some(before), Some(worst)) = (self.before, self.worst) {
            report.days_imported += 1;
            let told = (
                worst,
                Closed::Widened {
                    members: self.closed,
                },
            );
            note_day(ctx.run.kind, day, told, (before, report));
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
        ..
    } = files_of_day;
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
    let (held_day, member_bytes) = (read.held, extras.member_bytes);
    take_read((run.kind, day), (read, extras), report, &mut failures);
    if !held_day {
        close_missing(ctx, (key, day), report, failures);
        return Ok(None);
    }
    // A member key's close never writes the whole-day line of the day
    // ledger: that line is the base close's (D-4878, D-3335).
    write_flags(
        (run, run.only.is_empty() && pass.member.is_none()),
        flags,
        (day, ledger, member_bytes),
        report,
        &mut failures,
    );
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
    let closed = match pass.member {
        None => {
            report.days_imported += 1;
            Closed::Day
        }
        Some(member) => Closed::Member { member, key },
    };
    let outcome = close_day(ctx, (key, day), (before, report, day_back), &mut failures);
    record_close(run.kind, (day, outcome, closed), (before, report), failures);
    Ok(Some(outcome))
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

/// Files a closed day in the report, by how it closed, with its failures, and
/// logs its one summary event ([`note_day`]).
fn record_close(
    kind: ImportKind,
    (day, outcome, closed): (Day, Outcome, Closed<'_>),
    (before, report): (Mark, &mut Report),
    failures: Vec<Staged>,
) {
    // A DAY IS NAMED ONCE however many of its member keys closed that way
    // (D-4878): the days of a run ascend, so the day is the list's last.
    let named = |days: &mut Vec<Day>| {
        if days.last() != Some(&day) {
            days.push(day);
        }
    };
    match outcome {
        Outcome::Done => {}
        Outcome::DoneWithRefusals => named(&mut report.closed_with_refusals),
        Outcome::Incomplete => named(&mut report.closed_incomplete),
        Outcome::Refused => named(&mut report.closed_refused),
    }
    name_staged(report, failures);
    note_day(kind, day, (outcome, closed), (before, report));
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

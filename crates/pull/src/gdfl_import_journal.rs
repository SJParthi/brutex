//! The importer's journal (R3-35, a move-only split of `gdfl_import.rs`):
//! its path, the bar definition its days are built under, its lines read
//! and written, and the closing and `run` lines.

use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use crate::session::Day;

use super::causes::ImportRefusal;
use super::day::{Close, Counts};
use super::{ImportKind, Origin, TARGET, encoded};

/// The import journal's path under a store root.
#[must_use]
pub fn journal_path(store_root: &Path) -> PathBuf {
    store_root.join("imports").join("gdfl.journal")
}

/// The version of the one-second bar definition (the row rule, the
/// placement and the fold this module applies), written into every `done`
/// and `incomplete` line as `definition=<n>` (D-2822). Bump it whenever a
/// change makes one file build different bars: a day closed `done` under
/// another number is imported again, and the append-only store judges the
/// result, answering `AlreadyPresent` for the same bars and refusing the file
/// by name for different ones, never writing over them.
///
/// 1 is D-2802 as first written, whose placement took the running maximum
/// of traded rows only; 2 is D-2812, which takes it over every row; 3 is
/// D-3342 (the operator's answer, 7 Oct 2026): an option bar's open interest
/// is the OI of the LAST row placed in its second, traded or not
/// (`fold::fold_seconds_with_last_oi`), where 2 took the last traded row's;
/// 4 is the closing auction's band through order matching and its booked
/// match (SC-CLOSE-5, SC-CLOSE-6; D-3320, D-4878 item 10): from 2026-08-03 an
/// eligible share's file leaves the rows of `[15:28:00, 15:35:00)` at one
/// price, booked or not, and the zero-book rows of `[15:30:00, 15:35:00)`, to
/// the `.auc` sidecar instead of the fold, so one such file builds different
/// bars under 4 than under 3 or 2 (`gdfl_cm::AUCTION_BAND_END`,
/// `gdfl_cm::AUCTION_MATCH_FROM`).
pub const BAR_DEFINITION: u32 = 4;

/// The definition D-3342 replaced.
const D3342_PREVIOUS: u32 = 2;

/// The definition the closing auction's band through matching replaced
/// (SC-CLOSE-5, SC-CLOSE-6).
const AUCTION_PREVIOUS: u32 = 3;

/// Whether a day of `kind` built under `definition` holds the bars this
/// build would build (D-2822, D-3342, D-4878 item 10): [`BAR_DEFINITION`]
/// always; 3 for every day but a share's from 2026-08-03; and 2 for an index,
/// and for a share before 2026-08-03.
///
/// Definition 3 changed only the open interest of a second, and the
/// capital-market trees carry none (`OI_NULL`, D-3322), so an index's or a
/// share's bars under 2 and 3 are the same bars, byte for byte. Definition 4
/// changed only what an eligible share's file folds from 2026-08-03, the one
/// rule [`crate::vendor::cash_auction_eligibility_required`] answers, so an
/// index's, an option's and a share's earlier day hold under 3 the bars 4
/// builds. Reading those days again would change nothing and is not asked of
/// them. A share's day from 2026-08-03 under 2 or 3 is restated, eligible or
/// not (the journal's key names no share's flag, and the store judges what a
/// second reading offers: `AlreadyPresent` for the same bars, a refusal by
/// name for different ones); so is an option day under 2, whose open
/// interest may differ (CMP-07: before this, a share day under 2 passed as
/// current whichever rule set had built it).
#[must_use]
pub fn definition_is_current(kind: ImportKind, day: Day, definition: u64) -> bool {
    current_under(
        kind,
        crate::vendor::cash_auction_eligibility_required(day),
        definition,
    )
}

/// [`definition_is_current`] once the day is reduced to the one question
/// definition 4 asks of it: whether it is in the closing auction's era.
fn current_under(kind: ImportKind, auction_era: bool, definition: u64) -> bool {
    if definition == u64::from(BAR_DEFINITION) {
        return true;
    }
    if kind == ImportKind::Stocks && auction_era {
        return false;
    }
    definition == u64::from(AUCTION_PREVIOUS)
        || (definition == u64::from(D3342_PREVIOUS) && kind != ImportKind::Options)
}

/// [`definition_is_current`] for a journal key's kind and day words; a word
/// that is no kind is current only under [`BAR_DEFINITION`], and a day word
/// that is no `YYYY-MM-DD` day is taken to be in the auction's era, so a
/// share's key that cannot say when it was is read again, never passed.
fn current_for(kind: &str, day: &str, definition: u32) -> bool {
    let Ok(kind) = ImportKind::parse(kind) else {
        return definition == BAR_DEFINITION;
    };
    let auction_era = journal_day(day).is_none_or(crate::vendor::cash_auction_eligibility_required);
    current_under(kind, auction_era, u64::from(definition))
}

/// A journal key's day word as a day: exactly `YYYY-MM-DD`, a real date.
fn journal_day(text: &str) -> Option<Day> {
    let mut parts = text.split('-');
    let (Some(y), Some(m), Some(d), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    if y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    let day = Day::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?).ok()?;
    (day.to_string() == text).then_some(day)
}

/// The definition the code of D-2811 and D-2812 built under. Both landed in
/// one commit, so every journal line that code wrote was written under 2.
const D2811_DEFINITION: u32 = 2;

/// The stats fields, in order, of every `done` and `incomplete` line the
/// journal wrote from D-2811 until it recorded `definition=` (D-2822).
/// Before D-2811 a line carried only the first three, under definition 1.
const D2811_STATS: [&str; 6] = [
    "files",
    "seconds",
    "failures",
    "late",
    "late_unresolved",
    "max_back_s",
];

/// The `definition=` field of a journal line's stats: `Ok(None)` when it
/// has none, `Err(())` when it has two or one that is not a number in the
/// form [`BAR_DEFINITION`] is written in.
fn definition_of(stats: &[&str]) -> Result<Option<u32>, ()> {
    let mut found = None;
    for field in stats {
        if let Some(text) = field.strip_prefix("definition=") {
            let value: u32 = text.parse().map_err(|_| ())?;
            if found.is_some() || value.to_string() != text {
                return Err(());
            }
            found = Some(value);
        }
    }
    Ok(found)
}

/// The bar definition a `done` line says built its day: its `definition=`
/// field, else [`D2811_DEFINITION`] when its stats are exactly
/// [`D2811_STATS`], each a decimal count, as the code of D-2811 wrote them
/// before the field existed; else `None`, a day no definition is known for
/// (D-2822). O(fields).
fn built_under(stats: &[&str]) -> Result<Option<u32>, ()> {
    if let Some(stated) = definition_of(stats)? {
        return Ok(Some(stated));
    }
    let d2811 = stats.len() == D2811_STATS.len()
        && stats.iter().zip(D2811_STATS).all(|(field, name)| {
            field
                .strip_prefix(name)
                .and_then(|rest| rest.strip_prefix('='))
                .is_some_and(|count| !count.is_empty() && count.bytes().all(|b| b.is_ascii_digit()))
        });
    Ok(d2811.then_some(D2811_DEFINITION))
}

/// The decimal count a field holds (`name=<n>`), `Ok(None)` when the line
/// has no such field, `Err(())` when it has two or one that is not a count as
/// this journal writes it (no sign, no leading zero but `0`). O(fields).
pub(super) fn count_of(stats: &[&str], name: &str) -> Result<Option<u64>, ()> {
    let mut found = None;
    for field in stats {
        if let Some(text) = field
            .strip_prefix(name)
            .and_then(|rest| rest.strip_prefix('='))
        {
            let value: u64 = text.parse().map_err(|_| ())?;
            if found.is_some() || value.to_string() != text {
                return Err(());
            }
            found = Some(value);
        }
    }
    Ok(found)
}

/// The `run=` field of a closing line, refused when it is there and not a
/// count (D-3317 item 2). Lines written before D-3317 have none.
fn run_of(stats: &[&str]) -> Result<Option<u64>, ()> {
    count_of(stats, "run")
}

/// What closes a torn journal line, so every later load knows it for one
/// (D-2814). A line ending in it is skipped; no line the journal writes
/// whole can end in it, because a key is a kind, a day and symbol names.
const TORN: &str = " (torn)";

/// Whether `fragment`, a line cut short by a crash, can be the start of a
/// line the journal writes: a prefix of one of its [`VERBS`] and the space
/// after it, or text that starts with one (D-2814). Anything else at the end
/// of the file is not a torn journal line but a foreign one.
///
/// The close a load writes can itself be cut by a second crash, leaving `d`
/// followed by part of [`TORN`] (`d (t`), and the next load closes that
/// again. So every trailing piece of the mark, whole or cut, is taken off
/// before the verb rule is asked: each is a byte the journal wrote, and
/// judging `d (t` foreign refused every later run for ever (D-2817). The
/// mark's seven bytes are all different, so its last byte names the one
/// piece a fragment can end in. What is left is never empty: a close follows
/// only a fragment of at least a verb's first letter, which no piece of the
/// mark ends in. O(fragment length).
pub(super) fn torn_fragment(fragment: &str) -> bool {
    let mut rest = fragment;
    'strip: loop {
        for cut in (1..=TORN.len()).rev() {
            if let Some(shorter) = TORN.get(..cut).and_then(|piece| rest.strip_suffix(piece)) {
                rest = shorter;
                continue 'strip;
            }
        }
        break;
    }
    !rest.is_empty()
        && VERBS
            .into_iter()
            .any(|verb| verb.starts_with(rest) || rest.starts_with(verb))
}

/// Every verb the journal holds, each with the space after it: `refused`,
/// `run` and `recounted` since D-3317 (`recounted` is written by `cli
/// census-recount`, D-3316).
const VERBS: [&str; 6] = [
    "begin ",
    "done ",
    "incomplete ",
    "refused ",
    "run ",
    "recounted ",
];

/// The journal, read once per run: the keys done under [`BAR_DEFINITION`]
/// (and which of them closed with refusals), the keys begun and not
/// finished, the keys closed `incomplete` or `refused`, the keys whose last
/// `done` line names another definition (D-2821, D-2822), and the run ids its
/// `run` lines hold (D-3317).
#[derive(Debug, Default)]
pub(super) struct Journal {
    pub(super) path: PathBuf,
    pub(super) done: HashSet<String>,
    pub(super) open: HashSet<String>,
    pub(super) incomplete: HashSet<String>,
    pub(super) restated: HashSet<String>,
    /// The done keys whose last `done` line carries `vendor_refused=` above
    /// zero: read again only under [`RunOptions::retry_refused`].
    pub(super) with_refusals: HashSet<String>,
    /// Every id a `run` line holds, so a new run takes one no line holds.
    pub(super) runs: HashSet<u64>,
    /// The keys whose last line is `refused`, each with that line less its
    /// `run=` field: a day refused again for the same stage, reason and count
    /// is not written again ([`Ctx::close`]), so a re-run over a special
    /// session or a day the source lacks writes nothing (D-2803).
    pub(super) refused_close: HashMap<String, String>,
    /// The journal's length in bytes and in whole lines as this run last
    /// knew them, so the line a `begin` lands on can be named in a
    /// provenance record (D-3334's `journal_ordinal`). A journal another
    /// process appended to since is counted again before the next line.
    known: std::cell::Cell<(u64, u64)>,
    /// The `<kind> <day>` of every key in `open`, whatever its filter: the
    /// days an earlier run began and never closed (it was killed, or its day
    /// could not be made durable), so what it wrote of them was never made
    /// durable (WP-16, D-3353; [`Journal::left_open`]).
    open_days: HashSet<String>,
    /// The `scope=` of each key's last `done` line (WP-12f, D-4878): the
    /// digest of the members that close was asked to read. A key whose last
    /// `done` carries none was closed before scopes were recorded.
    pub(super) scope_done: HashMap<String, String>,
    /// The members a base key (`<kind> <day> <filter>`) has keys of their own
    /// for (`<kind> <day> <filter>+<member>`), whatever closed them: the
    /// members a wider scope reopened that day (D-4878).
    pub(super) member_keys: HashMap<String, std::collections::BTreeSet<String>>,
    /// Per `<kind> <day>`, the filters whose base key is closed `done`, in
    /// order (D-4878): every close a later run may ask what it owes
    /// ([`reach_back`]). Built once at the load's end, O(done keys), each
    /// day's filters an ordered set (a day holds the operator's few, so an
    /// insert is a few compares and the order needs no sort); one hash probe
    /// to read.
    pub(super) closed_by: HashMap<String, std::collections::BTreeSet<String>>,
    /// Per `stocks <day>` of the closing auction's era, the filters (`*`, a
    /// names list, or `<filter>+<member>`) of the keys that may have left
    /// that day's months holding a bar definition 4 no longer builds (WP-12f,
    /// D-4878 item 10): a key whose last `done` or `recounted` names an
    /// earlier definition, one closed `incomplete` under one, and one begun
    /// by a run under one and never closed. Kept in journal order while the
    /// load reads, so a later close under 4 that covers a key's shares ends
    /// it ([`Journal::read_line`]); only the load reads it. Each filter maps
    /// to how the closes under 4 after it ended it ([`Ended`]).
    auction_keys: HashMap<String, HashMap<String, Ended>>,
    /// The keys of [`Journal::auction_keys`] begun by a run under an earlier
    /// definition and not closed since, each with its filter's state before
    /// that `begin` (absent, or the closes that had ended it) and the id of
    /// the run that began it (the last whole `run` line before the `begin`;
    /// `None` when no such line precedes it). Only THAT run's own `refused` close (which writes
    /// nothing) gives the filter back its earlier state: a `refused` line of
    /// another run, or one whose run is unknown, says nothing of what the
    /// early run wrote before it was killed, and keeps the entry (check 5 of
    /// WP-12f: a later run's own refusal had erased the early begin, so the
    /// next run read the share through the append door).
    begun_early: HashMap<String, (Option<Ended>, Option<u64>)>,
    /// The `definition=` of the last `run` line read: `None` before one, and
    /// for a run line that names none, which every build before definition 4
    /// wrote (WP-12f, D-4878 item 10).
    run_definition: Option<u32>,
    /// The id of the last whole `run` line read: the run every `begin` after
    /// it belongs to, since a run writes its `run` line before any other
    /// (D-3317 item 4) and a `begin` carries no `run=` of its own.
    run_id: Option<u64>,
    /// [`Journal::auction_keys`] folded once at the load's end into the
    /// shares each day's months may hold such a bar of: what admission asks,
    /// one hash probe a day and one a name the asking key names
    /// ([`restated_past_append`]).
    pub(super) auction_restated: HashMap<String, AuctionRestated>,
}

/// How the closes under 4 after a restated key of the auction era ended it
/// (WP-12f, D-4878 item 10).
#[derive(Clone, Debug, Default)]
struct Ended {
    /// A close under 4 of a names filter or a member key that reads every
    /// share this key reads: built as 4 builds them, so the key is ended.
    pub(super) named: bool,
    /// The whole-tree closes under 4 after it, each by its `scope=` (`""` for
    /// one that names none: the swept surface, D-4878). A whole-tree close
    /// read the members of ITS scope and no others, so it ends the key only
    /// for the shares that scope holds ([`AuctionRestated::meets`]). Taking
    /// it as a reading of every share let a member key the close was owed,
    /// and a names key outside its scope, read the share through the append
    /// door (check 5 of WP-12f). A day holds the operator's few closes.
    pub(super) whole: Vec<String>,
}

/// What a whole-tree close's scope held, as [`Ended::whole`] spells the
/// scope: whether a close under that scope read `share`. A run that cannot
/// tell (no scope ledger, or a digest the ledger does not hold) answers
/// `false`: refusing too much fails loudly, refusing too little fails
/// silently.
pub(super) type ScopeHeld<'h> = &'h dyn Fn(&str, &str) -> bool;

/// The shares of one `stocks <day>` of the closing auction's era whose months
/// may hold a bar definition 4 no longer builds (WP-12f, D-4878 item 10).
#[derive(Debug, Default)]
pub(super) struct AuctionRestated {
    /// A whole-tree key (`*`) no close under 4 ended is among them: every
    /// share of the day.
    pub(super) every: bool,
    /// The shares of the names and member keys no close under 4 ended.
    pub(super) names: HashSet<String>,
    /// Per share a names or member key named, for each such key only
    /// whole-tree closes under 4 ended, those closes' scopes: the share
    /// stands unless one scope of every such key held it.
    whole_named: HashMap<String, Vec<Vec<String>>>,
    /// For each whole-tree key only whole-tree closes under 4 ended, those
    /// closes' scopes: every share stands unless one of them held it.
    whole_every: Vec<Vec<String>>,
}

impl AuctionRestated {
    /// Whether a key under `filter` would read a share this day holds such a
    /// bar of: under `*`, when any key's share stands; otherwise one probe a
    /// share the filter names, and for a share only whole-tree closes ended
    /// one probe of `held` a scope of the keys naming it (the day's few).
    pub(super) fn meets(&self, filter: &str, held: ScopeHeld<'_>) -> bool {
        let unheld =
            |share: &str, scopes: &[String]| !scopes.iter().any(|scope| held(scope, share));
        let stands = |share: &str| {
            self.names.contains(share)
                || self
                    .whole_named
                    .get(share)
                    .is_some_and(|keys| keys.iter().any(|scopes| unheld(share, scopes)))
                || self.whole_every.iter().any(|scopes| unheld(share, scopes))
        };
        self.every
            || match shares_of(filter) {
                // The whole tree reads every share its scope holds: a whole
                // tree's key ended only by whole-tree closes stands for it
                // unless one of them held every share, which no probe of one
                // share can tell; a names key's share is walked, the day's
                // few, ending at the first that stands.
                None => {
                    !self.names.is_empty()
                        || !self.whole_every.is_empty()
                        || self
                            .whole_named
                            .iter()
                            .any(|(share, keys)| keys.iter().any(|scopes| unheld(share, scopes)))
                }
                Some(mut names) => names.any(stands),
            }
    }
}

/// The shares a journal key's filter part reads: `None` for `*` (every one),
/// for a member key (`<filter>+<member>`) the one share its member is filed
/// under ([`crate::gdfl_cm::member_share`]: `NSE-CASH-RELIANCE.BE` reads
/// RELIANCE's months), else the names.
fn shares_of(filter: &str) -> Option<impl Iterator<Item = &str>> {
    let names = match filter.split_once('+') {
        Some((_base, member)) => crate::gdfl_cm::member_share(member),
        None if filter == "*" => return None,
        None => filter,
    };
    Some(names.split(','))
}

/// A closing line's `scope=`, `None` when it carries none (D-4878).
fn scope_of<'l>(stats: &[&'l str]) -> Option<&'l str> {
    stats.iter().find_map(|field| field.strip_prefix("scope="))
}

/// Whether `kind` and `day`, as a journal line spells them, are a share's day
/// of the closing auction's era (D-4878 item 10).
fn auction_share_day(kind: &str, day: &str) -> bool {
    kind == ImportKind::Stocks.as_str()
        && journal_day(day).is_some_and(crate::vendor::cash_auction_eligibility_required)
}

impl Journal {
    /// Reads the journal at `path`; a missing one is empty. A torn last line
    /// (no newline: a crash mid-write) is ignored and closed before the next
    /// write. The lines are those of the module doc (D-3317): `run`, `begin`,
    /// `done`, `incomplete`, `refused` and `recounted`; the last closing line
    /// of a key wins. Any other line, or a `definition=`, `run=`,
    /// `vendor_refused=` or `months=` that is not one count as this journal
    /// writes it, refuses the run (D-2814).
    pub(super) fn load(path: PathBuf) -> Result<Self, ImportRefusal> {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(why) => {
                return Err(ImportRefusal::Journal {
                    why: format!("{}: {why}", path.display()),
                });
            }
        };
        let mut journal = Self {
            path,
            ..Self::default()
        };
        journal.known.set((
            u64::try_from(text.len()).unwrap_or(u64::MAX),
            newlines(text.as_bytes()),
        ));
        let (whole, tail) = text.rsplit_once('\n').unwrap_or(("", text.as_str()));
        let shown = journal.path.display().to_string();
        let foreign = |line: &str| ImportRefusal::Journal {
            why: format!("{shown}: not a journal line: {line:?}"),
        };
        // Every whole line is judged BEFORE anything is written, and a torn
        // tail must be the start of a line the journal writes: a foreign file
        // is refused on every run and never touched. Closing first let a
        // one-line foreign file be refused once and then, its line now
        // "torn", read as an empty journal by every later run (D-2814).
        if !tail.is_empty() && !torn_fragment(tail) {
            return Err(foreign(tail));
        }
        for line in whole.split('\n').filter(|line| !line.is_empty()) {
            if let Some(fragment) = line.strip_suffix(TORN) {
                if torn_fragment(fragment) {
                    continue;
                }
                return Err(foreign(line));
            }
            let fields: Vec<&str> = line.split(' ').collect();
            journal
                .read_line(fields.as_slice())
                .map_err(|()| foreign(line))?;
        }
        if !tail.is_empty() {
            // Closed with a mark, never a bare newline: a bare one made the
            // torn fragment a whole line that the NEXT load refused as
            // foreign, so one crash mid-write stopped every later run
            // (D-2814).
            journal.write(None)?;
        }
        // Once, O(open keys): the days left open, whatever filter began them.
        journal.open_days = journal
            .open
            .iter()
            .filter_map(|key| key.rsplit_once(' ').map(|(day, _only)| day.to_owned()))
            .collect();
        // Once, O(done keys): which filters closed each day done, in order
        // whatever order the set hands them in. A member key
        // (`<filter>+<member>`) is its base key's and is not one. Pre-sized:
        // a done key names at most one day.
        let mut closed_by: HashMap<String, std::collections::BTreeSet<String>> =
            HashMap::with_capacity(journal.done.len());
        for key in &journal.done {
            if let Some((day, only)) = key.rsplit_once(' ')
                && !only.contains('+')
            {
                closed_by
                    .entry(day.to_owned())
                    .or_default()
                    .insert(only.to_owned());
            }
        }
        journal.closed_by = closed_by;
        // Once, O(the era's restated keys): the shares each day's months may
        // hold a bar definition 4 no longer builds of (D-4878 item 10).
        let mut auction_restated: HashMap<String, AuctionRestated> =
            HashMap::with_capacity(journal.auction_keys.len());
        for (of_day, filters) in std::mem::take(&mut journal.auction_keys) {
            let mut restated = AuctionRestated::default();
            for (filter, ended) in filters {
                if ended.named {
                    continue;
                }
                let shares = shares_of(&filter).map(|names| names.map(str::to_owned).collect());
                match (shares, ended.whole.is_empty()) {
                    (None, true) => restated.every = true,
                    (Some(shares), true) => restated.names.extend::<Vec<String>>(shares),
                    (None, false) => restated.whole_every.push(ended.whole),
                    (Some(shares), false) => {
                        for share in shares {
                            restated
                                .whole_named
                                .entry(share)
                                .or_default()
                                .push(ended.whole.clone());
                        }
                    }
                }
            }
            if restated.every
                || !restated.names.is_empty()
                || !restated.whole_named.is_empty()
                || !restated.whole_every.is_empty()
            {
                auction_restated.insert(of_day, restated);
            }
        }
        journal.auction_restated = auction_restated;
        Ok(journal)
    }

    /// Notes that a key under `only` of a share's day of the auction era may
    /// have left a bar definition 4 no longer builds in that day's months
    /// (D-4878 item 10).
    fn note_auction_key(&mut self, kind: &str, day: &str, only: &str) {
        self.auction_keys
            .entry(format!("{kind} {day}"))
            .or_default()
            .insert(only.to_owned(), Ended::default());
    }

    /// A close under 4 of `only`, whose `scope=` is `scope`: what it read is
    /// built as 4 builds it. A names or member close ends every key of the
    /// day whose shares it names all of; a whole-tree close is noted on every
    /// key of the day with its scope, and ends a key's share only where that
    /// scope held it ([`Ended::whole`]). O(the day's entries): a day holds
    /// the operator's few filters.
    fn end_auction_keys(&mut self, (kind, day, only): (&str, &str, &str), scope: Option<&str>) {
        let Some(filters) = self.auction_keys.get_mut(&format!("{kind} {day}")) else {
            return;
        };
        match shares_of(only) {
            None => {
                let scope = scope.unwrap_or("");
                for ended in filters.values_mut() {
                    if !ended.whole.iter().any(|held| held == scope) {
                        ended.whole.push(scope.to_owned());
                    }
                }
            }
            Some(wide) => {
                let wide: Vec<&str> = wide.collect();
                for (filter, ended) in filters.iter_mut() {
                    if shares_of(filter)
                        .is_some_and(|mut narrow| narrow.all(|name| wide.contains(&name)))
                    {
                        ended.named = true;
                    }
                }
            }
        }
    }

    /// What one closing or opening line does to [`Journal::auction_keys`]
    /// (D-4878 item 10). `built` is the definition the line says built the
    /// day, `None` for a `begin`, whose run line says it, or a `refused`;
    /// `run` is the closing line's `run=`, `None` for a `begin` or a line
    /// that carries none; `scope` its `scope=`, `None` for none.
    fn auction_line(
        &mut self,
        verb: &str,
        (kind, day, only): (&str, &str, &str),
        (built, run, scope): (Option<u32>, Option<u64>, Option<&str>),
    ) {
        if !auction_share_day(kind, day) {
            return;
        }
        let key = format!("{kind} {day} {only}");
        let current = |definition: Option<u32>| {
            definition.is_some_and(|definition| current_for(kind, day, definition))
        };
        match verb {
            "begin" => {
                if !current(self.run_definition) {
                    let before = self
                        .auction_keys
                        .get(&format!("{kind} {day}"))
                        .and_then(|filters| filters.get(only))
                        .cloned();
                    self.begun_early.entry(key).or_insert((before, self.run_id));
                    self.note_auction_key(kind, day, only);
                }
            }
            "refused" => {
                // The early run's OWN refusal wrote nothing, so its `begin`
                // is undone. Any other run's, or one whose run is unknown on
                // either side, keeps the entry: refusing too much fails
                // loudly, refusing too little reads the share through the
                // append door and closes it done, unsaid.
                let own = matches!(
                    (self.begun_early.get(&key), run),
                    (Some(&(_, Some(begun))), Some(closed)) if begun == closed
                );
                if own
                    && let Some((before, _)) = self.begun_early.remove(&key)
                    && let Some(filters) = self.auction_keys.get_mut(&format!("{kind} {day}"))
                {
                    match before {
                        Some(ended) => filters.insert(only.to_owned(), ended),
                        None => filters.remove(only),
                    };
                }
            }
            "incomplete" => {
                self.begun_early.remove(&key);
                if !current(built) {
                    self.note_auction_key(kind, day, only);
                }
            }
            _ => {
                self.begun_early.remove(&key);
                if current(built) {
                    self.end_auction_keys((kind, day, only), scope);
                } else {
                    self.note_auction_key(kind, day, only);
                }
            }
        }
    }

    /// Whether an earlier run began `day` of `kind`, under any filter, and
    /// never closed it (WP-16, D-3353). Such a run was killed, or stopped at
    /// a day it could not make durable, after it had written some of the
    /// day's months: read again, those months answer "already present" and
    /// nothing is written, so this run must make them durable itself before
    /// it closes the day ([`store::file::DurableBatch::confirm_month`]). One
    /// hash probe.
    pub(super) fn left_open(&self, kind: ImportKind, day: Day) -> bool {
        self.open_days.contains(&format!("{} {day}", kind.as_str()))
    }

    /// A `done` line of `kind day only`: [`Journal::read_line`]'s arm, apart
    /// so that function stays one screen. O(fields).
    fn read_done(
        &mut self,
        (kind, day, only): (&str, &str, &str),
        stats: &[&str],
    ) -> Result<(), ()> {
        let definition = built_under(stats)?;
        let run = run_of(stats)?;
        let refused = count_of(stats, "vendor_refused")?.unwrap_or(0);
        self.auction_line(
            "done",
            (kind, day, only),
            (definition, run, scope_of(stats)),
        );
        let key = format!("{kind} {day} {only}");
        self.refused_close.remove(&key);
        self.open.remove(&key);
        self.incomplete.remove(&key);
        // The LAST `done` of a key says which definition built what
        // the store holds (D-2822); an index day built under 2 or 3,
        // and a share's before 2026-08-03, holds the bars 4 builds
        // (D-3342, D-4878 item 10, [`definition_is_current`]).
        // The LAST `done` of a key also says which members it was
        // asked to read (D-4878); one that says nothing is a close
        // from before scopes were recorded.
        match scope_of(stats) {
            Some(digest) => self.scope_done.insert(key.clone(), digest.to_owned()),
            None => self.scope_done.remove(&key),
        };
        if definition.is_some_and(|definition| current_for(kind, day, definition)) {
            self.restated.remove(&key);
            if refused > 0 {
                self.with_refusals.insert(key.clone());
            } else {
                self.with_refusals.remove(&key);
            }
            self.done.insert(key);
        } else {
            self.done.remove(&key);
            self.with_refusals.remove(&key);
            self.restated.insert(key);
        }
        Ok(())
    }

    /// Folds one whole line into the journal's sets, or `Err(())` for a line
    /// this journal does not write. O(fields).
    pub(super) fn read_line(&mut self, fields: &[&str]) -> Result<(), ()> {
        let key = |kind: &str, day: &str, only: &str| format!("{kind} {day} {only}");
        // A member key names its base key and its member (D-4878); `+` is
        // outside the symbol alphabet, so no filter name holds one.
        if let [_, kind, day, only, ..] = fields
            && let Some((base, member)) = only.split_once('+')
        {
            self.member_keys
                .entry(key(kind, day, base))
                .or_default()
                .insert(member.to_owned());
        }
        match fields {
            ["run", text, rest @ ..] => {
                let id: u64 = text.parse().map_err(|_| ())?;
                if id.to_string() != *text || rest.iter().any(|field| !field.contains('=')) {
                    return Err(());
                }
                self.runs.insert(id);
                // The definition the run's days are built under, and the run
                // they belong to, which its `begin` lines do not say (D-4878
                // item 10).
                self.run_definition = definition_of(rest)?;
                self.run_id = Some(id);
            }
            ["begin", kind, day, only] => {
                self.auction_line("begin", (*kind, *day, *only), (None, None, None));
                let key = key(kind, day, only);
                self.refused_close.remove(&key);
                self.open.insert(key);
            }
            ["done", kind, day, only, stats @ ..] => {
                self.read_done((kind, day, only), stats)?;
            }
            // Not done, whatever closed the key before: the last line wins
            // (D-3317). A `refused` day wrote nothing this time; each is
            // imported again and named in `resumed`.
            [
                verb @ ("incomplete" | "refused"),
                kind,
                day,
                only,
                stats @ ..,
            ] => {
                let built = definition_of(stats)?;
                let run = run_of(stats)?;
                self.auction_line(verb, (*kind, *day, *only), (built, run, scope_of(stats)));
                let key = key(kind, day, only);
                if *verb == "refused" {
                    let unrun: Vec<&str> = fields
                        .iter()
                        .copied()
                        .filter(|field| !field.starts_with("run="))
                        .collect();
                    self.refused_close.insert(key.clone(), unrun.join(" "));
                } else {
                    self.refused_close.remove(&key);
                }
                self.open.remove(&key);
                self.done.remove(&key);
                self.with_refusals.remove(&key);
                self.incomplete.insert(key);
            }
            // A census-refused day the census now counts (D-3316's recount):
            // done, as a `done` line under its definition is.
            ["recounted", kind, day, only, stats @ ..] => {
                let definition = definition_of(stats)?;
                count_of(stats, "months")?;
                let run = run_of(stats)?;
                self.auction_line(
                    "recounted",
                    (*kind, *day, *only),
                    (definition, run, scope_of(stats)),
                );
                let key = key(kind, day, only);
                self.refused_close.remove(&key);
                self.open.remove(&key);
                self.incomplete.remove(&key);
                self.with_refusals.remove(&key);
                if definition.is_some_and(|definition| current_for(kind, day, definition)) {
                    self.restated.remove(&key);
                    self.done.insert(key);
                } else {
                    self.done.remove(&key);
                    self.restated.insert(key);
                }
            }
            _ => return Err(()),
        }
        Ok(())
    }

    /// THE RUN'S ID (D-3317 item 4): its start, raised past every id a `run`
    /// line holds, so two runs started in one second are two runs. At most
    /// one step per `run` line in the journal.
    pub(super) fn fresh_run(&self, started: u64) -> u64 {
        let mut id = started;
        while self.runs.contains(&id) {
            id = id.saturating_add(1);
        }
        id
    }

    /// Appends `line` and a newline, durably, and answers the line's number
    /// in the file, from 1.
    pub(super) fn append(&self, line: &str) -> Result<u64, ImportRefusal> {
        self.write(Some(line))?;
        Ok(self.known.get().1)
    }

    /// Appends `line` and a newline, durably, after closing a torn last line
    /// with [`TORN`]. A write that failed part way earlier IN THIS RUN left
    /// such a line, and the run went on: written straight after it, the next
    /// line was glued to it, a foreign line every later load refused or a
    /// line that hid the one glued on (D-2826). So the file's last byte is
    /// read first, one seek and one byte, and a line that does not end in a
    /// newline is closed exactly as a load closes one. `None` only closes.
    pub(super) fn write(&self, line: Option<&str>) -> Result<(), ImportRefusal> {
        use std::io::{Read as _, Seek as _};
        let refused = |why: std::io::Error| ImportRefusal::Journal {
            why: format!("{}: {why}", self.path.display()),
        };
        // The journal always sits in `imports/` under the store root.
        std::fs::create_dir_all(self.path.with_file_name("")).map_err(refused)?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.path)
            .map_err(refused)?;
        let mut text = String::new();
        let at = file.metadata().map_err(refused)?.len();
        if at != self.known.get().0 {
            // Another process appended since this run last wrote: its lines
            // are counted once, so the next line's number is the file's.
            let mut bytes = Vec::new();
            file.seek(std::io::SeekFrom::Start(0)).map_err(refused)?;
            file.read_to_end(&mut bytes).map_err(refused)?;
            self.known.set((
                u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                newlines(&bytes),
            ));
        }
        if at > 0 {
            let mut last = [0_u8; 1];
            file.seek(std::io::SeekFrom::End(-1)).map_err(refused)?;
            file.read_exact(&mut last).map_err(refused)?;
            if last != *b"\n" {
                let shown = self.path.display().to_string();
                let _dropped_when_filtered = telemetry::emit(
                    &telemetry::Event::warn(TARGET, "closing a torn journal line")
                        .with("journal", shown.as_str()),
                );
                text.push_str(TORN);
                text.push('\n');
            }
        }
        if let Some(line) = line {
            text.push_str(line);
            text.push('\n');
        }
        if text.is_empty() {
            return Ok(());
        }
        file.write_all(text.as_bytes()).map_err(refused)?;
        file.sync_all().map_err(refused)?;
        let (bytes, lines) = self.known.get();
        self.known.set((
            bytes.saturating_add(u64::try_from(text.len()).unwrap_or(u64::MAX)),
            lines.saturating_add(newlines(text.as_bytes())),
        ));
        Ok(())
    }
}

/// The newlines in `bytes`: the whole lines of a journal.
pub(super) fn newlines(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0_u64, |lines, &b| {
        lines.saturating_add(u64::from(b == b'\n'))
    })
}

/// Appends `lines`, each with a newline, to the ledger at `path`, durably:
/// one write and one `fsync`, after closing a last line a crash left torn
/// with a bare newline (that fragment is then one line a reader counts as a
/// fault, never glued to the next). Creates the file and its directory.
/// Nothing to append opens nothing. Shared by the day, lineage and lot
/// ledgers (D-3335, D-3331, D-3343).
///
/// # Errors
///
/// The path and why.
pub(crate) fn append_lines(path: &Path, lines: &[String]) -> Result<(), String> {
    use std::io::{Read as _, Seek as _};
    if lines.is_empty() {
        return Ok(());
    }
    let refused = |why: std::io::Error| format!("{}: {why}", path.display());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(refused)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)
        .map_err(refused)?;
    let mut text = String::new();
    if file.metadata().map_err(refused)?.len() > 0 {
        let mut last = [0_u8; 1];
        file.seek(std::io::SeekFrom::End(-1)).map_err(refused)?;
        file.read_exact(&mut last).map_err(refused)?;
        if last != *b"\n" {
            text.push('\n');
        }
    }
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    file.write_all(text.as_bytes()).map_err(refused)?;
    file.sync_all().map_err(refused)
}

/// A day's closing line, all but its `run=` (D-3317 items 2 and 3). `scope`
/// is the ` scope=<digest>` a `done` or `incomplete` line of a recorded run
/// carries after its counts (D-4878), or nothing; a `refused` line carries
/// none.
pub(super) fn closing_line(key: &str, close: Close, counts: &Counts, scope: &str) -> String {
    match close {
        Close::Done => format!("done {key} {}{scope}", counts.text()),
        Close::WithRefusals(code) => format!(
            "done {key} {}{scope} vendor_refused={} reason={code}",
            counts.text(),
            counts.failures
        ),
        Close::Incomplete(cause) => format!(
            "incomplete {key} {}{scope} stage={} reason={}",
            counts.text(),
            cause.stage.as_str(),
            cause.code
        ),
        Close::Refused(cause) => format!(
            "refused {key} definition={BAR_DEFINITION} stage={} reason={} failures={}",
            cause.stage.as_str(),
            cause.code,
            counts.failures
        ),
    }
}

/// The run's `run` line (D-3317 item 4): its source as the caller measured
/// it, or `unrecorded` for every part a caller did not hand on, and last the
/// bar definition its days are built under (D-4878 item 10): a `begin` line
/// does not say it, and a day an earlier definition began and never closed
/// is told from one this build began only by it. Every build before
/// definition 4 wrote none, and a load reads a run line that names none as
/// one of those; each earlier build reads any `name=value` field of a run
/// line, so the field leaves the journal readable by them.
pub(super) fn run_line(id: u64, started: u64, origin: Option<&Origin>) -> String {
    let Some(origin) = origin else {
        return format!(
            "run {id} source=unrecorded archive=unrecorded archive_bytes=unrecorded \
             commit=unrecorded started={started} definition={BAR_DEFINITION}"
        );
    };
    format!(
        "run {id} source={} archive={} archive_bytes={} commit={} started={started} {}={} \
         definition={BAR_DEFINITION}",
        origin.source.as_str(),
        encoded(origin.path.as_os_str().as_encoded_bytes()),
        origin.archive_bytes,
        origin.commit_text(),
        origin.source.digest_field(),
        origin.digest_hex(),
    )
}

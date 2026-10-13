//! A DAY IS DONE ONLY FOR THE MEMBERS ITS CLOSE WAS ASKED TO READ (WP-12f,
//! D-4878; DB-B06, DB-B17, DB-B40, DB-B61, DB-B76).
//!
//! On 10 Oct 2026 a run under a scope of eight indices met 1,985 days closed
//! `done` under a scope of two and skipped every one; six index series were
//! left holding 12 special sessions each. These tests build that shape from
//! invented files: a store imported under the swept surface, then the same
//! days under the stored scope, and read back the bars, the journal's member
//! keys and the scope ledger beside a store that imported the wide scope in
//! order. They also hold the trade-for-trade spell (a day whose only file of
//! a share is its `.BE` file) to the share's own key.
//!
//! Every value is invented at run time; no vendor row is quoted
//! (`gdfl_cm::tests::fixtures_are_built_not_pasted` walks this file too).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::gdfl_cm::{CmScope, DayListing, ListedFile};
use crate::gdfl_fixtures::scratch;

/// The stored scope of D-3333, no option underlying named.
fn stored() -> Arc<CmScope> {
    Arc::new(CmScope::stored(std::iter::empty::<&str>()).unwrap())
}

/// One capital-market run over the tick store under `root`, under the swept
/// surface or under `scope`.
fn run_on(
    root: &Path,
    kind: ImportKind,
    (from, to): (Day, Day),
    store: &Path,
    scope: Option<Arc<CmScope>>,
) -> Result<Report, ImportRefusal> {
    let source = TickStore::new(&root.join("ts"));
    let source = match scope {
        Some(scope) => source.with_scope(scope),
        None => source,
    };
    let run = Run {
        kind,
        from,
        to,
        only: &[],
        store_root: store,
    };
    run_cm(&source, &run)
}

/// Whether the store holds the April 2024 one-second month of `symbol`,
/// the month every day of these worlds lies in.
fn held(store: &Path, segment: &str, symbol: &str) -> bool {
    store
        .join("bars/gdfl/NSE")
        .join(segment)
        .join(symbol)
        .join("1s/2024-04.bin")
        .exists()
}

/// The April 2024 one-second month of `symbol`, every bar of it.
fn month(store: &Path, segment: &str, symbol: &str) -> Vec<Bar> {
    bars_on(store, (segment, symbol, None), d1())
}

fn journal_text(store: &Path) -> String {
    std::fs::read_to_string(journal_path(store)).unwrap()
}

fn scopes_text(store: &Path) -> String {
    std::fs::read_to_string(scope::scopes_path(store)).unwrap_or_default()
}

/// How many lines of `text` start with `head`.
fn lines_starting(text: &str, head: &str) -> usize {
    text.lines().filter(|line| line.starts_with(head)).count()
}

/// A source that counts the day listings it is asked for.
struct Counting<'a> {
    inner: &'a TickStore,
    listed: AtomicUsize,
}

impl CmSource for Counting<'_> {
    type Locator = <TickStore as CmSource>::Locator;

    fn day(&self, kind: CmKind, day: Day) -> Result<Option<DayListing<Self::Locator>>, CmRefusal> {
        self.listed.fetch_add(1, Ordering::Relaxed);
        self.inner.day(kind, day)
    }

    fn scope(&self) -> Arc<CmScope> {
        self.inner.scope()
    }

    fn fetch(
        &self,
        listing: &DayListing<Self::Locator>,
        file: &ListedFile<Self::Locator>,
    ) -> Result<Vec<u8>, CmRefusal> {
        self.inner.fetch(listing, file)
    }
}

/// THE 10 OCT SHAPE, IN ORDER. Three days closed `done` under the swept
/// surface (two indices) are read again under the stored scope for the one
/// new member whose file they hold, each under a key of its own; the two old
/// members' months are not touched, the base keys are not closed again, and a
/// third run writes nothing.
#[test]
fn a_wider_scope_reads_a_done_day_again_for_its_new_members_only() {
    let root = scratch("wp12f-widen");
    world(&root, &[d1(), d2(), d3()]);
    let (store, whole) = (root.join("S"), root.join("whole"));
    let range = (d1(), d3());

    let narrow = run_on(&root, ImportKind::Indices, range, &store, None).unwrap();
    assert!(narrow.failures.is_empty(), "{:?}", narrow.failures);
    assert_eq!((narrow.days_imported, narrow.days_widened), (3, 0));
    let swept = scope::Record::of(CmScope::swept().members(CmKind::Indices));
    assert_eq!(
        swept.members(),
        ["NSE-INDEX-BANKNIFTY", "NSE-INDEX-NIFTY"],
        "the swept surface of the index tree"
    );
    let first = journal_text(&store);
    assert_eq!(
        lines_starting(&first, "done indices "),
        3,
        "three closes: {first}"
    );
    for line in first.lines().filter(|line| line.starts_with("done ")) {
        assert!(
            line.contains(&format!(" scope={} run=", swept.digest())),
            "every close names its scope: {line}"
        );
    }
    assert_eq!(
        scopes_text(&store),
        format!(
            "scope {} indices n=2 NSE-INDEX-BANKNIFTY,NSE-INDEX-NIFTY .\n",
            swept.digest()
        )
    );
    assert!(!held(&store, "INDEX", "INDIAVIX"));
    let before = tree(&store.join("bars"));

    let wide = run_on(&root, ImportKind::Indices, range, &store, Some(stored())).unwrap();
    assert!(wide.failures.is_empty(), "{:?}", wide.failures);
    assert_eq!(
        (
            wide.days_widened,
            wide.members_reopened,
            wide.days_skipped,
            wide.days_imported,
            wide.files
        ),
        (3, 3, 0, 3, 3),
        "each done day is read again for the one new member, and nothing else; a day so \
         read is a day imported, counted once"
    );
    let in_order = run_on(&root, ImportKind::Indices, range, &whole, Some(stored())).unwrap();
    assert!(in_order.failures.is_empty(), "{:?}", in_order.failures);
    let landed = month(&store, "INDEX", "INDIAVIX");
    assert!(!landed.is_empty(), "the new member's days landed");
    assert_eq!(
        landed,
        month(&whole, "INDEX", "INDIAVIX"),
        "exactly what an in-order wide import holds"
    );
    let after = tree(&store.join("bars"));
    for (path, bytes) in &before {
        assert_eq!(
            after.get(path),
            Some(bytes),
            "{}: an old member's file is not touched",
            path.display()
        );
    }
    let second = journal_text(&store);
    assert!(second.starts_with(&first), "the journal is appended to");
    assert_eq!(
        lines_starting(&second, "done indices "),
        6,
        "three member closes beside the three base closes: {second}"
    );
    for on in ["2024-04-01", "2024-04-02", "2024-04-03"] {
        assert_eq!(
            lines_starting(
                &second,
                &format!("done indices {on} *+NSE-INDEX-INDIAVIX definition=")
            ),
            1,
            "{on}: {second}"
        );
        assert_eq!(
            lines_starting(&second, &format!("done indices {on} * ")),
            1,
            "{on}: the base key is not closed again"
        );
    }
    let ledger = scopes_text(&store);
    assert_eq!(
        lines_starting(&ledger, "examined indices "),
        3,
        "each day examined once: {ledger}"
    );

    let again = run_on(&root, ImportKind::Indices, range, &store, Some(stored())).unwrap();
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    assert_eq!(
        (again.days_skipped, again.days_widened, again.files),
        (3, 0, 0)
    );
    assert_eq!(journal_text(&store), second, "a re-run writes no line");
    assert_eq!(scopes_text(&store), ledger, "nor a ledger line");
    assert_eq!(tree(&store.join("bars")), after, "nor a byte of a bar");
    std::fs::remove_dir_all(root).unwrap();
}

/// A JOURNAL FROM BEFORE SCOPES WERE RECORDED, as the real one is: its
/// `done` lines carry no `scope=` and no scope ledger exists. Each is read as
/// built under the swept surface, so the new member is read and the two old
/// ones are not; the old lines stay byte for byte.
#[test]
fn a_close_with_no_scope_is_read_as_the_swept_surface() {
    let root = scratch("wp12f-legacy");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    let range = (d1(), d2());
    let narrow = run_on(&root, ImportKind::Indices, range, &store, None).unwrap();
    assert!(narrow.failures.is_empty(), "{:?}", narrow.failures);
    let mut legacy = String::new();
    for line in journal_text(&store).lines() {
        let kept: Vec<&str> = line
            .split(' ')
            .filter(|field| !field.starts_with("scope="))
            .collect();
        legacy.push_str(&kept.join(" "));
        legacy.push('\n');
    }
    assert!(!legacy.contains("scope="));
    std::fs::write(journal_path(&store), &legacy).unwrap();
    std::fs::remove_file(scope::scopes_path(&store)).unwrap();

    let wide = run_on(&root, ImportKind::Indices, range, &store, Some(stored())).unwrap();
    assert!(wide.failures.is_empty(), "{:?}", wide.failures);
    assert_eq!(
        (wide.days_widened, wide.members_reopened, wide.files),
        (2, 2, 2),
        "one file a day: the new member's, never NIFTY's or BANKNIFTY's again"
    );
    assert!(!month(&store, "INDEX", "INDIAVIX").is_empty());
    assert!(
        journal_text(&store).starts_with(&legacy),
        "the old lines are not rewritten"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// THE DAMAGE ITSELF: the new member's month holds only a LATER day, as the
/// six alias indices hold only a special session in 67 months. The earlier
/// days, done under the narrow scope, are read again and land through the
/// versioned month repair (D-3350), and the month ends holding what an
/// in-order import holds.
///
/// The shape is built later day first: no run of this build makes it from
/// days already done (`a_run_starting_past_owed_days_of_its_month_reads_them_first`),
/// but the real store holds it from the run of 10 Oct 2026.
#[test]
fn a_new_member_holding_only_a_later_day_takes_its_earlier_done_days() {
    let root = scratch("wp12f-sparse");
    world(&root, &[d1(), d2(), d3()]);
    let (store, whole) = (root.join("S"), root.join("whole"));
    let late = run_on(
        &root,
        ImportKind::Indices,
        (d3(), d3()),
        &store,
        Some(stored()),
    )
    .unwrap();
    assert!(late.failures.is_empty(), "{:?}", late.failures);
    let narrow = run_on(&root, ImportKind::Indices, (d1(), d2()), &store, None).unwrap();
    assert!(narrow.failures.is_empty(), "{:?}", narrow.failures);
    let only_late = month(&store, "INDEX", "INDIAVIX");
    assert!(!only_late.is_empty(), "the month holds the later day alone");

    let range = (d1(), d3());
    let wide = run_on(&root, ImportKind::Indices, range, &store, Some(stored())).unwrap();
    assert!(wide.failures.is_empty(), "{:?}", wide.failures);
    assert_eq!(
        (wide.days_widened, wide.members_reopened, wide.days_skipped),
        (2, 2, 1),
        "the two narrow days are read again; the wide day is done"
    );
    assert_eq!(
        wide.months_repaired, 2,
        "each landed as a generation of the month"
    );
    let in_order = run_on(&root, ImportKind::Indices, range, &whole, Some(stored())).unwrap();
    assert!(in_order.failures.is_empty(), "{:?}", in_order.failures);
    let full = month(&whole, "INDEX", "INDIAVIX");
    assert_eq!(month(&store, "INDEX", "INDIAVIX"), full);
    assert_eq!(
        full.len(),
        only_late.len() * 3,
        "three like days, where the month held one"
    );
    assert!(
        full.ends_with(&only_late),
        "the later day is still the last"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A DONE DAY WITH NO FILE OF ANY NEW MEMBER is examined once and
/// remembered: the next run asks the source for no listing of it.
#[test]
fn a_day_with_no_file_of_a_new_member_is_examined_once_and_never_listed_again() {
    let root = scratch("wp12f-examined");
    let on = d1();
    let two = |d: Day| {
        vec![
            (
                entry_of(Tree::Indices, d, "NIFTY 50.NSE_IDX"),
                index_file("NIFTY 50.NSE_IDX", d, 22_000),
            ),
            (
                entry_of(Tree::Indices, d, "NIFTY BANK.NSE_IDX"),
                index_file("NIFTY BANK.NSE_IDX", d, 47_000),
            ),
        ]
    };
    world_of(&root, &[(on, Tree::Indices, two(on))]);
    let store = root.join("S");
    let narrow = run_on(&root, ImportKind::Indices, (on, on), &store, None).unwrap();
    assert!(narrow.failures.is_empty(), "{:?}", narrow.failures);

    let inner = TickStore::new(&root.join("ts")).with_scope(stored());
    let run = Run {
        kind: ImportKind::Indices,
        from: on,
        to: on,
        only: &[],
        store_root: &store,
    };
    let counting = Counting {
        inner: &inner,
        listed: AtomicUsize::new(0),
    };
    let first = run_cm(&counting, &run).unwrap();
    assert!(first.failures.is_empty(), "{:?}", first.failures);
    assert_eq!(
        (first.days_widened, first.members_reopened, first.files),
        (1, 0, 0)
    );
    assert_eq!(
        counting.listed.load(Ordering::Relaxed),
        1,
        "listed once to look"
    );
    let ledger = scopes_text(&store);
    assert_eq!(
        lines_starting(&ledger, &format!("examined indices {on} * scope=")),
        1,
        "{ledger}"
    );
    assert!(
        ledger.trim_end().ends_with(" opened=0 ."),
        "no member key was opened: {ledger}"
    );
    let journal = journal_text(&store);

    let second = run_cm(&counting, &run).unwrap();
    assert_eq!((second.days_skipped, second.days_widened), (1, 0));
    assert_eq!(
        counting.listed.load(Ordering::Relaxed),
        1,
        "the examined day is not listed again"
    );
    assert_eq!(scopes_text(&store), ledger);
    assert_eq!(journal_text(&store), journal);
    std::fs::remove_dir_all(root).unwrap();
}

/// A FILTERED KEY READS AGAIN ONLY WHAT ITS FILTER NAMES. A day done for
/// `NIFTY` alone under the swept surface owes the stored scope's new members
/// nothing its filter asks for: no member key is opened, no file is read, the
/// day is examined once, and the unfiltered key of the same day is another
/// key, imported whole.
#[test]
fn a_filtered_key_opens_no_member_its_filter_does_not_name() {
    let root = scratch("wp12f-filtered");
    world(&root, &[d1()]);
    let store = root.join("S");
    let only = ["NIFTY".to_owned()];
    let filtered = |scope: Option<Arc<CmScope>>| {
        let source = TickStore::new(&root.join("ts"));
        let source = match scope {
            Some(scope) => source.with_scope(scope),
            None => source,
        };
        let run = Run {
            kind: ImportKind::Indices,
            from: d1(),
            to: d1(),
            only: &only,
            store_root: &store,
        };
        run_cm(&source, &run).unwrap()
    };
    let narrow = filtered(None);
    assert!(narrow.failures.is_empty(), "{:?}", narrow.failures);
    assert_eq!(narrow.files, 1);
    let journal = journal_text(&store);

    let wide = filtered(Some(stored()));
    assert!(wide.failures.is_empty(), "{:?}", wide.failures);
    assert_eq!(
        (wide.days_widened, wide.members_reopened, wide.files),
        (1, 0, 0),
        "the new members are not the filter's"
    );
    // NO MEMBER KEY, NO DAY LINE: the journal gains the run's own `run` line
    // and nothing else, and the id the scope ledger names is that line's, so
    // no id is written that the journal does not hold.
    let now = journal_text(&store);
    let added: Vec<&str> = now
        .strip_prefix(journal.as_str())
        .unwrap()
        .lines()
        .collect();
    assert_eq!(added.len(), 1, "{added:?}");
    let id = added[0]
        .strip_prefix("run ")
        .and_then(|rest| rest.split(' ').next())
        .unwrap();
    assert!(!held(&store, "INDEX", "INDIAVIX"));
    let ledger = scopes_text(&store);
    assert!(
        ledger
            .trim_end()
            .ends_with(&format!(" run={id} opened=0 .")),
        "examined, remembered, and by a run the journal holds: {ledger}"
    );
    let again = filtered(Some(stored()));
    assert_eq!((again.days_skipped, again.days_widened), (1, 0));
    assert_eq!(
        journal_text(&store),
        now,
        "a run that examines nothing writes nothing"
    );

    let whole = run_on(
        &root,
        ImportKind::Indices,
        (d1(), d1()),
        &store,
        Some(stored()),
    )
    .unwrap();
    assert!(whole.failures.is_empty(), "{:?}", whole.failures);
    assert_eq!((whole.days_imported, whole.files), (1, 3));
    assert!(held(&store, "INDEX", "INDIAVIX"));
    std::fs::remove_dir_all(root).unwrap();
}

/// THE LEDGER KILLED MID-WRITE, AND A FOREIGN ONE. A fragment with no line
/// end is stepped over and the run goes on; a whole line this module does not
/// write, or a member list that is not the one its digest names, refuses the
/// run by name before a byte is written.
#[test]
fn a_torn_ledger_line_is_stepped_over_and_a_foreign_one_refuses_the_run() {
    let root = scratch("wp12f-torn");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    let narrow = run_on(&root, ImportKind::Indices, (d1(), d1()), &store, None).unwrap();
    assert!(narrow.failures.is_empty(), "{:?}", narrow.failures);
    let path = scope::scopes_path(&store);
    let whole = scopes_text(&store);

    std::fs::write(&path, format!("{whole}examined indices 2024-04-0")).unwrap();
    let loaded = scope::Ledger::load(path.clone()).unwrap();
    assert_eq!(loaded.torn(), 1);
    let _earlier = summaries_taken();
    let wide = run_on(
        &root,
        ImportKind::Indices,
        (d1(), d2()),
        &store,
        Some(stored()),
    )
    .unwrap();
    assert!(wide.failures.is_empty(), "{:?}", wide.failures);
    // The fragment is said, once, at `warn`, by the run that met it.
    let said: Vec<Summary> = summaries_taken()
        .into_iter()
        .filter(|one| one.text.get("code").map(String::as_str) == Some("scope_ledger_torn"))
        .collect();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].warn);
    assert_eq!(
        (wide.days_widened, wide.days_imported),
        (1, 2),
        "the first day for its new member, the second whole"
    );
    let reread = scope::Ledger::load(path.clone()).unwrap();
    assert_eq!(reread.torn(), 1, "the fragment stays a fragment");
    assert!(
        reread
            .examined_under(&format!("indices {} *", d1()))
            .is_some()
    );

    let (journal, bars) = (journal_text(&store), tree(&store.join("bars")));
    let good = scopes_text(&store);
    for foreign in [
        "hello world .\n".to_owned(),
        "scope 0123456789abcdef indices n=1 NSE-INDEX-NIFTY .\n".to_owned(),
        "examined indices 2024-04-01 * scope=abc run=one opened=0 .\n".to_owned(),
    ] {
        std::fs::write(&path, format!("{good}{foreign}")).unwrap();
        let refused = run_on(
            &root,
            ImportKind::Indices,
            (d1(), d2()),
            &store,
            Some(stored()),
        )
        .unwrap_err();
        let said = refused.to_string();
        assert!(
            said.contains("gdfl.scopes") && said.contains("not a scope ledger line"),
            "{foreign:?}: {said}"
        );
        assert_eq!(journal_text(&store), journal, "{foreign:?}");
        assert_eq!(tree(&store.join("bars")), bars, "{foreign:?}");
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// A MEMBER KEY'S CLOSE IS THE MEMBER'S EVENT, AND THE DAY IS TOLD ONCE
/// (WP-12f repair 1). The import's pace is measured from the gaps between
/// "day imported" stamps, so a member key that told its close as "day
/// imported" made one day of N new members read as N days, and no event said
/// which member had closed. One day, two new members: two "member imported"
/// events, each naming its member and its journal key, then ONE "day
/// imported" for the day, and one day in `days_imported`.
#[test]
fn a_member_keys_close_is_told_as_the_members_and_the_day_is_told_once() {
    let root = scratch("wp12f-told");
    let on = d1();
    let file = |stem: &str, base: u32| {
        (
            entry_of(Tree::Indices, on, stem),
            index_file(stem, on, base),
        )
    };
    world_of(
        &root,
        &[(
            on,
            Tree::Indices,
            vec![
                file("NIFTY 50.NSE_IDX", 22_000),
                file("NIFTY BANK.NSE_IDX", 47_000),
                file("INDIA VIX.NSE_IDX", 13),
                file("NIFTY IT.NSE_IDX", 35_000),
            ],
        )],
    );
    let store = root.join("S");
    let _earlier = summaries_taken();

    let narrow = run_on(&root, ImportKind::Indices, (on, on), &store, None).unwrap();
    assert!(narrow.failures.is_empty(), "{:?}", narrow.failures);
    let told = summaries_taken();
    assert_eq!(told.len(), 1, "a day read whole tells one event: {told:?}");
    assert_eq!(told[0].words, "day imported");
    assert!(!told[0].text.contains_key("member"), "{told:?}");

    let wide = run_on(&root, ImportKind::Indices, (on, on), &store, Some(stored())).unwrap();
    assert!(wide.failures.is_empty(), "{:?}", wide.failures);
    assert_eq!(
        (
            wide.days_widened,
            wide.members_reopened,
            wide.days_imported,
            wide.files
        ),
        (1, 2, 1, 2),
        "one day, two members: the day is counted once"
    );
    let told = summaries_taken();
    let journal = journal_text(&store);
    let members: Vec<&Summary> = told
        .iter()
        .filter(|event| event.words == "member imported")
        .collect();
    assert_eq!(
        members
            .iter()
            .map(|event| event.text["member"].as_str())
            .collect::<Vec<_>>(),
        ["NSE-INDEX-INDIAVIX", "NSE-INDEX-NIFTYIT"],
        "each member key tells its own close, by name: {told:?}"
    );
    for event in &members {
        let key = &event.text["key"];
        assert_eq!(
            key,
            &format!("indices {on} *+{}", event.text["member"]),
            "the journal key of the member"
        );
        assert_eq!(
            lines_starting(&journal, &format!("done {key} ")),
            1,
            "the event's key is the closing line's: {journal}"
        );
        assert_eq!(event.text["day"], on.to_string());
        assert!(!event.warn);
    }
    let days: Vec<&Summary> = told
        .iter()
        .filter(|event| event.words == "day imported")
        .collect();
    assert_eq!(days.len(), 1, "ONE day event for the day: {told:?}");
    assert!(!days[0].text.contains_key("member"));
    assert_eq!(
        told.last().map(|event| event.words.as_str()),
        Some("day imported"),
        "after its last member key"
    );
    assert_eq!(told.len(), 3, "and nothing else: {told:?}");

    // A re-run tells nothing: nothing closed.
    let again = run_on(&root, ImportKind::Indices, (on, on), &store, Some(stored())).unwrap();
    assert_eq!((again.days_skipped, again.days_imported), (1, 0));
    assert_eq!(summaries_taken(), []);
    std::fs::remove_dir_all(root).unwrap();
}

/// A NEW MEMBER'S MONTH IS NEVER MADE FROM A LATER DAY WHILE AN EARLIER DAY OF
/// IT IS OWED (WP-12f repair 1; DB-B76's "a new member is never created from
/// special days alone"). On 10 Oct 2026 six index series were made from the
/// 12 special sessions while every regular day before them was closed `done`
/// for two indices; every later landing in such a month is a new full copy
/// of it. A run whose range starts past done days of its first month reads
/// those days first for the members they owe, in order: the month is made
/// from its first owed day, nothing is repaired, and it holds what an
/// in-order import holds.
#[test]
fn a_run_starting_past_owed_days_of_its_month_reads_them_first() {
    let root = scratch("wp12f-reach");
    world(&root, &[d1(), d2(), d3()]);
    let (store, whole) = (root.join("S"), root.join("whole"));
    let narrow = run_on(&root, ImportKind::Indices, (d1(), d2()), &store, None).unwrap();
    assert!(narrow.failures.is_empty(), "{:?}", narrow.failures);
    let before = month(&store, "INDEX", "NIFTY");

    let late = run_on(
        &root,
        ImportKind::Indices,
        (d3(), d3()),
        &store,
        Some(stored()),
    )
    .unwrap();
    assert!(late.failures.is_empty(), "{:?}", late.failures);
    assert_eq!(
        (
            late.days_before_range,
            late.days_widened,
            late.members_reopened,
            late.days_imported,
            late.months_repaired
        ),
        (2, 2, 2, 3, 0),
        "the two owed days first, then the day asked for; no month is repaired"
    );
    let in_order = run_on(
        &root,
        ImportKind::Indices,
        (d1(), d3()),
        &whole,
        Some(stored()),
    )
    .unwrap();
    assert!(in_order.failures.is_empty(), "{:?}", in_order.failures);
    for symbol in ["INDIAVIX", "NIFTY", "BANKNIFTY"] {
        assert_eq!(
            month(&store, "INDEX", symbol),
            month(&whole, "INDEX", symbol),
            "{symbol}: exactly what an in-order import holds"
        );
    }
    let after = tree(&store.join("bars"));
    assert!(
        !after
            .keys()
            .any(|path| path.to_string_lossy().contains("bar-revisions")),
        "no generation of any month: {:?}",
        after.keys().collect::<Vec<_>>()
    );
    let nifty = month(&store, "INDEX", "NIFTY");
    assert!(
        nifty.len() > before.len() && nifty.starts_with(&before),
        "an old member's month is only appended to"
    );
    // The owed days' member keys are closed BEFORE the asked day's line.
    let journal = journal_text(&store);
    let at = |head: &str| {
        journal
            .find(head)
            .unwrap_or_else(|| panic!("{head}: {journal}"))
    };
    let asked = at(&format!("done indices {} * ", d3()));
    for owed in [d1(), d2()] {
        assert!(
            at(&format!("done indices {owed} *+NSE-INDEX-INDIAVIX ")) < asked,
            "{owed} lands first: {journal}"
        );
    }
    let ledger = scopes_text(&store);

    // A second run reaches back for nothing and writes nothing.
    let again = run_on(
        &root,
        ImportKind::Indices,
        (d3(), d3()),
        &store,
        Some(stored()),
    )
    .unwrap();
    assert_eq!(
        (
            again.days_before_range,
            again.days_widened,
            again.days_skipped,
            again.files
        ),
        (0, 0, 1, 0)
    );
    assert_eq!(journal_text(&store), journal);
    assert_eq!(scopes_text(&store), ledger);

    // A DAY BEFORE THE RANGE THAT NO RUN CLOSED IS NOT THE RUN'S: only a done
    // day owes anything, so a store that never read the earlier days reads
    // the asked day alone and writes no line of another.
    let fresh = root.join("fresh");
    let alone = run_on(
        &root,
        ImportKind::Indices,
        (d3(), d3()),
        &fresh,
        Some(stored()),
    )
    .unwrap();
    assert_eq!(
        (
            alone.days_before_range,
            alone.days_widened,
            alone.days_imported
        ),
        (0, 0, 1)
    );
    let fresh_journal = journal_text(&fresh);
    for other in [d1(), d2()] {
        assert!(
            !fresh_journal.contains(&format!(" {other} ")),
            "{other}: {fresh_journal}"
        );
    }

    // The reach is the range's own month and never past it: at most 30 days.
    assert_eq!(before_range(d3()), [d1(), d2()]);
    assert_eq!(before_range(d1()), []);
    let march = before_range(Day::new(2024, 3, 31).unwrap());
    assert_eq!(march.len(), 30);
    assert_eq!(
        (march.first().copied(), march.last().copied()),
        (
            Some(Day::new(2024, 3, 1).unwrap()),
            Some(Day::new(2024, 3, 30).unwrap())
        )
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Three invented index days of four files each: the two swept indices and
/// two the stored scope adds.
fn four_index_days(root: &Path) {
    let plan: Vec<DayPlan> = [d1(), d2(), d3()]
        .into_iter()
        .map(|on| {
            let file = |stem: &str, base: u32| {
                (
                    entry_of(Tree::Indices, on, stem),
                    index_file(stem, on, base),
                )
            };
            (
                on,
                Tree::Indices,
                vec![
                    file("NIFTY 50.NSE_IDX", 22_000),
                    file("NIFTY BANK.NSE_IDX", 47_000),
                    file("INDIA VIX.NSE_IDX", 13),
                    file("NIFTY IT.NSE_IDX", 35_000),
                ],
            )
        })
        .collect();
    world_of(root, &plan);
}

/// One indices run under a filter, and how many day listings it asked for.
fn filtered_on(
    root: &Path,
    (from, to): (Day, Day),
    store: &Path,
    only: &[String],
    scope: Option<Arc<CmScope>>,
) -> (Report, usize) {
    let source = TickStore::new(&root.join("ts"));
    let source = match scope {
        Some(scope) => source.with_scope(scope),
        None => source,
    };
    let counting = Counting {
        inner: &source,
        listed: AtomicUsize::new(0),
    };
    let run = Run {
        kind: ImportKind::Indices,
        from,
        to,
        only,
        store_root: store,
    };
    let report = run_cm(&counting, &run).unwrap();
    (report, counting.listed.load(Ordering::Relaxed))
}

/// The journal with every `scope=` taken out and no scope ledger: the lines
/// every build before scopes were recorded wrote.
fn as_before_scopes(store: &Path) {
    let mut legacy = String::new();
    for line in journal_text(store).lines() {
        let kept: Vec<&str> = line
            .split(' ')
            .filter(|field| !field.starts_with("scope="))
            .collect();
        legacy.push_str(&kept.join(" "));
        legacy.push('\n');
    }
    std::fs::write(journal_path(store), &legacy).unwrap();
    std::fs::remove_file(scope::scopes_path(store)).unwrap();
}

/// The summary events of this thread that carry `code`.
fn told_with(told: &[Summary], code: &str) -> Vec<Summary> {
    told.iter()
        .filter(|one| one.text.get("code").map(String::as_str) == Some(code))
        .cloned()
        .collect()
}

/// A RUN UNDER A FILTER ASKS EVERY KEY THAT CLOSED AN EARLIER DAY, THE
/// WHOLE-DAY KEY AMONG THEM (WP-12f repair 2; DB-B76 clause 2). Two days are
/// closed `done` under `*` for the swept surface. A run of the third day
/// alone under a filter (one named symbol; and the names the stored scope
/// adds, which is `cli`'s `--stored`) used to ask only its own key, find
/// nothing closed, and make each new member's month from the third day
/// alone, with no warning: the 10 Oct shape. It now reads the two owed days
/// first for the members its filter wants, under the whole-day key's member
/// keys, says so at `warn`, and repairs nothing. What it remembers is its own
/// (`*/<filter>`), so the unfiltered run that follows still reads the
/// members the filter did not name.
#[test]
fn a_filtered_run_reads_first_what_the_whole_day_key_owes_it() {
    for (name, only, legacy) in [
        ("one", vec!["INDIAVIX".to_owned()], false),
        (
            "stored",
            vec!["INDIAVIX".to_owned(), "NIFTYIT".to_owned()],
            true,
        ),
    ] {
        let root = scratch(&format!("wp12f-lent-{name}"));
        four_index_days(&root);
        let (store, whole) = (root.join("S"), root.join("whole"));
        let narrow = run_on(&root, ImportKind::Indices, (d1(), d2()), &store, None).unwrap();
        assert!(narrow.failures.is_empty(), "{:?}", narrow.failures);
        assert_eq!(narrow.files, 4, "two days of the two swept indices");
        if legacy {
            as_before_scopes(&store);
        }
        let filter = only.join(",");
        let wanted = only.len();
        let _earlier = summaries_taken();

        let (late, _) = filtered_on(&root, (d3(), d3()), &store, &only, Some(stored()));
        assert!(late.failures.is_empty(), "{name}: {:?}", late.failures);
        assert_eq!(
            (
                late.days_before_range,
                late.days_widened,
                late.members_reopened,
                late.days_imported,
                late.files,
                late.months_repaired
            ),
            (2, 2, 2 * wanted, 3, 3 * wanted, 0),
            "{name}: the two owed days first, then the day asked for; nothing repaired"
        );
        let told = summaries_taken();
        let reach = told_with(&told, "scope_reach_back");
        assert_eq!(reach.len(), 1, "{name}: said once: {told:?}");
        assert!(reach[0].warn, "{name}");
        assert_eq!(reach[0].text["first_owed"], d1().to_string(), "{name}");
        assert_eq!(reach[0].text["key"], format!("indices {} *", d1()));
        assert_eq!(
            told.iter()
                .filter(|one| one.words == "day imported")
                .count(),
            3,
            "{name}: each day told once: {told:?}"
        );

        // The owed days' members are closed under the WHOLE-DAY key's member
        // keys, before the asked day's line.
        let journal = journal_text(&store);
        let at = |head: &str| {
            journal
                .find(head)
                .unwrap_or_else(|| panic!("{name}: {head}: {journal}"))
        };
        let asked = at(&format!("done indices {} {filter} ", d3()));
        for owed in [d1(), d2()] {
            assert!(
                at(&format!("done indices {owed} *+NSE-INDEX-INDIAVIX ")) < asked,
                "{name}: {owed} lands first: {journal}"
            );
            assert!(
                !journal.contains(&format!("indices {owed} {filter}")),
                "{name}: no key of the filter's own on a day it was not asked for"
            );
        }
        // What it remembers is its own, never the whole-day key's.
        let ledger = scopes_text(&store);
        for owed in [d1(), d2()] {
            assert_eq!(
                lines_starting(
                    &ledger,
                    &format!("examined indices {owed} */{filter} scope=")
                ),
                1,
                "{name}: {ledger}"
            );
            assert_eq!(
                lines_starting(&ledger, &format!("examined indices {owed} * ")),
                0,
                "{name}: {ledger}"
            );
        }

        // Exactly what an in-order import holds, and no generation of a month.
        let in_order = run_on(
            &root,
            ImportKind::Indices,
            (d1(), d3()),
            &whole,
            Some(stored()),
        )
        .unwrap();
        assert!(in_order.failures.is_empty(), "{:?}", in_order.failures);
        for symbol in &only {
            assert_eq!(
                month(&store, "INDEX", symbol),
                month(&whole, "INDEX", symbol),
                "{name}: {symbol}"
            );
        }
        assert!(
            !tree(&store.join("bars"))
                .keys()
                .any(|path| path.to_string_lossy().contains("bar-revisions")),
            "{name}: no month was repaired"
        );

        // A second run under the same filter lists no day and writes nothing.
        let _told = summaries_taken();
        let (again, listed) = filtered_on(&root, (d3(), d3()), &store, &only, Some(stored()));
        assert_eq!(
            (
                again.days_before_range,
                again.days_widened,
                again.days_skipped,
                again.files,
                listed
            ),
            (0, 0, 1, 0, 0),
            "{name}"
        );
        assert_eq!(journal_text(&store), journal, "{name}");
        assert_eq!(scopes_text(&store), ledger, "{name}");
        assert_eq!(summaries_taken(), [], "{name}");

        // THE UNFILTERED RUN THAT FOLLOWS reads what the filter did not name
        // and nothing the filter read: in order, so nothing is repaired.
        let rest = run_on(
            &root,
            ImportKind::Indices,
            (d1(), d3()),
            &store,
            Some(stored()),
        )
        .unwrap();
        assert!(rest.failures.is_empty(), "{name}: {:?}", rest.failures);
        assert_eq!(
            (rest.members_reopened, rest.months_repaired),
            (2 * (2 - wanted), 0),
            "{name}: the members the filter left, once each"
        );
        for symbol in ["NIFTY", "BANKNIFTY", "INDIAVIX", "NIFTYIT"] {
            assert_eq!(
                month(&store, "INDEX", symbol),
                month(&whole, "INDEX", symbol),
                "{name}: {symbol} after the unfiltered run"
            );
        }
        let ledger = scopes_text(&store);
        assert_eq!(
            lines_starting(&ledger, &format!("examined indices {} * scope=", d1())),
            1,
            "{name}: the whole-day key is examined by its own run: {ledger}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// A KEY CLOSED UNDER NAMES OWES ONLY WHAT IT NAMED, AND A DAY TWO KEYS
/// CLOSED IS READ AND TOLD ONCE (WP-12f repair 2). Two days are closed under
/// the filter `INDIAVIX,NIFTY` while the scope filed NIFTY alone, so that
/// close owes INDIAVIX and never NIFTYIT, which it did not name:
///
/// - a run of the third day under `NIFTYIT` lists the two days once, reads
///   nothing of them, says nothing at `warn`, and remembers it;
/// - a run of the third day under `INDIAVIX` reads the two days first, under
///   that key's member keys;
/// - with the first day ALSO closed under `*`, both keys owe INDIAVIX: it is
///   read once, under the first key asked, and the day is told once.
#[test]
fn a_key_closed_under_names_owes_only_what_it_named() {
    let root = scratch("wp12f-named");
    four_index_days(&root);
    let lender = ["INDIAVIX".to_owned(), "NIFTY".to_owned()];
    let close_two = |store: &Path| {
        let (narrow, _) = filtered_on(&root, (d1(), d2()), store, &lender, None);
        assert!(narrow.failures.is_empty(), "{:?}", narrow.failures);
        assert_eq!(narrow.files, 2, "NIFTY alone: the scope files no INDIAVIX");
    };

    // NIFTYIT: not named by the key that closed the days, so not owed.
    let store = root.join("S");
    close_two(&store);
    let other = ["NIFTYIT".to_owned()];
    let _earlier = summaries_taken();
    let (alone, listed) = filtered_on(&root, (d3(), d3()), &store, &other, Some(stored()));
    assert!(alone.failures.is_empty(), "{:?}", alone.failures);
    assert_eq!(
        (
            alone.days_before_range,
            alone.days_widened,
            alone.members_reopened,
            alone.days_imported,
            alone.files,
            listed
        ),
        (0, 2, 0, 1, 1, 3),
        "two days listed and nothing of them read"
    );
    assert_eq!(told_with(&summaries_taken(), "scope_reach_back"), []);
    let journal = journal_text(&store);
    assert!(!journal.contains("+NSE-INDEX-NIFTYIT"), "{journal}");
    let ledger = scopes_text(&store);
    assert_eq!(
        lines_starting(
            &ledger,
            &format!("examined indices {} INDIAVIX,NIFTY/NIFTYIT scope=", d1())
        ),
        1,
        "{ledger}"
    );
    assert!(ledger.trim_end().ends_with(" opened=0 ."), "{ledger}");
    let (again, listed) = filtered_on(&root, (d3(), d3()), &store, &other, Some(stored()));
    assert_eq!((again.days_widened, listed), (0, 0), "remembered");

    // INDIAVIX: named and never covered, so owed and read first.
    let vix = ["INDIAVIX".to_owned()];
    let (late, _) = filtered_on(&root, (d3(), d3()), &store, &vix, Some(stored()));
    assert!(late.failures.is_empty(), "{:?}", late.failures);
    assert_eq!(
        (
            late.days_before_range,
            late.members_reopened,
            late.days_imported,
            late.months_repaired
        ),
        (2, 2, 3, 0)
    );
    let journal = journal_text(&store);
    for owed in [d1(), d2()] {
        assert!(
            journal.contains(&format!(
                "done indices {owed} INDIAVIX,NIFTY+NSE-INDEX-INDIAVIX "
            )),
            "{owed}: {journal}"
        );
    }

    // TWO KEYS CLOSED THE FIRST DAY and both owe INDIAVIX: read once, under
    // the first key asked (`*` sorts first), and the day is told once.
    let both = root.join("both");
    close_two(&both);
    let star = run_on(&root, ImportKind::Indices, (d1(), d1()), &both, None).unwrap();
    assert!(star.failures.is_empty(), "{:?}", star.failures);
    let _earlier = summaries_taken();
    let (late, _) = filtered_on(&root, (d2(), d2()), &both, &vix, Some(stored()));
    assert!(late.failures.is_empty(), "{:?}", late.failures);
    assert_eq!(
        (
            late.days_before_range,
            late.days_widened,
            late.members_reopened,
            late.days_imported,
            late.files
        ),
        (1, 2, 1, 2, 2),
        "one day before the range, two keys asked, one member read once"
    );
    let told = summaries_taken();
    assert_eq!(
        told.iter()
            .filter(|one| one.words == "day imported")
            .count(),
        2,
        "the first day once, the asked day once: {told:?}"
    );
    let journal = journal_text(&both);
    assert!(
        journal.contains(&format!("done indices {} *+NSE-INDEX-INDIAVIX ", d1())),
        "{journal}"
    );
    assert!(
        !journal.contains(&format!("indices {} INDIAVIX,NIFTY+", d1())),
        "not read a second time under the other key: {journal}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// `retry_refused` READS AGAIN A MEMBER KEY CLOSED DONE WITH REFUSALS, as it
/// does a base key (D-3317): without the flag the member key is done and
/// skipped; with it the member's file is read again, nothing is written
/// twice, and the key closes again by what the reader says now. The closing
/// line with refusals is written here as the importer writes one, over a
/// member key the wide run closed.
#[test]
fn retry_refused_reads_again_a_member_key_closed_with_refusals() {
    let root = scratch("wp12f-retry");
    world(&root, &[d1()]);
    let store = root.join("S");
    let on = (d1(), d1());
    let narrow = run_on(&root, ImportKind::Indices, on, &store, None).unwrap();
    assert!(narrow.failures.is_empty(), "{:?}", narrow.failures);
    let wide = run_on(&root, ImportKind::Indices, on, &store, Some(stored())).unwrap();
    assert!(wide.failures.is_empty(), "{:?}", wide.failures);
    let journal = journal_text(&store);
    let head = format!("done indices {} *+NSE-INDEX-INDIAVIX ", d1());
    let clean = journal
        .lines()
        .find(|line| line.starts_with(&head))
        .unwrap();
    assert!(clean.contains(" failures=0 "), "{clean}");
    let (stats, run) = clean.rsplit_once(" run=").unwrap();
    let refusing = format!(
        "{} vendor_refused=1 reason=price_refused run={run}",
        stats.replace(" failures=0 ", " failures=1 ")
    );
    std::fs::write(journal_path(&store), format!("{journal}{refusing}\n")).unwrap();

    let source = TickStore::new(&root.join("ts")).with_scope(stored());
    let run = Run {
        kind: ImportKind::Indices,
        from: d1(),
        to: d1(),
        only: &[],
        store_root: &store,
    };
    let plain = run_cm(&source, &run).unwrap();
    assert_eq!(
        (plain.days_skipped, plain.days_widened, plain.files),
        (1, 0, 0),
        "done with refusals is done: {:?}",
        plain.failures
    );
    let retry = RunOptions {
        retry_refused: true,
        ..RunOptions::default()
    };
    let retried = run_cm_from(&source, &run, retry).unwrap();
    assert!(retried.failures.is_empty(), "{:?}", retried.failures);
    assert_eq!(
        (
            retried.days_widened,
            retried.files,
            retried.seconds_committed
        ),
        (1, 1, 0),
        "the member's file is read again and nothing is written twice"
    );
    assert_eq!(retried.resumed, vec![d1()]);
    let last = journal_text(&store);
    let closes: Vec<&str> = last
        .lines()
        .filter(|line| line.starts_with(&head))
        .collect();
    assert_eq!(closes.len(), 3, "{last}");
    assert!(
        closes[2].contains(" failures=0 ") && !closes[2].contains("vendor_refused"),
        "the key closes by what the reader says now: {}",
        closes[2]
    );
    // Clean again, the flag has nothing to read.
    let settled = run_cm_from(&source, &run, retry).unwrap();
    assert_eq!((settled.days_skipped, settled.files), (1, 0));
    assert_eq!(journal_text(&store), last);
    std::fs::remove_dir_all(root).unwrap();
}

/// The journal of `store` with the LAST `done` of `key` restated as one that
/// closed with a refusal, the way the importer writes one: the same stats,
/// `failures=1`, and the vendor's refusal named before the run.
fn close_with_refusal(store: &Path, key: &str) {
    let journal = journal_text(store);
    let head = format!("done {key} ");
    let clean = journal
        .lines()
        .rfind(|line| line.starts_with(&head))
        .unwrap_or_else(|| panic!("{key}: {journal}"));
    assert!(clean.contains(" failures=0 "), "{clean}");
    let (stats, run) = clean.rsplit_once(" run=").unwrap();
    let refusing = format!(
        "{} vendor_refused=1 reason=header_unknown run={run}",
        stats.replace(" failures=0 ", " failures=1 ")
    );
    std::fs::write(journal_path(store), format!("{journal}{refusing}\n")).unwrap();
}

/// `retry_refused` NEVER CANCELS WHAT A DONE DAY BEFORE THE RANGE OWES
/// (WP-12f check 3, D-4878 item 6). The flag reads again, whole, a key of the
/// run's own range that closed done with refusals; a day before the range is
/// never read whole, so a close there that has refusals still owes the
/// members its scope never covered. Two days are closed `done` under `*` for
/// the swept surface, the second one with a refusal. A run of the third day
/// alone under the stored scope WITH the flag (under one named symbol, and
/// under no filter, where the earlier key is the run's own) used to treat
/// the second day as one it would read again, skip it unsaid, and make each
/// new member's month from the first and third days: the later repair then
/// landed the second day as a new generation. It now reads both owed days
/// first, in order, under the whole-day key's member keys, and repairs
/// nothing; the base close with its refusal is not read again or rewritten.
#[test]
fn retry_refused_never_cancels_what_a_done_day_before_the_range_owes() {
    for (name, only) in [("one", vec!["INDIAVIX".to_owned()]), ("none", Vec::new())] {
        let root = scratch(&format!("wp12f-retry-early-{name}"));
        four_index_days(&root);
        let (store, whole) = (root.join("S"), root.join("whole"));
        let narrow = run_on(&root, ImportKind::Indices, (d1(), d2()), &store, None).unwrap();
        assert!(narrow.failures.is_empty(), "{:?}", narrow.failures);
        close_with_refusal(&store, &format!("indices {} *", d2()));
        let wanted = if only.is_empty() { 2 } else { only.len() };
        let _earlier = summaries_taken();

        let source = TickStore::new(&root.join("ts")).with_scope(stored());
        let run = Run {
            kind: ImportKind::Indices,
            from: d3(),
            to: d3(),
            only: &only,
            store_root: &store,
        };
        let retry = RunOptions {
            retry_refused: true,
            ..RunOptions::default()
        };
        let late = run_cm_from(&source, &run, retry).unwrap();
        assert!(late.failures.is_empty(), "{name}: {:?}", late.failures);
        assert_eq!(
            (
                late.days_before_range,
                late.days_widened,
                late.members_reopened,
                late.days_imported,
                late.months_repaired
            ),
            (2, 2, 2 * wanted, 3, 0),
            "{name}: both owed days first, the one closed with a refusal among them"
        );
        let told = summaries_taken();
        let reach = told_with(&told, "scope_reach_back");
        assert_eq!(reach.len(), 1, "{name}: said once: {told:?}");
        assert_eq!(reach[0].text["first_owed"], d1().to_string(), "{name}");

        let journal = journal_text(&store);
        let filter = if only.is_empty() {
            "*".to_owned()
        } else {
            only.join(",")
        };
        let asked = journal
            .find(&format!("done indices {} {filter} ", d3()))
            .unwrap_or_else(|| panic!("{name}: {journal}"));
        for owed in [d1(), d2()] {
            let member = journal
                .find(&format!("done indices {owed} *+NSE-INDEX-INDIAVIX "))
                .unwrap_or_else(|| panic!("{name}: {owed} owed and not read: {journal}"));
            assert!(member < asked, "{name}: {owed} lands first: {journal}");
        }
        assert_eq!(
            lines_starting(&journal, &format!("done indices {} * ", d2())),
            2,
            "{name}: the base close with its refusal is not read again: {journal}"
        );

        let in_order = run_on(
            &root,
            ImportKind::Indices,
            (d1(), d3()),
            &whole,
            Some(stored()),
        )
        .unwrap();
        assert!(in_order.failures.is_empty(), "{:?}", in_order.failures);
        let members: &[&str] = if only.is_empty() {
            &["INDIAVIX", "NIFTYIT"]
        } else {
            &["INDIAVIX"]
        };
        for symbol in members {
            assert_eq!(
                month(&store, "INDEX", symbol),
                month(&whole, "INDEX", symbol),
                "{name}: {symbol} holds the day closed with a refusal, in order"
            );
        }
        assert!(
            !tree(&store.join("bars"))
                .keys()
                .any(|path| path.to_string_lossy().contains("bar-revisions")),
            "{name}: no month was repaired"
        );

        // Again with the flag: nothing is owed any more and nothing is read.
        let _told = summaries_taken();
        let again = run_cm_from(&source, &run, retry).unwrap();
        assert_eq!(
            (again.days_before_range, again.files, again.months_repaired),
            (0, 0, 0),
            "{name}"
        );
        assert_eq!(journal_text(&store), journal, "{name}");
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// One invented stocks day: RELIANCE's own file, a share whose only file is
/// its trade-for-trade one, and a share holding both.
fn spell_day(d: Day) -> DayPlan {
    let file = |stem: &str| (entry_of(Tree::Stocks, d, stem), stock_file(stem, d));
    (
        d,
        Tree::Stocks,
        vec![
            file("RELIANCE.NSE"),
            file("ADANIPOWER.BE.NSE"),
            file("CGPOWER.BZ.NSE"),
            file("SUZLON.NSE"),
            file("SUZLON.BE.NSE"),
            file("RELIANCE.SM.NSE"),
        ],
    )
}

/// A TRADE-FOR-TRADE SPELL IS THE SHARE'S OWN DAY (DB-B06). Under the stored
/// scope a `.BE` or `.BZ` file is filed under its share's key; the swept
/// surface keeps skipping it; a day closed done under the swept surface is
/// read again for the spell alone; a share with both its own file and a
/// series file that day has both refused by name and nothing filed; and any
/// other series (`.SM`) stays a skip.
#[test]
fn a_trade_for_trade_spell_is_filed_under_its_shares_key() {
    let root = scratch("wp12f-series");
    let on = d1();
    world_of(&root, &[spell_day(on)]);
    let (store, whole) = (root.join("S"), root.join("whole"));

    let narrow = run_on(&root, ImportKind::Stocks, (on, on), &store, None).unwrap();
    assert!(narrow.failures.is_empty(), "{:?}", narrow.failures);
    assert_eq!(narrow.files, 2, "RELIANCE and SUZLON, their own files");
    for share in ["ADANIPOWER", "CGPOWER"] {
        assert!(
            !held(&store, "CASH", share),
            "{share}: the swept surface files no series file"
        );
    }
    let suzlon = month(&store, "CASH", "SUZLON");
    assert!(!suzlon.is_empty());

    let wide = run_on(&root, ImportKind::Stocks, (on, on), &store, Some(stored())).unwrap();
    assert_eq!(
        (wide.days_widened, wide.days_imported),
        (1, 1),
        "{:?}",
        wide.failures
    );
    let journal = journal_text(&store);
    for (share, member) in [
        ("ADANIPOWER", "NSE-CASH-ADANIPOWER.BE"),
        ("CGPOWER", "NSE-CASH-CGPOWER.BZ"),
    ] {
        assert!(
            !month(&store, "CASH", share).is_empty(),
            "{share}: the spell's day is the share's own"
        );
        assert_eq!(
            lines_starting(
                &journal,
                &format!("done stocks {on} *+{member} definition=")
            ),
            1,
            "{member}: {journal}"
        );
    }
    // The share with both: its series member is refused by name as a twin,
    // and the bars its own file gave under the narrow scope are untouched.
    let twins: Vec<&Failure> = wide
        .failures
        .iter()
        .filter(|failure| failure.instrument.contains("SUZLON"))
        .collect();
    assert_eq!(twins.len(), 1, "{:?}", wide.failures);
    assert_eq!(wide.failures.len(), 1, "{:?}", wide.failures);
    assert_eq!(month(&store, "CASH", "SUZLON"), suzlon);

    // In order under the stored scope: the same bars for the spells, and the
    // twins refused, never one of them picked.
    let in_order = run_on(&root, ImportKind::Stocks, (on, on), &whole, Some(stored())).unwrap();
    assert_eq!(in_order.failures.len(), 1, "{:?}", in_order.failures);
    assert!(
        in_order.failures[0].instrument.contains("SUZLON"),
        "{:?}",
        in_order.failures
    );
    assert!(!held(&whole, "CASH", "SUZLON"));
    for share in ["ADANIPOWER", "CGPOWER", "RELIANCE"] {
        assert_eq!(
            month(&store, "CASH", share),
            month(&whole, "CASH", share),
            "{share}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// A member is the vendor's stem, by its exact bytes: no prefix and no
/// look-alike is another's member (DB-B12's two name traps), a series is its
/// own member, and the member list is sorted, without repeats, and one
/// journal field.
#[test]
fn a_member_is_named_by_its_stem_and_never_by_a_look_alike() {
    let scope = CmScope::stored(["RUCHI", "KPIT"]).unwrap();
    let of = |stem: &str| scope.member_of(CmKind::Stocks, stem);
    assert_eq!(of("KPIT.NSE").as_deref(), Some("NSE-CASH-KPIT"));
    assert_eq!(of("KPITTECH.NSE").as_deref(), Some("NSE-CASH-KPITTECH"));
    assert_eq!(of("RUCHI.BE.NSE").as_deref(), Some("NSE-CASH-RUCHI.BE"));
    assert_eq!(of("RUCHINFRA.NSE"), None);
    assert_eq!(of("RUCHIRA.NSE"), None);
    assert_eq!(of("RUCHI.SM.NSE"), None, "no other series is a member");
    assert_eq!(of("RUCHI.NSE_IDX"), None, "another tree's suffix");
    assert_eq!(
        scope
            .member_of(CmKind::Indices, "NIFTY IT.NSE_IDX")
            .as_deref(),
        Some("NSE-INDEX-NIFTYIT")
    );
    assert_eq!(
        CmScope::swept().member_of(CmKind::Indices, "NIFTY IT.NSE_IDX"),
        None
    );
    assert_eq!(CmScope::swept().member_of(CmKind::Stocks, "KPIT.NSE"), None);
    assert_eq!(
        CmScope::swept().member_of(CmKind::Stocks, "SUZLON.BE.NSE"),
        None,
        "the swept surface files no series"
    );

    let members = scope.members(CmKind::Stocks);
    let mut sorted = members.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(members, sorted);
    for member in ["NSE-CASH-KPIT", "NSE-CASH-KPIT.BZ", "NSE-CASH-SUZLON.BE"] {
        assert!(members.iter().any(|one| one == member), "{member}");
    }
    assert!(
        members
            .iter()
            .all(|member| !member.contains([' ', '+', ','])),
        "each is one field of a key and one item of a list"
    );
    let swept = CmScope::swept().members(CmKind::Stocks);
    assert!(swept.iter().all(|member| !member.contains('.')));
    assert!(swept.len() < members.len());

    // The digest names the list, however it was handed over.
    let mut shuffled = members.clone();
    shuffled.reverse();
    shuffled.push(members[0].clone());
    assert_eq!(scope::Record::of(shuffled), scope::Record::of(members));
    assert_ne!(
        scope::Record::of(swept).digest(),
        scope::Record::of(scope.members(CmKind::Stocks)).digest()
    );
    assert_eq!(
        crate::gdfl_cm::series_file("ADANIPOWER.BE.NSE.csv"),
        Some("BE")
    );
    assert_eq!(crate::gdfl_cm::series_file("ADANIPOWER.NSE.csv"), None);
    assert_eq!(crate::gdfl_cm::series_file("ADANIPOWER.SM.NSE.csv"), None);
}

/// The context of a stocks run over `store` under `record`, as `drive_core`
/// builds it.
fn ctx_of<'a>(
    run: &'a Run<'a>,
    journal: &'a Journal,
    (record, swept): (&'a scope::Record, &scope::Record),
) -> Ctx<'a> {
    Ctx {
        run,
        journal,
        options: RunOptions::default(),
        id: 2,
        started: 0,
        announced: std::cell::Cell::new(false),
        scoped: Some(Scoped {
            record,
            swept: Arc::clone(swept.set()),
            ledger: std::cell::RefCell::new(
                scope::Ledger::load(scope::scopes_path(run.store_root)).unwrap(),
            ),
            beyond: std::cell::RefCell::new(HashMap::new()),
        }),
    }
}

/// The 50th and 99th centile and the largest of `nanos`, sorted here.
fn spread(nanos: &mut [u128]) -> (u128, u128, u128) {
    nanos.sort_unstable();
    let at = |per_mille: usize| nanos[(nanos.len() - 1) * per_mille / 1_000];
    (at(500), at(990), nanos[nanos.len() - 1])
}

/// WHAT A DONE DAY OWES IS DECIDED IN O(1) A DAY, over a journal the size of
/// the real one's span (every calendar day 2018-01-01..2026-09-30 closed
/// `done` with no scope, as the real stocks days are): the members beyond a
/// close's scope are computed ONCE a run and shared by every day (one set,
/// by address), a day already examined under this scope is two hash probes,
/// and neither walks the journal or the scope's 600-odd members again. The
/// timings are printed for the record (`--nocapture`), never asserted: the
/// proof is the sharing and the absence asserted here, and this test,
/// `pull::gdfl_import::tests::scope_reopen::what_a_done_day_owes_is_decided_without_walking_the_journal_or_the_scope_again`,
/// is the one `Ctx::owed`'s doc names.
#[test]
fn what_a_done_day_owes_is_decided_without_walking_the_journal_or_the_scope_again() {
    let root = scratch("wp12f-owed-cost");
    let store = root.join("S");
    let all = days(day(2018, 1, 1), day(2026, 9, 30));
    let mut text = String::new();
    for d in &all {
        let _infallible = writeln!(
            text,
            "done stocks {d} * definition={BAR_DEFINITION} files=1 seconds=1 failures=0 late=0 \
             late_unresolved=0 max_back_s=0 run=1"
        );
    }
    put(&store, "imports/gdfl.journal", text.as_bytes());
    let journal = Journal::load(journal_path(&store)).unwrap();
    let scope = CmScope::stored(std::iter::empty::<&str>()).unwrap();
    let record = scope::Record::of(scope.members(CmKind::Stocks));
    let swept = scope::Record::of(CmScope::swept().members(CmKind::Stocks));
    let run = Run {
        kind: ImportKind::Stocks,
        from: all[0],
        to: all[all.len() - 1],
        only: &[],
        store_root: &store,
    };

    let ctx = ctx_of(&run, &journal, (&record, &swept));
    let mut nanos = Vec::with_capacity(all.len());
    let mut shared: Option<Arc<HashSet<String>>> = None;
    for d in &all {
        let key = format!("stocks {d} *");
        let asked = Asked {
            key: &key,
            day: *d,
            remembered: &key,
            names: None,
            early: false,
        };
        let at = std::time::Instant::now();
        let owed = ctx.owed(&asked);
        nanos.push(at.elapsed().as_nanos());
        let Owed::Probe(beyond) = owed else {
            panic!("{d}: a close with no scope owes the members beyond the swept surface");
        };
        match &shared {
            Some(first) => assert!(Arc::ptr_eq(first, &beyond), "{d}: computed again"),
            None => shared = Some(beyond),
        }
    }
    let beyond = shared.unwrap();
    assert_eq!(
        ctx.scoped.as_ref().unwrap().beyond.borrow().len(),
        1,
        "one set for the one scope the journal's closes name"
    );
    assert_eq!(beyond.len(), record.members().len() - swept.members().len());
    assert!(beyond.contains("NSE-CASH-SUZLON.BE") && !beyond.contains("NSE-CASH-SUZLON"));
    let (p50, p99, max) = spread(&mut nanos);
    println!(
        "WP-12f Ctx::owed, a close with no scope: n={} p50={p50}ns p99={p99}ns max={max}ns \
         ({} members, {} beyond)",
        all.len(),
        record.members().len(),
        beyond.len()
    );

    // Every day examined under this scope, none with a member key: nothing
    // is owed, and no set is built at all.
    let mut ledger = scope::Ledger::load(scope::scopes_path(&store)).unwrap();
    ledger.record("stocks", &record).unwrap();
    // The lines `Ledger::examine` writes, in one write: a flush a line is
    // the ledger's cost, not this measurement's.
    let mut examined = scopes_text(&store);
    for d in &all {
        let _infallible = writeln!(
            examined,
            "examined stocks {d} * scope={} run=2 opened=0 .",
            record.digest()
        );
    }
    std::fs::write(scope::scopes_path(&store), examined).unwrap();
    let ctx = ctx_of(&run, &journal, (&record, &swept));
    let mut nanos = Vec::with_capacity(all.len());
    for d in &all {
        let key = format!("stocks {d} *");
        let asked = Asked {
            key: &key,
            day: *d,
            remembered: &key,
            names: None,
            early: false,
        };
        let at = std::time::Instant::now();
        let owed = ctx.owed(&asked);
        nanos.push(at.elapsed().as_nanos());
        assert!(matches!(owed, Owed::Nothing), "{d}");
    }
    assert!(ctx.scoped.as_ref().unwrap().beyond.borrow().is_empty());
    let (p50, p99, max) = spread(&mut nanos);
    println!(
        "WP-12f Ctx::owed, a day examined under this scope: n={} p50={p50}ns p99={p99}ns \
         max={max}ns",
        all.len()
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// CMP-07, D-4878 item 10: definition 4 is the closing auction's band through
/// matching (SC-CLOSE-5, SC-CLOSE-6), which changed only what an eligible
/// share's file folds from 2026-08-03. So a share's day from 2026-08-03
/// closed `done` under 2 or 3 is restated, no longer done (before, every
/// share day under 2 passed as current whichever rule set had built it, and
/// the auction's print stayed a bar; admission then refuses it by name,
/// `a_restated_share_day_of_the_auction_era_is_refused_by_name_never_appended`);
/// the same definitions stay current for
/// a share's day before it, for an index day, and under 3 for an option day
/// (an option day under 2 is restated, D-3342). A share key whose day word is
/// no date is read again, never passed. The rule asks one predicate, the
/// vendor's own `cash_auction_eligibility_required`, for every kind.
#[test]
fn a_share_day_of_the_auction_era_under_an_older_definition_is_restated() {
    let lines = [
        ("stocks 2026-07-31 *", 3, true),
        ("stocks 2026-07-31 NSE-CASH-RELIANCE", 2, true),
        ("stocks 2026-08-03 *", 2, false),
        ("stocks 2026-08-04 *", 3, false),
        ("stocks 2026-08-05 *", 4, true),
        ("stocks 2026-08-06 *", 1, false),
        ("stocks 2026-8-7 *", 3, false),
        ("stocks 2026-8-7 *+NSE-CASH-INFY", 4, true),
        ("indices 2026-08-04 *", 2, true),
        ("indices 2026-08-05 *", 3, true),
        ("options 2026-08-04 *", 3, true),
        ("options 2026-08-05 *", 2, false),
        ("options 2019-02-01 *", 4, true),
    ];
    let root = scratch("wp12f-definition4");
    let mut text = String::new();
    for (key, definition, _) in &lines {
        let _ = writeln!(text, "begin {key}");
        let _ = writeln!(
            text,
            "done {key} definition={definition} files=1 seconds=1 failures=0"
        );
    }
    std::fs::create_dir_all(root.join("imports")).unwrap();
    std::fs::write(journal_path(&root), text).unwrap();
    let journal = Journal::load(journal_path(&root)).unwrap();
    for (key, definition, current) in lines {
        assert_eq!(
            (journal.done.contains(key), journal.restated.contains(key)),
            (current, !current),
            "{key} under {definition}"
        );
    }
    let on = |y, m, d| Day::new(y, m, d).unwrap();
    assert!(definition_is_current(ImportKind::Stocks, on(2026, 8, 2), 3));
    assert!(!definition_is_current(
        ImportKind::Stocks,
        on(2026, 8, 3),
        3
    ));
    assert!(definition_is_current(
        ImportKind::Stocks,
        on(2026, 8, 3),
        u64::from(BAR_DEFINITION)
    ));
    assert_eq!(BAR_DEFINITION, 4);
    std::fs::remove_dir_all(root).unwrap();
}

/// D-4878 item 10 (CMP-07): a share's day of the closing auction's era that
/// definition 4 restated is refused by name at admission, every run, and no
/// file of it is read or appended: the second reading takes the print out of
/// the bars, the held day less its last bar is answered `AlreadyPresent`, and
/// the day would close `done` under 4 with the print still a bar. A names
/// filter on such a day is refused too (its months were made by the restated
/// whole-tree key); a share's day before 2026-08-03 under 3 stays done. The
/// day's key stays restated through its `refused` line, so the next run
/// refuses it again and names it in its failures (the journal keeps the one
/// identical close); a `done` under 4 (WP-80's new generation) ends it.
#[test]
fn a_restated_share_day_of_the_auction_era_is_refused_by_name_never_appended() {
    let root = scratch("wp12f-restated-auction");
    let (before, era) = (
        Day::new(2026, 7, 31).unwrap(),
        Day::new(2026, 8, 4).unwrap(),
    );
    let plan = |d: Day| -> DayPlan {
        (
            d,
            Tree::Stocks,
            vec![(
                entry_of(Tree::Stocks, d, "RELIANCE.NSE"),
                stock_file("RELIANCE.NSE", d),
            )],
        )
    };
    world_of(&root, &[plan(before), plan(era)]);
    let store = root.join("S");
    std::fs::create_dir_all(store.join("imports")).unwrap();
    let mut legacy = String::new();
    for d in [before, era] {
        let _ = writeln!(legacy, "begin stocks {d} *");
        let _ = writeln!(
            legacy,
            "done stocks {d} * definition=3 files=1 seconds=1 failures=0"
        );
    }
    std::fs::write(journal_path(&store), legacy).unwrap();
    let run = |d: Day, only: &[String]| {
        let source = TickStore::new(&root.join("ts"));
        run_cm(
            &source,
            &Run {
                kind: ImportKind::Stocks,
                from: d,
                to: d,
                only,
                store_root: &store,
            },
        )
        .unwrap()
    };
    let kept = run(before, &[]);
    assert_eq!(
        (kept.days_skipped, kept.days_refused, kept.files),
        (1, 0, 0),
        "{:?}",
        kept.failures
    );
    let names = ["RELIANCE".to_owned()];
    for only in [&[][..], &names[..], &[][..]] {
        let refused = run(era, only);
        assert_eq!(
            (refused.days_refused, refused.days_imported, refused.files),
            (1, 0, 0),
            "{only:?}: {:?}",
            refused.failures
        );
        assert!(
            refused
                .failures
                .iter()
                .any(|f| f.why.contains("restated by bar definition 4")),
            "{:?}",
            refused.failures
        );
    }
    let journal = journal_text(&store);
    assert_eq!(
        lines_starting(
            &journal,
            "refused stocks 2026-08-04 * definition=4 stage=file reason=restated_auction_band"
        ),
        1,
        "the third run names the same refusal in its failures and does not write an \
         identical close twice: {journal}"
    );
    assert_eq!(
        lines_starting(
            &journal,
            "refused stocks 2026-08-04 RELIANCE definition=4 stage=file reason=restated_auction_band"
        ),
        1,
        "{journal}"
    );
    assert!(
        !store.join("bars").exists(),
        "no file of the restated day was read or appended"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// THE AUCTION ERA'S ADMISSION CHECK COSTS THE SAME AT ANY JOURNAL SIZE
/// (D-4878 item 10, CMP-07): `restated_past_append` is two hash probes of the
/// journal's restated set and a date compare, never a walk of the journal.
/// Two journals a hundredfold apart (every day of the era restated under one
/// filter, then under a hundred) answer the same keys in the same per-call
/// time: 200 batches of 1,000 calls each, the two journals interleaved batch
/// by batch so the machine's load falls on both alike; p50, p99 and max of
/// the per-call nanoseconds printed, and the larger journal's p50 held within
/// three times the smaller's (a walk of the larger set would be hundreds of
/// times slower). Its answers are asserted first: a restated key, a key
/// under a names filter whose whole-tree key is restated, a day before the
/// era and an index day.
#[test]
fn the_auction_era_admission_check_costs_the_same_at_any_journal_size() {
    let root = scratch("wp12f-admit-cost");
    let era: Vec<Day> = days(day(2026, 8, 3), day(2026, 9, 30));
    let journal_of = |filters: usize, at: &str| -> Journal {
        let store = root.join(at);
        let mut text = String::new();
        for d in &era {
            for n in 0..filters {
                let only = if n == 0 {
                    "*".to_owned()
                } else {
                    format!("SYM{n:03}")
                };
                let _ = writeln!(text, "begin stocks {d} {only}");
                let _ = writeln!(
                    text,
                    "done stocks {d} {only} definition=3 files=1 seconds=1 failures=0"
                );
            }
        }
        std::fs::create_dir_all(store.join("imports")).unwrap();
        std::fs::write(journal_path(&store), text).unwrap();
        Journal::load(journal_path(&store)).unwrap()
    };
    let (small, large) = (journal_of(1, "small"), journal_of(100, "large"));
    assert_eq!(
        (small.restated.len(), large.restated.len()),
        (era.len(), era.len() * 100)
    );
    // What it answers, on both.
    let probe = era[era.len() / 2];
    let (whole, named) = (format!("stocks {probe} *"), format!("stocks {probe} INFY"));
    for journal in [&small, &large] {
        assert!(restated_past_append(
            journal,
            ImportKind::Stocks,
            &whole,
            probe
        ));
        assert!(
            restated_past_append(journal, ImportKind::Stocks, &named, probe),
            "a names filter on a day whose whole-tree key is restated"
        );
        assert!(!restated_past_append(
            journal,
            ImportKind::Indices,
            &whole,
            probe
        ));
        let before = day(2026, 7, 31);
        assert!(!restated_past_append(
            journal,
            ImportKind::Stocks,
            &format!("stocks {before} *"),
            before
        ));
    }
    // The same keys asked of both journals, half of them restated.
    let asked: Vec<(String, Day)> = era
        .iter()
        .flat_map(|d| {
            [
                (format!("stocks {d} *"), *d),
                (format!("stocks {d} ACC"), *d),
            ]
        })
        .collect();
    let batch = |journal: &Journal| -> u128 {
        let at = std::time::Instant::now();
        let mut hits = 0_usize;
        for n in 0..1_000 {
            let (key, d) = &asked[n % asked.len()];
            hits += usize::from(std::hint::black_box(restated_past_append(
                journal,
                ImportKind::Stocks,
                key,
                *d,
            )));
        }
        assert_eq!(hits, 1_000, "every day of the era is restated in both");
        at.elapsed().as_nanos() / 1_000
    };
    let (mut at_small, mut at_large) = (Vec::with_capacity(200), Vec::with_capacity(200));
    for _ in 0..200 {
        at_small.push(batch(&small));
        at_large.push(batch(&large));
    }
    let (s50, s99, smax) = spread(&mut at_small);
    let (l50, l99, lmax) = spread(&mut at_large);
    println!(
        "WP-12f restated_past_append, ns a call (200 batches of 1,000): {} restated keys \
         p50={s50} p99={s99} max={smax}; {} restated keys p50={l50} p99={l99} max={lmax}",
        small.restated.len(),
        large.restated.len()
    );
    assert!(
        l50 <= 3 * s50.max(1),
        "a hundredfold journal costs the check {l50} ns against {s50} ns: it walks"
    );
    std::fs::remove_dir_all(root).unwrap();
}

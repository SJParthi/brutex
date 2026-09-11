//! Grouped-barrier, crash-order and directory-memo checks for sweep evidence.
//! Every write targets a unique temporary fixture, never a market store.

use super::*;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::rc::Rc;

type Hook = Box<dyn FnMut(&Path) -> std::io::Result<()>>;

std::thread_local! {
    static BARRIERS: Cell<u64> = const { Cell::new(0) };
    static HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) };
    static MODELLED: Cell<bool> = const { Cell::new(false) };
}

/// Durability barriers this module issued on the calling thread.
pub(crate) fn flush_count() -> u64 {
    BARRIERS.with(Cell::get)
}

/// Count one barrier and let an installed watch observe or refuse it. Returns
/// whether the storage barrier must still be issued: a modelled watch makes
/// durability the test's own explicit model, so no device flush is spent.
pub(super) fn observe(path: &Path) -> std::io::Result<bool> {
    BARRIERS.with(|count| count.set(count.get() + 1));
    // Lent out while it runs, so evidence calls made by the watch cannot re-enter it.
    if let Some(mut hook) = HOOK.with(|slot| slot.borrow_mut().take()) {
        let seen = hook(path);
        HOOK.with(|slot| *slot.borrow_mut() = Some(hook));
        seen?;
    }
    Ok(!MODELLED.with(Cell::get))
}

/// A thread-local barrier watch, removed when dropped.
pub(crate) struct BarrierWatch;

impl BarrierWatch {
    /// Observe every barrier on this thread; real storage barriers still run.
    pub(crate) fn install(hook: impl FnMut(&Path) -> std::io::Result<()> + 'static) -> Self {
        HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
        Self
    }
    /// Observe every barrier while the caller models durability explicitly.
    pub(crate) fn modelled(hook: impl FnMut(&Path) -> std::io::Result<()> + 'static) -> Self {
        MODELLED.with(|modelled| modelled.set(true));
        Self::install(hook)
    }
}

impl Drop for BarrierWatch {
    fn drop(&mut self) {
        HOOK.with(|slot| *slot.borrow_mut() = None);
        MODELLED.with(|modelled| modelled.set(false));
    }
}

const LIMIT: u64 = 1_000_000;
const FRESH: [u8; 32] = [0xEE; 32];
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Result<Self, String> {
        let root = std::env::temp_dir().join(format!(
            "brutex-sweep-evidence-unit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).map_err(text)?;
        Ok(Self(root))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn text(why: impl std::fmt::Display) -> String {
    why.to_string()
}

fn depth(k: u64) -> DepthRow {
    DepthRow {
        k,
        generated: 5,
        duplicates: 0,
        pruned: 1,
        infrequent: 2,
        frequent: 2,
        admitted: 7,
        pairs: 11,
        reconciles: true,
        excluded: 0,
    }
}

fn rank(index: u64) -> RankedRow {
    RankedRow {
        rank: index,
        mask_words: [1_u64 << 63, index, 0, 0, 0, 1_u64 << 49],
        hits: 37,
        observations: 31,
        mean_bits: 0x3ff0_0000_0000_0001,
        t_bits: (-0.0_f64).to_bits(),
        refused: 2,
        mismatched: 4,
        wins: 19,
        losses: 12,
        win_sum_bits: 123.456_789_f64.to_bits(),
        loss_sum_bits: (-98.765_432_f64).to_bits(),
        adverse_sum_bits: (-765.432_198_f64).to_bits(),
        favourable_sum_bits: 987.654_321_f64.to_bits(),
    }
}

/// Acknowledged children an attempt published, keyed by its token.
type Published = BTreeMap<u64, (Vec<DepthRow>, Vec<RankedRow>)>;

fn publish(
    attempt: &Attempt,
    levels: u64,
    ranks: u64,
    published: &mut Published,
) -> Result<(), String> {
    let depths: Vec<DepthRow> = (1..=levels).map(depth).collect();
    let rows: Vec<RankedRow> = (1..=ranks).map(rank).collect();
    for row in &depths {
        attempt.level(*row)?;
    }
    attempt.ranked(&rows)?;
    published.insert(attempt.token(), (depths, rows));
    Ok(())
}

/// Every evidence file below `root`, with event timestamps and the seals that
/// cover them zeroed: the only bytes two correct writers may legitimately differ in.
fn normalized(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    let base = base(root);
    let mut files = BTreeMap::new();
    let mut pending = vec![base.clone()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).map_err(text)? {
            let path = entry.map_err(text)?.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let mut bytes = fs::read(&path).map_err(text)?;
            if bytes.starts_with(&EVENTS) {
                let rows = bytes.get_mut(16..).unwrap_or_default();
                for row in rows.chunks_exact_mut(EVENT_BYTES) {
                    for (at, byte) in row.iter_mut().enumerate() {
                        if (40..56).contains(&at) || at >= 88 {
                            *byte = 0;
                        }
                    }
                }
            }
            files.insert(path.strip_prefix(&base).map_err(text)?.to_path_buf(), bytes);
        }
    }
    Ok(files)
}

/// What a power loss at one barrier may leave: every earlier barrier held; the
/// write being made durable may be present, missing or torn; entries created
/// since their directory's last barrier may be present or lost.
struct Crashes {
    root: PathBuf,
    shots: PathBuf,
    taken: usize,
    synced: HashMap<PathBuf, u64>,
    listed: HashMap<PathBuf, BTreeSet<OsString>>,
    kinds: BTreeSet<String>,
}

enum Loss {
    Nothing,
    Tail(PathBuf, u64),
    Lost(Vec<PathBuf>),
}

impl Crashes {
    fn new(root: &Path, shots: &Path) -> Result<Self, String> {
        let mut crashes = Self {
            root: root.to_path_buf(),
            shots: shots.to_path_buf(),
            taken: 0,
            synced: HashMap::new(),
            listed: HashMap::new(),
            kinds: BTreeSet::new(),
        };
        // Everything written before the scenario passed its own barriers.
        let mut pending = vec![root.to_path_buf()];
        while let Some(directory) = pending.pop() {
            crashes
                .listed
                .insert(directory.clone(), listing(&directory)?);
            for entry in fs::read_dir(&directory).map_err(text)? {
                let path = entry.map_err(text)?.path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    let len = fs::metadata(&path).map_err(text)?.len();
                    crashes.synced.insert(path, len);
                }
            }
        }
        Ok(crashes)
    }

    /// Photograph the states a power loss before this barrier may leave, then
    /// record what the barrier makes durable.
    fn at(&mut self, path: &Path) -> Result<(), String> {
        if !path.starts_with(&self.root) {
            return Ok(()); // An ancestor above the fixture: outside this model.
        }
        self.kinds.insert(kind(path));
        let metadata = fs::metadata(path).map_err(text)?;
        let mut undurable = Vec::new();
        self.undurable(&self.root, &mut undurable)?;
        let mut losses = vec![Loss::Nothing];
        if undurable.len() > 1 {
            losses.extend(
                undurable
                    .iter()
                    .map(|entry| Loss::Lost(vec![entry.clone()])),
            );
        }
        if !undurable.is_empty() {
            losses.push(Loss::Lost(undurable));
        }
        if metadata.is_file() {
            let durable = self.synced.get(path).copied().unwrap_or(0);
            losses.push(Loss::Tail(path.to_path_buf(), durable));
            if metadata.len() > durable + 1 {
                losses.push(Loss::Tail(path.to_path_buf(), durable + 1));
            }
        }
        for loss in losses {
            self.shoot(loss)?;
        }
        if metadata.is_file() {
            self.synced.insert(path.to_path_buf(), metadata.len());
        } else {
            self.listed.insert(path.to_path_buf(), listing(path)?);
        }
        Ok(())
    }

    /// Top-most entries created since their directory's last barrier.
    fn undurable(&self, directory: &Path, found: &mut Vec<PathBuf>) -> Result<(), String> {
        let durable = self.listed.get(directory);
        for name in listing(directory)? {
            let path = directory.join(&name);
            if durable.is_none_or(|names| !names.contains(&name)) {
                found.push(path);
            } else if path.is_dir() {
                self.undurable(&path, found)?;
            }
        }
        Ok(())
    }

    fn shoot(&mut self, loss: Loss) -> Result<(), String> {
        let target = self.shots.join(self.taken.to_string());
        self.taken += 1;
        copy_tree(&self.root, &target)?;
        let moved = |path: &Path| {
            path.strip_prefix(&self.root)
                .map(|relative| target.join(relative))
                .map_err(text)
        };
        match loss {
            Loss::Nothing => Ok(()),
            Loss::Tail(path, len) => OpenOptions::new()
                .write(true)
                .open(moved(&path)?)
                .and_then(|file| file.set_len(len))
                .map_err(text),
            Loss::Lost(paths) => {
                for path in paths {
                    let copy = moved(&path)?;
                    if copy.is_dir() {
                        fs::remove_dir_all(copy).map_err(text)?;
                    } else {
                        fs::remove_file(copy).map_err(text)?;
                    }
                }
                Ok(())
            }
        }
    }
}

/// A barrier's kind: a file name without its token, or a directory's role.
fn kind(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if path.is_dir() {
        return if name.len() == 64 {
            "identity directory".to_owned()
        } else {
            format!("{name} directory")
        };
    }
    name.trim_start_matches(|c: char| c.is_ascii_digit())
        .to_owned()
}

fn listing(directory: &Path) -> Result<BTreeSet<OsString>, String> {
    fs::read_dir(directory)
        .map_err(text)?
        .map(|entry| entry.map(|entry| entry.file_name()).map_err(text))
        .collect()
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(text)?;
    for name in listing(from)? {
        let (source, target) = (from.join(&name), to.join(&name));
        if source.is_dir() {
            copy_tree(&source, &target)?;
        } else {
            fs::copy(&source, &target).map_err(text)?;
        }
    }
    Ok(())
}

/// Tokens a durable start already names: reservations and lifecycle files.
fn used_tokens(shot: &Path) -> Result<BTreeSet<u64>, String> {
    let mut used = BTreeSet::new();
    let mut pending: Vec<PathBuf> = vec![base(shot)]
        .into_iter()
        .filter(|base| base.is_dir())
        .collect();
    while let Some(directory) = pending.pop() {
        for name in listing(&directory)? {
            let path = directory.join(&name);
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let name = name.to_string_lossy();
            for suffix in ["-start.bin", "-lifecycle.bin"] {
                used.extend(
                    name.strip_suffix(suffix)
                        .and_then(|token| token.parse::<u64>().ok()),
                );
            }
        }
    }
    Ok(used)
}

fn torn_journal(shot: &Path) -> bool {
    fs::metadata(base(shot).join("attempts.bin")).is_ok_and(|journal| {
        journal.len() != 0 && (journal.len() < 16 || !(journal.len() - 16).is_multiple_of(96))
    })
}

/// Readers may refuse or report Running, never a completion without its
/// acknowledged rows; a later writer is refused loudly only by a torn global
/// journal, and otherwise receives a token no durable start already names.
fn judge(shot: &Path, identities: &[[u8; 32]], published: &Published) -> Result<(), String> {
    let mut seen: Vec<Evidence> = identities
        .iter()
        .filter_map(|identity| read(shot, *identity, LIMIT).ok().flatten())
        .collect();
    seen.extend(latest(shot, LIMIT).ok().flatten());
    for saved in seen
        .iter()
        .filter(|saved| saved.completion == Completion::Completed)
    {
        let expected = published.get(&saved.attempt).cloned().unwrap_or_default();
        let found = (
            depth_page(shot, saved, 0, 4096, LIMIT)?,
            ranked_page(shot, saved, 0, 4096, LIMIT)?,
        );
        assert_eq!(
            found,
            expected,
            "{}: a completion must carry its acknowledged rows",
            shot.display()
        );
    }
    let used = used_tokens(shot)?;
    let _quiet = BarrierWatch::modelled(|_| Ok(()));
    match begin(shot, FRESH, Operation::Sweep) {
        Ok(attempt) => {
            assert!(
                !used.contains(&attempt.token()),
                "{}: token {} was reused",
                shot.display(),
                attempt.token()
            );
            std::mem::forget(attempt); // A stopped process writes no terminal.
        }
        Err(why) => assert!(
            torn_journal(shot) && why.contains("torn or short"),
            "{}: {why}",
            shot.display()
        ),
    }
    Ok(())
}

/// One grouped start, children and grouped finish with every barrier
/// photographed; then every photograph is judged. Returns the barrier kinds seen.
fn crash_scenario(
    root: &Path,
    identities: &[[u8; 32]],
    mut published: Published,
) -> Result<BTreeSet<String>, String> {
    let shots = Fixture::new()?;
    let crashes = Rc::new(RefCell::new(Crashes::new(root, &shots.0)?));
    let watch = BarrierWatch::modelled({
        let crashes = Rc::clone(&crashes);
        move |path| crashes.borrow_mut().at(path).map_err(std::io::Error::other)
    });
    let mut begun = Vec::new();
    begin_many(root, identities, Operation::Sweep, &mut begun)?;
    for (levels, attempt) in (1_u64..).zip(&begun) {
        publish(attempt, levels, 2, &mut published)?;
    }
    finish_many(begun, Completion::Completed)?;
    drop(watch);
    let crashes = crashes.borrow();
    for shot in 0..crashes.taken {
        judge(&shots.0.join(shot.to_string()), identities, &published)?;
    }
    Ok(crashes.kinds.clone())
}

fn completed_earlier(
    root: &Path,
    identity: [u8; 32],
    published: &mut Published,
) -> Result<(), String> {
    let _quiet = BarrierWatch::modelled(|_| Ok(()));
    let earlier = begin(root, identity, Operation::Sweep)?;
    publish(&earlier, 1, 1, published)?;
    earlier.finish(Completion::Completed)
}

#[test]
fn every_grouped_barrier_crash_leaves_only_harmless_states() -> Result<(), String> {
    // A first group on a fresh root starts the journal, its directories and every identity index.
    let fresh = Fixture::new()?;
    let mut kinds = crash_scenario(&fresh.0, &[[41; 32], [42; 32], [43; 32]], Published::new())?;
    // A later group reruns a completed identity beside a new one.
    let rerun = Fixture::new()?;
    let mut published = Published::new();
    completed_earlier(&rerun.0, [44; 32], &mut published)?;
    kinds.extend(crash_scenario(&rerun.0, &[[44; 32], [45; 32]], published)?);
    for expected in [
        "attempts.bin",
        "-start.bin",
        "sweep-evidence-v1 directory",
        "-lifecycle.bin",
        "identity directory",
        "starts.bin",
        "-levels.bin",
        "-ranked.bin",
    ] {
        assert!(
            kinds.contains(expected),
            "{expected} was never photographed: {kinds:?}"
        );
    }
    Ok(())
}

/// Refuse exactly the `at`-th barrier on this thread, counting from zero.
fn refuse_at(at: u64) -> (BarrierWatch, Rc<Cell<u64>>) {
    let seen = Rc::new(Cell::new(0_u64));
    let counter = Rc::clone(&seen);
    let watch = BarrierWatch::modelled(move |_| {
        let index = counter.get();
        counter.set(index + 1);
        if index == at {
            Err(std::io::Error::other("injected barrier refusal"))
        } else {
            Ok(())
        }
    });
    (watch, seen)
}

#[test]
fn a_refusal_at_any_grouped_barrier_is_loud_and_keeps_only_a_durable_prefix() -> Result<(), String>
{
    let identities = [[71; 32], [72; 32], [73; 32]];
    let mut at = 0_u64;
    loop {
        let fixture = Fixture::new()?;
        let mut published = Published::new();
        completed_earlier(&fixture.0, [71; 32], &mut published)?;
        let (watch, seen) = refuse_at(at);
        let mut begun = Vec::new();
        let started = begin_many(&fixture.0, &identities, Operation::Sweep, &mut begun);
        for attempt in &begun {
            let saved = read_attempt(&fixture.0, attempt.identity(), attempt.token(), LIMIT)?;
            assert_eq!(
                saved.map(|saved| saved.completion),
                Some(Completion::Running),
                "barrier {at}"
            );
        }
        let outcome = started
            .and_then(|()| {
                begun
                    .iter()
                    .try_for_each(|attempt| publish(attempt, 2, 2, &mut published))
            })
            .and_then(|()| finish_many(std::mem::take(&mut begun), Completion::Completed));
        drop(begun);
        drop(watch);
        if seen.get() <= at {
            outcome?;
            assert!(
                at > 30,
                "the scenario passes through every grouped barrier: {at}"
            );
            return Ok(());
        }
        let why = outcome.err().unwrap_or_default();
        assert!(
            why.contains("injected barrier refusal"),
            "barrier {at}: {why}"
        );
        assert!(
            !flushed().contains(&base(&fixture.0)),
            "barrier {at}: a refusal forgets remembered chains"
        );
        judge(&fixture.0, &identities, &published)?;
        at += 1;
    }
}

#[test]
fn the_directory_memo_skips_a_flushed_chain_and_forgets_it_after_an_io_refusal()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let base = base(&fixture.0);
    let quiet = BarrierWatch::modelled(|_| Ok(()));
    // Another test's refusal may forget every chain meanwhile; then measure
    // again -- on a chain this test has NOT already remembered. A retry that
    // re-measured the same path would count the memo's own skip, read zero
    // barriers where the chain length is expected, and fail while the memo was
    // working exactly as it should.
    let mut measured = None;
    for attempt in 0..16_u32 {
        let target = base.join(format!("memo-{attempt}"));
        let chain = u64::try_from(target.ancestors().count()).map_err(text)?;
        let epoch = FORGOTTEN.load(Ordering::Acquire);
        let before = flush_count();
        durable_directory(&target)?;
        let first = flush_count() - before;
        durable_directory(&target)?;
        let repeat = flush_count() - before - first;
        if FORGOTTEN.load(Ordering::Acquire) == epoch {
            assert_eq!((first, repeat), (chain, 0));
            measured = Some(target);
            break;
        }
    }
    let target = measured.ok_or("no quiet window for the memo in sixteen tries")?;
    // Any evidence I/O refusal forgets every remembered chain.
    let _ = io_error("injected refusal");
    assert!(!flushed().contains(&target));
    drop(quiet);
    // A refusal while a chain is flushing leaves that chain unremembered.
    let forgetting = BarrierWatch::modelled(|_| {
        forget_flushed();
        Ok(())
    });
    durable_directory(&target)?;
    assert!(!flushed().contains(&target));
    drop(forgetting);
    Ok(())
}

#[test]
fn eight_single_starts_and_one_group_of_eight_leave_the_same_evidence() -> Result<(), String> {
    let identities: Vec<[u8; 32]> = (1..=8_u8).map(|byte| [byte; 32]).collect();
    let _quiet = BarrierWatch::modelled(|_| Ok(()));
    let mut ignored = Published::new();
    let single = Fixture::new()?;
    let mut begun = Vec::new();
    for identity in &identities {
        begun.push(begin(&single.0, *identity, Operation::IndexStop)?);
    }
    for attempt in begun.iter().step_by(2) {
        publish(attempt, 2, 1, &mut ignored)?;
    }
    for attempt in begun {
        attempt.finish(Completion::Completed)?;
    }
    let grouped = Fixture::new()?;
    let mut begun = Vec::new();
    begin_many(&grouped.0, &identities, Operation::IndexStop, &mut begun)?;
    for attempt in begun.iter().step_by(2) {
        publish(attempt, 2, 1, &mut ignored)?;
    }
    finish_many(begun, Completion::Completed)?;
    let (single, grouped) = (normalized(&single.0)?, normalized(&grouped.0)?);
    assert_eq!(single.len(), 1 + 8 + 2 * 8 + 2 * 4);
    assert_eq!(single, grouped);
    Ok(())
}

#[test]
fn a_refused_journal_while_dropping_is_loud_and_never_completes() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let attempt = {
        let _quiet = BarrierWatch::modelled(|_| Ok(()));
        begin(&fixture.0, [96; 32], Operation::Sweep)?
    };
    let token = attempt.token();
    let refusing = BarrierWatch::modelled(|path| {
        if path.ends_with("attempts.bin") {
            Err(std::io::Error::other("injected journal refusal"))
        } else {
            Ok(())
        }
    });
    drop(attempt);
    drop(refusing);
    // The lifecycle holds the Refused terminal. The journal row whose barrier
    // was refused may or may not survive: the journal ends at this attempt's
    // start or its Refused terminal, never at a completion.
    let saved = read_attempt(&fixture.0, [96; 32], token, LIMIT)?.map(|saved| saved.completion);
    assert_eq!(saved, Some(Completion::Refused));
    let tail = latest(&fixture.0, LIMIT)?.map(|saved| (saved.attempt, saved.completion));
    assert!(
        matches!(tail, Some((attempt, Completion::Running | Completion::Refused)) if attempt == token),
        "{tail:?}"
    );
    Ok(())
}

#[test]
fn a_refused_chain_barrier_refuses_the_start_and_is_not_remembered() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let base = base(&fixture.0);
    let refusing = BarrierWatch::modelled(|_| Err(std::io::Error::other("injected chain refusal")));
    let mut begun = Vec::new();
    let refusal = begin_many(&fixture.0, &[[97; 32]], Operation::Sweep, &mut begun)
        .err()
        .unwrap_or_default();
    drop(refusing);
    assert!(refusal.contains("injected chain refusal"), "{refusal}");
    assert!(begun.is_empty() && !flushed().contains(&base));
    assert!(!base.join("attempts.bin").exists(), "nothing is allocated");
    Ok(())
}

#[test]
fn the_event_header_constant_is_the_header_shape_writes() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let path = fixture.0.join("events.bin");
    let mut file = open_append(&path)?;
    let _quiet = BarrierWatch::modelled(|_| Ok(()));
    assert_eq!(shape::<EVENT_BYTES>(&mut file, &path, EVENTS, true)?, 0);
    assert_eq!(fs::read(&path).map_err(text)?, EVENT_HEADER);
    Ok(())
}

#[test]
fn empty_groups_write_nothing_and_running_is_never_terminal() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let mut begun = Vec::new();
    begin_many(&fixture.0, &[], Operation::Sweep, &mut begun)?;
    assert!(begun.is_empty() && !base(&fixture.0).exists());
    finish_many(Vec::new(), Completion::Completed)?;
    assert!(finish_many(Vec::new(), Completion::Running).is_err());
    Ok(())
}

#[test]
fn attempts_from_two_roots_cannot_share_one_journal_barrier() -> Result<(), String> {
    let (one, two) = (Fixture::new()?, Fixture::new()?);
    let _quiet = BarrierWatch::modelled(|_| Ok(()));
    let attempts = vec![
        begin(&one.0, [51; 32], Operation::Sweep)?,
        begin(&two.0, [52; 32], Operation::Sweep)?,
    ];
    let refusal = finish_many(attempts, Completion::Completed)
        .err()
        .unwrap_or_default();
    assert!(refusal.contains("different evidence roots"), "{refusal}");
    let first = read(&one.0, [51; 32], LIMIT)?.map(|saved| saved.completion);
    let second = read(&two.0, [52; 32], LIMIT)?.map(|saved| saved.completion);
    assert_eq!(
        (first, second),
        (Some(Completion::Completed), Some(Completion::Refused))
    );
    Ok(())
}

#[test]
fn a_refused_attempt_stops_a_group_finish_in_sequential_order() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let _quiet = BarrierWatch::modelled(|_| Ok(()));
    let identities = [[61; 32], [62; 32], [63; 32]];
    let mut begun = Vec::new();
    begin_many(&fixture.0, &identities, Operation::Sweep, &mut begun)?;
    let middle = begun.get(1).ok_or("the middle attempt")?;
    middle.ranked(&[])?;
    assert!(
        middle.ranked(&[]).is_err(),
        "a second publication poisons the attempt"
    );
    let refusal = finish_many(begun, Completion::Completed)
        .err()
        .unwrap_or_default();
    assert!(
        refusal.contains("already published ranked evidence"),
        "{refusal}"
    );
    let completions = identities
        .iter()
        .map(|identity| {
            read(&fixture.0, *identity, LIMIT).map(|saved| saved.map(|saved| saved.completion))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let done = [
        Some(Completion::Completed),
        Some(Completion::Refused),
        Some(Completion::Refused),
    ];
    assert_eq!(completions, done);
    // The journal ends as sequential finishes would leave it: the last refusal.
    let tail = latest(&fixture.0, LIMIT)?.map(|saved| (saved.identity, saved.completion));
    assert_eq!(tail, Some(([63; 32], Completion::Refused)));
    Ok(())
}

#[test]
fn a_superseded_start_inside_a_group_hands_back_only_the_durable_prefix() -> Result<(), String> {
    let fixture = Fixture::new()?;
    // An existing journal: the allocation then takes no directory barrier under its lock.
    completed_earlier(&fixture.0, [80; 32], &mut Published::new())?;
    let root = fixture.0.clone();
    let newer: Rc<RefCell<Option<Attempt>>> = Rc::new(RefCell::new(None));
    let allocated = Rc::new(Cell::new(false));
    let watch = BarrierWatch::modelled({
        let (newer, allocated) = (Rc::clone(&newer), Rc::clone(&allocated));
        move |path| {
            allocated.set(allocated.get() || path.ends_with("attempts.bin"));
            // After the group's allocation, outside every lock, a second writer
            // starts one of the group's identities with a newer token.
            if allocated.get() && path == base(&root) && newer.borrow().is_none() {
                let second =
                    begin(&root, [82; 32], Operation::Sweep).map_err(std::io::Error::other)?;
                *newer.borrow_mut() = Some(second);
            }
            Ok(())
        }
    });
    let mut begun = Vec::new();
    let identities = [[81; 32], [82; 32], [83; 32]];
    let refusal = begin_many(&fixture.0, &identities, Operation::Sweep, &mut begun)
        .err()
        .unwrap_or_default();
    drop(watch);
    assert!(refusal.contains("superseded"), "{refusal}");
    let prefix: Vec<[u8; 32]> = begun.iter().map(Attempt::identity).collect();
    assert_eq!(prefix, [[81; 32]]);
    let first = begun.first().map_or(0, Attempt::token);
    let newer = newer
        .borrow_mut()
        .take()
        .ok_or("the second writer started")?;
    assert_eq!(
        read(&fixture.0, [82; 32], LIMIT)?.map(|saved| saved.attempt),
        Some(newer.token())
    );
    for (identity, token) in [([82; 32], first + 1), ([83; 32], first + 2)] {
        let saved = read_attempt(&fixture.0, identity, token, LIMIT)?.map(|saved| saved.completion);
        assert_eq!(
            saved,
            Some(Completion::Refused),
            "the group's own later attempts are refused"
        );
    }
    newer.finish(Completion::Completed)?;
    finish_many(begun, Completion::Completed)
}

#[test]
fn obstructed_group_paths_refuse_before_any_attempt_is_handed_back() -> Result<(), String> {
    let _quiet = BarrierWatch::modelled(|_| Ok(()));
    let mut begun = Vec::new();
    // An unusable journal: nothing is allocated or reserved.
    let journal = Fixture::new()?;
    fs::create_dir_all(base(&journal.0).join("attempts.bin")).map_err(text)?;
    assert!(begin_many(&journal.0, &[[91; 32]], Operation::Sweep, &mut begun).is_err());
    assert!(begun.is_empty() && !start_path(&journal.0, 1).exists());
    // An identity directory that cannot be created refuses after reservation.
    let blocked = Fixture::new()?;
    fs::create_dir_all(base(&blocked.0)).map_err(text)?;
    fs::write(directory(&blocked.0, &[92; 32]), b"obstruction").map_err(text)?;
    assert!(begin_many(&blocked.0, &[[92; 32]], Operation::Sweep, &mut begun).is_err());
    assert!(begun.is_empty() && start_path(&blocked.0, 1).exists());
    // A lifecycle path that cannot be opened refuses the start itself.
    let lifecycle = Fixture::new()?;
    fs::create_dir_all(directory(&lifecycle.0, &[93; 32]).join("1-lifecycle.bin")).map_err(text)?;
    assert!(begin_many(&lifecycle.0, &[[93; 32]], Operation::Sweep, &mut begun).is_err());
    assert!(begun.is_empty());
    Ok(())
}

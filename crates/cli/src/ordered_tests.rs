#![cfg(test)]
//! The shared journal and ledger order of an `ordered::map` fan-out is a
//! function of the inputs. audit-20261003 hunt-conc-1 and hunt-conc-2, D-1556.
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::sweep_evidence::{self, Completion, Operation};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "brutex-ordered-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("scratch root");
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const WORKERS: u8 = 12;

fn id(worker: u8, step: u8) -> [u8; 32] {
    let mut id = [0; 32];
    id[0] = worker;
    id[1] = step;
    id[31] = 0x5A;
    id
}

/// One whole-command worker: a data-dependent number of attempts, each begun
/// and finished in the shared journal, and one shared ledger row between them.
/// `pause` is how long each step computes, so the caller decides which worker
/// would finish first.
fn worker(root: &Path, worker: u8, pause: Duration) -> Result<u8, String> {
    let outer = sweep_evidence::begin(root, id(worker, 0), Operation::Sweep)?;
    std::thread::sleep(pause);
    for step in 0..worker % 3 {
        let probe = sweep_evidence::begin(root, id(worker, step + 1), Operation::AutoProbe)?;
        std::thread::sleep(pause);
        probe.finish(Completion::Completed)?;
    }
    let mut record = crate::results::Record::from_bytes(&[0; crate::results::STRIDE_BYTES]);
    record.identity = id(worker, 0);
    record.feed = crate::results::field("zerodha");
    record.underlying = crate::results::field("NIFTY");
    record.timeframe = crate::results::field("5min");
    crate::results::with_shared_writer(root, |ledger| ledger.append(&record))?;
    std::thread::sleep(pause);
    outer.finish(Completion::Completed)?;
    Ok(worker)
}

/// Journal rows without their wall-clock stamps, and the ledger identities.
fn durable_order(root: &Path) -> (Vec<Vec<u8>>, Vec<[u8; 32]>) {
    let journal = std::fs::read(
        root.join("results")
            .join("sweep-evidence-v1")
            .join("attempts.bin"),
    )
    .expect("the shared journal");
    let rows = journal[16..]
        .chunks(96)
        .map(|row| [&row[..40], &row[56..58]].concat())
        .collect();
    let mut ledger = crate::results::Results::open_read(root).expect("the shared ledger");
    let filed = (0..ledger.len().expect("rows"))
        .map(|index| ledger.read(index).expect("row").identity)
        .collect();
    (rows, filed)
}

fn fan_out(threads: usize, pause: impl Fn(u8) -> Duration + Sync) -> (Vec<Vec<u8>>, Vec<[u8; 32]>) {
    let scratch = Scratch::new();
    let workers: Vec<u8> = (0..WORKERS).collect();
    let done = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("a pool")
        .install(|| crate::ordered::map(&workers, |w| worker(&scratch.0, *w, pause(*w))))
        .expect("every lane starts");
    let done: Vec<u8> = done.into_iter().map(|w| w.expect("worker")).collect();
    assert_eq!(done, workers, "results come back in input order");
    durable_order(&scratch.0)
}

/// **The journal and the ledger hold the same rows in the same order whatever
/// the thread count and whichever worker computes fastest.**
///
/// Before D-1556 the fan-out was an indexed parallel map, so the report came
/// back in input order while every durable write landed in completion order:
/// one thread filed workers 0, 1, 2…; many threads with the earliest workers
/// slowest filed them roughly backwards.
#[test]
fn shared_durable_writes_follow_the_inputs_not_the_schedule() {
    let one = fan_out(1, |_| Duration::from_millis(2));
    let first_slowest = fan_out(WORKERS.into(), |w| {
        Duration::from_millis(u64::from(WORKERS - w) * 6)
    });
    let last_slowest = fan_out(3, |w| Duration::from_millis(u64::from(w) * 6));
    assert_eq!(one.0.len(), first_slowest.0.len());
    assert_eq!(one.1.len(), usize::from(WORKERS));
    assert_eq!(first_slowest, one, "a fast late worker reordered the store");
    assert_eq!(last_slowest, one, "the thread count reordered the store");
    let rerun = fan_out(WORKERS.into(), |w| {
        Duration::from_millis(u64::from(WORKERS - w) * 6)
    });
    assert_eq!(rerun, one, "a rerun reordered the store");
}

/// The order the module doc states, derived from [`worker`]'s own program
/// order rather than from a run: every lane's shared writes are its events
/// `1, 2, 3, …`, a window's events land sorted by `(k, i)`, and windows land
/// one after another. Returns the journal identities and the ledger identities.
fn stated_order() -> (Vec<[u8; 32]>, Vec<[u8; 32]>) {
    let workers: Vec<u8> = (0..WORKERS).collect();
    let mut journal = Vec::new();
    let mut ledger = Vec::new();
    for window in workers.chunks(crate::ordered::WINDOW) {
        // (round, lane, is the ledger row, identity)
        let mut events = Vec::new();
        for (lane, &w) in window.iter().enumerate() {
            let mut lane_events = vec![(false, id(w, 0))];
            for step in 0..w % 3 {
                lane_events.push((false, id(w, step + 1)));
                lane_events.push((false, id(w, step + 1)));
            }
            lane_events.push((true, id(w, 0)));
            lane_events.push((false, id(w, 0)));
            for (round, (to_ledger, identity)) in lane_events.into_iter().enumerate() {
                events.push((round, lane, to_ledger, identity));
            }
        }
        events.sort_by_key(|&(round, lane, _, _)| (round, lane));
        for (_, _, to_ledger, identity) in events {
            if to_ledger {
                ledger.push(identity);
            } else {
                journal.push(identity);
            }
        }
    }
    (journal, ledger)
}

/// **The order is the INPUT order, not merely a repeatable one.** The test
/// above compares runs only with each other, so lanes that each waited on the
/// HIGHER lanes instead (`other < at` mutated to `other > at` in
/// `Turns::ready`) would land every round in reverse input order, the same on
/// every run, and pass it. P10-02.
#[test]
fn shared_durable_writes_land_round_by_round_in_input_order() {
    let (journal, ledger) = fan_out(WORKERS.into(), |w| {
        Duration::from_millis(u64::from(WORKERS - w) * 3)
    });
    let (stated_journal, stated_ledger) = stated_order();
    assert_eq!(ledger, stated_ledger, "the ledger is not in (k, i) order");
    let identities: Vec<[u8; 32]> = journal
        .iter()
        .map(|row| row[8..40].try_into().expect("32 bytes"))
        .collect();
    assert_eq!(
        identities, stated_journal,
        "the journal is not in (k, i) order"
    );
    // Round one of the first window is every lane's outer begin, lane 0 first.
    let first: Vec<[u8; 32]> = (0..WORKERS)
        .take(crate::ordered::WINDOW)
        .map(|w| id(w, 0))
        .collect();
    assert_eq!(identities[..first.len()], first[..]);
}

/// The body of `fn name` in `source`, up to the next item at column zero.
fn body<'a>(source: &'a str, name: &str) -> &'a str {
    let start = source.find(name).unwrap_or_else(|| panic!("{name} exists"));
    let rest = &source[start..];
    let end = rest.find("\n}\n").expect("the item closes");
    &rest[..end]
}

/// The functions that sweep AND RECORD one rung. Each writes preparation and
/// probe attempts, frontier, trade and receipt blocks and a `runs.bin` row
/// from inside itself, so two in flight at once file them in thread-completion
/// order (GAP13-13) and each takes the whole machine's ceiling and every core
/// (R9-cli-o1-0). D-1701, D-4700.
const RECORDING_KERNELS: [&str; 2] = ["one_rung", "one_rung_cached"];

/// Every spelling in this crate that puts work on another thread.
pub(crate) const PARALLEL: [&str; 12] = [
    "par_iter",
    "par_bridge",
    "par_chunks",
    "par_extend",
    "par_drain",
    "rayon::join",
    "rayon::scope",
    "rayon::spawn",
    "ThreadPoolBuilder",
    "thread::spawn",
    "thread::scope",
    "ordered::map",
];

/// A function that calls a recording rung kernel (level 1) or calls a
/// non-test level-1 caller (level 2), as [`recording_callers`] finds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Caller {
    /// Its file, below `src`.
    pub(crate) file: String,
    pub(crate) name: String,
    /// A `#[test]`, or any function in a test-only file or `mod tests`.
    pub(crate) test: bool,
    pub(crate) level: u8,
    /// From its head line to its closing brace.
    pub(crate) body: String,
}

/// Every `.rs` file under this crate's `src`, as its path below `src` and its
/// text, read when the test runs, in path order. A new file is in the scan the
/// day it is added: there is no list to forget to extend.
pub(crate) fn sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut pending = vec![root.clone()];
    let mut found = Vec::new();
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("a source directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let name = path
                    .strip_prefix(&root)
                    .expect("below src")
                    .to_string_lossy()
                    .into_owned();
                found.push((name, std::fs::read_to_string(&path).expect("a source file")));
            }
        }
    }
    found.sort();
    found
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `line` with its leading visibility and qualifier words removed, so a
/// function head reads `fn name(`.
fn without_qualifiers(line: &str) -> &str {
    let mut rest = line.trim_start();
    loop {
        let before = rest;
        for word in [
            "pub(crate) ",
            "pub(super) ",
            "pub ",
            "const ",
            "async ",
            "unsafe ",
        ] {
            rest = rest.strip_prefix(word).unwrap_or(rest);
        }
        if rest == before {
            return rest;
        }
    }
}

/// The byte offset of each CALL `name(` in `text`: not on a comment line, not
/// inside a string literal on its line, not a definition (`fn name(`), not a
/// method (`.name(`) and not the tail of a longer name.
fn calls(text: &str, name: &str) -> Vec<usize> {
    let needle = format!("{name}(");
    let mut out = Vec::new();
    let mut line_start = 0;
    for line in text.split_inclusive('\n') {
        if !line.trim_start().starts_with("//") {
            let mut from = 0;
            while let Some(found) = line.get(from..).and_then(|rest| rest.find(&needle)) {
                let at = from + found;
                from = at + needle.len();
                let before = &line[..at];
                let quotes = before.matches('"').count() - before.matches("\\\"").count();
                if before
                    .chars()
                    .next_back()
                    .is_some_and(|c| is_ident(c) || c == '.')
                    || before.ends_with("fn ")
                    || quotes % 2 == 1
                {
                    continue;
                }
                out.push(line_start + at);
            }
        }
        line_start += line.len();
    }
    out
}

/// The function whose body holds byte `at` of `text`: its name, whether it is
/// a test, and its text from its head line to its closing brace. A function
/// that closes before `at` (one nested earlier in the same body) is skipped.
fn enclosing(text: &str, at: usize) -> Option<(String, bool, String)> {
    let mut lines = Vec::new();
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        lines.push((start, line));
        start += line.len();
    }
    let index = lines.iter().rposition(|&(start, _)| start <= at)?;
    let test_file = text.starts_with("#![cfg(test)]");
    let test_module = text.find("\nmod tests {").is_some_and(|module| module < at);
    for head in (0..=index).rev() {
        let (head_start, line) = lines[head];
        let Some(after) = without_qualifiers(line).strip_prefix("fn ") else {
            continue;
        };
        let name: String = after.chars().take_while(|&c| is_ident(c)).collect();
        if name.is_empty() {
            continue;
        }
        let indent = &line[..line.len() - line.trim_start().len()];
        let close = format!("{indent}}}");
        let Some(&(end_start, end_line)) = lines
            .get(head + 1..)?
            .iter()
            .find(|(_, l)| l.trim_end() == close)
        else {
            continue;
        };
        if end_start < at {
            continue;
        }
        let marked = lines[..head]
            .iter()
            .rev()
            .map(|(_, l)| l.trim())
            .take_while(|l| !l.is_empty() && *l != "}")
            .any(|l| l == "#[test]");
        let body = text[head_start..end_start + end_line.len()].to_owned();
        return Some((name, marked || test_file || test_module, body));
    }
    None
}

/// Every function in `sources` that calls one of `names`, at `level`.
///
/// # Errors
///
/// A call outside any function: refused rather than skipped.
fn callers_of(
    sources: &[(String, String)],
    names: &[String],
    level: u8,
) -> Result<Vec<Caller>, String> {
    let mut found: Vec<Caller> = Vec::new();
    for (file, text) in sources {
        for name in names {
            for at in calls(text, name) {
                let (caller, test, body) = enclosing(text, at)
                    .ok_or_else(|| format!("{file}: a call of {name} outside any fn"))?;
                let row = Caller {
                    file: file.clone(),
                    name: caller,
                    test,
                    level,
                    body,
                };
                if !found.contains(&row) {
                    found.push(row);
                }
            }
        }
    }
    Ok(found)
}

/// Every caller of a [`RECORDING_KERNELS`] function in `sources`: level 1.
///
/// # Errors
///
/// As [`callers_of`].
pub(crate) fn kernel_callers(sources: &[(String, String)]) -> Result<Vec<Caller>, String> {
    let kernels: Vec<String> = RECORDING_KERNELS.iter().map(|k| (*k).to_owned()).collect();
    callers_of(sources, &kernels, 1)
}

/// Every caller of each non-test level-1 caller that is not itself a kernel:
/// level 2. Found by scanning, so a new verb, file or wrapper is checked the
/// day it lands.
///
/// # Errors
///
/// A wrapper whose name is defined in more than one file, so its callers
/// cannot be told apart by name: refused rather than skipped.
pub(crate) fn wrapper_callers(
    sources: &[(String, String)],
    level_one: &[Caller],
) -> Result<Vec<Caller>, String> {
    let mut wrappers: Vec<String> = level_one
        .iter()
        .filter(|c| !c.test && !RECORDING_KERNELS.contains(&c.name.as_str()))
        .map(|c| c.name.clone())
        .collect();
    wrappers.sort();
    wrappers.dedup();
    for wrapper in &wrappers {
        let head = format!("fn {wrapper}(");
        let generic = format!("fn {wrapper}<");
        let defined: Vec<&str> = sources
            .iter()
            .filter(|(_, text)| {
                text.lines().any(|line| {
                    let line = without_qualifiers(line);
                    line.starts_with(&head) || line.starts_with(&generic)
                })
            })
            .map(|(file, _)| file.as_str())
            .collect();
        if defined.len() != 1 {
            return Err(format!(
                "`{wrapper}` calls a recording rung kernel and is defined in {defined:?}; \
                 a wrapper needs a crate-unique name so its callers can be checked"
            ));
        }
    }
    callers_of(sources, &wrappers, 2)
}

/// [`kernel_callers`] then [`wrapper_callers`].
///
/// # Errors
///
/// As either.
pub(crate) fn recording_callers(sources: &[(String, String)]) -> Result<Vec<Caller>, String> {
    let mut found = kernel_callers(sources)?;
    let more = wrapper_callers(sources, &found)?;
    found.extend(more);
    Ok(found)
}

/// The first [`PARALLEL`] spelling in `body`, if any.
pub(crate) fn parallel_in(body: &str) -> Option<&'static str> {
    PARALLEL.iter().copied().find(|token| body.contains(token))
}

/// **Every function in this crate that calls a recording rung kernel, and
/// every function that calls one of those, runs it on the calling thread:**
/// no parallel spelling anywhere in its body, and no level-1 production
/// caller raises `SharedBy`. Found by scanning every file under `src`, not a
/// list, so `pool-oos`'s pass 1 (a rayon parallel map over the surface around
/// `one_rung`, G1-1) fails here, and so would the next verb that copies it.
/// GAP13-13, R9-cli-o1-0, D-1701, D-4700.
#[test]
fn every_caller_of_a_recording_rung_kernel_runs_it_in_input_order() {
    let sources = sources();
    let level_one = kernel_callers(&sources).unwrap_or_else(|why| panic!("{why}"));
    let mut callers = level_one.clone();
    for caller in &level_one {
        assert_eq!(
            parallel_in(&caller.body),
            None,
            "{} `{}` calls a recording rung kernel and spells parallel work",
            caller.file,
            caller.name
        );
    }
    callers.extend(wrapper_callers(&sources, &level_one).unwrap_or_else(|why| panic!("{why}")));
    for caller in &callers {
        assert_eq!(
            parallel_in(&caller.body),
            None,
            "{} `{}` (level {}) calls a recording rung kernel and spells parallel work",
            caller.file,
            caller.name,
            caller.level
        );
        if caller.level == 1 && !caller.test {
            assert!(
                !caller.body.contains("SharedBy::these"),
                "{} `{}`: one sweep in flight shares nothing",
                caller.file,
                caller.name
            );
        }
    }
    let has = |file: &str, name: &str, level: u8| {
        callers
            .iter()
            .any(|c| c.file == file && c.name == name && c.level == level && !c.test)
    };
    assert!(has("lib.rs", "sweep_rungs", 1), "{callers:#?}");
    assert!(has("lib.rs", "one_rung", 1), "{callers:#?}");
    assert!(has("lib.rs", "descend_in", 1), "{callers:#?}");
    assert!(has("pool.rs", "screen_pass_one", 1), "{callers:#?}");
    assert!(
        has("pool.rs", "run_under", 2),
        "pool pass 1 is the shared one"
    );
    assert!(has("pool_oos.rs", "run_under", 2), "pool-oos pass 1 is too");
}

/// The scan finds a parallel caller, skips comments, strings, methods,
/// definitions and an earlier nested function, follows a wrapper one level
/// up, and refuses a wrapper whose name is not unique.
#[test]
fn the_recording_caller_scan_sees_calls_and_nothing_else() {
    let kernel = concat!("one_rung", "(");
    let wrap = concat!("wrapper", "(");
    let file = format!(
        "fn wrapper(items: &[u8]) {{\n    \
         fn nested() {{}}\n    \
         // {kernel}x) in a comment\n    \
         let s = \"{kernel}\";\n    \
         x.{kernel}1);\n    \
         items.par_iter().map(|i| {kernel}i));\n\
         }}\n\
         fn outer() {{\n    \
         {wrap}&[]);\n\
         }}\n\
         #[test]\n\
         fn a_test() {{\n    \
         crate::{kernel}1);\n\
         }}\n"
    );
    let callers = recording_callers(&[("x.rs".to_owned(), file.clone())]).expect("scanned");
    let named: Vec<(&str, u8, bool)> = callers
        .iter()
        .map(|c| (c.name.as_str(), c.level, c.test))
        .collect();
    assert_eq!(
        named,
        [
            ("wrapper", 1, false),
            ("a_test", 1, true),
            ("outer", 2, false)
        ]
    );
    assert_eq!(parallel_in(&callers[0].body), Some("par_iter"));
    assert_eq!(parallel_in(&callers[1].body), None);
    assert_eq!(parallel_in(&callers[2].body), None);
    let twice = [
        ("x.rs".to_owned(), file),
        ("y.rs".to_owned(), "fn wrapper() {\n}\n".to_owned()),
    ];
    assert!(
        recording_callers(&twice).is_err_and(|why| why.contains("crate-unique")),
        "an ambiguous wrapper is refused"
    );
    let quiet = "fn f() {\n    // one_rung(\n    let s = \"one_rung(\";\n}\n";
    assert!(
        recording_callers(&[("q.rs".to_owned(), quiet.to_owned())])
            .expect("scanned")
            .is_empty()
    );
}

/// **Every whole-command fan-out that writes the shared journal or ledger
/// writes in input order: the Boolean family pools as ordered lanes (D-1556),
/// `range-all` and pool pass 1 -- `pool`'s and `pool-oos`'s, one shared
/// `screen_pass_one` -- one call at a time through `in_input_order` (D-1701,
/// kept over D-1556 for those two by D-1709; D-4700).** None is an indexed
/// parallel map or a private thread pool. Every OTHER caller of a recording
/// rung kernel is found by scanning, in
/// `every_caller_of_a_recording_rung_kernel_runs_it_in_input_order`.
#[test]
fn every_whole_command_fan_out_writes_in_input_order() {
    for (file, source, name, shape) in [
        (
            "lib.rs",
            include_str!("lib.rs"),
            "fn sweep_rungs(",
            "in_input_order(rungs,",
        ),
        (
            "pool.rs",
            include_str!("pool.rs"),
            "fn screen_pass_one(",
            "crate::in_input_order(surface,",
        ),
        (
            "boolean_catalog_prepared.rs",
            include_str!("boolean_catalog_prepared.rs"),
            "pub(crate) fn run(",
            "ordered::map(",
        ),
        (
            "boolean_oos_command.rs",
            include_str!("boolean_oos_command.rs"),
            "fn execute(",
            "ordered::map(",
        ),
    ] {
        let body = body(source, name);
        assert!(body.contains(shape), "{file} {name}");
        assert!(!body.contains("ThreadPoolBuilder"), "{file} {name}");
        let pass_one = body.split("PASS 2").next().unwrap_or(body);
        assert!(!pass_one.contains("par_iter"), "{file} {name}");
    }
}

/// A turn outside any lane, and a turn nested inside one held, return at once.
#[test]
fn a_turn_outside_a_lane_or_inside_a_held_one_never_waits() {
    let outer = crate::ordered::turn();
    let inner = crate::ordered::turn();
    drop(inner);
    drop(outer);
    let nested = crate::ordered::map(&[0_u8, 1], |lane| {
        let _outer = crate::ordered::turn();
        let _inner = crate::ordered::turn();
        *lane
    })
    .expect("both lanes start");
    assert_eq!(nested, vec![0, 1]);
}

/// **A lane's own slot never blocks it, and a lower lane must be a round
/// ahead.** G18-cli-b-02, D-2021.
///
/// Fails fast where the fan-out tests would hang: a lane that compared its own
/// slot as a LOWER one (`other < at` widened to `other <= at`) would wait for
/// itself forever. Checked on the pure rule, no thread involved.
#[test]
fn a_lane_never_waits_on_its_own_slot() {
    use super::{Lane, Turns};
    let idle = Lane {
        performed: 0,
        finished: false,
    };
    assert!(Turns::ready(&[idle], 0), "a lone lane goes");
    assert!(Turns::ready(&[idle, idle], 0), "lane 0 leads round 1");
    assert!(
        !Turns::ready(&[idle, idle], 1),
        "lane 1 waits for lane 0's round 1"
    );
    let ahead = Lane {
        performed: 1,
        finished: false,
    };
    assert!(Turns::ready(&[ahead, idle], 1), "lane 0 has done round 1");
    assert!(
        !Turns::ready(&[ahead, idle], 0),
        "lane 0's round 2 waits for lane 1's round 1"
    );
    let saturated = Lane {
        performed: u64::MAX,
        finished: false,
    };
    assert!(
        Turns::ready(&[saturated], 0),
        "a saturated count still goes"
    );
    assert!(
        Turns::ready(&[idle, idle], 2),
        "a lane past the end is not held"
    );
}

/// **Every lane gets its turns in round-then-input order, and a lane that
/// finishes early never holds the others, within a deadline.**
/// G18-cli-b-26, D-2033.
///
/// The fan-out runs on its own thread and its answer is awaited with
/// `recv_timeout`, so a turn that never comes (an update that changes nothing,
/// a finished lane never marked finished, a wait taken when ready) fails this
/// test at the deadline instead of hanging the suite.
#[test]
fn turns_are_granted_in_round_then_input_order_within_a_deadline() {
    use std::sync::{Arc, Mutex, mpsc};
    let log = Arc::new(Mutex::new(Vec::new()));
    let (send, receive) = mpsc::channel();
    let lanes = Arc::clone(&log);
    std::thread::spawn(move || {
        // Lane 0 takes one turn and finishes; lanes 1-3 take three each.
        let items: Vec<u8> = (0..4).collect();
        let done = crate::ordered::map(&items, |lane| {
            let rounds = if *lane == 0 { 1 } else { 3 };
            for round in 0..rounds {
                let _turn = crate::ordered::turn();
                lanes
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push((round, *lane));
            }
            *lane
        });
        let _ = send.send(done);
    });
    let done = receive
        .recv_timeout(Duration::from_secs(30))
        .expect("every lane was granted its turns before the deadline")
        .expect("every lane starts");
    assert_eq!(done, [0, 1, 2, 3], "results come back in input order");
    let log = log
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    assert_eq!(
        log,
        [
            (0, 0),
            (0, 1),
            (0, 2),
            (0, 3),
            (1, 1),
            (1, 2),
            (1, 3),
            (2, 1),
            (2, 2),
            (2, 3),
        ],
        "round by round, lanes in input order"
    );
}

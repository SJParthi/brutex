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

/// **Every whole-command fan-out that writes the shared journal or ledger is an
/// ordered lane, not an indexed parallel map.** D-1556.
#[test]
fn every_whole_command_fan_out_is_an_ordered_lane() {
    for (file, source, name) in [
        ("lib.rs", include_str!("lib.rs"), "fn sweep_rungs("),
        ("pool.rs", include_str!("pool.rs"), "fn run_under("),
        (
            "boolean_catalog_prepared.rs",
            include_str!("boolean_catalog_prepared.rs"),
            "pub(crate) fn run(",
        ),
        (
            "boolean_oos_command.rs",
            include_str!("boolean_oos_command.rs"),
            "fn execute(",
        ),
    ] {
        let body = body(source, name);
        assert!(body.contains("ordered::map("), "{file} {name}");
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

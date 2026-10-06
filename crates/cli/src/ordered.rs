//! Whole-command workers whose shared durable writes land in an order fixed by
//! the inputs, not by the thread schedule. audit-20261003 hunt-conc-1 and
//! hunt-conc-2, D-1556.
//!
//! # The defect
//!
//! `range-all`, `pool` pass 1 and the Boolean family pools each ran one whole
//! command per item under an indexed parallel map. The report came back in
//! input order, but every shared durable write inside an item, the evidence
//! journal's attempt tokens and terminals and the run ledger's rows, landed in
//! whichever order the threads reached it. `sweep-all` was split into ordered
//! phases (D-1564, kept as D-1701's chunks by D-1708); these run whole audit
//! and candidate transactions, so the order is imposed at the shared writes
//! themselves instead.
//!
//! # Who uses it
//!
//! The Boolean family pools. `range-all` and `pool` pass 1 run their rungs and
//! instruments one at a time through `crate::in_input_order` instead (D-1701,
//! kept for those two by D-1709): eight lanes in flight would have to divide
//! the machine's ceiling by eight, and the run identity folds the ceiling, so
//! each rung would record a different run from the same rung run alone.
//!
//! # The order
//!
//! Every item is a lane. A lane's shared writes are its events `1, 2, 3, …` in
//! program order. Event `k` of lane `i` waits until every lower lane has
//! performed its event `k` or finished, and every higher lane its event `k-1`
//! or finished. So the writes land sorted by `(k, i)`: round `k` of every lane
//! in input order, then round `k+1`. That order is a function of what each
//! lane writes, which is a function of its inputs, and of nothing else.
//!
//! # Why it cannot deadlock
//!
//! Every wait is for an event strictly smaller in `(k, i)` order, so the
//! smallest pending event never waits and always proceeds. Two conditions keep
//! that argument true, and both are structural:
//!
//! * **Every lane runs on its own scoped OS thread**, never as a Rayon task.
//!   A waiting Rayon task can be stacked under a stolen higher item that waits
//!   for it; an OS thread holds only its own lane. Nested parallel work inside
//!   a lane still goes to the Rayon pool, which never waits on a turn.
//! * **Lanes are admitted a window at a time.** A higher lane that never
//!   started could hold back a lower lane's next round forever, so every lane
//!   of a window is started at once and the next window begins only after the
//!   last lane of this one has returned. [`WINDOW`] is a constant, not the
//!   machine's core count, so the order does not depend on the machine.
//!
//! A turn is held across one whole durable transaction and is re-entrant on its
//! own thread, so a transaction that nests another shared write is one event.
//! The gate is taken before any lock a lower lane might need. Outside
//! [`map`], [`turn`] never waits.
//!
//! # Cost
//!
//! Compute runs in parallel; only the shared writes are ordered, and a lane may
//! wait at a write for a slower lane to reach the same round. A window costs at
//! most its slowest lane's compute plus every round's waits, and windows run
//! one after another. Stated in `docs/06-limits.md`.

use std::cell::{Cell, RefCell};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

/// Lanes admitted at once. A constant, so the order is the same on every
/// machine.
pub(crate) const WINDOW: usize = 8;

/// Bytes of stack each lane thread gets: the main thread's customary size, so
/// a command run as a lane has the depth it has when run alone.
const LANE_STACK: usize = 8 << 20;

#[derive(Clone, Copy, Default)]
struct Lane {
    performed: u64,
    finished: bool,
}

struct Turns {
    lanes: Mutex<Vec<Lane>>,
    moved: Condvar,
}

impl Turns {
    fn lanes(&self) -> MutexGuard<'_, Vec<Lane>> {
        self.lanes.lock().unwrap_or_else(PoisonError::into_inner)
    }
    /// Whether lane `at` may perform its next event now.
    fn ready(lanes: &[Lane], at: usize) -> bool {
        let Some(own) = lanes.get(at) else {
            return true;
        };
        let round = own.performed.saturating_add(1);
        lanes.iter().enumerate().all(|(other, lane)| {
            lane.finished
                || other == at
                || lane.performed >= if other < at { round } else { round - 1 }
        })
    }
    fn update(&self, at: usize, change: impl FnOnce(&mut Lane)) {
        if let Some(lane) = self.lanes().get_mut(at) {
            change(lane);
        }
        self.moved.notify_all();
    }
}

std::thread_local! {
    static LANE: RefCell<Option<(Arc<Turns>, usize)>> = const { RefCell::new(None) };
    static HELD: Cell<bool> = const { Cell::new(false) };
}

/// Marks a lane finished however its work ends, unwinding included.
struct Finished(Arc<Turns>, usize);
impl Drop for Finished {
    fn drop(&mut self) {
        LANE.with(|lane| lane.borrow_mut().take());
        self.0.update(self.1, |lane| lane.finished = true);
    }
}

/// Runs `work` over `items` and returns the results in input order, with every
/// shared durable write ordered as the module doc states.
///
/// # Errors
///
/// A lane thread the OS would not start. Its window's other lanes still run to
/// the end, so nothing is left half-begun, and the refusal names the item.
pub(crate) fn map<T: Sync, R: Send>(
    items: &[T],
    work: impl Fn(&T) -> R + Sync,
) -> Result<Vec<R>, String> {
    let mut out = Vec::with_capacity(items.len());
    let mut refused = None;
    for (window, chunk) in items.chunks(WINDOW).enumerate() {
        let turns = Arc::new(Turns {
            lanes: Mutex::new(vec![Lane::default(); chunk.len()]),
            moved: Condvar::new(),
        });
        let work = &work;
        std::thread::scope(|scope| {
            let lanes: Vec<_> = chunk
                .iter()
                .enumerate()
                .map(|(at, item)| {
                    let lane = Arc::clone(&turns);
                    let spawned = std::thread::Builder::new()
                        .name(format!("brutex-ordered-{at}"))
                        .stack_size(LANE_STACK)
                        .spawn_scoped(scope, move || {
                            LANE.with(|held| *held.borrow_mut() = Some((Arc::clone(&lane), at)));
                            let _finished = Finished(lane, at);
                            work(item)
                        });
                    if spawned.is_err() {
                        // Never started, so it writes nothing: mark it finished
                        // so no other lane waits for it.
                        turns.update(at, |lane| lane.finished = true);
                    }
                    (at, spawned)
                })
                .collect();
            for (at, lane) in lanes {
                match lane {
                    Ok(handle) => match handle.join() {
                        Ok(done) => out.push(done),
                        Err(unwound) => std::panic::resume_unwind(unwound),
                    },
                    Err(why) => {
                        refused.get_or_insert_with(|| {
                            format!(
                                "ordered worker {} could not be started: {why}",
                                window.saturating_mul(WINDOW).saturating_add(at)
                            )
                        });
                    }
                }
            }
        });
        if let Some(why) = refused {
            return Err(why);
        }
    }
    Ok(out)
}

/// The right to perform one shared durable write: held for the whole
/// transaction, released on drop.
pub(crate) struct Turn(Option<(Arc<Turns>, usize)>);

impl Drop for Turn {
    fn drop(&mut self) {
        if let Some((turns, at)) = self.0.take() {
            HELD.with(|held| held.set(false));
            turns.update(at, |lane| lane.performed = lane.performed.saturating_add(1));
        }
    }
}

/// Waits until this thread's lane may perform its next shared write.
///
/// Returns at once outside [`map`], and inside a turn this thread already
/// holds: a nested write is part of the outer event.
#[must_use]
pub(crate) fn turn() -> Turn {
    if HELD.with(Cell::get) {
        return Turn(None);
    }
    let Some((turns, at)) = LANE.with(|lane| lane.borrow().clone()) else {
        return Turn(None);
    };
    let mut lanes = turns.lanes();
    while !Turns::ready(&lanes, at) {
        lanes = turns
            .moved
            .wait(lanes)
            .unwrap_or_else(PoisonError::into_inner);
    }
    drop(lanes);
    HELD.with(|held| held.set(true));
    Turn(Some((turns, at)))
}

#[cfg(test)]
#[path = "ordered_tests.rs"]
mod tests;

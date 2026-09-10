//! Bounded boundary observations; no event is emitted per bar or trade.
use super::{Link, PreparedSources, Progress, Request, display, run_rung};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{SyncSender, sync_channel},
};

/// Actual current boundary for one selected physical timeframe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RungStage {
    /// Strict source/context preparation or a reserved batch waiting for its worker.
    Preparing,
    /// Original training executions are being computed and saved.
    Training,
    /// Fixed later-day executions retain original-first indicator history.
    Later,
    /// Existing institutional statistics and index day/week checks are running.
    Institutional,
    /// This timeframe's candidate and qualification artifacts were saved.
    Saved,
    /// This timeframe's current boundary returned an explicit error.
    Refused,
}
impl RungStage {
    /// Stable browser and structural-log label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Preparing => "preparing",
            Self::Training => "training",
            Self::Later => "later",
            Self::Institutional => "institutional",
            Self::Saved => "saved",
            Self::Refused => "refused",
        }
    }
}

/// Additive observation preserves the absence of a source-bound identity while
/// source preparation is still underway.
#[derive(Clone, Debug, PartialEq, Eq)]
#[expect(
    clippy::large_enum_variant,
    reason = "fixed eight-rung structural snapshots avoid a heap allocation per boundary and never enter the bar loop"
)]
pub enum Observation {
    /// Source/context boundary before an immutable search identity exists.
    Preparing {
        /// Selected physical intraday timeframe slot.
        rung: usize,
        /// Preparation or explicit refusal, never claimed execution progress.
        stage: RungStage,
    },
    /// Exact declaration plus acknowledged counters and current boundary states.
    Search(Progress),
}

pub(super) fn preparing(
    observe: &mut dyn FnMut(Observation) -> Result<(), String>,
    rung: usize,
    stage: RungStage,
) -> Result<(), String> {
    note(None, None, rung, stage);
    observe(Observation::Preparing { rung, stage })
}

/// One coordinator drains at most one queued boundary per configured timeframe.
/// Observer failure cancels at the next boundary and joins every worker before
/// returning, so the outer execution lease cannot be released while work lives.
pub(super) fn parallel(
    request: &Request<'_>,
    prepared: &[PreparedSources<'_>],
    pool: &rayon::ThreadPool,
    programs: &[runner::expression::Expression],
    snapshot: &mut Progress,
    observe: &mut dyn FnMut(Observation) -> Result<(), String>,
) -> Result<Vec<Result<Link, String>>, String> {
    use rayon::prelude::*;
    let batch = snapshot
        .current_batch
        .ok_or("single-stop worker lacks its reserved batch")?;
    let identity = snapshot.identity;
    let cancelled = AtomicBool::new(false);
    let (sender, receiver) = sync_channel(prepared.len().max(1));
    std::thread::scope(|scope| {
        let cancelled_ref = &cancelled;
        let worker = std::thread::Builder::new()
            .name("index-stop-boundaries".into())
            .spawn_scoped(scope, move || {
                pool.install(|| {
                    prepared
                        .par_iter()
                        .map(|source| {
                            let boundary = Boundary {
                                sender: &sender,
                                cancelled: cancelled_ref,
                                rung: source.sources.rung,
                            };
                            let result =
                                run_rung(request, source, identity, batch, programs, &boundary);
                            let final_stage = if result.is_ok() {
                                RungStage::Saved
                            } else {
                                RungStage::Refused
                            };
                            let recorded = boundary.send(final_stage);
                            match (result, recorded) {
                                (Ok(link), Ok(())) => Ok(link),
                                (Err(why), _) | (_, Err(why)) => Err(why),
                            }
                        })
                        .collect::<Vec<_>>()
                })
            })
            .map_err(display)?;
        let mut failure = None;
        for (rung, stage) in receiver {
            let result = (|| {
                let slot = snapshot
                    .rung_stages
                    .get_mut(rung)
                    .filter(|slot| slot.is_some())
                    .ok_or("single-stop progress contains an unselected timeframe")?;
                *slot = Some(stage);
                note(Some(identity), Some(batch), rung, stage);
                observe(Observation::Search(snapshot.clone()))
            })();
            if let Err(why) = result {
                failure = Some(why);
                cancelled.store(true, Ordering::Release);
                break;
            }
        }
        let result = worker.join().map_err(|_| {
            "single-stop worker stopped unexpectedly; pending checkpoint retained".to_owned()
        });
        if let Some(why) = failure {
            return Err(why);
        }
        result
    })
}

pub(super) struct Boundary<'a> {
    sender: &'a SyncSender<(usize, RungStage)>,
    cancelled: &'a AtomicBool,
    rung: usize,
}
impl Boundary<'_> {
    pub(super) fn send(&self, stage: RungStage) -> Result<(), String> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err("single-stop progress observer refused; no next stage starts".into());
        }
        self.sender.send((self.rung, stage)).map_err(|_| {
            "single-stop progress observer is unavailable; no next stage starts".into()
        })
    }
}

fn note(identity: Option<[u8; 32]>, batch: Option<u64>, rung: usize, stage: RungStage) {
    let Some(label) = crate::ledger_all::LEDGER_RUNGS.get(rung) else {
        return;
    };
    let mut event = telemetry::Event::info("cli.index-stop", "single-stop research boundary")
        .with("stage", stage.as_str())
        .with("rung", *label);
    if let Some(batch) = batch {
        event = event.with("batch", batch);
    }
    let encoded = identity.map(|id| crate::identity_hex(&id));
    if let Some(id) = encoded.as_ref() {
        event = event.with("identity", id.as_str());
    }
    // The coordinator retains the outer browser operation's audit context.
    crate::note_attempt(crate::binding_attempt(), &event);
}

#[cfg(test)]
#[path = "index_stop_search_progress_tests.rs"]
mod tests;

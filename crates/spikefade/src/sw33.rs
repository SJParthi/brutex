//! v3.3 SPEED SWITCHES (MAIN-RULINGS-ENGINE-v22 Addendum 28.2; job v33-fast, agent build:v33; the order of v32/speed/SPEED-PLAN.md).
//!
//! Every switch here is an EXACT speed-up or memory cut: with it on, every result file of a run is byte for byte the one it gives off
//! (proven on test slices before use; named tests + one killed mutant each). Each defaults to OFF, so a v3.3 binary with no v3.3 flag
//! replays round B's FINAL binary (bf337da9...) byte for byte (proof P0). `run3 --v33` turns every one on; each can be set alone.
//!
//! The switches never print into RESULTS (they change no result); RUN.log prints their words and the day-major checkpoint fingerprint
//! names them (a resume never mixes two state layouts, batch orders or list stores).
//!
//! Process-wide in the binary (set once from the command line before any work); under `cfg(test)` each test thread has its own copy.

/// A v3.3 switch (an index into the v3.3 switch table).
pub type Sw33 = usize;
/// C2 (DESIGN-D1): the per-config `facts` are never built in run3 (write-only there), and each day's screener output is merged as soon as
/// its chunk of days is prepared (the set-up never holds every day's per-config tuples at once).
pub const NOFACTS: Sw33 = 0;
/// C1 (DESIGN-D1): day-major account states only for the WALKED share levels (`--shares`), not all 5.
pub const COMPACT: Sw33 = 1;
/// C3 (DESIGN-D1, the plain-u32 form): the unique lists are DERIVED from the per-config lists (the representative pair's list filtered by
/// its liquidity value: `build_lists`' own definition); `ul_evs` is never stored for the period.
pub const DERIVED: Sw33 = 2;
/// D3 3.9a (DESIGN-D3): each day's work items claimed LARGEST FIRST (the day's sub-list length, ties by list index) instead of list order.
pub const LARGEST: Sw33 = 3;
/// D3 3.5 (DESIGN-D3): `walk_day`'s per-call position buffer hoisted to one per worker thread (positions never cross a day).
pub const HOIST: Sw33 = 4;
/// C6 / D2-1a (DESIGN-D1, DESIGN-D2): the batches take the unique lists in CONFIG-SORTED order for INDEX options (creation order for stock,
/// where the liquidity value is the strongest filter); every list keeps its id.
pub const ORDER: Sw33 = 5;
/// C7a (DESIGN-D1): the next day's cube and seconds tape are loaded on a loader thread while the current day is priced and walked.
pub const PREFETCH: Sw33 = 6;
/// D2-1 (DESIGN-D2, SPEED-PLAN S2): day-class ghost traces: lists with the same day sub-list share ONE ghost walk per account slot; each
/// account then replays the ghost's trace through the real flush / pieces / segment-series hooks when its start-of-day realized net is at or
/// above the trace's threshold R*, else it is walked as before.
pub const DAYCLASS: Sw33 = 7;
/// C4a (DESIGN-D1): the liquidity table (Table L) summed ONLINE at each event's counted pricing (integer sums, every event counted exactly
/// once in the day-major mode) and each event's per-level fill list dropped after it (250-380 B an event; its only reader is Table L).
pub const TABLEL: Sw33 = 8;
/// v3.3 (build:v33, after the MEASURE33 month runs): stage A work items of [`ACHUNK_N`] contract(s) instead of round B's 64. MEASURED on
/// GDFL 2022-07 (SEARCH1D, 12 threads): a day-major batch-day's pricing ran on 0.7 (index) to 1.1 (stock) cores, because an index day has
/// about 65 contracts with events (1 or 2 items of 64). Exact: the items are cut at contract boundaries only and appended in order, so every
/// table, mark, counter and record is the same (named test `v33_achunk_same_sequence_whole_contracts`).
pub const ACHUNK: Sw33 = 9;
/// Contracts per stage A work item under [`ACHUNK`] (each contract's tape, trail index and marks are built once per item either way).
pub const ACHUNK_N: usize = 1;
const N: usize = 10;
/// The number of v3.3 switches (tests reset every one).
pub const COUNT: usize = N;
/// `--measure33` (a MEASUREMENT option, never a result switch): the day-major stage B samples the process CPU around each day's stage A
/// and walk and prints one MEASURE33 line per batch in RUN.log (about 2 `ps` calls a day).
pub static MEASURE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(not(test))]
static FLAGS: [std::sync::atomic::AtomicBool; N] = [const { std::sync::atomic::AtomicBool::new(false) }; N];
#[cfg(test)]
thread_local! {
    static FLAGS: [std::cell::Cell<bool>; N] = const { [const { std::cell::Cell::new(false) }; N] };
}

/// One v3.3 switch's value.
#[inline]
#[must_use]
pub fn on(s: Sw33) -> bool {
    #[cfg(not(test))]
    {
        FLAGS.get(s).is_some_and(|f| f.load(std::sync::atomic::Ordering::Relaxed))
    }
    #[cfg(test)]
    {
        FLAGS.with(|f| f.get(s).is_some_and(std::cell::Cell::get))
    }
}

/// Sets one v3.3 switch.
pub fn set(s: Sw33, v: bool) {
    #[cfg(not(test))]
    {
        if let Some(f) = FLAGS.get(s) {
            f.store(v, std::sync::atomic::Ordering::Relaxed);
        }
    }
    #[cfg(test)]
    {
        FLAGS.with(|f| {
            if let Some(c) = f.get(s) {
                c.set(v);
            }
        });
    }
}

/// Whether any v3.3 switch is on.
#[must_use]
pub fn any() -> bool {
    (0..N).any(on)
}

/// The v3.3 switch words (RUN.log and the checkpoint fingerprint only; empty when none is on).
#[must_use]
pub fn words() -> String {
    if !any() {
        return String::new();
    }
    let w = |s: Sw33, a: &str, b: &str| if on(s) { a.to_owned() } else { b.to_owned() };
    [
        w(NOFACTS, "C2 no facts + per-chunk merge", "facts as round B"),
        w(COMPACT, "C1 compact states (walked share levels only)", "states for all 5 share levels"),
        w(DERIVED, "C3 unique lists derived from the per-config lists (no ul_evs)", "ul_evs stored"),
        w(LARGEST, "largest-first per-day queue", "list-order queue"),
        w(HOIST, "position buffer hoisted per thread", "position buffer per call"),
        w(ORDER, "batch order config-sorted (index) / creation (stock)", "batch order creation"),
        w(PREFETCH, "next day's cube prefetched", "cube loaded on demand"),
        w(DAYCLASS, "D2-1 day-class ghost traces (O(trades) replay through the real hooks)", "no day classes"),
        w(TABLEL, "C4a Table L summed online (fill lists dropped)", "Table L from every event's fill list"),
        w(ACHUNK, "stage A items of 1 contract", "stage A items of 64 contracts"),
    ]
    .join("; ")
}

/// Reads the v3.3 switches (`--v33` = every one; each flag may override).
///
/// # Errors
/// A switch value that is not one of its words, or a switch that needs another.
pub fn from_args(a: &[String]) -> Result<(), String> {
    let all = a.iter().any(|x| x == "--v33");
    MEASURE.store(a.iter().any(|x| x == "--measure33"), std::sync::atomic::Ordering::Relaxed);
    let flag = |k: &str, yes: &str, no: &str| -> Result<Option<bool>, String> {
        match a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str) {
            None => Ok(None),
            Some(v) if v == yes => Ok(Some(true)),
            Some(v) if v == no => Ok(Some(false)),
            Some(v) => Err(format!("{k} {yes}|{no}, not {v}")),
        }
    };
    for (s, k, yes, no) in [
        (NOFACTS, "--facts", "drop", "keep"),
        (COMPACT, "--states", "compact", "full"),
        (DERIVED, "--ul", "derived", "stored"),
        (LARGEST, "--queue", "largest", "list"),
        (HOIST, "--hoist", "on", "off"),
        (ORDER, "--batch-order", "sorted", "creation"),
        (PREFETCH, "--prefetch", "on", "off"),
        (DAYCLASS, "--day-class", "on", "off"),
        (TABLEL, "--table-l", "online", "events"),
        (ACHUNK, "--stage-a-items", "small", "64"),
    ] {
        set(s, flag(k, yes, no)?.unwrap_or(all));
    }
    Ok(())
}

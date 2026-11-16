//! v3.4 (round D, build:D-high, 16 Nov 2026; MAIN-RULINGS-ENGINE-v22 Addendum 30: NO accepted gaps): the HIGH items of the v3.3 gates
//! (`v32/audit-v33/STANDING-SCORECARD.md`, `v32/exits-audit-v33/EXITS-SCORECARD.md`) and of `v33/RECHECK-V33.md` that can change a result,
//! plus the quick result-neutral HIGH items.
//!
//! Like the `fix33` / `fixg` switches these CHANGE results (except where a switch says "result-neutral"). Each defaults to OFF, so with every
//! one off the binary replays v3.3 final (8d9a5b67...) byte for byte (proof P0). `run3 --fix34` turns on every switch that needs no MAIN
//! ruling (the DIALS below stay off until their own flag names a value: a ruling is owed before a dial drives a headline); each switch has
//! its own flag. Their words go to RUN.log, to the RESULTS header and to the day-major checkpoint fingerprint (empty when none is on, so
//! every v3.3 print stays byte for byte). Process-wide in the binary (set once from the command line before any work); under `cfg(test)`
//! each test thread has its own copy.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::cube::Cube;

/// A v3.4 switch (an index into the switch table).
pub type Fx = usize;
/// MATRIX X20 / STANDING F6 remainder (round C's CAP20, ported): the Rs 20 L cap at EVERY trading minute of EVERY account: per minute b
/// (09:15 + b, b < 375) margin in use + the running MTM loss (minute-HIGH marks, the X22 rule) above Rs 20,00,000 is counted IN THE KERNEL
/// for every account (an O(1) bound skips a range that cannot pass the cap; otherwise the minutes are evaluated exactly), and an account
/// with such a minute is KEPT OUT of every view, of every window row whose learning years hold one (a row whose test years hold one is NOT
/// PASSED) and of every split whose learning months touch one (a split whose test months touch one fails). D2-1 replays carry the ghost
/// day's count (the count reads only the day's positions: identical for every class member at or above R*).
pub const CAP20: Fx = 0;
/// STANDING F17 / EXITS F-H3 / MATRIX X09 (+ the counted refusal of EXITS F-M1): the dated halt / pause table (`--halts FILE`): a session
/// holding a market-wide halt gets its tape in SEGMENTS (the crate's segment walk: no block spans the halt, the carried entitlement resets
/// at it, a stop decided across it buys back at the first post-halt block's worst), and an event decided inside a halt is refused
/// (`halted`); a session whose recorded trading ends before the 15:10 square-off (special and Muhurat sessions) and a session the store
/// cut short by an outage are REFUSED by name (`short_session`, `session_outage`), never valued as if the square-off had run.
pub const HALTS: Fx = 1;
/// STANDING F18 (out-of-box G5): an INDEX contract whose cube lot is 0 is resolved to the lot its underlying's other contracts carry that
/// day when that lot divides every traded second's quantity of the lot-0 contracts (exact), else it stays out of the universe COUNTED and
/// NAMED (2019-05-30: quantities not in units: the day's index options named "NO DATA (lot 0)").
pub const LOT0: Fx = 2;
/// STANDING F28 (Addendum 4): STOCK exposure before NSE's first AEL file (2018-10-01 .. 2019-06-13: 172 sessions refused `no_elm_rate`):
/// the dated table `--elm-pre-ael FILE` (per session and stock: the higher of 5% and 1.5 standard deviations of the daily log returns
/// over the previous six months from NSE's own cash bhavcopies, on a rolling day basis AND on NSE's month-end basis: the larger),
/// labelled "exposure from NSE's rule, no AEL file", the first AEL file's rate printed beside.
pub const ELMPRE: Fx = 3;
/// STANDING F30: stamp duty before 2020-07-01 from a dated table (`--stamp-dated FILE`: from, to, buy rate, both-sides rate, class,
/// source); the class words go to every pick and to checklist row 8.
pub const STAMP: Fx = 4;
/// EXITS F-H2 / MATRIX X10 (the HOOK for standing F7): with a dated per-contract-day band table loaded (integrate wires data:F7's reader),
/// a buy block printing at the upper band gives 0 buy capacity (counted) and a sale block at the lower band 0 sale capacity; every limit is
/// clipped to the band. With no table the switch refuses to start (it never pretends to model bands).
pub const BANDS: Fx = 5;
/// STANDING F24 = RECHECK-V33 F2 (round C's BAN_ONCE, ported; result-neutral): an F&O-ban refusal counted once per EVENT (session,
/// contract, decision second), never per pricing (a day-major run prices an event once per batch whose lists hold it).
pub const BANONCE: Fx = 6;
/// STANDING F8 (result-neutral): the run id = second + the segments + the pid + an 8-hex argv hash (two runs started in one second never
/// share a folder), the run folder made with `create_dir` (refused if it exists) and a LOCK file holding the pid.
pub const RUNID: Fx = 7;
/// STANDING F11 (result-neutral): checkpoints written durably (the file synced, the previous generation kept as `<seg>.ckpt.prev`, a
/// SHA-256 line over the text, the directory synced; a torn or failing checkpoint falls back to the previous generation) and every result
/// file written as `.part` then renamed (no reader ever sees a half-written result).
pub const DURABLE: Fx = 8;
/// STANDING F12 (result-neutral): the memory guard: after each day-major batch the process RSS (and the system's swap use) is read; above
/// `--mem-guard-gb G` the run STOPS AT THAT CHECKPOINT (exit with FAILED-MEMORY words; a resume with a smaller gate continues exactly).
pub const MEMGUARD: Fx = 9;
/// STANDING F21 (result-neutral): the carry fill walk MEASURED: blocks walked per order (sale and buy-back walks) p50 / p99 / max printed
/// per segment.
pub const CARRYSTAT: Fx = 10;
/// MATRIX X07 (a DIAL; MAIN rules the budget before it drives a headline): lots = min(margin fit, floor(budget / ((stop % + overshoot %)
/// x price x lot))) (`--riskcap BUDGET_RS,OVERSHOOT_BP`).
pub const RISKCAP: Fx = 11;
/// MATRIX X08 (a DIAL): at most K open shorts on one side (CE or PE) per account (`--side-cap K`; refused `same_side_cap`).
pub const SIDECAP: Fx = 12;
/// MATRIX X11 (DIALS): on an index option's own expiry day, no sale decided after HH:MM (`--expiry-last-sale HH:MM`; refused
/// `expiry_last_sale`) and / or half the lots (`--expiry-size half`).
pub const EXPDIAL: Fx = 13;
/// MATRIX X12 (a DIAL + label): a dated event calendar (`--event-cal FILE`: date, kind, source); `--event-day skip|half|allow`.
pub const EVENTCAL: Fx = 14;
/// MATRIX X17 (a GATE; MAIN rules the limits): a headline pick's worst trade <= W% of its slot and its p99 trade loss <= P% of its slot
/// (`--tail-gate W,P` in percent); the reading is printed beside whatever the switch.
pub const TAILGATE: Fx = 15;
/// MATRIX X23 (a DIAL): on a stale-file decision (the live spot moved more than 1/3 of the price scan since the file) the margin is the
/// STRICT bound: SPAN + |dS| and option value + |dS| per unit (|delta| <= 1, so never below the re-priced scan).
pub const STRICTSPAN: Fx = 16;
/// MATRIX X19: each loaded day's cube extremes against the NSE F&O bhavcopy's day HIGH / LOW per contract: a contract-day whose cube HIGH is
/// below the bhavcopy HIGH (a wick the cube dropped: a buy-back may be priced too low) is FLAGGED (`WICK_DROPPED`), counted, listed in
/// `cube-vs-bhav.tsv`, and every trade on it carries the flag and the BOUND re-price (every unit bought back at the bhavcopy HIGH, the
/// day's highest print: never kinder than the truth) beside its net. Never a refusal (removing a trade could remove a loss).
pub const WICK: Fx = 17;
/// MATRICES X14 / X18 / X25 / X26 (result-neutral readings beside every pick): the never-closed units re-priced at the next session's first
/// block (a stress), the store's stress days replayed (worst trade and day on the stress set), the per-trade re-spike ladder, closest
/// approach and stop hunts, the intraday MTM drawdown at the minute-HIGH marks beside the close-instant drawdown.
pub const READINGS: Fx = 18;
/// EXITS F-H1 (words only; the numbers unchanged): a TT exit (the time-armed breakeven stop the index HEADLINE picks use) is worded "time-armed
/// breakeven stop" in EXIT_WHY, never "trailing target trigger".
pub const TTWORDS: Fx = 19;
const N: usize = 20;
/// The number of switches (tests reset every one).
pub const COUNT: usize = N;
/// The switches `--fix34` turns on (every one that needs no MAIN ruling; BANDS needs data:F7's table, so integrate turns it on).
pub const FIX34_ALL: [Fx; 13] = [CAP20, HALTS, LOT0, ELMPRE, STAMP, BANONCE, RUNID, DURABLE, MEMGUARD, CARRYSTAT, WICK, READINGS, TTWORDS];

#[cfg(not(test))]
static FLAGS: [std::sync::atomic::AtomicBool; N] = [const { std::sync::atomic::AtomicBool::new(false) }; N];
#[cfg(test)]
thread_local! {
    static FLAGS: [std::cell::Cell<bool>; N] = const { [const { std::cell::Cell::new(false) }; N] };
}

/// One switch's value.
#[inline]
#[must_use]
pub fn on(s: Fx) -> bool {
    #[cfg(not(test))]
    {
        FLAGS.get(s).is_some_and(|f| f.load(Ordering::Relaxed))
    }
    #[cfg(test)]
    {
        FLAGS.with(|f| f.get(s).is_some_and(std::cell::Cell::get))
    }
}

/// Sets one switch.
pub fn set(s: Fx, v: bool) {
    #[cfg(not(test))]
    {
        if let Some(f) = FLAGS.get(s) {
            f.store(v, Ordering::Relaxed);
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

/// Whether any switch is on.
#[must_use]
pub fn any() -> bool {
    (0..N).any(on)
}

/// Whether any RESULT-CHANGING switch is on (the result-neutral ones never enter the checkpoint fingerprint).
#[must_use]
pub fn any_result() -> bool {
    (0..N).filter(|&s| !matches!(s, BANONCE | RUNID | DURABLE | MEMGUARD | CARRYSTAT)).any(on)
}

/// The words of one switch (on).
fn word(s: Fx) -> String {
    match s {
        CAP20 => "Rs 20 L cap at EVERY trading minute of EVERY account (margin in use + running MTM loss at minute-HIGH marks, counted in the kernel; an account with a minute above is kept out of every view, of the window rows whose learning years hold one (test years: NOT PASSED) and of the splits whose learning months touch one (test months: fail)) (matrix X20, standing F6)".to_owned(),
        HALTS => format!("dated halts / pauses as tape segments, decisions inside a halt refused (halted), sessions ending before the 15:10 square-off refused (short_session), outage sessions refused (session_outage); table {} (standing F17, exits F-H3, matrix X09)", HALT_T.get().map_or_else(|| "(none)".to_owned(), |t| t.src.clone())),
        LOT0 => "index contracts at cube lot 0 resolved to the underlying's lot that day when it divides every traded quantity, else counted and named NO DATA (lot 0) (standing F18)".to_owned(),
        ELMPRE => format!("stock exposure before NSE's first AEL file from the dated rule table {} ({}) (standing F28)", ELM_T.get().map_or_else(|| "(none)".to_owned(), |t| t.src.clone()), elm_pre_rule_words()),
        STAMP => format!("stamp duty before 2020-07-01 from the dated table {} (standing F30)", STAMP_T.get().map_or_else(|| "(none)".to_owned(), |t| t.src.clone())),
        BANDS => format!("option price bands from the dated table {}: 0 capacity at a band-locked block, limits clipped to the band (exits F-H2, matrix X10)", BAND_T.get().map_or_else(|| "(none)".to_owned(), |t| t.src.clone())),
        BANONCE => "F&O-ban refusals counted once per event (standing F24)".to_owned(),
        RUNID => "unique run id (second + segments + pid + argv hash), run folder created exclusively with a LOCK file (standing F8)".to_owned(),
        DURABLE => "durable checkpoints (synced, previous generation kept, SHA-256 line, fallback) and atomic result files (standing F11)".to_owned(),
        MEMGUARD => format!("memory guard: stop at the next checkpoint above {} GB RSS (standing F12)", MEM_GB.load(Ordering::Relaxed)),
        CARRYSTAT => "carry fill walk measured: blocks per order p50 / p99 / max (standing F21)".to_owned(),
        RISKCAP => format!("DIAL risk cap: lots <= floor(Rs {} / ((stop + {} bp overshoot) x price x lot)) (matrix X07)", RISK_BUDGET.load(Ordering::Relaxed) / 100, RISK_OVER_BP.load(Ordering::Relaxed)),
        SIDECAP => format!("DIAL same-side book cap: at most {} open shorts per side (matrix X08)", SIDE_K.load(Ordering::Relaxed)),
        EXPDIAL => format!("DIAL index expiry day: last sale {} ; size {} (matrix X11)", crate::cal::fmt_sod(i64::try_from(EXP_LAST.load(Ordering::Relaxed)).unwrap_or(0)), if EXP_HALF.load(Ordering::Relaxed) == 1 { "half" } else { "full" }),
        EVENTCAL => format!("DIAL event days {} from {} (matrix X12)", ["allow", "half", "skip"].get(usize::try_from(EVENT_MODE.load(Ordering::Relaxed)).unwrap_or(0)).copied().unwrap_or("?"), EVENT_T.get().map_or_else(|| "(none)".to_owned(), |t| t.src.clone())),
        TAILGATE => format!("GATE tail: a headline pick's worst trade <= {}% and p99 trade loss <= {}% of its slot (matrix X17)", TAIL_W.load(Ordering::Relaxed), TAIL_P.load(Ordering::Relaxed)),
        STRICTSPAN => "DIAL strict stale-file margin: SPAN + |dS| and option value + |dS| per unit on a stale-file decision (matrix X23)".to_owned(),
        WICK => "cube day extremes checked against the NSE bhavcopy: a contract-day whose cube HIGH is below the bhavcopy HIGH flagged WICK_DROPPED, listed in cube-vs-bhav.tsv, its trades carry the flag and the bound re-price at the bhavcopy HIGH (matrix X19)".to_owned(),
        READINGS => "readings beside every pick: next-session stress of never-closed units, the stress-day set, the per-trade re-spike ladder, the intraday MTM drawdown (matrices X14, X18, X25, X26)".to_owned(),
        TTWORDS => "a TT exit worded \"time-armed breakeven stop\" in EXIT_WHY (exits F-H1)".to_owned(),
        _ => String::new(),
    }
}

/// The words of one switch (a dial's value included).
#[must_use]
pub fn dial_words(s: Fx) -> String {
    word(s)
}

/// The words (RUN.log, the RESULTS header; empty when none is on).
#[must_use]
pub fn words() -> String {
    (0..N).filter(|&s| on(s)).map(word).collect::<Vec<_>>().join("; ")
}

/// The fingerprint part (empty when no result-changing switch is on: the v3.3 fingerprints unchanged; the result-neutral switches never
/// enter it, so a resume may change them).
#[must_use]
pub fn fp_part() -> String {
    if any_result() { format!("; fix34 {}", (0..N).filter(|&s| on(s) && !matches!(s, BANONCE | RUNID | DURABLE | MEMGUARD | CARRYSTAT)).map(word).collect::<Vec<_>>().join("; ")) } else { String::new() }
}

/// Reads the switches (`--fix34` = every switch of [`FIX34_ALL`]; each flag may override) and their tables and dial values.
///
/// # Errors
/// A switch value that is not one of its words; a switch on without its table; a dial value that does not parse.
pub fn from_args(a: &[String]) -> Result<(), String> {
    let all = a.iter().any(|x| x == "--fix34");
    let get = |k: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str);
    let flag = |k: &str, yes: &str, no: &str| -> Result<Option<bool>, String> {
        match get(k) {
            None => Ok(None),
            Some(v) if v == yes => Ok(Some(true)),
            Some(v) if v == no => Ok(Some(false)),
            Some(v) => Err(format!("{k} {yes}|{no}, not {v}")),
        }
    };
    let def = |s: Fx| all && FIX34_ALL.contains(&s);
    for (s, k, yes, no) in [
        (CAP20, "--cap20", "on", "off"),
        (HALTS, "--halts-dated", "on", "off"),
        (LOT0, "--lot0", "resolve", "drop"),
        (ELMPRE, "--elm-pre-ael-dated", "on", "off"),
        (STAMP, "--stamp-dated-on", "on", "off"),
        (BANDS, "--bands-dated", "on", "off"),
        (BANONCE, "--ban-count34", "event", "pricing"),
        (RUNID, "--run-id34", "unique", "second"),
        (DURABLE, "--durable", "on", "off"),
        (MEMGUARD, "--mem-guard", "on", "off"),
        (CARRYSTAT, "--carry-stat", "on", "off"),
        (WICK, "--wick-check", "on", "off"),
        (READINGS, "--readings34", "on", "off"),
        (TTWORDS, "--tt-words", "fixed", "v33"),
    ] {
        set(s, flag(k, yes, no)?.unwrap_or(def(s)));
    }
    // the DIALS: on only when their own value is given (a ruling is owed before one drives a headline)
    if let Some(v) = get("--riskcap") {
        let (b, o) = v.split_once(',').ok_or("--riskcap BUDGET_RS,OVERSHOOT_BP")?;
        RISK_BUDGET.store(b.trim().parse::<u64>().map_err(|_| format!("--riskcap {v}"))? * 100, Ordering::Relaxed);
        RISK_OVER_BP.store(o.trim().parse::<u64>().map_err(|_| format!("--riskcap {v}"))?, Ordering::Relaxed);
        set(RISKCAP, true);
    }
    if let Some(v) = get("--side-cap") {
        SIDE_K.store(v.parse::<u64>().map_err(|_| format!("--side-cap {v}"))?.max(1), Ordering::Relaxed);
        set(SIDECAP, true);
    }
    if let Some(v) = get("--expiry-last-sale") {
        let (h, m) = v.split_once(':').ok_or("--expiry-last-sale HH:MM")?;
        let s = h.parse::<u64>().map_err(|_| format!("--expiry-last-sale {v}"))? * 3_600 + m.parse::<u64>().map_err(|_| format!("--expiry-last-sale {v}"))? * 60;
        EXP_LAST.store(s, Ordering::Relaxed);
        set(EXPDIAL, true);
    }
    match get("--expiry-size") {
        None | Some("full") => {}
        Some("half") => {
            EXP_HALF.store(1, Ordering::Relaxed);
            set(EXPDIAL, true);
        }
        Some(o) => return Err(format!("--expiry-size full|half, not {o}")),
    }
    if let Some(v) = get("--event-day") {
        let m = match v {
            "allow" => 0,
            "half" => 1,
            "skip" => 2,
            o => return Err(format!("--event-day allow|half|skip, not {o}")),
        };
        EVENT_MODE.store(m, Ordering::Relaxed);
        set(EVENTCAL, true);
    }
    if let Some(v) = get("--tail-gate") {
        let (w, p) = v.split_once(',').ok_or("--tail-gate W,P (percent of the slot)")?;
        TAIL_W.store(w.trim().parse::<u64>().map_err(|_| format!("--tail-gate {v}"))?, Ordering::Relaxed);
        TAIL_P.store(p.trim().parse::<u64>().map_err(|_| format!("--tail-gate {v}"))?, Ordering::Relaxed);
        set(TAILGATE, true);
    }
    if flag("--strict-span", "on", "off")?.unwrap_or(false) {
        set(STRICTSPAN, true);
    }
    if on(MEMGUARD) {
        let g = get("--mem-guard-gb").map_or(Ok(40), |v| v.parse::<u64>().map_err(|_| format!("--mem-guard-gb {v}")))?;
        MEM_GB.store(g.max(1), Ordering::Relaxed);
    }
    if on(HALTS) {
        let p = get("--halts").ok_or("--halts-dated on (or --fix34) needs --halts <the dated halt / session table>")?;
        let _ = HALT_T.set(HaltTable::load(std::path::Path::new(p))?);
    }
    if on(ELMPRE) {
        let p = get("--elm-pre-ael").ok_or("--elm-pre-ael-dated on (or --fix34) needs --elm-pre-ael <the dated pre-AEL stock exposure table>")?;
        let _ = ELM_T.set(ElmTable::load(std::path::Path::new(p))?);
    }
    if on(STAMP) {
        let p = get("--stamp-dated").ok_or("--stamp-dated-on on (or --fix34) needs --stamp-dated <the dated stamp duty table>")?;
        let _ = STAMP_T.set(StampTable::load(std::path::Path::new(p))?);
    }
    if on(BANDS) {
        let p = get("--bands").ok_or("--bands-dated on needs --bands <the dated per-contract-day band table> (integrate wires data:F7's reader)")?;
        // integrate:D F7 (data:F7 5.1): data:F7's session -> NSE contract file manifest (F7-OPR-SESSIONS.tsv) is read per session from NSE's
        // own files (sha256 checked; never one whole-run file); build:D-high's normalised one-file table otherwise
        if crate::f7::is_manifest(p) {
            crate::f7::load_manifest(p)?;
        } else {
            let _ = BAND_T.set(BandTable::load(std::path::Path::new(p))?);
        }
        if let Some(ca) = get("--f7-ca") {
            crate::f7::load_ca(ca)?;
        }
    }
    if on(EVENTCAL) {
        let p = get("--event-cal").ok_or("--event-day needs --event-cal <the dated event calendar>")?;
        let _ = EVENT_T.set(EventCal::load(std::path::Path::new(p))?);
    }
    if on(CAP20) && !(crate::sw::on(crate::sw::MTM) && crate::sw::on(crate::sw::MARGIN_EXACT)) {
        return Err("--cap20 on (or --fix34) needs the running MTM marks and the exact margin (--v31 / --v32)".to_owned());
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------------ dial values

/// RISKCAP: the budget (paisa) and the overshoot (bp of the premium) beyond the stop.
pub static RISK_BUDGET: AtomicU64 = AtomicU64::new(0);
pub static RISK_OVER_BP: AtomicU64 = AtomicU64::new(0);
/// SIDECAP: at most K open shorts per side.
pub static SIDE_K: AtomicU64 = AtomicU64::new(u64::MAX);
/// EXPDIAL: the last sale second of the day on an index expiry day (0 = none) and half size (1).
pub static EXP_LAST: AtomicU64 = AtomicU64::new(0);
pub static EXP_HALF: AtomicU64 = AtomicU64::new(0);
/// EVENTCAL: 0 allow, 1 half, 2 skip.
pub static EVENT_MODE: AtomicU64 = AtomicU64::new(0);
/// TAILGATE: the worst trade and the p99 trade loss limits (percent of the slot).
pub static TAIL_W: AtomicU64 = AtomicU64::new(100);
pub static TAIL_P: AtomicU64 = AtomicU64::new(100);
/// MEMGUARD: the RSS limit (GB).
pub static MEM_GB: AtomicU64 = AtomicU64::new(40);

// ------------------------------------------------------------------------------------------------ CAP20 (matrix X20)

/// CAP20 counters per segment: accounts kept out of the views, window candidates refused (a breach in a learning year), window picks
/// failed (a breach in a test year), split candidates refused / split test windows failed; the exact per-minute evaluations and the
/// minutes they evaluated (a work count: RUN.log only).
pub static CAP_EXCL: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];
pub static CAP_LEARN: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];
pub static CAP_TESTF: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];
pub static CAP_SPLIT_L: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];
pub static CAP_SPLIT_T: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];
pub static CAP_EXACT: AtomicU64 = AtomicU64::new(0);
pub static CAP_MINUTES: AtomicU64 = AtomicU64::new(0);

/// CAP20: whether an account is kept out of the views (a trading minute above Rs 20 L anywhere in the run). Counted by the caller once per
/// account ([`cap_count_excluded`]).
#[inline]
#[must_use]
pub fn cap_view_out(sm: &crate::acct::Sum) -> bool {
    // MUTANT m_v34_cap_view_ignored: an account with a minute above the cap kept in the views
    on(CAP20) && sm.cap_first1 > 0 && !cfg!(feature = "m_v34_cap_view_ignored")
}

/// CAP20: the Tops groups a sweep keeps per group: 2 with the switch on (the kept accounts' groups, then the same groups of the accounts
/// kept out, for the WIN candidates only), 1 otherwise (v3.3).
#[inline]
#[must_use]
pub fn cap_groups() -> usize {
    if on(CAP20) { 2 } else { 1 }
}

/// CAP20: the WIN candidate source: the kept groups and (switch on) the groups of the accounts kept out, so the sliding rows' candidate set
/// never depends on a breach the row's own learning years cannot see (a breach in a test year then fails the row instead).
#[must_use]
pub fn cand_tops(tops: &[crate::acct::Tops], tops_x: &[crate::acct::Tops]) -> Vec<crate::acct::Tops> {
    // MUTANT m_v34_cap_cands_kept_only: the candidates read the kept groups only (a later breach removes a sliding row's true pick)
    if cfg!(feature = "m_v34_cap_cands_kept_only") {
        return tops.to_vec();
    }
    tops.iter().chain(tops_x).cloned().collect()
}

/// integrate:D (v3.4): the CAP20 counters a stage B SWEEP adds for a segment (accounts kept out, window candidates refused, split candidates
/// refused, split test windows failed), in the checkpoint's CAPC order. The checkpoint and the shard kit's batch files carry them, so a resumed
/// or merged run prints the uninterrupted run's counts (they were process counters only: a resume printed the later batches' alone).
#[must_use]
pub fn cap_snapshot(seg: u8) -> [u64; 4] {
    let s = usize::from(seg.min(1));
    let g = |v: &[AtomicU64; 2]| v.get(s).map_or(0, |x| x.load(Ordering::Relaxed));
    [g(&CAP_EXCL), g(&CAP_LEARN), g(&CAP_SPLIT_L), g(&CAP_SPLIT_T)]
}

/// integrate:D: adds a checkpoint's CAP20 sweep counters (the finished batches') to this process's (a resume; a merged checkpoint).
pub fn cap_restore(seg: u8, v: [u64; 4]) {
    let s = usize::from(seg.min(1));
    for (c, x) in [&CAP_EXCL, &CAP_LEARN, &CAP_SPLIT_L, &CAP_SPLIT_T].into_iter().zip(v) {
        if let Some(a) = c.get(s) {
            a.fetch_add(x, Ordering::Relaxed);
        }
    }
}

/// CAP20: counts one finished account kept out (once per account, at its summary).
pub fn cap_count_excluded(seg: u8) {
    if let Some(x) = CAP_EXCL.get(usize::from(seg.min(1))) {
        x.fetch_add(1, Ordering::Relaxed);
    }
}

/// CAP20: whether a window candidate is refused (its learning pieces hold `cap` minutes above Rs 20 L), counted.
#[must_use]
pub fn cap_learn_refused(cap: u32, seg: u8) -> bool {
    // MUTANT m_v34_cap_learn_ignored: a learning window with a minute above the cap still offered
    if !on(CAP20) || cap == 0 || cfg!(feature = "m_v34_cap_learn_ignored") {
        return false;
    }
    if let Some(x) = CAP_LEARN.get(usize::from(seg.min(1))) {
        x.fetch_add(1, Ordering::Relaxed);
    }
    true
}

/// CAP20: whether a window pick's test pieces hold a minute above Rs 20 L (its row reads NOT PASSED).
#[must_use]
pub fn cap_test_failed(cap: u32) -> bool {
    on(CAP20) && cap > 0
}

/// CAP20 (splits): the first and the last run MONTH (split calendar index) holding a minute above Rs 20 L of a summary, if any.
#[must_use]
pub fn cap_months(sm: &crate::acct::Sum, mo_of_day: &[u8]) -> Option<(usize, usize)> {
    if !on(CAP20) || sm.cap_first1 == 0 {
        return None;
    }
    let f = usize::from(mo_of_day.get(usize::from(sm.cap_first1 - 1)).copied().unwrap_or(0));
    let l = usize::from(mo_of_day.get(usize::from(sm.cap_last1.max(1) - 1)).copied().unwrap_or(u8::MAX));
    Some((f, l.max(f)))
}

/// CAP20 (splits): whether months a..=b may hold a minute above the cap (exact for a window from the run's first month or to its last
/// month, the only windows way B and the per-combo statistics read; conservative for any other window: refused or failed whenever the
/// first and last such months bracket any part of it).
#[must_use]
pub fn cap_in_months(cm: Option<(usize, usize)>, a: usize, b: usize) -> bool {
    cm.is_some_and(|(f, l)| f <= b && l >= a)
}

/// CAP20 (splits): counts a split candidate refused for its learning months, or a split test window failed.
pub fn cap_count_split(seg: u8, learning: bool) {
    let v = if learning { &CAP_SPLIT_L } else { &CAP_SPLIT_T };
    if let Some(x) = v.get(usize::from(seg.min(1))) {
        x.fetch_add(1, Ordering::Relaxed);
    }
}

/// CAP20: the words of a segment (RESULTS).
#[must_use]
pub fn cap_words(seg: usize) -> String {
    let g = |v: &[AtomicU64; 2]| v.get(seg).map_or(0, |x| x.load(Ordering::Relaxed));
    format!("accounts with a trading minute above Rs 20 L (margin in use + running MTM loss, minute-HIGH marks) KEPT OUT of every view {}; window candidates refused for such a minute in their learning years {}; window picks NOT PASSED for one in their test years {}; split candidates refused (learning months) {}; split test windows failed {}", g(&CAP_EXCL), g(&CAP_LEARN), g(&CAP_TESTF), g(&CAP_SPLIT_L), g(&CAP_SPLIT_T))
}

/// CAP20: the work count (RUN.log only; whole and day-major modes differ).
#[must_use]
pub fn cap_cost_words() -> String {
    format!("CAP20 cost (a work count, RUN.log only): exact per-minute evaluations {} ({} minutes evaluated) where the O(1) bound could not rule a breach out", CAP_EXACT.load(Ordering::Relaxed), CAP_MINUTES.load(Ordering::Relaxed))
}

/// CAP20: the suffix maxima of a contract-day's minute marks (index b = the highest mark from minute b to the day's end): the O(1) loss
/// bound of the per-minute cap.
#[must_use]
pub fn suffix_max(v: &[i32]) -> Vec<i32> {
    let mut out = vec![0_i32; v.len()];
    let mut m = i32::MIN;
    for i in (0..v.len()).rev() {
        m = m.max(v.get(i).copied().unwrap_or(0));
        if let Some(o) = out.get_mut(i) {
            *o = m;
        }
    }
    out
}

// ------------------------------------------------------------------------------------------------ HALTS (standing F17, exits F-H3)

/// One dated row of the halt / session table: the session (day number), the kind (`halt` = a market-wide halt with a resumption:
/// [from, resume) seconds of the day; `outage` = trading stopped at `from` and the store holds no later print: the session refused;
/// `short` = the session's recorded trading ends at `from` (before the 15:10 square-off): the session refused), the source.
#[derive(Clone, Debug)]
pub struct HaltRow {
    pub day: i64,
    pub kind: String,
    pub from: i64,
    pub resume: i64,
    pub source: String,
}

/// The dated halt table (`--halts`).
#[derive(Clone, Debug, Default)]
pub struct HaltTable {
    pub rows: Vec<HaltRow>,
    pub src: String,
}

/// The run's halt table.
pub static HALT_T: OnceLock<HaltTable> = OnceLock::new();
#[cfg(test)]
thread_local! {
    /// A test thread's own halt table.
    pub static TEST_HALTS: std::cell::RefCell<Option<HaltTable>> = const { std::cell::RefCell::new(None) };
}

fn hms(s: &str) -> Option<i64> {
    let mut f = s.trim().split(':');
    let h: i64 = f.next()?.parse().ok()?;
    let m: i64 = f.next()?.parse().ok()?;
    let x: i64 = f.next().map_or(Some(0), |v| v.parse().ok())?;
    Some(h * 3_600 + m * 60 + x)
}

impl HaltTable {
    /// Parses the table (tab-separated; header `date\tkind\tfrom\tresume\tsource`; times HH:MM:SS; `-` = none).
    ///
    /// # Errors
    /// A header or row that is not the format.
    pub fn parse(text: &str) -> Result<Vec<HaltRow>, String> {
        let mut lines = text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#'));
        let head = lines.next().unwrap_or("");
        if !head.starts_with("date\tkind\tfrom\tresume\tsource") {
            return Err(format!("--halts: header must be date<TAB>kind<TAB>from<TAB>resume<TAB>source, not {head}"));
        }
        let mut rows = Vec::new();
        for l in lines {
            let f: Vec<&str> = l.split('\t').collect();
            let (Some(day), Some(kind), Some(from), Some(source)) = (f.first().and_then(|x| crate::cal::parse_day(x.trim())), f.get(1).map(|x| x.trim().to_owned()), f.get(2).and_then(|x| hms(x)), f.get(4)) else {
                return Err(format!("--halts: bad row {l}"));
            };
            let resume = f.get(3).and_then(|x| hms(x)).unwrap_or(i64::MAX);
            if !matches!(kind.as_str(), "halt" | "outage" | "short") || (kind == "halt" && resume <= from) {
                return Err(format!("--halts: bad row {l} (kind halt|outage|short; a halt resumes after it starts)"));
            }
            rows.push(HaltRow { day, kind, from, resume, source: (*source).trim().to_owned() });
        }
        rows.sort_by_key(|r| (r.day, r.from));
        Ok(rows)
    }

    /// Loads the table file.
    ///
    /// # Errors
    /// An unreadable file or a refused row.
    pub fn load(p: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(p).map_err(|e| format!("--halts {}: {e}", p.display()))?;
        let rows = Self::parse(&text)?;
        let src = format!("{} (sha256 {}, {} dated rows)", p.display(), crate::cube::sha256(p), rows.len());
        Ok(Self { rows, src })
    }

    /// The rows of session `day`.
    #[must_use]
    pub fn of(&self, day: i64) -> Vec<&HaltRow> {
        let i = self.rows.partition_point(|r| r.day < day);
        self.rows.get(i..).unwrap_or(&[]).iter().take_while(|r| r.day == day).collect()
    }
}

/// HALTS: the halts of session `day` as [from, resume) seconds of the day (empty with the switch off or no row).
#[must_use]
pub fn halts_of(day: i64) -> Vec<(i64, i64)> {
    if !on(HALTS) {
        return Vec::new();
    }
    #[cfg(test)]
    {
        if let Some(v) = TEST_HALTS.with(|t| t.borrow().as_ref().map(|t| t.of(day).iter().filter(|r| r.kind == "halt").map(|r| (r.from, r.resume)).collect::<Vec<_>>())) {
            return v;
        }
    }
    HALT_T.get().map_or_else(Vec::new, |t| t.of(day).iter().filter(|r| r.kind == "halt").map(|r| (r.from, r.resume)).collect())
}

/// HALTS: the refusal of a whole session (`short_session` / `session_outage`), if any.
#[must_use]
pub fn session_refusal(day: i64) -> Option<crate::trade::Refuse> {
    if !on(HALTS) {
        return None;
    }
    let pick = |rows: Vec<&HaltRow>| -> Option<crate::trade::Refuse> {
        rows.iter().find_map(|r| match r.kind.as_str() {
            "short" => Some(crate::trade::Refuse::ShortSession),
            "outage" => Some(crate::trade::Refuse::SessionOutage),
            _ => None,
        })
    };
    #[cfg(test)]
    {
        if let Some(v) = TEST_HALTS.with(|t| t.borrow().as_ref().map(|t| pick(t.of(day)))) {
            return v;
        }
    }
    HALT_T.get().and_then(|t| pick(t.of(day)))
}

/// HALTS: whether decision second `t_sod` of session `day` falls inside a halt (refused `halted`).
#[must_use]
pub fn in_halt(day: i64, t_sod: i64) -> bool {
    // MUTANT m_v34_halt_decisions_taken: a decision inside a halt priced as if the market were open
    !cfg!(feature = "m_v34_halt_decisions_taken") && halts_of(day).iter().any(|&(a, b)| t_sod >= a && t_sod < b)
}

/// HALTS: the tape segments of session `day` (the run's one segment [open, end) cut at every halt). One segment with the switch off.
#[must_use]
pub fn segments_of(day: i64, open_us: i64, end_us: i64) -> Vec<common_fill::Seg> {
    let us = common_fill::US_S;
    let mut v = Vec::new();
    let mut a = open_us;
    // MUTANT m_v34_one_segment: the halts read but the tape kept as one segment (the carried entitlement crosses the halt)
    let halts = if cfg!(feature = "m_v34_one_segment") { Vec::new() } else { halts_of(day) };
    for (from, resume) in halts {
        let f = crate::cal::at_sod(day, from);
        let r = crate::cal::at_sod(day, resume);
        if f > a && r < end_us {
            v.push(common_fill::Seg { open: a, last_sec: f - us, auction_open: false });
            a = r;
        }
    }
    v.push(common_fill::Seg { open: a, last_sec: end_us - us, auction_open: false });
    v
}

/// HALTS: refusal counters (distinct (session, contract, second) decisions inside a halt; distinct sessions refused whole).
pub static HALT_SEEN: Mutex<BTreeSet<(i64, u32, i64)>> = Mutex::new(BTreeSet::new());
pub static SESS_REFUSED: Mutex<BTreeMap<i64, &'static str>> = Mutex::new(BTreeMap::new());

/// HALTS: the words (RESULTS, checklist row 9).
#[must_use]
pub fn halt_words() -> String {
    let t = HALT_T.get();
    let halts: Vec<String> = t.map(|t| t.rows.iter().filter(|r| r.kind == "halt").map(|r| format!("{} {}-{}", crate::cal::fmt_day(r.day), crate::cal::fmt_sod(r.from), crate::cal::fmt_sod(r.resume))).collect()).unwrap_or_default();
    let n = HALT_SEEN.lock().map_or(0, |s| s.len());
    let sess: Vec<String> = SESS_REFUSED.lock().map(|m| m.iter().map(|(d, k)| format!("{} {k}", crate::cal::fmt_day(*d))).collect()).unwrap_or_default();
    format!("halts as tape segments: {} ; decisions inside a halt refused (halted): {n} ; whole sessions refused by name (the store holds no 15:10 square-off for them): {}", if halts.is_empty() { "none in the table".to_owned() } else { halts.join(", ") }, if sess.is_empty() { "none in this run".to_owned() } else { sess.join(", ") })
}

/// HALTS: each print's previous print clamped to its segment's open - 1 s (the day's first print reads 09:14:59: a post-halt first print
/// reads the second before the resumption, never the pre-halt print: its volume is spread from the reopening, as at the open).
pub fn clamp_prev_to_segments(prev: &mut [i64], rows: &[common_fill::SecIn], segs: &[common_fill::Seg]) {
    if segs.len() < 2 {
        return;
    }
    for (p, r) in prev.iter_mut().zip(rows) {
        if let Some(s) = segs.iter().rev().find(|s| s.open <= r.ts) {
            *p = (*p).max(s.open - common_fill::US_S);
        }
    }
}

// ------------------------------------------------------------------------------------------------ LOT0 (standing F18)

/// LOT0: per (session, underlying name): (lot-0 contracts, resolved lot or 0, the lot-0 contracts' traded quantum GCD).
pub static LOT0_SEEN: Mutex<BTreeMap<(i64, String), (u32, u32, u64)>> = Mutex::new(BTreeMap::new());

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// LOT0: resolves the day's INDEX contracts at cube lot 0 (no-op with the switch off): per underlying, the lots its other contracts carry
/// that day; a single such lot L that divides the GCD of every traded MINUTE volume of the lot-0 contracts becomes their lot (every
/// trade is a whole number of real lots; the cube alone is read, so the pass-0 screener levels and every later load resolve alike); otherwise they stay at lot 0 (out of the universe), counted and named. Stock contracts are
/// not touched (the LOT guard of fix:v33 covers stock lots).
pub fn lot0_resolve(c: &mut Cube) {
    if !on(LOT0) {
        return;
    }
    let mut by: HashMap<u16, (Vec<usize>, BTreeSet<u32>, u64)> = HashMap::new();
    for (ci, k) in c.cons.iter().enumerate() {
        if k.kind != 0 {
            continue;
        }
        let e = by.entry(k.und).or_default();
        if k.lot == 0 {
            e.0.push(ci);
            for m in 0..crate::cal::NM {
                let v = c.vol.get(Cube::ix(ci, m)).copied().unwrap_or(0);
                if v > 0 {
                    e.2 = gcd(e.2, u64::from(v));
                }
            }
        } else {
            e.1.insert(k.lot);
        }
    }
    for (und, (zero, lots, g)) in by {
        if zero.is_empty() {
            continue;
        }
        let name = c.unds.get(usize::from(und)).map_or("?", |u| u.name.as_str()).to_owned();
        // MUTANT m_v34_lot0_any_lot: the first known lot taken without the traded quantum's check
        let fits: Vec<u32> = lots.iter().copied().filter(|&l| cfg!(feature = "m_v34_lot0_any_lot") || (g > 0 && g % u64::from(l) == 0)).collect();
        let resolved = if fits.len() == 1 { fits.first().copied().unwrap_or(0) } else { 0 };
        if resolved > 0 {
            for &ci in &zero {
                if let Some(k) = c.cons.get_mut(ci) {
                    k.lot = resolved;
                }
            }
        }
        if let Ok(mut m) = LOT0_SEEN.lock() {
            m.insert((c.day, name), (u32::try_from(zero.len()).unwrap_or(u32::MAX), resolved, g));
        }
    }
}

/// LOT0: the words (RESULTS, the Addendum 17 universe table).
#[must_use]
pub fn lot0_words() -> String {
    let m = LOT0_SEEN.lock().map(|m| m.clone()).unwrap_or_default();
    let res: Vec<String> = m.iter().filter(|x| x.1.1 > 0).map(|((d, u), (n, l, g))| format!("{} {u}: {n} at lot 0 -> lot {l} (traded GCD {g})", crate::cal::fmt_day(*d))).collect();
    let nodata: Vec<String> = m.iter().filter(|x| x.1.1 == 0).map(|((d, u), (n, _, g))| format!("{} {u}: {n} contracts NO DATA (lot 0; traded GCD {g}: {})", crate::cal::fmt_day(*d), if *g <= 1 { "quantities not in units" } else { "no single known lot divides it" })).collect();
    format!("index contracts at cube lot 0 (fix34 LOT0, standing F18): resolved {} contract-day group(s){}; NO DATA (lot 0, out of the universe, named) {} group(s){}", res.len(), if res.is_empty() { String::new() } else { format!(": {}", res.iter().take(30).cloned().collect::<Vec<_>>().join("; ")) }, nodata.len(), if nodata.is_empty() { String::new() } else { format!(": {}", nodata.iter().take(30).cloned().collect::<Vec<_>>().join("; ")) })
}

// ------------------------------------------------------------------------------------------------ ELMPRE (standing F28)

/// The dated pre-AEL stock exposure table: (session yyyymmdd, NSE symbol) -> (rate in hundredths of a percent, the first AEL file's OTH
/// rate beside or -1).
#[derive(Clone, Debug, Default)]
pub struct ElmTable {
    pub rows: HashMap<(u32, String), (i32, i32)>,
    /// The last session the table covers (the first AEL file is published from the next one).
    pub last: u32,
    pub src: String,
}

/// The run's pre-AEL table.
pub static ELM_T: OnceLock<ElmTable> = OnceLock::new();
#[cfg(test)]
thread_local! {
    pub static TEST_ELM: std::cell::RefCell<Option<ElmTable>> = const { std::cell::RefCell::new(None) };
}

impl ElmTable {
    /// Parses the table (tab-separated; header `session\tsymbol\trate_h\t...`; the 4th column = the first AEL OTH rate or -1).
    ///
    /// # Errors
    /// A header or row that is not the format.
    pub fn parse(text: &str) -> Result<(HashMap<(u32, String), (i32, i32)>, u32), String> {
        let mut lines = text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#'));
        let head = lines.next().unwrap_or("");
        if !head.starts_with("session\tsymbol\trate_h\tfirst_ael_oth_h") {
            return Err(format!("--elm-pre-ael: header must start session<TAB>symbol<TAB>rate_h<TAB>first_ael_oth_h, not {head}"));
        }
        let mut m = HashMap::new();
        let mut last = 0_u32;
        for l in lines {
            let f: Vec<&str> = l.split('\t').collect();
            let (Some(d), Some(sym), Some(r), Some(b)) = (f.first().and_then(|x| crate::cal::parse_day(x.trim())).map(crate::cal::ymd_of), f.get(1), f.get(2).and_then(|x| x.trim().parse::<i32>().ok()), f.get(3).and_then(|x| x.trim().parse::<i32>().ok())) else {
                return Err(format!("--elm-pre-ael: bad row {l}"));
            };
            if !(500..=10_000).contains(&r) {
                return Err(format!("--elm-pre-ael: rate below the 5% floor or above 100% in {l}"));
            }
            last = last.max(d);
            m.insert((d, (*sym).trim().to_owned()), (r, b));
        }
        Ok((m, last))
    }

    /// Loads the table file.
    ///
    /// # Errors
    /// An unreadable file or a refused row.
    pub fn load(p: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(p).map_err(|e| format!("--elm-pre-ael {}: {e}", p.display()))?;
        let (rows, last) = Self::parse(&text)?;
        let src = format!("{} (sha256 {}, {} (session, stock) rows to {last})", p.display(), crate::cube::sha256(p), rows.len());
        Ok(Self { rows, last, src })
    }
}

/// ELMPRE: the dated rule rate of stock `sym` on session `ymd` (None = off, or no row).
#[must_use]
pub fn elm_pre(ymd: u32, sym: &str) -> Option<i32> {
    if !on(ELMPRE) {
        return None;
    }
    // MUTANT m_v34_elm_pre_floor_only: the 5% floor used whatever the table says
    let look = |t: &ElmTable| t.rows.get(&(ymd, sym.to_owned())).map(|x| if cfg!(feature = "m_v34_elm_pre_floor_only") { 500 } else { x.0 });
    #[cfg(test)]
    {
        if let Some(v) = TEST_ELM.with(|t| t.borrow().as_ref().map(look)) {
            return v;
        }
    }
    ELM_T.get().and_then(look)
}

/// ELMPRE: decisions priced at the dated rule (distinct (session, contract) of the run).
pub static ELM_USED: Mutex<BTreeSet<(u32, u32)>> = Mutex::new(BTreeSet::new());

// ------------------------------------------------------------------------------------------------ STAMP (standing F30)

/// One dated stamp row: [from, to] (days), the buy-side and the both-sides rate (paise per crore of premium), the class, the source.
#[derive(Clone, Debug)]
pub struct StampRow {
    pub from: i64,
    pub to: i64,
    pub buy: i128,
    pub both: i128,
    pub class: String,
    pub source: String,
}

/// The dated stamp table (`--stamp-dated`).
#[derive(Clone, Debug, Default)]
pub struct StampTable {
    pub rows: Vec<StampRow>,
    pub src: String,
}

/// The run's stamp table.
pub static STAMP_T: OnceLock<StampTable> = OnceLock::new();
#[cfg(test)]
thread_local! {
    /// A test thread's own stamp table.
    pub static TEST_STAMP: std::cell::RefCell<Option<StampTable>> = const { std::cell::RefCell::new(None) };
}

impl StampTable {
    /// Parses the table (header `from\tto\tbuy_paise_per_crore\tboth_paise_per_crore\tclass\tsource`).
    ///
    /// # Errors
    /// A header or row that is not the format, or rows out of order.
    pub fn parse(text: &str) -> Result<Vec<StampRow>, String> {
        let mut lines = text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#'));
        let head = lines.next().unwrap_or("");
        if !head.starts_with("from\tto\tbuy_paise_per_crore\tboth_paise_per_crore\tclass\tsource") {
            return Err(format!("--stamp-dated: header must be from<TAB>to<TAB>buy_paise_per_crore<TAB>both_paise_per_crore<TAB>class<TAB>source, not {head}"));
        }
        let mut rows: Vec<StampRow> = Vec::new();
        for l in lines {
            let f: Vec<&str> = l.split('\t').collect();
            let (Some(from), Some(to), Some(buy), Some(both), Some(class), Some(source)) = (f.first().and_then(|x| crate::cal::parse_day(x.trim())), f.get(1).and_then(|x| crate::cal::parse_day(x.trim())), f.get(2).and_then(|x| x.trim().parse::<i128>().ok()), f.get(3).and_then(|x| x.trim().parse::<i128>().ok()), f.get(4), f.get(5)) else {
                return Err(format!("--stamp-dated: bad row {l}"));
            };
            if from > to || buy < 0 || both < 0 || rows.last().is_some_and(|r| r.to >= from) {
                return Err(format!("--stamp-dated: bad or overlapping row {l}"));
            }
            rows.push(StampRow { from, to, buy, both, class: (*class).trim().to_owned(), source: (*source).trim().to_owned() });
        }
        Ok(rows)
    }

    /// Loads the table file.
    ///
    /// # Errors
    /// An unreadable file or a refused row.
    pub fn load(p: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(p).map_err(|e| format!("--stamp-dated {}: {e}", p.display()))?;
        let rows = Self::parse(&text)?;
        let src = format!("{} (sha256 {}, {} dated rows)", p.display(), crate::cube::sha256(p), rows.len());
        Ok(Self { rows, src })
    }
}

/// STAMP: the (buy, both-sides) stamp rates of day `d` from the dated table (None = off or not covered: v3.3's rows stand).
#[must_use]
pub fn stamp_rates(d: i64) -> Option<(i128, i128)> {
    if !on(STAMP) {
        return None;
    }
    #[cfg(test)]
    let tt = TEST_STAMP.with(|x| x.borrow().clone());
    #[cfg(test)]
    let t = match tt.as_ref() {
        Some(t) => t,
        None => STAMP_T.get()?,
    };
    #[cfg(not(test))]
    let t = STAMP_T.get()?;
    let i = t.rows.partition_point(|r| r.to < d);
    // MUTANT m_v34_stamp_table_ignored: the dated table read but never applied
    if cfg!(feature = "m_v34_stamp_table_ignored") {
        return None;
    }
    t.rows.get(i).filter(|r| r.from <= d).map(|r| (r.buy, r.both))
}

/// STAMP: the class of day `d`'s stamp row (checklist row 8 and the pick words).
#[must_use]
pub fn stamp_class(d: i64) -> Option<String> {
    if !on(STAMP) {
        return None;
    }
    let t = STAMP_T.get()?;
    let i = t.rows.partition_point(|r| r.to < d);
    t.rows.get(i).filter(|r| r.from <= d).map(|r| format!("{} ({})", r.class, r.source))
}

// ------------------------------------------------------------------------------------------------ BANDS (exits F-H2, the F7 hook)

/// The dated per-contract-day band table: (session yyyymmdd, underlying, expiry yyyymmdd, strike paisa, right) -> (lower band paisa, upper
/// band paisa). The reader below takes data:F7's normalised form; integrate wires its tables in.
#[derive(Clone, Debug, Default)]
pub struct BandTable {
    pub rows: HashMap<(u32, String, u32, i64, u8), (i64, i64)>,
    pub src: String,
}

/// The run's band table.
pub static BAND_T: OnceLock<BandTable> = OnceLock::new();

impl BandTable {
    /// Parses the table (header `session\tunderlying\texpiry\tstrike_paisa\tright\tlower_paisa\tupper_paisa`).
    ///
    /// # Errors
    /// A header or row that is not the format.
    pub fn parse(text: &str) -> Result<HashMap<(u32, String, u32, i64, u8), (i64, i64)>, String> {
        let mut lines = text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#'));
        let head = lines.next().unwrap_or("");
        if !head.starts_with("session\tunderlying\texpiry\tstrike_paisa\tright\tlower_paisa\tupper_paisa") {
            return Err(format!("--bands: header must be session<TAB>underlying<TAB>expiry<TAB>strike_paisa<TAB>right<TAB>lower_paisa<TAB>upper_paisa, not {head}"));
        }
        let mut m = HashMap::new();
        for l in lines {
            let f: Vec<&str> = l.split('\t').collect();
            let p = |i: usize| f.get(i).map(|x| x.trim());
            let (Some(d), Some(u), Some(e), Some(k), Some(r), Some(lo), Some(hi)) = (p(0).and_then(crate::cal::parse_day).map(crate::cal::ymd_of), p(1), p(2).and_then(crate::cal::parse_day).map(crate::cal::ymd_of), p(3).and_then(|x| x.parse::<i64>().ok()), p(4).map(|x| u8::from(x == "PE" || x == "1")), p(5).and_then(|x| x.parse::<i64>().ok()), p(6).and_then(|x| x.parse::<i64>().ok())) else {
                return Err(format!("--bands: bad row {l}"));
            };
            if lo < 0 || hi <= lo {
                return Err(format!("--bands: bad band in {l}"));
            }
            m.insert((d, u.to_owned(), e, k, r), (lo, hi));
        }
        Ok(m)
    }

    /// Loads the table file.
    ///
    /// # Errors
    /// An unreadable file or a refused row.
    pub fn load(p: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(p).map_err(|e| format!("--bands {}: {e}", p.display()))?;
        let rows = Self::parse(&text)?;
        let src = format!("{} (sha256 {}, {} contract-day rows)", p.display(), crate::cube::sha256(p), rows.len());
        Ok(Self { rows, src })
    }
}

/// BANDS: the band of contract `ci` of the day (None = off or no row: the contract trades without a band reading, counted).
#[must_use]
pub fn band_of(c: &Cube, ci: usize) -> Option<(i64, i64)> {
    if !on(BANDS) {
        return None;
    }
    let k = c.cons.get(ci)?;
    let und = c.unds.get(usize::from(k.und))?.name.clone();
    let key = (crate::cal::ymd_of(c.day), und, k.expiry, k.strike, k.right & 1);
    #[cfg(test)]
    {
        if let Some(v) = TEST_BANDS.with(|t| t.borrow().as_ref().map(|t| t.rows.get(&key).copied())) {
            return v;
        }
    }
    // integrate:D F7: with data:F7's manifest the exact band comes from NSE's file of the session (never the no-file bound)
    match BAND_T.get() {
        Some(t) => t.rows.get(&key).copied(),
        None => crate::f7::exact_band(c, ci).map(|b| (b.0, b.1)),
    }
}

#[cfg(test)]
thread_local! {
    /// A test thread's own band table.
    pub static TEST_BANDS: std::cell::RefCell<Option<BandTable>> = const { std::cell::RefCell::new(None) };
    /// A test thread's own event calendar.
    pub static TEST_EVENTS: std::cell::RefCell<Option<EventCal>> = const { std::cell::RefCell::new(None) };
}

/// BANDS: the band of contract `ci` of the day as the crate's tape band (lower, upper, tick) (None = off, or no row: counted). The engine
/// puts it on the contract-day's tape view; the sale limit, the square-off and the buy-back limits and every re-sent exit limit are then
/// clipped one tick inside it (crate v2.5 B1).
#[must_use]
pub fn band3(c: &Cube, ci: usize) -> Option<(i64, i64, i64)> {
    if !on(BANDS) {
        return None;
    }
    // MUTANT m_v34_band_ignored: the band read but never put on the tape (a band-locked block fills AT the band)
    if cfg!(feature = "m_v34_band_ignored") {
        return None;
    }
    match band_of(c, ci) {
        Some((lo, hi)) => Some((lo, hi, crate::trade::TICK)),
        None => {
            if let Ok(mut s) = BAND_MISSING.lock() {
                s.insert((c.day, c.cons.get(ci).map_or(0, |k| k.sid)));
            }
            None
        }
    }
}

/// BANDS: contract-days priced without a band row (the table did not cover them; counted and named).
pub static BAND_MISSING: Mutex<BTreeSet<(i64, u32)>> = Mutex::new(BTreeSet::new());

/// BANDS: the words (RESULTS, checklist row 5).
#[must_use]
pub fn band_words() -> String {
    let n = BAND_MISSING.lock().map_or(0, |s| s.len());
    // integrate:D: with data:F7's manifest the B1 band is NSE's exact file band of the session (before 2024-02-06 there is none: those contract-days are
    // counted here and read the proven no-file bound in the strict readings only)
    let src = BAND_T.get().map(|t| t.src.clone()).or_else(|| crate::f7::OPR_T.get().map(|o| format!("data:F7's manifest {} (B1 on NSE's exact file band only; the no-file bound only in the strict readings)", o.src))).unwrap_or_else(|| "(no table)".to_owned());
    format!("price bands from {} (crate v2.5 B1: every sale, square-off, buy-back and re-sent exit limit clipped one tick inside the contract-day's band; a block whose worst print sits at the band is band-locked: no fill, the rest walks on); contract-days priced WITHOUT {} band row: {n}", src, if BAND_T.get().is_some() { "a" } else { "an exact" })
}

// ------------------------------------------------------------------------------------------------ RUNID (standing F8)

/// RUNID: an 8-hex FNV-1a hash of the arguments (the run id's argv part).
#[must_use]
pub fn argv_hash(a: &[String]) -> String {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for x in a {
        for b in x.bytes().chain(std::iter::once(0)) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{:08x}", h & 0xFFFF_FFFF)
}

/// RUNID: the unique run id: the v3.3 id + the segments + the pid + the argv hash.
#[must_use]
pub fn unique_id(base: &str, segs: &[u8], a: &[String]) -> String {
    let s = if segs.len() == 2 { "both".to_owned() } else { segs.iter().map(|&x| if x == 0 { "idx" } else { "stk" }).collect::<Vec<_>>().join("") };
    format!("{base}-{s}-p{}-{}", std::process::id(), argv_hash(a))
}

/// RUNID: creates the run folder exclusively (its parent with `create_dir_all`, the folder itself with `create_dir`: refused if it exists)
/// and writes its LOCK file (the pid).
///
/// # Errors
/// An existing folder (another run holds the id) or an unwritable one.
pub fn create_run_dir(rdir: &std::path::Path) -> Result<(), String> {
    if let Some(p) = rdir.parent() {
        std::fs::create_dir_all(p).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    // MUTANT m_v34_runid_dir_shared: create_dir_all (an existing folder silently shared by two runs)
    if cfg!(feature = "m_v34_runid_dir_shared") {
        std::fs::create_dir_all(rdir).map_err(|e| e.to_string())?;
    } else {
        std::fs::create_dir(rdir).map_err(|e| format!("{}: {e} (run folder exists: two runs never share one; refused)", rdir.display()))?;
    }
    std::fs::write(rdir.join("LOCK"), format!("{}\n", std::process::id())).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------------------------------------ DURABLE (standing F11)

/// SHA-256 of bytes (FIPS 180-4; in-process, no external tool: the checkpoint's integrity line).
#[must_use]
pub fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4, 0xab1c_5ed5, 0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3, 0x72be_5d74, 0x80de_b1fe, 0x9bdc_06a7, 0xc19b_f174, 0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc, 0x2de9_2c6f, 0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da, 0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7, 0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967, 0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc, 0x5338_0d13, 0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85, 0xa2bf_e8a1, 0xa81a_664b, 0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070, 0x19a4_c116, 0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3, 0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208, 0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7, 0xc671_78f2,
    ];
    let mut h: [u32; 8] = [0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a, 0x510e_527f, 0x9b05_688c, 0x1f83_d9ab, 0x5be0_cd19];
    let bits = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0_u32; 64];
        for (i, wi) in w.iter_mut().take(16).enumerate() {
            let b = chunk.get(4 * i..4 * i + 4).unwrap_or(&[0, 0, 0, 0]);
            *wi = u32::from_be_bytes([b.first().copied().unwrap_or(0), b.get(1).copied().unwrap_or(0), b.get(2).copied().unwrap_or(0), b.get(3).copied().unwrap_or(0)]);
        }
        for i in 16..64 {
            let (w15, w2) = (w.get(i - 15).copied().unwrap_or(0), w.get(i - 2).copied().unwrap_or(0));
            let s0 = w15.rotate_right(7) ^ w15.rotate_right(18) ^ (w15 >> 3);
            let s1 = w2.rotate_right(17) ^ w2.rotate_right(19) ^ (w2 >> 10);
            let v = w.get(i - 16).copied().unwrap_or(0).wrapping_add(s0).wrapping_add(w.get(i - 7).copied().unwrap_or(0)).wrapping_add(s1);
            if let Some(x) = w.get_mut(i) {
                *x = v;
            }
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K.get(i).copied().unwrap_or(0)).wrapping_add(w.get(i).copied().unwrap_or(0));
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let mj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(mj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

/// DURABLE: writes `path` atomically AND durably: `<path>.part` written and synced, renamed over `path`, the directory synced. With the
/// switch off: v3.3's plain write (proof P0).
///
/// # Errors
/// An unwritable file.
pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write as _;
    if !on(DURABLE) {
        return std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()));
    }
    let mut part = path.as_os_str().to_owned();
    part.push(".part");
    let part = std::path::PathBuf::from(part);
    let mut f = std::fs::File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
    f.write_all(bytes).map_err(|e| format!("{}: {e}", part.display()))?;
    f.sync_all().map_err(|e| format!("{}: {e}", part.display()))?;
    drop(f);
    std::fs::rename(&part, path).map_err(|e| format!("{}: {e}", path.display()))?;
    sync_dir(path);
    Ok(())
}

/// Syncs the directory holding `path` (the rename made durable).
pub fn sync_dir(path: &std::path::Path) {
    if let Some(d) = path.parent()
        && let Ok(f) = std::fs::File::open(d)
    {
        let _ = f.sync_all();
    }
}

/// DURABLE: checkpoint loads that fell back to the previous generation (the newest failed its SHA-256 line).
pub static CKPT_FALLBACK: AtomicU64 = AtomicU64::new(0);

/// DURABLE: a checkpoint text with its SHA-256 line (after `END`).
#[must_use]
pub fn with_sha_line(text: &str) -> String {
    format!("{text}SHA256\t{}\n", sha256_hex(text.as_bytes()))
}

/// DURABLE: the text of a checkpoint whose SHA-256 line verifies (the text without that line), else None (torn, edited or partial).
#[must_use]
pub fn verified_text(raw: &str) -> Option<String> {
    let body = raw.strip_suffix('\n')?;
    let (text, line) = body.rsplit_once('\n').map(|(a, b)| (format!("{a}\n"), b))?;
    let want = line.strip_prefix("SHA256\t")?;
    // MUTANT m_v34_ckpt_sha_unchecked: the integrity line read but never compared (a torn checkpoint accepted)
    (cfg!(feature = "m_v34_ckpt_sha_unchecked") || sha256_hex(text.as_bytes()) == want).then_some(text)
}

// ------------------------------------------------------------------------------------------------ MEMGUARD (standing F12)

/// MEMGUARD: the process RSS in KB (`ps`; 0 when unread) and the system's swap use in MB (`sysctl vm.swapusage`; 0 when unread).
#[must_use]
pub fn mem_now() -> (u64, u64) {
    let pid = std::process::id().to_string();
    let rss = std::process::Command::new("/bin/ps").args(["-o", "rss=", "-p", &pid]).output().ok().and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().ok()).unwrap_or(0);
    let swap = std::process::Command::new("/usr/sbin/sysctl").args(["-n", "vm.swapusage"]).output().ok().and_then(|o| {
        let s = String::from_utf8_lossy(&o.stdout).to_string();
        // "total = 8192.00M  used = 6912.25M  free = ..."
        s.split("used =").nth(1).and_then(|x| x.trim().split('M').next()).and_then(|x| x.trim().parse::<f64>().ok())
    });
    (rss, swap.map_or(0, |x| x as u64))
}

/// MEMGUARD: whether the run must stop at this checkpoint (RSS above the limit). The words go to RUN.log either way.
#[must_use]
pub fn mem_over(rss_kb: u64) -> bool {
    // MUTANT m_v34_memguard_never: the guard reads the RSS but never stops
    on(MEMGUARD) && !cfg!(feature = "m_v34_memguard_never") && rss_kb > MEM_GB.load(Ordering::Relaxed).saturating_mul(1_048_576)
}

// ------------------------------------------------------------------------------------------------ CARRYSTAT (standing F21)

/// CARRYSTAT (standing F21): (orders, p50, p99, max) of blocks per order from the stage A counters (exact counts; never sampled).
#[must_use]
pub fn carry_stats(v: &[u32]) -> (u64, u32, u32, u32) {
    if v.is_empty() {
        return (0, 0, 0, 0);
    }
    let mut s = v.to_vec();
    s.sort_unstable();
    let at = |q: usize| s.get((s.len() * q).div_ceil(100).saturating_sub(1).min(s.len() - 1)).copied().unwrap_or(0);
    (u64::try_from(s.len()).unwrap_or(u64::MAX), at(50), at(99), s.last().copied().unwrap_or(0))
}

/// CARRYSTAT: the words of one segment's counters (RESULTS: exact, counted once per event).
#[must_use]
pub fn carry_words(sale: &[u32], buy: &[u32]) -> String {
    let (n, p50, p99, mx) = carry_stats(sale);
    let (nb, b50, b99, bmx) = carry_stats(buy);
    format!("carry fill walk (standing F21; exact counts, once per event): sale orders {n}, blocks read per order p50 {p50} / p99 {p99} / max {mx}; own buy-back paths {nb}, blocks recorded per path p50 {b50} / p99 {b99} / max {bmx} (each walk is bounded by its order window: at most one block per block length of the session)")
}

// ------------------------------------------------------------------------------------------------ EVENTCAL (matrix X12)

/// The dated event calendar: session -> (kind, source).
#[derive(Clone, Debug, Default)]
pub struct EventCal {
    pub days: BTreeMap<i64, (String, String)>,
    pub src: String,
}

/// The run's event calendar.
pub static EVENT_T: OnceLock<EventCal> = OnceLock::new();

impl EventCal {
    /// Loads the calendar (header `date\tkind\tsource`).
    ///
    /// # Errors
    /// An unreadable file or a bad row.
    pub fn load(p: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(p).map_err(|e| format!("--event-cal {}: {e}", p.display()))?;
        let mut lines = text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#'));
        let head = lines.next().unwrap_or("");
        if !head.starts_with("date\tkind\tsource") {
            return Err(format!("--event-cal: header must be date<TAB>kind<TAB>source, not {head}"));
        }
        let mut days = BTreeMap::new();
        for l in lines {
            let f: Vec<&str> = l.split('\t').collect();
            let (Some(d), Some(k), Some(s)) = (f.first().and_then(|x| crate::cal::parse_day(x.trim())), f.get(1), f.get(2)) else { return Err(format!("--event-cal: bad row {l}")) };
            days.insert(d, ((*k).trim().to_owned(), (*s).trim().to_owned()));
        }
        let src = format!("{} (sha256 {}, {} dated sessions)", p.display(), crate::cube::sha256(p), days.len());
        Ok(Self { days, src })
    }
}

/// EVENTCAL: the event of session `d` (None = off or an ordinary session).
#[must_use]
pub fn event_of(d: i64) -> Option<(String, String)> {
    if !on(EVENTCAL) {
        return None;
    }
    #[cfg(test)]
    {
        if let Some(v) = TEST_EVENTS.with(|t| t.borrow().as_ref().map(|t| t.days.get(&d).cloned())) {
            return v;
        }
    }
    EVENT_T.get().and_then(|t| t.days.get(&d).cloned())
}

// ------------------------------------------------------------------------------------------------ WICK (matrix X19)

/// WICK: per (session, security id): (cube HIGH, bhavcopy HIGH, cube LOW, bhavcopy LOW) of every contract-day flagged.
pub static WICK_SEEN: Mutex<BTreeMap<(i64, u32), (i64, i64, i64, i64)>> = Mutex::new(BTreeMap::new());
/// WICK: contract-days compared (both a cube print and a bhavcopy row), counted once per session.
pub static WICK_CHECKED: AtomicU64 = AtomicU64::new(0);
/// WICK: per session, the flagged contract indices of its cube (computed ONCE per session: a streamed run reloads a day once per batch;
/// only the flagged indices are kept, so the cache is tiny) and the bhavcopy HIGH of each.
static WICK_DAY: Mutex<BTreeMap<i64, std::sync::Arc<Vec<(u32, i64)>>>> = Mutex::new(BTreeMap::new());

/// WICK: the flagged contracts of a loaded day: a contract whose cube day HIGH (the highest 1-minute HIGH of its bars) is below the SAME
/// session's NSE F&O bhavcopy HIGH (index into `c.cons` -> flagged, with that HIGH). With the switch off or no bhavcopy for the day: none.
pub fn wick_mark(c: &mut Cube) {
    if !on(WICK) {
        return;
    }
    let cached = WICK_DAY.lock().ok().and_then(|g| g.get(&c.day).cloned());
    let list = match cached {
        Some(l) => l,
        None => {
            let Some(bh) = crate::bhav::day_extremes(c) else { return };
            let (v, checked, rows) = wick_compare(c, &bh);
            if let Ok(mut m) = WICK_SEEN.lock() {
                for (k, x) in rows {
                    m.insert(k, x);
                }
            }
            let a = std::sync::Arc::new(v);
            if let Ok(mut g) = WICK_DAY.lock()
                && !g.contains_key(&c.day)
            {
                g.insert(c.day, std::sync::Arc::clone(&a));
                WICK_CHECKED.fetch_add(checked, Ordering::Relaxed);
            }
            a
        }
    };
    let mut bad = vec![false; c.cons.len()];
    for &(ci, _) in list.iter() {
        if let Some(b) = bad.get_mut(ci as usize) {
            *b = true;
        }
    }
    c.wickbad = bad;
}

/// WICK: the comparison of one day's cube against its bhavcopy extremes (contract index -> (HIGH, LOW)): the flagged contracts (index, the
/// bhavcopy HIGH), the contract-days compared, and the listing rows ((session, security id) -> (cube HIGH, bhavcopy HIGH, cube LOW, bhavcopy
/// LOW)). A contract is flagged when its cube day HIGH (the highest 1-minute HIGH of its bars) is BELOW the bhavcopy HIGH.
#[must_use]
pub fn wick_compare(c: &Cube, bh: &HashMap<usize, (i64, i64)>) -> (Vec<(u32, i64)>, u64, Vec<((i64, u32), (i64, i64, i64, i64))>) {
    let mut v: Vec<(u32, i64)> = Vec::new();
    let mut rows = Vec::new();
    let mut checked = 0_u64;
    for (ci, k) in c.cons.iter().enumerate() {
        let Some(&(bhi, blo)) = bh.get(&ci) else { continue };
        let (mut chi, mut clo) = (0_i64, i64::MAX);
        for m in 0..crate::cal::NM {
            let ix = Cube::ix(ci, m);
            if c.flags.get(ix).is_some_and(|f| f & 1 == 1) {
                chi = chi.max(i64::from(c.high.get(ix).copied().unwrap_or(0)));
                clo = clo.min(i64::from(c.low.get(ix).copied().unwrap_or(i32::MAX)));
            }
        }
        if chi == 0 {
            continue;
        }
        checked += 1;
        // MUTANT m_v34_wick_low_only: only the LOW side compared (a dropped HIGH wick, the side that hurts a short, missed)
        let hi_bad = if cfg!(feature = "m_v34_wick_low_only") { clo > blo } else { chi < bhi };
        if hi_bad {
            v.push((u32::try_from(ci).unwrap_or(u32::MAX), bhi));
            rows.push(((c.day, k.sid), (chi, bhi, clo, blo)));
        }
    }
    (v, checked, rows)
}

/// WICK: the bhavcopy HIGH of a flagged contract of session `day` (None: not flagged).
#[must_use]
pub fn wick_high(day: i64, ci: usize) -> Option<i64> {
    WICK_DAY.lock().ok().and_then(|g| g.get(&day).and_then(|l| l.iter().find(|x| x.0 as usize == ci).map(|x| x.1)))
}

/// WICK: `cube-vs-bhav.tsv` (every flagged contract-day with both differences; empty body when none).
#[must_use]
pub fn wick_tsv() -> String {
    let mut s = String::from("session\tsecurity_id\tcube_high_rs\tbhav_high_rs\tcube_low_rs\tbhav_low_rs\thigh_gap_rs\tlow_gap_rs\n");
    if let Ok(m) = WICK_SEEN.lock() {
        for ((d, sid), (ch, bh, cl, bl)) in m.iter() {
            s.push_str(&format!("{}\t{sid}\t{}\t{}\t{}\t{}\t{}\t{}\n", crate::cal::fmt_day(*d), crate::cal::fmt_rs(*ch), crate::cal::fmt_rs(*bh), crate::cal::fmt_rs(if *cl == i64::MAX { 0 } else { *cl }), crate::cal::fmt_rs(*bl), crate::cal::fmt_rs(bh - ch), crate::cal::fmt_rs(if *cl == i64::MAX { 0 } else { cl - bl })));
        }
    }
    s
}

/// WICK: whether contract `ci` of the day is flagged (its cube day HIGH below the bhavcopy's: its trades carry the flag and the bound).
#[inline]
#[must_use]
pub fn wick_flagged(c: &Cube, ci: usize) -> bool {
    on(WICK) && c.wickbad.get(ci).copied().unwrap_or(false)
}

/// WICK: the words (RESULTS, checklist row 14).
#[must_use]
pub fn wick_words() -> String {
    let m = WICK_SEEN.lock().map(|m| m.clone()).unwrap_or_default();
    format!("cube day extremes vs the NSE F&O bhavcopy (fix34 WICK, matrix X19): {} contract-days compared; {} with the cube HIGH below the bhavcopy HIGH FLAGGED WICK_DROPPED (listed in cube-vs-bhav.tsv; their trades carry the flag and the bound re-price){}", WICK_CHECKED.load(Ordering::Relaxed), m.len(), if m.is_empty() { String::new() } else { format!(": {}", m.iter().take(20).map(|((d, sid), (ch, bh, _, _))| format!("{} sid {sid} cube Rs {} < bhav Rs {}", crate::cal::fmt_day(*d), crate::cal::fmt_rs(*ch), crate::cal::fmt_rs(*bh))).collect::<Vec<_>>().join("; ")) })
}

// ------------------------------------------------------------------------------------------------ sizing DIALS (matrices X07 / X11 / X12)

/// The sizing dials on one slot count's margin fit (every one off: `fit` unchanged): RISKCAP lots <= floor(budget / (move x price x lot))
/// with move = the stop + overshoot in bp of the decision price (the option's close at the last minute ended by T); EXPDIAL half size on an
/// index option's own expiry day; EVENTCAL half size on a dated event session.
#[must_use]
pub fn dial_lots(fit: i64, c: &Cube, ci: usize, t: u32, idx_expiry_day: bool, event_day: bool) -> i64 {
    let mut f = fit;
    if on(RISKCAP) {
        let b = (i64::from(t) - crate::cal::OPEN_S) / 60;
        let px = if b >= 1 { i64::from(c.close_at(ci, usize::try_from(b - 1).unwrap_or(0))) } else { 0 };
        let lot = c.cons.get(ci).map_or(0, |k| i64::from(k.lot));
        let mv = i64::try_from(RISK_OVER_BP.load(Ordering::Relaxed)).unwrap_or(i64::MAX);
        let budget = i64::try_from(RISK_BUDGET.load(Ordering::Relaxed)).unwrap_or(0);
        // loss of one lot at the move = move_bp / 10,000 x price x lot (paisa); lots = floor(budget / that)
        let per_lot = i128::from(mv) * i128::from(px) * i128::from(lot);
        // MUTANT m_v34_riskcap_ignored: the budget computed but the margin fit kept
        if per_lot > 0 && !cfg!(feature = "m_v34_riskcap_ignored") {
            let cap = i64::try_from(i128::from(budget) * 10_000 / per_lot).unwrap_or(i64::MAX);
            f = f.min(cap);
        }
    }
    if on(EXPDIAL) && idx_expiry_day && EXP_HALF.load(Ordering::Relaxed) == 1 {
        f /= 2;
    }
    if on(EVENTCAL) && event_day && EVENT_MODE.load(Ordering::Relaxed) == 1 {
        f /= 2;
    }
    f.max(0)
}

// ------------------------------------------------------------------------------------------------ READINGS (matrices X14 / X18 / X25 / X26)

/// READINGS (matrix X26): one trade for the intraday MTM drawdown: its session, decision and last-fill instants, units and value sold, the
/// contract-day's marks offset (1 + offset; 0 = none) and its net.
#[derive(Clone, Copy, Debug, Default)]
pub struct MtmTrade {
    pub day: i64,
    pub t0_us: i64,
    pub t1_us: i64,
    pub units: u64,
    pub sold: i64,
    pub mark1: u32,
    pub net: i64,
}

/// READINGS (matrix X26): the max drawdown of the equity marked at every trading minute (realized net of the closed trades + each open
/// short's value sold - units x its minute mark, the X22 marks: the minute HIGH, else the carried close) and the largest running loss of the
/// open shorts in one minute. A trade is open from its decision's minute to the minute holding its last fill; it realizes after that
/// minute. Equity starts at 0; at least the close-instant drawdown when every mark sits at the fills (`sf22_intraday_mtm_drawdown_at_least_realised`).
#[must_use]
pub fn intraday_mtm_dd(trades: &[MtmTrade], marks: &[i32]) -> (i64, i64) {
    let mut by: BTreeMap<i64, Vec<MtmTrade>> = BTreeMap::new();
    for t in trades {
        by.entry(t.day).or_default().push(*t);
    }
    let (mut realized, mut peak, mut dd, mut worst_loss) = (0_i64, 0_i64, 0_i64, 0_i64);
    let nm = usize::from(crate::kern::CAP_NM);
    for (day, ts) in by {
        let span: Vec<(usize, usize)> = ts
            .iter()
            .map(|t| {
                let a = usize::from(crate::kern::cap_minute(u32::try_from((t.t0_us - crate::cal::at_sod(day, 0)) / common_fill::US_S).unwrap_or(0)));
                let b = usize::from(crate::kern::cap_end(u32::try_from((t.t1_us - crate::cal::at_sod(day, 0)) / common_fill::US_S).unwrap_or(0))).max(a + 1);
                (a, b)
            })
            .collect();
        let mut day_real = 0_i64;
        for b in 0..nm {
            let mut open_pl = 0_i64;
            let mut open_loss = 0_i64;
            for (t, &(a, e)) in ts.iter().zip(&span) {
                if b == e {
                    // the trade realized at the end of its last minute
                    day_real += t.net;
                }
                if a <= b && b < e && t.mark1 > 0 {
                    let mk = marks.get(t.mark1 as usize - 1 + b.min(crate::cal::NM - 1)).copied().unwrap_or(0);
                    if mk > 0 {
                        let pl = t.sold - i64::from(mk) * i64::try_from(t.units).unwrap_or(0);
                        open_pl += pl;
                        open_loss += (-pl).max(0);
                    }
                }
            }
            // MUTANT m_v34_mtm_dd_closes_only: the open shorts' marks left out (the close-instant reading again)
            let eq = realized + day_real + if cfg!(feature = "m_v34_mtm_dd_closes_only") { 0 } else { open_pl };
            peak = peak.max(eq);
            dd = dd.max(peak - eq);
            worst_loss = worst_loss.max(open_loss);
        }
        // trades whose last fill is at or after the cap's last minute realize at the day's end
        for (t, &(_, e)) in ts.iter().zip(&span) {
            if e >= nm {
                day_real += t.net;
            }
        }
        realized += day_real;
        peak = peak.max(realized);
        dd = dd.max(peak - realized);
    }
    (dd, worst_loss)
}

/// READINGS (matrix X14): the first minute's HIGH (the first bar of the session) of the contract with security id `sid` in cube `c`.
#[must_use]
pub fn first_minute_high(c: &Cube, sid: u32) -> Option<i64> {
    // MUTANT m_v34_x14_no_reprice: the next session never read (every never-closed unit left unstressed)
    if cfg!(feature = "m_v34_x14_no_reprice") {
        return None;
    }
    let ci = c.cons.iter().position(|k| k.sid == sid)?;
    (0..crate::cal::NM).find_map(|m| {
        let ix = Cube::ix(ci, m);
        c.flags.get(ix).is_some_and(|f| f & 1 == 1).then(|| i64::from(c.high.get(ix).copied().unwrap_or(0)))
    })
}

/// READINGS (matrix X18): the stress set (AUDIT-EXTREME-MATRIX 7: the COVID crash and halts, the Budget Saturday, the 2021 outage, the 2022
/// invasion day, the 2024 election results day).
#[must_use]
pub fn stress_set() -> Vec<i64> {
    // MUTANT m_v34_x18_set_empty: no stress session known (every run reads STRESS SET NOT REPLAYED)
    if cfg!(feature = "m_v34_x18_set_empty") {
        return Vec::new();
    }
    [(2020, 2, 1), (2020, 3, 12), (2020, 3, 13), (2020, 3, 16), (2020, 3, 23), (2021, 2, 24), (2022, 2, 24), (2024, 6, 4)].iter().map(|&(y, m, d)| crate::cal::days_from_civil(y, m, d)).collect()
}

/// WICK (matrix X19): the bound's extra cost of a trade on a flagged contract-day: every unit bought back (or valued) at the bhavcopy HIGH
/// (the day's highest print) instead of what was paid (paisa, before charges; 0 when it paid at least that).
#[must_use]
pub fn wick_bound_extra(units: u64, bought: i64, wick_hi: i64) -> i64 {
    (i64::try_from(units).unwrap_or(0).saturating_mul(wick_hi) - bought).max(0)
}

/// TAILGATE (matrix X17, a GATE: off unless MAIN's limits are given): a headline account's worst trade loses at most W% of its slot (the
/// sweep holds every account's worst trade; the p99 trade loss is printed per pick beside and gated in the pick lines).
#[must_use]
pub fn tail_ok(sm: &crate::acct::Sum) -> bool {
    if !on(TAILGATE) {
        return true;
    }
    let slot = crate::trade::ACCOUNT / crate::acct::n_of_s(usize::from(sm.seg), usize::from(sm.n)).max(1);
    let w = i64::try_from(TAIL_W.load(Ordering::Relaxed)).unwrap_or(100);
    // MUTANT m_v34_tailgate_ignored: the gate read but never applied
    cfg!(feature = "m_v34_tailgate_ignored") || sm.worst >= 0 || (-sm.worst).saturating_mul(100) <= w.saturating_mul(slot)
}

/// READINGS (matrix X25): the best move our way before the re-spike peak at `peak_at` (the lowest LOW over [e0, peak_at), bp below the sale's
/// average price `avg`; 0 when no print or no move our way).
#[must_use]
pub fn best_move_bp(tape: &common_fill::Tape<'_>, e0: i64, peak_at: i64, avg: i64) -> i64 {
    if avg <= 0 {
        return 0;
    }
    tape.extreme(e0, peak_at.max(e0 + 1), false).map_or(0, |(lo, _)| ((avg - lo) * 10_000 / avg).max(0))
}

/// READINGS (matrix X25): a STOP HUNT: the stop fired at second `trig_us` and the very next second's HIGH traded back below the stop level
/// (the spike reverted at once). A next second with no print is not a hunt (nothing says it reverted).
#[must_use]
pub fn stop_hunt(tape: &common_fill::Tape<'_>, trig_us: i64, level: i64) -> bool {
    // MUTANT m_v34_hunt_off: stop hunts never counted
    !cfg!(feature = "m_v34_hunt_off") && level > 0 && tape.extreme(trig_us + common_fill::US_S, trig_us + 2 * common_fill::US_S, true).is_some_and(|(h, _)| h < level)
}

/// TTWORDS (exits F-H1): the EXIT_WHY family word of a TT exit: the time-armed breakeven stop (v3.3's "trailing target trigger" with the
/// switch off).
#[must_use]
pub fn tt_word() -> &'static str {
    // MUTANT m_v34_tt_words_trailing: the old words kept under the switch
    if on(TTWORDS) && !cfg!(feature = "m_v34_tt_words_trailing") { "time-armed breakeven stop" } else { "trailing target trigger" }
}

/// ELMPRE: whether the margin of contract `ci` on session `ymd` was priced at the dated pre-AEL rule (a stock with no AEL rate that day).
#[must_use]
pub fn elm_pre_used(ymd: u32, ci: usize) -> bool {
    on(ELMPRE) && ELM_USED.lock().is_ok_and(|s| s.contains(&(ymd, u32::try_from(ci).unwrap_or(u32::MAX))))
}

/// ELMPRE: the rule words of the RUN.log / RESULTS header: the rule the dated table APPLIES (NSE's regime x sqrt 2 + 0.80%, or the
/// additional-ELM bound), never Addendum 4's literal rule alone (that one sits beside in the table, unused). Found on the plain3 runs'
/// header at 14:1x (the per-trade label was already right); words only, no figure moves.
#[must_use]
pub fn elm_pre_rule_words() -> &'static str {
    if cfg!(feature = "m_v34_elm_words_literal") {
        return "max of 5% and 1.5 sd of daily log returns over the previous 6 months, rolling and month-end; Addendum 4";
    }
    "NSE's regime: max of 5% and 1.5 sd of daily log returns over the previous 6 months (rolling and month-end) x sqrt 2 + 0.80% safety, or the stock's highest first-quarter AEL total when higher (the bound for the unpublished additional ELM); Addendum 4's literal rule (no sqrt 2) sits beside in the table, not used"
}

/// ELMPRE: the trade's exposure words when the dated rule priced it (Addendum 4's label, the first AEL file's TOTAL rate beside).
#[must_use]
pub fn elm_pre_words(ymd: u32, sym: &str) -> String {
    let beside = ELM_T.get().and_then(|t| t.rows.get(&(ymd, sym.to_owned())).map(|x| x.1)).filter(|&b| b >= 0);
    format!("exposure from NSE's rule, no AEL file (the dated table: max(5%, 1.5 sd) x sqrt 2 + 0.80%, or the stock's first-quarter AEL total as the bound for the unpublished additional ELM; Addendum 4){}", beside.map_or_else(String::new, |b| format!("; the first AEL file's (2019-06-14) total {}.{:02}% beside", b / 100, b % 100)))
}

/// HALTS: the words of a trade whose decision falls before a halt of its session and whose last fill is known after its start (the walk
/// crossed the halt: no block spans it, the carried entitlement reset, the rest filled from the resumption); "-" otherwise or off.
#[must_use]
pub fn halt_trade_words(day: i64, t_sod: i64, known_sod: i64) -> String {
    let h: Vec<String> = halts_of(day).iter().filter(|&&(from, _)| t_sod < from && known_sod >= from).map(|&(from, resume)| format!("crossed the halt {}-{} (no block spans it; the carried entitlement reset; the rest filled from the resumption)", crate::cal::fmt_sod(from), crate::cal::fmt_sod(resume))).collect();
    if h.is_empty() { "-".to_owned() } else { h.join("; ") }
}

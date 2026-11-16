//! `sniper_host`: the THIN wiring of the ADANIENT sniper (SNIPER-ADANIENT-PLAN v2; SNIPER-REQUIREMENTS s11 (1): "The sniper
//! host is only the thin wiring between them. It is never a second copy of any law").
//!
//! | Module | Role (plan section) | Law it calls |
//! |---|---|---|
//! | [`rows`] | the data rows: RUN, VIEWS, WORDS, the dials file's core ladders, the exits file, common_wf's WINDOWS rows | none (data) |
//! | [`law`] | the fill-law table: `req70-2s` only; the money-cap law refused at load (S3, SN-X45, m-S17) | crate v2 names |
//! | [`cands`] | the candidate rows of one (timeframe, side): RAW readings from `common_feat` on Zerodha minute candles (S1), exact buckets, outer dial values, exit anchors | `common_feat` |
//! | [`ctxdials`] | DIALS BUILD: the seven context, calendar and range readings host v1 left `live_not_wired` (X-NIFTY, X-VIX, D-EXP, D-MEND, G-52H, G-52L, G-DRNG), each from data arrived by S (K11); none read under `--context-dials off` | `common_feat` (`Atr`, CA factors), `feat_io` (the store) |
//! | [`price`] | one (row, side, exit set) priced through crate v2 `walk` + `path::scan_one` (never `FillPath::cut_at`, S2: a winning stop re-walks the entry), Dhan charges, the 112-B / 352-B outcome rows | `common_fill` v2, `costs` |
//! | [`sweep`] | S0 + S1 of one context over all worker threads: the kernel's LINE engine into `wf_sweep::SweepAcc` + `Lists` | `sniper_kernel`, `wf_sweep` (D-S14) |
//! | [`twin`] | the random-entry twin's draws: one seeded stream per (seed, session), so a day appended never moves an earlier draw (S9 x F10) | none |
//! | [`newday`] | the NEW-DAY path (S9): fold, price, append a partition; picks never change | the above |
//! | [`db`] | TSV partitions written once (temp, fsync, rename; refused if present) with the hash-chained AUDIT; a row without its standing labels is refused | none |
//! | [`query`] | `sniper-q`: fixed-width offset indexes, O(1) a lookup | none |
//! | [`monitor`] | the launch gate (memory pressure, swap growth and free swap, disk floor), HEARTBEAT.tsv | none |
//! | [`search`] | the RUN driver of one family: every N exact, the merged lists re-walked into `common_wf`'s Lists, picks, verdicts and words from `common_wf`, the random-entry twin, DB rows | `common_wf` |
//! | [`wfio`] | the `common_wf` inputs: calendar, RULES, partitions, its own windows, views and words files | `common_wf` |
//! | [`words`] | every printed line from WORDS-SNIPER-v1.tsv (s12, S3, S6, fixed_after, trade-count words, proof step) | none |
//! | [`sha`] | SHA-256 for the audit chain and the identity lines | none |
//! | [`size`] | SNIPER s16: the trade's whole count by the RUN row `qty_rule` (closest, half way the lower; or floor = crate S11's `size_units`), the account base cut (Q7) | `common_fill::limits::size_units` (floor), `common_fill::money::money_in` |
#![cfg_attr(test, allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod cands;
pub mod ctxdials;
pub mod db;
pub mod esn1;
// S7 BUILDER wave 4: the INDEX OPTIONS modes (NIFTY / BANKNIFTY, intraday and positional) on host v3.
pub mod idx;
// S7 BUILDER wave 4: the INDEX POSITIONAL mode (lane B's groups + opt_pos::pos per candidate, OUT-POS 792 B, Layout::new_pos).
pub mod idxpos;
pub mod instr;
pub mod law;
pub mod monitor;
pub mod newday;
// S7 BUILDER wave 4: lane B's positional groups (Supertrend 10 x 3 episodes, XS / FLIP exits, FIRST per group), copied byte for byte
// from scratch/sniper-pos-v2 (its P3 mutants m_pos_look / m_pos_group / m_pos_sess included).
pub mod posgroups;
pub mod price;
pub mod query;
pub mod rows;
pub mod search;
pub mod sec;
pub mod secrun;
pub mod sha;
pub mod size;
pub mod sweep;
pub mod twin;
pub mod wfio;
pub mod words;
#[cfg(feature = "m_s16")]
pub mod mutant_s16;

/// The host's version line.
pub const VERSION: &str = "sniper-host-v1 1.0.0";

/// The seconds driver's version line (SECONDS BUILDER, 2026-11-14; the minute families keep [`VERSION`] byte for byte).
pub const SEC_VERSION: &str = "sniper-host-v1-sec-b 1.0.0 (seconds families 3/5/10/15/30 s: compact rows, design change (d); repair round: common_feat repair 2, halt-window volume-0 seconds dropped)";

/// The law tags of every linked module (printed in SN1 and on every result).
#[must_use]
pub fn law_line() -> String {
    format!(
        "fill={} ({}, crate {} step {}); feat={} (pinned repair 3 = S4 index rows on repair 2, SHA256SUMS-r3 b05fafb38dab24d0); kernel={}; sweep={}; wf={} (LINKED; pinned repair 2, SHA256SUMS 5e01f0cf8fc749c7)",
        common_fill::law_compiled(),
        common_fill::LAW_STD,
        common_fill::VERSION,
        common_fill::STEP,
        common_feat::LAW,
        sniper_kernel::law(),
        if wf_sweep::RED { "wf-sweep-v1-RED" } else { "wf-sweep-v1 (D-S14 staged)" },
        common_wf::VERSION
    )
}

/// Every search door: refuses a red or mutant build of any linked module (crate v2, common_feat, the kernel, the sweep).
///
/// # Errors
/// The first refusal, named.
pub fn refuse_if_red() -> Result<(), String> {
    common_fill::refuse_if_red().map_err(|e| e.to_string())?;
    common_feat::refuse_if_red().map_err(str::to_string)?;
    sniper_kernel::refuse_if_red().map_err(str::to_string)?;
    wf_sweep::refuse_if_red().map_err(|e| e.to_string())?;
    common_wf::refuse_if_red().map_err(|e| e.to_string())?;
    if common_wf::MUTANT {
        return Err("common_wf built with a named mutant: no search run".to_string());
    }
    if cfg!(any(feature = "m_s16", feature = "m_s17", feature = "m_rw", feature = "m_s4", feature = "m_h1_reset", feature = "m_h1_entry", feature = "m_b2", feature = "m_b3", feature = "m_s16q", feature = "m_s16a", feature = "m_sec_eq", feature = "m_sec_htf", feature = "m_xc_flip", feature = "m_xc_clock", feature = "m_k11", feature = "m_xc_cut", feature = "m_ix_sq", feature = "m_s7w4_refuse_idx", feature = "m_d2_broker_sq", feature = "m_pos_look", feature = "m_pos_group", feature = "m_pos_sess", feature = "m_pos_mk", feature = "m_pos_future_refusal")) {
        return Err("sniper_host built with a named mutant: no search run".to_string());
    }
    Ok(())
}

/// The IST time label, always from the `date` command (MAIN's rule: every IST label comes from `date`).
#[must_use]
pub fn ist_now() -> String {
    std::process::Command::new("date")
        .env("TZ", "Asia/Kolkata")
        .arg("+%F %T IST")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "date-unavailable".to_string(), |s| s.trim().to_string())
}

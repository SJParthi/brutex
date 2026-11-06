//! SEARCH-SPEC 2.3: the bare cores re-create the preview. (K3, X1), (K4, X2), (K5, X2)
//! with no filter, on LONG, SHORT and BOTH, every timeframe, through T1's own `admit` and
//! `metrics` (`prev.rs`, copied unchanged), must print the preview's rows.tsv line, figure
//! for figure. Any difference stops the run by name before any result is written.

use std::collections::HashMap;
use std::path::Path;

use crate::common::{ACCOUNT_NAMES, Cand, N_MONTHS, RS_NAMES, Status, TFS, fmt_date};
use crate::phase_a::Sym;
use crate::prev::{AccountOut, Stats, admit, metrics, passes};
use crate::spool::load_full;

/// The preview's rows.tsv data lines keyed by (rule set, timeframe, account).
pub(crate) fn read_rt(path: &Path) -> Result<HashMap<(String, String, String), String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut m = HashMap::new();
    for line in text.lines() {
        if line.starts_with('#') || line.starts_with("rule_set\t") || line.is_empty() {
            continue;
        }
        let mut it = line.split('\t');
        let (Some(a), Some(b), Some(c)) = (it.next(), it.next(), it.next()) else { continue };
        m.insert((a.to_owned(), b.to_owned(), c.to_owned()), line.to_owned());
    }
    Ok(m)
}

/// T1's rows.tsv line format.
pub(crate) fn row_line(rs: usize, ti: usize, acct: usize, out: &AccountOut, st: &Stats) -> String {
    let o = |x: Option<i128>| x.map_or_else(|| "NA".to_owned(), |v| v.to_string());
    let e = |i: usize| st.exits.get(i).copied().unwrap_or(0);
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        RS_NAMES.get(rs).copied().unwrap_or("?"),
        TFS.get(ti).copied().unwrap_or(0),
        ACCOUNT_NAMES.get(acct).copied().unwrap_or("?"),
        if passes(st) { "PASS" } else { "MISS" },
        st.months_pass,
        N_MONTHS - st.months_pass,
        st.months_pos,
        o(st.worst_bp),
        o(st.median_bp),
        o(st.best_bp),
        st.max_dd_bp,
        st.max_dd_paisa,
        fmt_date(st.dd_peak_ts),
        fmt_date(st.dd_low_ts),
        st.total_net,
        st.total_bp,
        st.trades,
        st.wins,
        st.losses,
        o(st.win_pm),
        o(st.avg_win),
        o(st.avg_loss),
        o(st.pf_th),
        st.charges,
        st.gross,
        out.signals,
        out.while_open,
        out.slots_full,
        out.not_filled,
        out.zero_qty,
        out.no_ratio,
        st.late,
        e(0),
        e(1),
        e(2),
        e(3),
        e(4),
        st.fallback_trades,
        st.fallback_stock_days
    )
}

/// Check one timeframe's nine bare-core rows against the preview's lines; returns the
/// report lines and the number of mismatches.
pub(crate) fn check_tf(out: &Path, syms: &[Sym], ti: usize, rt: &HashMap<(String, String, String), String>, start_ts: i64) -> Result<(Vec<String>, usize), String> {
    let mut lines = Vec::new();
    let mut bad = 0_usize;
    // rule set -> (core bit, lane)
    for (rs, core, lane) in [(0_usize, 2_usize, 0_usize), (1, 3, 1), (2, 4, 1)] {
        let full = load_full(out, ti, lane, syms)?;
        let mut cands: Vec<Cand> = full
            .iter()
            .filter(|r| r.cores & (1 << core) != 0)
            .map(|r| {
                let t = r.tr.first().copied().unwrap_or_default();
                Cand {
                    t_signal: r.t,
                    stock: r.stock,
                    rs: u8::try_from(rs).unwrap_or(0),
                    tf: u8::try_from(ti).unwrap_or(0),
                    side: r.side,
                    status: t.status,
                    exit_kind: t.kind,
                    fallback: t.fallback,
                    late: t.late,
                    entry_us: t.entry_us,
                    entry_px: t.entry_px,
                    qty: t.qty,
                    exit_us: t.exit_us,
                    exit_px: t.exit_px,
                    gross: if t.status == Status::Filled { t.gross } else { 0 },
                    charges: if t.status == Status::Filled { t.charges } else { 0 },
                    net: if t.status == Status::Filled { t.net } else { 0 },
                    stop0: t.stop0,
                    sig: r.sig,
                }
            })
            .collect();
        cands.sort_by_key(|c| (c.t_signal, c.stock, c.side));
        for acct in 0..3 {
            let o = admit(&cands, acct);
            let st = metrics(&cands, &o.trades, start_ts);
            let mine = row_line(rs, ti, acct, &o, &st);
            let key = (
                RS_NAMES.get(rs).copied().unwrap_or("?").to_owned(),
                TFS.get(ti).copied().unwrap_or(0).to_string(),
                ACCOUNT_NAMES.get(acct).copied().unwrap_or("?").to_owned(),
            );
            match rt.get(&key) {
                Some(theirs) if *theirs == mine => {
                    lines.push(format!("{} {} min {}: EQUAL ({} trades, net {} paisa)", key.0, key.1, key.2, st.trades, st.total_net));
                }
                Some(theirs) => {
                    bad += 1;
                    let diff: Vec<String> = mine
                        .split('\t')
                        .zip(theirs.split('\t'))
                        .enumerate()
                        .filter(|(_, (a, b))| a != b)
                        .map(|(i, (a, b))| format!("column {i}: search {a} preview {b}"))
                        .collect();
                    lines.push(format!("{} {} min {}: DIFFERS: {}", key.0, key.1, key.2, diff.join("; ")));
                }
                None => {
                    bad += 1;
                    lines.push(format!("{} {} min {}: no preview line to compare", key.0, key.1, key.2));
                }
            }
        }
    }
    Ok((lines, bad))
}

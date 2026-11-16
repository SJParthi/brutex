//! DB rows and audit (section 13): the crate BUILDS the rows (frozen field orders, schema `cwf-rows-v6` = repair 3's
//! `cwf-rows-v5` plus A2's `row_provenance` / `partition_provenance`, B4's `side_setting` and A3's `fixed_after`); the host's DB
//! module writes them through a [`Sink`] (no I/O here). Every file is TSV with its frozen header, written once (temp then rename,
//! refused if the target exists). W8 is the audit: one hash-chained file per run_id, ONE writer, each row carrying the SHA-256 of
//! the row before it, so any edit is visible; a correction is a new run plus a SUPERSEDE row. The MANIFEST lists every
//! partition's SHA-256 and row count. SHA-256 is implemented here (std only) and checked against the node reference's
//! `crypto` digests in the tests.

use crate::WfErr;

/// The schema name.
pub const SCHEMA: &str = "cwf-rows-v6";

/// W1 `wf_runs`.
pub const W1: &[&str] = &["run_id", "schema", "tool", "lane", "round", "profile", "wf_compiled", "crate_version", "common_wf_sha", "windows_sha", "views_sha", "words_sha", "calendar_sha", "rules_sha", "vectors_sha", "matrix_sha", "binary_sha", "common_fill_sha_or_na", "trade_order", "hole_source", "alt_reading", "booking", "grids_or_none", "grid_main_lines_or_none", "mtm_marks", "comparison_partitions_or_none", "headline_partition_or_none", "window_sets", "partitions", "worlds", "lists_bound_bytes", "subjects_walked", "offers", "fixed_subjects", "started_ist", "finished_ist", "load_at_start", "supersedes_run_id", "views_provenance", "headline_partition_provenance", "fixed_grid_members_sha", "fixed_overlaps", "d49_reading", "d50_reading", "windows_provenance", "views_file_and_header", "paper_from", "d55_reading", "d56_reading"];
/// W2 `wf_windows`.
pub const W2: &[&str] = &["run_id", "window_id", "kind", "honest", "test_name", "learn_from", "learn_to", "test_from", "test_to", "gap_months", "window_set", "pieces", "prefix_point", "anchored", "learn_parts", "test_parts", "missing_parts", "walked_sessions_per_part", "not_run_reason", "provenance", "gap_words", "fixed_after"];
/// W3 `wf_picks`.
pub const W3: &[&str] = &["run_id", "window_set", "partition", "partition_role", "world", "reading", "view_id", "window_id", "role", "rank", "combo_code", "tickets", "ticket_grids_and_provenance", "key_names", "key_values", "ties_at_1", "eligible", "offered", "rejected_read_set", "rejected_no_grid", "rejected_chosen_grid", "learn_trades", "learn_sessions", "learn_net", "learn_maxdd_trade", "learn_maxdd_session", "learn_maxdd_mtm_or_na", "dd_basis", "open_at_learn_end", "open_mark_at_learn_end", "learn_worst_rd_bp", "learn_months_not_pos", "learn_worst_month", "learn_fingerprint", "own_verdict_code", "own_words", "test_months_pos", "test_months", "test_worst_month", "cross_window_id", "cross_reading", "cross_figures_words", "same_combo", "proof_step", "data_labels", "label", "margin_words", "facts_sha", "rewalk_equal", "cross_all_pos", "tt_verdict_code", "tt_words_or_na", "fixed_twin_or_na", "rule_provenance", "tickets_dominated", "replay_levels_or_na", "replay_all_passed_or_na", "row_provenance", "partition_provenance"];
/// W4 `wf_parts` (one row per shown pick and year unit).
pub const W4: &[&str] = &["run_id", "partition", "view_id", "rank", "part", "part_label", "months", "walked_sessions", "nodata", "trades", "carried", "straddling", "net", "hole_trades", "hole_win_net", "second_net_or_na", "booked_pos", "second_pos", "best_trade", "unclosable", "refused_signals_or_na", "in_learn", "in_test"];
/// W5 `wf_misses`.
pub const W5: &[&str] = &["run_id", "partition", "view_id", "rank", "combo_code", "missed_checks"];
/// W6 `wf_luck`.
pub const W6: &[&str] = &["run_id", "partition", "view_id", "test_top1_bp", "test_top3_bp", "test_top5_bp", "learn_top1_bp", "learn_top3_bp", "learn_top5_bp", "flips", "eligible", "offered", "views", "partitions", "twin_seeds", "twin_all_pos", "mirror_code_or_na", "mirror_all_pos_or_na", "words"];
/// W7 `wf_agg` (kinds `view` and `grid_stage`).
pub const W7: &[&str] = &["run_id", "kind", "window_set", "partition", "view_id", "offered", "eligible", "kept", "rejected_read_set", "rejected_no_grid", "rejected_chosen_grid", "fixed_offered", "rejected_no_learn_trade", "rejected_learn_year", "rejected_floor", "steps_add", "steps_mtm", "steps_offer", "steps_insert", "steps_key_per_shape", "members_walked", "fixed_overlap_excluded", "tickets_dominated", "too_many_tickets", "grid", "stage", "members_declared", "stage_dropped", "offered_derived", "dropped_fixed_overlap"];
/// W8 `audit`.
pub const W8: &[&str] = &["seq", "ist", "run_id", "action", "path", "sha256", "rows", "bytes", "writer", "binary_sha", "prev_row_sha256", "note"];
/// W9 `wf_partitions` (A2: a provenance per partition value; B4: `side_setting`).
pub const W9: &[&str] = &["run_id", "partition", "partition_role", "world", "reading", "underlying", "signal_source", "instrument", "size_rule", "fill_law", "fill_level", "timing", "family", "timeframe", "exit_set", "period_from", "period_to", "proof_step", "data_labels", "account", "universe", "rank_set", "share_level", "timeframe_family", "side_setting", "partition_provenance"];

/// The frozen header line of a table.
#[must_use]
pub fn header(cols: &[&str]) -> String {
    cols.join("\t")
}

/// One TSV line of a row; refuses a row whose field count is not the table's or whose field holds a tab or a newline.
///
/// # Errors
/// `BadRow(R, fields, 20)`.
pub fn line(cols: &[&str], fields: &[String]) -> Result<String, WfErr> {
    if fields.len() != cols.len() || fields.iter().any(|f| f.contains('\t') || f.contains('\n')) {
        return Err(WfErr::BadRow { file: b'R', row: u32::try_from(fields.len()).unwrap_or(u32::MAX), rule: 20 });
    }
    Ok(fields.join("\t"))
}

/// The host's writer: write-once files (a temp file, then rename; refused if the target exists).
pub trait Sink {
    /// Writes `body` to `path` once.
    ///
    /// # Errors
    /// The sink's own refusal.
    fn write_once(&mut self, path: &str, body: &str) -> Result<(), WfErr>;
}

/// W8: one run's hash-chained audit (ONE writer).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Audit {
    run_id: String,
    writer: String,
    binary_sha: String,
    rows: Vec<String>,
}
/// The audit actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// The run opens.
    OpenRun,
    /// A partition file is written.
    WritePartition,
    /// A verify step.
    Verify,
    /// An independent check.
    Check,
    /// A correction: this run supersedes another.
    Supersede,
    /// The run closes.
    CloseRun,
}
impl Action {
    const fn word(self) -> &'static str {
        match self {
            Self::OpenRun => "OPEN_RUN",
            Self::WritePartition => "WRITE_PARTITION",
            Self::Verify => "VERIFY",
            Self::Check => "CHECK",
            Self::Supersede => "SUPERSEDE",
            Self::CloseRun => "CLOSE_RUN",
        }
    }
}
impl Audit {
    /// A new audit of `run_id` by `writer`.
    #[must_use]
    pub fn new(run_id: &str, writer: &str, binary_sha: &str) -> Self {
        Self { run_id: run_id.to_owned(), writer: writer.to_owned(), binary_sha: binary_sha.to_owned(), rows: Vec::new() }
    }
    /// Appends one row; its `prev_row_sha256` is the SHA-256 of the previous row's line ("-" for the first).
    ///
    /// # Errors
    /// `BadRow(R, n, 20)` a field with a tab or a newline.
    #[allow(clippy::too_many_arguments, reason = "the W8 columns")]
    pub fn append(&mut self, ist: &str, action: Action, path: &str, sha256: &str, rows: u64, bytes: u64, note: &str) -> Result<&str, WfErr> {
        let prev = self.rows.last().map_or_else(|| "-".to_owned(), |l| hex(&sha256_bytes(l.as_bytes())));
        let f = vec![(self.rows.len() + 1).to_string(), ist.to_owned(), self.run_id.clone(), action.word().to_owned(), path.to_owned(), sha256.to_owned(), rows.to_string(), bytes.to_string(), self.writer.clone(), self.binary_sha.clone(), prev, note.to_owned()];
        let l = line(W8, &f)?;
        self.rows.push(l);
        Ok(self.rows.last().map_or("", String::as_str))
    }
    /// The audit file body (header + rows).
    #[must_use]
    pub fn body(&self) -> String {
        let mut s = header(W8);
        for r in &self.rows {
            s.push('\n');
            s.push_str(r);
        }
        s.push('\n');
        s
    }
}
/// Verifies an audit body: the header, the sequence and every row's link to the row before it.
///
/// # Errors
/// `BadRow(R, row, 21)` the first row that breaks the chain.
pub fn verify_audit(body: &str) -> Result<u64, WfErr> {
    let mut lines = body.lines();
    if lines.next() != Some(header(W8).as_str()) {
        return Err(WfErr::BadRow { file: b'R', row: 0, rule: 21 });
    }
    let mut prev: Option<&str> = None;
    let mut n = 0u64;
    for l in lines.filter(|l| !l.is_empty()) {
        n += 1;
        let f: Vec<&str> = l.split('\t').collect();
        let want = prev.map_or_else(|| "-".to_owned(), |p| hex(&sha256_bytes(p.as_bytes())));
        if f.len() != W8.len() || f.first().copied() != Some(n.to_string().as_str()) || f.get(10).copied() != Some(want.as_str()) {
            return Err(WfErr::BadRow { file: b'R', row: u32::try_from(n).unwrap_or(u32::MAX), rule: 21 });
        }
        prev = Some(l);
    }
    Ok(n)
}

/// Verifies the audit of a FINISHED run (repair of check-build-v1 F7): [`verify_audit`]'s header, sequence and chain, and the
/// run's ONE `CLOSE_RUN` row is its last row. The chain alone cannot see a cut tail (rows 1-4 of 5 still link); a finished run's
/// audit that does not end at its `CLOSE_RUN` is refused. [`verify_audit`] stays the check of an audit still being written.
///
/// # Errors
/// [`verify_audit`]'s; `BadRow(R, n, 22)`: the last row `n` is not `CLOSE_RUN` (n = 0: no row), or a `CLOSE_RUN` row `n` is
/// not the last row.
pub fn verify_closed_audit(body: &str) -> Result<u64, WfErr> {
    let n = verify_audit(body)?;
    let close = Action::CloseRun.word();
    let mut last = 0u64;
    for (i, l) in body.lines().skip(1).filter(|l| !l.is_empty()).enumerate() {
        let seq = u64::try_from(i).unwrap_or(u64::MAX).saturating_add(1);
        let is_close = l.split('\t').nth(3) == Some(close);
        if is_close && seq != n {
            return Err(WfErr::BadRow { file: b'R', row: u32::try_from(seq).unwrap_or(u32::MAX), rule: 22 });
        }
        if is_close {
            last = seq;
        }
    }
    if n == 0 || last != n {
        return Err(WfErr::BadRow { file: b'R', row: u32::try_from(n).unwrap_or(u32::MAX), rule: 22 });
    }
    Ok(n)
}

/// The MANIFEST of a run: per partition file its SHA-256 and rows; then the W1 row.
#[must_use]
pub fn manifest(files: &[(&str, &str)], w1_row: &str) -> String {
    let mut s = String::from("path\tsha256\trows\tbytes\n");
    for (p, body) in files {
        let rows = body.lines().filter(|l| !l.is_empty()).count().saturating_sub(1);
        s.push_str(&format!("{p}\t{}\t{rows}\t{}\n", hex(&sha256_bytes(body.as_bytes())), body.len()));
    }
    s.push_str("#W1\t");
    s.push_str(w1_row);
    s.push('\n');
    s
}

/// Lower-case hex.
#[must_use]
pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

const K: [u32; 64] = [
    0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4, 0xab1c_5ed5, 0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3, 0x72be_5d74, 0x80de_b1fe, 0x9bdc_06a7, 0xc19b_f174, 0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc, 0x2de9_2c6f, 0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da, 0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7, 0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967, 0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc, 0x5338_0d13, 0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85, 0xa2bf_e8a1, 0xa81a_664b, 0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070, 0x19a4_c116, 0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3, 0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208, 0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7, 0xc671_78f2,
];

/// SHA-256 (FIPS 180-4), std only.
#[must_use]
pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a, 0x510e_527f, 0x9b05_688c, 0x1f83_d9ab, 0x5be0_cd19];
    let bitlen = u64::try_from(data.len()).unwrap_or(0).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, b) in chunk.chunks_exact(4).enumerate() {
            if let (Some(x), Ok(arr)) = (w.get_mut(i), <[u8; 4]>::try_from(b)) {
                *x = u32::from_be_bytes(arr);
            }
        }
        for i in 16..64 {
            let (a, b, c, d) = (w.get(i - 15).copied().unwrap_or(0), w.get(i - 2).copied().unwrap_or(0), w.get(i - 16).copied().unwrap_or(0), w.get(i - 7).copied().unwrap_or(0));
            let s0 = a.rotate_right(7) ^ a.rotate_right(18) ^ (a >> 3);
            let s1 = b.rotate_right(17) ^ b.rotate_right(19) ^ (b >> 10);
            if let Some(x) = w.get_mut(i) {
                *x = c.wrapping_add(s0).wrapping_add(d).wrapping_add(s1);
            }
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for (k, wi) in K.iter().zip(w.iter()) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(*k).wrapping_add(*wi);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
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
    let mut out = [0u8; 32];
    for (i, x) in h.iter().enumerate() {
        if let Some(dst) = out.get_mut(i * 4..i * 4 + 4) {
            dst.copy_from_slice(&x.to_be_bytes());
        }
    }
    out
}

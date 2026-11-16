//! S7 of SNIPER-IDXOPT-PLAN-v3.1 (INDEX S7 builder, main 5b507203, 15 Nov 2026): the INSTRUMENT as DATA ROWS (plan s3.4: the
//! instrument rows live in `SNIPER-INSTRUMENT-v1.tsv` beside the base RUN file, one source per row) and the LAW TABLE that refuses an inconsistent row set at
//! load, before any reading. The host takes its underlying, segment, block, hold and the cash instrument's supporting files from
//! these rows; no host path names ADANIENT or CASH as a code constant any more (plan s2: "the host hard-codes the instrument").
//!
//! What runs today: `instrument CASH` (the ADANIENT cash path of host v3, byte for byte: proof P2). `IDXOPT` / `STKOPT` rows are
//! parsed and law-checked here, then REFUSED BY NAME before any reading ([`Instrument::require_built`]): the index feature rows
//! (S4, `feat_io` index segment), the option pack (S5) and the option outcome rows (S7's 442 B / OUT-POS 792 B) are not built in
//! this binary. The pinned `feat_io` reads the CASH segment only, so a CASH row naming another segment is refused too.
//!
//! The law rows (s3.4; each a named refusal, each with a red mutant):
//! * L1 `m_ix1a`: IDXOPT with block 2 (index options fill on 1-s blocks, REQ 73b / 73c);
//! * L2 `m_ix1b`: STKOPT with block 1 (stock options stay on the 2-s law);
//! * L3 `m_ix_hold`: `hold = intraday` with a POS exit, or `hold = positional` with XC;
//! * L4 `m_ix_roll`: IDXOPT with `roll_rule = last_working_day_before`, or STKOPT with `expiry_day_at`;
//! * L5 `m_ix_rollopen`: `roll_open_rule` on a `hold = intraday` row;
//! * L6 `m_ix_itm_stk`: `contract_rule = itm_fit_one_order` on STKOPT (STOCK options use volume alone, s18a);
//! * L7 `m_ix_zgap`: `zgap_rule = fallback_zgap_w` without the operator's recorded "declined" line (O1);
//! * L8 `m_ix_cash_rows`: a CASH row set naming another segment, block or hold, or carrying an option row (one source per row).
//! * L9 `m_ix_sq` (S7 BUILDER wave 4; MAIN-RULINGS-S7-S10 item 3, D2): `sq_charge` other than `own_order` (the 15:10 square-off is the
//!   strategy's OWN order, charged as a normal executed order; never Dhan's auto square-off fee).
//!
//! S7 BUILDER wave 4 (15 Nov 2026): IDXOPT row sets are PRICED now (intraday and positional, `crate::idx`): the S4 index feature rows
//! (feat_io r3), the S5 pack (DP-NEWPX-IDXOPT under the DATA law) and the S8b pricer (opt_pos) are linked; their files are data rows
//! (`idx_series_file`, `idx_close_file`, `fno_end_file`, `halt_file`, `option_pack`, `option_store`, beside `expiry_cal`,
//! `freeze_file`, `lot_source`), every one sha-printed. The 15-value N grid, the stored N and the proxy N of an IDXOPT row set are
//! instrument rows (`n_grid`, `stored_n`, `proxy_n`): its base RUN file must not hold them (one source per row). STKOPT stays refused
//! by name (lane B's tree prices it).
//!
//! W4 REPAIR (check-wave4-S9 D-W4-2): `muhurat_file` = S1's `MUHURAT-LIST.tsv` is an IDXOPT row too (required, sha-printed): the
//! session facts of `opt_pos` hold the Muhurat sessions as data, so a carried exit rest is held through them (plan s4.4 / Q39).

use std::collections::BTreeMap;

use crate::rows::RunRows;

/// The instrument kinds of plan s3.4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Cash intraday (ADANIENT, host v1 .. v3).
    Cash,
    /// Index options (NIFTY, BANKNIFTY).
    IdxOpt,
    /// Stock options (lane B).
    StkOpt,
}

/// The hold of a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hold {
    /// Square-off the same session.
    Intraday,
    /// Held overnight until the exit (and rolled).
    Positional,
}

/// The rows an option row set carries that a CASH row set must not (one source per row, L8).
pub const OPTION_ROWS: [&str; 28] = [
    "contract_rule",
    "compare_rule",
    "strict_fit",
    "freeze_file",
    "close_rule",
    "expiry_cal",
    "lot_source",
    "headline_n",
    "roll_rule",
    "roll_ist",
    "roll_open_rule",
    "entry_cut",
    "session_end",
    "spot_fallback",
    "muhurat_signals",
    "zgap_rule",
    "n_grid",
    "stored_n",
    "proxy_n",
    "sq_charge",
    "idx_series_file",
    "idx_close_file",
    "fno_end_file",
    "halt_file",
    "option_pack",
    "option_store",
    "contract_target_paisa",
    "muhurat_file",
];

/// The rows every IDXOPT row set must carry (plan s3.4), beside the base RUN rows.
pub const IDXOPT_REQUIRED: [&str; 32] = [
    "contract_rule",
    "compare_rule",
    "strict_fit",
    "freeze_file",
    "close_rule",
    "expiry_cal",
    "lot_source",
    "headline_n",
    "exit",
    "roll_rule",
    "roll_ist",
    "entry_cut",
    "session_end",
    "spot_fallback",
    "muhurat_signals",
    "reserve_rule",
    "afr2",
    "zgap_rule",
    "acct_over_cap",
    "fill_crate_version",
    "n_grid",
    "stored_n",
    "proxy_n",
    "sq_charge",
    "idx_series_file",
    "idx_close_file",
    "fno_end_file",
    "halt_file",
    "option_pack",
    "option_store",
    "contract_target_paisa",
    "muhurat_file",
];

/// The instrument of a run, from its RUN rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instrument {
    /// The kind.
    pub kind: Kind,
    /// The underlying (`ADANIENT`, `NIFTY`, `BANKNIFTY`, ...): the symbol every reading and the words use.
    underlying: String,
    /// The segment (`CASH`, `INDEX`).
    pub segment: String,
    /// The fill block, seconds (2: `req70-2s`; 1: `req73b-1s`).
    pub block_s: u8,
    /// The hold.
    pub hold: Hold,
    /// The instrument rows as read (key -> value), for the identity lines of an option run.
    pub rows: BTreeMap<String, String>,
}

fn opt<'a>(run: &'a RunRows, k: &str) -> Option<&'a str> {
    run.get(k).ok().map(str::trim).filter(|v| !v.is_empty())
}

fn symbol_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= 32 && s.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'&' || b == b'-')
}

/// The law table on a row set given as a lookup (plan s3.4 L1-L8). `declined` answers whether the O1 record file holds the
/// operator's "declined" line.
///
/// # Errors
/// The first refused row, by its law name.
pub fn law_rows(get: &dyn Fn(&str) -> Option<String>, declined: &dyn Fn(&str) -> bool) -> Result<(Kind, Hold, u8), String> {
    let kind = match get("instrument").as_deref() {
        Some("CASH") => Kind::Cash,
        Some("IDXOPT") => Kind::IdxOpt,
        Some("STKOPT") => Kind::StkOpt,
        Some(x) => return Err(format!("instrument {x}: not an instrument of plan s3.4 (CASH | IDXOPT | STKOPT)")),
        None => return Err("RUN row instrument missing: the instrument is a data row (plan s3.4; SNIPER-RUN-IDX-v1.tsv), never a code default".to_string()),
    };
    let block: u8 = get("block").ok_or("RUN row block missing (plan s3.4)")?.parse().map_err(|_| "RUN row block is not 1 or 2".to_string())?;
    if block != 1 && block != 2 {
        return Err(format!("RUN row block {block}: only 1 (req73b-1s) or 2 (req70-2s)"));
    }
    let hold = match get("hold").as_deref() {
        Some("intraday") => Hold::Intraday,
        Some("positional") => Hold::Positional,
        Some(x) => return Err(format!("RUN row hold {x}: intraday | positional")),
        None => return Err("RUN row hold missing (plan s3.4)".to_string()),
    };
    // L1 / L2 (m-IX1a / m-IX1b)
    if kind == Kind::IdxOpt && block != 1 && !cfg!(feature = "m_ix1a") {
        return Err("L1 (m-IX1a): IDXOPT with block 2 refused: index options fill on 1-s blocks (REQ 73b / 73c)".to_string());
    }
    if kind == Kind::StkOpt && block != 2 && !cfg!(feature = "m_ix1b") {
        return Err("L2 (m-IX1b): STKOPT with block 1 refused: stock options fill on the 2-s law (REQ 70)".to_string());
    }
    // L3: the hold against the exit
    if let Some(ex) = get("exit") {
        let pos_exit = ex.starts_with("POS-");
        let xc_exit = ex.starts_with("XC-");
        let bad = (hold == Hold::Intraday && pos_exit) || (hold == Hold::Positional && xc_exit);
        if bad && !cfg!(feature = "m_ix_hold") {
            return Err(format!("L3: hold {hold:?} with exit {ex} refused (intraday takes XC-ST10-30-T300; positional takes POS-ST10-30)"));
        }
    }
    // L4: the roll rule against the instrument
    if let Some(rr) = get("roll_rule") {
        let bad = (kind == Kind::IdxOpt && rr == "last_working_day_before") || (kind == Kind::StkOpt && rr == "expiry_day_at");
        if bad && !cfg!(feature = "m_ix_roll") {
            return Err(format!("L4: {kind:?} with roll_rule {rr} refused (IDXOPT expiry_day_at; STKOPT last_working_day_before)"));
        }
    }
    // L5: no roll-open rule on an intraday row
    if hold == Hold::Intraday && get("roll_open_rule").is_some() && !cfg!(feature = "m_ix_rollopen") {
        return Err("L5: roll_open_rule on a hold = intraday row refused (an intraday trade never rolls)".to_string());
    }
    // L6: ITM-fit is an INDEX option rule
    if kind == Kind::StkOpt && get("contract_rule").as_deref() == Some("itm_fit_one_order") && !cfg!(feature = "m_ix_itm_stk") {
        return Err("L6: contract_rule itm_fit_one_order on STKOPT refused (stock options use volume alone, s18a)".to_string());
    }
    // L7: the ZGAP fallback needs the operator's recorded "declined" line (O1)
    if get("zgap_rule").as_deref() == Some("fallback_zgap_w") && !cfg!(feature = "m_ix_zgap") {
        let ok = get("o1_declined_record").is_some_and(|p| declined(&p));
        if !ok {
            return Err("L7: zgap_rule fallback_zgap_w refused without the operator's recorded \"declined\" line (O1; row o1_declined_record)".to_string());
        }
    }
    // L8: a CASH row set is the cash path only
    if kind == Kind::Cash && !cfg!(feature = "m_ix_cash_rows") {
        let seg = get("segment").unwrap_or_default();
        if seg != "CASH" || block != 2 || hold != Hold::Intraday {
            return Err(format!("L8: CASH with segment {seg} / block {block} / hold {hold:?} refused (the cash path is CASH, req70-2s, intraday)"));
        }
        if let Some(r) = OPTION_ROWS.iter().find(|r| get(r).is_some()) {
            return Err(format!("L8: CASH row set carrying the option row {r} refused (one source per row)"));
        }
    }
    // L9 (D2, MAIN-RULINGS-S7-S10 item 3): the 15:10 square-off is the strategy's OWN order
    if let Some(sq) = get("sq_charge")
        && sq != "own_order"
        && !cfg!(feature = "m_ix_sq")
    {
        return Err(format!("L9: sq_charge {sq} refused: the 15:10 square-off is the strategy's OWN order, charged as a normal executed order with the dated rates, never Dhan's auto square-off fee (MAIN-RULINGS-S7-S10 item 3, D2)"));
    }
    if kind == Kind::IdxOpt {
        if get("segment").as_deref() != Some("INDEX") {
            return Err("IDXOPT needs segment INDEX (plan s3.4)".to_string());
        }
        if let Some(r) = IDXOPT_REQUIRED.iter().find(|r| get(r).is_none()) {
            return Err(format!("IDXOPT row {r} missing (plan s3.4)"));
        }
        if hold == Hold::Positional && get("roll_open_rule").is_none() {
            return Err("IDXOPT positional row roll_open_rule missing (plan s3.4: sequential headline, same_decision beside)".to_string());
        }
        let hn = get("headline_n").unwrap_or_default();
        for list in ["n_grid", "stored_n"] {
            if !get(list).unwrap_or_default().split(',').any(|x| x.trim() == hn) {
                return Err(format!("IDXOPT headline_n {hn} not in {list} (A1: N 4 stored and ranked exactly)"));
            }
        }
    }
    Ok((kind, hold, block))
}

fn declined_line(path: &str) -> bool {
    std::fs::read_to_string(path).is_ok_and(|t| t.lines().any(|l| l.contains("declined") && l.contains("O1")))
}

impl Instrument {
    /// Reads the instrument rows (`SNIPER-INSTRUMENT-v1.tsv`, key / value / source like the RUN rows) and applies the law table (L1-L8).
    ///
    /// # Errors
    /// A missing or refused row.
    pub fn from_rows(run: &RunRows) -> Result<Self, String> {
        let get = |k: &str| opt(run, k).map(str::to_string);
        let (kind, hold, block_s) = law_rows(&get, &declined_line)?;
        let underlying = get("underlying").ok_or("RUN row underlying missing: the instrument is a data row (plan s3.4)")?;
        if !symbol_ok(&underlying) {
            return Err(format!("RUN row underlying {underlying}: not a symbol"));
        }
        let segment = get("segment").ok_or("RUN row segment missing (plan s3.4)")?;
        let mut rows = BTreeMap::new();
        for k in ["instrument", "underlying", "segment", "block", "hold", "bhav_file", "dexp_census_file", "exit", "o1_declined_record"].iter().chain(OPTION_ROWS.iter()).chain(IDXOPT_REQUIRED.iter()) {
            if let Some(v) = get(k) {
                rows.insert((*k).to_string(), v);
            }
        }
        Ok(Self { kind, underlying, segment, block_s, hold, rows })
    }

    /// The underlying every reading and the words name (the RUN row `underlying`).
    #[must_use]
    pub fn underlying(&self) -> &str {
        if cfg!(feature = "m_s7_sym") {
            // MUTANT m_s7_sym: the host ignores the data row and names the instrument by a code constant again.
            return "ADANIENT";
        }
        &self.underlying
    }

    /// The cash tick member of the GDFL tick store (`<UNDERLYING>.NSE.csv`, cm/STOCKS; the CASH segment only).
    #[must_use]
    pub fn cash_tick_member(&self) -> String {
        format!("{}.NSE.csv", self.underlying())
    }

    /// The cash path's supporting files must be the ones the pinned `feat_io` / `ctxdials` read (`bhav_file`,
    /// `dexp_census_file`): checked rows, refused when they differ (the pin reads those paths until S4 re-points them).
    ///
    /// # Errors
    /// A missing row or a row naming another file.
    pub fn check_cash_files(&self, bhav: &str, census: &str) -> Result<(), String> {
        for (k, want) in [("bhav_file", bhav), ("dexp_census_file", census)] {
            match self.rows.get(k) {
                Some(v) if v == want => {}
                Some(v) => return Err(format!("RUN row {k} {v}: the pinned reader reads {want} (refused until S4 re-points it)")),
                None => return Err(format!("RUN row {k} missing (the cash instrument's supporting file is a data row)")),
            }
        }
        Ok(())
    }

    /// The modes this binary prices: CASH and (S7 BUILDER wave 4) IDXOPT intraday and positional. STKOPT passes the law table and is
    /// then refused by name before any reading (lane B's tree prices it). Mutant `m_s7w4_refuse_idx` refuses IDXOPT again (the wave-3
    /// binary): `s7w4_index_modes_are_priced` must turn red.
    ///
    /// # Errors
    /// Every STKOPT row set.
    pub fn require_built(&self) -> Result<(), String> {
        match self.kind {
            Kind::Cash => Ok(()),
            Kind::IdxOpt if !cfg!(feature = "m_s7w4_refuse_idx") => Ok(()),
            k => Err(format!(
                "{k:?} {} ({} hold, block {} s): law table PASSED; refused by name before any reading: the index feature rows (S4: feat_io index segment, X-PEER, D-EXP / D-DTE from DP-IDXCAL, G-52H / L from the close file, V-RVOL refusal, Muhurat exclusion), the option pack (S5: DP-IDXITM / DP-IDXNEED / DP-NEWPX-IDXOPT) and the option outcome rows (442 B / OUT-POS 792 B, the 15-value N grid) are not built in this binary (plan s8 S4-S7); this binary prices CASH only",
                self.underlying(),
                if self.hold == Hold::Intraday { "intraday" } else { "positional" },
                self.block_s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
    }

    fn law(m: &BTreeMap<String, String>) -> Result<(Kind, Hold, u8), String> {
        law_rows(&|k: &str| m.get(k).cloned(), &|p: &str| p == "/declined")
    }

    fn idx(hold: &str, exit: &str) -> BTreeMap<String, String> {
        let mut m = rows(&[
            ("instrument", "IDXOPT"),
            ("underlying", "NIFTY"),
            ("segment", "INDEX"),
            ("block", "1"),
            ("hold", hold),
            ("exit", exit),
            ("contract_rule", "itm_fit_one_order"),
            ("compare_rule", "busiest"),
            ("strict_fit", "fit_strict"),
            ("freeze_file", "F"),
            ("close_rule", "split_at_max_order"),
            ("expiry_cal", "C"),
            ("lot_source", "listed"),
            ("headline_n", "4"),
            ("n_grid", "1,2,3,4,5,10,15,20,25,30,40,50,60,80,100"),
            ("stored_n", "1,4,5,20,40,100"),
            ("roll_rule", "expiry_day_at"),
            ("roll_ist", "15:00:00"),
            ("entry_cut", "index_close"),
            ("session_end", "fno_dated"),
            ("spot_fallback", "zerodha_close"),
            ("muhurat_signals", "excluded"),
            ("reserve_rule", "pay_from_free_cash"),
            ("afr2", "on"),
            ("zgap_rule", "repaired"),
            ("acct_over_cap", "cash_short"),
            ("fill_crate_version", "v2.1"),
            ("proxy_n", "4"),
            ("sq_charge", "own_order"),
            ("idx_series_file", "S"),
            ("idx_close_file", "C"),
            ("fno_end_file", "E"),
            ("halt_file", "H"),
            ("option_pack", "P"),
            ("option_store", "R"),
            ("contract_target_paisa", "50000000"),
            ("muhurat_file", "M"),
        ]);
        if hold == "positional" {
            m.insert("roll_open_rule".into(), "sequential".into());
        }
        m
    }

    fn cash() -> BTreeMap<String, String> {
        rows(&[("instrument", "CASH"), ("underlying", "ADANIENT"), ("segment", "CASH"), ("block", "2"), ("hold", "intraday")])
    }

    /// The plan's own rows pass: CASH (ADANIENT), IDXOPT intraday XC, IDXOPT positional POS with the sequential roll.
    #[test]
    fn s7_law_accepts_the_plan_rows() {
        assert_eq!(law(&cash()), Ok((Kind::Cash, Hold::Intraday, 2)));
        assert_eq!(law(&idx("intraday", "XC-ST10-30-T300")).map(|x| x.0), Ok(Kind::IdxOpt));
        assert_eq!(law(&idx("positional", "POS-ST10-30")).map(|x| x.1), Ok(Hold::Positional));
    }

    /// L1 (m-IX1a) and L2 (m-IX1b).
    #[test]
    fn s7_law_l1_l2_block() {
        let mut m = idx("intraday", "XC-ST10-30-T300");
        m.insert("block".into(), "2".into());
        assert!(law(&m).is_err_and(|e| e.starts_with("L1")), "IDXOPT with block 2 must be refused");
        let mut s = idx("intraday", "XC-ST10-30-T300");
        s.insert("instrument".into(), "STKOPT".into());
        s.insert("contract_rule".into(), "busiest".into());
        assert!(law(&s).is_err_and(|e| e.starts_with("L2")), "STKOPT with block 1 must be refused");
    }

    /// L3: hold against exit, both ways.
    #[test]
    fn s7_law_l3_hold_exit() {
        assert!(law(&idx("intraday", "POS-ST10-30")).is_err_and(|e| e.starts_with("L3")));
        assert!(law(&idx("positional", "XC-ST10-30-T300")).is_err_and(|e| e.starts_with("L3")));
    }

    /// L4: roll rule against instrument.
    #[test]
    fn s7_law_l4_roll_rule() {
        let mut m = idx("positional", "POS-ST10-30");
        m.insert("roll_rule".into(), "last_working_day_before".into());
        assert!(law(&m).is_err_and(|e| e.starts_with("L4")));
        let mut s = idx("positional", "POS-ST10-30");
        s.insert("instrument".into(), "STKOPT".into());
        s.insert("block".into(), "2".into());
        s.insert("contract_rule".into(), "busiest".into());
        assert!(law(&s).is_err_and(|e| e.starts_with("L4")), "STKOPT with expiry_day_at must be refused");
    }

    /// L5: roll_open_rule on an intraday row.
    #[test]
    fn s7_law_l5_roll_open_intraday() {
        let mut m = idx("intraday", "XC-ST10-30-T300");
        m.insert("roll_open_rule".into(), "sequential".into());
        assert!(law(&m).is_err_and(|e| e.starts_with("L5")));
    }

    /// L6: ITM-fit on stock options.
    #[test]
    fn s7_law_l6_itm_on_stkopt() {
        let mut s = idx("positional", "POS-ST10-30");
        s.insert("instrument".into(), "STKOPT".into());
        s.insert("block".into(), "2".into());
        s.insert("roll_rule".into(), "last_working_day_before".into());
        assert!(law(&s).is_err_and(|e| e.starts_with("L6")));
    }

    /// L7: the ZGAP fallback without (and with) the operator's recorded line.
    #[test]
    fn s7_law_l7_zgap_needs_o1_record() {
        let mut m = idx("intraday", "XC-ST10-30-T300");
        m.insert("zgap_rule".into(), "fallback_zgap_w".into());
        assert!(law(&m).is_err_and(|e| e.starts_with("L7")));
        m.insert("o1_declined_record".into(), "/declined".into());
        assert!(law(&m).is_ok(), "with the recorded line the fallback is allowed");
    }

    /// L8: a CASH row set is CASH / req70-2s / intraday and carries no option row.
    #[test]
    fn s7_law_l8_cash_rows() {
        let mut m = cash();
        m.insert("segment".into(), "INDEX".into());
        assert!(law(&m).is_err_and(|e| e.starts_with("L8")));
        let mut b = cash();
        b.insert("contract_rule".into(), "itm_fit_one_order".into());
        assert!(law(&b).is_err_and(|e| e.starts_with("L8")));
        let mut c = cash();
        c.remove("instrument");
        assert!(law(&c).is_err_and(|e| e.contains("never a code default")), "no instrument row: refused, never a default");
    }

    /// The underlying flows from the data row to every reading (m_s7_sym: a code constant again).
    #[test]
    fn s7_underlying_is_the_data_row() {
        let t = "key\tvalue\tsource\ninstrument\tCASH\tt\nunderlying\tSBIN\tt\nsegment\tCASH\tt\nblock\t2\tt\nhold\tintraday\tt\n";
        let run = RunRows::parse(t).unwrap();
        let i = Instrument::from_rows(&run).unwrap();
        assert_eq!(i.underlying(), "SBIN");
        assert_eq!(i.cash_tick_member(), "SBIN.NSE.csv");
        assert!(i.require_built().is_ok());
    }

    /// An IDXOPT row set passes the law and is then refused by name before any reading.
    #[test]
    fn s7_index_mode_refused_by_name() {
        let m = idx("positional", "POS-ST10-30");
        let mut t = String::from("key\tvalue\tsource\n");
        for (k, v) in &m {
            t.push_str(&format!("{k}\t{v}\ts\n"));
        }
        let i = Instrument::from_rows(&RunRows::parse(&t).unwrap()).unwrap();
        assert!(i.require_built().is_ok(), "S7 wave 4: IDXOPT is priced (intraday and positional)");
        let mut s = idx("positional", "POS-ST10-30");
        s.insert("instrument".into(), "STKOPT".into());
        s.insert("block".into(), "2".into());
        s.insert("contract_rule".into(), "busiest".into());
        s.insert("roll_rule".into(), "last_working_day_before".into());
        let mut t2 = String::from("key\tvalue\tsource\n");
        for (k, v) in &s {
            t2.push_str(&format!("{k}\t{v}\ts\n"));
        }
        let j = Instrument::from_rows(&RunRows::parse(&t2).unwrap()).unwrap();
        assert!(j.require_built().is_err_and(|e| e.contains("law table PASSED")), "STKOPT stays refused by name");
    }

    /// S7 wave 4: both index holds are priced (mutant m_s7w4_refuse_idx: refused again).
    #[test]
    fn s7w4_index_modes_are_priced() {
        for hold in [("intraday", "XC-ST10-30-T300"), ("positional", "POS-XS10-30")] {
            let m = idx(hold.0, hold.1);
            let mut t = String::from("key\tvalue\tsource\n");
            for (k, v) in &m {
                t.push_str(&format!("{k}\t{v}\ts\n"));
            }
            let i = Instrument::from_rows(&RunRows::parse(&t).unwrap()).unwrap();
            assert!(i.require_built().is_ok(), "{} must be priced", hold.0);
        }
    }

    /// L9 (D2): sq_charge own_order passes; broker is refused (m_ix_sq accepts it).
    #[test]
    fn s7w4_law_l9_square_off_is_the_strategys_own_order() {
        let mut m = idx("intraday", "XC-ST10-30-T300");
        assert!(law(&m).is_ok());
        m.insert("sq_charge".into(), "broker".into());
        assert!(law(&m).is_err_and(|e| e.starts_with("L9")), "the broker reading of the square-off is refused");
    }
}

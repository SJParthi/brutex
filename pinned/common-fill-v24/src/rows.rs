//! The DB and audit rows of design section 12 (the crate BUILDS the rows; each host's DB module writes them; append-only). The field
//! orders below are FROZEN, schema [`SCHEMA`] (`cf-rows-v2`): a row is built field by field by NAME ([`Rec::set`]) and printed in the
//! frozen order ([`Rec::tsv`], refused while a field is unset). No I/O here: a host writes through its own [`Sink`] (temp file then rename,
//! refused if the target exists, partitions keyed (run_id, session_date); a law, RULES or crate change is a NEW run_id).
//!
//! Scope (critic B F11): T2-T5 for SHOWN results and PICKS only; the search writes T7 per (combo, level).

use std::fmt::Display;

use crate::std70::{BlockFill, LevelFill, Order};
use crate::{FillErr, Paisa, Side, US_S, Units, Us};

/// The schema version of every table here.
pub const SCHEMA: &str = "cf-rows-v2";

/// One table: its name and its FROZEN field order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Table {
    /// The table's name (its file stem).
    pub name: &'static str,
    /// The frozen field order.
    pub fields: &'static [&'static str],
}

/// T1 `fill_runs`: one row per run.
pub const T1: Table = Table {
    name: "fill_runs",
    fields: &[
        "run_id", "schema", "tool", "lane", "round", "law_headline", "law_compiled", "dial_bp", "timing_headline", "comparisons", "crate_version", "common_fill_sha", "vectors_sha", "matrix_sha", "live_timing_sha", "rules_sha", "session_table_sha", "store_manifest_sha", "binary_sha", "started_ist", "finished_ist", "load_at_start", "filled_secs", "one_price_secs", "holes", "hole_span_orders", "inblock_stop_trades", "supersedes_run_id",
    ],
};

/// T2 `fill_blocks`: one row per block that filled or was skipped by a limit, cap or pay while the level had a rest (empty blocks are only
/// counted). `px_pre` is the 1-second price of a block clipped by its stop (S9); for other blocks it equals the block price.
pub const T2: Table = Table {
    name: "fill_blocks",
    fields: &[
        "run_id", "law", "level_bp", "session_date", "symbol", "contract_token", "expiry", "strike", "right", "unit", "trade_id", "order_id", "order_kind", "side", "anchor_ist", "k", "block_at_ist", "block_to_ist", "clipped", "n_traded", "one_price_secs", "block_high", "block_low", "block_vol", "px_pre", "cap_units", "ledger_charge", "take_units", "price_paisa", "cum_units", "rest_after", "why", "known_at_ist",
    ],
};

/// T3 `fill_orders`: one row per order per level. The two `secs_*` fields are REQ 57's fill timing under blocks.
pub const T3: Table = Table {
    name: "fill_orders",
    fields: &[
        "run_id", "law", "level_bp", "trade_id", "order_id", "kind", "side", "anchor_ist", "stop_ist", "limit_paisa", "cap_paisa", "qty_ordered", "units_filled", "value_paisa", "avg_px_4dp", "blocks_filled", "blocks_no_trade", "blocks_past_limit", "blocks_past_cap", "blocks_clipped", "zero_take_blocks", "skips", "first_block", "first_px", "last_block", "last_px", "e0_ist", "e1_ist", "secs_anchor_to_first_fill_block", "secs_first_to_last_block", "known_at_ist", "rest_at_stop", "cancel_reason", "unpaid_units", "money_in_paisa", "slot_paisa", "reserve_paisa", "excess_paisa", "free_cash_at_known_paisa", "paid_paisa", "cut", "hole_span", "unclosable_units", "unclosable_px", "unclosable_from_to", "settlement", "steps_blocks", "steps_rows",
    ],
};

/// T4 `fill_trades`: one row per trade per level (`up_at` / `down_at`: the design's `up_at[x...]` / `down_at[x...]`, one `x:instant`
/// pair per dial value x, comma separated). `fills_before_anchor` and `fills_inside_signal_candle` must be 0 (REQ 56).
pub const T4: Table = Table {
    name: "fill_trades",
    fields: &[
        "run_id", "level_bp", "trade_id", "symbol", "instrument_kind", "underlying", "signal_source", "entry_order_id", "exit_order_ids", "sq_order_id", "e0_ist", "e1_ist", "x_end_ist", "first_fill_px", "avg_entry_px", "avg_exit_px", "gross_paisa", "charges_paisa", "net_paisa", "first_side", "up_at", "down_at", "both_secs", "best_px", "best_at_ist", "worst_px", "worst_at_ist", "run_up_paisa", "run_down_paisa", "first_side_from_e1", "inblock_stop", "winner_went_down_first", "loser_went_up_first", "fills_before_anchor", "fills_inside_signal_candle", "incomplete_at_bell", "whatif_rule_ids", "whatif_trigger_ist", "whatif_net_paisa", "whatif_saved_paisa", "whatif_hindsight", "old_rule_net_paisa", "cap_net_paisa", "precise_net_paisa_or_na", "block_touch_net_paisa_or_na", "grid_cells_ref",
    ],
};

/// T5 `law_compare`, per trade and level: the design's eight fields after the partition keys (run_id, level_bp, trade_id); `verdict` is
/// the design's `kinder|harsher|equal` column (one of those three words).
pub const T5: Table = Table {
    name: "law_compare",
    fields: &["run_id", "level_bp", "trade_id", "std70_net", "old_rule_net", "diff", "verdict", "units_worse_than_old_second_price", "cap_diff", "precise_diff_or_na", "block_touch_diff_or_na"],
};

/// T6 `audit`: an append-only log, never edited; each run_id its OWN audit file with ONE writer; each row carries the sha256 of the row
/// before it in that file (the host computes it: no hashing in this crate). `action` is one of [`AUDIT_ACTIONS`].
pub const T6: Table = Table {
    name: "audit",
    fields: &["seq", "ist", "run_id", "action", "path", "sha256", "rows", "bytes", "writer", "binary_sha", "prev_row_sha256", "note"],
};

/// The audit actions (T6).
pub const AUDIT_ACTIONS: [&str; 6] = ["OPEN_RUN", "WRITE_PARTITION", "VERIFY", "CHECK", "SUPERSEDE", "CLOSE_RUN"];

/// T7 `fill_agg`: the search's only rows, one per (combo, level).
pub const T7: Table = Table {
    name: "fill_agg",
    fields: &["run_id", "combo_id", "level_bp", "trades", "units", "value_paisa", "gross_paisa", "charges_paisa", "net_paisa", "unpaid_units", "excess_n", "excess_max_paisa", "clipped_blocks", "zero_take_blocks", "holes_spanned"],
};

/// Every table, in design order.
pub const TABLES: [Table; 7] = [T1, T2, T3, T4, T5, T6, T7];

/// A host's writer (temp file then rename; refused if the target exists): the crate never writes.
pub trait Sink {
    /// Write one line (a header or a row) of `table`.
    ///
    /// # Errors
    /// The host's refusal.
    fn write(&mut self, table: &Table, line: &str) -> Result<(), FillErr>;
}

/// The header line of `t` (tab separated, its frozen order).
#[must_use]
pub fn header(t: &Table) -> String {
    t.fields.join("\t")
}

/// One row of one table, filled field by field by name and printed in the frozen order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rec {
    table: Table,
    vals: Vec<Option<String>>,
}

impl Rec {
    /// An empty row of `table`.
    #[must_use]
    pub fn new(table: Table) -> Self {
        Self { table, vals: vec![None; table.fields.len()] }
    }

    /// Sets field `name` (no tab or newline in the value).
    ///
    /// # Errors
    /// [`FillErr::RulesInvalid`] for a name the table does not have, a field set twice, or a value holding a tab or a newline.
    pub fn set(&mut self, name: &str, v: impl Display) -> Result<&mut Self, FillErr> {
        let i = self.table.fields.iter().position(|&f| f == name).ok_or(FillErr::RulesInvalid)?;
        let s = v.to_string();
        if s.contains('\t') || s.contains('\n') {
            return Err(FillErr::RulesInvalid);
        }
        let slot = self.vals.get_mut(i).ok_or(FillErr::RulesInvalid)?;
        if slot.is_some() {
            return Err(FillErr::RulesInvalid);
        }
        *slot = Some(s);
        Ok(self)
    }

    /// The row's line in the frozen order.
    ///
    /// # Errors
    /// [`FillErr::RulesInvalid`] while a field is unset (a row is never written with a hole).
    pub fn tsv(&self) -> Result<String, FillErr> {
        let mut out = String::new();
        for (k, v) in self.vals.iter().enumerate() {
            if k > 0 {
                out.push('\t');
            }
            out.push_str(v.as_deref().ok_or(FillErr::RulesInvalid)?);
        }
        Ok(out)
    }

    /// The names of the fields still unset.
    #[must_use]
    pub fn missing(&self) -> Vec<&'static str> {
        self.table.fields.iter().zip(&self.vals).filter(|(_, v)| v.is_none()).map(|(f, _)| *f).collect()
    }
}

/// Days from the civil date (proleptic Gregorian) to 1970-01-01 and back (Howard Hinnant's algorithm, integer only).
const fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// An instant as IST (UTC+05:30, no daylight saving): `YYYY-MM-DD HH:MM:SS`, with `.ffffff` when not on a whole second; `-` for 0.
#[must_use]
pub fn ist(us: Us) -> String {
    if us == 0 {
        return "-".to_owned();
    }
    let t = us + 19_800 * US_S;
    let secs = t.div_euclid(US_S);
    let frac = t.rem_euclid(US_S);
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let sod = secs.rem_euclid(86_400);
    let base = format!("{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}", sod / 3_600, sod / 60 % 60, sod % 60);
    if frac == 0 { base } else { format!("{base}.{frac:06}") }
}

/// The session date (IST) of an instant: `YYYY-MM-DD`.
#[must_use]
pub fn ist_date(us: Us) -> String {
    let t = us + 19_800 * US_S;
    let (y, m, d) = civil_from_days(t.div_euclid(US_S).div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// The average price in paisa, value / units, to 4 decimals (half away from zero); `-` for no units.
#[must_use]
pub fn avg_4dp(value: Paisa, units: Units) -> String {
    if units == 0 {
        return "-".to_owned();
    }
    let num = i128::from(value) * 10_000;
    let den = i128::from(units);
    let q = num / den;
    let r = num % den;
    let q = if 2 * r.abs() >= den { q + num.signum() } else { q };
    format!("{}.{:04}", q / 10_000, (q % 10_000).abs())
}

/// The side's code.
#[must_use]
pub const fn side_code(s: Side) -> &'static str {
    match s {
        Side::Buy => "BUY",
        Side::Sell => "SELL",
    }
}

/// The context a host gives the T2 / T3 builders (everything the crate's own types do not carry).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ctx {
    /// The run's id.
    pub run_id: String,
    /// The law's name (`req70-2s`, `req70-2s-cap`, ...).
    pub law: String,
    /// The symbol (or the underlying of an option).
    pub symbol: String,
    /// The contract token (`-` for cash).
    pub contract_token: String,
    /// The expiry (`-` for cash).
    pub expiry: String,
    /// The strike (`-` for cash).
    pub strike: String,
    /// CE / PE (`-` for cash).
    pub right: String,
    /// The trade's id.
    pub trade_id: String,
    /// The order's id.
    pub order_id: String,
    /// The order's kind as the host names it.
    pub order_kind: String,
}

/// T2: one block row (`cap_units` = the level's share of the block's volume in whole units before any ledger charge; `ledger_charge` the
/// charge read (0 without a ledger); `rest_after` the level's rest after this row).
///
/// # Errors
/// As [`Rec::set`].
#[allow(clippy::too_many_arguments, reason = "one frozen row")]
pub fn block_row(c: &Ctx, level_bp: u16, o: &Order, bf: &BlockFill, cap_units: Units, ledger_charge: Units, rest_after: Units) -> Result<Rec, FillErr> {
    let mut r = Rec::new(T2);
    let b = &bf.block;
    r.set("run_id", &c.run_id)?.set("law", &c.law)?.set("level_bp", level_bp)?.set("session_date", ist_date(o.anchor))?;
    r.set("symbol", &c.symbol)?.set("contract_token", &c.contract_token)?.set("expiry", &c.expiry)?.set("strike", &c.strike)?.set("right", &c.right)?;
    r.set("unit", o.unit.max(1))?.set("trade_id", &c.trade_id)?.set("order_id", &c.order_id)?.set("order_kind", &c.order_kind)?;
    r.set("side", side_code(o.side))?.set("anchor_ist", ist(o.anchor))?.set("k", bf.k)?.set("block_at_ist", ist(b.at))?.set("block_to_ist", ist(b.to))?;
    r.set("clipped", u8::from(b.clipped))?.set("n_traded", b.n)?.set("one_price_secs", b.one_price)?.set("block_high", b.high)?.set("block_low", b.low)?;
    r.set("block_vol", b.vol)?.set("px_pre", if o.side == Side::Buy { b.high_pre } else { b.low_pre })?.set("cap_units", cap_units)?;
    r.set("ledger_charge", ledger_charge)?.set("take_units", bf.units)?.set("price_paisa", bf.px)?.set("cum_units", bf.cum)?.set("rest_after", rest_after)?;
    r.set("why", bf.why.code())?.set("known_at_ist", ist(if bf.units > 0 { b.known } else { 0 }))?;
    Ok(r)
}

/// T3: the crate's part of one order row (the order and its level summary); the host sets the money and trade fields it owns
/// (`cancel_reason`, `money_in_paisa`, `slot_paisa`, `reserve_paisa`, `excess_paisa`, `free_cash_at_known_paisa`, `paid_paisa`, `cut`,
/// `hole_span`, `unclosable_units`, `unclosable_px`, `unclosable_from_to`, `settlement`) before [`Rec::tsv`].
///
/// # Errors
/// As [`Rec::set`].
#[allow(clippy::too_many_arguments, reason = "one frozen row")]
pub fn order_row(c: &Ctx, level_bp: u16, o: &Order, cap: Option<Paisa>, qty: Units, f: &LevelFill) -> Result<Rec, FillErr> {
    let mut r = Rec::new(T3);
    r.set("run_id", &c.run_id)?.set("law", &c.law)?.set("level_bp", level_bp)?.set("trade_id", &c.trade_id)?.set("order_id", &c.order_id)?;
    r.set("kind", &c.order_kind)?.set("side", side_code(o.side))?.set("anchor_ist", ist(o.anchor))?.set("stop_ist", ist(o.stop))?;
    r.set("limit_paisa", o.limit.map_or_else(|| "-".to_owned(), |x| x.to_string()))?.set("cap_paisa", cap.map_or_else(|| "-".to_owned(), |x| x.to_string()))?;
    r.set("qty_ordered", qty)?.set("units_filled", f.units)?.set("value_paisa", f.value)?.set("avg_px_4dp", avg_4dp(f.value, f.units))?;
    r.set("blocks_filled", f.blocks_filled)?.set("blocks_no_trade", f.no_trade)?.set("blocks_past_limit", f.past_limit)?.set("blocks_past_cap", f.past_cap)?;
    r.set("blocks_clipped", f.clipped)?.set("zero_take_blocks", f.zero_take_blocks)?.set("skips", f.steps.skips)?;
    let blk = |b: Option<BlockFill>| b.map_or_else(|| "-".to_owned(), |x| format!("{}-{}", ist(x.block.at), ist(x.block.to)));
    r.set("first_block", blk(f.first))?.set("first_px", f.first.map_or_else(|| "-".to_owned(), |x| x.px.to_string()))?;
    r.set("last_block", blk(f.last))?.set("last_px", f.last.map_or_else(|| "-".to_owned(), |x| x.px.to_string()))?;
    r.set("e0_ist", ist(f.e0))?.set("e1_ist", ist(f.e1))?;
    let secs = |a: Us, b: Us| if a == 0 || b == 0 { "-".to_owned() } else { ((b - a) / US_S).to_string() };
    r.set("secs_anchor_to_first_fill_block", secs(o.anchor, f.e0))?.set("secs_first_to_last_block", secs(f.e0, f.last.map_or(0, |x| x.block.at)))?;
    r.set("known_at_ist", ist(f.known_at))?.set("rest_at_stop", f.rest_at_stop)?.set("unpaid_units", f.unpaid)?;
    r.set("steps_blocks", f.steps.blocks)?.set("steps_rows", f.steps.rows_read)?;
    Ok(r)
}

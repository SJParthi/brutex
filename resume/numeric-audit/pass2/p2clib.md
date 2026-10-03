# SLICE p2clib: cli lib.rs 20359..end + candidate_trades (numeric pass 2)

Commit 5140aca (origin/final/all-fixes-zero). Read only.

## Verdict

**NO NEW FINDINGS.**

- **lib.rs 20359..27943 has no production code.** The last production item, the result-set commit closure, ends at line 20358. From there on the file is three `#[cfg(test)]` modules: `mod tests` (20360/20368-27110), `mod horizon_tests` (27111/27120-27277) and `mod derived_floor_tests` (27279/27288-end). I checked this with `grep -n '^#\[cfg(test)\]\|^mod '`. Under the slice rules there was nothing in that range to audit.
- **candidate_trades.rs (1..1402) and candidate_trades/codec.rs are persistence and verification code.** They do not compute prices. They carry a cell and its trade rows that `runner::grid::materialize_{,expression_}cell_over` already produced, and the timing of those rows belongs to that runner code, not to this slice. The only arithmetic here is extent and budget sizing, the slot and key mapping, and a re-fold of the trade rows that reconciles them against the cell.

## Checked and clean

- **`Financial::add`/`matches` (candidate_trades.rs:1130-1167) against `runner::grid::reconcile_rows` (grid.rs:3010-3035) and `accrue_risk` (grid.rs:4656-4670).** Both start from the same zeros: `worst_trade` starts at 0 ("Zero when nothing lost", grid.rs:374), and `peak` starts at 0, so drawdown is measured from the zero-equity start. Both use the same `saturating_add`/`saturating_sub` ordering. The reader therefore reproduces the cell's `trades`, `pessimistic`, `optimistic`, `worst_trade` and `max_drawdown` exactly, and an all-winner cell still reconciles. Any saturation of the paisa total happens identically on both sides. The writer is in runner, outside this slice.
- **Casts.** `tiers.len() as u64`, `bytes as u64`, `extent as u64`, `payload.len() as u64`, `row.{signal,entry,exit}_bar as u64`, `(slot/2) as u64 + 1` and `value.top as u64` all widen `usize` to `u64`. Every narrowing goes through `try_from`: `seq` to `u32`, `horizon` to `u32`, decoded sizes to `usize`, streaks to `u32`.
- **`Encoder::optional` uses `u64::MAX` as its `None` sentinel.** `Some(usize::MAX)` would collide with it, but these values are rung indices bounded by the ladder length, so the collision is unreachable.
- **No division by zero is reachable.** The only divisions are by constants: `raw.len()/40`, `/32`, `/8` and `slot/2`.
- **Extent arithmetic is all checked.** `Key::slot` (rank-1)*2+dir, the trade extent and offset (`checked_mul(136)+48`), the tier acknowledgement budget and `count*2` all use checked operations and refuse on overflow. `require_start`'s budget `120 + ENCODED_LEN` is exact: 32+8+32 bytes of payload, plus the expression, plus 16 header bytes and 32 seal bytes. The minimum bounds in `decode_summary` (40 bytes per tier, 32 per candidate digest) are exact.
- **Codec completeness.** `decode_cell`, `decode_rules`, `decode_tier` and `decode_candidate` are full struct literals with no `..Default`. A field added to `grid::Cell` or `crate::Rules` without a matching codec change fails to compile, so no statistic can be dropped silently. `i64` values are stored as raw little-endian bytes with no rounding, which satisfies CLAUDE.md §7.
- **Look-ahead.** Nothing here moves a fill. `checked_trade` (1168-1186) enforces `entry_bar <= exit_bar`, `entry_micros <= exit_micros` and `worst <= best`. It does not re-check `signal_bar < entry_bar`. The writer checks that (grid.rs:3501 `row.entry_bar != candidate.entry`), and the rows are bound by the BLAKE3 trade digest chained into the sealed catalog. This is a missing reader-side cross-check, not a reachable look-ahead, so it is not reported.
- **Cost per operation.** `record` costs O(cells) for the `shown_cell` re-selection plus Θ(rows) and O(trades × holding) for the replay, with `SliceFacts` built once per capture through `OnceLock`. A cold catalog read is O(C) and a cold `TradeReader::open` is O(trades). Warm pages are O(page) plus `fstat`. All of these match the module doc (lines 7-27, D-0991, D-1184, CUH-06/07/08) and `docs/06-limits.md`. No candidate step scans the history.
- **`candidates_page` byte budget (902-914).** It takes the file size from metadata before the sealed read and subtracts with `checked_sub`, so running out of budget is a loud refusal.

No probe was needed: no candidate defect survived reading.

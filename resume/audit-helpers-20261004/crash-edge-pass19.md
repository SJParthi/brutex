# Crash/edge pass 19: labels that stop a failure passing as a success (ce19)

Checkout: /home/claude/wt/zero3 at 1f4de71 (read only). No cargo or node was run. Both findings are low and are argued from quoted source.

## Verdict

2 new findings (0 high, 0 medium, 2 low): CE-93 and CE-94.
Verification: 2 earlier rows in this theme were re-checked. Both are NOT FIXED.
Known IDs not re-reported: conc13-6, conc13-7, P8-01, p13num-3, p16num-3.

## Output path x label table

Key: Y = label present and correct. N = label missing (finding). n/a = the label cannot apply on this path. "-" = the path has no such state.

| # | Output path | (a) GENERATED vs STORED banner | (b) equity gross label | (c) UNVERIFIED cost | (d) in-sample / multiplicity | (e) halted / partial / budget | (f) nothing measured vs zero |
|---|---|---|---|---|---|---|---|
| 1 | cli `sweep` / `auto` / `audit` stdout (generated bars) | Y `PROVENANCE` lib.rs:2770, :3927, :16553 | n/a (no instrument) | n/a | `audit` validates (:16566) | Y in the render; exit code is conc13-6 / P8-01 | Y |
| 2 | cli stored reports (`sweep-stored`, `range-rung`, `range-all`, `screen`, `pool`) | Y `STORED_PROVENANCE` / `stored_provenance_of` lib.rs:2805 | Y `equity_note` in the banner; pool `EQUITY_TOTALS_GROSS` pool.rs:666 | n/a: no equity charge is applied; an UNVERIFIED dated row refuses (costs dated.rs:181) | Y `IN_SAMPLE_WARNING` lib.rs:15890 (range table); `UNVALIDATED` :11764/:11819; pool :696 | Y `complete NO`, `halted` | Y "traded nothing", retention block |
| 3 | cli `results` / `top` (ledger readers) | Y `render_top_record` lib.rs:8190 | Y `listing_equity_note` lib.rs:8532; `stored_provenance` in `top` | n/a | **N**: no validation state, no in-sample line (`render_top_record`) | Y `complete` column, `newest_complete` | Y "NO COMPLETE RUN" |
| 4 | api `/engine/top.json` (text of #3) | Y | Y | n/a | **N** (same as #3) | Y | Y |
| 5 | api `/backtest.json` | implicit (the ledger holds stored runs only) | Y `equity_note` member backtest.rs:652 | n/a | **N**: no field exists (results.rs:289-369) | Y `halted`, `months_found` :663/:669 | Y `trades:0` and `has_complete_trade_total` |
| 6 | api `/frontier.json`, `/trades.json` | implicit | Y `equity_note_member` frontierjson.rs:136 | n/a | **N** | via the ledger parent | Y |
| 7 | api sweep console `report` (cli text relayed live) | Y; generated words refused, sweeprun.rs:3063 | Y (cli text) | n/a | Y while live (cli text) | Y | Y |
| 8 | web `/backtest`: drill-down of an open run | n/a | Y `openCharges` +page.svelte:9311, :10223 | n/a | **N** | Y :8638, :8965 | Y `noTrades` :4129 |
| 9 | web `/backtest`: crown "The answer", comparison board, rung leaders | n/a | **N** :7828, :8047, :8221, :8245, :8587 | n/a | **N**; a row is labelled "Finished result" (comparison.js:76) | Y (excluded from ranking, counted) | Y `NO_TRADES` |
| 10 | saved ledger `results/runs.bin` + `frontier.bin` | stored only, by construction (lib.rs:16546) | derivable from `underlying` | n/a | **N**: no `validated` byte | Y `halted` byte, `months_found` | Y `trades == 0` sentinel |
| 11 | web saved-backtest viewer (Boolean search) | "Real OHLCV" | Y "before costs" App.svelte:177 | n/a | Y "Training found the setting. The later period..." | Y "Saved checkpoint / refused" :162 | Y |

Label (a) holds on every path. Synthetic bars reach only cli stdout, which always carries `PROVENANCE`. The api console refuses `sweep`, `audit` and `auto` by name (sweeprun.rs:3063-3072). The ledger is never written for generated bars (lib.rs:16546-16549). No path combines synthetic bars with the stored banner. `runner::synthetic` appears in cli/api production code only at lib.rs:2764, :3920 and :16552; every other hit is inside test modules.

Label (c) is n/a everywhere. An UNVERIFIED dated cost row refuses with a remediation and produces no figure (`Dated::Unverified => Err(Refusal::with_remediation(..))`, costs dated.rs:180-186). Equity paths charge nothing and say so. The unsourced "Verified" rates are earlier findings (hunt-costs-5, p13num-1) and are not re-reported here.

## New findings

### CE-93 (low): the in-sample / not-validated state of a recorded run is never saved, so every ledger reader outside `range-all` shows candidates as finished results with no in-sample or multiplicity warning

- **Where:**
  - crates/cli/src/results.rs:289-369 (`Record`: `halted` is stored; nothing records `validate` or a validation verdict).
  - crates/cli/src/lib.rs:17837-17880 (`record_run` writes the row).
  - crates/cli/src/lib.rs:8184-8260 (`render_top_record`, served as `/engine/top.json`).
  - crates/api/src/backtest.rs:640-670 (`/backtest.json` run object) and :835 (`best_complete`, taken across the whole ledger).
  - crates/api/src/frontierjson.rs (no validation or in-sample member; `grep -i 'validat\|in sample' frontierjson.rs backtest.rs topjson.rs trades.rs` finds only page-window text).
  - web/src/routes/backtest/+page.svelte:7817-7829 (crown "best complete run") and web/src/lib/comparison.js:74-78 (`label: 'Finished result'`, `eligible: true`).
- **Code:**
  ```rust
  /// `1` when a budget stopped the walk short, so `depth` is PARTIAL ...
  pub halted: u8,
  ```
  `halted` is the only run-state byte in the record. `validate` is folded into the identity hash (`policy_of`, lib.rs:10625) and cannot be read back.
  ```rust
  const UNVALIDATED: &str = "!! NOT VALIDATED -- walk-forward, PBO and the bootstrap did NOT run.\n\
     These are CANDIDATES, not findings. ...";
  ```
  This is printed only into the live text (lib.rs:11764, :11819). The same ledger rows rendered by `range-all` carry `IN_SAMPLE_WARNING` (lib.rs:15890: "the variant was CHOSEN on the same bars it is SCORED on ... Treat these totals as an upper bound").
- **Why it is wrong:**
  - The web launch form offers `validate` off ("Disabling validation produces unvalidated candidates", +page.svelte:610), and the api passes it through (sweeprun.rs:531, :734-738).
  - Once the run finishes, `/backtest.json`, `/frontier.json`, `/engine/top.json`, `cli top` and the `/backtest` crown show that row exactly like a validated one. The crown calls it "best complete run" and the comparison board calls it "Finished result".
  - Even a validated run's ledger money is the in-sample best-of-search total. Its walk-forward, PBO and bootstrap verdicts are not stored (`range_over_inner`'s own text: "computes the walk-forward, the PBO and the bootstrap per rung and discards the report that carries them").
  - `best_complete` and the crown take the maximum across every instrument and feed in the ledger. That is the §1 "largest of N" comparison, presented with no in-sample line.
  - The cli carries the warning on `range-all`'s rendering of the same stored records (lib.rs:15890) and drops it on `top` and `results`. The api and web drop it everywhere. This is the label loss §5 says a banner exists to prevent ("byte-identical in shape, so the banner is the only thing separating them", lib.rs:10588-10591).
- **Repro (not run):** POST a `range-rung` (or `screen`) for NIFTY with `"validate":"0"`, then GET `/backtest.json`. The run object has `halted:false`, a nonzero `pessimistic` and no validation field. Open `/backtest`: the run is crowned "best complete run" with no "not validated" or "in sample" text. Run `cli top`: TOP COMBINATIONS prints with only the stored banner.
- **Minimal fix:**
  - Append a `validated: u8` byte to the next `Record` version (append-only, a new stride), carry it in `/backtest.json`, and mark or exclude `validated == 0` rows in `best_complete` and the crown.
  - Put one fixed in-sample sentence (the `IN_SAMPLE_WARNING` text, served once) on `/backtest.json`, `/frontier.json` and `render_top_record`, and render it above the crown and the comparison board.

### CE-94 (low): the `/backtest` crown, comparison board and rung leaders show a cash equity's totals with no gross-of-every-charge label. The api sends `equity_note` on every stock run, and the page renders it only in the drill-down

- **Where:** web/src/routes/backtest/+page.svelte:4111 (`const openCharges = $derived(chargeScope(openRun));`, the only `chargeScope` call). Render sites: :9311-9314 and :10221-10224 (drill-down only). The crown at :7817-7829 (`<span class="v up">{money(best.pessimistic)}</span>`), the comparison cells at :8047, the rung leaders at :8221 and :8245, and the run table at :8587 print money with no charge statement. The `coverScope` at :3623 is about the launch form's picked symbols, not the recorded runs on screen.
- **Why it is wrong:**
  - CLAUDE.md §1: "equities may be ranked cost-excluded only as labelled research, every such report stating it is gross of every charge".
  - The crown is a ranking ("best complete run", highest worst-case total) and can crown RELIANCE. The api already sends the note on that run object (backtest.rs:644-652: "a RELIANCE run here is a ranked cash equity ... served with neither the gross-of-every-charge label ...").
  - The cli listing over the same rows prints the note above its table and its BEST COMPLETE RUN line (`listing_equity_note`, lib.rs:8520-8546, whose own doc says it was "left bare on the reasoning that it only lists the ledger").
  - The web repeats the defect that cli fixed: the label is in the JSON and is never rendered on the headline. The crown also ranks a gross stock total directly against index totals with no note.
- **Repro (not run):** use a ledger holding one complete RELIANCE run and one NIFTY run where RELIANCE has the higher `pessimistic`. Open `/backtest` and choose "Show every feed" if needed. The crown names RELIANCE with "worst-case total ₹..." and no "gross" text. The note appears only after "Drill in".
- **Minimal fix:** when any run in `rankableRuns` (or `best`) carries `equity_note`, render `chargeScope(best).serverNote` (or `.note`) inside the crown and once above the comparison board. Add a page test that a stock crown contains "gross of every charge".

## Verification of earlier rows in this theme

| ID | Status | Evidence at 1f4de71 |
|---|---|---|
| CE-79 (pass 15) | NOT FIXED | web/src/routes/ingest/+page.svelte:4661 `pilot = { ...pilot, busy: false, error: String(why) };` still has no render site. `grep 'blocked_by\|surveyed'` finds 0 hits in the page. |
| CE-81 (pass 15) | NOT FIXED | same file :8789 `class:up={f.finished \|\| (f.legsDone ?? 0) >= (f.legs ?? 0)}` and :8756 `n(runState.rowsNow - runState.rowsAtStart)` are unchanged. `f.skipped` is not rendered. |

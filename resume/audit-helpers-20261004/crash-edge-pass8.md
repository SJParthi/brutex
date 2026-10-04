# Crash and edge-input audit, pass 8 (vendor parsers, web under hostile replies)

**Verdict at `final/all-fixes-zero` 1f4de71: no crash path found. There are 5 new findings, CE-56 to CE-60: 1 medium and 4 low.** Three of the low findings are silent or near-silent degrades on the parser side, and one is a precision gap in the web front end. The medium one is a duplicate-key gap: D-1531 closed it on the intraday decoder only, and it is still open on the Dhan rolling-option reader.

Method: I read the source for every finding. For CE-56..CE-59 the parser behaviour was proved with a throwaway `cargo test -p pull --lib` in a scratch worktree at 1f4de71 (ran; the worktree has since been removed). CE-60 is from source only (not run).

## New findings

### CE-56 (medium): the Dhan rolling-option reader silently keeps the last of a repeated key
- `crates/pull/src/rolling.rs:545` `let root: serde_json::Value = serde_json::from_str(body).map_err(|_| RollingError::NotJson)?;`
- **Why it is wrong.** attackdata-3 / D-1531 added `http::repeated_key` because `serde_json` keeps the last value of a repeated key and says nothing. The comment calls this "two answers in one body". That check runs only inside `http::decode_body` (http.rs:962). `rolling::read` parses the body on its own and never calls it. Its only caller is `api/src/server.rs:14011`, which is the live F&O rolling path. So `"close":[100.00],"close":[200.00]` lands as 200.00 and nothing reports it. `fno::names` (fno.rs:175) and `masters::nse_index_csv` (masters.rs:354) have the same gap; see CE-58.
- **Repro (ran).** I fed `{"data":{"ce":{"timestamp":[1700000000],"open":[100.00],"high":[100.00],"low":[100.00],"close":[100.00],"close":[200.00],"volume":[7]}}}` to `read(.., "CALL", Rupees)`. The result was `Ok`, with close = 20000 paisa.
- **Fix.** Make `repeated_key` `pub(crate)` and call it in `rolling::read`, `fno::names` and `nse_index_csv` right after the parse succeeds, refusing by name as `decode_body` already does.

### CE-57 (low): the CSV archive decoder's degrades are logged only at Debug, which is filtered by default
- `crates/pull/src/csv.rs:555` `&telemetry::Event::debug("pull.csv", "file decoded")` is the only place three counts are reported:
  - `unreadable_volume`: the volume is stored as 0 (csv.rs:885).
  - `unreadable_oi`: the open interest is stored as absent.
  - `negative_volume`: the whole row is skipped (csv.rs:879).
- **Why it is wrong.** The default telemetry floor is Info (`telemetry/src/sink.rs:302`). The api keeps Info unless `BRUTEX_LOG_LEVEL` says otherwise (server.rs:18727). So on a default run, a GDFL/TrueData file whose rows were dropped or zero-filled leaves no trace in the log, and the receipt's census only counts window and session drops. The comments here say these counters exist to keep the degrade from being silent under §4. The JSON siblings for the same facts emit at Warn and also `eprintln!` (http.rs:1733, 1767, 3497).
- **Repro (ran).** `decode("…,abc,0\n", TrueDataIndex)` returned `Ok` with volume 0. At the default floor no event is written; I confirmed this from the level check in source.
- **Fix.** When any of the three counts is non-zero, emit a separate Warn event plus an `eprintln!`, the same pattern as `note_negative_volume_bars`.

### CE-58 (low): `nse_index_csv` silently drops categories that are not shaped as its doc says, and keeps the last of a repeated category
- `crates/pull/src/masters.rs:366-372`: `let Some(list) = names.as_array() else { continue; }; … let Some(name) = name.as_str() else { continue; };`
- **Why it is wrong.** The doc (masters.rs:332-339) says it "refuses rather than emits ... an object whose values are not arrays of strings". The code skips such values and refuses only when zero rows remain overall. If NSE changes one category to `[{"name":..}]` or to a bare string, that category disappears from `nse_indices.csv` and the master lands as `Written`. A repeated category key keeps only its last list (the CE-56 class).
- **Repro (ran).** Input: `{"Broad":["NIFTY 50"],"Sectoral":[{"name":"NIFTY BANK"}],"Thematic":"NIFTY X","Broad":["NIFTY NEXT 50"]}`. Output: `Ok("index_name,category\nNIFTY NEXT 50,Broad\n")`. NIFTY 50, the Sectoral category and the Thematic category are all lost.
- **Fix.** Turn both `continue`s into refusals that name the category and the element, and add the `repeated_key` check from CE-56.

### CE-59 (low): the CSV archive decoder passes a negative open interest through, and the store kills the member late
- `crates/pull/src/csv.rs:853` `let parsed = text.trim().parse::<i64>().ok();`. Only `i64::MIN` is refused.
- **Why it is wrong.** `-5` decodes to `Some(-5)`. `store::format::Bar::counts_are_sane` (store/src/format.rs:823) then refuses it at the append with `ImpossibleCount`. That loses the whole member and its derived rungs, and the error carries a batch index with no CSV line. This is the D-0323 failure mode the volume path was fixed for. The JSON shapes skip and count such rows instead (c4a-2 / D-1490, http.rs:1767). So the third door is still open on the archive path.
- **Repro (ran).** `decode("20221003,09:15:01,38445.65,0,-5\n", TrueDataIndex)` returned `Ok`, with oi = `Some(-5)`.
- **Fix.** Treat a negative open interest like a negative volume: skip the row, count it in `Tally`, and emit at Warn (see CE-57). The alternative is to refuse with `CsvError` naming the line.

### CE-60 (low): `/bars.json` and `/bars/window.json` send i64 bar fields as bare JSON numbers, and `/db` and `/markets` never check them against 2^53
- `crates/api/src/server.rs:2080-2092` `r#"{{"t":{},"o":{},"h":{},"l":{},"c":{},"v":{},"oi":{}}}"#, … bar.volume, … bar.open_interest.to_string()`. The window route uses the same shape (server.rs:3495).
- **Why it is wrong.** The store accepts any non-negative i64 volume, open interest or price. `ohlc_is_sane` and `counts_are_sane` have no upper bound, and `one_number` takes any i64. Other routes handle this already:
  - `frontierjson.rs:857`, `livejson.rs:207` and `logs.rs:450` state the 2^53 rule.
  - The index-stop routes send these same fields as strings (indexstopcandlesjson.rs:224).
  - `/backtest` admits rows through `validateRunForComputation`.

  `web/src/routes/db/+page.svelte` and `routes/markets/+page.svelte` contain no `isSafeInteger` check at all. A stored value above 2^53, such as a corrupt vendor volume, is shown rounded in the grid and the chart with no refusal.
- **Repro (not run).** From source: store a volume of 9007199254740993. `/bars.json` emits that literal, and `JSON.parse` yields 9007199254740992.
- **Fix.** Either send `o h l c v oi` as decimal strings on both routes, as the index-stop routes do, or have the two pages refuse any non-safe integer with a named reason.

## Sites checked that hold

**JSON intraday decode (`http.rs`)**
- **Not JSON, HTML served with 200, empty body, or a body truncated mid-field:** `parse_answer` fails, and `not_json` refuses by name (http.rs:956, 1044).
- **BOM:** `reqwest` text decoding strips it. A raw BOM is refused by `serde_json`.
- **Repeated key:** `repeated_key` refuses it (D-1531).
- **Rupee prices:**
  - Exponent and arbitrary-precision text are shifted exactly by `number_text`, which is bounded by `MAX_PRICE_TEXT`.
  - A value below half a paisa that snaps to 0 is refused.
  - A negative price is refused.
  - A Paisa-scale float is refused, because `as_i64` returns None.
- **Overflow:** `Paisa::from_rupee_text_half_up`, `csv::paisa`, `to_paisa` and `ts_micros` all use checked arithmetic, so an overflow is refused by name.
- **Counts:** `i64::MIN` is refused as the null sentinel. A negative volume is zeroed and counted on an index, and skipped and counted on other listings.
- **Price fields:** a null price is skipped and counted at Warn with an `eprintln!`. A missing field is refused with its bar index.
- **Envelope:** the envelope is resolved once (`container`), so two objects cannot be spliced together. A ragged array is refused by `LengthDisagreement`. More than `MAX_ROWS` rows is refused. A body over the cap is refused before it is read.
- **Timestamps:**
  - Milliseconds or microseconds in a seconds feed land past `MAX_DAY_NUMBER` and are refused as `TimestampRefused`.
  - Text stamps must be exactly 19 bytes with range-checked fields.
  - The stated offset is bounded to ±14 h.
  - An off-grid broker stamp is refused before the fold (ingest.rs:1061).
  - A duplicate stamp is deduplicated if identical and refused if it conflicts.
  - Out-of-order input is refused by `fold::fold` (`OutOfOrder`) before any append.
- **Decoded price scale:** `DECODED_PRICE_SCALE = Paisa`, so callers do not multiply by 100 twice (server.rs:6520).

**Rolling (`rolling.rs`)**
- A null side means an empty answer, and an absent side is refused.
- Ragged arrays are refused.
- A stamp, volume or open-interest cell that is null, text, fractional, negative or `i64::MIN` is refused.
- The sub-half-paisa and negative price guards are present.
- A non-numeric implied-volatility cell is refused, not filed as absent.
- Only the repeated-key gap remains (CE-56).

**CSV archive (`csv.rs`, `archive.rs`)**
- CRLF is trimmed.
- A header shape is checked exactly, so a BOM header is refused by name.
- Field counts are held in a fixed array.
- Dates and clock times are checked digit by digit, with no sign accepted.
- A negative price, or one with three or more decimals, is refused with its line number.
- A member over `MAX_MEMBER_BYTES` is refused before it is read.
- A member that is not UTF-8 is refused (`MemberNotText`).
- `..` in a path is refused.
- The gaps are CE-57 and CE-59.

**Masters (`masters.rs`)**
- Size floor and ceiling are enforced.
- A body opening with `{`, `[` or `<` is refused.
- A first line without a comma is refused.
- Required columns are checked; a BOM-prefixed first column fails this loudly.
- Every row is validated through `decode_master_row` and refused with its line number.
- An empty master is refused.
- The swap is locked and atomic.
- The gap is CE-58.

**NSE constituents and links (`nse.rs`)**
- Size is checked first, and a BOM is stripped.
- A file whose header does not match, and so is not CSV, is refused.
- Quoted fields are honoured.
- A wrong field count, an empty symbol or ISIN, or a duplicate symbol is refused by row.
- The comment-stripped href scan uses only `get`, so it cannot panic.
- Links are capped by `MAX_INDEX_LINKS`.

**Cash auction master (`cash_auction.rs`)**
- A BOM is stripped. A header with a duplicate or empty name is refused.
- A row whose width differs from the header is refused.
- The eligibility flag must be exactly 0 or 1.
- A conflicting identity refuses the whole master.
- Records are bounded and quoting is strict.
- Lifecycle dates use checked conversion.

**SSM (`ssm.rs`)**
- A response that is not JSON, or has no `Parameter.Value`, is refused by name.
- An empty value is refused (`SecretError::Empty`).
- The refusal detail echoes only allowlisted text.

**`fno::names`**
- A list that is missing or contains a non-string element is refused, with nothing partial returned. The repeated-key gap is noted in CE-56.

**Web (`web/src`)**
- Every request goes through `lib/ask.js` with a deadline (default 15 s; the gaps route asks for 60 s, the masters refresh 90 s), so a hung server cannot leave a spinner forever.
- `/gaps` reads the body as text, then parses it, and treats a 404 or HTML body as a named refusal. `isWholeAudit` fails closed on any missing key.
- `/backtest` checks `mask_words` as exactly six canonical decimal strings decoded with BigInt (`lib/mask.js`). Its run-admission step refuses unsafe integers.
- `lib/live-progress.ts` validates every field before folding events, and its identifiers are safe-integer aware.
- The index-stop launch path fails closed on any unconfirmed response and never resends.
- `/db` and `/markets` bars:
  - A body that is not JSON shows `HTTP n, and the body was not JSON`.
  - A server `error` field is shown verbatim.
  - A 206 surfaces its faults.
  - The gap is CE-60.
- The `/mapping` status read keeps its previous list when the server answers non-2xx. I accepted that as a deliberate choice: the comment explains it, and an empty list hides the section rather than claiming the folder is empty.

## Verification tally
No earlier findings were in scope for re-verification in this pass, so there are no FIXED / NOT FIXED rows.

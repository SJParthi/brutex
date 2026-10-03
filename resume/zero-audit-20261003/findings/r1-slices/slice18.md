# slice18 — 3 findings

Slice: crates/cli/src/{execution_v3,execution_v4,expression,expression_pricing,expression_search,expression_search_reader,fold_audit,frontier}.rs (production code; tests read where useful).

## F1 [medium] docs/02-store-format.md §13 documents frontier.bin as version 4 / 272-byte rows; the code writes and reads only version 7 / 280-byte rows with a different layout
- where: docs/02-store-format.md:703-748 vs crates/cli/src/frontier.rs:184 (`const VERSION: u32 = 7`), :198-201 (`STRIDE = 280`), :418-462 (`to_bytes`), :1803-1840 (`check_header` refuses any version != 7)
- what: 02-store-format.md says it is the authority for bytes on disk ("This document is the authority; the code follows it", line 3). Its §13 says:
  - the header version is `4`,
  - each row is 272 bytes,
  - bytes `195..200` are "reserved, all zero; any non-zero byte is refused",
  - the seal is at `264` and covers `0..264`.

  The code does something else:
  - It writes and accepts only version 7, and `check_header` refuses a version-4 file outright.
  - Rows are 280 bytes.
  - Byte 195 is `require_protective_exits`, and a nonzero value there is legal.
  - Bytes 196..200 hold `min_fill_headroom_bp` as an `i32`. Nonzero is legal; only a negative value is refused.
  - Bytes 264..272 hold `min_avg_rr_bp`.
  - The seal is the first eight BLAKE3 bytes over `0..272`, stored at 272..280.

  Anyone who decodes `frontier.bin` from the authoritative document gets three things wrong. They refuse every real file on the version check. They use the wrong stride, so every row after row 0 is misaligned. They also treat every row whose protective-exit or headroom byte is set as corrupt, which is every row under the default rules (`require_protective_exits` defaults to on and `min_fill_headroom_bp` defaults to 200). Versions 5, 6 and 7 (D-1623 is the latest) changed the format without a matching change to the document.
- evidence: a precise trace. `sed -n 703,748p docs/02-store-format.md` shows "version 4", "Each row is 272 bytes", "| 195 | 5 | reserved, all zero; any non-zero byte is refused |" and "| 264 | 8 | first eight BLAKE3 bytes over `0..264` |". frontier.rs:184 `const VERSION: u32 = 7;`, :198 `pub const STRIDE: u64 = 280;`. In frontier.rs `to_bytes`, byte 195 is `put(&[u8::from(self.rules.require_protective_exits)])`, bytes 196..200 are `put(&i32::try_from(self.rules.min_fill_headroom_bp)...)` and bytes 264..272 are `put(&self.rules.min_avg_rr_bp...)`. `seal_of` hashes `raw[..PAYLOAD_BYTES]` with `PAYLOAD_BYTES = 280 - 8 = 272`. No test checks §13 against the code (`grep -rln "Ranked frontier" crates/` finds nothing).
- fix: Rewrite §13 as "version 7, 280-byte rows". The table should show:
  - byte 195 as the protective-exit flag (0/1, any other value refused),
  - bytes 196..200 as `min_fill_headroom_bp` as `i32` (negative refused),
  - bytes 264..272 as `min_avg_rr_bp`,
  - the seal at 272..280 over `0..272`.

  Note that versions 4–6 are refused with the `mv` remedy. Add a docs test that compares the §13 stride and version numbers against `frontier::STRIDE` and the header version.

## F2 [low] frontier.rs rustdoc states the wrong stride, the wrong row width and that rows carry no P&L
- where: crates/cli/src/frontier.rs:192-197 (STRIDE doc), :214-219 (`Row` doc), :1666-1672 (`index_of` comment)
- what: The doc on `STRIDE` says "144, and the last eight are the seal", but the constant beneath it is `280`. The `Row` doc says "There is no P&L here and that is deliberate … What a trade of it would have earned … lives on `crate::results::Record`, one per run". In fact `Row` stores `trades`, `cell_wins`, `pessimistic`, `worst_trade`, `max_drawdown`, `min_win`, `gross_win` and `gross_loss` for every ranked row (`Row::of` fills them from the priced `grid::Cell`). The `index_of` comment justifies the BufReader with "at 208 bytes and the default 8 KiB, 39 per read"; with 280-byte rows the real figure is 29. Someone sizing reads or deciding where per-row money lives would be misled by text sitting right next to the code.
- evidence: frontier.rs:192-197 `/// 144, and the last eight are the seal. … pub const STRIDE: u64 = 280;`. Lines 214-219 contradict the fields at lines 258-288 and `Row::of` (:668-678 `pessimistic: cell.map_or(0, |c| c.pessimistic)`, etc.). 8192 / 280 = 29.
- fix: Change the STRIDE doc to 280 (272 payload + 8 seal). Replace the "no P&L" paragraph with a statement that each row carries the chosen cell's raw money fields, all zero when unpriced (`trades == 0`). Change the 208/39 numbers to 280/29.

## F3 [low] The on-disk formats for Execution V3 and Execution V4 (eight append-only files) are specified in no document
- where: crates/cli/src/execution_v3.rs:66-72 (1,024/128/1,024/1,024-byte records) and crates/cli/src/execution_v4.rs:68-75, 79-82, 117-120 (1,280/128/1,024/1,280-byte records, magics `BTX-EXV4-PARAM`/`-PCTL`/`-DISP`/`-CMPL`, files `execution-{parameters,percentiles,dispositions,completions}-v4.bin`); docs/02-store-format.md:3 ("This document is the authority; the code follows it") and :1594-1602 (successor widths "are specified in those documents")
- what: Both modules persist four fixed-stride, sealed, append-only files each. Execution V4 is on the production `ledger-v6 → … → Execution V4 → Selection V6` route (docs/21-institutional-sweep.md:16). CLAUDE.md §10 gives 02-store-format.md authority over bytes on disk. No document gives these files' magics, strides, field offsets, reserves, seal domains or publication order. 02-store-format.md has no section for them, and docs/21-institutional-sweep.md names Execution V4 only at the level of the pipeline. The format exists only in the code, so the rule "the code follows it" cannot be checked for these files.
- evidence: `grep -rln "execution-parameters-v4.bin\|BTX-EXV4" docs/` and `grep -rln "execution-parameters-v3.bin\|BTX-EXV3" docs/` both return nothing. `grep -n "^## " docs/02-store-format.md` has no Execution V3/V4 section. The only nearby section is §17, which covers `execution-percentiles-v1.bin` / `BRUTXEG1`.
- fix: Add a 02-store-format.md section, or a linked successor document like docs/21, giving for each V3/V4 file its magic, version, domain, stride, field offsets, reserve and canonical-encoding rules, seal domain, and the parameters → percentiles → dispositions → completion sync order.

## Known, still present / status notes
- W2-cli5-2 (expression_pricing `begin_expression` re-hashing the bars for each candidate) is listed as NOT-FIXED in the known ledgers. The current code at crates/cli/src/expression_pricing.rs:124-159 computes `execution_digest` once in `Prepared::new` and calls `Capture::begin_expression_with_digest`, so the issue appears fixed. Someone should update the ledger rather than reopen the finding.
- hunt-cli-b-1 (a rerun of an exhausted expression search still publishes a checkpoint) is still present at crates/cli/src/expression_search.rs:562-564. Beyond what is recorded: every rerun also re-verifies the whole history twice (:261, :295, each O(checkpoints + saved rows)). The common journal admits at most 1,000,000 links (`HISTORY_LIMIT`, :17), so after enough reruns of a finished search, `verify_history` refuses for good with "history admission limit exceeded".

## What I checked and found clean
- expression_search: the checkpoint encoding and decoding match the documented 3,246 bytes. `verify_transition` replays with the exact work delta; `Cursor::advance` charges one unit per node and returns the Candidate on the node that emits it, so the replay is exact. The counter reconciliation, the initial-state checks and the 64 MiB signal-file admission (56·rows + 3,561 = HEADER 3,497 + FOOTER 64) all hold.
- expression_search_reader: the byte charges match the documented passes, and continuation admission holds.
- expression.rs: the row codec, seals, publication and two-pass verified reader hold.
- expression_pricing: `Plan::words` covers every `Rules` field, the override validation is stricter than `Rules::stated`, and `Prepared` and `Verify` hash the same execution bars (`project_onto_execution` returns the full slice on both paths).
- fold_audit: the two-cursor pairing (traced through missing, extra and moved buckets) and the runtime calendar (`Runtime::kind_of` equals static `kind_of`) hold. The cash-schedule gap is documented.
- frontier: append, rollback, duplicate refusal, index and refresh hold. The remaining issues are already known (W2-cli5-4, W2-cli9-0).
- execution_v3/v4 codecs: the fixed reader requires reserves to be zero and re-encodes byte-canonically, and enum decoders refuse unknown values. Their open source-authority and lookup-cost issues are already recorded (F-232FBF, F-A02E36, F-A5D460, F-D3D96B, etc.).

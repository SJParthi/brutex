# Batch-1 C4 groups never fixed: original finding text

Copied on 2026-10-03 from the operator's Mac (work-20260925/state/c4/wave1-args.json for the group-to-finding map, and the finding ledger in state/c4/wave1-free.json and state/*.json). Finding text only; no GDFL data rows.

| Group | Severity | Findings |
|---|---|---|
| api-02 | medium | W1-api3-6, W1-api3-1, W1-api4-6, W1-api6-0 |
| api-03 | medium | W1-api6-7, W1-api2-8, W1-api2-7, UC-19 |
| cli-04 | medium | W2-cli5-3, W2-cli5-4, W2-cli6-0, W2-cli7-5 |
| cli-07 | low | W2-cli1-4, W2-cli1-6, W2-cli1-3, W2-cli1-2 |
| cli-09 | low | W2-cli2-5, GAP14-59, W2-cli3-5, W2-cli5-2 |
| cli-11 | low | GAP15-18, GAP15-19, W2-cli6-3, W2-cli7-0 |
| cli-13 | low | W2-cli10-1, W2-cli10-2, W2-cli10-0, GAP15-17 |
| cli-16 | low | W2-cli14-1, W2-cli14-2, W2-cli14-3, GAP15-20 |
| cli-17 | low | GAP14-66, W2-cli15-2, W2-cli15-4, W2-cli16-1 |
| cli-18 | low | W2-cli16-3, W2-cli16-2 |
| engine-02 | low | W3-engine1-2, W3-engine1-4 |
| pull-06 | low | GAP16-24, GAP16-28, GAP2-45, W1-pull2-8 |
| pull-08 | low | W1-pull3-2, W1-pull3-4, W1-pull3-7, GAP16-23 |
| runner-02 | medium | W3-runner2-2, W3-runner2-3, W3-runner2-1, W3-runner2-4 |
| runner-03 | medium | AC-whp-tb-2, W3-runner4-1, ET-strategies-trades-ranking-costs-7, ET-strategies-trades-ranking-costs-6 |
| runner-05 | low | W3-runner1-3, AC-whp-cx-2, W3-runner2-5, W3-runner2-8 |

## api-02

### W1-api3-6 (medium, bug) crates/api/src/ingest.rs:84

parse_spot / queue (and server::pull_spot, pull_run) (crates/api/src/ingest.rs:84): Maximum member input. MAX_MEMBERS = 2,000, and its doc says a caller 'may legitimately tick all' 750 NIFTY Total Market names. The `member=SYM&` fields for all 750 NTM symbols are 11,309 bytes (computed from core::universe::NIFTY_TOTAL_MARKET, with & encoded as %26), which exceeds MAX_FORM_BYTES = 8,192. The request gets a framework 413 and never reaches the parser. The named TooManyMembers refusal is unreachable over HTTP (2,000 members need at least 18,000 bytes) Code path: server.rs:52: `const MAX_FORM_BYTES: usize = 8 * 1024;`. server.rs:14871: `.layer(axum::extract::DefaultBodyLimit::max(MAX_FORM_BYTES))`. The handlers take `body: String` (ingest.rs:2139, server.rs:10126, 10018). Test search: a_request_naming_more_members_than_the_bound_refuses_before_it_allocates calls parse_spot directly.

Evidence:

- crates/api/src/server.rs:52 sets `const MAX_FORM_BYTES: usize = 8 * 1024;`. server.rs:14871 applies `.layer(axum::extract::DefaultBodyLimit::max(MAX_FORM_BYTES))` inside `admitted()`, which wraps every route. The handlers `ingest::queue` (ingest.rs:2137), `server::pull_spot` (10124) and `pull_run` (10016) all take `body: String`, so the limit applies when that string is extracted.
- ingest.rs:84 has `pub const MAX_MEMBERS: usize = 2_000;`. Its doc at ingest.rs:77-78 says the widest set is the NIFTY Total Market at 750 and "a caller may legitimately tick all of it". But server.rs:52 has `const MAX_FORM_BYTES: usize = 8 * 1024;`, applied at server.rs:14871 as `.layer(axum::extract::DefaultBodyLimit::max(MAX_FORM_BYTES))`.

### W1-api3-1 (medium, cost) crates/api/src/pullrun.rs:483

rows_now (called by conduct_with's ROWS_TICK ticker and before/after every pass) (crates/api/src/pullrun.rs:483): per one progress refresh of a running pull (every 5 s, plus 2 per pass), the cost is O(total manifest bytes + E log E) per tick during an active pull. Synchronous inside tokio::spawn'd tasks, so it blocks a Tokio worker; it grows with store size: the bytes of every vendor manifest (read cap 268,468,224 B each) plus entries·log(entries) for held_entries, paid on every call whose manifest stamp moved, and a pull rewrites the manifest on each commit. Auditor verdict: undocumented-scan. Documented: Partly. docs/06-limits.md 'Census-backed GET routes … D-0686' names `pullrun::rows_now` only as a census_now caller in its freshness paragraph, and says the first call after a manifest changes pays read_all+held_entries. ...

Evidence:

- pullrun.rs:483-489: `rows_now` calls `crate::server::census_now(site)` and sums `counters().1`. It is called at 907, at 940 and 943 around every pass, at 980, and from the ticker at 921-927: `loop { tokio::time::sleep(ROWS_TICK).await; let seen = rows_now(&site); ... `conduct` is started with `tokio::spawn(crate::pullrun::conduct(...))` at server.rs:10048.
- `crates/api/src/pullrun.rs:483-489` is `let (censuses, _) = crate::server::census_now(site); censuses.iter().filter_map(census::VendorCensus::counters).map(|(_months, rows, _entries)| rows).sum()`. The ticker at `pullrun.rs:921-927` is `tokio::spawn(async move { loop { tokio::time::sleep(ROWS_TICK).await; let seen = rows_now(&site); ...

### W1-api4-6 (medium, bug) crates/api/src/recovery.rs:1065

reconcile_pending (crates/api/src/recovery.rs:1065): Rerun over an unchanged store is not idempotent. Each run re-appends every in-scope Unverified gap attempt with diagnostics increased by 1, so the recorded diagnostics count and both journals grow with the rerun count, not with new evidence. This also applies to Unverified outcomes that retry_day's receipt_state recorded with a receipt, which then get the 'lost receipt' qualification Code path: the pending filter L1034-1039 includes `Status::Unverified`. An in-scope item goes through assess (L1060). If `found.retry_days.is_empty()` then L1068 `item.diagnostics = item.diagnostics.saturating_add(1);` and the status stays `Status::Unverified` (L1069).

Evidence:

- Setup: NIFTY 1day, 2026-08-27, empty store.
- The test used an empty store and a NIFTY 1day plan for 2026-08-27.

### W1-api6-0 (medium, cost) crates/api/src/verify.rs:194

vendor (called per GET /verify.json from server.rs:4070) (crates/api/src/verify.rs:194): per one GET /verify.json?feed=... request, the cost is O(W + K) time with K file opens, O(K) space per request; it grows with manifest append-log length W (one row per write, so every resumed backfill day adds one) plus K held instrument-months (one bar-file open, try-lock, fstat, header read and two record preads each). Auditor verdict: unbounded-per-request. Documented: none: docs/06-limits.md mentions /verify.json only as a census_now caller in the D-0686 section; verify.rs:38-41 itself says 'UNVERIFIED as a measurement ... no bench'

Evidence:

- crates/api/src/verify.rs:194 has `for entry in manifest.newest() {`. crates/pull/src/manifest.rs:2741 has `for held in self.log.iter().rev() {`. Then verify.rs:203 calls `scrub::one(&entry, root, census.vendor, symbol_id)` once per key. File::open of the bar file (store/src/file.rs:1137) File::open of the lock plus `Flock::try_lock_shared` (file.rs:1144-1147) fstat (file.rs:1153)
- (1) server.rs:4070 is `let report = crate::verify::vendor(&site.store_root, census);`. It sits directly inside `async fn verify_json` (server.rs:4041). `/store.json` is wrapped in `crate::detail::run(...)` at server.rs:4169, with a 429 when saturated. The router layers in `admitted()` (server.rs ~14866-14872) are same-origin, logging, body-limit and framing. (2) verify.rs:194 is `for entry in manifest.newest() {`.


## api-03

### W1-api6-7 (medium, bug) crates/api/src/verify.rs:203

vendor (crates/api/src/verify.rs:203): A pull holding a month's writer lock while /verify.json runs gets reported as a store disagreement. Code path: store/src/file.rs:1146 `Flock::try_lock_shared(handle, lock_path.clone()).map_err(|refusal| lock_fault(...))?` refuses; pull/src/scrub.rs:214 special-cases only `StoreError::Missing`, so a lock becomes `Finding::Unreadable`. verify.rs:204 counts it, `verified()` is false, and `say()` (:109-119) tells the operator 'these month(s) read as held while the disk says otherwise'.

Evidence:

- The pull writes each month through `crates/pull/src/ingest.rs:2622-2623`: `let mut file = BarFile::open_or_create(store_root, path, symbol_id)`. That takes the exclusive month lock at `crates/store/src/file.rs:999-1003` (`Flock::try_lock(...)`).
- store/src/file.rs:1145-1147: `open_existing` takes the month lock with `Flock::try_lock_shared(handle, lock_path.clone()).map_err(|refusal| lock_fault(&lock_path, refusal))?`. If a writer holds the lock, `lock_fault` (:2733-2736) turns `TryLockError::WouldBlock` into `StoreError::Locked`. pull/src/scrub.rs:214-219: only `StoreError::Missing` gets its own finding.

### W1-api2-8 (low, cost) crates/api/src/booleanqualification_projection.rs:114

row_detail (crates/api/src/booleanqualification_projection.rs:114): per one row on a Boolean qualification page (up to 256 rows), the cost is O(limit × folds) per page; it grows with walk-forward folds per row; every fold of every row on the page is rendered, and the api layer has no fold paging and no named fold bound. Auditor verdict: needs-measurement. Documented: none

Evidence:

- booleanqualification_projection.rs:114 renders every fold of every row: `"rows":folds.folds().iter().enumerate().map(|(index,fold)|json!({...})).collect::<Vec<_>>()`. It runs inside row_detail, which :47 calls once for each row of the page (the loop at :25). The page limit is 1..=256 (booleanevidencejson.rs:99 `if !(1..=256).contains(&limit)`, and again at boolean_qualification_reader.rs:267 `limit > 256`).
- `crates/api/src/booleanqualification_projection.rs:114` builds `"rows":folds.folds().iter().enumerate().map(...)`, one JSON object per fold. `row_detail` runs once for every page row (`:47`). Pages are capped at 256 rows by `booleanevidencejson.rs:99` (`!(1..=256).contains(&limit)`) and `cli/src/boolean_qualification_reader.rs:267` (`limit > 256`).

### W1-api2-7 (low, cost) crates/api/src/indexmap.rs:42

Published::read / join (called per /indexmap.json request) (crates/api/src/indexmap.rs:42): per one /indexmap.json request, the cost is O(file + S × C × L + universe) per request; it grows with catalogue file bytes (re-read and re-parsed per request, with no byte cap) + index symbols × catalogue names × name length (every verbatim miss scans every catalogue name) + the merged universe (server.rs scans merged.by_key to find index symbols). Auditor verdict: undocumented-scan. Documented: none. The module header claims the work is only over the ~136 index symbols and that the masters are parsed once at startup; the catalogue re-read and the universe scan are not mentioned

Evidence:

- What the code does on each GET /indexmap.json (server.rs:30153-30208): (1) It re-reads the catalogue file with no size cap: `Published::read` calls `std::fs::read_to_string(path)` at indexmap.rs:42-43, then `from_text` parses and collapses every line.
- `indexmap_json` (server.rs:30171-30173) calls `crate::indexmap::Published::read`, which runs `std::fs::read_to_string(path)` with no byte cap (indexmap.rs:42-43), then `from_text`.

### UC-19 (low, bug) crates/api/src/indexmap.rs:42

Published::read (crates/api/src/indexmap.rs:42): Oversized catalogue file: an unbounded read_to_string on every request (the same defect D-0033 fixed with MAX_MASTER_BYTES and census fixed with MAX_MANIFEST_BYTES) Code path: indexmap.rs:42-43 `let text = std::fs::read_to_string(path)` has no metadata size check first, and it is called on every /indexmap.json request (server.rs:30171-30173). Test search: indexmap.rs tests use Published::from_text only; none reads a file or checks a size ceiling.

Evidence:

- `crates/api/src/indexmap.rs:41-43`: `Published::read` calls `std::fs::read_to_string(path)` with no `metadata` size check before it and no ceiling constant anywhere in the file (grep for MAX_/metadata/set_len finds none). Its only production caller is `crates/api/src/server.rs:30171-30173` (`masters_dir()/nse_indices.csv`), inside the `/indexmap.json` handler, which is registered at `server.rs:14938`.
- crates/api/src/indexmap.rs:41-49 `Published::read` calls `std::fs::read_to_string(path)` with nothing in front of it. crates/api/src/server.rs:30171-30173 (`indexmap_json`) calls it on every `GET /indexmap.json`, on `masters_dir()/nse_indices.csv`. crates/api/src/master.rs:304 `MAX_MASTER_BYTES`, checked at master.rs:331-339 before `read_to_string` (D-0033, docs/05-decisions.md:1198-1204).


## cli-04

### W2-cli5-3 (medium, bug) crates/cli/src/frontier.rs:1146

Frontier::refresh / absorb_new_rows (crates/cli/src/frontier.rs:1146): A corrupt or concurrent file: a single bad-seal, invalid-schema or non-contiguous row anywhere in frontier.bin makes every cached /frontier.json refresh fail, so requests alternate between a refusal and an O(rows) fresh reopen Code path: Exact code path: index_of sets write_refusal and returns Ok (1607-1611). The read-only open succeeds (834-846). absorb_new_rows begins `self.refuse_integrity_failure_for_write()?;` (1146). api detail.rs:431-434 drops the slot on a refresh Err and returns it, and frontierjson.rs:162 turns it into missing_file_response. The next request reopens fresh.

Evidence:

- At the claim's line, frontier.rs:1137-1146, `pub fn refresh(&mut self) -> Result<(), Refusal> { self.absorb_new_rows() }` and absorb_new_rows opens with `self.refuse_integrity_failure_for_write()?;`. That check (1224-1231) returns Err whenever `write_refusal` is Some, and nothing tells a reader from a writer: `require_parent` is not consulted.
- crates/cli/src/frontier.rs index_of (1603-1646) finds a bad seal (1607-1611), an invalid schema (1618-1621) or a non-contiguous duplicate block (1628-1634). open_read_bounded (834-846) then returns Ok with `write_refusal` set. That function returns Err whenever `write_refusal` is Some (1224-1231).

### W2-cli5-4 (medium, bug) crates/cli/src/frontier.rs:2920

Row::verdict (crates/cli/src/frontier.rs:2920): The rule set is only partly checked: `admitted` ignores min_avg_rr_bp (which can be computed from stored fields) and min_fill_headroom_bp/min_weakest_bp, and none of them is flagged unchecked, so /frontier.json shows PASS for rows Rules::admits refuses Code path: verdict reads only min_win_rate_bp, min_rr_bp, min_ret_over_dd_bp, min_trades and min_assurance_bp (2933-2949). Rules::admits also requires `Self::fills_hold(self.min_fill_headroom_bp, cell) && Self::avg_payoff_holds(self.min_avg_rr_bp, cell)` (lib.rs:8517-8518). avg_payoff_holds uses only wins, trades, gross_win and gross_loss (lib.rs:8642-8660), all stored on Row.

Evidence:

- `Row::verdict` (crates/cli/src/frontier.rs:2920-2957) sets `admitted: win_rate && reward_to_risk && return_over_drawdown && trades && assurance`. `Rules::admits` (crates/cli/src/lib.rs:8510-8518) also requires `Self::fills_hold(self.min_fill_headroom_bp, cell) && Self::avg_payoff_holds(self.min_avg_rr_bp, cell)`. `avg_payoff_holds` (lib.rs:8642-8660) reads only wins, trades, gross_win and gross_loss.
- Row::verdict (crates/cli/src/frontier.rs:2920-2953) builds `admitted: win_rate && reward_to_risk && return_over_drawdown && trades && assurance`. Rules::admits (crates/cli/src/lib.rs:8511-8518) also requires `Self::fills_hold(self.min_fill_headroom_bp, cell) && Self::avg_payoff_holds(self.min_avg_rr_bp, cell)`. avg_payoff_holds (lib.rs:8642-8660) reads only cell.wins, cell.trades, cell.gross_win and cell.gross_loss.

### W2-cli6-0 (medium, cost) crates/cli/src/index_stop_search_checkpoint.rs:457

checkpoint::recover (called from index_stop_search::open_checkpoint on every execute) (crates/cli/src/index_stop_search_checkpoint.rs:457): per one launch/resume of a single-stop search, per acknowledged historical batch x selected rung, the cost is O(B * R * (C*P*D + C*S + C*days)) per launch; grows linearly with search history; it grows with B completed batches x selected rungs (<=8) x each child's full numeric replay: C candidates x P later periods x (draws+1) x 3 bootstrap procedures, plus C x S CSCV splits, ~7 calendar span walks per candidate, and a full decode+reconcile of both candidate cata .... Auditor verdict: undocumented-scan. Documented: Not documented for this path. docs/06-limits.md:8412-8414 (D-0524, for AND/expression searches) says 'verifying the complete linked history still costs O(checkpoints + saved child rows)', which understates this path beca ...

Evidence:

- `execute_observed_with` calls `open_checkpoint` at index_stop_search.rs:191. That calls `checkpoint::recover(` at :402 before any batch runs. It walks the whole acknowledged chain (checkpoint.rs:401-436) and reverses it (:440).
- api/src/indexstoplaunch.rs:319 calls `admission.execute_observed`. That leads to index_stop_search.rs:191 `open_checkpoint(...)` and then :402 `checkpoint::recover(`. index_stop_search_checkpoint.rs:442-465 runs `for mut frame in history { ... index_stop_qualification.rs:808 calls `Reader::open_bounded(..., Some(replay), Some(expected))`, which decodes both candidate catalogs (:374-380) and the daily reader (:392).

### W2-cli7-5 (medium, bug) crates/cli/src/institutional_statistics.rs:1204

InstitutionalStatisticsLedgerV1::require_files_unchanged / append_complete_locked (crates/cli/src/institutional_statistics.rs:1204): concurrent/replaced file: the data file is atomically replaced (rename) by a byte-identical copy between open and append Code path: The staleness check compares only FileSnapshotV1 { length, digest } of content (:862-866, :1381-1407): the held handle hashes to the old snapshot and the reopened path (:1208 `open_read(path)`) hashes to identical bytes, so both compares pass. append_sync (:1170, :1181) then writes and fsyncs the held, now unlinked, inode and returns `Appended`.

Evidence:

- `require_files_unchanged` (crates/cli/src/institutional_statistics.rs:1196-1216) first compares `snapshots(&mut self.files, ...)` from the held handles against `self.snapshots`. It then reopens each path with `let mut reopened = open_read(path)?;` (:1208) and checks `if snapshot_file(&mut reopened, path)? != expected` (:1209). `FileSnapshotV1` holds only `{ length: u64, digest: [u8; 32] }` (:863-866).
- `FileSnapshotV1` is only `{ length: u64, digest: [u8; 32] }` (institutional_statistics.rs:862-866). `snapshot_file` (:1381-1407) hashes the length and the file's contents, and nothing else. `require_files_unchanged` (:1196-1217) runs two checks. `append_sync` (:1358-1363) seeks, writes and calls `sync_all` on the held `File` (called at :1170 and :1181), and :1193 returns `Appended`.


## cli-07

### W2-cli1-4 (low, bug) crates/cli/src/admission_store.rs:958

AdmissionAuthorityLedger::append_decisions / commit_locked / verify_supplied_decisions (crates/cli/src/admission_store.rs:958): Crash (process kill) between the per-decision write_all calls leaves a whole-record PREFIX of a population's decision block. That block can never be completed, and every later population is refused, so the admission ledger is permanently wedged and a rerun can never succeed (CLAUDE.md §3 rule 5). Code path: The append loop writes one 544-byte record per write_all (:958-968) and rolls back only on an in-process I/O error (rollback_message :1739). After a kill with k<n records written, the file length is still header + whole records, so check_header accepts it (:1540). scan_decisions makes P the trailing orphan with block.count = k (:1229-1252, :1384-1402).

Evidence:

- append_decisions writes one 544-byte record per call: `for decision in decisions { let raw = decision.to_bytes()?; if let Err(why) = self.decision_file.write_all(&raw) { return Err(rollback_message(...)) } }` (:958-968). rollback_message (`match file.set_len(at)`, :1739-1740) runs only when there is an in-process io::Error.
- `append_decisions` (crates/cli/src/admission_store.rs:958-968) writes each record in its own call, `for decision in decisions { ... }`, on an unbuffered `std::fs::File` (imported at :32). The only rollback is `rollback_message`'s `file.set_len(at)` (:1739-1740), and it runs only when an I/O error is seen inside the process. On reopen, `check_header` accepts the length because it is a whole number of records (:1540).

### W2-cli1-6 (low, bug) crates/cli/src/admission_store.rs:1189

scan_decisions (crates/cli/src/admission_store.rs:1189): Maximum input: open reserves its per-population indexes at the DECISION count, and the retained `blocks` map keeps that capacity. Retained index space is therefore proportional to decisions, not populations as documented. Code path: `let record_count = len.saturating_sub(HEADER) / ADMISSION_DECISION_STRIDE; ... reserve_map(&mut blocks, capacity, ...)?; order.try_reserve(capacity) ...; seen.try_reserve(capacity)` (:1189-1200). `blocks` is moved into the ledger (:724-736), and HashMap never shrinks.

Evidence:

- In crates/cli/src/admission_store.rs, scan_decisions (:1184) computes `let record_count = len.saturating_sub(HEADER) / ADMISSION_DECISION_STRIDE;` (:1189). It then calls `reserve_map(&mut blocks, capacity, "admission decision block index")?` (:1193), `order.try_reserve(capacity)` (:1196) and `seen.try_reserve(capacity)` (:1199). `blocks` gets one insert per population (:1210, :1258).
- *The code.** crates/cli/src/admission_store.rs:1189-1193 computes `let record_count = len.saturating_sub(HEADER) / ADMISSION_DECISION_STRIDE;`, where the stride is 544 (:62). It then calls `reserve_map(&mut blocks, capacity, "admission decision block index")?`, and `reserve_map` (:1798-1804) is `map.try_reserve(additional)`. `from_files` moves `blocks` into the ledger struct (:707, :730; field at :539).

### W2-cli1-3 (low, cost) crates/cli/src/anchored_search_lineage_v2.rs:873

AnchoredSearchLineageV2Ledger::append_completion / structural_receipt (crates/cli/src/anchored_search_lineage_v2.rs:873): per one V2 lineage pair append or lookup, the cost is O(file bytes + pairs) per append; O(file bytes) per lookup (3 single-pass file hashes); it grows with ledger file bytes plus completed pairs; cumulative Theta(N^2) over N appends. Auditor verdict: undocumented-scan. Documented: The lookup's own rustdoc (v2:741-743) says 'Generation validation is O(file bytes); only the final hash lookup is average O(1)'. Nothing in docs/06-limits.md. Dead in non-test builds (lib.rs:60-67 `allow(dead_code)`).

Evidence:

- **Append rescans everything.** `append_completion` rehashes the Completion file and then calls `self.scan()` (v2:868-873). }` and ends in `self.require_unchanged()` (v2:737). Each call hashes the whole file through `hash_exact_prefix(&mut reader, path, before.len())` (v2:1615), whose loop runs until `remaining` reaches 0.
- Lookup: `structural_receipt` calls `self.require_unchanged().map(|()| self.receipts.get(pair_id).copied())` (v2:760-762). Each call re-reads its whole file through `let digest = hash_exact_prefix(&mut reader, path, before.len())?;` (v2:1615) in 16 KiB chunks. Append: `append_locked` calls `require_unchanged` (v2:794), writes the members (v2:831) and rehashes the member file.

### W2-cli1-2 (low, cost) crates/cli/src/anchored_search_lineage_v3.rs:939

AnchoredSearchLineageV3Ledger::append_completion / structural_receipt_locked_with_after_lookup (crates/cli/src/anchored_search_lineage_v3.rs:939): per one V3 lineage pair append or lookup, the cost is O(file bytes + pairs) per append or lookup; each lookup makes 2 require_unchanged calls x 3 file_generation calls x 2 hash passes; it grows with ledger file bytes plus completed pairs; cumulative Theta(N^2) over N appends. Auditor verdict: undocumented-scan. Documented: Module rustdoc only (v3:16-22); nothing in docs/06-limits.md. The module is dead in non-test builds: lib.rs:69-76 `#[cfg_attr(not(test), expect(dead_code, ...))] mod anchored_search_lineage_v3;`

Evidence:

- `append_completion` finishes with `self.completion_generation = file_generation(...)?; self.scan()` (v3:934-939). `scan()` rebuilds the index from scratch: it runs `self.receipts.clear()` (v3:736), then `while sequence < completion_records { read_member_pair(...); read_completion_at(...); validate_complete_pair(...); self.receipts.insert(...) }` (v3:744-758), and ends with `self.require_unchanged()` (v3:792).
- `append_completion` (920-939) re-hashes the whole file and rescans every pair: `self.completion_generation = file_generation(...)?; self.scan()`. `scan` (715-792) loops `while sequence < completion_records` (744). A lookup, `structural_receipt_locked_with_after_lookup` (823-833), calls `self.require_unchanged()` twice (829, 832).


## cli-09

### W2-cli2-5 (low, cost) crates/cli/src/boolean_search_projection.rs:245

RungReader::rows / require_current (search detail page); also boolean_search_reader::Reader::require_current and boolean_qualified_observer::Reader::require_current (crates/cli/src/boolean_search_projection.rs:245): per one dashboard page of a search rung (<= 256 rows), the cost is O(H + H') full record re-reads with seal hashing, twice per page; it grows with search journal history length H and qualified-campaign history H' (both grow with batches and retries). Auditor verdict: undocumented-scan. Documented: no. The reader doc says only 'Recheck every retained journal record'. There is no per-page cost statement in docs/06-limits.md.

Evidence:

- `RungReader::rows` calls `self.require_current()?` before paging and again after (crates/cli/src/boolean_search_projection.rs:245 and :252). `require_current` runs `parent.require_current()`, then `campaign.require_current()`, then `self.source.require_current()` (lines 182-190).
- RungReader::rows calls `self.require_current()?` before and after the page (boolean_search_projection.rs:245 and :252). That call fans out to `parent.require_current()` and `campaign.require_current()` (:182-189). Each one loops `for old in &self.history { let now = self.snapshot.read(old.sequence, self.max_bytes)?; ...` (boolean_search_reader.rs:226-231; boolean_qualified_observer.rs:175-180).

### GAP14-59 (low, test-gap) crates/cli/src/boolean_statistics_v1.rs:179-181 (produce calls admit), 276-292 (admit)

cli Boolean statistics: produce's call to admit (the physical-budget and degenerate-shape refusal before numeric work) is not caught by any cli Boolean test

Evidence:

- The stopped auditor's run filtered to `boolean_candidate_v1::statistics::` (21 tests): `MISSED crates/cli/src/boolean_statistics_v1.rs:281:5: replace admit -> Result<(), String> with Ok(())`.  My run over all 172 tests matching `boolean` (run-cli-x2.log): `1 mutant tested in 12m: 1 caught`. But the sole failure is `generated_search_recovers_same_ordinal_and_refuses_missing_ancestry`, with `Error: "generated search child exceeded its 360s/8MiB bound after 360.1s"`; the other 171 passed. The unmutated control binary (cli-682f45f1eabc4f9b, same test, --exact) fails the same way: `test result: FAILED. 0 passed; 1 failed ... finished in 361.48s` / `rc=101 elapsed=362s`. So that failure is not a catch.  Callers of statistics::produce: boolean_catalog_command.rs:259 (the Prepared/campaign/search paths) and boolean_qualification_tests.rs:87. tests/binary.rs and tests/sweep_evidence.rs mention "boolean" 0 times.

### W2-cli3-5 (low, cost) crates/cli/src/execution_capability.rs:529

ExecutionStrategyCapabilityV1::require_binding / ::new (via validate_population_capabilities per population row) (crates/cli/src/execution_capability.rs:529): per one population row checked against its capability, or one capability minted, the cost is Theta(A) per row (two passes over all percentiles: policy.digest() plus three put_percentiles), so preparation is O(R x A), not O(R + A); it grows with A = runtime percentile atoms of the side policy (3 x up to max_levels_per_axis; max_levels_per_axis is runtime-sized with no named constant ceiling, only != 0 at runner exit_grid_policy.rs:565). Auditor verdict: false-o1-claim. Documented: Contradicted. docs/06-limits.md §139 (line ~7212-7216): 'For R population rows and A percentile atoms, preparation and ordered hashing are O(R+A) time'.

Evidence:

- `PreparedExecutionCapabilitiesV1::from_population_v4` (execution_capability.rs:965) first runs `validate_population_parameter_pair` (execution_capability.rs:977). That function calls `parameters.validate()?` once for each side (execution_capability.rs:1187). For every row it calls `capability.require_binding(parameters_for_row, &row)?` (execution_capability.rs:1249).
- In `validate_population_capabilities`, the page loop (execution_capability.rs:1229) calls `capability.require_binding(parameters_for_row, &row)?` at :1249 once per population row. `require_binding` starts with `parameters.validate()?;` (:529). It always runs `if self.parameter_id != self.derived_id()?` (:356).

### W2-cli5-2 (low, cost) crates/cli/src/expression_pricing.rs:148

Prepared::capture -> candidate_trades::Capture::begin_expression -> begin_with (crates/cli/src/expression_pricing.rs:148): per per priced expression candidate, the cost is O(bars) of redundant hashing per candidate; the value is identical for every candidate; it grows with execution bars in the month (7 blake3 updates per bar). Auditor verdict: undocumented-scan. Documented: not in 06-limits (grep for execution_digest, begin_expression and Capture::begin found nothing)

Evidence:

- The search loop calls `evaluate_candidate(root, run, column, bars, &expression, pricing)?` once per `Step::Candidate` (`crates/cli/src/expression_search.rs:542`). That calls `pricing.capture(root, &attempt, expression)?` (`:617`). Which calls `Capture::begin_expression(root, attempt, &self.bars, &self.column, expression)?` (`crates/cli/src/expression_pricing.rs:147-148`).
- expression_search.rs:541-542 calls `evaluate_candidate` for every `Step::Candidate`. Lines 616-617 then run `if let Some(pricing) = pricing { pricing.capture(root, &attempt, expression)?; }`. expression_pricing.rs:147-148 calls `Capture::begin_expression(root, attempt, &self.bars, &self.column, expression)?`, which goes to `begin_with`.


## cli-11

### GAP15-18 (low, bug) crates/cli/src/global_replay_v4.rs:228-232 (budget), 436-447 (push ceiling), 586-592 (kind-5 row per admitted priceable decision)

The Global Replay V4 candidate budget ignores the VIX records it will write, so a replay admitted under the budget can still refuse at the record ceiling after all OOS work

Evidence:

- global_replay_v4.rs:228-231 `let candidate_budget = bounds.records.checked_sub(10 + selected_count).ok_or(...)? / 2;`. Record kinds come from codec Writer::new calls: 0 header (codec:97), 1 envelope (global_replay_v4.rs:280), 2 witness (codec:120), 3 candidate (codec:176), 4 decision (codec:230), 5 VIX (codec:264), 6 completion (codec:290). The VIX row is pushed at global_replay_v4.rs:591 `push(records, &codec::vix(sequence, entry, exit)?, bounds)?;` for each admitted priceable decision. ledger_all.rs:1235 `CEILING_BYTES: u64 = 1 << 36`. No runtime probe was taken for this item.

### GAP15-19 (low, bug) crates/cli/src/global_replay_v4.rs:571-606 (account_decision), 425-426 (per-row check only)

Global Replay V3 and V4 dropped GR V1's enforcement of the frozen exit-policy quality ceilings (max_ambiguous_bars / max_gap_fills) over globally admitted trades

Evidence:

- grep over fix/c2-final crates/cli/src and crates/runner/src for `max_ambiguous_bars|max_gap_fills` finds GR V1 (global_replay.rs:335-336, 461-462, 782-792), execution_capability, candidate_universe:4475-4476 (per-candidate in-sample) and boolean_candidate_grid. There are no hits in global_replay_v3.rs or global_replay_v4*.rs. GR V4 account_decision (571-606) pushes decision and VIX rows and sums money with no quality accumulation.

### W2-cli6-3 (low, bug) crates/cli/src/index_stop_store.rs:310

encode_admitted (crates/cli/src/index_stop_store.rs:310): The pre-publication decode gate passes the byte limit where the record limit belongs Code path: `// Decode the exact bytes before publication, using the same cold-read gates.` is followed by `decode(&bytes, identity, max_bytes)?;` (:309-310), but the signature is `fn decode(bytes: &[u8], identity: [u8; 32], max_records: u64)` (:318). The record admission check (`u64::try_from(count)? > max_records`, :327, and `charged > max_records`, :365) is therefore checked against bytes.

Evidence:

- In crates/cli/src/index_stop_store.rs, `encode_admitted(identity, evaluations, length, max_bytes)` (:252-256) has no record limit available to it. At :309-310 it runs `// Decode the exact bytes before publication, using the same cold-read gates.` followed by `decode(&bytes, identity, max_bytes)?;`.
- crates/cli/src/index_stop_store.rs:309-310 reads `// Decode the exact bytes before publication, using the same cold-read gates.` then `decode(&bytes, identity, max_bytes)?;`. The signature at :318 is `fn decode(bytes: &[u8], identity: [u8; 32], max_records: u64)`. > max_records` (:327) and `charged > max_records` (:365).

### W2-cli7-0 (low, cost) crates/cli/src/institutional_evidence.rs:1078

build_institutional_evidence_v1 -> complete_population_values / data_completeness_value (crates/cli/src/institutional_evidence.rs:1078): per one admission-evidence record per Population V4 cell (population_authority = Measured), the cost is O(E) per cell (+ O(L^2) resolved-grid digest re-check); it grows with E = bars in the one-minute execution/training series; hashed once per cell, twice when data completeness is Complete (cells x E over a population). Auditor verdict: undocumented-scan. Documented: no - module doc lines 40-41 say 'each cell projection and ordinal check is O(1)'; docs/06-limits.md §141 lists O(T), O(S), O(sum C_i) for evidence but not O(E) per cell; §126 only admits run-identity hashing once per run

Evidence:

- institutional_evidence.rs:1078 runs `let derived = derive_population_id_v1(authority)` inside complete_population_values. population_admission_writer.rs:314 then runs `validate_complete_population_authority(authority)?;`, and :1144 runs `let execution_digest = runner::identity::data_digest(authority.execution_series.bars());`. runner identity.rs:340 is `for bar in bars { hasher.update(...) }`.
- *The per-cell path.** `build_institutional_evidence_v1` (institutional_evidence.rs:947) calls `completeness_values` (:966) once per call. With `population_authority` set to Measured, that reaches `complete_population_values`, and :1078 `let derived = derive_population_id_v1(authority)` runs on every call.


## cli-13

### W2-cli10-1 (low, cost) crates/cli/src/population_admission_writer.rs:851

evaluate_population_side (crates/cli/src/population_admission_writer.rs:851): per per closed mask per side (result append into evaluated_sides), the cost is one realloc per push; worst case O(n) copy per push (O(n^2) total), allocator-dependent; Rust's amortised-O(1) doubling guarantee is forfeited; it grows with number of evaluated sides already retained (2 x closed masks). Auditor verdict: needs-measurement. Documented: Not documented. Writer rustdoc (lines 18-22) claims only O(rows) preparation. CLAUDE.md §3 rule 4 requires amortised-O(1) result append.

Evidence:

- At crates/cli/src/population_admission_writer.rs:850-858, evaluate_population_side runs `evaluated_sides.try_reserve_exact(1)...?; evaluated_sides.push(EvaluatedPopulationSideV1 {...})`. It runs once per closed mask per side, through evaluate_population_member (779-797), inside the uncapped population callback of produce_population_admission_v1 (576-580, `let mut evaluated_sides = Vec::new();`).
- *The code is as the claim describes.** At crates/cli/src/population_admission_writer.rs:850-858 the writer calls `evaluated_sides.try_reserve_exact(1).map_err(...)?;` and then `evaluated_sides.push(EvaluatedPopulationSideV1 { mask_words, support_hits: member.item.hits, direction, evaluated });`. `evaluate_population_member` calls this once per side, twice for each `ClosureVerdict::Closed` mask (lines 780-797).

### W2-cli10-2 (low, cost) crates/cli/src/population_admission_writer.rs:356

derive_strategy_digest_v1 (pub) (crates/cli/src/population_admission_writer.rs:356): per per exit cell (strategy identity for one cell_ordinal), the cost is O(G) per cell call; O(G^2) if a caller derives every cell of a grid through this public entry; it grows with G = cells in the evaluated exit grid. Auditor verdict: undocumented-scan. Documented: docs/06-limits.md §141 says per-cell projection is O(1) and that 'the previous accidental O(G^2) shape where each cell rebuilt the complete validation' is gone. That holds for the pub(crate) fast path only;

Evidence:

- crates/cli/src/population_admission_writer.rs:356-358: `let validated = resolved.validate_evaluation(evaluated).map_err(...)?;`, then it calls `derive_strategy_digest_from_validated_v1(... &validated, cell_ordinal)` at 359-369. validate_evaluation is O(G) per call (crates/runner/src/exit_grid_policy.rs:2170-2181).
- At crates/cli/src/population_admission_writer.rs:346, `pub fn derive_strategy_digest_v1` runs `let validated = resolved.validate_evaluation(evaluated).map_err(...)?;` (356-358) and only then enters `derive_strategy_digest_from_validated_v1` (359). `validate_evaluation` at runner/src/exit_grid_policy.rs:2170-2181 calls `validated_evaluation`, which runs `validate_complete_grid(&evaluated.grid)` (2582-2591).

### W2-cli10-0 (low, cost) crates/cli/src/population_base_evidence_ledger_v2.rs:1277

append_and_reopen_base_evidence_v2 (LedgerV2::open_inner -> scan -> validate_block) (crates/cli/src/population_base_evidence_ledger_v2.rs:1277): per per Candidate-universe Base Evidence family append (record append); production caller candidate_universe.rs:2692 via step3_orchestrator.rs:4148, the cost is O(R_total) per family append (2 full opens, each decoding and hashing every record) plus O(r) compare; cumulative quadratic in number of families; it grows with total Base records of every universe ever committed to the ledger (bounds.records), not the appended family. Auditor verdict: undocumented-scan. Documented: Module rustdoc lines 5-7 only ('Opening, append, exact retry comparison and fresh-reopen comparison are O(records)').

Evidence:

- No build or timing was run: load average was about 52, and a cli test rebuild takes 14-20 minutes, so this rests on reading the code only. `append_and_reopen_base_evidence_v2` (population_base_evidence_ledger_v2.rs:1271) calls `LedgerV2::open(root, bounds)` at 1277 and `LedgerV2::open_read(root, bounds)` at 1281. Both go to `open_inner` (862-874), and `open_inner` always calls `ledger.scan()?` (966).
- population_base_evidence_ledger_v2.rs:1277 runs `let mut ledger = LedgerV2::open(root, bounds)?;`, then append. For each record it does one seek and a 1 KiB read_exact (read_record_bytes, 1536-1547), a full decode that recomputes the seal hash (`if seal != digest_domain(RECORD_SEAL_DOMAIN, payload)`, population_base_evidence_v2.rs:691), and an ordered-hash update. scan_orphan (1358-1398) decodes any uncommitted tail.

### GAP15-17 (low, bug) crates/cli/src/population_base_evidence_v2.rs:578-610 (rate projections), 1609-1630 (measured_rate/rate_ppm floor); consumed at population_admission_v4.rs:800-805

Max-gated admission rates are floored before a `value > ceiling` test, so a candidate whose exact rate is above the operator's ceiling is Admitted into Selection V6 and execution authority

Evidence:

- Code: population_base_evidence_v2.rs:1621-1629 `let scaled = u128::from(part).checked_mul(u128::from(PPM))...; let quotient = scaled.checked_div(denominator)` (floor). runner/src/admission.rs:2891 `let violates = matches!(observation, ObservedU64V1::Measured(value) if value > ceiling);`. runner admission.rs:1503-1504 documents the 'canonical floor projection count * 1_000_000 / trades'. Probe (throwaway worktree wt/aud2-15 at 19acd032, uncommitted test appended to runner admission tests; policy max_losing_trade_rate_ppm=333_333, trades=3 wins=2 losses=1 losing_trade_rate_ppm=333_333; and max_ambiguous_fill_rate_ppm=333_333 with ambiguous_fill_rate_ppm=1_000_000/3): `CARGO_TARGET_DIR=.../tgt/aud2-15 CARGO_BUILD_JOBS=2 nice -n 19 cargo test -p runner --lib --locked --offline aud2_15 -- --nocapture` Output: `AUD2-15 losing-rate failed bit = false` / `AUD2-15 ambiguous-rate failed bit = false` / `test admission::tests::aud2_15_floored_max_rate_admits_an_exact_rate_above_its_ceiling ... ok` / `test result: ok. 1 passed`. The exact rate 1/3 is 333_333.33 ppm, above the 333_333 ceiling, and neither gate failed.


## cli-16

### W2-cli14-1 (low, cost) crates/cli/src/selection_v5.rs:2471

CommittedStoredSelectionV5::top_twenty_five (also top_ten 2500, successor_winners 2507, commit_stored_selection_v5 2577) (crates/cli/src/selection_v5.rs:2471): per one read of one rung's persisted Top-25/Top-10 (at most 25 fixed 1 KiB rows), the cost is top_twenty_five/top_ten: 8 full two-family Candidate Execution V3 grid replays + 2 O(C) projections + 10 whole-file BLAKE3 passes; successor_winners: 14 replays; it grows with C (all Population V5 rows/Execution V3 dispositions of the rung) times the per-mask-group exit-grid re-evaluation over training bars and grid cells, plus F. Auditor verdict: undocumented-scan. Documented: Partly. §163 says "Top-10 is a constant-size prefix operation only after the complete authoritative Top-25 has been reproduced".

Evidence:

- selection_v5.rs:2474-2475 and 2488-2489: top_twenty_five calls PreparedSelectionV5::from_committed_execution twice. from_committed_execution (selection_v5.rs:856-883) calls source.population_execution_source(), then source.ordered_authenticated_dispositions(), then population_execution_source() again.
- `selection_v5.rs:2474-2475` and `:2488-2489`: `top_twenty_five` calls `PreparedSelectionV5::from_committed_execution` once before the read and once after it. `from_committed_execution` (`:856-869`) makes three calls: `population_execution_source`, then `ordered_authenticated_dispositions`, then `population_execution_source` again.

### W2-cli14-2 (low, cost) crates/cli/src/selection_v6.rs:141

CommittedStoredSelectionV6::top_twenty_five (also top_ten 160, snapshot 85, stored_oos_witnesses 106) (crates/cli/src/selection_v6.rs:141): per one read of one rung's committed Top-25/Top-10 (one 16 KiB block), the cost is 2 Prepared::from_execution calls (each = 2 Population V6 -> execution_v3_replay_authority replays) + an O(H) scan (2 BLAKE3 over 16 KiB per block + a HashSet of H ids). snapshot = 3 from_execution + 2 scans; stored_oos_witnesses = 2 snapshots; it grows with C (Execution V4 candidates) x replay cost; H = committed 16 KiB blocks in global-selection-v6.bin (ceiling CEILING_BYTES/16384 = 4,194,304 via ledger_v6.rs:552-556). Auditor verdict: undocumented-scan. Documented: Only in the module doc, selection_v6.rs:10-12: "Source reauthentication/ranking is O(candidates + upstream file bytes). File integrity checks are O(history)".

Evidence:

- `top_twenty_five` (crates/cli/src/selection_v6.rs:141-157) does three things: `scan` (lines 252-293) reads every committed 16 KiB block: `verify_block` (lines 411-412) recomputes `block_identity` and `seal`, which is two BLAKE3 hashes of about 16 KiB each; `from_execution` (selection_v6_source.rs:38-43) calls `selection_v6_source`:
- `top_twenty_five` (selection_v6.rs:141-157) calls `Prepared::from_execution` twice, with `require_committed` in between. `from_execution` (selection_v6_source.rs:38-43) calls `source.selection_v6_source()`. `selection_v6_source` (execution_v4.rs:3603-3690) calls `population_execution_source()` twice, before and after, and re-opens every durable disposition in between (`ordered_authenticated_dispositions`).

### W2-cli14-3 (low, cost) crates/cli/src/selection_v6.rs:302

persist (called by commit_stored_selection_v6 180) (crates/cli/src/selection_v6.rs:302): per one Selection V6 commit (append one 16 KiB block), the cost is O(H) scan before the append + another O(H) scan in require_committed (190) + 2 Prepared::from_execution; it grows with H committed blocks. Auditor verdict: undocumented-scan. Documented: Only the module doc line 11 ("File integrity checks are O(history)"). No section of 06-limits covers V6 append cost.

Evidence:

- persist (selection_v6.rs:307) calls `let (len, found) = scan(&mut file, &path, bounds, expected)?;`. ledger_v6.rs:567-568 then calls selected.top_twenty_five() and selected.top_ten(). This path runs once per rung per ledger-v6 run: 8 files, one each at ROOT/selection/<rung>/global-selection-v6.bin (ledger_all.rs:176-189). An identical rerun reuses the existing block (`found` means nothing is appended, 309-315).
- This is a durable ledger commit, run once per rung per `ledger-v6` run (ledger_v6.rs:308 loops over the 8-entry `LEDGER_RUNGS`, then 384 `render_selection`, then 566 `commit_stored_selection_v6`). *Not a bounded constant.** H is capped by `CEILING_BYTES = 1 << 36` (ledger_all.rs:1235) divided by 16,384, which is 4,194,304 records, or a 64 GiB scan twice per commit.

### GAP15-20 (low, doc-false) crates/cli/src/step3_orchestrator.rs:139-141; also cli/src/lib.rs:1774-1779, cli/src/population_v6.rs:20-23, cli/src/execution_v4.rs:41-44, cli/src/stored_post_training_oos.rs:17-26, runner/src/exit_grid_policy.rs:9-11

Stale scope and caller claims in the V6-chain sources: 'two-instrument sweep surface', '§1 names two instruments', and dead_code reasons that say production callers are still pending

Evidence:

- step3_orchestrator.rs:139-140 `/// commit, depth, fallback or pre-resolved result. `underlying` is validated by` / `/// the canonical stored loader against the two-instrument sweep surface, and`. stored.rs:1885-1892 `let as_cash = InstrumentKey::cash(Exchange::Nse, underlying)...; as_cash.require_sweepable()...; Ok(as_cash)`. population_v6.rs:22 `reason = "Population V6 source retention and fixed receipt-last codec await their all-rung production caller"`. ledger_v6.rs:365 `commit_stored_population_v6_route(`. stored_post_training_oos.rs:25 `reason = "the stored OOS capability is the agreed Step-3 seam for the pending Selection V6 / Global Replay V4 coordinator"`. lib.rs:1774 `/// chain does not: `CLAUDE.md` §1 names two instruments as the engine surface`.


## cli-17

### GAP14-66 (low, test-gap) crates/cli/src/step3_orchestrator.rs:5178-5237 (test); 2331 (pub(crate) fn commit_stored_observation_statistics_v2)

step3_orchestrator: a test named 'is_crate_private', cited as proof for invariant OS-01, cannot observe privacy

Evidence:

- docs/04-invariants.md:3603 (OS-01) lists `cli::step3_orchestrator::tests::paired_observation_statistics_seam_is_crate_private_and_source_retaining` as proof of "The crate-private Observation-to-Statistics V2 transaction...".  The test ends `std::hint::black_box((entry, nifty, banknifty, base_evidence, observations, observation, statistics, projection));`.  The scan classified it NONE (no assert, no should_panic, no ?, no unwrap).

### W2-cli15-2 (low, cost) crates/cli/src/stored_data_completeness.rs:510

StoredDataCompletenessAuthorityV1::require_population (crates/cli/src/stored_data_completeness.rs:510): per one institutional-evidence binding, i.e. per strategy cell (institutional_evidence.rs:1119, reached from build_institutional_evidence_v1 per cell), the cost is O(E) per candidate cell, so O(C*E) per population; it grows with E execution bars. Auditor verdict: undocumented-scan. Documented: No. §142 (docs/06-limits.md:7338-7345) covers preparation only.

Evidence:

- In crates/cli/src/stored_data_completeness.rs:552-553, `require_population` ends with `let execution = StreamFactsV1::of("population execution", population.execution_series.bars())?;` and then, at :554, `if execution != receipt.execution`. `StreamFactsV1::of` walks every adjacent pair at :126-132 (`for (index, (left, right)) in bars.iter().zip(bars.iter().skip(1)).enumerate()`).
- *The claimed scan.** `StoredDataCompletenessAuthorityV1::require_population` (crates/cli/src/stored_data_completeness.rs:510) does exactly what the claim says: :552-553 `StreamFactsV1::of("population execution", population.execution_series.bars())?` inside it, the pairwise loop `for (index, (left, right)) in bars.iter().zip(bars.iter().skip(1)).enumerate()` at :126-132, then `digest: data_digest(bars)` at :137

### W2-cli15-4 (low, bug) crates/cli/src/stored_data_completeness.rs:325

prepare_stored_data_completeness_v1 / StoredDataCompletenessReceiptV1::validate (crates/cli/src/stored_data_completeness.rs:325): strict ChecksumReceiptV1 integrity permutation Code path: Path: :662-663 `daily_integrity: daily_reference.daily_integrity.byte()`. ChecksumReceiptV1 maps to byte 1 (runner/src/identity.rs:435-438). Then :667 `receipt.validate()?`, and :325-329 `if self.daily_integrity > 0 || self.minute_integrity > 0 { Err("...unknown integrity-evidence byte") }`.

Evidence:

- `crates/cli/src/stored_data_completeness.rs:662-663` copies it: `daily_integrity: daily_reference.daily_integrity.byte(), minute_integrity: daily_reference.minute_integrity.byte()`. `:667` then calls `receipt.validate()?`. `:325-329` refuses it: `if self.daily_integrity > 0 || self.minute_integrity > 0 { return Err("stored-data completeness receipt carries an unknown integrity-evidence byte"...`.
- crates/cli/src/strict_v6_inputs.rs:47-48 returns `ReferenceIntegrity::ChecksumReceiptV1(self.identity)`. The strict Step-3 route passes that into the binding at crates/cli/src/step3_orchestrator.rs:3051: `daily_integrity: context.integrity()`. crates/runner/src/identity.rs:435-438 turns `ChecksumReceiptV1(_)` into byte `1`.

### W2-cli16-1 (low, cost) crates/cli/src/stored_post_training_oos.rs:342

StoredPostTrainingOosCohortV1::mint_witness_inner (mint_witness / mint_witness_recorded) (crates/cli/src/stored_post_training_oos.rs:342): per one witness mint = one selected strategy's OOS replay. population_v6.rs:3224-3227 calls it once per admitted strategy with the cohort cached per family; §169 bounds P ≤ 200., the cost is Θ(S+Q+D+E) of non-replay work per witness: 1 full evaluator fold, ≥5 data_digest_with_daily_reference passes, 8 data_digest passes and 3 execution calendar receipts, on top of the inherent replay; it grows with S signal bars + Q exact-minute context bars + D daily bars + E execution minutes of the cohort. These are cohort-invariant but recomputed P times.. Auditor verdict: undocumented-scan. Documented: Contradicted. docs/06-limits.md §169 (line 8187) says constructing the cohort 'derives previous-day and exact-minute causal columns and hashes the retained snapshot' and that 'Minting every witness is proportional to the ...

Evidence:

- from_retained (stored_post_training_oos.rs:217-283) keeps the raw BoundedStoredContextV1 bars (step3_orchestrator.rs:487-497) and only runs derive_cohort_id and require_integrity (280-281). That is one execution calendar receipt (453), one data_digest_with_daily_reference (462) and four data_digest passes (488-491). Another calendar receipt over the execution minutes (349-356).
- (1) stored_post_training_oos.rs:347 calls `self.require_integrity()?`. derive_cohort_id (451-503) runs: Per runner identity.rs:527-600, this is a full data_digest of each of the three streams plus an eligibility scan; (2) Lines 349-356 build another execution calendar receipt.


## cli-18

### W2-cli16-3 (low, cost) crates/cli/src/strict_v6_inputs.rs:63

strict::size_sweeper (crates/cli/src/strict_v6_inputs.rs:63): per sizing one ledger-v6 rung's support threshold (ledger_v6.rs:309), 8 rungs per run, the cost is O(span bytes + months × fsyncs) per rung to learn one integer, and the NIFTY family commit then repeats the identical load; it grows with months × bytes of the NIFTY signal/daily/exact-minute span including prior context. Each month file gets a cold CRC audit plus a sweep_evidence begin/finish in the receipt root (checksum_receipts.rs:137-152).. Auditor verdict: undocumented-scan. Documented: Not in docs/06-limits.md. docs/21-institutional-sweep.md:46 says only 'It uses receipt-checked sizing'. The ledger_v6.rs comment 'before sizing loads all eight market spans' concedes the load but not the duplication.

Evidence:

- `strict_v6_inputs.rs:63-76` calls `load(StoredContextLoadSpecV1 { vendor, underlying: "NIFTY", rung_name: rung, from: request.from, to: request.to, signal_bound/minute_bound/daily_bound: bounds.*, .. Only `count` and the `strict` Arc (lines 78-80) are kept. `ledger_v6.rs:309` calls `size_sweeper` once per rung inside `for rung in LEDGER_RUNGS`.
- ledger_v6.rs:309-316 calls `strict::size_sweeper(&source_root, vendor, request, rung, bounds, &strict)` once per rung. At strict_v6_inputs.rs:63-76 that call does `load(StoredContextLoadSpecV1 { vendor, underlying: "NIFTY", rung_name: rung, from: request.from, to: request.to, signal_bound: bounds.signal_records, minute_bound: bounds.minute_records, daily_bound: bounds.daily_records }, &root, config)`.

### W2-cli16-2 (low, cost) crates/cli/src/trades.rs:473

Trades::open (writer), called by lib.rs ensure_trade_rows once per recorded run (crates/cli/src/trades.rs:473): per recording one run's chosen trades: record_all_attempt → record_trades → ensure_trade_rows (lib.rs:19407, 16310, 16256), the cost is O(H + T) per recorded run, where T is the run's own rows; it grows with H = every chosen-trade row ever recorded in the store. The writer open has no byte ceiling.. Auditor verdict: unbounded-per-request. Documented: Partly. docs/06-limits.md (~line 6353): 'Opening `chosen-trades.bin` rebuilds its identity index in O(all chosen rows)'.

Evidence:

- *Each open reads every row ever written, with no size limit.** `Trades::open` (trades.rs:473) measures the file length, calls `check_header`, then `index_of(&mut file, len)?`. *The source documentation is wrong for the writer.** trades.rs:56 says `| open | O(rows) | one pass to rebuild the index, once per process |`. `Frontier::open(root)` at lib.rs:16210. `result_set::Receipts::open(root)?` at lib.rs:16487.
- *The writer is reopened for every run.** lib.rs:16256 `let mut store = trades::Trades::open(root)?;` is inside `ensure_trade_rows`. *Each open reads every row.** On a non-empty file, `Trades::open` (trades.rs:473) runs `check_header`, then `index_of(&mut file, len)?` at 503-506. The API server calls `cli::range_over_for_attempt` on each backtest request (api sweeprun.rs:1476).


## engine-02

### W3-engine1-2 (low, cost) crates/engine/src/keep.rs:304

Best::offer (with sift_up keep.rs:378, take_root keep.rs:356, sift_down keep.rs:408; offer_level keep.rs:333) (crates/engine/src/keep.rs:304): per per survivor offered to the retention, the cost is O(1) to reject; O(log cap) to admit or evict; it grows with retention cap (sift depth about log2 cap). Auditor verdict: undocumented-scan. Documented: keep.rs:223-224 says 'an offer is one comparison to reject and a sift to admit' and never states the sift's cost. docs/06-limits.md does not mention keep::Best; its ranking paragraphs describe runner's heap.

Evidence:

- crates/engine/src/keep.rs:304-324 `offer`: when it is not full, it does `self.held.push(candidate); self.sift_up(...)`. `take_root` (keep.rs:355-361) does swap, pop, then `self.sift_down(0)`. `sift_up` (keep.rs:378-389) walks from a node to its parent (`above / 2`), so the index halves at every step. `sift_down` (keep.rs:408-426) walks to the weaker child (`at*2+1` / `+2`).
- `git diff 9f5c13c7 29025433 -- crates/engine/src/keep.rs` is empty, so the claim's line numbers still hold: offer at keep.rs:304, offer_level at 333, take_root at 356, sift_up at 378, sift_down at 408. A full heap does one `ranks_below(weakest, &candidate)` on `self.held.first()` (keep.rs:315-319).

### W3-engine1-4 (low, bug) crates/engine/src/keep.rs:262

Best::with_capacity (crates/engine/src/keep.rs:262): A caller-supplied cap is reserved eagerly and without a fallible path: cap*56 B beyond isize::MAX panics ('capacity overflow'), and a large representable cap aborts the process on allocation failure instead of refusing. Code path: keep.rs:265 `held: Vec::with_capacity(cap)` uses infallible Vec::with_capacity, whose documented behaviour is a panic past isize::MAX bytes and handle_alloc_error (abort) on refusal. The rest of the crate refuses through try_reserve (lib.rs:114-119, Breach::Memory). Test search: Best::with_capacity is called with 0/1/4/5/7/9/11/39/40/100/1<<20 in keep.rs and lib.rs tests; nothing larger.

Evidence:

- *What the code does.** `crates/engine/src/keep.rs:262-267` is `pub fn with_capacity(cap: usize) -> Self { Self { cap, held: Vec::with_capacity(cap), discarded: 0 } }`. `Itemset` is `{ mask: ConditionMask, hits: u64 }` (lib.rs:380-390, pinned by a const assert), which is 56 bytes.
- Code: crates/engine/src/keep.rs:262-266 is `pub fn with_capacity(cap: usize) -> Self { Self { cap, held: Vec::with_capacity(cap), discarded: 0 } }`. lib.rs:114-118 `fn reserved<T>` calls `rows.try_reserve_exact(capacity)?`. lib.rs:2096-2097 `fn cannot_grow` calls `out.try_reserve(by).is_err()`. lib.rs:2284 is `out.try_reserve(batch.len()).map_err(|_| Breach::Memory)?`. resume.rs:199/274/494 use `try_reserve` too.


## pull-06

### GAP16-24 (low, bug) crates/pull/src/http.rs:1242-1250 (also rolling.rs:662-681)

JSON rupee prices are rounded twice: serde_json parses the vendor decimal into an f64 before the 'text' half-up, contrary to the 'STILL NO FLOAT / the text is the truth' claims

Evidence:

- Probe, real `pull::http::decode_body` with the shipped Dhan descriptor (prices=Rupees): - `PROBE json 100.00499999999999999 serde-rendered 100.005 decode_body Ok(10001) exact-text-half-up Ok(10000)` - `100.004999999999999999999 -> serde 100.005, Ok(10001) vs exact Ok(10000)` - `24500.004999999999999 -> 24500.005, Ok(2450001) vs Ok(2450000)` - `0.0049999999999999999 -> 0.005, Ok(1) vs Ok(0)` - Control: `35922.6016 -> Ok(3592260) == Ok(3592260)`  Code: - http.rs:1247 `let text = number.to_string();` - serde_json-1.0.151 src/number.rs:356 `N::Float(f) => formatter.write_str(zmij::Buffer::new().format_finite(f))` - src/de.rs:638-667 (non-float_roundtrip `let mut f = significand as f64; ... f /= pow;`)

### GAP16-28 (low, doc-false) crates/pull/src/http.rs:873-877; fetch.rs:669-671; csv.rs:283-286; core price.rs:9-12; core vendor.rs:32-35; fno.rs:1088-1098; rolling.rs:702-703; api server.rs:12937

The snap-location and 'one law' documentation disagrees with the code in eight places; the store write boundary performs no snap

Evidence:

- Quoted lines at fix/c2-final: - http.rs:876 "`prices` therefore scales first and rounds once, while" - fetch.rs:671 "and a second rounding site is a second answer." - csv.rs:286 "snapping site competing with the one at the write boundary." - price.rs:11-12 "it happens once at the ingest boundary, and it is the only function in this crate that touches an `f64`" - vendor.rs:35 "[`Paisa::from_rupees_half_up`] like every other price." - server.rs:12937 "`rolling::read` converted at the boundary through" - fno.rs:1095 "`CLAUDE.md` s7 fixes ONE law for rupees to paisa." - rolling.rs:748-758 (shift_six adds 1 to the magnitude, then negates).  The callers of from_rupees_half_up come from a `git grep` over crates/*.rs: only lake/bar.rs:162.

### GAP2-45 (low, doc-false) crates/pull/src/http.rs:26-34; crates/pull/src/ssm.rs:824-826; crates/api/src/autopilot.rs:134-139, 921

Headers and docs describe the re-read and halt as live behaviour, and say the credential is read once per run

Evidence:

- http.rs:27-34 quoted above. ssm.rs:824 `// \`Info\`, and once per run.` ssm.rs:139-143 `the call is per instrument: \`api::server::broker_run\` loops its target list and each iteration reaches \`read_credential\``.

### W1-pull2-8 (low, cost) crates/pull/src/manifest.rs:2219

Manifest::entry / held / closes (documented bound) (crates/pull/src/manifest.rs:2219): per one census lookup, the cost is expected O(1); worst case O(keys) under hash collision; it grows with colliding keys in the std HashMap (adversarial or pathological only). Auditor verdict: false-o1-claim. Documented: §17 (measured flat to 1.05× at 100×, which is not a worst-case bound)

Evidence:

- crates/pull/src/manifest.rs:2219 says "**Lookup — O(1) worst case.** [`Manifest::entry`] is one probe into a table". manifest.rs:2767 (`closes`) says "**O(1) worst case**, the same bound [`Manifest::entry`] carries". manifest.rs:156 is `use std::collections::{HashMap, HashSet};`.
- crates/pull/src/manifest.rs:156 has `use std::collections::{HashMap, HashSet};`. manifest.rs:2264 declares `index: HashMap<EntryKey, Held>` with no hasher parameter. manifest.rs:2474-2475 builds it with `HashMap::with_capacity(reservation_for(header.n_valid))`.


## pull-08

### W1-pull3-2 (low, cost) crates/pull/src/refusal.rs:618

refusal::disposition_of (reached from http::HttpSource::weigh_answered_body on every 2xx answer) (crates/pull/src/refusal.rs:618): per one successful vendor HTTP response, before decode_body parses it again, the cost is O(body) full serde_json::Value DOM parse, plus DOM allocation, on every successful request, on top of decode_body's own O(body) parse; it grows with response body bytes, up to MAX_RESPONSE_BYTES = 64 MiB (http.rs:55). Auditor verdict: undocumented-scan. Documented: Contradicted. docs/06-limits.md:5254-5256 (§83.1): '**What would make it a defect.** Calling it on a success path, or per row, or without the transport bound in front of it. None of those exist today'.

Evidence:

- In `http.rs::window_async`, after the 64 MiB read (`body_within(&mut answer, MAX_RESPONSE_BYTES)`, http.rs:2679) and the `too_large` check, line 2712 runs `self.weigh_answered_body(&text, status)?;`. Inside, at http.rs:2820-2821: `if let Some(contract) = self.spec.error_names && let Some(named) = crate::refusal::disposition_of(text, contract)`.
- (1) crates/pull/src/refusal.rs:618-619: `pub fn disposition_of(body: &str, contract: &ErrorNames) -> Option<Disposition> { let value: serde_json::Value = serde_json::from_str(body).ok()?;` builds a full DOM of whatever body it is given. (2) On the success path, crates/pull/src/http.rs:2679 reads the body under MAX_RESPONSE_BYTES (64 MiB, http.rs:55).

### W1-pull3-4 (low, cost) crates/pull/src/request_minutes.rs:33

request_minutes::audit / audit_venue (called once per ingested request from ingest.rs:1113) (crates/pull/src/request_minutes.rs:33): per one vendor request window's minute-coverage audit, the cost is O(rows + days + gaps) time. Output is also O(gaps): one String, one Failure and two error/warn telemetry events per gap, with no cap; it grows with rows + days in the window + number of gaps. Gaps can reach about 187 per session day for an alternating series. The window is the vendor cap (e.g. max_days_per_call 45, vendor.rs:4317) or one month when the vendor publishes no cap. Auditor verdict: undocumented-scan. Documented: request_minutes.rs:2 states the scan cost ('O(rows + days + gaps)'). docs/06-limits.md has nothing on it and nothing on the uncapped per-gap log output

Evidence:

- (1) request_minutes.rs:34-36 runs a separate ordering pass before the main work: `rows.windows(2).any(|pair| matches!(pair, [a, b] if a.timestamp > b.timestamp))`. The monotone cursor at :112-124 then walks the rows a second time. (2) Every gap goes through `missing` (:151), then `note_request_failure` (:146-149).
- The audit runs once per fetched chunk: server.rs:5705-5734 builds a BarRequest per chunk and calls pull::ingest::from_window, which calls request_minutes::audit at ingest.rs:1113. The cursor at request_minutes.rs:112-127 visits each stamp once and each day once. The `rows.windows(2).any(...)` pre-pass at lines 34-36 is a second O(rows) pass with constant state.

### W1-pull3-7 (low, bug) crates/pull/src/rolling.rs:532

rolling::read (via number) (crates/pull/src/rolling.rs:532): A timestamp cell that is null, a JSON string ("1756698300"), a number with 3+ decimals, or a value beyond i64 is read as 0, so the bar is filed as ts_micros = 0 (1970-01-01). `number` (rolling.rs:635-647) returns 0 for any unreadable cell, and its own doc says zero is only for counts. The row is then keyed by rolling_key, whose expiry_of on a 1970 day refuses with the misleading reason 'the day is before this weekly regime was verified from'. Code path: `let ts_secs = number(stamps, at);`. `number` tries `cell.as_i64()` (None for null, a string or a decimal), then `crate::csv::paisa(&cell.to_string()).map_or(0, ...)`. csv::paisa (csv.rs:287-306) returns None for quoted text, for 'null' and for more than 2 decimals. read() therefore returns Ok with ts_micros 0 instead of refusing, which is a silent substitution §4 bans.

Evidence:

- *What is right in the claim.** crates/pull/src/rolling.rs:532 reads `let ts_secs = number(stamps, at);`. `number` (rolling.rs:635-647) calls `cell.as_i64()`. `csv::paisa` (csv.rs:287-306) returns None for any of these inputs: The bar is NOT filed as 1970-01-01. The only caller is api/src/server.rs:12815.
- crates/pull/src/rolling.rs:532 has `let ts_secs = number(stamps, at);`, and line 538 converts it with `ts_micros: ts_secs.saturating_mul(1_000_000)`. `number` (rolling.rs:635-647) returns `whole` from `cell.as_i64()`. csv::paisa (csv.rs:287-306) returns None in three cases: more than 2 decimals, non-digit text (so the quoted string `"\"1756698300\""` and `null` both fail), and `checked_mul` overflow.

### GAP16-23 (low, bug) crates/pull/src/rolling.rs:563-566, 650-682

The rolling price and spot conversion lacks http::one_price's two guards: a non-zero value under half a paisa lands as a zero price or spot, and a negative spot lands in the overlay

Evidence:

- Code: - fix/c2-final:crates/pull/src/rolling.rs:679 `PriceScale::Rupees => brutex_core::price::Paisa::from_rupee_text_half_up(&cell.to_string())...` — no zero-snap check, no sign check. - :563-566 `spot: match spot { Some(a) => paisa(a, at, scale, "spot")?, None => OI_NULL }`. - store/src/format.rs:1007 `fn is_sane(&self) -> bool { true }` for Overlay.  Probe: - `PROBE rolling price 100.0 spot -24650.05 -> (bar.close, overlay.spot) Ok((10000, -2465005))` - `PROBE rolling price 0.001 spot 0.001 -> Ok((0, 0)) ; http decode_body same price -> Err("...\"open\" holds 0.001, which is not zero and is smaller than half...")` - `PROBE rolling price -0.001 spot 24650.05 -> Ok((0, 2465005)) ; http decode_body same price -> Err(...)`


## runner-02

### W3-runner2-2 (medium, cost) crates/runner/src/exit_grid_policy.rs:2057

ResolvedExitGridV1::evaluate_training_grid_attested (combined attest-and-price door) (crates/runner/src/exit_grid_policy.rs:2057): per one closed mask x side in the production Candidate / Population producers, the cost is about 7 linear passes per candidate: minute-grid walk, BLAKE3 over every bar, acceptance census, source coordinates, two envelope passes and a column digest; it grows with B execution bars and column rows. The whole attestation is repeated per candidate. Auditor verdict: undocumented-scan. Documented: partially. The door's own doc (2045-2046) says "A caller with many runs over one slice should attest once". Nothing in docs/06-limits.md states that production producers re-attest per candidate.

Evidence:

- *The combined door attests every time it is called.** exit_grid_policy.rs:2064-2065 reads `let attested = self.attest_training(series, column, horizon)?; self.evaluate_with_attested(&attested, run)`. That calls resolved_grid_view.rs:143-173, which does this work on each call: `validate_execution_bars(bars)` (exit_grid_policy.rs:3113), a walk over every bar checking timestamps and each candle;
- exit_grid_policy.rs:2064 does `let attested = self.attest_training(series, column, horizon)?;` before evaluate_with_attested. resolved_grid_view.rs:143-173 then runs these passes, all linear in bars or column rows: validate_execution_bars(bars): one pass (exit_grid_policy.rs:3113) candidate_universe.rs:4557 is in replay_execution_side.

### W3-runner2-3 (medium, cost) crates/runner/src/exit_grid_policy.rs:269

ExecutionRunV1::new / new_with_daily_reference / seal (and ExpressionExecutionRunV1::new*, expression_execution.rs:38-70) (crates/runner/src/exit_grid_policy.rs:269): per minting the per-candidate run capability required by every per-run door, the cost is O(S + 2E), or O(S + M + D + 2E) plus an O(M) search, per candidate; it grows with signal bars S + execution bars E (hashed twice). With the daily reference, add the minute context M, daily bars D and the eligibility and excluded-day lists. Auditor verdict: undocumented-scan. Documented: no statement in docs/06-limits.md. 6864 says "Run identity hashing must read every input byte", which is about computing a digest, not recomputing an identical one per candidate.

Evidence:

- At 274-277 it calls `crate::identity::data_digest_with_execution(signal_bars, execution_bars)`. That function (identity.rs:384-412) runs `data_digest` over the signal slice and over the execution slice. Each `data_digest` makes 7 `hasher.update` calls per bar (identity.rs:334-350). `new_with_daily_reference` (310-317) is more expensive:
- `ExecutionRunV1::new` (exit_grid_policy.rs:264-273) calls `data_digest_with_execution(signal_bars, execution_bars)`, which hashes the signal bars S once and the execution bars E once (identity.rs:410-411, `term(... `seal` (exit_grid_policy.rs:347) then runs `execution_digest: crate::identity::data_digest(exact_execution)`, which hashes E again.

### W3-runner2-1 (medium, cost) crates/runner/src/expression_execution.rs:437

materialize_expression_coordinate / EvaluatedExpressionOosV1::materialize (expression_oos.rs:142) / BoundFixedTrainingFoldsV1::materialize_coordinate (expression_validation.rs:213) (crates/runner/src/expression_execution.rs:437): per one exit coordinate (cell) of one program's complete grid, materialised to trade rows, the cost is O(B + rows + C.H.T + trades) per coordinate. The achievable cost is O(C + trades_of_cell) after one shared O(B + rows + C.H.T) pass; it grows with B execution bars + column rows + C candidate paths x hold x armed rungs, paid again for every coordinate. G coordinates per program (about 625 at 4/4/4 rungs, read off checked_policy_cell_count) gives O(G x (B + rows + C.H.T)) per program. Auditor verdict: undocumented-scan. Documented: no. docs/06-limits.md §111 (6330-6336) covers materialising ONE selected cell. bind()'s doc (expression_validation.rs:125-128) says the later run is indexed "once, before materializing its coordinates", which implies per ...

Evidence:

- grid.rs:2821 `materialize_expression_cell` has no cache and does three things before the cell is used: grid.rs:2830 `let facts = crate::trade::SliceFacts::of(bars, column);`. Per trade.rs:407-462 this is O(B): a `HashMap::with_capacity(bars.len())` filled per accepted bar, two prefix vectors, `forced_exits_with_step`, and possibly `median_step_micros_over`.
- grid.rs:2830-2838 (`materialize_expression_cell`) starts with `let facts = crate::trade::SliceFacts::of(bars, column);` and then `walk_expression_over(bars, column, expression, horizon, direction_of(side), &facts)`. `levelled_timed` (grid.rs:3540-3596) then builds every candidate path's `cross: crossings_checked(bars, path.entry_bar, path.exit_bar, entry_price, side, ladders, facts.acceptance())`.

### W3-runner2-4 (medium, cost) crates/runner/src/expression_oos.rs:167

ResearchResolvedExitGridV1::evaluate_expression_oos + FixedTrainingFoldPlanV1::bind (expression_validation.rs:131) (crates/runner/src/expression_oos.rs:167): per one program x side (training anchor) replayed on the shared later series, the cost is O(B) re-attestation, a B-length fold map and SliceFacts per program; it grows with B later bars and column rows, repeated per program even though every check and the fold mapping are functions of (resolution, series, column, windows) only. Auditor verdict: undocumented-scan. Documented: no. bind's doc claims "O(bars + folds) work" for "the complete later run once", but there is one later run per program. No docs/06-limits.md statement.

Evidence:

- *The loop repeats the same whole-series work for every program and side.** `crates/cli/src/boolean_oos_v1.rs:425` runs `for (group, anchor) in training.anchors.iter().enumerate()`, with 2 × P anchors. The `series`, `column` and `validation.windows` passed in are the same objects on every pass, built once at 414-423. 188 `validate_execution_bars(bars)` walks all B bars (`exit_grid_policy.rs:3113-3151`).
- The caller builds the column once, `let (column, sessions) = prepare_column(request, source)?;` at cli/src/boolean_oos_v1.rs:414, and the series once, `ExecutionSeriesV1::new(...)` at :416-423, both outside the loop. It then loops `for (group, anchor) in training.anchors.iter().enumerate()` at :425, which is two anchors per program (`group / 2`, `group % 2`).


## runner-03

### AC-whp-tb-2 (medium, bug) crates/runner/src/rank.rs:212

The operator's rule min(win) >= 3x max(loss) is ranked only by `Lens::Asymmetry`, and no production code builds one. The only non-test mention in cli is the identity match arm `runner::rank::Lens::Asymmetry => 3`. D-0593 calls it 'Not the default', but no command can choose it.

It is also unguarded by tests:
- **Accumulator arm:** every lens loop in the tests leaves Asymmetry out — rank.rs:977, runner lib.rs:1445 and 1696 (the last loops Detectability, Payoff, Path). The arm at rank.rs:223/295/314 never runs.
- **Tie-break (AS-04):** the only test compares two edges whose primary keys differ (300 vs 12), so the `max_win_paisa` tie-break it calls 'load-bearing' is never reached. AS-04's proof cell is the code itself, not a test.
- **Identity term 3 (AS-06):** the cited test `every_knob_that_moves_the_answer_moves_the_identity` compares only Detectability with Payoff. Mapping Asymmetry to 1 or 2 would collide two runs' identities (§3 rule 3) and nothing would fail.

Evidence:

- The production ones are fixed constants: cli lib.rs:1351, 5959 and 15518 are Detectability; lib.rs:6668, 13810, 14300 and audited_range_command.rs:206 are Payoff. The only non-test mentions are the identity arm (fix/c2-final:crates/cli/src/lib.rs:10140 `runner::rank::Lens::Asymmetry => 3,`) and the report label (runner/src/report.rs:573). `worst_reward_risk_bp` and `max_win_paisa` are only read inside `ByAsymmetry::cmp` (rank.rs:553-561), so the operator's min-win / max-loss rule never reaches ...
- `git grep 'Lens::'` over crates on fix/c2-final finds production lenses only at cli lib.rs:5959 (Detectability), lib.rs:6668 (Payoff), audited_range_command.rs:206 (Payoff) and and_checkpoint.rs:56 (Detectability). `Lens::Asymmetry` appears in cli only at lib.rs:10140, the identity match arm `runner::rank::Lens::Asymmetry => 3`. (2) The Accumulator arms rank.rs:223, 295 and 314 never run in a test.

### W3-runner4-1 (medium, cost) crates/runner/src/resolved_grid_view.rs:143

ResolvedGridViewV1::attest_training (legacy via exit_grid_policy.rs:1986, research via resolved_grid_view.rs:93) (crates/runner/src/resolved_grid_view.rs:143): per one candidate/program x side evaluation, because evaluate_training_grid_attested re-attests on every call and production callers use it per candidate, the cost is O(B) per candidate: identity::data_digest(bars) (blake3 over every bar, line 155), validate_execution_bars (152), require_complete_acceptance (counts every verdict, exit_grid_policy.rs:3671), validate_column_sources (163), validate_arithmetic_envelope_view (16 ...; it grows with B training bars (plus ladder size L for the three require_runtime_integrity digest recomputations). Auditor verdict: undocumented-scan. Documented: Only as intent. resolved_grid_view.rs:82 says "Attest exact source, column, horizon and full arithmetic envelope once".

Evidence:

- resolved_grid_view.rs:143-173 calls require_runtime_integrity (149), validate_execution_bars(bars) (152) and `crate::identity::data_digest(bars) != self.training_digest` (155). Each of these walks every bar or every column row: exit_grid_policy.rs:3113, 3661-3687, 3689-3719, 3729ff and 3657.
- resolved_grid_view.rs:143-172 attest_training is O(B). It then runs require_complete_acceptance (exit_grid_policy.rs:3661-3687, which does `accepted.iter().filter(..).count()`), validate_column_sources (3689, a loop over every source), validate_arithmetic_envelope_view (3729, two max/min passes over bars) and digest_column (3657).

### ET-strategies-trades-ranking-costs-7 (low, bug) crates/runner/Cargo.toml:39

Stale documentation. Two places say trades price through worst_case_fills, which nothing calls. §72 says grid::peak runs per cell, but it now runs once per trade. The cli cost prose uses WALK_FORWARD_SPLITS, but the fold count is derived from the bar count.

Evidence:

- crates/runner/Cargo.toml:40-43 says `crate::trade` prices every entry and exit through `costs::fill::worst_case_fills`. crates/runner/src/trade.rs:310 says each signal does "two `worst_case_fills`". round_trip builds two FillBars (trade.rs:1125-1126) and calls `fills_at` with `Anchor::Open` (:1127) and `Anchor::PrintedExtreme` (:1128-1132).
- crates/runner/Cargo.toml:40-43 says "`crate::trade` prices every entry and exit through `costs::fill::worst_case_fills`". crates/runner/src/trade.rs:309-311 says "two `costs::fill::Bar` constructions and two `worst_case_fills`". `round_trip` (trade.rs:1125-1134) builds `FillBar::new` twice and calls `fills_at(.., Anchor::Open)` and `fills_at(.., Anchor::PrintedExtreme)`.

### ET-strategies-trades-ranking-costs-6 (low, bug) crates/runner/src/bootstrap.rs:199

The miscalibration note prints the bucket label as the sample size, so a 40-session family reads 'sample: 30 periods'. The 13.3% it quotes is the measured rate for the 30..=99 bucket, not a rate for this sample.

Evidence:

- crates/runner/src/bootstrap.rs:193-204. crates/runner/src/audit.rs:1227 prints `"  sample: {}"` followed by that string. `bootstrap_family` (cli/src/lib.rs:17065) sizes each series to `session_index(bars).len()`, and `session_index` (lib.rs:5435) produces one entry per distinct IST day. bootstrap.rs:189-192 says a sample between two rows "takes the WORSE of them".
- Code: crates/runner/src/bootstrap.rs:193-203 `calibration()` returns fixed band strings. crates/runner/src/audit.rs:1227 renders this as `writeln!(out, "  sample: {}", v.calibration())`. Nowhere in `audit::bootstrap` (audit.rs:1176-1236) is `v.periods` printed. In the cli, each series has days.len() entries, from session_index (cli/src/lib.rs:5435-5456) and bootstrap_family (lib.rs:17084, 17126).


## runner-05

### W3-runner1-3 (low, bug) crates/runner/src/closed.rs:153

closed / redundant_count (crates/runner/src/closed.rs:153): On a halted sweep the partial last level is treated as complete, so a set whose equal-support superset was cut off by the budget is reported as closed. Code path: closed.rs:125-127 and 153-155 zip every adjacent pair of `sweep.levels` and never read `sweep.halted`. engine/src/lib.rs:1545-1552 retires a halted successor with `None` because "A successor stopped by a budget is PARTIAL, so it cannot prove closure". The Retaining sink still pushes that partial level (engine lib.rs:867-868 `self.levels.push(level)`, and 1577 `sink.retire(current, None)` after `break`).

Evidence:

- (1) The loops at closed.rs:125-127 and 153-155 zip each adjacent pair of `sweep.levels`. (2) When the engine halts, it retires level k with `None` at engine lib.rs:1548-1552. `Retaining::retire` (867-868) is `self.levels.push(level)`, so the partial level k+1 becomes the last entry in `Sweep::levels`. (3) next_level (1820-1990) stops the join partway with `break 'join` and drains only the candidates already batched.
- closed.rs:121-127 (`redundant_count`) and closed.rs:152-154 (`closed`) pair each level with the next using `sweep.levels.iter().zip(sweep.levels.iter().skip(1))`. In engine/src/lib.rs `continue_walk`, a budget halt calls `sink.retire(current, None)` at 1545-1552 and then `break`s at 1573-1575. `Retaining::retire` (867-868) ignores `next` and does `self.levels.push(level)`.

### AC-whp-cx-2 (low, bug) crates/runner/src/closed.rs:37

The closure proof in closed.rs relies on the sentence 'that superset is always PRESENT to be seen: if sup(Y) = sup(X) and X cleared min_hits, then Y cleared it too, so the sweep kept Y'. That stopped being true when the engine added the meaning prune. `try_next_level_observing` refuses any candidate whose new pair is non-informative (engine lib.rs:1941), and by the induction its comment states, no itemset containing an implied pair is ever enumerated. Take X that contains a pivot rung bit p, and let y be a rung that p exactly implies (vocab::implication::implies(p, y), e.g. close_above_pivot_s2_band implies close_above_pivot_s3_band). Then Y = X ∪ {y} has exactly sup(X) on every bar, and Y is never in any frontier. redundant_between therefore marks X `ClosureVerdict::Closed`, whose doc says 'No immediate superset has equal support', while such a superset exists. vocab/src/implication.rs:30-34 still says `runner::closed` 'deletes {above_s2} and KEEPS {above_s2, above_s3, above_s4, above_s5} — the maximal form'. With the prune the opposite now happens: the minimal form is kept and the maximal one is never built. The numbers are unaffected (Y was never a counted trial), but three docu

Evidence:

- At fix/c2-final:crates/engine/src/lib.rs:1941, `if !parents_informative(a, b) { pruned = pruned.saturating_add(1); continue; }` runs after the subset prune and before `batch.push(cand)`. At lib.rs:2031-2039, `parents_informative` calls `vocab::implication::pair_is_informative`. At implication.rs:191-192, that returns `!implies(a, b) && !implies(b, a) && compatible(a, b)`.
- The ladder loop in engine continue_walk calls `self.next_level(column, &current, k, ...)` (engine/src/lib.rs:1542). parents_informative (lib.rs:2031-2039) calls `vocab::implication::pair_is_informative` on the two parents' highest bits. implication.rs:150-163 has `implies(a,b)` true for (above rung a, above rung b) when rank_a > rank_b.

### W3-runner2-5 (low, cost) crates/runner/src/exit_grid_policy.rs:416

require_exact_execution_subslice (via ExecutionRunV1::new_with_daily_reference) (crates/runner/src/exit_grid_policy.rs:416): per one candidate's run-capability mint (per closed mask x side, per Boolean program), the cost is O(M + E) per candidate, including a membership search; it grows with M = minute-context length (linear search) plus E (slice compare). Auditor verdict: undocumented-scan. Documented: no. docs/07-o1-architecture.md layer 4 (line 42) says "No search of any kind". CI gate 11 (ci.yml:6323) greps only `binary_search|partition_point`, so a linear `position` is invisible to it.

Evidence:

- crates/runner/src/exit_grid_policy.rs:423 does `reference_minute_context.iter().position(|bar| bar == first)`. It runs once per closed mask and side: in `expand_population_side` (cli/candidate_universe.rs:4785), in `replay_execution_side` (4548, which `build_execution_v3_replay_authority` loops over at 4353/4368), and once per program and side in boolean_candidate_v1.rs:746.
- crates/runner/src/exit_grid_policy.rs:423 does a linear search: `let Some(start) = reference_minute_context.iter().position(|bar| bar == first) else { return Err(ExitGridErrorV1::EvaluatedExecutionOutsideReferenceContext); };`. It runs once per candidate through `ExecutionRunV1::new_with_daily_reference` (exit_grid_policy.rs:316): candidate_universe.rs:4548: per mask group and side, driven by the loop at 4353/4368.

### W3-runner2-8 (low, bug) crates/runner/src/exit_grid_policy.rs:3618

column_digest_v1 (crates/runner/src/exit_grid_policy.rs:3618): identity does not move when the column's three-valued evidence (`known`) changes Code path: The digest hashes bits, sources, sourced, census, first_swept, collided, the spec fingerprint, acceptance and acceptance_census. It never reads `column.known()`, yet the doc says "Stable V1 digest of every durable field". expression.rs:77-83 feeds `column.known()` into `expression.evaluate(*truth, *known)`. The public `Column::clear_before` (indicators/src/column.rs:695-707) sets `*known = ConditionMask::ZERO`.

Evidence:

- crates/runner/src/exit_grid_policy.rs:3618-3655 `column_digest_v1` hashes only these fields: `bits()`, `sources()`, `sourced()`, `census()`, `first_swept()`, `collided()`, the spec fingerprint, `acceptance()` and `acceptance_census()`. `Column` has a `known: Vec<ConditionMask>` field (indicators/src/column.rs:344), and `PartialEq` is derived, so two columns can compare unequal yet produce the same digest.
- `column_digest_v1` (crates/runner/src/exit_grid_policy.rs:3618-3655) hashes `bits()`, `sources()`, `sourced()`, `census()`, `first_swept()`, `collided()`, the spec fingerprint, `acceptance()` and `acceptance_census()`. `known` is the only `Column` field it leaves out (indicators/src/column.rs:342-393).


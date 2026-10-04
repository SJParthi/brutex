# Crash and edge-input audit, pass 10: counts that size allocations and loops

Head: `1f4de71` (origin/final/all-fixes-zero, read in /home/claude/wt/zero3). Audit only. Nothing in any checkout was edited.

**Verdict: 2 new findings at 1f4de71 (0 high, 1 medium, 1 low).** The theme is CE-61's class: a count read from a file, request, env var or vendor payload that sizes an allocation or a loop. Across cli, api, lake, telemetry, store and pull, every count read from a ledger, checkpoint or cursor that sizes an allocation is now bounded in one of four ways:

- by an explicit bound (`bounds.*`, `max_bytes`, `MAX_*`) before `try_reserve`/`try_reserve_exact`;
- by the file's own length, with no more than about 1x memory amplification;
- by a page or limit cap on query parameters;
- by a design ceiling (manifest `MAX_ENTRIES`).

The only unbounded count-to-allocation route in the class is still CE-61's (store `n_valid` checked against file length rather than the month). Its extra sinks are listed below so that the fix covers them all.

The new defects are in the neighbouring class, "a byte count read from metadata sizes a read". Two by-path readers still trust `metadata().len()`: one reads the vendor census (the manifest) and one reads a vendor master. For a FIFO, a device or `/proc` that length is 0. P-19 / D-1502 / R9-api-cx-1 fixed this exact pattern in `pull::config`, `api::census::sized` and `api::indexmap`. The api's own reader of the **same manifest file** refuses a FIFO by name. The pull ingest reader of that file does not.

Method: grep across the workspace for `with_capacity`, `reserve`, `vec![x; n]`, `resize`, `read_exact` and `fs::read`/`read_to_end`/`read_to_string`, and for `for _ in 0..n` and `while i < n` loops. Test-only sites were dropped, and each remaining site was traced to its source of n. Cargo was used once, for CE-64, in a scratch worktree (since removed).

## Table: site | source of n | bound | worst cost | verdict

| Site | Source of n | Bound | Worst cost | Verdict |
|---|---|---|---|---|
| api/src/bars.rs:967 `with_capacity(total)` | sum of store `n_valid` | file length only | abort (handle_alloc_error) | **CE-61 (known)** |
| api/src/server.rs:13275 `read_month_bars` | `n_valid` | file length only | abort | **CE-61 (known)** |
| api/src/server.rs:3194 `bars::page(&file,0,n)` | `n_valid` | file length only | unbounded fault-String growth | **CE-61 (known)** |
| api/src/calendar_of.rs:593, :646 `while index < file.records()` | `n_valid` | file length only (sparse) | ~2e10 `read_record` calls, a CPU hang, no allocation (one BTreeMap entry per day) | CE-61 sink not listed in CE-61. Covered by its root fix. No new ID |
| pull/src/ingest.rs:2037, :2554 `0..n_valid` | `n_valid` | file length only | CPU hang during derive and history checks | same: CE-61 root |
| cli/src/fold_audit.rs:422, stored.rs:2390 (`decode_loaded`), vix_reference.rs:128 | `records()` | `try_reserve` then loop. `stored` also has the `remaining_records` span ceiling | named refusal when the reservation exceeds commit. Otherwise a sparse-file read loop | OK (`try_reserve`). Loop cost is CE-61 root |
| cli/src/trades.rs:1076, :1266 | index block span / `(len-HEADER)/STRIDE` | file length | about 1x file (HashMap len/136/64) | OK. Sparse-file caveat is CE-61's threat class |
| cli/src/frontier.rs:1581 `vec![0;width]`, :1609, :1714 | block span of indexed rows | file length (rows actually indexed) | ≤ file bytes | OK |
| cli/src/results.rs:1023 HashMap `records` | `(len-HEADER)/stride` | file length | < 1x | OK |
| api/src/backtest.rs:1155 | `limit.min(MAX_RUNS)` | MAX_RUNS | constant | OK |
| api/src/audit.rs:1297 `vec![0;span]` | `wanted` ≤ MAX_PAGE_RECORDS | 51,200 B | constant | OK |
| api/src/recovery_journal.rs:572 / :665 | `wanted` (page) / `whole_records(len)` | page cap / file length | `try_reserve`. Replay holds an index per record | OK |
| api/src/bars.rs:441, :783, :893; census.rs:750; indexstoprankingjson.rs:343; render.rs pages | query `page`/`limit`/months | PAGE_BARS, MAX_WINDOW_MONTHS, `try_reserve_exact`, `end.min(total)` | constant | OK |
| api/src/server.rs:16342, :16455 HashSet from `&` count | query string / form body | request target cap / body cap | ≤ request bytes | OK |
| api/src/master.rs:230, :376 | header comma count | MAX_ROW_BYTES row cap after CE-1 | ≤ header | OK |
| api/src/master.rs:332-341 `metadata(path).len()` then `read_to_string(path)` | metadata length | **none for a non-regular file**. TOCTOU for growth | FIFO hang. `/dev/zero` read until OOM | **CE-65** |
| pull/src/ingest.rs:2909-2925 `read_census` | `fs::metadata(path).len()` | **none for a non-regular file**. TOCTOU for growth | FIFO: pull hangs for ever holding CensusLock. `/dev/zero`: Vec doubles to RAM size, then "out of memory" or the OOM killer | **CE-64** |
| cli/src/selection_v6.rs:400 `fs::read(&aside)` | whatever the `.abandoned-<n>` file holds | none (semantic bound is < 16,384 B, `tail_len`) | unbounded / FIFO hang | part of **CE-65** |
| pull/src/masters.rs:1149 `read_to_string(&target)` | whatever the target holds | none (write side caps the body) | unbounded / FIFO hang | part of **CE-65** |
| cli/src/selection_v6.rs:388 `vec![0; tail_len]` | `len % BLOCK_BYTES` | < 16,384 | constant | OK |
| cli/src/search_checkpoint.rs:572, boolean_candidate_persistence.rs:315, candidate_trades.rs:1378 | header length / file length | `max_bytes` + exact-length equality, `try_reserve_exact` | ≤ admission | OK |
| cli/src/boolean_candidate_v1.rs:662/967, population_statistics_v2.rs:2479/2859, v3:2618/2710 | ledger counts | `bounds.candidates_per_audit`, record existence, `try_reserve*` | ≤ admission | OK. v3:2710 `vec![false; n]` is reached only after n candidates were read and digest-checked |
| cli/src/index_stop_*_codec.rs, boolean_candidate_reader.rs, and_checkpoint.rs, index_consistency_store.rs, boolean_qualification_wire.rs | in-body counts | `count > remaining()/STRIDE`, `max_records`, `try_reserve_exact`, pre-pass `admit` | ≤ body | OK |
| cli/src/selection*.rs `Indexes::with_capacity(count)`, execution_v3/v4 | `receipt_capacity(len, max_receipts)` | max_receipts + `try_reserve` | ≤ admission | OK |
| cli/src/live.rs:763 | header `count` | `count.min(available)`. `row_limit` exact length | file length | OK |
| cli/src/population_observations_v1.rs:2778, :3727 | file length | `bounds.max_file_bytes` + `try_reserve_exact` | ≤ admission | OK |
| cli/src/ledger_all.rs:259 `BRUTEX_GRID_RUNGS` | env var | `machine_count(raw, rungs_within_cell_budget())` + `try_reserve_exact` | ≤ cell budget | OK |
| api boolean_observation_budget / boolean_search_budget, cli strict_range_config | env vars | canonical positive integer, aggregate ≤ isize::MAX | operator ceiling | OK |
| lake/src/reader.rs:560/630/753, page.rs:274 | footer `num_rows`, page `expect` | `rows ≤ bytes.len()`, MAX_PAGE_BYTES | about 10x file per column (stated in source). No production caller of `lake` | OK (documented) |
| telemetry/src/tail.rs:323/589, sink.rs:1485 | block size | READ_BLOCK, `max_scan_bytes` budget, 64 KiB | constant | OK |
| store/src/file.rs:1477, :2050, :3277 | len ≤ REGION_LEN, covered block range | REGION_LEN / one block | constant | OK |
| pull/src/manifest.rs:2482-2483 | census `n_valid` | `reservation_for` ≤ MAX_ENTRIES (2,097,152) | design ceiling | OK |
| pull/src/archive.rs:865, cash_session_cache.rs:423/466, config.rs:1029; api census.rs:444, indexmap.rs:58; cli research_policy.rs:41, boolean_catalog_command.rs:116, search_checkpoint.rs:390 | file length | `is_file`/size check + `take(cap+1)` | ≤ cap | OK. These are the pattern CE-64 and CE-65 should match |
| pull/src/http.rs:1919, :2106; fetch.rs:511 | vendor payload row count | decoded column lengths (body cap), MAX_ROWS | ≤ body | OK |
| cli/src/boolean_search_command.rs:232, :415, :422 `0..completed_batches()` reverify | checkpoint history | `spec.records` | O(B) campaign and rung reopens per new batch, so O(B²) per search | Not a crash. A deliberate re-check. Noted as an O(1)-law observation, not a finding |

## New findings

### CE-64 (medium): pull ingest reads the vendor census by path with a metadata length that is 0 for a FIFO or device. A FIFO at the manifest path hangs the pull for ever while it holds the census lock.

- **Where:** `crates/pull/src/ingest.rs:2909-2925` (`read_census`), called at :810 and :1208 after `CensusLock::take`.
  ```rust
  let bytes = match fs::metadata(path) {
      Ok(found) if beyond_ceiling(found.len()) => { return Err(...) }
      Ok(_) => {
          fs::read(path).map_err(|why| format!("{} could not be read: {why}", path.display()))?
      }
  ```
- **Why it is wrong:**
  - `fs::metadata` follows symlinks and reports `len() == 0` for a FIFO, a character device or `/proc`. So the `MAX_CENSUS_BYTES` ceiling passes, and `fs::read` then either opens a FIFO, which blocks in `open(2)` with no writer, or reads an endless device.
  - It is also a stat-then-read-by-path TOCTOU: a file swapped or grown between the two calls is read in full.
  - The api reads the **same file** (`pull::manifest::manifest_path`) through `api::census::sized` (census.rs:419-449). That reader refuses a non-regular file by name ("this reader neither waits on a FIFO nor reads a device", R9-api-cx-1) and reads through `take(MAX_MANIFEST_BYTES+1)`.
  - Invariant P-19 states the rule ("A path that is not a regular file is refused by name before it is opened…"), and D-1502 / D-1362 applied it to every other bounded reader. The writer-side reader of the census never got it.
  - Pull legs run inside the api process. A hung `read_census` holds `CensusLock` and the vendor's seat for ever, so every later pull of that vendor is refused as busy, with no refusal naming the cause. That breaks CLAUDE.md §4: degrade loudly, never hang silently.
- **Repro (ran)** in a scratch worktree at 1f4de71, with a throwaway `#[cfg(test)]` module in `pull::ingest`:
  - **FIFO:** `mkfifo <tmp>/groww.man`, then `read_census(&p, Vendor::Groww)` on a thread. After 3 s, `recv_timeout` gave `Err(Timeout)`, so the call was still blocked. The test then opened the write end to release it.
  - **Device:** a symlink to `/dev/zero`, run under `ulimit -v 2 GiB`. `fs::metadata(..).len()` printed `0`, and the call returned `Err("…/groww.man could not be read: out of memory")` after filling the address-space cap.
    - Without the cap, on this 15 GB no-swap box, the Vec doubles through about 8 GB of resident zeros before a reservation fails, unless the OOM killer takes the process (the api) first. Either way it is a multi-GB spike instead of a named refusal.
  - Worktree and target directory removed afterwards.
- **Minimal fix:** reuse the P-19 shape.
  - `let file = open_nonblocking_nofollow(path)` (or share `api::census::open_without_waiting`'s logic by moving it into `pull`), then `meta = file.metadata()`, then `!meta.is_file()` → `Err("… is not a regular file (a FIFO/device); refused unread")`.
  - Then `file.take(MAX_CENSUS_BYTES + 1).read_to_end(..)` and refuse on the bytes actually read.
  - Keep `NotFound` → genesis.
  - Add an invariant row beside P-19 and a FIFO test with a deadline, like the api's `a_manifest_path_that_is_not_a_regular_file_is_refused_unread_and_never_waits`.

### CE-65 (low): three more by-path whole-file reads with no regular-file check and no read cap

- **Where:**
  1. `crates/api/src/master.rs:332-341` (`master::load`): `let size = std::fs::metadata(path)…len(); if size > MAX_MASTER_BYTES {…} let text = std::fs::read_to_string(path)…`. Same shape as CE-64: a FIFO or device at the master path passes the 256 MiB check with size 0. Reached from `server.rs:664` and `emitted.rs:359`.
  2. `crates/cli/src/selection_v6.rs:400`: `let held = std::fs::read(&aside)` for an existing `selection-v6….abandoned-<committed>` quarantine. Nothing checks its type or size, though the only legal content is `< SELECTION_V6_BLOCK_BYTES` (16,384) bytes, the `tail_len` it is compared with.
  3. `crates/pull/src/masters.rs:1149`: `std::fs::read_to_string(&target).map_or(true, |held| held != body)`. This reads the whole current master to compare with the new body, with no type check or bound.
- **Why it is wrong:** each is a size taken from a file (or no size at all) that decides how much is read. A FIFO hangs the caller, and a device or oversized file is read until memory runs out. P-19 / D-1502 require a regular-file check and `take(cap+1)`, and the siblings comply (census.rs:444, indexmap.rs:58, archive.rs:865, config.rs:1029, cash_session_cache.rs:423). The threat is a local actor with write access to the masters or ledger directory, the same model as CE-61 and R9-api-cx-1. Hence low.
- **Repro (not run):**
  - Site 1 follows the same `std::fs` path as the CE-64 device repro: `metadata` gives 0, and `read_to_string` is unbounded on `/dev/zero`, where NUL is valid UTF-8, so it grows until "out of memory".
  - Sites 2 and 3: `mkfifo` at the quarantine or target name makes `open` block.
- **Minimal fix:** open once and `fstat` that handle. Refuse `!is_file()` by name. Read with `take(cap + 1)`, where cap is `MAX_MASTER_BYTES`, `SELECTION_V6_BLOCK_BYTES` or the masters body cap. A mismatched size on the quarantine is already "different bytes", so it can be refused without reading.

## Not re-reported

CE-1..CE-63 and P1-19-03 (`pull/src/ssm.rs:397` bare `read_to_string`, still not fixed at this head). The `n_valid` sinks in the table above all close with CE-61's proposed `CounterExceedsMonth` refusal in `BarFile::validated`. If that fix lands only at the three sinks CE-61 lists, calendar_of.rs:593/646 and pull/src/ingest.rs:2037/2554 keep the unbounded read loop.

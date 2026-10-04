# Numeric pass 12 (num12): what each operation outside the sweep engine costs

Head: `1f4de71` (origin/final/all-fixes-zero), read-only checkout `/home/claude/wt/zero3`. Audit only. Nothing was edited and no cargo was run, because the one finding is low severity. Every cost class below comes from reading the source.

Classes used in this report:
- **(a)** O(1), or O(page) / O(batch), and correct.
- **(b)** Not O(1), but `docs/06-limits.md` states it with a reason.
- **(c)** Not O(1) and not stated anywhere. Only (c) counts as a finding.

Pass 7 already covered the engine's own primitives, so they are not repeated here.

**Counts: 1 new finding (low). 0 high, 0 medium. Verified: 2 known items (1 fixed as documentation, 1 not fixed).**

## New finding

### p12num-1 (low): every recorded run reopens and re-indexes `frontier.bin` and `detail-sets.bin`; the frontier rustdoc says "once per process", and docs/06 states only the trades copy of this cost

**Where**
- `crates/cli/src/lib.rs:17371`: `let mut store = frontier::Frontier::open(root)?;` sits inside `ensure_frontier_rows`.
- `ensure_frontier_rows` is called by `record_frontier` at `lib.rs:17620`.
- `record_frontier` is called once per recorded run, at `lib.rs:19514` (`record_unadmitted`) and `lib.rs:20712`.
- If the append is refused, the file is reopened a second time at `lib.rs:17385`.
- `crates/cli/src/lib.rs:17652`: `let state = result_set::Receipts::open(root)?.append_exact(receipt)?;` sits inside `ensure_detail_receipt`. It is called once per run, at `lib.rs:19515` and `lib.rs:20744`.

**What each open costs**
- `Frontier::open` (`frontier.rs:902-931`) takes the exclusive `file.lock()` and runs `index_locked`. That calls `index_of` (`frontier.rs:1711-1741`), which reads every 280-byte row ever written and seal-checks each one with `blake3`: `for index in 0..count { buffered.read_exact(&mut raw)…; index_row(…) }`.
- `Receipts::open` (`result_set.rs:312-344`) calls `from_file` (`result_set.rs:350-380`): `while at.saturating_add(STRIDE) <= len { let record = read_at(&mut file, &path, at)?; … }`. That is one unbuffered seek plus one read per 64-byte receipt, and each receipt is seal-checked (`read_at`, `result_set.rs:1107-1117`).

**Why it is wrong**
- `crates/cli/src/frontier.rs:69-71` says: "Opening reads the whole file once, O(rows) … and it happens once per process rather than once per question." The code opens it once per recorded run. A sweep that records N runs therefore pays Θ(N·F_total) row reads and hashes in total, not O(F_total). Part of that time is spent holding the exclusive lock, so `ordered` lanes recording at the same time wait on each other.
- The identical defect for `chosen-trades.bin` (W2-cli16-2) was fixed as documentation by D-1634 (`docs/06-limits.md:14379-14384`). Its two siblings in the same three-file commit were left out:
  - `docs/06-limits.md` §116 (:6725) and D-1560 (:12541) state the cost of one cold open, O(F) or O(R).
  - Neither says the open happens on every recorded run.
  - `results.rs` avoids this cost through `with_shared_writer` (`results.rs:788-815`), which shows the per-run reopen is not inherent.

**Size extrapolation (labelled as an extrapolation, not measured)**
- Assume 25 frontier rows per run (the figure in `frontier.rs:64`) and one 64-byte receipt per run.
- Run k reads 7,000·k frontier bytes, seal-hashes 25·k rows, and makes k receipt reads.
- Over N runs that totals about 3,500·N² bytes and 12.5·N² row hashes, plus N²/2 receipt seek+read pairs.
- At N = 1,680 (one `sweep-all` pass over 210 instruments × 8 rungs): about 9.9 GB read and 35 M row hashes, plus 1.4 M receipt reads. This is on top of the trades file's own Θ(N·H).
- At N = 10,000: about 350 GB and 1.25 G hashes, plus 50 M receipt reads.

**Repro:** not run. It is a structural trace: `record_unadmitted` → `record_frontier` → `ensure_frontier_rows` → `Frontier::open` → `index_of`, all in the same call.

**Minimal fix:** either of these.
- Hold one `Frontier` writer and one `Receipts` writer per root, the way `results::with_shared_writer` does, and use `refresh` (already O(delta), D-0913 / §116) before each append.
- Or, at minimum, follow D-1634: correct `frontier.rs:69-71` to say "on every open; once per recorded run through `ensure_frontier_rows`", and add a docs/06 bullet next to W2-cli16-2 covering frontier.bin and detail-sets.bin.

## Per-operation table

### API routes

| Route / operation | Cost per request | file:line | Class |
|---|---|---|---|
| `/backtest/audit.json` page and exact read | O(limit) fixed-stride reads, limit ≤ `MAX_PAGE`. No directory scan. | cli/src/operation_audit.rs:584-669 | (a) |
| journal `begin` (every audited request) | O(1) append plus about 7 fsyncs. File count grows with history. | operation_audit.rs:540-576 | (b) D-1445, 06:11752 |
| `/audit.json`, `/audit/page` journal page | seek plus one read of ≤ `MAX_PAGE_RECORDS` | api/src/audit.rs:1254-1300 | (a) |
| `/audit.json` feed rollup | O(E_v) over the asked feed's manifest, cached per census snapshot | audit_json.rs:421-479 | (b) 06:10108 |
| `/logs`, `/logs.json` | backward tail capped by `max_scan_bytes`, which reports `hit_scan_cap`. Sort of ≤ 2·limit. | api/src/logs.rs:361-423, telemetry/src/tail.rs:29-35 | (a), capped |
| `/pull/recovery.json`, `/pull/recovery` page | three `tail` reads, each a seek to `count−wanted`, wanted ≤ 100 | api/src/recovery.rs:1640-1677, recovery_journal.rs:526-578 | (a) |
| recovery start (POST) | replays each journal three times | recovery_journal.rs:661 | (b) D-0907, 06:10198 |
| `/pull/run.json` | O(feeds), at most 5 | pullrun.rs:287-330 | (a) |
| `/autopilot.json`, `/ingest/status.json` | snapshot of in-memory status; failures capped by `MAX_FAILURES` | autopilot.rs:1754-1793, 2244-2253; ingest.rs:1968 | (a), 06 §48 |
| `/masters/status.json` | one `metadata` per `masters::SOURCES` entry | mastersrun.rs:716-722 | (a) |
| `/dashboard`, `/health`, `/universes.json`, `/feeds.json`, `/vocab.json` | precomputed counts, fixed vendor/rung/target loops, 370-row table | server.rs:1020-1036, 758-773, 1547-1571, 1800-1840, 34318-34342 | (a) |
| `/instruments` page | one slice plus ≤ `PAGE_ROWS` rows | catalog.rs:480-489 | (a), C-11 bench |
| `/instruments.json`, `/calendar.json`, `/store` filtered, `/verify.json`, `/bars.json` past end, `/indexmap.json`, `/universe/resolve` and pull POSTs (`spot_targets`) | O(U), O(E), O(log length) and similar | server.rs, census.rs | (b) W1-api5-1..11, 06:11690-11740; UC-19 06:14336 |
| census-backed routes on a miss | O(manifest bytes + E log E) | census.rs | (b) W1-api5-2 |
| `/backtest.json` | O(limit) seek+read rows, ≤ `MAX_RUNS` = 20,000 | api/src/backtest.rs:391, 1020-1280 | (a) bounded; module cost table |
| `/frontier.json`, `/trades.json` | cached parents plus O(delta) refresh; cold open O(rows) | frontierjson.rs:128-170, trades.rs:111-156 | (b) §116, §125, D-0913 06:10433 |
| Boolean / candidate / OOS / campaign / top JSON | cold O(saved bytes), warm O(page) | booleanjson.rs, candidatejson.rs, topjson.rs | (b) D-1444 06:11879-11938 |
| `/index-stop-ranking.json` unpinned | checkpoint directory walk on every request | indexstoprankingjson.rs | (b) D-0904 06:10169 |
| `/index-stop-qualification.json`, `/index-stop.json`, `/index-stop-vix.json`, `/index-stop-candles.json` | one slot. Cold: open plus rank sort O(R log R) under `BooleanObservationBudget`. Warm: `require_current` plus a ≤ 256 window. | indexstopqualificationjson.rs:141-236 | (b) §125 06:7025 (cold detail opens bounded and linear); `indexstopjson` named at 06:11902 |
| form parsing | k scans of a capped body | server.rs:791-798 | (b) D-1202 / D-1499 / D-1769 |

### Hot paths outside the API

| Path | Cost | file:line | Class |
|---|---|---|---|
| store `BarFile::append` | O(batch + log n): `first_at_or_after` bisection plus a per-row compare in `already_stored` / `diagnose_overlap` / `suffix_that_follows`; reseals only the touched blocks | store/src/file.rs:2233-2300, 2819-2930, 3123-3155 | (a), amortised O(1) per bar |
| store cold read | O(1) per block, measured | file.rs `verify_block_of` | (a) D-0914 C-BC-01 |
| lake reads | whole file read once, O(file); row-group reservations bounded by file length. No production consumer (no crate depends on `lake`). | lake/src/reader.rs:87-120, 221-300 | (a) per file; 06 §37 |
| pull ingest fold | per bar O(1). `derive_all` re-reads the committed month once per batch. | pull/src/ingest.rs:2209-2243 | (b) ET-bars-candles-store-1, 06:11579-11600 |
| pull `from_window` census read per body | O(manifest + E_v) | pull/src/ingest.rs | (b) W1-api5-1 |
| telemetry emit | one atomic floor check, target levels ≤ `MAX_TARGET_LEVELS`, one locked write | telemetry/src/sink.rs:1141-1167 | (a); per-request bound UNVERIFIED as timing, 06:14486 |
| `runs.bin` reuse lookup and append | cached shared writer, `refresh` O(delta), `holds` expected O(1) | cli/src/results.rs:788-815, 1130, 1222; lib.rs:17740 | (a) (D-1560 growth recheck stated) |
| `chosen-trades.bin` per run | O(H + T) open per recorded run | lib.rs:17421 | (b) D-1634 W2-cli16-2 |
| `frontier.bin`, `detail-sets.bin` per run | O(F_total), O(R_total) open per recorded run | lib.rs:17371, 17652 | **(c) p12num-1** |
| `latest_for` | O(runs) per rung | lib.rs:15959-15968 | (b) D-1567 06:12580 |
| `cli top` / `cli results` (`newest_complete`, `results_at`) | O(runs) per command, no longer on any HTTP route (`/engine/top.json` uses `topjson`) | lib.rs:8289-8305, 8432-8450 | (a)/(b), a one-shot listing |
| sweep evidence begin/level/read | fixed-stride appends; `last_event` at a fixed offset | cli/src/sweep_evidence.rs:458-575, 918-975 | (a), module doc |
| seen and dedup sets | `HashSet` / `HashMap` everywhere checked: frontier `seen` (lib.rs:17565), recovery `days`/`expected` (recovery.rs:460, 1273), `minute_gaps::withhold` (minute_gaps.rs:256), merge `asserted` / `raw_ambiguous` (merge.rs:403, 417), `fnowork::gaps` (fnowork.rs:579). Remaining `Vec::contains` uses run over at most 8 rungs or 5 vendors (sweeprun.rs:1162, constituents.rs:484). | — | (a) |
| web transforms | no O(n²) over a series found. `fill` is 12 × rows (trade-analytics.js:647-662); chart session cut is one `find` (markets/+page.svelte:1519); live-progress, frontier-analytics and comparison are single passes with `Set`/`Map`. Nested `find`/`includes`/`some` calls are over constant lists (RUNGS, UNIVERSES, feeds, ≤ 8 rungs) or one-off lookups. | web/src/lib/*.js, routes/*/+page.svelte | (a) |

## Verification of known items in this theme

| ID | Verdict | Evidence |
|---|---|---|
| W2-cli16-2 (Trades::open per recorded run) | FIXED (as documentation, D-1634) | docs/06-limits.md:14379-14384; the per-run open remains at lib.rs:17421. Its frontier and receipt siblings are p12num-1. |
| rep-1 (`find_offered_stream` linear rescan) | NOT FIXED (info, known) | cli/src/global_replay.rs:750-765, global_replay_v2.rs:3493-3508 |

Tally: 1 FIXED, 1 NOT FIXED.

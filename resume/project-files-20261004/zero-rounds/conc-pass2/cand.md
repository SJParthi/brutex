Verdict: 2 new findings at 331b05c in the candidate slice (0 high, 0 medium, 2 low). All 7 pass-1/workspace findings that touch the slice re-verify as CONFIRMED. None is refuted.

Slice: crates/cli/src/candidate_universe.rs, candidate_trades.rs + candidate_trades/codec.rs, boolean_candidate_{grid,persistence,reader,v1}.rs (plus boolean_observation_file.rs, which persistence includes with `#[path]`), crates/api/src/candidatejson.rs. Method: source reading only. No cargo was run.

---

## cand-1 (low): a foreign-identity row orphan wedges the shared Candidate Universe ledger, and after a rebuild nothing can complete it

New site of the pop2-4 class. pop2-4 lists only the Population Admission, Finalization and Statistics ledgers. Pass-1 search marked candidate_universe "checked clean" for rollback and ragged tails, but not for this.

- **Where:** `crates/cli/src/candidate_universe.rs:3700-3712` (`append_complete_locked`). Identity is set at `:5504-5519` (`derive_universe_id` hashes `identities`, including `source_commit_digest`, `:5548-5563`). Source commit is set at `:4272` (`source_commit_digest: hash(source.execution_series.commit().as_bytes())`).
- **Code:**
  ```rust
  let (first_row, prefix) = match self.orphan {
      Some(orphan) => {
          if orphan.universe_id != receipt.universe_id() {
              return Err(format!(
                  "candidate row tail belongs to {}, not requested {}; no fallback may hide it", ...
  ```
- **Why it is wrong:**
  - The rows are synced first (`:3747-3750`) and the receipt is written after them (`:3758-3761`).
  - A process death between those two leaves a whole-row orphan block for universe U. `scan_orphan` reconstructs it on every open.
  - After that, the only append the ledger accepts is the exact retry of U. U's id includes the build commit, the data digest, the requested span and every policy digest.
  - The Step 3 root is shared: `step3_orchestrator.rs:3135` passes `root.path()` for every family and span, and default bounds allow 8 universes (`:4734`). So the orphan blocks every other universe on that root too: the other family, other spans, and later runs.
  - Usually the fix for a crash is to rebuild. Once the build commit changes, the exact retry can never be built again, and the root refuses every later Candidate append with no recovery tool. A grep for quarantine or discard-orphan in the module finds nothing.
  - CU-02 (`docs/04-invariants.md:3634`) documents that a foreign orphan refuses. It does not say that the refusal is permanent across builds. docs/06-limits.md is silent on it.
- **Repro:**
  1. Run a Step 3 Candidate commit for family NIFTY on root R. SIGKILL the process after `self.row_file.sync_data()` at `:3747` returns and before `append_receipt` at `:3758`. The rows are durable and no receipt exists.
  2. Rebuild at a new commit, for example with the fix for whatever crashed. `commit_stamp` requires a clean build, so the commit differs.
  3. Rerun Step 3 on R. `produce_candidate_universe_v1` derives U' != U, because `source_commit_digest` changed.
  4. `append_complete_locked` refuses with "candidate row tail belongs to U, not requested U'".
  5. Every later run on R refuses the same way, for any family or span.
- **Minimal fix:** One option is to let the writer skip a fully valid, receipt-less foreign orphan, as Execution V1 does (EC-01). It starts the new block after the orphan and leaves the orphan as unreferenced evidence. `scan` would need to accept more than one receipt-less block, or use a skip marker. The other option is an operator verb that records the orphan and quarantines it. At minimum, put the orphan's source commit into the refusal text, so the operator knows which build can heal it.

---

## cand-2 (low): `/candidate-trades.json` waits on a blocking process-wide mutex while it holds a shared detail permit, so contention on trade pages returns 429 on unrelated detail routes

- **Where:** `crates/api/src/candidatejson.rs:302-328` (`trade_page`), reached inside `crate::detail::run` (`:146`). `detail::run` takes one of `MAX_CONCURRENT = 4` permits (`detail.rs:14`, `:105`). Those permits are shared by the routes in 23 api files.
- **Code:**
  ```rust
  static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
  let mut cached = CACHE.get_or_init(|| Mutex::new(None)).lock()   // blocking
      .map_err(|_| "candidate trade reader cache poisoned")?;
  if cached.as_ref().is_none_or(|held| ... || held.key != key) {
      let reader = TradeReader::open(root, summary, key, crate::detail::MAX_SCAN_BYTES)?;  // under the lock
  ```
- **Why it is wrong:**
  - `TradeReader::open` (`candidate_trades.rs:1047-1092`) runs while the global mutex is held. It reads, BLAKE3-hashes and seal-checks every trade row, up to 64 MiB. It also runs `pinned` twice, and the first `pinned` can be a cold catalog read.
  - There is one slot, so any change of key causes a cold open (documented as W1-api2-3 / D-1444).
  - Every other trade-page request blocks in `lock()` on its `spawn_blocking` thread and keeps its detail permit while it waits.
  - With one cold open running and three requests waiting, all 4 permits are held. Every other detail route then returns 429 "capacity is full", even though only one thread is doing work. That includes `/trades.json`, `/frontier.json`, booleanjson and indexstop*.
  - The sibling caches in the same pool avoid this by design: indexstopcandles, indexstopvix, indexstopqualification and indexstopranking use `try_lock` with a busy refusal and no queueing.
  - D-1444 states the per-request cost. It does not state the head-of-line blocking or the permit starvation.
- **Repro:**
  1. Two viewers page the trades of two different candidates K1 and K2 of one capture, each with a large trade file (for example 400k trades, about 54 MB). Each request evicts the other's slot, so every request is a cold open under the mutex.
  2. With 4 such requests in flight, a concurrent `GET /trades.json` gets 429 for the whole time the queue drains.
- **Minimal fix:** Use `try_lock` and refuse "candidate trade reader busy; retry", as the indexstop routes do. Alternatively, run `TradeReader::open` outside the lock and install it afterwards, holding the lock only for the swap and the `page` call. Either way, also evict on a `page` error (apicache-1).

---

## Pass-1 verification

| finding | verdict | reason (code at 331b05c) |
|---|---|---|
| **apicache-1** (stale `TradeReader` never evicted) | CONFIRMED | `candidatejson.rs:326-327`: `let held = cached.as_mut()...; let rows = held.reader.page(offset, limit)?;`. An Err returns with the slot still occupied. The key check at `:308-315` compares content only (model, root, identity, attempt, digest, key). `page_locked` (`candidate_trades.rs:1106-1108`) refuses any generation change, ctime included (`result_set.rs:577-591`). |
| **cli2-5** (`write_exact` exposes an empty file before it locks) | CONFIRMED | `candidate_trades.rs:1316-1322`: `create_new` comes first, then `Flock::lock(file, path)`. `read_model` at `:742-744` checks only `try_exists`, then `read_sealed_generation` at `:1361-1368` takes `try_lock_shared` and refuses `len < HEADER+SEAL` as "nonregular, truncated". The same path is also permanent for an attempt whose process died between `create_new` and `sync_all` of `catalog.bin`. The api then reports corruption for that attempt, not "missing/incomplete". The attempt token is never reused, so a rerun is not wedged. |
| **xcut-3** (candidate-trades ancestors never fsynced) | CONFIRMED | `candidate_trades.rs:322`: `fs::create_dir_all(&directory)` creates up to 4 levels (`directory_for`, `:1249-1257`). The only directory sync is `File::open(path.parent())...sync_all()` at `:1342-1344`, which is the leaf. |
| **search-1** (`write_or_equal` crash wedges a content-addressed identity) | CONFIRMED | `boolean_candidate_persistence.rs:189-211` is unchanged. `create_new` comes before `write_all`/`sync_all`, and the `AlreadyExists` arm refuses any prefix as "different or incomplete bytes". Identities are deterministic (`boolean_candidate_v1.rs:300-301`). |
| **cli3-1** (index-stop VIX companion `symlink_metadata` shortcut) | CONFIRMED | `index_stop_vix.rs:386-392`: `Ok(_) => Reader::open(...)` whenever the directory exists. `prepare_in_namespace` creates and syncs the directory first (`boolean_candidate_persistence.rs:92-95`) and writes `complete.bin` last (`:74`). `Observation::open` then fails on the missing `complete.bin` (`boolean_observation_file.rs:47`). |
| **hunt-conc-2** (Boolean attempt tokens follow the rayon schedule) | CONFIRMED | `boolean_catalog_prepared.rs:224-231`: `families().par_iter().map(... self.produce(...))`. That reaches `boolean_candidate_v1.rs:303-304` (`sweep_evidence::begin`) and, per program, `:752-753`. |
| **errpaths** `candidate_trades.rs:389` poison-swallowing `if let Ok` (info) | CONFIRMED (now `:425-429`, `refuse`) | It is still latent. `check`, `tier`, `record_inner`, `finish` and `confirm` all map poison to Err, so a dropped `failed` latch cannot let a poisoned capture publish. |
| pop2-7 class (an unrelated append invalidates a held handle; nothing serializes runs on one root) | applies here, not re-counted | `CandidateUniverseLedgerV1` releases its flock between `open_read`/`reopen_audit` and `complete_population_rows` (`step3_orchestrator.rs:696-729`). A concurrent append by another process changes `row_generation`, and `require_unchanged` refuses. That fails closed. |

---

## Checked and clean (not findings)

- **candidate_trades publication order.**
  - `start.bin` and the tier files are written before pricing. Each candidate's trades are written before its manifest, and both before `catalog.bin`.
  - `finish` re-reads every acknowledged child under shared locks before it seals the catalog. `confirm` re-reads the catalog before `attempt.finish(Completed)` (`lib.rs:19037-19049`), so no Completed audit lacks a sealed catalog.
  - Paths are per (identity, globally unique attempt token), so two processes or a rerun never share a `create_new` name.
  - The `facts` `OnceLock` initialiser (`SliceFacts::of`) is serial, with no rayon inside. That rules out reentrant initialisation from a work-stealing rayon worker.
  - Tier indices come from serial `tier()` calls. Records in `par_iter` are keyed by deterministic rank and direction, so file names and catalog bytes do not depend on the schedule.
  - The state mutex is held across each child's fsyncs. That serializes publication by design ("the short publication section is serialized") and is not a correctness issue.
- **candidate_trades readers.**
  - `pinned` warm-path refusals release through the guard's Drop.
  - `TradeReader::page` unlocks on both arms.
  - `candidates_page` budget `metadata` TOCTOU is harmless, because the content is digest-checked against the pinned catalog.
- **candidatejson render.** Catalog digest pin, then a re-read of the catalog after the pages, then `read_attempt`. A mixed page is refused. `audit_completion` is read independently and labelled as such.
- **candidate_universe.**
  - The open takes an exclusive (writer) or shared (reader) flock across the header check and the scan.
  - Append holds the exclusive flock across `require_unchanged`, rows, `sync_data`, receipt and `sync_data`.
  - Write errors roll back with `set_len(original)` (`:6292-6299`, `:6316-6323`).
  - The dup'd lock fd shares the OFD, so dropping the original does not drop the lock.
  - `ensure_header` writes only into a zero-length file.
  - `audits` is a `HashMap` used only for get/insert, and no iteration order reaches bytes.
  - Lock order is Candidate ledger, dropped, then Base Evidence. These are never nested.
- **Boolean persistence and observation.**
  - Publication holds the exclusive `owner.lock` from `prepare` through `finish`. Readers take a shared owner lock for `Observation::open` and for each projection lease, so a reader never sees a `complete.bin` being written. The "busy" answer is documented (docs/27 table).
  - `ReadLease` field order unlocks before the nesting flag clears, and `compare_exchange(AcqRel/Acquire)` is correct.
  - `with_current_many` takes leases in sorted directory order, so there is no lock-order inversion between two compound projections.
  - Body and receipt shared locks are never upgraded or contended by an exclusive lock.
- **Boolean `finish_stored_month` order.** The attempt is marked Completed before `complete.bin` is written (`boolean_candidate_v1.rs:331-339`, `lib.rs:3765-3774`). This is the documented contract: docs/24 §"finish_stored_month" says "a completed attempt describes that completed computation snapshot, not … proof of a later summary append". A retry heals it unless search-1's torn file applies. Not counted.
- **boolean_candidate_grid.rs and boolean_candidate_reader.rs.** No file I/O, locks, statics or threads beyond the `Observation` they wrap.

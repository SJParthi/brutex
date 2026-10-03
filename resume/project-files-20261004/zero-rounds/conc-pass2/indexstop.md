Verdict: 2 new findings at 331b05c in the index-stop slice (0 high, 0 medium, 2 low). Pass-1 findings that touch the slice are re-verified below: 3 confirmed (two of them have triggers or fix gaps pass 1 missed) and 1 sub-claim refuted.

# conc-pass2 / indexstop: the index-stop search, from launch to the readers that read while it writes

Slice:
- api: `indexstoplaunch.rs` and `indexstoplaunch_metadata.rs`, plus the `sweeprun.rs` admission, lease and `spawn_blocking` wiring (2990-3160).
- cli: `index_stop.rs`, `index_stop_launch.rs`, `index_stop_search.rs`, `index_stop_search_checkpoint.rs`, `index_stop_search_progress.rs`, `index_stop_search_reader.rs`, `index_stop_source_context.rs`, `index_stop_vix.rs`, `index_stop_qualification.rs`, and the `index_consistency_store::produce` call.
- Shared helpers I read where the slice calls them: `search_checkpoint.rs` (Journal and Snapshot), `boolean_candidate_persistence.rs` (`prepare_in_namespace` and `write_or_equal`), `boolean_observation_file.rs` (Observation and ReadLease), `sweep_evidence::begin_group` and `append_events`, `vix_reference.rs`, and `store::file::BarFile::open_existing`.
- api readers: `indexstopjson.rs`, `indexstopcandlesjson.rs`, `indexstopqualificationjson.rs`, `indexstoprankingjson.rs` and `indexstopvixjson.rs`.

Method: source reading only. I ran no cargo and edited nothing.

---

## indexstop-1 (low): VIX companion bytes depend on whether the VIX store happened to be locked, or not yet pulled, at capture time, but the companion is keyed only by the catalog and kept forever

**Where:**
- crates/cli/src/index_stop_vix.rs:530-559 (`load_month`), which turns every open error into a saved "unavailable" month:
  ```rust
  match VixReferenceMonth::open(store, feed, month) {
      Ok(loaded) => Ok((Month { ..., records: Some(loaded.records()), ... }, Some(loaded))),
      Err(reason) => {
          ...
          Ok((Month { ..., records: None, snapshot_digest: None, unavailable_reason: Some(reason) }, None))
  ```
- crates/cli/src/vix_reference.rs:122 (`BarFile::open_existing`). Its doc at :78-80 says it "Refuses a missing, locked, torn, malformed, or otherwise unreadable store file".
- The lock in question is a non-blocking shared lock: `store/src/file.rs:1401-1404` `Flock::try_lock_shared(handle, ...)` returns `StoreError::Locked` when any writer holds the month (`file.rs:3272`).
- The companion's key, index_stop_vix.rs:707-713:
  ```rust
  pub(crate) fn lookup_identity(catalog: [u8; 32], pin: [u8; 32]) -> [u8; 32] { ... catalog ... pin ... }
  ```
- First writer wins, index_stop_vix.rs:386-395. Once the directory exists, every later `publish` returns the saved companion.

**Why it is wrong:**
- The companion body is a function of the catalog and of the VIX store's state at that moment. Its address is a function of the catalog alone.
- A transient condition is captured and published as permanent evidence. The clearest case is a pull writer holding month M of `NSE-INDIAVIX/1min`, which yields `StoreError::Locked`.
- Pulls do not take the execution lease. `claim_execution` exists only in `sweeprun.rs`, so an api Pull press or autopilot run can append VIX month M while the index-stop worker captures it.
- No later run can correct the companion. The catalog identity is deterministic, so `publish` takes the `Ok(_)` shortcut and returns the frozen "reference_month_refused" for every trade in M. This holds for that catalog and pin for good.
- This is the engine-1 pattern (an environmental refusal made durable under an identity that does not name the environment), on a reference-only artifact. It also breaks "same inputs, same outputs, byte for byte" (§3 rule 5) for these bytes: two otherwise identical runs differ only in scheduling.
- A month that has simply not been pulled yet is frozen the same way. The code comment (index_stop.rs:409-411) presents that case as a visible snapshot, so I note it but do not count it.
- Second consequence: because the body is not a function of the address, a retry after an interrupted publication can capture different bytes once a pull has changed the VIX store. That retry fails `write_or_equal`'s equality check. So the minimal fixes proposed for cli3-1 and search-1 do not, alone, make this namespace recoverable (see the verification section).

**Repro (two api threads):**
1. Launch an index-stop search whose training window covers month M. Batch N starts.
2. During batch N, press Pull for a range including M that writes `NSE-INDIAVIX` month M. The ingest `BarFile` writer holds the exclusive month lock while it appends.
3. Inside that window, one lane reaches `publish_vix` → `capture` → `load_month(M)` → `BarFile::open_existing`. `try_lock_shared` gets WouldBlock, `Err("... could not be opened as reference evidence: ...locked...")` is returned, and it is saved as `unavailable_reason`.
4. `prepare_in_namespace`/`finish` publish it, and the batch completes normally.
5. Every later `/index-stop-vix.json` for every trade in M shows `reference_month_refused`, although the month is complete on disk. Rerunning the identical search reuses the same catalog identity and pin and returns the frozen companion.

**Minimal fix:** In `load_month`, treat a lock refusal as a failure, not as data: return `Err` so that publication refuses and the batch stays pending, and the retry recaptures. Keep only a genuinely absent or invalid month as a saved "unavailable" reason. Better still, fold a digest of the captured months' state (records count plus snapshot digest, or "absent") into the address, or verify on reuse, so that a changed VIX store is a new companion rather than a byte conflict. Add a test that holds the VIX month lock during `publish` and expects a refusal, not a saved "unavailable".

---

## indexstop-2 (low): browser read leases make the search writer's non-blocking exclusive `try_lock` refuse, and for VIX a refusal at the right moment becomes the permanent cli3-1 wedge with no crash needed

**Where:**
- Writer side: crates/cli/src/boolean_candidate_persistence.rs:101-118 (`prepare_in_namespace`):
  ```rust
  let owner = Flock::try_lock(
      crate::readonly_file::open(&owner_path).map_err(display)?,
      owner_path.clone(),
  )
  .map_err(|why| format!("Boolean candidate namespace already owned or lock refused: {why}"))?;
  ```
  This runs on every launch for every source context (index_stop_search.rs:322-335 → index_stop_source_context.rs:321). It also runs for every catalog, relation, VIX, qualification and consistency child.
- Reader side: a shared lease on the same `owner.lock`:
  - `boolean_observation_file.rs:228` `Flock::try_lock_shared(&observation.owner, ...)` is held across a whole projection.
  - `Observation::open` (:46) takes `read_held(&owner_path, 0)` shared before it even looks for `complete.bin`.
  - The candles reader holds catalog, relation and source-context leases at once (index_stop_source_context.rs:701-712). It runs from `/index-stop-candles.json` on api detail threads.
- Search journal:
  - The reader probe is search_checkpoint.rs:108 `owner.try_lock_shared()`, reached from `Snapshot::open` through `Reader::latest_checkpoint` on every unpinned `/index-stop-ranking.json`.
  - The writer is `Journal::open` :153-157 `Flock::try_lock(...)` → "this exact search is already owned or cannot be locked". It runs only after `load_sources` and `prepare_sources` have finished (index_stop_search.rs:176-191).

**Why it is wrong:**
- flock shared and exclusive locks conflict across open file descriptions, including within one process. The writer uses `try_lock` and never waits.
- A browser read of saved evidence that overlaps a writer step on the same identity therefore turns the write into a refusal. The refusal message calls the namespace "already owned", which names no owner.
- Case (a), source context:
  - Continuing a search re-publishes the same content-addressed source contexts the candles page is reading. The identity is `context_identity(body)` and the sources are unchanged.
  - The refusal lands in `prepare_sources`, so the whole invocation fails after all selected training and later sources were loaded and decoded.
  - The rung is reported `Refused` with the misleading owner message.
- Case (b), journal: a ranking request's probe that straddles `Journal::open` refuses the launch at the same late point. The window is two syscalls, so this is rare.
- Case (c), VIX, which is the one that persists:
  - `prepare_in_namespace` creates the directory and `owner.lock`, and `sync_all`s it, before it calls `try_lock`.
  - If a `/index-stop-vix.json` request for that catalog and pin lands in that window, its `Observation::open` holds `owner.lock` shared until it fails on the missing `complete.bin`. The writer's `try_lock` then refuses.
  - The catalog is complete before VIX publication, and `/index-stop.json?identity=` returns the pin unpinned, so the request is possible.
  - The refusal leaves `index-stop-vix-reference-v1/<lookup>/` holding only `owner.lock`. Every retry then takes `publish`'s `Ok(_)` directory shortcut (index_stop_vix.rs:387) into `Reader::open` and refuses for good.
  - This is the cli3-1 wedge reached by a live, ordinary race rather than a crash. The window is the owner fsync plus a few syscalls, so it is narrow, but the result is permanent.

**Repro (case a, the likely one):**
1. Search S has saved batches. The operator opens a saved trade's candles. `indexstopcandlesjson::render` → `reader.with_current(...)` holds a shared lease on `index-stop-source-context-v1/<C>/owner.lock`.
2. In parallel, the operator presses Run to continue S. The worker loads every source (minutes), then `prepare_sources` → `source_context::publish` → `prepare_in_namespace(..., C, ...)` → `Flock::try_lock` → WouldBlock.
3. The invocation returns "Boolean candidate namespace already owned or lock refused: ...". The status marks the rung Refused, and the loaded sources are discarded. A retry after the read finishes succeeds.

**Repro (case c, permanent):**
1. Batch N's lane publishes catalog K (pin P) and enters `publish_vix`. `symlink_metadata` gives NotFound. `prepare_in_namespace` makes the directory, creates `owner.lock` and is inside `file.sync_all()`.
2. An api detail thread serving `/index-stop-vix.json?identity=K&pin=P` runs `Reader::open` → `Catalog::open` (succeeds, K is complete) → `Observation::open(..., lookup, ...)` → `read_held(owner.lock)` → `try_lock_shared` succeeds.
3. The writer's `Flock::try_lock` gets WouldBlock and `publish_vix` errors. The reader then fails on `complete.bin` and releases.
4. The batch stays pending. Every retry: `symlink_metadata(directory)` is Ok → `Reader::open` → "saved VIX reference companion unavailable or invalid". The search can never pass batch N.

**Minimal fix:**
- In `prepare_in_namespace`, take the owner lock with a blocking `lock()` (bounded by a deadline if needed) rather than `try_lock`. Readers hold it only for one projection.
- Report a contended lock as "busy, retry", not "already owned".
- Fix the cli3-1 shortcut (key it on `complete.bin`), which removes the permanent half of case (c).
- Move `Journal::open` ahead of `load_sources`, so a contended or owned journal refuses before any source is loaded.

---

## Pass-1 verification (findings that touch this slice)

- **cli3-1 (VIX directory-exists shortcut wedges the search): CONFIRMED, and wider than reported.**
  - index_stop_vix.rs:386-395 returns `Reader::open` whenever the directory exists. `prepare_in_namespace` creates that directory first (boolean_candidate_persistence.rs:96-98), and `complete.bin` is written last by `finish` (:74).
  - The trigger is not limited to a crash. Any live error after the directory exists has the same effect:
    - ENOSPC or EIO in `write_or_equal(body.bin)` or `(complete.bin)`;
    - a `verify_body` mismatch;
    - an owner `try_lock` refusal (indexstop-2 case c).
    The error is returned, the directory stays, and every retry takes the shortcut.
  - Fix gap: the proposed fix (shortcut only on `complete.bin`, otherwise `prepare_in_namespace`) is not enough for this namespace. The VIX body is not a function of its lookup identity (indexstop-1). After a pull changes the VIX store, the recapture differs from a fully written `body.bin` left by the failed attempt, and `write_or_equal` refuses for good.
- **cli3-1 second, transient case ("the api's index-stop worker and a cli run" reaching the same catalog): REFUTED.**
  - No cli verb runs an index-stop search or produces a catalog. `produce_catalog` and `index_stop_search::execute*` have no caller outside `api::indexstoplaunch` and tests; main.rs names neither.
  - In the api, launches are serialized by the `admit` slot and by `cli::execution_lease` (`claim_execution`, sweeprun.rs:3079).
  - Within one batch, lanes are distinct rungs, so their catalog and VIX lookup identities differ. Two writers cannot reach one VIX directory.
- **search-1 (torn content-addressed body/receipt wedges the identity and the search): CONFIRMED.**
  - `write_or_equal` (boolean_candidate_persistence.rs:189-211): `create_new`, then `write_all`, then `sync_all`. The `AlreadyExists` arm refuses any prefix. All five index-stop namespaces route through it.
  - `Frame::done` and `transition` (index_stop_search_checkpoint.rs:165-207, 529-543) force the exact batch retry.
  - Two additions:
    1. A live ENOSPC or EIO in `write_all` also leaves a short file. Nothing removes it and the error is returned, so no crash is needed.
    2. For `index-stop-source-context-v1` the wedge is wider than one batch. That context is re-published at the start of every launch over that source (index_stop_search.rs:322-335), so every search over that source, new or continued, refuses in `prepare_sources` forever.
- **cli2-1 (torn `attempts.bin` after a short write, no rollback): CONFIRMED for the index-stop trigger it names.** `append_events` (sweep_evidence.rs:1383-1386) has no `set_len` on a `write_all` error. Index-stop calls `begin_many` in groups of 16 (index_stop.rs:474-477) and `finish_many` (:427), plus single `begin` calls for the catalog, preparation, qualification and consistency attempts.
- **hunt-conc-5 (worker count in report bytes): CONFIRMED, info.** index_stop_search.rs:206-221 prints `lanes`, which comes from `available_parallelism`. It is not part of the declaration or identity (`legacy_declaration` omits `workers`).

## Checked and sound (not findings)

- **Single writer per search.**
  - `Journal::open` holds an exclusive owner flock for the whole invocation.
  - api launches are serialized by `admit` and by the store-scoped execution lease, which the `TaskFinisher` holds until the worker returns.
  - No cli verb writes `index-stop-search-v1`.
- **Checkpoint crash points.**
  - The pending frame is published before any child, and the done frame after all of them.
  - A crash between the two re-runs the same reserved batch. `recover` requires `history.len() == acknowledged` and a pinned predecessor chain, and `transition` forbids skipping or changing a reservation.
  - Reserved directories without `complete` are counted as interrupted and skipped by `next`.
  - I found no skip or double count. The 0-byte `complete` marker is GAP11-0, already known.
- **Determinism of retried children.** Catalog, qualification, consistency and source-context bodies contain no wall clock. Qualification numerics use no rayon, and the only parallelism (lanes) is reassembled by index. A retry after the done-publish fails is therefore byte-identical, except for VIX (indexstop-1).
- **`live::parallel` and `schedule`.**
  - A bounded channel. The receiver is dropped on `break`, so a blocked `send` returns Err.
  - `cancelled` is Release/Acquire. Scoped join with panic mapping. `broadcast` jobs are pinned to their thread, so the `workers` cap holds.
  - Telemetry `note` and the api observer run on the coordinator (`spawn_blocking`) thread, which carries the operation-audit thread-local.
- **`indexstoplaunch::conduct`.**
  - The observer takes `site.sweep` only for one assignment and never across a cli call.
  - Poison maps to a refusal, which cancels at the next boundary.
  - `Status::observe` monotonicity matches `progress()` across pending, stage and done transitions, including a recovered pending frame (identical repeat observation accepted).
- **Readers during writes.**
  - `Snapshot::open_prefix` pins a sequence ceiling, so a newer append cannot widen a pinned read.
  - An unpinned read during the publish window gets "busy" (payload exclusive lock) or a marker-width refusal. These are transient and leave the api caches intact, because the error comes before any slot mutation.
  - Every index-stop api cache uses `try_lock` (busy, no queueing) and evicts on currency failure. `indexstopjson` re-admits on an unpinned request.
- **Lock ordering.** Every observation lease is a non-blocking `try_lock_shared`. The only blocking flocks on the path are `attempts.bin` (taken while holding shared leases, never the reverse) and the fresh `create_new` checkpoint payload. A wait cycle is impossible.
- **Environment.** There is no runtime `set_var`. The configuration env vars read at admission and at execution are fingerprint-compared.

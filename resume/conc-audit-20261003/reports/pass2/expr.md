# conc-pass2 / expr: 3 findings (0 high, 0 medium, 3 low) at 331b05c

Verdict: the checkpoint chain itself is resume-safe: no skip and no double count after a kill at any write. Every defect I found is a read-side or lock-interaction problem: a reader's momentary shared flock makes a non-blocking writer acquisition fail, and the two API caches hold one global mutex across disk I/O. The known torn-`complete` wedge (GAP11-0) is still present and is the only crash-time defect in this slice.

Slice: `crates/cli/src/search_checkpoint.rs` (the shared journal under every search here), `expression_search.rs`, `expression_search_reader.rs`, `boolean_grammar_batch.rs`, `boolean_grammar_campaign.rs`, `boolean_campaign.rs`, `boolean_campaign_codec.rs` (transition), `boolean_campaign_reader.rs`, `crates/vocab/src/expression_search.rs` (`Cursor` encode/decode), `crates/api/src/expressionsearchjson.rs`, `booleanjson.rs` and `booleancampaignjson.rs`. I followed calls out into `boolean_candidate_persistence.rs`, `boolean_observation_file.rs`, `boolean_catalog_prepared.rs::run` and `sweep_evidence.rs` (begin/Drop/last_event) as far as the slice reaches them. Audit only: source reading, no cargo run, nothing edited.

---

## expr-1 (low): a read-only `Snapshot` probe takes a shared flock on the writer's `owner.lock`, so a CLI that starts or resumes the same search at that instant is refused as "already owned"

**Where:** `crates/cli/src/search_checkpoint.rs:106-115` (reader probe) against `:153-156` (writer acquisition).

```rust
// Snapshot::open_through (every API / observer read)
let owner = crate::readonly_file::open(&owner_path).map_err(error)?;
let before = crate::result_set::file_generation(&owner, &owner_path)?;
let writer_observed = match owner.try_lock_shared() {
    Ok(()) => { owner.unlock().map_err(error)?; false }
    Err(std::fs::TryLockError::WouldBlock) => true,
    ...
// Journal::open (the only writer door)
let owner =
    Flock::try_lock(open_owner(&owner_path)?, owner_path.clone()).map_err(|why| {
        format!("this exact search is already owned or cannot be locked: {why}")
    })?;
```

**Why it is wrong:** `Journal::open` uses a non-blocking exclusive `flock` to mean "another writer owns this search". A reader that only wants to *observe* liveness takes a shared lock on the same file, and an exclusive `try_lock` fails with `EWOULDBLOCK` against a shared holder too. So a harmless observer turns into a false "already owned" refusal for the real writer. Every `Snapshot::open` takes this probe. That covers `/expression-search.json` (`expression_search_reader.rs:97`), `/boolean-campaign.json` (`boolean_campaign_reader.rs:26`, taken twice per request: once in `Reader::open` and again in `require_current`), and the CLI's own `boolean_campaign::verify_complete` (`boolean_campaign.rs:693`). The refusal costs the writer more than a retry:

- `expression_search::run` has already appended an `ExpressionSearch` attempt start (`expression_search.rs:254-258`) before `Journal::open` (`:259`). That attempt is now sealed `Refused` by `Attempt::drop`.
- `boolean_grammar_campaign::execute_with` builds the full eight-rung `probe` preparation (`:147`) before `Journal::open` (`:162`). That preparation loads and audits every source month, and all of it is thrown away.
- `boolean_campaign::run_borrowed` (`:419`), reached from a grammar batch, fails the batch. The grammar `PLAN` stays pending until someone reruns the command by hand.

**Repro (two processes):**
1. Search S is paused. A browser tab loads `/expression-search.json?identity=S`. The API thread reaches `search_checkpoint.rs:108` and holds `LOCK_SH` on `expression-search-v1/<S>/owner.lock`.
2. Before that thread reaches `:110`, the operator runs `cli expression-search-stored ...` for S. Its `Journal::open` reaches `:154`, and `flock(LOCK_EX|LOCK_NB)` returns `EWOULDBLOCK`.
3. The CLI prints "this exact search is already owned or cannot be locked: …", and the attempt journal records a Refused `ExpressionSearch` attempt. No process owned S.

The window is short (open, fstat, lock, unlock), so this is low. It is still deterministic once the interleaving happens, and it does the opposite of what the probe was added for.

**Minimal fix:** make the writer tolerate observers. In `Journal::open`, retry `try_lock` on `WouldBlock` for a short bounded period (for example 50 attempts × 10 ms) before refusing as owned. A real owner holds the lock for the whole run, so it still refuses. Alternatively, have readers detect a writer without taking a lock that conflicts with the writer's acquisition, for example a separate `writer.lock` that only writers take exclusively and readers probe on a duplicate descriptor. The first fix is smaller.

---

## expr-2 (low): resuming a campaign rung re-prepares every child already completed, under a non-blocking exclusive owner lock, so one dashboard read of that child makes the rung refuse and burns two campaign checkpoints

**Where:** `crates/cli/src/boolean_candidate_persistence.rs:111-115`, reached on every rung (re)run from `boolean_catalog_prepared.rs:224-233` (all families recomputed and `persistence::prepare` called, with no completed-child shortcut), `boolean_statistics_v1.rs:191` and `boolean_admission_v1.rs:117`.

```rust
let owner = Flock::try_lock(
    crate::readonly_file::open(&owner_path).map_err(display)?,
    owner_path.clone(),
)
.map_err(|why| format!("Boolean candidate namespace already owned or lock refused: {why}"))?;
```

The readers that hold a shared lock on that same `owner.lock`:
- `boolean_campaign_reader.rs:170`: `Flock::try_lock_shared(&owner, …)` for every completed child listed in the snapshot, on each `/boolean-campaign.json` request, twice (open and `require_current`).
- `boolean_observation_file.rs:47` (`read_held(&owner_path, 0)`, shared), at every cold open, plus `ReadLease::acquire` (`:228`, shared) for every projection: `/boolean.json`, `/boolean-evidence.json` and the OOS pages.

**Why it is wrong:** the identities are content-addressed, so a resumed rung recomputes and re-`prepare`s exactly the children the snapshot already records as complete (`record_stage` accepts the identical pin). Those are exactly the children the dashboard reads. An exclusive `try_lock` meant to detect a concurrent *publisher* also fails against a *reader's* shared lease. The family result becomes `Err`, `collect_families` (`boolean_catalog_command.rs:189-209`) refuses the whole rung, and `run_borrowed` publishes `Refused` (`boolean_campaign.rs:478-495`). Each such retry costs two of the 1,024 campaign checkpoints (`MAX_CHECKPOINTS`, `:24`): one `Running` and one `Refused`. A grammar campaign whose pending batch needs the rung fails that invocation.

**Repro (two processes):**
1. Campaign C, rung `5m`. Families F1..Fn completed, then the statistics stage was refused, or the process was killed. The snapshot records F1..Fn completions.
2. The operator opens `/boolean.json?identity=<F3>` (or `/boolean-campaign.json?identity=C`). The API thread is inside `Observation::open` or `with_current`, holding `LOCK_SH` on `boolean-candidates-v1/<F3>/owner.lock`.
3. Meanwhile `cli boolean-campaign-stored …` (or `boolean-grammar-campaign-stored`) resumes C. The rayon worker for F3 reaches `persistence::prepare` → `:111`, gets `EWOULDBLOCK`, and returns "Boolean candidate namespace already owned or lock refused".
4. `collect_families` refuses. The rung is checkpointed `Refused`, and the command exits non-zero, although no other writer existed.

**Minimal fix:** in `prepare_in_namespace`, take the owner lock with the blocking `Flock::lock` (two publishers of one content-addressed identity produce identical bytes, so waiting is safe), or with a bounded retry loop. The cleaner fix is to short-circuit before `prepare`: when `complete.bin` already exists and `persistence::verify` accepts the recomputed `(identity, payload, bytes)`, return the committed value without taking the exclusive lock at all.

---

## expr-3 (low): `/expression-search.json` and `/boolean.json` hold a process-wide std `Mutex` across cold disk verification, so concurrent requests park shared `detail::run` permits and unrelated routes answer 429

**Where:**
- `crates/api/src/expressionsearchjson.rs:115-154`: `SESSIONS.lock()` is held across `Reader::open` (directory discovery up to 1,000,000 entries, then a checkpoint read) **and** `held.reader.page(...)` (up to 256 links within `MAX_SCAN_BYTES` = 64 MiB: each child's signal file is read and verified, `sweep_evidence::read_attempt` and `candidate_trades::read_model` run, and `last_event` takes a **blocking** `lock_shared` at `sweep_evidence.rs:1422`, which waits out a CLI append's write and fsync).
- `crates/api/src/booleanjson.rs:192-221`: `CACHE.lock()` is held across a cold `Reader::open`, which hashes and decodes the whole catalog body (bounded by `BooleanObservationBudget`), and across `project`.

```rust
let mut sessions = SESSIONS.get_or_init(...).lock().map_err(|_| "search snapshot cache poisoned")?;
... Reader::open(root, asked.identity, crate::detail::MAX_SCAN_BYTES)? ...
let page = held.reader.page(asked.cursor, asked.limit, crate::detail::MAX_SCAN_BYTES)?;
```

**Why it is wrong:** both handlers run inside `crate::detail::run`, which first takes one of `MAX_CONCURRENT = 4` shared permits (`detail.rs:14`, refused as `Saturated` with nothing queued). A second request to the same route, **for any identity**, then blocks on the mutex while still holding its permit. With three such requests parked behind one slow cold read, every other `detail::run` route in the server returns 429 "capacity full". That is the same mechanism as pass-1 **runs-4** (the `ADMISSION` mutex), at two new sites. It also serializes reads of unrelated identities, which the per-identity design does not need.

**Repro:** tab 1 opens the first page of a long expression search (`limit=256`) whose 256 child signal files total about 60 MiB. That page reads all of them three times under `SESSIONS`. Tabs 2 and 3 request any other expression-search page, and tab 4 requests a third page. Each takes a permit and blocks on `SESSIONS`. Every other `detail::run` route (`/boolean-campaign.json`, `/candidate-trades.json`, `/backtest/run.json` status, …) now gets `RunError::Saturated` until tab 1's verification ends. The same happens with `/boolean.json` while a cold catalog hash holds `CACHE`.

**Minimal fix:** take the mutex only to look up, insert or evict entries. Hold each session or reader behind its own `Arc<Mutex<…>>` (or take it out of the deque and put it back afterwards). Run `Reader::open` and `page`/`project` outside the global lock. Alternatively, `try_lock` the global mutex and answer 503 "busy" instead of parking a permit, as the `indexstop*json` handlers already do.

---

## Pass-1 verification (findings touching this slice)

| Pass-1 ID | Verdict | Reason (at 331b05c) |
|---|---|---|
| **apicache-2** (unpinned `/expression-search.json` discards another viewer's session) | **CONFIRMED** | `expressionsearchjson.rs:133-137` still `retain`s away every session with the same `(root, identity, checkpoint)` and pushes a fresh `Reader` whose `admitted` is `{checkpoint}` only (`expression_search_reader.rs:127`). The pinned lookup at `:119-127` matches on checkpoint alone, so the old viewer's learned cursor fails `page()` at `expression_search_reader.rs:151-152`. **Addendum (same class, not counted):** eviction at `:138-140` is `pop_front` by insertion order, and pinned use never moves a session to the back. So eight first-page loads of *other* identities also evict an actively paginating viewer ("snapshot is not admitted or expired"). The proposed fix should also move a pinned hit to the back. |
| **search-1** (crash inside `write_or_equal` wedges a content-addressed identity forever) | **CONFIRMED**, and it reaches this slice | `boolean_candidate_persistence.rs:189-211` is unchanged: `create_new` comes before the write, and on retry `AlreadyExists` → `read_exact(path, body.len()) != body` → permanent refusal. In this slice the effect is: every resume of the campaign rung re-`prepare`s that identity (`boolean_catalog_prepared.rs:224-233`) and refuses, publishing `Running` then `Refused` (two of `MAX_CHECKPOINTS` = 1024) each time, until `publish` refuses "checkpoint history ceiling reached". A grammar campaign cannot route around the batch, because `restore` refuses "grammar advanced past an unfinished batch" (`boolean_grammar_campaign.rs:383-385`), so the whole grammar search is wedged too. That was already implied by search-1's "every resumable search whose pending batch needs it". |
| search (pass-1) "boolean_campaign.rs and boolean_grammar_campaign.rs: checkpoint chains are safe after a publish error because `Journal::publish` self-poisons" (clean note) | **CONFIRMED** | `search_checkpoint.rs:204-211` sets `poisoned` on any `publish_inner` error, so the refusal publish in `run_borrowed:478-495` errors loudly and does not fork the chain. |
| xcut H8 note: torn `complete` marker (GAP11-0) and no `DIRECTORY_LIMIT` check (W2-cli13-5), both KNOWN | **CONFIRMED still present** | `search_checkpoint.rs:263-267` is still `File::create_new(...complete)` followed by `write_all(&seal)`. `discover_through:457-463` counts a 0-byte marker as acknowledged and makes it `latest`. `read_saved:486-488` refuses "marker width mismatch" from then on for both `Journal::latest` (the writer can never resume) and every `Snapshot` reader. **Not counted, but a trigger the earlier reports did not name:** the same 0-byte marker is visible *without a crash*. A `Snapshot::open` that runs between `:263` and `:265` of a live publish returns "checkpoint marker width mismatch" to `/expression-search.json` or `/boolean-campaign.json`, which reads as corruption rather than "busy". The GAP11-0 fix (write the seal under a temporary name, fsync, then `link`/`rename` to `complete`) removes both. `publish_inner:224-230` still has no `DIRECTORY_LIMIT` check. `expression_search` relies only on discovery refusing at 1,000,000 entries, after the fact. |

## Checked and clean (not findings)

- **Expression search resume after a kill at each write** (`expression_search.rs:509-551`, `run:246-310`):
  - Killed after a child's `attempt.finish(Completed)` but before `journal.publish`: the child attempt is an unreferenced orphan, and the rerun re-advances the same cursor from the last acknowledged checkpoint, evaluates the same expression under a **new** token, and publishes it. `candidates`/`rows`/`qualifying` live only in the checkpoint, so nothing is double-counted.
  - Killed inside `publish_inner` before the marker: a reserved hole, counted as `interrupted`, with `previous` still pointing at the last acknowledged seal.
  - Killed mid-`evaluate_candidate`: the child attempt never gets a terminal record (no Drop on SIGKILL). It is token-isolated, and `verify_history` only follows `last` pointers in acknowledged checkpoints.
  - `verify_history`/`verify_transition` re-prove every transition, with work delta ≤ 4096, matching `execute`'s `remaining.min(4096)`.
- **`Cursor` resume determinism** (`vocab/src/expression_search.rs`): equality and the encoded form exclude scratch (D-0750/D-0754), and `decode` rebuilds `code`/`starts`/`depths` below `at` through `place` (`:370-373`). A resumed cursor therefore advances exactly as an uninterrupted one. `Batch::follows` compares encodings only.
- **Grammar campaign** (`boolean_grammar_campaign.rs`):
  - A kill after `PLAN` and before `DONE` leaves a pending batch, and resume re-runs the same campaign identity.
  - A kill after the inner campaign completes but before `DONE`: `run_borrowed` sees `Completed` and returns the same pin without publishing (`:422-429`). `codec::transition` forbids any record after `Completed`, so the pin `DONE` stores can never be superseded.
  - `restore` refuses skips, repeats and post-exhaustion records. `admit_checkpoints` charges holes.
- **Boolean campaign** (`boolean_campaign.rs`): `Running`, `Paused` and `Refused` rows resume to `Running`. Completed and Excluded rows are immutable under `transition`. Completion is published in the same record as the last rung's `Completed`. The reader refuses `sequence >= MAX_CHECKPOINTS`, and the writer's guard keeps sequences ≤ 1023.
- **Two writers of one search identity:** both `Journal::open` calls use an exclusive non-blocking flock that the owner keeps until drop, so the second refuses. (A side note, not counted: `expression_search::run` appends its `ExpressionSearch` attempt start before `Journal::open`. A refused second process therefore becomes the identity's newest attempt, marked Refused, while the first is still running. No reader consumes that operation's latest attempt; grep shows only `expression_search.rs:256` and tests.)
- **Child identity sharing across searches:** expression candidate identities omit the search alphabet, so two searches can evaluate the same child concurrently. `sweep_evidence::begin` allocates distinct tokens under the flocked journal, and every reader reads `(identity, attempt)` exactly, so they do not collide.
- **Reader staleness:** `Snapshot` holds no lock between calls. A held `expression_search_reader::Reader` stays pinned to an immutable checkpoint and re-verifies it on every page (`:155-157`, `:204-206`). The `booleanjson` cached `Observation` keeps body and receipt shared-locked, but the writer only ever takes shared locks on those two files (`read_held`), so a cached reader never blocks a publish.
- **Ordering and atomics:** no atomics, threads or `HashMap` iteration reach output in this slice. The `HashSet<Anchor>` in the expression reader is membership-only. Rayon family results in `boolean_catalog_prepared::run` come back in input order (`par_iter().map().collect()`). The grammar identity and the campaign descriptor hash fixed-order inputs.
- **Poisoning:** `SESSIONS`, `CACHE` and `ranked_digest` map poison to a refusal. I found no reachable panic under `SESSIONS` or `CACHE` (all arithmetic in `page`/`transition` is checked or guarded), so the wedge-after-panic path is not reachable.

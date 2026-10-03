# conc-pass1 / apicache — api read caches vs files changing underneath

**Verdict: 2 findings (0 high, 1 medium, 1 low).** Both are in-process cache-eviction defects. Neither serves wrong bytes. Each one makes a route refuse requests it should answer after a file underneath changes. Audited at commit 331b05c, by reading the source only (no cargo).

Slice: `crates/api/src` non-test files, excluding server, ingest, assets, sweeprun, pullrun, mastersrun, autopilot, recovery*, credential_law, audit and render.

---

## apicache-1 (medium): `/candidate-trades.json` keeps a stale `TradeReader` and refuses that candidate until restart

**Where:** `crates/api/src/candidatejson.rs:302-328` (`trade_page`). The refusal is raised at `crates/cli/src/candidate_trades.rs:1106-1108` (`TradeReader::page_locked`).

```rust
static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
...
if cached.as_ref().is_none_or(|held| {
    held.model != summary.model || held.root != root || held.identity != summary.identity
        || held.attempt != summary.attempt || held.digest != summary.digest || held.key != key
}) { /* cold TradeReader::open, replace slot */ }
let held = cached.as_mut().ok_or("candidate reader cache missing")?;
let rows = held.reader.page(offset, limit)?;      // Err leaves `held` in the slot
```

and in cli:

```rust
fn page_locked(&mut self, ...) {
    let generation = crate::result_set::file_generation(&self.file, &self.path)?;
    crate::result_set::require_generation_unchanged(self.generation, generation, &self.path)?;
```

**Why it is wrong.**
- The slot is keyed only on content: model, root, identity, attempt, catalog digest and candidate key. Every request reads `summary` fresh through `read_model`.
- The cached reader also pins the trade file's filesystem generation: device, inode, mtime and ctime (`result_set.rs:200-214`).
- Suppose the trade file's generation moves while its content and the catalog digest stay the same. `page()` then refuses with "...validated filesystem generation changed; ... Reopen to validate the complete manifest".
- The cache is not evicted on that error. The next request computes the same key, hits the same stale reader, and refuses again. This repeats for every page and for the unpinned first page too. Nothing reopens, even though the error text says to reopen.
- It clears only when some other candidate key is requested, or when the process restarts.
- Every sibling cache evicts on this exact condition:
  - `detail::Cached::with_verified` sets `*held = None`.
  - `indexstopcandlesjson.rs:186-192` and `indexstopvixjson.rs:162-166` say in their comments that a retry "may re-admit byte-identical restored files under the same original completion".
  - `indexstoprankingjson.rs:221-223` and `indexstopqualificationjson.rs:192-194` also evict.
- `candidatejson` is the one cache in the slice without eviction.

**Repro (one process, no race needed).**
1. Start the api. Open `/candidate-trades.json?identity=I&attempt=A&tier=0&rank=0&direction=long`. A cold `TradeReader` is cached for key K.
2. Change the trade file's ctime without changing its bytes. Any one of these works:
   - `ln <store>/.../trades-0-0-long.bin /backup/x` (link() bumps ctime; `cp -al` and rsnapshot-style backups do this to every file);
   - `chmod`/`chown`/`touch` on the file;
   - an operator deletes a capture directory that a crash left torn, and reruns the capture. `write_exact` is deterministic and idempotent, so the catalog digest is the same but the inodes are new.
3. Request the same URL again, with or without `digest=`. The response is a 503 "kept length N but its validated filesystem generation changed". Every later request for K gets the same 503 until restart. A cold `TradeReader::open` on the same files would succeed.

**Minimal fix.** Evict when a page fails, the same way `with_verified` does:

```rust
let page = held.reader.page(offset, limit);
if page.is_err() { *cached = None; }
let rows = page?;
```

Alternatively, refuse only when `require_current` fails, as `indexstopvixjson::project_cached` does. Either way the next request cold-opens and re-verifies every row and the seal.

---

## apicache-2 (low): an unpinned `/expression-search.json` request discards another viewer's pagination session

**Where:** `crates/api/src/expressionsearchjson.rs:129-144` (`render`).

```rust
let Some(reader) = Reader::open(root, asked.identity, crate::detail::MAX_SCAN_BYTES)? else { ... };
sessions.retain(|held| {
    held.root != root
        || held.identity != asked.identity
        || held.reader.progress().checkpoint != reader.progress().checkpoint
});
...
sessions.push_back(Session { root, identity, reader });
```

**Why it is wrong.**
- A `Session`'s reader holds `admitted`: the continuation anchors learned while walking pages (`cli/src/expression_search_reader.rs:209-218`).
- A pinned page with a cursor is accepted only if that cursor is in `admitted` (`page()`: "search cursor is not linked to this admitted snapshot").
- Pinned requests look up the session by checkpoint alone (`position(... checkpoint == Some(snapshot))`).
- An unpinned (first-page) request for the same identity opens a fresh reader. If the search has not written a new checkpoint, the fresh reader has the same checkpoint, and the old session (with all its learned cursors) is deleted and replaced by one whose `admitted` holds only the head.
- This happens exactly when the search is paused, stopped or exhausted, which is when people browse its history.
- One viewer's first-page load therefore breaks every other viewer's deep pagination on the same search.

**Repro (two clients, any order).**
1. Search S is paused at checkpoint C.
2. Tab A requests `identity=S` and gets `snapshot=C, next=N1`. It then requests `snapshot=C&cursor=N1` and gets `next=N2`. A's session now has `admitted={C,N1,N2}`.
3. Tab B (or A's own reload in another window) requests `identity=S` with no snapshot. `retain` drops A's session (same checkpoint C) and pushes a new one with `admitted={C}`.
4. Tab A requests `snapshot=C&cursor=N2`. `position` finds B's session (checkpoint C), and `page()` returns `Err("search cursor is not linked to this admitted snapshot")`. A must restart from page 1.

**Minimal fix.** On an unpinned request whose freshly opened checkpoint equals a held session's checkpoint, keep the held session and move it to the back of the LRU. Do not replace it. Optionally drop the fresh reader. The fresh reader is still needed to *learn* the current checkpoint, but the old one's `admitted` set is a superset of what the fresh one would hold.

---

## Checked and found sound (not findings)

- **`detail::Cached` (TRADES, FRONTIER, PARENTS, LEDGER; topjson SELECTION):**
  - `with_verified` evicts on a refresh refusal, and the next request reopens. The lock is read through poison.
  - No path nests two `Cached` locks. trades and frontier take PARENTS, then TRADES or FRONTIER, one after the other.
  - The `Results::refresh` and `read` shared `flock` waits out a cli append, so there is no torn-record false alarm.
- **topjson `Selection::extend`:** a mid-batch `read` error leaves `best`/`pairs` partly updated. `with_verified` then discards the whole slot (`*held = None`), so the partial state is never served.
- **`detail::recorded_underlying`:** the zero-length check races only toward "no run recorded". That is true for a header-less ledger.
- **booleanjson, booleanevidencejson, booleanoosjson, indexstopjson:**
  - A pinned request over a held reader reuses it and refuses a changed generation by design. The slot is not evicted.
  - The unpinned first page that the client must restart from always re-admits (`must_admit` / `completion.is_none()`), so they recover.
  - Their `lock().map_err("poisoned")` would wedge after a panic. I found no reachable panic under the lock (`start+n` sites are bounded by rows actually returned), so this is not reported.
- **indexstopcandlesjson, indexstopvixjson, indexstopqualificationjson, indexstoprankingjson:** `try_lock` (busy refusal, no queueing) plus eviction on currency failure.
- **`calendar_of::Cache`:**
  - The single-flight `Landing` drop wakes followers on a panic. Followers loop to a new leader.
  - The stamp is the caller's pre-read census stamp (D-0695).
  - A slow older-stamp leader can overwrite a newer entry in `held`. Only requests carrying that older stamp can hit it, so the cost is one extra derivation and never a stale answer.
- **`audit_json::RollupCache` and `store_wire::Cache`:**
  - Both are keyed by `Weak::ptr_eq`. The held `Weak` keeps the `ArcInner` allocation alive, so the address cannot be reused (no ABA).
  - A late publisher of an older snapshot only causes a recount.
- **`census::sized`:** one `O_NONBLOCK` open, size from that descriptor, and a capped read. The manifest is installed by temp+rename (`pull/src/manifest.rs:3006`), so a reader never sees a half-written manifest.
- **`livejson` CENSUS:** the lock is read through poison, and the refresh logic lives in cli (`cli::live::CensusCache`).
- **`logs::both_halves`:** a stable sort with a `(millis, seq)` tiebreak, served half first, so the order is deterministic. Torn tails are reported by `telemetry::tail`.
- **`detail::Permit`:** `fetch_update(AcqRel)` admission, and Drop releases the slot. `owed()` counts past the cap intentionally (D-1445).
- **booleanlaunch / indexstoplaunch `conduct`:** the status lock is held only for one assignment. `finished_micros = Some(started)` is overwritten by sweeprun.rs:3159.
- **No writes:** no non-test file in the slice creates, renames or writes store files. `folder.rs`'s `NEXT` and `isolated`/`scratch`/`emitted` are test-only.

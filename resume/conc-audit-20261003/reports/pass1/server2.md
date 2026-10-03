# conc-pass1 / server2: verdict: no high or medium defects in the slice; 2 low findings (one of them unreachable in production)

Slice: `crates/api/src/server.rs` lines 17000 to the end, plus `crates/api/src/ingest.rs` and `crates/api/src/assets.rs`. Commit 331b05c. I read the source only and did not run cargo.

The production code in this slice is small. Most of server.rs 17000+ is `#[cfg(test)]`: modules at 17134-17450, 18583-31114, 31116-31270, 31413-32110 and 32354-32750, plus the test modules declared at 33528+. What is left is the connection wrapper (17000-17133), the serve lock and the serve arm (17451-18582), universe/indexmap/vocab/calendar handlers (31271-33527), ingest status/queue (ingest.rs:1-2246), and static assets (assets.rs:1-1031). ingest.rs has no production disk writes; `status_json` and `queue` only read the autopilot's in-memory state.

## Findings

### server2-1 (low): the front-end root and build state are cached at startup, so `/` stays 503 after the operator builds the bundle the 503 page tells them to build

- File: `crates/api/src/assets.rs:681-701` (`Assets::new`) and `:798-800` (`respond`); `:988-1021` (`not_built`). The value is constructed once at `crates/api/src/server.rs:18432`.
- Code:
  - `let root = std::fs::canonicalize(&named).ok().filter(|resolved| resolved.is_dir());` (read once, in `new`)
  - `let Some(root) = self.root.as_deref() else { return self.not_built(); };` (every request)
- Why it is wrong: `root: Option<PathBuf>` is fixed for the life of the process. If `web/build` is absent at startup, every page request answers 503 "The front end is not on disk ... What produces it: `npm --prefix web ci && npm --prefix web run build`" for the rest of the process, even after that command has succeeded and the files are on disk. The page and the banner never say that a restart is also needed. The operator follows the printed instruction and still gets the same 503. That is a cache serving a stale answer after the file changed, and the refusal names an incomplete remedy (CLAUDE.md §4: name the reason). The reverse case is also cached: `root` is the canonical target, so if `web/build` is a symlink that is retargeted (an atomic `build -> build.v2` swap) and the old target is removed, every request canonicalizes under the dead old root and answers 404 or the 503 shell page until restart.
- Repro: (1) `mv web/build web/build.off`. (2) Start `api serve`. The banner says `NOT BUILT`. (3) `mv web/build.off web/build`, or run the printed npm command. (4) `GET /` still answers 503 "The front end is not on disk", because `self.root` is still `None`.
- Minimal fix: when `root` is `None`, re-resolve it per request (`canonicalize(&self.named)` on that branch only, so the served path costs nothing extra). Or keep the cache and add one sentence to `not_built` (and to `Build::note`'s NOT BUILT/NO SHELL text) saying the server must be restarted after the build. Re-resolving is the more honest of the two.

### server2-2 (low; unreachable in production, reachable in-process): dropping a pass-through `ServeLock` frees the in-process key that a still-live holder owns

- File: `crates/api/src/server.rs:17582-17589` (`impl Drop for ServeLock`) together with `:17653-17673`.
- Code:
  - take: `if !held.insert(key.clone()) { return Ok(ServeLock { held: None, root: key }); }`
  - drop: `drop(self.held.take()); if let Ok(mut held) = serving_roots().lock() { held.remove(&self.root); }`
- Why it is wrong: the second in-process serve of a root gets a pass-through (`held: None`), but its `Drop` still removes the key unconditionally. The set has no owner or refcount, so whichever value drops first frees the key, and the file lock goes with the value that is `Some`.
- Repro, interleaving in one process (the case the doc at 17591-17600 says is allowed, "tests that run in parallel against the developer's real store root"):
  1. A: `take_serve_lock(R)` gives `held: Some` (file locked, key inserted).
  2. B: `take_serve_lock(R)` gives `held: None`.
  3. A drops: unlock, key removed.
  4. C: `take_serve_lock(R)` inserts the key, opens a new OFD, and `try_lock` succeeds (`Some`).
  5. B drops: removes the key that C owns.
  6. D: `take_serve_lock(R)` inserts the key, opens a third OFD, and `try_lock` fails against C's OFD. It refuses "another brutex api is already serving this store" and quotes its own pid.

  A second variant: if A drops while B is still serving, the flock is released and the key removed while B is still serving R, so a second process can take `serve.lock` and serve the same store alongside B.
- Reachability: `api::main` calls `server::run` exactly once, so a production binary never holds two `ServeLock`s. The tests I found that exercise the lock use unique scratch roots (`crate::scratch::path("serve-lock-…")`), so I found no current test that overlaps. This is a latent defect in a path the doc declares supported, not a live production bug.
- Minimal fix: replace the `BTreeSet<PathBuf>` with a `BTreeMap<PathBuf, usize>` refcount. Increment on take, including the pass-through, and decrement in `Drop`, removing at zero. Only the `Some` holder unlocks the file, and it must not unlock while the count is above 1 (otherwise the second variant above remains). Alternatively, make the pass-through return the refusal instead of `Ok`.

## Previously reported, re-checked and still present (not counted)

- hunt-api-2 (medium, KNOWN): Ctrl-C releases `serve.lock` while `spawn_blocking` sweeps and backfills keep writing. This is still present at 331b05c. `server.rs:18555-18569`: `flying.abort(); code` and then `one_server` drops at the end of the arm. `main.rs:31` is still `#[tokio::main]` with no `shutdown_timeout`. The D-0693 explicit unlock in `ServeLock::drop` releases the store before the runtime's blocking pool has drained, so a second `api serve` can start next to the ghost writers. The comment at 18562-18566, "A sweep aborted mid-append is safe by construction", is about torn appends. It does not address two writers, and `abort()` does not stop a blocking task. Nothing material to add beyond the earlier report.
- errpaths line 58 (`server.rs:17533/17691` `if let Ok(mut held)` on a poisoned lock). The lines have moved to 17585 and 17822 (`release_root`). This is still latent only: the guarded code is `BTreeSet::insert/remove`, which cannot panic.

## Checked and clean

- `take_serve_lock` order: it checks that the root exists, canonicalizes it, takes the in-process key, opens `<canonical>/serve.lock` (no truncate), takes `Flock::try_lock`, and only then stamps and truncates to the stamp's length. Truncation never happens before the lock is held. A crash between `write_all` and `set_len` leaves the new stamp on line 1, and readers quote only line 1. A crash at any point releases the OFD lock in the kernel, so no stale lock file wedges the next start. The failure paths release the in-process key, and on a stamp failure the flock too (`drop(file); release_root(&key)`).
- The serve lock opened through the canonical path, not the configured symlink: a retarget after admission cannot move it. Two processes reaching one directory through different paths, such as bind mounts, still contend on the same inode.
- The `open_in_browser` child: std opens files `O_CLOEXEC`, and D-0693's explicit unlock covers the duplicate-descriptor case anyway.
- `Assets::respond`: it reads instead of `is_file()` then `read`, so there is no exists/read TOCTOU. The `missing` counter is Relaxed and gates only the log-line cadence; it publishes no data.
- `Build::read` and `newest_under` run once at startup and feed only the banner and the first event, never a per-request decision.
- `ingest::status_json`: `paused`, `seat` and the status snapshot are three separate reads, so they can be momentarily inconsistent. The output is display-only and is refused by name when poisoned. Not a finding.
- `universe_resolve`: the universe read guard is released before the ~150-request crawl.
- `indexmap_reading` and `calendar_json_reading`: they run on the admitted blocking pool. HashMap keys are sorted before emission (33397-33398). The calendar cache is keyed by the stamp taken before the census was read; the stamp's definition (`census_now_stamped`, server.rs:3564) is outside this slice.
- `HeadDeadline` / `serve_limited`: no locks and no disk. The slot counter is constructed here, and its accounting (`Slots`) is defined before line 17000, outside this slice.

Verdict: 3 findings in the pull HTTP/capture/credential slice at 331b05c (0 high, 1 medium, 2 low). No deadlock, no lock held across `.await`, and no data race found.

Slice: crates/pull/src/http.rs, capture.rs, resolve.rs, and the credential/token re-read path with its shared state. That covers pull/src/rate.rs `Governor`, pull/src/secret.rs `reread_after_rejection`, and the live re-read in api/src/credential_law.rs `Watch`, plus its callers in api/src/server.rs, because that is where pull's `CredentialPrint` is consumed. Method: source reading only. No cargo, no edits.

## Findings

### pull1-1 (medium): a throttle named in a 2xx body backs the shared governor off twice
- Sites:
  - crates/pull/src/http.rs:3077-3086 (`settle_answer` calls `weigh_parsed`)
  - crates/pull/src/http.rs:3180-3187 (`weigh_body_parsed`)
  - crates/api/src/server.rs:9759-9768 (`with_retry`, `Step::Again`)
- Code, in pull (`window_async` → `settle_answer` → `weigh_parsed` → `weigh_body_parsed`):
  ```rust
  if named == crate::refusal::Disposition::Throttled
      && let Some(lock) = self.governor.as_ref()
  { let mut g = lock.lock()...; g.record_throttled(); }
  return Some(named);
  ```
  This becomes `Err(FetchError::VendorRefused { status /* 200 */, named: Some(Throttled), .. })`.
- Code, in api `with_retry`, on the same error:
  ```rust
  if throttled
      && status != Some(429)
      && let Ok(budgets) = site.budgets.lock()
      && let Some(Some(shared)) = budgets.get(feed as usize)
  { shared.lock()...record_throttled(); }
  ```
- Why it is wrong:
  - `step(Some(200), _, Some(Throttled), ..)` returns `throttle_ladder` → `Step::Again { throttled: true }`. The status is 200, not 429, so api records the throttle again.
  - The source's governor is the same `Arc` as `site.budgets[feed]`, because the source is built with `.sharing(shared_governor(site, feed))` (server.rs:10325).
  - So one vendor answer applies `relax()` twice: two additive steps down, and credit is zeroed.
  - D-0322 (docs/05-decisions.md:24353-24356) says the wrapper records "only when throttled is set and the status is not 429, which is exactly the set the transport misses. Neither path now records twice". That held when the transport judged on status alone. P-63/D-0950 then taught `window_async` to read a refusal from a 2xx body and record `Throttled` itself. After that, "status != 429" no longer equals "the transport missed it". The 2xx-with-named-throttle case is now counted by both.
  - This is the double-count class D-0322 measured, where the allowance collapsed and a 213-instrument pull could not finish. Here it is reached through Dhan's documented habit of putting refusals under a 200 (P-63 cites Dhan's SDK testing `status == "failure"`).
- Repro:
  - A Dhan bars request is answered `200 {"status":"failure","errorCode":"DH-904",...}`.
  - `window_async` → `weigh_body_parsed` → `record_throttled()`. permitted drops by step_of(ceiling).
  - The function returns `VendorRefused{status:200, named:Throttled}`.
  - `with_retry` → `step` → `Again{throttled:true}`. `status=Some(200) != Some(429)` → `record_throttled()` a second time on the same governor.
  - The allowance drops two steps for one refusal. Every retry attempt (up to `THROTTLE_ATTEMPTS`) doubles again.
- Fix: in `with_retry`, narrow the condition to statuses the transport does not weigh. That is non-2xx and non-429: `throttled && status.is_some_and(|s| s != 429 && !(200..=299).contains(&s))`. Alternatively, have `window_async` record a named throttle in its `!is_success()` branch and delete the api block, so "the transport owns the feedback" (D-0322's rule) holds without exceptions. Add an api test: a 200 carrying DH-904 must move the allowance exactly one step.

### pull1-2 (low): `HttpSource::sharing(None)` drops the source's own governor, and api reaches that through a poison-swallowing `.ok()?`
- Sites:
  - crates/pull/src/http.rs:602-616 (`sharing`)
  - crates/pull/src/http.rs:644-651 (`wait_for_permit`)
  - crates/api/src/server.rs:5183-5190 (`shared_governor`)
- Code:
  ```rust
  // pull
  if self.governor.is_some() {
      self.charged_by_caller = governor.is_some();
      self.governor = governor;          // None replaces the descriptor's own governor
  }
  // wait_for_permit
  let Some(lock) = self.governor.as_ref() else { return Ok(()); };
  // api
  fn shared_governor(site, feed) -> Option<SharedGovernor> {
      site.budgets.lock().ok()?.get(feed as usize)?.as_ref().map(Arc::clone)
  }
  ```
- Why it is wrong:
  - The `charged_by_caller` doc says "a caller that does not share keeps the private governor `new` built and is gated here exactly as before". `sharing(None)` is "not sharing", yet it discards the private governor. The source then has no governor at all: `wait_for_permit` returns Ok immediately, and `record_throttled`/`record_success` are skipped.
  - api's `shared_governor` turns a poisoned `site.budgets` mutex into `None` with no word. Meanwhile `await_budget` (server.rs:8585) refuses by name on the same poison. So the same fault is a loud refusal on one path and a silent ungoverned source on another: the §4 shape.
- Repro (latent: it needs a panic while `site.budgets` is held, at server.rs:8585 or 9761):
  - After that panic, every `credentials.source(feed)` → `.sharing(shared_governor(..))` (server.rs:8005, 10325, 12197) yields a source with `governor: None`.
  - Any request path that does not first call `await_budget` sends ungoverned. The discovery and rolling `get`/`post_json` calls rely on `wait_for_permit` or on the caller charging.
- Fix:
  - In pull, make `sharing(None)` a no-op: `if let Some(g) = governor { if self.governor.is_some() { self.charged_by_caller = true; self.governor = Some(g); } }`.
  - In api, have `shared_governor` return `Result` and refuse on poison, as `await_budget` does.

### pull1-3 (low): a capture is created at its final name and written in place, so a crash leaves a torn "verbatim" fixture
- Site: crates/pull/src/capture.rs:236-262 (`write_new_capture_with_stamp`).
- Code:
  ```rust
  let opened = OpenOptions::new().write(true).create_new(true).open(&path);
  ...
  if let Err(why) = file.write_all(bytes).and_then(|()| file.sync_all()) { ... remove_file ... }
  return Ok(path);
  ```
  There is no temp name, no rename, and no fsync of `captures/`.
- Why it is wrong:
  - The cleanup arm only runs when the write returns an error. If the process dies (SIGKILL, OOM, power loss) after `create_new` and before `write_all`/`sync_all` completes, the final-named file stays on disk empty or partial.
  - Nothing marks it incomplete. A 0-byte file has no header. A partial one has a `bytes: N` line followed by fewer than N bytes.
  - The module's purpose is "a request the operator spends once becomes a fixture for ever" (capture.rs:31-32), and it is the verbatim evidence for vendor-defect triage (`record_unreadable`). A torn file looks like a short vendor answer, which is exactly the misdiagnosis the module exists to prevent.
  - The directory entry is also not fsynced, so after power loss a capture reported as written may be absent.
  - No production reader exists, so the damage is limited to fixtures an operator promotes by hand.
- Repro: an api pull hits its first Dhan rolling POST. `record` creates `captures/dhan-POST-0-p<pid>-t<stamp>-c0.txt`. The process is killed during `write_all` of a multi-MB body. On restart nothing removes or flags the file, and the new process writes its captures under a new stamp beside it.
- Fix:
  - Write to `<name>.partial` with `create_new` and `sync_all` it.
  - Then `std::fs::hard_link(partial, final)`. That is no-clobber, and it fails with AlreadyExists so the next attempt is used.
  - Then `remove_file(partial)` and `File::open(dir)?.sync_all()`.
  - Alternatively, append a fixed end-trailer line and document that a file without it is torn.

## Checked and clean
- capture.rs slot budgets: `fetch_update` decides capture under the race, so two threads cannot both take the last slot. The `kept()` pre-check in `keep_first`/`keep_unreadable` is only an early exit. File names carry seq, pid and a process stamp, and `create_new` is the authority, so two threads or two processes cannot overwrite each other (covered by tests at capture.rs:209, 237). A slot spent on a failed write is counted in `REFUSED` and emitted, not hidden.
- capture.rs `process_stamp`: `OnceLock` init is race-free.
- http.rs `pooled_client` / ssm.rs `POOL`: `OnceLock<Result<..>>` caches a build error for the life of the process. That is deterministic and the error is surfaced on every call, so it is not hidden.
- http.rs governor locking: every `lock()` is a std mutex taken and dropped before any `.await` (`wait_for_permit` computes `reserve` inside a block, then sleeps). Poison is recovered with `into_inner`, deliberately and with documentation. Lock order is api `site.budgets` → governor (server.rs:8585, 9761), and pull never takes `budgets`, so there is no inversion.
- rate.rs `Governor`: `advance_to` is monotonic (cursor never moves backwards when a thread with an older `now` locks second). In `reserve` the unchecked `charge` after `advance_to(at)` is safe: `wait >= shortfall_w` for every window, and `capacity >= len_micros` because `permitted >= 1`. `monotonic_micros` uses a `OnceLock<Instant>`. `ABSORBED_MICROS` is a Relaxed counter that publishes no data.
- `charged_by_caller` stops a double withdrawal on shared governors. Feedback (`record_*`) is taken exactly once per answer in pull, apart from pull1-1.
- Credential re-read (api credential_law.rs `Watch`, pull secret.rs): `Watch` is a per-run local (server.rs:7834, 11532, 14114) driven sequentially, so no state is shared across tasks. The comparison is by `CredentialPrint` (SHA-256 of length-prefixed secrets). A re-read returning a previously rejected print halts. A rotated source is re-wrapped with the shared governor in `reread_wire`. pull's `CredentialReader::reread_after_rejection` has no production caller, as its doc says. There is no token cache anywhere: `credentialed_source` re-reads config, identity and SSM on every call, so there is no stale-token cache to serve.
- resolve.rs: `crawl` is sequential, holds no shared state, writes nothing, and returns the snapshot. `to_wire`/`digest` iterate Vecs in crawl order. The `HashSet` in `MasterIndex::resolve` feeds only a sum, so no hash order reaches the bytes or the digest. Out of angle and not counted: `HttpDocuments::body_of` (resolve.rs:~727) decodes with `String::from_utf8_lossy`, silently repairing invalid UTF-8. This contradicts the `DocumentSource` doc ("a transport that cannot produce valid UTF-8 ... has already found something worth refusing").
- Threads: every `std::thread::spawn` in http.rs, capture.rs, resolve.rs and ingest.rs is inside `#[cfg(test)]`. There are no production spawns in the slice.

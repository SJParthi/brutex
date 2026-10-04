# conc-pass8: state and failure audit of the vendor network layer (`crates/pull` and its `api` callers)

**Head:** `/home/claude/wt/zero3` detached at `1f4de71` (origin/final/all-fixes-zero).

**Verdict:** 4 new findings (0 high, 1 medium, 3 low).

- **conc8-1 (medium):** the run-level vendor-down breaker cannot trip on any path. The spot bars ladder never writes the marker. The F&O ladder writes it, but nothing on that path reads it. A proof test was run.
- **conc8-2 (low):** the `VENDOR_DOWN` control character from the F&O ladder is never stripped, so it reaches operator text and the journal.
- **conc8-3 (low):** Parameter Store faults that are permanent (`ExpiredTokenException`, `UnrecognizedClientException`, `InvalidSignatureException`, `ValidationException` and others) are classed as transport. The run then keeps going, one SSM read per instrument, and tells the operator to retry.
- **conc8-4 (low):** the Parameter Store body is read with an unbounded `.text()`.

The following are sound at this head:

- **Timeouts:** every client bounds the whole exchange, body included.
- **Body caps:** every vendor body is checked against its cap chunk by chunk.
- **Credentials (CLAUDE.md §8):** the parameter path comes from local configuration only. The value comes from Parameter Store only. Nothing mints, caches to disk or logs the value. A token that is re-read once and comes back unchanged (or comes back as an earlier dead print) halts the run loudly.
- **Governor clock:** the governor runs on the monotonic clock.

Known ids (pull1-\*, pull2-\*, equity-\*, clock-\*, autopilot-2, lifecycle-1, barflow-1, census-1) are cited and not reported again.

## Table A: the HTTP clients

| Client (file:line) | Used for | Connect timeout | Whole request/body timeout | Body cap | Redirects | Credential on it |
|---|---|---|---|---|---|---|
| `http::pooled_client` (http.rs:127-138) | every broker request (bars `window_async` :2970, `post_json` :2688, discovery `get`) | none separate; covered by the total | 30 s (`REQUEST_TIMEOUT_SECS`, :61), which reqwest applies to the body read too. A one-byte-a-minute trickle dies at 30 s | 64 MiB (`MAX_RESPONSE_BYTES`); declared length checked, then counted per chunk (`body_within`, :2474); refusal bodies capped at 8 KiB | `Policy::none` | yes (header built per source) |
| `ssm::pooled_client` (ssm.rs:174-189) | Parameter Store `GetParameter` | none separate | 10 s (`CREDENTIAL_TIMEOUT_SECS`, :125) | **none**: `.text()` at ssm.rs:879-882 (**conc8-4**) | `Policy::none` | SigV4 signature only |
| `resolve::Https::new` (resolve.rs:540-584) | index-constituent crawl | 10 s | 30 s (`DOCUMENT_TIMEOUT_SECS`, :489) | 8 MiB (`nse::MAX_DOCUMENT_BYTES`), declared length then counted | `Policy::none` | none |
| `masters::PublicFetch::new` (masters.rs:552-597) | instrument masters | 10 s | 60 s | 256 MiB, declared length then counted (:706-735) | `limited(5)`, safe because no credential is carried (masters.rs:580-591) | none (cookie jar only) |
| `cash_session_cache::download` (cash_session_cache.rs:533-583) | NSE cash master | 10 s | 30 s | 4 MiB compressed, then 32 MiB inflated (`take(MAX_EXPANDED+1)`, :465) | `Policy::none` | none |

No other crate opens a socket. `runner` refuses `reqwest` (runner/src/lib.rs:2041), and `api` uses `pull`'s transports.

## Table B: retries and backoff, per call site

| Site | Retries on | Attempts | Backoff | Jitter | Retry-After | Governor | Run-level stop |
|---|---|---|---|---|---|---|---|
| spot bars: `with_retry` (server.rs:9677), called from `fetch_chunks` :9056 and the F&O name walk :12007 | 429 (and a throttle the vendor names): exponential 1, 2, 4, 8, 16 s. 5xx: `1000·n²` ms, 5 answers. No status: `250·n²` ms | 6 (`THROTTLE_ATTEMPTS`), with a compile-time sum bound (:9387-9401) | deterministic | none | ignored; documented limit (docs/06-limits.md:912) | one permit per retry (:9696); AIMD decrease taken once (D-0322, CE-30) | **breaker never fed (conc8-1)** |
| discovery `get` and the rolling POST: `laddered` (server.rs:8469, :13996) | same `step` table | 6 | same | none | ignored | same | none on these walks; the marker it writes leaks (**conc8-2**) |
| masters `fetch` (masters.rs:1441-1487) | no status, 408/425/429, 5xx, 520-599 | 5 per URL (`ATTEMPTS_PER_URL`) | 500 ms doubling, capped at 32 s; deterministic by design (:1263-1270) | none, argued: one operator, no herd | ignored | n/a (public) | bounded by URL list × 5 |
| index crawl (`resolve`) | none inside the call; the caller re-crawls on its own schedule | 1 | n/a | n/a | n/a | n/a | n/a |
| cash master `download` | none | 1 | n/a | n/a | n/a | n/a | the caller refuses the day |
| SSM `get_parameter` | none inside the call. A **transport**-class failure refuses that instrument and the next instrument reads again (credential_law.rs:325-345) | 1 per instrument | none | n/a | n/a | n/a | no stop for transport-class failures; **permanent faults are misclassed as transport (conc8-3)** |

A retry storm across feeds is not possible. Each feed has its own shared governor (`site.budgets[feed]`), every retry waits for a permit, and the feeds are different hosts. A retried request is never a non-idempotent call: the bars GET, Dhan's historical POST and SSM `GetParameter` are all read-only queries. SSM is never retried in-call.

## Table C: credentials (CLAUDE.md §8)

| Check | Evidence | Result |
|---|---|---|
| The path comes only from local configuration | `credentialed_source` (server.rs:10276) reads `~/.brutex/credentials.toml` through `pull::config::CredentialConfig::load`. An empty or relative HOME is refused (CE-38) | OK |
| The value comes only from Parameter Store | `read_credential` (server.rs:10207) → `ssm::get_parameter`. `Credentials::Scripted` exists only under `cfg(test)` (credential_law.rs:139-148) | OK |
| Nothing mints a token | `totp.rs` computes a code and has no production caller (`no_path_outside_this_module_computes_a_code`). No token-exchange endpoint appears anywhere | OK |
| Nothing caches the token to disk | the token lives in `HttpSource` memory only and is re-read on every instrument. No `fs::write` of a credential | OK |
| Nothing logs the token | `Credential` and `HttpSource` have hand-written `Debug`. The SSM log emits `value_len` only (ssm.rs:950-957). SSM refusal bodies go through an allowlist (ssm.rs:652-689) | OK |
| A stale token is re-read once, then the run halts | `Watch::reread` (credential_law.rs:360-427). Same print gives `SameValue` and a halt. A previously dead print also halts (v3a-1). A failed re-read also halts | OK |
| Failure classification | `of_secret` sends only `Unreachable` to transport. But `get_parameter` sets `kind` from `AccessDenied`/403 and `ParameterNotFound`/404 alone (ssm.rs:886-893) | **conc8-3** |

## Table D: crash mid-pull (what is on disk, and what the next start does)

| Step dies at | On disk | Next start | Status |
|---|---|---|---|
| during the fetch (before any landing) | nothing | resume point is the store's own last held day; the chunk is re-asked | OK |
| after a prefix of chunks lands, before the rest | contiguous older prefix (`prefix_or_refusal`, server.rs:9304) | resumes from the last held day | OK by design |
| capture file being written | torn capture at its final name | | known pull1-3 |
| census or manifest append | in-place append or torn slot; `CensusLock` unlocked on error | | known pull2-1, pull2-5, census-1 |
| cash master payload before its receipt | payload without a receipt; the day is wedged | | known pull2-3 |
| shutdown during a press | pull-journal record lost | | known lifecycle-1 |
| expired F&O cell | no ledger; the resume point is the manifest key set (fnowork.rs:17-31) | discovery is re-run and held cells are skipped | OK |

## Table E: clock

| Concern | Evidence | Result |
|---|---|---|
| Governor waits | `pull::rate::monotonic_micros` (server.rs:8688); `Governor` takes monotonic micros (rate.rs:867) | immune to a clock step |
| Retry sleeps | `tokio::time::sleep` (monotonic) | immune |
| SigV4 stamp | `now_stamp` wall clock (ssm.rs:973). A clock before 1970 is a configuration fault. Skew past AWS's tolerance comes back as `InvalidSignatureException` | misclassed as transport: conc8-3 |
| Session closed / IST date rollover | wall clock, zero margin | known clock-2 |
| Autopilot feed set from boot clock; backoff deadlines on the wall clock | | known clock-1, clock-3 |

---

## conc8-1 (medium): the vendor-down breaker cannot trip on any path. The spot ladder never writes `VENDOR_DOWN`, and the only ladder that writes it is never read

- **Where:** `crates/api/src/server.rs:9824-9831` (`with_retry`, `Step::ServerDown` arm, with no marker), `:7972-7974` and `:8030` (`broker_run`, the breaker's only consumer), `:8597-8613` (`laddered` writes the marker), `:10108-10109` (the doc claims `laddered` feeds `broker_run`).
- **Code:**
  ```rust
  // with_retry, the ONLY ladder under broker_run -> broker_window -> fetch_chunks
  Step::ServerDown { answered } => {
      return Err(format!(
          "{text} — and its own side has now failed {answered} \
           time(s) on this chunk, out of {SERVER_ERROR_ATTEMPTS} ..."
  ```
  ```rust
  // broker_run
  let marked = read_markers(&why);
  vendor_down_streak = breaker_next(vendor_down_streak, marked.vendor_down);
  ```
- **Why it is wrong:**
  - `VENDOR_DOWN_INSTRUMENTS` (:10062-10076) and `SERVER_ERROR_ATTEMPTS` (:9350-9358) are documented as one decision. The 5xx ladder was lengthened to about 30 s per instrument *because* three consecutive 5xx instruments stop the run, so an outage "costs three ladders instead of 785".
  - `broker_run` is the breaker's only reader, and it gets refusals only from `broker_window` → `fetch_chunks` → `with_retry`. That path never writes `VENDOR_DOWN`. `fetch_chunks` re-prefixes `CREDENTIAL_DEAD` only (:9069, :9111). `laddered` does write the marker, but its callers (discovery `get` :8469, rolling :13996) feed `roll_every` and the name walk, which never call `read_markers`.
  - So `vendor_down_streak` is always 0, and `vendor_down_sentence` is unreachable in production.
  - A vendor outage (Kite's HTML 503, measured 2026-08-25 per the doc) costs every instrument the full 5xx ladder: at least 30 s of sleeps plus five requests. Over the ~785-instrument equity universe that is about 6.5 h or more, which is the "three-hour sleep" the constants' own note warns about.
  - It also makes autopilot-2 unreachable on the spot path: the breaker's stop is never produced.
  - The unit test `three_consecutive_vendor_failures_stop_the_run_and_a_success_clears_it` tests `breaker_next` and `breaker_trips` in isolation, so it passes.
- **Repro (ran):**
  - Setup: a throwaway test in a scratch worktree at 1f4de71, using the `credential_law_tests` harness. A loopback vendor answers 503 to everything. `broker_window` was called for NSE NIFTY, then `read_markers` on the error.
  - Output: `hits=5 reached_wire=true vendor_down=false`, finished in 30.01 s. The reason reads "...has now failed 5 time(s) on this chunk, out of 5 allowed". The marker the breaker needs is absent.
  - The worktree was removed afterwards.
- **Minimal fix:** in `with_retry`'s `ServerDown` arm, prefix `VENDOR_DOWN`, and in `fetch_chunks` lift it and re-prefix it the way `CREDENTIAL_DEAD` is lifted (the order must be `VENDOR_DOWN` then `CREDENTIAL_DEAD`, after `WIRE_REACHED`, as `read_markers` expects). Then:
  - Add a `broker_run` test with three instruments against an all-503 vendor that asserts `stopped` names the breaker.
  - Fix autopilot-2 at the same time, because the breaker stop starts reaching the autopilot.
  - Add a breaker (or strip the marker) on the F&O walks (conc8-2).

## conc8-2 (low): `laddered`'s `VENDOR_DOWN` control character is never stripped, and lands mid-sentence in operator text and the journal

- **Where:** `server.rs:8607-8612` writes it. Its comment says "The marker is stripped in `broker_run` before the reason reaches an operator or the journal". The consumers that do not strip it are `server.rs:14005-14012` (`format!("{label}: {why}")`) and the discovery path through `chain::Refusal`'s `Display` (pull/src/chain.rs:169-175, which writes `detail` verbatim).
- **Why it is wrong:**
  - No `laddered` refusal ever reaches `broker_run` or `read_markers`. Every refusal it gives on a 5xx outage carries `\u{2}`, and that character ends up after `{label}: ` in the middle of the reason.
  - The reason goes to the receipt page, the rolling log and the fixed-stride journal note, where it costs a byte and renders as an invisible control character.
  - The comment states a strip that does not happen. A marker device that leaks is the D-0351 failure mode the markers were built to avoid.
- **Repro:** not run. This is from reading the source: `grep -n "read_markers(" server.rs` shows one production call (:7972), inside `broker_run`.
- **Minimal fix:** have the F&O walks call `read_markers` on a `laddered` refusal (and give them a breaker), or have `laddered` return the verdict as a field (`Refusal { vendor_down: bool }`, the way `credential_dead` is already a field) instead of prefixing the detail.

## conc8-3 (low): permanent Parameter Store faults are classed as transport, so the run neither stops nor names the fault class correctly

- **Where:** `crates/pull/src/ssm.rs:886-893`, and `crates/api/src/credential_law.rs:100-105` (`of_secret`) and `:331-345` (`unreadable`).
- **Code:**
  ```rust
  let kind = if text.contains("AccessDenied") || status.as_u16() == 403 {
      SecretError::AccessDenied
  } else if text.contains("ParameterNotFound") || status.as_u16() == 404 {
      SecretError::NotFound
  } else {
      SecretError::Unreachable
  };
  ```
- **Why it is wrong:**
  - The same file's allowlist (`AWS_FAULTS`, :676-689) names `ExpiredTokenException`, `UnrecognizedClientException`, `InvalidSignatureException`, `MissingAuthenticationToken`, `InvalidKeyId` and `ValidationException`. Each means the identity, the clock or the path is wrong, so retrying cannot fix any of them.
  - Unless AWS happens to send one under 403 or 404, it becomes `Unreachable`, which `of_secret` turns into `Unread::Transport`. `Watch::unreadable` stops the run only for `Configuration`.
  - **UNVERIFIED (external fact, not in docs/00-charter.md):** that AWS answers these under HTTP 400. The defect does not depend on it: the classifier ignores the fault name for all six.
  - The consequence: an expired SSO or STS session, or a wrong AWS key, makes every instrument in the run do its own SSM round trip and be refused as "Parameter Store was not reached ... worth retrying". The autopilot then backs off and retries a fault only the operator can fix.
  - A wall-clock skew past AWS's signing tolerance (Table E) lands in the same arm.
  - This is the inverse of GAP2-37: there, transport was flattened into configuration; here, configuration is flattened into transport.
- **Repro:** not run (low). From reading the source: `refusal_detail` echoes the name, while `kind` is decided only by the two checks quoted above.
- **Minimal fix:** classify by fault name before status. Map `ThrottlingException`, `InternalServerError` and 5xx to `Unreachable`, and every other allowlisted name to a configuration-class kind (for example `AccessDenied`, or a new `Rejected` that `of_secret` maps to `Configuration`). Add a unit test per name.

## conc8-4 (low): the Parameter Store answer is read with an unbounded `.text()`

- **Where:** `crates/pull/src/ssm.rs:879-882`.
- **Code:** `let text = answer.text().await.map_err(...)?;`
- **Why it is wrong:**
  - Every other client in the crate bounds its body by the declared length and by a running count (Table A). `docs/07-o1-architecture.md` law 5 says to bound every input at the boundary.
  - This one is bounded only by the 10 s timeout, so a proxy or an endpoint answering with a large body is buffered whole before `value_of` parses it.
  - The host is fixed to `ssm.ap-south-1.amazonaws.com` (region enforced), so exposure is small.
- **Repro:** not run.
- **Minimal fix:** reuse the `body_within` pattern with a small cap (a `GetParameter` answer is well under 16 KiB; SSM advanced parameters are at most 8 KB of value). Refuse if the declared length is over the cap, then count chunks.

## Checked and not a finding

- **Retry-After is ignored on every ladder.** This is a documented limit (docs/06-limits.md:904-914), because the charter records no vendor header semantics.
- **No jitter.** It is argued and deterministic (masters.rs:1263-1270). The spot ladder spaces retries through the shared per-feed governor (one permit per retry, server.rs:9696), so there is no synchronized herd.
- **A trickling vendor** is bounded by the total-request timeout on every client (30 s, 10 s or 60 s), which reqwest applies to the body stream.
- **A body larger than the cap** is refused by the declared length first and then by the running count, on every client except SSM (conc8-4).
- **Decompression bomb on the cash master:** bounded at 4 MiB in and 32 MiB out.
- **Redirect credential leak:** the credentialed clients use `Policy::none`. Masters follows redirects but carries no credential.
- **SSM outage (true transport):** each instrument is refused and the next one re-reads. This is a stated design (credential_law.rs:325-330), with each read bounded at 10 s. It is not counted.

## Verification tally (known ids touched)

| Id | Status at 1f4de71 | Evidence |
|---|---|---|
| pull1-1 | FIXED | `transport_missed_throttle(status)` guards the named-throttle decrease (server.rs:9870-9880, CE-30 / D-1769) |
| pull1-3 | not re-checked | out of slice |
| autopilot-2 | moot on spot until conc8-1 is fixed | the breaker stop is unreachable (conc8-1). It becomes live again the moment conc8-1 is fixed |
| GAP2-37 | FIXED in its own direction | `Unread` split (credential_law.rs:59-64); the inverse misclass is conc8-3 |
| v3a-1 | FIXED | `Watch::dead` list checked in `admit` and `reread` (credential_law.rs:304-323, :375-393) |

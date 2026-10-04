# Pass 11: security attack surface (final/all-fixes-zero @ 1f4de71)

Read-only audit of `/home/claude/wt/zero3`, detached at `1f4de71`. Severity is rated by how exploitable each issue really is for one operator running `api serve` on their own Mac. I ran one throwaway test, in `/home/claude/wt/scratch-tds11`, to prove P11-01, and then removed that worktree and its target directory.

## Verdict

Four new findings: one medium and three low.

- **P11-01 (medium, ran):** a cross-site page can make the live pull's month writes fail. An ordinary cross-origin GET to a bar-reading route takes a shared `flock` on the month. The writer only *tries* for an exclusive lock, so it fails with `Locked` and the message "another writer holds …".
- **P11-02 (low):** `pull::ssm::Signable` is `pub` and derives `Debug`. It carries the AWS session token and the request body, which holds the SSM parameter path.
- **P11-03 (low):** the broker token and the AWS secret are never wiped from memory, and the header value is not marked sensitive.
- **P11-04 (low):** personal data is committed in tracked docs and in reachable commit metadata.

## Theme results (no new finding unless named)

| Area | Result | Evidence |
|---|---|---|
| Bind address and default | Holds | server.rs:33 `DEFAULT_ADDR` = 127.0.0.1:8080. server.rs:189 `loopback_serve_addr` refuses any non-loopback address, with no override. |
| Auth on writes, CSRF, DNS rebinding | Holds (no auth, by design: loopback only) | server.rs:16980-17054. `Host` is checked on every method. Writes need an exact `http://` `Origin` and `Sec-Fetch-Site` same-origin or absent, and forwarded headers are refused. Journaled reads refuse cross-site requests (`operation_audit::AUDITED`). No CORS header. axum 0.8.9 is built without `http2` (Cargo.lock: no `h2`), so `:authority` cannot replace `Host`. |
| Other loopback servers | Hold | `web/saved-backtest/viewer.rs:317-417` and `web/sweep-readiness/main-inspector.rs:369-470`: GET/HEAD only, exact `Host`, cross-site fetch metadata refused, no bodies. |
| Path traversal | Holds | Store paths pass through `StorePath`/`check_segment`. Static assets are decoded once, checked per segment, and canonicalised with a `starts_with` check. The `folder` POST field is the operator's own same-origin form (that is the feature). |
| Size and slowloris limits | Already recorded | P1-03-1, attacksweep-1/1b, P5-05/06. Nothing new. |
| Error bodies | Not exploitable | 409/404 bodies name absolute store paths, but no cross-origin page can read them, because there is no CORS. |
| Credential in logs, Debug, errors | One latent gap (P11-02) | `pull.ssm`/`pull.secret` events carry only region, status and length. `Credential`, `Secret`, `HttpSource` and `AwsIdentity` have redacting `Debug` impls. Tokens go only in headers (`AuthScheme` has no query form), and redirects are refused. |
| Literal `/<org>/<env>/<vendor>/` path | Clean, tree and history | `git grep` and `git log HEAD -p -G` find only the placeholders `orgone`/`testenv`/`/org/env/vendor/field`. |
| Cargo supply chain | Holds | All 182 packages come from the crates.io index, with no git sources. deny.toml sets `unknown-registry`/`unknown-git = "deny"`, `wildcards = "deny"`, `yanked = "deny"`, advisories v2 with no ignores, and a licence allowlist. `cargo deny` was not run. |
| web lockfile | Holds | lockfileVersion 3, 87 entries, every one with `integrity`, all from registry.npmjs.org, no git URLs. The only install script is `fsevents`, which is optional and macOS-only. CI uses `npm ci`. |
| Workflows | Hold (hunt-ci-12 now fixed) | ci.yml:21 `permissions: contents: read`. main-check.yml has `actions: write, contents: read`. Every `uses:` is pinned to a 40-hex SHA. No `pull_request_target`. Every `${{ }}` sits in `env:`, `concurrency`, `name` or `matrix`, never inside a `run:` body. |
| Secrets in history | Clean | 750 commits reachable from HEAD. No AKIA/ASIA keys (only AWS's documented `AKIAIOSFODNN7EXAMPLE`), JWTs, PEM keys, `ghp_`/`github_pat_`, credentials.toml, `.env` or log files. |

## New findings

### P11-01 medium: a cross-site GET holds a shared month lock that makes a concurrent pull's write fail as "another writer holds"

- **Where:**
  - crates/store/src/file.rs:1594-1595 (`open_existing` takes `Flock::try_lock_shared` for the life of the handle)
  - crates/store/src/file.rs:1417-1421 (`open_or_create` uses `Flock::try_lock`, and `WouldBlock` becomes `StoreError::Locked`, :3601-3605)
  - crates/store/src/file.rs:988: `Self::Locked { path } => write!(f, "another writer holds {}", path.display())`
  - crates/pull/src/ingest.rs:2632-2633: `BarFile::open_or_create(store_root, path, symbol_id).map_err(|why| why.to_string())?;`
  - Reader routes:
    - crates/api/src/bars.rs:330 `BarFile::open_existing(...)` (`/bars.json`, `/bars/window.json`)
    - crates/api/src/server.rs:4792 `verify_json` -> `verify::vendor` -> crates/pull/src/scrub.rs:219 `BarFile::open_existing` for every held month
- **Why it is wrong:**
  - None of these GET routes is in `operation_audit::AUDITED`. `cross_origin_refusal` returns `None` for any GET whose `Host` is `127.0.0.1:8080` (server.rs:17020-17022). So a page on any site the operator has open can call `fetch('http://127.0.0.1:8080/bars/window.json?...', {mode:'no-cors'})` in a loop. Safari and Firefox send it without a prompt.
  - Each such request holds a shared `flock` on the month it reads, for the whole read. The P1-04-01 sort or extremes window reads up to about 1.9 M bars, so that can be seconds. The current month of NIFTY/BANKNIFTY on Dhan or Groww is trivially guessable.
  - Any autopilot or `/pull/run` append to that month in the same moment gets `Locked` and fails the month with "another writer holds …", which names a writer that does not exist.
  - The autopilot classes the reason as not-store (autopilot.rs:385-391). It backs off, and after `MAX_MONTH_ATTEMPTS = 3` (autopilot.rs:100) it stalls the month (`Next::Stall`, autopilot.rs:1415-1421).
  - Writer seats (`take_seat`) exclude writers only. Readers take no seat.
  - The operator's own terminal page polling `/bars/window.json` during a live ingest hits the same collision with no attacker at all.
  - The integrity of the stored bytes holds. Availability does not, and the refusal names the wrong cause, against CLAUDE.md §4.
- **Repro (ran):** I added a throwaway store test: `open_or_create` then drop, `open_existing` (reader held), then `open_or_create` again. Output:
  - `TDS11 writer while a reader is open: Err(Locked { path: ".../bars/groww/NSE/INDEX/NIFTY/1min/2024-06.lock" })`
  - `TDS11 display: another writer holds .../2024-06.lock`
  - after the reader is dropped: `Ok(())`

  I did not run the end-to-end cross-site loop against a live pull.
- **Minimal fix:**
  - Make the writer wait a bounded time (for example a blocking `lock` with a short deadline, or retry `WouldBlock` for up to about 1 s) when the holder is shared. Alternatively, have readers copy the bounded header and records and release the lock before doing slow work.
  - Give `Locked` a reader/writer-neutral sentence ("the month is held by another open handle").
  - Consider refusing cross-site (`Sec-Fetch-Site: cross-site`) on the store-reading GETs, as journaled reads already do.

### P11-02 low: `pull::ssm::Signable` derives `Debug` while carrying the AWS session token and the parameter path

- **Where:** crates/pull/src/ssm.rs:523-535 (`pub mod ssm` at pull/src/lib.rs:192); constructed at ssm.rs:828-834
- **Evidence:**

  ```rust
  #[derive(Debug, Clone, Copy)]
  pub struct Signable<'a> { ... pub body: &'a str, pub session_token: Option<&'a str>, }
  ```

  `body` is `{"Name":"<real parameter path>","WithDecryption":true}` (ssm.rs:638-639).
- **Why it is wrong:**
  - The same file's `AwsIdentity` hand-writes `Debug` for exactly this reason (ssm.rs:209-221): "a struct holding a secret that derives Debug is one dbg! away from that secret in a log file". `Signable` holds the STS session token, which is a live credential, and the runtime path that §8 keeps out of tracked files.
  - It is `pub`, `Copy`, and sits in the signing error path. One `{:?}` in a future `SsmError` detail or a test failure message would print both.
  - There is no such call site today, so this is latent.
- **Repro (not run):** `format!("{:?}", Signable { session_token: Some("SESSIONTOKEN"), body: &body("/a/b/c/d", true), .. })` contains `SESSIONTOKEN` and `/a/b/c/d`.
- **Minimal fix:** hand-write `Debug` to redact `session_token` and `body`, the way `AwsIdentity` does, and add `Signable` to the redaction test that already covers `Credential`/`Secret`/`AwsIdentity`.

### P11-03 low: credentials are never wiped from memory, and the auth header is not marked sensitive

- **Where:**
  - crates/pull/src/http.rs:149-154 (`Credential { token: String, key: Option<String> }`)
  - http.rs:310 and :495-501 (`header_value: String`, built by `format!`)
  - crates/pull/src/secret.rs:165 (`Secret(String)`, `Clone`)
  - crates/pull/src/ssm.rs:200-206 (`AwsIdentity`, `Clone`, holds the secret key and session token)
  - http.rs:836 (header attached with no `HeaderValue::set_sensitive(true)`)
- **Evidence:**
  - `zeroize 1.9.0` is already in Cargo.lock as a transitive dependency, but no crate uses it (`grep -rn zeroiz crates` finds nothing).
  - The doc at http.rs:203-206 argues against "a second copy of the secret alive for the life of a run". Meanwhile the raw token, the `Bearer …` copy, every `Secret` clone and the AWS secret are all freed without being cleared.
- **Why it is low:** on a single-user Mac, reading this process's heap already requires the operator's own account or root. The practical exposures are core dumps or crash reports and swap. Marking the header sensitive also keeps it out of hyper's `Debug` of a `HeaderMap`.
- **Repro:** not run (memory-forensics class).
- **Minimal fix:**
  - Hold `token`/`key`/`header_value` and the AWS secret in `zeroize::Zeroizing<String>`. This needs a docs/05 entry for promoting `zeroize` to a direct dependency.
  - Build the header once as `HeaderValue::from_str(..)?` with `set_sensitive(true)`. This also closes P1-19-01.

### P11-04 low: personal data is committed in tracked docs and in reachable commit metadata (public repository)

- **Where:**
  - docs/05-decisions.md:4472-4474 (`lake v0.1.0 (/Users/parthi/IdeaProjects/brutex/crates/lake)`)
  - docs/05-decisions.md:15337, :15353, :15422, :15462, :15508 (decision headings that name `/Users/parthi/IdeaProjects/brutex/...`)
  - 26 added lines in history
  - Commit metadata reachable from HEAD:
    - 389 commits by `parthi@Parthibans-MacBook-Pro.local` (local account plus machine hostname)
    - 25 by `sjparthi93@gmail.com`
    - 20 by `subramaniaparthiban@gmail.com`
- **Why it is wrong:** the repository is public, and CLAUDE.md §8 is written around that fact. The tracked docs publish the operator's macOS account name and project layout. The commit metadata publishes two personal email addresses and the machine hostname. None of this is needed by any document, and the decision headings are meant to name repository paths, not local ones.
- **Repro (ran):** `git grep -n '/Users/parthi'`, and `git log --format='%an <%ae>' | sort | uniq -c`.
- **Minimal fix:**
  - Replace `/Users/parthi/IdeaProjects/brutex/` with repository-relative paths. A docs/05 note keeps the ledger append-only.
  - Set `user.email` to the GitHub noreply address for future commits.
  - Rewriting history is the operator's call and is not recommended unless the emails matter.

## Verification of earlier security rows at 1f4de71

| Row | Verdict | Evidence |
|---|---|---|
| hunt-ci-12 (CI token scope, mutable action refs) | FIXED | ci.yml:21 `permissions: contents: read` (D-1610), and every `uses:` is pinned to a SHA (ci.yml:29, 6987, 6991, 7515, 7533, 7797 …). |
| P1-19-01 (token never validated as a header value) | NOT FIXED | http.rs:497 `(AuthScheme::Raw, None) => Ok(held.token)` is unchanged, and there is no `HeaderValue::from_str`. |
| P1-19-02 (discovery expiry spliced before validation) | FIXED | As recorded in pass 4. Unchanged. |
| P1-19-03 (unbounded credential reads) | NOT FIXED | ssm.rs:397 `std::fs::read_to_string(path)` and ssm.rs:881 `.text()`. |
| P1-03-1 (100-continue rearms the head clock) | NOT FIXED | server.rs:17463 and :17476 `this.rearm()` on every write. |
| P1-03-2 (port-80 `Host` refused) | NOT FIXED | server.rs:17100 `authority.port_u16() != Some(local_addr.port())`. |
| P1-09-01 (fork comment credits a read-only token) | NOT FIXED | auto-merge.yml:39, :222 and :227 are unchanged. |

Tally: 1 FIXED, 1 FIXED (re-confirmed), 5 NOT FIXED.

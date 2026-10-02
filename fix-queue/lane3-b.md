# Cloud fix lane 3, second set

Handoff only. Branch `fix-queue` is never merged. Source: the batch-2 audit queue, 2 Oct 2026. Base every fix on origin/main. Do this file after lane3.md. Decision numbers for this lane come from the D-0910 to D-0959 range shared by the cloud lanes.

**Done means:** a test that fails before the fix and passes after; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` and the CI gates green; for a cost finding, the operation is O(1) or the honest bound is named in docs/06-limits.md; extreme cases (empty, one, max, overflow, rerun, corrupt input) covered by tests. One branch and one PR per item, named `fix/cloud-<id>`.

## GAP2-36 · high law · 

**Where:** `crates/api/src/server.rs:7009-7113 (broker_run loop); 9205-9234 (read_credential); 13150-13193 (F&O cell loop); 12811-12814 (fetch_rolling)`

**Finding:** The shipped pull never compares a re-read with the rejected token and never halts. Every remaining instrument or F&O cell is sent the dead credential again. 

**Evidence:**
  - A
  - l
  - l

**Expected fix and test:** Fix: implement the §8 rule on the shipped path. Either (a) `impl pull::secret::ParameterStore` over `ssm::get_parameter` and route read_credential through `CredentialReader`, calling `reread_after_rejection` when a refusal carries CREDENTIAL_DEAD / Refusal.credential_dead; or (b) have credentialed_source also return a SHA-256 fingerprint of the token (never logged). Keep `rejected: Option<[u8;32]>` in broker_run and in the F&O walk. On a credential-dead refusal, record the fingerprint and re-read once. If the fresh fingerprint equals the rejected one, set out.stopped to a halt naming the vendor and field and `break`; if it differs, continue with the rotated value. Also carry Refusal.credential_dead out of fetch_rolling instead of flattening it to a String. Pre-socket credential refusals (c

## W1-api1-2 · high cost · api

**Where:** `crates/api/src/autopilot.rs:1234`

**Finding:** FeedState::observe / reconsider / Status::json (crates/api/src/autopilot.rs:1234): per one idle pass (reconsider, stall_note, survey clone) and one GET /autopilot.json or control request (Status::json under the status mutex), the cost is O(sum of stalls) per idle pass and per status request. The sum is unbounded because the claimed retry bound is not enforced; it grows with number of stall entries, which grows by one per failed reconsideration for the life of the process (no cap, no dedup, never removed). Auditor verdict: false-o1-claim. Documented: Claimed as bounded, not as a cost: autopilot.rs:151-157 and 1279-1281 say 'MAX_MONTH_ATTEMPTS x (1 + STALL_RETRIES) = 9 attempts per stalled month per process', and docs/06-limits.md §70 repeats '9 attempts'

**Evidence:**
  - `observe` runs `self.stalls.push(Stall { month: self.frontier, attempts: self.attempts, reason: why.clone(), retried: 0, at_unix: 0 })` (1234-1244) whenever `self.attempts >= MAX_MONTH_ATTEMPTS`. The `complete` branch of `observe` (1210-1215) does not touch `stalls`, and neither does `revive` or `clear_month`.
  - The only production push is crates/api/src/autopilot.rs:1234-1244, `self.stalls.push(Stall { month: self.frontier, ... A grep over crates/ finds `stalls` mutated only by that push, by the stamping loop (1309-1313), by `get_mut(nth)` in reconsider (1336) and by a push in a test (6473). The test comment at 6276-6277 says the stall "leaves only when the month actually completes", but no code does that.

**Expected fix and test:** None

## W1-api1-7 · high bug · api

**Where:** `crates/api/src/autopilot.rs:2633`

**Finding:** survey / fly (alternating Day1 and Minute1 rungs) (crates/api/src/autopilot.rs:2633): Month/rung boundary: one monotone frontier per feed is shared by the day and minute rungs. A day pass that completes month F runs settle -> Advance and moves the frontier to F+1 (or past yesterday when day is fully held). The next minute pass calls frontier(..., state.frontier, ...), which scans only upward from the hint. Minute months below the day frontier are never examined for the life of the process, and each restart repeats this. Code path: fly passes the same `&mut feeds` to round for both rungs (2433-2447); survey calls `frontier(|key| ..., series, state.frontier, floor, yesterday)` and sets `state.frontier = at;` (2633-2644); frontier() loops `while ordinal(month) <= ordinal(last)` starting at `hint` (2222-2234). Test search: every round() call in autopilot.rs tests (lines 4831, 4903, 4943, 4956, 6181, 6428, 6481, 6506) uses one granularity.

**Evidence:**
  - *One frontier is shared by both rungs.** `fly` builds the feed table once, at crates/api/src/autopilot.rs:2397: `let mut feeds = drivable(yesterday);`. It then alternates rungs over that same table, at 2433-2447: `let rung = if day_rung_next { ...Day1 } else { ...Minute1 }; day_rung_next = !day_rung_next; ...
  - *There is one frontier per feed, not one per rung.** `FeedState` has a single `pub frontier: YearMonth` (autopilot.rs:1010). It passes that same vector to `round(&site, &mut feeds, &rung_series, rung)` for both rungs, and `day_rung_next` flips the rung each pass (lines 2433-2447).

**Expected fix and test:** None

## GAP2-37 · medium bug · 

**Where:** `crates/api/src/autopilot.rs:370-386 (classify CREDENTIAL list); 1181-1200 (observe); 921-929 (Halt::Credential)`

**Finding:** The autopilot permanently halts a feed as 'credential dead, re-read returned the same value' for any feed-wide refusal whose text says 'credential'. That includes SSM unreachable, SSM throttling or 5xx, a missing credentials.toml, and no AWS identity. 

**Evidence:**
  - R
  - e
  - a

**Expected fix and test:** Fix: decide a credential halt from run.credential_dead (the vendor's structural verdict) plus fingerprint equality from the first finding, and never from prose. Classify pre-socket credential failures on their own: SSM transport, throttle or 5xx go to the Transport ladder or stall; config and identity faults get a distinct halt whose text names the configuration and claims no re-read. Remove "returned the same value" from any path that did not compare. Test: build a FeedState and call observe twice with TickOutcome{attempted:3, reached:0, reason:Some("NIFTY: this feed's credential field \"access-token\" could not be read: ssm.ap-south-1.amazonaws.com was not reached: operation timed out")}. Assert the second call does not return Next::Halt with halt_kind Halt::Credential, and no reason con

## GAP14-57 · medium test-gap · 

**Where:** `crates/api/src/operation_audit.rs:50-73 (match), 59 (/frontier.json); tests operation_audit_tests.rs:35-47; server.rs:15440`

**Finding:** api audited_route: 11 of its 21 routes are unpinned; deleting the "/frontier.json" arm survives the whole api suite and silently drops both its audit record and its cross-site read refusal 

**Evidence:**
  - M
  - y
  -  

**Expected fix and test:** Hold the 21 routes in one `const AUDITED: [&str; 21]` that both the match and the test read. The test asserts every entry maps to itself and that every JSON/control route registered in server.rs is either in AUDITED or in an explicit exemption list.

Add a per-route test through audited_router_serving: a GET with `Sec-Fetch-Site: cross-site` must be refused and must write no journal record.

Both fail with the /frontier.json arm deleted and pass today.

## GAP5-50 · medium doc-false · 

**Where:** `docs/00-charter.md:586 (charter 4e CSCV row); crates/cli/src/population_statistics_v2.rs:5009-5012 (canonical_split_count = choose(S-1,S/2)), 5051-5062 (first_train_mask excludes bit 0), 5099 (policy digest 'train-bit-zero-absent'); crates/cli/src/population_observations_v1.rs:1305-1331; docs/05-decisions.md D-0476 (line ~30941)`

**Finding:** The CSCV implementation enumerates C(S-1,S/2) splits, one orientation per complementary pair with segment 0 always tested. The cited paper uses all C(S,S/2) splits, and charter section 4e says that enumeration is 'carried'. 

**Evidence:**
  - P
  - D
  - F

**Expected fix and test:** This needs an operator choice, because D-0476 locked the half enumeration.
(a) Enumerate all C(S,S/2) train masks, including those containing segment 0, under a new CSCV policy version and digest (the old digest names 'train-bit-zero-absent'). Test: a cli population_statistics test with S=2, N=3 and segment totals seg0=[10,9,0], seg1=[5,0,10] asserts contributing=2, bottom_half=1. Today it yields 1 and 1.
(b) Or keep the half enumeration and correct charter 4e and D-0476 so they state it is a deviation from Algorithm 2.3, with its effect on small S. Add a docs test that the charter no longer says 'carried' for the enumeration.

## GAP17-33 · medium bug · 

**Where:** `docs/04-invariants.md:merge-tree conflicts: docs/04-invariants.md, docs/05-decisions.md, docs/06-limits.md; crates/cli/src/lib.rs; crates/cli/src/sweep_wiring_tests.rs; crates/runner/src/audit.rs`

**Finding:** Every piece branch conflicts with fix/c2-final and with every other piece; landing by branch name can silently carry nothing (c3-lookahead is an ancestor of c2-final) 

**Evidence:**
  - 1
  - .
  -  

**Expected fix and test:** Fix: key landing and re-verification on commit SHAs, not branch names. A group counts as landed only when every SHA in its fixed_by rows passes `git merge-base --is-ancestor <sha> fix/c2-final`. After each merge, resolve the ledger conflicts by keeping both sides in order, then re-run the digest and table checks that read those docs, plus each row's named test on the merged tree.

Check: today, a run that lists landed groups with the SHA rule reports 0 of 11. If fix/c3-lookahead were merged now, it must still report lookahead as not landed, because it has no SHAs. With a branch-name rule it would report it as landed.

## GAP2-38 · medium test-gap · 

**Where:** `docs/04-invariants.md:248-249, 277; crates/pull/tests/unit.rs:880-990; crates/pull/src/ssm.rs:113`

**Finding:** The ledger marks P-05, P-06 and P-09 as proven, but their tests drive only the unwired CredentialReader port. Nothing pins the shipped path, and the SSM action literal is in no test. 

**Evidence:**
  - d
  - o
  - c

**Expected fix and test:** Fix: mark P-06 and P-09 partial until the first finding lands, then point them at the shipped-path tests described there. Move the SSM fixed headers into a function (for example `fixed_headers() -> [(&str,&str);3]`) that get_parameter uses. Test: `ssm::tests::the_only_action_on_the_wire_is_get_parameter` asserts the x-amz-target entry equals the literal "AmazonSSM.GetParameter" and that body(name,true) has exactly the keys Name and WithDecryption. It fails today if TARGET is changed, because no test names the literal.

## W1-api1-8 · medium bug · api

**Where:** `crates/api/src/autopilot.rs:1234`

**Finding:** FeedState::observe / reconsider (crates/api/src/autopilot.rs:1234): Idempotence and retry bound: a reconsidered month that fails again pushes a duplicate Stall with retried: 0. That gives it a fresh allowance, so retries per stalled month are unbounded (one every 6 h while idle) and the stall list grows without limit. The comment that a stall 'leaves only when the month actually completes' is not implemented anywhere. Code path: The only mutation of `stalls` is `self.stalls.push(Stall { month: self.frontier, ..., retried: 0, at_unix: 0 })` (1234-1244). No retain, remove or dedup exists (grep 'stalls' lists only push/iter/clone). reconsider resets `state.frontier = month; state.clear_month();` (1343-1344).

**Evidence:**
  - In production code, the only place a stall is ever added or removed is the push in `FeedState::observe` (crates/api/src/autopilot.rs:1234-1244): `self.stalls.push(Stall { month: self.frontier, attempts: self.attempts, reason: why.clone(), retried: 0, at_unix: 0 });`.
  - *The only write to the list is an unconditional push.** In crates/api/src/autopilot.rs:1234-1244, `FeedState::observe` runs `self.stalls.push(Stall { month: self.frontier, attempts: self.attempts, reason: why.clone(), retried: 0, at_unix: 0 })` once `self.attempts >= MAX_MONTH_ATTEMPTS`.

**Expected fix and test:** None

## W1-api1-5 · medium cost · api

**Where:** `crates/api/src/booleanjson.rs:181`

**Finding:** render_with_budget (Boolean catalog JSON route; same shape in booleanevidencejson.rs:257) (crates/api/src/booleanjson.rs:181): per one HTTP request with no `completion` pin (every first page) or any cache miss, the cost is O(body bytes) hash + decode per completion-less request or cache miss, under one process-wide mutex. A single slot, so alternating identities or models turns every request cold; it grows with serialized evidence bytes of the requested identity (for statistics and admission, including every linked catalog body). Auditor verdict: undocumented-scan. Documented: none in docs/06-limits.md (§125 covers the /trades.json and /frontier.json detail routes, not these). Module doc booleanjson.rs:2-3 says only 'Cold admission hashes and decodes the bounded body'

**Evidence:**
  - booleanjson.rs:181-197: `if asked.completion.is_none() || !cache.as_ref().is_some_and(|held| held.root == root && held.identity == asked.identity && held.budget == budget) { *cache = None; let reader = Reader::open(root, asked.identity, budget.bytes())...` Reader::open (boolean_candidate_reader.rs:63-71) calls Observation::open (boolean_observation_file.rs:63-67).
  - In crates/api/src/booleanjson.rs:181-187 the code is `if asked.completion.is_none() || !cache.as_ref().is_some_and(|held| held.root == root && held.identity == asked.identity && held.budget == budget) { *cache = None; let reader = Reader::open(root, asked.identity, budget.bytes())`.

**Expected fix and test:** None

## W1-api2-1 · medium cost · api

**Where:** `crates/api/src/calendar_of.rs:299`

**Finding:** read_days / read_minute_spans (crates/api/src/calendar_of.rs:299): per one daily-rung month read, or one minute-rung month walk, inside a derivation (cache-miss path), the cost is O(records) with one pread per record; it grows with bars in the month: every daily bar in every month, plus every minute bar (~8,250) in each month whose counter misses. Auditor verdict: undocumented-scan. Documented: module header only ('0.28 s' per instrument, 'UNVERIFIED as a measurement'); nothing in docs/06-limits.md

**Evidence:**
  - calendar_of.rs:299-300 and 395-396 both run `while index < file.records() { match file.read_record(index) {`. store/src/file.rs:1895 `read_record` goes through `read_row`, which does one `read_fully` positional read per record. `derive` (calendar_of.rs:165-181) calls `read_days` on every daily month in `months`, with no counter shortcut.
  - `read_days` (crates/api/src/calendar_of.rs:299-307) runs `while index < file.records() { match file.read_record(index) {` over every DAY_1 record. `derive` (calendar_of.rs:165-182) calls `read_days` for every month in `months`, whatever the minute counter says. `read_minute_spans` (calendar_of.rs:395-410) runs the same per-record loop over MINUTE_1.

**Expected fix and test:** None

## W1-api2-9 · medium bug · api

**Where:** `crates/api/src/calendar_of.rs:907`

**Finding:** cached (crates/api/src/calendar_of.rs:907): A corrupt or unreadable daily month is served as a run of exchange holidays: cached discards derive's Report, and json() omits Closed days Code path: calendar_of.rs:907 `let (calendar, _report) = derive(...)` throws away `report.unreadable`. read_days (:316-323) refuses the whole month and emits no telemetry. pull calendar.rs:1121 `let mut kinds = vec![DayKind::Closed; span];` makes every unobserved in-span day Closed, and json() (:952-957) skips Closed days, so /calendar.json?symbol= answers 200 with those sessions missing and no field saying why.

**Evidence:**
  - `derive` (crates/api/src/calendar_of.rs:180 and :225) pushes the failure into `report.unreadable`. `cached` at :907 discards it with `let (calendar, _report) = derive(...)`. `cached` returns only `Arc<Calendar>` (:873), so no caller can recover the reason. `read_days` counts failed `read_record` calls (:305) and returns `Err` if any failed (:316-323).
  - crates/api/src/calendar_of.rs:907 is `let (calendar, _report) = derive(store_root, vendor, exchange, segment, symbol, months);`. I grepped every `derive(` in crates/api/src: the only other calls are tests (:463, :484, :571, :727, :787, :1477, :1496).

**Expected fix and test:** None

## W1-api2-11 · medium bug · api

**Where:** `crates/api/src/calendar_of.rs:907`

**Finding:** cached / derive (reached from calendar_json and gaps_json); also folder::answer, indexmap::Published::read (crates/api/src/calendar_of.rs:907): Request flood or post-pull thundering herd: store I/O blocks Tokio async workers with no admission permit and no single-flight Code path: server.rs:30444 `async fn calendar_json` contains no .await, spawn_blocking or detail::run, so a cache miss runs derive for every spot series (the module measured 0.28 s per instrument) inline on a runtime worker. cached releases its lock before deriving (:905-907), so concurrent misses each derive. folder.rs:69-148 (read_census) and server.rs:30153-30202 (indexmap) are also inline. Impact not measured.

**Evidence:**
  - (1) `crates/api/src/server.rs:30444` `async fn calendar_json` contains no `.await` anywhere in lines 30444-30700 (checked with awk). It calls `census_now(&site)` at :30460. `census_now` runs `census::read_all` inline on a stamp change (server.rs:3315 and after), and the manifest reader's size cap is 268,468,224 bytes (06-limits D-0686 section).
  - `async fn calendar_json` (crates/api/src/server.rs:30444) contains no `.await`, `spawn_blocking` or `detail::run`; grep over lines 30444-30700 finds zero. It calls `census_now(&site)` (stat calls, and a whole manifest read when the stamp moves) and then `crate::calendar_of::cached(...)` once per spot series in the exchange branch (:30583) or once for the asked symbol (:30685).

**Expected fix and test:** None

## R9-api-law-0 · medium bug · api

**Where:** `crates/api/src/calendar_of.rs:1147`

**Finding:** This is what backlog W1-api2-9 left behind, not a repeat of it. That entry reported that an unreadable or corrupt daily month is served as holidays. Its fix (4a91b24a, D-0695's fourth repair) closed only one trigger: a daily file the census HOLDS that did not open. Two lines in the range then declare the neighbouring case correct: cached's doc and the D-0695 decision both say a month held at the minute rung and not the daily one is not a fault and is kept.

Why that month still comes out as holidays:
- derive adds days only from the daily rung. The minute walk only updates days that are already there (`get_mut`), so it adds none.
- Calendar::from_observed fills every day between the first and last day seen with `Closed`.
- A month inside that span with no daily file, while the census holds the same month at 1min or another rung, therefore becomes a run of Closed days.
- census_holds(DAY_1, M) answers false for that month, so cached keeps the calendar under the manifest stamp.
- json() leaves Closed days out, so the response is 200 with that month simply missing and no word why.

Who reads it:
- The /ingest page's isSession treats an in-span day that is absent from the calendar as "

**Evidence:**
  - The behaviour is real. I ran it on the api piece and on origin/main and got the same result, so the defect was already on main and C2 did not introduce it. Scratch run on the api piece (fix/c2f-r-api 16736f23), in a throwaway worktree and a test that was never committed: - Setup: Jan and Mar daily bars on disk, Feb held only at 1min. The holds answer for that census: Jan and Mar at 1day, Feb at 1min. - Result of `cached`: `SK2 unopened=[] kept=1 feb_kind=Closed sessions=2 ...
  - The behaviour is real, and it is identical at origin/main (96194c11). C2 did not introduce it. I checked this with a scratch probe, now deleted, run on fix/c2f-r-api. calendar_of.rs is identical on fix/c2-final: `git diff --stat fix/c2-final fix/c2f-r-api -- crates/api/src/calendar_of.rs` is empty. The fixture had NIFTY 1day bars for 2026-01 and 2026-03 and 1min bars only for 2026-02, with a full session on Feb 2. `holds` said the census had Jan and Mar at 1day and Feb at 1min. `cached` was ...

**Expected fix and test:** None

## W1-api2-2 · medium cost · api

**Where:** `crates/api/src/candidatejson.rs:167`

**Finding:** render (crates/api/src/candidatejson.rs:167): per one /candidates.json page (at most 256 rows), the cost is about 5 × O(catalog bytes) of blake3 plus decode per page; it grows with total candidate sides in the whole sealed catalog (32 bytes each, plus 40 bytes per tier): every read hashes and decodes the whole catalog.bin. Auditor verdict: undocumented-scan. Documented: none (06-limits only lists crates/cli/src/candidate_trades.rs in the 19 Sep 2026 'UNVERIFIED' audit bullet list)

**Evidence:**
  - *Call chain for one candidate page.** The route is `/candidate-trades.json` (crates/api/src/server.rs:15081), not `/candidates.json`. `render` (crates/api/src/candidatejson.rs:166) reaches `candidate_trades::read_model` five times on the candidates branch. :167, `candidate_trades::read_model(...)`. :188, `candidate_trades::tier(...)`.
  - :167 `candidate_trades::read_model(...)`. :188 `candidate_trades::tier(...)` calls `pinned` (crates/cli/src/candidate_trades.rs:764), and `pinned` calls `read_model` (:739-746). :210 `candidates_page` calls `pinned` on entry (:798). `candidates_page` calls `pinned` again on exit (:821, `let _dir = pinned(root, summary, max_bytes)?;`). :228 a second `read_model`.

**Expected fix and test:** None

## W1-api3-5 · medium bug · api

**Where:** `crates/api/src/operation_audit.rs:148`

**Finding:** request_audited (crates/api/src/operation_audit.rs:148): Detail-permit saturation (MAX_CONCURRENT = 4, shared with every detail read) at the TERMINAL step after the handler has already run. detail::run returns Saturated before spawning and drops the closure that owns `attempt`. Attempt::drop (cli/src/operation_audit.rs:448-456) then calls finish(Phase::Cancelled, 0) synchronously, fsync included, on the Tokio worker. The durable record says Cancelled/status 0 for a handler that completed (e.g. POST /backtest/run, which launched an engine task), and the real response is replaced by a 503 Code path: detail.rs:71: `let permit = Permit::try_take().ok_or(RunError::Saturated)?;`. operation_audit.rs:148-151 moves `attempt` into the closure. Lines 163-166 map any Err to `failure(..., true)`. cli operation_audit.rs:450-456: `if self.armed { ... Phase::Cancelled ... self.finish(phase, 0)`. Test search: grep 'Saturated|MAX_CONCURRENT' in crates/api/src/operation_audit_tests.rs and crates/api/tests found nothing

**Evidence:**
  - `crates/api/src/operation_audit.rs:148-151` moves the armed `Attempt` into the terminal closure (`move || { let mut attempt = attempt; attempt.finish(phase, status.as_u16()) }`) and passes it as `work` to `crate::detail::run`. `crates/api/src/detail.rs:71` does `let permit = Permit::try_take().ok_or(RunError::Saturated)?;` before `spawn_blocking`.
  - `request_audited` (crates/api/src/operation_audit.rs:141-167) runs the handler with `let mut response = handler.await;`. `detail::run` (crates/api/src/detail.rs:71) starts with `let permit = Permit::try_take().ok_or(RunError::Saturated)?;`. The limit is `MAX_CONCURRENT = 4` (detail.rs:13). `Attempt::drop` (crates/cli/src/operation_audit.rs:448-463) then does `if self.armed { ...

**Expected fix and test:** None

## W1-api5-0 · medium cost · api

**Where:** `crates/api/src/server.rs:5609`

**Finding:** ingestion_observations (called from land_bodies_scheduled, once per body in land_spot) (crates/api/src/server.rs:5609): per one landed vendor body (one chunk of one instrument) on an NSE INDEX/CASH one-minute spot pull, the cost is O(E_v log E_v) every chunk, plus O(all manifest bytes + E_all log E_all) + O(index bars in held months) on every chunk after an append, which is every chunk of a first fill; it grows with the vendor's census entries (always re-sorted); total bytes and entries of every vendor manifest whenever census_now misses; NIFTY/BANKNIFTY index bars held (the calendar is re-derived whenever the vendor manifest mtime moved). Auditor verdict: undocumented-scan. Documented: none. D-0686 (06-limits:8828-8833) lists only the once-per-pull read_all in broker_answer/recovery_spot/fno_land. A grep of 06-limits for ingestion_observations/land_spot found nothing

**Evidence:**
  - `land_spot` (server.rs:6764) runs `for bodies in landed.bodies.chunks(1) {` and calls `land_bodies_scheduled(...)` once per body (6766). `ingestion_observations` returns early only when the pull is not an NSE one-minute pull on INDEX or CASH with no contract (5597-5601). 5613 `let held = census::held_entries(std::slice::from_ref(vendor));` collects every held key and runs `out.sort_unstable_by(...)` (census.rs:754).
  - `land_spot` (server.rs:6764) does `for bodies in landed.bodies.chunks(1) {` and calls `land_bodies_scheduled` for each body. A single form submission is "up to 81 requests" (server.rs:8056). `broker_run` calls `land_spot` for each instrument (server.rs:7089), and the autopilot drives `broker_run` (autopilot.rs:3220), so this path repeats. `ingestion_observations` handles INDEX|CASH Minute1 on NSE (5597-5603).

**Expected fix and test:** None

## W1-api5-1 · medium cost · api

**Where:** `crates/api/src/server.rs:5728`

**Finding:** land_bodies_scheduled -> pull::ingest::from_window; roll_one -> pull::ingest::record_held (crates/api/src/server.rs:5728): per one landed chunk body (spot and F&O name walk), and one rolling vendor answer (Dhan offset walk, one per cell of offsets x sides x cadences x ordinals x chunks), the cost is O(manifest bytes + entries) read and decode per chunk and per rolling answer. The census install is now incremental; the read is not; it grows with size of the vendor's manifest, which grows through the backfill. Auditor verdict: undocumented-scan. Documented: 06-limits §34 documents the per-window install rewrite and says it 'is not reachable today'. That is stale: server.rs:9393-9421 (D-0136) removed the target guard. The per-call full read_census is not documented anywhere

**Evidence:**
  - `land_bodies_scheduled` loops `for (chunk, body) in bodies` and calls `done.absorb(pull::ingest::from_window(` once per body (server.rs:5706, 5728). `land_spot` feeds it `landed.bodies.chunks(1)` (server.rs:6764). The comment at server.rs:9385 puts this at "~97,524 times for a full Groww backfill".
  - crates/api/src/server.rs:5706-5735 loops `for (chunk, body) in bodies` and calls `done.absorb(pull::ingest::from_window(`. Its caller `land_spot` (server.rs:6764) walks `landed.bodies.chunks(1)`, so each chunk is its own call. `from_window` calls `from_members` (ingest.rs:1111), which reaches `from_members_inner`. That function runs `let mut census = match read_census(&census_path, vendor) {` at ingest.rs:810.

**Expected fix and test:** None

## W1-api5-3 · medium cost · api

**Where:** `crates/api/src/server.rs:1100`

**Finding:** instruments_json (+ bars_by_symbol) (crates/api/src/server.rs:1100): per one GET /instruments.json, the cost is O(U + E_all + T log T) per request, plus a HashMap pre-sized to E_all; it grows with the merged universe (every by_key entry is filtered, ~2,780); the census entries of ALL vendors (bars_by_symbol walks the deduped all-vendor list and probes one census); the tracked set (sorted). Auditor verdict: undocumented-scan. Documented: none for this walk. 06-limits §11a (530-544) covers only per-row universe_tokens and says no bench times the endpoint; 8809 lists it only as a census_now caller. The code doc at 1239-1243 states O(entries + universe)

**Evidence:**
  - Lines 1100-1109 filter every entry of `universe.read.merged.by_key` on each request. `entries` is `census::held_entries(&censuses)`, which merges the held keys of every vendor (census.rs:745-756) and dedups on `(Series, YearMonth)`. It then probes the one selected feed's census at lines 1277-1280 (`census.rows_for(&series.at(*month))`), which is O(E_all) per request.
  - server.rs:1099 takes `let universe = site.universe();`, an RwLock read guard (server.rs:4716-4720). 1100-1109 walks every key of `universe.read.merged.by_key`, a `HashMap<InstrumentKey, Entry>` (merge.rs:169). On a hit that is five `stat` calls and two `Arc` clones (3344-3364). On a miss it runs `census::read_all` and `held_entries` (3367-3368).

**Expected fix and test:** None

## W1-api5-4 · medium cost · api

**Where:** `crates/api/src/server.rs:3150`

**Finding:** bars_window_json -> bars::window (scanning sort or extremes=1) (crates/api/src/server.rs:3150): per one GET /bars/window.json whose sort is not ts, or which asks extremes=1, the cost is O(bars in range) reads, O(n) partition + O(want log want), and O(bars in range) memory per request; it grows with bars in the requested month range (up to MAX_WINDOW_MONTHS=240; bars.rs:984 puts that at 'roughly 1.9 million bars'). Auditor verdict: undocumented-scan. Documented: none in 06-limits (a grep for '/bars/window', 'bars::window', 'MAX_WINDOW' found nothing); only the bars.rs code doc

**Evidence:**
  - It allocates `let mut all: Vec<WindowBar> = Vec::with_capacity(usize::try_from(total).unwrap_or(0));` (bars.rs:953). It reads every record of every opened month with `page(file, 0, held)`, one `read_record` per row (bars.rs:417-434). It folds `with_change` over those rows, and runs `extremes_of(&all)` (bars.rs:961).
  - server.rs:3150 calls `bars::window` straight from the async handler. server.rs:3116 reads `offset: number("offset", 0)?` with no cap. bars.rs:953 allocates `Vec::with_capacity(usize::try_from(total).unwrap_or(0))`. bars.rs:956 calls `page(file, 0, held)` on every opened month. `page` (bars.rs:417) does one `read_record(index)` per record. bars.rs:1008 computes `let want = offset.saturating_add(limit);`.

**Expected fix and test:** None

## UC-20 · medium bug · api

**Where:** `crates/api/src/server.rs:14211`

**Finding:** store_html (show=gaps) and its notes (crates/api/src/server.rs:14211): Boot-snapshot staleness. After a pull by this process, /store?show=gaps renders its axis and counts from the startup census, and every /store view prints startup census notes beside fresh vendor cards Code path: 14206 `census::grid_rows(site.series.len())` and 14210-14212 `census::coverage_page(&site.series, &site.censuses, ...)`; 14257 `notes.extend(site.censuses.iter().map(census::VendorCensus::note));`. Both fields are set once in Site::new (4836, 4870-4871) and Site is shared immutably behind Arc, while the default view reads census_now (14191).

**Evidence:**
  - The default (held_only) rows and the vendor cards use it (14195-14201, 14278). 14206 is `census::grid_rows(site.series.len())` and 14210-14212 is `census::coverage_page(&site.series, &site.censuses, ...)`. `Site.censuses` and `Site.series` are plain fields (4645, 4649), set once in Site::new (4836, 4870-4871). The note at 14247-14250 says "The counters below are this process's startup read".
  - 14206-14212: the `show=gaps` branch still builds its axis and cells from `site.series` and `site.censuses`. `site.censuses` and `site.series` are filled only in `Site::new` (4835, 4870-4871). The guard test `no_handler_answers_a_request_from_the_boot_entries_snapshot` (29780-29798) only looks for `site.entries` and `self.entries`.

**Expected fix and test:** None

## W1-api6-5 · medium bug · api

**Where:** `crates/api/src/sweepevidence.rs:128`

**Finding:** render (crates/api/src/sweepevidence.rs:128): An audit's ranked evidence has more than 4096 rows, so every ranked page, including page 0, is refused with 503. Code path: cli/src/lib.rs:18225 saves `&ranked.top` with keep = `audit_keep()` = `screen_cap().max(250)`, and screen_cap's DEFAULT is 10_000 (lib.rs:9509). sweepevidence.rs:128-131 `crate::detail::window(usize::try_from(total)..., asked.page)?` -> detail.rs:254-257 refuses `total_u64 > MAX_RESULT_ROWS`.

**Evidence:**
  - crates/api/src/sweepevidence.rs:128-131, `render` calls `crate::detail::window(usize::try_from(total)..., asked.page)?`. crates/api/src/detail.rs:252-257, `window` runs the check `if total_u64 > MAX_RESULT_ROWS { return Err(...) }` (MAX_RESULT_ROWS = 4_096, detail.rs:18) before it looks at the page number at all. crates/cli/src/lib.rs:16842, `ranked_with_progress` passes `audit_keep()`.
  - (1) Cap: crates/api/src/detail.rs:18 `pub const MAX_RESULT_ROWS: u64 = 4_096;`. `window()` at detail.rs:252-258 refuses before it looks at the page: `if total_u64 > MAX_RESULT_ROWS { return Err(format!("run owns {total_u64} detail rows; one request verifies at most {MAX_RESULT_ROWS}.

**Expected fix and test:** None

## W1-api6-1 · medium cost · api

**Where:** `crates/api/src/sweeprun.rs:2014`

**Finding:** run_json (snapshot clone) + Progress::to_json (crates/api/src/sweeprun.rs:2014): per one GET /backtest/run.json poll, the cost is O(report bytes) clone under a std Mutex, plus O(report bytes) JSON escape and transfer, on every poll; it grows with bytes of the retained Progress.report. For sweep-all that is at least one line per instrument-month swept (cli/src/batch.rs:694 `writeln!(out, " REFUSED {} — {why}", row.label)` inside `for row in rows`); range and descent reports grow per rung or result. Auditor verdict: unbounded-per-request. Documented: none

**Evidence:**
  - In `run_json` at crates/api/src/sweeprun.rs:2014-2018, `let local = site.sweep.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();` runs before any branch. The slot is `std::sync::Mutex<Option<crate::sweeprun::Progress>>` (server.rs:4619), and `Progress` derives `Clone` with `pub report: Option<String>` (sweeprun.rs:85-106).
  - (1) The slot is `pub sweep: std::sync::Mutex<Option<crate::sweeprun::Progress>>` (crates/api/src/server.rs:4619). `Progress` is `#[derive(Clone)]` and holds `pub report: Option<String>` (sweeprun.rs:85, :106). (2) `run_json` clones the whole `Progress` while holding the lock, on the async worker (sweeprun.rs:2014-2018: `site.sweep.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()`).

**Expected fix and test:** None

## W1-api6-4 · medium bug · api

**Where:** `crates/api/src/sweeprun.rs:2278`

**Finding:** observe_elsewhere / claim_execution (crates/api/src/sweeprun.rs:2278): A healthy CLI telemetry log larger than SCAN_BYTES (4 MiB) whose newest 4 MiB holds fewer than 256 cli.lifecycle records makes every browser run, descend and command refuse, and makes /backtest/run.json report 'unknown'. Code path: telemetry/src/tail.rs:486-487 sets `out.hit_scan_cap = true` once bytes_read reaches max_scan_bytes, because take_line counts only matching records (tail.rs:592-596). sweeprun.rs:2257 `|| tail.hit_scan_cap` makes tail_fault Some. :2279-2286 returns `launch_clear: false`. :1680-1684 claim_execution then refuses with 'the external command evidence still reports activity or is damaged/unconfirmed'.

**Evidence:**
  - crates/api/src/sweeprun.rs:2278 runs `status_tail(dir, "cli.lifecycle", None, 256)`. That query is capped with `.scanning_at_most(crate::logs::SCAN_BYTES)` (:2240). logs.rs:51 sets `SCAN_BYTES: u64 = 4 * 1024 * 1024`. telemetry/src/tail.rs:486-488 does `if out.bytes_read >= query.max_scan_bytes { out.hit_scan_cap = true; return Stop::Stopped; }`.
  - telemetry/src/tail.rs:486-488: `if out.bytes_read >= query.max_scan_bytes { out.hit_scan_cap = true; return Stop::Stopped; }`. The walk only stops early through take_line (:564-599), and that returns true only when `out.records.len() >= limit` or the `since` floor is crossed.

**Expected fix and test:** None

## ET-bars-candles-store-1 · medium bug · pull

**Where:** `crates/pull/src/ingest.rs:2216`

**Finding:** Deriving the rungs is not incremental. Each ingest re-reads the entire stored 1-minute month and every stored derived month, then re-folds all 7 rungs. Read work per ingest grows linearly with how full the month is, and over a month filled one session at a time it is quadratic.

**Evidence:**
  - crates/pull/src/ingest.rs:1786 then :1817. ingest.rs:2214-2218. The request's own batch is not what it uses: :2236 replaces it with that history. ingest.rs:2237 loops over all 7 derived rungs.
  - ingest.rs:2199-2200 says "Resume from the committed source month, not just this request's suffix. ingest.rs:2216-2218 runs `(0..file.header().n_valid).map(|i| file.read_record(i))`. The loop is at ingest.rs:2237 and calls crate::fold::complete_minutes_with_calendar(minutes, ...) at ingest.rs:2480.

**Expected fix and test:** None

## W1-pull2-0 · medium cost · pull

**Where:** `crates/pull/src/ingest.rs:810`

**Finding:** from_members_inner (via from_window) -> read_census (crates/pull/src/ingest.rs:810): per one broker window (one from_window call; api server.rs:5728 calls it once per chunk body), the cost is Θ(census entries) per window: full fs::read of the manifest, then decode + CRC-32C + HashMap insert of every committed entry; it grows with census entries: store history, up to MAX_ENTRIES 2,097,152 entries / 268,468,224-byte file. Auditor verdict: undocumented-scan. Documented: none. It contradicts §17 'The load', which says "It is paid once per process." §34 covers only the install (write) half, and §34 is itself stale: install_census now appends positionally (ingest.rs:3173-3176).

**Evidence:**
  - crates/pull/src/ingest.rs:810 `let mut census = match read_census(&census_path, vendor) {` runs inside from_members_inner every time it is called. read_census (ingest.rs:2899) does a whole-file `fs::read(path)` (ingest.rs:2914). It then calls `Manifest::open_image(vendor, &bytes)`, which goes through open, load, walk_generations and walk (manifest.rs:2343, 2305, 2395ff).
  - *The chain is per window.** `api/src/server.rs:5728` calls `pull::ingest::from_window(body, ...)` inside `for (chunk, body) in bodies`. `land_spot` (`server.rs:6765`) goes further and calls `land_bodies_scheduled` once per body via `landed.bodies.chunks(1)`. `from_window` calls `from_members(std::slice::from_ref(&member), ...)` (`ingest.rs:1112`). `from_members` calls `from_members_inner` (`ingest.rs:448`).

**Expected fix and test:** None

## W1-pull2-6 · medium cost · pull

**Where:** `crates/pull/src/ingest.rs:1208`

**Finding:** record_all (record_held) (crates/pull/src/ingest.rs:1208): per one rolling vendor answer batch (api server.rs:11888), the cost is Θ(census entries) per call (whole-file read + decode every entry), plus O(offered); it grows with census entries. Auditor verdict: undocumented-scan. Documented: none in docs/06-limits.md. Stated in the code comment at ingest.rs:1195: "Per call: Θ(entries) for the read".

**Evidence:**
  - `record_all` (crates/pull/src/ingest.rs:1202) takes the lock and then calls `read_census(&census_path, vendor)` (ingest.rs:1208) every time. `read_census` reads the whole file with `fs::read(path)` (ingest.rs:2914), then calls `Manifest::open_image(vendor, &bytes)` (ingest.rs:2933).
  - *The scan is real and happens on every call.** `record_held` (crates/pull/src/ingest.rs:1250) calls `record_all`. At ingest.rs:1208 `record_all` calls `let mut census = match read_census(&census_path, vendor) {` on every call. `read_census` reads the whole file with `fs::read(path)` (ingest.rs:2914), then calls `Manifest::open_image(vendor, &bytes)` (ingest.rs:2933).

**Expected fix and test:** None

## GAP14-63 · low test-gap · 

**Where:** `crates/api/src/server.rs:30597, 30603`

**Finding:** api /calendar.json: the empty-calendar guard at the call site is untested, so the `from` list could name a symbol the feed holds nothing for 

**Evidence:**
  - r
  - u
  - n

**Expected fix and test:** In calendar_route_tests, add a symbol with no files for the queried feed and assert it is absent from the response's `from` list. Fails under the mutant; passes today.

## GAP12-12 · low bug · 

**Where:** `crates/pull/src/fold.rs:893 (diagnostic); rewritten at ingest.rs:2490-2500`

**Finding:** A withheld OpenLengthUnmeasured Muhurat day is reported as 'incomplete or invalid minute coverage ... restore complete minute source ... before retrying derivation' 

**Evidence:**
  - p
  - u
  - l

**Expected fix and test:** Fix: add explicit arms in complete_minutes_with_calendar. OpenLengthUnmeasured → 'session length unmeasured (charter non-regular day); derived buckets withheld; nothing to repair', emitted once per day. Closed → 'bars on a measured closed day; store defect'. Test: `an_unmeasured_length_day_is_withheld_once_with_its_own_reason` asserts diags.len()==1 and that the text contains 'unmeasured' and not 'restore complete minute source'.

## GAP2-41 · low law · 

**Where:** `crates/pull/src/ingest.rs:2072-2076 (identify)`

**Finding:** pull::ingest accepts a plan naming exchange BSE and writes BSE bars and a manifest entry (§1: BSE is not pulled) 

**Evidence:**
  - P
  - r
  - o

**Expected fix and test:** Fix: make identify (the write path) refuse Exchange::Bse with a D-0017 sentence, or make Plan.exchange an NSE-only newtype. Reading existing BSE files stays allowed (append-only history). Test: invert the probe. from_dir with exchange "BSE" must report a failure naming the venue and leave no file under bars/*/BSE. Today it writes 3 bars.

## GAP12-8 · low doc-false · 

**Where:** `crates/pull/src/vendor.rs:463-501 and 639-656 vs 658-679; also crates/store/src/path.rs:436-452, 502-510; docs/05-decisions.md:35793-35795 (D-0603)`

**Finding:** The docs still say the fold grid is midnight-anchored and 30/60min open with a stub, and store_timeframe still refuses vendor 30/60min pulls for that stale reason; the const block meant to catch the anchor move did not fire 

**Evidence:**
  - v
  - e
  - n

**Expected fix and test:** Fix: decide whether vendor-served 30/60min bars may be filed. If yes, check that the vendor stamps them on the open grid and return Some(...). If no, rewrite the refusal with its true reason and delete the 658-679 sentence. Replace the const block with a check tied to pull::fold's actual first-bar stamp, and correct store/src/path.rs:436-452 and 502-510 and the D-0603 wording (by a new D-entry, since the ledger is append-only). Test (fails today): `store_timeframe_follows_the_fold_anchor` in crates/pull/tests/anchor.rs asserts that for each intraday Granularity, store_timeframe().is_some() == (fold of a 375-minute session at that width has its first bar stamped at 09:15).

## GAP12-11 · low doc-false · 

**Where:** `crates/pull/src/vendor.rs:669-673; also fold.rs:175-176 and indicators/src/anchored.rs:618-620`

**Finding:** The docs say only 2/10/30/60 are ragged, which is true only for a 375-minute session; on the post-CAS F&O venue (385 minutes) 3min and 15min also end with 1- and 10-minute bars 

**Evidence:**
  - p
  - u
  - l

**Expected fix and test:** Fix: state the tiling per venue session length, not per rung. Test: `derived_rung_stub_minutes_follow_the_venue_session` asserts that complete_minutes_for_venue(NseDerivatives, day >= 2026-08-03) keeps a 1-minute 3min bar and a 10-minute 15min bar at the close, so any doc table must be derived from the venue row.

## GAP2-43 · low doc-false · 

**Where:** `crates/pull/src/vendor.rs:4677-4679; .github/workflows/ci.yml:731-735`

**Finding:** Comments say pull::totp mints Groww's daily token, and that TOTP recognises a stale token. Neither is true. 

**Evidence:**
  - `
  - g
  - i

**Expected fix and test:** Fix: correct both comments. Groww's api-key is not used by this build; totp.rs is an unwired code generator. Test: a source-shape test in pull (like the server.rs source tests) that walks crates/*/src and fails on any `totp::code_at` or `code_at_counter` reference outside crates/pull/src/totp.rs and cfg(test) modules. It pins that no path trades a TOTP for a token.

## W1-api1-9 · low bug · api

**Where:** `crates/api/src/audit.rs:1150`

**Finding:** Journal::appended (crates/api/src/audit.rs:1150): A store root that vanishes (for example an unmounted external volume whose mountpoint path is writable): create_dir_all(root/audit) creates every missing ancestor, root included. The pull journal is then written onto the wrong device, splitting append-only history. autopilot.rs:637-646 sets the rule 'NEVER RECREATE A MISSING STORE ROOT' for exactly this case. Code path: `if let Some(dir) = self.path.parent() { std::fs::create_dir_all(dir).map_err(...)?; }`, with path = root.join("audit").join("pull.journal") (1070-1071, 1149-1151). std::fs::create_dir_all creates all missing parents. Test search: audit.rs tests appending_creates_the_directory_and_the_count_is_the_length_divided (~2130) and the file-where-the-directory-goes test (~2361) both start from an existing scratch root;

**Evidence:**
  - *The journal creates missing directories on every append.** In crates/api/src/audit.rs:1149-1151, `Journal::appended` runs `if let Some(dir) = self.path.parent() { std::fs::create_dir_all(dir)... }`, where `path = root.join("audit").join("pull.journal")` (audit.rs:1070-1071).
  - What holds: crates/api/src/audit.rs:1149-1151 is `if let Some(dir) = self.path.parent() { std::fs::create_dir_all(dir)...?; }`, and journal_path (1070-1071) is `root.join("audit").join("pull.journal")`. The test helper `scratch` (audit.rs:1663-1667) always runs create_dir_all on the root first. (1) The "NEVER RECREATE A MISSING STORE ROOT" comment at autopilot.rs:637 belongs to the recovery probe `probe_io`.

**Expected fix and test:** None

## W1-api1-1 · low cost · api

**Where:** `crates/api/src/autopilot.rs:2446`

**Finding:** fly -> tracked_series (crates/api/src/autopilot.rs:2446): per one autopilot round (every pass: back-to-back while working, every IDLE_POLL_SECS=60 s when idle), the cost is O(U log U) filter + sort_unstable + dedup per round; it grows with size of the merged master universe (site.universe().read.merged.by_key; 2,787 real, 50,000 in the bench fixture). Auditor verdict: false-o1-claim. Documented: none in docs/06-limits.md. The code claims the opposite at autopilot.rs:2134-2136

**Evidence:**
  - The doc comment at crates/api/src/autopilot.rs:2134-2136 says the list is "Computed once when the autopilot starts ... That call filters the whole `site.universe().read.merged.by_key` through `catalog::tracked`, then runs `out.sort_unstable(); out.dedup();` (lines 2139-2159).
  - `tracked_series` (crates/api/src/autopilot.rs:2138-2160) filters `site.universe().read.merged.by_key` through `catalog::tracked`, then does `out.sort_unstable(); out.dedup();`. `fly` calls it on every pass of the loop: `let rung_series = tracked_series(&site, rung_timeframe);` at :2446. The pre-loop call at :2396 only feeds `opening.instruments`.

**Expected fix and test:** None

## W1-api1-4 · low cost · api

**Where:** `crates/api/src/booleancampaignjson.rs:95`

**Finding:** render_qualified (qualified campaign JSON route) (crates/api/src/booleancampaignjson.rs:95): per one HTTP request (campaign state page), the cost is O(H) snapshot reads + 2H decodes in QualifiedCampaign::open, plus H more snapshot reads and payload compares in require_current. No cache; it grows with acknowledged checkpoint count H of that campaign's history. Auditor verdict: undocumented-scan. Documented: none in docs/06-limits.md

**Evidence:**
  - What the code does on each GET of /boolean-qualified-campaign.json, from booleancampaignjson.rs:94-117 and boolean_qualified_observer.rs: Snapshot::open runs discover_through (search_checkpoint.rs:430-476). It checks every reservation directory, interrupted holes included, up to DIRECTORY_LIMIT = 1,000,000 (search_checkpoint.rs:14).
  - `render_qualified` (crates/api/src/booleancampaignjson.rs:95) calls `QualifiedCampaign::open(root, asked.identity, crate::detail::MAX_SCAN_BYTES)`. crates/cli/src/boolean_evidence.rs:2 re-exports that type as `boolean_qualified_journal::Reader`. The walk at :57-90 does one `snapshot.read` and one `decode` per record. The replay at :101-110 does a second `decode` per record.

**Expected fix and test:** None

## W1-api2-0 · low cost · api

**Where:** `crates/api/src/calendar_of.rs:186`

**Finding:** derive (crates/api/src/calendar_of.rs:186): per one calendar derivation for one instrument (runs on every calendar_of::cached miss: after every manifest rewrite, and on every call when the feed has no manifest stamp), the cost is O(M × D) calls to in_month → Day::from_days, i.e. O(span²); it grows with months in the span × trading days in the span (quadratic in history length); /calendar.json without a symbol repeats it for every spot series the feed holds (~205). Auditor verdict: undocumented-scan. Documented: none — the module header says 'UNVERIFIED as a measurement'; the D-0686 section of docs/06-limits.md names only the calendar_of::cached lookup, not derive

**Evidence:**
  - The code at crates/api/src/calendar_of.rs:185-199 (tree C2 @ 9f5c13c7) is O(M x D). When the month count matches (lines 194-198), `for (day, slot) in &mut traded { if in_month(*day, *month) {` walks the whole map again. `in_month` (line 243) calls `day_month`, which calls `pull::session::Day::from_days` (session.rs:581-621).
  - *Where the extra work is.** `crates/api/src/calendar_of.rs:164` builds one `traded: BTreeMap<i64, Vec<(u16,u16)>>` that holds every daily-bar day in the whole span. Lines 194-197 run `for (day, slot) in &mut traded { if in_month(*day, *month) {` again for every month whose counter matched.

**Expected fix and test:** None

## W1-api2-3 · low cost · api

**Where:** `crates/api/src/candidatejson.rs:298`

**Finding:** trade_page (crates/api/src/candidatejson.rs:298): per one exact candidate trade page, the cost is O(trades of candidate) per cold open, then O(page) warm; it grows with trades held by the selected candidate (every row is re-read, re-hashed and financially re-summed on each key change); a single-slot cache means alternating between two candidates re-validates on every request. Auditor verdict: undocumented-scan. Documented: none in docs/06-limits.md (only the cli doc comment 'cold open checks every row')

**Evidence:**
  - crates/api/src/candidatejson.rs:285 keeps a single slot, `static CACHE: OnceLock<Mutex<Option<Cached>>>`. Any change of key (`|| held.key != key`, :296) re-runs `TradeReader::open(root, summary, key, crate::detail::MAX_SCAN_BYTES)` (:298). That open reaches crates/cli/src/candidate_trades.rs:940, `for seq in 0..count { ...
  - crates/api/src/candidatejson.rs:285-309 holds a single process-wide slot (`static CACHE: OnceLock<Mutex<Option<Cached>>>`). The cold open in crates/cli/src/candidate_trades.rs:940-945 visits every row: `for seq in 0..count { ... candidate_trades.rs:929 `if len != expected || len > max_bytes` refuses larger files, and max_bytes is `MAX_SCAN_BYTES: u64 = 64 * 1024 * 1024` (crates/api/src/detail.rs:16).

**Expected fix and test:** None

## R9-api-cx-1 · low bug · api

**Where:** `crates/api/src/census.rs:392`

**Finding:** `sized` calls `std::fs::metadata(path)?.len()` and then `std::fs::read(path)`. That is a second, blocking O_RDONLY open with no `is_file` check and no read bound.

Three consequences:
- **A FIFO at a manifest path blocks until some writer opens it.** `manifest_stamps` stats a FIFO as `ManifestStamp::At` (server.rs:3558), so a cold or moved key goes straight to the read. The census lock is released before the read (server.rs:3428, 3463), so every concurrent request that misses blocks too. Every census-backed handler runs synchronously on a Tokio worker; for example `calendar_json` calls `calendar_json_reading` directly (server.rs:30972-30976). Worker by worker, /calendar.json, /gaps.json, /bars and /store stop answering.
- **Boot hangs silently.** `Site::load` calls `census::read_all(store_root)` at startup (server.rs:5232), so the server hangs before serving anything, with no refusal printed.
- **A character device (for example a symlink to /dev/zero) reports len 0, passes the MAX_MANIFEST_BYTES check, and is read without bound.** More generally, the bound applies to what `metadata` saw, not to what the second open reads: a file swapped in between the two calls is read whole.

The 

**Evidence:**
  - The defect is real, and it is already on main. `census::sized` (crates/api/src/census.rs) runs `std::fs::metadata(path)?.len()` and then `std::fs::read(path)`. It never checks the file type and never opens without blocking. `git show origin/main:crates/api/src/census.rs` has the same body at line 345. The api piece's census.rs diff only adds `Fault` and `Census::of_io_error` around the `sized` call; `sized` itself is unchanged, and fix/c2-final leaves census.rs as it is on main. I confirmed it ...
  - The route to the hang is also on main: - At boot, `Site::load` calls `census::read_all` (main server.rs:4834-4838). On main, `calendar_of` already reached `BarFile::open_existing` through `crate::bars::open`, and `open_existing` does a plain `File::open` there (store/src/file.rs:1126).

**Expected fix and test:** None

## W1-api3-0 · low cost · api

**Where:** `crates/api/src/operation_audit.rs:93`

**Finding:** note_request / request_audited (-> cli::operation_audit::begin + Attempt::finish) (crates/api/src/operation_audit.rs:93): per one HTTP request to any audited route, including read-only polled GETs (/live.json, /backtest.json, /engine/top.json, /trades.json, /frontier.json, ...), the cost is fixed code work (1 index append, 1 create_new file, about 7 fsyncs at begin, 1 or more at finish), but the per-request directory insert and lookup depend on the filesystem and on the directory's entry count, and disk use is O(history); it grows with total audited requests ever served: one new file per request in the single flat directory audit/invocations-v1/, plus 256 B appended to index.bin; no retention, rotation or sharding. Auditor verdict: needs-measurement. Documented: none in docs/06-limits.md. docs/05-decisions.md D-0568 says 'total storage grows with history and filesystem latency is not constant-time assurance'. 06-limits §124 covers admission refusals, not this growth

**Evidence:**
  - Production serving uses `audited_router_serving` (crates/api/src/server.rs:16538). That router layers `crate::operation_audit::note_request` (server.rs:14795-14797). `audited_route` (crates/api/src/operation_audit.rs:50-72) journals read-only GETs, including "/live.json", "/backtest.json" and "/backtest/run.json".
  - api/src/operation_audit.rs:50-72 journals read-only GETs, e.g. server.rs:14788-14798 layers `note_request` over the whole route table. Every admitted request runs cli/src/operation_audit.rs:513-581 `begin`.

**Expected fix and test:** None

## W1-api5-2 · low cost · api

**Where:** `crates/api/src/server.rs:3367`

**Finding:** census_now (cache miss) (crates/api/src/server.rs:3367): per one HTTP request to a census-backed route (/store.json, /instruments.json, /verify.json, /calendar.json, /gaps.json, /bars, /store, /audit.json) after any manifest mtime moved, the cost is hit: 5 stats + one lock (bounded); miss: O(manifest bytes + E log E); it grows with total bytes of all vendor manifests (up to census::MAX_MANIFEST_BYTES each) and total entries (sorted); a /store.json miss also re-encodes O(entries) and hashes O(body). Auditor verdict: false-o1-claim. Documented: 06-limits 'Census-backed GET routes ... D-0686' (8753-8838) and the §41.1 correction

**Evidence:**
  - `crates/api/src/server.rs:3367` runs `census::read_all(&site.store_root)`. That calls `read_vendor`, then `sized`, which does `let bytes = std::fs::read(path)?;` at `crates/api/src/census.rs:353`. The cap is `MAX_MANIFEST_BYTES` = 268,468,224 bytes per vendor (census.rs:85-88). `server.rs:3368` calls `held_entries`, which ends with `out.sort_unstable_by(...)` and `out.dedup()` at census.rs:754-755.
  - A cache miss runs `census::read_all` (server.rs:3367) and then `census::held_entries` (server.rs:3368). `sized()` does a whole-file `let bytes = std::fs::read(path)?;` (census.rs:353), capped at MAX_MANIFEST_BYTES = 268,468,224 (census.rs:85-88). `held_entries` does `out.sort_unstable_by(...)` and `out.dedup()` (census.rs:754-755).

**Expected fix and test:** None

## W1-api5-5 · low cost · api

**Where:** `crates/api/src/server.rs:30222`

**Finding:** calendar_json (crates/api/src/server.rs:30222): per one GET /calendar.json (with or without ?symbol=), the cost is O(E_v log E_v + S * (stat + days)) per request; it grows with the asked feed's census entries (collected, sorted, deduped); the number of spot series held (one stat, three String allocations and a deep calendar clone each); total calendar days (agree). Auditor verdict: false-o1-claim. Documented: 06-limits D-0686 (8779-8788) states the true cost

**Evidence:**
  - *The cost is real, per request, and grows with the store.** Both branches run inside `calendar_json` (server.rs:30444), as the claim says. server.rs:30568-30572 does `fresh.iter().filter(|census| census.vendor == feed).flat_map(|census| census::held_entries(std::slice::from_ref(census))).collect()`. `held_entries` (census.rs:745-757) extends with every held key, then runs `out.sort_unstable_by(..)` and `out.dedup()`.
  - crates/api/src/server.rs:30210-30224 is the `/// GET /calendar.json` doc, ending with "One `stat` and one map probe on a hit; a re-derivation only after the vendor's manifest has been rewritten". 30568-30572 run `fresh.iter().filter(|census| census.vendor == feed).flat_map(|census| census::held_entries(std::slice::from_ref(census))).collect()`.

**Expected fix and test:** None

## W1-api5-6 · low cost · api

**Where:** `crates/api/src/server.rs:14194`

**Finding:** store_html (held-only view with any filter) (crates/api/src/server.rs:14194): per one GET /store with kind=, symbol=, from= or to=, the cost is O(E) filter plus a copy of the kept entries per request (O(page) when unfiltered); it grows with census entries of all vendors. Auditor verdict: undocumented-scan. Documented: none that says so. §32 records the copy and allocation defects that were fixed, not that a filtered request still walks and copies every entry.

**Evidence:**
  - crates/api/src/server.rs:14193-14195 takes the held-only branch and calls `let kept = census::filtered(&entries, &filter); let total = kept.len();`. census.rs:896-901 builds `Cow::Owned(entries.iter().filter(|&&(series, month)| filter.keeps(&series, month)).copied().collect())`. The borrowed O(1) branch runs only when `filter.is_empty()` (census.rs:893).
  - `store_html` (crates/api/src/server.rs:14137) is the handler for the `/store` route (server.rs:15031, `store_get` at :14079). The held-only branch calls `let kept = census::filtered(&entries, &filter);` at server.rs:14194. `census::filtered` (census.rs:875-902) borrows when `filter.is_empty()`. `store_filter` (server.rs:14300-14323) builds those fields from the query string, so any of them makes the filter non-empty.

**Expected fix and test:** None

## W1-api5-7 · low cost · api

**Where:** `crates/api/src/server.rs:4070`

**Finding:** verify_json -> verify::vendor (crates/api/src/server.rs:4070): per one GET /verify.json?feed=, the cost is O(log length) + O(E_v) file opens per request; it grows with the manifest's append-log length (Manifest::newest builds a HashSet over the whole log) and held entries (each opens a bar file and reads its header plus two records). Auditor verdict: undocumented-scan. Documented: none. 06-limits:8809 names /verify.json only as a census_now caller; the code doc (4032-4040) says 'O(1) per entry ... UNVERIFIED'

**Evidence:**
  - `verify_json` is registered at crates/api/src/server.rs:14955 as `.route("/verify.json", axum::routing::get(verify_json))`. verify.rs:194 runs `for entry in manifest.newest() {`. `Manifest::newest` (crates/pull/src/manifest.rs:2738-2750) allocates `HashSet::with_capacity(self.index.len())` and walks `self.log.iter().rev()`.
  - server.rs:4061 `let (censuses, _entries) = census_now(&site);` is cached on manifest modified times since D-0686. server.rs:4070 `let report = crate::verify::vendor(&site.store_root, census);`, then verify.rs:194 `for entry in manifest.newest() {`. manifest.rs:2738-2750 walks `self.log.iter().rev()` on every call and nothing caches the result.

**Expected fix and test:** None

## W1-api5-8 · low cost · api

**Where:** `crates/api/src/server.rs:2291`

**Finding:** bars_json (bisection fallback) (crates/api/src/server.rs:2291): per one GET /bars.json whose from= is after the month's last stored bar, the cost is O(n_valid) reads to return [] (O(log n) otherwise); it grows with bars in the month (n_valid). Auditor verdict: undocumented-scan. Documented: none in 06-limits (the code comment at 2286-2290 states it)

**Evidence:**
  - The bisection is `store::file::first_at_or_after` (file.rs:2397-2418). server.rs:2291-2295 is `let begins = from_micros.and_then(|at| file.first_at_or_after(at).ok()).and_then(|index| usize::try_from(index).ok()).filter(|&index| index < held).unwrap_or(0);`. `to` is at least `from` (`day_window_bounds`, server.rs:1993), so its bisection also returns n_valid.
  - crates/store/src/file.rs:1906-1908 says "`n_valid` when every bar is older, which is the correct answer for a window starting past the end of the month". The helper at file.rs:2397-2418 starts with `high = n_valid` and runs `low = mid+1` whenever `stamp < ts`, so it returns n_valid. crates/api/src/server.rs:2291-2295 runs `.filter(|&index| index < held).unwrap_or(0)`, so begins = 0.

**Expected fix and test:** None

## W1-api5-9 · low cost · api

**Where:** `crates/api/src/server.rs:30171`

**Finding:** indexmap_json (crates/api/src/server.rs:30171): per one GET /indexmap.json, the cost is O(file + U) per request; it grows with bytes of nse_indices.csv (re-read and re-parsed per request) and the merged universe (by_key is filtered). Auditor verdict: undocumented-scan. Documented: none in 06-limits (the code doc at 30146-30152 declines the §3 rule 4 bound)

**Evidence:**
  - crates/api/src/server.rs:30171-30173 runs `masters_dir().map(|dir| dir.join("nse_indices.csv")).and_then(|path| crate::indexmap::Published::read(&path))`. `Published::read` (crates/api/src/indexmap.rs:41-49) calls `std::fs::read_to_string(path)` and then `from_text`. `from_text` rebuilds a `Catalogue` HashMap over every line (indexmap.rs:56-66, pull/src/nseindex.rs:166-177).
  - The route is registered at crates/api/src/server.rs:14938 as `.route("/indexmap.json", axum::routing::get(indexmap_json))`. Each call runs server.rs:30171-30173, `masters_dir().map(|dir| dir.join("nse_indices.csv")).and_then(|path| crate::indexmap::Published::read(&path))`.

**Expected fix and test:** None

## W1-api5-11 · low cost · api

**Where:** `crates/api/src/server.rs:6517`

**Finding:** spot_targets (POST /pull/spot, recovery_mapping) and resolved_master_rows (POST /universe/resolve) (crates/api/src/server.rs:6517): per one pull POST / one resolve POST, the cost is O(U) / O(U log U) per request; it grows with merged universe size whatever the target (target=Swept still walks every entry); resolve also sorts. Auditor verdict: undocumented-scan. Documented: none in 06-limits (resolved_master_rows' own doc at 29494-29495 says 'O(n log n) preparation')

**Evidence:**
  - (1) spot_targets, crates/api/src/server.rs:6513-6560, walks the whole merged universe on every call, whatever the target. `by_key` is a `HashMap<InstrumentKey, Entry>` (merge.rs:169) with no per-target index. So target=Swept (`key.is_sweepable()`, ingest.rs:231) walks every entry to return 2 keys. The comment at 6541-6543 ("ONE HASH PROBE PER CANDIDATE ...
  - (1) spot_targets, crates/api/src/server.rs:6513-6560. `by_key` is a `HashMap<InstrumentKey, Entry>` (merge.rs:169). Callers: `broker_run` (server.rs:6940, reached from the hand POST at 5772/7137 and from autopilot.rs:3220), and `recovery_mapping` (server.rs:7173-7174, called from recovery.rs:1238 `assess`). (2) resolved_master_rows, server.rs:29496-29511.

**Expected fix and test:** None

## R9-api-cx-2 · low bug · api

**Where:** `crates/api/src/server.rs:16170`

**Finding:** `take_serve_lock` opens serve.lock with `.truncate(false)` and writes `addr=.. pid=..\n` at offset 0 once the lock is held. When the previous holder's stamp was longer, its tail survives. The refusal arm reads the whole file with `read_to_string(&path)` and prints it trimmed as "held by", so the operator is shown a second, stale pid as though it held the store.

The code's own claim is false in that case: "STAMPED AFTER THE LOCK IS HELD, so the value a refused instance reads was written by the instance that actually holds it" (server.rs:16212-16213).

The lines predate this range. D-0693 (c355840f) rewrote the lock around them and left the write as it was.

**Evidence:**
  - I tried to refute this and could not. The defect is real, but it already exists on main. What the code does on fix/c2f-r-api (crates/api/src/server.rs, and at the same lines on fix/c2-final and fix/c2f-r-cli, around 15785/15826): - `take_serve_lock` opens serve.lock with `.truncate(false)`. - It takes the lock, then writes `addr={addr} pid={pid}\n` at offset 0 with `write_all(&mut &*file, ..)`. Nothing calls `set_len`. - `ServeLock::drop` only unlocks the file and removes the key. It never ...
  - I could not refute it; the defect is real. In fix/c2f-r-api, crates/api/src/server.rs `take_serve_lock` opens serve.lock with `.read(true).write(true).create(true).truncate(false)` (line 16174). Once it holds the lock it writes `addr={addr} pid={pid}\n` at offset 0 with `std::io::Write::write_all(&mut &*file, ...)` (line 16215). It never calls `set_len`, and neither `ServeLock::drop` nor the new `store::flock::Flock` (crates/store/src/flock.rs, whose drop only unlocks) truncates or removes the ...

**Expected fix and test:** None

## W1-api6-2 · low cost · api

**Where:** `crates/api/src/sweeprun.rs:1763`

**Finding:** run_with (same critical section in descend_with :1923 and command_with_configuration :2952) (crates/api/src/sweeprun.rs:1763): per one POST admission; also every concurrent run.json poll, which takes the same std Mutex on a Tokio worker, the cost is lock hold time equals admission I/O latency, and a poll can wait that long on an async worker; it grows with filesystem I/O done while the slot mutex is held: 2x canonicalize, lease acquire, observe_elsewhere (<= 8 MiB log scan), booleanlaunch/indexstoplaunch::prepare (unmeasured), operation_audit::begin (fsync), telemetry marker write. Auditor verdict: needs-measurement. Documented: §124 says admission latency depends on the telemetry sink and filesystem. Holding the mutex across that I/O, and the Tokio worker blocking on it, are not documented.

**Evidence:**
  - In run_with the guard is taken at sweeprun.rs:1763 (`let mut held = match site.sweep.lock()`) and released only after `*held = Some(accepted.clone());` at :1803. claim_execution (:1663-1688).
  - run_with takes the lock at crates/api/src/sweeprun.rs:1763 with `let mut held = match site.sweep.lock() {`. `claim_execution(site, true)` at :1780. `cli::preflight_store_root()` (:1668) `require_same_execution_store`, which calls `std::fs::canonicalize` twice (:1693-1701)

**Expected fix and test:** None

## W1-api6-3 · low cost · api

**Where:** `crates/api/src/topjson.rs:150`

**Finding:** report -> SELECTION.with_verified (persistent-refusal path) (crates/api/src/topjson.rs:150): per each GET /engine/top.json while one ledger record's seal is damaged, or while any other refusal persists on disk, the cost is O(history) per request, then 503; it grows with results-ledger history: the cold index walk plus seal-verified re-reads from record 0 up to the bad one, repeated on every request. Auditor verdict: undocumented-scan. Documented: none. detail.rs:338-344 calls the reopen 'the single O(rows) path a cache should ever take', but nothing says it repeats on every request while the damage persists.

**Evidence:**
  - In crates/api/src/detail.rs `with_verified`, lines 431-433 are `if let Err(why) = refresh(handle) { *held = None; return Err(why); }`, and line 437 is `let (_, handle) = held.insert((root.to_path_buf(), open()?));`. topjson.rs:150 calls `SELECTION.with_verified(root, || Selection::open(root), ...)`.
  - topjson.rs:150 calls `SELECTION.with_verified(root, || Selection::open(root), ...)`. When the slot is empty, detail.rs:437 runs `let (_, handle) = held.insert((root.to_path_buf(), open()?));`. The warm path (detail.rs:428-433) also resets it: `if let Err(why) = refresh(handle) { *held = None; return Err(why); }`. `Selection::open` (topjson.rs:23-31) first calls `Results::open_read_bounded`.

**Expected fix and test:** None

## UC-6 · low bug · ci

**Where:** `docs/05-decisions.md:16655`

**Finding:** Claimed: 'The only non-Rust file left anywhere in the graph is libc's etc/libc-util.py' Actual: The host graph (155 packages) ships 15 non-Rust files in 6 crates. None is .c/.h/.S/.asm/.js; none is referenced by a build script I could find (parquet, parquet-format-safe, tracing and untrusted have no build script in the host metadata).

**Evidence:**
  - The sentence exists as quoted, at docs/05-decisions.md:16655-16657 (D-0211) and docs/06-limits.md:3029 (§42). ring 0.17.14 is still in Cargo.lock as an optional dependency of rustls-webpki (Cargo.lock:1081), and crates/vocab/tests/workspace_is_rust.rs:58-100 declares it, together with cc, wit-bindgen and iana-time-zone-haiku.
  - docs/05-decisions.md:16655-16657 (D-0211) says: "The only non-Rust file left anywhere in the graph is `libc`'s `etc/libc-util.py`". docs/06-limits.md:3028-3030 (§42) repeats it in the present tense. Grepping docs/, crates/, CLAUDE.md, AGENTS.md and .github for the other script names or "non-Rust file" turns up only these two lines and a ring comment in workspace_is_rust.rs:12.

**Expected fix and test:** None

## AC-whp-o1-1 · low cost · cli

**Where:** `docs/06-limits.md:146`

**Finding:** §5 'Ranked streaming removes retention' claims three things:
- the ranked path no longer returns survivor vectors;
- ranking's transient peak is 'never O(total frequent combinations)';
- 'only ranked entry points take the streamed result'.

sweep-stored and sweep-audited-stored are ranked entry points. Both go through and_checkpoint::run, which walks with the Retaining checkpoint sink (every level pushed into `levels`). runner::rank_checkpointed_sweep then ranks the complete retained Sweep after the walk ends. So the retention this section says was removed is live on the stored door, which docs/24-checksum-admission.md:246 calls the shared path for both stored doors.

docs/20-sweep-resume.md:24 says 'Encoding uses fixed scratch space'. That is true of engine CheckpointView::write_to. It is false for the only production caller: and_checkpoint::encode collects the whole payload into a growing Vec (BoundedBytes, up to 64 MiB) before publish. On recovery, journal.latest reads the full payload into memory and decode then rebuilds every Frontier.

**Evidence:**
  - fix/c2-final:docs/06-limits.md:146 says "The ranked path no longer returns those survivor vectors" and :152 says its "engine result is therefore O(depth)". Both stored ranked doors take a different path: - cli/src/lib.rs:3103 (sweep-audited-stored) and :3159 (sweep-stored) both call stored_month_kernel. - and_checkpoint.rs:124 calls ladder.walk_checkpointed.
  - **Code path, checked at fix/c2-final:** - `sweep-stored` (lib.rs:3159) and `sweep-audited-stored` (lib.rs:3103) both call `stored_month_kernel`. - That kernel calls `and_checkpoint::run` at lib.rs:3415. On main the same call is at lib.rs:3223.

**Expected fix and test:** None

## AC-gates-o1-4 · low cost · docs

**Where:** `CLAUDE.md:315`

**Finding:** CLAUDE.md §5 reads: '`cli` holds no loop over bars and none over candidates: it is the structural boundary, one event per run and one per instrument-month'. This is the reason Gate 17 silences only `vocab engine indicators runner` (ci.yml:2763) and never scans cli. But `screen` walks `by_evidence.par_iter()` over every candidate (cli/src/lib.rs:11722-11723). It calls `progress.tick()` per candidate (:11821), and tick does a `fetch_add` and emits `note_grid_progress` every `stride` candidates (:17718-17726). The code's own doc says so at cli/src/lib.rs:17748-17750: 'The first half is true and the second is not: `screen` runs `by_evidence.par_iter()` over every candidate'. Five documents still state the false half.

**Evidence:**
  - fix/c2-final:CLAUDE.md:313-317 reads "Gate 17 silences `vocab engine indicators runner`, because those hold the loops ... - on origin/main: CLAUDE.md:312, AGENTS.md:276, crates/cli/src/lib.rs:2855 and :17228, docs/05-decisions.md:27864 - on fix/c2-final: CLAUDE.md:315, AGENTS.md:276, lib.rs:2935 and :17636, 05-decisions.md:27864
  - **Where the claim still stands.** - CLAUDE.md:315 and AGENTS.md:276 both say "`cli` holds no loop over bars and none over candidates: it is the structural boundary".

**Expected fix and test:** None

## AC-whp-tb-3 · low bug · docs

**Where:** `docs/04-invariants.md:2608`

**Finding:** The crossing family is tracked by `last_side` and `crossings_seen` (evaluator.rs:360 and 381). They are reset at the rollover (evaluator.rs:866-872). There is no `previous_mask` anywhere in crates/.

Each row fails differently:
- **CX-01** says `crossings_of` 'reads `previous_mask` ... and nothing else'. It reads `self.last_side` and `self.crossings_seen`.
- **CX-02** gives as proof 'the rollover sets `previous_mask = None`' (no such code) and `suffix_independence` / `two_runs_of_one_slice_agree_exactly`. Both run `bits_over` (invariants.rs:63-77), which drives only CurDayFib, Patterns and SessionState, never the `Evaluator`, so no crossing bit is ever computed. Each also runs over one session only (`session(20_100, 60)`, `session(20_300, 200)`), so no rollover happens.
- **CX-03**'s proof is 'the field's type; the early return in `crossings_of`', but the field does not exist and `crossings_of` has no early return (it uses `continue`).
- **CX-04** says 'the only production `Evaluator::new` passes `Availability::Absent`'. cli binds `Present` for every cash equity, so the reason it gives for excluding the VWAP crossing pair ('dead on every runnable path') is false on the equity surfa

**Evidence:**
  - **No field named `previous_mask` exists.** - `git grep -n previous_mask` on fix/c2-final and on origin/main returns the same four places: comments at crates/vocab/src/table.rs:1090 and :1261, the invariant rows at docs/04-invariants.md:2608-2610, and docs/05-decisions.md:19390. - The real crossing state is `last_side: [Side; CROSSINGS.len()]` at evaluator.rs:360 and `crossings_seen: [u8; ..]` at :381. - The rollover resets both at evaluator.rs:869 (`self.last_side = [Side::Unknown; ...]`) and ...
  - - two comments, vocab/src/table.rs:1090 and :1261 - docs/04-invariants.md:2608-2610 - docs/05-decisions.md:19390

**Expected fix and test:** None

## AC-whp-tb-6 · low bug · docs

**Where:** `docs/04-invariants.md:62`

**Finding:** S-06 (marked ◐) says `store::file::initialise` passes flag 0, so FLAG_CHECKSUMS 'is set nowhere in production', the sidecar is never created, block::seal and verify have no production caller, and 'every file this build has written is therefore uncovered'. S-06b says 'what is true of a real file today' is that `block::verify` answers `ChecksumsAbsent` for a file this build wrote. Both describe the code before it changed: `initialise` now passes `crate::format::FLAG_CHECKSUMS`, and its own comment records the change. The rows therefore misstate the integrity status of every month this build writes and reads on the sweep's input path.

**Evidence:**
  - (1) The S-06 row (docs/04-invariants.md:62) says `store::file::initialise` passes `Header::genesis(symbol_id, timeframe_secs, 0)`, so FLAG_CHECKSUMS "is set nowhere in production". fix/c2-final:crates/store/src/file.rs:2439-2466 reads "BORN WITH CHECKSUMS, AND ONLY EVER BORN WITH THEM.
  - On fix/c2-final:docs/04-invariants.md:62, S-06 says "`store::file::initialise` passes `Header::genesis(symbol_id, timeframe_secs, 0)` — flag **clear** — so `FLAG_CHECKSUMS` is set nowhere in production, the `FileKind::Checksums` sidecar is never created, and `block::seal`/`block::verify` have no production caller." Line 63 (S-06b) says "the flag is clear, so `block::verify` returns `ChecksumsAbsent`" for any file this build wrote. The current code on the same branch contradicts each of those ...

**Expected fix and test:** None

## AC-whp-tb-7 · low bug · docs

**Where:** `docs/04-invariants.md:2540`

**Finding:** R-03 says the short walk differs from the long one and that 'a direction accepted and then ignored fails it'. Since D-0387, ignoring the caller's direction is the design. WF-01 requires a Long caller and a Short caller to produce identical folds. `the_fold_decides_the_side_and_the_caller_cannot` asserts that. R-03's own test doc measures the two walks as fold-for-fold identical. R-03's named test asserts only the `side_of` mapping, never the walks. Two ✓ rows in one file therefore contradict each other about what `walk_forward` does with its `Direction`.

**Evidence:**
  - The row is at docs/04-invariants.md:2540, byte-identical on origin/main 96194c11, fix/c2-final, fix/c2f-r-api and fix/c2f-r-cli. `walk_forward` at validate.rs:1743 accepts `direction` and passes it through `walk_forward_shaped` (1869). validate.rs:5904-5945 checks that the short walk has 3 folds, trains on bars and does not ref
  - fix/c2-final:docs/04-invariants.md:2540 says "The short direction is exercised, and differs from the long one" and "a direction accepted and then ignored fails it". In the code, walk_forward (fix/c2-final:crates/runner/src/validate.rs:1743) passes `direction` to walk_forward_shaped, and that passes it on to walk_forward_shaped_with_rungs. That is D-0387 (docs/05-decisions.md:27755), stated as WF-01 at invariants:3025: "A Long caller and a Short caller must now produce identical folds".

**Expected fix and test:** None

## ET-o1-proof-coverage-8 · low bug · docs

**Where:** `docs/06-limits.md:75`

**Finding:** Stale limits. §5 uses 238 live positions and counts memory for a 'seen' dedup set that was deleted (engine lib.rs:1703). §7c says there is no bar reader and the per-bar read is unmeasured (C-28/C-29 now time read_record), that every row but C-08 is a ratio (all 13 crates now have budget rows, per §81), and that /instruments has no index (superseded by §24/D-0042). 'No bench in this repository times a syscall' is false: C-28/C-29 time pread and C-T-01 times file writes. The C-V-03 rows cite 234 live bits; the bench prints 328.

**Evidence:**
  - docs/06-limits.md:75 and :81 still say 238 live positions. docs/04-invariants.md:1805, :1823-1824 and :1829-1831 still say 234. It has a `HashSet<ConditionMask>` column (docs/06-limits.md:83) and says "`out` and the raw `seen` keys alone are 12.63 GiB" (:97-98). crates/engine/src/lib.rs:1702 says "THE DEDUP SET IS GONE", and :1704 explains that `seen` was a `MaskSet` and was removed.
  - docs/06-limits.md:75 and :81 say "238 live". The live count is pinned at 328 by crates/vocab/src/table.rs:1761-1766 (`assert_eq!(LIVE.popcount(), 328, ...)`) and stated in docs/03-vocabulary.md:724 ("live | 328"). At 328, C(328,4)=473,490,550, which is 3.53x DEFAULT_CEILING=2^27 (crates/engine/src/lib.rs:742). docs/06-limits.md:97-98 say "`out` and the raw `seen` keys alone are 12.63 GiB".

**Expected fix and test:** None

## ET-o1-proof-coverage-9 · low bug · engine

**Where:** `docs/04-invariants.md:127`

**Finding:** These rows still say the measurement is pending. My re-run produced numbers, but they measure the stand-ins, not the live path (see the first bug).

**Evidence:**
  - C-03 at docs/04-invariants.md:128 and C-04 at :129, both reading "SOURCE CORRECTED; CURRENT MEASUREMENT PENDING" with status "—". C-E-10 at :1924 and C-E-11 at :1925, both with status "—". :1970 and :1971, both reading "pending rerun after production-shape correction". All four are far under the 3.0x ceiling (CEILING_PERMILLE=3_000, ratio.rs:62).
  - In crates/engine/benches/ratio.rs, lines 346-353 and 370-410 build their own `HashSet<u32>::with_capacity` and insert key 0 over and over. Lines 427-455 push into their own `Vec<Itemset>::with_capacity`. The live code is at crates/engine/src/lib.rs:1405-1428 (`offered.try_reserve` + `offered.insert`) and :2284/:2303 (`out.try_reserve(batch.len())` then `out.push(Itemset{..})`).

**Expected fix and test:** None

## ET-o1-proof-coverage-7 · low bug · engine

**Where:** `docs/07-o1-architecture.md:160`

**Finding:** States that Layer 8 is asserted against a 1.4x ceiling. The harnesses assert 3.0x.

**Evidence:**
  - The doc line: docs/07-o1-architecture.md:160-164 says "Layer 8's flatness is asserted against the 1.4× ceiling in `docs/04-invariants.md`: C-E-02 checks ordinary k=1/4/8 candidates and C-E-09 drives the complete k=384 representation." What the code asserts: crates/engine/benches/ratio.rs:61-62 has `/// 3.0x — docs/04-invariants.md, the shared-CI number.` followed by `const CEILING_PERMILLE: u128 = 3_000;`.
  - The doc claim: docs/07-o1-architecture.md:160-161 says "Layer 8's flatness is asserted against the 1.4× ceiling in `docs/04-invariants.md`: C-E-02 ... What the code asserts: crates/engine/benches/ratio.rs:62 is `const CEILING_PERMILLE: u128 = 3_000;`, and its doc comment at :61 reads "3.0x — docs/04-invariants.md, the shared-CI number". `ratio()` at :105-124 passes when `up.max(down) <= CEILING_PERMILLE` (:113).

**Expected fix and test:** None

## UC-17 · low cost · engine

**Where:** `docs/07-o1-architecture.md:160`

**Finding:** Claimed: 'Layer 8's flatness is asserted against the 1.4× ceiling in docs/04-invariants.md' Actual: The engine and vocab harnesses assert 3.0x (CEILING_PERMILLE 3_000). docs/04-invariants.md:229-233 says 1.4x applies only to dedicated hardware and the harness asserts the CI number

**Evidence:**
  - docs/07-o1-architecture.md:160-161 says "Layer 8's flatness is asserted against the 1.4× ceiling in docs/04-invariants.md: C-E-02 ... Both of those rows call `ratio()` in crates/engine/benches/ratio.rs (C-E-02 at :335-339, C-E-09 at :518-522).
  - (1) What the doc says: docs/07-o1-architecture.md:160-161 reads "Layer 8's flatness is asserted against the 1.4× ceiling in `docs/04-invariants.md`: C-E-02 checks ordinary k=1/4/8 candidates and C-E-09 drives the complete k=384 representation". git blame dates line 160 to f1dacda68 (2026-08-01).

**Expected fix and test:** None

## W1-pull4-2 · low bug · pull

**Where:** `docs/04-invariants.md:771`

**Finding:** RG-07 (pull::vendor::a_rung_on_the_wire_needs_a_word_and_the_unrecorded_ones_stay_absent) (docs/04-invariants.md:771): the invariant ledger states the opposite of what its named test asserts at the day-rung boundary. Code path: RG-07 reads 'No descriptor carries a day-level window cap or a daily interval word'. vendor.rs ships `window_caps: &[(Granularity::Minute1, 30), (Granularity::Day1, 180)]` (4751) and `&[(Granularity::Minute1, 60), (Granularity::Day1, 2000)]` (5432), plus day words "1day"/"day" (4768-4771, 5441).

**Evidence:**
  - *What the row says.** docs/04-invariants.md:771 (RG-07) states: "No descriptor carries a day-level window cap or a daily interval word". The GROWW descriptor (`const GROWW: Descriptor` at crates/pull/src/vendor.rs:4585) has `window_caps: &[(Granularity::Minute1, 30), (Granularity::Day1, 180)]` at 4751, and `(Granularity::Day1, "1day")` in `granularity_tokens`.
  - docs/04-invariants.md:771 (RG-07) says "No descriptor carries a day-level window cap or a daily interval word". Groww, crates/pull/src/vendor.rs:4751: `window_caps: &[(Granularity::Minute1, 30), (Granularity::Day1, 180)]`, with the day word `(Granularity::Day1, "1day")` at 4770. Zerodha, vendor.rs:5432: `window_caps: &[(Granularity::Minute1, 60), (Granularity::Day1, 2000)]`, with `(Granularity::Day1, "day")` at 5441.

**Expected fix and test:** None

## W1-pull1-0 · low cost · pull

**Where:** `crates/pull/src/cash_session_cache.rs:313`

**Finding:** prepare_observed_with (via prepare_observed / prepare_local_observed) (crates/pull/src/cash_session_cache.rs:313): per one landed HTTP body of an NSE cash minute pull (api/src/server.rs:6764 `for bodies in landed.bodies.chunks(1) { match prepare_cash_schedule(...)`), the cost is O(D x B) per body: D = days in the touched months, B = master bytes (expanded + parse); it grows with committed post-2026-08-03 cash days in the body's whole source months (api/src/server.rs:6650-6665 `historical = committed_cash_days(...months)`) x master size (read <=4 MiB, SHA-256, gunzip <=32 MiB, CSV parse <=250,000 rows).. Auditor verdict: undocumented-scan. Documented: partial: docs/06-limits.md 'Dated cash-session acquisition limits (D-0519)' states 'Reading/hashing/decompressing a security master is O(bytes)' and 'Cached source evidence is revalidated at the instrument boundary, not ...

**Evidence:**
  - crates/pull/src/cash_session_cache.rs:313-325: `for day in days { let lock = lock_day(root, day)?; match read_entry(root, day)? `read_entry` (433-453) reads up to MAX_COMPRESSED (4 MiB) and SHA-256s it through `receipt()`. `decode` (455-481) gunzips up to MAX_EXPANDED (32 MiB) and runs `DailyEligibility::parse`, which is capped by MAX_ROWS = 250,000 (cash_auction.rs:27).
  - In crates/pull/src/cash_session_cache.rs:313-317, the loop `for day in days { let lock = lock_day(root, day)?; match read_entry(root, day)? `read_entry` (lines 431-452) reads the payload (up to 4 MiB) and runs `receipt(day,&bytes)`, which does `Sha256::digest`. `decode` (lines 454-480) gunzips up to 32 MiB and runs `DailyEligibility::parse`, which is capped at `MAX_ROWS = 250_000` (cash_auction.rs:27).

**Expected fix and test:** None

## ET-bars-candles-store-4 · low bug · pull

**Where:** `crates/pull/src/fold.rs:333`

**Finding:** `bar.volume = bar.volume.saturating_add(snap.volume)` silently caps a bucket's summed volume at i64::MAX instead of refusing. The same function refuses timestamp overflow on the stated grounds that saturation `would silently file a bar ... which CLAUDE.md §4 bans`.

**Evidence:**
  - crates/pull/src/fold.rs:333 is `bar.volume = bar.volume.saturating_add(snap.volume);`. In the same function, fold.rs:302-303 says: "Checked at both ends. Saturating here would silently file a bar in the wrong bucket rather than refuse, which `CLAUDE.md` §4 bans." That is why the timestamp shift uses checked_add/checked_mul/checked_sub with FoldError::AnchorOverflow (fold.rs:304-316).
  - crates/pull/src/fold.rs:333 is `bar.volume = bar.volume.saturating_add(snap.volume);`. In the same function, the timestamp shift is checked at both ends: `checked_add(anchor)` at fold.rs:309-314, and `checked_mul`/`checked_sub` at fold.rs:315-321.

**Expected fix and test:** None

## ET-bars-candles-store-8 · low cost · pull

**Where:** `crates/pull/src/ingest.rs:2216`

**Finding:** Claimed: Operator requirement: incremental and streaming wherever possible; CLAUDE.md §3 rule 4 (result append O(1)) Actual: Every ingest into a month re-reads every stored minute record and every stored derived record, and re-folds the whole month for 7 rungs, so work per ingest grows with how full the month already is. Filling a month one session at a time is quadratic in sessions (about 230k reads over 23 sessions where about 19k would do).

**Evidence:**
  - `derive_all` runs `(0..file.header().n_valid).map(|i| file.read_record(i))` over the whole committed 1-minute month file (:2201-2218). Its own comment at :2200 says "This is O(month rows) once per batch". `reconcile_derived` runs `for index in 0..file.header().n_valid` over every stored derived record of each rung (:2544). Its doc at :2522 says "O(source + held)".
  - crates/pull/src/ingest.rs:2201-2219 (derive_all). ingest.rs:2471-2477 (derive). ingest.rs:2526-2575 (reconcile_derived). options return early (ingest.rs:2196-2198) reconcile returns early when no derived file exists (2532-2539) The growth is bounded by the month partition (months_in, ingest.rs:1916) and does not grow with history or store size. total per ingest: 150-250 ms at every fill level

**Expected fix and test:** None

## W1-pull2-3 · low cost · pull

**Where:** `crates/pull/src/ingest.rs:2216`

**Finding:** derive_all (history read) + derive (fold per rung) (crates/pull/src/ingest.rs:2216): per one broker window, for each month it touches (MINUTE_1 spot, cash or futures), the cost is Θ(committed month rows) preads (one pread per 56-byte record), plus 7 × O(month rows) complete_minutes_with_calendar folds per window per month. A rerun of a month already held (AlreadyPresent) pays the same.; it grows with committed rows already in the instrument-month (month-to-date), times 7 derived rungs. Auditor verdict: undocumented-scan. Documented: none in docs/06-limits.md. Stated only in the code comment at ingest.rs:2199-2200.

**Evidence:**
  - (1) crates/pull/src/ingest.rs:2216-2218 `(0..file.header().n_valid).map(|i| file.read_record(i)...).collect()` reads every committed MINUTE_1 record of the month, one positional read each. store/file.rs:1981-2005 `read_row` does one `read_fully` per record, and `verify_block_of` (file.rs:2072-2083) checks each block once because it caches the last block, so the read is Θ(n) overall.
  - In `one()`, `write_and_count` runs at ingest.rs:1786. `derive_all` then runs at ingest.rs:1817 with no condition on `wrote`. Lines 2216-2218 do `(0..file.header().n_valid).map(|i| file.read_record(i)...)`. `read_record` goes through `read_row`, which does one `read_fully` at `offset_of(index)` per record (store/file.rs:1981-1995). The comment at 2199-2200 admits the cost: "This is O(month rows) once per batch".

**Expected fix and test:** None

## W1-pull2-5 · low cost · pull

**Where:** `crates/pull/src/ingest.rs:2039`

**Finding:** committed_cash_days (crates/pull/src/ingest.rs:2039): per one broker run (minute equity instrument request; api server.rs:6655 prepare_cash_schedule), the cost is Θ(committed rows) preads per touched month, per run; it grows with committed minute rows in every month the request touches (~8,250 per full month). Auditor verdict: undocumented-scan. Documented: partial. The 'Dated cash-session acquisition limits (D-0519) / Recovery-audit follow-up' section says full-month source reads are kept off the per-bar path and audits are O(stored rows).

**Evidence:**
  - WHAT I CONFIRMED FROM THE CODE: crates/pull/src/ingest.rs:2039-2042 does read every committed record of each touched month: `for index in 0..file.header().n_valid { let row = file.read_record(index)`. The only caller is crates/api/src/server.rs:6655, inside prepare_cash_schedule, and it runs only for Minute1 equity.
  - *The scan is real.** crates/pull/src/ingest.rs:2039-2041 walks every committed record of each month file: Its only caller is api crates/api/src/server.rs:6655, inside prepare_cash_schedule, and only for Minute1 equity. read_record (store/src/file.rs:1895) is one pread per record plus one block verify per 73 records, cached by `verified` at file.rs:2081.

**Expected fix and test:** None

## R9-csr-cx-0 · low bug · pull

**Where:** `crates/pull/src/ingest.rs:3027`

**Finding:** CensusLock::take opens `<vendor>.man.lock` with `.create(true).truncate(false).write(true)`, which is O_WRONLY|O_CREAT with no read(true). If a FIFO sits at that path (a misfiled lock file), open(2) blocks until some process opens the FIFO for reading. The ingest therefore never refuses and never proceeds, which CLAUDE.md §4 forbids ('degrade loudly and name the reason, or refuse'). The function's own comment lists the misfiled-lock shapes it refuses: PermissionDenied, and IsADirectory for 'a directory occupying the lock's name'. A FIFO is not among them. Every other lock opener in this area uses read(true).write(true), which is O_RDWR and does not block on a FIFO: store/src/file.rs:2603 open_rw, pull/src/cash_session_cache.rs lock_day, and pull/src/masters.rs lock_source. The open lines predate the range, but D-0693 rewrote this function (c355840f) and left them as they were.

**Evidence:**
  - `run`/`record_all` call it before anything else touches the lock path (ingest.rs:805 and 1204).
  - I could not refute the mechanism. The defect is real, but C2 did not introduce it. Code: in fix/c2-final crates/pull/src/ingest.rs, CensusLock::take opens `<census>.man.lock` with `fs::OpenOptions::new().create(true).truncate(false).write(true).open(&lock_path)`. That is O_WRONLY|O_CREAT with no read. Nothing inspects the lock path before this open; the only earlier step is `create_dir_all` on the parent. Only PermissionDenied and IsADirectory are refused. Opening a FIFO write-only blocks ...

**Expected fix and test:** None

## R9-csr-cx-1 · low bug · pull

**Where:** `crates/pull/src/ingest.rs:3091`

**Finding:** D-0693 rewrote the census lock acquisition as `match Flock::try_lock(lock, lock_path.clone())` with a bare `Err(_)` arm. That arm maps both TryLockError::WouldBlock and TryLockError::Error(io) to one message: another ingest holds the lock, so the operator should wait for it and try again. When the host refuses flock itself, no other run exists and waiting never helps, yet the refusal names a cause that is not the real one. Two ways this happens: ENOTSUP for a non-regular lock file on macOS, measured below, and ENOLCK on an NFS mount without lockd. That goes against CLAUDE.md §4 ('name the reason'). The sibling sites in the same range keep the host's error: store/src/file.rs lock_fault, store/src/repair.rs shared_lock and cash_session_cache lock_day, which puts `{why}` in its message. This conflation predates the range (96194c11 had `if lock.try_lock().is_err()`), but the rewrite kept it.

**Evidence:**
  - At `crates/pull/src/ingest.rs:3091-3101`, `match Flock::try_lock(lock, lock_path.clone()) { Ok(held) => ..., Err(_) => Err(format!("another ingest holds the census lock at {}.
  - On fix/c2-final (19acd032), crates/pull/src/ingest.rs:3091-3100 `CensusLock::take` does `match Flock::try_lock(lock, lock_path.clone()) { Ok(held) => .., Err(_) => Err("another ingest holds the census lock at ..

**Expected fix and test:** None

## W1-pull4-3 · low bug · pull

**Where:** `crates/pull/src/vendor.rs:3956`

**Finding:** Descriptor::granularity_verdict / HttpSpec::listing_words (doc-stated bounds) (crates/pull/src/vendor.rs:3956): stated sizes behind the O(1) arguments are stale. The doc says 'walks the whole 4 × 11 matrix' (3956, and docs/04-invariants.md GF-03 line 2222), but FEED_COUNT = 5 (3628). listing_words' 'the number of listing classes — two' (3160) is wrong: Listing has three variants and Groww ships three rows. The bounds still hold, but the premises written down for them do not. Code path: `pub const FEED_COUNT: usize = 5;` (3628). `DESCRIPTORS: [&Descriptor; FEED_COUNT] = [&DHAN, &GROWW, &TRUE_DATA, &GDFL, &ZERODHA]` (5550). `enum Listing { Index, Equity, Derivative }` (1696-1716). Groww's listings hold three ListingWords rows (4775-4791). Documentation defect only.

**Evidence:**
  - The doc on `granularity_verdict`, crates/pull/src/vendor.rs:3955-3956, says the test "walks the whole 4 × 11 matrix". The same "4 × 11" appears in the test's own doc at vendor.rs:8289 and in docs/04-invariants.md:2222, row GF-03.
  - crates/pull/src/vendor.rs:3956 says the test "walks the whole 4 × 11 matrix". The test's own doc at :8289 and docs/04-invariants.md:2222 (GF-03) say the same. `pub const FEED_COUNT: usize = 5;` (:3628), with `Feed::ALL` = Dhan, Groww, TrueData, Gdfl, Zerodha (:3658-3664). `pub const GRANULARITY_COUNT: usize = 11;` (:175). `const _: () = assert!(Feed::ALL.len() == FEED_COUNT);` (:5553).

**Expected fix and test:** None

## ET-strategies-trades-ranking-costs-9 · low cost · runner

**Where:** `docs/06-limits.md:4425`

**Finding:** Claimed: peak_adverse/peak_favourable 'are called for every profitable candidate in every cell, so the real bar-visit count carries a 2 × variants × winners × span term' Actual: Stale in the pessimistic direction. peak is now called only in pass two, once per trade (grid.rs:2133-2151), and the per-cell reads are indexed (grid.rs:4105-4118; docs/04-invariants.md EB-06).

**Evidence:**
  - docs/06-limits.md:4425-4433 (under "## 72. `grep peak_adverse|peak_favourable` over crates/ finds only two non-test call sites: crates/runner/src/grid.rs:2138 and :2145. Both are inside `for t in &timed.trades` (grid.rs:2131-2152), which is PASS TWO of evaluate_timed. Every other hit is the definitions (grid.rs:5158/5163/5168) or tests at line 6216 and later, all after `#[cfg(test)]` at 5198.
  - docs/06-limits.md:4425-4432 is out of date, and it overstates the cost. §72 (heading at docs/06-limits.md:4377) says `peak_adverse` and `peak_favourable` "are called for every profitable candidate in every cell". The only non-test calls are in pass two, once per trade: grid.rs:2138 and :2145, inside `for t in &timed.trades` at grid.rs:2133.

**Expected fix and test:** None

## R9-csr-o1-0 · low cost · store

**Where:** `docs/06-limits.md:8907`

**Finding:** The range makes one behaviour a pinned invariant: every touch of a tail block that an interrupted append sealed past the commit, when it follows a touch of another block, re-runs the D-0688 proof (AF-47, AF-48, and the new crates/store/tests/tail_proof.rs, 7f617f01 and 9fc9233b). Each re-run costs one fstat, two heap allocations, a pread of up to 4,088 bytes, a pread of up to 72 records past the commit, a CRC, and one `store.block` WARN written to the telemetry file.

The cost statements for that path say otherwise, and none of them was corrected. The range edited the same limits section in 7dd8de76 and left them in place:
- docs/06-limits.md:8907 says "The tail block's first verification per handle adds one `fstat`." It is added on every verification of the tail, not only the first per handle.
- docs/06-limits.md:8936 says the bit-rot case "warns twice, `store.header` for the fallback and `store.block` for the interrupted append". In fact `store.block` is written once per re-verification, so the count grows with alternating reads.
- crates/store/src/file.rs:868-872, on the `verified` field, still calls re-verification on alternation "a cost, not a correctness hole: a re-verificati

**Evidence:**
  - Confirmed from the code, but the defect is not C2's. No test was run; everything below comes from reading the code and diffing the two refs. Why it is real (low): `verify_block_of` caches exactly one block (`if self.verified.load(..) == block { return Ok(()) }`). On every cache miss of the tail it calls `past_the_commit` unconditionally, and that means one fstat via `self.bars.metadata()`. Every admission of an interrupted append then goes through `block::verify_through`, which calls ...
  - The doc problem is real, but C2 did not introduce it. The sentences at fault and the code behind them are the same on origin/main (96194c11). 1. The code is unchanged. `git diff 96194c11 fix/c2-final -- crates/store/src/block.rs crates/store/src/header.rs` is empty. The file.rs diff hunks sit at lines 174-1667 and 4026 and later, and none touches verification. The text from `fn verify_block_of` through `past_the_commit` hashes the same on both refs (md5 1002aa48...). - So `past_the_commit`, ...

**Expected fix and test:** None

## ET-vocabulary-conditions-bits-3 · low bug · vocab

**Where:** `docs/04-invariants.md:1805`

**Finding:** C-V-03 says the wide candidate is 'all 234 live bits', but the bench uses LIVE, which is 328 bits: the shipped bench prints 'C-V-03 candidate width (context) 1 bit(s) -> 328 bit(s)'. C-V-06 says it probes 'position 279', but the bench probes NEXT_FREE-1 = 369 ('C-V-06 is_live: position 0 -> position 369'). C-V-06 also claims 'a scan does not [cost the same everywhere]', which is false for a flat scan (A9).

**Evidence:**
  - ratio.rs:169-171 `all_live_bits()` returns `LIVE`. table.rs:1460-1572 builds LIVE: words 0-3 give positions 0..=234, three tombstones are removed (6, 19, 25), and bits 274..=369 are added. ratio.rs:200 sets `last = NEXT_FREE.saturating_sub(1)`, and table.rs:1431 sets `NEXT_FREE = 370`, so the probed position is 369. ratio.rs:201 sets `past = NEXT_FREE.saturating_add(64)`, which is 434.
  - docs/04-invariants.md:1805 (C-V-03) says "a 1-bit candidate against all 234 live bits". docs/04-invariants.md:1823 says "1-bit candidate → 234-bit candidate". docs/04-invariants.md:1829 says "a 1-bit and a 234-bit candidate". crates/vocab/benches/ratio.rs:24 says "234-bit" and :294 says "k=234". The bench actually uses `all_live_bits()`, which returns `LIVE` (ratio.rs:169-171).

**Expected fix and test:** None

## W1-api1-6 · none cost · api

**Where:** `crates/api/src/booleanevidencejson.rs:343`

**Finding:** admission / statistics (Boolean evidence page projections) (crates/api/src/booleanevidencejson.rs:343): per one warm page request (1..=256 rows), the cost is about 4 x (2C + 2) lease + generation checks per admission page, and about 3 x (2C + 1) per statistics page. Each check is a shared flock/unlock plus 6 metadata calls, done before and after, so roughly 112C syscalls per admission page; it grows with number C of source catalogs linked to the statistics artifact, independent of page size. Auditor verdict: needs-measurement. Documented: none

**Evidence:**
  - *Cost of one check.** `Observation::with_current` (crates/cli/src/boolean_observation_file.rs:99-109) does four things. It runs `check_generations()` before and after the work, and each run stats 3 files (owner.lock, body.bin, complete.bin) with `file.metadata()` plus `std::fs::metadata(path)` (result_set.rs:567-573).
  - admission() (lines 343-400) makes four currency calls: `reader.require_current()?` (344) goes to Admission::require_current, which is `self.observation.with_current(|| self.statistics.require_current())` (boolean_admission_reader.rs:103-106). `reader.rows(pin, ...)` (349) nests `observation.with_current(|| self.statistics.with_current(|| page(...)))` (admission_reader:119-122).

**Expected fix and test:** None

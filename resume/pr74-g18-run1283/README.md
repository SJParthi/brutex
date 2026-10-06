# Gate 18 survivors on run 1283 (37251141390, head 969493e1)

Collected 2026-10-06 03:10 UTC from check-run annotations of the failed shards
(`GET /repos/SJParthi/brutex/check-runs/<job id>/annotations`, unauthenticated, through the proxy).
Each `##[warning]` annotation is one MISSED or TIMEOUT mutant; read the shard's log with
`mcp__github__get_job_logs` (job id from `/actions/runs/37251141390/jobs`) for which.

296 distinct survivors from 142 shards: cli 161 (split cli-a = lib.rs and files before it, cli-b = after),
api 50, runner 45, rest 47 (store, pull, lake, indicators, vocab, telemetry, costs, ...).
Not tested on this run (infra): shards 125,126,128,131,132,136,140,141,142,144,145 never got a runner;
146,150 hit the 4 h limit; 8,97,111,116,119,120,167,168 lost the runner/git. Shards 178-202 were
still running at collection time.

Fixer sessions: one per file here, each on its own branch `pr74/g18-<group>` off origin/final/all-fixes.

2026-10-06 04:05 UTC addendum: shards 178, 180-184 finished with 13 more distinct survivors (runner 7, api 4, cli-a 1, rest 1), appended to the survivors files above. 17 shards were still running.

2026-10-06 04:40 UTC — TIMEOUTS ARE NOT IN THESE LISTS. cargo-mutants 26.2.0 emits a GitHub `::warning` only for MISSED
mutants (src/console.rs: `outcome.mutant_missed()`), so the annotation-based lists above hold MISSED cases only. Timed-out
cases appear only in each shard's log ("TIMEOUT ..." lines and the verify line "surviving=N, timed-out=M"). Example found
by the coordinator's independent re-run: `crates/telemetry/src/tail.rs:503:15: replace > with >= in walk_back` (u64
`pos >= 0` never ends; shard 58). The coordinator is reading every failed shard's log for TIMEOUT lines; results will be
appended here as timeouts-<group>.md. ALWAYS read shard logs, not only annotations, when collecting survivors.

Also 04:40: vocab `fnv1a`/`name_index` endless compile-time loops (shards 146, 150 died at the 240m limit) are fixed by
pr74/g18-rest 3565e479 (D-2083, recursion; session proof 39 mutants: 13 caught, 26 unviable) and CI gets a build bound
(coordinator commit 46439dee on local integ, D-2090: --build-timeout-multiplier 2).

2026-10-06 04:50 UTC — TIMEOUT lists from every failed shard log (8 parallel readers, get_job_logs tail 100): 23 timeouts,
none of them in the MISSED lists: timeouts-cli-a.md (4), timeouts-cli-b.md (10), timeouts-rest.md (3), timeouts-runner.md (3),
timeouts-api.md (3). Shards 8, 111, 116, 119, 120, 138, 167, 168 printed no summary (runner lost): their cases are untested
(138 added to the coordinator's local pre-run). A timeout costs a shard about 45-76 minutes, which is why 13 shards ran past 3 h.

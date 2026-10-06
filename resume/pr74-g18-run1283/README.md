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

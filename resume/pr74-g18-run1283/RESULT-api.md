# RESULT-api (PARTIAL, paused 14:45 UTC) — pr74/g18-api head 0be9754

Mutation proof so far (CI flags; the 3 timeouts were also run with --test-threads=1 and --build-timeout 300): 31 caught, 0 missed, 0 timeout. The proof is still running in the worktree; about 27 survivor mutants and 62 in-diff mutants remain. Survivors still unfixed: none found yet.

Update 15:00 UTC: 36 caught, 1 missed, 0 timeout. The missed mutant is `server.rs:7685 replace - with / in observed_cash_source_days`. It is NOT on the api survivor list: my wide `server.rs: replace - with /` regex picked it up, and it sits on unchanged code outside this branch's diff. It is listed here and not fixed (pause). Remaining: about 22 survivor mutants and the 62 in-diff mutants. The run continues.

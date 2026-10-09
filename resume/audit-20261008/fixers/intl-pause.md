# intl pause — worktree /home/claude/wt-l, branch audit/int-l, HEAD af71bb1e (base a7a27dc3); tree clean, no cargo running

| item | state | decision | commit |
|---|---|---|---|
| 1 runner clippy (research_family_readiness too_many_lines) | DONE | — | 58383fcb |
| 2 fxr WIP: satk-8, satk-7, closure flag, c4a-6 C-R-06, r64-3, satk-4 | DONE (targeted tests, runner clippy, ratio bench, core cost_invariants green) | D-4501..D-4506, FXR-06..08, C-R-06 | 1ee02e24 |
| 3 fxa WIP: so1-6, so1-2 | DONE (store and lake benches run once; 0 brutex-bench-* left) | D-4420 | 55a340fe |
| 4 fxd WIP + CLAUDE.md support_each | DONE apart from the full runner suite (it goes into the item 6 validation) | D-4487, D-4488 | 89cd6e90, b6ae1e9e |
| 5a rnew-2, r53-2/srust-6, sobs-17 | DONE | D-4489, INTL-01 | af71bb1e |
| 5 r64-2, sobs-19, W3-runner2-5 | NOT-STARTED | D-4509.. free | — |
| 6 validation | NOT-STARTED | — | — |

The resumer needs to know:
- Gates 11, 14 and 17 pass. Gates 10, 10b, 27 and 27b pass. Gate 12 passes for the lower crates. It still refuses `api/src/operation_audit_tests.rs:816` and `api/src/topjson.rs:1`; those are for intu.
- Not fixed: the engine bench comments for C-E-07 and C-E-09 still call `Column::support` the live path. It is now `support_each`.
- Gate scripts are in the scratchpad (`Gate*.sh`). Run each with `rungate.sh`, which uses `RUNNER_TEMP=/tmp/claude-0/gate-tmp-intl` (the tools are built there).
- Next command: start item 5 with r64-2 (`crates/pull/src/http.rs:1414`, `crates/core/src/price.rs:12-33`). Then run item 6: `JOBS=2 /tmp/claude-0/bin/validate.sh /home/claude/wt-l /home/claude/t-l runner engine vocab pull core store lake telemetry indicators costs greeks`.
- The report `/tmp/claude-0/audit/out/intl-report.md` is not written yet.

# OWNER RULES ADDENDUM (2026-10-06 03:40 UTC)

From the repository owner (Parthiban), relayed by the coordinator session. Apply on top of your prompt; CLAUDE.md still wins on any conflict.

1. **Zero means evidence, not a claim.** Every fix ships with a test that FAILS without the fix (run it once against the reverted code and record the failing output) and passes with it. Every claim in your RESULT file names the command that proves it and its output. No hallucination: if you did not run it, it is UNVERIFIED.
2. **Attack everything you touch, adversarially.** For each changed function enumerate extreme permutations and combinations: boundary values (0, 1, max, min, i64::MIN/MAX, empty, one element, exactly at capacity, capacity+1), malformed / truncated / corrupt input, duplicate / out-of-order / missing data, a crash at every write boundary, disk full, permission denied, concurrent callers, clock edges (IST session open/close, holidays, month and year boundaries), error-path ordering, and out-of-the-box cases nobody listed. Each attack becomes a test. Where a small domain can be enumerated exhaustively, enumerate it all; where an independent naive reference can be written, compare against it on every case (differential testing).
3. **O(1) with measured p99.** Any per-operation path you touch (per request, per bar, per candidate, per row, per event) must stay O(1). Measure p50/p99/max before and after on your box and record the numbers. Anything that cannot be O(1) is named in docs/06-limits.md with its bound and measured numbers. It is never claimed O(1).
4. **Persisted, logged, auditable, visible.** Any state change or failure on a path you touch must be persisted and must emit a telemetry event that the /logs page can show and search, at O(1) per event. Respect gate 17: no logging inside the vocab/engine/indicators/runner inner loops; log at the structural boundary (per run, per instrument-month). No fallback may hide a failure (CLAUDE.md §4).
5. **Rust only.** Every file you add must have an allowed extension per CLAUDE.md §2; web/ is the only exception. No interpreted runtime, no build.rs that spawns a process, no foreign binding. Avoid new dependencies; if you add one, check Cargo.lock for all of the above and run cargo deny.
6. **One common, dynamic, incremental path.** No per-instrument special-casing. One authority per fact (calendar, fold, costs, session, vocabulary). Resumable and incremental wherever a run can be interrupted.
7. **Parallel.** Use subagents for research and reading in parallel. Keep heavy cargo jobs one at a time on your box.
8. **Honest report.** Report found / fixed / not fixed and why. Label anything unmeasured UNVERIFIED.

Owner rule, 08:10 UTC 2026-10-06: every agent and subagent uses Opus 5.5 (claude-opus-5-5),
and nothing else. No haiku or sonnet model override anywhere: not in the Agent tool, not in
workflow agent() calls, not for pollers. The coordinator polls with its own tools.

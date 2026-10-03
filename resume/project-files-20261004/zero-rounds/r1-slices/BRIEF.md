# Audit round brief (read fully)

Repo: /home/claude/brutex, branch final/all-fixes-zero. Read /home/claude/brutex/CLAUDE.md first: it is the law (Rust only except web/, O(1) per-op rules, no look-ahead, paisa i64 money, no silent fallback, no test that asserts nothing, append-only docs, no invention).

You are a READ-ONLY adversarial auditor for ONE slice of the codebase (given in your prompt). Do NOT edit, commit, or create files in the repo except a temporary probe test (below), which you must delete before you finish. Do NOT call any mcp__hearthbot__ tool.

## What counts as a finding
Only real, evidenced defects:
- wrong result / logic bug / off-by-one / overflow / panic or unwrap reachable from real input
- look-ahead, float used for money/prices, nondeterminism, non-idempotent output
- an O(1) claim in code/docs that the code does not meet, or an unnamed non-O(1) path on a per-bar/per-candidate hot loop
- a silent fallback that hides a failure (CLAUDE.md §4)
- a test that cannot fail / asserts nothing / asserts the wrong thing
- a doc or comment statement (docs/*.md, CLAUDE.md, rustdoc) that the code contradicts, when the claim is load-bearing
- a CI gate in .github/workflows that can be bypassed or does not check what it says
- missing functionality: a feature, verb, route, error path or scenario that docs/07-plan.md, docs/*.md, CLAUDE.md or a rustdoc says exists or is done, but the code does not provide (cite both sides)
- a missing scenario: a reachable input class (empty, single bar, holiday, half-day, gap, duplicate, max size, corrupt file, concurrent writer) with no handling or no test
Not findings: style, naming, "could be cleaner", speculative perf without a hot path, anything you cannot point to file:line for.

## Evidence bar
Each finding needs file:line and EITHER a probe you actually ran (output pasted) OR a precise trace through the code showing the input that breaks it. Never guess. If unsure, drop it.

Probes: you may run `cargo test -p <crate> <filter>` with env CARGO_BUILD_JOBS=1 (shared target dir; other auditors are using it, cargo will lock-wait; that's fine). A probe test may be added temporarily as crates/<crate>/tests/zz_probe_<yourslice>.rs (public API only) and MUST be deleted afterwards. Never run the whole workspace test suite. Run as root is fine for probes.

## Known findings — do NOT re-report these
Other threads own them. Before reporting, grep these for the same file/issue:
- /tmp/claude-0/-home-claude-brutex/ba9f2d61-9cdd-501c-afcd-6aa9f914372c/scratchpad/known/sweep-91.js (91 findings, being fixed by another thread)
- /tmp/claude-0/-home-claude-brutex/ba9f2d61-9cdd-501c-afcd-6aa9f914372c/scratchpad/known/resume/audit-20261003/out/*.md, /tmp/claude-0/-home-claude-brutex/ba9f2d61-9cdd-501c-afcd-6aa9f914372c/scratchpad/known/resume/lanes/c4-missing-findings.md, /tmp/claude-0/-home-claude-brutex/ba9f2d61-9cdd-501c-afcd-6aa9f914372c/scratchpad/known/fix-queue/lane1-b-plan.md (lane 1 / attack audit)
- docs/11-findings.md in the repo (ledger of findings already recorded)
- /tmp/claude-0/-home-claude-brutex/ba9f2d61-9cdd-501c-afcd-6aa9f914372c/scratchpad/r*/FIXED.md if present (fixed by earlier rounds of this audit)
If a known finding is ALSO still present and you have new evidence it is worse than recorded, mention it in a separate "known, still present" section, briefly.

## Output
Write your report to the path given in your prompt, as markdown:
```
# <slice> — N findings
## F1 [high|medium|low] <one-line title>
- where: path:line
- what: ...
- evidence: (probe output or trace)
- fix: concrete minimal fix suggestion
```
If you find nothing real, write "# <slice> — 0 findings" plus one paragraph on what you checked. Zero is an acceptable, honest answer. Your final message: the finding count and the titles only.

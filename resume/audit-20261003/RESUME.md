# Audit of final/all-fixes (PR #74) — STOPPED at 99% weekly usage, 2026-10-03 ~04:00 UTC

Scope: verify all 547 earlier findings (250 batch-2 in fix-queue/lane{1,2,3}{,-b}.md, 53 in resume/audit-20261002/new-findings.md, 64 batch-1 groups in resume/lanes/lane4-items.tsv.md) against final/all-fixes head 1087e544, and hunt new bugs (fold regressions, Rust-only, O(1), adversarial edges). Do not push to final/all-fixes (lane 1's).

## Done
- fmt --check: clean. clippy --workspace --all-targets -D warnings: clean (run as non-root user).
- Full `cargo test --workspace --locked` as non-root was ~half way when stopped: 1 failure seen so far:
  cli `boolean_search_command::integration_tests::generated_search_recovers_same_ordinal_and_refuses_missing_ancestry` (cause not yet read).
- v2 (lane 2, 82 findings): 73 FIXED, 5 PARTIAL, 4 DOCUMENTED, 0 NOT-FIXED; 3 NEW (v2-1 medium: Gate 14 layer 5 still uses the old awk step check, a disabled Gate 8 bench step still reads "present"; v2-2, v2-3 low). Report: out/v2.md.
- v1 (lane 1, 79 findings): 20 FIXED, 2 DOCUMENTED, 2 PARTIAL, **55 NOT-FIXED** — the lane1-b batch was lost in the 2026-10-03 00:00 restart; only U1, U14, U2, U21 landed. 1 NEW low. Report: out/v1.md.
- h2 partial: adversarial probes on api text/query parsing and pull (out/h2-evidence, probes in out/h2-probes); no verdicts written yet.

## Not done (next)
1. Lane 3 (89 findings), audit-53 and batch-1 (64 groups): not verified — workers stopped before writing.
2. Fold-regression hunt (append-only docs/05, duplicate D-numbers, invariant test names, conflict debris, Rust-only walk): not written.
3. Finish the non-root full test run and read the cli failure above.
4. Route the 55 NOT-FIXED lane1-b items and v2-1 to a fix lane (PR #74 will merge without them).
5. Publish the comparison Artifact from out/*.md. Worker rules: RULES.md (paths need adjusting).

## Resumed 2026-10-03 04:10 UTC (new account, audit thread)
- Workers running against final/all-fixes 1087e544: v3a (lane3.md 16 + lane3-b first 36), v3b (lane3-b last 37), v53 (audit-53), v4 (batch-1 64 groups), fold (fold-regression hunt). Reports will land in resume/audit-20261003/out/{v3a,v3b,v53,v4,fold}.md.
- The CLI test failure and #74 CI belong to the PR-#74 thread; the 55 lane1-b NOT-FIXED items belong to the lane-1 redo thread. This thread does not duplicate them.
- If stopped again: rerun only the workers whose out/*.md file is missing, using RULES.md.

## State 2026-10-03 ~05:45 UTC (audit thread)
- All verification done; reports in out/: v1 v2 (earlier), v3a v3b (lane 3, 89), v53 (audit-53), v4 (batch-1 64 groups), fold, c4a/c4a-verdicts/c4b (16 lost batch-1 groups, original text in resume/lanes/c4-missing-findings.md: 60 findings, 44 NOT-FIXED, 8 PARTIAL).
- Three local fixers running on branches audit-fixes (new findings v3b-1/-2, v3a-1, v53-1/-2, audit-root, docs, v4-1..3; D-1480..1489), audit-fixes-2 (c4 non-cli + c4a-N + v4-4; D-1490..1519), audit-fixes-3 (c4 cli + c4b-N; D-1620..1659, moved off the sweep thread's D-1520..1619). These are local; if lost, redo from the out/*.md reports with those decision ranges.
- When they finish: merge each into final/all-fixes (fetch + merge first, never force), validate, push onto PR #74, then publish the comparison Artifact.
- Local fix branches are mirrored as wip/audit-fixes, wip/audit-fixes-2, wip/audit-fixes-3 (pushed at each checkpoint; resume from them).

## State 2026-10-03 ~11:05 UTC
- Artifact published: https://claude.ai/artifact/K6QyihZvvh1d4UXQnApVZT (source brutex-audit-ledger.html here).
- Fix batches A-C combined on local branch audit-combined (= wip/audit-fixes + -2 + -3 merged, union-resolved docs tails). fmt+clippy clean.
- Local cargo-mutants 26.2.0 (nextest, max-fail=1) running on 249 mutants of the combined diff vs 1087e544. PR #74 CI run 37092404876 still running Gate 18; push only once, after it finishes or after local mutation is clean.
- Batch D (wip/audit-fixes-4, D-1503..1519): v2-1/-2/-3, v1-1, R9-api-law-0 web page, W1-api2-11 — in progress.

## State 2026-10-03 12:35 UTC
- PUSHED: all audit fixes (batches A-D) on final/all-fixes 331b05c6 (fast-forward from 1087e544). Artifact v2 updated.
- Still running locally: full workspace test of 331b05c6, cargo-mutants --iterate on 268 changed-line mutants (49 earlier: all caught/unviable). Any survivor/failure -> fix as a normal commit on final/all-fixes (fetch+merge first).
- Waiting on user decision card: GAP15-19, GAP15-17, W2-cli5-4, W3-runner2-8 (recommended: leave documented).

## State 2026-10-03 13:20 UTC (user: fix everything fully; throttle to 3 agents for usage)
- Full workspace test on 331b05c6: 6,612 passed, 0 failed.
- Running (cap 3): F5 (GAP15-19, GAP15-17, cli halves AC-whp-tb-2/ET-7; D-1643..1649, D-1800..1809, AGA-), F7 (gate8 self-test, excursion look-ahead, docs-web-01, probeapi-1 body, GAP17-33, h-api-1..3; D-1509..1519, D-1820..1829, AGC-), F9 (h-cli-1..3; D-1850..1859, AHA-). Branches wip/audit-fixes-5/7/9.
- QUEUED: F8 (remove documented costs where O(1) possible; triage out/f8-triage.md; branch wip/audit-fixes-8 has 1 commit + 1 WIP commit; D-1830..1849, AGD-), F10 (h-eng-1, h-eng-2 Fib rung co-firing vs implication screen; D-1860..1869, AHB-), hunter pull/store/lake (stopped before reporting).
- BLOCKED by safety check, needs the user's explicit words: W2-cli5-4 (frontier rule single authority across cli/api/web), W3-runner2-8 (column digest v2), plus W3-runner2-7, W3-runner5-0 that were in the same request.
- Fixer rules: out/FIXRULES.md. Gate extractor: awk step `run: |` block by line (see FIXRULES).

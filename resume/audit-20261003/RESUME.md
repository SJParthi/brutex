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
- Three local fixers running on branches audit-fixes (new findings v3b-1/-2, v3a-1, v53-1/-2, audit-root, docs, v4-1..3; D-1480..1489), audit-fixes-2 (c4 non-cli + c4a-N + v4-4; D-1490..1519), audit-fixes-3 (c4 cli + c4b-N; D-1520..1559). These are local; if lost, redo from the out/*.md reports with those decision ranges.
- When they finish: merge each into final/all-fixes (fetch + merge first, never force), validate, push onto PR #74, then publish the comparison Artifact.

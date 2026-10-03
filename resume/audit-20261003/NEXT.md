# Resume the attack-audit fix queue from GitHub only (written 2026-10-03 ~18:55 UTC)

Read this file, then RESUME.md (its log), then FIXRULES.md. Everything a new account needs is on this branch.

## Where things are
- Combined PR: #74, branch `final/all-fixes` (head 331b05c6 has audit batches A-D). Never force-push it. Merge commits only. No new PRs.
- Fix branches not yet merged into final/all-fixes (each based on 331b05c6):
  - `wip/audit-fixes-9` F9: h-cli-1/2/3 committed (9d4a3e7b D-1850, 18d8c2e0 D-1851, b660db78 D-1852). Final checks were running; mutation on its diff not yet confirmed. Re-run fmt/clippy/`cargo test -p cli --locked`, gates, mutation, then merge.
  - `wip/audit-fixes-7` F7: 3afa02c3 gate8 D-1509 done. Restarted 18:37 UTC on: excursion look-ahead, docs-web-01 4th, probeapi-1 body deadline, GAP17-33, h-api-1/2/3. If its branch head is still a "WIP (unvalidated)" commit, redo from that.
  - `wip/audit-fixes-5` F5: 88cbe235 GAP15-19 done + WIP commit. Remaining: GAP15-17 (new Base Evidence version), cli halves of AC-whp-tb-2 and ET-7.
  - `wip/audit-fixes-8` F8: e0ced8ac D-1830 done + WIP commit. Remaining: re-examine EVERY row of out/f8-triage.md for a real removal (not only "yes" rows); only items needing user-only facts may stay documented, and say why on the board.
- Not started: F6 (USER APPROVED 13:27 UTC "Fix them"): W2-cli5-4 one frontier-rule authority across cli/api/web; W3-runner2-8 column digest v2 (keep v1 by name); W3-runner2-7; W3-runner5-0. F10: h-eng-1 stale align docs; h-eng-2 Fibonacci rungs co-fire after paisa rounding vs implication-screen pruning (vocab version bump if semantics change). Hunter rerun: pull + store + lake (stopped before reporting).
- Queue order: finish F9 and F7, merge; then F5, F6, F8, F10, hunter.

## Files on this branch
- `FIXRULES.md` rules every fixer follows (paths in it point at the old scratchpad: replace with this directory). `RULES.md` audit-worker rules.
- `prompts/agent-briefs.md` EVERY agent brief used, verbatim (22). Reuse F5/F6/F7/F8/F10/Hunter briefs as-is, fixing paths. `prompts/gate.sh.md` = the CI gate extractor script (save as gate.sh), `prompts/union.awk.md` = ledger-tail conflict resolver.
- `out/*.md` all verifier/hunter/triage reports. `board/attack-audit.tsv.md` the item board (id, state found|fixing|branch|pushed|green, commit, note). `brutex-audit-ledger.html` Artifact source (claude.ai/artifact/K6QyihZvvh1d4UXQnApVZT, republish to the same URL if accessible, else a new one).

## Numbers owned by this work
Decisions: D-1480..1519, D-1620..1659, D-1800..1899 (used so far: 1480-1488, 1490-1509, 1620-1642, 1830, 1850-1852). Per fixer: F5 D-1643..1649+D-1800..1809 AGA-; F6 D-1810..1819 AGB-; F7 D-1510..1519+D-1820..1829 AGC-; F8 D-1830..1849 AGD-; F9 D-1850..1859 AHA-; F10 D-1860..1869 AHB-. Others own D-1520..1619 (sweep), D-1660..1759 (lane1 redo), D-1760..1799/D-1900..1999 (zero-audit loop). New work: D-2000+.

## How to check a batch
fmt --check; clippy --workspace --all-targets -D warnings; cargo test -p <crates> --locked (root box: permission tests via setpriv --reuid=65534); build gate 0 first and export SOURCE_SCAN; gates 0,1c,1d,10,11,12,14,23,27,27b; ci.yml < 524,288 bytes; cargo-mutants 26.2.0 + nextest 0.9.145 `--in-diff <diff file> --test-tool nextest --cargo-test-arg=--max-fail=1:immediate`. Ledger conflicts (docs/04,05,06 tails): keep both sides (union.awk).

## User rules (verbatim intent)
Rust only except web/. O(1) wherever possible, name what can't be. Work in parallel. Attack edge cases adversarially. Verify with real evidence, never guess. No new PRs except one combined PR. Never ask the user to tap/paste/approve. Keep running continuously. Fix everything fully: no partial, no documented-only. Stop at 98% weekly usage after saving state to GitHub. Deliver results as comparison-table Artifacts. Never push the GDFL reader branch while real GDFL data rows are in its history.

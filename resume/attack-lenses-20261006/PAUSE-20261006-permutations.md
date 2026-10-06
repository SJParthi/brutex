# PAUSE (4th, 09:23 UTC) — lens L3 permutations (attack/permutations) — 2026-10-06

Head: `attack/permutations` @ `24c7e3aa` = **WIP (paused, not validated)**. Last validated push: `eacf255`.

## Done (pushed, validated)
- R1 F-25A074 (D-3402), R2 F-3B4D5A (D-3403), R3 F-109F13 (D-3404) — see permutations.tsv.
- R4 doc fixes: F-36228D crossings by last definite side (D-3405, XPERM-05) and F-8D5073 vocabulary counts/kinds (D-3406, XPERM-06), commit 3c5c237 + eacf255.
- R4 refuted: OverlapWindow exit > i+H (latent; only Signal columns in tests/public edge; hardening note in D-3407).

## In WIP commit 24c7e3aa (needs validation)
- F-0486DA (D-3407, XPERM-07): Edge::worst_reward_risk_bp now returns 0 for n<2 (as payoff_bp). Test `runner::outcome::tests::the_worst_case_ratio_is_never_above_the_mean_ratio_on_any_small_sample` failed before (`[-30]: worst 9223372036854775807 above payoff 0`), passes after (5 ms); NOT DONE: the n=2 assertion (`edge_of_moves(&[5, 10]).worst_reward_risk_bp() == i64::MAX`, kills the `<`→`<=` mutant on the new guard) failed to apply at the pause; add it first after RESUME.
- Runner + cli suites (non-root) were running at the pause: log /tmp/claude-0/r4-downstream.log.

## Next steps after RESUME
1. Read /tmp/claude-0/r4-downstream.log (re-run `/tmp/claude-0/runtests.sh runner` and `cli` if the container was reclaimed); fix any pin that moved (cli elite asymmetry tests).
2. clippy -D warnings, fmt, gates; mutants on the runner diff: `cd /tmp/claude-0/wt-mut && git checkout -q --detach <head> && git diff origin/final/all-fixes...HEAD -- crates/runner/src/outcome.rs > /tmp/claude-0/r4.diff && cargo mutants --in-diff /tmp/claude-0/r4.diff --baseline skip --in-place --timeout 900 --cap-lints true -p runner`.
3. Replace "FIXED (sha pending)" for F-0486DA with the sha; push; update permutations.tsv.
4. Round 4 found defects, so run round 5 (fresh angles); repeat until a round finds zero; then RESULT-permutations.md.

## Restart
`git fetch origin attack/permutations final/all-fixes && git checkout attack/permutations`; tests as non-root via /tmp/claude-0/runtests.sh <crate> (script in PAUSE notes above if /tmp was lost: cargo test --no-run, copy binaries, run via setpriv --reuid=65534 --regid=65534 --clear-groups).

## Background result during pause
runner + cli suites (non-root) on the WIP source: == runner == runner done == cli == cli done ALLDONE 

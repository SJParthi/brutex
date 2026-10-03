# Group E mutants: partial results (stopped on coordinator instruction)

Branch lane1b/e, worktree /home/claude/wt/e, HEAD be01adc. Tree clean, no cargo-mutants process left.

## Counts
caught 0 / missed 0 / unviable 0 / timeout 0. No mutant finished testing.

- Run 1 (mut1, 63 mutants): baseline built and passed (99 filtered lib tests, 39 s). Each mutant rebuild then took about 13 min or more, because cargo-mutants built every cli integration-test binary at opt-level 3. I stopped it with SIGINT during mutant 1 (population_v6 selected_stored_oos_witnesses -> Ok(vec![])), and the tree was restored.
- Run 2 (mut2, 65 mutants, `-C --lib` so only the lib test target builds): one attempt was interrupted externally. The restart's baseline build of the cli lib had still not finished after 2 h (load average 18-31 on 4 cores) when the background time limit killed it.

## Finished test (committed, not yet compiled)
be01adc lifts run_route's NIFTY hand-off into `ledger_v6::preloaded_for` and adds `ledger_v6::tests::the_sizing_load_is_handed_to_nifty_once_and_to_no_other_family`. It replaces the run_route `== -> !=` mutant at ledger_v6.rs:336, which no test could reach, with a testable helper. That test was also added to LBE-10 in docs/04.

That commit has NOT been built: fmt was clean before the commit, but clippy and the test were stopped. The `super::` path fix was folded into the commit by amend.

## Not yet run (65 mutants, list in scratchpad/e/mlist2.txt)
None are known killed or survived. From static reading:
- run_route -> Ok(vec![]): expected caught by missing_policy_refuses_before_ledger_v6_market_sizing, since the worksheet would be missing.
- run_route -> Ok(vec![Default::default()]): likely unviable, because CommittedStoredSelectionV6 has no Default.
- population_v6 selected_stored_oos_witnesses (2 mutants): reachable only via strict_v6_one_oos_fold_serves_every_witness_of_its_cohort. Unverified.
- strict_v6_inputs size_sweeper and load whole-function replacements: likely unviable (no Default) or caught by strict_v6 fixture tests. Unverified.

Resume command (from a clean tree):
`cargo mutants --in-place --in-diff scratchpad/e/e.diff -p cli --cap-lints true --timeout-multiplier 2 -C --lib -- -- candidate_universe::tests:: pre_admission_data::tests:: population_statistics_v2::tests:: strict_v6_fixture_tests ledger_v6::tests::`

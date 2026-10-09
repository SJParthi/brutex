# fxb1 pause — branch audit/fx-b1, worktree /home/claude/wt-fx-b1, head 32896202

Validation state for all items: the lib suite ran once on the tree before the commits. 1532 tests passed and 4 failed. Fixes for those 4 are committed but have NOT been re-run: the bars extremes test, verifier_walks_the_index, the scrub_route page test, and the qualified history fixture. Also run after the commits: fmt is clean, the docs gates 10/10b/27/27b are OK, and gates 11/19 are OK. Clippy and the full `cargo test -p api --locked` have NOT been run.

Next step for everyone: run `cargo clippy -p api --all-targets --locked -- -D warnings` and `cargo test -p api --locked`. Then run `api-<hash> --ignored --nocapture latency_qualified_campaign` and fill the four placeholders.

- W1-api5-4: DONE f2324b9c (D-4439). Its kept-folds test fix still needs a rerun.
- W1-api1-4: WIP 32896202 (D-4440). Left: run `a_qualified_history_of_h_records_is_read_whole_for_every_answer`, then the ignored `latency_qualified_campaign_by_history_length`, then put the numbers in place of ⟨a14_*⟩ in docs/06-limits.md (D-1444 W1-api1-4 bullet).
- W1-api2-3: DONE 15c2dd2f (D-4434, plus the api::latency harness).
- W1-api3-0: DONE 871bdf2a (D-4441). Measured; sharding or retention is NEEDS-OWNER (a new cli journal layout).
- W1-api5-7: DONE 7deb5b6f, plus 26a5ffe1 for AHD-02 (D-4435). Its two test fixes still need a rerun.
- W1-api5-8: DONE 2d38c60a (D-4432).
- W1-api6-3: DONE c41d1ad5 (D-4433).
- W1-api1-6: DONE 43ef4971 (D-4442).
- o1api-4: DONE e344bb6f (D-4436).
- so1-4: DONE 63318ac9 (D-4437). Tested and measured at the ceilings; the cost itself is in cli and is unchanged.
- so1-5: DONE 486f338a (D-4438).
- rustonly-4: DONE 40669ba3 (D-4430).
- W1-api6-0: DONE 7deb5b6f (D-4435), the same fix as W1-api5-7.
- W1-api2-8: DONE d5cd85d9 (D-4443).
- r64-6: DONE 916083a5 (D-4431).
- Report file /tmp/claude-0/audit/out/fxb1-report.md: NOT-STARTED. Write it after validation.

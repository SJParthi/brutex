# defp pause note (worktree /home/claude/wt-dp, branch audit/def-p, base a7a27dc3)

| item | state | decision | commit |
|---|---|---|---|
| satk-3 | DONE | D-4550 (invariants DEFP-01, DEFP-02) | 386c4f07 |
| W1-pull3-4 | NOT-STARTED | - | - |
| W1-pull1-0 | NOT-STARTED | - | - |
| rnew-3 | NOT-STARTED | - | - |
| o1api-33 | NOT-STARTED | - | - |

Working tree is clean: no files are half-edited. No cargo process is running.

## What a resumer must know
- The next free ids are D-4551 and DEFP-03. New invariant rows go under "### Audit fixer defp" at the end of docs/04-invariants.md.
- satk-3 checks:
  - Passed: fmt; `clippy -p pull --all-targets`; `pull --lib fold::` (10 tests); `--test anchor` (28 tests); gates-doc.
  - Not run:
    - `clippy -p runner`. The resample.rs change is doc-only.
    - The edit to `crates/cli/tests/resample_matches_fold.rs`, which adds 7 and 75 minutes. cli is never built here.
    - validate.sh, and gates 1c, 1d and 14.
- Gate 11 also fails at base a7a27dc3, in runner trade.rs, outcome.rs and report.rs. No file this branch touched is listed.
- Plans:
  - W1-pull3-4: cap the lines `request_minutes::audit` emits, loudly, naming how many were cut.
  - W1-pull1-0: memoise `prepare_observed_with` per instrument by fingerprint, or measure and hold. Weigh D-2319 against D-0519.
  - rnew-3: add bench rows to pull/benches/ratio.rs and store/benches/ratio.rs.
  - o1api-33: prototype a RawValue columnar decode and measure it.
- Next command:
  ```
  cd /home/claude/wt-dp && CARGO_TARGET_DIR=/home/claude/t-dp CARGO_BUILD_JOBS=1 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 cargo test -p pull --lib request_minutes
  ```
  Run it after implementing the cap.
- The final report still needs writing at /tmp/claude-0/audit/out/defp-report.md.

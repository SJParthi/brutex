# fxd pause state — worktree /home/claude/wt-fx-d, branch audit/fx-d

- o1engine-20: DONE (368a103d, D-4480, FXD-01). Its engine mutation run is still owed with the engine run below.
- so1-1: WIP (2cfa70ae, D-4481, FXD-02/03). Code, docs, clippy, engine tests and bench are green. Left: gates, then mutants. Next step: `$S/rungate.sh "Gate 10 "` (also 11, 12, 14, 27, 27b), then `cargo mutants --in-place --in-diff <(git diff 21443a2a HEAD -- crates) -p engine ...` per FIXRULES. Kill any MISSED.
- so1-3: WIP (2cfa70ae, D-4482, FXD-07). Bench is green clean and under extra load, and both plants breach. Left: gates and mutants (bench-only change, shared with the engine run).
- W3-engine1-1: WIP (2cfa70ae, D-4483, FXD-08). `sort_level` is routed and FXD-08 is green. Left: gates and engine mutants.
- o1engine-22: WIP (2cfa70ae, D-4484, FXD-04/05). Vocab clippy, tests and bench are green. Left: gates, then `cargo mutants ... -p vocab`.
- o1engine-23: WIP (2cfa70ae, D-4485, FXD-06). Bench is green. Left: gates and vocab mutants.
- GAP16-26: WIP (2cfa70ae, D-4486, FXD-09). Runner clippy is green and the targeted runner tests (money, rank, report) pass. Left: full `cargo test -p runner --locked`, which the pause interrupted (takes >10 min, so run it in the background). Then gates, then `cargo mutants ... -p runner`.
- Final steps not started: `cargo fmt --all -- --check` (it was clean before the commit), the report at /tmp/claude-0/audit/out/fxd-report.md, and SubagentHandback.
- NEEDS-OWNER, to carry into the report: CLAUDE.md §3 rule 4 still names `engine::column::Column::support` as the production path. It is now `Column::support_each` (D-4481). This was not edited.

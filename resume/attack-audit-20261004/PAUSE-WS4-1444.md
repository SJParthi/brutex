# PAUSE ws4 attack-audit batch 3 — 2026-10-06 14:44 UTC (weekly 75% brake)

Branch **wip/audit-batch3**, pushed head **181e9e42** (contains c936e19c D-3695 and 181e9e42 D-3692..D-3694).
Validated on 0075064 (fmt, clippy -D warnings, all language-purity gates, every crate's tests non-root, doctests); later commits
re-checked per change: runner clippy + tests (bb40f17e), vocab clippy + tests (c936e19c), gate tools unit tests + Gate 6c + all gates (181e9e42).

## Mutation state (worktree /home/user/mutwt, detached at bb40f17e, root, --baseline skip; baseline of the cli filter verified green as root, 739/739)
- engine in-diff: 14/14 caught.
- runner rank_checkpointed_streamed: 17 caught, 1 unviable (3 survivors of the first run killed in bb40f17e).
- cli batch-3 functions, 55 mutants in 4 chunks (A 20 screen fns, B 18 SpanShare/AuditCache/load_audit_inputs/column_withholding_at_build,
  C 15 finish_written/append_locked/Selection V6, D 2 and_checkpoint). Chunk A running (/tmp/claude-0/mut-cli-A.log), at pause:
```
  7 /tmp/claude-0/mut-cli-A/mutants.out/caught.txt
  4 /tmp/claude-0/mut-cli-A/mutants.out/missed.txt
  0 /tmp/claude-0/mut-cli-A/mutants.out/timeout.txt
  2 /tmp/claude-0/mut-cli-A/mutants.out/unviable.txt
crates/cli/src/lib.rs:14190:14: replace < with <= in least_first
crates/cli/src/lib.rs:14203:5: replace calendar_holds -> bool with true
crates/cli/src/lib.rs:14203:5: replace calendar_holds -> bool with false
crates/cli/src/lib.rs:14205:38: replace <= with > in calendar_holds
```
  Survivors so far (4, listed above, not fixed per the pause): least_first `<`→`<=` (14190), calendar_holds →true, →false, `<=`→`>` (14203-14205). Chunks B, C, D not started.
- Not run here (left to CI Gate 18): the rest of the in-diff set (runner ~46, api 86, cli ~3000 merged F6/F8/F8b mutants).

## Ready to merge
Not yet: the cli batch-3 mutation chunks are unfinished and RESULT-20261006.md / RESUME.md / tracker rows are unwritten.

## Exact next step
1. Read /tmp/claude-0/mut-cli-A/mutants.out; then run /tmp/claude-0/mutcli.sh B|C|D with the --re and nextest filters in PAUSE-20261006.md
   (each < 2 h; --re for any chunk-A mutants not reached). Kill every MISSED/TIMEOUT with a test.
2. Write resume/attack-audit-20261004/RESULT-20261006.md (draft table in this session), append a RESUME.md section, run
   /tmp/claude-0/out/tracker.py <head> on the tracker, push fix-queue.
- Chunk A ended: 9 caught, 5 missed, 0 timeout, 2 unviable of 20 (rc 124); missed: crates/cli/src/lib.rs:14190:14: replace < with <= in least_first;crates/cli/src/lib.rs:14203:5: replace calendar_holds -> bool with true;crates/cli/src/lib.rs:14203:5: replace calendar_holds -> bool with false;crates/cli/src/lib.rs:14205:38: replace <= with > in calendar_holds;crates/cli/src/lib.rs:14218:85: replace > with < in final_selection_split

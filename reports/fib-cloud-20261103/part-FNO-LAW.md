# Part FNO-LAW: held by the 2 TB disk floor

Status: held by the 2 TB disk floor. Nothing was built.

Checked: 2026-10-30 23:37 IST, before any write, as the task's DISK FLOOR rule requires.

Command and output (`df -k /Volumes/WD_BLACK`):

```
Filesystem   1024-blocks       Used  Available Capacity   iused       ifree %iused  Mounted on
/dev/disk7s1  7813821744 6564863368 1248122536    85% 156630972 12481225360    1%   /Volumes/WD_BLACK
```

Arithmetic (shell `$(( ))`):

- Available: 1,248,122,536 KiB
- Less 50 GiB headroom for this build: 1,248,122,536 - 52,428,800 = 1,195,693,736 KiB
- Floor (2 TB): 1,953,125,000 KiB
- 1,195,693,736 < 1,953,125,000, so the build must not start. Shortfall: 757,431,264 KiB (about 722 GiB).

Not done (all of it):

- No worktree created (`wt/FIB-FNO-LAW`, branch `fix/fib-fno-law` do not exist from this run).
- No edits to CLAUDE.md, AGENTS.md, docs/00-charter.md, docs/05-decisions.md, docs/04-invariants.md, docs/07-plan.md.
- No `crates/core/tests/fno_law.rs`.
- No cargo command run, so no target dir grown (`tgt/fib-shared`, `tgt/fib-shared-deep` untouched by this run).
- No coverage, fmt, clippy or test evidence; this part has no benches by design.
- Nothing deleted to make room (the task forbids it).

To resume: free space must reach at least 1,953,125,000 + 52,428,800 = 2,005,553,800 KiB available on WD_BLACK (by the operator's choice of what to move; nothing may be deleted by this build), then rerun the part from the df check.

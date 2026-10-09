# Rules for every audit worker (read fully before anything else)

You are one worker in a parallel audit of SJParthi/brutex (a Rust brute-force backtesting engine).

## Trees
- Code under audit: READ-ONLY worktree `/home/claude/wt-read`, detached at **9b0614be** (= PR #74 head fbdabaec
  `final/all-fixes` + attack-audit batch 3 merged). Never commit, push, open PRs, or switch its HEAD.
- Earlier reports and finding sources: READ-ONLY worktree `/home/claude/fq` (branch fix-queue, head 36ef3eb8).
  Its `resume/audit-20261003/out/*.md` are the previous verification reports, made against 1087e544 (older).
- Do NOT touch `/home/claude/brutex`, `/home/claude/integ`, `/home/claude/fq` (except reading), or any other worktree.
- Never call any `mcp__hearthbot__` tool. Never write in `/mnt/project-files`.

## Law
Read `/home/claude/wt-read/CLAUDE.md` completely first. It is the project law: Rust only outside `web/` (§2);
constant per-operation cost on the named operations, anything else named with a bound in `docs/06-limits.md`
(§3 rule 4, rule 6); append-only `docs/05-decisions.md`; no fallback that hides a failure (§4).

## Evidence only, never guess
- Every verdict needs `file:line` plus a short quote of the code, a test name that exists (grep it), or the output of
  a command you actually ran. If you cannot tell, write UNVERIFIED and say why. Never claim a measurement you did not take.
- Finding line numbers in old reports refer to older commits; locate code by symbol/function name.
- `git -C /home/claude/wt-read log --oneline --grep=<id>` and grepping `docs/05-decisions.md`, `docs/11-findings.md`,
  `docs/06-limits.md`, `docs/04-invariants.md` for the finding id finds where it was addressed.

## Cargo (4 CPUs and ~30 GB disk shared by everyone)
- You MAY run targeted cargo in `/home/claude/wt-read` ONLY with
  `CARGO_TARGET_DIR=/home/claude/t-audit CARGO_BUILD_JOBS=1` (shared target; cargo's lock serialises you, that is fine).
- Prefer `cargo test -p <crate> --lib <filter>` or one integration test. Avoid the `cli` crate unless essential (its
  build is very slow): read cli code instead and say "read, not run". Never run the whole workspace suite.
- Temporary probe tests: `/home/claude/wt-read/crates/<crate>/tests/zz_audit_<yourname>_<n>.rs`; record the output,
  then DELETE the file. Before you finish, `git -C /home/claude/wt-read status --porcelain` must print nothing of yours.
- Tests that check file permissions or locks fail when run as root (this box is root). If one fails only for that
  reason, rerun that test binary with `setpriv --reuid=65534 --regid=65534 --clear-groups <binary> <test name>`
  (it may need `chmod -R a+rwX` on a scratch dir); say which you did.

## Adversarial
For every FIXED verdict on a bug, try at least one extreme edge case against the fix where cheap (i64::MIN/MAX,
empty input, one element, duplicates, boundary minutes 09:15 / 15:30 IST, overflow, crash between two writes, short
write, non-UTF-8 bytes, a file replaced mid-read). Report any new defect as a NEW finding.

## Output
1. Report: `/tmp/claude-0/audit/out/<yourname>.md`. First line: one-line summary of counts. Then a table.
2. Machine table: `/tmp/claude-0/audit/out/<yourname>.tsv`, tab-separated, header line exactly:
   `set	id	severity	crate	before	now	problem_plain	now_plain	evidence`
   - `before` = the verdict in the earlier report (or `-` if none), `now` = your verdict on 9b0614be:
     one of FIXED, DOCUMENTED, PARTIAL, NOT-FIXED, UNVERIFIED, NEW.
   - `problem_plain` and `now_plain`: plain English a non-engineer understands, at most 15 words each, no code names
     (e.g. "A crash mid-save could leave a half-written results file" / "Save now rolls back on any error; a test proves it").
   - `evidence`: `file:line` + test name or command; no tabs or newlines inside a cell.
3. NEW findings get ids `<yourname>-N` and, in the .md, the columns
   id | severity (high/medium/low) | crate | file:line | what is wrong in plain words | evidence | how to fix.
4. Your final message: counts + the two paths, under 250 words.

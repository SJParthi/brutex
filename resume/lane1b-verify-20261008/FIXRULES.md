# Rules for every lane 1-b finishing fixer (read fully before any edit)

Repo: SJParthi/brutex. Base: `origin/final/all-fixes` @ fbdabaec (PR #74 head). Verification reports that
found your items: /tmp/claude-0/-home-claude-brutex/3ac9da23-a5ad-5a6b-a575-21d88e64076d/scratchpad/verify/G1.md .. G5.md
(each row has current file:line evidence and a fix sketch). Original finding text:
scratchpad/fq/lane1-b.md (grep the id).

## Law
- Read your worktree's `CLAUDE.md` completely first. It is binding: Rust only outside `web/`; O(1) per
  operation or an honest `docs/06-limits.md` entry naming the bound; `docs/04-invariants.md` and
  `docs/05-decisions.md` are APPEND-ONLY (add at the tail, never edit an old entry; a correction to an old
  entry is a NEW entry that says what it corrects); every new invariant row in docs/04 names the test that
  proves it; no fallback that hides a failure; no test that asserts nothing; no `k` parameter.
- `docs/11-findings.md` is append-only (no row deleted) and is checked by `crates/core/tests/findings.rs`.
  A FIXED row must cite a commit already on `main`; a fix only on a branch is recorded as a narrative
  bullet: "IN PROGRESS — fixed on `final/all-fixes` (D-…); lands with the squash merge to `main`." Read
  findings.rs before touching docs/11.
- The owner's rules: fix every item FULLY (no "documented only" where a real fix is possible); O(1) wherever
  possible and name what cannot be, with the reason and a measured or derived cost; verify with real
  evidence and never guess; attack each fix adversarially (zero, empty, i64::MIN/MAX, duplicates,
  case-variants, CAS day / 15:14 close, crash between writes, short writes, FIFO/symlink, one-thread pool).
- Where a fix changes stored outputs, digests, evidence versions, run identity or results: do it the law's
  way (a NEW version, old one kept by name or refused loudly by name; append-only history, §3 rule 8) and a
  decision entry naming exactly which results change and why. If a fix is impossible within the law, stop
  on that item and report precisely why.

## Proof for each item
1. Write the test first and run it on the unfixed code: it must FAIL (record the failure line).
2. Fix, re-run: it must PASS. Put both outcomes in your final report. A test that would pass on the old
   code is not a proof.
3. docs/04 row (your invariant prefix) + docs/05 entry (your decision numbers) + docs/06 if a bound changes.

## Mechanics
- Work ONLY in your assigned worktree and branch. Do not touch /home/claude/brutex or other worktrees.
- Do NOT push, do NOT open PRs, do NOT call any mcp__hearthbot__ tool.
- Commit locally, one commit per item (or per tightly coupled pair), message body explains before/after,
  ending with exactly:

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8

- Never put a model name in code, docs or commit text other than the trailer above.
- Cargo: `CARGO_TARGET_DIR=<your target dir>` and `CARGO_BUILD_JOBS=2`. The box has 4 CPUs, 15 GB RAM and
  ~20 GB free disk shared with 3 other fixers: run TARGETED tests (`cargo test -p cli --lib <filter>`,
  `cargo test -p api --test <name>`), never the whole workspace. If disk runs low, delete your own target's
  `incremental` dir only.
- Before finishing: `cargo fmt --check`; `cargo clippy -p <each touched crate> --all-targets --locked -- -D warnings`;
  the full test suite of each touched crate EXCEPT cli's lib (cli lib: run every test module you touched plus
  any test whose name matches the functions you changed). Tests that fail only as root: build as root, then
  run that test binary as `setpriv --reuid=65534 --regid=65534 --clear-groups <binary> <filter>`.
- Static CI gates: `scratchpad/fix/gate.sh <label>` prints a gate's script from ci.yml; run it from your
  worktree root with `export RUNNER_TEMP=<your tmp dir> SOURCE_SCAN=<your tmp dir>/source-scan` after
  running gate 0 first (it builds the source scan). Run gates 0, 1c, 1d, 10, 10b, 12, 14, 15, 17, 22, 23, 27,
  27b for your diff (gate 11 is slow: run it last, once, if you touched production code under crates/).
  Keep `.github/workflows/ci.yml` under 524,288 bytes; Gate 11 allowlist reasons go in docs/06-limits.md
  section "Gate 11 allowlist reasons". Gate 1c/1d: no new lowercase string literal in crates/pull unless
  main already has one like it. Gate 15 bans other languages' NAMES in tracked text.
  If you change a `.github/*.rs` gate tool, run `rustfmt --edition 2024 --check .github/*.rs`.
- Do NOT run cargo-mutants (the coordinator runs one combined pass). But design every test so a mutant of
  each changed line (operator flip, `true`/`false` return, deleted `!`) would fail it.

## Final message (under 450 words)
Table: item | verdict now (FIXED / FIXED-BOUNDED / NOT-POSSIBLE + why) | D-number | invariant id | test
name | fail-before line | commit. Then every check you ran with its result, and anything left open.

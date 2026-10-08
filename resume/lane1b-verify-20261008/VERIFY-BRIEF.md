# Verify the 55 lane 1-b findings on the current PR #74 head

You are a read-only verification worker for the Rust repo at /home/claude/brutex
(GitHub SJParthi/brutex). The checkout is on branch `final/all-fixes-xp04wq`, which equals
`origin/final/all-fixes` at commit fbdabaec (PR #74's head). Read /home/claude/brutex/CLAUDE.md first.

## Background
On 2026-10-03 an audit (report: scratchpad/fq/v1.md) found 55 lane 1-b findings NOT FIXED on
final/all-fixes at 1087e544 (plus W2-cli8-6 PARTIAL). A redo then fixed them on staging branch
`final/all-fixes-00bbns` and merged it into final/all-fixes on 2026-10-04, with follow-ups
D-2100..D-2106. The Fix Board (scratchpad/fq/fixboard-lane1.tsv) lists the fix commit for each id.
Since then 1,000+ more commits landed on final/all-fixes, so a fix may have been lost, reverted,
weakened, or left a sibling site unfixed. Your job: establish with real evidence, on the CURRENT
tree, whether each finding in your group is fully fixed.

Files (all under /tmp/claude-0/-home-claude-brutex/3ac9da23-a5ad-5a6b-a575-21d88e64076d/scratchpad/):
- fq/lane1-b.md: the ORIGINAL finding text for each id (grep the id).
- fq/lane1-b-plan.md: the fix plan per unit.
- fq/v1.md: the 2026-10-03 NOT-FIXED evidence (file:line at 1087e544; line numbers have moved).
- fq/fixboard-lane1.tsv: id -> fix commit and D-number.

## For each id in your group
1. Read the original finding and the v1 evidence. Find the fix commit (`git show <sha> --stat`,
   `git log --all --oneline --grep=<id>`, and grep docs/05-decisions.md, docs/04-invariants.md,
   docs/06-limits.md, docs/11-findings.md for the id).
2. Check the CURRENT code (not the commit) does what the fix claims. Quote file:line on the current tree.
3. Find the test(s) that prove it and give their full paths (e.g. `cli::pool::tests::name` and the file).
   Read the test: does it actually fail if the fix is reverted? A test that asserts nothing, or that
   would pass on the old code, is a gap.
4. Attack it adversarially: look for a sibling call site with the same defect, an edge input the fix
   misses (zero, empty, i64::MIN/MAX, duplicate, case-variant, CAS day, crash mid-write, etc.), or a
   later merge that re-introduced the old shape. Grep for the old pattern across crates/.
5. If the original finding is about cost (O(n) vs O(1)): is the cost now O(1) in code, or only
   stated in docs/06-limits.md? If only stated, say whether a real O(1) (or cheaper bounded) fix is
   feasible and sketch it concretely (which function, what to cache/index, what invariant).

Verdicts: FIXED | FIXED-BOUNDED (cannot be O(1), honest limits entry names the bound) |
DOCUMENTED-ONLY (code unchanged, a real fix looks feasible) | PARTIAL | NOT-FIXED | REGRESSED.

## Rules
- READ ONLY. Do not edit any tracked file, do not commit, do not push, do not switch branches.
- Do NOT run cargo (another process is building; it would contend). Static evidence only; name tests.
- Never guess. If you cannot establish something, write UNVERIFIED and why.
- Do not call any mcp__hearthbot__ tool.
- Write your report to scratchpad/verify/<GROUP>.md with:
  - a table: | id | verdict | current evidence (file:line + short quote) | proving test(s) | fix commit / D-number | remaining gap + concrete fix sketch |
  - a "NEW" section for any new defect you find (id G<n>-k, severity, file:line, evidence, fix).
- Your final message: the verdict counts and every non-FIXED id with one line each.

# Exact agent prompts used by this thread

Every fix agent is started with the Agent tool, `isolation: "worktree"`,
`run_in_background: true`, and a prompt of exactly this shape. The briefs are in
`briefs/` here; copy them to the new session's scratchpad and fix the path.
Replace the base commit with the current head of `final/all-fixes-zero`.

## Ledger torn tails (D-1900..1919, ZL-) — RUNNING at save time, branch zero/ledger-tails
"Read <scratchpad>/LEDGER-BRIEF.md and carry it out exactly." Follow-up sent:
concurrency.md Pass 3 ledgers-1/2/3 and locks-1/2/3; crash-edge CE-3, CE-11, CE-31.

## Tests that cannot fail (D-1920..1939, ZT-) — RUNNING at save time, branch zero/test-teeth
"Read <scratchpad>/TESTS-BRIEF.md and carry it out exactly. Start from commit
<HEAD> (branch final/all-fixes-zero): run `git checkout -b zero/test-teeth <HEAD>`
in your worktree first. Commit your work on that local branch only; never push,
never open a PR, never create a tag. Return the list the brief asks for."

## Docs that state stale facts (D-1940..1959, ZD-) — DONE, merged (D-1940..1947)
Same shape with DOCS-BRIEF.md and branch zero/doc-truth.

## Autopilot and vendor pull (D-1960..1969, ZA-) — NOT STARTED (stopped for usage)
"Read <scratchpad>/COMMON-BRIEF.md first and follow it. Your branch:
`zero/pull-autopilot`. Your decision range: D-1960..D-1969. Your invariant prefix:
`ZA-`. Fix these confirmed findings. Evidence and suggested fixes are in
/mnt/project-files/zero-rounds/crash-edge.md (CE-*) and
/mnt/project-files/zero-rounds/tests-docs-security.md (P1-19-*), under the
matching `### ID` heading: CE-28 (vendor 429s floor Dhan's allowance to 1), CE-29
(non-2xx bodies never read, dead Dhan token undetected; CLAUDE.md §8), CE-30,
CE-14, CE-15 and P1-19-02, CE-16, P1-19-01, P1-19-03, and one shared empty-path
refusal plus one shared boolean-knob parser for CE-5, CE-6, CE-33, CE-36, CE-37,
CE-38, CE-39. No real vendor network calls, no credentials: use the loopback
listeners and recorded fixtures the crates already use."
(CE-23 and CE-24 were fixed by the main thread in D-1767.)

## Web pages and API routes (D-1970..1979, ZW-) — NOT STARTED
COMMON-BRIEF, branch zero/web-routes. Findings: CE-25, CE-26, CE-27, CE-32,
P3-01-01, P3-02-01, P3-02-02, P3-01-02, P3-02-06, P3-01-03, P3-01-04, P3-01-05,
P3-02-03, P3-02-04, P3-02-05, P3-02-07, P1-06-01, P1-06-02, P1-06-03.
"Web code under web/ may be JS/Svelte; everything under crates/ is Rust."

## cli and lake crash-edge (D-1980..1989, ZE-) — NOT STARTED
COMMON-BRIEF, branch zero/cli-edges. Findings: CE-4, CE-7, CE-8, CE-9 (design
call: never record a host limit as a property of the run; state it), CE-10,
CE-12, CE-13, CE-17, CE-18..CE-22, CE-34, CE-35. (CE-6 moved to the pull agent's
shared boolean parser.) "Do NOT take CE-3, CE-11 or CE-31."

## Numeric (D-1990..1999, ZN-) — NOT STARTED
COMMON-BRIEF, branch zero/numeric. Findings in
/mnt/project-files/zero-rounds/numeric-complexity.md. Priority: floor-vs-max
class (p2bool-1, p2inst-1, p2idx-1, run3-1, D-0743 pbo_ppm; div_ceil for
MAX-gated only, MIN-gated keep floor; stored bytes change so a decision entry and
CLAUDE.md §3 rule 8 apply); D-0742 refuse block > periods; mediums pst-1, grk-1,
run1-1, xcut-1; then the rest. NAME, do not guess: pst-3, clib-1, clib-2, gaps-7.

## Concurrency follow-up
Already in the ledger agent's follow-up list above.

## Rule for agent count
The user allowed up to 2 fix agents at a time (coordinator, 18:37 UTC) until 93%
weekly usage; save at 93%, stop at 98%. The main thread also fixes directly.

---

## 2026-10-04 agents (launched 04:20 UTC, worktree isolation, base bde50c0c)

Both prompts share these sections, copied from briefs/COMMON-BRIEF.md with:
branch `git checkout -b <branch> bde50c0c`; never push; CARGO_BUILD_JOBS=2;
build only in the agent's own worktree target; tests only for touched modules;
non-root for permission tests; read-only /mnt/project-files; install nothing;
every fix gets a break-tested test; no silent fallback; decisions appended as
`### D-NNNN — title — 2026-10-04`; invariant rows with the agent's prefix;
gate 11 / pedantic clippy rules; api emit-site accounting; fmt + clippy per
touched crate; mutants if installed; commit per finding with the attribution
lines; on a stop message, finish the current edit, commit and report.
Return: branch head, then one line per finding (FIXED / ALREADY FIXED / NOT
FIXED + why, commit, test, break-tested?), then what was not run.

### api-routes (branch zero/api-routes, D-1970..1979, prefix ZW-)
Task: fix P3-01-02 (routes silently drop known WireBody fields: refuse by name
any known field a route does not consume, like `strict_knobs`; keep the page
working), P3-01-03 (client disconnect during sweep/descent/command admission
records Cancelled/0 while the detached admission still launches: never record
Cancelled while the handler can still dispatch), P3-01-04 (masters refresh
runs its ladder inside the request and the page aborts at 90 s: detach like
`recovery::start` or bound below the page ceiling; always record and reload),
P3-02-01, P3-02-06, P3-02-07. Evidence: /mnt/project-files/zero-rounds/
tests-docs-security.md (### headings by ID). Already fixed -> say so with
evidence.

### numeric (branch zero/numeric, D-1990..1999, prefix ZN-)
Task: from /mnt/project-files/zero-rounds/numeric-complexity.md and numeric/:
1. floor-then-max-gated class p2bool-1, p2inst-1, p2idx-1, run3-1, D-0743
   pbo_ppm (round UP only max-gated fields; reuse D-1769's
   `Cell::avg_loss_magnitude_ceil` / `Crossings::adverse_ppm_ceil_at` shape;
   numeric/pass3/p3floor.md lists ~20 SAFE fields not to touch);
2. D-0742 (block length > period count must refuse);
3. mediums pst-1, grk-1, run1-1, xcut-1;
4. then run1-2, run1-3, run2-1, run2-2, pst-2, pst-4, grk-2.
Operator-decision items pst-3, clib-1, clib-2, gaps-7: NOT FIXED with the exact
question. Golden fingerprints re-taken only where the test's comment allows.

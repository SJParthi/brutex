# Prompts and agent briefs: workspace-audit thread (2026-10-03)

Everything a fresh session needs to restart this thread's work from GitHub alone.

## 1. The user's own words (Parthiban Subramanian), in order

1. The thread's opening ask (verbatim):
   > Are you sure can do a deep in depth detailed thorough research by spinning and activating all agents and all sub-agents to ensure the guarantee and assurance… I dont need any hallucination or illusion… Try to attack evrythign… Always achieve O(1) everywhere. Always work on everyhtgin parallely. Ensure to use one and only RUST O(1) in the entire workspace codebase except frontend alone… make eveythign as 100 percentage common runtime dyanmic incremental scalable approach… cover all kinds of extreme permuations combiantions of excpeitons errors situations conditions bugs scenarios ideas… Provide the artifact as the detailed extreme easy understandble comparison… automated easy table level comapriosn view…
2. "See run activate and spin all the agents and subagents espeiclly to fidn all the bugs gaps scenarios fucntioanlities eveyhtign entirley dude okay?"
3. "bro what about finding all the gaps fixes bigs scenairons defects and 100s aof agents run and 500 bugs defects criticlaities hapepnded ddude?"
4. "take care of the 5 horu usage and continue as well once its reset ddue okay? montiro the usage continuosuly dude okay?"
5. "dude along woth all tehse even with p99 also can your eally achieve O(1) dude okay?Always achieve O(1) everywhere…"
6. "why idle dude why?" (twice)
7. "can you lsit me as of now how many agents are runnign can you shwo me the progres sover thread itself how many agents runnigna dn tis progress dud eokay?" Every status line must show the running-agent count and progress.
8. "i celalry todl you to fox and resolve eevryhtiunf dude okay? no partly fixed dude okay? … i need zero errors zero gaps zeor defects zerp blcoks …" So nothing may stay partial or documented-only.
9. "dude slwo the agents and queue it accoridngly ebcause see the 5 hour usage dude okay?"
10. "okay so whenevr it touches 98 percnetage weekly you wil lsav eevryhtignr ight becuase tomrorow 12.30 pm usign a dfiferent claude code account i need tor esume dude that too from brutex rigth so evrythgin shdou lbe entitley saved udpated progreess statsu eveyhtign alogn with prompts a swell right dude"

Standing project rules (from the project's first message):
- Rust only, except web/.
- O(1) wherever possible, and name anything that can't be.
- Work in parallel.
- Attack edge cases adversarially.
- Verify with real evidence; never guess.
- No new PRs, only the one combined PR #74 (branch final/all-fixes). Fetch and merge before pushing, and never force-push.
- Never ask the user to tap, paste or approve anything.
- Keep work running continuously.
- Stop at 98% weekly usage after saving resume state to GitHub.
- Deliver results as easy comparison-table Artifacts.
- Never push the GDFL reader branch while real GDFL data rows are in its history.

## 2. Coordinator directives this thread followed

These are the coordinator's notes, not the user's words.
- Fix all 91 findings: high first, then medium, then low. Prove each with a test.
- Add p50/p99/max measured at small and huge sizes as an Artifact column.
- No partial or documented-only fixes. Where code truly can't fix something, say exactly why.
- Agent caps:
  - 13:14 UTC: at most 3 agents.
  - 13:19 UTC: 1 agent until 18:00 UTC.
  - 18:37 UTC: 1 agent until weekly usage hits 93%, then save, and stop at 98%.
- Before pushing a batch to final/all-fixes, send the commit to the PR #74 CI thread first. Its session is cse_01TPJRnnkg5yuNeRBHDyzaaP; it may be gone in a new account, in which case coordinate through the project chat.
- Keep /mnt/project-files/fix-board/status/sweep.tsv current, with one line per finding: id, state (found|fixing|branch|pushed|green), commit. That path exists only in this project; STATUS.md here is its copy.
- Gate 15 bans spelling other programming languages' names in tracked text. Write "another language" instead.
- Do not edit api Slots::try_take, LimitedListener::accept, audit_one or calendar_of.rs, or .github/workflows/ci.yml. The CI thread owns them.

## 3. Worker rules

FIXRULES2.md in this folder is the full rule set every fix worker gets. RULES.md holds the audit-worker rules.

## 4. The four fix-worker briefs (verbatim except paths)

Each worker uses its own git worktree on audit-fix/wN, based on final/all-fixes. Each first reads FIXRULES2.md.

### w6 (api), DONE on origin audit-fix/w6 @ 700e644, not yet on PR #74
D-1551..D-1555 (used 1551-1553); AFF-01..AFF-19 (used 01-04).
1. **hunt-api-2:** shutdown waits at most 10 s, then names what it abandons (D-1582), but the running sweep has no cancel flag. Add a real cooperative cancel. The sweep or job the api launches checks a cancel signal at its structural boundaries: per instrument-month or per batch, never inside the innermost bar loop (gate 17). It then stops and records a named, loud "cancelled" outcome. Test that shutdown during a sweep cancels it promptly and that the outcome names the cancellation.
2. **hunt-api-3:** cross-site failure logging is capped at 50 lines a minute (D-1583), but same-origin clients are still logged in full. Bound the same-origin path too, with a counted, named suppression line. Test both paths.
3. **errpaths-4:** Widths::new refuses swapped widths (D-1546), but public fields still let a caller build a swapped Widths. Make the fields private, route the degraded-path tests through a constructor or a test-only builder, and prove a swapped Widths can't be built outside the module.

### w7 (cli), DONE on origin audit-fix/w7 @ 858c8bb, not yet on PR #74
Commits so far: 5b6ce01 (D-1569) and 6516811 (D-1556, D-1557). D-1556..D-1559 and D-1569; AFF-20..AFF-39.
1. **hunt-conc-1:** sweep-all files rows in walk order (D-1564), but range-all and pool still file in completion order. Make both deterministic, and test byte-identical output across thread counts and reruns.
2. **hunt-conc-2:** the ordered phases were left as "needs transaction re-plumbing". Do the re-plumbing, and test the ordering under concurrency.
3. **hunt-cli-a-5:** not addressed yet. Read it in hunt-cli-a.md, verify it at HEAD, and fix it with a test that fails first.
4. **o1surface2-1:** cli descend rebuilds a column per step. Remove the repeated rebuild correctly, for example with a cache keyed by the full identity. Test that a mismatch rebuilds and a match reuses, and update 06-limits. Return BLOCKED only with a precise reason.

### w8 (data/engine), DONE on origin audit-fix/w8 @ c6d03c6 (attackdata-4, attackdata-8, o1eng2-1 fixed; gaps-1, gaps-3 BLOCKED; final checks not confirmed)
D-1570..D-1575; AFF-40..AFF-59. It was in the middle of attackdata-4, adding the serde_json feature and running the pull suite.
1. **attackdata-4:** don't use a lossy f64 path. Use arbitrary_precision or raw number text, and test values an f64 would round. Check that cargo deny still passes.
2. **attackdata-8:** add a new on-disk format version beside the old one, with old files still readable. Test round-trips and old reads, and document it in 02-store-format.
3. **o1eng2-1:** make the windowed computation incremental (O(1) amortised per step). Prove its results equal the old ones, and update 06-limits. The Newey-West effect stays UNVERIFIED.
4. **gaps-1 and gaps-3:** wire each unwired module into its real caller, with a test proving it is reached. Otherwise return BLOCKED, naming the missing decision.

### w9 (features), NOT STARTED, branch audit-fix/w9 at a21d031
D-1576..D-1579 and D-1593..D-1594; AFF-60..AFF-79.
1. **gaps-5:** add an out-of-sample split plus a multiple-comparison correction across the 210-instrument pool. Reuse runner's bootstrap, Romano-Wolf, White and SPA code. Test that a planted in-sample-only winner fails out of sample.
2. **gaps-11:** add an automated handoff from discovery to qualification, with a test.
3. **gaps-10:** add a Selection V6 results route in crates/api and a page under web/. The route must refuse equities loudly (CLAUDE.md §1). Test it.

Free D-numbers left in this thread's range: D-1554, D-1555, D-1595..D-1599, D-1615..D-1619. Invariant prefix AFF has AFF-80..AFF-99 free.

## 5. Earlier one-off briefs (finished)
- **p99 worker:** timed every claimed-O(1) operation at a small and a huge size, with at least 10,000 samples after warm-up, measuring p50/p99/p99.9/max. Probes are in p99-probes.md and results in p99.md.
- **mut1 worker:** killed the 32 Gate 18 survivors (28 in indicators pattern.rs, 4 in api audit_one) with 2 tests. The ledger is mutants-killed.md. It is on PR #74 in 0cab319.

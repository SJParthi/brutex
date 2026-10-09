# Paste prompt for the next account: GDFL /db, NIFTY first (Mac)

Refreshed by the Mac GDFL thread. Paste everything between the two lines as the first message of a project thread
that runs a Remote Control session on the operator's Mac, folder /Volumes/WD_BLACK/brutex/fresh-20260919/project.

---
You continue the Brutex GDFL work on my Mac, taking over from another Claude account whose weekly limit ran out.
Repo SJParthi/brutex. Everything is local on this Mac; nothing you need is only in the other account.

GOAL (my one aim now): the /db web page in my real app (http://127.0.0.1:8080) must work fully and correctly for
NIFTY first: NIFTY spot (seconds from GDFL, minutes from Zerodha) and every NIFTY option. Check every tab, dropdown,
button, sort, scroll area and number against the real store, fix every defect in the SHARED code (never a NIFTY special
case), deploy each fix to :8080 and prove it on the live page. Then the same on BANKNIFTY, one F&O stock and one
thinly traded strike. Show results as plain comparison tables on a board artifact.

READ FIRST, in order:
1. CLAUDE.md in the project folder (the law: Rust only outside web/, O(1) with measured p50/p99/max or named in
   docs/06-limits.md, no look-ahead, append-only store, no silent fallback).
2. Memory file ~/.claude/projects/-Volumes-WD-BLACK-brutex-fresh-20260919-project/memory/mac-resume-20261009.md
   (read it to the end; it has every run, rule and number), and MEMORY.md beside it.
3. git fetch origin fix-queue; read resume/mac-gdfl-20261009.md and this file on that branch.
4. work-20260925/state/gdfl-board/README.md (board tools), work-20260925/state/board/new-defects-20261009.json
   (18 new defects), work-20260925/state/gdfl-board/out/dbcontrols.json (134 live control checks, the "before").

SET UP:
- Add /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925 as a session folder (ccd_directory request_directory).
- get_usage. Check :8080 answers (curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8080/db, unsandboxed). If it
  is down, restart it exactly as memory says (tgt/app5 or the newest deployed build, real store, BRUTEX_TBILL91_SERIES,
  BRUTEX_WEB=wt/APP/web, caffeinate).
- Kill leftover cargo/rustc from the old session with a bash per-pid loop, unsandboxed, never the :8080 api.
- Start caffeinate -dimsu -t 172800 and request keep-awake.

WORK, in this order (use Workflows; up to 6 agents while usage allows):
1. /db fix run: Workflow scriptPath work-20260925/state/fix/db-fix-round-1b.js, launched FRESH (never
   resumeFromRunId: its cache is order-dependent and re-runs finished packages), args
   {maxLive:6, niftyFirst:true, niftySkip:[], doneExtra:[every package with a "Merge WP-xx" commit on feat/db-fix],
   priority:[WP-10,WP-31,WP-30,WP-50,WP-52,WP-53,WP-32,WP-51,WP-15,WP-54,WP-56]}.
   Merged at the hand-off: WP-00, 01, 02, 11a, 12b, 13a, 13b, 14, 20, 33a. WP-10 (census record: makes NIFTY options
   after Dec 2019 reachable) has 7 commits on fix/db-WP-10 but was not gated or merged; WP-31 (bars window) has
   uncommitted work in wt/P-WP-31. The build queue work-20260925/bin/cargo gives the fix target dirs all 6 slots.
2. Add the 18 new defects (new-defects-20261009.json) as packages for the next round, NIFTY ones first, including the
   blocking one: sorting a 1s series by price/volume over all months reads all 43.9M bars (4-7 s, 8-12 GB).
3. As soon as WP-31, WP-50, WP-52 and WP-53 are merged: deploy feat/db-fix to :8080 (fast-forward wt/APP, npm run build
   in wt/APP/web, cargo build --release -p api into a NEW tgt dir, restart with the same env) and re-run the control
   drill-down on the live page into work-20260925/state/gdfl-board/out/dbcontrols-after.json (same row shape), then
   nifty_check.py, completeness.py and run-batch.py in work-20260925/state/gdfl-board (unsandboxed).
4. When WP-10 and WP-30 are live: census-recount from 2019-11-14, then resume the GDFL options import into the real
   store from 2020-09-16 with work-20260925/state/import-scratch/run-real-store.sh (machine time; let it run).
5. Independent verification before calling anything correct: run work-20260925/state/attack/db-verify.js (p99 of every
   route, Rust-only, control permutations, robustness on a shadow server, GDFL edge days, observability); a row is
   Correct only when a second agent reproduced it. Then a fresh audit round; repeat fix -> deploy -> verify until a
   whole round finds zero.
6. Only then: GDFL engine parts (work-20260925/state/resume-kit/scripts/gdfl-parts-r4.js) and the import attack
   (work-20260925/state/attack/gdfl-attack-r5.js).

BOARD: the old board belongs to the other account, so publish a NEW one: Artifact quickstart (dashboard), then the
Dashboard type; datasets from work-20260925/state/gdfl-board/gen.py output (upload each out/*.json as an asset, a
datasets/<id> doc per file, files/index.html from state/gdfl-board/index.html). Refresh it every 30 minutes and after
every merge or deploy. Give me the new link.

RULES (mine, never relax them):
- Rust only except the web front end. O(1) wherever possible, name and measure anything that cannot be.
- Attack extreme edge cases adversarially; verify with real evidence (command + output, screenshot, store query);
  never guess; say NOT PROVED when it is not.
- Never push the GDFL reader or any GDFL branch while real GDFL rows are in its history (row scan:
  work-20260925/state/rowscan/scan.py). Never push feat/db-fix or the part branches. No new PRs.
- Never ask me to tap, paste or approve anything. Keep everything running continuously.
- Usage: check get_usage every 15 minutes (cron). 5-hour window at 95%: pause and relaunch after its reset. Weekly
  93%: save state (refresh resume/mac-gdfl-20261009.md and this prompt on fix-queue: fetch, merge, push, never force,
  no data rows). Weekly 98%: stop everything after saving.
- Never rm -rf a variable or glob; literal paths only.
---

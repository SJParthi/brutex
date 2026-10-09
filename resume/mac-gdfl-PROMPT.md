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
3. git fetch origin fix-queue; read resume/mac-gdfl-20261009.md and resume/mac-gdfl-PROMPT.md on that branch.
4. work-20260925/state/gdfl-board/README.md (board tools), work-20260925/state/board/new-defects-20261009.json
   (18 new defects), work-20260925/state/gdfl-board/out/dbcontrols.json (134 live control checks, the "before").

SET UP:
- Add /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925 as a session folder (ccd_directory request_directory).
- get_usage. Check :8080 answers (curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8080/db, unsandboxed). If it
  is down, restart it exactly as memory says (newest deployed build, real store, BRUTEX_TBILL91_SERIES,
  BRUTEX_WEB=wt/APP/web, caffeinate).
- Kill leftover cargo/rustc from the old session with a bash per-pid loop, unsandboxed, never the :8080 api.
- Start caffeinate -dimsu -t 172800 and request keep-awake.
- Run one small Workflow agent first to confirm this account can run agents: on 9 Oct the last agents failed with
  "Your organization has disabled Claude subscription access for Claude Code". If that recurs, say so plainly and stop.

WORK, in this order (use Workflows; up to 6 agents while usage allows):
1. /db fix run: Workflow scriptPath work-20260925/state/fix/db-fix-round-1b.js, launched FRESH (never
   resumeFromRunId: its cache is order-dependent and re-runs finished packages), args
   {maxLive:6, niftyFirst:true, niftySkip:[], doneExtra:[every package with a "Merge WP-xx" commit on feat/db-fix],
   priority:[WP-30,WP-50,WP-52,WP-53,WP-32,WP-51,WP-54,WP-56,WP-55,WP-11b,WP-34,WP-37,WP-38b]}.
   Check resume/mac-gdfl-20261009.md (its last "Update" section) for what merged last: 17 of 37 at 16:55 UTC 9 Oct.
   WP-30 (census serving) had uncommitted work in wt/P-WP-30 and was the last thing running.
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
6. AUTOMATE INGEST AND THE CHECKS. The design is written but NOT yet reviewed: work-20260925/state/design/ingest-automation.md
   (1,273 lines, 128-row input case matrix, packages IA-0..IA-14). First run an adversarial review of it (claims checked
   first-hand, every O(1) claim, every case) and revise it; answer its section 17 questions with sensible defaults and say
   which; then build the IA packages in its order. AUTOMATE THE CHECKS (operator, 9 Oct): stop relying on agents to prove the data. Port the checks Claude ran by hand
   (work-20260925/state/gdfl-board/reconcile.py: rebuild a day's one-second bars from the raw GDFL file with the D-2802
   late-row rule and compare price/volume/OI with the store; completeness.py: every trading day present against the
   archive's day list and the calendar, no gaps or duplicates, journal seconds = stored seconds; the Zerodha minute
   checks) into Rust cli verbs, run automatically at the end of every GDFL import day and every Zerodha pull, written
   to the store's audit as one record per instrument-day (O(1) to look up), and shown on /verify and as a per-day
   verified/failed badge on /db. Then any pull or import is captured, stored, checked and visible with no agent in
   the loop. Record it as a decision; Rust only; tests that fail when the check is broken.
   NOTE: the design says reconcile.py places a late row at the running maximum, which it calls different from D-2802
   (its test GV-04 pins the difference). Check that first-hand before trusting the 29-day accuracy table.
   ALSO GDFL FROM THE INGEST PAGE (operator, 9 Oct): today Zerodha pulls start from /ingest (POST /pull/run), but a GDFL
   import starts only from the command line (cli gdfl-import via state/import-scratch/run-real-store.sh) and /ingest
   only shows its journal (/imports.json). Add a GDFL import form to /ingest that runs the same Rust import: source
   (CM zip archive with nested per-day zips, tick store, options yearly zips), kind (indices, stocks, options), date
   range, resume from the last finished day; seconds from each file's own date and the row times (D-2802 late rows),
   option names mapped to expiry/strike/side by the one decoder (1,218,362 archive names already decode one way);
   live progress, refusals listed by name, the automated per-day checks above, all O(1) per day/file with measured
   p99, one authority shared with the cli. Attack it adversarially (corrupt and missing zips, duplicate days, renamed
   files, holidays, special sessions, expiry days, restarts mid-day) and prove it on :8080 before calling it done.
7. Only then: GDFL engine parts (work-20260925/state/resume-kit/scripts/gdfl-parts-r4.js) and the import attack
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
  93%: save state (refresh resume/mac-gdfl-20261009.md and resume/mac-gdfl-PROMPT.md on fix-queue: fetch, merge, push, never force,
  no data rows). Weekly 98%: stop everything after saving.
- Never rm -rf a variable or glob; literal paths only.
---

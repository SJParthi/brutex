# Paste prompt for the next account: GDFL NIFTY on /db + ingest automation (Mac) - refreshed 10 Oct 2026 19:00 IST

For the fresh account starting 11 Oct 2026, 12:30 IST. The 10 Oct account may still merge packages until its weekly limit;
read git (feat/db-fix "Merge WP-"/"Merge IA-" commits) for the true state, not this note.

Paste everything between the two lines as the first message of a project thread that runs on the operator's Mac
(folder /Volumes/WD_BLACK/brutex). Prefer a Claude desktop-app session in BYPASS PERMISSIONS mode (a Remote Control
session cannot use bypass and raises approval cards; the operator does not want cards).

---
You continue the Brutex work on my Mac (repo SJParthi/brutex; everything is local except the resume notes on GitHub).
Use ultracode: multi-agent workflows for every substantive step, every result reproduced by a second agent.

READ FIRST, in order (newest commit of each, branch fix-queue): resume/mac-gdfl-20261010.md (read every Update section to
the end), then resume/mac-gdfl-20261009.md, then this file. Memory: ~/.claude/projects/-Volumes-WD-BLACK-brutex/memory/
(MEMORY.md, gdfl-nifty-db-20261010.md, goal-self-sufficient-ingest.md). Add /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925
as a session folder first.

HAND-OFF FACTS (10 Oct 20:15 IST, weekly 97%): 33 of 50 /db packages merged on feat/db-fix (head de9f590a): WP-00 WP-01 WP-02 WP-10 WP-11a WP-11b WP-12a WP-12b WP-12c WP-13a WP-13b WP-13c WP-14 WP-15 WP-20 WP-30 WP-31 WP-32 WP-33a WP-33b WP-33c WP-34 WP-35 WP-36 WP-37 WP-38a WP-38b WP-50 WP-51 WP-52 WP-53 WP-54 WP-55.
Committed on their branches but not merged (commits ahead): WP-39(9) WP-81(3) IA-5(3). Live on :8080: deployed app8 d73e1112543ffd1cdb7549e9ff075d6392370911 at 2026-10-10T13:05:17Z (later merges are NOT
deployed). The 10 Oct runs (fix wf_22e529d2-a94, blind-spot hunt wf_1b621293-02b, GDFL attack wf_b419e32f-fbd, design loop
wf_e0313932-312) belong to the old account and stop at its limit: relaunch db-fix-round-1b.js FRESH with doneExtra = the merged
list + WP-70. Store steps done: census migrate+recount (2020 NIFTY options live), time index for 142 NIFTY/BANKNIFTY months.
Zerodha repair code is live (GET /pull/repair.json?feed=zerodha&instrument=NIFTY&from=2018-01-01&to=2026-09-30 lists 42
sessions); running it needs that day's Kite access token.

STATE (local, never pushed): branch feat/db-fix in work-20260925/wt/DBFIX; each package fix/db-<WP> in work-20260925/wt/P-<WP>
(half-done work stays there; the fix prompt reuses it). Plan state/board/fix-plan.json; findings state/board/audit-result.json
(343 confirmed: DB-*, DB-N01..N26, DB-A01..A53, DB-B01..B79). Run script state/fix/db-fix-round-1b.js (args: maxLive,
niftyFirst, doneExtra, priority, only, noMerge, portBase). Relaunch it FRESH (never resumeFromRunId) with doneExtra = every
package with a 'Merge WP-xx'/'Merge IA-xx' commit on feat/db-fix plus WP-70. Packages in it but outside one run: round 3
(WP-62, WP-13d, WP-12e, WP-12f, WP-81) and IA-5 - include them in the next fresh run so it merges them itself.
Deploy: state/fix/deploy-8080.sh app<N> (new N each time; never roll back to app5 - it hides 139,295 instrument-months).

PRIORITY 1 (NIFTY on /db): WP-39 1s sort, WP-32 option analytics, WP-52 bars grid, WP-53 option cells, WP-16 import speed,
WP-13c Zerodha repair, then deploy and re-check every NIFTY /db control live into state/gdfl-board/out/dbcontrols-after.json,
each row reproduced by a second agent. Then the store steps (operator authorised all of them on 10 Oct, in his words: "Yes,
run the store steps on the real store: census migrate and recount, the GDFL re-imports and options import, the Zerodha repair
re-pull, and the sort-index backfill"): options import ONLY with an explicit range from 2020-09-16 (never over 2018-09..2020-09),
the 12-day + six alias-index back-fill through the versioned repair (WP-12d/WP-12f), Zerodha re-pull of the holes (needs that
day's Kite sign-in), WP-39 sort-sidecar backfill. Prove NIFTY completeness (GDFL seconds, Zerodha minutes, options).
PRIORITY 2 (ingest automation, the operator's core aim: the app imports GDFL from /ingest itself, fast and self-checking):
design state/design/ingest-automation.md (3 revision rounds; latest critic verdict in state/design/critic-round*-20261010.md);
close any remaining blocking gaps, then build IA packages to milestone M1 (GDFL import from /ingest), M2 (every day verified),
M3 (Zerodha verified), M4 (holes repaired).
THEN: the remaining /db packages, round 3, blind-spot hunt round 2 (wf script under the session workflows; dedupe vs
state/blindspots/known-index.json + confirmed-r1.json), GDFL import attack r5 (state/attack/gdfl-attack-r5.js), PR 74
(resume/pr74-ci-thread.md, head 2a986236, next D-4184).

BOARD: https://claude.ai/artifact/QykmH2ZyFNzw8VsPx6ghL2 (belongs to the 10 Oct account; if you cannot publish to it, publish
a new one from state/gdfl-board/board2.html and give me the link). Refresh: gen.py (env BOARD_WF, BOARD_DBFIX), assure_gen.py,
cases_gen.py, board2.py; the plain view reads out/glance.json and out/tracks.json - update their numbers from evidence.
Plan page: https://claude.ai/artifact/FtnutnQEi1yaD392pkoFBS.

RULES (mine): Rust only except web/. O(1) at p99 everywhere, measured p50/p99/max, any slow path named in docs/06-limits.md.
Work in parallel. Attack every extreme case. No claim without real evidence; say NOT PROVED. Everything saved, audited,
searchable, on the board as plain comparison tables. Never ask me to tap or approve. Save state to fix-queue resume/ after
every merge; check usage every 10 minutes; keep working until usage reaches 100%, with a final save just before.
---

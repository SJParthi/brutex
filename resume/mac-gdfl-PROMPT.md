# Paste prompt for the next session: GDFL NIFTY on /db + ingest automation + compressed store (Mac) - refreshed 12 Oct 2026 12:30 IST

For a NEW Claude Code session (any account) from 13 Oct 2026. The 11-12 Oct account reached its weekly limit on 12 Oct
(resets 18 Oct 12:30 IST). Read git (feat/db-fix "Merge <pkg>" commits) and state/fix/deploys.log for the true state.

Paste everything between the two lines as the first message of a session that runs on the operator's Mac (folder
/Volumes/WD_BLACK/brutex). Prefer a Claude desktop-app session in BYPASS PERMISSIONS mode: a Remote Control / auto-mode
session raises approval cards, and sub-agents that hit a card hang for hours.

---
You continue the Brutex work on my Mac (repo SJParthi/brutex; everything is local except the resume notes on GitHub).
Use ultracode: multi-agent workflows for every substantive step, every result reproduced by a second agent.
Do not ask me to tap or approve anything except the two things named under MY YES below.

READ FIRST, in this order:
1. git fetch origin fix-queue, then resume/mac-gdfl-20261011.md on that branch: read EVERY "Update" section to the end
   (the last three carry the relaunch recipe, the compression design and the browser checks). Then this file.
   Locally the branch is checked out at /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/FQ (detached; push
   notes with: git -C <that folder> push origin HEAD:fix-queue).
2. Memory: ~/.claude/projects/-Volumes-WD-BLACK-brutex/memory/ (MEMORY.md, gdfl-nifty-db-20261010.md,
   goal-self-sufficient-ingest.md, tick-store-resume.md).
3. Add /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925 as a session folder. Call it W below.

WHERE THINGS ARE (all local, never pushed): integration branch feat/db-fix in W/wt/DBFIX; one worktree per package
W/wt/P-<pkg> on branch fix/db-<pkg> (half-done work stays there and is reused); real store
/Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store; the live app on http://127.0.0.1:8080 serves W/wt/APP/web and
a frozen build W/tgt/app<N>. Plan: W/state/board/fix-plan.json (118 packages). Findings: W/state/board/audit-result.json.
Designs: W/state/design/ingest-automation.md, critic-round7-20261011.md, rules-audit-20261011.md,
store-compression-20261012.md. Reports of every builder and second check: W/state/fix/round2/.

STATE ON 12 Oct 12:30 IST:
- 50 of 118 packages are merged on feat/db-fix (head 9ebc1a04, which includes two gate commits after the last merge):
  WP-00 WP-01 WP-02 WP-10 WP-11a WP-11b WP-11c WP-12a WP-12b WP-12c WP-12d WP-12e WP-13a WP-13b WP-13c WP-13d WP-14
  WP-15 WP-16 WP-20 WP-30 WP-31 WP-32 WP-33a WP-33b WP-33c WP-34 WP-35 WP-36 WP-37 WP-38a WP-38b WP-39 WP-50 WP-51
  WP-52 WP-53 WP-54 WP-55 WP-56 WP-56b WP-57 WP-58 WP-60 WP-61 WP-81 IA-5 IA-11a IA-11r IA-17.
- Live: read the last line of W/state/fix/deploys.log (app16 = 7b709b14 with 48 packages; app17 with all 50 was being
  built at hand-off). If the last line is not head, deploy: W/state/fix/deploy-8080.sh app<N+1> (new N every time),
  then append "deployed app<N> <sha> at <UTC time>" to deploys.log.
- Two packages FAILED their third check and are NOT merged; their work and reports are kept: WP-62
  (round2/verify-WP-62-r3.json) and WP-12f (round2/verify-WP-12f-r3.json). They need one more repair and check.
- A full-gate run on feat/db-fix was still finishing (W/state/fix/round2/gate.json: fmt, clippy, deny, web gates pass;
  tests 8,552 passed and 3 failed for reasons of this Mac; mutation testing not finished). Read it; finish what it left.
- Store steps DONE on the real store: index back-fill (NIFTY 1s 1,997 of 1,997 sessions, 44,012,990 bars), sort index
  for NIFTY and BANKNIFTY spot, options import for NIFTY and BANKNIFTY from 2020-09-16 to 2026-09-24 (1,495 days,
  3,720,834,600 seconds, 95 failures by name in W/state/fix/store-steps/real-options-import-20261012.log).
- That options import took 518 GB uncompressed. I was promised: NO stock-options import until the compressed form
  exists, and a disk projection before every store step.

DO NEXT, in this order:
1. Deploy if needed (above), then open http://127.0.0.1:8080/db?feed=gdfl&seg=spot&sym=NIFTY&tf=1s in a browser and
   confirm it shows 44,012,990 bars (the segment key is "spot", not "index").
2. Relaunch the fix run FRESH (never resumeFromRunId): Workflow with scriptPath W/state/fix/db-fix-round-2.js and args
   {doneExtra: [every package with a "Merge <pkg>" commit on feat/db-fix], noCritic: true,
    roundBase: {"WP-62": 3, "WP-12f": 3}, maxLive: 5}.
   Its order is already set: SC-0 first (no time index and no sort index for a month under 1,024 bars: stops 208 GiB
   of small files), WP-62 and WP-12f repairs, then the ingest chain to milestone M1 (IA-1, IA-2, IA-3, IA-16, IA-10a,
   IA-12a: the app imports GDFL from /ingest by itself), then SC-1 to SC-12 (compressed store), then the R3 packages.
   The script's context already tells agents: delete nothing, write the report before tidying, commit work in progress
   every 15 minutes, skip the one slow cli test. Keep those paragraphs.
   Dry-run first: node W/state/fix/wfsim/sim.mjs W/state/fix/db-fix-round-2.js ok (sim2.mjs takes the args as JSON).
3. After every merge: deploy if it touches the NIFTY page or /ingest, refresh the board, append an Update section to
   resume/mac-gdfl-20261011.md (or a new dated note) and push it to fix-queue.
4. When the compressed store (SC packages) is merged and rehearsed on a private copy: show me the measured saving and
   ask my yes before any sealed month's old files are removed.
5. The finish line I asked for: wipe nothing of mine, but PROVE on an empty private store that the app alone (no
   Claude) imports the GDFL zips from /ingest from scratch and the result equals today's store; judge it against every
   rule below; only then tell me the automation works.

MY YES (ask me once, in plain words, yes or no; my earlier yes does not carry into your session):
- any write to the REAL store (remaining steps: alias-index and shares back-fill, sort index for the rest, re-reading
  the 94 files of 2020-09-16, the Zerodha re-pull, which also needs my own Kite sign-in that day);
- any deletion (zips, older build folders under W/tgt, tools-work copies, scratch stores, old files of sealed months).
  Rehearse each store step on a private copy first: W/state/fix/shadow.sh makes one and deletes nothing.

FACTS THAT SAVE YOU HOURS:
- Inside the Bash sandbox a direct connection to 127.0.0.1 is refused: use
  curl --noproxy '' --proxy "$HTTP_PROXY" http://127.0.0.1:<port>/... ; deploy and cargo need the sandbox off.
- Sub-agents: GET only on :8080, never write the real store, never rm -rf / find -delete / git worktree remove.
- Real GDFL rows never go into the repo; GDFL branches are never pushed; only resume notes go to fix-queue.
- Usage on a Max plan: one 5-hour window is about 28% of the week; 4 to 5 lean agents use about 0.2% of the window a
  minute. Check usage every 10 minutes; save state to fix-queue BEFORE the limit, because you stop too.
- Known open weakness, measured on app16: the NIFTY options view opened with no expiry downloads the whole store list
  (53 MB, 8 to 18 s); with an expiry in the address it is 5 requests of 1 to 2 ms. Owner: package R3-12.

BOARD (plain comparison tables for me): https://claude.ai/artifact/1YHn8RcqBCewUGfNEqztEk and the design page
https://claude.ai/artifact/2ArMami2wYAt3HhFpX616d belong to the 11 Oct account. Rebuild with
python3 -I W/state/gdfl-board/board3.py "<one-line note>" and publish W/state/gdfl-board/board3.html; if you cannot
publish to that link, publish a new artifact from the same file and give me the new link. Speed rows:
python3 -I W/state/gdfl-board/speed_live.py app<N> 200.

MY RULES: Rust only except web/. O(1) at p99 everywhere, measured p50/p99/max, any slow path named in
docs/06-limits.md. Work on everything in parallel. Cover every extreme case, error and combination, including the ones
nobody listed. Everything saved, audited, logged, searchable, monitored and shown on the board as easy comparison
tables any person can read. One common, runtime, dynamic, incremental, scalable approach, no special cases. No claim
without real evidence: say NOT PROVED when it is not. Keep working until usage reaches 100%, with a final save before.
---

# Context shared by every ingest + /db part (24 Oct 2026)

This is the background every part in `reports/db-specs-20261024/` shares. Read it after `CLAUDE.md`
and `README.md`, and before your part's `<NAME>.md`.

**Order of authority.**

- `CLAUDE.md` is the law.
- `README.md` says how a cloud session works: GO and HOLD, branches, gates, reporting.
- Section 4 of this file ("Current state, 24 Oct 2026") wins over the older background text in
  section 2 wherever the two differ.
- Your part's file binds its own scope. If it contradicts a rule in section 4, follow section 4 and
  name the contradiction in your report. Never pick silently.

## 1. brutex only

Every part here is built in the **brutex** repository and nowhere else. The operator said it again on
24 Oct: "only brutex".

- The cloud environment these sessions run in may be labelled "TickVault". That is only the
  environment's name.
- Never clone, edit, branch or push the tickvault repository.
- `docs/10-shared-core.md` names the crates other projects may consume (`core`, `vocab`, `indicators`,
  `engine`, `greeks`, `costs`). If your part touches one of them, that document's contracts bind. The
  work still lands only here.

## 2. The background text, 7 to 14 Oct 2026 (verbatim)

This is the context embedded in the Mac's build launcher (`EMBED.context`). It has not changed since
14 Oct.

- It is byte for byte the `CONTEXT.md` of the 16 Oct cloud specs
  (`origin/cloud/db-base-20261016:reports/db-specs-20261016/CONTEXT.md`), checked with `diff` on 24 Oct.
- **One edit for this public repository:** the two commit co-author lines carried an e-mail address.
  Each is replaced below by `<address removed: see README.md>`. Use the co-author line `README.md`
  gives.
- The Mac paths in it (`/Volumes/WD_BLACK/...`, `wt/`, `tgt/`, `state/`, ports, shadow servers,
  `bin/cargo`) exist only on the operator's Mac. `README.md` ("What differs in the cloud") says what
  replaces them.
- Section 3 lists what in it no longer holds.

```text
CONTEXT (brutex, 7 Oct 2026; RESUMED 9 Oct 2026 on a new Claude account after the weekly usage limit stopped round 1 mid-run). The operator wants /db — and everything it depends on — entirely correct: every selection on Zerodha and GDFL (indices, stocks, expired options with moneyness, intrinsic, extrinsic, IV, greeks, OI) shows exactly the right data; everything searchable, findable, auditable, logged and visualised; O(1) per operation with measured p50/p99; every extreme case covered. A read-only audit confirmed 185 defects (each verified twice); the fix plan splits them into work packages with non-overlapping files.

FILES: plan = /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/board/fix-plan.json ; findings with full evidence = .../state/board/audit-result.json (key "confirmed"). Integration worktree /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/DBFIX on branch feat/db-fix (from feat/app-all plus the Zerodha minute-calendar fix). The operator's real app (:8080) serves wt/APP (feat/app-all) — never touch it; it is redeployed by the main session after merges.

REPO LAW (CLAUDE.md, binding): Rust only outside web/ (web/ is unrestricted, but no crate may need Node to build/test); no invention — vendor/exchange claims need a source or UNVERIFIED; run identity includes feed; O(1) per operation; append-only store (new versions/generations, never in-place mutation); no look-ahead; fallbacks never hide a failure (degrade loudly with the reason, or refuse); no test that asserts nothing; prices are paisa i64; decisions live in docs/05-decisions.md (WP-02 writes D-3316..D-3343 up front; the Zerodha calendar fix uses D-3344+). Never copy raw vendor rows into the repository; fixtures are synthetic.

OPERATOR ANSWERS (defaults the operator can still override; WP-02 records them):
1. The GDFL options import was STOPPED by the operator at 23:23 IST on 6 Oct, mid-day 2020-09-16. Bars after 2019-11-13 are on disk but uncounted; they get counted by the census v4 recount (no vendor reread), then the import resumes from 2020-09-16.
2. Store writes and API rebuild/restart are approved in principle ("fix and resolve everything"); they run in the main session after the code lands (WP-80), each as a new month/generation, never in place. Agents never write the real store.
3. GDFL closing-auction prints (from 2026-08-03): keep them, as a labelled 'auction' field/row kind — never silently dropped, never mixed into ordinary trades.
4. Spot for an option bar: exact second stays the default (D-2807); add an opt-in 'last spot at or before t' that always shows the spot's age. Never a later spot.
5. Yes: import the GDFL index files (FINNIFTY, MIDCPNIFTY, NIFTYNXT50, NIFTYIT, NIFTYCPSE, NIFTYFPI, INDIA VIX) and the old-name stock files (TATAMOTORS, HDFC, ZOMATO and the others) — stored, not swept — so every option underlying gets spot and greeks.
6. Zerodha 2015–2017 index 1day bars: the operator's window is 2018-01-01..2026-09-30. Hide them by default; a visible toggle shows them labelled 'outside your window'. Never delete.
7. Dhan, Groww and TrueData: removed from every picker (the operator uses only Zerodha and GDFL).
8. NSE circulars for the 2020-03-13 / 2020-03-23 halts, renames and index aliases: WP-02 may look them up (WebSearch/WebFetch on nseindia.com / nsearchives); record the circular number and URL if found, else keep UNVERIFIED and show 'suspected'.
9. 30 May 2019 LTQ=1 marker rows and LTQ=0 price rows: flag only; bars unchanged.
10. OI of a 1s bar: the last row of the second (bar-close semantics), applied to new imports; existing months are re-imported as new generations by WP-80.
11. /db opens on NIFTY · Spot · the feed's finest timeframe · newest first.

HARD RULES: never write the real store /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store; never POST to any server; never start, stop or restart :8080; never kill a process you did not start; never push; never rm -rf outside your own worktree, target dir or shadow dir. Commands that bind ports or write outside the sandbox may need dangerouslyDisableSandbox. The machine is shared by ~12 agents: keep builds to your own CARGO_TARGET_DIR and avoid needless rebuilds.


RESUME FACTS (9 Oct): WP-02 is committed and merged into feat/db-fix (b875f22e), so D-3316..D-3343 exist. WP-70 is done (outside the repo). Every other package's first agent was cut off by the usage limit; six of them (WP-00, WP-01, WP-10, WP-11a, WP-12b, WP-14) left UNCOMMITTED work in their worktrees, backed up as patches in /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/resume-kit/wip-20261009/. The Mac rebooted on 8 Oct; :8080 was restarted by the main session on the same build (tgt/app5) and real store. BUILDS: export PATH=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/bin:$PATH before any cargo command, so builds go through the machine-wide queue (8 at once, 4 jobs each; queued waiting is normal, not a hang). Start long cargo commands in the background writing a log that ends with a line EXIT <code>, then wait with one blocking `until grep -q '^EXIT ' <log>; do sleep 20; done` call (600000 ms timeout, repeated). SAFETY: never rm -rf "$HOME", "$TMPDIR", ~ or any unquoted variable or glob; use literal absolute paths inside your own worktree, target or shadow dir. Never commit or copy a real GDFL data row (Ticker,DD/MM/YYYY,HH:MM:SS,price...) into the repository: fixtures are invented, with prices that cannot be mistaken for real ones.

ROUND 1c (10 Oct, new account): 17 packages are merged (feat/db-fix 3cd0623c). The 18 defects found on 9 Oct are now findings DB-N01..DB-N18 in audit-result.json, added to WP-50, WP-52, WP-53, WP-56 and a new package WP-39 (bars sort index: the blocking full-history 1s sort). Co-author line for commits: 'Co-Authored-By: Claude Opus 5.5 (1M context) <address removed: see README.md>'.

ROUND 2 (11 Oct 2026, a new account; where this paragraph and the text above disagree, this paragraph is right). 33 packages are merged on local feat/db-fix (WP-00 WP-01 WP-02 WP-10 WP-11a WP-11b WP-12a WP-12b WP-12c WP-13a WP-13b WP-13c WP-14 WP-15 WP-20 WP-30 WP-31 WP-32 WP-33a WP-33b WP-33c WP-34 WP-35 WP-36 WP-37 WP-38a WP-38b WP-50 WP-51 WP-52 WP-53 WP-54 WP-55); read git -C /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/DBFIX log --oneline --grep='^Merge ' feat/db-fix for the true state. The confirmed findings are now 343 (DB-*, DB-N01..N26, DB-A01..A53, DB-B01..B79). The operator's live app :8080 runs a frozen copy of a build (tgt/app<N>) over the real store and is redeployed by the main session after merges, never by you. This run also builds the INGEST AUTOMATION packages (IA-*): the operator's core aim is that the app itself imports GDFL from /ingest, fast and self-checking, with no Claude session in the loop; their specification is state/design/ingest-automation.md.

OPERATOR'S STANDING RULES (11 Oct, binding on every package): (1) O(1) per operation at p99 everywhere, MEASURED: report p50, p99 and max with n and the method; anything that cannot be O(1) is bounded or refused out loud and named for docs/06-limits.md. (2) One common, runtime-dynamic, incremental, scalable code path: no instrument or vendor special case, no hard-coded list the data already knows, no work redone that an index or a sidecar can remember. (3) Every state change is saved, audited, logged, searchable and visible on a page; nothing fails silently. (4) Every extreme case is covered by a test that fails before the fix: empty, single, maximum, duplicate, out-of-order, late, torn, truncated, missing-in-the-middle, renamed, special session, concurrent, killed mid-write, disk full, garbage parameters. (5) No claim without evidence a second agent can repeat; say NOT PROVED otherwise. A SECOND AGENT re-runs your gates, reproduces every 'fixed' finding on a fresh shadow server and attacks your change before it is merged.

PORTS AND SANDBOX (11 Oct): test servers left by earlier runs still listen on 8100-8142, 8231, 8347 and 8372; never trust an answer from a port you did not just start, and never stop a server that is not your own package's. Inside the Bash sandbox a direct connection to 127.0.0.1 is refused ('Operation not permitted'). Reach your own shadow port either through the sandbox proxy - curl --noproxy '' --proxy "$HTTP_PROXY" http://127.0.0.1:<port>/... with 127.0.0.1:<port> listed in that command's allowed_domains - or by running the command with dangerouslyDisableSandbox. Starting a server (binding a port) needs dangerouslyDisableSandbox. Never change permission settings; never ask the operator to approve anything: if a command is refused, find another way that stays inside these rules, or report it in notes. Commit co-author line: 'Co-Authored-By: Claude Opus 5.5 (1M context) <address removed: see README.md>'.

USAGE BUDGET (11 Oct): the account's usage window is small and an agent cut off by it loses everything it has read. Work economically: grep before you read; read by line range, never whole large files; never print a whole log (tail -40, or grep for the failures); do not re-read what you have read; run each gate once after your last edit, not after every edit; keep throwaway exploration out of your context. If an earlier attempt of your package left commits and uncommitted changes in the worktree, they hold up to 30 minutes of an interrupted agent's work from today: start from them. Commit your work in progress on your own branch at least every 15 minutes (message 'WIP <package>: <what>'; squash before your final report), so a cut-off loses little. Aim to have the package committed within about 45 minutes; if a part cannot be finished well in that time, commit what is complete and proved, and list the rest exactly under notes as NOT DONE.

KNOWN SLOW TEST (11 Oct, measured by a second agent): cli's pool::tests::the_pool_verb_prints_its_whole_page_on_a_generated_store ran for more than 95 minutes on this loaded machine and held up every package that runs the cli suite. Unless your package changes crates/cli/src/pool* or what that test reads, run the cli suite as: cargo test -p cli --locked -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store , and say in your gates line that this one test was skipped as the known slow one. The final Gate run on feat/db-fix runs it. Never wait more than 25 minutes on one test run without looking at which test is still running (the log's last 'test ... has been running for over 60 seconds' lines name it).

NO DELETIONS THAT NEED APPROVAL (12 Oct, learned the hard way): a command with rm -rf, or any deletion run with dangerouslyDisableSandbox, raises an approval card that nobody answers, and the agent that sent it hangs for hours. NEVER run rm -rf. Do not delete browser profiles, scratch worktrees' build output, target directories or shadow stores at the end: leave them where they are and list their paths in your notes; the main session cleans up. This holds inside the sandbox too: find -delete, git worktree remove, git clean and rm of a directory all raised cards. Delete nothing at all; if you need a clean directory, make a NEW one with a new name. Write your report file BEFORE any tidying. The helper scripts state/fix/shadow.sh, serve.sh and stop.sh delete nothing (shadow.sh moves an older shadow of the same name aside): use them as written, and always make a FRESH shadow for a check; never reuse one another agent left. If any command has not returned after 20 minutes with no output, treat it as stuck, do not repeat it, and finish your report without it.

RELAUNCH 13 OCT 2026 (a new session; where this paragraph and the text above disagree, this paragraph is right). 50 packages are merged on feat/db-fix (head 29dc84e6 at launch); git -C /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/DBFIX log --oneline --grep='^Merge ' feat/db-fix is the truth. :8080 runs a frozen build tgt/app18 and is redeployed by the main session only. WP-62 and WP-12f failed their third check (round2/verify-<pkg>-r3.json): their builders repair first. The compressed-store design (state/design/store-compression-20261012.md, section 9) names packages that collide; those are now dependencies (IA-1 after R3-35; R3-06 after SC-0 and SC-2; SC-3 after R3-08; SC-5 after R3-07 and R3-08; R3-22 after SC-4) and each affected package's spec says what to do. DISK (operator's rule): before any step that writes many files (a backfill, an import, a sweep, an index build), project its disk use and file count from a measured sample and write the projection in your report; no import of stock options happens until the compressed form exists. The operator restated the standing rules on 13 Oct, word for word in spirit: O(1) at p99 everywhere, measured; everything in parallel; every state change saved, audited, logged, tracked, searchable, monitored and shown on a page as plain comparison tables; one 100% common, runtime-dynamic, incremental, scalable path; and every extreme permutation and combination of exceptions, errors, situations and conditions covered by a test, including the out-of-the-box ones nobody listed.

ROUND 3 ADDENDUM (14 Oct 2026). The operator's MAIN AIM: one press of Ingest (and a daily timer) runs the whole chain end to end with no human or Claude step: GDFL Dropbox download, verify, compress into the tick store, ticks to 1-second bars; Zerodha daily + 1-minute + the folded 2-60 minute candles; every check; the pages. Everything compressed with O(1) per lookup at p99; a fresh clone plus only the Dropbox link and the daily Kite sign-in rebuilds code and data. Packages R4-01..R4-29 come from /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round3-packages.json (read its notes for SHARED LINES, SEQUENCED EDITS and split items). FAILURE MATRIX /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/e2e-failure-matrix.json: every row whose owner is your package, one of your item ids, or (for round-2 packages) your package id is part of your package: make its test exist and pass, or report it NOT DONE with the reason. AMENDMENTS: A1 IA-12a depends on R4-01 and adds its panel as web/src/routes/ingest/panels/GdflImport.svelte with no +page.svelte edit. A2 SC-8 depends on R4-01 and R4-04, puts its band at web/src/routes/ingest/panels/Storage.svelte and its /db badge at web/src/routes/db/badges/Storage.svelte, edits neither +page.svelte, and uses server.rs for route lines only. A3 IA-6 and IA-16 add their codes' rows to crates/pull/src/reasons.rs if R4-05 merged first. A4 SC-2's sealed summary section carries R3-06's month summary bytes with their version; no R4 package edits sealed.rs. A5 R3-32 ships no tracked config/launchd/*.plist (CLAUDE.md section 2, gate 1). A6 decision numbers are taken at landing from the ledger's head (D-4928 is already used; the head is past D-4950). Never delete any file; never write the real store.
```

## 3. What in that text no longer holds

Everything not named in this table still holds:

- the repo law;
- no invention;
- append-only history;
- no look-ahead;
- no fallback that hides a failure;
- paisa `i64`;
- synthetic fixtures only;
- the operator's standing rules of 11 and 13 Oct (`README.md` now carries them verbatim);
- the disk-projection rule;
- delete nothing, never `rm -rf`;
- OPERATOR ANSWERS 2 to 11, as the defaults they were.

OPERATOR ANSWER 1 describes the store as it stood on 7 Oct. What the store holds now is for the Mac to
check. The same goes for the 13 Oct clause "no import of stock options happens until the compressed
form exists" (RELAUNCH 13 OCT): it is a store-operation rule of the Mac's main session, not a code
rule. No part here imports anything into a real store, and which imports have run since is for the
Mac to check.

| The text says | Now (24 Oct) | Source |
|---|---|---|
| ROUND 2 and the ROUND 3 ADDENDUM: the app imports GDFL "with no Claude session in the loop", and "one press of Ingest (and a daily timer)" runs the chain "with no human or Claude step" | Ingest is attended: Claude Code or the operator runs one cli command from a terminal. Nothing unattended is built: no timer, no watcher, no scheduler, no chain started by a button or a route. | D-6900 (20 Oct); section 4.3 |
| ROUND 3 ADDENDUM: "a fresh clone plus only the Dropbox link and the daily Kite sign-in rebuilds code and data" | There is never a fresh-from-scratch rebuild: a new vendor link is a patch into the existing store. The disaster path is a verified backup with a restore drill. | D-7135, D-7137 (23 Oct); sections 4.4 and 4.5 |
| AMENDMENT A1: IA-12a adds `web/src/routes/ingest/panels/GdflImport.svelte` | IA-12a is dropped and never built: its panel called only the dropped `/gdfl/*` routes. | D-6900 item 1 |
| A6: decision numbers are taken at landing from the ledger's head (and REPO LAW's "WP-02 writes D-3316..D-3343 up front; the Zerodha calendar fix uses D-3344+", and the IA-* rule of the Mac's `fixPrompt` step 3 to "re-grep the highest D-48xx") | Each part takes ids only from its own block in the decision-number registry. | registry of 18 Oct (OPS-08); section 4.9 |
| USAGE BUDGET: a WIP commit at least every 15 minutes, "squash before your final report", "committed within about 45 minutes" | Commit and push your own branch at every save point: about every 20 minutes, and before any long test. Never squash or rewrite what you pushed; the Mac's merge-back folds the commits. Work until the part is done, with no time box. | no-rework rule (21 Oct); `README.md`; section 4.8 |
| The co-author line "Co-Authored-By: Claude Opus 5.5 (1M context) ..." | The line in `README.md`. | `README.md` |
| "never push" | Push only your own branch, `cloud/db-<NAME>-20261024`. | `README.md` |
| OPERATOR ANSWER 7: "only Zerodha and GDFL" | Still true for DATA. Dhan appears only as the broker whose brokerage, with the NSE and SEBI statutory charges, prices a trade (CLOUD-CHG1, merged 22 Oct). Never pull data from, verify against or build a data feature for Dhan, Groww or TrueData. | operator, 6 and 13 Oct |
| `:8080` on `tgt/app18`, ports, shadow servers, the `bin/cargo` queue, shared target pools, and the KNOWN SLOW TEST skip | These are Mac facts. The cloud has no store, no `:8080` and no shadow server: use plain `cargo` in your checkout. The slow cli test (`pool::tests::the_pool_verb_prints_its_whole_page_on_a_generated_store`) was timed only on the loaded Mac. Run the whole suite here. If a test runs past 25 minutes, name it in your report; never skip it silently. | `README.md` |

## 4. Current state, 24 Oct 2026

Everything here binds a builder of code. Pacing, accounts, usage, pages and progress counts are left
out: they are the Mac's business.

### 4.1 The integration head

- **`feat/db-fix` is `e851da1ca`** (Merge DATA-2, 23 Oct 23:12 IST), and `origin/feat/db-fix` is the
  same commit.
- **The truth for what is merged** is
  `git log origin/feat/db-fix --first-parent --merges --format=%s` (subjects `Merge <NAME>: ...`).
- **Merged since 22 Oct, newest first:** DATA-2, LEDGER-1023 (the decision records D-7135 to D-7138),
  BASE-GATE11, CLOUD-NOPY1, CLOUD-MUTAPIA, CHG-0, IA-9, IA-16, R3-24, R3-17, BASE-REDS-1021, CAL-1,
  SC-4, CLOUD-CHG1, PIT-1.
- **Branch from the newest `origin/feat/db-fix`.** If it has moved past `e851da1ca` by the time you
  start, the newer head is the fact.
- **The last head the Mac gated green is `e0668a4bd`.** That gate covered fmt, clippy, the workspace
  tests, `cargo deny`, web test and web build. One store timing test failed under machine load there
  and passed three reruns, recorded as FLAKE-CONFIRMED.
- **`e851da1ca` is not gated yet.** It adds only DATA-2. Its gate was stopped on 24 Oct because the Mac
  was overloaded.

### 4.2 Stocks, indices and options together (20 Oct)

The operator, 20 Oct ~09:50 IST: "include entire stocks, indices, options ... don't put options as the
second step, put everything combined together". Ingest and /db now cover stocks, indices and options
in one build.

- **The 18 Oct split that parked options is withdrawn.** No part narrows itself to stocks and indices.
- **An options half is real work with real tests,** never a stub.
- **A mixed part that merged narrowed, or was held back by a cut, carries its options half as a
  separate `-OPT` part** (the 20 Oct options replan, `state/options-20261020/opt-packages.json`). In this
  set that is CLOUD-CHG1-OPT: the F&O remainder of CLOUD-CHG1, which merged on 22 Oct.
- **One code path for every instrument kind** (standing rule 2). An option, an index and a stock go
  through the same shared code, never a special case for one of them.
- **The 18 Oct backtest-first cut is kept.** A BACKTEST-FIRST or CUT LINK note in your part's file
  binds.

### 4.3 One command, attended (D-6900, 20 Oct)

Ingest is always driven by Claude Code, or by the operator from a terminal. So everything that exists
only to run with nobody watching is dropped.

- **Dropped, never built:** IA-7, R4-12, R4-14, R4-15 and IA-12a.
- **Split:** R4-11, IA-10a, IA-16, R4-13, R4-10, IA-10b, R4-23, R4-26 and R4-28 keep their data half and
  drop their unattended half.
  - Each of these specs ends with its exact KEEP and "DROP, DO NOT BUILD" text. That text binds over
    the part's title.
  - For example, IA-10a's title still names the `/gdfl/*` routes, the child mode and the alert routes.
    All three are dropped; only its read side is built.
- **Kept, whoever runs the chain:**
  - crash-safe resume of a cut import;
  - an idempotent rerun that writes nothing twice;
  - locks against two imports at once;
  - the disk floor;
  - verification of every file against the vendor;
  - loud, named refusals;
  - the store format and compression;
  - /db correctness;
  - one command for the whole chain, `cli ingest-run` (R4-11). It is not merged yet, and no part here
    builds a substitute for it.
- **Never add a way to start or watch a chain unattended:** no timer, watcher, scheduler, daemon,
  launchd or cron entry, keep-awake, network-down auto-pause, or route that starts a chain.
- **Every scrub, sweep, restate and recount is an attended verb** (D-6900, as the 20 Oct follow-ups
  restate it).
- **Staleness is computed when a page is read,** and shown by name.
- **An alert kind is appended only when a kept producer puts it.** `KINDS` is append-only (`CLAUDE.md`
  §3 rule 8), so a kind nobody raises would stay for ever. D-6900 item 5 (j): network_down,
  feed_behind, gdfl_day_late and ingest_chain_failed are not appended (their only producers are
  dropped); vendor_format_changed and vendor_restated stay.
- **Deferred, not dropped (D-7094, 20 Oct, recorded 23 Oct):** SC-9 (retiring raw months beside sealed
  ones; it waits for the operator's D-4931, because unlinking a stored file touches `CLAUDE.md` §1 and
  §3 rule 8), SC-12, CHG-2 and CC-11 (with its failure case FM-618). No part here builds any of them,
  and no part unlinks or retires a stored file.

### 4.4 A patch, never a fresh rebuild (D-7135, D-7136, 23 Oct)

The operator, 23 Oct ~07:45 IST: "even if I provide a fresh new GDFL link, just consider it as
re-import or patching". The rule: a new GDFL link, or any vendor's re-delivery, is an incremental
re-import (a patch) of new or changed days into the EXISTING store, append-only. There is never a
fresh-from-scratch rebuild of the whole store.

**The patch path, in order:**

1. **CC-3's intake** sorts every day the store holds into same, new, changed or gone, by its bytes: the
   member sizes and CRCs against the source member its own provenance names. Being held never makes a
   day "same".
2. **PREC-5** rebuilds each changed day once, as a new generation, "through R3-22's restate door and
   R4-21's moves" (D-7135 item 1). D-7136: PREC-5 owns the append-only new tick-store version of a
   changed re-delivered day.
3. **PREC-19** re-derives order flow.

Every file is checked against the vendor, the re-delivered files included (D-7135 item 1).

**What happens to each day:**

- A changed day gets a new generation. The old generation stays, byte for byte.
- Anything bound to the old generation reads as stale until it is rebuilt.
- An unchanged day writes nothing, and so does a second press.
- A held day missing from the new download is "gone". It stays held, stops the command by name, and
  goes on only after a one-use, journalled acceptance (D-7135 item 1).

**Until PREC-5 merges, the head refuses a changed held day by name.** `pull::gdfl_dropbox` refuses it as
`redelivered`, and the tick-store writer refuses it with `Refusal::HeldDayDiffers`. That is the correct
behaviour today: no other part weakens it or works around it.

**Refused and never built** (D-7135 item 2):

- the full rebuild (R4-16's second store of about 2 TB beside production);
- the fresh-clone harness;
- `cli fresh-check`;
- the digest manifest;
- CC-3's `--against <old root>`;
- R4-11's chain run on an empty root;
- a "root A";
- a whole-store old-versus-new compare;
- a delete list.

**Proof 3 is a determinism test only.** One month of every kind is built twice, into two new, empty
`role=scratch` roots, and the two must show 0 bar differences (`CLAUDE.md` §3 rule 5, D-7135 item 3).
It never touches production.

**One-time passes over the kept store are O(bytes) by nature.**

- Each is listed in `docs/06-limits.md` as a one-time pass, with a measured time and a disk projection.
- None is ever counted as a per-operation cost.
- Every route and every store step stays O(1) at p99.

Data rigour is never traded for speed. The operator, 23 Oct ~09:05 IST, re-pasting the standing
principles: "whatever it is we need maximum precision, accuracy and a working solution".

### 4.5 R4-09b: the backup and its restore drill (D-7137, 23 Oct)

Under D-7135 a verified second copy is the store's only disaster path.

- **R4-09b is split out of R4-09** (item AUT-06) and placed right behind the critical path.
- **The two verbs** are `cli backup --to <dir> [--plan] [--mib-per-s N]` and
  `cli backup-drill --from <dir> [--months N]` (N defaults to 3).
- **What the pages show:** `/disk.json` and the /db band show the last backup, the manifest's file
  count and bytes, the last drill and its result.
- **R4-09b leaves alone** R4-09's scrub and /verify scope, `crates/pull/src/scrub.rs` (SC-9) and
  `crates/store/src/file.rs`.
- **D-7137 is already written** (by LEDGER-1023). R4-09b writes no second D-7137; its own block is
  D-7175 to D-7184.
- **The real backup is the operator's store write.** The operator still owes the second drive's path,
  so the first full copy and the first drill are NEEDS THE MAC.

### 4.6 Feeds by level, and how order flow is named

Sources: the operator's FEEDS BY LEVEL rule (16 Oct 08:45 IST) and naming rule (14 Oct); D-5170 and
D-5174 in `docs/05-decisions.md`; the measured corporate-action rows of `docs/00-charter.md`.

- **Minutes always come from Zerodha:** the 1-minute bars, the 2 to 60 minute rungs folded from them,
  and Zerodha's day bars. D-5170 item 2: decisions are taken on Zerodha 1-minute bars and the candles
  folded from them at the timeframes the search generates, with day bars as context from earlier days.
  `pull::fold` is the one fold authority.
- **GDFL is used only at the seconds level:** fills at the worst price of each second, and order flow.
- **Volume is GDFL's, never Zerodha's minutes** (D-5170 item 1, verbatim): "Zerodha minute volume is
  never used for a volume condition, a volume rank, a liquidity or capacity figure, or a fill's size.
  GDFL's `1s` volume is, summed as stored (into a Zerodha minute, the GDFL seconds inside it; a minute
  with no GDFL second has no volume value, never 0). Zerodha's `1day` volume is a cross-check of GDFL's
  day, never a substitute for it."
- **Zerodha's historical candles are corporate-action adjusted; GDFL's ticks are raw.**
  - Never compare or mix the two feeds without first putting them on one basis per stock-day.
  - Where the two meet (fills), decisions taken on Zerodha minutes are converted to GDFL raw prices by
    DATA-1's per-stock-day factor (the operator's rule of 16 Oct; DATA-1 merged 21 Oct).
  - That factor has one authority, `cli::feed_reconcile::FactorIndex` (D-5174), read and never
    recomputed. It is an exact paisa pair, never a float (`FactorPair::to_gdfl`); a day with no factor
    has no conversion and its refusal is named, never 1.0 (D-5170 item 3).
- **Run identity includes the feed** (`CLAUDE.md` §3 rule 3).
- **Always write "order flow delta" and "order flow cumulative delta",** never the bare word. The store's
  `delta` is the option greek. This applies to code identifiers, condition names, store fields, pages and
  documents alike.
- **Order flow is computed per GDFL second.** It is attached to each Zerodha minute by summing the
  seconds inside it (per-day prefix sums, O(1) per bar).
  - A minute with no GDFL data has no value, never 0.
  - The GDFL/Zerodha minute-volume coverage ratio is shown beside it.

### 4.7 Forced exit and tradable day

Not repeated here: none of this set's parts prices an exit or builds own-past statistics.

- CAL-1 (merged 22 Oct) holds the tradable-day rule, `pull::calendar::tradable_day`.
- A part that finds it needs either clause uses the merged authority and says so in its report. It
  never writes a second calendar.

### 4.8 Save as you go, never redo (21 Oct)

The operator, 21 Oct: "don't repeat the same work twice; always continue or resume after holding it".
A usage limit, a HOLD or a restart can end a session at any minute. Whatever is not committed and
written down is lost.

**Before you start:**

- If your branch already exists on origin, read it and its report `reports/cloud-db-<NAME>.md` first.
- Continue from the report's last NEXT line.
- Redo nothing the report records as done. A recorded gate is re-run only as part of your one final
  gate run.

**At every save point** (about every 20 minutes of editing, and always right before a long test, build
or gate run):

- Commit only your own files, as `WIP <NAME>: <what>`, and push your branch.
- Append a dated entry to the report, with:
  - the IST time;
  - what was done, and in which files;
  - each command with its exact result line;
  - the commit sha;
  - a NEXT line that says exactly what comes next.
- Never hold more than 20 minutes of edits uncommitted.

**Never rewrite what you pushed:** no squash, rebase or force-push. The Mac's merge-back folds the
commits.

**A HOLD is never a reason to abandon or restart work.**

### 4.9 Decision numbers: one registry

The Mac keeps one registry of decision-id blocks (`state/decision-blocks.json`, since 18 Oct).

- A part takes D-ids only from its own block, in order.
- The merge refuses a duplicate `D-` heading in `docs/05-decisions.md`.

**Why the registry exists:**

- R4-28's branch wrote D-7139 and D-7140 inside the no-fresh cut's block. It is renumbered to D-7185 to
  D-7194 before it merges.
- R4-26's repair wrote D-7139. It is renumbered to D-7195 to D-7204.
- R4-18 took D-7121 from BASE-REDS-1021's block, and was moved to D-7290 to D-7294.
- R3-13's entry moved to D-7300.

**Never write:**

- D-7135 to D-7138 (written by LEDGER-1023);
- D-7139 to D-7144 (reserved for the cut, PREC-5's writer);
- any number in another part's block.

The next free block starts at D-7205, and only the Mac assigns it.

**Blocks for this set, from the registry on 24 Oct:**

| Part | Block |
|---|---|
| R4-09b | D-7175..D-7184 (D-7137 already written: no second one) |
| IA-3b | D-7000..D-7009 (it also lands the design ids D-4802, D-4819, D-4850, D-4864 and the critic id D-4908, none written yet) |
| CC-5 | D-7040..D-7049 |
| AG-06b | D-7100..D-7109 |
| CC-20 | D-6800..D-6819 |
| CLOUD-CHG1-OPT | D-6740..D-6759 |
| R3-20 | D-7093 (its recovery rule, inside the follow-ups' block D-7090..D-7099) |
| R3-12 | D-7111, only if it locks a choice (block D-7110..D-7119) |
| IA-17b, IA-10a, R4-05 | No block in the registry. Use only the ids your part's file names (design ids from `ingest-automation.md` section 15); otherwise `D-TBD-<NAME>-n` as `README.md` says. IA-17b's D-4875 is already written. |

If your part's file and this table disagree, write `D-TBD-<NAME>-n`, report the difference, and the Mac
resolves it at merge-back.

### 4.10 Gates at the base

**`cargo deny` has been green since BASE-REDS-1021 merged on 22 Oct.** That part bumped the lock for
RUSTSEC-2026-0381 (webpki-roots) and RUSTSEC-2026-0382 (rustls-webpki). It must stay green after you
merge the newest `origin/feat/db-fix`.

If a new advisory turns it red on the base itself:

- show `cargo deny check` red on the unmodified `origin/feat/db-fix` with the same advisory ids;
- leave `Cargo.toml` and `Cargo.lock` alone unless your part owns them;
- list it under NEEDS THE MAC.

This is the 17 Oct cloud rule.

**Some CI shell gates under `.github/` are red at the base.**

- The Mac's deploy gate scripts (`gate-<sha>.sh`) run fmt, clippy, the tests, deny and the web steps
  only. The CI shell gates run apart (`state/fix/ci-shell-gates.sh`) and are reported apart until
  BASE-REDS-1023 merges.
- On 23 Oct the CI shell gates were found red on `feat/db-fix`:
  - 27b (D-6247 heads two entries);
  - 11 (since IA-16);
  - 1c (since IA-9's `zerodha_spool` lock path);
  - 1d, 10, 12, 19, 23 and 30 (older).
- BASE-REDS-1023 makes all of them green. It is Mac-owned, holds block D-7145..D-7154, and is not
  merged yet.

Run the CI shell gates that touch your files:

- A gate that is equally red on the unmodified base is a base red owned by BASE-REDS-1023. Report it
  as such. Never fix it in another part's files, and never hide it.
- A red your change causes is yours to fix.

**Some timing tests fail only under load.** On the loaded Mac three of them failed and then passed on
rerun:

- api `pool_load::the_feed_list_stays_fast_while_32_clients_loop_each_route`;
- one store sealed-open timing test;
- a web calendar-loader test.

A timing test counts as flaky only after three clean reruns, each one recorded. Until then it is red.

### 4.11 Public repository

This branch and yours are public. Never write any of these into a file or a commit:

- a Dropbox or other share link;
- a token, key, password, account id or e-mail address;
- an AWS Parameter Store path with real segments (only the shape `/<org>/<env>/<vendor>/<field>`);
- anything from the operator's local untracked configuration;
- a real vendor row, such as a GDFL tick row or a Zerodha candle copied from a file.

Fixtures are invented, with prices that cannot be mistaken for real ones. Describe evidence in words and
numbers: a count, a date, a symbol and what was wrong.

### 4.12 This set: one owner per part

Since 24 Oct these 11 parts are cloud-owned. The Mac does not build them while a cloud session owns
them. Each gets branch `cloud/db-<NAME>-20261024` and report `reports/cloud-db-<NAME>.md`.

| Part | Title (short) |
|---|---|
| R4-09b | the store's backup verb and its restore drill |
| IA-17b | D-4875 remainder: the once-per-month gather and the CASH `.auc` writer |
| IA-3b | ingest preflight refusals IA-3 left |
| CC-5 | copies register: every extra copy listed with its bytes, reader and proof |
| R3-20 | month recovery after a power cut |
| R3-12 | /db reads its lists from the server |
| AG-06b | pull runs never start unjournalled; no tokio worker waits on the journal lock |
| CC-20 | benches for every check primitive, with limits rows |
| IA-10a | API, import half: the read side only (D-6900) |
| R4-05 | one reasons dictionary with its gate and route, and the kept alert kinds |
| CLOUD-CHG1-OPT | charges: the options remainder of CHG-1 |

**A merge-order link is not a prerequisite.** Two parts here have one (FILES_ONLY): AG-06b on IA-10c,
and R4-05 on R3-21. Neither IA-10c nor R3-21 is merged at `e851da1ca`.

- Such a link does not block your start.
- It means the other part edits the same lines, so it merges first.
- Build from the newest `origin/feat/db-fix`. The Mac waits for the other part at merge-back.
- Only a true prerequisite, merged or not, decides `STATUS: BLOCKED` (`README.md`, "Before you build").

### 4.13 Rules that bind every part, from the launcher, landed decisions and the critic

**The Mac builder's own procedure is in your part's file.** Each `<NAME>.md` transcribes `fixPrompt(p)`
and `specStep(p)` for that part (NIFTY FIRST, RULES AUDIT, the self-check of step 5b, the extreme-case
counts, `store_writes_needed`), adapted to the cloud. Those texts bind as your part's file gives them.

**The head is the fact** (launcher `specStep(p)`, the `ia` branch, verbatim): "Where the design states a
fact about merged code that is false at the head of feat/db-fix, the code at the head is the fact:
build to the design's intent and say so in notes." The same holds for any spec here: a part's "code
facts" section was grepped at `e851da1ca`, and the newest `origin/feat/db-fix` wins over it. Say so in
your report.

**Every state change is an audited event.**

- D-4893 (verbatim): "A state change is an event at a target ending in `lifecycle`, and the package that
  adds it asserts `telemetry::is_audited(level, target, message)` in a table test."
- D-5890 (R3-24, merged 22 Oct, verbatim): "`telemetry::run_dirs::state_events::TABLE` holds one row per
  `(target, message)` a writer emits as a state change, with its levels, its class (operator, server,
  maintenance, lag, run) and the file its emit sits in; `is_audited` takes every row at `info` or above
  whatever its words."
- At e851da1ca, `telemetry::is_audited` (`crates/telemetry/src/sink.rs`) takes every `error`, and at
  `info` or above a target ending `lifecycle` or the access segment, a lifecycle word in the message, or
  a row of that table. A `warn` alone is not audited. Its tests `every_row_names_its_emit_site` and
  `every_state_change_event_is_audited` (`crates/telemetry/src/state_events_tests.rs`) fail the build
  when a row and its emit site drift apart.

**Shared exact lines and shared ledgers** (D-4891 decision, verbatim): "(1) A crate's `cover` line is a
shared exact line. As its last step before its gates a package merges the head, recounts its bench by
the gate's own rule, and writes the head's ids plus its own with that count. A merger never takes
"ours" or "theirs" on that line: it takes the union of the ids and recounts. (2)
`docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md` and `.github/gates_ledger.rs` are
append-only shared files: two packages' appended sections conflict textually and resolve as "both, in
merge order". Entries are never renumbered to restore numeric order". The `cover` line is gate 14's, in
`.github/workflows/ci.yml` ("now an EXACT count: adding a measurement fails until its pin is raised").

**One heading per decision number.** Gate 27b (`.github/gates_ledger.rs`): "every decision number heads
exactly one entry, the pins aside". Where a design id lands in parts, the critic's resolution X-4
(`state/design/critic-round7-20261011.md`, verbatim) is the rule: "the first package to land a number
heads the whole §15 text, re-grounded, with an "Implemented by" line naming each package and part; a
later package never adds a second heading and never edits that entry: its proof is its own `docs/04`
rows." So IA-17b adds no second D-4875 and R4-09b no second D-7137.

**A verb that writes the store is listed.** `cli::store_write::WRITERS` (19 verbs at e851da1ca) holds
"Every verb that can write bars, sidecars, a census, a journal or a ledger of the store"
(`crates/cli/src/store_write.rs`, its doc comment). An unstamped build is refused before such a verb
reads any root (DB-B71, D-4894). The critic's X-6: "Every new verb that writes under the store root
joins `WRITERS` in the change that adds it".

## 5. Where this came from (on the operator's Mac, read-only)

- **The Mac build launcher:** `state/fix/db-fix-round-2-run0738.js.fat`. Its `specStep` and
  `fixPrompt` are what a Mac builder is told, and its `FILES_ONLY` handling is in `run()`.
- **The launcher's tables as JSON:** `state/cloud-20261024/embed.json` (`EMBED`) and
  `state/cloud-20261024/args.json` (`__ARGS`: `specAppend`, `filesOnly`, `extraPackages` and the cut
  notes).
- **The operator's rules:** `state/resume-kit/next-prefix.md`, the blocks from "20 OCT" to "24 OCT".
- **The decisions:** `docs/05-decisions.md` at `e851da1ca`, entries D-6900, D-7094 and D-7135 to D-7138;
  for 4.6, D-5170 and D-5174; for 4.13, D-4891, D-4893, D-4894 and D-5890.
- **The design critic:** `state/design/critic-round7-20261011.md`, "Cross-package gaps (they bind every
  package)", X-4 and X-6.
- **The options replan:** `state/options-20261020/opt-packages.json` (`_note`: which parts got an `-OPT`
  half).
- **The number registry:** `state/decision-blocks.json` (`free_from` D-7205).
- **The cut folders:** `state/options-20261020/`, `state/oneclick-cut-20261020/` and
  `state/nofresh-20261023/`.

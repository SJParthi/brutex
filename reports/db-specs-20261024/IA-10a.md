# IA-10a: API, import half, cut by D-6900 to its read side (member keys and holes in every api journal reader, the day band's O(1) run lookup D-4896, the census names and calendar horizon on the cli receipt)

Self-contained specification for the cloud session that builds part **IA-10a**. Everything the
Mac builder of this part would read is copied below, verbatim where it is a specification, each
block labelled with its source file and line numbers on the operator's Mac. The Mac paths
(`/Volumes/WD_BLACK/...`, `wt/`, `tgt/`, `state/`) do not exist in the cloud: read
`README.md` and `CONTEXT.md` in this folder for what differs. This repository is brutex only.

**Read this first.** The package's title and its design row describe a large unattended "import
half" (an api child mode, `/gdfl/*` routes, alerts routes, a restart scan). **On 20 Oct 2026 the
operator cut all of that** (decision D-6900, the one-command cut): ingest is always driven by
Claude Code or the operator from a terminal through `cli gdfl-import`. The part that remains is
the **read side**: the text in section 3.2 (`__ARGS.specAppend["IA-10a"]`) is the binding scope,
and it says, item by item, what to BUILD and what to DROP. The design and critic text in sections
5 and 6 is copied because the Mac builder is told to read it; wherever it describes a dropped
piece, it is history, not work. Section 4 gives the kept/dropped split in one table.

## 1. Header

| Field | Value |
|---|---|
| Name | `IA-10a` |
| Title (EMBED, unchanged since round 3) | API, import half: /gdfl/* routes, the child mode, the alerts routes |
| Scope in force | the read-side half only, per `__ARGS.specAppend["IA-10a"]` (section 3.2): items (0) to (7), four tests, the D-4896 landing and the D-6900 amendments |
| Kind | `ia` (an INGEST AUTOMATION package of EMBED; `specStep(p)` takes its **`ia` branch**: the design is the specification, plus the fix-plan entry if any and the findings) |
| EMBED flags | `waitCritic: true` (the launcher runs with `noCritic: true`, so no new critic step runs; the round-7 critic file is still read, section 6) |
| Priority rank | **67 of 137** in `__ARGS.priority` (0-based index 66): after `CC-20`, before `SC-6` |
| Critical path | **No** (`__ARGS.criticalPath` does not name it) |
| Skip / done / parked | not in `skipFix`, not in `doneExtra`, not parked; `only` names it (index 7 of the `only` array; index 5 of the launcher's `only`-filtered package list, which gives the port) |
| Decision numbers | **D-4896** (design/critic number this part lands); no registry block; anything else `D-TBD-IA-10a-n` (section 12) |
| Base | `feat/db-fix` head **e851da1ca** (`e851da1ca743ae308c8437d5055e0cbaa1d88d83`, 23 Oct 2026 23:12 IST) |
| Mac port / pool (for the NEEDS THE MAC commands only) | shadow server port **8405** (8400 + index 5 of the launcher's `only`-filtered package list; `portBase` unset), shared build pool `tgt/pool-5` |
| Earlier attempts | none: no `wt/P-IA-10a` worktree, no `fix/db-IA-10a` branch, no `state/fix/round2/fix-IA-10a*.json` or progress file on the Mac |
| Merged? | **NOT MERGED** (no `Merge IA-10a:` on `feat/db-fix`) |

### PREREQUISITES

Computed as `(package deps + __ARGS.extraDeps["IA-10a"]) - __ARGS.dropDeps["IA-10a"]` (launcher
line 211; `parked` is empty, so it removes nothing):

- package `deps` (EMBED): `IA-2`, `IA-16`, `IA-3`, `IA-11a`, `IA-11r`, `WP-39`, `WP-81`
- `__ARGS.extraDeps["IA-10a"]`: none
- `__ARGS.dropDeps["IA-10a"]`: `IA-11a`, `WP-39`, `WP-81` (cut with the dropped half; specAppend's EDGE NOTE)

| Prerequisite | State on `feat/db-fix` at e851da1ca |
|---|---|
| `IA-2` | **merged** (`a8931ea22 Merge IA-2: locks, the runs ledger, claims, the two-phase start and the whole stop`) |
| `IA-16` | **merged** (`c099cf003 Merge IA-16: failure kinds and causes: the admit table, decisive order, the typed repair door`; also in `doneExtra`) |
| `IA-3` | **merged** (`c76f9178c Merge IA-3: preflight, the source cache, zip faults, archive and file-name cases`) |
| `IA-11r` | **merged** (`078ee1fd6 Merge IA-11r: audit roll reaper, widened import-evidence readers and the O(1) run index (R3, R5, R6)`; also in `doneExtra`) |

All four prerequisites are merged: the part may start.

Dropped dependencies (no longer prerequisites; all three are merged anyway): `IA-11a`
(`bdb3f02b4`), `WP-39` (`8004664f9`), `WP-81` (`f0f57f661`).

FILES_ONLY links (`__ARGS.filesOnly["IA-10a"]`): **none**. Nothing to "merge first, do not block the start".

Related parts named by the spec or the design, for orientation (not prerequisites):

| Part | Relation | Merged at e851da1ca? |
|---|---|---|
| IA-1 | `pull::gdfl_run`, the one import authority; `cli gdfl-import` is its verb | merged (`304a2320c`) |
| AF-01 | the cli door `crates/cli/src/gdfl_import_guard.rs` (kind lock and claim, `run_busy`) | merged (`3911d1b9c`) |
| AF-02 | `Owners::names` in `imports.rs` (replaced the design's `merged`), no phantom `*` rows | merged (`381a9cca3`) |
| WP-33c | `/logs.json`, `/imports.json`, audit and log search | merged (`f350c156d`) |
| WP-36, WP-32 | calendar cold start; option analytics (cited in the design row) | merged (`ab2810b83`, `f31267242`) |
| R3-05, R3-07, R3-08, R3-09, R3-10, R3-14 | added `server.rs` registration and admission lines "before you" (round-3 spec) | merged |
| R3-24 | marks contradicting "day imported" lines on `/logs.json` (DB-A48's log half) | merged (`48adf5a39`) |
| R3-23 | takes DB-A50, FM-069's display half and FM-084 from this part | NOT MERGED |
| R3-31 | "adds the alerts held in memory to your alerts route after you"; that route is dropped | NOT MERGED |
| R4-10 | `/logs.json` limits, the import-journal measurement D-6901 | NOT MERGED |
| R4-11 | `cli ingest-run`, the one command; takes FM-124; gets the receipt lines "by calling the verb" | NOT MERGED |
| IA-12a | the `/ingest` panel; DROPPED by D-6900 with this part's routes | NOT MERGED (never built) |
| IA-10b | the api's verification half; still lists IA-10a as a dependency | NOT MERGED |
| LEDGER-1023 | recorded D-6900 in `docs/05-decisions.md` (23 Oct) | merged (`e0668a4bd`) |

Parts that wait on IA-10a in this launcher (EMBED `deps` not removed by `dropDeps`): `IA-10b`,
`R3-23`, `R4-03` (each in `only`); `IA-7` (not in `only`, dropped by D-6900). Parts whose edge to
IA-10a is cut by `dropDeps`: `IA-13`, `R3-21`, `R3-25`, `R3-26`, `R3-31`, `R4-02`, `R4-11`,
`R4-24`, `R4-29`. `IA-12a` and `R4-14` are dropped.

## 2. What the Mac builder is told (launcher text, verbatim)

The Mac builder's prompt is `fixPrompt(p)` (launcher lines 351-373), which embeds
`specStep(p)` (lines 341-349). For IA-10a (`kind: 'ia'`) `specStep` returns its **first
branch**. `${C}` is EMBED's shared context (this folder's `CONTEXT.md`), `${p.spec}` is section
3.1 plus 3.2 below (line 211 appends `specAppend` to the spec), `${p.row || p.name}` is `IA-10a`
(EMBED gives no `row`), `${DESIGN}` is the design (section 5), `${CRITIC}` the round-7 critic
(section 6), `${W}` is `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`, `${OUT}` is
`${W}/state/fix/round2`, `${p.port}` is 8405, `${POOL(p.name)}` is 5, `${p.deps}` is
`IA-2, IA-16, IA-3, IA-11r`. Steps 2, 5 and the pool rules are Mac mechanics; in the cloud
follow `README.md` instead, and list every step-5 check under NEEDS THE MAC (section 13).
`args.niftyFirst` is unset, so the NIFTY FIRST paragraph is part of the prompt.

Source: `state/fix/db-fix-round-2-run0738.js.fat`, lines 341,349

````text
function specStep(p) {
  if (p.kind === 'ia') {
    return `1. Read your package. It is an INGEST AUTOMATION package; its specification is the design ${DESIGN} (6,154 lines; read by line range, never all at once). Start with your row of the table in section 16.2: grep -n '^| ${p.row || p.name} ' ${DESIGN} (rows are single long lines; print them whole with sed -n '<line>p'), the round notes above that table that name ${p.name}, "Lanes" and "The order" below it, and your row in 16.3 (CI). Then read EVERY section, decision (section 15, column "Lands with"), file entry (section 11), route (12), web change (13), test id (14), limit row (10.2), matrix row (8) and edge case (9) your row cites. If ${CRITIC} exists, read its part for ${p.name}: its resolutions are your defaults where the design is silent or contradicts itself. Where the design states a fact about merged code that is false at the head of feat/db-fix, the code at the head is the fact: build to the design's intent and say so in notes. ${p.spec || ''}
   Also read ${W}/state/board/fix-plan.json's entry if one exists: jq '[.work_packages[] | select((.name | split(" ")[0]) == "${p.name}")]' ${W}/state/board/fix-plan.json
   The findings your package answers (ids starting DB-A with package IA, and the gaps F1..F18 your row names) are in ${W}/state/board/audit-result.json (key "confirmed"). Report one findings[] entry per test id, decision and gap your row names (id = the test id, decision number or gap id).`
  }
  return `1. Read your package: jq '[.work_packages[] | select((.name | split(" ")[0]) == "${p.name}")][0]' ${W}/state/board/fix-plan.json
   Read the full text of every finding it lists (observed, expected, evidence, location, fix_hint) from ${W}/state/board/audit-result.json (key "confirmed", match on "id"). Read the plan's summary and all_sources_view in fix-plan.json if your package touches them. D-number assignments are in WP-02's decisions_needed. ${p.spec || ''}`
}
````

Source: `state/fix/db-fix-round-2-run0738.js.fat`, lines 351,373

````text
function fixPrompt(p) {
  return `${drainFirst('fix', p.name)}${C}

YOU OWN WORK PACKAGE ${p.name}: "${p.title}".
${(args && args.niftyFirst === false) ? '' : `\nNIFTY FIRST (operator): prove your work first on NIFTY spot and NIFTY options before anything else, on every NIFTY selection your package touches (feed GDFL 1s and Zerodha 1min..1day; segment Spot and expired options; every expiry form 2018-09..2020-09, strikes, CE/PE; dates, time filters, sorts, Go to, paging, export, coverage, moneyness/IV/greeks cells; for import packages: one NIFTY spot day, one NIFTY options day, one special session). Fix in the SHARED, instrument-agnostic code path, never as a NIFTY special case, then repeat the check on one BANKNIFTY selection, one F&O stock and one illiquid strike. The NIFTY evidence so far is in ${W}/state/gdfl-board/out/nifty.json.\n`}
IF ${W}/wt/P-${p.name} ALREADY EXISTS (an earlier attempt of this same package was cut off by a usage limit; it may hold commits ahead of feat/db-fix, "WIP ... to be squashed" commits and UNCOMMITTED changes), reuse it: read git log feat/db-fix..HEAD and git diff first, judge every hunk critically against your package, keep what is right, fix what is wrong, finish what is missing; never discard it blindly, never git checkout/reset away its changes. Changed files under web/build/ are generated output of an earlier npm run build: reset them (git checkout -- web/build && git clean -fdq web/build) before you judge the diff; never commit web/build unless your package's row owns it.
IF ${OUT}/fix-${p.name}.json or ${OUT}/verify-${p.name}-r<n>.json EXIST, an earlier run today already built this package (and a second agent may have checked it): read the newest of each first. Every BLOCKING issue in the newest verify report, and every follow-up in it that names your files, is part of your package now; fix those first, each with a test that fails before, then finish whatever the builder's notes list as NOT DONE. If the branch is committed and nothing is open, re-run the gates once, write your report and return quickly.
SAVE AS YOU GO, RESUME, NEVER REDO (operator 21 Oct 2026: "don't repeat the same work twice; always continue or resume after holding it"). A usage limit, an app restart or a hold can end you at any minute, and whatever is only in your head is then lost. (a) FIRST, if ${OUT}/fix-${p.name}-progress.md exists, read it whole: an earlier attempt of this package wrote it. Continue from its last NEXT line. What it records as done stands for every commit still on the branch (git log feat/db-fix..HEAD): redo only what it does not record, and re-run a recorded gate only as part of your one final gate run. Shadow-server checks it records from before your last code commit do not stand: take them again on the final build before your last commit (a running server tests every file at once). If git log --first-parent --format=%s feat/db-fix shows "Merge ${p.name}:", that earlier attempt was already merged: its entries are history and its NEXT line does not bind you; append an entry "attempt after merge" and do the task below. (b) Keep that file as you go: append (never rewrite) an entry after every finished piece of work, headed by the output of TZ=Asia/Kolkata date '+%F %T IST': what you did and in which files, each gate or test command with its exact result line, each shadow-server check (URL + value), the WIP commit sha, and a NEXT line that says exactly what comes next. (c) Commit on fix/db-${p.name} at least every 20 minutes of editing and always right before any long test, build or gate run: git add <only your files>; git commit -m "WIP ${p.name}: <what> (to be squashed)" -m "${CO}". Never hold more than 20 minutes of edits uncommitted. Do NOT squash these commits yourself, whatever the context says about squashing before the final report (21 Oct): the merge folds every WIP commit into one squash commit with the same tree. Your last commit (step 6) is a normal one. (d) A BRANCH ALREADY MERGED BY A FOLD (review fix 21 Oct): before ANY git merge feat/db-fix into this branch (steps 2 and 4), run B=$(git log -1 --format=%H --grep='^${p.name}: squash of fix/db-${p.name} at ' feat/db-fix). If B is set and git merge-base --is-ancestor $B HEAD fails, take T0=$(git log -1 --format=%s $B | grep -oE '[0-9a-f]{40}' | head -1); if git merge-base --is-ancestor $T0 HEAD succeeds, first run git merge -s ours -m "${p.name}: record fold $B of $T0 (same content, no file changed)" $B. It changes no file (that fold holds exactly the tree of $T0, which your branch already has) and makes the later commits of feat/db-fix merge onto your own lines without conflicts. The commits that fold already merged still show in git log feat/db-fix..HEAD: they are merged, not open work.
RULES AUDIT (11 Oct): ${W}/state/design/rules-audit-20261011.md lists, per owner, the gaps ten agents found against the operator's rules (O(1) at p99, parallel, saved and searchable, common and scalable, extreme cases). Find the section for ${p.name} (grep -n '^## .*${p.name}' that file): every gap listed there is part of your package, with the test or bench it names. If a gap there cannot be closed inside your files, say so in notes with the owner it needs.
${specStep(p)}
2. Set up: git -C ${W}/wt/DBFIX worktree add ${W}/wt/P-${p.name} -b fix/db-${p.name} feat/db-fix   (if the worktree or the branch already exists, reuse it, then git merge feat/db-fix into it, resolving conflicts by keeping BOTH sides' intent). Then: ln -s ${W}/wt/APP/web/node_modules ${W}/wt/P-${p.name}/web/node_modules (skip if present); export CARGO_TARGET_DIR=${W}/tgt/pool-${POOL(p.name)}. [SHARED POOL, OPS-09 18 Oct: hold its lock around every cargo command: mkdir ${W}/tgt/pool-${POOL(p.name)}.lock before, rmdir it after; while it exists wait 30 s and retry; a lock older than 90 minutes is stale: rmdir it. OPS-10 20 Oct (AG-05, R3-24, PIT-1 and PREC-1 checks): cargo names the artifacts of a workspace crate by its path inside the workspace and trusts mtimes, so in a shared pool a newer build from another worktree reads as fresh and runs as yours. Take the lock ONCE for a whole sequence of cargo commands (gates, release build) and rmdir it after the last of them, not after each. Touch the lock directory right before each cargo command of the sequence, and never from a background loop or a timer: a loop that outlives you keeps the lock of a dead holder fresh, and the 90-minute rule never frees it. Right after taking the lock, and again after any git merge, checkout or reset inside the hold, run (cd <the worktree you build> && git ls-files -z | xargs -0 touch) before the first cargo command. A gate result taken without that touch is not counted. A release api or cli you serve or measure is copied under the lock (mkdir -p, then cp) to ${W}/tgt/bin-${p.name}/release/ (the second agent: ${W}/tgt/bin-${p.name}-v/release/) and served from there, never from the pool] Work ONLY in that worktree.
   Dependencies: ${(p.deps || []).join(', ') || 'none'}. All are merged into feat/db-fix, so your branch contains them once you merge feat/db-fix.
3. Do the package completely and precisely, the way its specification says, with tests that fail before and pass after. Edit only your package's owned files (new test files for your own modules are fine). If you truly must touch another file, keep it minimal and list it in files_outside_ownership. /db packages (WP-*) do not edit docs/05-decisions.md, docs/04-invariants.md, docs/06-limits.md, docs/11-findings.md or docs/07-plan.md unless they are WP-99: cite the D-numbers WP-02 assigned in code comments instead. Ingest packages (IA-*) land their own rows in those ledgers in the same change (D-4861): re-grep the highest D-48xx number first and never reuse one.
4. Gates in your worktree (all must pass before you commit; merge feat/db-fix into your branch once more just before them): cargo fmt --all --check; cargo clippy -p <each touched crate> --all-targets -- -D warnings; cargo test -p <each touched crate> --locked; if web/ changed: (cd web && npm run build && npm test), then reset the generated web/build files unless you own them. Anything you add to a hot path must stay O(1) per operation; measure p50, p99 and max for every route or operation you touch (n >= 200 sequential GETs against your shadow server, or the crate's bench) and report them with n and the method. Anything that cannot be O(1) is refused or bounded, never silently slow, and is named for docs/06-limits.md in your notes.
5. Check against REAL data, never on :8080: build in release (cargo build --release -p api -p cli), then
   ${W}/state/fix/stop.sh ${p.name}; ${W}/state/fix/shadow.sh ${p.name} && ${W}/state/fix/serve.sh ${p.name} ${p.port} ${W}/tgt/bin-${p.name} [SHARED POOL, OPS-09 18 Oct: hold its lock around every cargo command: mkdir ${W}/tgt/pool-${POOL(p.name)}.lock before, rmdir it after; while it exists wait 30 s and retry; a lock older than 90 minutes is stale: rmdir it. OPS-10 20 Oct (AG-05, R3-24, PIT-1 and PREC-1 checks): cargo names the artifacts of a workspace crate by its path inside the workspace and trusts mtimes, so in a shared pool a newer build from another worktree reads as fresh and runs as yours. Take the lock ONCE for a whole sequence of cargo commands (gates, release build) and rmdir it after the last of them, not after each. Touch the lock directory right before each cargo command of the sequence, and never from a background loop or a timer: a loop that outlives you keeps the lock of a dead holder fresh, and the 90-minute rule never frees it. Right after taking the lock, and again after any git merge, checkout or reset inside the hold, run (cd <the worktree you build> && git ls-files -z | xargs -0 touch) before the first cargo command. A gate result taken without that touch is not counted. A release api or cli you serve or measure is copied under the lock (mkdir -p, then cp) to ${W}/tgt/bin-${p.name}/release/ (the second agent: ${W}/tgt/bin-${p.name}-v/release/) and served from there, never from the pool] ${W}/wt/P-${p.name}
   This serves your build at http://127.0.0.1:${p.port} over a private shadow of the real store (bar files read through a link, everything else cloned). Confirm YOUR server is the listener: lsof -nP -iTCP:${p.port} -sTCP:LISTEN -t must print the pid in ${W}/shadow/${p.name}/test-server.pid. GET requests only against a shadow whose bars are the real store's link: never POST there, never run an import, pull, repair or backfill against it. For code that WRITES (importer, repair, sidecar or index writer, migration, verdict or alert store): exercise the write path only on a store root you own entirely - a temp dir built from synthetic fixtures, or a shadow in which you REPLACED the bars link by a real directory holding copies of the few months you need (cp -c) - and prove it never touched /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store. Reproduce each finding's selection and confirm the right answer (record URL + value as evidence a second agent can repeat). If the page is web-only, curl the API and run the web unit tests; you may open http://127.0.0.1:${p.port}/db in the browser pane only if no other agent is using it. Stop your server with ${W}/state/fix/stop.sh ${p.name} when done.
5b. SELF-CHECK BEFORE YOU COMMIT (19 Oct speed rule: a failed second check costs a whole repair round plus a re-check): do the second agent's checks yourself first. (i) Read your own full diff against feat/db-fix and judge every hunk against repo law (O(1) per operation: name every loop over months, files, records or instruments that a request or a per-row step can reach; append-only store; no look-ahead; no fallback that hides a failure; prices in paisa i64; Rust only outside web/; no test that asserts nothing; no real vendor row in a fixture) and the operator's rules (one common runtime-dynamic code path, incremental and scalable, every state change saved, audited, logged and searchable; every failure, refusal and extreme value tracked so it can be found, debugged, monitored and seen where the operator looks (/logs, /db or the part's own page) through a read that stays O(1) at p99). (ii) Add tests for at least 8 extreme cases that fit your change, at least 2 of them combinations of extremes at once and at least 1 out-of-the-box case no list names (empty, one-record and maximum inputs; month, year and expiry boundaries; special sessions; renamed symbols; duplicate, out-of-order and late rows; torn or zero-length files; a missing month mid-range; garbage parameters; concurrent requests; a kill mid-write; two writers). (iii) Re-run every finding you call fixed on your shadow server and record URL + value. Fix whatever this finds before you commit, and list in notes what you checked.
6. Commit on fix/db-${p.name} (git add the files you changed; never git add -A at the root, never commit vendor data, binaries, shadow or target dirs). Message: what was fixed and why, finding ids, D-numbers, ending with the line "${CO}". Never push. Never touch :8080, wt/APP, tgt/app*, the real store, or another package's worktree.
7. If something needs a store write on the real store (re-import, recount, migration, re-pull, backfill), implement and test the code path on fixtures and list the exact command under store_writes_needed; do not run it on the real store.
8. Write your structured report also as JSON to ${OUT}/fix-${p.name}.json (mkdir -p ${OUT}).
Return the structured report. Be honest: a finding is "fixed" only if a test or a shadow-server check proves it; say NOT PROVED where you could not check.`
}
````

The `${drainFirst('fix', p.name)}` prefix is the Mac's drain check (hold when the usage
governor says so); the cloud equivalent is the `GO`/`HOLD` switch in `README.md`. In `CO`, `<the no-reply address>` stands for the attribution address; `README.md` gives the exact commit trailer to use.

Launcher constants used above, and the line that builds each package's `spec` and `deps`:

Source: `state/fix/db-fix-round-2-run0738.js.fat`, lines 26,31 207,211

````text
const W = '/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925'
const POOL = n => { let h = 0; for (const c of n) h = (h * 31 + c.charCodeAt(0)) >>> 0; return h % 8 }   // 15 Oct: 8 shared build folders (tgt/ grew to 749 GiB with one per part)
const OUT = `${W}/state/fix/round2`
const DESIGN = `${W}/state/design/ingest-automation.md`
const CRITIC = `${W}/state/design/critic-round7-20261011.md`
const CO = 'Co-Authored-By: Claude Opus 5.5 (1M context) <the no-reply address>'
// 17 Oct 10:30: args.extraDeps adds dependencies (e.g. CLOUD-ATTACKO1 before the parts that touch its files); args.filesOnly lists FILES_ONLY links that may not block a START (they still merge first); args.criticalPath parts run outside the slot pool so the path never waits behind off-path checks and repairs
const EXTRA_DEPS = (args && args.extraDeps) || {}
const SOFT = (args && args.filesOnly) || {}
const PATH = new Set((args && args.criticalPath) || [])
const byName = Object.fromEntries(PK.map((p, i) => [p.name, { ...p, spec: (p.spec || "") + (((args && args.specAppend) || {})[p.name] ? "\n\n" + args.specAppend[p.name] : ""), deps: [...(p.deps || []), ...(EXTRA_DEPS[p.name] || [])].filter(d => !((args && args.parked) || []).includes(d) && !(((args && args.dropDeps) || {})[p.name] || []).includes(d)), port: ((args && args.portBase) || 8400) + i }]))
````

## 3. The package's own text

### 3.1 Title and spec (EMBED, verbatim)

Source: `state/cloud-20261024/embed.json` (= launcher line 10, `EMBED.packages[56]`; the two
copies were compared and are byte-identical).

````text
name:  IA-10a
title: API, import half: /gdfl/* routes, the child mode, the alerts routes
kind:  ia
deps:  IA-2, IA-16, IA-3, IA-11a, IA-11r, WP-39, WP-81
waitCritic: true

spec:
ROUND 3 ADDITION (12 Oct): Leftovers for your api half: show an interrupted run and each closing line's time on /gdfl/import.json and /imports.json (DB-A50); return the run's own log lines by run id (DB-A41); mark a logged "day imported" that the journal closed incomplete as contradicted on /imports.json (DB-A48; R3-24 does the same on /logs.json); keep the day band's run lookup O(1) per D-4896 (DB-A34). R3-05, R3-08, R3-09, R3-10 and R3-14 add registration lines to server.rs before you, and R3-08 and R3-07 change the admission lines of its request middleware: rebase on them. R3-31 adds the alerts held in memory to your alerts route after you. ANSWERS: DB-A41; DB-A34
````

### 3.2 specAppend (the binding scope since 20 Oct, verbatim)

Source: `state/cloud-20261024/args.json` `.specAppend["IA-10a"]` (= launcher line 6, `__ARGS`;
the two copies were compared and are byte-identical). This is the one-command cut's split
verdict for this part (D-6900); no later amendment (options, no-fresh D-7135, ledger
follow-ups, R4-09b, priorities) appended anything else to IA-10a.

````text
ONE-COMMAND SCOPE (operator 20 Oct 2026: ingest is always driven by Claude Code; the unattended layer is dropped). Verdict: split. KEEP: ONE-CLICK CUT (operator 20 Oct 2026 ~12:50 IST: ingest is always driven by Claude Code or the operator from a terminal through cli gdfl-import, IA-1's one import authority). This part is cut to its read-side half, F84 (design :615, :4294; SF-9). That half covers EVERY api reader of the GDFL journal's filter, not two files, because /db reads four of them. It also covers the two read items its own round-3 spec names that are not unattended. BUILD ONLY:
(0) One line in crates/pull/src/gdfl_cm.rs: make member_share (:1309, pub(crate) today) pub. Every api reader then maps a member key to its share by pull's one rule (shares_of, gdfl_import_journal.rs:549-560), never by a copy.
(1) crates/api/src/import_journal.rs parses a `<filter>+<member>` key into its base filter and its member. No api file parses '+' today, while merged pull code writes member keys (gdfl_import_journal.rs:551, :1153). An `incomplete` row whose reason is one of IA-16's five store-hole reasons carries `hole: {reason, repairable}` and is never shown as a plain incomplete.
(2) crates/api/src/imports.rs: /imports.json counts holes by reason, and `holes_unrepairable` apart from incomplete. AF-02's Owners::names (imports.rs:1072-1085, which replaced the design's `merged` at :792) reads a member key as naming its one member's share and as a close of its base filter's kind-day. So a day whose only keys are member keys counts as journalled and gets no made-up log-only `*` row (logged_only, imports.rs:793).
(3) crates/api/src/dayflags.rs, the filter match (journal_rows :981; Names::names :1044-1050 splits on ',' only): a member-key row covers its member's share and nothing else. A member's hole then shows on /dayflags.json and on /db's day band (DbDayFlags.svelte:164), and the base close's members keep the base close.
(4) crates/api/src/feeds_route.rs:1124, the open-day scan (`row.filter == "*"`): it keeps its answer for base keys and states what it does with an open member key.
(5) crates/api/src/provenance.rs filter_holds (:1022, called at :964 and :1046). This is the GDFL reconstruction behind /provenance.json, which /db reads (web/src/lib/db/notes-labels.js:937). A member-key row is its member's share's day row, so a member close that filed bars is never missed and its bars are never credited to the base close's run. A member hole is shown as a hole.
(6) D-4896, which this part's own spec keeps ('keep the day band's run lookup O(1) per D-4896 (DB-A34)'; critic round 7 :560). dayflags.rs refusals_of (:1181) resolves a refused event's run from the event's own `run` field (IA-16 gives every refused event that field) through IA-11r's merged run-id index, O(1) per event. Placement by time (imports::placed) stays only for an event without the field, and the answer says so. Rewrite L-IA-39 (docs/06-limits.md:16950) and land D-4896 (the forward reference at docs/05-decisions.md:70649).
(7) The cli receipt, the watcher's surface now that /gdfl/sources.json and /gdfl/plan.json are dropped. (a) D-4817's names: no production code reads pull::gdfl_archive::ArchiveCensus (:471) or gdfl_dayzips::DayZipsCensus (:179), and docs/05-decisions.md:74754 says 'Showing these names on /ingest is IA-10a's and IA-12a's'. So cli gdfl-import's receipt prints each source's census: the ignored, misfiled and invalid counts and the first 16 names of each (at most 48 names, O(1)). (b) FM-080: the receipt prints calendar::record_horizon(today) (calendar.rs:553-591) when it is ends_soon or ended. R4-11's command gets both by calling the verb.
TESTS: IA-JR-01 the_api_journal_reader_names_member_keys_and_holes covers /imports.json, /audit.json?feed=gdfl through imports::summary (audit_json.rs:311), /dayflags.json and /provenance.json, on a fixture where `*+M` is a hole and `*` is done for the same day. Also a_day_bands_refusal_names_its_run_from_the_event_not_the_clock, a_source_census_is_named_on_the_receipt, and IA-GR-34 reworded to the_calendar_horizon_is_warned_on_the_receipt_before_it_passes, with an injected today.
COST: every lookup it adds stays O(1) per row, one memoised split per distinct filter as the readers already do, named in docs/06-limits.md (DB-A34).
ORDER: it waits only for IA-16 (the reasons and the run field); IA-11r, IA-2 and IA-3 are merged.
DECISIONS: it lands D-4896. The cut's block D-6900..D-6909 retires D-4821, D-4840, D-4846 and D-4884 by name, moves D-4805 to R3-23 and D-4848's surviving clauses to IA-6, and amends merged D-4883's caller list. It also amends merged docs/04 IA1-01 ('The api half waits for POST /gdfl/import (IA-10a)', docs/04-invariants.md:7601), whose api half will never exist.
DROPPED, never built:
- the child mode in api/src/main.rs and its build-skew check;
- /gdfl/sources.json, /gdfl/plan.json, POST /gdfl/import, /gdfl/import.json, stop and dismiss;
- GET /alerts.json and POST /alerts/ack (IA-11a's merged `cli alerts` stays the queue's reader);
- the restart scan, auto-resume and their alerts;
- the build-skew and calendar-horizon alerts;
- the shutdown stop files and release lines in end_runtime;
- the child's streams;
- plan.json's free-space line;
- those routes' audit-list rows, the IA-MX R1-R14 route rows, and bench C-AL-03;
- the page fields DB-A50 (R3-23 answers it), DB-A41's log-lines-by-run route (imports.rs run_json and /logs.json?run= already find them) and DB-A48 (/imports.json uses 'day imported' events only for pace, imports.rs:1115-1146; R3-24 marks the contradicting lines on /logs.json).
A second import of a kind stays refused run_busy at the cli door (AF-01) under IA-2's locks. That door (crates/cli/src/gdfl_import_guard.rs) is the ONLY place an import takes its kind lock and claim: pull::gdfl_run::execute and execute_kind run unclaimed and take none (gdfl_run.rs:1335-1340). So every caller, R4-11's single command included, goes through the door.
FAILURE ROWS: FM-080 stays here (item 7b); FM-124 moves to R4-11; FM-069's display half and FM-084 move to R3-23; FM-071 closes as not applicable.
DROP, DO NOT BUILD: Everything except the reader half and the receipt lines above. That is:
- the api child mode with its build-skew check;
- every /gdfl/* route (sources, plan, POST import, import.json, stop, dismiss) and the IA-MX R1-R14 route-validation rows (design :3991-4003);
- the alerts routes (GET /alerts.json with older_unacknowledged, POST /alerts/ack) and bench C-AL-03;
- the restart scan and auto-resume with their alerts (store_lost from the child's exit);
- the build-skew and calendar-horizon alerts;
- the shutdown stop files and release lines in end_runtime;
- the child's stdin, stdout and stderr streams;
- plan.json's free-space line;
- the route audit lists;
- the page fields DB-A50, DB-A48 and DB-A41's route half (design :4901).
Each of these exists only so the api can start, watch, resume or announce an import that nobody launched from a terminal: a press from a page, an auto-resume after an api restart, an alert queue for an absent operator. With Claude Code or the operator running cli gdfl-import, the cli is the process, and its door already holds the lock, the claim, the progress record and the receipt. Two items are NOT dropped although the first verdict's list implied they went with the pages:
- D-4896, the day band's run lookup, which is /db's /dayflags.json read;
- D-4817's near-miss names, which move from /ingest to the receipt.
The design decisions only this half needed are retired by name in the cut's block: D-4821 (plan ETA), D-4840 (route classes), D-4846 (every run a child of the api) and D-4884 (build skew and the horizon alert).
EDGE NOTE (assembler): the dependencies on IA-11a, WP-39 and WP-81 are cut with the dropped half (all three are merged); IA-16 stays; IA-2, IA-3 and IA-11r are merged. IA-12a is dropped by this cut on this verdict (its panel called only the dropped routes), and R4-05 -> IA-12a is cut.
(Decision D-6900; one-command cut, round 2; verdicts: state/oneclick-cut-20261020/verdicts.json; failure cases: state/oneclick-cut-20261020/cases.json.)
````

### 3.3 Global launcher notes that name IA-10a (verbatim)

Source: `state/cloud-20261024/args.json` (launcher line 6), the three `_note*` keys whose text
names IA-10a. They explain how the part got its place and its cut.

````text
_note_1730: 18 Oct split (operator ~13:17 IST: apply at the 17:30 relaunch): IA-4, R4-21, R4-25 parked with every edge to them cut (args.parked); mixed parts narrowed to their step-1 halves (args.specAppend); IA-6 and IA-10a right after IA-16; SC-3, SC-4, R3-07, R3-10 back on the Mac (their 17 Oct cloud owner never started and the old account is gone; R4-03 and R4-23 on the path need them); R4-11 and R4-16 wait for SURV-1 (new-work run); SC-6 seals now, provenance later as a new sealed generation

_note_backtest_first: 18 Oct backtest-first cut (operator 21:03 IST; wf_7817300e-98c re-cut + attack; state/backtest-first-20261018/): critical path 18 -> 8 steps (IA-2 > IA-3 > IA-16 > IA-10c > R4-20|R3-28|R4-19 > PREC-2|R4-23 > PREC-14|PREC-10 > PREC-19|PROVEN-GATE). Needed set (53 parts incl. new PROVEN-GATE D-6230.., R4-09a D-6240.., NEW-34q D-6250.., IA-10c D-6260.. as extraPackages) first in priority; every deferred automation part (IA-10a, IA-7, IA-10b routes half, R4-02, R4-10..R4-15, SC-5..SC-12, R3-xx automation, conditional R3-22/PREC-5/CC-3/CC-18, ...) last and still built here after (never parked); every edge from a needed part to a deferred part cut via dropDeps, FILES_ONLY links to deferred parts removed (kept in _filesOnly_cut_backtest_first); IA-10b's dependents re-pointed to IA-10c; re-cut links R4-18 -> R3-28 and R4-26 -> R4-23; MIXED parts narrowed to their data-proof piece by specAppend (BACKTEST-FIRST), cut links explained (CUT LINK). Zerodha proof fixed to mode (i): full re-pull with kept answers. cloudOwned parts (R4-17) are not added to only (one owner per part). Needed parts whose package lives in no launcher yet (PREC-2/3/4/9/11..16/19, CC-1/2/7/17) are listed in only by name, which is a no-op in this launcher: the run that owns them must add their packages before the backtest; no in-run part waits on them.

_note_oneclick_20261020: 20 Oct 2026 ~12:50 IST, operator over Remote Control: ingest is always driven by Claude Code or the operator from a terminal; the unattended layer of the one-click ingest is dropped and everything that protects the data stays (decision D-6900, block D-6900..D-6909; round 2, assembled from the second attack round of wf_5e601f33-82f). DROPPED (never built): IA-7, R4-12, R4-14, R4-15, and IA-12a (by IA-10a's verdict). SPLIT (unattended half dropped, text in specAppend): R4-11, IA-10a, IA-16, R4-13, R4-10, IA-10b, R4-23, R4-26, R4-28. KEPT with amended scope: R4-02, R4-19, PREC-10. Receivers of moved items carry MOVED IN / AMENDED / NOTE text. File-order links added: R4-05 -> R3-21, R3-27 -> R3-24. Proof 2 now reads 'one command from the Dropbox link' (state/oneclick-cut-20261020/decision.md). Failure cases re-owned: state/oneclick-cut-20261020/cases.json.

````

### 3.4 The 16 Oct cloud spec

`git show origin/cloud/db-base-20261016:reports/db-specs-20261016/IA-10a.md` (889 bytes) holds
the prerequisites line `IA-2, IA-16, IA-3, IA-11a, IA-11r, WP-39, WP-81` and the EMBED spec of
section 3.1, word for word. **It adds nothing**; its prerequisite list predates the cut's
`dropDeps`.

### 3.5 D-6900 as recorded at the head (verbatim excerpts)

D-6900 is already headed in `docs/05-decisions.md` at e851da1ca (LEDGER-1023, 23 Oct). These
are its paragraphs that bind IA-10a. Do **not** re-head D-6900; it is merged.

Source: `docs/05-decisions.md at e851da1ca`, lines 82417,82429 82484,82502 82656,82690

````text
### D-6900 — Ingest is always driven by Claude Code or the operator: the unattended one-click layer is dropped, one `cli ingest-run` command carries the chain, and everything that protects the data stays (one-command cut) — 2026-10-20

**Status: locked.** Decided by the operator on 20 October 2026 at about 12:53
IST, over Remote Control, in these words: *"anyhow always claude code ... just
go ahead"*. **Recorded here on 23 October 2026** by the main session
(LEDGER-1023) from the cut's draft, `state/oneclick-cut-20261020/decision.md`
in the operator's work folder, outside this repository; the 21 Oct hand-over
listed it as owed at landing. Block D-6900..D-6909: D-6901 is reserved for
R4-10's import-journal decision and D-6902..D-6909 stay unused. The text below
is the decision as made on 20 Oct. Where D-7135 (23 Oct, below) binds over a
part of it, that part is named in **Read with D-7135** at the end, and is not
reworded in place.

   - **IA-12a**, as a consequence of IA-10a's verdict: its /ingest panel calls
     only the dropped `/gdfl/*` routes. It owns no failure case.

   They leave `.only`, `.priority` and `.criticalPath`, and every edge to them
   is cut.

2. **Split.** The unattended half is dropped and the data half is built. Each
   part's exact KEEP and "DROP, DO NOT BUILD" text is appended to its spec.
   - **R4-11.** Kept: `cli ingest-run` and `pull::chain_journal`. Dropped: the
     drop-folder watcher (DLY-02), `POST /ingest/run`, the panels (E2E-06) and
     the route door (E2E-08).
   - **IA-10a.** Kept: the read side. Every api reader of the GDFL journal's
     filter understands member keys and holes (`/imports.json`,
     `/audit.json?feed=gdfl`, `/dayflags.json`, `/provenance.json`, the
     open-day scan), the day band finds a refusal's run from the event itself
     (D-4896, O(1)), and the cli receipt prints the source census names
     (D-4817) and the calendar horizon. Dropped: the api child mode, every
     `/gdfl/*` route, the alerts routes, the restart scan and auto-resume.
   - **IA-16.** Dropped: only its `alerts::put` calls for holes,
6. **Design decisions this cut retires, moves or amends:**
   - **Retired** (never landed; nothing to build): D-4821 (the plan ETA);
     D-4840 (the route classes); D-4846 (every run a child of the api); D-4884
     (the build-skew and calendar-horizon alerts); D-4806 and D-4886 (inline
     verification); D-4856's round-5 end-of-run rules, including "No day waits
     for a person while its input exists". A pending day now waits at most
     until the next command, and it is named on every command until then.
   - **Moved:**
     - D-4805 to R3-23 (an interrupted run is a claim in a free lock);
     - D-4848's surviving clauses (mode= on every claim, auto_resume=0 for
       every cli claim, an interrupted verification re-run as a verification
       and never resumed as an import) to IA-6;
     - D-4837 and D-4863's code-6 sentence to IA-6, reworded to the cold mode;
       D-4863's audit-cap sentence to IA-16;
     - the verdict store's caller duties of D-4925 (near_ceiling) and D-4924
       (lock_stalled) to IA-6's cold run, printed on its receipt with no
       alert.
   - **Landed by a kept half:** D-4896 (the day band's O(1) run lookup) by
     IA-10a, with L-IA-39 rewritten.
   - **Amended merged entries:**
     - D-4883's caller list loses IA-16's three puts (disk_full, hole,
       store_damaged), IA-7's two (mismatch, verdict_refused), IA-10a's alert
       kinds and routes, and bench C-AL-03. `cli alerts` (IA-11a, merged) stays
       the queue's reader.
     - D-4925's "The alert is IA-7's": near_ceiling is printed by IA-6's cold
       run, with no alert.
     - The ownership line "IA-7: the deferred pass's driver" now reads: the
       deferred pass is `cli gdfl-verify <kind> - --deferred` (IA-6), run by
       R4-11's command or by hand.
     - docs/04 IA1-01 ("The api half waits for POST /gdfl/import (IA-10a)"):
       that api half will never exist; the cli door is the one import door.
     - L-IA-12 is WP-16's alone, with no inline gather term (merged L-IA-60
       cites it); L-IA-3 and L-IA-35 are never measured.
     - D-4968 stays as written; "R4-10's rolled non-bar segments" are not
       built.
````

## 4. Scope in one table: what to build, what never to build

This table is a reading aid assembled from section 3.2; where it and section 3.2 differ,
section 3.2 wins. Line numbers in the "Where at e851da1ca" column were re-grepped at the head
(several spec citations drifted by a few lines; section 14 lists each).

| # | BUILD (kept) | Where at e851da1ca | Answers |
|---|---|---|---|
| 0 | `pull::gdfl_cm::member_share` from `pub(crate)` to `pub`, so every api reader maps a member key to its share by pull's one rule | `crates/pull/src/gdfl_cm.rs:1349` (spec says `:1309`); its user `shares_of`, `gdfl_import_journal.rs:554-561` | F84 |
| 1 | `crates/api/src/import_journal.rs` parses `<filter>+<member>` into base filter and member; an `incomplete` row whose reason is one of the five store-hole reasons carries `hole: {reason, repairable}`, never shown as a plain incomplete | `DayRow.filter` is one opaque `String` (`:142-172`); `fold` reads `[verb, kind, day, filter, rest @ ..]` (`:433-455`); no `'+'` parse in any api file | F84, SF-9, IA-JR-01 |
| 2 | `crates/api/src/imports.rs`: `/imports.json` counts holes by reason and `holes_unrepairable` apart from incomplete; `Owners::names` reads a member key as naming its member's share and as a close of its base filter's kind-day, so a day with only member keys is journalled and gets no made-up `logged_only` `*` row | `Owners::names` `:1072-1085`; `logged_only` `:793`; `merged` `:909` | F84, IA-JR-01 |
| 3 | `crates/api/src/dayflags.rs` filter match: a member-key row covers its member's share and nothing else; the base close's members keep the base close; a member's hole shows on `/dayflags.json` and on `/db`'s day band | `journal_rows` `:905`; `Names::names` `:1044-1053` (splits on `,` only) | F84 |
| 4 | `crates/api/src/feeds_route.rs` open-day scan keeps its answer for base keys and states what it does with an open member key | `row.filter == "*"` at `:1124` | F84 |
| 5 | `crates/api/src/provenance.rs` `filter_holds`: a member-key row is its member's share's day row (its bars never credited to the base close's run); a member hole shown as a hole | `filter_holds` `:1022-1024`, called at `:964` and `:1046` | F84 |
| 6 | D-4896: `dayflags.rs` `refusals_of` resolves a refused event's run from the event's own run through IA-11r's run-id index, O(1) per event; time placement (`imports::placed`) only for an event without it, and the answer says so; rewrite L-IA-39; head D-4896 | `refusals_of` `:1181`; `start_of` `:1130` is a linear `find`; `Parsed::runs_of` `import_journal.rs:333` is the index | DB-A34, D-4896, L-IA-39 |
| 7a | `cli gdfl-import`'s receipt prints each source's census (ignored, misfiled, invalid counts and the first 16 names of each; at most 48 names) | `ArchiveCensus` `gdfl_archive.rs:471`, `DayZipsCensus` `gdfl_dayzips.rs:199`; receipt `render` in `crates/cli/src/gdfl_import_command.rs:834` | D-4817 |
| 7b | the receipt prints `calendar::record_horizon(today)` when `ends_soon` or `ended` | `record_horizon` `crates/pull/src/calendar.rs:1056` (spec says `:553-591`) | FM-080, IA-GR-34 (reworded) |
| T | tests: IA-JR-01 `the_api_journal_reader_names_member_keys_and_holes` (over `/imports.json`, `/audit.json?feed=gdfl` via `imports::summary`, `/dayflags.json`, `/provenance.json`, fixture: `*+M` a hole and `*` done the same day); `a_day_bands_refusal_names_its_run_from_the_event_not_the_clock`; `a_source_census_is_named_on_the_receipt`; IA-GR-34 as `the_calendar_horizon_is_warned_on_the_receipt_before_it_passes` with an injected today | none exists at the head | |
| L | ledgers: head D-4896 (and rewrite the forward-reference bullet `docs/05-decisions.md:70649`); rewrite L-IA-39 (`docs/06-limits.md:16983-17011`) and name each added lookup's cost (DB-A34); amend docs/04 IA1-01 (`docs/04-invariants.md:7628`, "The api half waits for `POST /gdfl/import` (IA-10a)"); docs/04 rows for the four tests | | D-4896, DB-A34 |

| DROPPED, never built (section 3.2 "DROP, DO NOT BUILD") |
|---|
| the api child mode in `api/src/main.rs` and its build-skew check |
| every `/gdfl/*` route: `sources.json`, `plan.json`, `POST /gdfl/import`, `import.json`, stop, dismiss; the IA-MX R1-R14 route rows (design :3991-4003) |
| `GET /alerts.json` (with `older_unacknowledged`), `POST /alerts/ack`, bench C-AL-03 (IA-11a's merged `cli alerts` stays the queue's reader) |
| the restart scan and auto-resume with their alerts (`store_lost` from the child's exit) |
| the build-skew and calendar-horizon alerts |
| the shutdown stop files and `release` lines in `end_runtime` |
| the child's stdin, stdout and stderr streams |
| `plan.json`'s free-space line |
| the route audit lists (`operation_audit_tests.rs` rows for the dropped routes) |
| the page fields DB-A50 (R3-23 answers it), DB-A48 (R3-24 marks it on `/logs.json`; `/imports.json` uses "day imported" events only for pace) and DB-A41's log-lines-by-run route (`imports.rs` `run_json` and `/logs.json?run=` already find them) |
| decisions retired by D-6900 at the head: D-4821, D-4840, D-4846, D-4884; moved: D-4805 to R3-23, D-4848's surviving clauses to IA-6; the critic's D-4913, D-4914, D-4915 have no surviving subject (they amend only dropped routes) |

The cli door `crates/cli/src/gdfl_import_guard.rs` (AF-01) is the ONLY place an import takes its
kind lock and claim; `pull::gdfl_run::execute` and `execute_kind` take none. Do not add a second
door.

## 5. The design text (verbatim)

Source file on the Mac: `state/design/ingest-automation.md` (6,154 lines). Every piece the
`ia` branch of `specStep` points to is copied. The round notes, lanes and order describe the
plan of 10-11 Oct; the cut of 20 Oct (section 3.2) overrides every part of them that concerns
the dropped half.

### 5.1 Section 16.2: the round notes above the table that name IA-10a, the table header and the IA-10a row

Source: `state/design/ingest-automation.md`, lines 4806,4848 4884,4885 4901

````text
### 16.2 The packages

Round 3 dissolves IA-0 (each decision lands with its package, §15 "Lands
with"), withdraws IA-15 into WP-12d (D-4874), and adds the six new fix-round
packages to the "Depends on" column wherever they own a file or a rule first.
**IA-5's specification changed in one way only** (code 18 `owed` and its
severity, D-4879, part of format v1); it still depends on nothing unmerged,
and `fix-plan.json`'s IA-5 package should take that change.

**Round 4** gives IA-2 the whole stop (SF-11), IA-3 its cause arms and
D-4818's (SF-10), IA-4 the options ledger count (BL-E), IA-16 `admit`'s table
and `decisive()`'s hole order (BL-B, BL-C, SF-18), IA-6 D-4845 instead of IA-5
(BL-F), and adds two packages: **IA-11a**, the alert queue's writer, at step 2
before every caller (SF-7), and **IA-17**, D-4875 built, only if WP-13c or
WP-12d merge without it (SF-5).

**Round 5** gives IA-7 the deferred pass's driver alone (SF-12) and the rule
for who runs a pending day (SF-6), IA-16 the `repair_impossible` split, the
`repair_in_open_month` warning and the held-back check's place (SF-10, SF-9,
SF-11), IA-6p the move of WP-37's five merged tolerance shapes (SF-1), IA-6
the journal rule set of a repaired month (D-4887, B4) and the base-close
ledger comparison (B3), IA-11a the alert format of §7.1 (B2), IA-10a the
child's streams (SF-7), narrows **IA-17 to the row ceiling** (and, if WP-12d
leaves them, the once-per-month gather and the CASH `.auc`), adds IA-2 to
IA-9's dependencies (SF-8) and WP-81 to IA-1's and IA-2's (F76), and finds
that **IA-5's WIP needs nine amendments, not five** (B1, SF-13): the round-4
claim "ready once its WIP takes the five amendments" was false, and is
withdrawn.

**Round 6** (WP-33c and WP-13c merged before the round-5 text was saved)
makes **IA-11r exactly R3, the widened R5 and R6's O(1) half**, with a
mechanism, tests, benches and limits for each, at step 2 before IA-1 (BG-1);
makes **IA-17 live at step 2** with its test and bench ids, and adds the
`before_promote` step to it (SF-2, SF-3); gives IA-16 `pull::audit_ledger`
(D-4888, SF-4), the one repair-reason rule (SF-10) and the Zerodha side of its
door narrowing (SF-5); gives IA-10b a runs-ledger id for every Zerodha landing,
`cli zerodha-repair --from-store` included, the gap ledger's move to the audit
ledger lock, and a dependency on IA-16 (SF-1, SF-10, SF-4); gives IA-10a the
api journal reader's member keys and holes (SF-9) and a dependency on IA-11r;
gives IA-11a the readers' shared lock, the text cut and the damaged-queue
replacement, and IA-14 its runbook step (SF-8); gives IA-6 detail 17 from the
pointer and the journal (SF-6); and adds a **tenth amendment** to IA-5's WIP
(SF-7: the design takes the WIP's names).
| Pkg | Name | Depends on | Files owned (non-overlapping, but for §11's shared lines) | Size | Lane |
|---|---|---|---|---|---|
| IA-10a | API, import half: the child mode in `api/src/main.rs` with the build-skew check; `/gdfl/sources.json`, `/gdfl/plan.json`, `POST /gdfl/import` (one child per kind on v4, one per press on v3, and the api's half of the two-phase start, round 4, BL-A; with `repair_reason`, `snapshot`), `/gdfl/import.json`, stop, dismiss; `GET /alerts.json` (with `older_unacknowledged`, round 5, B2) and `POST /alerts/ack`; the restart scan, auto-resume and its alerts (`store_lost` from the child's exit status, round 5); the build-skew and calendar-horizon alerts; shutdown stop files and `release` lines in `end_runtime`; **the child's streams** (two stdin lines, one stdout line, stderr the kind's `stderr` file; the 202 handler's wait on `spawn_blocking`; the partial press named; round 5, SF-7); `plan.json`'s free-space line from WP-81's `/disk.json` measurement (round 5, F76); route audit lists; **round 6 (SF-9, F84)**: the api's own journal reader and `/imports.json` name member keys `<filter>+<member>`, show the five hole reasons as holes (repairable or not) and count `holes_unrepairable`, test IA-JR-01 | IA-2, IA-16, IA-3, IA-11a, **IA-11r** (round 6: it edits `imports.rs` and `import_journal.rs` first, and IA-1 waits for it anyway), WP-36 (merged), WP-32 (merged), WP-39; WP-81 only for the free-space line | `api/src/main.rs`, `gdfl_run_route.rs`, `alerts_route.rs`, `server.rs` (registration, `Site`, shutdown, exe identity), `operation_audit_tests.rs`, `import_journal.rs` and `imports.rs` (member keys and holes, after IA-11r) | L | F |
````

### 5.2 Section 16.2: "Lanes" and "The order"

Source: `state/design/ingest-automation.md`, lines 4910,5013

````text
**Lanes.** A: IA-1, IA-2, IA-16, IA-7 (after IA-6), IA-17 (step 2; its later parts after IA-16). B:
IA-3, IA-4, IA-0b. C: IA-11a, IA-11r. D: IA-5, IA-6, IA-6p. E: IA-8, IA-9. F:
IA-10a, IA-10b. G: IA-12a, IA-12b. H: IA-13. Each lane owns files no other lane
owns at the same time; where two packages edit one file (`gdfl_import.rs`,
`gdfl_run.rs`, `gdfl_nfo.rs`, `server.rs`) they are in one lane, in order,
after the fix-round owners, **or the cross-lane edit is named and ordered by a
dependency** (round 4, SF-10: round 3's "the one cross-lane edit" was false).
`gdfl_import.rs` is edited, in order, by WP-16, WP-11c, WP-12d, WP-12f, then
IA-1, IA-2, **IA-3** (lane B: the `cm_cause` and `nfo_cause` arms of its
variants and D-4818's three, ordered by IA-3's dependency on IA-2 and IA-16's
on IA-3), IA-16, **IA-4** (lane B: the constant, `definition_is_current`,
`option_handed`'s counts and its `nfo_cause` arms, ordered by IA-4's
dependency on IA-16), IA-7, IA-17 (the gather, if WP-12d leaves it).
`gdfl_import_quality.rs` is edited by IA-16, then IA-4 (lane B, the same
dependency). **Round 5 names two more cross-lane edits, each ordered by a
dependency:** IA-7 (lane A) adds one `Spawn` hook to
`cli/src/gdfl_verify_command.rs` (IA-6's, lane D) and one to `api/src/main.rs`
(IA-10a's, lane F), after both, because IA-7 depends on IA-6 and IA-10a; and
IA-17 (lane A), if it builds the CASH `.auc`, edits `gdfl_cm_auction.rs` after
IA-16, which it depends on. **Round 6 names four more, each ordered by a
dependency:** IA-11r (lane C) edits `imports.rs` and `import_journal.rs`, and
IA-10a (lane F) edits them after it for member keys and holes (IA-10a depends
on IA-11r); IA-10b (lane F) edits `ingest.rs` after IA-16 (lane A) and
`request_minutes.rs` after IA-9 (lane E) and WP-62, for `Plan.run` and the gap
ledger's lock, and depends on both; IA-10b also edits WP-13c's merged
`cli/src/zerodha_repair.rs`, which no other IA package edits (IA-16 changes
only the expected words of its tests, before IA-10b); and IA-17's step-2 edits
of `store/src/repair.rs` and `store/benches/ratio.rs` share `ratio.rs` with
IA-5 (lane D), each appending its own bench under its own heading (§11's
shared-file rule).

**The order, with every dependency (a topological order; `→` is "then";
round 5 moves IA-11r's R5 part ahead of IA-1, nit; round 6 moves all of IA-11r
and IA-17's first part to step 2, BG-1, SF-2):**

1. Fix round: WP-16 → WP-11c; {WP-12d, WP-13d, WP-62} (WP-13c has merged);
   WP-12d → WP-12f (after WP-16 too); WP-12c → WP-12e; WP-36; WP-39; WP-61;
   WP-54; WP-81. (WP-11b, WP-38b, WP-50, WP-36, WP-37, WP-32, WP-51, and since
   round 6 WP-33c, WP-13c and WP-52 have merged.)
2. Four packages that need nothing unmerged: **IA-5** (ready once its WIP
   takes the **ten** amendments of its row, round 5, B1; round 6, SF-7),
   **IA-11a** (the alert queue's writer, before every caller, round 4, SF-7;
   its format as §7.1 fixes it, round 5, B2; its readers' lock and the
   damaged-queue replacement, round 6, SF-8), **IA-11r** (R3, the widened R5
   and R6's O(1) half, round 6, BG-1: WP-33c merged without them, and IA-1's
   directory switch needs the widened R5 first) and **IA-17's first part**
   (the grid ceiling and the `before_promote` step in `store::repair`, round
   6, SF-2, SF-3: WP-13c merged with the 100,000-row ceiling).
3. IA-1 (after WP-16, WP-11c, WP-12d, WP-12f, WP-81 and IA-11r) → IA-2 (after IA-11a, WP-81, WP-12c) →
   IA-3 (B, after IA-2, WP-12c, WP-12e, WP-12f) → IA-16 (after IA-3, WP-13c,
   WP-13d, WP-12d, WP-39, IA-17's first part) → IA-10a (after WP-39 and IA-11r; WP-36 has merged) → IA-12a
   (after WP-61). IA-1 and IA-2 land back to back, and neither leaves a build
   whose stop closes a day `refused`: the stop is IA-2's whole (round 4,
   SF-11). **Milestone M1: the app imports GDFL from `/ingest` itself**, fast
   (WP-16), stoppable, resumable, with no Claude session in the loop, and it
   names every hole, alerts on it, and never re-reads it without a repair
   reason. **Where M1's evidence lives** (round 6, BG-1): IA-1's directory
   switch lands after IA-11r's widened R5, so from IA-1 on each run's events
   and refusal evidence live only in its own directory (`logs/gdfl/<kind>`, or
   `logs/gdfl/store` on v3) and every page that shows them (`/logs`,
   `/imports.json`, `/dayflags.json`'s day band, `/store`) reads them there;
   before IA-1 every run logs to `logs/cli`, where the same pages read it.
   **What M1 does not yet do, said plainly** (round 3, SF-4, SF-1, SF-3; round
   4, SF-5): **a repair of a real 1s month runs, but not yet once per month**:
   its row ceiling no longer stops it, because IA-17 raises the ceiling at
   step 2 (round 6, SF-2: WP-13c merged with 100,000 rows, `repair.rs:29`,
   against about 425,817 bars a NIFTY month; a build without IA-17 would answer
   `repair_refused (RowLimit)`, named, and admit those holes again once it
   lands); but until D-4875 (3)'s once-per-month gather is built (WP-12d's,
   else IA-17's at step 6) each repaired day copies its whole month, so IA-14
   lists V-4 and V-5 as ready only at M4 (L-IA-26);
   the journal keeps one decisive reason per day, and refusals by name are in
   the progress file (newest 8) and the audit stream (first 16 per (run, day,
   stage, code)) only, until M2's day images; no day after 2026-10-04 imports
   until WP-62 extends the calendar (a horizon already passed on 10 Oct, so the
   horizon alert fires daily from IA-10a's landing, §6.9); a stopped half day shows on `/ingest`
   and `/imports.json` but not on `/db` until WP-54. Alerts reach the operator
   through IA-11a's queue on `/ingest` and in `cli alerts`; a notification or
   webhook is WP-61's to add.
4. IA-4 (after IA-16) → IA-0b; IA-6 (after IA-16, IA-4, IA-5, WP-13c, WP-12c, WP-12e,
   WP-12f) → IA-7 (after IA-10a too, round 5) → IA-10b (after WP-33c, WP-11c, WP-62, WP-13d) → IA-12b
   (after WP-54, WP-61). **Milestone M2: every imported day verifies
   itself**, with `/verify` and the `/db` badge, and every refusal by name in
   the day image; V-1 can run (V-0b is done).
5. IA-8 (after WP-13c, WP-13d, WP-62) → IA-9 (after IA-2 too, round 5, SF-8) → IA-10b's Zerodha hook.
   **Milestone M3: Zerodha kept answers and verification**; V-3 after WP-13d's
   repairs.
6. **D-4875 built** (round 4, SF-5: a named step and owner; narrowed in round
   5, B4; round 6, SF-2: its first part moved to step 2): the row ceiling and
   the `before_promote` step are IA-17's at step 2; the once-per-month gather
   and the CASH `.auc` writer are WP-12d's, else IA-17's after WP-12d and
   IA-16; the revision's `.tix` is already WP-13c's, its `.srt` WP-39's, and it
   carries no `.prv` (D-4887). **Milestone M4: holes a re-run can fill only
   by a repair are filled by month generations**; V-5, V-4 (each after V-S,
   which needs WP-81's runbook); `repair_impossible` and `bars_differ_held`
   holes stay named, never read (round 5).
7. IA-6p (after IA-6; WP-37 has merged); IA-13 as each package lands.
   (Round 5's "IA-11r's R3 part" is at step 2 since round 6.)

**The options backfill and definition 4 (round 3, D-4877, L-IA-28).** WP-16
makes the options backfill fast before IA-4 lands. Every options day imported
in between closes `definition=3` and is restated by IA-4's bump. The plan
shows the count and the cost; the operator may run it either way. To avoid the
restate, run it after IA-4.
````

### 5.3 Section 16.3 (CI): the gates, the table header and the IA-10a row

Source: `state/design/ingest-automation.md`, lines 5015,5033 5049

````text
### 16.3 CI: what each package adds so its own change is green (D-4861)

Gates read here: gate 10 (`ci.yml:2656`, every invariant row names a test that
exists), gate 11 (`:2820`, banned constructs; allowlists in
`.github/gates_ledger.rs`: `ALLOW_SEARCH` `:1337` allows only
`crates/store/src/file.rs`, `ALLOW_SORT` `:1445`, `ALLOW_SCAN` `:1558`,
`ALLOW_MEMBER` `:1623`, `ALLOW_UNSIZED` `:1379`; each entry's reason in
`docs/06-limits.md` under "Gate 11 allowlist reasons", `:13173`), gate 12
(`:2976`, a cost claim in a `///` or `//!` block names a test or bench, a
`docs/04` row whose test exists, or says UNVERIFIED), gate 14 (`:3346`, a crate
that claims a bound re-measures it), gate 19 (`:2537`, a failure an operator
can see is a failure the log records), gate 21 (`:1997`, telemetry's declared
`std::fs` surface, by construct and count), gate 23 (`:1477`, declared stderr
writers), gate 27 (`:2463`, one invariant per id; `id_grammar`,
`gates_ledger.rs:458`), gate 27b (`:2489`, one entry per D-number), gate W1
(`:4438`, the committed `web/build` equals a fresh build; 64 tracked files).

| Pkg | `gates_ledger.rs` entries (count added) | `docs/06-limits.md` rows | `docs/04-invariants.md` rows | Other gates |
|---|---|---|---|---|
| IA-10a | none expected | — | IA-GR-01, -02 (the api's half, with the partial press), -06, -06b, -12, -21 (its half), -23, -25, -29 (with the broken-pipe case), -33, -34 (its half); IA-JR-01 (round 6, SF-9); IA-MX rows of §8 R1–R14 | 23: the child mode's one declared stderr writer (round 5, SF-7); 12: C-GR-03, -05, -10, -12, -13, -14, C-AL-02; the route audit lists (`/alerts.json`, `POST /alerts/ack` among them) |
````

The 16.3 row's gate 23 item (the child's stderr writer) and gate 12 benches C-GR-03, -05, -10,
-12, -13, -14 and C-AL-02 belong to the dropped half. What the kept half meets: gate 10 (each new
docs/04 row names a test that exists), gate 27 and 27b (one invariant per id; D-4896 headed
once), gate 12 (any `///` cost claim names a test, bench or docs/04 row, or says UNVERIFIED),
gate 11 (no new sort or scan without its allowlist entry; the memoised filter split needs none if
written as a hash probe).

### 5.4 Section 3: the gaps this part fixes (F84 kept; F33, F76, F77 dropped with their subjects)

Source: `state/design/ingest-automation.md`, lines 504,511 544 593,596 602,603 605,608 615

````text
## 3. What grounding this design found

These gaps exist in the code today. Each one changes the design. A bare
`F<n>` outside §8 names a finding of this table; a row of the input case
matrix is cited as `§8 F<n>` (inside §8's own tables, as "row F<n>").

| ID | Finding | Evidence | Consequence today | Fixed in |
|---|---|---|---|---|
| F33 | A server shutdown abandons a run it holds | `api/src/main.rs:52` drives shutdown from `ctrl_c()`; `end_runtime` (`server.rs:16711` at 9cfa4577) calls only `cli::cancel::request()` and waits on `sweeprun::engine_tasks_running()` (`wait_then_end`, `:16721`) for `SHUTDOWN_GRACE` 10 s (`:16693`). The cli has no signal handling (no `ctrlc` or `signal-hook` in `cli/Cargo.toml`) | An in-process import would be cut mid-file with no release; with auto-resume on, the operator's "stop everything" is undone at the next start | IA-10a (D-4846) |
**Round 5 findings** (code at 9cfa4577; the WIP branches named in the header read for intent; the store not read; each changes the design):

| ID | Finding | Evidence | Consequence | Fixed in |
|---|---|---|---|---|
| F76 | WP-81's WIP surfaces free space in the api, writes the snapshot step, and edits two files IA packages rewrite | `fix/db-WP-81` ce189751: `crates/api/src/disk_route.rs` (`GET /disk.json`, from `/bin/df -Pk`, measured at most every 30 s); `docs/36-ops-runbook.md` §4 ("Snapshot before every format or bulk write … `tmutil localsnapshot /Volumes/WD_BLACK` and note its name in the deploy or WP-80 log"); `cli/src/gdfl_import_command.rs` gains `unstamped_write_refusal` (DB-B71); `census_store.rs` gains `seal_legacy` (`census-migrate --seal`, DB-B68); no alert queue | `pull` cannot call the api's free-space measurement, so a preflight "call to WP-81's function" had no callee; IA-1's move of the cli setup and IA-2's `census_store` edits must come after WP-81's; V-S has a concrete command | §5.2 free-space row (IA-10a reads `/disk.json`'s measurement); §5.1 moved-functions table; §5.3 lock table (`--seal`); §6.8 V-S; §16.1 WP-81 row; §16.2 IA-1, IA-2 (round 5) |
| F77 | A Rust program that writes to a closed pipe panics, and a release build aborts | Rust's runtime ignores `SIGPIPE`, so a write to a closed pipe returns `EPIPE`, and `println!` and `eprintln!` panic on a failed write; `Cargo.toml:194` sets `panic = "abort"` for release | An api child whose stdout or stderr is a pipe to an api that has exited dies at its next print, mid-run | §4 "The child's standard streams"; D-4846; IA-GR-29 (round 5, SF-7) |
**Round 6 findings** (code at d73e1112, after WP-33c, WP-13c and WP-52 merged; `fix/db-IA-5` read for intent; the store not read; each changes the design):

| ID | Finding | Evidence | Consequence | Fixed in |
|---|---|---|---|---|
| F84 | The api's own journal reader shows a member key, a hole and an unrepairable hole as nothing in particular | `import_journal.rs` keeps `<filter>` as one opaque token (`:307`–`:328`) and an `incomplete` close's reason as text (`:397`); `merged` asks only for `row.filter == "*"` (`imports.rs:792`) | `/imports.json`, `/audit.json?feed=gdfl` and `/dayflags.json` would list the 556 member-day holes as `incomplete` and could not count `holes_unrepairable` | IA-10a (SF-9; test IA-JR-01) |
````

### 5.5 Section 5.5: Resume, member keys, the five hole reasons and `admit`'s table (the definitions IA-JR-01 tests against)

Source: `state/design/ingest-automation.md`, lines 1390,1458

````text
- **Resume.** `Range::Resume{to}` resolves `from` to **the first day, from
  the source's first day, on which `admit_rule` (below) admits any key of the
  day**: the base key under this filter, or a member key `<filter>+<member>`
  that row 1 opens (round 5, SF-14: read per base key, "neither done nor a
  hole" put `from` after every scope-widened day, whose base key is `done`, so
  a Resume never reached a reopen; IA-GR-37's "plan = run" already required
  this reading). The days it passes over are the ones `admit_rule` skips: done
  keys with no new member, and the holes. The holes are: `source_lacks` (the
  last line is `refused reason=day_not_held` and the chosen source's day table
  still lacks the day); `calendar` (a calendar refusal the calendar still
  gives); and the **store holes, each the reason of the key's last
  `incomplete` line** (round 3, SF-11): `needs_month_generation`,
  `month_superseded`, `repair_refused`, `repair_impossible` and
  `bars_differ_held` (below; D-4851, D-4873, D-4876). **Round 3 said a
  re-run "writes nothing" for such a day because `refused_close` suppresses an
  identical refusal; that is false at the code** (round 4, BL-B, F60):
  `refused_close` suppresses only an identical `Close::Refused` line
  (`gdfl_import.rs:2393`), every `incomplete` line clears it (`:2003`), and
  `admit` skips only `done` keys (`:2927`–`:2931`). So the skip is a rule of
  `admit`'s, below, and it holds for **every** range: Resume resolves `from`,
  and every hole after `from` is skipped by `admit` too; an explicit
  `Days{from,to}` skips the holes in it the same way and names them
  (`holes_in_range` in the plan and the receipt). A run without a repair reason
  reads no file of a hole and writes no line for it, whatever its range;
  `retry_refused` never admits one (a hole is not a refused close). **A day
  done under a narrower scope is not a hole and not done for the new members**
  (round 3, D-4878): WP-12f's admit reopens it for each new member under that
  member's own key, `<filter>+<member>` (for the `*` filter,
  `indices 2019-01-02 *+NSE-INDEX-NIFTYIT`; `+` is outside the symbol
  alphabet), reading the day's listing once and closing one key per member
  (round 4, BL-D, D-4878), and the plan counts it `scope_widened`. **A member
  key's close writes no stocks or options D-3335 line** (the `*` line stays the
  base close's; an index member writes its own index's line; round 5, B3,
  D-4878, IA-GR-40). `plan.json`
  shows `first_not_done` beside `first_to_import`, `last_done`, each hole
  count, `scope_widened` and `restated` (D-4821), O(days in range) hash probes
  into the loaded sets plus one O(1) day-table probe per day, and one probe per
  (day, new member) only on the days whose base scope lacks members. **A run
  with a repair reason** (D-4874) reads the repairable hole days in its range
  too (`needs_month_generation`, `month_superseded`, `repair_refused`), and
  lands their refused months through WP-12d's filing path; it is the only run
  that reads them. **No run reads a hole no repair can fill**
  (`bars_differ_held`, `repair_impossible`; round 5, SF-10).
- **`admit`'s rules, one owner, one order (round 4, BL-B, SF-18, D-4885).**
  `admit` (`gdfl_import.rs:2927` at ab2810b8) skips only `done` keys today.
  After WP-12f adds its per-member reopening, **IA-16 owns the function and its
  order**: it rewrites `admit` as one exhaustive table, `admit_rule(key state,
  request) -> Admit`, keeps WP-12f's rule as its first row (WP-12f's own test
  still passes), and adds the rest. Each key (a base key or a member key) is
  judged by the first row that matches:

  | # | The key's state | Without a repair reason | With a repair reason | Owner |
  |---|---|---|---|---|
  | 1 | base key `done`, and today's scope has members its last `done` close's scope lacks, with no key of their own yet | admit a member key `<filter>+<m>` for each such member, reading only their files; the base key itself is not read; a member key that exists already is judged by rows 2–6 | the same | WP-12f |
  | 2 | `done` under a current definition (a day done with refusals is admitted instead when the run says `retry_refused`, D-3317 item 2) | skip, `days_skipped` | skip | today's rule, kept |
  | 3 | `done` under a definition no longer current for the kind (restated) | `restate=1`: admit; `restate=0`: skip, `restated_skipped` | as without | IA-16 |
  | 4a | last line `incomplete` with a repairable store-hole reason (`needs_month_generation`, `month_superseded`, `repair_refused`) | skip, `holes_skipped` (never read, never written) | admit; the refused month goes to WP-12d's filing path | IA-16 |
  | 4b | (round 5, SF-10) last line `incomplete` with a store-hole reason no repair can fill: `bars_differ_held` (a held bar is never changed, D-3350) or `repair_impossible` (WP-13c refused for good: `HeldBarDiffers`, `IndexedSibling`, `RevisionLimit`) | skip, `holes_unrepairable` | skip, `holes_unrepairable`, named in `repair_days_refused`; never read again by any run, so a repair-reason run over the range is never refused again for it | IA-16 |
  | 5 | a calendar day not regular | closed: counted; any other answer: refused, as today | the same | today's rule, kept |
  | 6 | anything else (no line, `begin`, `incomplete` with any other reason, `refused`: an identical refusal is then not written again, `refused_close`) | admit | admit | today's rule, kept |

  Rows 4a and 4b are evaluated before the calendar gate because a hole's reason
  already says why the day cannot be read. **What can still fill a 4b hole** is
  named, not automatic: a `repair_impossible` month whose cause goes away (an
  overlay or greeks sibling WP-13c's `IndexedSibling` refused, removed by its
  owner) or a `bars_differ_held` day whose restatement a later decision allows
  (D-4882 names the D-3342 conflict for WP-80) is reopened by that decision's
  own package, never by this table. The plan calls the same function per key, so
  the plan and the run count the same days (IA-GR-19, IA-GR-37).
````

The five store-hole reasons as the code spells them at the head (`FailKind::code`,
`crates/pull/src/ingest_failkind.rs:233-248`; `HOLE_REASONS`,
`crates/pull/src/gdfl_import_causes.rs:838-844`): `needs_month_generation`, `month_superseded`,
`repair_refused` (the three a repair-reason run reads, `REPAIRABLE_HOLES`, admit row 4a), and
`bars_differ_held`, `repair_impossible` (no repair fills them, row 4b).

### 5.6 Section 6.9: the calendar's horizon (item 7b's source; the alert half is dropped, the receipt line is kept)

Source: `state/design/ingest-automation.md`, lines 3210,3239

````text
### 6.9 What stays manual, said plainly (critic: checks that still need a person)

**Three recurring human steps, each with an owner (round 3, SF-1, SF-2,
SF-3).**

- **The calendar's horizon, once a year** (DB-B25, DB-B41, F55). The calendar
  of record answers only to its last sourced day (`calendar::SOURCED_LAST_DAY`,
  2026-10-04 at 3f65ad43, `calendar.rs:523`), so an import of any later day is
  refused `calendar_unmeasured` and every Zerodha minute past it reads
  `calendar_unverified`. WP-62 owns the 2026 extension (to 2026-12-31 from NSE
  circular NSE/CMTR/71775, which DB-B41's reproducer read: four weekday
  holidays and a Sunday Muhurat on 8 Nov 2026). Each later year needs the
  operator to obtain NSE's holiday circular and a change that records it in
  the charter and the calendar (no invention, `CLAUDE.md` §3.1). This design
  automates the reminder only: `plan.json` and `/ingest` warn from 30 days
  before the horizon (§5.2), and from that day on the api (IA-10a) puts one
  alert a day into the queue through IA-11a's writer, at its start or its
  first plan of the day, until the record is extended (keyed
  `calendar_horizon:<last day>:<today>`, so a restart does not repeat it;
  D-4883, D-4884). **Concrete steps and owners** (round 4): WP-62 extends
  `SOURCED_LAST_DAY` to 2026-12-31 from NSE/CMTR/71775; from 2027 the yearly
  step is IA-14's runbook item "calendar: next year", performed by the
  operator (obtain NSE's holiday circular for the next year, then a change that
  records it in the charter and `calendar.rs` under a new D-number), due 30
  days before the horizon, which is when the alert starts. **On 10 Oct 2026 the
  horizon (2026-10-04) has already passed** (`days_left: -6`): WP-62 has no
  branch yet, so from the day IA-10a lands until WP-62's extension merges the
  api puts the horizon alert once a day, and M1 imports nothing after
  2026-10-04 (§16.2, M1's text). The yearly step after 2026 is the same alert
  and the same runbook item, with WP-62's extension as its model.
````

### 5.7 Section 8: the legend and table R (R1-R14 are this part's; all DROPPED with the routes)

Source: `state/design/ingest-automation.md`, lines 3717,3739 3987,4007

````text
## 8. INPUT CASE MATRIX

**Legend.** "Recognised by" and "Today" cite code at 10babda7 (the same
lines as fd2119fe for every `pull`, `cli` and `store` file). Test names
marked ✓ exist today; names marked + are new. **Each + test lands with its
package and gets its own `docs/04-invariants.md` row `IA-MX-<row>`** (§14),
owned by the section's package unless the row names another: A, B, C, D, E
and T → IA-3 (except A3 → WP-16, round 5, SF-17); F → IA-4 for the options reader's rows and IA-6 for the oracle's;
Z → IA-9 (Z16 with IA-10b); R → IA-10a, except R15 and R16 → IA-10b. Cost is per operation.
Every "O(bytes)" or "O(entries)" is a named limit (§10). A = containers and
sources. B = zip structure. C = day names and dates. D = CM file names. E =
option contract names. F = row formats. G = measured vendor faults. T = the
tick store. Z = Zerodha answers. R = requests to the api. **212 rows**: A 22,
B 27, C 14, D 25, E 28, F 44, G 8, T 12, Z 16, R 16 (round 4 adds F44, BL-E,
names the R3, R15 and R16 tests, N-2, and moves IA-GR-27 to IA-3, SF-10;
round 1: 179; round 2
added A22, F39–F43, T1–T12, Z12–Z14 and R11–R16; round 3 added D24, D25, E28,
Z15 and Z16, named a test for every row that had none, and changed the rows
§18 names). In these tables a bare `F<n>` in a "Today" or "After" cell names a §3
finding, and "row F<n>" names a row of table F. Where an "After" cell of
round 1 said "flag" for a per-day vendor-quality fact, round 2 reads it as the
D-3335 ledger field of D-4841 (the verifier's own observation goes in its
record's detail), and round 1's `absent_from_source` is `source_lacks_day`.
### R. Requests to the api

| # | Case | Recognised by | Answer | Test |
|---|---|---|---|---|
| R1 | A path in a request body | `gdfl_run_route` | 400 `path_not_accepted` | + `a_path_in_a_request_is_refused` (IA-10a; round 3 names each R test, SF-15) |
| R2 | A repeated field (`kind=a&kind=b`, `only=A&only=B`) | the form and query middlewares (`server.rs:14178`, `:14244`) | 400 before any handler; the page sends one `kinds=` and one `only=` field, comma-separated (F25) | + `a_press_sends_each_list_as_one_field` (web) and an api test that `kinds=a&kind=b` is refused |
| R3 | `stored=1` with options | `gdfl_run::plan` | 400 `stored_with_options`; the cli exits `MISUSED` with the same code (IA-GR-13) | + `stored_with_options_is_refused_by_one_authority` (IA-GR-13; round 4 names it, N-2) |
| R4 | A source word whose variable is unset for one of the kinds | `resolve_source` | 412 `source_not_configured{kind, variable}`; no kind starts | + `an_unset_source_variable_refuses_every_kind_of_the_press` |
| R5 | A range with nothing to import | `dry_plan` / `plan` | 200 `{"started":false,"why":"nothing to import"}`; no lock, no claim, no line (§5.3) | + IA-GR-01 |
| R6 | No kind, or an unknown kind | `gdfl_run_route` | 400 `kind_unknown` | + `a_press_with_no_or_an_unknown_kind_is_refused` |
| R7 | `workers` outside 1 to WP-16's bound | `gdfl_run_route` | 400 naming the field | + `workers_outside_the_bound_are_refused_by_name` |
| R8 | `from` after `to` | `Range` | 400 `range_backwards` (existing code) | + `a_backwards_range_is_refused` |
| R9 | Stop for a run that is not live; stop for a run the cli holds | `lock_state` | 404; or 200 `held_by:"cli"` after writing its stop file | + IA-GR-05 |
| R10 | Dismiss while the lock is held, or naming another run | `lock_state` | 409; 404 | + IA-GR-06 |
| R11 | `resume=1` together with `from=` | `gdfl_run_route` | 400 `range_conflicting` (Resume chooses `from`) | + `resume_with_a_from_is_refused` |
| R12 | An unknown source word (`source=zips`) | `resolve_source` | 400 `source_unknown{word}` | + `an_unknown_source_word_is_refused` |
| R13 | An invalid `only=` element (empty, longer than `SYMBOL_CAPACITY`, or outside the symbol alphabet), or **more than 256 names** (round 3, SF-6: the claim carries the filter's count and digest, the request sidecar the names) | `gdfl_run_route` | 400 `only_invalid{element}`; 400 `only_too_many{max: 256}` | + `an_invalid_or_oversized_filter_is_refused` |
| R14 | (round 3, replacing round 2's `POST /gdfl/repair`) `POST /gdfl/import` with `repair_reason=` that is empty, over 200 bytes or holds a control character (round 6: the rule `POST /pull/repair` applies); or a repair reason over a range with no hole | `gdfl_run_route`, `gdfl_run::plan` | 400 `repair_reason_invalid`; 200 `{"started":false,"why":"no hole to repair"}` | + `a_repair_reason_is_bounded_and_a_range_without_holes_starts_nothing` (IA-10a) |
| R15 | `POST /verify/run` with an unknown feed or mode, or `mode=refetch` | `verify_route` | 400 `feed_unknown`, `mode_unknown`; `refetch_is_a_pull_run`, naming `POST /pull/run` | + `a_verify_run_with_an_unknown_feed_or_mode_or_a_refetch_is_refused_by_name` (IA-10b; round 4, N-2) |
| R16 | `POST /verify/run feed=zerodha` with a GDFL-only field (`kinds=`, `source=`), or a range past the calendar's span | `verify_route` | 400 naming the field; 400 `calendar_unmeasured{day}` | + `a_zerodha_verify_run_refuses_gdfl_fields_and_unmeasured_days` (IA-10b; round 4, N-2) |

````

### 5.8 Section 9 (runtime edge cases)

No row of section 9 (design lines 4010-4104) names IA-10a; the design row cites none. Nothing to
copy.

### 5.9 Section 10: header, the 10.1 rows of IA-10a (all dropped routes) and the runs-page row the kept half reads beside; 10.2 header and L-IA-9 (free space, dropped)

Source: `state/design/ingest-automation.md`, lines 4105,4121 4122 4125 4127 4159 4171,4175 4177 4181,4184 4193

````text
## 10. O(1), p50/p99, and the honest limits

**Who measures what (critic: "every p50/p99/max waits for IA-13").** Each
package adds its own benches and its own `docs/06-limits.md` rows **in the same
change as the code**, because gate 12 (`ci.yml:2976`) refuses a cost claim in
a doc block that names no test, bench or `docs/04` row, and gate 11
(`ci.yml:2820`) refuses a new sort without its reason in `docs/06-limits.md`
(`:13173`). A row lands as "bench C-xx-nn; UNVERIFIED as a measurement until
run on the operator's Mac", with its bench already in the tree; the package's
own CI run gives the ratio-gate result. IA-13 then runs every bench in release
on the operator's Mac against WD_BLACK and writes the measured p50, p99 and
max beside each row. No package's merge waits for IA-13.

### 10.1 Per operation

| Operation | Bound | Proof or measurement | Bench | Package |
|---|---|---|---|---|
| Admit a press (slot, locks, id, claims) | O(1) per kind apart from the probe retry | 1 slot; ≤ 5 shared probes and, per kind's child, ≤ 2 exclusive or shared `try_lock`s, each retried ≤ 10 × 10 ms when busy; under the runs ledger lock (round 3, D-4880): one or two 128-byte runs-ledger reads and one write, one claim write and, with a filter or a repair reason, one request-sidecar write; one `F_FULLFSYNC` per write; then (round 4, BL-A) one `claimed` line out and one `go` or `release` line in per child, the api waiting ≤ 10 s for the claims **on a `spawn_blocking` thread, never a Tokio worker** (round 5, SF-7) and a child ≤ `GO_WAIT` 15 s for its answer | C-GR-01 (one to three kinds, and the release path) | IA-2 (the child), IA-10a (the api) |
| `GET /gdfl/plan.json` | O(days in range) hash and day-table probes over cached sets, plus O(new journal bytes), ≤ 5 probes, 2 census slot reads and one listing read per kind | §5.2, §5.5; on the bounded store-read pool (`run_store_read`, `detail.rs:150`) with 429/503 when full (`admission_refused`, `:181`; D-1508); fetched when a parameter changes, never polled | C-GR-05 | IA-10a |
| `GET /gdfl/import.json` | O(1) | ≤ 6 shared probes and ≤ 6 reads of 4 KiB; no lock shared with any run | C-GR-03 | IA-10a |
| (round 6) `/imports.json` runs page and `run=` | O(page): ≤ 20 `run` lines decoded, and `run=`'s own lines through an id index kept at parse time | §7.2 R6 | C-LG-02 | IA-11r |
| (round 5, B2) Alert acknowledgement | O(1) inside the newest 1,024 records; O(records after it) for an older one, from `cli alerts --ack` only (L-IA-34) | the window read, one 128-byte append, one `sync_data` | C-AL-01 | IA-11a (the writer), IA-10a (the route) |
| (round 3) Build-skew check | O(1) | one `metadata` of `current_exe()` | C-GR-10 (round 4, SF-19); IA-GR-33 pins the behaviour | IA-10a |
| (round 4, SF-19) `GET /gdfl/sources.json` | O(sources) = O(1): ≤ 4 sources | one `metadata` per configured source root to see a change; the rest from the source cache | C-GR-12 | IA-10a |
| (round 4, SF-19) `POST /gdfl/import/stop` | O(1) | `lock_state` (≤ 6 shared probes), then one stop file: write `.tmp`, `sync_all`, rename, directory fsync | C-GR-13 | IA-10a (the route), IA-2 (the stop file) |
| (round 4, SF-19) `POST /gdfl/import/dismiss` | O(1) | take the lock (`try_lock`), one claim read, truncate, `sync_all`, release; one audited event | C-GR-14 | IA-10a |
| (round 4, SF-7) `GET /alerts.json` | O(1) | **one** positional read of the newest ≤ 1,024 records (≤ 128 KiB), the same window `cli alerts` reads, and `older_unacknowledged` from the newest good record's "open after" (round 5, B2: round 4 named two windows, 64 here and 1,024 in §11) | C-AL-02 | IA-10a |
### 10.2 Limits for `docs/06-limits.md` (new section "Ingest automation, D-4800 to D-4888"; each row lands with its package, **in the same change as its code**, D-4861)

| ID | Limit | Why it is not O(1) | Bound | Package |
|---|---|---|---|---|
| L-IA-9 | No free-space refusal in this design's code | std has no `statvfs`; no dependency added. **Round 3:** the free-space and growth surface is WP-81's (DB-B73); **round 5:** its WIP measures it in the api (`GET /disk.json`, `/bin/df -Pk`, at most every 30 s, F76), and IA-10a's `plan.json` handler shows that figure beside the projected growth and warns, never refuses | `StorageFull`, typed, stops the run at the first write that meets it | IA-16; WP-81; IA-10a (the plan line) |
````

L-IA-39, the one limit row the kept half rewrites, is not in the design's 10.2 (the round-7
critic added it); it is in `docs/06-limits.md` at the head and copied in section 10.2 below.

### 5.10 Section 11: header and every file entry naming IA-10a

Source: `state/design/ingest-automation.md`, lines 4226,4235 4238 4271 4273 4283,4285 4288 4290 4294

````text
## 11. Files and types to add or change

"After" names the fix-round packages that own the file first (`fix-plan.json`
`files_owned`, or a branch's diff where the plan is silent); the IA package
edits it only after they merge (§16). A file with two IA packages lists them
in order; they are in one lane.

| Crate / path | Add or change | What | Package | After |
|---|---|---|---|---|
| `crates/pull/src/gdfl_run.rs` | **add** | IA-1: `Request`, `Mode`, `By`, `SourceWord`, `SourceId`, `Range`, `VerifyMode`, `SourceCache`, `Plan`, `PlanSummary` (with the round-3 classes: `scope_widened`, `scope_unrecorded`, the five hole reasons (round 5), `holes_unrepairable`, predicted holes, `holes_in_range`, `calendar_horizon`, `free_space`), `Observer`, `RunRefusal`, `plan`, `dry_plan`, `execute` (this process's kinds in order, no thread per kind, round 4), `resolve_source`, `open_source`; the progress-file observer. IA-2: the locks, probes, the store-wide runs ledger with its record CRC (`audit/runs.ledger`, D-4880, round 4 SF-12), claims and request sidecars, release, `lock_state`, `RunControl` and the stop file, `StartGate` and `await_go` (the two-phase start, round 4, BL-A). IA-1 also: `RunRefusal::Unstamped`, moved from WP-81's cli check (round 5, F76). IA-16: the `restate` field, `repair_reason` if WP-12d defined no trigger (round 4, SF-18), and the plan's `repair_in_open_month` warning and `holes_unrepairable` (round 5, SF-9, SF-10). **IA-7: the end-of-run deferred step, rules 1 and 2 of §6.1, and the `Spawn` hook in `RunControl`** (round 5, SF-6, SF-12: round 4 gave "the deferred pass's driver" to IA-2 here and to IA-7 in §16.2; IA-7 is its one owner, since IA-2 lands before IA-6 builds the pass) | IA-1, IA-2, IA-16, IA-7 | WP-81 (IA-1's unstamped move) |
| `crates/pull/src/gdfl_run_tests.rs` | **add** | IA-GR-01..IA-GR-39 | IA-1, IA-2, IA-16, IA-10a (its tests run the child through `gdfl_run`) | — |
| `crates/api/benches/ratio/logs_dirs.rs` (+ one line in `crates/api/benches/ratio.rs`; round 6, BG-1) | **add** | C-LG-01 (a keyed `/logs.json` request and an importer-evidence read over the 8 directories, 1x and 10x posting sizes), C-LG-02 (`/imports.json`'s runs page over 100,000 `run` lines); its own file because IA-11r lands at step 2, before IA-10a adds `ingest_automation.rs` | IA-11r | WP-33c (merged) |
| `crates/api/benches/ratio/ingest_automation.rs` (+ one line in `crates/api/benches/ratio.rs`) | **add** | C-GR-03, C-GR-05, C-VD-03; round 4 (SF-19): C-GR-10 (build skew), C-GR-12 (`/gdfl/sources.json`), C-GR-13 (stop), C-GR-14 (dismiss), C-AL-02 (`/alerts.json`), IA-10a's; C-VD-06 (`/verify/day.json?code=`), IA-10b's | IA-10a, IA-10b | — |
| `crates/api/src/main.rs` | change | the hidden `gdfl-run` mode: read the claim text on stdin, run `pull::gdfl_run` (D-4846); its streams (two stdin lines, one stdout line, stderr the kind's `stderr` file through the declared writer; round 5, SF-7) (IA-10a); the api child's `Spawn` hook for a `deferred` run (round 5, SF-6; IA-7) | IA-10a, IA-7 | — |
| `crates/api/src/gdfl_run_route.rs` | **add** | `/gdfl/sources.json`, `/gdfl/plan.json`, `POST /gdfl/import` (spawns one child per kind on v4, one per press on v3, and drives the two-phase start: `go` only when every kind claimed, else `release`, round 4, BL-A), `/gdfl/import.json`, `POST /gdfl/import/stop`, `POST /gdfl/import/dismiss`, the restart scan and auto-resume, its alerts; one `Journal` and one `SourceCache` per store in `Site` | IA-10a | — |
| `crates/api/src/alerts_route.rs` (round 4, SF-7) | **add** | `GET /alerts.json` (the unacknowledged alerts of the newest 1,024, newest first, `older_unacknowledged`, and any `damaged{ordinal}`; round 5, B2), `POST /alerts/ack` (appends an acknowledgement inside the window; 409 `ack_outside_window` naming the cli otherwise; audited) | IA-10a | IA-11a |
| `crates/api/src/server.rs` | change | register the routes in `route_table` (`:14255` at 9cfa4577); the slots in `Site`; `end_runtime` and `wait_then_end` write and wait for the children's stop files (`:16711`, `:16721`); the api's executable identity at start (D-4884); on the pull path, a durable run id from the runs ledger at the pull run's claim (`:8704`–`:8719` at d73e1112), and, round 6 (SF-1, SF-10), one per `/pull/spot`, `/pull/fno` and `/pull/repair` request from its route after its seat (`spot_pull_held`, `:8937`, seat at `:9003`; `fno_pull_held`, seat at `:12870`), a `/pull/run` leg carrying its run's id down, passed in `Plan.run` through `land_bodies_observed` (`:3329`) and claimed as the sink's run in `note_run_started` (`:4171`); `pull_repair`'s reason check (`:8892`) calls `pull::ingest::repair_reason_ok` and its private `MAX_REPAIR_REASON` (`:8915`) is removed; and keep each answer and spool its verdicts after **both** landing doors (`from_window`, `:3419`; WP-13c's `repair_window`, `:3426`) | IA-10a (GDFL), IA-10b (verify, Zerodha) | WP-36, **WP-32** (merged at f3126724; it moved every cite here, SF-4), WP-39, WP-13c, **WP-62** (pull run routes and conductor), **WP-81** (its WIP adds 135 lines here, round 5) |
| `crates/api/src/operation_audit_tests.rs` | change | each new route in `EXEMPT` with `DATA`, the reason `/pull/run` and `/verify.json` carry (`:72`, `:77`–`:80`); `AUDITED` (`operation_audit.rs:60`, `[&str; 22]`) is unchanged, so `every_registered_route_is_audited_or_exempt_by_name` (`operation_audit_tests.rs:154`–`:197`) passes | IA-10a, IA-10b | — |
| `crates/api/src/import_journal.rs` (round 6) | change | IA-11r (R6): the run-id index beside `runs` (`:226`), kept at parse time. IA-10a (SF-9, F84): a `<filter>+<member>` key parsed into its base filter and member; an `incomplete` row whose reason is one of the five hole reasons carries `hole: {reason, repairable}`; `/imports.json`'s counts list holes by reason and `holes_unrepairable` apart from `incomplete`, and `merged` (`imports.rs:792`) counts a day journalled when any key of the base filter `*` exists; test IA-JR-01 | IA-11r, IA-10a | WP-33c (merged), IA-16 (the reasons) |
````

Of these, only the `import_journal.rs` entry (round 6, F84) survives the cut. `main.rs`,
`gdfl_run_route.rs`, `alerts_route.rs`, the `server.rs` lines, `operation_audit_tests.rs`, the
`gdfl_run_tests.rs` child tests and the `ingest_automation.rs` bench are dropped.

### 5.11 Section 12: the rules every new route keeps, and the route rows this part touched

Source: `state/design/ingest-automation.md`, lines 4322,4343 4349,4355

````text
## 12. API routes

**Rules every new route keeps.** Each is in `EXEMPT` with `DATA` in
`operation_audit_tests.rs` (the class `/pull/run`, `/imports.json` and
`/verify.json` carry); none joins `AUDITED`, which lists sweep and backtest
routes. The audit trail of a press is its audited lifecycle events (§7.1). The
blocking GETs (`/gdfl/plan.json`, `/verify/days.json`, `/verify/day.json`) run
on the bounded store-read pool (`run_store_read`, `detail.rs:150`) and answer
429 or 503 when it is full (`admission_refused`, `:181`; D-1508). Imports and
verifications run in child processes (D-4846), never on that pool and never on
the api's threads. A list travels as **one** comma-separated field (`kinds=`,
`only=`), because the form and query middlewares refuse a repeated key
(`server.rs:14036`, `:14132`, `:14198` at 9cfa4577); no `MULTI_VALUED_*` entry is added.

| Method and path | Request | 2xx response | Refusals |
|---|---|---|---|
| `GET /gdfl/sources.json` | — | `{"sources":[{"id":"cm_zip","word":"zip","kinds":["indices","stocks"],"configured":true,"path":"...","bytes":128918864518,"digest":"…","span":{"from":"2018-09-03","to":"2026-09-24"},"fault":null}, {"id":"nfo_zips","word":"zip","kinds":["options"],...}, {"id":"tickstore","word":"tickstore","configured":true,"format_sha256":"d21139…","pinned":true,"pinned_by":"docs/00-charter.md:1108-1109","misfiled":[],...}, {"id":"dayzips","word":"dayzips","configured":false,"variable":"BRUTEX_GDFL_DAYZIPS"}], "v3_ledger":{"configured":false,"variable":"BRUTEX_GDFL_V3_LEDGER"}}` (from the source cache; one `metadata` per source per GET to see a change) | — |
| `GET /gdfl/plan.json?kinds=options,stocks&source=zip&from=&to=&resume=1&only=A,B&stored=1&retry_refused=1&restate=1` | query | `{"layout":"v4","kinds":[{"kind":"options","source":"nfo_zips","from":"2020-09-16","to":"2026-09-24","days":{"calendar_open":1497,"held_by_source":1497,"done":0,"restated":0,"incomplete":1,"refused":0,"scope_widened":{"days":0,"members":[]},"scope_unrecorded":0,"holes":{"source_lacks":0,"calendar":0,"needs_month_generation":0,"month_superseded":0,"repair_refused":0,"needs_generation_predicted":0},"holes_unrepairable":{"bars_differ_held":0,"repair_impossible":0},"restated":{"days":0,"cost":{"value":"…","label":"extrapolation from measured day costs"}},"to_import":1497,"repair_days":[],"repair_days_refused":[]},"warnings":[],"first_not_done":"2020-09-16","first_to_import":"2020-09-16","last_done":"2020-09-15","census":{"free_now":…,"free_after":…,"need_first_day":…,"projected_stop":null},"legacy":{"needs_ack":true,"key":"options 2020-09-16 *"},"inactive_checks":["stored_index_absent: spans not recorded (IA-0b)","lineage: WP-12f's detector not landed"],"preflight":[]}],"calendar_horizon":{"last_day":"2026-10-04","days_left":-6},"free_space":"unknown (WP-81)" (or, once WP-81 merges, its `/disk.json` measurement: `{"free_bytes":…,"measured_at":…,"projected_growth":…,"label":"extrapolation"}`),"locks":[{"lock":"gdfl.options","state":"free"}],"eta":{"value":"…","label":"extrapolation from measured day costs, over days to import and restated days"}}` | 400 naming the parameter; 429/503 when the pool is full; 200 with `preflight:[{kind,code,why}]` when a kind would be refused |
| `POST /gdfl/import` (form, ≤ 8 KiB) | `kinds=options,stocks&source=zip&from=2020-09-16&to=2026-09-24&verify=inline&workers=8&auto_resume=1&legacy_ack=1` (or `resume=1` in place of `from`); round 3: `repair_reason=<text ≤ 200 B, no control character>` makes it a repair-reason run (D-4874; 200 since round 6, the Zerodha rule), and `snapshot=<name>` records the operator's snapshot (§6.8 V-S) | 202 `{"started":true,"layout":"v4","runs":[{"kind":"stocks","run":1791234567,"source":"cm_zip","pid":5120},{"kind":"options","run":1791234568,"source":"nfo_zips","pid":5121}]}`, after every child's first stdout line said `claimed run=… kind=…` (at most 10 s) and the api then wrote `go` to each (round 4, BL-A: one child per kind on v4, each holding its own locks, stop file and log directory); a v3 store runs the kinds in sequence in one child (`"layout":"v3","sequence":true`). 200 `{"started":false,"why":"nothing to import"}` when no day is owed | 400 `{"started":false,"code":"...","why":"..."}` (R1–R3, R6–R8, R12–R14, including `repair_reason_invalid` and `only_too_many`); 412 `unstamped_write` when WP-81's check refuses an unstamped build (round 5); 409 `run_busy` per busy kind (no kind starts: every child that claimed was sent `release` and exited with no journal line); 412 preflight refusals, per kind (no kind starts, the same way), including `legacy_run_may_be_live`; 504 `child_silent` when a child writes no first line in 10 s (no kind starts: the api writes `release` to every child, the silent one included, which releases when it reads it or after `GO_WAIT`); 500 `build_skew` when the child refused because the executable changed since the api started (round 3, D-4884). The handler waits for the children's first lines on a `spawn_blocking` thread, never a Tokio worker; if the api dies between two `go` writes, the press is partial and the next page says which kinds run (round 5, SF-7, §4) |
| `GET /gdfl/import.json` | — | §5.7 | — |
| `POST /gdfl/import/stop` | `run=<id>`, `kind=<k>` or `run=all` | 200 `{"stop_written":true,"delivered":"at the run's next file boundary","held_by":"cli"}` (for any live run: import, verification, deferred, completeness, repair, Zerodha cold, `verify-merge`; the progress file then shows `stopping`) | 404 no live run with that id; 409 the lock is held by another run than named; 503 when the stop file cannot be written |
| `POST /gdfl/import/dismiss` | `lock=<gdfl\|gdfl.<kind>\|verify.zerodha>&run=<id>` | 200 `{"dismissed":true}` (the interrupted claim is truncated; audited) | 409 the lock is held; 404 the claim names another run |
| `GET /dayflags.json` (exists) | unchanged | each record gains `"verdict":{"code":"match","generation":3,"run":1791234567,"stale":null,"rule_set":"2a","flags":[],"href":"/verify?..."}` for the feed's finest rung, or `{"code":"unset"}`; ≤ 92 lookups per page; `quality` reads D-3335 v=1 and v=2 lines | — |
| `GET /alerts.json` (round 4, SF-7; round 5, B2) | — | `{"alerts":[{"ordinal":41,"kind":"hole","run":1791234567,"day":"2021-02-24","text":"…","href":"/ingest?run=…","put_at":…}],"older_unacknowledged":0,"older":"cli alerts --older","damaged":[],"window":1024}`: the unacknowledged alerts among the newest 1,024 records (IA-11a's queue, the same window `cli alerts` reads), and the exact count of older unacknowledged alerts from the newest record's "open after", O(1) | 429/503 |
| `POST /alerts/ack` (round 4) | `ordinal=<n>` | 200 `{"acknowledged":41}`; audited | 404 no such alert; 409 already acknowledged; 409 `ack_outside_window` for an alert older than the newest 1,024 records, naming `cli alerts --ack <n>` (round 5, L-IA-34) |
| `GET /logs.json`, `GET /imports.json` (exist; WP-33c's, merged) | WP-33c's keys (§7.2); round 6: `runs_before=<id>` on `/imports.json` (R6) | WP-33c's answers, with every importer-evidence field read from the run's own directory among the 8 and an `evidence` block naming each directory read (`absent`, `unreadable` or its files and bytes), never `"none"` for a directory not read (IA-11r, R5); `/imports.json`'s `runs` a page of ≤ 20 with `runs_total` (R6); member keys, holes by reason and `holes_unrepairable` named (IA-10a, SF-9) | WP-33c's |

Every POST is POST only, never GET (the D-0128 reasoning: a link preview must
never start work).
````

The kept half adds **no route**. It changes the answers of existing routes: `/imports.json`
(row 4352's last clause: member keys, holes by reason, `holes_unrepairable`), `/dayflags.json`,
`/provenance.json`, `/audit.json?feed=gdfl` (through `imports::summary`) and `/feeds.json`'s open-day
scan. Every one of them stays GET-only and is unchanged for a journal with no member key and no
hole.

### 5.12 Section 13 (web changes)

The design gives IA-10a no web file. Its routes' only web caller was IA-12a's
`web/src/routes/ingest/GdflImportPanel.svelte` (design :4363), dropped by D-6900. The kept half's
answers are read by existing web code that needs no change if the new fields are additive:
`web/src/routes/db/DbDayFlags.svelte` (the day band, fetching `/dayflags.json` at `:155-175`) and
`web/src/lib/db/notes-labels.js` (`provenanceUrl`, `:930-938`). If a web change turns out to be
needed to show a member hole, it is outside ownership: keep it minimal and name it.

### 5.13 Section 14: header and every test row this part's 16.3 row names

Source: `state/design/ingest-automation.md`, lines 4381,4397 4398,4399 4403,4404 4410 4419 4421 4423 4427 4431,4432 4531

````text
## 14. Tests that fail if it breaks (→ `docs/04-invariants.md`)

**Every new invariant id carries the prefix `IA-`.** `docs/04-invariants.md`
already uses `GR-01`..`GR-06` for grain and report invariants (`:3196`–`:3201`)
and `AU-01`, `AU-01b`.. for autopilot ones (`:2249`–`:2256`); at 10babda7 a
grep finds no row starting `IA-`. **Each row lands with the package that owns
its test, in the same change** (gate 10, `ci.yml:2656`: every row names a test
that exists; gate 27, `:2463`: every id names one invariant), and a code doc
block that claims a cost cites its row or its bench (gate 12). Round 1's
wildcard row `IA-MX-*` is withdrawn: gate 27's id grammar (`id_grammar`,
`.github/gates_ledger.rs:458`) does not read `IA-MX-` as an id, and gate 10
needs one named test per row. **Every `+` test of the matrix (§8) gets its own
row `IA-MX-<row>`** (for example `IA-MX-A6`, `IA-MX-T3`), added by the package
named in §8's legend for that section.

| ID | Invariant | Test | Package |
|---|---|---|---|
| IA-GR-01 | The api child and the cli run one plan and leave byte-identical stores and journals, except run ids, `started=`, `pid=` and `host=` in claims and the runs ledger, the image's "written at", and `.prv`'s `landed_at`; a press over days all done starts no run and writes nothing (D-2803) | `the_api_and_the_cli_run_the_same_plan_and_leave_the_same_store` | IA-10a |
| IA-GR-02 | A second press on a held lock is refused, naming the holder; a press for several kinds with one busy starts none: the kinds whose children claimed are sent `release`, read no day, write no journal line, leave their lock files empty and their ids unused in the runs ledger; a child that hears nothing for `GO_WAIT` releases the same way (round 4, BL-A); **an api killed between two `go` writes** leaves the kinds that got `go` running and the others released with no line, and the next page names the partial press (round 5, SF-7) | `a_second_press_on_a_held_lock_is_refused_naming_the_holder`; `a_press_with_one_kind_busy_releases_every_claimed_kind`; `an_api_dying_between_go_writes_leaves_a_named_partial_press` | IA-2 (the child), IA-10a (the api) |
| IA-GR-06 | A claim in a free lock file is interrupted; at most one per lock file; a new claim supersedes it; auto-resume only for an api import that asked, not stopped, and at most once per stuck day | `a_dead_runs_claim_is_interrupted_and_resumes_at_most_once_per_day` | IA-10a |
| IA-GR-06b | A shutdown during an import leaves `outcome=shutdown`, no claim, and no auto-resume | `a_shutdown_releases_the_run_and_never_auto_resumes` | IA-10a |
| IA-GR-12 | A live cli run is reported held, never interrupted, and no auto-resume fires into it | `a_live_cli_run_is_held_not_interrupted` | IA-10a |
| IA-GR-21 | A cli claim always says `auto_resume=0`; `cli gdfl-stop` stops a run at its next file boundary | `a_cli_run_is_never_auto_resumed_and_can_be_stopped_by_name` | IA-2 (claims and stop, round 4, SF-11), IA-10a (no auto-resume) |
| IA-GR-23 | A panic in a child run leaves the api serving and the claim interrupted with the exit status | `a_panicking_import_never_takes_the_server_down` | IA-10a |
| IA-GR-25 | A press refused at preflight or busy leaves an audited event | `a_press_that_never_started_is_on_record` | IA-10a |
| IA-GR-29 | A child run never holds `serve.lock`: the api restarts while it runs; **and the api exits right after `go`**, after which the child writes a stderr line (to the kind's `stderr` file, through the declared writer) and finishes its run, never writing stdout again and never aborting (round 5, SF-7, F77) | `a_child_run_never_holds_the_serve_lock`; `a_child_outlives_the_api_without_a_broken_pipe` | IA-10a |
| IA-GR-33 | (round 3) A child whose executable differs from the api's at start refuses `build_skew` before any lock | `a_child_from_another_build_refuses_before_any_lock` | IA-10a |
| IA-GR-34 | (round 3) The plan warns within 30 days of the calendar's horizon and past it, and one alert a day is put | `the_calendar_horizon_is_warned_before_it_passes` | IA-3, IA-10a |
| IA-JR-01 | The api's journal reader names a member key's base filter and member, shows each of the five hole reasons as a hole (repairable or not) and not as a plain `incomplete`, counts `holes_unrepairable` apart, and `/imports.json` counts a day whose only keys are member keys as journalled | `the_api_journal_reader_names_member_keys_and_holes` | IA-10a |
````

Kept: **IA-JR-01** (4531) and **IA-GR-34**'s receipt half, reworded by section 3.2 to
`the_calendar_horizon_is_warned_on_the_receipt_before_it_passes` with an injected today (the
plan and alert halves are gone with `plan.json` and the alert). All other rows above test the
dropped child, press, shutdown or route and are not built.

### 5.14 Section 15: the numbering check, the decision rows this part landed or touches, and "Lands with"

Source: `state/design/ingest-automation.md`, lines 4539,4540 4563,4569 4575 4587 4591 4610 4616 4618 4653,4654 4660,4663 4668 4676

````text
## 15. Decisions (proposed numbers)

WP-99. Re-run
the grep at landing, because another lane may claim the block first; gate 27b
(`ci.yml:2489`) refuses a number that heads two entries. None of these is
minted yet, so round 2 amends round 1's proposals in place (marked "round 2").

| D | Decision |
|---|---|
| D-4805 | A lock file holding a claim while its lock is free is an **interrupted** run (`lock_state`, a shared probe released at once; a held lock is a live run, never interrupted). At most one per lock file; the next claim supersedes it and an audited event records the old claim. Round 2: auto-resume follows D-4848. **The auto-resume box is ticked by default for page presses** (§17 Q1). |
| D-4817 | Near-miss names are named, not just counted. The skip census is bounded at 16 named per reason per day. |
| D-4821 | The ETA is always labelled an extrapolation from measured day costs, and counts the days to import and the restated days it will read. The plan shows `first_not_done` beside `first_to_import` and counts each class of hole a re-run cannot fill. |
| D-4840 | Every new api route is an `EXEMPT` `DATA` route; `AUDITED` is unchanged. The blocking GETs run on the bounded store-read pool and answer 429/503 when it is full (D-1508); imports and verifications run in child processes (D-4846). |
| D-4846 | **Every GDFL run is a process.** The api runs each import, verification or repair in a child of its own executable, in its own process group, with the request as one claim line on stdin; the child takes the locks. **Round 4 (BL-A):** on a v4 store one child per kind, each with a one-kind request, its own locks, stop file, progress file and log directory; on a v3 store one child per press, running its kinds in sequence; no process runs two kinds at once (one telemetry sink per process, `telemetry/src/lib.rs:173`–`:176`). Each child starts in two phases: it claims (locks, id, claim) and prints `claimed run=<id> kind=<k>` (or `busy`, `refused`, `nothing`), then waits for `go` or `release` on stdin; the api sends `go` only when every kind of the press has claimed, else `release`; on `release`, EOF or `GO_WAIT` = 15 s the child truncates its claim, removes its request sidecar, releases its flocks and exits `NOTHING` with no journal line. A panic ends the child only. Every run writes a 4 KiB progress file (≤ 1 write a second) and watches its stop file; the page reads and stops any run the same way. On the server's shutdown, `end_runtime` writes each live child's stop file (`by=shutdown`) and waits for it within the grace; the child releases `outcome=shutdown` at its next file boundary, even after the server exits. A shutdown never auto-resumes. **Round 5 (SF-7):** the child's stdin carries two lines and its stdout one, after which neither is used; its stderr is the kind's `logs/gdfl/<kind>/stderr` file, opened by the api before the spawn, never a pipe; every run writes stderr through one declared writer that ignores its own failure, never `eprintln!` (Rust ignores `SIGPIPE`, so a print to a closed pipe panics, and a release build aborts); the api waits for the first lines on a `spawn_blocking` thread, never a Tokio worker; and an api that dies between two `go` writes leaves a **partial press**, named on the next page, with the released kinds having written nothing. Rejected: running imports on the api's threads (a panic aborts the server, `Cargo.toml:194`), and a pipe from child to parent for events (a parent that dies would lose them). |
| D-4848 | **Run modes and their lifecycle.** A claim carries `mode=` (`import`, `verify`, `deferred`, `completeness`, `census-migrate`, `census-recount`, `dayflags`; round 3: `repair` is withdrawn with IA-15, a repair being an import with `repair=`; the runs ledger also records the Zerodha modes `pull`, `zerodha-verify` and `verify-merge`). Auto-resume happens only for `mode=import`, `by=api`, `auto_resume=1`, no stop file naming the run, and not when the run was itself a resume that died on its resume day (at most once per stuck day); then once per lock file per api start. A cli claim always says `auto_resume=0`: a cli Ctrl-C is an interruption, resumed only by a press. An interrupted verification is offered to run again, never resumed as an import; an interrupted census verb shows its own recovery. "Run interrupted" and "run resumed" are audited lifecycle events. |
| D-4883 | **(round 3) Alerts** (critic SF-1, DB-B78; made concrete in round 4, SF-7; **its format and readers rewritten in round 5, B2**). A run interrupted twice on one day, `store_lost` once the volume returns, a build skew, the calendar horizon, `disk_full`, a new store hole, `store_damaged`, `runs_ledger_damaged`, a run with mismatch days, a damaged spool and a refused day image each put one alert into **one** durable queue, `<store>/audit/alerts`, **keyed per run (or per store-wide fact), never per day or per path**, so one run puts at most 22 alerts (12 on v4); the `kind` numbers 1–12 are append-only (§7.1; round 6 adds 12, `queue_replaced`). **Format v1** (`docs/02` §38; IA-11a; every offset in §7.1): a 64-byte header (`BRUTEXAL`, version, stride 128, header length, created, CRC-32C at 60), born whole by tmp and hard link; 128-byte records (type, kind, day, run, put-at, a 16-byte key hash, the acknowledged ordinal, **"open after"**, a 75-byte text, CRC-32C at 124). **The put** is O(1): under `audit/alerts.lock`, one read of the newest ≤ 1,024 records; a length-based tail rule checks only the last whole record (a torn append or an unsynced last record is written over, with an error event naming the offset; two bad records refuse `alerts_damaged`; a record whose CRC passes is never written over); de-duplication against that window by key hash (a key older than the window is put again: a repeat, never a loss, L-IA-30); one append and `sync_data`. **The readers** (`GET /alerts.json`, `cli alerts`) read the same one window under `audit/alerts.lock` held shared (round 6, SF-8: never a half-written record), a text over 75 bytes is cut at a character boundary and flagged `text_cut`, and a queue whose tail is damaged is replaced by the operator's `cli alerts --replace-damaged`, which keeps the damaged file and starts a new queue whose first alert (kind 12) names its open count (IA-14's runbook step). The readers list the window's unacknowledged alerts and name damaged records, and show `older_unacknowledged` = the newest record's "open after" less the alerts listed, exact and O(1), so no unacknowledged alert leaves view; `cli alerts --older` lists those (L-IA-34). An acknowledgement inside the window is O(1); an older one is the cli's. **IA-11a builds the writer, `cli alerts` and the format at step 2, before every caller**, unless WP-61 or WP-81 merged a Rust queue first (WP-81's WIP ce189751 has none); each caller adds its `alerts::put` in its own change (IA-2, IA-7, IA-9, IA-10a, IA-16); IA-10a serves `GET /alerts.json` and `POST /alerts/ack`; WP-61 keeps the finding and its page, and delivery by notification or webhook is its choice. |
| D-4884 | **(round 3) Build skew and the calendar horizon** (critic SF-3, DB-B25, DB-B41, DB-B71, DB-B72). An api child compares its executable's (device, inode, mtime, length) with the api's at start, passed as `exe=` in the claim, and refuses `build_skew` before any lock when they differ (and the commits too, once WP-81 stamps the api). `plan.json` and `/ingest` warn from 30 days before the calendar's last sourced day, and one alert a day is put from then until the record is extended; extending it is WP-62's for 2026 and a yearly sourced human step after (§6.9). |
**Lands with (round 3, critic BL-8: IA-0 is dissolved).** Round 2 had one package, IA-0, mint all of D-4800..D-4873 before any code, which would have recorded texts that were false at the head (D-4804, D-4864, D-4873) or contradicted by WP-13c (D-4851, D-4857). Now **each entry lands with the first package that implements it**, re-checked against the head that package starts from (§16.3, Re-grounding), and an entry whose substance a fix-round package has since decided is amended before it lands, never minted false. An entry proposed to a fix-round package (D-4874 to WP-12d, D-4875 to WP-13c or WP-12d, D-4878 to WP-12f) is recorded by that package under the number it takes at commit if it builds it; this design's number is then retired by name in the IA package that relies on it, never minted twice. The IA-5 subset is the one that can land now:

| Package | Lands |
|---|---|
| IA-3 | D-4802, D-4814, D-4815, D-4817, D-4818 (round 4, SF-10: its arms change with the per-day refusal), D-4819, D-4828, D-4834, D-4835, D-4850, D-4864, D-4866, D-4867 |
| IA-10a | D-4805, D-4821, D-4840, D-4846, D-4848, D-4884 |
````

All six decisions of IA-10a's "Lands with" row are disposed of by D-6900 at the head (retired:
D-4821, D-4840, D-4846, D-4884; moved: D-4805 to R3-23, D-4848's surviving clauses to IA-6). Do
not head any of them. D-4817 (IA-3's, headed and merged at `docs/05-decisions.md:77751`) and
D-4883 (IA-11a's, headed and merged, amended in D-6900) are copied because item 7a and the cut
cite them.

### 5.15 Section 16.1: the ownership rows that name IA-10a

Source: `state/design/ingest-automation.md`, lines 4733,4735 4747,4749 4751 4762,4764 4777 4781 4791 4794,4795 4798

````text

| WP package (owner) | Its goal, in a line | What this design does about it |
|---|---|---|
| **WP-81** (round 3) ops safety (build stamp, rollback, census reader guards, the store scrub, sidecar faults, backups, free-space and growth surface) | B51, B58, B59, B65, B68, B70–B75, B79 | **Owns backups, the free-space and growth surface and the scrub** (critic BL-1(f), SF-2, N-2). L-IA-9 reads its free-space figure; IA-14's V-S snapshot step is its runbook's; its scrub reads `.vdt` trailers through IA-5's `verdict::scrub` (round 6, SF-7: the WIP's name for round 5's `check_trailer`); its build stamp extends D-4884's skew check. Requirement on it: list `verify/`, `responses/`, `bar-revisions-v1/`, `bar-generations-v1/`, `audit/alerts` and the census epochs in its growth surface. Its title names "alerts" (round 4, F69): if it builds a Rust queue before IA-11a lands, that is the one queue and IA-11a closes by name. **Round 5 (F76), its WIP ce189751:** free space is measured in the api (`crates/api/src/disk_route.rs`, `GET /disk.json`, `/bin/df -Pk`), so IA-10a's `plan.json` reads that measurement (no `pull` call); its runbook (`docs/36-ops-runbook.md` §4) names the snapshot step `tmutil localsnapshot /Volumes/WD_BLACK`, which makes V-S concrete; it adds an unstamped-write refusal to `cli/src/gdfl_import_command.rs` (IA-1 moves it into `gdfl_run` and so waits for WP-81) and `census-migrate --seal` to `census_store.rs` (IA-2's lock table covers it, and IA-2 edits the file after it); it has no alert queue, so IA-11a stands |
| **WP-36** (merged at ab2810b8, round 4) calendar cold start (`calendar_route.rs`, `dayflags.rs`, `server.rs`) | first `/calendar.json` (and `/dayflags.json`) fast after a start | Merged: IA-10a's and IA-10b's WP-36 dependency is met; they still edit `server.rs` and `dayflags.rs` after WP-39 and WP-33c |
| **WP-32** (merged at f3126724, round 5) option analytics API (`crates/api`, `crates/pull/src/pricing.rs`) | month guard, as-of spot, point-in-time chain, greeks flags | Merged. It moved every `server.rs` line this design cites by 49 or 50 (critic SF-4; §16.3's round-5 table), and it touches the file IA-10a and IA-10b edit, so they re-ground at the head they start from. No rule of this design overlaps it |
| **WP-39** bars sort index, admission, window telemetry (`bars.rs`, `bars_route.rs`, `server.rs` bar handlers, a store index step, a cli verb) | sorted pages without a full read | IA-10a and IA-10b edit `server.rs` after it; the serve-path sample calls its window handler; IA-16's one `file.rs` line waits for its store change. **Round 5 (SF-2, F75), its WIP b0adcb88:** it adds `StoreError::SortIndex` (`file.rs:729`), returned only by its rebuild door and mapped to `StoreDamaged` in §5.8 (IA-GR-39); its writer appends a `.srt` run after every commit, and a month without one is read around, never refused (`docs/02-store-format.md:806`, `:813`–`:814` on its branch), so a repaired month written through the writer gets its `.srt` with nothing from this design (round 4 asked D-4875 (5) to write it). Its D-3349 (the sort index) no longer collides with WP-13c, whose second pass took D-3350..D-3352 |

| Rule or file | Owner | IA relation |
|---|---|---|
| The alert queue: writer, format v1 with its offsets and kind table, the length-based tail rule, "open after" and `older_unacknowledged`, `cli alerts [--ack] [--older]` (`pull/src/alerts.rs`, `docs/02` §38) | **IA-11a** (round 4, SF-7; round 5, B2), unless WP-61 or WP-81 merged a Rust queue first (WP-81's WIP has none) | WP-61 keeps the B78 finding and its page; IA-2, IA-7, IA-9, IA-10a and IA-16 add their own `alerts::put` calls with the per-run keys of §7.1; IA-10a serves `/alerts.json` |
| The free-space figure | WP-81 (measured in the api, `/disk.json`) | IA-10a shows it in `plan.json` (round 5, F76); `pull` never calls it |
| Audit and log search; `sink.rs`, `logs.rs`, `imports.rs`, `dayflags.rs`, `verify.rs` | WP-33c (merged at f350c156) | IA-11 replaced by WP-33c; **IA-11r: R3, the widened R5 and R6's O(1) half** (round 6, BG-1); IA-1's directories after IA-11r; IA-10a and IA-10b after |
| (round 6) `/imports.json`'s runs page and the run-id index (`imports.rs`, `import_journal.rs`) | **IA-11r** (D-4813) | IA-10a edits the same two files after it |
| (round 6) Member keys, the five hole reasons and `holes_unrepairable` in the api's journal reader and `/imports.json` (`import_journal.rs`, `imports.rs`) | **IA-10a** (SF-9) | after IA-11r and IA-16 |
| (round 6) The repair-reason rule, one for both feeds (≤ 200 bytes, no control character; `pull::ingest::repair_reason_ok`, `MAX_REPAIR_REASON`) | **IA-16** (D-4874) | IA-1 and IA-10a call it for GDFL; IA-10b makes `POST /pull/repair` call it |
````

### 5.16 Section 18 (the critic dispositions that created the kept half)

Source: `state/design/ingest-automation.md`, lines 5724 5779,5780 5787 5871 5935,5936 5945

````text
### Round 5 (10 Oct)
| Item | Verdict | Where fixed |
|---|---|---|
| SF-7 the api child's stdio | upheld; stderr is a file, stdout one line, the wait on `spawn_blocking`, the partial press named | F77; §4 "The child's standard streams"; §5.6 shutdown; §7.1 directory note; §10.1 admit row; §11 `api/src/main.rs`, `cli/src/main.rs`; §12 `POST /gdfl/import`; D-4846, D-4862; IA-GR-02, IA-GR-29; §16.2 IA-1, IA-10a; §16.3 IA-10a (gate 23) |
### Round 6 (10 Oct; the workflow's "Round 4 (10 Oct)" revision)
| Item | Verdict | Where fixed |
|---|---|---|
| SF-9 (the api's journal reader has no owner for member keys and holes) | upheld; IA-10a, after IA-11r and IA-16, with test IA-JR-01 | F84; §11 `import_journal.rs` row; §12 `/imports.json`; §14 IA-JR-01; §16.1 ownership map; §16.2 IA-10a; §16.3 IA-10a |
````

## 6. The round-7 critic (verbatim)

Source file on the Mac: `state/design/critic-round7-20261011.md` (585 lines). `specStep` says:
"its resolutions are your defaults where the design is silent or contradicts itself". Copied:
the verdict table, the cross-package header and X-5 (cited by IA-10a-3), IA-11r-1 (the source of
D-4896), the whole IA-10a section, and the decision rows that land with IA-10a. Of the IA-10a
gaps, **only IA-10a-3's D-4896 clause survives the cut**; IA-10a-1 (D-4913), IA-10a-2 (D-4914),
IA-10a-4 and D-4915 concern only dropped routes and presses.

Source: `state/design/critic-round7-20261011.md`, lines 1 52,90 92,95 100 153,163 164 283,306 549,552 560 577,579

````text
# Design critic, round 7: `ingest-automation.md` (6,154 lines, after round 6 of 10 Oct)
## Verdict

**The design is not ready as written, for one reason above all: two branches
it leans on were built after it was saved, and both decided things the design
only proposed.** WP-12d repairs without being asked and raised the row ceiling
itself; WP-81 moved the unstamped-write check; IA-5 and IA-11r took names and
limits the design does not have. None of these needs a new design round. Each
has one resolution a builder can take today, given below with its decision
number.

| Package | Blocking gaps | Ready to build today? |
|---|---|---|
| (all) | 7: X-1 to X-7 (shared rules; X-7 is in the IA-1 section) | — |
| IA-5 | 2 | yes: the branch is built; one name to read the other way, then rebase (17 behind) and merge first |
| IA-11a | 4 | yes, with the four resolutions |
| IA-11r | 3 | yes: finish the WIP with the three resolutions |
| IA-17 | 3 | **only after WP-12d merges**; its part (1) is withdrawn |
| IA-1 | 5 | yes, on WP-16, WP-12d, WP-81 and IA-11r, without waiting for WP-11c or WP-12f (X-7) |
| IA-2 | 3 | after IA-1 and IA-11a |
| IA-3 | 2 | after IA-2 |
| IA-16 | 3 (and X-1, X-2) | after IA-3 and WP-12d |
| IA-10a | 3 | after IA-16 |
| IA-12a | 2 | after IA-10a, WP-61 and WP-81 |
| IA-4 | 1 | after IA-16 |
| IA-0b | 2 | after IA-3 and IA-16 |
| IA-6 | 4 | after IA-16, IA-4, IA-5 |
| IA-7 | 3 | after IA-6 and IA-10a |
| IA-10b | 3 | its GDFL half after IA-7; its Zerodha half is a new package, IA-10c, after IA-9 |
| IA-12b | 1 | after IA-10b, WP-61 and WP-81 |
| IA-8 | 2 | yes, today: it is `pull` only once it stops changing `window_async` |
| IA-9 | 2 | after IA-8, IA-5, IA-2, IA-11a |
| IA-6p | 0 | after IA-6 and IA-7 |
| IA-13 | 1 | as each package lands |

56 blocking gaps in all. Two new packages come out of the resolutions:
**IA-10c** (the Zerodha half of IA-10b) and **IA-18** (the restate door the
operator's answer 10 needs).

---
## Cross-package gaps (they bind every package; cited below as X-n)

| Gap | Where | Resolution | Decision |
|---|---|---|---|
| **X-5. Three events the design calls audited are not, or have no name.** `is_audited` audits Error, or Info and above whose target's last segment is `lifecycle` (or the access segment) or whose message holds one of six words (`telemetry/src/sink.rs:2156`–`:2169`, words at `:2053`). (a) §7.1's "alert queue replaced | warn | `pull.alerts` | yes" is **false**: a warn at that target with that message is not audited (the F53 defect again). (b) `POST /gdfl/import/dismiss` and `POST /alerts/ack` are said to be "audited" (§5.6, §12) and §7.1 has no event row for either. (c) A stop request has no event of its own, and its stop file, which alone says who asked (`by=`), is removed at release | design §7.1 lines 3349–3379; §5.6 line 1703; §12 lines 4342–4351 | (a) target `pull.alerts.lifecycle` (IA-11a). (b) two rows: "run dismissed", warn, `gdfl.import.lifecycle`, fields run, kind, lock, by, old claim (IA-10a); "alert acknowledged", info, `pull.alerts.lifecycle`, fields ordinal, kind, run, by (`cli` or `api`) (IA-11a, emitted by `alerts::ack`, so the cli and the route both log it). (c) "stop requested", info, `gdfl.import.lifecycle`, fields run, kind, by, requested (emitted by whoever writes the stop file: IA-2's `cli gdfl-stop`, IA-10a's route and shutdown), and the run's "run stopped" event gains `stop_by` and `requested` read from the stop file before it is removed (IA-2). A test in each package asserts `telemetry::is_audited(level, target, message)` for every event it adds: a table test, not a claim | **D-4893** (lands with IA-11a; IA-2 and IA-10a cite it) |
## IA-11r — R3, the widened R5, R6 (branch `fix/db-IA-11r` 21f3f8d2, WIP)

Checked on the branch: `audit.idx.gone` and `telemetry-reaper`
(`sink.rs:2051`, `:2056`), `gone_waiting`, `reap_failures`, `reaper_behind`
(`:2315`–`:2347`); gate 21's list changed as §7.2 says (`create_dir_all` 5,
`read_dir` 1, `rename` 3, `remove_dir_all` 1); the six tests of IA-LG-01..03
exist by name; `docs/04` rows IA-LG-01..03, `docs/06` L-IA-13, -36, -37; D-4813,
D-4838, D-4862 headed. Its diff touches only the files §16.2 gives it.

| Gap | Where | Resolution | Decision |
|---|---|---|---|
| **IA-11r-1. One reader still decodes every `run` line on every request, against R6 and §7.2's "the third caller … takes the same page".** The branch's D-4813 says so: "`/dayflags.json`'s day band resolves a refusal's run by time over the whole list, so the crate-internal caller (`imports::RunsAsked::Every`) still decodes every run: O(runs)". `/dayflags.json` is what `/db` draws its day band from, so the operator's most-used page keeps an unbounded term | design §7.2 lines 3692–3703; `fix/db-IA-11r:docs/05-decisions.md:66516`–`:66548` | IA-11r is right that a page of 20 would drop runs the band needs, so the design's sentence is withdrawn. The fix is to stop resolving by time: from IA-16 on, every `refused` event carries its `run` (§7.1's field list), so **IA-10a** (which edits `imports.rs` after IA-11r and IA-16) resolves the band's run from the event's own field through the run-id index, O(1), and keeps `RunsAsked::Every` only for events with no `run` field, read at most once per api process into a cached map and never per request. Until IA-10a lands the O(runs) decode is IA-11r's named limit, **L-IA-39** (its own row, not a sentence inside L-IA-13), with bench C-LG-02 run at 100,000 `run` lines for this caller too (p50, p99, max) | **D-4896** (lands with IA-10a; IA-11r lands L-IA-39 and cites it) |
## IA-10a — the api's import half

| Gap | Where | Resolution | Decision |
|---|---|---|---|
| **IA-10a-1. The worker bound is per process, and a press starts up to three processes.** WP-16: `Workers::MAX` = 12 "on the operator's 14 cores, so two stay free for the api", default 8 (`gdfl_import.rs:1406`–`:1420` on the branch). On v4 a press for three kinds starts three children; with the default each takes 8 workers, 24 threads on 14 cores, and §5.5's own memory sum is "about 4.9 GB plus the census images" at W = 12 and three kinds. §8 R7 bounds `workers` only per request, and `plan.json` (§12) carries no bound at all, though the page "offers 1 to WP-16's bound" (§13), which would be a number kept by hand in `web/` | design §5.1 line 786; §5.5 lines 1312–1313, 1369–1371; §8 R7; §12 line 4339; §13 line 4363 | The bound is per **press**: each child of a press of k kinds gets `min(asked, max(1, Workers::MAX / k))` workers (4 each for three kinds; 12 for one), and a press that asks for more is not refused but told: the 202 answer and each claim carry the number each child got. `plan.json` gains `"workers":{"max":12,"default":8,"per_kind":<n>}` from `Workers`' own constants, and the page reads them. A cli run beside api runs is not counted against the bound; named in L-IA-12. R7's test gains the three-kind case | **D-4913** (amends D-4806; lands with IA-10a) |
| **IA-10a-2. The source cache is filled "once at start", on the api's start path, with work the design itself calls unmeasured.** §5.2: "The api reads, once at start, …" and the cache holds each source's `Origin` and day table: the CM archive's directory ("UNVERIFIED as a measured time", C-GI-05), the tick store's `Origin` walk (18,400 entries today, cap 1,000,000), the `dayzips` walk (cap 100,000), each year zip's tail. The same design keeps the spool merge off the start path for that reason (N-5), and the operator's app is restarted after every merge | design §5.1 line 849; §5.2 lines 916–923, 945–950; §6.3 lines 2533–2536 | The cache is filled **on first use, on the bounded store-read pool, never on the start path**. `/gdfl/sources.json` answers each source `state: "measured"`, `"measuring"` or `"fault"` (with `measured_at`); `plan.json` and `POST /gdfl/import` answer 503 `source_measuring{source}` with `Retry-After` while a source they need is being measured, never a stale or empty span. C-GR-12 gains the first measurement of each source on WD_BLACK (p50, p99, max, n, cold and warm). Test `the_api_serves_before_any_source_is_measured` (`IA-MX-R17`) | **D-4914** (amends D-4802; lands with IA-10a, the cache type with IA-1) |
| **IA-10a-3. Events and the resume.** "run dismissed" and "stop requested" (X-5); auto-resume only on a verified sidecar (IA-2-3); the request line (IA-1-2); the `/dayflags.json` day band's run from the event's own field (IA-11r-1); the unstamped check called from the child (X-6) | as cited | as cited | D-4893, D-4907, D-4902, D-4896, D-4894 |
| **IA-10a-4. Garbage parameters with no stated answer and no test id.** §8 R covers a path, a repeated field, an unknown kind or source, `from` after `to`, `workers` out of range, a bad `only=`. Not covered: a kind named twice in the one field (`kinds=options,options`); a field the route does not know; a body over the 8 KiB the route allows; `from` or `to` that is not a date; `snapshot=` outside its alphabet; `verify=` or `auto_resume=` with another word; `POST /gdfl/import/stop` with neither `run=` nor `kind=`; `POST /alerts/ack` with `ordinal=0` or text | design §8 R1–R16; §12 | each is 400 naming the field (`kind_repeated`, `field_unknown{name}`, `field_invalid{name}`), 413 for the body, before any child is spawned; one table test, `garbage_press_parameters_are_refused_by_name` (`IA-MX-R18`), over every route IA-10a adds | none new (in D-4801's entry, which IA-1 lands) |

Non-blocking. (1) **A redeploy stops every import.** The api's shutdown runs
from `ctrl_c()` only, writes each child's stop file, and "a shutdown never
auto-resumes" (§5.6). The main session redeploys `:8080` after every merge. If
it stops the api with Ctrl-C, the operator's backfill stops at each redeploy
and waits for a press; if it ends the api any other way, the children run on
(they are built to outlive it) and the new api shows them from their claims.
IA-14's runbook must say which, and `/gdfl/import.json` should mark a run whose
claim's `exe=` is not the serving api's ("started by another build"). (2) The
api must `wait()` each child it started, on a plain thread, or dead children
stay as zombies until it exits; at most 6 at once (the lock files). (3) The
horizon passed on 4 Oct (`SOURCED_LAST_DAY = 20_730`, `calendar.rs:535` at the
head), so the horizon alert fires daily from IA-10a's first start; that is one
record a day and is as designed.

---
### Decisions proposed by this review (D-4888 is the design's last; re-grep at landing)

| D | What | Lands with |
|---|---|---|
| D-4896 | The day band's run comes from the event's own field; the O(runs) decode is L-IA-39 until then | IA-10a |
| D-4913 | The worker bound is per press; `plan.json` carries it | IA-10a |
| D-4914 | The source cache is filled on first use, never on the start path | IA-10a |
| D-4915 | `/gdfl/import.json` carries `alerts_open` | IA-10a |
````

## 7. The fix-plan entry and the confirmed findings

**Fix-plan entry.** `jq '[.work_packages[] | select((.name | split(" ")[0]) == "IA-10a")]'
state/board/fix-plan.json` returns `[]`: there is **no** fix-plan work package for IA-10a (the
design is its specification). Nothing to copy.

**Confirmed findings** (`state/board/audit-result.json`, key `confirmed`). The round-3 spec
(section 3.1) names four, and ends "ANSWERS: DB-A41; DB-A34". No confirmed finding carries a
package field naming IA-10a, and the design row names no F1..F18 gap (its gaps are F33, F76, F77
and F84 of section 5.4). After the cut: **DB-A34 stays** (item 6 and the docs/06 row); **DB-A41's
route half, DB-A48 and DB-A50 are dropped** from this part (section 3.2: R3-23 answers DB-A50,
R3-24 marks DB-A48 on `/logs.json`, and `imports.rs` `run_json` plus `/logs.json?run=` already
find a run's lines). Full text, verbatim (one word in DB-A48's evidence is withheld by the
operator's language rule):

Source: `state/board/audit-result.json`, `.confirmed[]` where `.id` is DB-A34, DB-A41, DB-A48 or DB-A50 (all fields)

````text
DB-A34 (package WP-33c; severity should-fix; area api /dayflags.json, /imports.json; selection o1-static)
  observed: /dayflags.json and /imports.json (WP-33a/33b) costs are not in 06-limits; dayflags walks the 1.08 GB pull journal on the first Zerodha spot request. read_pull_journal holds the global pull_tails mutex while reading the whole journal on the first read in a process. /imports.json reads its whole journal on the first read and walks the asked range on each request. Neither is reached from a page yet: DbDayFlags is a placeholder and no /imports.json caller exists.
  expected: Named in 06-limits before WP-54 wires the client.
  evidence: dayflags.rs:46-68 (module doc defers to WP-99), 1786-1850 (mutex 1818, make_mut 1828); imports.rs:155, 201; web/src/routes/db/DbDayFlags.svelte:1-12 placeholder. | Reproduced by a second agent (assurance run wf_204fce21-fe8); report state/assurance/
  location: api /dayflags.json, /imports.json
  fix_hint: Add 06-limits entries. Do not hold pull_tails across the 1 GiB read.

DB-A41 (package WP-33c; severity blocking; area import journal / logs search; selection observability)
  observed: Import runs cannot be identified or searched by run id. The real journal has 0 run lines, so /imports.json gives run null on all 4,497 rows with runs []. /logs.json?run=4025 returns only 'command started'. The audit index has no run dimension and /imports.json has no run filter.
  expected: Every import step carries one run id, and that id finds all of the run's days, refusals and its start and end.
  evidence: Journal census; pinned /imports.json pages; cli/src/lib.rs:2424-2426; sink.rs:1824; imports.rs:81-134. | Reproduced by a second agent (assurance run wf_204fce21-fe8); report state/assurance/
  location: import journal / logs search
  fix_hint: Stamp the importer's run id as the top-level record run (claim_run) on every gdfl.import event. Add a run dimension to the audit index and a run= filter to /imports.json.

DB-A48 (package IA; severity should-fix; area historic log vs journal; selection observability)
  observed: The old CLI log calls 214 non-done days 'day imported'. 212 days the journal closed 'incomplete', and indices 2022-03-07 (no journal line, files 0), are logged as 'day imported'. Keyed day search shows these lines.
  expected: The log agrees with the journal, or the reader flags the conflict.
  evidence: [a scripting language; its name withheld by the operator rule] cross-check of the cli log against the journal. 3cd0623c fixes new runs (gdfl_import.rs:2541-2546). | Reproduced by a second agent (assurance run wf_204fce21-fe8); report state/assurance/
  location: historic log vs journal
  fix_hint: Have /logs.json and /imports.json mark pre-D-3317 'day imported' lines whose journal outcome is not done.

DB-A50 (package IA; severity should-fix; area import journal; selection observability)
  observed: Day lines carry no wall-clock time; interrupted runs are not named. Only the new 'run' line has started=. A day's close time lives in the ring only (Info/Warn, not audited). The options run started 2026-10-06T08:57:38Z has no finish line in either the log or the journal.
  expected: Each closing line carries a timestamp and run, and an interrupted run is shown as such.
  evidence: Journal key census; tail of the cli log; pinned /imports.json open row with no run. | Reproduced by a second agent (assurance run wf_204fce21-fe8); report state/assurance/
  location: import journal
  fix_hint: Add at=<unix_s> to closing lines and a 'run stopped' or 'run ended' line.

````

Report one `findings[]` entry per item the kept scope names: IA-JR-01, the three new tests,
D-4896, D-4817 (receipt), FM-080, F84 / SF-9, DB-A34; and DB-A41, DB-A48, DB-A50 with status
`not_fixed` and how = "dropped by D-6900; owner <R3-23 / R3-24 / merged code>".

**The other confirmed findings with `package` "IA"** (`specStep` points every `ia` part at "ids
starting DB-A with package IA"; independent check, 24 Oct): DB-A02, DB-A05, DB-A19, DB-A22,
DB-A24, DB-A25 and DB-A49, besides DB-A48 and DB-A50 above. None is this part's. DB-A22 is
claimed by IA-6's spec and DB-A49 by IA-3's and VKI-1's. DB-A05, DB-A19, DB-A24 and DB-A25 are
about the operator's hand-run reconcile checker in `state/gdfl-board/` (outside this
repository), not the api. DB-A02 (the 12 missing indices days with no journal line) is claimed
by no launcher spec. Its subject is the importer's D-3317 refused lines, not an api reader.
DB-A34 and DB-A41 carry `package` "WP-33c", not "IA": they reach IA-10a only through the
round-3 spec of section 3.1.

## 8. The rules audit (verbatim)

Source file on the Mac: `state/design/rules-audit-20261011.md`; `grep -n '^## .*IA-10a'`
finds five sections (lines 688, 1122, 1309, 1326, 1336). All five are copied whole.

**How the cut reads them.** Every one of the 15 rows IA-10a *leads* (RA-277, -286, -248, -293,
-194, -311, -316, -070, -324, -327, -213, -336, -342, -229, -232) concerns the api child, a
press, `/gdfl/*`, shutdown, alerts or `/ingest`: all **dropped** with those pieces; name each in
`notes` as "dropped by D-6900". Of the 38 rows it is "also named in", the ones the kept half must
answer are **RA-030** (the day band's run, D-4896), **RA-033** (`/imports.json` over the whole
journal), **RA-191** (holes shown), **RA-157** (the calendar horizon: the receipt line only),
**RA-055** (the source census names: the receipt only), **RA-103** (hole reasons) and
**RA-203** (finding lines by run: already met by `/logs.json?run=`; say so). The D-4913, D-4914
and D-4915 sections are dropped with their routes; the D-4896 section is kept whole.

Source: `state/design/rules-audit-20261011.md`, lines 688,752 1122,1137 1309,1352

````text
## IA-10a: the api's import half

Leads 15 rows (2 high). Also named in 38 rows led by another owner.

| row | gap | rule | where (file:line or design line) | what to build or prove | test or bench to add |
|---|---|---|---|---|---|
| RA-277 high, does not meet it yet. Machine: macOS restarts for an update, the operator logs out, or the Terminal window is closed (the app gets 'terminate' or 'hang up', not Ctrl-C) | On those signals no stop file is written, each child dies mid-file, and after the restart nothing starts the app again, so the 'resumes by itself' promise never runs. A self-sufficient import stops silently. | Every extreme case | api/main.rs:52: shutdown is driven by Ctrl-C only. Design line 544 (F33): 'The cli has no signal handling'. 'SIGTERM' and 'SIGHUP' appear 0 times in the design; IA-GR-06b tests Ctrl-C only. WP-81's runbook has no start-at-login step. | Treat 'terminate' and 'hang up' exactly like Ctrl-C in the app and in the child. Runbook: start the app at login, and only once the drive is mounted. Test with the real signals. (owner line: IA-10a (the app's signals); IA-2 (the command-line and child side); WP-81 (runbook); NEW-33 'App supervisor and watchdog' (start at login)) | Test with the real signals. |
| RA-286 high, not proved yet. User: a press whose connection drops before the answer (browser closed, request cancelled mid-flight) | A start that is dropped half-way could leave the slot taken (every later press 'run_busy' until restart) or children claimed and never told to go. | Every extreme case | Design line 1182: the app keeps 'one in-memory slot per kind for the seconds between a press and its child's claim'. IA-GR-02 covers children that hear nothing for 15 s. Whether the start goes on when its caller has gone is not stated. | The start runs in its own task that owns the slot, never tied to the request. Test: 'a press whose client went away starts or releases, and frees its slot'. (owner line: IA-10a) | Test: 'a press whose client went away starts or releases, and frees its slot'. |
| RA-248 medium, does not meet it yet. Where the app looks for its files is set by a hand-run start script | Moving the drive, adding an archive folder or pointing at next year's files means editing a script and restarting the app, which stops a running import. The operator is not a programmer. No page shows the current settings. | One common, dynamic, scalable path | Design lines 916-923: 'The api reads, once at start' five settings; 'None has a default'. Today's start script, outside the code, sets the store, the masters folder, the rate table and the web folder the same way (deploy-8080.sh:40-41). | A settings file inside the store, shown on /ingest and changed there (each path checked before it is saved, each change audited), re-read without a restart; the environment settings stay as a fallback and are shown as such. (owner line: IA-10a (a design amendment: it builds the sources route); NEW-11 'Settings in the store, on a page' (settings in the store, on a page)) | None named yet: name a test or a bench with the fix. |
| RA-293 medium, does not meet it yet. Machine: the drive is moved to a second Mac after a crash | The second Mac would resume, by itself, a run started elsewhere, with its own source folders, worker count and build. | Every extreme case | Design lines 1142 and 1148: the claim records the host. The auto-resume rule (lines 1677-1690) checks mode, who started it, the auto-resume tick, the stop file and the resume day; it never reads the host. | Auto-resume only when the claim's host is this Mac; otherwise the page says 'interrupted on <host>' and waits for one press. Test IA-GR-06c. (owner line: IA-10a) | Test IA-GR-06c. |
| RA-194 medium, partly. A press is refused before any work starts (source missing, census room short, unstamped build, another run holds the lock) | Not built. The on-page list is memory only: 8 entries, gone at every restart. A command-line press refused while another run of that kind is live cannot log at all and leaves one line on the terminal. There is no forever record. | Saved, logged, searchable, on a page | Design lines 3355-3356: 'run refused' and 'run busy' are audited events; line 1775: the page lists the newest 8 refused presses from the app's memory; test IA-GR-25. A refused command-line press is 'on stderr' only (critic D-4903). | The page reads refused presses from the audit index (newest 20), not app memory. A press that cannot open its log folder writes its refusal as one record in a small 'presses' file under the runs-ledger lock. Test: listed after a restart. (owner line: IA-10a (the page list, test IA-GR-25); NEW-24 'IA-2b: forever records complete' (the command-line refusal record, beside the runs ledger, IA-2's file)) | Test: listed after a restart. |
| RA-311 medium, partly. User: Start pressed while the app is shutting down for a deploy | A press accepted inside the 10-second grace, after the stop files were written, starts a child that gets no stop file and outlives the server unseen. | Every extreme case | Tests IA-GR-06b and IA-GR-29 cover a shutdown during a run and a restart beside a live child (design lines 1713-1725). No 'shutting down' refusal exists (search: 0). | From the first shutdown signal every press answers 'shutting down' (503). Test. (owner line: IA-10a) | Test. |
| RA-316 medium, partly. Machine: the drive ignores upper and lower case in names (measured: the store folder opens under an upper-case spelling) | The new routes take words that become file names (kind, feed, instrument). Nothing says they are matched exactly before a path is built, so 'kind=Options' could reach the same file under another spelling and split the app's in-memory state. | Every extreme case | Source names are covered (design rows D19, B20: 'ambiguous_case_twins'). Second agents found feed=ZERODHA accepted and symbols=nifty answering 'count 0' with no error (verify-WP-61-r1 attack 3; verify-WP-61-r2 follow-up 7). | Rule: a request word is matched byte-for-byte against a closed list before any path is built; no path is ever built from request text. Table test with case variants on every new route (matrix row R19). (owner line: IA-10a, IA-10b) | Table test with case variants on every new route (matrix row R19). |
| RA-070 medium, not proved yet. Live progress on the page (/gdfl/import.json, asked once a second): watching several runs at once without slowing any of them | Not built. Every open tab repeats the 12 to 13 file steps every second; with 60 pollers the lock can be probed back to back, so a press could lose and answer a false 'run busy'. An alert raised mid-run would not show until the run ends. | O(1) at p99, measured + Everything in parallel + Every extreme case | Design line 4127 and section 5.7: at most 6 lock probes and 6 reads of 4 KiB, nothing kept in memory; bench C-GR-03. The critic adds one more read (IA-12a-1, D-4915): 13 file steps a second for each open tab. Not built. | Build IA-10a and IA-12a; answer every poll from one probe result kept for 1 second, so ten tabs cost the same as one (test IA-GR-17 with 60 probers; a bench at 60). Critic IA-12a-1 (D-4915): the read carries the open-alert count. (owner line: IA-10a, IA-12a; critic IA-12a-1 (D-4915)) | answer every poll from one probe result kept for 1 second, so ten tabs cost the same as one (test IA-GR-17 with 60 probers; a bench at 60). |
| RA-324 medium, not proved yet. Machine: the app cannot start the worker program at all (program file gone or not runnable, too many processes, the error-text file cannot be opened) | The press has no named answer; the page would show a general error or wait 10 s for a child that never existed, and the per-kind slot (line 1182) has no stated release. | Every extreme case | Design line 4340, a press's answers: 400, 409, 412, 500 'build_skew', 504 'child_silent'. None is for a start that fails outright or a child that exits before its first line. Lines 728-731: no rule if the error-text file cannot be opened. | A named refusal 'child_not_started{why}' (and 'log_unavailable' when the error-text file cannot be opened), the slot freed, an audited event, nothing written. Test with a missing program file and with a process limit of 1. (owner line: IA-10a) | Test with a missing program file and with a process limit of 1. |
| RA-327 medium, not proved yet. User: a flood of bad presses (a stuck script sending 10,000 refused requests a minute) | Every refused press writes an audited line, so the audit file rolls and the real history is pushed out. | Every extreme case | Test IA-GR-25: 'a press refused at preflight or busy leaves an audited event'. The app keeps only the newest 8 refused presses in memory (design line 1776). Test IA-AU-04 caps refusals per day inside a run, not refused presses. | One audited line per (code, minute) with a count. Test with 10,000 refused presses. (owner line: IA-10a) | Test with 10,000 refused presses. |
| RA-213 medium, designed to meet it, not built yet. A run is planned (the preview shown before Start is pressed) | Not built. Once Start is pressed, the plan the operator approved (how many days, which holes, what cost) is kept only while the run lives: the next run overwrites it, and the 'run claimed' event carries none of the plan's counts. | Saved, logged, searchable, on a page | Design line 4339 (GET /gdfl/plan.json: days to import, holes by reason, census room, free space, an ETA labelled 'extrapolation') and line 4363; bench C-GR-05. Merged code has no /gdfl/ route (server.rs:14480-14880) and no import panel. | Build IA-10a and IA-12a. Add one field (plan_days) to 'run claimed' and copy the plan's counts into the run's first progress record, so a finished run can be compared with what was promised. Test: both carry the plan's day count. (owner line: IA-10a, IA-12a (build); IA-2 (the plan's day count in the claim); IA-1 (in the first progress record, critic D-4904)) | Test: both carry the plan's day count. |
| RA-336 low, partly. User: Stop then Start at once; Stop pressed twice | Start while a run is 'stopping' answers 'run_busy' for as long as the current file takes (minutes on a large options file) with no hint of when. A second Stop replaces the stop file, losing who asked first. | Every extreme case | Test IA-GR-05 covers a single Stop; design line 1388 covers a Stop for a press with kinds not yet started; row R9 a Stop for a run that is not live. | The refusal says 'stopping, free in about N seconds'. A stop file is never replaced; a second Stop answers 'already requested'. Test. (owner line: IA-10a (the route's answer) and IA-2 (the stop file is IA-2's whole, section 16.2)) | Test. |
| RA-342 low, not proved yet. Machine: the child's error-text file grows without limit (a fault that prints once per row) | A printing loop can fill the disk through the one file that has no bound. | Every extreme case | Design lines 728-737: one append-only error-text file per kind, with no cap or roll stated. | Cap it at 16 MiB, then write one last line 'capped'. A limits row and a test. (owner line: IA-10a) | A limits row and a test. |
| RA-229 low, designed to meet it, not built yet. A claimed run is released before it starts (another kind was busy, or the page gave up waiting) | Not built, and a released press is on no page: the operator who pressed Start for three kinds and got none sees the refusal once and nothing afterwards. | Saved, logged, searchable, on a page | Design line 3352: 'run released before start' is an audited event with its reason; test IA-GR-02 says such a run writes no journal line and leaves its id unused. It reaches the page once, as that press's 409 answer, and the list of 8. | List released presses beside refused presses on /ingest (the same audit-index read as the refused presses). The release must also write an end record, or a released press would read as interrupted for ever. (owner line: IA-10a (the list); IA-2 (the child's release)) | None named yet: name a test or a bench with the fix. |
| RA-232 low, designed to meet it, not built yet. A child run built from a different version of the app than the one serving | Not built. | Saved, logged, searchable, on a page | Design lines 1729-1740 and 3374: the child refuses 'build_skew' before taking any lock; an audited error; alert kind 3 (line 3410); test IA-GR-33; bench C-GR-10. Critic X-6 (D-4894) keeps the unstamped-build check in one place. Not built. | Build IA-10a with the program-identity check; add the commit to the check when WP-81's stamp merges. (owner line: IA-10a (the check and alert kind 3), IA-2 (the claim's field); critic X-6 (D-4894; the unstamped check, IA-1)) | None named yet: name a test or a bench with the fix. |

Also named in:

| row | gap | this owner's part (the row's owner line) | led by |
|---|---|---|---|
| RA-154 high. A run is interrupted (the process died or the power went) | The exact case that already happened (a command-line import dies once) still raises no alert and no event under the design. | NEW-25 'Run watcher and new alert kinds' (IA-10a addition: a once-a-minute lock watcher; alert kind 14 in IA-11a) | NEW-25 |
| RA-155 high. A run is alive but stuck (for example a read hanging on a drive that stalls, as WD_BLACK did on 29 Sep) | A hung run looks like a slow one for ever: the page shows 'running', Stop never arrives, no alert is put and no button ends the process. The operator would need a terminal and a kill command. | NEW-25 'Run watcher and new alert kinds' (the watcher, the 'stalled' state, alert kind 15 and 'Force end'; with IA-10a, IA-2 and IA-11a); lock waits themselves: critic IA-2-2 (D-4906) | NEW-25 |
| RA-157 high. The trading calendar and holiday list stop on 4 Oct 2026, which has already passed | No day after 4 Oct 2026 can be imported or pulled today, and no page says so up front: the alert is unbuilt and the fix has not started. Every year a programmer must type next year's holidays into code. | WP-62 (extend now; no commits); IA-3 and IA-10a (the warning and alert kind 4, D-4884); IA-12a (the warning on the page); IA-14 (the yearly step); NEW-35 'Reference tables as store data' (the calendar as a sourced data file) | WP-62 |
| RA-161 high. An alert that cannot be written (the disk is full, the queue's lock is stuck, the queue is damaged, the store drive is gone) | The alert that matters most is the one that cannot be saved: 'disk full' is written to the full disk. A failed put leaves one line on a terminal nobody is watching. The critic names the case but its answer is only 'print and go on'. | critic IA-11a-4 (d) (D-4895) names the case; NEW-26 'IA-11n: alerts reach the operator' (the reserved space, with IA-11a, and the in-memory list, with IA-10a) | critic D-4895 |
| RA-164 high. The source is lost in mid-run (the archive drive drops), or the store's own drive is lost | A dropped archive drive ends the backfill with no alert. A lost store followed by an app restart is shown as an ordinary interrupted run. | NEW-25 'Run watcher and new alert kinds' (alert kind 13 and the watcher, with IA-10a); NEW-26 'IA-11n: alerts reach the operator' (the in-memory top-bar warning); WP-62 (the Zerodha pull's storage breaker; no commits) | NEW-25 |
| RA-020 high. Where the small page actions run: Stop, Dismiss, Acknowledge an alert, the status and sources reads | If these run on the server's shared workers, one slow disk write freezes every other request for that time. This exact effect was measured before it was fixed elsewhere: the feed list at p50 36 ms while workers were blocked. | NEW-04 'Every read in a named, bounded pool' (a new rule for IA-10a; extends critic IA-10b-3) | NEW-04 |
| RA-023 high. An alert is raised, listed on a page and acknowledged | Today there is no alert of any kind: nothing built, nothing measured. As designed an alert raised mid-run was not shown until the run ended. An alert that cannot be written (disk full, stalled lock) is printed on the terminal and dropped. | IA-11a (critic IA-11a-1 to -4), IA-10a, IA-12a (critic IA-12a-1, D-4915) | IA-11a |
| RA-173 high. A run is claimed and starts (who started it, what it reads, from where) | The lock, claim, ledger and live route are not merged. Today 'is an import running?' is a guess from log recency, so a dead run looks like a quiet one. For a run limited to named instruments only a 16-character digest of the list is kept. | IA-1, IA-2, IA-10a, IA-12a | IA-1 |
| RA-030 medium. Day band under the grid and the coverage calendar (/dayflags.json), which also says which run refused a day | Bounded per day, not per run: the more imports and pulls there have been, the slower every day band; and until the event carries a run id, matching a refusal to its run is a guess by time. Not on the live speed sheet; no limits row. | critic IA-11r-1 (D-4896, lands with IA-10a) for the import journal; NEW-14 'Pull journal at scale' for the Zerodha half (natural home IA-10c); WP-99 (row) | critic D-4896 |
| RA-033 medium. The import history on /ingest (/imports.json for the whole journal) | Grows with every day and every run imported; the critic's fix (IA-11r-1) covers the run lines only. | NEW-13 'Plan and history from counters' (a new rule, to land in IA-10a's change to imports.rs, beside critic IA-11r-1, D-4896) | NEW-13 |
| RA-120 medium. How many workers run when three kinds are pressed together (the limit is per program, not per press) | More threads than cores when three kinds run together, and a command-line run beside them is not counted at all. | critic IA-10a-1 (D-4913), lands with IA-10a | critic D-4913 |
| RA-122 medium. The per-press worker limit (D-4913) is passed around by three separate presses, by auto-resume after a restart and by a command-line run | Press options, then stocks, then indices: each is a press of one kind and gets up to 12 workers, 36 in all on 14 cores, plus 32 flush threads each. Same after a restart. Checking runs, the merge, the scrub and index builds are not counted. | NEW-15 'Machine-wide run budget' (one machine-wide run budget read from the live claims; extends critic IA-10a-1, D-4913, which lands with IA-10a; IA-2 writes the claims) | NEW-15 |
| RA-123 medium. The per-press limit splits workers evenly, not by work, and does not count flush threads | A three-kind press would run options on 4 workers, about 40% slower by the one measurement there is, while 8 workers given to indices and stocks have almost nothing to do. And three programs still start 96 flush threads. | NEW-15 'Machine-wide run budget' (an amendment to critic IA-10a-1, D-4913, landing with IA-10a); WP-16 (the flush-thread share) | NEW-15 |
| RA-124 medium. The three timers of one start are out of order: the app waits 10 s for the claims, a claimed child waits 15 s for 'go', and a lock may now wait 30 s | A lock that is slow for 11 s makes the press fail as 'child silent' after 10 s, never as 'lock stalled': the wrong reason on the page. The children that did claim are released; the slow one may claim later with nobody listening. | critic IA-2-2 (a new line in decision D-4906, lands with IA-2); IA-10a (the 504 row of design section 12) | critic IA-2-2 |
| RA-178 medium. A stop is asked for (who asked, and when) | As the design is written, who stopped a run and when is recorded nowhere that lasts. The critic's fix names the event but gives it no test id. The command-line stop can still write the stop file and log nothing. | critic X-5 (D-4893), landing with IA-2 and IA-10a | critic D-4893 |
| RA-179 medium. An interrupted run is dismissed by the operator | As written, dismissing a dead run's claim leaves no event; who dismissed what cannot be found later. | critic X-5 (D-4893), landing with IA-10a | critic D-4893 |
| RA-246 medium. The import's routes, files, locks and panel are named after one vendor | A second archive vendor would need its own copy of about 20 design packages (IA-1 to IA-12a) or a rename of everything after it is built. | NEW-36 'Feed, kind, timeframe and exchange tables' (a design amendment to IA-1, IA-10a and IA-12a before they start) | NEW-36 |
| RA-247 medium. Adding a second data vendor tomorrow: every place that has to change | A new vendor is edits in about 20 files across 4 crates and the pages, found by search, with nothing that fails when one is missed. | NEW-36 'Feed, kind, timeframe and exchange tables' (with a conformance test; a design amendment before IA-1, IA-6 and IA-10a are built) | NEW-36 |
| RA-249 medium. Data: a day with more than 2,000 logged refusals (a keyed log search stops after 2,000 line reads) | A very bad day (the kind the operator most needs to see whole) is shown in part with no word that it is partial, and the page does not offer 'show the next 2,000'. | IA-11r (it widened this reader; in repair after a failed second check); IA-10a edits the same file after it | IA-11r |
| RA-297 medium. Machine: the safety snapshots taken before every bulk write are never removed | Each snapshot pins every block that is later replaced (repair generations, old census files, rolled logs). The disk fills with space no file listing shows, and the plan's free-space estimate does not count it. | WP-81 (runbook section 4) and IA-10a (the plan line) | WP-81 |
| RA-055 medium. Where the source files are, what days they hold, and measuring the archives | Nothing measured; the first measuring of each archive was on the start path. The design misses a new day dropped into a folder. Source locations are read once at start, so moving the drive means a restart. The source kinds are a fixed list. | IA-3 with critic IA-3-2 (D-4909); IA-10a with critic IA-10a-2 (D-4914) | IA-3 |
| RA-057 medium. The plan preview table on /ingest (/gdfl/plan.json) | Not built. Grows with the range asked for, has no cap on the range and no stated p99 budget, is recomputed on every change of the form, and shares the pool of 8 with about a dozen page routes. | NEW-13 'Plan and history from counters' (a new rule inside IA-10a and IA-3); IA-10a, IA-3 (build) | NEW-13 |
| RA-061 medium. Free space on the drive: seeing what is free and what the design will add | No single disk budget and no merged page shows free space. The app warns but never refuses to start a run it cannot finish; the projection leaves out repairs and sort indexes. The first alert about space is the failed write itself. | WP-81 (in repair after a failed second check); WP-99 (the disk-budget row); IA-10a (the plan line and the watcher); IA-16 (the full-disk stop); NEW-25 'Run watcher and new alert kinds' (a 'disk low' alert kind, with IA-11a) | WP-81 |
| RA-191 medium. A day's quality facts are recorded and shown (holes, late rows, a minute-sampled day, refused-row counts) | The minute-sampled-day and halt labels are not built: WP-11c has no branch. 'Day rescanned' is an info line, so who rescanned which days is not audited. The read grows with the number of import runs and is not measured on the live app. | WP-11c (the two labels; no branch); critic IA-11r-1 (D-4896, lands with IA-10a); WP-99 (live measurement) | WP-11c |
| RA-196 medium. A day is done (imported), with or without refusals: what, and when | The forever line carries no time of day (finding DB-A50, unowned; the design does not cite it). 'When was this day imported?' can only be answered from the short log. The unfiltered list grows with the number of days. | NEW-24 'IA-2b: forever records complete' (one field in IA-2's journal change; closes the first half of finding DB-A50); IA-10a (the paging default) | NEW-24 |
| RA-200 medium. The bought archive changes or rots after a day was imported from it | For everything imported so far, a later change in the bought files cannot be detected at all; for new imports it is found only when someone runs a cold check. Nothing re-reads the archives for rot, and they are the only copy. | IA-3 (the source cache), IA-6 (the two flags), IA-10a (the route); NEW-32 'Forever files and indexes watched' (the event, the alert and the archive scrub) | IA-3 |
| RA-201 medium. An alert raised during a run shows on the page the operator has open | Even with the critic's fix, an operator sitting on /db, /audit or /markets sees no alert: the banner is on two pages. | critic IA-12a-1 (D-4915), landing with IA-10a and IA-12a; NEW-26 'IA-11n: alerts reach the operator' (the top-bar count) | critic D-4915 |
| RA-203 medium. Finding log lines by run, day, month, instrument, reason or code | 'Every malformed row in September 2020' (the design's own example) cannot be asked; the page's designed links would return unfiltered or already-rolled answers. The 8-folder build failed its second check. Not on the live speed sheet. | IA-11r (new additions; the log route is its file); IA-10a, IA-10b, IA-12a, IA-12b (the links) | IA-11r |
| RA-207 medium. The census is near or at its size ceiling | Capacity is a number in a table; there is no gauge and no early alert (for example at 90 percent), so the first notice is a stopped run. | IA-3 (the room check), IA-10a (the plan line); NEW-29 'IA-19: health page' (the gauge); NEW-25 'Run watcher and new alert kinds' (the early alert) | IA-3 |
| RA-264 medium. The alert list shows the newest 1,024 alerts; older ones need a command line | The operator is not a programmer, and the design sends him to a terminal for anything older than the window. | IA-11a (the writer; critic IA-11a-1 to -4, D-4893, D-4895); IA-10a (the route, with a new small step: page backwards on the alert list; critic IA-12a-1, D-4915) | IA-11a |
| RA-068 medium. Memory used by an import run (many workers, three kinds at once) | No measured peak. Three kinds at once can ask for 24 to 36 worker threads on 14 cores. A run the system kills for memory looks, to the operator, like a run that silently stopped. | IA-7 and IA-13 (limit L-IA-12 and the peak-memory measurement, critic IA-1-5); critic IA-10a-1 (D-4913); IA-10a (the plan line) | IA-7 |
| RA-069 medium. Starting an import from the /ingest page: the press is admitted (slot, locks, run id, claims), and all its kinds start together or not at all | Nothing exists to measure. The worst wait for one press is about 40 s by the design's own parts, written nowhere. Start-to-claimed is unmeasured while the app waits 10 s, so on a cold drive the day's first press may fail with nothing wrong. | IA-2, IA-10a, IA-12a (the words); critic IA-2-2 (D-4906) for the lock wait; the critic's note (3) under IA-2 for start-to-claimed | IA-2 |
| RA-140 medium. The three kinds of data (indices, stocks, options) imported at the same time, one separate program per kind | Not built. Today the operator starts each kind by hand, one command each. Three kinds together have never been run, so their combined time and memory are unknown. | IA-1, IA-2, IA-10a (build); IA-13 (memory for 1 and 3 kinds, plus a new bench line: three kinds side by side on one drive) | IA-1 |
| RA-214 medium. Live progress of a running import (day, files, rows, refusals, rate, ETA) | Not built and not measured. The record keeps only the newest 8 refusals, and the next run of the same kind overwrites it, so a finished run's last numbers vanish when a new run starts. It refreshes only while its tab is visible. | IA-1, IA-2, IA-10a; NEW-24 'IA-2b: forever records complete' (the final copy) | IA-1 |
| RA-334 low. Machine: the app's program file is overwritten in place while a child run is working | macOS can kill a running program whose file changes under it. The child dies mid-day, the automatic resume then refuses 'build_skew', and the page is not told to say 'restart the app, then press Resume'. | WP-81 (runbook); IA-10a (the page text) | WP-81 |
| RA-339 low. Time: a run that is still going when the date changes at midnight (India time) | Nobody says which 'today' a run uses once the date rolls over: the once-a-day alert, the plan's 'days left' and a Zerodha pull's 'today' can each change in the middle of one run. | IA-1 (a run fixes its 'today' when it takes its lock: the run's parameters are IA-1's in design section 16.2); IA-10a (the once-a-day alert); WP-62 (a Zerodha pull's 'today') | IA-1 |
| RA-103 low. Small fixed checks the design adds around a run: build mismatch, census room, hole reasons, predicted holes | Not built; the 776 header reads of a first plan are not given a time. | IA-3, IA-10a | IA-3 |
| RA-230 low. A run is resumed, or replaced by a newer run | Not built. No forever record ties run B to the run A it resumed (the claim is overwritten; the event is in the rolling window). | IA-2, IA-10a; critic IA-2-3 (D-4907); NEW-24 'IA-2b: forever records complete' (the 'resumes' link needs the same ledger format change) | IA-2 |

## Critic resolution D-4896: The day band's run comes from the event's own field; the O(runs) decode is L-IA-39 until then (lands with IA-10a)

Leads 1 rows (0 high). Also named in 3 rows led by another owner.

| row | gap | rule | where (file:line or design line) | what to build or prove | test or bench to add |
|---|---|---|---|---|---|
| RA-030 medium, does not meet it yet. Day band under the grid and the coverage calendar (/dayflags.json), which also says which run refused a day | Bounded per day, not per run: the more imports and pulls there have been, the slower every day band; and until the event carries a run id, matching a refusal to its run is a guess by time. Not on the live speed sheet; no limits row. | O(1) at p99, measured + Saved, logged, searchable, on a page + One common, dynamic, scalable path + Every extreme case | At most 92 days an answer (dayflags.rs:96). Today p99 3 to 14 ms (fix-IA-11r, n=200). With 100,002 run lines in the journal: p50 104 ms, p99 116 (verify-IA-11r-r1). It also reads every run of the Zerodha pull index (dayflags.rs:62-65). | Critic resolution IA-11r-1: take a refusal's run from the event's own field (D-4896), so no request decodes every run; the same for the pull journal. Then a bench at 1x, 10x, 100x runs and the limits row. (owner line: critic IA-11r-1 (D-4896, lands with IA-10a) for the import journal; NEW-14 'Pull journal at scale' for the Zerodha half (natural home IA-10c); WP-99 (row)) | Then a bench at 1x, 10x, 100x runs and the limits row. |

Also named in:

| row | gap | this owner's part (the row's owner line) | led by |
|---|---|---|---|
| RA-012 high. Readers find a run's refusals wherever that run logged: searching the log across every run's own folder (8 folders) | Unmerged, failing its second check: a log folder that cannot be read is answered as 'read' and the day's row says 'none': missing evidence reads as good news. The folder list is fixed in code, in four hand-kept copies. IA-1 must wait. | IA-11r (in repair after a failed second check); critic IA-11r-2 (D-4897) and IA-11r-1 (D-4896); order: IA-1 waits for IA-11r | IA-11r |
| RA-033 medium. The import history on /ingest (/imports.json for the whole journal) | Grows with every day and every run imported; the critic's fix (IA-11r-1) covers the run lines only. | NEW-13 'Plan and history from counters' (a new rule, to land in IA-10a's change to imports.rs, beside critic IA-11r-1, D-4896) | NEW-13 |
| RA-191 medium. A day's quality facts are recorded and shown (holes, late rows, a minute-sampled day, refused-row counts) | The minute-sampled-day and halt labels are not built: WP-11c has no branch. 'Day rescanned' is an info line, so who rescanned which days is not audited. The read grows with the number of import runs and is not measured on the live app. | WP-11c (the two labels; no branch); critic IA-11r-1 (D-4896, lands with IA-10a); WP-99 (live measurement) | WP-11c |

## Critic resolution D-4913: The worker bound is per press; plan.json carries it (lands with IA-10a)

Leads 1 rows (0 high). Also named in 4 rows led by another owner.

| row | gap | rule | where (file:line or design line) | what to build or prove | test or bench to add |
|---|---|---|---|---|---|
| RA-120 medium, does not meet it yet. How many workers run when three kinds are pressed together (the limit is per program, not per press) | More threads than cores when three kinds run together, and a command-line run beside them is not counted at all. | Everything in parallel | WP-16 branch (gdfl_import.rs, Workers): MAX 12 and DEFAULT 8 apply to each program, so three kinds run 24 worker threads on 14 cores. The design's memory sum, about 4.9 GB, is from type sizes, not measured (lines 1369-1371; limit L-IA-12). | Critic IA-10a-1 (D-4913): the limit is per press; each of k kinds gets min(asked, max(1, 12 / k)) workers; the answer and each claim say the number given; the plan carries max, default and per-kind. Test R7 gains the three-kind case. (owner line: critic IA-10a-1 (D-4913), lands with IA-10a) | Test R7 gains the three-kind case. |

Also named in:

| row | gap | this owner's part (the row's owner line) | led by |
|---|---|---|---|
| RA-121 medium. Every parallel width is a number typed into the code for this one Mac, not read from the machine | Nothing adapts to a busier or a different machine. The builder's own timing of one options day moved from 7.5 s to 13.2 s as the machine load went from about 6 to about 30. This breaks the operator's 'runtime dynamic' rule. | NEW-15 'Machine-wide run budget' (extends D-4913; pull and api) | NEW-15 |
| RA-122 medium. The per-press worker limit (D-4913) is passed around by three separate presses, by auto-resume after a restart and by a command-line run | Press options, then stocks, then indices: each is a press of one kind and gets up to 12 workers, 36 in all on 14 cores, plus 32 flush threads each. Same after a restart. Checking runs, the merge, the scrub and index builds are not counted. | NEW-15 'Machine-wide run budget' (one machine-wide run budget read from the live claims; extends critic IA-10a-1, D-4913, which lands with IA-10a; IA-2 writes the claims) | NEW-15 |
| RA-123 medium. The per-press limit splits workers evenly, not by work, and does not count flush threads | A three-kind press would run options on 4 workers, about 40% slower by the one measurement there is, while 8 workers given to indices and stocks have almost nothing to do. And three programs still start 96 flush threads. | NEW-15 'Machine-wide run budget' (an amendment to critic IA-10a-1, D-4913, landing with IA-10a); WP-16 (the flush-thread share) | NEW-15 |
| RA-068 medium. Memory used by an import run (many workers, three kinds at once) | No measured peak. Three kinds at once can ask for 24 to 36 worker threads on 14 cores. A run the system kills for memory looks, to the operator, like a run that silently stopped. | IA-7 and IA-13 (limit L-IA-12 and the peak-memory measurement, critic IA-1-5); critic IA-10a-1 (D-4913); IA-10a (the plan line) | IA-7 |

## Critic resolution D-4914: The source cache is filled on first use, never on the start path (lands with IA-10a)

Leads 0 rows (0 high). Also named in 1 rows led by another owner.

Also named in:

| row | gap | this owner's part (the row's owner line) | led by |
|---|---|---|---|
| RA-055 medium. Where the source files are, what days they hold, and measuring the archives | Nothing measured; the first measuring of each archive was on the start path. The design misses a new day dropped into a folder. Source locations are read once at start, so moving the drive means a restart. The source kinds are a fixed list. | IA-3 with critic IA-3-2 (D-4909); IA-10a with critic IA-10a-2 (D-4914) | IA-3 |

## Critic resolution D-4915: /gdfl/import.json carries alerts_open (lands with IA-10a)

Leads 1 rows (0 high). Also named in 4 rows led by another owner.

| row | gap | rule | where (file:line or design line) | what to build or prove | test or bench to add |
|---|---|---|---|---|---|
| RA-201 medium, partly. An alert raised during a run shows on the page the operator has open | Even with the critic's fix, an operator sitting on /db, /audit or /markets sees no alert: the banner is on two pages. | Saved, logged, searchable, on a page | Critic IA-12a-1: as written the banner is fetched 'with the page and on each run's end'; resolved by D-4915 (the status read carries the open-alert count). Design section 13, lines 4363 and 4366, puts the banner on /ingest and /verify only. | Put the open-alert count in the top bar of every page from one small route, with a link to the list; keep the full banner on /ingest and /verify. Build it after WP-61 and WP-81, which edit the same file. (owner line: critic IA-12a-1 (D-4915), landing with IA-10a and IA-12a; NEW-26 'IA-11n: alerts reach the operator' (the top-bar count)) | None named yet: name a test or a bench with the fix. |

Also named in:

| row | gap | this owner's part (the row's owner line) | led by |
|---|---|---|---|
| RA-023 high. An alert is raised, listed on a page and acknowledged | Today there is no alert of any kind: nothing built, nothing measured. As designed an alert raised mid-run was not shown until the run ended. An alert that cannot be written (disk full, stalled lock) is printed on the terminal and dropped. | IA-11a (critic IA-11a-1 to -4), IA-10a, IA-12a (critic IA-12a-1, D-4915) | IA-11a |
| RA-264 medium. The alert list shows the newest 1,024 alerts; older ones need a command line | The operator is not a programmer, and the design sends him to a terminal for anything older than the window. | IA-11a (the writer; critic IA-11a-1 to -4, D-4893, D-4895); IA-10a (the route, with a new small step: page backwards on the alert list; critic IA-12a-1, D-4915) | IA-11a |
| RA-070 medium. Live progress on the page (/gdfl/import.json, asked once a second): watching several runs at once without slowing any of them | Not built. Every open tab repeats the 12 to 13 file steps every second; with 60 pollers the lock can be probed back to back, so a press could lose and answer a false 'run busy'. An alert raised mid-run would not show until the run ends. | IA-10a, IA-12a; critic IA-12a-1 (D-4915) | IA-10a |
| RA-337 low. User: two tabs, with Dismiss, Acknowledge or Resume pressed in one of them | The new panel has no web test with two tabs; the second tab keeps the old banner. | IA-12a (critic IA-12a-1, D-4915; plus a two-tab web test) | IA-12a |

````

## 9. The e2e failure matrix rows

Source: `state/fix/e2e-failure-matrix.json` `.rows[]`. **One row has `owner` "IA-10a": FM-071**
(status `refused` on purpose under D-6900). **FM-080** is copied too: section 3.2 says "FM-080
stays here (item 7b)", while the matrix's `owner` is now "R4-11" (re-owned by D-7135 on 23 Oct;
its `owner_note` records both). Build item 7b and its test here, as section 3.2 says; R4-11
gets the warning by calling the verb. FM-124 moved to R4-11, FM-069's display half and FM-084 to
R3-23 (neither is this part's any more).

Six more rows name IA-10a without owning it (independent check, 24 Oct). FM-161, FM-250, FM-253
and FM-256 list it only in `related`. FM-407's evidence says "FM-080 partly (IA-10a)". FM-195
(owner R3-21) has the `owner_note` "IA-16 (holes, store_damaged, disk_full) and IA-10a
(interrupted, build skew, calendar horizon) each put their own" alerts. D-6900 drops those three
IA-10a alert puts, so **this part puts no alert**. The horizon is the receipt line of item 7b
only. None of these rows assigns any kept work here.

Source: `state/fix/e2e-failure-matrix.json`, `.rows[]` where `.id` is FM-071 or FM-080 (`case` is the row's `scenario` field; absent fields print null)

````text
{
  "id": "FM-071",
  "case": "/ingest shows nothing about GDFL imports. There is no /gdfl/import.json, /gdfl/plan.json or progress file, and the cli prints its receipt only at the end. During a 7 h options run the only signs of progress are the cli log and /imports.json day rows.",
  "how": "Every GDFL run.",
  "expected": "Each run writes a fixed 4 KiB progress file at most once a second. /ingest shows stage, counts, newest refusals and the ETA labelled as an extrapolation.",
  "test": "a_cli_runs_progress_shows_like_the_pages_own",
  "status": "refused",
  "status_note": "Refused on purpose under the one-command cut D-6900 (applied 23 Oct): The cli says one line a day on stderr through its one declared writer (gdfl_import_command.rs:75-77) and prints its receipt. The watcher (Claude Code or the operator) follows that line, the per-kind log logs/gdfl/<kind> and the journal tail. The 4 KiB progress record (D-4904) is written (:350-356) but has no production reader (Progress::read appears only in gdfl_import_command_tests.rs:856), and a watched run does not need one. What IA-10a still owed is the /ingest view of /gdfl/import.json, which serves only an unattended press.",
  "owner": "IA-10a",
  "owner_note": null,
  "evidence": "CONC-047: crates/api/src/server.rs:15755-15915 (no /gdfl routes); crates/cli/src/gdfl_import_command.rs:405-432 (receipt at the end)",
  "severity": "high",
  "related": [
    "IA-1",
    "IA-12a",
    "E2E-06"
  ]
}
{
  "id": "FM-080",
  "case": "The calendar of record ends 2026-12-31 (SOURCED_LAST_DAY, compile-time). From 2027-01-01 every GDFL day is CalendarUnmeasured (refused) and Zerodha minute coverage is NOT JUDGED, so the daily automation stops importing about 11 weeks from now.",
  "how": "A compile-time horizon; the 2027 NSE holiday circular arrives in December. | INFRA-041: Time passing; a scheduler running unattended across the new year. | CONC-052: Time passes, so the record runs out. | HUMAN-063: Year rollover.",
  "expected": "record_horizon warns 30 days ahead on /ingest and /pull (exists for pull status). Extend the warning to the GDFL chain and alerts. Ideally the calendar extends at runtime from a sourced NSE holiday file, recorded in the charter. After the end the scheduler pauses S4 and S5 with one `calendar_horizon` line a day (never hundreds of refusals), a red banner from 1 Dec, and catches up once the calendar is extended.",
  "test": "pull calendar tests: the GDFL chain status carries record_horizon; an alert fires at days_left <= 30. | INFRA-041: the_calendar_horizon_is_warned_before_it_passes (IA-GR-34), with an injected today. | CONC-052: the_calendar_horizon_is_warned_before_it_passes | HUMAN-063: the_calendar_horizon_is_warned_before_it_passes (IA-GR-34) and the_timer_refuses_past_the_record",
  "status": "partly",
  "status_note": null,
  "owner": "R4-11",
  "owner_note": "Kept with IA-10a under D-6900 (applied 23 Oct). Alert half: DROPPED under D-6900: the calendar-horizon alert is dropped with IA-10a's alerts half; the receipt line is the proof (test reworded to the_calendar_horizon_is_warned_on_the_receipt_before_it_passes). Both verdicts keep it. IA-10a keeps it as its owner (item 7b: every cli gdfl-import receipt prints calendar::record_horizon(today) when ends_soon or ended; test the_calendar_horizon_is_warned_on_the_receipt_before_it_passes, IA-GR-34 reworded, injected today). R4-11 also stops by name past SOURCED_LAST_DAY 2026-12-31 in preflight and gets the warning by calling the verb. | Kept with R4-11 under D-7135 (23 Oct). Omitted by the first agent. It was moved in from IA-10a by D-6900 (data.json and the matrix still name IA-10a). The calendar_horizon stop is incremental.",
  "evidence": "DATA-029: crates/pull/src/calendar.rs:546-583 (SOURCED_LAST_DAY 20_818 = 2026-12-31; record_horizon) || INFRA-041: crates/pull/src/calendar.rs:553 SOURCED_LAST_DAY = 20,818 (2026-12-31); record_horizon warns on the pull run status (:571-582); alert kind exists with no producer (crates/pull/src/alerts.rs:74-95). || CONC-052: crates/pull/src/calendar.rs:553-583 (horizon, 30-day warning); crates/api/src/pullrun.rs:1690-1712 (shown on /pull/run.json only) || HUMAN-063: crates/pull/src/calendar.rs:553 (SOURCED_LAST_DAY = 2026-12-31) and :577-585 (record_horizon, a 30-day warning). There is no timer.",
  "severity": "high",
  "related": [
    "IA-3",
    "R3-30",
    "E2E-04"
  ]
}
````

## 10. Repository text at the head that this part changes or cites (verbatim, e851da1ca)

### 10.1 docs/05-decisions.md: the D-4896 forward reference (to be replaced by a headed entry) and D-4817

Source: `docs/05-decisions.md at e851da1ca`, lines 70649,70650 77751,77764

````text
* **D-4896.** A forward reference: the day band resolving a run through the
  run-id index, which `docs/06-limits.md` says "lands with IA-10a". Not built.
### D-4817 — Near-miss names are named, not just counted: at most 16 per reason per source — 2026-10-20

**Implemented by** IA-3. A day-zip name under another folder than its own, a
day-zip name whose date is not real or lies outside the calendar's measured
span, and a stray entry of a loose-zip root or a tick-store tree are each
counted, and the first 16 of each reason are kept by name (4 for a loose
root's strays, the run line's bound, design §8 A19) in the source's census
(`ArchiveCensus`, `DayZipsCensus`, `DayTable`). A count with no names was
the gap: 43 of 50 index files of 2018-11-07 appear nowhere today. Showing
these names on `/ingest` is IA-10a's and IA-12a's; the run line's
`ignored_entries` field is IA-1's door. **Proof.**
`an_impossible_day_zip_name_is_named`, `a_loose_zip_in_the_wrong_kind_or_month_folder_is_named`,
`the_tick_stores_writer_files_are_never_opened`.

````

### 10.2 docs/06-limits.md: the section header citing D-4896 and L-IA-39 (to be rewritten)

Source: `docs/06-limits.md at e851da1ca`, lines 16867,16871 16983,17011

````text
## Ingest automation: the audit roll's reaper, log search over the run directories, the importer-evidence tails and the day band's run list — D-4813, D-4838, D-4862, D-4897, 11 October 2026

Rows L-IA-13, L-IA-36 and L-IA-37 of the ingest automation design (its §10.2),
landed by IA-11r with the code they bound, and row L-IA-39, which the design's
round-7 review added (it cites D-4896, which lands with IA-10a).
- **L-IA-39. `/dayflags.json`'s day band decodes every `run` line of the
  import journal on every request: O(runs), not O(1).** `/imports.json`'s
  route decodes at most 20 `run` lines and `run=<id>` the id's own (D-4813,
  bench C-LG-02). The day band cannot take that page: it attributes a logged
  refusal to the run that was running when it was logged, by time, over the
  whole list (`dayflags::refusals_of`), and a page of 20 would silently drop
  the runs older refusals belong to. So the crate-internal read it makes
  (`imports::answer` with `RunsAsked::Every`) decodes, renders and sorts every
  `run` line, once per page of days of every `/dayflags.json?feed=gdfl`
  request. `/dayflags.json` is what `/db` draws its day band from. The bound
  is the journal: 2 `run` lines on the operator's store on 11 Oct 2026, one
  more per import run. It stops being a limit when the band resolves a run
  from the refusal event's own `run` field through the run-id index, O(1),
  which is D-4896 and lands with IA-10a (after IA-16 gives every `refused`
  event that field); until then this row is IA-11r's. Nothing gates it: a
  ratio gate would only restate the row.
- **L-IA-39, measured.** Bench C-LG-02's day-band rows, the same run as
  above (`/dayflags.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2019-10&to=2019-10`
  through `api::logs::dayflags_json_over`, single calls, n=100 each, so the
  p99 printed is the slowest call), over journals holding only `run` lines:
  1,000 lines p50 1.17 ms, p99 1.44 ms, max 1.44 ms; 10,000 lines p50
  8.69 ms, p99 9.86 ms, max 9.86 ms; 100,000 lines p50 97.5 ms, p99 121.3 ms,
  max 121.3 ms. So about 1 µs a `run` line a request, linear, where the runs
  page beside it stayed at 82 µs at every size. The second agent's check of
  this package measured the same read over HTTP on a shadow of the operator's
  store with 100,000 `run` lines appended to the shadow's journal: p50
  103.85 ms, p99 115.65 ms, max 125.21 ms (n=200), against 1.74, 3.71 and
  4.89 ms with the journal's own 2 lines. Printed by the bench at every run
  and gated by nothing.
````

### 10.3 docs/04-invariants.md: IA1-01 (its last sentence is to be amended)

Source: `docs/04-invariants.md at e851da1ca`, lines 7628

````text
| IA1-01 | **`--stored` with options is refused by one authority, by one code** (IA-GR-13): the request, the plan and the request line each say `stored_with_options`, a misuse (the cli exits `MISUSED`), before anything is read or written. The api half waits for `POST /gdfl/import` (IA-10a) | `stored_with_options_is_refused_by_one_authority`; cli `a_run_whose_sink_was_refused_refuses_before_any_lock` (its misuse arm) | ✓ |
````

## 11. Files this part owns, and files it must not touch

### 11.1 Owned (from section 3.2; the design's own list in section 5.10 is superseded by the cut)

| File | What changes | Exists at e851da1ca |
|---|---|---|
| `crates/pull/src/gdfl_cm.rs` | item (0) only: `member_share` from `pub(crate)` to `pub` (one line; doc comment may say who calls it). Nothing else in this file | yes, 5,795 lines |
| `crates/api/src/import_journal.rs` | item (1): member-key parse, `hole: {reason, repairable}` on an `incomplete` row; its tests module | yes, 1,393 lines |
| `crates/api/src/imports.rs` | item (2): holes by reason, `holes_unrepairable`, `Owners::names` and `merged` reading member keys; its tests | yes, 2,627 lines |
| `crates/api/src/dayflags.rs` | item (3) the filter match, item (6) `refusals_of`'s run from the event (D-4896); its tests | yes, 8,675 lines |
| `crates/api/src/feeds_route.rs` | item (4): the open-day scan's stated rule for an open member key | yes, 3,069 lines |
| `crates/api/src/provenance.rs` | item (5): `filter_holds` and the GDFL reconstruction's member rows and holes | yes, 1,972 lines |
| `crates/cli/src/gdfl_import_command.rs` | item (7): the receipt's census lines and the horizon line (`render`, `receipt`); its tests (`crates/cli/src/gdfl_import_command_tests.rs` if that is where the module's tests live) | yes, 966 lines |
| new test functions in the owned modules' own test files | IA-JR-01 and the three named tests | none exists |
| `docs/05-decisions.md` | head **D-4896** (new `### D-4896 — ...` entry) and turn the forward-reference bullet at `:70649-70650` into a pointer to it; nothing else (D-6900 is already headed) | yes |
| `docs/06-limits.md` | rewrite **L-IA-39** (`:16983-17011`) to the O(1) lookup with its measured cost or "UNVERIFIED as a measurement", and name every added lookup's cost (DB-A34) | yes |
| `docs/04-invariants.md` | rows for IA-JR-01, `a_day_bands_refusal_names_its_run_from_the_event_not_the_clock`, `a_source_census_is_named_on_the_receipt`, IA-GR-34 (reworded); amend IA1-01's last sentence (`:7628`) per D-6900 | yes |

`crates/api/src/audit_json.rs` is **read, not edited**: `/audit.json?feed=gdfl` reaches the
journal through `crate::server::imports::summary` (`audit_json.rs:311`), so IA-JR-01 covers it
through `imports.rs`. If the summary's shape must change there, keep it minimal and name it.

### 11.2 Sequenced or shared edits outside ownership (each, if made, listed in `files_outside_ownership`)

- **The census on the receipt (item 7a).** The CM archive is opened inside
  `pull::gdfl_run` (`crates/pull/src/gdfl_run.rs:1595`, `Archive::open(&kind.root)`), and
  `pull::gdfl_import::Report` (`gdfl_import.rs:1548`) carries no census. So the receipt cannot
  read `Archive::census()` without either (a) a minimal change in `gdfl_run.rs` / the report
  that carries the source's `ArchiveCensus` back, or (b) the cli opening the archive a second
  time (an O(entries) read of the outer zip's directory: name its cost). (a) is the smaller
  and keeps one read; it is outside ownership. `DayZips` (with `DayZipsCensus`) is opened by
  **no production code** at the head (`grep` finds callers only in `gdfl_dayzips.rs` itself and
  tests), so there is no day-zips census to print on any real run today: say so on the receipt
  path and test it on a fixture.
- **The five hole reasons in the api.** `pull::gdfl_import::hole_is_repairable(reason)` is `pub`
  (`gdfl_import.rs:1018`) and answers row 4a's three; `HOLE_REASONS` (all five) is
  `pub(crate)` in `gdfl_import_causes.rs:838`, and `FailKind` (`pull::ingest::FailKind`, `pub`)
  has `code()` and `is_hole()` but no parse from a code. The api needs "is this reason one of
  the five" by pull's one rule, never a copied list (the D-0288 rule against a second table).
  The smallest honest route is one `pub` accessor beside `hole_is_repairable` in
  `crates/pull/src/gdfl_import.rs` (outside ownership: one line, named). Do not hand-copy the
  five strings into `crates/api`.
- **Index members.** `member_share` maps a SHARE member (`NSE-CASH-<sym>[.BE|.BZ]`) to its
  share. An INDEX member is `NSE-INDEX-<store key>` (`gdfl_cm.rs:1331-1333`, `index_member`),
  and `member_share` returns it unchanged. Every api reader that matches a member against an
  instrument's symbol must handle both forms by pull's rule (pull's own `shares_of`,
  `gdfl_import_journal.rs:554-561`, has the same reach). Test an index member and a share
  member, including a `.BE` series.

### 11.3 Files of other parts: do not touch

- `crates/api/src/main.rs`, `crates/api/src/server.rs` (no route is added; registration,
  `Site`, `end_runtime` are untouched), `crates/api/src/operation_audit_tests.rs`: the dropped
  half's. Never create `crates/api/src/gdfl_run_route.rs` or `crates/api/src/alerts_route.rs`.
- `crates/cli/src/gdfl_import_guard.rs` (AF-01's door: the only place an import takes its kind
  lock and claim). Read it; do not add a second door.
- `crates/pull/src/gdfl_import*.rs`, `gdfl_run*.rs`, `ingest_failkind.rs`, `alerts.rs`,
  `calendar.rs` (IA-1, IA-2, IA-16, IA-11a, WP-62 and others; read only, except the one-line
  accessor of 11.2 if you take it).
- `crates/api/src/logs.rs`, `/logs.json` (R3-24 merged; R4-10 open), `crates/api/src/pullrun.rs`.
- `web/**` (no web file is this part's; IA-12a is dropped), `web/build/**`.
- `config/store-steps.toml`, `.github/**`.

Open parts that also edit the owned files (merge the newest `origin/feat/db-fix` before your
final gates and keep each edit local to the function named): `import_journal.rs`: CC-12, IA-4,
IA-6, PREC-18, R4-10, R4-11, R4-25; `imports.rs`: AH-06, CC-12, PREC-18, R3-23, R4-02, R4-03;
`dayflags.rs`: IA-10b, AG-02-OPT, AH-06, CC-12, PREC-3, PREC-14, PREC-18, R3-10-OPT, R3-27,
R4-19, R4-19d, R4-21, R4-23, R4-25, R4-26, VKI-1; `provenance.rs`: CC-18, PREC-10, R3-27,
R4-23; `gdfl_import_command.rs`: AH-06, IA-3b, PREC-17, R3-23, R3-26, R4-11, R4-29;
`gdfl_cm.rs`: IA-4, PREC-2..5, PREC-14, PREC-15, PREC-17, PREC-19, R4-22. (All open at
e851da1ca; GF-1 and IA-3-OPT, which also named them, are merged.)

## 12. Decision numbers

- **D-4896** is this part's: the design critic's number (`critic-round7-20261011.md:560`,
  "The day band's run comes from the event's own field; the O(runs) decode is L-IA-39 until
  then | IA-10a"), reserved by name in `docs/05-decisions.md:70649` and in D-6900
  ("Landed by a kept half: D-4896 ... by IA-10a, with L-IA-39 rewritten"). Head it once
  (gate 27b). It is not in `state/decision-blocks.json` (no block lists it) and no branch
  heads it (`grep` of `docs/05-decisions.md` at e851da1ca: forward reference only).
- **No registry block is assigned to IA-10a** in `state/decision-blocks.json` (62 blocks;
  `free_from` D-7205). The one-command cut's block **D-6900..D-6909** belongs to the cut
  (D-6900 headed; D-6901 reserved for R4-10; D-6902..D-6909 "stay unused"): do not take a
  number from it.
- Any further decision this part needs (for example the stated rule for an open member key in
  the open-day scan, item 4, if it is a locked choice): **`D-TBD-IA-10a-1`, `D-TBD-IA-10a-2`,
  ...**, listed in the report; the Mac assigns real numbers at merge-back.
- Do not head D-4813, D-4817, D-4883 (merged), D-4805, D-4821, D-4840, D-4846, D-4848, D-4884
  (disposed of by D-6900), or D-4913, D-4914, D-4915 (their subjects are dropped).

## 13. NEEDS THE MAC

Run at merge-back, on the operator's Mac, after the cloud branch is merged into a Mac worktree
`$W/wt/P-IA-10a` (branch `fix/db-IA-10a`). `W=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`.
Never `:8080`, never the real store: GET only against a shadow whose bars are the real store's
link; anything that writes runs on a store root the check owns entirely. Every value below
that says "expected" is what the change must answer; record each URL and value as evidence.

### 13.1 Build and serve on the shadow (pool 5, port 8405)

```sh
W=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925
cd $W/wt/P-IA-10a
until mkdir $W/tgt/pool-5.lock 2>/dev/null; do sleep 30; done      # stale after 90 min: rmdir it
git ls-files -z | xargs -0 touch
export CARGO_TARGET_DIR=$W/tgt/pool-5
touch $W/tgt/pool-5.lock; cargo fmt --all --check
touch $W/tgt/pool-5.lock; cargo clippy -p pull -p api -p cli --all-targets -- -D warnings
touch $W/tgt/pool-5.lock; cargo test -p pull -p api -p cli --locked
touch $W/tgt/pool-5.lock; cargo build --release -p api -p cli
mkdir -p $W/tgt/bin-IA-10a/release && cp $W/tgt/pool-5/release/api $W/tgt/pool-5/release/cli $W/tgt/bin-IA-10a/release/
rmdir $W/tgt/pool-5.lock
$W/state/fix/stop.sh IA-10a; $W/state/fix/shadow.sh IA-10a && $W/state/fix/serve.sh IA-10a 8405 $W/tgt/bin-IA-10a $W/wt/P-IA-10a
test "$(lsof -nP -iTCP:8405 -sTCP:LISTEN -t)" = "$(cat $W/shadow/IA-10a/test-server.pid)" && echo OURS
```

Expected: every gate green; `OURS`.

### 13.2 Real journal, unchanged answers (NIFTY first)

The real journal (`runtime/store/imports/gdfl.journal`, read-only count on 24 Oct 2026,
file last written 14 Oct): **23,270 lines** = 11,519 `begin`, 11,291 `done`, 226
`incomplete`, 210 `recounted`, 24 `run`; **0 member keys** (no filter field holds `+`); of the
`incomplete` closes **14 name `reason=filing`** and none names any of the five hole reasons.
Re-count before the check (`awk '$1!="run" && $4 ~ /\+/' <journal> | wc -l`, and
`grep -o 'reason=[a-z_]*'` over the `incomplete` lines): if a member key or a hole reason has
appeared since, the expectations below change accordingly.

So on real data the change must be **invisible except for its new, zero-valued fields**. Take
each answer from a baseline build of `feat/db-fix` (the commit the branch merged) and from this
build, both served in turn on the same shadow at 8405, and compare with `jq -S` after deleting
only the fields this part adds:

```sh
B=http://127.0.0.1:8405
for q in \
  '/imports.json?feed=gdfl' \
  '/imports.json?feed=gdfl&kind=indices&from=2019-10-01&to=2019-10-31' \
  '/imports.json?feed=gdfl&kind=options&from=2019-11-14&to=2019-11-14' \
  '/dayflags.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2019-10&to=2019-10' \
  '/dayflags.json?feed=gdfl&instrument=NSE-INDEX-BANKNIFTY&from=2019-10&to=2019-10' \
  '/provenance.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=1s&month=2019-10' \
  '/audit.json?feed=gdfl' \
  '/feeds.json' ; do
  curl -s "$B$q" | jq -S . > "$W/shadow/IA-10a/$(echo "$q" | tr '/?&=' '____').json"
done
```

Expected: identical to the baseline for every route except the added hole fields, which read
zero (`holes` by reason empty or all 0, `holes_unrepairable` 0) on `/imports.json`; no row shows
a `hole` object; `/dayflags.json`'s day band names the same runs for the same refusals as the
baseline (for a refusal whose record carries its run, now from that run, never by time; the
answer says which); `/feeds.json`'s open days unchanged. Repeat one selection on an F&O stock
(`/dayflags.json?feed=gdfl&instrument=<NSE stock key>&from=2021-02&to=2021-02`) and one
illiquid option strike's provenance month.

### 13.3 Member keys and holes on the shadow's cloned journal (synthetic lines, never the real one)

The shadow's `imports/` is an APFS clone (`shadow.sh`), so appending to
`$W/shadow/IA-10a/imports/gdfl.journal` cannot reach the real store (the L-IA-39 measurement
of 11 Oct appended 100,000 lines to a shadow journal the same way). Stop the server, append,
restart. Lines use the journal grammar `begin <kind> <YYYY-MM-DD> <filter>` and
`<done|incomplete|refused|recounted> <kind> <day> <filter> <field>=<value>...`; pick a NIFTY
indices day the real journal holds as `done` under `*` (take one from
`/imports.json?feed=gdfl&kind=indices&from=2019-10-01&to=2019-10-31`), call it `D`, and a run
id `R` from the journal's `run` lines:

```
begin indices D *+NSE-INDEX-NIFTYIT
incomplete indices D *+NSE-INDEX-NIFTYIT reason=needs_month_generation run=R
begin indices D *+NSE-INDEX-NIFTYNXT50
incomplete indices D *+NSE-INDEX-NIFTYNXT50 reason=bars_differ_held run=R
```

(and, for a share member, the same two pairs on a stocks day under `*+NSE-CASH-<sym>` and
`*+NSE-CASH-<sym>.BE`). Expected after the restart:

- `/imports.json?feed=gdfl&kind=indices&from=D&to=D`: the two member rows name base filter `*`
  and their members; the first carries `hole: {reason: "needs_month_generation", repairable:
  true}`, the second `hole: {reason: "bars_differ_held", repairable: false}`; neither is a plain
  `incomplete`; the counts show holes by reason 1 + 1 and `holes_unrepairable` 1; the base `*`
  row stays `done`; no `logged_only` `*` row is invented for the day.
- `/dayflags.json?feed=gdfl&instrument=NSE-INDEX-NIFTYIT&from=<D's month>&to=<D's month>`: day
  `D` shows the member's hole; the same request for `NSE-INDEX-NIFTY` still shows `D` as the
  base close's (done), unchanged.
- `/provenance.json?feed=gdfl&instrument=NSE-INDEX-NIFTYIT&timeframe=1s&month=<D's month>`: day
  `D` is a hole, not credited to the base close's run.
- `/audit.json?feed=gdfl`: its journal summary agrees with `/imports.json`'s counts.
- `/feeds.json`: the open-day list is unchanged (the member keys are closed); then append a
  lone `begin indices D2 *+NSE-INDEX-NIFTYIT` with no close and check the answer does what
  item (4)'s stated rule says.
- On `/db` in the browser pane at `http://127.0.0.1:8405/db` (only if no other agent uses the
  pane): NIFTYIT's day band shows `D`'s hole.

### 13.4 Timings (DB-A34, D-4896, L-IA-39)

n = 200 sequential GETs each, `curl -s -o /dev/null -w '%{time_total}'`, p50, p99 and max, for
`/imports.json?feed=gdfl`, `/dayflags.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2019-10&to=2019-10`
and `/provenance.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=1s&month=2019-10`, on the
real journal and again after appending 100,000 synthetic `run` lines to the shadow's journal.
Expected: `/dayflags.json`'s day band no longer grows with the `run` lines (the 11 Oct figures
were p50 1.74 ms with 2 lines against 103.85 ms with 100,000: after D-4896 the two must be of the
same order). Write the measured numbers into L-IA-39's rewrite.

### 13.5 The receipt (items 7a and 7b) on a store root the check owns

`cli gdfl-import` writes (journal, runs ledger, logs), so it never runs against a shadow whose
bars are the real link. Build a private root: `$W/state/fix/shadow.sh IA-10a-rx`, then replace
its `bars` link by a real directory holding copies (`cp -c`) of the one NIFTY index month of
`D`, and point the source variable at the operator's CM archive from his local untracked
configuration (never written into any file):

```sh
R=$W/shadow/IA-10a-rx
mv "$R/bars" "$R/bars.link"   # move the link aside; never delete
mkdir -p "$R/bars/gdfl/NSE/INDEX/NIFTY" && cp -cR <real store>/bars/gdfl/NSE/INDEX/NIFTY/<D's year-month files> "$R/bars/gdfl/NSE/INDEX/NIFTY/"
BRUTEX_STORE="$R" BRUTEX_GDFL_CM_ARCHIVE="<CM archive path from the local config>" \
  $W/tgt/bin-IA-10a/release/cli gdfl-import indices zip D D NIFTY
```

(The link is moved aside, never deleted: the operator's rules forbid deletions by agents.)
Expected: the receipt prints the archive's census (ignored, misfiled and invalid counts, each
with at most its first 16 names), and, on 24 Oct 2026 or any day of 2026, a horizon line
(`record_horizon(today)`: `last_day` 2026-12-31, `ends_soon` true, `days_left` 68 on 24 Oct,
`owed_year` 2027); prove `$W/../runtime/store` was not touched (its journal's size and mtime
unchanged). Also run the same with `indices tickstore` if the tick store is configured.

### 13.6 Store writes needed

None. This part changes readers and the receipt only; it writes no store file of its own and
needs no re-import, recount, migration or backfill.

## 14. Code facts at e851da1ca

All found with `grep` / `sed` on `feat/db-fix` at `e851da1ca743ae308c8437d5055e0cbaa1d88d83`.
Where section 3.2 cites a line that has since moved, the current line is given.

**`crates/pull/src/gdfl_cm.rs`** (exists, 5,795 lines). `pub(crate) fn member_share(member:
&str) -> &str` at **:1349** (spec: `:1309`; `:1309` is now `fn series_of`): strips
`SHARE_MEMBER_PREFIX = "NSE-CASH-"` (`:1336`) and a trade-for-trade series via `series_of`
(`:1309-1317`, `rsplit_once('.')` against `SHARE_SERIES`). `fn share_member` `:1340`,
`fn index_member(store_key) -> "NSE-INDEX-{store_key}"` `:1331-1333`, `pub fn member_of`
`:1215` (doc `:1203-1214`: a member never holds a space or a `+`). Module is `pub mod gdfl_cm`
(`pull/src/lib.rs:164`).

**`crates/pull/src/gdfl_import_journal.rs`** (pull's own reader, not owned). `fn shares_of`
**:554-561** (`split_once('+')` then `member_share`); `read_line` `:1153` records
`member_keys` (`:1158` `split_once('+')`); hole reasons kept per key (`hole_reason` map,
`:338`, `:956`); `VERB_WORDS` `:305` = begin, done, incomplete, refused, run, recounted.

**`crates/api/src/import_journal.rs`** (exists, 1,393 lines; the api's own journal parser,
`crate::server::import_journal`). `enum DayOutcome` `:65` (Done, DoneWithRefusals,
Incomplete, Refused, Open); `struct Stats` `:116` (`run: Option<u64>` among them);
`struct DayRow` `:142` with `filter: String` (opaque; `*` or the names), `reason:
Option<String>`, `stage`, `previous`; `struct RunLine` `:193`; `struct Parsed` `:265` with
`rows: BTreeMap<Key, DayRow>`, `runs: Vec<RunLine>`, **`runs_by_id: HashMap<u64, Vec<u32>>`**
(IA-11r's run-id index, kept at parse time), `runs_unindexed`; `Parsed::range` `:307`,
`runs_of(id)` **:333** (one hash probe), `runs_before` `:346`, `nth_of` `:354`; `fold` `:386`
matches `["begin", kind, day, filter]` `:433` and `[verb @ (done|incomplete|refused|recounted),
kind, day, filter, rest @ ..]` `:444-455`; `pub(crate) fn parse_day` `:613`; `snapshot`
`:698` (a process-wide tail cache behind a `Mutex<HashMap<PathBuf, Tail>>`, `:685`). **No `'+'`
handling anywhere in the file.**

**`crates/api/src/imports.rs`** (exists, 2,627 lines). `PARAMS` `:109` (feed, from, to, kind,
outcome, order, page, run, stage, reason, why, runs_before); `asked` `:136`; `imports_json`
`:306`; `answer` `:352`; **`summary` `:393`** (used by `/audit.json`); `summary_of` `:450`;
`row_json` `:678` (its `logged` entries carry seq, at_ms, message, about, why, stage, code:
**no `run`**, `:688-700`); `wanted` `:744`; **`logged_only` `:793`** (the made-up `*` refused
row); `walk` `:831`; **`merged` `:909`**; `enum Placed` `:966`; **`placed` `:979`**;
`next_start` `:995`; `started_of` `:1001`; `subject_of` `:1032`; **`struct Owners` `:1065`,
`Owners::names` `:1072-1085`** (`*` names everything; else a memoised `split(',')` set per
distinct filter; no `+` handling), `Owners::owned` `:1089`; `kinds_json` `:1119`; `run_json`
`:1159`; `day_band_json` `:1390`; `runs_page` `:1452`; `RUNS_PAGE = 20` `:104`.

**`crates/api/src/dayflags.rs`** (exists, 8,675 lines). `runs_of(body)` `:880` (the run
lines of an `/imports.json` answer, sorted by start); **`journal_rows` `:905`** (asks
`imports::answer` page by page, ranks rows by closing line and whether the filter names the
symbol); **`struct Names` `:1027`, `Names::names` `:1044-1053`** (memoised per distinct
filter; `filter != "*" && filter.split(',').any(|name| name == self.symbol)`; a test counter
`SPLITS` pins the memo); **`start_of(runs, id)` `:1130-1134`, a linear `runs.iter().find`**;
**`refusals_of` `:1181`** (places each logged refusal by time with `imports::placed`
`:1213`, and finds a superseded refusal's run by `runs.iter().rev().find(...)` on start time).

**How a refused event carries its run today.** `pull::gdfl_import::note_failure`
(`gdfl_import.rs:1884-1896`) emits `Event::error(TARGET, "refused")` with kind, day, about,
why, stage, code: **no `run` field of its own**. The run is on the record's top-level `run`:
`RunKey::claim(id)` (`gdfl_import_day.rs:1286-1310`, DB-A41's leftover) takes the sink's run
key for the run's whole read, so every event the run writes carries the run's journal id as
`telemetry::Record.run` (when the sink was free; else one warn "run key held by another run").
So "the event's own `run`" of item (6) is, at the head, `Record.run`; it is not copied into
`row_json`'s `logged` entries yet. An event logged before DB-A41's fix carries none: that is
the "event without the field" case that keeps time placement and must say so.

**`crates/api/src/feeds_route.rs`** (exists, 3,069 lines). `journal_reach` `:892`; the
open-day scan **`:1121-1130`**: `row.kind == *kind && row.filter == "*" && row.outcome == Open
&& !row.previous.is_some_and(wrote)`; serves `/feeds.json` (`server.rs:16056`).

**`crates/api/src/provenance.rs`** (exists, 1,972 lines). **`fn filter_holds(filter, symbol)`
`:1022-1024`** (`filter == "*" || filter.split(',').any(...)`), called at **`:964`** and
**`:1046`**; `gdfl_reconstruction` `:1030`; route `/provenance.json` (`server.rs:16146`),
params feed, instrument, timeframe, month (and `t`).

**`crates/api/src/audit_json.rs`** (exists, not owned). `audit_json` `:100`; the journal's
summary at **`:311`** (`crate::server::imports::summary(&site.store_root)`).

**`crates/cli/src/gdfl_import_command.rs`** (exists, 966 lines). `command_in` `:444` (runs
through the door, then `gdfl_run::execute_claimed` / `execute_kind`, `:565-568`); **`receipt`
`:572-616`** (Report, Refused, Unopened arms); `render` **`:834`**; `render_holes` `:819`,
`holes` `:763` (already prints hole days by reason using `pull::gdfl_import::hole_is_repairable`,
D-6900 keep item 2, IA-16's), `standing` `:805`; `say_on_stderr` `:633` (the one declared
stderr writer). **No census line and no horizon line today** (`grep` for `census`,
`horizon`: none in the receipt path).

**Census types.** `pub struct ArchiveCensus` `gdfl_archive.rs:471` (ignored: count only, **no
names**; misfiled + `misfiled_named`; invalid + `invalid_named`); `CENSUS_NAMED = 16` `:464`;
`Archive::census()` `:687`; `Archive::ignored()` `:681` (sum of the three). `pub struct
DayZipsCensus` `gdfl_dayzips.rs:199` (held; misfiled/invalid/stray, each with its first
`DAYZIPS_NAMED = 16` names, `:77`); `DayZips::census()` `:347`. No production code reads
either census (spec's claim confirmed by `grep`), and no production code opens `DayZips`.
Note: section 3.2's "the ignored, misfiled and invalid counts and the first 16 names of each"
cannot name ignored entries of the CM archive, which keeps only their count; say so.

The tick-store source has a census of its own, which section 3.2 does not name but D-4817 does
("`ArchiveCensus`, `DayZipsCensus`, `DayTable`", section 10.1). It comes from the independent
check of 24 Oct. `pub struct DayTable` is at `gdfl_tickstore.rs:548`. It has `held`,
`misfiled_count` (the first misfiled file of each (tree, day) is kept in a private map, read
through `misfiled(tree, day)` at `:571`), `stray` and `stray_named` (`:559-561`, the first
`TICKSTORE_NAMED = 16` names at `:536`) and `fault`. `TickStore::table()` (`:716`) walks it
lazily, once per source. `NfoTickStore` uses the same table for options (`gdfl_nfo.rs:1068`).
`gdfl_run` opens `TickStore::new(&kind.root)` and `NfoTickStore::new(&kind.root)` at
`gdfl_run.rs:1586-1592`. So for `source=tickstore`, "each source's census" on the receipt is
the `DayTable`: misfiled and stray counts, with the first 16 stray names. It is reached the same
way as the archive's, through a minimal change outside ownership (section 11.2). The options
zips source `NfoZips` (`gdfl_nfo.rs:1141`) keeps no census struct at the head: say so on its
receipt rather than invent one. D-4817's run-line field `ignored_entries` ("IA-1's door") does
not exist at the head (`grep`: no match in `crates/`).

**`crates/pull/src/calendar.rs`** (exists, not owned). `pub struct RecordHorizon` **`:1034`**
(last_day, days_left, ends_soon, ended, owed_year); **`pub const fn record_horizon(newest_day:
i64)` `:1056`** (spec: `:553-591`); `SOURCED_LAST_DAY` `:990` (= 2026-12-31 per the matrix
row's evidence); `ends_soon` is `newest_day >= LAST_SOURCED_YEAR_STARTS`. Existing callers:
`cli/src/calendar_days.rs:669`, `api/src/pullrun.rs:2870`.

**Hole reasons.** `pull::ingest::FailKind` (`ingest_failkind.rs:35`, re-exported
`ingest.rs:153`): `code()` `:233` (NotHeld = `needs_month_generation`, Superseded =
`month_superseded`, Differs = `bars_differ_held`, RepairRefused = `repair_refused`,
RepairImpossible = `repair_impossible`), `is_hole()` `:254` (those five), `repairable()` `:269`
(only NotHeld, Superseded: the doors' sense, **not** admit row 4a's).
`HOLE_REASONS: [&str; 5]` `gdfl_import_causes.rs:838` and `REPAIRABLE_HOLES: [&str; 3]` `:848`
(NotHeld, Superseded, RepairRefused) are `pub(crate)`; **`pub fn hole_is_repairable(reason)`
`gdfl_import.rs:1018`** is the public row-4a answer. `Report.holes_unrepairable`
`gdfl_import.rs:1655` is pull's own count. For `hole.repairable` use `hole_is_repairable`
(row 4a: three reasons), not `FailKind::repairable`.

**The cli door.** `crates/cli/src/gdfl_import_guard.rs` (1,470 lines, AF-01's): the only place
an import takes its kind lock and claim; `gdfl_run::execute` / `execute_kind` take none.

**Tests named by this part.** None exists at the head (`grep -rn` for
`the_api_journal_reader_names_member_keys_and_holes`,
`the_calendar_horizon_is_warned`, `a_day_bands_refusal_names_its_run`,
`a_source_census_is_named_on_the_receipt`: no match; docs/04 has no IA-JR-01 or IA-GR-34 row).

## 15. Not found, contradictions, and choices left to the builder

- No fix-plan work package for IA-10a (`fix-plan.json` returns `[]`); no confirmed finding is
  tagged with package IA-10a; the design row names no F1..F18 gap.
- No decision-number block for IA-10a in `state/decision-blocks.json` (D-4896 is its one
  number; otherwise `D-TBD-IA-10a-n`).
- FM-080: section 3.2 keeps it here; the failure matrix's `owner` now says R4-11 (D-7135).
  Built here per section 3.2; the matrix owner is the Mac's to reconcile.
- FM-071 is owned here in the matrix but `refused` on purpose under D-6900 ("closes as not
  applicable" in section 3.2): report it `not_fixed`, dropped.
- Section 3.2's line citations drifted: `member_share` `:1309` is now `:1349`; `shares_of`
  `:549-560` is now `:554-561`; `record_horizon` `:553-591` is now `:1034-1066`; L-IA-39
  `docs/06-limits.md:16950` is now `:16983`; IA1-01 `docs/04-invariants.md:7601` is now `:7628`.
  The others (`imports.rs:793`, `:1072-1085`, `dayflags.rs:1044-1050`, `:1181`,
  `feeds_route.rs:1124`, `provenance.rs:1022`, `:964`, `:1046`, `audit_json.rs:311`,
  `docs/05-decisions.md:70649`) still hold.

  The independent check of 24 Oct found four more drifts:
  - `gdfl_dayzips::DayZipsCensus` `:179` is now `:199`.
  - D-4817's sentence "Showing these names on `/ingest` is IA-10a's and IA-12a's" at
    `docs/05-decisions.md:74754` is now `:77761-77762`.
  - The "unclaimed, take none" lines `gdfl_run.rs:1335-1340` are now `:1520-1546`: `execute`
    is at `:1526`, `execute_kind` at `:1542` ("with no claim"), and `execute_claimed` at
    `:1549`. Lines `:1335-1340` are now `held_by_a_run`. The claim itself still holds.
  - `dayflags.rs` "journal_rows :981" is the `names.names(filter)` call inside `journal_rows`
    (`:905`). `Names::names` runs `:1044-1053`.
- **The horizon warning is a year, not 30 days (D-5156).** Several sources say the warning
  starts "30 days" before the horizon: design section 6.9, IA-GR-34's design row ("warns within
  30 days") and FM-080's `expected`. At the head that is false. `record_horizon` sets
  `ends_soon: newest_day >= LAST_SOURCED_YEAR_STARTS` (`calendar.rs:1061`, `:1073`), which is
  1 January of the last sourced year, so it is true on every day of 2026. D-5156 (headed at
  `docs/05-decisions.md:74469`, "The record's end is warned from 1 January of its last sourced
  year, not 30 typed days before") records this. `RECORD_ENDS_SOON_DAYS` is gone. The reworded
  test `the_calendar_horizon_is_warned_on_the_receipt_before_it_passes` must pin the code's
  rule, with an injected today:
  - no line on a day before 1 January of the last sourced year;
  - an `ends_soon` line from that day to `SOURCED_LAST_DAY`;
  - an `ended` line after it.

  Derive every one of those days from `calendar`'s own constants. Never type a date or a
  30-day lead.
- "IA-16 gives every refused event that field": at the head the refused event has no `run`
  field of its own; the run is the record's top-level `run` (section 14). Build item (6) on
  `Record.run`, and name the record without one.
- `ArchiveCensus` keeps no names for ignored entries; `DayZips` has no production caller; the
  census is not on the import `Report` (section 11.2).
- The api has no public "is a hole reason" test from pull (section 11.2); index members are not
  mapped by `member_share` (section 11.2).
- The design's sections 4 ("The child's standard streams"), 5.6 (restart, shutdown), 5.7
  (progress document), 7.1 (alerts) and 12's POST routes describe only the dropped half. Their
  table rows that the IA-10a row cites are copied above (sections 5.7, 5.9-5.11, 5.13); their
  full prose (design lines 619-756, 1649-1813, 3305-3544) is not, because section 3.2 says
  never to build them.

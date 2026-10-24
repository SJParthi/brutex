# IA-3b: Ingest preflight refusals IA-3 left

Self-contained specification for the cloud session that builds part **IA-3b**. Everything the
Mac builder of this part would read is copied below, verbatim where it is a specification, each
block labelled with its source file and line numbers on the operator's Mac. The Mac paths
(`/Volumes/WD_BLACK/...`, `wt/`, `tgt/`, `state/`) do not exist in the cloud: read
`README.md` and `CONTEXT.md` in this folder for what differs. This repository is brutex only.

## 1. Header

| Field | Value |
|---|---|
| Name | `IA-3b` |
| Title | Ingest preflight refusals IA-3 left: year zips missing, unreadable or empty, oversized members, sources changing under a run, census room under the v4 model, the legacy cutover, a read-only or non-local store and the open-file limit, where each source begins, the tick-store format digest |
| Kind | `nw` (a new package of the 20 Oct follow-ups, carried in `__ARGS.extraPackages`, not in EMBED). The launcher's `specStep(p)` therefore takes its **non-`ia` branch** (fix-plan entry plus findings, section 2 below), even though the work is ingest automation. The package spec itself cites the design, the critic and IA-3's report by line, so those are copied too (section 4). |
| Priority rank | **57 of 137** in `__ARGS.priority` (0-based index 56): after `PREC-15`, before `R4-27-OPT` |
| Critical path | **No** (`__ARGS.criticalPath` does not name it) |
| Decision block | **D-7000..D-7009** (section 7) |
| Base | `feat/db-fix` head **e851da1ca** (24 Oct 2026, "Merge DATA-2: ...") |
| Mac port / pool (for the NEEDS THE MAC commands only) | shadow server port **8506** (8400 + index 106 of the launcher's `only`-filtered list), shared build pool `tgt/pool-4` |
| Source of the package | `state/followups-20261020` (`replan-critic.json` [1].items[3], critic gap "The data-safety preflight half of IA-3 belongs to neither replan"), with `state/fix/round2/fix-IA-3.json` findings left `not_fixed` or `partial` |

### PREREQUISITES

Computed as `(package deps + __ARGS.extraDeps["IA-3b"]) - __ARGS.dropDeps["IA-3b"]` (launcher
`db-fix-round-2-run0738.js.fat` line 211; `parked` is empty, so it removes nothing):

- package `deps`: `IA-3-OPT`, `IA-16`
- `__ARGS.extraDeps["IA-3b"]`: none
- `__ARGS.dropDeps["IA-3b"]`: none

| Prerequisite | State on `feat/db-fix` at e851da1ca |
|---|---|
| `IA-3-OPT` | **merged** (`5ff4956cc Merge IA-3-OPT: Ingest, options remainder of IA-3: the read-only _v3 extraction-ledger reader ...`; also in `__ARGS.doneExtra`) |
| `IA-16` | **merged** (`c099cf003 Merge IA-16: failure kinds and causes: the admit table, decisive order, the typed repair door`; also in `__ARGS.doneExtra`) |

FILES_ONLY links (`__ARGS.filesOnly["IA-3b"]`): **none**. Nothing to "merge first, do not block the start".

Related parts named by the spec, for orientation (not prerequisites):

| Part | Relation | Merged at e851da1ca? |
|---|---|---|
| IA-3 | parent; this part is its unbuilt preflight half | merged (`c76f9178c`) |
| AF-01 | its door (`gdfl_import_guard.rs`) is cited, never rebuilt | merged (`3911d1b9c`) |
| R4-07 | the disk-projection floor check (`pull::disk_projection`) | merged (`f85ef6b22`) |
| R4-08 | `pull::tickstore_write` (`MEMBER_CAP`, `LOCK_NAME`, `member_too_large`) | merged (`016f62a65`) |
| R3-19 | owns RA-011 (census publish cost) | merged (`d6189b557`) |
| CC-1 | the source scrub; shares the one identity function of item (4) | NOT MERGED (define the identity here; CC-1 calls yours) |
| R3-23 | its durable refusal line (FM-084) is asserted only once it merges | NOT MERGED |
| R4-05 | its reasons dictionary takes one sentence per new code, if on `feat/db-fix` when you commit | NOT MERGED (so R4-05 takes them) |
| R4-11 | the one command; its import children are `cli gdfl-import` and meet these codes | NOT MERGED; **R4-11 waits on IA-3b** (`__ARGS.extraDeps["R4-11"]`) |
| R4-29 | extends `gdfl_run_preflight.rs` after you with its PERF-13 floor | NOT MERGED; **waits on IA-3b** (`extraDeps["R4-29"]`) |
| IA-13 | re-measures C-GR-05 on the operator's Mac | NOT MERGED; **waits on IA-3b** |
| WP-99 | the ledger closer | NOT MERGED; **waits on IA-3b** |
| R3-26, R4-22, IA-4 | also edit `gdfl_import_causes.rs` | NOT MERGED (IA-4: not merged) |

`__ARGS.specAppend["IA-3b"]`: **none** (no later amendment: no options, one-command-cut,
no-fresh, ledger follow-up, R4-09b or priority text was appended to this part). The global
notes that shaped it are quoted in section 3.3.

## 2. What the Mac builder is told (launcher text, verbatim)

The Mac builder's prompt is `fixPrompt(p)` (launcher lines 351-373), which embeds
`specStep(p)` (lines 341-349). For IA-3b (`kind: 'nw'`) `specStep` returns its second branch.
`${C}` is EMBED's shared context (this folder's `CONTEXT.md`), `${p.spec}` is section 3.1
below, `${W}` is `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`, `${OUT}` is
`${W}/state/fix/round2`, `${p.port}` is 8506, `${POOL(p.name)}` is 4, `${p.deps}` is
`IA-3-OPT, IA-16`. Steps 2, 5 and the pool rules are Mac mechanics; in the cloud follow
`README.md` instead, and list every step-5 check under NEEDS THE MAC (section 8).


Source: `state/fix/db-fix-round-2-run0738.js.fat`, lines 341-349 (specStep)

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

Source: `state/fix/db-fix-round-2-run0738.js.fat`, lines 351-373 (fixPrompt)

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
governor says so); the cloud equivalent is the `GO`/`HOLD` switch in `README.md`.

What the prompt's conditional paragraphs find for IA-3b (checked 24 Oct 2026, read-only):

- **No earlier attempt exists**: no worktree `wt/P-IA-3b`, no branch `fix/db-IA-3b` (or any ref
  naming IA-3b) in `wt/DBFIX`, and no `state/fix/round2/fix-IA-3b.json`,
  `verify-IA-3b-r<n>.json` or `fix-IA-3b-progress.md`. Start fresh; there is nothing to resume.
  `feat/db-fix` has no `Merge IA-3b:` and no `IA-3b: squash of ...` commit, so the fold rule (d)
  does not apply.
- **The NIFTY evidence file** `state/gdfl-board/out/nifty.json` (9 Oct 2026, 21 entries) is
  evidence about the `/db` page and the api (spot and expired-option selections); none of it is
  about import or preflight. For this part the NIFTY FIRST rule means the import checks: one
  NIFTY spot day, one NIFTY options day and one special session (section 8, M9).
- **Global switches**: `__ARGS.noCritic` and `__ARGS.noGate` are both `true` (they switch off the
  launcher's own critic and final workspace-gate agents, not anything this builder does);
  `__ARGS.portBase` is unset, so the port is 8400 + the part's index in the `only`-filtered
  package list (106).

Launcher constants used above (lines 26-31, verbatim):

````text
const W = '/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925'
const POOL = n => { let h = 0; for (const c of n) h = (h * 31 + c.charCodeAt(0)) >>> 0; return h % 8 }   // 15 Oct: 8 shared build folders (tgt/ grew to 749 GiB with one per part)
const OUT = `${W}/state/fix/round2`
const DESIGN = `${W}/state/design/ingest-automation.md`
const CRITIC = `${W}/state/design/critic-round7-20261011.md`
const CO = 'Co-Authored-By: Claude Opus 5.5 (1M context) <the no-reply address>'
````

(Line 31's address is elided above for the public repository; in the cloud, end commits with the
trailer `README.md` gives.)

## 3. The package's own text

### 3.1 Title and spec (the whole specification, verbatim)

Source: `state/cloud-20261024/args.json` `.extraPackages[34]` (the same object as launcher line 6,
`__ARGS.extraPackages`, name `IA-3b`). Fields `name`, `title`, `deps`, `kind`, `source`,
`decisionBlock` are in section 1; this is `.title` then `.spec`, unaltered.

````text
Ingest preflight refusals IA-3 left: year zips missing, unreadable or empty, oversized members, sources changing under a run, census room under the v4 model, the legacy cutover, a read-only or non-local store and the open-file limit, where each source begins, the tick-store format digest

DECISION BLOCK D-7000..D-7009 (the 20 Oct follow-ups run; the main session records it in state/decision-blocks.json before launch). Also land, each under its own number, the design's and the round-7 critic's ids for this work. None has a heading in docs/05-decisions.md on feat/db-fix 453de6989 or on any fix/* or origin/* branch (checked 20 Oct): D-4802 and D-4850 (state/design/ingest-automation.md:4572, :4620), D-4819 (:4589), D-4864 (:4634) and D-4908 (state/design/critic-round7-20261011.md:572). Re-grep docs/05 on feat/db-fix and on every fix/* and origin/* branch before committing. Any other locked choice takes the next id of D-7000..D-7009 in order; an unused id stays unused. Read CLAUDE.md in your worktree first: it is law. Rust only outside web/. Never the Py-language: no such files, no interpreter commands, never its name in any file. Section 3 rule 1: every cap and threshold is measured or cited, otherwise UNVERIFIED. Rule 4: each check is O(1), or bounded and named in docs/06. Section 4: no fallback that hides a failure. Section 8: every source root comes from configuration, never a default.

WHAT AND WHY. IA-3 merged at c76f9178c and left its preflight half not_fixed (state/fix/round2/fix-IA-3.json .findings: D-4802, D-4850, D-4864, IA-GR-07/15/20/26, FM-060, FM-096, FM-133, RA-056, RA-103, RA-207, RA-238, RA-306, RA-314, RA-322, RA-323, RA-340; D-4815, D-4819 and RA-304 partial; its notes: 'Preflight lock-file check (D-4908): deferred'). IA-3-OPT takes only the options remainder (the _v3 reader, FM-043) and says IA-3's preflight 'is NOT this part'. The one-command cut (D-6900) keeps 'loud named refusals of bad data' and 'verification of every file'. This part is those refusals. Every check runs once per run inside pull::gdfl_run::plan (gdfl_run.rs:1133), after the scope is resolved and before any lock, claim or journal line. cli gdfl-import already calls plan before AF-01's door takes the kind lock (gdfl_import_command.rs:421, :464). R4-11's one command meets the same codes, because each of its import children IS `cli gdfl-import KIND tickstore FROM TO` through the verb's door (R4-11's Import stage). Nothing here caches across runs, watches a folder or runs on a timer.

ALREADY AT HEAD: cite these or call them, never rebuild them (line numbers at 453de6989; re-find them):
- AF-01's door, crates/cli/src/gdfl_import_guard.rs (06464aa35): source_root_unknown (FM-060's root half: no FORMAT.md, no <yyyy>.zip, no day zip of the tree), run_busy, no_file_read, source_lost and store_lost. Its tests: gdfl_import_guard_tests.rs:481 a_wrong_source_root_is_refused_before_any_day_is_journalled and gdfl_import_command_tests.rs:343 a_run_that_read_no_file_never_exits_ok.
- gdfl_dayzips::DayZips with DAYZIPS_WALK_CAP and dayzips_too_many_entries (gdfl_dayzips.rs:53, :363; D-4834); gdfl_tickstore::DayTable (D-4866).
- manifest::epoch_capacity (manifest.rs:577); gdfl_import_census::census_room (gdfl_import_census.rs:98), which stays on the refusal path.
- RunRefusal (gdfl_run.rs:747), plan (:1133), SourceCache (:1249, one per run).
- calendar::record_horizon, RECORD_ENDS_SOON_DAYS, past_record and SOURCED_LAST_DAY (calendar.rs:553-600).
- pull::tickstore_write::MEMBER_CAP (2 GiB, tickstore_write.rs:126) and its refusal member_too_large{name, size} (:463, :519, :1091): the cap on the one member a worker holds in memory.
- pull::tickstore_write::LOCK_NAME, the tick-store root's .lock (:120).
- R4-07's pull::disk_projection floor check: the one floor (settings Key::DiskFloor).

BUILD. Each item adds a RunRefusal variant, appended, so codes are never renamed or reused (design section 5.8). Each fires before any lock, claim or journal line, and is named on the cli's GDFL IMPORT REFUSED line.
(1) Options year zips (FM-060's remainder; design section 8 rows A9 and A11).
- Every year in [from, to] has exactly <yyyy>.zip under the options root, else year_zip_missing{year, near_miss:[at most 4 names]}, for example 2020.ZIP or '2020 (1).zip'.
- Each year zip must be readable: a 1-byte positional read of its end record, else source_unreadable{path, mode}. Design section 2.1 measured 2021-26 at mode 0600 and nlink 2.
- Agree the rule with IA-3-OPT's fiscal-year misfile check (NfoZips::year and ::day, DayZipMisfiled), and state in docs/05 which refusal a year held only inside another year's zip gets.
(2) Empty archives (RA-322; new matrix rows A23 and B28a in docs/04 beside their tests).
- A year zip whose central directory lists zero entries is year_zip_empty{year} at preflight.
- A day zip nested in the CM archive or in an options year zip that lists zero entries refuses its own day as day_zip_empty{name}, and the day stays open. This follows IA-GR-27's rule: one Cause::open arm in gdfl_import_causes.rs, exhaustive, with no '_' arm.
- A tick-store day file whose index lists zero entries refuses its day the same way.
- Loose day zips are not here: they reach the import through R4-11's write_day stage, which refuses a zip holding no file of its day by name (no_file_in_day, tickstore_write.rs:461).
(3) member_too_large (RA-238, matrix row B28).
- The check runs before any read of a member's packed bytes (gdfl_archive.rs member_bytes :836) and before a tick-store entry is rebuilt (gdfl_tickstore.rs rebuild :298). A stated unpacked size above the cap refuses that day, and the day stays open.
- Use the code and fields R4-08 already ships, member_too_large{name, size}, and by default tickstore_write::MEMBER_CAP, which states the same memory rule ('one member and its columns are held in memory by each worker'). Only if the import reader holds a different multiple of the member in memory may it name a second constant, with a docs/05 entry giving the measured multiple.
- Measure, read-only on this machine, the largest member of the real CM archive, of the options year zips and of the tick store. State each against the cap, with its margin, in docs/06. Never a typed guess.
(4) source_changing (RA-323, and RA-304's remainder).
- For each source file, take one identity when its listing is read and another after its last read in this run: (len, inode, mtime, ctime), without st_dev, which changed on remount (16777240 -> 16777244, CC-2.md).
- Read each file through ONE open handle (RA-304: never list by path and then read by path again). At the end, compare the handle's own identity with the path's, so a file replaced by rename is caught even when its length and mtime are preserved.
- The CM outer archive and the year zips are checked once per run per file; a tick-store day file once per day.
- A difference is source_changing{path, before, after}. Every day read from that file in this run stays OPEN, never closed and never done, and the run ends FAILED naming the file. The next run reads it again.
- Also probe the tick-store root's lock (tickstore_write::LOCK_NAME) once at plan: shared, non-blocking, never creating the file. A held lock is source_busy{root, holder}, because a writer is replacing day files.
- One identity function only. If CC-1 (the source scrub) has merged its archive identity first, call it; otherwise define it here and CC-1 calls yours.
- Cost: two stats a file.
(5) Census room (D-4802, D-4850, IA-GR-15, IA-GR-26, FM-096, and the room half of RA-207).
- Read only the census header: n_valid, n_keys and, at v4, capacity (manifest.rs:2248-2260, two slot reads).
- Read the first planned day's listing once: need_first_day is its count of wanted files.
- free_now = capacity - n_valid; on v3, MAX_ENTRIES - n_valid. free_after = epoch_capacity(segment, n_keys) - n_keys, computed with the writer's own function, never a copy.
- Refuse census_room_short{layout, free_now, free_after, need_first_day} when need_first_day > max(free_now, free_after).
- FM-096's DISK half: when only a compaction gives the room, pass the new epoch's bytes (its record stride times epoch_capacity, from the writer's constants) to R4-07's floor check. If they do not fit, refuse census_compaction_no_disk{need_bytes, free_bytes, floor}. There is one floor, never a second number.
- When the range may need more than the room, the plan names census.projected_stop{day}, labelled 'extrapolation from the first day'.
- The full decode stays on the refusal path.
- RA-207's gauge and early warning are not this part: they are R3-25's tile and R3-33's page.
(6) Legacy cutover (D-4864, D-4908, IA-GR-20, RA-056).
- Refuse legacy_run_may_be_live{key} when two things are both true: no imports/gdfl.lock or imports/gdfl.<kind>.lock exists on this store, and the loaded journal has a key whose last line is begin.
- The lock-file test is one metadata call per name. A file is never opened with create: a probe that created it would erase the very evidence D-4908 reads.
- The cli flag --legacy-run-not-live acknowledges it, and the claim records the acknowledgement.
- State the refusal path's cost in docs/06.
(7) The store the run writes (RA-314, RA-340, RA-306). It always runs after (6), never before.
- store_read_only{root}: an existing kind lock file, opened for writing without create, fails with EROFS or EACCES; or, when the file is absent, the volume's own read-only flag says so (the seam below). Never create a file on the store to test it. Its sentence names the same fix as IA-16's store_not_writable (FM-129): reuse those words, one sentence per condition.
- store_not_local{root, fs}: the store's volume is not local, read from the volume's own flags through a seam a test can fake.
- open_files_short{have, need}: the process's soft open-file limit is below need. need = the run's workers x the open files one worker holds, measured (L-IA-35, IA-GR-08), never typed. Measured 20 Oct: `launchctl limit maxfiles` answers soft 256 on this Mac, so a run from a fresh Terminal can meet this refusal; its sentence names the operator's hand step.
- RA-306's other half: the importer's io error 24 (EMFILE) is named open_files_exhausted at IA-16's store-level classifier (one arm, after IA-16 merges), never filed as a general store error.
- How the flags and the limit are read is one docs/05 entry. Every crate root forbids unsafe. Use a fixed system binary that is not an interpreter, through std::process::Command with an argument vector, no shell and no interpolated input (precedent: disk_route.rs:142 runs /bin/df -Pk), or a pure-Rust read. /usr/bin/ulimit is a POSIX shell script (file(1), 20 Oct) and is never called. If no exact read exists, the check is NOT DONE in docs/06 by name, never faked.
- R4-11's own preflight (volume_not_mounted, started_inside_sandbox, the BRUTEX_READ_ONLY writer gate) runs before its children. Keep your codes distinct from those, and cite them in docs/05.
(8) Where each source begins (FM-133, and the count half of RA-057).
- The plan states each kind's source_span{first, last} from the day table it already builds: the Archive day table, the NfoZips year index, or DayTable.
- It counts the asked days outside that span as outside_source, never as holes.
- The journal is unchanged.
(9) The tick store's format digest (D-4819).
- Once per run, take the sha256 of FORMAT.md (at most 1 MiB).
- Compare it with the two digests docs/00-charter.md records; gdfl_tickstore.rs:5-11 quotes both.
- Any other digest is not a refusal: the run line carries format_digest_unpinned=1 and the cli warns. The magic decides.
(10) D-4815's remainder: no dayzips source word is added to cli gdfl-import.
- R4-11 reads every configured dayzips root through R4-08's write_day into the tick store (R4-11's Sources clause). FM-052 is R4-11's, through E2E-03.
- A second import path for loose day zips would mean two places to verify one source.
- Land a docs/05 entry from this block amending D-4815 to say so, and correct gdfl_run.rs:41 ('dayzips is IA-3's') to name R4-11.

NOT THIS PART:
- The calendar horizon warning on the receipt (FM-080, IA-GR-34). IA-10a item 7b owns it, with the test the_calendar_horizon_is_warned_on_the_receipt_before_it_passes, and R4-11 owns its calendar_horizon stop. Days past the record stay calendar_unmeasured per day, as at head.
- RA-011, the census publish cost (L-IA-11): this is R3-19's.
- The source cache, Origin measurements, folder watching (RA-055, RA-200, RA-333), IA-10a's dropped plan routes, and any alert kind (D-6900).
- The archive CRC scrub: this is CC-1's.

OWNED FILES:
- crates/pull/src/gdfl_run_preflight.rs (new), declared from gdfl_run.rs with #[path]: one mod line, and one call in plan placed after the scope and before any lock;
- crates/pull/src/gdfl_run_preflight_tests.rs (new);
- crates/pull/benches/ratio/gdfl_preflight.rs (new; bench C-GR-05).
SEQUENCED EDITS, each listed in files_outside_ownership. Merge feat/db-fix just before committing, and keep both sides on a conflict:
- gdfl_run.rs: only the new RunRefusal variants, the mod line, the one call and the :41 doc line. IA-16 (a dependency, which merges first), R3-23 and R3-26 also edit this file.
- gdfl_archive.rs: member_bytes's size check and the empty-directory arm.
- gdfl_tickstore.rs: rebuild's size check and the empty-index arm.
- gdfl_import_causes.rs: the cm_cause arms, exhaustive with no '_' arm. R4-22, R3-26, IA-4 and IA-3-OPT also edit it.
- gdfl_nfo.rs: the empty nested day zip only. This comes after IA-3-OPT: keep its NfoZips edits.
- cli gdfl_import_command.rs: --legacy-run-not-live only. R4-29 owns the file; R3-23, IA-16 and R4-11 also edit it.
- IA-16's store-level classifier: the EMFILE arm.
R4-29 extends gdfl_run_preflight.rs after you with its PERF-13 floor (edge R4-29 -> IA-3b). If R4-05's reasons dictionary is on feat/db-fix when you commit, add one sentence per new code there; otherwise R4-05 takes them.

FAILING-FIRST TESTS. Write each one red on feat/db-fix and paste the failing assertion into notes. Fixtures are built in code, with prices no index ever printed and never a real vendor row. Every refusal test asserts that no import-journal line, no progress record and no lock file was written. Once R3-23 has merged, its one durable refusal line (FM-084) is expected, and the test asserts it.
- a_missing_or_misspelt_year_zip_is_refused_with_its_near_misses
- an_unreadable_year_zip_is_refused_before_the_run
- an_empty_year_zip_is_refused_before_any_line
- an_empty_nested_day_zip_refuses_its_day_and_stays_open
- an_empty_tick_store_day_refuses_its_day_and_stays_open
- a_member_stating_more_than_the_cap_is_refused_before_any_read
- a_tick_store_entry_rebuilding_past_the_cap_is_refused
- a_zip_that_is_still_being_copied_is_never_closed
- a_tick_store_day_file_replaced_by_rename_mid_read_is_source_changing (same length and mtime, new inode)
- a_held_tick_store_lock_is_source_busy_and_never_created
- a_census_without_room_for_the_first_day_is_refused_before_any_line
- the_v4_preflight_models_the_compaction_the_writer_makes
- a_compaction_the_disk_cannot_hold_is_refused_before_any_line
- the_first_claim_asks_about_a_legacy_run, with the acknowledgement recorded in the claim
- the_legacy_probe_never_creates_a_lock_file
- a_read_only_store_is_refused_before_any_line: a seam answering EROFS, plus an #[ignore] real test on a read-only disk image
- a_network_store_is_refused_store_not_local
- a_run_under_an_open_file_limit_of_256_is_refused_open_files_short
- an_emfile_during_a_run_is_named_open_files_exhausted
- days_before_a_sources_first_day_are_named_once: the plan names source_span and counts outside_source, never a hole
- an_unpinned_tick_store_format_digest_warns_and_marks_the_run_line
Also re-run AF-01's two FM-060 tests unchanged and cite them.

REAL DATA. Read-only. Never write the store, the tick store or a source; scratch stores go under the system temp dir only.
- With the operator's BRUTEX_GDFL_* roots (fix-IA-3.json notes give them), print each check's answer and its time.
- Measure the largest member of each container, for item (3).
- One NIFTY spot day, one NIFTY options day and the 2024-11-01 special session still import identically to feat/db-fix, with every check passing.

BENCH. C-GR-05 (design section 10.1; RA-103): p50, p99 and max (n >= 200) of the fixed checks, with the first (cold) plan and a repeated one printed separately, and of the one listing read at the largest options day (26,820 entries). Name everything that is not O(1) in docs/06. IA-13 re-measures it on the operator's Mac.

MATRIX ROWS. FM-096 and FM-133 close by their tests here. FM-060 closes by AF-01's two tests re-run here, together with item (1)'s year-zip tests. docs/04 has a row beside every test (A23, B28 and B28a included), docs/05 the entries above, and docs/06 the measured rows.

GATES. CLAUDE.md section 9: fmt; clippy -D warnings; cargo test --workspace --locked; cargo deny check; 100% line and branch coverage and no surviving mutant on the touched modules.
````

### 3.2 specAppend

`__ARGS.specAppend["IA-3b"]` is **absent**: no amendment was appended after the 20 Oct
follow-ups (checked in `state/cloud-20261024/args.json` on 24 Oct 2026).

### 3.3 Global launcher notes that bind this part (verbatim)

Source: `state/cloud-20261024/args.json` (launcher line 6, `__ARGS`), the notes that name or
govern IA-3b.

````text
_note_followups_20261020: 20 Oct 2026 follow-ups (state/followups-20261020/followups.jq, assembled from items.json; workflow wf_1ce34c6b-527). Applied AFTER build-options.jq | oneclick-cut.jq | r4-19d.jq | the R4-11 raise. NEW PACKAGES (10, decision blocks D-7000..D-7119, registered by the main session): IA-3b D-7000, CC-1 D-7010 (CC-2 folded in), CC-3 D-7020, CC-7 D-7030 (CC-17 folded in), CC-5 D-7040, CC-8 D-7050, CC-14 D-7060 (CC-13 and CC-15 folded in), CC-19 D-7070, VKI-1 D-7080, AG-06b D-7100; D-7090..D-7099 the folds (CC-9 into R4-29, CC-10 into R4-09) and brought-in parts; D-7110..D-7119 existing parts. BROUGHT INTO .only: WP-99, IA-13, R4-29, SC-8, SC-11, R3-20, R3-33, R3-34, R4-24. NAMES OUT (folded, no package): CC-2 and CC-17 from .only and .priority; CC-9, CC-10, CC-13 and CC-15 from .priority (CC-2's specAppend and dropDeps stay for the backtest-first check). FIRST GOAL: R3-22, R4-21 and R4-27-OPT just before SC-5 (edges-and-scope/attack2/first-goal.jq). DEFERRED, NOT DROPPED (D-7094): SC-9 (operator D-4931), SC-12, CHG-2, CC-11. No timer, watcher or unattended job is added (D-6900); every scrub, sweep, restate and recount is an attended verb.

_note_oneclick_20261020: 20 Oct 2026 ~12:50 IST, operator over Remote Control: ingest is always driven by Claude Code or the operator from a terminal; the unattended layer of the one-click ingest is dropped and everything that protects the data stays (decision D-6900, block D-6900..D-6909; round 2, assembled from the second attack round of wf_5e601f33-82f). DROPPED (never built): IA-7, R4-12, R4-14, R4-15, and IA-12a (by IA-10a's verdict). SPLIT (unattended half dropped, text in specAppend): R4-11, IA-10a, IA-16, R4-13, R4-10, IA-10b, R4-23, R4-26, R4-28. KEPT with amended scope: R4-02, R4-19, PREC-10. Receivers of moved items carry MOVED IN / AMENDED / NOTE text. File-order links added: R4-05 -> R3-21, R3-27 -> R3-24. Proof 2 now reads 'one command from the Dropbox link' (state/oneclick-cut-20261020/decision.md). Failure cases re-owned: state/oneclick-cut-20261020/cases.json.

_note_options_20261020: 20 Oct 2026 options-combined replan (operator ~09:50 IST: stocks + indices + options together in ingest + /db; the 18 Oct split is withdrawn, the 18 Oct backtest-first cut is kept). parked emptied (IA-4, R4-21, R4-25, PREC-8, AH-06 built here; CHG-0 in new work); new packages R3-07-OPT, R3-10-OPT, AG-02-OPT, R4-27-OPT, CLOUD-CHG1-OPT, IA-3-OPT, CC-12, CC-18, CC-20, AH-06 (moved from fix C) and PREC-5/7/8/17/18 (precision plan); only += those + R3-11, R4-04 and their unowned closure (R3-12, IA-12b, R3-26, R3-19, R4-09, SC-5, SC-6, SC-7); every split narrowing in specAppend removed or replaced by in-scope text; split cut edges restored for unmerged dependents except where the backtest-first cut must hold (PREC-6 and PREC-19 never wait for R4-21; PREC-3 never waits for R4-25; R4-25 and R3-26 / R3-11 reversed into FILES_ONLY merge-order links); criticalPath = every zero-slack part of the first goal (the backtest-first proof path PROVEN-GATE / PREC-19 and the options proof leaf PREC-8, all depth 6): 24 parts; decision blocks D-6660..D-6899 recorded per package (state/options-20261020/decision-blocks-add.json). Evidence: state/options-20261020/ (decisions.json, opt-packages.json, critical-path.md, dry-check.md).

_note_nofresh_20261023: 23 Oct 2026 no-fresh-root cut (operator ~07:45 IST; proposed D-7135): a new GDFL or any vendor link is an incremental re-import (patch) into the existing store, append-only; never a fresh-from-scratch rebuild. No part dropped; 10 narrowed (R4-16, CC-3, PREC-5, R3-22, R4-21, R4-11, IA-10c, PREC-17, AH-06, IA-4) and 7 kept with an amendment (CC-18, PREC-19, VKI-1, CC-1, PREC-6, PREC-13, WP-99) through .specAppend. Proof 3 is the one-month build-twice determinism test (S-42). state/nofresh-20261023/README.md

````

### 3.4 Where the part came from (verbatim)

Source: `state/followups-20261020/replan-critic.json`, `[1].items[3]`.

````text
{
  "gap": "The data-safety preflight half of IA-3 belongs to neither replan. IA-3-OPT explicitly leaves it out, and the one-click CONTEXT says 'IA-3 is merged: nothing to drop there'. state/fix/round2/fix-IA-3.json leaves these not_fixed: FM-060 (critical: a wrong or empty source root such as BRUTEX_GDFL_NFO_ARCHIVES closes every day 'refused day_not_held'); RA-323 source_changing (a zip still being copied, which is the Dropbox re-delivery case); RA-322 year_zip_empty (the options year zips); RA-238 member_too_large; RA-207 census room; RA-306, 314, 340 and 056; D-4802, D-4850 and D-4864; IA-GR-07/15/20/26/34; FM-096; FM-133. These are loud refusals and verification, which the one-click KEEP list explicitly keeps.",
  "severity": "high",
  "fix": "Add a new package IA-3b to the build launcher, covering the preflight refusals only (the source cache and watching can be dropped with the unattended layer). Deps: IA-3-OPT. Place it in the first goal, or at least before R4-11. Give it its own decision block from D-6900 on.",
  "owner": "build launcher (new package IA-3b)"
}
````

Source: `state/decision-blocks.json`, the block entry (lines 168-171).

````text
    {
      "block": "D-7000..D-7009",
      "owner": "IA-3b (20 Oct follow-ups; also lands the design ids D-4802, D-4819, D-4850, D-4864 and the critic id D-4908)"
    },
````

## 4. What specStep points to, and what the spec cites

### 4.1 The fix-plan work package

`jq '[.work_packages[] | select((.name | split(" ")[0]) == "IA-3b")][0]' state/board/fix-plan.json`
answers **null**: `fix-plan.json` has no IA-3b entry (nor any IA-3 entry; the IA packages were
planned in the design, not the fix plan). D-number assignments do not come from WP-02 for this
part: its block is D-7000..D-7009 (section 7).

### 4.2 Confirmed audit findings

`state/board/audit-result.json` (key `confirmed`, 387 findings, ids `DB-*`, `SC-CLOSE-*`,
`CMP-*`) holds **no finding for IA-3b**: the spec names no `DB-*` id. The findings this part
answers are IA-3's open ones, recorded in IA-3's own build report, copied next.

### 4.3 IA-3's open findings that this part closes (verbatim)

Source: `state/fix/round2/fix-IA-3.json` (IA-3's build report, branch `fix/db-IA-3`, head
65b3fc017, base `feat/db-fix` aab82424b), `.findings[]` entries the IA-3b spec names, each as
`id [status] how: ...`. The ones not named by the IA-3b spec (D-4814, D-4867, FM-043 are
IA-3-OPT's; IA-GR-34 is IA-10a's; RA-055, RA-200, RA-333 are dropped with the source cache and
watching; RA-011 is R3-19's; RA-157 is the calendar's) are listed after, for reference only.

````text
D-4815 [partial] how: gdfl_dayzips::DayZips is a CmSource and an NfoSource over <ROOT>/{INDICES,STOCKS,OPTIONS}/<MON_yyyy>/<name>.zip. NOT DONE: it is not yet a source WORD (gdfl_run's SourceWord/SourceId, BRUTEX_GDFL_DAYZIPS, Origin's measurement): that is the run's door in IA-1's file and the automation piece.
    evidence: loose_day_zips_import_like_the_archive_day_they_copy; real data: loose 2026-09-25 NIFTY close equals the tick store's (listing=350a71cc...ac22, seconds=22500)
D-4819 [partial] how: Both charter digests of FORMAT.md are recorded in gdfl_tickstore.rs's module doc. NOT DONE: the runtime digest compare and the run-line field (the source cache's work, automation piece).
D-4802 [not_fixed] how: Preflight (census_room_short, root checks, Origin/day-table cache): the automation piece, deferred by the operator's 18 Oct backtest-first order.
D-4850 [not_fixed] how: The v4 census preflight model: automation piece, deferred.
D-4864 [not_fixed] how: legacy_run_may_be_live cutover guard: automation piece, deferred.
IA-GR-07 [not_fixed] how: Preflight (wrong source root before any journal line): automation piece, deferred.
IA-GR-15 [not_fixed] how: Preflight census room: deferred.
IA-GR-20 [not_fixed] how: Legacy-run guard: deferred.
IA-GR-26 [not_fixed] how: v4 preflight compaction model: deferred.
FM-060 [not_fixed] how: Preflight wrong-root refusal: deferred.
FM-096 [not_fixed] how: v4 preflight: deferred.
FM-133 [not_fixed] how: Days before a source's first day named once (plan): deferred.
RA-304 [partial] how: The tick store opens a day file once and reads every block through that one handle (a rename-published replacement cannot mix two versions). NOT DONE: the identity/length compare at the end and source_changing.
RA-238 [not_fixed] how: member_too_large (256 MiB cap before any read) is not built; not in the backtest-first list. Today's caps stand: 64 MiB directory, 1,000,000 rows.
RA-056 [not_fixed] how: Preflight lock-file check (D-4908): deferred.
RA-207 [not_fixed] how: Census room check: preflight, deferred.
RA-306 [not_fixed] how: open_files_short preflight: deferred.
RA-314 [not_fixed] how: store_read_only preflight: deferred.
RA-322 [not_fixed] how: year_zip_empty preflight and the empty-archive matrix rows: deferred.
RA-323 [not_fixed] how: source_changing for a zip still being copied: not built.
RA-340 [not_fixed] how: store_not_local preflight: deferred.
RA-103 [not_fixed] how: Preflight fixed checks and C-GR-05: deferred.
RA-057 [not_fixed] how: Plan preview counts: deferred.
````

One more entry of the same source is not named by the IA-3b spec, but its unbuilt half holds
the matrix rows whose tests the IA-3b spec names (A9, A11, A21, A22 of design table A, and the
new A23, B28 and B28a), so it is copied too. At e851da1ca the proved rows sit in
`docs/04-invariants.md` lines 8220-8237 (`IA-MX-A4` .. `IA-MX-T11`, the IA-3 section); the new
`IA-MX-*` rows go beside them:

````text
IA-MX-ABCDET [partial] how: Proved rows: A4, A13, A16, A17, A20, B7, B8, B12, B13, B19, B22, B27, C2, C3, D23, T9, T10, T11 (docs/04-invariants.md IA-3 section). Not built: the rows that belong to preflight, the source cache, the _v3 reader and the empty-archive rows (RA-322).
    evidence: docs/04-invariants.md section 'Sources: per-day container faults...'
````

Not this part (reference only), same source:

````text
D-4814 [not_fixed] how: The _v3 ledger reader: options work, parked by the operator's 18 Oct order.
D-4867 [not_fixed] how: BRUTEX_GDFL_V3_LEDGER: options work, parked.
IA-GR-34 [not_fixed] how: Calendar-horizon warning: deferred (automation piece).
FM-043 [not_fixed] how: An options day zip in another year's zip: options work, parked by the operator's 18 Oct order.
RA-055 [not_fixed] how: Source cache and watching the newest month folder: automation piece, deferred.
RA-200 [not_fixed] how: Source change event and archive scrub: source cache, deferred (NEW-32 owns the scrub).
RA-333 [not_fixed] how: Change-time key in the source cache: deferred with the cache.
RA-011 [not_fixed] how: IA-3's measurement part (L-IA-11 at real size): not done.
RA-157 [not_fixed] how: Calendar-horizon warning: deferred.
````

IA-3 report notes (verbatim, `.notes`; the real-data roots a second agent used are Mac paths):

````text
RESUMED an earlier cut-off attempt: it left 4 WIP commits plus uncommitted test-expectation updates; I judged every hunk, kept them, and squashed everything into one commit 65b3fc017 on fix/db-IA-3 (parent feat/db-fix aab82424b). The pre-squash history is kept on branch backup/IA-3-wip-20261020.

ADDED THIS RUN: the failure matrix's named tests at import level (FM-023, FM-026, FM-040 x2, FM-044, FM-119), each proving the faulty day stays open across two runs while the next day closes done; the leap-day import test (RA-305, import half); C-GI-04 in gate 8's ratio bench (RA-111); measured times replacing UNVERIFIED in docs/06-limits.md; the new tests cited in docs/04-invariants.md.

NIFTY FIRST, real data (ignored tests in release, read only, writes only to scratch stores under the system temp dir): NIFTY spot 2024-04-01, the 2024-11-01 special session (seconds=3577), BANKNIFTY 2020-03-23, RELIANCE 2024-04-01, a NIFTY options day 2024-04-04 (files=973, seconds=1,869,196, entries=11065) and the loose 2026-09-25 NIFTY day each closed with identical files, seconds, skipped and listing digest from two sources (CM zip vs tick store; options year zip vs tick store; loose zip vs tick store). A second agent repeats with: BRUTEX_GDFL_TICKSTORE=/Volumes/WD_BLACK/brutex/tickstore-data BRUTEX_GDFL_CM_ARCHIVE='/Volumes/WD_BLACK/NSE (Stock+Indices)_01.09.2018 to 24.09.2026_Tick.zip' BRUTEX_GDFL_NFO_ARCHIVES=/Volumes/WD_BLACK/Options BRUTEX_GDFL_DAYZIPS=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tools-work/update-20260925-30/downloads cargo test --release -p pull --lib -- --ignored --nocapture --test-threads=1 ia3_lookup_costs real_days_close_with_the_same_listing_from_two_sources the_real_containers_open_with_no_prefix_and_print_their_facts

IA-0b CONTAINER FACTS (printed by the_real_containers_open_with_no_prefix_and_print_their_facts, for the charter): CM outer zip: 4,209 entries, prefix 0, 1,997 index days and 1,997 stock days 2018-09-03..2026-09-24, 0 refused days, census ignored=215 misfiled=0 invalid=0, 3,994 members with a data descriptor (GP bit 3), 0 encrypted, 0 not stored. Options year zips 2018..2026: prefix 0 each; day zips 79, 245, 252, 248, 248, 246, 249, 249, 181; every day zip has a data descriptor; 0 encrypted; 0 not stored. Tick store: cm 4,002 day files, options 2,001, 0 misfiled.

SHADOW: my release build served on 127.0.0.1:8402 over a fresh shadow (listener pid matched test-server.pid), GET only, then stopped. The package changes no route; /imports.json?feed=gdfl was measured as a regression check. The new close fields cannot show on the shadow because the real journal predates them and no import may run there; the import was proved on scratch stores from the real sources instead.

SHARED POOL HAZARD (for the main session): cargo writes dep-info with paths relative to the package root and hashes path packages relative to the workspace, so in a target dir shared by several worktrees a crate another worktree built LATER than this tree's files were edited counts as fresh. My first 'cargo test -p pull' finished in 0.22 s and ran another worktree's pull test binary (no ia3 tests in it). Every gate above was re-run after touching all of this tree's crates/**/*.rs inside the pool lock, and the log shows each crate recompiled from wt/P-IA-3. Other packages using tgt/pool-* may have the same false-green risk. I also ran the whole chain in one build slot (CARGO_QUEUE_REAL pointed at a script running the six cargo commands in order) after 20+ minute queues per command; still one slot, never more.

LEFT IN PLACE (delete nothing): shadow dir /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/shadow/IA-3; served binary copy /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/ia3-serve-20261020; real-data scratch stores /var/folders/bk/hwtgtk4n18lcqxnc4kjj4fs40000gn/T/brutex-gdfl-ia3-real-87105-0-1792466321567300000; bench scratch /var/folders/bk/hwtgtk4n18lcqxnc4kjj4fs40000gn/T/brutex-tsw-bench-71644-1792466121681498000 and brutex-bench-dropbox-71644-1792466124013870000; test scratch dirs brutex-gdfl-ia3-* under the same temp dir; branch backup/IA-3-wip-20261020; logs in this session's scratchpad.

NOT DONE (by the operator's backtest-first order, or named per finding): the whole automation piece (preflight as a press gate: source_not_configured, near-miss year-zip names, census_room_short with the v4 model, legacy_run_may_be_live, open_files_short, store_read_only, store_not_local, year_zip_empty, the calendar-horizon warning; the source cache with Origin measurements for /gdfl/sources.json and plan.json; the plan's day counts and predicted holes); dayzips as a source WORD (gdfl_run.rs is IA-1's file); the _v3 reader and the nfo_cause arms (options, parked); FM-043 (options); member_too_large (RA-238); source_changing (RA-304/RA-323); FM-044's naming of both members and CRCs; FM-119's unresolved=<n> field. Limits to name in docs/06-limits.md are landed: L-IA-8 (part), L-IA-59, L-IA-60, L-IA-61.

DECISION NUMBERS: D-4815, D-4817, D-4818, D-4828, D-4834, D-4835 and D-4866 are the design's own numbers (none was in the ledger at the base); D-5903 was the next free number after the head D-5902 at commit time and must be re-checked at landing (amendment A6).

SELF-CHECK done: full diff read; hot-path loops named (the archive's two in-memory passes over its directory, once per archive; the dayzips and tick-store walks once per source object, capped; the listing digest O(entries) once per day); no store mutation (journal lines are appended); no look-ahead; every container fault refuses loudly by name and never closes a day; no float prices; Rust only; every new test asserts; fixtures invented (prices like 11111.25 that no index printed on those days); no real vendor row committed. Gates were run once after the last code edit; the only later edit was prose in docs/06-limits.md's IA-3 section (no test reads that section).

````

### 4.4 The e2e failure matrix rows this part closes

Source: `state/fix/e2e-failure-matrix.json` `.rows[]` (24 Oct 2026). **No row names `IA-3b` as
owner.** The three rows below are owned by `IA-3` and the IA-3b spec closes them ("MATRIX ROWS.
FM-096 and FM-133 close by their tests here. FM-060 closes by AF-01's two tests re-run here,
together with item (1)'s year-zip tests."). Each row in full, every field as the file holds it
(`jq '.rows[] | select(.id=="FM-060" or .id=="FM-096" or .id=="FM-133")'`). None of the three
has a `status_note` field: the status note is `owner_note` (FM-133 has neither `owner_note` nor
`related`):

````text
{
  "id": "FM-060",
  "stages": [
    "S4",
    "S9"
  ],
  "scenario": "The source root is wrong: a typo in BRUTEX_GDFL_TICKSTORE, an empty directory, an old copy without FORMAT.md, or the drive mounted as 'WD_BLACK 1'. Every day is NotFound and closes `refused day_not_held`. report.failures stays empty, so the cli exits OK (0). A wrapper or the timer reads success for a run that imported nothing.",
  "how": "A human edits an env var, or macOS remounts the drive under a new name.",
  "expected": "Refuse before any day if the root is not a tick store, a CM outer zip or a directory of `<yyyy>.zip` (`wrong_source_root{found}`), with no journal day lines. Any run with days_missing>0 and files==0 exits FAILED, and /ingest is red.",
  "test": "a_wrong_source_root_is_refused_before_any_day_is_journalled (IA-GR-07) and a_run_that_read_no_file_never_exits_ok",
  "status": "not_handled",
  "evidence": "HUMAN-004: crates/pull/src/gdfl_tickstore.rs:559-562 turns NotFound into Ok(None). crates/pull/src/gdfl_import.rs:4506-4511: day_not_held 'names no failure of its own'. crates/cli/src/gdfl_import_command.rs:412-413: failures empty means OK, and :281-288 root() checks only that the variable is non-empty. Origin::measure (gdfl_import.rs:1114) only warns 'unmeasured'. Nothing in crates matches wrong_source.",
  "severity": "critical",
  "owner": "IA-3",
  "sources": [
    "HUMAN-004"
  ],
  "verdicts": {
    "HUMAN-004": "not_handled/critical"
  },
  "owner_note": "IA-GR-07: a_wrong_source_root_is_refused_before_any_day_is_journalled.",
  "origin": "hunters"
}
{
  "id": "FM-096",
  "stages": [
    "S4",
    "S10"
  ],
  "scenario": "A v4 partition reaches capacity on a nearly full disk, so compaction cannot write its new epoch, the census is refused and the import stops. The day's bars are on disk one day ahead of the census. On a v3 single file the 2,097,152-row ceiling stops the run in the same way (CensusFull).",
  "how": "Disk below the size of one census epoch (the census was 268 MB) while an import compacts.",
  "expected": "A preflight models the compaction and the free space and refuses before any journal line (IA-GR-15, IA-GR-26), raising a disk alert.",
  "test": "the_v4_preflight_models_the_compaction_the_writer_makes",
  "status": "partly",
  "evidence": "CONC-030: crates/pull/src/gdfl_import.rs:150-170 (a census refusal stops the run); no preflight exists (git grep)",
  "severity": "medium",
  "owner": "IA-3",
  "sources": [
    "CONC-030"
  ],
  "verdicts": {
    "CONC-030": "partly/medium"
  },
  "owner_note": "IA-GR-26: the_v4_preflight_models_the_compaction_the_writer_makes.",
  "origin": "hunters",
  "related": [
    "FM-617"
  ]
}
{
  "id": "FM-133",
  "stages": [
    "S4",
    "S7"
  ],
  "scenario": "An import is asked for days at the edge of the window, before the source's first day (2018-01-01 to 2018-09-02 for CM) or after its last. Each such day writes one `refused day_not_held` line, and the line is not written again on a re-run with the same reason. That is safe, but the plan does not say where the source begins, so the window looks owed.",
  "how": "A rebuild from scratch, or a Resume over the full 2018-2026 window.",
  "expected": "The plan states each source's first and last day and counts days outside them as `outside_source`, not as holes.",
  "test": "days_before_a_sources_first_day_are_named_once",
  "status": "partly",
  "evidence": "CONC-053: crates/pull/src/gdfl_import.rs:205-215 (refused lines deduped), :2362 (refused_close)",
  "severity": "low",
  "owner": "IA-3",
  "sources": [
    "CONC-053"
  ],
  "verdicts": {
    "CONC-053": "partly/low"
  },
  "origin": "hunters"
}
````

One row owned by another part names IA-3b in its `status_note` (reference only: it is not this
part's to close, but it says the scratch builds of the no-fresh cut rely on this part's preflight
running on **every** run, scratch stores included):

````text
FM-235 (owner E2E-07, status refused, severity high), status_note: "Refused on purpose under the no-fresh-root cut D-7135 (23 Oct 2026): Refused on purpose under the 23 Oct no-fresh-rebuild rule: no ~2 TB rebuild ever runs beside production. Proof 3's two one-month scratch builds (about 21 GB each by R4-16's own figure, an extrapolation) and repair scratch stores go through the same import preflight and free-space floor as every run (IA-3b, R4-29's PERF-13 floor, R3-05's setting)."
````

Other IA-3-owned rows (not this part: FM-023, FM-026, FM-040, FM-044 and FM-119 were IA-3's per-day faults, FM-043 IA-3-OPT's), status only:

````text
FM-023 partly
FM-026 partly
FM-040 partly
FM-043 partly
FM-044 partly
FM-119 partly
````

### 4.5 The rules audit

`grep -n '^## .*IA-3b' state/design/rules-audit-20261011.md` finds **no IA-3b section** (the
audit is of 11 Oct, before this part existed). Its gaps are the preflight rows of the **IA-3**
section, which the IA-3b spec takes over (RA-056, RA-103, RA-207, RA-238, RA-304, RA-306,
RA-314, RA-322, RA-323, RA-340, and the count half of RA-057). Both sections are copied whole;
rows RA-055, RA-200, RA-333 (the source cache and watching), RA-305 (IA-3's leap day) and
RA-111 (IA-3's C-GI-04) are not this part.

Source: `state/design/rules-audit-20261011.md`, lines 487-516 (## IA-3: preflight and sources)

````text
## IA-3: preflight and sources

Leads 15 rows (1 high). Also named in 3 rows led by another owner.

| row | gap | rule | where (file:line or design line) | what to build or prove | test or bench to add |
|---|---|---|---|---|---|
| RA-238 high, does not meet it yet. Machine and data: one source file that states (truthfully) an enormous size, for example 100 MB packed that unpacks to tens of GB; and the largest day the importer will read | No cap by name on one file. The child is killed for memory mid-day, is resumed once, dies again, then waits for a press, with 8 to 12 workers times three kinds. The margins of the caps that do exist are not in the limits file. | One common, dynamic, scalable path + Every extreme case | pull/gdfl_archive.rs:586-616 reads the packed bytes whole and unpacks to the stated length; the only caps: the zip directory's 64 MiB (:82) and 1,000,000 rows after unpacking. The tick-store reader does the same (gdfl_tickstore.rs:115-119). | Refuse 'member_too_large' before any read (cap 256 MiB), the same for a tick-store file's rebuilt size. Matrix row B28 with a test. A limits row with the measured margins and one synthetic day ten times today's size in the bench. (owner line: IA-3 (preflight and the member-bytes seam); IA-13 (the bench)) | Matrix row B28 with a test. A limits row with the measured margins and one synthetic day ten times today's size in the bench. |
| RA-055 medium, partly. Where the source files are, what days they hold, and measuring the archives | Nothing measured; the first measuring of each archive was on the start path. The design misses a new day dropped into a folder. Source locations are read once at start, so moving the drive means a restart. The source kinds are a fixed list. | O(1) at p99, measured + One common, dynamic, scalable path | Design lines 916-930, 4138: five settings read at start; spans measured from the files; opening an archive is a pass over 4,209 entries, 'UNVERIFIED as a time' (limit L-IA-8; bench C-GR-12). Critic IA-3-2, IA-10a-2: two faults. Not built. | Build with critic D-4909 (watch the newest month folder) and D-4914 (measure on first use, on the bounded pool; bench C-GR-12 cold and warm). Tests: a new day file, and a new year zip, show without a restart. Source locations on a page. (owner line: IA-3 with critic IA-3-2 (D-4909); IA-10a with critic IA-10a-2 (D-4914)) | Build with critic D-4909 (watch the newest month folder) and D-4914 (measure on first use, on the bounded pool; bench C-GR-12 cold and warm). Tests: a new day file, and a new year zip, show without a restart. |
| RA-056 medium, partly. Checks before a run starts (preflight) | One check grows with history as designed; the refusal path decodes up to 1 GiB. Not built. | O(1) at p99, measured | Design line 4124: fixed checks plus one listing read (up to 26,820 entries); the whole census is decoded only on a refusal; bench C-GR-05. One check, 'has any GDFL run ever claimed', scans a ledger growing with every pull (critic IA-3-1). | Critic IA-3-1: decide it by whether the lock file exists, one file check (D-4908). State the refusal path's cost in the limits row. (owner line: IA-3; critic IA-3-1 (D-4908)) | None named yet: name a test or a bench with the fix. |
| RA-200 medium, partly. The bought archive changes or rots after a day was imported from it | For everything imported so far, a later change in the bought files cannot be detected at all; for new imports it is found only when someone runs a cold check. Nothing re-reads the archives for rot, and they are the only copy. | Saved, logged, searchable, on a page | Design section 9: a cold check compares each member's checksum with the one recorded at landing (IA-GV-24). On the real store no existing month has that record (verify-WP-81-r1). No event, alert or scrub for a changed source. | An audited event and alert when a source's size, time or identity changes between two plans. A slow, resumable archive scrub against the zip's own checksums, its last pass on a page. Record a checksum list for the existing days once. (owner line: IA-3 (the source cache), IA-6 (the two flags), IA-10a (the route); NEW-32 'Forever files and indexes watched' (the event, the alert and the archive scrub)) | None named yet: name a test or a bench with the fix. |
| RA-207 medium, partly. The census is near or at its size ceiling | Capacity is a number in a table; there is no gauge and no early alert (for example at 90 percent), so the first notice is a stopped run. | Saved, logged, searchable, on a page | Today: 'census refused; run stopped' is an audited error (gdfl_import.rs:3290); the Import panel shows capacity used and left. Design: the plan shows room now and after, and refuses 'census_room_short' before any work (line 4339). | A capacity bar per segment on /ingest and the health page; one standing alert while any segment is above 90 percent, raised again only after it was acknowledged and is still true. (owner line: IA-3 (the room check), IA-10a (the plan line); NEW-29 'IA-19: health page' (the gauge); NEW-25 'Run watcher and new alert kinds' (the early alert)) | None named yet: name a test or a bench with the fix. |
| RA-304 medium, partly. Data: the tick store's own writer replaces a day file while that day is being read | The promise rests on a document, not on a test. A day read by path twice (list, then read) can mix two versions. | Every extreme case | Design row T9 (line 3961) and section 9 (line 4061): the tick store's files are 'published by rename', marked UNVERIFIED for the tool's code. | Open each day file once and read through that handle; note its identity and length at open and compare at the end: 'source_changing' when they differ. Test. (owner line: IA-3) | Test. |
| RA-305 medium, partly. Time: leap day. A day file named 29022024 (a Thursday and a monthly expiry), 29022020 (a Saturday), and 29022023 (a date that does not exist) | No test carries a leap day through the whole chain (import, checking, month repair, the 366-day calendar page). The leap rule is written at least twice in the code, so the two can drift apart. | Every extreme case | Name decoding is tested (pull/session.rs:226, :313; gdfl_nfo_attack_tests.rs:760; core/instrument.rs:164). Design: 'leap' appears 0 times; matrix C (lines 3800-3817) has no leap row. The year index has 366 day slots (line 2951). | Matrix row C15 with test 'the leap day imports, verifies and repairs, and 29 Feb of a plain year is refused by name'; one test that both leap rules agree for every year 1990-2100. (owner line: IA-3) | Matrix row C15 with test 'the leap day imports, verifies and repairs, and 29 Feb of a plain year is refused by name'; one test that both leap rules agree for every year 1990-2100. |
| RA-306 medium, partly. Machine: too many open files when the app is started the normal way (limit 256) rather than from a tuned shell | No test runs with the limit at 256 while three kinds import and the app serves pages. A 'too many open files' error is filed under a general store error instead of its own name. | Every extreme case | Design line 4222 (L-IA-35) and IA-GR-08 count open files at 1 and 12 workers. The test shell's limit is 1,048,576, so tests never meet 256. WP-39's branch now asks again after a failed open (bars_sort.rs:396-407), unverified and unmerged. | Preflight reads the limit and refuses 'open_files_short{have, need}'. A test run under limit 256. Second agent re-checks WP-39's repair of the memo. (owner line: IA-3 (preflight); WP-39 (the memo follow-up)) | A test run under limit 256. |
| RA-314 medium, partly. Machine: the whole drive is mounted read-only (after an unclean eject) | A press on a read-only drive passes preflight and fails at its first write with a general error, after a run id was already taken. | Every extreme case | Second agents tested read-only FOLDERS (verify-WP-12d-r1 attacks 9 and 11; verify-WP-39-r1 attack 8: refused by name). Design line 1906 maps a read-only error to the general 'store io' path. Preflight (section 5.2) has no write check. | Preflight opens the lock file for writing and refuses 'store_read_only' before any child starts. Test on a read-only disk image. (owner line: IA-3) | Test on a read-only disk image. |
| RA-322 medium, not proved yet. Data: a valid zip with nothing inside (a year zip or a day zip with zero entries) | An empty YEAR zip would make every day of that year 'the source lacks this day', a hole the plan says a re-run cannot fill, with no louder word. | Every extreme case | Design: 'zero entries' and 'empty zip' appear 0 times; matrix rows A and B have no empty-archive row. The nearest rule is 'no_wanted_file' for a held day with nothing wanted (line 1845). | Preflight refuses 'year_zip_empty{year}'. Matrix rows A23 and B28a with tests for an empty year zip, day zip and tick-store day. (owner line: IA-3) | Matrix rows A23 and B28a with tests for an empty year zip, day zip and tick-store day. |
| RA-323 medium, not proved yet. Data: a zip that is still being copied when the run reaches it (it grows while it is read) | If the refusal closes the day, the finished copy is never read. A file whose size changes between listing and reading can also mix two states. | Every extreme case | Design: 'growing' and 'still being copied' appear 0 times. Row B18 (line 3789) refuses a zip with no end record; row B27 (line 3798) refuses a loose zip's listing by name. Neither says whether that refusal closes the day. | Rule: a source file whose size or modified time changed between its listing and its last read is 'source_changing': the day stays open, never closed. Test: 'a zip that is still being copied is never closed'. (owner line: IA-3) | Test: 'a zip that is still being copied is never closed'. |
| RA-333 low, partly. Time: the clock steps back while the app is deciding whether a source file changed | A file rewritten in the same (stepped-back) second with the same length reads as unchanged, so the plan keeps an old day list. | Every extreme case | Ids, rates and elapsed times are covered (IA-GR-11, IA-GR-28); log stamps are clamped (telemetry/sink.rs:3383-3387). But the source cache tells 'changed' by (device, inode, modified time, length) (design lines 791, 849; critic IA-3-2). | Add the file's change time to the key, and re-measure whenever 'now' is earlier than the last measurement. Test with a fake clock. (owner line: IA-3 (with critic D-4909)) | Add the file's change time to the key, and re-measure whenever 'now' is earlier than the last measurement. Test with a fake clock. |
| RA-340 low, not proved yet. Machine: the store sits on a network share used by two Macs at once | File locks over a network share do not give the same promise, and nothing refuses a store that is not on a local drive. | Every extreme case | Every exclusion in design section 5.3 is a file lock on a local drive; nothing refuses a store that is not on one. Measured today: the store's drive is a local APFS volume. | Preflight refuses 'store_not_local' from the drive's own flags. Test with a fake answer. (owner line: IA-3) | Test with a fake answer. |
| RA-103 low, designed to meet it, not built yet. Small fixed checks the design adds around a run: build mismatch, census room, hole reasons, predicted holes | Not built; the 776 header reads of a first plan are not given a time. | O(1) at p99, measured | Design lines 4135, 4164, 4165, 4172: one file check of the running program; 2 small reads of the census header; one header read per (widened member, month), at most 8 x 97. Benches C-GR-05 and C-GR-10. Not built. | Build with IA-3 and IA-10a; C-GR-05 prints the first (cold) plan and a repeated one separately. (owner line: IA-3, IA-10a) | C-GR-05 prints the first (cold) plan and a repeated one separately. |
| RA-111 low, meets the rule, measured. Finding one day, or one file of a day, inside an archive | The day-table bench C-GI-04 is promised and not yet in the code (critic IA-1-5 gives it to IA-3). | O(1) at p99, measured | 06-limits.md:15824-15825 (WD_BLACK, 5 Oct): a contract's file p50 500 ns, p99 792 ns over 11,524 tickers, flat at 1x, 10x, 100x (bench C-GI-01); a share's file 42 to 166 ns (C-GI-02). 'Find one day' rests on a named test only. | Land C-GI-04. (owner line: IA-3) | Land C-GI-04. |

Also named in:

| row | gap | this owner's part (the row's owner line) | led by |
|---|---|---|---|
| RA-011 high. Publishing the day's new instrument-months to the census (the store's list of what it holds) at each day's close | Closing a day costs more with every day already imported, on one thread, under the census lock, and nobody has timed it at real size (367,189 entries, about 10,428 more a day). WP-16's fast days were measured into empty stores. | NEW-12 'Census publish costs only new rows' (keep the list in memory across the days of a run; IA-1's day loop with WP-16's census code); IA-3 (the measurement and limit row L-IA-11, critic IA-1-5) | NEW-12 |
| RA-157 high. The trading calendar and holiday list stop on 4 Oct 2026, which has already passed | No day after 4 Oct 2026 can be imported or pulled today, and no page says so up front: the alert is unbuilt and the fix has not started. Every year a programmer must type next year's holidays into code. | WP-62 (extend now; no commits); IA-3 and IA-10a (the warning and alert kind 4, D-4884); IA-12a (the warning on the page); IA-14 (the yearly step); NEW-35 'Reference tables as store data' (the calendar as a sourced data file) | WP-62 |
| RA-057 medium. The plan preview table on /ingest (/gdfl/plan.json) | Not built. Grows with the range asked for, has no cap on the range and no stated p99 budget, is recomputed on every change of the form, and shares the pool of 8 with about a dozen page routes. | NEW-13 'Plan and history from counters' (a new rule inside IA-10a and IA-3); IA-10a, IA-3 (build) | NEW-13 |

````

Source: `state/design/rules-audit-20261011.md`, lines 1253-1262 (## Critic resolution D-4908)

````text
## Critic resolution D-4908: "Has claimed" is the lock file's existence: one metadata (lands with IA-3)

Leads 0 rows (0 high). Also named in 1 rows led by another owner.

Also named in:

| row | gap | this owner's part (the row's owner line) | led by |
|---|---|---|---|
| RA-056 medium. Checks before a run starts (preflight) | One check grows with history as designed; the refusal path decodes up to 1 GiB. Not built. | IA-3; critic IA-3-1 (D-4908) | IA-3 |

````

### 4.6 The 16 Oct cloud spec

`origin/cloud/db-base-20261016:reports/db-specs-20261016/IA-3b.md` **does not exist** (IA-3b
was created on 20 Oct). The parent's file `IA-3.md` there adds only IA-3's own round-3 item
(DB-A49: record the files a day skipped), which IA-3 built and merged; it adds nothing to this
part.

### 4.7 The design text the spec cites (verbatim)

The IA-3b spec cites the design `state/design/ingest-automation.md` (6,154 lines, 10-11 Oct
2026) by section and line: sections 2.1 (the measured containers), 5.2 (sources and preflight),
5.8 (new refusals and causes), 8 rows A9, A11 (and new rows A23, B28, B28a, which do not exist
yet: this part adds them), 10.1 (C-GR-05), 10.2 (L-IA-35), the decisions D-4802 (:4572),
D-4819 (:4589), D-4850 (:4620), D-4864 (:4634), D-4815, D-4834, D-4866, and the tests IA-GR-07,
-08, -15, -20, -26, -27. The parent IA-3's row of 16.2, its lane and order, and its 16.3 row are
copied for the file-ordering rules. **Where the design names line numbers in code, they are of
10-11 Oct; section 9 below gives the e851da1ca facts, and the code at the head is the fact.**
**Where the IA-3b spec and the design differ, the IA-3b spec wins** (it is later and re-scoped by
D-6900): notably the source cache, `plan.json`, the free-space line, folder watching and the
calendar-horizon warning are not this part; C-GR-05 lives in `crates/pull/benches/ratio/gdfl_preflight.rs`,
not in the api bench the design's section 11 names; and the precedent "`disk_route.rs:142` runs
`/bin/df -Pk`" is false at the head (section 9).

#### Section 2.1 Containers

Source: `state/design/ingest-automation.md`, lines 346-362

````text
### 2.1 Containers

| Container | Path | Measured shape |
|---|---|---|
| CM outer zip | `/Volumes/WD_BLACK/NSE (Stock+Indices)_01.09.2018 to 24.09.2026_Tick.zip` | 128,918,864,518 bytes, ZIP64, 4,209 entries. 215 are directories, including a root `/`. 1,997 `INDICES/<yyyy>/<MON_yyyy>/GFDLCM_INDICES_TICK_<ddmmyyyy>.zip` and 1,997 `STOCKS/.../GFDLCM_STOCK_TICK_<ddmmyyyy>.zip`, all **stored** (method 0). Days run 2018-09-03 to 2026-09-24. Both trees hold the same day set. No misfiled, impossible or duplicate day names. **Every one of the 3,994 day zips sets general-purpose bit 3**: its local header carries CRC 0 and sizes 0, and a `PK\x07\x08` data descriptor follows its data. The 215 directories carry flag 0. The end of the central directory sits 76 bytes past `cd_offset + cd_size` (the 56-byte ZIP64 end record plus the 20-byte locator), so the archive has no prefix. |
| CM day zip (nested) | inside the outer | Members are deflate (8). Index days hold 50–141 files. Stock days hold 1,764–3,710 files. 1,995 of 1,997 index day zips and 1,962 of 1,997 stock day zips carry one folder entry. **Forward slashes in all 3,994 day zips**, including the 2 index and 35 stock day zips whose "version made by" is 2.0. 474 index files are spelt `.CSV`, all on 9 index days of Sep 2018. 48 index files are header only (≤ 80 B): `NIFTY MID100 FREE`, `NIFTY QUALITY 30` and `NIFTY SML100 FREE`, 8 days each spelling. No zero-byte file. No duplicate member, even ignoring case. Largest member: 15,279,399 B (index, 2026-09-18) and 2,874,937 B (stocks, 2022-07-11). Largest day zip: 81,398,055 B. |
| Options yearly zips | `/Volumes/WD_BLACK/Options/<yyyy>.zip`, 2018–2026 | 9.5 GB to 74.7 GB each, so ZIP64. 84–265 entries each: `/`, `<MON_yyyy>/` and 1,997 stored `<MON_yyyy>/GFDLNFO_TICK_<ddmmyyyy>.zip` (79, 245, 252, 248, 248, 246, 249, 249 and 181 a year). The day set equals the CM day set exactly. **All 1,997 day zips set bit 3** (local CRC and sizes 0, a `PK\x07\x08` descriptor after the data). `Options/2019.zip` also ends 76 bytes past `cd_offset + cd_size`. 2020–2026 zips have link count 2 (hard links). 2021–2026 are mode 0600. A stray `.DS_Store` (6,148 B) sits beside them. |
| Options day zip | inside a yearly zip | 2,758–26,820 files a day, 20,825,673 in all (10,428 a day on average). Every member is deflate. **1,996 days** use backslash paths `GFDLNFO_TICK_<d>\Options\<T>.NFO.csv`, made by zip 2.0, with no directory entries. **One day, 2026-09-24**, uses forward slashes, made by 3.1, with 2 directory entries. 7 days (Sep and Oct 2018) spell `.NFO.CSV`. No non-`Options` entry and no `Futures` folder. No member ≤ 80 B. Largest member 2,359,609 B (2019-05-30). Largest day zip 400,033,639 B, under 4 GiB. |
| Loose day zips | `work-20260925/tools-work/update-20260925-30/downloads/` | Layout `{INDICES,STOCKS,OPTIONS}/SEP_2026/<name>.zip` (`ls -R`, 10 Oct): 12 standalone day zips for 2026-09-25, 28, 29, 30, named `GFDLCM_INDICES_TICK_`, `GFDLCM_STOCK_TICK_` and `GFDLNFO_TICK_<ddmmyyyy>.zip`. Made by 3.1, forward slashes, stored directory entries, deflate files. Options days hold 17,428, 18,943, 20,054 and 10,364 files. **No source reads them today.** |
| Tick store BRTXTS01 | `/Volumes/WD_BLACK/brutex/tickstore-data` | `cm/{INDICES,STOCKS}/<yyyy>/<MON_yyyy>/*.bts`: 4,002 files. `options/...`: 2,001 files, through 2026-09-30, which is 4 days past the zips. 18,400 entries under the root. `FORMAT.md` sha256 is `d21139f5…5be9` (`shasum -a 256`, 10 Oct). **The charter records both digests**: `docs/00-charter.md:1065` pins `6edc62a7…3259` for format version 1 (frozen 2026-10-04), and `:1108`–`:1109` cite `d21139f5…5be9` as "FORMAT.md §6 as revised 2026-10-04". Only the `gdfl_tickstore.rs:7` module doc cites `6edc62…` alone, so that doc's pin is stale. The magic is unchanged. **The writer's own files sit in the same tree** (`find`, 10 Oct): 11 `.lock` and 11 `ingest-log.tsv` (the root, and per kind and year), plus `tickstore-bin`, `build-all.sh`, `supervise.sh`, logs and `RESUME.md`. `FORMAT.md:22` says a `*.bts.tmp` is incomplete and must be ignored, and `:112` that a day file is written to `.tmp`, fsynced, renamed and its directory fsynced (the spec's statement; the writer's code is not in this repository) |
| `_v3` extraction ledger | `/Volumes/WD_BLACK/NSE_Options_Tick/<yyyy>/_v3/` | **Not a tick store.** It is a separate tool's verify-and-place ledger: `manifest/<hex(day zip name)>.ok.tsv`, `ok/<hex>.ok` (crc, size and sha256 of the manifest), `owner/<dev>-<ino>`, `runs/<stamp>/{plan,list,verdicts,summary,unclaimed,walk}.tsv`, `fsid`, plus the lock files `lock.guard` and `place.lock`. **A manifest line has 6 tab fields**: the path as `<MON_yyyy>/<day folder>/Options/<T>.NFO.csv` (forward slashes, even where the day zip itself uses backslashes), the size, the CRC-32 in hex, a field that is 1 on exactly the 2 directory lines of 2026-09-24 and 0 on all 20,825,673 file lines, a field that is 0 everywhere, and the source zip. One line per zip entry, directories included: 2026-09-24's manifest has 16,467 lines for 16,465 files and 2 directories. Counts (manifests = ok markers): 79, 245, 252, 248, 248, 246, 249, 249, 181. That is 1,997, matching the zip days. 2018's summary reads `CLEAN mode=INTACT inner_zips=79 verified=79`. The folders it describes are gone: each year directory holds only `_v3`. |
| Extracted folders | `/Volumes/WD_BLACK/NSE_Tick_2018-09-01_to_2026-09-24` | Only `_verify_log` remains. The folders were retired on 2026-10-04 (D-2800). |
| Whole-tree rescans | `work-20260925/tools-work/rescan-20261001/results/cash/` and `.../optrescan-20261001/results/options-<yyyy>/` | Run on 1 Oct 2026 over the extracted folders before D-2800 retired them. `days.tsv` holds per-day counts of each check (`FMT-*`, `OPT-*`, `DAY-*`); `checks.tsv` defines them; `files-flagged.tsv` lists **at most 10 files a day per check plus "N more files" aggregate lines**, so a file count is taken from sampled plus aggregate lines, and a row count from `days.tsv`, never from a line count of `files-flagged.tsv`. |
| Placeholder zips | `project/GFDLNFO_TICK_06012019.zip`, `..._07012019.zip` (450 B each), `outer.zip` (1,042 B), all untracked | Each 450-B zip holds two directory entries and `Options/S.NFO.csv` (18 B, stored, a two-line text whose header is not GDFL's). 06012019 is a **Sunday**. 07012019 is **Monday 2019-01-07, a trading day** the CM and options archives also hold. `outer.zip` is **a streamed outer zip cut off before its own central directory**: a local header at 0 for the stored member `GFDLNFO_TICK_06012019.zip` (flag 8, sizes 0), that inner zip's records and its own end record at 483 (3 entries, CD 229 B at 199), a `PK\x07\x08` descriptor at 505, a second local header at 521 for `GFDLNFO_TICK_07012019.zip`, its end record at 1,004, a descriptor at 1,026, and no outer central directory. unzip's "576 extra bytes at beginning" is where the second inner zip starts (521 + 55). The name `outer.zip` matches no `GFDL*_TICK_<ddmmyyyy>.zip` pattern. These are test shapes, not data. |
| Name census | `state/name-census` | `appearances.tsv` holds 1,218,362 option names with their first day, last day and day count; they span 2,179,443 (name, month) pairs. `summary.json` gives F1M1 20,254,771 and F0M1 570,902 file-days. `answers-1246.txt` gives: the last monthly-form trade 2019-01-31; 0 leading-zero strikes; 64,477 decimal dated strikes; 0 weekend dated expiries; 0 BOMs in 33,949 sampled files (the whole-tree rescans find 0 BOMs in every CM and options file, `FMT-04`); 119 sampled files with the spaced `Open Interest` header. |
| Census (store) | `fresh-20260919/runtime/store/manifest/` | **9 Oct and 10 Oct morning**: `gdfl.man` 268,430,208 B and `gdfl.man.lock`; `zerodha.man`, `zerodha.man.lock`; no `v4/` directory, so the GDFL census was version 3, one file: (268,430,208 − 32,768) / 128 = 2,096,855 entries of `MAX_ENTRIES` 2,097,152 (`crates/pull/src/manifest.rs:374`), 297 free. **Since 10 Oct 08:54: version 4** (`manifest/v4/gdfl/{INDEX,CASH,FNO}/<epoch>.man`, one `.lock` per segment, the receipt `MIGRATED`); the v3 file stays beside it, frozen. Partition sizes and free room: §2.2 |

````

#### Section 5.2 Sources and preflight

Source: `state/design/ingest-automation.md`, lines 914-1001

````text
### 5.2 Sources and preflight (D-4801, D-4802, D-4815, D-4819, D-4850, D-4864, D-4866, D-4867)

**Sources are configured, not typed.** The api reads, once at start, the same
three variables the cli reads (`BRUTEX_GDFL_CM_ARCHIVE`,
`BRUTEX_GDFL_NFO_ARCHIVES`, `BRUTEX_GDFL_TICKSTORE`), plus two new ones:
`BRUTEX_GDFL_DAYZIPS` (the loose day zips) and `BRUTEX_GDFL_V3_LEDGER` (the
`_v3` extraction ledger root, `/Volumes/WD_BLACK/NSE_Options_Tick` on this
machine; evidence for the completeness check, never a source, D-4867). None
has a default (`CLAUDE.md` §8's rule for configuration). `/gdfl/sources.json`
lists each with its cached `Origin`, or names the variable that is not set.
**A request names a kind-agnostic source word** (`zip`, `tickstore` or
`dayzips`), as the cli does (`gdfl_import_command.rs:255`–`:258`).
`gdfl_run::resolve_source` resolves it per kind: `zip` is `cm_zip` for
indices and stocks and `nfo_zips` for options. The resolved id is what the
claim, the plan and the progress file show, so it always says which archive
was used. A path in a request body is refused `path_not_accepted`. **A press
is all or nothing**: when any of its kinds is refused at preflight or finds
its lock busy, no kind starts, and the answer lists each kind's refusal. The
api's `dry_plan` refuses before any child starts; a refusal or a busy lock a
child meets is answered through the two-phase start of §4: the other kinds'
children are sent `release` and exit with no journal line (round 4, BL-A).

**Preflight** runs in `gdfl_run::preflight` (its own file,
`gdfl_run_preflight.rs`) before any journal line. The api's route runs
`dry_plan` on the bounded store-read pool; the child repeats the checks in
`plan` before it takes a lock. Each check is bounded:

| Check | Cost | Refusal (code) |
|---|---|---|
| The source word resolves for every kind, and its variable is set | O(1) | `source_not_configured{kind, variable}` |
| The root exists and has the expected type (file for `cm_zip`; directory otherwise) | 1 `metadata` | `source_missing`, `source_is_a_directory`, `source_is_a_file` |
| `cm_zip`: the archive opens and its day table is built | O(4,209 entries), once per source (cached; re-measured on change). **UNVERIFIED as a measured time**: no bench times `Archive::from_source` (`docs/06-limits.md:11614`–`:11619`); bench C-GI-05 times it over the real 493,028-byte, 4,209-entry directory | `archive_malformed:<what>` (existing words), `archive_prefixed{bytes}` (new; defined from the end record, §8 B7) |
| `nfo_zips`: every year in [from, to] has `<yyyy>.zip`, readable (1-byte read of the EOCD tail) | O(years) | `year_zip_missing{year, near_miss:[≤4 names]}` (for example `2020.ZIP` or `2020 (1).zip`), `source_unreadable{path, mode}` |
| `tickstore`: `FORMAT.md` exists (≤ 1 MiB) and the first `.bts` of the range starts `BRTXTS01` | 2 reads | `source_root_unknown{found}`. `found` names a `_v3` directory when present: "an extraction ledger, not a tick store" |
| `tickstore`: `FORMAT.md` sha256 against the two digests the charter records: `6edc62a7…3259` (`docs/00-charter.md:1065`, format v1) and `d21139f5…5be9` (`:1108`–`:1109`, §6 as revised 2026-10-04) | 1 hash ≤ 1 MiB, once per source (cached) | Not a refusal. `Origin` records `format_sha256`. Today's file is `d21139…`, so it is pinned. Any other digest is a `warn`, and the run line carries `format_digest_unpinned=1` (D-4819) |
| `tickstore`: the `Origin` walk (`measure_tick_store`, `gdfl_import.rs:1062`–`:1140`), which now also builds the tick store's day table by exact path | O(entries under the root), capped at `ORIGIN_WALK_CAP` 1,000,000 (`:776`); 18,400 on the real store. Once per source, never per plan | as today; a `.bts` outside its `<kind>/<yyyy>/<MON_yyyy>/` folder is named per day as `day_file_misfiled{found, expected}`; a `*.bts.tmp` is never read (`FORMAT.md:22`); the writer's own `.lock`, `ingest-log.tsv`, logs and tools are counted as stray entries, never opened or locked (D-4866) |
| `dayzips`: the layout is `<ROOT>/{INDICES,STOCKS,OPTIONS}/<MON_yyyy>/<name>.zip` and nothing else (D-4834); the directory holds at least one zip of the kind; a name whose day is outside the calendar's measured span is refused **before** the day table is sized | O(entries ≤ `DAYZIPS_WALK_CAP` = 100,000), once per source (cached) | `no_day_zip_of_kind`, `dayzips_too_many_entries{cap}`, `day_zip_name_invalid{name}`; per day: `day_zip_misfiled{found, expected}`, `archive_duplicate_day` (§8 A16–A20) |
| The calendar spans the range | O(1) per end | `calendar_unmeasured{day}` (exists at day level; also checked here for the ends) |
| **The calendar's horizon** (round 3, SF-3, F55): the last day the calendar of record is sourced for (`calendar::SOURCED_LAST_DAY`, `calendar.rs:523`, 2026-10-04 at 3f65ad43; WP-62 extends it, DB-B25, DB-B41) | O(1) | Not a refusal. `plan.json` carries `calendar_horizon{last_day, days_left}` and the page shows it when `to` is within 30 days of that day or past it; a day past it is still refused `calendar_unmeasured` per day (§8 C6). Extending it is a yearly human step (§6.9) |
| **Free space against the projected growth** (round 3, SF-2, L-IA-9; round 5, F76) | O(1): no call from `pull`. WP-81's WIP measures free space **in the api** (`fix/db-WP-81:crates/api/src/disk_route.rs`, `GET /disk.json`, from `/bin/df -Pk`, at most every 30 s), which `pull` cannot name, so this line is **IA-10a's**: the `plan.json` handler reads that in-process measurement (no second `df`) and puts it beside the plan's projected growth (bars from the days to import at the measured bytes a day, verdict images L-IA-7, kept answers L-IA-6, census epochs L-IA-11, repair copies L-IA-26). The cli's plan prints `free space: see /disk.json` | Not a refusal (an extrapolation never refuses). `plan.json` warns `storage_projection_short` when the projection exceeds the free bytes; before WP-81 merges it says `free_space: unknown (WP-81)` |
| `stored=1` with options | O(1) | `stored_with_options` (moved from the cli, F24) |
| **Census room** (F20, F39): read the census header (2 slot reads), which carries `n_valid`, `n_keys` and, at v4, `capacity` (`manifest.rs:2248`–`:2260`). `free_now` = capacity − `n_valid` (v3: `MAX_ENTRIES` − `n_valid`). On v4 a full partition compacts once to one row per key, so `free_after` = `manifest::epoch_capacity(segment, n_keys)` − `n_keys`, computed by **the same function the writer uses** (`manifest.rs:577`; `census_store.rs:796`–`:830`); on v3 there is no compaction, so `free_after` = `free_now`. The first planned day's listing is read once to count its wanted files, `need_first_day` (one day-zip central directory, one `.bts` index, or one loose zip's directory) | 2 slot reads and one listing read, per kind | `census_room_short{layout, free_now, free_after, need_first_day}` when `need_first_day` > max(`free_now`, `free_after`) (each filed file appends at most one entry). When the range may need more than that, `plan.json` names `census.projected_stop`: the first planned day plus room ÷ `need_first_day` days, **an extrapolation from the first day, labelled so**; on v4 the projection includes the compaction. The run itself still stops at the census if the writer refuses (D-3317). The full decode (`census_room`, `gdfl_import.rs:2190`–`:2224`) stays on the refusal path only, as its own doc says (`:178`–`:182`) |
| **Legacy cutover** (D-4864): no `imports/gdfl.lock` or `imports/gdfl.<kind>.lock` has ever held a claim on this store (no GDFL-mode record in the runs ledger), and the journal has a key whose last line is `begin` | O(1) on the loaded journal sets | `legacy_run_may_be_live{key}` unless the request carries `legacy_ack=1` (cli `--legacy-run-not-live`; a checkbox the page shows only in this state), recorded in the claim. On the real journal the one such key is options 2020-09-16 (§2.2), so the first press there needs it |
| Stray entries beside year zips or day zips (`.DS_Store`) | none extra | Not refused. Counted in the run line `ignored_entries=<n>`, with ≤ 4 names |
| The locks are free | ≤ 5 shared probes, each released after one claim read | `run_busy{lock, run, kind, mode, pid, started, by}`, from the holder's claim (§5.3), only after the bounded retry of §5.3 |

On today's store (v4 since 10 Oct 08:54; FNO about 8.0M free entries before a
compaction, INDEX and CASH about 1.03M) no kind is refused for census room.
The `census_room_short` refusal now guards a store whose partition is truly
full even after one compaction, and every v3 test store (§2.2).

**The plan's day counts** (D-4821, D-4873), per kind, all O(days in range)
hash probes into the loaded journal sets plus one O(1) day-table probe per
day: `calendar_open`, `held_by_source`, `done`, `restated` (a `done` under a
definition that is no longer current for the kind, D-2822 with WP-11b's
per-kind currency, re-read unless `restate=0`; its cost is shown as days
times the measured cost of a day of that kind, an extrapolation), `incomplete`,
`refused`, **`scope_widened`** (round 3, D-4878, made per member in round 4,
BL-D: a day whose base key's last `done` close recorded a scope that lacks
members today's scope files, for each such member that has no key of its own
for the day yet; the count names the new members, ≤ 16, and Resume reads each
under its own key `<filter>+<member>`, §5.5), **`scope_unrecorded`** (a close
written before WP-12f recorded scopes, which the rules of §6.1 "Scope"
resolve), the holes a re-run without a repair reason cannot fill
(`holes.source_lacks`, `holes.calendar`, `holes.needs_generation`,
`holes.month_superseded`, `holes.repair_refused`; each class is the reason of a
key's last `incomplete` line, a base key's or a member key's, which is a hole
only when every open failure of that close was a store hole, §5.4, D-4851),
**the holes no repair can fill** (round 5, SF-10: `holes.bars_differ_held` and
`holes.repair_impossible`, counted `holes_unrepairable` and named in
`repair_days_refused`, read by no run), and `to_import`. **A repair-reason plan
over a month still being traded** (its last trading day by the calendar of
record not yet passed) warns `repair_in_open_month{month}` (round 5, SF-9,
D-4876; IA-16's, one calendar probe per month in range). **A predicted hole** (round 3): for
a scope-widened member, one header read per (member, month) (the month's first
held stamp; at most 8 index members × 97 months on the real store) says which
of its days lie before the month's first held bar and will need a repair;
those are counted `holes.needs_generation_predicted`, so the 556 days of §2.2
show before any run. **The plan and the run count the same days** (round 4,
BL-B): `to_import` is exactly the keys `admit` will admit (D-4885), for a
Resume and an explicit range alike, and the holes in an explicit range are
named in `holes_in_range`; a run without a repair reason reads none of them.
**With a repair reason** (D-4874) the repairable hole keys in range
(`needs_month_generation`, `month_superseded`, `repair_refused`) are counted in
`to_import` and named `repair_days`; the unrepairable ones stay in
`repair_days_refused` (round 5, SF-10). The ETA counts `to_import` plus, when
included, `restated`.

````

#### Section 5.8 New refusals and causes (the refusal table and the exhaustive-cause rule; the FailKind mapping table after it is IA-16's, merged, and is not copied)

Source: `state/design/ingest-automation.md`, lines 1814-1879

````text
### 5.8 New refusals and causes (codes are never renamed or reused)

| Variant | Code | Stage | Closes a day? | Where |
|---|---|---|---|---|
| `RunRefusal::RunBusy` | `run_busy` | — (run) | — | `gdfl_run`, after `PROBE_RETRY` (§5.3) |
| `RunRefusal::Source*` (5.2 table) | as listed | — (run) | — | `gdfl_run::preflight` |
| `RunRefusal::SourceNotConfigured{kind, variable}` | `source_not_configured` | — (run) | — | `gdfl_run::resolve_source` |
| `RunRefusal::StoredWithOptions` | `stored_with_options` | — (run) | — | `gdfl_run::plan` (moved from `gdfl_import_command.rs:268`–`:274`) |
| `RunRefusal::CensusRoomShort{layout, free_now, free_after, need_first_day}` | `census_room_short` | — (run) | — | `gdfl_run::preflight` (F20, F39, D-4850) |
| `RunRefusal::CensusLayoutChanged` | `census_layout_changed` | — (run) | — | `gdfl_run::plan` (§5.3) |
| `RunRefusal::LegacyRunMayBeLive{key}` | `legacy_run_may_be_live` | — (run) | — | `gdfl_run::preflight` (D-4864) |
| `RunRefusal::BuildSkew{expected, found}` (round 3) | `build_skew` | — (run) | — | the api child, before any lock (D-4884) |
| `RunRefusal::RepairReasonInvalid` (round 3) | `repair_reason_invalid` | — (run) | — | `gdfl_run::plan`: empty, over 200 bytes, or holding a control character, by `pull::ingest::repair_reason_ok`, the rule `POST /pull/repair` already applies (`server.rs:8892`, `MAX_REPAIR_REASON` = 200 at `:8915`), moved into `pull` so both feeds use one rule (round 6, SF-10; D-4874; round 3 said 120 bytes and a newline) |
| `RunRefusal::RunsLedgerDamaged{offset}` (round 4, SF-12) | `runs_ledger_damaged` | — (run) | — | the runs ledger allocator: two records at the tail fail their CRC (§5.3); error event and alert; no run of either feed starts |
| `RunRefusal::Unstamped` (round 5, F76; only if WP-81 merges its refusal first) | `unstamped_write` | — (run) | — | `gdfl_run::plan`, moved from WP-81's `unstamped_write_refusal` in `gdfl_import_command.rs`: a build with no commit stamp writes the store only with `BRUTEX_ALLOW_UNSTAMPED_WRITE=1`, recorded `unstamped=1` in the claim (DB-B71) |
| (not a refusal) a child released before `go` (round 4, BL-A) | exit `NOTHING`, lifecycle word `released` | — (run) | no line at all | `gdfl_run::await_go` (§4) |
| `ImportRefusal::Stopped` | `stopped` | `run` (new) | no; and no line at all when the day's `begin` was not written (D-4853) | `gdfl_import` |
| `ImportRefusal::SourceLost` | `source_lost` | `run` | no (the run ends) | `gdfl_import` (D-4852) |
| `ImportRefusal::StoreLost` | `store_lost` | `run` | nothing is written (the run ends) | `gdfl_import` (D-4852) |
| `ImportRefusal::DiskFull` | `disk_full` | `file` | no (the run ends) | `gdfl_import` (D-4851) |
| `ImportRefusal::HeldBackCap` | `held_back_cap` | `run` | no (the run ends) | `gdfl_import` (D-4851) |
| `ImportRefusal::MonthBusy{month}` | `month_busy` | `file` | no | `gdfl_import`, after `MONTH_BUSY_RETRY` (D-4851) |
| `ImportRefusal::HeldBack{after}` | `held_back` | `file` | no | `gdfl_import` (D-4851) |
| `ImportRefusal::NeedsMonthGeneration{instrument, month}` | `needs_month_generation` | `file` | **no** (round 3, SF-11: the store never closes a day, `gdfl_import.rs:625`–`:634`). The day closes `incomplete` with this reason, and the reason makes it a **hole** in the plan that Resume skips; only a repair-reason run reads it, through WP-12d (D-4851, D-4874) | `gdfl_import` |
| `ImportRefusal::MonthSuperseded{instrument, month}` (round 3) | `month_superseded` | `file` | no; a hole, as above: the month is served from a revision and the batch holds a new bar (`StoreError::Superseded`, D-4876) | `gdfl_import` |
| `ImportRefusal::RepairRefused{instrument, month, why}` (round 3; narrowed in round 5, SF-10) | `repair_refused` | `file` | no; a hole a later build or repair can lift (`RowLimit` until D-4875 (1), `NoBasisWitness`, `UnsealedSource`, `Conflict`, `Incomplete`, `InvalidReceipt`), named with the repair's own refusal; `admit` row 4a gives it to the next repair-reason run | `gdfl_import` (WP-12d's filing path returns it) |
| `ImportRefusal::RepairImpossible{instrument, month, why}` (round 5, SF-10) | `repair_impossible` | `file` | no; a hole **no repair can fill** (`HeldBarDiffers`, `IndexedSibling`, `RevisionLimit`), named with the repair's own refusal; `admit` row 4b never reads it again, with or without a repair reason | `gdfl_import` (WP-12d's filing path returns it) |
| `ImportRefusal::BarsDifferHeld{definition}` | `bars_differ_held` | `file` | no; a hole, as above: a restated file whose new bars the store refused; a held bar is never changed (D-4873) | `gdfl_import` |
| `ImportRefusal::StoreDamaged{why}` (round 4, SF-1) | `store_damaged` | `file` | no; **never a hole**: the month refused to open or append for a fault of its own (CRC, format, torn tail, a generation pointer that does not verify); held back; an error event and an alert | `gdfl_import` |
| `ImportRefusal::BatchRefused{why}` (round 4, SF-1) | `batch_refused` | `file` | no; never a hole: the store refused a batch the importer built wrongly (empty, unordered, off grid, outside the month, an impossible bar), or a repair met a merge without a held stamp; an importer defect, error event | `gdfl_import` |
| `ImportRefusal::StoreIo{kind}` (round 4, SF-1) | `store_io` | `file` | no; never a hole; reached only after the loss probes found both roots present; held back | `gdfl_import` |
| `ImportRefusal::NoWantedFile` | `no_wanted_file` | `vendor` | no | `gdfl_import` (F10). Raised only for an **unfiltered** run on a calendar-open day the source holds, when the listing yields zero wanted files. A filtered run asking for a symbol that is not listed that day still closes `done files=0`, because a share before its listing or after its delisting is a real absence (D-4872) |
| `ImportRefusal::AbsentFromListing{instrument, unknown: ≤4}` | `absent_from_listing` | `vendor` | yes (vendor bytes) | `gdfl_import` (F10). Only for the swept indices (NIFTY, BANKNIFTY), present on every one of the 1,997 measured days. An absent swept **share** is counted (`shares_absent`) and named in the day image's detail section (≤ 16), never a refusal, because no charter source lists each share's trading days |
| `ImportRefusal::StoredIndexAbsent{stem, unknown: ≤4}` | `stored_index_absent` | `vendor` | yes (vendor bytes) | `gdfl_import` (D-4833): a stored alias index (`core::alias` `INDEX_ROWS`, `alias.rs:399`) absent from a held day inside its listing span **as recorded in the charter**. Until IA-0b records the spans, the check is inactive and `plan.json` says so by name |
| `CmRefusal::DayZipMisfiled{found, expected}` | `day_zip_misfiled` | `vendor` | no (a copy fault) | `gdfl_archive`, `gdfl_dayzips` (F9) |
| `CmRefusal::DayFileMisfiled{found, expected}` | `day_file_misfiled` | `vendor` | no (a copy fault) | `gdfl_tickstore` (F46, D-4866) |
| `CmRefusal::ArchiveMemberEncrypted{member}` | `archive_member_encrypted` | `vendor` | no (a copy fault) | `gdfl_archive` (F12) |
| `CmRefusal::ArchivePrefixed{bytes}` | `archive_prefixed` | `vendor` | no (a copy fault) | `gdfl_archive` (§8 B7) |
| `CmRefusal::ArchiveMemberCompressed` (**existing**; now per day for CM too, moved from `Archive::from_source` into `day_zip_span`) | `archive_member_compressed` (existing) | `vendor` | **no, for both trees** (`cm_cause`'s arm, `gdfl_import.rs:677`–`:679` at ab2810b8, changes from `Cause::vendor` to `Cause::open`: a container fault never closes, `:625`–`:634`; D-4818). **IA-3 changes the arm in the same change that makes the refusal per day** (round 4, SF-10), so no build closes a day for it | `gdfl_archive` (F13). No second code is minted |
| `CmRefusal::ArchiveDuplicateDay` (**existing**) | `archive_duplicate_day` (existing) | `vendor` | **no** (its arm changes the same way, in IA-3, F38): two copies of one day are the source's layout, and removing one lets a re-run fill the day (§8 A17, B19; D-4818) | `gdfl_archive`, `gdfl_dayzips` |
| `CmRefusal::ArchiveMethodUnknown` (**existing**) | `archive_method_unknown` (existing) | `vendor` | **no** (its arm changes the same way, in IA-3, F38): a member this reader cannot inflate is a fault of the container as read, and a build that reads the method can fill the day (D-4818) | `gdfl_archive` |
| `CmRefusal::ExtensionMissing{stem}` | `extension_missing` | `vendor` | yes | `gdfl_cm` (F29, D-4835): the whole entry name equals a wanted stem |
| `NfoRefusal::ExtensionNearMiss{name}` | `extension_near_miss` | `vendor` | yes: the vendor's own bytes name the file so, and a re-read reads the same name (D-4872) | `gdfl_nfo` (F17) |
| `NfoRefusal::TooManyBadRows` (parity) | `too_many_bad_rows` | `vendor` | yes | `gdfl_nfo` (F8) |

**Every cause is decided in one exhaustive place.** `cm_cause` and `nfo_cause`
(`gdfl_import.rs:664`, `:717` at ab2810b8) are exhaustive over `CmRefusal` and
`NfoRefusal` only, with no `_` arm, so **every package that adds a variant adds
its arm in the same change** (round 4, SF-10): IA-3 the arms of
`DayZipMisfiled`, `DayFileMisfiled`, `ArchiveMemberEncrypted`,
`ArchivePrefixed`, `ExtensionMissing` and `ExtensionNearMiss`, and D-4818's
three archive arms; IA-4 the arms of its parity variants (`TooManyBadRows`,
and any per-row fault it adds to `NfoRefusal`). Those are `gdfl_import.rs`
edits by lane-B packages, ordered after IA-2 and before IA-16 (IA-3) and after
IA-16 (IA-4) (§16.2). An `ImportRefusal` reaches a `Cause` through `staged()`
(`:2198`), which calls `Cause::open` for every variant, so as written round 1's
`AbsentFromListing` would silently have closed nothing. IA-2 adds `const fn
import_cause(&ImportRefusal) -> Cause`, exhaustive, and `staged()` takes the
cause from it. Every call site
stages a variant one way today (`:2133` and `:2322` stage journal refusals at
`Stage::Journal`; `:2305` and `:2343` stage `convert` and filing refusals at
`Stage::File`), so the call sites' stage argument is dropped and a variant
cannot be staged two ways. Each new variant's (stage, code, closes) is pinned
in a test next to the `Stage::ALL` pin (`gdfl_import_tests.rs:1093`–`:1096`),
test IA-GR-16; IA-GR-27 pins the three archive arms of F38. Every new refusal
emits its event at the refusal site, as gate 19 requires (`ci.yml:2537`).
````

#### Section 8 Input case matrix: legend and table A (containers and sources)

Source: `state/design/ingest-automation.md`, lines 3717-3767

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

### A. Containers and sources

| # | Case | On disk (measured) | Recognised by | Today | After | Test | Cost |
|---|---|---|---|---|---|---|---|
| A1 | CM outer zip, ZIP64, stored day zips | 128.9 GB, 4,209 entries; every day zip with a data descriptor (B13) | `gdfl_archive.rs:198` (`central_directory`); `Archive::from_source` | read in place; the day table is built once per run | unchanged. Preflight records the day span, the plan shows it, and the api caches the day table per source | ✓ `an_archive_on_disk_is_read_through_the_file` (**synthetic**: its builder writes flags 0 with real CRC and sizes, `gdfl_archive.rs:789`–`:795`, not the real shape), ✓ `a_stored_member_is_read_as_it_is`; + B13's test | O(entries) once per source; **UNVERIFIED as a measured time** (`docs/06-limits.md:11614`–`:11619`), bench C-GI-05; O(1) per day |
| A2 | Nested day zip (a stored member holding its own central directory) | 3,994 | `day_zip_span` `gdfl_archive.rs:458` | listed per day | unchanged | ✓ `a_deflated_member_is_inflated_and_read_end_to_end` | O(entries of the day) once per day |
| A3 | Options yearly zip `<yyyy>.zip` | 9 zips, 84–265 entries, ZIP64 | `NfoZips::day` `gdfl_nfo.rs:1014`; `listing_in` `:1042` | **reopens the year zip and walks its directory every day** (F6) | **WP-16's** `NfoYears` indexes each year zip **once per run** into a day table (as `Archive`); each run is its own process, so no index outlives its run; the api's source cache holds only what `plan.json` and the api's preflight read (round 5, SF-17: round 4 said "once per source" and gave the test to IA-3) | ✓ `every_fault_of_the_yearly_zip_is_refused_by_name`; + `a_year_zip_is_indexed_once_and_each_day_is_one_lookup` (**WP-16's**, with its row `IA-MX-A3` only if WP-16 left the test to this design, then IA-3's), bench C-GI-04 (WP-16's) | O(entries of the year) once per run; O(1) per day |
| A4 | Loose day zips directory | 12 zips, 2026-09-25..30, laid out `{INDICES,STOCKS,OPTIONS}/SEP_2026/` | none | **unreadable** (sources are `tickstore` and `zip`, `gdfl_import_command.rs:255`) | `SourceKind::DayZips`: the layout `<ROOT>/<KIND>/<MON_yyyy>/<name>.zip` only (D-4834); the directory is indexed once by exact name, capped at `DAYZIPS_WALK_CAP` = 100,000 entries; each zip read with `zip_entries` and `member_bytes` | + `loose_day_zips_import_like_the_archive_day_they_copy` (IA-3) | O(entries in dir ≤ 100,000) once per source; O(1) per day |
| A5 | Tick store BRTXTS01 | cm 4,002, options 2,001 `.bts`; 18,400 entries under the root | `TickStore` `gdfl_tickstore.rs:539`; `NfoTickStore` `gdfl_nfo.rs:934` | read | preflight checks FORMAT.md and the magic, and compares the digest with the charter's two (`docs/00-charter.md:1065`, `:1108`–`:1109`); today's `d21139…` is pinned (F14, D-4819) | ✓ `the_tick_store_and_the_zips_give_the_same_file`; + `a_format_digest_outside_the_charters_two_is_recorded_and_warned` | 2 reads and 1 hash, once per source |
| A6 | Tick store root pointed at the wrong folder | `NSE_Options_Tick` holds only `_v3` | NotFound to `Ok(None)` `gdfl_nfo.rs:959` | every trading day is journalled `refused reason=day_not_held` (F1) | preflight `source_root_unknown{found:"_v3 extraction ledger"}`, before any journal line | + `a_wrong_source_root_is_refused_before_any_day_is_journalled` (IA-GR-07) | O(1) |
| A7 | `_v3` extraction ledger | 1,997 manifests and ok markers, owner, runs, locks; 6-field lines (§2.1) | none | unused | read-only completeness evidence (`pull::gdfl_v3`). The manifest is trusted only when the `ok` sha256 matches. A listing entry maps to `<MON_yyyy>/` + the entry with `\` turned to `/`; directory lines are kept. Never written, never `flock`ed (D-4814) | + `the_v3_manifest_agrees_or_names_the_first_difference` (on a backslash day and on 2026-09-24 with its 2 directory lines), + `an_unverified_v3_manifest_is_not_trusted` | O(entries of the day) per day |
| A8 | Extracted folder given as a source | retired (D-2800) | `Archive::open` on a directory | the read fails, so `archive unavailable`; the cli prints `GDFL IMPORT REFUSED` | preflight `source_is_a_directory` with the hint | + `a_folder_is_refused_as_a_source_by_name` | O(1) |
| A9 | Options dir without `<yyyy>.zip`, or with a near-miss name | none today | `measure_zips` only warns (`gdfl_import.rs:984`) | each day of that year: `day_not_held` | preflight `year_zip_missing{year, near_miss}` | + `a_missing_or_misspelt_year_zip_is_refused_naming_the_near_miss` | O(years) |
| A10 | Stray file beside the year zips | `.DS_Store` 6,148 B | filtered by name in `measure_zips` | ignored silently | counted in the run line `ignored_entries=1`, with ≤ 4 names | + in the test above | O(entries) once |
| A11 | Hard-linked or 0600 year zips | 2020–26 nlink 2; 2021–26 mode 0600 | none | a read by another user fails per day | preflight reads 1 byte of each EOCD and names `source_unreadable{path, mode}` | + `an_unreadable_year_zip_is_refused_before_the_run` | O(years) |
| A12 | Source lost mid-run (drive unmounted) | memory note: WD_BLACK access lost 29 Sep | the CM archive: a listing error (`gdfl_import.rs:2884`, `:3033`); the tick store, the year zips and the day zips: NotFound becomes `Ok(None)` (`gdfl_nfo.rs:959`, `:1022`; `gdfl_tickstore.rs:562`) and the day closes `day_not_held` (`gdfl_import.rs:2770`–`:2771`) | every remaining day is refused; a drive pulled under an open file gives EIO, which std names `Uncategorized` (F37) | at the first failing **file**, whatever its `ErrorKind`, both roots are probed (D-4852): the store gone is `store_lost` (nothing more written; WD_BLACK holds both, so this is the usual case); the source gone is `source_lost` (the day closes `incomplete stage=run reason=source_lost`, the run releases and ends) (F16) | + `a_lost_source_or_store_stops_the_run_at_the_failing_file` (IA-GR-09, IA-16: an EIO `ReadAt`, and each root removed mid-run, for the CM archive, the year zips, the tick store and the day zips) | 2 `metadata` per failing file |
| A13 | Placeholder zips in the project root | `GFDLNFO_TICK_06012019.zip` (Sunday) and `..._07012019.zip` (Monday 2019-01-07, a trading day the archives hold), 450 B each; `outer.zip`, 1,042 B, a streamed outer zip cut off before its own central directory (§2.1); all untracked | not under any configured root | never read | if placed under `dayzips` (in the right `<KIND>/<MON_yyyy>/` folder): 06012019 is calendar-closed, so `archive_day_on_closed_day`. 07012019 lists `Options/S.NFO.csv`, so `entry_ticker` yields `S` and `decode_ticker` refuses it `ticker_unparsed` (the ticker is refused before any byte is read, so no header refusal follows); an unfiltered run then refuses the day `no_wanted_file`, so it is never closed `done` from that zip. `outer.zip` matches no day-zip name, so `dayzips` counts it as a stray entry; read as a zip it is refused `archive_malformed` `NO_END_RECORD` (`gdfl_archive.rs:53`, raised `:210`), B18. Fixtures are built in code and never read from these files | + `the_placeholder_shapes_are_refused_by_name` (built in memory, including the streamed shape of `outer.zip`) | O(1) |
| A14 | Two sources in one run | — | the cli takes one (module doc, `gdfl_import_command.rs:17`–`:18`) | one source per run | one kind-agnostic source word per request, resolved per kind (`zip` → `cm_zip` or `nfo_zips`); no fallback between sources | ✓ `a_source_of_the_wrong_kind_is_refused_by_name` | O(1) |
| A15 | Sources disagree on their day span | zips ≤ 2026-09-24; tick store and loose ≤ 2026-09-30 | none | a zip run journals 25–30 Sep as `day_not_held` | `plan.json` shows each source's span. Per-source completeness: `source_lacks_day{source}` (code 12) | + `each_source_names_its_own_span_in_the_plan` | O(1) per day |
| A16 | `dayzips`: a zip in the wrong kind or month folder | 0 measured | the layout rule (D-4834) | — (no `dayzips` source) | `day_zip_misfiled{found, expected}`; that day refused at the vendor stage | + `a_loose_zip_in_the_wrong_kind_or_month_folder_is_named` | O(1) per name |
| A17 | `dayzips`: the same day twice (two month folders, or a copy such as `... (1).zip`) | 0 measured | the day table build | — | `archive_duplicate_day` for that day only (as B19); the day stays `incomplete`, never closed (D-4818, F38), so removing the copy lets a re-run fill it | + `a_loose_day_given_twice_is_refused_for_that_day` | O(1) per name |
| A18 | `dayzips`: the zip's inner folder names another day than its file name | 0 measured | `DayListing::push` / `entry_ticker` (folder filter) | — | as C11: members outside the day's folder are skipped and counted `outside_folder`; an unfiltered run refuses the day `no_wanted_file` | + `a_loose_zip_whose_folder_names_another_day_is_refused_not_done` | O(1) |
| A19 | `dayzips`: a stray non-zip file or an unrecognised name | the layout on disk has none | the layout rule | — | counted, ≤ 4 named in the run line (`ignored_entries`) | + in A16's test | O(1) per name |
| A20 | `dayzips`: a name whose day is impossible or outside the calendar's span | 0 measured | refused before the day table is sized (`gdfl_archive.rs:1201`–`:1211` pins the dense table's worst case at 2,932,897 cells × 24 B, about 70 MB, for one far-dated name) | — | `day_zip_name_invalid{name}`, ≤ 16 named | + `a_far_dated_loose_zip_name_never_sizes_the_day_table` | O(1) per name |
| A21 | Census without room for the run | v3, 297 free of 2,097,152 until 10 Oct 08:54 (§2.1); every v3 test store | `census_room` (refusal path only, `gdfl_import.rs:2190`) | the run starts and stops at the census on its first options or stocks day | preflight `census_room_short{layout, free_now, free_after, need_first_day}` from the header (2 slot reads) and the first day's listing; `plan.json` names the projected stop day, an extrapolation from the first day (F20, D-4802) | + `a_census_without_room_for_the_first_day_is_refused_before_any_line` (IA-GR-15) | 2 slot reads + one listing read |
| A22 | A v4 partition whose free room is short of the first day, while one compaction would make room | the real FNO partition: 367,189 entries of 8,388,608 (§2.2); a test store built near capacity | `census_store`'s write path compacts once and refuses only when one row per key leaves no room (`census_store.rs:796`–`:830`); the header carries `n_keys` (`manifest.rs:2250`) | — (round 1's preflight would have refused it, F39) | not refused: room = max(`free_now`, `epoch_capacity(segment, n_keys)` − `n_keys`), by the writer's own function (D-4850); refused only when both are short | + `the_v4_preflight_models_the_compaction_the_writer_makes` (IA-GR-26) | 2 slot reads + one listing read |

````

#### Section 8 table B (zip structure): header and the size and empty-archive rows (B28 and B28a are new, added by this part)

Source: `state/design/ingest-automation.md`, lines 3768-3771, 3775, 3779-3781, 3789, 3797-3798

````text
### B. Zip structure

| # | Case | On disk | Recognised by | Today | After | Test | Cost |
|---|---|---|---|---|---|---|---|
| B4 | Central directory over 64 MiB, or past the end | outer CD 493,028 B | `CENTRAL_CAP` `:235` | refused | unchanged | ✓ `a_central_directory_past_the_end_or_over_the_cap_refuses` | O(1) |
| B8 | Outer member compressed (a day zip deflated) | 0 measured | CM `:401`–`:405`; NFO `stored_span` `:686`–`:692` | **CM: the whole archive is refused**; NFO: that day only, and the day closes (F13) | per day for both, under the **existing** code `archive_member_compressed`, which no longer closes a day in either tree (D-4818); the CM check moves from `Archive::from_source` into `day_zip_span` | ✓ `outer_member_not_stored_refuses` (changed to day-level) | O(1) |
| B9 | Inner member stored (method 0) | placeholder, directory entries | `member_bytes` `:611` | read | unchanged | ✓ `a_stored_member_is_read_as_it_is` | O(bytes) |
| B10 | Inner member deflate (8) | every data file | `:612` | inflated to at most len+1 | unchanged | ✓ `a_deflate_bomb_is_inflated_one_byte_past_its_stated_length_and_no_further` | O(bytes) |
| B18 | Truncated archive (no end record), including a streamed outer zip cut off before its central directory (`outer.zip`: its two inner end records end at 505 and 1,026, short of 1,042) | `outer.zip` | `NO_END_RECORD` (`gdfl_archive.rs:53`; `:202`–`:211` accepts only an end record whose comment ends at the last byte) | refused at open | at preflight, before the lock | ✓ `no_end_record_refuses`; + the streamed shape in `the_placeholder_shapes_are_refused_by_name` | O(1) |
| B26 | Deflate bomb | — | `limit = len + 1` | stopped one byte past | unchanged | ✓ (B10) | O(len) |
| B27 | Not a zip at all (an HTML error page saved as `.zip`) | refused by the update script | `NO_END_RECORD` | refused | `dayzips`: that day's listing refused by name | ✓ `no_end_record_refuses`; + loose-zip case | O(1) |
````

#### Section 8 table T (tick store): header and rows T1, T6, T9, T12

Source: `state/design/ingest-automation.md`, lines 3947-3953, 3958, 3961, 3964

````text
### T. The tick store (BRTXTS01, `gdfl_tickstore.rs`; critic S9)

Every case below is a `CmRefusal::TickStoreMalformed{what}` or `TickStoreUnavailable{kind}`, both open causes (`gdfl_import.rs:622`–`:625`): the day is incomplete, never closed, and the file's (instrument, month) is held back (D-4851). Counts are of the 4,002 CM and 2,001 options `.bts` files under `tickstore-data` (§2.1).

| # | Case | On disk | Recognised by | Today | After | Test | Cost |
|---|---|---|---|---|---|---|---|
| T1 | Not a day file: too short, or no `BRTXTS01` magic | 0 measured | `NO_MAGIC` (`gdfl_tickstore.rs:40`) | `tick_store_malformed`, day incomplete | unchanged; preflight also reads the range's first `.bts` (§5.2) | ✓ `a_file_without_its_magic_or_footer_refuses` | O(1) |
| T6 | A malformed columnar block: its column count, row count, column text, a numeric shape, a time out of range, or a rebuild past the stated size | 0 | `COLUMNAR` (`:60`) to `PAST_SIZE` (`:76`) | that file refused | unchanged | ✓ `a_malformed_columnar_block_refuses`, ✓ `a_column_that_is_not_its_rows_refuses`, ✓ `every_tick_store_refusal_says_what_it_refused` | O(bytes) |
| T9 | A `*.bts.tmp` beside a day (a write interrupted, or in progress), or a day published while a run is going | none seen on 10 Oct | the day lookup by exact name (`gdfl_tickstore.rs:557`) | the day reads absent | never read (`FORMAT.md:22`); a day that appears by rename during a run is read whole when the run reaches it (`FORMAT.md:112`, the spec's statement, UNVERIFIED for the writer's code; D-4866) | ✓ `a_temporary_or_absent_day_is_none_and_an_unreadable_one_refuses`; + `a_tick_store_day_published_during_a_run_is_read_whole` | O(1) |
| T12 | Days past the zips' end (2026-09-25..30) | §2.1 | the per-source span | a zip run journals them `day_not_held` | the per-source span (A15); V-2 imports them from the tick store | + (A15) | O(1) |
````

#### Section 9 Runtime edge cases: header and the two preflight rows

Source: `state/design/ingest-automation.md`, lines 4010-4013, 4062, 4066

````text
## 9. Runtime edge cases

| Case | What happens | Named as | Test |
|---|---|---|---|
| Census without room (v3, or a v4 partition full even after one compaction) | preflight `census_room_short` before any line (A21); a run that runs out mid-range still stops at the census (D-3317) | §5.2 | ✓/+ IA-GR-15, IA-GR-26 |
| First press after the cutover while an old-build import may still run | `legacy_run_may_be_live{key}` unless acknowledged | D-4864 | + IA-GR-20 |
````

#### Section 10 O(1), p50/p99 and the honest limits: who measures, 10.1 header and the preflight rows (C-GR-05)

Source: `state/design/ingest-automation.md`, lines 4105-4121, 4124, 4135, 4164

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
| Preflight on `POST` | O(1) checks, plus one listing read of the first planned day (O(entries of that day)) | §5.2; the source cache; the full census decode only on the refusal path | C-GR-05 (against a v4 FNO epoch of the real size, 8,388,608 entries, and a v3 copy of 268 MB) | IA-3 |
| Census room | O(1) | 2 slot reads of the census header (`n_valid`, `n_keys`, capacity) and one `epoch_capacity` call | C-GR-05 | IA-3 |
| (round 3) Hole-reason read per journal line | O(1) | one `reason=` field kept per key in a map | C-GR-05 | IA-2 |
````

#### Section 10.2 Limits: header, L-IA-11 (census publish, cited by item 5 for the full decode) and L-IA-35 (open files, item 7)

Source: `state/design/ingest-automation.md`, lines 4181-4184, 4195, 4222

````text
### 10.2 Limits for `docs/06-limits.md` (new section "Ingest automation, D-4800 to D-4888"; each row lands with its package, **in the same change as its code**, D-4861)

| ID | Limit | Why it is not O(1) | Bound | Package |
|---|---|---|---|---|
| L-IA-11 | The census publish per day, and the epochs kept | reads and decodes the whole partition (v4) or file (v3), then 2 `F_FULLFSYNC` per appended entry (until WP-16 batches them), under the census lock. On the real store (v4) the FNO epoch grows by about 10,428 entries per options day: from 367,189 entries (§2.2) it reaches its 8,388,608 capacity after about 769 options days, then compacts to one row per key into a new epoch; the 1,495 remaining options days give about 2 compactions. Each epoch is kept forever (D-3316 item 2), up to 8,388,608 × 128 B = 1 GiB each, so about 2–3 GB of FNO epochs after the backfill, and each day's publish reads up to 1 GiB (about 0.5 GiB on average, about 0.8 TB over the backfill). All **extrapolations** from §2.2's counts | Θ(entries) per day; measured by WP-16 on 2026-08-03 and by C-GR-05 | WP-16 (publish), IA-3 (projection in `plan.json`) |
| L-IA-35 (round 5, SF-16) | Open files during phase 2 | a day's read-back touches every instrument-month it filed (up to 26,820 contract-months on an options day, each a `.bin`, `.crc`, `.lock` and `.tix`) | at most W month handles at once (≤ 5 descriptors each) when phase 2 opens through `open_existing` after WP-16's flush, the default; when WP-16 keeps its writer handles past the flush, WP-16's own stated bound, set under the process's `RLIMIT_NOFILE` read at start; IA-GR-08 counts descriptors at W = 1 and 12 | IA-7; WP-16 (its bound, if it keeps handles) |
````

#### The open-file paragraph L-IA-35 and IA-GR-08 come from (section 6.1)

Source: `state/design/ingest-automation.md`, lines 2160-2174

````text
   takes the next, so phase 2 holds at most W month handles at once (at most
   5 descriptors each: pointer, `.bin`, `.lock`, `.crc`, `.tix`), whatever the
   day's file count; the run's kind lock makes the shared flock safe from every
   other writer. Reading **through the writer's own `BarFile` handle** (no
   second open, and no shared flock, which would fail with `StoreError::Locked`
   while that handle holds the month, `store/src/file.rs:1709`) is used only
   when WP-16 keeps its handles past the flush, and then **WP-16 names its
   open-file bound**: an options day writes up to 26,820 contract-months, each a
   `.bin`, `.crc`, `.lock` and `.tix`, far past macOS's per-process default, so
   WP-16 either closes each month after its write (and the default above
   applies) or keeps at most a stated number of handles, set under the
   process's `RLIMIT_NOFILE` read at start, and phase 2 then reads through
   those it kept and opens the rest one at a time. L-IA-35 names the bound;
   IA-GR-08 pins both paths at W = 1 and W = 12 with a descriptor count. A day's
   instruments in a month repair not yet published are read back only after it
````

#### Section 11 Files: header and the preflight file entry (and the bench file entry the spec replaces)

Source: `state/design/ingest-automation.md`, lines 4226-4235, 4236, 4273

````text
## 11. Files and types to add or change

"After" names the fix-round packages that own the file first (`fix-plan.json`
`files_owned`, or a branch's diff where the plan is silent); the IA package
edits it only after they merge (§16). A file with two IA packages lists them
in order; they are in one lane.

| Crate / path | Add or change | What | Package | After |
|---|---|---|---|---|
| `crates/pull/src/gdfl_run.rs` | **add** | IA-1: `Request`, `Mode`, `By`, `SourceWord`, `SourceId`, `Range`, `VerifyMode`, `SourceCache`, `Plan`, `PlanSummary` (with the round-3 classes: `scope_widened`, `scope_unrecorded`, the five hole reasons (round 5), `holes_unrepairable`, predicted holes, `holes_in_range`, `calendar_horizon`, `free_space`), `Observer`, `RunRefusal`, `plan`, `dry_plan`, `execute` (this process's kinds in order, no thread per kind, round 4), `resolve_source`, `open_source`; the progress-file observer. IA-2: the locks, probes, the store-wide runs ledger with its record CRC (`audit/runs.ledger`, D-4880, round 4 SF-12), claims and request sidecars, release, `lock_state`, `RunControl` and the stop file, `StartGate` and `await_go` (the two-phase start, round 4, BL-A). IA-1 also: `RunRefusal::Unstamped`, moved from WP-81's cli check (round 5, F76). IA-16: the `restate` field, `repair_reason` if WP-12d defined no trigger (round 4, SF-18), and the plan's `repair_in_open_month` warning and `holes_unrepairable` (round 5, SF-9, SF-10). **IA-7: the end-of-run deferred step, rules 1 and 2 of §6.1, and the `Spawn` hook in `RunControl`** (round 5, SF-6, SF-12: round 4 gave "the deferred pass's driver" to IA-2 here and to IA-7 in §16.2; IA-7 is its one owner, since IA-2 lands before IA-6 builds the pass) | IA-1, IA-2, IA-16, IA-7 | WP-81 (IA-1's unstamped move) |
| `crates/pull/src/gdfl_run_preflight.rs` | **add** | `preflight` and every check of §5.2, including the v4 census model and the legacy cutover | IA-3 | — |
| `crates/api/benches/ratio/ingest_automation.rs` (+ one line in `crates/api/benches/ratio.rs`) | **add** | C-GR-03, C-GR-05, C-VD-03; round 4 (SF-19): C-GR-10 (build skew), C-GR-12 (`/gdfl/sources.json`), C-GR-13 (stop), C-GR-14 (dismiss), C-AL-02 (`/alerts.json`), IA-10a's; C-VD-06 (`/verify/day.json?code=`), IA-10b's | IA-10a, IA-10b | — |
````

#### Section 14 Tests: header and the rows the spec cites

Source: `state/design/ingest-automation.md`, lines 4381-4406, 4413-4414, 4418, 4424-4425

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
| IA-GR-03 | The cli and the api exclude each other: on a v3 store across every kind, on a v4 store per kind only | `the_cli_and_the_api_exclude_each_other_by_the_census_layout` | IA-2 |
| IA-GR-04 | v3: a press's kinds run in sequence and a second press is refused; v4: kinds run in parallel, never interleave or tear a journal line, and neither stops at `census_refused` | `two_kinds_follow_the_census_layout_and_never_tear_a_line` | IA-2 |
| IA-GR-05 | Stop closes the day `incomplete stage=run reason=stopped` when its `begin` was written, writes no line when not, closes `done` when the stop came after the day's last file, writes no further `.auc`, works on a run held by another process, and a resume converges | `a_stop_closes_the_day_by_what_was_written_and_a_resume_converges` | IA-2 (round 4, SF-11) |
| IA-GR-06 | A claim in a free lock file is interrupted; at most one per lock file; a new claim supersedes it; auto-resume only for an api import that asked, not stopped, and at most once per stuck day | `a_dead_runs_claim_is_interrupted_and_resumes_at_most_once_per_day` | IA-10a |
| IA-GR-06b | A shutdown during an import leaves `outcome=shutdown`, no claim, and no auto-resume | `a_shutdown_releases_the_run_and_never_auto_resumes` | IA-10a |
| IA-GR-07 | A wrong root is refused before any journal line | `a_wrong_source_root_is_refused_before_any_day_is_journalled` | IA-3 |
| IA-GR-08 | W workers produce the bytes W = 1 produces (store, journal, census, `.auc`, `.prv` but `landed_at`; and, with IA-7, verdict images but "written at"); **phase 2 never holds more than W month handles** (or WP-16's stated bound when it keeps its handles), counted by the process's open descriptors at W = 1 and 12 on a synthetic day of 26,820 files (round 5, SF-16, L-IA-35) | WP-16's test, extended by IA-7: `w_workers_build_the_same_store_journal_and_verdicts_as_one`; `phase_two_holds_at_most_w_month_handles` | WP-16, IA-7 |
| IA-GR-15 | A v3 census without room for the first day's wanted files is refused before any journal line | `a_census_without_room_for_the_first_day_is_refused_before_any_line` | IA-3 |
| IA-GR-16 | Every new `ImportRefusal`'s (stage, code, closes) is what `import_cause` says, and `staged()` uses it; every new `CmRefusal` and `NfoRefusal` variant has its arm in the change that adds it; no store refusal closes a day; `decisive()` takes census, then run, then the first non-hole open cause, then the first hole (round 4, BL-C) | `every_import_refusal_has_one_pinned_cause` | IA-2, IA-3, IA-16, IA-4 (each its own variants) |
| IA-GR-20 | The first plan on a never-claimed store with an open `begin` refuses `legacy_run_may_be_live` unless acknowledged, and records the acknowledgement | `the_first_claim_asks_about_a_legacy_run` | IA-3 |
| IA-GR-26 | A v4 partition whose free room is short of the first day but whose compaction leaves room is not refused; one with no room after compaction is | `the_v4_preflight_models_the_compaction_the_writer_makes` | IA-3 |
| IA-GR-27 | `archive_duplicate_day`, `archive_method_unknown` and `archive_member_compressed` never close a day, from the change that makes them per day (round 4, SF-10) | `no_container_fault_closes_a_day` | IA-3 |
````

#### Section 15 Decisions: the numbering check, and the entries the spec lands or amends

Source: `state/design/ingest-automation.md`, lines 4539-4572, 4585, 4589, 4604, 4620, 4634, 4636

````text
## 15. Decisions (proposed numbers)

**The numbering check.** On 9 Oct,
`git grep -h -o -E 'D-[0-9]{4}' $(git for-each-ref --format='%(refname)')`
over 358 refs found the highest D-number anywhere to be **D-4799**, the end of
a lane reservation (D-4700..D-4799,
`origin/fix-queue:resume/lane1b-verify-20261008/RESUME.md:14`); the highest in
any ref's `docs/05-decisions.md` D-3408; on `feat/db-fix` D-3348. On 10 Oct,
round 2 ran `git grep -h -o -E 'D-4[89][0-9]{2}'` over **368 refs** and found
only D-4800 and D-4825, both in `origin/fix-queue:resume/mac-gdfl-20261009.md:76`,
which names this design's own block. So **D-4800 to D-4873** are free. **Round 3** re-ran it over all **375 refs** (10 Oct afternoon) and found D-4800, D-4825 and D-4873 only, all in `origin/fix-queue:resume/mac-gdfl-20261009.md` and `…-20261010.md`, which name this block; so **D-4874 to D-4884**, the entries round 3 adds, are free too. WP-13c takes D-3349..D-3351 on its branch. **Round 4** re-ran it per ref over all **375 refs** (10 Oct evening) and found D-4800, D-4825 and D-4873 (this block's names, in `origin/fix-queue`) and, **new since round 3, the IA-5 WIP branch `fix/db-IA-5` 4900b2ca**, whose `docs/05-decisions.md` heads D-4808, D-4809, D-4844, D-4845 and D-4865 (and cites D-4807, D-4841, D-4854, D-4859) from this block, as IA-5 should, except D-4845 (§16.2, F70); so **D-4885 and D-4886**, the two entries round 4 adds, are free; the highest entry heading on `feat/db-fix` ab2810b8 is D-3348, on `fix/db-WP-13c` D-3351, and WP-39's WIP cites D-3349 in `docs/02` only (its `docs/05` still ends at D-3348). **Round 5** re-ran it per ref over all
**377 refs** (10 Oct, after 16:21): it finds D-4800, D-4807, D-4808, D-4809,
D-4825, D-4841, D-4844, D-4845, D-4854, D-4859, D-4865 and D-4873, every one
this block's own name (on `fix/db-IA-5` and in `fix-queue`'s resume notes),
and nothing from D-4874 up; so D-4800 to D-4886 remain this block's, and
**D-4887**, the one entry round 5 adds, is free. On D-33xx: the highest
entry heading on `feat/db-fix` 9cfa4577 is still D-3348; WP-13c's second pass
cites D-3350, D-3351 and D-3352 in code and has removed its `docs/05` entries;
WP-39's WIP cites D-3349 (header, "Cites, round 5"). **Round 6** grepped all
**380 refs** (10 Oct, after 19:25) for D-4888 and above and found none, so
**D-4888**, the one entry round 6 adds, is free; the round-6 critic's grep of
D-48xx and D-49xx over the same refs found this block's names only (on
`fix/db-IA-5` and `origin/fix-queue`); and `feat/db-fix` d73e1112 still heads
no entry above D-3348, WP-13c having merged with its D-3350..D-3352 handed to
WP-99. Re-run
the grep at landing, because another lane may claim the block first; gate 27b
(`ci.yml:2489`) refuses a number that heads two entries. None of these is
minted yet, so round 2 amends round 1's proposals in place (marked "round 2").

| D | Decision |
|---|---|
| D-4800 | One GDFL run authority, `pull::gdfl_run`, shared by the cli and the api. The cli keeps only argument and env parsing. The `--stored`-with-options refusal moves into `gdfl_run::plan` (`stored_with_options`), so both edges refuse it with one code. |
| D-4801 | Sources are named by configuration (the same `BRUTEX_GDFL_*` variables, plus `BRUTEX_GDFL_DAYZIPS`, and, round 2, `BRUTEX_GDFL_V3_LEDGER` for the `_v3` evidence, D-4867; no default). A request names a kind-agnostic source word (`zip`, `tickstore`, `dayzips`), resolved per kind (`zip` → `cm_zip` or `nfo_zips`); the resolved id is what claims, plans and progress show. A browser never sends a path. A list travels as one comma-separated field (`kinds=`, `only=`), so the api's one-value-per-field rule stands unchanged. A press is all or nothing, at preflight and at its locks (D-4849). **Round 3 (SF-6):** a claim stays ≤ 1 KiB: it carries the filter's count and BLAKE3 prefix and the repair reason's digest, and the full filter (≤ 256 names) and reason go into a request sidecar beside the lock file, removed at release. |
| D-4802 | Preflight refuses, before any journal line, a root that cannot hold the range and a census without room for the first day's wanted files (`census_room_short`), modelling a v4 compaction (D-4850). Its checks read headers only; the full census decode stays on the refusal path. `Origin` and day tables are cached per source and re-measured when the root changes. `plan.json` is O(days in range) over cached sets, on the bounded store-read pool, fetched on a change, never polled, and counts restated days and each hole class (D-4873, D-4851). |
| D-4815 | Loose day zips are a third source kind, `dayzips`. |
| D-4819 | The FORMAT.md digest is recorded and compared with **the two digests the charter records**: `6edc62a7…3259` (`docs/00-charter.md:1065`, format v1) and `d21139f5…5be9` (`:1108`–`:1109`, §6 as revised 2026-10-04). Today's file is the second, so it is pinned. Any other digest warns, and the run line carries `format_digest_unpinned=1`. The magic decides. The `gdfl_tickstore.rs` module doc cites both rows. |
| D-4834 | `dayzips` reads `<ROOT>/{INDICES,STOCKS,OPTIONS}/<MON_yyyy>/<name>.zip` and nothing else, indexes at most 100,000 entries (`dayzips_too_many_entries`), and refuses a name outside the calendar's span before the day table is sized. |
| D-4850 | **The v4 census preflight models the compaction.** Room is max(`capacity` − `n_valid`, `epoch_capacity(segment, n_keys)` − `n_keys`) from the header alone (`n_keys` is in every header slot), computed by the writer's own `manifest::epoch_capacity`; on v3 it is `MAX_ENTRIES` − `n_valid`. `census_room_short` names both rooms and the first day's need. |
| D-4864 | **The cutover is guarded.** The first plan on a store that never held a claim refuses `legacy_run_may_be_live{key}` while a journal key's last line is `begin`, unless the request acknowledges it; the acknowledgement is in the claim. **Round 3 (BL-8):** round 2 ended this entry "so a rollback to an older build stays possible"; that was already false on the real store (DB-B68, DB-B69) and is withdrawn. This design adds no journal verb, so it raises no reader floor of its own (D-4804). |
| D-4866 | **The tick store as a source**: its day table is built by exact path during the `Origin` walk; a `.bts` outside its folder is `day_file_misfiled`; a `*.bts.tmp` is never read (`FORMAT.md:22`); the writer's own `.lock`, `ingest-log.tsv`, logs and tools are stray entries, never opened, locked or written; a writer extending the store during a run is safe because day files are published by rename (`FORMAT.md:112`; the writer's code is outside this repository, UNVERIFIED). |
````

#### Section 15 "Lands with": header and the IA-3 row (the parent row; D-4802, D-4819, D-4850, D-4864 were IA-3's and are now IA-3b's)

Source: `state/design/ingest-automation.md`, lines 4658-4662, 4668

````text
| D-4888 | **(round 6) Every `audit/<vendor>.<name>` ledger append is locked and closes a torn tail** (critic SF-4; F81). WP-13c merged `request_minutes::append_named` (`request_minutes.rs:853`–`:886`), which writes the gap ledger `audit/<vendor>.gaps` and the repair journal `audit/<vendor>.repairs` with `O_APPEND` and `sync_data`, no lock and no torn-tail rule, while several processes write them: the api's pull path and `cli zerodha-repair --from-store` (Zerodha), and on v4 an INDEX and a CASH repair-reason run at once (`audit/gdfl.repairs`). So every such append goes through one function, `pull::audit_ledger::append(store_root, vendor, name, lines)`: it takes `<store>/audit/<vendor>.<name>.lock` exclusive and blocking (`store::flock`), reads the file's last byte, writes one newline first when that byte is not one (a crash's torn tail stays one malformed line, never joined to the next), then the lines (the line rules `append_named` applies today), then `sync_data`. O(1) per append (bench C-GR-15); readers name a line they cannot parse (the gap ledger's reader counts it among its `faults`). Round 5 had put `audit/gdfl.repairs` under the GDFL journal lock (§5.3); that is withdrawn: one lock per ledger file, taken by the one function every writer calls. **Owners and order:** IA-16 builds `audit_ledger` and moves the repair journal's two writers to it (`ingest.rs:2387`, `:2422`); IA-10b, which edits `request_minutes.rs` after WP-62, moves `append_named` (the gap ledger) to it in the same change as `Plan.run`; until then the gap ledger's unlocked append is named (L-IA-38). Test IA-GR-41. Lands with IA-16. |

**Lands with (round 3, critic BL-8: IA-0 is dissolved).** Round 2 had one package, IA-0, mint all of D-4800..D-4873 before any code, which would have recorded texts that were false at the head (D-4804, D-4864, D-4873) or contradicted by WP-13c (D-4851, D-4857). Now **each entry lands with the first package that implements it**, re-checked against the head that package starts from (§16.3, Re-grounding), and an entry whose substance a fix-round package has since decided is amended before it lands, never minted false. An entry proposed to a fix-round package (D-4874 to WP-12d, D-4875 to WP-13c or WP-12d, D-4878 to WP-12f) is recorded by that package under the number it takes at commit if it builds it; this design's number is then retired by name in the IA package that relies on it, never minted twice. The IA-5 subset is the one that can land now:

| Package | Lands |
| IA-3 | D-4802, D-4814, D-4815, D-4817, D-4818 (round 4, SF-10: its arms change with the per-day refusal), D-4819, D-4828, D-4834, D-4835, D-4850, D-4864, D-4866, D-4867 |
````

#### Section 16.2 The packages: the round-4 note that names IA-3, the table header and the IA-3 row

Source: `state/design/ingest-automation.md`, lines 4806-4808, 4815-4820, 4884-4885, 4890

````text
### 16.2 The packages

Round 3 dissolves IA-0 (each decision lands with its package, §15 "Lands
**Round 4** gives IA-2 the whole stop (SF-11), IA-3 its cause arms and
D-4818's (SF-10), IA-4 the options ledger count (BL-E), IA-16 `admit`'s table
and `decisive()`'s hole order (BL-B, BL-C, SF-18), IA-6 D-4845 instead of IA-5
(BL-F), and adds two packages: **IA-11a**, the alert queue's writer, at step 2
before every caller (SF-7), and **IA-17**, D-4875 built, only if WP-13c or
WP-12d merge without it (SF-5).
| Pkg | Name | Depends on | Files owned (non-overlapping, but for §11's shared lines) | Size | Lane |
|---|---|---|---|---|---|
| IA-3 | Preflight (the v4 census model, the legacy cutover, the calendar horizon, predicted holes; round 5: the free-space line is IA-10a's, from WP-81's api measurement, F76), the source cache's measurements, near-miss naming, per-day zip faults, `archive_prefixed`, the data-descriptor and placeholder-shape tests, `dayzips`, the `_v3` reader, the CM `\` separator, `extension_missing`, the tick store's day table and misfiled files, the member-bytes seam, the ignored real-data tests (spans, and the container facts, SF-17); **the `cm_cause` and `nfo_cause` arms of its new variants and D-4818's three archive arms** (round 4, SF-10), with IA-GR-27 | IA-1 (types), **IA-2** (round 4: its `gdfl_import.rs` edits come after IA-2's), WP-16, WP-12c, **WP-12e** (`gdfl_nfo.rs`), **WP-12f** (`gdfl_cm.rs`) | `gdfl_run_preflight.rs`, `gdfl_archive.rs`, `gdfl_cm.rs`, `gdfl_nfo.rs` (sources), `gdfl_tickstore.rs`, `gdfl_dayzips.rs`, `gdfl_v3.rs`, `gdfl_import.rs` (the cause arms only) | L | B |
````

#### Section 16.2 "Lanes" and "The order" (steps 1 to 3; IA-3 is lane B, step 3)

Source: `state/design/ingest-automation.md`, lines 4910-4988

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
````

#### Section 16.3 CI: the gates each package meets, the table header and the IA-3 row

Source: `state/design/ingest-automation.md`, lines 5015-5033, 5038

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
| IA-3 | none expected; any sort added for a day table lands with its `ALLOW_SORT` count and reason | L-IA-8, L-IA-29; the census projection note under L-IA-11 | IA-GR-07, -15, -16 (its variants), -20, -26, -27, -34 (its half); IA-MX rows of §8 A, B, C, D, E, T | 12: the source cache's cost claims cite C-GI-04, C-GI-05; 27b (D-4818 with its arms) |
````

### 4.8 The round-7 critic (verbatim)

The critic has **no IA-3b part** (it predates the part). Its **IA-3** part is copied whole: its
resolutions are the defaults where the design is silent (IA-3-1 is D-4908, which this part
lands; IA-3-2, D-4909, is the source cache's change key and is NOT this part, since the source
cache is dropped with the unattended layer). The decision row the spec cites (`:572`) and the two
cross-package rules that bind this part's bench line and decision headings (X-3, X-4) follow.

Source: `state/design/critic-round7-20261011.md`, lines 245-261 (## IA-3 — preflight and sources)

````text
## IA-3 — preflight and sources

| Gap | Where | Resolution | Decision |
|---|---|---|---|
| **IA-3-1. The legacy-cutover check is not O(1) as defined.** §5.2: "no `imports/gdfl.lock` or `imports/gdfl.<kind>.lock` has ever held a claim on this store (no GDFL-mode record in the runs ledger)", cost "O(1) on the loaded journal sets". The runs ledger holds both feeds and has no index by mode; finding "no GDFL-mode record" is a scan of every record, and D-4880 adds one record per Zerodha landing request | design §5.2 line 956; D-4864 | Define it by a file that exists or does not: a kind **has claimed** when `imports/gdfl.<kind>.lock` exists (v4) or `imports/gdfl.lock` (v3). Lock files are created only by a run's `plan` taking them and are never deleted; probes open without create, and `ENOENT` reads `Free`. One `metadata`. On the real store the first options press still asks for `legacy_ack` (key `options 2020-09-16 *`), once. IA-GR-20 gains: the check makes one `metadata` beside a ledger of 1,000,000 records | **D-4908** (amends D-4864 before it is minted; lands with IA-3) |
| **IA-3-2. The api's source cache cannot see a day added to a directory source.** It is re-measured "when the root's (dev, ino, mtime, len) changes". For `tickstore`, `dayzips` and the year-zip directory the root is a directory, whose mtime moves only when a direct child is added. A new `.bts` under `cm/INDICES/2026/OCT_2026/` (§8 T9, T12, the case the design itself raises) changes nothing at the root, so `plan.json` keeps the old span, and `dry_plan` answers "nothing to import" and **starts no run** for days the source holds | design §5.1 `SourceCache` line 791, 849; §5.2 lines 949–950; §5.3 lines 1175–1180 | The change key of a directory source is the chain to its newest month folder: per tree, the root, the newest year directory and the newest month directory (≤ 3 `metadata` each, O(1)); for `nfo_zips` each year zip in range (≤ 9). A changed key re-measures off the request path. Every plan answer carries the source's `measured_at`. Matrix row **T9** gains test `a_day_published_after_the_api_started_is_in_the_next_plan` (`IA-MX-T9`), and C-GR-12 counts the `metadata` calls | **D-4909** (amends D-4802; lands with IA-3) |
| **IA-3-3. Dependencies.** WP-12e and WP-12f do not exist (X-7); IA-3 lands C-GI-04 and L-IA-11 (IA-1-5); IA-3's cause arms follow IA-2's `import_cause` and WP-16's `cm_cause` / `nfo_cause` (`gdfl_import.rs:743`, `:796` on WP-16) | X-7, IA-1-5 | as there | D-4900 |

Non-blocking. The member-bytes seam (`bytes: Arc<[u8]>`) should be dropped by
the worker as soon as the file is filed when the run says `verify=off`, or
L-IA-12 pays for bytes nobody reads. `stored_index_absent` needs each alias
index's span in code, not only in the charter: say now that it lives in
`core::alias::INDEX_ROWS` (two optional days per row, `None` until IA-0b), so
IA-16 writes the check against a field that exists and IA-0b only fills it.

---

````

Source: `state/design/critic-round7-20261011.md`, lines 92-95, 98-99 (cross-package gaps X-3 and X-4)

````text
## Cross-package gaps (they bind every package; cited below as X-n)

| Gap | Where | Resolution | Decision |
|---|---|---|---|
| **X-3. Gate 14's bench pins are exact counts on one line per crate, and no rule orders the packages that raise them.** `ci.yml:3566`–`:3576` (`cover=`): "The pin is now an EXACT count: adding a measurement fails until its pin is raised in the same change". `fix/db-IA-5` sets the `store` line to 18 (C-VD-01, -02, -04); `fix/db-IA-11r` sets `telemetry` to 10 and adds ids to `api`. IA-17 adds C-RP-01 to the same `store` line; the `pull` line is raised by IA-11a, IA-1, IA-2, IA-3, IA-16, IA-6, IA-7, IA-8, IA-9 in five lanes. §11's `ci.yml` row names only gates 21 and 23 | design §11 line 4300; §16.3; `ci.yml:3533`–`:3576` | Add to D-4861: **the `cover` line of a crate is a shared exact line.** Each package, as its last step before its gates, merges the head, recounts its bench with the gate's own counter and writes head's ids plus its own; the merger never takes "ours" or "theirs" on that line, it takes the union and the recount. §11's `ci.yml` row lists every bench-adding package | **D-4891** (amends D-4861, which `fix/db-IA-5` already heads: IA-11a, the next package to start, lands it) |
| **X-4. A decision whose parts land with several packages can be headed only once (gate 27b), and the two built branches did it two different ways.** `fix/db-IA-5` heads D-4865 **whole**, with "Implemented by IA-5 (…); IA-16 (…); IA-8 (…)". `fix/db-IA-11r` heads D-4813 with **only its own part** (the runs page). §15's "Lands with" gives halves of one number to two packages: D-4813 (IA-11r, IA-6), D-4856 (IA-6, IA-7), D-4874 (IA-1 or IA-16), D-4878 (IA-6 and WP-12f), D-4880 (IA-2, IA-10b), D-4845, D-4871 ("the first IA package to merge": neither built branch heads it) | design §15 lines 4660–4681; `fix/db-IA-5:docs/05-decisions.md:66682`; `fix/db-IA-11r:docs/05-decisions.md:66516` | **IA-5's way is the rule**: the first package to land a number heads the whole §15 text, re-grounded, with an "Implemented by" line naming each package and part; a later package never adds a second heading and never edits that entry: its proof is its own `docs/04` rows. Where the first lander already headed only its part (D-4813), the later part takes a new number that says "continues D-48xx": **D-4892** = D-4813's verification half (one event per (run, segment, timeframe, day), at most 16 named mismatches, never code 6), lands with IA-6. **D-4871** lands with IA-11a (not started, so it can) | **D-4892**; D-4871 with IA-11a |
````

Source: `state/design/critic-round7-20261011.md`, lines 549-552, 572-573 (decisions proposed by the review: D-4908, D-4909)

````text
### Decisions proposed by this review (D-4888 is the design's last; re-grep at landing)

| D | What | Lands with |
|---|---|---|
| D-4908 | "Has claimed" is the lock file's existence: one `metadata` | IA-3 |
| D-4909 | A directory source's change key is the chain to its newest month folder | IA-3 |
````

## 5. Files this part owns

From the spec's OWNED FILES (section 3.1), all **new** (none exists at e851da1ca):

| File | What |
|---|---|
| `crates/pull/src/gdfl_run_preflight.rs` | every check of items (1)-(9); declared from `gdfl_run.rs` with `#[path]` (one `mod` line) and called once in `plan`, after the scope is resolved and before any lock, claim or journal line. Design section 11 (line 4236) names the same file. R4-29 extends it after you (PERF-13 floor) |
| `crates/pull/src/gdfl_run_preflight_tests.rs` | the failing-first tests (the 21 names in section 3.1) and the extreme cases |
| `crates/pull/benches/ratio/gdfl_preflight.rs` | bench C-GR-05 (p50, p99, max, n >= 200; cold first plan and repeated plan printed apart; the one listing read at the largest options day, 26,820 entries) |

The ledgers this part lands its own rows in, in the same change (D-4861; IA packages do this
themselves): `docs/05-decisions.md` (D-4802, D-4819, D-4850, D-4864, D-4908, the D-4815
amendment, and D-7000.. for any other locked choice), `docs/04-invariants.md` (a row beside every
new test, including matrix rows `IA-MX-A23`, `IA-MX-B28`, `IA-MX-B28a`), `docs/06-limits.md` (the
measured rows: the largest member against the cap with its margin, the refusal path's cost of the
legacy check, C-GR-05, anything not O(1), and any check that is NOT DONE by name).

### SEQUENCED EDITS outside ownership (each listed in your report's files_outside_ownership)

Minimal, and only these (from the spec, plus the edits the head makes necessary, marked *; the
gate 14 row says why no `ci.yml` edit is expected):

| File | The only edit allowed | Also edited by |
|---|---|---|
| `crates/pull/src/gdfl_run.rs` | the new `RunRefusal` variants (appended; codes never renamed or reused) and their `code()` arms, the `#[path]` mod line, the one call in `plan`, and the doc line 41 (`dayzips` is IA-3's -> names R4-11). * **Also needed by item (6), and not in the spec's list:** one `Request` field carrying the acknowledgement into `plan` (the head's precedent is `accept_open_month`: field :423, `to_line` :530/:547, parsed back at :657), because `plan` reads only the `Request`. The claim half already exists: `ClaimAsk.legacy_ack` (:1666) -> `claim_template` (:1711) -> `Claim.legacy_ack`, written `legacy_ack=` (`gdfl_run_locks.rs:861`, :1024); do not add a second claim field | IA-16 (merged), R3-23, R3-26 |
| `crates/pull/src/gdfl_archive.rs` | `member_bytes`'s size check before any read of packed bytes, and the empty-directory arm | - |
| `crates/pull/src/gdfl_tickstore.rs` | `rebuild`'s size check, and the empty-index arm | - |
| `crates/pull/src/gdfl_import_causes.rs` | the `cm_cause` arms of the new per-day variants: exhaustive, no `_` arm (`Cause::open`) | R4-22, R3-26, IA-4, IA-3-OPT (merged) |
| `crates/pull/src/gdfl_nfo.rs` | the empty nested day zip only; keep IA-3-OPT's `NfoZips` edits | IA-3-OPT (merged) |
| `crates/cli/src/gdfl_import_command.rs` | `--legacy-run-not-live` only (a `pub(crate) const` beside `--accept-open-month` at :113, parsed into the request as `accept_open_month` is at :149, :208, :338), and the `ClaimAsk` it builds at :519-:533 (`claim_template`), which today hard-codes `legacy_ack: false` (:529): set it from the flag, so the claim records the acknowledgement | owner R4-29; also R3-23, IA-16 (merged), R4-11 |
| `crates/pull/src/gdfl_import_filing.rs` (IA-16's store-level classifier, `DayFiler::ends_with`) | the one EMFILE arm naming `open_files_exhausted`. **It cannot be written from what reaches it today** (section 9): `ends_with` sees `FailKind::Io { kind: ErrorKind, .. }` only, and std names no kind for EMFILE (it becomes the unnameable `Uncategorized`), so the raw error number must be carried from the one place an `io::Error` becomes a `StoreError` (`crates/store/src/file.rs:6242` `classify`, `StoreError::Io` :536) through `FailKind::of_store` (`ingest_failkind.rs:103`, `:113`). Either carry it (each such file listed in files_outside_ownership, with a docs/05 entry from D-7000..) or mark the arm NOT DONE by name in docs/06; never match the error's display text | IA-16 (merged) |
| * `crates/pull/src/gdfl_cm.rs` | only the new `CmRefusal` variants (`day_zip_empty`, the import-side `member_too_large`) and their code/Display arms: `CmRefusal` is declared here (:266), not in `gdfl_import_causes.rs` | WP-12f (merged) |
| * `crates/pull/benches/ratio.rs` | one `#[path = "ratio/gdfl_preflight.rs"] mod gdfl_preflight;` pair and its `ok &= gdfl_preflight::...();` call in `main` (:1840), as the other `ratio/*.rs` modules are (lines 57-78; `day_table::day_lookup_is_flat()` is C-GI-04's) | every bench-adding part |
| (no edit expected) `.github/workflows/ci.yml` gate 14 `cover=` line for `pull` (:3878, pin 38) | **Leave it unless the recount changes.** Gate 14 reads only the file the row names, `crates/pull/benches/ratio.rs` (`.github/gates_bounds.rs` `gate14`, :1023): layer 3 pins the exact count of `ratio(` call sites in that file's code, layer 4 needs each listed id to be the first word of a string literal printed from that same file and a `docs/04` row. A bench living in `ratio/gdfl_preflight.rs` changes neither count, which is why C-GI-04, C-TSW-01/02, C-ZI-01..03, C-DP-01/02 and C-PIT-01 (all printed from `ratio/*.rs` modules) are not on the line and the pin is still 38. Merge the head last, recount with the gate's own counter (critic X-3, D-4891), and change the line only if your change adds a `ratio(` call or a printed id to `ratio.rs` itself; adding `C-GR-05` to the line while it is printed only from the module fails layer 4 (`in-bench=0`). Gate 8 (`cargo bench`) is what runs C-GR-05 and fails on its ratio | every bench-adding part |
| * root `Cargo.toml` `rustix` features (only if you read the open-file limit through `rustix::process::getrlimit`, which needs the `process` feature; today `["std", "fs"]`, line 148) | add `"process"`, with the reason in the comment above it | - |

If R4-05's reasons dictionary is on `feat/db-fix` when you commit, add one sentence per new code
there; at e851da1ca it is not (R4-05 not merged), so R4-05 takes them.

## 6. Files of other parts: do not touch

- `crates/cli/src/gdfl_import_guard.rs` (AF-01's door: `source_root_unknown`, `run_busy`,
  `no_file_read`, `source_lost`, `store_lost`; cite and call, never rebuild). Its two FM-060 tests
  are re-run unchanged: `gdfl_import_guard_tests.rs:481`
  `a_wrong_source_root_is_refused_before_any_day_is_journalled` and
  `gdfl_import_command_tests.rs:343` `a_run_that_read_no_file_never_exits_ok`.
- `crates/pull/src/gdfl_dayzips.rs`, `gdfl_v3.rs`, `manifest.rs`, `census_store.rs`,
  `gdfl_import_census.rs` (call `manifest::epoch_capacity` and keep `census_room` on the refusal
  path; never copy them), `calendar.rs`, `tickstore_write.rs` (use `MEMBER_CAP`, `LOCK_NAME`
  and the `member_too_large{name, size}` code; never a second cap unless measured, section 3.1
  item 3), `disk_projection.rs` (R4-07's one floor: call `floor_check`), `gdfl_import.rs`,
  `gdfl_import_day.rs`.
- R4-11's one-command files and its preflight codes (`volume_not_mounted`,
  `started_inside_sandbox`, the `BRUTEX_READ_ONLY` writer gate): keep your codes distinct and cite
  them in docs/05.
- R3-23's durable refusal line (FM-084), R4-05's reasons dictionary, CC-1's archive scrub, IA-10a's
  receipt (the calendar-horizon warning, item 7b), R3-25's gauge tile and R3-33's page (RA-207's
  gauge and early warning), R3-19's census publish (RA-011). No `web/` change, no api route.
- No timer, watcher, folder watch, source cache, alert kind or unattended job (D-6900).

## 7. Decision numbers

From `state/decision-blocks.json` (section 3.4): **D-7000..D-7009**, owner "IA-3b (20 Oct
follow-ups; also lands the design ids D-4802, D-4819, D-4850, D-4864 and the critic id D-4908)".
Take ids in order; an unused id stays unused.

| Id | What | State at e851da1ca |
|---|---|---|
| D-4802 | preflight refuses before any journal line (design 15, line 4572; amended by D-4909 only for the source cache, which is not this part) | no heading in `docs/05-decisions.md` on `feat/db-fix`, nor on any of 580 `fix/*` and `origin/*` refs (grepped 24 Oct) |
| D-4819 | the FORMAT.md digest compare (line 4589) | none anywhere |
| D-4850 | the v4 census preflight models the compaction (line 4620) | none anywhere |
| D-4864 | the legacy cutover guard (line 4634) | none anywhere |
| D-4908 | "has claimed" is the lock file's existence: one `metadata` (critic line 572; amends D-4864 before it is minted) | none anywhere |
| D-4815 | already headed on `feat/db-fix` (`docs/05-decisions.md:77794`, "Loose day zips are a third GDFL source kind, `dayzips` — 2026-10-20"): **amend it with a new entry from D-7000..**, saying no `dayzips` word is added to `cli gdfl-import` (item 10) | headed once |
| D-7000..D-7009 | every other locked choice: the year-zip rule shared with IA-3-OPT's fiscal-year misfile check (item 1), how the volume flags and the open-file limit are read (item 7), a second member cap only if measured (item 3), the D-4815 amendment (item 10) | none of D-7000..D-7009 is headed on any ref |

Highest D heading on `feat/db-fix` at e851da1ca: D-7138. Re-grep `docs/05-decisions.md` on
`origin/feat/db-fix` and every `origin/*` branch before committing (gate 27b refuses a number
heading two entries). Entry heading shape at the head: `### D-NNNN — <title> — <yyyy-mm-dd>`.
Follow critic X-4: the first package to land a number heads the whole text, re-grounded on the
code at the head, with an "Implemented by" line.

## 8. NEEDS THE MAC

What the Mac builder would run in its step 5 (and the spec's REAL DATA and BENCH clauses), written
as commands the Mac runs at merge-back, on the cloud branch checked out at
`$W/wt/P-IA-3b`. All of it is **read-only on the real store, the tick store and every source**;
scratch stores live under the system temp dir only. The cloud session names its `#[ignore]`
real-data tests exactly as below (or lists its own names in its report) so these commands run
unchanged.

````text
W=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925
P=$W/wt/P-IA-3b
# roots exactly as IA-3's report gives them (Mac paths, no secret):
export BRUTEX_GDFL_TICKSTORE=/Volumes/WD_BLACK/brutex/tickstore-data
export BRUTEX_GDFL_CM_ARCHIVE='/Volumes/WD_BLACK/NSE (Stock+Indices)_01.09.2018 to 24.09.2026_Tick.zip'
export BRUTEX_GDFL_NFO_ARCHIVES=/Volumes/WD_BLACK/Options
export BRUTEX_GDFL_DAYZIPS=$W/tools-work/update-20260925-30/downloads
export CARGO_TARGET_DIR=$W/tgt/pool-4      # take the pool lock and touch the tree first (launcher step 2)
````

| # | Check | Command (Mac) | Expected answer |
|---|---|---|---|
| M1 | Every preflight check's answer and its time on the real roots | `(cd $P && cargo test --release -p pull --lib -- --ignored --nocapture --test-threads=1 ia3b_real_preflight_prints_every_check_and_its_time)` | Every check passes for indices, stocks and options over 2018-09-03..2026-09-24: year zips 2018..2026 all present and readable (2021-26 are mode 0600 and nlink 2 but owned by the operator, so readable); none empty; no `source_changing`; tick-store `.lock` probe free (`source_busy` only if the tick-store writer is running: say which); FORMAT.md digest `d21139f5…5be9`, pinned, no `format_digest_unpinned`; `store_read_only` and `store_not_local` pass (WD_BLACK is a local volume mounted read-write); `source_span`: CM zip 2018-09-03..2026-09-24, options zips the same 1,997 days, tick store through 2026-09-30. Each check's time printed |
| M2 | A range wider than the source | same test, or `cli gdfl-import indices zip 2018-01-01 2018-09-07 NIFTY` on a scratch store | the plan names `source_span{2018-09-03, 2026-09-24}` and counts 2018-01-01..2018-09-02 as `outside_source` (calendar-open days only), never as holes; the journal is unchanged |
| M3 | A year with no zip | `cli gdfl-import options zip 2017-12-01 2018-09-10` on a scratch store | `GDFL IMPORT REFUSED: year_zip_missing{year: 2017, near_miss: []}` before any journal line, lock or progress record (the `.DS_Store` beside the zips is not a near miss) |
| M4 | The largest member of each container, against the cap (item 3) | `(cd $P && cargo test --release -p pull --lib -- --ignored --nocapture --test-threads=1 ia3b_real_largest_member_of_each_container)` | CM archive: about 15,279,399 B (an index member of 2026-09-18, design 2.1); options year zips: about 2,359,609 B (2019-05-30); tick store: largest stated entry, measured (not in the design: UNMEASURED until this run). Each against `tickstore_write::MEMBER_CAP` = 2 GiB (2,147,483,648 B) with its margin; the docs/06 rows take these numbers |
| M5 | The legacy cutover on the real journal | copy only the journal into a scratch root: `S=$(mktemp -d -t ia3b-legacy); mkdir -p $S/imports; cp -c /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store/imports/gdfl.journal $S/imports/; BRUTEX_STORE=$S $W/tgt/bin-IA-3b/release/cli gdfl-import options zip 2020-09-16 2020-09-16; ls -la $S/imports` | `GDFL IMPORT REFUSED: legacy_run_may_be_live{key: options 2020-09-16 *}` (measured read-only 24 Oct: the real store has no `imports/gdfl*.lock` and that key's last line is `begin`; it is the only such key). `ls` shows **no** `.lock` file created. With `--legacy-run-not-live` the plan passes the check and the claim records the acknowledgement. For indices and stocks: refused only if the check is per kind as the critic's IA-3-1 defines (no open key of those kinds); the build says which in docs/05. If a check that runs before (6) refuses first on this bare root (the scratch root has no census, so item 5's header read has nothing to read), do not copy the census (a v4 epoch is up to 1 GiB, design L-IA-11, and the temp dir is on the internal disk, not WD_BLACK, so `cp -c` cannot clone it): drive the legacy check alone through an `#[ignore]` test that reads the real `imports/gdfl.journal` and probes the real `imports/gdfl*.lock` names by `metadata` only, read-only, and report which was run |
| M6 | Census room on the real (v4) census | M1 prints `free_now`, `free_after`, `need_first_day` per kind (2 header slot reads + one listing read) | no kind refused (10 Oct: FNO about 8.0M free entries before a compaction, INDEX and CASH about 1.03M; re-read the header now); `census.projected_stop` printed only when the range may need more, labelled "extrapolation from the first day" |
| M7 | The open-file limit (item 7) | `launchctl limit maxfiles` (measured 20 Oct: soft 256); then in a fresh Terminal (soft 256): `cli gdfl-import options zip 2024-04-04 2024-04-04` on a scratch store | `GDFL IMPORT REFUSED: open_files_short{have: 256, need: <workers x measured files per worker>}` when need > 256, with the operator's hand step in the sentence; passes when raised. `need` is the measured number (L-IA-35, IA-GR-08), never typed |
| M8 | A read-only store | `hdiutil create -size 64m -fs APFS -volname IA3BRO $TMPDIR/ia3b-ro.dmg && hdiutil attach -readonly $TMPDIR/ia3b-ro.dmg`, then `(cd $P && BRUTEX_IA3B_RO_ROOT=/Volumes/IA3BRO cargo test --release -p pull --lib -- --ignored --nocapture a_read_only_store_is_refused_before_any_line)`, then `hdiutil detach /Volumes/IA3BRO` | `store_read_only{root}` before any line, with IA-16's `store_not_writable` sentence words; nothing created on the image |
| M9 | The import is unchanged when every check passes | `(cd $P && cargo test --release -p pull --lib -- --ignored --nocapture --test-threads=1 real_days_close_with_the_same_listing_from_two_sources)` (IA-3's test) on feat/db-fix's build and on IA-3b's | identical close lines for NIFTY spot 2024-04-01, the NIFTY options day 2024-04-04 (IA-3 measured files=973, seconds=1,869,196, entries=11065) and the 2024-11-01 special session (seconds=3577): same files, seconds, skipped and listing digest; then one BANKNIFTY selection, one F&O stock and one illiquid strike (NIFTY FIRST) |
| M10 | Bench C-GR-05 on the operator's Mac | `(cd $P && cargo bench -p pull --bench ratio 2>&1 \| grep -A12 C-GR-05)` | p50, p99, max (n >= 200) of the fixed checks, cold first plan and repeated plan apart, and the one listing read at the largest options day (26,820 entries); every non-O(1) part named in docs/06; IA-13 re-measures it later |
| M11 | Shadow-server regression (no route changes) | `$W/state/fix/stop.sh IA-3b; $W/state/fix/shadow.sh IA-3b && $W/state/fix/serve.sh IA-3b 8506 $W/tgt/bin-IA-3b $P`; `lsof -nP -iTCP:8506 -sTCP:LISTEN -t` equals `cat $W/shadow/IA-3b/test-server.pid`; `for i in $(seq 200); do curl -s -o /dev/null -w '%{time_total}\n' 'http://127.0.0.1:8506/imports.json?feed=gdfl'; done \| sort -n` and the same for `/logs`; `$W/state/fix/stop.sh IA-3b` | GET only; answers equal feat/db-fix's; p50/p99/max reported with n=200. Never POST there, never run an import against the shadow |
| M12 | Gates the cloud may lack | `cargo deny check`; `cargo llvm-cov --branch -p pull -p cli`; `cargo mutants --in-diff <diff>` on the touched modules | green; 100% line and branch on the touched modules; no surviving mutant |
| M13 | Nothing touched the real store | before and after M1-M11: `ls -la /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store/imports /Volumes/WD_BLACK/brutex/tickstore-data` | identical listings (sizes and mtimes); no new `.lock` anywhere |

Store writes needed on the real store: **none** (a preflight writes nothing; the first real
options press after merge needs the operator's `--legacy-run-not-live` once, which is the
operator's call, not a merge step).

## 9. Code facts at e851da1ca (grepped, not guessed)

Line numbers the spec quotes were taken at 453de6989 (20 Oct); several moved. The head's are:

| File | Exists | Key facts |
|---|---|---|
| `crates/pull/src/gdfl_run_preflight.rs` | **no** | new |
| `crates/pull/src/gdfl_run_preflight_tests.rs` | **no** | new |
| `crates/pull/benches/ratio/gdfl_preflight.rs` | **no** | new; siblings in `benches/ratio/`: `day_table.rs` (C-GI-04), `disk_projection.rs`, `fno_membership.rs`, `identity.rs`, `tickstore_write.rs`; each is a `#[path]` module of `benches/ratio.rs` (lines 57-78) called from its `main` (:1840); `harness = false`, `test = false` (`crates/pull/Cargo.toml:159`) |
| `crates/pull/src/gdfl_run.rs` (2,898 lines) | yes | doc line **41**: "`dayzips` is IA-3's" (item 10 corrects it to R4-11); `pub enum RunRefusal` at **:851** (spec said :747): variants `RequestLineInvalid`, `StoredWithOptions`, `RangeBackwards`, `EmptySymbol`, `FilterTooLong`, `SnapshotInvalid`, `LogUnavailable`, `Unstamped`, `SourceUnset`, `Scope`, `SourceUnopened`, `SymbolInvalid`, `RunBusy`, `RunLockRefused`, `LockReplaced`, `LockStalled`, `CensusLayoutChanged`, `CensusUnreadable`, `RunsLedgerDamaged`, `ClockImplausible`, `RunsLedgerUnwritten`, `ClaimUnwritten`, `RepairReasonInvalid`, `RepairInOpenMonth` (append after the last); `impl RunRefusal` at **:1024**, its `pub const fn code()` at :1027 (`SourceUnset` codes `source_unset`); `pub struct Request` :379 (its last fields `accept_open_month` :423 and `free_floor`, written by `to_line` :530 and parsed back at :647-:669: the pattern for the acknowledgement field of item 6); `pub struct ClaimAsk` :1652 with `legacy_ack` :1666, copied into the claim by `claim_template` :1678 (:1711); `pub struct Plan` :1251; `pub fn plan(request, process, env, cache: &mut SourceCache) -> Result<Plan, RunRefusal>` at **:1271** (spec :1133): order is misuse, `RepairInOpenMonth`, log sink / `held_by_a_run`, `Unstamped`, then per kind `resolve_source` (:207), `source_root` (:221, `SourceUnset`), `scope::for_request`, `cache.origin(...)` (:1409), then `Ok(Plan)` at about :1325: **the preflight call goes after the per-kind scope loop and before `Ok(Plan)`**, and plan takes no lock itself (locks are IA-2's `gdfl_run_locks.rs`, taken after plan); `pub struct SourceCache` at **:1398** (spec :1249); existing `#[path]` mods: `gdfl_run_scope.rs`, `gdfl_run_locks.rs`, `gdfl_run_ledger.rs`, `gdfl_run_stop.rs`, `gdfl_run_start.rs` (:62-79) |
| `crates/cli/src/gdfl_import_command.rs` (966 lines) | yes | grammar `cli gdfl-import KIND SOURCE FROM TO [SYMBOL ...]`, store root `BRUTEX_STORE`; flags as `pub(crate) const` (:103 `--retry-refused`, :105 `--stored`, :107 `--snapshot`, :110 `--repair-reason`, :113 `--accept-open-month`); `gdfl_run::plan(request, &process, env, &mut SourceCache::default())` at **:494** (spec :421), then AF-01's `guard::Door::open` at **:537** (spec :464); prints `GDFL IMPORT REFUSED: ...` at :479 and :505 |
| `crates/cli/src/gdfl_import_guard.rs` (1,470 lines) | yes | AF-01's codes: `RUN_BUSY` :84, `SOURCE_ROOT_UNKNOWN` :91, `NO_FILE_READ` :93, `STORE_LOST` :95, `SOURCE_LOST` :98, `SOURCE_CHANGED` (`source_changed_since_last_run`) :100, `SOURCE_CHANGED_UNCHECKED` :106; `root_fault` :223 (one `metadata`); `identity(source, digest, tail)` :976 is the **run-to-run** zip digest identity of `imports/gdfl.sources`, not a per-file (len, inode, mtime, ctime) identity: item (4)'s identity is new unless CC-1 lands one first (CC-1 not merged) |
| `crates/pull/src/gdfl_archive.rs` (2,491 lines) | yes | `Archive::from_source` :592; `day_zip_span` :694; `pub fn member_bytes<B: ReadAt>(source, at: ZipLocator, entry: &str, len: u64) -> Result<Vec<u8>, CmRefusal>` at **:836** (encrypted check first, then the span; the size check goes before any read); `CENTRAL_CAP` 64 MiB :82; an empty archive today "holds no day" (`an_empty_archive_holds_no_day`, :1738: `day()` answers `Ok(None)`), which is exactly RA-322's silent case |
| `crates/pull/src/gdfl_tickstore.rs` (1,957 lines) | yes | module doc :5-11 quotes both charter digests in full (`6edc62a7…3259` :7, `d21139f5…5be9` :10); `pub fn rebuild<B: ReadAt>(src, entry: &IndexEntry) -> Result<Vec<u8>, CmRefusal>` at **:298**; `TICKSTORE_WALK_CAP` 1,000,000 :532; `pub struct DayTable` :548 (D-4866), walked once via `OnceLock` (:693-718); an empty index today fails as "no zstd frame" (:1221 test doc), not by a named code |
| `crates/pull/src/gdfl_cm.rs` | yes | `pub enum CmRefusal` at **:266** (the per-day variants live here) |
| `crates/pull/src/gdfl_nfo.rs` (1,758 lines) | yes | `pub enum NfoRefusal` :188; `pub struct NfoZips` :1141; `fn year(&self, years, year: u16)` :1212; `fn day(&self, day)` :1377 refusing `CmRefusal::DayZipMisfiled` at :1394 (IA-3-OPT's fiscal-year misfile check, FM-043) |
| `crates/pull/src/gdfl_import_causes.rs` (1,092 lines) | yes | `pub enum ImportRefusal` :17 (has `StoreNotWritable` :260, code `store_not_writable` :363); `pub const fn cm_cause(&CmRefusal) -> Cause` **:887**, `nfo_cause` :961, `import_cause` :1011: none has a `_` arm (the two `_ =>` in the file, :385 and :405, are in other matches) |
| `crates/pull/src/gdfl_import_filing.rs` | yes | IA-16's store-level classifier: `DayFiler::ends_with(kind: FailKind, why, filed)` at **:770**: `DiskFull` -> `disk_full`; root lost -> `store_lost`; `FailKind::Io` with `PermissionDenied` or `ReadOnlyFilesystem` -> `not_writable_said(why)` (:515, the FM-129 sentence to reuse for `store_read_only`). The EMFILE arm (`open_files_exhausted`) goes here, one arm; it is called from :740 with `failure.why`, and `kind` there holds only an `ErrorKind` (see `ingest_failkind.rs` and `store/src/file.rs` below) |
| `crates/pull/src/gdfl_dayzips.rs` (497 lines) | yes | `DAYZIPS_WALK_CAP` 100,000 :53; `dayzips_too_many_entries` :386; `pub struct DayZips` :237 (D-4834) |
| `crates/pull/src/manifest.rs` (4,578 lines) | yes | `MAX_ENTRIES` 2,097,152 :374; `pub const fn epoch_capacity(segment: Segment, keys: u64) -> Result<u64, ManifestError>` **:577** (unchanged); `pub struct ManifestHeader` :2250 with `n_valid`, `n_keys` (:2266) and the v4 `capacity` field (read from bytes 12..16; `ManifestHeader::limit` is the number every check uses) |
| `crates/pull/src/gdfl_import_census.rs` (283 lines) | yes | `pub(super) fn census_room(store_root, held) -> Result<CensusRoom, String>` at **:108** (spec :98): v3 only, full decode, refuses a v4 census by name; stays on the refusal path |
| `crates/pull/src/calendar.rs` (6,615 lines) | yes | `SOURCED_LAST_DAY` :990; `record_horizon(newest_day)` :1056; `past_record(first, last)` :1079; `RECORD_ENDS_SOON_DAYS` is named only in a doc comment (:1029), there is no const of that name (not this part's) |
| `crates/pull/src/tickstore_write.rs` (1,953 lines) | yes | `LOCK_NAME = ".lock"` :120; `MEMBER_CAP: u64 = 2 << 30` (2 GiB) :126; its `MemberTooLarge { name: String, size: u64 }` (:334) codes `member_too_large` (:463), Display "`{name}` states {size} bytes, over {MEMBER_CAP}" (:519); `NoFileInDay` -> `no_file_in_day` (:461) |
| `crates/pull/src/disk_projection.rs` (731 lines) | yes | R4-07's floor: `pub fn floor_check(space: Space, projected: u64, floor: Floor) -> Result<Headroom, FloorRefusal>` :652, `floor_of` :720, refusal code `disk_floor` (:595); the floor setting is `store::settings::Key::DiskFloor` (`crates/store/src/settings.rs:128`) |
| `crates/pull/src/gdfl_import.rs` | yes | `pub fn volume_space(path) -> Result<disk_projection::Space, String>` (:967): one `rustix::fs::statfs` on Apple, `statvfs` elsewhere, no child process: **the precedent for reading the volume's flags (item 7)** |
| `crates/api/src/disk_route.rs` | yes | **the spec's precedent is false at the head**: `/bin/df -Pk` was removed (D-5240, "NO CHILD PROCESS", :209); `/disk.json` reads one `statfs(2)`/`statvfs(2)` through `rustix`. `crates/vocab/tests/no_runtime_spawn.rs` refuses any backend `std::process::Command` outside a two-entry allowlist (the browser opener and the watchdog's notice page). So **no** `std::process::Command` for item (7): read the volume flags from `rustix::fs::statfs` (`f_flags`, on macOS `MNT_RDONLY` and `MNT_LOCAL`; cite their values' source in docs/00-charter or mark UNVERIFIED) and the soft open-file limit from `rustix::process::getrlimit(Resource::Nofile)` (needs the `process` feature on the workspace `rustix`, today `features = ["std", "fs"]`, root `Cargo.toml:148`). If no exact read exists, the check is NOT DONE in docs/06 by name |
| `crates/pull/Cargo.toml` | yes | already depends on `rustix` (workspace, lines 112-116, comment: one `statfs(2)` for the free-space floor, no child process) |
| `.github/workflows/ci.yml` | yes | gate 14 `cover=` at :3876; the `pull` line (:3878) pins **38** `ratio(` call sites and lists 17 ids (C-11 .. C-ZV-03), all counted in `crates/pull/benches/ratio.rs` alone (`.github/gates_bounds.rs` `gate14` :1023: layer 3 counts that file's code, layer 4 needs each id printed from that file). None of the `ratio/*.rs` module ids (C-GI-04, C-TSW-01/02, C-ZI-01..03, C-DP-01/02, C-PIT-01) is on the line, so a C-GR-05 printed only from `ratio/gdfl_preflight.rs` leaves the line unchanged (section 5) |
| `crates/pull/src/ingest_failkind.rs` | yes | `pub enum FailKind` :35; `Io { kind: ErrorKind, publication_may_be_visible }` :65 carries **no raw error number**; `FailKind::of_store` :103 maps `StoreError::Io { kind, .. }` to it (:113) |
| `crates/store/src/file.rs` | yes | `classify(path, action, refusal: &io::Error) -> StoreError` :6242, "the one place an `io::Error` becomes a `StoreError`": `StorageFull`, `PermissionDenied`, `ReadOnlyFilesystem`, `IsADirectory`, `NotADirectory`, `NotFound` get their own variants and every other kind becomes `StoreError::Io { path, action, kind }` (:536), dropping the error number. std (toolchain 1.97.1, `sys/io/error/unix.rs` `decode_error_kind`) maps no named kind to EMFILE, so an EMFILE arrives as `ErrorKind::Uncategorized`, which stable Rust cannot name in a match: the `open_files_exhausted` arm needs the error number carried (section 5) |
| `crates/store/src/flock.rs` | yes | `try_lock_shared(file, path)` :580 (shared, non-blocking, `WouldBlock` while another description holds it exclusively): the probe item (4) needs for the tick-store `.lock`, opened without create. R4-08's writer takes that lock with std `File::try_lock` (exclusive) in `lock_root` (`tickstore_write.rs:1782`, opened with `create(true)`), so the shared probe sees it. The 11 `.lock` files the external writer left in `tickstore-data` (design 2.1; its code is not in this repository) use a mechanism that is UNVERIFIED |
| `docs/04-invariants.md` | yes | no row `IA-GR-07`, `-15`, `-20`, `-26`, `IA-MX-A9`, `-A11`, `-A23`, `-B28`, `-B28a` or `C-GR-05` yet |
| `docs/06-limits.md` | yes | no `L-IA-35` and no `C-GR-05` row yet |
| `docs/05-decisions.md` | yes | D-4815 headed at :77794; D-4802, D-4819, D-4850, D-4864, D-4908 and D-7000..D-7009 not headed; highest heading D-7138 |
| real store (read-only look, 24 Oct) | - | `runtime/store/imports/` holds `gdfl.dayflags`, `gdfl.journal` (14,027,805 B), `gdfl.lineage`, `gdfl.lots`, and **no** `gdfl.lock` or `gdfl.<kind>.lock`; journal verbs: 11,519 `begin`, 11,291 `done`, 226 `incomplete`, 210 `recounted`, 24 `run`; the only key whose last line is `begin` is `options 2020-09-16 *` |

### Things to know before you start

- The spec's "ALREADY AT HEAD" list is right in substance; only its line numbers moved (above).
- `RunRefusal::SourceUnset` is the head's name for the design's `source_not_configured`; keep it.
- `plan` is pure of locks today (IA-2's locks come after it in the cli): "before any lock, claim or
  journal line" holds by placing the call inside `plan`. Every refusal test still asserts no
  journal line, no progress record and no lock file was written.
- `member_too_large` must fire before `member_bytes` reads packed bytes and before `rebuild`
  inflates; today neither has a size check beyond the deflate-bomb `len + 1` limit (B10, B26).
- An empty year zip, an empty nested day zip and an empty tick-store index are silent or unnamed
  today (`Ok(None)`, "no zstd frame"): each gets its own named code (item 2), with matrix rows
  A23 and B28a.
- Item 6's acknowledgement has its claim half already: `ClaimAsk.legacy_ack` -> `Claim.legacy_ack`
  -> the claim line's `legacy_ack=` field (IA-2's). What is missing is the way in: the cli
  hard-codes `legacy_ack: false` (`gdfl_import_command.rs:529`) and `Request` has no field `plan`
  could read, so the preflight cannot see the flag until both are added (section 5).
- Item 7's EMFILE arm cannot be told apart from other host errors with what reaches IA-16's
  classifier today (`ErrorKind` only; std gives EMFILE no named kind). Carry the error number from
  `store::file::classify`, listing every file it touches, or name the arm NOT DONE in docs/06;
  never match the error's display text (std's wording is not a contract, so a changed sentence
  would silently file EMFILE as `store_io` again).
- Gate 14 needs no `ci.yml` edit for a bench printed from `benches/ratio/gdfl_preflight.rs`
  (section 5); gate 8's `cargo bench` runs it and fails on its ratio.

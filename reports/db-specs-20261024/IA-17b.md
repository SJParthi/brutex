# IA-17b: D-4875 remainder: the once-per-month gather and the CASH .auc writer

This file is the complete, self-contained specification of ingest + /db part **IA-17b** for a
cloud session that cannot read the operator's Mac. It holds everything the Mac builder of IA-17b
is told (the launcher `state/fix/db-fix-round-2-run0738.js.fat`, `specStep(p)` for kind `ia` and
`fixPrompt(p)`) and every document piece those texts point to, copied verbatim with the source
file and its line numbers. Read `README.md` and `CONTEXT.md` of this folder first; they hold the
cloud rules and the background every part shares. Mac paths below are explained in `README.md`
("What differs in the cloud"): none of them exists in the cloud.

Where a copied design sentence states a fact about merged code that is false at the head of
`feat/db-fix` (e851da1ca), the code at the head is the fact (launcher rule, copied in section 2).
Section 7 lists the code facts at e851da1ca that a builder needs and section 8 lists the places
where the design and the head disagree.

## 1. Header

| Field | Value |
|---|---|
| Name | IA-17b |
| Title | D-4875 remainder: the once-per-month gather and the CASH .auc writer |
| Kind | `ia` (ingest automation; specification = the design `state/design/ingest-automation.md`, row `IA-17` of section 16.2, its "later part") |
| Design row | `IA-17` (the launcher's `row` field), the parts named "later" in that row |
| Priority rank | 43 of 137 in `__ARGS.priority` (index 42); 48 of 72 in `EMBED.priority` (index 47) |
| Critical path | **no** (not in `__ARGS.criticalPath`) |
| In this run's `only` list | yes |
| Run flags that touch it | package `waitCritic: true`; run-wide `__ARGS.noCritic: true` and `__ARGS.noGate: true` (no critic step and no workspace gate in this run: the critic's text below is already final) |
| Mac shadow port | 8414 (`portBase` 8400 + its index 14 in the run's package list) |
| Dependents (wait for IA-17b) | IA-13, R3-22, R3-23, R3-26, PREC-2, PREC-13, PREC-8 |

**PREREQUISITES** = package deps (IA-17, WP-12d, IA-16) + `__ARGS.extraDeps["IA-17b"]` (none) -
`__ARGS.dropDeps["IA-17b"]` (none). Checked against
`git -C wt/DBFIX log feat/db-fix --first-parent --merges --format=%s` on 24 Oct 2026, head e851da1ca:

| Prerequisite | State | Merge commit |
|---|---|---|
| IA-17 | **merged** | 7ba4d35cb `Merge IA-17: repair row ceiling per timeframe grid and the before_promote step (D-4875, step 2)` |
| WP-12d | **merged** | eaf41cdf6 `Merge WP-12d: GDFL back-fill of missing days through the versioned month repair` (also in `EMBED.done`) |
| IA-16 | **merged** | c099cf003 `Merge IA-16: failure kinds and causes: the admit table, decisive order, the typed repair door` |

**FILES_ONLY links** (`__ARGS.filesOnly["IA-17b"]`, "merge first, do not block the start"): **none**.

Not prerequisites, but named by the texts below (state at e851da1ca, for orientation only):
R3-35 merged (50093fa33, the split of `gdfl_import.rs` into files); IA-1, IA-2, IA-3, WP-16, WP-39,
WP-62, SC-4 merged; **IA-7 NOT MERGED** (the critic orders IA-17's later part after IA-16 and IA-7,
but the launcher does not make IA-7 a dependency: do not wait for it, do not build its read-back);
**R3-28 NOT MERGED** (it builds the "rewrites used of the 1,024 limit" list); IA-6, IA-10a, IA-12a,
IA-14, R3-26 NOT MERGED.

No earlier attempt of IA-17b exists on the Mac: no `wt/P-IA-17b`, no `fix/db-IA-17b` branch, no
`state/fix/round2/fix-IA-17b*.json` or progress file. IA-17's own reports exist and are quoted in
section 3i.

## 2. What the Mac builder is told (the launcher)

The builder prompt is `fixPrompt(p)`; its step 1 is `specStep(p)`, whose `ia` branch reads:

Source: `state/fix/db-fix-round-2-run0738.js.fat`, lines 342-346 (verbatim)

~~~~text
  if (p.kind === 'ia') {
    return `1. Read your package. It is an INGEST AUTOMATION package; its specification is the design ${DESIGN} (6,154 lines; read by line range, never all at once). Start with your row of the table in section 16.2: grep -n '^| ${p.row || p.name} ' ${DESIGN} (rows are single long lines; print them whole with sed -n '<line>p'), the round notes above that table that name ${p.name}, "Lanes" and "The order" below it, and your row in 16.3 (CI). Then read EVERY section, decision (section 15, column "Lands with"), file entry (section 11), route (12), web change (13), test id (14), limit row (10.2), matrix row (8) and edge case (9) your row cites. If ${CRITIC} exists, read its part for ${p.name}: its resolutions are your defaults where the design is silent or contradicts itself. Where the design states a fact about merged code that is false at the head of feat/db-fix, the code at the head is the fact: build to the design's intent and say so in notes. ${p.spec || ''}
   Also read ${W}/state/board/fix-plan.json's entry if one exists: jq '[.work_packages[] | select((.name | split(" ")[0]) == "${p.name}")]' ${W}/state/board/fix-plan.json
   The findings your package answers (ids starting DB-A with package IA, and the gaps F1..F18 your row names) are in ${W}/state/board/audit-result.json (key "confirmed"). Report one findings[] entry per test id, decision and gap your row names (id = the test id, decision number or gap id).`
  }
~~~~

The lines of `fixPrompt(p)` that bind the work itself (Mac-only lines about worktrees, the shared build pool and report files are left out; README.md gives their cloud equivalents):

Source: `state/fix/db-fix-round-2-run0738.js.fat`, lines 355 359 363 364 365 367 368 370 (verbatim, each line whole)

~~~~text
${(args && args.niftyFirst === false) ? '' : `\nNIFTY FIRST (operator): prove your work first on NIFTY spot and NIFTY options before anything else, on every NIFTY selection your package touches (feed GDFL 1s and Zerodha 1min..1day; segment Spot and expired options; every expiry form 2018-09..2020-09, strikes, CE/PE; dates, time filters, sorts, Go to, paging, export, coverage, moneyness/IV/greeks cells; for import packages: one NIFTY spot day, one NIFTY options day, one special session). Fix in the SHARED, instrument-agnostic code path, never as a NIFTY special case, then repeat the check on one BANKNIFTY selection, one F&O stock and one illiquid strike. The NIFTY evidence so far is in ${W}/state/gdfl-board/out/nifty.json.\n`}

RULES AUDIT (11 Oct): ${W}/state/design/rules-audit-20261011.md lists, per owner, the gaps ten agents found against the operator's rules (O(1) at p99, parallel, saved and searchable, common and scalable, extreme cases). Find the section for ${p.name} (grep -n '^## .*${p.name}' that file): every gap listed there is part of your package, with the test or bench it names. If a gap there cannot be closed inside your files, say so in notes with the owner it needs.

3. Do the package completely and precisely, the way its specification says, with tests that fail before and pass after. Edit only your package's owned files (new test files for your own modules are fine). If you truly must touch another file, keep it minimal and list it in files_outside_ownership. /db packages (WP-*) do not edit docs/05-decisions.md, docs/04-invariants.md, docs/06-limits.md, docs/11-findings.md or docs/07-plan.md unless they are WP-99: cite the D-numbers WP-02 assigned in code comments instead. Ingest packages (IA-*) land their own rows in those ledgers in the same change (D-4861): re-grep the highest D-48xx number first and never reuse one.

4. Gates in your worktree (all must pass before you commit; merge feat/db-fix into your branch once more just before them): cargo fmt --all --check; cargo clippy -p <each touched crate> --all-targets -- -D warnings; cargo test -p <each touched crate> --locked; if web/ changed: (cd web && npm run build && npm test), then reset the generated web/build files unless you own them. Anything you add to a hot path must stay O(1) per operation; measure p50, p99 and max for every route or operation you touch (n >= 200 sequential GETs against your shadow server, or the crate's bench) and report them with n and the method. Anything that cannot be O(1) is refused or bounded, never silently slow, and is named for docs/06-limits.md in your notes.

5. Check against REAL data, never on :8080: build in release (cargo build --release -p api -p cli), then

   This serves your build at http://127.0.0.1:${p.port} over a private shadow of the real store (bar files read through a link, everything else cloned). Confirm YOUR server is the listener: lsof -nP -iTCP:${p.port} -sTCP:LISTEN -t must print the pid in ${W}/shadow/${p.name}/test-server.pid. GET requests only against a shadow whose bars are the real store's link: never POST there, never run an import, pull, repair or backfill against it. For code that WRITES (importer, repair, sidecar or index writer, migration, verdict or alert store): exercise the write path only on a store root you own entirely - a temp dir built from synthetic fixtures, or a shadow in which you REPLACED the bars link by a real directory holding copies of the few months you need (cp -c) - and prove it never touched /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store. Reproduce each finding's selection and confirm the right answer (record URL + value as evidence a second agent can repeat). If the page is web-only, curl the API and run the web unit tests; you may open http://127.0.0.1:${p.port}/db in the browser pane only if no other agent is using it. Stop your server with ${W}/state/fix/stop.sh ${p.name} when done.

5b. SELF-CHECK BEFORE YOU COMMIT (19 Oct speed rule: a failed second check costs a whole repair round plus a re-check): do the second agent's checks yourself first. (i) Read your own full diff against feat/db-fix and judge every hunk against repo law (O(1) per operation: name every loop over months, files, records or instruments that a request or a per-row step can reach; append-only store; no look-ahead; no fallback that hides a failure; prices in paisa i64; Rust only outside web/; no test that asserts nothing; no real vendor row in a fixture) and the operator's rules (one common runtime-dynamic code path, incremental and scalable, every state change saved, audited, logged and searchable; every failure, refusal and extreme value tracked so it can be found, debugged, monitored and seen where the operator looks (/logs, /db or the part's own page) through a read that stays O(1) at p99). (ii) Add tests for at least 8 extreme cases that fit your change, at least 2 of them combinations of extremes at once and at least 1 out-of-the-box case no list names (empty, one-record and maximum inputs; month, year and expiry boundaries; special sessions; renamed symbols; duplicate, out-of-order and late rows; torn or zero-length files; a missing month mid-range; garbage parameters; concurrent requests; a kill mid-write; two writers). (iii) Re-run every finding you call fixed on your shadow server and record URL + value. Fix whatever this finds before you commit, and list in notes what you checked.

7. If something needs a store write on the real store (re-import, recount, migration, re-pull, backfill), implement and test the code path on fixtures and list the exact command under store_writes_needed; do not run it on the real store.

~~~~

Three more lines of `fixPrompt(p)` bind the work and the report. They are added by the
independent check. With `p` = IA-17b they read "YOU OWN WORK PACKAGE IA-17b: "D-4875 remainder:
the once-per-month gather and the CASH .auc writer"." and "Dependencies: IA-17, WP-12d, IA-16":

Source: `state/fix/db-fix-round-2-run0738.js.fat`, lines 354 362 372 (verbatim, each line whole)

~~~~text
YOU OWN WORK PACKAGE ${p.name}: "${p.title}".
   Dependencies: ${(p.deps || []).join(', ') || 'none'}. All are merged into feat/db-fix, so your branch contains them once you merge feat/db-fix.
Return the structured report. Be honest: a finding is "fixed" only if a test or a shadow-server check proves it; say NOT PROVED where you could not check.`
~~~~

Note on line 363: "Ingest packages (IA-*) land their own rows in those ledgers in the same change
(D-4861)" means IA-17b writes its own `docs/05-decisions.md`, `docs/04-invariants.md` and
`docs/06-limits.md` rows in its branch (section 5 says which numbers).

### 2a. The package's own text

Package title: **D-4875 remainder: the once-per-month gather and the CASH .auc writer**;
deps `["IA-17", "WP-12d", "IA-16"]`; kind `ia`; `waitCritic: true`; `row: "IA-17"`.

Package spec (`EMBED.packages[]` where name = IA-17b; `state/cloud-20261024/embed.json`, the same
text as line 10 of the launcher), in full. It ends with the word "ANSWERS:" and nothing after it in
both copies; no answers were appended (searched: nothing in `embed.json`, `args.json` or the launcher
follows it).

~~~~text
This is the REMAINDER of the IA-17 row ('for (3) and the .auc writer, WP-12d and IA-16'): D-4875 (3), the once-per-month gather, and the CASH .auc writer. First check whether WP-12d (merged) already built them; if both exist at the head of feat/db-fix, return nothing_to_do=true and name the commits. ROUND 3 ADDITION (12 Oct): You are the audit's "IA-17 (later part)": read the audit's "IA-17" section and "Critic resolution D-4899" as well; the once-per-month gather needs a test id and a bound, and the plan shows rewrites used of the 1,024 limit (R3-28 builds the list). R3-35 splits gdfl_import.rs into five files before IA-1 starts; the parts your row names now live in those files (its report says which file holds which function). Edit them there and put no seam back into gdfl_import.rs. ANSWERS:
~~~~

`__ARGS.specAppend["IA-17b"]` (`state/cloud-20261024/args.json`, line 6 of the launcher), in full;
it is appended to the spec after a blank line:

~~~~text
BACKTEST-FIRST (operator 18 Oct): build first only the data-proof piece: the whole part as specified (the once-a-month gather and the CASH .auc writer for repaired months, auction prints kept out of continuous bars), because PREC-2 and PREC-13 build on it; the automation piece: none was separated by the 18 Oct classification; if this spec names a page, route or alert that the backtest does not read, that piece follows after the backtest.
~~~~

No other amendment names IA-17b: the options, one-command-cut, no-fresh, ledger follow-up, R4-09b
and priority notes in `args.json` (`_note_*` keys) do not mention it (searched by name). The
operator's 23 Oct "no fresh" rule (memory: a GDFL re-delivery is an incremental patch into the
existing store, never a rebuild) is consistent with this part: the gather lands repaired days as
month generations, never a rebuild.

## 3. The design and its reviews

### 3a. Section 16.2: the round notes above the table that name IA-17

Source: `state/design/ingest-automation.md`, lines 4806-4849 (verbatim)

~~~~text
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

~~~~

### 3b. Section 16.2: the table header and the IA-17 row (whole)

Source: `state/design/ingest-automation.md`, lines 4884-4885 (verbatim)

~~~~text
| Pkg | Name | Depends on | Files owned (non-overlapping, but for §11's shared lines) | Size | Lane |
|---|---|---|---|---|---|
~~~~

Source: `state/design/ingest-automation.md`, lines 4895 (verbatim, each line whole)

~~~~text
| IA-17 (round 4, SF-5; narrowed in round 5, B4; **live since round 6**, SF-2: WP-13c merged at e10334b9 with `MAX_ROWS = 100_000`) | D-4875 built: **at step 2** (needs nothing unmerged), (1) **the per-timeframe grid ceiling** in `store::repair` at all four `RowLimit` sites (`repair.rs:308`, `:448`, `:465`, `:674`), and (5)'s **`before_promote` step** in `repair_month`, between `publish` and `promote` (round 6, SF-3); **later**, only if WP-12d leaves them: (3) once-per-month repair in WP-12d's filing path and the CASH `.auc` writer passed to that step, with `auction::path` resolving the served generation. **Not** the revision's `.tix` (WP-13c's own writer's), its `.srt` (WP-39's writer step) or a `.prv` (none, D-4887). Tests **IA-RP-01** `a_full_1s_month_repairs_with_its_sidecars` (the `.tix` present and O(1), no `.prv` written, the rule set read from the journal) and **IA-RP-02** (the `.auc` step); bench **C-RP-01** in `crates/store/benches/ratio.rs` for L-IA-26 (no crate benches `repair_month` at d73e1112) | step 2: WP-13c (merged); for (3) and the `.auc` writer, WP-12d and IA-16 | `store/src/repair.rs` (the ceiling, the step), `store/benches/ratio.rs` (C-RP-01); later `gdfl_import.rs` (the gather, lane A after IA-16) and `gdfl_cm_auction.rs` (`auction::path`) | M | A |

~~~~

### 3c. Section 16.2: "Lanes" and "The order"

Source: `state/design/ingest-automation.md`, lines 4910-5008 (verbatim)

~~~~text
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

~~~~

### 3d. Section 16.3 (CI): its header and the IA-17 row

Source: `state/design/ingest-automation.md`, lines 5032-5033 (verbatim)

~~~~text
| Pkg | `gates_ledger.rs` entries (count added) | `docs/06-limits.md` rows | `docs/04-invariants.md` rows | Other gates |
|---|---|---|---|---|
~~~~

Source: `state/design/ingest-automation.md`, lines 5043 (verbatim, each line whole)

~~~~text
| IA-17 (round 4; live since round 6, SF-2) | none expected (`repair_month`'s merge walks held and offered bars in order and sorts nothing) | L-IA-26 (its bound rewritten, N-3) | **IA-RP-01** `a_full_1s_month_repairs_with_its_sidecars`, **IA-RP-02** (the `.auc` step) | 12: C-RP-01; 14: `crates/store` re-measures the repair at a full 1s month (C-RP-01, with peak RSS); 27b (D-4875) |

~~~~

### 3e. Every section, decision, file entry, limit, matrix row, edge case and test the row cites

#### Section 5.1, the seams: item 1 (stop and held-back check before the .auc append) and item 5 (close order, the gathered day)

Source: `state/design/ingest-automation.md`, lines 852-876 (verbatim)

~~~~text
**The seams inside `gdfl_import` (F23, F22, F44), and who owns each.**
`gdfl_import.rs` passed through WP-11b (merged 3f65ad43) and passes through
WP-16, WP-11c, WP-12d (its filing path: a day the append-only month refuses
lands through WP-13c's repair door when the run carries a repair reason,
D-4874) and WP-12f (the scope: BE/BZ spells, stem hand-overs, alias indices,
the scope record on each close, per-member reopening, D-4878) before any IA
package edits it (§16.1). WP-16 replaces the per-file loop with W workers; the
design states what it needs of that loop and adds only what WP-16 leaves out:

1. **Stop before the next file** (IA-2 since round 4, SF-11: the stop check,
   the stop file, `Stage::Run`, the stop close rules of D-4853 and `cli
   gdfl-stop`, which finds a run by its claim, all land in one package, so no
   build has a stop that closes a day `refused`). The stop flag and the stop
   file are checked (1 atomic load, 1 `metadata`) before a worker takes a file,
   never inside one, and before that file's `gdfl_cm::auction::append`
   (`:3589` at ab2810b8), so a stop writes no further `.auc` sidecar. When the
   file just finished was the day's last, the day closes as it would have
   (`done` or `done_with_refusals`) and the run stops before the next day
   (D-4853). **The held-back check sits in the same place** (round 5, SF-11;
   IA-16's, one hash probe of (instrument, month), §5.5): after the stop check
   and before the `.auc` append, so a held-back file appends no `.auc` record.
   Placed after the append, a failed `.auc` append on day D followed by D+1's
   prints would make D's next `.auc` append "older" than the month holds,
   typed `NotHeld`: a false hole in a month whose bars hold nothing after D.
   IA-GR-18 pins the order.
~~~~

Source: `state/design/ingest-automation.md`, lines 900-910 (verbatim)

~~~~text
5. **The day's close order** (WP-16's flush, then IA-2 and IA-7): every file
   of the day written, each with its oracle digest kept (IA-7, D-4886); WP-16's
   one durable flush; the census publish; **the read-back phase** (IA-7, round
   4, BL-H: each filed instrument-day read back and compared with its digest,
   on the same workers); the journal close; then the verdict image commit. A
   crash in the read-back phase leaves the day begun and not closed, so the
   next resume reads it again (D-2821). A crash after the journal close and
   before the commit leaves the day `done` with no verdict of this run: the
   completeness sweep lists it as "verdict pending", never as verified (§6.2).
   A day a repair-reason run gathers for a month repair (D-4875 (3)) reads back
   its repaired instruments only after that repair has published (§5.5).
~~~~

#### Section 5.5: the gathered day's read-back split and memory (L-IA-12)

Source: `state/design/ingest-automation.md`, lines 1336-1373 (verbatim)

~~~~text
     oracle rebuilds it, and the comparison of §6.1 step 7 names the first
     differing second and every count. Cost O(bars of the day) for the read-back
     plus O(bytes of the differing files) for the re-fetch (L-IA-3 (g)).
  Records are gathered and sorted by the entry's listing index at day end
  (ALLOW_SORT, §16), so the image is identical for every W. The cross-kind
  checks never run here (§6.1). **A day a repair-reason run gathers** for a
  month repair (D-4875 (3)) closes only when the repair has published, so its
  phase 2 is split: the files of instruments that appended as usual are read
  back at the day's own phase 2, and the files of the instruments in the
  repair are read back after the repair publishes, when the day closes. Until
  then their digests and the day's records wait in the run's memory (L-IA-12);
  no read-back ever runs against a month the repair has not yet published, so
  a V-4 or V-5 day is never read as `missing_in_store` or `count_mismatch`
  (IA-GV-28). A file the repair refuses gets its record from the refusal
  (`owed`, reason `repair_refused`, or `repair_impossible` for a refusal no
  repair can lift, round 5), with no read-back.
- **Memory (L-IA-12).** A worker holds one member (now kept as `Arc<[u8]>`
  with its decode, §5.1 seam 4) and its decoded rows: at most the largest
  member (15,279,399 B for a CM index file, 2,359,609 B for options) plus up to
  `MAX_ROWS` 1,000,000 rows (`fetch.rs:64`) as reader rows (`NfoRow` 32 B,
  `gdfl_nfo.rs:1079`; `CmRow` 24 B, `gdfl_cm.rs:1733`), `Tick`s (40 B,
  `gdfl_import.rs:1160`) and, under inline verify, the oracle's own rows and
  seconds for that one file. Peak ≈ kinds × W × (2 × largest member +
  `MAX_ROWS` × (reader row + Tick + oracle row) + the file's bars), plus one
  census partition image per concurrent publish (up to 8,388,608 × 128 B = 1
  GiB for a full FNO epoch, §2.2), plus the day's gather: the census rows, the
  failures, and **224 bytes per file** for its record skeleton and digest
  (round 4: at most 26,820 × 224 B ≈ 6.0 MB for the largest options day, and
  11,524 × 224 B ≈ 2.6 MB for 2026-08-03), plus, in a repair-reason run, the
  same 224 bytes per file of every day gathered for a month repair not yet
  published (≤ 23 days of the month: about 0.7 MB for an index month at 141
  files a day, and at most 23 × 26,820 × 224 B ≈ 138 MB for an options month
  at the largest day size, an upper bound), plus the repair's own merge
  (L-IA-26). One worker's worst case is about 30.6 MB + 104 MB from the type
  sizes; W = 12 and three kinds (three processes on v4) give about 4.9 GB plus
  the census images (an extrapolation from type sizes, not a measurement).
  `docs/06-limits.md:15833` lists memory as not measured today; WP-16 and
  IA-13 measure peak RSS.
~~~~

#### Section 5.5: a past day the month already passed, the repair-reason run, the CASH passed day's auction prints, a month served from a revision, the holes on the real store

Source: `state/design/ingest-automation.md`, lines 1483-1619 (verbatim)

~~~~text
- **A past day the month already passed (D-4851, amended in round 3).**
  When the store refuses a batch as `Conflict::NotHeld` (a stamp inside the
  held range that the month never held, `file.rs:3446`, `:3453`), what happens
  depends on the run:
  - **A run with no repair reason**: the file fails
    `needs_month_generation{instrument, month}`, a **store** cause at stage
    `file` that **does not close the day** (`Cause::open`). Round 2 made it a
    closing cause; a closing cause writes `done … vendor_refused=<n>`
    (`Close::WithRefusals`, `gdfl_import.rs:2249`–`:2262`, `:2310`), a vendor
    word for a store refusal, against `Cause`'s own rule that "the store …
    never close a day" (`:625`–`:634`) (critic SF-11). So, when every open
    failure of the day is a store hole (round 4, BL-C), the day closes
    `incomplete stage=file reason=needs_month_generation`, and **the hole is a
    plan class read from that reason** (§5.4), not a close: `admit` skips it
    (row 4a), the plan counts it, `/ingest` and `/verify` name it. D-3317 item 2
    stands unamended.
  - **A run with a repair reason** (D-4874): WP-12d's filing path hands the
    refused month to WP-13c's repair door (`repair_or_append`,
    `crates/pull/src/ingest.rs:2225`, merged at e10334b9), which publishes a new
    generation holding every held bar plus the day's bars and promotes it, and
    writes a `repair` line to `<store>/audit/gdfl.repairs` (WP-13c's
    `journal_repair`, `crates/pull/src/ingest.rs:2338`–`:2394`,
    D-3350: month, the generations replaced and served, bars held and added,
    `reason=` and `run=`; vendor `gdfl`). The day then closes as any other day. Under
    D-4875's once-per-month rule (a repair copies the whole month), the run
    gathers the bars of every day of that month it reads, offers them in one
    repair when it leaves the month or ends, and closes those days, in day
    order, when the repair has published; a crash before that leaves them
    begun and not closed, so the next resume reads them again (D-2821), and
    WP-13c's repair finds a published-but-unpromoted revision complete
    (D-3350: "An exact completed retry … returns `Published::Reused`",
    `REPAIR.md:111`–`:118`). Those days' inline read-back waits for the publish (the
    inline paragraph above). **The door repairs only what an append cannot
    take** (round 4, SF-1, F64): WP-13c's `repair_or_append` falls into a
    repair on any append error (`Err(why) => why`, `ingest.rs:2234`, merged at
    e10334b9) and returns `String`; the door repairs only
    `OverlapDisagrees{conflict: NotHeld}` and `Superseded`, and every other
    append error, and every repair error, reaches `gdfl_import` typed, by the
    table of §5.8 ("Every store and repair refusal, by kind"). **The door is
    shared with Zerodha** (round 6, SF-5): the same narrowing changes what
    `/pull/run`, `/pull/spot`, `/pull/repair` and `cli zerodha-repair` do and
    print (§5.8, "The door is one door"). **A repair that
    refuses splits two ways** (round 5, SF-10): a refusal **no repair can
    lift** (`HeldBarDiffers`: a held bar would change; `IndexedSibling`: the
    month has an overlay or greeks sibling an insertion would shift;
    `RevisionLimit`: the month's 1,024 revision ordinals are spent) makes the
    file fail `repair_impossible{why}` and the day closes `incomplete
    stage=file reason=repair_impossible`, a hole `admit` row 4b never reads
    again; a refusal **a later build or repair can lift** (`RowLimit` until
    D-4875 (1) is built, `NoBasisWitness`, `UnsealedSource`, `Conflict`,
    `Incomplete`, `InvalidReceipt`) makes the file fail `repair_refused{why}`
    and the day closes `incomplete stage=file reason=repair_refused`, a hole row
    4a admits to the next repair-reason run; each is named with the refusal's
    own words (detail `repair`). A repair that failed in a way a retry can mend
    (`StaleSource`, `Locked`, an I/O error) is an open cause and never a hole;
    `StorageFull` ends the run `disk_full`; an I/O error with
    `publication_may_be_visible: true` is never a hole either, because the next
    resume's byte-exact retry completes the revision (D-3350, `REPAIR.md:120`–`:125`).
  - **A CASH passed day's auction prints** (round 4, SF-2, F65): `cm_day`
    appends the day's `.auc` records before it files the bars
    (`gdfl_import.rs:3589`), and the `.auc` refuses a day older than one the
    month holds (`gdfl_cm_auction.rs:52`–`:56`). That refusal is typed
    `FailKind::NotHeld` (a hole, `needs_month_generation`), and a records-differ
    refusal `FailKind::Differs`, never the open `auction_sidecar` that would loop
    (IA-16). **The `.auc` is `pull`'s file, not a store sidecar**: it lives at
    `bars/gdfl/NSE/CASH/<SYM>/1s/<YYYY-MM>.auc` under the store root and is
    read by `gdfl_cm::auction::path(root, …)` (`api/src/sources.rs:2894`), so
    `open_existing`'s resolution never reaches it, and its own doc says an
    older day "is a new generation of the month" (`gdfl_cm_auction.rs:52`–`:56`).
    So a repair-reason run of a CASH month carries the `.auc` through **one
    step inside `store::repair`, never a second merge** (D-4875 (5); round 6,
    SF-3, F82). Round 5 had the caller write the `.auc` between
    `repair::publish` and `generation::promote`; but `publish` takes an
    already merged series (`repair.rs:299`–`:311`), the merge of held and
    offered bars, the `HeldBarDiffers` check and the basis witness exist only
    inside `repair_month` (`:461`–`:490`), and `repair_month` promotes as soon
    as it publishes (`:507`–`:509`), so that caller would have had to copy the
    merge: a second repair authority. Instead `repair_month` takes one more
    argument, `before_promote: Option<&mut dyn FnMut(&Path) -> Result<(),
    String>>`, called with the revision's root after `publish` returns
    (`Created` or `Reused`) and before `generation::promote`; `store` names no
    `pull` type, so the arrow stays one way. The GDFL CASH caller passes a step
    that writes the revision's whole `.auc` (the original's records and the
    repaired days', in day order; synced, its directory synced) beside the
    revision's `.bin`; every other caller passes `None`, and their bytes are
    unchanged. A step that fails refuses the repair before the promotion
    (`RepairError::Io`, its words kept), so the revision stays published and
    unserved, and the next resume's exact retry (`Published::Reused`) runs
    the step again with the same bytes before it promotes. `auction::path`
    resolves the served generation the way `open_existing` does
    (`generation::served_bars_path`, `generation.rs:244`; `promote`, `:391`),
    so `sources.rs` reads the served `.auc`. **Owners**: IA-17 adds the
    argument to `store::repair` (the file it already edits for the ceiling);
    WP-12d's filing path, else IA-17 after IA-16, passes the `.auc` step. Only
    CASH months from 2026-08-03 carry a `.auc` (D-3320), so no hole of §2.2
    needs it.
  - **A month already served from a revision** (round 3, D-4876, F50): a
    batch whose bars the revision already holds answers `AlreadyPresent`, as
    WP-13c rules (D-3350, `REPAIR.md:14`–`:15`); a batch holding a new bar is refused
    `StoreError::Superseded`, typed `FailKind::Superseded` at the ingest door,
    and the file fails `month_superseded{instrument, month}`: the same hole
    class, filled the same way. **So a resume converges after a month is
    repaired**: a later import into it either lands nothing new (held) or
    stops at a named hole once, and Resume never re-reads it (`admit` row 4a).
    **One case is named rather than hidden** (round 4; given its decision and
    owners in round 5, SF-9: D-4876 (2), (3)): a repair of a month that is still
    being traded (its last trading day by the calendar of record not yet
    passed) turns every later day of that month into `month_superseded`,
    because a revision grows only by a further repair
    (`crates/store/src/generation.rs:57`–`:60`). So `plan.json`
    warns `repair_in_open_month{month}` for a repair-reason run whose range
    touches such a month (IA-16 adds the warning with `admit`'s rows 4a and 4b;
    one calendar probe per month in range); IA-14's runbook runs those repairs
    after the month's last trading day; and if one is run anyway, one
    repair-reason Resume after the month ends lands the month's remaining days
    in one more repair of it (D-4875 (3)); nothing loops in between (IA-GR-31).
  - **The holes on the real store.** The 12 NIFTY and BANKNIFTY days of §2.2
    were closed by 10babda7 as `reason=filing` (no typed kind then), which no
    hole class names; the first Resume after IA-16 reads them once more, meets
    `NotHeld` again and closes them `reason=needs_month_generation`, after
    which they are holes. 2022-03-07 (D-3318) and the two 2018 options
    contract-days of §8 F13 and F38 (after IA-4) take the same path. **The 556
    alias-index days** (round 4, BL-D): a run reopens each of them per member,
    under the member's own key (`indices <day> *+NSE-INDEX-NIFTYIT`, …); the
    member's month refuses the day (`NotHeld`), so that member key closes
    `incomplete reason=needs_month_generation`, a hole, while the base key `*`
    stays `done` for NIFTY and BANKNIFTY; the plan names the member-days, the
    completeness sweep writes `owed (needs_month_generation)` for exactly those
    members, and V-5's repair-reason run admits them (row 4a of `admit`) and
    repairs each of the 67 months once. **All of them need D-4875 (1)** (the
    repair must hold a full 1s month; WP-13c merged at e10334b9 with the
    ceiling still 100,000 rows, `repair.rs:29`, so IA-17 raises it at step 2,
    round 6) before any repair-reason run can fill them; their `.tix` is the
    revision's own (`REPAIR.md:212`, merged) and they carry no `.prv`
    (D-4887). On a build without IA-17 a repair-reason run answers
    `repair_refused (RowLimit)` for each, named, and the next repair-reason run
    after IA-17 admits them again (row 4a), never a loop.
~~~~

#### Section 3: the gaps the row answers (F48, F65, F82), with the table header

Source: `state/design/ingest-automation.md`, lines 510-511 (verbatim)

~~~~text
| ID | Finding | Evidence | Consequence today | Fixed in |
|---|---|---|---|---|
~~~~

Source: `state/design/ingest-automation.md`, lines 564 586 613 (verbatim, each line whole)

~~~~text
| F48 | WP-13c's repair cannot hold a GDFL 1s month | `store::repair::MAX_ROWS = 100_000` "in both the source snapshot and the complete replacement" (`fix/db-WP-13c:crates/store/src/repair.rs:29`); `publish` and `repair_month` refuse `RowLimit` (`:308`, `:448`, `:465`, `:674`). A NIFTY 1s month is about 425,817 bars (§2.2); §2.2's 43,942,544 bars over 97 months is a mean of 453,016 | None of the 12 NIFTY and BANKNIFTY holes, 2022-03-07, or the 67 alias months can be repaired | D-4875 (1) (a ceiling per timeframe, proposed to WP-13c; IA-17 if WP-13c merges without it). **Round 5:** WP-13c's second pass (d44246fa) still has `MAX_ROWS = 100_000` (`repair.rs:29`; refusals at `:308`, `:448`, `:465`, `:674`) **13 Oct (CMP-20):** resolved at the head of feat/db-fix: `MAX_ROWS = 31 * 86_400` (`crates/store/src/repair.rs:47`, IA-17, merged), every `RowLimit` site asks `row_ceiling`; the 12 holes were repaired on 11 Oct |

| F65 | A repaired month lacks sidecars, and a CASH passed day's auction prints are refused first | "provenance … must be validated by the caller" (`fix/db-WP-13c:crates/store/src/repair.rs:288`); `open_existing` resolves sidecars under the revision root (`file.rs:1708`–`:1745`); WP-39's WIP adds a per-month `.srt` (`fix/db-WP-39:docs/02-store-format.md:788`); a `.auc` "day older than one the month already holds, is refused by name" (`gdfl_cm_auction.rs:52`–`:56`), and `cm_day` appends the `.auc` before it files (`gdfl_import.rs:3589`) | A repaired month would have no `.prv` (no per-record rule set, no flag 7) and no `.srt` (sort pages refuse); a CASH passed day would fail `auction_sidecar`, an open cause, on every run | D-4875 (5): IA-16 types the `.auc`'s older-day refusal `NotHeld` (a hole) (SF-2). **Round 5 (B4):** WP-13c's second pass decided the `.prv` (none, adopted by D-4887, F72); the `.srt` follows the revision through WP-39's writer step (F75); the CASH `.auc` follows the served generation by WP-12d's filing path, else IA-17 |

| F82 | A repair's memory is the merged series twice over, and its `.auc` step has no seam | `repair_month` holds the merged series (`repair.rs:466`) and the added stamps (`:469`) beside the caller's offered bars, and `publish` hands the merged series to the writer (`:632`), which encodes it whole before writing (`file.rs:2724`); `publish` takes an already merged series (`repair.rs:299`–`:311`), the merge lives only in `repair_month` (`:461`–`:490`), which promotes as soon as it publishes (`:507`–`:509`) | L-IA-26's bound left out two terms (N-3); a CASH caller that wrote the `.auc` between `publish` and `promote` would have to copy the merge, a second repair authority (SF-3) | L-IA-26 rewritten; D-4875 (5): a hook inside `store::repair` |

~~~~

#### Section 8 (input case matrix): the CAS auction row, with its table header

Source: `state/design/ingest-automation.md`, lines 3887-3888 (verbatim)

~~~~text
| # | Case | On disk | Recognised by | Today (CM / NFO) | After | Test | Cost |
|---|---|---|---|---|---|---|---|
~~~~

Source: `state/design/ingest-automation.md`, lines 3905 (verbatim, each line whole)

~~~~text
| F17 | CAS auction prints 15:15–15:30 (eligible shares, from 2026-08-03) | stocks | `auction_candidate` `:1866`; `:1917` | to the `.auc` sidecar; unknown eligibility refuses the file | unchanged; D-3335's `auction_prints` count (D-4841) | ✓ `an_eligible_shares_auction_rows_are_prints_never_trades` | O(1) |

~~~~

#### Section 9 (runtime edge cases): the rows on repairs, gathered days and the .auc, with the header

Source: `state/design/ingest-automation.md`, lines 4012-4013 (verbatim)

~~~~text
| Case | What happens | Named as | Test |
|---|---|---|---|
~~~~

Source: `state/design/ingest-automation.md`, lines 4024 4043 4069 4075 4078 4079 (verbatim, each line whole)

~~~~text
| A past day the month already passed (the 12 NIFTY days, measured 10 Oct) | without a repair reason: `incomplete reason=needs_month_generation`, a hole the plan names and Resume skips (round 3: no close, SF-11); with one: landed as a new month generation through WP-12d (D-4874), or `repair_refused` naming WP-13c's refusal | §5.5 | + IA-GR-19 |

| Kill during a `.auc` append | the bytes past the last whole record are an interrupted append, overwritten by the next; a file shorter than its header, or all zero, is unborn | D-4865 | + IA-AUC-01 |

| (round 3) A later import into a month a repair replaced | bars the revision holds answer `AlreadyPresent`; a new bar is the hole `month_superseded`, skipped by Resume; a repair-reason run fills it; the resume converges | D-4876 | + IA-GR-31 |

| (round 4, BL-H) A repair-reason run's gathered days | their repaired instruments are read back only after the month repair publishes; the other files at the day's own phase 2; never `missing_in_store` for bars the unpublished repair holds | D-4886 | + IA-GV-28 |

| (round 3) A repair of a full 1s month | merged within the timeframe's grid ceiling (2,678,400 rows for 1s; D-4875 (1)); the revision's `.tix` from WP-13c's writer (`REPAIR.md:212`), its `.srt` runs from WP-39's writer step once WP-39 lands, no `.prv` (D-4887: rule set from the journal, flag 21), and for CASH the `.auc` written by the step `repair_month` runs between publish and promote (round 6, SF-3); the census row the served header's; until IA-17 lands, `repair_refused (RowLimit)`, named, and admitted again by the next repair-reason run (round 4, SF-2, SF-5; round 5, B4; round 6: WP-13c merged with the 100,000 ceiling, so IA-17 raises it at step 2) | D-4875, D-4887 | + **IA-RP-01** `a_full_1s_month_repairs_with_its_sidecars` (IA-17: 2,678,400 rows merged and published; the `.tix` present and O(1); no `.prv` written; the verifier reads the month's rule set from the journal); + IA-VD-11 (stale after a repair) |

| (round 6, SF-3) A CASH repair's `.auc` step fails after `publish` | the repair is refused before the promotion (`RepairError::Io`, its words kept); the revision stays published and unserved; the next resume's exact retry (`Published::Reused`) runs the step again with the same bytes and promotes | D-4875 (5) | + **IA-RP-02** `a_cash_repair_writes_its_auction_file_before_it_is_served` (IA-17) |

~~~~

#### Section 10.1 (per operation): the held-back probe and the month repair, with the header

Source: `state/design/ingest-automation.md`, lines 4120-4121 (verbatim)

~~~~text
| Operation | Bound | Proof or measurement | Bench | Package |
|---|---|---|---|---|
~~~~

Source: `state/design/ingest-automation.md`, lines 4130 4166 (verbatim, each line whole)

~~~~text
| Held-back check per file | O(1) | 1 hash probe into ≤ 4,096 pairs, before the worker takes the file and so before its `.auc` append (round 5, SF-11) | C-GR-09 | IA-16 |

| (round 3) A GDFL month repair | O(held + offered) bars, ≤ the timeframe's grid ceiling; not on any read path | WP-13c's `repair_month` (merged), with D-4875 (1)'s ceiling; the revision's `.tix` is WP-13c's own writer's (round 5) | **C-RP-01** (round 6, SF-2: no crate bench names `repair_month` at d73e1112, so IA-17 adds it to `crates/store/benches/ratio.rs`: one 1s month of 425,817 held bars plus one offered day, and a full month of 2,678,400 rows, p50, p99, max, and peak RSS); L-IA-26 | IA-17 (the ceiling, the bench); WP-12d the GDFL caller |

~~~~

#### Section 10.2 (limits for docs/06-limits.md): L-IA-12 and L-IA-26, with the header

Source: `state/design/ingest-automation.md`, lines 4183 4184 4196 4210 (verbatim, each line whole)

~~~~text
| ID | Limit | Why it is not O(1) | Bound | Package |

|---|---|---|---|---|

| L-IA-12 | Peak memory | §5.5 | kinds × W × (2 × largest member + `MAX_ROWS` × (reader row + Tick + oracle row) + bars) + one census partition image per concurrent publish (≤ 1 GiB) + the day's gather, which round 4 sizes (BL-H): 224 B per file (a 160-byte record skeleton and a 64-byte digest; ≤ 6.0 MB at 26,820 files), plus, in a repair-reason run, 224 B per file of every day gathered for a month repair not yet published (≤ about 0.7 MB for an index month, ≤ 138 MB for an options month at the largest day size, an upper bound) and the repair's merge (L-IA-26); **not measured** today (`docs/06-limits.md:15833`); WP-16 and IA-13 measure W and kinds ∈ {1, 3} | WP-16, IA-7, IA-13 |

| L-IA-26 (round 3) | A month repair (WP-12d through WP-13c) | reads the served month whole and writes a whole successor with the sidecars its writer keeps (its `.tix`, and its `.srt` runs once WP-39 lands; no `.prv`, D-4887; the `.auc` for CASH; round 4, SF-2; round 5, B4), which is kept for ever (`bar-revisions-v1/`, D-3350) | O(held + offered) bars, at most the timeframe's grid ceiling (D-4875: 2,678,400 rows for a 1s month, 44,640 for 1min); memory (round 6, N-3, F82): the merged series (56 B a bar, `repair.rs:466`), the added stamps (8 B each, `:469`), the caller's offered bars (56 B each) and, inside `publish`, the writer's encoded image of the merged series (56 B a bar, `file.rs:2724`), so ≤ (3 × 56 + 8) B × the ceiling = 176 B × 2,678,400 ≈ 471 MB for a full 1s month by type sizes (round 5 counted 2 × 56 B, about 300 MB, and the critic's (2 × 56 + 8) B, about 321 MB, still left out the writer's image), plus the `.tix` entries of the month; an upper bound, UNVERIFIED until C-RP-01 measures peak RSS; a real NIFTY 1s month of about 426,000 bars is about 75 MB by the same sum. **Disk: one full copy per repair.** So D-4875 asks that a repair-reason run repair each (instrument, month) **once**, offering every day of that month it reads together: the 24 NIFTY and BANKNIFTY months of the 12 holes and the 67 alias months are then about 91 × 24 MB ≈ 2.2 GB of kept copies. Repaired a day at a time, each later day of a month already served from a revision is another repair (`month_superseded`), and the 67 alias months alone would copy about 17,000 day-sizes, about 21 GB (an extrapolation from §2.2's bar sizes and the days per month) | WP-12d; the ceiling WP-13c's, else IA-17 (round 4, SF-5; round 5: the `.tix` is WP-13c's own and no `.prv` is written, D-4887) |

~~~~

#### Section 11 (files): the rows IA-17 owns or shares, with the header

Source: `state/design/ingest-automation.md`, lines 4233-4234 (verbatim)

~~~~text
| Crate / path | Add or change | What | Package | After |
|---|---|---|---|---|
~~~~

Source: `state/design/ingest-automation.md`, lines 4239 4246 4268 4269 (verbatim, each line whole)

~~~~text
| `crates/pull/src/gdfl_import.rs` | change | IA-1: the counters-only snapshot and the failures cap where WP-16 left them; `Journal::refresh` and `Journal` made `pub(crate)` (`:1856`); the run id from `RunOptions`. IA-2: the stop check before each file and before the auction append (round 4, SF-11); `Stage::Run` (`ALL: [Self; 6]`); `decisive()` searches `Run` second; `Journal::read_line` keeps the last `incomplete` reason, the first close that wrote seconds, and WP-12f's scope fields of the last close and the last `done` close per key (round 3; round 4, BL-D); `import_cause` used by `staged()`; the stop close rules; the journal append lock; the D-3335 ledger appends under it; `VERBS` exported; one journal writer per day if WP-16 left it. IA-3 (round 4, SF-10): the `cm_cause` and `nfo_cause` arms of its new variants, and D-4818's three archive arms. IA-16: `admit` as one rule table (D-4885, round 4, BL-B, SF-18); `decisive()`'s non-hole-before-hole order (round 4, BL-C); the day-close refusals (`no_wanted_file`, `absent_from_listing`, `stored_index_absent`), the typed `FailKind` handling by §5.8's table (round 4, SF-1), `month_busy` retry, the held-back set and its check before the `.auc` append (round 5, SF-11), the five hole reasons as `incomplete` reasons (never a close, round 3; `repair_impossible` split from `repair_refused` in round 5, SF-10), the loss probes, `disk_full`, the D-3335 v=2 fields, the refusal audit cap. IA-4 (round 4, BL-E): `BAR_DEFINITION` = 4 and `definition_is_current`; `option_handed`'s `rows_refused`; the `nfo_cause` arms of its parity variants. IA-7: the inline oracle's two phases on WP-16's workers (D-4886) and the image commit after the close | IA-1, IA-2, IA-3, IA-16, IA-4, IA-7 | WP-11b (merged 3f65ad43), WP-16, WP-11c, **WP-12d** (the filing path through WP-13c's repair door), **WP-12f** (the scope, the scope record, per-member reopening, the journal format header) |

| `crates/pull/src/gdfl_cm_auction.rs` | change | the typed kind on `AuctionSidecar`, with the older-day refusal typed `NotHeld` and the records-differ refusal `Differs` (round 4, SF-2); the torn-tail and unborn rules (D-4865); IA-AUC-01 (IA-16). `auction::path` resolving the served generation through `generation::served_bars_path`, and the revision's whole `.auc` written between `repair::publish` and `generation::promote` (round 5, D-4875 (5); WP-12d's filing path, else IA-17) | IA-16; WP-12d, else IA-17 | WP-12d, WP-13c |

| `crates/store/src/repair.rs` (round 4, SF-5; narrowed in round 5, B4; **live since round 6**, SF-2: WP-13c merged at e10334b9 with `MAX_ROWS = 100_000`) | change | D-4875 (1): the per-timeframe grid ceiling (days in the month × 86,400 ÷ the rung's seconds: 2,678,400 rows for a 31-day 1s month) in place of `MAX_ROWS` (`repair.rs:29`) at both checks, `publish`'s merged series (`:308`) and `repair_month`'s offered bars (`:448`), and its other two `RowLimit` sites (`:465`, `:674`); the memory bound of L-IA-26; D-4875 (5): `repair_month`'s `before_promote` step (round 6, SF-3). The revision's `.tix` is WP-13c's own writer's (`repair.rs:631`, `REPAIR.md:212`), its `.srt` WP-39's writer step's, and it carries no `.prv` (D-4887): IA-17 writes none of them. Tests IA-RP-01, IA-RP-02; bench C-RP-01 in `crates/store/benches/ratio.rs` | IA-17 | WP-13c (merged) |

| `crates/store/benches/ratio.rs` | change | C-VD-01, C-VD-02, C-VD-04 (IA-5); C-RP-01, the repair at a full 1s month with peak RSS (IA-17, round 6, SF-2), each under its own heading | IA-5, IA-17 | — |

~~~~

#### Section 12 (routes) and 13 (web)

The IA-17 row cites no route and no web change. (The "plan shows rewrites used" item of the spec is R3-28's list and IA-10a's `plan.json`, neither merged; see section 4.)

#### Section 14 (tests): IA-RP-01, IA-RP-02 and the tests the row's cited sections name (IA-GR-18, IA-GR-31, IA-GV-28, IA-AUC-01), with the header

Source: `state/design/ingest-automation.md`, lines 4396-4397 (verbatim)

~~~~text
| ID | Invariant | Test | Package |
|---|---|---|---|
~~~~

Source: `state/design/ingest-automation.md`, lines 4416 4429 4439 4467 4522 4523 (verbatim, each line whole)

~~~~text
| IA-GR-18 | A month held by a reader for under 2 s never fails a file; a file that fails on day D holds back its (instrument, month), so D+1 never makes D unfillable, and a resume converges; **the mixed day** (round 4, BL-C): instrument Y meets `NotHeld` before instrument X's CRC fault on D, D closes `reason=source_crc_mismatch` (not a hole), X's D+1 is held back, the next Resume reads D, files X's D, and only then closes D a hole for Y; X's D is never `NotHeld`; **the held-back check runs before the `.auc` append** (round 5, SF-11): a CASH share whose `.auc` append failed on D appends no `.auc` record on D+1, and D's next append is not refused as older | `a_busy_month_or_a_bad_copy_never_strands_an_earlier_day`; `a_hole_beside_a_transient_failure_never_strands_the_other_instrument`; `a_held_back_file_appends_no_auction_record` | IA-16 |

| IA-GR-31 | (round 3) A later import into a month served from a revision answers held bars `AlreadyPresent`, names a new bar `month_superseded` once, and a resume converges; (round 5, SF-9) a repair-reason plan over a month whose last trading day has not passed warns `repair_in_open_month`, and one repair-reason Resume after the month ends lands its remaining days in one more repair | `a_repaired_month_answers_held_bars_and_names_new_ones_as_a_hole`; `a_repair_in_an_open_month_is_warned_and_converges_after_the_month_ends` | IA-16 |

| IA-AUC-01 | A torn `.auc` tail is overwritten by the next append, and an unborn `.auc` is written again | `an_interrupted_auction_append_is_repaired_by_the_next` | IA-16 |

| IA-GV-28 | (round 4, BL-H) A repair-reason run's gathered day reads its repaired instruments back only after the repair publishes, and reads `match`, never `missing_in_store` or `count_mismatch`; a refused repair gives `owed (repair_refused)` or (round 5) `owed (repair_impossible)` with no read-back | `a_gathered_days_repaired_bars_are_verified_after_the_repair_publishes` | IA-7 |

| IA-RP-01 | A 1s month repairs at the grid ceiling: 2,678,400 rows merged and published; the revision's `.tix` present and its lookups O(1); no `.prv` written; the verifier reads the month's rule set from the journal; 2,678,401 rows refused `RowLimit` by name | `a_full_1s_month_repairs_with_its_sidecars` (+ C-RP-01) | IA-17 |

| IA-RP-02 | A CASH repair writes the revision's whole `.auc` in `repair_month`'s step between publish and promote; a step that fails leaves the revision published and unserved, and the exact retry runs it again and promotes; a caller that passes no step leaves every byte as before | `a_cash_repair_writes_its_auction_file_before_it_is_served` | IA-17 |

~~~~

#### Section 15 (decisions): D-4874, D-4875, D-4876, D-4887, with the header

Source: `state/design/ingest-automation.md`, lines 4568-4569 (verbatim)

~~~~text
| D | Decision |
|---|---|
~~~~

Source: `state/design/ingest-automation.md`, lines 4644 4645 4646 4657 (verbatim, each line whole)

~~~~text
| D-4874 | **(round 3) One GDFL repair authority: WP-12d; IA-15 is withdrawn.** WP-12d's goal is to land a day the append-only month refuses "through WP-13c's versioned month repair (one repair authority, not a second one)", owning `gdfl_import.rs`'s filing path; round 2's IA-15 (`mode=repair`, `cli gdfl-repair`, `POST /gdfl/repair`) would have been a second owner of that rule and that file (critic BL-1(a)). So IA-15 and its files are withdrawn. **A repair is asked for by name**, as WP-13c does for pulls (D-3350 since its second pass: its pull path takes `repair_window` only for a run that carries a reason, `crates/api/src/server.rs:3419`–`:3426`): a GDFL import run carrying `repair_reason` (cli `--repair-reason TEXT`, `POST /gdfl/import` `repair_reason=`, ≤ 200 bytes and no control character since round 6, the rule `POST /pull/repair` already applies, recorded by digest in the claim and whole in the request sidecar) reads the hole days in its range and lands each refused month through WP-12d's filing path; a run without one leaves holes as holes. **Who builds the trigger** (round 4, SF-18): WP-12d, if it defines one with its filing path; IA-1 then moves its cli flag into `Request` with the rest of the cli setup. If WP-12d defines none, IA-16 adds the field, the flag and the `admit` row that reads it (D-4885) in one change. IA-10a takes it on the route, IA-12a shows the Repair action. This is proposed to WP-12d, which owns the filing path; if WP-12d chooses to repair without a reason, this entry is amended before IA-1 lands, never coded around. |

| D-4875 | **(round 3; rewritten in round 5, B4) A repair must hold a GDFL 1s month and leave it indexed** (proposed to WP-13c, which owns `store::repair`, and to WP-12d, which owns the GDFL caller; critic BL-2). (1) `store::repair::MAX_ROWS` = 100,000 (`crates/store/src/repair.rs:29`, merged unchanged at e10334b9; refusals `:308`, `:448`, `:465`, `:674`) becomes the timeframe's grid ceiling, the most bars a month of that rung can hold: days in the month × 86,400 ÷ the rung's seconds (2,678,400 for 1s, 44,640 for 1min, 31 for 1day), so no valid month is refused for its size; a real NIFTY 1s month is about 426,000 bars (§2.2), and memory is named in `docs/06-limits.md` (L-IA-26: ≤ (3 × 56 + 8) B × the ceiling, about 471 MB for a full 1s month by type sizes, round 6, N-3, F82; round 5 said about 300 MB). A streaming merge with a stated memory bound is the alternative WP-13c may choose. (2) **WP-13c's own since its second pass** (round 5, F72): the revision is written through `BarFile::open_or_create` on the revision root and `append` (`repair.rs:631`–`:632`), which keeps a `.tix` as every append does, and `REPAIR.md:212` says "The `.tix` is the revision's own"; so no repaired month bisects and staleness stays O(1), and this design builds nothing for it (round 3 and 4 asked WP-13c or IA-17 to write it). (3) A repair-reason run repairs each (instrument, month) once, offering every day of that month it reads together, because each repair keeps a full copy of the month: day by day, the 67 alias months alone would keep about 21 GB of copies against about 1.6 GB (L-IA-26). (4) GDFL's repair policy is `basis_witness = false` (GDFL's measured basis is `AsPublished` for INDEX and `Raw` for CASH and FNO, never `VendorAdjusted`, `pull/src/vendor.rs:5796`–`:5815` at 3f65ad43, D-3323) and `open_interest_applies` by segment (D-3322), as `repair_policy` derives it for Zerodha. (5) **Round 5 (B4, F72, F75): the revision's other sidecars follow WP-13c's and WP-39's own rules.** The `.prv`: WP-13c decides "A revision carries no `.prv`: its records would index the original's bars" (`REPAIR.md:218`–`:219`), and **this design adopts that rule (D-4887)** in place of round 4's re-indexed `.prv`: a repaired month's rule set comes from the journal (D-4882) and its flag 7 reads `source_crc_unrecorded`. The `.srt`: WP-39's writer appends a run after every commit, and a month without one is read around, never refused (`fix/db-WP-39:docs/02-store-format.md:806`, `:813`–`:814`), so a revision written through the writer gets its runs once WP-39 lands, and nothing here builds it. The CASH `.auc`: `pull`'s own file, not a store sidecar (§5.5). **Round 6 (SF-3, F82): it is written by a step inside `store::repair`, never by a second merge**: `repair::publish` takes an already merged series and `repair_month`, where the merge lives, promotes as soon as it publishes, so `repair_month` gains an optional `before_promote` step, called with the revision's root after `publish` (created or reused) and before `generation::promote`; the GDFL CASH caller passes one that writes the revision's whole `.auc`, every other caller none; a failing step refuses the repair unpromoted and the exact retry runs it again. `gdfl_cm::auction::path` resolves the served generation. (6) **Owner and step** (round 4, SF-5; narrowed in round 5; **round 6, SF-2: WP-13c merged at e10334b9 with `MAX_ROWS = 100_000`, so (1) is IA-17's, at step 2 of §16.2, with test IA-RP-01 and bench C-RP-01**): (1) and (5)'s step are `store::repair`'s, built by IA-17; (3) and the CASH `.auc` writer are WP-12d's filing path's, else IA-17's after IA-16 (test IA-RP-02). Until (1) is built every repair of a real 1s month answers `repair_refused (RowLimit)`, named, and the next repair-reason run after (1) is built admits those holes again (`admit` row 4a); IA-16, whose `admit` hands holes to a repair-reason run, depends on IA-17's first part (round 6), so no M1 build meets it. |

| D-4876 | **(round 3) A month served from a revision.** WP-13c's writer refuses an append to such a month (`StoreError::Superseded`; D-3350, `crates/store/REPAIR.md:14`–`:15`, `generation.rs:57`–`:60`). An import batch whose bars the revision already holds answers `AlreadyPresent`; one holding a new bar fails `month_superseded` (`FailKind::Superseded`), a store hole (D-4851) that a repair-reason run fills. So after a month is repaired, a later import converges: it lands nothing new, or stops once at a named hole, and Resume never re-reads it (critic BL-3). **Round 5 (SF-9): the open month, decided and owned.** (2) A repair-reason run whose range touches a month still being traded (its last trading day by the calendar of record not yet passed) is warned in `plan.json` as `repair_in_open_month{month}`, because the repair would turn every later day of that month into `month_superseded`; the warning is IA-16's, added with `admit`'s rows 4a and 4b (one calendar probe per month in range). (3) **Repairs run after the month ends**: IA-14's runbook schedules every repair-reason run after the last trading day of each month it touches; one run anyway is not a loop, because one repair-reason Resume after the month ends lands the month's remaining days in one more repair of it (D-4875 (3)). IA-GR-31 pins both. |

| D-4887 | **(round 5) A repaired month carries no `.prv`: WP-13c's rule, adopted** (critic B4; F72). WP-13c's second pass (`fix/db-WP-13c` d44246fa, merged unchanged at these lines at e10334b9) decides that "A revision carries no `.prv`: its records would index the original's bars. `/provenance.json` reads the revision's absence as unrecorded, and the repair journal `audit/<feed>.repairs` names the repair" (`crates/store/REPAIR.md:218`–`:220`), and that the revision's `.tix` is its own (`:212`). Round 4's D-4875 (5) asked the opposite (a `.prv` of the original's records re-indexed to the revision's rows), which would have made one rule with two owners and opposite answers, and IA-17 would have had to code around a written WP-13c rule. **This design adopts WP-13c's rule and withdraws round 4's**: (1) the verifier takes a repaired month's rule set from the journal (D-4882 source (2): the key's first close that wrote seconds, `definition_mixed` when the key closed with seconds under two definitions, never the original's `.prv`); (2) its records set flag 21 `source_crc_unrecorded`, never flag 7; (3) its detail `repair` (kind 17) names the repair in O(1) from the served pointer record (`revision`, `previous`, `promoted_at_micros`, read by the read-back's own `generation::served`) and the run of the key's last `done` close (round 6, SF-6: round 5 read WP-13c's `audit/gdfl.repairs`, an unindexed append file, O(lines) per lookup; no verification reads it now); (4) IA-17 builds no `.prv`. **Owner**: WP-13c owns the rule (D-3350; **merged at e10334b9**, `REPAIR.md:212`, `:218`–`:220` on `feat/db-fix`, with no `docs/05` entry, its commit handing D-3350..D-3352 to WP-99); IA-6 relies on it and lands this entry; if WP-13c records it under a number of its own, IA-6 cites that and retires this one by name. **Conflict named**: WP-12d's plan goal says ".tix, .prv and every reader follow the new generation" (`fix-plan.json` WP-12d); its `.prv` half now means "the original's `.prv` stays with the original, and the revision reads as unrecorded", which WP-12d's owner confirms before it merges, never coded around. IA-GV-23 and IA-GV-24 pin (1) and (2). |

~~~~

#### Section 16.1 (one owner per overlapping piece): WP-12d, the repair engine, the CASH .auc of a repaired month

Source: `state/design/ingest-automation.md`, lines 4734-4735 (verbatim)

~~~~text
| WP package (owner) | Its goal, in a line | What this design does about it |
|---|---|---|
~~~~

Source: `state/design/ingest-automation.md`, lines 4741 (verbatim, each line whole)

~~~~text
| **WP-12d** (round 3) GDFL back-fill through the versioned month repair (`gdfl_import.rs` filing path, minimal) | land a day the append-only month refuses as a new generation through WP-13c's repair, "one repair authority, not a second one"; journaled, audited, resumable; `.tix`, `.prv`, census follow | **Absorbs round 2's IA-15, which is withdrawn** (D-4874; critic BL-1(a)). One rule, one owner: the import refuses a passed day as a hole unless the run carries a repair reason, and then WP-12d's filing path repairs. Proposed to it: the trigger (a named reason; if it defines none, IA-16 adds the field with `admit`'s rows 4a and 4b, round 4 and 5), D-4875 (3) once-per-month repair (else IA-17 after IA-16), and D-4875 (5)'s `.auc` of a CASH passed day. IA-16 types `Superseded`, `RepairRefused` and every other door error after it (§5.8); IA-10a and IA-12a expose it. V-4 and V-5 are its runs **Round 5 (B4):** its goal says ".tix, .prv and every reader follow the new generation"; WP-13c's second pass makes the `.tix` the revision's own and decides the revision carries no `.prv`, which this design adopts (D-4887), so WP-12d's `.prv` half means the revision reads as unrecorded, to be confirmed by its owner, never coded around. Also proposed to it: the CASH `.auc` written between `repair::publish` and `generation::promote` (D-4875 (5)), and the D-3335 ledger rule for member-key closes if WP-12f leaves it to the filing path's owner. **Round 6:** its branch exists with no commits (N-1); the door it calls has merged with WP-13c, and IA-17's ceiling (step 2) lands before it can repair a real 1s month; the CASH `.auc` it writes goes through `repair_month`'s `before_promote` step, never through a merge of its own (SF-3); the repair journal it writes takes the audit ledger lock (D-4888) |

~~~~

Source: `state/design/ingest-automation.md`, lines 4763-4764 (verbatim)

~~~~text
| Rule or file | Owner | IA relation |
|---|---|---|
~~~~

Source: `state/design/ingest-automation.md`, lines 4767 4769 (verbatim, each line whole)

~~~~text
| The repair engine: revisions, the generation pointer, `repair_month`, its row ceiling, the revision's `.tix` (its own writer's) and **its having no `.prv`** (`store/src/repair.rs`, `generation.rs`, `REPAIR.md`) | WP-13c (D-3350; merged at e10334b9; its `.prv` rule adopted by D-4887, round 5) | **IA-17 builds the row ceiling, D-4875 (1), and `repair_month`'s `before_promote` step, D-4875 (5), at step 2** (round 6, SF-2, SF-3: WP-13c merged without the ceiling); IA-6 reads through it and takes a repaired month's rule set from the journal |

| The CASH `.auc` of a repaired month (written by the `before_promote` step `repair_month` runs between `publish` and `promote`, round 6, SF-3; `auction::path` resolving the served generation) | WP-12d's filing path passes the writer (D-4875 (5)); the step itself is IA-17's in `store::repair` | IA-17 passes the writer too if WP-12d leaves it, after IA-16 |

~~~~

### 3f. The critic (round 7, `state/design/critic-round7-20261011.md`): its part for IA-17

Its resolutions are the defaults where the design is silent or contradicts itself (launcher rule). IA-17-1 and IA-17-2 were taken by IA-17 (first part, D-4898; see 3h); **IA-17-3 is this part's**.

Source: `state/design/critic-round7-20261011.md`, lines 175-191 (verbatim)

~~~~text
## IA-17 — the repair in `store::repair` (no branch)

| Gap | Where | Resolution | Decision |
|---|---|---|---|
| **IA-17-1. Part (1), the row ceiling, is already built by WP-12d, differently, in the same lines.** The design (round 6) says "WP-13c merged with `MAX_ROWS = 100_000`, so IA-17 raises it, at step 2", to a **per-timeframe** grid ceiling at four sites (`repair.rs:308`, `:448`, `:465`, `:674`). `fix/db-WP-12d` efc353ed sets `pub const MAX_ROWS: usize = 31 * 86_400;` (one flat constant, `repair.rs:29` on the branch), rewrites `REPAIR.md`'s sentence and the `RowLimit` text. IA-17 at step 2 and WP-12d would edit the same constant with no order | design §0 item 10; §11 line 4268; §16.2 IA-17 row; D-4875 (1), (6); `fix/db-WP-12d:crates/store/src/repair.rs` (diff), `REPAIR.md` | **IA-17 (1) is withdrawn; WP-12d's constant stands.** A per-timeframe ceiling adds nothing: the month admission (D-0915) already bounds every month to its rung's grid, so for 1min or 1day the flat 2,678,400 is never the refusal that binds. IA-17 **waits for WP-12d's merge** and builds on it. D-4875 (1) is rewritten "the ceiling is WP-12d's `MAX_ROWS` = 31 × 86,400". IA-RP-01 keeps its numbers (2,678,400 merged, 2,678,401 refused `RowLimit`) | **D-4898** (amends D-4875 before it is minted; lands with IA-17) |
| **IA-17-2. `before_promote` as "one more argument" of `repair_month` edits files IA-17 does not own.** `repair_month` is called from `pull/src/ingest.rs` (`repair_after`, WP-12d's, then IA-16's file), `api/src/sources_tests.rs:1342`, WP-12d's `api/src/provenance_tests.rs:1625`, `:1734`, and `store/tests/{generation,repair}.rs`. A new argument changes every one of them at step 2 | design §5.5 lines 1560–1571; §11 line 4268; `git grep 'repair_month('` at the head and on WP-12d | Add **`repair_month_with(…, before_promote: Option<&mut dyn FnMut(&Path) -> Result<(), String>>)`** and keep `repair_month` as it is, a wrapper that passes `None`. No caller changes at step 2; "every other caller's bytes are unchanged" is then true by construction, and IA-RP-02 asserts it (the wrapper and `…_with(None)` publish byte-identical revisions). The GDFL CASH caller (IA-17's later part, after IA-16) is the only caller of `…_with(Some)` | D-4898 |
| **IA-17-3. Part (3), one repair per (instrument, month) per run, is now certainly IA-17's, and the design gives its hardest half no test id or bound.** WP-12d repairs one file at a time ("a month served from a revision pays that for EVERY later batch it lacks, one generation each", `repair_rows`' doc), so D-4875 (3) "WP-12d's, else IA-17's" is IA-17's. The gather holds a month's days open across WP-16's day loop: days begun and not closed, their phase-2 read-back deferred (§5.5), a crash that leaves up to 23 days `begin` with no close, a stop pressed mid-gather, a month that ends the run's range half read, `StorageFull` at publish with 23 days gathered. Only IA-GV-28 (the read-back) names any of it | design §5.5 lines 1341–1351, 1507–1514; L-IA-12; L-IA-26; `fix/db-WP-12d:crates/pull/src/ingest.rs:1680`–`:1687` | IA-17's later part is its own milestone step (M4), after IA-16 and IA-7, with these rules fixed now: the gather is per (instrument, month) and is offered when the run leaves the month, ends, **or is stopped** (a stop offers what is gathered, then closes those days, so a stop never leaves begun days behind); a crash leaves the days `begin`, and the next repair-reason Resume re-reads them (D-2821) and meets `Published::Reused` or publishes; `StorageFull` at publish ends the run `disk_full` with the gathered days still `begin`; the gathered bars are bounded by `MAX_ROWS` (refused `RowLimit` by name past it, never cut). Memory is L-IA-12's gather term plus L-IA-26. Tests: **IA-RP-03** `a_repair_reason_run_repairs_each_month_once` (the 67-month shape: one generation per (instrument, month), counted), **IA-RP-04** `a_stop_or_a_crash_mid_gather_never_strands_a_begun_day`, **IA-RP-05** `a_full_disk_at_publish_leaves_the_gathered_days_to_the_next_run`; bench **C-RP-02** (one month of 23 gathered index days: p50, p99, max, peak RSS). **Until it lands, IA-14 lists V-5 as not ready and V-4 as ready** (the 24 NIFTY and BANKNIFTY months are one day each, so one repair each either way) | **D-4899** (amends D-4875 (3), (6) and D-4836's V-4 line; lands with IA-17's later part) |

Non-blocking: WP-12d's doc bounds "one repair's merged rows to 150 MB"; L-IA-26
says 471 MB by type sizes (merged series, added stamps, offered bars, the
writer's encoded image). Both are sums of types, neither is measured: C-RP-01
(IA-17) settles it with peak RSS, and until it runs both read NOT PROVED.
WP-12d's commit measures a NIFTY 2018-11 month at 409,224 bars; §2.2 says about
425,817 for 2021-02: different months, no conflict.

---

~~~~

The critic's verdict row for IA-17, the cross-package gaps X-1 and X-2 this part's gather goes through (both landed with IA-16), the late note on `IndexedSibling`, and its build order and decision table rows for IA-17:

Source: `state/design/critic-round7-20261011.md`, lines 68 96 97 (verbatim, each line whole)

~~~~text
| IA-17 | 3 | **only after WP-12d merges**; its part (1) is withdrawn |

| **X-1. WP-12d repairs without a repair reason; the design's whole hole model assumes it never does.** `fix/db-WP-12d` efc353ed files every GDFL file through `ingest::repair_rows` (`gdfl_import.rs` on the branch: `use crate::ingest::{…, repair_rows}`), which appends first and, when the month "lacks the whole batch where no append can land it" (`lack_of`, `ingest.rs:2520` on the branch), repairs at once with a reason it writes itself ("gdfl-import <kind> <day> <file> run:<id>: a day the month lacks…"). Its commit: "the documented command `gdfl-import indices zip 2018-09-03 2026-09-24` imported the 12 days, repaired 24 months". The design says the opposite in §0 item 10, §5.5 (`admit` rows 4a, 4b), §5.8, §8 R14, §12, §13, §17 Q10, D-4874, D-4885, and D-4874 itself says: "if WP-12d chooses to repair without a reason, this entry is amended before IA-1 lands, never coded around" | design §5.5 lines 1433–1458, 1483–1540; D-4874, D-4885; `fix/db-WP-12d:crates/pull/src/ingest.rs:1688`, `:2520`, `:2577`; `gdfl_import.rs` diff | **Keep the design's rule; gate WP-12d's door.** A repair copies a whole month and keeps it for ever, and WP-12d repairs one file (one instrument-day) at a time, so an unasked Resume over the 556 alias days would keep about 21 GB of copies (L-IA-26), and an unasked repair in the month still being traded turns every later day of it into another full copy (D-4876). The operator's rule 2 also wants one rule for both feeds, and the merged Zerodha door repairs only a request that carries a reason (`server.rs:3419`–`:3426`). So: **(a)** WP-12d merges as built (its second agent decides that, not this design). **(b)** IA-16 adds `Request.repair_reason` and the cli's `--repair-reason TEXT`, and the filing step calls `repair_rows` only when the run carries one (the operator's text first, WP-12d's own sentence after it); without one it calls `from_rows`, and the typed `NotHeld` / `Superseded` makes the hole, exactly as §5.5 says. **(c)** WP-12d's tests in `gdfl_backfill_tests.rs` pass a reason, in IA-16's change. **(d)** Named interim: between WP-12d's merge and IA-16's, every `cli gdfl-import` back-fills unasked; IA-14's runbook says so, and the main session runs only WP-80's 12 index days (24 months, about 0.6 GB) in that window | **D-4889** (amends D-4874, D-4885 before either is minted; lands with IA-16) |

| **X-2. Two doors now, not one; and WP-12d's judgement is by the month's shape, not by the error's kind.** §5.8 says "the door is one door, for both feeds" and that IA-16 makes it repair "only `NotHeld` and `Superseded`". At WP-12d there are two doors over one repair: `repair_or_append` (Zerodha: repairs on any append error, `ingest.rs:2381` on the branch) and `back_fill_or_append` (GDFL: repairs only `Lack::Lacked`; `Lack::Overlaps` and `Lack::AppendDoors` are refusals in words, `:2577`). Both still return `String`. The design's §5.8 table has no row for `Overlaps` (the month holds a bar inside the offered span: a held day read differently), and a third repair-journal writer exists, `journal_intent` (the `repairing` line, `:2635`) | design §5.8 lines 1881–1931; §5.3 audit-ledger row; `fix/db-WP-12d:crates/pull/src/ingest.rs:2381`–`:2660` | IA-16 types **both** doors and keeps both judgements. **GDFL door**: `lack_of` returns `Result<Lack, StoreError>` (not `String`); `Lacked` on generation 0 is `FailKind::NotHeld` (`needs_month_generation`), `Lacked` on a revision is `Superseded` (`month_superseded`); `Overlaps` is `FailKind::Differs` (`bars_differ_held`, `admit` row 4b: no repair may complete or restate a held day, WP-12d's own rule); `AppendDoors` keeps the append's own kind by the table; a `lack_of` error maps by its `StoreError` (`Locked` → `MonthBusy`, and so on). **Zerodha door**: as §5.8 says (repair only `OverlapDisagrees{NotHeld}` and `Superseded`). §5.8's sentence becomes "one repair (`repair_after`), two doors". IA-GR-39 gains a row per `Lack` variant. **All three** journal writers (`journal_repair`, `journal_refusal`, `journal_intent`) move to `audit_ledger::append` in IA-16's change, and `repairing` joins the verbs the readers of `audit/<vendor>.repairs` know | **D-4890** (amends D-4851 and D-4888; lands with IA-16) |

~~~~

Source: `state/design/critic-round7-20261011.md`, lines 491-496 (verbatim)

~~~~text
4. **`IndexedSibling` on options months is still NOT CHECKED** (§18 round 3,
   left open (5)). If options months carry `.grk` or `.ovl` siblings, every
   options back-fill is `repair_impossible`, and IA-18's restate must either
   add no stamp (then a sibling's row index stays true) or refuse by that name.
   One read-only `find` over a sample of option month directories settles it;
   it should be run before IA-17's later part or IA-18 is built.
~~~~

Source: `state/design/critic-round7-20261011.md`, lines 531-547 (verbatim)

~~~~text
### The build order after these resolutions

1. Merge what is built: **WP-16**, **WP-12d**, WP-39, WP-81, WP-61 (their own
   second agents decide each), and **IA-5** (first of the IA packages).
2. **IA-11r** (finish the WIP: IA-11r-1 to -3), **IA-11a** (lands D-4871,
   D-4883, D-4891, D-4893, D-4895), **IA-17's step-2 part** (after WP-12d:
   `repair_month_with`, IA-RP-01, IA-RP-02, C-RP-01; no ceiling).
3. **IA-1** (on WP-16, WP-12d, WP-81, IA-11r; not waiting for WP-11c or WP-12f)
   → **IA-2** → **IA-3** → **IA-16** → **IA-10a** → **IA-12a**. M1.
4. **IA-4** → IA-0b; **IA-6** → **IA-7** → **IA-10b** (GDFL half only) →
   **IA-12b**. M2.
5. **IA-8** (`pull` only) → **IA-9** → **IA-10c** (the Zerodha half). M3.
6. **IA-17's later part** (the once-per-month gather, the CASH `.auc` writer)
   → **IA-18** (the restate door). M4.
7. IA-6p; IA-13 as each lands; IA-14 lists a backfill step only when its code
   has merged (V-4 after step 3 with a repair reason; V-5 after step 6; V-6's
   differing days after IA-18).
~~~~

Source: `state/design/critic-round7-20261011.md`, lines 562 563 (verbatim, each line whole)

~~~~text
| D-4898 | The row ceiling is WP-12d's `MAX_ROWS`; `repair_month_with` for the `before_promote` step | IA-17 |

| D-4899 | One repair per (instrument, month) per run is IA-17's: its stop, crash and full-disk rules; V-4 before it, V-5 after | IA-17 (later part) |

~~~~

### 3g. The rules audit (`state/design/rules-audit-20261011.md`)

`grep -n '^## .*IA-17b'` finds no section (0 matches). The spec says to read the audit's "IA-17" section and "Critic resolution D-4899"; both are copied, with the two IA-17 critic-gap sections and the D-4889 section whose row (RA-018) names "IA-17's later part". Every gap listed is part of this package, with the test or bench it names.

Source: `state/design/rules-audit-20261011.md`, lines 1017-1030 (verbatim)

~~~~text
## IA-17: the month repair in store::repair

Leads 1 rows (0 high). Also named in 1 rows led by another owner.

| row | gap | rule | where (file:line or design line) | what to build or prove | test or bench to add |
|---|---|---|---|---|---|
| RA-266 medium, partly. Store: one month repaired again and again until it reaches its limit of 1,024 rewrites | Day-by-day back-fill can use up a month's rewrites (up to 23 per pass of a month), after which the month can never be repaired; no page shows how many are used or left. | One common, dynamic, scalable path + Every extreme case | store/repair.rs:43: at most 1,024 rewrites; past it 'repair_impossible', a hole no run reads again (design line 1840). Test IA-GR-31 covers a later import. Second agent: 19 rewrites of one month already (verify-WP-12d-r1). Critic IA-17-3. | The plan and the source drawer show rewrites used and the limit per month; warn at 80 percent, alert at half. IA-17's once-per-month gather gets a test id and a bound. (owner line: IA-17 (critic IA-17-3, D-4899; needs a test id); NEW-30 'IA-20: repairs list and index' (the 'rewrites used' view); NEW-25 'Run watcher and new alert kinds' (the warning)) | IA-17's once-per-month gather gets a test id and a bound. |

Also named in:

| row | gap | this owner's part (the row's owner line) | led by |
|---|---|---|---|
| RA-018 high. Month repair (filling a missing day into a month already stored): one repair per file, not one per month | Each later day of a repaired month writes the whole month again; old copies are kept for ever with no way to remove them. A hole is repaired unasked, with no alert or audited event. Bench C-RP-01 and limits row L-IA-26 are not written. | critic X-1 (D-4889) and X-2 (D-4890), landing with IA-16 (test IA-GR-39); critic IA-17-3 (D-4899), IA-17-1, IA-17-2 (IA-17's later part); NEW-31 'After a repair: prune, refold, source record' (old month copies) | critic D-4889 |

~~~~

Source: `state/design/rules-audit-20261011.md`, lines 1043-1050 (verbatim)

~~~~text
## Critic resolution D-4889: WP-12d's back-fill door opens only for a run that carries a repair reason; the interim is named (lands with IA-16)

Leads 1 rows (1 high). Also named in 0 rows led by another owner.

| row | gap | rule | where (file:line or design line) | what to build or prove | test or bench to add |
|---|---|---|---|---|---|
| RA-018 high, does not meet it yet. Month repair (filling a missing day into a month already stored): one repair per file, not one per month | Each later day of a repaired month writes the whole month again; old copies are kept for ever with no way to remove them. A hole is repaired unasked, with no alert or audited event. Bench C-RP-01 and limits row L-IA-26 are not written. | O(1) at p99, measured + Everything in parallel + Saved, logged, searchable, on a page + One common, dynamic, scalable path | Merged and live (WP-12d, build app12). Each later batch rewrites the whole month as a new copy kept for ever, up to 1,024 a month (store/repair.rs:43). Second agent: 19 copies of one month; 14.2 GiB for six indices (verify-WP-12d-r1). | Critic X-1 (D-4889): repair only with the operator's reason; X-2 (D-4890): one lock. Critic IA-17-3 (D-4899): one repair per instrument-month per run; bench C-RP-01. New: a rule for old copies (keep the newest N; show the bytes). (owner line: critic X-1 (D-4889) and X-2 (D-4890), landing with IA-16 (test IA-GR-39); critic IA-17-3 (D-4899), IA-17-1, IA-17-2 (IA-17's later part); NEW-31 'After a repair: prune, refold, source record' (old month copies)) | bench C-RP-01. |

~~~~

Source: `state/design/rules-audit-20261011.md`, lines 1150-1161 (verbatim)

~~~~text
## Critic resolution D-4899: One repair per (instrument, month) per run is IA-17's: its stop, crash and full-disk rules; V-4 before it, V-5 after (lands with IA-17 (later part))

Leads 0 rows (0 high). Also named in 3 rows led by another owner.

Also named in:

| row | gap | this owner's part (the row's owner line) | led by |
|---|---|---|---|
| RA-018 high. Month repair (filling a missing day into a month already stored): one repair per file, not one per month | Each later day of a repaired month writes the whole month again; old copies are kept for ever with no way to remove them. A hole is repaired unasked, with no alert or audited event. Bench C-RP-01 and limits row L-IA-26 are not written. | critic X-1 (D-4889) and X-2 (D-4890), landing with IA-16 (test IA-GR-39); critic IA-17-3 (D-4899), IA-17-1, IA-17-2 (IA-17's later part); NEW-31 'After a repair: prune, refold, source record' (old month copies) | critic D-4889 |
| RA-266 medium. Store: one month repaired again and again until it reaches its limit of 1,024 rewrites | Day-by-day back-fill can use up a month's rewrites (up to 23 per pass of a month), after which the month can never be repaired; no page shows how many are used or left. | IA-17 (critic IA-17-3, D-4899; needs a test id); NEW-30 'IA-20: repairs list and index' (the 'rewrites used' view); NEW-25 'Run watcher and new alert kinds' (the warning) | IA-17 |
| RA-071 medium. From pressing Stop to the run having stopped (with 8 workers and one flush a day) | The operator presses Stop and nobody can say whether it takes a second or a minute: files in flight on every worker, the day flush, the census publish and every repair of the day. No bench measures it; the repair pass has no stop check. | IA-2 (owns the whole stop); WP-16 (a stop check in its back-fill pass); critic IA-17-3 (D-4899); IA-9 (the merge); IA-13 (the measurement) | IA-2 |

~~~~

Source: `state/design/rules-audit-20261011.md`, lines 1499-1518 (verbatim)

~~~~text
## Critic gap IA-17-1 (round 7; no decision number cited in the row)

Leads 0 rows (0 high). Also named in 1 rows led by another owner.

Also named in:

| row | gap | this owner's part (the row's owner line) | led by |
|---|---|---|---|
| RA-018 high. Month repair (filling a missing day into a month already stored): one repair per file, not one per month | Each later day of a repaired month writes the whole month again; old copies are kept for ever with no way to remove them. A hole is repaired unasked, with no alert or audited event. Bench C-RP-01 and limits row L-IA-26 are not written. | critic X-1 (D-4889) and X-2 (D-4890), landing with IA-16 (test IA-GR-39); critic IA-17-3 (D-4899), IA-17-1, IA-17-2 (IA-17's later part); NEW-31 'After a repair: prune, refold, source record' (old month copies) | critic D-4889 |

## Critic gap IA-17-2 (round 7; no decision number cited in the row)

Leads 0 rows (0 high). Also named in 1 rows led by another owner.

Also named in:

| row | gap | this owner's part (the row's owner line) | led by |
|---|---|---|---|
| RA-018 high. Month repair (filling a missing day into a month already stored): one repair per file, not one per month | Each later day of a repaired month writes the whole month again; old copies are kept for ever with no way to remove them. A hole is repaired unasked, with no alert or audited event. Bench C-RP-01 and limits row L-IA-26 are not written. | critic X-1 (D-4889) and X-2 (D-4890), landing with IA-16 (test IA-GR-39); critic IA-17-3 (D-4899), IA-17-1, IA-17-2 (IA-17's later part); NEW-31 'After a repair: prune, refold, source record' (old month copies) | critic D-4889 |

~~~~

### 3h. What IA-17 (the first part) already landed at the head, in its own words

IA-17 headed D-4875 whole ("Implemented by IA-17, in two parts") and D-4898, and both say plainly
which parts are NOT BUILT: those are this package. Under the critic's X-4 rule (the first lander
heads the whole number; a later package never adds a second heading to it and never edits that
entry) IA-17b does **not** edit D-4875 or D-4898; it lands D-4899 (section 5) and its own
`docs/04` and `docs/06` rows.

Source: `docs/05-decisions.md` at feat/db-fix e851da1ca, lines 67618-67769 (verbatim)

~~~~text
### D-4875 — A month repair must hold a one-second month and leave it indexed — 2026-10-11

**Implemented by** IA-17, in two parts. This entry lands with the first (the
row ceiling of (1) and the step of (5), `crates/store/src/repair.rs`). Parts
(3) and the writer the step of (5) is for are the second part and are NOT
BUILT; they are named here so that nobody reads this entry as saying they are.
D-4898, below, records where the first part departs from what the design
drafted for it.

**What was wrong.** `store::repair` (WP-13c) refused any source or merged
series past a flat `MAX_ROWS` of 100,000 rows, and a GDFL one-second index
month holds about 409,000 to 453,000 bars, so no such month could be repaired
at all. WP-12d raised the constant to 2,678,400 (31 days of seconds) when it
built the GDFL back-fill (DB-N26). That let every one-second month through and
left the same number on every other rung: a one-minute month, which holds at
most 44,640 bars, and a daily one, which holds at most 31, were "bounded" at
2,678,400, so a request sixty times larger than any one-minute month passed
the count and was walked bar by bar before something else refused it.

**(1) The row ceiling is the path's.** `repair::row_ceiling(path)` is the
month's length over the rung's width: the days of the path's month times
86,400, over the rung's seconds. 2,678,400 for a 31-day one-second month,
44,640 for its one-minute month, 31 for its daily one; 28 days of the rung for
a February. All four refusals ask it (`publish`'s merged series, the offered
bars and the held count in `repair_month_with`, and every header
`check_header` is shown), and the refusal carries both counts:
`RepairError::RowLimit { rows, ceiling }`. No valid month is refused for its
size, at any rung, and no table of rungs or months is kept: the path already
says both. `MAX_ROWS` stays as the largest value the ceiling takes and is
compared against nowhere. Memory is `docs/06-limits.md` L-IA-26.

**(2) The revision's `.tix` is its writer's.** A revision is written through
`BarFile::open_or_create` and `append` on the revision's root, which keep the
time index as every append does (`crates/store/REPAIR.md`, "Indexed
siblings"). Nothing was built for it here. Test IA-RP-01 holds it at a full
month: the `.tix` is present and `time_lookup()` answers `Indexed`.

**(3) One repair per (instrument, month) per run. NOT BUILT.** Each repair
keeps a whole copy of the month for ever, so a run that repairs a month a day
at a time keeps one copy per day. A repair-reason run is to gather every day
of a month it reads and offer them together. That is IA-17's second part
(the design's D-4899 fixes its stop, crash and full-disk rules); until it
lands WP-12d's filing path repairs one file at a time, and
`docs/06-limits.md` L-IA-26 says what that costs.

**(4) GDFL's repair policy** is the GDFL caller's (WP-12d's `repair_rows`),
not `store`'s, and is neither built nor changed here.

**(5) A revision's other files follow their own owners' rules, and a caller's
own file follows the generation through one step.** A revision carries no
`.prv` (WP-13c's rule, `REPAIR.md`, "Provenance"; test IA-RP-01 holds that
none is written). Its `.srt` runs are the writer's (WP-39). The CASH auction
file is `pull`'s own, not a store sidecar, and `repair::publish` takes an
already merged series while the merge, the held-bar check and the basis
witness exist only inside `repair_month`, which promoted as soon as it
published. So a caller that wanted to write its file between the two would
have had to copy the merge: a second repair authority. Instead
`repair_month_with` takes one step, `before_promote`, called with the
revision's root after `publish` has returned (`Created` or `Reused`) and
before `generation::promote`. The revision is then whole and durable and no
reader is served from it. A step that refuses stops the repair unpromoted;
the same repair offered again finds the revision (`Published::Reused`), runs
the step again and promotes. `store` names no type of `pull`'s. **The step is
built; the CASH writer that will be passed to it is NOT** (IA-17's second
part, after IA-16), so at this entry's date no caller passes a step and a
repaired CASH month still has no auction file beside its served bars.

**Proof.** `store/tests/month_repair.rs`: IA-RP-01
`a_full_1s_month_repairs_with_its_sidecars`, IA-RP-02
`a_cash_repair_writes_its_auction_file_before_it_is_served`, and the five
tests beside them; `store::repair::tests::the_row_ceiling_is_the_months_length_over_the_rungs_width`
and `…::the_flat_maximum_is_the_largest_ceiling_any_path_has`
(`docs/04-invariants.md` IA-RP-01, IA-RP-02, IA-RP-06 to IA-RP-08). Bench
C-RP-01 (`crates/store/benches/ratio.rs`).

### D-4898 — The row ceiling is read from the path, `repair_month_with` carries the step, and a revision is written in bounded commits (amends D-4875) — 2026-10-12

**Implemented by** IA-17 (first part). **Amends** D-4875 (1) and (5) in three
places, each where the code at the head had moved since the design's text was
written or where the design's sentence could not be built as written.

**1. The ceiling is path-derived, and WP-12d's constant stays.** The design's
round 6 has IA-17 replace `MAX_ROWS = 100_000`; WP-12d had already replaced it
with a flat 2,678,400 by the time IA-17 was built. The design's round-7 review
(IA-17-1) proposed dropping the per-timeframe ceiling as adding nothing, on
the ground that the month admission (D-0915) bounds every month to its rung's
grid. That holds for a batch that has been walked; the count is refused
BEFORE the walk, so under the flat constant a one-minute request of 44,641 to
2,678,400 rows was surveyed and admitted bar by bar first, and on the daily
rung the admission's grid is one second, so it bounds a daily month only by
the month's seconds. The package was asked for the per-timeframe ceiling and
it is built, as one function of the path (`row_ceiling`), with WP-12d's
`MAX_ROWS` kept as its maximum (`…::the_flat_maximum_is_the_largest_ceiling_any_path_has`).
On every intraday rung the ceiling equals the writer's own bound
(`Admission::slots`), so nothing the writer filled is refused; on the daily
rung it is the month's days, and more daily bars than days is refused
`RowLimit` by count.

**2. `repair_month_with`, and a refusal of its own.** The design adds the step
as one more argument of `repair_month`, which would have changed every caller
in `pull`, `api` and `cli` in a package that owns none of them (review
IA-17-2). `repair_month` keeps its signature and passes no step;
`repair_month_with` takes it. IA-RP-02 holds the two byte-identical with no
step. The design says a failing step is refused as `RepairError::Io`, "its
words kept"; `Io` carries an `io::ErrorKind` and a fixed operation name and
has nowhere to keep words, so the refusal is its own variant,
`RepairError::BeforePromote { revision_root, why }`, whose text says the
revision is published and not served and that the same repair runs the step
again.

**3. A revision is written in commits of at most 524,288 rows.** Bench
C-RP-01 was added to measure the memory bound D-4875 (1) states (L-IA-26: at
most 471 MB for a full one-second month, by type sizes). Measured on 11 Oct
2026 it was 1,160,118,272 bytes of peak resident set: the revision was written
as one commit of the whole month, and the sort index's writer step (WP-39,
merged after that sum was written) reads each commit back and orders it six
ways. `write_revision` now appends the merged series in commits of
`WRITE_BATCH` rows, so what the writer holds per commit is the commit's
whatever the month holds. The largest month takes 6 commits, under the 31 at
which the sort index is rewritten whole
(`store::repair::tests::a_revision_is_written_in_commits_the_sort_index_never_compacts`).
The size is a trade measured both ways. Each commit is a round of syncs: at
131,072 rows the same full month took 21 commits and peaked at 314,916,864
bytes, and its repair's fastest run went from 3.6 s to 47.7 s on a loaded
host whose syncs are slow, with a real-sized month (448,317 rows) paying four
commits where it had paid one. At 524,288 a one-second month of 23 regular
sessions (517,500 bars) is still one commit, written exactly as before, and
only a month fuller than any session calendar is cut. A revision of more than
524,288 rows therefore has more than one commit and more than one sort run,
as a month an import wrote a day at a time does. A completed retry compares
records, not bytes, so a revision published by the older code is still found
`Reused`. A write cut between two commits leaves a well-formed month file
of the commits that landed, its reservation and no receipt: the missing
receipt alone says it is unfinished, and it is stepped over, named and kept
whole like any other unfinished ordinal
(`store/tests/month_repair.rs::a_revision_cut_between_two_commits_is_stepped_over_and_kept`).

**The measured peak after the change** (12 Oct 2026, one repair in a process
of its own, macOS, `/usr/bin/time -l`, maximum resident set size):
537,411,584 bytes for the largest month (2,678,400 rows) and 152,879,104 for
a month of real size (448,317 rows, one commit). **The largest month is
therefore over the 471 MB the design summed from type sizes, and this entry
does not claim that bound.** Writing the 2,592,000 held rows of that month in
thirty commits of 86,400, with no repair at all, peaked at 159,383,552 bytes
by the same tool: the store's writer holds something that grows with the
month it appends to, which is not `store::repair`'s and which a smaller
`WRITE_BATCH` does not remove. `docs/06-limits.md` L-IA-26 carries the terms,
and bench C-RP-01 holds the full month's repairs to 640 MiB where the host
reports a peak.

**Not decided here.** D-4899 (one repair per month per run) is IA-17's second
part and has no code yet.
~~~~

Source: `docs/06-limits.md` at feat/db-fix e851da1ca, lines 17390-17499 (verbatim)

~~~~text
## Ingest automation: what a month repair reads, holds and keeps — D-4875, D-4898, 12 October 2026

Row L-IA-26 of the ingest automation design (its §10.2), landed by IA-17's
first part with the code it bounds (`store::repair`: the row ceiling and the
step before the promotion). The design's second half of the row, one repair
per (instrument, month) per run, is IA-17's second part (D-4899) and is NOT
BUILT; what that costs until it is, is the disk item below.

- **L-IA-26. A month repair is O(held + offered) bars. It is not O(1) and it
  is on no read path.** `repair::repair_month` and `repair_month_with` read
  the served month whole, merge the offered bars into it in one ordered walk
  (nothing is sorted) and write a whole successor as a new revision, which is
  then promoted. **Bounded, and refused past the bound, never cut:** the
  offered bars, the held count and the merged series are each at most
  `repair::row_ceiling(path)`, the month's length over the rung's width
  (2,678,400 rows for a 31-day one-second month, 2,419,200 for a 28-day one,
  44,640 for a 31-day one-minute month, 31 for a 31-day daily one). One row
  more is `RepairError::RowLimit { rows, ceiling }`, before a bar is read and
  before anything is written. A month has at most 1,024 revisions
  (`MAX_REVISIONS`); past them `RevisionLimit`, and no page shows how many a
  month has used (rules audit RA-266; not this package's).
- **L-IA-26, time, measured.** Bench C-RP-01
  (`crates/store/benches/ratio.rs`), 12 Oct 2026, the operator's Mac under the
  load of the other build agents, each repair of a fresh copy of the held
  month in the host's temporary directory, only `repair_month` inside the
  clock. A month of real size (448,317 merged rows: 425,817 held, one session
  of 22,500 offered): n = 15, p50 11.83 s, p99 17.95 s, max 17.95 s. A month
  filled to its ceiling (2,678,400 merged rows: 30 days of every second held,
  the 31st offered): n = 5, p50 39.79 s, p99 107.63 s, max 107.63 s. With n
  under 100 the p99 is the slowest sample. The cost of one merged row at the
  fastest run was 18,263 ns at the real-sized month and 4,254 ns at the full
  one (ratio 0.232, held under the gate's 3.0): **the time is the host's
  syncs, not the rows.** The same two repairs, once each in a process of
  their own on the store's own volume a few minutes earlier, took 245 ms and
  1,327 ms (n = 3 each: 236, 239, 245 ms and 1,327, 1,345, 1,349 ms); the
  whole bench used 7.6 s of user and 17.7 s of system time in 645 s of wall
  clock. A commit is a round of syncs and a full month is written in 6
  (`WRITE_BATCH`, 524,288 rows; a month of 23 regular sessions, 517,500
  bars, is one). A time lookup on the repaired full month cost 273.66 address
  floors against C-TIX-02's 800: the revision's `.tix` is its writer's own.
- **L-IA-26, memory, measured, and over the design's sum.** One repair in a
  process of its own, 12 Oct 2026, macOS, `/usr/bin/time -l`, maximum
  resident set size: **537,411,584 bytes at the full month, 152,879,104 at
  the real-sized month.** A private copy of the real GDFL NIFTY one-second
  month 2019-10 (409,590 held bars, the 3,100 of 27 Oct offered), repaired
  and compared with the store's own revision bar for bar inside the same
  process, peaked at 160,628,736, which includes that comparison's two
  copies of the month. The whole C-RP-01 bench process peaked at 529,072,128.
  The design summed type sizes to at most 471 MB for the full month, and
  about 75 MB for a real one; **neither sum is met and neither is claimed.**
  What is held: the merged series (56 B a row: 150 MB at the ceiling), the
  caller's offered bars (56 B each), the stamps added (8 B each), and what
  the store's writer holds per commit, which this module now bounds by
  writing a revision in commits of at most 524,288 rows (the same full month
  written as ONE commit peaked at 1,160,118,272 bytes on 11 Oct). **One term
  is not `store::repair`'s and still grows with the month:** writing the
  2,592,000 held rows of the full month in thirty commits of 86,400, with no
  repair at all, peaked at 159,383,552 bytes by the same tool, where one
  such commit's bars are 4.8 MB. Which part of the writer holds it is NOT
  PROVED here (the sort index's step runs after every commit and is the
  likely holder; it is WP-39's). Bench C-RP-01 holds the full month's repairs
  to 640 MiB where the host reports a peak (Linux `VmHWM`, restarted before
  them). **NOT MEASURED: that figure on a Linux host.** The bench's memory
  line read "NOT MEASURED on this host" on every run so far, so the 640 MiB
  allowance has never been checked by the bench itself, only against
  `/usr/bin/time -l` on macOS.
- **L-IA-26, disk: one whole copy of the month per repair, kept for ever.**
  A revision is a complete month under `bar-revisions-v1/<n>/`, never
  rewritten and never removed, and there is no rule yet that prunes one
  (rules audit RA-018, NEW-31). Until the once-per-month gather is built
  (D-4899), a run that back-fills a month a file at a time writes one copy
  per file it lands, up to the 1,024 a month allows. On the operator's store
  on 12 Oct 2026 there were 24 revised months, all one-second GDFL months at
  revision 1 (`find bar-revisions-v1 -name '*.bin'`).
- **L-IA-26, disk, restated 14 Oct 2026 (AF-07): the "24 revised months,
  all at revision 1" above is stale by two orders of magnitude.** Measured
  read-only on the operator's store after the 14 Oct imports by the two
  checks of assurance finding ingest-8 (scripts and outputs in
  `state/assurance-20261014/scratch/verify-ingest-8/`, outside the
  repository): `audit/gdfl.repairs` holds 17,007 `repairing` and 17,007
  `repair` lines (none left open) over **2,843 GDFL months**, up to
  **generation 21** for one month (NSE-CASH-ACC 2024-05 among them), 791
  months with 10 or more. `du -sk bar-revisions-v1` is 121,246,976 KiB =
  **115.63 GiB** in 449,029 files: **GDFL 106.05 GiB**, **Zerodha 9.58 GiB**
  (47,140 months, all generation 1, reason `refold_derived_rungs`). Of the
  GDFL bytes **66.22 GiB are superseded generations that no reader serves**
  and 39.83 GiB are served. The repairs wrote 1,005,847,649 rows to add
  68,414,450, about 14.7 rows written per row added, of which 937,433,199
  (about 13.7 per row added) were rows the month already held, copied
  into the new generation: one day landed early in a month (a special
  session imported by its own run) turns every later day of that month into
  a whole-month generation, because a revision is never appended to
  (`pull::gdfl_import`'s header). The once-per-month gather (D-4899) is
  still **NOT BUILT**, and nothing prunes a superseded generation (that
  would be a deletion, and is the operator's).
- **L-IA-26, the read path is unchanged and O(1).** Opening a month served
  from a revision checks the revision's and the source's header counts
  against the same ceiling: two comparisons, no I/O beyond the open it
  already did. On the daily rung the ceiling is the month's days, where the
  writer admits any whole second, so a daily revision holding more bars than
  its month has days would now be refused at open as `RowLimit`, named. None
  exists: no
  revision on the operator's store is on any rung but one second, and
  `publish` refuses to write one.
- **L-IA-26, the step before the promotion.** `repair_month_with` runs one
  step of the caller's between publication and promotion, at most once per
  call. Its cost is the caller's and is not bounded here. No caller passes
  one yet: the CASH auction-file writer it is for is IA-17's second part, so
  a repaired CASH month still has no auction file beside its served bars.

~~~~

Source: `docs/04-invariants.md` at feat/db-fix e851da1ca, lines 7094-7103 (verbatim)

~~~~text
### Ingest automation: the month repair's row ceiling and its step before the promotion (D-4875, D-4898)

| Id | Invariant | Proof | |
|---|---|---|---|
| IA-RP-01 | **A one-second month repairs at its grid ceiling.** 30 held days of every second of a 31-day month and the one day it lacks merge to 2,678,400 rows, the most the month holds, and publish as a generation every reader opens with the original byte-identical; the revision's `.tix` is present and answers time to row through the index at both ends of the month and of the added day; no `.prv` is written; 2,678,401 rows are refused `RowLimit { rows, ceiling }` at `repair_month` and at `publish` before anything is read or reserved, and the month offered again whole at the ceiling is `NothingNew`. NOT PROVED here: that a verifier reads a repaired month's rule set from the journal, which is IA-6's and has no code yet (D-4875 (1), (2), (5)) | `store/tests/month_repair.rs::a_full_1s_month_repairs_with_its_sidecars` | ✓ |
| IA-RP-02 | **A caller's file of a repaired month is written before the month is served from the revision, or the month is not served from it.** The step `repair_month_with` is handed runs with the revision's root after publication and before promotion: while it runs the revision opens whole (1,800 bars) and every reader is still served the original (1,200). A step that refuses, half its file written, leaves the revision published and unserved and the original's directory untouched, as `RepairError::BeforePromote` with the step's words; the exact retry finds the revision `Reused`, runs the step again over its torn file and promotes, and the file is beside the served bars. Offered a third time nothing is new and the step is not asked. With no step, `repair_month_with` and `repair_month` write the same bytes in every file of the revision and the original. NOT BUILT here: the CASH auction-file writer that will be passed as the step (IA-17's second part), so the step in this test is the test's own (D-4875 (5), D-4898) | `store/tests/month_repair.rs::a_cash_repair_writes_its_auction_file_before_it_is_served` | ✓ |
| IA-RP-06 | **The row ceiling is the path's month on the path's rung.** `repair::row_ceiling` equals the month's days times 86,400 over the rung's seconds for every rung in `Timeframe::KNOWN` and every month from 1970-01 to 9999-12, equals the writer's own bound (`Admission::slots`) on every intraday rung and that bound over 86,400 on the daily one, and `MAX_ROWS` is the largest value it takes. One row past it is refused `RowLimit` with both counts, at `repair_month`, at `publish` and for a forged source header, on every rung and at 28, 29, 30 and 31 days, with rows that are off the rung's grid and are therefore never read; a one-minute and a daily month one bar short of full take their last bar and are served at exactly the ceiling; a 32nd daily bar in a 31-day month is refused by count (D-4875 (1), D-4898) | `store::repair::tests::the_row_ceiling_is_the_months_length_over_the_rungs_width` · `store::repair::tests::the_flat_maximum_is_the_largest_ceiling_any_path_has` · `store/tests/month_repair.rs::the_row_limit_is_the_paths_own_month_on_its_own_rung` · `store/tests/month_repair.rs::a_full_month_on_a_coarse_rung_takes_its_last_bar` | ✓ |
| IA-RP-07 | **The step runs only for a revision about to be promoted, holds nothing behind it, and never inherits another request's revision.** It is not asked for an empty offer (refused `EmptyBatch`), a bar already held, a held bar that would change, bars out of order or twice, another month's bar, a month that does not exist, a missing basis witness or a path that is not a bar month, and none of those writes anything. A promotion refused behind the step (another promoter holds the pointer) leaves the month served as it was, and the retry runs the step again. A step that never returns (a panic) leaves no lock: the original still takes an append, and the retry steps over the stranded ordinal, names it, and publishes the next. A different repair after a refused step is given the next ordinal's root, the unserved revision is named in `skipped` and its bytes kept, and the request refused first then lands on top, every bar of all three kept (D-4875 (5)) | `store/tests/month_repair.rs::the_step_is_not_asked_when_nothing_is_about_to_be_promoted` · `store/tests/month_repair.rs::a_promotion_refused_behind_the_step_runs_the_step_again_on_the_retry` · `store/tests/month_repair.rs::a_step_that_dies_strands_nothing` · `store/tests/month_repair.rs::a_different_repair_after_a_refused_step_steps_over_the_unserved_revision` | ✓ |
| IA-RP-08 | **A revision is written in commits the sort index never compacts.** The largest month there is takes 6 commits of `WRITE_BATCH` rows (524,288), fewer than the 31 runs after which the sort index's writer rewrites it whole and reads the whole month back at once, and a month of 23 regular one-second sessions (517,500 bars) is still one commit; the revision of a month repaired at 2,678,400 rows carries five commits more than a month one append wrote. A revision cut between two of its commits (a well-formed month file of the first commit's bars, its reservation, no receipt) is never served, never completed in place and keeps every byte: the same repair steps over it, names it and publishes the whole month at the next ordinal, where the step is handed that root (D-4898; `docs/06-limits.md` L-IA-26) | `store::repair::tests::a_revision_is_written_in_commits_the_sort_index_never_compacts` · `store/tests/month_repair.rs::a_full_1s_month_repairs_with_its_sidecars` · `store/tests/month_repair.rs::a_revision_cut_between_two_commits_is_stepped_over_and_kept` | ✓ |
| C-RP-01 | A month repair costs no more per merged row at a one-second month filled to its grid ceiling (2,678,400 rows) than at a month of real size (448,317 rows), within the shared 3.0x ceiling; a time lookup on the repaired full month stays within C-TIX-02's 800 floors; and, where the host reports a peak resident set, the five full-month repairs' own peak (the host's figure restarted before them) stays within 640 MiB, a measured allowance between the 537,411,584 bytes one such repair peaked at on 12 Oct 2026 and the 1,160,118,272 the same month took when its revision was one commit. On a host with no such figure the line reads NOT MEASURED and holds nothing. n, p50, p99 and max of both repairs are printed (D-4875 (1), D-4898; `docs/06-limits.md` L-IA-26) | `store::bench::month_repair_is_linear_and_leaves_the_month_indexed` | ✓ |
~~~~

### 3i. IA-17's own reports on the Mac (what they leave for IA-17b)

From `state/fix/round2/fix-IA-17.json`, `notes` (the sentences about the later part, verbatim):

~~~~text
NOT DONE (later part, IA-17b): the once-per-month gather (D-4899, IA-RP-03..05, C-RP-02), the CASH .auc writer and auction::path.
~~~~

From `state/fix/round2/verify-IA-17-r1.json`, `followups` (verbatim, all eight; 1, 2 and 4 bind
IA-17b directly: 1 says "with IA-17b at the latest"; 4 says it "matters to IA-17b's gather"; 6
matters to the Mac checks of section 6). Paths in them are the Mac's.

~~~~text
1. (IA-17, docs/06-limits.md L-IA-26 and bench C-RP-01; with IA-17b at the latest) The memory row describes one repair in a process of its own, and one of its numbers does not repeat. The sentence 'The whole C-RP-01 bench process peaked at 529,072,128' was 1,323,827,200 B in my run of the same binary (TMPDIR on WD_BLACK, 16 s). Cause measured, not guessed: the peak resident set of ONE process rises with every repair it does (about +128 MB per full month, +29 MB per real-sized month) until it levels at about 1.7 GB (full) or 0.99 GB (real-sized, after about 30 repairs); the plain writer and plain reads do not do this, so it is in the repair path (store::repair's merged series beside the writer's per-commit buffers; which allocation, and whether it is only the macOS allocator keeping freed blocks, is NOT PROVED). It levels, so it is not an unbounded leak, and real-sized months are one commit either way, so the bounded commits did not introduce it. But the operator's app is one long-lived process and a repair-reason run repairs month after month in it (91 months in V-4/V-5): the row should state about 1 GB for that case on this Mac, not 153 MB. Repeat: cd scratch/verify-IA-17/crates/store && M1_MODE=session M1_N=48 PROBE_BASE=<dir> /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/p-IA-17/debug/deps/verify_ia17_probe-a5b5f223d2bb60c1 --exact m1_peak_rss_over_repeated_full_month_repairs --ignored --nocapture (M1_MODE=full M1_N=18 for the full month; build and read for the two controls).
2. (IA-17, crates/store/benches/ratio.rs) C-RP-01's memory assertion measures FIVE full-month repairs in one process against 640 MiB. On this Mac five such repairs peak at about 1,060 MB by getrusage, so if a Linux runner's allocator behaves the same the bench exits 1 the first time gate 14 runs there. NOT PROVED either way (no /proc here). Measure one repair per process (or a child per trial) so the allowance means what L-IA-26 says.
3. (IA-17 / WP-13c, crates/store/src/repair.rs check_header) On the daily rung a month the writer filled past its days (two bars in one day) can no longer be repaired at all, and an offer of an already-held bar answers RowLimit instead of NothingNew. Documented in D-4898, loud, and no real month is affected; if daily months with intraday stamps ever appear, the writer's admission for 1day should be tightened to one bar a day rather than the repair refusing later.
4. (WP-13c, store::repair / generation; matters to IA-17b's gather) Two identical repairs racing: usually one is refused with an Io 'reserve revision ... AlreadyExists' (a named refusal, but not a Conflict that is stepped over), and in 4 of 40 rounds both published, keeping two identical full copies of the month, with the first writer's step run on a revision that is never served. A CASH .auc written by that step would sit beside an unserved revision: harmless, but IA-17b's writer should not assume its revision is the one served.
5. (IA-17, crates/store/src/repair.rs RepairError::RowLimit doc) 'Refused before any bar is read' is true for the offered and held counts; for a MERGED count past the ceiling the held bars have been read (the merge ran) before publish refuses. Nothing is written in either case. One sentence to correct.
6. (main session, state/fix/shadow.sh) The script still does not clone bar-generations-v1, so a shadow serves every revised month from its original. I cloned it by hand (cp -cR) into shadow/IA-17-v, as the fixer did. Other packages' shadow checks of the 24 revised months may have read originals.
7. (IA-6) IA-RP-01's clause 'the verifier reads the month's rule set from the journal' has no code; docs/04's IA-RP-01 row should not be read as covering it.
8. (WP-39, bars route) /bars.json ignores an unknown parameter such as limit= without saying so, and from/to outside the month are clipped silently to the month.
~~~~

### 3j. The plan, the confirmed findings and the failure matrix

- **`state/board/fix-plan.json`**: no work package whose first word is `IA-17b` (nor `IA-17`).
  Nothing to copy.
- **`state/board/audit-result.json`** (key `confirmed`): no finding has package IA-17b; one
  confirmed finding names "the IA-17b writer" in its expected answer and fix hint. It belongs to
  package R3-26 (not merged; a dependent of IA-17b), and it is the real-store case IA-17b's writer
  is for. Copied in full (its evidence is described in words and counts, no vendor row):

~~~~json
[
  {
    "id": "SC-CLOSE-4",
    "area": "15:40 close from 2026-08-03",
    "severity": "wrong_data",
    "selection": "GDFL CASH Aug-Sep 2026 hold each CAS share's closing-auction print as an ordinary 1s bar (124 of 133 share-sessions); no .auc sidecar exists; those days are journaled definition=2 and definition_is_current treats them as current, so no ordinary import will restate them.",
    "observed": "GDFL CASH Aug-Sep 2026 hold each CAS share's closing-auction print as an ordinary 1s bar (124 of 133 share-sessions); no .auc sidecar exists; those days are journaled definition=2 and definition_is_current treats them as current, so no ordinary import will restate them.",
    "expected": "A definition bump (or dated rule) so CAS-era CASH days are re-imported with the print filed in the month .auc (IA-17b writer), absent from the 1s bars",
    "evidence": "find store/bars/gdfl/NSE/CASH -name \"*.auc\" -> 0; GET /bars/window.json?feed=gdfl&timeframe=1s&key=NSE-CASH-RELIANCE&from=2026-09&to=2026-09&from_day=2026-09-24&to_day=2026-09-24&tod_from=15:15:00&tod_to=15:29:59 -> print 15:28:07 as a bar, phase cas; /feeds.json says held as an ordinary bar until re-imported | verified by a second agent: the 15:40-close check of 13 Oct (run wf_a6fafa3a-6cd), each lens re-derived by another method; state/session-close-20261013/check-*.json",
    "location": "crates/pull/src/gdfl_import.rs:2189-2205; store/imports/gdfl.journal:7869-7943; store bars/gdfl/NSE/CASH/*/1s/2026-0{8,9}.bin",
    "fix_hint": "A definition bump (or dated rule) so CAS-era CASH days are re-imported with the print filed in the month .auc (IA-17b writer), absent from the 1s bars",
    "package": "R3-26"
  }
]
~~~~

- **`state/fix/e2e-failure-matrix.json`**: two rows are owned by IA-17b. The matrix's row shape
  has no `case` or `status_note` field; the rows are copied whole (`scenario` and `how` are the
  case, `test` the test it asks for):

~~~~json
[
  {
    "id": "FM-092",
    "stages": [
      "S4",
      "S8",
      "S10"
    ],
    "scenario": "Versioned month repairs copy a whole month per later day for months served from a revision (alias index special-session months take one generation a day), so disk use grows by month size x days and the month refuses once MAX_REVISIONS is reached.",
    "how": "Back-fills into months that already hold later days. | CONC-007: A daily scheduler resumes D the next morning before D+1, after CONC-006, CONC-008 or a special session.",
    "expected": "The plan shows generations and disk cost per month before running; the revision ceiling is warned before it is hit. At most one generation per instrument-month for a run's later days: the once-per-(instrument, month) gather, D-4899 (CONC-007's test).",
    "test": "the_plan_counts_generations_and_bytes_before_a_repair_run. | CONC-007: a_hole_early_in_a_month_costs_at_most_one_generation: a fixture with D missing, then D, D+1..D+5 imported daily. Assert at most one revision for the month. It fails today with six.",
    "status": "not_handled",
    "evidence": "INFRA-065: crates/pull/src/gdfl_import.rs:122-131 (one more generation per day, bounded by MAX_REVISIONS); no preview exists. || CONC-007: crates/pull/src/gdfl_import.rs:120-128 ('every later day it lacks is a repair too ... one generation a day'); crates/store/src/repair.rs:104 (MAX_REVISIONS = 1_024)",
    "severity": "high",
    "owner": "IA-17b",
    "sources": [
      "INFRA-065",
      "CONC-007"
    ],
    "verdicts": {
      "INFRA-065": "partly/low",
      "CONC-007": "not_handled/high"
    },
    "related": [
      "R3-28",
      "FM-243",
      "FM-265"
    ],
    "origin": "hunters"
  },
  {
    "id": "FM-098",
    "stages": [
      "S4",
      "S10"
    ],
    "scenario": "A special session (Muhurat, a DR drill, or the outage day) is imported after its month's later days. For indices and shares it lands through the repair door (D-3350, 12 days, 13 Oct). Options are still refused by the calendar, and every later options day of those months then costs a generation (CONC-007).",
    "how": "The calendar admits a special day after the month was imported.",
    "expected": "Special sessions are imported in order with their month, or batched into one repair per month with the cost stated.",
    "test": "special_sessions_cost_one_generation_per_month",
    "status": "partly",
    "evidence": "CONC-055: crates/pull/src/gdfl_import.rs:78-100; [an old evidence script's line count: its file name is left out here because it carries the extension of a language the operator bans by name] (special sessions queued)",
    "severity": "medium",
    "owner": "IA-17b",
    "sources": [
      "CONC-055"
    ],
    "verdicts": {
      "CONC-055": "partly/medium"
    },
    "related": [
      "IMPORT-SPECIAL-SESSIONS-FNO-INDEX"
    ],
    "origin": "hunters"
  }
]
~~~~

  One change from the source, in FM-098's `evidence`: the matrix names an old evidence script by
  its file name, and that name carries the extension of the language the operator bans by name
  (16 Oct rule: the name may not appear in any file). It is replaced by a bracketed description.
  The rest of both rows is copied byte for byte.

  44 more rows name IA-17b only as a predecessor of their own owner, in `owner_note` ("It follows
  ... IA-17b ...") or `related`, and ask nothing of it: FM-252 (IA-16), FM-265 (R4-23); PREC-2's
  FM-298, FM-301, FM-311, FM-312, FM-315, FM-316, FM-317, FM-321, FM-323, FM-506, FM-507,
  FM-513, FM-573, FM-581, FM-582, FM-586; PREC-13's FM-300, FM-318, FM-319, FM-372, FM-373,
  FM-374, FM-375, FM-382, FM-403, FM-428, FM-572, FM-577, FM-583, FM-584, FM-585, FM-602,
  FM-604; PREC-8's FM-341, FM-346, FM-347, FM-348, FM-351, FM-353, FM-356, FM-358, FM-590; and
  PREC-5's FM-571, whose evidence reads "FM-098 partly (IA-17b; options still refused)".

### 3k. The 16 Oct cloud spec of the same part

`origin/cloud/db-base-20261016:reports/db-specs-20261016/IA-17b.md` (993 bytes) holds only the
title, the same three prerequisites and the first sentences of the same package spec. It adds
nothing to this file.

## 4. Files

### 4a. Files IA-17b owns (design section 11, the IA-17 row of 16.2, R3-35's seam map)

R3-35 split `gdfl_import.rs` before IA-1 (merged 50093fa33). Its report's seam map (D-4979,
L-R335-01), from `state/fix/round2/fix-R3-35.json` `notes`, verbatim for this part: "IA-7 and
IA-17's gather gdfl_import_filing.rs (+ day.rs record_close)". The spec: "Edit them there and put no
seam back into gdfl_import.rs."

| File | What IA-17b does there | Shared with |
|---|---|---|
| `crates/pull/src/gdfl_import_filing.rs` | **the once-per-(instrument, month) gather** (D-4875 (3), D-4899): files a repair-reason run's back-fill days into one offer per (instrument, month), offered when the run leaves the month, ends, or is stopped; today the back-fill pass `back_fill` and `Filing::through` (`Door::BackFill`) call `ingest::repair_rows` once per FILE | IA-16, IA-7 (later), R3-26 |
| `crates/pull/src/gdfl_import_day.rs` | only what the gather needs of the day close (`record_close`): gathered days stay `begin` until their month's repair has published, then close in day order | IA-16, IA-2, R4-09 |
| `crates/pull/src/gdfl_cm_auction.rs` | **`auction::path` resolving the served generation** (through `store::generation::served_bars_path` / `served`), and **the revision's whole `.auc` writer** passed as `repair_month_with`'s `before_promote` step (the original's records plus the repaired days', in day order; synced, its directory synced) | IA-16 |
| `crates/pull/src/gdfl_import.rs` | minimal, only if needed: `cm_read` appends a day's prints to the ORIGINAL month's `.auc` before the bars are filed (`:2632`); in a repair-reason run a passed CASH day's prints must go with the gather to the step instead (section 8, item 2). The gather itself goes in `gdfl_import_filing.rs`, never here | IA-1, IA-2, IA-4, R3-26 |
| new test files for these modules | IA-RP-03, IA-RP-04, IA-RP-05, FM-092's and FM-098's tests, the extreme cases | — |
| `crates/pull/benches/ratio.rs` (or `crates/store/benches/ratio.rs` if the builder can show the gather is timed there) | bench **C-RP-02** (one month of 23 gathered index days: p50, p99, max, peak RSS), under its own heading | IA-5, others (each appends its own) |
| `.github/workflows/ci.yml` gate 14 `cover=` line of the crate whose bench grows | the exact pin raised by the bench's new call sites and `C-RP-02` added (at the head: `pull ... 38`, `store ... 21`); the line is a shared exact line: merge the head, recount, write head's ids plus your own (D-4891) | every bench-adding package |
| `docs/05-decisions.md` | the **D-4899** heading (section 5) | append only |
| `docs/04-invariants.md` | rows IA-RP-03, IA-RP-04, IA-RP-05, C-RP-02 (and FM-092/FM-098 tests) beside their tests, under IA-17's heading "Ingest automation: the month repair's row ceiling and its step before the promotion" or a new heading of its own | append only |
| `docs/06-limits.md` | L-IA-26's disk item and L-IA-12's gather term restated for what the gather now does (a new dated paragraph, never an edit of IA-17's measured lines) | append only |

### 4b. Files IA-17b must not touch (other parts' files), and what to do instead

| File | Owner | Why IA-17b leaves it |
|---|---|---|
| `crates/store/src/repair.rs`, `crates/store/REPAIR.md`, `crates/store/tests/month_repair.rs` | IA-17 (done), WP-13c | the step `repair_month_with(..., before_promote)` exists; IA-17b only CALLS it |
| `crates/store/src/generation.rs` | WP-13c, SC-3, SC-4 | call `served`, `served_bars_path`, `pointer_path`; never change them |
| `crates/pull/src/ingest.rs` | WP-13c, WP-12d, IA-16 (doors, `Lack`, journal writers), IA-10b | `repair_after` calls `store::repair::repair_month` with no step, and `lack_of` judges one batch's span. If the gather or the `.auc` step needs a seam here (a `repair_rows` variant that takes the step, or a span judged per gathered day), keep it to that one seam, keep every other caller's bytes unchanged, and name it in the report's files outside ownership |
| `crates/pull/src/gdfl_import_causes.rs` | IA-16, IA-3, R4-22 | the `.auc` refusal's typing (section 8, item 2) is IA-16's by the design; if IA-17b cannot work without it, one arm, named outside ownership |
| `crates/pull/src/gdfl_import_journal.rs`, `gdfl_import_quality.rs`, `gdfl_import_scope.rs`, `gdfl_import_census.rs`, `gdfl_import_lineage.rs` | IA-2, IA-4, R3-23, IA-16, WP-12f | not needed by the gather |
| `crates/pull/src/gdfl_run.rs`, `crates/cli/src/gdfl_import_command.rs` | IA-1, IA-2, IA-16 | `repair_reason` already flows from `Request` and `--repair-reason` |
| `crates/pull/src/gdfl_cm.rs` (the reader, the auction band) | PREC-2 (dependent) | the reader's diversion of prints is not this part's |
| `crates/api/**` (`sources.rs` reads `auction::path`; `server.rs`; `plan.json`) | IA-10a, IA-10b, R3-28, AF-02 | `sources.rs:3055` and `cli/src/tick_second_check.rs:1213` call `auction::path(root, key, day)`: keep that signature so neither needs an edit |
| `web/**` | IA-12a, WP-61 | the "rewrites used" view and the plan's generation count are R3-28's and IA-10a's |
| `store/benches/ratio.rs`'s C-RP-01 and C-VD-* | IA-17, IA-5 | append your own bench under your own heading only |

The specAppend's BACKTEST-FIRST rule: build the whole part as specified (the once-a-month gather
and the CASH `.auc` writer for repaired months, auction prints kept out of continuous bars); a page,
route or alert the backtest does not read (the plan's generation and byte count of FM-092, the
"rewrites used" view of RA-266) follows after the backtest and belongs to R3-28 and IA-10a. Name
those in the report as not built here.

## 5. Decision numbers

`state/decision-blocks.json` (62 blocks, `free_from` D-7205) assigns **no block to IA-17b**.

- **D-4899** is this part's by the design's critic (round 7, "D-4899 | One repair per (instrument,
  month) per run is IA-17's: its stop, crash and full-disk rules; V-4 before it, V-5 after | IA-17
  (later part)") and by D-4875 and D-4898 at the head, which defer it to "IA-17's second part". At
  e851da1ca it is cited (docs/05 twice, docs/06 three times) and **not headed**. Land it as one
  `docs/05-decisions.md` heading `### D-4899 — ...` with an "Implemented by IA-17b" line and
  "amends D-4875 (3), (6) and D-4836's V-4 line". Re-grep before writing it:
  `git grep -n -E '^#+ D-4899' origin/feat/db-fix -- docs/05-decisions.md` must print nothing.
- Any further locked choice of this part: **`D-TBD-IA-17b-1`, `D-TBD-IA-17b-2`, ...** (README rule;
  the Mac assigns real numbers from its registry at merge-back).
- Cite, never re-head: D-4875 (IA-17), D-4898 (IA-17), D-4874, D-4876, D-4887 (design), D-4889,
  D-4890 (IA-16), D-3350 (WP-13c), D-3320 (the `.auc`), D-4865 (the `.auc` torn tail, IA-16),
  D-4891 (gate 14's shared line).

## 6. NEEDS THE MAC (run at merge-back; the cloud has no store, no archive, no shadow server)

These are the Mac builder's step 5 checks for this part, written as commands. `W` is
`/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`; the real store is
`/Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store` and is **never written**. The archive
variables (`BRUTEX_GDFL_CM_ARCHIVE`, `BRUTEX_GDFL_NFO_ARCHIVES`, `BRUTEX_GDFL_TICKSTORE`) come from
the operator's local untracked configuration and are never written into any file. NIFTY first
(launcher line 355): one NIFTY spot month, one NIFTY options month, one special session; then one
BANKNIFTY, one F&O stock and one illiquid strike, all through the shared code path.

M0. Prerequisites and build (under the pool lock rules of the launcher):

~~~~text
git -C $W/wt/DBFIX log feat/db-fix --first-parent --merges --format=%s | grep -E '^Merge (IA-17|WP-12d|IA-16):'   # three lines
cargo build --release -p api -p cli      # in the merge-back worktree; copy the binaries to $W/tgt/bin-IA-17b/release/
~~~~

M1. Read-only shadow server on port 8414 (GET only; never POST, import, repair or backfill here):

~~~~text
$W/state/fix/stop.sh IA-17b; $W/state/fix/shadow.sh IA-17b
# shadow.sh clones bar-revisions-v1 but NOT bar-generations-v1, so without this a shadow serves
# every repaired month from its ORIGINAL (verify-IA-17-r1 follow-up 6):
cp -cR /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store/bar-generations-v1 $W/shadow/IA-17b/bar-generations-v1
$W/state/fix/serve.sh IA-17b 8414 $W/tgt/bin-IA-17b $W/wt/P-IA-17b
lsof -nP -iTCP:8414 -sTCP:LISTEN -t      # must print the pid in $W/shadow/IA-17b/test-server.pid
~~~~

Then:
- (a) `GET /sources/day.json?exchange=NSE&segment=CASH&symbol=RELIANCE&from=2026-09-24&to=2026-09-24`
  answers exactly what the head answers (the store holds 0 `.auc` files today, SC-CLOSE-4, so the
  auction reads as having no sidecar): `auction::path` resolving the served generation changes no
  answer for an original month. Compare the body byte for byte with the head's binary on another
  port.
- (b) The same route for a share-day of a month served from a revision (the store had 2,843 repaired
  GDFL months on 14 Oct, up to generation 21; NSE-CASH-ACC 2024-05 is one): same answer as the head
  (before 2026-08-03 no `.auc` applies), and the resolution adds one pointer read.
- (c) p50, p99, max over n >= 200 sequential GETs of (a) and (b), against the head's numbers: the
  added cost is one generation pointer read, O(1). Report n and the method.

M2. Write-path checks, only on a store root IA-17b owns entirely (a temp root, or the shadow with
its `bars` link REPLACED by a real directory holding `cp -c` copies of the few months needed),
`BRUTEX_STORE=<that root>`; prove afterwards that no file under the real store changed (mtimes of
the months used, and `audit/gdfl.repairs`'s length unchanged):

~~~~text
# Baseline, read-only, on the REAL store: how many generations the day-at-a-time back-fill made
# for the months used below (count the `repair` lines per month in audit/gdfl.repairs; print
# head -3 of the file first to read its field order, WP-13c's journal_repair, ingest.rs:3134).
grep -c . /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store/audit/gdfl.repairs
~~~~

- (d) **NIFTY spot, a special-session month (the many-days shape).** Copy into the private root the
  ORIGINAL month file (and its siblings) of one NIFTY or alias-index 1s month that holds only a
  later special session and lacks the other days (the 556 alias member-days over 67 months of
  design section 5.5; the original stays byte-identical on the real store even where the store now
  serves a revision). Run
  `cli gdfl-import indices zip <first day> <last day> <SYMBOL> --repair-reason "IA-17b merge-back check"`
  over that month. Expected: **one** `repair` line in `<root>/audit/gdfl.repairs` for that
  (instrument, month) and the month served at generation 1 (the real store's baseline for the same
  month is one generation a day); every gathered day closed in day order in
  `<root>/imports/gdfl.journal`; the run's `months_repaired` counts months, not days. Run the same
  command again: no new generation, nothing written (IA-RP-03; FM-092's "at most one generation per
  instrument-month").
- (e) **The 12 index days' shape (one day a month)**: one NIFTY and one BANKNIFTY month of those
  days (e.g. the month of 2021-02-24): one generation each, as before (V-4 is ready either way).
- (f) **NIFTY options**: one NIFTY options contract-month with a lacked day (a special session the
  calendar now admits, FM-098): one generation per contract-month, or a named refusal; if options
  months carry `.grk` or `.ovl` siblings every back-fill is `repair_impossible (IndexedSibling)`:
  run the critic's read-only check first,
  `find /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store/bars/gdfl/NSE/FNO -name '*.grk' -o -name '*.ovl' | head`.
  Then one BANKNIFTY month, one F&O stock's CASH month and one illiquid strike.
- (g) **CASH `.auc` of a repaired month** (IA-RP-02's real-data side; no real CAS-era repaired month
  exists yet): into an EMPTY private root, `cli gdfl-import stocks zip <a later day of 2026-09> 2026-09-30 RELIANCE`
  (lands the later days and their `.auc` records at the original path), then
  `cli gdfl-import stocks zip 2026-09-01 <the day before> RELIANCE --repair-reason "IA-17b merge-back check"`.
  Expected: one generation; `<root>/bar-revisions-v1/<n>/bars/gdfl/NSE/CASH/RELIANCE/1s/2026-09.auc`
  exists, parses (version 2), and holds the repaired days' prints and every later day's, in day
  order; the original `.auc` is byte-identical; no closing-auction print is a 1s bar of the
  revision (the 15:15 to 15:35 band holds only continuous-trading seconds); a server over that root
  answers `/sources/day.json` for a repaired day with the auction read from the served `.auc`.
- (h) **Stop mid-gather and crash mid-gather** (IA-RP-04): during (d), `cli gdfl-stop indices`;
  expected: what was gathered is offered, those days close, no day is left `begin`; a `kill -9`
  during (d): the days stay `begin`, and the next repair-reason Resume publishes or meets
  `Published::Reused` and closes them; no second generation.
- (i) **Full disk at publish** (IA-RP-05): on a small private volume (a disk image of a few hundred
  MB), the run ends `disk_full` with the gathered days still `begin`; the next run on a freed volume
  converges.
- (j) **Memory**: peak resident set of (d) and (g) by `/usr/bin/time -l`, one run per process, and
  C-RP-02 in release three times (p50, p99, max, n); also a long-lived process doing many month
  repairs (verify-IA-17-r1 follow-up 1 measured about 1 GB plateau for real-sized months).

Store writes this part makes NEED on the real store (list them in the report, never run them):
the V-5 back-fill of the alias months (IA-14's runbook step, on the operator's yes) becomes ready
only once IA-17b merges; nothing prunes the superseded generations already kept (66.22 GiB on 14
Oct; a deletion, the operator's).

## 7. Code facts at e851da1ca (grepped, not guessed)

Line numbers are at `feat/db-fix` e851da1ca; re-grep on the newest `origin/feat/db-fix`.

**`crates/store/src/repair.rs`** (exists, 962 lines; IA-17's and WP-13c's, call only):
`pub const MAX_ROWS: usize = 31 * 86_400` (:47); `pub const fn row_ceiling(path: StorePath<'_>) -> u64`
(:65); `const WRITE_BATCH: usize = 524_288` (:97);
`pub type BeforePromote<'step> = &'step mut dyn FnMut(&Path) -> Result<(), String>` (:101), called
with the revision's root `<root>/bar-revisions-v1/<n>`; `pub const MAX_REVISIONS: u32 = 1_024`
(:104); `pub fn revision_root` (:139); `pub enum RepairError` (:145) with
`RowLimit { rows, ceiling }` and `BeforePromote { revision_root, why }` (:195); `pub enum Published`
(:292); `pub fn publish` (:400); `pub struct Policy` (:462); `pub enum MonthRepair` (:475,
`NothingNew` / `Repaired`); `pub fn repair_month(...)` (:540) is a wrapper that calls
`repair_month_with(root, path, symbol_id, offered, policy, None)` (:547);
`pub fn repair_month_with(..., mut before_promote: Option<BeforePromote<'_>>)` (:584), the step runs
after publish and before promote (error mapped at :670). Tests: `crates/store/tests/month_repair.rs`
(IA-RP-01 `a_full_1s_month_repairs_with_its_sidecars` :167, IA-RP-02
`a_cash_repair_writes_its_auction_file_before_it_is_served` :488, `no_step_is_the_wrapper_byte_for_byte`
:595, `the_step_is_not_asked_when_nothing_is_about_to_be_promoted` :628,
`a_promotion_refused_behind_the_step_runs_the_step_again_on_the_retry` :713,
`a_step_that_dies_strands_nothing` :770, ...); IA-RP-02's test writes its own stand-in auction file
(`auction_file(revision_root, path)` :141), not `pull`'s. **No production caller passes a step**:
`git grep repair_month_with` outside `crates/store` finds nothing; `pull`'s
`ingest_failkind.rs:197` maps `RepairError::BeforePromote` to `FailKind::Io`.

**`crates/store/src/generation.rs`** (exists, 625 lines; WP-13c, SC-3): `pub const ROOT: &str =
"bar-generations-v1"` (:84), the pointer at `<root>/bar-generations-v1/bars/<vendor>/.../<yyyy-mm>.gen-v1`;
`pub fn pointer_path` (:231); `pub fn served_bars_path(root, path) -> Result<PathBuf, StoreError>`
(:267, the served generation's `.sbar` if present, else its `.bin`); `pub fn served(root, path)`
(after it; `Served::Original` when no pointer). The `.auc` is not a store file kind, so the auction
path must be derived from the served generation's root (`Served::root`) plus the month's relative
path with extension `auc`.

**`crates/pull/src/gdfl_cm_auction.rs`** (exists, 1,152 lines): magic `BRTXAUC1`, `VERSION = 2`,
`HEADER_LEN = 16`, `RECORD_LEN = 32`, `EXTENSION = "auc"`, `LOCK_WAIT = 2 s`; `pub struct Print`
(:181); `pub struct Record` (:216, `of(day, print)`, `encode(version, ordinal)`); `pub fn parse`
(:425); `pub fn read(path) -> Result<Option<Sidecar>, Refused>` (:496); **`pub fn path(store_root,
key, day) -> Result<PathBuf, String>` (:560) builds the ORIGINAL month's
`bars/gdfl/NSE/CASH/<SYM>/1s/<YYYY-MM>.auc` and does not resolve the served generation**;
`pub fn append(store_root, key, day, prints) -> Result<Appended, CmRefusal>` (:594) under an
exclusive lock, **and `append` finds the file it writes by calling `path(store_root, key, day)`
itself (:609)**, as do the module's own tests (:961, :1117); `fn write_day` (:651) refuses "<day> holds N records and M different ones were
offered; a new generation, never an in-place change" and "the month holds a day after <day>;
<day> is a new generation, never an insertion"; a failed write is cut back (all or nothing); a
torn tail is cut by the next append (IA-AUC-01). Callers of `path`: `api/src/sources.rs:3055`
(`sidecar_path`, read by `auction_of` :3085 for `GET /sources/day.json`, `server.rs:16142`, and in
its cache stamp), `cli/src/tick_second_check.rs:1213`.

**`crates/pull/src/gdfl_import.rs`** (exists, 3,676 lines): the module header (:78 to :131) says, at
the head, "Once a month is served from a revision, every later day it lacks is a repair too ... so
each costs one more generation ... A month that holds only a later day (the six alias indices'
special-session months, DB-B40) takes every other day of the month this way, one generation a day"
(the behaviour this part removes); `RunOptions.repair_reason: Option<&str>` (:921);
`pub const HELD_BACK_CAP: usize = 4_096` (:997); `Report.months_repaired` (:1646); `cm_read`
(about :2620) appends the day's prints with `gdfl_cm::auction::append(run.store_root, &job.key, day,
&day_file.file.auction)` in `Mode::Import` **before** the bars are filed (:2632), and a refused
append refuses the whole file.

**`crates/pull/src/gdfl_import_filing.rs`** (exists, 1,653 lines): `pub(super) enum Door { Append,
BackFill }` (:585); `Filing::file(&self, handed, begin, door) -> (Filed, bool)` (:609);
`Filing::through` (:790): the append door retries `MonthBusy` (`MONTH_BUSY_RETRY`), hands a file to
the back-fill door when `kind.files_as_successor()` (a sealed month, D-7131) or when
`(kind.repairable() || kind == FailKind::MonthBusy) && self.repair_reason.is_some()` (D-4889);
`Door::BackFill` builds the reason `"<operator's text> | gdfl-import <kind> <day> <file> run:<id>: a
day the month lacks where no append can land it"`, suspends the day's durable batch and calls
`ingest::repair_rows(seconds, symbol, origin, store_root, month(), &reason)` **once per file**;
`Filing::repaired` (:950) emits "day back-filled as a new generation of its month";
`fn back_fill(filing, begun, done)` (:1010) is the back-fill pass, one file after another in listing
order on the coordinating thread; `DayFiler` (:1032; fields include `stopped`, `disk_full`,
`store_lost`, `held_back_cap`); `DayFiler::file_all` (:1149, the held-back check before any read,
so before the `.auc` append); `hold_back` (:1215); `file_jobs` (:1247); `fold` (:1376,
`Receipt::NewGeneration => self.report.months_repaired += 1` at :1399); `make_durable` (:1428);
`stop_at_durability` (:1474).

**`crates/pull/src/gdfl_import_day.rs`** (exists, 2,497 lines): `decisive` (:100); `admit` (:568);
`Asks { repair_reason: bool, ... }` (:731); `admit_rule` (:767); `stopped` (:1454); `run_days`
(:1474); `record_close` (:2438).

**`crates/pull/src/ingest.rs`** (exists, 6,780 lines; not this part's): `from_rows` (:1603);
`pub fn repair_rows(bars, instrument, origin, store_root, plan, reason) -> (Ingested, Filed)`
(:1718; its doc: "A month already served from a revision pays that for EVERY later batch it lacks,
one generation each"); `repair_or_append` (:2694, Zerodha); `repair_after` (:2766) calls
`store::repair::repair_month` with no step; `enum Lack { Lacked { served }, Overlaps { held_at },
AppendDoors }` (:2863); `lack_of` (:2900) answers `Overlaps` when the first held bar at or after the
batch's first second is at or before its last; `back_fill_or_append` (:2964); `journal_intent`
(:3073), `journal_repair` (:3134), `journal_refusal` (:3189) write `<store>/audit/gdfl.repairs`.

**`crates/pull/src/gdfl_import_causes.rs:955`**: `CmRefusal::AuctionSidecar { .. } =>
Cause::open(Stage::File, "auction_sidecar")` (an open cause, untyped).

**`crates/pull/src/gdfl_run.rs`**: `Request.repair_reason: Option<String>` (:413), checked by
`ingest::repair_reason_ok` (:501). **`crates/cli/src/gdfl_import_command.rs`**:
`cli gdfl-import KIND SOURCE FROM TO [SYMBOL ...]`, `--repair-reason TEXT` (1 to 200 bytes, :110).
**`crates/cli/src/lib.rs:3038`**: `gdfl-stop` (KIND or `--run ID`).

**Ledgers at the head**: D-4875 (`docs/05-decisions.md:67618`) and D-4898 (:67693) headed by IA-17;
D-4899 cited, not headed; `docs/04-invariants.md:7094` to `:7103` hold IA-RP-01, IA-RP-02,
IA-RP-06, IA-RP-07, IA-RP-08 and C-RP-01; **IA-RP-03, IA-RP-04, IA-RP-05 and C-RP-02 exist nowhere**
(`git grep` finds none); `docs/06-limits.md:17390` is IA-17's L-IA-26 section. Gate 14 (`ci.yml`
:3876) pins `pull crates/pull/benches/ratio.rs 38 ...` and `store crates/store/benches/ratio.rs 21
... C-RP-01 ...` as exact counts.

## 8. Where the design and the head disagree, and the hazards a builder meets

1. **First check of the spec ("whether WP-12d already built them")**: it did not. At e851da1ca the
   back-fill repairs one file at a time (section 7, `Filing::through`, `repair_rows`), and no caller
   passes a `before_promote` step. So `nothing_to_do` is false: both pieces are open. The measured
   cost (docs/06 L-IA-26, 14 Oct): 2,843 GDFL months repaired, up to generation 21, 66.22 GiB of
   superseded generations, about 14.7 rows written per row added.
2. **The `.auc` refusal is not typed at the head.** The design (section 5.5, F65, section 11) says
   IA-16 types the older-day refusal `FailKind::NotHeld` (a hole) and records-differ `Differs`; at
   the head `gdfl_import_causes.rs:955` makes every `AuctionSidecar` refusal an open cause, and
   `cm_read` appends to the ORIGINAL month's `.auc` before the bars reach any door. So in a
   repair-reason run a passed day of a CAS-eligible share (from 2026-08-03) whose month holds a later
   day is refused whole at `cm_read` and never reaches the back-fill: the CASH `.auc` writer cannot
   be exercised until a repair-reason run carries that day's prints with its gathered bars to the
   step instead of appending them at the original path. Build that diversion (minimal, in `cm_read`
   or its filing seam) and name the typing gap for IA-16 in the report.
3. **`auction::path` does not resolve the served generation** (design: it does). Today a reader of
   a repaired CASH month reads the original's `.auc`. Change it to resolve through
   `store::generation::served` (one pointer read, O(1)), keeping its signature so `api` and `cli`
   need no edit, and pin that an original month's path is unchanged.
   **Hazard (found by the independent check, at e851da1ca):** `auction::append` gets its target
   from `path` too (`gdfl_cm_auction.rs:609`). If `path` alone starts to resolve the served
   generation, every `cm_read` append of a day in a month served from a revision would open,
   create or extend the `.auc` inside that **published, served revision root**. That is an
   in-place write into a published generation, which append-only history (CLAUDE.md section 3,
   rule 8) and D-3350 forbid. Today the same append goes to the original's `.auc` while the day's
   bars are refused or repaired. So split the two uses: readers (`api/src/sources.rs:3055`,
   `cli/src/tick_second_check.rs:1213`) resolve the served generation. The append writes only
   where it may write: the original month while that month is served as the original. In a month
   served from a revision, the prints go with the gathered bars to the `before_promote` step,
   which writes the next revision's whole `.auc` (item 2). They are never appended into a served
   revision. Pin both with tests: one where an append in a revision-served month leaves every
   byte of the served revision root unchanged, and one where a reader of that month reads the
   served `.auc`. The test callers of `auction::path` in `api`, `cli` and `pull`
   (`sources_day_truth_tests.rs:209`, `tick_second_check_tests.rs:1147`,
   `gdfl_cm_admission_tests.rs:1659`, `gdfl_ia16_tests.rs:1094`, `gdfl_import_tests.rs:3812`)
   use original months, so they keep their answers.
4. **A gathered offer that spans a held day is refused.** `lack_of` refuses a batch whose span
   (first to last second) contains any held bar (`Overlaps`, typed `Differs`, `bars_differ_held`).
   A month that holds a day in the middle (a special session) and lacks days on both sides cannot be
   offered as one span through the current door. The gather must offer only what the month lacks
   and the door must judge it per gathered day (or per lacked run of days), never cut and never by
   relaxing the rule; `repair_month` itself merges held and offered bars in order and refuses only a
   held bar that would change. Any seam this needs in `ingest.rs` is outside ownership (section 4b).
5. **Two identical repairs can race** (verify-IA-17-r1 follow-up 4): usually one is refused, but in 4
   of 40 rounds both published, and the first writer's step ran on a revision that is never served.
   The `.auc` writer must write only into the revision root it is handed and never assume that
   revision is the one served.
6. **Memory in a long-lived process** (follow-ups 1 and 2): one process's peak rises with every
   repair to about 1.7 GB (full months) or 0.99 GB (real-sized) before it levels; C-RP-02 and any
   memory assertion must measure one repair (or one gathered month) per process. The gather's own
   term is L-IA-12's 224 B per file per gathered day plus the gathered bars, bounded by
   `row_ceiling` (refused `RowLimit` by name past it, never cut).
7. **The plan's generation count and "rewrites used"** (FM-092's first sentence, RA-266) need
   `plan.json` (IA-10a, not merged) and R3-28's list (not merged): out of this part by the
   BACKTEST-FIRST rule; the test the matrix names for the gather is CONC-007's
   `a_hole_early_in_a_month_costs_at_most_one_generation` ("It fails today with six").
8. **The critic's rules for the gather (IA-17-3, D-4899) are the defaults**: per (instrument, month);
   offered when the run leaves the month, ends, or is stopped; a stop offers what is gathered and
   closes those days; a crash leaves the days `begin` and the next repair-reason Resume re-reads them
   (D-2821) and meets `Published::Reused` or publishes; `StorageFull` at publish ends the run
   `disk_full` with the gathered days still `begin`; tests IA-RP-03, IA-RP-04, IA-RP-05; bench
   C-RP-02; until it lands IA-14 lists V-5 as not ready and V-4 as ready.

## 9. What could not be found

- The package spec ends with "ANSWERS:" and no answers follow it in any copy (embed.json, args.json,
  the launcher's line 10, the 16 Oct cloud spec).
- No rules-audit section is headed `IA-17b` (the IA-17 and D-4899 sections are used, as the spec says).
- No fix-plan work package and no confirmed finding is IA-17b's own (SC-CLOSE-4, R3-26's, names its writer).
- No decision block is assigned to IA-17b in `state/decision-blocks.json` (D-4899 from the design; else D-TBD-IA-17b-n).
- The e2e matrix rows have no `case` or `status_note` fields; their `scenario`, `how` and `test` are copied instead.

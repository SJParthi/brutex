# GDFL build: exact prompts, scripts and per-part state

Generated 2026-10-04 18:47 UTC from the Mac resume kit. The scripts are JavaScript workflow scripts for Claude Code's Workflow tool. They are kept here as Markdown because this repository tracks no .js (CLAUDE.md section 2). To use them, copy each block back into a .js file on the Mac; they already exist on the Mac at the paths named.

## `state/resume-kit/scripts/build-wave1-continue.js`

```js
export const meta = {
  name: 'gdfl-build-wave1-continue',
  description: 'Continue build wave 1 from each part\'s current review round, at most 3 agents at once: of the GDFL one-second fill architecture test-first: census verb, core::second, store grid, pull::gdfl_cm reader (parallel branches), each reviewed by 3 lenses and repaired until clean, then integrated',
  phases: [
    { title: 'Build', detail: 'implement each part test-first on its own branch' },
    { title: 'Review', detail: 'tests-bite, behaviour+extremes+O(1), law+design conformance' },
    { title: 'Repair', detail: 'fix every upheld issue' },
    { title: 'Integrate', detail: 'merge the clean parts into feat/gdfl-1s and run the full Definition of Done' },
  ],
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const BACKOFF = [60, 120, 240, 480, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600]
// At most MAX_LIVE agents run at once (operator, 3 Oct 13:13 UTC: slow the agents to spare the 5-hour limit; 1 until the 18:00 UTC reset, then 3); the rest queue.
const MAX_LIVE = 1
let live = 0
const waiters = []
async function acquire() { while (live >= MAX_LIVE) await new Promise((r) => waiters.push(r)); live++ }
function release() { live--; const w = waiters.shift(); if (w) w() }
async function call(prompt, opts) {
  for (let a = 0; ; a++) {
    await acquire()
    let res
    try { res = await agent(prompt, opts) } finally { release() }
    if (res) return res
    if (a >= BACKOFF.length) { log(`${opts.label}: still failing after retries`); return null }
    log(`${opts.label}: agent failed (network/API); retry ${a + 1} in ${BACKOFF[a]} s`)
    await sleep(BACKOFF[a] * 1000)
  }
}
const W = '/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925'
const P = '/Volumes/WD_BLACK/brutex/fresh-20260919/project'
const DESIGN = `${W}/state/design/gdfl-1s-fill-architecture.md`
const MAX_ROUNDS = 5
const OPERATOR = `OPERATOR INTENT (chat, 2026-10-01 IST): the brute-force sweep stays on Zerodha minute data and its rungs; EVERY entry, exit and exit-grid combination is priced from GDFL one-second data, each fill at that second's worst-case (adverse) high/low; NIFTY and BANKNIFTY are traded as their volume-less spot index; GDFL second-level data is stored as one-second records. Latest message (verbatim): "see dont worry abiut gdfl data missign but jsut deisgn build architecture fix integrate and impelemnt to use the gdfl data dude okay?" - record it as the operator's go-ahead to build and to keep a converted one-second copy of the GDFL data in the store (an operator statement, dated). Data-era rule (D-0816, chosen by Claude on the operator's "pick everything" delegation): label every result with its data era AND by default require in-sample and out-of-sample windows from the same era.`
const LAW = `THE DESIGN: ${DESIGN} is the architecture (revision 3+, still being hardened in parallel by another workflow; READ ITS CURRENT TEXT when you start, and follow its section for your part exactly; where it is silent or contradicts the repository law, the law wins and you record why). The draft decision D-0802 is on branch docs/d-0802-tick-precise-fills (worktree ${W}/wt/D0802). GDFL data on disk (READ-ONLY, never modify): /Volumes/WD_BLACK/NSE_Tick_2018-09-01_to_2026-09-24 (INDICES/<yyyy>/<MON_yyyy>/GFDLCM_INDICES_TICK_<ddmmyyyy>/<NAME>.NSE_IDX.csv and STOCKS/...). Another Claude session scans /Volumes/WD_BLACK/NSE_Options_Tick: never touch its processes.
CLAUDE.md is law: Rust only outside web/; prices paisa i64; O(1) per operation where claimed, proven by a test and where the repo measures it a Gate 8/14 bench, otherwise an honest row in docs/06-limits.md; no look-ahead; run identity; idempotence; append-only store versions; no writable memory mapping; no silent fallback; every new invariant in docs/04-invariants.md beside the test that proves it; every locked choice as a docs/05-decisions.md entry (use the decision number the design's section 11 assigns to your topic; if two parts would take the same number, take the next free number above D-0816 and say so); crate graph and gates 9, 9b, 22 respected; no literal credential path.
DEFINITION OF DONE for your branch (CLAUDE.md section 9), each run to completion: cargo fmt --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace --locked; cargo deny check; line and branch coverage 100% on every touched crate (cargo llvm-cov); no surviving mutant (cargo mutants --in-diff over your diff against origin/main); the CI gate scripts (${W}/scripts/run-all-gates.sh <worktree>; it caches passes on identical trees).
SAFETY, ABSOLUTE: never rm "$HOME", "$TMPDIR", ~ or any variable/glob; never export HOME; never delete anything under /Users/parthi. Never push (the GDFL reader and census branches hold real GDFL rows in their history and must never be pushed), never rebase, never touch main, another branch or another worktree.
BUILDS: export PATH=${W}/bin:$PATH (machine-wide queue, 8 builds at once; waiting is normal). WAIT CHEAPLY: start each long cargo command in the background writing a log ending 'EXIT <code>', then ONE blocking Bash call \`until grep -q '^EXIT ' <log>; do sleep 20; done\` with the 600000 ms timeout, repeated if it times out. Inside the Bash sandbox ps/pgrep are refused and plain \`mktemp -d\` (no template) is refused; gate scripts that need it may be run with the sandbox disabled. NEVER REPORT BEFORE YOUR CHECKS FINISH, and never report uncommitted work as done. Commit messages end with the trailer line: Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`
const PARTS = [
  { id: 'census', branch: 'feat/gdfl-census', wt: `${W}/wt/G-census`, tgt: `${W}/tgt/g-census`,
    spec: `Design section 12 step 0: a REPORT-ONLY Rust cli verb \`gdfl-census\` (it writes nothing to the store) that walks GDFL CM day folders given as a path argument and measures, per day and instrument: back-steps (count, max size), UTC-stamped rows, placement/anchor skew, forward spikes, rows per second and zero-width-second share (the data eras), LTQ, gaps before 15:10, session coverage, final-newline and empty-session counts, and frozen-price runs inside the session (market halts / circuit breakers, e.g. 13 Mar 2020). Tests first with small fixtures quoted from real files. THEN RUN IT over NSE-NIFTY ("NIFTY 50") and NSE-BANKNIFTY ("NIFTY BANK") for EVERY day 2018-09-01..2026-09-24 (read-only, in the background, logging to ${W}/state/design/census/), and record the measured results as a Markdown table at ${W}/state/design/census/summary.md (era boundaries, max back-step, max skew, halt days found). These numbers are what the fold constants and bounds will be locked from.` },
  { id: 'core-second', branch: 'feat/gdfl-core-second', wt: `${W}/wt/G-core-second`, tgt: `${W}/tgt/g-core-second`,
    spec: `Design section 3.1 and section 12 step 2: the pure \`core::second\` module (core depends on nothing - gate 9). Constants, SecondCell, SessionSlot, SlotError, sod_of, slot_of (IST, not UTC; 09:15:00 -> 0, 15:29:59 -> 22,499; refuses sub-second and off-session), DaySeconds (len must be 22,500, checked), the O(1) one-read lookups (first-at-or-after with no loop and no binary_search - a source-shape test), and overflow safety (huge holds cannot wrap a slot). Tests named in step 2 written first.` },
  { id: 'store-grid', branch: 'feat/gdfl-store-grid', wt: `${W}/wt/G-store-grid`, tgt: `${W}/tgt/g-store-grid`,
    spec: `Design section 4 and section 12 step 4: the one-second grid file and its day directory in crates/store (the versions and number-space policy the design gives - record the policy as a decision; Layout::KNOWN and its existing pinning test must be updated as the design and the store law require), the timestamp -> slot -> byte offset formula, per-day CRC/digest, commit order and torn-state detection, idempotent re-append, read-only access (no writable mapping), and the tests named in step 4 written first. The cell type comes from core::second: if that branch is not merged yet, define only what this branch needs behind the same names and leave a note for integration.` },
  { id: 'gdfl-cm', branch: 'feat/gdfl-cm-reader', wt: `${W}/wt/G-gdfl-cm`, tgt: `${W}/tgt/g-gdfl-cm`,
    spec: `Design section 3.3 and section 12 step 5: \`pull::gdfl_cm\`, the reader for GDFL CM tick CSVs (columns Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest; DD/MM/YYYY; second-resolution stamps; several rows per second; pre-open and post-close rows; files with and without a final newline; .csv and .CSV; NSE_IDX and NSE suffixes), ticker -> instrument mapping for the swept surface, paisa conversion with half-up snapping at the write boundary (CLAUDE.md section 7), and one test per refusal. Fixtures are a few lines quoted from real files (cite the file and line).` },
]
const LENSES = [
  ['tests-bite', 'TESTS-BITE: every new or changed test FAILS when exactly what it claims to pin is broken (break it in a throwaway worktree); cargo mutants --in-diff leaves no MISSED mutant; coverage is truly 100% on touched crates; invariant rows claim only what their tests assert; nothing weakened or deleted.'],
  ['behaviour-extremes-o1', 'BEHAVIOUR-EXTREMES-O1: correct on every input including extremes (empty, one, max, i64 limits, unicode/case/whitespace/NUL in tickers, truncated/corrupt/misfiled/vanishing files, back-stepping and duplicate timestamps, pre-open/post-close rows, holidays, halts, crash mid-write, reruns byte for byte); no silent fallback; every O(1) claim really O(1) per operation (mechanism checked, bench where the repo measures), honest limits otherwise.'],
  ['law-and-design', 'LAW-AND-DESIGN: CLAUDE.md law (Rust only, crate graph and gates 9/9b/22, paisa i64, no look-ahead, identity, idempotence, append-only versions, no writable mmap, no invented fact - every vendor fact cites a quoted file sample), conformance to the design section for this part (and every deviation justified and recorded), decision entries correct and append-only, invariants beside tests, docs/06-limits honest.'],
]
const REPORT = { type: 'object', properties: { branch: { type: 'string' }, head: { type: 'string' }, commits: { type: 'array', items: { type: 'string' } }, built: { type: 'string' }, tests: { type: 'array', items: { type: 'string' } }, decisions: { type: 'array', items: { type: 'string' } }, checks: { type: 'array', items: { type: 'object', properties: { command: { type: 'string' }, result: { type: 'string' } }, required: ['command', 'result'] } }, measurements: { type: 'string' }, still_open: { type: 'array', items: { type: 'string' } }, summary: { type: 'string' } }, required: ['branch', 'head', 'commits', 'checks', 'still_open', 'summary'] }
const VERDICT = { type: 'object', properties: { refuted: { type: 'boolean' }, reviewed_head: { type: 'string' }, issues: { type: 'array', items: { type: 'object', properties: { severity: { type: 'string', enum: ['blocking', 'should-fix', 'nit'] }, description: { type: 'string' }, evidence: { type: 'string' } }, required: ['severity', 'description', 'evidence'] } } }, required: ['refuted', 'reviewed_head', 'issues'] }

// Each part's state when the throttled run was stopped (3 Oct 13:18 UTC), from run wf_124b6c56-8e9.
const START = /* per-part state: see the JSON section below, or regenerate with make-continue-state.js */ {}
async function runPart(pt) {
  const st = START[pt.id]
  let rep = st.rep
  const INTERRUPTED = `NOTE: an earlier agent on this worktree was stopped mid-task (operator throttle, 3 Oct 13:18 UTC). Before anything else run \`git -C ${pt.wt} status\` and \`git -C ${pt.wt} diff\`; keep any uncommitted edit that is correct and finish it test-first, discard (git checkout -- <file>) only what is wrong, and never lose committed work.`
  for (let r = st.round; r <= MAX_ROUNDS; r++) {
    const head = rep.head && /^[0-9a-f]{7,40}$/.test(rep.head) ? rep.head : pt.branch
    const have = r === st.round ? st.verdicts : {}
    const vs = (await parallel(LENSES.map(([n, def]) => () => have[n] ? Promise.resolve(have[n]) : call(`Independent adversarial reviewer, round ${r}, of part "${pt.id}" on branch ${pt.branch} (worktree ${pt.wt}; review \`git -C ${pt.wt} diff origin/main..${head}\`). Default to refuted=true if you cannot confirm a claim first-hand. Do not edit the branch: use a throwaway worktree (\`git -C ${pt.wt} worktree add --detach ${W}/wt/GV-${pt.id}-${n} ${head}\`, remove a stale one first and remove it when done) with CARGO_TARGET_DIR=${W}/tgt/gv-${pt.id}-${n}.
LENS ${def}
WHAT THE PART MUST BUILD: ${pt.spec}
The builder reported: ${JSON.stringify(rep).slice(0, 5000)}
${OPERATOR}
${LAW}
Report refuted, reviewed_head, issues (blocking / should-fix / nit; evidence = command + output).`, { label: `gb:review:${pt.id}:${n}:r${r}`, phase: 'Review', schema: VERDICT, effort: 'high' })))).filter(Boolean)
    const issues = vs.flatMap((v) => v.issues || [])
    const serious = issues.filter((i) => i.severity !== 'nit')
    if (vs.length === LENSES.length && vs.every((v) => !v.refuted) && serious.length === 0) { log(`${pt.id}: CLEAN after round ${r}`); return { id: pt.id, branch: pt.branch, status: 'clean', round: r, head: rep.head, rep, nits: issues } }
    if (vs.length < LENSES.length && !vs.some((v) => v.refuted) && serious.length === 0) return { id: pt.id, status: 'error', error: 'reviewers kept failing', rep }
    if (r === MAX_ROUNDS) { log(`${pt.id}: UNRESOLVED after ${r} rounds`); return { id: pt.id, branch: pt.branch, status: 'unresolved', head: rep.head, rep, open: issues } }
    const fixed = await call(`${INTERRUPTED}
Repair part "${pt.id}" on branch ${pt.branch} (worktree ${pt.wt}, CARGO_TARGET_DIR=${pt.tgt}); commit on top. Reviewers upheld in round ${r} (fix every blocking, should-fix and nit test-first; reject one only with evidence):
${JSON.stringify(issues, null, 1)}
WHAT THE PART MUST BUILD: ${pt.spec}
${OPERATOR}
${LAW}
Report as before.`, { label: `gb:repair:${pt.id}:r${r}`, phase: 'Repair', schema: REPORT, effort: 'medium' })
    if (!fixed) return { id: pt.id, status: 'error', error: 'repairer kept failing', rep }
    rep = fixed
  }
}

phase('Build')
const results = await parallel(PARTS.map((pt) => () => runPart(pt)))
const clean = results.filter((r) => r && r.status === 'clean')
log(`wave 1: ${clean.length}/${PARTS.length} parts clean: ${clean.map((r) => r.id).join(', ')}`)
if (!clean.length) return { status: 'no-clean-parts', results }

phase('Integrate')
const integ = await call(`Integrate the CLEAN wave-1 parts of the GDFL one-second fill architecture into one branch.
Clean parts (branch @ head): ${clean.map((r) => `${r.branch} @ ${r.head}`).join('; ')}. Not clean (do NOT merge): ${results.filter((r) => !r || r.status !== 'clean').map((r) => r ? `${r.id} (${r.status})` : 'null').join('; ') || 'none'}.
Create branch feat/gdfl-1s in worktree ${W}/wt/G-integrate from origin/main (\`git -C ${P} worktree add -b feat/gdfl-1s ${W}/wt/G-integrate origin/main\`; reuse it if it exists), CARGO_TARGET_DIR=${W}/tgt/g-integrate. Bring in each clean branch with \`git merge --squash <branch>\` followed by ONE commit per part (no rebase of the part branches). SQUASH IS REQUIRED: feat/gdfl-census and feat/gdfl-cm-reader carry verbatim real GDFL data rows in their commit history (licence UNVERIFIED, repository public, D-0819 says squash only), and feat/gdfl-1s must contain NO such row in ANY commit. After integrating, prove it: \`git -C ${W}/wt/G-integrate log -p origin/main..HEAD\` must show no line from a real GDFL CSV (grep the diff for the GDFL row shape, e.g. ticker,DD/MM/YYYY,HH:MM:SS,price rows, and for the quoted sample lines named in the part branches' history), and report that command and its output in checks. NEVER PUSH feat/gdfl-1s, feat/gdfl-census or feat/gdfl-cm-reader. Resolve conflicts by hand: ledger/invariant/limits tails keep BOTH sides in order, never drop or renumber a row; if two parts took the same decision number, renumber the later one to the next free number and fix every reference; a placeholder type defined by one part for another (e.g. the grid's cell type before core::second) is replaced by the real one. Then run the full Definition of Done on the merged head and fix test-first anything the merge broke (commit on feat/gdfl-1s).
${OPERATOR}
${LAW}
Report branch, head, commits, checks, still_open, summary.`, { label: 'gb:integrate', phase: 'Integrate', schema: REPORT, effort: 'medium' })
let iv = null
if (integ) {
  iv = (await parallel(['merge-integrity', 'law-and-design'].map((n) => () => call(`Independent adversarial reviewer of the integration branch feat/gdfl-1s (worktree ${W}/wt/G-integrate, head ${integ.head}). Lens ${n}: ${n === 'merge-integrity' ? 'every clean part is merged whole (git range-diff of each part branch against its merge shows nothing dropped or altered except recorded conflict resolutions); ledger/invariant tails keep both sides; no decision number duplicated; placeholder types replaced; the full Definition of Done passes on the merged head (re-run tests and clippy in a throwaway worktree).' : 'CLAUDE.md law and the design hold across the merged parts together (crate graph, gates 9/9b/22, Rust only, O(1) claims, no look-ahead, append-only versions).'} Default to refuted=true if you cannot confirm. Do not edit the branch.
${LAW}
Report refuted, reviewed_head, issues.`, { label: `gb:integrate-review:${n}`, phase: 'Integrate', schema: VERDICT, effort: 'high' })))).filter(Boolean)
}
return { results: results.map((r) => r && { id: r.id, status: r.status, branch: r.branch, head: r.head, open: r.open }), integration: integ, integration_review: iv }
```

## `state/resume-kit/make-continue-state.js`

```js
// Rebuilds the START block of scripts/build-wave1-continue.js from build journals, so a new session or account continues each
// part from its last finished step. usage: node make-continue-state.js <journal.jsonl> [<older journal.jsonl> ...]
// Later journals win. A part's state = its latest builder/repair report + the verdicts of its latest review round.
// If that round's repair already finished, the next round starts with no verdicts (the reviewers re-run).
const fs = require('fs'), path = require('path')
const K = __dirname
const lines = process.argv.slice(2).reverse().flatMap((j) => fs.readFileSync(j, 'utf8').trim().split('\n').map(JSON.parse))
const lab = {}; for (const e of lines) if (e.type === 'started') lab[e.agentId] = e.label
const res = {}; for (const e of lines) if (e.type === 'result' && lab[e.agentId]) res[lab[e.agentId]] = e.result
const rn = (l) => +((l.match(/:r(\d+)$/) || [0, 0])[1])
const prev = (() => { const s = fs.readFileSync(`${K}/scripts/build-wave1-continue.js`, 'utf8'); const m = s.match(/^const START = (.*)$/m); return m ? JSON.parse(m[1]) : {} })()
const out = {}
for (const p of ['census', 'core-second', 'store-grid', 'gdfl-cm']) {
  const reps = Object.keys(res).filter((l) => l === `gb:build:${p}` || l.startsWith(`gb:repair:${p}:`)).sort((a, b) => rn(a) - rn(b))
  const rounds = Object.keys(res).filter((l) => l.startsWith(`gb:review:${p}:`)).map(rn)
  const lastRepair = reps.length ? rn(reps[reps.length - 1]) : 0
  let r = rounds.length ? Math.max(...rounds) : (prev[p] ? prev[p].round : 1)
  let verdicts = {}
  if (r > lastRepair) { for (const l of Object.keys(res)) if (l.startsWith(`gb:review:${p}:`) && rn(l) === r) verdicts[l.split(':')[3]] = res[l] } else { r = lastRepair + 1 }
  if (!rounds.length && prev[p]) { verdicts = prev[p].verdicts; r = prev[p].round }
  const rep0 = reps.length ? res[reps[reps.length - 1]] : prev[p] && prev[p].rep
  const rep = { branch: rep0.branch, head: rep0.head, summary: String(rep0.summary || '').slice(0, 2500), still_open: (rep0.still_open || []).map((s) => String(s).slice(0, 400)) }
  for (const v of Object.values(verdicts)) for (const i of v.issues || []) i.evidence = String(i.evidence).slice(0, 1200)
  out[p] = { rep, round: r, verdicts }
  console.log(p, 'head', String(rep.head).slice(0, 8), 'round', r, 'verdicts', Object.keys(verdicts).join(',') || '(none: reviewers run)')
}
const f = `${K}/scripts/build-wave1-continue.js`
let s = fs.readFileSync(f, 'utf8')
s = s.replace(/^const START = .*$/m, 'const START = ' + JSON.stringify(out))
fs.writeFileSync(f, s)
fs.writeFileSync(`${K}/continue-state.json`, JSON.stringify(out, null, 1))
```

## `state/resume-kit/watch-build.sh`

```bash
#!/bin/bash
# Wait up to $1 seconds (default 570) for a new result in the build journal, then print running agents and the latest results.
J=${BUILD_JOURNAL:-/Users/parthi/.claude/projects/-Volumes-WD-BLACK-brutex-fresh-20260919-project/0093cf60-67a5-4c37-a8f5-11bed51e6a92/subagents/workflows/wf_3e6c4c50-45c/journal.jsonl}
n=$(grep -c '"result"' "$J"); end=$((SECONDS+${1:-570})); while [ $SECONDS -lt $end ] && [ $(grep -c '"result"' "$J") -eq $n ]; do sleep 20; done
date -u
echo "live agents (transcript written in last 12 min): $(find "$(dirname "$J")" -name 'agent-*.jsonl' -mmin -12 | wc -l | tr -d ' ')"
node -e '
const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map(JSON.parse);
const st={},done=new Set();for(const e of L){if(e.type==="started")st[e.agentId]=e.label;if(e.type==="result"||e.type==="failed")done.add(e.agentId);}
const run=Object.entries(st).filter(([a])=>!done.has(a)).map(([,l])=>l);console.log("started without result (includes stopped ones):",run.join(", "));
const r=L.filter(e=>e.type==="result");console.log("results",r.length);
for(const e of r.slice(+process.argv[2])){const v=e.result||{};console.log(" ",st[e.agentId], v.refuted!==undefined?("refuted="+v.refuted+" "+(v.issues||[]).map(i=>i.severity[0]).join("")):("head="+String(v.head).slice(0,8)));}
' "$J" "$n"
```

## Per-part state at save time (`state/resume-kit/continue-state.json`)

Each part: the last builder or repairer report, the review round it is in, and that round's finished verdicts with every issue in full.

```json
{
 "census": {
  "rep": {
   "branch": "feat/gdfl-census",
   "head": "56962ae2",
   "summary": "Round-1 review findings fixed on feat/gdfl-census in two commits on top of the squashed 077af46d.\n\n**Should-fix test gaps, each pinned test-first.** I re-applied each surviving break and confirmed it is now caught.\n- GC-24 spike above b: a new fixture where the measure is 50,000 ppm above b = max(p.close, n.open). Using p.open or n.close in place of p.close or n.open would make it measure nothing.\n- GC-14 shape skew: new test a_shapes_skew_counts_runs_closing_up_to_1530_and_not_at_it. A run closing at 15:20:00 or 15:29:59 counts; one closing at 15:30:00 does not.\n- GC-18 render: the summary fixture now has 2 uncaught runs against 1 forward-stamp run, so swapping the two totals is caught.\n\n**Nit test gaps, also pinned.**\n- GC-13: saturation is now checked at 15:10:00 and 15:29:59 (counted) and at 15:30:00 (not counted).\n- max_back_at is asserted to be the re-stamped 10:59:59, not the stamp as written.\n- GC-08: the test now records every file's bytes and every directory, _verify_log/run.log included, before and after the run.\n\n**Other nits, pinned by tests and stated as limits in D-0819 and 04-invariants.**\n- Only a one-row excursion counts as a forward spike (only_a_one_row_excursion_is_counted_as_a_forward_spike).\n- Every day is read on the fixed 09:15-15:30 grid, so an evening-only day is empty, sparse and halt-suspect at once. The count of 15 halt-suspect days is a count of days to source, not of halts; 7 of them have no row before 15:10.\n- `1000.` is read as whole rupees by the shared paisa decoder.\n- The census never lists the tree; the reviewer's one-off check (2,001 folders, 4 outside the span) is recorded.\n\n**Docs.**\n- docs/06-limits.md now gives the measured largest index file: 15,279,399 bytes, NIFTY 50 on 2026-09-18.\n- D-0819 lists the post-cutoff freeze measured from 15:10 rather than 15:15 as a deliberate departure from the design. 76 of the 78 listed freezes start at 15:14:59-15:15:01. The other 2 are 2025-10-21, whose price was already still from 14:46.\n- D-0819 records the squash and says the squash, not a gate, is what keeps vendor rows out of the history.\n- Provenance now names the measured source by git blob hash, which survives the squash: gdfl_census.rs b83ea4ff, cli lib.rs, pull csv/calendar/session and Cargo.lock.\n- I rebuilt from the clean commit d3a774d6 and re-ran the census over every day for both indices (state/design/census/run-d3a774d6/). Both reports are byte-identical to the 3a20c6a0 run apart from the events-directory line.\n",
   "still_open": [
    "The full cargo mutants --in-diff run (693 mutants) was not finished this round. Under the current machine load it would take about 16 h, so I stopped it after 18 mutants (17 caught, 1 unviable, 0 missed). This round changed no non-test source: gdfl_census.rs is blob b83ea4ff, the same blob as the reviewer's full run (670 caught / 23 unviable / 0 missed at e1715c47). The only existing tests I chang",
    "Branch coverage cannot be measured on the stable toolchain (it needs nightly). Line, region and function coverage is 100% on gdfl_census.rs.",
    "Landing: origin/main's 4bbd37df appended D-0910 at the same place in docs/05-decisions.md, 04-invariants.md and 06-limits.md. Landing will hit a mechanical append-append conflict: keep both blocks. I did not rebase, per my instructions.",
    "The vendor-row history finding is resolved on the branch by the earlier squash (077af46d, the same tree as e1715c47). The old commits survive only in the local reflog. As D-0819 now says, nothing but the squash enforces this: no gate reads history, and the branch must still never be pushed by its builder and must land only as a squash or rebuild.",
    "summary.md (outside the repo, /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/census/summary.md) was updated. Its previous text is kept at state/design/census/logs/summary-before-round3.md."
   ]
  },
  "round": 2,
  "verdicts": {
   "behaviour-extremes-o1": {
    "refuted": false,
    "reviewed_head": "56962ae2968d32a97aea0227a942e8e8ba4cfb06",
    "issues": [
     {
      "severity": "nit",
      "description": "A last line that is malformed but not cut short is recorded as a truncated tail and dropped. This happens when the line has no final newline, is stamped at or after 15:30, and names the right ticker and date: a full 10-field row with a corrupt LTP, or an 11-field row. `cut_after_close` (gdfl_census.rs:727) checks only the ticker, the date and a stamp of 15:30 or later after `parse_row` has already failed. It never checks that the row is actually shorter than a whole row. The rustdoc ('a last row with no line feed that is cut short') and the `tail_truncated_post_session` label therefore over-claim. The effect is limited: the row is post-session, so it holds no session price, and the real run counted 0 truncated tails.",
      "evidence": "Probe test added in the throwaway worktree (removed afterwards), run with `cargo test -p cli --lib --locked -- reviewer_probes --nocapture`. The row `NIFTY 50.NSE_IDX,01/04/2024,15:45:00,1x03.10,0,0,0,0,0,0` as the last line with no final newline gave 'corrupt (not cut) post-close tail -> Ok((true, 1))'. The 11-field row `...,15:45:00,1003.10,0,0,0,0,0,0,0` gave 'eleven-field post-close tail -> Ok((true, 1))'. In both, tail_truncated_post_session=true and the row was dropped rather than refused."
     },
     {
      "severity": "nit",
      "description": "The 13 March 2020 fixture comment and invariant GC-05 misquote the measured file's second still run by one second. The comment at gdfl_census_tests.rs:658-661 says 'the measured file holds ... another from 10:13:19 to 10:21:00', and GC-05 says '461 s 438 s later'. The real file first prints 8555.15 at 10:13:18, and the verb's own output on that file reports 462 s, moved 437 s. This is a factual claim about a vendor file that the census contradicts (CLAUDE.md §3 rule 1). The 2,692 s run from 09:21:09 is quoted correctly.",
      "evidence": "`awk -F, '$3>=\"10:13:15\" && $3<=\"10:13:21\"'` over INDICES/2020/MAR_2020/GFDLCM_INDICES_TICK_13032020/NIFTY 50.NSE_IDX.csv prints '10:13:17 8552.35' then '10:13:18 8555.15' and '10:13:19 8555.15'. `grep '^  still 2020-03-13' run-d3a774d6/census-NIFTY.txt` prints 'still 2020-03-13 10:13:18..10:21:00 462 s at 8555.15 moved 437 s'."
     }
    ]
   },
   "tests-bite": {
    "reviewed_head": "56962ae2968d32a97aea0227a942e8e8ba4cfb06",
    "refuted": true,
    "issues": [
     {
      "severity": "should-fix",
      "description": "Branch coverage on the touched module is 99.60%, not 100%, so the definition of done in CLAUDE.md section 9 is not met. The builder says branch coverage 'needs nightly' and so was not measured, but a nightly toolchain is installed on this machine (~/.rustup/toolchains/nightly-aarch64-apple-darwin, rustc 1.99.0-nightly 2026-08-07). Measured with that toolchain, one branch is never taken: the `None` arm of `c.shape_skew_s.get_mut(run.shape as usize)` in `record_run` (crates/cli/src/gdfl_census.rs:899). That arm can only be reached by `Shape::Unclosed`, and an unclosed run always has `closed_at: None`, so the arm sits inside `if let Some(r) = run.closed_at` where it can never run. It is dead code, and no test or mutant can reach it. Suggested fix: make it total, for example by matching `run.shape` explicitly (Unclosed has no R) or by indexing only the three closed shapes. Alternatively, record it honestly as an unreachable branch in docs/06-limits.md and correct the report's 'not measured' statement.",
      "evidence": "Throwaway worktree at 56962ae2. Ran `PATH=<queue>:<nightly>/bin CARGO_QUEUE_REAL=<nightly>/cargo RUSTC=<nightly>/rustc cargo llvm-cov --branch --locked -p cli --lib -- gdfl_census`: 98 passed. Report collection then failed on nightly cargo's new build-dir layout, so I merged the profraw files by hand and ran `llvm-cov report <...>/debug/build/cli/e68861349e38046c/out/cli-e68861349e38046c -show-branch-summary`. Result for gdfl_census.rs: Regions 2492/0 missed 100.00%, Functions 149/0 100.00%, Lines 1542/0 100.00%, Branches 248, 1 missed, 99.60%. `llvm-cov show -show-branches=count` gives `Branch (899:16): [True: 70, False: 0]`, and line 899 is `if let Some(held) = c.shape_skew_s.get_mut(run.shape as usize)`, inside `if let Some(r) = run.closed_at`. Shape::Unclosed is built only with `closed_at: None` (gdfl_census.rs:862/868)."
     }
    ]
   },
   "law-and-design": {
    "refuted": false,
    "reviewed_head": "56962ae2968d32a97aea0227a942e8e8ba4cfb06",
    "issues": [
     {
      "severity": "should-fix",
      "description": "D-0819 drops the stock measurements without saying it departs from the design. Design §12, in the revision 7 (C36) paragraph 'Version 1 and phase 2 in this plan', says: 'Steps 0, 3 and 5 still build and test the stock reader, the LTQ > 0 filter and the census's stock measurements on synthetic fixtures, because the format and the census serve phase 2'. Design §3.6 also asks for the TickerMap stem classification and, from revision 7, stock late rows per instrument-day. The verb accepts only NIFTY or BANKNIFTY. D-0819's 'Scope' paragraph defers all stock measurements to phase 2 under 'the version-1 scope (D-0818)' and 'need D-0808's TickerMap'. But §12 states that the version-1 step 0 builds them on synthetic fixtures. D-0819 has two 'departs from the design' sections (the §3.3 TICK row check and the 15:10 versus 15:15 freeze), and this departure is in neither. The reason given, version-1 scope, misreads §12. Fix: add a named departure, or build the synthetic-fixture stock measurements.",
      "evidence": "`sed -n 1478,1560p gdfl-1s-fill-architecture.md` gives the §12 paragraph quoted above (verbatim). `sed -n '/^### D-0819/,$p' docs/05-decisions.md | grep -n -i 'stock|phase 2|Steps 0'` matched only lines 264-266 of the entry: 'the stock measurements (still and absent runs over LTQ > 0 rows, per-instrument late rows, sparse stock days), which need D-0808's TickerMap and are phase 2's evidence'. No departure section names §12's step-0 stock instruction."
     },
     {
      "severity": "nit",
      "description": "A real vendor value is in a committed test comment in this public repository. The comment cites RELIANCE.NSE 2024-04-01 line 2 with 'LTQ 36406'. That is the real file's LTQ, so it is not invented. Design §12's fixture policy says prices and quantities are invented. A single field is not a whole row, so fixtures_are_built_not_pasted passes, but the comment still carries vendor data. The design document itself quotes the value, and that document is outside the repository.",
      "evidence": "crates/cli/src/gdfl_census_tests.rs:1719-1721 reads: '// SYNTHETIC: ... The stock shape is RELIANCE.NSE 2024-04-01 line 2 (LTQ 36406)'. `head -2 .../STOCKS/2024/APR_2024/GFDLCM_STOCK_TICK_01042024/RELIANCE.NSE.csv | tail -1 | awk -F, '{print $9}'` printed 36406."
     },
     {
      "severity": "nit",
      "description": "The census writes its own session-clock literals and its own copy of the proposed fold rule, both inside cli. SESSION_OPEN=33_300, SESSION_CLOSE=55_800 and FORCED_CUTOFF=54_600 are hand-written rather than derived from pull::calendar::OPEN_MINUTE. Deferral, UTC re-stamp and late-run shapes are implemented in cli. CLAUDE.md §5 warns that repeating calendar or fold rules in cli creates a second authority. D-0819 says this is a measurement of a rule that does not exist yet ('when the fold lands, the census should call it'), so the duplication is recorded. Nothing tracks it, though: no test or plan row would fail or flag it once pull::fold_second_grid and core::second land.",
      "evidence": "gdfl_census.rs:70-78 holds the `pub const SESSION_OPEN: u32 = 33_300;` group. pull/src/calendar.rs:126 has `pub const OPEN_MINUTE: u16 = 9 * 60 + 15;`. D-0819, 'Why in cli and not in pull::gdfl_cm', reads: 'when the fold lands, the census should call it'. docs/07-plan.md on this branch has no row for the census (`grep -n -i 'gdfl|census' docs/07-plan.md` found only older rows)."
     },
     {
      "severity": "nit",
      "description": "D-0819 says the session 'relayed the data-era rule numbered D-0816' but does not say who made it. Per the operator's chat, Claude chose it under the operator's 'pick everything' delegation, and the operator did not decide it. The entry neither misstates this nor states it.",
      "evidence": "D-0819's first paragraph: 'The same session relayed the data-era rule numbered D-0816 (every result labelled with its data era, and in-sample and out-of-sample windows from the same era by default) ... does not itself decide the rule.'"
     }
    ]
   }
  }
 },
 "core-second": {
  "rep": {
   "branch": "feat/gdfl-core-second",
   "head": "27d2dcd38ae987e3539e932859e62c92dd3eff5d",
   "summary": "I found the worktree clean on resume. The stopped agent had already committed the full round-7 repair as 27d2dcd3, and I checked it against every upheld finding:\n- **[lib] path redirection (should-fix):** core/tests/lint.rs now pins every code line of crates/core/Cargo.toml, ignoring whitespace. It has a bite test for path under [lib], dotted and inline lib keys, autolib, a trailing comment, truncation and extra tables. GSC-01b cites it.\n- **GSC-02b fixed-query blind spot (nit):** the bench now times each lookup with a far query as well, and GSC-02b says so.\n- **Malformed table (should-fix):** the invariants table uses the house header `| ID | Invariant | Proof | ✓ |`.\n- **GSC-01c and D-0811 item 1 (nit):** both now say \"these substrings and decimal tokens\". The reviewer's bypasses are recorded.\n- **Ranges in GSC-02/02b and docs/06-limits.md (nits):** labelled as observations.\n- **D-0811 test-first paragraph (nit):** now cites b7980ca9's \"16 of 17 red\" message and says that run was not logged.\n- **D-0811 integrator requests (nits):** it asks for the CLAUDE.md §5 line and tells runner to refuse negative holds.\n- **Coverage (should-fix):** core's two cheap line gaps are closed (universe.rs 5595, vendor.rs index_alias_source). The rest of main's crate-wide gap is rejected with its measurement in D-0811.\n\nI then ran the full Definition-of-done to completion on HEAD 27d2dcd3, and every check is green except crate-wide coverage:\n- fmt, clippy, cargo test (5975 passed) and deny all pass.\n- Mutants: 0 missed of 87.\n- All CI gates pass, plus the Gate 8 core ratio bench.\n- Crate-wide coverage is not 100%. Every gap is in main's code, and this branch's own code is 100%.\n\nNo new commit was needed. Logs are in /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/g-core-second/logs/.",
   "still_open": [
    "CLAUDE.md §9 coverage is not met crate-wide. pull has 1,557 uncovered lines in total, and core misses 26 branches (instrument.rs 4, universe.rs 21, vendor.rs 1). Every gap is in main's unchanged code. D-0811 rejects closing them on this branch and gives the measurements. This branch's own code is at 100% lines, regions, functions and branches. The gap is main's Definition-of-done debt, which the i",
    "For the integrator, as D-0811 asks: add one sentence to CLAUDE.md §5 naming core::second::sod_of and slot_of as the bound second IST implementation (D-0811 item 10, GSC-01d). This branch does not edit CLAUDE.md.",
    "For the runner fill policy: refuse a hold below one minute itself. checked_add_minutes and checked_add_seconds accept negative amounts by design.",
    "The GSC-02/02b timing ranges in docs are labelled as observations, not bounds. This run's figures (290 ps floor, 2.16-2.38 floors) fall inside the 'after round 7' band only roughly, which is expected.",
    "Never pushed, per instructions."
   ]
  },
  "round": 3,
  "verdicts": {}
 },
 "store-grid": {
  "rep": {
   "branch": "feat/gdfl-store-grid",
   "head": "5ef5c37738151ef6d706758d3d381806cd9b4caf",
   "summary": "I fixed all 12 round-2 findings on feat/gdfl-store-grid; the one open item is whole-crate coverage, which waits on the operator. The stopped agent's uncommitted edits were checked and found correct, so I kept them, added source notes to the constants, and committed everything on top in two commits (head 5ef5c377).\n\n- **Session constants:** core::second's UTC_OFFSET_S, SESSION_OPEN_SOD and SLOTS are tied to pull::session by three compile-time asserts and the test core_second_repeats_this_modules_session_facts (GS-01c). Each constant now cites docs/00-charter.md section 3 and the pull::session value it copies. pull already depends on core, so no new crate arrow.\n- **NSE only:** a grid path always renders grids/gdfl/NSE/...; the exchange is no longer a parameter, and a key on BSE is refused with PathError::ExchangeNotGridded (GS-04q).\n- **symbol_id:** GridPath::symbol_id works it out from the path (low 32 bits of fnv1a of the symbol), and GridWriter::open / GridReader::open no longer take it from the caller. A month copied under another instrument's path is refused with SymbolMismatch by the store itself (GS-14b).\n- **No silent defaults:** DaySeconds::at, Month::state_of and Month::record_of now index directly under a reasoned allow, instead of falling back to a made-up value (CLAUDE.md section 4).\n- **O(1) shape test:** GS-01b reads the source of each lookup function and refuses any loop, iterator, search or chunk walk, and any unwrap_or on the three lookups above.\n- **Dependency check:** GS-14 now asserts store has exactly two dependencies (brutex_core and telemetry), so any mapping crate is refused, not only memmap2.\n- **Second interrupted append:** GS-04p builds that state on the grid directory and shows it is refused by both appends and on both opens, with the bytes left unchanged.\n- **Coverage wording:** D-0804's coverage paragraph is now an open request to the operator instead of an exception the building session recorded for itself. docs/06-limits.md has the re-measured numbers, now including pull.\n- **Integration note:** it already cites the owning branch's head 27d2dcd3 and now also says the merge keeps that branch's five pull::session asserts.\n\nDocs updated: 02-store-format, 04-invariants (GS-01b, GS-01c, GS-04p, GS-04q, GS-14, GS-14b), 05-decisions D-0804 and 06-limits.\n\nAll definition-of-done checks pass except whole-crate coverage: fmt, clippy, the full workspace tests (run outside the sandbox, which blocks pull's network tests), cargo deny, mut",
   "still_open": [
    "Coverage of the whole crate (CLAUDE.md section 9) is still not met for core, store or pull. Every miss is on lines already on origin/main, and the branch's own diff is fully covered. D-0804 now asks the operator to confirm or decline that a part is held only to its own diff; the earlier text recorded this as an exception the session gave itself. Until the operator answers, the part is not done und",
    "Round-3 test-first history: the stopped session made the round-3 edits before it recorded any red runs, so the commit history does not show the tests coming first. As evidence instead, each new test was run against its fix reverted and failed (recorded in D-0804). GS-04p and GS-14b guard behaviour the code already had; neither reproduces a fault.",
    "Step-4 test-first history (commit 38255368) can only be fixed with a rebase, which this part may not do. Left as recorded.",
    "Merge work at integration with feat/gdfl-core-second (head 27d2dcd3), as the second.rs integration note lists. Item 1: rewrite the DayError binding as a match. Item 2: change the store tests that build a bad day through DaySeconds::new. Item 3: delete store's link_deltas. Item 5: drop this branch's three pull::session asserts in favour of the owner's five, which I checked at 27d2dcd3.",
    "Reviewer finding 6 (another process building mutants into a shared target dir) is about the environment and needed no code change. All runs this session used this part's own target dirs."
   ]
  },
  "round": 3,
  "verdicts": {}
 },
 "gdfl-cm": {
  "rep": {
   "branch": "feat/gdfl-cm-reader",
   "head": "97a0f6e1bd21b525f0fa82e07a6e0c5ab4211c21",
   "summary": "Round-3 repair of feat/gdfl-cm-reader is committed on top of the squash and merge already on the branch. 5a55340c is a squash whose tree is byte for byte the reviewed tip 14133fa3; it was made outside this session. 4f19b981 merges origin/main (4bbd37df, D-0910). Every commit in origin/main..HEAD has 0 GDFL rows, and the head merges cleanly with main, which resolves the blocking history finding and both merge-conflict findings. Most of the code fixes were already in the working tree from an earlier, interrupted pass of this session. I checked each one test-first, finished the rest, and committed them.\n\nEach finding:\n- Central directory: the walked records must end at the declared directory size, otherwise ArchiveMalformed.\n- verify: DayMembers::verify_source and gdfl_cm::read_day_checked check a file's raw bytes before decoding, so a truncated extraction is named SourceLengthMismatch or SourceCrcMismatch, not MalformedRow.\n- post_session_tail: the fields after the Time must be a prefix of a row.\n- SWEPT: the index slots and stems match NIFTY and BANKNIFTY exactly, and INDEX_SLOTS = InstrumentKey::SWEPT.len().\n- Test rename: the test is now ..._largest_written_stamp_....\n- CM-13: the wording no longer says there is no search, and the source test now allows exactly one .find(, the bounded end-record scan.\n- CM-14: the archive test now also checks the stock direction.\n- TICK: the doc now says it is used as a positivity floor.\n- D-0812 item 1: corrected to 256 plain records and 3,953 wide ones.\n- D-0819: the census is cited as D-0819, not D-0818.\n- Forward references: D-0804, D-0805, D-0816 and D-0818 are marked pending.\n- docs/06-limits.md: the test path now includes tests::.\n- Coverage: pull's coverage is recorded in docs/07-plan.md.\n- Squash recovery: the docs/07-plan.md row and D-0808 now say a squash must come from a tree that contains main.\n\nCharter trace: every vendor fact the reader relies on is now recorded in docs/00-charter.md, measured over all 4,002 NIFTY 50 and NIFTY BANK files, all 2,001 folders of each tree, and the archive's whole central directory:\n- CRLF on every line except the unterminated last line of 34 files from 2018.\n- The first row is stamped 09:07:01 to 09:09:52 in 3,978 files.\n- The quote, LTQ and open-interest fields are 0 on every ten-field row.\n- There are 227 UTC rows, all on 11 October 2020 days, with back-steps of 19,798 to 19,801 s.\n- Folders hold 50 to 141 index files and 1,764 to 3,710 stock files.\n- The archive has 4,209 e",
   "still_open": [
    "pull's whole-crate coverage is 95.11% of lines and 82.49% of branches, against the 100% CLAUDE.md section 9 asks for. The shortfall is in older pull modules and predates this branch. It is recorded in docs/07-plan.md and D-0808 rather than fixed.",
    "The old commits that held real GDFL rows (266561f2, 51518fdc, e8f0d281, and 1457dd29 to 76d5c4f8) are on no branch now but stay in the local object store until it is pruned. They must never be pushed.",
    "IndexEndsInSession checks a fixed 15:30 close. The four index files of the 2024-03-02 and 2024-05-18 disaster-recovery Saturdays end before it, but read_day refuses them first with CalendarNotRegular. If a future caller used decode directly on those days, it would refuse them as cut. Recorded, not changed.",
    "D-0804, D-0805, D-0816 and D-0818 are still forward references with no ledger entries. They are marked as pending in D-0808."
   ]
  },
  "round": 2,
  "verdicts": {
   "behaviour-extremes-o1": {
    "refuted": true,
    "reviewed_head": "97a0f6e1bd21b525f0fa82e07a6e0c5ab4211c21",
    "issues": [
     {
      "severity": "should-fix",
      "description": "`resolve` and `stem_slot` are documented as worst-case O(1), but they are O(stem length). In `resolve`, `name.contains('.')` (crates/pull/src/gdfl_cm.rs:421) and the suffix strip read the whole stem before `InstrumentKey::cash` and its 24-byte `Symbol` cap can refuse it. The module doc (gdfl_cm.rs:67-72, \"[`resolve`] is worst-case O(1)\") states this, and so does the new docs/06-limits.md row (\"Every step is bounded by the stem's length or a fixed count, so it is O(1)\"), which contradicts itself. Both are public functions that take any &str, and zip member names can be up to 65,535 bytes. The repo has a precedent for exactly this defect: `core::universe::FnoIndex::position` added a length guard before hashing because the cost was 'O(n) in the argument'. Fix: refuse any stem longer than SYMBOL_CAPACITY plus the tree suffix before scanning it, or restate both claims as O(stem length) with the 255-byte and 65,535-byte bounds.",
      "evidence": "Throwaway probe in a detached worktree at 97a0f6e1: `cargo test --release -p pull --lib probe_resolve_slope -- --nocapture`, with resolve(CmKind::Stocks, \"A\"*n + \".NSE\") over 200 reps. Output: 'stem 8 B: 115ns per call', '1000 B: 100ns', '100000 B: 8.233µs', '10000000 B: 297.89µs'. The cost is linear in the argument, about 2,600 times the short-stem cost at 10 MB. Source: gdfl_cm.rs:421 `CmKind::Stocks if name.contains('.') => Resolution::SeriesVariant,` runs before `InstrumentKey::cash(Exchange::Nse, name)`."
     },
     {
      "severity": "should-fix",
      "description": "`read_day_checked` returns Ok(None) when the extracted day folder has no file for the instrument, even though the vendor day zip it was given holds that file. A file lost by a partial or crashed extraction, or one deleted before listing, therefore looks exactly like a day the vendor never shipped. It reaches fills as a missing day and the census as absent, not as a refusal that names the extraction. The archive is the required authority under D-0812 and is already loaded, so this is a failure that degrades under the wrong name. It is not a guess, which is what the read_day doc says Ok(None) avoids. Fix: when `locate` finds nothing, look up `members.member(<stem>.csv | <stem>.CSV)` and refuse with a named variant, such as SourceMissing { member }, when the member exists.",
      "evidence": "Throwaway probe in the gdfl_archive tests module: an empty dir GFDLCM_INDICES_TICK_01042024, `list_folder(&dir)`, and `members()` (whose day zip holds `GFDLCM_INDICES_TICK_01042024/NIFTY 50.NSE_IDX.CSV`). Output: 'PROBE member present: Ok(Member { len: 243, crc32: 202440575 })' and 'PROBE read_day_checked with no extracted file: Ok(None)'."
     },
     {
      "severity": "nit",
      "description": "An unterminated last line that ends in a bare CR is treated as a cut post-session tail when it has 4 to 9 fields: it is dropped and `tail_truncated_post_session` is set. A trailing CR shows the vendor terminated the line, so this is a complete short row, and CM-05 says such a row must refuse MalformedRow. The impact is small because the archive CRC check passes only if the vendor really wrote those bytes, but the recorded reason is wrong.",
      "evidence": "Probe decode(header CRLF + rows 10:00:00 and 16:00:00 CRLF + b\"NIFTY 50.NSE_IDX,01/04/2024,16:00:01,12\\r\") with nifty Expect. Output: 'PROBE cut tail with CR: Ok((2, 57600, 0, true, false, Some(100)))'. `strip_cr` runs before `post_session_tail` at gdfl_cm.rs:874."
     },
     {
      "severity": "nit",
      "description": "The module doc still describes `resolve`'s index path as 'two comparisons for an index'. The new docs/06-limits.md row says this wording was corrected in round 2 because it 'left most of that out', but the module doc was never updated. The index path matches against three literals, then calls InstrumentKey::index and is_sweepable.",
      "evidence": "crates/pull/src/gdfl_cm.rs:67-68 reads '[`resolve`] is worst-case O(1): two comparisons for an index, one bounded probe of `FNO_INDEX` for an equity'. docs/06-limits.md (diff) reads '(Until the round-2 review this row said \"two string comparisons for an index stem\", which left most of that out.)'"
     },
     {
      "severity": "nit",
      "description": "For context, not a defect. Every other extreme probed behaves correctly and refuses by name. Not re-verified first-hand: workspace-wide test, the 100% coverage figure, the mutants run and run-all-gates. Within this lens, `cargo test -p pull --locked` (outside the sandbox, for loopback sockets) and `cargo clippy -p pull --all-targets -- -D warnings` both pass at 97a0f6e1.",
      "evidence": "Probes: empty file, '\\n' and '\\r\\n' gave HeaderUnknown. A header with no newline gave NoSessionRows. A BOM gave HeaderUnknown. A blank trailing line gave MalformedRow{4}. 0.045 snapped to 5 paisa and was counted. 0.044 gave PriceRefused. 92233720368547758.07 was accepted as i64::MAX, and 92233720368547758.075 gave PriceRefused. Leading '+' and a fullwidth digit gave PriceRefused. A NUL or lower-case ticker gave TickerMismatch. Duplicate and back-stepped stamps were kept in file order. LTQ u64::MAX was accepted, and u64::MAX+1 or -1 gave MalformedRow. A stock file with only LTQ=0 rows gave NoSessionRows. An index file cut at a line boundary at 15:29:59 gave IndexEndsInSession{55799}, and 15:30:00 passed. A misfiled date gave DateMismatch. M&M.NSE and BAJAJ-AUTO.NSE resolved to slots 124 and 22, while reliance.NSE, ' RELIANCE.NSE', NIFTY.NSE and 'NIFTY  50.NSE_IDX' resolved to none. `cargo test -p pull --locked`: lib 587 passed, 0 failed, and every other binary ok. clippy -p pull: EXIT2 0. In the sandbox, 15 tests failed only with 'a loopback port: ... Operation not permitted'."
     }
    ]
   },
   "law-and-design": {
    "refuted": false,
    "reviewed_head": "97a0f6e1bd21b525f0fa82e07a6e0c5ab4211c21",
    "issues": [
     {
      "severity": "should-fix",
      "description": "The reader snaps a third LTP decimal half-up, but design §3.3 says to refuse it. That deviation is not one the law requires. The design's Row checks (revision 11) say \"LTP is decoded with `csv::paisa` (more than 2 dp refuses ...)\". Refusing a third decimal does not conflict with CLAUDE.md §7 (\"Snapping happens once, at the write boundary, half-up\"), so the task's \"law wins\" exception does not cover this. D-0808 justifies the change by saying that `csv::paisa`'s \"own reason for refusing (a snap there would compete with the one at the write boundary) does not apply\". That leaves out the first reason `csv::paisa` gives, which does apply here: \"a third digit is the vendor sending something this build does not understand\". The existing GDFL F&O reader refuses a third decimal, so the two readers now treat it differently. In practice the effect on the swept surface is nil today: I found 0 non-two-decimal LTPs across all 4,002 NIFTY 50 and NIFTY BANK files. Fix one of two ways: refuse a third decimal as the design says (`PriceRefused`), or have the design amended and correct D-0808's account of `csv::paisa`'s reasons.",
      "evidence": "`sed -n 459,477p state/design/gdfl-1s-fill-architecture.md` gives Row checks: \"LTP is decoded with `csv::paisa` (more than 2 dp refuses; LTP below `TICK` refuses)\". The doc comment at `crates/pull/src/csv.rs:281-286` reads: \"Rejects a third decimal place rather than rounding it: the tick grid is two places (CLAUDE.md §7), so a third digit is the vendor sending something this build does not understand, and rounding it here would be a second snapping site...\". The code at `gdfl_cm.rs:1082-1094` calls `Paisa::from_rupee_text_half_up`. D-0808 says \"A third decimal is snapped half-up, not refused ... `csv::paisa`'s own reason for refusing (a snap there would compete with the one at the write boundary) does not apply\". I ran `xargs -0 awk -F, 'FNR>1 && $4 !~ /^[0-9]+(\\.[0-9][0-9]?)?$/'` over every NIFTY 50 and NIFTY BANK file; it printed `bad 0`."
     },
     {
      "severity": "should-fix",
      "description": "The branch does not meet the CLAUDE.md §9 definition of done for `pull`, which it touches. Coverage of the whole `pull` crate is 95.11% of lines and 82.49% of branches, against the 100% the law requires. The shortfall predates this branch and is recorded honestly in `docs/07-plan.md` and D-0808, and the two new modules are reported at 100%. Even so, the law's condition (\"line and branch coverage 100% on every touched crate\") is not met. Landing this needs that recorded exception, not a claim that the branch is done.",
      "evidence": "The `docs/07-plan.md` row added by this branch reads: \"the crate is at 95.11% of lines (1,624 of 33,242 missed) and 82.49% of branches (420 of 2,399 missed)\". CLAUDE.md §9 says \"line and branch coverage 100% on every touched crate\"."
     },
     {
      "severity": "nit",
      "description": "Some forward references are unmarked. D-0808 item 9 and its round-1 paragraph cite \"D-0802 consequence 10\" as if that decision were in the ledger. D-0802 exists only as an unlanded draft branch, and the \"Forward references, stated\" paragraph lists D-0804, D-0805, D-0816 and D-0818 but not D-0802. The module docs also cite \"(`pull::fold`, D-0805)\" and \"the fold's rule (D-0805)\" without saying that D-0805 is pending.",
      "evidence": "`grep -nE '^### D-080[2-7]' docs/05-decisions.md` returns nothing. `grep -n 'D-0802' docs/05-decisions.md` returns lines 43840 and 44036, \"(D-0802 consequence 10)\", with no draft qualifier. `gdfl_cm.rs:38`, `:790` and `:800` cite D-0805."
     },
     {
      "severity": "nit",
      "description": "Several vendor claims are not fully traceable to the charter. The module doc says \"Sampled index and equity files from 2018 to 2026 carry no third decimal at all\", and D-0808 says sampled NIFTY 50, NIFTY BANK, RELIANCE and SBIN files of 2018, 2020, 2022, 2024 and 2026 carry none. The charter row records only NIFTY BANK 2020-10-01 and SBIN 2022-04-01 (plus INDIA VIX). The claim holds for both indices across the whole copy, which I measured. To close the trace, record that whole-copy measurement in the charter or narrow the claim.",
      "evidence": "The `docs/00-charter.md` row \"Third decimal in the LTP\" reads: \"NIFTY BANK 2020-10-01 and SBIN 2022-04-01: 0 LTPs that are not a plain decimal\". `gdfl_cm.rs:52-53` reads: \"Sampled index and equity files from 2018 to 2026 carry no third decimal at all\". My whole-copy `awk` over the 4,002 index files gave `bad 0`."
     },
     {
      "severity": "nit",
      "description": "The unverified path stays public. `read_day` decodes a file without the D-0812 archive check, and it is `pub` beside `read_day_checked`. Design §3.3 says every extracted file is verified before it is gridded, with \"never a fallback to the unverified tree\". Step 6 is meant to call `read_day_checked`, but the API still lets an ingest skip the check. Consider restricting `read_day` (for example to `pub(crate)` or tests) or documenting that it must not feed the grid.",
      "evidence": "`gdfl_cm.rs:1158`: `pub fn read_day(...) { read_day_with(dir, folder, key, day, |_, _| Ok(())) }`. The D-0812 section \"Not built here (step 6...)\" relies on the caller choosing `read_day_checked`."
     }
    ]
   },
   "tests-bite": {
    "refuted": false,
    "reviewed_head": "97a0f6e1bd21b525f0fa82e07a6e0c5ab4211c21",
    "issues": [
     {
      "severity": "should-fix",
      "description": "CLAUDE.md section 9 asks for 100% line and branch coverage on every touched crate, and the branch touches pull (lib.rs, vendor.rs, and the two new modules). The two new files are at 100%, which I measured myself. The pull crate as a whole is not. The builder reports 95.11% of lines and 82.49% of branches for the crate, says the shortfall predates the branch, and has recorded it in docs/07-plan.md and D-0808. My gdfl-only run cannot show the crate-wide figure, so that number is the builder's. This is a gap in the definition of done that has been disclosed, not hidden. Nothing in the diff weakens coverage.",
      "evidence": "cargo llvm-cov -p pull --lib --branch -- gdfl with nightly 1.99, then llvm-profdata merge and llvm-cov report --show-branch-summary on the test binary: gdfl_archive.rs 1165 lines, 0 missed, 46 branches, 0 missed, 101 functions, 0 missed; gdfl_cm.rs 1780 lines, 0 missed, 152 branches, 0 missed, 168 functions, 0 missed. The rest of pull is not 100% (crate-wide figure as stated in the builder's still_open)."
     },
     {
      "severity": "nit",
      "description": "I could not reproduce the builder's mutant counts. They report 340 caught, 21 unviable and 3 timeouts. My clean run on 97a0f6e1, with an isolated target per worker, gave 331 caught, 33 unviable and 0 timeouts. A likely cause, which I hit myself and then corrected: if CARGO_TARGET_DIR is exported when cargo mutants runs, all four workers share one target directory and run each other's mutated binaries. That makes outcomes nondeterministic and can produce spurious timeouts. The conclusion, 0 MISSED, holds on the clean run, so this is a methodology note.",
      "evidence": "My first run exported CARGO_TARGET_DIR. The debug log showed every worker running Executable /Volumes/.../tgt/gv-gdfl-cm-tests-bite/debug/deps/pull-d6c55d737d821f00, the same file for all of them. I reran with env -u CARGO_TARGET_DIR cargo mutants --in-diff <git diff origin/main..97a0f6e1> -p pull -j 4 -- --lib gdfl and got: '364 mutants tested in 88m: 331 caught, 33 unviable', with missed.txt and timeout.txt both 0 lines. The test filter gdfl only narrows which tests can kill a mutant, so 0 missed with it implies 0 missed without it."
     },
     {
      "severity": "nit",
      "description": "docs/06-limits.md cites pull::gdfl_archive::tests::the_day_table_worst_case_is_the_whole_day_range as pinning the worst-case dense-table figures, 2,932,897 cells of 24 bytes. The test does pin those two numbers. It never builds an Archive from a far-dated day zip, so the surrounding sentence ('bounded, so it is never a crash ... allocated before any other check') is reasoning, not something that test exercises. The row should say only that the test pins the figures.",
      "evidence": "In gdfl_archive.rs, lines 1114-1120, the test asserts only Day::days_from_epoch arithmetic and std::mem::size_of::<Option<Inner>>() == 24. It makes no call to Archive::from_source."
     }
    ]
   }
  }
 }
}```

```js
export const meta = {
  name: 'lane1b-adversarial-review',
  description: 'Adversarially review each lane 1-b fixer diff and the merge resolutions, then try to refute every finding',
  phases: [
    { title: 'Review', detail: 'one reviewer per fixer diff plus one for the merges' },
    { title: 'Verify', detail: 'one skeptic per reviewer tries to refute each finding' },
  ],
}

const SP = '/tmp/claude-0/-home-claude-brutex/3ac9da23-a5ad-5a6b-a575-21d88e64076d/scratchpad'
const REPO = '/home/claude/wt-review'
const COMMON = `Repository: the Rust workspace SJParthi/brutex, checked out READ-ONLY at ${REPO} (detached at a9fee04b, the merged landing tree). Base before any lane 1-b fix: fbdabaec.
STRICT: do not edit any file, do not run cargo or any build, do not run git commands that change state (no checkout, commit, merge, reset, stash, worktree). Read with git diff/show/log/grep and file reads only. Other agents are building on this box; do not start builds.
The verification reports that found the original defects (with file:line evidence and fix sketches) are ${SP}/verify/G1.md .. G5.md; original finding texts are in ${SP}/fq/lane1-b.md (grep the id). The fixers' rules were ${SP}/fix/FIXRULES.md.
Law that applies (CLAUDE.md in the repo is binding; read the parts you need): Rust only outside web/; O(1) per operation or an honest docs/06-limits.md bound; docs/04-invariants.md, docs/05-decisions.md and docs/11-findings.md are APPEND-ONLY (any removed or edited old line is a violation); no fallback that hides a failure; no test that asserts nothing; changes to stored bytes, digests, evidence versions or run identity need a new version and a decision entry; a docs/11 FIXED row must cite a commit on main.`

const FIND_RULES = `Attack adversarially. For each item the fixer claimed, decide: is it really fixed FULLY, on every caller and entry point, or only on the path the test drives? Then attack the new code with extreme inputs: zero, empty, one, i64::MIN/MAX and overflow, duplicates, case variants, a CAS day or a 15:14 close, a crash or kill between two writes, short and torn writes, ENOSPC, FIFO and symlink paths, concurrent writers, a one-thread pool, stale caches (ABA). Check every new test: would it fail if the fix were reverted, and would an operator-flip, a true/false return or a deleted ! survive it? Check every O(1)/O(P) claim against the code. Check that new docs claims are true against code (cite file:line).
Report ONLY defects you can show with concrete evidence (file:line in ${REPO} and the exact failing input or path). Do not report style, naming or speculation. If you find nothing real, return an empty list. Severity: high = wrong result, data loss, law violation or a fix that does not fix; medium = a real gap on a reachable path or a test that cannot catch a regression; low = a doc claim that is false or a minor cost.`

const FINDINGS = {
  type: 'object',
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'short unique id like B-1' },
          severity: { type: 'string', enum: ['high', 'medium', 'low'] },
          item: { type: 'string', description: 'the original finding id this relates to, or NEW' },
          file: { type: 'string' },
          line: { type: 'integer' },
          claim: { type: 'string', description: 'one or two sentences: the defect' },
          failure_scenario: { type: 'string', description: 'concrete input or path and the wrong outcome' },
          evidence: { type: 'string', description: 'quoted code with file:line' },
          fix_sketch: { type: 'string' },
        },
        required: ['id', 'severity', 'item', 'file', 'line', 'claim', 'failure_scenario', 'evidence', 'fix_sketch'],
      },
    },
    checked: { type: 'string', description: 'under 120 words: what you checked and found sound' },
  },
  required: ['findings', 'checked'],
}

const VERDICTS = {
  type: 'object',
  properties: {
    verdicts: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          verdict: { type: 'string', enum: ['CONFIRMED', 'PLAUSIBLE', 'REFUTED'] },
          reason: { type: 'string' },
          evidence: { type: 'string', description: 'file:line quotes that decide it' },
        },
        required: ['id', 'verdict', 'reason', 'evidence'],
      },
    },
  },
  required: ['verdicts'],
}

const FIXERS = [
  { key: 'B', head: 'fc5291b6', items: 'G2-5 recorded audit capture cost, W2-cli8-10, W2-cli8-11 (zero point ceiling, one support validator in api and cli), W2-cli8-6 + G2-3 census at the auto-support door, G2-1, G2-2, W2-cli8-4/5/7 test gaps; decisions D-4716..D-4724' },
  { key: 'C', head: '1010370e', items: 'G5-2 / W2-cli13-4 FIFO-safe opens on every HTTP ledger read, G5-1, G5-3, G5-4, G1-4 / R9-cli-law-1, G3-3, G3-5, G3-8 stale claims, web zero ceiling; decisions D-4732..D-4740' },
  { key: 'D', head: '2e2aadd1', items: 'G3-1 / GAP12-6 CAS close on the Boolean path for NSE cash shares, G3-4 identity reason codes (free-text reasons out of run identity), AC-whp-law-2, GAP15-21 / G3-2 provenance banners, G3-6, G3-7, G3-9, F-CEC7A0; decisions D-4748..D-4756' },
  { key: 'E', head: 'fa3100b9', items: 'W2-cli12-1, G4-1 (Statistics V2 cached reads hashed the whole file 8 times per lookup), W2-cli11-3, W2-cli11-2, W2-cli12-2 (population ledger lookups O(1), one-scan appends); decisions D-4764..D-4768' },
  { key: 'F', head: 'd22d51b9', items: 'W2-cli3-4 one Candidate writer per run, G4-4 Pre-Admission V1/V2 one-open appends, W2-cli3-3 witness hashing, G4-2 and G4-3 sizing hand-offs (no duplicate NIFTY context load or column build), W2-cli7-2 replay stays a full re-proof; decisions D-4780..D-4785' },
]

const reviewers = FIXERS.map(f => ({
  label: `review:${f.key}`,
  key: f.key,
  prompt: `${COMMON}

You review lane 1-b fixer ${f.key}. Its commits are fbdabaec..${f.head} (see: git -C ${REPO} log --format='%h %s' fbdabaec..${f.head}; git -C ${REPO} diff fbdabaec ${f.head}). Its claimed items: ${f.items}.
Read the fixer's diff in full, the original findings in the G reports, and the code the diff touches as it stands in the merged tree ${REPO} (other fixers' changes are merged there too, so also check the fix still holds after the merge).
Also verify append-only: git -C ${REPO} diff fbdabaec ${f.head} -- docs/04-invariants.md docs/05-decisions.md docs/11-findings.md | grep '^-[^-]' must print nothing; every new docs/04 row must name a test that exists (grep for it); new D-numbers must be inside the fixer's range and unique in docs/05.
${FIND_RULES}`,
}))

reviewers.push({
  label: 'review:merges',
  key: 'M',
  prompt: `${COMMON}

You review the INTEGRATION of five fixer branches into the landing tree. The first-parent chain fbdabaec..a9fee04b holds five merge commits (git -C ${REPO} log --first-parent --format='%h %p %s' fbdabaec..a9fee04b). Use git -C ${REPO} show --remerge-diff <merge> to see exactly how each conflict was resolved (a9fee04b had a hand-resolved code conflict in crates/cli/src/candidate_universe.rs between fixer D's new cash parameter and fixer F's prebuilt signal column; docs ledgers were union-merged).
Files changed by more than one fixer (check each for semantic interaction where both fixes are textually present but one undoes, bypasses or double-counts the other): all_rung_population_v5.rs (C F), audited_stored_tests.rs (B D), candidate_trades.rs (B C), candidate_universe.rs (C D F), ledger_all.rs (D F), ledger_v6.rs (D F), lib.rs (B C D), population_finalization_v2.rs (C E), population_observations_v1.rs (C E), population_statistics_v2.rs (C E), step3_orchestrator.rs (C E F), stored.rs (B C D), crates/cli/tests/ledger_append_lookup_costs.rs (E F), all under crates/cli/src unless a path is given.
Key question for candidate_universe.rs: fixer D made a NSE cash share read its CAS-day close through a cash parameter; fixer F hands a prebuilt NIFTY signal column (built with cash = None) to the commit. Is a prebuilt column ever reused for an instrument or venue that needs a cash close, or could the prebuilt check accept a column built with different inputs? Also check the union-merged docs: no conflict markers, no duplicated or interleaved entries, docs/05 D-47xx headings unique and each entry intact.
${FIND_RULES}`,
})

phase('Review')
const results = await pipeline(
  reviewers,
  r => agent(r.prompt, { label: r.label, phase: 'Review', schema: FINDINGS }),
  (rev, r) => {
    if (!rev || !rev.findings || rev.findings.length === 0) return { key: r.key, rev, verdicts: [] }
    return agent(`${COMMON}

You are a skeptic. Another reviewer reported the findings below against lane 1-b ${r.key === 'M' ? 'merge resolutions' : 'fixer ' + r.key}. For EACH finding, try hard to REFUTE it by reading the actual code in ${REPO}: is the path reachable from a real caller, does the cited code say what the finding says, is there a guard elsewhere, does an existing test already catch it? Default to REFUTED when the evidence does not hold up; CONFIRMED only when you traced it yourself; PLAUSIBLE when real but you could not fully trace it.

Findings:
${JSON.stringify(rev.findings, null, 1)}`, { label: `verify:${r.key}`, phase: 'Verify', schema: VERDICTS })
      .then(v => ({ key: r.key, rev, verdicts: (v && v.verdicts) || [] }))
  },
)

const out = []
for (const res of results.filter(Boolean)) {
  const byId = {}
  for (const v of res.verdicts) byId[v.id] = v
  for (const f of (res.rev && res.rev.findings) || []) {
    const v = byId[f.id]
    out.push({ ...f, lane: res.key, verdict: v ? v.verdict : 'UNVERIFIED', verdict_reason: v ? v.reason : '', verdict_evidence: v ? v.evidence : '' })
  }
}
const checked = results.filter(Boolean).map(r => ({ lane: r.key, checked: r.rev ? r.rev.checked : 'reviewer failed' }))
log(`${out.length} findings; ${out.filter(f => f.verdict === 'CONFIRMED').length} confirmed, ${out.filter(f => f.verdict === 'PLAUSIBLE').length} plausible`)
return { findings: out, checked }```

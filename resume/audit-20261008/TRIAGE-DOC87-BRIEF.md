# Triage brief: the 87 "documented only" findings (untracked)

Owner rule (Parthi, standing since 2026-10-03): every finding is fully fixed.
"Documented only" is not done. A limit may stay ONLY where it truly cannot be
removed (an inherent bound: combinatorial search, an external vendor rate, a
per-byte checksum, a third-party dependency fact), and then it must carry a
MEASURED number (p50/p99/max, n, load average) from a committed test or bench.

## Input
- The list: `python3 -c` over `/tmp/claude-0/audit/data/findings.json`, rows
  with `after == "DOCUMENTED"` (96 rows, 87 unique ids). Fields: id, set,
  crate, severity, problem_plain, now_plain, evidence.
- Evidence: `/tmp/claude-0/audit/out/{so1,sobs,srust,r3,r53,r64,rnew}.md` and
  `.tsv` (grep the id).
- Code and docs: read-only worktree `/home/claude/integ` (audit/integ,
  a7a27dc3). Look at `docs/06-limits.md`, `docs/04-invariants.md`,
  `docs/05-decisions.md`, the named code, benches under `crates/*/benches`
  and tests named in the evidence.
- Work already assigned elsewhere (mark OVERLAP with the owner, do not plan
  twice): the briefs in this folder's sibling notes are summarised here:
  - intu (api, cli): api clippy fixes, cli cost model v3 switch,
    telemetry::sync at exit, one stderr authority, W1-api1-4 placeholders,
    W2-cli10-0.
  - intl (runner engine vocab pull core store lake telemetry): fxr WIP
    (satk-4,7,8, r64-3, c4a-6, closure label), so1-2, so1-6, fxd WIP (so1-1,
    so1-3, W3-engine1-1, o1engine-22/23, GAP16-26), rnew-2, r53-2, srust-6,
    sobs-17, r64-2, sobs-19, W3-runner2-5.
  - defc (cli): AC-whp-o1-1, cli-14, W2-cli12-3/4, W2-cli6-0, W2-cli2-5,
    W2-cli14-1/2/3, W3-runner2-3.
  - defp (pull store core): satk-3, W1-pull3-4, W1-pull1-0, rnew-3, o1api-33.

## For each unique id decide one verdict
- `FIX`: a real fix exists that makes the operation O(1) per call (or bounded
  by a named compile-time constant), or adds the missing log/flush/page.
  Give: files, the approach in one or two sentences, the test that proves it
  (count of reads/opens/allocs, or a behaviour assertion), and size S/M/L.
- `KEEP-MEASURED`: inherent, and a committed test/bench already records
  p50/p99/max with n and load average. Cite file:line of the measurement in
  docs/06-limits.md and the bench/test name. Say in one sentence why it cannot
  be removed.
- `KEEP-MEASURE`: inherent but no committed measurement yet. Name the bench or
  test to write and the sizes to run.
- `ALREADY-FIXED`: the code at a7a27dc3 already removed the cost (cite the
  commit/decision). Example to check: W3-engine1-2 and engine-02 (keep::Best
  removed by D-4480).
- `OVERLAP`: covered by intu/intl/defc/defp above (name which).
Be strict: "the doc says so" is not a measurement; a figure from an
uncommitted scratch run is not committed. Never guess: if you cannot tell,
write `UNVERIFIED` with what you would need.

## Output
1. `/tmp/claude-0/audit/out/doc87.tsv`: columns
   `id	crate	verdict	why	plan	test	size	files	measure_ref`.
2. `/tmp/claude-0/audit/out/doc87.md`: counts per verdict, then three work
   packages ready to hand to fixers, each a numbered item list with id, plan,
   test, files:
   - WP-api: api items (incl. BD-5, BD-6, BD-7, BD-11, crate-spawns-production).
   - WP-pull-store: pull, store, lake, core items.
   - WP-lower: engine, vocab, telemetry, cli, workspace, deps, ci items.
   Put FIX items first, then KEEP-MEASURE.

## Rules
- Read only. No cargo, no builds, no edits to any git repository.
- Keep reading lean: grep, sed -n ranges.
- Final message: the counts per verdict, per package, and the paths. Under
  300 words.

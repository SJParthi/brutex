# Artifact agent brief (untracked)

Goal: fill the Dashboard artifact https://claude.ai/artifact/7cvC2yoWTHviHri8zz4TPx
with the audit comparison table. The reader is Parthi, who wants an extremely
easy table comparison any person can understand: each problem, its state
BEFORE, NOW (at 9b0614be, the PR 74 head the audit checked) and AFTER (the
combined fix branch), with the proof.

## Step 1. Read the artifact and its type instructions
- `Artifact` action `read` url above. Follow the Dashboard type's instructions
  exactly (dash/meta, datasets/<id>, files/index.html, `#dash-root`, `d3`
  global, `dash.data`/`dash.onData`/`dash.calc`, marking attributes,
  uploaded assets for data files, never publish a file_path page, do not
  render to verify).
- Load `ArtifactData` with ToolSearch (`select:ArtifactData`).

## Step 2. Build `after.tsv` from evidence (never guess)
Write `/tmp/claude-0/audit/data/after.tsv` (tab separated, header
`id	after	after_plain	fix`). One row per finding id that the fix
work changed. Sources, all local:
- fixer reports and pause notes: `/tmp/claude-0/audit/out/fx*-report.md`,
  `/tmp/claude-0/audit/out/fx*-pause.md`
- the integration tree `/home/claude/integ` (branch audit/integ, read only:
  never commit, checkout or edit there). Its `docs/05-decisions.md` entries
  D-3211..D-3224, D-3516..D-3522, D-4410..D-4418, D-4430..D-4443, D-4454,
  D-4460..D-4469, D-4480..D-4486, D-4500..D-4508 name the finding ids they
  fix. Use `git -C /home/claude/integ log --oneline fbdabaec..HEAD` for
  commit ids.
- A row may say `after=FIXED` only when a D-entry or commit in
  /home/claude/integ names that finding id AND a report says it is done.
  Work in progress (WIP), not started, or owner-only items get
  `after=OPEN` / `after=OWNER` with a plain one-line reason. Ids use the
  same spelling as the `id` column of `/tmp/claude-0/audit/out/*.tsv`.
- `after_plain` is one short plain-English sentence (no jargon, no ids)
  saying what a person would now see. `fix` is `D-xxxx, <short sha>`.
Then run `python3 /tmp/claude-0/audit/build-data.py` and report its counts.

## Step 3. Build `proof.json`
`/tmp/claude-0/audit/data/proof.json`: one row per standing rule of Parthi's,
fields `check` (key), `rule` (plain words), `what_checked`, `result`
(PASS / PASS WITH NAMED LIMITS / OPEN / PENDING), `proof` (file, count or
command output that backs it). Rules: Rust only outside web/; O(1) at p99
everywhere, naming what cannot be; saving, logging, tracking, monitoring;
extreme edge-case attacks; evidence for every claim; one final combined PR
(PR 74); final validation (fmt, clippy, tests, mutation) = PENDING for now.
Counts come from the out/*.tsv files (count rows, do not estimate).

## Step 4. Fill the artifact
- `dash/meta`: title "Attack audit: before, now, after".
- Upload findings.json and proof.json as assets, write `datasets/findings`
  and `datasets/proof` docs pointing at them.
- `files/index.html`: adapt `/tmp/claude-0/audit/index.html` (a draft written
  for this type). Must show: summary tiles (total problems, fixed after,
  still open, owner-only), a per-set table (set, count, before/now/after
  counts), the rules table from `proof`, and the full findings table with
  search, set filter, state filter and click-to-sort, with columns
  Set, Id, Severity, Crate, Problem (plain), Before, Now, After, What you see
  now (after_plain), Fix, Proof. State words shown as plain labels with
  colour (FIXED green, PARTIAL amber, NOT-FIXED/OPEN red, OWNER grey).
  Every number comes from a dataset via dash.calc or dash.data, none typed in.
- Keep it usable at phone width.

## Step 5. Leave a refresh recipe
Write `/tmp/claude-0/audit/ARTIFACT-REFRESH.md`: the exact ArtifactData calls
(collection/doc ids, fields) and asset upload steps to replace the two
datasets later, so the coordinator thread can refresh after the final merge
without re-reading the type.

## Rules
- Never touch any git repository except reading /home/claude/integ.
- No cargo builds (four other agents share four cores).
- Report back: the artifact url, the counts (rows, FIXED/PARTIAL/OPEN/OWNER
  after), any finding you could not place, and the refresh recipe path.

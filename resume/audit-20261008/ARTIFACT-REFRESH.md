# Refreshing the audit dashboard (untracked)

Artifact: https://claude.ai/artifact/7cvC2yoWTHviHri8zz4TPx (type Dashboard,
owned by the person; its content lives in its own store, written with the
`ArtifactData` tool, never by publishing a `file_path` page).

## What is in the store now (filled 2026-10-09 09:49 UTC)

| collection / doc_id | version | fields |
|---|---|---|
| `dash` / `meta` | 1 | `title`: "Attack audit: before, now, after" |
| `datasets` / `findings` | 1 | `title`, `description`, `source: {kind: "file", url: "/_blob/2a8235cc9ce8a78d1a46da0330c70148", name: "findings.json", format: "json"}`, `updated: {at, by}` |
| `datasets` / `proof` | 1 | same shape, `url: "/_blob/52bec98c9b7b12a099208ad2733a20d7"`, `name: "proof.json"` |
| `files` / `index.html` | 1 | `text`: the whole page (copy at `/tmp/claude-0/audit/index-live.html`) |

Asset ids: findings.json `2a8235cc9ce8a78d1a46da0330c70148` (sha256 9caf9d60...),
proof.json `52bec98c9b7b12a099208ad2733a20d7` (sha256 68429bcf...).

The page computes every number itself (`dash.calc` ids `totals`, `stages`,
`by_set`) from the two datasets. Refreshing data never needs a page edit.

## Row shapes the page reads

- findings.json: array of objects with `key` (unique, `set:id`), `set`,
  `set_title`, `kind`, `id`, `severity`, `crate`, `before`, `now`, `after`,
  `after_plain`, `fix`, `problem_plain`, `now_plain`, `evidence`.
  State words the page knows: FIXED, PARTIAL, OPEN, NOT-FIXED, NEW, OWNER,
  DOCUMENTED, HELD, UNVERIFIED, `-`. Another word still shows, uncoloured.
- proof.json: array of objects with `check` (unique key), `rule`,
  `what_checked`, `result` (PASS / PASS WITH NAMED LIMITS / OPEN / PENDING),
  `proof`. The headline sentence reads the `result` of `check` =
  `final-validation`, so keep that key.

## Recipe after the final merge

1. Rebuild the data (no cargo, nothing in any git tree is written):
   - Edit the mapping in `/tmp/claude-0/audit/build-after.py` (one `r(ids,
     state, plain sentence, "D-xxxx, sha")` line per id; FIXED only when a
     decision entry or commit on the merged branch names the id AND a fixer
     report says done). Then:
   - `python3 -I /tmp/claude-0/audit/build-after.py` (writes `data/after.tsv`)
   - `python3 /tmp/claude-0/audit/build-data.py` (writes `data/findings.json`, prints counts)
   - `python3 -I /tmp/claude-0/audit/check-after.py | tail -1` must print `bad 0`
     (every id exists, every cited D-entry and short sha exists on /home/claude/integ;
     point its two paths at the final tree if that is not /home/claude/integ)
   - `python3 -I /tmp/claude-0/audit/build-proof.py` (writes `data/proof.json`;
     edit its `proof` texts for the final validation and the PR 74 push, and its
     `result` words, from what was actually run)
2. Upload both files as new assets, one call each (a JSON file must go alone):
   - `Artifact` `{action: "publish", url: "https://claude.ai/artifact/7cvC2yoWTHviHri8zz4TPx", asset: true, file_path: "/tmp/claude-0/audit/data/findings.json"}`
   - `Artifact` `{action: "publish", url: "https://claude.ai/artifact/7cvC2yoWTHviHri8zz4TPx", asset: true, file_path: "/tmp/claude-0/audit/data/proof.json"}`
   - Keep each reply's `url` exactly as given (`/_blob/<new id>`).
3. Read the two dataset documents for their current `version`:
   - `ArtifactData` `{action: "get", url: <artifact>, collection: "datasets", doc_id: "findings"}`
   - `ArtifactData` `{action: "get", url: <artifact>, collection: "datasets", doc_id: "proof"}`
4. Point them at the new files in one batch, each pinned to the version read:
   ```
   ArtifactData {action: "batch", url: <artifact>, writes: [
     {op: "update", collection: "datasets", doc_id: "findings", if_version: <v>,
      data: {source: {kind: "file", url: "/_blob/<new findings id>", name: "findings.json", format: "json"},
             updated: {at: "<ISO UTC now>", by: "<who>"}}},
     {op: "update", collection: "datasets", doc_id: "proof", if_version: <v>,
      data: {source: {kind: "file", url: "/_blob/<new proof id>", name: "proof.json", format: "json"},
             updated: {at: "<ISO UTC now>", by: "<who>"}}}
   ]}
   ```
   (`update` merges; `source` is replaced whole because it is one field.)
   If a pinned write is refused, re-read that document and redo the write.
5. Optional, once nothing names them: remove the old files with
   `Artifact {action: "delete", url: <artifact>, path: "2a8235cc9ce8a78d1a46da0330c70148"}`
   and `... path: "52bec98c9b7b12a099208ad2733a20d7"` (or the ids in use before your refresh).
6. Leave `files/index.html` alone unless the page itself must change. To change
   it: `get` it, write the whole page as `{"text": "<page>"}` to a JSON file and
   `update` with `file_path` and `if_version`. No `<form>`, no `on...=`
   attributes, no storage, text via `textContent`; under 256 KB.
7. Do not render or screenshot the dashboard to check it.

## Known pending inputs for the next refresh

- `audit/int-l` (worktree /home/claude/wt-l) holds 1ee02e24 "runner: finish
  fxr's WIP: satk-8, satk-7, W3-runner1-3/c4a-7/AC-whp-cx-2, c4a-6, r64-3,
  satk-4 (D-4501..)", not merged into audit/integ at a7a27dc3; those eight rows
  are OPEN until it is merged and its report says done.
- Rows still OPEN as work in progress: so1-2 (D-4419), so1-6, W1-api1-4
  (D-4440, placeholders in docs/06-limits.md), W2-cli10-0 (D-4467), so1-1,
  so1-3, W3-engine1-1, o1engine-22, o1engine-23, GAP16-26 (D-4481..D-4486).
- PARTIAL by the decision's own words: sobs-13 (D-4411: api and cli mains must
  call `telemetry::sync()`), lookahead and r64-5 (D-4500: cli must switch to
  `printed_ohlcv_cost_model_id_v3`).
- proof.json `one-pr` and `final-validation` are PENDING; replace their
  `proof` text with the push and the real fmt/clippy/test/mutants results.

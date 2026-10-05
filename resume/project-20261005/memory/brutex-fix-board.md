---
name: brutex-fix-board
description: New-account Fix Board artifact URL, its Rust builder on fix-queue, and where threads write per-finding status (2026-10-04)
metadata:
  type: reference
  modified: 2026-10-04T07:18:57.573Z
---
Fix Board (one row per finding, owner + state checked against PR #74 in git): https://claude.ai/artifact/5Jr1kKxEUEmzZF9f19YfTi. Republish every refresh to this same URL; never make a new artifact for it. The old account's board (Air7S5kQkMHWGkqYws9dka) cannot be edited from this account.

- Builder: Rust crate `fixboard` at `resume/fix-board/builder/` on branch fix-queue (standalone, not a workspace member). Exact refresh command and the order it applies inputs: `resume/fix-board/README.md` (51e5e72a).
- Page source + catalog: `resume/fix-board/brutex-fix-board.html`; snapshot: `resume/fix-board/ledger-snapshot.json.md`; hand-kept inputs (PR #74 facts, CI checks, one line per workstream): `resume/fix-board/inputs/*.tsv.md`.
- Threads write per-item status to `/mnt/project-files/fix-board/status/<thread>.tsv` (header `id state commit note`, tab-separated; states found, fixing, branch, partial, pushed, green, doc). Seeded 2026-10-04 from the old copies.
- 2026-10-04 07:44 UTC (v2): 877 distinct findings (9 more rows are the same defect filed twice): 388 on PR #74, 238 being fixed / partly / on a branch, 12 written limits with the real fix queued, 239 not started. Two adversarial audits of every row found 17 defects; all fixed (fix-queue 539f740d). Hand-checked state changes go in `resume/fix-board/inputs/corrections.tsv.md` with evidence; duplicates in `same-as.tsv.md`.
- 2026-10-04 (v3, fix-queue 3f3337d2): builder reads the live `/mnt/project-files/zero-rounds/` and the helpers' verification tables (highest pass wins). 893 distinct: 389 on PR #74, 279 being fixed/partly/on a branch, 12 written limits, 213 not started. No helper check contradicts an on-PR row. 2 rows have no area.
- 2026-10-04 09:2x (fix-queue ae8d2599): 991 distinct: 404 on PR #74, 382 fixing/partial/branch, 13 doc, 192 not started. run.sh at /tmp/claude-0/run.sh passes `--checked-at c97ff00` (a newer on-head status commit lifts a correction). Status states "fixed" read as branch, "operator"/"held" as found. Fix Board's own fixer branches fixboard/zero-p5 (7b9e6124, D-2700..2703, FB-01..03) and fixboard/zero-num56 (93283ef6, D-2710..2714, FB-11..15) were handed to zero-findings for comparison (overlapped its work).

Related: [[brutex-user-rules]].

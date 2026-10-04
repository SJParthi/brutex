# Fix Board

Live board: https://claude.ai/artifact/5Jr1kKxEUEmzZF9f19YfTi (new account, first published 2026-10-04 07:18 UTC).
Republish every refresh to that same URL. The old account's board (Air7S5kQkMHWGkqYws9dka) is frozen at v24.

## What is here

| Path | What it is |
|---|---|
| `brutex-fix-board.html` | The page. Its `ledger-data` block is also the catalog the next build starts from, so rows are never dropped. |
| `builder/` | `fixboard`, the Rust builder (standalone crate, not a workspace member). Replaces the lost Python scripts. |
| `inputs/*.tsv.md` | Hand-kept inputs: PR #74 facts, its CI checks (from GitHub), and one line per workstream. |
| `ledger-snapshot.json.md` | State of every finding at the last refresh; the next refresh reads it for `prev` and the "not moved" flag. |

## What the builder does, in order (each later step wins)

1. Catalog = the page's own ledger rows (779 at v20, 886 at the first rebuild).
2. Re-reads the growing finding files under `resume/project-files-20261004/zero-rounds/` (concurrency + every conc-pass dir,
   crash-edge headings and `- **CE-n**` bullets, tests-docs-security `P*-nn-nn`, numeric tables with a `sev` column,
   round-1 slice files `sliceNN.md` as `Z1-sliceNN-Fn`) and appends any id the catalog lacks.
3. Applies the previous snapshot's states.
4. Applies each `--status` TSV (`id<TAB>state<TAB>commit<TAB>note`), in the order given. Compound ids
   (`A / -8`, `D-1631 (W2-cli1-2/-3)`, `W2-cli14-1/2/3`) expand; `conc:` and `num:` prefixes are tried. A finding named
   only in a status file gets its own row (group "Named only in a status file"), so nothing is dropped.
5. Git evidence: any commit named in a status line that is an ancestor of `origin/final/all-fixes` makes the row
   `pushed`; a `pushed` claim whose commits all exist and none is on the head is demoted to `branch` and flagged.
6. If the `ci-ok` row of `inputs/checks.tsv.md` passed, `pushed` becomes `green`.

Re-running with the same inputs gives a byte-identical page and snapshot (checked at the first rebuild).

## Refresh

```
git fetch origin '+refs/heads/*:refs/remotes/origin/*'
cd resume/fix-board/builder && cargo build --release && cd ../../..
P=resume/project-files-20261004
resume/fix-board/builder/target/release/fixboard \
  --board resume/fix-board/brutex-fix-board.html --out resume/fix-board/brutex-fix-board.html \
  --snapshot resume/fix-board/ledger-snapshot.json.md --snapshot-out resume/fix-board/ledger-snapshot.json.md \
  --status $P/fix-board/status/lane1.tsv.md --status $P/fix-board/status/sweep.tsv.md \
  --status $P/fix-board/status/attack-audit.tsv.md --status resume/zero-audit-20261003/zero-findings.tsv.md \
  --status /mnt/project-files/fix-board/status/<each live file>.tsv \
  --zero-dir $P/zero-rounds --repo . \
  --checks resume/fix-board/inputs/checks.tsv.md --pr resume/fix-board/inputs/pr.tsv.md \
  --streams resume/fix-board/inputs/streams.tsv.md --as-of "$(date -u '+%Y-%m-%d %H:%M UTC')"
```

Threads in the new project write per-item status to `/mnt/project-files/fix-board/status/<thread>.tsv`
(header `id	state	commit	note`; states found, fixing, branch, partial, pushed, green, doc). Those files were seeded
from the copies above on 2026-10-04 and are passed last, so they win.

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

1. Catalog = the page's own ledger rows. Rows are never dropped. Each row keeps `base_note`, the note it first came
   with, and every run rebuilds `note` from it, so a run over its own output changes nothing.
2. Re-reads the growing finding files under `--zero-dir` (the live `/mnt/project-files/zero-rounds/` when it exists, else
   the copy in `resume/project-files-20261004/zero-rounds/`): concurrency + every conc-pass dir, every `crash-edge*.md`
   (headings and `- **CE-n**` bullets), every `tests-docs-security*.md` (`P*-nn-nn` and `P4-nn`), `numeric-pass*.md`
   headings as `num:` ids, numeric tables with a `sev` column,
   round-1 slice files `sliceNN.md` as `Z1-sliceNN-Fn`) and appends any id the catalog lacks. Severity is a whole word
   from the heading, else the section's `Severity:` line, else (bullets) the range heading's; `area` is the first
   source path in the finding's own section; a bare `name.rs:line` is placed in its crate when exactly one file of that
   name exists under `crates/` on the PR head; failing both, the first `docs/`, `.github/`, `CLAUDE.md` or `AGENTS.md`
   path in the section, then in the title or first note. Blank catalog fields are filled, set ones never overwritten.
3. Starts every row from `base_state`, the state it had on the old board's last snapshot (v24, 2026-10-03 22:44 UTC), never from a state an earlier run derived. The snapshot passed with `--snapshot` only supplies `prev` ("moved since last refresh").
4. Applies each `--status` TSV (`id<TAB>state<TAB>commit<TAB>note`), in the order given. Compound ids
   (`A / -8`, `D-1631 (W2-cli1-2/-3)`, `W2-cli14-1/2/3`) expand; `conc:` and `num:` prefixes are tried; an entry wrapped
   over two lines (unbalanced parentheses) is joined. A finding named only in a status file gets its own row. Commit
   hashes are read from the commit column only, and a later line never lowers a state an earlier line backed with a commit.
5. `--corrections` (`inputs/corrections.tsv.md`, id/state/commit/note/title): hand-checked states, each with its evidence
   in the note. They win over every status line and are never promoted by step 7.
6. Git: a status commit that is an ancestor of `origin/final/all-fixes` makes the row `pushed`; a `pushed` claim whose
   commits all exist and none is on the head is demoted to `branch` and flagged. `--not-a-fix` hashes (the audit bases
   331b05c, 1087e54) are never evidence.
7. A commit in `--base-ref..head` (331b05c..head, newer than every audit) or a decision entry added to
   `docs/05-decisions.md` in that range that names the finding on a line reading as a fix makes it `pushed`. Lines and
   decision titles with a not-a-fix phrase (`NOT_A_FIX` in main.rs: stated, documented, blocked, still open, ...) do not count.
8. `--same-as` (`inputs/same-as.tsv.md`): the same defect filed twice. Both rows take the further state, the duplicate
   gets `same_as`, and the page counts the pair once.
9. The helpers' verification tables (any table under `--zero-dir` whose first column is `id`/`ce` and which has a
   `state` or `verdict` column, each a file-and-line check at a named head) are quoted in the note. When two passes
   checked one finding, the highest pass number in the file or directory name decides: FIXED moves `found`/`fixing` to
   `branch`, PARTIAL moves `found` to `partial`, NOT FIXED on a `pushed`/`green` row adds the `audit-disagrees` flag.
   Corrected rows keep their state.
10. If the `ci-ok` row of `inputs/checks.tsv.md` passed, `pushed` becomes `green`.

Checked: 12 unit tests, clippy `-D warnings`; a second run over its own output gives identical states, notes and counts (only `prev` moves, as it should). Two adversarial audits of every row against the sources and git found 17 defects in total; all are fixed. Since then the bare-file and document fallbacks leave 2 of 893 findings with no `area` (D-0742-block and
h-cli-4 name no file at all).

## Refresh

```
git fetch origin '+refs/heads/*:refs/remotes/origin/*'
cd resume/fix-board/builder && cargo build --release && cd ../../..
P=resume/project-files-20261004
ST=""; for f in /mnt/project-files/fix-board/status/*.tsv; do ST="$ST --status $f"; done
resume/fix-board/builder/target/release/fixboard \
  --board resume/fix-board/brutex-fix-board.html --out resume/fix-board/brutex-fix-board.html \
  --snapshot resume/fix-board/ledger-snapshot.json.md --snapshot-out resume/fix-board/ledger-snapshot.json.md $ST \
  --corrections resume/fix-board/inputs/corrections.tsv.md --same-as resume/fix-board/inputs/same-as.tsv.md \
  --not-a-fix 331b05c --not-a-fix 1087e54 --base-ref 331b05c \
  --zero-dir /mnt/project-files/zero-rounds --repo . \
  --checks resume/fix-board/inputs/checks.tsv.md --pr resume/fix-board/inputs/pr.tsv.md \
  --streams resume/fix-board/inputs/streams.tsv.md --as-of "$(date -u '+%Y-%m-%d %H:%M UTC')"
```

Threads in the new project write per-item status to `/mnt/project-files/fix-board/status/<thread>.tsv`
(header `id	state	commit	note`; states found, fixing, branch, partial, pushed, green, doc). They were seeded on
2026-10-04 from `resume/project-files-20261004/fix-board/status/` and `resume/zero-audit-20261003/zero-findings.tsv.md`.

## First rebuild, counted

886 rows = the v20 page's 779 + 40 the v24 snapshot already had + 67 that no earlier board listed
(44 zero-loop round-1 slice findings, 9 numeric pass 2/3, 9 crash-edge CE-18..22 and CE-36..39, 5 named only in a
status file). Correction to commit 51e5e72a's message, which called all 107 additions new to v24; 67 were.

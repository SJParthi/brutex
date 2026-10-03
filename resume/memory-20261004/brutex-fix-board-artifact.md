---
name: brutex-fix-board-artifact
description: URL, data layout and per-finding ledger pipeline of the combined Brutex Fix Board comparison artifact (2026-10-03); republish to the same link
metadata:
  type: reference
  modified: 2026-10-03T13:24:22.054Z
---
Combined comparison-table artifact for all Brutex work: https://claude.ai/artifact/Air7S5kQkMHWGkqYws9dka ("Brutex Fix Board", from the "Overall comparison table Artifact" thread). Never make a new artifact for it; republish to this URL.

Page data: JS arrays at the top of the main script (AS_OF, GROUPS, STREAMS with agents counts, FIXPROG, PR, CHECKS, KNOWN, SWEEP_TOP, SWEEP_AREAS, O1).

Per-finding ledger (v12, 13:35 UTC 2026-10-03; user said "nothing should ever be missed"): 779 findings, one row each with owner + state (found, fixing, branch, pushed, green, doc, partial), flags no-owner / stuck (state unchanged since previous snapshot) / new.
- Builder: scratchpad build_ledger.py (sources copied to scratchpad src/ from fix-queue resume/audit-20261003/out/*.md, resume/lanes/c4-missing-findings.md, brutex-audit-ledger.html; sweep const F in /mnt/project-files/audit-20261003-workspace/brutex-workspace-audit.html; helpers' /mnt/project-files/zero-rounds/*.md and conc-pass2/*.md). Run `build_ledger.py "<time>" --commit` then inject_ledger.py (markers LEDGER-START / LEDGER-JS-START in the board).
- Shared snapshot: /mnt/project-files/fix-board/ledger-snapshot.json. Threads may write per-item status to /mnt/project-files/fix-board/status/<thread>.tsv (id, state, commit, note); the builder applies them.
- If the scratchpad is lost, rebuild the builder from this description.

Related: [[audit-20261003-state]], [[resume-20261003-pr74-state]].

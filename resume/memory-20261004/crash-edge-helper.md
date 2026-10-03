---
name: crash-edge-helper
description: Crash/edge-input audit helper thread state (2026-10-03): passes done, findings file, what to run next
metadata:
  type: project
---
Helper thread "Audit helper: crashes and edge inputs" for [[zero-audit-loop]]. Read-only audits; never edits the repo.

- Findings: /mnt/project-files/zero-rounds/crash-edge.md (CE-1..CE-41). Snapshot + resume header on fix-queue: resume/zero-rounds/crash-edge.md (1644b23).
- Passes: 1 (20 agents, 331b05c) CE-1; 2 (21 agents) CE-2..22; 3 (5 themes, final/all-fixes-zero 5140aca) CE-23..35; 4 CE-36..41 (HTML sinks and durable appends: 0 new). All CE-1..41 sent to the zero-findings session (session_019avWwsbRWYrj7eHCev5fVr), which assigned fixers and will report the new head.
- Stopped 18:35 UTC on the coordinator's usage stop (weekly 86%).
- Next: pass 5 theme "same rule implemented twice that drifted" (stopped at launch), then re-audit every CE fix on the new head; 2 agents max; stop when a pass finds nothing new.
- Fact: workspace denies unwrap/expect/panic/indexing; release has overflow-checks=true + panic=abort, so unchecked arithmetic is the crash route.

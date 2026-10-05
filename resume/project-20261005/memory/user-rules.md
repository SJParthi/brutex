---
name: user-rules
description: Parthiban's standing rules for all Brutex work (one PR, Rust only, O(1), evidence, no asks, usage limits)
metadata:
  type: feedback
  modified: 2026-10-04T07:09:26.853Z
---

Owner: Parthiban Subramanian (GitHub SJParthi), repo SJParthi/brutex. Resumed on a new Claude account 2026-10-04; only GitHub carried over.

- **One PR only:** PR #74 (branch `final/all-fixes`) is the one combined PR. Open no other PR. If #74 has merged, open exactly ONE new combined PR. Merge commits only; never force-push or rebase shared branches.
- **Rust only**, except the `web/` front end (CLAUDE.md §2).
- **O(1) wherever possible**; name anything that can't be. Measure p50/p99/max; never claim a guarantee.
- **Verify with real evidence; never guess.** Attack extreme edge cases adversarially. Work in parallel.
- **Fix every finding fully**, nothing "documented only". Audit rounds repeat until a whole round finds zero. Only facts/settings only the user can give stay open, and they are NAMED once.
- **Never ask the user to tap, paste or approve anything.** Keep work running continuously.
- **Usage:** save resume state to GitHub branch `fix-queue` (under `resume/`) at 93% weekly usage; stop at 98%.
- Deliver results as easy comparison-table Artifacts that anyone can understand at a glance.
- 2026-10-04 07:14 UTC standing rules (all workstreams): deep, thorough research using agents/subagents in parallel; no hallucination, only verified working results; attack everything for extreme worst-case permutations, combinations, errors and out-of-the-box scenarios; O(1) everywhere; Rust only in the whole workspace except web/ (check every corner); everything persisted, audited, logged, searchable, monitorable and visible on a dashboard while staying O(1); a common, dynamic, incremental, scalable runtime approach.
- **GDFL:** never push real GDFL data or any GDFL branch with GDFL rows in history. GDFL build runs only on the user's Mac at /Volumes/WD_BLACK/brutex/fresh-20260919/project. Never touch ~/IdeaProjects/brutexOld; never delete/move raw data on WD_BLACK unasked.
- Already approved (don't ask again): F6 = W2-cli5-4, W3-runner2-8 (column digest v2, keep v1 by name), W3-runner2-7, W3-runner5-0; GAP15-19 and GAP15-17 fixed for real with a decision entry each.

**Why:** stated by the user in the 2026-10-04 resume message and the old project's memory.
**How to apply:** every thread, every push. See [[brutex-resume-facts]] and [[brutex-ci-gotchas]].

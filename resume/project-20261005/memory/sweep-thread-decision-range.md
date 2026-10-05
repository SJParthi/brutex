---
name: sweep-thread-decision-range
description: Decision-number range the "Rust and O(1) sweep" thread uses (D-2300..D-2399), set by the coordinator 2026-10-04
metadata:
  type: project
  modified: 2026-10-04T07:24:12.940Z
---

The "Rust and O(1) sweep" thread writes new docs/05-decisions.md entries only in D-2300..D-2399 (coordinator, 2026-10-04 07:15 UTC). Zero-findings owns D-2500..D-2699 and invariant prefixes ZC/ZK/ZQ/ZX. Gate 27b refuses duplicate D-numbers across the combined PR #74.

**Why:** many threads push to final/all-fixes (PR #74) at once; overlapping numbers collide at merge.
**How to apply:** pick the next free number in your range by grepping docs/05-decisions.md on origin/final/all-fixes before writing; use a fresh invariant prefix (this thread: AFG-).

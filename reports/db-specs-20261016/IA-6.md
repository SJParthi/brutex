# IA-6: pull::gdfl_verify: the per-day oracle, compare and completeness sweep

Prerequisites (must be merged into feat/db-fix first): IA-16, IA-4, IA-5, WP-12e, WP-12f

## Specification

```text
ROUND 3 ADDITION (12 Oct): Leftovers your in-app check replaces: the hand checker's late-row rule and count (DB-A05, DB-A19, DB-A24), its file match by stem that compares a share against a bond file (DB-A25), the stale completeness rows of the board (DB-A06), and a record for a file that was read and yielded no in-session bar (DB-A22). State in your report that the old non-Rust reconcile and completeness files in state/gdfl-board/ are retired; they are outside the repository and nobody else owns them. ANSWERS: DB-A05; DB-A19; DB-A22; DB-A24; DB-A25
```

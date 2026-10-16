# IA-2: locks, the runs ledger, claims, the two-phase start and the whole stop

Prerequisites (must be merged into feat/db-fix first): IA-1, IA-11a, WP-16, WP-12f, WP-81

## Specification

```text
ROUND 3 ADDITION (12 Oct): Leftovers DB-A50 and DB-A41: add a clock time (at=) to every closing line and a journal line for a run that ended or was stopped; the runs ledger gives the run id that the log lines carry. This is yours alone: R3-23 adds the end record of every run, the kept request and rows refused by reason, and builds the readers; it does not write at= again. R3-35 splits gdfl_import.rs into five files before IA-1 starts; the parts your row names now live in those files (its report says which file holds which function). Edit them there and put no seam back into gdfl_import.rs. ANSWERS:
```

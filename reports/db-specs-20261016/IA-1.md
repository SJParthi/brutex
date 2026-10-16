# IA-1: pull::gdfl_run core: the one import authority

Prerequisites (must be merged into feat/db-fix first): WP-16, WP-11c, WP-12d, WP-12f, IA-11r, WP-81, R3-35

## Specification

```text
ROUND 3 ADDITION (12 Oct): Leftover DB-A41: every importer log line of a run (per day and per refusal) carries the run id from RunOptions, so /logs.json?run=<id> returns more than the start and finish lines. You now wait for R3-35, the move-only split of gdfl_import.rs (the audit's IA-00, row RA-142): make your seams in the file that holds each one, as its report lists them. ANSWERS:
```

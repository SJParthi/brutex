# IA-3: preflight, the source cache, zip faults, archive and file-name cases

Prerequisites (must be merged into feat/db-fix first): IA-1, IA-2, WP-16, WP-12e, WP-12f

## Specification

```text
ROUND 3 ADDITION (12 Oct): Leftover DB-A49: record the files a day skipped (a per-day count and a digest of the day's file listing at least), so a skipped file is counted for ever; for indices 2018-11-07 today 43 of 50 files appear nowhere. This is yours alone: R3-23 only reads and serves the count. R3-35 splits gdfl_import.rs into five files before IA-1 starts; the parts your row names now live in those files (its report says which file holds which function). Edit them there and put no seam back into gdfl_import.rs. ANSWERS:
```

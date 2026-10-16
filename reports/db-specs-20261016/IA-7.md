# IA-7: inline verification on the import workers; the end-of-run deferred step

Prerequisites (must be merged into feat/db-fix first): IA-6, IA-16, IA-11a, WP-16, IA-10a

## Specification

```text
ROUND 3 ADDITION (12 Oct): Nothing new to build, one thing to know. R3-35 splits gdfl_import.rs into five files before IA-1 starts; the parts your row names now live in those files (its report says which file holds which function). Edit them there and put no seam back into gdfl_import.rs. R3-23 and R3-26 edit gdfl_run.rs and the split files after you. ANSWERS:
```

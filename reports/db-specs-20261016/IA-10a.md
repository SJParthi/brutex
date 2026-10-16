# IA-10a: API, import half: /gdfl/* routes, the child mode, the alerts routes

Prerequisites (must be merged into feat/db-fix first): IA-2, IA-16, IA-3, IA-11a, IA-11r, WP-39, WP-81

## Specification

```text
ROUND 3 ADDITION (12 Oct): Leftovers for your api half: show an interrupted run and each closing line's time on /gdfl/import.json and /imports.json (DB-A50); return the run's own log lines by run id (DB-A41); mark a logged "day imported" that the journal closed incomplete as contradicted on /imports.json (DB-A48; R3-24 does the same on /logs.json); keep the day band's run lookup O(1) per D-4896 (DB-A34). R3-05, R3-08, R3-09, R3-10 and R3-14 add registration lines to server.rs before you, and R3-08 and R3-07 change the admission lines of its request middleware: rebase on them. R3-31 adds the alerts held in memory to your alerts route after you. ANSWERS: DB-A41; DB-A34
```

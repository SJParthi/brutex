# IA-9: pull::zerodha_verify, the spool and verify-merge

Prerequisites (must be merged into feat/db-fix first): IA-5, IA-8, IA-11a, IA-2, WP-13d, WP-62

## Specification

```text
ROUND 3 ADDITION (12 Oct): Leftovers DB-G46 and DB-A12: a month stored before the gap ledger existed gets its reasons from R3-18's stored-month ledger, not "no_ledger"; the class "neither vendor has it" is named for the 213 missing NIFTY minutes. ANSWERS: DB-G46; DB-A12 COMPLETENESS AUDIT 13 Oct (state/board/audit-result.json, read each id there): CMP-18: Defect · Zerodha bars that belong to another security (WP-13d (series identity), WP-13a (refuse non-positive CASH prices))
```

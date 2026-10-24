STATUS: IN PROGRESS

# AG-06b (cloud): pull runs never start unjournalled, and no tokio worker waits on the journal lock

Branch `cloud/db-AG-06b-20261024`, cut from `origin/feat/db-fix` e851da1ca (Merge DATA-2).
Spec: `reports/db-specs-20261024/AG-06b.md` on `origin/cloud/db-base-20261024`. GO seen 2026-10-24 09:08 IST.

Prerequisites: none hard. IA-10c is a FILES_ONLY link (not merged at e851da1ca): the start is not
blocked, the merge-back waits for it.

## Log

### 2026-10-24 09:20 IST
- Read README, CONTEXT, AG-06b spec in full. Branch created. First background build of `api` tests started.
- NEXT: AG06B-3 harness close; AG06B-1 refusal in `broker_run_by`; AG06B-2 `off_the_workers` inside `RunLedger`.

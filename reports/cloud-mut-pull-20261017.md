STATUS: IN PROGRESS

# Cloud mutation and coverage pass — `crates/pull`, 2026-10-17

Branch `cloud/mut-pull-20261017`, cut from `origin/feat/db-fix` at `f9cdb584`.
Continues `cloud/mut-pull-20261016` (report
`reports/cloud-mut-pull-20261016.md` on that branch), whose test work was
re-applied with `git diff c9e3472e5 origin/cloud/mut-pull-20261016 -- .
':(exclude)reports' | git apply -3`: every hunk applied cleanly, no conflict.

## Done

- Re-applied the 2026-10-16 tests onto the newest `feat/db-fix`.

## Left

- Build and run the re-applied tests against the new base.
- Every item of the old report's NOT DONE list.

## Gates

Not yet run on this branch.

## NEEDS THE MAC

Nothing yet.

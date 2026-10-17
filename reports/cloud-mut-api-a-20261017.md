STATUS: IN PROGRESS

# cloud-mut-api-a-20261017 — the api half-A mutation and coverage pass, carried onto the newest `feat/db-fix`

Branch `cloud/mut-api-a-20261017`, cut from `origin/feat/db-fix` at
`f9cdb584`. It carries the 16 Oct pass (`origin/cloud/mut-api-a-20261016`,
report `reports/cloud-mut-api-a-20261016.md` on that branch) and finishes its
NOT DONE list.

## Done so far

- Re-applied `git diff c9e3472e5 origin/cloud/mut-api-a-20261016 -- . ':(exclude)reports'`
  with `git apply -3`. Six files applied cleanly (`bars_change.rs`,
  `bars_series.rs`, `bars_sort.rs`, `gaps_route.rs`, `render.rs`, the new
  `tests/cloud_mutants_bars.rs`). One conflict, at the end of `bars.rs`:
  `feat/db-fix` added `mod unknown_series_tests` (AG-03) where the old pass
  added `mod cloud_mutant_tests`. Both are kept, in that order; nothing of
  `feat/db-fix` was dropped.

## Left

- Build and run the re-applied tests against the newer base.
- Every item of the old report's NOT DONE list.

## Gates

Not run yet.

## NEEDS THE MAC

Nothing yet.

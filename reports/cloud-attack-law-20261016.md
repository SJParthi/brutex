# cloud-attack-law-20261016 — the law, attacked lens by lens

Work in progress. Branch `cloud/attack-law-20261016` from
`origin/cloud/base-20261016` (c9e3472e, a snapshot of `feat/db-fix` at
4756cce0). The full tables per lens, the tests added and the gate results are
written into this file as each lens is verified.

## Fixed so far

| where | what | test |
|---|---|---|
| `crates/cli/src/srt_builder.rs` `name_refused` | `symlink_metadata(name).ok()?` read every lookup failure as "absent", so the dry run said `would build` where the build step refuses the month | `srt_builder::tests::a_dry_run_names_an_uninspectable_name_rather_than_reading_it_as_absent` |

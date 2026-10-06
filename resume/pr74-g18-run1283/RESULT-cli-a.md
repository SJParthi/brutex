# RESULT — cli-a (Gate 18 run 1283)

Branch `pr74/g18-cli-a`, head **7ad5c66e25f888cb1479f500fe83c7e51936f8f7** (pushed; no `changed by cargo-mutants` marker; no PR opened).
`origin/final/all-fixes` is still 969493e1, so no merge was needed.
Scope: the 73 survivors in survivors-cli-a.md, plus the shard-178 `results_at` survivor and the 4 timeouts in timeouts-cli-a.md (78 in all).
Decisions D-2002..D-2018 and invariants G18-cli-a-01..36 are in docs/05 and docs/04.

## Survivors closed: YES (every listed MISSED/TIMEOUT is caught, unviable, or no longer generated)

| item | fixed? | commit | evidence |
|---|---|---|---|
| 72 run-1283 MISSED (sites listed in survivors-cli-a.md) | yes | 0cf7480, bd9acfa, fed7c4b, 211c34b, 688da75 | cargo-mutants 26.2.0 `--baseline skip --cap-lints true`, filtered lib tests: run A 7 caught, run C 4, C2 30, C3 18, C4 55 caught + 6 unviable. **0 missed, 0 timeout** |
| results_at `>`→`>=` (shard 178) | yes | 157dc0e | caught (`the_omitted_row_line_starts_one_row_past_the_listing`, 1/39/40/41/42 rows) |
| first_accepted_in_order `&&`→`\|\|`, `<`→`<=` (TIMEOUT) | yes, restructured | ab3382b (D-2017) | counted loop. Both mutants are no longer generated, and every new mutant in the loop is caught or unviable |
| validate_from_env→true, validates→Some(true) (TIMEOUT) | yes | fcbecf6, 805abec (D-2018) | applied by hand, run non-root: 10 and 11 tests FAIL in 0.08s and 0.11s; no hang |
| Restructured equivalents (candidate_universe, pbo_ppm, FileIdentityV1::of, file_winner, best_shown, forced_stop, stop_rungs `pt > 0`, div_round signum, first_accepted guard, load_audit split, screen_swept wrapper, elite_ceiling_ppm, tick_with max) | yes | see D-2004..D-2016 | `cargo mutants --list -p cli` at head: none of the original survivor mutants is generated any more. Same-named mutants still listed are other, unchanged lines that were never survivors |
| stop_rungs_in_points ceiling `per_point - 1` → `+1` / `/1` (not run-1283 survivors; missed by my filtered set) | yes | da59859 | applied by hand, run non-root: `every_points_rung_is_a_point_or_more_and_within_the_ceiling` FAILS on both (exact-whole-point fixture) |

## Validation on 7ad5c66e25f888cb1479f500fe83c7e51936f8f7
- `cargo fmt --all --check`: clean.
- `cargo clippy -p cli --all-targets --locked -- -D warnings`: clean. A workspace clippy run before the last fix commit failed only in cli, and those 5 errors are fixed in 7ad5c66. Only cli was touched.
- cli tests, all 16 targets, non-root (`setpriv --reuid=65534`): every integration target passes. Lib: 1947 passed, 25 failed.
  The same binary set at base 969493e1, run the same way: 1919 passed, **the same 25 failed**
  (`comm` of the two sorted failure lists: no head-only failure, no base-only failure). These 25 fail on this box's
  non-root setup (e.g. `public_generated_probe_and_screen_agree_with_durable_results`: PermissionDenied, also at base).
  NOT a regression, but **not green here**. CI is the arbiter for them.
- Static gates: language-purity steps 1..30 except 1e, extracted from ci.yml: **all 29 rc=0**.

## Not done / UNVERIFIED
- p50/p99/max measurement of the restructured per-op paths (owner addendum rule 3): harness written
  (/tmp/claude-0/g18_measure.rs, not committed), **not run — UNVERIFIED**. Every restructure keeps the cost class (a constant
  arithmetic change, or one added O(len) select pass in first_accepted_in_order when top >= len), but that is unmeasured.
- Mutation runs used opt-level 1 and no debuginfo for speed (C3/C4), and filtered lib tests. They were not the full CI matrix.

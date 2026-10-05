# Data-path attack: resume state (paused 2026-10-05 ~02:00 UTC, weekly usage 95%)

## Branches (all pushed)
- `claude/attack-data-pipeline-hgxmw9` @ 0368dbb (off final/all-fixes 956424c). Main data path.
  Rounds 1-3 done: round 1 fixed 23 (D-3100..3158), round 2 fixed 2 (D-3180/3181),
  round 3 fixed round-2 open items (D-3182..3184), cleared static gates (D-3185..3189, 28 pass/0 fail),
  and found+fixed 4 (D-3123..3126). See r2-main.md, r3-main.md.
  NOT YET CONFIRMED after round 3: full `cargo test --workspace --locked` and workspace clippy
  (the round-3 agent was stopped while the workspace run was going). Run them first.
- `claude/attack-gdfl` @ d40c165 (off feat/gdfl-import 8e6b521). GDFL import.
  Rounds 1-4 fixed 21 (D-3160..3179, D-3190..3199; DPN-01..10, DPT-01..28). bda0464 is round 4, validated
  (gates 28/0, 252 tests pass as nobody). d40c165 is a WIP of round 5: unvalidated, do not hand it on as is.
  Round-5 D-numbers reserved: D-3141..3149, invariant rows DPT-29+.

## Next steps, in order
1. Main branch: run fmt, workspace clippy, `cargo test --workspace --locked`, static gates
   (gate runner notes: memory `brutex-ci-gotchas`). Fix anything red.
2. Main round 3 still found 4 -> run main round 4 (D-3127..3129, 3133..3139).
3. GDFL: finish round 5 from d40c165 (gdfl_r5_attack_tests.rs + gdfl_import.rs/ingest.rs edits),
   validate, then round 6 until a round finds zero.
4. Merge latest origin/final/all-fixes into the main branch with a merge commit; re-run checks.
5. Hand main branch to the PR 74 CI thread via the coordinator (only it pushes final/all-fixes).
   GDFL branch goes to the Mac GDFL thread to merge/port into its local feat/gdfl-import
   (which already has D-2802..2807; keep its D-2806, port our tests as behaviour checks).
6. Publish one comparison-table Artifact: attack | cases | failures | fixed | evidence file:line,
   per area and round, plus p50/p99 tables (in r*-*.md) and open items.

## Open items (need a sourced fact or Mac data)
- D-3113 Saturday expiry needs a sourced calendar.
- Shapeless undecodable names under nested filters (LT/LTI).
- Tick-store raw/index decode length has no sourced bound (UNVERIFIED).
- Do Mac journals hold `done` lines with no `definition=`? (they will re-import once).
- pull::calendar::FIRST_DAY is 2019-12-02 on pushed branches; Mac D-2805 fixes it.
- Store lookup by time is bisection O(log n) (owned by the Rust/O(1) sweep thread).

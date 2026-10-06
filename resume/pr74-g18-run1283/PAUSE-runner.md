# PAUSE — g18 runner (session for pr74/g18-runner), 2026-10-06

Head of `pr74/g18-runner`: **c216c97** (pushed; working tree clean; `git grep "changed by cargo-mutants"` prints nothing on it).
Base: origin/final/all-fixes 969493e1 (unchanged at pause time; final merge not yet done).

## Done (pushed)
- All 45 original survivors handled by a test or by restructuring so the mutant cannot exist
  (D-2055..D-2063, invariants G18-runner-01..17 in docs/04-invariants.md).
- 7 late survivors (shards 178-183): spa ceiling added to `the_ceiling_is_read_on_a_series_longer_than_it`
  (3664c49); envelope_extremes Ok((1,1))/Ok((-1,1))/Ok((-1,-1)) and validate_envelope_extremes Ok(()) expected
  killed by `a_training_slice_past_the_arithmetic_envelope_is_refused_at_attestation`; significance.rs:499 `-`->`+`
  expected killed by `ln_gamma_at_ten_is_the_unshifted_series` (bit-exact). EXPECTED = UNVERIFIED until step 2 below.
- 3 TIMEOUT mutants in ln_gamma: loop bounded to ten fixed steps, bit-identical to the old loop
  (D-2064, G18-runner-18, c216c97). UNVERIFIED by cargo-mutants until step 2.
- Local checks on 3664c49/c216c97: cargo fmt --check clean; cargo clippy -p runner --all-targets -D warnings clean;
  touched runner lib tests green as uid 65534; static gates (language-purity job, gate 1e skipped) all PASS on f097f06,
  gate 11 re-run PASS on c216c97.

## In progress at pause
- cargo-mutants run #2 (113 targeted mutants = CI-style --in-diff of my diff + all mutants on the survivor lines,
  copied tree, lib tests, nextest fail-fast), started on tree fe5c87a (before 3664c49/c216c97):
  at pause 103 caught, 4 unviable (Default-less return types), 0 missed, 0 timeout, 6 remaining.
  Output: scratchpad .../mut2/mutants.out (this container only). Its ln_gamma `<`-line results are stale
  (pre-c216c97) and may time out; that is expected and superseded by step 2.

## Next steps on RESUME
1. Read final counts of run #2 (`wc -l mut2/mutants.out/{caught,missed,timeout,unviable}.txt`); fix any missed.
2. Targeted run on c216c97 for: spa line (`if block > MAX_BLOCK` in `pub fn spa`), envelope_extremes /
   validate_envelope_extremes in exit_grid_policy.rs, every mutant in `ln_gamma` (significance.rs), with
   `cargo mutants --baseline skip --jobs 1 --timeout 900 --build-timeout 300 --cap-lints true -p runner
    --file <file> --re '<lines>' --test-tool nextest --cargo-arg=--lib --cargo-test-arg=--max-fail=1:immediate`
   (never --in-place in the commit checkout). Required: 0 missed, 0 timeout.
   If walk_forward_shaped->Default or grid.rs:2137 needs integration tests, rerun those without --cargo-arg=--lib.
3. Owner addendum: p50/p99/max before (969493e1) / after for grid::evaluate (one_variant went_for), Sides::observe,
   lentz_guard / ln_gamma, via an untracked timing test; record in RESULT (and docs/06-limits.md only if not O(1)).
4. Merge newest origin/final/all-fixes (merge commit), cargo fmt --check, clippy --workspace --all-targets -D warnings,
   runner tests as non-root, static gates; push pr74/g18-runner.
5. Write RESULT-runner.md here (table item | fixed? | commit | evidence).

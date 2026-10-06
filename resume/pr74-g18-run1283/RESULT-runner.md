# RESULT — g18 runner (pr74/g18-runner), 2026-10-06

Branch `pr74/g18-runner` head **c216c97** (pushed; contains origin/final/all-fixes 969493e1, which has not moved, so the
final merge is a no-op). No PR opened. Decisions D-2055..D-2064, invariants G18-runner-01..18.

## Evidence runs (this box)
- **M2**: `cargo mutants --baseline skip --jobs 1 --timeout 900 --cap-lints true -p runner --file <8 files> --re <113 exact names>
  --test-tool nextest --cargo-arg=--lib --cargo-test-arg=--max-fail=1:immediate` on fe5c87a (copied tree, not in-place).
  Targets = CI-style `--in-diff` of my diff + every mutant on the original survivor lines.
  Result: **108 caught, 0 missed, 1 timeout, 4 unviable**. The timeout was `significance.rs:509:13 < -> > in ln_gamma`
  (old `while` loop), superseded by c216c97. Unviable = `Some/vec![Default::default()]` on types with no `Default`.
- **M3**: same flags plus `--build-timeout 300`, on c216c97: spa ceiling line, all `envelope_extremes` replacements,
  `validate_envelope_extremes -> Ok(())`, and **every** mutant in `ln_gamma` (56 total).
  Result: **56 caught, 0 missed, 0 timeout, 0 unviable**.
- "cannot exist" = the mutant is absent from `cargo mutants --list -p runner` on c216c97 (code restructured, D-0192).
- Final checks on c216c97: `cargo fmt --check` clean; `cargo clippy --workspace --all-targets --locked -- -D warnings` clean;
  all 13 runner test binaries green as uid 65534 (lib 753 passed; integration 46 passed); static gates (language-purity job,
  gate 1e skipped) all PASS; `git grep "changed by cargo-mutants"` empty.

## Survivors (run 1283)
| item | fixed? | commit | evidence |
|---|---|---|---|
| audit.rs:237 `<`->`>` / `==` / `<=` largest_overnight_move | yes, restructured to `ratio.is_negative()` (cannot exist) + saturation test | aaa9fcb | absent from list; `an_unrepresentable_move_saturates_toward_its_sign` |
| audit.rs:238 `>`->`>=` largest_overnight_move | yes | aaa9fcb | M2 caught (245:54 >=) by `a_tie_in_magnitude_keeps_the_earlier_session` |
| audit.rs:258 rupees `<`->`<=`/`==`/`>` | yes | aaa9fcb | M2 caught (265:25 x3), `rupees_signs_only_a_negative_amount` |
| audit.rs:269 overnight_line `<`->`<=` | yes | aaa9fcb | M2 caught (276:29 <=) |
| audit.rs:1187 guard->true, `<`->`<=` in grid | yes, restructured to `cmp::min_by_key`/`max_by_key` (cannot exist) | aaa9fcb, f097f06 | absent from list; `two_chosen_rows_below_the_cut_print_in_key_order` |
| bootstrap.rs:175 Verdict::clears `>`->`>=` | yes | e593f98 | M2 caught |
| bootstrap.rs:429 reality_check `>`->`>=`/`==` | yes | e593f98 | M2 caught, `the_ceiling_is_read_on_a_series_longer_than_it` (1,000,001 periods) |
| bootstrap.rs:506 reality_check `||`->`&&` | yes, redundant clauses removed (cannot exist) | e593f98 | absent from list; zero-draw/one-period tests still green |
| bootstrap.rs:1232 romano_wolf `>`->`>=`/`==` | yes | e593f98 | M2 caught |
| bootstrap.rs:1257 romano_wolf_receipt `>`->`==`/`>=` | yes | e593f98 | M2 caught (block and alpha clauses) |
| bootstrap.rs:1323 romano_wolf_adjusted_p_values_v1 `>`->`>=`/`==` | yes | e593f98 | M2 caught |
| bootstrap.rs:573 spa `>`->`==`/`>=` (shard 178/180) | yes | 3664c49 | M3 caught (582:14 x3) |
| exit_grid_policy.rs:1455 AttestedTrainingV1 Debug -> Ok | yes | 34af157 | M2 caught, `an_attested_training_slice_debug_prints_its_identity` |
| exit_grid_policy.rs:4194 validate_arithmetic_envelope_view -> Ok(()) | yes | 34af157 | M2 caught, `a_training_slice_past_the_arithmetic_envelope_is_refused_at_attestation` |
| exit_grid_policy.rs:4201 envelope_extremes Ok((0,-1))/((1,-1))/((0,1)) + shard 178-182 ((1,1))/((-1,1))/((-1,-1)) | yes | 34af157 | M2 + M3 caught (all 9 replacements) |
| exit_grid_policy.rs:4222 validate_envelope_extremes -> Ok(()) (shard 183) | yes | 34af157 | M3 caught |
| grid.rs:2137 delete field signals in evaluate_timed | yes | 0a43ee1 | M2 caught, `an_invalid_stop_ladder_or_step_is_refused_and_named` |
| grid.rs:4576 `>`->`>=` one_variant | yes, `went_for` read on demand (cannot exist) | 0a43ee1 | absent from list |
| outcome.rs:1598 largest_gain_paisa `<`->`<=` | yes | 9ac4962 | M2 caught, `a_flat_mean_reads_the_largest_up_move_as_its_gain` |
| outcome.rs:1833 `<`->`<=` (min_win) | yes, folded with `min` (cannot exist) | 9ac4962 | absent from list; `==`->`!=` M2 caught |
| outcome.rs:1841 x5 (min_loss) | yes, folded with `min` + test | 9ac4962 | remaining `==`->`!=` M2 caught; others absent; `the_smallest_win_and_loss_...in_any_order` |
| significance.rs:444 x5 regularized_incomplete_beta | yes (bit-exact side test) | fbfcbbd | M2 caught all 15 mutants on the line |
| significance.rs:460 `<`->`<=`/`==` Lentz guard | yes, named `lentz_guard` + boundary test | fbfcbbd, fe5c87a | M2 caught (462:20 x3) |
| significance.rs:493 `<`->`<=` ln_gamma | yes | fbfcbbd, c216c97 | M3 caught (bounded loop `>=`->`<`), bit-exact `ln_gamma(10)` |
| significance.rs:499 `-`->`+` ln_gamma (shard 178) | yes | fbfcbbd | M3 caught (all series mutants) |
| trade.rs:651 refused_within `>`->`==`/`>=` | yes | 49563eb | M2 caught |
| validate.rs:2028 delete field refused | yes, complete literal (cannot exist) | 40852b4 | absent from list |

## Timeouts (run 1283, from shard logs)
| item | fixed? | commit | evidence |
|---|---|---|---|
| significance.rs:493:13 `<`->`>` ln_gamma (shard 154) | yes, loop bounded to 10 fixed steps, bit-identical | c216c97 | `while` gone; M3: every ln_gamma mutant caught, 0 timeout; `the_bounded_gamma_shift_is_the_unbounded_loop_to_the_bit` |
| significance.rs:495:11 `+=`->`-=` (shard 158) | yes | c216c97 | M3 caught (519:15, 520:11) |
| significance.rs:495:11 `+=`->`*=` (shard 159) | yes | c216c97 | M3 caught (519:15, 520:11) |

## Owner addendum: per-operation timing (p50 / p99 / max, ns; untracked probe `crates/runner/tests/zz_timing.rs`, 3 alternating rounds, base 969493e1 vs head)
| path | base (rounds 1-3 p50; p99; max) | head (rounds 1-3 p50; p99; max) |
|---|---|---|
| grid::evaluate per trade-variant (one_variant, `went_for`) | 316/328/339; 355-434; 404-500 | 334/332/329; 396-484; 404-507 |
| outcome::edge per observed bar (Sides::observe) | 61/61/61; 90-119; 108-163 | 94/57/54; 81-138; 96-163 |
| student_t_two_sided_tail per call (lentz_guard, ln_gamma) | 791/835/804; 1091-1151; 17244-29511 | 807/792/736; 993-1134; 18279-18970 |
All three paths are O(1) per operation before and after; the differences are within run-to-run noise on this box
(4 cores, shared). Nothing added to docs/06-limits.md, as nothing became non-constant. Timing evidence is from this box only.

## Not done / notes
- Mutation evidence used lib tests only (`--cargo-arg=--lib`); every listed mutant was caught there, so integration tests were
  not needed for any kill. The full suite was run separately and is green.
- Gate 11 refused two first attempts (a `.sort_*` in audit.rs, one extra `f64` line in significance.rs); both reworked (f097f06).

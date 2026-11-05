# result-PSRB-STV.json.md

The machine-read result of session PSRB-STV (CLOUD.md item 10). `head` is the code tip,
`git rev-list -1 HEAD -- . ':(exclude)reports'`. Detail: `report-PSRB-STV.md`.

```json
{
  "name": "PSRB-STV",
  "branch": "cloud/psrb-stv-20261103",
  "head": "c3c39a44bc23d6c08c806140f00d17444da9e490",
  "committed": true,
  "start": "35234cead4009299a93fcd79a3d1fffd773dcc16",
  "token": "PSRB-STV-20261103-1",
  "tests": {
    "file": "crates/indicators/src/dual_band_supertrend_tests.rs",
    "new": 23,
    "indicators": "527 passed, 0 failed, 0 ignored",
    "indicators_at_start": "504 passed, 0 failed",
    "core_cost_invariants_and_decision_citations": "9 passed, 0 failed",
    "each_new_test_red_under_a_hand_change": true,
    "hand_changes": 25,
    "names": [
      "a_zero_period_and_a_factor_outside_its_range_are_refused",
      "nothing_is_read_before_the_atr_has_its_period",
      "the_first_atr_candle_is_down_on_the_upper_band_and_unsettled",
      "period_one_reads_the_published_zero_band_first",
      "both_bands_ratchet_on_every_candle_and_reset_as_published",
      "a_turn_needs_a_close_beyond_the_current_band",
      "fixture_st_1_the_seed_differs",
      "fixture_st_2_a_flip_down_keeps_the_ratcheted_upper_band",
      "fixture_st_3_a_flip_up_keeps_the_ratcheted_lower_band_and_turns_down_first",
      "fixture_st_4_a_flat_series_reads_up_in_the_latch_and_down_here",
      "fixture_st_5_a_wide_candle_turns_the_two_band_form_first",
      "fixture_st_9_equal_bands_do_not_turn_an_up_candle",
      "a_gap_through_either_band_turns_on_the_gap_candle",
      "an_all_up_and_an_all_down_series",
      "the_stream_equals_a_from_scratch_transcription_of_the_published_rules",
      "the_atr_is_the_engine_atr",
      "no_reading_depends_on_a_later_candle",
      "a_time_gap_reads_as_contiguous_candles",
      "the_extremes_never_overflow",
      "a_corrupt_candle_is_refused_and_changes_nothing",
      "the_state_is_a_fixed_size",
      "the_bench_measures_both_rows_at_tiny_sizes",
      "a_cost_is_read_by_rank_and_judged_flat_or_not"
    ]
  },
  "gates": {
    "toolchain": "rustc 1.97.1 (8bab26f4f 2026-07-14)",
    "fmt_all_check": "clean",
    "clippy_indicators_all_targets_deny_warnings": "exit 0",
    "test_indicators_locked": "exit 0",
    "test_core_cost_invariants_decision_citations": "exit 0",
    "trend_rs_diff_from_start": "empty",
    "evaluator_rs_and_vocab_diff_from_start": "empty",
    "local_gates_10_11_12": "exit 1 at START and at the tip with identical refusals, all in other crates; this part adds none",
    "gate_14": "not run; the cover-table row for the two ids is the main session's at the family assembly"
  },
  "coverage": {
    "route": "B",
    "rustc": "rustc 1.99.0-nightly (1a98b1e13 2026-08-07)",
    "llvm": "LLVM version 23.1.0-rust-1.99.0-nightly",
    "measured_at": "0fecf16c610bd3df1bb95a5abde09c8e1367870a",
    "rust_identical_to_head": true,
    "reading": "narrow",
    "bench_target_run_once_under_the_same_instrumentation": true,
    "files": {
      "crates/indicators/src/dual_band_supertrend.rs whole": { "lines": "148/148", "branches": "28/28" },
      "crates/indicators/src/dual_band_supertrend_tests.rs whole": { "lines": "775/775", "branches": "54/54" },
      "crates/indicators/benches/ratio/psrb_stv.rs whole": { "lines": "45/48", "branches": "3/6", "f_items": 6 },
      "crates/indicators/benches/ratio.rs main 758-821": { "lines": "24/26", "branches": "1/2", "f_items": 3 },
      "crates/indicators/benches/ratio.rs line 73": { "lines": "0/0", "branches": "0/0" },
      "crates/indicators/benches/ratio.rs line 74": { "lines": "0/0", "branches": "0/0" },
      "crates/indicators/src/lib.rs line 66": { "lines": "0/0", "branches": "0/0" }
    },
    "gaps": 0,
    "f_items": 9,
    "u_items": 0,
    "not_proved": 0,
    "coverage_proved": true,
    "mutants": "deferred (operator 30 Oct)"
  },
  "benches": {
    "advisory": true,
    "machine": "Intel Xeon @ 2.10GHz, 4 cores, 15 GiB",
    "command": "cargo bench -p indicators --bench ratio --locked",
    "runs": 2,
    "C-PSRB-STV-01_best_round_p99_ns_at_2000_20000_200000": [[62, 64, 64], [81, 82, 84]],
    "C-PSRB-STV-02_best_round_p99_ns_at_period_10_100_1000": [[63, 65, 65], [82, 81, 82]],
    "ceiling_permille": 3000,
    "budget_ns": 2000,
    "verdict": "every row of both runs ok, exit 0"
  },
  "decisions": ["D-8600", "D-8601", "D-8602", "D-8603", "D-8604", "D-8605"],
  "needs_mac": [
    "check 1 on the Mac's pin: Route B on the D2 set against the seam, the bench wrapper and main by one instrumented bench run",
    "gate 8: cargo bench -p indicators --bench ratio with both C-PSRB-STV rows",
    "gates 10, 11, 12 and 14; gate 14's cover-table row for C-PSRB-STV-01 and C-PSRB-STV-02 at the family assembly",
    "macOS rows: none expected; the_state_is_a_fixed_size pins 224 bytes, measured on x86_64 only",
    "the anchors on the then-current feat/db-fix",
    "git diff --exit-code <seam> <tip> -- crates/indicators/src/trend.rs crates/indicators/src/evaluator.rs crates/vocab (empty here)"
  ],
  "held": false
}
```

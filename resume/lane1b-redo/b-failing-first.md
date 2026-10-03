```
thread 'screen_policy_tests::a_stricter_tier_that_alone_admits_is_found_and_reported_met' (21777) panicked at crates/cli/src/screen_policy_tests.rs:194:5:
assertion `left == right` failed
  left: [3]
 right: [0, 1]
stack backtrace:
--
thread 'screen_policy_tests::the_tier_walk_checks_each_tier_in_order_and_stops_at_the_first_admission' (21999) panicked at crates/cli/src/screen_policy_tests.rs:154:5:
assertion `left == right` failed
  left: [3]
 right: [0, 1, 2, 3]
stack backtrace:
--
thread 'screen_policy_tests::the_tier_walk_propagates_refusals' (22043) panicked at crates/cli/src/screen_policy_tests.rs:222:5:
assertion failed: matches!(refused, Err(why) if why == "refused: tier 1")
stack backtrace:
             at /rustc/8bab26f4f68e0e26f0bb7960be334d5b520ea452/library/std/src/panicking.rs:689:5
--
thread 'screen_policy_tests::a_cascade_that_admits_nothing_prints_no_tier_as_met' (21776) panicked at crates/cli/src/screen_policy_tests.rs:93:5:
the walk must end on the mildest tier's verdict:
YOUR RULES: UNMET — nothing cleared the policy you stated. The tier ladder below says what the strictest MET standard was.

TIER LADDER

[exited with code 0]
```
```
test sweeprun::tests::a_screen_without_a_support_or_a_ceiling_is_refused ... ok
test sweeprun::tests::a_negative_stop_ceiling_is_refused_rather_than_swept_with ... ok
test sweeprun::tests::a_stop_ceiling_of_zero_means_no_ceiling_as_cli_reads_it ... FAILED
thread 'sweeprun::tests::a_stop_ceiling_of_zero_means_no_ceiling_as_cli_reads_it' (18611) panicked at crates/api/src/sweeprun.rs:5667:14:
zero is no ceiling, not a refusal: Malformed("`max_points` is 0. A ceiling of zero admits no trade and a negative one is not a distance.")
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 1354 filtered out; finished in 0.41s
```

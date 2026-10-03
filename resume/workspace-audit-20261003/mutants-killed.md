# Mutation-kill ledger (worker mut1, merged by CI thread in 0cab319)

# mut1 survivor ledger

Each survivor was applied by hand (the run's own diff) and the WHOLE package's
tests run (cargo nextest run -p <crate>). Verdicts:
KILLED by <test> / EQUIVALENT because <reason> / CAUGHT-BY-FULL-PACKAGE (<test>).

## crates/indicators/src/pattern.rs (28)
All 28 were applied and the whole indicators package run. 20 failed no pre-existing test (really missed). For 8 (750:13 751:13 761:13 762:13 763:13 791:13 792:13 793:13), gap::tests::complete_sessions_through_the_evaluator_are_byte_identical also failed, so they were CAUGHT-BY-FULL-PACKAGE. The new test also kills all 28.

- crates/indicators/src/pattern.rs:652:13: replace && with || in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:651:13: replace && with || in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:650:13: replace && with || in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:649:13: replace && with || in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:652:26: replace < with <= in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:703:40: replace && with || in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:750:13: replace && with || in Patterns::bits -> CAUGHT-BY-FULL-PACKAGE (gap::tests::complete_sessions_through_the_evaluator_are_byte_identical); also KILLED by pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:751:13: replace && with || in Patterns::bits -> CAUGHT-BY-FULL-PACKAGE (gap::tests::complete_sessions_through_the_evaluator_are_byte_identical); also KILLED by pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:749:26: replace > with >= in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:750:26: replace < with <= in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:751:27: replace < with <= in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:763:13: replace && with || in Patterns::bits -> CAUGHT-BY-FULL-PACKAGE (gap::tests::complete_sessions_through_the_evaluator_are_byte_identical); also KILLED by pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:752:27: replace > with >= in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:762:13: replace && with || in Patterns::bits -> CAUGHT-BY-FULL-PACKAGE (gap::tests::complete_sessions_through_the_evaluator_are_byte_identical); also KILLED by pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:761:13: replace && with || in Patterns::bits -> CAUGHT-BY-FULL-PACKAGE (gap::tests::complete_sessions_through_the_evaluator_are_byte_identical); also KILLED by pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:760:26: replace < with <= in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:761:26: replace > with >= in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:762:27: replace > with >= in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:793:13: replace && with || in Patterns::bits -> CAUGHT-BY-FULL-PACKAGE (gap::tests::complete_sessions_through_the_evaluator_are_byte_identical); also KILLED by pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:763:27: replace < with <= in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:792:13: replace && with || in Patterns::bits -> CAUGHT-BY-FULL-PACKAGE (gap::tests::complete_sessions_through_the_evaluator_are_byte_identical); also KILLED by pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:791:13: replace && with || in Patterns::bits -> CAUGHT-BY-FULL-PACKAGE (gap::tests::complete_sessions_through_the_evaluator_are_byte_identical); also KILLED by pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:789:13: replace && with || in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:790:13: replace && with || in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:788:27: replace > with >= in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:789:25: replace < with <= in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:792:26: replace > with >= in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own
- crates/indicators/src/pattern.rs:793:27: replace < with <= in Patterns::bits -> KILLED by indicators pattern::exemplars::every_clause_of_the_reshaped_patterns_is_required_on_its_own

## crates/api/src/server.rs (4, run stopped early)
- crates/api/src/server.rs:3136:30: replace == with != in audit_one -> KILLED by api server::tests::only_an_nse_derivatives_series_owes_the_derivatives_venue_hours (no other api test failed)
- crates/api/src/server.rs:3136:39: replace && with || in audit_one -> KILLED by api server::tests::only_an_nse_derivatives_series_owes_the_derivatives_venue_hours (no other api test failed)
- crates/api/src/server.rs:3136:68: replace || with && in audit_one -> KILLED by api server::tests::only_an_nse_derivatives_series_owes_the_derivatives_venue_hours (no other api test failed)
- crates/api/src/server.rs:3136:85: replace == with != in audit_one -> KILLED by api server::tests::only_an_nse_derivatives_series_owes_the_derivatives_venue_hours (no other api test failed)

EQUIVALENT: none.
Commits on audit-fix/mut1: 65e10f0, c4ce28f, fb4bd7c.

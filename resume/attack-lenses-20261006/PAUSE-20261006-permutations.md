# PAUSE (3rd, 08:28 UTC) — lens L3 permutations (attack/permutations) — 2026-10-06

Head: `attack/permutations` @ `364ce11` = **WIP (paused, not validated)**. Last validated push: `09a9793`.

## Done
- R1: F-25A074 FIXED (pattern midpoints, D-3402, XPERM-02). R2: F-3B4D5A FIXED (SuperTrend stop exact, D-3403, XPERM-03). D-3400 EMA withdrawn (dup of ind1-2).
- R3 (4 agents, all reported): core/costs/greeks clean; engine/runner clean (tracked: c4a-7/D-1496, D-1439, D-1497); fold/calendar special sessions: day lists agree; ONE new doc defect F-109F13 (charter + evaluator said the pull keeps unmeasured-Muhurat minutes; D-2670 refuses them). Reflection lens: gap ladder floor (IF-23/D-1861) and SuperTrend seed tie REFUTED as owner calls (in D-3404); bit 68 floored gap mid belongs to the ind1-2 owner.

## In WIP commit (needs validation)
- D-3404 doc corrections (docs/00-charter.md, indicators/src/evaluator.rs, gap.rs comment), XPERM-04 test `indicators::evaluator::muhurat_claims::no_document_says_the_pull_keeps_an_unmeasured_muhurat_minute` (moved out of pull: Gate 1d refused path literals there). Proven to fail on the base charter.
- Gate 18 at 09a9793: 2 MISSED — pattern.rs:692 `>`→`>=` and :700 `<`→`<=` (163/164). Kill asserts added (morning_even/star_even, whole 151 midpoint). Not yet re-run.

## Next steps after RESUME
1. `cargo test -p indicators --lib -- midpoint_exact muhurat_claims` green; full indicators + pull non-root; clippy; gates (all but 1e).
2. Re-run the two mutants: `cd /tmp/claude-0/wt-mut && git checkout -q <new head> && cargo mutants --baseline skip --in-place --timeout 900 --cap-lints true -p indicators --file crates/indicators/src/pattern.rs --re 'pattern.rs:(692|700)'` (line numbers may shift; use --list).
3. Replace "FIXED (sha pending)" for F-109F13 with the commit sha; push; update permutations.tsv.
4. Round 4 (fresh angles); repeat until a round finds zero; RESULT-permutations.md.

## Restart
`git fetch origin attack/permutations final/all-fixes && git checkout attack/permutations`; tests as non-root via /tmp/claude-0/runtests.sh <crate>.

## Gate 18 final (09a9793, 81 mutants)
caught 76, missed 2, timeout 0, unviable 3. Missed: crates/indicators/src/pattern.rs:692:31: replace > with >= in Patterns::bits;crates/indicators/src/pattern.rs:700:31: replace < with <= in Patterns::bits;

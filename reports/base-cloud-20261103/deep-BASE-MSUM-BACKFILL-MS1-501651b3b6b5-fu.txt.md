# deep-BASE-MSUM-BACKFILL-MS1-501651b3b6b5-fu.txt.md

The (F)/(U) list of BASE-MSUM-BACKFILL-MS1's Route B coverage at code tip `501651b3b6b56ff30b7635ee9bce7e4d6d8b75f1` (recipe v4 step 4). One item, and it is (U). No (F) item: every failing side of an assertion in the set's tests was written so that no branch or region of it is left only for a failing run (compound conditions split into one assertion each, exact `assert_eq!` values instead of `match`/`matches!` arms that panic).

`````text
(U) crates/cli/src/msum_builder.rs:184-187, 4 lines, in checked():
        other => {
            return Line::Refused {
                why: format!("the store reported an outcome this build does not know: {other:?}"),
            };
    Unreachable on every target, aarch64-apple-darwin included. Proof from the code: the value matched is a
    store::file::Summarized (crates/store/src/month_summary_writer.rs:86-96), declared #[non_exhaustive] (line 86)
    with exactly four variants: Kept, Built(String), Empty, BelowThreshold. checked() matches each of the four
    explicitly before this arm (msum_builder.rs:173-176). A #[non_exhaustive] enum of another crate makes rustc
    demand a wildcard arm (E0004 without it), and no value of a fifth variant can exist in a build whose store crate
    declares four: no input, file, folder, variable or call reaches the arm. It is the loud refusal for the day the
    store adds a fifth outcome (CLAUDE.md section 4: degrade loudly, never a silent fallback), and it is reached in
    that build, not this one.
`````

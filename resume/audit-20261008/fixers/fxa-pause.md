# fxa pause — worktree /home/claude/wt-fxa, branch audit/fx-a, HEAD 8ec02b03 (base 1c9ec4ca)

- sobs-2 (loss ledger) — DONE a9066b92 (D-4410, FXA-01)
- sobs-13 (clean-shutdown sync) — DONE a9066b92 (D-4411, FXA-02); api/cli mains must call telemetry::sync() at exit
- sobs-3 (torn tail + full disk) — DONE b0e992af (D-4412, FXA-03)
- r53-1 (stderr never panics) — DONE 94983010 (D-4413, FXA-04); api/cli eprintln sites should use telemetry::stderr_line
- sobs-11 (capture event + dir fsync) — DONE 37ed211d (D-4414, FXA-05)
- satk-9 (lake dictionary panic) — DONE d8547a8e (D-4415, FXA-06)
- satk-2 (.tix bound to held bars) — DONE 8bcf95cf (D-4416, FXA-07)
- satk-5 / satk-6 (NaN round trip, repeated keys) — DONE 5cc12525 (D-4417, D-4418, FXA-08, FXA-09)
- so1-2 (lake Batch::row random p99) — WIP 8ec02b03. The code, measurements, bite proof and docs are done. Left: clippy -p lake --all-targets, fmt check, gates 10/11/27, then amend the message out of WIP. Next step: `cargo clippy -p lake --all-targets --locked -- -D warnings`.
- so1-6 (store bench temp dirs) — WIP 8ec02b03. The Scratch guard is written. Left: build and run `cargo bench -p store`, check that $TMPDIR has no brutex-bench-* afterwards, write D-4420 and its limits note, run clippy -p store.
- rnew-2 (stale D-2290 bisection row) — NOT-STARTED. Plan: rewrite docs/06-limits.md:16359 to say the .tix path is one entry read and the bisection is legacy only; add a guard in crates/store/tests/bisect_cost.rs that the phrase "an index file would be a new store format version" is gone.
- rnew-3 (bench the D-2290 kept costs) — NOT-STARTED. Plan: store gets the legacy bisection p99 (remove the .tix, then open_existing) and the tail-block-with-fstat cold read; pull gets per-entry Manifest::load at 15,857 and 93,776 entries and per-record committed_cash_days; print p50/p99/max and gate under 3.0x.
- W1-pull1-0 (receipt revalidation) — NOT-STARTED. Measure prepare_observed_with (crates/pull/src/cash_session_cache.rs:302) in the pull bench.
- sobs-16, sobs-19, sobs-17, r53-2/srust-6 docs — NOT-STARTED.
- ET-bars-candles-store-3 — NOT-STARTED. Assessed as NEEDS-OWNER. The session verdict depends on venue (cash vs derivatives clock, 15:40 since 2026-08-03), on cadence (daily is exempt) and on unclassified days, and the store holds BSE, MCX and contracts. Moving calendar.rs and session.rs (3,485 lines) into core would also move §5's calendar authority.
- o1api-33 — NOT-STARTED. Assessed: already measured (D-2291). A typed or streaming decode means re-implementing the spec-driven refusals in decode_value against a token stream. Likely NOT-FIXABLE-in-scope, with the reason recorded.
- Final checks not yet run on the full diff: clippy/tests per crate, the gates, and cargo mutants per touched crate (telemetry, pull, store, lake). Next decision is D-4420; next invariant is FXA-11.

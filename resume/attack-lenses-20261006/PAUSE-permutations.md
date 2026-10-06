# PAUSE — lens L3 permutations (attack/permutations) — 2026-10-06

Head: `attack/permutations` @ `cee4ccb` (pushed). Last validated push: `ec68278`.

## Done
- Round 1 (7 read-only agents: core/identity, vocab, engine, indicators A+B, costs+greeks, pull fold/calendar/session, runner numerics).
  - FIXED F-25A074 (D-3402, XPERM-02): candlestick midpoints floored → 162/164/212 lost one price, 226 off-centre. Tests failed before fix (`prior 100->103, bar 104->101`, `101 103 100 101`), pass after. Commit f9ab789 + ec68278.
  - REFUTED: VWAP below-bias (docs/26-vwap-mapping.md:19-20 defines floor), SuperTrend seed carried past warm (D-1542 locked; wording fixed, D-3401).
  - Already tracked: fold width not dividing a day (D-3130/D-3131, DPR-08/10).
- Validation at ec68278: fmt, clippy -D warnings, tests (indicators, runner, cli, api as non-root), static gates 0-14 (1e skipped) all green.
- Round 2 (4 agents): runner trade boundaries clean (p9num-2 tracked); evaluator truth/known/look-ahead clean; pull decoders: R2-1/R2-2 already tracked (STO-2); floored-level class: F1 gap mid 66/67 = ind1-2 (tracked, fixed on wip/zero/numeric-edges de48df5), F3 daily pivot REFUTED (daily.rs:30-40 defines floor, pinned test), F2 SuperTrend SURVIVES refutation.

## Dedupe miss, being corrected
- My D-3400 (EMA sides, F-DBC24E, XPERM-01) duplicates ind1-2, owned by numeric helper, fixed on `wip/zero/numeric-edges` de48df5. cee4ccb withdraws the EMA code; the rest still refers to it.

## In progress / exact next steps
1. Re-take the three pins for the pattern-only change (D-3402): `indicators::gap::tests::complete_sessions_through_the_evaluator_are_byte_identical`, `runner::signal_candle_stop::tests::a_tabled_day_window_seals_exactly_what_the_full_row_walk_did`, `cli::index_stop::tests` PINS. Verify each passes on base with base indicators/src first.
2. Docs: rewrite D-3400 as withdrawn/duplicate of ind1-2; F-DBC24E disposition → "DUPLICATE of ind1-2 (wip/zero/numeric-edges de48df5), withdrawn"; delete the XPERM-01 row (its tests are gone).
3. F2 (new, survives refutation): `trend.rs` SuperTrend::fold uses `Atr::value()` (paisa floor), `midpoint` floor, band floor; bits 64/65 can land on the WRONG side (CLASSICAL: bars 0-9 H101 L100 C101; bar 10 H97 L96 C96; close 100 → code 64, exact stop 100.7 → 65). Fix: compare in scaled space while keeping `stop() -> Option<i64>` and C4-INDICATORS-01. Failing test first. D-3403, XPERM-03.
4. Gate 18 pre-run in a separate worktree (`/tmp/claude-0/wt-mut`): `cargo mutants --in-diff <diff> --baseline skip --in-place --timeout 900 --cap-lints true -p indicators` (no `--jobs` with `--in-place` in 26.2.0). Partial run: 31/71 caught, 0 missed before the pause.
5. Validate (fmt, clippy, indicators/runner/cli tests non-root, gates), merge origin/final/all-fixes, push; then round 3; tracker permutations.tsv + RESULT-permutations.md.

## Restart
`git fetch origin attack/permutations final/all-fixes && git checkout attack/permutations`; non-root test runner script: build with `cargo test --no-run`, copy binaries, run via `setpriv --reuid=65534 --regid=65534 --clear-groups`.

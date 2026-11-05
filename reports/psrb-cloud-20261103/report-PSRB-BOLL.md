STATUS: IN PROGRESS

# PSRB-BOLL: cloud build report (in progress)

- Branch `cloud/psrb-boll-20261103`, TOKEN `PSRB-BOLL-20261103-1`, cut from `origin/cloud/psrb-kit-20261103`
  (`56f3043d`). START = `35234cead4009299a93fcd79a3d1fffd773dcc16` (PSRB-SEAM on `1ad6e24ef8e0`).
- Kit check: all 12 MANIFEST.md rows match by sha256; MANIFEST.md itself hashes to the kit commit's
  `85b873eb…`; `git diff --name-only START origin/cloud/psrb-kit-20261103` lists only the 13 kit files.
- Baseline on START (stable 1.97.1, this container): `cargo test -p indicators --locked` 504 passed, 0 failed;
  `cargo test -p core --locked --test cost_invariants --test decision_citations` 5 + 4 passed. No host red.
- Code commit `8c77e2f9`: `crates/indicators/src/bollinger.rs`, `src/bollinger_tests.rs` (23 tests),
  `benches/ratio/psrb_boll.rs`, the `lib.rs` line, the two `benches/ratio.rs` insertions. fmt and clippy
  `-D warnings` clean; `cargo test -p indicators --locked` 527 passed, 0 failed.

NEXT: the four ledger blocks (docs/04, 05, 06, 07; D-8590 to D-8595), the core tests, each new test red under a
hand change, Route B on `nightly-2026-08-08`, the bench run, then part-PSRB-BOLL.md and result-PSRB-BOLL.json.md.

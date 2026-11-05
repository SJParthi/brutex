STATUS: IN PROGRESS
ASSEMBLED: dac5db855377bd0880854aa7826b4de35e179be3

# report-BASE-COV-PULL-PSA (TOKEN BASE-COV-PULL-PSA-20261103-1)

Interim report, written after step (3)'s first Route B. Session BASE-COV-PULL-PSA of the 3 Nov 2026 cloud wave: the
assembly of BASE-COV-PULL's three shards. Branch `cloud/cov-pull-psa-20261103`.

## Done so far

- **Step (1)**: PS1, PS2, PS3 stacked with `replay-minus-reports.sh` (P1-P4 held each time); ASSEMBLED
  `dac5db855377` (line 2). Details in the final report.
- **Gates on the assembled tree** (stable 1.97.1, root, 4 cores): `cargo test -p pull --locked --no-fail-fast` 34
  binaries, 3607 passed, 0 failed, 54 ignored (no host red left); fmt and clippy `-D warnings` clean.
- **Step (2)**, code tip `a52270b1f00a`: PS3's `tenor.rs` 203-204 closed by
  `vendor::tests::a_tenor_refused_for_unverified_hours_quotes_the_refusal` (red under a hand change); PS1's gate 1d
  words: the 648 the package added declared in `.github/gates_tree.rs` (`BASE_COV_PULL_LITERAL`), gate 1d now names
  exactly feat/db-fix's own 481.
- **Step (3)**, Route B whole:pull on `a52270b1f00a` (kit script unchanged, pinned nightly): 255 files, lines
  175,277/179,855 (4,578 uncovered), branches 12,614/14,454 (1,840 sides); every uncovered row carries its shard's
  class: F 2,226, U 364, IGNORED (L-BCP-3) 3,798, UNMEASURED (L-BCP-5) 11, GAP 19. Lists: `gaps-PSA.txt.md`,
  `base-cov-pull-fu.txt.md`, `deep-BASE-COV-PULL-PSA-a52270b1f00a-set.txt.md`.
- **Step (4)**: D-7335, D-7336, D-7337 written; BCP-08..12 flipped (their tests green), BCP-13..21 added, BCP-02 names
  its tests.

## Next

A planted kill inside a month's creation for the two kill-test GAP rows (test code only), then Route B again, D-7338,
docs/06, the final gates, the fix json and line 1.

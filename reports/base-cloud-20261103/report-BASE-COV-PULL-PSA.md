STATUS: IN PROGRESS
ASSEMBLED: dac5db855377bd0880854aa7826b4de35e179be3

# report-BASE-COV-PULL-PSA (TOKEN BASE-COV-PULL-PSA-20261103-1)

Interim report, written after step (1). Session BASE-COV-PULL-PSA of the 3 Nov 2026 cloud wave: the assembly of
BASE-COV-PULL's three shards. Branch `cloud/cov-pull-psa-20261103`.

## Step (1): the stack

The kit checks out: all 26 `MANIFEST.md` rows match (sha256 of the whole file, or of the fenced block extracted
with the manifest's own `awk`).

Shard tips on origin, fetched at the start, equal the tips the brief wrote, so no tip moved:
PS1 `db608fb4fbd4bc613a4caaffb7678c562df75393`, PS2 `3b18385cda5b14ab68f885c1d78ee8dfc29a7de0`,
PS3 `f8435521675929e0662bd307271d82ccae7c20cb`. All three reports read `STATUS: BLOCKED OUTSIDE`.

The list: `git diff --name-only 2e7a0b6d3b56 <TIP> -- . ':(exclude)reports'` prints 60 files for PS2 and 48 for PS3,
so neither is left out. Stacked with the kit's `replay-minus-reports.sh` (sha256 `f223185d...`), under `nice -n 15`:

| call | START | FROM | TIP | ONTO | NEWREF | result |
|---|---|---|---|---|---|---|
| PS1 | `f98a899398f6` | `5789f4353f6a` (kit) | `db608fb4fbd4` | (START) | `refs/heads/stack/psa-1` | `OK ... = 4e6cbdbc1e7b` (26 commits replayed, 10 skipped as reports only; P1-P4 hold; 68 files in X) |
| PS2 | `2e7a0b6d3b56` | `2e7a0b6d3b56` | `3b18385cda5b` | `stack/psa-1` | `refs/heads/stack/psa-2` | `OK ... = 341fca3998ee` (51 replayed, 6 skipped; P1-P4 hold; 60 files in X) |
| PS3 | `2e7a0b6d3b56` | `2e7a0b6d3b56` | `f84355216759` | `stack/psa-2` | `refs/heads/cloud/cov-pull-psa-20261103` | `OK ... = dac5db855377` (17 replayed, 5 skipped; P1-P4 hold; 48 files in X) |

Checked again on the assembled tree: every file a shard changed equals the version of the last shard that changed it
(the two files PS1 and PS3 share, `gdfl_dropbox_cov_tests.rs` and `gdfl_run_start_cov_tests.rs`, were changed by PS1
before the GREEN-TIP only, so they equal PS3's); 174 files differ from START outside `reports/` (68 + 60 + 48 - 2);
94 commits over START, no merge. The `stack/psa-*` refs stay local and are never pushed.

## Next

Step (2) settle every BLOCKED OUTSIDE need; step (3) Route B whole:pull on the pinned nightly; step (4) the docs
entries D-7335..D-7339; step (5) the final gates and the fix json.

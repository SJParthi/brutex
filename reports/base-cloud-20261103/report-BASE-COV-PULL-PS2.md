STATUS: IN PROGRESS

# report-BASE-COV-PULL-PS2 (TOKEN BASE-COV-PULL-PS2-20261103-1)

Interim report, 3 Nov 2026 18:03 IST. Session BASE-COV-PULL-PS2 of the 3 Nov 2026 cloud wave, group G2 of
`SHARDS.md`. Branch `cloud/cov-pull-ps2-20261103`, cut from `2e7a0b6d3b568190024e2e9b405e803805f3fdca` (PS1's
GREEN-TIP). Owner file `reports/base-cloud-20261103/OWNER-BASE-COV-PULL-PS2.md` (commit `4f0ffe7c`).
This file is replaced by the final report; until then it says what is on the branch.

## Kit check

All 26 manifest rows match `MANIFEST.md` (sha256 of the whole file, or of the fenced block extracted with the
manifest's own awk); `MANIFEST.md` hashes to `078a849c...c6479`.

## Host and pins

Linux x86_64 cloud container, uid 0, 4 cores, 15 GiB memory, ~25-29 GiB free on `/`. Stable `rustc 1.97.1`
(rust-toolchain.toml) for fmt, clippy and the suite. Route B: `rustup toolchain install nightly-2026-08-08`
gave exactly the pins: `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, `llvm-cov` and `llvm-profdata`
`LLVM version 23.1.0-rust-1.99.0-nightly`. The kit's `d3-route-b-cloud.sh` is run unchanged (D3_W, D3_DIR,
D3_NIGHTLY, D3_LLVM_BIN, D3_CARGO set; `CARGO_PROFILE_DEV_DEBUG=0`; TMPDIR `/dev/shm/ps2-tmp` mode 1777).
A watcher beside it sets the profile subfolder `<P>/pull` to mode 1777 the moment the script creates it, so the
D-0995 permission children (uid 65534) write their profiles: 17 of the 73 profiles at START were theirs.

## Baseline: Route B on START

`2e7a0b6d`, set `deep-BASE-COV-PULL-f98a899398f6-set.txt` (its 206 coverage files are START's src/ and tests/
files), 16:05-16:42 IST: build 2 min; tests 5 min (2939 passed, 10 failed, 54 ignored: PS1's 9 root-only reds
plus `nse::tests::deduplicating_index_links_costs_one_probe_each_rather_than_a_scan`, a timing test of G1's
file, under instrumentation); reading 30 min. `ROUTE-B DONE ... every package measured and read (pull)`, exit 0.

- whole `pull`: lines 153,379/160,118 (6,739 uncovered), branches 11,562/14,244 (2,682 sides).
- G2's 77 files: lines 52,247/54,617 (2,370 uncovered), branches 3,800/4,622 (822 sides).

## Work so far (all pushed)

Three workers on this host: this session (census_store, alerts, emit_sites, the gdfl_import sidecars, the
rest of G2) and two helpers in their own worktrees (fork B: gdfl_rekey, gdfl_run, gdfl_run_ledger, gdfl_cm,
gdfl_v3, gdfl_dayzips, gdfl_format and their test files; fork C: response_keep, zerodha_spool, landing_run,
repair_plan, calendar, fno, chain and the rest), whose commits are cherry-picked here.

Dev-loop figures (instrumented opt-level 0, the new tests alone, START's rows moved through each file's diff;
the final Route B replaces them): census_store.rs 231 -> 35; alerts.rs 30 -> 2; gdfl_import_filing.rs 65 -> 35;
gdfl_import_quality.rs 13 -> 2; gdfl_import_census.rs 2 -> 0; emit_sites.rs 19 -> 11, all (F); response_keep.rs
94 -> 25; zerodha_spool.rs 92 -> 12; landing_run.rs 43 -> 2; repair_plan.rs 39 -> 18; calendar.rs 34 -> 16 (all
(U)); gdfl_rekey.rs 168 -> 40; gdfl_run_ledger.rs 44 -> 15; gdfl_v3.rs 3 -> 0; gdfl_cm.rs 19 -> 11 (all (U)).

## Still to do

The helpers' remaining files; my red checks; the gates; the final Route B; `gaps-PS2.txt.md`, `fu-PS2.txt.md`,
`fix-BASE-COV-PULL-PS2.json.md`; this report's final form with D-7336's draft.

## NEEDS THE MAC

Route B whole:pull on its pin, the shadow GET re-check, macOS rows, non-root 14-core tests, the 54 ignored
real-data tests (L-BCP-3), the D1 landing.

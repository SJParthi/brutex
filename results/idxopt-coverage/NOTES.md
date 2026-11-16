# NOTES: idxopt box 0, coverage (JOB cov)

Labels as in JOB-IDXOPT.md: MEASURED, ESTIMATE, EXACT. No figure here is a trading result.

## 1. Source

- Branch `cloud/idxopt-v3-20261116`, commit `10dacdd629ee7052c4ca3fe2312b3af275cda172`: equal to READY.txt and to the
  clone's HEAD (step 1).
- `JOB-IDXOPT.md` sha256 `183ad0d58c71f74b3ca629b532a027cf44e10a25527c97c9e985585d6e1e3552`, as READY.txt names it.
- `~/box.sh` extracted verbatim (step 2): sha256 `2e54ba7c260a43c2e217dc11e71c1aad95eebc83a6c59dbeb8370112e5783124`, as
  required.
- Manifest check (`box.sh setup`): 366 files, sha256 `e082593fbc5101f8691bc68b08846e44bc5a0388be924c91d79f75416d4cff30`,
  equal to the expected value.

## 2. Box facts (FACTS.txt)

- Linux 6.18.44-fc-v470 x86_64; nproc 4; MemTotal 16473776 KiB; cgroup memory.max n/a; disk free 30385500 KiB.
- rustc 1.97.1 (8bab26f4f 2026-07-14); cargo 1.97.1 (c980f4866 2026-06-30); cargo-llvm-cov 0.9.1.

## 3. Baselines (BASELINE.txt; passed failed ignored filtered, EXACT)

```
BASELINE set=A1 rc=0 passed_failed_ignored_filtered="73 0 0 1" expected="73 0 0 1" 2026-11-16 16:04:06 UTC
BASELINE set=A2 rc=0 passed_failed_ignored_filtered="440 0 2 0" expected="440 0 2 0" 2026-11-16 16:04:12 UTC
BASELINE set=B2 rc=0 passed_failed_ignored_filtered="24 0 0 2" expected="24 0 0 2" 2026-11-16 16:04:16 UTC
BASELINE set=B1 rc=0 passed_failed_ignored_filtered="73 0 0 4" expected="73 0 0 4" 2026-11-16 16:04:34 UTC
```

## 4. SUMMARY lines

```
SUMMARY idxopt-coverage set=A-idxopt lines=41.17% (14927/36254) functions=36.95% regions=44.30% branches=not-measured wall_s=20 rc=0 runs=A1:0,A2:0 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_llvm_cov=0.9.1
SUMMARY idxopt-coverage set=B-host lines=37.20% (4152/11162) functions=31.88% regions=31.87% branches=not-measured wall_s=35 rc=0 runs=B1:0,B2:0 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_llvm_cov=0.9.1
```

## 5. Wall (MEASURED, from box.log timestamps at 1 s resolution)

| step | wall |
|---|---:|
| setup: manifest, toolchain 1.97.1 by rustup (27 s), cargo-llvm-cov 0.9.1 install (68 s) | 96 s |
| baselines: A1 10 s, A2 6 s, B2 4 s, B1 18 s | 38 s |
| coverage A-idxopt | 20 s |
| coverage B-host | 35 s |
| **in all**, 16:02:20 to 16:05:29 UTC | **189 s (3 min 9 s)** |

Section 9's ESTIMATE for box 0 was 20 to 40 min.

## 6. Unusual

- **The 7 binary drivers of workspace A are not in the report.** Section 4 says box 0's coverage reports the 9 binary
  drivers. Only `sniper_host`'s two (`src/bin/sniper-host.rs`, `src/bin/sniper-q.rs`) appear, both at 0.00%, because
  B1's integration-test targets build them. `opt_pos` `src/bin_*.rs` (5), `idx_cal` `src/main.rs` and
  `src/bin_served.rs`, and `idx_f1` `src/main.rs` are absent from A-idxopt's report and totals: the A1 coverage run is
  `--lib` only, so they were never built.
- **A2 ran 381 tests under coverage, not 440** (381 passed, 0 failed, 2 ignored). The difference is exactly the 59
  `costs` doc tests (`Doc-tests costs`, 59 passed, in baseline-A2.log): cargo-llvm-cov runs doc tests only with
  `--doctests`, which needs nightly. Every other coverage run equals its baseline: A1 73 passed / 1 filtered, B1 73 / 4,
  B2 24 / 2.
- The workspace totals include crates that no test here runs, by design (section 4): `core`, `store` and `telemetry` in A.
  48 of A's 96 files are at 0.00% lines (store 27, telemetry 12, core 8, idx_cal 1); 8 of B's 40 (sniper_host 7,
  wf_sweep 1).
- `wf_sweep/src/tests_wf.rs` is at 0.00% because its only two tests are the two B2 skips.
- `B-host/option_clock-row.txt` is empty: `covsum` greps for `option_clock.rs` in both sets, and that file is in A only.
- `cargo llvm-cov` builds into each workspace's default `target/` inside the checkout (`sniper-host-v3-idx-w4r3/target/`
  and `.../opening5s-5b3r70/target/`). Both are untracked, outside the manifest (`*/target/*`) and never staged
  (`progress` adds `results/` only). No tracked file changed.
- No `warning` or `error` line in any baseline or coverage log. No restart.
- The harness refused `sleep 240; ...`; the poll used `timeout 240 tail -n 0 -f ~/logs/box.log`, then `status`, as
  READY.txt allows. BOX DONE came during the first poll, so the first progress push is also the final one.
- READY appeared at 16:00:46 UTC (polled from 14:52 UTC); the box started at 16:02:20 UTC.

## 8. Coverage per package and least-covered files

Per package (PACKAGES.txt):

| workspace | package | files | regions | functions | lines (hit/total) |
|---|---|---:|---:|---:|---|
| A | costs | 25 | 99.16% | 99.21% | 99.46% (12040/12105) |
| A | idx_f1 | 2 | 94.74% | 94.74% | 94.27% (181/192) |
| A | opt_pos | 11 | 81.14% | 68.56% | 81.36% (2108/2591) |
| A | idx_cal | 8 | 45.81% | 44.94% | 46.43% (540/1163) |
| A | core | 11 | 4.59% | 6.01% | 4.26% (58/1362) |
| A | store | 27 | 0.00% | 0.00% | 0.00% (0/15815) |
| A | telemetry | 12 | 0.00% | 0.00% | 0.00% (0/3026) |
| B | sniper_kernel | 7 | 86.37% | 82.81% | 87.80% (439/500) |
| B | wf_sweep | 7 | 74.95% | 75.52% | 80.56% (1098/1363) |
| B | sniper_host | 26 | 23.86% | 24.85% | 28.12% (2615/9299) |

`costs/src/option_clock.rs` (A2's only mutated file): lines 97.67% (168/172), regions 97.35% (441/453), functions
87.50% (21/24).

10 least-covered files, ranked by line % with ties broken by most missed lines.

A-idxopt, all files (all ten at 0.00%, all in `store` and `telemetry`):

| file | lines (hit/total) |
|---|---|
| store/src/file.rs | 0/2395 |
| store/src/verdict.rs | 0/2076 |
| telemetry/src/sink.rs | 0/2013 |
| store/src/quarantine.rs | 0/1538 |
| store/src/sealed.rs | 0/1420 |
| store/src/provenance.rs | 0/791 |
| store/src/sort_index.rs | 0/776 |
| store/src/repair.rs | 0/759 |
| store/src/month_summary.rs | 0/714 |
| store/src/settings.rs | 0/708 |

A-idxopt, the tool's own packages only (`costs`, `idx_cal`, `idx_f1`, `opt_pos`):

| file | lines | hit/total |
|---|---:|---|
| idx_cal/src/ticks.rs | 0.00% | 0/327 |
| idx_cal/src/storeio.rs | 8.40% | 10/119 |
| idx_cal/src/crc.rs | 45.83% | 11/24 |
| opt_pos/src/secs.rs | 52.22% | 47/90 |
| opt_pos/src/chain.rs | 55.41% | 82/148 |
| idx_cal/src/bhav.rs | 57.69% | 105/182 |
| idx_cal/src/listing.rs | 60.49% | 49/81 |
| opt_pos/src/pos.rs | 74.48% | 677/909 |
| idx_cal/src/cal.rs | 82.51% | 283/343 |
| opt_pos/src/pack.rs | 83.04% | 186/224 |

B-host, all files:

| file | lines | hit/total |
|---|---:|---|
| sniper_host/src/secrun.rs | 0.00% | 0/1301 |
| sniper_host/src/sec.rs | 0.00% | 0/877 |
| sniper_host/src/esn1.rs | 0.00% | 0/386 |
| wf_sweep/src/tests_wf.rs | 0.00% | 0/171 |
| sniper_host/src/bin/sniper-host.rs | 0.00% | 0/141 |
| sniper_host/src/wfio.rs | 0.00% | 0/86 |
| sniper_host/src/bin/sniper-q.rs | 0.00% | 0/62 |
| sniper_host/src/words.rs | 0.00% | 0/52 |
| sniper_host/src/search.rs | 13.53% | 154/1138 |
| sniper_host/src/cands.rs | 16.25% | 117/720 |

## 9. Data and secrets

No market data was read, fetched or written; no link or secret was used.

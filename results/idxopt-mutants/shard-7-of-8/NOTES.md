# NOTES: idxopt mutants, box 8, JOB k = 7, shard 7 of 8

1. **Source.** Branch `cloud/idxopt-v3-20261116`, commit `10dacdd629ee7052c4ca3fe2312b3af275cda172`
   (the clone's HEAD, equal to READY.txt's commit). `JOB-IDXOPT.md` sha256
   `183ad0d58c71f74b3ca629b532a027cf44e10a25527c97c9e985585d6e1e3552` and `box.sh` sha256
   `2e54ba7c260a43c2e217dc11e71c1aad95eebc83a6c59dbeb8370112e5783124`, both as required. Manifest check OK:
   366 files, sha256 `e082593fbc5101f8691bc68b08846e44bc5a0388be924c91d79f75416d4cff30` (expected the same).

2. **Box facts** (`FACTS.txt`). Linux 6.18.44-fc-v470 x86_64, nproc 4, mem_total_kib 16473776, cgroup memory.max n/a,
   disk_free_kib 30322684. rustc 1.97.1 (8bab26f4f 2026-07-14), cargo 1.97.1 (c980f4866 2026-06-30),
   cargo-mutants 27.1.0 (installed by setup with `--locked --version 27.1.0`).

3. **Baselines** (`BASELINE.txt`; all four equal to section 4):
   - `BASELINE set=A1 rc=0 passed_failed_ignored_filtered="73 0 0 1" expected="73 0 0 1"`
   - `BASELINE set=A2 rc=0 passed_failed_ignored_filtered="440 0 2 0" expected="440 0 2 0"`
   - `BASELINE set=B2 rc=0 passed_failed_ignored_filtered="24 0 0 2" expected="24 0 0 2"`
   - `BASELINE set=B1 rc=0 passed_failed_ignored_filtered="73 0 0 4" expected="73 0 0 4"`

4. **SUMMARY lines** (`SUMMARY.txt`):
   ```
   SUMMARY idxopt-mutants shard=7/8 set=A1 mutants=283 caught=168 missed=114 timeout=0 unviable=1 wall_s=137 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
   SUMMARY idxopt-mutants shard=7/8 set=A2 mutants=54 caught=54 missed=0 timeout=0 unviable=0 wall_s=70 rc=0 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
   SUMMARY idxopt-mutants shard=7/8 set=B2 mutants=123 caught=61 missed=54 timeout=4 unviable=4 wall_s=183 rc=3 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
   SUMMARY idxopt-mutants shard=7/8 set=B1 mutants=481 caught=253 missed=214 timeout=2 unviable=12 wall_s=1126 rc=3 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
   SUMMARY idxopt-mutants shard=7/8 set=ALL mutants=941 caught=536 missed=382 timeout=6 unviable=17 wall_s=1516 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0
   ```
   - The mutant counts equal section 4's table for k = 7 exactly (283 / 54 / 123 / 481 = 941), although that table is
     from 26.2.0 (EXACT).
   - With the default slice sharding, shard 7 holds the last slice of each list. A1 is all `opt_pos`, A2 all `costs`
     (`option_clock.rs`), B2 all `wf_sweep`, B1 all `sniper_host` (EXACT, from each run's `mutants.json`).

5. **Wall (MEASURED, box.log timestamps and `wall_s`).**
   - Setup took 74 s, from 16:02:28 to 16:03:42 UTC; 72 s of it was the cargo-mutants 27.1.0 install.
   - Baselines took 41 s, from 16:03:42 to 16:04:23.
   - The mutant sets took 1516 s (25 min 16 s): A1 137 s, A2 70 s, B2 183 s and B1 1126 s.
   - The whole box took 1632 s (27 min 12 s), from 16:02:28 to BOX DONE at 16:29:40 UTC.
   - This is far below section 9's ESTIMATE of about 2 to 4.5 h. Median seconds per mutant, from `outcomes.json`
     (MEASURED):

     | set | build | test |
     |---|---:|---:|
     | A1 | 0.66 | 0.15 |
     | A2 | 1.41 | 0.51 |
     | B2 | 0.46 | 2.06 |
     | B1 | 1.84 | 1.36 |

   - Each cargo-mutants run's own unmutated baseline: A1 9 s build + 0 s test, A2 6 s + 1 s, B2 3 s + 1 s,
     B1 13 s + 4 s. The auto-set test timeouts were 20, 20, 20 and 24 s.

6. **Unusual.**
   - **Timeouts (6).**
     - B2 (4), all in `crates/wf_sweep/src/lists.rs` `Lists::offer_learn`: 100:20 `+=`→`*=`, 102:26 `>`→`==`,
       102:26 `>`→`>=`, 106:41 `-`→`+`. Each was cut at the auto 20 s.
     - B1 (2), in `crates/sniper_host/src/sha.rs`: 20:21 `%`→`+` in `sha256`, and 157:26 `%`→`+` in `Sha256::finish`.
       Each was cut at the auto 24 s.
   - **Unviable (17).**
     - A1 (1): `opt_pos/src/price.rs:311:25`.
     - B2 (4): `wf_sweep/src/lists.rs` 93:13, 94:13 and 186:9, and `views.rs` 32:5.
     - B1 (12): `sniper_host/src/sweep.rs` 8, `wfio.rs` 3, `size.rs` 1.
   - **Flaky catches:** not assessed. Each mutant ran once; no reruns.
   - **Restarts:** none. `box.sh all 7` was started once; no VM reclaim, no resume.
   - **Warnings:** no WARN or ERROR line in any of the four cargo-mutants logs.
   - **Polling.** The harness refused a bare `sleep 240; ...`. Every poll used READY.txt's fallback,
     `timeout 240 tail -n 0 -f ~/logs/box.log`, then `status`.
   - **Progress pushes** before this one: 16:15:24 UTC (`17c31cd`) and 16:27:46 UTC (`640cacf`).

7. **The 10 files with the most MISSED mutants** (this shard's slice only; a file cut by a slice boundary is partial).
   The full list is in each set's `missed.txt`.

   | missed | set | file |
   |---:|---|---|
   | 128 | B1 | `crates/sniper_host/src/secrun.rs` |
   | 89 | A1 | `crates/opt_pos/src/secs.rs` |
   | 46 | B2 | `crates/wf_sweep/src/lists.rs` |
   | 21 | B1 | `crates/sniper_host/src/wfio.rs` |
   | 20 | A1 | `crates/opt_pos/src/price.rs` |
   | 18 | B1 | `crates/sniper_host/src/words.rs` |
   | 17 | B1 | `crates/sniper_host/src/sweep.rs` |
   | 10 | B1 | `crates/sniper_host/src/twin.rs` |
   | 9 | B1 | `crates/sniper_host/src/sha.rs` |
   | 9 | B1 | `crates/sniper_host/src/size.rs` |

   MISSED by crate: `sniper_host` 214, `opt_pos` 114, `wf_sweep` 54, `costs` 0.

8. Not applicable: the per-package coverage table belongs to box 0.

9. No market data was read, fetched or written; no link or secret was used.

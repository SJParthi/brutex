# NOTES: idxopt mutants, box 7 = JOB 6 (shard 6 of 8)

1. **Source.**
   - Branch `cloud/idxopt-v3-20261116`, commit `10dacdd629ee7052c4ca3fe2312b3af275cda172`.
     It equals the commit READY.txt names (MAIN e462fe4d).
   - `JOB-IDXOPT.md` sha256 `183ad0d58c71f74b3ca629b532a027cf44e10a25527c97c9e985585d6e1e3552`, as READY.txt names.
   - The extracted `box.sh` sha256 is `2e54ba7c260a43c2e217dc11e71c1aad95eebc83a6c59dbeb8370112e5783124`, as section 5 step 2 requires.
   - Manifest check: OK, 366 files, sha256 `e082593fbc5101f8691bc68b08846e44bc5a0388be924c91d79f75416d4cff30`.

2. **Box facts** (`FACTS.txt`): Linux 6.18.44-fc-v470 x86_64, nproc 4, MemTotal 16,473,776 KiB, cgroup memory.max n/a,
   30,304,116 KiB free disk, rustc 1.97.1 (8bab26f4f 2026-07-14), cargo 1.97.1 (c980f4866 2026-06-30), cargo-mutants 27.1.0.

3. **Baselines** (all equal the expected counts):
   ```
   BASELINE set=A1 rc=0 passed_failed_ignored_filtered="73 0 0 1" expected="73 0 0 1"
   BASELINE set=A2 rc=0 passed_failed_ignored_filtered="440 0 2 0" expected="440 0 2 0"
   BASELINE set=B2 rc=0 passed_failed_ignored_filtered="24 0 0 2" expected="24 0 0 2"
   BASELINE set=B1 rc=0 passed_failed_ignored_filtered="73 0 0 4" expected="73 0 0 4"
   ```

4. **SUMMARY lines:**
   ```
   SUMMARY idxopt-mutants shard=6/8 set=A1 mutants=283 caught=146 missed=132 timeout=0 unviable=5 wall_s=129 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
   SUMMARY idxopt-mutants shard=6/8 set=A2 mutants=58 caught=53 missed=5 timeout=0 unviable=0 wall_s=59 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
   SUMMARY idxopt-mutants shard=6/8 set=B2 mutants=123 caught=101 missed=19 timeout=0 unviable=3 wall_s=115 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
   SUMMARY idxopt-mutants shard=6/8 set=B1 mutants=481 caught=4 missed=472 timeout=0 unviable=5 wall_s=1295 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
   SUMMARY idxopt-mutants shard=6/8 set=ALL mutants=945 caught=304 missed=628 timeout=0 unviable=13 wall_s=1598 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0
   ```

5. **Wall time** (MEASURED, from `box.log`):

   | step | wall |
   |---|---|
   | setup (cargo-mutants 27.1.0 install about 61 s) | 16:01:39 to 16:02:42 UTC, about 63 s |
   | baselines | 16:02:42 to 16:03:19 UTC, about 37 s |
   | A1 | 129 s |
   | A2 | 59 s |
   | B2 | 115 s |
   | B1 | 1,295 s |
   | all four sets | 1,598 s |
   | whole box, start to `BOX DONE` | 16:01:39 to 16:29:59 UTC = 1,700 s (28 min 20 s) |

   This is far below section 9's ESTIMATE of 2 to 4.5 h. B1 mutants took about 1 to 2 s to build and 3 to 4 s to test.

6. **Unusual:**
   - **Polling.** The harness refused `sleep 240; bash ~/box.sh status 6`.
     - Every poll used the fallback READY.txt approves: `timeout 240 tail -n 0 -f ~/logs/box.log; bash ~/box.sh status 6`.
     - `progress` ran every third poll. Its pushes were at 16:14:12 and 16:26:27 UTC.
   - **No timeouts and no restarts** (no VM loss).
     - Flaky catches: none seen, but each mutant ran once, so flakiness was not measured.
     - No warnings in the four cargo-mutants logs.
   - **Mutant counts per set equal section 4's counts for k = 6** (283, 58, 123, 481; 945 in all). Those counts came from
     cargo-mutants 26.2.0.
   - **B1: 472 of 481 MISSED.** This is a consequence of the scope. It is not a broken run:
     - All 481 mutants of this shard's B1 slice are in `crates/sniper_host/src/secrun.rs` (351) and
       `crates/sniper_host/src/sec.rs` (130).
     - Of `sniper_host`'s integration test targets, only `tests/sec.rs` and `tests/secdials.rs` name these modules. Section 4
       leaves both targets unbuilt in B1, because they read data by absolute Mac paths.
     - A sampled mutant log (`secrun.rs:105:5`, `read_cells` replaced with `Ok(())`) shows the in-scope tests ran and passed:
       51 + 1 + 19 + 2 = 73 passed and 4 filtered, the baseline counts.
     - So read these 472 as "not reached by the tests that can run here". They are not the Mac's mutation score for these
       files.
     - The 4 caught mutants are `sec.rs:1309:5` and `secrun.rs:97:20`, `258:44` and `568:28`.
   - **No pull request.** `git push` printed GitHub's usual "create a pull request" hint. No pull request was opened.

7. **The files with the most MISSED mutants**, all sets of this shard. Only 7 files have any.

   | MISSED | file | set |
   |---:|---|---|
   | 343 | `crates/sniper_host/src/secrun.rs` | B1 |
   | 129 | `crates/sniper_host/src/sec.rs` | B1 |
   | 83 | `crates/opt_pos/src/pos.rs` | A1 |
   | 49 | `crates/opt_pos/src/price.rs` | A1 |
   | 18 | `crates/wf_sweep/src/acc.rs` | B2 |
   | 5 | `crates/costs/src/option_clock.rs` | A2 |
   | 1 | `crates/wf_sweep/src/lists.rs` | B2 |

   Each set's `missed.txt` holds the full list.

9. **Data and secrets.** No market data was read, fetched or written. No presigned link, token or secret was used. Only the
   public code branch was cloned, and only `results/` was pushed.

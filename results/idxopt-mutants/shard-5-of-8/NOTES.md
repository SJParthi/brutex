# NOTES: idxopt mutants, shard 5 of 8 (box 6, JOB 5)

1. **Source.** Branch `cloud/idxopt-v3-20261116`, commit `10dacdd629ee7052c4ca3fe2312b3af275cda172`, equal to the commit
   READY.txt named (section 5, step 1). `JOB-IDXOPT.md` sha256 `183ad0d58c71f74b3ca629b532a027cf44e10a25527c97c9e985585d6e1e3552`
   and `box.sh` sha256 `2e54ba7c260a43c2e217dc11e71c1aad95eebc83a6c59dbeb8370112e5783124` (step 2), both as required.
   Manifest check by `box.sh setup`: 366 files, sha256 `e082593fbc5101f8691bc68b08846e44bc5a0388be924c91d79f75416d4cff30`,
   equal to the expected one.

2. **Box facts** (`FACTS.txt`): Linux 6.18.44-fc-v470 x86_64; nproc 4; MemTotal 16,473,776 KiB; cgroup memory.max n/a;
   disk free 29,729,308 KiB; rustc 1.97.1 (8bab26f4f 2026-07-14); cargo 1.97.1 (c980f4866 2026-06-30); cargo-mutants 27.1.0.

3. **Baselines** (unmutated; all four equal to section 4):

   ```
   BASELINE set=A1 rc=0 passed_failed_ignored_filtered="73 0 0 1" expected="73 0 0 1" 2026-11-16 16:04:32 UTC
   BASELINE set=A2 rc=0 passed_failed_ignored_filtered="440 0 2 0" expected="440 0 2 0" 2026-11-16 16:04:39 UTC
   BASELINE set=B2 rc=0 passed_failed_ignored_filtered="24 0 0 2" expected="24 0 0 2" 2026-11-16 16:04:44 UTC
   BASELINE set=B1 rc=0 passed_failed_ignored_filtered="73 0 0 4" expected="73 0 0 4" 2026-11-16 16:05:03 UTC
   ```

4. **SUMMARY lines:**

   ```
   SUMMARY idxopt-mutants shard=5/8 set=A1 mutants=283 caught=146 missed=136 timeout=0 unviable=1 wall_s=145 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
   SUMMARY idxopt-mutants shard=5/8 set=A2 mutants=58 caught=58 missed=0 timeout=0 unviable=0 wall_s=62 rc=0 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
   SUMMARY idxopt-mutants shard=5/8 set=B2 mutants=123 caught=106 missed=15 timeout=0 unviable=2 wall_s=91 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
   SUMMARY idxopt-mutants shard=5/8 set=B1 mutants=481 caught=2 missed=475 timeout=0 unviable=4 wall_s=1369 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
   SUMMARY idxopt-mutants shard=5/8 set=ALL mutants=945 caught=312 missed=626 timeout=0 unviable=7 wall_s=1667 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0
   ```

5. **Wall (MEASURED**, from the `box.log` stamps and `wall_s`): setup 73 s (16:03:09 to 16:04:22 UTC, of which the
   cargo-mutants 27.1.0 install took 69 s); baselines 41 s (to 16:05:03); A1 145 s; A2 62 s; B2 91 s; B1 1,369 s; the four
   sets 1,667 s (27 min 47 s); the whole box 1,782 s (29 min 42 s, 16:03:09 to 16:32:51 UTC). Section 9's ESTIMATE was
   about 2 to 4.5 h.

6. **Unusual:**
   - The mutant count per set (283 / 58 / 123 / 481, 945 in all) equals section 4's 26.2.0 count for k = 0 to 6 exactly.
   - Each set's shard-5 slice lies in a single file (from each set's `mutants.json`): A1 all 283 in `crates/opt_pos/src/pos.rs`;
     A2 all 58 in `crates/costs/src/option_clock.rs`; B2 all 123 in `crates/wf_sweep/src/acc.rs`; B1 all 481 in
     `crates/sniper_host/src/sec.rs`.
   - **B1: 475 of 481 MISSED** (2 caught, 4 unviable). I checked (read-only) that this is not a harness fault. cargo-mutants'
     own B1 baseline log and the log of the missed mutant `sec.rs:1060:65` both show the whole B1 set running and passing:
     51 + 1 + 19 + 2 = 73 passed, 4 filtered, as in BASELINE. So the B1 tests that can run here do not reach most of `sec.rs`.
     Not checked: whether the `sec` and `secdials` targets that section 4 leaves out would reach it.
   - A1 and B2 have the same shape. Each mutant runs its own package's tests (the 27.1.0 default). The missed `pos.rs:405:89`
     mutants ran opt_pos's 60 lib tests (1 filtered), and the missed `acc.rs:675:20` (`+=` to `*=`) ran wf_sweep's 15 + 2 + 0
     tests (2 filtered). Each count equals cargo-mutants' own baseline for that set, and every test passed.
   - No timeouts and no restarts or resumes. Each set ran once, so flaky catches cannot be seen. cargo-mutants itself printed
     no warning or error line in the four logs.
   - Polling: the session's shell refused `sleep 240; bash ~/box.sh status 5`, so each poll was READY.txt's fallback,
     `timeout 240 tail -n 0 -f ~/logs/box.log` followed by `bash ~/box.sh status 5`. Progress was pushed at 16:15:58 and
     16:28:13 UTC.
   - Outside the JOB, before READY arrived, this VM installed cargo-mutants 26.2.0 (the repository CI's pin) and ran
     `cargo fetch --locked` in the session's own separate brutex checkout. `box.sh setup` then installed 27.1.0 as the JOB
     says, and `FACTS.txt` shows 27.1.0. Nothing in `~/idxopt` was touched by either.

7. **Files with the most MISSED mutants.** Only three files have any. The full lists are in each set's `missed.txt`.

   | missed | set | file |
   |---:|---|---|
   | 475 | B1 | `crates/sniper_host/src/sec.rs` |
   | 136 | A1 | `crates/opt_pos/src/pos.rs` |
   | 15 | B2 | `crates/wf_sweep/src/acc.rs` |

8. Box 0 only: not applicable.

9. No market data was read, fetched or written; no link or secret was used.

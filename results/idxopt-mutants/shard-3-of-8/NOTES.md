# NOTES: idxopt mutants, shard 3 of 8 (box 4, JOB 3)

## 1. Source
- Branch `cloud/idxopt-v3-20261116`, commit `10dacdd629ee7052c4ca3fe2312b3af275cda172` (the commit READY.txt names; `git rev-parse HEAD` of the clone equal to it).
- `JOB-IDXOPT.md` sha256 `183ad0d58c71f74b3ca629b532a027cf44e10a25527c97c9e985585d6e1e3552`, as READY.txt states.
- `~/box.sh` extracted by section 5 step 2: `bash -n` clean, sha256 `2e54ba7c260a43c2e217dc11e71c1aad95eebc83a6c59dbeb8370112e5783124`, as required.
- Manifest check (box.sh setup): OK, 366 files, sha256 `e082593fbc5101f8691bc68b08846e44bc5a0388be924c91d79f75416d4cff30` (expected the same).

## 2. Box facts (FACTS.txt)
- Linux 6.18.44-fc-v470 x86_64; nproc 4; MemTotal 16,473,776 KiB; cgroup memory.max n/a; disk free 30,322,592 KiB at setup.
- rustc 1.97.1 (8bab26f4f 2026-07-14); cargo 1.97.1 (c980f4866 2026-06-30); cargo-mutants 27.1.0.
- rustup was already present; the 1.97.1 check passed in both workspaces. cargo-mutants 27.1.0 was installed by setup (`--locked`).

## 3. Baselines (BASELINE.txt): all four equal to section 4
```
BASELINE set=A1 rc=0 passed_failed_ignored_filtered="73 0 0 1" expected="73 0 0 1" 2026-11-16 16:04:07 UTC
BASELINE set=A2 rc=0 passed_failed_ignored_filtered="440 0 2 0" expected="440 0 2 0" 2026-11-16 16:04:15 UTC
BASELINE set=B2 rc=0 passed_failed_ignored_filtered="24 0 0 2" expected="24 0 0 2" 2026-11-16 16:04:20 UTC
BASELINE set=B1 rc=0 passed_failed_ignored_filtered="73 0 0 4" expected="73 0 0 4" 2026-11-16 16:04:39 UTC
```
Each cargo-mutants run's own unmutated baseline also passed: A1 10 s build + 0 s test, A2 5 s + 1 s, B2 2 s + 1 s,
B1 13 s + 3 s. Auto-set test timeout: 20 s in all four.

## 4. SUMMARY lines
```
SUMMARY idxopt-mutants shard=3/8 set=A1 mutants=283 caught=162 missed=114 timeout=0 unviable=7 wall_s=136 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=3/8 set=A2 mutants=58 caught=57 missed=1 timeout=0 unviable=0 wall_s=61 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=3/8 set=B2 mutants=123 caught=97 missed=19 timeout=0 unviable=7 wall_s=73 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=3/8 set=B1 mutants=481 caught=302 missed=157 timeout=2 unviable=20 wall_s=1068 rc=3 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=3/8 set=ALL mutants=945 caught=618 missed=291 timeout=2 unviable=34 wall_s=1338 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0
```
The per-set mutant counts equal section 4's EXACT counts for shard k = 0 to 6 (283 / 58 / 123 / 481 = 945), and in each
set caught + missed + timeout + unviable equals the set's mutant count.

## 5. Wall (MEASURED, box.log and SUMMARY wall_s)
| step | wall | per mutant |
|---|---:|---:|
| setup (cargo-mutants install 65 s) | 68 s | - |
| baselines, 4 sets | 43 s | - |
| A1 | 136 s | 0.48 s |
| A2 | 61 s | 1.05 s |
| B2 | 73 s | 0.59 s |
| B1 | 1,068 s | 2.22 s |
| the four mutant sets | 1,338 s (22 min 18 s) | 1.42 s |
| **whole box**, 16:02:48 to 16:26:57 UTC | **1,449 s (24 min 9 s)** | - |

The section 9 ESTIMATE was about 2 to 4.5 h. This box measured about 24 min.

## 6. Unusual
- **2 TIMEOUT, both in B1:** `crates/sniper_host/src/posgroups.rs:129:19` and `:142:19`, `replace += with *= in exit_of`.
  Each mutates the loop step `j += 1;` of a `while let Some(x) = s.get(j)` loop. With `j *= 1` the index never
  advances, so the loop does not end, and the 20 s auto timeout killed the test run. This is why B1 ended with rc 3.
- No warnings in the four cargo-mutants logs. Besides the MISSED and TIMEOUT outcome lines, each holds only `Found`, the
  unmutated baseline, the INFO auto-timeout line and the final tally. No test failed in any baseline.
- Flaky catches: none seen. Each set ran once, and no rerun was made to test for flakiness.
- No restarts and no resume: one `box.sh all 3` run, start to `BOX DONE`, no VM loss.
- Polling: this box's harness refused `sleep 240; ...`, so each poll was
  `timeout 240 tail -n 0 -f ~/logs/box.log; bash ~/box.sh status 3`, the fallback READY.txt names. Progress pushes:
  `bf9d900` (16:15:42 UTC) and `f5f4b23` (16:28:11 UTC), then this final one.
- MISSED share: A1 114 of 283 (40%), A2 1 of 58, B2 19 of 123 (15%), B1 157 of 481 (33%).

## 7. The 10 files with the most MISSED mutants
Paths are relative to the set's workspace root (A = `sniper-host-v3-idx-w4r3/pinned/w4m/opening5s-5b3r70`, B = `sniper-host-v3-idx-w4r3`).

| missed | set | file |
|---:|---|---|
| 66 | A1 | `crates/opt_pos/src/intraday.rs` |
| 64 | B1 | `crates/sniper_host/src/price.rs` |
| 40 | B1 | `crates/sniper_host/src/monitor.rs` |
| 24 | B1 | `crates/sniper_host/src/posgroups.rs` |
| 23 | A1 | `crates/opt_pos/src/chain.rs` |
| 17 | B1 | `crates/sniper_host/src/instr.rs` |
| 12 | B2 | `crates/wf_sweep/src/acc.rs` |
| 11 | A1 | `crates/opt_pos/src/newpx.rs` |
| 10 | B1 | `crates/sniper_host/src/idxpos.rs` |
| 9 | A1 | `crates/opt_pos/src/ladder.rs` |

The other 15: B2 `crates/wf_sweep/src/lib.rs` 7, A1 `crates/opt_pos/src/freeze.rs` 5, B1 `crates/sniper_host/src/newday.rs` 2,
and A2 `crates/costs/src/option_clock.rs` 1 (line 196, `replace || with && in side`). That makes 291 in all. The full
lists are in each set's `missed.txt`.

## 8. Coverage (box 0 only)
Not applicable to this box.

## 9. Data and secrets
No market data was read, fetched or written; no link or secret was used. The only network use was cloning and pushing
this public repository and fetching cargo-mutants 27.1.0 and the 8 locked crates from crates.io.

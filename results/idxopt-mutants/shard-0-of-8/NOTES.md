# NOTES: idxopt mutants, shard 0 of 8 (box 1, JOB 0)

## 1. Source
- Branch `cloud/idxopt-v3-20261116`, commit `10dacdd629ee7052c4ca3fe2312b3af275cda172`, cloned by step 1;
  `git rev-parse HEAD` equals the commit READY.txt names.
- `JOB-IDXOPT.md` sha256 `183ad0d58c71f74b3ca629b532a027cf44e10a25527c97c9e985585d6e1e3552` (as READY.txt names).
- `~/box.sh` extracted by step 2: sha256 `2e54ba7c260a43c2e217dc11e71c1aad95eebc83a6c59dbeb8370112e5783124` (as required).
- Manifest check by `box.sh setup`: 366 files, sha256 `e082593fbc5101f8691bc68b08846e44bc5a0388be924c91d79f75416d4cff30`,
  equal to the expected value.

## 2. Box facts (FACTS.txt)
- Linux 6.18.44-fc-v470 x86_64; nproc 4; mem_total_kib 16473776; cgroup memory.max n/a (not visible in the container,
  but the kernel does apply a memory-cgroup limit to this session's commands: see 6).
- disk_free_kib 30322508.
- rustc 1.97.1 (8bab26f4f 2026-07-14); cargo 1.97.1 (c980f4866 2026-06-30); cargo-mutants 27.1.0 (installed by setup).

## 3. Baselines (BASELINE.txt): all four equal the expected EXACT counts, rc 0
```
BASELINE set=A1 rc=0 passed_failed_ignored_filtered="73 0 0 1" expected="73 0 0 1" 2026-11-16 16:03:44 UTC
BASELINE set=A2 rc=0 passed_failed_ignored_filtered="440 0 2 0" expected="440 0 2 0" 2026-11-16 16:03:51 UTC
BASELINE set=B2 rc=0 passed_failed_ignored_filtered="24 0 0 2" expected="24 0 0 2" 2026-11-16 16:03:55 UTC
BASELINE set=B1 rc=0 passed_failed_ignored_filtered="73 0 0 4" expected="73 0 0 4" 2026-11-16 16:04:14 UTC
```

## 4. SUMMARY lines
```
SUMMARY idxopt-mutants shard=0/8 set=A1 mutants=283 caught=136 missed=136 timeout=3 unviable=8 wall_s=114 rc=3 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=0/8 set=A2 mutants=58 caught=47 missed=11 timeout=0 unviable=0 wall_s=69 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=0/8 set=B2 mutants=123 caught=69 missed=46 timeout=3 unviable=5 wall_s=76 rc=3 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=0/8 set=B1 mutants=481 caught=90 missed=380 timeout=2 unviable=9 wall_s=1335 rc=3 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=0/8 set=ALL mutants=945 caught=342 missed=573 timeout=8 unviable=22 wall_s=1594 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0
```
- The mutant counts equal section 4's table for shard k = 0 to 6 exactly (283, 58, 123, 481; 945).
- What shard 0 holds (slice sharding, list in path order), EXACT from `outcomes.json`:
  - A1: `idx_cal` only (bhav.rs 76, cal.rs 117, crc.rs 30, day.rs 43, lib.rs 17); no `opt_pos` or `idx_f1` mutant.
  - A2: `costs/src/option_clock.rs` 58.
  - B2: `sniper_kernel` only (bits.rs 46, code.rs 36, ladder.rs 38, lib.rs 3); no `wf_sweep` mutant.
  - B1: `sniper_host` cands.rs 364, ctxdials.rs 112, lib.rs 5.

## 5. Wall (MEASURED, box.log timestamps and SUMMARY wall_s)
- setup 73 s (16:02:21 to 16:03:34 UTC; the cargo-mutants install took 71 s of it).
- baselines 40 s (16:03:34 to 16:04:14).
- A1 114 s, A2 69 s, B2 76 s, B1 1335 s: mutant sets 1594 s.
- In all 1707 s = 28 min 27 s (16:02:21 to 16:30:48 UTC), one run, no resume.
- Wall per mutant with `--jobs 2` (MEASURED): A1 0.40 s, A2 1.19 s, B2 0.62 s, B1 2.78 s. Section 9's ESTIMATE (about
  2 to 4.5 h, 3 to 20 s a mutant) was well above this.

## 6. Unusual
- **One OOM kill (MEASURED, dmesg).** During B2 the kernel's memory-cgroup OOM killer killed the test binary
  `sniper_kernel-16f0801d69585803` (anon RSS 7,045,632 kB) of mutant
  `crates/sniper_kernel/src/bits.rs:71:28: replace - with + in Bits::ones`.
  - Its log shows `signal: 9, SIGKILL`; cargo test exited 101, so cargo-mutants recorded it CAUGHT.
  - Read that CAUGHT as "killed by runaway memory", not as a failed assertion.
  - Its neighbours on the same line (`- with /` at col 28, `&= with |=` at col 23) are TIMEOUTs.
  - It was the only OOM kill of the run (dmesg count 1 at the end). No build was killed; no phase in any set ended
    on a bare signal (every non-success phase is a timeout or exit 101).
- **8 timeouts** (auto test timeout 20 s in every set):
  - A1: `idx_cal/src/cal.rs:143:15 replace == with != in find`; `cal.rs:346:15 replace += with -= in build`;
    `cal.rs:346:15 replace += with *= in build`.
  - B2: `sniper_kernel/src/bits.rs:71:23 replace &= with |= in Bits::ones`; `bits.rs:71:28 replace - with / in
    Bits::ones`; `ladder.rs:142:18 replace < with <= in Ladder::bucket`.
  - B1: `sniper_host/src/cands.rs:894:15 replace += with *= in htf_slots`; `cands.rs:901:15 replace += with *= in
    htf_slots`.
- **B1 catches 90 of 481.** The selected tests do run in full on every mutant: the log of a missed mutant
  (`sniper_host/src/lib.rs:63:5 replace law_line -> String with String::new()`) sums to 73 passed, 0 failed,
  0 ignored, 4 filtered (the B1 baseline), and the mean test phase of the 380 missed B1 mutants is 4.26 s against the
  3.67 s baseline. Whether the targets not built here (`h1`, `sec`, `dials`, `secdials`, `f1`) would catch them is
  not measured.
- A1's tests take 0.05 s (baseline test phase 0.052 s; missed mutants 0.053 s on average), which is why A1 is fast.
- No WARN or ERROR line in the four cargo-mutants logs. No restart; box.sh ran once.
- The shell refused `sleep 240; ...`; each poll used `timeout 240 tail -n 0 -f ~/logs/box.log` then `status`, as
  READY.txt allows. Progress pushes at 16:16 and 16:28 UTC, then the final one.

## 7. The 10 files with the most MISSED mutants (missed / mutants in this shard)
| missed | of | file | set |
|---:|---:|---|---|
| 329 | 364 | `crates/sniper_host/src/cands.rs` | B1 |
| 62 | 117 | `crates/idx_cal/src/cal.rs` | A1 |
| 56 | 76 | `crates/idx_cal/src/bhav.rs` | A1 |
| 46 | 112 | `crates/sniper_host/src/ctxdials.rs` | B1 |
| 20 | 46 | `crates/sniper_kernel/src/bits.rs` | B2 |
| 16 | 17 | `crates/idx_cal/src/lib.rs` | A1 |
| 16 | 36 | `crates/sniper_kernel/src/code.rs` | B2 |
| 11 | 58 | `crates/costs/src/option_clock.rs` | A2 |
| 9 | 38 | `crates/sniper_kernel/src/ladder.rs` | B2 |
| 5 | 5 | `crates/sniper_host/src/lib.rs` | B1 |

The full lists are in `<set>/missed.txt`.

## 8. Box 0 only
Not applicable to this box.

## 9. Data
No market data was read, fetched or written; no presigned link, token or secret was used.

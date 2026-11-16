# NOTES: idxopt box 5 (JOB k = 4, cargo-mutants shard 4 of 8)

Result branch `cloud-results/idxopt-mutants-4-of-8`. Labels as in JOB-IDXOPT.md: MEASURED, ESTIMATE, EXACT.

## 1. Source
- Branch `cloud/idxopt-v3-20261116`, commit `10dacdd629ee7052c4ca3fe2312b3af275cda172` (equals READY.txt's commit and
  the branch tip; `git rev-parse HEAD` of the fresh clone `~/idxopt`).
- `JOB-IDXOPT.md` sha256 `183ad0d58c71f74b3ca629b532a027cf44e10a25527c97c9e985585d6e1e3552` (as READY.txt names it).
- `~/box.sh` extracted with section 5 step 2: sha256 `2e54ba7c260a43c2e217dc11e71c1aad95eebc83a6c59dbeb8370112e5783124` (as required).
- Manifest check by `box.sh setup`: 366 files, sha256 `e082593fbc5101f8691bc68b08846e44bc5a0388be924c91d79f75416d4cff30` (expected). OK.

## 2. Box facts (FACTS.txt)
- Linux 6.18.44-fc-v470 x86_64; nproc 4; MemTotal 16,473,776 KiB; cgroup memory.max n/a; disk free 30,354,516 KiB at setup.
- rustc 1.97.1 (8bab26f4f 2026-07-14); cargo 1.97.1 (c980f4866 2026-06-30).
- cargo-mutants 27.1.0, installed by setup (`cargo install cargo-mutants --locked --version 27.1.0`).

## 3. Baselines (BASELINE.txt): all four rc 0 and equal to the expected counts
```
BASELINE set=A1 rc=0 passed_failed_ignored_filtered="73 0 0 1" expected="73 0 0 1" 2026-11-16 16:04:09 UTC
BASELINE set=A2 rc=0 passed_failed_ignored_filtered="440 0 2 0" expected="440 0 2 0" 2026-11-16 16:04:14 UTC
BASELINE set=B2 rc=0 passed_failed_ignored_filtered="24 0 0 2" expected="24 0 0 2" 2026-11-16 16:04:18 UTC
BASELINE set=B1 rc=0 passed_failed_ignored_filtered="73 0 0 4" expected="73 0 0 4" 2026-11-16 16:04:33 UTC
```

## 4. SUMMARY lines (SUMMARY.txt)
```
SUMMARY idxopt-mutants shard=4/8 set=A1 mutants=283 caught=168 missed=107 timeout=2 unviable=6 wall_s=147 rc=3 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=4/8 set=A2 mutants=58 caught=58 missed=0 timeout=0 unviable=0 wall_s=53 rc=0 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=4/8 set=B2 mutants=123 caught=92 missed=31 timeout=0 unviable=0 wall_s=74 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=4/8 set=B1 mutants=481 caught=118 missed=342 timeout=2 unviable=19 wall_s=1189 rc=3 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=4/8 set=ALL mutants=945 caught=436 missed=480 timeout=4 unviable=25 wall_s=1463 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0
```
- In every set caught + missed + timeout + unviable equals the set's mutant count (EXACT).
- rc 2 and rc 3 are among the codes `box.sh` accepts as a finished set (0, 2, 3).

## 5. Wall (MEASURED, box.log timestamps and box.sh `wall_s`)
| step | wall | per mutant (wall, `--jobs 2`, 4 vCPU) |
|---|---:|---:|
| setup (16:02:58 to 16:04:00 UTC; cargo-mutants install 60 s of it) | 62 s | - |
| baselines, 4 unmutated sets (16:04:00 to 16:04:33) | 33 s | - |
| A1 | 147 s | 0.52 s |
| A2 | 53 s | 0.91 s |
| B2 | 74 s | 0.60 s |
| B1 | 1,189 s | 2.47 s |
| the four sets | 1,463 s (24 min 23 s) | 1.55 s |
| **whole box (16:02:58 to 16:28:56 UTC)** | **1,558 s (25 min 58 s)** | - |

- Section 9's ESTIMATE was about 2 to 4.5 h for a mutants box; this box took 26 min.
- Each cargo-mutants run's own unmutated baseline: A1 7 s build + 0 s test, A2 4 s + 1 s, B2 2 s + 1 s, B1 12 s + 3 s.

## 6. Anything unusual
- **Mutant counts equal section 4's table exactly** for a shard k = 0 to 6: 283 / 58 / 123 / 481 = 945 (the table is
  cargo-mutants 26.2.0 on the Mac; this run is 27.1.0).
- **4 timeouts**, at cargo-mutants' auto-set test timeout of 20 s (auto-set to 20 s in all four runs):
  - A1 `crates/opt_pos/src/pack.rs:112:25: replace += with *= in Live::advance`
  - A1 `crates/opt_pos/src/pack.rs:300:14: replace == with != in put_var`
  - B1 `crates/sniper_host/src/price.rs:605:19: replace % with / in encode`
  - B1 `crates/sniper_host/src/price.rs:605:19: replace % with + in encode`
- **B1's miss rate is high: 342 of 481 (71%)**, and 249 of them are in `search.rs` (249 of the 284 `search.rs` mutants in
  this shard). Section 4 says B1 runs here without the 5 data-only test targets (`h1`, `sec`, `dials`, `secdials`, `f1`)
  and with 4 matrix tests skipped. Whether those would catch these mutants was not measured here: UNVERIFIED.
- No restart (one `nohup` run, never resumed); no warning, error or panic line in the four cargo-mutants logs; no skip
  added; no flag beyond `box.sh`'s own.
- Flaky catches: not assessable here (each mutant was tested once).
- Progress pushes: `85c5b59` (16:15:30 UTC) and `ec95ce8` (16:27:41 UTC), then the final push carrying this file.
- This session's shell refused `sleep 240; bash ~/box.sh status 4`, so every poll waited with
  `timeout 240 tail -n 0 -f ~/logs/box.log` and then ran `status`, as READY.txt allows.

## 7. Files with the most MISSED mutants
Only 9 files have any MISSED mutant in this shard (a shard is one contiguous slice of each set's list). The full lists are
in `<set>/missed.txt`. Paths are relative to each workspace root: A = `sniper-host-v3-idx-w4r3/pinned/w4m/opening5s-5b3r70/`,
B = `sniper-host-v3-idx-w4r3/`. Counts are EXACT, from `<set>/missed.txt` and `<set>/outcomes.json`.

| # | set | file | missed | that file's mutants in this shard |
|---:|---|---|---:|---:|
| 1 | B1 | `crates/sniper_host/src/search.rs` | 249 | 284 |
| 2 | A1 | `crates/opt_pos/src/pos.rs` | 66 | 97 |
| 3 | B1 | `crates/sniper_host/src/rows.rs` | 51 | 80 |
| 4 | A1 | `crates/opt_pos/src/pack.rs` | 39 | 157 |
| 5 | B1 | `crates/sniper_host/src/price.rs` | 33 | 72 |
| 6 | B2 | `crates/wf_sweep/src/acc.rs` | 31 | 123 |
| 7 | B1 | `crates/sniper_host/src/sec.rs` | 6 | 7 |
| 8 | B1 | `crates/sniper_host/src/query.rs` | 3 | 38 |
| 9 | A1 | `crates/opt_pos/src/newpx.rs` | 2 | 29 |

- A2 `crates/costs/src/option_clock.rs`: 0 missed of 58.

## 8. Coverage
Not applicable (box 0 only).

## 9. Data and secrets
No market data was read, fetched or written; no link or secret was used.

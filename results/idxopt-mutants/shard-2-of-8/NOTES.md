# NOTES: idxopt mutants, shard 2 of 8 (box 3)

1. **Source.** Branch `cloud/idxopt-v3-20261116`, commit `10dacdd629ee7052c4ca3fe2312b3af275cda172` (the clone's HEAD
   equals the READY commit). `JOB-IDXOPT.md` sha256 `183ad0d58c71f74b3ca629b532a027cf44e10a25527c97c9e985585d6e1e3552`
   and the extracted `box.sh` sha256 `2e54ba7c260a43c2e217dc11e71c1aad95eebc83a6c59dbeb8370112e5783124` both match
   READY.txt. Manifest check OK: 366 files, sha256 `e082593fbc5101f8691bc68b08846e44bc5a0388be924c91d79f75416d4cff30`
   (expected the same).

2. **Box facts** (`FACTS.txt`). Linux 6.18.44-fc-v470 x86_64; nproc 4; MemTotal 16,473,776 KiB; cgroup memory.max n/a;
   disk free 30,322,640 KiB at setup; rustc 1.97.1 (8bab26f4f 2026-07-14); cargo 1.97.1 (c980f4866 2026-06-30);
   cargo-mutants 27.1.0, installed by `box.sh setup` with `--locked`.

3. **Baselines** (all equal the section 4 counts, EXACT):

```
BASELINE set=A1 rc=0 passed_failed_ignored_filtered="73 0 0 1" expected="73 0 0 1" 2026-11-16 16:04:07 UTC
BASELINE set=A2 rc=0 passed_failed_ignored_filtered="440 0 2 0" expected="440 0 2 0" 2026-11-16 16:04:14 UTC
BASELINE set=B2 rc=0 passed_failed_ignored_filtered="24 0 0 2" expected="24 0 0 2" 2026-11-16 16:04:18 UTC
BASELINE set=B1 rc=0 passed_failed_ignored_filtered="73 0 0 4" expected="73 0 0 4" 2026-11-16 16:04:37 UTC
```

4. **SUMMARY lines.**

```
SUMMARY idxopt-mutants shard=2/8 set=A1 mutants=283 caught=92 missed=181 timeout=0 unviable=10 wall_s=123 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=2/8 set=A2 mutants=58 caught=52 missed=4 timeout=0 unviable=2 wall_s=63 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=2/8 set=B2 mutants=123 caught=53 missed=65 timeout=3 unviable=2 wall_s=68 rc=3 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=2/8 set=B1 mutants=481 caught=102 missed=370 timeout=2 unviable=7 wall_s=1284 rc=3 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=2/8 set=ALL mutants=945 caught=299 missed=620 timeout=5 unviable=21 wall_s=1538 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0
```

5. **Wall (MEASURED).** A1 123 s, A2 63 s, B2 68 s, B1 1,284 s; the four sets 1,538 s (25 min 38 s). Whole job
   16:02:46 to 16:30:16 UTC = 1,650 s (27 min 30 s): setup 70 s (of it the cargo-mutants install about 68 s),
   baselines 41 s. Per mutant: A1 0.43 s, A2 1.09 s, B2 0.55 s, B1 2.67 s, all 1.63 s. That is far below the section 9
   ESTIMATE (2 to 4.5 h in all).

6. **Unusual.**
   - 5 timeouts (cargo-mutants auto-set the test timeout to 20 s in every set):
     B2 `crates/sniper_kernel/src/scale.rs` 27:17 `<` to `<=`, 30:52 `==` to `!=`, 31:17 `+=` to `*=` (all in `replicate`);
     B1 `crates/sniper_host/src/idxpos.rs` 236:19 `%` to `/` and `%` to `+` (in `encode_pos`).
   - No restart, no resume. No warning line in the four mutant logs. Each mutant ran once, so flakiness was not
     measured; none was seen.
   - The mutant count of every set equals the section 4 table for shards 0 to 6 (283, 58, 123, 481; 945 in all) under
     27.1.0.
   - Many A1 MISSED lines read "0s build + 0s test": incremental rebuilds round down to 0 s. `ticks.rs` is
     `pub mod ticks` in `idx_cal`'s lib (`lib.rs` line 16), so those mutants were built into the tested lib target.
   - This shard's B1 slice lies entirely in `crates/sniper_host/src/idxpos.rs`.
   - The agent shell refused `sleep 240; ...`, so each poll used READY.txt's alternative
     `timeout 240 tail -n 0 -f ~/logs/box.log`, then `status`. Progress was pushed at 16:15:52 and 16:28:06 UTC, then
     with these notes.

7. **The 10 files with the most MISSED mutants** (these are all 10 files with any MISSED in this shard; full list in
   each set's `missed.txt`; A paths are under `sniper-host-v3-idx-w4r3/pinned/w4m/opening5s-5b3r70/`, B paths under
   `sniper-host-v3-idx-w4r3/`):

| MISSED | set | file |
|---:|---|---|
| 370 | B1 | `crates/sniper_host/src/idxpos.rs` |
| 118 | A1 | `crates/idx_cal/src/ticks.rs` |
| 39 | A1 | `crates/opt_pos/src/chain.rs` |
| 34 | B2 | `crates/sniper_kernel/src/steps.rs` |
| 18 | B2 | `crates/sniper_kernel/src/scale.rs` |
| 15 | A1 | `crates/opt_pos/src/acct.rs` |
| 9 | A1 | `crates/idx_f1/src/host_size.rs` |
| 9 | B2 | `crates/sniper_kernel/src/line.rs` |
| 4 | A2 | `crates/costs/src/option_clock.rs` |
| 4 | B2 | `crates/wf_sweep/src/lib.rs` |

8. Box 0 only: not applicable.

9. No market data was read, fetched or written; no link or secret was used.

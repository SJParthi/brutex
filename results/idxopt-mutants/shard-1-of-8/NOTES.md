# NOTES: idxopt mutants, box 2, JOB 1 (cargo-mutants shard 1 of 8)

1. Source: branch `cloud/idxopt-v3-20261116`, commit `10dacdd629ee7052c4ca3fe2312b3af275cda172` (`git rev-parse HEAD`
   after the clone matched READY.txt). `JOB-IDXOPT.md` sha256 `183ad0d58c71f74b3ca629b532a027cf44e10a25527c97c9e985585d6e1e3552`
   and `box.sh` sha256 `2e54ba7c260a43c2e217dc11e71c1aad95eebc83a6c59dbeb8370112e5783124`, both as READY.txt states.
   Manifest check OK: 366 files, sha256 `e082593fbc5101f8691bc68b08846e44bc5a0388be924c91d79f75416d4cff30`.

2. Box facts (FACTS.txt): Linux 6.18.44-fc-v470 x86_64; nproc 4; mem_total_kib 16473776; cgroup memory.max n/a;
   disk_free_kib 30068760; rustc 1.97.1 (8bab26f4f 2026-07-14); cargo 1.97.1 (c980f4866 2026-06-30);
   cargo-mutants 27.1.0 (installed by setup, `--locked --version 27.1.0`).

3. Baselines (BASELINE.txt), all rc 0 and equal to the expected counts:
   - `BASELINE set=A1 rc=0 passed_failed_ignored_filtered="73 0 0 1" expected="73 0 0 1"`
   - `BASELINE set=A2 rc=0 passed_failed_ignored_filtered="440 0 2 0" expected="440 0 2 0"`
   - `BASELINE set=B2 rc=0 passed_failed_ignored_filtered="24 0 0 2" expected="24 0 0 2"`
   - `BASELINE set=B1 rc=0 passed_failed_ignored_filtered="73 0 0 4" expected="73 0 0 4"`

4. SUMMARY lines (SUMMARY.txt):

```
SUMMARY idxopt-mutants shard=1/8 set=A1 mutants=283 caught=108 missed=169 timeout=0 unviable=6 wall_s=74 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=1/8 set=A2 mutants=58 caught=52 missed=6 timeout=0 unviable=0 wall_s=62 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=1/8 set=B2 mutants=123 caught=59 missed=61 timeout=3 unviable=0 wall_s=54 rc=3 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=1/8 set=B1 mutants=481 caught=151 missed=312 timeout=0 unviable=18 wall_s=1202 rc=2 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=1/8 set=ALL mutants=945 caught=370 missed=548 timeout=3 unviable=24 wall_s=1392 commit=10dacdd629ee7052c4ca3fe2312b3af275cda172 cargo_mutants=27.1.0
```

5. Wall (MEASURED, UTC, from box.log and wall_s):
   - setup 16:02:31 to 16:03:41, 70 s (the cargo-mutants install was 16:02:33 to 16:03:40);
   - baselines 16:03:41 to 16:04:18, 37 s;
   - sets: A1 74 s, A2 62 s, B2 54 s, B1 1,202 s; the four sets 1,392 s (23 min 12 s);
   - box in all: 16:02:31 to `BOX DONE` at 16:27:30, 1,499 s (24 min 59 s).
   - This is far under section 9's ESTIMATE (about 2 to 4.5 h). Wall per mutant with `--jobs 2`: A1 0.26 s, A2 1.07 s,
     B2 0.44 s, B1 2.50 s. cargo-mutants logged most A1 and B2 mutants as "0s build + 0s test".

6. Unusual:
   - 3 timeouts, all in B2, in `crates/sniper_kernel/src/ladder.rs`, `Ladder::bucket`: 143:38 replace / with %,
     143:32 replace - with +, 145:26 replace + with *. The auto-set test timeout was 20 s (B1: 21 s). B2 ended rc 3
     (timeouts); A1, A2 and B1 ended rc 2 (missed). box.sh accepts 0, 2 and 3.
   - This shard's slices: A1 is all `idx_cal` (283 of 283 mutants; `idx_cal`'s lib has 8 unit tests), and B2 is all
     `sniper_kernel` (123 of 123; 7 lib unit tests). MISSED per package: idx_cal 169 of 283, costs (`option_clock.rs`)
     6 of 58, sniper_kernel 61 of 123, sniper_host 312 of 481.
   - Mutant counts per set equal section 4's 26.2.0 counts for shards 0 to 6 exactly: 283, 58, 123, 481 (945).
   - Flaky catches: not measured (each mutant ran once). Restarts: none (one `box.sh all` run, no resume).
   - Warnings: no WARN or ERROR line in the four cargo-mutants logs, and no compiler warning in the four baseline logs.
     No zstd or tool-path error: the only zstd lines are the `ruzstd` crate's build and mutants of `idx_cal`'s `unzstd`.
   - Polling: this session's shell refused `sleep 240; ...`. Each poll was therefore
     `timeout 240 tail -n 0 -f ~/logs/box.log` then `status`, as READY.txt allows. Progress pushes went out at 16:15:42
     (8f07f50) and 16:28:04 (cfea71b).

7. The 10 files with the most MISSED mutants. Paths are relative to each set's workspace root: A =
   `sniper-host-v3-idx-w4r3/pinned/w4m/opening5s-5b3r70/`, B = `sniper-host-v3-idx-w4r3/`. The full lists are in
   `<set>/missed.txt`.

| missed | set | file |
|---:|---|---|
| 146 | B1 | `crates/sniper_host/src/idx.rs` |
| 96 | A1 | `crates/idx_cal/src/ticks.rs` |
| 73 | B1 | `crates/sniper_host/src/esn1.rs` |
| 66 | B1 | `crates/sniper_host/src/ctxdials.rs` |
| 50 | A1 | `crates/idx_cal/src/storeio.rs` |
| 48 | B2 | `crates/sniper_kernel/src/line.rs` |
| 16 | A1 | `crates/idx_cal/src/day.rs` |
| 15 | B1 | `crates/sniper_host/src/db.rs` |
| 13 | B2 | `crates/sniper_kernel/src/ladder.rs` |
| 12 | B1 | `crates/sniper_host/src/idxpos.rs` |

9. No market data was read, fetched or written. No link or secret was used: none was created, printed or asked for,
   and git used only the repository URL of section 5.

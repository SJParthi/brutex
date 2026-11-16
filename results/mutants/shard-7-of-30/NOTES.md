# Mutation slice 7/30 of engine v3.4: notes

Box 7 of 30: cargo-mutants slice 7/30 of the code-only branch `cloud/spikefade-v34int-20261116` at commit
`7a5428d30bdb8a5e0fec427848178c82907de2fb`, run as JOB-MUTANTS.md (sha256 `078206fd...a88a7b74`, checked) says. Nothing
here is a trading result.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- Memory: MemTotal 16,473,776 kB (15.7 GiB), no swap.
- cgroup memory limit: none. The box runs cgroup v1, so section 2's `cat /sys/fs/cgroup/memory.max` printed nothing (the
  file is absent). `/sys/fs/cgroup/memory/memory.limit_in_bytes` reads 9223372036854771712, which is no limit.
- Disk: 30 GB free at the start.

## Toolchain, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`: the pin of `rust-toolchain.toml`.
- `cargo-mutants 27.1.0`, installed with `cargo install cargo-mutants --locked --version 27.1.0` (exit 0).
- Case **A**: all 815 mutants of the slice are in `crates/spikefade`, none in `core`, so there are no case arguments.
  Section 7 (case C only) ran as written and did nothing.
- The exact command (`~/run-main.sh`, in the checkout `~/mut`):

  `cargo mutants --workspace --shard 7/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip <name>` with one
  `--skip` for each of the 17 names of `skips.txt`, in its order (skips sha256 `0cd96e3e...c41fe557`).
- cargo-mutants' own baseline: 15 s build + 4 s test. It auto-set the test timeout to 22 s. No build timeout (default).

## List check

`list_check=MATCH`: 24,429 mutants with sha256 `3a041951...1c15432a`, and slice 7 of 815 mutants with sha256
`81be7935...3b4f5972`, both equal to the table. All 815 are in `crates/spikefade`.

## Baseline (`baseline.log`)

| section | command | rc | wall |
|---|---|---|---|
| B1 | `cargo build --locked --workspace --tests --keep-going` | 101 | 17.9 s |
| B2a | `cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade` | 101 | 6.0 s |
| B2b | `cargo test --locked --no-fail-fast -p core` with the 4 compilable targets | 101 | 0.1 s |
| B3a | B2a with `-- --exact` and the 17 skips | 0 | 6.3 s |
| B3b | B2b with `-- --exact` and the 17 skips | 0 | 0.1 s |

- B1: exactly the 6 expected `could not compile` errors, all `core`: its lib test, and `banned_lists`, `claim_reader`,
  `decision_citations`, `findings` and `graph`. No other target failed to compile.
- B2: 17 failed tests (12 in B2a, 5 in B2b), the same set as `skips.txt`. B2a: 691 passed, 12 failed, 3 ignored over 11
  test targets. B2b: 9 passed, 5 failed over 4 targets.
- B3: 0 failed and rc 0 in both sections (B3a 691 passed, 3 ignored, 12 filtered out; B3b 9 passed, 5 filtered out).
  The check's last line printed 0.
- ENV-EXTRA: none.

## Exit codes and results

- Main run: **exit 2** (some missed). 815 tested in 53 min (15:55:32Z to 16:48:35Z): caught 360, missed 441, timeout 0,
  unviable 14.
- Wall-clock re-check (step 8): 22 timing-only catches (their only FAILED tests are in `timing-tests.txt`).
  - `recheck-list.txt` holds 65 mutants: the 22 names, plus 43 extra `delete field ... from struct ... expression`
    mutants of the same files, which cargo-mutants 27.1.0 does not filter with `-F` (as the JOB expects).
  - Re-check baseline: 18 s build + 3 s test, test timeout auto-set to 20 s. **Exit 2**: 65 tested in 7 min, 20 caught,
    45 missed.
  - Of the 22 names, 20 are MISSED with `--jobs 1` (`recheck_missed=20`) and 2 are caught (`recheck_caught=2`).
  - The 2 still caught are `trade.rs:59:27` `replace * with +` and `replace * with /`. Line 59 is
    `pub const BASE_S: i64 = 9 * 3_600;`, the 09:00:00 base of the u16 second offsets. With `--jobs 1` both again fail
    only the wall-clock test `tests32r::v32r_o1_decision_at_n100_and_sampled_steps`. So their catch rests on a time bound,
    and no value test of this run catches a change to that base.
- Corrected for the re-check (the official files stay as cargo-mutants wrote them):
  - caught = 360 - 20 = 340
  - missed = 441 + 20 = 461

## Anything odd

- The toolchain job (section 2) found every component already up to date, exit 0 in seconds. Before the JOB began, a
  `rustc --version` on this box inside the session's own checkout of the repository, which pins the same 1.97.1, had
  made rustup install it. rustup also checked for, and downloaded, a self-update of itself during that job.
- Section 5's check block exits 1 only because its last `grep -c` counted 0 lines. It printed 0, as required.
- Progress commits were pushed at 16:08, 16:21, 16:35 and 16:48 UTC. `tested=` grew between every pair.
- The READY marker appeared at 15:51 UTC, after polling from 14:53 UTC. `box.start` is 15:52:59Z.

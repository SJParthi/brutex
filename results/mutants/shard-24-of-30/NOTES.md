# Mutation slice 24/30 of engine v3.4: notes (the box writes these; see step 9)

All numbers below are EXACT counts or MEASURED on this box in this run. Nothing here is a trading result.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- Memory: `MemTotal: 16473776 kB` (about 15.7 GiB).
- Cgroup: the box uses cgroup v1, so `/sys/fs/cgroup/memory.max` does not exist (step 2 printed nothing for it). The v1
  limit `/sys/fs/cgroup/memory/memory.limit_in_bytes` is `9223372036854771712`, which means no limit.
- Disk at set-up: 252G device, 30G available.
- Set-up began at 2026-11-16T15:52:24Z (`box.start`), right after the READY marker appeared.

## Toolchain, case and the exact command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, as `rust-toolchain.toml` pins them. The
  toolchain was already installed, so step 2's `rustup toolchain install` reported it `unchanged`. rustup then updated
  itself to 1.29.3, its default behaviour, which did not change the toolchain.
- `cargo-mutants 27.1.0`, from `cargo install cargo-mutants --locked --version 27.1.0` (exit 0, release build 1m 06s).
- Case **A**: every one of the slice's 815 mutants is in `spikefade`, with no `core` mutant, so `case-args.txt` is empty.
- Command (`run-main.sh`, in the checkout at commit `7a5428d30bdb8a5e0fec427848178c82907de2fb`):
  `cargo mutants --workspace --shard 24/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact` followed by
  `--skip <name>` for each of the 17 names in `skips.txt` (sha256
  `0cd96e3e2906f52103be4c894a44beec8c32170a5a8828ea7e2ea302c41fe557`).

## List check

- `list_check=MATCH`. The whole-workspace list has 24,429 mutants (sha256
  `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`). Slice 24 has 815 (sha256
  `a946e0e4e255077dab4850796f32789151d4c478d2e68dea4a5876b3147a3858`). Both are exactly the table's values.
- The slice is one function. All 815 lines are `crates/spikefade/src/fix34.rs:1301:5: replace wick_compare -> (Vec<(u32,
  i64)>, u64, Vec<((i64, u32), (i64, i64, i64, i64))>) with ...`, the return-value replacements section 12 describes.

## Baseline (step 5)

| section | command | rc | wall |
|---|---|---|---|
| B1 | `cargo build --locked --workspace --tests --keep-going` | 101 | 20,793 ms |
| B2a | `cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade` | 101 | 8,275 ms |
| B2b | `cargo test --locked --no-fail-fast -p core` with core's 4 compilable `--test` targets | 101 | 196 ms |
| B3a | B2a's command, plus `-- --exact` and the 17 skips | 0 | 7,798 ms |
| B3b | B2b's command, plus `-- --exact` and the 17 skips | 0 | 71 ms |

- **B1:** exactly the 6 expected `could not compile` errors, all for `core`: its lib test, and the test targets
  `banned_lists`, `claim_reader`, `decision_citations`, `findings` and `graph`. No other target failed to compile.
- **B2:** 17 tests failed, exactly the 17 in `skips.txt` (12 engine, 5 core). Without skips, the engine suite gave
  204 passed, 12 failed and 1 ignored. That is the Mac's 216 passed and 1 ignored, with the 12 engine skips failing here.
- **B3:** 0 failed tests in both sections. The engine suite gave 204 passed, 1 ignored and 12 filtered out.
- **ENV-EXTRA:** none. No test was added to `skips.txt`, and `bl2` was not needed.

## The run (step 6)

- cargo-mutants' own baseline passed: `Unmutated baseline in 17s build + 5s test`. The test timeout was auto-set to 27 s.
- `815 mutants tested in 74m: 815 caught`, from 2026-11-16T15:55:25Z to 2026-11-16T17:09:50Z.
  - Result: 815 caught, 0 missed, 0 timeout, 0 unviable. Exit 0.
  - That is about 5.5 s a mutant, inside section 13's ESTIMATE of 50 to 100 min.
- `main.log` holds no warning or error line.
- Five progress commits were pushed, at 138, 281, 427, 573 and 720 tested. The `tested=` count rose at every one.

## Step 7

- Not run. This is case A, so the block's case-C guard was false.

## The wall-clock re-check (step 8)

- `find-timing.sh` found **0** timing-only catches.
- Checked independently:
  - `outcomes.json` holds 815 `CaughtMutant` outcomes and 1 `Success` (the unmutated baseline).
  - Each of the 815 catch logs names at least one `FAILED` test.
  - 91 catch logs name `split18::tests::v32rb_split_o1_gates` among their failures. None names the other three
    wall-clock tests. Each of those 91 also names at least one test outside `timing-tests.txt`, so none of them is
    timing-only.
- `run-recheck.sh` printed `no timing-only catches` and exited 0 without running cargo-mutants.
  - There is no `recheck-jobs1/` output.
  - `recheck_caught`, `recheck_missed` and `recheck_extra` are 0.
  - The corrected counts equal the official ones.

## Exit codes

- `toolchain` 0, `install` 0, `bl` (`baseline.sh`) 0, `main` 0, `recheck` 0.

## Anything odd

- Nothing failed and nothing was stopped.
- Two things are worth recording, and neither changes a count:
  - rustup updated itself during the toolchain step (above).
  - 91 of the 815 catches had the wall-clock test `split18::tests::v32rb_split_o1_gates` failing alongside other tests
    under `--jobs 2`. This log does not show why that test failed.

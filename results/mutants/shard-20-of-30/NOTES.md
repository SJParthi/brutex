# Mutation slice 20/30 of engine v3.4: notes (the box writes these; see step 9)

Labels as JOB-MUTANTS.md: EXACT = a count, MEASURED = a number from a named run on this box. Nothing here is a trading
result. Code only: no market data was fetched, read or written.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux` (cloud container).
- `nproc`: 4. MemTotal: 16,473,776 kB.
- Memory limit: the box uses cgroup v1, so `/sys/fs/cgroup/memory.max` does not exist and step 2's `cat` printed
  nothing. The memory cgroup the jobs ran in has `memory.limit_in_bytes` = 14,338,772,992 bytes (13.35 GiB). The levels
  above it have no limit.
- Disk at set-up: 29 GB free (`df -P -BG ~`).

## Versions, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by the branch's
  `rust-toolchain.toml`. rustup is 1.29.3.
- `cargo-mutants 27.1.0`.
- Source: branch `cloud/spikefade-v34int-20261116` at `7a5428d30bdb8a5e0fec427848178c82907de2fb`. Step 2 printed
  `HEAD OK`. The sha256 of `JOB-MUTANTS.md` at that commit equals the one READY.txt names (`078206fd...`).
- Case A: the slice has no `core` mutant, so `case-args.txt` is empty.
- The exact command (`run-main.sh`, run from `~/mut`):
  `cargo mutants --workspace --shard 20/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip <name>`, with
  one `--skip` for each of the 17 names in `skips.txt`, in that file's order.
- cargo-mutants' own baseline: 17 s build and 5 s test (MEASURED). It set the test timeout to 29 s. There is no build
  timeout, which is the default.

## List check

- `list_check=MATCH` (EXACT). The whole list is 24,429 mutants, sha256 `3a0419510101562b...`. Slice 20 is 815 mutants,
  sha256 `558091359d50285b...`. Both equal the job's table.
- All 815 mutants of the slice are at one place: `crates/spikefade/src/fix34.rs:1301:5`. Each one replaces the return
  value of `wick_compare`.

## Baseline (step 5)

| section | command | rc | wall (MEASURED) | tests (EXACT) |
|---|---|---|---|---|
| B1 | `cargo build --locked --workspace --tests --keep-going` | 101 | 20,166 ms | 6 `could not compile`, all `core` |
| B2a | `cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade` | 101 | 8,596 ms | 691 passed, 12 failed, 3 ignored |
| B2b | `cargo test --locked --no-fail-fast -p core` with its 4 compilable `--test` targets | 101 | 183 ms | 9 passed, 5 failed |
| B3a | B2a's command plus `-- --exact` and the 17 skips | 0 | 8,226 ms | 691 passed, 0 failed, 3 ignored, 12 filtered out |
| B3b | B2b's command plus `-- --exact` and the 17 skips | 0 | 72 ms | 9 passed, 0 failed, 5 filtered out |

- B1 failed as the job expects. The 6 errors are `core`'s lib test and the test targets `banned_lists`,
  `claim_reader`, `decision_citations`, `findings` and `graph`. No other target failed to compile.
- B2: 17 tests failed, and they are exactly the 17 in `skips.txt` (12 engine, 5 core). None is outside it.
- B3: 0 failed tests and `rc=0` in both sections. The failure count over the last B3 pair printed 0.

## ENV-EXTRA

None. `skips.txt` is the job's 17 names, unchanged, and there was no `-r2` baseline.

## Exit codes and results

- Job exits: `toolchain` 0, `install` 0, `bl` 0, `main` 0, `recheck` 0. Step 7 did not run, because this is case A.
- Main run (MEASURED): 815 mutants tested in 68 min, from 2026-11-16T15:53:48Z to 2026-11-16T17:01:48Z.
- Outcomes (EXACT): caught 815, missed 0, timeout 0, unviable 0.
- Step 8: `find-timing.sh` found 0 timing-only catches. It read all 815 `CaughtMutant` entries of `outcomes.json`, and each
  of the 815 mutant logs names at least one `FAILED` test. `run-recheck.sh` printed `no timing-only catches` and exited
  0. So there is no `recheck-jobs1/` folder, and the summary's re-check fields are 0.
- Which tests caught them, from the mutant logs (EXACT):
  - `tests34::v34_cube_day_high_vs_bhavcopy_high` failed for all 815 mutants.
  - The wall-clock test `split18::tests::v32rb_split_o1_gates` also failed in 67 of them. It always failed together with
    the test above, so none of those 67 is a timing-only catch.

## Anything odd

- While polling for READY (14:55:03Z to 15:51:41Z), the box installed cargo-mutants early with
  `cargo install cargo-mutants --version 27.1.0 --locked`. That is the same version and the same `--locked`.
  - Step 2's own install command then printed `already installed` and exited 0.
- rustup had installed toolchain 1.97.1 on this box before step 2 ran, because another checkout of the same repository
  pins it. So step 2 printed `unchanged`. rustup also ran its self-update check in that step.
- The checkout has an untracked `target/` folder from the builds. Only `results/mutants/shard-20-of-30/` is staged.

# Mutation slice 2/30 of engine v3.4: notes

## The box

- `uname -a`: Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC x86_64 GNU/Linux
- `nproc`: 4
- Memory: MemTotal 16,473,776 kB (about 15.7 GiB).
- Cgroup limit: there is no cgroup v2 `memory.max` file (the box uses cgroup v1). The v1
  `memory/memory.limit_in_bytes` reads 9223372036854771712, so no memory limit is set.
- Disk at set-up: 30 GB available.

## Toolchain, cargo-mutants, case and commands

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`. The toolchain was already installed. The
  step 2 `rustup toolchain install` reported every component up to date.
- `cargo-mutants 27.1.0`, installed with `cargo install cargo-mutants --locked --version 27.1.0`.
- Commit `7a5428d30bdb8a5e0fec427848178c82907de2fb` on `cloud/spikefade-v34int-20261116`. The `~/mut` clone's HEAD
  matched it ("HEAD OK"). JOB-MUTANTS.md sha256 `078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74`
  matched the READY note.
- Every JOB block was cut from JOB-MUTANTS.md as written and run. The only change was the first line of section 1b
  (`K=2; SHA=7a5428d30bdb8a5e0fec427848178c82907de2fb`).
- **Case A**: the slice has no `core` mutant, so there are no case arguments.
- Main command (from `run-main.sh`), with the 17 names of `skips.txt` in file order:
  `cargo mutants --workspace --shard 2/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip <name> ...`
- Re-check command (from `run-recheck.sh`), with one `-F '^<name>$'` for each of the 4 timing-only names:
  `cargo mutants --workspace -f crates/spikefade/src/acct.rs -F ... --jobs 1 --cargo-arg=--locked -o ~/mo-recheck -- -- --exact --skip <name> ...`

## List check

- `list_check=MATCH`.
  - Whole list: 24,429 mutants, sha256 `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`.
  - Slice 2: 815 mutants, sha256 `c99a91ff77cfe0cf14c7fc43e6d4ca499c66f349e34900aa0eb394e25793d2d5`.
  - Both equal the table.
- Slice by package: costs 324, greeks 345, spikefade 146. This is the expected split.

## Baseline (before cargo-mutants)

| section | command | rc | wall ms | tests |
|---|---|---|---|---|
| B1 | `cargo build --locked --workspace --tests --keep-going` | 101 | 19,337 | build only |
| B2a | `cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade` | 101 | 7,265 | 11 binaries: 691 passed, 12 failed, 3 ignored |
| B2b | `cargo test --locked --no-fail-fast -p core --test bound_probe --test cost_invariants --test lint --test zerodha_index` | 101 | 152 | 4 binaries: 9 passed, 5 failed, 0 ignored |
| B3a | B2a plus `-- --exact` and the 17 skips | 0 | 6,289 | 691 passed, 0 failed, 3 ignored |
| B3b | B2b plus `-- --exact` and the 17 skips | 0 | 62 | 9 passed, 0 failed, 0 ignored |

- **B1** has exactly the 6 expected `could not compile` errors, all for `core`: the lib test, and the test targets
  `banned_lists`, `claim_reader`, `decision_citations`, `findings` and `graph`. No other target failed to compile.
- **B2**: 17 failed. The set is identical to `skips.txt`: 12 engine tests in B2a, and the 5 `core` `cost_invariants`
  names in B2b. None is outside `skips.txt`.
- **B3**: 0 failed, and rc=0 in both sections. The step 5 count for the last B3 pair printed 0.
- **cargo-mutants' own baseline**: "Unmutated baseline in 17s build + 6s test". It auto-set the test timeout to 31 s.

## ENV-EXTRA tests

None. `skips.txt` is the JOB's 17 names, unchanged.

## Exit codes and outcomes

| job | exit |
|---|---|
| `toolchain` | 0 |
| `install` | 0 |
| `bl` | 0 (the baseline script itself; the section rcs are above) |
| `main` | **2** (some missed) |
| step 7 | not run (case A) |
| `recheck` | **2** (some missed) |

- **Main run.** It ran from 2026-11-16T15:55:18Z to 2026-11-16T16:10:32Z. cargo-mutants reported "815 mutants tested in
  15m". The counts are:
  - **caught 580** (costs 200, greeks 331, spikefade 49)
  - **missed 96** (costs 1, greeks 1, spikefade 94; spikefade's are 55 in `acct.rs` and 39 in `main.rs`)
  - **timeout 0**
  - **unviable 139** (costs 123, greeks 13, spikefade 3)
- **Wall-clock re-check (step 8).**
  - `find-timing.sh` found 4 timing-only catches, all in `crates/spikefade/src/acct.rs`:
    - `27:8 replace / with % in x_parts`
    - `27:32 replace / with % in x_parts`
    - `171:53 replace + with * in verdict_key`
    - `237:27 delete - in Tops::offer`
  - Under `--jobs 1` **all 4 were MISSED** (recheck_caught 0, recheck_missed 4).
  - The corrected counts are therefore **caught 576 and missed 100**. The official files are unchanged, as cargo-mutants
    wrote them.
- **Re-check extras.** The re-check list held 38 mutants: the 4 names plus 34 extra `delete field ... from struct ...
  expression` mutants of `acct.rs`. cargo-mutants 27.1.0 does not apply `-F` to those, as the JOB says. Of the 34, 2 were
  caught and 32 missed. They belong to the re-check only.

## Anything odd

- **rustup updated itself.** The step 2 toolchain job ran rustup's own self-update check, and rustup went from 1.28.2 to
  1.29.3 (rustup itself, not the compiler). rustc stayed 1.97.1 (8bab26f4f 2026-07-14).
- **The run was faster than the estimate.** The JOB estimated 15 to 30 min for slices 0 to 2; this one took about 15 min.
- **Progress pushes.** There was one progress commit during the main run, at 784 of 815 tested, after the third wait. The
  run ended on the fourth wait.
- **Missed and the skips.** As the JOB's limit says, a mutant that only a skipped test would catch is counted as missed.
  This box did not test which of the 96 missed (100 corrected) are of that kind.

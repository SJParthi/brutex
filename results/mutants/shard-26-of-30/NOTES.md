# Mutation slice 26/30 of engine v3.4: notes (the box writes these; see step 9)

Labels follow JOB-MUTANTS.md: EXACT = a count, MEASURED = a number from a named run on this box. No figure here is a
trading result.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4 (Intel(R) Xeon(R) Processor, 2.10 GHz).
- Memory: `MemTotal: 16473776 kB`. The cgroup is v1: `/sys/fs/cgroup/memory.max` does not exist, so step 2's `cat`
  printed nothing. `/sys/fs/cgroup/memory/memory.limit_in_bytes` is 9223372036854771712, which means no limit.
- Disk: 29 GB free at step 2.

## Versions, case and command

- **Source:** branch `cloud/spikefade-v34int-20261116` at `7a5428d30bdb8a5e0fec427848178c82907de2fb`, cloned fresh into
  `~/mut` (`HEAD OK`). This is the commit READY.txt names. JOB-MUTANTS.md at that commit has sha256
  `078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74`, the value READY.txt gives. Every bash block of the
  JOB was run as written. Only the first line of section 1b was filled in, with `K=26` and the commit.
- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`.
- `cargo-mutants 27.1.0`.
- **Case A** (EXACT): the slice holds no `core` mutant, because all 815 are in `spikefade`. `case-args.txt` is empty.
- **Command** (`run-main.sh`, from `~/mut`):

  ```sh
  cargo mutants --workspace --shard 26/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact \
    --skip <each of the 17 names in skips.txt, in its order>
  ```

- **How each mutant was tested** (the argv is in `outcomes.json`):
  - built with `cargo test --no-run --verbose --package=spikefade@0.1.0 --locked`;
  - tested with `cargo test --verbose --package=spikefade@0.1.0 --locked -- --exact --skip <the 17>`.

## list_check

- **MATCH.** `--list` gave 24,429 mutants with sha256 `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`.
  Slice 26 is 815 lines with sha256 `957e5d42576fd76c62a67f859d8a8050caefac6323994c14adb971abd073f19d`. Both pairs equal
  the JOB's table.
- **The slice** (EXACT): all 815 mutants are `replace wick_compare -> (Vec<(u32, i64)>, u64, Vec<((i64, u32), (i64,
  i64, i64, i64))>) with (...)` at `crates/spikefade/src/fix34.rs:1301:5`.

## Baseline (`baseline.log`)

| section | command | rc | wall ms |
|---|---|---:|---:|
| B1 | `cargo build --locked --workspace --tests --keep-going` | 101 | 25,911 |
| B2a | `cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade` | 101 | 7,239 |
| B2b | `cargo test --locked --no-fail-fast -p core --test bound_probe --test cost_invariants --test lint --test zerodha_index` | 101 | 158 |
| B3a | B2a's command plus `-- --exact` and the 17 skips | 0 | 6,940 |
| B3b | B2b's command plus `-- --exact` and the 17 skips | 0 | 76 |

- **B1:** exactly 6 `could not compile` errors, all for `core`: its lib test and the test targets `banned_lists`,
  `claim_reader`, `decision_citations`, `findings` and `graph`. This is as the JOB expects, and no other target failed.
- **B2:** 17 failed tests, the same set as `skips.txt`.
  - 12 are in `spikefade`'s lib test target, which reported 204 passed, 12 failed and 1 ignored (216 + 1, as on the Mac).
  - The other 5 are the `core` `cost_invariants` names.
- **B3a and B3b:** rc 0 and 0 failed. The JOB's count of failures in the last B3 pair printed 0.
- **ENV-EXTRA:** none. `skips.txt` is the JOB's 17, unchanged, and `baseline.sh -r2` was not needed.

## The run (EXACT, from `outcomes.json`)

- **cargo-mutants' own baseline:** build 19.2 s and test 4.7 s. It reported 204 passed, 0 failed, 1 ignored and 12
  filtered out, then set the test timeout to 24 s itself.
- **Wall (MEASURED):** `main.start` is 2026-11-16T15:54:31Z and `main.end` is 17:03:40Z, so 4,149 s.
  - `outcomes.json` gives a start of 15:54:34.7Z and an end of 17:03:40.5Z.
  - cargo-mutants' last line: `815 mutants tested in 69m: 815 caught`.
- **Result:** caught 815, missed 0, timeout 0, unviable 0.
- **Exit codes:** `toolchain` 0, `install` 0, `bl` 0, `main` 0 (all caught) and `recheck` 0. There was no `core` job
  because this is case A: step 7's guard did nothing.
- **Which test caught them:** `tests34::v34_cube_day_high_vs_bhavcopy_high` failed in all 815 mutant logs. It is the
  only test outside `timing-tests.txt` that failed in any of them.

## Wall-clock re-check (step 8)

- `find-timing.sh` found **0 timing-only catches**.
- The wall-clock test `split18::tests::v32rb_split_o1_gates` did fail, under `--jobs 2`, in 68 of the 815 mutant logs.
  Each of those times it failed together with `tests34::v34_cube_day_high_vs_bhavcopy_high`, so none of those catches is
  timing-only.
  - This was checked separately, by reading every caught mutant's log with `jq` and `grep`.
  - All 815 logs hold at least one `... FAILED` test line.
- `run-recheck.sh` printed `no timing-only catches` and exited 0. Nothing was re-run, so there is no `recheck-jobs1/`
  folder.
- `recheck_caught`, `recheck_missed` and `recheck_extra` are 0, so the corrected counts equal the official ones.

## Anything odd

- **Installed before READY.** While waiting for the READY branch, two things were installed ahead of the JOB, from
  outside the checkout:
  - the pinned toolchain, with `rustup toolchain install 1.97.1 --profile minimal -c rustfmt -c clippy -c
    llvm-tools-preview -t wasm32-unknown-unknown`;
  - cargo-mutants, with `cargo install cargo-mutants --locked`. At 15:26Z crates.io's latest was 27.1.0, so it resolved
    to 27.1.0.

  Step 2 then reported the toolchain `unchanged` and printed `Ignored package cargo-mutants v27.1.0 is already
  installed`. Both jobs exited 0, and the versions were checked as step 2 requires.
- **rustup updated itself** from 1.28.2 to 1.29.3 during that toolchain install, as it does by default. `rustc` and
  `cargo` are the pinned 1.97.1.
- **Five progress commits** were pushed during the run, at 150, 313, 467, 621 and 777 tested. `PROGRESS.txt` keeps the
  last of them.
- **Tools used.** Only the 1.97.1 toolchain and cargo-mutants were installed. `node` (v22) and `jq` were already on the
  box. `node` was used only by the JOB's `find-timing.sh`, and `jq` only to read `outcomes.json` for these notes.
- **No market data** was fetched, read or written, and no code was changed.

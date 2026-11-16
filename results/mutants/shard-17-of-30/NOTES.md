# Mutation slice 17/30 of engine v3.4: notes

Box K=17 of 30. Code: branch `cloud/spikefade-v34int-20261116`, commit `7a5428d30bdb8a5e0fec427848178c82907de2fb`.
Job file: `JOB-MUTANTS.md` at that commit (sha256 `078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74`,
checked before use). Labels as in the job file: EXACT = a count, MEASURED = a number from a named run. Nothing here is a
trading result. No market data was fetched, read or written.

## The box (MEASURED, step 2)

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- `MemTotal`: 16,473,776 kB
- Memory cgroup limit: `/sys/fs/cgroup/memory.max` does not exist on this box (cgroup v1), so step 2's `cat` printed
  nothing. The shell's v1 memory cgroup has `memory.limit_in_bytes` = 14,338,772,992 bytes (13.35 GiB), and its
  `memory.max_usage_in_bytes` reached that limit during the run (see "Anything odd", item 1).
- Disk at the start: 252G device, 30G available.

## Versions, case and the exact commands

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`. The
  step-2 `rustup toolchain install` reported the toolchain unchanged (exit 0).
- `cargo-mutants 27.1.0`, from `cargo install cargo-mutants --locked --version 27.1.0` (exit 0).
- **Case A**: no `core` mutant in the slice, so `case-args.txt` is empty.
- Main run (`run-main.sh` as written, K=17):
  `cargo mutants --workspace --shard 17/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip <each of the 17 names in skips.txt>`
- Re-check (`run-recheck.sh` as written):
  `cargo mutants --workspace -f crates/spikefade/src/split18.rs -F '^<name>$' (one -F for each of the 8 names) --jobs 1 --cargo-arg=--locked -o ~/mo-recheck -- -- --exact --skip <the same 17 names>`
- Step 7 (the core follow-up) was not run: it is for case C only.

## The list (EXACT, step 3)

- `list_check=MATCH`. The whole list holds 24,429 mutants (sha256 `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`).
  Slice 17 holds 815 (sha256 `c63f3ed5f2191b3b772bc3e6c4c6125a274c55ed4703944753bd100e021782cd`). Both match the table.
- The slice has 808 mutants in `crates/spikefade/src/split18.rs` and 7 in `crates/spikefade/src/sw33.rs`.

## Baseline (MEASURED, `baseline.log`)

| section | command | rc | wall | result |
|---|---|---|---|---|
| B1 | `cargo build --locked --workspace --tests --keep-going` | 101 | 23.0 s | exactly 6 `could not compile` errors, all in `core`: lib test, `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph` |
| B2a | `cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade` | 101 | 9.9 s | 691 passed, 12 failed, 3 ignored |
| B2b | `cargo test --locked --no-fail-fast -p core` with its 4 compilable targets | 101 | 0.2 s | 9 passed, 5 failed |
| B3a | B2a with the 17 skips | 0 | 9.8 s | 691 passed, 0 failed, 3 ignored |
| B3b | B2b with the 17 skips | 0 | 0.1 s | 9 passed, 0 failed |

- The 17 B2 failures are exactly the 17 names in `skips.txt`: 12 engine and 5 core. In B3a the spikefade unit-test
  binary ran 204 passed, 1 ignored and 12 filtered out, which is 217 tests.
- The failure count for the last B3 pair printed 0.
- **ENV-EXTRA: none.** `skips.txt` is the job file's 17 names, unchanged.

## Exit codes

- `toolchain` 0, `install` 0, `bl` 0.
- `main` 3, which means some mutants timed out and the job allows it. The run went from 2026-11-16T15:55:28Z to
  16:47:12Z. cargo-mutants' own last line: `815 mutants tested in 52m: 214 missed, 300 caught, 298 unviable, 3 timeouts`.
  The unmutated baseline took 24 s to build and 7 s to test, and the test timeout auto-set to 36 s.
- `recheck` 2, which means some mutants were missed and the job allows it.

## The wall-clock re-check (step 8)

- `find-timing.sh` found **8 timing-only catches**. In all 8 the only FAILED test was
  `split18::tests::v32rb_split_o1_gates`.
- Rerun with `--jobs 1`, **all 8 were MISSED**: `recheck_caught=0` and `recheck_missed=8`.
  - corrected caught = 300 - 8 = **292**
  - corrected missed = 214 + 8 = **222**
  - The official files stay as cargo-mutants wrote them.
- `recheck-list.txt` also holds 5 `delete field ... from struct Self expression` mutants of `split18.rs`. One is in
  `SCal::from_days`, at 91:28. The other four are in `Sel18::merge`, at 1111, for the fields dst, items, pre and suf.
  cargo-mutants 27.1.0 does not apply `-F` to these, as the job file says. 1 was caught (91:28) and 4 were missed. They
  belong to the re-check only (`recheck_extra=5`).
- The re-check's own baseline took 17 s to build and 5 s to test, and the test timeout auto-set to 27 s.

## Anything odd

1. **Two OOM kills (MEASURED, dmesg and the mutant logs).** Twice during the main run, the kernel's memory-cgroup OOM
   killer killed a mutant's spikefade test binary. The anon RSS was 13,705,748 kB and 13,864,548 kB, about 13.1 and
   13.2 GiB. These are the two mutants:
   - `crates/spikefade/src/split18.rs:994:27: replace >= with < in splits`
   - `crates/spikefade/src/split18.rs:999:23: replace += with *= in splits`

   cargo-mutants logged `signal: 9, SIGKILL: kill` and recorded both as `CaughtMutant`, so both are in `caught.txt`
   unchanged. **The kill caught them, not a test assertion.** dmesg shows exactly these 2 kills, both of the runaway test
   binary. No cargo-mutants, cargo or rustc process was killed.
2. **The 3 timeouts** are all in `Dst::build`:
   - `split18.rs:887:15` (`+=` to `*=`)
   - `893:30` (`<<` to `>>`)
   - `918:19` (`+=` to `*=`)
3. **Unviable:** 288 of the 298 are return-value replacements of `Sel18::batch_parts`, whose return type is a 6-part
   tuple. That function has 360 mutants in the slice: 288 unviable, 72 missed and 0 caught.
4. **sw33.rs:** 5 of its 7 mutants were caught and 2 were missed.
5. **rustup** updated itself from 1.28.2 to 1.29.3 during the step-2 `rustup toolchain install`. This was rustup's own
   self-update check, shown in the last two lines of `toolchain.log`. `rustc` and `cargo` did not change.

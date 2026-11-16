# Mutation slice 14/30 of engine v3.4: notes (the box writes these; see step 9)

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- Memory: MemTotal 16473776 kB (about 15.7 GiB). `/sys/fs/cgroup/memory.max` does not exist on this box, so no cgroup
  limit could be read.
- Disk at set-up: 252G, 30G available.

## Versions, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`.
- `cargo-mutants 27.1.0`, installed with `cargo install cargo-mutants --locked --version 27.1.0`.
- Case **A**: the slice is 815 `spikefade` mutants and no `core` mutant (ban.rs 33, bhav.rs 161, expiry.rs 322,
  freeze.rs 168, gk.rs 64, wf.rs 67). `case-args.txt` is empty, so no argument was added.
- The exact command (`run-main.sh`, in the clone at the commit, on the result branch made from it):
  `cargo mutants --workspace --shard 14/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact` followed by
  `--skip <name>` for each of the 17 names in `skips.txt`.

## List check

`list_check=MATCH`. The whole list is 24,429 mutants with sha256 `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`,
and slice 14 is 815 mutants with sha256 `57fc76c93ea6342dee8d0ee7f938cdeaf636b33847526119f938ff089060789d`. Both equal the
job's table.

## Baseline (from `baseline.log`)

| section | command | rc | wall ms |
|---|---|---|---|
| B1 | `cargo build --locked --workspace --tests --keep-going` | 101 | 26590 |
| B2a | `cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade` | 101 | 9573 |
| B2b | `cargo test --locked --no-fail-fast -p core` with core's 4 compilable targets | 101 | 193 |
| B3a | B2a with `-- --exact` and the 17 skips | 0 | 10103 |
| B3b | B2b with `-- --exact` and the 17 skips | 0 | 90 |

- B1: exactly 6 `could not compile` errors, all for `core`: its lib test and the test targets `banned_lists`,
  `claim_reader`, `decision_citations`, `findings` and `graph`. Each is an `include_str!` of an app-repository file that
  this branch does not hold. No other target failed. This is what the job expects.
- B2 (no skips): 17 failed tests, and they are exactly the 17 names of `skips.txt`. None is outside it.
- B3 (with the skips): 0 failed tests and rc 0 in both sections. The `spikefade` unit tests gave 204 passed, 1 ignored and
  12 filtered out by the skips.

## ENV-EXTRA

None. `skips.txt` is the job's 17 names, unchanged (sha256
`0cd96e3e2906f52103be4c894a44beec8c32170a5a8828ea7e2ea302c41fe557`).

## Exit codes

- `toolchain` 0: 1.97.1 was already on the box and rustup reported it `unchanged`. `install` 0. The baseline job 0.
- Main run: **exit 2** (some missed). The cargo-mutants baseline was ok (22 s build + 6 s test) and it auto-set the test
  timeout to 31 s. It tested 815 mutants in 81 min (run wall 4887 s, about 6.0 s a mutant): caught 361, missed 444,
  timeout 0, unviable 10.
- Step 7 does not apply (case A), so there is no `core-followup/`.
- Re-check (`--jobs 1`): **exit 2**. Its baseline was ok (17 s build + 5 s test), with an auto-set test timeout of 28 s. It
  tested 88 mutants in 12 min: 87 missed, 1 caught.

## Anything odd

- **79 of the 361 catches (22%) were timing-only.** Every one of them named only
  `split18::tests::v32rb_split_o1_gates` as FAILED. None of the other three wall-clock tests appeared.
  - Re-tested with `--jobs 1`, all 79 passed their tests, so they are missed: `recheck_caught=0`, `recheck_missed=79`.
  - Corrected caught = 361 - 79 = **282**. Corrected missed = 444 + 79 = **523**.
  - The official `caught.txt` and `missed.txt` are left as cargo-mutants wrote them.
- **The re-check list held 88 mutants:** the 79 names plus 9 extra `delete field ... from struct ... expression`
  mutants of the same files, because cargo-mutants 27.1.0 does not apply `-F` to them (`recheck_extra=9`).
  - 5 of the extras are in this slice, and each got the same outcome as in the main run: 4 missed, and 1 caught
    (`freeze.rs:88:72: delete field first from struct Self expression in StockDated::parse`).
  - 4 are not in this slice (`wf.rs:69:23`, and `gk.rs:118:21`, `:27` and `:34`), and all 4 were missed.
  - They belong to the re-check only.
- Timing: the READY branch appeared at 15:50:57Z, the box started at 15:52:24Z, and the main run ran from 15:55:33Z to
  17:17:00Z.

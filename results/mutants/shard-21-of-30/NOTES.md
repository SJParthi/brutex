# Mutation slice 21/30 of engine v3.4: notes (the box writes these; see step 9)

Box 21 of 30, wave 2, 16 Nov 2026. Code commit `7a5428d30bdb8a5e0fec427848178c82907de2fb`, the tip of
`cloud/spikefade-v34int-20261116`. The READY marker `cloud/ready-v34-mutants-20261116` appeared at 2026-11-16T15:51:41Z.
`JOB-MUTANTS.md` at that commit had sha256 `078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74`, the value
READY.txt names, and was checked before use. Every block ran as written; only the first line of section 1b was filled in.
Data-free: no market data was fetched, read or written, and no code was changed.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- MemTotal: 16,473,776 kB.
- cgroup: version 1. The job's `/sys/fs/cgroup/memory.max` is a version 2 file, so it does not exist here and step 2
  printed nothing for it. The version 1 memory cgroup the jobs ran in has `memory.limit_in_bytes` 14,338,772,992
  (13.35 GiB). Its `memory.max_usage_in_bytes` read 3,937,943,552 (3.67 GiB) after the run. That is MEASURED, but it is
  a peak over the cgroup's whole life, so it covers the build, the baseline and the mutants run together. The root
  memory cgroup has no limit.
- Disk at set-up: 252 GB, with 30 GB available.

## Toolchain and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`.
  - The toolchain was already on the box before step 2. rustup had installed it from the same pin when the session's
    own checkout was first used.
  - So step 2's `rustup toolchain install` reported it `unchanged`, and exited 0.
- `cargo-mutants 27.1.0`, from `cargo install cargo-mutants --locked --version 27.1.0`. It exited 0 after a 1 m 17 s
  release build.
- Case **A**: the slice holds no `core` mutant, so `case-args.txt` is empty.
- The exact command (`run-main.sh`), from `~/mut`:
  `cargo mutants --workspace --shard 21/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact`, then
  `--skip <name>` for each of the 17 names in `skips.txt`, in that file's order.

## List check

- `list_check MATCH`. The full list is 24,429 mutants, sha256
  `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`.
- Slice 21 holds 815, sha256 `ff98a493832f3da4df14058d0a04d12ae903076180357c3e0cc4bc11d2d85062`. Both equal the table.
- All 815 are in `spikefade`, and all sit at `crates/spikefade/src/fix34.rs:1301:5`. Every one replaces the return value
  of `wick_compare`, as section 12 expects for slices 19 to 28.

## Baseline (step 5)

| section | rc | wall ms | result |
|---|---|---|---|
| B1 | 101 | 21,498 | exactly the 6 expected `could not compile` errors, all `core`: its lib test and `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph` |
| B2a | 101 | 9,246 | 12 failed, all in `skips.txt`; the spikefade lib target ran 204 passed, 12 failed, 1 ignored |
| B2b | 101 | 168 | 5 failed (`cost_invariants`), all in `skips.txt` |
| B3a | 0 | 9,848 | 0 failed; the spikefade lib target ran 204 passed, 1 ignored, 12 filtered out |
| B3b | 0 | 77 | 0 failed; `cost_invariants` ran with its 5 filtered out |

- B2 failed 17 tests, exactly the 17 skips.
- **No `ENV-EXTRA` test.** The count of failures in the last B3 pair printed 0.
- cargo-mutants' own baseline printed `ok Unmutated baseline in 22s build + 6s test`, then
  `Auto-set test timeout to 31s`.

## Exit codes

- toolchain 0, install 0, baseline job 0, main 0, recheck 0.
- Step 7 is for case C only. Its block ran as written and did nothing.
- main printed `815 mutants tested in 83m: 815 caught`. It started at 2026-11-16T15:56:37Z and ended at
  2026-11-16T17:19:44Z. The counts are caught 815, missed 0, timeout 0, unviable 0.
- Progress commits went out about every 13 minutes, at 123, 249, 381, 501, 632 and 771 tested.

## Step 8 and anything odd

- `find-timing.sh` found 0 timing-only catches. So `run-recheck.sh` printed `no timing-only catches` and exited 0, and
  there is no `recheck-jobs1/` folder.
- Before trusting that 0, the box checked by hand that `outcomes.json` holds the fields the script reads: 815
  `CaughtMutant` and 1 `Success`, each with `scenario.Mutant.name` and a distinct `log_path`.
- Checked by hand from the 815 mutant logs (MEASURED, a `grep` for `test ... FAILED` lines):
  - all 815 name `tests34::v34_cube_day_high_vs_bhavcopy_high` as FAILED, so every catch rests on that non-timing test;
  - the wall-clock gate `split18::tests::v32rb_split_o1_gates` is also FAILED in 101 of the 815 logs, under `--jobs 2`
    load. Each of those 101 also has the non-timing failure above, so no catch depends on the gate, and the timing-only
    rule rightly selects none. It is the same gate the last wave's slice 26 found failing on load alone.
- Limit (section 4): a mutant that only a skipped test would catch counts as missed. There are no missed mutants here.

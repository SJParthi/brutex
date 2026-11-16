# Mutation slice 23/30 of engine v3.4: notes

Box 23 of 30, data-free and code-only. Code commit 7a5428d30bdb8a5e0fec427848178c82907de2fb, the tip of branch
cloud/spikefade-v34int-20261116, as named by READY.txt on cloud/ready-v34-mutants-20261116 (READY commit 503c6660).
JOB-MUTANTS.md at that commit has sha256 078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74, the value
READY.txt states. Every block was taken from JOB-MUTANTS.md by its line range and run with bash unchanged, except for K and
the commit in the first line of section 1b.

## The box

- uname: Linux 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC x86_64 GNU/Linux
- nproc: 4
- memory: MemTotal 16473776 kB (about 15.7 GiB). The cgroup is v1: /sys/fs/cgroup/memory.max does not exist, so section
  2's command printed nothing for it. /sys/fs/cgroup/memory/memory.limit_in_bytes reads 9223372036854771712, which means
  no limit.
- disk: 30 GB free on / at the start.

## Versions, case and command

- rustc 1.97.1 (8bab26f4f 2026-07-14) and cargo 1.97.1 (c980f4866 2026-06-30), as required. The toolchain step found
  1.97.1 already installed with every listed component, and reported it unchanged (exit 0).
- cargo-mutants 27.1.0, installed with `cargo install cargo-mutants --locked --version 27.1.0` (exit 0).
- Case A. The slice holds no `core` mutant, so `case-args.txt` is empty.
- The exact command (run-main.sh, run in ~/mut):
  `cargo mutants --workspace --shard 23/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact`, followed by
  `--skip <name>` for each of the 17 names in skips.txt, in that file's order.

## list_check

MATCH. The full list is 24,429 mutants with sha256 3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a.
Slice 23 is 815 mutants with sha256 01e6f1b235292fa13336076d20aff78979a14c0b77779ce0f6aebc3c05cdeeb7. Both equal the
table. All 815 are at crates/spikefade/src/fix34.rs:1301:5, and each replaces the return value of `wick_compare` (the
3-part tuple of section 12).

## Baseline (section 5)

| section | command | rc | wall ms |
|---|---|---|---|
| B1 | cargo build --locked --workspace --tests --keep-going | 101 | 20,653 |
| B2a | cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade | 101 | 7,081 |
| B2b | cargo test --locked --no-fail-fast -p core, targets bound_probe, cost_invariants, lint, zerodha_index | 101 | 148 |
| B3a | B2a's command with `-- --exact` and the 17 skips | 0 | 6,920 |
| B3b | B2b's command with `-- --exact` and the 17 skips | 0 | 77 |

- B1: exactly the 6 expected `could not compile` errors, all for `core`: its lib test and the test targets banned_lists,
  claim_reader, decision_citations, findings and graph. Each fails on a file this branch does not hold ("couldn't read
  ...: No such file or directory"). No other target failed to compile.
- B2: 17 failed tests, exactly the 17 names in skips.txt (12 engine, 5 core). None is outside skips.txt. The two
  `tests34s` failures show the known cause: zstd at the Mac path /opt/homebrew/bin/zstd, "No such file or directory"
  (kit.rs:470; the fix is not in v3.4).
- B3a: spikefade 204 passed, 1 ignored, 12 filtered out (217 tests); costs and greeks passed in full. B3b: core's 4
  targets passed, with cost_invariants' 5 tests filtered out. The failure count in the last B3 pair is 0.

## ENV-EXTRA

None. No test outside skips.txt failed, so baseline.sh was not run again with a tag.

## Exit codes and the run

- main: exit 0. 815 tested in 65 minutes (start 2026-11-16T15:55:06Z, end 2026-11-16T17:00:05Z: 3,899 s, about 4.8 s a
  mutant). Caught 815, missed 0, timeout 0, unviable 0. cargo-mutants' own baseline was ok (18 s build + 4 s test), and
  it auto-set the test timeout to 24 s.
- Step 7 (core follow-up): not run, because the case is A.
- Step 8 (wall-clock re-check): find-timing.sh found 0 timing-only catches. run-recheck.sh printed "no timing-only
  catches" and exited 0. It wrote no re-check output, so there is no recheck-jobs1 folder.
- MEASURED from this run's 815 test logs: every catch names `tests34::v34_cube_day_high_vs_bhavcopy_high` as FAILED. In 54
  of them the wall-clock test `split18::tests::v32rb_split_o1_gates` failed as well, under `--jobs 2` load, but never
  alone, so none of them is a timing-only catch.

## Anything odd

- The READY branch appeared after 55 minutes of polling, at about 15:51 UTC. This box started its job at 15:52 UTC.
- The toolchain step also ran rustup's self-update check and downloaded a self-update (its log ends there, exit 0). The
  1.97.1 toolchain itself was unchanged.
- Section 3's block ran inside a 520 s `timeout` wrapper, with its console output piped to `tail` for display, to stay
  inside the 9-minute command limit. The block itself was unchanged and finished well within that limit.
- Progress commits: 5 before the final one, about 13 minutes apart.

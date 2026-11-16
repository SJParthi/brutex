# Mutation slice 12/30 of engine v3.4: notes (the box writes these; see step 9)

Box 12 of 30. Code commit `7a5428d30bdb8a5e0fec427848178c82907de2fb` on `cloud/spikefade-v34int-20261116`, as READY.txt
named it. The JOB file's sha256 matched READY.txt (`078206fd...a7b74`) before anything ran. Every block of the JOB was run
as written; the only edit was K and the commit in the first line of section 1b. Nothing here is a trading result.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- `MemTotal`: 16,473,776 kB
- cgroup limit: the box runs cgroup v1, so `/sys/fs/cgroup/memory.max` (v2) does not exist and step 2's `cat` printed
  nothing. The shell's v1 memory cgroup has `memory.limit_in_bytes` = 14,338,772,992 bytes (13.35 GiB); its parent
  groups show the no-limit value.
- disk at set-up (`df -P -BG ~`): 252G, 10G used, 30G available.

## Versions, case and exact command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, from the repository's `rust-toolchain.toml`.
- `cargo-mutants 27.1.0`, from `cargo install cargo-mutants --locked --version 27.1.0` (56 s release build).
- Odd but harmless: toolchain 1.97.1 was already on the box before step 2 (rustup installed it on the first `rustc` call
  of the session, from another checkout pinning the same version), so step 2's `rustup toolchain install` reported it
  `unchanged`. That same command then self-updated rustup itself from 1.28.2 to 1.29.3, which is rustup's default. The
  rustc and cargo-mutants versions are exactly the JOB's.
- Case **A**: no `core` mutant in the slice, so no case arguments.
- Exact command (`~/run-main.sh` with K=12 and an empty `case-args.txt`):
  `cargo mutants --workspace --shard 12/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact`, then
  `--skip <name>` for each of the 17 names in `skips.txt`, in that file's order.

## list_check

**MATCH.** Whole list 24,429 mutants, sha256 `3a041951...c15432a`; slice 12 is 815 mutants, sha256 `700692f8...c6134b6f`.
Both equal the table. All 815 are in `crates/spikefade/src/report3.rs`, lines 3639 to 4884.

Slice 12 by function (EXACT, from `list-shard.txt` and `missed.txt`):

| function | mutants | missed | caught | timeout | unviable |
|---|---|---|---|---|---|
| `speed_probe` (9-part tuple return) | 349 | 340 | 9 | 0 | 0 |
| `row32_of` | 128 | 121 | 7 | 0 | 0 |
| `write3` | 127 | 120 | 2 | 0 | 5 |
| `hand_sample` | 91 | 89 | 2 | 0 | 0 |
| `trade_row` | 79 | 74 | 4 | 0 | 1 |
| `dedup_ratio` | 13 | 11 | 2 | 0 | 0 |
| `dd_grouped` | 10 | 0 | 8 | 2 | 0 |
| `sum_eq` | 5 | 5 | 0 | 0 | 0 |
| `Replay::sweep` | 4 | 4 | 0 | 0 | 0 |
| `ma_tf` | 3 | 3 | 0 | 0 | 0 |
| `group_name` | 3 | 3 | 0 | 0 | 0 |
| `now` | 2 | 2 | 0 | 0 | 0 |
| `set_tfdep` | 1 | 1 | 0 | 0 | 0 |
| total | 815 | 773 | 34 | 2 | 6 |

## Baseline (`baseline.log`)

| section | command | rc | wall |
|---|---|---|---|
| B1 | `cargo build --locked --workspace --tests --keep-going` | 101 | 19,549 ms |
| B2a | `cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade` | 101 | 7,312 ms |
| B2b | `cargo test --locked --no-fail-fast -p core --test bound_probe --test cost_invariants --test lint --test zerodha_index` | 101 | 131 ms |
| B3a | B2a's command with `-- --exact` and the 17 skips | 0 | 5,983 ms |
| B3b | B2b's command with `-- --exact` and the 17 skips | 0 | 81 ms |

- B1: exactly the 6 expected `could not compile` errors, all `core`: its lib test, and `findings`, `decision_citations`,
  `graph`, `banned_lists`, `claim_reader`. No other target failed to compile.
- B2: 17 failed tests, every one in `skips.txt`. That is 12 in spikefade's unit tests (204 passed, 12 failed, 1 ignored:
  217 tests) and the 5 in `core`'s `cost_invariants`. `costs` and `greeks` pass in full.
- B3: 0 failed and rc 0 in both sections (spikefade 204 passed, 12 filtered out, 1 ignored). The step-5 check printed 0.

## ENV-EXTRA

None. Every baseline failure was already in `skips.txt`, so `bl2` was not run.

## Exit codes and the run

- **main: exit 3** (some timed out), which step 6 allows. cargo-mutants' own baseline passed in 17 s build + 4 s test and
  auto-set the test timeout to 22 s. 815 mutants tested in 54 min (15:55:14Z to 16:49:41Z): 34 caught, 773 missed,
  2 timeout, 6 unviable. The JOB's ESTIMATE for a slice like this one was 50 to 100 min.
- The 2 timeouts, both in `dd_grouped`: `3992:19 replace != with ==` and `3996:15 replace += with *=`.
- The 6 unviable are all `replace && with ||` and did not build: `3722:13` in `trade_row`, and `4298:17`, `4310:16`,
  `4311:13`, `4371:17`, `4385:17` in `write3`.
- Step 7 is case C only; its guarded block was run as written and did nothing.
- **Re-check (step 8): exit 2**, which step 8 allows.
  - `find-timing.sh` found **26 timing-only catches**. For all 26, the only FAILED test in the mutant's log was
    `split18::tests::v32rb_split_o1_gates`, the same wall-clock test as the last wave's slice 26 finding.
  - `recheck-list.txt` held 97 mutants: the 26 names (none absent) plus 71 extra `delete field ... from struct ...
    expression` mutants of `report3.rs`, as step 8 foresaw. 15 of the extras are inside slice 12 and 56 are not.
  - With `--jobs 1` (baseline 14 s build + 3 s test, auto timeout 20 s), 97 tested in 10 min: 1 caught, 96 missed.
  - **All 26 timing-only catches were MISSED under `--jobs 1`**: recheck_caught = 0, recheck_missed = 26. The one
    re-check catch is an extra outside slice 12 (`3420:16 delete field x from struct Sum expression in twin10`) and
    belongs to the re-check only.
- **Corrected for the re-check:** caught 34 - 26 = **8**, missed 773 + 26 = **799**; timeout 2 and unviable 6 are
  unchanged. The official files stay as cargo-mutants wrote them.
- So, corrected, the only real catches in this slice are the 8 in `dd_grouped`. Every other function in the slice has no
  mutant caught by a test that ran here.
- Limit (JOB step 4): a mutant that only a skipped test would catch is counted as missed.

## Progress commits

`e8415ac` (179 tested), `c34a1cd` (378), `5d0c989` (579), `14be685` (787), `f80168e` (815, during the re-check), then
the final commit.

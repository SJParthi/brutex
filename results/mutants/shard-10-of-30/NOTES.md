# Mutation slice 10/30 of engine v3.4: notes (the box writes these; see step 9)

## The box

- `uname -a`: Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux
- `nproc`: 4
- Memory: MemTotal 16473776 kB. The box uses cgroup v1, so `/sys/fs/cgroup/memory.max` does not exist. The shell's
  memory cgroup has `memory.limit_in_bytes` 14338772992 (about 13.4 GiB); its parent cgroups are unlimited.
- Disk at the start: 252G, 30G available.

## Tools, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`: the pinned toolchain.
- `cargo-mutants 27.1.0`, installed with `cargo install cargo-mutants --locked --version 27.1.0`.
- Case **A**: the slice is 815 `spikefade` mutants (`p3.rs` 86, `read.rs` 275, `report3.rs` 454) and no `core`
  mutant, so there are no case arguments and step 7 did not apply.
- The exact command, run in `~/mut` at commit 7a5428d30bdb8a5e0fec427848178c82907de2fb:
  `cargo mutants --workspace --shard 10/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact`, followed by
  `--skip NAME` for each of the 17 names in `skips.txt` (sha256
  0cd96e3e2906f52103be4c894a44beec8c32170a5a8828ea7e2ea302c41fe557, the job's 17 names, unchanged).

## list_check

**MATCH.** The whole list is 24429 mutants, sha256 3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a.
Slice 10 is 815 mutants, sha256 d42b5c5e97e00a28c3a05aefd911223c1ba400bfbd5d297fbb69aab679810adc. Both equal the
table.

## Baseline (`baseline.log`)

| section | rc | wall ms | result |
|---|---|---|---|
| B1 `cargo build --workspace --tests --keep-going` | 101 | 19759 | exactly the 6 expected `core` compile failures: lib test, `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph`. Nothing else failed to compile. |
| B2a test `costs`, `greeks`, `spikefade`, no skips | 101 | 8422 | `spikefade` unit tests 204 passed, 12 failed, 1 ignored; every `costs` and `greeks` target passed |
| B2b test `core`'s 4 compilable targets, no skips | 101 | 176 | `cost_invariants` 0 passed, 5 failed; `bound_probe` 2, `lint` 2, `zerodha_index` 5 passed |
| B3a as B2a, with the 17 skips | 0 | 8363 | `spikefade` 204 passed, 0 failed, 1 ignored, 12 filtered out |
| B3b as B2b, with the 17 skips | 0 | 88 | `cost_invariants` 5 filtered out; nothing failed |

- B2 failed 17 distinct tests (12 engine, 5 `core`), and all 17 are in `skips.txt`. Failures in the last B3 pair: 0.
- **No ENV-EXTRA test.** `skips.txt` was not changed and `baseline.sh -r2` was not needed.

## Main run (step 6)

- 2026-11-16T15:55:44Z to 2026-11-16T17:14:51Z, 79 min. cargo-mutants' own baseline passed (17 s build + 5 s test)
  and auto-set the test timeout to 30 s.
- **Exit 2** (some missed). 815 tested: caught 390, missed 408, timeout 0, unviable 17.
- 7 progress commits, about 13 min apart; the scan was clean on every push.

## Wall-clock re-check (step 8)

- `find-timing.sh`: **68 of the 390 catches** failed only wall-clock tests. In all 68, the one failed test was
  `split18::tests::v32rb_split_o1_gates`.
- `run-recheck.sh` with `--jobs 1`: the list held 132 mutants, the 68 names plus 64 extra
  `delete field ... from struct ... expression` mutants of `report3.rs`, which cargo-mutants 27.1.0 does not filter with
  `-F`. Exit 2; 132 tested in 19 min: 131 missed, 1 caught. Its own baseline passed and set the timeout to 29 s.
- **All 68 timing-only catches were MISSED under `--jobs 1`:** recheck_caught 0, recheck_missed 68, recheck_extra 64.
- The 1 catch is one of the extras and lies outside this slice: `crates/spikefade/src/report3.rs:3420:16: delete field
  x from struct Sum expression in twin10`, caught by `tests32::v32_wanted_picks_x_and_twins`. It belongs to the
  re-check only.
- **Corrected for this slice:** caught 390 - 68 = 322, missed 408 + 68 = 476 (timeout 0, unviable 17). The official
  files stay as cargo-mutants wrote them.

## Anything odd

- The timing-only share is high: 68 of 390 catches (17%), all from `split18::tests::v32rb_split_o1_gates` failing under
  `--jobs 2` load. The last wave's slice 26 had 11 of 83. Without the re-check, this slice's catch count would be
  overstated by 68.
- `split18::tests::v32rb_split_o1_gates` failed in 116 of the 390 caught mutants' logs; in the other 48 of them another
  test failed too, so they stay caught.
- Nothing else: no timeout, no hang, no build failure outside the expected `core` targets. The baseline build's
  untracked `target/` directory in the checkout was never staged.
- Limit, from the job: a mutant that only a skipped test would catch is counted as missed.

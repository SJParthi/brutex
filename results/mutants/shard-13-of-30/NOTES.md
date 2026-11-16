# Mutation slice 13/30 of engine v3.4: notes (the box writes these; see step 9)

Every block of JOB-MUTANTS.md ran as written, in order. The blocks were taken straight out of the file, after its
sha256 matched the one READY.txt gives (078206fd...7b74); the only edit was K=13 and the commit in the first line
of section 1b. No STOP.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- memory: MemTotal 16,473,776 kB (about 15.7 GiB). cgroup limit: none readable here, because
  `/sys/fs/cgroup/memory.max` does not exist on this box.
- disk: 30 GB free at the start.
- The READY branch appeared at 15:51:20Z, after 57 minutes of polling. Box start (step 1b): 2026-11-16T15:52:55Z.

## Toolchain, case and command

- rustc 1.97.1 (8bab26f4f 2026-07-14) and cargo 1.97.1 (c980f4866 2026-06-30), as `rust-toolchain.toml` pins.
- cargo-mutants 27.1.0, installed with `cargo install cargo-mutants --locked --version 27.1.0`.
- Code: `cloud/spikefade-v34int-20261116` at 7a5428d30bdb8a5e0fec427848178c82907de2fb. `HEAD OK`.
- Case: **A**. The slice holds no `core` mutant: all 815 are in `spikefade` (report3.rs 430, stage.rs 358, wf.rs 27).
- The exact command (`run-main.sh`, in `~/mut`):
  `cargo mutants --workspace --shard 13/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip <name>`
  with one `--skip` for each of the 17 names in `skips.txt`, in that order. No case arguments.
- cargo-mutants' own baseline: 18 s build + 4 s test, so it auto-set the test timeout to 23 s. No build timeout.

## list_check

**MATCH.** The whole list is 24,429 mutants, sha256 3a0419...432a, and slice 13 is 815 mutants, sha256
b182db...12f0. Both equal the table.

## Baseline (`baseline.log`)

| section | rc | wall | what it showed |
|---|---|---|---|
| B1 build, workspace tests, keep-going | 101 | 20.2 s | exactly 6 `could not compile`, all `core`: lib test, `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph`. As expected. |
| B2a tests of costs, greeks, spikefade, no skips | 101 | 7.1 s | 691 passed, 12 failed, 3 ignored |
| B2b core's 4 compilable targets, no skips | 101 | 0.2 s | 9 passed, 5 failed |
| B3a as B2a, with the 17 skips | 0 | 7.2 s | 691 passed, 0 failed, 3 ignored |
| B3b as B2b, with the 17 skips | 0 | 0.06 s | 9 passed, 0 failed |

The 17 B2 failures are exactly the 17 names in `skips.txt` (12 engine, 5 core). Nothing else failed.

**No ENV-EXTRA test.**

## Exit codes and results

- **Main run: exit 2** (some missed). 815 tested in 52 min (15:55:43Z to 16:48:07Z). Caught 116, missed 564,
  timeout 0, unviable 135.
  - By file (caught / missed / unviable): report3.rs 38 / 379 / 13, stage.rs 54 / 183 / 121, wf.rs 24 / 2 / 1.
  - 108 of the 135 unviable replace the return value of `trade3_once` or `trade3_once_s` (54 each).
- **Step 7:** not run (case A).
- **Step 8: 35 timing-only catches.** All 35 came from ONE test: `split18::tests::v32rb_split_o1_gates`.
  - `run-recheck.sh` listed 122 mutants: the 35, plus 87 `delete field ... from struct ... expression` mutants of the
    same two files, which `-F` does not filter (as the JOB says).
  - It ran them with `--jobs 1`: **exit 2**, 122 tested in 15 min, 4 caught, 118 missed. Test timeout 22 s.
  - **recheck_caught 0, recheck_missed 35:** every one of the 35 passed its tests at `--jobs 1`.
  - The 4 re-check catches are all extras (report3.rs:3420, report3.rs:3707, stage.rs:139 twice). They belong to
    the re-check only.
  - **Corrected caught = 116 - 35 = 81. Corrected missed = 564 + 35 = 599.** The official files stay as
    cargo-mutants wrote them.

## Anything odd

- The wall-clock gate `v32rb_split_o1_gates` made 30% of this slice's catches (35 of 116), and none survived
  `--jobs 1`. Under `--jobs 2` on 4 vCPUs it fails on load alone, as the last wave's slice 26 found.
- `target/` shows as untracked in `~/mut`: the code branch has no ignore entry for it. Nothing outside
  `results/mutants/shard-13-of-30/` was ever staged, and `scan.sh` passed on every push.
- The baseline was much faster than its ESTIMATE of 4 to 8 min: about 35 s in all.

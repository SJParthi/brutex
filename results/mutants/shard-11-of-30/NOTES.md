# Mutation slice 11/30 of engine v3.4: notes (the box writes these; see step 9)

Code only. No market data, parameter table, presigned link, token or secret was fetched, read, written or printed.
Labels: EXACT = a count. MEASURED = a wall time taken on this box. Nothing here is a trading result.

## 1. The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC x86_64 x86_64 x86_64 GNU/Linux`.
- `nproc`: 4 (Intel Xeon Processor at 2.10GHz).
- Memory: MemTotal 16,473,776 kB (15.7 GiB). No swap.
- Cgroup limit: `/sys/fs/cgroup/memory.max` does not exist on this box (cgroup v1), so step 2 printed nothing for it.
  - The memory cgroup that holds every command of this session has `memory.limit_in_bytes` = 14,338,772,992 (13.35 GiB).
  - Its `memory.max_usage_in_bytes` reached that value. Section 6 has the out-of-memory kills.
- Disk: 29 GiB available at the start (`df`).

## 2. Tools, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`.
  - Step 2's `rustup toolchain install` exited 0 with the toolchain unchanged.
- `cargo-mutants 27.1.0`.
  - This box installed it while it waited for the READY marker, with the same command step 2 gives:
    `cargo install cargo-mutants --locked --version 27.1.0`.
  - Step 2's install therefore exited 0 with "already installed".
  - Nothing else was installed, and nothing was added to the checkout.
- The JOB file's sha256 was checked against READY.txt (`078206fd...7b74`, equal).
  - Every bash block was cut out of JOB-MUTANTS.md verbatim and run in bash.
  - The only edit was section 1b's first line: K=11 and the commit.
- **Case A.** All 815 mutants of the slice are in `crates/spikefade/src/report3.rs` (EXACT):
  - from line 1642 (`build_lists`) to line 3639 (`trade_row`);
  - list positions 8,966 to 9,780 of the whole list.
  - There are no case arguments.
- The command, as `run-main.sh` ran it:
  `cargo mutants --workspace --shard 11/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip <each of the 17 names in skips.txt>`

## 3. list_check

**MATCH** (EXACT). Both lists equal the table:
- The whole list is 24,429 mutants, sha256 `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`.
- Slice 11 is 815 mutants, sha256 `5c7181169d3226eb8f38285039f6adc54e0e42761c6750970fa49f9ee9932223`.

## 4. Baseline (`baseline.log`)

| section | what | rc | wall (MEASURED) | tests (EXACT) |
|---|---|---:|---:|---|
| B1 | `cargo build --locked --workspace --tests --keep-going` | 101 | 22.2 s | exactly the 6 expected `core` targets fail to compile |
| B2a | `costs`, `greeks`, `spikefade`, no skips | 101 | 8.5 s | 691 passed, 12 failed, 3 ignored |
| B2b | `core`'s 4 compilable targets, no skips | 101 | 0.2 s | 9 passed, 5 failed |
| B3a | B2a with the 17 skips | 0 | 8.6 s | 691 passed, 0 failed, 3 ignored, 12 filtered out |
| B3b | B2b with the 17 skips | 0 | 0.1 s | 9 passed, 0 failed, 5 filtered out |

- B1's 6 targets: `core`'s lib test, `banned_lists`, `claim_reader`, `decision_citations`, `findings` and `graph`.
- The 17 B2 failures are exactly the 17 names in `skips.txt`. None is outside it.
- The last B3 pair has 0 failures.
- **ENV-EXTRA: none.** `skips.txt` is the JOB's 17 names, unchanged.

## 5. The runs and their exit codes

**Main run: exit 2** (some missed).
- cargo-mutants' own baseline passed: 19 s build + 5 s test. It auto-set the test timeout to 28 s. There was no build
  timeout.
- 815 tested in 77 min (MEASURED, 15:54:41Z to 17:11:48Z on 2026-11-16).
- **162 caught, 620 missed, 0 timeout, 33 unviable** (EXACT).

**Step 7:** not run (case A).

**Step 8, the `--jobs 1` re-check: exit 2.**
- `find-timing.sh` found **100 timing-only catches**. In each of the 100, the only failed test was
  `split18::tests::v32rb_split_o1_gates`.
- The re-check list holds 172 mutants:
  - the 100 names;
  - 72 extra `delete field ... from struct ... expression` mutants of `report3.rs`. cargo-mutants 27.1.0 lists them
    despite `-F`, as the JOB expected.
- Its baseline passed: 18 s build + 5 s test, with the timeout auto-set to 30 s.
- 172 tested in 24 min (MEASURED): 2 caught, 170 missed.
- Of the 100 names: **1 caught, 99 missed**.
  - The one caught is `report3.rs:2415:5: replace batch_next ... with Ok((0, 0))` (section 6).
- Of the 72 extras: 1 caught, 71 missed. They belong to the re-check only.
  - The extra caught is `report3.rs:3420:16: delete field x from struct Sum expression in twin10`.
- **Corrected counts** (EXACT): caught = 162 - 99 = **63**, missed = 620 + 99 = **719**, timeout 0, unviable 33.
  - The official files stay as cargo-mutants wrote them.

## 6. Odd: five out-of-memory kills, all of the spikefade test binary

The kernel's memory-cgroup OOM killer killed the `spikefade` test binary 5 times:
- 4 in the main run, at 16:28:56Z, 16:29:06Z, 16:29:16Z and 16:29:29Z. The kernel log gives anon-rss 11,302,472 to
  13,875,692 kB (10.8 to 13.2 GiB).
- 1 in the re-check, at 17:27:25Z, with anon-rss 13,859,900 kB (13.2 GiB).

The kernel log names the threads that hit the limit: `tests34s::batch` 4 times and `tests34s::sha25` once.

The kills map one to one to the mutants at `report3.rs:2415:5` that replace `batch_next -> Result<(usize, usize), String>`:
- in the main run: `Ok((0, 0))`, `Ok((0, 1))`, `Ok((1, 0))` and `Ok((1, 1))`;
- in the re-check: `Ok((0, 0))` again.

Exactly those 5 mutant logs hold `signal: 9, SIGKILL: kill`, and no other log does.
- cargo-mutants scored all 5 as CAUGHT, because the test command failed.
- No rustc, cargo or cargo-mutants process was killed. Both runs went on to the end.
- `Ok((0, 0))` was flagged timing-only because `split18::tests::v32rb_split_o1_gates` printed FAILED before the kill.
  - Under `--jobs 1` it was killed again, so it stays caught (`recheck_caught=1`).

## 7. Other notes

- Before the main run, a `git push --dry-run` checked that the clone could push to the result branch. It sent nothing.
- Progress commits were pushed at most 15 minutes apart, from 16:07:52Z to 17:36:43Z.
- `scan.sh` was clean on every push.

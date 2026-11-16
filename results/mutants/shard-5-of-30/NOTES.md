# Mutation slice 5/30 of engine v3.4: notes

Labels follow JOB-MUTANTS.md: EXACT = a count, MEASURED = a number from a named run on this box. Nothing here is a trading
result. Code only: no market data was read, fetched or written, and no presigned link or secret was used.

## Box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4 (`Intel(R) Xeon(R) Processor @ 2.10GHz`).
- Memory: `MemTotal: 16473776 kB` (about 15.7 GiB). There is no `/sys/fs/cgroup/memory.max` on this box, so no cgroup
  memory limit could be read.
- Disk: 29 GiB available at the start.

## Toolchain, source, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, from the repository's
  `rust-toolchain.toml` pin. Step 2's `rustup toolchain install` reported the toolchain as unchanged and exited 0.
- `cargo-mutants 27.1.0`.
  - This box installed it while it waited for the READY marker, with `cargo install cargo-mutants --version 27.1.0
    --locked`, the same command as step 2.
  - Step 2's `install` job then printed that `cargo-mutants v27.1.0` was already installed, and exited 0.
  - Nothing from the code branch was built or run before the READY marker appeared.
- Source: branch `cloud/spikefade-v34int-20261116` at `7a5428d30bdb8a5e0fec427848178c82907de2fb`. Step 2 printed
  `HEAD OK`. JOB-MUTANTS.md had the sha256 READY.txt names (`078206fd...`).
- Case **A**: all 815 mutants of the slice are in `spikefade`, so there are no case arguments.
- The command, run from the checkout `~/mut` by `run-main.sh`:

  ```sh
  cargo mutants --workspace --shard 5/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip <each of the 17 names in skips.txt>
  ```

## Slice (EXACT)

- `list_check=MATCH`. The whole list holds 24,429 mutants with sha256 `3a041951...`. The slice holds 815 with sha256
  `04d001d5...`. Both equal the table.
- The slice is list positions 4,076 to 4,890 (1-based). All 815 are in `crates/spikefade/src/`:
  - `mtable.rs`: 101;
  - `stress.rs`: 123;
  - `sw.rs`: 85;
  - `report.rs`: 506.

## Baseline (MEASURED, `baseline.log`)

| section | what | rc | wall | result |
|---|---|---:|---:|---|
| B1 | build every test target, `--keep-going` | 101 | 22.0 s | exactly the 6 expected `core` targets failed to compile: its lib test, `banned_lists`, `claim_reader`, `decision_citations`, `findings` and `graph` |
| B2a | `costs`, `greeks`, `spikefade`, no skips | 101 | 7.0 s | `spikefade`: 204 passed, 12 failed, 1 ignored (217 tests). Every `costs` and `greeks` target passed. |
| B2b | `core`'s 4 compilable targets, no skips | 101 | 0.15 s | the 5 `cost_invariants` names failed; the other 3 targets passed |
| B3a | as B2a, with the skips | 0 | 6.7 s | `spikefade`: 204 passed, 1 ignored, 12 filtered out; 0 failed anywhere |
| B3b | as B2b, with the skips | 0 | 0.06 s | 0 failed |

- B2's failures are exactly the 17 names of `skips.txt`, and no other.
- **No ENV-EXTRA test.** `skips.txt` is the JOB's 17 names, unchanged.

## Main run

- **Wall (MEASURED):** 15:55:00Z to 16:56:12Z, which is 3,672 s. cargo-mutants reports 61 min.
- cargo-mutants' own unmutated baseline passed in 17 s build + 4 s test. It auto-set the test timeout to 22 s.
- **Exit code 2:** some mutants were missed.

**Result (EXACT, from `outcomes.json`).** The last 2 columns come from step 8.

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total | timing-only catches | of them missed at `--jobs 1` |
|---|---:|---:|---:|---:|---:|---:|---:|
| `mtable.rs` | 23 | 78 | 0 | 0 | 101 | 11 | 10 |
| `stress.rs` | 24 | 99 | 0 | 0 | 123 | 6 | 6 |
| `sw.rs` | 10 | 71 | 0 | 4 | 85 | 4 | 4 |
| `report.rs` | 59 | 444 | 0 | 3 | 506 | 35 | 35 |
| **all 4 files** | **116** | **692** | **0** | **7** | **815** | **56** | **55** |

The 7 unviable mutants:

```
crates/spikefade/src/sw.rs:208:9: replace + with - in words32
crates/spikefade/src/sw.rs:208:9: replace + with * in words32
crates/spikefade/src/sw.rs:314:9: replace && with || in from_args
crates/spikefade/src/sw.rs:313:9: replace && with || in from_args
crates/spikefade/src/report.rs:468:33: replace && with || in cmd_run
crates/spikefade/src/report.rs:580:5: replace bench_windows -> Bench with Default::default()
crates/spikefade/src/report.rs:1189:25: replace && with || in write_results
```

## Wall-clock re-check (step 8)

- **Timing-only catches (EXACT):** `find-timing.sh` found 56 of the 116 catches. In all 56, the ONLY failing test was
  `split18::tests::v32rb_split_o1_gates`. None of the other 3 wall-clock tests appears.
- **The re-check:** `--jobs 1`, 76 mutants: the 56 names plus 20 extras.
  - Wall (MEASURED): 16:56:31Z to 17:05:08Z.
  - Its own baseline passed in 17 s build + 3 s test, and it auto-set the test timeout to 20 s.
  - Exit code 2.
- **Counts (EXACT):** `recheck_caught=1`, `recheck_missed=55`, `recheck_extra=20`.
- **Corrected by the JOB's step 8 formula:**
  - caught = 116 - 55 = **61**;
  - missed = 692 + 55 = **747**.
  - The official files are unchanged, as cargo-mutants wrote them.

### Odd 1: the one re-check catch is again timing-only

`crates/spikefade/src/mtable.rs:503:86: replace + with * in cmd_margincheck2` stayed CAUGHT at `--jobs 1`. In the re-check,
its only failing test was again `split18::tests::v32rb_split_o1_gates`, panicking at `split18.rs:2172:9`:

```
the account's split cost grows with the months: p50 13810 ns at 25 vs 49834 ns at 97
```

So that gate failed once even with a single job. It was not run a third time.
- If this catch is read as a timing flake too, the slice would be 60 caught and 748 missed.
- That is a reading, not an official field. The SUMMARY line keeps `recheck_caught=1`.

### Odd 2: the 20 extras

- **What they are:** `delete field ... from struct ... expression` mutants: 9 in `mtable.rs` and 11 in `report.rs`.
  cargo-mutants 27.1.0 did not apply `-F` to them, as the JOB says.
- **Where they sit in the full list:** 5 are in slice 4, 4 in slice 5 (this slice) and 11 in slice 6.
- **8 were caught in the re-check by real tests, not by a timing test:**
  - 4 by `tests33g::v33f_idx_exposure_dated_conservative` (`mtable.rs:297`, `idx_dated`);
  - 4 by `tests34::v34_stale_file_strict_rescan` (`mtable.rs:619`, `strict_span`).
- The other 12 were missed.
- They belong to the re-check only.

### Other notes

- **Missed mutants by file:** they are mostly in `report.rs`: 444 of its 506 mutants were missed, mostly in `cmd_run` and
  `write_results`.
- **Exit codes:**
  - toolchain 0, install 0, baseline job `bl` 0;
  - main run 2;
  - re-check 2.
- **Progress pushes:** 16:08, 16:21, 16:34, 16:47 and 17:00 (UTC).
- **Commit trailers:** a local, untracked `commit-msg` hook in the checkout's `.git/hooks` adds the session's attribution
  trailers to each commit message. It changes no file.

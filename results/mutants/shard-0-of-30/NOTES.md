# Mutation slice 0/30 of engine v3.4: notes

Labels follow JOB-MUTANTS.md: EXACT = a count, MEASURED = a number from a named run on this box. Nothing here is a trading
result.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4. `MemTotal`: 16473776 kB (about 15.7 GiB).
- cgroup limit: `/sys/fs/cgroup/memory.max` does not exist on this box, so no cgroup memory limit could be read.
- Disk: 29 GB free at the start (`df -P -BG ~`).

## Versions, source and case

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, `cargo-mutants 27.1.0`.
- Source: branch `cloud/spikefade-v34int-20261116` at `7a5428d30bdb8a5e0fec427848178c82907de2fb` (HEAD OK). JOB file
  `JOB-MUTANTS.md` sha256 `078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74`, equal to the READY marker's.
- **Case C** (EXACT): the slice holds core 642 + costs 173 = 815 mutants, as the JOB's table expects.
- `list_check`: **MATCH**. Whole list 24,429 mutants, sha256 `3a041951...c15432a`; slice 0 815 lines, sha256
  `c70307bc...7d13157d1`. Both equal the table.

### Exact commands (run from `~/mut`)

Main run (`run-main.sh`, case C arguments from `case-args.txt`):

```sh
cargo mutants --workspace --shard 0/30 --jobs 2 --cargo-arg=--locked --test-package costs --baseline=skip --timeout 60 \
  -o ~/mo -- -- --exact --skip <each of the 17 names in skips.txt>
```

Core follow-up (`run-core.sh`, step 7). Its selection check passed first: `cargo mutants --workspace -f 'crates/core/**' --list`
equals the slice's 642 `crates/core/` lines byte for byte, so there was no exit 9 and no `-p core` retry.

```sh
cargo mutants --workspace -f 'crates/core/**' --jobs 2 --cargo-arg=--locked --cargo-arg=--test=bound_probe \
  --cargo-arg=--test=cost_invariants --cargo-arg=--test=lint --cargo-arg=--test=zerodha_index \
  -o ~/mo-core -- -- --exact --skip <the same 17 names>
```

Wall-clock re-check (step 8): `find-timing.sh` read all 113 caught mutants of the main run from `outcomes.json` and found
**0** whose failing tests are only wall-clock tests. None of the 113 logs names any of the 4 tests of `timing-tests.txt`
(they are `spikefade` tests, and this slice ran `costs`' tests). `run-recheck.sh` printed `no timing-only catches` and
exited 0 without running cargo-mutants, so there is no `recheck-jobs1/` folder.

## Baseline (MEASURED, `baseline.log`)

| section | command | rc | wall | tests |
|---|---|---:|---:|---|
| B1 | `cargo build --locked --workspace --tests --keep-going` | 101 | 25.6 s | exactly 6 `could not compile`, all `core`: lib test, `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph` |
| B2a | `cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade` | 101 | 8.5 s | 691 passed, 12 failed, 3 ignored |
| B2b | `cargo test --locked --no-fail-fast -p core --test bound_probe --test cost_invariants --test lint --test zerodha_index` | 101 | 0.15 s | 9 passed, 5 failed |
| B3a | B2a with `-- --exact` and the 17 skips | 0 | 8.2 s | 691 passed, 0 failed, 3 ignored, 12 filtered out |
| B3b | B2b with `-- --exact` and the 17 skips | 0 | 0.08 s | 9 passed, 0 failed, 5 filtered out |

- B1 is exactly the JOB's prediction. No other target failed to compile.
- B2's 17 failures are exactly the 17 names in `skips.txt` (12 engine, 5 core). None is outside the list.
- The engine's lib suite is 204 passed, 12 failed and 1 ignored, so 217 tests, the count the JOB names.
- The last B3 pair has 0 failures.

**ENV-EXTRA: none.** `skips.txt` is the JOB's 17 names, unchanged.

## Results (EXACT)

| run | caught | missed | timeout | unviable | total | exit | test timeout | wall (MEASURED) |
|---|---:|---:|---:|---:|---:|---:|---|---|
| main, case C (`~/mo`) | 113 | 536 | 3 | 163 | 815 | 3 | fixed 60 s (`--timeout 60`) | 15:54:34Z to 16:15:57Z, 21 min 23 s |
| core follow-up (`core-followup/`) | 51 | 506 | 2 | 83 | 642 | 3 | auto-set 20 s | 16:16:09Z to 16:21:34Z, about 5 min 25 s |
| re-check `--jobs 1` | - | - | - | - | 0 | 0 | - | not run: no timing-only catch |

Exit 3 means at least one mutant timed out. The JOB allows it, so both runs went on.

### Main run by file (caught / missed / timeout / unviable)

| file | caught | missed | timeout | unviable |
|---|---:|---:|---:|---:|
| `crates/core/src/alias.rs` | 0 | 59 | 0 | 40 |
| `crates/core/src/blake3.rs` | 0 | 96 | 0 | 2 |
| `crates/core/src/error.rs` | 0 | 3 | 0 | 0 |
| `crates/core/src/instrument.rs` | 7 | 46 | 0 | 18 |
| `crates/core/src/isin.rs` | 0 | 35 | 0 | 1 |
| `crates/core/src/knob.rs` | 0 | 7 | 0 | 0 |
| `crates/core/src/price.rs` | 3 | 39 | 0 | 3 |
| `crates/core/src/secret_shape.rs` | 0 | 49 | 0 | 1 |
| `crates/core/src/symbol.rs` | 2 | 5 | 0 | 0 |
| `crates/core/src/universe.rs` | 0 | 50 | 0 | 20 |
| `crates/core/src/vendor.rs` | 0 | 147 | 0 | 9 |
| `crates/costs/src/charge_error.rs` | 1 | 0 | 0 | 2 |
| `crates/costs/src/dated.rs` | 9 | 0 | 0 | 13 |
| `crates/costs/src/day.rs` | 91 | 0 | 3 | 54 |
| **core (11 files)** | **12** | **536** | **0** | **94** |
| **costs (3 files)** | **101** | **0** | **3** | **69** |

### Core follow-up by file (caught / missed / timeout / unviable)

| file | caught | missed | timeout | unviable |
|---|---:|---:|---:|---:|
| `alias.rs` | 0 | 59 | 0 | 40 |
| `blake3.rs` | 0 | 96 | 0 | 2 |
| `error.rs` | 0 | 3 | 0 | 0 |
| `instrument.rs` | 1 | 63 | 0 | 7 |
| `isin.rs` | 0 | 35 | 0 | 1 |
| `knob.rs` | 0 | 7 | 0 | 0 |
| `price.rs` | 0 | 42 | 0 | 3 |
| `secret_shape.rs` | 0 | 49 | 0 | 1 |
| `symbol.rs` | 2 | 5 | 0 | 0 |
| `universe.rs` | 12 | 37 | 1 | 20 |
| `vendor.rs` | 36 | 110 | 1 | 9 |

The first ten rows equal the last wave's shard 0 (v3.3 kit) row for row. `vendor.rs` grew from 36 mutants to 156.

### Timeouts

- Main (60 s): `crates/costs/src/day.rs:292:22` (`<` to `<=` in `TradeDay::before`), `day.rs:504:35` (`-` to `/` in
  `days_from_civil`) and `day.rs:504:25` (`+` to `-` in `days_from_civil`).
- Core follow-up (20 s): `crates/core/src/universe.rs:4469:22` (`+` to `*` in `MemberIndex<N>::position`) and
  `crates/core/src/vendor.rs:458:15` (`+=` to `*=` in `MasterRow<'_>::over_wide`).

## Read this before using a MISSED count

- **Main, 536 missed:** every one is a `core` mutant that `costs`' tests do not catch. Case C tests every mutant of the
  slice with `--test-package costs`. None of `costs`' own 173 mutants was missed.
- **Core follow-up, 506 missed:** not caught by the 9 `core` tests that can run here (`bound_probe` 2, `lint` 2,
  `zerodha_index` 5; `cost_invariants`' 5 tests are all skipped). `core`'s lib unit tests and the 5 other integration
  targets cannot compile on this branch, so `core`'s own full suite never ran.
- **Across the two runs, derived from their own lists (EXACT):** 61 of the 642 `core` mutants were caught by at least
  one run. 12 were caught by `costs`' tests and 51 by `core`'s 4 targets; 2 were caught by both. 485 were missed by both.
- A mutant that only a skipped test would catch counts as missed (JOB step 4).

## Anything odd

1. **11 `core` mutants are unviable in the main run only.** All 11 are in `crates/core/src/instrument.rs`, lines 165 to
   193, for example `instrument.rs:165:9` (`Expiry::days_in_month -> u8` with 0 or 1). In the follow-up all 11 are
   viable and missed. Each of their 11 main-run logs fails only with `error[E0080]: evaluation panicked: assertion
   failed: is_series(...)`, at `crates/costs/src/equity_dated.rs` lines 783 to 788. Those lines are compile-time `const`
   assertions in `costs` that call `core`'s const functions, so building `costs` rejects the mutant. cargo-mutants counts
   a build failure as unviable, not caught.
2. **`test_timeout_s=none` in SUMMARY.** `final.sh` reads the timeout from cargo-mutants' `Auto-set test timeout` line
   in `main.log`. Case C passes `--timeout 60` and skips the baseline, so that line does not exist. The main run's
   timeout was the fixed 60 s. The core follow-up's was auto-set to 20 s (`core.log`).
3. **Two steps outside the JOB's blocks.** Both happened while waiting.
   - Before the READY marker appeared, cargo-mutants 27.1.0 was pre-installed with
     `cargo install --locked cargo-mutants --version 27.1.0`. Step 2's install job therefore printed `already installed`
     and exited 0.
   - After the main run started, one `git push --dry-run` of the result branch checked push access. It changed nothing
     on the remote.
4. The last command of step 5's check block prints 0, as required. Its exit status is 1 because `grep -c` found no match.
5. The checkout holds an untracked `target/` from the baseline. Only `results/mutants/shard-0-of-30/` is ever staged.
6. The READY marker was first seen at 15:51:15Z. This box began polling at 14:53:20Z, checking every 60 s.

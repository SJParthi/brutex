# Mutation slice 18/30 of engine v3.4: notes (the box writes these; see step 9)

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- memory: `MemTotal: 16473776 kB` (about 15.7 GiB).
- cgroup limit: there is no `/sys/fs/cgroup/memory.max` (the box is on cgroup v1, so the step 2 `cat` printed
  nothing). The v1 file `/sys/fs/cgroup/memory/memory.limit_in_bytes` reads `9223372036854771712`, the "no limit" value.
- disk at set-up: `df -P -BG ~` gave 252G size, 12G used, 28G available.

## Versions, case and commands

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)` (the step 2 toolchain command reported
  1.97.1 "unchanged": it was already installed).
- `cargo-mutants 27.1.0`, installed by step 2's `cargo install cargo-mutants --locked --version 27.1.0` (exit 0).
- Checkout: `~/mut`, a clone of `cloud/spikefade-v34int-20261116`, HEAD = `7a5428d30bdb8a5e0fec427848178c82907de2fb`
  (step 2 printed `HEAD OK`). The job file at that commit hashed to the sha256 that READY.txt names.
- Case: **A** (all 815 mutants of the slice are in `spikefade`; no `core` mutant), so no case arguments were added.
- The exact main command (run-main.sh, K=18, case A):

  `cargo mutants --workspace --shard 18/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip <name>`,
  with one `--skip <name>` for each of the 17 lines of `skips.txt`, in order.

- The re-check command (run-recheck.sh, step 8):

  `cargo mutants --workspace -f crates/spikefade/src/fix33.rs -f crates/spikefade/src/fix34.rs -f crates/spikefade/src/fixg.rs`
  plus one anchored `-F '^<name>$'` for each of the 25 names in `recheck-jobs1/recheck-names.txt`, then
  `--jobs 1 --cargo-arg=--locked -o ~/mo-recheck -- -- --exact` and the same 17 skips.

## The list

- `list_check=MATCH`. The whole-workspace list is 24,429 mutants with sha256
  `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`, and slice 18 is 815 mutants with sha256
  `45b904d605df00434012fcb3cbc152ddb53223742ea571d68acea4ecb31ae59f`, both exactly the table's values.
- The slice by file: `fix34.rs` 383 (most of them `BandTable::parse` return-value replacements), `fix33.rs` 216,
  `fixg.rs` 203, `sw33.rs` 13. All `spikefade`.

## Baseline (step 5)

| section | command (short) | rc | wall |
|---|---|---|---|
| B1 | `cargo build --locked --workspace --tests --keep-going` | 101 | 20,844 ms |
| B2a | `cargo test -p costs -p greeks -p spikefade` (no skips) | 101 | 8,290 ms |
| B2b | `cargo test -p core` with the 4 compilable targets (no skips) | 101 | 178 ms |
| B3a | as B2a, with `--exact` and the 17 skips | 0 | 8,722 ms |
| B3b | as B2b, with `--exact` and the 17 skips | 0 | 81 ms |

- B1: exactly the 6 expected `could not compile` errors, all `core`: its lib test and the test targets `banned_lists`,
  `claim_reader`, `decision_citations`, `findings` and `graph` (missing `include_str!` files of the app repository).
  No other target failed to compile.
- B2: 17 failed tests, and all 17 are the 17 names of `skips.txt` (12 engine, 5 core). The engine suite's main target
  gave 204 passed, 12 failed, 1 ignored (217 tests); `core`'s `cost_invariants` gave 0 passed, 5 failed.
- B3: 0 failed tests in both sections (the engine target: 204 passed, 1 ignored, 12 filtered out; `core`: 5 filtered
  out). The last check line printed 0.
- No `ENV-EXTRA` test. `skips.txt` is unchanged from step 4 (17 lines).

## The run (step 6)

- cargo-mutants' own baseline: `ok Unmutated baseline in 18s build + 5s test`, then `Auto-set test timeout to 30s`.
- Started 2026-11-16T15:55:41Z, ended 2026-11-16T17:15:50Z (80 min). cargo-mutants' last line:
  `815 mutants tested in 80m: 193 missed, 605 caught, 7 unviable, 10 timeouts`.
- **Exit code 3** (some mutants timed out), which step 6 lists as "go on".
- Progress commits were pushed about every 13 minutes (after every third `wait.sh`); `tested=` rose at every one.
- The 10 timeouts (at the 30 s limit): 7 in `fix33.rs` `kth_union` (lines 170 to 176), `fix33.rs:354:15` (`+=` to `*=` in
  `cap_minutes`), `fix33.rs:437:31` (`<<` to `>>`) and `fix33.rs:449:33` (`>` to `<` in `StepAcc::merge`).
- The 7 unviable mutants are return-value replacements that do not compile (for example `Default::default()` for a
  row type, and two `*` to `/` changes in `cap_minutes`). See `unviable.txt`.
- Case A, so step 7 (the `core` follow-up) did not run: its block was run as written and did nothing.

## The wall-clock re-check (step 8)

- `find-timing.sh` found **25** caught mutants whose test log names only wall-clock tests as FAILED. In all 25 the only
  failed test is `split18::tests::v32rb_split_o1_gates`; the other three tests of `timing-tests.txt` never appear.
- The re-check list is exactly those 25 names (`recheck_extra=0`: no extra `delete field` mutants were listed).
- Re-check run: cargo-mutants' baseline `ok` (18s build + 5s test), test timeout auto-set to 28 s, then
  `25 mutants tested in 4m: 25 missed`. **Exit code 2**, which step 8 lists as fine.
- So every one of the 25 timing-only catches passes its tests under `--jobs 1`: `recheck_caught=0`,
  `recheck_missed=25`.
- The official files stay as cargo-mutants wrote them. Corrected per step 8: **caught 605 - 25 = 580**, **missed
  193 + 25 = 218**, timeout 10, unviable 7.

## Anything else

- While the box waited for READY (before it had read the job file), it pre-installed the repository CI's pinned tools,
  `cargo-mutants 26.2.0` and `cargo-nextest 0.9.145`, from crates.io, and ran `cargo fetch --locked` (downloads only,
  no build) in its own session checkout. Step 2 then replaced cargo-mutants with 27.1.0, exactly as written. No command
  of this job used `cargo-nextest`. No market data was fetched, read or written at any point.
- Step 2's `rustup toolchain install` also printed rustup's own `checking for self-update` / `downloading self-update`
  lines; the toolchain itself was unchanged.
- Limit (from the job file): a mutant that only a skipped test would catch is counted as missed.

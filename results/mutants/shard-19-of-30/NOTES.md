# Mutation slice 19/30 of engine v3.4: notes

Box k=19 of 30 in the data-free cloud wave 2, 16 Nov 2026. Code only: no market data was fetched, read or written, and
no code was changed. Every number below is MEASURED on this box unless it says otherwise.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- Memory: `MemTotal: 16473776 kB` (about 15.7 GiB).
- cgroup memory limit: none. The box runs cgroup v1, so `/sys/fs/cgroup/memory.max` does not exist and step 2's `cat`
  printed nothing. The v1 memory cgroup's `memory.limit_in_bytes` reads 9223372036854771712, which means no limit.
- Disk at the start: a 252G file system with 30G available.

## Toolchain, case and the exact command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, as the JOB requires. The toolchain
  was already on the box: step 2's `rustup toolchain install` reported it unchanged and exited 0.
- `cargo-mutants 27.1.0`, installed with `cargo install cargo-mutants --locked --version 27.1.0` (exit 0).
- Checkout: `~/mut`, branch `cloud/spikefade-v34int-20261116`, HEAD 7a5428d30bdb8a5e0fec427848178c82907de2fb (HEAD OK).
- **Case A**: the slice holds no `core` mutant, so no case arguments were added (`case-args.txt` is empty).
- The exact command (`run-main.sh`, run in `~/mut`):

  `cargo mutants --workspace --shard 19/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip <name> ...`

  with one `--skip <name>` for each of the 17 lines of `skips.txt`, in file order.

## The list

- **list_check MATCH.** The whole list has 24,429 mutants with sha256
  3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a, and slice 19 holds 815 with sha256
  68a587480a6ce53674624f22b4a3f25653a211b69dbdb57778c5e9d36780ca09. Both equal the JOB's table.
- All 815 mutants of the slice are in `crates/spikefade/src/fix34.rs`, from line 913 (`BandTable::parse`) to line 1301
  (`wick_compare`). By function (EXACT, from `list-shard.txt`): `BandTable::parse` 413, `wick_compare` 234, `sha256_hex`
  54, `band3` 29, `carry_stats` 20, `band_of` 13, `mem_over` 8, `event_of` 6, `verified_text` 5, `EventCal::load` 5,
  `wick_mark` 4, `unique_id` 4, `mem_now` 4, `argv_hash` 4, `write_atomic` 2, `with_sha_line` 2, `sync_dir` 2,
  `carry_words` 2, `band_words` 2, `create_run_dir` 1, `BandTable::load` 1.

## The baseline (section 5)

| section | command | rc | wall |
|---|---|---|---|
| B1 | `cargo build --locked --workspace --tests --keep-going` | 101 | 19,028 ms |
| B2a | `cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade` | 101 | 7,868 ms |
| B2b | `cargo test --locked --no-fail-fast -p core` with the 4 `--test` targets | 101 | 204 ms |
| B3a | B2a plus `-- --exact` and the 17 skips | 0 | 7,863 ms |
| B3b | B2b plus `-- --exact` and the 17 skips | 0 | 66 ms |

- B1: exactly the 6 expected `could not compile` errors, all `core`: its lib test, and the test targets `banned_lists`,
  `claim_reader`, `decision_citations`, `findings` and `graph`. No other target failed to compile.
- B2: 17 failed tests, every one in `skips.txt`: 12 in `spikefade`'s unit tests (204 passed, 12 failed, 1 ignored) and 5
  in `core`'s `cost_invariants`. Every `costs` and `greeks` target passed.
- B3: 0 failed tests and rc 0 in both sections. The step 5 count over the last B3 pair printed 0.
- B2b and B3b are short because B1 had already built their test binaries.

## ENV-EXTRA

None. No test outside `skips.txt` failed, so `skips.txt` is the JOB's 17 names unchanged.

## The run and the exit codes

- Exit codes: toolchain 0, install 0, baseline job 0, **main 3**, **recheck 2**. Step 7 did not run (case A).
- cargo-mutants' own baseline: ok, 17 s build + 5 s test. It auto-set the test timeout to 29 s.
- **Main run** (exit 3, some timed out): 2026-11-16T15:56:20Z to 2026-11-16T17:08:38Z, 72 minutes. 815 tested:
  **423 caught, 389 missed, 1 timeout, 2 unviable.**
  - Timeout: `fix34.rs:1063:21: replace % with + in sha256_hex` (it reached the 29 s limit).
  - Unviable: `fix34.rs:1131:9: replace && with || in sync_dir` and `fix34.rs:1279:17: replace && with || in wick_mark`.
  - Missed, by function: `BandTable::parse` 368, `EventCal::load` 5, `sha256_hex` 3, `wick_mark` 2, `mem_now` 2,
    `carry_stats` 2, `band_words` 2, `write_atomic` 1, `sync_dir` 1, `carry_words` 1, `argv_hash` 1, `BandTable::load` 1.
- **Wall-clock re-check (step 8):** 48 timing-only catches. Every one of the 48 failed ONLY
  `split18::tests::v32rb_split_o1_gates`. The `--jobs 1` re-check listed exactly those 48 mutants (no extra
  `delete field` mutants, so `recheck_extra` is 0), set its own test timeout to 28 s, and exited 2 after 7 minutes:
  **all 48 MISSED, 0 caught.** By function: `BandTable::parse` 45, `wick_mark` 1, `mem_over` 1, `carry_words` 1.
  - corrected caught = 423 - 48 = **375**
  - corrected missed = 389 + 48 = **437**
  - The official `caught.txt` and `missed.txt` stay as cargo-mutants wrote them.

## Anything odd

- **Every `BandTable::parse` mutant is missed once the re-check is applied:** 368 missed in the main run plus 45
  timing-only catches that the re-check found missed, which is all 413 of the slice's `BandTable::parse` mutants. Nearly
  all of them replace the function's return value with a fixed `Ok(HashMap::from_iter(...))`, so no test that runs on this
  branch checks what `BandTable::parse` returns. (The tables that would exercise it are not on this code-only branch;
  that is a reading of the result, not a measured cause.)
- **Every `wick_compare` mutant was caught** (all 234 of the slice's).
- The timing-only effect is the one the JOB describes for the last wave's slice 26 (11 of 83 there). Here it is 48 of 423
  catches: under `--jobs 2` the O(1) gate test failed on load alone, and under `--jobs 1` it passed for all 48.
- The READY marker appeared 56 minutes after this box began polling for it. `box.start` (2026-11-16T15:53:22Z) is when
  section 1b ran, after READY, so `box_wall_s` does not include that wait.
- Progress commits were pushed at most 15 minutes apart (six of them before the final commit).

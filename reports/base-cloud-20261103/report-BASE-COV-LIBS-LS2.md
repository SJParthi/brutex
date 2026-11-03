STATUS: IN PROGRESS

# BASE-COV-LIBS-LS2 report (shard LS2: crate `runner`)

TOKEN BASE-COV-LIBS-LS2-20261103-1. Branch `cloud/cov-libs-ls2-20261103`, cut from `origin/cloud/base-kit-libs-20261103` (`eeec369be7dc`, code START `bd04fb299d23`).

## Host

Linux x86_64, 4 cores, 15 GiB memory, root (uid 0), 29 GiB free at start. Stable `rustc 1.97.1 (8bab26f4f 2026-07-14)` (the repository's pin). Coverage nightly installed as `nightly-2026-08-08`: `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, its `llvm-cov` and `llvm-profdata` print `LLVM version 23.1.0-rust-1.99.0-nightly` (the pins of `d3-route-b-cloud.sh`). `TMPDIR=/dev/shm/ls2-tmp` (tmpfs), one `CARGO_TARGET_DIR=/home/user/tgt-ls2` (Route B builds into its `cov/`), `CARGO_PROFILE_DEV_DEBUG=0`.

## Progress (append-only)

### 2026-11-03 15:07 IST start
Kit checked: the 14 kit files I use match `MANIFEST.md`'s sha256 (whole-file and fenced forms); `fix-BASE-COV-LIBS.json` passes `jq -e .`. The 28 Oct "after" gap list (`base-cov-libs-gaps-a1-after.txt.md`) has no runner row (runner was not re-measured after attempt 1), so runner is measured fresh here. OWNER commit `0de5bfcd`.

### 2026-11-03 15:10 IST step 1: `cargo test -p runner --lib --locked --no-fail-fast`
First run (START + OWNER): did not compile, one error, attempt 4's `signal_candle_stop_tests.rs:888` called `Evaluation::alignment()`, which does not exist (`Prepared::alignment()` does). Fixed to `fixture.prepared().alignment()`. Second run: `test result: ok. 800 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 68.52s` (stable, opt-level 3). So every runner lib test of attempts 1-4 compiles and passes on this host. Commit `4f49dbf43e79`.

### 2026-11-03 15:12 IST Route B started at `4f49dbf43e79`
`d3-route-b-cloud.sh /home/user/deep-ls2 /home/user/tgt-ls2 /home/user/ls2-prof/4f49dbf43e79-1-B <logs>/4f49dbf43e79-cov <set> runner` with `D3_NIGHTLY`, `D3_LLVM_BIN`, `D3_CARGO` set to the nightly's own binaries and `D3_DIR` to the extracted kit scripts. `/home/user/deep-ls2` is a detached worktree at the tip. Set: `reading whole:runner`, 70 coverage files whole (`src/`, `tests/`, `benches/`). Instrumented build rc 0 in 13 s, 14 test objects.

NEXT: read the Route B figures; write `gaps-LS2.txt.md` and `fu-LS2.txt.md`; close the GAP rows (validate.rs, expression_execution, exit_grid_policy replay refusals, report.rs, signal_candle_stop.rs first).

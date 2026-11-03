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

### 2026-11-03 16:43 IST Route B at START done (before figures)
`ROUTE-B DONE 2026-11-03 16:42:58 IST WITH NOT PROVED lines above`, exit 3. Build 13 s (30 artifacts, 0 reused, 14 test objects); `== test runner rc=0 END 2026-11-03 16:42:30 IST (89 min); tests passed 857 failed 0 ignored 0`; 14 profraw files merged. **runner at `4f49dbf43e79` (= START + the one-line test fix): lines 46145/47136 (991 uncovered), branches 3349/4218 (869 uncovered), 69 of 70 set files read.** NOT PROVED: `crates/runner/benches/ratio.rs` (its `[[bench]]` target is `test = false`, so `cargo test --no-run` builds no object for it; the Mac's attempt 1 recorded the same for every benches/ file, operator item L-BCL-1). For comparison the Mac's attempt 1 measured runner at f01d7a0e4: lines 44019/45694 (1675 uncovered), branches 3156/4188 (1032 uncovered). Classified (my node classifier, scratchpad; recipe v4 step 4 rules): 224 (F) rows, 1587 GAP rows, 49 rows to read by hand; GAP rows in production code by file: validate 402, exit_grid_policy 218, grid 184, signal_candle_stop 72, expression_execution 67, trade 56, resolved_grid_view 39, outcome 34, bootstrap 33, audit 31, expression_validation 28, excursion 17, admission 13, lib.rs 13, research_admission_projection 9, topn 5, bootstrap_zero_v2 4, expression_oos 3, research_exit_grid 3, report 2, and 1 each in expression, bootstrap_family_pass, pbo, significance.

### Since then (commits `c1b915c2`, `074584f3`, `0d315ca4`)
- **signal_candle_stop**: every reachable refusal arm of Trade/Event/Period/Metrics decode, the run-id scope refusal, canonical_timeframe and validate_source (8 tests in `signal_candle_stop_tests.rs`). Unofficial figure for the file (instrumented, its own tests only): lines 1189/1202 -> 1198/1202, branches 189/248 -> 236/248; the remaining 4 lines and 12 sides are (U) with proofs, except line 939 (fault-only, see below). Correction: the commit message of `c1b915c2` says "1185/1198 -> 1198/1202 lines"; the measured before figure is 1189/1202 (branches 189/248 as written).
- **report**: NotLive exclusion and Workers halt on the plain path; the existing exclusion test now asserts its fixture's reason and measurement exactly. **research_admission_projection**: test module for `with_search_probability_ceiling`. **expression**: the sources-length alignment refusal; the refusal test's visitor also runs on good columns. **significance**: a fraction that never converges answers NaN. **bootstrap_zero_v2**: the work cap. **topn**: the fixture helper refuses an out-of-range bit. **rank**: attempt 4's `assert!(a == b)` made `assert_eq!` (clippy `manual_assert_eq`).
- **validate**: new `validate_cover_tests.rs` (11 tests): V2-to-V3 refusal conversion, V4 accessors, every seal and provenance cross-wire of the V2/V3/V4 reconciliations, every fold-capture refusal, choice_matches_visible_v2 operand by operand, finish, zero-rung capture policies, a trailing take-profit's two rungs, the eight signal-length labels.
- Red checks: each new test was run under a hand change of the rule it pins and failed (signal_candle_stop 8 of 8, batch 2 6 of 6, validate 11 of 11), then the file was restored and re-checked clean.

NEXT: validate.rs walk_forward_core / walk_forward_exact_grid_v4 / authenticate_signal_column_shape_v4 / project_oos_fold; then exit_grid_policy.rs, grid.rs, expression_execution.rs, trade.rs, resolved_grid_view.rs, outcome.rs, bootstrap.rs, audit.rs, expression_validation.rs, excursion.rs, admission.rs, lib.rs (tests only: lib.rs is LSA's).

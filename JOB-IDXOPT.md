# JOB-IDXOPT: coverage and mutation testing of the index-options tool (cloud, code only)

Branch `cloud/idxopt-v3-20261116`. **CODE ONLY.** The repository is public: every push publishes.

Labels: **MEASURED** = a number from a named run. **ESTIMATE** = measured inputs times a model. **EXACT** = a count.
No figure here is a trading result.

Every box follows this file exactly. Your first message names your JOB (`cov` or a shard number `k` from 0 to 7), the
source commit and your result branch. The driver `box.sh` (section 11) runs every command; this file says why.

## 0. Rules (every box)

- **Code only.** Never fetch, add, print or upload market data. No presigned links: this job needs none. No secrets or
  tokens. Nothing from the app's runtime folder.
- **Never change a file of the checkout.** You write only under `results/<your folder>/` in the checkout, plus `~/box.sh`,
  `~/logs`, `~/out-*`, `~/tgt-*`, `~/cmt`, `~/SRC_SHA` and `~/MANIFEST-idxopt.txt` outside it.
  - No package installs (apt or other), no extra symlinks, no placeholder files, no extra test skips, no other flags.
- **Push only to your result branch** (section 1), only files under `results/`, never force, never another branch.
  `box.sh progress` refuses anything else.
- **Keep your turn busy until the end.** An idle box's VM pauses and loses background work. Every command you run returns
  within 270 s. The long work runs under `nohup`, and you poll it (section 5).
- **On any failure, stop.** Push what you have and report it. Do not work around it, do not retry with changes.

## 1. Boxes

| box | JOB | work | result branch | result folder |
|---|---|---|---|---|
| 0 | `cov` | coverage, cargo-llvm-cov 0.9.1 | `cloud-results/idxopt-coverage` | `results/idxopt-coverage/` |
| 1 to 8 | `k` = box - 1, so 0 to 7 | cargo-mutants 27.1.0, shard k of 8 | `cloud-results/idxopt-mutants-<k>-of-8` | `results/idxopt-mutants/shard-<k>-of-8/` |

## 2. What this branch is

- **The tool.** The index-options tool (NIFTY / BANKNIFTY options bought, wave 4) at its newest checked state.
  - The host tree `sniper-host-v3-idx-w4r3/`: the S9 re-check, round 3, passed it on 16 Nov 2026.
  - `common-fill-v2/` beside it, as on the operator's Mac. The host reaches it by the relative path `../../../common-fill-v2`.
- **Two cargo workspaces.**
  - **B, "host":** `sniper-host-v3-idx-w4r3/`. Members `sniper_kernel`, `wf_sweep`, `sniper_host`.
  - **A, "idxopt":** `sniper-host-v3-idx-w4r3/pinned/w4m/opening5s-5b3r70/`. Members `core`, `store`, `telemetry`,
    `costs`, `idx_cal`, `idx_f1`, `opt_pos`. The host uses `opt_pos`, `idx_cal` and `costs` as path dependencies.
- **366 files**, only `.rs`, `.toml` and `Cargo.lock`. Each is byte-identical to the file at the same path in the
  operator's scratch folder.
  - The source manifest (section 5, step 3) has 366 lines and sha256
    `e082593fbc5101f8691bc68b08846e44bc5a0388be924c91d79f75416d4cff30`.
  - `box.sh setup` checks it and stops on any difference.
- **Left out on purpose:**
  - `data/` (the run rows);
  - every `.tsv` (tables, test vectors, index closes);
  - the `.mjs` generators, the `.md` notes, the `SHA256SUMS*` files and one `Cargo.toml.orig`;
  - `crates/store/tests/cited_commits.rs`: `store`'s own test, never built here, holding an e-mail-shaped fixture.
- **Dependencies.** Eight crates.io crates (`adler2`, `cfg-if`, `crc32fast`, `flate2`, `miniz_oxide`, `ruzstd`,
  `simd-adler32`, `twox-hash`) are fetched under `--locked`.
  - zstd is pure Rust (`ruzstd`). No external tool is called by any test that runs here.
  - The Mac-only zstd path of the spike-selling kit (`kit.rs:470`, the Linux C2 "DIFFER" cause) is not in this tree. If a
    zstd or tool-path error appears anyway, report it as that known kind of cause.

## 3. Toolchain and tools (`box.sh setup` does all of this)

- `rust-toolchain.toml` pins **1.97.1** in both workspaces: components rustfmt, clippy and llvm-tools-preview, profile
  minimal. rustup installs it on first use, which needs the network.
  - Expected `rustc --version`: `rustc 1.97.1 (8bab26f4f 2026-07-14)`. Setup stops on any other version.
  - If `rustup` is missing, setup installs it with `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s --
    -y --profile minimal --default-toolchain none`.
- **Boxes 1 to 8:** `cargo install cargo-mutants --locked --version 27.1.0`, the version every shard of the 16 Nov wave
  reported. Every shard needs the same version: slices are ranges of one list.
- **Box 0:** `cargo install cargo-llvm-cov --locked --version 0.9.1`, the version of the 16 Nov coverage box.
- Nothing else: no nightly, no apt.

## 4. Test sets, exclusions and the expected baseline

Each workspace is tested in two sets. A set is one cargo-mutants run (boxes 1 to 8) or one cargo-llvm-cov run (box 0).

| set | workspace | packages | test targets | mutated files | tests skipped by exact name | expected baseline: passed / failed / ignored / filtered (EXACT) |
|---|---|---|---|---|---:|---|
| A1 | A | `opt_pos`, `idx_cal`, `idx_f1` | `--lib` (these three have no integration or doc tests) | every `src` file but the binary drivers | 1 | 73 / 0 / 0 / 1 |
| A2 | A | `costs` | all: lib, 4 integration targets, 59 doc tests | `crates/costs/src/option_clock.rs` only | 0 | 440 / 0 / 2 / 0 |
| B2 | B | `sniper_kernel`, `wf_sweep` | all | every file | 2 | 24 / 0 / 0 / 2 |
| B1 | B | `sniper_host` | `--lib`, `--test matrix`, `--test no_cut_at`, `--test fast` | every file but `src/bin/` | 4 | 73 / 0 / 0 / 4 |

- The expected counts are the Mac's checked runs of 16 Nov (the same tests), minus the skips.
- The 2 ignored tests in A2 are the suite's own `#[ignore]` timing measurements.
- **`box.sh` stops on any other baseline count (BASELINE MISMATCH).** Do not add skips. Push and report the failing test
  and its message.

**The 7 tests skipped by exact name** (`-- --exact --skip NAME`). Each reads a file this branch does not hold.

| test | package / target | what it reads |
|---|---|---|
| `price::tests::d3_per_leg_slices_by_the_one_order_maximum` | `opt_pos` lib | the dated freeze table, by an absolute Mac path |
| `the_data_rows_load_fail_closed` | `sniper_host` `matrix` | the run rows in `data/` |
| `the_standing_labels_and_trade_count_words` | `sniper_host` `matrix` | the run rows in `data/` |
| `sn_b3_sniper_q_reads_the_352_byte_store_with_its_own_width` | `sniper_host` `matrix` | `sniper-q` reads `data/SNIPER-RUN-v3.tsv` |
| `sn20_the_exit_sets_v2_and_v3_fail_closed` | `sniper_host` `matrix` | an exits plan by an absolute Mac path, and `data/` |
| `tests_wf::sweep_eq_against_common_wf_acc` | `wf_sweep` lib | common_wf test data by an absolute Mac path |
| `tests_wf::sweep_eq_miss_and_n_against_common_wf` | `wf_sweep` lib | the same |

**Whole test targets not built in B1:** `h1` (2 tests), `sec` (5), `dials` (6), `secdials` (3) and `f1` (2).
- Every test in them reads the frozen candle store, the tick store, the session table or the run rows by absolute Mac
  paths. `f1` also needs 1,098 GiB free on its temp disk.
- On the Mac's checked runs they passed (f1 apart, by its disk gate).

**Not mutated:**
- `core`, `store`, `telemetry`: the app's shared crates. Their own tests need app-repository files and do not build here.
- `costs` apart from `option_clock.rs`. Its other 53 files are byte-identical to the `costs` and `core` that the 16 Nov wave
  mutated on branch `cloud/spikefade-shard-20261116`.
- The 9 binary drivers: `opt_pos` `src/bin_*.rs` (5), `idx_cal` `src/main.rs` and `src/bin_served.rs`, `idx_f1`
  `src/main.rs`, and `sniper_host` `src/bin/` (2).
  - No test runs them, so every mutant there would be MISSED by construction.
  - Box 0's coverage reports them.

**Mutant counts** (EXACT for cargo-mutants 26.2.0 `--list` on the Mac with the same flags; 27.1.0 may differ slightly):

| set | all shards | shard k = 0 to 6 | shard k = 7 |
|---|---:|---:|---:|
| A1 | 2,264 (opt_pos 1,512, idx_cal 689, idx_f1 63) | 283 | 283 |
| A2 | 460 | 58 | 54 |
| B2 | 984 (sniper_kernel 355, wf_sweep 629) | 123 | 123 |
| B1 | 3,848 | 481 | 481 |
| **all** | **7,556** | **945** | **941** |

## 5. How to run (every box)

Replace `JOB` below with `cov` (box 0) or your shard number k (boxes 1 to 8).

**Step 1. Clone and check the commit.**

```sh
git clone --branch cloud/idxopt-v3-20261116 --single-branch https://github.com/SJParthi/brutex.git ~/idxopt
cd ~/idxopt && git rev-parse HEAD      # must equal the commit in your first message; else stop and report
```

**Step 2. Extract the driver exactly and check it.**

```sh
awk '/^```bash box.sh$/{f=1;next} f&&/^```$/{exit} f' ~/idxopt/JOB-IDXOPT.md > ~/box.sh && bash -n ~/box.sh && sha256sum ~/box.sh
```

It must print `2e54ba7c260a43c2e217dc11e71c1aad95eebc83a6c59dbeb8370112e5783124`. Else stop and report.

**Step 3. Start the whole job in the background.**

```sh
mkdir -p ~/logs && nohup bash ~/box.sh all JOB >> ~/logs/box.log 2>&1 &
```

- `all` runs `setup`, then `baseline`, then `run`.
- **setup:** the source manifest, the toolchain, the pinned tool, and your result branch (created from the source commit,
  or resumed if it already exists on origin).
- **baseline:** the four unmutated test sets of section 4, which must give the expected counts.
- **run:** coverage (box 0), or the four mutant sets A1, A2, B2, B1 of your shard (boxes 1 to 8).

**Step 4. Poll. Never leave the turn idle.**

```sh
sleep 240; bash ~/box.sh status JOB
```

- `status` prints RUNNING or NOT RUNNING, the last `BOX` line of `~/logs/box.log`, and each set's progress
  ("B1 120 of 481").
- **Every third poll (at most 15 minutes apart), push progress:** `bash ~/box.sh progress JOB`.
  - It copies the partial mutant lists, appends a line to `PROGRESS.md`, commits `results/` only and pushes to your result
    branch.
  - It prints `PUSHED ...` or `PROGRESS FAIL ...`. A `PROGRESS FAIL` is a failure: stop and report.

**Step 5. On `BOX DONE`.**
1. Write `NOTES.md` in your result folder (section 7).
2. Run `bash ~/box.sh progress JOB` and check that it printed `PUSHED`.
3. Send your final message (section 8).

**Step 6. On `BOX STOP`.**
1. Run `bash ~/box.sh progress JOB` (it pushes the logs).
2. Write `NOTES.md` with the stop line, the tail of the failing log and what you saw.
3. Push again, then send your final message. Do not retry with changes.

**Resume (a reclaimed VM).** If `status` says NOT RUNNING and the log has neither `BOX DONE` nor `BOX STOP`:
- rerun step 3 once. It resumes on your result branch and skips every set marked DONE;
- if `~/idxopt` is gone, redo steps 1 to 3 first;
- if it is lost a second time, push what you have (`progress`) and report.

## 6. What `box.sh` runs, and why

**Baselines** (in place, each workspace with its own target folder `~/tgt-A` or `~/tgt-B`):

```sh
cd sniper-host-v3-idx-w4r3/pinned/w4m/opening5s-5b3r70
CARGO_PROFILE_DEV_DEBUG=0 cargo test --locked --target-dir ~/tgt-A -p opt_pos -p idx_cal -p idx_f1 --lib --no-fail-fast -- --exact --skip <A1 name>
CARGO_PROFILE_DEV_DEBUG=0 cargo test --locked --target-dir ~/tgt-A -p costs --no-fail-fast
cd ~/idxopt/sniper-host-v3-idx-w4r3
CARGO_PROFILE_DEV_DEBUG=0 cargo test --locked --target-dir ~/tgt-B -p sniper_kernel -p wf_sweep --no-fail-fast -- --exact --skip <2 B2 names>
CARGO_PROFILE_DEV_DEBUG=0 cargo test --locked --target-dir ~/tgt-B -p sniper_host --lib --test matrix --test no_cut_at --test fast --no-fail-fast -- --exact --skip <4 B1 names>
```

**Mutants** (boxes 1 to 8; run from each workspace's root, in this order):

```sh
# common to every set:
env -u CARGO_TARGET_DIR TMPDIR=~/cmt CARGO_PROFILE_DEV_DEBUG=0 cargo mutants --shard k/8 --jobs 2 --cargo-arg=--locked <set args> -o ~/out-<set> -- -- --exact --skip <the set's names>
# A1: -p opt_pos -p idx_cal -p idx_f1 -e 'crates/opt_pos/src/bin_*.rs' -e 'crates/idx_cal/src/main.rs' -e 'crates/idx_cal/src/bin_served.rs' -e 'crates/idx_f1/src/main.rs' --cargo-arg=--lib
# A2: -p costs -f 'crates/costs/src/option_clock.rs'
# B2: -p sniper_kernel -p wf_sweep
# B1: -p sniper_host -e 'crates/sniper_host/src/bin/**' --cargo-arg=--lib --cargo-arg=--test=matrix --cargo-arg=--test=no_cut_at --cargo-arg=--test=fast
```

- **Kept from the 16 Nov wave:**
  - `--jobs 2` and `--cargo-arg=--locked`;
  - `-- -- --exact --skip NAME`;
  - the default slice sharding;
  - the automatic test timeout, and no build timeout;
  - each mutant is tested by its own package's tests only (the 27.1.0 default).
- **`TMPDIR=~/cmt` and the symlink `~/cmt/common-fill-v2 -> ~/idxopt/common-fill-v2`.**
  - cargo-mutants copies only the workspace folder into a temporary folder, and rewrites only the copy's root `Cargo.toml`.
  - So `sniper_host`'s dependency path `../../../common-fill-v2` must resolve next to that copy.
  - `box.sh` makes the symlink. Nothing in the checkout changes.
- **`CARGO_PROFILE_DEV_DEBUG=0`** builds without debug info: faster builds, smaller target folders, the same code and the
  same checks (overflow checks and debug assertions stay on).
- **`CARGO_TARGET_DIR` stays unset.** Each cargo-mutants job keeps its own target folder.
- **Target selection (A1 `--lib`; B1 `--lib` + 3 tests) only skips building targets whose tests would all be skipped or
  that hold no tests.** Each mutant meets the same tests as with all targets built.
  - A1: the binaries' test harnesses hold 0 tests, and there are 0 doc tests.
  - B1: the 5 data-only targets, and 0 doc tests.

**Coverage** (box 0; two runs per workspace, merged by `report`; dev profile; branches are NOT measured, because
`--branch` needs nightly):

```sh
cd <workspace A root> && cargo llvm-cov clean --workspace
cargo llvm-cov --no-report --locked -p opt_pos -p idx_cal -p idx_f1 --lib --no-fail-fast -- --exact --skip <A1 name>
cargo llvm-cov --no-report --locked -p costs --no-fail-fast
cargo llvm-cov report --lcov --output-path <result folder>/A-idxopt/lcov.info
cargo llvm-cov report --summary-only > <result folder>/A-idxopt/cov-summary.txt
cd <workspace B root> && cargo llvm-cov clean --workspace
cargo llvm-cov --no-report --locked -p sniper_host --lib --test matrix --test no_cut_at --test fast --no-fail-fast -- --exact --skip <4 B1 names>
cargo llvm-cov --no-report --locked -p sniper_kernel -p wf_sweep --no-fail-fast -- --exact --skip <2 B2 names>
cargo llvm-cov report --lcov --output-path <result folder>/B-host/lcov.info
cargo llvm-cov report --summary-only > <result folder>/B-host/cov-summary.txt
```

## 7. Results layout and NOTES.md

**Boxes 1 to 8:** `results/idxopt-mutants/shard-<k>-of-8/`

| file | what |
|---|---|
| `SUMMARY.txt` | one SUMMARY line per set and one `set=ALL` line (format below) |
| `<set>/caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt`, `outcomes.json` | cargo-mutants' own output for that set, unedited |
| `<set>/mutants-tail.log` | the last 300 lines of that set's cargo-mutants log |
| `<set>/DONE` | the set ended with rc 0, 2 or 3 |
| `BASELINE.txt`, `BASELINE-OK`, `baseline-<set>.log` | the unmutated baselines |
| `FACTS.txt`, `PROGRESS.md`, `box.log` | box facts, dated progress lines, the driver's log |
| `NOTES.md` | yours (below) |

```
SUMMARY idxopt-mutants shard=<k>/8 set=<A1|A2|B2|B1> mutants=<n> caught=<c> missed=<m> timeout=<t> unviable=<u> wall_s=<s> rc=<rc> commit=<source sha> cargo_mutants=27.1.0 rustc=1.97.1
SUMMARY idxopt-mutants shard=<k>/8 set=ALL mutants=<n> caught=<c> missed=<m> timeout=<t> unviable=<u> wall_s=<s> commit=<source sha> cargo_mutants=27.1.0
```

**Box 0:** `results/idxopt-coverage/`

| file | what |
|---|---|
| `SUMMARY.txt` | one line per workspace (format below) |
| `<A-idxopt or B-host>/lcov.info`, `cov-summary.txt`, `cov.log` | cargo-llvm-cov's report, per-file table and log |
| `<A-idxopt or B-host>/PACKAGES.txt` | per-package regions, functions and lines, summed from `cov-summary.txt` |
| `A-idxopt/option_clock-row.txt` | the `option_clock.rs` row |
| `BASELINE.txt`, `baseline-*.log`, `FACTS.txt`, `PROGRESS.md`, `box.log`, `NOTES.md` | as above |

```
SUMMARY idxopt-coverage set=<A-idxopt|B-host> lines=<pct> (<hit>/<total>) functions=<pct> regions=<pct> branches=not-measured wall_s=<s> rc=<rc> runs=<set>:<rc>,... commit=<source sha> cargo_llvm_cov=0.9.1
```

**NOTES.md** (yours, short, plain):
1. The source branch, commit and manifest check.
2. The box facts (from `FACTS.txt`).
3. The baseline lines.
4. The SUMMARY lines.
5. The wall per set and in all (MEASURED).
6. Anything unusual: timeouts, flaky catches, restarts, warnings.
7. Mutants boxes: the 10 files with the most MISSED mutants, with their counts. The full list is in `missed.txt`.
8. Box 0: the per-package table, and the 10 least-covered files of each workspace.
9. One line: no market data was read, fetched or written; no link or secret was used.

## 8. Final message

- Your SUMMARY lines.
- The result branch and the sha of your last pushed commit.
- The wall time.
- Any stop or failure, in plain words.
- At most 8 lines.

## 9. Expected wall (ESTIMATE)

| step | boxes 1 to 8 | box 0 |
|---|---|---|
| setup: toolchain, tool install | 5-10 min | 4-8 min |
| baselines: 4 unmutated sets, full debug builds of both workspaces | 6-12 min | 6-12 min |
| A1: 283 mutants at 4-8 s each | 19-38 min | - |
| A2: 58 mutants at 6-15 s each | 6-15 min | - |
| B2: 123 mutants at 3-6 s each | 6-12 min | - |
| B1: 481 mutants at 8-20 s each | 64-160 min | - |
| each cargo-mutants run's own baseline and first build (4 runs) | 15-30 min | - |
| coverage: 2 instrumented builds and test runs, reports | - | 10-20 min |
| **total** | **about 2 to 4.5 h** | **about 20 to 40 min** |

- The per-mutant times are wall per mutant with `--jobs 2` on 4 vCPU.
- Their basis is MEASURED on 16 Nov:
  - the last wave's cloud shards: 522 spike-selling mutants in 29 min 24 s (shard 5), and 501 in 33 min 37 s (shard 29),
    so 3.4 to 4.0 s a mutant;
  - the Mac's dev rebuild of `sniper_host` after a source change: 23 s (lib test target only, Mac under heavy load).
- B1 rebuilds the largest crate and links 4 test targets for every mutant, so it gets the widest band.

## 10. Changes from the 16 Nov wave (and why)

1. **Two workspaces, four sets per box**, each run with `--shard k/8`. The tool spans two cargo workspaces, and one
   cargo-mutants run covers one workspace.
2. **Scope is fixed in advance** (section 4: packages, files, targets, skips). In the last wave, boxes chose their own
   exclusions and their results did not line up. Here a baseline mismatch stops the box.
3. **The binary drivers are not mutated** (no test can reach them), and `costs` only in `option_clock.rs` (the rest was
   mutated on 16 Nov).
4. **Target selection** in A1 and B1 (section 6): the same tests, fewer builds.
5. **`TMPDIR` plus one symlink outside the checkout** (section 6), for the host's outside path dependency.
6. **`CARGO_PROFILE_DEV_DEBUG=0`**, for speed only.
7. **The tool versions are pinned by `--version`** (the last wave took "the newest", which was 27.1.0).
8. **Progress pushes every 15 minutes at most**, and a resumable driver, so that a lost VM loses at most 15 minutes of
   lists.
9. **Results go to `results/idxopt-mutants/shard-<k>-of-8/<set>/`**, with the same files as before
   (`outcomes.json`, `caught`, `missed`, `timeout` and `unviable` `.txt`), plus SUMMARY lines.

## 11. box.sh

```bash box.sh
#!/usr/bin/env bash
# box.sh: the IDXOPT cloud job driver (JOB-IDXOPT.md section 11). Extract it verbatim, never edit it.
# Usage: bash ~/box.sh STEP JOB
#   STEP = all | setup | baseline | run | progress | status
#   JOB  = cov (box 0, coverage) or 0..7 (boxes 1-8, cargo-mutants shard JOB of 8)
# Code only. It reads the checkout ~/idxopt and writes only ~/idxopt/results/<your folder>,
# ~/logs, ~/out-*, ~/tgt-*, ~/cmt, ~/SRC_SHA and ~/MANIFEST-idxopt.txt. It never fetches data.
set -u
export PATH="$HOME/.cargo/bin:$PATH"
SRC="$HOME/idxopt"
HOST="$SRC/sniper-host-v3-idx-w4r3"
IDX="$HOST/pinned/w4m/opening5s-5b3r70"
LOGS="$HOME/logs"
EXP_FILES=366
EXP_MANIFEST=e082593fbc5101f8691bc68b08846e44bc5a0388be924c91d79f75416d4cff30
CM_VER=27.1.0
COV_VER=0.9.1
RUSTC_WANT="rustc 1.97.1 "
# Tests skipped by exact name (JOB-IDXOPT.md section 4): each needs a file this branch does not hold.
A1_SKIP=(--skip price::tests::d3_per_leg_slices_by_the_one_order_maximum)
B1_SKIP=(--skip the_data_rows_load_fail_closed --skip the_standing_labels_and_trade_count_words
         --skip sn_b3_sniper_q_reads_the_352_byte_store_with_its_own_width --skip sn20_the_exit_sets_v2_and_v3_fail_closed)
B2_SKIP=(--skip tests_wf::sweep_eq_against_common_wf_acc --skip tests_wf::sweep_eq_miss_and_n_against_common_wf)
# Expected baseline sums, "passed failed ignored filtered" (EXACT, section 4).
EXP_A1="73 0 0 1"; EXP_A2="440 0 2 0"; EXP_B2="24 0 0 2"; EXP_B1="73 0 0 4"

now() { date -u '+%Y-%m-%d %H:%M:%S UTC'; }
say() { echo "BOX $(now) $*"; }
die() { echo "BOX STOP $(now) $*"; exit 1; }
pfail() { echo "PROGRESS FAIL $(now) $*"; exit 1; }
srcsha() { cat "$HOME/SRC_SHA" 2>/dev/null || echo unknown; }
rustc_v() { (cd "$HOST" && rustc --version) 2>/dev/null; }
lines() { if [ -f "$1" ]; then wc -l < "$1" | tr -d ' '; else echo 0; fi; }

step="${1:-}"; job="${2:-}"
case "$job" in
  cov) RB="cloud-results/idxopt-coverage"; RD="$SRC/results/idxopt-coverage" ;;
  0|1|2|3|4|5|6|7) RB="cloud-results/idxopt-mutants-$job-of-8"; RD="$SRC/results/idxopt-mutants/shard-$job-of-8" ;;
  *) echo "usage: bash ~/box.sh STEP JOB  (STEP = all|setup|baseline|run|progress|status; JOB = cov or 0..7)"; exit 2 ;;
esac
mkdir -p "$LOGS"

step_setup() {
  cd "$SRC" || die "no checkout at $SRC (section 5, step 1)"
  [ -s "$HOME/SRC_SHA" ] || git rev-parse HEAD > "$HOME/SRC_SHA" || die "git rev-parse failed"
  LC_ALL=C find . -type f ! -path './.git/*' ! -path './results/*' ! -path '*/target/*' ! -path '*/mutants.out*' \
      ! -name JOB-IDXOPT.md | LC_ALL=C sort | xargs sha256sum > "$HOME/MANIFEST-idxopt.txt" || die "manifest failed"
  local n m v d cur
  n=$(lines "$HOME/MANIFEST-idxopt.txt"); m=$(sha256sum "$HOME/MANIFEST-idxopt.txt" | cut -c1-64)
  if [ "$n" != "$EXP_FILES" ] || [ "$m" != "$EXP_MANIFEST" ]; then
    die "checkout differs: $n files (want $EXP_FILES), manifest $m (want $EXP_MANIFEST)"
  fi
  say "manifest OK: $n files, sha256 $m; source commit $(srcsha)"
  if ! command -v rustup > /dev/null 2>&1; then
    say "installing rustup (the toolchain itself comes from rust-toolchain.toml)"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none \
      > "$LOGS/rustup.log" 2>&1 || die "rustup install failed ($LOGS/rustup.log)"
  fi
  for d in "$HOST" "$IDX"; do
    v=$(cd "$d" && rustc --version 2>> "$LOGS/rustup.log") || die "rustc failed in $d ($LOGS/rustup.log)"
    case "$v" in "$RUSTC_WANT"*) ;; *) die "rustc in $d is '$v', want 1.97.1 (rust-toolchain.toml)" ;; esac
  done
  say "toolchain OK: $(rustc_v)"
  if [ "$job" = cov ]; then
    if [ "$(cargo llvm-cov --version 2> /dev/null)" != "cargo-llvm-cov $COV_VER" ]; then
      say "installing cargo-llvm-cov $COV_VER"
      (cd "$HOST" && cargo install cargo-llvm-cov --locked --version "$COV_VER") > "$LOGS/install.log" 2>&1 \
        || die "cargo-llvm-cov $COV_VER install failed ($LOGS/install.log)"
    fi
    say "tool OK: $(cargo llvm-cov --version)"
  else
    if [ "$(cargo mutants --version 2> /dev/null)" != "cargo-mutants $CM_VER" ]; then
      say "installing cargo-mutants $CM_VER"
      (cd "$HOST" && cargo install cargo-mutants --locked --version "$CM_VER") > "$LOGS/install.log" 2>&1 \
        || die "cargo-mutants $CM_VER install failed ($LOGS/install.log)"
    fi
    say "tool OK: $(cargo mutants --version)"
  fi
  # The result branch: stay on it if checked out; resume on it if it exists on origin; else make it from the source commit.
  cur=$(git rev-parse --abbrev-ref HEAD)
  if [ "$cur" != "$RB" ]; then
    if git show-ref --verify --quiet "refs/heads/$RB"; then
      git checkout -q "$RB" || die "cannot check out the local $RB"
    elif git ls-remote --exit-code --heads origin "$RB" > /dev/null 2>&1; then
      git fetch -q origin "refs/heads/$RB" && git checkout -q -b "$RB" FETCH_HEAD || die "cannot check out the existing $RB"
      say "resuming on the existing $RB"
    else
      git checkout -q -b "$RB" || die "cannot create $RB"
    fi
  fi
  mkdir -p "$RD"
  {
    echo "# FACTS $(now)"
    echo "job $job; result branch $RB; source branch cloud/idxopt-v3-20261116; source commit $(srcsha)"
    echo "manifest $n files, sha256 $m (expected $EXP_MANIFEST)"
    echo "uname $(uname -srm); nproc $(nproc 2> /dev/null)"
    echo "mem_total_kib $(awk '/MemTotal/ {print $2}' /proc/meminfo 2> /dev/null); cgroup memory.max $(cat /sys/fs/cgroup/memory.max 2> /dev/null || echo n/a)"
    echo "disk_free_kib $(df -Pk "$HOME" | awk 'NR == 2 {print $4}')"
    echo "$(rustc_v); $(cd "$HOST" && cargo --version)"
    echo "tools: $(cargo mutants --version 2> /dev/null) $(cargo llvm-cov --version 2> /dev/null)"
  } >> "$RD/FACTS.txt"
  say "SETUP OK"
}

# bl NAME DIR ARGS...: one baseline cargo test (unmutated, in place, own target dir); prints "rc|passed failed ignored filtered".
bl() {
  local name=$1 dir=$2; shift 2
  (cd "$dir" && env -u CARGO_TARGET_DIR CARGO_PROFILE_DEV_DEBUG=0 cargo test --locked "$@") > "$LOGS/baseline-$name.log" 2>&1
  local rc=$?
  cp "$LOGS/baseline-$name.log" "$RD/baseline-$name.log"
  echo "$rc|$(grep -E '^test result:' "$LOGS/baseline-$name.log" | awk '{p += $4; f += $6; i += $8; x += $12} END {printf "%d %d %d %d", p, f, i, x}')"
}

step_baseline() {
  mkdir -p "$RD"
  local s r rc got exp bad=0
  for s in A1 A2 B2 B1; do
    case $s in
      A1) exp=$EXP_A1; r=$(bl A1 "$IDX" --target-dir "$HOME/tgt-A" -p opt_pos -p idx_cal -p idx_f1 --lib --no-fail-fast -- --exact "${A1_SKIP[@]}") ;;
      A2) exp=$EXP_A2; r=$(bl A2 "$IDX" --target-dir "$HOME/tgt-A" -p costs --no-fail-fast) ;;
      B2) exp=$EXP_B2; r=$(bl B2 "$HOST" --target-dir "$HOME/tgt-B" -p sniper_kernel -p wf_sweep --no-fail-fast -- --exact "${B2_SKIP[@]}") ;;
      B1) exp=$EXP_B1; r=$(bl B1 "$HOST" --target-dir "$HOME/tgt-B" -p sniper_host --lib --test matrix --test no_cut_at --test fast \
                           --no-fail-fast -- --exact "${B1_SKIP[@]}") ;;
    esac
    rc=${r%%|*}; got=${r#*|}
    echo "BASELINE set=$s rc=$rc passed_failed_ignored_filtered=\"$got\" expected=\"$exp\" $(now)" >> "$RD/BASELINE.txt"
    if [ "$rc" != 0 ] || [ "$got" != "$exp" ]; then bad=1; say "baseline $s MISMATCH: rc $rc, got '$got', expected '$exp'"
    else say "baseline $s OK: $got"; fi
  done
  [ "$bad" = 0 ] || die "BASELINE MISMATCH: see BASELINE.txt and baseline-*.log in the result folder (section 5, step 6)"
  touch "$RD/BASELINE-OK"
  say "BASELINE OK"
}

# set_args SET: the cargo-mutants arguments of one set (section 6).
set_args() {
  case $1 in
    A1) dir=$IDX; skip=("${A1_SKIP[@]}")
        args=(-p opt_pos -p idx_cal -p idx_f1 -e 'crates/opt_pos/src/bin_*.rs' -e 'crates/idx_cal/src/main.rs'
              -e 'crates/idx_cal/src/bin_served.rs' -e 'crates/idx_f1/src/main.rs' --cargo-arg=--lib) ;;
    A2) dir=$IDX; skip=(); args=(-p costs -f 'crates/costs/src/option_clock.rs') ;;
    B2) dir=$HOST; skip=("${B2_SKIP[@]}"); args=(-p sniper_kernel -p wf_sweep) ;;
    B1) dir=$HOST; skip=("${B1_SKIP[@]}")
        args=(-p sniper_host -e 'crates/sniper_host/src/bin/**' --cargo-arg=--lib --cargo-arg=--test=matrix
              --cargo-arg=--test=no_cut_at --cargo-arg=--test=fast) ;;
  esac
}

collect() {
  local s=$1 rc=$2 w=$3 o="$HOME/out-$1/mutants.out" d="$RD/$1" f c m t u n
  mkdir -p "$d"
  for f in caught.txt missed.txt timeout.txt unviable.txt outcomes.json; do [ -f "$o/$f" ] && cp "$o/$f" "$d/$f"; done
  for f in caught.txt missed.txt timeout.txt unviable.txt; do [ -f "$d/$f" ] || : > "$d/$f"; done
  tail -n 300 "$LOGS/mut-$s.log" > "$d/mutants-tail.log"
  c=$(lines "$d/caught.txt"); m=$(lines "$d/missed.txt"); t=$(lines "$d/timeout.txt"); u=$(lines "$d/unviable.txt")
  n=$(jq length "$o/mutants.json" 2> /dev/null || echo unknown)
  echo "SUMMARY idxopt-mutants shard=$job/8 set=$s mutants=$n caught=$c missed=$m timeout=$t unviable=$u wall_s=$w rc=$rc commit=$(srcsha) cargo_mutants=$CM_VER rustc=$(rustc_v | awk '{print $2}')" >> "$RD/SUMMARY.txt"
}

total() {
  awk -v K="$job/8" -v C="$(srcsha)" -v V="$CM_VER" '
    /^SUMMARY idxopt-mutants / && / set=(A1|A2|B1|B2) / {
      for (i = 1; i <= NF; i++) { split($i, kv, "="); if (kv[1] == "set") s = kv[2] }
      line[s] = $0 }
    END {
      for (s in line) { n = split(line[s], f, " ")
        for (i = 1; i <= n; i++) { split(f[i], kv, "=")
          if (kv[1] ~ /^(mutants|caught|missed|timeout|unviable|wall_s)$/) tot[kv[1]] += kv[2] } }
      printf "SUMMARY idxopt-mutants shard=%s set=ALL mutants=%d caught=%d missed=%d timeout=%d unviable=%d wall_s=%d commit=%s cargo_mutants=%s\n",
        K, tot["mutants"], tot["caught"], tot["missed"], tot["timeout"], tot["unviable"], tot["wall_s"], C, V }' "$RD/SUMMARY.txt" > "$LOGS/total.tmp"
  grep -v ' set=ALL ' "$RD/SUMMARY.txt" > "$LOGS/summary.tmp"
  cat "$LOGS/summary.tmp" "$LOGS/total.tmp" > "$RD/SUMMARY.txt"
}

step_mut() {
  local s rc t0 t1 dir args skip
  mkdir -p "$RD" "$HOME/cmt"
  # cargo-mutants copies only the workspace folder into TMPDIR and rewrites only its root Cargo.toml: the host's
  # path ../../../common-fill-v2 must resolve next to that copy (section 6).
  [ -e "$HOME/cmt/common-fill-v2" ] || ln -s "$SRC/common-fill-v2" "$HOME/cmt/common-fill-v2" || die "symlink failed"
  for s in A1 A2 B2 B1; do
    if [ -f "$RD/$s/DONE" ]; then say "set $s already DONE"; continue; fi
    set_args "$s"
    say "set $s start: shard $job/8"
    t0=$(date -u +%s)
    (cd "$dir" && env -u CARGO_TARGET_DIR TMPDIR="$HOME/cmt" CARGO_PROFILE_DEV_DEBUG=0 \
      cargo mutants --shard "$job/8" --jobs 2 --cargo-arg=--locked "${args[@]}" -o "$HOME/out-$s" \
      -- -- --exact ${skip[@]+"${skip[@]}"}) > "$LOGS/mut-$s.log" 2>&1
    rc=$?
    t1=$(date -u +%s)
    collect "$s" "$rc" "$((t1 - t0))"
    case "$rc" in
      0|2|3) touch "$RD/$s/DONE"; say "set $s DONE: rc $rc, $(tail -n 1 "$RD/SUMMARY.txt" | cut -d' ' -f4-9)" ;;
      *) die "set $s: cargo mutants rc $rc (see $s/mutants-tail.log in the result folder)" ;;
    esac
  done
  total
  say "BOX DONE: $(tail -n 1 "$RD/SUMMARY.txt" | cut -d' ' -f4-9)"
}

covsum() {
  local set=$1 rc=$2 w=$3 f="$RD/$1/cov-summary.txt" runs
  runs=$(grep -E '^rc (A1|A2|B1|B2) ' "$RD/$set/cov.log" | awk '{printf "%s%s:%s", (NR > 1 ? "," : ""), $2, $3}')
  [ -s "$f" ] || die "no coverage report for $set (rc $rc; see $set/cov.log in the result folder)"
  awk 'NR > 2 && NF >= 13 && $1 != "TOTAL" && $1 !~ /^-+$/ {
         p = $1; sub(/^crates\//, "", p); split(p, q, "/"); k = q[1]
         R[k] += $2; MR[k] += $3; F[k] += $5; MF[k] += $6; L[k] += $8; ML[k] += $9; N[k]++ }
       END { for (k in R) printf "PACKAGE %s files=%d regions=%.2f%% functions=%.2f%% lines=%.2f%% (%d/%d)\n", k, N[k],
               (R[k] ? 100 * (R[k] - MR[k]) / R[k] : 0), (F[k] ? 100 * (F[k] - MF[k]) / F[k] : 0),
               (L[k] ? 100 * (L[k] - ML[k]) / L[k] : 0), L[k] - ML[k], L[k] }' "$f" | sort > "$RD/$set/PACKAGES.txt"
  grep -E 'option_clock\.rs' "$f" > "$RD/$set/option_clock-row.txt" 2> /dev/null || true
  awk -v S="$set" -v W="$w" -v RC="$rc" -v RUNS="$runs" -v C="$(srcsha)" -v V="$COV_VER" '$1 == "TOTAL" {
         printf "SUMMARY idxopt-coverage set=%s lines=%s (%d/%d) functions=%s regions=%s branches=not-measured wall_s=%s rc=%s runs=%s commit=%s cargo_llvm_cov=%s\n",
           S, $10, $8 - $9, $8, $7, $4, W, RC, RUNS, C, V }' "$f" >> "$RD/SUMMARY.txt"
}

step_cov() {
  local t0 t1 rc
  mkdir -p "$RD/A-idxopt" "$RD/B-host"
  say "coverage A-idxopt start"
  t0=$(date -u +%s)
  (cd "$IDX" && cargo llvm-cov clean --workspace && {
     cargo llvm-cov --no-report --locked -p opt_pos -p idx_cal -p idx_f1 --lib --no-fail-fast -- --exact "${A1_SKIP[@]}"; echo "rc A1 $?"
     cargo llvm-cov --no-report --locked -p costs --no-fail-fast; echo "rc A2 $?"; } \
   && cargo llvm-cov report --lcov --output-path "$RD/A-idxopt/lcov.info" \
   && cargo llvm-cov report --summary-only > "$RD/A-idxopt/cov-summary.txt") > "$LOGS/cov-A.log" 2>&1
  rc=$?; t1=$(date -u +%s)
  cp "$LOGS/cov-A.log" "$RD/A-idxopt/cov.log"
  covsum A-idxopt "$rc" "$((t1 - t0))"
  say "coverage A-idxopt done: $(tail -n 1 "$RD/SUMMARY.txt" | cut -d' ' -f3-8)"
  say "coverage B-host start"
  t0=$(date -u +%s)
  (cd "$HOST" && cargo llvm-cov clean --workspace && {
     cargo llvm-cov --no-report --locked -p sniper_host --lib --test matrix --test no_cut_at --test fast --no-fail-fast \
       -- --exact "${B1_SKIP[@]}"; echo "rc B1 $?"
     cargo llvm-cov --no-report --locked -p sniper_kernel -p wf_sweep --no-fail-fast -- --exact "${B2_SKIP[@]}"; echo "rc B2 $?"; } \
   && cargo llvm-cov report --lcov --output-path "$RD/B-host/lcov.info" \
   && cargo llvm-cov report --summary-only > "$RD/B-host/cov-summary.txt") > "$LOGS/cov-B.log" 2>&1
  rc=$?; t1=$(date -u +%s)
  cp "$LOGS/cov-B.log" "$RD/B-host/cov.log"
  covsum B-host "$rc" "$((t1 - t0))"
  say "coverage B-host done: $(tail -n 1 "$RD/SUMMARY.txt" | cut -d' ' -f3-8)"
  say "BOX DONE"
}

alive() {
  local p
  p=$(cat "$LOGS/box-$job.pid" 2> /dev/null) || return 1
  kill -0 "$p" 2> /dev/null && ps -p "$p" -o args= 2> /dev/null | grep -q "box.sh all $job"
}

step_status() {
  local s o
  if alive; then printf 'RUNNING | '; else printf 'NOT RUNNING | '; fi
  printf '%s | ' "$(grep -E '^BOX ' "$LOGS/box.log" 2> /dev/null | tail -n 1 | cut -c1-200)"
  if [ "$job" != cov ]; then
    for s in A1 A2 B2 B1; do
      o="$HOME/out-$s/mutants.out"
      if [ -f "$RD/$s/DONE" ]; then printf '%s done | ' "$s"
      elif [ -d "$o" ]; then printf '%s %s of %s | ' "$s" "$(cat "$o/caught.txt" "$o/missed.txt" "$o/timeout.txt" "$o/unviable.txt" 2> /dev/null | wc -l | tr -d ' ')" "$(jq length "$o/mutants.json" 2> /dev/null || echo '?')"
      fi
    done
  fi
  echo "$(now)"
}

step_progress() {
  local line s o f out hit
  cd "$SRC" || pfail "no checkout at $SRC"
  if [ "$(git rev-parse --abbrev-ref HEAD)" != "$RB" ]; then echo "PROGRESS $(now) not on $RB yet (setup not finished): nothing to push"; return 0; fi
  mkdir -p "$RD"
  line="$(now) $(grep -E '^BOX ' "$LOGS/box.log" 2> /dev/null | tail -n 1 | cut -c1-160)"
  if [ "$job" != cov ]; then
    for s in A1 A2 B2 B1; do
      o="$HOME/out-$s/mutants.out"
      if [ -f "$RD/$s/DONE" ]; then line="$line | $s done"
      elif [ -d "$o" ]; then
        mkdir -p "$RD/$s"
        for f in caught.txt missed.txt timeout.txt unviable.txt; do [ -f "$o/$f" ] && cp "$o/$f" "$RD/$s/$f"; done
        line="$line | $s $(cat "$o/caught.txt" "$o/missed.txt" "$o/timeout.txt" "$o/unviable.txt" 2> /dev/null | wc -l | tr -d ' ') of $(jq length "$o/mutants.json" 2> /dev/null || echo '?')"
      else line="$line | $s not started"; fi
    done
  fi
  echo "- $line" >> "$RD/PROGRESS.md"
  [ -f "$LOGS/box.log" ] && cp "$LOGS/box.log" "$RD/box.log"
  git add -A -- results || pfail "git add failed"
  out=$(git diff --cached --name-only | grep -v '^results/' || true)
  [ -z "$out" ] || pfail "staged files outside results/: $out (nothing pushed)"
  hit=$(git grep --cached -I -i -l -e "x-am""z-" -e "awsaccess""keyid" -- results 2> /dev/null || true)
  [ -z "$hit" ] || pfail "a presigned-link marker in: $hit (nothing pushed)"
  if git diff --cached --quiet; then echo "PROGRESS $(now) nothing new to push"; return 0; fi
  git commit -q -m "idxopt $job: results $(date -u '+%Y-%m-%dT%H:%M:%SZ')" || pfail "commit failed"
  git push -q origin "HEAD:refs/heads/$RB" || pfail "push to $RB failed (never force; report it)"
  echo "PUSHED $(now) $(git rev-parse --short HEAD) to $RB: $line"
}

case "$step" in
  all) if alive; then echo "BOX $(now) already running (pid $(cat "$LOGS/box-$job.pid")): not started twice"; exit 0; fi
       echo $$ > "$LOGS/box-$job.pid"
       step_setup; [ -f "$RD/BASELINE-OK" ] || step_baseline
       if [ "$job" = cov ]; then step_cov; else step_mut; fi ;;
  setup) step_setup ;;
  baseline) step_baseline ;;
  run) if [ "$job" = cov ]; then step_cov; else step_mut; fi ;;
  progress) step_progress ;;
  status) step_status ;;
  *) echo "usage: bash ~/box.sh STEP JOB  (STEP = all|setup|baseline|run|progress|status; JOB = cov or 0..7)"; exit 2 ;;
esac
```

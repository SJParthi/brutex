# Mutation slice 4/30 of engine v3.4: notes

Box k=4 of n=30. Labels as in JOB-MUTANTS.md: EXACT = a count, MEASURED = a number from a named run on this box.
Nothing here is a trading result.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- Memory: MemTotal 16473776 kB. cgroup v1: `/sys/fs/cgroup/memory.max` does not exist;
  `/sys/fs/cgroup/memory/memory.limit_in_bytes` is 9223372036854771712, i.e. no limit.
- Disk: 252G volume, 30G free at set-up, 28G free after the run.
- The READY marker branch appeared at 2026-11-16T15:51:44Z (polled every 60 s from 14:53Z). Box start (step 1b):
  2026-11-16T15:53:05Z.

## Code, toolchain, case and command

- Checkout `~/mut` of `cloud/spikefade-v34int-20261116` at 7a5428d30bdb8a5e0fec427848178c82907de2fb: HEAD OK.
  `JOB-MUTANTS.md` sha256 078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74, as READY.txt names.
- Every JOB block was copied verbatim out of `JOB-MUTANTS.md` and run with bash. The only edit was the first line of
  section 1b (`K=4; SHA=<the commit>`). No code was changed; the branch's commits add only files under this folder.
- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`.
  `rustup toolchain install 1.97.1 ...` ended exit 0 (the toolchain was already synced on this box).
- `cargo-mutants 27.1.0`, from `cargo install cargo-mutants --locked --version 27.1.0` (exit 0).
- Case **A**: the slice holds no `core` mutant, so `case-args.txt` is empty.
- The exact command (`run-main.sh`):
  `cargo mutants --workspace --shard 4/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact` followed by
  `--skip <name>` for each of the 17 names in `skips.txt` (sha256 0cd96e3e2906f52103be4c894a44beec8c32170a5a8828ea7e2ea302c41fe557).

## list_check

- **MATCH** (EXACT): 24,429 mutants, sha256 3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a;
  slice 4 is 815 mutants, sha256 0c8636dbdb6aa61ea9f3a25d4c9091db8166157f5447f2f378a4f465f1c2d243, as the table says.
- All 815 are in `spikefade`: `charges.rs`, `csvgz.rs`, `cube.rs`, `margin.rs`, `mtable.rs`.

## Baseline (step 5, MEASURED)

| section | rc | wall ms | what |
|---|---|---|---|
| B1 | 101 | 19229 | exactly the 6 expected `could not compile` errors, all `core`: lib test, `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph` (`include_str!` of app-repository files not on this branch) |
| B2a | 101 | 6445 | spikefade unit tests 204 passed, 12 failed, 1 ignored; every costs and greeks target passed |
| B2b | 101 | 141 | `cost_invariants` 0 passed, 5 failed; `bound_probe` 2, `lint` 2, `zerodha_index` 5 passed |
| B3a | 0 | 6462 | 0 failed (spikefade: 204 passed, 12 filtered out by the skips) |
| B3b | 0 | 60 | 0 failed (`cost_invariants`: 5 filtered out) |

- B2 failed 17 tests (12 engine, 5 core), every one of them in `skips.txt`. None outside it.
- The last B3 pair's failure count: 0.

## ENV-EXTRA

- None. No test outside `skips.txt` failed, so `baseline.sh -r2` was not needed.

## Main run (step 6)

- Start 2026-11-16T15:55:34Z, end 2026-11-16T16:59:07Z: 63 min (MEASURED).
- cargo-mutants' own baseline: ok, 16 s build + 4 s test. Auto-set test timeout: 25 s.
- **Exit 3** (some mutants timed out): "go on" per step 6.
- EXACT: 815 tested: caught 459, missed 335, timeout 12, unviable 9.
- Progress commits: f59ff17 (163 tested), 68e6906 (338), ee85f7a (514), 32ee674 (671), 2d1327c (815).

## Wall-clock re-check (step 8)

- `find-timing.sh`: **28** timing-only catches. In every one of the 28 logs the only FAILED test is
  `split18::tests::v32rb_split_o1_gates` (the same gate as the last wave's slice 26 finding).
- `run-recheck.sh` (`--jobs 1`): its selection listed 37 = the 28 names plus 9 extra
  `delete field ... from struct ... expression` mutants of `mtable.rs` (lines 297, 307 and 619), which cargo-mutants
  27.1.0 does not filter with `-F`, as the last wave found. Its baseline: ok, 19 s build + 4 s test; auto-set test
  timeout 24 s. **Exit 2.**
- Of the 28 names: **0 caught, 28 missed** at `--jobs 1`. Every timing-only catch was the wall-clock gate failing
  under `--jobs 2` load, not the mutant being detected.
- The 9 extras: 8 caught, 1 missed (`mtable.rs:307:30: delete field fallback from struct MarginAt expression in
  exact_at`). They belong to the re-check only.
- The official files stay as cargo-mutants wrote them. Corrected per step 8:
  - corrected caught = 459 - 28 = **431**
  - corrected missed = 335 + 28 = **363**
  - timeout 12 and unviable 9 are unchanged.

## Anything odd

- Missed mutants by file, official (corrected): `cube.rs` 220 (239), `mtable.rs` 53 (55), `margin.rs` 30 (32),
  `csvgz.rs` 17 (19), `charges.rs` 15 (18). Most of `cube.rs`'s misses are in `build`.
- The 12 timeouts are all loop-arithmetic mutants (`+=` to `*=`/`-=`, `%` to `/`/`+`, `&&` to `||`) in `csvgz.rs`
  (`split`, `paisa`) and `cube.rs` (`pad`, `Rd::align`), cut by the 25 s test timeout.
- The 9 unviable are as cargo-mutants reported them: `Default::default()` replacements in `csvgz.rs`, `cube.rs` and
  `mtable.rs`, and three `&&` to `||` replacements in `cube.rs` (`load_m`, `load_sec_m`, `decode_cube`).
- Limit (from the JOB): a mutant that only a skipped test would catch is counted as missed.

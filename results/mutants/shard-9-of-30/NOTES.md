# Mutation slice 9/30 of engine v3.4: notes

Box k=9 of 30, wave 2 (16 Nov 2026). Code only: no market data was fetched, read or written. Labels as in
JOB-MUTANTS.md: EXACT = a count, MEASURED = a number from a named run on this box.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- Memory: `MemTotal: 16473776 kB` (about 15.7 GiB).
- Cgroup limit: `/sys/fs/cgroup/memory.max` does not exist on this box (it uses the cgroup v1 layout), so the
  step 2 `cat` printed nothing. The v1 file `/sys/fs/cgroup/memory/memory.limit_in_bytes` reads
  `9223372036854771712`, that is, no memory limit.
- Disk: 30 GB free at the start (`df`), and at least 28 GB stayed free during the run.

## Versions, case and command

- The READY marker branch `cloud/ready-v34-mutants-20261116` appeared at 15:51:38Z, after polling every 60 s from
  14:53:51Z. Its READY.txt named code branch `cloud/spikefade-v34int-20261116`, commit
  `7a5428d30bdb8a5e0fec427848178c82907de2fb` (the branch tip), and `JOB-MUTANTS.md`, whose sha256 here is
  `078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74`, as READY.txt states.
- Step 2 printed `HEAD OK`, and the box started at `2026-11-16T15:53:21Z` (`~/box.start`).
- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`. The rustup step reported
  the 1.97.1 toolchain `unchanged`, with every listed component already up to date.
- `cargo-mutants 27.1.0`, installed with `cargo install cargo-mutants --locked --version 27.1.0` (exit 0).
- **Case A.** The slice holds 815 spikefade mutants and no `core` mutant, so `case-args.txt` is empty.
- The exact command (`~/run-main.sh`), where the 17 skips are the names in `skips.txt`:
  `cargo mutants --workspace --shard 9/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip <name> ...`

## List check

- `list_check MATCH` (EXACT). The full list has 24,429 mutants, sha256
  `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`. Slice 9 has 815, sha256
  `cff3dbdf73750d195da07a5c75655ee8608d34909105118c24c14a48aa7e2a43`. Both equal the table.
- The slice by file: `crates/spikefade/src/kern.rs` 509 (from line 563), then `crates/spikefade/src/menu.rs` 306
  (to line 728).

## Baseline (`baseline.log`, MEASURED)

| section | rc | wall | what it showed |
|---|---|---|---|
| B1 `cargo build --locked --workspace --tests --keep-going` | 101 | 20,563 ms | exactly the 6 expected `could not compile` errors, all `core`: its lib test and `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph` (each `include_str!`s app-repository files this branch does not hold) |
| B2a `cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade` | 101 | 7,535 ms | spikefade's unit tests: 204 passed, 12 failed, 1 ignored. Every costs and greeks target passed |
| B2b `cargo test --locked --no-fail-fast -p core` with the 4 targets | 101 | 163 ms | `cost_invariants`: 0 passed, 5 failed. `bound_probe`, `lint` and `zerodha_index` passed |
| B3a, B2a's command with `-- --exact` and the 17 skips | 0 | 7,108 ms | spikefade: 204 passed, 0 failed, 1 ignored, 12 filtered out |
| B3b, B2b's command with `-- --exact` and the 17 skips | 0 | 69 ms | 0 failed, with `cost_invariants`' 5 filtered out |

- B2 failed 17 tests, all 17 of them in `skips.txt`, and the last B3 pair has 0 failures.
- The B2 causes are the ones in the JOB's table:
  - the 5 `menu::tests::*` fail with `menu_head: missing`;
  - `tests33s::v33_search1d_sets_load_and_count` fails on `.../crates/spikefade/data/EXITS-DYN-MENU-v1.tsv: No such file or directory`;
  - `tests3::sf3_ttp_replaces_tp_close` fails with `No such file or directory`;
  - the 3 `tests34i::*` fail on a real table at an absolute path of the operator's Mac, a missing file, and
    `assertion failed: crate::f7::is_manifest(&p)`;
  - 4 of the `core` `cost_invariants` names fail on `docs/04-invariants.md must be readable ... No such file or
    directory`, and `every_crate_carries_a_ratio_bench` fails on no `benches/ratio.rs` for `spikefade`.
- **KNOWN cause.** `tests34s::root_digest_same_for_every_form` and `tests34s::sfz_roundtrip_all_rows` fail because,
  with every kit switch off, the engine calls zstd at the Mac path `/opt/homebrew/bin/zstd` (kit.rs:470). The error
  here is `zstd (/opt/homebrew/bin/zstd) ["-3", "-q", "-c"]: No such file or directory (os error 2)`. The fix is not in
  v3.4.

## ENV-EXTRA

- None. Every B2 failure was already in `skips.txt`. `skips.txt` is the JOB's 17 unchanged, and `baseline.sh -r2`
  was not needed.

## The main run (MEASURED)

- cargo-mutants' own baseline passed (`ok Unmutated baseline in 20s build + 4s test`) and auto-set the test timeout
  to 25 s.
- It ran from `2026-11-16T15:56:10Z` to `2026-11-16T17:06:47Z`. Its last line reads `815 mutants tested in 71m: 354
  missed, 419 caught, 30 unviable, 12 timeouts`.
- **Exit 3** (some timeouts), which step 6 accepts.
- By file (EXACT):

| file | caught | missed | timeout | unviable |
|---|---|---|---|---|
| kern.rs | 392 | 81 | 12 | 24 |
| menu.rs | 27 | 273 | 0 | 6 |

- All 12 timeouts are in kern.rs. Each replaces `+=` with `*=`, or `+` with `-` or `*`, at lines 1013 and 1015
  (`walk_account`), 1106 and 1108 (`walk_day`), and 1152 and 1154 (`walk_day_core`). Each ran the full 25 s; they
  were not examined further.
- Most of menu.rs is missed because the menu tests are among the skips: they need the EXITS-DYN tables that this
  branch leaves out. Under the JOB's stated limit, a mutant that only a skipped test would catch counts as missed.
- Progress commits were pushed at 16:09, 16:22, 16:36, 16:49, 17:02 and 17:07 (UTC).

## The wall-clock re-check (step 8, MEASURED)

- `find-timing.sh` found 48 caught mutants whose logs name only wall-clock tests as FAILED (`timing_only=48`): 21 in
  kern.rs and 27 in menu.rs.
- The name selection listed 51 mutants: the 48, plus 3 `delete field` mutants of menu.rs that 27.1.0 does not filter
  with `-F`. These are `sha` in `Menu::load`, and `sha` and `name` in `Sets::load` (`recheck_extra=3`). All 3 were
  missed, and they belong to the re-check only.
- The re-check's own baseline passed (21 s build + 5 s test) and auto-set the timeout to 28 s. It ran 7 minutes and
  exited **2**, which step 8 accepts.
- **Of the 48, 41 were MISSED at `--jobs 1`** (`recheck_missed=41`: 14 in kern.rs and all 27 in menu.rs), and 7 were
  still CAUGHT (`recheck_caught=7`).
- **Odd:** each of the 7 still caught again failed only one wall-clock test,
  `tests32r::v32r_o1_decision_at_n100_and_sampled_steps`. They are kern.rs `seg_of` (line 979), 5 `walk_accs` mutants
  (lines 1377, 1405 and 1407) and `step_weighted` (line 1436). So they are still timing-only catches, now at
  `--jobs 1`. This box did not determine whether the mutants really slow the code or the gate is noisy here.
- **Corrected, as in step 8:** caught = 419 - 41 = **378**, and missed = 354 + 41 = **395**. By file, kern.rs has
  378 caught and 95 missed; menu.rs has 0 caught and 300 missed. The official `caught.txt` and `missed.txt` stay as
  cargo-mutants wrote them.

## Exit codes

- `toolchain` 0, `install` 0, `bl` 0 (the job's own exit; the section rcs are in the baseline table), `main` 3,
  `recheck` 2.
- Step 7, the core follow-up, did not run because this is case A.

## Anything else

- The code branch does not ignore `target/`, so the baseline build left an untracked `target/` in the checkout.
  The scripts stage only `results/mutants/shard-9-of-30/`, so nothing else was committed. No code was changed.

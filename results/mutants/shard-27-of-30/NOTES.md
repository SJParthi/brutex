# Mutation slice 27/30 of engine v3.4: notes (the box writes these; see step 9)

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- Memory: `MemTotal: 16473776 kB` (about 15.7 GiB).
- Cgroup limit: this box has no cgroup v2 `/sys/fs/cgroup/memory.max`, so step 2's `cat` printed nothing. It runs
  cgroup v1, and this shell's v1 `memory.limit_in_bytes` reads 9223372036854771712, which means no limit.
- Disk at set-up: 30 GB free of 252 GB.

## Versions, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, the version `rust-toolchain.toml` pins.
  - The 1.97.1 toolchain and every requested component (rustfmt, clippy, llvm-tools, the wasm32-unknown-unknown target)
    were already on the box.
  - `rustup toolchain install` reported it unchanged and exited 0. In the same call rustup updated itself from 1.28.2 to
    1.29.3. That changes rustup only; the toolchain stayed 1.97.1.
- `cargo-mutants 27.1.0`, from `cargo install cargo-mutants --locked --version 27.1.0` (exit 0).
- Case A: all 815 mutants of the slice are in `spikefade`, so `case-args.txt` is empty.
- The exact command, run by `run-main.sh` from `~/mut`:

```
cargo mutants --workspace --shard 27/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts --skip menu::tests::sf3_menu_fail_closed --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry --skip menu::tests::sf3_distance_floor_counted --skip menu::tests::sf3_tp_theta_level_and_absence --skip tests33s::v33_search1d_sets_load_and_count --skip tests3::sf3_ttp_replaces_tp_close --skip tests34s::root_digest_same_for_every_form --skip tests34s::sfz_roundtrip_all_rows --skip every_declared_cost_invariant_reaches_a_bench --skip the_document_declares_the_five_operations_rule_4_names --skip every_measured_cost_invariant_is_declared --skip every_crate_carries_a_ratio_bench --skip the_reconciliation_is_reading_something --skip tests34i::v34i_f3_real_rows_spot --skip tests34i::v34i_f7_rules_tables_load_dated_and_minrange --skip tests34i::v34i_f7_opr_file_is_next_session_and_columns
```

## List check

- `list_check` MATCH. Both lists equal the table:
  - the whole list: 24,429 mutants, sha256 `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`;
  - slice 27: 815 mutants, sha256 `6e7f4358262841c27af4fc71185ab1ca26739ac775d67ff52cd68533774cd590`.
- All 815 mutants are `replace wick_compare -> (Vec<(u32, i64)>, u64, Vec<((i64, u32), (i64, i64, i64, i64))>) with ...`
  at `crates/spikefade/src/fix34.rs:1301:5`, as section 12 says for slices 19 to 28.

## Baseline (`baseline.log`)

| section | rc | wall ms | what it showed |
|---|---|---|---|
| B1 `cargo build --locked --workspace --tests --keep-going` | 101 | 21,208 | exactly the 6 expected `core` compile failures: the lib test, `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph` |
| B2a `cargo test` for costs, greeks, spikefade, no skips | 101 | 7,820 | spikefade 204 passed, 12 failed, 1 ignored; every costs and greeks target passed |
| B2b `cargo test` for core's 4 targets, no skips | 101 | 145 | `cost_invariants` 0 passed, 5 failed; `bound_probe`, `lint`, `zerodha_index` passed |
| B3a as B2a, with the 17 skips | 0 | 7,125 | 0 failed (spikefade 204 passed, 12 filtered out) |
| B3b as B2b, with the 17 skips | 0 | 71 | 0 failed (`cost_invariants` 5 filtered out) |

- B2 failed 17 distinct tests, and all 17 are in `skips.txt`. None is outside it.
- The step 5 count of failures in the last B3 pair printed 0.
- The workspace here is the 4 crates plus one local pinned crate, with no crates.io dependency. That is why B1 took 21 s.

## ENV-EXTRA

None. `skips.txt` is the JOB file's 17 names, unchanged.

## Exit codes

- `toolchain` 0, `install` 0, `bl` 0.
- `main` 0: "815 mutants tested in 69m: 815 caught".
  - cargo-mutants' own baseline: 19 s build + 5 s test. It auto-set the test timeout to 26 s.
  - The run went from 2026-11-16T15:55:50Z to 2026-11-16T17:04:53Z.
- Step 7, the core follow-up: not run, because this is case A.
- Step 8:
  - `find-timing.sh` found 0 timing-only catches.
  - `run-recheck.sh` exited 0 with "no timing-only catches", so there is no `recheck-jobs1/` folder.
- Outcome: caught 815, missed 0, timeout 0, unviable 0.

## Anything odd

- **A wall-clock test also failed, but never alone.** In 92 of the 815 caught logs, the wall-clock gate
  `split18::tests::v32rb_split_o1_gates` also shows FAILED under `--jobs 2`.
  - All 815 logs also show the value test `tests34::v34_cube_day_high_vs_bhavcopy_high` FAILED.
  - So none of the 92 is a timing-only catch, and step 8's 0 stands.
  - I checked this from `outcomes.json` and the per-mutant logs. All 815 logs were present, and each had at least one
    FAILED test line.
  - The logs themselves are not pushed. `final.sh` copies only `outcomes.json` and the 4 lists.
- **Timing.** The READY marker appeared 55 minutes after this box started polling for it. `box.start`
  (2026-11-16T15:52:56Z) was written after GO, so `box_wall_s` does not include that wait.
- **Progress commits.** Five were pushed during the main run (16:09, 16:22, 16:35, 16:48 and 17:01 UTC), each at most
  about 13.3 minutes after the one before. The first came 13.2 minutes after the run started.

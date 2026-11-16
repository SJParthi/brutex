# Mutation slice 22/30 of engine v3.4: notes (the box writes these; see step 9)

Labels as in the JOB: EXACT = a count, MEASURED = a number from this box's own runs. Nothing here is a trading result.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- Memory: MemTotal 16,473,776 kB. `/sys/fs/cgroup/memory.max` does not exist, because this box uses cgroup v1. The
  shell's v1 memory cgroup has a limit of 14,338,772,992 bytes (about 13.4 GiB), and its parent cgroups are unlimited.
  That cgroup's recorded peak after the run, page cache included, was 3,935,404,032 bytes (MEASURED).
- Disk: 30 GB available at set-up.

## Toolchain, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`. The pinned 1.97.1 toolchain was
  already present, so `rustup toolchain install` reported it unchanged and exited 0.
- `cargo-mutants 27.1.0`, from `cargo install cargo-mutants --locked --version 27.1.0` (exit 0).
- Code: branch `cloud/spikefade-v34int-20261116` at commit 7a5428d30bdb8a5e0fec427848178c82907de2fb, which was also
  the branch tip. Step 2 printed `HEAD OK`. The sha256 of JOB-MUTANTS.md equalled the one READY.txt names.
- Case A: the slice holds no `core` mutant, so there are no case arguments.
- The exact command, from `run-main.sh`, with the 17 names of `skips.txt` written out:

```
cargo mutants --workspace --shard 22/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact \
  --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts \
  --skip menu::tests::sf3_menu_fail_closed \
  --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry \
  --skip menu::tests::sf3_distance_floor_counted \
  --skip menu::tests::sf3_tp_theta_level_and_absence \
  --skip tests33s::v33_search1d_sets_load_and_count \
  --skip tests3::sf3_ttp_replaces_tp_close \
  --skip tests34s::root_digest_same_for_every_form \
  --skip tests34s::sfz_roundtrip_all_rows \
  --skip every_declared_cost_invariant_reaches_a_bench \
  --skip the_document_declares_the_five_operations_rule_4_names \
  --skip every_measured_cost_invariant_is_declared \
  --skip every_crate_carries_a_ratio_bench \
  --skip the_reconciliation_is_reading_something \
  --skip tests34i::v34i_f3_real_rows_spot \
  --skip tests34i::v34i_f7_rules_tables_load_dated_and_minrange \
  --skip tests34i::v34i_f7_opr_file_is_next_session_and_columns
```

## list_check

- `MATCH` (EXACT): the whole list is 24,429 mutants with sha256
  3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a, and slice 22 is 815 mutants with sha256
  c7dfadf0cd8d487c425bd58375833b8c2cd669d15920d5c87f10307daaf276f9. Both equal the JOB's table.
- All 815 mutants of the slice are at `crates/spikefade/src/fix34.rs:1301:5`, and every one replaces the return value
  of `wick_compare` (its 3-part tuple), as section 12 says for slices 19 to 28.

## Baseline (step 5)

| section | rc | wall ms | result |
|---|---|---|---|
| B1 `cargo build --locked --workspace --tests --keep-going` | 101 | 19,160 | exactly the 6 expected `core` compile failures: its lib test, and `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph` |
| B2a costs, greeks, spikefade, no skips | 101 | 8,949 | 691 passed, 12 failed, 3 ignored |
| B2b core's 4 targets, no skips | 101 | 215 | 9 passed, 5 failed |
| B3a costs, greeks, spikefade, with the skips | 0 | 7,886 | 691 passed, 0 failed, 3 ignored |
| B3b core's 4 targets, with the skips | 0 | 93 | 9 passed, 0 failed |

- B2 failed 17 tests, 12 engine and 5 core, and all 17 are in `skips.txt`. None is outside it.
- spikefade's lib suite has 217 tests here: 204 passed, 12 failed (all skipped in B3), 1 ignored.
- The last B3 pair counts 0 failures. `bl` exited 0.

## ENV-EXTRA

None. No test outside `skips.txt` failed, so `skips.txt` is the JOB's 17 unchanged and no `-r2` baseline ran.

## Exit codes

- `toolchain` 0, `install` 0, `bl` 0, `main` 0, `recheck` 0. Step 7 (the core follow-up) did not run: case A.
- Main run (MEASURED): cargo-mutants' own unmutated baseline took 18 s to build and 6 s to test, and it auto-set the
  test timeout to 31 s. 815 mutants tested in 73 minutes (2026-11-16T15:57:13Z to 2026-11-16T17:10:28Z): 815 caught,
  0 missed, 0 timeout, 0 unviable. Exit 0.
- Re-check (step 8): `find-timing.sh` found 0 timing-only catches, so `run-recheck.sh` printed
  `no timing-only catches` and exited 0. There is no `recheck-jobs1/` folder.

## Anything odd

- Every one of the 815 catches lists `tests34::v34_cube_day_high_vs_bhavcopy_high` as a FAILED test in its log
  (MEASURED from the per-mutant logs; the logs themselves are not pushed).
- The wall-clock test `split18::tests::v32rb_split_o1_gates` was also FAILED in 80 of the 815 mutant logs, each time
  together with the test above. So none of the 80 was a timing-only catch, and the re-check had nothing to run. This
  box did not measure why those 80 failed; section 4 of the JOB names load under `--jobs 2` as the known cause.
- The 0 timing-only catches is not a parse failure: `outcomes.json` holds 816 outcomes, 1 baseline `Success` and 815
  `CaughtMutant`, and all 815 logs name at least one FAILED test.
- Before the run, a `git push --dry-run` of the code commit to this result branch confirmed the push would
  authenticate. It created nothing.
- The READY marker appeared at 2026-11-16T15:51:48Z, after this box had polled for it from 14:55Z. `box.start`
  (section 1b) is 2026-11-16T15:53:57Z, so `box_wall_s` does not include that wait.

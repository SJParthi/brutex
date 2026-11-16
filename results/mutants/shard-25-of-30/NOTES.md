# Mutation slice 25/30 of engine v3.4: notes

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- Memory: MemTotal 16473776 kB (about 15.7 GiB).
- Cgroup limit: the box uses cgroup v1, so the job's `/sys/fs/cgroup/memory.max` (a v2 file) does not exist and step 2
  printed nothing for it. The v1 memory cgroup the commands ran in has `memory.limit_in_bytes` 14338772992 (about
  13.4 GiB); its parents are unlimited (9223372036854771712).
- Disk at set-up: 252G, 10G used, 30G available.

## Toolchain, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, as `rust-toolchain.toml` pins.
- `cargo-mutants 27.1.0`, from `cargo install cargo-mutants --locked --version 27.1.0` (exit 0).
- Code: a fresh single-branch clone of `cloud/spikefade-v34int-20261116` in `~/mut`, HEAD checked equal to
  `7a5428d30bdb8a5e0fec427848178c82907de2fb` (`HEAD OK`). Nothing in the code was changed.
- Case **A**: no `core` mutant in the slice, so `case-args.txt` is empty.
- The exact command, run in `~/mut`:

```
cargo mutants --workspace --shard 25/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts --skip menu::tests::sf3_menu_fail_closed --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry --skip menu::tests::sf3_distance_floor_counted --skip menu::tests::sf3_tp_theta_level_and_absence --skip tests33s::v33_search1d_sets_load_and_count --skip tests3::sf3_ttp_replaces_tp_close --skip tests34s::root_digest_same_for_every_form --skip tests34s::sfz_roundtrip_all_rows --skip every_declared_cost_invariant_reaches_a_bench --skip the_document_declares_the_five_operations_rule_4_names --skip every_measured_cost_invariant_is_declared --skip every_crate_carries_a_ratio_bench --skip the_reconciliation_is_reading_something --skip tests34i::v34i_f3_real_rows_spot --skip tests34i::v34i_f7_rules_tables_load_dated_and_minrange --skip tests34i::v34i_f7_opr_file_is_next_session_and_columns
```

## The list

- `list_check` **MATCH**. Whole list 24429 mutants, sha256 `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`;
  slice 25 has 815 mutants, sha256 `923b654787945cee6b438ef138de34be50de42dda13d57a8032f2aa70b935a66`. Both equal the table.
- All 815 mutants of the slice are in `crates/spikefade/src/fix34.rs` at line 1301, and every one replaces the return
  value of `wick_compare`, as section 12 of the job says for slices 19 to 28.

## The baseline (step 5)

| section | rc | wall ms | what it showed |
|---|---|---|---|
| B1 | 101 | 16567 | exactly the 6 expected `could not compile` errors, all for `core`: its lib test and the test targets `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph`. No other target failed to compile. |
| B2a | 101 | 5950 | `costs` and `greeks` pass; `spikefade` unit tests 204 passed, 12 failed, 1 ignored (216 + 1, as on the Mac) |
| B2b | 101 | 139 | `core`: `bound_probe` 2, `lint` 2 and `zerodha_index` 5 pass; `cost_invariants` 0 passed, 5 failed |
| B3a | 0 | 5595 | `spikefade` 204 passed, 0 failed, 1 ignored, 12 filtered out; `costs` and `greeks` pass |
| B3b | 0 | 55 | `core`'s 4 targets: 0 failed; the 5 `cost_invariants` names filtered out |

- The set of B2 failures is exactly the 17 names of `skips.txt` (12 engine, 5 core). None is outside it.
- The last command of step 5 (failures in the last B3 pair) printed 0.
- **ENV-EXTRA: none.** `skips.txt` is the job's 17 names, unchanged. There was no `-r2` baseline.

## Exit codes and results

- toolchain 0, install 0, bl 0, main 0, recheck 0. Step 7 is for case C only; its block was run and did nothing.
- cargo-mutants' own baseline: 14 s build + 3 s test. The test timeout was auto-set to 20 s.
- The main run went from 2026-11-16T15:55:23Z to 2026-11-16T16:48:15Z. cargo-mutants reported
  `815 mutants tested in 53m: 815 caught`.
- **caught 815, missed 0, timeout 0, unviable 0.**
- Step 8: `timing-only catches: 0`, so the `--jobs 1` re-check had nothing to run (`no timing-only catches`, exit 0) and
  there is no `recheck-jobs1/` folder.
- Progress commits: ef4b40d (196 tested), 77a5002 (409), 92cdb2b (603), 53ade8f (814).

## Anything odd

- **One test catches the whole slice.** Every one of the 815 caught logs names
  `tests34::v34_cube_day_high_vs_bhavcopy_high` as FAILED. The only other name that appears as FAILED in any log is the
  wall-clock gate below. So this slice's catches rest on that one test, as far as the test targets that ran show.
- **The wall-clock gate fails under load.** 42 of the 815 caught logs also name
  `split18::tests::v32rb_split_o1_gates` as FAILED under `--jobs 2`, about 5 percent. It never failed alone: each of
  the 42 also failed `tests34::v34_cube_day_high_vs_bhavcopy_high`. So none of them is a timing-only catch.
- The `node` parse in `find-timing.sh` was checked by hand: `outcomes.json` holds 1 Success (the baseline) and 815
  CaughtMutant entries, each with a mutant name and a log path. So the 0 above is a real 0, not a failed parse.
- Rust 1.97.1 with all the named components was already on the box (rustup installed it earlier in this session),
  so the toolchain step reported `unchanged`. rustup then checked for and downloaded a self-update of itself. Exit 0.
- Before step 6 the box ran one `git push --dry-run` to its result branch, to confirm it could push before the long
  run. It changed no ref.
- The job's bash blocks were run as written, taken straight from `JOB-MUTANTS.md` (sha256
  `078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74`, checked). Only the first line of section 1b
  was filled in, with K=25 and the commit.
- The READY marker appeared at 15:51:13Z, 55 minutes after this box began polling for it at 14:56:02Z.

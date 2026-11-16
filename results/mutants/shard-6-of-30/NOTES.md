# Mutation slice 6/30 of engine v3.4: notes (the box writes these; see step 9)

Box 6 of 30, wave v34-mutants of 16 Nov 2026. Code branch `cloud/spikefade-v34int-20261116` at commit
`7a5428d30bdb8a5e0fec427848178c82907de2fb`; `JOB-MUTANTS.md` sha256 `078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74`,
both checked against `READY.txt` before use. Code only: no market data was fetched, read or written, and no code was changed.

## The box

- `uname -a`: Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux
- `nproc`: 4
- Memory: MemTotal 16473776 kB. There is no cgroup v2 `memory.max` file; the cgroup v1 memory limit reads
  9223372036854771712, which is no limit.
- Disk: `/dev/vda` 252G, 30G free at the start and 28G free after the run.

## Versions, case and the exact command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`. The toolchain
  step reported the toolchain unchanged (exit 0). In the same step rustup checked for and downloaded a self-update; it now
  reports rustup 1.29.3.
- `cargo-mutants 27.1.0`, from `cargo install cargo-mutants --locked --version 27.1.0` (exit 0).
- Case A: all 815 mutants of the slice are in `spikefade` (report.rs 290, rules.rs 43, screen.rs 482), so no case arguments.
- The exact command (`run-main.sh`, run in `~/mut`):

```
cargo mutants --workspace --shard 6/30 --jobs 2 --cargo-arg=--locked -o /root/mo -- -- --exact --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts --skip menu::tests::sf3_menu_fail_closed --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry --skip menu::tests::sf3_distance_floor_counted --skip menu::tests::sf3_tp_theta_level_and_absence --skip tests33s::v33_search1d_sets_load_and_count --skip tests3::sf3_ttp_replaces_tp_close --skip tests34s::root_digest_same_for_every_form --skip tests34s::sfz_roundtrip_all_rows --skip every_declared_cost_invariant_reaches_a_bench --skip the_document_declares_the_five_operations_rule_4_names --skip every_measured_cost_invariant_is_declared --skip every_crate_carries_a_ratio_bench --skip the_reconciliation_is_reading_something --skip tests34i::v34i_f3_real_rows_spot --skip tests34i::v34i_f7_rules_tables_load_dated_and_minrange --skip tests34i::v34i_f7_opr_file_is_next_session_and_columns
```

## list_check

MATCH. The whole list holds 24,429 mutants, sha256 `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`;
slice 6 holds 815, sha256 `dd17f481eb9a423597d2f394e34cf1618785bbb36a76fe729faf5287c9f56b9e`. Both equal the table.

## Baseline (`baseline.log`)

| section | command | rc | wall ms | result |
|---|---|---|---|---|
| B1 | `cargo build --locked --workspace --tests --keep-going` | 101 | 17368 | exactly the 6 expected `core` targets do not compile: lib test, `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph` |
| B2a | `cargo test --locked --no-fail-fast -p costs -p greeks -p spikefade` | 101 | 7565 | costs and greeks all pass; spikefade 204 passed, 12 failed, 1 ignored |
| B2b | `cargo test --locked --no-fail-fast -p core` with the 4 targets | 101 | 141 | 5 failed, all in `cost_invariants` |
| B3a | B2a with `-- --exact` and the 17 skips | 0 | 7358 | 0 failed |
| B3b | B2b with `-- --exact` and the 17 skips | 0 | 77 | 0 failed |

B2 failed 17 tests, every one of them in `skips.txt` (12 engine, 5 core), as the JOB expects. No `bl2` re-run was needed.

## ENV-EXTRA

None.

## Exit codes and the run

- Exit codes: toolchain 0, install 0, bl 0, main 2, recheck 2. Step 7 (the core follow-up) did not run: case A.
- main: exit 2 (some missed), 2026-11-16T15:56:07Z to 17:03:22Z, 67 min. cargo-mutants' own baseline took 17 s build + 5 s
  test, and it auto-set the test timeout to 26 s. 815 tested: caught 251, missed 557, timeout 0, unviable 7.
  - report.rs: caught 15, missed 275
  - rules.rs: caught 35, missed 8
  - screen.rs: caught 201, missed 274, unviable 7
- Step 8, the wall-clock re-check: 41 of the 251 catches were timing-only (report.rs 15, screen.rs 26). 40 of them failed
  only `split18::tests::v32rb_split_o1_gates`; 1 failed that and `tests32r::v32r_o1_range_max_and_first_reach`.
  - The `--jobs 1` re-check (exit 2, test timeout auto-set to 27 s) tested 55 mutants. All 41 names were MISSED:
    recheck_caught 0, recheck_missed 41.
  - The other 14 are `delete field ... from struct ... expression` mutants of the same files, which `-F` does not select,
    as the JOB foresees. 1 of them was caught (screen.rs:503:30, delete field lists from struct DayEvents expression in
    generate). They belong to the re-check only.
- Corrected for the re-check: caught 251 - 41 = 210, missed 557 + 41 = 598. By file: report.rs 0 caught of 290 (all 15 of its
  catches were timing-only), rules.rs 35 of 43, screen.rs 175 of 482 (7 unviable).

## Anything odd

- report.rs has no catch left in this slice once the re-check is applied: its 15 catches came only from the wall-clock gate
  failing under `--jobs 2`.
- The 7 unviable mutants are all in screen.rs (lines 147, 288 four times, 290 and 640).
- Progress commits were pushed after every third wait: 8a0bfd2, 97e8fc5, f990ead, 3efc49c, 5bb4f69, aaf188e.
- Beyond the JOB's blocks the box ran only read-only checks: a `git push --dry-run` of the result branch before the first
  progress push (no ref changed); a count of the CaughtMutant entries in `outcomes.json` with find-timing.sh's own node
  one-liner (251 of 816 outcomes, the baseline included); a tally of the failed wall-clock tests; and scan.sh's two patterns,
  counts only, over every file final.sh adds (0 hits) before final.sh ran.
- The JOB's limit applies: a mutant that only a skipped test would catch is counted as missed.
- Waiting for GO: the READY marker was polled every 60 s from 14:53:37Z and appeared at 15:51:40Z; the box started at 15:53:21Z.

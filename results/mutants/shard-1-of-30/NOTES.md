# Mutation slice 1/30 of engine v3.4: notes (the box writes these; see step 9)

Box K=1 of 30, wave 2, 16 Nov 2026. Code commit `7a5428d30bdb8a5e0fec427848178c82907de2fb`, the tip of
`cloud/spikefade-v34int-20261116`. `JOB-MUTANTS.md` sha256 `078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74`,
equal to READY.txt. Code only: no market data was fetched, read or written. Nothing here is a trading result.

Labels as in the job file: EXACT = a count, MEASURED = a number from a named run on this box.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- memory: MemTotal 16,473,776 kB (about 15.7 GiB)
- cgroup limit: this box runs cgroup v1, so `/sys/fs/cgroup/memory.max` does not exist and step 2 printed nothing for
  it. The v1 `memory/memory.limit_in_bytes` is 9223372036854771712, which is no limit.
- disk: 30 GB free of 252 GB at set-up, 29 GB free after the run.

## Toolchain, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, `cargo-mutants 27.1.0` (installed with
  `cargo install cargo-mutants --locked --version 27.1.0`, exit 0).
- The toolchain step reported 1.97.1 `unchanged`: the box had already synced it from `rust-toolchain.toml` before the
  job began. rustup then checked for and downloaded a self-update of rustup itself (a Rust download). Exit 0.
- **Case A.** All 815 mutants of the slice are in `crates/costs`, none in `core`, so `case-args.txt` is empty.
- Exact command, run by `run-main.sh` in `/root/mut`:

```
cargo mutants --workspace --shard 1/30 --jobs 2 --cargo-arg=--locked -o /root/mo -- -- --exact --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts --skip menu::tests::sf3_menu_fail_closed --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry --skip menu::tests::sf3_distance_floor_counted --skip menu::tests::sf3_tp_theta_level_and_absence --skip tests33s::v33_search1d_sets_load_and_count --skip tests3::sf3_ttp_replaces_tp_close --skip tests34s::root_digest_same_for_every_form --skip tests34s::sfz_roundtrip_all_rows --skip every_declared_cost_invariant_reaches_a_bench --skip the_document_declares_the_five_operations_rule_4_names --skip every_measured_cost_invariant_is_declared --skip every_crate_carries_a_ratio_bench --skip the_reconciliation_is_reading_something --skip tests34i::v34i_f3_real_rows_spot --skip tests34i::v34i_f7_rules_tables_load_dated_and_minrange --skip tests34i::v34i_f7_opr_file_is_next_session_and_columns
```

## List check

- **`list_check=MATCH`** (EXACT). The whole list is 24,429 mutants, sha256 `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`.
  Slice 1 is 815 mutants, sha256 `b5e4cdcf5432e2da59dc203823668ca335327122dc42225bad7f8e4ed117875d`. Both equal the table.
- The slice by file (EXACT): day 44, dhan_calc 101, equity_clock 47, equity_dated 62, equity_readings 134, error 16,
  exact 65, expiry 15, fill 27, ledger 39, lot 94, money 19, moneyness 33, rate 6, regime 41, schedule 54, scope 10,
  stack 8 (all `crates/costs/src/`).

## Baseline (MEASURED, `baseline.log`)

| section | rc | wall | what it showed |
|---|---|---|---|
| B1 build | 101 | 21,580 ms | exactly the 6 expected `core` compile failures: lib test, and the test targets `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph`; each is a `couldn't read` of an `include_str!` file this branch does not hold |
| B2a costs, greeks, spikefade | 101 | 7,477 ms | spikefade lib 204 passed, 12 failed, 1 ignored; the 12 are the 12 engine skips; every other target ok |
| B2b core, 4 targets | 101 | 151 ms | `cost_invariants` 0 passed, 5 failed: the 5 core skips; `bound_probe`, `lint`, `zerodha_index` ok |
| B3a with skips | 0 | 7,219 ms | 0 failed; spikefade lib 204 passed, 1 ignored, 12 filtered out |
| B3b with skips | 0 | 87 ms | 0 failed; `cost_invariants` 5 filtered out |

- B2 failures: 17, every one in `skips.txt`, none outside it. The last B3 pair: 0 failures.
- **ENV-EXTRA: none.** `skips.txt` is the job file's 17 names, unchanged.

## The run and the exit codes

- **main: exit 3** (some timed out; step 6 says go on). Start 2026-11-16T15:56:33Z, end 2026-11-16T16:09:05Z, 752 s
  (MEASURED). cargo-mutants' own baseline: ok in 5 s build + 1 s test, and it auto-set the test timeout to 20 s.
- Outcomes (EXACT, cargo-mutants' own files): 815 tested; caught 587, missed 24, timeout 5, unviable 199.
- Missed by file: equity_clock.rs 11, lot.rs 10, and 1 each in equity_dated.rs, equity_readings.rs, fill.rs. See
  `missed.txt`.
- The 5 timeouts each took 1 s build + 20 s test, so they hit the 20 s cap. Four are in `day.rs` `civil_from_days`
  (537:19, 539:19, 539:40, 540:49) and one is in `expiry.rs` `next_weekly_on` (454:47, `&&` to `||`). From reading the
  code, not from a measurement: `civil_from_days` is straight-line arithmetic, so those hangs must be in a caller that
  loops on its result (not investigated); in `next_weekly_on`, with `||` the `find` can return a start at or before
  `from`, so the enclosing loop need not advance. No wall-clock test is in `costs`.
- Step 7 (core follow-up): not run, case A.
- Step 8 (wall-clock re-check): `find-timing.sh` found 0 timing-only catches, and `run-recheck.sh` exited 0 with
  `no timing-only catches`, so there is no `recheck-jobs1/` folder. Cross-check on this box: node read 587
  `CaughtMutant` outcomes from `outcomes.json`, and each of the 587 logs names at least one `FAILED` test; none of those
  sets is made only of the 4 wall-clock tests, which live in `spikefade` and so are not in a `costs` mutant's test run.
- Other exits: toolchain 0, install 0, bl 0 (the shell's own; the section rcs are above), recheck 0.

## Anything odd

- Nothing outside the job file's expectations. `main.log` holds no WARN or ERROR line.
- The run took 12.5 min, under section 13's ESTIMATE of 15 to 30 min for slice 1.
- Before step 1b the box polled 58 min for the READY marker (from 14:53Z; it appeared at 15:51:43Z), so `box.start`
  is 15:53:18Z.
- The limit in step 4 applies: a mutant that only a skipped test would catch is counted as missed.

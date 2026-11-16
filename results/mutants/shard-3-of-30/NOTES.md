# Mutation slice 3/30 of engine v3.4: notes

Box K=3 of 30, wave 2, 16 Nov 2026. Code only: no market data was fetched, read or written, and nothing outside this
folder was committed.

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- Memory: MemTotal 16,473,776 kB, no swap.
- Memory cgroup (cgroup v1): the job's cgroup has `memory.limit_in_bytes` = 14,338,772,992 bytes (13,674 MiB). Step 2's
  `cat /sys/fs/cgroup/memory.max` printed nothing, because that cgroup-v2 file does not exist on this box.
- Disk at set-up: 30 GB free of 252 GB.
- Box start: 2026-11-16T15:52:37Z.

## Versions, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, `cargo-mutants 27.1.0` (installed with
  `cargo install cargo-mutants --locked --version 27.1.0`).
- Code: branch `cloud/spikefade-v34int-20261116` at `7a5428d30bdb8a5e0fec427848178c82907de2fb` (step 2 printed
  `HEAD OK`). `JOB-MUTANTS.md` sha256 `078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74`, as READY.txt
  names it.
- Case **A**: all 815 mutants of the slice are in `spikefade` and none in `core`, so there are no case arguments.
- The exact command (`run-main.sh`, run in `~/mut`):

```
cargo mutants --workspace --shard 3/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts --skip menu::tests::sf3_menu_fail_closed --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry --skip menu::tests::sf3_distance_floor_counted --skip menu::tests::sf3_tp_theta_level_and_absence --skip tests33s::v33_search1d_sets_load_and_count --skip tests3::sf3_ttp_replaces_tp_close --skip tests34s::root_digest_same_for_every_form --skip tests34s::sfz_roundtrip_all_rows --skip every_declared_cost_invariant_reaches_a_bench --skip the_document_declares_the_five_operations_rule_4_names --skip every_measured_cost_invariant_is_declared --skip every_crate_carries_a_ratio_bench --skip the_reconciliation_is_reading_something --skip tests34i::v34i_f3_real_rows_spot --skip tests34i::v34i_f7_rules_tables_load_dated_and_minrange --skip tests34i::v34i_f7_opr_file_is_next_session_and_columns
```

- cargo-mutants' own baseline: ok, 16 s build + 3 s test. It auto-set the test timeout to 20 s. There was no build
  timeout (the default).

## List check

- `list_check=MATCH`. The whole list: 24,429 mutants, sha256
  `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`. Slice 3: 815 mutants, sha256
  `3622e038eeaf9e4836923e59dbb03aa254b577d18dcc410af8464f3c92934fa5`. Both equal the table.
- The slice runs from `crates/spikefade/src/acct.rs:252:21` to `crates/spikefade/src/charges.rs:157:73`.

## Baseline (`baseline.log`)

| section | rc | wall ms | what it showed |
|---|---|---|---|
| B1 | 101 | 17,944 | exactly 6 `could not compile` errors, all `core`: lib test, `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph`, as expected |
| B2a | 101 | 5,975 | `spikefade` bin: 204 passed, 12 failed, 1 ignored (216 run, the Mac's count); `costs` and `greeks` all pass |
| B2b | 101 | 188 | `cost_invariants`: 5 failed; `bound_probe`, `lint` and `zerodha_index` pass |
| B3a | 0 | 5,885 | 0 failed (`spikefade` 204 passed, 12 filtered out) |
| B3b | 0 | 64 | 0 failed |

- B2's 17 failures are exactly the 17 names in `skips.txt` (12 engine, 5 core). The last B3 pair counts 0 failures.
- **ENV-EXTRA: none.** `skips.txt` is the job's 17 names, unchanged (sha256
  `0cd96e3e2906f52103be4c894a44beec8c32170a5a8828ea7e2ea302c41fe557`).

## Exit codes and counts

- Main run: **exit 3** (some timed out). 815 tested in 52 min, 2026-11-16T15:54:57Z to 16:46:49Z: caught 404,
  missed 407, timeout 1, unviable 3.
- Step 7 (case C only): not run.
- Step 8 re-check (`--jobs 1`): **exit 2**. All 18 timing-only catches had been caught by one test only,
  `split18::tests::v32rb_split_o1_gates`. With `--jobs 1` all 18 were **MISSED** (recheck_caught 0, recheck_missed 18).
- The re-check list also held 34 extra `delete field ... from struct ... expression` mutants of `acct.rs` (lines 403, 472
  and 483), as the job says cargo-mutants 27.1.0 adds. 2 of them were caught and 32 missed. They belong to the re-check
  only.
- Corrected counts (step 8's rule): caught 404 - 18 = **386**, missed 407 + 18 = **425**. The official files stay as
  cargo-mutants wrote them.

## Anything odd

1. **One mutant was killed by the memory limit, and is counted as caught.**
   `crates/spikefade/src/cal.rs:19:35: replace + with *` made the `spikefade` test binary grow to about 13.5 GB resident
   (anon-rss 13,531,904 kB). The memory cgroup's OOM killer sent it SIGKILL, `cargo test` returned 101, and cargo-mutants
   recorded the mutant as CAUGHT (log `crates__spikefade__src__cal.rs_line_19_col_35_001.log`). It is the only OOM kill in
   the kernel log for the run. Neither the other job's test nor cargo-mutants itself was killed. This catch is by
   resource exhaustion, not by an assertion.
2. **The one timeout:** `crates/spikefade/src/acct.rs:435:18: replace -= with /= in flush` (20 s test timeout).
3. **The 3 unviable** (they do not build): `acct.rs:471:5` in `detail_list`, `charges.rs:49:5` in `trade_day` and
   `charges.rs:60:5` in `rates_on`, each a return value replaced with `Default::default()`.
4. **rustup updated itself.** Step 2's `rustup toolchain install 1.97.1 ...` also self-updated rustup from 1.28.2 to
   1.29.3. The toolchain did not change: 1.97.1 was already installed with every component and target the job names.
5. The checkout keeps an untracked `target/` from the baseline build. It was never staged, and nothing outside
   `results/mutants/shard-3-of-30/` was pushed.

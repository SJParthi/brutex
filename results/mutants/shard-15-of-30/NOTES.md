# Mutation slice 15/30 of engine v3.4: notes (the box writes these; see step 9)

Box 15 of 30 of the v34-mutants wave, 16 Nov 2026. The READY marker `cloud/ready-v34-mutants-20261116` (commit
503c6660) named code commit 7a5428d30bdb8a5e0fec427848178c82907de2fb on `cloud/spikefade-v34int-20261116` (the branch
tip) and JOB-MUTANTS.md with sha256 078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74. The file at that
commit hashed to exactly that value. Every JOB block was run from the JOB file's own text; the only edit was line 1 of
section 1b (K=15 and the commit). The run used `~/mut`, a fresh clone, as section 2 says.

## The box

- `uname -a`: Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC x86_64 GNU/Linux
- `nproc`: 4
- memory: MemTotal 16473776 kB (about 15.7 GiB)
- cgroup memory limit: none. The box runs cgroup v1, so `/sys/fs/cgroup/memory.max` does not exist and section 2's
  `cat` printed nothing. The v1 file `/sys/fs/cgroup/memory/memory.limit_in_bytes` reads 9223372036854771712, which is
  v1's "no limit" value.
- disk: 30 GB free on `/` at the start.

## Tools, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`. The repository's `rust-toolchain.toml`
  had already installed 1.97.1; the JOB's `rustup toolchain install` reported it unchanged (exit 0), then rustup
  checked for and downloaded a self-update of rustup itself.
- `cargo-mutants 27.1.0`, from `cargo install cargo-mutants --locked --version 27.1.0` (exit 0).
- Case **A**: the slice has no `core` mutant, so `case-args.txt` is empty.
- The exact command (`run-main.sh`, in `~/mut`):

```
cargo mutants --workspace --shard 15/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts --skip menu::tests::sf3_menu_fail_closed --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry --skip menu::tests::sf3_distance_floor_counted --skip menu::tests::sf3_tp_theta_level_and_absence --skip tests33s::v33_search1d_sets_load_and_count --skip tests3::sf3_ttp_replaces_tp_close --skip tests34s::root_digest_same_for_every_form --skip tests34s::sfz_roundtrip_all_rows --skip every_declared_cost_invariant_reaches_a_bench --skip the_document_declares_the_five_operations_rule_4_names --skip every_measured_cost_invariant_is_declared --skip every_crate_carries_a_ratio_bench --skip the_reconciliation_is_reading_something --skip tests34i::v34i_f3_real_rows_spot --skip tests34i::v34i_f7_rules_tables_load_dated_and_minrange --skip tests34i::v34i_f7_opr_file_is_next_session_and_columns
```

## The list

- `list_check` **MATCH**. The whole list is 24,429 mutants, sha256 3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a;
  slice 15 is 815 mutants, sha256 115221b0882bbaf47e249eb5badb7f4c13caa20e61dee506f5b03f2ea202b336. Both equal the table.
- The slice is all `spikefade`: 249 mutants in `crates/spikefade/src/gk.rs` and 566 in `crates/spikefade/src/v32.rs`.

## The baseline (section 5)

| section | rc | wall | what it showed |
|---|---|---|---|
| B1 `cargo build --locked --workspace --tests --keep-going` | 101 | 20.8 s | exactly the 6 expected `core` targets failed to compile (lib test, `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph`), each on an `include_str!` of an app-repository file this branch does not hold |
| B2a costs, greeks, spikefade, no skips | 101 | 7.4 s | 12 failures, all in `skips.txt`; the spikefade lib target gave 204 passed, 12 failed, 1 ignored |
| B2b core's 4 compilable targets, no skips | 101 | 0.17 s | 5 failures (the 5 `cost_invariants` names), all in `skips.txt` |
| B3a as B2a, with the 17 skips | 0 | 7.2 s | 0 failed; spikefade lib 204 passed, 1 ignored, 12 filtered out |
| B3b as B2b, with the 17 skips | 0 | 0.07 s | 0 failed; 5 filtered out |

B2 failed exactly 17 tests, the 17 of `skips.txt`. The failure count for the last B3 pair is 0.

## ENV-EXTRA

None. No test failed outside `skips.txt`, so `skips.txt` is the JOB's 17 unchanged and no `-r2` baseline ran.

## The main run (section 6)

- Started 2026-11-16T15:56:30Z, ended 2026-11-16T17:02:58Z (66 min). Exit **2** (some missed).
- cargo-mutants' own baseline: ok in 19 s build + 4 s test. It auto-set the test timeout to 25 s.
- 815 tested: 546 caught, 265 missed, 0 timeout, 4 unviable.
- The 4 unviable are all `replace * with /` in `v32.rs`: 232:60 and 233:88 in `utilization`, 265:60 and 266:88 in
  `margin_mtm_peak`.
- Progress commits were pushed at 153, 316, 482, 649 and 808 tested, about 13.5 minutes apart.

## The wall-clock re-check (section 8)

- `find-timing.sh` found 18 timing-only catches. In every one of the 18 logs the only FAILED test is
  `split18::tests::v32rb_split_o1_gates`, the same gate the last wave's slice 26 re-checked.
- The `-f`/`-F` selection listed 22 mutants: the 18 names, plus 4 `delete field ... from struct ... expression` mutants
  of the same files that `-F` does not filter, as section 8 predicts: `gk.rs` 118:21, 118:27 and 118:34 in `bar_greeks`,
  and `v32.rs` 363:27 in `port_walk`.
- With `--jobs 1`, all 22 were MISSED (22 tested in 3 min; its own baseline 20 s build + 5 s test, timeout auto-set to
  27 s). Exit **2**.
- recheck_caught 0, recheck_missed 18, recheck_extra 4. So every timing-only catch of this slice was load, not the
  mutant.
- **Corrected counts:** caught 546 - 18 = **528**; missed 265 + 18 = **283**. The official files keep cargo-mutants'
  own output unchanged.

## Exit codes

`toolchain` 0, `install` 0, `bl` 0 (the script; its section rcs are above), `main` 2, `recheck` 2. No core follow-up
(case A, so section 7 did nothing).

## Anything odd

- Nothing outside the JOB's expectations: list MATCH, the baseline exactly as section 5 expects, no timeout, no STOP.
- Limit, as section 4 says: a mutant that only a skipped test would catch is counted as missed.
- Before the run, one `git push --dry-run` to this branch confirmed push access. It pushed nothing.

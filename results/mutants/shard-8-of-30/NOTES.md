# Mutation slice 8/30 of engine v3.4: notes

Labels follow JOB-MUTANTS.md. EXACT means a count. MEASURED means a number from a named run on this box. No figure here is
a trading result.

## Box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`.
- `nproc`: 4. `MemTotal`: 16,473,776 kB.
- **Cgroup memory limit:** none.
  - `/sys/fs/cgroup/memory.max` does not exist here, because this box uses cgroup v1.
  - The v1 memory controller's `memory.limit_in_bytes` for the shell's cgroup is 9223372036854771712, which means no
    limit.
- Disk: 252 GB volume, 29 GB free at set-up.

## Toolchain and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, cargo-mutants 27.1.0.
- **cargo-mutants was already installed when step 2 ran.**
  - I installed it while waiting for the READY branch, with the same pinned command:
    `cargo install cargo-mutants --version 27.1.0 --locked`.
  - Step 2's own install then printed "already installed" and exited 0.
  - `cargo mutants --version` printed `cargo-mutants 27.1.0`.
- Source: branch `cloud/spikefade-v34int-20261116` at `7a5428d30bdb8a5e0fec427848178c82907de2fb`.
  - This is the commit READY.txt names.
  - JOB-MUTANTS.md at that commit has the sha256 READY.txt gives (`078206fd...`).
- Case **A**: the slice holds no `core` mutant, so no case arguments were added.
- Command, from `~/mut` (`run-main.sh`):

  ```sh
  cargo mutants --workspace --shard 8/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip <each of the 17 names in skips.txt>
  ```

## List check (EXACT)

- `list_check=MATCH`.
  - The workspace list holds 24,429 mutants, sha256 `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`.
  - Slice 8 holds 815 mutants, sha256 `fce194eb42c39123c01a9ac50aaae55bb83856879d3f0616bfb42d3754104906`.
  - Both equal the job's table.
- The slice is all `spikefade`: `kern.rs` 269, `trade.rs` 250 and `v2.rs` 296.

## Baseline (MEASURED, `baseline.log`)

| section | rc | wall ms | what it showed |
|---|---:|---:|---|
| B1 | 101 | 22,959 | exactly the 6 expected `core` targets fail to compile: lib test, `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph` |
| B2a | 101 | 7,959 | `spikefade` 204 passed, 12 failed, 1 ignored; `costs` and `greeks` all pass |
| B2b | 101 | 155 | `core` `cost_invariants` 0 passed, 5 failed; `bound_probe`, `lint` and `zerodha_index` pass |
| B3a | 0 | 7,339 | 0 failed (`spikefade` 204 passed, 1 ignored, 12 filtered out) |
| B3b | 0 | 85 | 0 failed (`cost_invariants` 5 filtered out) |

- **B2 failed 17 tests, and all 17 are in `skips.txt`.** The final check over the last B3 pair printed 0.
- **Causes on this box** (the panic messages in `baseline.log`):
  - **The 5 `menu::tests::*`:** `menu_head: missing`.
  - **`tests33s::v33_search1d_sets_load_and_count` and `tests3::sf3_ttp_replaces_tp_close`:** `No such file or
    directory` for the exit-rule tables under `crates/spikefade/data/`.
  - **`tests34s::root_digest_same_for_every_form` and `tests34s::sfz_roundtrip_all_rows`:** the KNOWN cause. With every
    kit switch off, the engine calls zstd at the Mac path `/opt/homebrew/bin/zstd` (kit.rs:470), and that path is absent
    here. The fix is not in v3.4.
  - **The 3 `tests34i::*`:** the real tables at an absolute path on the operator's Mac are absent.
  - **The 5 `core` `cost_invariants` names:**
    - 4 fail because `docs/04-invariants.md` is not on this branch;
    - `every_crate_carries_a_ratio_bench` fails because `crates/spikefade` has no `benches/ratio.rs`.
- **ENV-EXTRA:** none. No failure outside `skips.txt`.

## Main run (MEASURED and EXACT)

- 2026-11-16T15:54:06Z to 16:58:15Z, so 3,849 s.
- **Exit 3:** some mutants timed out. JOB step 6 allows it.
- cargo-mutants' own unmutated baseline: 20 s build + 4 s test. It auto-set the test timeout to 25 s.

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| `kern.rs` | 207 | 59 | 1 | 2 | 269 |
| `trade.rs` | 106 | 135 | 0 | 9 | 250 |
| `v2.rs` | 15 | 279 | 0 | 2 | 296 |
| **all** | **328** | **473** | **1** | **13** | **815** |

- **The one timeout:** `kern.rs:392:18: replace -= with /= in flush`, which ran 25 s.
- **Missed, by function (EXACT; the 12 largest):**

  | file | function | missed |
  |---|---|---:|
  | `v2.rs` | `evidence_md` | 52 |
  | `v2.rs` | `blocks_md` | 48 |
  | `trade.rs` | `out_of` | 30 |
  | `v2.rs` | `parse_settings` | 28 |
  | `v2.rs` | `remark_peak` | 27 |
  | `trade.rs` | `size_event_mx` | 21 |
  | `trade.rs` | `trip_rd` | 20 |
  | `trade.rs` | `price_event` | 20 |
  | `v2.rs` | `r1_overlaps` | 13 |
  | `v2.rs` | `why_words` | 11 |
  | `kern.rs` | `cap_eval` | 11 |
  | `v2.rs` | `interval_peak` | 9 |

- **Limit:** a mutant that only a skipped test would catch is counted as missed.

## Wall-clock re-check (step 8)

- **The timing-only catches:** 30 of the 328 catches failed only tests in `timing-tests.txt`.
  - By file: `trade.rs` 10, `v2.rs` 15, `kern.rs` 5.
  - They are in `recheck-jobs1/recheck-names.txt`.
- **`--jobs 1` re-check:**
  - Its list holds 68 mutants: the 30 names plus 38 extra `delete field ... from struct ... expression` mutants of the
    same files. The job file foresaw these extras.
  - cargo-mutants' baseline: 19 s build + 5 s test, with the timeout auto-set to 26 s.
  - "68 mutants tested in 8m". **Exit 2.**
- **Result for the 30 names (EXACT):**
  - 0 caught and 30 MISSED: every timing-only catch passed every test when run alone.
  - So `recheck_caught=0` and `recheck_missed=30`.
  - The 38 extras (17 caught, 21 missed) belong to the re-check only (`recheck_extra=38`).
- **Corrected counts:** 298 caught (328 - 30) and 503 missed (473 + 30).
  - By file: `kern.rs` 202 caught and 64 missed, `trade.rs` 96 and 145, `v2.rs` 0 and 294.
  - **All 15 of `v2.rs`'s catches were timing-only,** so no test catches any viable `v2.rs` mutant on its own.
- **The official files stay as cargo-mutants wrote them:** `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt`
  and `outcomes.json`.

## Odd things

- **The READY branch** appeared 58 minutes after this box started polling: first poll 14:53:43Z, seen at 15:51:49Z.
  Nothing ran against the code before then.
- **Every progress push succeeded:** `0a99b29`, `e0822ed`, `8da513b`, `1366ee6`, `fb3878c` and `77f2880`.
- **Before the final push** I scanned every result file with `scan.sh`'s patterns, counting hits without printing them.
  There were 0 hits.

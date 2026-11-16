# Mutation slice 28/30 of engine v3.4: notes

Box 28 of 30. Code branch `cloud/spikefade-v34int-20261116` at commit `7a5428d30bdb8a5e0fec427848178c82907de2fb`,
the commit that READY.txt named (READY branch `cloud/ready-v34-mutants-20261116`, commit `503c666`, which appeared at
15:51Z). Before use, JOB-MUTANTS.md was checked against the sha256 in READY.txt (`078206fd...7b74`, equal). Its bash
blocks were then run as written, extracted from the file itself, with only the first line of section 1b set to K=28
and the commit.

## The box

- uname: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- nproc: 4
- memory: MemTotal 16,473,776 kB, no swap. `/sys/fs/cgroup/memory.max` does not exist on this box (cgroup v1).
  The kernel's OOM report (see "Anything odd") gives the memory cgroup limit that all of the box's commands share:
  14,002,708 kB (`hierarchical_memory_limit` 14,338,772,992 bytes).
- disk: `/dev/vda`, 30 GB available at the start, 28 GB at the end.

## Versions, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`.
  `rustup toolchain install` left 1.97.1 unchanged; it did update rustup itself, from 1.28.2 to 1.29.3.
- `cargo-mutants 27.1.0`, from `cargo install cargo-mutants --locked --version 27.1.0`.
- case **A**: all 815 mutants of the slice are in `spikefade`, so there are no case arguments.
- the command (`run-main.sh`, in `~/mut`):
  `cargo mutants --workspace --shard 28/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact`, then
  `--skip <name>` for each of the 17 names in `skips.txt`, in that file's order.

## list_check

**MATCH.** The whole list is 24,429 mutants, sha256 `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`.
Slice 28 is 815 mutants, sha256 `34fbb4e47bab57a2e2ee2e50a8e198da37147df71a409cd5ec584cc2cafaec17`. Both equal the table.

The slice by file: `kit.rs` 378, `fix34.rs` 258, `ckmerge.rs` 150, `slice.rs` 29. Section 12 says slices 19 to 28 are
almost all `wick_compare`. Slice 28 holds only the tail of that run: 64 return-value replacements of `wick_compare`
(`fix34.rs:1301`). After them come the rest of `fix34.rs` (`wick_high`, `wick_flagged`, `wick_words`,
`intraday_mtm_dd`, `elm_pre_*` and others), then `ckmerge.rs`, `kit.rs` and `slice.rs`. The list hash equals the
table, so this is the intended slice.

## Baseline (`baseline.log`)

| section | rc | wall ms | result |
|---|---|---|---|
| B1 build | 101 | 17,867 | exactly 6 `could not compile`, all `core`: lib test, `banned_lists`, `claim_reader`, `decision_citations`, `findings`, `graph` |
| B2a costs, greeks, spikefade | 101 | 7,195 | 12 failures, all in `skips.txt`; the spikefade lib: 204 passed, 12 failed, 1 ignored (217 tests) |
| B2b core, 4 targets | 101 | 137 | 5 failures (`cost_invariants`), all in `skips.txt` |
| B3a, with the skips | 0 | 7,126 | 0 failed |
| B3b, with the skips | 0 | 70 | 0 failed |

B2 failed 17 tests, exactly the 17 in `skips.txt`. The count for the last B3 pair is 0.

## ENV-EXTRA

None.

## Exit codes

- toolchain 0, install 0, bl (the baseline) 0.
- **main 3** (some mutants timed out). 815 tested in 4,333 s (72 min), from 15:55:27Z to 17:07:40Z. cargo-mutants'
  own baseline passed in 15 s build + 4 s test, and it auto-set the test timeout to 25 s.
  caught 548, missed 247, timeout 13, unviable 7.
- step 7 was not run (case A).
- **recheck 2** (some missed). There were 31 timing-only catches: mutants in `caught.txt` whose logs name only
  wall-clock gate tests as FAILED. With `--jobs 1`, all 31 were MISSED: recheck_caught 0, recheck_missed 31. The
  re-check list held one extra, `kit.rs:707:29: delete field sha from struct Self expression in Plan::parse`. It was
  caught (recheck_extra 1), and it belongs to the re-check only.
- **corrected caught = 548 - 31 = 517; corrected missed = 247 + 31 = 278.** The official files stay as cargo-mutants
  wrote them.

## Anything odd

1. **Out-of-memory kills at 16:54Z.** Between 16:54:09Z and 16:54:40Z, the kernel's memory-cgroup OOM killer killed
   five `spikefade` test processes. Their anon RSS was 10.2 GB, 3.7 GB, 7.0 GB, 13.7 GB and 0.13 GB.
   - All five were testing mutants of `kit.rs:799`, `Plan::batch_at`. Their logs (`kit.rs_line_799_col_9`,
     `_col_9_001`, `_col_9_002`, `_col_9_003` and `_col_48`) each end with `signal: 9, SIGKILL: kill`.
   - The five kills in the kernel log and the five SIGKILL logs are the same five, so no mutant of any other
     function had its test process killed. cargo-mutants itself was not killed.
   - The 0.13 GB process was killed 4 ms after the 13.7 GB one, before that one's memory was freed. It was the other
     worker's job, also a `kit.rs:799` mutant.
   - All seven `kit.rs:799` mutants are in `caught.txt`. Each of the five killed logs shows
     `tests34s::plan_roundtrip_and_refusals ... FAILED`, printed before the kill. The `col_48` log also shows
     `tests34s::batch_plan_target_independent ... FAILED`. So those catches rest on a failed test, not on the kill
     alone.
   - The panic text is not in those logs: libtest prints it at the end of a run, which a killed process never reaches.
   - One of the five (`col_9_002`) also lists `split18::tests::v32rb_split_o1_gates` as FAILED. It is not a
     timing-only catch, because `plan_roundtrip_and_refusals` failed too.
2. 12 of the 13 timeouts are in `kit.rs`'s hash code (`args_digest`, `Sha256::update`, `Sha256::update_pad`). The 13th
   is `par_digest`. All of them hit the 25 s test timeout.
3. 31 of the 548 catches (5.7%) were timing-only, and with `--jobs 1` all 31 then passed every test. The last wave's
   slice 26 saw the same effect (11 of 83).
4. rustup updated itself (1.28.2 to 1.29.3) during the toolchain step. The Rust toolchain stayed 1.97.1.

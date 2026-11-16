# Mutation slice 29/30 of engine v3.4: notes (the box writes these; see step 9)

## The box

- `uname -a`: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- `nproc`: 4
- memory: MemTotal 16473776 kB (about 15.7 GiB).
- cgroup limit: the box runs cgroup v1, so `/sys/fs/cgroup/memory.max` does not exist and the step-2 command printed
  nothing for it. The v1 limit `/sys/fs/cgroup/memory/memory.limit_in_bytes` reads 9223372036854771712, which is no limit.
- disk: 30G available at the start.

## Versions, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`.
- `cargo-mutants 27.1.0`, installed with `cargo install cargo-mutants --locked --version 27.1.0` (exit 0).
- Code: commit 7a5428d30bdb8a5e0fec427848178c82907de2fb, the head of `cloud/spikefade-v34int-20261116`.
  JOB-MUTANTS.md was checked against the READY sha256 078206fd515aff3e6f28ba676799312a5efcffd489a48646953a73dca88a7b74
  before use.
- Case A: the slice holds no `core` mutant, so `case-args.txt` is empty.
- The exact command (`run-main.sh`, run in `~/mut`):
  `cargo mutants --workspace --shard 29/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip <each of the 17 names of skips.txt, in its order>`
- The re-check command (`run-recheck.sh`, step 8):
  `cargo mutants --workspace -f crates/spikefade/src/f7.rs -f crates/spikefade/src/int34.rs -f crates/spikefade/src/slice.rs -f crates/spikefade/src/synth.rs -F '^<name>$' (once for each of the 86 names) --jobs 1 --cargo-arg=--locked -o ~/mo-recheck -- -- --exact --skip <the same 17>`

## list_check

MATCH. The whole list is 24,429 mutants with sha256 3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a,
and the slice is 794 mutants with sha256 bc5b8cfcc0383a5756d0c42828ea248e12bed9d8be6f946de436b21b6f89fb28. Both equal
the JOB's table. The slice is all `spikefade`: `f7.rs` 211, `int34.rs` 31, `slice.rs` 307, `synth.rs` 245.

## The baseline (step 5)

| section | rc | wall ms | what it showed |
|---|---|---|---|
| B1 | 101 | 18,591 | exactly the 6 expected `could not compile` errors, all `core`: lib test, `graph`, `banned_lists`, `claim_reader`, `decision_citations`, `findings` |
| B2a | 101 | 8,097 | `spikefade` lib 204 passed, 12 failed, 1 ignored; every other target ok. The 12 are the 12 engine names of `skips.txt` |
| B2b | 101 | 194 | `cost_invariants` 0 passed, 5 failed (the 5 `core` names of `skips.txt`); `bound_probe`, `lint`, `zerodha_index` ok |
| B3a | 0 | 7,769 | 0 failed (`spikefade` lib 204 passed, 12 filtered out, 1 ignored) |
| B3b | 0 | 83 | 0 failed (`cost_invariants` 5 filtered out) |

B2 failed 17 names, all in `skips.txt`, none outside it. The failure count of the last B3 pair is 0.

## ENV-EXTRA

None. `skips.txt` is the JOB's 17 names, unchanged (sha256
0cd96e3e2906f52103be4c894a44beec8c32170a5a8828ea7e2ea302c41fe557).

## Exit codes and results

- `toolchain` 0, `install` 0, baseline job `bl` 0 (its sections as above).
- Main run: exit 2 (some missed), 15:55:59Z to 17:08:02Z, 72 minutes. cargo-mutants' own baseline: 17 s build + 5 s
  test, and it auto-set the test timeout to 30 s. 794 tested: caught 194, missed 564, timeout 0, unviable 36.
- Re-check (step 8): `find-timing.sh` found 86 timing-only catches. Every one of them names
  `split18::tests::v32rb_split_o1_gates` as its only FAILED test. The `--jobs 1` re-check listed 88 mutants (the 86 names
  plus 2 extras, below), auto-set its test timeout to 27 s, and ended with exit 2: 88 tested, 0 caught, 88 missed. So
  recheck_caught=0, recheck_missed=86, recheck_extra=2.
- Corrected, by step 8's rule: caught 194 - 86 = 108, missed 564 + 86 = 650 (timeout 0, unviable 36). The official files
  stay as cargo-mutants wrote them.

Per file (as written by cargo-mutants, then corrected):

| file | mutants | caught | missed | unviable | timing-only | corrected caught | corrected missed |
|---|---|---|---|---|---|---|---|
| `crates/spikefade/src/f7.rs` | 211 | 37 | 168 | 6 | 24 | 13 | 192 |
| `crates/spikefade/src/int34.rs` | 31 | 6 | 25 | 0 | 3 | 3 | 28 |
| `crates/spikefade/src/slice.rs` | 307 | 83 | 195 | 29 | 30 | 53 | 225 |
| `crates/spikefade/src/synth.rs` | 245 | 68 | 176 | 1 | 29 | 39 | 205 |
| total | 794 | 194 | 564 | 36 | 86 | 108 | 650 |

## Anything odd

1. 86 of the 194 catches (44%) came only from the wall-clock test `split18::tests::v32rb_split_o1_gates`, and all 86
   passed it under `--jobs 1`, so they are MISSED. This is the last wave's slice 26 finding (11 of 83), larger here.
2. The two re-check extras are `crates/spikefade/src/synth.rs:66:24: delete field ymd from struct Cube expression in day`
   and `crates/spikefade/src/synth.rs:66:29: delete field day from struct Cube expression in day`. cargo-mutants 27.1.0
   did not apply `-F` to them, as the JOB says. Both were MISSED; they belong to the re-check only.
3. In step 2, rustup printed `checking for self-update` and `downloading self-update` after the toolchain step. The 1.97.1
   toolchain itself was reported unchanged, and `rustc --version` printed the required version.
4. The checkout's `target/` is untracked (not ignored) on this branch. It was never staged: only
   `results/mutants/shard-29-of-30/` is added, and `scan.sh` passed on every push.
5. Step 7's block returned rc 1 because its case-C test is false here (case A). Nothing ran.
6. Two checks beyond the JOB, both read-only. First, the helper scripts were generated a second time from the JOB file's
   own text in a scratch directory and compared byte for byte: all 11 scripts and `job.env` were the same. The blocks of
   steps 3, 4, 5 (the checks), 7 and 8 were run as extracted from the JOB file's own lines. Second, one
   `git push --dry-run` of the code commit to this branch name ran before the first progress push, to confirm the push
   path. It created nothing.
7. The READY marker was polled for from 14:56:42Z and appeared at 15:51:07Z.

## Timeline (UTC)

- box start (step 1b) 15:52:57Z; baseline job `bl` 15:54:54Z to 15:55:29Z (from its pid and exit files' times)
- main run 15:55:59Z to 17:08:02Z; re-check 17:08:17Z to 17:20:27Z (same source)
- progress pushes: 809a843 (156 tested), 69f792b (298), 45fbe42 (445), 59a442b (589), 6bc0380 (728), e38df56 (794)

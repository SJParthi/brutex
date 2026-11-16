# Mutation slice 16/30 of engine v3.4: notes (the box writes these; see step 9)

## The box

- uname: `Linux vm 6.18.44-fc-v470 #1 SMP PREEMPT_DYNAMIC @0 x86_64 x86_64 x86_64 GNU/Linux`
- nproc: 4
- memory: MemTotal 16473776 kB (about 15.7 GiB). Cgroup limit: none found; `/sys/fs/cgroup/memory.max` does not
  exist on this box.
- disk: 30G free on the home filesystem at set-up.

## Versions, case and command

- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, `cargo-mutants 27.1.0` (installed with
  `cargo install cargo-mutants --locked --version 27.1.0`).
- Code: branch `cloud/spikefade-v34int-20261116` at commit `7a5428d30bdb8a5e0fec427848178c82907de2fb` (the branch tip).
  The JOB file's sha256 matched the one READY.txt names.
- Case A: the slice holds no `core` mutant. All 815 are `spikefade`: `side17.rs` 385, `split18.rs` 239, `v32.rs` 140,
  `ckpt.rs` 51. So `case-args.txt` is empty.
- The exact command (run from `~/mut`):
  `cargo mutants --workspace --shard 16/30 --jobs 2 --cargo-arg=--locked -o ~/mo -- -- --exact --skip <each of the 17 names in skips.txt>`
- Test timeout: auto-set by cargo-mutants to 20 s from its own baseline (15 s build + 3 s test). No build timeout.

## list_check

- MATCH. The whole list is 24,429 mutants with sha256 `3a0419510101562bd0ff85ad8a10e795dc7b80ce03995983698d46b01c15432a`;
  slice 16 is 815 mutants with sha256 `a48ee0ccbb4d1b21be39e73b2dac950adeee3e9843c59ade6e356f7e11816b43`. Both equal the
  table.

## Baseline (section: rc, wall)

- B1 (`cargo build --locked --workspace --tests --keep-going`): rc 101, 18,615 ms. Exactly the 6 expected compile
  failures, all `core`: its lib test and the test targets `banned_lists`, `claim_reader`, `decision_citations`,
  `findings`, `graph` (each an `include_str!` of a file this branch does not hold). No other target failed.
- B2a (costs, greeks, spikefade; no skips): rc 101, 6,008 ms. 691 passed, 12 failed, 3 ignored. The 12 failures are the
  12 engine names in `skips.txt`. The spikefade lib suite alone: 204 passed, 12 failed, 1 ignored (217 tests; the Mac's
  216 passed and 1 ignored).
- B2b (`core`'s 4 compilable targets; no skips): rc 101, 127 ms. 9 passed, 5 failed: the 5 `cost_invariants` names in
  `skips.txt`.
- B2 in all: 17 failed names, every one in `skips.txt`.
- B3a (with the skips): rc 0, 5,696 ms. 691 passed, 0 failed, 3 ignored.
- B3b (with the skips): rc 0, 66 ms. 9 passed, 0 failed.
- The failure count of the last B3 pair: 0.

## ENV-EXTRA

- None. Every B2 failure was already in `skips.txt`, so `skips.txt` is the JOB's 17 names unchanged.

## Exit codes and outcome

- Set-up jobs: toolchain exit 0, cargo-mutants install exit 0, baseline job exit 0.
- Main run: exit 2 (some missed). 815 tested in 54 min (2026-11-16T15:55:14Z to 2026-11-16T16:49:02Z):
  caught 350, missed 453, timeout 0, unviable 12.
- No core follow-up (case A; step 7 does nothing).
- Wall-clock re-check (step 8): 22 timing-only catches. Every one of the 22 was caught ONLY by
  `split18::tests::v32rb_split_o1_gates`. Re-run with `--jobs 1`: exit 2, 32 mutants tested (the 22 plus 10 extras),
  31 missed, 1 caught.
  - recheck_caught 0, recheck_missed 22: all 22 timing-only catches passed every test under `--jobs 1`, so they are
    missed. The last wave's slice 26 saw the same thing with this same test.
  - Corrected caught = 350 - 22 = 328. Corrected missed = 453 + 22 = 475. The official files stay as cargo-mutants
    wrote them.
  - recheck_extra 10: all 10 are `delete field ... from struct ... expression` mutants of the same 4 files, which
    cargo-mutants 27.1.0 does not filter with `-F`, as the JOB says. The one re-check catch is one of them
    (`split18.rs:91:28: delete field days from struct Self expression in SCal::from_days`, also caught in the main
    run). They belong to the re-check only.

## Anything odd

- `rustup toolchain install` also self-updated rustup itself (1.28.2 to 1.29.3). The toolchain stayed
  1.97.1 (8bab26f4f 2026-07-14), unchanged.
- The JOB's limit applies: a mutant that only a skipped test would catch counts as missed.
- Otherwise nothing odd: no timeouts, no hang, no progress stall. Progress commits were pushed at 16:08, 16:21, 16:34
  and 16:47 UTC.

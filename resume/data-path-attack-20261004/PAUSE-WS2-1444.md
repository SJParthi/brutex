# WS2 data-path: paused 14:44 UTC (weekly 75% brake)

- Branch: `claude/attack-data-pipeline-hgxmw9`. Pushed head: **66eb9eb**. Tree clean.
- Validated at 66eb9eb:
  - fmt and clippy on pull and api are clean;
  - pull and api tests pass as nobody (2539; a TOTP-scan failure introduced by 78a4642 is fixed in 66eb9eb);
  - gates 1, 1d, 10, 11, 12, 19, 27 and 27b pass.
  The last full static-gate run was on c5d972f: 28/0/2.
- Gate 18 pre-run (nobody, with a baseline; worktree /home/user/brutex-nb/mut at 66eb9eb):
  - greeks 29 caught / 0 missed and store 32 caught / 0 missed, both at e16ded7 (unchanged by 66eb9eb);
  - api: shard 0/8 running at pause;
  - pull: 8 shards not started.
  Current state:
```
greeks-all rc=0 31 mutants tested in 2m: 29 caught, 2 unviable
store-all rc=0 46 mutants tested in 22m: 32 caught, 14 unviable
api-0of8 so far: 6 caught, 0 missed
```
- Exact next step: run the remaining shards, one per background call
  (`/home/user/brutex-nb/mutants4.sh api k/8` for k=1..7, then `mutants4.sh pull k/8` for k=0..7).
  Read `/home/user/brutex-nb/out2/<crate>-<k>of8/mutants.out/missed.txt` and kill every survivor.
  Then round 8; rounds 4-7 each found defects.
- Table: RESULT-20261006.md.
- Pre-run result after the pause: `api-0of8 rc=0 9 mutants tested in 81m: 8 caught, 1 unviable`

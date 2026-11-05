STATUS: IN PROGRESS
ASSEMBLED: bbacf363b5d2f3be7a2c19a6423cacfe361fdb67

# BASE-COV-LIBS-LSA: the COV-LIBS assembly (3 Nov 2026 cloud wave)

TOKEN `BASE-COV-LIBS-LSA-20261103-1`. Branch `cloud/cov-libs-lsa-20261103`.

## Step 1: the stack (done)

All four shard branches were fetched; each origin tip equals the tip the brief wrote, so TIP = the written tip for all
four. FROM = the kit commit `eeec369be7dc00c8ca71de19a797641947100905` for all four. Every shard has code outside
`reports/` (`git diff --name-only <FROM> <TIP> -- . ':(exclude)reports'` prints 43, 42, 43 and 23 files), so none was
left out. The kit's `replay-minus-reports.sh` (sha256 `f223185d...`, as MANIFEST.md gives) ran four times, under
`nice -n 15`, in the order LS1, LS2, LS3, LS4:

| call | TIP | ONTO | NEWREF | result line |
|---|---|---|---|---|
| LS1 | `25d6eb50acf7f73f51926fee665e525dae9c47d2` | START | `refs/heads/stack/lsa-1` | `OK refs/heads/stack/lsa-1 = 2f51700d1fe03cdb26a2d1cd2557787abcf78a17 (20 commits replayed onto bd04fb299d23, 11 skipped as reports only; P1-P4 hold; 43 files in X)` |
| LS2 | `0630015e806e1fb83798c0b39acc9af75b87a565` | `stack/lsa-1` | `refs/heads/stack/lsa-2` | `OK refs/heads/stack/lsa-2 = 2c9a3036c1e34ce28d8b48564dd08b04e524f13a (54 commits replayed onto 2f51700d1fe0, 19 skipped as reports only; P1-P4 hold; 42 files in X)` |
| LS3 | `16a9e6c38d4165e334811ea0b841e350be6a1f42` | `stack/lsa-2` | `refs/heads/stack/lsa-3` | `OK refs/heads/stack/lsa-3 = dd62496f51bb1a30f940f1387bd3e92b0b4e6de8 (11 commits replayed onto 2c9a3036c1e3, 7 skipped as reports only; P1-P4 hold; 43 files in X)` |
| LS4 | `1872fde300490b6261299f7ca4d6b75c8fe9c202` | `stack/lsa-3` | `refs/heads/cloud/cov-libs-lsa-20261103` | `OK refs/heads/cloud/cov-libs-lsa-20261103 = bbacf363b5d2f3be7a2c19a6423cacfe361fdb67 (14 commits replayed onto dd62496f51bb, 4 skipped as reports only; P1-P4 hold; 23 files in X)` |

The `stack/lsa-*` refs are local only and were never pushed.

Shard report line 1: LS1 `STATUS: DONE`, LS2 `STATUS: BLOCKED OUTSIDE`, LS3 `STATUS: DONE`, LS4 `STATUS: BLOCKED OUTSIDE`.

## Next

Step 2 (the shards' BLOCKED OUTSIDE needs), then Route B on the ten crates whole.

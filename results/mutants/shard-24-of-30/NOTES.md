# Mutation testing: shard 24 of 30 (job MUT-24)

Labels follow README-CLOUD.md. MEASURED means a number from a named run on this box. EXACT means a count. No figure here is
a trading result.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`. The README-CLOUD.md
  section 4 manifest was checked here: 146 files, `c3722928...`. Code only: no market data, none of the 3 parameter tables.
- **Box:** a Linux x86_64 cloud container with 4 vCPU and 15 GB RAM. `rustc 1.97.1 (8bab26f4f 2026-07-14)`, cargo-mutants
  27.1.0.
- **Command:** `cargo mutants --workspace --shard 24/30 --jobs 2 --cargo-arg=--locked -- -- --exact --skip <14 tests>`. The
  14 skips are the same list as shards 0-3. `baseline.log` opens with the full command and the reason for each skip.
- **Scope (EXACT):** under the default `slice` sharding, shard 24 is lines 12529-13050 of the workspace's 15,639 mutants.
  All 522 are in `spikefade`: side17.rs 369, v32.rs 115, ckpt.rs 38. Each mutant ran against spikefade's 158 runnable tests.
- **Wall (MEASURED):** 2026-11-16 11:42:10Z to 12:22:18Z, so 40 min 8 s.
  - cargo-mutants' own baseline: 14.7 s build and 4.4 s test. It auto-set the test timeout to 23 s.
- **Exit code 2:** some mutants were missed.

## Result as cargo-mutants reported it (EXACT)

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| side17.rs | 84 | 283 | 0 | 2 | 369 |
| v32.rs | 35 | 80 | 0 | 0 | 115 |
| ckpt.rs | 9 | 26 | 0 | 3 | 38 |
| **all 3 files** | **128** | **389** | **0** | **5** | **522** |

## Read this before using the CAUGHT count: 40 of the 128 rest on a wall-clock test failing under load

`split18::tests::v32rb_split_o1_gates` asserts wall-clock percentiles: `close` p99 under 20 us, `account` p99 under 2 ms,
and a p50 ratio between 25 and 97 months.

In this run it was the **only** failing test for 40 of the 128 caught mutants (listed below). It also failed alongside
other tests for 10 more.

Three checks were run on this box after the shard. All are MEASURED, and none of their output is in this folder.

1. **The 40 against spikefade's other tests,** with the 14 skips plus this test skipped, at `--jobs 2`: 40 of 40 missed.
2. **The 40 against this test alone,** at `--jobs 1`, so no second job ran beside it: 40 of 40 missed. The test passed on
   every one of the mutated builds.
3. **The UNMUTATED suite, two copies at once.** This is the contention that `--jobs 2` creates.
   - This test failed in 5 of 16 runs. Each time it was the only failure, with `account p99` between 4.04 and 4.07 ms
     against the 2 ms limit.
   - Run alone 30 times, beside a 2-job cargo-mutants run, it passed all 30.

**So no test of this branch detects those 40 mutants:**
- functional catches: **88**;
- mutants that no test catches: **389 + 40 = 429**.

The same test will add false catches to any spikefade shard run at `--jobs 2`. Either of these removes them:
- skip the test (`--skip split18::tests::v32rb_split_o1_gates`);
- re-run its sole catches at `--jobs 1`, with only that test.

spikefade's other timing tests (`tests32r::v32r_o1_*`, `tests33f::v33f_day_major_step_sampled`) appear in no failure of
this shard.

Checks 1 and 2 selected the 40 with `-f` and anchored `--re` patterns. Those calls also picked up 5 struct-field-deletion
mutants that slip past cargo-mutants 27.1.0's `--re` and `--exclude-re` name filters. Those 5 are ignored here.

### The 40 (in `caught.txt`, caught only by `split18::tests::v32rb_split_o1_gates`)

```text
crates/spikefade/src/ckpt.rs:166:13: delete match arm "POS" in load
crates/spikefade/src/side17.rs:198:5: replace loss_at -> i64 with -1
crates/spikefade/src/side17.rs:205:68: replace - with / in loss_at
crates/spikefade/src/side17.rs:207:13: replace > with < in loss_at
crates/spikefade/src/side17.rs:226:42: replace / with % in side_trades
crates/spikefade/src/side17.rs:226:66: replace % with + in side_trades
crates/spikefade/src/side17.rs:238:105: replace + with * in side_trades
crates/spikefade/src/side17.rs:238:167: replace + with - in side_trades
crates/spikefade/src/side17.rs:280:42: replace - with + in CandSet::add
crates/spikefade/src/side17.rs:280:42: replace - with / in CandSet::add
crates/spikefade/src/side17.rs:299:80: replace > with < in CandSet::of_run
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (0, 0, 0, 1, 0, 0, -1)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (0, 0, 0, 1, 0, 0, 1)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (0, 0, 0, 1, 1, 0, 1)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (0, 0, 1, 0, 0, 0, 0)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (0, 0, 1, 1, 1, 1, 1)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (0, 1, 0, 0, 0, 0, -1)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (0, 1, 0, 0, 0, 0, 0)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (0, 1, 0, 0, 1, 0, -1)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (0, 1, 0, 1, 0, 0, -1)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (0, 1, 1, 0, 0, 0, 0)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (0, 1, 1, 0, 1, 0, 0)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (0, 1, 1, 1, 1, 0, -1)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (1, 0, 1, 0, 0, 1, 0)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (1, 0, 1, 1, 0, 0, -1)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (1, 1, 0, 1, 0, 0, -1)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (1, 1, 0, 1, 1, 1, 1)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (1, 1, 1, 0, 0, 1, 1)
crates/spikefade/src/side17.rs:379:5: replace a19_of -> (usize, usize, usize, usize, usize, usize, i64) with (1, 1, 1, 0, 1, 0, 0)
crates/spikefade/src/side17.rs:386:14: replace += with *= in a19_of
crates/spikefade/src/side17.rs:471:37: replace > with >= in combined
crates/spikefade/src/side17.rs:498:17: replace += with *= in combined
crates/spikefade/src/v32.rs:533:71: replace > with == in pick_tables_m
crates/spikefade/src/v32.rs:533:71: replace > with >= in pick_tables_m
crates/spikefade/src/v32.rs:542:73: replace + with - in pick_tables_m
crates/spikefade/src/v32.rs:580:95: delete ! in pick_tables_m
crates/spikefade/src/v32.rs:598:64: replace / with % in pick_tables_m
crates/spikefade/src/v32.rs:606:50: replace += with -= in pick_tables_m
crates/spikefade/src/v32.rs:609:43: replace > with < in pick_tables_m
crates/spikefade/src/v32.rs:689:42: replace + with - in portfolio
```

# Mutation shard 11/30: results (job MUT-11)

Code only. No market data, parameter table or package file was read, fetched or written. **EXACT** = a count.
**MEASURED** = a wall time taken on this box. No figure here is a trading result.

## Source and tools

- Branch `cloud/spikefade-shard-20261116`, commit `15fd4f4`, a single root commit.
- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`.
- `cargo-mutants 27.1.0`, installed with `cargo install cargo-mutants --locked`: the same version as shards 0-3, so the slices
  line up.
- The box: 4 vCPU (Intel Xeon @ 2.30GHz), 15 GiB RAM, Linux x86_64. zstd is not installed. Nothing was added to the box or
  the checkout.

## What shard 11/30 covers

- `cargo mutants --workspace --list` gives **15,639** mutants (EXACT), as shards 0 and 1 found.
- Default `slice` sharding, 0-indexed: `--shard 11/30` is list positions **5743-6264** (1-based), **522 mutants**, all in
  `crates/spikefade`:
  - `screen.rs`: 28, the last 28 of `cmd_aggcheck`'s 39. The slice boundary splits `screen.rs:849:15` (its `-=` variant is
    shard 10's last mutant, its `*=` variant shard 11's first).
  - `trade.rs`: 494, from `share_on` (line 40) to `charged_trip` (line 584). The boundary splits `trade.rs:584:22` (its `-`
    variant is shard 11's last mutant, its `*` variant shard 12's first).

## Baseline and exclusions (see `baseline.log`)

1. **The literal `cargo test --locked --workspace` does not compile**: rc 101 after 1.8 s (MEASURED), 0 tests ran. `core`'s
   lib unit-test target `include_str!`s `CLAUDE.md` and `docs/*.md` of the app repository.
2. **`cargo build --locked --workspace --tests --keep-going`**: exactly 6 test targets fail to compile, all in `core`, and all
   23 errors are "couldn't read" of app-repository files: the lib unit tests, `banned_lists`, `claim_reader`,
   `decision_citations`, `findings`, `graph`.
3. **Every compilable target, no exclusions, `--no-fail-fast`**: **658 passed, 14 failed, 3 ignored** (EXACT). Every failure
   needs a file this branch does not hold:
   - 7 read `crates/spikefade/data/EXITS-DYN-*.tsv` (README-CLOUD.md section 5);
   - 2 exec the Mac-only path `/opt/homebrew/bin/zstd`;
   - 4 read `docs/04-invariants.md`;
   - 1 requires `crates/spikefade/benches/ratio.rs`.
4. **With exactly those 14 skipped** (`-- --exact --skip <name>` x14): **658 passed, 0 failed, 3 ignored, 14 filtered out**.
   - The shard's own scope, `cargo test --locked -p spikefade -- --exact --skip <the 14>`: 158 passed, 0 failed, 1 ignored,
     9 filtered out, in 4.3 s and 3.8 s (MEASURED, two runs).
   - The 14 names are the same 14 that shards 0-3 skipped.

## The run: `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt`

```
cargo mutants --workspace --shard 11/30 --jobs 2 --cargo-arg=--locked -- -- --exact \
  --skip <each of the 14 names listed in baseline.log>
```

- cargo-mutants built and tested only the `spikefade` package for this shard, baseline included, so `core`'s 6
  non-compiling targets were never built.
- Its baseline passed (build 15 s, test 3 s). It auto-set the test timeout to 20 s, with no build timeout.
- **Result (EXACT, from `outcomes.json`): 522 tested: 249 caught, 265 missed, 0 timeout, 8 unviable.** Exit code 2 (missed
  mutants).
- **Wall (MEASURED): 34 min 37 s**, 11:38:22Z to 12:12:59Z on 2026-11-16. cargo-mutants' own stamps are 11:38:25Z to
  12:12:59Z.

| file | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| `crates/spikefade/src/screen.rs` | 2 | 26 | 0 | 0 | 28 |
| `crates/spikefade/src/trade.rs` | 247 | 239 | 0 | 8 | 494 |
| **both** | **249** | **265** | **0** | **8** | **522** |

## Read this before using the CAUGHT count

**23 of the 249 caught mutants were caught only by `split18::tests::v32rb_split_o1_gates`**, and that test is a wall-clock
gate:
- It times `account()` with `std::time::Instant` and asserts three bounds: close p99 < 20 us, account p99 < 2 ms, and p50
  growth from 25 to 97 months of at most 3x + 2 us.
- In a debug build, with two mutation jobs sharing 4 vCPU, it fails on timing alone. The logged failures read like
  "25 months: account p99 4050352 ns" and "the account's split cost grows with the months: p50 14872 ns at 25 vs 56251 ns
  at 97".
- **Two of the 23 cannot be anything but noise.** They are in `cmd_aggcheck`, which only the `aggcheck` CLI dispatch in
  `main.rs` reaches, so no test executes the mutated code.

**Follow-up.** These figures are NOT in `outcomes.json`; that run's output was kept outside the checkout.
- The same 23 were re-tested with `--skip split18::tests::v32rb_split_o1_gates` added to the 14.
- Its baseline passed. **All 23 were MISSED** (EXACT), in 260 s of wall time (MEASURED).
- So, judged by the functional tests alone: **226 caught, 288 missed, 0 timeout, 8 unviable** (derived: 249 - 23 and
  265 + 23).
- 11 more caught mutants also failed the timing test, but each failed at least one other test, so they stand.
- The MISSED count is not affected: a spurious failure can only turn a miss into a catch.

The 23 timing-only catches (each MISSED in the follow-up):

```
screen.rs:864:30: replace == with != in cmd_aggcheck
screen.rs:866:29: replace += with *= in cmd_aggcheck
trade.rs:220:5: replace gap_facts -> (u32, u32, u64, u64, u64, u64, bool) with (0, 0, 1, 0, 0, 1, true)
trade.rs:220:5: replace gap_facts -> (u32, u32, u64, u64, u64, u64, bool) with (0, 0, 1, 1, 0, 1, true)
trade.rs:220:5: replace gap_facts -> (u32, u32, u64, u64, u64, u64, bool) with (0, 1, 0, 0, 1, 0, false)
trade.rs:220:5: replace gap_facts -> (u32, u32, u64, u64, u64, u64, bool) with (1, 0, 0, 1, 0, 1, true)
trade.rs:220:5: replace gap_facts -> (u32, u32, u64, u64, u64, u64, bool) with (1, 0, 1, 0, 0, 0, true)
trade.rs:220:5: replace gap_facts -> (u32, u32, u64, u64, u64, u64, bool) with (1, 0, 1, 1, 1, 1, true)
trade.rs:220:5: replace gap_facts -> (u32, u32, u64, u64, u64, u64, bool) with (1, 1, 0, 0, 0, 1, false)
trade.rs:220:5: replace gap_facts -> (u32, u32, u64, u64, u64, u64, bool) with (1, 1, 0, 0, 1, 1, false)
trade.rs:220:5: replace gap_facts -> (u32, u32, u64, u64, u64, u64, bool) with (1, 1, 0, 1, 1, 0, false)
trade.rs:220:5: replace gap_facts -> (u32, u32, u64, u64, u64, u64, bool) with (1, 1, 1, 0, 0, 0, false)
trade.rs:228:16: replace += with *= in gap_facts
trade.rs:230:17: replace += with -= in gap_facts
trade.rs:232:52: replace < with == in gap_facts
trade.rs:239:54: replace < with > in gap_facts
trade.rs:239:54: replace < with <= in gap_facts
trade.rs:425:27: replace + with - in EvT::idx
trade.rs:425:32: replace * with + in EvT::idx
trade.rs:430:48: replace + with - in EvT::aidx
trade.rs:446:20: replace < with == in cand_of
trade.rs:483:47: replace / with % in rules_of
trade.rs:553:138: replace > with == in Trip::orders_a10
```

**Selection caveat for anyone repeating this on another shard.** In cargo-mutants 27.1.0 the workspace's 206
"delete field ... from struct ... expression" mutants ignore `-F` and `-E`: `-F '^NO_SUCH_MUTANT$' --list` still lists all
206. `-f` does narrow them.
- The follow-up therefore used `-f crates/spikefade/src/trade.rs -f crates/spikefade/src/screen.rs` plus 23 anchored `-F`
  names.
- That selected the 23 plus the 40 delete-field mutants of those two files. Only the 23 are reported here.
- The 6 of those 40 that belong to shard 11 (`trade.rs:569`) reproduced the main run's verdicts: 4 caught, 2 missed.

## Per function (EXACT, from `outcomes.json`; "timing-only" = of the caught, those caught only by the wall-clock test)

| function | total | caught | timing-only | missed | unviable |
|---|---:|---:|---:|---:|---:|
| `screen.rs` `cmd_aggcheck` | 28 | 2 | 2 | 26 | 0 |
| `share_on` | 6 | 5 | 0 | 1 | 0 |
| (outside any function, `trade.rs:59`) | 2 | 2 | 0 | 0 | 0 |
| `block_of` | 4 | 0 | 0 | 4 | 0 |
| `cap_mode_of` | 3 | 0 | 0 | 2 | 1 |
| `run_reading` | 1 | 0 | 0 | 1 | 0 |
| `reading_name` | 2 | 0 | 0 | 2 | 0 |
| `reading_short` | 2 | 0 | 0 | 2 | 0 |
| `prev_prints` | 18 | 16 | 0 | 2 | 0 |
| `DayTape::with_reading` | 6 | 4 | 0 | 1 | 1 |
| `DayTape::tape` | 3 | 2 | 0 | 0 | 1 |
| `DayTape::mode` | 1 | 0 | 0 | 0 | 1 |
| `DayTape::prev` | 4 | 0 | 0 | 4 | 0 |
| `DayTape::rows` | 2 | 0 | 0 | 2 | 0 |
| `DayTape::prev_any` | 6 | 0 | 0 | 6 | 0 |
| `gap_facts` | 170 | 15 | 15 | 154 | 1 |
| `dial` | 1 | 0 | 0 | 0 | 1 |
| `Refuse::code` | 2 | 2 | 0 | 0 | 0 |
| `side_key` | 12 | 9 | 0 | 3 | 0 |
| `EvT::cover_ok` | 9 | 9 | 0 | 0 | 0 |
| `cover_bits` | 21 | 21 | 0 | 0 | 0 |
| `EvT::idx` | 12 | 6 | 2 | 6 | 0 |
| `EvT::aidx` | 8 | 1 | 1 | 7 | 0 |
| `cut_off` | 4 | 4 | 0 | 0 | 0 |
| `cand_of` | 21 | 6 | 1 | 15 | 0 |
| `cand_anchor` | 12 | 10 | 0 | 2 | 0 |
| `cand_kind` | 4 | 0 | 0 | 3 | 1 |
| `rules_of` | 13 | 7 | 1 | 5 | 1 |
| `Trip::orders_a10` | 28 | 13 | 1 | 15 | 0 |
| `charges_orders_n` | 60 | 58 | 0 | 2 | 0 |
| `charged_trip` | 57 | 57 | 0 | 0 | 0 |

**Where the gaps are:**
- **`gap_facts`** (`trade.rs:219`): 170 mutants, 128 of them body replacements by each 0/1/true/false 7-tuple. Every one
  of its 15 catches was the timing test's, so no test in the suite notices any change to its output. Its callers are
  `report.rs` lines 1983 and 2347.
- **`cmd_aggcheck`** (`screen.rs:821`): reached only from the `aggcheck` CLI dispatch, and no test calls it.
- **No test detects any change to:** the `DayTape` accessors `prev`, `rows` and `prev_any`; `block_of`; `reading_name`,
  `reading_short` and `run_reading`. Every mutant of these is missed. Whether a test executes them was not checked.
- **Fully caught:** `charged_trip`, `cover_bits`, `EvT::cover_ok`, `cut_off` and `Refuse::code`.

## Every missed mutant (`missed.txt`, grouped by position, nothing dropped)

Each line is one position. Every replacement that survived there is listed, separated by commas.

- 90 lines name 147 mutants.
- One line counts the 118 `gap_facts` tuple replacements, which `missed.txt` lists one by one.
- 147 + 118 = 265.

- `screen.rs:849:15` cmd_aggcheck: `+=` -> `*=`
- `screen.rs:852:107` cmd_aggcheck: `&` -> `|`, `^`
- `screen.rs:852:111` cmd_aggcheck: `==` -> `!=`
- `screen.rs:854:30` cmd_aggcheck: `+=` -> `-=`, `*=`
- `screen.rs:859:32` cmd_aggcheck: `==` -> `!=`
- `screen.rs:860:30` cmd_aggcheck: `==` -> `!=`
- `screen.rs:861:22` cmd_aggcheck: `+=` -> `-=`, `*=`
- `screen.rs:862:20` cmd_aggcheck: `+=` -> `-=`, `*=`
- `screen.rs:863:21` cmd_aggcheck: `+=` -> `-=`, `*=`
- `screen.rs:863:37` cmd_aggcheck: `&&` -> `||`
- `screen.rs:864:36` cmd_aggcheck: `-` -> `+`, `/`
- `screen.rs:865:28` cmd_aggcheck: `+=` -> `-=`, `*=`
- `screen.rs:866:29` cmd_aggcheck: `+=` -> `-=`
- `screen.rs:866:45` cmd_aggcheck: `&&` -> `||`
- `screen.rs:870:41` cmd_aggcheck: `==` -> `!=`
- `screen.rs:870:67` cmd_aggcheck: `*` -> `+`, `/`
- `screen.rs:870:78` cmd_aggcheck: `/` -> `%`, `*`
- `trade.rs:40:5` share_on: body replaced with `true`
- `trade.rs:74:5` block_of: body replaced with `0`, `1`, `-1`
- `trade.rs:74:13` block_of: `==` -> `!=`
- `trade.rs:86:9` cap_mode_of: match arm `0` deleted
- `trade.rs:87:9` cap_mode_of: match arm `1` deleted
- `trade.rs:95:5` run_reading: body replaced with `Default::default()`
- `trade.rs:101:5` reading_name: body replaced with `""`, `"xyzzy"`
- `trade.rs:111:5` reading_short: body replaced with `""`, `"xyzzy"`
- `trade.rs:139:38` prev_prints: `>` -> `>=`
- `trade.rs:139:51` prev_prints: `>` -> `>=`
- `trade.rs:173:21` DayTape::with_reading: `-` -> `+`
- `trade.rs:199:9` DayTape::prev: body replaced with `Vec::leak(Vec::new())`, `Vec::leak(vec![0])`, `Vec::leak(vec![1])`, `Vec::leak(vec![-1])`
- `trade.rs:205:9` DayTape::rows: body replaced with `Vec::leak(Vec::new())`, `Vec::leak(vec![Default::default()])`
- `trade.rs:211:9` DayTape::prev_any: body replaced with `vec![]`, `vec![0]`, `vec![1]`, `vec![-1]`
- `trade.rs:211:99` DayTape::prev_any: `-` -> `+`, `/`
- `trade.rs:220:5` gap_facts: body replaced with a constant 7-tuple, 118 of the 128 combinations (each u32/u64 in {0, 1}, the bool in {true, false}); each is listed in missed.txt
- `trade.rs:227:47` gap_facts: `==` -> `!=`
- `trade.rs:228:16` gap_facts: `+=` -> `-=`
- `trade.rs:229:19` gap_facts: `+=` -> `-=`, `*=`
- `trade.rs:230:17` gap_facts: `+=` -> `*=`
- `trade.rs:231:20` gap_facts: `+=` -> `-=`, `*=`
- `trade.rs:232:52` gap_facts: `<` -> `>`, `<=`
- `trade.rs:233:93` gap_facts: `<` -> `==`, `>`, `<=`
- `trade.rs:233:106` gap_facts: `&&` -> `||`
- `trade.rs:233:115` gap_facts: `>` -> `==`, `<`, `>=`
- `trade.rs:233:153` gap_facts: `+` -> `-`, `*`
- `trade.rs:233:182` gap_facts: `-` -> `+`, `/`
- `trade.rs:235:23` gap_facts: `-` -> `+`, `/`
- `trade.rs:235:27` gap_facts: `>` -> `==`, `<`, `>=`
- `trade.rs:235:33` gap_facts: `*` -> `+`, `/`
- `trade.rs:237:20` gap_facts: `+=` -> `-=`, `*=`
- `trade.rs:238:23` gap_facts: `+=` -> `-=`, `*=`
- `trade.rs:239:22` gap_facts: `==` -> `!=`
- `trade.rs:239:27` gap_facts: `&&` -> `||`
- `trade.rs:239:41` gap_facts: `&&` -> `||`
- `trade.rs:239:54` gap_facts: `<` -> `==`
- `trade.rs:243:18` gap_facts: `==` -> `!=`
- `trade.rs:340:34` side_key: `|` -> `^`
- `trade.rs:340:55` side_key: `&` -> `^`
- `trade.rs:340:67` side_key: `|` -> `^`
- `trade.rs:425:32` EvT::idx: `*` -> `/`
- `trade.rs:425:48` EvT::idx: `+` -> `-`, `*`
- `trade.rs:425:53` EvT::idx: `*` -> `+`, `/`
- `trade.rs:425:61` EvT::idx: `+` -> `*`
- `trade.rs:430:9` EvT::aidx: body replaced with `0`, `1`
- `trade.rs:430:28` EvT::aidx: `+` -> `-`, `*`
- `trade.rs:430:32` EvT::aidx: `*` -> `+`, `/`
- `trade.rs:430:48` EvT::aidx: `+` -> `*`
- `trade.rs:446:20` cand_of: `<` -> `>`, `<=`
- `trade.rs:446:36` cand_of: `+` -> `-`, `*`
- `trade.rs:447:20` cand_of: `<` -> `==`, `>`, `<=`
- `trade.rs:447:36` cand_of: `+` -> `-`, `*`
- `trade.rs:449:19` cand_of: `||` -> `&&`
- `trade.rs:454:19` cand_of: `==` -> `!=`
- `trade.rs:455:11` cand_of: `+` -> `-`, `*`
- `trade.rs:457:11` cand_of: `+` -> `-`, `*`
- `trade.rs:464:21` cand_anchor: `<` -> `<=`
- `trade.rs:465:19` cand_anchor: `||` -> `&&`
- `trade.rs:472:9` cand_kind: match arm `0..= 4` deleted
- `trade.rs:473:9` cand_kind: match arm `5..= 8` deleted
- `trade.rs:474:9` cand_kind: match arm `9..= 12` deleted
- `trade.rs:482:46` rules_of: `+` -> `-`
- `trade.rs:483:43` rules_of: `*` -> `+`, `/`
- `trade.rs:483:47` rules_of: `/` -> `*`
- `trade.rs:484:63` rules_of: `*` -> `+`
- `trade.rs:553:47` Trip::orders_a10: `>` -> `==`, `<`, `>=`
- `trade.rs:553:77` Trip::orders_a10: `>` -> `==`, `<`, `>=`
- `trade.rs:553:82` Trip::orders_a10: `+` -> `-`, `*`
- `trade.rs:553:108` Trip::orders_a10: `>` -> `==`, `<`, `>=`
- `trade.rs:553:138` Trip::orders_a10: `>` -> `<`, `>=`
- `trade.rs:556:34` Trip::orders_a10: `+` -> `-`
- `trade.rs:557:37` Trip::orders_a10: `+` -> `*`
- `trade.rs:569:39` charges_orders_n: field `qty` deleted from the `crate::charges::Trip` literal
- `trade.rs:569:154` charges_orders_n: field `broker_orders` deleted from the `crate::charges::Trip` literal


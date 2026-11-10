# O1-ATTACK-REPORT — the constant-time claims, attacked at p99 and at extreme sizes

Cloud lane `cloud/o1p99-attack-20261110`, 10 Nov 2026, branched from
`origin/feat/db-fix` at `beaf2e3c4` (the 8 Nov tip). An independent attack:
**no product code is changed.** The branch adds benches, `#[ignore]`d
reproductions, eight `[[bench]]` entries and two documents; the findings are
for a local lane to fix.

## The answer first

Every claim in the attack set was re-timed per call, with p50 and p99 read at
1, 10², 10⁴ and 10⁶ of the size it claims to be flat in (or as far as a
fixture could reach in a run), and with adversarial inputs: worst-case keys,
very long keys and texts, both sides of a threshold, an unparseable tail, cold
handles and foreign writers.

* **The per-operation primitives `CLAUDE.md` §3 rule 4 names hold at p99 up to
  10⁶**: bar lookup by index (warm and cold) and time to row on indexed
  months, condition lookup, mask evaluation, k=1
  duplicate rejection and reserved result append. So do the identity guards in
  `core`, the manifest entry lookup at a million entries, the results ledger's
  read and duplicate probe, and the keyed `/logs.json`.
* **Seven failures at p99**, each reproduced by an `#[ignore]`d `finding_*`
  test that fails today:
  * **New:** **F-COSTS-01** (`LotTable::lot_on` costs the length of its key:
    65 ns → 339 µs at p99 from a 1-byte to a 1 MB key, 5,207×),
    **F-PULL-01** (`alerts::fit_text` walks and copies a whole text it cuts
    to 75 bytes: 57 ns → 3.9 ms, 67,775×), **F-PULL-02**
    (`fnowork::futures_plan` grows with the snapshots held: 118 µs → 2.0 ms
    from 1 to 1,000 daily snapshots, 15× to 20× over four runs, against
    C-MS-02's "must not grow").
  * **Known, re-measured here, still open:** **F-STORE-02** (a one-bar
    append onto a 10⁶-bar month has a p99 of about half a second, 665× to
    787× the 1-bar month's: the `.srt` sort index is rewritten whole every
    32nd commit, row H9 of the 16 Oct attack); **F-STORE-01** (time to row on
    a month under 1,024 bars bisects: 29.8 µs at p99 at 1,023 bars against
    273 ns at 1,024, 109× to 130×, `docs/06-limits.md` L-SC-13);
    **F-CLI-01** (`Results::refresh` after ONE foreign append costs the
    whole ledger: 91× at p99 at 10⁴ records, 2,360× at 10⁵; the cost is
    admitted by D-1560, the doc on `refresh` still says "O(new rows)");
    **F-API-01** (an unkeyed `/logs.json` filter reads up to 8 × 4 MiB per
    request: 2.4 ms → 199.5 ms at p99, 82×, flat only once the cap is reached;
    the 16 Oct attack's row P7).
* **One tail beyond p99:** `engine::keep::Best::offer` ("O(1) to refuse") has
  5 to 17 calls in 10⁶ over 100 µs, up to 7.0 ms, at a cap of 10⁵,
  consistent with its mask set rehashing out deleted slots inside `offer`;
  p99 is flat, the max is not.
* **Not a defect, and printed so it is not mistaken for one:** a random key in
  a 10⁶-entry manifest costs 11× a hot key at p99, while one bare random load
  in a table of that size grows 25× to 153× (the cache hierarchy, not the
  code).

What could not be tested is at the end, and it is not short: the disk-cold
path, the full HTTP router, concurrency, the operator's machine, and the fat-LTO
bench profile for `pull`, `cli` and `api`.

## What was added (no product code)

| file | what |
|---|---|
| `crates/{core,vocab,engine,store,costs,pull,cli,api}/benches/o1p99.rs` | one attack bench per crate, `harness = false`, `test = false`, the repo's bench style |
| `crates/*/benches/o1p99/harness.rs` | the shared p50/p99 harness, eight identical copies (a bench links only its own crate) |
| `crates/{store,cli,costs,pull,api}/tests/o1p99_findings.rs` | `#[ignore]`d `finding_*` reproductions, one per failure |
| the eight crates' `Cargo.toml` | a `[[bench]] name = "o1p99"` block, placed BEFORE `ratio` (see the gates note) |
| `O1-ATTACK-REPORT.md` | this report |
| `O1-ATTACK-INVENTORY.md` | step 1: every cost claim found (776 code rows, 211 document rows), with its bench or `none` |

Run them (`--profile dev` is how they were run here; see the limits):

```text
cargo bench --locked --profile dev -p <crate> --bench o1p99
cargo test  --locked -p <crate> --test o1p99_findings -- --ignored --nocapture
```

## How p99 was read

* **One sample** is `batch` calls between two `Instant` reads, divided by
  `batch`, less the clock's own cost (calibrated at start: 23 to 35 ns a pair
  on this host, depending on load). `batch` is 1 wherever a call costs well
  over the clock, so one slow call is one slow sample; calls near the clock are
  batched per size so the clock's jitter is divided rather than read as cost.
  A row whose sample is under 50 ns at some size says `UNDER-RESOLVED`.
* **Interleaved rounds.** Every size is measured in the same round, one after
  another, and the round is repeated (4 or 5 rounds; 25 to 2,000 samples a
  round a size, more where a call is cheap). Drift and a noisy neighbour land
  on every size alike.
* **p99 is the quietest round's p99**: the smallest of the per-round p99s (the
  sample at rank ⌈0.99 n⌉). A structural tail (a probe chain, a rehash, a
  rewrite, a scan, a cold read) recurs in every round and survives that
  minimum; a scheduler stall lands in one round and does not. The p99 pooled
  over all rounds and the max are printed beside it.
* **Verdict.** FLAT when the quietest-round p99 at **every** size is within
  **3.0×** of the smallest size's (the shared-CI ceiling in
  `docs/04-invariants.md`); GROWS otherwise. The peak over all sizes is gated,
  not only the largest, so a cost that rises and falls back (a bisection under
  a threshold, an index above it) is read at its top.
* **Physics is separated, not hidden.** A random access into 10⁶ entries
  misses the cache and a scan of 48 MB streams from memory. Where that is the
  input, the row is gated on a hot key set (the probe count), and a random-key
  row is printed beside a **memory floor** (one random 8-byte load in a buffer
  the size of the structure) or a fold of the same bytes; the gate is then on
  the cost in floors. A growth the floor shares is the machine; one it does not
  share is the code.
* **Bounded is not flat.** A cost that grows with size up to a fixed cap and
  then stops is reported as GROWS-to-a-cap, with the cap and the p99 there.
* **Host.** A 4-core x86_64 cloud VM, 15 GB RAM, Linux 6.18, rustc 1.97.1,
  every file under the system temporary directory (page cache warm).
  **Profile:** `cargo bench --profile dev` (opt-level 3, no LTO, debug
  assertions and overflow checks on). Ratios are the shape the gate reads; the
  absolute numbers are this host's, not the operator's M4 Pro, and not the
  fat-LTO bench profile's.

## Table 1 — every claim attacked

Cells are **p50 / quiet-round p99 per call** at each size (`fl` = in floors,
the cost divided by the same-size memory floor). Ratios are the quietest-round
p99 at the PEAK size over the smallest. **FLAT** = within 3.0×; **GROWS** =
failed (the finding's name follows); **printed** = shown beside a gated row,
never gated. Numbers are from the final consolidated run on 10 Nov 2026; most
rows were also run at least once before it, with the same verdict.

| row | where | claim | input, sizes | p50 / p99 per size | verdict |
|---|---|---|---|---|---|
| CORE-01 | `crates/core/src/vendor.rs:1565` `decode_master_row` | C-09: decode costs the same whether a field is 28 B or 4 MiB | `trading_symbol` of 1, 10², 10⁴, 10⁶ B (the 1-B row decodes; longer ones are refused by the width gate) | 1: 126 ns / 138 ns; 10²: 15 / 15 ns; 10⁴: 15 / 15 ns; 10⁶: 15 / 15 ns | **FLAT** (0.11×) |
| CORE-02 | `crates/core/src/universe.rs:4123` `of_equity` (guard `:4433`) | a string longer than any symbol is never hashed | argument of 1 … 10⁶ B starting `RELIANCE` | 1: 12 / 13 ns; 10²: 0.9 / 0.9 ns; 10⁴: 0.9 / 0.9 ns; 10⁶: 0.9 / 0.9 ns | **FLAT** (0.07×) |
| CORE-03 | `crates/core/src/symbol.rs:62` `Symbol::new` | refuses past 24 B before reading the bytes | 1 … 10⁶ B | 1: 8.0 / 8.1 ns; 10²: 1.2 / 1.3 ns; 10⁴: 1.2 / 1.2 ns; 10⁶: 1.2 / 1.8 ns | **FLAT** (0.22×) |
| CORE-04 | `crates/core/src/universe.rs:4433` `FNO_INDEX.position`, miss | layer 4: a miss is at most 12 probes (`docs/07-o1-architecture.md:42`) | an adversary's miss homed at the table's longest run (9 slots from slot 632: `ZQ509761`, 10 probes) against one homed at an empty slot (`ZQ247263`, 1 probe) | 1 probe: 3.5 / 3.6 ns; 10 probes: 11 / 12 ns | printed: bounded, 3.17× worst key over best (a bound, not a size) |
| VOCAB-01 | `crates/vocab/src/table.rs:1691` `index_of` (condition lookup) | a token longer than any name is refused before it is hashed | 1 … 10⁶ B | 1: 4.8 / 5.1 ns; 10²: 1.0 / 1.1 ns; 10⁴: 1.0 / 1.1 ns; 10⁶: 1.0 / 2.1 ns | **FLAT** (0.41×) |
| VOCAB-02 | `crates/vocab/src/mask.rs:144` `ConditionMask::hits` (mask evaluation) | C-V-01…03: the same six words whatever the candidate requires | k = 1, 100, 384 (the representation's ceiling) | 1: 1.4 / 1.6 ns; 100: 1.4 / 1.4 ns; 384: 1.4 / 1.4 ns | **FLAT** (0.88×) |
| ENGINE-01 | `crates/engine/src/column.rs:105` `Column::support` | C-E-01: the per-bar cost does not grow with the column | 1 … 10⁶ bars, per bar; gated in floors (a fold of the same rows) | raw 1: 2.7 / 3.1 ns; 10²: 1.2 / 1.2; 10⁴: 1.1 / 2.4; 10⁶: 3.0 / 3.7 ns · in floors 1.84 / 1.72 / 1.12 / 1.36 | **FLAT** (raw 1.18×, floors 0.74×) |
| ENGINE-02 | `crates/engine/src/lib.rs:141` `primitives::offer` | C-E-10: k=1 duplicate rejection does not grow with what was offered | re-offer of 64 held positions in turn, 1 … 10⁶ offered | 1: 8.9 / 9.3 ns; 10²: 9.1 / 9.5; 10⁴: 9.1 / 9.9; 10⁶: 9.8 / 10 ns | **FLAT** (1.09×) |
| ENGINE-03 | `crates/engine/src/lib.rs:149` `primitives::append` | C-E-11: a reserved append is flat in what is held | 1 … 10⁶ held, 64 pushes a sample | 1: 3.0 / 3.6 ns; 10²: 2.6 / 3.4; 10⁴: 2.7 / 3.6; 10⁶: 2.5 / 3.1 ns | **FLAT** (0.86×) |
| ENGINE-04 | `crates/engine/src/keep.rs:360` `keep::Best::offer`, refused | "O(1) to refuse" | full retention of cap 1, 10², 10⁴, 10⁵; gated in floors (one random load in the set's bytes) from cap 10⁴, where the floor is above the clock | raw 1: 48 / 69 ns; 10²: 48 / 100; 10⁴: 58 / 365; 10⁵: 113 / 468 ns · floor 12 / 14 / 348 / 412 ns | **FLAT** in floors (raw 6.8×, floor 34×: the cache) |
| ENGINE-05 | same | "O(1) to refuse", as a worst case | 10⁶ refusals in a row at cap 10⁵ | p50 106 ns, p99 494 ns, p99.9 708 ns, **max 3.05 ms; 6 calls over 100 µs** (17 and 5 in two earlier runs, max 2.9 and 7.0 ms) | p99 holds; **max fails** (see *Beyond p99*) |
| COSTS-01 | `crates/costs/src/lot.rs:1003` `LotTable::lot_on` | "bounded, whatever the table holds" | a miss whose key is 1, 10², 10⁴, 10⁶ B | 1: 30 / 65 ns; 10²: 49 / 62 ns; 10⁴: 2.3 / 2.6 µs; 10⁶: 268 / 339 µs | **GROWS 5,207×** → F-COSTS-01 |
| COSTS-02 | same, hits | flat across the table's underlyings | each compiled underlying in turn (the table holds 2) | 1: 26 / 29 ns; 2: 29 / 31 ns | **FLAT** (1.08×) |
| STORE-01 | `crates/store/src/file.rs:3477` `BarFile::read_record`, warm | C-28: bar lookup costs the same whatever the file holds | the last index again, months of 1, 10², 1,023, 1,024, 10⁴, 10⁶ one-second bars | 23-24 / 25 ns at every size | **FLAT** (1.00×) |
| STORE-02 | same, cold | C-BC-01: a cold lookup (one that verifies its block) is flat | a uniformly random index, 10² … 10⁶ bars (one block is never cold) | 10²: 39 ns / 3.0 µs; 1,023: 2.9 / 3.3 µs; 1,024: 2.9 / 3.4; 10⁴: 3.0 / 3.8; 10⁶: 3.6 / 4.9 µs | **FLAT** (1.65×) |
| STORE-03 | `crates/store/src/file.rs:3553` `first_at_or_after` | C-TIX-01: time to row costs the same whatever the file holds | a uniformly random stamp, 1 … 10⁶ bars, both sides of 1,024 | 1: 36 / 39 ns; 10²: 1.8 / 9.1 µs; **1,023: 15.1 / 29.8 µs**; 1,024: 219 / 273 ns; 10⁴: 224 / 266 ns; 10⁶: 244 / 609 ns | **GROWS 764×** at 1,023 (109× over 1,024) → F-STORE-01 |
| STORE-03 | same, indexed months | C-TIX-01 where it is benched | 64 stamps spread over the month, 1,024 … 10⁶ bars | 1,024: 213 / 234 ns; 10⁴: 213 / 229; 10⁶: 217 / 241 ns | **FLAT** (1.03×) |
| STORE-04 | `crates/store/src/file.rs:2105` `open_existing` + one lookup | an open reads a fixed header; no allocation that grows (S3) | a fresh read handle per sample, 1 … 10⁶ bars | 1: 13.4 / 29.8 µs; 10²: 16.8 / 33.5; 1,023: 27.9 / 46.9; 1,024: 14.1 / 29.2; 10⁴: 14.0 / 26.6; 10⁶: 14.2 / 30.7 µs | **FLAT** (1.58×, the 1,023-bar bisection) |
| STORE-05 | `crates/store/src/file.rs:3234` `BarFile::append`, one bar | C-07 (seal) and rule 4 | one-bar commits onto 1, 10², 10⁴, 10⁶ bars held | 1: 458 / 741 µs; 10²: 394 / 577 µs; 10⁴: 0.99 / 5.2 ms; **10⁶: 2.7 / 542 ms** | **GROWS 732×** → F-STORE-02 |
| PULL-01 | `crates/pull/src/fnowork.rs:746` `futures_plan` (+ `Book::open`) | C-MS-02 / R428-09: "its cost does not grow with the snapshots held" | 1, 10, 100, 1,000 DAILY snapshots of a 40-row dump | 1: 86.6 / 117.9 µs; 10: 101.5 / 133.3 µs; 100: 256 / 303 µs; **1,000: 1.8 / 2.0 ms** | **GROWS 16.9×** → F-PULL-02 |
| PULL-02 | `crates/pull/src/gdfl_dropbox.rs:1674` `status` | C-DP-02: "O(1) in the journal's length" (a 64 KiB tail) | journals of 1, 10², 10³, 10⁴, 10⁵, 10⁶ lines | 1: 2.6 / 5.3 µs; 10²: 7.8 / 19.1 µs; 10³: 39.9 / 56.5; 10⁴: 40.3 / 58.0; 10⁵: 39.0 / 56.2; 10⁶: 38.5 / 58.2 µs | **FLAT** once the window is full (1.03× from 10³); grows 11× up to it (printed) |
| PULL-02 | same, hostile tail | the window doubles to `MAX_STATUS_WINDOW` (4 MiB) | 1,000 good lines then 0, 70 KiB, 4 MiB this build cannot parse | 0: 40.9 / 59.3 µs; 70 KiB: 369 / 435 µs; 4 MiB: 20.2 / 31.5 ms | printed: grows 531× to the 4 MiB cap (bounded, documented) |
| PULL-03 | `crates/pull/src/alerts.rs:466` `fit_text` (inside `put_at`, `:936`) | rule 4 and law 5: keeps at most `TEXT_MAX` = 75 bytes | alert texts of 1, 10², 10⁴, 10⁶ B | 1: 28 / 57 ns; 10²: 415 / 692 ns; 10⁴: 26.2 / 40.7 µs; **10⁶: 2.7 / 3.9 ms** | **GROWS 67,775×** → F-PULL-01 |
| PULL-04 | `crates/pull/src/manifest.rs:3685` `Manifest::entry` | C-12: one probe whatever the census holds (benched to 10⁵, one hot key) | a loaded census of 1, 10², 10⁴, 10⁶ entries; 64 held keys in turn | 1: 61 / 81 ns; 10²: 63 / 82; 10⁴: 64 / 78; 10⁶: 67 / 80 ns | **FLAT** (1.01×) |
| PULL-04 | same, random key | (the cache, not the probe) | a uniformly random held key; beside it one random load in a table of 300 B an entry | 1: 70 / 87 ns; 10²: 72 / 142; 10⁴: 264 / 554; 10⁶: 606 / 981 ns · floor 25 / 67 / 414 / 633 ns | printed: 11.3× raw, the floor grows 25× (153× in an earlier run): physics |
| CLI-01 | `crates/cli/src/results.rs:1552` `Results::read` | "O(1) — HEADER + index · STRIDE, one seek" (its doc: UNVERIFIED as a measurement) | a uniformly random index, ledgers of 1, 10², 10³, 10⁴ records | 1: 1.6 / 2.0 µs; 10²: 1.6 / 2.0; 10³: 1.7 / 2.3; 10⁴: 1.9 / 2.6 µs | **FLAT** (1.29×) |
| CLI-02 | `crates/cli/src/results.rs:1199` `Results::holds`, hit | C-CLI-03: the duplicate check does not scan (benched only at the LAST record) | a random held identity, 1 … 10⁴ records | 1: 46 / 49 ns; 10²: 46 / 52; 10³: 46 / 75; 10⁴: 53 / 77 ns | **FLAT** (1.58×) |
| CLI-02 | same, miss | same | an identity no ledger holds | 1: 44 / 47 ns; 10²: 46 / 57; 10³: 45 / 53; 10⁴: 50 / 84 ns | **FLAT** (1.78×) |
| CLI-03 | `crates/cli/src/results.rs:1525` `Results::refresh`, unchanged | "constant metadata checks when unchanged" | nothing appended, 1 … 10⁴ records | 1: 946 ns / 1.1 µs; 10²: 948 ns / 1.1; 10³: 950 ns / 1.2; 10⁴: 944 ns / 991 ns | **FLAT** (1.05×) |
| CLI-04 | same, after a foreign append | "O(new rows)" | ONE record appended by another handle before each sample | 1: 82.9 / 86.1 µs; 10²: 155 / 159 µs; 10³: 0.81 / 0.93 ms; **10⁴: 7.6 / 7.8 ms** | **GROWS 91×** → F-CLI-01 |
| API-01 | `crates/api/src/logs.rs:6501` `logs_json_over`, keyed | C-LG-01: a keyed `/logs.json` is answered from the index (benched at 200 and 2,000 lines, median gated) | `run=<id>&durable=1&limit=20` over 200, 2,000, 20,000, 100,000 lines in each of the 8 log directories | 200: 443 / 580 µs; 2,000: 786 / 982 µs; 20,000: 804 / 982 µs; 100,000: 835 µs / 1.1 ms | **FLAT** (1.87×) |
| API-02 | same, unkeyed | `logs.rs:27-31`: "A request cannot make the server read the disk without a bound" | `run=1&limit=20` (stays the ring's filter; matches no line) over the same stores | 200: 2.4 / 2.4 ms; 2,000: 23.2 / 24.4 ms; **20,000: 183 / 199.5 ms**; 100,000: 183 / 193 ms | **GROWS 82×** up to the 8 × 4 MiB scan cap → F-API-01 |

## The failures, one at a time

Each is reproduced by the named `#[ignore]`d test, which asserts the flat
shape (3.0× between the two sizes it times) and fails on `beaf2e3c4`. Causes
marked *traced* were read in the source and not instrumented.

### F-COSTS-01 — `LotTable::lot_on` costs the length of its key (NEW)

* **Claim.** `crates/costs/src/lot.rs:1004-1009`, `lot_on`: "One hash probe
  and one binary search of at most `MAX_LOT_ROWS_PER_UNDERLYING` rows:
  bounded, whatever the table holds." Measured only in table size
  (`costs::lot::table_tests::a_lookup_is_flat_in_the_table`).
* **Input.** A miss whose key is 1, 10², 10⁴ and 10⁶ bytes.
* **Measured.** p50 / quiet-round p99: 1 B 30 / 65 ns; 10² B 49 / 62 ns; 10⁴ B 2.3 / 2.6 µs; 10⁶ B 268 / 339 µs. **5,207× at p99** (8,063× and 5,259× in two earlier runs), linear at about 0.3 ns a byte.
* **Cause** (traced). `rows_of` (`lot.rs:983`) hashes the whole `underlying`
  with the map's keyed SipHash, and a miss copies it whole into
  `LotAbsence::NoRow` (`underlying.to_owned()`, `lot.rs:1013`), before
  anything compares its length with `MAX_LOT_TEXT_BYTES` (2,048,
  `lot.rs:644`), the longest underlying a row can hold. This is the class
  D-0033 fixed in `core::universe::MemberIndex::position` (refuse a string
  longer than any member before hashing it).
* **Fix.** At the top of `lot_on` (and `rows_of`), answer a key longer than
  `MAX_LOT_TEXT_BYTES` as `NoRow` without hashing it, and put at most that many
  bytes (or the length) into the refusal. Reach today: no production code
  calls `lot_on` yet (its callers are tests; `pull::reference_data` builds
  the table), so this is a public function whose stated bound does not hold,
  and it goes live the day a caller forwards an outside string to it.
* **Reproduction.** `costs::o1p99_findings::finding_costs_lot_on_costs_the_length_of_its_key`
  (2,048-byte key → 10⁶: 525×).

### F-PULL-01 — `alerts::fit_text` is linear in a text it cuts to 75 bytes (NEW)

* **Claim.** `CLAUDE.md` §3 rule 4 (constant per-operation cost) and
  `docs/07-o1-architecture.md` law 5, "Bound every input at the boundary";
  `alerts::put_at`, which calls it under the queue's exclusive lock, claims
  one positional read "whatever the file's length" (`alerts.rs:11`).
* **Input.** Alert texts of 1, 10², 10⁴ and 10⁶ bytes.
* **Measured.** p50 / quiet-round p99: 1 B 28 / 57 ns; 10² B 415 / 692 ns; 10⁴ B 26.2 / 40.7 µs; 10⁶ B 2.7 / 3.9 ms. **67,775× at p99** (134,006× in an earlier run), about 4 ns a byte.
* **Cause** (traced). `fit_text` (`crates/pull/src/alerts.rs:466-480`) builds
  `clean: String` from EVERY char of the text, then cuts it to `TEXT_MAX`
  (75 bytes, `alerts.rs:56`). An alert's text can carry a vendor's error body.
* **Fix.** Clean lazily and stop at `TEXT_MAX` output bytes (take chars until
  the next one would pass the limit; set `cut` if any input remains), so the
  work is O(`TEXT_MAX`) whatever the input. The same pass should bound `key`
  before `key_hash` hashes it.
* **Reproduction.** `pull::o1p99_findings::finding_pull_alert_text_costs_its_whole_length_not_text_max`
  (`TEXT_MAX` → 10⁶ bytes: 10,463×).

### F-PULL-02 — `fnowork::futures_plan` grows with the snapshots held (NEW)

* **Claim.** C-MS-02 and invariant R428-09 (`docs/04-invariants.md:8645`):
  "A futures plan opens no snapshot but the newest, so its cost does not grow
  with the snapshots held"; `crates/pull/benches/ratio/master_snapshots.rs:146`.
* **Input.** 1, 10, 100 and 1,000 DAILY snapshots of a 40-row Kite dump; the
  book opened as a request opens it (`Book::open`), then the plan.
* **Measured.** p50 / quiet-round p99: 1 snapshot 86.6 / 117.9 µs; 10: 101.5 / 133.3 µs; 100: 256 / 303 µs; 1,000: 1.8 / 2.0 ms. **16.9× at p99** (15.6×, 20.1× and 14.8× in three other runs), about 2 µs a snapshot.
* **Why C-MS-02 passes.** It holds 1, 10 and 100 WEEKLY snapshots of a
  2,000-row dump and takes the minimum, so parsing the newest dump is most of
  every call and the per-snapshot work hides under it.
* **Cause** (traced). Two O(snapshots) steps per plan: `Book::open` lists the
  whole snapshot directory (`Catalog::read`, `crates/pull/src/master_snapshots.rs:411`),
  and `first_listed::listing` reads the whole absorbed log, one line per
  snapshot ever taken (`read_log`, `crates/pull/src/master_snapshots_first_listed.rs:482`),
  every time. About 2 µs a snapshot here; a book of daily snapshots over
  several years is thousands.
* **Fix.** Keep the catalog's newest stamp in a fixed-size pointer the
  snapshot writer updates (read one record, never list the directory), and
  key the first-listed index by underlying (or keep a per-underlying tail
  offset) so a plan reads that underlying's lines only. Then make C-MS-02 use
  a tiny dump and ≥ 1,000 snapshots so it can see a regression.
* **Reproduction.** `pull::o1p99_findings::finding_pull_futures_plan_grows_with_the_snapshots_held`
  (1 → 1,000 daily snapshots: 14.8×).

### F-STORE-02 — a one-bar append's p99 grows with the month (KNOWN: H9)

* **Claim.** C-07, "Sealing one block costs the same at 1×, 10× and 100× the
  file's record count", and `CLAUDE.md` §3 rule 4. C-07 times the seal step
  alone; nothing gates the whole append's tail.
* **Input.** One-bar `BarFile::append` onto one-second months already holding
  1, 10², 10⁴ and 10⁶ bars.
* **Measured.** p50 / quiet-round p99: 1 bar held 458 / 741 µs; 10² 394 / 577 µs; 10⁴ 0.99 / 5.2 ms; 10⁶ 2.7 / 542 ms. **732× at p99** (742×, 787× and 665× in three other runs). The p50 grows 5.8× too (not traced here; each commit also syncs and walks the `.srt`'s runs), but the p99 is the rewrite.
* **Cause.** `sort_index_writer.rs:226`: when the next run would reach
  `sort_index::COMPACT_AT` (32, `sort_index.rs:91`) runs, `Plan::Whole` →
  `write_whole` (`sort_index_writer.rs:337`) re-reads all n rows and sorts
  them six ways, O(n log n), inside the commit. The module doc states the
  cost (`sort_index_writer.rs:36-41`) and `docs/06-limits.md:19891` lists it
  as H9, "named, not fixed". At 10⁶ bars it is half a second every 32 commits,
  so ≥ 3% of one-bar commits are slow and the p99 is one of them.
* **Fix.** Take the whole rewrite off the commit path: append the run and
  leave compaction to a background/idle pass or to the next writer open
  (readers already handle up to `MAX_RUNS` = 64), or merge incrementally:
  each commit advances a pending merge by a fixed number of entries, so no
  single commit does month-sized work. (Tiered compaction alone lowers the
  average but still lands an occasional month-sized merge on one commit.)
* **Reproduction.** `store::o1p99_findings::finding_store_one_bar_append_p99_grows_with_the_month`
  (1 → 10⁶ bars held: 665×).

### F-STORE-01 — time to row bisects under 1,024 bars (KNOWN: L-SC-13, D-4933)

* **Claim.** C-TIX-01 (`docs/04-invariants.md:214`), "Time to row —
  `BarFile::first_at_or_after` through the `.tix` index — costs the same
  whatever the file holds", and `CLAUDE.md` §3 rule 4's bar lookup.
* **Input.** Random stamps in months of 1, 10², 1,023, 1,024, 10⁴ and 10⁶
  one-second bars.
* **Measured.** p50 / quiet-round p99: 1 bar 36 / 39 ns; 10² 1.8 / 9.1 µs; **1,023 15.1 / 29.8 µs**; 1,024 219 / 273 ns; 10⁴ 224 / 266 ns; 10⁶ 244 / 609 ns. 1,023 bars costs **109×** the 1,024-bar month at p99 (116× and about 130× in two other runs), 764× the 1-bar month. Above the threshold it holds: 64 spread stamps, 1,024 → 10⁶ bars, 1.03× (STORE-03, gated).
* **Cause.** A month under `time_index::INDEX_MIN_ROWS` (1,024) gets no
  `.tix` (the small-month rule, D-4933) and `locate` bisects
  (`crates/store/src/file.rs:3579-3596`): up to ten record reads, most in a
  different checksum block, each a block `pread` plus a CRC-32C. C-TIX-01's
  bench starts at 1,024 bars on purpose (`benches/ratio.rs:531-537`), so the
  claim is measured only where it holds. It matters because whole rungs live
  under the threshold: a day-rung month is ~22 bars, an hour-rung month ~150,
  a 15-minute month ~550, so every lookup on them bisects. (A month written
  before the index existed bisects too, on the legacy path, up to
  ⌈log2 n⌉ ≈ 21 reads at a one-second month: on a shadow of the operator's
  store 142 of 194 GDFL one-second months had no `.tix`,
  `crates/api/src/calendar_route.rs:32-33`. `BarFile::index_existing` closes
  that case; nothing closes the small-month one.)
* **Fix.** Give every month a `.tix` (a 1,023-bar month's index is a few
  entries), or, on a handle's first lookup of a small month, read its stamps
  once (≤ 1,023 × 8 bytes) and answer every later lookup from memory with no
  block verify.
* **Reproduction.** `store::o1p99_findings::finding_store_time_to_row_is_not_flat_below_the_index_threshold`
  (1,024 bars → 1,023: 116×).

### F-CLI-01 — `Results::refresh` after a foreign append costs the whole ledger (KNOWN cost, STALE claim)

* **Claim.** `crates/cli/src/results.rs:1525`, `Results::refresh`: "O(new
  rows), with constant metadata checks when unchanged" (the unchanged half
  holds: CLI-03 below).
* **Input.** A reader handle on ledgers of 1, 10², 10³ and 10⁴ records
  (10⁵ in the finding); before each sample a writer handle appends ONE record
  (outside the timer); the reader's `refresh` is timed.
* **Measured.** p50 / quiet-round p99: 1 record 82.9 / 86.1 µs; 10² 155 / 159 µs; 10³ 0.81 / 0.93 ms; 10⁴ 7.6 / 7.8 ms. **91× at p99** (122× in an earlier run), about 0.77 µs a record held: linear. The finding test at 10⁵ records: 40.6 µs → 95.7 ms, **2,360×**.
* **Cause.** On growth, `absorb_new_records` (`results.rs:1472`) calls
  `self.prefix.require_unchanged` (`results.rs:1499`), which re-reads and
  re-hashes bytes `0..covered`, every byte already indexed
  (`result_set.rs:663-675`, `hash_range`), before it decodes the new tail.
  `docs/06-limits.md` D-1560 (line 13444) admits exactly this, "O(indexed
  bytes), paid by `append`, `append_exact` and the read-side `refresh`
  alike"; the doc on `refresh` and the header of `results.rs` were not
  updated. The same branch makes two handles appending in turn pay O(n) per
  append (the inventory agent traced it; not timed here).
* **Fix.** Keep the indexed prefix's digest as a running (incremental) hash
  state in the handle and have the writer record a digest of the committed
  prefix in a header or sidecar it updates on append, so a reader verifies the
  old prefix by comparing a stored digest (O(1)) and hashes only the appended
  bytes. Until then, change the doc on `refresh` to the D-1560 wording.
* **Reproduction.** `cli::o1p99_findings::finding_cli_refresh_after_a_foreign_append_costs_the_whole_ledger`
  (1 → 10⁵ records: 2,360×).

### F-API-01 — an unkeyed `/logs.json` filter reads up to 8 × 4 MiB (KNOWN: P7)

* **Claim.** `crates/api/src/logs.rs:27-31`: "A request cannot make the
  server read the disk without a bound", and the ring answer is "constant …
  only the fallback reads the files". True as a BOUND; not flat.
* **Input.** `run=1&limit=20` (no `durable=1`, so it stays the ring's
  filter; no line has run 1, so it falls back to `telemetry::tail` in every
  directory) over 200, 2,000, 20,000 and 100,000 lines in each of the eight
  log directories, through `api::logs::logs_json_over` (what the route runs).
* **Measured.** p50 / quiet-round p99: 200 lines a directory 2.4 / 2.4 ms; 2,000 23.2 / 24.4 ms; 20,000 183 / 199.5 ms; 100,000 182.6 / 193.1 ms. **82× at p99** (81× in an earlier run), flat from about 16,000 lines a directory on: the cap. A cheap GET costs about 200 ms at p99 on a store whose logs are full.
* **Cause.** The fallback walks each directory back to `SCAN_BYTES`
  (4 MiB, `logs.rs:51`) looking for `limit` matches; a filter that matches
  little reads all of it, eight directories a request. The keyed form
  (`durable=1`) is answered from the index and holds (API-01).
* **Fix.** Answer `run=` (and target/level filters) from the durable index
  whether or not `durable=1` is set, or keep per-directory postings for the
  filterable fields; failing that, cut `SCAN_BYTES` for filter fallbacks to
  what fits the route's p99 budget and say "partial" in the answer (degrade
  loudly) rather than read 32 MiB inline.
* **Reproduction.** `api::o1p99_findings::finding_api_unkeyed_logs_filter_costs_the_log_size_up_to_the_scan_cap`
  (200 → 20,000 lines a directory: 83.6×).

### Beyond p99: `keep::Best::offer` refuses in O(1) except when it rehashes

* **Claim.** `crates/engine/src/keep.rs:360`, "O(1) to refuse", and that the
  mask set "never grows past its `cap + 1` reservation".
* **Measured.** 10⁶ refusals in a row into a full retention of cap 10⁵: p50 106 ns, p99 494 ns, p99.9 708 ns, **max 3.05 ms, 6 calls over 100 µs** (two earlier runs: 17 calls and max 2.9 ms; 5 calls and max 7.0 ms). One refused offer at p99 is flat in floors (ENGINE-04).
* **Cause** (traced, not instrumented). A refused offer is an insert into the
  `MaskSet` and a remove from it. In a set kept about three quarters full the
  removes leave deleted slots that do not return capacity; when the spare
  capacity is used up the table rehashes (in place, or grows), inside one
  `offer`. Whether it GROWS past the reservation is UNVERIFIED: capacity is not
  observable through the public API.
* **Fix.** Do not churn the hash set on a refusal: a candidate that ranks
  strictly below the weakest held itemset cannot equal a held one (every held
  itemset ranks at or above the root, and `ranks_below` compares hits and all
  six words), so it can be counted as discarded without touching the set,
  provided a repeated mask always carries the hits of its held copy (it does
  within one walk, where hits is the mask's support; a caller that could
  re-offer a mask with other hits would see it counted as discarded rather
  than repeated). Only a candidate that would be admitted needs the probe. Or use a table
  whose remove leaves no tombstone (backward-shift deletion), or rebuild the
  set off the hot path.
* **No `finding_*` test:** the failure is at max, not p99, and the ask was
  p99. ENGINE-05 prints it on every run.

### Bounded, documented, and still a large tail

* `gdfl_dropbox::status` with a tail this build cannot parse:
  59 µs at p99 with no junk, 435 µs with 70 KiB, 31.5 ms with 4 MiB (531×) (bounded by `MAX_STATUS_WINDOW`, 4 MiB; the doubling is in its
  doc). In journal LENGTH it holds (PULL-02).
* Two more costs on the one-bar append path, found by the inventory, admitted
  in the code, and not re-timed here: the month summary is rebuilt whole,
  O(n), on every commit that takes a finer-than-minute month back over half
  density, so a month oscillating at the line pays it every second commit
  (`crates/store/src/month_summary.rs:62-70`, "not amortised away"); and a
  bar after a long gap writes `gap / 64` `.tix` entries in that commit
  (`crates/store/src/time_index.rs`, `extend`; the 16 Oct attack's H18), up
  to about 41,850 entries (670 KB) for one bar at the end of a one-second
  month.
* The 16 Oct attack's other open rows (`docs/06-limits.md:19876-19891`: R11,
  R13, R25, R27, R28, C1, C6, C24, E1, E4, H20) and its R1-R30, C1-C29, P1-P16
  and E1-E19 route rows were read there and are UNVERIFIED as latencies; this
  lane did not re-time them (see the limits). The inventory agents traced the
  same shapes again on `beaf2e3c4` (import-journal deep copy on append behind
  a held `Arc`, the `/sources/day.json` memo emptied whole at 65,536 entries,
  `/chain.json` moving the facet index on the request thread, `/bars` sorting
  each vendor's census when `exchange`/`segment` is omitted).

## What could not be tested, and why

* **Most of the ~1,000 claims were inventoried, not timed.** Step 1 found 776
  code claim rows and 211 document rows (`O1-ATTACK-INVENTORY.md`). This lane
  timed the 32 rows of Table 1: the five `CLAUDE.md` §3 rule 4 primitives, the
  identity guards, and the claims the inventory marked most likely to fail.
  The rest keep their own benches (most of them a minimum of a few trials at
  1×/10×/100×, which hides a p99), and no `o1p99` pin was written for them.
  In particular **no `o1p99` bench exists for `telemetry`, `indicators`,
  `runner`, `greeks` or `lake`**; their claims are covered only by their
  `ratio` benches (C-T-01b is the one row there that gates a p99).
* **The full HTTP router was not driven.** `api` was attacked through its
  public functions (`logs_json_over`). Routes whose cost depends on state held
  across requests (a cache emptied whole, an `Arc` deep-copied on append, a
  census re-read on the request thread) need a served store with appends
  between requests; that fixture (`benches/ratio/served.rs`) was not extended
  here. Those rows stay UNVERIFIED as latencies, as the 16 Oct attack left
  them.
* **No concurrency.** Every number is one thread. Lock hold times under many
  clients (the `/logs.json` filter holds no lock but reads 32 MiB; the
  results ledger takes a shared lock across a whole-ledger hash) were not
  measured.
* **No disk-cold read.** Every store file had just been written, so the page
  cache was warm. Dropping it needs a privilege the bench does not take; a
  bar read past RAM is ~616× slower (`docs/07-o1-architecture.md:83`), and
  nothing here measures that path.
* **Sizes stopped short of 10⁶ where a fixture could not reach it in a run.**
  The results ledger: 10⁴ records in the bench and 10⁵ in the finding (each
  append syncs; 10⁶ would take minutes to build). Snapshot books: 1,000.
  Log directories: 100,000 lines each (≈ 200 MB in all). Mask width: k = 384
  is the representation's ceiling. Lot keys and alert texts: 10⁶ bytes.
* **The dev profile, not the bench profile.** All timings were taken with
  `cargo bench --profile dev` (opt-level 3, no LTO, debug assertions on): the
  real `[profile.bench]` (fat LTO, `codegen-units = 1`) builds one crate's
  bench for minutes, and the 16 Oct attack recorded `cli` being killed for
  memory under it on a host of the same size (4 cores, 15 GB). Ratios are what the verdicts read;
  an LTO build changes constants, not the shapes above. Under the real bench
  profile the five smaller crates' `o1p99` benches build (`core`, `vocab`,
  `engine`, `costs`, `store`: `cargo bench --no-run`, 35 s here); `pull`,
  `cli` and `api` were built and run in the dev profile only.
* **Not the operator's machine.** A 4-core shared cloud VM, not the M4 Pro
  the documents quote; the quietest-round p99 is robust to a noisy neighbour,
  not immune to one.
* **Causes are traced, not profiled.** The cause lines name the code the
  inventory and this lane read; no profiler or allocation counter was run (an
  allocation counter needs `unsafe`, which the workspace denies).
* **`CLAUDE.md` §9 was not run in full.** `cargo fmt` and `cargo clippy -D
  warnings` are clean on every new file; `cargo test --workspace --locked
  --no-run` builds all 183 test binaries, the five new ones included (the
  findings are `#[ignore]`d, so `cargo test` stays green on them: run without
  `--ignored`, `store` and `costs` report 2 and 1 ignored, 0 failed);
  coverage, mutants and `cargo deny` were not run, and no CI gate ran (no pull
  request, as asked).

## Notes for whoever lands this

* **Gate 8 will run the new benches.** It runs `cargo bench --workspace`, and
  every `o1p99` target is a normal bench (`test = false`). Measured run times
  here (the final run's exit times): `core`, `vocab` and `costs` 1 s or less,
  `engine` 6 s, `store` 11 s, `pull` 34 s; `cli` 112 s and `api` 257 s, each
  including a rebuild of `cli` and `api` that a CI build would not repeat
  (their measuring phases are a few seconds, and about a minute for `api`,
  which writes about 200 MB of log fixtures). The gated rows passed on this host; the
  failing claims are printed, not gated, and live in the `finding_*` tests.
* **Gate 14's manifest reader** keeps the last of two adjacent `[[bench]]`
  blocks (`.github/gates_bounds.rs`, `bench_decls`), so each `o1p99` block is
  declared BEFORE `ratio`, never after it.
* **Gate 1d will name four new words.** It refuses any undeclared
  `[a-z0-9_-]` string literal under `crates/pull`, attributes included, and
  the `pull` bench and finding test add `o1p99` (the `#[path]` to the harness
  and a snapshot URL), `benches` (the test's `#[path]`) and the harness's
  `ns` and `floors`. The gate is already red on `beaf2e3c4` (about 370
  undeclared literals, `docs/06-limits.md` L-BG11-04), so it was left
  alone: declare the four in `.github/gates_tree.rs` `DECLARED`, or move the
  harness to a directory name with a capital or a dot.
* **`cli`'s build script stamps the working tree**, so any change to a tracked
  or untracked file rebuilds `cli` and `api` (about 3 to 4 minutes a time
  here); run the `api` and `cli` benches last.
* **Un-ignore a `finding_*` test when its fix lands**; it is then the
  regression guard for that claim, and its bench row can move from printed to
  gated.

## How to rerun

```text
cargo bench --locked --profile dev -p core   --bench o1p99
cargo bench --locked --profile dev -p vocab  --bench o1p99
cargo bench --locked --profile dev -p engine --bench o1p99
cargo bench --locked --profile dev -p costs  --bench o1p99
cargo bench --locked --profile dev -p store  --bench o1p99
cargo bench --locked --profile dev -p pull   --bench o1p99
cargo bench --locked --profile dev -p cli    --bench o1p99
cargo bench --locked --profile dev -p api    --bench o1p99
cargo test  --locked -p store --test o1p99_findings -- --ignored --nocapture --test-threads 1
cargo test  --locked -p costs --test o1p99_findings -- --ignored --nocapture
cargo test  --locked -p pull  --test o1p99_findings -- --ignored --nocapture --test-threads 1
cargo test  --locked -p cli   --test o1p99_findings -- --ignored --nocapture
cargo test  --locked -p api   --test o1p99_findings -- --ignored --nocapture
```

Every measured line starts `O1P99|<row>|<scenario>|n=<size>|p50=…|p99=…|quiet-p99=…|max=…|samples=…`
and every row ends with one `VERDICT` line, so a run greps straight into a
table. A bench exits non-zero only when a GATED row grows; the
`finding_*` tests fail today by design.

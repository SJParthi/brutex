so1 (o1-p99) at 9b0614be: 65 paths inventoried; 6 NEW (1 medium, 5 low), 5 FIXED, 54 DOCUMENTED; 0 NOT-FIXED; rule-4 ops: 4 of 5 have a p99 row, mask evaluation has none.

Machine table: `/tmp/claude-0/audit/out/so1.tsv` (set `o1-p99`, 65 rows). Each `evidence` cell starts with `class=`, usually one of O1-measured-p99, O1-no-p99, bounded-constant, inherent-documented-measured, inherent-documented-UNMEASURED or UNDOCUMENTED. A few rows carry narrower tags: amortised-documented, inherent-documented (vendor-bound), bounded-documented-UNMEASURED, gate-robustness and bench-hygiene.

**What I measured, and how.** All timings below were taken on 2026-10-08 on the shared 4-core box: Intel Xeon @ 2.10 GHz, 2 MiB L2 per core, 260 MiB L3 shared by the VM host. Load average was 3.5 to 10 because other workers were building at the same time. There were two kinds of run:
- `cargo bench -p engine --bench ratio` (four runs) and `cargo bench -p store --bench ratio` (one run), in the release bench profile.
- Temporary probe tests in the `test` profile (opt-level 3, no LTO), run and then deleted: `crates/{engine,core,lake}/tests/zz_audit_so1_{1,2,3}.rs`.

Each p99 is the smallest p99 over 5 rounds, the same method the O1P rows use. `git status --porcelain` shows nothing of mine.

## 1. The five rule-4 operations (CLAUDE.md §3 rule 4)

| Op | Live code (quoted) | O(1)? | Repo measurement | p99 in repo? | so1 measurement (p50 / p99 / max) |
|---|---|---|---|---|---|
| Bar lookup | `store/src/file.rs:2586` `read_record` → `:2988 read_row`: `self.layout.offset_of(index)` + `read_verified` (one cached block, else one block verify) | yes | C-28/29, C-BC-01/02, **O1P-01** `store/benches/ratio.rs:661` | **yes**, gated 10^3..10^6 | Bench under load 10: O1P-01 p99 6,837 / 6,546 / 4,707 / 8,077 ns at 10^3..10^6 (1.18×, ok). O1P-02 (`.tix`) p99 430 / 337 / 367 / 356 ns (0.83×, ok) |
| Condition lookup | `vocab/src/table.rs:1587` `TABLE.get(usize::from(index))`; `is_live` :1762 | yes | C-V-06 `vocab/benches/ratio.rs:204`, **fixed** positions, min of mean | **no** (06-limits:16070 names it as a mean) | Per 32 lookups at random positions 0..434: `is_live` 234 / 320 / 1.2 ms; `definition` 146 / 148 ns. At fixed position 369: 60 / 62 ns. A random position costs about 4× a fixed one, which is branch prediction, not a scan |
| Mask evaluation | `vocab/src/mask.rs:144` `hits`, six `(a&c)^c` terms and one OR; `engine/src/column.rs:105` `self.rows.iter().fold(.. row.hits(candidate))` | yes, in operations | C-E-01/02/03/09, min of mean | **no** | Flat in candidate width (p99 per bar: k=1 2,514 ps, k=8 3,115 ps, k=384 2,513 ps). **Not flat in column length**: see so1-1 |
| Duplicate rejection | `engine/src/lib.rs:141` `offered.insert(position)`, pre-sized at :1521 `offered.try_reserve(live.len())` | expected O(1) | C-E-10, **O1P-03** | **yes**, gated 10^4 and 10^5 | Four bench runs: 1.06–1.28× at 10^4, 1.12–1.77× at 10^5, 2.57–**7.66×** at 10^6. 10^6 is printed but not gated, and production never holds more than 384 entries |
| Result append | `engine/src/lib.rs:149` `out.push(item)`. `drain` reserves at :2384 `out.try_reserve(batch.len())` and then pushes at :2403. k=1 pushes at :1569 | amortised; O(1) inside the reservation | C-E-11, **O1P-04** | **yes**, gated 10^3..10^6 | Typical runs 0.97–1.35×. **One of four runs was red with no code change**: see so1-3 |

## 2. Every path inventoried (summary; the full rows are in the .tsv)

| Class | Rows | Row ids |
|---|---|---|
| O1-measured-p99 | 7 | rule4-bar-lookup, rule4-time-lookup-tix, rule4-dup-reject, rule4-result-append, pull-manifest-entry (gated to 10^4), telemetry-emit (C-T-01b), telemetry-tail20 |
| O1-no-p99 | 7 | rule4-condition-lookup, rule4-mask-eval-width, engine-join-pair, core-vendor-decode, api-instruments-page, api-dashboard, p99-absent-benches (9 crates' benches have no p99 row) |
| bounded-constant (bound stated) | 6 | vocab-name-index (≤4/7 probes), engine-subset-prune (k≤384), core-universe-membership (≤24 B, ≤8/12 probes × 6 tables), store-checksum-block (4 KB), api-bounded-routes (`/vocab.json` 370 rows, `/masters/status.json` 4 stats, `/pull/recovery.json` seek to ≤100, `/pull/run.json` ≤5 feeds), api-autopilot-json (≤2×months per feed, untimed) |
| amortised (documented) | 1 | pull-manifest-append (§23) |
| inherent-documented-measured | 13 | store-time-lookup-bisection, engine-sweep-total, store-tix-rebuild, store-committed-cash-days, pull-read-census-per-body, pull-json-decode, telemetry-filtered-tail, api-instruments-search, api-census-now, api-health, api-run-json-poll, api-audit-page-fsync, cli-screen-select |
| inherent-documented (vendor-bound) | 2 | pull-vendor-pull, api-pull-posts |
| inherent-documented-UNMEASURED | 23 | vocab-expression-evaluate, vocab-cursor-advance, engine-level-sort, store-append, pull-held-series, lake-open-decode, api-json-memos, api-verify-json, api-bars-window-sorted, api-bars-json-past-end, api-logs, api-boolean-campaign, api-boolean-evidence-cold, api-candidate-trades, api-engine-top-refusal, api-index-stop-cold, api-trades-json, api-assets, api-universe-resolve, cli-results-ledger, cli-results-top, cli-span-reloads, cli-research-replays |
| NEW | 6 | so1-1 … so1-6 (four UNDOCUMENTED cost or p99 gaps, one flaky gate, one bench leak) |

Coverage note: the L2 lens had already checked all 58 GET routes. I also read every POST handler (`/pull/*`, `/autopilot/*`, `/universe/resolve`, `/ingest/queue`, `/masters/refresh`, `/backtest/*`, `/engine/command`) and the fallback route that serves static assets. I did not open every command in `cli` (318k lines) one by one. For `cli` I relied on the per-command sections of `docs/06-limits.md` (o1cli-2..6, D-1400, D-1560, D-1631..D-1642, D-1681, D-1683, D-1729) and checked only their headline claims.

## 3. NEW findings

| id | severity | crate | file:line | what is wrong in plain words | evidence | how to fix |
|---|---|---|---|---|---|---|
| so1-1 | medium | engine | `crates/engine/src/column.rs:105`; `docs/06-limits.md:16070-16073`; `docs/04-invariants.md:2088` | Mask evaluation is the hottest rule-4 operation, and it has no p99 row. The limits file justifies that with "no size that grows, so there is nothing for a 10^3 → 10^6 sweep to vary". That is false: the per-bar time does grow with column length once the column leaves L2 (48 B per row). The recorded C-E-01 figure, "0.663× (cheaper)", called "empirical noise", was taken on an M4 Pro. On this box the effect is a systematic step. Production spans reach about 6×10^5 bars (`docs/07-o1-architecture.md:209`, 618,296 NIFTY 1-minute bars; spans are built by `stored::load_span` → `Column::build`). | Probe, three runs, per-bar p50 1.20 ns at 10^3 → 2.08–2.43 ns at 10^6. p99 ratio 10^6/10^3 = 1.87×, 1.80×, **3.04×**. `cargo bench -p engine`, C-E-01 10^4→10^6: 1.896×, 1.998×, 1.827×, **2.710×**, and 10^4→10^5 about 1.51× every time | (a) Add an O1P row for `Column::support` per bar at 10^3..10^6. Gate it where the column fits in L2 and print the larger sizes, as O1P-03 and O1P-05 do. (b) Correct 06-limits:16073 and the C-E-01 note to say "flat in operations, not in time past the cache". (c) It could also be made flat and about 2× faster: `drain` makes every lane re-stream the whole column once per candidate (`column.support(mask)` per mask). Cache-block it instead: for each chunk of about 4k rows (about 190 KB, L2-resident), loop over all of the lane's candidates and accumulate their counts |
| so1-2 | low | lake | `crates/lake/src/batch.rs:73`; `crates/lake/benches/ratio.rs:173`; `docs/06-limits.md:16271-16272` | The limits file says `Batch::row` "is an index into a decoded batch, which C-L-01 measures". C-L-01 only re-reads index 0 and the last index. Those stay in cache, which is the blind spot D-3307 found in C-12. At random indices the row read, which touches 7 columns, is not flat in time. Mitigating: no crate depends on `lake` (no `Cargo.toml` names it), so no request or command pays this today | Probe, per 32 random rows, p99 against 2,480 rows: 24,800 rows 1.2–3.4×; 248,000 rows (C-L-01's own LARGE size) **3.0×, 6.0×, 6.0×**; 2,480,000 rows 13–37×. p50 240 → 1,000 ns at 248,000 rows | Add a random-index p99 row (O1P-07) that gates the cache-resident size and prints the rest, and reword 06-limits:16271 to "flat in probes, not in time past the cache". Or state that `lake` has no production caller and drop the claim |
| so1-3 | low | engine (bench) | `crates/engine/benches/ratio.rs:558`, `:590-646` | O1P-04 is a two-sided gate (`up.max(down) <= CEILING_PERMILLE`) against **one** 10^3 baseline taken over a window of about 50 ms. If that window is slow, the gate fails on unchanged code. The bench's own reasoning for not gating O1P-03 at 10^6 ("a red build a matter of luck") applies here | Run 1 of 4: O1P-04 at n=1000 gave p99 11,458 ns (normal is 3,469–3,593 ns), so n=10000 read **0.309× BREACH** and the bench exited 1. The other three runs were green | Interleave the rounds across sizes, so that a burst of load lands on every size and not only on the baseline. Or compare each size's median-of-rounds p99. Or make the downward direction a warning rather than a failure |
| so1-4 | low | api | `crates/api/src/livejson.rs:114`; `crates/cli/src/live.rs:845-849`, `:912-966` | `/live.json`, which the backtest page polls, does a `read_dir` of up to 4,096 entries on every request. On top of that it makes one `stat` per live file (up to 128), clones each cached run (up to 256 rows) under one global mutex, and renders up to 128×256 rows with no `MAX_RESPONSE_BYTES` cap. That is bounded only by constants in the code. `docs/06-limits.md` names none of `LIVE_ENTRY_LIMIT`, `LIVE_RUN_LIMIT` or `LIVE_ROW_LIMIT`, and states no cost for this route | Code read. `grep LIVE_ docs/06-limits.md` returns nothing | Add a 06-limits entry that states the bound: O(entries ≤ 4,096) stats per poll and O(≤ 32,768 rows) copied and rendered. Consider handing out `Arc<Run>` instead of cloning |
| so1-5 | low | api | `crates/api/src/mastersrun.rs:379` → `crates/api/src/server.rs:5693` `reparse` → `:5701` `universe(masters)` → `Read::new` (`Catalog::build`, `constituents::Join::build`, `coverage::Coverage::build`); `docs/06-limits.md:13908-13911` | Every press of `POST /masters/refresh` re-parses all four masters and rebuilds the catalog, the join and the coverage, including their sorts, on the request path. The Gate 11 reasons in 06-limits still say "ALL THREE SORTS ARE STARTUP-ONLY. Not one is reachable from an HTTP handler … once per process", which has been stale since the in-place reload (D-1762). The only cost figure on record is the startup catalog build, 41 ms, ungated (06-limits:2033) | Code read of the call chain quoted at left. `grep -n reparse docs/06-limits.md` finds only the line about the memo being dropped | Correct the Gate 11 reason text and add a 06-limits entry: O(master bytes + U log U) per press, run under `reload_lock` and not timed. Optionally time the reparse in the api bench |
| so1-6 | low | store (bench) | `crates/store/benches/ratio.rs:142-143` | `loaded()` removes its temp root only **before** it creates the root, and the name includes the PID, so every `cargo bench -p store` leaves its month files behind in `$TMPDIR` | One run with TMPDIR set to my scratch directory left 16 `brutex-bench-*-7248` directories, 81 MB in total. I deleted them | Remove each root once its row is done, or with a drop guard at the end of `main` |

## 4. Round-2 items from `PAUSE-o1-p99.md`, now run

| Item | Status at 9b0614be | so1 |
|---|---|---|
| lake `Batch::row` p99 | no row in the repo; the doc says C-L-01 covers it | measured: not flat (so1-2) |
| pull manifest entry lookup p99 | O1P-05 landed (D-3307) | not rerun; the row is in the repo |
| core universe membership p99 | no row; the doc says there is no n to sweep | measured. Per 32 lookups: hit p99 4.3–6.8 µs, 24-byte miss p99 8.6–9.3 µs, 4 KiB string refused unhashed 0.10 µs. Bounded (`universe.rs:4452` length guard) |
| telemetry tail at p99 | O1P-06 landed (D-3306) | not rerun; the row is in the repo |

## 5. Edge cases tried against the FIXED rows

The adversarial case was load: I ran the O1P rows at load average 3.5 to 10.
- O1P-01 and O1P-02 stayed green.
- O1P-03 at 10^6 rose to 7.66× (not gated).
- O1P-04 went red once (so1-3).

For the k=1 table I checked whether production can grow past the size the gate covers. It cannot: `try_reserve(live.len())` with `live ≤ COUNT` (370), and `ConditionMask::BITS` = 384. That matches D-3301.

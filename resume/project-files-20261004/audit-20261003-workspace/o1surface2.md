# o1surface2: O(1) audit of `crates/api` and `crates/cli` at 1087e54

**Verdict.** None of the five `CLAUDE.md` §3 rule-4 primitives lives in `api` or `cli`, and nothing found here breaks one. Since bc53131 almost every NOT-O(1) row from the prior pass (prior/o1api.md, prior/o1cli.md) has been either fixed or written into `docs/06-limits.md`, and most of those limits entries are held to the code by source-reading tests. These were fixed: o1api-21, the slowloris (probeapi-1), o1api-26, o1api-4/-3 (now capped), and the elite-descent half of o1cli-1. These are now DOCUMENTED: W1-api5-1..11, the D-1444 JSON routes, and o1cli-2..6.

What is still open:
- **o1cli-1, plain `descend` half (medium).** D-0997 fixed only `cli elite`. Plain `cli descend` still runs a full `one_rung` on every support step: two span loads plus a column build per step. Nothing documents that per-step cost.
- **`latest_for` (KNOWN W2-cli8-4, not fixed).** It still opens the whole ledger on every rung. I measured it: Results::open ratio **14.13x** for 10x runs, and even the "ordinary O(1)" newest-row match grows about 10.9x.
- **Two small NEW items in api:**
  - the autopilot `tick` reads every vendor's whole manifest twice per tick, inline on an async worker, though it uses one vendor's;
  - the pull's ingest (`land_spot`, which leads to `pull::ingest::from_window`) runs synchronous store I/O on Tokio workers. The D-1443 residual list does not mention it.

I ran two probes (api and cli), recorded their output below, and deleted them. `git status --porcelain` shows no file of mine.

## Findings

| id | sev | crate | file:line | what is wrong | evidence | status |
|---|---|---|---|---|---|---|
| o1surface2-1 | medium | cli | lib.rs:14858 (`descend`), :14934-14949 | Every step after the first calls `one_rung` from scratch. Each step reloads the signal span (one_rung's own load), the kernel reloads it again and rebuilds the anchored column, and it reloads the 1-min execution span and both contexts. None of that depends on the support. D-0997 added `ScreenCache` to `cli elite` only. Its decision says "`cli elite` walks one span down a ladder"; `descend` is not touched. | `for (step, &support) in ladder.iter().enumerate() { let row = if step == 0 { first.outcome.clone() } else { one_rung(vendor_word, underlying, known, from, to, Some(support), None,) .outcome };` (lib.rs:14934-14949). `ScreenCache` is used only by `screen_range_kernel_cached` (lib.rs:15774, 15889). The o1cli-2 limits entry states two loads **per rung**, not per descend step. | KNOWN: prior o1cli-1 (named `descend` at old :14567). PARTIALLY FIXED: elite half fixed by D-0997, descend half NOT FIXED and not documented per step. Fix: pass a `ScreenCache`-style held input set (`AuditOptions::prepared_column`) through `one_rung` for the descend ladder. |
| o1surface2-2 | low | api | autopilot.rs:3443, 3487 (`tick`) | One autopilot tick calls `census::read_all` twice, unconditionally. Each call reads every vendor's whole manifest (each up to 268,468,224 B) and CRC-checks every entry, inline in an `async fn` on a Tokio worker. The second read uses only `state.vendor`'s manifest. 06-limits:9069-9072 says "The autopilot's `round` and `tick` read it once per pass". | `crate::server::broker_run(&asked, site, &crate::census::read_all(&site.store_root)).await;` (:3443) then `let censuses = census::read_all(&site.store_root); let manifest = manifest_of(&censuses, state.vendor);` (:3487-3488). Plus `land_spot`'s `ingestion_observations` → `census_now`, which misses after every append. | NEW. Fix: read the census once per tick through `census_now` (its stamp cache), or read only `state.vendor`'s manifest for the completeness probe. |
| o1surface2-3 | low | api | server.rs:7534-7560 (`land_spot`), 6365 (`land_bodies_observed` → `pull::ingest::from_window`); spawned by server.rs:10899 `tokio::spawn(crate::pullrun::conduct(..))` and :18405 `tokio::spawn(autopilot::fly(..))` | The broker pull's landing runs synchronously inside async fns on the shared Tokio runtime that also serves HTTP. That landing is: a census read under the lock (O(manifest bytes + E_v) per body, W1-api5-1), bar-file appends with fsync, `derive_all`, and on a miss a calendar derivation (documented at 0.28 s) or a Condvar wait on another request's derivation (`calendar_of.rs` `flight.ready.wait`). The conductor and the autopilot can each hold one worker for the length of a landing. Impact is UNVERIFIED: not timed. | `let observed_calendar = ingestion_observations(landed, site);` and `done.absorb(land_bodies_observed(...))` inside `async fn land_spot` (no `spawn_blocking`). `grep spawn_blocking pullrun.rs autopilot.rs` returns nothing. D-1443's "Not done" lists `folder::answer`, `indexmap::Published::read` and `/gaps.json` only. | NEW (same class as D-0435 / D-1443; this path is not listed). Fix: run `land_bodies_observed` and `ingestion_observations` through `spawn_blocking` behind a permit, as `detail::run_calendar` does. |
| o1surface2-4 | low | cli | lib.rs:15511-15549 (`latest_for`) | The doc says "That is `O(1)` in the ordinary case". Each call runs `Results::open`, which is an O(runs) cold index build, then a backward scan. Measured below: the newest-row match grows about 10.9x for 10x runs. | Probe output (below): `Results::open ratio (10k/1k) = 14.13`; `latest_for-like n=1000: match newest steps=1 ns=1359021` vs `n=10000: match newest steps=1 ns=14801186`; oldest match ratio `10.59` | KNOWN: W2-cli8-4 (resume_audit-20261003 lists it NOT-FIXED). New here: the measurement. |
| o1surface2-5 | info | api | server.rs:781-785 (`param`) | The K·Q query scan is bounded by `MAX_REQUEST_TARGET_BYTES` (8,192), but 06-limits D-1202 says "Not timed". Measured below: 7 absent-key reads cost 10.5 µs at 800 B and 101 µs at 8,000 B, a ratio of 9.61. So it is O(Q) per call and about 0.1 ms per request at the cap. | probe output below | DOCUMENTED: 06-limits D-1202 ("bounded rather than parsed once"). Measurement added. |
| o1surface2-6 | info | api | census.rs:1017 (`filtered`), 1055 (`held_page`) | The filtered `/store` view is O(E) per request: ratio 9.73 for 10x entries. Paging by offset is **not** O(offset): `held_page` at offset 199,800 costs the same as offset 0 (ratio 0.99), because `slice::Iter::skip` is an O(1) `nth`. | probe output below | DOCUMENTED: W1-api5-6 (06-limits D-0953). Measurement added; the paging is confirmed O(1) in offset. |

## Probe output (verbatim; both files deleted after the run)

`crates/api/tests/zz_audit_o1surface2_1.rs`, run with `cargo test -p api --test zz_audit_o1surface2_1 -- --nocapture`:
```
param x7 absent keys: query_bytes=800 ns_per_request=10538 sink=2001
param x7 absent keys: query_bytes=8000 ns_per_request=101290 sink=2001
param ratio (8000B / 800B) = 9.61
census::filtered symbol=SYM1: entries=20000 ns_per_request=620996 kept=233100
census::filtered symbol=SYM1: entries=200000 ns_per_request=6039847 kept=2333100
filtered ratio (200k / 20k) = 9.73
held_page E=200000 offset=0 ns=3568; offset=199800 ns=3533; ratio=0.99
held_page end page E=20000 ns=3537; E=200000 ns=3533; ratio=1.00
test probe_param_filtered_and_held_page ... ok
```
`kept` is summed over 21 calls.

`crates/cli/tests/zz_audit_o1surface2_1.rs`, run with `cargo test -p cli --test zz_audit_o1surface2_1 -- --nocapture`. It uses real `Results::append`/`open`/`read`, and its loop copies `latest_for`'s, which is private:
```
appended n=1000 in 4.05s (4048989 ns/append)
Results::open n=1000 median_ns=1352242
latest_for-like n=1000: match newest steps=1 ns=1359021; match oldest steps=1000 ns=3518801
appended n=10000 in 38.00s (3799998 ns/append)
Results::open n=10000 median_ns=19105881
latest_for-like n=10000: match newest steps=1 ns=14801186; match oldest steps=10000 ns=37259522
Results::open ratio (10k/1k) = 14.13
latest_for-like oldest-match ratio (10k/1k) = 10.59
test probe_results_open_and_latest_for_scan ... ok
```
- The append is flat at about 4 ms (the fsync), so ledger append is O(1) per record as `results.rs` claims.
- The dev profile is optimized with debuginfo. The box is shared, with other builds running, so absolute times are noisy and only the ratios are offered.

## Hot-path table (re-verified at 1087e54)

Verdict key: O(1); bounded (capped by a constant); exp-amortised O(1); NOT O(1).

### api, per request

| path | file:line | unit | cost | verdict | why / fix | documented / status |
|---|---|---|---|---|---|---|
| route dispatch | server.rs:16078-16380 (`route_table`) | request | radix match (axum 0.8 / matchit) | O(1) in routes | — | prior o1api-1, unchanged |
| request head | server.rs:16697-16960 (`HeadDeadline`, `LimitedListener`) | connection | O(1) per byte; 10 s head deadline; 256-connection cap | bounded | the body has no deadline | FIXED probeapi-1 (D-1200); body stall DOCUMENTED 06-limits D-1200 |
| target/header caps, repeated key | server.rs:15917, 15934, 15977-16035 | request | one pass over ≤8 KiB target and ≤64 KiB headers; one HashSet insert per segment | bounded | hyper still buffers up to 417,792 B | FIXED o1api-3/probeapi-5 (D-1202) |
| `param` | server.rs:781 | field read | O(Q), Q ≤ 8,192; measured ratio 9.61 | bounded (not O(1) in Q) | parse once into slices | DOCUMENTED D-1202 (o1surface2-5) |
| form body | server.rs:52, 15898 | POST | ≤ 8 KiB | bounded | — | prior o1api-2 |
| request log | logs.rs:1020-1046 | request | path copy plus one synchronous `telemetry::emit` | O(1) plus emit | emit is file I/O on the worker | DOCUMENTED §46 |
| static asset fallback | assets.rs:737, 817, 836, 870, 892 | request | `canonicalize` plus a whole-file `fs::read` (also `/masters.js`, `/typeahead.js`, index) | NOT O(1) (file bytes) | cache the immutable build at startup | DOCUMENTED §39 (prior o1api-7) |
| `/vocab.json` | server.rs:32641 | request | 370 rows rebuilt, about 20 KB | O(1) (fixed table) | could be a `LazyLock<String>` | prior o1api-11 |
| `/health` | server.rs:4960 | request | O(notes) | NOT O(1) (small) | — | DOCUMENTED §67 |
| `/instruments(.json)` | server.rs:1061, 1114; catalog.rs:537 | request | O(U) filter + census_now + O(E) `bars_by_symbol` + sort | NOT O(1) | needs an index | DOCUMENTED W1-api5-3 (D-0953) |
| census-backed routes (`census_now`) | server.rs:3489-3560 | request | hit: 5-6 `stat`s; miss: O(manifest bytes + E log E) | O(1) on a hit / NOT O(1) on a miss | misses on every request during a pull | DOCUMENTED W1-api5-2, D-0686 |
| `/audit.json` store block | audit_json.rs (`feed_rollup`, `RollupCache::get`) | poll | hit: one mutex and an `Arc` clone; miss: O(E_v) of the asked feed | exp-amortised O(1) | keyed on the census `Arc` identity | FIXED o1api-21 (D-1202/D-0732) |
| `/audit.json` journal page | audit_json.rs:282-296 | poll | fixed-record page, `skip = page*per_page` | bounded (≤ MAX_PAGE_RECORDS) | — | OK |
| `/store` with a filter | census.rs:1017 | request | O(E); measured ratio 9.73 | NOT O(1) | index | DOCUMENTED W1-api5-6 |
| `/store` page | census.rs:1055 | request | O(page); offset is O(1) (ratio 0.99) | O(1) in offset | — | verified by probe |
| `/bars.json` | server.rs:2313; bars.rs:417-460 | request | O(log n + rows); fallback past the end O(n_valid) | bounded by month | — | DOCUMENTED W1-api5-8 |
| `/bars/window.json`, `ts` sort | bars.rs:826-887 | request | O(months) header reads + O(limit) records | bounded (240 months) | — | stated in bars.rs doc |
| `/bars/window.json`, other sort or extremes | bars.rs:1040-1060, 1098-1120 | request | O(n) reads and memory (n up to about 1.9 M) + O(n) partition + O(limit log limit) | NOT O(1) | needs a precomputed index | DOCUMENTED W1-api5-4 (D-1446); ordering cost FIXED |
| `/calendar.json`, `/gaps.json` peer vote | server.rs:32895; calendar_of.rs:2030-2105 | request | `held_entries` O(E_v log E_v); a miss derives once (single flight), off the async workers (`run_calendar`, ≤ 8) | NOT O(1) | — | DOCUMENTED W1-api5-5, D-1443 |
| `/verify.json` | server.rs:4677 | request | O(log length) + O(E_v) file opens | NOT O(1) | — | DOCUMENTED W1-api5-7 |
| `/indexmap.json` | server.rs:32529 | request | CSV re-read + O(U) | NOT O(1) | read once at load | DOCUMENTED W1-api5-9 |
| `/logs(.json)` | logs.rs:44, 51 | request | ≤ 4 MiB scan, ≤ 200 events | bounded | — | code doc |
| `/backtest.json` | backtest.rs:390 | request | O(limit ≤ 20,000) | bounded | — | code doc |
| `/live.json` | cli live.rs:820-822 | poll | ≤ 4,096 entries, ≤ 128 runs | bounded | — | DOCUMENTED D-0523 |
| `/backtest/run.json` | sweeprun.rs:2091-2095, 115 | poll | `Arc<str>` clone under the lock | O(1) lock hold (the body transfer is O(report)) | — | FIXED o1api-26 (D-0954) |
| `/sweep-evidence.json` | sweepevidence.rs:143-170 | request | fixed-record page via `ranked_page`/`depth_page` at `page.offset()` | bounded (≤ 256 rows) | — | OK |
| `/candidate-trades.json` | candidatejson.rs (`render`) | page | about 5 × O(catalog bytes) | NOT O(1) | — | DOCUMENTED D-1444 (W1-api2-2) |
| Boolean evidence, campaign, `/engine/top.json` | booleanjson.rs, booleanevidencejson.rs, booleancampaignjson.rs, topjson.rs | request | cold O(body) / O(H) / O(history) | NOT O(1) | — | DOCUMENTED D-1444 |
| `/autopilot.json` stall list | autopilot.rs | poll | bounded by the calendar | bounded | — | FIXED-ish W1-api1-2 (D-0949) |
| credential watch | credential_law.rs:271-400 | instrument or cell | one fingerprint compare; a re-read is one SSM read + SHA-256 | O(1) | — | NEW code, OK |
| `POST /pull/spot`, `/universe/resolve` | server.rs:10975, 31919 | POST | O(U) / O(U log U) | NOT O(1) | — | DOCUMENTED W1-api5-11 |
| autopilot tick | autopilot.rs:3443, 3487 | tick | 2 × `read_all` of every vendor | NOT O(1) | see o1surface2-2 | NEW |
| pull landing on the runtime | server.rs:7534-7560 | instrument | synchronous ingest on a Tokio worker | NOT O(1), blocking | see o1surface2-3 | NEW |

### cli, per run / per instrument-month / per operation

| path | file:line | unit | cost | verdict | why / fix | documented / status |
|---|---|---|---|---|---|---|
| run identity | lib.rs:3448-3463 (`identity(&Run{..})`), `stored_executed_digest` | run | digest O(bars) once, then one blake3 over the 9 terms | O(1) per run beyond the inherent data digest | — | OK |
| ledger append | results.rs:1175-1215 | run | O(delta+1); measured flat at about 4 ms (the fsync) | exp-amortised O(1) | — | results.rs:20-36 |
| duplicate-run check | results.rs:1083-1085 (`holds`) | run | one HashMap probe | exp O(1) | — | C-CLI-03 |
| `Results::open` | results.rs:839 | open | O(runs): measured 14.13x for 10x | NOT O(1) | — | DOCUMENTED D-0523 (cold opens O(history)) |
| `latest_for` | lib.rs:15521-15549 | rung | open O(runs) + backward scan | NOT O(1) (doc says O(1)) | key by identity via `of_identity` on a shared writer | KNOWN W2-cli8-4, NOT FIXED |
| `newest_complete` / `results_at` | lib.rs:8023, 8168 | request | O(ledger rows) | NOT O(1) | — | KNOWN W2-cli8-5, NOT FIXED |
| sweep-stored month | lib.rs:3318-3470 | instrument-month | loads + context + column: O(month bars) | O(1) per bar | — | OK |
| sweep-all | batch.rs:350-362 | instrument-month | indexed `par_iter` collect, then a sequential fold | O(1) per month on top of the sweep | — | prior o1cli-21; the catalog walk is DOCUMENTED §86 |
| per-bar load / filter | stored.rs:2361, 117-130, 2843, 2991-3005; minute_gaps.rs:214, 256-266 | bar | one pread, a 9-day position, amortised reserve, monotone cursor, one HashSet probe | O(1) / exp O(1) | — | prior o1cli-7..14 re-verified |
| fold_audit | fold_audit.rs:197-325, 374-460 | bucket | two-cursor walk, 7 fields; 7 folds of the month's minutes | O(1) per bucket; O(7·minutes) per month | — | module doc "Cost" |
| AND checkpoint | and_checkpoint.rs:137-243 | depth boundary | new level's chunks + a boundary record listing every chunk; resume and final re-read every chunk | O(level bytes) per boundary; O(history) on resume or finish | — | DOCUMENTED D-0712 |
| search checkpoint discovery | search_checkpoint.rs `discover_through` | journal open | `read_dir` ≤ 1,000,000 + a stat each | bounded | — | DOCUMENTED D-0524 |
| expression-search node | vocab expression_search (`place`) | grammar node | O(1) except the sibling slice compare | not proven O(1) | — | DOCUMENTED D-0983 |
| Boolean resumable paths | boolean_grammar_campaign.rs, boolean_oos_v1.rs, boolean_candidate_v1.rs | step | re-verify all completed work | NOT O(1) | — | DOCUMENTED D-1400 (tests/c4_cli_02_limits.rs) |
| checksum receipts | checksum_receipts.rs:1-4 | month | cold O(source); warm one block + one receipt | NOT O(1) cold | — | module doc |
| elite descent step | lib.rs:14095, 15774-15890 (`ScreenCache`) | step | inputs held across steps; the per-step copy is O(bars) | NOT O(1) (frontier + trade walk) | — | FIXED o1cli-1 elite half (D-0997); §91 |
| plain descend step | lib.rs:14934-14949 | step | full `one_rung` reload + column build | NOT O(1), repeated | see o1surface2-1 | NOT FIXED |
| one_rung / sweep_rungs / audit kernel / screen sorts / reference loads | lib.rs:13326, 15144, 6658, 993, 14451, 14550, 14589, 13963 | rung / command | repeated span, context and column loads; 2 full sorts | NOT O(1) | — | DOCUMENTED o1cli-2..6 (tests/limits_o1cli_2..6.rs) |
| Selection ledger absorb after a peer append | selection.rs:2013, 2328, 3026 (`require_indexed_records_unchanged`) | refresh after growth | O(indexed records) re-read | NOT O(1) | — | NEW code, DOCUMENTED 06-limits:11042 (W2-cli14-5) |
| frontier / chosen-trade refresh | frontier.rs, trades.rs (`absorb_read_only`, `refresh_locked`) | refresh | O(delta) rows | exp-amortised O(1) per new row | — | NEW code, OK |
| population / finalization ledgers | population_finalization_v3/v4.rs, population_v5.rs, step3_orchestrator.rs | read / commit | whole-file rehash or rescan | NOT O(1) | — | DOCUMENTED (tests/ledger_scan_costs.rs) |
| columns layout | columns.rs:102-160 | table | O(rows × cols) once per report | bounded by output | — | NEW code, OK |

## Re-verification of prior rows (summary)

- **o1api.** Rows 1, 2, 5, 8, 11, 12 and 22-24 are unchanged and OK. Rows 3, 4, 21 and 26 are FIXED. Rows 9, 13-20, 25 and 27-31 are now DOCUMENTED under D-0953, D-1443, D-1444 and D-0686. The cost_limits_tests and the d0951 bullet tests in api hold these to the code. I did not run them; that is UNVERIFIED in this pass.
- **o1cli.**
  - Row 1 is PARTIAL (o1surface2-1).
  - Rows 2-6 are DOCUMENTED with source-bound tests.
  - Rows 7-21 were re-verified at the HEAD lines quoted above.
  - Rows 22-43 (the W2-cli queue) were spot-checked. `latest_for` (o1cli-26) and `newest_complete` (o1cli-27) are unchanged and not fixed; `latest_for` was measured.
  - I did not re-open every candidate_universe and population row (o1cli-33..43). Their status is UNVERIFIED beyond the fact that `ledger_scan_costs.rs` and `c4_cli_02_limits.rs` now document some of that class.

`git status --porcelain`: no `zz_audit_o1surface2_*` file remains. The other untracked `zz_audit_*` files belong to other workers.

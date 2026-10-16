# cloud-attack-law-20261016 — the law, attacked lens by lens

Branch `cloud/attack-law-20261016`, cut from `origin/cloud/base-20261016`
(c9e3472e, the one-commit snapshot of `feat/db-fix` at 4756cce0). Every claim
below names the `file:line` it was read at. **Line numbers are of the base
tree** unless a row says "here", in which case they are of this branch. Anything
not checked is marked UNVERIFIED. Locked files (`crates/costs/**`,
`crates/pull/src/calendar.rs`, `crates/api/src/dayflags.rs`,
`crates/cli/src/lib.rs`) were audited and never edited; their findings say so.

Companion file: `reports/cloud-attack-law-20261016-lens2.md` lists every one of
the 3,355 fallback-shaped lines classified for lens 2.

## Summary

| lens | result |
|---|---|
| 1 look-ahead | No entry signal reads a later bar. `vwap::availability_of` has **no production caller**. One real exposure: in-sample exit levels are fitted to the same trades' later outcomes (medium, finding). |
| 2 hidden fallbacks | 3,355 lines classified: 62 swallow a real failure (≈45 distinct findings). **9 fixes here cover 12 of the 62 sites**, each with a test; the rest are findings with the exact fix. |
| 3 tests that assert nothing | 8,710 `#[test]`s scanned: none asserts nothing outside one compile-time type check. But **15 permission tests failed (13) or passed vacuously (2) as root** (this container runs as uid 0): **all 15 fixed here** with D-0995's root-safe wrapper; one flaky inode assertion fixed. A 16th, in `cli`, is a finding: its fix needs a line in the locked `crates/cli/src/lib.rs`. |
| 4 append-only | No writable mapping anywhere. Store format versions are never mutated in place. Durability gaps: missing directory fsyncs, failed barriers not rolled back, a probe that wrote through a link. **4 fixed here**, the rest findings. |
| 5 money | No price, cost, stop or premium is a float. One ranking defect: the float mean's sign picked the traded side on an exact-zero sum — **fixed here**. |
| 6 depth | No depth parameter on the Apriori sweep; the ladder stops only on an empty frontier or a recorded halt. The separate expression search has an undocumented ~1M-candidate wall (finding). |
| 7 docs vs code | §5's graph equals `cargo metadata` exactly. **15 ✓ invariant rows name tests that exist nowhere**, several describing behaviour later decisions reversed. **35 decision ids cited in code have no ledger entry** — D-3349 to D-3353 alone are cited 317 times. |
| 8 extreme cases | Coverage matrix below. **Ten tests added**; three of them exposed real parser bugs, fixed here. |
| gates | `fmt` and workspace `clippy -D warnings` clean. Seven crates tested as uid 0: **7,529 passed, 6 failed, 33 ignored**. None of the six failures is in code this branch changed (see Gates). `cargo deny`, coverage and mutation testing are not installed here: UNVERIFIED. |

## Lens 1 — look-ahead

Method: every function on an evaluation, signal, rank, selection, backtest,
outcome or trade path that takes a slice of bars and reads past the current
index or aggregates the whole slice, with its callers traced to an entry point.

### `vwap::availability_of` and `PastPrefix`

| where | what | verdict |
|---|---|---|
| `crates/indicators/src/vwap.rs:277-283` | `Present` iff `bars.iter().any(|b| b.volume > 0)` — bar 0's answer depends on bar N | **no production caller**: callers are tests (`vwap.rs:934`, `vwap.rs:952`) and one bench fixture check (`crates/runner/benches/ratio.rs:409`); every other hit is a comment |
| `crates/cli/src/stored.rs:2301-2305` `vwap_availability` | production decides from instrument kind: `Equity → Present`, else `Absent`, no bar read | clean; callers `cli/src/batch.rs:772`, `index_stop.rs:709`, `boolean_candidate_v1.rs:594`, `pool.rs:3533/3654` |
| `crates/indicators/src/lib.rs:197` `PastPrefix` | declared; uses at `lib.rs:1182/1188/1191/1447/1455` all inside `#[cfg(test)]` | **zero production call sites**, as CLAUDE.md §3.7 says |

### Findings

| where file:line | what | why it breaks the law | severity | fixed here / exact fix |
|---|---|---|---|---|
| `crates/runner/src/grid.rs:2146-2169, 2210-2226, 2289, 2326` (`single_far_target` 1711-1723, `merged_stops` 1740-1745); reached from `cli/src/lib.rs:9744 → 9822` | in-sample exit levels: the target is the 90th percentile of the same trades' best favourable move, and stops/trails are percentiles of those trades' moves | a trade in month 1 is priced with a target set by how far trades in later months ran: the in-sample best cell's money is fitted to outcomes. Out-of-sample is clean (walk-forward trains on training trades, `runner/src/validate.rs:4842`, replays at `5118-5130`) | medium | finding. Build each trade's ladders only from trades with `exit_bar < entry_bar` (expanding window) or a declared training window; at minimum label in-sample money "exit levels fitted to the sample" |
| `crates/cli/src/lib.rs:1803-1810, 5344-5583, 5699-5800` (LOCKED) | stop rungs, grid step and rung count from whole-span bar-range percentiles | a whole-span volatility statistic sets earlier trades' stop distances | low | finding; accepted by D-1660 ("in-sample by construction"); walk-forward re-sizes per window (`validate.rs:5829`) |
| `crates/cli/src/step3_orchestrator.rs:3371-3408` → `crates/runner/src/exit_grid_policy.rs:1029-1110` | Step 3 resolves exit grids over the whole requested span, then prices the in-sample family with them | same in-sample fitting; Search V4 re-resolves per fold (`validate.rs:2602-2605`) | low | finding; UNVERIFIED whether Selection V6 ranks on that in-sample money |
| `crates/cli/src/minute_gaps.rs:275, 334, 417` | a day with any missing minute is withheld whole, bars before the hole included | a later missing minute decides whether earlier rows are sampled (selection on availability, not price) | low | documented (D-1662, D-1781) and bound into the run identity (`MINUTE_GAP_POLICY = 3`); no change |
| `crates/indicators/src/column.rs:1879-1938` | the prefix-cut test cuts at `1 + 25k`; sessions in its fixture start at `375k` | no cut ever lands on a session boundary, so a read of the next session's first bar would pass | low | finding; add a cut at `b-1, b, b+1` for each IST day boundary of `warm_run()` |

### Cleared (looked like look-ahead, is not)

| where | why it is clean |
|---|---|
| `crates/indicators/src/column.rs:647-719` `Column::build` | hands the evaluator one bar at a time and steps before emitting (§3.7's shape); pinned by `column.rs:1842`, `:1879` |
| `crates/indicators/src/evaluator.rs:883-1092, 1236-1288` | day rollover happens on the new day's first bar; previous-day values use only the finished day |
| `crates/indicators/src/orb.rs:171-203`, `gap.rs:283-327`, `lib.rs:484-498` | each emits before folding the bar; opening range closes by clock |
| `crates/indicators/src/trend.rs:541-640, 803-824` | a swing is confirmed only after its two following bars; pinned `trend.rs:1416`, `:2054` |
| `crates/indicators/src/anchored.rs:389-509, 723-917` | daily references only for `reference.day < signal_day`; pinned by `appending_valid_future_daily_prices_cannot_change_a_current_mask` |
| `crates/runner/src/align.rs:112-188`, `crates/pull/src/fold.rs:163` | bars are stamped at their open; a signal fills at exactly open + length, same IST day |
| `crates/runner/src/trade.rs:962-1315`, `outcome.rs:931-1088, 1141-1199` | entry on the next bar; later bars read only for the outcome; pinned by `runner/tests/prefix_only_cadence.rs` |
| `crates/runner/src/validate.rs:4453-5222` | per-fold columns, rungs and direction from training only; `an_anchored_fold_never_trains_on_its_own_future` |
| `crates/runner/src/signal_candle_stop.rs:1138-1423` | stop = completed signal bar's extreme; `duplicates` reads the next row only to detect a duplicate stamp |
| `crates/runner/src/outcome.rs:227` (`.enumerate().rev()`) | session-bounds walk, not a signal; part of the outcome path above |
| CLAUDE.md §3.7 on this branch (`CLAUDE.md:188-197`) | already states the equity `Present` rule (D-1764); nothing to correct |

## Lens 2 — hidden fallbacks

Method: every non-test line of `crates/**` holding one of `unwrap_or(`,
`unwrap_or_default(`, `unwrap_or_else(`, `.ok()`, `let _ =`, `Err(_) =>`,
`filter_map(`, `.flatten()`, `eprintln!`, `.is_ok()`, `_ =>` — 4,271 lines.
916 are `let _ = write!/writeln!` into a `String` (the io-typed writers,
`crates/api/src/server.rs:99-116`, return their results). The other **3,355
were each read in their enclosing function** and classified; the full list
is the companion file.

| class | count |
|---|---|
| SWALLOWED (a real failure becomes a default / skip / log-and-continue, output not marked degraded) | **62** |
| SURFACED (named and counted, refused or alerted) | 378 |
| BENIGN-OPTION (an `Option` default that hides no failure) | 1,016 |
| BENIGN-INFALLIBLE (cannot fail, or irrelevant by construction) | 1,294 |
| BENIGN-MATCH (`_ =>` that is not a default on a failure) | 605 |
| AMBIGUOUS | 0 |

Two bulk rulings: every `PoisonError::into_inner` is a read-through of a cache
or flag, and the release profile aborts on panic (`Cargo.toml:194`), so a
poisoned lock cannot exist in the shipped binary; every
`uri.query().unwrap_or("")` means "no query string" and each parameter is then
parsed or refused on its own.

### Every swallowed failure

| where (base) file:line | what | why it breaks the law | severity | fixed here (test) / exact fix |
|---|---|---|---|---|
| `crates/runner/src/grid.rs:1794-1823` (`entry_fills`) with `realised` (`grid.rs:4235`) | a long's pessimistic entry is `fills_at(bar, bar, Long, PrintedExtreme)`, whose sell leg refuses a positive low under one tick (`crates/costs/src/fill.rs:454`, `TICK` 5 paisa at `costs/src/rate.rs:256`); the refusal answered `0`, and `realised` charged 0 paisa as the entry price | a long bought at 0 on the ranking path; `Candle::check_evaluable` (`crates/indicators/src/lib.rs:1772-1786`) admits a low of 1–4 paisa | high | **fixed here**: `Candidate::unmeasured` and `refused_at` treat `entry_pess == 0` as unpriceable — `grid::tests::a_sub_tick_entry_low_refuses_the_path_rather_than_charging_a_zero_entry` (failed before) |
| `crates/runner/src/validate.rs:5140` fed by `grid.rs:3806` | a test-window replay with **every path refused** returns no cell and becomes `0` in `out_of_sample_all`, identical to a flat candidate; `with_levels_over_from` drops `refused_paths` | an unmeasured candidate is ranked as flat by `runner::pbo::place_v1` (`cli/src/lib.rs:22673`); V4 already refuses this (`validate.rs:2905-2910`) | high | finding (a design call: refuse the fold, or mark it unrankable). Return `ReplayOutcomeV1` from `with_levels_over_from`; in the walk map `(None, refused > 0)` to `Err("walk-forward fold {f}: candidate {c}'s out-of-sample replay refused {n} path(s) and priced none")` and set `out.refused` as `validate.rs:5016-5018` does |
| `crates/runner/src/grid.rs:1844, 1851, 5026` (`exit_fill`) | an unpriceable pessimistic exit falls back to the entry price, booking the trade flat; no counter | a refused fill counted as a real 0-P&L trade | medium | finding; give `Priced` an "unpriced" flag that `one_variant` counts into `refused_paths` like `refused_at` |
| `crates/pull/src/manifest.rs:4512` (`read_contract`, caller `:2203`) | a CRC-valid contract field that does not read back (length > 24, not UTF-8, `Contract::parse` refuses) decoded as **no contract** | `None` is the spot key: the contract row was counted under its underlying's spot series and two such rows merged | high | **fixed here**: `EntryFault::ContractUnreadable { len }` — `census::a_contract_field_that_does_not_read_back_is_refused_not_read_as_spot` (did not compile before: no such refusal) |
| `crates/pull/src/census_store.rs:1742, 1746` (`legacy_drift`) | an unreadable receipt, or one naming no `legacy_bytes`, answered "no drift" | the only warning that a pre-v4 writer filed months the census does not read (`api/src/store_route.rs:419`, `cli/src/census_recount.rs:259`) | medium | **fixed here**: only `NotFound` is "not migrated" — `census_store::tests::a_migration_receipt_that_cannot_be_read_is_named_not_read_as_no_drift` |
| `crates/pull/src/gdfl_rekey.rs:202, 209` | a second `superseded`/`retired` line for one key with different fields silently replaced the first | a ledger documented "read whole or refused" let the later target win | low | **fixed here** — `gdfl_rekey::tests::a_key_withdrawn_twice_with_different_records_refuses_the_ledger` (failed before) |
| `crates/pull/src/gdfl_import.rs:7075, 7077, 7112, 7117` (`warm_files`) | lineage warm-up drops every file `read_file` refuses, unlogged; the doc (7001-7002) says it is logged | a contract adjustment can be missed with nothing said | medium | finding; route both `.ok()` through a helper that emits `Event::warn(TARGET, "lineage warm-up file unread")` with `day`, `file`, `why`, then `None` |
| `crates/pull/src/repair_plan.rs:167, 171` (`held_months`) | a `read_dir` error answers "no months held"; entry errors dropped | weakens the landing identity screen (`identity_screen.rs:81-82`) and `day_book` (`:296`) | medium | finding; return `Result<Vec<YearMonth>, String>` with `NotFound → Ok(vec![])`; callers `repair_plan.rs:127, 152`, `identity_screen.rs:81, 296` |
| `crates/pull/src/repair_plan.rs:247, 249` | a time-index or record read error on a held day plans it as `missing` | a read fault becomes a day to re-pull (`cli zerodha-repair`, `POST /pull/repair` at `api/src/repair_route.rs:161`) | medium | finding; on `Err(why)` set `out.refused = Some(why.to_string())` and return |
| `crates/pull/src/ingest.rs:2327, 2329` (`not_held_days`) | a read error on a held day counted as `refused_day` | the D-3351 ledger misstates which day was missing | low | finding; treat like a month that will not open, with a named failure |
| `crates/pull/src/census_store.rs:1031` (`install_epoch`) | `symlink_metadata(..).is_ok()` reads any lstat error as "absent" before `write_whole` renames over the path | the write-once guard opens on a probe error | low | finding; `NotFound → proceed`, `Ok → refuse`, other `Err → refuse naming it` |
| `crates/pull/src/http.rs:2758` | a non-ASCII `Location` is reported as "no Location header" | the refusal's words are false | low | finding; name the byte length instead |
| `crates/api/src/audit.rs:2865` | damaged `faults` index entries dropped by `filter_map`, uncounted | `/audit.json` under-reports the records that will not decode | medium | **fixed here**: counted into `malformed` — `audit::tests::a_damaged_fault_or_refusal_entry_is_counted_or_refused_not_dropped` (failed before) |
| `crates/api/src/audit.rs:2889` (`refusal_ordinals`) | damaged refusal entries dropped | `/dayflags.json`'s refusal history one record short; the caller (`dayflags.rs:2421`, LOCKED) already maps `Err` to a named failure | medium | **fixed here** (same test) |
| `crates/api/src/audit.rs:2851` + `crates/api/src/audit_json.rs:713` | a damaged `runs` entry is counted into `malformed`; `pullrun::older_runs` refuses on it (`pullrun.rs:1624`) but `/audit.json` read the index as whole | a run vanished into the run before it on `/audit.json` | medium | **fixed here**: attribution walks the journal and says why — `audit_json::tests::an_index_that_lost_a_run_entry_is_walked_and_says_why` (failed before). The investigator's claim that "nothing reads `malformed`" was false (`pullrun.rs:1624`), so `read_index` was left returning `Ok` |
| `crates/api/src/audit.rs:2722-2739` (`append_synced`) | a short index write is not rolled back, so later entries misalign (the root cause of the three rows above) | the journal writer rolls back (`audit.rs:1955-1960`); its index does not | medium | finding; record the length before the append and `set_len` + `sync_all` back to it on error |
| `crates/api/src/audit.rs:3106` (`ensure_index`) | the index-build thread's spawn error is dropped | the index is never built and nothing says why | low | finding; emit `Event::error("api.audit", "journal index build not started")` with `why` |
| `crates/api/src/server.rs:2596, 2599` (`newest_record`) | a page-read failure or an undecodable newest record becomes `None`, and `/pull` says "No pull has been recorded against this store root yet" (`server.rs:2857-2863`) | a corrupt journal reads as an empty one | medium | finding; branch on `log.records() > 0` and say the newest record would not read or decode |
| `crates/api/src/server.rs:11971` and `:10846-10853` | with `rate=` supplied and no swept slot, greeks are silently not computed; comments at `:10621`, `:11864` claim a reason is given | a requested computation is skipped with no reason on the receipt | medium | finding; push `"greeks not computed: {underlying} has no recorded strike-ladder regime"` into `PricedCount.why` on both paths |
| `crates/api/src/pullrun.rs:2018, 2048` (`rows_now`, `rows_of`) | `filter_map(VendorCensus::counters)` drops `Census::Unreadable`, adding 0 | `rowsAtStart`/`rowsNow` come out low instead of unknown (also `recovery.rs:1045, 1788`) | medium | finding; return `None` when any counted census is `Unreadable` (UNVERIFIED that a garbage manifest reads as `Unreadable`, not `Absent`) |
| `crates/api/src/sources.rs:433, 565, 566` | `metadata().ok()`, `StorePath::new(..).ok()`, `served_bars_path(..).ok()` make "unreadable" and "no file" one `DAYS` memo key | a cached day can be served over an unreadable generation pointer — the fallback `crates/store/src/generation.rs:256-260` names as banned | medium | finding; `day_stamp` returns `Result<Option<Stamp>, Refusal>` with only `NotFound → Ok(None)`; its one caller `day_facts` already returns `Result` |
| `crates/api/src/server.rs:2142` (`shared_governor`) | `budgets.lock().ok()?` on a poisoned lock strips the source's rate governor (`pull/src/http.rs:602-613`) | ungoverned vendor calls; unreachable in release (`panic = "abort"`) | low | finding; `.unwrap_or_else(PoisonError::into_inner)` as the `budgets` policy at `server.rs:2367` says; same at `:8434` |
| `crates/api/src/autopilot.rs:2427` (`hand_in_flight`) | `.unwrap_or(false)` on a poisoned status lock answers "no hand pull in flight" to Stop (`server.rs:9912`) | `inspect`'s own doc (`autopilot.rs:2487-2493`) forbids this; unwind builds only | low | finding; return `Option<bool>` and have Stop stop every hand run and answer 503 on `None` |
| `crates/api/src/autopilot.rs:2836, 2838, 2852` (`floor_day`) | clock / `clamp_to_floor` errors drop a feed from the backfill loop unnamed | the status page never names the dropped feed | low | finding; `floor_day -> Result<Day, String>`, `drivable` keeps the feed as halted with the reason |
| `crates/api/src/calendar_of.rs:667-668` (`withhold_unproved`) | an unrepresentable first day returns early: unproved months keep their `Closed` days | fails open; unreachable from valid bars | low | finding; on `None` call `calendar.withhold_closed(calendar.first_day(), last)` before returning |
| `crates/api/src/detail.rs:163` | the admission-clock thread's spawn error is dropped and `None` cached forever | every later wait is refused as "Saturated" — the wrong reason | low | finding; log the error and do not cache `None` |
| `crates/api/src/disk_route.rs:163, 194, 197` | a permission error reads as "not sealed"; unreadable entries vanish from `/disk` totals | a file's role mislabelled; totals silently short | low | finding; `NotFound → false`, other error → an "unreadable" role; count skipped entries |
| `crates/api/src/mastersrun.rs:728` | `metadata().ok()` reports EACCES/EIO as `"present":false,"bytes":0` | unreadable reported as absent | low | finding; `NotFound` only is absent; else `"unreadable":"<why>"` |
| `crates/api/src/import_journal.rs:350` | `runs_unindexed` counted, read nowhere (doc 250-252 promises it is never silently missing) | a counter that is never surfaced | low | finding; add `"runs_unindexed"` beside `"torn"` at `imports.rs:1168` |
| `crates/api/src/import_journal.rs:682` | a non-UTF-8 line folded lossily and, if it parses, kept uncounted | corruption inside a value field shown but not counted | low | finding; count it as foreign |
| `crates/api/src/sources.rs:1589`, `assets.rs:648`, `render.rs:3271`, `catalog.rs:118, 279` | display-only lists drop unreadable entries / unknown `sort=` and pills fall back to defaults with 200 | degraded display not marked (catalog's is documented at `catalog.rs:256-259`) | low | findings; name the effective selection or count the dropped entries |
| `crates/api/src/dayflags.rs:2853` (LOCKED) | a refused open or unreadable first daily record reported as `none_held` | a corrupt Zerodha daily file looks like "not held" | medium | finding only (locked); add `SeriesStart::Unreadable(why)` → `"state":"unreadable"` |
| `crates/api/src/dayflags.rs:3237` (LOCKED) | unreadable last bar before the page → `prev_close` `None`, no note | the impossible-jump check (`dayflags.rs:3777`) silently skipped on the page's first day | low | finding only (locked); return `Result` and push a note |
| `crates/telemetry/src/sink.rs:1729, 1749` (`resume_point`) | an unreadable or undecodable newest log tail restarts sequence numbers and the run-id floor at 0 with no runtime notice | run ids can be reissued, merging two attempts on `/logs` | low | finding; return the reason and `sink.report(&why)` at the caller (`sink.rs:956-968`) |
| `crates/cli/src/srt_builder.rs:502` (`name_refused`) | `symlink_metadata(name).ok()?` read every lookup failure as "absent"; `--dry-run` said `would build` where the build refuses (`crates/store/src/sort_index_writer.rs:651-652`) | a dry run that says what the real run would not do | low | **fixed here** — `srt_builder::tests::a_dry_run_names_an_uninspectable_name_rather_than_reading_it_as_absent` |
| `crates/cli/src/series_identity.rs:271`, `crates/cli/src/zerodha_repair.rs:440` | `entries.filter_map(Result::ok)` drops an unreadable listing entry; the every-series table can exit `OK` without that series | the quarantine works from that table | medium | **fixed here**: shared `series_identity::series_names` refuses — `series_identity::tests::an_unreadable_listing_entry_refuses_rather_than_shortening_the_table`. Honest limit: a real directory cannot be made to fail one entry, so the test drives the helper; it did not exist before |
| `crates/cli/src/live.rs:659` (`census_at`) | `entries.flatten()` drops unreadable entries uncounted | no production caller (`census`/`current` only from tests); production `CensusCache` refuses (`live.rs:936-937`) | low | finding; count into `skipped`, or delete the unused functions |
| `crates/cli/src/lib.rs:6867` (LOCKED) `session_index` | an out-of-order day prints to stderr and returns a TRUNCATED index | White/SPA/Romano-Wolf p-values on partial data (`lib.rs:20803`, `pool_oos.rs:207`); only reachable if the store's strictly-increasing rule breaks | medium | finding only (locked); return `Result<Vec<i64>, String>` and refuse |
| `crates/cli/src/lib.rs:21017` (LOCKED) | `BRUTEX_HORIZON_BARS=rung` over an unmeasurable spacing silently uses `Horizon::DEFAULT` | the operator's request replaced, nothing recorded | low | finding only (locked); `unwrap_or_else(|| { knobs::refuse_value("BRUTEX_HORIZON_BARS", &raw); Horizon::DEFAULT })` |
| `crates/cli/src/lib.rs:8968` (LOCKED) | `let _ = remove_dir_all(&dir)` on verify scratch | debris left silently (16 leftovers refuse loudly, `lib.rs:8990-9001`) | low | finding only (locked) |

Lines that looked like violations and are handled (selection; every one is in
the companion file): `crates/cli/src/population.rs:3393` (surfaced by a warn
event, `fixed_tail.rs:340-350`); `stored.rs:594` (bound into the run digest,
`stored.rs:730-735`); `csv.rs:1018` (counted `unreadable_volume`);
`ingest.rs:4885` (D-2765); `knobs.rs:256, 312` (documented `knobs.rs:122-123`);
`frontier.rs:1639` and `boolean_catalog_command.rs:195` (refused downstream);
`config.rs:272, 1106, 1145` (the credential path refuses — CLAUDE.md §8 holds);
`ssm.rs:245, 310, 353` (AWS identity, not the broker credential, D-1534).

## Lens 3 — tests that assert nothing

Method: every `#[test]` / `#[tokio::test]` function under `crates/**` (8,710,
which equals the count of test attributes in the tree) was extracted with its
body and checked for `assert!`, `assert_eq!`, `assert_ne!`, `matches!`,
`panic!`/`unreachable!`, `#[should_panic]`, a `?` in a `Result`-returning test,
or a call to a helper in the same crate that itself asserts (followed
transitively). Separately, every test that calls nothing from the code under
test was listed (54), and every early `return;` in a test was read (33).

| where file:line | what | why it breaks the law | severity | fixed here / exact fix |
|---|---|---|---|---|
| `crates/pull/src/gdfl_import_tests.rs:2175`, `:3751`; `crates/pull/src/gdfl_tickstore.rs:1375` | chmod tests with no root guard — **measured failing as root** here (`cargo test -p pull`, before this change) | a permission test that cannot bind is red for a reason that is not true, or would be skipped by hand | medium | **fixed here**: each wraps its body in `crate::support::where_permission_binds` (D-0995); `cargo test -p pull --lib` is now 1221 passed, 0 failed as root |
| `crates/api/src/server.rs:20200, 20660, 20956, 21073, 27213`; `crates/api/src/pullrun.rs:6009, 6406`; `crates/api/src/logs.rs:6517, 6728` (10 tests) | the same: **measured failing as root** (`cargo test -p api --lib`: 1966 passed, 10 failed); each of the 10 **passes when rerun as uid 65534** with this branch's fixes in place | as above | medium | **fixed here**: each body runs through `crate::isolated::where_permission_binds` (`crates/api/src/isolated.rs:75`), the pattern `census_request_tests.rs:1235` already uses (the async one through a current-thread runtime) |
| `crates/api/src/bars_sort_tests.rs:1649-1653`, `:1702-1706` | `if File::open(&feb).is_ok() { …; return; }` — as root the test returns before its first assertion and passes | a test that asserts nothing where it runs (§4); no line says it was skipped | medium | **fixed here**: wrapped in `crate::isolated::where_permission_binds`, and the early return replaced by an assertion that mode 000 binds |
| `crates/cli/src/scrub_blocks_tests.rs:610-611` `scrub_blocks::tests::an_empty_census_a_single_record_and_an_unwritable_ledger` | `locked(0o444)` on the ledger, then `expect_err("an unwritable ledger")` — **measured failing as root** here (full `cargo test -p cli --lib`: 2035 passed, 1 failed, panicking at `:611:53`), and **passing when the same binary runs it as uid 65534** (`setpriv --reuid=65534`, 1 passed) | the same root-only red as above. It fails loudly, so nothing is hidden, but it is red for a reason that is not true of the code | low | finding, not fixed: `cli` holds no copy of D-0995's helper, and adding one is a `mod` line in `crates/cli/src/lib.rs`, which is locked for this branch. Exact fix once it is free: add `#[cfg(test)] mod isolated;` with `crates/api/src/isolated.rs:55-123` copied, then run this test's body through `crate::isolated::where_permission_binds("scrub_blocks::tests::an_empty_census_a_single_record_and_an_unwritable_ledger", …)` |
| `crates/runner/src/identity.rs:1427-1429` `the_vocabulary_version_is_in_the_identity` | asserts only `vocab::VOCAB_VERSION == 3` | the name claims the version is IN the identity; the assertion pins a constant. The property is in fact guarded by `a_persisted_run_identity_does_not_drift` (`identity.rs:1474-1484`, a golden identity hex) | low | finding: rename to `the_vocabulary_version_is_three`, or assert two identities over runs differing only in the vocab term differ |
| `crates/vocab/tests/implication_pointers.rs:115-128` | asserts a property of `i64::saturating_add` over constructed values; no code of the crate is called | tests the standard library, not this code | low | finding: keep as a documented assumption, or call the rung-ordering function that relies on it |
| `crates/cli/src/step3_orchestrator.rs:5374-5394` | no runtime assertion: coerces two fn items to fn-pointer types and `black_box`es them | a compile-time type assertion — the build fails if the signatures diverge | low (cleared) | none; a `const _: PublicEntry = …;` would state the same at compile time without a test |
| `crates/core/tests/findings.rs:587, 686, 768`; `crates/store/tests/cited_commits.rs:689, 777, 935`; `crates/store/tests/fifo.rs:212`; `crates/lake/tests/real_lake.rs:168, 200`; `crates/lake/tests/refusals.rs:1140, 1157` | return early when `git`, `.git`, `/dev/null`, `$HOME` or the operator's lake is absent | each prints `SKIPPING …` (or is `#[ignore]` with its reason); visible, documented | low (cleared) | none |
| the other 45 tests that call nothing from the crate (list: `crates/vocab/src/table.rs:2145` … `crates/core/src/universe.rs:4772`) | `assert_eq!(CONSTANT, literal)` pins of production constants (strides, sentinels, limits, rung lists) | a pin of a value the code holds is an assertion about the code | cleared | none |

`crates/store/tests/cited_commits.rs:637` `every_commit_the_store_records_cite_is_on_main_or_says_why_not`
**fails on this branch** because the snapshot is one commit: commits
`3cc6b8e0` and `521066e6` cited in `docs/04-invariants.md` and
`docs/06-limits.md` are not ancestors of `HEAD` here. The test reads only those
documents and git ancestry, and this branch edits neither cited document, so it
fails on the base for the same reason (reasoned, not run on the base);
UNVERIFIED on `feat/db-fix`, whose full history it expects.

## Lens 4 — append-only and durable writes

**(f) No writable memory mapping exists.** No crate maps memory: `memmap2`
appears only in the root `[workspace.dependencies]` (`Cargo.toml:95`) and no
member declares it; every crate root forbids `unsafe` (`crates/store/src/lib.rs:61`,
`crates/api/src/lib.rs:40`, …; `Cargo.toml:73`). Gate 24's pattern
(`.github/workflows/ci.yml:3912`, `MmapMut|MmapRaw|map_mut(|map_anon(|map_copy(`)
would miss `make_mut()`, `map_raw(`, `libc::mmap`/`PROT_WRITE` and a `memmap2`
line in a member manifest — all of which need `unsafe`, so the forbid lint
closes the practical gap (defence-in-depth finding, low).

**§3.8 store format versions are not mutated in place**: bar v2 files stay v2
for life (`docs/02-store-format.md:96-123`); the v2 run ledger is read-only
(`crates/cli/src/results.rs:1319-1328`); the census v1→v3 republish is a
documented whole-file tmp+rename of a derived counter (`docs/02-store-format.md:1294-1312`,
low tension with append-only). No reader accepts one version with two layouts.

### Helpers, audited once each

| helper | tmp + sync + rename/link | dir fsync | half-written final name visible | rewritten in place / torn tail | durability errors | verdict |
|---|---|---|---|---|---|---|
| `store::file::BarFile::append` (`crates/store/src/file.rs:2756-2987`) | final name by design: records, barrier, `.crc`, `.tix`, header slot, barrier | new `.crc` (`file.rs:2430-2432`), created dirs (`4800-4815`) | no — reader shared lock vs writer exclusive (`file.rs:90-104`); bytes past `n_valid` never served | only the free A/B header slot, CRC'd | propagated; a failed barrier is remembered (`file.rs:2720`, D-1907) | cleared |
| `store::sort_index_writer::write_image` (`548-579`) | `create_new` tmp, sync, rename | yes (`578`) | no | run append in place, checksummed | `let _cut = file.set_len(at)` (`306`) ignores a failed rollback; the original error is returned and the next walk checks the run | cleared (low) |
| `store::verdict::publish` (`3089-3111`) | staging, sync, `hard_link`, remove tmp | yes (`3109`, `3269`) | no | never overwrites a linked image | tmp removed on failure | cleared |
| `cli::fixed_tail` (`fixed_tail.rs:84-91, 164-189, 219-308`) | in-place append | caller's | — | torn tail healed under the lock | **rolls back** with `set_len` + `sync_all` | cleared (model) |
| `cli::append_rollback` (`append_rollback.rs:12-14, 68-75`) | in-place append | caller's | — | none; readers refuse a torn tail forever | a failed barrier after a whole write is **left in place**; the rollback `set_len` is unsynced | finding F-1/F-4 |
| `cli::search_checkpoint::publish_*` (`215-336`), `cli::expression::EvidenceWriter::finish` (`360-397`), `pull::alerts` (`657-688, 1740-1790`), `pull::masters::replace_locked` (`975-1010`), `pull::census_store::write_whole` (`1053-1060`) | tmp + sync + rename / link | yes | no | — | propagated | cleared |

### Findings

| where file:line | what | why it breaks the law | severity | fixed here / exact fix |
|---|---|---|---|---|
| `crates/api/src/autopilot.rs:783` (`probe_io`) | `File::create` at the probe name follows a symlink and truncates its target; `remove_file` then removes only the link | a store file linked there was **overwritten with the probe line** — measured: the before-run test found the victim holding `"brutex write probe\n"` | medium | **fixed here**: remove the name, then `create_new` — `autopilot::tests::the_write_probe_never_writes_through_a_link_at_its_name` |
| `crates/cli/src/candidate_trades.rs:1313-1327` (`write_exact`) | a failed write or barrier left the partial/unacknowledged file at its final immutable name | a torn file wedged the identity; a whole one was accepted by the next run over bytes no barrier reached — D-1915 fixed exactly this in `boolean_candidate_persistence::write_or_equal` | medium | **fixed here**: barrier through `fixed_tail::sync_all_hooked`, file withdrawn on failure — `candidate_trades::tests::a_failed_detail_barrier_withdraws_the_file_and_the_retry_writes_it` |
| `crates/cli/src/results.rs:972` (and the same shape at `trades.rs:482, 536`, `frontier.rs:908-916, 962`, `result_set.rs:313-324, 1052`, `selection.rs:1776-1789`, `selection_v3.rs`, `selection_v4.rs`, `global_replay*.rs`, `stored_data_completeness.rs:724-773`, `institutional_statistics.rs:1321`, `candidate_universe.rs:6263-6283`, `scrub_blocks.rs:478-485`, `pool_oos.rs:466`) | a new ledger file's directory entry is never fsynced | a crash can lose `runs.bin` itself, and `Results::open` then reads "No run has been recorded" — D-1903's rule applied only to Statistics V2/V3 and Observations | medium | **fixed here for `results.rs`**: `sync_directory(&dir)` and `sync_directory(root)` after the fresh header — `results::tests::creating_the_run_ledger_syncs_its_directory_and_the_root` (source shape, D-1903's pattern; a dirent's durability cannot be observed without a power cut); the guard `the_ledger_is_fsynced_and_never_merely_flushed` now counts three barriers and names the third. Finding for the rest: the same two lines after each first header write |
| `crates/pull/src/request_minutes.rs:925-935` (`append_named`) | a torn last line of `audit/<vendor>.gaps` was not closed, so the next run's first line glued onto it | one fault where there were two lines; D-2826 fixed this for every GDFL ledger | low | **fixed here**: appends through `gdfl_import::append_lines` — `ingest::request_minutes::tests::a_torn_gap_line_is_closed_before_the_next_append` (failed before) |
| F-1: raw barrier with no rollback, and a reuse path that re-syncs and vouches: `anchored_search_lineage_v2.rs:835-837, 862-864`; `_v3.rs:901-903, 928-930`; `_v4.rs:996-999, 1023-1026`; `population_admission_v2.rs:2192-2194, 2203-2205`; `population_finalization_v2.rs:1703-1705, 1748-1750`; `population_v5.rs:2543-2545, 2622-2624`; `selection.rs:1984-1986, 2320-2322`; `selection_v3.rs:1323-1325`; `selection_v4.rs:1910-1912`; `selection_v5.rs:2107-2110, 2225-2228`; `selection_v6.rs:362, 369` (reuse `321`); `execution_capability.rs:1760-1789`; `execution_disposition_v2.rs:2416-2445`; `global_replay_v4_store.rs:64, 71`; `checksum_receipts.rs:345, 354`; `pull/src/cash_session_cache.rs:704-711`; `api/src/audit.rs:1750-1751, 1823-1824`; `cli/src/operation_audit.rs:357-369, 631, 662` | a failed `sync_all` after a whole append leaves the bytes; a later barrier on a fresh descriptor "confirms" them | D-1900: on Linux a failed fsync marks pages clean, so a second barrier proves nothing; D-1915 lists four of these as open and omits the `append_rollback` users; `selection_v6` and `global_replay_v4_store` are on the execution-authority path | medium | finding: per module, capture the block start and replace the barrier with `fixed_tail::sync_or_roll_back(&file, &path, start, File::sync_data)`; call `fixed_tail::refuse_after_failed_barrier(&path)?` before each reuse re-sync; sync `append_rollback`'s cut |
| `crates/api/src/recovery_journal.rs:343-345, 502-507` | reopen re-syncs a record whose sync failed; `sync_failure_preserves_old_index_and_reopen_recovers_the_uncertain_record` (`1776-1811`) asserts it | the D-1900 argument; documented in `docs/02-store-format.md` | low-medium | finding: needs a decision extending D-1900 to `api` |
| `crates/pull/src/census_store.rs:2240-2249` (`seal_legacy`) | `hard_link(legacy → superseded)` then `install_stub` renames over `legacy` with one dir fsync at the end (`:2294`) | the rename can become durable without the link (`alerts.rs:1740-1745` orders it right) | low | finding: `legacy.parent().map_or(Ok(()), sync_dir)?;` after the link |
| `crates/api/src/audit.rs:2742-2753` (`write_cover`), `2722-2739` | the 16-byte cover rewritten in place, unsynced; new index files never dir-synced | the cover can outlive a new `refusals/<src>.idx` dirent, which `read_or_empty` (`3085-3091`) reads as "no refusals" | low | finding: `write_all_at(..).and_then(|()| file.sync_all())`; dir-sync a newly created index file's parent |
| `crates/pull/src/gdfl_cm_auction.rs:369-373` (`write_day`) | a failed write leaves a partial record that `parse` (`220-231`) refuses forever | D-1854's rollback not applied in `pull` | low | finding: `set_len(at)` + `sync_all` on a failed write or sync |
| `crates/cli/src/boolean_candidate_persistence.rs:166-172` | `committed()` is length-only; a reader can see the 112-byte receipt before its fsync returns | (c) final name visible before durable | low | finding: write `complete.bin.tmp`, sync, `hard_link`, remove, dir-sync |
| `docs/05-decisions.md:58026-58029` (D-1915) | lists Admission V4 as open; the code rolls back (`population_admission_v4.rs:2804-2825`) and the F-1 modules are missing | stale record | low | finding: a new decision closing Admission V4 and listing F-1 |

## Lens 5 — money

Method: every non-test line with a float type, float literal, float method or
`as` cast near float math, cross-checked against every `float_arithmetic` and
`cast_possible_truncation` exemption (both lints are denied workspace-wide at
`Cargo.toml:85-86`); 661 lines classified.

| where file:line | what | why it breaks the law | severity | fixed here / exact fix |
|---|---|---|---|---|
| `crates/runner/src/outcome.rs:2175-2176, 2313-2322, 2382` (`edge`), read at `outcome.rs:1419, 1537, 1598, 1642`, `validate.rs:1798-1803`, `cli/src/lib.rs:6679` (LOCKED) | `Edge::mean_paisa` was Welford's running mean, and its sign picks the traded side (`mean_paisa < 0.0` is Short, "exactly zero is Long"). Moves `[-9, -3, 7, 5]` sum to 0 and leave Welford at `-2.22e-16` | a sample with no edge traded Short, and the side-dependent ratios (`payoff_bp`, `path_ratio_bp`, `worst_reward_risk_bp`) read the short's figures; these rank the Payoff, Path and Asymmetry lenses (`rank.rs:378-418`) | medium | **fixed here**: the reported mean is the exact `i128` sum over the count (`exact_mean`), and `t` is formed from it; Welford stays for `m2` — `outcome::tests::an_exactly_zero_sum_sample_has_a_zero_mean_and_is_read_long` (did not compile before). Full runner suite 740/740 after |
| `crates/core/src/price.rs:10-15, 30-33` | the module doc said a JSON price is "rendered back to its shortest round-tripping text" from an `f64` | stale since D-1570 (`serde_json` `arbitrary_precision`, `Cargo.toml:117`; `pull/src/http.rs:1514-1531` passes the vendor's digits) — §3.6 honest limits | low | **fixed here** (doc only) |
| `crates/lake/src/bar.rs:161-167` → `core/src/price.rs:113-171` | lake rupee doubles snapped with `from_rupees_half_up(f64)`: `0.145` → 14 | latent (no manifest depends on `lake`); exact for every two-decimal value; documented (D-1494) | low | finding: refuse an off-grid scaled value if `lake` ever gains a consumer |

Decided float use, listed not violations: `Edge` money sums are exact
`i128`/`i64` converted once (D-1173, D-1178, `outcome.rs:1250-1329`); the
`.grk` row's volatility and greeks (D-0046, D-0692, D-1524,
`store/src/format.rs:531-558`); model inputs in `pull/src/pricing.rs:612-635`
(D-2830; no model price is stored or used as a trade level); the one sanctioned
float price function, half-up via floor-and-compare refusing NaN
(`core/src/price.rs:112-171`, D-0061, D-0786). The vendor write boundary reads
decimal text digit by digit and snaps half-up once (`pull/src/http.rs:1579-1642`,
`rolling.rs:957-975`, `gdfl_cm.rs:2734-2754`): `"123.455"` gives 12346.
`crates/costs` (LOCKED), `indicators`, `engine`, `vocab` and every trade,
excursion and grid P&L hold no float token in non-test code.

## Lens 6 — depth

| where file:line | what | why it breaks the law | severity | fixed here / exact fix |
|---|---|---|---|---|
| `crates/engine/src/lib.rs:1013-1021` (`Ladder`), `:1644` | the ladder carries `min_hits`, `ceiling`, `pair_budget`, `support_lanes` — no depth; the loop is `while !current.frequent.is_empty() && progress.halted.is_none()`, a halt is recorded in `Sweep::halted` (`:698`) and `completed()` is false (`:729-731`) | — | cleared | none: §6 holds |
| `crates/cli/src/expression_search.rs:571, 577, 584` with `crates/cli/src/search_checkpoint.rs:14` (`DIRECTORY_LIMIT = 1_000_000`), refusal `:244-248` | one checkpoint per emitted candidate; the namespace refuses its millionth entry, so expression search V1 cannot emit more than ~1M candidates per run identity whatever CANDIDATES/NODES say | the report text (`expression_search.rs:340`, "work budgets, not a depth limit") and `vocab/src/expression_search.rs:3-5` ("the representation's existing capacity is the only size bound") are false past this wall — §3.6. The stop is loud, so it is not a silent truncation; D-1563 documents the mechanism, not this effect | medium | finding: checkpoint every N candidates or compact the journal (needs a decision); at minimum state the wall in that report text, that module doc and `docs/06-limits.md` |
| `crates/engine/src/lib.rs:884-894` | said "no production caller sets the ceiling at all" | false: `cli/src/lib.rs:4754, 4899, 15293, 19343`, `cli/src/batch.rs:711`, `runner/src/lib.rs:1138` set it; D-0305 (`docs/05-decisions.md:22836`) withdrew that sentence of D-0304 | low | **fixed here** (doc only) |
| `crates/cli/src/lib.rs:4640` (LOCKED) | says `DEFAULT_CEILING` = `1 << 26`; the constant is `1 << 27` (`engine/src/lib.rs:839`) | stale arithmetic | low | finding only (locked) |

Separate searches with decided bounds (not §6 violations): `MAX_INSTRUCTIONS = 1151`
(`vocab/src/expression.rs:15`, D-0523: a separate versioned explicit-expression
route, not an Apriori variant); `MAX_NESTING = 32` (D-0523); CANDIDATES / NODES
budgets that pause loudly (D-0752); the Boolean grammar allowances
(`api/src/booleanlaunch.rs:340-346`) that pause or refuse. Resource guards on
the ladder all stop loudly and are in the identity: `DEFAULT_CEILING`,
`DEFAULT_PAIR_BUDGET`, `Breach::Memory/Workers` (`engine/src/lib.rs:839, 905,
1733-1747`), `BRUTEX_CEILING` (D-0305), `BATCH_CEILING` (`cli/src/batch.rs:110`).
No environment variable read anywhere sets a depth.

## Lens 7 — docs against code

**CLAUDE.md §5's crate graph equals `cargo metadata --no-deps` exactly.**
Measured (`cargo metadata --no-deps --format-version 1 --locked`, normal
dependencies between workspace members; no dev- or build-dependency arrows
exist): `core`, `vocab`, `greeks`, `telemetry` depend on nothing; `costs` →
core; `store`, `lake` → core, telemetry; `indicators`, `engine` → vocab; `pull` →
core, costs, greeks, store, telemetry; `api` → cli, core, pull, store,
telemetry, vocab; `runner` → core, costs, engine, indicators, vocab; `cli` →
core, costs, engine, indicators, pull, runner, store, telemetry, vocab. Thirteen
members, `Cargo.toml:23-…` lists exactly the thirteen `crates/` directories.

**Every gate CLAUDE.md names exists** in `.github/workflows/ci.yml`: gate 1
(`:28`), 1b (`:244`), 1c (`:330`), 1e (`:116`), 7 (`:1805`), 9 (`:1593`), 9b
(`:1683`), 17 (`:2708`), 22 with its clauses (`:3594`), and the bench gate,
gate 8 (`:7998`). Every code fact CLAUDE.md states was found where it says:
`engine::column::Column::support` (`crates/engine/src/column.rs:105`),
`vocab::ConditionMask::hits` (`crates/vocab/src/mask.rs:144`), `Column::build`'s
`for (index, bar) in bars.iter().enumerate()` (`crates/indicators/src/column.rs:662`),
`engine::primitives::append` (`crates/engine/src/lib.rs:149`), `drain`
(`:2372`), the pre-sized k=1 `HashSet<u32>` (`:1520-1521`), `GridProgress::tick`
(`crates/cli/src/lib.rs:21174`), `FNO_UNDERLYINGS` 213 less 5 indices = 208
(`crates/core/src/universe.rs:914, 1159`), `vocab::table::TABLE` of 370 rows
(`crates/vocab/src/table.rs:257`), `the_generated_and_stored_banners_make_opposite_claims`
(`crates/cli/src/lib.rs:25803`), and the C-E-02/C-E-09/C-E-11 bench functions
(`crates/engine/benches/ratio.rs:330, 436, 589`).

### Invariant rows that name a test that does not exist

Gate 10's qualified-token loop, run locally on this tree (the step's own
script from `.github/workflows/ci.yml:4443-4638`): **3,695 rows read, 3,257
named tests checked, 15 missing — every one in a `✓` row.**

| where | row | names | what exists instead | severity | exact fix |
|---|---|---|---|---|---|
| `docs/04-invariants.md:2587` | SS-06 | `api::server::instrument_bar_counts_are_one_pass_over_the_census` | nothing | medium | write the pass-count test, or mark the row `✗` |
| `docs/04-invariants.md:3080` | MR-34 | `api::calendar_route_tests::calendar_reads_current_feed_and_cash_identity_after_startup` | nothing | medium | same |
| `docs/04-invariants.md:5344` | AF-23 | `api::server::census_request_tests::another_feeds_rewrite_rereads_every_manifest_once` | `another_feeds_rewrite_leaves_this_feeds_manifest_unread` (`crates/api/src/census_request_tests.rs:841`) — **the opposite claim** | medium | rewrite the row's sentence to the behaviour that test pins and cite the decision that changed it |
| `docs/04-invariants.md:5414` | AF-28j | `api::server::census_request_tests::a_fault_the_census_cache_declines_rebuilds_the_store_body_on_every_request` | nothing | medium | write it or mark `✗` |
| `docs/04-invariants.md:5620` | PIF-02 | `pull::anchor::an_unmeasured_length_day_is_withheld_once_with_its_own_reason` | nearest `an_unsized_day_is_one_note_for_its_own_reason_and_counted` (`crates/pull/src/fold.rs:2146`), UNVERIFIED same claim | medium | re-point after reading it, or mark `✗` |
| `docs/04-invariants.md:5635` | APIC-05 | `api::server::calendar_route_tests::a_symbol_this_feed_holds_no_file_for_is_not_named_in_from` | nothing | medium | write it or mark `✗` |
| `docs/04-invariants.md:5760` | C4-CLI-03-03 | `cli::fold_audit::tests::a_legacy_plain_fold_of_a_withheld_session_disagrees` | `a_legacy_plain_fold_of_a_withheld_bucket_disagrees` (`crates/cli/src/fold_audit.rs:696`) — D-3321 made a plain fold of a special session's complete windows **agree** | medium | rewrite the row to "a withheld bucket" per D-3321 and re-point |
| `docs/04-invariants.md:6274` | AFA-13 | `pull::anchor::an_exceptional_session_is_named_once_per_day_not_once_per_bucket` | nearest `a_day_the_venue_refuses_is_named_once_not_once_per_bucket` (`crates/pull/tests/anchor.rs:849`), UNVERIFIED same claim | medium | re-point after reading, or mark `✗` |
| `docs/04-invariants.md:6409` | ZR-04 | `api::server::tests::an_instrument_counts_only_its_own_one_minute_spot_bars` | nothing | medium | write it or mark `✗` |
| `docs/04-invariants.md:6530` | ZX-20 | `pull::session::tests::an_unmeasured_muhurat_minute_is_refused_by_name_not_dropped` | nothing (the row's other test exists) | medium | write it or mark `✗` |
| `docs/04-invariants.md:6768` | AFG-71 | `pull::request_gap_events::each_request_minute_gap_is_exactly_one_error_event` | `each_month_and_reason_of_request_minute_gaps_is_exactly_one_error_event` (`crates/pull/tests/request_gap_events.rs:82`) — one event per month and reason, not per line | medium | rewrite the row to the aggregated rule and re-point |
| `docs/04-invariants.md:6915` | GI-01 | `pull::gdfl_import::tests::an_option_second_carries_the_last_open_interest_of_its_traded_rows` | `an_option_second_carries_the_open_interest_of_its_last_row_traded_or_not` (`crates/pull/src/gdfl_import_tests.rs:123`) — a changed rule | medium | rewrite and re-point |
| `docs/04-invariants.md:6921` | GI-07 | `pull::gdfl_import::tests::a_day_older_than_what_the_month_holds_is_refused_by_the_store` | `a_day_older_than_what_the_month_holds_lands_as_a_new_generation` (`gdfl_import_tests.rs:2290`, D-3350) — **the opposite**: the row still says "refused by the store" | high (the row asserts a refusal the code no longer makes) | rewrite per D-3350 |
| `docs/04-invariants.md:6921` | GI-07 | `pull::gdfl_import::tests::a_census_that_will_not_publish_fails_the_day_by_name` | `a_census_that_will_not_publish_stops_the_run_at_that_day_by_name` (`gdfl_import_tests.rs:2535`, D-3317) — the row says "the run continues" | high | rewrite per D-3317 (`docs/05-decisions.md:65395`) |
| `docs/04-invariants.md:6924` | GI-10 | `pull::gdfl_nfo::tests::an_expiry_on_a_weekday_holiday_is_kept_as_the_vendor_stated_it` | nothing | medium | write it or mark `✗` |

Not edited here: six of the fifteen describe behaviour a later decision
reversed, so the fix is a sentence the operator should own, not a re-point.

Gate 10's first half (`invariant_paths.rs`) also exits 1 here, for a reason
that is the snapshot's: `main`'s workflow (kept by the snapshot) builds its
function table from `crates/*` only, while `feat/db-fix`'s rows name 107 tests
that live in `.github/*.rs` (all found, e.g. `p_03_left_the_pending_allowlist`
at `.github/gates_ledger.rs:3446`). Four rows give a wrong module path for a
test that exists: `docs/04-invariants.md:4492` (`audited_stored::range::tests::…`,
really `crates/cli/src/audited_range_tests.rs:408`), `:4604`
(`research_policy::adversarial_tests::…`, really `crates/cli/src/research_policy_tests.rs:66`),
`:5032-5033` (`server::store_wire::tests::…`, really `crates/api/src/store_wire.rs:337, 484`) —
low, re-point.

### Decision ids cited in code with no entry in the ledger

Every `D-NNNN` in `crates/**` was checked for a heading in
`docs/05-decisions.md` (any `#`-heading that names it). **35 have none.**

| id | cites in `crates/` | where first cited | note |
|---|---|---|---|
| D-3349 | 84 | `crates/api/benches/ratio/window.rs:12` | the ledger admits it: "D-3349 to D-3353 are cited in code by WP-39, WP-13c and WP-16, so this package's entries start here" (`docs/05-decisions.md:67014-67016`) — the five entries were never written |
| D-3350 | 119 | `crates/api/src/autopilot.rs:3832` | as above; the month-generation rule that reversed GI-07 |
| D-3351 | 52 | `crates/api/src/repair_route.rs:223` | as above |
| D-3352 | 2 | `crates/pull/src/calendar.rs:86` (LOCKED) | as above |
| D-3353 | 60 | `crates/cli/src/gdfl_import_command.rs:87` | as above; the batch-durability window |
| D-0051, D-0805 | 3, 5 | `crates/api/src/server.rs:3202`, `crates/pull/src/gdfl_cm.rs:41` | mentioned in the ledger's body text, no heading |
| D-2018, D-2026 | 1, 1 | `crates/pull/tests/derive.rs:1735, 1737` | not in the ledger at all |
| D-3160 … D-3199 (19 ids) | 1–11 each | `crates/pull/src/gdfl_*_attack_tests.rs` | attack-round ids; some appear in body text |
| D-4841, D-4854, D-4887, D-4894, D-4896, D-4906, D-4931 | 1–3 each | `crates/store/src/verdict.rs:425, 625, 207`, `store/tests/month_repair.rs:164`, `cli/src/alerts_command.rs:367`, `api/benches/ratio/logs_dirs.rs:285`, `store/src/sort_index_writer.rs:32` | — |

Severity high for D-3349 to D-3353 (§9: "a `docs/05-decisions.md` entry exists
for every locked choice"; 317 citations point at nothing). Exact fix: write
the five entries from WP-39, WP-13c and WP-16 — not done here, because their
content is not in this tree to transcribe and writing it would be invention
(§3 rule 1).

## Lens 8 — extreme cases

### Coverage matrix (what existed before this branch)

| extreme input | store readers | importer (GDFL) | api routes |
|---|---|---|---|
| empty / zero records | writer TESTED (`store/tests/write.rs:229`); **reader door on a zero-byte `.bin` UNTESTED** → added | TESTED (`gdfl_import_tests.rs:249`, `gdfl_cm.rs:4730`, `gdfl_archive.rs:1411`) | TESTED (`imports.rs:1767`, `bars_window_route_tests.rs:215`) |
| one record | TESTED n=0..150 (`file.rs:7690`) | UNVERIFIED (nearest `gdfl_import_tests.rs:208`) | TESTED (`bars_window_route_tests.rs:491`) |
| maximum size / header claims more | TESTED (`write.rs:1945`, `:773`, `:920`; `fault.rs:693, 782`; `file.rs:7731`) | TESTED (`gdfl_cm.rs:4624`; bombs `gdfl_seconds_attack_tests.rs:1117, 1281`) | TESTED (`serve_edge_tests.rs:95`, `detail.rs:1763`, `bars.rs:2646`) |
| duplicate | TESTED (`write.rs:424, 465, 536-562`) | TESTED (`gdfl_import_tests.rs:233`, `gdfl_archive.rs:1194`) | TESTED (repeated key 400, `serve_edge_tests.rs:58`) |
| out of order | TESTED (`write.rs:536`, `fault.rs:807, 874`) | TESTED (`gdfl_import_tests.rs:179, 221`) | TESTED (from > to refused, `bars_window_route_tests.rs:138`) |
| late (after the month holds later days) | refused `OverlapDisagrees` (`write.rs:1851`) | TESTED as a new generation (`gdfl_import_tests.rs:2290`) | N/A |
| torn write | TESTED (`file.rs:7450`, `write.rs:736, 1602`, `fault.rs:108`) | TESTED (`gdfl_import_tests.rs:2132, 4844, 5445`) | TESTED (`bars_sort_tests.rs:829`, `audit.rs:4263`) |
| truncated | TESTED (`write.rs:894, 973, 998`, `fault.rs:768`) | TESTED (`gdfl_archive.rs:1521, 1050`) | window TESTED (`bars_window_route_tests.rs:360`); **`/bars.json` on a truncated month UNTESTED** → added |
| missing in the middle | N/A (the store admits gaps; sessions belong to `pull`) | TESTED (`gdfl_import_tests.rs:841, 2227`) | TESTED (`pull/src/gaps.rs:837, 747`) |
| renamed symbol | TESTED (`store/tests/misfiled.rs:195`) | TESTED (`gdfl_wp12e_tests.rs:479, 566, 625`) | TESTED (retired key 410, `bars_window_route_tests.rs:1978`) |
| special session | N/A | DR/Muhurat TESTED (`gdfl_cm_admission_tests.rs:344, 404, 450`); **Saturday 2025-02-01 UNTESTED through the importer** → added | Muhurat TESTED (`pull/src/gaps.rs:865`) |
| concurrent readers + a writer | concurrent readers TESTED (`cold_read.rs:354`); **reader-door vs writer-door UNTESTED** → added | **two runs into one store UNTESTED, no run lock** → added | pool admission TESTED (`spot_route_tests.rs:513`, `detail.rs:1303`) |
| killed mid-write | TESTED (`staging_tests.rs:45, 94, 134`) | TESTED (`gdfl_import_tests.rs:2086`, `census_store_tests.rs:203`) | N/A |
| disk full | scripted host only (`file.rs:7125`); **a real ENOSPC UNTESTED** → added | classified the same way, no end-to-end | N/A |
| read-only dir | TESTED root-safe (`write.rs:697`) | **3 tests failed as root** → fixed | **10 tests failed as root, 2 passed vacuously** → fixed (lens 3) |
| garbage / signed / huge parameters | N/A | N/A | overflow strings refused (`booleanjson_tests.rs:102` …); **window month, chain expiry and sources month accepted signed or mis-padded fields** → 3 bugs fixed |
| 20 concurrent requests | N/A | N/A | pool-level admission TESTED (`server.rs:34705`); no end-to-end 20-request test (UNVERIFIED gap, not added) |

### The ten tests added for the most dangerous untested cases

| # | test (file) | case | before this branch | after |
|---|---|---|---|---|
| 1 | `server::bars_window_route_tests::the_window_refuses_a_month_not_spelled_yyyy_mm` (`crates/api/src/bars_window_route_tests.rs`) | `/bars/window.json?from=2025-1`, `02025-01`, `%2B2025-01`, `2025-%2B1` | **failed**: `2025-1` read as January (`bars_route.rs:498-505` split on `-` and `str::parse`d) | passes: the one strict `bars::month_of_text` |
| 2 | `server::chain::tests::an_expiry_with_a_signed_field_is_refused_not_read_as_another_spelling` (`crates/api/src/chain_route_tests.rs`) | `/chain.json?expiry=2019-+3-28` | **failed**: read as 2019-03-28 (`chain.rs:189-199` checked widths only) | passes: digits only |
| 3 | `server::sources::tests::a_signed_month_file_name_is_not_listed_as_a_held_month` (`crates/api/src/sources_tests.rs`) | a month file named `2024-+3.bin` | **failed**: listed as a held March (`sources.rs:510-516`) | passes: the one strict parser |
| 4 | `server::bars_window_route_tests::the_month_route_names_a_held_or_truncated_month_and_never_answers_it_empty` | `/bars.json` while a writer holds the month, and on a month cut to its header | passed (pins the named refusal against a regression to `200 []`) | passes |
| 5 | `a_reader_and_a_writer_of_one_month_refuse_each_other_by_name` (`crates/store/tests/write.rs`) | reader door vs writer door on one month | passed (pins `Locked`, and that the refused writer wrote nothing) | passes |
| 6 | `the_reader_door_refuses_a_zero_byte_month_and_creates_nothing` (`crates/store/tests/write.rs`) | zero-byte `.bin` at the reader door | passed (pins `Format { NoValidHeader }` and no `.lock`/`.crc`/`.tix` created) | passes |
| 7 | `file::tests::the_kernels_own_enospc_from_dev_full_is_disk_full_by_name` (`crates/store/src/file.rs`) | a real kernel `ENOSPC` through `/dev/full` | passed — turns `write.rs:9-15`'s "UNVERIFIED" into a measurement: the kernel's error reaches `DiskFull` by name | passes |
| 8 | `gdfl_import::tests::the_budget_saturday_2025_02_01_is_imported_and_an_ordinary_saturday_is_closed` (`crates/pull/src/gdfl_import_tests.rs`) | the charter's Saturday session through the importer | passed (pins it) | passes |
| 9 | `gdfl_import::tests::two_runs_into_one_store_at_once_leave_what_one_run_leaves` (`crates/pull/src/gdfl_import_tests.rs`) | two concurrent import runs, no run lock | passed: per-month locks, the census lock and the journal leave exactly one run's bars and a rerun writes nothing (one run on one day; UNVERIFIED for wider spans) | passes |
| 10 | `a_journal_that_cannot_be_written_fails_the_day_by_name`, `a_day_ledger_that_will_not_take_a_line_leaves_the_day_open`, `gdfl_tickstore::tests::a_temporary_or_absent_day_is_none_and_an_unreadable_one_refuses` | read-only journal / ledger / tick-store day as root | **failed as root** (mode bits do not bind uid 0) | pass: bodies run where the bits bind (D-0995's `where_permission_binds`) |

## Every test added or changed on this branch

"Failed before" means the test was run on this tree with the source fix
removed and the test present, and failed for the reason it names; "did not
compile before" means it calls the item the fix adds; "pins" means it passed
before and holds existing correct behaviour that had no test.

| crate | test | lens | before the fix |
|---|---|---|---|
| api | `audit::tests::a_damaged_fault_or_refusal_entry_is_counted_or_refused_not_dropped` | 2 | failed (`(faults, malformed)` read `(0, 0)`) |
| api | `audit_json::tests::an_index_that_lost_a_run_entry_is_walked_and_says_why` | 2 | failed (`index_covered` was `Some(5)`: the damaged index was trusted) |
| api | `autopilot::tests::the_write_probe_never_writes_through_a_link_at_its_name` | 4 | failed (the linked file held `"brutex write probe\n"`) |
| api | `server::bars_window_route_tests::the_window_refuses_a_month_not_spelled_yyyy_mm` | 8 | failed (`2025-1` accepted) |
| api | `server::bars_window_route_tests::the_month_route_names_a_held_or_truncated_month_and_never_answers_it_empty` | 8 | pins |
| api | `server::chain::tests::an_expiry_with_a_signed_field_is_refused_not_read_as_another_spelling` | 8 | failed (`2019-+3-28` → 2019-03-28) |
| api | `server::sources::tests::a_signed_month_file_name_is_not_listed_as_a_held_month` | 8 | failed (`2024-+3` → March 2024) |
| api | `bars::sort::tests::a_host_refusal_of_an_index_is_not_remembered`, `…_met_while_reading_an_index_is_this_requests_alone` (changed) | 3 | passed vacuously as root (returned before asserting); now run where mode bits bind, and assert they bind |
| api | `logs::tests::{a_directory_that_turns_unreadable_is_never_answered_as_read, a_ring_file_that_stays_unreadable_is_named_once, logs_names_a_directory_it_may_not_search}`, `pullrun::tests::{an_index_that_cannot_be_read_never_leaves_the_answer_clean, an_absent_index_is_built_by_the_scan_and_not_known_is_said_when_it_cannot_be}`, `server::tests::{a_failure_of_the_store_keeps_the_leg_owed_and_is_never_called_standing, a_probe_name_that_cannot_be_unlinked_is_a_fault_by_name, a_run_cut_short_by_a_restart_is_logged_when_it_is_found, a_store_that_cannot_take_a_write_is_found_by_asking_it, an_expired_fno_walk_cut_short_is_seen_at_the_next_start}` (changed) | 3 | failed as root; each passed as uid 65534; bodies now run where mode bits bind |
| cli | `srt_builder::tests::a_dry_run_names_an_uninspectable_name_rather_than_reading_it_as_absent` | 2 | not run before (committed with its fix; `name_refused` returned `None` for `ENOTDIR` by construction of `.ok()?`) |
| cli | `series_identity::tests::an_unreadable_listing_entry_refuses_rather_than_shortening_the_table` | 2 | did not compile before (drives the new shared helper) |
| cli | `candidate_trades::tests::a_failed_detail_barrier_withdraws_the_file_and_the_retry_writes_it` | 4 | failed (`write_exact` returned `Ok` past an injected barrier failure) |
| cli | `results::tests::creating_the_run_ledger_syncs_its_directory_and_the_root` | 4 | failed (source shape) |
| cli | `results::tests::the_ledger_is_fsynced_and_never_merely_flushed` (changed) | 4 | counts three direct barriers now and names the third |
| pull | `census::a_contract_field_that_does_not_read_back_is_refused_not_read_as_spot` (`crates/pull/tests/census.rs`) | 2 | did not compile before (`EntryFault::ContractUnreadable` is new) |
| pull | `census_store::tests::a_migration_receipt_that_cannot_be_read_is_named_not_read_as_no_drift` | 2 | failed: run against the base's `census_store.rs`, it panicked at `census_store_tests.rs:1570:51` with "an unreadable receipt is named". A non-UTF-8 receipt was read as no drift (`None`) |
| pull | `gdfl_rekey::tests::a_key_withdrawn_twice_with_different_records_refuses_the_ledger` | 2 | failed |
| pull | `ingest::request_minutes::tests::a_torn_gap_line_is_closed_before_the_next_append` | 4 | failed (`"gap TORNgap NIFTY …"`) |
| pull | `gdfl_import::tests::the_budget_saturday_2025_02_01_is_imported_and_an_ordinary_saturday_is_closed` | 8 | pins |
| pull | `gdfl_import::tests::two_runs_into_one_store_at_once_leave_what_one_run_leaves` | 8 | pins |
| pull | `gdfl_import::tests::{a_journal_that_cannot_be_written_fails_the_day_by_name, a_day_ledger_that_will_not_take_a_line_leaves_the_day_open}`, `gdfl_tickstore::tests::a_temporary_or_absent_day_is_none_and_an_unreadable_one_refuses` (changed) | 3/8 | failed as root; now run where mode bits bind |
| runner | `grid::tests::a_sub_tick_entry_low_refuses_the_path_rather_than_charging_a_zero_entry` | 2 | failed (the path was measured and booked) |
| runner | `outcome::tests::an_exactly_zero_sum_sample_has_a_zero_mean_and_is_read_long` | 5 | did not compile before (`exact_mean` is new); its witness asserts Welford's `-2.22e-16` |
| store | `a_reader_and_a_writer_of_one_month_refuse_each_other_by_name` (`crates/store/tests/write.rs`) | 8 | pins |
| store | `the_reader_door_refuses_a_zero_byte_month_and_creates_nothing` (`crates/store/tests/write.rs`) | 8 | pins |
| store | `file::tests::the_kernels_own_enospc_from_dev_full_is_disk_full_by_name` | 8 | pins (first measurement of a real `ENOSPC`) |
| store | `staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported` (changed) | 3 | **failed once** in a full `cargo test -p store` run here: its `assert_ne!(ino)` raced inode reuse; the leftover is now held open so its number cannot be reissued, and the test also asserts the leftover's bytes were not cut |

## Gates

Run here on this branch's code (head `e52425a`; only `reports/` changed after
`9748776`, the last code commit), as uid 0 in this container, 4 cores, on
2026-10-16 between 05:26 and 05:58 UTC. Logs are in the session scratchpad;
none is committed.

| gate | command | result |
|---|---|---|
| format | `cargo fmt --all --check` | **clean**: exit 0, no output |
| lint | `cargo clippy --workspace --all-targets -- -D warnings` | **clean**: exit 0, no warning, 1m 40s |
| runner | `cargo test -p runner --locked --no-fail-fast` | **green**: exit 0. lib 740 passed; 12 integration binaries 46 passed; doc-tests 6 passed |
| store | `cargo test -p store --locked --no-fail-fast` | **exit 101, one failure, environmental**: lib 229 passed; 33 integration binaries, 412 passed, 1 failed, 4 ignored (32 green); doc-tests 11 passed. The one failure is `cited_commits::every_commit_the_store_records_cite_is_on_main_or_says_why_not` (`crates/store/tests/cited_commits.rs:637`), explained under lens 3: the snapshot base is one commit, so commits the docs cite are not ancestors of `HEAD`. This branch edits neither cited document |
| pull | `cargo test -p pull --locked --no-fail-fast` | **green**: exit 0. lib 1221 passed, 17 ignored (each `#[ignore]` carries its reason); 20 integration binaries, 375 passed, 0 failed, 6 ignored (`census` 21 passed, `census_v4` 9 passed and 1 ignored, `store_evidence` 5 ignored); doc-tests 38 passed |
| api | `cargo test -p api --locked --no-fail-fast` | **green as root**: exit 0. lib 1976 passed, 0 failed, 5 ignored (before this branch: 1966 passed, 10 failed as root, the ten permission tests lens 3 names); `main.rs` 3 passed; `tests/binary.rs` 4 passed; doc-tests 1 passed |
| cli | `cargo test -p cli --locked --no-fail-fast` | **exit 101, one failure, root-only**: lib 2035 passed, 1 failed, 1 ignored (731.9s); `main.rs` 0 tests; 14 integration binaries, 60 passed, 0 failed; doc-tests 1 passed. The one failure is `scrub_blocks::tests::an_empty_census_a_single_record_and_an_unwritable_ledger` (`crates/cli/src/scrub_blocks_tests.rs:611:53`). It passes when the same binary runs it as uid 65534 (lens 3; open finding 13) |
| engine | `cargo test -p engine --locked --no-fail-fast` | **green**: exit 0. lib 104 passed; 4 integration binaries, 27 passed; doc-tests 0 |
| core | `cargo test -p core --locked --no-fail-fast` | **exit 101, two binaries red, both from the snapshot's workflow file**: lib 192 passed; 6 of 8 integration binaries green (`graph` 15, `findings` 14, `cost_invariants` 5, `zerodha_index` 5, `bound_probe` 2, `lint` 2); doc-tests 4 passed. `banned_lists` fails 0 of 2 at `crates/core/tests/banned_lists.rs:81:10` ("the step declares its list as VARIABLE=…"), the panic CI's Gate 1e shows. `claim_reader` fails 2 of 3, at `crates/core/tests/claim_reader.rs:66:5` (gate 12's step does not run `"$tool" gate12 "$allow_claim"`) and `:124:9` (an allowance reads `["crates/cli/src/lib.rs", "2"]`, the old `FILE COUNT` form D-2314 replaced). Both binaries `include_str!` `.github/workflows/ci.yml` (`banned_lists.rs`, `claim_reader.rs:24`). On this snapshot that file is `main`'s copy, while the tests are `feat/db-fix`'s, and this branch changes neither. CI never reaches `claim_reader`: without `--no-fail-fast`, cargo stops at the first failing binary, `banned_lists` |

"Ignored" tests were not run by these commands, and none is counted as passed
above. Every `#[ignore]` attribute in the seven crates carries its reason. Each
reason falls in one of three groups: data outside the repository, named by an
environment variable (vendor archives, NSE samples, the operator's store); a
timing measurement meant for a release build; or, in one case, a fixture that
"peaks near 2 GB".

**Not run here, and why.** `cargo deny check`, line and branch coverage, and
mutation testing (CLAUDE.md §9) were not run: none of the three tools is
installed in this container, and the task's gate list did not name them.
Whether this branch keeps coverage at 100% and leaves no surviving mutant on
the modules it touched is **UNVERIFIED**. CI runs the repository's workflows,
which come from `main`, against this tree, which comes from `feat/db-fix`. Its
three red checks are Gate 1e (at `crates/core/tests/banned_lists.rs:81`), Gate
W (`web/build` drift), and `ci-ok`, which only rolls the others up. Neither
underlying failure is in a file this branch changes: `git diff --name-only`
against the base lists nothing under `.github/`, `web/` or `crates/core/tests/`.
The local `core` run above reproduces Gate 1e's panic on this tree byte for
byte, so the failure is in the tree rather than in CI. The stand-down comment
on the PR says so.

**Net of the gates.** Two gates are clean: format and lint. Four crates are
green as uid 0: runner, pull, api and engine. Across the seven crates run,
7,529 tests passed, 6 failed and 33 were ignored. By crate, passed/failed/
ignored: runner 792/0/0, store 652/1/4, pull 1634/0/23, api 1984/0/5, cli
2096/1/1, engine 131/0/0, core 240/4/0. None of the six failures is in code
this branch changed:

- 1 needs history the snapshot does not have (store `cited_commits`).
- 1 is a mode-bit test that root cannot fail (cli `scrub_blocks`).
- 4 compare against the workflow file the snapshot took from `main` (core
  `banned_lists` ×2, `claim_reader` ×2).

## Open findings, most dangerous first

Each is in its lens table above with its exact fix; this is the order to take
them in.

| # | where | what | severity |
|---|---|---|---|
| 1 | `crates/runner/src/validate.rs:5140`, `grid.rs:3806` | walk-forward scores an out-of-sample replay that refused every path as 0, the same as a flat candidate, and PBO ranks on it | high |
| 2 | `docs/05-decisions.md` (D-3349 to D-3353) | five decisions cited 317 times in code have no entry | high |
| 3 | `docs/04-invariants.md:6921` (GI-07) and five more rows | ✓ rows assert behaviour the code reversed (refused-by-the-store, the-run-continues, re-reads-every-manifest) | high |
| 4 | `crates/runner/src/grid.rs:2146-2326` | in-sample exit levels fitted to the same trades' later outcomes | medium |
| 5 | F-1 (lens 4, ~18 modules incl. `selection_v6.rs:362, 369`) | a failed barrier's bytes vouched for by a later barrier, against D-1900 | medium |
| 6 | `crates/runner/src/grid.rs:1844, 1851, 5026` | an unpriceable exit booked as a flat trade, uncounted | medium |
| 7 | `crates/cli/src/lib.rs:6867` (LOCKED) | a truncated session index feeds the bootstrap p-values | medium |
| 8 | `crates/cli/src/expression_search.rs:571-584`, `search_checkpoint.rs:14` | an undocumented ~1M-candidate wall under text that says there is no size bound | medium |
| 9 | `crates/pull/src/gdfl_import.rs:7075-7117`, `repair_plan.rs:167-249` | warm-up refusals unlogged; read faults planned as missing days | medium |
| 10 | `crates/api/src/server.rs:2596-2599`, `:11971`, `pullrun.rs:2018-2048`, `sources.rs:433-566` | a corrupt journal shown as "no pull", greeks skipped with no reason, an unreadable census counted as 0, a memo key that cannot tell unreadable from absent | medium |
| 11 | 15 `✓` invariant rows (lens 7) | name tests that exist nowhere; gate 10 is red on this tree | medium |
| 12 | `crates/cli/src/results.rs` siblings (lens 4 F-3) | no directory fsync on creating `trades`, `frontier`, `result_set`, selection and replay ledgers | medium |
| 13 | `crates/cli/src/scrub_blocks_tests.rs:610-611` | a chmod test with no root guard, red as root only (lens 3) | low |

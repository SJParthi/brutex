# Pass 4: tests, docs and security (final/all-fixes-zero @ 1f4de71)

**Verdict at 1f4de71:** 76 open items re-checked. 54 are FIXED, 22 are NOT FIXED, and none is a wrong fix or fixed only on an unmerged branch. This pass found 2 new defects (1 medium, 1 low). Both are invariant rows the fix merges left behind, and the first one turns CI gate 10 red.

**Scope.**
- The work was first done at 6104a4b in /home/claude/wt/zero. It was then re-checked at 1f4de71 in /home/claude/wt/zero2, which merges zero/api-routes, zero/cli-edges-2, zero/numeric and #74 c97ff00. Every state below is the state at 1f4de71.
- The analysis is static. The one exception is a shell reimplementation of gate 10's row loop, which I ran over docs/04 at both heads.
- **Tracker gap:** P1-03-1 and P1-03-2 have no row in fix-board/status/zero-findings.tsv.md, and neither is fixed.

## Verification table

For each test given teeth, the evidence column names the mutation the test now catches.

| ID | State | Evidence |
|---|---|---|
| P1-03-1 | NOT FIXED | server.rs:17463 and :17476: `poll_write` still calls `this.rearm()` on every write. |
| P1-03-2 | NOT FIXED | server.rs:17100: `authority.port_u16() != Some(local_addr.port())` is unchanged. |
| P1-04-01 | NOT FIXED | `bars_window_json` (server.rs:3406) still calls `bars::window(` inline, with no `spawn_blocking` or admission. |
| P1-04-02 | NOT FIXED | logs.rs has zero `spawn_blocking`/`run_store_read`. Its diff changed only the failed-line rations. |
| P1-04-03 | NOT FIXED | `universe_resolve` (server.rs:33463) has no lock, semaphore or busy refusal. |
| P1-04-04 | NOT FIXED | recovery.rs:613-631: `preflight_submission` still runs on the blocking pool before `claim`, and `prepare_only` returns before any slot. |
| P1-06-01 | NOT FIXED | `grep journal_error web/src` finds nothing. The only web/src changes are instrument.js and the backtest, db and mapping pages. |
| P1-06-02 | NOT FIXED | live-progress.ts and logs `PAGE_LIMIT` are unchanged, and the start marker is still required in every window. |
| P1-06-03 | NOT FIXED | The audit page and audit_json offset paging are unchanged. |
| P1-07-01 | NOT FIXED | ci.yml:1843 and :2472 still use `grep -E ':[[:space:]]*(build|links)[[:space:]]*='`. |
| P1-07-02 | NOT FIXED | Gate 1c (ci.yml:597) still greps raw bytes. Gate 1d (ci.yml:1700-1740) still never splits a literal on `/`. |
| P1-07-03 | NOT FIXED | Gate 1 still uses `grep -Eq ... <<< "$base"` / `<<< "$ext"` (line-wise). There is no control-character refusal. |
| P1-07-04 | NOT FIXED | ci.yml:405 and :441 still use `listing=$(git ls-files)`, without `-z`. |
| P1-08-01 | NOT FIXED | No gate or scanner mentions `mutants::skip`. |
| P1-08-02 | NOT FIXED | No `bench = false` or `required-features` refusal, and gate 8 still counts files only. |
| P1-08-03 | NOT FIXED | `PINNED='vocab indicators engine'` (ci.yml:4019). The gate 17 rule and gate 22 `store_rule` still match only key spellings. The "WHAT IS STILL NOT PINNED" claim at :4012 is unchanged. |
| P1-08-04 | NOT FIXED | Gate 23 is still `git ls-files -z -- 'crates/*/src/*.rs'` (ci.yml:3203). `crates/cli/commit_stamp.rs` is still outside it. |
| P1-08-05 | NOT FIXED | ci.yml:2046: gate 7 still uses the flat `[dependencies]` awk. |
| P1-09-01 | NOT FIXED | auto-merge.yml:39, :222 and :227 still credit "a fork gets a read-only token". |
| P1-10-01 | FIXED | binary.rs:147-182 runs `auto 6` and reads `threshold chosen`, `ladders walked`, `combinations found` and `swept` as numbers, and refuses `NOTHING MEASURED`/`REFUSED`. Mutation caught: an empty `Sweeper::auto` renders `threshold chosen NONE`, so `row_number` returns None and the `.expect` fails. |
| P1-10-02 | FIXED | audited_range_tests.rs and checksum_receipts_tests.rs build the child name from `module_path!()` and require `test result: ok. 1 passed;`. Mutation caught: renaming the test makes the child run 0 tests, so the assert fails. |
| P1-10-03 | FIXED | equity_statement_tests.rs bounds both bodies at `\n}\n`, drops comment lines, and reads `load_screen_inputs` (lib.rs:15907, the screen's loader, called at :16051). Mutation caught: reordering the loaders inside `load_screen_inputs`. A later helper can no longer satisfy it. |
| P1-10-04 | FIXED | candidate_universe.rs: `assert_eq!(first.admission_values(), &sealed)`, plus a non-default `support_hits > 0`. Mutation caught: the projection decoding wrong fields or defaults. |
| P1-11-01 | FIXED | lib.rs:25853-25887 reads the `dispatch` body (bounded, comments dropped) and requires the set of first words to equal `COMMANDS`. Mutation caught: a deleted `research-plan` arm. I re-checked at 1f4de71 that the 35 words still match, `expression-stored` included (multi-line arm). |
| P1-11-02 | FIXED | All four children (readonly_file, research_policy, research_policy_tests, results_report_tests) use the `module_path!()` name and require `1 passed`. readonly_file now pipes stdout. |
| P1-11-03 | FIXED | index_stop_source_context_tests.rs restores `body.bin`, proves the archive opens, then removes `complete.bin` and requires `original_context_refused` with `os error 2`. Mutation caught: a reader that ignores the missing receipt now opens, so `.err().ok_or` fails. |
| P1-12-01 | FIXED | server.rs calls `instrument_word(&rolling, true/false)` directly. The call-site needle is split with `concat!` and counted `== 1`. Mutation caught: `instrument_word` always returning `index_word`. |
| P1-12-02 | FIXED | Both tests take `emitted::mark()` and require exactly one `api.server`/`cannot bind the listening address` Error event with `addr` equal to the held address and a non-empty `why`. |
| P1-12-03 | FIXED | strict_sweep_tests routes through `crate::isolated::rerun` (which requires `1 passed`). indexstoplaunch_tests requires `1 passed` in the child log. Both build the name from `module_path!()`. |
| P1-12-04 | FIXED | serve_edge_tests.rs `without_test_items` drops every `#[cfg(test)]` item, and asserts that `vocab_json`/`calendar_json` are scanned and no `mod tests {` remains. |
| P1-12-05 | FIXED | The fixture writes `dir/Cargo.toml` = `[package] TRAVERSAL-LEAK-SENTINEL`. The test requires the status line `HTTP/1.1 400` and no sentinel. Mutation caught: a handler that resolves `..` serves the sentinel or a non-400. |
| P1-13-01 | FIXED | The evaluator, trend and column idempotence tests now require `len == bars` and a non-constant run (`later != earlier`). Mutation caught: a constant `ZERO` body, or one that refuses every bar. |
| P1-13-02 | FIXED | runner lib.rs: the refused bar is inserted at 6*375+100 (after the 1,876-bar warm-up, asserted). Whole `Column::bits()` and `all_frequent()` are compared with the clean run. |
| P1-13-03 | FIXED | engine column.rs:480-515: anchor through to `\n    }\n`, `//` stripped, exactly one `row.hits(candidate)` and one `.fold(`. `for /while /loop /.all(/.any(/.filter(/.count(/.get(/set_positions/popcount` are banned. |
| P1-14-01 | FIXED | ssm.rs pins `signing_key_for(.., "iam")` to AWS's published `c4afb1cc…a4b9`, the ssm chain to `1b014a52…3358`, and one full `Authorization` header. |
| P1-14-02 | FIXED | graph.rs `documented_row_names_from` reads names before the member filter. `row_name_faults` names invented and duplicate rows, and has its own test. |
| P1-14-03 | FIXED | graph.rs `workspace_members` panics on a non-`crates/` member or a glob and strips `#` comments. It has its own test. |
| P1-14-04 | FIXED | findings.rs `history_is_at` skips only when git cannot run or `.git` is absent, and fails otherwise. A test covers the bare, broken `gitdir:` and dangling-symlink cases. |
| P1-14-05 | FIXED | The tautological `writes` counter is removed. The test now requires each of `ParameterStore`/`SecretSource` to declare exactly one read method (read from secret.rs). The secret.rs doc and the P-05 row are corrected. |
| P1-14-06 | FIXED | costs adversarial.rs compares every cell with the exact i128 quotient (Ok iff it fits i64), requires `checked >= 49` and `checked+refused == cells`, and the monotonic ladder counts its comparisons. |
| P1-14-07 | FIXED | lake refusals.rs: `boundary_cases == cuts.len()`. |
| P1-16-04 | FIXED | docs/02 §27-§34 cover all eight formats (runs.bin, population, live, Pre-Admission V2, Execution V3/V4, Global Replay V3, Statistics V3). Each is bound by a ZD-01..08 test that reads the doc against the constants (e.g. results.rs:1579). |
| P1-17-01 | FIXED | The docs/04 paragraph now says the guard was deleted (D-0136) and cites the real `the_broker_path_addresses_a_set_and_no_target_guard_stands_in_its_way`. |
| P1-17-02 | FIXED | `note_unstamped_lock` is extracted. The test drives it and reads back one WARN plus the stderr line, and pins `stamp_serve_lock` calling it (split needle). |
| P1-17-03 | FIXED | binary.rs `a_closed_stdout_is_said_on_stderr_and_never_panics` reads the run's log dir and requires exactly one `cli.output` WARN with `"exit":0`. |
| P1-17-04 | FIXED | anchored_search_lineage_v4.rs source-shape check: `append_raw` delegates to `append_with_rollback(file, raw, Write::write_all)` with no `file.write_all(`, and `append_locked` has one `append_raw(` carrying `encode_members(&members)?, 2`. |
| P1-18-01 | FIXED | docs/09-verify.md §2 excludes `crates/cli/build.rs` by name. §4/§5 name existing pages with `feeds\.active *=[^=]`. Re-ran every command: 0/0/0/0, 0, 0/0, terminal 47, ingest 3, 1812 and 25, all as stated. |
| P1-18-02 | FIXED | docs/10 §1 and §3 both say twelve sources and 328. `evaluator_position_count::this_documents_position_counts_are_the_live_table` binds both. |
| P1-18-03 | FIXED | docs/10 says plainly that the tag snippet is not yet possible and gives a `rev =` form (D-1943). |
| P1-18-04 | FIXED | The only hash left in docs/07-plan.md is `ffa41c6d`, an ancestor of origin/main. cited_commits.rs `WHOLE` now includes docs/07-plan.md. |
| P1-18-05 | FIXED | §5 is marked "All three are CLOSED", and the §7.1 row is "Partly CLOSED". |
| P1-18-06 | FIXED | §6 now states D-1436's withdrawal and points to the docs/04 rows instead of copied figures. R-9 says thirteen crates. |
| P1-18-07 | FIXED | docs/07-o1 layer 4 states hit <= 8 (worst 7) and miss <= 12 (worst 11/10), citing the miss test. |
| P1-19-01 | NOT FIXED | http.rs:495-507 `header_value` still returns the token unchecked. No `HeaderValue::from_str` is present. ssm.rs:928 still checks only `is_empty()`. |
| P1-19-02 | FIXED | chain.rs:343 calls `iso_expiry` before `contracts_url`/`get` (:350-351). fno.rs `join` percent-encodes every value (`push_encoded`, :232). Covered by ZR-49. |
| P1-19-03 | NOT FIXED | ssm.rs:397 still uses a bare `std::fs::read_to_string(path)`. The SSM answer is still read whole with `.text()` (:881). |
| P1-20-01 | NOT FIXED | ledger_all.rs:875 `LedgerTree::create` still runs before `build_sweepers` (:877). `ledger_all_arm` has no 1..=12 or order check. |
| P2-01-01 | FIXED | frontier.rs:2078: the doc-binding test sits above the stride test's own doc block (:2099-2116). |
| P2-01-02 | FIXED | `bars_feed_word`/`bars_vendor` (server.rs:15631/15649) are above the `locate_series` doc, which again documents `locate_series` (:15722). |
| P2-01-03 | FIXED | D-1766 corrects the route to `/backtest/audit.json`. ZR-26 and ZR-27 cite existing tests. |
| P2-01-04 | FIXED | bars.rs doc, docs/06-limits.md:3275 and the test needle all say "three call sites — `page`, `window`'s seek branch, and `read_in_time`". The count of 4 = 3 calls + 1 definition. |
| P2-02-01 | FIXED | The five routes are in vite.config.js ROUTES. proxy.test.js also collects every absolute `.json` literal, whatever its callee. |
| P2-02-02 | FIXED | prefix.test.js test 1 probes `A` (bucket grows ~100x) and checks by-reference identity. Test 2 holds the `ABCD` bucket fixed and asserts a ratio < 3. Both the copy mutant and the full-scan mutant now exceed 3. |
| P2-02-03 | FIXED | Docs 14/15/31 have 0 `](../target/` links, and each carries the session-artifact caveat. |
| P3-01-01 | FIXED | `form_read_bound` maps `/pull/run`/`/pull/recovery` to `MAX_RUN_FORM_BYTES` (server.rs:16373). The route layers match (:16586, :16593). The body is read inside `same_origin_writes_only`, so a cross-site page cannot make the server read 26.5 MB. |
| P3-01-02 | FIXED | Merged in 332cfc3. sweeprun.rs:851 `refuse_unread` is applied to `/backtest/run` (`rung` refused), `/backtest/descend` (knobs refused) and the ordinary command words. No web caller sends a now-refused field (checked: the backtest run/descend bodies and the receipt-batch plan, which uses `audit-audited-range`). |
| P3-01-03 | FIXED | operation_audit.rs: non-GET requests go through `request_audited_detached`, which runs `tokio::spawn`s the audited handler to its real terminal. |
| P3-01-04 | FIXED | mastersrun.rs: `refresh` = `detached(refresh_work(site))`. A test drops the waiter and sees the work finish. |
| P3-01-05 | FIXED | MR-20 (docs/04:3048) describes `Site::reparse` and cites the renamed test. The same stale name survives in AFD-15; see P4-01. |
| P3-02-01 | FIXED | The Rust page is `/audit/page` (server.rs:16651, render.rs nav/pager/links). `/audit` falls through to the Svelte build. The vite.config.js text is corrected. |
| P3-02-02 | FIXED | backtest/+page.svelte uses `sweptSymbolOf` (instrument.js: everything after exchange and segment, spot only). ZR-55 covers it. |
| P3-02-03 | FIXED | lib.rs:387: USAGE shows `[TIMEFRAMES]` and "or all eight". |
| P3-02-04 | FIXED | No `%%` in USAGE. The test asserts `!USAGE.contains("%%")` (lib.rs:22439). |
| P3-02-05 | FIXED | lib.rs:598: "UNDERLYING  the index, e.g. NIFTY or BANKNIFTY, or one of the F&O" shares. |
| P3-02-06 | FIXED | sweeprun.rs:3125-3143: `sweep-all` reads only feed, rung and min_hits. `COMMAND_SWEEP_ALL` refuses `underlying` and span fields by name (test at :6307). |
| P3-02-07 | FIXED | .claude/launch.json no longer sets `BRUTEX_COMMIT`. Its name says build.rs stamps a clean tree and that a dirty tree refuses. |

## New findings

### P4-01 medium CI gate 10 is red at 1f4de71: three `✓` invariant rows cite tests that no longer exist (two left behind by zero/ledger-tails, one by the #74 merge)

**Where:**
- docs/04-invariants.md:3325 (RS-03)
- docs/04-invariants.md:3683 (PS-02)
- docs/04-invariants.md AFD-15 (~:6285), also docs/05-decisions.md D-1585 (~:54098)

**Evidence:**
- RS-03 cites `cli::tests::a_torn_prepared_tail_blocks_every_later_commit`. Commit 4cee129 (D-1901) renamed it to `a_torn_prepared_tail_is_cut_and_the_exact_preparation_resumes`.
- PS-02 cites `cli::population_statistics_v2::tests::exact_trailing_prefix_retry_completes_and_foreign_retry_refuses`. Commit 4cee129 (D-1905) split it into `exact_trailing_prefix_retry_completes` and `a_foreign_writer_discards_a_receipt_less_orphan_and_records`.
- AFD-15 cites `api::mastersrun::tests::the_page_says_a_restart_is_required_rather_than_pretending_otherwise`. 3f22aed renamed that test to `the_page_says_a_refresh_reloads_and_names_when_a_restart_is_required`. #74's D-1585 row was written against the old name, and bde50c0/e49875c merged it in. This is the same rename P3-01-05 fixed in MR-20, which missed this second citation.
- Gate 10 (ci.yml, "Gate 10 — every invariant names a test that exists") does `grep -qxF "${c} ${fn}" "$fns"` and sets `bad=1` on any miss.

**Why it is wrong:**
- `cargo test` cannot see a stale citation. Gate 10 is the only check, and it fails on these three rows, so CI on #74 goes red as soon as this staging head is pushed there.
- CLAUDE.md §9 requires every invariant to sit beside the test that proves it.

**Repro (ran):** a reimplementation of gate 10's row loop over docs/04 (tokens `a::b::c`, crate taken from the first segment, `fn` names collected per `crates/<c>`). It prints exactly:
```
MISSING RS-03 cli::tests::a_torn_prepared_tail_blocks_every_later_commit
MISSING PS-02 cli::population_statistics_v2::tests::exact_trailing_prefix_retry_completes_and_foreign_retry_refuses
MISSING AFD-15 api::mastersrun::tests::the_page_says_a_restart_is_required_rather_than_pretending_otherwise
```
The output is the same at 6104a4b and at 1f4de71. `git grep -n "fn <name>"` finds none of the three anywhere in crates/.

**Minimal fix:** cite the renamed tests in the three rows, and append one D-entry recording the stale citations.

### P4-02 low RS-03 and PS-02 still state the refusal behaviour that D-1901 and D-1905 replaced, so docs/04 contradicts itself (ZL-09/ZL-10/ZL-11 say the opposite)

**Where:** docs/04-invariants.md:3325 (RS-03) and :3683 (PS-02), against ZL-04, ZL-09 and ZL-10 (:6460-6466).

**Evidence:**
- RS-03 says: "a torn child/receipt tail ... all refuse before the ledger. Neither state is overwritten, padded, truncated". Since D-1901 the writer cuts the torn byte and the exact preparation resumes (lib.rs test `a_torn_prepared_tail_is_cut_and_the_exact_preparation_resumes`: "exactly the torn byte was cut").
- PS-02 says: "foreign retry ... refuse". Since D-1905, `a_foreign_writer_discards_a_receipt_less_orphan_and_records` expects `PopulationStatisticsV2Append::Written`, and ZL-09 states "discard a foreign receipt-less orphan".

**Why it is wrong:** CLAUDE.md §10 makes docs/04 the authority for "what must hold". Two `✓` rows assert the opposite of current code and of later rows in the same file. Re-pointing the citation (P4-01) without rewording the claim would put a passing test beside a false sentence.

**Repro (not run; static):** read RS-03 and PS-02, then read the bodies of the two replacement tests.

**Minimal fix:** reword RS-03 to "a torn sub-record tail is cut under the writer's lock and the exact preparation resumes; a mismatched block or corrupt seal refuses". Reword PS-02 to say a foreign receipt-less orphan is discarded and the writer records (D-1905). Cite the new tests.

## Checked in the diffs (5140aca..1f4de71) and found holding

- Every other `a::b::c` citation in docs/04 resolves. So does every backticked long identifier added to docs/02, 05, 06, 07, 09, 10 and 11. The remaining misses are history prose that says the name was removed: testgaps phantoms, af_16's old name, `only_the_swept_target...`.
- New source-shape tests (autopilot `tick`, `land_spot`, `pull_spot`/`pull_fno`, `stamp_serve_lock`, index_stop_vix `publish`, ordered lanes, `ensure_header` sync, observation root sync, the docs/02 bindings) are bounded, and their first anchor occurrence is production code or a split `concat!`. None matches its own needle on the passing path.
- New env/knob reads go through `brutex_core::knob` (folder, home, switch). `BRUTEX_VALIDATE` and `BRUTEX_ARCHIVE_SUGGESTIONS` name a refused word and keep the safe side.
- The archive-suggestion refusal text is passed through `escape` before it reaches HTML.
- Layer order puts `same_origin_writes_only` outside `one_value_per_form_field`. The origin check therefore runs before the body is read, which is why the 26.5 MB `/pull/run` body bound is the only one I found that is reachable cross-site, and it is not.
- The new `request_audited_detached` and the masters `detached` spawn hold no lock across the await.
- No new test asserts a tautology: I scanned for `assert!(true`, `x == x`, `>= 0`, and `is_ok() ||`.

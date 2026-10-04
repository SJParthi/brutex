# Pass 5: tests, docs and security (final/all-fixes-zero @ 1f4de71)

**Verdict at 1f4de71:** CI gate 10 is still red. This pass found 7 new defects (1 medium, 6 low). Gate 10 now fails in two places: in `invariant_paths.rs`, which pass 4's replica never ran (P5-01), and in the qualified-token loop (P4-01, still open). Three invariant rows, one docs/06 statement and one form guard say something the code no longer does.

**Scope.**
- Read-only, in /home/claude/wt/zero3 at 1f4de71. Diff base 331b05c.
- No cargo was run. I compiled two small files with rustc into the scratchpad:
  - `.github/source_scan.rs` and `.github/invariant_paths.rs`, to replay gate 10 exactly;
  - a standalone copy of `repeated_form_key` with a counting allocator, for P5-05.
- P1-*, P2-*, P3-*, P4-01 and P4-02 are not re-reported.

## Gate-10 replica (ran)

Every step of ci.yml "Gate 10 — every invariant names a test that exists" ran in order:
1. `source_scan fns` over `git ls-files -z 'crates/*.rs' '.github/*.rs'` (rc 0).
2. `source_scan modules` (rc 0, 0 UNRESOLVED).
3. `invariant_paths docs/04-invariants.md path-fns modules`.
4. The qualified-token row loop, with the P-03/X-13 allowlist.

```
invariant path reference refused:
line 3442: proof `a_stale_handle_refuses_to_extend_a_ragged_chosen_trade_tail` is declared in no tracked source file
line 6424: brutex_core::knob::tests::an_empty_or_blank_folder_is_refused_by_name_and_unset_is_none names no crate, and no tracked file of a module `brutex_core` declares its function
ip_rc=1          <- with `set -euo pipefail` the step exits HERE
--- row loop (run anyway, to show what follows) ---
INVARIANT POINTS AT A TEST THAT DOES NOT EXIST: cli::tests::a_torn_prepared_tail_blocks_every_later_commit (RS-03)
INVARIANT POINTS AT A TEST THAT DOES NOT EXIST: cli::population_statistics_v2::tests::exact_trailing_prefix_retry_completes_and_foreign_retry_refuses (PS-02)
INVARIANT POINTS AT A TEST THAT DOES NOT EXIST: api::mastersrun::tests::the_page_says_a_restart_is_required_rather_than_pretending_otherwise (AFD-15)
rows=3152 checked=2477 missing=3
```
The last three misses are P4-01, still NOT FIXED. The first two are new (P5-01).

## New findings

### P5-01 medium Gate 10 also fails in `invariant_paths.rs` on RS-08 and ZR-44, a failure the pass-4 replica could not see

**Where:**
- docs/04-invariants.md:3442 (RS-08)
- docs/04-invariants.md:6424 (ZR-44)

**Evidence:**
- **RS-08** cites the bare name `a_stale_handle_refuses_to_extend_a_ragged_chosen_trade_tail`. Commit 4cee129 (D-1901) renamed it in crates/cli/src/trades.rs to `a_stale_handle_cuts_a_ragged_chosen_trade_tail_before_appending`. Its diff shows `-fn a_stale_handle_refuses_to_extend_a_ragged_chosen_trade_tail` and `+fn a_stale_handle_cuts_a_ragged_chosen_trade_tail_before_appending`.
- **ZR-44** cites `brutex_core::knob::tests::an_empty_or_blank_folder_is_refused_by_name_and_unset_is_none`.
  - The function exists at crates/core/src/knob.rs:92.
  - But `brutex_core` is the lib name, not a crate directory. The row loop skips the token as "module-first", and `invariant_paths` then refuses it, because no tracked file mounts a module `brutex_core`.

**Why it is wrong:**
- Gate 10 runs `"$work/invariant-paths" ...` under `set -euo pipefail`, before the row loop. The step therefore exits 1 on these two rows even after P4-01's three rows are fixed.
- Pass 4 replayed only the row loop, so fixing P4-01 alone still leaves gate 10, and #74's CI, red.

**Repro (ran):** see the gate-10 replica above (ip_rc=1).

**Minimal fix:**
- RS-08: cite `a_stale_handle_cuts_a_ragged_chosen_trade_tail_before_appending`.
- ZR-44: write `core::knob::tests::an_empty_or_blank_folder_is_refused_by_name_and_unset_is_none`.
- Add one D-entry for both.
- Whoever fixes P4-01 should re-run the whole gate-10 step (both binaries plus the loop), not only the loop.

### P5-02 low RS-08's text still says writers refuse a ragged tail; since D-1901 they cut it (ZL-04 says the opposite)

**Where:** docs/04-invariants.md:3442 (RS-08) against :6464 (ZL-04).

**Evidence:**
- RS-08 says: "stale handles and fresh reopens refuse a ragged tail".
- At 1f4de71, every writer's open (fresh reopen) cuts a sub-record tail under its lock:
  - `Frontier::open` → `index_locked` → `fixed_tail::heal_torn_tail` (frontier.rs:945);
  - `trades.rs:523` and `:659`;
  - `Receipts::open` (result_set.rs:329), with the comment "THE WRITER'S DOOR CUTS A TORN TAIL".
- Frontier and trades stale handles also cut it before appending (frontier.rs:1103, `append_locked_with`; trades `a_stale_handle_cuts_a_ragged_chosen_trade_tail_before_appending`).
- Only a stale *receipt* handle still refuses: result_set.rs:1326 `a_stale_handle_refuses_a_new_ragged_tail_before_appending`.
- The frontier's own refusal text now reads "the next writer cuts it under its lock" (frontier.rs check_header).

**Why it is wrong:** This is the P4-02 class. A `✓` row asserts behaviour that D-1901 replaced, and it contradicts ZL-04 in the same file. Re-pointing the citation (P5-01) without rewording would put a passing test beside a false sentence.

**Repro:** not run (static). Read RS-08, then frontier.rs:930-990 and trades.rs:515-530.

**Minimal fix:** reword RS-08 to: "a writer's open, and a frontier or chosen-trade stale handle, cut a sub-record tail under the exclusive lock (D-1901); a stale receipt handle refuses one; a bad sealed row, schema-invalid row and second non-contiguous block refuse".

### P5-03 low A docs/04 row and its test still say "no power-of-two table is built"; D-1572 built one, and the test passes only because it greps old spellings

**Where:**
- docs/04-invariants.md:6182 (the unnamed `forward` row, D-1170)
- crates/runner/src/outcome.rs:4545 `forward_builds_no_power_of_two_table`
- outcome.rs:771-803 `BlockExtremes`

**Evidence:**
- The row says the extremes are "read from two monotonic deques whose ends only advance. A backwards or jumping query is answered by refilling ... No power-of-two table is built".
- Since b241799 (D-1572), a backward query goes to `over_blocks` (outcome.rs:683). `over_blocks` reads `BlockExtremes`, whose doc says `levels[k][b]: the extremes of blocks b ..= b + 2^k - 1`. It is built by a doubling loop (`let doubled = span.saturating_mul(2); if doubled > below.len() { break; }`). The code comment itself says "every power-of-two run of blocks".
- The test asserts only `!live.contains("RangeExtremes")` and `!live.contains("width.saturating_mul(2) <= n")`. Those are D-1170's old identifier and loop condition. A doubling table under another name and loop condition passes.
- A second, smaller drift: the `BlockExtremes::over` doc comment cites `runner::window_tests::a_backward_right_end_is_answered_in_constant_reads`. The module is `runner::outcome::window_tests`.

**Why it is wrong:**
- The `✓` row contradicts D-1572 and docs/06-limits.md:14234 ("`BlockExtremes` ... `(n/64)·log₂(n/64)` pairs").
- The test's name states a property that is false at HEAD and still passes. That is the "test that passes for the wrong reason" shape.

**Repro:** not run (static). Read outcome.rs:671-684 and 771-803, then the test at 4545-4558.

**Minimal fix:**
- Reword the row: "forward queries slide the deques; a backward query is answered from a block-level power-of-two table built once per `forward` (D-1572); no per-bar table is built".
- Rename the test to `forward_builds_no_per_bar_sparse_table`, and make it assert that `BlockExtremes::of` folds `chunks(EXTREME_BLOCK)` first.
- Fix the doc-comment path.

### P5-04 low AFF-43 states a per-query bound of `3n + 130`; the test proves a total of `3n + 130·queries`

**Where:** docs/04-invariants.md:6513 (AFF-43) against outcome.rs:4507.

**Evidence:**
- The row: "reads at most `3n + 130` bars per query over a slice of `n` bars".
- The test asserts `window.touched <= 3 * n + 130 * queries` for 17,999 queries over n = 20,000. That is a cumulative bound: 3n once, plus an amortised 130 per query.
- docs/06-limits.md:14241 states it correctly ("held to `3n + 130·queries`").

**Why it is wrong:**
- The per-query sentence names a bound nobody measured. Golden rule 6 says never claim a measurement you did not take.
- The total bound does not imply it. The first backward query alone pays the n-bar table build.

**Repro:** not run (static).

**Minimal fix:** "reads at most `3n + 130·q` bars across `q` queries (amortised 130 per query after an O(n) one-time build)".

### P5-05 low The D-1587 repeated-form-field guard sizes its set from the count of `&` bytes: a 27.3 MB `/pull/run` body of `&` makes it allocate 570 MB

**Where:** crates/api/src/server.rs:16341-16353 (`repeated_form_key`). The body is read within `form_read_bound` (`/pull/run` and `/pull/recovery` = `pullrun::MAX_RUN_FORM_BYTES` = 8,192 + 31 × 881,924 = 27,347,836 bytes).

```rust
let mut seen = std::collections::HashSet::with_capacity(
    body.bytes().filter(|b| *b == b'&').count().saturating_add(1),
);
```

**Why it is wrong:**
- The capacity is attacker-sized: one hash bucket per `&` byte, reserved before a single key is read. A legitimate form has a handful of distinct non-list keys.
- Measured amplification is 20 to 34 times the body. With `MAX_CONNECTIONS` = 256, concurrent bodies multiply it.
- An allocation failure aborts the process, which makes this a crash route under the release profile.
- Only same-origin or header-less (local) clients get here, because `same_origin_writes_only` answers first. That keeps it low.
- docs/06-limits.md:14148 also still says this read is "bounded by `MAX_FORM_BYTES` (8 KiB)". Since D-1769/D-1770 it is bounded by `form_read_bound`: up to 27.3 MB, and 168 KB on `/ingest/queue` and `/pull/spot`. The section 40 lines further down says 26.5 MB, so the document contradicts itself.

**Repro (ran):** a verbatim copy of `repeated_form_key` under a counting global allocator, compiled with `rustc -O`:
```
body 8192 bytes -> extra peak 278544 bytes (34x)
body 168192 bytes -> extra peak 4456464 bytes (26x)
body 27347836 bytes -> extra peak 570425360 bytes (20x)
```

**Minimal fix:**
- Size the set as `with_capacity(64)` and refuse a body naming more than, say, 64 distinct non-list keys, as "too many fields". Growth past that never happens.
- Correct docs/06:14148 to `form_read_bound`.

### P5-06 low The repeated-form-field guard is skipped by a client-chosen `Content-Type` or a leading `{`, while every form handler still reads the first value

**Where:**
- server.rs:16417-16422 (`json` sniff in `one_value_per_form_field`)
- the readers: `server::param` (server.rs:800, "first match wins"), `autopilot::control` (autopilot.rs:3908 `param(&body, "action")`), pullrun.rs:534, server.rs:11276/15103 and ingest.rs:1503/1752 (`param(body, "vendor")`). Every one takes `body: String` and checks no content type.

```rust
let json = parts.headers.get(CONTENT_TYPE)...is_some_and(|value| value.to_ascii_lowercase().contains("json"))
    || matches!(bytes.trim_ascii_start().first(), Some(b'{' | b'['));
```

**Why it is wrong:**
- D-1587 says `action=stop&action=start` and `vendor=dhan&vendor=groww` are now refused. They are refused only when the client does not label them.
- Send either body with `Content-Type: text/plain; x=json`, or prefix it with `{=&`. The middleware passes it through as "JSON", and the handler parses it as a form with first-match-wins. The duplicate is silently ignored again, which is what the guard exists to refuse. §4 bans a fallback that hides a failure.

**Repro:** not run (static; the path is three lines).

**Minimal fix:** decide JSON-ness by route, not by request. Skip the check only for the routes that decode JSON (`/backtest/*`, `/engine/command`), or only when the body actually parses as JSON.

### P5-07 low Two new tests assert wall-clock thresholds on a shared runtime with no stated slack policy

**Where:** server.rs:28199 `blocking_landing_work_leaves_the_runtime_answering` and server.rs:28236 `a_dropped_pull_route_does_not_cancel_the_pull`. Both came from ad40a87b, which is not an ancestor of 331b05c.

**Evidence:**
- The first test requires a spawned task to be answered within 300 ms while another task does 400 ms of blocking work. The fix (`block_in_place`) passes with about 280 ms of slack, and a revert fails by about 100 ms. Both margins are scheduler latency on a loaded host: this box runs 4 cores shared with 3 auditors.
- The second test requires `timeout(20 ms)` to fire before a 200 ms sleep completes. A runtime starved for more than 180 ms polls the inner future first, and `cut.is_err()` fails.
- Neither test has a teeth problem: reverting either fix makes it fail. The problem is that they can also fail with the fix in place.

**Why it is wrong:** CLAUDE.md §3 rule 5 (same inputs, same outputs). The existing T/T_SLACK tests (server.rs:17583) state their slack. These state none.

**Repro:** not run.

**Minimal fix:**
- First test: replace the timing with a handshake. The blocking closure waits on a channel that the second task signals, which deadlocks if the blocking code runs inline. Use a `recv_timeout` of several seconds as the failure.
- Second test: use `tokio::time::pause()` and `advance`.

## Checked and found holding

- **D-1981 / ZE-02.** `api::detail` has `const _: () = assert!(cli::frontier::MAX_ROWS as u64 == MAX_RESULT_ROWS)`. `admit_top` is called at lib.rs:6226, 7103, 14464, 14835, 16351 and 17507, and in the strict `BRUTEX_TOP` knob (strict_range_knobs.rs:64). The tests hit both descents and `record_frontier`, and check that no directory is written.
- **D-1991 / ZN-05.** livejson.rs:244 requires `n >= MIN_JUDGEABLE_OBSERVATIONS` (= `runner::report::MIN_OBSERVATIONS` = 30). The test covers n = 5/29/30.
- **D-1990 / ZN-04.** The test calls every entry point D-1990 lists with `periods + 1`. A revert makes each return `Some` or non-empty, so it fails. The `block == periods` edge is not pinned; that is a mutation gap, not reported.
- **D-1761.** `basis_points` uses checked sub and mul; `-rest` cannot overflow, because |rest| < first.
- **D-1980.** `footer::check` runs at reader.rs:141, before `ParquetMetaDataReader` at :143. `MAX_DEPTH` = 32, and `MAX_SCHEMA_ELEMENTS` = F&O columns + 1, as stated.
- **D-1972 to D-1974 / ZW-03 to ZW-06.** The tests drive real behaviour. The ZW-06 source anchor `pub async fn refresh(` is production code, first occurrence.
- **Theme 2 (`#[ignore]`, root).**
  - No `#[ignore]` was added since 331b05c.
  - The one new permission test (durability.rs:447) goes through `support::where_permission_binds` (D-0995), so it is honest as root.
- **Theme 3 (new surface).**
  - `/audit/page` is the only new route.
  - `Cache-Control: immutable` is applied only to `_app/immutable/*`, from a static string.
  - `cross_site` reads `Sec-Fetch-Site` only to pick a log ration.
  - The new env reads all go through `knob::{folder, home, switch}`.
  - Nothing new from a request reaches a header value or a filesystem path.
- **Duplicate decision ids D-0370 and D-0372** already existed at 331b05c, so they are not new.

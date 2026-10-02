## o1runner-1 · medium cost · runner
**Where:** `crates/runner/src/resolved_grid_view.rs:143-173; crates/cli/src/boolean_candidate_v1.rs:673-686, 757-760`
**Finding:** Boolean candidate production re-checks and re-hashes the same training data about six times for every rule and side, although the code's own doc says to do this once.
**Evidence:** boolean_candidate_v1.rs:673 for program .. for resolved in &resolutions -> :757 resolved.attest_training(series, column, ..); exit_grid_policy.rs:2052: 'should attest once'
**Fix:** Attest once per resolution (2 per request) and reuse the token.

## o1runner-2 · medium cost · runner
**Where:** `crates/runner/src/expression_oos.rs:167-215; crates/cli/src/boolean_oos_v1.rs:425-460`
**Finding:** Out-of-sample checking of Boolean rules re-validates and re-hashes the same later data, and rebuilds the bar-to-fold map, for every rule and side, although none of that work depends on the rule.
**Evidence:** expression_oos.rs:187 validate_execution_bars, :198-205 checks and data_digest(bars); expression_validation.rs:164 per-bar mapping; caller loop boolean_oos_v1.rs:425
**Fix:** Build one OOS attestation token and one shared bar-to-fold map per series and column.

## o1cli-1 · medium cost · cli
**Where:** `crates/cli/src/lib.rs:13896-13905, 14567-14573`
**Finding:** Each step of a support-threshold descent reloads all the price data and rebuilds the condition column from scratch, though none of it depends on the threshold; the docs mention only the frontier and trade walk cost.
**Evidence:** for (step, support) in ladder.iter().enumerate() { .. screen_step(..) } -> load_span at :15335, :15344, contexts at :15409/:15411, prepared_column: None at :15461
**Fix:** Load once before the loop and pass prepared_column: Some(column).

## o1api-44 · medium cost · pull
**Where:** `crates/pull/src/fold.rs:802-896; ingest.rs:2480-2512; calendar.rs:114`
**Finding:** For all data after 4 Sep 2026 the calendar is unknown, so every derived bar is withheld and each one produces its own warning text and log event, repeated every download window for 7 timeframes.
**Evidence:** Probe: 5 weekdays from 2026-09-07, 1,800 minute bars, 2-minute rung -> complete=0 diagnostics=905 diag_bytes=158085
**Fix:** Aggregate diagnostics per day or per run and emit once per rung per window.

## probestore-1 · medium bug · pull
**Where:** `crates/pull/src/csv.rs:309-319`
**Finding:** The CSV time reader accepts plus and minus signs, so a garbled time like -9:15:00 is silently stored as a bar on a different minute or even the previous day instead of being rejected.
**Evidence:** csv::decode("20240103,-9:15:00,100.00,5,0") -> ACCEPTED ist=2024-01-02 15:15; window filter keeps it; code checks h>23||m>59||s>59 with no lower bound or digit check
**Fix:** Require exactly two ASCII digits per field before parsing.

## probestore-3 · medium bug · store
**Where:** `crates/store/src/catalog.rs:205-209`
**Finding:** The store's directory scan follows folder shortcuts (symlinks) without remembering where it has been, so a shortcut pointing back to its own folder makes sweep-all run essentially forever with no error.
**Evidence:** bars/x/{a,b,c} -> . then catalog::walk(root): 'fanout walk STILL RUNNING after 60.000167041s (3 self-symlinks)'
**Fix:** Do not follow symlinked directories, or track visited directories and refuse a loop.

## probeengine-1 · medium bug · costs
**Where:** `crates/costs/src/trip.rs:938; crates/costs/src/fill.rs:437-443`
**Finding:** Pricing a trade that exits at a bar's printed low can crash the program on an extreme negative low, and a modest negative low is priced as a real fill at a negative price; no production path does this today.
**Evidence:** fills_at(.., PrintedExtreme) with low i64::MIN -> 'panicked at crates/costs/src/trip.rs:938:17: attempt to subtract with overflow'; low -100 -> Ok(.. sell_fill: Paisa(-10000), gross_pnl: Paisa(-20100) ..)
**Fix:** Refuse a negative or sub-tick low under PrintedExtreme and use checked subtraction in charge_stack.

## probeapi-1 · medium bug · api
**Where:** `crates/api/src/server.rs:16115-16127`
**Finding:** The web server never times out a client that sends half a request and goes quiet, so enough such clients can use up every connection slot and block real requests.
**Evidence:** partial GET /health still open after 100 s; 19,990 partial connections -> server fd count 20000 (limit 20000); GET /vocab.json got 0 bytes in 10.2 s
**Fix:** Set a header read timeout and/or a connection limit.

## probeapi-2 · medium bug · api
**Where:** `crates/api/src/server.rs:2396-2399, 2979-2986`
**Finding:** The gaps page answers 'success' with a full missing-data report for nonsense or unsafe addresses (empty or '..' parts) instead of refusing them, while the bars page correctly refuses the same input.
**Evidence:** /gaps.json?month=2026-01 -> 200 {.. lost_minutes:7500, absent_file:" 2026-01: path segment exchange is empty", reason:"vendor-hole"}; same on /bars.json -> 400
**Fix:** Refuse an invalid store path with 400 instead of treating it as an absent month.

## probeapi-3 · medium bug · api, cli
**Where:** `crates/api/src/main.rs:33; crates/cli/src/main.rs:66`
**Finding:** Both programs crash (exit 101) when given a command-line argument that is not valid text, instead of refusing it, and skip their normal exit logging.
**Evidence:** cli $'\xff' -> panicked at .../std/src/env.rs:876:51: called Result::unwrap() on an Err value: "\xFF", rc=101; same for api
**Fix:** Use std::env::args_os() and refuse non-UTF-8 arguments with a documented exit code.

## rustonly-6 · medium test · ci
**Where:** `.github/workflows/ci.yml:1827 (gate 13)`
**Finding:** The CI gate meant to stop foreign or native code passes even when the banned ring C library is switched back on and compiled, so only cargo-deny (unverified here) would catch it.
**Evidence:** ring enabled via rustls-webpki feature, 30 .o files built; gate 13: 'OK — no foreign runtime is declared, resolved, or built', exit 0
**Fix:** Make gate 13 check what cargo actually builds (cargo tree -e normal,build for the host target) against the banned list.

## o1store-1 · low doc · store
**Where:** `crates/store/src/file.rs:1854-1861`
**Finding:** A code comment says writing to the store costs almost nothing when logging is quiet, but every write also builds a text copy of the file path that is then thrown away.
**Evidence:** Comment: 'pays one relaxed atomic load and writes nothing'; code: .with("file", Value::Str(&self.bars_path.display().to_string())) before emit filters
**Fix:** Use telemetry::emit_if! so the path is only formatted when the event is kept.

## o1store-2 · low cost · core
**Where:** `crates/core/src/price.rs:180-221`
**Finding:** The function that reads a price written as text checks the whole text three times and never refuses an overly long input, so its speed depends on callers keeping the text short, and two data-pull callers do not.
**Evidence:** bytes().all(..) twice and fraction.bytes().skip(3).any(..); no text.len() check; pull callers http.rs:1248 and rolling.rs:679 pass to_string() of a decoded cell
**Fix:** Refuse text longer than a fixed MAX_PRICE_TEXT at the top of the function.

## o1engine-22 · low cost · vocab
**Where:** `crates/vocab/src/expression.rs:209-212`
**Finding:** Checking a Boolean rule against each bar takes time that grows with the rule's length (up to 1151 steps), and it also clears a 1151-slot scratch area every time even for a one-condition rule; no document states this per-bar cost.
**Evidence:** let mut stack = [Truth::Unknown; MAX_INSTRUCTIONS]; for instruction in self.code.iter().take(self.len); called per row at runner/src/expression.rs:107-108. Timing UNVERIFIED (probe not run)
**Fix:** Size the scratch stack to the rule's maximum depth at parse time (or use MaybeUninit) and document the per-bar bound.

## o1engine-23 · low cost · vocab
**Where:** `crates/vocab/src/expression_search.rs:278-300`
**Finding:** While searching rule shapes, every step re-checks the whole partial rule from scratch and clears a 9.2 KB array, so each step gets slower as the rule grows; this is not documented.
**Evidence:** fn valid_prefix(..) { let mut starts = [0_usize; MAX_INSTRUCTIONS]; ... if code.get(left..right) > code.get(right..index) { return false; } }, called per node at :134
**Fix:** Keep the starts stack inside the Cursor and update it as the search moves.

## o1engine-40 · low cost · indicators
**Where:** `crates/indicators/src/column.rs:753-762`
**Finding:** Merging the exact-minute evidence into each row loops over 31 condition positions one at a time when a single whole-mask operation would do the same job.
**Evidence:** for position in (*positions).clone() { without_local = without_local.without_bit(position); *known = known.without_bit(position); ... }
**Fix:** Precompute a constant position mask once and combine with six-word mask operations.

## o1engine-20 · low doc · engine
**Where:** `crates/engine/src/keep.rs:223-224, 304-325`
**Finding:** The top-results keeper takes time that grows with its size (log of capacity) to admit a new item, and its doc does not state that bound; it has no production caller.
**Evidence:** self.held.push(candidate); self.sift_up(...); sift_up/sift_down at :378-428
**Fix:** State the O(log cap) admit cost in the doc.

## o1runner-3 · low cost · runner
**Where:** `crates/runner/src/exit_grid_policy.rs:416-433`
**Finding:** Locating the trade slice inside the minute data is done by searching bar by bar and then comparing every bar, once per candidate side, where simple position arithmetic would do.
**Evidence:** let Some(start) = reference_minute_context.iter().position(|bar| bar == first) else {..} (:423); slice compare at :429
**Fix:** Compute the start from pointer offsets or a timestamp-derived index.

## o1runner-4 · low cost · runner
**Where:** `crates/runner/src/validate.rs:4865, 4911`
**Finding:** Each out-of-sample fold builds a lookup structure and then calls a helper that builds the identical structure again.
**Evidence:** let test_facts = SliceFacts::of(trade_test, confined); ... walk(trade_test, confined, ..) which rebuilds SliceFacts::of(bars, column)
**Fix:** Call walk_over(.., &test_facts).

## o1runner-5 · low cost · runner
**Where:** `crates/runner/src/validate.rs:4597, 4613; crates/runner/src/outcome.rs:585-588`
**Finding:** Each training fold builds the same lookup structure twice, once directly and once inside forward().
**Evidence:** validate.rs:4597 SliceFacts::of(trade_train, train_column); :4613 outcome::forward(..) builds its own at outcome.rs:587
**Fix:** Add forward_over(bars, column, horizon, &facts) and pass the existing facts.

## o1runner-6 · low cost · runner
**Where:** `crates/runner/src/grid.rs:2319-2334, 1723-1731`
**Finding:** Building the stop/target ratio table tests every stop-target pair against up to 512 ratios for each candidate, with no stated cost and no pre-sizing.
**Evidence:** for stop .. { for target .. { bitmap.push(pairs_at_a_ratio(..)) } }; ratios.iter().any(..); cap out.len() < 512
**Fix:** Use an exact integer test against a ratio set (O(1) per pair) and reserve S*T.

## o1runner-7 · low cost · runner
**Where:** `crates/runner/src/bootstrap.rs:1381-1398`
**Finding:** The older Romano-Wolf significance step recomputes averages in every round, so its worst case is one factor of strategies worse than documented; it is still used in production.
**Evidence:** while !alive.is_empty() { .. for &s in &alive { let centred = mean_at(series, index) - own.mean; ..; callers cli lib.rs:17490, institutional_evidence.rs:2537; capped at 16 candidates
**Fix:** Compute the null-statistic matrix once, or reuse the suffix-max scheme of the newer function.

## o1runner-8 · low cost · runner
**Where:** `crates/runner/src/outcome.rs:146-147`
**Finding:** A per-day map is created empty and grows by resizing, against the project's rule to pre-size every map.
**Evidence:** let mut required: HashMap<i64, (u64, Option<usize>)> = HashMap::new();
**Fix:** Use with_capacity(bars.len()/375+1) or a day-indexed Vec.

## o1runner-9 · low doc · docs
**Where:** `docs/06-limits.md:6908 (§126)`
**Finding:** The limits document says fold cutting uses a slower binary search, but the code now uses a faster running cursor; the doc is out of date in the pessimistic direction.
**Evidence:** 06-limits: 'Binary partition_point ... is O(log E)'; validate.rs:3313-3360 MonotonicExecutionPrefix::advance; grep partition_point in runner/cli -> no output
**Fix:** Update §126 to say amortised O(1) per bar.

## o1runner-10 · low doc · docs
**Where:** `docs/06-limits.md §113`
**Finding:** The limits document says a trade walk costs time proportional to the signals, but it actually visits every row of the column, so the stated cost is too low.
**Evidence:** trade.rs:690 for (index, (bits, &signal)) in column.bits().iter().zip(column.sources()).enumerate() { if !fires(bits, index) { continue; }
**Fix:** Restate §113 as linear in column rows.

## o1runner-11 · low cost · runner
**Where:** `crates/runner/src/audit.rs:946-947`
**Finding:** To print only the top rows of a report, the code sorts every grid cell; this runs once per report, not on a sweep hot path.
**Evidence:** let mut ordered: Vec<(usize, &Cell)> = g.cells.iter().enumerate().collect(); ordered.sort_by_key(..)
**Fix:** Select the top keep rows first, then sort only those.

## o1cli-2 · low cost · cli
**Where:** `crates/cli/src/lib.rs:13086, 13127-13149, 13190`
**Finding:** Each rung loads its data and may build the column just to size a threshold, then the audit loads and builds it all again; only a code comment admits this.
**Evidence:** one_rung: load_span(..), column_withholding_unsourceable_days(..), affordable_min_hits(..), then audit_range_for_attempt(..); stored.rs:2815-2816 'twice per rung'
**Fix:** Pass the loaded span and column into the audit kernel.

## o1cli-3 · low cost · cli
**Where:** `crates/cli/src/lib.rs:14776-14799`
**Finding:** Running all 8 timeframes in parallel makes each one reload the same 1-minute data 3-4 times, about 24-32 reads of identical data per command.
**Evidence:** rungs.par_iter().map(|&rung| one_rung(..)); stored.rs:3274 load_span(.., "1min", ..); lib.rs:6585 load_span(.., EXECUTION_RUNG, ..)
**Fix:** Load the 1-minute span once per command and share it across rungs.

## o1cli-4 · low cost · cli
**Where:** `crates/cli/src/lib.rs:6658-6677`
**Finding:** The audit reads the daily and minute context data a second time just to confirm it has not changed, a full extra read that is not stated as a cost.
**Evidence:** column_withholding_at_build(..)?; exact_minute_withholding_unsourceable_days(..)?; load_daily_context(..)?; if stored_anchored_digest(..)? != preparation_digest {
**Fix:** Reuse the contexts already loaded and keep the change check with a cheaper file-generation test.

## o1cli-5 · low cost · cli
**Where:** `crates/cli/src/lib.rs:14173-14181, 14239-14260, 14324, 13682`
**Finding:** Several commands load the whole price history just to read one reference price or count bars, throw it away, and load it again for the real work.
**Evidence:** let span = stored::load_span(..); let reference = reference_price(&span.bars); drop(span); descent_bar_count: loaded.bars.len()
**Fix:** Read the record count from the file header (O(1)) or share one load.

## o1cli-6 · low cost · cli
**Where:** `crates/cli/src/lib.rs:12008, 12064`
**Finding:** The screen sorts up to 10 million rows twice although only the top few are measured or printed; neither sort is documented.
**Evidence:** rows.sort_by_key(|r| money_key(&r.cell)); .. rows.sort_by_key(|r| { Reverse((r.admitted, ..)) }); .. rows.iter_mut().take(rules.top)
**Fix:** Select the top rows first, then sort only those (unless the full order is persisted).

## o1api-3 · low cost · api
**Where:** `crates/api/src/server.rs:16115-16127`
**Finding:** The web server sets no explicit limit on request header size and relies on the HTTP library's hidden default of about 418 KB; no document states it.
**Evidence:** axum::serve(listener, app).with_graceful_shutdown(..) with no limit; hyper DEFAULT_MAX_BUFFER_SIZE = 8192 + 4096*100
**Fix:** Set an explicit header/URI cap and document it.

## o1api-4 · low cost · api
**Where:** `crates/api/src/server.rs:692-700, 682, 2095-2136`
**Finding:** Reading each URL parameter rescans the whole query string, and some routes do this seven times, so cost grows with query length times parameter count.
**Evidence:** let prefix = format!("{name}="); for pair in raw.split('&') { if let Some(v) = pair.strip_prefix(..) { return percent_decode(v); } }
**Fix:** Parse the query once into slices.

## o1api-21 · low cost · api
**Where:** `crates/api/src/audit_json.rs:420-432; server.rs:3499; census.rs:810-816`
**Finding:** The audit page, polled every few seconds, walks every stored entry of all five vendors on each poll with nothing cached.
**Evidence:** for (series, month) in entries.iter() { .. census.rows_for(..) .. months.entry(*month) } where entries = census::held_entries(&censuses) over every vendor
**Fix:** Cache the month roll-up keyed on the census generation, as /store.json does.

## o1api-33 · low cost · pull
**Where:** `crates/pull/src/http.rs:820-823`
**Finding:** Each vendor response (up to 64 MB) is first turned into a full in-memory JSON tree before being converted, so peak memory is several times the body and has never been measured.
**Evidence:** let root: serde_json::Value = serde_json::from_str(body)...
**Fix:** Use a streaming or typed decode straight into arrays.

## o1api-34 · low doc · pull
**Where:** `crates/pull/src/csv.rs:41-43, 563, 588`
**Finding:** The CSV reader's doc says it allocates nothing per row and pre-sizes its output, but the code allocates a new list for every row and does not pre-size.
**Evidence:** Doc: 'No allocation per row ... reserved from a caller-supplied bound'; code: let mut rows: Vec<RawRow> = Vec::new(); let fields: Vec<&str> = line.split(',').collect();
**Fix:** Split into a fixed [&str; 10] array and use Vec::with_capacity(bound).

## o1api-36 · low cost · pull
**Where:** `crates/pull/src/ingest.rs:1710-1712`
**Finding:** Each fetched member's rows are copied once more before being stored, an avoidable full copy.
**Evidence:** let raw = fetch::RawWindow { rows: member.rows.clone() };
**Fix:** Borrow the rows (land_* already takes &RawWindow).

## o1api-39 · low doc · pull
**Where:** `crates/pull/src/calendar.rs:371-414`
**Finding:** The holiday lookup's cost note mentions one small table walk but the code walks two; harmless, just inaccurate.
**Evidence:** while m < LENGTH_UNMEASURED.len() {..} while i < IRREGULAR.len() {..}; doc mentions only the four-element IRREGULAR walk
**Fix:** Mention the 5-entry LENGTH_UNMEASURED walk in the doc.

## o1api-54 · low cost · pull
**Where:** `crates/pull/src/http.rs:534-568; crates/api/src/server.rs:8187-8262`
**Finding:** Waiting for the download rate limiter is not first-come-first-served: waiters wake together and race, one loop never gives up, and the server's version can refuse after 64 losses with a message that blames the wrong cause.
**Evidence:** loop { match g.admit(now) { Admit => return, Deny { wait_micros, .. } => .. } sleep(..) } and for _ in 0..MAX_ADMISSION_WAITS { held.admit(..) }; real contention UNVERIFIED
**Fix:** Use a ticketed FIFO reservation so each acquire is one O(1) call.

## probestore-2 · low bug · pull
**Where:** `crates/pull/src/csv.rs:323-351`
**Finding:** The CSV date reader accepts plus signs, so malformed text like 2024+1+3 is read as 3 Jan 2024 instead of being refused.
**Evidence:** "2024+1+3,10:00:00,100.00,5,0" -> ACCEPTED ts=1704256200 (2024-01-03 10:00 IST)
**Fix:** Require digit-only date fields before parsing.

## probestore-4 · low bug · store
**Where:** `crates/store/src/catalog.rs:308-317`
**Finding:** A hand-made file named 2024-+1.bin is listed as the January 2024 month, but opening that month later reads a different file, 2024-01.bin.
**Evidence:** file bars/groww/NSE/INDEX/NIFTY/1min/2024-+1.bin -> held=[.. YearMonth{2024,1}] census{spot:1, malformed_month:0}
**Fix:** Require digits only in the month name.

## probestore-5 · low bug · store
**Where:** `crates/store/src/catalog.rs:232-235`
**Finding:** Folder names that are not valid text are silently dropped when counting folder depth, so an options-contract file can be miscounted as a plain index holding.
**Evidence:** bars/groww/NSE/FNO/NIFTY/<0xFF>CE/1min/2024-01.bin -> held=[.. NIFTY ..] census{spot:1, with_contract:0, wrong_depth:0}
**Fix:** Refuse or count non-UTF-8 components instead of filtering them out.

## probestore-6 · low bug · core
**Where:** `crates/core/src/price.rs (from_rupees_half_up)`
**Finding:** Converting a decimal rupee number to paisa can round the wrong way: a value just below a half-paisa rounds up, and very large odd values come out one paisa too high.
**Evidence:** from_rupees_half_up(0.004999999999999999) -> Paisa(1) (text path gives 0); x=(2^52+1)/100 -> Paisa(4503599627370498), off by one
**Fix:** Round without the add-0.5-then-floor step (e.g. compare the fractional part exactly).

## probestore-7 · low bug · lake
**Where:** `crates/lake/src/reader.rs:255-270`
**Finding:** The lake file reader accepts impossible values (negative prices or volume, low above high, extreme timestamps) and turns a literal minimum open interest into 'no value', though its doc says everything bad is refused; nothing uses lake yet.
**Evidence:** ACCEPTED Bar{timestamp_micros:-9223372036854775808, open:10000, high:100, low:20000, volume:-7, open_interest:i64::MIN} open_interest()=None
**Fix:** Refuse the i64::MIN open-interest sentinel by name, and validate or document the other fields.

## probeengine-2 · low bug · runner
**Where:** `crates/runner/src/resample.rs:72-76, 79-83`
**Finding:** The runner's resampler lines up bars from midnight rather than the 09:15 market open, so the first bar of the day is a partial bar stamped before the open, contrary to its doc; odd periods like 7 minutes shift day to day.
**Evidence:** period 60: first bar stamped 09:00 holding 45 of 60 minutes; period 7: 09:14 on day 19723, 09:09 on day 19724
**Fix:** Anchor buckets at the session open like pull::fold, and refuse periods that do not divide the session.

## probeapi-4 · low bug · api
**Where:** `crates/api/src/server.rs:87`
**Finding:** 'api serve ADDRESS' ignores any extra words after the address and starts serving, though its own comment says unrecognised arguments must be refused.
**Evidence:** api serve 127.0.0.1:18794 --typo-flag 0.0.0.0:80 -> 'brutex api listening on http://127.0.0.1:18794/'
**Fix:** Refuse any argument after ADDR.

## probeapi-5 · low bug · api
**Where:** `crates/api/src/server.rs:692-700`
**Finding:** If a URL repeats a parameter, the server silently uses the first value and ignores the rest, even an invalid one, contrary to its own rule that a query must never silently change meaning.
**Evidence:** /audit.json?feed=zerodha&feed=dhan -> "feed":"zerodha"; ?feed=dhan&feed=bogus -> 200; repeated Host header -> 403
**Fix:** Refuse a repeated query key.

## probeapi-6 · low bug · cli
**Where:** `crates/cli/src/lib.rs:2056-2063`
**Finding:** 'cli sweep' with 1-5 sessions reports that nothing was measured yet exits with success code 0, while 'cli audit' with the same input exits 1; affects generated data only.
**Evidence:** cli sweep 1 10 -> 'outcome NOTHING MEASURED', 'trustworthy as a whole answer NO', rc=0; cli audit 1 10 -> rc=1
**Fix:** Exit non-zero when nothing was measured, and update the test that pins sweep 1 1 as OK.

## probeapi-7 · low bug · api
**Where:** `crates/api/src/server.rs:16980`
**Finding:** If the server's output is closed while it prints its start-up banner, it crashes after already taking the store lock, leaving lock and telemetry files behind.
**Evidence:** api serve 127.0.0.1:18794 2>err.txt | head -1 -> rc=101, 'failed printing to stdout: Broken pipe (os error 32)'; serve.lock and telemetry/ left
**Fix:** Write the banner with error handling instead of println!.

## rustonly-3 · low doc · docs
**Where:** `docs/06-limits.md:6012`
**Finding:** The limits document says no build script exists in the repository, but crates/cli/build.rs exists and gate 13 allow-lists it by name.
**Evidence:** git ls-files '*build.rs' -> crates/cli/build.rs; ci.yml:2008 allow_build='crates/cli/build.rs ...'
**Fix:** Correct the sentence to name the one allowed build script.

## rustonly-4 · low law · api
**Where:** `crates/api/src/server.rs:16733-16755`
**Finding:** On Linux the api server starts xdg-open to open a browser, which is usually a shell script, so the shipped binary can launch an interpreter. No document mentions it.
**Evidence:** BrowserHost::Other => ("xdg-open", &[]); docs grep for xdg-open: 0 hits. Whether it is a script on the target host is unverified.
**Fix:** Record a decision that allows it as an OS handler, or drop the auto-open on Linux.

## audit-root · low test · api, pull, store, telemetry
**Where:** `e.g. crates/api/src/folder.rs:494-499`
**Finding:** 16 permission tests fail when the suite runs as root, because root can read a folder set to no permissions. They pass as a normal user.
**Evidence:** cargo test as uid 0: 16 FAILED; same binaries as user 'tester': 0 failed.
**Fix:** Skip or adapt those tests when running as root, and say so in the test output, so cloud and container runs are green.

## o1engine-24 · info cost · vocab
**Where:** `crates/vocab/src/expression.rs:562-565`
**Finding:** Turning a condition name into its number scans the whole 370-row table for each word; it only happens when a rule is parsed, so it is not on a hot path.
**Evidence:** table::TABLE.iter().find(|row| row.name == token).map(|row| row.index)
**Fix:** Use a compile-time perfect hash of names, as core's member tables do (low value).


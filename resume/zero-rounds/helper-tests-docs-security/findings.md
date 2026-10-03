# Zero rounds: tests, docs, security, CI (helper audit)

Audited: final/all-fixes @ 331b05c, read only. IDs P<pass>-<agent>-<n>. All 20 agents done; agent 12 appended at the end.

Known overlap: P1-01-01 and P1-02-02 are the same /bars/window.json silent-default issue (fix once).


---
## Source: 01-api-validation-a-l.md

# P1-01 — crates/api request parsing and query validation, files a-l (static, commit 331b05c)

Scope: `crates/api/src/[a-l]*.rs`, non-test. Where an a-l module gets its values from a parser in `server.rs`, that parser was read too. Deduplicated against `docs/11-findings.md` and `/mnt/project-files/audit-20261003-workspace/*.md`: hunt-api-5 (repeated body keys), probeapi-5 (repeated query keys) and o1surface2 (window scan cost) are already recorded and are not repeated here.

### P1-01-01 low `/bars/window.json` silently clamps `limit` to 1000, answers `limit=0` with an empty 200, and reads any `dir` other than exact `asc` as descending. The answer echoes none of the three.
- Where: crates/api/src/bars.rs:944; crates/api/src/server.rs:3324-3327 (`WindowAsk::parse`, which feeds `bars::window`); crates/api/src/bars.rs:1104
- Evidence:
  - bars.rs:944 `let limit = limit.min(MAX_WINDOW_LIMIT);` (`MAX_WINDOW_LIMIT: usize = 1_000`, bars.rs:538)
  - server.rs:3324 `desc: !matches!(param(query, "dir").as_str(), "asc"),`
  - server.rs:3327 `want_extremes: matches!(param(query, "extremes").as_str(), "1" | "true"),`
  - bars.rs:1104 `if limit == 0 || offset >= all.len() { return Vec::new(); }`, and `seek_page` with `limit == 0` also returns no rows.
  - The response written by `render_window` (server.rs ~3446) is `{"total":..,"months_read":..,"months_missing":..,"scanned":..,"extremes":..,"faults":..,"bars":..}`. It carries no `limit`, `offset` or `dir`.
  - The parser's own header (server.rs:3252) says: "Every field REFUSES rather than defaulting where a default would answer a different question than the one asked". The `offset=banana` comment says the same.
- Why it is wrong: `dir=ASC`, `dir=ascending` and `dir=up` each return the newest-first page, and `offset` is then counted from the other end of the window. That is a different set of rows, with nothing in the answer to show it. `limit=5000` returns 1000 rows. A caller that advances `offset` by the limit it asked for skips 4000 rows on every page, and the server never says it shortened the page. `limit=0` gets HTTP 200 with `bars: []` beside a non-zero `total`. This is the silent fallback CLAUDE.md §4 bans, and it contradicts the parser's own stated rule. Today's browser callers stay within bounds (`DAY_BUDGET = 512`, `dir: desc ? 'desc' : 'asc'`), so this is latent for the UI and real for any direct caller.
- Repro: `GET /bars/window.json?feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&from=2025-01&to=2025-01&dir=ASC&limit=5000` gives newest-first rows, `bars.length == 1000`, and no field that names either change. Test sketch: `WindowAsk::parse("...&dir=ASC").unwrap().desc == true`, and `bars::window(.., limit=5000, ..).bars.len() == 1000`, with no flag set.
- Suggested fix: refuse `dir` outside {``, `asc`, `desc`}, `extremes` outside {``, `0`, `1`, `true`, `false`}, and `limit` outside `1..=MAX_WINDOW_LIMIT` with a 400, the way `offset=banana` is already refused. Alternatively, echo the effective `limit`/`offset`/`dir` in the body.

### P1-01-02 low `/logs` and `/logs.json` silently drop an unrecognised `level` or a malformed `run` filter and return unfiltered events. The JSON never echoes either filter.
- Where: crates/api/src/logs.rs:131-152 (`asked`), crates/api/src/logs.rs:437-452 (`json_of` trailer)
- Evidence:
  - `let level = telemetry::Level::of_label(&level_word.to_ascii_lowercase());` followed by `if let Some(level) = level { query = query.at_least(level); }`. An unknown word becomes `None`, which means no floor.
  - `let run = crate::server::param(raw, "run").parse::<u64>().unwrap_or(0);` and "ZERO IS "EVERY RUN"". So `run=abc`, `run=-3` and `run=1.0` all mean every run.
  - The JSON trailer writes `"limit":{}` (the clamped value) but no `level`, `target` or `run`.
  - The stated justification (logs.rs:120-123) is: "nothing here can be made wrong by a bad number — the worst a bad `limit` can do is show a different count."
- Why it is wrong: the justification covers `limit` only. A malformed `run` or `level` does not change a count. It replaces a narrowed answer with the unfiltered one. A poller asking `?run=<id>&level=eror` gets every run's debug and info events, with nothing in the body to tell it apart from a correctly filtered answer. The JSON trailer never echoes either filter. The same crate treats this exact pattern as a §4 violation elsewhere: `audit_json::note_page_ignored` says that "?page=banana and ?page=0 produce byte-identical answers ... Taking the default and then saying so is CLAUDE.md §4", and it emits a Warn for it. `/logs` does neither.
- Repro: `GET /logs.json?level=eror&run=12x` returns the same records as `GET /logs.json`. Test sketch: `assert_eq!(asked("level=eror&run=12x").query, asked("").query)`. It passes today.
- Suggested fix: refuse an unknown `level` and a non-canonical `run` with a 400, or echo the effective `level`/`target`/`run` in the JSON and emit one Warn the way `note_page_ignored` does.

### P1-01-03 low `/backtest.json` silently uses `DEFAULT_LIMIT` for an unparseable `limit` and says nothing, unlike `/audit.json`'s identical `page` default.
- Where: crates/api/src/backtest.rs:1283-1288 (`limit_asked`)
- Evidence: `crate::server::param(raw, "limit").parse::<usize>().unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_RUNS)`. The doc reads "An unparseable value takes [`DEFAULT_LIMIT`] for the same reason." There is no telemetry, and the response has no `limit` field (it carries `total`/`scanned`/`hit_scan_cap`/`max_runs`).
- Why it is wrong: `?limit=all`, `?limit=-1` and `?limit=1e3` return the default-sized window, which looks the same as an intentional default request. `audit_json.rs:189-205` documents this exact shape (an invisible default for an unparseable view parameter) as a §4 problem, and fixed it there with a Warn. The two routes in the same crate now disagree on whether this is allowed.
- Repro: `limit_asked("limit=all") == limit_asked("")`, and the answers are byte-identical. No `api.backtest` event is emitted.
- Suggested fix: emit a Warn for a present-but-unparseable `limit`, as `note_page_ignored` does, or refuse it.

### P1-01-04 low `detail::Selector`/`Page` (used by `/frontier.json` and `/trades.json`) accept unknown keys and non-canonical integers that every other detail route refuses.
- Where: crates/api/src/detail.rs:240-262 (`Page::parse`), :291-305 (`Selector::parse`), :315-325 (`integer_param`); caller crates/api/src/frontierjson.rs:81-91
- Evidence:
  - `Selector::parse` reads only `page`, `limit` and `identity` through `param` and never walks the pairs. In contrast, each of `candidatejson.rs:30-52`, `booleanjson.rs:38-58`, `booleanevidencejson.rs:59-71`, `booleanoosjson.rs:28-40`, `booleansearchjson.rs:28-40`, `booleancampaignjson.rs:24-31`, `expressionsearchjson.rs:25-38` and `indexstop*json.rs` returns `Err("empty, repeated or unknown ... field")` for any key outside its list.
  - `integer_param` is `value.parse::<u64>().map(Some)`. It accepts `+3` and `003`, and treats `page=` (present but empty) as absent. `candidatejson::integer` refuses both: `if raw != value.to_string() { return Err("... canonical unsigned integer") }`.
- Why it is wrong: the boolean/indexstop routes paginate with `offset`, and frontier/trades paginate with `page`. A client using the wrong vocabulary on frontier or trades gets an answer: `?identity=X&offset=256&limit=16` returns page 0 again and again, and a typo like `pgae=3` falls back to page 0. The same mistake on every sibling route is a 400. One crate applies two different rules to the same "is this selector exact" question.
- Repro: `Selector::parse(Ok(root), "identity=<64hex>&offset=256")` returns `Ok` with `page.number == 0`. `candidatejson::Asked::parse("identity=..&attempt=1&offset=+1")` returns `Err`, but `Page::parse("page=+1")` returns `Ok(number=1)`.
- Suggested fix: give `Selector::parse` the same allowed-key/non-empty walk the sibling parsers use (`identity|page|limit`), and use `candidatejson::integer` (the canonical check) for `page`/`limit`.

## Checked and holding (no finding)
- Strict detail parsers (`candidatejson`, `boolean*json`, `expressionsearchjson`, `indexstop*json`): bounded by `query_is_bounded`; refuse unknown, empty and repeated keys; use canonical integers and lowercase hex; bound `limit` to 1..=256; require continuation pins. Every `offset + n` and slice is reached only after `window`/`next_offset`/`checked_add` has bounded it (`booleanjson.rs:290-313`, `indexstopjson.rs:224-232`, `indexstopvixjson.rs:52`, `indexstopcandlesjson.rs:58-64`).
- `detail::Page::parse`: `page <= MAX_PAGE`, `limit` in 1..=256, and a checked `page*limit`.
- `audit_json`: a missing or unknown feed gives a 400 plus Warn; `page` is clamped to the last page and echoed; an unparseable page gives a Warn.
- `folder.rs`: an unknown feed, a REST feed, or an unknown or ambiguous segment is refused by name.
- `ingest.rs` form parsers: digits-only dates (D-0905), `MAX_WINDOW_DAYS`, future-window refusal, `MAX_MEMBERS` checked before allocation, members refused rather than skipped, members that do not resolve caught by `server::spot_mapping_refusal`, `rate` NaN/inf/out-of-band refused by `Rate::measured`, expiry gate, unknown vendor or granularity refused (absent → Dhan/1min, documented).
- `autopilot::control`: exact `action` slug only.
- `booleanlaunch`/`indexstoplaunch_metadata`: exact `max_points=` only, or a strict key walk with canonical positive decimals.
- `assets.rs`: single-pass percent-decode that rejects bad escapes, NUL and non-UTF-8, plus a one-name segment rule.
- `census::coverage_page`/`held_page`: saturating ordinal arithmetic. `bars::months_of`: inverted or >240-month ranges refused.
- No `unwrap`/`expect`/indexing on request data found in non-test code in these files.

---
## Source: 02-api-validation-m-z.md

# P1-02: crates/api request parsing and query-parameter validation (files m-z)

Repo: /home/claude/brutex-audit @ 331b05c. Static reading only.
Not repeated here because it is already recorded: repeated keys in a POST body are read first-match-wins (hunt-api-5 / attacksweep-3).

### P1-02-01 medium `POST /pull/run` trusts each leg's self-declared vendor and rung and never checks them against the payload it runs
- Where: crates/api/src/pullrun.rs:418-455 (`legs_from`), :140-146 (`ladder_rank`), :474-485 (`by_feed`), :766-791 (`run_chain` gating); payload parse at crates/api/src/ingest.rs:1495-1502
- Evidence:
  - `legs_from` splits `route|vendor|dir|label|payload`. It checks only that `vendor` is some `Feed::ALL` wire name and stores `dir` as given: `vendor: vendor.to_owned(), dir: dir.to_owned(), ... body: percent_decode(payload)`.
  - The chain, the ladder order and the failure gating all use the envelope: `groups.iter_mut().find(|(vendor, _)| *vendor == leg.vendor)`, `held.sort_by_key(|leg| ladder_rank(&leg.dir))`, `ladder_rank(&leg.dir) > rank`, `failed_spot_rank = Some(ladder_rank(&leg.dir))`.
  - What actually runs is `pull_spot(.., leg.body)`, which reads the payload's own fields: `let raw = param(body, "vendor"); parse_feed(&raw)` and `let raw = param(body, "granularity");`. `parse_feed("")` answers `Feed::Dhan`.
  - The recovery path checks exactly this and refuses: recovery.rs:173-176 `if leg.route != Route::Spot || leg.vendor != "zerodha" || leg.dir != asked.granularity.dir() { return Err("recovery leg envelope disagrees with its payload".to_owned()); }`.
- Why it is wrong: a leg whose envelope says `groww|1day` while its payload says `granularity=1min` (or has no `vendor=`, which means Dhan) runs a Dhan 1-minute pull. That pull is filed under Groww's chain and status row and sorted onto the ladder as a day pass. The day-before-minute order that `pull::fold` requires, and the rule "a failed daily leg holds back minutes", then rest on a label the server never checked. The same mismatch is refused on `/recovery`, so the two routes validate the same envelope differently.
- Repro: `POST /pull/run` with body `leg=` + enc(`/pull/spot|groww|1day|Spot · 1 day|` + enc(`member=NIFTY&granularity=1min&from=2026-01-01&to=2026-01-31`)). The answer is `202 {"started":true,"legs":1}`. `/pull/run.json` then reports feed `groww`, while the request reaches Dhan's 1-minute path. A unit test: `legs_from(field("/pull/spot","groww","1day","x","granularity=1min"))` returns `Ok`, but the same leg passed to `recovery::plan` returns Err.
- Suggested fix: in `legs_from` (or `pull_run`), parse each payload's `vendor` and `granularity` as `pull_spot`/`pull_fno` would, and refuse the leg unless both equal the envelope's `vendor`/`dir`. This is the check `recovery::plan` already makes.

### P1-02-02 low `/bars/window.json` silently clamps `limit` to 1000 and silently defaults unknown `dir`/`extremes`
- Where: crates/api/src/server.rs:3324-3327 (`WindowAsk::parse`); crates/api/src/bars.rs:944
- Evidence:
  - The parse doc says: "Every field REFUSES rather than defaulting where a default would answer a different question than the one asked." and "A NON-NUMBER IS A REFUSAL, NOT A ZERO ... the pager would look right while showing the wrong rows."
  - But: `desc: !matches!(param(query, "dir").as_str(), "asc"),` / `want_extremes: matches!(param(query, "extremes").as_str(), "1" | "true"),`
  - and in `bars::window`: `let limit = limit.min(MAX_WINDOW_LIMIT);` (`MAX_WINDOW_LIMIT = 1_000`). `render_window` emits `total, months_read, months_missing, scanned, extremes, faults, bars` and never the limit that was applied.
- Why it is wrong: `limit=5000` returns 1000 rows under 200 with nothing saying the page was cut. A pager that steps `offset += limit` then skips rows 1000-4999 without noticing. That is the "pager would look right while showing the wrong rows" case the same function says it refuses. `dir=ASC`, `dir=ascending` or `dir=up` all answer newest-first, and `extremes=yes` quietly omits the extremes. Each is a different question answered in silence (CLAUDE.md §4).
- Repro: `GET /bars/window.json?feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&from=2025-01&to=2025-12&limit=5000` returns `bars.length == 1000` with no field naming the clamp. `...&dir=ASC` returns rows descending.
- Suggested fix: refuse `limit > MAX_WINDOW_LIMIT` in `WindowAsk::parse` (or echo `"limit"` in the body). Accept only `""|"asc"|"desc"` for `dir` and `""|"0"|"1"|"false"|"true"` for `extremes`, and refuse anything else by name.

### P1-02-03 low `/bars` reads `vendor=` while every sibling route reads `feed=`, so `/bars?feed=groww` is silently served Dhan's file
- Where: crates/api/src/server.rs:15673 (`bars_html`), :15448 (`locate_series` also reads `param(query, "vendor")`)
- Evidence: `let asked_vendor = param(query, "vendor"); let Some(vendor) = ingest::parse_vendor(&asked_vendor) else { ... }`. `ingest::parse_vendor("")` is `Some(Vendor::Dhan)` (ingest.rs:1562-1563). `/bars.json` and `/bars/window.json` (`Addressed::parse` / `WindowAsk::parse`: `param(query, "feed")`), `/store`, `/store.json`, `/verify.json`, `/instruments.json`, `/calendar.json` and `/indexmap.json` all read `feed=`. No route refuses unknown keys, so `feed=` on `/bars` is ignored.
- Why it is wrong: a hand-typed or copied `/bars?feed=groww&symbol=NIFTY&month=2025-07` is the same address spelled the way every other read route spells it. It renders Dhan's month under 200. The comment four lines above calls this exact outcome "the worst class of defect in this repository: a wrong answer wearing the shape of a right one".
- Repro: with both Dhan and Groww holding NIFTY 2025-07, `GET /bars?feed=groww&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month=2025-07` returns 200 with Dhan's bars. The page header says `dhan`, but nothing refuses the ignored `feed`.
- Suggested fix: accept `feed=` on `/bars` as the canonical name. Keep `vendor=` as an alias only if needed, and refuse a request that carries both with different values, or one that carries `feed=` while the page reads only `vendor=`.

### P1-02-04 low `POST /universe/resolve` defaults an absent feed to Groww, matches case-sensitively, and answers refusals with 200
- Where: crates/api/src/server.rs:32182-32205
- Evidence: `let feed = if feed.is_empty() { brutex_core::vendor::Vendor::Groww.as_str().to_owned() } else { feed };` then `.find(|v| v.as_str() == feed)`. The handler's return type is `([(HeaderName, &str); 1], String)` with no status, so the `{"ok":false,"why":...}` refusal goes out as 200. Every other feed parse in the crate goes through `ingest::parse_vendor`, where empty means Dhan and matching is `eq_ignore_ascii_case` (ingest.rs:1561-1568).
- Why it is wrong: an absent `feed` means Dhan on every other route and Groww here. This is an unstated per-route default, and it starts a ~148-request crawl against a feed the caller never named. `feed=Dhan` is refused here but accepted everywhere else. A refusal under 200 is invisible to anything that checks status, which this crate elsewhere calls a refusal "a monitor cannot see" (server.rs `store_get` comment).
- Repro: `POST /universe/resolve` with an empty body crawls and reports `"feed":"groww"`. Body `feed=Dhan` gets HTTP 200 `{"ok":false,"why":"\"Dhan\" is not a vendor this build names..."}`.
- Suggested fix: parse with `ingest::parse_vendor`, or require `feed` explicitly and refuse it when absent. Return 400 on the refusal arm.

### P1-02-05 low `/store` ignores `timeframe=`, and the comment claiming it is parsed is false while the store holds two rungs
- Where: crates/api/src/server.rs:15319-15324 (`store_filter`)
- Evidence: `// Not offered in the bar yet: the store holds one timeframe today ... The field is parsed so a URL can still carry it.` followed by `timeframe: None,`. The query is never read for `timeframe`. `census::StoreFilter::keeps` does filter on it (census.rs:947 `if self.timeframe.is_some_and(|t| t != series.timeframe)`). The store holds `MINUTE_1` and `DAY_1` (`timeframe_param` doc: "before the store held a second rung"; render.rs:2460-2463 "for 100% of the data on disk, which is `1day`").
- Why it is wrong: `/store?timeframe=1day` silently lists every rung's rows. The function's own contract ("What it must never do is silently apply a DIFFERENT filter than the one shown") is kept only because nothing is shown, and the comment tells a maintainer the opposite of what the code does.
- Repro: on a store that holds both 1min and 1day NIFTY months, `GET /store?timeframe=1day` returns the same rows and total as `GET /store`.
- Suggested fix: parse `timeframe` with the same `Timeframe::KNOWN` lookup `timeframe_param` uses, refusing an unknown value or rendering it in the filter bar. Otherwise delete the false sentence.

### P1-02-06 low `/engine/top.json` accepts any `feed`/`underlying` and answers a typo with a wrong explanation
- Where: crates/api/src/topjson.rs:96-117 (`parse`), :158
- Evidence: `parse` checks only that the key names are known and present together. The values are used as-is in an exact `BTreeMap` lookup (`self.pairs.get(pair)`), and the stored keys are the lowercase wire names. A miss renders: `"  NO COMPLETE RUN matches. Every matching row halted on a budget or traded nothing, or nothing has been recorded yet"`.
- Why it is wrong: `feed=Zerodha`, which every route parsed with `parse_vendor` accepts, or `feed=zerodah` is not refused. It gets 200 with a sentence saying the matching rows halted or traded nothing, which is a statement about runs that do not exist. Other read routes refuse an unknown feed by name (`no_such_feed_json`).
- Repro: with a populated ledger holding zerodha/NIFTY complete runs, `GET /engine/top.json?feed=Zerodha&underlying=NIFTY` returns 200 `"NO COMPLETE RUN matches. Every matching row halted on a budget..."`.
- Suggested fix: validate `feed` with `ingest::parse_feed`/`parse_vendor` and canonicalise it to the wire name before lookup, refusing an unknown feed with 400. Optionally check `underlying` against the swept universe.

## Checked and holding (no finding)
- `param`/`params`/`percent_decode`/`hex_escape` (server.rs:769-870): no panics. Indexing is via `get`, malformed escapes are kept literally, and the repeated query key is refused (`repeated_query_key`).
- `page_number` defaulting is documented and clamped per page (`page.min(last)`).
- `day_window_bounds`, `timeframe_param`, `month_param`, `Addressed::parse` (`/bars.json`, `/gaps.json`): malformed or reversed input refuses, `/gaps.json` range truncation is flagged, and the store path is validated before I/O.
- `sweepevidence::Asked::parse`, `operation_audit::parse`, `sweeprun::requested_attempt`: strict key sets, duplicates refused, canonical integers.
- `sweeprun::wire_body`: strict serde decode with duplicate-field refusal. Numeric knobs refuse zero or negative values where meaningful.
- `recovery::one_parameter` refuses repeats. `recovery::plan` cross-checks envelope against payload (the contrast for P1-02-01).
- `render::query_value` and `link_params` percent-encode values, so `M&M` and `GVT&D` round-trip.
- `/bars/window.json` offset arithmetic (bars.rs `seek_page`, `page_of`) is saturating, with no overflow or panic at `usize::MAX`.

---
## Source: 03-api-security.md

# P1-03 — crates/api HTTP server security (static assets, bind, Host/Origin, headers, parsing, writes)

Scope read: `crates/api/src/server.rs` (route table 16117-16410, `admitted` 15921-15940, request bounds 15942-16070, `never_framed`, `same_origin_writes_only` / `cross_origin_refusal` / `local_host_authority` / `origin_matches_host` 16537-16720, `LimitedListener` / `HeadDeadline` 16760-17075, `Command::parse` / `loopback_serve_addr`), `crates/api/src/assets.rs` (decode, segments, resolve, respond), `render::escape` / `json_string`, `folder.rs`, `recovery.rs` page/recent, `bars::open` -> `store::path::check_segment`. hyper 1.11.0 `proto/h1/conn.rs` and `dispatch.rs` were fetched to confirm the transport behaviour in P1-03-1.

Already recorded and skipped (workspace `hunt-api.md`, `attacksweep.md`): leading-blank-line head-deadline bypass (attacksweep-1, still unfixed at 331b05c: `observe` at server.rs:16880-16895 is unchanged), slow POST body (attacksweep-1b / 06-limits D-1200), repeated keys in a POST body (hunt-api-5 / attacksweep-3), cross-site log rotation (hunt-api-3), cancellation of `/pull/spot` when the client disconnects (hunt-api-1).

### P1-03-1 low `Expect: 100-continue` re-arms the head deadline mid-request: a long POST is killed at 10 s with a stray 408 and its handler future is dropped
- Where: crates/api/src/server.rs:17031-17055 (`poll_write` / `poll_write_vectored` call `rearm`), 16900-16920 (`rearm`), 16866-16897 (`observe`), 17014-17024 (alarm polled in the `Pending` branch of `poll_read`)
- Evidence:
  - `poll_write`: `if matches!(written, std::task::Poll::Ready(Ok(n)) if n > 0) { this.rearm(); }`
  - `rearm`: `HeadState::Delivered => { let deadline = tokio::time::Instant::now() + self.timeout; self.state = HeadState::Awaiting { deadline, progress: 0, partial: false }; ...}` and `// A pipelined head already under way keeps its own clock. HeadState::Awaiting { .. } => {}`
  - `poll_read`, Pending arm: `if let HeadState::Awaiting { partial, .. } = this.state && this.alarm.as_mut().poll(cx).is_ready() { return Poll::Ready(Err(this.expire(partial))); }`. `expire` does `self.io.try_write(HEAD_TIMEOUT_REPLY)` when `partial`.
  - hyper 1.11.0 `conn.rs:410-414`: `if let Writing::Init = self.state.writing { trace!("automatically sending 100 Continue"); let cont = b"HTTP/1.1 100 Continue\r\n\r\n"; self.io.headers_buf().extend_from_slice(cont); }`. That write goes through `HeadDeadline::poll_write`.
  - hyper `conn.rs:435-445` and `491-499`: while the handler runs (reading = KeepAlive, writing = Init, so `is_mid_message`), `poll_read_keep_alive` -> `mid_message_detect_eof` -> `force_io_read`, which polls this io's read. An `Err` from it closes the connection (`self.state.close()`).
- Why it is wrong: The deadline is meant to cover only the request head (doc at 16858-16862: "no deadline runs until the response is written"). With `Expect: 100-continue`, hyper writes the interim `100 Continue` after the head is delivered. `rearm` treats that write as "the response was written" and starts a new 10 s head clock. The body bytes then pass through `observe`, which sets `partial = true` (a form body has no blank line). From then on hyper's EOF probe polls the read while the handler runs, the alarm fires at 10 s, and the read returns `TimedOut`. hyper closes the connection and drops the handler future in the middle of the work. The client also gets an unsolicited `408 ... the request head did not arrive in time` after its `100 Continue`, which is false: the head and the body both arrived. `/pull/spot` runs the whole vendor pull inside the request (hunt-api-1), and `/universe/resolve` opens about 300 sockets, so both easily run past 10 s. This hits exactly the non-browser client the admission doc says must be supported (`a bare curl must opt in by sending a matching Origin`). curl adds `Expect: 100-continue` by itself above a body-size threshold (1024 bytes in older releases; UNVERIFIED for the installed version), and any client may send it. Lesser effect: if the handler finishes in time, `rearm` sees `Awaiting` and keeps the old clock. The next keep-alive request then gets only the time left on a clock started at the `100 Continue`.
- Repro: Use the existing `serve_limited` test harness with a 400 ms deadline and a router whose POST handler reads `body: String` and then sleeps 1 s before answering. Send `POST /x HTTP/1.1\r\nHost: 127.0.0.1:P\r\nOrigin: http://127.0.0.1:P\r\nContent-Length: 5\r\nExpect: 100-continue\r\n\r\n`, wait for `100 Continue`, then send `a=b&c`. Expected: `200`. Observed (by code path): `HTTP/1.1 100 Continue` followed by `HTTP/1.1 408 Request Timeout`, the connection closed at about 400 ms, and a drop guard in the handler showing the future was dropped. The same request without `Expect` answers `200`.
- Suggested fix: Re-arm only on the first write of a final (non-1xx) response, or simpler: track "response complete" from hyper's side and never move `Delivered` back to `Awaiting` while the current request's body or handler is outstanding. Add the `Expect: 100-continue` case to the D-1200 slow-client tests.

### P1-03-2 low A listener on port 80 refuses every browser request: `Host` must carry an explicit port, and browsers omit the default one
- Where: crates/api/src/server.rs:16684-16705 (`local_host_authority`), 16708-16718 (`origin_matches_host`); `loopback_serve_addr` 177-188 accepts any loopback port
- Evidence:
  - `if raw.contains('@') || authority.port_u16() != Some(local_addr.port()) { return false; }`
  - `cross_origin_refusal` applies this to EVERY method: `Ok(Some(host)) => return Some(cross_origin_sentence(method, "Host", host)),`
  - `loopback_serve_addr` refuses only non-loopback IPs, so `api serve 127.0.0.1:80` is accepted.
- Why it is wrong: User agents omit the default port from `Host` and `Origin` (RFC 9110 §7.2: a default port is elided). A browser at `http://127.0.0.1/` or `http://localhost/` sends `Host: 127.0.0.1` or `Host: localhost`, so `port_u16()` is `None`, which is not `Some(80)`. Every request, including `GET /` and every read, is answered `403 REFUSED ... Host says this one came from somewhere else: 127.0.0.1`. The CLI accepts the address and serves, so the server is up and unusable from a browser. The 403 body blames a cross-origin page for what is the operator's own browser. The same applies to any listener on the scheme's default port.
- Repro: Call `local_host_authority("127.0.0.1", "127.0.0.1:80".parse().unwrap())` and `local_host_authority("localhost", ...)`. Both return `false`, while `"127.0.0.1:80"` returns `true`. End to end: `api serve 127.0.0.1:80` (with the capability to bind), then open `http://127.0.0.1/` in a browser, and every request gets a 403.
- Suggested fix: Treat a missing port as 80 for this plain-HTTP listener (`authority.port_u16().unwrap_or(80) == local_addr.port()`). Apply the same default in `origin_matches_host`, by comparing parsed host and effective port rather than raw strings. Alternatively, refuse port 80 in `loopback_serve_addr` with a sentence explaining why.

## Checked and holding (no finding)
- Bind: `DEFAULT_ADDR` is 127.0.0.1:8080. `loopback_serve_addr` refuses non-loopback IPs and IPv4-mapped IPv6. The production router is built with the bound `local_addr` (server.rs:18544).
- Static assets: the path is percent-decoded once. Bad escapes, NUL and non-UTF-8 are refused. `\` is treated as a separator. Each segment must be exactly one `Component::Normal`. The root is canonicalised at startup, and the target is canonicalised and checked with `starts_with(root)`, so a symlink that escapes is refused. `content_type` never sniffs. Only GET and HEAD reach the fallback.
- Store paths from query parameters: `bars::open` -> `StorePath` -> `check_segment` allows only `[A-Za-z0-9_&-]`, length-capped, and refuses `.`/`..`. Identity parameters are hex-only.
- Admission: Host is checked on every method (DNS rebinding). Writes need an exact `http://` Origin match plus `Sec-Fetch-Site` absent or `same-origin`. Forwarded headers are refused on writes, and repeated security headers are refused. Journaled reads refuse cross-site requests. Admission sits outside the audit journal. There is no CORS header anywhere.
- Headers: `X-Frame-Options: DENY`, CSP `frame-ancestors 'none'` and `nosniff` on every response (outermost layer). HTML pages and the refusal pages I sampled (`/bars`, `/store` filter, `/instruments`, `/pull/recovery`, the assets 503 pages) escape every interpolated value with `render::escape` (which covers `&<>"'`).
- Request bounds: 8 KiB target, 64 KiB headers, repeated query keys refused, 8 KiB form bodies (with a route-level member-form cap). There is no unbounded `to_bytes` on a request body.
- No GET handler I inspected (`verify.json`, `folder.json`, `recovery` page/recent, `autopilot.json`, `masters` page, `top.json`, the launch metadata routes) writes state or starts work, apart from the request log already recorded as hunt-api-3.

---
## Source: 04-api-dos.md

# P1-04 — crates/api resource exhaustion (audit at 331b05c)

I checked these and did not record them again, because they are already in docs/11-findings.md, the audit-20261003 workspace notes or docs/06-limits.md:
- head slowloris (probeapi-1, fixed D-1200)
- the leading-`\n\n` bypass (attacksweep-1, still present at 331b05c: `observe` at server.rs `(1 | 2, b'\n') => Delivered` with `(_, b'\n') => 1`)
- the body stall (attacksweep-1b / 06-limits D-1200)
- the Ctrl-C wait on spawn_blocking (hunt-api-2)
- client-disconnect cancelling a hand pull (hunt-api-1)
- log rotation by cross-site GETs (hunt-api-3)
- the autopilot tick and `land_spot` running inline (o1surface2-2/-3)
- the operation-audit journal growing one file per audited request (06-limits D-1445)

Also checked and holding:
- the 256-connection cap and slot release on drop
- target, header and repeated-key caps
- `DefaultBodyLimit` 8 KiB, and the member forms' own limit
- `detail::run` / `run_calendar` / `run_store_read`: try-take permits, held inside the blocking closure, so a disconnect cannot leak a slot
- the sweep `ADMISSION` mutex and lease, which run inside `detail::run`
- pullrun spawns: one per feed, behind the `site.run` claim
- the expression-search session cache, capped at 8
- the single-slot JSON caches (`try_lock`/`lock` + `map_err`, no unwrap)
- every production lock: poison is handled with `into_inner` or a named refusal

### P1-04-01 medium `/bars/window.json` with a non-`ts` sort or `extremes=1` reads and holds up to ~1.9 M bars per request, inline on a Tokio worker, with no admission. Any cross-site page can trigger it.
- Where: crates/api/src/server.rs:3332-3375 (`bars_window_json`), route at server.rs:16151; crates/api/src/bars.rs:535, 1030-1070
- Evidence:
  - The handler is a plain `async fn` that calls the synchronous reader directly. It uses no `crate::detail::run*`, no `spawn_blocking` and no permit:
    ```
    async fn bars_window_json(axum::extract::State(site): ..., uri: axum::http::Uri) -> ... {
        ...
        let window = match bars::window(&site.store_root, asked.vendor, ..., asked.sort, asked.desc,
                                        asked.offset, asked.limit, asked.want_extremes) {
    ```
  - bars.rs:535 `pub const MAX_WINDOW_MONTHS: usize = 240;`
  - bars.rs:1040 comment: "A 240-month window is roughly 1.9 million bars", and "THE BARS ARE STILL ALL READ ... the read is O(bars)".
  - 06-limits W1-api5-4: "still reads every bar in its month range (one `read_record` per bar) ... and holds them all in memory: O(n) reads and O(n) memory".
  - Every other store-scanning GET was moved off the async workers behind a try-take pool that answers 429 (D-1443, D-1508):
    - detail.rs:36 `MAX_CALENDAR_CONCURRENT: usize = 8`
    - detail.rs:45 `MAX_STORE_READ_CONCURRENT: usize = 8`
    - 06-limits: "`/folder.json` ... and `/indexmap.json` ... run in a third pool ... a ninth is answered 429".
  - `/bars/window.json` is not in that set. It is also not in `operation_audit::AUDITED` (operation_audit.rs:59-81), so `journaled_read_refusal` returns early. `cross_origin_refusal` admits any GET whose `Host` names the loopback listener (server.rs:16604-16608: "A LOCAL READ IS NOT REFUSED HERE"). The default port is fixed: server.rs:33 `DEFAULT_ADDR ... 8080`.
- Why it is wrong:
  - The O(n) cost is documented. What 06-limits does not state is that the work is unbounded in concurrency and runs on the async runtime itself.
  - Each such request does about 1.9 M positional reads and holds a `Vec<WindowBar>`. A `WindowBar` is 7×i64 + 2×Option<i64> + 2×&str ≈ 120 B, so a request holds ≈ 228 MB. That figure is argued, not measured.
  - All of this happens on one of the `#[tokio::main]` workers, one per core.
  - A handful of concurrent requests therefore pins every worker. Then every other route stalls, including the head-deadline timers that need a worker to fire. A few more multiply the resident memory.
  - The trigger can be a local process, or a no-cors `fetch` from any site open in the operator's browser. hunt-api-3 already proved such cross-site GETs reach the server.
- Repro: run `api serve` on a store holding ~20 years of 1-minute NIFTY bars. From another origin, fire 4–6 concurrent `fetch('http://127.0.0.1:8080/bars/window.json?vendor=…&exchange=NSE&segment=…&symbol=NIFTY&timeframe=1m&from=2006-01&to=2025-12&sort=close&extremes=1',{mode:'no-cors'})`. While they run, `GET /health` gets no answer until they finish, and RSS grows by roughly 200 MB+ per request. Test sketch: give a test runtime 2 workers, start 2 such window requests over a large fixture, and assert `/health` answers within 1 s. That assertion fails today.
- Suggested fix: run `bars::window` through `detail::run_store_read`, or its own try-take pool, so saturation answers 429 the way `/folder.json` and `/indexmap.json` do. State the per-request memory bound in 06-limits W1-api5-4.

### P1-04-02 low `/logs.json` and `/logs` scan up to 8 MiB of NDJSON per request, inline on a Tokio worker, with no admission. A `target=` filter that matches nothing forces the full scan, and the routes answer cross-site GETs.
- Where: crates/api/src/logs.rs:144, 164-200, 325-336, 495-536; routes at server.rs:16251-16252
- Evidence:
  - logs.rs:51 `pub const SCAN_BYTES: u64 = 4 * 1024 * 1024;` and logs.rs:144 `query.max_scan_bytes = SCAN_BYTES;`.
  - The user controls `target`: `let target = crate::server::param(raw, "target"); ... query = query.from_target(target.clone());`.
  - logs.rs:330-336 `both_halves` walks two directories, synchronously, inside the `async fn` handler:
    ```
    let served = telemetry::tail(dir, telemetry::DEFAULT_KEEP_FILES, query);
    let ran = cli_dir.map_or_else(telemetry::Tail::default, |d| {
        telemetry::tail(d, telemetry::DEFAULT_KEEP_FILES, query)
    });
    ```
  - `telemetry::tail` → `walked` → `walk_back` does `std::fs::File::open` / reads (telemetry/src/tail.rs:366-450). The walk stops only when `limit` matches are found or the scan budget is spent. A target nothing logs never fills `limit`, so it reads the full 4 MiB of each half and parses every line.
  - Neither route uses `detail::run*` or `spawn_blocking`. Neither is in `AUDITED`, so cross-site GETs are admitted (server.rs:16604-16608).
- Why it is wrong:
  - Each request is bounded at 8 MiB of disk reads plus line parsing.
  - The work runs on the async workers, and nothing caps how many requests do it at once.
  - A few concurrent `?target=x` polls therefore occupy every Tokio worker and stall all other routes and the head-deadline timers.
  - That is the same class D-1443/D-1508 fixed for `/calendar.json`, `/gaps.json`, `/folder.json` and `/indexmap.json`. o1surface2 listed `/logs(.json)` only as "bounded" and never as running inline.
- Repro: with a full log (64 MiB across 8 files, which hunt-api-3 shows any page can produce), loop N = #cores concurrent `GET /logs.json?target=nomatch&limit=200`. Measure `GET /health` latency before and during the loop. Each logs request's `bytes_read` in the JSON answer is ≈ 8,388,608, which confirms the full scan.
- Suggested fix: run `both_halves` through `detail::run_store_read`, or a dedicated small pool, and answer 429 when it is saturated.

### P1-04-03 low `POST /universe/resolve` has no concurrency guard. Every press starts its own sequential crawl of ~300 third-party documents, each allowed 30 s.
- Where: crates/api/src/server.rs:32176-32285 (`universe_resolve`), route at server.rs:16210; crates/pull/src/resolve.rs:270-293, 489, 540-583
- Evidence:
  - The handler builds a fresh client and awaits the crawl inline. It takes no lock, slot or permit:
    ```
    let source = match pull::resolve::HttpDocuments::new() { ... };
    ...
    let snap = pull::resolve::crawl(&source, INDEX_HOST, today, &feed, ..., &master).await;
    ```
  - Its own doc says: "this route opens **300 sockets** to a third party". resolve.rs:270 also gives the count: "4 + 2n for n indices, which is 300".
  - resolve.rs:489 `pub const DOCUMENT_TIMEOUT_SECS: u64 = 30;`, used as `.timeout(...)` with `.connect_timeout(10 s)`.
  - `grep -n 'static\|Mutex\|Semaphore' crates/pull/src/resolve.rs` finds no guard. By contrast, the sibling masters refresh is serialised by `static REFRESH: tokio::sync::Mutex<()>` (mastersrun.rs:40), and pulls by the `site.run` claim.
- Why it is wrong:
  - Two presses, or a double-click, or a local script, each run a full crawl against niftyindices.com in parallel. Each holds a connection slot and an HTTP client for up to ~300 × 30 s against a host the code itself records as bot-filtering and hanging.
  - Nothing bounds how many crawls run at once.
  - That multiplies third-party load, and with it the bot-check failures the crawl is designed to avoid. It also consumes connection slots.
- Repro: send two `POST /universe/resolve` requests back to back with valid same-origin headers (`Origin: http://127.0.0.1:8080`, `Sec-Fetch-Site: same-origin`). Both run, and both answer after their own full crawl. No 409 is ever returned. Test sketch: replace `INDEX_HOST` with a local stub that counts concurrent GETs, fire two presses, and observe a concurrency of 2.
- Suggested fix: guard the handler the way `mastersrun::REFRESH` does, with a process-wide `tokio::sync::Mutex::try_lock`, and answer 409 "a resolution is already running" on contention.

### P1-04-04 low `POST /pull/recovery` runs its O(history) journal replays on the blocking pool before the run-slot check and with no admission, so even a request that will get 409 pays the full replay. `prepare_only` never takes a slot at all.
- Where: crates/api/src/recovery.rs:534 (`prepare_reply`), 588-640 (`start`), 344-372 (`preflight_plan`)
- Evidence:
  - recovery.rs:614-632 shows the order:
    ```
    let preflight = tokio::task::spawn_blocking(move || {
        preflight_submission(&preflight_site, &asked)?;
    ...
    if let Err(why) = claim(&site, Some((id, true))) {
        return (StatusCode::CONFLICT, ...
    ```
  - recovery.rs:534 `let prepared = tokio::task::spawn_blocking(move || prepare_successor(&site, &asked))`. The `prepare_only` branch claims nothing: "This branch never claims a run slot".
  - `preflight_plan` calls `active_history(site)` and `recovery_journal::snapshot` of the plan and of `attempts.bin`. 06-limits states each of these "replays the whole file ... and nothing compacts one". Neither path uses `detail::run`'s permit.
- Why it is wrong:
  - The replay cost grows with all recovery history on the store, as 06-limits states.
  - Every press queues an uncapped blocking task for that replay, even while a pull or recovery already owns the slot and the answer will be 409.
  - Repeated presses, or a local client, can therefore fill Tokio's blocking pool. The detail, calendar and store-read pools spawn into that same pool, so their already-admitted work queues behind the replays.
  - 06-limits documents the per-press cost. It does not document that the cost is paid before the busy check and without admission.
- Repro: start a pull run (`POST /pull/run`) so `site.run` is held. Then loop `POST /pull/recovery` with a valid plan body and same-origin headers. Every answer is 409, yet each one first performs the three journal replays. Instrument `recovery_journal::replay`, or count the reads, to see one full replay set per refused press.
- Suggested fix:
  - Check `site.run` (a cheap `claim` probe) before `preflight_submission`.
  - Route the preflight and `prepare_successor` through `detail::run` (or `run_store_read`) so they share a capped pool and answer 429 when it is full.

---
## Source: 05-web-xss.md

# P1-05 web/ front-end security (XSS, URL/path injection)

NO FINDINGS (new). The one real sink in web/ is already recorded.

## Already recorded (skipped as duplicate)
- `web/masters.js:13-17, 34-37, 56-61, 98, 104-114, 117`: `innerHTML` with unescaped API/vendor strings (`a.detail`, `m.refusal`, `d.error`, `x.symbol`, `${e}`). Recorded as **webcontract-2** in `/mnt/project-files/audit-20261003-workspace/webcontract.md`. The recorded description is correct. Its line citations are slightly off (`d.error` is at line 98, `x.symbol` at 104/110), but the fix it implies (textContent/DOM building) covers every sink.
- Missing CSP `script-src`: noted inside webcontract-2.

## What I checked, and why it holds
- `{@html}`, `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`, `eval`, `new Function`, `postMessage`/`message` listeners, `srcdoc`, `<iframe>`, `setHTML`, `createContextualFragment`. Searched across `web/src`, `web/saved-backtest`, `web/sweep-readiness/*.js|index.html`, `web/policy-guide/index.html`, `web/typeahead.js` and `web/masters.js`. The only live hits are in masters.js (above). The design/*.html files mention innerHTML only in comments and build with DOM methods.
- `target="_blank"`: two external links, `markets/+page.svelte:2492` and `IndexStopChart.svelte:31`. Both carry `rel="noreferrer"`, which implies noopener.
- Data-built `href`s:
  - `typeahead.js:106-117`: same-origin check on `new URL`.
  - `runtime-inspection.js:16-29` (`savedResultsUrl`): exact http, loopback host, `/backtest` path, canonical query.
  - `indexStopQualificationHref`: hex-validated.
  - `BooleanQualifiedSearch` `planned_campaign`: hex-validated at `boolean-qualified-search.js:54`.
  - `InvocationAudit` `record.invocation`: digit-validated in `invocation-audit.js:21`.
  - `saved-backtest/App.svelte` `shareLink`: URLSearchParams over validated fields.
  - `db/+page.svelte:7002`: a blob URL.
  - `+layout.svelte` NAV: static.
  - `sweep-readiness/index.html` evidence links: from a locally generated `evidence.js`, not network data.
- Fetch paths built from URL state or API data:
  - Every interpolated value is either `encodeURIComponent`-ed, built with `URLSearchParams`, or validated as digits. This covers `/backtest/run.json?attempt=` at `backtest/+page.svelte:2434`, where `sweepSubmission` requires `/^[1-9]\d{0,19}$/`.
  - The `/folder.json` `segment` value is an internal constant.
  - `urlstate.js` decode output only selects page state and goes through `encodeURIComponent` before reaching fetches.
  - The `saved-backtest/selection.js` allow-list rejects unknown or repeated keys and validates hex and integers.
- `ingest/+page.svelte:5694`: `DOMParser` HTML is read only through `textContent` and never inserted.
- `policy-guide/build-guide.mjs:57`: escapes `<` as `<` in the JSON it inlines into `<script>`. The page then renders with `textContent`.
- `saved-backtest/viewer.rs`: static path admission (`safe_path`, canonicalize, no symlinks, size bound) has tests for `..`, `%2e%2e` and `%2f`.
- CSV export (`db/+page.svelte:6907-6925`) has no formula-prefix neutralisation. Not recorded as a finding: exported text fields are store keys and enums, so no evidence of attacker-controlled leading `=`/`+`/`@`.

---
## Source: 06-web-contract.md

# P1-06 web/src <-> crates/api contract (audit at 331b05c)

Prior items not repeated here: webcontract-1..6 in /mnt/project-files/audit-20261003-workspace/webcontract.md. webcontract-1 (`/vocab.json` has no `commit_digest`) is **still open** at 331b05c: `grep commit_digest crates/api/src/server.rs` finds nothing, and web/src/routes/backtest/+page.svelte:2104 still reads the field.

### P1-06-01 medium /autopilot drops `journal_error`, so a journal that cannot be written is invisible, and the page then points the operator to that journal as the durable record
- Where: web/src/routes/autopilot/+page.svelte:536-648 (`readState`), 2179-2183, 2201-2203; crates/api/src/autopilot.rs:1722-1728, 3297-3300, 3482
- Evidence:
  - Server, autopilot.rs:3482: `let journal_error = site.journal().append(&record).err();`, published at 3297-3300 as `status.journal_error = journal_error;`, and emitted at 1722: `r#"{{"state":{},"why":{},"pull_active":{},"cursor":{},"journal":{},"journal_error":{},...`
  - The doc comment at autopilot.rs:1575-1580 says: "**A path is not a proof.** `journal` said where the file is and every page read it as evidence the runs were being recorded; the append's answer was discarded ... This is that answer, and `/autopilot.json` carries it as `journal_error`."
  - Web: `readState` returns `journal: str(raw.journal)` and has no read of `raw.journal_error`. `grep -rn journal_error web/src` returns nothing.
  - The page still tells the operator: "the durable record is `<code>{ap.journal ?? JOURNAL}</code>`, rendered at Audit. A failure here and not there was never written down, and that is a defect in the journal, not in this page."
- Why it is wrong: The server added this field to make a failed journal append visible (CLAUDE.md §4). The only browser consumer throws it away. When appends fail (full disk, permissions), the page shows the journal path as the durable record and says nothing is wrong.
- Repro: Make the journal unwritable (for example, chmod 0444 `~/.brutex/store/audit/pull.journal`) and let one autopilot tick run. `GET /autopilot.json` then carries a non-empty `"journal_error":"..."`. /autopilot shows no trace of it, and the bay note still points to the journal as the record.
- Suggested fix: Read `journal_error` in `readState`. When it is non-empty, render it as a loud banner next to the journal path, and stop calling the journal "the durable record" in that state.

### P1-06-02 medium Live sweep progress dies partway through a validated multi-rung sweep, because `/logs.json?limit=200&run=` cannot hold the attempt's start marker
- Where: web/src/routes/backtest/+page.svelte:1970-1972; web/src/lib/live-progress.ts:335-343; crates/api/src/logs.rs:44, 131-134; crates/cli/src/lib.rs:18345, 18565-18570, 18644-18652 (and the other `note_attempt` sites)
- Evidence:
  - Web: ``ask_(`/logs.json?limit=200&run=${encodeURIComponent(attempt)}`, ...)``
  - Server: `pub const PAGE_LIMIT: usize = 200;` and `.unwrap_or(50).clamp(1, PAGE_LIMIT)`. The limit applies after the run filter (telemetry tail.rs:595-633, `out.records.len() >= limit`), so the reply is the newest 200 events for this attempt.
  - Fold: `const markers = matching.filter((record) => record.message === START); if (markers.length !== 1) { return refused(... 'The event feed does not contain this attempt\'s start marker.' ...`
  - Events per rung under the attempt's run key, all through `note_attempt` and therefore all matching `run=`: 1 `rung sweeping`, 1 `exit grid entered`, up to 10 `exit grid progress` (`GRID_PROGRESS_STEPS: usize = 10`), 1 `exit grid finished`, 6 `validation stage entered/finished` ("two events per stage, six per rung"), 16 `validation fold finished` ("Two shapes of eight folds is sixteen events a rung"), and 1 `rung finished`. That is about 36 per rung, and about 288 for the 8 rungs in `EVERY_RUNG`. The plus-one is the start marker.
- Why it is wrong: After about 5.5 validated rungs, more than 199 events newer than `sweep attempt started` exist under the attempt's key. The marker falls out of the only window the page can ask for (200 is the server maximum), and `reduceLiveProgress` refuses on every poll for the rest of the run. The progress view fails exactly on the long runs it exists for. The refusal is loud, but it is wrong: the feed is healthy and the browser asked for too little. No `target`, `before` or cursor parameter exists to page further back.
- Repro: Launch a browser sweep with all 8 rungs and `validate` on (the server default). Poll `/logs.json?limit=200&run=<attempt>` once rung 6 is validating. The `records` array no longer contains `"message":"sweep attempt started"`, and the page shows "The event feed does not contain this attempt's start marker."
- Suggested fix: Have the fold carry the start context forward once it has seen the marker, instead of re-requiring it in each window. Alternatively, add a server-side `message=`/`target=` filter or a cursor so the page asks only for the lifecycle boundaries it folds. Either way, size the window to the per-rung event count.

### P1-06-03 low /audit "load older" pages are offsets from a moving end, so while the journal grows the merged list silently skips records (or repeats them)
- Where: web/src/routes/audit/+page.svelte:381, 421-433, 696; crates/api/src/audit_json.rs:283-292; crates/api/src/audit.rs:1273-1277
- Evidence:
  - Server pages from the newest end: `let skip = u64::try_from(page).unwrap_or(0).saturating_mul(per_page);` and in `read_page`: `let end = records.saturating_sub(skip); let start = end.saturating_sub(take);` with `MAX_PAGE_RECORDS = 200`.
  - Web refreshes page 0 on every poll (`const got = await read(ticket, 0, feed); ... payload = body;`). It fetches older pages once and keeps them (`older = [...older, ...got.body.runs]; pagesHeld += 1;`), then concatenates: `const runs = $derived([...(payload?.runs ?? []), ...older]);`. Nothing reconciles the two by `ordinal`.
- Why it is wrong: Page 0 covers `[N-200, N)` and older page 1 covers `[N-400, N-200)`. After g new records, page 0 covers `[N+g-200, N+g)`, and records `[N-200, N-200+g)` are on neither page. The list then shows "holding 2 page(s) of M · newest first" with a silent hole. If page 1 is fetched after growth that page 0 has not yet picked up, the two overlap and the same `ordinal` appears twice in `runs`, which is used as the keyed-`each` key at line 1537. This happens exactly when the page polls fast (`hot`), that is, while a pull is appending.
- Repro: With more than 400 journal records and a pull running, click "load older" once, wait for a few appends and polls, then compare the ordinals shown against the contiguous range. A run of g ordinals just below page 0's oldest row is missing.
- Suggested fix: Page by an absolute ordinal cursor (`?before=<ordinal>`) instead of an offset from the moving end. Or, on the client, re-fetch older pages against the same `records` total, dedupe by `ordinal`, and flag any gap.

## Checked, no new defect
- `/bars.json` and `/bars/window.json`: client params (`feed, exchange, segment, symbol, contract, timeframe, month|from|to, sort, dir, offset, limit, extremes`) match `Addressed::parse`, `day_window_bounds` and `bars::window`. `DAY_BUDGET` 512 is at most `MAX_WINDOW_LIMIT` 1000. The day bound is inclusive via the next midnight. 206 `{bars,faults}` is handled on /db, /markets and terminal.
- `/backtest/run` JSON body keys: all 15 knob names plus `feed/underlying/from_*/to_*/rungs` match `sweeprun.rs` 527-541 and 658-687.
- `/pull/run` (`started`, `why`) and `/pull/run.json` (`running`, `feeds[].legs/legsDone/credentialDead/skipped/lastError`) match pullrun.rs:259-317 and server.rs:10942-10948.
- `/autopilot.json` states and `target`/`now`/`failures` shape match autopilot.rs:1722-1770. `/autopilot/control` `action/accepted/why/status` match autopilot.rs:3931-3935.
- `/frontier.json` paging (`identity, page, limit`, `page_complete`, `total_count`): integer fields are guarded by `Number.isSafeInteger`. Mask words are strings decoded with BigInt.
- `/logs.json` `run=` token: an exact decimal string up to u64 (live-progress.ts `exactToken`). The server parses u64.
- `/universe/resolve` (`ok/why/publishable`), `/indexmap.json` error body, `/masters/status.json` `restart_required`, `/masters/refresh` 502 fields (mapping page), and `/gaps.json` (`truncated`, `calendar.stale/covers_span`, per-month `absent_file/evidence_error/invalid_timestamps`) are all read.
- `/store.json` HEAD census headers: the note format `"{wire}: N month(s), M row(s), generation G"` (census.rs:265) matches feed-summary.js's regex. The `note_alphabet` ASCII folding leaves the `"{wire}: UNAVAILABLE"` prefix intact.
- `/backtest.json` `underlying`/`equity_note`, read by charge-scope.js.

---
## Source: 07-ci-gate1.md

# P1-07: can CI gates 1, 1b, 1c, 1d, 1e (and the adjacent 1g / 2 / 13-layer-3 key checks) be bypassed?

Repo: /home/claude/brutex-audit at 331b05c. Method: static reading. I built `.github/source_scan.rs` with `rustc --edition=2024 -D warnings` in scratch. I extracted the gate bodies verbatim from `ci.yml` (gate 1: lines 68-228; gate 2: lines 1715-1830) and ran them in throwaway `git init` repos under `scratchpad/p1/p107/`. The only cargo command was one `cargo metadata --no-deps --offline` in a scratch repo. No cargo build, test or clippy was run.

These bypasses are already recorded, so they are not repeated below:
- rustonly2.md: rustonly2-1 to -9 (uncompiled `.github/*.rs` counted as roots, `.gitignore`/`.gitattributes` content, gate 1e not shadowing `sh` or absolute paths, gate 1g env regex, gate 2 via a build-dependency or `rustc-link-arg`).
- hunt-ci.md: hunt-ci-1 (the PR's own `ci.yml` certifies itself).

---

### P1-07-01 medium A quoted, dotted or inline `build` key renames a build script past gate 2 and gate 13 layer 3
- Where: .github/workflows/ci.yml:1810 (gate 2), :2439 (gate 13 layer 3), :1734 and :2384/:2407 (filename-only root discovery)
- Evidence:
  - Gate 2 builds a `path:line:text` corpus of every manifest. It refuses only:
    `| grep -E ':[[:space:]]*(build|links)[[:space:]]*=' || true; } )` (:1810)
  - Gate 13 layer 3 uses the same regex: `bk=$( { grep -E ':[[:space:]]*(build|links)[[:space:]]*=' "$decl" || true; } )` (:2439)
  - Otherwise, both gates find build scripts only by filename: `roots=$(git ls-files '*build.rs')` (:1734) and `awk -F/ '$NF == "build.rs"'` (:2384, :2407).
  - The gate's own comment says the key is the fix: "So the KEY is refused, not just the name … One `build =` line defeats both" (:1773-1777).
- Why it is wrong: TOML has other spellings of the same key:
  - quoted, `"build" = "gen.rs"` under `[package]`
  - dotted, `package.build = "gen.rs"` at top level
  - inline, `package = { …, build = "gen.rs" }`
  
  None of them puts `build` right after optional spaces at the start of a line, so neither regex matches. Cargo then runs `gen.rs` as the build script. Gate 2 never scans it, because its roots are `*build.rs` only, and layer 3 never counts it. The scanner's own TOML walker does read the key, so gate 1's orphan check treats `gen.rs` as a compiled root and passes too. The same holds for `"links" = …`.
- Repro (run):
  - `crates/a/Cargo.toml` contains `[package]\nname = "a"\nversion = "0.1.0"\nedition = "2024"\n"build" = "gen.rs"`.
  - `crates/a/gen.rs` contains `fn main() { let _ = std::process::Command::new("sh").arg("-c").arg("echo hi").status(); }`.
  - Results:
    - Gate 1 → `OK … every .rs outside web/ is compiled.` rc=0
    - Gate 2 → `none — no tracked build.rs` / `OK — no build script invokes an external process, and no manifest names one under another filename.` rc=0
    - `cargo metadata --no-deps` → `"kind":["custom-build"],"name":"build-script-gen","src_path":".../crates/a/gen.rs"`
    - `source-scan toml` lists `package.build = "gen.rs"` for all three spellings. The `:[[:space:]]*(build|links)[[:space:]]*=` grep matches none of them.
  - Gate 1e does not shadow `sh` (rustonly2-4), so it stays green as well.
- Suggested fix: take build roots from the scanner's TOML leaves, refusing any `package.build` or `package.links` path whatever its spelling, instead of grepping lines. Gate 2 should also scan every root that `compiled_roots` gives for `package.build`, not just `*build.rs`.

### P1-07-02 medium A joined credential path in crates/pull passes both gate 1c and gate 1d (Rust escapes, or an environment word outside the ten)
- Where: .github/workflows/ci.yml:574 (gate 1c grep), :1687-1692 (gate 1d word extraction); docs/06-limits.md:822-827; docs/04-invariants.md:930-931
- Evidence:
  - Gate 1c greps the raw bytes: `xargs -0 -r grep -niE "/[A-Za-z0-9_-][A-Za-z0-9_.-]*/(${envs})/"`.
  - Gate 1d keeps a decoded literal only when the WHOLE literal is segment-shaped: `grep -E '^[a-z0-9_-][a-z0-9_-]{0,31}$' "$work/values"` and `grep -ohE '\\"[a-z0-9_-][a-z0-9_-]{0,31}\\"'`.
  - 06-limits §18 names gate 1c's two blind spots ("does not catch a bare segment, and it does not catch a joined path whose environment segment is outside its ten-word alphabet"). It then says "**Gate 1d is the compensating check, and only for `crates/pull`.**"
- Why it is wrong:
  - Gate 1d never splits a literal on `/`. A literal that holds a whole path is not "segment-shaped", so its segments are never compared with the allowlist. 1d therefore covers only the bare-segment gap, not the joined-path gap that §18 says it compensates for.
  - Separately, gate 1c reads source bytes, not decoded literals. One escape in a Rust string hides even a standard environment word, although `source_scan` already decodes escapes (`unescape`, source_scan.rs:61, called at :265).
  - So §8's "no literal parameter path appears in any tracked file" has no working check for these spellings inside the one crate the law singles out.
- Repro (run): a file in crates/pull holding
  ```
  pub const A: &str = "/acmeorg/prd/groww/access-token";
  pub const B: &str = "/acmeorg/\x70rod/groww/access-token";
  pub const C: &str = "/acmeorg/pr\u{6f}d/groww/access-token";
  pub const D: &str = "/acmeorg/prod/groww/access-token";
  ```
  - Gate 1c's grep matches only line 4 (D).
  - `source-scan strings` decodes all four to `/acmeorg/p…d/groww/access-token`. Gate 1d's two word greps return nothing.
  - So A, B and C pass both gates. `acmeorg` and `prd` are undeclared and would be refused if written as bare literals.
- Suggested fix: run gate 1c's pattern over `source-scan strings` output (the decoded literals) as well as over raw bytes. In gate 1d, split every decoded literal under crates/pull on `/` and check each segment-shaped piece against the allowlist. Or correct §18 so it no longer claims that 1d covers joined paths.

### P1-07-03 low A newline in a tracked file name defeats gate 1's extension and name checks (and gate 1b)
- Where: .github/workflows/ci.yml:155 and :178 (gate 1); :391-392 (gate 1b)
- Evidence:
  - The listing is read NUL-safe (`git ls-files -s -z`, :135). Each name is then matched with a line-oriented grep:
    - `if grep -Eq "^(${any_depth_names})$" <<< "$base"; then continue; fi` (:155)
    - `if ! grep -Eq "^(${list})$" <<< "$ext"; then` (:178)
  - grep succeeds if ANY line of the here-string matches.
  - Gate 1b reads `listing=$(git ls-files)` without `-z`. git then C-quotes such names (`"x.yml\nmd"`), so `\.(json|yml)$` never sees the real ending.
- Why it is wrong:
  - `.gitignore<LF>payload.py` has extension `py`. Its first line `.gitignore` matches `any_depth_names`, so the gate takes `continue` and skips every remaining check for that file.
  - `run.py<LF>md` has extension `py<LF>md`. Its second line `md` matches the allowlist.
  - `setup.sh<LF>rs` passes the allowlist the same way. Because the name does not end in `.rs`, the shebang and orphan checks skip it as well.
  - In every case the OK line ("every tracked file is in the allowlist for where it lives … carries no content its name hides") is false.
  - Mode 100644 is still enforced, so nothing becomes directly executable. Impact is a mislabelled foreign-language file in the tree.
- Repro (run): in a scratch repo with a minimal workspace, track `$'.gitignore\npayload.py'` (Python), `$'crates/a/setup.sh\nrs'` (`#!/bin/sh`), `$'crates/a/run.py\nmd'` and `$'config.yml\nmd'`. The verbatim gate 1 body prints `read 7 tracked file(s)` / `OK — every tracked file is in the allowlist…`, rc=0.
- Suggested fix: refuse any tracked path that contains a control character (`[[:cntrl:]]`) at the top of the loop. Or compare with `[[ $ext =~ ^(…)$ ]]` / `case`, which match the whole string rather than per line. Switch gate 1b to `git ls-files -z`.

### P1-07-04 low Gate 1g's "tool config only at the root" check is blind to any path git C-quotes
- Where: .github/workflows/ci.yml:425 and :436-437 (gate 1g)
- Evidence:
  - `listing=$(git ls-files)` (no `-z`, default `core.quotePath`)
  - `hits="$hits$(grep -E '(^|/)(rust-toolchain(\.toml)?|nextest\.toml)$' <<< "$listing" | grep -vxE 'rust-toolchain\.toml|\.config/nextest\.toml' || true)"`
  - The gate's own reason: "only one of each tool file may exist, at the root, because each tool also reads a copy in any ancestor of the directory it runs in" (:432-435).
- Why it is wrong: git prints a path with a non-ASCII byte, a control character, `"` or `\` as a quoted string, for example `"crates/\303\251/rust-toolchain.toml"`. The closing `"` breaks the `$` anchor. A nested `rust-toolchain.toml` carrying `[toolchain] path = "…"` therefore passes gate 1g. `check_keys` reads only the root file, and gate 1 accepts the `.toml`. rustup honours that file for any cargo command run in or below that directory. That is exactly the door D-1105 says gate 1g closes.
- Repro (run): track `crates/é/rust-toolchain.toml` and `crates/a/rust-toolchain.toml`, both `[toolchain]\npath = "/tmp/evil"`. Gate 1g's pipeline reports only `crates/a/rust-toolchain.toml`. The `é` copy is listed as `"crates/\303\251/rust-toolchain.toml"` and is not reported. Gate 1 passes both.
- Suggested fix: build the listing with `git ls-files -z | tr '\0' '\n'`, or with `git -c core.quotePath=false ls-files` plus a refusal of control characters, everywhere a gate greps a listing. That covers gates 1b, 1g and 13 layer 3.

---

Checked and found sound (no finding):
- Uppercase or mixed-case extensions (`.RS`, `.Py`, `.YML`, `.yaml`) are refused outside web/.
- No-extension files and trailing-dot names are refused.
- `LICENSE` and `CODEOWNERS` are matched by full path.
- The mode check refuses symlinks, gitlinks and executables outside web/.
- `web/` is matched by prefix only.
- Gate 1c's file list is NUL-safe.
- Gate 2's token scan catches alias imports (`use std::process as p`, `{Command as C}`), fully qualified `std::process::Command`, `r#Command`, `macro_rules`, `asm!`, `unsafe`, `extern`, `libc` and `nix`.
- The scanner's `dependencies()` covers `build-dependencies`, `build_dependencies`, `target.<cfg>.*`, `patch` and `replace`.
- A BOM before `#!` evades the `.rs` shebang check, but the kernel would not honour that shebang either, and the mode is 100644, so this is not a finding.

---
## Source: 08-ci-gates.md

# P1-08 — CI gates outside the gate-1 family: bypasses and vacuous passes

Audited: `.github/workflows/ci.yml` at 331b05c (gates 7, 8, 9, 9b, 11, 14, 17, 18, 22, 23, coverage, W*, ci-ok), `.github/source_scan.rs`, `.github/mutation_gate.rs`, and cargo-mutants 26.2.0 `src/visit.rs` (fetched from the upstream tag, saved as `scratchpad/p1/visit.rs`). Static reading only. Nothing was run.

### P1-08-01 medium One attribute that compiles to nothing, `#[cfg_attr(miri, mutants::skip)]`, removes a function from gate 18, and no gate looks for it
- Where: `.github/workflows/ci.yml:7437-7525` (mutant-plan), `:7572-7609` (shards). The exclusion logic is cargo-mutants 26.2.0 `src/visit.rs:852-857, 912-933`.
- Evidence (cargo-mutants 26.2.0, `src/visit.rs`):
  ```rust
  fn attrs_excluded(attrs: &[Attribute]) -> bool {
      attrs.iter().any(|attr| attr_is_cfg_test(attr) || attr_is_test(attr) || attr_is_mutants_skip(attr))
  }
  fn attr_is_mutants_skip(attr: &Attribute) -> bool {
      if path_is(attr.path(), &["mutants", "skip"]) { return true; }
      if !path_is(attr.path(), &["cfg_attr"]) { return false; }
      let mut skip = false;
      if let Err(err) = attr.parse_nested_meta(|meta| {
          if path_is(&meta.path, &["mutants", "skip"]) { skip = true; }
          Ok(())
      }) { ... return false; }
      skip
  }
  ```
  `grep -rn "mutants::skip\|mutants::" --include=*.rs --include=*.toml --include=*.yml .` returns nothing, and `source_scan.rs` never mentions `mutants`.
- Why it is wrong: cargo-mutants checks `cfg_attr` by syntax and never evaluates the predicate. `miri` is a well-known cfg (so `unexpected_cfgs` stays quiet under `-D warnings`) and it is false in every CI build, so rustc drops the attribute. No `mutants` dependency is needed and the code compiles clean, yet cargo-mutants generates zero mutants for that function. Gate 18 says it plans "without excluding any case" and reconciles exactly what was listed, so a function nothing lists has no surviving mutant to report. CLAUDE.md §9's "no surviving mutant on touched modules" is defeated by one line. (The repository already relies on this syntax-only reading in the other direction: `crates/cli/src/lib.rs:71` spells `#[cfg(all(test))]` "so cargo-mutants walks the module".)
- Repro: in a PR, change a function body in `crates/engine/src/lib.rs` in a way its tests do not detect, and put `#[cfg_attr(miri, mutants::skip)]` above it. `cargo build`, clippy `-D warnings` and tests stay green. `cargo mutants --list --in-diff changed.diff` lists no mutant for that function. The shards reconcile `assigned == caught ∪ unviable` with the function absent, and ci-ok is green.
- Suggested fix: add a source-scan rule, inside gate 18 or gate 11, that refuses any attribute whose token stream contains `mutants :: skip` (bare or inside `cfg_attr`) anywhere under `crates/`. Also make the plan step fail when a changed fn line produces no mutant and is not a test.

### P1-08-02 medium Gate 8 is vacuous for any bench marked `bench = false` (or given unmet `required-features`); gate 14 checks only the name and `harness`
- Where: `.github/workflows/ci.yml:7647-7661` (gate 8), `:6741-6771` (gate 14 layer 2b)
- Evidence:
  ```
  n=$(git ls-files 'crates/*/benches/*.rs' | wc -l | tr -d ' ')
  if [ "$n" -eq 0 ]; then ... exit 1
  ...
  cargo bench --workspace --locked
  ```
  Gate 14's manifest check reads only `name` and `harness`:
  ```
  /^\[\[bench\]\]/ { inb = 1; nm = ""; hz = ""; next }
  ...  if ($0 ~ /^[[:space:]]*name[[:space:]]*=/) ...  if ($0 ~ /^[[:space:]]*harness[[:space:]]*=/) ...
  if grep -qxF "${stem} false" /tmp/gate-bench-decl.txt; then  printf '  ok ...
  ```
  `grep -rn "bench = false\|required-features"` over `docs/`, `ci.yml` and the audit workspace returns nothing.
- Why it is wrong: Cargo's `bench = false` on a `[[bench]]` target tells `cargo bench` not to run it, and a target with unmet `required-features` is skipped silently when it is not named explicitly. Gate 8 only counts tracked files and trusts the exit code of `cargo bench --workspace`. Gate 14 refuses `harness = true` for exactly this reason (the breach exit would "never reach cargo"), but it does not refuse the attribute that keeps the bench from running at all. CLAUDE.md §3 rule 4 makes this gate the enforcement of O(1).
- Repro: add `bench = false` under `[[bench]] name = "ratio"` in `crates/engine/Cargo.toml` (it sits with `harness = false`, so gate 14 prints `ok`), then make `Column::support` scan. Gate 8 prints "13 bench source file(s) tracked" and `cargo bench` builds and runs 12 harnesses with engine's left out. Exit 0, ci-ok green.
- Suggested fix: in gate 14 layer 2b, also refuse a `[[bench]]` that sets `bench` to anything but `true` or sets `required-features`. In gate 8, count the `Running benches/...` lines in cargo's output and refuse unless there is one per tracked bench file.

### P1-08-03 medium Gates 17 and 22 recognise `telemetry`/`log`/`tracing`/`store`/`pull` only by the dependency KEY, so a Cargo `package =` rename in `runner` gets past both. Gate 22's comment says the capability is still refused, and it is not
- Where: `.github/workflows/ci.yml:3069` (gate 17 rule), `:4099` (gate 22 `store_rule`), `:4057-4059` (clause C), `:3967-3973` (the claim), `:3935-3953` (`runner` deliberately not in `PINNED`)
- Evidence:
  ```
  rule='^.+:[0-9]+:((telemetry|log|tracing)(::.+|!)|((std|core|alloc)::)?(e?print(ln)?|dbg)!|...'
  store_rule='^.+:[0-9]+:(brutex_)?(store|pull|lake|api|telemetry)::.+$'
  # WHAT IS STILL NOT PINNED, said rather than implied: nothing stops
  # `crates/runner` declaring `store` in its manifest, because clause A
  # does not read it. Clause C would refuse the `use`, and clause B the
  # `std::fs` call that followed -- so the capability is still refused at
  # the two places it has to be exercised, and only the DECLARATION is
  # unguarded.
  ```
  `source_scan.rs:1203-1296` (`canonical_paths`) expands only `use` aliases. It has no notion of a manifest's `package =` key. The workspace already uses renames as house style: `crates/runner/Cargo.toml` has `brutex_core = { path = "../core", package = "core", ... }`.
- Why it is wrong: `runner` holds the per-bar and per-trial loops (gate 17's own comment says so), and it is the one swept crate whose manifest no gate pins. With `bars = { package = "store", path = "../store", version = "0.1.0" }`, a call `bars::file::BarFile::open_existing(..)` prints as `bars::file::...`. That matches neither clause C, C2 nor gate 17, and clause B never fires because the file I/O happens inside `store`. So "the capability is still refused" is false. Likewise `t = { package = "telemetry", ... }` or `lg = { package = "tracing", version = "0.1" }` lets `t::emit(..)` and `lg::info!(..)` run per trial with gate 17 green. A non-swept intermediary works the same way: `costs` is unpinned and not in `SWEEP`, so it can take `store` and hand `runner` a reader. The only remaining tripwire is `crates/core/tests/graph.rs::every_documented_arrow_is_a_real_arrow`, and the same PR satisfies it by adding the arrow to the `docs/01-architecture.md` table.
- Repro: (1) add `bars = { package = "store", path = "../store", version = "0.1.0" }` to `crates/runner/Cargo.toml`; (2) add `store` to runner's row in `docs/01-architecture.md` §1; (3) in `crates/runner/src/lib.rs`, call `bars::file::BarFile::open_existing(...)` (not `BarFile::open`, whose `File::open` substring trips clause B's text grep). Gate 22 prints "OK — vocab, indicators, engine and runner cannot reach a bar". The same steps with `t = { package = "telemetry", path = "../telemetry", version = "0.1.0" }` and `t::emit(..)` inside `runner::trade` leave gate 17 green.
- Suggested fix: resolve each swept crate's dependency keys to packages with `source_scan deps` (the `$NF` column already carries the package) and refuse, by package, any `store`/`pull`/`lake`/`api`/`telemetry`/`log`/`tracing` in `runner`'s manifest and in the transitive closure of every `SWEEP` crate (`cargo metadata` or a manifest walk). Then delete the false sentence at `:3967-3973`.

### P1-08-04 low Gates 11, 19, 23 and 14 pick files by a `/src/` glob and exempt a "whole-file test module" by its file stem alone. Production code at another path with that stem, or mounted by `#[path]` from outside `src/`, is never scanned
- Where: `.github/workflows/ci.yml:5384-5391` and `:5424-5428` (gate 11), `:4760-4768` (gate 19), `:6657` (gate 14), `:3123` and `:3170` (gate 23)
- Evidence:
  ```
  whole_file_test_module() {
    if [ "$(head -1 "$1")" = '#![cfg(test)]' ]; then return 0; fi
    stem="${1##*/}"; stem="${stem%.rs}"
    root="${1%/src/*}/src/lib.rs"
    [ -f "$root" ] \
      && grep -Eq "^(pub(\([a-z]+\))? )?mod ${stem};$" \
           <<< "$(grep -A 1 '^#\[cfg(test)\]$' "$root" || true)"
  }
  ...
  done < <(git ls-files 'crates/*.rs' | grep '/src/' | ...
  ```
  Gate 23: `# This walks crates/*/src BY GLOB. There is no crate list to fall out of date.` and `git ls-files -z -- 'crates/*/src/*.rs' > "$g23/files.z"`.
  Production module outside `src/`: `crates/cli/src/lib.rs:80-81` `#[path = "../commit_stamp.rs"] mod commit_stamp;`, used at `crates/cli/src/lib.rs:156` `commit_stamp::canonical(candidate)`.
  Stems exempted today by a crate-root `#[cfg(test)] mod X;`: engine `manifest`, store `emits`, pull `emit_sites`, api `scratch`/`isolated`/`emitted`/`saved_response_boundary_tests`, and seven cli `*_tests`.
- Why it is wrong: the comment says "A crate-root test attribute ... must exclude the file from production. Names and comments cannot do so", but the check compares only the stem, never the directory. So `crates/engine/src/resume/manifest.rs`, mounted from `resume.rs` by a plain `mod manifest;`, is production code that gate 11's seven rules and gate 19 skip completely. Separately, `crates/cli/commit_stamp.rs` is compiled into every `cli` build but lies outside `/src/`, so gates 11 and 23 never open it. Gates 17 and 22 added a `source_scan closure` walk for exactly this `#[path]` case (D-1119), and 11, 19 and 23 did not. The workspace lint table does not deny `print_stderr`/`print_stdout`, so an `eprintln!` there is caught by nothing.
- Repro: (a) append `pub fn stamp_note() { eprintln!("x"); }` to `crates/cli/commit_stamp.rs`. Gate 23's clause A counts are unchanged and it passes. (b) create `crates/engine/src/resume/manifest.rs` containing `let mut m = std::collections::HashMap::new(); v.sort();`, mounted by `mod manifest;` in `resume.rs`. Gate 11 rules 3 and 4 see nothing.
- Suggested fix: build the file list from `source_scan closure` over each crate's `lib.rs`/`main.rs`, as gates 17 and 22 do. Exempt a file only when the closure shows it is reached solely through a `#[cfg(test)]` module declaration, never by stem.

### P1-08-05 low Gate 7 still holds a reappearing `crates/web` to the one-spelling `[dependencies]` awk that D-1107 removed from gates 9 and 9b
- Where: `.github/workflows/ci.yml:2001-2021`
- Evidence:
  ```
  # ... a crate of this name reappearing must be held to it again.
  [ -f "$f" ] || { echo "skip — crates/web does not exist (D-0052 moved it to web/)"; exit 0; }
  deps=$(awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f && /=/ {print $1}' "$f" \
  ```
- Why it is wrong: the comment says the step is kept to hold a reappearing `crates/web` to "core alone". The parse only sees `name =` inside a flat `[dependencies]` table. `[dependencies.store]`, `dependencies.store.path = ...`, `[target.'cfg(..)'.dependencies]` and `[dev-dependencies]` all print nothing and the step says "OK.". Gates 9 and 9b document this exact hole as having "defeated NINE guarantees at once" and replaced it with `source_scan deps`.
- Repro: create `crates/web/Cargo.toml` with `[package] name="web"` and `[dependencies.store]` / `path = "../store"`. Gate 7 prints `OK.`
- Suggested fix: use `"$scan" deps "$f"` and require that its only package is `core`, as gate 9 does.

## Checked and found sound, or already recorded (not repeated)
- ci-ok (`:7871-7899`): `needs` all 7 jobs, `if: always()`, accepts only `success`, refuses an empty result. Sound. That nothing guards its composition is hunt-ci-4.
- Gates 9 and 9b: parse the manifest with `source_scan deps`, refuse when it is absent or unparseable, and print any declaration. Sound. The workspace-root `[patch]` blind spot is stated in the gate.
- Coverage at 90/89 and no branch coverage: recorded (hunt-ci-10, D-0677). W4 pipefail and silent zero: hunt-ci-7. `step-runs` accepting `|| true`: hunt-ci-3. Gate 18 diff-only and `HEAD~1` on push: documented.
- Gate 11 rule 7 blind to `.contains(x)` with `x` already a reference: recorded and deliberately left unfixed in `docs/05-decisions.md:18936-18942`.
- No `continue-on-error`, `paths:` or `paths-ignore:` filter on any job. Every `|| true` sits inside a `$( { grep ...; } || true)` capture whose result is tested afterwards. The `mutants` shard captures `PIPESTATUS[0]` explicitly.

---
## Source: 09-actions-security.md

# P1-09 GitHub Actions security (auto-merge.yml, ci.yml, deny.toml) at 331b05c

### P1-09-01 low The only thing stopping a fork PR from arming auto-merge is the workflow's own `isCrossRepository` check, but the file says a read-only token stops it. On `workflow_run` the token can write.
- Where: .github/workflows/auto-merge.yml:37-40, 65-72, 185-190
- Evidence:
  - `:38-40` "the PR is from a fork (a fork gets a read-only token by design)"
  - `:185-190` "A fork PR gets a read-only token, so this workflow can NEVER arm it — not on a later push, not ever." then `[ "$fork" = "false" ] || stopped "... a fork gets a read-only token by design. Auto-merge can never be armed for it ..."`
  - `:65-72` `workflow_run: workflows: [CI] types: [completed]` with `permissions: contents: write, pull-requests: write`
- Why it is wrong: GitHub runs a `workflow_run`-triggered workflow in the base repository with the base repository's token. That token gets the `permissions:` block above, so it can write even when the run that triggered it came from a fork. Only the `pull_request` trigger gets a read-only token for a fork. A fork PR's own CI also runs the fork's copy of `ci.yml`, so it can produce a green `CI` run and a `ci-ok` check from the GitHub Actions app (protection requires app 15368 `ci-ok`, per hunt-ci-1). That means the `[ "$fork" = "false" ]` line is the one control between a fork and a self-certified auto-merge, not a cosmetic duplicate of a token limit. The comment gives the credit to the token, so someone tidying this file could delete the line as redundant.
- Repro: Static. Remove lines 189-190, then have a fork PR (after approval to run, if the repo requires it) change `ci.yml` to a single job `ci-ok` that runs `true`, and keep the workflow name `CI`. The fork's CI run succeeds. `workflow_run` fires in the base repo with `CI_CONCLUSION=success`. Section 1 resolves the fork PR through `commits/{sha}/pulls`. Sections 3-5 pass, and `gh pr merge --auto --squash` runs with a write token. Protection then sees `ci-ok` succeed on that head and merges. A test sketch: a source_scan or gate assertion that `auto-merge.yml` keeps an `isCrossRepository` refusal ahead of `gh pr merge`.
- Suggested fix: Rewrite the comments at :38-40 and :185-188 (and the `stopped` text) to say this check is the load-bearing refusal, because on `workflow_run` the token can write. Pin the check with a gate so deleting it turns CI red.

## Checked, nothing new (or already recorded in hunt-ci.md)
- `pull_request_target`: not used. Script injection: every `github.event.*` value in both workflows crosses into the shell through `env:` and is quoted (auto-merge.yml:86-101; ci.yml:7440 `BASE_REF`). The `concurrency.group` expression is not a `run:` body. No `secrets.` references and no `curl | sh`. Tools are installed with `cargo install --version X --locked`.
- Recorded already: CI has no `permissions:` block and actions are pinned by mutable refs (hunt-ci-12). No review is required and auto-merge self-certifies same-repo PRs (hunt-ci-1). A STOP does not disarm auto-merge (hunt-ci-8). PR selection takes `.[0]` (hunt-ci-9). `step-runs` is fooled by `cargo deny check || true` (hunt-ci-3).
- Cache poisoning: `actions/cache` restores `~/.cargo/bin/cargo-mutants` and `cargo-nextest` under a fixed key, checked only by their version string (ci.yml:7413-7431, 7548-7567). That cache can only be poisoned from the main-branch scope, which already needs write access. A fork PR's cache stays in its own PR scope. Not exploitable from outside.
- auto-merge permissions `contents: write, pull-requests: write, checks: read` are the minimum `gh pr merge --auto` needs. No checkout and no PR code runs in the privileged job.
- deny.toml: `[advisories] version = 2` with no `ignore` list, so vulnerabilities, unmaintained and unsound crates are denied by default. `yanked = "deny"`. Licences use an allowlist, and the two exceptions are per crate with ledger entries. `private = { ignore = true }` reaches only `publish = false` workspace crates. `[bans]` has `wildcards = "deny"` and an explicit runtime/C deny list (`multiple-versions = "warn"` is a deliberate soft setting, not a disabled check). `[sources]` denies unknown registries and unknown git sources and allows only crates.io. No crate declares `[features]`, so default-feature resolution misses nothing. Gate 3 runs a bare `cargo deny check`, which covers all four checks, with cargo-deny pinned at 0.19.0 (ci.yml:7126-7155).

---
## Source: 10-tests-cli-a.md

# P1-10: tests in crates/cli, first half alphabetically (static audit, commit 331b05c)

Scope: the first 105 entries of `ls crates/cli/src | sort` (admission_join.rs .. global_replay_v4_tests.rs, columns*.rs included) plus `crates/cli/tests/binary.rs`. Static reading only.

### P1-10-01 medium `the_threshold_search_runs_end_to_end` can only fail if the binary crashes; it has the exact defect its sibling test's comment describes
- Where: crates/cli/tests/binary.rs:146-158; crates/cli/src/lib.rs:2295-2299, 3893-3911
- Evidence:
  ```rust
  fn the_threshold_search_runs_end_to_end() {
      let out = command("auto").args(["auto", "2"]).output().expect("the binary runs");
      assert!(out.status.success(), "a valid auto exits zero");
      let text = String::from_utf8_lossy(&out.stdout);
      assert!(text.contains("THESE BARS ARE GENERATED"), "provenance travels");
  }
  ```
  and the dispatch and render it exercises:
  ```rust
  ["auto", sessions] => match parse_sessions(sessions) {
      Ok(s) => { out.push_str(&auto(s)); OK }
  ...
  let mut out = String::from(PROVENANCE);
  out.push('\n');
  out.push_str(&runner::report::render_auto(&found, None));
  ```
  The test directly above (binary.rs:85-96) gives the reason this is wrong: "This test ran `sweep 2 50` and asserted `text.contains("bars")` ... the assertion was satisfied by the report's own [banner] ... asserts nothing".
- Why it is wrong: `auto` pushes `PROVENANCE` and returns `OK` whatever `Sweeper::auto` found, so both assertions hold for an empty, refused or "nothing measured" threshold search. The sweep test was fixed to read `swept`, `combinations found` and the extinction verdict. This test still checks only the banner the renderer always prints.
- Repro: replace the body of `Sweeper::auto` (or `render_auto`) so it returns or renders an empty result. `the_threshold_search_runs_end_to_end` still passes.
- Suggested fix: assert on what `render_auto` reports, for example a found threshold greater than zero or a non-empty ladder. Also assert that the text holds neither `NOTHING MEASURED` nor `REFUSED`, as the sweep test does.

### P1-10-02 low Two child-process tests cited as invariant proofs accept a child that ran zero tests
- Where: crates/cli/src/audited_range_tests.rs:610-630; crates/cli/src/checksum_receipts_tests.rs:294-333
- Evidence:
  ```rust
  let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
      .args(["--exact", "audited_stored::range::tests::strict_invalid_runtime_settings_refuse_before_real_source_admission_or_preparation", "--test-threads=1"])
      .env(CHILD, name).env(name, raw)
      .status()
      .expect("isolated malformed environment child");
  assert!(status.success(), "{name}");
  ```
  ```rust
  .args(["--exact", "checksum_receipts::tests::receipt_fifo_cannot_block_either_read_or_publication"])
  .env(PROBE, &path)
  .stdout(std::process::Stdio::null())
  ...
  if let Some(status) = child.try_wait()? { assert!(status.success()); break; }
  ```
  Every other `--exact` child in this half of the crate also asserts `stdout.contains("1 passed")`, for example audited_stored_tests.rs:731, batch_verb_tests.rs:50, boolean_catalog_command.rs:399, boolean_grammar_campaign_tests.rs:822, fold_audit_io_tests.rs:317 and boolean_runtime_knob_tests.rs:216. docs/04-invariants.md:4489 and :4429 cite these two tests as the proofs.
- Why it is wrong: if libtest's `--exact` filter matches nothing, it prints "running 0 tests" and exits 0. Renaming either test, or moving its module (the path depends on the `#[path]` mount in audited_stored.rs:12 and audited_range.rs:376), makes every child run zero tests. The parent then passes without running any assertion inside the child. The 13 malformed-environment cases and the FIFO no-block property would go unchecked, and nothing would report it.
- Repro: rename `strict_invalid_runtime_settings_refuse_before_real_source_admission_or_preparation` to `..._x` and leave the string literal unchanged. The test passes and every child prints `running 0 tests`. The same holds for the FIFO test.
- Suggested fix: capture stdout and assert it contains `1 passed`, as the other children do. Better, derive the name from `module_path!()` so it cannot drift.

### P1-10-03 low The source-order test bounds the sweep's function body but not the screen's, so the screen half can match code in later functions
- Where: crates/cli/src/equity_statement_tests.rs:369-406
- Evidence:
  ```rust
  let body = |name: &str| { ... rest.split_once("\n}\n").expect("the function has a closing brace").0 };
  let sweep = body("stored_sweep_inputs");
  let screen = lib
      .split_once("\nfn screen_range_inner(")
      .expect("the screen exists")
      .1;
  ```
  The doc comment says: "Pinned by name because the two are separate functions".
- Why it is wrong: `screen` is all of lib.rs after `fn screen_range_inner(`, through the end of the file (about 12,000 lines past its start at lib.rs:15732, lib.rs is 27,943 lines). `validate_one_minute_execution(` already occurs again after the function, at lib.rs:16351, 16398 and 27805 onward. Today the test still fails on a reorder only because the other four steps happen not to recur later in the file. Any later function that calls the same loaders in order makes the screen half of the check pass no matter what `screen_range_inner` does.
- Repro: add a new helper after `screen_range_inner` in lib.rs that calls the five steps in order, then move `stored::load_daily_context(` in `screen_range_inner` to before `minute_gaps::withhold(`. The test still passes.
- Suggested fix: take `screen` through the same `body("screen_range_inner")` closure. It needs a `fn screen_range_inner(` split with the multi-line signature, ending at `"\n}\n"`.

### P1-10-04 low A Base Evidence projection's `admission_values` is read back and thrown away; no test anywhere asserts on it
- Where: crates/cli/src/candidate_universe.rs:6941 (test `durable_base_is_receipt_last_idempotent_fixed_offset_and_zero_row_safe`, line 6910); projection built at crates/cli/src/population_base_evidence_ledger_v2.rs:1098-1107
- Evidence:
  ```rust
  assert_ne!(first.trade_rows_digest(), [0; 32]);
  let _ = first.admission_values();
  ```
  `rg '\.admission_values\(\)' crates` shows this is the only call to `BaseEvidenceRecordProjectionV2::admission_values` in the workspace. The other hits are on `BaseEvidenceRecordV2` or a draft.
- Why it is wrong: the test is the only coverage of the durable projection's admission values, and it discards them. A defect in the decoded record's `admission_values()` mapping (`population_base_evidence_ledger_v2.rs:1105`) would survive: wrong fields, values from a different record, or a reconciliation that always produces the default. The same goes for a mutant of the accessor at line 458. This cuts against the §9 "no surviving mutant" requirement.
- Repro: change the projection to fill `admission_values` from the original record's values with one field zeroed. Nothing in the suite fails.
- Suggested fix: assert that `first.admission_values()` equals the admission values computed from `candidate_base_fixture`'s first base record. One field-by-field `assert_eq!` against the pre-append draft is enough.

## Checked and not reported
- All 15 `--exact` child-process tests: 13 check `1 passed` or use a TEST constant whose module path matches its `#[path]` mount. The two that do not are P1-10-02.
- Source-shape (`include_str!`) tests in batch_verb_tests, boolean_candidate_tests, boolean_oos_tests, boolean_statistics_tests, boolean_search_integration_tests, candidate_universe and execution_capability: each is bounded to `"\n}\n"` and carries positive `contains` anchors, so a rename fails loudly. The one exception is P1-10-03.
- Bare `matches!`, `is_err();` and `contains();` statements: none outside `assert!`.
- Self-comparisons `assert_eq!(x, x)` and `len() >= 0`: none.
- `.all()` over possibly empty collections in boolean_* tests: each has a length or count guard nearby, or its rows exist by construction (boolean_oos_v1 `append_rows`).
- Tests with no assertion: the remaining hits delegate to asserting helpers (`kernel_heads_by_kind`, `contract_is_refused_by`, `under` in columns_tests, `refused_without_rows`).
- `if let Ok/Some` guards in *_tests.rs: each is paired with an `is_ok()` or `is_some()` equality assert or a counter.

---
## Source: 11-tests-cli-b.md

# P1-11 — tests in crates/cli (second half of `ls crates/cli/src crates/cli/tests`) that cannot fail or do not test what they claim

Scope: `crates/cli/src/global_replay_v4_tests.rs` .. `vix_reference.rs` plus every file in `crates/cli/tests/`, commit 331b05c. Static reading only, nothing was run.

### P1-11-01 medium `every_command_is_listed_in_both_places` is cited as proof that the list, the dispatch and the usage agree, but it never looks at the dispatch
- Where: crates/cli/src/lib.rs:25635-25661 (test); crates/cli/src/lib.rs:2341-2343 (claim); crates/cli/src/lib.rs:2318-2328 (`unmatched`); docs/05-decisions.md:32670; docs/04-invariants.md:4011 (SC-08)
- Evidence:
  - lib.rs:2341: `/// So it is written down, and \`every_command_is_listed_in_both_places\` asserts` / `/// the list, the dispatch and the usage all name the same set.`
  - Test doc, lib.rs:25632: `/// [\`COMMANDS\`], the dispatch and [\`USAGE\`] name the same set of commands.`
  - The test body reads only `COMMANDS` and `USAGE`: `USAGE.contains(&format!("cli {word} ")) || ...`, then `for line in USAGE.lines() { ... COMMANDS.contains(&word) ... }`, then a sortedness check. Nothing calls `run`, and nothing reads the `match` arms.
  - The sibling `a_known_command_with_the_wrong_arity_is_not_called_unknown` (lib.rs:25670) does call `run(&argv(&[word]))`. But its expected text comes from `unmatched`, which decides "is a command" from the same list: `if COMMANDS.contains(&word) { format!("\`{word}\` is a command, but not with {given} argument{}...` (lib.rs:2319). So it also passes for a word that has no dispatch arm.
  - decisions.md:32670: "`every_command_is_listed_in_both_places` pins the wiring. Invariant SC-08." SC-08 lists this test as one of its proofs.
- Why it is wrong: The dispatch is the one set the doc says the test guards, and the test never reads it. Delete a dispatch arm and keep its `COMMANDS` entry and usage line: both tests stay green, and the binary tells the operator "`X` is a command, but not with N arguments" about a command it can no longer run.
- Repro: Delete `["research-plan", v] => research::command(v, out),` (lib.rs:2248). No test calls `run` with `research-plan` (the only other mention is the `matches!` list at sweep_wiring_tests.rs:156). Both `every_command_is_listed_in_both_places` and `a_known_command_with_the_wrong_arity_is_not_called_unknown` still pass, and `cli research-plan zerodha` now refuses as a wrong-arity call.
- Suggested fix: For each `COMMANDS` word, assert that `run` with that word and a correct-arity placeholder argv does not reach the `[word, rest @ ..]` fallback, for example with a test-only marker from `unmatched`. Failing that, correct the two docs and the SC-08 row so they stop claiming the dispatch is checked.

### P1-11-02 low Four child-process tests cannot tell that their child ran zero tests, so a rename turns them into passes that prove nothing
- Where: crates/cli/src/readonly_file.rs:~85-115; crates/cli/src/research_policy.rs:~510-530; crates/cli/src/research_policy_tests.rs:~(`isolated_process_precedence_preserves_owner_selected_path_and_refusals`); crates/cli/src/results_report_tests.rs:374-400
- Evidence:
  - Each test re-runs the current test binary with `--exact "<module path>::<test name>"` plus an env flag. The real assertions run only in the child, and the parent checks only the exit status:
    - readonly_file.rs: `.args(["--exact", "readonly_file::tests::fifo_without_a_writer_refuses_and_the_probe_cannot_hang"])` `.stdout(std::process::Stdio::null())` ... `assert!(status.success(), "nonblocking FIFO child must refuse normally");`
    - research_policy.rs: `.args(["--exact", "research_policy::tests::selected_runtime_profile_reaches_v6_preflight_and_preserves_override_refusals", "--nocapture"])` ... `assert!(output.status.success(), ...)`
    - research_policy_tests.rs: `"research_policy::adversarial_tests::isolated_process_precedence_preserves_owner_selected_path_and_refusals"` ... `assert!(output.status.success(), "mode {mode}: ...")`
    - results_report_tests.rs: `"results_report_tests::public_listing_obeys_the_configured_owned_root"` ... `assert!(output.status.success(), ...)`
  - libtest exits 0 when `--exact` matches no test ("0 passed; ... N filtered out"). Every other child-process test in this half guards against that by checking the child's stdout. For example operator_boundary_tests.rs:153 has `assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));`, and pool.rs and lib.rs do the same. These four do not, and readonly_file.rs discards the child's stdout entirely.
- Why it is wrong: The paths are correct today. But renaming the test, its module, or the `#[path]` mount (`research_policy.rs:5-6` mounts `research_policy_tests.rs` as `adversarial_tests`) makes the child match nothing and exit 0. The parent then reports green while the FIFO refusal, policy precedence and owned-root listing assertions never run. That is "a test that asserts nothing" (CLAUDE.md §4), arrived at silently.
- Repro: Rename `fn public_listing_obeys_the_configured_owned_root` to `..._root2` without updating the string literal. The parent spawns the child, the child prints `running 0 tests ... ok`, and the parent passes. Only `fixture.unchanged(&before)` runs, and it holds trivially.
- Suggested fix: Capture the child's stdout in all four tests (do not null it) and assert it contains `"1 passed"`, as the sibling tests do. Better still, build the path from `module_path!()` plus the function name so a rename cannot drift.

### P1-11-03 low The "partial archive" case in `original_context_partial_torn_and_changed_archives_never_supply_candles` runs on an archive that is already corrupt, so it cannot fail
- Where: crates/cli/src/index_stop_source_context_tests.rs:602-620; cited in docs/04-invariants.md:5121
- Evidence:
  ```
  let mut body = fs::read(&path).map_err(display)?;
  let last = body.last_mut().ok_or("generated source body")?;
  *last ^= 1;
  fs::write(path, body).map_err(display)?;
  assert!(reader.require_current().is_err());
  assert!(reader.window(0, 0, 0).is_err());
  assert!(open(&fixture, &saved, 0, bounds()).is_err());
  fs::remove_file(directory.join("complete.bin")).map_err(display)?;
  assert!(open(&fixture, &saved, 0, bounds()).is_err());
  ```
- Why it is wrong: The flipped byte in `body.bin` is never restored. So the last assertion, the "partial" case with the completion receipt removed, runs on an archive that line 616 has already shown to be refused, and it uses a bare `is_err()`. Removing the `complete.bin` check from the source-context open path would leave this test green. The invariant row cites this test for "partial" archives, but this test does not exercise that case on its own.
- Repro: Make the source-context reader ignore a missing `complete.bin`, so it validates only `body.bin`. The test still passes, because `body.bin` is corrupt and `open` still fails on it.
- Suggested fix: Test the missing receipt on a fresh `saved()` fixture with an intact `body.bin`, or restore the byte before removing `complete.bin`. Also assert the refusal text names the receipt rather than accepting any `Err`.

## Checked and not reported (no defect found)
- `should_panic` without `expected =`: none in scope. The only `should_panic` (tests/limits_doc_drift.rs:180) has `expected`.
- `#[ignore]`: only `index_stop_tests.rs:411 catalog_attempt_throughput_measurement`. It is not cited in docs, and its `_group` binding does keep the `Group` override guard alive, so `BRUTEX_MEASURE_GROUP` works.
- Self-matching source-shape tests (`include_str!` of their own file): lib.rs, pool.rs and knobs.rs build their needles with `concat!`, filter out comment lines, or split off at `\nmod tests {`. None of the ones checked matches its own needle.
- `policy_of` index pins in `every_knob_that_moves_the_answer_moves_the_identity`: they do catch an inserted term (horizon ≠ 0 sits at the index next to the budget).
- `commit_stamp()` branches (pool.rs:2386, lib.rs:23543): both branches assert. This is the documented, deliberate design.
- Early `return Ok(())` hits in this half: all are child-mode branches of child-process tests, or production code.
- Tests reported by the awk scan as having no assertion (lib.rs:26710, step3_orchestrator.rs:5277, the tests/* helpers): false positives. They are helpers, a compile-time type test, or tests whose assertions sit inside closures.
- `index_stop_store_tests` reconciliation tests: the first test's mutations are caught by the digest check alone. The second test recomputes the digest, and direction is not in the digest, so the first test's direction case is genuine.

---
## Source: 13-tests-engine.md

# P1-13: tests in runner / engine / vocab / indicators that cannot fail or do not test what they claim

Audited statically at /home/claude/brutex-audit (331b05c). Nothing was run. Checked against
docs/11-findings.md and /mnt/project-files/audit-20261003-workspace/*.md for duplicates. Already
recorded and skipped: V-02/V-03 cover only 3 of 9 sources (F-62285F, F-F2FC17), PastPrefix (F-C8BDE2),
the stale `seen` doc passages (o1eng2-4), and the `duplicates == 0` witness (D-1440).

### P1-13-01 medium Three more §3 rule 5 idempotence tests pass for a constant body, including the production `Evaluator`'s only one (D-0373's census stopped at seven)
- Where: crates/indicators/src/evaluator.rs:1993-2008; crates/indicators/src/trend.rs:1480-1499; crates/indicators/src/column.rs:1748-1755
- Evidence:
  - evaluator.rs:1993 `/// Two runs over one slice agree exactly — §3.5, measured rather than asserted.` and then
    `bars.iter().filter_map(|b| e.step(b).ok()).collect()` run twice, then `assert_eq!(again, once, ...)`. That is the whole test.
  - trend.rs:1490-1498 does the same thing: `series.iter().filter_map(|c| t.step(c, tol()).ok()).collect::<Vec<_>>()`, then `assert_eq!(run(), first, ...)`.
  - column.rs:1752 `assert_eq!(Column::build(&bars, &mut a), Column::build(&bars, &mut b));` and nothing else.
  - docs/05-decisions.md D-0373: "`crates/indicators` holds seven `assert_eq!(run(), run())` sites ... The equality holds for ANY deterministic body — including `bits() -> ConditionMask::ZERO` and any constant". The fix added the non-constant guard `.any(|(later, earlier)| later != earlier)`. `grep -rn "later != earlier" crates/` hits only daily, orb, fib, session, vwap, pattern and lib.rs.
- Why it is wrong: these three are spelled differently (`assert_eq!(again, once` / `run(), first` / `Column::build == Column::build`), so the census grep missed them. They are exactly the defect D-0373 fixed elsewhere. Indicators holds no HashMap, clock or RNG (one HashSet sits in a lib.rs test), so two in-process runs agree for any deterministic body, constant ZERO included. The evaluator and trend versions also `filter_map(.ok())` away refusals, so an evaluator that refused every bar compares `[] == []` and passes. `Evaluator::step` is the aggregate the production `Column::build` drives, and its test is the one left unguarded.
- Repro: replace the body of `Evaluator::step` (or `TrendState::step`) with `candle.check_evaluable()?; Ok(ConditionMask::ZERO)`, or with `Err(Corrupt::HighBelowLow)`. `two_runs_agree_byte_for_byte` stays green in both cases. Do the same to `Column::build_from`'s emitted row and `the_same_bars_give_the_same_column_twice` stays green.
- Suggested fix: add the same "run is not constant" guard to all three tests, plus an assertion that the collected vector is non-empty (`once.len() == bars.len()` for the evaluator fixture). Extend D-0373's census to every §3 rule 5 test, not only the `run(), run()` spelling.

### P1-13-02 low `a_refused_bar_is_named_and_changes_no_mask` cannot see a mask change: the refused bar is the LAST bar, and the test compares only a count
- Where: crates/runner/src/lib.rs:1590-1610
- Evidence:
  ```
  let mut bars = synthetic::sessions(8);
  let last = bars.last().copied()...;
  bars.push(candle(last.ts_micros + 60_000_000, last.close, last.close - 10, last.close + 10, last.close));
  ...
  // The column is exactly what the clean run produced.
  let clean = Sweeper::new(bounded()).run(&synthetic::sessions(8), &mut evaluator());
  assert_eq!(out.census.swept, clean.census.swept);
  ```
- Why it is wrong: the name and the comment both claim the column, meaning the masks, is unchanged. The only comparison is `census.swept`, a `u64` count (indicators/src/column.rs:231). The refused bar is also appended after every real bar. Even a refusal that corrupted evaluator state (for example by folding into an EMA or session range before refusing) has no later bar whose mask could show it. So the test cannot fail for the property it names.
- Repro: make `Evaluator::step` fold the candle into its trend/session accumulators before `check_evaluable` refuses it. Masks of every bar after a refusal would change, and this test stays green: no bar follows the refusal, and `swept` is unchanged.
- Suggested fix: insert the high<low bar mid-stream (for example in session 4). Assert that `out.sweep` (or the built `Column::bits()`) equals the clean run's on every non-refused row, not just that the counts match.

### P1-13-03 low The engine's source-shape guard for `Column::support` reads comments as code, which is the bypass already fixed in its vocab twin
- Where: crates/engine/src/column.rs:471-488 (test), :99-110 (the body it scans)
- Evidence:
  ```
  let body = source
      .split("pub fn support(&self, candidate: &ConditionMask) -> u64 {")
      .nth(1)
      .and_then(|tail| tail.split("pub fn support_fingerprinted").next())
  ...
  body.matches("row.hits(candidate)").count(), 1,
  ...
  !body.contains("set_positions(candidate)") && !body.contains("popcount()"),
  ```
  Compare crates/vocab/src/mask.rs:472-484: "Comments are stripped before anything is inspected, and this line is load-bearing ... An adversarial audit beat the previous version of this test by moving the word-5 term into a comment".
- Why it is wrong: CLAUDE.md §3 rule 4 says "`C-E-02`, `C-E-09` and the source-shape tests bind the corrected live method". This guard extracts everything up to `pub fn support_fingerprinted`, which includes the ~40-line doc comment of the next function, and strips no comments. One `// row.hits(candidate)` line meets the "exactly one hit test" count while the code does anything else. The ban list matches exact spellings only, so `set_positions(&candidate)`, `candidate.popcount ()` or a hand-written per-position loop all pass. The bench rows are still the guard of record. This unit test, which the invariants lean on as the source-level pin, does not pin what it says.
- Repro: replace the body with
  `// row.hits(candidate)` + `self.rows.iter().filter(|row| set_positions(&candidate).all(|p| row.get(p))).count() as u64`.
  This is the Θ(k) vertical-shaped loop. `the_live_support_body_is_one_fixed_width_hit_test` passes.
- Suggested fix: strip `//` comments from the extracted region and end it at the function's closing `\n    }` rather than at the next function's name, as `vocab::mask::hits_does_the_same_work_for_every_input` does. Also ban the `for `/`while `/`.all(`/`.any(` shapes, not only two exact call spellings.

## Checked and found sound (no finding)
- engine E-02 `the_apriori_kept_set_equals_the_brute_force_kept_set`: exhaustive oracle that uses the free `support` and not `Column::support`, with exclusion handling symmetric.
- engine `every_generated_candidate_has_exactly_k_bits` and `the_prefix_join_finds_exactly_what_an_exhaustive_join_would`.
- engine resume oracle (`tests/resume_readiness.rs`): an independent powerset oracle.
- engine C-E-01/02/03/05/06/07/09/10/11/04 bench rows: zero-baseline refusal, setup-extreme refusals, and a structural-equality precondition for the one-sided C-E-04.
- vocab `hits_does_the_same_work_for_every_input` (comment-stripped and hardened), the mask naive-reference tests, and the per-word hit/intersect test.
- indicators readiness oracles (orb, sweep_predicate), which are independent of the module code. vwap sigma has a big-integer oracle. V-05 `differential_vs_naive` follows the document's recurrence.
- runner `prefix_only_cadence.rs` look-ahead tests, `split::an_anchored_fold_never_trains_on_its_own_future`, the closure/rank idempotence tests (HashSet/heap order is a real nondeterminism source there), and the `end_to_end` equity rerun.

---
## Source: 14-tests-store.md

# P1-14: tests that cannot fail, or do not test what they claim (store, lake, pull, telemetry, core, costs, greeks)

Static review at 331b05c. Nothing was run. Every finding below was checked against docs/11-findings.md and /mnt/project-files/audit-20261003-workspace/*.md before it was recorded.

### P1-14-01 medium `the_signing_key_matches_the_published_derivation` never compares against the published vector, so a wrong SigV4 key derivation passes every test
- Where: crates/pull/src/ssm.rs:1044-1077 (test), crates/pull/src/ssm.rs:460-465 (`signing_key`), :565-571 (`authorization`)
- Evidence:
  ```rust
  /// AWS publishes worked `SigV4` examples with every intermediate value. This
  /// is the canonical one, so the four hashes below are checked against the
  /// specification rather than against this implementation's own output.
  ...
  /// **THE VECTOR AWS PUBLISHES.** `AWS4-HMAC-SHA256` over the documented
  /// `get-vanilla` example yields a signing key whose hex is fixed by the
  /// specification. If this drifts, every request this module signs is
  /// rejected
  fn the_signing_key_matches_the_published_derivation() {
      let key = signing_key(EXAMPLE_SECRET, "20150830", "us-east-1");
      assert_eq!(key.len(), 32, "HMAC-SHA256 is 32 bytes");
      assert_eq!(key, signing_key(EXAMPLE_SECRET, "20150830", "us-east-1"));
      assert_ne!(key, signing_key(EXAMPLE_SECRET, "20150831", "us-east-1"));
      ...
  ```
  The only 64-hex literals in ssm.rs are the SHA-256 of `""` and `"abc"` (lines 1085 and 1089). No test pins a signing-key hex or a `Signature=` value. `the_authorization_header_never_carries_the_secret` checks only `header.contains("Signature=")` and determinism.
- Why it is wrong: The name and the doc both claim a known-answer check against AWS's published derivation. The body checks only length, determinism and inequality, and every HMAC chain satisfies those. Mutating `format!("AWS4{secret}")` to `format!("AWS{secret}")`, renaming `"aws4_request"`, reordering the chain links, or reordering the lines of `to_sign` (ssm.rs:566-570) still passes every test in the crate. As the doc itself says, that drift would only show up against a live endpoint, where it reads as a credentials problem.
- Repro: Change ssm.rs:461 to `hmac(format!("AWS{secret}").as_bytes(), date)`. Every ssm test still passes. Recomputed with `openssl dgst -sha256 -mac HMAC`: the published iam key for this secret/date/region is `c4afb1cc5771d871763a393e44b703571b55cc28424d1a5e86da6ed3c154a4b9`, matching AWS's documentation. The same chain with service `ssm` gives `1b014a52e2c4682dbb4f9c057f77de175576bae388238bec84a63594a1c63358`.
- Suggested fix: Assert `hex(&signing_key(EXAMPLE_SECRET, "20150830", "us-east-1")) == "1b014a52…3358"`, or factor out the service so the published iam vector can be pinned directly. Also pin one full `authorization()` output for a fixed `Signable` so the string-to-sign layout is covered.

### P1-14-02 medium graph.rs "every row names a real crate" cannot fail: the parser drops non-member rows before the check
- Where: crates/core/tests/graph.rs:450-452 (`documented_graph_from`), :507-525 (`the_documented_table_lists_every_crate_and_no_others`)
- Evidence:
  ```rust
  let name = cells[1].trim_matches('`');
  if !members.contains(name) {
      continue;
  }
  ```
  and then
  ```rust
  /// Every crate in the workspace has a row, and every row names a real crate.
  ...
  let invented: Vec<&String> = documented.difference(&real).collect();
  assert!(
      invented.is_empty(),
      "§1 has a row for {invented:?}, which is not a workspace member. `CLAUDE.md` \
       §5 names a `cli` crate that does not exist; §1 must not repeat that."
  );
  ```
  `unknown_document_members_do_not_enter_the_graph` (graph.rs:745-761) pins this dropping behaviour as intended.
- Why it is wrong: `documented_graph()` only ever returns keys that are already in `member_names()`, so `documented.difference(&real)` is empty by construction. The "invented" half of the test is dead. A §1 row for a crate that does not exist, such as the `web` crate that CLAUDE.md §5 records as a past drift, passes. A second row for an existing crate also passes: `graph.insert` keeps only the last row, so an earlier, contradictory row is never checked.
- Repro: Add `| \`web\` | browser | \`core\` | ✓ |` to docs/01-architecture.md §1. All graph.rs tests stay green. Adding a second `| \`store\` | … | \`vocab\` | ✓ |` row above the real one is also green.
- Suggested fix: Collect every backticked first-cell name from §1 rows before the member filter, assert that set equals the members, and refuse duplicate row names.

### P1-14-03 low graph.rs workspace-members parser silently drops any member outside `crates/`
- Where: crates/core/tests/graph.rs:487-497 (`the_manifest_list_is_the_whole_workspace`)
- Evidence:
  ```rust
  .split('"')
  .skip(1)
  .step_by(2)
  .filter_map(|p| p.strip_prefix("crates/"))
  .collect();
  ```
- Why it is wrong: The test exists so that "a twelfth crate could be added and every other test here would still pass while never looking at it" is caught. A member added as `"tools/x"` or `"xtask"` is filtered out before the comparison. So the closure check, the arrow check, acyclicity, the four roots and the shareable six never see it. The same applies to a glob member such as `"crates/*"`, which becomes `*`. The parser also stops at the first `]` and counts quote parity across comment lines, so a comment inside `members = [ ... ]` that contains `]` or a `"` would truncate or shift the list.
- Repro: Add `"tools/probe"` (with a Cargo.toml that depends on `store`) to the root `members`. `the_manifest_list_is_the_whole_workspace` still passes.
- Suggested fix: Refuse any member string that does not start with `crates/` (or that contains `*`), and strip `#` comments before splitting.

### P1-14-04 low core/tests/findings.rs still skips all three commit-history checks whenever `git rev-parse --git-dir` fails, which is the defect D-0693 item 8 fixed only in store
- Where: crates/core/tests/findings.rs:497-504, :601-608, :688-695
- Evidence:
  ```rust
  if git(&["rev-parse", "--git-dir"]).is_none() {
      println!(
          "SKIPPING: {} is not a git work tree, so no commit can be resolved. ...",
          repo.display()
      );
      return;
  }
  ```
  docs/05-decisions.md:39382-39388 (D-0693 item 8): "*A skip taken on any refusal.* `cited_commits.rs` skipped its history checks whenever `git rev-parse --git-dir` failed … A review ran it in a work tree under a `GIT_DIR` naming nothing: it printed that, and passed with every history check skipped. It now skips only where `git` cannot be run and where the tree's root holds no `.git`." The fix is at crates/store/tests/cited_commits.rs:221-261 (`history_is_at`, with `GIT_CEILING_DIRECTORIES` and `symlink_metadata`).
- Why it is wrong: `every_named_commit_exists`, `every_named_commit_is_in_this_branchs_history` and `every_named_commit_is_on_main` guard the ledger's `FIXED <sha>` claims. With a `.git` that git cannot read (a `GIT_DIR` naming nothing, or a `.git` file or symlink pointing nowhere), each one prints SKIPPING and passes without checking a single sha. The `checked > 0` guard is after the early `return`, so it never runs. The same refusal was judged a defect and fixed in store and was not carried over here.
- Repro: `GIT_DIR=/nonexistent cargo test -p core --test findings`. All three tests print SKIPPING and pass. Equivalently, replace `.git` with a file `gitdir: /nowhere`.
- Suggested fix: Reuse the `history_is_at` logic: skip only when `git` cannot be spawned or `.git` is absent by `symlink_metadata`, and fail with git's stderr otherwise.

### P1-14-05 low `readonly_credentials` asserts `writes == 0` on a method no production code can reach, so the "behavioural half" of P-05 cannot fail
- Where: crates/pull/tests/unit.rs:894-898, :918-944; crates/pull/src/secret.rs:16-20 (module doc), :325-336 (`ParameterStore`)
- Evidence:
  ```rust
  /// The write the real client offers and this repository never calls.
  fn put_parameter(&self, _name: &str, _value: &str) -> ! {   // inherent method on Double
  ...
  assert_eq!(double.writes.get(), 0, "a whole credential read reached the store without one write");
  ```
  `pub trait ParameterStore { fn get_parameter(&self, name: &str, with_decryption: bool) -> ...; }` has only that one method. secret.rs:16-17: "The behavioural half is `pull::unit::readonly_credentials`, which drives a whole credential read through a double whose write **panics**, and asserts afterwards that the write was never reached."
- Why it is wrong: `put_parameter` is an inherent method on the test double, not a trait method. `SsmSecretSource<C: ParameterStore>` can only name `get_parameter`, so no change to production code can increment `writes`. The assertion is true by the type system, and "asserted from a process that would have died" (unit.rs:872-874) is not what the test does. P-05's row already admits the port is not the shipped path (GAP2-38). It does not say that the cited behavioural check is also tautological.
- Repro: Add a `put_parameter` call anywhere in `SsmSecretSource::read`. It will not compile against `C: ParameterStore`, so the "write panics" path can never fire. Nothing a regression could do makes this assertion fail.
- Suggested fix: Drop the `writes` claim from the module doc and the P-05 row, and rely on the structural argument plus `the_only_action_on_the_wire_is_get_parameter`. Or add `put_parameter` to a wider trait the double implements, so a call is actually possible.

### P1-14-06 low costs adversarial matrix: every invariant except "the fixture is non-negative" sits under `if let Ok(..)`, and the 63-cell count measures loop iterations, not checked cells
- Where: crates/costs/tests/adversarial.rs:84-153
- Evidence:
  ```rust
  if let (Ok(c), Ok(f)) = (ceil, floor) { ... }
  if let Ok(levy) = levy_ceiling(notional, rate) { assert!(levy.raw() >= 0, ...) }
  if let Ok(levy) = statutory_levy(notional, rate) { ... assert_eq!(levy.raw() % 100, 0, ...) }
  ...
  assert_eq!(
      cells, 63,
      "9 notionals x 7 rates = 63 cells; a generator that produced fewer \
       would pass every assertion above without testing anything"
  );
  ```
  The module header lists invariant 6 as "**Overflow is a `Result`, never a wrap.**"
- Why it is wrong: If `levy_ceiling`, `statutory_levy`, `ceil_to_paisa` or `floor_to_paisa` regressed to return `Err` for every input, invariants 2, 4 and 5 would pass vacuously, and `cells == 63` would still hold because it counts iterations, not Ok cells. The guard's own message names this exact failure ("would pass every assertion above without testing anything") and does not prevent it. The matrix also never asserts that the `i64::MAX` cells refuse, so a wrap to a positive whole-rupee value would pass despite invariant 6. The monotonicity ladder (:186-208) has the same shape: `Err(_) => break` on the first step compares nothing.
- Repro: Make `statutory_levy` return `Err(..)` unconditionally. `every_extreme_notional_against_every_shipped_rate_holds_the_money_invariants` still passes.
- Suggested fix: Count the cells where each guarded block actually ran, and assert a floor (for example, every notional up to a crore is `Ok` for every rate). Assert `Err` explicitly for the `i64::MAX` × non-zero-rate cells.

### P1-14-07 low lake page-boundary cut check can pass with zero cases exercised
- Where: crates/lake/tests/refusals.rs:393-437, helper :461-505
- Evidence:
  ```rust
  for pages in [1_i64, 2, 4, 8, 12] {
      let Some(len) = exact_page_prefix(&sound, OI, pages) else {
          continue;
      };
      if len >= full {
          continue;
      }
      ...
  }
  assert!(fabricated.is_empty(), ...);
  ```
  `exact_page_prefix` returns `None` on any header-parse failure (`.ok()?`) or an empty remainder.
- Why it is wrong: The doc comment says this half covers "the case that lands exactly on a page boundary", which is the `Ok(None)` path that previously fabricated nulls. Nothing records how many of the five boundary cuts actually ran. If the helper's Thrift header read stops matching the writer's page layout (for example, a dictionary page first, or a changed parquet-format crate), every iteration `continue`s and the test is green with that half unexecuted.
- Repro: Make `exact_page_prefix` return `None` (for example, change `read_from_in_protocol(..).ok()?` so it fails). The test still passes, with no boundary case run.
- Suggested fix: Count the iterations that reached `read_row_group` and assert the count is at least 1 (ideally 5 for the 13-page fixture).

---
## Source: 15-docs-claude.md

# P1-15 — CLAUDE.md / AGENTS.md / README.md / HANDOVER-*.md vs code

Audited the TRACKED files at /home/claude/brutex-audit (331b05c). Note: the session-context copy of CLAUDE.md (from /home/claude/brutex) is older than the tracked one (it still says lane-local `kept` + `extend` in drain, and "cli holds no loop"); the tracked file already fixes both (D-1440, D-1448), so neither is reported.

### P1-15-01 medium CLAUDE.md §3 rule 7 still says `cli` and `runner` pass `Availability::Absent`; cli derives `Present` for every equity since D-0507
- Where: CLAUDE.md:188-190; crates/cli/src/stored.rs:2063-2068
- Evidence:
  - CLAUDE.md:188-190: "`vwap::availability_of` reads the whole slice, which is exactly why `cli` and `runner` pass `Availability::Absent` rather than deriving it."
  - stored.rs:2063-2068: `pub(crate) const fn vwap_availability(key: &InstrumentKey) -> Availability { match key.kind { Kind::Equity => Availability::Present, Kind::Index | Kind::Future { .. } | Kind::Option { .. } => Availability::Absent, } }` and its doc (stored.rs:2041-2043): "every stored path pinned [`Availability::Absent`] until D-0507".
  - AGENTS.md:144-147 already carries the corrected sentence ("stored callers select availability from instrument kind ... eligible cash equities use `Availability::Present` (D-0507)"); CLAUDE.md was not updated.
- Why it is wrong: The law file (which per §10 "wins" over documents) states a behaviour cli no longer has; 208 of the 210 swept instruments get `Present`. A reader relying on it would believe the 20 VWAP positions are dead on every run.
- Repro: `grep -n "Kind::Equity => Availability::Present" crates/cli/src/stored.rs` vs `sed -n 188,190p CLAUDE.md`; `diff CLAUDE.md AGENTS.md` shows the hunk.
- Suggested fix: Port AGENTS.md:144-147 into CLAUDE.md §3 rule 7.

### P1-15-02 low CLAUDE.md/AGENTS.md §10 "All fourteen are listed now" — docs/ holds 36 documents and a third duplicated number (22-)
- Where: CLAUDE.md:433-455 (AGENTS.md:408-412 identical)
- Evidence:
  - CLAUDE.md:450-454: "**The table was eight rows while fourteen documents existed** ... All fourteen are listed now. Two numbers are used twice — `07-` and `09-`".
  - `ls docs` lists 00- through 35- plus `research-policy/`: e.g. `12-ingest-audit.md` ... `35-ci-integration-audit.md`, and both `22-expression-search.md` and `22-research-policy.md`.
- Why it is wrong: The section claims completeness while 22 documents (12-35) carry no stated authority — exactly the defect the paragraph says it fixed — and it misses the third double-used prefix `22-`.
- Repro: `ls docs/*.md | wc -l` → 36; `ls docs/22-*`.
- Suggested fix: Either list 12-35 with their authority (or declare them non-authoritative evidence reports) and name `22-` as a third collision.

### P1-15-03 low CLAUDE.md §5 says gate 22 clause A pins `vocab` "to `vocab` alone"; the gate pins vocab to nothing
- Where: CLAUDE.md:340-341; .github/workflows/ci.yml:4014-4016
- Evidence:
  - CLAUDE.md:340-341: "clause A pins `vocab`, `indicators` and `engine` to `vocab` alone."
  - ci.yml:4014-4016: `vocab)             want='' ;;` / `indicators|engine) want='dependencies:vocab ' ;;`
- Why it is wrong: `vocab` is pinned to an empty dependency set (a crate cannot depend on itself); the sentence misstates the gate's `want` for one of its three crates. (CLAUDE.md:252 and :309 state it correctly.)
- Repro: read the two places side by side.
- Suggested fix: "clause A pins `vocab` to no dependencies and `indicators`/`engine` to `vocab` alone."

### P1-15-04 low CLAUDE.md §3 rule 4 says result append goes "through `engine::primitives::append`, at every k"; k=1 calls `Vec::push` directly
- Where: CLAUDE.md:152-153; crates/engine/src/lib.rs:1555
- Evidence:
  - CLAUDE.md:152-153: "*Result append* is one `Vec::push` per surviving candidate, through `engine::primitives::append`, at every k."
  - lib.rs:1554-1555 (k=1 loop): `} else if hits >= self.min_hits { first.push(Itemset { mask: m, hits });` — no `primitives::append`. Only `drain` (lib.rs:2390) calls `primitives::append`.
- Why it is wrong: The named primitive (the thing C-E-11 benches, CLAUDE.md:162) is not on the k=1 path; the class is the same but the doc names a call site that does not exist. Low impact.
- Repro: `grep -n 'primitives::append\|first.push' crates/engine/src/lib.rs`.
- Suggested fix: Route the k=1 push through `primitives::append`, or say "at k≥2" in the doc.

### P1-15-05 low AGENTS.md misquotes gate 1's comment ("a `AGENTS.md` edit") — find/replace artefact
- Where: AGENTS.md:55; .github/workflows/ci.yml:105
- Evidence:
  - AGENTS.md:55: "in these words: *\"Resolving that is a `AGENTS.md` edit and a ...\"*"
  - ci.yml:105: `# on its own. It read: "Resolving that is a CLAUDE.md edit and a`
- Why it is wrong: Presented as a verbatim quote "in these words", but the words were changed; the gate never said AGENTS.md (and the grammar "a `AGENTS.md`" shows a blind replace).
- Repro: `grep -n 'Resolving that is a' AGENTS.md .github/workflows/ci.yml`.
- Suggested fix: Restore `CLAUDE.md` inside the quote.

### P1-15-06 medium README.md contradicts CLAUDE.md and the tree on language, vocabulary size, sweep surface, look-ahead mechanism and crate graph
- Where: README.md:7-11, 17, 33, 51-52, 81-93, 98-104
- Evidence:
  - README.md:7-11: "CI ... fails the build on any extension outside `.rs .toml .md .lock .html .css .yml` ... including in the web UI, which is server-rendered HTML with zero JavaScript." — `git ls-files web | grep -cE '\.(js|svelte|ts|mjs)$'` → 253 tracked files; CLAUDE.md §2: under `web/` "the front end is unrestricted" (D-0052/D-0053); gate 1 `web_allowed='.*'` (ci.yml).
  - README.md:17: "every surviving combination of 74 market conditions" — crates/vocab/src/table.rs:257 `pub const TABLE: [BitDef; 370] = [`.
  - README.md:33: "Swept | `NSE-NIFTY`, `NSE-BANKNIFTY` — exactly two" and ":35 Stored, never swept | ... single stocks" — CLAUDE.md §1 adds the 208 F&O cash equities (D-0506/D-0682); research.rs test `assert_eq!(expected.len(), 208, ...)`; stored.rs:2064 sweeps `Kind::Equity`.
  - README.md:51-52: "No look-ahead ... enforced by an index-guarded accessor rather than by review." — CLAUDE.md §3.7: `PastPrefix` "has **zero production call sites**"; only references are crates/indicators/src/lib.rs:193/197 (decl) and :1178-1451 (tests). F-C8BDE2 corrected CLAUDE.md but not README.
  - README.md:82-93: "core (no dependencies at all) ├── store ├── indicators ├── vocab ├── engine ... ├── web browser UI, wasm32 — depends on core ONLY └── cli" — CLAUDE.md §5: "it named `web` ... which [does not] exist as a crate; it omitted `greeks`, `costs`, `lake`, `telemetry` and `runner` ... drew every crate as a child of `core` when four of them depend on nothing"; `crates/indicators/Cargo.toml` and `crates/engine/Cargo.toml` depend on `vocab` only, not `core`; there is no `crates/web` (gate 7 skips).
  - README.md:96-103: documents table has 8 rows; docs/ has 36.
- Why it is wrong: README is the public entry point and every one of these is a concrete, checkable statement that the code and the governing CLAUDE.md contradict (the same drawing CLAUDE.md §5 explicitly retracted as wrong). Not recorded in docs/11-findings.md.
- Repro: commands above.
- Suggested fix: Rewrite README scope/layout/rules from CLAUDE.md §1, §2, §3.7, §5 (or reduce it to a pointer to CLAUDE.md).

### P1-15-07 low HANDOVER-store-crc-read-path.md reports an open defect ("No read path calls block::verify") that the store now closes
- Where: HANDOVER-store-crc-read-path.md:10-11, 45-49; crates/store/src/file.rs:2365-2373
- Evidence:
  - HANDOVER:10-11: "`block::seal` writes a CRC32C sidecar on every commit. **No read path calls `block::verify`.** A flipped bit in a real bar file enters a sweep undetected." and ":49 `block::verify` has **no caller in `file.rs`**".
  - file.rs:2371 (bar read path): `self.verify_block_of(sidecar, block, at, image, &mut cache)?;`, and verify_block_of (file.rs:2452+) reads the `.crc` (`read_fully(crc, at, block.saturating_mul(4), &mut sum)?`) and calls `block::verify_through` ("the body of `block::verify`").
- Why it is wrong: A root-level handover still asserts in present tense that corrupted bars are swept undetected; the code verifies every block on read. Nothing marks the handover resolved.
- Repro: `grep -n 'verify_block_of(' crates/store/src/file.rs`.
- Suggested fix: Mark the handover RESOLVED with the D-number/commit that wired verify_block_of, or delete it (history in git).

### P1-15-08 low HANDOVER-web-backtest.md gives a wrong ledger stride (205) and a stale api dependency set
- Where: HANDOVER-web-backtest.md:50-52, 82-85, 125; crates/cli/src/results.rs:125,132,166; crates/api/Cargo.toml
- Evidence:
  - HANDOVER:82-85: "record 205 bytes ... Address of record *i* is `16 + i * 205`. Count is `(file_len - 16) / 205`."
  - results.rs:125 `pub const STRIDE: u64 = 261;`, :166 `const STRIDE_V2: u64 = 213;` — no 205 layout exists.
  - HANDOVER:50-52: "its dependency set is `core, pull, store, telemetry` ... there is no backtest page at all." — crates/api/Cargo.toml now declares `brutex_core, pull, store, telemetry, cli, vocab`; server.rs:16349 `.route("/backtest/run", axum::routing::post(crate::sweeprun::run))`, and `/backtest.json` is served.
  - HANDOVER:125: "(`.github/workflows/ci.yml:3134`)" — `PINNED='vocab indicators engine'` is at ci.yml:3974.
- Why it is wrong: A reader re-declaring the ledger reader from this document (which spells out offsets) would compute every record address wrongly; the "one-sentence problem" it hands over has been solved.
- Repro: `grep -n 'STRIDE' crates/cli/src/results.rs`; `grep -n '/backtest' crates/api/src/server.rs`.
- Suggested fix: Mark the handover done/superseded, or update the stride and drop the hard-coded offsets.

## Checked and consistent (no finding)
- §5 graph: every arrow vs all 13 `crates/*/Cargo.toml` `[dependencies]` path deps (api: core pull store telemetry cli vocab; cli: 9 arrows incl. store/pull/vocab; runner, pull, lake, store, costs, indicators, engine; core/vocab/greeks/telemetry depend on nothing); root `members` = 13.
- `FNO_UNDERLYINGS: [&str; 213]` less `FNO_INDEX_UNDERLYINGS: [&str; 5]` = 208 (test asserts 208).
- `PROVENANCE` (cli/lib.rs:337), `STORED_PROVENANCE` (:2772), `the_generated_and_stored_banners_make_opposite_claims` (:22307).
- `engine::column::Column::support` is a single `row.hits` fold; `drain` matches tracked §3.4 text; k=1 `HashSet<u32>` offered set pre-reserved; C-E-02/02b/09/11 exist.
- `indicators::Column::build` uses `for (index, bar) in bars.iter().enumerate()`; `PastPrefix` has no non-test caller.
- Run identity order in runner/src/identity.rs:695+: mask, direction, instrument(+kind), timeframe, params, data_digest, vocab_version, commit, feed — matches §3 rule 3.
- Vocab TABLE = 370 rows (matches "370 rows" in §5); no VIX row in vocab table.
- Gate 1 allowlist (rs|toml|md|lock|html|css; yml only .github; json only .claude; web .*); only `.claude/launch.json` uses the `.claude/` clause and it has two configurations; gates 7 (skips), 9, 9b, 17 (`vocab engine indicators runner`), 22 PINNED.
- Named items in §5: `vocab::expression_search::Cursor`, `CURSOR_BYTES`, `MAX_INSTRUCTIONS`, `cli::verify` with ConditionMask suffix check, candidate-universe digest over VOCAB_VERSION and TABLE, `pull::calendar::kind_of`, `pull::session::{Day,IstMoment}`, `pull::fold`, `cli::fold_audit`, `runner::synthetic`, `sweep-stored`/`sweep-all`, `window_range_percentile`, `GridProgress::tick`, `/vocab.json`, `/backtest.json` mask_words.

---
## Source: 16-docs-01-03.md

# P1-16: docs/01-architecture.md, 02-store-format.md, 03-vocabulary.md checked against the code

Repo `/home/claude/brutex-audit` at 331b05c. Static reads only.

Checked and found to match (no finding):
- Bar format §1-§3, §6: slot offsets 0..64 (header.rs:112-135), BRUTEXB2 / v2 / stride 56 / 73-record 4088-byte block / HEADER_LEN 32768 (format.rs:50-182).
- Overlay `BRUTEXB9` v9 stride 24 and Greeks `BRUTEXB8` v8 stride 80 (format.rs:276-282, 430-436). Sidecar extensions match path.rs:712-718.
- §12.1/12.2 BRUTEXRC v2 64-byte receipt (result_set.rs:34-46, 84-175).
- §12.4 BRUTEXCT v1 136-byte row and direction 1/2 (trades.rs:88-121, 225-331).
- §14 BRUTXPV4 864/856 (population.rs:71-117). §15 admission 544/480 (admission_store.rs:51-68). §16 BRUTXSL3 40/3112 (selection_v3.rs:64-73).
- §18, §20 (global_replay.rs:63-76, global_replay_v2.rs:82-118), §19 (stored_data_completeness.rs:47-57), §21 row and completion sizes (execution_disposition_v2.rs:61-81).
- §22-§26 strides (institutional_statistics.rs, candidate_universe.rs:118-141, pre_admission_data.rs:67-76, population_statistics_v2.rs:63-72, population_observations_v1.rs:85-91).
- docs/03: all 370 (bit, name) pairs match `vocab::table::TABLE`, checked by script. Kinds: 247 plain + 81 near = 328 live, 3 retired (6, 19, 25), 39 void (235-273). `VOCAB_VERSION` 3. `WORDS` 6.
- docs/01 §1 dependency table matches the manifests. The six shareable crates declare no external packages. cli takes `rayon` and has `batch::sweep_under`.

Already recorded, so skipped: §7 says "truncate" but the code does not (hunt-store-6). `table_of`/`KNOWN` (hunt-store-1).

---

### P1-16-01 medium docs/02 §11 (census `BRUTEXM`) leaves out format version 3, which is the version this build writes
- Where: docs/02-store-format.md:431, :470-472, :527, :560 vs crates/pull/src/manifest.rs:196, :497-514, :586-604
- Evidence:
  - doc :431 `## 11. The census file — \`BRUTEXM\`, versions 1 and 2`
  - doc :470 `` `b"BRUTEXM1"` or `b"BRUTEXM2"` — the last byte is the version ``
  - doc :471 `` `format_version` | `1` or `2`. **Selects the geometry.** ``
  - doc :527 `| 80 | 44 | reserved | zero |` (closes half, bytes 80..124)
  - doc :560 `This build **writes** version 2 only.`
  - code manifest.rs:196 `pub const FORMAT_VERSION: u16 = 3;`
  - code manifest.rs:598 `pub const V3: Self = Self::declared(3, MAGIC_V2, ENTRY_STRIDE, ENTRY_LEN, true);`
  - code manifest.rs:604 `pub const CURRENT: Self = Self::V3;`
  - code manifest.rs:497-507: the comment says `version 3 spends 25 of those 44 bytes`, with `const C_CONTRACT: usize = 16;` and `C_CONTRACT_N = C_CONTRACT + C_CONTRACT_LEN`. That is entry bytes 80..105 of the closes half, which the doc calls reserved zero.
- Why it is wrong: The doc says it is "the authority; the code follows it". It describes two versions. The code writes a third, which puts contract text and a length into bytes the doc calls reserved. The doc also says the magic's last byte is the version, but a v3 slot begins `BRUTEXM2` while carrying version 3. A reader built from §11 would refuse every census this build writes, or would treat a non-zero reserved area as corruption.
- Repro: Run any census publish with this build and dump bytes 8..10 of the newest slot. They read `03 00`, and bytes 0..8 read `BRUTEXM2`. Both contradict the table at :470-471. For an option or futures row, entry bytes 80..105 are non-zero.
- Suggested fix: Add an §11.x for version 3: magic stays `BRUTEXM2`, contract text at closes-half 16..40 plus its length byte, and the rest reserved. Change "writes version 2 only" to version 3, and correct the "last byte is the version" sentence.

### P1-16-02 medium docs/02 §13 describes frontier version 4 (272-byte rows), but the code writes version 7 (280-byte rows)
- Where: docs/02-store-format.md:703-742 vs crates/cli/src/frontier.rs:161, :184-187, :204-211, :434-484
- Evidence:
  - doc :703 `## 13. Ranked frontier — \`results/frontier.bin\`, version 4`
  - doc :711 `Each row is 272 bytes.`
  - doc :733 `| 195 | 5 | reserved, all zero; any non-zero byte is refused |`
  - doc :742 `| 264 | 8 | first eight BLAKE3 bytes over \`0..264\` |`
  - code :161 `const VERSION: u32 = 7;`
  - code :184 `pub const STRIDE: u64 = 280;`
  - code :434 `put(&[u8::from(self.rules.require_protective_exits)], &mut at); // 195   1 -> 196`
  - code :448-452: `min_fill_headroom_bp` written as an i32 at `// 196   4 -> 200`
  - code :484 `put(&self.rules.min_avg_rr_bp.to_le_bytes(), &mut at); // 264   8 -> 272`, followed by the seal at 272..280
- Why it is wrong: The doc says the frontier reader refuses any non-zero byte in 195..200 and puts the seal at 264. The code has used those bytes for two rules since v5 and v6, and since v7 has carried a ninth rule at 264 with the seal moved to 272. The authoritative layout is three versions stale. A reader built from it would reject every current file on version, stride or seal.
- Repro: Write one ranked run and read `results/frontier.bin` bytes 8..12. They read `07 00 00 00`. File length minus 16 is a multiple of 280, not 272.
- Suggested fix: Rewrite §13 for v7: bytes 195 (protective-exit flag), 196..200 (`min_fill_headroom_bp`, i32), 264..272 (`min_avg_rr_bp`), seal 272..280. Also note that v4 to v6 are refused by version.

### P1-16-03 medium Execution-parameter record grew 640 to 648 bytes while keeping format version 1 (a version changed in place), and docs/02 §17/§21 still say 608/640
- Where: crates/cli/src/execution_capability.rs:62, :68-74, :101, :105-110, :1350; docs/02-store-format.md:918, :928, :1165
- Evidence:
  - code :62 `const FORMAT_VERSION: u32 = 1;`
  - code :68-74 `// 616, NOT 608. The payload carries the evaluation-spec fingerprint, and that widened 155 -> 163 ...` then `const PARAMETER_PAYLOAD_BYTES: usize = 616;`
  - code :101 `const _: () = assert!(EXECUTION_PARAMETER_STRIDE == 648);`
  - code :1350 is a stale field doc in the same file: `/// Scalar parameter records (640-byte stride).`
  - doc :918 `| \`results/execution-parameters-v1.bin\` | \`BRUTXEP1\` | 608 | 640 |`
  - doc :928 `| 192 | 155 | exact evaluation-spec fingerprint |`
  - doc :1165 `| \`results/execution-disposition-parameters-v2.bin\` | \`BRUX2PR1\` | 608 | 640 |` (it "reuse[s] the exact sealed V1 scalar-parameter ... codecs")
  - The upstream `indicators::column::EVALUATION_SPEC_FINGERPRINT_V1_LEN` also changed length (155 to 163) while still named V1.
- Why it is wrong: CLAUDE.md §3 rule 8 says store format versions are never changed in place. Here magic `BRUTXEP1`, version 1 now names two geometries: 640-byte records with the payload offsets in the doc, and 648-byte records with every field after byte 192 moved by 8. A v1 file written before the change is refused as a stride mismatch instead of being read at its own stride or refused by version number. The authoritative doc and an in-file comment still give the old geometry. The same applies to the BRUX2PR1 v2 parameter file, which reuses the codec.
- Repro: Make a `results/execution-parameters-v1.bin` with the doc's layout (24-byte header with stride 640, then one 640-byte record whose seal covers 0..608). Open it through the execution-capability reader. It refuses on stride, although magic and version are the ones the doc and the code both claim.
- Suggested fix: Mint new versions/magics for the 648-byte parameter record (and for the V2 disposition parameter file), and keep the 640-byte v1 readable or explicitly retired by number. Then update §17's offsets (fingerprint 192..355, later fields +8), §21's sizes, and the comment at :1350.

### P1-16-04 low Several production file formats have no byte layout in any doc, despite 02's claim to be the authority for bytes on disk
- Where: docs/02-store-format.md:3 and CLAUDE.md §10 ("`docs/02-store-format.md` | bytes on disk") vs:
  - crates/cli/src/results.rs:60 `MAGIC = *b"BRUTEXRS"` (`runs.bin`, which 02 §12.3 calls "the public commit marker")
  - crates/cli/src/population.rs:68 `ROW_MAGIC = *b"BRUTEXPP"`
  - crates/cli/src/live.rs:124 `*b"BRUTEXLV"`
  - crates/cli/src/pre_admission_data.rs:94, :106-109 (`BTX-PREADMIT-V2\0`, `pre-admission-data-v2.bin`, stride 812)
  - crates/cli/src/execution_v4.rs:83-86, :120 (`BTX-EXV4-*`, `execution-parameters-v4.bin`)
  - crates/cli/src/global_replay_v3.rs:83-87, :110 (`BTX-GRV3-*`, `global-replay-money-v3.bin`)
  - crates/cli/src/population_statistics_v3.rs:72-73 (`BTX-POPSTATS-V3\0`, `population-statistics-v3-audit.bin`)
- Evidence: `grep -rn 'BRUTEXRS\|BRUTEXPP\|BRUTEXLV\|BTX-PREADMIT-V2\|BTX-EXV4\|BTX-GRV3\|BTX-POPSTATS-V3' docs/` returns nothing. `grep -rl` over docs/ for the file names `pre-admission-data-v2`, `execution-parameters-v4`, `global-replay-money-v3` and `population-statistics-v3-audit` also returns nothing. Other successors (Selection V6, Global Replay V4) are handed off from 02 to docs/21, so delegation is the pattern, but these have no description anywhere.
- Why it is wrong: The repository's rule is that a doc owns the bytes. These append-only formats, including the ledger that marks every run public, are described only by Rust constants, which is the situation 02 §11 says it was written to end ("a Rust comment the authority for bytes on a disk").
- Repro: The greps above.
- Suggested fix: Add sections to 02, or links to a sub-document, for `runs.bin`, population rows, live, Pre-Admission V2, Execution V3/V4, Global Replay V3 and Population Statistics V3.

### P1-16-05 low docs/01 §3 data flow says bar files are read through a read-only mmap with pointer-arithmetic reads, which no build does
- Where: docs/01-architecture.md:207, :286-288
- Evidence:
  - doc :207 `store::open (read-only mmap)`
  - doc :286-288 `1. **The disk is touched once per slice per launch.** After the initial open, every bar read is pointer arithmetic against a mapping that is already resident.`
  - code crates/store/src/lib.rs:54 `#![forbid(unsafe_code)]`. docs/02 §4 (:126-131) says: `No crate maps a bar file ... A read is BarFile::read_record → read_row → read_fully → Positional::get, which is FileExt::read_at — one pread of one record's bytes.` (D-0790)
- Why it is wrong: D-0790 corrected 02 and the store module docs, but 01's data flow and its first "property that matters" still state the opposite. Each read is a `pread` plus a block CRC check (`verify_block_of`). It is not arithmetic over resident pages, and the disk is touched on every cold block.
- Repro: `grep -rn memmap crates/*/Cargo.toml` returns nothing, and crates/store/src/lib.rs:54 forbids the `unsafe` a mapping needs.
- Suggested fix: Replace with "`BarFile` open (positional reads, no mapping)", and restate property 1 as one `pread` plus one block CRC per read.

### P1-16-06 low docs/01 §5 says the store commit counter is "published with a release store"; there is no atomic in crates/store
- Where: docs/01-architecture.md:385
- Evidence: doc `| \`store\` append | one writer, positional writes, commit counter published last | the counter, published with a release store |`. `grep -rn -E 'Ordering::|Atomic' crates/store/src/*.rs` returns no hits outside tests. The counter is published as one 64-byte header-slot `pwrite` followed by `sync_all` (02 §5 :155-159, header.rs `Commit`).
- Why it is wrong: The doc names a memory-ordering mechanism that does not exist. Cross-process visibility comes from the slot write, its CRC and the generation choice, which is a different guarantee from a release store.
- Repro: The grep above.
- Suggested fix: Change the cell to "the counter, published by one checksummed header-slot pwrite after the records are synced (02 §5)".

### P1-16-07 low docs/01 §2 still says display rules in `core` are "compiled twice, once native and once to WASM"; nothing compiles to WASM
- Where: docs/01-architecture.md:188-191
- Evidence: doc `The payoff: every display rule — how a price renders, how a percentage is computed, what counts as a valid mask — lives in \`core\` and is compiled twice, once native and once to WASM. One implementation, two targets, no drift between what the server believes and what the browser shows.` The same section (:177-183) says there is no `crates/web`. `git ls-files web` has no wasm artifact or Rust crate manifest, and the browser formats in JS (e.g. web/src/lib/*.js).
- Why it is wrong: The section already concedes the WASM crate is gone, then claims its payoff as current. There is a second implementation of display rules (JS), which is the drift the sentence says cannot happen.
- Repro: `find web -name Cargo.toml -o -name '*.wasm'` (excluding node_modules) returns nothing.
- Suggested fix: Delete the payoff paragraph, or restate it as historical and point to `/vocab.json`-style server-served tables as the current anti-drift mechanism.

### P1-16-08 low docs/01 §4 says "There is no catalogue to consult" and that directory listings are never globbed on a read path; `store::catalog::walk` does exactly that for `sweep-all`
- Where: docs/01-architecture.md:366-373; docs/01-architecture.md:205
- Evidence:
  - doc :366 `There is no catalogue to consult, no index to rebuild, no registry that can disagree with the filesystem.`
  - doc :371 `Directory listings are never globbed on a read path.`
  - code crates/store/src/catalog.rs:25 `[\`walk\`] is **O(entries under \`root\`)**` and :279 `pub fn walk(root: &Path) -> Result<Holdings, CatalogError>`, called on the stored-sweep path at crates/cli/src/batch.rs:318 `let holdings = catalog::walk(root)...`
  - Also doc :205 draws the path as `bars/<exch>/<seg>/<sym>/<tf>/<yyyy-mm>.bin`, leaving out the leading vendor segment that §4 itself (:358) and `StorePath` (D-0019) require.
- Why it is wrong: The bar-reading path for multi-month sweeps now starts with a directory walk, through a module literally named `catalog`, whose own doc says it was added because the store was "address-only". The data-flow path template is also wrong by one segment.
- Repro: `cli sweep-all` reaches `catalog::walk` (batch.rs:318) before any `BarFile` open.
- Suggested fix: Say that single-month reads are address-only and that `sweep-all` enumerates through `store::catalog::walk` (O(entries), a setup step). Add `<vendor>/` to the :205 template.

---
## Source: 17-docs-invariants.md

# P1-17 — docs/04-invariants.md: named proofs vs the code (commit 331b05c)

## Method
- Script (`p1/w17/`): pulled every backticked `a::b...` token (2,774) and every bare snake_case token with 3 or more underscores from `docs/04-invariants.md`, with line numbers. Built an inventory of all `fn <name>` under `crates/` (14,245 names). Classified each token's last segment as found or missing. Also checked each cited name against the 13 `#[ignore]`d fns.
- Result: 21 two-segment-plus tokens and 6 bare tokens were missing. Each one I checked by hand. Almost all are history prose, allowlisted rows (P-03, X-13, X-01), module or file names (`cli::sweep_wiring_tests`, `exact_minute_orb_gap_readiness`), or are already recorded in `/mnt/project-files/audit-20261003-workspace/testgaps.md`:
  - testgaps-2: ER-01, `grid::every_trade_ends_by_exactly_one_of_the_five_exits`
  - testgaps-3: MR-23, `the_cross_verification_is_also_bound_to_a_press`
  - testgaps-4: AD-02, `exact_codecs_bind_every_status...`
  - testgaps-5: ED-01, `parameterized_fixture_proves_banknifty_hourly_top25_capacity`
  - testgaps-7: the `#[ignore]`d proofs at :1451 and :4168
  - testgaps-10: FX3-11, which is compile-time only

  All of these are still unfixed at 331b05c, and I have not repeated them below. One phantom is new: P1-17-01.
- Read the cited test bodies for 50 rows, newest first. Coverage was every row in "Audit fixer 2/3/4 follow-ups" (AFX-01..16, FX3-01..18, AUF-01..05), plus PIF-08/09, APIC-07/08, C-V53-02, TC-04, M-36, CUH-04/06, D-1190, and D-1142/1144/1175/1180/1193/1196 and AF-O1API3-a. The AFX, FX3 and AUF sections did not exist at 1087e54, where testgaps ran. Most tests do prove their rows. The ones below prove less than their `✓` row claims.

### P1-17-01 low Prose cites a test that does not exist, and states the opposite of what the code does
- Where: docs/04-invariants.md:2276-2282
- Evidence: the doc says: "The broker path refuses every target that names a set — tiers included … because `pull::vendor::HttpSpec` carries no request-parameter map; `api::server::broker_target_tests::only_the_swept_target_names_a_single_instrument_this_path_can_reach` pins that to one target". No such fn exists anywhere (`rg only_the_swept_target` finds only this doc line). The module's one test, crates/api/src/server.rs:31146 `fn the_broker_path_addresses_a_set_and_no_target_guard_stands_in_its_way`, asserts the reverse: `assert!(!body.contains("asked.target != ingest::SpotTarget::"), "broker_window refuses a target again. …")`. Its doc comment says "THE TARGET GUARD IS GONE … Both halves were false by the time it was removed".
- Why it is wrong: the paragraph tells the reader that tier targets cannot be fetched from the broker, and names a phantom test as the guard. The code fetches sets per member, and the real test pins that the guard is absent. Gate 10 does not see this because the citation sits in prose, not in a table row.
- Repro: `rg -n only_the_swept_target crates` returns nothing. Then read server.rs:31123-31200.
- Suggested fix: rewrite the paragraph to say the guard was removed, and cite `the_broker_path_addresses_a_set_and_no_target_guard_stands_in_its_way` and its decision entry.

### P1-17-02 low APIC-07 is marked `✓` for "one WARN event and a stderr line", but no test drives that branch
- Where: docs/04-invariants.md (APIC-07 row, "Audit fixer" block before AFX); crates/api/src/server.rs:17753-17778, 21051
- Evidence: the row says: "**A serve-lock stamp that fails is never silently discarded.** A failed stamp whose file can be emptied serves with one WARN event and a stderr line naming the path and the host's error". The cited test calls only the pure decision fn `stamp_outcome(full, || Ok(()), path)` and checks the returned string. It also runs the `/dev/full` refusal end to end. The WARN and stderr side lives in `stamp_serve_lock`: `if let Some(warning) = stamped { let _noted = telemetry::emit(...Level::Warn, "api.serve", ...); warn_line!("{warning}"); }`. No test reaches it, and crates/api/src/emitted.rs:1665-1672 lists that emit as unreachable ("No fixture produces that split").
- Why it is wrong: if someone deleted the `if let Some(warning)` block, the stamp failure would be silently discarded, which is exactly what the row's headline forbids. The cited test would still pass. That limit is admitted in emitted.rs but not in the row, which wears `✓` instead of `◐`.
- Repro: delete server.rs lines 17767-17777 (the `if let Some(warning)` block) and run `cargo test -p api a_serve_lock_stamp_that_fails`. It still passes.
- Suggested fix: inject the event sink or writer into `stamp_serve_lock`, or extract the "warning means emit plus line" step into a testable fn and assert both. Until then, mark the row `◐` and state the unproven clause.

### P1-17-03 low C-V53-02 claims "each failure emits one `cli.output` event", and neither cited test checks it
- Where: docs/04-invariants.md (C-V53-02 row); crates/cli/src/lib.rs:3137-3172; crates/cli/src/operator_boundary_tests.rs:509; crates/cli/tests/binary.rs:290
- Evidence: the row says: "Its output goes through `cli::deliver`: a closed pipe keeps the run's exit code and is said on stderr; any other write failure turns `OK` into `FAILED`; each failure emits one `cli.output` event." `deliver` does call `note(&telemetry::Event::new(Level::Warn, "cli.output", ...))`. But `a_report_that_cannot_be_written_is_said_and_never_panics` asserts only the returned codes and the stderr text. `a_closed_stdout_is_said_on_stderr_and_never_panics` asserts only the exit code and stderr. `rg 'cli\.output|the report was not shown whole' crates --glob '!crates/cli/src/lib.rs'` finds nothing.
- Why it is wrong: removing the `note(...)` call breaks no test, so the event clause of a `✓` row has no proof.
- Repro: delete lib.rs:3149-3161 (the `note(...)` call) and run `cargo test -p cli a_report_that_cannot_be_written`. It passes.
- Suggested fix: in the binary test, point telemetry at a scratch log and assert exactly one `cli.output` WARN line. Or drop the clause from the row.

### P1-17-04 low FX3-01's test exercises the rollback helper directly, so "written in one call" and the production wiring are both unpinned
- Where: docs/04-invariants.md (FX3-01 row); crates/cli/src/anchored_search_lineage_v4.rs:972-974, 1001, 1637-1639, 2568
- Evidence: the row says: "A Search Lineage V4 pair's members are written in one call; a failed member or Completion write truncates back and leaves the file byte-identical and readable (D-1620)". The test calls the private helper with an injected writer: `append_with_rollback(&mut file, &[0x5a; 700], |file, raw| { file.write_all(raw.get(..333)...)?; Err(...) })`. Production goes through `append_raw` (`append_with_rollback(file, raw, Write::write_all)`) at lines 974 (members, `None => (encode_members(&members)?, 2)`) and 1001 (Completion). No test calls `append_raw` or `append_locked` under a failing write, and none counts member writes per pair.
- Why it is wrong: two regressions the row rules out would both pass every test. One is restoring two `append_raw` calls, one per member: the D-1620 crash window. The other is `append_raw` calling `file.write_all` directly, which removes the rollback. `full_synced_tail_is_retryable…` checks orphan recovery, not the one-write shape.
- Repro: change line 972 to append `encode_member(&members[0])` and then `encode_member(&members[1])` in two `append_raw` calls, or change `append_raw` to `file.write_all(raw).map_err(...)`. The anchored_search_lineage_v4 tests stay green.
- Suggested fix: add a source-shape assertion like the repo's other ones (one `append_raw(&mut self.member_file` in `append_locked`, and `append_raw` delegates to `append_with_rollback`). Or route `append_locked` through an injectable writer and count calls.

## Checked and found sound (sample)
- AFX-01..16, FX3-02..10, 12..18, AUF-01, 02, 04, 05 (AUF-03 JS tests exist and run under `node --test web/tests/*.test.js`, ci.yml:7698).
- PIF-08, PIF-09, TC-04, M-36, CUH-04, CUH-06, D-1190, D-1142, D-1144, D-1175, D-1180, D-1193, D-1196, AF-O1API3-a.
- Gaps I noticed but did not raise: AFX-12 asserts "reaches the parser" only as "not a 413", and FX3-18's `families` bound is checked in `Group::new`, not `admit`. Both are below the bar for a finding.

---
## Source: 18-docs-05-10.md

# P1-18: docs/05 (newest entries), 06, 07-plan, 07-o1, 09-verify, 10-shared-core versus code

Repo: /home/claude/brutex-audit @ 331b05c. Static reading only.

### P1-18-01 medium docs/09-verify.md: the "exit 0 on all six" procedure fails at HEAD in four places
- Where: docs/09-verify.md:30-35 (§2), :50-58 (§4), :62-66 (§5)
- Evidence:
  - §2 says `find crates -name build.rs` "must print nothing. A hit in either is a §2 build failure". At HEAD it prints `crates/cli/build.rs`. That file exists on purpose: its header says "CI gate 13 layer 3 bans a build script anywhere in the tree … This is that escape, taken, with the entry."
  - §4: `grep -c 'feeds.active *=' web/src/routes/+page.svelte` targets a file that no longer exists. `web/src/routes/` holds `+page.js`, a redirect to `/terminal` whose header says "`routes/+page.svelte` held the Markets console … the Markets page moved". The grep fails with "No such file or directory".
  - §4: `grep -c 'feeds.active *=' web/src/routes/ingest/+page.svelte` "must print 0". It prints 1, matching `{#if feedsChosen.includes('zerodha') || feeds.active === 'zerodha'}` (ingest/+page.svelte:8409). The regex `feeds.active *=` also matches a `===` comparison, so the check cannot tell a comparison from an assignment.
  - §4 says "`/db` prints 2". At HEAD `grep -c 'feeds.active *=' web/src/routes/db/+page.svelte` prints 0.
  - §5: `awk '/^<style/,/^<\/style>/' web/src/routes/+page.svelte | grep -coE '#[0-9a-fA-F]{3,8}'` reads the same missing file, so it checks nothing.
- Why it is wrong: The page says "Exit 0 on all six means it is safe to press Run api" and frames itself as checkable "without trusting anyone". As written, an operator following it gets a hit for §2, a hard error for §4/§5, and a wrong count for the ingest page. So the document either blocks a correct tree or, worse, teaches readers to ignore its failures.
- Repro: From the repo root, run the §2, §4 and §5 commands verbatim. Outputs: `crates/cli/build.rs`; `grep: web/src/routes/+page.svelte: No such file or directory`; `1`; `0` for /db.
- Suggested fix: Make §2 allow `crates/cli/build.rs` by name (as gate 13's allowlist does). Point §4/§5 at the pages that exist now (`terminal`, `markets`). Use an assignment-only regex (`feeds\.active *=[^=]`) and fix the expected /db count.

### P1-18-02 low docs/10-shared-core.md §3 says 11 sources and 272 positions; §1 and the code say 12 and 328
- Where: docs/10-shared-core.md:131-134 (§3, "The set of positions is derived"), contradicting :21 (§1 table)
- Evidence:
  - doc §3: "`indicators::evaluator::Evaluator::positions()` is the union of the ELEVEN position sources' own `positions()` … — **272 positions** in total."
  - doc §1: "`indicators` | **`vocab` only** | Candle in, condition bits out. Twelve position sources, 328 positions."
  - code, crates/indicators/src/evaluator.rs:1396: "That is 328 positions today". At :1741 the test asserts `all.len() == 238 + CROSSINGS.len()*5 + 5`, commented "328 = 238 measured by a module + 85 derived from the mask + 5 weekdays".
  - `positions()` (:1401-1440) unions 8 modules, 276-279, the weekday rows, CROSSINGS and the current-day Fib range. That is 12 sources, not 11.
- Why it is wrong: The document consumers are told to rely on gives two different counts for the same derived set. The §3 number is stale by 56 positions and one source. It also mis-describes the paragraph's whole point, that the derived set and its count cannot drift. `tests/module_doc_counts.rs` checks the crate header, not this file, so nothing caught it.
- Repro: `grep -n '272 positions\|ELEVEN position' docs/10-shared-core.md` against `grep -n '328' crates/indicators/src/evaluator.rs`.
- Suggested fix: Change §3 to "twelve … 328 positions", or drop the literal and cite `tests/evaluator_position_count.rs`. Consider adding this file to the doc-count test.

### P1-18-03 low docs/10-shared-core.md §2 tells consumers to pin a tag that does not exist
- Where: docs/10-shared-core.md:83-88
- Evidence: `vocab = { git = "https://github.com/SJParthi/brutex", tag = "vocab-v0.1.0" }` (same for `indicators`, `engine`), followed by "**Pin to a tag, not to a branch.**" `gh api repos/SJParthi/brutex/tags --jq length` returns `0`, and `releases` returns `0`. `git tag` in the clone lists nothing.
- Why it is wrong: The page calls this snippet "the whole mechanism" for a downstream consumer such as `tickvault`. Copied as written, `cargo` fails to resolve the dependency. The only way to make it build is to pin a branch, which is exactly what the page forbids.
- Repro: Put the snippet into a scratch crate's Cargo.toml and run `cargo fetch`. It fails because tag `vocab-v0.1.0` is not found.
- Suggested fix: Create and push the `vocab-v0.1.0` tag at the commit the shared contract describes. Otherwise mark the snippet as not yet possible and record that in a decision.

### P1-18-04 low docs/07-plan.md §2 DONE cites 17 commit hashes, and none resolves in the repository
- Where: docs/07-plan.md:113-126 (§2 DONE table), :119/:227, :360
- Evidence: The status legend (:16) defines DONE as "Landed on `feat/pull`, gates green, and named here with its commit". The cited hashes are `a8cadb4 c1ea7ab bf8f86e 0596621 a91026e d2fa20c f8be537 44ce731 2efdce7 6602a9a 4a5953f f34875b 6dd2e97 b96294f 7661a61 62274e0`. §7.4 also cites `dbaafd6` and `10b11b2`. `git cat-file -t <hash>` returns "Not a valid object name" for all 17+ in a full, non-shallow clone (`git rev-parse --is-shallow-repository` = false, 606 commits, origin/main and origin/final/all-fixes fetched). `crates/store/tests/cited_commits.rs` enforces this rule for docs/04, docs/06 and four D-entries ("A measurement cited to a commit no reader can check out cannot be checked (CLAUDE.md §3 rule 6)"), but it does not read docs/07-plan.md.
- Why it is wrong: Every DONE row's proof is a commit no reader can check out. That is the class of defect cited_commits.rs exists to refuse, and it survives here only because this file is outside the test's reading list. §7.4's argument, "`git log -S StemGroupSymbol` returns exactly two commits; the older, `dbaafd6`…", cannot be reproduced.
- Repro: `for h in $(grep -oE '`[0-9a-f]{7,12}`' docs/07-plan.md | tr -d '`' | sort -u); do git cat-file -t $h; done` reports invalid for every hash.
- Suggested fix: Replace each hash with the decision that carried the change, or with the squash commit on main. Add docs/07-plan.md to `cited_commits.rs`'s reading list.

### P1-18-05 low docs/07-plan.md §5 and §7.1 call descriptor gaps "OPEN" that the code has closed
- Where: docs/07-plan.md:211-221 (§5), :304-312 (§7.1), :338 (§7.2 "No zip reader" is still true and is not part of this finding)
- Evidence:
  - §5: "Three descriptor fields **cannot express an arbitrary broker at all**". It lists `HttpSpec::bars_path` as "a fixed string", `AuthScheme` as `Raw | Bearer` with no second secret, and `TimestampEncoding::IsoDateTimeText` as unable to carry `+0530`, then: "All three compile as lies … It is not N-feed for an arbitrary broker".
    - Code, crates/pull/src/vendor.rs:1838: "[`HttpSpec::bars_path`] was a `&'static str` … `docs/07-plan.md` §5 predicted exactly the vendor that would break it … Zerodha is that vendor". The path is now a typed segment list.
    - vendor.rs:1569-1575: `AuthScheme` has a third variant, "a prefix, a **first** secret, a separator, and a **second** secret … `docs/07-plan.md` §5 named this exact shape".
    - vendor.rs:2025 `IsoDateTimeOffset`, used by Zerodha at :5401: "`2017-12-15T09:15:00+0530` — it carries its own zone … D-0135".
  - §7.1: "The archive half of the descriptor table has **zero non-test consumers** … `run_local` … never asks the descriptor anything: it hardcodes `Columns::Gdfl`". Code, crates/api/src/server.rs:10781-10829 (`run_local`): "This read `Columns::Gdfl` … for EVERY archive feed … That is no longer true". Then `let pull::vendor::Transport::LocalArchive(archive) = feed.descriptor().transport` … `archive.layout(Segment::Fno)` … `columns: layout.shape`. The line numbers §7 cites (vendor.rs 2517/2573, server.rs 3950/3999-4005, the test module "at 2645") also no longer match: `run_local` is at server.rs:10719, and vendor.rs's `#[cfg(test)]` is at 5831.
- Why it is wrong: The plan is the file that records "what is not built". It still lists as open, and as the "root cause [that] re-reads every row below", gaps the code says it closed and that cite this very section. A reader planning work off §5/§7 would redo finished work. That is stale status, not history. Not covered by gaps-13, which names R-4, §9.3, §10 #11 and §11 only.
- Repro: Read the quoted lines of vendor.rs and server.rs against docs/07-plan.md §5 and §7.1.
- Suggested fix: Mark the three §5 bullets and the §7.1 "zero non-test consumers / hardcodes Columns::Gdfl" row closed, naming the decisions. Keep only what is still literal (the FNO segment and `PriceScale::Paisa` in `run_local`), and refresh or drop the line numbers.

### P1-18-06 low docs/07-plan.md §6 says a flat C-V-02 ratio proves branchless code; D-1436 and docs/04 say it cannot
- Where: docs/07-plan.md:263-275 (§6 table and the paragraph after it); also R-9 at :101
- Evidence:
  - §6: "The 0.996× is the one that matters: an early-exit loop would return sooner on a candidate failing in word 0, so a flat ratio across the six words is what says the branchless implementation is the one that actually runs."
  - docs/04-invariants.md:1904 (C-V-02): "**This row is evidence, not the guard against an early-exit loop:** audit findings ET-o1-proof-coverage-3 and -13 measured an early-exit word loop in `hits` at 1.093×, … 2.056× for words 1 to 5, under the 3.0× ceiling … The guard is the source-shape unit test `vocab::mask::hits_does_the_same_work_for_every_input` … D-1436".
  - In the same §6 table, "Duplicate rejection | `C-E-04` | 0.842× per bar" names a bench that, per docs/04-invariants.md:2018, times a whole ladder walk. The current figure there is 0.337× (:2078). "Result append … 0.633×–0.811×" does not match C-E-11's recorded 1.015× / 1.204× (:2025, :2076).
  - "Condition lookup … 1 bit → 234 bits" and "k=1 → k=234" carry the stale live count. 07-o1 and docs/04 both say it is 328 now.
  - R-9: "**All eleven crates covered as of D-0103**". There are 13 crates, each with `benches/ratio.rs`.
- Why it is wrong: §6 is the plan's answer to "how is O(1) ensured". It still makes the claim D-1436 withdrew, that the ratio detects an early-exit loop, and quotes figures that docs/04 has since replaced. A reader trusting §6 would conclude the bench guards something it does not.
- Repro: Compare docs/07-plan.md:263-275 with docs/04-invariants.md:1904, :2018, :2025, :2076-2078.
- Suggested fix: Replace the paragraph with D-1436's statement that the source-shape test is the guard. Point the table at docs/04's current rows instead of copying numbers. Say "thirteen crates" in R-9.

### P1-18-07 low docs/07-o1-architecture.md quotes the hit-only probe bound as layer 4's whole bound; a miss walks up to 12
- Where: docs/07-o1-architecture.md:46 (layer 4 row), :79 (measured table "Universe membership | worst probe 6 (750 members) / 7 (213 members)"), :190-192 ("Layer 4's probe length is asserted at `<= 8` and printed")
- Evidence: crates/core/src/universe.rs:4934-4972, `a_miss_probes_further_than_a_hit_and_its_bound_is_measured_too`: "The `<= 8` that test pins was quoted -- in this file's header, on `MemberIndex`, on `position` and on `nse_isin` -- as though it covered both. It does not … `of_equity` probes six tables and the question it is asked most is about a name outside every tier … every one of them is a miss six times over." The test then asserts `worst <= 12` for misses ("12 is one step above the worst measured"). Also :4889-4892: "Every symbol here is one the table HOLDS … this test says nothing about a miss."
- Why it is wrong: The O(1) architecture document still gives the hit bound (≤ 8, worst 6/7) as the layer's proof. The code says quoting it that way is "wrong in the unsafe direction", because the most common call (`of_equity` on a non-member) pays six misses at up to 11-12 probes each. The four in-code quotes were corrected; this document was not.
- Repro: Read universe.rs:4934-4972 against the 07-o1 lines above.
- Suggested fix: State both bounds in the layer 4 row and the measured table, citing the miss test: hit ≤ 8 (worst 7), miss ≤ 12 (worst 11), and `of_equity` = six misses.

## Also checked, no finding
- Duplicate D-numbers (scripted over `^#+ D-NNNN` headings, 987 entries): D-0076, D-0077, D-0078, D-0370 and D-0372 each head two entries. All five are already recorded (D-0104 for 76-78, decisions :38074-38141 for 370/372) and are pinned by a test that allows only those five. No other duplicate. Numbering is non-monotone in file order (D-1450 renumbered collisions), which is not a defect.
- Newest ~60 decisions (D-1182 … D-1508, D-1620 … D-1642): each named test, constant and code change checked by grep. All present, including D-1484 `cli::deliver` (no print! in main.rs), D-1497 `try_with_capacity`/`repeated`, D-1503 step-runs in gate 14 layer 5, D-1504 `npt -eq pts`, D-1505 the HashSet distinctness in the runner test, D-1506 `store/tests/docs.rs`, D-1507 `withheldDays`, D-1508 `MAX_STORE_READ_CONCURRENT = 8` and both tests, D-1623 the frontier refusal at frontier.rs:1083, D-1625 `i128` buckets, D-1626 `continue`, D-1629 `shrink_to_fit`, and D-1637 `RECORDS_PER_CANDIDATE = 3`. Six names absent from code (e.g. `evaluate_with_attested_replay`) are explicitly renamed in their own entries (D-0990 fold note, D-1198).
- docs/06-limits.md: all 215 long snake identifiers and 514 `crate::path` references resolve, except two that the text itself marks as historical. All 25 constants quoted with values match the code (e.g. MAX_MANIFEST_BYTES = 32,768 + 2,097,152 × 128 = 268,468,224; ITERATION_CEILING = 128 + 2 = 130). Every §N cross-reference into 06-limits resolves.
- docs/10-shared-core.md: the six shareable crates' dependency sets match their manifests (zero external packages). `size_of::<Evaluator>() <= 1760` matches. NEXT_FREE = 370 and WORDS = 6 match.
- docs/07-o1-architecture.md: CEILING_PERMILLE = 3_000, the api bench C-14..C-17, M-19's test, AGENTS.md, and the 750/213 table sizes all exist.

---
## Source: 19-pull-security.md

# P1-19 crates/pull security versus CLAUDE.md section 8

Audited at /home/claude/brutex-audit (331b05c). Static read only.

### P1-19-01 medium A credential value that is not a legal HTTP header value (e.g. a trailing newline) is never refused; each request fails as a reqwest builder error, gets classed as a transport blip, is retried 6 times per instrument, and the credential law never fires

- Where: crates/pull/src/http.rs:496-505 (`header_value`), crates/pull/src/http.rs:2889-2895 (`window_async` send), crates/pull/src/http.rs:2575-2581 and 2743-2747 (`post_json`, `Discovery::get`), crates/pull/src/ssm.rs:880 (empty-only check), caller crates/api/src/server.rs:10098 and the retry `step` at crates/api/src/server.rs:9533-9540
- Evidence:
  - `(AuthScheme::Raw, None) => Ok(held.token),` / `(AuthScheme::Bearer, None) => Ok(format!("Bearer {}", held.token)),`. The value is stored as a `String` and is never checked as a header value. `HttpSource::new` refuses only a scheme/arity mismatch.
  - ssm.rs:880 `if value.is_empty() {`. That is the only check on the Parameter Store value. It is returned untrimmed, so `"tok\n"` and `" "` both pass. api server.rs:10098 `let token = read("access-token").await?;` hands it straight to `Credential::token`.
  - http.rs:2884-2895: `self.wait_for_permit()...?;` and then `builder.header(name, value).send().await.map_err(|why| FetchError::TransportFailed { detail: format!("{url} was not reached: {why}") })`
  - api server.rs:9533-9540: `// Nothing was answered, so it is a transport blip` then `None => { if attempt >= THROTTLE_ATTEMPTS { return Step::Exhausted; } Step::Again { wait_ms: 250 * attempt as u64 * attempt as u64, ...`, with `THROTTLE_ATTEMPTS: u32 = 6` (server.rs:9220).
- Why it is wrong: reqwest 0.12's `RequestBuilder::header` converts with `HeaderValue::try_from`. That refuses control bytes such as `\n`, stores a builder error, and `send()` then returns it without opening a socket. So a credential stored with a trailing newline, which is a common `put-parameter --value file://...` artefact, has these effects:
  - Every instrument spends a governor permit.
  - It is retried six times with quadratic backoff, as if it were a network blip.
  - It is reported as "<url> was not reached: builder error".
  - It never yields a 401/403, so the section 8 dead-token re-read and halt in `credential_law` is never reached.

  This is a section 4 failure that does not name its real reason: the credential is malformed, but the report says the network is down.
- Repro: In a unit test, build `HttpSource::new(dhan_spec, Credential::token("tok\n".into()))`. It returns `Ok`. Then point `window_async` at a loopback server and see an `Err(FetchError::TransportFailed { detail })` whose detail contains "builder error", with the loopback server never receiving a connection. Through `with_retry`, the same request is attempted `THROTTLE_ATTEMPTS` times. Expected instead: a construction-time refusal that names the credential, with no permit spent.
- Suggested fix: In `HttpSource::new`, validate `header_value` once with `reqwest::header::HeaderValue::from_str` (and mark it sensitive). Refuse with a credential-classed `FetchError` that names the field and not the value. Also refuse whitespace-only or whitespace-padded values in `ssm::get_parameter`, next to the existing empty check.

### P1-19-02 low Contracts discovery sends a credentialed request built from the vendor's own unvalidated, unencoded expiry string, validates it only afterwards, and then reports "its contracts were not asked for"

- Where: crates/pull/src/chain.rs:303-341, crates/pull/src/fno.rs:222-232 (`join`)
- Evidence:
  - chain.rs:303 `let mut expiries = fno::names(&body, field)...` takes vendor strings, and the only filtering is dedup.
  - chain.rs:318-322 `let url = fno::contracts_url(&spec, &keyed)...?; let body = from.get(&url).await...?;` sends first.
  - chain.rs:336-341 `let Some(keyed_expiry) = iso_expiry(&keyed.expiry) else { chain.unreadable.push(format!("expiry {:?} (its contracts were not asked for)", keyed.expiry)); continue; };` validates after the send.
  - fno.rs:228-231 `out.push(if first { '?' } else { '&' }); ... out.push_str(p.name); out.push('='); out.push_str(&value);` does raw concatenation with no percent-encoding. The bars path uses `reqwest::RequestBuilder::query` (encoded) or `path_safe`. Discovery uses neither.
- Why it is wrong: A malformed or hostile expiry entry such as `2025-03-27&underlying=BANKNIFTY` or `x#` is spliced into the query string. It is then sent to the vendor carrying the live credential and spends a governed request. Only after that is the entry found unreadable. The unreadable note then states as fact that the request was never made. The host cannot change, because the base URL is constant, so the credential cannot leak to another host. What goes wrong is the request integrity and the accuracy of the receipt.
- Repro: Drive `chain::month` with a fake `Discovery` whose expiries answer is `{"expiries":["2025-03-27&underlying=X"]}`, and record the URLs it receives. A second `get` arrives with `...expiry_date=2025-03-27&underlying=X`, and `chain.unreadable` contains "(its contracts were not asked for)".
- Suggested fix: Move the `iso_expiry` check ahead of `contracts_url`/`get`, so only a strict `YYYY-MM-DD` ever reaches the wire. Also percent-encode values in `fno::join`, or build discovery requests with `.query()`.

### P1-19-03 low The two reads on the credential path are unbounded, which is the defect D-0036 fixed for credentials.toml

- Where: crates/pull/src/ssm.rs:355-356 (`from_credentials_file`), crates/pull/src/ssm.rs:832-835 (`get_parameter` body)
- Evidence:
  - ssm.rs:356 `let text = std::fs::read_to_string(path).map_err(|why| {`. There is no `metadata().is_file()` check and no `take()` bound.
  - ssm.rs:832-835 `let text = answer.text().await.map_err(...)?;`. Every other vendor read in this crate goes through `body_within`, `strict_discovery_body` or the chunked `MAX_DOCUMENT_BYTES` loop. Contrast config.rs:1021-1031, `read_bounded`, whose doc says: "`/dev/zero` grew the string until the allocator gave up ... a FIFO with no writer blocks in `open` forever, and a hang is the one failure nothing reports."
- Why it is wrong: `~/.aws/credentials` sits beside `~/.brutex/credentials.toml` and is read on the same start-up path (`AwsIdentity::discover`). If it is a FIFO, the credential read hangs with no event. If it is a symlink to a device, the process OOMs. Neither bound exists here, and the crate's own law 5 ("every O(1) claim dies at the first unbounded input") is applied to one credential file and not the other. The SSM answer is also read whole, with only the client's 10 s timeout bounding it.
- Repro: `mkfifo $HOME/.aws/credentials` with no AWS_* env vars set, then trigger any broker pull. `AwsIdentity::discover()` blocks in `read_to_string` forever, and nothing is logged. Contrast: `mkfifo ~/.brutex/credentials.toml` is refused as `NotARegularFile`.
- Suggested fix: Reuse `config::read_bounded` (or the same metadata-then-`take` shape) for the AWS credentials file. Read the SSM body through `body_within`/`strict_discovery_body` with a small cap (a few KiB).

## Checked and clean (beyond hunt-pull.md's credential row)

- No vendor-token minting anywhere. `totp.rs` computes codes only, has no production caller, and is guarded by `no_path_outside_this_module_computes_a_code`.
- The vendor credential value comes only from Parameter Store (`ssm::get_parameter`). The AWS identity comes from env or `~/.aws/credentials`, and that is the AWS key, not the broker credential, as stated in `AwsIdentity` docs. The token is never logged: only `value_len`/`byte_len` and a fingerprint are recorded. `CredentialPrint` has no Display and no byte accessor. Redacting Debug on `Credential`, `HttpSource`, `AwsIdentity`, `Secret`, `CredentialPath`, `VendorPaths` and `CredentialConfig`.
- SSM refusal bodies are never quoted, only allowlisted fault tokens. `value_of` errors carry serde's position text, not content. Redirects are disabled on both the SSM and broker clients.
- The config parser has no default. It refuses unknown or duplicate keys and tables, wrong regions, and segments that are empty, too long, `.`/`..`, contain `/` or other bytes outside `[a-z0-9_-]`, or look like a secret. Line and file bounds are taken at the read, and non-regular files are refused. Nothing from the file is echoed back.
- TLS: reqwest `rustls-tls-webpki-roots-no-provider` with the graviola provider, and no `danger_accept_invalid_certs` anywhere. All shipped base URLs are https.
- Path traversal: `path_safe` covers bar URL path segments. The capture filenames come from a feed enum, the pid and a timestamp. The cash-session cache names come from the day. No pull-side file name is derived from vendor text.
- Price text to paisa (`Paisa::from_rupee_text_half_up`) is length-capped, uses checked arithmetic and refuses non-decimal text. Body reads are capped on the bars, discovery, masters, resolve and cash-cache paths, and gzip expansion is capped (`MAX_EXPANDED`).

---
## Source: 20-store-cli-input.md

# P1-20 untrusted bytes and arguments (store, lake, cli)

### P1-20-01 low `ledger-all` creates its 24-directory output tree before it validates MONTH, range order or the data, so a bad argument leaves directories behind
- Where: crates/cli/src/lib.rs:1890-1919 (ledger_all_arm), crates/cli/src/ledger_all.rs:870-877 (run_chain), crates/cli/src/ledger_all.rs:173-187 (LedgerTree::create)
- Evidence:
  - The arm parses MONTH as a bare `u8` and has no range check. Its "MONTH must be 1..=12" refusal only fires when the text does not parse:
    ```rust
    from.1.parse::<u8>(), ... to.1.parse::<u8>(),
    ...
    (Ok(fy), Ok(fm), Ok(ty), Ok(tm), Ok(support_ppm), Ok(max_points)) if max_points > 0 => {
    ...
    (_, Err(_), _, _, _, _) | (_, _, _, Err(_), _, _) => refuse(out, "MONTH must be 1..=12"),
    ```
    `pool_arm` (lib.rs:1616-1620) does run `.filter(|m| (1..=12).contains(m))` and refuses a backwards range at the arm. `ledger-all` does neither.
  - `run_chain` creates the tree before any month is looked at:
    ```rust
    let source_root = crate::store_root()...?;
    let tree = LedgerTree::create(request.root)?;
    let sweepers = build_sweepers(&source_root, vendor, request, LEDGER_ALL_VERB)?;
    ```
    `LedgerTree::create` calls `std::fs::create_dir_all` for `authority/<rung>`, `execution/<rung>` and `selection/<rung>`, for all eight rungs. Months are first checked inside `build_sweepers` → `stored::load_span` → `stored::months_between` (stored.rs:2548-2562), which says "FROM month 13 is not a month ... Nothing was read." / "the range runs backwards".
  - `ledger_v6` runs these steps in the safe order: `size_sweeper` comes before `RungRoots::create` (ledger_v6.rs:309-318). The test `missing_policy_refuses_before_legacy_ledger_market_sizing_or_output_creation` (ledger_all.rs:1528) shows the intent: a refused run must not create the tree. Only the policy refusal is covered by that test.
- Why it is wrong: A bad MONTH, a backwards range, a span over the limit, or a missing NIFTY month is refused only after 24 directories are made under the operator's ROOT. The refusal text still says "Nothing was read", but files were written. The arm's own "MONTH must be 1..=12" message suggests the arm checks this, and it does not.
- Repro: Configure a complete research policy so that `admission_policy` passes. Then run `cli ledger-all dhan 2026 13 2026 12 200000 50 /tmp/lt`. The output refuses with "FROM month 13 is not a month", yet `/tmp/lt/authority/1min` ... `/tmp/lt/selection/60min` all exist. `cli ledger-all dhan 2026 6 2026 1 ...` (backwards range) gives the same result. Test sketch: copy the missing-policy test, set a valid policy and `from: (2024, 13)`, and assert `!root.exists()`.
- Suggested fix: Validate months in `ledger_all_arm` the way `pool_arm` does, with range and order checks. In `run_chain`, call `build_sweepers` before `LedgerTree::create`, which is the order ledger_v6 already uses.

## Checked and not filed (already hardened, or already recorded)
- store `read_header` / `Header::decode_parts`: checks magic, version, stride, CRC before use, reserved == 0, unknown flags. `validated` checks the counter against the file (`claimed > n_valid` → CounterExceedsFile) and the symbol and timeframe. The region allocation is clamped to `REGION_LEN`. `read_row` uses a stack buffer and a checked `offset_of`.
- store `open_read` uses O_NONBLOCK and fstat, and refuses anything that is not a regular file. The sidecar is never created on read or audit doors. checksum_audit uses O_NOFOLLOW and checks generation and nlink.
- store `catalog::walk` and `parse_month`: does not follow symlinks, rejects non-UTF-8 names, checks depth, accepts digits only.
- store `repair` RevisionReader/publish: receipt length is exact, header is re-encoded and compared, MAX_ROWS is enforced.
- lake reader: row count capped at file length, `uncompressed_page_size` capped, LongColumnChunk / ShortColumnChunk / UnreadChunkBytes checks. The unbounded `fs::read` in `LakeFile::open` is already recorded (o1store2-4), and lake has no workspace consumer.
- cli: `run_durable_os` rejects non-UTF-8 arguments. research-policy and catalog reads are byte-bounded with a generation check. checksum receipts canonicalise the root and refuse symlinks and nlink != 1. Paths are built only from validated `Symbol`s, hex identities or fixed names. Frontier block width comes from the pass over the file. The `boolean-qualified-search-stored` parse checks months, order and canonical positive integers. pool, range and stored arms rely on `stored::months_between` for month validation before any read.

---
## Remaining plan (paused 13:14 UTC on the user's throttle request)

- Pass 1: 20/20 agents done, 82 findings (22 medium, 60 low, 0 high).
- Pass 2 (queued, resume after 18:00 UTC at 3 agents at a time): re-audit the newest final/all-fixes-zero head with these angles in order:
  1. Verify the pass-1 fixes landed and the fixed tests now fail on the regression they name.
  2. Areas pass 1 did not cover: web/src logic tests (web/tests), docs 11-35, crates/costs and greeks numeric correctness, api routes for launches/POST state changes.
  3. Second look at CI gate allowlists and the ci-ok aggregator after fixes change ci.yml.
- Stop when a pass finds nothing new.

---
## Source: 12-tests-api.md (late arrival)

# P1-12: crates/api tests that cannot fail or do not test what they claim

Static audit of commit 331b05c (`/home/claude/brutex-audit`). Nothing was built or run. I checked each finding for duplicates against `docs/11-findings.md` and `/mnt/project-files/audit-20261003-workspace/*.md` (testgaps.md, hunt-api.md and the rest). None of the findings below is recorded there.

### P1-12-01 medium A source-shape assertion matches its own needle and passes against code that no longer has the shape
- Where: crates/api/src/server.rs:24918-24922 (test `the_stock_strike_width_is_reachable_and_narrower_than_the_index_one`)
- Evidence:
  ```rust
  let src = include_str!("server.rs");
  assert!(
      src.contains("let word = if is_index {"),
      "the strike width follows the underlying's own type"
  );
  ```
  `grep -n "let word = if is_index {" crates/api/src/server.rs` finds only line 24920, the assertion itself. Production now reads `let word = instrument_word(&rolling, is_index);` (server.rs:14460), and the selection lives in `const fn instrument_word` (server.rs:9854-9860).
  A few lines further down (24923-24927), the same test explains the self-match hazard for its *negative* needle and splits that needle with `concat!`. The positive needle was left whole.
- Why it is wrong: `include_str!("server.rs")` includes this test's own string literal, so the positive `contains` can never fail. The code was refactored to `instrument_word` and the test stayed green. The half the doc comment calls essential ("That the SELECTION reads the resolved type is what makes the narrow list reachable at all") is unproven. No other test calls `instrument_word`.
- Repro: change server.rs:14460 to `let word = instrument_word(&rolling, true);`, or make `instrument_word` always return `rolling.index_word`. The test still passes, and the stock width is unreachable again (the original defect it describes).
- Suggested fix: split the needle with `concat!` and point it at the current call site, `instrument_word(&rolling, is_index)`. Better: unit-test `instrument_word(&rolling, false) == rolling.stock_word` directly.

### P1-12-02 medium "A refused bind is logged" is proven only by an exit code, so the log event it names can be deleted with the test still green
- Where: crates/api/src/server.rs:20665-20687 (`a_refused_bind_is_logged_and_not_only_printed`) and server.rs:24945-24956 (`run_refuses_an_address_it_cannot_bind_and_says_which`). The code under test is the bind-error arm at server.rs:18558-18574.
- Evidence:
  ```rust
  assert_eq!(
      run_in(&agreeing("bindrefused"), &argv(&["serve", &taken.to_string()]), fired()).await,
      FAILED,
      "a port already held is a refused bind, and the exit code says so"
  );
  drop(squatter);
  ```
  That is the whole assertion surface of the "is logged" test. There is no `crate::emitted::mark()`/`landed(.., "api.server", "cannot bind the listening address")` and no check of the `addr`/`why` fields. The second test likewise asserts only `== FAILED`, and nothing checks that it "says which" address.
  `docs/05-decisions.md:10675` and `crates/api/src/emitted.rs:1538-1541` both state that this event "is now driven by `a_refused_bind_is_logged_and_not_only_printed`".
- Why it is wrong: both test names claim an observable (the event is logged, the address is named) that neither test inspects. `FAILED` is also returned by several earlier arms of `run_from`/`run_in_over` (an underivable masters dir or store root, `sole_server`). So the exit code does not even prove the bind arm was the one reached. "Driven" here means covered, not asserted.
- Repro: delete the `telemetry::emit(...)` block at server.rs:18558-18569, or change its message or drop its `addr` field. Both tests still pass.
- Suggested fix: in `a_refused_bind_is_logged_and_not_only_printed`, take `crate::emitted::mark()` before the call. Then assert that one `api.server` / `cannot bind the listening address` record at `Error` landed, with `addr` equal to the held address and a non-empty `why`.

### P1-12-03 medium Two child-process tests accept a child that ran no test, which is the exact shape `crate::isolated` documents as forbidden
- Where: crates/api/src/strict_sweep_tests.rs:506-519 (`strict_invalid_server_environment_refuses_before_configuration_slot_or_start`) and crates/api/src/indexstoplaunch_tests.rs:509-571 (`configured_launch_admits_only_exact_requests_and_records_missing_source_refusal`)
- Evidence:
  ```rust
  // strict_sweep_tests.rs
  let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
      .arg("--exact")
      .arg("sweeprun::strict_tests::strict_invalid_server_environment_refuses_before_configuration_slot_or_start")
      ...
      .status()
      .expect("isolated environment test child");
  assert!(status.success());
  return;
  ```
  ```rust
  // indexstoplaunch_tests.rs
  const CHILD_TEST: &str = "indexstoplaunch::tests::configured_launch_admits_only_exact_requests_and_records_missing_source_refusal";
  ...
  let output = std::fs::read_to_string(log).unwrap();
  assert!(status.success(), "{output}");
  Ok(())
  ```
  Compare crates/api/src/isolated.rs:15-20: "`--exact` with a name that matches nothing is not an error to the test harness: it prints `0 passed` and exits zero. A parent that only checked the exit status would then pass on a child that tested nothing ... So [`rerun`] requires the harness's own `1 passed` line". The sibling `booleanlaunch_tests.rs:471-478` at least requires a line the child printed. These two require nothing.
- Why it is wrong: the hard-coded child names are correct today. But renaming the test function, renaming `strict_tests`/`tests`, or moving the module makes the child match zero tests and exit 0, and the parent then passes without testing anything. The strict test is the cited proof in `docs/04-invariants.md:4488` ("invalid strict ... environment settings refuse before a slot or attempt").
- Repro: rename `mod strict_tests;` (sweeprun.rs:3366) to `mod strict_sweep_tests;`. The parent then runs under its new path, the child's `--exact sweeprun::strict_tests::...` runs `0 passed`, exits 0, and the test is green with no assertion executed. Do the same with `CHILD_TEST` in indexstoplaunch_tests.rs.
- Suggested fix: route both through `crate::isolated::rerun`, which requires `1 passed`. Or capture stdout and assert on `1 passed` plus a sentinel line the child prints after its last assertion.

### P1-12-04 low The print-macro ban for server.rs stops at the first `mod tests {` and never scans about 2,500 lines of production code after it
- Where: crates/api/src/serve_edge_tests.rs:248-271 (`production_code_in_server_rs_prints_through_the_panic_free_writers`)
- Evidence:
  ```rust
  let source = include_str!("server.rs");
  let production = source
      .split_once("\nmod tests {")
      .map_or(source, |(before, _)| before);
  ```
  In server.rs, `mod tests {` opens at line 18590 and closes at 31114. Production items follow it: `universe_token_of`, `universes_json`, `universe_resolve`, `indexmap_json`, `indexmap_reading`, `vocab_json`, `calendar_json`, `calendar_json_reading` and more, out to line 33690. They are interleaved with later `#[cfg(test)]` modules.
- Why it is wrong: the doc comment claims "no print macro that can panic is left in this file's production code", but the scan covers only the part of the file before line 18590. CI gate 23's per-file print count partly compensates, because a *new* print changes the count. A print moved out of test code into one of those handlers keeps the count and passes both checks.
- Repro: add `println!("x");` inside `calendar_json` (after line 31114) and remove one test-only `println!` from server.rs. This test and gate 23's exact count both stay green.
- Suggested fix: compute production as the file minus every `#[cfg(test)]` item (the shared `emitted` helpers already handle multiple test regions). At minimum, assert that the scanned prefix contains a late production item such as `async fn calendar_json(`, as `sweeprun.rs:4044-4047` does for `top_json`.

### P1-12-05 low The over-the-wire traversal test's "nothing leaked" assertion cannot fail, because the fixture holds no file at the traversal target
- Where: crates/api/src/server.rs:22225-22228 (in `the_server_answers_every_route_and_then_shuts_down_gracefully`). The fixture is `front()` at server.rs:21955-21976.
- Evidence:
  ```rust
  // 8. AND A TRAVERSAL IS REFUSED OVER THE WIRE, not only in a unit test.
  let escape = get(addr, "/%2e%2e/Cargo.toml").await;
  assert!(escape.contains("400"), "{escape}");
  assert!(!escape.contains("[package]"), "nothing leaked: {escape}");
  ```
  `front()` writes only `build/index.html`, `build/_app/app.js`, `typeahead.js` and `build/store.json` under a fresh `scratch::path("front-static")`. No `Cargo.toml` exists anywhere under that directory or in its temp-dir parent.
- Why it is wrong: even if the traversal were served, there is no `Cargo.toml` to serve, so `!contains("[package]")` always holds. The only discriminating check left is a bare substring `"400"` over the whole response (headers included) instead of the status line. Contrast the decoy pattern step 1 of the same test uses (`build/store.json` = `"DECOY"`), which exists "precisely so this can fail".
- Repro: make the static handler resolve `..` segments without refusing. The response becomes a 404 or the SPA shell, and the "nothing leaked" assertion still passes. Only the weak `"400"` substring stands between the mutant and green.
- Suggested fix: write a sentinel file at the path the traversal would reach (for example `dir/Cargo.toml` holding `[package] LEAK`). Assert the status line is `400` and the body does not contain the sentinel.

## Checked and found sound (no finding)
- `crate::isolated::rerun` and `where_permission_binds` both require `1 passed`. Every `rerun`/`where_permission_binds` caller fails loudly on a name mismatch.
- `booleanlaunch_tests.rs` child: the parent `unwrap`s the child's `BRUTEX_BOOLEAN_LAUNCH_METADATA=` line, so a zero-test child fails.
- `#[should_panic]`: none in crates/api. `#[ignore]` (calendar_of.rs:1827, recovery.rs:2974) is already recorded as testgaps-7.
- A whole-crate scan for positive `contains`/`find`/`split_once` source needles over `include_str!` of the test's own file found exactly one self-only positive needle (P1-12-01). No positive source needle matches only in test code. No `split_once`/`find` anchor resolves first to a comment or test region.
- `booleanevidencejson_tests::on_every_page`/`code_only` and its controls; `census_request_tests` logging and caching tests; `bars_window_route_tests`; `verification_route_tests`; `sweeprun_admission_tests`; `tests/binary.rs`; the cost_limits source pins; `if let Some(..) { assert }` blocks (each is guarded by a preceding count or equality assertion).
- `let _ =` sites in tests are cleanup, a deliberate no-panic call (`rupees(i64::MAX)`), or `emit_for_run` results whose effect a later assertion observes.

---
# Pass 2 (final/all-fixes-zero @ 5140aca)

## Source: p2/01-verify.md (pass-1 fix verification + diff review)

# P2-01 — verification of pass-1 tests/docs/security findings (331b05c -> 5140aca)

Static reading only, at /home/claude/brutex-audit (final/all-fixes-zero @ 5140aca). The fix commits are d773ab9, 3f22aed, bfaf961 and 5140aca (D-1760..D-1765). Files outside the diff (`git diff --stat 331b05c HEAD`, 47 files) were checked as unchanged. Where a finding's code sits in a changed file (server.rs, logs.rs, bars.rs, docs/04), I grepped the cited code at HEAD and confirmed it is still there.

Totals: 25 FIXED, 57 NOT FIXED, 0 FIX WRONG OR INCOMPLETE. Four new low-severity defects are listed below the table.

## Verdicts

| ID | Verdict | Note |
|---|---|---|
| P1-01-01 | FIXED | `WindowAsk::parse` refuses `limit` outside `1..=MAX_WINDOW_LIMIT`, `dir` outside ``/asc/desc and `extremes` outside ``/0/1/false/true (server.rs ~3339-3384). Web callers send desc/asc, 1, 512 or 1000, and `extremes=0`, so none of them is broken. |
| P1-01-02 | FIXED | `asked()` collects `ignored` and emits one `api.logs filter ignored` Warn. The JSON echoes level/target/run/run_key/ignored. `run` must be canonical. The HTML `/logs` page has only the Warn and does not render `ignored` (the pass-1 fix allowed this). |
| P1-01-03 | FIXED | `limit_asked` emits an `api.backtest limit ignored` Warn for a present, unparseable `limit`. emitted.rs goes 61->63 and both new rows are driven. |
| P1-01-04 | FIXED | `Selector::parse` walks the keys (identity/page/limit only; no repeats, no empty values). `integer_param` is canonical. The web sends only `identity`, `page` and `limit`. |
| P1-02-01 | FIXED | `envelope_disagreement` in `legs_from` checks the payload's vendor (`parse_feed`) and granularity or series against the envelope. Web legs (`wireBodyFor` sets vendor and granularity=dir; fno sets series fut/opt with dir futures/options) agree. The recovery path also goes through `legs_from` and stays consistent. |
| P1-02-02 | FIXED | Same fix as P1-01-01. Test `an_unknown_direction_or_extremes_flag_is_refused_not_defaulted` covers dir, extremes and limit. |
| P1-02-03 | FIXED | `bars_feed_word`/`bars_vendor` read `feed=` or `vendor=` and refuse a disagreement with 400. See P2-01-02 for a doc-comment defect the edit introduced. |
| P1-02-04 | FIXED | Feed is required and parsed by `ingest::parse_vendor`. A refusal answers 400; a missing client or clock answers 503. The mapping page sends `feed=${F.active}` and returns early when no feed is set. |
| P1-02-05 | FIXED | `store_filter` parses `timeframe` from `Timeframe::KNOWN`, and `store_filter_bar` renders the radio pills. An unknown value is shown as "All" (stated in the code). |
| P1-02-06 | FIXED | topjson `parse` canonicalises through `ingest::parse_feed(..).wire()` and refuses an unknown feed. |
| P1-03-1 | NOT FIXED | `poll_write` still calls `rearm` on any write (server.rs:17148, 17161). |
| P1-03-2 | NOT FIXED | `authority.port_u16() != Some(local_addr.port())` is unchanged (server.rs:16799). |
| P1-04-01 | NOT FIXED | `bars_window_json` still calls `bars::window` inline, with no `run_store_read`/`spawn_blocking`. |
| P1-04-02 | NOT FIXED | logs.rs has no `run_store_read`/`spawn_blocking`. |
| P1-04-03 | NOT FIXED | `universe_resolve` gained status codes but has no mutex or try_lock guard. |
| P1-04-04 | NOT FIXED | recovery.rs is not in the diff. |
| P1-06-01 | NOT FIXED | `grep journal_error web/src` finds nothing. |
| P1-06-02 | NOT FIXED | web live-progress and logs `PAGE_LIMIT` are unchanged. |
| P1-06-03 | NOT FIXED | audit page and audit_json paging are unchanged. |
| P1-07-01 | NOT FIXED | .github/ is not in the diff. |
| P1-07-02 | NOT FIXED | .github/ is not in the diff. |
| P1-07-03 | NOT FIXED | .github/ is not in the diff. |
| P1-07-04 | NOT FIXED | .github/ is not in the diff. |
| P1-08-01 | NOT FIXED | ci.yml is unchanged. |
| P1-08-02 | NOT FIXED | ci.yml is unchanged. |
| P1-08-03 | NOT FIXED | ci.yml is unchanged. |
| P1-08-04 | NOT FIXED | ci.yml is unchanged. |
| P1-08-05 | NOT FIXED | ci.yml is unchanged. |
| P1-09-01 | NOT FIXED | auto-merge.yml is unchanged. |
| P1-10-01 | NOT FIXED | cli/tests/binary.rs and cli/src/lib.rs are unchanged. |
| P1-10-02 | NOT FIXED | audited_range_tests and checksum_receipts_tests are unchanged. |
| P1-10-03 | NOT FIXED | equity_statement_tests is unchanged. |
| P1-10-04 | NOT FIXED | candidate_universe.rs is unchanged. |
| P1-11-01 | NOT FIXED | cli/src/lib.rs is unchanged. |
| P1-11-02 | NOT FIXED | readonly_file, research_policy and results_report_tests are unchanged. |
| P1-11-03 | NOT FIXED | index_stop_source_context_tests is unchanged. |
| P1-13-01 | NOT FIXED | indicators is unchanged. |
| P1-13-02 | NOT FIXED | runner/src/lib.rs is unchanged. |
| P1-13-03 | NOT FIXED | engine/src/column.rs is unchanged. |
| P1-14-01 | NOT FIXED | pull/src/ssm.rs is unchanged. |
| P1-14-02 | NOT FIXED | core/tests/graph.rs is unchanged. |
| P1-14-03 | NOT FIXED | core/tests/graph.rs is unchanged. |
| P1-14-04 | NOT FIXED | core/tests/findings.rs is unchanged. |
| P1-14-05 | NOT FIXED | pull/tests/unit.rs changed only in the manifest geometry test, and pull/src/secret.rs is unchanged. |
| P1-14-06 | NOT FIXED | costs/tests is unchanged. |
| P1-14-07 | NOT FIXED | lake/tests is unchanged. |
| P1-15-01 | FIXED | CLAUDE.md §3 rule 7 now states Absent for indices and Present for eligible equities (D-0507). |
| P1-15-02 | FIXED | §10 in CLAUDE.md and AGENTS.md: the fourteen documents carry authority, 12- to 35- and research-policy/ carry none, and 22- is doubled. The count of 25 matches `ls docs`. |
| P1-15-03 | FIXED | Both files now say clause A pins vocab "to no dependency at all". |
| P1-15-04 | FIXED | k=1 now calls `primitives::append(&mut first, ..)` (engine lib.rs:1555), and rule 4 at HEAD says "at every k". The new test pins this. |
| P1-15-05 | FIXED | AGENTS.md now quotes "a CLAUDE.md edit". |
| P1-15-06 | FIXED | README language, vocab (370 = `TABLE: [BitDef; 370]`), sweep surface, look-ahead and layout are corrected. |
| P1-15-07 | FIXED | A resolved banner was added. `verify_block_of` exists (store/src/file.rs:2452). |
| P1-15-08 | FIXED | A done banner was added. STRIDE 261 and STRIDE_V2 213 match cli/src/results.rs:125,166. |
| P1-16-01 | FIXED | §11 covers version 3 and adds §11.5a. Offsets 80/104/105..124/124 match manifest.rs `C_CONTRACT=16` (+64), `OFF_CRC=60`. Minor leftover: the §11.3 heading still reads "64 bytes, both versions". |
| P1-16-02 | FIXED | §13 is rewritten for version 7 / 280 bytes, and a new doc-binding test reads the constants. See P2-01-01 for a defect in how that test was inserted. |
| P1-16-03 | FIXED | Docs give 616/648. The in-place growth is recorded as a breach. Both readers refuse a 640-stride file by name (`retired_parameter_stride`). |
| P1-16-04 | NOT FIXED | D-1763 explicitly queues this ("Not done here"). |
| P1-16-05 | FIXED | docs/01 now says one `pread` plus a block CRC, with no mapping (`store` has `#![forbid(unsafe_code)]`). |
| P1-16-06 | FIXED | The concurrency table now names the checksummed header-slot pwrite. |
| P1-16-07 | FIXED | The WASM claim is replaced. |
| P1-16-08 | FIXED | `store::catalog::walk` is named as the exception. |
| P1-17-01 | NOT FIXED | docs/04 only gained appended ZR rows; the cited row is unchanged. |
| P1-17-02 | NOT FIXED | The APIC-07 row is unchanged. |
| P1-17-03 | NOT FIXED | The C-V53-02 row is unchanged. |
| P1-17-04 | NOT FIXED | The FX3-01 row is unchanged. |
| P1-18-01 | NOT FIXED | docs/09-verify.md is unchanged. |
| P1-18-02 | NOT FIXED | docs/10-shared-core.md is unchanged. |
| P1-18-03 | NOT FIXED | docs/10-shared-core.md is unchanged. |
| P1-18-04 | NOT FIXED | docs/07-plan.md is unchanged. |
| P1-18-05 | NOT FIXED | docs/07-plan.md is unchanged. |
| P1-18-06 | NOT FIXED | docs/07-plan.md is unchanged. |
| P1-18-07 | NOT FIXED | docs/07-o1-architecture.md is unchanged. |
| P1-19-01 | NOT FIXED | pull http.rs and ssm.rs are unchanged. |
| P1-19-02 | NOT FIXED | pull chain.rs and fno.rs are unchanged. |
| P1-19-03 | NOT FIXED | pull ssm.rs is unchanged. |
| P1-20-01 | NOT FIXED | cli ledger_all.rs and lib.rs are unchanged. |
| P1-12-01 | NOT FIXED | The server.rs test region was not touched. |
| P1-12-02 | NOT FIXED | The server.rs test region was not touched. |
| P1-12-03 | NOT FIXED | strict_sweep_tests and indexstoplaunch_tests are unchanged. |
| P1-12-04 | NOT FIXED | serve_edge_tests is unchanged. |
| P1-12-05 | NOT FIXED | The server.rs test region was not touched. |

## Front-end compatibility checks for the new refusals (no defect)

- `/bars/window.json`: terminal.svelte.js sends `dir=desc|asc`, `limit=1|512`. database-pages.js sends `dir`, a bounded `limit` and `extremes=0`. backtest/+page.svelte sends `limit=1000|1`. All are accepted.
- `/pull/run`: ingest/+page.svelte `wireBodyFor` sets `vendor` and `granularity=dir`. `fnoBodies` sets `vendor`, `series=fut|opt` and `dir=futures|options`. All agree with `envelope_disagreement`.
- `/universe/resolve`: mapping/+page.svelte always sends `feed=`.
- `/frontier.json` and `/trades.json`: the web sends only `identity`, `page` and `limit`.
- `/logs.json`: live-progress.ts does not check the key set, so the added fields break nothing. The `run` it sends is canonical.
- `/engine/top.json` and `/store`: changed only to accept more or to show more.

## New defects introduced by the fixes

### P2-01-01 low The new frontier doc-binding test was inserted inside another test's doc comment
- Where: crates/cli/src/frontier.rs:2007-2048
- Evidence: the doc block that opens `/// The stride is what the writer writes, not what a comment claims.` ... `..Default::default()` had been supplying a zero for it.` now runs straight into `/// `docs/02-store-format.md` §13 states the version, the stride and the seal offset ...` followed by `#[test] fn the_store_format_doc_states_the_frontier_this_build_writes()`. Then `#[test] fn the_stride_is_exactly_what_the_writer_writes() {` follows with no doc comment.
- Why it is wrong: the stride test lost its own explanation (versions 2 and 3, the 144->192->208 history). That history is now attached to an unrelated doc-binding test. A reader of either test is told the wrong thing about what it guards.
- Repro: `sed -n 2007,2048p crates/cli/src/frontier.rs`.
- Suggested fix: move the new test and its three-line doc above the `/// The stride is what the writer writes` block, so that block attaches again to `the_stride_is_exactly_what_the_writer_writes`.

### P2-01-02 low `bars_feed_word` was inserted between `locate_series`'s doc comment and `locate_series`, so the D-0695 doc now documents the wrong function
- Where: crates/api/src/server.rs ~15497-15557
- Evidence: the doc paragraph ending `/// Only that census can say where that feed filed the name, so when it cannot / be read and the caller did not give the pair, the route refuses and names / it, as `/calendar.json` does.` runs straight into `/// The feed a `/bars` request names, under either spelling.` and `fn bars_feed_word(query: &str) -> Result<String, ()>`. `fn locate_series(` at 15557 now has no doc comment.
- Why it is wrong: rustdoc and every reader now attribute `locate_series`'s census-lookup contract (including the D-0695 "asked feed's census is refused, not stepped over" rule) to a two-line query-string helper. `locate_series` loses the only statement of that rule.
- Repro: `sed -n 15490,15560p crates/api/src/server.rs`.
- Suggested fix: move `bars_feed_word` and `bars_vendor`, with their docs, above the `locate_series` doc block.

### P2-01-03 low D-1762 names the wrong route for the audit-ID change, and two behaviour changes have no invariant row
- Where: docs/05-decisions.md:54681 (D-1762); crates/api/src/operation_audit.rs:244-251; crates/api/src/server.rs:16533-16534; docs/04-invariants.md ZR-01..ZR-25
- Evidence: D-1762 says "- `/audit.json` answered `invocation=` or `before=` at or below `journal::ID_BASE` with a 503. It now answers 400". The code changed is `operation_audit::parse`, which is served at `.route("/backtest/audit.json", axum::routing::get(crate::operation_audit::audit_json))`. `/audit.json` is `audit_json::audit_json` (the pull journal: `feed`/`page`) and takes no `invocation` or `before`. The ZR rows cite no test for this change (`queries_reject_aliases_duplicates_overflow_and_mixed_exact_pages` appears 0 times in docs/04). D-1761 also changes `master::Columns::widest` (CE-1) and names `a_row_cut_just_before_a_last_vendor_id_column_names_the_shortfall`, but docs/04 has no row for it (0 matches). ZR-03 covers only `basis_points`.
- Why it is wrong: CLAUDE.md §9 requires every new invariant in docs/04 beside its test. The ledger entry also sends a reader to the wrong route, one with a different parameter set. Its own "Evidence: the tests named in ZR-04 to ZR-13" does not cover the audit-ID item.
- Repro: `grep -n '"/backtest/audit.json"' crates/api/src/server.rs`; `grep -c 'a_row_cut_just_before\|queries_reject_aliases' docs/04-invariants.md` gives 0.
- Suggested fix: append a D-entry correcting the route name to `/backtest/audit.json`. Add ZR rows for the operation-audit ID-namespace 400 and for the CE-1 vendor-id width rule, citing the two tests.

### P2-01-04 low `note_unreadable_records` is documented as "called from exactly two places — `page` and `window`", but there are three call sites; the shape test pins four occurrences and says three
- Where: crates/api/src/bars.rs:374-387 (doc); docs/06-limits.md:3253-3259; bars.rs `read_in_time` (~995); bars.rs test `unreadable_records_are_reported_once_per_request_not_once_per_file`
- Evidence: doc: "this function is called from exactly two places — [`page`] and [`window`]". The call sites at HEAD are `page`, the seek branch of `window` (`if let Some(file) = first_faulted { note_unreadable_records(file, &record_faults, bars.len(), offset); }`) and `read_in_time` (`note_unreadable_records(file, &record_faults, all.len(), 0);`). The test says "three call sites" and `assert_eq!(code.matches("note_unreadable_records(").count(), 4);`, while it also asserts the 06-limits sentence "from exactly two places — `page` and `window`".
- Why it is wrong: the structural-bound argument names the exact call sites as its proof, and the test that checks the prose binds a sentence the same test's own count contradicts. The one-event-per-request property still holds, because `read_in_time` is called only from `window` and outside its loop. The stated proof does not match the call graph it cites. The commit message for 5140aca acknowledges the read moved into `read_in_time`.
- Repro: `grep -n "note_unreadable_records(" crates/api/src/bars.rs`.
- Suggested fix: say "three call sites — `page`, `window`'s seek branch and `read_in_time` (called only by `window`) — each once per request", and update the 06-limits sentence and the test's needle together.

## Checked in the diff and holding (no new defect)
- `basis_points` uses `checked_sub` (CE-2). The test covers `i64::MIN` deltas.
- `bars_by_symbol` is keyed by (exchange, segment, symbol) over 1-minute spot only. The test covers an option, the daily rung and BSE.
- autopilot `store_refused` is reset in `clear_month` and swapped in `park`/rung swap. The tests cover transport->store and store->transport->store.
- Window lookback across months: `earlier_in_time` is O(months). `read_in_time` folds in time order for both directions. Null OI ranks last in both directions.
- The three-piece date pad refuses non-digit and over-wide pieces.
- The receipt-last scratch rewrite (D-1760) is guarded by the owner lock and uses unlink rather than follow. The VIX publish shortcut is taken only on a whole receipt.
- 640-stride retirement is checked in both `check_record_file` copies. The test confirms the file is left byte-identical.

## Source: p2/02-docs-web-tests.md

# P2-02: docs 12-35 vs code, and web/tests + web/ci vacuity (checkout final/all-fixes-zero @ 5140aca)

### P2-02-01 medium The dev-proxy drift test cannot see helpers that take an injected `request`, and five live JSON routes are missing from `vite.config.js` ROUTES
- Where: web/tests/proxy.test.js:54-90 (scanner), web/vite.config.js:28-254 (ROUTES), web/src/lib/expression-search.js:11-17, web/src/lib/candidate-trades.js:49-58, web/src/lib/index-stop-vix.js:61-63, web/src/lib/index-stop-source.js:50-53, web/src/lib/index-stop-ranking.js:41-44
- Evidence:
  - The scanner only matches `fetch`, `ask`, and names brought in with `import { ask as X }`:
    `const callees = new Set(['fetch', 'ask']);`
    `for (const a of text.matchAll(/import\s*\{[^}]*\bask\s+as\s+([A-Za-z_$][\w$]*)/g)) {`
  - The lib readers take the fetcher as a parameter named `request`, not as an import alias:
    `export async function fetchExpressionSearch(selection, request) {` ... `await request('/expression-search.json?' + query);`
    `export async function fetchIndexStopVix(selection,request=ask){` ... `await request('/index-stop-vix.json?'+query);`
    (the same holds for `/candidate-trades.json`, `/index-stop-candles.json` and `/index-stop-ranking.json`)
  - None of the five is in ROUTES, and no entry is a prefix of any of them. `/index-stop.json` is not a prefix of `/index-stop-vix.json`. The proxy is built from that list: `proxy: Object.fromEntries(ROUTES.map((route) => [route, API]))` (vite.config.js:280).
  - The Rust side serves them. For example, crates/api/src/operation_audit.rs:68 lists `"/candidate-trades.json"` and :78 lists `"/expression-search.json"`.
- Why it is wrong: The test's own header explains why the gate exists. A route missing from ROUTES gets the dev server's HTML fallback with status 200, so `.json()` throws and the operator sees a parse error. The test is named "every route the pages fetch is proxied". It passes while five routes are unproxied, because every call made through a `request` parameter is invisible to it. I re-ran the test's own `fetched()` logic: it returns 27 paths, and all five above come back `false`. Other routes reached the same way (for example `/boolean-oos.json`, `/index-stop.json` and `/feeds.json` through `request(`) are covered only because someone listed them by hand.
- Repro: Run `npm run dev` and open the candidate-trades, expression-search or index-stop VIX/candles/ranking views. Each request goes to Vite and gets index.html. Statically: copy `fetched()` from proxy.test.js into a script and check `out.has('/expression-search.json')`. It is false, and the suite stays green.
- Suggested fix: Add the five routes to ROUTES. Widen the scanner to every `/...` string literal that ends in `.json`, or at least to calls of any identifier whose first argument is an absolute path. The gate should not depend on the callee name.

### P2-02-02 low The front end's two O(1) timing tests cannot fail for the regressions their names describe
- Where: web/tests/prefix.test.js:110-128 and :130-152; web/src/lib/prefix.js (`probe`)
- Evidence:
  - Test 1, "a keystroke costs the same at 800 instruments and at 80,000", times `probe(ix, rows, 'ABCD')` over `universe(n)`. Each symbol there is `A[i % 26] + A[(i >> 5) % 26] + A[(i >> 10) % 26] + A[(i >> 15) % 26] + String(i)`. The prefix `ABCD` needs `(i>>10)%26 === 2` and `(i>>15)%26 === 3`. That never happens for i < 800, nor for i < 80,000 (where `i>>15` is at most 2). I ran `build(universe(n)).has('ABCD')` and got `false` for both sizes. So both timings measure a `Map.get` miss returning `[]`, never "a bucket by reference", which is what the assertion message claims.
  - Test 2, "the one place cost is NOT constant is named, and it is bounded by a bucket", compares two catalogues of the same size (40,000 each). It asserts only `fatCost > thinCost`. The thin catalogue (`AB${i%100}D${i}`) has no `ABCD` bucket at all, and no row in it starts with `ABCD1`.
- Why it is wrong: I wrote two mutants of `probe` and timed both with the tests' own `cost()` harness. Mutant A copies the bucket on every keystroke (`return [...(ix.get(q) ?? [])]`), which is O(bucket) and scales with the catalogue. Test 1 gave a ratio of 1.46, under the 3.0 ceiling, so it passes. Mutant B drops the index and does a full scan (`rows.filter(r => r.symbol.startsWith(q))`). Test 2 gave fat > thin as true (1.09 ms vs 0.71 ms), so it passes. Neither test can catch the regression it names.
- Repro: Use the two mutants above with the `cost()` and `universe()` code copied from prefix.test.js (my script is at scratchpad/p2/w02/pfx2.mjs).
- Suggested fix: Test 1 should probe a prefix whose bucket is non-empty and grows about 100x between the two universes (for example `'A'`), so a copy or scan would show up. Test 2 should compare a 100x larger catalogue against one with the same 4-prefix bucket size, as its comment describes, and assert a ratio below 3.0. "Fat > thin" is not that check.

### P2-02-03 low Docs 14, 15 and 31 cite verification through Markdown links into the gitignored `target/` directory, and those links are dead in the repository
- Where: docs/14-sweep-readiness-20260906.md (12 links, e.g. :56, :57, :69, :108, :123), docs/15-indicator-readiness.md (1 link), docs/31-backtest-integration-20260908.md (14 links). Docs 16, 18, 20-26 and 32 cite similar bare paths under `target/sweep-audit-*` and `/private/tmp/brutex-*`.
- Evidence: `[All 18 cases and exact result reuse](../target/sweep-audit-20260906/strict-range-matrix-174435aa/matrix-report.txt)` (doc 14:56); `[Eight strict integration tests](../target/sweep-audit-20260906/strict-v6-final-tests.log)` (doc 14:108). `.gitignore:2: /target`. In total, 27 relative links in docs 14, 15 and 31 point to files that do not exist in the tree.
- Why it is wrong: CLAUDE.md §3 rule 6 says "Never claim a measurement you did not take". These tables present links as the evidence for measured claims ("18 strict range cases plus 18 retries passed"). In this public repository every one of those links is a 404, and no reader can check the claim. Doc 18 does say "The local logs above are session artifacts, not permanent test authorities". Docs 14, 15 and 31 carry no such caveat, and they show the artifacts as clickable evidence.
- Repro: `for p in $(grep -oE '\]\(\.\./target/[^)]+' docs/14-*.md docs/15-*.md docs/31-*.md | sed 's/](//'); do [ -e "docs/$p" ] || echo dead $p; done` prints 27 dead links.
- Suggested fix: Replace the links with the named reproducible test or command. Alternatively, add doc 18's sentence about session artifacts to each of these docs and change the links to plain text, so they do not read as checkable evidence.

## Checked and found consistent (no finding)
- Script over docs 12-35: every backticked snake_case/SCREAMING identifier (169 candidates) was checked against all tokens in crates/, web/src, web/tests, web/ci and .github. The only misses were indicator test *file* names (all exist under crates/indicators/tests/) and `BRUTEX_ADMIT_MIN_WORST_REWARD_RISK_PPM`, which is generated by `format!("BRUTEX_ADMIT_{}", name.to_ascii_uppercase())` (crates/cli/src/ledger_all.rs:323).
- Every `crates/`, `web/`, `config/` and `.github/` path named in docs 12-35 exists. The exceptions are a `web.yml` that doc 34 itself describes as removed, and a temporary `.svelte-kit` build dir.
- Every `cli <subcommand>` string and `/route` named in those docs exists in crates/cli or crates/api.
- Doc 15's test counts match the `#[test]` counts per file: crossing 3, session 8, trend 7.
- Doc 26's VWAP bit table (20 positions: 52/53, 143-152, 190-197) matches vocab/table.rs, and its formula matches `pv.div_euclid(3*v)`.
- Doc 22-research-policy: 37 profile fields (config has 37 plus `policy_version`), and the 39-field `AdmissionFieldV1::ALL`.
- Doc 18: the API limits match detail.rs (MAX_QUERY_BYTES 512, MAX_PAGE_ROWS 256, MAX_SCAN_BYTES 64 MiB, MAX_RESPONSE_BYTES 8 MiB, MAX_CONCURRENT 4).
- Doc 32: 2^328 ≈ 5.5e98 and 1,566 × 2 × 8 = 25,056.
- Doc 33: the losing-day streak resets only on a winning day (cli/src/index_consistency.rs:1420-1435).
- web/tests: all 90 files were scanned for tests with zero or one assertion (about 60 were looked at by hand), `?? []`-then-loop vacuity, stubbed fetchers, and `rejects`/`throws` without matchers. I checked these by hand and found them sound: backtest-truth (its zero branches really contain `exact(0)`/`money(0)`), expression-search (the mutation baseline is accepted under the same args), store-census, request-gate, pooled, sweep, rows, live-progress, gap-verdict, index-stop-chart, masters-load and dates (it really varies TZ in child processes).
- web/ci/css-comments.mjs: it fails on empty input and on an unreadable `<style>` block. CI passes it the full `find web/src` list.

## Pass 2 summary and pass 3 plan
- Pass 2: 7 new findings (1 medium, 6 low). Pass-1 status at 5140aca: 25 fixed, 57 not yet fixed, 0 fixed wrongly.
- Pass 3 (2 agents, usage-capped): api POST launch/state-changing routes; CI gates re-check once ci.yml changes land; re-verify the 57 open pass-1 findings after the next fix batch.

---
# Pass 3 (final/all-fixes-zero @ 5140aca)

Overlap: P3-01-02 and P3-02-06 both cover sweep-all's ignored underlying/span (fix once).

## Source: p3/01-api-post.md

# P3-01 — crates/api state-changing routes (pass 3), checkout final/all-fixes-zero @ 5140aca

Scope: every POST route in `route_table` (server.rs:16225-16540): `/pull/spot`, `/pull/fno`, `/pull/run`, `/pull/run/stop`, `/pull/recovery`, `/universe/resolve`, `/autopilot/{pause,resume,control}`, `/ingest/queue`, `/backtest/run`, `/backtest/descend`, `/engine/command`, `/masters/refresh`. I also checked the audit middleware around them (`operation_audit::request_audited`) and the cross-origin layer.

Skipped as already recorded: first-match repeated body keys (hunt-api-5 / attacksweep-3), hand-pull disconnect (hunt-api-1), the Ctrl-C/sweep hang (hunt-api-2), `pull_run_stop` fsync under `site.run` (runs-3 / recovery-5), recovery 503-versus-409 under flock (recauto "checked"), the `Attempt::drop` fsync on a Tokio worker (log-2 / resources-3), universe/resolve status codes and guard (P1-02-04, P1-04-03), the `/pull/run` envelope check (P1-02-01, now fixed), and the masters restart prose (Z1-slice13-F1; but see P3-01-05).

### P3-01-01 medium `/pull/run` and `/pull/recovery` still have the 8 KiB form cap. The ingest page's Pull press sends each ticked member twice-percent-encoded inside every leg, so a modest selection gets a framework 413 (D-1499 fixed `/pull/spot` only)
- Where: crates/api/src/server.rs:16045, 16295-16299; crates/api/src/server.rs:45-59; crates/api/src/ingest.rs:86-96; web/src/routes/ingest/+page.svelte:2865-2867, 6688-6702, 6740-6746
- Evidence:
  - Router-wide cap: `.layer(axum::extract::DefaultBodyLimit::max(MAX_FORM_BYTES))` with `pub(crate) const MAX_FORM_BYTES: usize = 8 * 1024;`. The only routes given a larger cap are `/pull/spot` and `/ingest/queue`: "**Two routes read more than this, and they are named.** `/pull/spot` and `/ingest/queue` take repeated `member` fields, and 750 ticked instruments do not fit 8 KiB" (server.rs:53-58).
  - `/pull/run` gets no layer: `.route("/pull/run", axum::routing::post(pull_run))`. `/pull/recovery` gets none either.
  - The page builds one spot body per feed × rung, with `for (const m of ticked) p.append('member', m.symbol ?? m.key);`. It then posts all of them to `/pull/run` as ``leg=${encodeURIComponent([b.route ?? '/pull/spot', b.vendor ?? '', b.dir, b.label, encodeURIComponent(b.body)].join('|'))}``. That encodes the payload twice: `&member=SYM` grows to `%2526member%253DSYM`, which is 16 + len(SYM) bytes.
  - `pullrun::request_leg` calls `crate::server::pull_spot(...)` directly (pullrun.rs:740-752). So the larger `MAX_MEMBER_FORM_BYTES` that D-1499 sized for `/pull/spot` never applies to this path. The only body boundary a press crosses is `/pull/run`'s 8 KiB.
- Why it is wrong: The Pull button sends only through `/pull/run`. With ~8-character symbols, one feed and one rung, the press refuses at about 340 ticked members. With 2 feeds × 3 rungs (6 legs, each repeating the member list) it refuses at about 55. D-1499's `MAX_MEMBERS = 2000` and "a caller may legitimately tick all of it" never reach the press path. The refusal is axum's plain-text "length limit exceeded" 413, not the route's JSON. So `r.json()` throws, and the page shows "The run could not be started: SyntaxError …" instead of a reason. Recovery is affected the same way: `recovery::checked` requires explicit F&O members, and each leg carries them, so a two-rung recovery over the ~210 F&O names (≈5 KB per leg) also exceeds 8 KiB.
- Repro: On /ingest, pick 2 feeds and the 1min, 5min and 1day rungs. Tick 100 of the 750 NIFTY Total Market names and press Pull. The form is 6 legs × ~100 × ~24 B, about 14 KB, so the POST /pull/run answer is a 413 text/plain. A server test: build `legs` the way the page does for 400 members and one leg, POST it through `router_serving`, and assert the status is not 413. It fails today.
- Suggested fix: Give `/pull/run` and `/pull/recovery` a route-level `DefaultBodyLimit` derived from `MAX_MEMBER_FORM_BYTES`, times the leg bound, times the 3× re-encoding. Or bound the leg count, and refuse an oversized body with the route's own JSON refusal.

### P3-01-02 low Known fields are silently dropped by the routes that do not use them: `/backtest/run` ignores `rung` (and then sweeps all eight rungs), and `/backtest/descend` and the ordinary `/engine/command` words ignore every engine knob
- Where: crates/api/src/sweeprun.rs:612-652 (`WireBody`, the union of the three bodies), 906-1013 (`asked_from_wire`), 1083-1148 (`descent_from_wire`), 2705-2730 (`rung_from`), 2773-2868 (`command_from_wire`), 1534-1560 (`conduct_descent`)
- Evidence:
  - `WireBody` is one struct for all three routes ("The union of the three browser-engine request bodies"). It declares `rung`, `rungs`, `command`, `min_hits`, `max_points`, `support_ppm`, `validate`, `screen_cap` and so on. Every route therefore decodes every field, and only reads the ones it needs.
  - `/backtest/run` reads `rungs` and nothing called `rung`: `let rungs = match list_field(body, "rungs") { None => EVERY_RUNG.to_vec(), ...`. So `{"rung":"5min", ...}`, the field name descend and command use, is decoded, ignored, and gives a sweep of all eight rungs.
  - `descent_from_wire` calls `let asked = asked_from_wire(body)?;`, which parses `rungs` and `knobs_in(body)`. It then builds `AskedDescent { feed, underlying, rung, from, to, max_points, top }`, which has no knobs field. `conduct_descent` calls `cli::elite_descend_in_points_for_attempt(...)` without applying any. So `"validate":"0"`, `"screen_cap":…` or `"support_ppm":…` on a descent are validated, then dropped.
  - `command_from_wire` has the same shape for `audit-range`, `screen`, `auto-stored`, `sweep-stored` and `sweep-all`. Only `audit-audited-range` goes through `strict_knobs`, which refuses an inapplicable field "No setting was ignored" (sweeprun.rs:3309-3335).
  - `sweep-all` goes further: `let asked = asked_from_wire(body)?;` *requires* `underlying` and all four span fields, then keeps only `feed`, `rung` and `min_hits`. A correct client must send an invented instrument and span, which are then discarded.
- Why it is wrong: D-0685 states the rule these routes claim to follow: "a budget the operator typed that silently did nothing would be the fallback `CLAUDE.md` §4 bans". It is enforced for `screen_budget_ms` alone (and for the strict word). For a recorded run, a typed `validate:false` or `screen_cap` that silently does nothing changes what the operator thinks was computed. A misplaced `rung` turns a one-rung request into an eight-rung, hours-long sweep. These are known fields, not the unknown fields BE-03/D-0685 keep for compatibility.
- Repro: Unit test `descent_from(r#"{"feed":"dhan","underlying":"NIFTY","from_year":2024,"from_month":1,"to_year":2024,"to_month":2,"rung":"5min","max_points":50,"top":5,"validate":"0","screen_cap":7}"#)` returns `Ok`, and nothing carries `validate` or `screen_cap`. Likewise `asked_from(r#"{...,"rung":"5min"}"#).unwrap().rungs == EVERY_RUNG`.
- Suggested fix: Each route should refuse, by name, any known `WireBody` field it does not consume, as `strict_knobs` already does. Or split the union into one struct per route with `deny_unknown_fields` limited to the sibling-route names. `sweep-all` should not require `underlying` or a span.

### P3-01-03 low A client that disconnects during a sweep/descent/command admission gets its HTTP invocation recorded as `cancelled` with status 0, while the detached admission keeps running and launches the run
- Where: crates/api/src/operation_audit.rs:129-197 (`request_audited`); crates/api/src/sweeprun.rs:1723-1734, 1959-1970, 2979-2990 (`run`/`descend`/`command` await `crate::detail::run(...)`); crates/api/src/detail.rs:100-107, 167-178; crates/cli/src/operation_audit.rs:445-461
- Evidence:
  - `let mut response = handler.await;` holds the armed `attempt` across the handler.
  - The handler is `crate::detail::run(move || run_with(&site, &body, cli::commit_stamp())).await`, and `admitted` is `tokio::task::spawn_blocking(move || { let _permit = permit; work() }).await`. A `spawn_blocking` closure is not cancelled when its `JoinHandle` is dropped.
  - On drop, `Attempt::drop` writes `let phase = if std::thread::panicking() { Phase::Failed } else { Phase::Cancelled }; ... self.finish(phase, 0)`.
  - The repo already calls this outcome wrong, in detail.rs:183-185: "refusing the write that records its outcome cannot undo it and used to leave a false `Cancelled`". D-1445 removed that false `Cancelled` only for the full-pool path.
- Why it is wrong: The HTTP boundary record for a POST whose handler went on to take the lease, write its browser invocation start and spawn the engine task says `cancelled`/0. The work was not cancelled. A disconnect does not stop admission (spawn_blocking cannot be cancelled), so the run starts with nobody holding its attempt id. `/backtest/audit.json` then shows a cancelled request next to a browser invocation that runs. The likely trigger is the page's own `ask()` timeout (15 s default) during a slow admission (ADMISSION wait, the 8 MiB external-log walk, the fsyncs). The fsync-on-worker half of this drop is recorded as log-2 / resources-3. The false terminal on a write route, and the launch that outlives the cancel, are not.
- Repro: Make admission slow, for example by holding `sweeprun::ADMISSION` in a test or by pointing `BRUTEX_LOG_DIR` at an 8 MiB log. POST `/backtest/run` with a valid body through `audited_router_serving`, and close the socket after 100 ms. Afterwards the HTTP record for `POST /backtest/run` reads `cancelled`, status 0, while `site.sweep` holds an in-flight `Progress` and a browser `sweep` invocation exists.
- Suggested fix: For the three launch routes, move the HTTP `Attempt` into the blocking closure, or record a distinct "client gone, handler outcome unknown" phase. Never `Cancelled` while the handler can still dispatch. Optionally re-check a cancellation flag inside `prepare` before taking the lease.

### P3-01-04 low `POST /masters/refresh` runs its whole retry ladder inside the request, and the page aborts at 90 s, so a slow host cuts the refresh off before the reload and before any per-source log record is written
- Where: crates/api/src/mastersrun.rs:502-640 (`refresh`), 54-75 (`record` doc); crates/pull/src/masters.rs:574, 1251-1261; web/src/routes/mapping/+page.svelte:144-153
- Evidence:
  - `refresh` awaits every source inline: `let mut landed = refresh_with(&from, &clock, &dir, Transport::Public).await; credentialed_leg(&mut landed, &clock, &dir).await;`. Only after that does it run `for (source, outcome, tried) in &landed { record(source, outcome, tried); }` (554), and only after that `let reloaded = reload(&site, &dir);` (625).
  - Per URL there are `ATTEMPTS_PER_URL: u32 = 5` attempts with `.timeout(core::time::Duration::from_mins(1))` each, and a timeout is verdict `Again` (masters.rs:1212). One hanging host therefore holds the request for about 5 × 60 s + 7.5 s of backoff, and the ladder repeats for each mirror.
  - The page: `await ask('/masters/refresh', { method: 'POST', ms: 90_000 });`. `ask` aborts through `AbortSignal.timeout(ms)` (web/src/lib/ask.js:61-63). The page comment says 90 s covers "a sick host".
  - `record`'s own doc lists as a defect it fixed: "**The attempt ledger was not durable.** It reached the HTTP response and nowhere else, so closing the tab destroyed the only record of what the host actually said."
- Why it is wrong: The abort closes the connection, and hyper drops the handler future (hunt-api-1 proved this on this server). Any sources that already landed stay replaced on disk, but `reload` never runs. The live universe stays on the old parse while the files are new. None of the `api.masters.attempt` / `api.masters.source` / `api.masters` events is written. So closing the tab, or the page's own timeout, still destroys the only record of what the hosts said, which is the defect the doc says is closed. The state can be recovered (`/masters/status.json` reports `newer_than_parse`), but the refusal is not loud: the page shows only "No response … within 90 s".
- Repro: Unit-level: run `refresh_with` with a `Discovery` whose `get` sleeps 60 s for the first source, and drop the future after 90 s. Assert that no `api.masters.source` event was emitted and that `site` generation is unchanged. End to end: block one master host at the firewall (packets dropped) and press Refresh on /mapping. After 90 s the page times out, the log holds no masters event, and `/masters/status.json` reports `newer_than_parse` for whichever files landed.
- Suggested fix: Detach the refresh from the connection, as `recovery::start` does: `tokio::spawn` under `REFRESH`, emit `record` per source as each one finishes, and always run `reload`. Have the route answer from that task or expose its result on `/masters/status.json`. Or bound the server ladder below the page's ceiling and state that bound.

### P3-01-05 low docs/04 MR-20 still says the masters have "no reload path" and cites a test that no longer exists. The Z1-slice13-F1 / D-1762 fix left this row behind
- Where: docs/04-invariants.md:3042; crates/api/src/mastersrun.rs:21-33, 625, 636, 1517
- Evidence:
  - The row reads: "`Site::load` parses the masters once at startup with no reload path. The row reports `newer_than_parse` and the post-refresh line asks for a restart … | `api::mastersrun::the_page_says_a_restart_is_required_rather_than_pretending_otherwise` · `the_status_answer_carries_when_each_master_was_written` | ✓".
  - The handler calls `let reloaded = reload(&site, &dir);` and answers `"restart_required":false`.
  - The test was renamed to `the_page_says_a_refresh_reloads_and_names_when_a_restart_is_required` (mastersrun.rs:1517). `rg the_page_says_a_restart_is_required crates/` finds nothing.
- Why it is wrong: The invariant table is the authority for "what must hold, and its proof" (CLAUDE.md §10). The row states the opposite of the handler's behaviour, and is marked ✓ against a proof that does not exist. Z1-slice13-F1's fix (D-1762, "Prose only") corrected mastersrun.rs, lib.rs and the page, but not this row.
- Repro: `rg -n 'no reload path' docs/04-invariants.md` matches line 3042. `rg -n 'fn the_page_says_a_restart_is_required_rather_than_pretending_otherwise' crates` gives no match.
- Suggested fix: Rewrite MR-20 to say a refresh re-parses through `Site::reparse`, and that a restart is needed only when the reparse is refused or a file changed by another hand. Cite `the_page_says_a_refresh_reloads_and_names_when_a_restart_is_required`.

## Checked and holding (no finding)
- Cross-origin: every non-GET/HEAD needs a local `Host` on the bound port, an exact `http` `Origin`, and `Sec-Fetch-Site` absent or `same-origin`. Forwarded headers are refused (server.rs:16646-16745). There are no CORS headers.
- `/pull/run`: the check and the claim happen under one `site.run` take, before `tokio::spawn`. Work is detached from the connection. The envelope and payload must agree (P1-02-01 fix verified).
- `/pull/recovery`: `recovery_action`, `recovery_supersedes` and `recovery_generation` must each appear once and be non-empty (`one_parameter`). Activation is detached from the connection (`tokio::spawn(activate_durable…)`). Prepare is idempotent under the journal flock.
- `/autopilot/control`: an unknown or empty `action` gets a named 400; a refused resume gets 409 without publishing; pause cannot be refused.
- `/ingest/queue`: parses in full and answers 501 with the reason. There is no side effect.
- Sweep routes: strict JSON object, duplicate known keys refused, nulls and wrong types refused, years bounded before the multiply. The `ADMISSION` mutex plus the store lease serialise launches. `TaskFinisher` is armed before `spawn_blocking`. A double click gets 409.

## Source: p3/02-cli-surface.md

# P3-02: operator-facing CLI and API surface, documented vs implemented

Checkout: /home/claude/brutex-audit @ 5140aca. Static reading only. Nothing was built or run.

Method (scripts in this directory):
- `cmds.txt`: the 35 `COMMANDS` words. For each one I compared the `USAGE` argument list (lib.rs:346-597) with its `dispatch` arm (lib.rs:2172-2304) by hand. All 35 are dispatched, every dispatched word is listed, and the argument counts match, with one exception (P3-02-03).
- `files.txt` (353 tracked files: docs/, README.md, AGENTS.md, .claude/, web/), `words.txt`: every `cli <word>` token. The only words that are not commands are prose ("cli batch 02", "cli digest", "cli repair", ...).
- `arity.awk`: counts the arguments of every backticked `cli <command> ...` invocation. The one mismatch is an elided `…` example in docs/11-findings.md:270, which is not an error. I also checked by hand the 27 code-block or `cargo run -p cli --` invocations (docs/17, 21, 24, 28, 29, web/policy-guide/README.md, docs/05:22327, docs/06:4823) and the 4 `cli range-all` strings in web/src. All have the correct shape.
- `served.txt` vs `vite.txt`: the axum route table (server.rs:15939-16541) against the `web/vite.config.js` ROUTES. The 5 fetched-but-unproxied JSON routes are already recorded as P2-02-01, so they are not repeated here. No doc claims a complete route list. The `/top.json`, `/pullrun.json` and `/viewer.json` mentions are historical ledger text or the separate viewer under web/.

### P3-02-01 medium In the shipped binary, `/audit` is still answered by two applications: a nav click renders the Svelte console, while a reload or bookmark renders the Rust page
- Where: crates/api/src/server.rs:16350; web/src/routes/+layout.svelte:110; web/vite.config.js:149-163; web/build/audit.html
- Evidence:
  - server.rs:16350 `.route("/audit", axum::routing::get(audit_get))`. `audit_get` (server.rs:14999) returns `audit_html(...)`, the Rust no-script page.
  - web/src/routes/audit/ exists and is built to web/build/audit.html. The nav links to it: `{ href: '/audit', label: 'Audit' }` (+layout.svelte:110), and terminal, ingest and autopilot pages also link to `/audit`.
  - The same file states the rule for `/backtest` (server.rs:16368-16374): "There is no `.route("/backtest", ...)`: the front end owns that path through the fallback below ... A registered route beats `Router::fallback` unconditionally, so a page here would make a CLICK render Svelte and a RELOAD render Rust -- the two applications on one URL that `web/vite.config.js` documents against `/audit`."
  - vite.config.js:161-163 claims the problem is fixed: "The page now owns `/audit` here, the Rust page still answers `/audit` on the API's own port, and nothing renders differently depending on how the operator arrived."
- Why it is wrong: The fix only removed `/audit` from the dev proxy. In production the Rust binary serves the Svelte build through `.fallback`, and the registered `/audit` route beats that fallback. So on port 8080 a client-side nav click shows the Svelte console, while a reload, a bookmark or a typed URL shows the Rust page. This is exactly the defect the server's own `/backtest` comment describes. The Svelte audit console can only be reached by clicking.
- Repro: Run `api serve` with web/build present. Open `http://127.0.0.1:8080/terminal` and click "Audit": the Svelte page renders. Press reload: `audit_get` answers with the Rust `audit_html` page (different nav, no theme). A test sketch: `get(addr, "/audit")` on the production router returns a body containing `audit_html`'s markup and not the contents of build/audit.html.
- Suggested fix: Move the Rust page to its own path (for example `/audit/page` or `/audit.html`, and update its links) so the fallback serves the Svelte shell at `/audit`. Do it the way `/backtest` was handled, and correct the vite.config.js sentence.

### P3-02-02 medium The backtest page names instruments by the last `-` segment, so the sweepable shares BAJAJ-AUTO and NAM-INDIA become `AUTO` and `INDIA`. The copyable `cli range-all` line and the Run/Descend requests then name an underlying the engine refuses
- Where: web/src/routes/backtest/+page.svelte:3335, 3446, 3521, 6557-6558, 2556, 2850; crates/api/src/census.rs (Series Display, `NSE-<SEGMENT>-<SYMBOL>`); crates/cli/src/stored.rs:1917-1948
- Evidence:
  - +page.svelte:3335 `const leaf = full.split('-').pop() ?? '';`, where `full` is `/store.json`'s `instrument` (`series.to_string()`, server.rs:4642). For a cash equity that is `NSE-CASH-<SYMBOL>` (`Segment::Cash => "CASH"`).
  - core/src/symbol.rs:75 admits `-` in a symbol. `FNO_UNDERLYINGS` contains `"BAJAJ-AUTO"` and `"NAM-INDIA"` (universe.rs:233 and others). Both are shares, so both are on the D-0506 sweep surface.
  - The picker offers `key: h.leaf, name: h.leaf` (6557-6558). `sweepSymbol` is the picked key (3261). The copy line is `` `cli range-all ${activeFeed} ${heldNow.leaf} ${from.year} ...` `` (3521). `/backtest/run` and `/backtest/descend` send `underlying: sweepSymbol` (2556, 2850).
  - `swept_index("AUTO")` tries the index and then `InstrumentKey::cash(Exchange::Nse, "AUTO").require_sweepable()`. `AUTO` is not an F&O underlying, so the result is the refusal "`AUTO` is not an instrument this engine sweeps ...".
- Why it is wrong: For two sweepable instruments held in the store, the page shows the wrong name (`AUTO`, `INDIA`). Its "exact `cli` line that would fill an empty ledger" (comment at 3513-3516) is a command the binary refuses, and every Run or Descend press on them is refused. Those instruments cannot be reached from the page at all, even though both the CLI and the census name them correctly.
- Repro: With a store holding `zerodha/NSE/CASH/BAJAJ-AUTO/...` months, open /backtest. The picker lists `AUTO` (title `NSE-CASH-BAJAJ-AUTO`), and the empty-ledger hint prints `cli range-all zerodha AUTO ...`. Running that line exits MISUSED/FAILED with "`AUTO` is not an instrument this engine sweeps". Pressing Run gets the same refusal from `/backtest/run`.
- Suggested fix: Strip the known `NSE-<SEGMENT>-` prefix (the first two segments) instead of taking the last segment, or have `/store.json` emit the bare symbol as its own field, and key the picker, the request and the command string on that.

### P3-02-03 low `USAGE` omits the optional 18th TIMEFRAMES argument of `boolean-qualified-search-stored` and says the command always covers all eight rungs
- Where: crates/cli/src/lib.rs:379-384 (USAGE); lib.rs:2270-2274 (dispatch); crates/cli/src/boolean_search_command.rs:32-43, 65
- Evidence:
  - USAGE: `cli boolean-qualified-search-stored VENDOR SYMBOLS FROM_Y FROM_M TO_Y TO_M BITS HORIZON MAX_POINTS BATCH_PROGRAMS NODE_ALLOWANCE BATCH_ALLOWANCE OUTPUT_ROOT LATER_FROM_Y LATER_FROM_M LATER_TO_Y LATER_TO_M` (17 arguments), "automatically process bounded batches across all eight intraday rungs".
  - Dispatch: `["boolean-qualified-search-stored", arguments @ ..] if matches!(arguments.len(), 17 | 18)`.
  - Parser: `[arguments @ .., selected] if arguments.len() == 17 => (arguments, campaign::RungScope::new(&selected.split(',').collect::<Vec<_>>())?)`. Its own refusal text names `[TIMEFRAMES comma-separated; omitted means all eight]`.
- Why it is wrong: USAGE is the operator's only reference ("printed on every refusal so the reader never has to guess"), and it hides a supported argument that narrows the run to a subset of rungs. It also states unconditionally that every run spans all eight. This is the only command whose accepted arity differs from its usage line, and `every_command_is_listed_in_both_places` cannot catch it (see P1-11-01).
- Repro: `cli boolean-qualified-search-stored <17 args> 5min,15min` is accepted and restricted to two rungs, yet nothing in `cli` output with no arguments mentions a TIMEFRAMES argument.
- Suggested fix: Add `[TIMEFRAMES]` to the usage line, explain that it is a comma-separated subset and that omitting it means all eight, and reword "across all eight intraday rungs".

### P3-02-04 low `USAGE` is a plain `&str` but uses printf escapes, so the operator sees `80%%` and `95%%`
- Where: crates/cli/src/lib.rs:475-476
- Evidence: `Every rule is on: 80%% of trades won AND 80%% on` / `the 95%% lower bound, ...` inside `pub const USAGE: &str = "\...`. USAGE is emitted with `out.push_str(USAGE)` (lib.rs:2428), and `deliver` writes `text.as_bytes()` (lib.rs:3143). Neither goes through `format!`, so `%%` is never collapsed.
- Why it is wrong: The `elite` help prints "80%% of trades won AND 80%% on the 95%% lower bound", which misstates the thresholds the operator is told are on. Rust format strings do not use `%` escapes in any case.
- Repro: `cli` with no arguments. The output contains the literal `80%%`. A test sketch: `assert!(!USAGE.contains("%%"))`.
- Suggested fix: Replace `%%` with `%` on both lines.

### P3-02-05 low `USAGE` defines UNDERLYING as "the index, e.g. NIFTY or BANKNIFTY", but every UNDERLYING command also accepts the 208 F&O cash equities
- Where: crates/cli/src/lib.rs:588; crates/cli/src/stored.rs:1934-1948
- Evidence: USAGE: `UNDERLYING  the index, e.g. NIFTY or BANKNIFTY`. `swept_index`: "THEN THE CASH EQUITY. Same word, the stock's own price series. Sweepable only when the symbol is an F&O underlying that is a share". Its refusal text says the surface is "the NSE spot indices ... and the NSE cash equities of the {shares} F&O underlyings" (D-0506, D-0682). USAGE's own `pool` entry (lib.rs:525-528) says "the two indices and the F&O cash equities".
- Why it is wrong: The glossary every refusal prints narrows the engine surface that CLAUDE.md §1 defines. An operator reading it would not know `cli range-all zerodha RELIANCE ...` is valid. That is a silent scope statement with no decision behind it, and it contradicts another entry in the same text.
- Repro: Compare `cli` output line `UNDERLYING  the index, ...` with `cli sweep-stored zerodha RELIANCE 15min 2025 1 100`, which resolves through the cash arm.
- Suggested fix: Reword to "the instrument: NIFTY, BANKNIFTY, or an F&O share's symbol (e.g. RELIANCE)".

### P3-02-06 low `POST /engine/command` with `sweep-all` requires `underlying` and a span, which `cli sweep-all` does not take, and then silently ignores them
- Where: crates/api/src/sweeprun.rs:2848-2864 (sweep-all arm), 906-972 (`asked_from_wire`), 2641, 2673
- Evidence:
  - The arm starts `let asked = asked_from_wire(body)?;`, and `asked_from_wire` refuses with "no `underlying` in the request." and "`from_year` is missing or not a number." (917-929). It also refuses a backwards span (967).
  - Only `asked.feed` is kept: `Ok(Command::SweepAll { feed: asked.feed, rung, min_hits: ... })`. `underlying()` reports `"ALL"` and `window()` reports `((0, 1), (0, 1))`.
  - The CLI shape is `cli sweep-all VENDOR RUNG MIN_HITS` (USAGE lib.rs:581-583; dispatch `["sweep-all", vendor, rung, min_hits]`). The `Command::SweepAll` doc reads "Every stored month for one feed and rung, in one batch."
  - The parse test works only because its helper injects the fields: `command_body` = `{"feed":"zerodha","underlying":"NIFTY",{SPAN},...}` (sweeprun.rs:5739-5741).
- Why it is wrong: A body that matches the documented command (`{"command":"sweep-all","feed":"zerodha","rung":"15min","min_hits":500}`) is refused for missing fields the command does not use. A body that names `"underlying":"BANKNIFTY"` and one month is accepted, and then sweeps every instrument and every month on the feed. That is input that is required and then discarded, with no word to the caller.
- Repro: `command_from(r#"{"command":"sweep-all","feed":"zerodha","rung":"15min","min_hits":500}"#)` returns `Err(Malformed("no `underlying` in the request."))`. With `"underlying":"XYZ","from_year":2024,"from_month":1,"to_year":2024,"to_month":1` added, it returns `Ok(SweepAll{..})` and the underlying and span are dropped.
- Suggested fix: Parse only `feed`, `rung` and `min_hits` for `sweep-all`, and refuse `underlying`/span fields by name if they are present, so a scoped-looking request cannot become a whole-store batch.

### P3-02-07 low `.claude/launch.json` tells the operator a build without `BRUTEX_COMMIT` "refuses every sweep"; `crates/cli/build.rs` now stamps the commit itself
- Where: .claude/launch.json:5-9; crates/cli/build.rs:1-2, 66-70, 82-91; crates/cli/src/lib.rs:589-593
- Evidence:
  - launch.json configuration name: "Stamps the build with HEAD, because crates/cli/src/lib.rs commit_stamp() reads BRUTEX_COMMIT at COMPILE time and a server built without it refuses every sweep by CLAUDE.md §3 rule 3." The run line is `BRUTEX_COMMIT=$(git rev-parse HEAD) exec cargo run --release -p api -- serve`.
  - build.rs:1-2: "Stamps `BRUTEX_COMMIT` from git's own files, so a plain `cargo run` produces a binary that can record a run." build.rs:66-70: an explicit value "is accepted only when it is canonical, equals locally resolvable HEAD, and the index and working tree both equal that commit." `api` depends on `cli` (crates/api/Cargo.toml:91), so this build script runs for the server too.
  - USAGE (lib.rs:589-593): "BRUTEX_COMMIT is optional and cannot bypass that proof".
- Why it is wrong: The one tracked operator run configuration (named in CLAUDE.md §2) gives a reason that is no longer true. Without the variable the build is stamped anyway, and with it a dirty tree is still left unstamped. An operator who sees a dirty-tree refusal will blame the missing variable instead of the uncommitted source.
- Repro: Build `cargo run -p api -- serve` from a clean HEAD without `BRUTEX_COMMIT`. `cli::commit_stamp()` is `Some(HEAD)` through build.rs, so sweeps record, which contradicts the configuration name.
- Suggested fix: Drop the `BRUTEX_COMMIT=$(git rev-parse HEAD)` prefix, or keep it as harmless, and reword the name to say the build is stamped by `crates/cli/build.rs` and only a clean tree records.

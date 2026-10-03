# slice14 (crates/api/src/server.rs) — 4 findings

## F1 [medium] `/instruments.json` "bars" adds up every series that shares the symbol: other segments, other exchanges, every rung and every F&O contract
- where: crates/api/src/server.rs:1353-1378 (`bars_by_symbol`), used at :1206-1214 (sort key, :1218) and :1220-1221 (emitted `bars`)
- what: the listing has one row per `InstrumentKey` {exchange, segment, underlying, kind}. The count is a map keyed by `series.symbol` alone (`*held.entry(series.symbol).or_insert(0) += rows`). `census::Series` also carries `exchange`, `segment`, `timeframe` and `contract` (census.rs:619-649), and none of them is part of the key. F&O bars are filed with the underlying as the symbol (server.rs:13291 `symbol: asked.underlying.as_str()`, and the same in `land_rolling_group`). That means:
  - the NSE-INDEX-NIFTY row's `bars` = NIFTY index 1min bars + 1day bars + the bars of every stored NIFTY option or future contract;
  - NSE-CASH-X and BSE-CASH-X get the same number;
  - an index with no spot bars at all, but with stored options, sorts into the "held first" group (`bars_of(..) == 0` is false) and shows a non-zero count. Clicking it charts nothing.
  The route's own comment (:1185-1189) says the field answers "how much of this do I have" for the row's instrument, and the D-0124 note says the number is a measurement.
- evidence (trace): `held_entries` (census.rs:887-899) returns every held key of the feed, contract and rung included, mapped through `Series::of`. `bars_by_symbol` sums `census.rows_for(&series.at(month))` into `held[series.symbol]`. `instruments_json` reads `held.get(&key.underlying)` for each row. The census for one feed holding `NSE/INDEX/NIFTY/1min/2026-07` (8,250 rows), `NSE/INDEX/NIFTY/1day/2026-07` (22 rows) and `NSE/FNO/NIFTY/<contract>/1min/2026-07` (375 rows) produces `"bars":8647` on the NIFTY index row. With only the FNO entry it produces `"bars":375` on an index the store holds no spot bars for.
- fix: key the map by `(exchange, segment, symbol)` and skip `series.contract.is_some()`. Either restrict it to one rung (the minute rung) or emit one count per rung. Then look up `held.get(&(key.exchange, key.segment, key.underlying))`.

## F2 [medium] The ingest page's live-broker text says chunking and expired-F&O do not exist, and both do
- where: crates/api/src/server.rs:5073-5088 (`HTTP_LIVE`), rendered by `pull_html` at :5908 (`halt: Some(halt_for(site.broker))`) on every served (`Broker::Live`) process
- what: the operator-facing constant ends "WHAT IS STILL MISSING, so this sentence does not overstate itself …: a window longer than the vendor's per-request cap is still sent whole rather than split, so a multi-year range is refused by the vendor or silently truncated by it (pull::session::split_window computes the chunks and has no caller yet); … and the expired-F&O endpoints are not modelled at all." Both claims are false in this file:
  - `split_window` has three production callers: server.rs:8913 (`fetch_chunks`, the spot broker path), :11403 (`owed_chunks`) and :11485.
  - the expired-options endpoints are modelled and called: `fno_roll`/`roll_every`/`roll_one`/`fetch_rolling` (:14408, :14005, :12535, :13725) over `pull::rolling` ("Dhan's ATM-relative expired-options driver").
  So the live page tells the operator a multi-year pull will be refused or silently truncated, and steers them away from the one path that splits. The `halt_for` test pins only which constant is chosen, not whether it is true (doc at :5051-5070), so nothing catches this. It is the exact defect the constant's own doc describes twice.
- evidence: `grep -n 'split_window(' crates/api/src/server.rs` → lines 8913, 11403 and 11485, all outside `#[cfg(test)]`. `grep -n 'split_window' crates/api/src/server.rs` shows 5086 is the only place that says "no caller".
- fix: rewrite the missing-features clause of `HTTP_LIVE` to the current state: windows are split to each rung's cap and to month boundaries, and expired options go through Dhan's rolling endpoint. Check the "STORED for an empty window" clause against P-68 too. Add a test that ties each "missing" clause to a fact the code can check, e.g. that `fetch_chunks` calls `split_window`.

## F3 [low] `/bars/window.json` reads an unknown `dir` as descending and an unknown `extremes` as off, against its own refuse-don't-default rule
- where: crates/api/src/server.rs:3324 and :3327 (`WindowAsk::parse`); the rule is stated on the struct at :3250-3254
- what: `WindowAsk`'s doc says "Every field REFUSES rather than defaulting where a default would answer a different question than the one asked". `offset`, `limit`, `sort`, `feed`, `timeframe` and `contract` follow that rule. Two fields do not: `desc: !matches!(param(query, "dir").as_str(), "asc")` and `want_extremes: matches!(param(query, "extremes").as_str(), "1" | "true")`.
  - `dir=ASC`, `dir=ascending` and `dir=up` are all answered in descending order with 200.
  - `extremes=yes` and `extremes=TRUE` silently drop the extremes. `render_window` then reports `scanned` from the sort alone, so the response does not show that the extremes were dropped.
  This is the "a query silently made into a different query" case that `param`'s doc and probeapi-5 refuse elsewhere.
- evidence (trace): `param(q,"dir")` = "ASC" → `matches!("ASC","asc")` is false → `desc = true` → `bars::window(.., desc=true, ..)` returns the newest rows first, under `StatusCode::OK`. No arm returns `Err`. The route tests (bars_window_route_tests.rs:230, 267, 297, 320) send only `dir=asc` and `extremes=1`, so no test covers another value.
- fix: parse `dir` as `"" | "desc" → true`, `"asc" → false`, anything else → `Err(format!("{raw:?} is not a direction; asc or desc"))`. Parse `extremes` the same way (`"" | "0" | "false"`, `"1" | "true"`, else refuse). Add one refusal test for each.

## F4 [low] A rustdoc says `crates/pull` has no telemetry dependency; the manifest declares one
- where: crates/api/src/server.rs:7101-7103 (doc of `note_member_failure`)
- what: "`crates/pull` declares no telemetry dependency and does not gain one: that would add an edge `CLAUDE.md` §5's graph does not carry. The event is emitted at the `api` layer". crates/pull/Cargo.toml:87 declares `telemetry = { path = "../telemetry" }`, and its own comment (:71-80) says pull -> telemetry keeps the graph acyclic. CLAUDE.md §5 draws `core costs greeks store telemetry <-- pull`, and pull/src/{http,ssm,csv,archive,work,...}.rs call `telemetry::`. The sentence states a crate-graph rule that is false, and it is the reason given for where this event is emitted. A reader who trusts it will believe adding pull-side telemetry breaks §5.
- evidence: `grep -n telemetry crates/pull/Cargo.toml` → line 87 `telemetry = { path = "../telemetry", version = "0.1.0" }`. `grep -rln 'telemetry::' crates/pull/src` lists 10+ files.
- fix: replace the paragraph with the real reason the event is emitted in `api`: the member, month and request context exist only here. Or drop it.

## Checked, no finding
I checked these and found nothing reportable:
- the query decoder (`param`, `field_value`, `params`, `percent_decode`, `hex_escape`), `repeated_query_key` and `request_bounds_refusal`;
- origin and host admission (`cross_origin_refusal`, `journaled_read_refusal`, `local_host_authority`, `origin_matches_host`, `sole_visible_header`);
- the `basis_points` rounding and overflow argument, and `month_before`/`next_month`;
- `ist_midnight_micros`/`day_window_bounds` (IST sign and the half-open window), and the `bars_json` bisection bounds and fallback;
- the `gaps_json` span walk (240-month cap, the truncation flag at the exact boundary) and the peer-calendar self-exclusion;
- the retry ladders (`step`, `throttle_ladder`, `server_ladder`; `attempt` starts at 1, so `attempt - 1` cannot underflow) and the breaker;
- `clamp_to_floor`, `owed_chunks` and `months_to_walk`;
- `store_dir_from` and `take_serve_lock` (an empty `BRUTEX_STORE` is refused by the metadata check);
- `note_alphabet`/`note_header`, `none_match_hit`, `render_elapsed`, `ist_stamp`, `bars_html` paging and `log_level_from`.

One defect I left out because only tests reach it: `ServeLock::drop` for an in-process second serve (`held: None`) removes the shared root key while the first server still holds it (:17580-17588). Production calls `run` once per process.
